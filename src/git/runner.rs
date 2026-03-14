use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub(super) fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let output = run_git_output(args, cwd)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn run_git_output(args: &[&str], cwd: Option<&Path>) -> Result<std::process::Output> {
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

pub(super) fn run_git_inherited(args: &[&str], cwd: Option<&Path>) -> Result<()> {
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
