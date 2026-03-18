# 🌳 arbor

[![Test & Lint](https://github.com/morellodev/arbor/actions/workflows/ci.yml/badge.svg)](https://github.com/morellodev/arbor/actions/workflows/ci.yml)
[![GitHub Release](https://img.shields.io/github/v/release/morellodev/arbor)](https://github.com/morellodev/arbor/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A CLI for managing git worktrees. It keeps all your worktrees under `~/.arbor/worktrees` so you can switch between branches without stashing or losing context.

![demo](demo.gif)

## Why arbor?

Git worktrees are great: multiple branches checked out at once, no stashing, no half-finished commits. The problem is that `git worktree add` makes you pick a directory every time, and you end up with worktrees scattered everywhere.

Arbor puts them all in one place. `arbor add feat/login` creates the worktree, and with shell integration it `cd`s you into it too. `arbor rm feat/login` cleans it up.

## Who is this for?

If you review PRs while working on your own feature, juggle hotfixes alongside long-running branches, or run tests in one worktree while coding in another — arbor keeps everything organized.

## Install

### Homebrew (macOS and Linux)

```sh
brew install morellodev/tap/arbor
```

### Pre-built binaries (macOS, Linux, Windows)

Download the latest binary for your platform from the [releases page](https://github.com/morellodev/arbor/releases/latest).

### From source

```sh
git clone https://github.com/morellodev/arbor.git
cd arbor
cargo install --path .
```

You'll need [Rust](https://www.rust-lang.org/tools/install) 1.85+ and [Git](https://git-scm.com/).

## Quick start

```sh
# Clone a repo (creates a bare repo + worktree for the default branch)
arbor clone user/my-app

# Create a worktree for a branch
arbor add feat/login

# Switch to an existing worktree
arbor switch main

# Or pick one interactively
arbor switch

# See all worktrees
arbor ls

# Done? Remove the worktree and its local branch
arbor rm -d feat/login
```

## Shell integration

Run `arbor init` to set up shell integration. It detects your shell, shows what it will add and where, and offers to do it for you:

```sh
arbor init
# ▸ This will add the following to ~/.zshrc:
#
#   # arbor
#   eval "$(arbor init zsh)"
#
# ? Add it now? [Y/n]
```

This sets up two things: a wrapper so `arbor add`, `arbor switch` (or `arbor cd`), and `arbor clone` automatically `cd` into the worktree, and dynamic tab completions for branch names.

The shell is auto-detected from `$SHELL`. You can also specify it explicitly:

```sh
arbor init zsh    # or bash, fish
```

For scripted or dotfile-bootstrap workflows, use `--inject` to skip the prompt:

```sh
arbor init --inject
```

## Commands

| Command | Alias | Description |
| --- | --- | --- |
| `arbor add <branch> [-b <base>] [--no-hooks]` | | Create a worktree. Checks out an existing local branch, tracks a remote branch, or creates a new one. `-b` / `--base` starts the new branch from a specific ref (branch, tag, or commit). `--no-hooks` skips post-create hooks. |
| `arbor switch [branch]` | `cd` | Switch to an existing worktree. With no argument, shows an interactive fuzzy selector. |
| `arbor list [--all] [--json] [--short]` | `ls` | List worktrees for the current repo. `--all` lists across all repos. `--json` for machine-readable output. `--short` hides the path column. |
| `arbor remove [branch] [-f] [-d]` | `rm` | Remove a worktree. With no argument, shows an interactive fuzzy selector. Use `.` to remove the current worktree. `-f` forces removal of dirty worktrees. `-d` also deletes the local branch. |
| `arbor dir [branch]` | | Print the worktree path for a branch. With no argument, shows an interactive fuzzy selector. Accepts both `feature/auth` and `feature-auth`. |
| `arbor clone <url> [--no-worktree] [--no-hooks]` | | Clone as a bare repo and create a worktree for the default branch. Supports `user/repo` shorthand for GitHub. `--no-hooks` skips post-create hooks. |
| `arbor fetch [--all]` | | Fetch from origin in the current bare repo. `--all` fetches across all repos. |
| `arbor clean [-d]` | | Interactively select and remove unused worktrees. `-d` also deletes local branches. |
| `arbor prune` | | Remove stale worktree references. |
| `arbor init [shell] [--inject]` | | Set up shell integration (cd wrapper + completions). Auto-detects shell from `$SHELL`. `--inject` writes to your shell config non-interactively. |

## How it works

`arbor add feat/login` inside a repo called `my-app` creates a worktree at:

```text
~/.arbor/worktrees/my-app/feat-login
```

Slashes in branch names become dashes in the directory name.

For a worktree-only workflow, start with `arbor clone` to set up a bare repo:

```sh
arbor clone user/my-app
arbor add feat/login
```

## Configuration

On first run, arbor creates `~/.arbor/config.toml`:

```toml
repos_dir = "~/.arbor/repos"
worktree_dir = "~/.arbor/worktrees"
```

Change these to store worktrees and bare repos somewhere else. You can also override the worktree directory per-project — see [Per-project worktree directory](#per-project-worktree-directory) below.

## Per-project worktree directory

By default, arbor puts all worktrees under `~/.arbor/worktrees`. You can override
this per-project by adding `worktree_dir` to your `.arbor.toml`:

```toml
worktree_dir = ".claude/worktrees"
```

This creates worktrees at `<project-root>/.claude/worktrees/<branch>` instead
of the global default. Useful for keeping worktrees colocated with the project
(e.g. for Claude Code parallel sessions).

**Path resolution:**

| Value | Resolves to |
| --- | --- |
| `.claude/worktrees` | `<repo-root>/.claude/worktrees/<branch>` |
| `~/my-worktrees` | `$HOME/my-worktrees/<branch>` |
| `/tmp/worktrees` | `/tmp/worktrees/<branch>` |

Relative paths are resolved from the repository root. When using the local
override, there is no `<repo-name>` subdirectory — the config is already
scoped to one project.

## Hooks

You can run commands automatically after a worktree is created by adding a `.arbor.toml` file to your repo root:

```toml
[hooks]
post_create = "npm install"
```

Multiple commands are supported:

```toml
[hooks]
post_create = ["npm install", "cp .env.example .env"]
```

Hooks run inside the new worktree directory with these environment variables available:

| Variable | Description |
| --- | --- |
| `ARBOR_WORKTREE` | Absolute path to the new worktree |
| `ARBOR_BRANCH` | Branch name |
| `ARBOR_REPO` | Repository name |
| `ARBOR_EVENT` | Hook event name (`post_create`) |

Hook output streams to stderr so it doesn't interfere with `cd $(arbor add ...)` piping. If a hook fails, arbor prints a warning and continues — the worktree is still created.

To skip hooks for a single invocation, pass `--no-hooks`:

```sh
arbor add feat/login --no-hooks
```

## Alternatives

If arbor isn't what you're after, here are some other tools in this space:

| | arbor | `git worktree` (bare) | [git-branchless](https://github.com/arxanas/git-branchless) | [git-town](https://github.com/git-town/git-town) |
| --- | --- | --- | --- | --- |
| Worktree management | Yes | Yes (manual) | Yes | No |
| Central worktree directory | Yes | No (you pick each time) | No | N/A |
| Auto-cd into worktree | Yes (shell integration) | No | No | No |
| GitHub shorthand clone | Yes (`user/repo`) | No | No | No |
| Status across worktrees | Yes | No | Yes | No |
| Stacked diffs / rebase workflows | No | No | Yes | Yes |
| Branch sync with remote | No | No | Yes | Yes |

arbor is intentionally narrow. It manages worktrees and gets out of your way. If you need stacked diffs or branch sync workflows, git-branchless or git-town are better fits.

## License

[MIT](LICENSE)
