mod cli;
mod commands;
mod config;
mod git;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Command::Add { ref branch } => commands::add(&config, branch),
        Command::List { all } => commands::list(&config, all),
        Command::Remove { ref branch, force } => commands::remove(&config, branch, force),
        Command::Dir { ref branch } => commands::dir(&config, branch),
        Command::Clone { ref url } => commands::clone(&config, url),
        Command::Prune => commands::prune(),
        Command::Status => commands::status(),
    }
}
