mod cli;
mod commands;
mod config;
mod display;
mod git;
mod hooks;

use std::process;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use config::Config;

fn main() {
    reset_sigpipe();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            eprint!("{}", e.render().ansi());
            process::exit(e.exit_code());
        }
    };
    display::configure_color(&cli.color);

    if let Err(e) = run(cli) {
        display::print_error(&format!("{e:#}"));
        process::exit(1);
    }
}

#[cfg(unix)]
fn reset_sigpipe() {
    unsafe extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(13, 0);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

// Config is loaded only where it's used, so a broken config.toml can't break
// `arbor init` at shell startup or the commands that never read it.
fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Add {
            ref branch,
            ref base,
            no_hooks,
        } => commands::add(&Config::load()?, branch, base.as_deref(), no_hooks),
        Command::Switch { ref branch } => commands::switch(branch.as_deref()),
        Command::List { all, json, short } => commands::list(all, json, short),
        Command::Remove {
            ref branch,
            force,
            delete_branch,
        } => commands::remove(branch.as_deref(), force, delete_branch),
        Command::Dir { ref branch } => commands::dir(branch.as_deref()),
        Command::Clone {
            ref url,
            no_worktree,
            hooks,
            ..
        } => commands::clone(&Config::load()?, url, no_worktree, hooks),
        Command::Clean {
            delete_branch,
            force,
        } => commands::clean(delete_branch, force),
        Command::Prune => commands::prune(),
        Command::Fetch { all } => commands::fetch(all),
        Command::Init { ref shell, inject } => commands::init(shell.as_deref(), inject),
    }
}
