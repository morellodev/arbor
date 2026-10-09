use std::borrow::Cow;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, bail};
use colored::Colorize;
use comfy_table::{ContentArrangement, Table, presets::NOTHING};
use dialoguer::FuzzySelect;
use unicode_width::UnicodeWidthStr;

use crate::git::{self, Tracking, WorktreeInfo};

static STDERR_COLOR: AtomicBool = AtomicBool::new(false);
static STDOUT_COLOR: AtomicBool = AtomicBool::new(false);

pub fn configure_color(mode: &crate::cli::ColorMode) {
    let no_color = std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());
    let force = std::env::var("CLICOLOR_FORCE").is_ok_and(|v| !v.is_empty() && v != "0");
    let auto = |is_terminal: bool| !no_color && (force || is_terminal);

    let (stderr, stdout) = match mode {
        crate::cli::ColorMode::Never => (false, false),
        crate::cli::ColorMode::Always => (true, true),
        crate::cli::ColorMode::Auto => (
            auto(std::io::stderr().is_terminal()),
            auto(std::io::stdout().is_terminal()),
        ),
    };

    STDERR_COLOR.store(stderr, Ordering::Relaxed);
    STDOUT_COLOR.store(stdout, Ordering::Relaxed);
    colored::control::set_override(stderr);
}

// `colored` has a single global switch, so output bound elsewhere than stderr flips
// it while rendering.
fn with_colors<T>(enabled: bool, render: impl FnOnce() -> T) -> T {
    colored::control::set_override(enabled);
    let rendered = render();
    colored::control::set_override(STDERR_COLOR.load(Ordering::Relaxed));
    rendered
}

fn with_stdout_colors<T>(render: impl FnOnce() -> T) -> T {
    with_colors(STDOUT_COLOR.load(Ordering::Relaxed), render)
}

pub fn cwd_is_inside(cwd: &Path, worktree_path: &Path) -> bool {
    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let worktree_path = worktree_path
        .canonicalize()
        .unwrap_or_else(|_| worktree_path.to_path_buf());
    cwd.starts_with(&worktree_path)
}

/// Index of the deepest path containing `cwd`, so a worktree nested inside the
/// main checkout wins over the main checkout.
pub fn innermost_containing<'a>(
    cwd: &Path,
    paths: impl IntoIterator<Item = &'a Path>,
) -> Option<usize> {
    paths
        .into_iter()
        .enumerate()
        .filter(|(_, path)| cwd_is_inside(cwd, path))
        .max_by_key(|(_, path)| path.components().count())
        .map(|(i, _)| i)
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

    // The fuzzy matcher searches the raw item text, color codes included.
    let items = with_colors(false, || format_worktree_items(&worktrees));

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
    innermost_containing(&cwd, entries.iter().map(|wt| wt.path.as_path()))
}

// Messages often carry repo-supplied text (hook commands, branch names, paths), which
// must not be able to rewrite or hide terminal output, e.g. with `\r` plus erase-line.
fn is_hidden(c: char) -> bool {
    (c.is_control() && c != '\n' && c != '\t')
        || matches!(
            c,
            '\u{061C}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2028}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
}

fn sanitize(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_hidden) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_hidden(c) {
            out.extend(c.escape_unicode());
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

pub fn print_ok(msg: &str) {
    eprintln!("{} {}", "✓".green().bold(), sanitize(msg));
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), sanitize(msg));
}

pub fn print_note(msg: &str) {
    note_line(&sanitize(msg));
}

fn note_line(msg: &str) {
    eprintln!("{} {msg}", "▸".dimmed());
}

pub fn print_heading(text: &str) {
    eprintln!("{}", sanitize(text).bold());
}

pub fn print_section(name: &str) {
    eprintln!("{}{}", "# ".bold(), sanitize(name).bold());
}

pub fn print_listing_section(name: &str) {
    let heading = with_stdout_colors(|| format!("{}{}", "# ".bold(), sanitize(name).bold()));
    println!("{heading}");
}

