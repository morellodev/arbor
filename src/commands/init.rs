use std::io::IsTerminal;

use anyhow::{Result, bail};
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

fn generate_completions(shell: clap_complete::Shell) -> String {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    generate(shell, &mut cmd, "arbor", &mut buf);
    String::from_utf8(buf).expect("clap_complete should produce valid UTF-8")
}

fn print_script(shell: &Shell) -> Result<()> {
    match shell {
        Shell::Bash => {
            print!("{SHELL_WRAPPER}\n");
            print!("{}", generate_completions(clap_complete::Shell::Bash));
            print!("{BASH_BRANCH_COMPLETIONS}");
        }
        Shell::Zsh => {
            print!("{SHELL_WRAPPER}\n");
            print!("{}", generate_completions(clap_complete::Shell::Zsh));
            print!("{ZSH_BRANCH_COMPLETIONS}");
        }
        Shell::Fish => {
            print!("{FISH_WRAPPER}\n");
            print!("{}", generate_completions(clap_complete::Shell::Fish));
            print!("{FISH_BRANCH_COMPLETIONS}");
        }
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

const FISH_WRAPPER: &str = r#"function arbor --wraps arbor
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
end"#;

const BASH_BRANCH_COMPLETIONS: &str = r#"
_arbor_branches() {
  _arbor
  case "${COMP_WORDS[1]}" in
    add)
      local branches
      branches=$(git for-each-ref --format='%(refname:short)' refs/heads/ refs/remotes/origin/ 2>/dev/null | sed 's|^origin/||' | sort -u)
      COMPREPLY=($(compgen -W "$branches" -- "${COMP_WORDS[COMP_CWORD]}"))
      ;;
    switch|rm|remove|dir)
      local branches
      branches=$(git worktree list --porcelain 2>/dev/null | grep '^branch ' | sed 's|^branch refs/heads/||')
      COMPREPLY=($(compgen -W "$branches" -- "${COMP_WORDS[COMP_CWORD]}"))
      ;;
  esac
}
complete -F _arbor_branches arbor
"#;

const ZSH_BRANCH_COMPLETIONS: &str = r#"
_arbor_branches() {
  _arbor "$@"
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
compdef _arbor_branches arbor
"#;

const FISH_BRANCH_COMPLETIONS: &str = r#"
complete -c arbor -n '__fish_seen_subcommand_from add' -f -a '(git for-each-ref --format="%(refname:short)" refs/heads/ refs/remotes/origin/ 2>/dev/null | string replace -r "^origin/" "" | sort -u)'

complete -c arbor -n '__fish_seen_subcommand_from switch rm remove dir' -f -a '(git worktree list --porcelain 2>/dev/null | string match -r "^branch refs/heads/(.*)" | string replace -r "^branch refs/heads/" "")'
"#;
