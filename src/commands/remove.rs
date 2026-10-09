use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

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
    let mut worktrees: Vec<_> = git::parse_worktree_list(&porcelain)
        .into_iter()
        .filter(|wt| !wt.bare)
        .collect();

    match display::innermost_containing(&cwd, worktrees.iter().map(|wt| wt.path.as_path())) {
        Some(idx) => {
            let wt = worktrees.swap_remove(idx);
            Ok((wt.path, wt.branch))
        }
        None => bail!("Not inside a worktree"),
    }
}

fn resolve_branch(branch: &str) -> Result<(PathBuf, Option<String>)> {
    let (path, actual) = git::resolve_worktree_branch(branch, None)?;
    Ok((path, Some(actual)))
}

fn remove_worktree(
    wt_path: &Path,
    actual_branch: Option<&str>,
    force: bool,
    delete_branch: bool,
) -> Result<()> {
    if !force {
        if git::is_worktree_dirty(wt_path) {
            bail!("Worktree has uncommitted changes. Use --force to remove anyway.");
        }
        // `git worktree remove` doesn't refuse a rebase or bisect in progress.
        if let Some(op) = git::operation_in_progress(wt_path) {
            bail!(
                "Worktree has a {} in progress. Use --force to remove anyway.",
                op.operation
            );
        }
    }

    let toplevel = display::escape_dir_if_cwd_inside(wt_path)?;

    if let Some(ref dir) = toplevel {
        std::env::set_current_dir(dir)?;
    }

    git::worktree_remove(wt_path, force)?;
    display::print_ok(&format!("Removed {}", display::shorten_path(wt_path)));

    let mut result = Ok(());
    if delete_branch {
        if let Some(branch) = actual_branch {
            match git::delete_branch(branch, toplevel.as_deref()) {
                Ok(output) => {
                    let hash = parse_was_hash(&output);
                    let suffix = hash.map_or(String::new(), |h| format!(" (was {h})"));
                    display::print_ok(&format!("Deleted branch '{branch}'{suffix}"));
                }
                Err(e) => result = Err(e.context(format!("Could not delete branch '{branch}'"))),
            }
        } else {
            display::print_note("Skipped branch deletion (detached HEAD)");
        }
    }

    // Printed even on failure: the worktree is gone, so the shell must still leave it.
    if let Some(toplevel) = toplevel {
        println!("{}", toplevel.display());
    }

    result
}
