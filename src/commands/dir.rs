use anyhow::Result;

use crate::git;

pub fn run(branch: &str) -> Result<()> {
    let (path, _) = git::resolve_worktree_branch(branch, None)?;
    println!("{}", path.display());
    Ok(())
}