pub fn print_hint(text: &str) {
    eprintln!("  {}", sanitize(text).dimmed());
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
    let Some(home) = std::env::home_dir() else {
        return path.display().to_string();
    };
    // git reports resolved paths, so a symlinked HOME (e.g. /tmp on macOS) only
    // matches once canonicalized.
    let canonical_home = home.canonicalize().ok().map(strip_verbatim);
    let relative = std::iter::once(home.as_path())
        .chain(canonical_home.as_deref())
        .find_map(|home| path.strip_prefix(home).ok());
    match relative {
        Some(relative) if relative.as_os_str().is_empty() => "~".to_string(),
        Some(relative) => format!("~/{}", relative.display()),
        None => path.display().to_string(),
    }
}

/// Windows `canonicalize` returns `\\?\C:\...`, which never prefix-matches the
/// `C:/...` paths git prints.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    if cfg!(windows)
        && let Some(rest) = path.to_str().and_then(|p| p.strip_prefix(r"\\?\"))
        && !rest.starts_with("UNC\\")
    {
        return PathBuf::from(rest);
    }
    path
}

fn colored_branch(entry: &WorktreeInfo) -> String {
    match &entry.branch {
        Some(name) => sanitize(name).bold().to_string(),
        None => "(detached)".yellow().to_string(),
    }
}

fn colored_state(entry: &WorktreeInfo) -> String {
    if entry.missing {
        "missing".red().to_string()
    } else if entry.dirty {
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
        Some(name) => sanitize(name).width(),
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
            let path = sanitize(&shorten_path(&entry.path)).dimmed().to_string();

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

pub fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

fn format_summary(label: &str, summary: &WorktreeSummary) -> String {
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
        "{} {} {} {} ({})",
        sanitize(label).bold(),
        "—".dimmed(),
        summary.total,
        plural(summary.total, "worktree", "worktrees"),
        details,
    )
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
    let label = format!("Total ({repos} {})", plural(repos, "repo", "repos"));
    println!(
        "{}",
        with_stdout_colors(|| format_summary(&label, &aggregate))
    );
}

fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_style(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

pub fn print_summary(label: &str, summary: &WorktreeSummary) {
    println!("{}", with_stdout_colors(|| format_summary(label, summary)));
}

pub fn print_table(entries: &[WorktreeInfo], show_paths: bool) {
    let table = with_stdout_colors(|| build_table(entries, show_paths));
    println!("{table}");
}

fn build_table(entries: &[WorktreeInfo], show_paths: bool) -> Table {
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
            row.push(sanitize(&shorten_path(&entry.path)).dimmed().to_string());
        }
        table.add_row(row);
    }

    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_reveals_hidden_characters() {
        assert_eq!(
            sanitize("touch x #\r\u{1b}[2K  npm install"),
            "touch x #\\u{d}\\u{1b}[2K  npm install"
        );
        assert_eq!(sanitize("a\u{202E}b"), "a\\u{202e}b");
        assert_eq!(sanitize("cp \"a b\" c"), "cp \"a b\" c");
        assert_eq!(sanitize("line one\n\tline two"), "line one\n\tline two");
        assert_eq!(sanitize("a\u{2028}b\u{061C}c"), "a\\u{2028}b\\u{61c}c");
    }

    #[test]
    fn worktree_items_align_wide_branch_names() {
        colored::control::set_override(false);
        let worktree = |branch: &str| WorktreeInfo {
            path: PathBuf::from("/wt"),
            branch: Some(branch.to_string()),
            dirty: false,
            tracking: None,
            missing: false,
            main: false,
            in_progress: None,
        };
        let items =
            format_worktree_items(&[worktree("功能"), worktree("🚀ship"), worktree("abcdef")]);
        let state_column = |item: &str| item[..item.find('✓').unwrap()].width();
        assert_eq!(state_column(&items[0]), state_column(&items[2]));
        assert_eq!(state_column(&items[1]), state_column(&items[2]));
    }

    #[test]
    fn plural_picks_singular_only_for_one() {
        assert_eq!(plural(0, "branch", "branches"), "branches");
        assert_eq!(plural(1, "branch", "branches"), "branch");
        assert_eq!(plural(2, "branch", "branches"), "branches");
    }

    #[test]
    fn format_summary_uses_singular_for_one_worktree() {
        colored::control::set_override(false);
        let summary = WorktreeSummary {
            total: 1,
            dirty: 0,
            ahead: 0,
            behind: 0,
            detached: 0,
        };
        assert_eq!(
            format_summary("repo", &summary),
            "repo — 1 worktree (all clean)"
        );
    }
}
