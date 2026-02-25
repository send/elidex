# elidex

An experimental browser engine written in Rust.

> **Status: Phase 0 (Foundation)**
> This project is in its earliest phase. The crate APIs are unstable and subject to
> breaking changes. Phase 0 focuses on project scaffolding, the plugin framework,
> an ECS-based DOM prototype, and a web compatibility survey.

## Project Structure

```
crates/
  elidex-plugin/    Plugin system traits and registry
  elidex-ecs/       ECS-based DOM storage
  elidex-crawler/   Web compatibility survey tool
docs/
  design/           Design documents (en/ja)
```

## Development

### Prerequisites

- Rust stable toolchain (MSRV: 1.75)
- [mise](https://mise.jdx.dev/) (task runner)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) (license/vulnerability checks)

### Common Tasks

```sh
mise run check     # cargo check
mise run test      # run all tests
mise run lint      # clippy + fmt check
mise run fmt       # format code
mise run deny      # license/vulnerability check
mise run ci        # run all CI checks locally
```

### Running the Crawler

```sh
# Crawl sites listed in CSV files
cargo run -p elidex-crawler -- crawl \
  --sites crates/elidex-crawler/data/sites-en.csv \
  --sites crates/elidex-crawler/data/sites-ja.csv \
  --output output/

# Analyze previous crawl results
cargo run -p elidex-crawler -- analyze --input output/results.json
```

## Roadmap

- **Phase 0** (current): Project scaffolding, plugin traits, ECS DOM prototype, web compatibility survey
- **Phase 1**: HTML parser, CSS engine, concrete plugin implementations
- **Phase 2**: Layout engine, rendering pipeline
- **Phase 3**: JavaScript integration, Web API bindings

## Security

If you discover a security vulnerability, please report it privately via
GitHub's [security advisory feature](https://github.com/send-sh/elidex/security/advisories/new).
Do not open a public issue for security vulnerabilities.

## Contributing

Contributions are welcome! Please:

1. Fork the repository and create a feature branch.
2. Ensure `mise run ci` passes locally before opening a PR.
3. Keep commits focused and write clear commit messages.
4. Add tests for new functionality.

CI runs on every pull request (Ubuntu, macOS, Windows) and checks formatting,
clippy lints, tests, and license compliance.

## License

MIT
