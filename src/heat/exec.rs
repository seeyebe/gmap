use crate::cli::CommonArgs;
use crate::cache::Cache;
use crate::git::GitRepo;
use anyhow::Context;
use super::{
    fetch_commit_stats,
    compute_heat,
    output_json,
    output_ndjson,
    output_heatmap,
};

pub fn exec(common: CommonArgs, json: bool, ndjson: bool, path: Option<String>) -> anyhow::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).context("Failed to open git repository")?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path()).context("Failed to initialize cache")?;

    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .context("Failed to resolve date range")?;

    let all_stats = fetch_commit_stats(&repo, &mut cache, &range, common.include_merges, common.binary)?;

    let heat_data = compute_heat(&all_stats, &cache, path.as_deref())
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
