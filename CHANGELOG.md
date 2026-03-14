# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.1.3](https://github.com/morellodev/arbor/compare/v0.1.2...v0.1.3) - 2026-03-14

### Added

- auto-cd and dynamic completions for shell init
- add switch command and --all flag for status and fetch
- fix repo detection from worktrees and improve CLI feedback

### Fixed

- resolve branch names in remove and force-delete with -f -d
- use stable toolchain in release workflow
- make integration tests work on Windows
- make integration tests work on Windows

### Other

- allow manual trigger of prepare-release workflow
- revert to PAT for release tagging
- use GITHUB_TOKEN for release tagging
- release v0.1.2
- bump GitHub Actions to Node 22 versions
- add pre-built binaries install section to README
- add integration tests for new features
- add Windows binary to release pipeline
- add usage examples to help text, update README and CLAUDE.md
- extract shared helpers and clean up display
- split git module into submodules
- only trigger release PRs for user-facing commits
- fix release pipeline and rename CI workflow
- add release tagging step to prepare-release workflow
- release v0.1.1
- clarify commit type conventions for version bumps
- pin Rust toolchain to 1.94.0
- apply rustfmt
- add CI, release automation, Homebrew tap, and project docs
- Fix all clippy warnings
- Reduce duplication and improve clarity across codebase
- Remove change preview from status and simplify WorktreeInfo
- Use comfy-table for dynamic column sizing and simplify display code
- Sort config keys alphabetically
- Update README with new commands and shell integration
- Improve UX across all commands
- Add CLAUDE.md with project guidance for Claude Code
- Add colored output and extract display module
- Support "user/repo" shorthand in clone command (defaults to GitHub)
- Update README clone URL to match new repo name
- Add MIT license and rename package from arbor-cli to arbor
- Add prerequisites and expanded install instructions to README
- Add README with install, usage, and configuration docs
- Fix dir command to query git worktree list instead of guessing paths
- Rename cd command to dir for clarity
- Add visible aliases: list -> ls, remove -> rm
- Improve add command output for interactive vs piped usage
- Add unit and integration tests, fix two bugs found by testing
- Initial implementation of arbor CLI

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
