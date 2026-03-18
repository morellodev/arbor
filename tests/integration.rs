use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

struct TestEnv {
    home: TempDir,
    repo: TempDir,
}

impl TestEnv {
    fn new() -> Self {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();

        let worktree_dir = home.path().join(".arbor/worktrees");
        let repos_dir = home.path().join(".arbor/repos");

        fs::create_dir_all(home.path().join(".arbor")).unwrap();
        let worktree_str = worktree_dir.to_string_lossy().replace('\\', "/");
        let repos_str = repos_dir.to_string_lossy().replace('\\', "/");
        fs::write(
            home.path().join(".arbor/config.toml"),
            format!("worktree_dir = \"{worktree_str}\"\nrepos_dir = \"{repos_str}\"\n"),
        )
        .unwrap();

        fs::write(
            home.path().join(".gitconfig"),
            "[init]\n\tdefaultBranch = main\n[user]\n\temail = test@test.com\n\tname = Test\n",
        )
        .unwrap();

        git(&repo, &["init", "--initial-branch=main"], home.path());
        git(
            &repo,
            &["commit", "--allow-empty", "-m", "init"],
            home.path(),
        );

        Self { home, repo }
    }

    fn arbor(&self, args: &[&str]) -> assert_cmd::Command {
        self.arbor_in(self.repo.path(), args)
    }

    fn arbor_in(&self, dir: &Path, args: &[&str]) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("arbor").unwrap();
        cmd.current_dir(dir);
        cmd.env("HOME", self.home.path());
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        cmd.env("GIT_CONFIG_GLOBAL", self.home.path().join(".gitconfig"));
        #[cfg(windows)]
        cmd.env("USERPROFILE", self.home.path());
        cmd.args(args);
        cmd
    }
}

fn git_cmd(dir: &Path, args: &[&str], home: &std::path::Path) -> std::process::Output {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(dir)
        .env("HOME", home)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join(".gitconfig"));
    #[cfg(windows)]
    cmd.env("USERPROFILE", home);
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn git(dir: &TempDir, args: &[&str], home: &std::path::Path) {
    git_cmd(dir.path(), args, home);
}

fn git_stdout(dir: &Path, args: &[&str], home: &std::path::Path) -> String {
    let output = git_cmd(dir, args, home);
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// ── add ──────────────────────────────────────────────────────────────

#[test]
fn add_creates_worktree() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "feat"]).output().unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_path = stdout.trim();
    assert!(
        Path::new(printed_path).exists(),
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

    let path_1 = String::from_utf8_lossy(&first.stdout).trim().to_string();
    let path_2 = String::from_utf8_lossy(&second.stdout).trim().to_string();
    assert_eq!(path_1, path_2, "should return the same path both times");
}

#[test]
fn add_sanitizes_branch_slashes() {
    let env = TestEnv::new();
    let output = env.arbor(&["add", "feature/auth"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_path = stdout.trim();
    assert!(
        printed_path.ends_with("feature-auth"),
        "path should end with 'feature-auth', got: {printed_path}"
    );
    assert!(
        Path::new(printed_path).exists(),
        "worktree directory should exist at {printed_path}"
    );
}

#[test]
fn add_from_worktree_uses_main_repo_name() {
    let env = TestEnv::new();

    let first = env.arbor(&["add", "feat-a"]).output().unwrap();
    assert!(first.status.success());
    let wt_a = String::from_utf8_lossy(&first.stdout).trim().to_string();

    // Run second add from inside the first worktree
    let second = env
        .arbor_in(Path::new(&wt_a), &["add", "feat-b"])
        .output()
        .unwrap();
    assert!(second.status.success());

    let wt_b = String::from_utf8_lossy(&second.stdout).trim().to_string();

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

    let wt_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let wt_head = git_stdout(Path::new(&wt_path), &["rev-parse", "HEAD"], env.home.path());
    assert_eq!(
        wt_head, first_commit,
        "worktree HEAD should match the base ref"
    );
}

#[test]
fn add_with_base_ignores_for_existing_branch() {
    let env = TestEnv::new();

    // Create the branch first
    let first = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(first.status.success());

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

// ── dir ──────────────────────────────────────────────────────────────

#[test]
fn dir_prints_worktree_path() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let add_path = fs::canonicalize(String::from_utf8_lossy(&add_out.stdout).trim()).unwrap();

    let dir_out = env.arbor(&["dir", "feat"]).output().unwrap();
    assert!(dir_out.status.success());

    let dir_path = fs::canonicalize(String::from_utf8_lossy(&dir_out.stdout).trim()).unwrap();
    assert_eq!(add_path, dir_path);
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
    let add_out = env.arbor(&["add", "feature/auth"]).output().unwrap();
    let add_path = fs::canonicalize(String::from_utf8_lossy(&add_out.stdout).trim()).unwrap();

    let dir_out = env.arbor(&["dir", "feature-auth"]).output().unwrap();
    assert!(dir_out.status.success());

    let dir_path = fs::canonicalize(String::from_utf8_lossy(&dir_out.stdout).trim()).unwrap();
    assert_eq!(add_path, dir_path);
}

// ── remove ───────────────────────────────────────────────────────────

#[test]
fn remove_deletes_worktree() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();
    assert!(Path::new(&wt_path).exists());

    let rm_out = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(rm_out.status.success());
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after remove"
    );
}

