# Contributing to elidex

Thanks for your interest in contributing!

## Getting Started

1. Fork the repository and clone your fork.
2. Install prerequisites: Rust stable toolchain, [mise](https://mise.jdx.dev/),
   and [cargo-deny](https://github.com/EmbarkStudios/cargo-deny).
3. Run `mise run ci` to verify everything works locally.

## Development Workflow

1. Create a feature branch from `main`.
2. Make your changes with clear, focused commits.
3. Add tests for new functionality.
4. Ensure `mise run ci` passes (formatting, clippy, tests, license checks).
5. Open a pull request against `main`.

## Code Style

- Run `cargo fmt --all` before committing.
- Fix all clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- Follow existing patterns in the codebase.

## Commit Messages

- Use imperative mood ("Add feature", not "Added feature").
- Keep the first line under 72 characters.
- Reference relevant issues where applicable.

## CI

Pull requests are tested on Ubuntu, macOS, and Windows. CI checks:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps` (with `-D warnings`)
- `cargo deny check` (licenses and vulnerabilities)
- MSRV compatibility (Rust 1.88)
