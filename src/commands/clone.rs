use std::fs;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::git;

pub fn run(config: &Config, url: &str) -> Result<()> {
    let name = repo_name_from_url(url)?;
    let bare_name = format!("{name}.git");
    let dest = config.repos_dir.join(&bare_name);

    if dest.exists() {
        bail!("repository already exists at {}", dest.display());
    }

    fs::create_dir_all(&config.repos_dir).with_context(|| {
        format!(
            "failed to create repos directory: {}",
            config.repos_dir.display()
        )
    })?;

    git::clone_bare(url, &dest)?;
    git::configure_bare_fetch(&dest)?;
    git::fetch_origin(&dest)?;

    eprintln!("Bare repo ready at {}", dest.display());
    println!("{}", dest.display());
    Ok(())
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
    .context("could not extract repository name from URL")?;

    Ok(segment.strip_suffix(".git").unwrap_or(segment).to_string())
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
}
