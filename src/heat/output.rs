use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::model::{HeatBucket, HeatOutput, SCHEMA_VERSION};
use anyhow::Result;
use chrono::Utc;
use console::style;

fn intensity_char<'a>(value: f64, max: f64, symbols: &'a [&str]) -> &'a str {
    if max <= 0.0 {
        return symbols[0];
    }
    let levels = (symbols.len() - 1) as f64;
    let mut level = ((value / max) * levels).round() as usize;
    if level > symbols.len() - 1 {
        level = symbols.len() - 1;
    }
    symbols[level]
}

pub fn output_json(
    heat_data: &[HeatBucket],
    repo: &GitRepo,
    common: &CommonArgs,
    path_prefix: Option<&str>,
) -> Result<()> {
    let output = HeatOutput {
        version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        repository_path: repo.path().display().to_string(),
        path_prefix: path_prefix.unwrap_or_default().to_string(),
        since: common.since.clone(),
        until: common.until.clone(),
        buckets: heat_data.to_vec(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn output_ndjson(heat_data: &[HeatBucket]) -> Result<()> {
    for bucket in heat_data {
        println!("{}", serde_json::to_string(bucket)?);
    }
    Ok(())
}

pub fn output_heatmap(heat_data: &[HeatBucket], common: &CommonArgs) -> Result<()> {
    if heat_data.is_empty() {
        println!("No data to display");
        return Ok(());
    }

    match (&common.since, &common.until) {
        (Some(since), Some(until)) => {
            println!("Filtering commits from {since} to {until}");
        }
        (Some(since), None) => {
            println!("Filtering commits since {since}");
        }
        (None, Some(until)) => {
            println!("Filtering commits until {until}");
        }
        _ => {}
    }

    let max_commits = heat_data.iter().map(|b| b.commit_count).max().unwrap_or(1) as f64;
    let max_lines = heat_data.iter().map(|b| b.lines_changed).max().unwrap_or(1) as f64;

    println!("{}", style("Commit Activity Heatmap").bold());
    println!("{}", "─".repeat(50));

    for bucket in heat_data {
        let commit_char = intensity_char(
            bucket.commit_count as f64,
            max_commits,
            &[" ", "▁", "▃", "▅", "▇", "█"],
        );
        let lines_char = intensity_char(
            bucket.lines_changed as f64,
            max_lines,
            &[" ", "░", "▒", "▓", "█", "█"],
        );

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
