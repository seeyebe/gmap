use crate::error::{GmapError, Result};
use crate::model::{CommitInfo, CommitStats, DateRange, FileStats, SCHEMA_VERSION};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::path::{Path};

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
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS commits (
                id TEXT PRIMARY KEY,
                author_name TEXT NOT NULL,
                author_email TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                parent_ids TEXT NOT NULL
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                commit_id TEXT NOT NULL,
                path TEXT NOT NULL,
                added_lines INTEGER NOT NULL,
                deleted_lines INTEGER NOT NULL,
                is_binary INTEGER NOT NULL,
                PRIMARY KEY (commit_id, path),
                FOREIGN KEY (commit_id) REFERENCES commits(id)
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_commits_timestamp ON commits(timestamp)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)",
            [],
        )?;
        self.check_schema_version()?;
        Ok(())
    }

    fn check_schema_version(&mut self) -> Result<()> {
        let version: Option<u32> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| Ok(row.get::<_, String>(0)?.parse().unwrap_or(0)),
            )
            .optional()?;

        match version {
            None => {
                self.conn.execute(
                    "INSERT INTO meta (key, value) VALUES ('schema_version', ?)",
                    params![SCHEMA_VERSION.to_string()],
                )?;
            }
            Some(v) if v != SCHEMA_VERSION => {
                return Err(GmapError::Cache(format!(
                    "Schema version mismatch: expected {SCHEMA_VERSION}, found {v}"
                )));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn get_commit_stats(&self, range: &DateRange) -> Result<Vec<CommitStats>> {
        let mut query = "SELECT c.id, f.path, f.added_lines, f.deleted_lines, f.is_binary
                         FROM commits c
                         LEFT JOIN files f ON c.id = f.commit_id
                         WHERE 1=1"
            .to_string();
        let mut params: Vec<i64> = Vec::new();

        if let Some(since) = &range.since {
            query.push_str(" AND c.timestamp >= ?");
            params.push(since.timestamp());
        }
        if let Some(until) = &range.until {
            query.push_str(" AND c.timestamp <= ?");
            params.push(until.timestamp());
        }
        query.push_str(" ORDER BY c.timestamp");

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            params
                .iter()
                .map(|p| p as &dyn rusqlite::ToSql)
                .collect::<Vec<_>>()
                .as_slice(),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<u32>>(2)?,
                    row.get::<_, Option<u32>>(3)?,
                    row.get::<_, Option<bool>>(4)?,
                ))
            },
        )?;

        let mut commits_map: HashMap<String, Vec<FileStats>> = HashMap::new();

        for row in rows {
            let (commit_id, path, added, deleted, binary) = row?;
            if let (Some(path), Some(added), Some(deleted), Some(binary)) =
                (path, added, deleted, binary)
            {
                commits_map
                    .entry(commit_id)
                    .or_default()
                    .push(FileStats { path, added_lines: added, deleted_lines: deleted, is_binary: binary });
            } else {
                commits_map.entry(commit_id).or_default();
            }
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
        for stats in commits {
            if let Some(info) = infos.get(&stats.commit_id) {
                tx.execute(
                    "INSERT OR REPLACE INTO commits (id, author_name, author_email, message, timestamp, parent_ids)
                     VALUES (?, ?, ?, ?, ?, ?)",
                    params![
                        info.id,
                        info.author_name,
                        info.author_email,
                        info.message,
                        info.timestamp.timestamp(),
                        serde_json::to_string(&info.parent_ids)?
                    ],
                )?;
                tx.execute("DELETE FROM files WHERE commit_id = ?", params![stats.commit_id])?;
                for f in &stats.files {
                    tx.execute(
                        "INSERT INTO files (commit_id, path, added_lines, deleted_lines, is_binary)
                         VALUES (?, ?, ?, ?, ?)",
                        params![
                            stats.commit_id,
                            f.path,
                            f.added_lines,
                            f.deleted_lines,
                            f.is_binary
                        ],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_missing_commits(&self, all_commit_ids: &[String]) -> Result<Vec<String>> {
        if all_commit_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", all_commit_ids.len()).collect::<Vec<_>>().join(",");
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
                let parent_json: String = row.get(5)?;
                let parent_ids: Vec<String> = serde_json::from_str(&parent_json)
                    .map_err(|_| rusqlite::Error::InvalidColumnType(5, "parent_ids".to_string(), rusqlite::types::Type::Text))?;
                Ok(CommitInfo {
                    id: row.get(0)?,
                    author_name: row.get(1)?,
                    author_email: row.get(2)?,
                    message: row.get(3)?,
                    timestamp: DateTime::from_timestamp(ts, 0)
                        .ok_or_else(|| rusqlite::Error::InvalidColumnType(4, "timestamp".to_string(), rusqlite::types::Type::Integer))?,
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
