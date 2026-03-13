use std::fs;

use anyhow::{bail, Context, Result};

use crate::config::Config;
use crate::git;

pub fn run(config: &Config, url: &str) -> Result<()> {
    let name = repo_name_from_url(url)?;
    let bare_name = format!("{name}.git");
    let dest = config.repos_dir.join(&bare_name);

    if dest.exists() {
        bail!("repository already exists at {}", dest.display());
    }

    fs::create_dir_all(&config.repos_dir)
        .with_context(|| format!("failed to create repos directory: {}", config.repos_dir.display()))?;

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
    let name = url
        .rsplit('/')
        .next()
        .or_else(|| url.rsplit(':').next())
        .context("could not extract repository name from URL")?;

    Ok(name.strip_suffix(".git").unwrap_or(name).to_string())
}
