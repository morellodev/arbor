use std::path::Path;

use colored::Colorize;
use comfy_table::{ContentArrangement, Table, presets::NOTHING};

use crate::git::WorktreeInfo;

pub fn print_ok(msg: &str) {
    eprintln!("{} {msg}", "✓".green().bold());
}

pub fn print_error(msg: &str) {
    eprintln!("{} {msg}", "✗".red().bold());
}

pub fn print_note(msg: &str) {
    eprintln!("{} {msg}", "▸".dimmed());
}

pub fn print_heading(text: &str) {
    eprintln!("{}", text.bold());
}

pub fn print_section(name: &str) {
    eprintln!("{}{}", "# ".bold(), name.bold());
}

pub fn print_hint(text: &str) {
    eprintln!("  {}", text.dimmed());
}

pub fn print_cd_hint(path: &Path) {
    print_hint(&format!("cd {}", shorten_path(path)));
}

pub fn print_path_hint(path: &Path) {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
        eprintln!("To switch to it, run:");
        print_cd_hint(path);
    } else {
        println!("{}", path.display());
    }
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
        "\u{2717}".yellow().to_string()
    } else {
        "\u{2713}".green().to_string()
    }
}

fn colored_tracking(entry: &WorktreeInfo) -> String {
    match entry.tracking {
        Some((0, 0)) => "=".green().to_string(),
        Some((ahead, 0)) => format!("\u{2191}{ahead}").cyan().to_string(),
        Some((0, behind)) => format!("\u{2193}{behind}").magenta().to_string(),
        Some((ahead, behind)) => format!("\u{2191}{ahead} \u{2193}{behind}")
            .magenta()
            .to_string(),
        None => "\u{2014}".dimmed().to_string(),
    }
}

pub fn format_worktree_item(entry: &WorktreeInfo) -> String {
    let branch = colored_branch(entry);
    let state = colored_state(entry);
    let tracking = colored_tracking(entry);
    let path = shorten_path(&entry.path).dimmed().to_string();

    format!("{branch}  {state}  {tracking}  {path}")
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

pub fn print_fetch_summary(success: usize, failed: usize) {
    let total = success + failed;
    if failed > 0 {
        eprintln!(
            "{} Fetched {}/{} repositories ({} failed)",
            "\u{25b8}".dimmed(),
            success,
            total,
            failed.to_string().red()
        );
    } else {
        eprintln!(
            "{} Fetched {} {}",
            "\u{25b8}".dimmed(),
            total,
            if total == 1 {
                "repository"
            } else {
                "repositories"
            }
        );
    }
}

pub fn print_batch_summary(summaries: &[WorktreeSummary]) {
    let aggregate = WorktreeSummary {
        total: summaries.iter().map(|s| s.total).sum(),
        dirty: summaries.iter().map(|s| s.dirty).sum(),
        ahead: summaries.iter().map(|s| s.ahead).sum(),
        behind: summaries.iter().map(|s| s.behind).sum(),
        detached: summaries.iter().map(|s| s.detached).sum(),
    };
    let repos = summaries.len();
    let label = format!(
        "Total ({} {})",
        repos,
        if repos == 1 { "repo" } else { "repos" }
    );
    eprintln!("{}", format_summary(&label, &aggregate));
}

fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

pub fn print_table(entries: &[WorktreeInfo], show_paths: bool) {
    let mut table = new_table();

    let mut header = vec![
        "Branch".dimmed().to_string(),
        "State".dimmed().to_string(),
        "Tracking".dimmed().to_string(),
    ];
    if show_paths {
        header.push("Path".dimmed().to_string());
    }
    table.set_header(header);

    for entry in entries {
        let mut row = vec![
            colored_branch(entry),
            colored_state(entry),
            colored_tracking(entry),
        ];
        if show_paths {
            row.push(shorten_path(&entry.path).dimmed().to_string());
        }
        table.add_row(row);
    }

    println!("{table}");
}
