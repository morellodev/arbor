use anyhow::{Result, bail};

use crate::{display, git};

fn parse_was_hash(output: &str) -> Option<&str> {
    let start = output.find("(was ")? + 5;
    let end = output[start..].find(')')? + start;
    Some(&output[start..end])
}

pub fn run(branch: &str, force: bool, delete_branch: bool) -> Result<()> {
    let (wt_path, actual_branch) = match git::resolve_worktree_branch(branch, None) {
        Ok(result) => result,
        Err(_) => {
            bail!("No worktree found for branch '{branch}'. Did you mean `arbor add {branch}`?")
        }
    };

    if !force && git::is_worktree_dirty(&wt_path) {
        bail!("Worktree has uncommitted changes. Use --force to remove anyway.");
    }

    let toplevel = std::env::current_dir()
        .ok()
        .filter(|cwd| display::cwd_is_inside(cwd, &wt_path))
        .map(|_| git::repo_toplevel())
        .transpose()?;

    git::worktree_remove(&wt_path, force)?;
    display::print_ok(&format!("Removed {}", display::shorten_path(&wt_path)));

    if delete_branch {
        match git::delete_branch(&actual_branch, force, None) {
            Ok(output) => {
                let hash = parse_was_hash(&output);
                let suffix = hash.map_or(String::new(), |h| format!(" (was {h})"));
                display::print_ok(&format!("Deleted branch '{actual_branch}'{suffix}"));
            }
            Err(e) => {
                display::print_error(&format!("Could not delete branch '{actual_branch}': {e}"))
            }
        }
    }

    if let Some(toplevel) = toplevel {
        println!("{}", toplevel.display());
    }

    Ok(())
}
