# CLAUDE.md — elidex

Experimental browser engine written in Rust.

## Development Rules

### Design philosophy

- **ECS-native first**: elidex は **既存にない novel な ECS-native browser engine**。OO browser engine (Blink / Gecko / WebKit) の inheritance + virtual dispatch + observer registry pattern を ideal として直訳しない。ECS first principles (component / system / query / marker / event queue / version counter) から設計を導く。OO pattern を翻訳する時は idiom 対応表を明示 (例: observer registry → marker component + query, subscriber list → component query, class-owned state → ECS component on entity)。**ECS-native であること自体が elidex の design contribution**。
  - **Side-store→component 判定ルール**: per-entity 状態を entity-keyed の side-store/registry (HostData の `*_cache` / `*Registry` / `HashMap<entity, _>`) に持たせる時、その値が `Send + Sync` **かつ per-VM identity handle でない**なら **ECS component on the entity が正**（SameObject = component get、GC = 単一 query、despawn = 自動 cleanup）。boa `HostBridge` side-store を VM へ移植する時の直訳が典型的な罠（D-21 R3 で CRIT 化）。`Send+Sync` は hecs 適格条件であって「すべき」とは別 — side-store が正当な例外は2つ:
    - **(a) per-VM identity handle (一時的例外)**: VM `ObjectId` (JS wrapper / callback) を保持するもの。値は Send (`ObjectId(u32)`) だが意味が per-VM — `EcsDom` は VM 間で entity-index 空間を共有し rebind され得るので、component 化すると **cross-DOM aliasing** (前 DOM の wrapper を拾う、`vm_api.rs` `Vm::unbind` の cache-clear 参照: コメント `cross-DOM references and must be cleared on unbind`)。`world_id` discriminator (`#11-wrapper-cache-cross-dom-discriminator`) が解禁条件で、それまで per-VM HostData (unbind-clear) が正。world_id 後は component 化可 (`#11-wrapper-identity-component-migration`)。
    - **(b) shared cross-cutting state (恒久的例外)**: cookie jar / `NetworkHandle` 等、単一 entity の事実でない browsing-context/session レベル資源 (per-entity でないので component 対象外、Send+Sync でも該当しない)。
    詳細 → `memory/ecs-native-side-store-audit-2026-05-21.md` / `memory/feedback_boa-hostbridge-port-is-not-a-registry.md`。
- **Whole-engine core/compat/deprecated consistency**: `docs/design/ja/01-executive-summary.md` / `07-plugin-system.md` / `13-script-session.md` の SSoT。HTML / DOM API / CSSOM / ECMAScript / Web API / parser recovery は同じ三層 pattern (clean core + optional compat + deprecated/removal) に従う。Legacy / non-standard / blocking / sync / quirks / Annex B / document.write / live collection / localStorage / XHR / document.cookie 等を core に混ぜない。Core に置くなら「modern standard baseline として必要」な理由、compat に置くなら modern equivalent への shim/normalization boundary を明示する。
- **Plugin-first extensibility**: elidex の拡張点は原則 `elidex-plugin` 型の同一 mental model (static enum dispatch for built-ins + dynamic trait object for runtime extension) に収束させる。HTML tag / CSS property / layout algorithm / network middleware / DOM/CSSOM/Web API handler / JS language feature を個別 registry / ad hoc hook / hard-coded branch で増やす前に、対応する plugin trait・SpecLevel・feature gate に載せる。Hot path built-in は static dispatch、user/experimental/policy は dynamic extension が正準形。
- **ScriptSession as the sole Script↔ECS boundary**: DOM / CSSOM / future OM (Selection, Range, Performance 等) と JS/Wasm は `ScriptSession` の Identity Map / Mutation Buffer / flush / GC coordination / live query 管理を共有する。OM ごとの ad hoc wrapper cache、直接 ECS mutation、host-local live query registry を増やさない。書き込みは session mutation と flush point に集約し、SameObject・MutationObserver・atomic script-task visibility を同一機構で守る。
- **Concurrency by ownership and phases**: renderer main thread は ECS DOM の唯一の owner/writer。Script mutation は `ScriptSession::flush`、style/layout/paint は event-loop phase ごとの明示 window、compositor は原則 owner-transfer された DisplayList / Layer data を読む。Lock 共有で辻褄を合わせる前に、ownership transfer・single writer・double buffer・FrameSource boundary で競合を構造的に消す。Thread pool は重複させず、CPU 並列 work は共有 rayon pool、I/O readiness は runtime/reactor 境界に寄せる。
- **Security by structure, not review convention**: security boundary は後付けしない。Renderer は direct network/file/storage/permission access を持たず Browser/Network/Storage broker 経由、web-content storage は origin 物理分離、browser-owned state は Browser Process 集中管理が原則。Policy をコアに焼かず mechanism を提供する領域 (例: content blocking) と、secure-by-default を engine が強制する領域 (HTTPS-only, third-party cookie blocking, permission gate, quota/eviction) を区別する。
- **Ideal over pragmatic**: 設計判断は「最もクリーンで spec-faithful + future-extensible な architecture」が default。"現実解" "minimal v1" "stub で済ます" "session 短縮のため scope 削る" は基本選ばない。defer cap / session 残見込は judgment 補助情報であって設計選択の制約条件にしない。User が明示的に "session 節約優先" "stub OK" "scope 制約あり" と言った時のみ pragmatic 路線。

