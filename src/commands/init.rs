use anyhow::{Result, bail};

pub fn run(shell: &str) -> Result<()> {
    let snippet = match shell {
        "bash" | "zsh" => BASH_ZSH_SNIPPET,
        "fish" => FISH_SNIPPET,
        _ => bail!("unsupported shell: {shell} (supported: bash, zsh, fish)"),
    };
    print!("{snippet}");
    Ok(())
}

const BASH_ZSH_SNIPPET: &str = r#"arbor() {
  if [ "$1" = "add" ]; then
    local dir
    dir=$(command arbor "$@") && cd "$dir"
  else
    command arbor "$@"
  fi
}
"#;

const FISH_SNIPPET: &str = r#"function arbor --wraps arbor
  if test "$argv[1]" = "add"
    set -l dir (command arbor $argv)
    and cd $dir
  else
    command arbor $argv
  end
end
"#;
