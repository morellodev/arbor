use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "arbor", version, about = "A friendly git worktree manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a worktree for a branch (or switch to it if it already exists)
    Add {
        /// Branch name to create a worktree for
        branch: String,
        /// Repository name (resolve against ~/.arbor/repos/) to add from any directory
        #[arg(long)]
        repo: Option<String>,
    },

    /// Switch to an existing worktree
    Switch {
        /// Branch name to switch to
        branch: String,
    },

    /// List worktrees for the current repository
    #[command(visible_alias = "ls")]
    List {
        /// Show worktrees for all repositories
        #[arg(long)]
        all: bool,
        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Remove the worktree for a branch
    #[command(visible_alias = "rm")]
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
    Clone {
        /// Repository URL or "user/repo" shorthand (defaults to GitHub)
        url: String,
        /// Don't create a worktree for the default branch after cloning
        #[arg(long)]
        no_worktree: bool,
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

    /// Print shell integration snippet for eval
    Init {
        /// Shell to generate integration for (bash, zsh, fish)
        shell: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}
