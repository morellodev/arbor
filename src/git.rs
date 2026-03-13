use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let output = run_git_output(args, cwd)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_output(args: &[&str], cwd: Option<&Path>) -> Result<std::process::Output> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to run: git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(output)
}

fn run_git_inherited(args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to run: git {}", args.join(" ")))?;

    if !status.success() {
        bail!("git {} exited with status {}", args.join(" "), status);
    }

    Ok(())
}

pub fn repo_toplevel() -> Result<PathBuf> {
    let porcelain = run_git(&["worktree", "list", "--porcelain"], None)
        .context("not inside a git repository")?;
    let first_line = porcelain
        .lines()
        .next()
        .context("empty worktree list output")?;
    let path = first_line
        .strip_prefix("worktree ")
        .context("unexpected worktree list format")?;
    Ok(PathBuf::from(path))
}

pub fn strip_git_suffix(name: &str) -> &str {
    name.strip_suffix(".git").unwrap_or(name)
}

pub fn repo_name_or_unknown() -> String {
    repo_name().unwrap_or_else(|_| "unknown".to_string())
}

pub fn repo_name() -> Result<String> {
    let toplevel = repo_toplevel()?;
    let name = toplevel
        .file_name()
        .context("repository path has no final component")?
        .to_string_lossy()
        .into_owned();
    Ok(strip_git_suffix(&name).to_string())
}

pub fn local_branch_exists(branch: &str, cwd: Option<&Path>) -> Result<bool> {
    let refspec = format!("refs/heads/{branch}");
    Ok(run_git(&["show-ref", "--verify", "--quiet", &refspec], cwd).is_ok())
}

pub fn remote_branch_exists(branch: &str, cwd: Option<&Path>) -> Result<bool> {
    let refspec = format!("refs/remotes/origin/{branch}");
    Ok(run_git(&["show-ref", "--verify", "--quiet", &refspec], cwd).is_ok())
}

pub fn worktree_add_existing(path: &Path, branch: &str, cwd: Option<&Path>) -> Result<()> {
    run_git(&["worktree", "add", &path.to_string_lossy(), branch], cwd)?;
    Ok(())
}

pub fn worktree_add_new_branch(path: &Path, branch: &str, cwd: Option<&Path>) -> Result<()> {
    run_git(
        &["worktree", "add", "-b", branch, &path.to_string_lossy()],
        cwd,
    )?;
    Ok(())
}

pub fn create_tracking_branch(branch: &str, cwd: Option<&Path>) -> Result<()> {
    let remote_ref = format!("origin/{branch}");
    run_git(&["branch", "--track", branch, &remote_ref], cwd)?;
    Ok(())
}

pub fn worktree_list_porcelain(cwd: Option<&Path>) -> Result<String> {
    run_git(&["worktree", "list", "--porcelain"], cwd)
}

pub fn worktree_remove(path: &Path, force: bool) -> Result<()> {
    let path_str = path.to_string_lossy();
    if force {
        run_git_inherited(&["worktree", "remove", "--force", &path_str], None)
    } else {
        run_git_inherited(&["worktree", "remove", &path_str], None)
    }
}

pub fn worktree_prune() -> Result<String> {
    let output = run_git_output(&["worktree", "prune", "--verbose"], None)?;
    Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
}

pub fn clone_bare(url: &str, dest: &Path) -> Result<()> {
    run_git_inherited(&["clone", "--bare", url, &dest.to_string_lossy()], None)
}

pub fn configure_bare_fetch(repo_path: &Path) -> Result<()> {
    run_git(
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
        Some(repo_path),
    )?;
    Ok(())
}

pub fn fetch_origin(repo_path: &Path) -> Result<()> {
    run_git_inherited(&["fetch", "origin"], Some(repo_path))
}

pub fn status_porcelain(cwd: &Path) -> Result<String> {
    run_git(&["status", "--porcelain"], Some(cwd))
}

pub fn ahead_behind(cwd: &Path) -> Option<(usize, usize)> {
    let output = run_git(
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        Some(cwd),
    )
    .ok()?;

    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() == 2 {
        let behind = parts[0].parse().ok()?;
        let ahead = parts[1].parse().ok()?;
        Some((ahead, behind))
    } else {
        None
    }
}

pub fn head_branch(repo_path: &Path) -> Result<String> {
    let output = run_git(&["symbolic-ref", "HEAD"], Some(repo_path))?;
    Ok(output
        .strip_prefix("refs/heads/")
        .unwrap_or(&output)
        .to_string())
}

