use anyhow::{Result, bail};

use crate::git;

pub fn run(branch: &str) -> Result<()> {
    let porcelain = git::worktree_list_porcelain(None)?;
    let worktrees = git::parse_worktree_list(&porcelain);

    for (path, wt_branch) in &worktrees {
        if wt_branch.as_deref() == Some(branch) {
            println!("{}", path.display());
            return Ok(());
        }
    }

    bail!("no worktree found for branch '{branch}'");
}
