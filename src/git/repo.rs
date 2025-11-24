use crate::error::{GmapError, Result};
use crate::model::{CommitInfo, CommitStats, DateRange, FileStats};
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, TimeZone, Utc};
use gix::object::tree::diff::ChangeDetached;
use gix::{discover, ObjectId, Repository};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct CommitMeta {
    timestamp: DateTime<Utc>,
    author_name: String,
    author_email: String,
    message_title: String,
    parent_ids: Vec<ObjectId>,
}

pub struct GitRepo {
    repo: Repository,
    path: PathBuf,
}

impl GitRepo {
    /// Open a repository at `path`, or current dir if `None`
    pub fn open<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let repo_path = path
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or(std::env::current_dir()?);

        let repo = discover(&repo_path)?;
        let path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();

        Ok(Self { repo, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn resolve_range(&self, since: Option<&str>, until: Option<&str>) -> Result<DateRange> {
        let mut range = DateRange::new();
        let since_dt = since.map(|s| self.parse_commit_or_date(s)).transpose()?;
        let until_dt = until.map(|u| self.parse_commit_or_date(u)).transpose()?;
        if let (Some(s), Some(u)) = (since_dt, until_dt) {
            if s > u {
                return Err(GmapError::InvalidDate(format!(
                    "Invalid range: since ({s}) is after until ({u})"
                )));
            }
        }

        if let Some(s) = since_dt {
            range = range.with_since(s);
        }
        if let Some(u) = until_dt {
            range = range.with_until(u);
        }

        Ok(range)
    }

    fn parse_commit_or_date(&self, input: &str) -> Result<DateTime<Utc>> {
        // RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
            return Ok(dt.with_timezone(&Utc));
        }

        // YYYY-MM-DD
        if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
            if let Some(datetime) = date.and_hms_opt(0, 0, 0) {
                return Ok(Utc.from_utc_datetime(&datetime));
            }
        }

        // Relative duration (e.g., "2 weeks ago")
        if let Some(duration) = parse_natural_duration(input) {
            let now = Utc::now();
            let target = now - duration;
            return Ok(target);
        }

        // Fallback to Git ref
        let id = self
            .repo
            .rev_parse_single(input)
            .map_err(|e| GmapError::Parse(format!("Invalid commit or date '{input}': {e}")))?;

        let commit = id
            .object()?
            .try_into_commit()
            .map_err(|_| GmapError::Parse(format!("Not a commit: {input}")))?;

        let secs = commit.time()?.seconds;
        Utc.timestamp_opt(secs, 0)
            .single()
            .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))
    }

    pub fn collect_commits(
        &self,
        range: &DateRange,
        include_merges: bool,
        binary: bool,
        progress: bool,
    ) -> Result<Vec<CommitStats>> {
        let mut head = self.repo.head()?;
        let head_commit = head.peel_to_commit_in_place()?;

        let mut commits = Vec::new();
        let mut seen: HashSet<ObjectId> = HashSet::new();
        let mut stack: VecDeque<ObjectId> = VecDeque::from([head_commit.id]);
        let mut commit_cache: HashMap<ObjectId, CommitMeta> = HashMap::new();
        let pb = if progress {
            ProgressBar::new_spinner()
        } else {
            ProgressBar::hidden()
        };
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.set_message("Collecting commits...");

        while let Some(commit_id) = stack.pop_back() {
            if !seen.insert(commit_id) {
                continue;
            }

            let meta = if let Some(cached) = commit_cache.get(&commit_id) {
                cached.clone()
            } else {
                let commit = self.repo.find_commit(commit_id)?;
                let secs = commit.time()?.seconds;
                let timestamp = Utc
                    .timestamp_opt(secs, 0)
                    .single()
                    .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))?;
                let author = commit.author()?;
                let message = commit.message()?;
                let parents: Vec<ObjectId> = commit.parent_ids().map(|id| id.into()).collect();
                let entry = CommitMeta {
                    timestamp,
                    author_name: author.name.to_string(),
                    author_email: author.email.to_string(),
                    message_title: message.title.to_string(),
                    parent_ids: parents.clone(),
                };
                commit_cache.insert(commit_id, entry.clone());
                entry
            };

            let timestamp = meta.timestamp;
            let parents = meta.parent_ids.clone();

            if !range.contains(&timestamp) {
                for pid in &parents {
                    stack.push_back(*pid);
                }
                pb.inc(1);
                continue;
            }

            if !include_merges && parents.len() > 1 {
                for pid in &parents {
                    stack.push_back(*pid);
                }
                pb.inc(1);
                continue;
            }

            let commit_info = CommitInfo {
                id: commit_id.to_string(),
                author_name: meta.author_name.clone(),
                author_email: meta.author_email.clone(),
                message: meta.message_title.clone(),
                timestamp,
                parent_ids: parents.iter().map(|id| id.to_string()).collect(),
            };

            let stats = if let Some(parent_id) = parents.first() {
                self.compute_commit_stats(&commit_info, commit_id, Some(*parent_id), binary)?
            } else {
                self.compute_commit_stats(&commit_info, commit_id, None, binary)?
            };

            commits.push(stats);
            for pid in &parents {
                stack.push_back(*pid);
            }

            pb.inc(1);
        }

        pb.finish_with_message("Commits collected");
        Ok(commits)
    }

    fn compute_commit_stats(
        &self,
        commit_info: &CommitInfo,
        commit_id: ObjectId,
        parent_id: Option<ObjectId>,
        binary: bool,
    ) -> Result<CommitStats> {
        let commit_tree = self.repo.find_commit(commit_id)?.tree()?;
        let parent_tree = if let Some(pid) = parent_id {
            Some(self.repo.find_commit(pid)?.tree()?)
        } else {
            None
        };
        let changes: Vec<ChangeDetached> =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;
        let mut files = Vec::new();
        for change in changes {
            if let Err(err) = self.handle_change(change, binary, &mut files) {
                eprintln!("Warning: {err}");
            }
        }

        Ok(CommitStats {
            commit_id: commit_info.id.clone(),
            files,
        })
    }

    fn handle_change(
        &self,
        change: ChangeDetached,
        binary: bool,
        files: &mut Vec<FileStats>,
    ) -> Result<()> {
        match change {
            ChangeDetached::Addition { id, location, .. } => {
                let (is_binary, lines, _) = self.inspect_object(id)?;
                if binary || !is_binary {
                    files.push(FileStats {
                        path: location.to_string(),
                        added_lines: if is_binary { 0 } else { lines },
                        deleted_lines: 0,
                        is_binary,
                    });
                }
            }
            ChangeDetached::Deletion { id, location, .. } => {
                let (is_binary, lines, _) = self.inspect_object(id)?;
                if binary || !is_binary {
                    files.push(FileStats {
                        path: location.to_string(),
                        added_lines: 0,
                        deleted_lines: if is_binary { 0 } else { lines },
                        is_binary,
                    });
                }
            }
            ChangeDetached::Modification {
                previous_id,
                id,
                location,
                ..
            } => {
                let (old_is_binary, _, old_obj) = self.inspect_object(previous_id)?;
                let (new_is_binary, _, new_obj) = self.inspect_object(id)?;
                let is_binary = old_is_binary || new_is_binary;
                if binary || !is_binary {
                    let (added, deleted) = if is_binary {
                        (0, 0)
                    } else {
                        self.compute_line_diff(&old_obj, &new_obj)?
                    };
                    files.push(FileStats {
                        path: location.to_string(),
                        added_lines: added,
                        deleted_lines: deleted,
                        is_binary,
                    });
                }
            }
            ChangeDetached::Rewrite {
                source_id,
                id,
                source_location,
                location,
                copy,
                ..
            } => {
                let (old_is_binary, _, old_obj) = self.inspect_object(source_id)?;
                let (new_is_binary, _, new_obj) = self.inspect_object(id)?;
                let is_binary = old_is_binary || new_is_binary;
                if binary || !is_binary {
                    let (added, deleted) = if is_binary {
                        (0, 0)
                    } else {
                        self.compute_line_diff(&old_obj, &new_obj)?
                    };
                    files.push(FileStats {
                        path: source_location.to_string(),
                        added_lines: 0,
                        deleted_lines: if copy { 0 } else { deleted },
                        is_binary,
                    });
                    files.push(FileStats {
                        path: location.to_string(),
                        added_lines: if copy { added } else { 0 },
                        deleted_lines: 0,
                        is_binary,
                    });
                }
            }
        }
        Ok(())
    }

    fn inspect_object(&self, id: gix::ObjectId) -> Result<(bool, u32, gix::Object<'_>)> {
        let obj = self.repo.find_object(id)?;
        let is_binary = self.is_binary_object(&obj);
        let lines = if is_binary {
            0
        } else {
            self.count_lines(&obj)?
        };
        Ok((is_binary, lines, obj))
    }

    fn is_binary_object(&self, object: &gix::Object) -> bool {
        object.data.as_slice().iter().take(8192).any(|&b| b == 0)
    }

    fn count_lines(&self, object: &gix::Object) -> Result<u32> {
        Ok(std::str::from_utf8(object.data.as_slice())
            .map(|t| t.lines().count() as u32)
            .unwrap_or(0))
    }

    fn compute_line_diff(
        &self,
        old_object: &gix::Object,
        new_object: &gix::Object,
    ) -> Result<(u32, u32)> {
        let old_text = std::str::from_utf8(old_object.data.as_slice()).unwrap_or("");
        let new_text = std::str::from_utf8(new_object.data.as_slice()).unwrap_or("");

        let mut added = 0u32;
        let mut deleted = 0u32;

        let diff = similar::TextDiff::from_lines(old_text, new_text);
        for op in diff.ops() {
            use similar::DiffTag::*;
            match op.tag() {
                Insert => added += op.new_range().len() as u32,
                Delete => deleted += op.old_range().len() as u32,
                Replace => {
                    deleted += op.old_range().len() as u32;
                    added += op.new_range().len() as u32;
                }
                Equal => {}
            }
        }

        Ok((added, deleted))
    }

    pub fn get_commit_info(&self, commit_id: &str) -> Result<CommitInfo> {
        let oid = ObjectId::from_hex(commit_id.as_bytes())
            .map_err(|e| GmapError::Parse(format!("Invalid commit ID: {e}")))?;
        let commit = self.repo.find_commit(oid)?;
        let secs = commit.time()?.seconds;
        let timestamp = Utc
            .timestamp_opt(secs, 0)
            .single()
            .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))?;
        let author = commit.author()?;
        let message = commit.message()?;
        Ok(CommitInfo {
            id: commit_id.to_string(),
            author_name: author.name.to_string(),
            author_email: author.email.to_string(),
            message: message.title.to_string(),
            timestamp,
            parent_ids: commit.parent_ids().map(|id| id.to_string()).collect(),
        })
    }

    /// List commit IDs within range, honoring include_merges, without computing diffs.
    pub fn list_commit_ids(
        &self,
        range: &DateRange,
        include_merges: bool,
    ) -> Result<Vec<ObjectId>> {
        let mut head = self.repo.head()?;
        let head_commit = head.peel_to_commit_in_place()?;

        let mut seen: HashSet<ObjectId> = HashSet::new();
        let mut stack: VecDeque<ObjectId> = VecDeque::from([head_commit.id]);
        let mut result: Vec<ObjectId> = Vec::new();

        while let Some(commit_id) = stack.pop_back() {
            if !seen.insert(commit_id) {
                continue;
            }

            let commit = self.repo.find_commit(commit_id)?;
            let secs = commit.time()?.seconds;
            let timestamp = Utc
                .timestamp_opt(secs, 0)
                .single()
                .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))?;

            let parents: Vec<ObjectId> = commit.parent_ids().map(|id| id.into()).collect();

            if !range.contains(&timestamp) {
                for pid in &parents {
                    stack.push_back(*pid);
                }
                continue;
            }

            if !include_merges && parents.len() > 1 {
                for pid in &parents {
                    stack.push_back(*pid);
                }
                continue;
            }

            result.push(commit_id);
            for pid in &parents {
                stack.push_back(*pid);
            }
        }

        Ok(result)
    }

    /// Compute commit stats for a single commit by ID, using first parent when present.
    pub fn compute_commit_stats_for(
        &self,
        commit_id: ObjectId,
        binary: bool,
    ) -> Result<CommitStats> {
        let commit = self.repo.find_commit(commit_id)?;
        let parent_id: Option<ObjectId> = commit.parent_ids().next().map(|id| id.into());

        let commit_tree = commit.tree()?;
        let parent_tree = if let Some(pid) = parent_id {
            Some(self.repo.find_commit(pid)?.tree()?)
        } else {
            None
        };

        let changes: Vec<ChangeDetached> =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;
        let mut files = Vec::new();
        for change in changes {
            self.handle_change(change, binary, &mut files)?;
        }

        Ok(CommitStats {
            commit_id: commit.id.to_string(),
            files,
        })
    }
}

fn parse_natural_duration(input: &str) -> Option<ChronoDuration> {
    let input = input.trim().to_lowercase();
    let patterns: &[(&str, fn(i64) -> ChronoDuration)] = &[
        (" days ago", ChronoDuration::days),
        (" day ago", ChronoDuration::days),
        (" weeks ago", ChronoDuration::weeks),
        (" week ago", ChronoDuration::weeks),
        (" months ago", |n| ChronoDuration::days(n * 30)),
        (" month ago", |n| ChronoDuration::days(n * 30)),
    ];
    for (suffix, dur_fn) in patterns {
        if let Some(s) = input.strip_suffix(suffix) {
            if let Ok(n) = s.trim().parse::<i64>() {
                return Some(dur_fn(n));
            }
        }
    }
    match input.as_str() {
        "yesterday" => Some(ChronoDuration::days(1)),
        "today" | "now" => Some(ChronoDuration::seconds(0)),
        "last week" => Some(ChronoDuration::weeks(1)),
        "last month" => Some(ChronoDuration::days(30)),
        _ => None,
    }
}
