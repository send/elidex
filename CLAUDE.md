# CLAUDE.md — elidex

Experimental browser engine written in Rust.

## Development Rules

### Design philosophy

- **至上命題: ECS ベースのブラウザの価値の検証**: elidex の存在理由は ECS-native なブラウザという設計の価値を検証すること。これを揺るがす判断は許容しない (他のどの原則よりも優先)。
  - 根拠の順位: (1) 仕様 → (2) 仕様が沈黙なら「ECS ベースのブラウザとしてどうあるべきか」(本節の原則) → (3) それでも決まらない時だけ既存ブラウザの実測を「証拠」として記録。
  - 既存ブラウザ (Chromium/Blink/V8, Firefox/Gecko/SpiderMonkey, Safari/WebKit/JSC, Servo, Ladybird 等) の「振る舞い」を core の要件・受け入れ条件・テスト期待値にしない。非標準の互換は compat 層。 [contrast]
  - ブラウザ名を書く時は [spec]/[evidence]/[compat]/[contrast]/[history] を付ける (hook が機械的に強制)。review での等級 (IMP/CRIT) は `.claude/skills/elidex-review/axes.md` Axis 2 の authority-source sub-check が正。
- **ECS-native first**: elidex は **既存にない novel な ECS-native browser engine**。既存ブラウザの「構造」(パイプライン順、状態の置き場所、registry、プロセスモデル) を設計の根拠にせず、OO browser engine (Blink / Gecko / WebKit) の inheritance + virtual dispatch + observer registry pattern を ideal として直訳しない。ECS first principles (component / system / query / marker / event queue / version counter) から導き、ブラウザ名は対比としてのみ書く。OO pattern を翻訳する時は idiom 対応表を明示 (例: observer registry → marker component + query, subscriber list → component query, class-owned state → ECS component on entity)。**ECS-native であること自体が elidex の design contribution**。 [contrast]
  - **Side-store→component 判定ルール**: per-entity 状態を entity-keyed の side-store/registry (HostData の `*_cache` / `*Registry` / `HashMap<entity, _>`) に持たせる時、その値が `Send + Sync` **かつ per-VM identity handle でない**なら **ECS component on the entity が正** (SameObject = component get、GC = 単一 query、despawn = 自動 cleanup)。boa `HostBridge` side-store を VM へ直訳するのが典型的な罠。`Send+Sync` は hecs 適格条件であって「すべき」とは別 — side-store が正当な例外は 2 つ:
    - **(a) per-VM identity handle (一時的例外)**: VM `ObjectId` (JS wrapper / callback) を保持するもの。現状の `EcsDom` は VM 間で entity-index 空間を共有し rebind され得るので、component 化すると cross-DOM aliasing になる (`Vm::unbind` の cache-clear: `git grep -n 'must be cleared on unbind' -- crates/script/elidex-js`)。それまでの正は per-VM HostData (unbind-clear)。解禁条件は agent-scoped `EcsDom` World (B1、`docs/plans/2026-06-agent-scoped-ecsdom-world.md`。world_id discriminator は撤回済)。migration では既存 `WrapperKey {owner, kind, subkey}` + realm の full keying が必須 — `Entity→ObjectId` や `(entity, realm)` だけでは衝突し SameObject を壊す (realm 軸は同 doc §5 req 6/7)。本項は B1 実装時 (world_id 実コード削除と同時) に改訂する。
    - **(b) shared cross-cutting state (恒久的例外)**: cookie jar / `NetworkHandle` 等、単一 entity の事実でない browsing-context/session レベル資源 (per-entity でないので component 対象外、Send+Sync でも該当しない)。
