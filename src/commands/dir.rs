use anyhow::{Result, bail};

use crate::git;

pub fn run(branch: &str) -> Result<()> {
    let porcelain = git::worktree_list_porcelain(None)?;
    let worktrees = git::parse_worktree_list(&porcelain);

    for (path, wt_branch, _) in &worktrees {
        if wt_branch.as_deref() == Some(branch) {
            println!("{}", path.display());
            return Ok(());
        }
    }

    let sanitized_input = git::sanitize_branch(branch);
    for (path, wt_branch, _) in &worktrees {
        if let Some(b) = wt_branch.as_deref() {
            if git::sanitize_branch(b) == sanitized_input {
                println!("{}", path.display());
                return Ok(());
            }
        }
    }

    bail!("no worktree found for branch '{branch}'");
}