#[test]
fn remove_nonexistent_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["remove", "nonexistent"]).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No worktree found"),
        "expected error message, got: {stderr}"
    );
}

#[test]
fn remove_with_delete_branch_resolves_sanitized_name() {
    let env = TestEnv::new();

    // Create a worktree for feature/auth (dir becomes feature-auth)
    env.arbor(&["add", "feature/auth"]).output().unwrap();

    // Remove using the sanitized name with -d to also delete the branch
    let rm_out = env
        .arbor(&["remove", "feature-auth", "-d"])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("Deleted branch 'feature/auth'"),
        "should delete the actual branch name, got: {stderr}"
    );
}

#[test]
fn remove_force_delete_unmerged_branch() {
    let env = TestEnv::new();

    // Create a worktree and make a commit in it so the branch is "unmerged"
    let add_out = env.arbor(&["add", "unmerged"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    // Make a commit in the worktree so the branch diverges
    git_cmd(
        Path::new(&wt_path),
        &["commit", "--allow-empty", "-m", "wip"],
        env.home.path(),
    );

    // Force-remove the worktree and delete the unmerged branch
    let rm_out = env
        .arbor(&["remove", "unmerged", "-f", "-d"])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("Deleted branch"),
        "should force-delete the branch, got: {stderr}"
    );
    assert!(
        stderr.contains("(was "),
        "should print commit hash for force-deleted branch, got: {stderr}"
    );
}

#[test]
fn remove_delete_branch_prints_commit_hash() {
    let env = TestEnv::new();

    env.arbor(&["add", "feat"]).assert().success();

    let rm_out = env.arbor(&["remove", "feat", "-d"]).output().unwrap();
    assert!(rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("(was "),
        "should print commit hash when deleting branch, got: {stderr}"
    );
}

#[test]
fn remove_dirty_worktree_rejected() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    // Create an untracked file to make the worktree dirty
    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let rm_out = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(!rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "should reject dirty worktree, got: {stderr}"
    );
}

#[test]
fn remove_alias_rm_works() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

    let output = env.arbor(&["rm", "feat"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
#[cfg(not(windows))]
fn remove_from_inside_worktree_prints_toplevel() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", "feat"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );

    let printed = fs::canonicalize(String::from_utf8_lossy(&rm_out.stdout).trim()).unwrap();
    let expected = fs::canonicalize(env.repo.path()).unwrap();
    assert_eq!(printed, expected, "should print the repo toplevel path");
}

#[test]
#[cfg(not(windows))]
fn remove_from_worktree_subdirectory_prints_toplevel() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    let subdir = Path::new(&wt_path).join("deep/nested");
    fs::create_dir_all(&subdir).unwrap();

    let rm_out = env
        .arbor_in(&subdir, &["rm", "feat", "-f"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );

    let printed = fs::canonicalize(String::from_utf8_lossy(&rm_out.stdout).trim()).unwrap();
    let expected = fs::canonicalize(env.repo.path()).unwrap();
    assert_eq!(printed, expected, "should print the repo toplevel path");
}

#[test]
fn remove_from_outside_worktree_prints_nothing_to_stdout() {
    let env = TestEnv::new();

    env.arbor(&["add", "feat"]).output().unwrap();

    let rm_out = env.arbor(&["rm", "feat"]).output().unwrap();
    assert!(rm_out.status.success());

    let stdout = String::from_utf8_lossy(&rm_out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "should print nothing to stdout, got: {stdout}"
    );
}

#[test]
fn remove_from_different_worktree_prints_nothing_to_stdout() {
    let env = TestEnv::new();

    let add_a = env.arbor(&["add", "feat-a"]).output().unwrap();
    let wt_a = String::from_utf8_lossy(&add_a.stdout).trim().to_string();
    env.arbor(&["add", "feat-b"]).output().unwrap();

    let rm_out = env
        .arbor_in(Path::new(&wt_a), &["rm", "feat-b"])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let stdout = String::from_utf8_lossy(&rm_out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "should print nothing to stdout when removing a different worktree, got: {stdout}"
    );
}

// ── switch ───────────────────────────────────────────────────────────

#[test]
fn switch_to_existing_worktree() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let add_path = fs::canonicalize(String::from_utf8_lossy(&add_out.stdout).trim()).unwrap();

    let switch_out = env.arbor(&["switch", "feat"]).output().unwrap();
    assert!(switch_out.status.success());

    let switch_path = fs::canonicalize(String::from_utf8_lossy(&switch_out.stdout).trim()).unwrap();
    assert_eq!(add_path, switch_path);
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
    env.arbor(&["add", "feature/auth"]).output().unwrap();

    let switch_out = env.arbor(&["switch", "feature-auth"]).output().unwrap();
    assert!(
        switch_out.status.success(),
        "switch with sanitized name should succeed"
    );
}

