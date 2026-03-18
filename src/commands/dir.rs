use anyhow::{Context, Result};

use crate::{display, git};

pub fn run(branch: Option<&str>) -> Result<()> {
    if let Some(branch) = branch {
        let (path, _) = git::resolve_worktree_branch(branch, None).with_context(|| {
            format!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
        })?;
        println!("{}", path.display());
        return Ok(());
    }

    let Some(wt) = display::fuzzy_select_worktree(
        "Print path for worktree",
        "Use `arbor dir <branch>` to print a path non-interactively.",
    )?
    else {
        return Ok(());
    };

    println!("{}", wt.path.display());

    Ok(())
}
