#![cfg(not(windows))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{TestEnv, commit_arbor_toml, git, git_stdout, stdout_path};

fn clone_output(env: &TestEnv, extra: &[&str]) -> Output {
    let url = env.repo.path().to_string_lossy().into_owned();
    let mut args = vec!["clone", url.as_str()];
    args.extend_from_slice(extra);
    let output = env.arbor_in(env.home.path(), &args).output().unwrap();
    assert!(
        output.status.success(),
        "clone should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn clone_origin(env: &TestEnv, extra: &[&str]) -> PathBuf {
    clone_output(env, extra);
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

#[test]
fn add_in_bare_repo_ignores_committed_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".\"\n");
    let bare = clone_origin(&env, &["--no-worktree"]);

    let add = env.arbor_in(&bare, &["add", "feat"]).output().unwrap();
    assert!(
        add.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    assert!(
        Path::new(&stdout_path(&add)).starts_with(env.home.path().join(".arbor/worktrees")),
        "a bare repo's worktrees must not land inside its git dir, got: {}",
        stdout_path(&add)
    );
}

#[test]
fn clone_dot_names_the_repo_after_its_directory() {
    let env = TestEnv::new();
    let output = env
        .arbor_in(env.repo.path(), &["clone", ".", "--no-worktree"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "clone . should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let repo_name = env.repo.path().file_name().unwrap().to_string_lossy();
    let expected = env
        .home
        .path()
        .join(format!(".arbor/repos/{repo_name}.git"));
    assert!(
        expected.is_dir(),
        "expected bare repo at {}",
        expected.display()
    );
}

#[test]
fn clone_ignores_worktree_dir_outside_the_repo() {
    let env = TestEnv::new();
    let outside = env.home.path().join("outside");
    commit_arbor_toml(
        &env,
        &format!("worktree_dir = \"{}\"\n", outside.to_string_lossy()),
    );

    let output = clone_output(&env, &[]);
    assert!(
        !outside.exists(),
        "a cloned repo must not place its worktree outside itself"
    );
    assert!(
        Path::new(&stdout_path(&output)).starts_with(env.home.path().join(".arbor/worktrees")),
        "should fall back to the configured worktree_dir, got: {}",
        stdout_path(&output)
    );
}
