use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

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
    // Bare repos cloned by arbor end in ".git" — strip it for the display name.
    Ok(name.strip_suffix(".git").unwrap_or(&name).to_string())
}

pub fn local_branch_exists(branch: &str) -> Result<bool> {
    let refspec = format!("refs/heads/{branch}");
    Ok(run_git(&["show-ref", "--verify", "--quiet", &refspec], None).is_ok())
}

pub fn remote_branch_exists(branch: &str) -> Result<bool> {
    let refspec = format!("refs/remotes/origin/{branch}");
    Ok(run_git(&["show-ref", "--verify", "--quiet", &refspec], None).is_ok())
}

pub fn worktree_add_existing(path: &Path, branch: &str) -> Result<()> {
    run_git(&["worktree", "add", &path.to_string_lossy(), branch], None)?;
    Ok(())
}

pub fn worktree_add_new_branch(path: &Path, branch: &str) -> Result<()> {
    run_git(
        &["worktree", "add", "-b", branch, &path.to_string_lossy()],
        None,
    )?;
    Ok(())
}

pub fn create_tracking_branch(branch: &str) -> Result<()> {
    let remote_ref = format!("origin/{branch}");
    run_git(&["branch", "--track", branch, &remote_ref], None)?;
    Ok(())
}

pub fn worktree_list_porcelain(cwd: Option<&Path>) -> Result<String> {
    run_git(&["worktree", "list", "--porcelain"], cwd)
}

pub fn worktree_list(cwd: Option<&Path>) -> Result<String> {
    run_git(&["worktree", "list"], cwd)
}

pub fn worktree_remove(path: &Path, force: bool) -> Result<()> {
    let path_str = path.to_string_lossy();
    if force {
        run_git_inherited(&["worktree", "remove", "--force", &path_str], None)
    } else {
        run_git_inherited(&["worktree", "remove", &path_str], None)
    }
}

pub fn worktree_prune() -> Result<()> {
    run_git_inherited(&["worktree", "prune", "--verbose"], None)
}

pub fn clone_bare(url: &str, dest: &Path) -> Result<()> {
    run_git_inherited(&["clone", "--bare", url, &dest.to_string_lossy()], None)
}

/// Bare clones don't set a fetch refspec, so `git fetch` won't pull remote branches
/// unless we configure it explicitly.
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

/// Returns `None` when no upstream is configured.
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

/// Branch is `None` for detached HEAD or bare worktree entries.
pub fn parse_worktree_list(porcelain: &str) -> Vec<(PathBuf, Option<String>)> {
    let mut results = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in porcelain.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(p) = current_path.take() {
                results.push((p, current_branch.take()));
            }
            current_path = Some(PathBuf::from(path));
            current_branch = None;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // refs/heads/main → main
            current_branch = Some(
                branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string(),
            );
        }
    }

    if let Some(p) = current_path {
        results.push((p, current_branch));
    }

    results
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

        assert_eq!(result[1].1.as_deref(), Some("main"));

        assert_eq!(
            result[2].0,
            PathBuf::from("/home/user/.arbor/worktrees/project/hotfix")
        );
        assert_eq!(result[2].1, None);
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
