use std::fs;

use anyhow::Result;

use crate::config::Config;
use crate::git;

pub fn run(config: &Config, all: bool) -> Result<()> {
    if all {
        list_all_repos(config)
    } else {
        let output = git::worktree_list(None)?;
        println!("{output}");
        Ok(())
    }
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

            match git::worktree_list(Some(&path)) {
                Ok(output) => {
                    found = true;
                    eprintln!("# {display_name}");
                    println!("{output}");
                    println!();
                }
                Err(_) => continue,
            }
        }
    }

    if !found {
        eprintln!("No repositories found in {}", repos_dir.display());
    }

    Ok(())
}
