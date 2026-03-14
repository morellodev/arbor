use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::runner::{run_git, run_git_inherited, run_git_output};
use super::types::{WorktreeInfo, parse_worktree_list, sanitize_branch};

pub fn repo_toplevel() -> Result<PathBuf> {
    let porcelain = run_git(&["worktree", "list", "--porcelain"], None)
        .context("Not inside a git repository")?;
    let first_line = porcelain
        .lines()
        .next()
        .context("Empty worktree list output")?;
    let path = first_line
        .strip_prefix("worktree ")
        .context("Unexpected worktree list format")?;
    Ok(PathBuf::from(path))
}

pub fn repo_name_or_unknown() -> String {
    repo_name().unwrap_or_else(|_| "unknown".to_string())
}

pub fn repo_name() -> Result<String> {
    let toplevel = repo_toplevel()?;
    let name = toplevel
        .file_name()
        .context("Repository path has no final component")?
        .to_string_lossy()
        .into_owned();
    Ok(super::strip_git_suffix(&name).to_string())
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
    run_git(&["worktree", "add", &path.to_string_lossy(), branch], cwd)?;
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
    let output = run_git_output(&["worktree", "prune", "--verbose"], None)?;
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

pub fn delete_branch(branch: &str, force: bool, cwd: Option<&Path>) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };
    run_git(&["branch", flag, branch], cwd)?;
    Ok(())
}

pub fn is_worktree_dirty(path: &Path) -> bool {
    status_porcelain(path)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn worktree_infos(cwd: Option<&Path>) -> Result<Vec<WorktreeInfo>> {
    let porcelain = worktree_list_porcelain(cwd)?;
    let entries = parse_worktree_list(&porcelain);

    let mut results = Vec::new();
    for entry in entries {
        if entry.bare {
            continue;
        }

        let tracking = ahead_behind(&entry.path);
        let dirty = is_worktree_dirty(&entry.path);
        results.push(WorktreeInfo {
            path: entry.path,
            branch: entry.branch,
            dirty,
            tracking,
        });
    }

    Ok(results)
}

pub fn resolve_worktree_branch(branch: &str, cwd: Option<&Path>) -> Result<(PathBuf, String)> {
    let porcelain = worktree_list_porcelain(cwd)?;
    let worktrees = parse_worktree_list(&porcelain);

    let sanitized_input = sanitize_branch(branch);
    let mut sanitized_match = None;

    for wt in &worktrees {
        if let Some(b) = wt.branch.as_deref() {
            if b == branch {
                return Ok((wt.path.clone(), branch.to_string()));
            }
            if sanitized_match.is_none() && sanitize_branch(b) == sanitized_input {
                sanitized_match = Some((wt.path.clone(), b.to_string()));
            }
        }
    }

    sanitized_match.ok_or_else(|| anyhow::anyhow!("No worktree found for branch '{branch}'"))
}
