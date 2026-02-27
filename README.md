# elidex

An experimental browser engine written in Rust.

> **Status: Phase 1 (HTML/CSS parsing & style resolution)**
> Phase 0 (foundation) is complete. Phase 1 focuses on building the HTML parser,
> CSS engine, and style resolution pipeline. Crate APIs are unstable and subject
> to breaking changes.

## Project Structure

```
crates/
  elidex-plugin/         Plugin system traits and registry
  elidex-plugin-macros/  Procedural macros for plugin system
  elidex-ecs/            ECS-based DOM storage
  elidex-crawler/        Web compatibility survey tool
  elidex-parser/         HTML/XML parser
  elidex-css/            CSS parser, value types, selector engine
  elidex-style/          Style resolution (cascade, inheritance)
  elidex-layout/         Layout algorithms (block, inline, flexbox)
  elidex-text/           Text shaping, measurement, line breaking
  elidex-render/         Rendering backend abstraction
  elidex-shell/          Window management and event loop
docs/
  design/                Design documents (en/ja)
```

## Development

### Prerequisites

- Rust stable toolchain (MSRV: 1.88)
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

- **Phase 0** (complete): Project scaffolding, plugin traits, ECS DOM prototype, web compatibility survey
- **Phase 1** (current): HTML parser, CSS engine, style resolution
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
