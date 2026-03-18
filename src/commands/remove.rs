use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

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
        None => {
            let Some(wt) = display::fuzzy_select_worktree(
                "Remove worktree",
                "Use `arbor remove <branch>` to remove non-interactively.",
            )?
            else {
                return Ok(());
            };
            (wt.path, wt.branch)
        }
    };

    remove_worktree(&wt_path, actual_branch.as_deref(), force, delete_branch)
}

fn resolve_dot() -> Result<(PathBuf, Option<String>)> {
    let cwd = std::env::current_dir()?;
    let porcelain = git::worktree_list_porcelain(None)?;
    let worktrees = git::parse_worktree_list(&porcelain);

    let wt = worktrees
        .into_iter()
        .filter(|wt| !wt.bare)
        .find(|wt| display::cwd_is_inside(&cwd, &wt.path));

    match wt {
        Some(wt) => Ok((wt.path, wt.branch)),
        None => bail!("Not inside a worktree"),
    }
}

fn resolve_branch(branch: &str) -> Result<(PathBuf, Option<String>)> {
    let (path, actual) = git::resolve_worktree_branch(branch, None).with_context(|| {
        format!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
    })?;
    Ok((path, Some(actual)))
}

fn remove_worktree(
    wt_path: &Path,
    actual_branch: Option<&str>,
    force: bool,
    delete_branch: bool,
) -> Result<()> {
    if !force && git::is_worktree_dirty(wt_path) {
        bail!("Worktree has uncommitted changes. Use --force to remove anyway.");
    }

    let toplevel = display::escape_dir_if_cwd_inside(wt_path)?;

    git::worktree_remove(wt_path, force)?;
    display::print_ok(&format!("Removed {}", display::shorten_path(wt_path)));

    if delete_branch {
        if let Some(branch) = actual_branch {
            match git::delete_branch(branch, force, toplevel.as_deref()) {
                Ok(output) => {
                    let hash = parse_was_hash(&output);
                    let suffix = hash.map_or(String::new(), |h| format!(" (was {h})"));
                    display::print_ok(&format!("Deleted branch '{branch}'{suffix}"));
                }
                Err(e) => display::print_error(&format!("Could not delete branch '{branch}': {e}")),
            }
        } else {
            display::print_note("Skipped branch deletion (detached HEAD)");
        }
    }

    if let Some(toplevel) = toplevel {
        println!("{}", toplevel.display());
    }

    Ok(())
}
