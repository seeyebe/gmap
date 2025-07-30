use std::collections::HashMap;
use crate::heat::FileExtensionStats;

pub struct WeekStats {
    pub week: String,
    pub commits: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub top_authors: Vec<String>,
    pub file_extensions: HashMap<String, FileExtensionStats>,
    pub top_files: Vec<(String, usize)>,
}

#[derive(Clone, Debug)]
pub struct CommitDetail {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
    pub files_changed: Vec<String>,
    pub lines_added: u32,
    pub lines_deleted: u32,
}

pub struct DetailedWeekStats {
    pub base: WeekStats,
    pub file_types: HashMap<String, usize>,
    pub hourly_distribution: Vec<usize>,
    pub daily_distribution: Vec<usize>,
    pub commit_messages: Vec<String>,
    pub largest_commit: Option<(usize, String)>,
}

pub struct TuiState {
    pub selected: usize,
    pub view_mode: ViewMode,
    pub tab_index: usize,
    pub show_help: bool,
    pub search_query: String,
    pub search_mode: bool,
    pub filtered_indices: Vec<usize>,
    pub commit_details: Vec<CommitDetail>,
    pub commit_selected: usize,
    pub loading_commits: bool,
    pub status_message: Option<(String, std::time::Instant)>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Heatmap,
    Statistics,
    FileTypes,
    Timeline,
    CommitDetails,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            selected: 0,
            view_mode: ViewMode::Heatmap,
            tab_index: 0,
            show_help: false,
            search_query: String::new(),
            search_mode: false,
            filtered_indices: Vec::new(),
            commit_details: Vec::new(),
            commit_selected: 0,
            loading_commits: false,
            status_message: None,
        }
    }
}