pub fn delete_branch(branch: &str, cwd: Option<&Path>) -> Result<()> {
    run_git(&["branch", "-d", branch], cwd)?;
    Ok(())
}

pub fn is_worktree_dirty(path: &Path) -> bool {
    status_porcelain(path)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn sanitize_branch(branch: &str) -> String {
    branch.replace('/', "-")
}

pub struct ParsedWorktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub bare: bool,
}

pub fn parse_worktree_list(porcelain: &str) -> Vec<ParsedWorktree> {
    let mut results = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut current_bare = false;

    for line in porcelain.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(p) = current_path.take() {
                results.push(ParsedWorktree {
                    path: p,
                    branch: current_branch.take(),
                    bare: current_bare,
                });
            }
            current_path = Some(PathBuf::from(path));
            current_branch = None;
            current_bare = false;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = Some(
                branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string(),
            );
        } else if line == "bare" {
            current_bare = true;
        }
    }

    if let Some(p) = current_path {
        results.push(ParsedWorktree {
            path: p,
            branch: current_branch,
            bare: current_bare,
        });
    }

    results
}

#[derive(Serialize)]
pub struct WorktreeInfo {
    #[serde(serialize_with = "serialize_path")]
    pub path: PathBuf,
    pub branch: Option<String>,
    pub dirty: bool,
    #[serde(serialize_with = "serialize_tracking")]
    pub tracking: Option<(usize, usize)>,
}

fn serialize_path<S: serde::Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.display().to_string())
}

fn serialize_tracking<S: serde::Serializer>(
    tracking: &Option<(usize, usize)>,
    s: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    match tracking {
        Some((ahead, behind)) => {
            let mut map = s.serialize_map(Some(2))?;
            map.serialize_entry("ahead", ahead)?;
            map.serialize_entry("behind", behind)?;
            map.end()
        }
        None => s.serialize_none(),
    }
}

pub fn worktree_infos(cwd: Option<&Path>) -> Result<Vec<WorktreeInfo>> {
    let porcelain = worktree_list_porcelain(cwd)?;
    let entries = parse_worktree_list(&porcelain);

    let mut results = Vec::new();
    for entry in entries {
        if entry.bare {
            continue;
        }

        let tracking = ahead_behind(&entry.path);
        let dirty = is_worktree_dirty(&entry.path);
        results.push(WorktreeInfo {
            path: entry.path,
            branch: entry.branch,
            dirty,
            tracking,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_worktree() {
        let input = "\
worktree /home/user/project
HEAD abc1234
branch refs/heads/main
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(result[0].branch.as_deref(), Some("main"));
        assert!(!result[0].bare);
    }

    #[test]
    fn parse_multiple_worktrees() {
        let input = "\
worktree /home/user/project
HEAD abc1234
branch refs/heads/main

worktree /home/user/.arbor/worktrees/project/feature-auth
HEAD def5678
branch refs/heads/feature/auth
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].branch.as_deref(), Some("main"));
        assert_eq!(
            result[1].path,
            PathBuf::from("/home/user/.arbor/worktrees/project/feature-auth")
        );
        assert_eq!(result[1].branch.as_deref(), Some("feature/auth"));
    }

    #[test]
    fn parse_empty_input() {
        let result = parse_worktree_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_mixed_bare_normal_and_detached() {
        let input = "\
worktree /home/user/.arbor/repos/project.git
HEAD 0000000000000000000000000000000000000000
bare

worktree /home/user/.arbor/worktrees/project/main
HEAD abc1234abc1234abc1234abc1234abc1234abc12
branch refs/heads/main

worktree /home/user/.arbor/worktrees/project/hotfix
HEAD def5678def5678def5678def5678def5678def56
detached
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 3);

        assert_eq!(
            result[0].path,
            PathBuf::from("/home/user/.arbor/repos/project.git")
        );
        assert_eq!(result[0].branch, None);
        assert!(result[0].bare);

        assert_eq!(result[1].branch.as_deref(), Some("main"));
        assert!(!result[1].bare);

        assert_eq!(
            result[2].path,
            PathBuf::from("/home/user/.arbor/worktrees/project/hotfix")
        );
        assert_eq!(result[2].branch, None);
        assert!(!result[2].bare);
    }

    #[test]
    fn parse_worktree_path_with_spaces() {
        let input = "\
worktree /Users/jane doe/My Projects/cool app
HEAD abc1234
branch refs/heads/develop
";
        let result = parse_worktree_list(input);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].path,
            PathBuf::from("/Users/jane doe/My Projects/cool app")
        );
        assert_eq!(result[0].branch.as_deref(), Some("develop"));
    }
}
