#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

pub struct TestEnv {
    pub home: TempDir,
    pub repo: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
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

    pub fn arbor(&self, args: &[&str]) -> assert_cmd::Command {
        self.arbor_in(self.repo.path(), args)
    }

    pub fn arbor_in(&self, dir: &Path, args: &[&str]) -> assert_cmd::Command {
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

    pub fn add_worktree(&self, branch: &str) -> String {
        let output = self.arbor(&["add", branch]).output().unwrap();
        assert!(
            output.status.success(),
            "setup: arbor add {branch} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        stdout_path(&output)
    }
}

pub fn stdout_path(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn stdout_canon(output: &std::process::Output) -> PathBuf {
    fs::canonicalize(stdout_path(output)).unwrap()
}

pub fn git_cmd(dir: &Path, args: &[&str], home: &std::path::Path) -> std::process::Output {
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

pub fn git(dir: &TempDir, args: &[&str], home: &std::path::Path) {
    git_cmd(dir.path(), args, home);
}

pub fn git_stdout(dir: &Path, args: &[&str], home: &std::path::Path) -> String {
    let output = git_cmd(dir, args, home);
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn commit_arbor_toml(env: &TestEnv, content: &str) {
    fs::write(env.repo.path().join(".arbor.toml"), content).unwrap();
    git(&env.repo, &["add", ".arbor.toml"], env.home.path());
    git(
        &env.repo,
        &["commit", "-m", "add .arbor.toml"],
        env.home.path(),
    );
}
