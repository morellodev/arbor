# Contributing to arbor

Thanks for your interest in contributing! Here's how to get started.

## Development setup

```sh
git clone https://github.com/morellodev/arbor.git
cd arbor
cargo build
```

## Running tests

```sh
cargo test                     # All tests (unit + integration)
cargo test --lib               # Unit tests only
cargo test --test integration  # Integration tests only
```

## Commit messages

This project uses [conventional commits](https://www.conventionalcommits.org/). The commit type determines the version bump at release time:

- `fix: ...` — patch release (0.1.0 → 0.1.1)
- `feat: ...` — minor release (0.1.0 → 0.2.0)
- `feat!: ...` or `BREAKING CHANGE:` in the footer — major release (0.1.0 → 1.0.0)
- `chore:`, `docs:`, `refactor:`, `test:` — no release

## Submitting changes

1. Fork the repo and create a branch from `main`.
2. Make your changes. Add tests if applicable.
3. Run `cargo fmt` and `cargo clippy` before committing.
4. Open a pull request with a clear description of what you changed and why.

## Releasing

Releases are fully automated via [release-plz](https://release-plz.ieni.dev/). When commits land on `main`, release-plz opens (or updates) a release PR that bumps the version in Cargo.toml and updates CHANGELOG.md. Merging that PR creates a git tag, which triggers cross-platform builds, a GitHub Release, and a Homebrew tap update. No manual steps needed.

## Reporting bugs

Open an [issue](https://github.com/morellodev/arbor/issues/new?template=bug_report.md) with steps to reproduce, expected behavior, and your OS/git version.

## Suggesting features

Open an [issue](https://github.com/morellodev/arbor/issues/new?template=feature_request.md) describing the use case and proposed solution.
