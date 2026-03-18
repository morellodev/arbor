use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use dialoguer::FuzzySelect;

use crate::{display, git};

fn parse_was_hash(output: &str) -> Option<&str> {
    let start = output.find("(was ")? + 5;
    let end = output[start..].find(')')? + start;
    Some(&output[start..end])
}

pub fn run(branch: Option<&str>, force: bool, delete_branch: bool) -> Result<()> {
    let (wt_path, actual_branch) = match branch {
        Some(".") => resolve_dot()?,
        Some(branch) => resolve_branch(branch)?,
        None => select_interactively()?,
    };

    remove_worktree(&wt_path, &actual_branch, force, delete_branch)
}

fn resolve_dot() -> Result<(PathBuf, String)> {
    let cwd = std::env::current_dir()?;
    let porcelain = git::worktree_list_porcelain(None)?;
    let worktrees = git::parse_worktree_list(&porcelain);

    let wt = worktrees
        .into_iter()
        .filter(|wt| !wt.bare)
        .find(|wt| display::cwd_is_inside(&cwd, &wt.path));

    match wt {
        Some(wt) => {
            let branch = wt
                .branch
                .ok_or_else(|| anyhow::anyhow!("Current worktree is in detached HEAD state"))?;
            Ok((wt.path, branch))
        }
        None => bail!("Not inside a worktree"),
    }
}

fn resolve_branch(branch: &str) -> Result<(PathBuf, String)> {
    match git::resolve_worktree_branch(branch, None) {
        Ok(result) => Ok(result),
        Err(_) => {
            bail!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
        }
    }
}

fn select_interactively() -> Result<(PathBuf, String)> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "Interactive terminal required. Use `arbor remove <branch>` to remove non-interactively."
        );
    }

    let worktrees = git::worktree_infos(None)?;

    if worktrees.is_empty() {
        bail!("No worktrees found");
    }

    let items = display::format_worktree_items(&worktrees);

    let selection = FuzzySelect::new()
        .with_prompt("Remove worktree")
        .items(&items)
        .interact_opt()?;

    let idx = match selection {
        Some(idx) => idx,
        None => bail!("Nothing selected"),
    };

    let wt = &worktrees[idx];
    let branch = wt
        .branch
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Selected worktree is in detached HEAD state"))?
        .clone();
    Ok((wt.path.clone(), branch))
}

fn remove_worktree(
    wt_path: &Path,
    actual_branch: &str,
    force: bool,
    delete_branch: bool,
) -> Result<()> {
    if !force && git::is_worktree_dirty(wt_path) {
        bail!("Worktree has uncommitted changes. Use --force to remove anyway.");
    }

    let toplevel = std::env::current_dir()
        .ok()
        .filter(|cwd| display::cwd_is_inside(cwd, wt_path))
        .map(|_| git::repo_toplevel())
        .transpose()?;

    git::worktree_remove(wt_path, force)?;
    display::print_ok(&format!("Removed {}", display::shorten_path(wt_path)));

    if delete_branch {
        match git::delete_branch(actual_branch, force, toplevel.as_deref()) {
            Ok(output) => {
                let hash = parse_was_hash(&output);
                let suffix = hash.map_or(String::new(), |h| format!(" (was {h})"));
                display::print_ok(&format!("Deleted branch '{actual_branch}'{suffix}"));
            }
            Err(e) => {
                display::print_error(&format!("Could not delete branch '{actual_branch}': {e}"))
            }
        }
    }

    if let Some(toplevel) = toplevel {
        println!("{}", toplevel.display());
    }

    Ok(())
}
