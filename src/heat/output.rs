use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::model::{HeatBucket, HeatOutput, SCHEMA_VERSION};
use chrono::Utc;
use console::style;
use anyhow::Result;

pub fn output_json(
    heat_data: &[HeatBucket],
    repo: &GitRepo,
    common: &CommonArgs,
    path_prefix: Option<&str>,
) -> Result<()> {
    let output = HeatOutput {
        version: SCHEMA_VERSION,
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
