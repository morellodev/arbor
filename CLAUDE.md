# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is arbor?

A CLI tool for managing git worktrees, written in Rust. It organizes worktrees under `~/.arbor/worktrees/` and bare repos under `~/.arbor/repos/`, configured via `~/.arbor/config.toml`.

## Build and test commands

```sh
cargo build                    # Build debug
cargo build --release          # Build release
cargo install --path .         # Install to ~/.cargo/bin/
cargo test                     # Run all tests (unit + integration)
cargo test --lib               # Unit tests only
cargo test --test integration  # Integration tests only
cargo test <test_name>         # Run a single test by name
cargo fmt                      # Format code
cargo clippy -- -D warnings    # Lint (warnings treated as errors)
```

## Pre-commit checks

**Always run `cargo fmt` and `cargo clippy -- -D warnings` after making changes, before committing.** Fix any issues they report before creating a commit.

## Architecture

- **`src/main.rs`** — Entry point. `main()` calls `run()` and formats errors with `display::print_error`. `run()` parses CLI args (clap) and dispatches to command handlers.
- **`src/cli.rs`** — CLI definition using clap derive. Defines `Cli` struct and `Command` enum (Add, Switch, List, Remove, Dir, Clone, Clean, Prune, Status, Fetch, Init).
- **`src/config.rs`** — Loads/creates `~/.arbor/config.toml` with tilde expansion. Uses serde + toml.
- **`src/git/`** — All git operations via `std::process::Command`, split into submodules:
  - `runner.rs` — `run_git`, `run_git_output`, `run_git_inherited` (`pub(super)`)
  - `types.rs` — `ParsedWorktree`, `WorktreeInfo`, `parse_worktree_list`, `sanitize_branch`, `strip_git_suffix` + unit tests
  - `commands.rs` — All pub fn git wrappers (`repo_toplevel`, `worktree_infos`, `resolve_worktree_branch`, `delete_branch`, etc.)
  - `mod.rs` — Re-exports all pub items (callers use `crate::git::*` unchanged)
- **`src/hooks.rs`** — Post-create hook support. Reads `.arbor.toml` from worktree root, executes `post_create` commands via shell with stdout redirected to stderr.
- **`src/display.rs`** — Colored terminal output (using `colored` crate — only file that imports it). Table formatting for worktree listings, summary stats, path shortening (`shorten_path`), terminal-aware path output (`print_path_hint`), and user-facing messages (`print_ok` ✓, `print_error` ✗, `print_note` ▸, `print_section`, `print_heading`, `print_hint`, `print_cd_hint`).
- **`src/commands/`** — One file per subcommand (add, clean, clone, dir, fetch, init, list, prune, remove, status, switch). Each exports a `run` function re-exported from `commands/mod.rs`.

## Key conventions

- Commands print the worktree path to **stdout** (for `cd $(arbor add ...)` workflows) and user messages to **stderr** via the `display` module. Commands that remove the user's cwd (`remove`, `clean`) also print a path to stdout so the shell wrapper can cd out of the deleted directory.
- All user-facing messages (errors, notes, success) start with a capital letter.
- All colored output goes through `src/display.rs` — no other file imports `colored`.
- Interactive prompts use `dialoguer` (currently used by `arbor clean`).
- Branch slashes become dashes in directory names (e.g., `feature/auth` → `feature-auth`).
- Error handling uses `anyhow::Result` throughout.
- Only add comments where the logic isn't self-evident. Do not add comments that restate what the code does.
- Rust edition 2024.
- Commit messages follow [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `chore:`, `ci:`, `docs:`, `refactor:`, `test:`. Use `feat!:` or a `BREAKING CHANGE:` footer for breaking changes. Only `feat:` and `fix:` trigger version bumps — use `ci:` for CI/workflow changes and `chore:` for other non-user-facing changes.

## Releasing

Releases are automated via [release-plz](https://release-plz.ieni.dev/). On each push to `main`, the `prepare-release.yml` workflow opens/updates a release PR with version bump and changelog. Merging that PR tags and triggers `release.yml`, which builds cross-platform binaries, creates a GitHub Release, and updates the Homebrew tap (`morellodev/homebrew-tap`).

## Maintaining this file

Keep CLAUDE.md in sync as the codebase evolves. When adding commands, changing architecture, or updating conventions, update the relevant sections here.

Command and flag completions in `src/commands/init.rs` are auto-generated from `cli.rs` via `clap_complete`, so they stay in sync automatically. Branch completion snippets for `add`, `switch`, `rm`, `remove`, and `dir` are appended as custom shell constants (`BASH_BRANCH_COMPLETIONS`, `ZSH_BRANCH_COMPLETIONS`, `FISH_BRANCH_COMPLETIONS`). The shell wrapper's case list (`add|switch|clone|remove|rm|clean`) must also be updated if a new command prints a path to stdout for cd purposes.

## Integration tests

Tests in `tests/integration.rs` use `assert_cmd` and `tempfile`. The `TestEnv` helper creates an isolated HOME with a custom `config.toml` and a temporary git repo, ensuring tests don't touch the real filesystem.
