use anyhow::Result;

use crate::git;

pub fn run() -> Result<()> {
    git::worktree_prune()?;
    eprintln!("Stale worktrees pruned.");
    Ok(())
}
