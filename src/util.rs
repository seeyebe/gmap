use crate::model::FileStats;
use chrono::{DateTime, Datelike, Months, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn week_key(timestamp: &DateTime<Utc>) -> String {
    format!("{}-W{:02}", timestamp.year(), timestamp.iso_week().week())
}

pub fn month_key(timestamp: &DateTime<Utc>) -> String {
    format!("{}-{:02}", timestamp.year(), timestamp.month())
}

pub fn period_key(timestamp: &DateTime<Utc>, monthly: bool) -> String {
    if monthly {
        month_key(timestamp)
    } else {
        week_key(timestamp)
    }
}

pub fn files_matching<'a>(
    files: &'a [FileStats],
    path_prefix: Option<&'a str>,
) -> impl Iterator<Item = &'a FileStats> + 'a {
    files.iter().filter(move |fs| {
        if let Some(prefix) = path_prefix {
            fs.path.starts_with(prefix)
        } else {
            true
        }
    })
}

pub fn path_excluded(path: &str, excludes: &[String]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let p = path.to_lowercase();
    excludes.iter().any(|ex| p.contains(&ex.to_lowercase()))
}

pub fn cutoff_timestamp(months_back: u32) -> DateTime<Utc> {
    let now = Utc::now();
    // subtract months, approximate by months API; clamp
    now.checked_sub_months(Months::new(months_back))
        .unwrap_or(now)
}

pub struct GitIgnoreMatcher {
    root: PathBuf,
    cache: HashMap<PathBuf, Option<ignore::gitignore::Gitignore>>,
}

impl GitIgnoreMatcher {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            cache: HashMap::new(),
        }
    }

    pub fn is_ignored(&mut self, rel_path: &str) -> bool {
        let abs = self.root.join(rel_path);
        let dir_buf: PathBuf = abs
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.root.clone());
        let gi = self.get_or_build_for_dir(&dir_buf);
        if let Some(gi) = gi {
            let m = gi.matched(Path::new(rel_path), false);
            m.is_ignore()
        } else {
            false
        }
    }

    fn get_or_build_for_dir(&mut self, dir: &Path) -> Option<&ignore::gitignore::Gitignore> {
        if !self.cache.contains_key(dir) {
            self.cache
                .insert(dir.to_path_buf(), self.build_for_dir(dir));
        }
        self.cache.get(dir).and_then(|o| o.as_ref())
    }

    fn build_for_dir(&self, dir: &Path) -> Option<ignore::gitignore::Gitignore> {
        use ignore::gitignore::GitignoreBuilder;
        let mut builder = GitignoreBuilder::new(&self.root);
        let root_gi = self.root.join(".gitignore");
        if root_gi.exists() {
            let _ = builder.add(&root_gi);
        }
        let info_excl = self.root.join(".git").join("info").join("exclude");
        if info_excl.exists() {
            let _ = builder.add(&info_excl);
        }
        // Add .gitignore files along the path from root to dir
        if let Ok(rel) = dir.strip_prefix(&self.root) {
            let mut cur = PathBuf::new();
            for comp in rel.components() {
                cur.push(comp);
                let f = self.root.join(&cur).join(".gitignore");
                if f.exists() {
                    let _ = builder.add(f);
                }
            }
        }
        builder.build().ok()
    }
}
