use crate::config::Config;
use crate::{display, git};
use anyhow::{Result, bail};

pub fn run(config: &Config, branch: &str, force: bool, delete_branch: bool) -> Result<()> {
    let repo_name = git::repo_name()?;
    let wt_path = config.worktree_path(&repo_name, branch);

    if !wt_path.exists() {
        bail!("No worktree found at {}", wt_path.display());
    }

    let actual_branch = if delete_branch {
        Some(
            git::resolve_worktree_branch(branch, None)
                .map(|(_, b)| b)
                .unwrap_or_else(|_| branch.to_string()),
        )
    } else {
        None
    };

    if !force && git::is_worktree_dirty(&wt_path) {
        bail!("Worktree has uncommitted changes. Use --force to remove anyway.");
    }

    git::worktree_remove(&wt_path, force)?;
    display::print_ok(&format!("Removed {}", display::shorten_path(&wt_path)));

    if let Some(actual_branch) = actual_branch {
        match git::delete_branch(&actual_branch, force, None) {
            Ok(()) => display::print_ok(&format!("Deleted branch '{actual_branch}'")),
            Err(e) => {
                display::print_error(&format!("Could not delete branch '{actual_branch}': {e}"))
            }
        }
    }

    Ok(())
}
