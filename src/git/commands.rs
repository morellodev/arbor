use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::runner::{
    run_git, run_git_inherited, run_git_output, run_git_output_with_env, run_git_with_input,
};
use super::types::{
    InProgress, PrunedWorktree, Tracking, WorktreeInfo, parse_prune_output, parse_worktree_list,
    sanitize_branch,
};

pub fn show_file_from_head(file: &str, cwd: &Path) -> Result<String> {
    run_git(&["show", &format!("HEAD:{file}")], Some(cwd))
}

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

pub fn worktree_add_new_branch(
    path: &Path,
    branch: &str,
    base: Option<&str>,
    cwd: Option<&Path>,
) -> Result<()> {
    let path_str = path.to_string_lossy();
    let mut args = vec!["worktree", "add", "-b", branch, &path_str];
    if let Some(b) = base {
        args.push(b);
    }
    run_git(&args, cwd)?;
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

pub fn worktree_prune() -> Result<Vec<PrunedWorktree>> {
    let output = run_git_output(&["worktree", "prune", "--verbose"], None)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_prune_output(&stderr))
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

/// `clone --bare` turns every remote branch into a local branch that fetches never update.
/// Keep only the default branch, tracking origin, so `add` builds the rest from fresh remote refs.
pub fn reset_bare_clone_branches(repo_path: &Path, default_branch: &str) -> Result<()> {
    let branches = run_git(
        &[
            "for-each-ref",
            "--format=%(refname:lstrip=2)",
            "refs/heads/",
        ],
        Some(repo_path),
    )?;
    // Fed through stdin: repos with thousands of branches overflow the argument limit.
    let deletions: String = branches
        .lines()
        .filter(|b| *b != default_branch)
        .map(|b| format!("delete refs/heads/{b}\n"))
        .collect();
    if !deletions.is_empty() {
        run_git_with_input(&["update-ref", "--stdin"], Some(repo_path), &deletions)?;
    }

    if remote_branch_exists(default_branch, Some(repo_path))? {
        let upstream = format!("origin/{default_branch}");
        run_git(
            &["branch", "--set-upstream-to", &upstream, default_branch],
            Some(repo_path),
        )?;
    }
    Ok(())
}

pub fn is_bare_repository(cwd: &Path) -> bool {
    run_git(&["rev-parse", "--is-bare-repository"], Some(cwd)).is_ok_and(|out| out == "true")
}

pub fn fetch_origin(repo_path: &Path) -> Result<()> {
    run_git_inherited(&["fetch", "origin"], Some(repo_path))
}

pub fn status_porcelain(cwd: &Path) -> Result<String> {
    run_git(&["status", "--porcelain"], Some(cwd))
}

pub fn ahead_behind(cwd: &Path) -> Option<Tracking> {
    let output = run_git(
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        Some(cwd),
    )
    .ok()?;

    // --left-right outputs: <upstream_count>\t<local_count>, i.e. behind\tahead
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() == 2 {
        let behind = parts[0].parse().ok()?;
        let ahead = parts[1].parse().ok()?;
        Some(Tracking { ahead, behind })
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

/// Refuses unmerged branches: removing the worktree also dropped its reflog.
pub fn delete_branch(branch: &str, cwd: Option<&Path>) -> Result<String> {
    run_git(&["branch", "-d", branch], cwd)
}

pub fn is_worktree_dirty(path: &Path) -> bool {
    status_porcelain(path)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn operation_in_progress(worktree: &Path) -> Option<InProgress> {
    let git_dir =
        PathBuf::from(run_git(&["rev-parse", "--absolute-git-dir"], Some(worktree)).ok()?);
    let (operation, head) = [
        ("rebase", "rebase-merge/head-name"),
        ("rebase", "rebase-apply/head-name"),
        ("bisect", "BISECT_START"),
    ]
    .into_iter()
    .find_map(|(operation, file)| {
        Some((operation, fs::read_to_string(git_dir.join(file)).ok()?))
    })?;
    // A rebase of a detached HEAD records "detached HEAD", a bisect records the commit.
    let head = head.trim();
    let name = head.strip_prefix("refs/heads/").unwrap_or(head);
    let branch = local_branch_exists(name, Some(worktree))
        .unwrap_or(false)
        .then(|| name.to_string());
    Some(InProgress { operation, branch })
}

pub fn worktree_infos(cwd: Option<&Path>) -> Result<Vec<WorktreeInfo>> {
    Ok(infos_from_porcelain(&worktree_list_porcelain(cwd)?))
}

/// Worktrees of the repo at `repo_path` itself. The ceiling stops git from climbing
/// into an enclosing repo (e.g. a dotfiles HOME) when `repo_path` isn't one.
pub fn repo_worktree_infos(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let ceiling = repo_path.parent().unwrap_or(repo_path);
    let output = run_git_output_with_env(
        &["worktree", "list", "--porcelain"],
        Some(repo_path),
        &[("GIT_CEILING_DIRECTORIES", ceiling.as_os_str())],
    )?;
    Ok(infos_from_porcelain(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn infos_from_porcelain(porcelain: &str) -> Vec<WorktreeInfo> {
    let entries = parse_worktree_list(porcelain);

    let mut results = Vec::new();
    for (i, entry) in entries.into_iter().enumerate() {
        if entry.bare {
            continue;
        }

        let tracking = ahead_behind(&entry.path);
        let dirty = is_worktree_dirty(&entry.path);
        // Checked on attached worktrees too: `bisect start --no-checkout` keeps HEAD on the branch.
        let in_progress = operation_in_progress(&entry.path);
        results.push(WorktreeInfo {
            path: entry.path,
            branch: entry.branch,
            dirty,
            tracking,
            main: i == 0,
            in_progress,
        });
    }

    results
}

pub fn resolve_worktree_branch(branch: &str, cwd: Option<&Path>) -> Result<(PathBuf, String)> {
    let porcelain = worktree_list_porcelain(cwd)?;
    let worktrees = parse_worktree_list(&porcelain);

    let sanitized_input = sanitize_branch(branch);
    let mut sanitized_match = None;

    for wt in &worktrees {
        let name = match &wt.branch {
            Some(b) => Some(b.clone()),
            None if !wt.bare => operation_in_progress(&wt.path).and_then(|op| op.branch),
            None => None,
        };
        if let Some(b) = name {
            if b == branch {
                return Ok((wt.path.clone(), b));
            }
            if sanitized_match.is_none() && sanitize_branch(&b) == sanitized_input {
                sanitized_match = Some((wt.path.clone(), b));
            }
        }
    }

    sanitized_match.ok_or_else(|| {
        anyhow::anyhow!(
            "No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?"
        )
    })
}
