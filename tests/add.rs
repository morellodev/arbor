mod common;

use std::path::Path;

use common::{TestEnv, git, git_cmd, git_stdout, stdout_path};

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

#[test]
fn add_branch_checked_out_elsewhere_returns_existing_path() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "main"]).output().unwrap();
    assert!(
        output.status.success(),
        "add of an already checked-out branch should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::canonicalize(stdout_path(&output)).unwrap(),
        std::fs::canonicalize(env.repo.path()).unwrap(),
        "should return the worktree that has the branch"
    );
}

#[test]
fn add_rejects_path_taken_by_another_branch() {
    let env = TestEnv::new();
    env.add_worktree("feature/auth");

    let output = env.arbor(&["add", "feature-auth"]).output().unwrap();
    assert!(
        !output.status.success(),
        "feature-auth must not reuse the feature/auth worktree"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is already the worktree for 'feature/auth'"),
        "should explain the collision, got: {stderr}"
    );
}

#[test]
fn add_detached_worktree_at_expected_path_returns_it() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");
    git_cmd(
        Path::new(&wt_path),
        &["checkout", "--detach"],
        env.home.path(),
    );

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "add should return the detached worktree, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout_path(&output), wt_path);
}

#[test]
fn add_rejects_existing_non_worktree_dir() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");
    env.arbor(&["rm", "feat"]).assert().success();
    std::fs::create_dir_all(&wt_path).unwrap();

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists and is not a worktree"),
        "should refuse a stray directory, got: {stderr}"
    );
}

#[test]
fn add_resolves_relative_config_worktree_dir_against_arbor_dir() {
    let env = TestEnv::new();
    let repos = env.home.path().join(".arbor/repos");
    std::fs::write(
        env.home.path().join(".arbor/config.toml"),
        format!(
            "worktree_dir = \"wts\"\nrepos_dir = \"{}\"\n",
            repos.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    let subdir = env.repo.path().join("sub");
    std::fs::create_dir(&subdir).unwrap();

    let output = env.arbor_in(&subdir, &["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Path::new(&stdout_path(&output)).starts_with(env.home.path().join(".arbor/wts")),
        "worktree should be under ~/.arbor/wts, got: {}",
        stdout_path(&output)
    );
}
