# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.2.1](https://github.com/morellodev/arbor/compare/v0.2.0...v0.2.1) - 2026-03-15

### Added

- auto-generate shell completions with clap_complete
- add aggregate summaries for --all batch operations
- use icons and compact notation in worktree tables
- add helpful suggestions to dir and remove error messages
- respect NO_COLOR env var and add --color global flag
- add `arbor clean` command and auto-cd out of removed worktrees

### Fixed

- skip remove-from-inside-worktree tests on Windows
- align columns in arbor clean worktree list
- enable ANSI-aware column alignment in tables

### Other

- reuse print_note in fetch summary and single-pass batch aggregation
- require fmt and clippy checks before every commit
- fix clippy print_with_newline warnings
- update CLAUDE.md for auto-generated completions
- update transitive dependencies
- reorder config fields to match struct declaration
- prevent duplicate release PR on release merge

## [0.2.0](https://github.com/morellodev/arbor/compare/v0.1.2...v0.2.0) - 2026-03-14

### Added

- [**breaking**] improve shell setup UX and consolidate display helpers

## [0.1.2](https://github.com/morellodev/arbor/compare/v0.1.1...v0.1.2) - 2026-03-14

### Added

- auto-cd and dynamic completions for shell init
- add switch command and --all flag for status and fetch

### Fixed

- resolve branch names in remove and force-delete with -f -d

### Other

- bump GitHub Actions to Node 22 versions
- add pre-built binaries install section to README
- add integration tests for new features
- add Windows binary to release pipeline
- add usage examples to help text, update README and CLAUDE.md
- extract shared helpers and clean up display
- split git module into submodules

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
