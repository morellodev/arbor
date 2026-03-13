use anyhow::Result;

use crate::git;

pub fn run() -> Result<()> {
    let porcelain = git::worktree_list_porcelain(None)?;
    let worktrees = git::parse_worktree_list(&porcelain);

    if worktrees.is_empty() {
        eprintln!("No worktrees found.");
        return Ok(());
    }

    for (path, branch) in &worktrees {
        let branch_label = branch.as_deref().unwrap_or("(detached)");

        let dirty = match git::status_porcelain(path) {
            Ok(output) => !output.is_empty(),
            Err(_) => false,
        };

        let state = if dirty { "dirty" } else { "clean" };

        let tracking = match git::ahead_behind(path) {
            Some((ahead, behind)) => format!(" [ahead {ahead}, behind {behind}]"),
            None => String::new(),
        };

        println!("{branch_label:20} {state:6}{tracking}  {}", path.display());
    }

    Ok(())
}
