use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub(super) fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let output = run_git_output(args, cwd)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn run_git_output(args: &[&str], cwd: Option<&Path>) -> Result<std::process::Output> {
    run_git_output_with_env(args, cwd, &[])
}

pub(super) fn run_git_output_with_env(
    args: &[&str],
    cwd: Option<&Path>,
    env: &[(&str, &OsStr)],
) -> Result<std::process::Output> {
    let mut cmd = Command::new("git");
    cmd.args(args).envs(env.iter().copied());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .with_context(|| format!("Failed to run: git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(output)
}

pub(super) fn run_git_with_input(args: &[&str], cwd: Option<&Path>, input: &str) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to run: git {}", args.join(" ")))?;
    child
        .stdin
        .take()
        .context("Failed to open git stdin")?
        .write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(())
}

pub(super) fn run_git_inherited(args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let status = cmd
        .status()
        .with_context(|| format!("Failed to run: git {}", args.join(" ")))?;

    if !status.success() {
        bail!("Git {} failed ({status})", args.join(" "));
    }

    Ok(())
}
