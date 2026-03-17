use std::fs;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::{display, git, hooks};

pub fn run(
    config: &Config,
    branch: &str,
    base: Option<&str>,
    repo: Option<&str>,
    no_hooks: bool,
) -> Result<()> {
    let (repo_name, repo_cwd) = resolve_repo(config, repo)?;
    let wt_path = resolve_wt_path(config, &repo_name, branch, repo_cwd.as_deref())?;

    if wt_path.exists() {
        display::print_note(&format!(
            "Already exists at {}",
            display::shorten_path(&wt_path)
        ));
        display::print_path_hint(&wt_path);
        return Ok(());
    }

    fs::create_dir_all(wt_path.parent().unwrap())
        .with_context(|| format!("Failed to create directory: {}", wt_path.display()))?;

    let cwd = repo_cwd.as_deref();

    if git::local_branch_exists(branch, cwd)? {
        if base.is_some() {
            display::print_note("--base ignored — branch already exists locally");
        }
        git::worktree_add_existing(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Linked '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    } else if git::remote_branch_exists(branch, cwd)? {
        if base.is_some() {
            display::print_note("--base ignored — tracking existing remote branch");
        }
        git::create_tracking_branch(branch, cwd)?;
        git::worktree_add_existing(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Linked '{branch}' (tracking origin) at {}",
            display::shorten_path(&wt_path)
        ));
    } else {
        display::print_note(&format!(
            "No existing branch found — creating new branch '{branch}'"
        ));
        git::worktree_add_new_branch(&wt_path, branch, base, cwd)?;
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
            event: "post_create".to_string(),
        });
    }

    display::print_path_hint(&wt_path);
    Ok(())
}

fn resolve_wt_path(
    config: &Config,
    repo_name: &str,
    branch: &str,
    repo_cwd: Option<&std::path::Path>,
) -> Result<std::path::PathBuf> {
    let local_override = match repo_cwd {
        Some(bare_path) => {
            let raw = hooks::load_worktree_dir_from_git(bare_path)?;
            raw.map(|r| hooks::resolve_worktree_dir(&r, bare_path))
                .transpose()?
        }
        None => {
            let repo_root = git::repo_toplevel()?;
            let raw = hooks::load_worktree_dir_from_path(&repo_root)?;
            raw.map(|r| hooks::resolve_worktree_dir(&r, &repo_root))
                .transpose()?
        }
    };

    match local_override {
        Some(dir) => Ok(dir.join(git::sanitize_branch(branch))),
        None => Ok(config.worktree_path(repo_name, branch)),
    }
}

fn resolve_repo(
    config: &Config,
    repo: Option<&str>,
) -> Result<(String, Option<std::path::PathBuf>)> {
    match repo {
        Some(name) => {
            let bare_name = format!("{name}.git");
            let repo_path = config.repos_dir.join(&bare_name);
            if !repo_path.exists() {
                bail!("Repository '{name}' not found at {}", repo_path.display());
            }
            Ok((name.to_string(), Some(repo_path)))
        }
        None => {
            let name = git::repo_name()?;
            Ok((name, None))
        }
    }
}
