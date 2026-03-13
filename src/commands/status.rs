use anyhow::Result;

use crate::{display, git};

pub fn run(short: bool) -> Result<()> {
    let worktrees = git::worktree_infos(None)?;
    if worktrees.is_empty() {
        eprintln!("No worktrees found.");
        return Ok(());
    }

    let label = git::repo_name_or_unknown();

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary(&label, &summary));

    if short {
        display::print_short_table(&worktrees);
    } else {
        display::print_table(&worktrees);
    }
    Ok(())
}
