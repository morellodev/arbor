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
        fs::write(
            home.path().join(".arbor/config.toml"),
            format!(
                "worktree_dir = \"{}\"\nrepos_dir = \"{}\"\n",
                worktree_dir.display(),
                repos_dir.display(),
            ),
        )
        .unwrap();

        git(&repo, &["init"]);
        git(&repo, &["config", "user.email", "test@test.com"]);
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["commit", "--allow-empty", "-m", "init"]);

        Self { home, repo }
    }

    fn arbor(&self, args: &[&str]) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("arbor").unwrap();
        cmd.current_dir(self.repo.path());
        cmd.env("HOME", self.home.path());
        cmd.args(args);
        cmd
    }

    fn worktree_dir(&self) -> std::path::PathBuf {
        self.home.path().join(".arbor/worktrees")
    }
}

fn git(dir: &TempDir, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir.path())
        .output()
        .unwrap();
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

    let repo_name = env
        .repo
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let expected_dir = env.worktree_dir().join(&repo_name).join("feature-auth");
    assert!(
        expected_dir.exists(),
        "expected worktree at {} (slashes replaced with dashes)",
        expected_dir.display()
    );
}

// ── dir ──────────────────────────────────────────────────────────────

#[test]
fn dir_prints_worktree_path() {
    let env = TestEnv::new();

    let add_out = env.arbor(&["add", "feat"]).output().unwrap();
    let add_path = String::from_utf8_lossy(&add_out.stdout).trim().to_string();

    let cd_out = env.arbor(&["dir", "feat"]).output().unwrap();
    assert!(cd_out.status.success());

    let cd_path = String::from_utf8_lossy(&cd_out.stdout).trim().to_string();
    assert_eq!(add_path, cd_path);
}

#[test]
fn dir_nonexistent_branch_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["dir", "nonexistent"]).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no worktree found"),
        "expected error message, got: {stderr}"
    );
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
        stderr.contains("no worktree found"),
        "expected error message, got: {stderr}"
    );
}

// ── list ─────────────────────────────────────────────────────────────

#[test]
fn list_shows_worktrees() {
    let env = TestEnv::new();
    env.arbor(&["add", "feat"]).output().unwrap();

    let output = env.arbor(&["list"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[feat]"),
        "list output should contain the branch name, got: {stdout}"
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

// ── prune ────────────────────────────────────────────────────────────

#[test]
fn prune_succeeds_on_clean_repo() {
    let env = TestEnv::new();
    let output = env.arbor(&["prune"]).output().unwrap();
    assert!(output.status.success());
}
