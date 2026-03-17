use std::io::IsTerminal;

use anyhow::{Result, bail};
use dialoguer::FuzzySelect;

use crate::{display, git};

pub fn run(branch: Option<&str>) -> Result<()> {
    if let Some(branch) = branch {
        let (path, _) = match git::resolve_worktree_branch(branch, None) {
            Ok(result) => result,
            Err(_) => {
                bail!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
            }
        };
        println!("{}", path.display());
        return Ok(());
    }

    if !std::io::stdin().is_terminal() {
        bail!(
            "Interactive terminal required. Use `arbor dir <branch>` to print a path non-interactively."
        );
    }

    let worktrees = git::worktree_infos(None)?;

    if worktrees.is_empty() {
        display::print_note("No worktrees found");
        return Ok(());
    }

    if worktrees.len() == 1 {
        println!("{}", worktrees[0].path.display());
        return Ok(());
    }

    let items = display::format_worktree_items(&worktrees);

    let selection = FuzzySelect::new()
        .with_prompt("Print path for worktree")
        .items(&items)
        .interact_opt()?;

    let idx = match selection {
        Some(idx) => idx,
        None => {
            display::print_note("Nothing selected");
            return Ok(());
        }
    };

    println!("{}", worktrees[idx].path.display());

    Ok(())
}
