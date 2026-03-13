use std::path::Path;

use colored::Colorize;

use crate::git::WorktreeInfo;

const BRANCH_WIDTH: usize = 24;
const STATE_WIDTH: usize = 8;
const TRACKING_WIDTH: usize = 30;

// --- Message helpers ---

pub fn print_ok(msg: &str) {
    eprintln!("{} {msg}", "ok:".green().bold());
}

pub fn print_note(msg: &str) {
    eprintln!("{} {msg}", "note:".cyan().bold());
}

pub fn print_cd_hint(path: &Path) {
    eprintln!("  {}", format!("cd {}", path.display()).dimmed());
}

// --- Label helpers (presentation logic for WorktreeInfo) ---

fn branch_label(entry: &WorktreeInfo) -> String {
    entry
        .branch
        .as_deref()
        .unwrap_or("(detached)")
        .to_string()
}

fn state_label(entry: &WorktreeInfo) -> &'static str {
    if entry.is_dirty() { "dirty" } else { "clean" }
}

enum TrackingState {
    UpToDate,
    Ahead,
    Behind,
    Diverged,
    NoUpstream,
}

fn tracking_state(entry: &WorktreeInfo) -> TrackingState {
    match entry.tracking {
        Some((0, 0)) => TrackingState::UpToDate,
        Some((_, 0)) => TrackingState::Ahead,
        Some((0, _)) => TrackingState::Behind,
        Some(_) => TrackingState::Diverged,
        None => TrackingState::NoUpstream,
    }
}

fn tracking_label(entry: &WorktreeInfo) -> String {
    match entry.tracking {
        Some((0, 0)) => "up-to-date".to_string(),
        Some((ahead, 0)) => format!("ahead {ahead}"),
        Some((0, behind)) => format!("behind {behind}"),
        Some((ahead, behind)) => format!("ahead {ahead}, behind {behind}"),
        None => "no upstream".to_string(),
    }
}

// --- Summary ---

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
        if wt.is_dirty() {
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

// --- Table ---

pub fn print_table(entries: &[WorktreeInfo], show_preview: bool) {
    print_header();
    for entry in entries {
        print_row(entry);
        if show_preview && entry.is_dirty() {
            print_preview(entry);
        }
    }
}

fn print_row(entry: &WorktreeInfo) {
    let branch = format!("{:width$}", branch_label(entry), width = BRANCH_WIDTH);
    let branch = if entry.branch.is_some() {
        branch.bold()
    } else {
        branch.yellow()
    };

    let state = format!("{:width$}", state_label(entry), width = STATE_WIDTH);
    let state = if entry.is_dirty() {
        state.yellow()
    } else {
        state.green()
    };

    let tracking = format!("{:width$}", tracking_label(entry), width = TRACKING_WIDTH);
    let tracking = match tracking_state(entry) {
        TrackingState::UpToDate => tracking.green(),
        TrackingState::Ahead => tracking.cyan(),
        TrackingState::Behind | TrackingState::Diverged => tracking.magenta(),
        TrackingState::NoUpstream => tracking.dimmed(),
    };

    let path = entry.path.display().to_string().dimmed();

    println!("{branch} {state} {tracking} {path}");
}

fn print_header() {
    let header = format!(
        "{:branch_width$} {:state_width$} {:tracking_width$} Path",
        "Branch",
        "State",
        "Tracking",
        branch_width = BRANCH_WIDTH,
        state_width = STATE_WIDTH,
        tracking_width = TRACKING_WIDTH
    );
    println!("{}", header.dimmed());
    println!("{}", "-".repeat(header.len()).dimmed());
}

fn print_preview(entry: &WorktreeInfo) {
    const INDENT: &str = "    ";
    println!("{}", format!("{INDENT}Changes:").dimmed());
    for line in &entry.status_preview {
        println!("{}", format!("{INDENT}  {line}").dimmed());
    }
    if entry.status_truncated() {
        println!(
            "{}",
            format!("{INDENT}  ... more changes not shown").dimmed()
        );
    }
}
