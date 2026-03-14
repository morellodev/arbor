mod cli;
mod commands;
mod config;
mod display;
mod git;

use std::io::IsTerminal;
use std::process;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use config::Config;

fn main() {
    // colored checks stdout for TTY, but arbor writes all colored output to stderr.
    // Force colors on when stderr is a terminal (e.g. stdout captured by shell wrapper).
    if std::io::stderr().is_terminal() {
        colored::control::set_override(true);
    }

    if let Err(e) = run() {
        display::print_error(&format!("{e:#}"));
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Command::Add {
            ref branch,
            ref repo,
        } => commands::add(&config, branch, repo.as_deref()),
        Command::Switch { ref branch } => commands::switch(branch),
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
        Command::Status { short, all } => commands::status(&config, short, all),
        Command::Fetch { all } => commands::fetch(&config, all),
        Command::Init { ref shell } => commands::init(shell.as_deref()),
    }
}
