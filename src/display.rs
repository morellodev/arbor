use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use colored::Colorize;
use comfy_table::{ContentArrangement, Table, presets::NOTHING};
use dialoguer::FuzzySelect;

use crate::git::{self, Tracking, WorktreeInfo};

pub fn configure_color(mode: &crate::cli::ColorMode) {
    let no_color = std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());

    match mode {
        crate::cli::ColorMode::Never => colored::control::set_override(false),
        crate::cli::ColorMode::Always => colored::control::set_override(true),
        crate::cli::ColorMode::Auto => {
            if no_color {
                colored::control::set_override(false);
            } else if std::io::stderr().is_terminal() {
                colored::control::set_override(true);
            }
        }
    }
}

pub fn cwd_is_inside(cwd: &Path, worktree_path: &Path) -> bool {
    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let worktree_path = worktree_path
        .canonicalize()
        .unwrap_or_else(|_| worktree_path.to_path_buf());
    cwd.starts_with(&worktree_path)
}

pub fn escape_dir_if_cwd_inside(wt_path: &Path) -> Result<Option<PathBuf>> {
    let inside = std::env::current_dir()
        .ok()
        .is_some_and(|cwd| cwd_is_inside(&cwd, wt_path));
    if inside {
        Ok(Some(git::repo_toplevel()?))
    } else {
        Ok(None)
    }
}

pub fn fuzzy_select_worktree(
    prompt: &str,
    non_interactive_hint: &str,
) -> Result<Option<WorktreeInfo>> {
    if !std::io::stdin().is_terminal() {
        bail!("Interactive terminal required. {non_interactive_hint}");
    }

    let mut worktrees = git::worktree_infos(None)?;

    if worktrees.is_empty() {
        print_note("No worktrees found");
        return Ok(None);
    }

    let items = format_worktree_items(&worktrees);

    let selection = FuzzySelect::new()
        .with_prompt(prompt)
        .items(&items)
        .interact_opt()?;

    match selection {
        Some(idx) => Ok(Some(worktrees.swap_remove(idx))),
        None => {
            print_note("Nothing selected");
            Ok(None)
        }
    }
}

fn find_current_index(entries: &[WorktreeInfo]) -> Option<usize> {
    let cwd = std::env::current_dir().ok()?;
    entries.iter().position(|wt| cwd_is_inside(&cwd, &wt.path))
}

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
    match &entry.tracking {
        Some(Tracking {
            ahead: 0,
            behind: 0,
        }) => "=".green().to_string(),
        Some(Tracking { ahead, behind: 0 }) => format!("\u{2191}{ahead}").cyan().to_string(),
        Some(Tracking { ahead: 0, behind }) => format!("\u{2193}{behind}").magenta().to_string(),
        Some(Tracking { ahead, behind }) => format!("\u{2191}{ahead} \u{2193}{behind}")
            .magenta()
            .to_string(),
        None => "\u{2014}".dimmed().to_string(),
    }
}

fn branch_visible_len(entry: &WorktreeInfo) -> usize {
    match &entry.branch {
        Some(name) => name.len(),
        None => "(detached)".len(),
    }
}

pub fn format_worktree_items(entries: &[WorktreeInfo]) -> Vec<String> {
    let current = find_current_index(entries);
    let max_branch = entries.iter().map(branch_visible_len).max().unwrap_or(0);

    entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let marker = if current == Some(i) {
                format!("{} ", "*".green().bold())
            } else {
                "  ".to_string()
            };
            let branch = colored_branch(entry);
            let pad = max_branch - branch_visible_len(entry);
            let state = colored_state(entry);
            let tracking = colored_tracking(entry);
            let path = shorten_path(&entry.path).dimmed().to_string();

            format!(
                "{marker}{branch}{}  {state}  {tracking}  {path}",
                " ".repeat(pad)
            )
        })
        .collect()
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
        if let Some(ref t) = wt.tracking {
            if t.ahead > 0 {
                ahead += 1;
            }
            if t.behind > 0 {
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
    let noun = if total == 1 {
        "repository"
    } else {
        "repositories"
    };
    if failed > 0 {
        print_note(&format!(
            "Fetched {success}/{total} {noun} ({} failed)",
            failed.to_string().red()
        ));
    } else {
        print_note(&format!("Fetched {total} {noun}"));
    }
}

pub fn print_batch_summary(summaries: &[WorktreeSummary]) {
    let aggregate = summaries.iter().fold(
        WorktreeSummary {
            total: 0,
            dirty: 0,
            ahead: 0,
            behind: 0,
            detached: 0,
        },
        |mut acc, s| {
            acc.total += s.total;
            acc.dirty += s.dirty;
            acc.ahead += s.ahead;
            acc.behind += s.behind;
            acc.detached += s.detached;
            acc
        },
    );
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
    let current = find_current_index(entries);
    let mut table = new_table();

    let mut header = vec![
        "".to_string(),
        "Branch".dimmed().to_string(),
        "State".dimmed().to_string(),
        "Tracking".dimmed().to_string(),
    ];
    if show_paths {
        header.push("Path".dimmed().to_string());
    }
    table.set_header(header);

    for (i, entry) in entries.iter().enumerate() {
        let marker = if current == Some(i) {
            format!("{}", "*".green().bold())
        } else {
            String::new()
        };
        let mut row = vec![
            marker,
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
