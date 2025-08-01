use crate::tui::{CommitDetail, TuiState, WeekStats};
use crate::cache::Cache;
use crate::model::CommitStats;
use crate::util::{files_matching, week_key};
use std::io;

pub fn get_commits_for_week(
    stats: &[CommitStats],
    cache: &Cache,
    week: &str,
    path_prefix: Option<&str>,
) -> crate::error::Result<Vec<CommitDetail>> {
    let mut commits = Vec::new();

    for commit_stats in stats {
        let commit_info = match cache.get_commit_info(&commit_stats.commit_id) {
            Ok(Some(info)) => info,
            _ => continue,
        };

        let commit_week = week_key(&commit_info.timestamp);
        if commit_week != week {
            continue;
        }

        let mut files_changed = Vec::new();
        let mut lines_added = 0u32;
        let mut lines_deleted = 0u32;
        let mut has_matching_files = false;

        for file_stats in files_matching(&commit_stats.files, path_prefix) {
            has_matching_files = true;
            files_changed.push(file_stats.path.clone());
            lines_added += file_stats.added_lines;
            lines_deleted += file_stats.deleted_lines;
        }

        if has_matching_files || path_prefix.is_none() {
            commits.push(CommitDetail {
                hash: commit_info.id.clone(),
                short_hash: commit_info.id.chars().take(8).collect(),
                message: commit_info
                    .message
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                author_name: commit_info.author_name.clone(),
                author_email: commit_info.author_email.clone(),
                timestamp: commit_info.timestamp,
                files_changed,
                lines_added,
                lines_deleted,
            });
        }
    }

    commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(commits)
}

pub fn load_commit_details(
    state: &mut TuiState,
    weeks: &[WeekStats],
    stats: &[CommitStats],
    cache: &Cache,
    path_prefix: Option<&str>,
) -> io::Result<()> {
    if state.selected >= weeks.len() {
        return Ok(());
    }

    state.loading_commits = true;
    let selected_week = &weeks[state.selected];

    match get_commits_for_week(stats, cache, &selected_week.week, path_prefix) {
        Ok(commits) => {
            state.commit_details = commits;
            state.commit_selected = 0;
            state.loading_commits = false;
        }
        Err(e) => {
            eprintln!("Error loading commits: {}", e);
            state.loading_commits = false;
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }
    }

    Ok(())
}
