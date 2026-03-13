use anyhow::{Result, bail};
use colored::Colorize;

use crate::config::Config;
use crate::{display, git};

pub fn run(config: &Config, branch: &str, force: bool, delete_branch: bool) -> Result<()> {
    let repo_name = git::repo_name()?;
    let sanitized = git::sanitize_branch(branch);
    let wt_path = config.worktree_dir.join(&repo_name).join(&sanitized);

    if !wt_path.exists() {
        bail!("no worktree found at {}", wt_path.display());
    }

    if !force && git::is_worktree_dirty(&wt_path) {
        bail!(
            "worktree has uncommitted changes. Use --force to remove anyway."
        );
    }

    git::worktree_remove(&wt_path, force)?;
    display::print_ok(&format!(
        "Worktree removed: {}",
        display::shorten_path(&wt_path)
    ));

    if delete_branch {
        match git::delete_branch(branch, None) {
            Ok(()) => display::print_ok(&format!("Branch '{branch}' deleted.")),
            Err(e) => eprintln!(
                "{} Could not delete branch '{branch}': {e}",
                "warning:".yellow(),
            ),
        }
    }

    Ok(())
}
