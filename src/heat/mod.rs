pub mod fetch;
pub mod aggregate;
pub mod output;
pub mod commit;
pub mod exec;

pub use fetch::fetch_commit_stats;
pub use aggregate::{aggregate_weeks, compute_heat};
pub use output::{output_json, output_ndjson, output_heatmap};
pub use commit::{get_commits_for_week, load_commit_details};
pub use exec::exec;

#[derive(Clone, Debug)]
pub struct FileExtensionStats {
    pub commits: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub files_changed: usize,
}
