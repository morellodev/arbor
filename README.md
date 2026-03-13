# arbor

A friendly CLI for managing git worktrees. Arbor keeps all your worktrees organized in a single location (`~/.arbor/worktrees`) so you can switch between branches without stashing, committing, or losing context.

## Install

```sh
cargo install --path .
```

This places the `arbor` binary in `~/.cargo/bin/`.

## Quick start

```sh
# From any git repo, create a worktree for a branch
arbor add feat/login

# Switch to it
cd $(arbor add feat/login)

# See all worktrees
arbor ls

# Check which are dirty or ahead/behind
arbor status

# Done with a branch? Remove the worktree
arbor rm feat/login
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `arbor add <branch>` | | Create a worktree. Checks out an existing local branch, fetches and tracks a remote branch, or creates a new branch. |
| `arbor list [--all]` | `ls` | List worktrees for the current repo. Pass `--all` to list across all repos cloned with `arbor clone`. |
| `arbor remove <branch> [-f]` | `rm` | Remove a worktree. Use `-f` to force removal of dirty worktrees. |
| `arbor dir <branch>` | | Print the filesystem path of the worktree for a branch. |
| `arbor clone <url>` | | Clone a repo as a bare repository into `~/.arbor/repos/`, configured for worktree workflows. |
| `arbor status` | | Show all worktrees with dirty/clean state and ahead/behind counts. |
| `arbor prune` | | Remove stale worktree references. |

## How it works

When you run `arbor add feat/login` inside a repo called `my-app`, arbor creates a worktree at:

```
~/.arbor/worktrees/my-app/feat-login
```

Branch name slashes are replaced with dashes in the directory name.

For a fully worktree-based workflow, use `arbor clone` to create a bare repo, then add worktrees from there:

```sh
arbor clone git@github.com:user/my-app.git
cd ~/.arbor/repos/my-app.git
arbor add main
arbor add feat/login
```

## Configuration

On first run, arbor creates `~/.arbor/config.toml` with default settings:

```toml
worktree_dir = "~/.arbor/worktrees"
repos_dir = "~/.arbor/repos"
```

Edit these to change where worktrees and bare repos are stored.

## Shell tip

Add a helper function to your `.zshrc` or `.bashrc` to combine `add` and `cd` in one step:

```sh
arborcd() {
  cd "$(arbor add "$1")"
}
```

Then just `arborcd feat/login` to create and switch in one go.

## License

MIT
