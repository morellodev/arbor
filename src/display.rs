use std::path::Path;

use colored::Colorize;
use comfy_table::{ContentArrangement, Table, presets::NOTHING};

use crate::git::WorktreeInfo;

pub fn print_ok(msg: &str) {
    eprintln!("{} {msg}", "ok:".green().bold());
}

pub fn print_note(msg: &str) {
    eprintln!("{} {msg}", "note:".cyan().bold());
}

pub fn print_cd_hint(path: &Path) {
    eprintln!("  {}", format!("cd {}", shorten_path(path)).dimmed());
}

pub fn shorten_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(relative) = path.strip_prefix(&home)
    {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}

fn colored_branch(entry: &WorktreeInfo) -> String {
    match &entry.branch {
        Some(name) => name.bold().to_string(),
        None => "(detached)".yellow().to_string(),
    }
}

fn colored_state(entry: &WorktreeInfo) -> String {
    if entry.dirty {
        "dirty".yellow().to_string()
    } else {
        "clean".green().to_string()
    }
}

fn colored_tracking(entry: &WorktreeInfo) -> String {
    match entry.tracking {
        Some((0, 0)) => "up-to-date".green().to_string(),
        Some((ahead, 0)) => format!("ahead {ahead}").cyan().to_string(),
        Some((0, behind)) => format!("behind {behind}").magenta().to_string(),
        Some((ahead, behind)) => format!("ahead {ahead}, behind {behind}")
            .magenta()
            .to_string(),
        None => "no upstream".dimmed().to_string(),
    }
}

pub struct WorktreeSummary {
    pub total: usize,
    pub dirty: usize,
    pub ahead: usize,
    pub behind: usize,
    pub detached: usize,
}

pub fn summarize(worktrees: &[WorktreeInfo]) -> WorktreeSummary {
    let mut dirty = 0;
    let mut ahead = 0;
    let mut behind = 0;
    let mut detached = 0;

    for wt in worktrees {
        if wt.dirty {
            dirty += 1;
        }
        if let Some((a, b)) = wt.tracking {
            if a > 0 {
                ahead += 1;
            }
            if b > 0 {
                behind += 1;
            }
        }
        if wt.branch.is_none() {
            detached += 1;
        }
    }

    WorktreeSummary {
        total: worktrees.len(),
        dirty,
        ahead,
        behind,
        detached,
    }
}

pub fn format_summary(label: &str, summary: &WorktreeSummary) -> String {
    let mut parts = Vec::new();

    if summary.dirty > 0 {
        parts.push(format!("{} dirty", summary.dirty).yellow().to_string());
    }
    if summary.ahead > 0 {
        parts.push(format!("{} ahead", summary.ahead).cyan().to_string());
    }
    if summary.behind > 0 {
        parts.push(format!("{} behind", summary.behind).magenta().to_string());
    }
    if summary.detached > 0 {
        parts.push(
            format!("{} detached", summary.detached)
                .yellow()
                .to_string(),
        );
    }

    let details = if parts.is_empty() {
        "all clean".green().to_string()
    } else {
        parts.join(", ")
    };

    format!(
        "{} {} {} worktrees ({})",
        label.bold(),
        "—".dimmed(),
        summary.total,
        details,
    )
}

fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

pub fn print_table(entries: &[WorktreeInfo]) {
    let mut table = new_table();
    table.set_header(vec![
        "Branch".dimmed().to_string(),
        "State".dimmed().to_string(),
        "Tracking".dimmed().to_string(),
        "Path".dimmed().to_string(),
    ]);

    for entry in entries {
        table.add_row(vec![
            colored_branch(entry),
            colored_state(entry),
            colored_tracking(entry),
            shorten_path(&entry.path).dimmed().to_string(),
        ]);
    }

    println!("{table}");
}

pub fn print_short_table(entries: &[WorktreeInfo]) {
    let mut table = new_table();

    for entry in entries {
        table.add_row(vec![
            colored_branch(entry),
            colored_state(entry),
            colored_tracking(entry),
        ]);
    }

    println!("{table}");
}

