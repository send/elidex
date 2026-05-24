# CLAUDE.md — elidex

Experimental browser engine written in Rust.

## Development Rules

### Design philosophy

- **ECS-native first**: elidex は **既存にない novel な ECS-native browser engine**。OO browser engine (Blink / Gecko / WebKit) の inheritance + virtual dispatch + observer registry pattern を ideal として直訳しない。ECS first principles (component / system / query / marker / event queue / version counter) から設計を導く。OO pattern を翻訳する時は idiom 対応表を明示 (例: observer registry → marker component + query, subscriber list → component query, class-owned state → ECS component on entity)。**ECS-native であること自体が elidex の design contribution**。
  - **Side-store→component 判定ルール**: per-entity 状態を entity-keyed の side-store/registry (HostData の `*_cache` / `*Registry` / `HashMap<entity, _>`) に持たせる時、その値が `Send + Sync` **かつ per-VM identity handle でない**なら **ECS component on the entity が正**（SameObject = component get、GC = 単一 query、despawn = 自動 cleanup）。boa `HostBridge` side-store を VM へ移植する時の直訳が典型的な罠（D-21 R3 で CRIT 化）。`Send+Sync` は hecs 適格条件であって「すべき」とは別 — side-store が正当な例外は2つ:
    - **(a) per-VM identity handle (一時的例外)**: VM `ObjectId` (JS wrapper / callback) を保持するもの。値は Send (`ObjectId(u32)`) だが意味が per-VM — `EcsDom` は VM 間で entity-index 空間を共有し rebind され得るので、component 化すると **cross-DOM aliasing** (前 DOM の wrapper を拾う、`vm_api.rs` `Vm::unbind` の cache-clear 参照: コメント `cross-DOM references and must be cleared on unbind`)。`world_id` discriminator (`#11-wrapper-cache-cross-dom-discriminator`) が解禁条件で、それまで per-VM HostData (unbind-clear) が正。world_id 後は component 化可 (`#11-wrapper-identity-component-migration`)。
    - **(b) shared cross-cutting state (恒久的例外)**: cookie jar / `NetworkHandle` 等、単一 entity の事実でない browsing-context/session レベル資源 (per-entity でないので component 対象外、Send+Sync でも該当しない)。
    詳細 → `memory/ecs-native-side-store-audit-2026-05-21.md` / `memory/feedback_boa-hostbridge-port-is-not-a-registry.md`。
- **Ideal over pragmatic**: 設計判断は「最もクリーンで spec-faithful + future-extensible な architecture」が default。"現実解" "minimal v1" "stub で済ます" "session 短縮のため scope 削る" は基本選ばない。defer cap / session 残見込は judgment 補助情報であって設計選択の制約条件にしない。User が明示的に "session 節約優先" "stub OK" "scope 制約あり" と言った時のみ pragmatic 路線。

### Design discipline

- **設計優先**: 場当たり的な reactive fix 禁止。新しい型を足す前に既存の抽象で解決できないか考える。dead code は接続するか削除
- **One issue, one way**: より良い機構を見つけたら、その種の処理は **単一の正準形に一括収束** (full unification) させる。「新 seam + N 個の legacy 実装」が共存する strangler 中間状態を残さない — 共存は「どっちで書く?/これは migration 対象?」という決定 tax を**消さず移すだけ**で、PR ごと・reader ごとに再発する。混沌度 (= 決定の表面積) を下げること自体が目的で、それは upfront の大きめ refactor に値する。Ideal over pragmatic の decision-surface 版。詳細 → `memory/feedback_one-issue-one-way.md`
- **TODO 先送り禁止**: 計画に含めた実装を安易に TODO にしない。実装不可能なら理由 + 対処時期を明示して確認を取る
- **後方互換性は維持しない**: デッドコードや shim は残さず削除
- **html5ever 非依存**: Layout は html5ever の暗黙的 DOM 構造修正に依存しない（anonymous table object generation 等は layout 側で実装、自前 parser 置換予定）

### Layering mandate (2026-05-04 incident 由来)

**VM host/ は engine-bound 責務のみ**: `crates/script/elidex-js/src/vm/host/` は prototype install / brand check / JsValue↔Entity marshalling に限定する。DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等の algorithm は engine-independent crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) 経由で呼ぶ。`EcsDom::*` の direct call は marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) に限定する。新規 algorithm を host/ に書く前に、対応する engine-independent crate に既存実装が無いか確認 + 無ければそちらを拡張。詳細 → `memory/m4-12-architectural-drift-incident.md`

### Spec citation

WHATWG / W3C の section number / anchor / WebIDL 確認は `.claude/tools/webref` (w3c/webref machine-readable extracts) を使う。WebFetch 経由の spec HTML は length truncate で citation 整合確認には不向き — 構造化 fact (number/title/anchor/IDL) は webref、algorithm の prose 自然文は WebFetch (spec 直) で使い分け。「§X.Y.Z = <name>」と書く時の number ↔ title pair は **必ず lookup** (well-known 風 cite を信用しない、D-17 で `§4.13.4 = upgrade queue` 系の drift が landing 後に発覚)。recipe → `.claude/skills/elidex-review/axes.md` § "Axis 4 — Verification recipe (webref)"

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
