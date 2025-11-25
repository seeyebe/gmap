use crate::heat::FileExtensionStats;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub type TopFile = (String, usize);

#[derive(Clone, Debug)]
pub struct WeekStats {
    pub week: String,
    pub commits: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub top_authors: Vec<String>,
    pub file_extensions: HashMap<String, FileExtensionStats>,
    pub top_files: Vec<TopFile>,
}

#[derive(Default, Clone, Debug)]
pub struct CommitDetail {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: DateTime<Utc>,
    pub files_changed: Vec<String>,
    pub lines_added: u32,
    pub lines_deleted: u32,
}

impl CommitDetail {
    // Intentionally minimal; constructed directly by heat::commit
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Heatmap,
    Statistics,
    Timeline,
    CommitDetails,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FocusPane {
    Periods,
    Commits,
}

#[derive(Clone, Debug)]
pub struct TuiState {
    pub selected: usize,
    pub view_mode: ViewMode, // kept for compatibility; unused in dashboard
    pub tab_index: usize,    // kept for compatibility; unused in dashboard
    pub show_all: bool,
    pub focus: FocusPane,
    pub show_help: bool,
    pub show_file_modal: bool,
    pub search_query: String,
    pub search_mode: bool,
    pub filtered_indices: Vec<usize>,
    pub commit_search_query: String,
    pub commit_search_mode: bool,
    pub commit_filtered_indices: Vec<usize>,
    pub path_filter: Option<String>,
    pub path_mode: bool,
    pub path_input: String,
    pub commit_details: Vec<CommitDetail>,
    pub commit_selected: usize,
    pub loading_commits: bool,
    pub status_message: Option<(String, std::time::Instant)>,
    pub last_refresh: Option<std::time::Instant>,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            selected: 0,
            view_mode: ViewMode::Heatmap,
            tab_index: 0,
            show_all: false,
            focus: FocusPane::Periods,
            show_help: false,
            show_file_modal: false,
            search_query: String::new(),
            search_mode: false,
            filtered_indices: Vec::new(),
            commit_search_query: String::new(),
            commit_search_mode: false,
            commit_filtered_indices: Vec::new(),
            path_filter: None,
            path_mode: false,
            path_input: String::new(),
            commit_details: Vec::new(),
            commit_selected: 0,
            loading_commits: false,
            status_message: None,
            last_refresh: None,
        }
    }
}
