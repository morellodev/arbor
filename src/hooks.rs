use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

use anyhow::Context;
use serde::Deserialize;

use crate::{display, git};

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
}

impl HookContext {
    fn env_vars(&self) -> [(&'static str, String); 4] {
        [
            (
                "ARBOR_WORKTREE",
                self.worktree_path.to_string_lossy().into_owned(),
            ),
            ("ARBOR_BRANCH", self.branch.clone()),
            ("ARBOR_REPO", self.repo_name.clone()),
            ("ARBOR_EVENT", "post_create".to_string()),
        ]
    }
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

fn run_hook_command(cmd: &str, cwd: &Path, env_vars: &[(&str, String)]) -> anyhow::Result<()> {
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
        anyhow::bail!("Hook failed: {} ({status})", one_line(cmd));
    }
    Ok(())
}

/// The worktree root a repo's own `.arbor.toml` asks for. Only a relative path inside the
/// working tree is used: anything else could place repo content where it gets executed.
pub fn repo_worktree_dir(raw: &str, repo_root: &Path, bare: bool) -> Option<PathBuf> {
    let path = Path::new(raw);
    let stays_inside = !raw.starts_with('~')
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    let enters_git_dir = path
        .components()
        .find_map(|c| match c {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .is_some_and(|first| first.eq_ignore_ascii_case(".git"));

    if !stays_inside || enters_git_dir {
        display::print_note(&format!(
            "Ignored worktree_dir {raw} from .arbor.toml: only relative paths inside the \
             repo are used. Set worktree_dir in ~/.arbor/config.toml for other locations"
        ));
        return None;
    }
    // A bare repo has no working tree; anything inside it is the git dir itself.
    if bare {
        return None;
    }
    Some(repo_root.join(path))
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

fn load_post_create_commands(worktree_path: &Path) -> anyhow::Result<Vec<String>> {
    Ok(load_project_config(worktree_path)?
        .and_then(|config| config.hooks.post_create)
        .map(HookCommands::into_vec)
        .unwrap_or_default())
}

// A newline inside a hook command could print a line that passes for arbor's own output.
fn one_line(cmd: &str) -> String {
    cmd.replace('\n', "\\n")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

pub fn note_skipped_post_create(ctx: &HookContext) {
    let commands = match load_post_create_commands(&ctx.worktree_path) {
        Ok(commands) if !commands.is_empty() => commands,
        Ok(_) => return,
        Err(e) => {
            display::print_error(&format!("Failed to load .arbor.toml: {e}"));
            return;
        }
    };

    let exports: Vec<String> = ctx
        .env_vars()
        .iter()
        .map(|(key, value)| format!("{key}={}", shell_quote(value)))
        .collect();
    display::print_note(
        "Skipped post_create hooks from the cloned repo (pass --hooks to run them). \
         To run them yourself after reviewing them, from the new worktree:",
    );
    display::print_hint(&format!("export {}", exports.join(" ")));
    for cmd in &commands {
        display::print_hint(&one_line(cmd));
    }
}

pub fn run_post_create(ctx: &HookContext) {
    let commands = match load_post_create_commands(&ctx.worktree_path) {
        Ok(commands) => commands,
        Err(e) => {
            display::print_error(&format!("Failed to load .arbor.toml: {e}"));
            return;
        }
    };

    let env_vars = ctx.env_vars();
    for cmd in &commands {
        display::print_note(&format!("Running hook: {}", one_line(cmd)));
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
    fn repo_worktree_dir_accepts_paths_inside_the_working_tree() {
        let root = Path::new("/some/repo");
        assert_eq!(
            repo_worktree_dir(".claude/worktrees", root, false),
            Some(root.join(".claude/worktrees"))
        );
        assert_eq!(
            repo_worktree_dir("./wt", root, false),
            Some(root.join("./wt"))
        );
    }

    #[test]
    fn repo_worktree_dir_rejects_paths_that_leave_the_working_tree() {
        let root = Path::new("/some/repo");
        for raw in [
            "~",
            "~/bin",
            "../outside",
            "wt/../../outside",
            ".git",
            ".GIT/hooks",
        ] {
            assert_eq!(repo_worktree_dir(raw, root, false), None, "{raw}");
        }
        let absolute = std::env::temp_dir().join("wt");
        assert_eq!(
            repo_worktree_dir(&absolute.to_string_lossy(), root, false),
            None
        );
    }

    #[test]
    fn repo_worktree_dir_is_never_used_in_a_bare_repo() {
        assert_eq!(repo_worktree_dir(".", Path::new("/r.git"), true), None);
        assert_eq!(repo_worktree_dir("wt", Path::new("/r.git"), true), None);
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
