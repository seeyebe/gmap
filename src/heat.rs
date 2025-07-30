use super::tui::WeekStats;
use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{HeatBucket, HeatOutput};
use anyhow::Context;
use chrono::{DateTime, Datelike, Utc};
use console::style;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FileExtensionStats {
    pub commits: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub files_changed: usize,
}
pub fn exec(common: CommonArgs, json: bool, ndjson: bool, path: Option<String>) -> anyhow::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).context("Failed to open git repository")?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path()).context("Failed to initialize cache")?;

    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .context("Failed to resolve date range")?;

    let cached_stats = cache
        .get_commit_stats(&range)
        .context("Failed to get cached commit stats")?;

    let repo_stats = repo
        .collect_commits(&range, common.include_merges, common.binary)
        .context("Failed to collect commits from repository")?;

    let missing_commits: Vec<_> = repo_stats
        .iter()
        .filter(|stats| !cached_stats.iter().any(|c| c.commit_id == stats.commit_id))
        .collect();

    if !missing_commits.is_empty() {
        let mut commit_infos = HashMap::new();
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

fn compute_heat(
    stats: &[crate::model::CommitStats],
    cache: &Cache,
    path_prefix: Option<&str>,
) -> Result<Vec<HeatBucket>> {
    let mut week_map: HashMap<String, (u32, u64)> = HashMap::new();

    for commit_stats in stats {
        let commit_info = cache
            .get_commit_info(&commit_stats.commit_id)?
            .ok_or_else(|| crate::error::GmapError::Cache("Commit info not found".to_string()))?;

        let week_key = get_week_key(&commit_info.timestamp);

        let mut lines_changed = 0u64;
        let mut has_matching_files = false;

        for file_stats in &commit_stats.files {
            if let Some(prefix) = path_prefix {
                if !file_stats.path.starts_with(prefix) {
                    continue;
                }
            }
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

fn get_week_key(timestamp: &DateTime<Utc>) -> String {
    format!("{}-W{:02}", timestamp.year(), timestamp.iso_week().week())
}

fn output_json(
    heat_data: &[HeatBucket],
    repo: &GitRepo,
    common: &CommonArgs,
    path_prefix: Option<&str>,
) -> anyhow::Result<()> {
    let output = HeatOutput {
        version: crate::model::SCHEMA_VERSION,
        generated_at: Utc::now(),
        repository_path: repo.path().to_string_lossy().to_string(),
        path_prefix: path_prefix.unwrap_or("").to_string(),
        since: common.since.clone(),
        until: common.until.clone(),
        buckets: heat_data.to_vec(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn output_ndjson(heat_data: &[HeatBucket]) -> anyhow::Result<()> {
    for bucket in heat_data {
        println!("{}", serde_json::to_string(bucket)?);
    }
    Ok(())
}

fn output_heatmap(heat_data: &[HeatBucket], common: &CommonArgs) -> anyhow::Result<()> {
    if heat_data.is_empty() {
        println!("No data to display");
        return Ok(());
    }

    if let (Some(since), Some(until)) = (&common.since, &common.until) {
        println!("Filtering commits from {} to {}", since, until);
    } else if let Some(since) = &common.since {
        println!("Filtering commits since {}", since);
    } else if let Some(until) = &common.until {
        println!("Filtering commits until {}", until);
    }

    let max_commits = heat_data.iter().map(|b| b.commit_count).max().unwrap_or(1);
    let max_lines = heat_data.iter().map(|b| b.lines_changed).max().unwrap_or(1);

    println!("{}", style("Commit Activity Heatmap").bold());
    println!("{}", "─".repeat(50));

    for bucket in heat_data {
        let commit_intensity = ((bucket.commit_count as f64 / max_commits as f64) * 5.0) as u32;
        let lines_intensity = ((bucket.lines_changed as f64 / max_lines as f64) * 5.0) as u32;

        let commit_char = match commit_intensity {
            0 => " ",
            1 => "▁",
            2 => "▃",
            3 => "▅",
            4 => "▇",
            _ => "█",
        };

        let lines_char = match lines_intensity {
            0 => " ",
            1 => "░",
            2 => "▒",
            3 => "▓",
            4 => "█",
            _ => "█",
        };

        println!(
            "{} {} {} commits: {:>3}, lines: {:>6}",
            bucket.week,
            style(commit_char).green(),
            style(lines_char).blue(),
            bucket.commit_count,
            bucket.lines_changed
        );
    }

    println!("\n{}", style("Legend").bold());
    println!("  {} commits intensity", style("▁▃▅▇█").green());
    println!("  {} lines intensity", style("░▒▓█").blue());

    Ok(())
}

pub fn aggregate_weeks(stats: &[crate::model::CommitStats], cache: &Cache, path_prefix: Option<&str>) -> Vec<WeekStats> {
    use std::collections::HashMap;
    use chrono::Datelike;
    use std::path::Path;

    let mut week_map: HashMap<String, (usize, usize, usize, HashMap<String, usize>, HashMap<String, FileExtensionStats>, HashMap<String, usize>)> = HashMap::new();

    for commit_stats in stats {
        let commit_info = match cache.get_commit_info(&commit_stats.commit_id) {
            Ok(Some(info)) => info,
            _ => continue,
        };

        let week_key = format!("{}-W{:02}", commit_info.timestamp.year(), commit_info.timestamp.iso_week().week());
        let mut added = 0usize;
        let mut deleted = 0usize;
        let mut has_matching_files = false;

        for file_stats in &commit_stats.files {
            if let Some(prefix) = path_prefix {
                if !file_stats.path.starts_with(prefix) {
                    continue;
                }
            }
            has_matching_files = true;
            added += file_stats.added_lines as usize;
            deleted += file_stats.deleted_lines as usize;
        }

        if has_matching_files || path_prefix.is_none() {
            let entry = week_map.entry(week_key.clone()).or_insert((0, 0, 0, HashMap::new(), HashMap::new(), HashMap::new()));
            entry.0 += 1; // commits
            entry.1 += added; // lines_added
            entry.2 += deleted; // lines_deleted
            *entry.3.entry(commit_info.author_name.clone()).or_insert(0) += 1; // authors

            for file_stats in &commit_stats.files {
                if let Some(prefix) = path_prefix {
                    if !file_stats.path.starts_with(prefix) {
                        continue;
                    }
                }

                let extension = Path::new(&file_stats.path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let ext_entry = entry.4.entry(extension).or_insert(FileExtensionStats {
                    commits: 0,
                    lines_added: 0,
                    lines_deleted: 0,
                    files_changed: 0,
                });
                ext_entry.commits += 1;
                ext_entry.lines_added += file_stats.added_lines as usize;
                ext_entry.lines_deleted += file_stats.deleted_lines as usize;
                ext_entry.files_changed += 1;

                *entry.5.entry(file_stats.path.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut weeks: Vec<WeekStats> = week_map.into_iter().map(|(week, (commits, added, deleted, authors, file_extensions, file_changes))| {
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
    }).collect();

    weeks.sort_by(|a, b| a.week.cmp(&b.week));
    weeks
}