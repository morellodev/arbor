use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::Cli;
use crate::display;

enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            _ => bail!("Unsupported shell: {s} (supported: bash, zsh, fish)"),
        }
    }

    fn detect() -> Result<Self> {
        let shell_env = std::env::var("SHELL").ok().filter(|s| !s.is_empty());
        let Some(shell_env) = shell_env else {
            bail!("Could not detect shell. Specify it explicitly: arbor init bash|zsh|fish");
        };
        let name = shell_env.rsplit('/').next().unwrap_or("");
        Self::parse(name).map_err(|_| {
            anyhow::anyhow!(
                "Unsupported shell: {name}. Specify it explicitly: arbor init bash|zsh|fish"
            )
        })
    }
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
}

fn config_file_path(shell: &Shell) -> Result<PathBuf> {
    let home = std::env::home_dir().context("Could not determine home directory")?;
    Ok(match shell {
        Shell::Bash if cfg!(target_os = "macos") => bash_login_file(&home),
        Shell::Bash => home.join(".bashrc"),
        Shell::Zsh => env_dir("ZDOTDIR").unwrap_or(home).join(".zshrc"),
        Shell::Fish => env_dir("XDG_CONFIG_HOME")
            .unwrap_or_else(|| home.join(".config"))
            .join("fish/config.fish"),
    })
}

/// macOS terminals start login shells, which skip ~/.bashrc and read only the first
/// of these that exists; creating ~/.bash_profile would hide an existing ~/.profile.
fn bash_login_file(home: &Path) -> PathBuf {
    [".bash_profile", ".bash_login", ".profile"]
        .into_iter()
        .map(|name| home.join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| home.join(".bash_profile"))
}

fn read_config(path: &Path) -> Result<Vec<u8>> {
    match fs::read(path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).context(format!("Failed to read {}", path.display())),
    }
}

fn eval_line(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash => r#"eval "$(arbor init bash)""#,
        Shell::Zsh => r#"eval "$(arbor init zsh)""#,
        Shell::Fish => "arbor init fish | source",
    }
}

fn already_configured(path: &Path) -> Result<bool> {
    let content = read_config(path)?;
    // The shell argument is optional (`eval "$(arbor init)"`), and the file is
    // already specific to this shell, so any line loading `arbor init` counts.
    Ok(String::from_utf8_lossy(&content).lines().any(|line| {
        !line.trim_start().starts_with('#')
            && line.contains("arbor init")
            && (line.contains("eval") || line.contains("source"))
    }))
}

fn inject_into_config(path: &Path, line: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let mut content = read_config(path)?;
    if !content.is_empty() && !content.ends_with(b"\n") {
        content.push(b'\n');
    }
    content.extend_from_slice(format!("# arbor\n{line}\n").as_bytes());

    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn print_restart_hint(config_path: &Path) {
    display::print_note("To activate now, run:");
    display::print_hint(&format!("source {}", display::shorten_path(config_path)));
}

pub fn run(shell: Option<&str>, inject: bool) -> Result<()> {
    let shell = match shell {
        Some(s) => Shell::parse(s)?,
        None => Shell::detect()?,
    };

    if !inject && !std::io::stdout().is_terminal() {
        return print_script(&shell);
    }

    let config_path = config_file_path(&shell)?;
    let line = eval_line(&shell);
    let short_path = display::shorten_path(&config_path);

    if already_configured(&config_path)? {
        display::print_ok("Shell integration is already configured");
        display::print_hint(&format!("Found in {short_path}"));
        return Ok(());
    }

    let should_inject = if inject {
        true
    } else {
        display::print_note(&format!("This will add the following to {short_path}:"));
        eprintln!();
        display::print_hint("# arbor");
        display::print_hint(line);
        eprintln!();

        dialoguer::Confirm::new()
            .with_prompt("Add it now?")
            .default(true)
            .interact_opt()?
            == Some(true)
    };

    if should_inject {
        inject_into_config(&config_path, line)?;
        display::print_ok(&format!("Added shell integration to {short_path}"));
        print_restart_hint(&config_path);
    } else {
        display::print_note(
            "No changes made. To set up manually, add the lines above to your shell config",
        );
    }

    Ok(())
}

fn generate_completions(shell: clap_complete::Shell) -> String {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    generate(shell, &mut cmd, "arbor", &mut buf);
    String::from_utf8(buf).expect("clap_complete should produce valid UTF-8")
}

fn print_script(shell: &Shell) -> Result<()> {
    match shell {
        Shell::Bash => {
            println!("{SHELL_WRAPPER}");
            print!("{}", generate_completions(clap_complete::Shell::Bash));
            print!("{BASH_BRANCH_COMPLETIONS}");
        }
        Shell::Zsh => {
            println!("{SHELL_WRAPPER}");
            print!("{ZSH_COMPINIT}");
            print!("{}", generate_completions(clap_complete::Shell::Zsh));
            print!("{ZSH_BRANCH_COMPLETIONS}");
        }
        Shell::Fish => {
            println!("{FISH_WRAPPER}");
            print!("{}", generate_completions(clap_complete::Shell::Fish));
            print!("{FISH_BRANCH_COMPLETIONS}");
        }
    }
    Ok(())
}

const SHELL_WRAPPER: &str = r#"arbor() {
  local arg subcommand skip_value=0
  for arg in "$@"; do
    if [ "$skip_value" = 1 ]; then skip_value=0; continue; fi
    case "$arg" in
      --color) skip_value=1 ;;
      -*) ;;
      *) subcommand="$arg"; break ;;
    esac
  done
  case "$subcommand" in
    add|switch|cd|clone|remove|rm|clean)
      # On failure, a path on stdout still means the cwd was deleted.
      local dir rc
      dir=$(command arbor "$@")
      rc=$?
      if [ -n "$dir" ] && [ -d "$dir" ]; then cd "$dir" || return; fi
      return $rc
      ;;
    *)
      command arbor "$@"
      ;;
  esac
}"#;

