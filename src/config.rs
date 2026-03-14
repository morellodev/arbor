use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const CONFIG_DIR_NAME: &str = ".arbor";
const CONFIG_FILE_NAME: &str = "config.toml";

const DEFAULT_CONFIG: &str = r#"repos_dir = "~/.arbor/repos"
worktree_dir = "~/.arbor/worktrees"
"#;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub repos_dir: PathBuf,
    pub worktree_dir: PathBuf,
}

impl Config {
    pub fn worktree_path(&self, repo_name: &str, branch: &str) -> PathBuf {
        self.worktree_dir
            .join(repo_name)
            .join(crate::git::sanitize_branch(branch))
    }

    pub fn load() -> Result<Self> {
        let config_dir = config_dir()?;
        let config_path = config_dir.join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            fs::create_dir_all(&config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
            fs::write(&config_path, DEFAULT_CONFIG).with_context(|| {
                format!("Failed to write default config: {}", config_path.display())
            })?;
        }

        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?;

        let mut config: Config =
            toml::from_str(&raw).with_context(|| "Failed to parse config.toml")?;

        config.worktree_dir = expand_tilde(&config.worktree_dir)?;
        config.repos_dir = expand_tilde(&config.repos_dir)?;

        Ok(config)
    }
}

fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(CONFIG_DIR_NAME))
}

fn expand_tilde(path: &Path) -> Result<PathBuf> {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix('~') {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(stripped.strip_prefix('/').unwrap_or(stripped)))
    } else {
        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_with_slash() {
        let home = dirs::home_dir().unwrap();
        let result = expand_tilde(Path::new("~/.arbor/worktrees")).unwrap();
        assert_eq!(result, home.join(".arbor/worktrees"));
    }

    #[test]
    fn expand_tilde_bare() {
        let home = dirs::home_dir().unwrap();
        let result = expand_tilde(Path::new("~")).unwrap();
        assert_eq!(result, home);
    }

    #[test]
    fn expand_tilde_absolute_path_unchanged() {
        let result = expand_tilde(Path::new("/tmp/worktrees")).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/worktrees"));
    }

    #[test]
    fn default_config_parses() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.worktree_dir, PathBuf::from("~/.arbor/worktrees"));
        assert_eq!(config.repos_dir, PathBuf::from("~/.arbor/repos"));
    }

    #[test]
    fn expand_tilde_relative_path_unchanged() {
        let result = expand_tilde(Path::new("some/relative/path")).unwrap();
        assert_eq!(result, PathBuf::from("some/relative/path"));
    }

    #[test]
    fn invalid_toml_fails() {
        let result = toml::from_str::<Config>("not valid { toml");
        assert!(result.is_err());
    }

    #[test]
    fn missing_field_fails() {
        let result = toml::from_str::<Config>(r#"worktree_dir = "/tmp/wt""#);
        assert!(result.is_err());
    }

    #[test]
    fn extra_fields_tolerated() {
        let input = r#"
worktree_dir = "/tmp/worktrees"
repos_dir = "/tmp/repos"
some_future_key = true
"#;
        let config: Config = toml::from_str(input).unwrap();
        assert_eq!(config.worktree_dir, PathBuf::from("/tmp/worktrees"));
        assert_eq!(config.repos_dir, PathBuf::from("/tmp/repos"));
    }
}
