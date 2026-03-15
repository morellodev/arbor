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

        git(&repo, &["init"], home.path());
        git(
            &repo,
            &["config", "user.email", "test@test.com"],
            home.path(),
        );
        git(&repo, &["config", "user.name", "Test"], home.path());
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
        #[cfg(windows)]
        cmd.env("USERPROFILE", self.home.path());
        cmd.args(args);
        cmd
    }
}

fn git(dir: &TempDir, args: &[&str], home: &std::path::Path) {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(dir.path())
        .env("HOME", home)
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    cmd.env("USERPROFILE", home);
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
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
    assert!(first.status.success());

    let second = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(second.status.success());

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
    let output = Command::new("git")
        .args(["commit", "--allow-empty", "-m", "wip"])
        .current_dir(&wt_path)
        .env("HOME", env.home.path())
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(output.status.success());

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
}

// ── status ───────────────────────────────────────────────────────────

#[test]
fn status_shows_clean_worktree() {
    let env = TestEnv::new();
    let output = env.arbor(&["status"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("clean"),
        "fresh repo should be clean, got: {stdout}"
    );
}

#[test]
fn status_short_omits_paths() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

    let output = env.arbor(&["status", "--short"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // --short should not include the "Path" column header
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
fn status_detects_dirty_worktree() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let wt_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    // Make the worktree dirty
    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let output = env.arbor(&["status"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dirty"),
        "should detect dirty worktree, got: {stdout}"
    );
}

#[test]
fn status_all_succeeds_with_no_repos() {
    let env = TestEnv::new();
    let output = env.arbor(&["status", "--all"]).output().unwrap();
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
