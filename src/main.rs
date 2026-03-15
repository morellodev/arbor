mod cli;
mod commands;
mod config;
mod display;
mod git;
mod hooks;

use std::io::IsTerminal;
use std::process;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, ColorMode, Command};
use config::Config;

fn main() {
    let cli = Cli::parse();
    configure_color(&cli.color);

    if let Err(e) = run(cli) {
        display::print_error(&format!("{e:#}"));
        process::exit(1);
    }
}

fn configure_color(mode: &ColorMode) {
    let no_color = std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());

    match mode {
        ColorMode::Never => colored::control::set_override(false),
        ColorMode::Always => colored::control::set_override(true),
        ColorMode::Auto => {
            if no_color {
                colored::control::set_override(false);
            } else if std::io::stderr().is_terminal() {
                colored::control::set_override(true);
            }
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let config = Config::load()?;

    match cli.command {
        Command::Add {
            ref branch,
            ref repo,
            no_hooks,
        } => commands::add(&config, branch, repo.as_deref(), no_hooks),
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
            no_hooks,
        } => commands::clone(&config, url, no_worktree, no_hooks),
        Command::Clean { delete_branch } => commands::clean(delete_branch),
        Command::Prune => commands::prune(),
        Command::Status { short, all } => commands::status(&config, short, all),
        Command::Fetch { all } => commands::fetch(&config, all),
        Command::Init { ref shell } => commands::init(shell.as_deref()),
    }
}
