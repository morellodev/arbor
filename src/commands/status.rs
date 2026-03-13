use anyhow::Result;

use crate::{display, git};

pub fn run(short: bool) -> Result<()> {
    let worktrees = git::worktree_infos(None)?;
    if worktrees.is_empty() {
        eprintln!("No worktrees found.");
        return Ok(());
    }

    let label = git::repo_name()
        .map(|name| name.to_string())
        .unwrap_or_else(|_| "Status".to_string());

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary(&label, &summary));

    if short {
        display::print_short_table(&worktrees);
    } else {
        display::print_table(&worktrees, true);
    }
    Ok(())
}
