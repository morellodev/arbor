use anyhow::Result;

use crate::{display, git};

pub fn run() -> Result<()> {
    let pruned = git::worktree_prune()?;
    if pruned.is_empty() {
        display::print_ok("Nothing to prune");
    } else {
        for entry in &pruned {
            display::print_note(&format!("Pruned '{}' ({})", entry.name, entry.reason));
        }
        let n = pruned.len();
        let noun = display::plural(n, "worktree", "worktrees");
        display::print_ok(&format!("Pruned {n} stale {noun}"));
    }
    Ok(())
}
