mod common;

use std::fs;

use common::{TestEnv, stdout_canon};

#[test]
fn dir_prints_worktree_path() {
    let env = TestEnv::new();
    let add_path = fs::canonicalize(env.add_worktree("feat")).unwrap();

    let dir_out = env.arbor(&["dir", "feat"]).output().unwrap();
    assert!(dir_out.status.success());

    assert_eq!(add_path, stdout_canon(&dir_out));
}

#[test]
fn dir_nonexistent_branch_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["dir", "nonexistent"]).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No worktree found"),
        "expected error message, got: {stderr}"
    );
}

#[test]
fn dir_resolves_sanitized_name() {
    let env = TestEnv::new();
    let add_path = fs::canonicalize(env.add_worktree("feature/auth")).unwrap();

    let dir_out = env.arbor(&["dir", "feature-auth"]).output().unwrap();
    assert!(dir_out.status.success());

    assert_eq!(add_path, stdout_canon(&dir_out));
}
