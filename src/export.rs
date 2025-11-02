use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::error::Result;
use crate::git::GitRepo;
use crate::heat::fetch_commit_stats_with_progress;
use crate::model::{ExportEntry, ExportOutput, CommitStats};
use anyhow::Context;
use chrono::Utc;
use std::collections::HashSet;

pub fn exec(common: CommonArgs, json: bool, ndjson: bool) -> anyhow::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref())
        .context("Failed to open git repository")?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path())
        .context("Failed to initialize cache")?;

    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .context("Failed to resolve date range")?;

    let cached_stats = fetch_commit_stats_with_progress(
        &repo,
        &mut cache,
        &range,
        common.include_merges,
        common.binary,
        false,
    )?;

    let export_data = prepare_export_data(
        &cached_stats,
        &cache,
        common.author.as_deref(),
        common.author_email.as_deref(),
    )
        .context("Failed to prepare export data")?;

    if json {
        output_json(&export_data, &repo, &common)?;
    } else if ndjson {
        output_ndjson(&export_data)?;
    } else {
        output_summary(&export_data)?;
    }

    Ok(())
}

fn prepare_export_data(
    stats: &[CommitStats],
    cache: &Cache,
    author: Option<&str>,
    author_email: Option<&str>,
) -> Result<Vec<ExportEntry>> {
    let mut entries = Vec::with_capacity(stats.len());

    for commit_stats in stats {
        let commit_info = cache
            .get_commit_info(&commit_stats.commit_id)?
            .ok_or_else(|| crate::error::GmapError::Cache("Commit info not found".to_string()))?;

        if let Some(a) = author {
            if !commit_info.author_name.to_lowercase().contains(&a.to_lowercase()) { continue; }
        }
        if let Some(ae) = author_email {
            if !commit_info.author_email.to_lowercase().contains(&ae.to_lowercase()) { continue; }
        }

        entries.push(ExportEntry {
            commit_id: commit_info.id,
            author_name: commit_info.author_name,
            author_email: commit_info.author_email,
            timestamp: commit_info.timestamp,
            message: commit_info.message,
            files: commit_stats.files.clone(),
        });
    }

    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(entries)
}

fn output_json(export_data: &[ExportEntry], repo: &GitRepo, common: &CommonArgs) -> anyhow::Result<()> {
    let output = ExportOutput {
        version: crate::model::SCHEMA_VERSION,
        generated_at: Utc::now(),
        repository_path: repo.path().to_string_lossy().to_string(),
        since: common.since.clone(),
        until: common.until.clone(),
        entries: export_data.to_vec(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn output_ndjson(export_data: &[ExportEntry]) -> anyhow::Result<()> {
    for entry in export_data {
        println!("{}", serde_json::to_string(entry)?);
    }
    Ok(())
}

fn output_summary(export_data: &[ExportEntry]) -> anyhow::Result<()> {
    use console::style;

    println!("{}", style("Export Summary").bold());
    println!("{}", "─".repeat(50));

    let total_commits = export_data.len();
    let total_files: usize = export_data.iter().map(|e| e.files.len()).sum();
    let total_added: u64 = export_data
        .iter()
        .flat_map(|e| &e.files)
        .map(|f| f.added_lines as u64)
        .sum();
    let total_deleted: u64 = export_data
        .iter()
        .flat_map(|e| &e.files)
        .map(|f| f.deleted_lines as u64)
        .sum();

    let unique_authors: HashSet<_> =
        export_data.iter().map(|e| &e.author_name).collect();

    println!("Total commits: {}", style(total_commits).cyan());
    println!("Total files changed: {}", style(total_files).cyan());
    println!("Total lines added: {}", style(total_added).green());
    println!("Total lines deleted: {}", style(total_deleted).red());
    println!("Unique authors: {}", style(unique_authors.len()).yellow());

    if !export_data.is_empty() {
        let first_commit = &export_data[0];
        let last_commit = &export_data[export_data.len() - 1];
        println!(
            "Date range: {} to {}",
            style(first_commit.timestamp.format("%Y-%m-%d")).dim(),
            style(last_commit.timestamp.format("%Y-%m-%d")).dim()
        );
    }

    println!("\nUse --json or --ndjson flags to export the raw data.");
    Ok(())
}
