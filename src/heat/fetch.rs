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
    fetch_commit_stats_with_progress(repo, cache, range, include_merges, binary, true)
}

pub fn fetch_commit_stats_with_progress(
    repo: &GitRepo,
    cache: &mut Cache,
    range: &DateRange,
    include_merges: bool,
    binary: bool,
    _progress: bool,
) -> anyhow::Result<Vec<CommitStats>> {
    let mut cached_stats = cache
        .get_commit_stats(range)
        .context("Failed to get cached commit stats")?;

    let existing_ids: HashSet<&str> = cached_stats.iter().map(|c| c.commit_id.as_str()).collect();

    let repo_ids: Vec<gix::ObjectId> = repo
        .list_commit_ids(range, include_merges)
        .context("Failed to list commits from repository")?;

    let mut missing_stats: Vec<CommitStats> = Vec::new();
    for oid in repo_ids {
        let id_str = oid.to_string();
        if existing_ids.contains(id_str.as_str()) {
            continue;
        }
        let stats = repo
            .compute_commit_stats_for(oid, binary)
            .context("Failed to compute commit stats for missing commit")?;
        missing_stats.push(stats);
    }

    if !missing_stats.is_empty() {
        let mut commit_infos = std::collections::HashMap::new();
        for stats in &missing_stats {
            if let Ok(info) = repo.get_commit_info(&stats.commit_id) {
                commit_infos.insert(stats.commit_id.clone(), info);
            }
        }
        cache
            .store_commit_stats(&missing_stats, &commit_infos)
            .context("Failed to store commit stats in cache")?;
        cached_stats.extend(missing_stats);
    }

    Ok(cached_stats)
}
