use anyhow::Result;

use crate::{display, git};

pub fn run() -> Result<()> {
    let output = git::worktree_prune()?;
    if output.is_empty() {
        display::print_ok("Nothing to prune");
    } else {
        eprintln!("{output}");
        display::print_ok("Pruned stale worktrees");
    }
    Ok(())
}