const FISH_WRAPPER: &str = r#"function arbor --wraps arbor
  set -l subcommand
  set -l skip_value 0
  for arg in $argv
    if test $skip_value = 1
      set skip_value 0
      continue
    end
    switch $arg
      case --color
        set skip_value 1
      case '-*'
      case '*'
        set subcommand $arg
        break
    end
  end
  switch "$subcommand"
    case add switch cd clone remove rm clean
      set -l dir (command arbor $argv)
      set -l rc $status
      if test -n "$dir"; and test -d "$dir"
        cd $dir
      end
      return $rc
    case '*'
      command arbor $argv
  end
end"#;

// A fresh zsh (no ~/.zshrc on macOS) has no compdef, so registering completions fails.
const ZSH_COMPINIT: &str = r#"
if (( ! $+functions[compdef] )); then
  autoload -Uz compinit && compinit -i
fi
"#;

const BASH_BRANCH_COMPLETIONS: &str = r#"
_arbor_branches() {
  _arbor "$@"
  [[ "${COMP_WORDS[COMP_CWORD]}" == -* ]] && return
  local i subcommand skip_value=0
  for ((i = 1; i < COMP_CWORD; i++)); do
    if [ "$skip_value" = 1 ]; then
      # bash splits --color=never into "--color" "=" "never".
      [ "${COMP_WORDS[i]}" = "=" ] || skip_value=0
      continue
    fi
    case "${COMP_WORDS[i]}" in
      --color) skip_value=1 ;;
      -*) ;;
      *) subcommand="${COMP_WORDS[i]}"; break ;;
    esac
  done
  case "$subcommand" in
    add)
      local branches
      branches=$(git for-each-ref --format='%(refname)' refs/heads/ refs/remotes/ 2>/dev/null | sed -E 's#^refs/(heads|remotes/[^/]+)/##' | grep -vx HEAD | sort -u)
      COMPREPLY=($(compgen -W "$branches" -- "${COMP_WORDS[COMP_CWORD]}"))
      ;;
    switch|cd|rm|remove|dir)
      local branches
      branches=$(git worktree list --porcelain 2>/dev/null | grep '^branch ' | sed 's|^branch refs/heads/||')
      COMPREPLY=($(compgen -W "$branches" -- "${COMP_WORDS[COMP_CWORD]}"))
      ;;
  esac
}
# Same options clap registers for _arbor, so paths still complete as a fallback.
if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
  complete -F _arbor_branches -o nosort -o bashdefault -o default arbor
else
  complete -F _arbor_branches -o bashdefault -o default arbor
fi
"#;

const ZSH_BRANCH_COMPLETIONS: &str = r#"
_arbor_branches() {
  _arbor "$@"
  local i subcommand skip_value=0
  for ((i = 2; i < CURRENT; i++)); do
    if (( skip_value )); then skip_value=0; continue; fi
    case "${words[i]}" in
      --color) skip_value=1 ;;
      -*) ;;
      *) subcommand="${words[i]}"; break ;;
    esac
  done
  case "$subcommand" in
    add)
      local -a branches=($(git for-each-ref --format='%(refname)' refs/heads/ refs/remotes/ 2>/dev/null | sed -E 's#^refs/(heads|remotes/[^/]+)/##' | grep -vx HEAD | sort -u))
      _describe 'branch' branches
      ;;
    switch|cd|rm|remove|dir)
      local -a branches=($(git worktree list --porcelain 2>/dev/null | grep '^branch ' | sed 's|^branch refs/heads/||'))
      _describe 'branch' branches
      ;;
  esac
}
compdef _arbor_branches arbor
"#;

const FISH_BRANCH_COMPLETIONS: &str = r#"
complete -c arbor -n '__fish_seen_subcommand_from add' -f -a '(git for-each-ref --format="%(refname)" refs/heads/ refs/remotes/ 2>/dev/null | string replace -r "^refs/(heads|remotes/[^/]+)/" "" | string match -v HEAD | sort -u)'

complete -c arbor -n '__fish_seen_subcommand_from switch cd rm remove dir' -f -a '(git worktree list --porcelain 2>/dev/null | string match -r "^branch refs/heads/(.*)" | string replace -r "^branch refs/heads/" "")'
"#;
