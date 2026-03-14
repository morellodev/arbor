use anyhow::{Result, bail};

use crate::{display, git};

pub fn run(branch: &str) -> Result<()> {
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