#[test]
fn switch_alias_cd_works() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

    let output = env.arbor(&["cd", "feat"]).output().unwrap();
    assert!(output.status.success());
}

// ── list ─────────────────────────────────────────────────────────────

#[test]
fn list_alias_ls_works() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

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
    env.arbor(&["add", "feat"]).output().unwrap();

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

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

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
    env.arbor(&["add", "feat"]).output().unwrap();

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
    env.arbor(&["add", "feat"]).output().unwrap();

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

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

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

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

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

// ── init ─────────────────────────────────────────────────────────────

#[test]
fn init_zsh_outputs_shell_function() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "zsh"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("arbor()"),
        "zsh init should define an arbor function, got: {stdout}"
    );
    assert!(
        stdout.contains("compdef"),
        "zsh init should include completions, got: {stdout}"
    );
}

#[test]
fn init_bash_outputs_shell_function() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "bash"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("arbor()"),
        "bash init should define an arbor function, got: {stdout}"
    );
    assert!(
        stdout.contains("complete -F"),
        "bash init should include completions, got: {stdout}"
    );
}

#[test]
fn init_fish_outputs_shell_function() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "fish"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function arbor"),
        "fish init should define an arbor function, got: {stdout}"
    );
}

#[test]
fn init_unsupported_shell_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "nushell"]).output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported shell"),
        "should reject unsupported shell, got: {stderr}"
    );
}

// ── clean ────────────────────────────────────────────────────────────

#[test]
fn clean_non_tty_fails_with_hint() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

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

// ── prune ────────────────────────────────────────────────────────────

#[test]
fn prune_succeeds_on_clean_repo() {
    let env = TestEnv::new();
    let output = env.arbor(&["prune"]).output().unwrap();
    assert!(output.status.success());
}

// ── hooks ────────────────────────────────────────────────────────────

#[test]
#[cfg(not(windows))]
fn add_runs_post_create_hook() {
    let env = TestEnv::new();
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = \"touch hook-ran.txt\"\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let wt_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        Path::new(&wt_path).join("hook-ran.txt").exists(),
        "hook should have created hook-ran.txt in the worktree"
    );
}

#[test]
#[cfg(not(windows))]
fn add_no_hooks_flag_skips_hook() {
    let env = TestEnv::new();
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = \"touch hook-ran.txt\"\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

    let output = env.arbor(&["add", "--no-hooks", "feat"]).output().unwrap();
    assert!(output.status.success());

    let wt_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        !Path::new(&wt_path).join("hook-ran.txt").exists(),
        "hook should not have run with --no-hooks"
    );
}

