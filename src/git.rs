use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

const STATUS_PREVIEW_LINES: usize = 4;

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to run: git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_inherited(args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to run: git {}", args.join(" ")))?;

    if !status.success() {
        bail!("git {} exited with status {}", args.join(" "), status);
    }

    Ok(())
}

pub fn repo_toplevel() -> Result<PathBuf> {
    let path =
        run_git(&["rev-parse", "--show-toplevel"], None).context("not inside a git repository")?;
    Ok(PathBuf::from(path))
}

pub fn repo_name() -> Result<String> {
    let toplevel = repo_toplevel()?;
    let name = toplevel
        .file_name()
        .context("repository path has no final component")?
        .to_string_lossy()
        .into_owned();
    Ok(name.strip_suffix(".git").unwrap_or(&name).to_string())
}

pub fn local_branch_exists(branch: &str, cwd: Option<&Path>) -> Result<bool> {
    let refspec = format!("refs/heads/{branch}");
    Ok(run_git(&["show-ref", "--verify", "--quiet", &refspec], cwd).is_ok())
}

pub fn remote_branch_exists(branch: &str, cwd: Option<&Path>) -> Result<bool> {
    let refspec = format!("refs/remotes/origin/{branch}");
    Ok(run_git(&["show-ref", "--verify", "--quiet", &refspec], cwd).is_ok())
}

pub fn worktree_add_existing(path: &Path, branch: &str, cwd: Option<&Path>) -> Result<()> {
    run_git(
        &["worktree", "add", &path.to_string_lossy(), branch],
        cwd,
    )?;
    Ok(())
}

pub fn worktree_add_new_branch(path: &Path, branch: &str, cwd: Option<&Path>) -> Result<()> {
    run_git(
        &["worktree", "add", "-b", branch, &path.to_string_lossy()],
        cwd,
    )?;
    Ok(())
}

pub fn create_tracking_branch(branch: &str, cwd: Option<&Path>) -> Result<()> {
    let remote_ref = format!("origin/{branch}");
    run_git(&["branch", "--track", branch, &remote_ref], cwd)?;
    Ok(())
}

pub fn worktree_list_porcelain(cwd: Option<&Path>) -> Result<String> {
    run_git(&["worktree", "list", "--porcelain"], cwd)
}

pub fn worktree_remove(path: &Path, force: bool) -> Result<()> {
    let path_str = path.to_string_lossy();
    if force {
        run_git_inherited(&["worktree", "remove", "--force", &path_str], None)
    } else {
        run_git_inherited(&["worktree", "remove", &path_str], None)
    }
}

pub fn worktree_prune() -> Result<String> {
    let output = Command::new("git")
        .args(["worktree", "prune", "--verbose"])
        .output()
        .context("failed to run: git worktree prune --verbose")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree prune failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
}

pub fn clone_bare(url: &str, dest: &Path) -> Result<()> {
    run_git_inherited(&["clone", "--bare", url, &dest.to_string_lossy()], None)
}

pub fn configure_bare_fetch(repo_path: &Path) -> Result<()> {
    run_git(
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
        Some(repo_path),
    )?;
    Ok(())
}

pub fn fetch_origin(repo_path: &Path) -> Result<()> {
    run_git_inherited(&["fetch", "origin"], Some(repo_path))
}

pub fn status_porcelain(cwd: &Path) -> Result<String> {
    run_git(&["status", "--porcelain"], Some(cwd))
}

pub fn ahead_behind(cwd: &Path) -> Option<(usize, usize)> {
    let output = run_git(
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        Some(cwd),
    )
    .ok()?;

    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() == 2 {
        let behind = parts[0].parse().ok()?;
        let ahead = parts[1].parse().ok()?;
        Some((ahead, behind))
    } else {
        None
    }
}

pub fn head_branch(repo_path: &Path) -> Result<String> {
    let output = run_git(&["symbolic-ref", "HEAD"], Some(repo_path))?;
    Ok(output
        .strip_prefix("refs/heads/")
        .unwrap_or(&output)
        .to_string())
}

