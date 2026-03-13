use std::fs;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::{display, git};

pub fn run(config: &Config, branch: &str, repo: Option<&str>) -> Result<()> {
    let (repo_name, repo_cwd) = resolve_repo(config, repo)?;
    let wt_path = config.worktree_path(&repo_name, branch);

    if wt_path.exists() {
        display::print_note(&format!(
            "Worktree already exists at {}",
            display::shorten_path(&wt_path)
        ));
        print_path_hint(&wt_path);
        return Ok(());
    }

    fs::create_dir_all(wt_path.parent().unwrap())
        .with_context(|| format!("failed to create directory: {}", wt_path.display()))?;

    let cwd = repo_cwd.as_deref();

    if git::local_branch_exists(branch, cwd)? {
        git::worktree_add_existing(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Worktree created for existing branch '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    } else if git::remote_branch_exists(branch, cwd)? {
        git::create_tracking_branch(branch, cwd)?;
        git::worktree_add_existing(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Worktree created for remote branch '{branch}' (tracking origin) at {}",
            display::shorten_path(&wt_path)
        ));
    } else {
        git::worktree_add_new_branch(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "New branch '{branch}' created with worktree at {}",
            display::shorten_path(&wt_path)
        ));
    }

    print_path_hint(&wt_path);
    Ok(())
}

fn resolve_repo(
    config: &Config,
    repo: Option<&str>,
) -> Result<(String, Option<std::path::PathBuf>)> {
    match repo {
        Some(name) => {
            let bare_name = format!("{name}.git");
            let repo_path = config.repos_dir.join(&bare_name);
            if !repo_path.exists() {
                bail!("repository '{name}' not found at {}", repo_path.display());
            }
            Ok((name.to_string(), Some(repo_path)))
        }
        None => {
            let name = git::repo_name()?;
            Ok((name, None))
        }
    }
}

fn print_path_hint(path: &Path) {
    if std::io::stdout().is_terminal() {
        eprintln!("To switch to it, run:");
        display::print_cd_hint(path);
    } else {
        println!("{}", path.display());
    }
}
