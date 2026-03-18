mod common;

use std::fs;
use std::path::Path;

use common::TestEnv;

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
