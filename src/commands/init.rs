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

// Login files often source ~/.bashrc first; the guard keeps the integration from loading twice.
const BASH_LOGIN_LINE: &str = r#"declare -F arbor >/dev/null || eval "$(arbor init bash)""#;

/// The startup files that must load the integration, with the line each one gets.
/// On macOS, terminals start login shells, which skip ~/.bashrc, while `bash`
/// started from another shell reads only ~/.bashrc.
fn config_files(shell: &Shell) -> Result<Vec<(PathBuf, &'static str)>> {
    let home = std::env::home_dir().context("Could not determine home directory")?;
    let line = eval_line(shell);
    let mut files = match shell {
        Shell::Bash if cfg!(target_os = "macos") => vec![
            (home.join(".bashrc"), line),
            (bash_login_file(&home), BASH_LOGIN_LINE),
        ],
        Shell::Bash => vec![(home.join(".bashrc"), line)],
        Shell::Zsh => vec![(env_dir("ZDOTDIR").unwrap_or(home).join(".zshrc"), line)],
        Shell::Fish => vec![(
            env_dir("XDG_CONFIG_HOME")
                .unwrap_or_else(|| home.join(".config"))
                .join("fish/config.fish"),
            line,
        )],
    };
    // A login file symlinked to ~/.bashrc is the same file.
    let mut seen = Vec::new();
    files.retain(|(path, _)| {
        let resolved = resolve_symlink(path);
        let first = !seen.contains(&resolved);
        seen.push(resolved);
        first
    });
    Ok(files)
}

fn resolve_symlink(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    // A dangling link's target doesn't exist yet, so canonicalize fails.
    match (fs::read_link(path), path.parent()) {
        (Ok(target), Some(parent)) => parent.join(target),
        _ => path.to_path_buf(),
    }
}

/// Login shells read only the first of these that exists; creating ~/.bash_profile
/// would hide an existing ~/.profile.
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
        let line = line.trim_start();
        !line.starts_with('#')
            && line.contains("arbor init")
            && (line.contains("eval") || line.contains("source") || line.starts_with(". "))
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

    let mut targets = Vec::new();
    let mut found = Vec::new();
    for (path, line) in config_files(&shell)? {
        if already_configured(&path)? {
            found.push(display::shorten_path(&path));
        } else {
            targets.push((path, line));
        }
    }

    if targets.is_empty() {
        display::print_ok("Shell integration is already configured");
        display::print_hint(&format!("Found in {}", found.join(", ")));
        return Ok(());
    }
    let short_path = targets
        .iter()
        .map(|(path, _)| display::shorten_path(path))
        .collect::<Vec<_>>()
        .join(" and ");

    let should_inject = if inject {
        true
    } else {
        for (path, line) in &targets {
            display::print_note(&format!(
                "This will add the following to {}:",
                display::shorten_path(path)
            ));
            eprintln!();
            display::print_hint("# arbor");
            display::print_hint(line);
            eprintln!();
        }

        dialoguer::Confirm::new()
            .with_prompt("Add it now?")
            .default(true)
            .interact_opt()?
            == Some(true)
    };

    if should_inject {
        for (path, line) in &targets {
            inject_into_config(path, line)?;
        }
        display::print_ok(&format!("Added shell integration to {short_path}"));
        print_restart_hint(&targets[0].0);
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

// Without compdef yet, registrations are queued and replayed at the first prompt, so a
// compinit later in .zshrc (oh-my-zsh, zinit) still runs once; a fresh zsh with no
// compinit at all (macOS has no default ~/.zshrc) gets one then.
const ZSH_COMPINIT: &str = r#"
if (( ! $+functions[compdef] )); then
  typeset -ga _arbor_compdefs
  # Quoted, so arguments with spaces (bashcompinit's `complete -C ...`) survive the replay.
  compdef() { _arbor_compdefs+=("${(j: :)${(q)@}}") }
  _arbor_compinit() {
    add-zsh-hook -d precmd _arbor_compinit
    local -a queued=("${_arbor_compdefs[@]}")
    local args
    _arbor_compdefs=()
    # A compdef that replaced the stub belongs to compinit, or to a framework that runs
    # it later (zinit turbo, zsh-defer).
    if [[ $functions[compdef] == *_arbor_compdefs* ]]; then
      autoload -Uz compinit && compinit -i
    fi
    for args in $queued; do eval "compdef $args"; done
    # Queued again: a wrapper around the stub, and nothing will run compinit.
    if (( $#_arbor_compdefs )); then
      autoload -Uz compinit && compinit -i
      for args in $_arbor_compdefs; do eval "compdef $args"; done
    fi
    unset _arbor_compdefs
    unfunction _arbor_compinit
  }
  autoload -Uz add-zsh-hook
  add-zsh-hook precmd _arbor_compinit
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
