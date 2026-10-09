#![cfg(not(windows))]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{TestEnv, git, git_stdout, stdout_path};

fn clone_origin(env: &TestEnv, extra: &[&str]) -> PathBuf {
    let url = env.repo.path().to_string_lossy().into_owned();
    let mut args = vec!["clone", url.as_str()];
    args.extend_from_slice(extra);
    let output = env.arbor_in(env.home.path(), &args).output().unwrap();
    assert!(
        output.status.success(),
        "clone should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let repos_dir = env.home.path().join(".arbor/repos");
    fs::read_dir(&repos_dir)
        .unwrap()
        .next()
        .expect("clone should create a bare repo")
        .unwrap()
        .path()
}

#[test]
fn clone_keeps_only_default_branch_tracking_origin() {
    let env = TestEnv::new();
    git(&env.repo, &["branch", "feature"], env.home.path());

    let bare = clone_origin(&env, &["--no-worktree"]);

    let branches = git_stdout(
        &bare,
        &[
            "for-each-ref",
            "--format=%(refname:lstrip=2)",
            "refs/heads/",
        ],
        env.home.path(),
    );
    assert_eq!(branches, "main");
    let upstream = git_stdout(
        &bare,
        &["rev-parse", "--abbrev-ref", "main@{upstream}"],
        env.home.path(),
    );
    assert_eq!(upstream, "origin/main");
}

#[test]
fn add_after_fetch_uses_latest_remote_commit() {
    let env = TestEnv::new();
    git(&env.repo, &["branch", "feature"], env.home.path());
    let bare = clone_origin(&env, &["--no-worktree"]);

    git(&env.repo, &["checkout", "feature"], env.home.path());
    git(
        &env.repo,
        &["commit", "--allow-empty", "-m", "newer"],
        env.home.path(),
    );

    let fetch = env.arbor_in(&bare, &["fetch"]).output().unwrap();
    assert!(fetch.status.success());
    let add = env.arbor_in(&bare, &["add", "feature"]).output().unwrap();
    assert!(
        add.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    let subject = git_stdout(
        &PathBuf::from(stdout_path(&add)),
        &["log", "-1", "--format=%s"],
        env.home.path(),
    );
    assert_eq!(subject, "newer", "worktree should be at the fetched commit");
}