#[test]
#[cfg(not(windows))]
fn add_hook_failure_does_not_fail_command() {
    let env = TestEnv::new();
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = \"exit 1\"\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

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
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = \"echo noise\"\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

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
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = [\"touch first.txt\", \"touch second.txt\"]\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let wt_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
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
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = \"env > env.txt\"\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let wt_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
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
    fs::write(
        env.repo.path().join(".arbor.toml"),
        "[hooks]\npost_create = \"touch hook-ran.txt\"\n",
    )
    .unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add hooks config"],
        env.home.path(),
    );

    let first = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(first.status.success());
    let wt_path = String::from_utf8_lossy(&first.stdout).trim().to_string();

    // Remove the file created by the hook
    fs::remove_file(Path::new(&wt_path).join("hook-ran.txt")).unwrap();

    // Second add should not run hooks (worktree already exists)
    let second = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(second.status.success());
    assert!(
        !Path::new(&wt_path).join("hook-ran.txt").exists(),
        "hook should not run when worktree already exists"
    );
}

// ── init --inject ────────────────────────────────────────────────────

#[test]
#[cfg(not(windows))]
fn init_inject_adds_line() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(
        output.status.success(),
        "init --inject should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Added shell integration"),
        "should confirm injection, got: {stderr}"
    );

    let zshrc = fs::read_to_string(env.home.path().join(".zshrc")).unwrap();
    assert!(
        zshrc.contains("arbor init zsh"),
        "zshrc should contain eval line, got: {zshrc}"
    );
    assert!(
        zshrc.contains("# arbor"),
        "zshrc should contain comment marker, got: {zshrc}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_idempotent() {
    let env = TestEnv::new();

    env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    let second = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(second.status.success());

    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("already configured"),
        "second run should say already configured, got: {stderr}"
    );

    let zshrc = fs::read_to_string(env.home.path().join(".zshrc")).unwrap();
    let count = zshrc.matches("arbor init zsh").count();
    assert_eq!(
        count, 1,
        "eval line should appear exactly once, got: {count}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_creates_fish_dirs() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "fish", "--inject"]).output().unwrap();
    assert!(
        output.status.success(),
        "init fish --inject should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = env.home.path().join(".config/fish/config.fish");
    assert!(config_path.exists(), "should create fish config file");

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("arbor init fish | source"),
        "fish config should contain source line, got: {content}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_already_configured() {
    let env = TestEnv::new();

    let zshrc_path = env.home.path().join(".zshrc");
    fs::write(
        &zshrc_path,
        "# existing config\neval \"$(arbor init zsh)\"\n",
    )
    .unwrap();

    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already configured"),
        "should detect existing config, got: {stderr}"
    );

    let zshrc = fs::read_to_string(&zshrc_path).unwrap();
    let count = zshrc.matches("arbor init zsh").count();
    assert_eq!(count, 1, "should not duplicate the line, got: {count}");
}

// ── help output ─────────────────────────────────────────────────────

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

// ── local worktree_dir override ─────────────────────────────────────

fn commit_arbor_toml(env: &TestEnv, content: &str) {
    fs::write(env.repo.path().join(".arbor.toml"), content).unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add .arbor.toml"],
        env.home.path(),
    );
}

#[test]
fn add_with_local_worktree_dir_relative() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_path = stdout.trim();
    let expected = env.repo.path().join(".worktrees/feat");
    let actual = fs::canonicalize(printed_path).unwrap();
    let expected_canon = fs::canonicalize(&expected).unwrap();
    assert_eq!(
        actual,
        expected_canon,
        "worktree should be at {}, got {}",
        expected.display(),
        printed_path
    );
}

#[test]
fn add_with_local_worktree_dir_absolute() {
    let env = TestEnv::new();
    let abs_dir = env.home.path().join("custom-wt");
    let abs_str = abs_dir.to_string_lossy().replace('\\', "/");
    commit_arbor_toml(&env, &format!("worktree_dir = \"{abs_str}\"\n"));

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_path = stdout.trim();
    let actual = fs::canonicalize(printed_path).unwrap();
    let expected = fs::canonicalize(abs_dir.join("feat")).unwrap();
    assert_eq!(
        actual, expected,
        "worktree should be at absolute path, got {}",
        printed_path
    );
}

#[test]
fn add_with_local_worktree_dir_omits_repo_name() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_path = stdout.trim();

    // Path should be <repo>/.worktrees/feat, NOT <repo>/.worktrees/<repo_name>/feat
    assert!(
        Path::new(printed_path).ends_with(Path::new(".worktrees").join("feat")),
        "path should end with .worktrees/feat (no repo name subdir), got: {printed_path}"
    );
}

#[test]
fn add_with_local_worktree_dir_is_idempotent() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let first = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(first.status.success());

    let second = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(second.status.success());

    let path_1 = String::from_utf8_lossy(&first.stdout).trim().to_string();
    let path_2 = String::from_utf8_lossy(&second.stdout).trim().to_string();
    assert_eq!(path_1, path_2, "should return the same path both times");
}

