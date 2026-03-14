use anyhow::{Result, bail};

pub fn run(shell: &str) -> Result<()> {
    let snippet = match shell {
        "bash" => format!("{SHELL_WRAPPER}\n{BASH_COMPLETIONS}"),
        "zsh" => format!("{SHELL_WRAPPER}\n{ZSH_COMPLETIONS}"),
        "fish" => FISH_SNIPPET.to_string(),
        _ => bail!("unsupported shell: {shell} (supported: bash, zsh, fish)"),
    };
    print!("{snippet}");
    Ok(())
}

const SHELL_WRAPPER: &str = r#"arbor() {
  case "$1" in
    add|switch|clone)
      local dir
      dir=$(command arbor "$@") && cd "$dir"
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
  subcmds="add switch list ls remove rm dir clone prune status fetch init completions"

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
  local -a subcmds=(add switch list ls remove rm dir clone prune status fetch init completions)
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
    case add switch clone
      set -l dir (command arbor $argv)
      and cd $dir
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
complete -c arbor -n '__fish_use_subcommand' -f -a 'prune' -d 'Remove stale worktree references'
complete -c arbor -n '__fish_use_subcommand' -f -a 'status' -d 'Show worktree status'
complete -c arbor -n '__fish_use_subcommand' -f -a 'fetch' -d 'Fetch from origin'
complete -c arbor -n '__fish_use_subcommand' -f -a 'init' -d 'Print shell integration snippet'
complete -c arbor -n '__fish_use_subcommand' -f -a 'completions' -d 'Generate shell completions'

complete -c arbor -n '__fish_seen_subcommand_from add' -f -a '(git for-each-ref --format="%(refname:short)" refs/heads/ refs/remotes/origin/ 2>/dev/null | string replace -r "^origin/" "" | sort -u)'

complete -c arbor -n '__fish_seen_subcommand_from switch rm remove dir' -f -a '(git worktree list --porcelain 2>/dev/null | string match -r "^branch refs/heads/(.*)" | string replace -r "^branch refs/heads/" "")'
"#;
