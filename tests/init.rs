mod common;

use std::fs;

use common::TestEnv;

#[test]
fn init_zsh_outputs_shell_function() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "zsh"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("arbor()"),
        "zsh init should define an arbor function, got: {stdout}"
    );
    assert!(
        stdout.contains("compdef"),
        "zsh init should include completions, got: {stdout}"
    );
}

#[test]
fn init_bash_outputs_shell_function() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "bash"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("arbor()"),
        "bash init should define an arbor function, got: {stdout}"
    );
    assert!(
        stdout.contains("complete -F"),
        "bash init should include completions, got: {stdout}"
    );
}

#[test]
fn init_fish_outputs_shell_function() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "fish"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function arbor"),
        "fish init should define an arbor function, got: {stdout}"
    );
}

#[test]
fn init_unsupported_shell_fails() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "nushell"]).output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported shell"),
        "should reject unsupported shell, got: {stderr}"
    );
}

// ── --inject ────────────────────────────────────────────────────────

#[test]
#[cfg(not(windows))]
fn init_inject_adds_line() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(
        output.status.success(),
        "init --inject should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Added shell integration"),
        "should confirm injection, got: {stderr}"
    );

    let zshrc = fs::read_to_string(env.home.path().join(".zshrc")).unwrap();
    assert!(
        zshrc.contains("arbor init zsh"),
        "zshrc should contain eval line, got: {zshrc}"
    );
    assert!(
        zshrc.contains("# arbor"),
        "zshrc should contain comment marker, got: {zshrc}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_idempotent() {
    let env = TestEnv::new();

    env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    let second = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(second.status.success());

    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("already configured"),
        "second run should say already configured, got: {stderr}"
    );

    let zshrc = fs::read_to_string(env.home.path().join(".zshrc")).unwrap();
    let count = zshrc.matches("arbor init zsh").count();
    assert_eq!(
        count, 1,
        "eval line should appear exactly once, got: {count}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_creates_fish_dirs() {
    let env = TestEnv::new();
    let output = env.arbor(&["init", "fish", "--inject"]).output().unwrap();
    assert!(
        output.status.success(),
        "init fish --inject should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = env.home.path().join(".config/fish/config.fish");
    assert!(config_path.exists(), "should create fish config file");

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("arbor init fish | source"),
        "fish config should contain source line, got: {content}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_already_configured() {
    let env = TestEnv::new();

    let zshrc_path = env.home.path().join(".zshrc");
    fs::write(
        &zshrc_path,
        "# existing config\neval \"$(arbor init zsh)\"\n",
    )
    .unwrap();

    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already configured"),
        "should detect existing config, got: {stderr}"
    );

    let zshrc = fs::read_to_string(&zshrc_path).unwrap();
    let count = zshrc.matches("arbor init zsh").count();
    assert_eq!(count, 1, "should not duplicate the line, got: {count}");
}
