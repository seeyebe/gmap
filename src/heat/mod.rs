pub mod aggregate;
pub mod commit;
pub mod exec;
pub mod fetch;
pub mod output;

pub use aggregate::{aggregate_weeks, compute_heat};
pub use commit::{get_commits_for_period, load_commit_details};
pub use exec::exec;
pub use fetch::{fetch_commit_stats, fetch_commit_stats_with_progress};
pub use output::{output_heatmap, output_json, output_ndjson};

#[derive(Clone, Debug)]
pub struct FileExtensionStats {
    pub commits: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub files_changed: usize,
}
