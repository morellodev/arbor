use std::io::IsTerminal;
use std::path::PathBuf;

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
) -> Result<Vec<PathBuf>> {
    let mut removed_paths = Vec::new();
    let mut branches_deleted = 0;

    for &idx in selections {
        let wt = &worktrees[idx];
        let short_path = display::shorten_path(&wt.path);

        let force = wt.dirty;
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
            "Cleaned {removed} worktree{}",
            if removed == 1 { "" } else { "s" }
        );
        if branches_deleted > 0 {
            summary.push_str(&format!(
                ", deleted {branches_deleted} branch{}",
                if branches_deleted == 1 { "" } else { "es" }
            ));
        }
        display::print_ok(&summary);
    }

    Ok(removed_paths)
}

pub fn run(delete_branch: bool) -> Result<()> {
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
    let cwd = std::env::current_dir().ok();
    let cwd_worktree = cwd.as_ref().and_then(|cwd| {
        selections.iter().find_map(|&idx| {
            display::cwd_is_inside(cwd, &worktrees[idx].path).then(|| worktrees[idx].path.clone())
        })
    });
    let toplevel = if cwd_worktree.is_some() {
        Some(git::repo_toplevel()?)
    } else {
        None
    };

    let removed = remove_worktrees(&worktrees, &selections, delete_branch)?;

    if let Some(ref cwd_wt) = cwd_worktree
        && let Some(toplevel) = toplevel
        && removed.iter().any(|p| p == cwd_wt)
    {
        println!("{}", toplevel.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn make_worktree(
        branch: Option<&str>,
        dirty: bool,
        tracking: Option<(usize, usize)>,
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
        let wt = make_worktree(None, false, Some((0, 0)));
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn named_branch_clean_no_upstream_is_not_abandoned() {
        let wt = make_worktree(Some("feat"), false, None);
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn named_branch_with_tracking_is_not_abandoned() {
        let wt = make_worktree(Some("main"), false, Some((0, 0)));
        assert!(!is_abandoned(&wt));
    }

    #[test]
    fn named_dirty_branch_is_not_abandoned() {
        let wt = make_worktree(Some("feat"), true, Some((1, 0)));
        assert!(!is_abandoned(&wt));
    }
}
