# CLAUDE.md — elidex project notes

elidex is an experimental browser engine written in Rust.

## Common Commands

```sh
mise run ci          # Run all CI checks locally (lint + test-all + deny)
mise run test        # cargo nextest run --workspace (no doc-tests, fast)
mise run test-all    # nextest + doc-tests (full)
mise run test-doc    # doc-tests only
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
cargo doc --workspace --no-deps  # Build docs
mise run bench                   # Run all benchmarks (CSS, style, layout)
```

## Key Files

- `SECURITY.md` — Vulnerability disclosure policy
- `CONTRIBUTING.md` — Contribution guidelines
- `.github/dependabot.yml` — Automated dependency updates (Cargo + Actions)
- `deny.toml` — License allow-list + supply chain (`unknown-registry`/`unknown-git` = deny)
- `docs/design/ja/29-survey-analysis.md` — JA/EN 900-site compatibility survey analysis (Ch. 29)

## Architecture Notes

Detailed per-crate architecture documentation is in `docs/architecture/`:

| File | Covers |
|------|--------|
| `core.md` | elidex-plugin, elidex-ecs |
| `css.md` | CSS plugin crates (box/text/flex/grid/table/float/anim), elidex-style |
| `dom.md` | elidex-html-parser, elidex-dom-api, elidex-a11y |
| `layout.md` | elidex-layout |
| `script.md` | elidex-script-session, elidex-js-boa, elidex-wasm-runtime |
| `net.md` | elidex-net, elidex-storage-core, elidex-api-sw |
| `shell.md` | elidex-shell, elidex-web-canvas |
| `tools.md` | elidex-crawler, elidex-wpt, benchmarks |

## Development Rules

- **設計優先**: 実装は常に最もクリーンで理想的な設計を目指す。場当たり的な対応・reactive fix 禁止。新しい型を足す前に既存の抽象で解決できないか考える。dead code は接続するか削除する
- **TODO 先送り禁止**: 計画に含めた実装を安易に TODO にしない。実装不可能な場合は理由と対処時期を明示して確認を取る
- **html5ever 非依存**: Layout は html5ever の暗黙的 DOM 構造修正に依存しない（自前 parser 置換予定）。anonymous table object generation 等は layout 側で実装する
- **Git ワークフロー**: main 直接 push 禁止、必ず PR 経由。`gh pr merge --auto` 禁止。CI 全 pass を目視確認してから squash merge
- **コミット前**: `cargo fmt --all` を実行
- **Push 前**: `mise run ci` を実行（fmt + clippy + test + deny）
- **テストは変更クレートに絞る**: `cargo test -p <crate>`。`--workspace` は最終検証時のみ
- **後方互換性は維持しない**: デッドコードや shim は残さず削除

## CI

- 4 jobs: `changes` (path filter), `check` (ubuntu/macos/windows: fmt + clippy + test), `doc` (cargo doc -D warnings), `deny` (standalone).
- Path-based skip: `dorny/paths-filter@v3`. Push to main always runs all jobs.
- Actions pinned: `actions/checkout@v4`, `Swatinem/rust-cache@v2`, `dorny/paths-filter@v3`, `taiki-e/install-action@v2`.
- `rust-toolchain.toml`: `channel = "stable"`.
