#![cfg(not(windows))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{TestEnv, commit_arbor_toml, git, git_stdout, stdout_path};

fn clone_output(env: &TestEnv, extra: &[&str]) -> Output {
    let url = env.repo.path().to_string_lossy().into_owned();
    let mut args = vec!["clone", url.as_str()];
    args.extend_from_slice(extra);
    let output = env.arbor_in(env.home.path(), &args).output().unwrap();
    assert!(
        output.status.success(),
        "clone should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn clone_origin(env: &TestEnv, extra: &[&str]) -> PathBuf {
    clone_output(env, extra);
    let repos_dir = env.home.path().join(".arbor/repos");
    fs::read_dir(&repos_dir)
        .unwrap()
        .next()
        .expect("clone should create a bare repo")
        .unwrap()
        .path()
}

#[test]
fn clone_keeps_only_default_branch_tracking_origin() {
    let env = TestEnv::new();
    git(&env.repo, &["branch", "feature"], env.home.path());

    let bare = clone_origin(&env, &["--no-worktree"]);

    let branches = git_stdout(
        &bare,
        &[
            "for-each-ref",
            "--format=%(refname:lstrip=2)",
            "refs/heads/",
        ],
        env.home.path(),
    );
    assert_eq!(branches, "main");
    let upstream = git_stdout(
        &bare,
        &["rev-parse", "--abbrev-ref", "main@{upstream}"],
        env.home.path(),
    );
    assert_eq!(upstream, "origin/main");
}

#[test]
fn add_after_fetch_uses_latest_remote_commit() {
    let env = TestEnv::new();
    git(&env.repo, &["branch", "feature"], env.home.path());
    let bare = clone_origin(&env, &["--no-worktree"]);

    git(&env.repo, &["checkout", "feature"], env.home.path());
    git(
        &env.repo,
        &["commit", "--allow-empty", "-m", "newer"],
        env.home.path(),
    );

    let fetch = env.arbor_in(&bare, &["fetch"]).output().unwrap();
    assert!(fetch.status.success());
    let add = env.arbor_in(&bare, &["add", "feature"]).output().unwrap();
    assert!(
        add.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    let subject = git_stdout(
        &PathBuf::from(stdout_path(&add)),
        &["log", "-1", "--format=%s"],
        env.home.path(),
    );
    assert_eq!(subject, "newer", "worktree should be at the fetched commit");
}

#[test]
fn add_in_bare_repo_ignores_committed_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".\"\n");
    let bare = clone_origin(&env, &["--no-worktree"]);

    let add = env.arbor_in(&bare, &["add", "feat"]).output().unwrap();
    assert!(
        add.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    assert!(
        Path::new(&stdout_path(&add)).starts_with(env.home.path().join(".arbor/worktrees")),
        "a bare repo's worktrees must not land inside its git dir, got: {}",
        stdout_path(&add)
    );
}

fn commit_marker_hook(env: &TestEnv) -> PathBuf {
    let marker = env.home.path().join("hook-ran.txt");
    commit_arbor_toml(
        env,
        &format!(
            "[hooks]\npost_create = \"touch {}\"\n",
            marker.to_string_lossy()
        ),
    );
    marker
}

#[test]
fn clone_skips_hooks_by_default_and_lists_them() {
    let env = TestEnv::new();
    let marker = commit_marker_hook(&env);

    let output = clone_output(&env, &[]);
    assert!(
        !marker.exists(),
        "cloning must not run the cloned repo's hooks by default"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Skipped post_create hooks") && stderr.contains("touch "),
        "should list the skipped hooks, got: {stderr}"
    );
    assert!(
        stderr.contains("ARBOR_BRANCH='main'") && stderr.contains("ARBOR_EVENT='post_create'"),
        "should give the hook environment for running them by hand, got: {stderr}"
    );
}

#[test]
fn clone_with_hooks_flag_runs_hooks() {
    let env = TestEnv::new();
    let marker = commit_marker_hook(&env);

    clone_origin(&env, &["--hooks"]);
    assert!(
        marker.exists(),
        "--hooks should run the cloned repo's hooks"
    );
}

#[test]
fn clone_ignores_worktree_dir_outside_the_repo() {
    let env = TestEnv::new();
    let outside = env.home.path().join("outside");
    commit_arbor_toml(
        &env,
        &format!("worktree_dir = \"{}\"\n", outside.to_string_lossy()),
    );

    let output = clone_output(&env, &[]);
    assert!(
        !outside.exists(),
        "a cloned repo must not place its worktree outside itself"
    );
    assert!(
        Path::new(&stdout_path(&output)).starts_with(env.home.path().join(".arbor/worktrees")),
        "should fall back to the configured worktree_dir, got: {}",
        stdout_path(&output)
    );
}

#[test]
fn clone_dot_names_the_repo_after_its_directory() {
    let env = TestEnv::new();
    let output = env
        .arbor_in(env.repo.path(), &["clone", ".", "--no-worktree"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "clone . should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let repo_name = env.repo.path().file_name().unwrap().to_string_lossy();
    let expected = env
        .home
        .path()
        .join(format!(".arbor/repos/{repo_name}.git"));
    assert!(
        expected.is_dir(),
        "expected bare repo at {}",
        expected.display()
    );
}

#[test]
fn clone_rejects_hooks_without_a_worktree() {
    let env = TestEnv::new();
    let url = env.repo.path().to_string_lossy().into_owned();
    let output = env
        .arbor_in(
            env.home.path(),
            &["clone", &url, "--hooks", "--no-worktree"],
        )
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--hooks has nothing to run without a worktree"
    );
}

#[test]
fn clone_reports_a_malformed_arbor_toml() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "[hooks\npost_create = \"true\"\n");

    let output = clone_output(&env, &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to load .arbor.toml"),
        "a broken .arbor.toml must not hide its hooks silently, got: {stderr}"
    );
}

