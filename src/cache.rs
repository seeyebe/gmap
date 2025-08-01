use crate::error::{GmapError, Result};
use crate::model::{CommitInfo, CommitStats, DateRange, FileStats, SCHEMA_VERSION};
use chrono::{Utc, TimeZone};
use rusqlite::{params, Connection, ToSql};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn new<CP: AsRef<Path>, RP: AsRef<Path>>(cache_path: Option<CP>, repo_path: RP) -> Result<Self> {
        let cache_dir = match cache_path {
            Some(path) => path.as_ref().to_path_buf(),
            None => repo_path.as_ref().join(".gmap"),
        };
        std::fs::create_dir_all(&cache_dir)?;
        let db_path = cache_dir.join("cache.db");
        let conn = Connection::open(&db_path)?;
        let mut cache = Self { conn };
        cache.initialize()?;
        Ok(cache)
    }

    fn initialize(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS commits (
                id TEXT PRIMARY KEY,
                author_name TEXT NOT NULL,
                author_email TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                parent_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS files (
                commit_id TEXT NOT NULL,
                path TEXT NOT NULL,
                added_lines INTEGER NOT NULL,
                deleted_lines INTEGER NOT NULL,
                is_binary INTEGER NOT NULL,
                PRIMARY KEY (commit_id, path),
                FOREIGN KEY (commit_id) REFERENCES commits(id)
            );
            CREATE INDEX IF NOT EXISTS idx_commits_timestamp ON commits(timestamp);
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
            ",
        )?;
        self.check_schema_version()?;
        Ok(())
    }

    fn check_schema_version(&mut self) -> Result<()> {
        let user_version: i64 = self
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))?;

        if user_version == 0 {
            let set_stmt = format!("PRAGMA user_version = {SCHEMA_VERSION};");
            self.conn.execute_batch(&set_stmt)?;
        } else if user_version != SCHEMA_VERSION as i64 {
            return Err(GmapError::Cache(format!(
                "Schema version mismatch: expected {}, found {}",
                SCHEMA_VERSION, user_version
            )));
        }

        Ok(())
    }

    pub fn get_commit_stats(&self, range: &DateRange) -> Result<Vec<CommitStats>> {
        let mut query = String::from(
            "SELECT c.id, f.path, f.added_lines, f.deleted_lines, f.is_binary
             FROM commits c
             LEFT JOIN files f ON c.id = f.commit_id
             WHERE 1=1",
        );
        let mut to_bind: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(since) = &range.since {
            query.push_str(" AND c.timestamp >= ?");
            to_bind.push(Box::new(since.timestamp()));
        }
        if let Some(until) = &range.until {
            query.push_str(" AND c.timestamp <= ?");
            to_bind.push(Box::new(until.timestamp()));
        }
        query.push_str(" ORDER BY c.timestamp");

        let mut stmt = self.conn.prepare(&query)?;
        let bind_refs: Vec<&dyn ToSql> = to_bind.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(
            bind_refs.as_slice(),
            |row| {
                let commit_id: String = row.get(0)?;
                let path_opt: Option<String> = row.get(1)?;
                let added_opt: Option<u32> = row.get(2)?;
                let deleted_opt: Option<u32> = row.get(3)?;
                let is_binary_opt: Option<i64> = row.get(4)?;
                let mut files = Vec::new();
                if let (Some(path), Some(added), Some(deleted), Some(is_binary_int)) =
                    (path_opt, added_opt, deleted_opt, is_binary_opt)
                {
                    let is_binary = is_binary_int != 0;
                    files.push(FileStats {
                        path,
                        added_lines: added,
                        deleted_lines: deleted,
                        is_binary,
                    });
                }
                Ok((commit_id, files))
            },
        )?;

        let mut commits_map: HashMap<String, Vec<FileStats>> = HashMap::new();
        for row in rows {
            let (commit_id, mut files) = row?;
            commits_map.entry(commit_id).or_default().append(&mut files);
        }

        let mut result: Vec<CommitStats> = commits_map
            .into_iter()
            .map(|(commit_id, files)| CommitStats { commit_id, files })
            .collect();

        result.sort_by(|a, b| a.commit_id.cmp(&b.commit_id));
        Ok(result)
    }

    pub fn store_commit_stats(
        &mut self,
        commits: &[CommitStats],
        infos: &HashMap<String, CommitInfo>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;

        let mut insert_commit_stmt = tx.prepare(
            "INSERT OR REPLACE INTO commits (id, author_name, author_email, message, timestamp, parent_ids)
             VALUES (?, ?, ?, ?, ?, ?)",
        )?;
        let mut delete_files_stmt = tx.prepare("DELETE FROM files WHERE commit_id = ?")?;
        let mut insert_file_stmt = tx.prepare(
            "INSERT INTO files (commit_id, path, added_lines, deleted_lines, is_binary)
             VALUES (?, ?, ?, ?, ?)",
        )?;

        for stats in commits {
            if let Some(info) = infos.get(&stats.commit_id) {
                insert_commit_stmt.execute(params![
                    info.id,
                    info.author_name,
                    info.author_email,
                    info.message,
                    info.timestamp.timestamp(),
                    serde_json::to_string(&info.parent_ids)?
                ])?;

                delete_files_stmt.execute(params![stats.commit_id])?;

                let mut seen_paths: HashSet<&String> = HashSet::new();
                for f in &stats.files {
                    if seen_paths.insert(&f.path) {
                        insert_file_stmt.execute(params![
                            stats.commit_id,
                            f.path,
                            f.added_lines,
                            f.deleted_lines,
                            if f.is_binary { 1 } else { 0 }
                        ])?;
                    }
                }
            }
        }

        drop(insert_commit_stmt);
        drop(delete_files_stmt);
        drop(insert_file_stmt);

        tx.commit()?;
        Ok(())
    }

    pub fn get_missing_commits(&self, all_commit_ids: &[String]) -> Result<Vec<String>> {
        if all_commit_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", all_commit_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!("SELECT id FROM commits WHERE id IN ({placeholders})");
        let mut stmt = self.conn.prepare(&query)?;
        let existing: HashSet<String> = stmt
            .query_map(rusqlite::params_from_iter(all_commit_ids.iter()), |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<HashSet<_>>>()?;
        Ok(all_commit_ids
            .iter()
            .filter(|id| !existing.contains(*id))
            .cloned()
            .collect())
    }

    pub fn get_commit_info(&self, commit_id: &str) -> Result<Option<CommitInfo>> {
        let result = self.conn.query_row(
            "SELECT id, author_name, author_email, message, timestamp, parent_ids FROM commits WHERE id = ?",
            params![commit_id],
            |row| {
                let ts: i64 = row.get(4)?;
                let timestamp = Utc
                    .timestamp_opt(ts, 0)
                    .single()
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidColumnType(
                            4,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Integer,
                        )
                    })?;

                let parent_json: String = row.get(5)?;
                let parent_ids: Vec<String> = serde_json::from_str(&parent_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        parent_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                Ok(CommitInfo {
                    id: row.get(0)?,
                    author_name: row.get(1)?,
                    author_email: row.get(2)?,
                    message: row.get(3)?,
                    timestamp,
                    parent_ids,
                })
            },
        );
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

}
