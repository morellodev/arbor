mod common;

use std::fs;
use std::path::Path;

use common::{TestEnv, commit_arbor_toml, stdout_canon, stdout_path};

#[test]
fn add_with_local_worktree_dir_relative() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let printed_path = stdout_path(&output);
    let expected = env.repo.path().join(".worktrees/feat");
    let actual = fs::canonicalize(&printed_path).unwrap();
    let expected_canon = fs::canonicalize(&expected).unwrap();
    assert_eq!(
        actual,
        expected_canon,
        "worktree should be at {}, got {}",
        expected.display(),
        printed_path
    );
}

#[test]
fn add_with_local_worktree_dir_absolute() {
    let env = TestEnv::new();
    let abs_dir = env.home.path().join("custom-wt");
    let abs_str = abs_dir.to_string_lossy().replace('\\', "/");
    commit_arbor_toml(&env, &format!("worktree_dir = \"{abs_str}\"\n"));

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(
        output.status.success(),
        "add should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let printed_path = stdout_path(&output);
    let actual = fs::canonicalize(&printed_path).unwrap();
    let expected = fs::canonicalize(abs_dir.join("feat")).unwrap();
    assert_eq!(
        actual, expected,
        "worktree should be at absolute path, got {}",
        printed_path
    );
}

#[test]
fn add_with_local_worktree_dir_omits_repo_name() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let output = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(output.status.success());

    let printed_path = stdout_path(&output);

    // Path should be <repo>/.worktrees/feat, NOT <repo>/.worktrees/<repo_name>/feat
    assert!(
        Path::new(&printed_path).ends_with(Path::new(".worktrees").join("feat")),
        "path should end with .worktrees/feat (no repo name subdir), got: {printed_path}"
    );
}

#[test]
fn add_with_local_worktree_dir_is_idempotent() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let first = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(first.status.success());

    let second = env.arbor(&["add", "feat"]).output().unwrap();
    assert!(second.status.success());

    assert_eq!(
        stdout_path(&first),
        stdout_path(&second),
        "should return the same path both times"
    );
}

#[test]
fn add_with_local_worktree_dir_from_secondary_worktree() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let wt_a = env.add_worktree("feat-a");

    let second = env
        .arbor_in(Path::new(&wt_a), &["add", "feat-b"])
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "add from secondary worktree should succeed, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let wt_b = stdout_path(&second);

    // Both should be under the same override dir
    let parent_a = Path::new(&wt_a).parent().unwrap();
    let parent_b = Path::new(&wt_b).parent().unwrap();
    assert_eq!(
        fs::canonicalize(parent_a).unwrap(),
        fs::canonicalize(parent_b).unwrap(),
        "both worktrees should be under the same override dir"
    );
}

#[test]
fn remove_works_with_local_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let wt_path = env.add_worktree("feat");

    let rm_out = env.arbor(&["remove", "feat"]).output().unwrap();
    assert!(
        rm_out.status.success(),
        "remove should succeed, stderr: {}",
        String::from_utf8_lossy(&rm_out.stderr)
    );
    assert!(
        !Path::new(&wt_path).exists(),
        "worktree directory should be gone after remove"
    );
}

#[test]
fn switch_works_with_local_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let add_path = fs::canonicalize(env.add_worktree("feat")).unwrap();

    let switch_out = env.arbor(&["switch", "feat"]).output().unwrap();
    assert!(switch_out.status.success());

    assert_eq!(add_path, stdout_canon(&switch_out));
}

#[test]
fn dir_works_with_local_worktree_dir() {
    let env = TestEnv::new();
    commit_arbor_toml(&env, "worktree_dir = \".worktrees\"\n");

    let add_path = fs::canonicalize(env.add_worktree("feat")).unwrap();

    let dir_out = env.arbor(&["dir", "feat"]).output().unwrap();
    assert!(dir_out.status.success());

    assert_eq!(add_path, stdout_canon(&dir_out));
}
