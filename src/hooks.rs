use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Context;
use serde::Deserialize;

use crate::{config, display, git};

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    worktree_dir: Option<String>,
    #[serde(default)]
    hooks: Hooks,
}

#[derive(Debug, Default, Deserialize)]
struct Hooks {
    post_create: Option<HookCommands>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HookCommands {
    Single(String),
    Multiple(Vec<String>),
}

impl HookCommands {
    fn into_vec(self) -> Vec<String> {
        match self {
            HookCommands::Single(s) => vec![s],
            HookCommands::Multiple(v) => v,
        }
    }
}

pub struct HookContext {
    pub worktree_path: PathBuf,
    pub branch: String,
    pub repo_name: String,
    pub event: String,
}

fn load_project_config(worktree_path: &Path) -> anyhow::Result<Option<ProjectConfig>> {
    let config_path = worktree_path.join(".arbor.toml");
    if !config_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: ProjectConfig = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;
    Ok(Some(config))
}

#[cfg(unix)]
fn stderr_as_stdio() -> std::io::Result<Stdio> {
    use std::os::fd::AsFd;
    let owned = std::io::stderr().as_fd().try_clone_to_owned()?;
    Ok(owned.into())
}

#[cfg(windows)]
fn stderr_as_stdio() -> std::io::Result<Stdio> {
    use std::os::windows::io::AsHandle;
    let owned = std::io::stderr().as_handle().try_clone_to_owned()?;
    Ok(owned.into())
}

fn run_hook_command(cmd: &str, cwd: &Path, env_vars: &[(String, String)]) -> anyhow::Result<()> {
    let stdout_redirect = stderr_as_stdio()?;

    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    let mut command = std::process::Command::new(shell);
    command.args([flag, cmd]);
    command.current_dir(cwd);
    command.stdout(stdout_redirect);
    for (key, value) in env_vars {
        command.env(key, value);
    }

    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("Hook failed: {cmd} ({status})");
    }
    Ok(())
}

pub fn resolve_worktree_dir(raw: &str, repo_root: &Path) -> anyhow::Result<PathBuf> {
    let path = Path::new(raw);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else if raw.starts_with('~') {
        config::expand_tilde(path)
    } else {
        Ok(repo_root.join(raw))
    }
}

pub fn load_worktree_dir_from_path(dir: &Path) -> anyhow::Result<Option<String>> {
    let config = match load_project_config(dir)? {
        Some(c) => c,
        None => return Ok(None),
    };
    Ok(config.worktree_dir)
}

pub fn load_worktree_dir_from_git(cwd: &Path) -> anyhow::Result<Option<String>> {
    let raw = match git::show_file_from_head(".arbor.toml", cwd) {
        Ok(content) => content,
        Err(_) => return Ok(None),
    };
    let config: ProjectConfig =
        toml::from_str(&raw).with_context(|| "Failed to parse .arbor.toml from HEAD")?;
    Ok(config.worktree_dir)
}

pub fn run_post_create(ctx: &HookContext) {
    let config = match load_project_config(&ctx.worktree_path) {
        Ok(Some(config)) => config,
        Ok(None) => return,
        Err(e) => {
            display::print_error(&format!("Failed to load .arbor.toml: {e}"));
            return;
        }
    };

    let commands = match config.hooks.post_create {
        Some(cmds) => cmds.into_vec(),
        None => return,
    };

    let env_vars = vec![
        (
            "ARBOR_WORKTREE".to_string(),
            ctx.worktree_path.to_string_lossy().into_owned(),
        ),
        ("ARBOR_BRANCH".to_string(), ctx.branch.clone()),
        ("ARBOR_REPO".to_string(), ctx.repo_name.clone()),
        ("ARBOR_EVENT".to_string(), ctx.event.clone()),
    ];

    for cmd in &commands {
        display::print_note(&format!("Running hook: {cmd}"));
        if let Err(e) = run_hook_command(cmd, &ctx.worktree_path, &env_vars) {
            display::print_error(&format!("{e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_command() {
        let toml_str = r#"
[hooks]
post_create = "npm install"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        let cmds = config.hooks.post_create.unwrap().into_vec();
        assert_eq!(cmds, vec!["npm install"]);
    }

    #[test]
    fn parse_multiple_commands() {
        let toml_str = r#"
[hooks]
post_create = ["npm install", "cp .env.example .env"]
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        let cmds = config.hooks.post_create.unwrap().into_vec();
        assert_eq!(cmds, vec!["npm install", "cp .env.example .env"]);
    }

    #[test]
    fn parse_no_hooks_section() {
        let config: ProjectConfig = toml::from_str("").unwrap();
        assert!(config.hooks.post_create.is_none());
    }

    #[test]
    fn parse_empty_hooks_section() {
        let config: ProjectConfig = toml::from_str("[hooks]\n").unwrap();
        assert!(config.hooks.post_create.is_none());
    }

    #[test]
    fn parse_malformed_toml() {
        let result = toml::from_str::<ProjectConfig>("not valid { toml");
        assert!(result.is_err());
    }

    #[test]
    fn parse_worktree_dir_with_hooks() {
        let toml_str = r#"
worktree_dir = ".claude/worktrees"

[hooks]
post_create = "npm install"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.worktree_dir.as_deref(), Some(".claude/worktrees"));
        assert!(config.hooks.post_create.is_some());
    }

    #[test]
    fn parse_worktree_dir_without_hooks() {
        let toml_str = r#"
worktree_dir = ".claude/worktrees"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.worktree_dir.as_deref(), Some(".claude/worktrees"));
        assert!(config.hooks.post_create.is_none());
    }

    #[test]
    fn parse_no_worktree_dir() {
        let toml_str = r#"
[hooks]
post_create = "npm install"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert!(config.worktree_dir.is_none());
        assert!(config.hooks.post_create.is_some());
    }

    #[test]
    fn resolve_absolute_worktree_dir() {
        let result = resolve_worktree_dir("/tmp/worktrees", Path::new("/some/repo")).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/worktrees"));
    }

    #[test]
    fn resolve_tilde_worktree_dir() {
        let home = dirs::home_dir().unwrap();
        let result = resolve_worktree_dir("~/custom/wt", Path::new("/some/repo")).unwrap();
        assert_eq!(result, home.join("custom/wt"));
    }

    #[test]
    fn resolve_relative_worktree_dir() {
        let result = resolve_worktree_dir(".claude/worktrees", Path::new("/some/repo")).unwrap();
        assert_eq!(result, PathBuf::from("/some/repo/.claude/worktrees"));
    }

    #[test]
    fn parse_extra_fields_tolerated() {
        let toml_str = r#"
some_future_key = true

[hooks]
post_create = "npm install"
some_other_key = "value"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        let cmds = config.hooks.post_create.unwrap().into_vec();
        assert_eq!(cmds, vec!["npm install"]);
    }
}
