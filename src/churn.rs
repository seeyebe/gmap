use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{ChurnEntry, ChurnOutput};
use anyhow::Context;
use chrono::Utc;
use console::style;
use std::collections::HashMap;

pub fn exec(common: CommonArgs, depth: Option<u32>, json: bool, ndjson: bool, path: Option<String>) -> anyhow::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).context("Failed to open git repository")?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path()).context("Failed to initialize cache")?;

    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .context("Failed to resolve date range")?;

    let cached = cache
        .get_commit_stats(&range)
        .context("Failed to get cached commit stats")?;

    let repo_stats = repo
        .collect_commits(&range, common.include_merges, common.binary,)
        .context("Failed to collect commits from repository")?;

    let missing: Vec<_> = repo_stats
        .iter()
        .filter(|s| !cached.iter().any(|c| c.commit_id == s.commit_id))
        .collect();

    if !missing.is_empty() {
        let mut infos: HashMap<_, _> = HashMap::new();
        for s in &missing {
            if let Ok(info) = repo.get_commit_info(&s.commit_id) {
                infos.insert(s.commit_id.clone(), info);
            }
        }
        cache
            .store_commit_stats(&missing.iter().map(|&s| s.clone()).collect::<Vec<_>>(), &infos)
            .context("Failed to store commit stats in cache")?;
    }

    let all_stats = cache
        .get_commit_stats(&range)
        .context("Failed to get final commit stats")?;

    let churn = compute_churn(&all_stats, &cache, depth, path.as_deref())
        .context("Failed to compute churn statistics")?;

    if json {
        output_json(&churn, &repo, &common)?;
    } else if ndjson {
        output_ndjson(&churn)?;
    } else {
        output_table(&churn)?;
    }

    Ok(())
}

fn compute_churn(
    stats: &[crate::model::CommitStats],
    cache: &Cache,
    depth: Option<u32>,
    path_prefix: Option<&str>,
) -> Result<Vec<ChurnEntry>> {
    let mut map: HashMap<String, ChurnEntry> = HashMap::new();
    for cs in stats {
        let info = cache
            .get_commit_info(&cs.commit_id)?
            .ok_or_else(|| crate::error::GmapError::Cache("Commit info not found".to_string()))?;

        for f in &cs.files {
            if let Some(prefix) = path_prefix {
                if !f.path.starts_with(prefix) {
                    continue;
                }
            }
            let agg = if let Some(d) = depth { aggregate_path(&f.path, d) } else { f.path.clone() };
            let entry = map.entry(agg.clone()).or_insert_with(|| ChurnEntry::new(agg));
            entry.add_stats(f, &info.author_name);
        }
    }
    let mut entries: Vec<_> = map.into_values().collect();
    entries.sort_by(|a, b| b.total_lines.cmp(&a.total_lines));
    Ok(entries)
}

fn aggregate_path(path: &str, depth: u32) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if depth == 0 || parts.len() <= depth as usize {
        path.to_string()
    } else {
        parts[..depth as usize].join("/")
    }
}

fn output_json(churn_data: &[ChurnEntry], repo: &GitRepo, common: &CommonArgs) -> anyhow::Result<()> {
    let output = ChurnOutput {
        version: crate::model::SCHEMA_VERSION,
        generated_at: Utc::now(),
        repository_path: repo.path().to_string_lossy().to_string(),
        since: common.since.clone(),
        until: common.until.clone(),
        depth: None,
        entries: churn_data.to_vec(),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn output_ndjson(churn_data: &[ChurnEntry]) -> anyhow::Result<()> {
    for e in churn_data {
        println!("{}", serde_json::to_string(e)?);
    }
    Ok(())
}

fn output_table(churn_data: &[ChurnEntry]) -> anyhow::Result<()> {
    println!("{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
             style("Path").bold(),
             style("Added").bold(),
             style("Deleted").bold(),
             style("Total").bold(),
             style("Commits").bold(),
             style("Authors").bold());
    println!("{}", "─".repeat(98));
    for e in churn_data.iter().take(50) {
        println!("{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
                 e.path,
                 e.added_lines,
                 e.deleted_lines,
                 e.total_lines,
                 e.commit_count,
                 e.authors.len());
    }
    if churn_data.len() > 50 {
        println!("\n... and {} more entries", churn_data.len() - 50);
    }
    Ok(())
}