### Design discipline

- **設計優先**: 場当たり的な reactive fix 禁止。新しい型を足す前に既存の抽象で解決できないか考える。dead code は接続するか削除
- **One issue, one way**: より良い機構を見つけたら、その種の処理は **単一の正準形に一括収束** (full unification) させる。「新 seam + N 個の legacy 実装」が共存する strangler 中間状態を残さない — 共存は「どっちで書く?/これは migration 対象?」という決定 tax を**消さず移すだけ**で、PR ごと・reader ごとに再発する。混沌度 (= 決定の表面積) を下げること自体が目的で、それは upfront の大きめ refactor に値する。Ideal over pragmatic の decision-surface 版。詳細 → `memory/feedback_one-issue-one-way.md`
- **Phase design is not a shortcut license**: design doc の Phase 1–3 / Phase 4+ は「長期簡素化を妨げない境界を先に置く」ための段階的緩和であって、temporary hack の免罪符ではない。外部 runtime / process boundary / transport / storage backend 等を暫定採用する時は、`IpcChannel` / `AsyncRuntime` / `FrameSource` / `StorageBackend` / `HttpTransport` のように、後続の統合・置換が設定変更または局所差し替えになる trait boundary を置く。存在しない代替のための抽象化は ADR #26 (Vello) のように明示例外化する。
- **Supported-surface testing**: elidex は WPT 100% を目標にせず、plugin / feature が宣言した supported subset の regression を防ぐ。新しい standard/plugin surface は担当 WPT subset・engine-independent unit/integration coverage・必要なら visual/fuzz/benchmark のどれで守るかを明示する。Compat は best-effort でも、core supported surface の新規失敗は原則 block。
- **TODO 先送り禁止**: 計画に含めた実装を安易に TODO にしない。実装不可能なら理由 + 対処時期を明示して確認を取る
- **後方互換性は維持しない**: デッドコードや shim は残さず削除
- **Edge-dense work = multi-PR program + 実装前 plan-review 必須**: ≥3 intersecting invariant axes を束ねる、または正準アルゴリズムが無い subsystem を触る work は (a) **単一 PR に束ねない** — umbrella plan + PR ごとの plan に分割し各 PR を個別に full review、(b) **各 PR は実装前に `/elidex-plan-review` 必須** (judgment でなく rule)、(c) **base case (再帰の終端)** = 承認済 umbrella 配下で plan-review を通った narrowly-scoped per-PR slice は terminal 単位 = 許容される単一 PR (per-PR slice が同 subsystem を触ること自体は再分割 trigger にしない — さもなくば各 slice もまた単一 PR 禁止と無限後退する)。edge-matrix を upfront map して review tail を pre-empt する。#339 (S2) が単一 PR + plan-review skip で実装 ~1 commit に対し review tail 30+ commit を払った incident 由来 → `memory/feedback_edge-dense-mandatory-plan-review-and-split.md` / `memory/feedback_cross-round-structural-root-check.md`
- **html5ever 非依存 (layout decoupling)**: Layout は html5ever の暗黙的 DOM 構造修正に依存しない（anonymous table object generation 等は HTML §17 に従い layout 側で実装）= parser 選択に依らない layout 層責務。〔parser backend: strict 解析 = 自前 `elidex-html-parser-strict` / tolerant (recovery) = html5ever / charset 検出 = 自前 (`charset.rs` + encoding_rs)。配備モデルは design doc §11.3「段階的デグラデーション」(Tier-1 strict-first → Tier-2 tolerant fallback) で確定済 = elidex 最初からの core 思想 (「browser=tolerant」も「配備未確定」も elidex の立場でない)。open は auto-dispatch 実装 + 勾配実測のみ → `memory/project_parser-strict-tolerant-deployment.md`〕

