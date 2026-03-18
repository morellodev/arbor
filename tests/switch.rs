mod common;

use std::fs;

use common::{TestEnv, stdout_canon};

#[test]
fn switch_to_existing_worktree() {
    let env = TestEnv::new();
    let add_path = fs::canonicalize(env.add_worktree("feat")).unwrap();

    let switch_out = env.arbor(&["switch", "feat"]).output().unwrap();
    assert!(switch_out.status.success());

    assert_eq!(add_path, stdout_canon(&switch_out));
}

#[test]
fn switch_nonexistent_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["switch", "nonexistent"]).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No worktree found"),
        "expected error message, got: {stderr}"
    );
    assert!(
        stderr.contains("arbor add"),
        "expected hint about arbor add, got: {stderr}"
    );
}

#[test]
fn switch_with_sanitized_name() {
    let env = TestEnv::new();
    env.add_worktree("feature/auth");

    let switch_out = env.arbor(&["switch", "feature-auth"]).output().unwrap();
    assert!(
        switch_out.status.success(),
        "switch with sanitized name should succeed"
    );
}

#[test]
fn switch_alias_cd_works() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["cd", "feat"]).output().unwrap();
    assert!(output.status.success());
}
