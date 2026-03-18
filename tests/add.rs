mod common;

use std::path::Path;

use common::{TestEnv, git, git_stdout, stdout_path};

#[test]
fn add_creates_worktree() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let printed_path = stdout_path(&output);
    assert!(
        Path::new(&printed_path).exists(),
        "worktree directory should exist at {printed_path}"
    );
}

#[test]
fn add_existing_worktree_is_idempotent() {
    let env = TestEnv::new();

    let first = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        first.status.success(),
        "first add should succeed, stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        second.status.success(),
        "second add should succeed, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    assert_eq!(
        stdout_path(&first),
        stdout_path(&second),
        "should return the same path both times"
    );
}

#[test]
fn add_sanitizes_branch_slashes() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "feature/auth"]).output().unwrap();
    assert!(output.status.success());

    let printed_path = stdout_path(&output);
    assert!(
        printed_path.ends_with("feature-auth"),
        "path should end with 'feature-auth', got: {printed_path}"
    );
    assert!(
        Path::new(&printed_path).exists(),
        "worktree directory should exist at {printed_path}"
    );
}

#[test]
fn add_from_worktree_uses_main_repo_name() {
    let env = TestEnv::new();
    let wt_a = env.add_worktree("feat-a");

    // Run second add from inside the first worktree
    let second = env
        .arbor_in(Path::new(&wt_a), &["add", "feat-b"])
        .output()
        .unwrap();
    assert!(second.status.success());

    let wt_b = stdout_path(&second);

    // Both worktrees should be under the same repo directory
    let parent_a = Path::new(&wt_a).parent().unwrap();
    let parent_b = Path::new(&wt_b).parent().unwrap();
    assert_eq!(
        parent_a,
        parent_b,
        "both worktrees should share the same repo directory, got:\n  {}\n  {}",
        parent_a.display(),
        parent_b.display()
    );
}

#[test]
fn add_new_branch_prints_note() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("creating new branch"),
        "should warn about new branch creation, got: {stderr}"
    );
}

#[test]
fn add_with_base_creates_branch_from_ref() {
    let env = TestEnv::new();

    // Create a second commit so HEAD and HEAD~1 differ
    git(
        &env.repo,
        &["commit", "--allow-empty", "-m", "second"],
        env.home.path(),
    );

    let first_commit = git_stdout(env.repo.path(), &["rev-parse", "HEAD~1"], env.home.path());

    let output = env
        .arbor(&["add", "feat", "--base", "HEAD~1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "add --base should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let wt_path = stdout_path(&output);
    let wt_head = git_stdout(Path::new(&wt_path), &["rev-parse", "HEAD"], env.home.path());
    assert_eq!(
        wt_head, first_commit,
        "worktree HEAD should match the base ref"
    );
}

#[test]
fn add_with_base_ignores_for_existing_branch() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    // Remove the worktree so it can be re-added
    let rm = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(rm.status.success());

    // Re-add with --base (should ignore since branch exists locally)
    let output = env
        .arbor(&["add", "feat", "--base", "HEAD~1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "add --base with existing branch should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--base ignored"),
        "should warn that --base was ignored, got: {stderr}"
    );
}
