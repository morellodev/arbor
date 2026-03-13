use std::fs;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use crate::{config::Config, display, git};

pub fn run(config: &Config, all: bool) -> Result<()> {
    if all {
        list_all_repos(config)
    } else {
        list_repo(None)
    }
}

fn list_repo(cwd: Option<&Path>) -> Result<()> {
    let worktrees = git::worktree_infos(cwd)?;
    if worktrees.is_empty() {
        eprintln!("No worktrees found for this repository.");
        return Ok(());
    }

    let label = git::repo_name()
        .map(|name| format!("Repo {name}"))
        .unwrap_or_else(|_| "Current repository".to_string());

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary(&label, &summary));
    display::print_table(&worktrees, false);
    Ok(())
}

fn list_all_repos(config: &Config) -> Result<()> {
    let repos_dir = &config.repos_dir;
    if !repos_dir.exists() {
        eprintln!("No repos directory found at {}", repos_dir.display());
        return Ok(());
    }

    let mut found = false;
    for entry in fs::read_dir(repos_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let display_name = name.strip_suffix(".git").unwrap_or(&name);

            match git::worktree_infos(Some(&path)) {
                Ok(worktrees) if !worktrees.is_empty() => {
                    found = true;
                    println!("{}", format!("# {display_name}").bold());
                    let summary = display::summarize(&worktrees);
                    println!("{}", display::format_summary("Summary", &summary));
                    display::print_table(&worktrees, false);
                    println!();
                }
                Ok(_) => {}
                Err(_) => continue,
            }
        }
    }

    if !found {
        eprintln!("No repositories found in {}", repos_dir.display());
    }

    Ok(())
}
