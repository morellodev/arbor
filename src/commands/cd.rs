use anyhow::{bail, Result};

use crate::config::Config;
use crate::git;

pub fn run(config: &Config, branch: &str) -> Result<()> {
    let repo_name = git::repo_name()?;
    let sanitized = branch.replace('/', "-");
    let wt_path = config.worktree_dir.join(&repo_name).join(&sanitized);

    if !wt_path.exists() {
        bail!("no worktree found for branch '{branch}' (expected at {})", wt_path.display());
    }

    println!("{}", wt_path.display());
    Ok(())
}
