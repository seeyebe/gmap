use crate::error::{GmapError, Result};
use crate::model::{CommitInfo, CommitStats, DateRange, FileStats};
use std::time::{SystemTime, Duration};
use chrono::{DateTime, NaiveDate, Utc, TimeZone};
use gix::{discover, ObjectId, Repository};
use gix::object::tree::diff::ChangeDetached;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

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

        let since_dt = if let Some(s) = since {
            Some(self.parse_commit_or_date(s)?)
        } else {
            None
        };

        let until_dt = if let Some(u) = until {
            Some(self.parse_commit_or_date(u)?)
        } else {
            None
        };

        if let (Some(s), Some(u)) = (since_dt, until_dt) {
            if s > u {
                return Err(GmapError::InvalidDate(format!(
                    "Invalid range: since ({}) is after until ({})",
                    s, u
                )).into());
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

        // Relative duration (e.g., "-90d", "2 weeks ago")
        if let Some(duration) = parse_natural_duration(input) {
            let now = SystemTime::now();
            let target = now.checked_sub(duration)
                .ok_or_else(|| GmapError::InvalidDate(format!("Duration overflow for '{input}'")))?;
            let dt: DateTime<Utc> = DateTime::<Utc>::from(target);
            return Ok(dt);
        }


        // Fallback to Git ref
        let id = self.repo.rev_parse_single(input)
            .map_err(|e| GmapError::Parse(format!("Invalid commit or date '{input}': {e}")))?;

        let commit = id.object()?
            .try_into_commit()
            .map_err(|_| GmapError::Parse(format!("Not a commit: {input}")))?;

        let secs = commit.time()?.seconds;
        DateTime::<Utc>::from_timestamp(secs, 0)
            .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))
    }


    pub fn collect_commits(
        &self,
        range: &DateRange,
        include_merges: bool,
        binary: bool,
    ) -> Result<Vec<CommitStats>> {
        let mut head = self.repo.head()?;
        let head_commit = head.peel_to_commit_in_place()?;

        let mut commits = Vec::new();
        let mut seen: HashSet<ObjectId> = HashSet::new();
        let mut stack: VecDeque<ObjectId> = VecDeque::from([head_commit.id]);

        let pb = ProgressBar::new_spinner();
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

            let commit = self.repo.find_commit(commit_id)?;
            let secs = commit.time()?.seconds;
            let timestamp = DateTime::from_timestamp(secs, 0)
                .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))?;

            let parents: Vec<ObjectId> = commit.parent_ids().map(|id| id.into()).collect();

            if !range.contains(&timestamp) {
                for pid in parents {
                    stack.push_back(pid);
                }
                continue;
            }

            if !include_merges && parents.len() > 1 {
                for pid in parents {
                    stack.push_back(pid);
                }
                pb.inc(1);
                continue;
            }

            let author = commit.author()?;
            let message = commit.message()?;
            let commit_info = CommitInfo {
                id: commit_id.to_string(),
                author_name: author.name.to_string(),
                author_email: author.email.to_string(),
                message: message.title.to_string(),
                timestamp,
                parent_ids: parents.iter().map(|id| id.to_string()).collect(),
            };

            let stats = if let Some(parent_id) = parents.first() {
                self.compute_diff_stats(&commit_info, commit_id, *parent_id, binary)?
            } else {
                self.compute_initial_commit_stats(&commit_info, commit_id, binary)?
            };

            commits.push(stats);

            for pid in parents {
                stack.push_back(pid);
            }

            pb.inc(1);
        }

        pb.finish_with_message("Commits collected");
        Ok(commits)
    }

    fn compute_diff_stats(
        &self,
        commit_info: &CommitInfo,
        commit_id: ObjectId,
        parent_id: ObjectId,
        binary: bool,
    ) -> Result<CommitStats> {
        let commit_tree = self.repo.find_commit(commit_id)?.tree()?;
        let parent_tree = self.repo.find_commit(parent_id)?.tree()?;

        let changes: Vec<ChangeDetached> =
            self.repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;

        let mut files = Vec::new();
        for change in changes {
            self.handle_change(change, binary, &mut files)?;
        }

        Ok(CommitStats {
            commit_id: commit_info.id.clone(),
            files,
        })
    }

    fn compute_initial_commit_stats(
        &self,
        commit_info: &CommitInfo,
        commit_id: ObjectId,
        binary: bool,
    ) -> Result<CommitStats> {
        let commit_tree = self.repo.find_commit(commit_id)?.tree()?;
        let changes: Vec<ChangeDetached> = self.repo.diff_tree_to_tree(None, Some(&commit_tree), None)?;

        let mut files = Vec::new();
        for change in changes {
            self.handle_change(change, binary, &mut files)?;
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
                if let Ok(obj) = self.repo.find_object(id) {
                    let is_binary = self.is_binary_object(&obj);
                    if binary || !is_binary {
                        let lines = if is_binary { 0 } else { self.count_lines(&obj)? };
                        files.push(FileStats {
                            path: location.to_string(),
                            added_lines: lines,
                            deleted_lines: 0,
                            is_binary,
                        });
                    }
                }
            }
            ChangeDetached::Deletion { id, location, .. } => {
                if let Ok(obj) = self.repo.find_object(id) {
                    let is_binary = self.is_binary_object(&obj);
                    if binary || !is_binary {
                        let lines = if is_binary { 0 } else { self.count_lines(&obj)? };
                        files.push(FileStats {
                            path: location.to_string(),
                            added_lines: 0,
                            deleted_lines: lines,
                            is_binary,
                        });
                    }
                }
            }
            ChangeDetached::Modification {
                previous_id,
                id,
                location,
                ..
            } => {
                if let (Ok(old_obj), Ok(new_obj)) =
                    (self.repo.find_object(previous_id), self.repo.find_object(id))
                {
                    let is_binary = self.is_binary_object(&old_obj) || self.is_binary_object(&new_obj);
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
            }
            ChangeDetached::Rewrite {
                source_id,
                id,
                source_location,
                location,
                copy,
                ..
            } => {
                if let (Ok(old_obj), Ok(new_obj)) =
                    (self.repo.find_object(source_id), self.repo.find_object(id))
                {
                    let is_binary = self.is_binary_object(&old_obj) || self.is_binary_object(&new_obj);
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
        }
        Ok(())
    }

    fn is_binary_object(&self, object: &gix::Object) -> bool {
        object.data.as_slice().iter().take(8192).any(|&b| b == 0)
    }

    fn count_lines(&self, object: &gix::Object) -> Result<u32> {
        Ok(std::str::from_utf8(object.data.as_slice())
            .map(|t| t.lines().count() as u32)
            .unwrap_or(0))
    }

    fn compute_line_diff(&self, old_object: &gix::Object, new_object: &gix::Object) -> Result<(u32, u32)> {
        let old_text = std::str::from_utf8(old_object.data.as_slice()).unwrap_or("");
        let new_text = std::str::from_utf8(new_object.data.as_slice()).unwrap_or("");

        let old_lines: Vec<&str> = old_text.lines().collect();
        let new_lines: Vec<&str> = new_text.lines().collect();

        let mut added = 0usize;
        let mut deleted = 0usize;
        let (mut oi, mut ni) = (0usize, 0usize);

        while oi < old_lines.len() || ni < new_lines.len() {
            if oi >= old_lines.len() {
                added += new_lines.len() - ni;
                break;
            }
            if ni >= new_lines.len() {
                deleted += old_lines.len() - oi;
                break;
            }

            if old_lines[oi] == new_lines[ni] {
                oi += 1;
                ni += 1;
                continue;
            }

            let mut found = false;
            for look_ahead in 1..=3 {
                if oi + look_ahead < old_lines.len() && old_lines[oi + look_ahead] == new_lines[ni] {
                    deleted += look_ahead;
                    oi += look_ahead;
                    found = true;
                    break;
                }
                if ni + look_ahead < new_lines.len() && old_lines[oi] == new_lines[ni + look_ahead] {
                    added += look_ahead;
                    ni += look_ahead;
                    found = true;
                    break;
                }
            }

            if !found {
                deleted += 1;
                added += 1;
                oi += 1;
                ni += 1;
            }
        }

        Ok((added as u32, deleted as u32))
    }

    pub fn get_commit_info(&self, commit_id: &str) -> Result<CommitInfo> {
        let oid = ObjectId::from_hex(commit_id.as_bytes())
            .map_err(|e| GmapError::Parse(format!("Invalid commit ID: {e}")))?;
        let commit = self.repo.find_commit(oid)?;
        let secs = commit.time()?.seconds;
        let timestamp = DateTime::from_timestamp(secs, 0)
            .ok_or_else(|| GmapError::InvalidDate(format!("Invalid timestamp: {secs}")))?;

        Ok(CommitInfo {
            id: commit_id.to_string(),
            author_name: commit.author()?.name.to_string(),
            author_email: commit.author()?.email.to_string(),
            message: commit.message()?.title.to_string(),
            timestamp,
            parent_ids: commit.parent_ids().map(|id| id.to_string()).collect(),
        })
    }
}

fn parse_natural_duration(input: &str) -> Option<Duration> {
    let input = input.trim().to_lowercase();

    if let Some(days) = input.strip_suffix(" days ago") {
        if let Ok(n) = days.trim().parse::<u64>() {
            return Some(Duration::from_secs(n * 86400));
        }
    }

    if let Some(weeks) = input.strip_suffix(" weeks ago") {
        if let Ok(n) = weeks.trim().parse::<u64>() {
            return Some(Duration::from_secs(n * 7 * 86400));
        }
    }

    if let Some(months) = input.strip_suffix(" months ago") {
        if let Ok(n) = months.trim().parse::<u64>() {
            return Some(Duration::from_secs(n * 30 * 86400));
        }
    }

    None
}