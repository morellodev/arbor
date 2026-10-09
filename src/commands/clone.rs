use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::{display, git, hooks};

pub fn run(config: &Config, url: &str, no_worktree: bool, no_hooks: bool) -> Result<()> {
    let url = expand_shorthand(url);
    let (url, name) = if is_local_path(&url) {
        let path = resolve_local_source(&url)?;
        let name = local_repo_name(&path)?;
        (path.to_string_lossy().into_owned(), name)
    } else {
        let name = repo_name_from_url(&url)?;
        (url, name)
    };
    let bare_name = format!("{name}.git");
    let dest = config.repos_dir.join(&bare_name);

    if dest.exists() {
        bail!("Repository already exists at {}", dest.display());
    }

    fs::create_dir_all(&config.repos_dir).with_context(|| {
        format!(
            "Failed to create repos directory: {}",
            config.repos_dir.display()
        )
    })?;

    display::print_note("Cloning bare repository...");
    git::clone_bare(&url, &dest)?;
    git::configure_bare_fetch(&dest)?;

    display::print_note("Fetching remote branches...");
    git::fetch_origin(&dest)?;

    let default_branch = git::head_branch(&dest).ok();
    if let Some(branch) = &default_branch {
        git::reset_bare_clone_branches(&dest, branch)?;
    }

    display::print_ok(&format!("Cloned to {}", display::shorten_path(&dest)));

    if !no_worktree && let Some(default_branch) = default_branch {
        let wt_path = match hooks::load_worktree_dir_from_git(&dest)? {
            Some(raw) => hooks::resolve_worktree_dir(&raw, &dest)?
                .join(git::sanitize_branch(&default_branch)),
            None => config.worktree_path(&name, &default_branch),
        };

        fs::create_dir_all(
            wt_path
                .parent()
                .context("Worktree path has no parent directory")?,
        )
        .with_context(|| format!("Failed to create directory: {}", wt_path.display()))?;

        git::worktree_add_existing(&wt_path, &default_branch, Some(&dest))?;
        display::print_ok(&format!(
            "Created '{}' at {}",
            default_branch,
            display::shorten_path(&wt_path)
        ));
        if !no_hooks {
            hooks::run_post_create(&hooks::HookContext {
                worktree_path: wt_path.clone(),
                branch: default_branch.clone(),
                repo_name: name.clone(),
            });
        }
        display::print_path_hint(&wt_path);
        return Ok(());
    }

    println!("{}", dest.display());
    display::print_heading("Next steps:");
    display::print_cd_hint(&dest);
    display::print_hint("arbor add <branch>  # create a worktree from the cloned repo");
    Ok(())
}

/// Expand a "user/repo" shorthand into a full GitHub HTTPS URL.
///
/// Strings that already look like full URLs (contain "://"), SSH addresses
/// (contain ":"), or explicit local paths are returned unchanged.
fn expand_shorthand(input: &str) -> String {
    if input.contains("://") || input.contains(':') || is_local_path(input) {
        return input.to_string();
    }

    let parts: Vec<&str> = input.splitn(3, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        return format!("https://github.com/{input}");
    }

    input.to_string()
}

// Only explicit prefixes count: a relative dir that happens to match `user/repo` in the
// cwd must not silently replace the GitHub clone.
fn is_local_path(input: &str) -> bool {
    input.starts_with('.') || input.starts_with('~') || Path::new(input).is_absolute()
}

fn resolve_local_source(input: &str) -> Result<PathBuf> {
    let expanded = crate::config::expand_tilde(Path::new(input))?;
    expanded
        .canonicalize()
        .with_context(|| format!("Repository not found at {}", expanded.display()))
}

/// Takes a canonical path, so `.` and `..` arrive resolved; `repo/.git` is named after `repo`.
fn local_repo_name(path: &Path) -> Result<String> {
    let dir = if path.file_name().is_some_and(|n| n == ".git") {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    let name = dir
        .file_name()
        .map(|n| git::strip_git_suffix(&n.to_string_lossy()).to_string())
        .filter(|n| !n.is_empty())
        .with_context(|| format!("Could not derive a repository name from {}", path.display()))?;
    Ok(name)
}

/// Extract the repository name from a git URL.
///
/// Handles patterns like:
///   https://github.com/user/repo.git → repo
///   git@github.com:user/repo.git    → repo
///   https://github.com/user/repo    → repo
fn repo_name_from_url(url: &str) -> Result<String> {
    let url = url.trim_end_matches('/');
    let segment = if url.contains('/') {
        url.rsplit('/').next()
    } else {
        url.rsplit(':').next()
    }
    .context("Could not extract repository name from URL")?;

    Ok(git::strip_git_suffix(segment).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_url_with_git_suffix() {
        let name = repo_name_from_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(name, "repo");
    }

    #[test]
    fn https_url_without_git_suffix() {
        let name = repo_name_from_url("https://github.com/user/repo").unwrap();
        assert_eq!(name, "repo");
    }

    #[test]
    fn https_url_with_trailing_slash() {
        let name = repo_name_from_url("https://github.com/user/repo/").unwrap();
        assert_eq!(name, "repo");
    }

    #[test]
    fn ssh_url() {
        let name = repo_name_from_url("git@github.com:user/my-project.git").unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn url_with_nested_path() {
        let name = repo_name_from_url("https://gitlab.com/org/group/subgroup/repo.git").unwrap();
        assert_eq!(name, "repo");
    }

    #[test]
    fn ssh_url_without_org() {
        let name = repo_name_from_url("git@github.com:repo.git").unwrap();
        assert_eq!(name, "repo");
    }

    #[test]
    fn empty_url_does_not_panic() {
        let name = repo_name_from_url("").unwrap();
        assert_eq!(name, "");
    }

    #[test]
    fn shorthand_expands_to_github_https() {
        assert_eq!(
            expand_shorthand("user/repo"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn shorthand_preserves_https_url() {
        let url = "https://github.com/user/repo.git";
        assert_eq!(expand_shorthand(url), url);
    }

    #[test]
    fn shorthand_preserves_ssh_url() {
        let url = "git@github.com:user/repo.git";
        assert_eq!(expand_shorthand(url), url);
    }

    #[test]
    fn shorthand_ignores_nested_path() {
        let input = "org/group/repo";
        assert_eq!(expand_shorthand(input), input);
    }

    #[test]
    fn shorthand_ignores_bare_name() {
        let input = "repo";
        assert_eq!(expand_shorthand(input), input);
    }

    #[test]
    fn shorthand_expands_even_when_a_matching_local_dir_exists() {
        let dir = tempfile::Builder::new()
            .prefix("arbor-user")
            .tempdir_in(".")
            .unwrap();
        let user = dir.path().file_name().unwrap().to_string_lossy();
        std::fs::create_dir(dir.path().join("repo")).unwrap();
        let input = format!("{user}/repo");
        assert_eq!(
            expand_shorthand(&input),
            format!("https://github.com/{input}")
        );
    }

    #[test]
    fn local_repo_name_uses_the_repository_directory() {
        assert_eq!(local_repo_name(Path::new("/code/app")).unwrap(), "app");
        assert_eq!(local_repo_name(Path::new("/code/app/.git")).unwrap(), "app");
        assert_eq!(local_repo_name(Path::new("/srv/app.git")).unwrap(), "app");
    }

    #[test]
    fn shorthand_ignores_relative_paths() {
        assert_eq!(expand_shorthand("../repo"), "../repo");
        assert_eq!(expand_shorthand("./repo"), "./repo");
    }
}
