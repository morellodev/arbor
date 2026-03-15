use anyhow::{Result, bail};

use crate::git;

pub fn run(branch: &str) -> Result<()> {
    let (path, _) = match git::resolve_worktree_branch(branch, None) {
        Ok(result) => result,
        Err(_) => {
            bail!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
        }
    };
    println!("{}", path.display());
    Ok(())
}