### Layering mandate (2026-05-04 incident 由来)

**VM host/ は engine-bound 責務のみ**: `crates/script/elidex-js/src/vm/host/` は prototype install / brand check / JsValue↔Entity marshalling に限定する。DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等の algorithm は engine-independent crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) 経由で呼ぶ。`EcsDom::*` の direct call は marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) に限定する。新規 algorithm を host/ に書く前に、対応する engine-independent crate に既存実装が無いか確認 + 無ければそちらを拡張。詳細 → `memory/m4-12-architectural-drift-incident.md`

**Core vs compat split (`docs/design/ja/14-script-engines-webapi.md` §14.1 由来)**: elidex-js **core** = ES2020+ strict-mode-only baseline (modern JavaScript)。**LegacySemantics compat plugin** 領域 = sloppy mode (型変換 / this auto-boxing) / Annex B (JS内HTMLコメント / __proto__ accessor / RegExp legacy / 文字列HTMLメソッド) / sloppy direct-eval (caller-env scope injection) / var hoisting quirks / `arguments.callee` `.caller` / `with` 文 等。VM internal design slot を立てる時、その motivation が plugin 領域 (= 上記 legacy 機能) なら **core slot 化しない** + compat plugin 着手時の concern に hand-off。`FrameKind` / opcode / dispatch path 等を direct/indirect eval 区別 / var hoisting / sloppy mode flag 等で拡張する案は core scope 外。VM impl 側も `vm/dispatch.rs:595` で "All code is strict, so we always throw" と明示。詳細 → `memory/reference_elidex-js-core-strict-only.md`

### Spec citation

WHATWG / W3C / TC39 (ECMA-262 / ECMA-402) / **CSS WG modules** の section number / anchor / WebIDL / AO (aoid) / **algorithm prose** 確認は `.claude/tools/webref` を使う (subcommand 詳細 = `.claude/tools/webref --help`)。Data source 自動切替: WHATWG/W3C は w3c/webref machine-readable extracts、tc39 は `@tc39/<spec>-biblio` (jsdelivr CDN 経由)、algorithm prose は spec multipage chapter HTML (HTTP cache 共有)。**WebFetch にフォールバックしない** — 仕様確認は原則 webref で完結する。

webref の cache refresh / snapshot / semantic diff / agent-brief workflow の設計方針は `.claude/tools/_webref/DESIGN.md` を参照（通常の citation lookup では不要、spec drift 対応時のみ読む）。

