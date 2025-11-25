use assert_cmd::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn has_git() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn init_git_repo(dir: &Path) {
    // init and basic identity
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["config", "core.autocrlf", "false"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["config", "core.safecrlf", "false"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["config", "user.email", "you@example.com"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["config", "user.name", "Your Name"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn ensure_clean(dir: &Path) {
    assert!(Command::new("git")
        .args(["reset", "--hard"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn commit_file(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.sync_all().unwrap();
    assert!(Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["commit", "-m", &format!("add {name}")])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
    ensure_clean(dir);
}

#[test]
fn heat_json_outputs_buckets() {
    let dir = tempdir().unwrap();
    if !has_git() {
        return;
    }
    init_git_repo(dir.path());
    commit_file(dir.path(), "src/a.rs", "fn a(){}\n");
    commit_file(dir.path(), "src/b.rs", "fn b(){}\n");

    let mut cmd = Command::cargo_bin("gmap").unwrap();
    cmd.current_dir(dir.path())
        .arg("--repo")
        .arg(dir.path())
        .args(["heat", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(v
        .get("buckets")
        .and_then(|b| b.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false));
}

#[test]
fn churn_json_outputs_entries() {
    let dir = tempdir().unwrap();
    if !has_git() {
        return;
    }
    init_git_repo(dir.path());
    commit_file(dir.path(), "lib.rs", "pub fn hi(){}\n");
    commit_file(dir.path(), "lib.rs", "pub fn hi(){ println!(\"hi\"); }\n");

    let mut cmd = Command::cargo_bin("gmap").unwrap();
    cmd.current_dir(dir.path())
        .arg("--repo")
        .arg(dir.path())
        .args(["churn", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(v
        .get("entries")
        .and_then(|b| b.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false));
}

#[test]
fn include_merges_flag_affects_counts() {
    let dir = tempdir().unwrap();
    if !has_git() {
        return;
    }
    init_git_repo(dir.path());

    // create base
    commit_file(dir.path(), "file.txt", "a\n");

    // create feature branch and diverge on a different file
    assert!(Command::new("git")
        .args(["checkout", "-b", "feat"])
        .current_dir(dir.path())
        .status()
        .unwrap()
        .success());
    commit_file(dir.path(), "feat.txt", "f1\n");
    ensure_clean(dir.path());

    // return to master and diverge on original file
    assert!(Command::new("git")
        .args(["checkout", "master"])
        .current_dir(dir.path())
        .status()
        .unwrap()
        .success());
    commit_file(dir.path(), "file.txt", "a\nc\n");

    // merge feature (creates a merge commit without conflicts)
    assert!(Command::new("git")
        .args(["merge", "--no-ff", "feat", "-m", "merge feat"])
        .current_dir(dir.path())
        .status()
        .unwrap()
        .success());

    // without merges (default)
    let mut cmd1 = Command::cargo_bin("gmap").unwrap();
    cmd1.current_dir(dir.path())
        .arg("--repo")
        .arg(dir.path())
        .args(["heat", "--json"]);
    let out1 = cmd1.assert().success().get_output().stdout.clone();
    let v1: serde_json::Value = serde_json::from_slice(&out1).unwrap();
    let sum1: u64 = v1["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["commit_count"].as_u64().unwrap())
        .sum();

    // with merges included
    let mut cmd2 = Command::cargo_bin("gmap").unwrap();
    cmd2.current_dir(dir.path())
        .arg("--repo")
        .arg(dir.path())
        .arg("--include-merges")
        .args(["heat", "--json"]);
    let out2 = cmd2.assert().success().get_output().stdout.clone();
    let v2: serde_json::Value = serde_json::from_slice(&out2).unwrap();
    let sum2: u64 = v2["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["commit_count"].as_u64().unwrap())
        .sum();

    assert!(sum2 >= sum1);
}
