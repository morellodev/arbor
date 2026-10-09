mod common;

#[cfg(not(windows))]
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

#[test]
#[cfg(not(windows))]
fn init_inject_detects_eval_without_shell_arg() {
    let env = TestEnv::new();
    let zshrc_path = env.home.path().join(".zshrc");
    fs::write(&zshrc_path, "eval \"$(arbor init)\"\n").unwrap();

    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(output.status.success());

    let zshrc = fs::read_to_string(&zshrc_path).unwrap();
    assert_eq!(
        zshrc.matches("arbor init").count(),
        1,
        "should not add a second line, got: {zshrc}"
    );
}

#[cfg(not(windows))]
fn write_bash_script(env: &TestEnv) -> std::path::PathBuf {
    let script = env.arbor(&["init", "bash"]).output().unwrap();
    let script_path = env.home.path().join("arbor.bash");
    fs::write(&script_path, &script.stdout).unwrap();
    script_path
}

#[test]
#[cfg(not(windows))]
fn init_bash_completion_keeps_flags_for_branch_commands() {
    let env = TestEnv::new();
    let script_path = write_bash_script(&env);

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(
            r#"source "$1"; COMP_WORDS=(arbor add --no); COMP_CWORD=2; _arbor_branches arbor --no add; echo "${COMPREPLY[*]}""#,
        )
        .arg("bash")
        .arg(&script_path)
        .current_dir(env.repo.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--no-hooks"),
        "flag completion should survive branch completion, got: {stdout} / {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Runs `commands` in bash with the wrapper loaded and the test binary on PATH.
#[cfg(not(windows))]
fn run_wrapped(env: &TestEnv, cwd: &std::path::Path, commands: &str) -> std::process::Output {
    let script_path = write_bash_script(env);
    let bin_dir = std::path::Path::new(env!("CARGO_BIN_EXE_arbor"))
        .parent()
        .unwrap();
    let path = format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap());

    std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(r#"source "$1" 2>/dev/null; {commands}"#))
        .arg("bash")
        .arg(&script_path)
        .current_dir(cwd)
        .env("PATH", path)
        .env("HOME", env.home.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", env.home.path().join(".gitconfig"))
        .output()
        .unwrap()
}

#[test]
#[cfg(not(windows))]
fn bash_wrapper_cds_when_global_flag_precedes_subcommand() {
    let env = TestEnv::new();
    let output = run_wrapped(
        &env,
        env.repo.path(),
        "arbor --color never add feat >/dev/null 2>&1; pwd",
    );

    let cwd = String::from_utf8_lossy(&output.stdout);
    assert!(
        cwd.trim_end().ends_with("/feat"),
        "wrapper should cd into the new worktree, ended in: {cwd}"
    );
}

#[test]
#[cfg(not(windows))]
fn bash_wrapper_leaves_removed_worktree_even_when_the_command_fails() {
    let env = TestEnv::new();
    let wt_path = env.add_worktree("unmerged");
    common::git_cmd(
        std::path::Path::new(&wt_path),
        &["commit", "--allow-empty", "-m", "wip"],
        env.home.path(),
    );

    let output = run_wrapped(
        &env,
        std::path::Path::new(&wt_path),
        "arbor rm -d . 2>/dev/null; echo \"$?\"; pwd -P",
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next(),
        Some("1"),
        "the failed branch deletion should be reported, got: {stdout}"
    );
    assert_eq!(
        lines.next().map(std::path::PathBuf::from),
        Some(fs::canonicalize(env.repo.path()).unwrap()),
        "the shell should leave the deleted worktree"
    );
}

#[test]
#[cfg(not(windows))]
fn bash_branch_completion_works_after_global_flags() {
    let env = TestEnv::new();
    env.add_worktree("feat");
    let script_path = write_bash_script(&env);

    for words in [
        "arbor --color never switch ''",
        "arbor --color = never switch ''",
    ] {
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(format!(
                r#"source "$1"; COMP_WORDS=({words}); COMP_CWORD=$((${{#COMP_WORDS[@]}} - 1)); _arbor_branches; echo "${{COMPREPLY[*]}}""#
            ))
            .arg("bash")
            .arg(&script_path)
            .current_dir(env.repo.path())
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.split_whitespace().any(|w| w == "feat"),
            "branches should complete after `{words}`, got: {stdout} / {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[cfg(not(windows))]
fn init_inject_writes_zshrc_under_zdotdir() {
    let env = TestEnv::new();
    let zdotdir = env.home.path().join("zdot");
    fs::create_dir_all(&zdotdir).unwrap();

    let output = env
        .arbor(&["init", "zsh", "--inject"])
        .env("ZDOTDIR", &zdotdir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let zshrc = fs::read_to_string(zdotdir.join(".zshrc")).unwrap();
    assert!(zshrc.contains("arbor init zsh"));
    assert!(!env.home.path().join(".zshrc").exists());
}

#[test]
#[cfg(not(windows))]
fn init_inject_writes_fish_config_under_xdg_config_home() {
    let env = TestEnv::new();
    let xdg = env.home.path().join("xdg");

    let output = env
        .arbor(&["init", "fish", "--inject"])
        .env("XDG_CONFIG_HOME", &xdg)
        .output()
        .unwrap();
    assert!(output.status.success());
    let config = fs::read_to_string(xdg.join("fish/config.fish")).unwrap();
    assert!(config.contains("arbor init fish | source"));
}

#[test]
#[cfg(not(windows))]
fn init_inject_ignores_lines_that_only_mention_arbor_init() {
    let env = TestEnv::new();
    let zshrc_path = env.home.path().join(".zshrc");
    fs::write(&zshrc_path, "alias ai=\"arbor init\"\n").unwrap();

    env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    let zshrc = fs::read_to_string(&zshrc_path).unwrap();
    assert!(
        zshrc.contains("eval \"$(arbor init zsh)\""),
        "an alias is not shell integration, got: {zshrc}"
    );
}

#[test]
#[cfg(not(windows))]
fn init_inject_keeps_non_utf8_config_intact() {
    let env = TestEnv::new();
    let zshrc_path = env.home.path().join(".zshrc");
    fs::write(&zshrc_path, b"export N=\xe9\n").unwrap();

    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let zshrc = fs::read(&zshrc_path).unwrap();
    assert!(zshrc.starts_with(b"export N=\xe9\n"));
    assert!(String::from_utf8_lossy(&zshrc).contains("arbor init zsh"));
}

#[test]
#[cfg(not(windows))]
fn zsh_script_loads_without_compinit() {
    let env = TestEnv::new();
    let script = env.arbor(&["init", "zsh"]).output().unwrap();
    let script_path = env.home.path().join("arbor.zsh");
    fs::write(&script_path, &script.stdout).unwrap();

    let Ok(output) = std::process::Command::new("zsh")
        .args(["-f", "-c", r#"source "$1" && echo loaded"#, "zsh"])
        .arg(&script_path)
        .env("HOME", env.home.path())
        .output()
    else {
        eprintln!("zsh not installed, skipping");
        return;
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("command not found"),
        "a fresh zsh must load the script cleanly, got: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "loaded");
}

#[test]
#[cfg(not(windows))]
fn bash_add_completion_skips_remote_head() {
    let env = TestEnv::new();
    let home = env.home.path();
    let clone = home.join("clone");
    common::git_cmd(
        home,
        &[
            "clone",
            &env.repo.path().to_string_lossy(),
            &clone.to_string_lossy(),
        ],
        home,
    );
    common::git_cmd(&clone, &["remote", "set-head", "origin", "main"], home);
    let script_path = write_bash_script(&env);

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(r#"source "$1"; COMP_WORDS=(arbor add ''); COMP_CWORD=2; _arbor_branches; echo "${COMPREPLY[*]}""#)
        .arg("bash")
        .arg(&script_path)
        .current_dir(&clone)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let words: Vec<&str> = stdout.split_whitespace().collect();
    assert!(words.contains(&"main"), "got: {stdout}");
    assert!(
        !words.contains(&"origin") && !words.contains(&"HEAD"),
        "origin/HEAD is not a branch, got: {stdout}"
    );
}

#[test]
fn init_ignores_a_broken_config() {
    let env = TestEnv::new();
    std::fs::write(
        env.home.path().join(".arbor/config.toml"),
        "worktree_dir = [broken\n",
    )
    .unwrap();

    let output = env.arbor(&["init", "zsh"]).output().unwrap();
    assert!(
        output.status.success(),
        "shell startup must not depend on config.toml, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("arbor()"));
}

#[test]
#[cfg(not(windows))]
fn init_inject_detects_dot_sourcing() {
    let env = TestEnv::new();
    let zshrc_path = env.home.path().join(".zshrc");
    fs::write(&zshrc_path, ". <(arbor init zsh)\n").unwrap();

    let output = env.arbor(&["init", "zsh", "--inject"]).output().unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("already configured"));
}

#[test]
#[cfg(target_os = "macos")]
fn init_inject_bash_covers_login_and_non_login_shells_on_macos() {
    let env = TestEnv::new();
    let home = env.home.path();
    fs::write(home.join(".profile"), "export A=1\n").unwrap();

    let output = env.arbor(&["init", "bash", "--inject"]).output().unwrap();
    assert!(output.status.success());
    for file in [".bashrc", ".profile"] {
        let content = fs::read_to_string(home.join(file)).unwrap();
        assert!(
            content.contains("arbor init bash"),
            "{file} should load arbor, got: {content}"
        );
    }
    assert!(
        !home.join(".bash_profile").exists(),
        "creating ~/.bash_profile would hide ~/.profile"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn init_inject_bash_relies_on_a_login_file_that_sources_bashrc() {
    let env = TestEnv::new();
    let home = env.home.path();
    let profile = "[ -f ~/.bashrc ] && . ~/.bashrc\n";
    fs::write(home.join(".bash_profile"), profile).unwrap();

    env.arbor(&["init", "bash", "--inject"]).output().unwrap();
    assert!(
        fs::read_to_string(home.join(".bashrc"))
            .unwrap()
            .contains("arbor init bash")
    );
    assert_eq!(
        fs::read_to_string(home.join(".bash_profile")).unwrap(),
        profile,
        "the integration must not load twice"
    );

    let again = env.arbor(&["init", "bash", "--inject"]).output().unwrap();
    assert!(String::from_utf8_lossy(&again.stderr).contains("already configured"));
}

#[test]
#[cfg(target_os = "macos")]
fn init_inject_bash_completes_an_existing_bashrc_setup_on_macos() {
    let env = TestEnv::new();
    let home = env.home.path();
    fs::write(home.join(".bashrc"), "eval \"$(arbor init bash)\"\n").unwrap();

    env.arbor(&["init", "bash", "--inject"]).output().unwrap();
    let bashrc = fs::read_to_string(home.join(".bashrc")).unwrap();
    assert_eq!(bashrc.matches("arbor init").count(), 1, "got: {bashrc}");
    assert!(
        fs::read_to_string(home.join(".bash_profile"))
            .unwrap()
            .contains("arbor init bash")
    );
}

#[test]
#[cfg(target_os = "macos")]
fn init_inject_bash_writes_once_when_the_login_file_links_to_bashrc() {
    let env = TestEnv::new();
    let home = env.home.path();
    fs::write(home.join(".bashrc"), "export A=1\n").unwrap();
    std::os::unix::fs::symlink(".bashrc", home.join(".bash_profile")).unwrap();

    env.arbor(&["init", "bash", "--inject"]).output().unwrap();
    let bashrc = fs::read_to_string(home.join(".bashrc")).unwrap();
    assert_eq!(
        bashrc.matches("arbor init bash").count(),
        1,
        "got: {bashrc}"
    );
}

#[test]
#[cfg(not(windows))]
fn zsh_queued_completions_keep_arguments_with_spaces() {
    let env = TestEnv::new();
    let script = env.arbor(&["init", "zsh"]).output().unwrap();
    let script_path = env.home.path().join("arbor.zsh");
    fs::write(&script_path, &script.stdout).unwrap();

    // Calls the first-prompt hook directly, since `zsh -c` never shows a prompt.
    let Ok(output) = std::process::Command::new("zsh")
        .args([
            "-f",
            "-c",
            r#"source "$1"
compdef "_bash_complete -o nospace -C /usr/bin/tf" tf
_arbor_compinit
print -r -- "$_comps[tf]""#,
            "zsh",
        ])
        .arg(&script_path)
        .env("HOME", env.home.path())
        .output()
    else {
        eprintln!("zsh not installed, skipping");
        return;
    };
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "_bash_complete -o nospace -C /usr/bin/tf",
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
