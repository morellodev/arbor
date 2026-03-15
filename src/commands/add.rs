use std::fs;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::{display, git, hooks};

pub fn run(config: &Config, branch: &str, repo: Option<&str>, no_hooks: bool) -> Result<()> {
    let (repo_name, repo_cwd) = resolve_repo(config, repo)?;
    let wt_path = config.worktree_path(&repo_name, branch);

    if wt_path.exists() {
        display::print_note(&format!(
            "Already exists at {}",
            display::shorten_path(&wt_path)
        ));
        display::print_path_hint(&wt_path);
        return Ok(());
    }

    fs::create_dir_all(wt_path.parent().unwrap())
        .with_context(|| format!("Failed to create directory: {}", wt_path.display()))?;

    let cwd = repo_cwd.as_deref();

    if git::local_branch_exists(branch, cwd)? {
        git::worktree_add_existing(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Linked '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    } else if git::remote_branch_exists(branch, cwd)? {
        git::create_tracking_branch(branch, cwd)?;
        git::worktree_add_existing(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Linked '{branch}' (tracking origin) at {}",
            display::shorten_path(&wt_path)
        ));
    } else {
        git::worktree_add_new_branch(&wt_path, branch, cwd)?;
        display::print_ok(&format!(
            "Created '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    }

    if !no_hooks {
        hooks::run_post_create(&hooks::HookContext {
            worktree_path: wt_path.clone(),
            branch: branch.to_string(),
            repo_name: repo_name.clone(),
            event: "post_create".to_string(),
        });
    }

    display::print_path_hint(&wt_path);
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
                bail!("Repository '{name}' not found at {}", repo_path.display());
            }
            Ok((name.to_string(), Some(repo_path)))
        }
        None => {
            let name = git::repo_name()?;
            Ok((name, None))
        }
    }
}
