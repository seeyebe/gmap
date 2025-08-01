use crate::cache::Cache;
use crate::tui::WeekStats;
use super::FileExtensionStats;
use crate::model::CommitStats;
use crate::error::{Result, GmapError};
use crate::util::{files_matching, week_key};
use std::collections::HashMap;
use std::path::Path;
use crate::model::HeatBucket;

struct WeekAccum {
    commits: usize,
    added: usize,
    deleted: usize,
    authors: HashMap<String, usize>,
    file_extensions: HashMap<String, FileExtensionStats>,
    file_changes: HashMap<String, usize>,
}

pub fn aggregate_weeks(
    stats: &[CommitStats],
    cache: &Cache,
    path_prefix: Option<&str>,
) -> Vec<WeekStats> {
    let mut week_map: HashMap<String, WeekAccum> = HashMap::new();

    for commit_stats in stats {
        let commit_info = match cache.get_commit_info(&commit_stats.commit_id) {
            Ok(Some(info)) => info,
            _ => continue,
        };

        let week_key = week_key(&commit_info.timestamp);

        let filtered_files: Vec<&crate::model::FileStats> =
            files_matching(&commit_stats.files, path_prefix).collect();

        if filtered_files.is_empty() && path_prefix.is_some() {
            continue;
        }

        let mut added = 0;
        let mut deleted = 0;
        for file_stats in &filtered_files {
            added += file_stats.added_lines as usize;
            deleted += file_stats.deleted_lines as usize;
        }

        let entry = week_map.entry(week_key.clone()).or_insert_with(|| WeekAccum {
            commits: 0,
            added: 0,
            deleted: 0,
            authors: HashMap::new(),
            file_extensions: HashMap::new(),
            file_changes: HashMap::new(),
        });

        entry.commits += 1;
        entry.added += added;
        entry.deleted += deleted;
        *entry.authors.entry(commit_info.author_name.clone()).or_insert(0) += 1;

        for file_stats in &filtered_files {
            let extension = Path::new(&file_stats.path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            let ext_entry = entry
                .file_extensions
                .entry(extension)
                .or_insert(FileExtensionStats {
                    commits: 0,
                    lines_added: 0,
                    lines_deleted: 0,
                    files_changed: 0,
                });
            ext_entry.commits += 1;
            ext_entry.lines_added += file_stats.added_lines as usize;
            ext_entry.lines_deleted += file_stats.deleted_lines as usize;
            ext_entry.files_changed += 1;

            *entry.file_changes.entry(file_stats.path.clone()).or_insert(0) += 1;
        }
    }

    let mut weeks: Vec<WeekStats> = week_map
        .into_iter()
        .map(
            |(week, WeekAccum {
                      commits,
                      added,
                      deleted,
                      authors,
                      file_extensions,
                      file_changes,
                  })| {
                let mut top_authors: Vec<_> = authors.into_iter().collect();
                top_authors.sort_by(|a, b| b.1.cmp(&a.1));
                let top_authors = top_authors.into_iter().map(|(name, _)| name).take(3).collect();

                let mut top_files: Vec<_> = file_changes.into_iter().collect();
                top_files.sort_by(|a, b| b.1.cmp(&a.1));
                let top_files = top_files.into_iter().take(10).collect();

                WeekStats {
                    week,
                    commits,
                    lines_added: added,
                    lines_deleted: deleted,
                    top_authors,
                    file_extensions,
                    top_files,
                }
            },
        )
        .collect();

    weeks.sort_by(|a, b| a.week.cmp(&b.week));
    weeks
}

pub fn compute_heat(
    stats: &[CommitStats],
    cache: &Cache,
    path_prefix: Option<&str>,
) -> Result<Vec<HeatBucket>> {
    let mut week_map: HashMap<String, (u32, u64)> = HashMap::new();

    for commit_stats in stats {
        let commit_info = cache
            .get_commit_info(&commit_stats.commit_id)?
            .ok_or_else(|| GmapError::Cache("Commit info not found".to_string()))?;

        let week_key = week_key(&commit_info.timestamp);

        let mut lines_changed = 0u64;
        let mut has_matching_files = false;

        for file_stats in files_matching(&commit_stats.files, path_prefix) {
            has_matching_files = true;
            lines_changed += (file_stats.added_lines + file_stats.deleted_lines) as u64;
        }

        if has_matching_files || path_prefix.is_none() {
            let entry = week_map.entry(week_key).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += lines_changed;
        }
    }

    let mut buckets: Vec<_> = week_map
        .into_iter()
        .map(|(week, (commit_count, lines_changed))| HeatBucket {
            week,
            commit_count,
            lines_changed,
        })
        .collect();

    buckets.sort_by(|a, b| a.week.cmp(&b.week));
    Ok(buckets)
}