fn repo_dir_name(env: &TestEnv) -> String {
    env.repo
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn clone_of_an_empty_repo_succeeds_without_a_worktree() {
    let env = TestEnv::new();
    let empty = tempfile::TempDir::new().unwrap();
    git(&empty, &["init", "--bare"], env.home.path());

    let url = empty.path().to_string_lossy().into_owned();
    let output = env
        .arbor_in(env.home.path(), &["clone", &url])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("No worktree created"),
        "should explain the missing worktree, got: {stderr}"
    );
}

#[test]
fn clone_of_a_detached_head_resets_branches_and_explains() {
    let env = TestEnv::new();
    git(&env.repo, &["branch", "other"], env.home.path());
    git(&env.repo, &["checkout", "--detach"], env.home.path());
    // git clone guesses a branch when the detached HEAD matches a branch tip.
    git(
        &env.repo,
        &["commit", "--allow-empty", "-m", "detached"],
        env.home.path(),
    );

    let output = clone_output(&env, &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not on a branch"),
        "should explain the missing worktree, got: {stderr}"
    );
    let bare = env
        .home
        .path()
        .join(format!(".arbor/repos/{}.git", repo_dir_name(&env)));
    let branches = git_stdout(
        &bare,
        &["for-each-ref", "--format=%(refname)", "refs/heads/"],
        env.home.path(),
    );
    assert_eq!(branches, "", "stale local branches should be removed");
}

#[test]
fn failed_clone_can_be_retried() {
    let env = TestEnv::new();
    let blocker = env
        .home
        .path()
        .join(format!(".arbor/worktrees/{}/main", repo_dir_name(&env)));
    fs::create_dir_all(&blocker).unwrap();
    fs::write(blocker.join("leftover"), "").unwrap();

    let url = env.repo.path().to_string_lossy().into_owned();
    let first = env
        .arbor_in(env.home.path(), &["clone", &url])
        .output()
        .unwrap();
    assert!(!first.status.success());
    let stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        stderr.contains("already exists"),
        "should name the blocking path, got: {stderr}"
    );

    fs::remove_dir_all(&blocker).unwrap();
    clone_output(&env, &[]);
}

#[test]
fn clone_rollback_also_removes_a_checked_out_worktree() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();
    let hooks = env.home.path().join("global-hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("post-checkout");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    let url = env.repo.path().to_string_lossy().into_owned();
    let failed = env
        .arbor_in(env.home.path(), &["clone", &url])
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", &hooks)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    let worktree = env
        .home
        .path()
        .join(format!(".arbor/worktrees/{}/main", repo_dir_name(&env)));
    assert!(
        !worktree.exists(),
        "a worktree of the removed clone must not be left behind"
    );

    clone_output(&env, &[]);
}
