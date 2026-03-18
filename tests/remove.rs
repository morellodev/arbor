mod common;

use std::fs;
use std::path::Path;

use common::{TestEnv, git_cmd, stdout_canon};
use tempfile::TempDir;

#[test]
fn remove_deletes_worktree() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let rm_out = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(rm_out.status.success());
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after remove"
    );
}

#[test]
fn remove_nonexistent_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["remove", "nonexistent"]).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No worktree found"),
        "expected error message, got: {stderr}"
    );
}

#[test]
fn remove_with_delete_branch_resolves_sanitized_name() {
    let env = TestEnv::new();
    env.add_worktree("feature/auth");

    // Remove using the sanitized name with -d to also delete the branch
    let rm_out = env
        .arbor(&["remove", "feature-auth", "-d"])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("Deleted branch 'feature/auth'"),
        "should delete the actual branch name, got: {stderr}"
    );
}

#[test]
fn remove_force_delete_unmerged_branch() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("unmerged");

    // Make a commit in the worktree so the branch diverges
    git_cmd(
        Path::new(&wt_path),
        &["commit", "--allow-empty", "-m", "wip"],
        env.home.path(),
    );

    // Force-remove the worktree and delete the unmerged branch
    let rm_out = env
        .arbor(&["remove", "unmerged", "-f", "-d"])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("Deleted branch"),
        "should force-delete the branch, got: {stderr}"
    );
    assert!(
        stderr.contains("(was "),
        "should print commit hash for force-deleted branch, got: {stderr}"
    );
}

#[test]
fn remove_delete_branch_prints_commit_hash() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let rm_out = env.arbor(&["remove", "feat", "-d"]).output().unwrap();
    assert!(rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("(was "),
        "should print commit hash when deleting branch, got: {stderr}"
    );
}

#[test]
fn remove_dirty_worktree_rejected() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    // Create an untracked file to make the worktree dirty
    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let rm_out = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(!rm_out.status.success());

    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "should reject dirty worktree, got: {stderr}"
    );
}

#[test]
fn remove_alias_rm_works() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["rm", "feat"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
#[cfg(not(windows))]
fn remove_from_inside_worktree_prints_toplevel() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", "feat"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );

    assert_eq!(
        stdout_canon(&rm_out),
        fs::canonicalize(env.repo.path()).unwrap(),
        "should print the repo toplevel path"
    );
}

#[test]
#[cfg(not(windows))]
fn remove_from_worktree_subdirectory_prints_toplevel() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let subdir = Path::new(&wt_path).join("deep/nested");
    fs::create_dir_all(&subdir).unwrap();

    let rm_out = env
        .arbor_in(&subdir, &["rm", "feat", "-f"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );

    assert_eq!(
        stdout_canon(&rm_out),
        fs::canonicalize(env.repo.path()).unwrap(),
        "should print the repo toplevel path"
    );
}

#[test]
fn remove_from_outside_worktree_prints_nothing_to_stdout() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let rm_out = env.arbor(&["rm", "feat"]).output().unwrap();
    assert!(rm_out.status.success());

    let stdout = String::from_utf8_lossy(&rm_out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "should print nothing to stdout, got: {stdout}"
    );
}

#[test]
fn remove_from_different_worktree_prints_nothing_to_stdout() {
    let env = TestEnv::new();
    let wt_a = env.add_worktree("feat-a");
    env.add_worktree("feat-b");

    let rm_out = env
        .arbor_in(Path::new(&wt_a), &["rm", "feat-b"])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let stdout = String::from_utf8_lossy(&rm_out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "should print nothing to stdout when removing a different worktree, got: {stdout}"
    );
}

// ── interactive / dot ───────────────────────────────────────────────

#[test]
fn remove_no_arg_requires_terminal() {
    let env = TestEnv::new();
    env.add_worktree("feat");

    let output = env.arbor(&["rm"]).output().unwrap();
    assert!(
        !output.status.success(),
        "rm with no arg and piped stdin should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Interactive terminal required"),
        "should mention terminal requirement, got: {stderr}"
    );
}

#[test]
fn remove_dot_removes_current_worktree() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", "."])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm . should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after rm ."
    );

    assert_eq!(
        stdout_canon(&rm_out),
        fs::canonicalize(env.repo.path()).unwrap(),
        "should print the repo toplevel path"
    );
}

#[test]
fn remove_dot_outside_worktree_fails() {
    let env = TestEnv::new();
    let outside = TempDir::new().unwrap();
    let output = env.arbor_in(outside.path(), &["rm", "."]).output().unwrap();
    assert!(
        !output.status.success(),
        "rm . outside a git repo should fail"
    );
}

#[test]
fn remove_dot_with_delete_branch() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", ".", "-d"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm . -d should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("Deleted branch"),
        "should mention deleted branch, got: {stderr}"
    );
}

#[test]
fn remove_dot_from_subdirectory() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    let subdir = Path::new(&wt_path).join("nested/deep");
    fs::create_dir_all(&subdir).unwrap();

    let rm_out = env.arbor_in(&subdir, &["rm", "."]).output().unwrap();
    assert!(
        rm_out.status.success(),
        "rm . from subdirectory should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone"
    );
}

#[test]
fn remove_dot_dirty_worktree_fails() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", "."])
        .output()
        .unwrap();
    assert!(
        !rm_out.status.success(),
        "rm . on dirty worktree should fail"
    );
    let stderr = String::from_utf8_lossy(&rm_out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "should mention uncommitted changes, got: {stderr}"
    );
}

#[test]
fn remove_dot_force_dirty_worktree() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("feat");

    fs::write(Path::new(&wt_path).join("dirty.txt"), "dirty").unwrap();

    let rm_out = env
        .arbor_in(Path::new(&wt_path), &["rm", ".", "-f"])
        .output()
        .unwrap();
    assert!(
        rm_out.status.success(),
        "rm . -f on dirty worktree should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after forced removal"
    );
}
