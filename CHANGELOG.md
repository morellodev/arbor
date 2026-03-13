# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.1.2](https://github.com/morellodev/arbor/compare/v0.1.1...v0.1.2) - 2026-03-13

### Other

- fix release pipeline and rename CI workflow

## [0.1.1](https://github.com/morellodev/arbor/compare/v0.1.0...v0.1.1) - 2026-03-13

### Added

- fix repo detection from worktrees and improve CLI feedback

### Other

- clarify commit type conventions for version bumps

## [0.1.0] - 2026-03-13

### Added

- `arbor clone` — clone a repo as a bare repository with GitHub shorthand support (`user/repo`)
- `arbor add` — create worktrees for existing, remote, or new branches
- `arbor list` — list worktrees with table or JSON output
- `arbor remove` — remove worktrees with optional branch deletion
- `arbor status` — show dirty/clean state and ahead/behind counts
- `arbor dir` — print worktree path for a branch
- `arbor fetch` — fetch from origin
- `arbor prune` — remove stale worktree references
- `arbor init` — shell integration for bash, zsh, and fish
- `arbor completions` — generate shell completions
- Configuration via `~/.arbor/config.toml`
