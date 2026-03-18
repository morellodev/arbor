mod common;

use std::fs;
use std::path::Path;

use common::{TestEnv, commit_arbor_toml, stdout_path};

#[test]
#[cfg(not(windows))]
fn add_runs_post_create_hook() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks]\npost_create = \"touch hook-ran.txt\"\n");

    let wt_path = env.add_worktree("feat");
    assert!(
        Path::new(&wt_path).join("hook-ran.txt").exists(),
        "hook should have created hook-ran.txt in the worktree"
    );
}

#[test]
#[cfg(not(windows))]
fn add_no_hooks_flag_skips_hook() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks]\npost_create = \"touch hook-ran.txt\"\n");

    let output = env.arbor(&["add", "--no-hooks", "feat"]).output().unwrap();
    assert!(output.status.success());

    let wt_path = stdout_path(&output);
    assert!(
        !Path::new(&wt_path).join("hook-ran.txt").exists(),
        "hook should not have run with --no-hooks"
    );
}

#[test]
#[cfg(not(windows))]
fn add_hook_failure_does_not_fail_command() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks]\npost_create = \"exit 1\"\n");

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "command should succeed even if hook fails"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Hook failed"),
        "stderr should warn about hook failure, got: {stderr}"
    );
}

#[test]
fn add_no_arbor_toml_succeeds_silently() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "feat"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add should succeed without .arbor.toml, stderr: {stderr}"
    );
    assert!(
        !stderr.contains("hook") && !stderr.contains("Hook"),
        "should not mention hooks when no .arbor.toml, got: {stderr}"
    );
}

#[test]
#[cfg(not(windows))]
fn add_hook_stdout_does_not_corrupt_path_output() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks]\npost_create = \"echo noise\"\n");

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("noise"),
        "hook stdout should not appear in stdout, got: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("noise"),
        "hook stdout should be redirected to stderr, got: {stderr}"
    );
}

#[test]
#[cfg(not(windows))]
fn add_multiple_hooks_run_in_order() {
    let env = TestEnv::new();
    commit_arbor_toml(
        &env,
        "[hooks]\npost_create = [\"touch first.txt\", \"touch second.txt\"]\n",
    );

    let wt_path = env.add_worktree("feat");
    assert!(
        Path::new(&wt_path).join("first.txt").exists(),
        "first hook should have run"
    );
    assert!(
        Path::new(&wt_path).join("second.txt").exists(),
        "second hook should have run"
    );
}

#[test]
#[cfg(not(windows))]
fn add_hook_receives_environment_variables() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks]\npost_create = \"env > env.txt\"\n");

    let wt_path = env.add_worktree("feat");
    let env_content = fs::read_to_string(Path::new(&wt_path).join("env.txt")).unwrap();
    assert!(
        env_content.contains("ARBOR_BRANCH=feat"),
        "should set ARBOR_BRANCH, got: {env_content}"
    );
    assert!(
        env_content.contains("ARBOR_EVENT=post_create"),
        "should set ARBOR_EVENT, got: {env_content}"
    );
    assert!(
        env_content.contains("ARBOR_WORKTREE="),
        "should set ARBOR_WORKTREE, got: {env_content}"
    );
    assert!(
        env_content.contains("ARBOR_REPO="),
        "should set ARBOR_REPO, got: {env_content}"
    );
}

#[test]
#[cfg(not(windows))]
fn add_existing_worktree_does_not_run_hooks() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks]\npost_create = \"touch hook-ran.txt\"\n");

    let wt_path = env.add_worktree("feat");

    // Remove the file created by the hook
    fs::remove_file(Path::new(&wt_path).join("hook-ran.txt")).unwrap();

    // Second add should not run hooks (worktree already exists)
    env.add_worktree("feat");
    assert!(
        !Path::new(&wt_path).join("hook-ran.txt").exists(),
        "hook should not run when worktree already exists"
    );
}
