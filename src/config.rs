use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const CONFIG_DIR_NAME: &str = ".arbor";
const CONFIG_FILE_NAME: &str = "config.toml";

const DEFAULT_CONFIG: &str = r#"worktree_dir = "~/.arbor/worktrees"
repos_dir = "~/.arbor/repos"
"#;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub worktree_dir: PathBuf,
    pub repos_dir: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = config_dir()?;
        let config_path = config_dir.join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("failed to create config directory: {}", config_dir.display()))?;
            fs::write(&config_path, DEFAULT_CONFIG)
                .with_context(|| format!("failed to write default config: {}", config_path.display()))?;
        }

        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read config: {}", config_path.display()))?;

        let mut config: Config =
            toml::from_str(&raw).with_context(|| "failed to parse config.toml")?;

        config.worktree_dir = expand_tilde(&config.worktree_dir)?;
        config.repos_dir = expand_tilde(&config.repos_dir)?;

        Ok(config)
    }
}

fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(CONFIG_DIR_NAME))
}

fn expand_tilde(path: &Path) -> Result<PathBuf> {
    let s = path.to_string_lossy();
    if s.starts_with('~') {
        let home = dirs::home_dir().context("could not determine home directory")?;
        Ok(home.join(s.strip_prefix("~/").unwrap_or(&s[1..])))
    } else {
        Ok(path.to_path_buf())
    }
}
