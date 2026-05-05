# CLAUDE.md — elidex

Experimental browser engine written in Rust.

## Development Rules

### Design discipline

- **設計優先**: 場当たり的な reactive fix 禁止。新しい型を足す前に既存の抽象で解決できないか考える。dead code は接続するか削除
- **TODO 先送り禁止**: 計画に含めた実装を安易に TODO にしない。実装不可能なら理由 + 対処時期を明示して確認を取る
- **後方互換性は維持しない**: デッドコードや shim は残さず削除
- **html5ever 非依存**: Layout は html5ever の暗黙的 DOM 構造修正に依存しない（anonymous table object generation 等は layout 側で実装、自前 parser 置換予定）

### Layering mandate (2026-05-04 incident 由来)

**VM host/ は engine-bound 責務のみ**: `crates/script/elidex-js/src/vm/host/` は prototype install / brand check / JsValue↔Entity marshalling に限定する。DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等の algorithm は engine-independent crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) 経由で呼ぶ。`EcsDom::*` の direct call は marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) に限定する。新規 algorithm を host/ に書く前に、対応する engine-independent crate に既存実装が無いか確認 + 無ければそちらを拡張。詳細 → `memory/m4-12-architectural-drift-incident.md`

### Workflow

- **コミット前**: `cargo fmt --all`
- **Push 前**: `mise run ci`（check + lint + test-all + doc + deny）。cargo を呼ぶ task (`check` / `test` / `test-doc` / `test-all` / `lint-clippy` / `doc`) は `--all-features` で gate されているので feature-gated code (`#![cfg(feature = "engine")]` 等) も含めて回る (`lint-fmt` / `deny` は feature と無関係のため対象外)
- **テストは変更クレートに絞る**: `cargo test -p <crate> --all-features`。`--workspace` / `mise run test` は最終検証時のみ
- **Git**: main 直接 push 禁止、PR 経由必須。`gh pr merge --auto` 禁止。CI 全 pass を目視確認してから squash merge

## Commands

```sh
mise run ci          # 全 CI (push 前必須)
mise run test        # cargo nextest run --workspace --all-features (fast, no doc-tests)
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
```

その他: `test-all` (+ doc-tests) / `test-doc` / `doc` (RUSTDOCFLAGS=-D warnings) / `bench` (CSS / style / layout)

## Architecture

詳細は `docs/architecture/`:

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

## Key Files

- `SECURITY.md` / `CONTRIBUTING.md` — security / contribution policy
- `deny.toml` — license allow-list + supply chain (`unknown-registry`/`unknown-git` = deny)
- `.github/dependabot.yml` — automated dependency updates (Cargo + Actions)
- `docs/design/ja/29-survey-analysis.md` — JA/EN 900-site compatibility survey

## CI

4 jobs: `changes` (path filter via `dorny/paths-filter@v4` — includes `.github/workflows/**` so workflow-only edits trigger the gate) / `check` (ubuntu/macos/windows: fmt check + clippy `--all-features` + nextest `--all-features` + doc-tests `--all-features`; cargo-nextest installed via `taiki-e/install-action@v2`) / `doc` (cargo doc `--all-features` -D warnings) / `deny` (standalone). Push to main always runs all jobs. Actions pinned (`actions/checkout@v6`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`). Toolchain stable (`rust-toolchain.toml`).