- **Whole-engine core/compat/deprecated consistency**: `docs/design/ja/01-executive-summary.md` / `07-plugin-system.md` / `13-script-session.md` の SSoT。HTML / DOM API / CSSOM / ECMAScript / Web API / parser recovery は同じ三層 pattern (clean core + optional compat + deprecated/removal) に従う。Legacy / non-standard / blocking / sync / quirks / Annex B / document.write / live collection / localStorage / XHR / document.cookie 等を core に混ぜない。Core に置くなら「modern standard baseline として必要」な理由、compat に置くなら modern equivalent への shim/normalization boundary を明示する。
- **Plugin-first extensibility**: elidex の拡張点は原則 `elidex-plugin` 型の同一 mental model (static enum dispatch for built-ins + dynamic trait object for runtime extension) に収束させる。HTML tag / CSS property / layout algorithm / network middleware / DOM/CSSOM/Web API handler / JS language feature を個別 registry / ad hoc hook / hard-coded branch で増やす前に、対応する plugin trait・SpecLevel・feature gate に載せる。Hot path built-in は static dispatch、user/experimental/policy は dynamic extension が正準形。
- **ScriptSession as the sole Script↔ECS boundary**: DOM / CSSOM / future OM (Selection, Range, Performance 等) と JS/Wasm は `ScriptSession` の Identity Map / Mutation Buffer / flush / GC coordination / live query 管理を共有する。OM ごとの ad hoc wrapper cache、直接 ECS mutation、host-local live query registry を増やさない。書き込みは session mutation と flush point に集約し、SameObject・MutationObserver・atomic script-task visibility を同一機構で守る。
- **Concurrency by ownership and phases**: renderer main thread は ECS DOM の唯一の owner/writer。Script mutation は `ScriptSession::flush`、style/layout/paint は event-loop phase ごとの明示 window、compositor は原則 owner-transfer された DisplayList / Layer data を読む。Lock 共有で辻褄を合わせる前に、ownership transfer・single writer・double buffer・FrameSource boundary で競合を構造的に消す。Thread pool は重複させず、CPU 並列 work は共有 rayon pool、I/O readiness は runtime/reactor 境界に寄せる。
- **Security by structure, not review convention**: security boundary は後付けしない。Renderer は direct network/file/storage/permission access を持たず Browser/Network/Storage broker 経由、web-content storage は origin 物理分離、browser-owned state は Browser Process 集中管理が原則。Policy をコアに焼かず mechanism を提供する領域 (例: content blocking) と、secure-by-default を engine が強制する領域 (HTTPS-only, third-party cookie blocking, permission gate, quota/eviction) を区別する。
- **Ideal over pragmatic**: 設計判断は「最もクリーンで spec-faithful + future-extensible な architecture」が default。"現実解" "minimal v1" "stub で済ます" "session 短縮のため scope 削る" は基本選ばない。defer cap / session 残見込は judgment 補助情報であって設計選択の制約条件にしない。User が明示的に "session 節約優先" "stub OK" "scope 制約あり" と言った時のみ pragmatic 路線。

### Design discipline

- **設計優先**: 場当たり的な reactive fix 禁止。新しい型を足す前に既存の抽象で解決できないか考える。dead code は接続するか削除
- **One issue, one way**: より良い機構を見つけたら、その種の処理は **単一の正準形に一括収束** (full unification) させる。「新 seam + N 個の legacy 実装」が共存する strangler 中間状態を残さない — 共存は「どっちで書く?/これは migration 対象?」という決定 tax を消さず移すだけで、PR ごと・reader ごとに再発する。決定の表面積を下げること自体が目的で、upfront の大きめ refactor に値する (Ideal over pragmatic の decision-surface 版)。
- **Phase design is not a shortcut license**: design doc の Phase 1–3 / Phase 4+ は「長期簡素化を妨げない境界を先に置く」ための段階的緩和であって、temporary hack の免罪符ではない。外部 runtime / process boundary / transport / storage backend 等を暫定採用する時は、`IpcChannel` / `AsyncRuntime` / `FrameSource` / `StorageBackend` / `HttpTransport` のように、後続の統合・置換が設定変更または局所差し替えになる trait boundary を置く。存在しない代替のための抽象化は ADR #26 (Vello) のように明示例外化する。
- **Supported-surface testing**: elidex は WPT 100% を目標にせず、plugin / feature が宣言した supported subset の regression を防ぐ。新しい standard/plugin surface は担当 WPT subset・engine-independent unit/integration coverage・必要なら visual/fuzz/benchmark のどれで守るかを明示する。Compat は best-effort でも、core supported surface の新規失敗は原則 block。
- **TODO 先送り禁止**: 計画に含めた実装を安易に TODO にしない。実装不可能なら理由 + 対処時期を明示して確認を取る
- **後方互換性は維持しない**: デッドコードや shim は残さず削除
- **1000-line debt = touch-time split (defer しない)**: >1000 行の file (source・test 問わず) を触る時、real cohesion seam があれば feature 着手前に **standalone な prereq split** (単独 PR / 単独 commit、feature PR に bundle しない) として分割する。line-count の機械適用でなく cohesion 判断 (一枚岩の cohesive unit・巨大 generated table・flat な case table は対象外)。review 時の backstop は `.claude/skills/elidex-review/axes.md` Axis 5 (trigger が別で、本 discipline の検出器ではない)。
- **Edge-dense work = multi-PR program + 実装前 plan-review 必須**: ≥3 intersecting invariant axes を束ねる、または正準アルゴリズムが無い subsystem を触る work は、単一 PR に束ねず umbrella plan + per-PR plan に分割して各 PR を個別に full review し、各 PR は実装前に `/elidex-plan-review` 必須 (judgment でなく rule)。edge-matrix を upfront に map して review tail を先取りする。再帰の終端 (承認済 umbrella 配下の per-PR slice) は `.claude/skills/elidex-plan-review/SKILL.md`「When to invoke」が正。
- **html5ever 非依存 (layout decoupling)**: Layout は html5ever の暗黙的 DOM 構造修正に依存しない (anonymous table box 生成等は css-tables-3 §2.2 Fixup に従い layout 側で実装) = parser 選択に依らない layout 層責務。parser backend の構成は `docs/architecture/dom.md`、配備モデル (strict-first → tolerant fallback) は design doc §11.3 が正。

