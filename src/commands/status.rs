use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::{display, git};

use super::list::scan_repos;

pub fn run(config: &Config, short: bool, all: bool) -> Result<()> {
    if all {
        return run_all(config, short);
    }

    let worktrees = git::worktree_infos(None)?;
    if worktrees.is_empty() {
        display::print_note("No worktrees found.");
        return Ok(());
    }

    let label = git::repo_name_or_unknown();

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary(&label, &summary));

    display::print_table(&worktrees, !short);
    Ok(())
}

fn run_all(config: &Config, short: bool) -> Result<()> {
    let repos = scan_repos(config)?;

    if repos.is_empty() {
        display::print_note(&format!(
            "No repositories found in {}",
            config.repos_dir.display()
        ));
        return Ok(());
    }

    for repo in &repos {
        println!("{}", format!("# {}", repo.display_name).bold());
        let summary = display::summarize(&repo.worktrees);
        println!("{}", display::format_summary("Summary", &summary));
        display::print_table(&repo.worktrees, !short);
        println!();
    }

    Ok(())
}
