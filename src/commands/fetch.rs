use anyhow::Result;

use crate::config::Config;
use crate::{display, git};

use super::list::scan_repos;

pub fn run(config: &Config, all: bool) -> Result<()> {
    if all {
        return run_all(config);
    }

    let toplevel = git::repo_toplevel()?;
    display::print_note("Fetching from origin...");
    git::fetch_origin(&toplevel)?;
    display::print_ok("Fetch complete");
    Ok(())
}

fn run_all(config: &Config) -> Result<()> {
    let repos = scan_repos(config)?;

    if repos.is_empty() {
        display::print_note(&format!(
            "No repositories found in {}",
            config.repos_dir.display()
        ));
        return Ok(());
    }

    let mut success = 0;
    let mut failed = 0;

    for repo in &repos {
        display::print_section(&repo.display_name);
        display::print_note("Fetching from origin...");
        match git::fetch_origin(&repo.path) {
            Ok(()) => {
                display::print_ok("Fetch complete");
                success += 1;
            }
            Err(e) => {
                display::print_error(&format!("Fetch failed: {e}"));
                failed += 1;
            }
        }
        eprintln!();
    }

    display::print_fetch_summary(success, failed);

    Ok(())
}
