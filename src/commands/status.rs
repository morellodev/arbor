use anyhow::Result;

use crate::{display, git};

pub fn run() -> Result<()> {
    let worktrees = git::worktree_infos(None)?;
    if worktrees.is_empty() {
        eprintln!("No worktrees found.");
        return Ok(());
    }

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary("Status", &summary));
    display::print_table(&worktrees, true);
    Ok(())
}
