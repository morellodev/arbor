use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Parser)]
#[command(name = "arbor", version, about = "A friendly git worktree manager")]
pub struct Cli {
    /// When to use colors: auto, always, or never
    #[arg(long, global = true, default_value = "auto")]
    pub color: ColorMode,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a worktree for a branch (or switch to it if it already exists)
    #[command(after_help = "\
Examples:
  arbor add feature/auth
  arbor add main --repo myapp")]
    Add {
        /// Branch name to create a worktree for
        branch: String,
        /// Repository name (resolve against ~/.arbor/repos/) to add from any directory
        #[arg(long)]
        repo: Option<String>,
        /// Skip post-create hooks
        #[arg(long)]
        no_hooks: bool,
    },

    /// Switch to an existing worktree
    #[command(
        visible_alias = "cd",
        after_help = "\
Examples:
  arbor switch feature/auth
  arbor switch feature-auth
  arbor switch              # interactive fuzzy selection"
    )]
    Switch {
        /// Branch name to switch to (interactive selection if omitted)
        branch: Option<String>,
    },

    /// List worktrees for the current repository
    #[command(
        visible_alias = "ls",
        after_help = "\
Examples:
  arbor ls
  arbor ls --all --json"
    )]
    List {
        /// Show worktrees for all repositories
        #[arg(long)]
        all: bool,
        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Remove the worktree for a branch
    #[command(
        visible_alias = "rm",
        after_help = "\
Examples:
  arbor rm feature/auth
  arbor rm -d feature/auth
  arbor rm -f -d stale-branch"
    )]
    Remove {
        /// Branch name whose worktree to remove
        branch: String,
        /// Force removal even if the worktree is dirty
        #[arg(long, short)]
        force: bool,
        /// Also delete the local branch after removing the worktree
        #[arg(long, short)]
        delete_branch: bool,
    },

    /// Print the filesystem path of the worktree for a branch
    Dir {
        /// Branch name to look up
        branch: String,
    },

    /// Clone a repository as a bare repo for worktree-based workflows
    #[command(after_help = "\
Examples:
  arbor clone user/repo
  arbor clone git@github.com:user/repo.git")]
    Clone {
        /// Repository URL or "user/repo" shorthand (defaults to GitHub)
        url: String,
        /// Don't create a worktree for the default branch after cloning
        #[arg(long)]
        no_worktree: bool,
        /// Skip post-create hooks
        #[arg(long)]
        no_hooks: bool,
    },

    /// Interactively remove unused worktrees
    Clean {
        /// Also delete local branches after removing worktrees
        #[arg(long, short)]
        delete_branch: bool,
    },

    /// Remove references to stale worktrees
    Prune,

    /// Show the status of all worktrees (dirty/clean, ahead/behind)
    Status {
        /// Show compact one-line-per-worktree output (no paths)
        #[arg(long)]
        short: bool,
        /// Show status across all repositories
        #[arg(long)]
        all: bool,
    },

    /// Fetch from origin in the current bare repo
    Fetch {
        /// Fetch across all repositories
        #[arg(long)]
        all: bool,
    },

    /// Set up shell integration (completions + cd wrapper)
    #[command(after_help = "\
Examples:
  arbor init              # auto-detects your shell
  arbor init zsh          # explicit shell
  eval \"$(arbor init)\"    # activate in current session")]
    Init {
        /// Shell to generate integration for (bash, zsh, fish). Auto-detected from $SHELL if omitted.
        shell: Option<String>,
        /// Automatically add the eval line to your shell config (non-interactive)
        #[arg(long)]
        inject: bool,
    },
}
