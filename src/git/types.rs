use std::path::{Path, PathBuf};

use serde::Serialize;

pub fn sanitize_branch(branch: &str) -> String {
    branch.replace('/', "-")
}

pub fn strip_git_suffix(name: &str) -> &str {
    name.strip_suffix(".git").unwrap_or(name)
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
