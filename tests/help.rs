mod common;

use common::TestEnv;

#[test]
fn help_flag_outputs_to_stderr_not_stdout() {
    let env = TestEnv::new();
    let output = env.arbor(&["clone", "--help"]).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "help should not appear on stdout, got: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Clone a repository"),
        "help should appear on stderr, got: {stderr}"
    );
}

#[test]
fn short_help_flag_outputs_to_stderr_not_stdout() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "-h"]).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "help should not appear on stdout, got: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Create a worktree"),
        "help should appear on stderr, got: {stderr}"
    );
}
