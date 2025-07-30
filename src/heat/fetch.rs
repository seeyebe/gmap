use crate::cache::Cache;
use crate::git::GitRepo;
use crate::model::CommitStats;
use anyhow::Context;

pub fn fetch_commit_stats(repo: &GitRepo, cache: &mut Cache, range: &crate::model::DateRange, include_merges: bool, binary: bool) -> anyhow::Result<Vec<CommitStats>> {
    let cached_stats = cache
        .get_commit_stats(&range)
        .context("Failed to get cached commit stats")?;

    let repo_stats = repo
        .collect_commits(&range, include_merges, binary)
        .context("Failed to collect commits from repository")?;

    let missing_commits: Vec<_> = repo_stats
        .iter()
        .filter(|stats| !cached_stats.iter().any(|c| c.commit_id == stats.commit_id))
        .collect();

    if !missing_commits.is_empty() {
        let mut commit_infos = std::collections::HashMap::new();
        for stats in &missing_commits {
            if let Ok(info) = repo.get_commit_info(&stats.commit_id) {
                commit_infos.insert(stats.commit_id.clone(), info);
            }
        }
        cache
            .store_commit_stats(
                &missing_commits.iter().map(|&s| s.clone()).collect::<Vec<_>>(),
                &commit_infos,
            )
            .context("Failed to store commit stats in cache")?;
    }

    let all_stats = cache
        .get_commit_stats(&range)
        .context("Failed to get final commit stats")?;
    Ok(all_stats)
}
