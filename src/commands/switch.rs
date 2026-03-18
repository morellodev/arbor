use anyhow::{Context, Result};

use crate::{display, git};

pub fn run(branch: Option<&str>) -> Result<()> {
    if let Some(branch) = branch {
        return switch_to(branch);
    }

    let Some(wt) = display::fuzzy_select_worktree(
        "Switch to worktree",
        "Use `arbor switch <branch>` to switch non-interactively.",
    )?
    else {
        return Ok(());
    };

    let branch = wt.branch.as_deref().unwrap_or("(detached)");
    display::print_ok(&format!(
        "Switched to '{branch}' at {}",
        display::shorten_path(&wt.path)
    ));
    display::print_path_hint(&wt.path);

    Ok(())
}

fn switch_to(branch: &str) -> Result<()> {
    let (path, actual_branch) = git::resolve_worktree_branch(branch, None).with_context(|| {
        format!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
    })?;

    display::print_ok(&format!(
        "Found '{actual_branch}' at {}",
        display::shorten_path(&path)
    ));

    display::print_path_hint(&path);

    Ok(())
}