**CSS module も webref 対象** (drafts.csswg.org 系、`css-text-3` / `selectors-4` / `css-values-4` / `css-overflow-3` 等)。`css <module-shortname> <name>` で property / `@rule` / selector / value metadata (value grammar / initial / inherited / appliesTo / computedValue / animationType / href)、`body <module-shortname> <anchor>` で §番号付き prose (truncate なし、例 `body css-text-3 line-break-transform` = §4.1.3 Segment Break Transformation Rules)。第1引数は **level 込みの module shortname 必須** (`css-text-3` であって `css-text` でない)、不明時は `specs <keyword>` で逆引き。CSS の citation 確認で WebFetch を使ったら誤用 (Copilot の `css-text-3 §4 #segment-break` 指摘に対し Fetch していた件が発端)。

「§X.Y.Z = <name>」と書く時の number ↔ title pair は **必ず lookup** (well-known 風 cite を信用しない、D-17 で `§4.13.4 = upgrade queue` 系の drift が landing 後に発覚、`tests_symbol_iter.rs:283` の `ECMA-262 §14.7.5.9 .return()` も §14.7.5.9 = "EnumerateObjectProperties" の drift)。**ECMA-262 では AO 名が版間で section 番号より安定なので `aoid <spec> <name>` で AO 名から正規番号を引き直すのが推奨**。algorithm step の prose 確認には `body <spec> <anchor-or-AO-name>` (multipage chapter cache 経由、truncate なし)。

### Workflow

- **コミット前**: `cargo fmt --all`
- **Push 前**: `mise run ci` (check + lint + test-all + doc + deny + ci-sweep cleanup; ci-sweep は `cargo-sweep` 未インストール時 no-op)
- **テストは変更クレートに絞る**: `cargo test -p <crate> --all-features`。`--workspace` / `mise run test` は最終検証時のみ
- **Git**: main 直接 push 禁止、PR 経由必須。`gh pr merge --auto` 禁止。CI 全 pass を目視確認してから squash merge
- **並行セッション / worktree 隔離**: 他 Claude instance と working tree を共有し得る (parallel sessions)。**コミットするブランチは専用 worktree で隔離して作業** (新規ブランチ = `git worktree add -b <branch> <dir> origin/main` ← clean base 明示で汚染 HEAD を継がない / 既存ブランチ [in-progress PR の復旧等] = shared tree から外してから `git worktree add <dir> <branch>` ← `-b` は既存名で fail) (shared main tree で直接 commit しない — 並行 instance の branch 切替/commit が HEAD を動かし、`git push HEAD:<branch>` で他人の commit が PR に混入する)。*自分が作っていない WIP / "file modified since read" / HEAD が動いた* のいずれかを見たら STOP → worktree 隔離。commit/push 直前に `git branch --show-current` + `git log --oneline origin/main..HEAD` でスコープ目視し、push は `HEAD:<other>` でなく明示 branch ref。背景 = 共有ツリー経由で並行セッションの commit が PR #285 に混入した incident。pre-push フック (`~/.claude/hooks/git-push-branch-guard.sh`) が branch-mismatch push を機械的にブロック。

## Commands

```sh
mise run ci          # 全 CI (push 前必須)
mise run test        # cargo nextest run --workspace --all-features (fast, no doc-tests)
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
```

その他: `test-all` (+ doc-tests) / `test-doc` / `doc` (RUSTDOCFLAGS=-D warnings) / `bench` (CSS / style / layout)。cargo を呼ぶ task (`check` / `test` / `test-doc` / `test-all` / `lint-clippy` / `doc`) は `--all-features` で gate (feature-gated code 含む)。`lint-fmt` / `deny` は feature と無関係。

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

`changes` path filter (`dorny/paths-filter@v4`、`.github/workflows/**` 含む) で以下 3 job を gate: `check` (3 OS × `cargo fmt --all -- --check` + clippy + nextest + doc-tests、後 3 つは `--all-features`) / `doc` (`cargo doc --workspace --no-deps --all-features` + `RUSTDOCFLAGS=-D warnings`) / `deny` (license + supply chain)。**Push to main は path filter bypass で常時全 job 実行**。コマンド詳細 = `.github/workflows/ci.yml`。
