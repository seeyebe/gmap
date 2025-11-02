use crate::cli::CommonArgs;
use crate::cache::Cache;
use crate::git::GitRepo;
use anyhow::Context;
use super::{fetch_commit_stats_with_progress, compute_heat, output_json, output_ndjson, output_heatmap};
use std::cell::RefCell;

pub fn exec(common: CommonArgs, json: bool, ndjson: bool, path: Option<String>, monthly: bool) -> anyhow::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).context("Failed to open git repository")?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path()).context("Failed to initialize cache")?;

    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .context("Failed to resolve date range")?;

    // Disable progress indicators in CLI to keep output clean in JSON/NDJSON
    let all_stats = fetch_commit_stats_with_progress(
        &repo,
        &mut cache,
        &range,
        common.include_merges,
        common.binary,
        false,
    )?;

    let gi = RefCell::new(crate::util::GitIgnoreMatcher::new(repo.path()));
    let heat_data = compute_heat(
        &all_stats,
        &cache,
        path.as_deref(),
        common.author.as_deref(),
        common.author_email.as_deref(),
        monthly,
        &common.exclude,
        Some(&gi),
    )
        .context("Failed to compute heat statistics")?;

    if json {
        output_json(&heat_data, &repo, &common, path.as_deref())?;
    } else if ndjson {
        output_ndjson(&heat_data)?;
    } else {
        output_heatmap(&heat_data, &common)?;
    }

    Ok(())
}
