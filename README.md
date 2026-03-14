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

### From source

```sh
git clone https://github.com/morellodev/arbor.git
cd arbor
cargo install --path .
```

You'll need [Rust](https://www.rust-lang.org/tools/install) 1.85+ and [Git](https://git-scm.com/).

Pre-built binaries are also available on the [releases page](https://github.com/morellodev/arbor/releases/latest).

## Quick start

```sh
# Clone a repo (creates a bare repo + worktree for the default branch)
arbor clone user/my-app

# Create a worktree for a branch
arbor add feat/login

# Switch to an existing worktree
arbor switch main

# See all worktrees
arbor ls

# Check which are dirty or ahead/behind
arbor status

# Done? Remove the worktree and its local branch
arbor rm -d feat/login
```

## Shell integration

Add this so `arbor add`, `arbor switch`, and `arbor clone` automatically `cd` into the worktree:

```sh
# ~/.zshrc or ~/.bashrc
eval "$(arbor init zsh)"    # or bash

# Fish: ~/.config/fish/config.fish
arbor init fish | source
```

Without this, arbor prints the worktree path but won't change your directory. The shell integration also provides dynamic tab completions for branch names.

## Commands

| Command | Alias | Description |
| --- | --- | --- |
| `arbor add <branch> [--repo <name>]` | | Create a worktree. Checks out an existing local branch, tracks a remote branch, or creates a new one. `--repo` lets you add from any directory. |
| `arbor switch <branch>` | | Switch to an existing worktree. Errors if the worktree doesn't exist. |
| `arbor list [--all] [--json]` | `ls` | List worktrees for the current repo. `--all` lists across all repos. `--json` for machine-readable output. |
| `arbor remove <branch> [-f] [-d]` | `rm` | Remove a worktree. `-f` forces removal of dirty worktrees. `-d` also deletes the local branch. |
| `arbor dir <branch>` | | Print the worktree path for a branch. Accepts both `feature/auth` and `feature-auth`. |
| `arbor clone <url> [--no-worktree]` | | Clone as a bare repo and create a worktree for the default branch. Supports `user/repo` shorthand for GitHub. |
| `arbor status [--short] [--all]` | | Show dirty/clean state and ahead/behind counts for all worktrees. `--all` shows across all repos. |
| `arbor fetch [--all]` | | Fetch from origin in the current bare repo. `--all` fetches across all repos. |
| `arbor prune` | | Remove stale worktree references. |
| `arbor init <shell>` | | Print shell integration snippet (bash, zsh, fish). |
| `arbor completions <shell>` | | Generate shell completions (bash, zsh, fish, elvish, powershell). |

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

## Shell completions

Generate and install completions so you get tab completion for commands and options:

```sh
# Zsh (most common on macOS)
arbor completions zsh > ~/.zsh/completions/_arbor

# Bash
arbor completions bash > ~/.local/share/bash-completion/completions/arbor

# Fish
arbor completions fish > ~/.config/fish/completions/arbor.fish
```

You may need to restart your shell or run `compinit` (zsh) for completions to take effect.

## Configuration

On first run, arbor creates `~/.arbor/config.toml`:

```toml
repos_dir = "~/.arbor/repos"
worktree_dir = "~/.arbor/worktrees"
```

Change these to store worktrees and bare repos somewhere else.

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
