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
    },

    /// List worktrees for the current repository
    #[command(visible_alias = "ls")]
    List {
        /// Show worktrees for all repositories
        #[arg(long)]
        all: bool,
    },

    /// Remove the worktree for a branch
    #[command(visible_alias = "rm")]
    Remove {
        /// Branch name whose worktree to remove
        branch: String,
        /// Force removal even if the worktree is dirty
        #[arg(long, short)]
        force: bool,
    },

    /// Print the filesystem path of the worktree for a branch
    Dir {
        /// Branch name to look up
        branch: String,
    },

    /// Clone a repository as a bare repo for worktree-based workflows
    Clone {
        /// Repository URL to clone
        url: String,
    },

    /// Remove references to stale worktrees
    Prune,

    /// Show the status of all worktrees (dirty/clean, ahead/behind)
    Status,
}
