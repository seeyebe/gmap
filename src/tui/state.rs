use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::heat::FileExtensionStats;

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

impl Default for WeekStats {
    fn default() -> Self {
        Self {
            week: String::new(),
            commits: 0,
            lines_added: 0,
            lines_deleted: 0,
            top_authors: Vec::new(),
            file_extensions: HashMap::new(),
            top_files: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
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
    pub fn from_hash(hash: String, message: String, author_name: String, author_email: String, timestamp: DateTime<Utc>, files_changed: Vec<String>, lines_added: u32, lines_deleted: u32) -> Self {
        let short_hash = hash.chars().take(8).collect();
        Self {
            hash,
            short_hash,
            message,
            author_name,
            author_email,
            timestamp,
            files_changed,
            lines_added,
            lines_deleted,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DetailedWeekStats {
    pub base: WeekStats,
    pub file_types: HashMap<String, usize>,
    pub hourly_distribution: Vec<usize>,
    pub daily_distribution: Vec<usize>,
    pub commit_messages: Vec<String>,
    pub largest_commit: Option<(usize, String)>,
}

impl Default for DetailedWeekStats {
    fn default() -> Self {
        Self {
            base: WeekStats::default(),
            file_types: HashMap::new(),
            hourly_distribution: Vec::new(),
            daily_distribution: Vec::new(),
            commit_messages: Vec::new(),
            largest_commit: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Heatmap,
    Statistics,
    Timeline,
    CommitDetails,
}

#[derive(Clone, Debug)]
pub struct TuiState {
    pub selected: usize,
    pub view_mode: ViewMode,
    pub tab_index: usize,
    pub show_help: bool,
    pub show_file_modal: bool,
    pub search_query: String,
    pub search_mode: bool,
    pub filtered_indices: Vec<usize>,
    pub commit_details: Vec<CommitDetail>,
    pub commit_selected: usize,
    pub loading_commits: bool,
    pub status_message: Option<(String, std::time::Instant)>,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            selected: 0,
            view_mode: ViewMode::Heatmap,
            tab_index: 0,
            show_help: false,
            show_file_modal: false,
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
