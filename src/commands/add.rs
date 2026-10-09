use std::fs;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::{display, git, hooks};

pub fn run(config: &Config, branch: &str, base: Option<&str>, no_hooks: bool) -> Result<()> {
    let repo_root = git::repo_toplevel()?;
    let repo_name = git::strip_git_suffix(
        &repo_root
            .file_name()
            .context("Repository path has no final component")?
            .to_string_lossy(),
    )
    .to_string();
    let wt_path = resolve_wt_path(config, &repo_name, branch, &repo_root)?;

    let worktrees = git::parse_worktree_list(&git::worktree_list_porcelain(None)?);
    let with_branch = worktrees
        .iter()
        .find(|wt| !wt.bare && wt.branch.as_deref() == Some(branch) && wt.path.exists());

    let existing = match with_branch {
        // git reports symlink-resolved paths; keep the configured spelling when it's the same dir.
        Some(wt) if fs::canonicalize(&wt.path).ok() == fs::canonicalize(&wt_path).ok() => {
            Some(wt_path.clone())
        }
        Some(wt) => Some(wt.path.clone()),
        None if wt_path.exists() => Some(wt_path.clone()),
        None => None,
    };

    if let Some(existing) = existing {
        display::print_note(&format!(
            "Already exists at {}",
            display::shorten_path(&existing)
        ));
        display::print_path_hint(&existing);
        return Ok(());
    }

    fs::create_dir_all(
        wt_path
            .parent()
            .context("Worktree path has no parent directory")?,
    )
    .with_context(|| format!("Failed to create directory: {}", wt_path.display()))?;

    if git::local_branch_exists(branch, None)? {
        if base.is_some() {
            display::print_note("--base ignored — branch already exists locally");
        }
        git::worktree_add_existing(&wt_path, branch, None)?;
        display::print_ok(&format!(
            "Linked '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    } else if git::remote_branch_exists(branch, None)? {
        if base.is_some() {
            display::print_note("--base ignored — tracking existing remote branch");
        }
        git::create_tracking_branch(branch, None)?;
        git::worktree_add_existing(&wt_path, branch, None)?;
        display::print_ok(&format!(
            "Linked '{branch}' (tracking origin) at {}",
            display::shorten_path(&wt_path)
        ));
    } else {
        display::print_note(&format!(
            "No existing branch found — creating new branch '{branch}'"
        ));
        git::worktree_add_new_branch(&wt_path, branch, base, None)?;
        display::print_ok(&format!(
            "Created '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    }

    if !no_hooks {
        hooks::run_post_create(&hooks::HookContext {
            worktree_path: wt_path.clone(),
            branch: branch.to_string(),
            repo_name: repo_name.clone(),
        });
    }

    display::print_path_hint(&wt_path);
    Ok(())
}

fn resolve_wt_path(
    config: &Config,
    repo_name: &str,
    branch: &str,
    repo_root: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let raw_override = if git::is_bare_repository(repo_root) {
        hooks::load_worktree_dir_from_git(repo_root)?
    } else {
        hooks::load_worktree_dir_from_path(repo_root)?
    };
    let local_override = raw_override
        .map(|r| hooks::resolve_worktree_dir(&r, repo_root))
        .transpose()?;

    match local_override {
        Some(dir) => Ok(dir.join(git::sanitize_branch(branch))),
        None => Ok(config.worktree_path(repo_name, branch)),
    }
}
