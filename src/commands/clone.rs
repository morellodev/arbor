use std::fs;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::{display, git};

pub fn run(config: &Config, url: &str, no_worktree: bool) -> Result<()> {
    let url = expand_shorthand(url);
    let name = repo_name_from_url(&url)?;
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

    display::print_ok(&format!("Cloned to {}", display::shorten_path(&dest)));

    if !no_worktree && let Ok(default_branch) = git::head_branch(&dest) {
        let wt_path = config.worktree_path(&name, &default_branch);

        fs::create_dir_all(wt_path.parent().unwrap())
            .with_context(|| format!("Failed to create directory: {}", wt_path.display()))?;

        git::worktree_add_existing(&wt_path, &default_branch, Some(&dest))?;
        display::print_ok(&format!(
            "Created '{}' at {}",
            default_branch,
            display::shorten_path(&wt_path)
        ));
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
/// Strings that already look like full URLs (contain "://") or SSH addresses
/// (contain ":") are returned unchanged.
fn expand_shorthand(input: &str) -> String {
    if input.contains("://") || input.contains(':') {
        return input.to_string();
    }

    let parts: Vec<&str> = input.splitn(3, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        return format!("https://github.com/{input}");
    }

    input.to_string()
}

/// Extract the repository name from a git URL.
///
/// Handles patterns like:
///   https://github.com/user/repo.git → repo
///   git@github.com:user/repo.git    → repo
///   https://github.com/user/repo    → repo
fn repo_name_from_url(url: &str) -> Result<String> {
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
}