#[test]
fn add_with_local_worktree_dir_from_secondary_worktree() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let first = env.arbor(&["add", "feat-a"]).output().unwrap();
    assert!(first.status.success());
    let wt_a = String::from_utf8_lossy(&first.stdout).trim().to_string();

    let second = env
        .arbor_in(Path::new(&wt_a), &["add", "feat-b"])
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "add from secondary worktree should succeed, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let wt_b = String::from_utf8_lossy(&second.stdout).trim().to_string();

    // Both should be under the same override dir
    let parent_a = Path::new(&wt_a).parent().unwrap();
    let parent_b = Path::new(&wt_b).parent().unwrap();
    assert_eq!(
        fs::canonicalize(parent_a).unwrap(),
        fs::canonicalize(parent_b).unwrap(),
        "both worktrees should be under the same override dir"
    );
}

#[test]
fn remove_works_with_local_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();
    assert!(Path::new(&wt_path).exists());

    let rm_out = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(
        rm_out.status.success(),
        "remove should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after remove"
    );
}

#[test]
fn switch_works_with_local_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let add_path = fs::canonicalize(String::from_utf8_lossy(&add_out.stdout).trim()).unwrap();

    let switch_out = env.arbor(&["switch", "feat"]).output().unwrap();
    assert!(switch_out.status.success());

    let switch_path = fs::canonicalize(String::from_utf8_lossy(&switch_out.stdout).trim()).unwrap();
    assert_eq!(add_path, switch_path);
}

#[test]
fn dir_works_with_local_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let add_path = fs::canonicalize(String::from_utf8_lossy(&add_out.stdout).trim()).unwrap();

    let dir_out = env.arbor(&["dir", "feat"]).output().unwrap();
    assert!(dir_out.status.success());

    let dir_path = fs::canonicalize(String::from_utf8_lossy(&dir_out.stdout).trim()).unwrap();
    assert_eq!(add_path, dir_path);
}

// ── remove interactive / dot ────────────────────────────────────────

#[test]
fn remove_no_arg_requires_terminal() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

    let output = env.arbor(&["rm"]).output().unwrap();
    assert!(
        !output.status.success(),
        "rm with no arg and piped stdin should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Interactive terminal required"),
        "should mention terminal requirement, got: {stderr}"
    );
}

#[test]
fn remove_dot_removes_current_worktree() {
    let env = TestEnv::new();
    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();
    assert!(Path::new(&wt_path).exists());

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", "."])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm . should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after rm ."
    );

    let printed = fs::canonicalize(String::from_utf8_lossy(&rm_out.stdout).trim()).unwrap();
    let expected = fs::canonicalize(env.repo.path()).unwrap();
    assert_eq!(printed, expected, "should print the repo toplevel path");
}

#[test]
fn remove_dot_outside_worktree_fails() {
    let env = TestEnv::new();
    let outside = TempDir::new().unwrap();
    let output = env.arbor_in(outside.path(), &["rm", "."]).output().unwrap();
    assert!(
        !output.status.success(),
        "rm . outside a git repo should fail"
    );
}

#[test]
fn remove_dot_with_delete_branch() {
    let env = TestEnv::new();
    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", ".", "-d"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm . -d should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("Deleted branch"),
        "should mention deleted branch, got: {stderr}"
    );
}

#[test]
fn remove_dot_from_subdirectory() {
    let env = TestEnv::new();
    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    let subdir = Path::new(&wt_path).join("nested/deep");
    fs::create_dir_all(&subdir).unwrap();

    let rm_out = env.arbor_in(&subdir, &["rm", "."]).output().unwrap();
    assert!(
        rm_out.status.success(),
        "rm . from subdirectory should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone"
    );
}

#[test]
fn remove_dot_dirty_worktree_fails() {
    let env = TestEnv::new();
    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", "."])
        .output()
        .unwrap();
    assert!(
        !rm_out.status.success(),
        "rm . on dirty worktree should fail"
    );
    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "should mention uncommitted changes, got: {stderr}"
    );
}

#[test]
fn remove_dot_force_dirty_worktree() {
    let env = TestEnv::new();
    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(add_out.status.success());
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", ".", "-f"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm . -f on dirty worktree should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after forced removal"
    );
}
