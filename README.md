# elidex

An experimental browser engine written in Rust.

> **Status: Phase 4 (Rendering Completeness, Web Platform API, JS Independence)**
> Phases 0–3.5 are complete. Phase 4 focuses on rendering completeness,
> plugin architecture, and CSS animations. Crate APIs are unstable and subject
> to breaking changes.

## Project Structure

```
crates/
  core/
    elidex-plugin/          Plugin traits, registry, CSS property handler system
    elidex-plugin-macros/   Procedural macros for plugin system
    elidex-ecs/             ECS-based DOM storage
    elidex-render/          Rendering backend (Vello + wgpu)
  css/
    elidex-css/             CSS parser, value types, selector engine
    elidex-style/           Style resolution (cascade, inheritance)
    elidex-css-box/         Box model CSS property handler plugin
    elidex-css-text/        Text/font CSS property handler plugin
    elidex-css-flex/        Flexbox CSS property handler plugin
    elidex-css-grid/        Grid CSS property handler plugin
    elidex-css-table/       Table CSS property handler plugin
    elidex-css-float/       Float/clear/visibility property handler plugin
    elidex-css-anim/        CSS Animations & Transitions plugin
  dom/
    elidex-html-parser/     HTML/XML parser (html5ever, charset detection)
    elidex-dom-api/         DOM API handler implementations
    elidex-dom-compat/      Legacy/compat DOM layer
    elidex-a11y/            Accessibility tree builder (AccessKit)
  layout/
    elidex-layout/          Layout orchestrator (block, inline, dispatch)
    elidex-layout-block/    Block layout
    elidex-layout-flex/     Flexbox layout
    elidex-layout-grid/     Grid layout
    elidex-layout-table/    Table layout
  text/
    elidex-text/            Text facade (shaping + linebreak + bidi)
    elidex-shaping/         Text shaping (rustybuzz)
    elidex-linebreak/       Line breaking (UAX #14)
    elidex-bidi/            BiDi algorithm (UAX #9)
  script/
    elidex-script-session/  Script session (JS ↔ ECS DOM bridge)
    elidex-js/              JavaScript parser (ES2020+)
    elidex-js-boa/          Boa JS engine integration
    elidex-wasm-runtime/    WebAssembly runtime (wasmtime)
  net/
    elidex-net/             HTTP network stack (hyper, TLS, cookies)
  web/
    elidex-web-canvas/      Canvas 2D API (tiny-skia)
  shell/
    elidex-shell/           Window management, event loop, browser chrome
    elidex-navigation/      Navigation controller (history, load pipeline)
  tools/
    elidex-crawler/         Web compatibility survey tool
    elidex-wpt/             WPT-style CSS conformance test harness
docs/
  design/                   Design documents (en/ja)
```

## Development

### Prerequisites

- Rust stable toolchain (MSRV: 1.88)
- [mise](https://mise.jdx.dev/) (task runner)
- [cargo-nextest](https://nexte.st/) (`cargo install cargo-nextest --locked`) — used by `mise run test` / `test-all` / `ci`
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
  --sites crates/tools/elidex-crawler/data/sites-en.csv \
  --sites crates/tools/elidex-crawler/data/sites-ja.csv \
  --output output/

# Analyze previous crawl results
cargo run -p elidex-crawler -- analyze --input output/results.json
```

## Roadmap

- **Phase 0** (complete): Project scaffolding, plugin traits, ECS DOM prototype, web compatibility survey
- **Phase 1** (complete): HTML parser, CSS engine, style resolution
- **Phase 2** (complete): Layout engine, rendering pipeline, JavaScript integration
- **Phase 3** (complete): Selectors, images, layout enhancement, integration
- **Phase 3.5** (complete): Grid, table, shadow DOM, WASM, multi-process, tabs
- **Phase 4** (current): Rendering completeness, plugin architecture, CSS animations

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
