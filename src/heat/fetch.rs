use crate::cache::Cache;
use crate::git::GitRepo;
use crate::model::{CommitStats, DateRange};
use anyhow::Context;
use std::collections::HashSet;

pub fn fetch_commit_stats(
    repo: &GitRepo,
    cache: &mut Cache,
    range: &DateRange,
    include_merges: bool,
    binary: bool,
) -> anyhow::Result<Vec<CommitStats>> {
    let mut cached_stats = cache
        .get_commit_stats(range)
        .context("Failed to get cached commit stats")?;

    let repo_stats = repo
        .collect_commits(range, include_merges, binary)
        .context("Failed to collect commits from repository")?;

    let existing_ids: HashSet<&str> = cached_stats.iter().map(|c| c.commit_id.as_str()).collect();

    let missing_commits: Vec<CommitStats> = repo_stats
        .into_iter()
        .filter(|stats| !existing_ids.contains(stats.commit_id.as_str()))
        .collect();

    if !missing_commits.is_empty() {
        let mut commit_infos = std::collections::HashMap::new();
        for stats in &missing_commits {
            if let Ok(info) = repo.get_commit_info(&stats.commit_id) {
                commit_infos.insert(stats.commit_id.clone(), info);
            }
        }
        cache
            .store_commit_stats(&missing_commits, &commit_infos)
            .context("Failed to store commit stats in cache")?;
        cached_stats.extend(missing_commits.into_iter());
    }

    Ok(cached_stats)
}
