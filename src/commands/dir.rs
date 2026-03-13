use anyhow::{Result, bail};

use crate::git;

pub fn run(branch: &str) -> Result<()> {
    let porcelain = git::worktree_list_porcelain(None)?;
    let worktrees = git::parse_worktree_list(&porcelain);

    for wt in &worktrees {
        if wt.branch.as_deref() == Some(branch) {
            println!("{}", wt.path.display());
            return Ok(());
        }
    }

    let sanitized_input = git::sanitize_branch(branch);
    for wt in &worktrees {
        if let Some(b) = wt.branch.as_deref()
            && git::sanitize_branch(b) == sanitized_input
        {
            println!("{}", wt.path.display());
            return Ok(());
        }
    }

    bail!("no worktree found for branch '{branch}'");
}
