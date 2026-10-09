#![cfg(not(windows))]

mod common;

use common::{TestEnv, git_cmd};

#[test]
fn fetch_all_fails_when_a_repo_fails() {
    let env = TestEnv::new();
    let url = env.repo.path().to_string_lossy().into_owned();
    let clone = env
        .arbor_in(env.home.path(), &["clone", &url])
        .output()
        .unwrap();
    assert!(clone.status.success());
    let bare = env.home.path().join(".arbor/repos").join(format!(
        "{}.git",
        env.repo.path().file_name().unwrap().to_string_lossy()
    ));
    git_cmd(
        &bare,
        &["remote", "set-url", "origin", "/nonexistent/repo"],
        env.home.path(),
    );

    let output = env
        .arbor_in(env.home.path(), &["fetch", "--all"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "a failed fetch must fail the command, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
