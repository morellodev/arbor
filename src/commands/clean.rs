use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use dialoguer::MultiSelect;

use crate::git::WorktreeInfo;
use crate::{display, git};

fn is_abandoned(wt: &WorktreeInfo) -> bool {
    wt.branch.is_none() && !wt.dirty && wt.tracking.is_none()
}

fn remove_worktrees(
    worktrees: &[WorktreeInfo],
    selections: &[usize],
    delete_branch: bool,
    force: bool,
) -> Result<Vec<PathBuf>> {
    let mut removed_paths = Vec::new();
    let mut branches_deleted = 0;

    for &idx in selections {
        let wt = &worktrees[idx];
        let short_path = display::shorten_path(&wt.path);

        match git::worktree_remove(&wt.path, force) {
            Ok(()) => {
                display::print_ok(&format!("Removed {short_path}"));
                removed_paths.push(wt.path.clone());

                if delete_branch && let Some(branch) = &wt.branch {
                    match git::delete_branch(branch, false, None) {
                        Ok(_) => {
                            display::print_ok(&format!("Deleted branch '{branch}'"));
                            branches_deleted += 1;
                        }
                        Err(e) => {
                            display::print_error(&format!(
                                "Could not delete branch '{branch}': {e}"
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                let label = wt.branch.as_deref().unwrap_or(&short_path);
                display::print_error(&format!("Failed to remove worktree for '{label}': {e}"));
            }
        }
    }

    let removed = removed_paths.len();
    if removed > 0 {
        let mut summary = format!(
            "Cleaned {removed} {}",
            display::plural(removed, "worktree", "worktrees")
        );
        if branches_deleted > 0 {
            summary.push_str(&format!(
                ", deleted {branches_deleted} {}",
                display::plural(branches_deleted, "branch", "branches")
            ));
        }
        display::print_ok(&summary);
    }

    Ok(removed_paths)
}

pub fn run(delete_branch: bool, force: bool) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "Interactive terminal required. Use `arbor rm` to remove worktrees non-interactively."
        );
    }

    git::worktree_prune()?;

    let worktrees = git::worktree_infos(None)?;

    if worktrees.len() <= 1 {
        display::print_ok("Nothing to clean");
        return Ok(());
    }

    let items = display::format_worktree_items(&worktrees);
    let defaults: Vec<bool> = worktrees.iter().map(is_abandoned).collect();

    let selections = MultiSelect::new()
        .with_prompt("Select worktrees to remove (Space to toggle, Enter to confirm)")
        .report(false)
        .items(&items)
        .defaults(&defaults)
        .interact_opt()?;

    let selections = match selections {
        Some(s) if !s.is_empty() => s,
        _ => {
            display::print_note("Nothing selected");
            return Ok(());
        }
    };
    let selected: Vec<&Path> = selections
        .iter()
        .map(|&idx| worktrees[idx].path.as_path())
        .collect();
    let cwd_worktree = std::env::current_dir()
        .ok()
        .and_then(|cwd| display::innermost_containing(&cwd, selected.iter().copied()))
        .map(|idx| selected[idx]);

    // Leave the cwd before deleting it, or every later git call in this process fails.
    let toplevel = match cwd_worktree {
        Some(path) => display::escape_dir_if_cwd_inside(path)?,
        None => None,
    };
    if let Some(dir) = &toplevel {
        std::env::set_current_dir(dir)?;
    }

    let removed = remove_worktrees(&worktrees, &selections, delete_branch, force)?;

    if let (Some(toplevel), Some(path)) = (toplevel, cwd_worktree)
        && removed.iter().any(|p| p == path)
    {
        println!("{}", toplevel.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::git::Tracking;

    use super::*;

    fn make_worktree(
        branch: Option<&str>,
        dirty: bool,
        tracking: Option<Tracking>,
    ) -> WorktreeInfo {
        WorktreeInfo {
            path: PathBuf::from("/tmp/test"),
            branch: branch.map(String::from),
            dirty,
            tracking,
        }
    }

    #[test]
    fn detached_clean_no_upstream_is_abandoned() {
        let wt = make_worktree(None, false, None);
        assert!(is_abandoned(&wt));
    }

    #[test]
    fn detached_dirty_is_not_abandoned() {
        let wt = make_worktree(None, true, None);
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn detached_with_upstream_is_not_abandoned() {
        let wt = make_worktree(
            None,
            false,
            Some(Tracking {
                ahead: 0,
                behind: 0,
            }),
        );
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn named_branch_clean_no_upstream_is_not_abandoned() {
        let wt = make_worktree(Some("feat"), false, None);
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn named_branch_with_tracking_is_not_abandoned() {
        let wt = make_worktree(
            Some("main"),
            false,
            Some(Tracking {
                ahead: 0,
                behind: 0,
            }),
        );
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn named_dirty_branch_is_not_abandoned() {
        let wt = make_worktree(
            Some("feat"),
            true,
            Some(Tracking {
                ahead: 1,
                behind: 0,
            }),
        );
        assert!(!is_abandoned(&wt));
    }
}