pub fn delete_branch(branch: &str, cwd: Option<&Path>) -> Result<()> {
    run_git(&["branch", "-d", branch], cwd)?;
    Ok(())
}

pub fn is_worktree_dirty(path: &Path) -> bool {
    status_porcelain(path)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn parse_worktree_list(porcelain: &str) -> Vec<(PathBuf, Option<String>, bool)> {
    let mut results = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut current_bare = false;

    for line in porcelain.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(p) = current_path.take() {
                results.push((p, current_branch.take(), current_bare));
            }
            current_path = Some(PathBuf::from(path));
            current_branch = None;
            current_bare = false;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = Some(
                branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string(),
            );
        } else if line == "bare" {
            current_bare = true;
        }
    }

    if let Some(p) = current_path {
        results.push((p, current_branch, current_bare));
    }

    results
}

pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub status_preview: Vec<String>,
    pub tracking: Option<(usize, usize)>,
}

impl WorktreeInfo {
    pub fn is_dirty(&self) -> bool {
        !self.status_preview.is_empty()
    }

    pub fn status_truncated(&self) -> bool {
        self.status_preview.len() > STATUS_PREVIEW_LINES
    }
}

pub fn worktree_infos(cwd: Option<&Path>) -> Result<Vec<WorktreeInfo>> {
    let porcelain = worktree_list_porcelain(cwd)?;
    let entries = parse_worktree_list(&porcelain);

    let mut results = Vec::new();
    for (path, branch, is_bare) in entries {
        if is_bare {
            continue;
        }

        let tracking = ahead_behind(&path);
        results.push(WorktreeInfo {
            status_preview: status_preview(&path),
            path,
            branch,
            tracking,
        });
    }

    Ok(results)
}

fn status_preview(path: &Path) -> Vec<String> {
    match status_porcelain(path) {
        Ok(output) if !output.is_empty() => output
            .lines()
            .take(STATUS_PREVIEW_LINES + 1)
            .map(|line| line.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_worktree() {
        let input = "\
worktree /home/user/project
HEAD abc1234
branch refs/heads/main
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, PathBuf::from("/home/user/project"));
        assert_eq!(result[0].1.as_deref(), Some("main"));
        assert!(!result[0].2);
    }

    #[test]
    fn parse_multiple_worktrees() {
        let input = "\
worktree /home/user/project
HEAD abc1234
branch refs/heads/main

worktree /home/user/.arbor/worktrees/project/feature-auth
HEAD def5678
branch refs/heads/feature/auth
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1.as_deref(), Some("main"));
        assert_eq!(
            result[1].0,
            PathBuf::from("/home/user/.arbor/worktrees/project/feature-auth")
        );
        assert_eq!(result[1].1.as_deref(), Some("feature/auth"));
    }

    #[test]
    fn parse_empty_input() {
        let result = parse_worktree_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_mixed_bare_normal_and_detached() {
        let input = "\
worktree /home/user/.arbor/repos/project.git
HEAD 0000000000000000000000000000000000000000
bare

worktree /home/user/.arbor/worktrees/project/main
HEAD abc1234abc1234abc1234abc1234abc1234abc12
branch refs/heads/main

worktree /home/user/.arbor/worktrees/project/hotfix
HEAD def5678def5678def5678def5678def5678def56
detached
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 3);

        assert_eq!(
            result[0].0,
            PathBuf::from("/home/user/.arbor/repos/project.git")
        );
        assert_eq!(result[0].1, None);
        assert!(result[0].2);

        assert_eq!(result[1].1.as_deref(), Some("main"));
        assert!(!result[1].2);

        assert_eq!(
            result[2].0,
            PathBuf::from("/home/user/.arbor/worktrees/project/hotfix")
        );
        assert_eq!(result[2].1, None);
        assert!(!result[2].2);
    }

    #[test]
    fn parse_worktree_path_with_spaces() {
        let input = "\
worktree /Users/jane doe/My Projects/cool app
HEAD abc1234
branch refs/heads/develop
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].0,
            PathBuf::from("/Users/jane doe/My Projects/cool app")
        );
        assert_eq!(result[0].1.as_deref(), Some("develop"));
    }
}
