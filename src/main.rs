mod cli;
mod commands;
mod config;
mod display;
mod git;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use cli::{Cli, Command};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Command::Add {
            ref branch,
            ref repo,
        } => commands::add(&config, branch, repo.as_deref()),
        Command::List { all, json } => commands::list(&config, all, json),
        Command::Remove {
            ref branch,
            force,
            delete_branch,
        } => commands::remove(&config, branch, force, delete_branch),
        Command::Dir { ref branch } => commands::dir(branch),
        Command::Clone {
            ref url,
            no_worktree,
        } => commands::clone(&config, url, no_worktree),
        Command::Prune => commands::prune(),
        Command::Status { short } => commands::status(short),
        Command::Fetch => commands::fetch(),
        Command::Init { ref shell } => commands::init(shell),
        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "arbor", &mut std::io::stdout());
            Ok(())
        }
    }
}
