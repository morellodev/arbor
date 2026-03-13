use anyhow::Result;

use crate::{display, git};

pub fn run() -> Result<()> {
    let output = git::worktree_prune()?;
    if output.is_empty() {
        display::print_ok("Nothing to prune — all worktrees are valid.");
    } else {
        eprintln!("{output}");
        display::print_ok("Stale worktrees pruned.");
    }
    Ok(())
}
