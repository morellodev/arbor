use anyhow::Result;

use crate::{display, git};

pub fn run() -> Result<()> {
    let toplevel = git::repo_toplevel()?;
    display::print_note("Fetching from origin...");
    git::fetch_origin(&toplevel)?;
    display::print_ok("Fetch complete");
    Ok(())
}
