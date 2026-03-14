use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::{config::Config, display, git};

pub fn run(config: &Config, all: bool, json: bool) -> Result<()> {
    if json {
        if all {
            list_all_repos_json(config)
        } else {
            list_repo_json(None)
        }
    } else if all {
        list_all_repos(config)
    } else {
        list_repo(None)
    }
}

pub(crate) struct RepoEntry {
    pub(crate) display_name: String,
    pub(crate) path: std::path::PathBuf,
    pub(crate) worktrees: Vec<git::WorktreeInfo>,
}

pub(crate) fn scan_repos(config: &Config) -> Result<Vec<RepoEntry>> {
    let repos_dir = &config.repos_dir;
    if !repos_dir.exists() {
        return Ok(Vec::new());
    }

    let mut dir_entries: Vec<_> = fs::read_dir(repos_dir)?.filter_map(|e| e.ok()).collect();
    dir_entries.sort_by_key(|e| e.file_name());

    let mut repos = Vec::new();
    for entry in dir_entries {
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let display_name = git::strip_git_suffix(&name).to_string();

        if let Ok(worktrees) = git::worktree_infos(Some(&path))
            && !worktrees.is_empty()
        {
            repos.push(RepoEntry {
                display_name,
                path,
                worktrees,
            });
        }
    }

    Ok(repos)
}

fn list_repo(cwd: Option<&Path>) -> Result<()> {
    let worktrees = git::worktree_infos(cwd)?;
    if worktrees.is_empty() {
        display::print_note("No worktrees found.");
        return Ok(());
    }

    let label = git::repo_name_or_unknown();

    let summary = display::summarize(&worktrees);
    println!("{}", display::format_summary(&label, &summary));
    display::print_table(&worktrees, true);
    Ok(())
}

fn list_all_repos(config: &Config) -> Result<()> {
    let repos = scan_repos(config)?;

    if repos.is_empty() {
        display::print_note(&format!(
            "No repositories found in {}",
            config.repos_dir.display()
        ));
        return Ok(());
    }

    for repo in &repos {
        display::print_section(&repo.display_name);
        let summary = display::summarize(&repo.worktrees);
        println!("{}", display::format_summary("Summary", &summary));
        display::print_table(&repo.worktrees, true);
        println!();
    }

    Ok(())
}

fn list_repo_json(cwd: Option<&Path>) -> Result<()> {
    let worktrees = git::worktree_infos(cwd)?;
    let json: Vec<serde_json::Value> = worktrees.iter().map(worktree_to_json).collect();
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn list_all_repos_json(config: &Config) -> Result<()> {
    let repos = scan_repos(config)?;
    let mut result = serde_json::Map::new();

    for repo in repos {
        let json: Vec<serde_json::Value> = repo.worktrees.iter().map(worktree_to_json).collect();
        result.insert(repo.display_name, serde_json::Value::Array(json));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Object(result))?
    );
    Ok(())
}

fn worktree_to_json(wt: &git::WorktreeInfo) -> serde_json::Value {
    serde_json::to_value(wt).expect("WorktreeInfo serialization should not fail")
}
