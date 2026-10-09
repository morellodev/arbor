use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::{display, git, hooks};

pub fn run(config: &Config, branch: &str, base: Option<&str>, no_hooks: bool) -> Result<()> {
    let repo_root = git::repo_toplevel()?;
    git::ensure_valid_branch_name(branch)?;
    let (branch, named_remote) = match git::split_remote_branch(branch, None)? {
        Some((remote, name)) if !git::local_branch_exists(branch, None)? => {
            let upstream = format!("{remote}/{name}");
            if git::local_branch_exists(&name, None)?
                && git::branch_upstream(&name, None).as_deref() != Some(upstream.as_str())
            {
                bail!(
                    "Local branch '{name}' already exists and doesn't track {upstream}. \
                     Use `arbor add {name}` to check out the local branch"
                );
            }
            display::print_note(&format!("Using branch '{name}' from '{remote}'"));
            (name, Some(remote))
        }
        _ => (branch.to_string(), None),
    };
    let branch = branch.as_str();

    let repo_name = git::strip_git_suffix(
        &repo_root
            .file_name()
            .context("Repository path has no final component")?
            .to_string_lossy(),
    )
    .to_string();
    let wt_path = resolve_wt_path(config, &repo_name, branch, &repo_root)?;

    // Paired with the effective branch, so a worktree mid-rebase of `branch` counts as its own.
    let worktrees: Vec<_> = git::parse_worktree_list(&git::worktree_list_porcelain(None)?)
        .into_iter()
        .filter(|wt| !wt.bare)
        .map(|wt| {
            let branch = git::effective_branch(&wt);
            (wt.path, branch)
        })
        .collect();
    let canonical_wt_path = fs::canonicalize(&wt_path).ok();
    let at_wt_path = worktrees.iter().find(|(path, _)| {
        canonical_wt_path.is_some() && fs::canonicalize(path).ok() == canonical_wt_path
    });
    let with_branch = worktrees
        .iter()
        .find(|(path, b)| b.as_deref() == Some(branch) && path.exists());

    // A detached worktree at the expected path is still ours.
    let existing = match (with_branch, at_wt_path) {
        (Some((path, _)), Some((at, _))) if path == at => Some(wt_path.clone()),
        (Some((path, _)), _) => Some(path.clone()),
        (None, Some((_, at_branch))) => match at_branch {
            None => Some(wt_path.clone()),
            Some(other) => bail!(
                "{} is already the worktree for '{other}'",
                display::shorten_path(&wt_path)
            ),
        },
        (None, None) if wt_path.exists() => {
            return Err(not_our_worktree(&wt_path, &repo_root, &repo_name));
        }
        (None, None) => None,
    };

    if let Some(existing) = existing {
        display::print_note(&format!(
            "Already exists at {}",
            display::shorten_path(&existing)
        ));
        display::print_path_hint(&existing);
        return Ok(());
    }

    fs::create_dir_all(
        wt_path
            .parent()
            .context("Worktree path has no parent directory")?,
    )
    .with_context(|| format!("Failed to create directory: {}", wt_path.display()))?;

    if git::local_branch_exists(branch, None)? {
        if base.is_some() {
            display::print_note("--base ignored — branch already exists locally");
        }
        git::worktree_add_existing(&wt_path, branch, None)?;
        display::print_ok(&format!(
            "Linked '{branch}' at {}",
            display::shorten_path(&wt_path)
        ));
    } else if let Some(remote) = pick_remote(branch, named_remote)? {
        if base.is_some() {
            display::print_note("--base ignored — tracking existing remote branch");
        }
        git::create_tracking_branch(branch, &remote, None)?;
        git::worktree_add_existing(&wt_path, branch, None)?;
        display::print_ok(&format!(
            "Linked '{branch}' (tracking {remote}) at {}",
            display::shorten_path(&wt_path)
        ));
    } else {
        display::print_note(&format!(
            "No existing branch found — creating new branch '{branch}'"
        ));
        git::worktree_add_new_branch(&wt_path, branch, base, None)?;
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
        });
    }

    display::print_path_hint(&wt_path);
    Ok(())
}

fn pick_remote(branch: &str, named: Option<String>) -> Result<Option<String>> {
    if named.is_some() {
        return Ok(named);
    }
    let remotes = git::remotes_with_branch(branch, None)?;
    match remotes.as_slice() {
        [] => Ok(None),
        [first, ..] if first == "origin" => Ok(Some(first.clone())),
        [only] => Ok(Some(only.clone())),
        _ => bail!(
            "Branch '{branch}' exists on several remotes ({}). Pick one, e.g. `arbor add {}/{branch}`",
            remotes.join(", "),
            remotes[0]
        ),
    }
}

/// Worktree paths are keyed by repo name only, so two repos with the same name collide.
fn not_our_worktree(wt_path: &Path, repo_root: &Path, repo_name: &str) -> anyhow::Error {
    let short = display::shorten_path(wt_path);
    let ours = git::common_dir(repo_root);
    match git::common_dir(wt_path) {
        Ok(theirs) if ours.is_ok_and(|ours| ours != theirs) => anyhow::anyhow!(
            "{short} is a worktree of another repository named '{repo_name}' ({}). \
             Set worktree_dir in this repo's .arbor.toml to keep their worktrees apart",
            display::shorten_path(&theirs)
        ),
        _ => anyhow::anyhow!("{short} already exists and is not a worktree"),
    }
}

fn resolve_wt_path(
    config: &Config,
    repo_name: &str,
    branch: &str,
    repo_root: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let bare = git::is_bare_repository(repo_root);
    let raw_override = if bare {
        hooks::load_worktree_dir_from_git(repo_root)?
    } else {
        hooks::load_worktree_dir_from_path(repo_root)?
    };
    let local_override =
        raw_override.and_then(|raw| hooks::repo_worktree_dir(&raw, repo_root, bare));

    match local_override {
        Some(dir) => Ok(dir.join(git::sanitize_branch(branch))),
        None => Ok(config.worktree_path(repo_name, branch)),
    }
}
