mod common;

use std::fs;
use std::path::Path;

use common::{TestEnv, git_cmd};

#[test]
fn list_alias_ls_works() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["ls"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("feat"),
        "ls alias should list worktrees, got: {stdout}"
    );
}

#[test]
fn list_shows_worktrees() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["list"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Branch"),
        "list output should include the header, got: {stdout}"
    );
    assert!(
        stdout.contains("feat"),
        "list output should include the branch name, got: {stdout}"
    );
    assert!(
        stdout.contains("clean"),
        "list output should show clean state, got: {stdout}"
    );
    assert!(
        stdout.contains("2 worktrees ("),
        "summary should use plural noun, got: {stdout}"
    );
}

#[test]
fn list_detects_dirty_worktree() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let output = env.arbor(&["list"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dirty"),
        "should detect dirty worktree, got: {stdout}"
    );
}

#[test]
fn list_short_omits_paths() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["list", "--short"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Path"),
        "short output should omit paths, got: {stdout}"
    );
    assert!(
        stdout.contains("clean"),
        "should still show status, got: {stdout}"
    );
}

#[test]
fn list_json_outputs_valid_json() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["list", "--json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed.is_array(), "JSON output should be an array");

    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "should contain at least one worktree");
    assert!(
        arr[0].get("branch").is_some(),
        "worktree entry should have a branch field"
    );
}

#[test]
fn list_json_does_not_contain_indicator() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let output = env
        .arbor_in(Path::new(&wt_path), &["list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('*'),
        "JSON output should not contain * indicator, got: {stdout}"
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed.is_array(), "JSON output should be an array");
}

#[test]
fn list_shows_current_indicator_from_worktree() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let output = env
        .arbor_in(Path::new(&wt_path), &["list"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('*'),
        "list from inside worktree should show * indicator, got: {stdout}"
    );
}

#[test]
fn list_all_succeeds_with_no_repos() {
    let env = TestEnv::new();
    let output = env.arbor(&["list", "--all"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn fetch_all_succeeds_with_no_repos() {
    let env = TestEnv::new();
    let output = env.arbor(&["fetch", "--all"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn list_summary_pluralizes_worktree_count() {
    let env = TestEnv::new();

    let output = env.arbor(&["list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 worktree ("),
        "single worktree should use singular noun, got: {stdout}"
    );
}

#[test]
fn list_marks_nested_worktree_as_current() {
    let env = TestEnv::new();
    common::commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");
    let wt_path = env.add_worktree("feat");

    let output = env
        .arbor_in(Path::new(&wt_path), &["ls", "--color", "never"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let current = stdout
        .lines()
        .find(|line| line.trim_start().starts_with('*'))
        .unwrap_or_else(|| panic!("no current marker, got: {stdout}"));
    assert!(
        current.contains("feat"),
        "nested worktree should be marked current, got: {current}"
    );
}

#[test]
fn list_all_ignores_non_repo_dirs_inside_an_enclosing_repo() {
    let env = TestEnv::new();
    let home = env.home.path();
    git_cmd(home, &["init"], home);
    git_cmd(home, &["commit", "--allow-empty", "-m", "dotfiles"], home);
    fs::create_dir_all(home.join(".arbor/repos/not-a-repo")).unwrap();
    let real = home.join(".arbor/repos/real.git");
    let origin = env.repo.path().to_string_lossy();
    git_cmd(
        home,
        &["clone", "--bare", &origin, &real.to_string_lossy()],
        home,
    );
    let real_wt = home.join("real-main");
    git_cmd(
        &real,
        &["worktree", "add", &real_wt.to_string_lossy(), "main"],
        home,
    );

    let output = env.arbor(&["ls", "--all", "--json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("not-a-repo"),
        "a plain dir must not be listed as a repo, got: {stdout}"
    );
    assert!(
        stdout.contains("\"real\""),
        "the real repo should still be listed, got: {stdout}"
    );
}

#[test]
fn list_all_notes_dirs_git_cannot_open() {
    let env = TestEnv::new();
    fs::create_dir_all(env.home.path().join(".arbor/repos/broken")).unwrap();

    let output = env.arbor(&["ls", "--all"]).output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Skipping broken"),
        "should say why the dir was skipped, got: {stderr}"
    );
}