### Layering mandate

**VM host/ は engine-bound 責務のみ**: `crates/script/elidex-js/src/vm/host/` は prototype install / brand check / JsValue↔Entity marshalling に限定する。DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等の algorithm は engine-independent crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) 経由で呼ぶ。`EcsDom::*` の direct call は marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) に限定する。新規 algorithm を host/ に書く前に、対応する engine-independent crate に既存実装が無いか確認 + 無ければそちらを拡張。

**Core vs compat split**: elidex-js **core** = ES2020+ strict-mode-only baseline。legacy semantics (sloppy mode (型変換 / this auto-boxing) / sloppy direct-eval の caller-env scope injection / var hoisting quirks / `arguments.callee` `.caller` / `with` 等) は **LegacySemantics** compat plugin、Annex B (JS 内 HTML コメント / `__proto__` accessor / RegExp legacy / 文字列 HTML メソッド) は **AnnexB** compat plugin の領域 — 一覧と動的コードの分割基準は `docs/design/ja/14-script-engines-webapi.md` §14.1 が正。VM internal design slot (`FrameKind` / opcode / dispatch path 等の拡張を含む) は motivation test で判定する: legacy semantics を成立させるための拡張は **core slot 化しない** (compat plugin 着手時の concern に hand-off)。spec が同一構文の strictness を経路ごとに別導出するがゆえに要る direct/indirect eval 区別 (ECMA-262 PerformEval) は legacy motivation ではないので **core が持つ**。

### Spec citation

- WHATWG / W3C / TC39 (ECMA-262 / ECMA-402) / CSS WG modules の section number / anchor / WebIDL / AO / algorithm prose は `.claude/tools/webref` で確認する。**WebFetch にフォールバックしない**。subcommand は `webref --help`、drift tooling を使う時は `webref agent-policy` / `.claude/tools/_webref/DESIGN.md`。
- 「§X.Y.Z = <name>」の number ↔ title pair は **必ず lookup** (well-known 風 cite を信用しない)。
- ECMA-262 は AO 名が版間で section 番号より安定なので、`aoid <spec> <name>` で AO 名から正規番号を引き直すのが推奨。step の prose は `body <spec> <anchor-or-AO-name>` (truncate なし)。
- CSS は第 1 引数に **level 込みの module shortname** (`css-text-3` であって `css-text` でない。不明時は `specs <keyword>`)。例: `body css-text-3 line-break-transform` = §4.1.3 Segment Break Transformation Rules。

### Workflow

