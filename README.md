# 🌳 arbor

A friendly CLI for managing git worktrees. Arbor keeps all your worktrees organized in a single location (`~/.arbor/worktrees`) so you can switch between branches without stashing, committing, or losing context.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.85+
- [Git](https://git-scm.com/)

## Install

```sh
git clone https://github.com/morellodev/arbor.git
cd arbor
cargo install --path .
```

This places the `arbor` binary in `~/.cargo/bin/`. Make sure it's on your `PATH` (it is by default with a standard Rust installation).

To uninstall:

```sh
cargo uninstall arbor
```

## Quick start

```sh
# Clone a repo for worktree-based workflows (auto-creates a worktree for the default branch)
arbor clone user/my-app

# Create a worktree for a branch
arbor add feat/login

# See all worktrees
arbor ls

# Check which are dirty or ahead/behind
arbor status

# Done with a branch? Remove the worktree and its local branch
arbor rm -d feat/login
```

## Shell integration

For the best experience, add shell integration so `arbor add` automatically `cd`s into the new worktree:

```sh
# Add to your ~/.zshrc or ~/.bashrc:
eval "$(arbor init zsh)"    # or bash

# Fish users — add to ~/.config/fish/config.fish:
arbor init fish | source
```

Then `arbor add feat/login` creates the worktree **and** switches to it in one step.

## Commands

| Command | Alias | Description |
| --- | --- | --- |
| `arbor add <branch> [--repo <name>]` | | Create a worktree. Checks out an existing local branch, tracks a remote branch, or creates a new one. Use `--repo` to add from any directory. |
| `arbor list [--all] [--json]` | `ls` | List worktrees for the current repo. `--all` lists across all repos. `--json` outputs machine-readable JSON. |
| `arbor remove <branch> [-f] [-d]` | `rm` | Remove a worktree. `-f` forces removal of dirty worktrees. `-d` also deletes the local branch. |
| `arbor dir <branch>` | | Print the worktree path for a branch. Accepts both `feature/auth` and `feature-auth`. |
| `arbor clone <url> [--no-worktree]` | | Clone a repo as a bare repository and create a worktree for the default branch. Use `--no-worktree` to skip. Supports `user/repo` shorthand for GitHub. |
| `arbor status [--short]` | | Show all worktrees with dirty/clean state and ahead/behind counts. `--short` for compact output. |
| `arbor fetch` | | Fetch from origin in the current bare repo. |
| `arbor prune` | | Remove stale worktree references. |
| `arbor init <shell>` | | Print shell integration snippet (bash, zsh, fish). |
| `arbor completions <shell>` | | Generate shell completions (bash, zsh, fish, elvish, powershell). |

## How it works

When you run `arbor add feat/login` inside a repo called `my-app`, arbor creates a worktree at:

```text
~/.arbor/worktrees/my-app/feat-login
```

Slashes in branch names become dashes in the directory name.

For a fully worktree-based workflow, use `arbor clone` to set up a bare repo:

```sh
arbor clone user/my-app
# Automatically creates a worktree for the default branch and prints the path
arbor add feat/login
```

## Configuration

On first run, arbor creates `~/.arbor/config.toml`:

```toml
worktree_dir = "~/.arbor/worktrees"
repos_dir = "~/.arbor/repos"
```

Edit these to change where worktrees and bare repos are stored.

## License

[MIT](LICENSE)
