use std::io::IsTerminal;

use anyhow::{Result, bail};
use dialoguer::FuzzySelect;

use crate::{display, git};

pub fn run(branch: Option<&str>) -> Result<()> {
    if let Some(branch) = branch {
        return switch_to(branch);
    }

    if !std::io::stdin().is_terminal() {
        bail!(
            "Interactive terminal required. Use `arbor switch <branch>` to switch non-interactively."
        );
    }

    let worktrees = git::worktree_infos(None)?;

    if worktrees.is_empty() {
        display::print_note("No worktrees found");
        return Ok(());
    }

    if worktrees.len() == 1 {
        let wt = &worktrees[0];
        let branch = wt.branch.as_deref().unwrap_or("(detached)");
        display::print_ok(&format!(
            "Switched to '{branch}' at {}",
            display::shorten_path(&wt.path)
        ));
        display::print_path_hint(&wt.path);
        return Ok(());
    }

    let items = display::format_worktree_items(&worktrees);

    let selection = FuzzySelect::new()
        .with_prompt("Switch to worktree")
        .items(&items)
        .interact_opt()?;

    let idx = match selection {
        Some(idx) => idx,
        None => {
            display::print_note("Nothing selected");
            return Ok(());
        }
    };

    let wt = &worktrees[idx];
    let branch = wt.branch.as_deref().unwrap_or("(detached)");

    display::print_ok(&format!(
        "Switched to '{branch}' at {}",
        display::shorten_path(&wt.path)
    ));
    display::print_path_hint(&wt.path);

    Ok(())
}

fn switch_to(branch: &str) -> Result<()> {
    let (path, actual_branch) = match git::resolve_worktree_branch(branch, None) {
        Ok(result) => result,
        Err(_) => {
            bail!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
        }
    };

    display::print_ok(&format!(
        "Found '{actual_branch}' at {}",
        display::shorten_path(&path)
    ));

    display::print_path_hint(&path);

    Ok(())
}
