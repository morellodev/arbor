mod common;

use common::TestEnv;

#[test]
fn clean_non_tty_fails_with_hint() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["clean"]).write_stdin("").output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Interactive terminal required"),
        "should require TTY, got: {stderr}"
    );
    assert!(
        stderr.contains("arbor rm"),
        "should suggest arbor rm, got: {stderr}"
    );
}
