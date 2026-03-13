use anyhow::{Result, bail};

use crate::config::Config;
use crate::{display, git};

pub fn run(config: &Config, branch: &str, force: bool) -> Result<()> {
    let repo_name = git::repo_name()?;
    let sanitized = branch.replace('/', "-");
    let wt_path = config.worktree_dir.join(&repo_name).join(&sanitized);

    if !wt_path.exists() {
        bail!("no worktree found at {}", wt_path.display());
    }

    git::worktree_remove(&wt_path, force)?;
    display::print_ok(&format!("Worktree removed: {}", wt_path.display()));
    Ok(())
}
