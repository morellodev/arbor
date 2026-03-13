use std::fs;
use std::io::IsTerminal;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::{display, git};

pub fn run(config: &Config, branch: &str) -> Result<()> {
    let repo_name = git::repo_name()?;
    let sanitized = branch.replace('/', "-");
    let wt_path = config.worktree_dir.join(&repo_name).join(&sanitized);

    if wt_path.exists() {
        display::print_note(&format!(
            "Worktree already exists at {}",
            wt_path.display()
        ));
        print_path_hint(&wt_path);
        return Ok(());
    }

    fs::create_dir_all(wt_path.parent().unwrap())
        .with_context(|| format!("failed to create directory: {}", wt_path.display()))?;

    if git::local_branch_exists(branch)? {
        git::worktree_add_existing(&wt_path, branch)?;
    } else if git::remote_branch_exists(branch)? {
        git::create_tracking_branch(branch)?;
        git::worktree_add_existing(&wt_path, branch)?;
    } else {
        git::worktree_add_new_branch(&wt_path, branch)?;
    }

    display::print_ok(&format!("Worktree created at {}", wt_path.display()));
    print_path_hint(&wt_path);
    Ok(())
}

fn print_path_hint(path: &std::path::Path) {
    if std::io::stdout().is_terminal() {
        eprintln!("To switch to it, run:");
        display::print_cd_hint(path);
    } else {
        println!("{}", path.display());
    }
}
