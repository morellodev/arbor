use std::io::IsTerminal;

use anyhow::{Result, bail};

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

pub fn run(shell: Option<&str>) -> Result<()> {
    let shell = match shell {
        Some(s) => Shell::parse(s)?,
        None => Shell::detect()?,
    };

    if std::io::stdout().is_terminal() {
        print_setup_instructions(&shell)
    } else {
        print_script(&shell)
    }
}

fn print_setup_instructions(shell: &Shell) -> Result<()> {
    let (config_file, eval_line) = match shell {
        Shell::Bash => ("~/.bashrc", r#"eval "$(arbor init bash)""#),
        Shell::Zsh => ("~/.zshrc", r#"eval "$(arbor init zsh)""#),
        Shell::Fish => ("~/.config/fish/config.fish", "arbor init fish | source"),
    };

    display::print_note(&format!("Add the following to {config_file}:"));
    display::print_hint(eval_line);

    Ok(())
}

fn print_script(shell: &Shell) -> Result<()> {
    match shell {
        Shell::Bash => print!("{SHELL_WRAPPER}\n{BASH_COMPLETIONS}"),
        Shell::Zsh => print!("{SHELL_WRAPPER}\n{ZSH_COMPLETIONS}"),
        Shell::Fish => print!("{FISH_SNIPPET}"),
    }
    Ok(())
}

const SHELL_WRAPPER: &str = r#"arbor() {
  case "$1" in
    add|switch|clone|remove|rm|clean)
      local dir
      dir=$(command arbor "$@") || return $?
      if [ -n "$dir" ]; then cd "$dir"; fi
      ;;
    *)
      command arbor "$@"
      ;;
  esac
}"#;

const BASH_COMPLETIONS: &str = r#"
_arbor() {
  local cur prev subcmds
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"
  subcmds="add switch list ls remove rm dir clone clean prune status fetch init help"

  if [ "$COMP_CWORD" -eq 1 ]; then
    COMPREPLY=($(compgen -W "$subcmds" -- "$cur"))
    return
  fi

  case "${COMP_WORDS[1]}" in
    add)
      local branches
      branches=$(git for-each-ref --format='%(refname:short)' refs/heads/ refs/remotes/origin/ 2>/dev/null | sed 's|^origin/||' | sort -u)
      COMPREPLY=($(compgen -W "$branches" -- "$cur"))
      ;;
    switch|rm|remove|dir)
      local branches
      branches=$(git worktree list --porcelain 2>/dev/null | grep '^branch ' | sed 's|^branch refs/heads/||')
      COMPREPLY=($(compgen -W "$branches" -- "$cur"))
      ;;
  esac
}
complete -F _arbor arbor
"#;

const ZSH_COMPLETIONS: &str = r#"
_arbor() {
  local -a subcmds=(add switch list ls remove rm dir clone clean prune status fetch init help)
  if (( CURRENT == 2 )); then
    _describe 'command' subcmds
    return
  fi
  case "${words[2]}" in
    add)
      local -a branches=($(git for-each-ref --format='%(refname:short)' refs/heads/ refs/remotes/origin/ 2>/dev/null | sed 's|^origin/||' | sort -u))
      _describe 'branch' branches
      ;;
    switch|rm|remove|dir)
      local -a branches=($(git worktree list --porcelain 2>/dev/null | grep '^branch ' | sed 's|^branch refs/heads/||'))
      _describe 'branch' branches
      ;;
  esac
}
compdef _arbor arbor
"#;

const FISH_SNIPPET: &str = r#"function arbor --wraps arbor
  switch $argv[1]
    case add switch clone remove rm clean
      set -l dir (command arbor $argv)
      or return $status
      if test -n "$dir"
        cd $dir
      end
    case '*'
      command arbor $argv
  end
end

complete -c arbor -n '__fish_use_subcommand' -f -a 'add' -d 'Create a worktree for a branch'
complete -c arbor -n '__fish_use_subcommand' -f -a 'switch' -d 'Switch to an existing worktree'
complete -c arbor -n '__fish_use_subcommand' -f -a 'list' -d 'List worktrees'
complete -c arbor -n '__fish_use_subcommand' -f -a 'ls' -d 'List worktrees'
complete -c arbor -n '__fish_use_subcommand' -f -a 'remove' -d 'Remove a worktree'
complete -c arbor -n '__fish_use_subcommand' -f -a 'rm' -d 'Remove a worktree'
complete -c arbor -n '__fish_use_subcommand' -f -a 'dir' -d 'Print worktree path'
complete -c arbor -n '__fish_use_subcommand' -f -a 'clone' -d 'Clone a repository'
complete -c arbor -n '__fish_use_subcommand' -f -a 'clean' -d 'Interactively remove unused worktrees'
complete -c arbor -n '__fish_use_subcommand' -f -a 'prune' -d 'Remove stale worktree references'
complete -c arbor -n '__fish_use_subcommand' -f -a 'status' -d 'Show worktree status'
complete -c arbor -n '__fish_use_subcommand' -f -a 'fetch' -d 'Fetch from origin'
complete -c arbor -n '__fish_use_subcommand' -f -a 'init' -d 'Set up shell integration'
complete -c arbor -n '__fish_use_subcommand' -f -a 'help' -d 'Print help'

complete -c arbor -n '__fish_seen_subcommand_from add' -f -a '(git for-each-ref --format="%(refname:short)" refs/heads/ refs/remotes/origin/ 2>/dev/null | string replace -r "^origin/" "" | sort -u)'

complete -c arbor -n '__fish_seen_subcommand_from switch rm remove dir' -f -a '(git worktree list --porcelain 2>/dev/null | string match -r "^branch refs/heads/(.*)" | string replace -r "^branch refs/heads/" "")'
"#;
