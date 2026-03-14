use anyhow::Result;

use crate::{display, git};

pub fn run(short: bool) -> Result<()> {
    let worktrees = git::worktree_infos(None)?;
    if worktrees.is_empty() {
        display::print_note("No worktrees found.");
        return Ok(());
    }

    let label = git::repo_name_or_unknown();

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary(&label, &summary));

    display::print_table(&worktrees, !short);
    Ok(())
}
