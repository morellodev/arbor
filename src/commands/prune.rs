use anyhow::Result;

use crate::{display, git};

pub fn run() -> Result<()> {
    git::worktree_prune()?;
    display::print_ok("Stale worktrees pruned.");
    Ok(())
}
