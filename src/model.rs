use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub author_name: String,
    pub author_email: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub parent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    pub path: String,
    pub added_lines: u32,
    pub deleted_lines: u32,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStats {
    pub commit_id: String,
    pub files: Vec<FileStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnEntry {
    pub path: String,
    pub added_lines: u64,
    pub deleted_lines: u64,
    pub total_lines: u64,
    pub commit_count: u32,
    pub authors: HashSet<String>,
}

impl ChurnEntry {
    pub fn new(path: String) -> Self {
        Self {
            path,
            added_lines: 0,
            deleted_lines: 0,
            total_lines: 0,
            commit_count: 0,
            authors: HashSet::new(),
        }
    }

    pub fn add_stats(&mut self, stats: &FileStats, author: &str) {
        self.added_lines += stats.added_lines as u64;
        self.deleted_lines += stats.deleted_lines as u64;
        self.total_lines += (stats.added_lines + stats.deleted_lines) as u64;
        self.commit_count += 1;
        if self.authors.len() < 100 {
            self.authors.insert(author.to_string());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnOutput {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub repository_path: String,
    pub since: Option<String>,
    pub until: Option<String>,
    pub depth: Option<u32>,
    pub entries: Vec<ChurnEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatBucket {
    pub week: String,
    pub commit_count: u32,
    pub lines_changed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatOutput {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub repository_path: String,
    pub path_prefix: String,
    pub since: Option<String>,
    pub until: Option<String>,
    pub buckets: Vec<HeatBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub commit_id: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub files: Vec<FileStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOutput {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub repository_path: String,
    pub since: Option<String>,
    pub until: Option<String>,
    pub entries: Vec<ExportEntry>,
}

#[derive(Debug, Clone)]
pub struct DateRange {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

impl DateRange {
    pub fn new() -> Self {
        Self { since: None, until: None }
    }

    pub fn with_since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn with_until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }

    pub fn contains(&self, timestamp: &DateTime<Utc>) -> bool {
        if let Some(since) = self.since {
            if timestamp < &since {
                return false;
            }
        }
        if let Some(until) = self.until {
            if timestamp > &until {
                return false;
            }
        }
        true
    }
}

impl Default for DateRange {
    fn default() -> Self {
        Self::new()
    }
}