- **コミット前**: `cargo fmt --all`
- **Push 前**: `mise run ci` (中身は `mise.toml` の `[tasks.ci]`)
- **テストは変更クレートに絞る**: `cargo test -p <crate> --all-features`。`--workspace` / `mise run test` は最終検証時のみ
- **Git**: main 直接 push 禁止、PR 経由必須。`gh pr merge --auto` 禁止。CI 全 pass を目視確認してから squash merge (`main-protection` に required-status-check は無いので、この目視が最終ゲート)
- **並行セッション / worktree 隔離** (一般則と停止条件は `~/.claude/CLAUDE.md`「並行セッション」。停止条件を見たら worktree 隔離へ): shared main tree では commit しない。新規ブランチ = `git worktree add -b <branch> <dir> origin/main` (clean base 明示で汚染 HEAD を継がない)、既存ブランチ = shared tree から外してから `git worktree add <dir> <branch>` (`-b` は既存名で fail)。commit/push 直前に `git branch --show-current` + `git log --oneline origin/main..HEAD` でスコープを目視し、push は `HEAD:<other>` でなく明示 branch ref (`~/.claude/hooks/git-push-branch-guard.sh` が branch-mismatch push をブロック)。
- **サブエージェント振り分け (model tiering)**: 規則は `~/.claude/CLAUDE.md`「サブエージェント振り分け (model tiering)」が正 (ここに書き写さない)。elidex での当てはめ: レビュー手順の clerical skip = `/elidex-plan-review` Step 5、ドライバーがその場で判定する実行 = `mise run ci` / `build_verify` の pass/fail (`grep 'ERROR task failed'`)、generator-layer 除去の先例 = #374、多軸レビュー skill = `/elidex-review` / `/elidex-plan-review` の 5 軸。
- **Plan-memo umbrella checker** (`.claude/tools/plan-memo-umbrella-check.py`): 何を検査するか・exit code (2 = 未走査であって clean ではない)・どこで走るかは script header が正。memo 編集でも `tools` job が赤になり得る (全 `docs/plans/*.md` を読む self-test control と、下の VM-P4 layout guard)。checker の lexer / population / assertion を触ったら control + mutant を同 commit で足す。
- **VM-P4 family** (`docs/plans/2026-07-vm-p4-*.md`): どれかを編集したら entry memo に checker を回す — `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md` (期待値 exit 0、1 は直す、2 は未走査)。family の構造変更・family 外リンク・family の拡張・pin の扱いは `docs/plans/test_vm_p4_family_layout.py` の docstring が正 (family を広げる時はこの行の glob も同 commit で更新)。

## Commands

```sh
mise run ci          # 全 CI (push 前必須)
mise run test        # nextest、doc-tests なし
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
```

他の task と中身は `mise tasks` / `mise.toml` (各 task のコメントが理由の正典)。

## Architecture

詳細は `docs/architecture/`:

| File | Covers |
|------|--------|
| `core.md` | elidex-plugin, elidex-ecs |
| `css.md` | CSS plugin crates (box/text/flex/grid/table/float/anim), elidex-style |
| `dom.md` | elidex-html-parser, elidex-dom-api, elidex-a11y |
| `layout.md` | elidex-layout |
| `script.md` | elidex-script-session, elidex-js, elidex-wasm-runtime |
| `net.md` | elidex-net, elidex-storage-core, elidex-api-sw |
| `shell.md` | elidex-shell, elidex-web-canvas |
| `tools.md` | elidex-crawler, elidex-wpt, benchmarks |

## Key Files

- `SECURITY.md` / `CONTRIBUTING.md` — security / contribution policy
- `deny.toml` — license allow-list + supply chain (`unknown-registry`/`unknown-git` = deny)
- `.github/dependabot.yml` — automated dependency updates (Cargo + Actions)
- `docs/design/ja/29-survey-analysis.md` — JA/EN 900-site compatibility survey

## CI

正典は `.github/workflows/ci.yml` — どの job が path filter で gate されるか、その理由は各 job のコメントにある。`trip-wires` / `tools` は意図的に filter で gate しない (filter を足す前に同 job のコメントを読む)。新しい check の置き場 (interpreter を宣言しない `trip-wires` か、宣言する `tools` か) も `tools` job のコメントが正。
