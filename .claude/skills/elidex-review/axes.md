# elidex review axes — SSoT

5 axis 定義、`elidex-review` (post-impl diff) + `elidex-plan-review` (pre-impl plan-memo) 共有。

Lifecycle (Step 1.5 / 3 / 3.5 / 4 + anti-patterns + change log) は `./workflow.md`。SKILL.md は axes.md + workflow.md を参照する thin wrapper。

Agent prompt は「Read axes.md Axis N → apply `Detect` の `[diff]`/`[plan]`/`[both]` context tag に該当する entry を input に適用 → output format で結果」。Section-header tag (e.g. `Sub-check 2b [both]`) は section 内全 entry / procedure step に inherit。Axis 番号は安定 (renumbering NG、SKILL.md が参照)。

**Fix 提案禁止 (Step 3.5 で行う) — output format の `Suggested fix` field の意味**: 各 axis の Output format には `Suggested fix` / `ECS-native alternative` / `Recommended action` 等の field が含まれるが、これらは **agent が input を読んで発見した raw suggestion** であって philosophy-aligned な user-facing 推奨 fix ではない。User-facing 推奨 fix は Step 3.5 (philosophy alignment re-evaluation) で構築される。Agent は raw suggestion を input として記録するに留め、「これが推奨 fix です」と endorse しない (smallest-patch bias 防止)。

**`memory/...` reference convention**: 本 file で `memory/X.md` と書かれている path は **CLAUDE Code per-user memory dir** (`~/.claude/projects/<encoded-repo-path>/memory/X.md`) を指し、git tracked な repo file ではない。Project member は CLAUDE memory dir で該当 file を read 可能、cross-dev reader (memory dir なし) は inline context summary に依拠する形。

## Common: severity calibration

- **CRIT** — merging で immediate damage / spec contract 違反 / Layering mandate 違反 (incident-derived) / build break。push gate / plan gate で mandatory fix
- **IMP** — fix-or-something-tangible-breaks: design 原則違反 (ECS-native lens / ideal-over-pragmatic) で observable consequence、Copilot R で必ず flag される候補
- **MIN** — preferable but no concrete consequence: 軽微 style / 雑然
- **FP** — agent 過剰反応 / context 不足 / 既 design 決定 / user 既明示意図あり (Step 3.5 では skip)

## MECE 整理ルール (axis 重複の所有権)

- **lesson #276 ObjectKind variant overuse** → Axis 2 専属 (Axis 5 では検出しない)
- **Test data-flow gap (test が依拠する state が実 mutation path で populate されない)** → Axis 2 sub-check 2b 専属 (the underlying problem)
- **Test drop / workaround を pragmatic shortcut として選択** → Axis 3 専属 (the response choice — broken assumption 自体は Axis 2 が catch)
- **Symptom-level patch when root fix is in-context** → Axis 3 専属 (Axis 2 で data-flow gap 検出 → Axis 3 が対応方法を評価)
- **Past lesson 違反** (lesson #235 trait cascade 等) → Axis 5 (lesson #276 を除く)
- **Host-side OO inheritance** → Axis 1 + Axis 2 両 flag (genuine overlap、Step 3.5 で subsumption)

---

## Axis 1 — Layering mandate

**Purpose**: 2 つの layering 軸を扱う:
- (a) **Engine vs Host**: `vm/host/*` は engine-bound 責務のみ。DOM algorithm は `elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler` 経由
- (b) **Core vs Compat**: elidex-js **core** = ES2020+ strict-mode-only baseline。LegacySemantics (sloppy mode / Annex B / sloppy direct-eval / var hoisting quirks / `arguments.callee` `.caller` / `with` 文 / `__proto__` accessor 等) は compat plugin 領域、core VM では実装しない (design doc §14.1)

**Reference**: `CLAUDE.md` § "Layering mandate (2026-05-04 incident 由来)" / `memory/m4-12-architectural-drift-incident.md` (PR #151: 4 R-loop × 17 IMP findings before drift detected) / `memory/reference_elidex-js-core-strict-only.md` (D-17b-r2 由来、core/compat split judgment axis)

### Detect

**(a) Engine vs Host**:

- `[both]` `vm/host/*.rs` に 10+ LoC の loop / walker / state machine / coercion algorithm を新規追加または拡張 (diff: 該当行、plan: §Body/§Implementation 記述)
- `[both]` `EcsDom::traverse_descendants` / `find_by_id` / `with_attribute` 等の direct call が marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) を超える
- `[both]` 新規 fn が DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等を `vm/host/` で実装
- `[both]` 既存 engine-indep crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) に類似 API があるかの確認・経由明示の有無
- `[plan]` plan §"Layering check" / §"Architecture" に既存 crate API への mapping 表が欠落 (MIN)

**(b) Core vs Compat (design doc §14.1)**:

- `[both]` core VM (`crates/script/elidex-js/src/vm/` 内、`host/` 除く) で LegacySemantics 機能 (sloppy mode coercion / Annex B / sloppy direct-eval scope injection / var hoisting quirks / `arguments.callee` `.caller` / `with` 文 / `__proto__` accessor / RegExp legacy / 文字列HTMLメソッド) を実装 → **CRIT/IMP** (compat plugin 領域に移管 OR drop)
- `[plan]` plan-memo で **direct/indirect eval 区別 / sloppy mode flag / Annex B 専用 dispatch path / var hoisting quirks 専用 opcode** 等の motivation で defer slot 立てる案が出ている → **IMP** (core では不要、compat plugin 着手まで slot 不要)
- `[plan]` plan-memo の `FrameKind` / opcode / dispatch path 拡張案が "direct/indirect eval semantic 区別" "sloppy this auto-boxing" 等 LegacySemantics 機能を motivation にしている → **IMP** (core scope 外、`memory/reference_elidex-js-core-strict-only.md` 参照)

**Acceptable exceptions**:
- (a) engine 自体 (boa_engine / hecs 等 low-level) / legacy code 変更無し
- (b) motivation が "compat plugin 設計時の hand-off note" として明示されている (= "本 slot は LegacySemantics plugin 着手時に再評価" と書かれている) → FP-allowed

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <violation summary>
  Suggested fix: <which engine-indep crate API OR new helper>
```

---

## Axis 2 — ECS-native lens

**Purpose**: OO browser engine pattern (inheritance + virtual dispatch + observer registry) を直訳しない。ECS first principles で設計。**component data-flow integrity (read/write balance) もこの axis 所属**。

**Reference**: `CLAUDE.md` § "Design philosophy: ECS-native first" / `memory/feedback_ideal-over-pragmatic.md` / `memory/feedback_objectkind-resolution-uniformity.md` (lesson #276)

### Sub-check 2a: OO pattern direct translation `[both]`

- 新規 trait に OO observer pattern marker (`*Observer` / `*Listener` / `*Subscriber` + `Vec<Box<dyn>>` registry shape)
- Inheritance hierarchy 風 trait + single concrete impl (over-abstracted; ECS では component pattern が natural)
- `ObjectKind` variant 追加で state shape が unique でないもの (ECS component + brand-check 統一が原則、**lesson #276 専属**)
- Class-owned mutable state を struct member field で保持 (ECS では Component-on-Entity が natural)
- **per-entity 状態を entity-keyed side-store/registry (HostData `*_cache` / `*Registry` / `HashMap<entity, _>`) に持たせ、その値が `Send + Sync` かつ per-VM identity handle でない** → ECS component on the entity が正。boa `HostBridge` side-store の VM 直訳が典型 (lesson: D-21 R3 CRIT)。`Send+Sync` は hecs 適格条件 (= component に *できる*) であって *すべき* とは別 — **side-store 許容の例外 (FP)** は2つ: **(a) per-VM identity handle** = VM `ObjectId` (JS wrapper / callback) を保持 — 値は Send (`ObjectId(u32)`) だが per-VM 意味で、component 化すると cross-DOM aliasing (EcsDom が VM 間で entity-index 共有 + rebind、`vm_api.rs` `Vm::unbind` cache-clear 参照)。`world_id` discriminator 解禁まで HostData が正 (= `#11-wrapper-cache-cross-dom-discriminator` / `-component-migration`)。**`Send+Sync` だが component にしないのが正しいので、wrapper-cache 群 (~25) を violation と flag しないこと。** **(b) shared cross-cutting state** = cookie jar / `NetworkHandle` 等、per-entity でない browsing-context/session 資源。詳細 → `memory/feedback_boa-hostbridge-port-is-not-a-registry.md` / `memory/ecs-native-side-store-audit-2026-05-21.md`
- OO → ECS idiom 翻訳が抜け (subscriber registry → marker component + system query、observer fan-out → version counter + lazy detection、virtual dispatch → typed entity ECS query)

### Sub-check 2b: Component data-flow integrity `[both]`

変更が ECS component field を read する場合、その field の write-path (全 mutation source) が新規/変更 path で reconcile されることを確認。**Step 1.5 mental dry-run の出力 (`<dry-run-file>`) を必ず incorporate**。

各新規 read (`fcs.X` / `Attributes.Y` / `ElementState.Z` 等) に対して以下 procedure を実行:

1. SoT (source-of-truth) 候補を列挙 (例: `FormControlState.name` の SoT は content attribute `name`)
2. SoT を変更する全 path を列挙 — typical mutation sources:
   - IDL property setter (`input.name = 'q'`)
   - `setAttribute` direct call
   - HTML parser path (`innerHTML` / `outerHTML` / `DOMParser` / initial document parse)
   - `MutationDispatcher` consumer callbacks
   - Bulk init (`init_form_controls` 等の attach-time hook)
3. 各 path で field が正しく reconcile されるか確認 — 該当変更が SoT 側を patch しているなら derived field の reconciliation hook 必要
4. 1 path でも抜けがあれば **IMP** (read 側が現実に存在しない state を仮定する形)
5. 「全 path 単一 reconciler 経由 (MutationDispatcher 等)」OR「全 path 個別 update」のいずれかに揃えば OK

**例 (D-29 ケース)**: `[IMP] plan §E-8 — fcs.name read depends on input.name IDL setter sync which does not exist; root fix = mutation-driven FCS reconciliation, symptom fix = 個別 IDL setter patch (regression risk)`

### Detect (top-level)

- `[plan]` plan §"ECS-native check" / §"OO → ECS mapping" subsection の欠落 (MIN)

**Acceptable exceptions**: engine 内部 (boa_engine 等 self-implementation) / legacy code 変更無し

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <sub-check 2a/2b short label> in <context>
  ECS-native alternative: <component / system / query / marker / reconciliation infra>
  Reference: <lesson # or feedback memory>
```

---

## Axis 3 — Pragmatic shortcut detection

**Purpose**: `// stub` / `// for now` / `// minimal v1` / `// session 短縮` / `// scope cut` 等の pragmatic shortcut marker を検出。CLAUDE.md "TODO 先送り禁止" + "後方互換性は維持しない"。

**Reference**: `CLAUDE.md` § "Design philosophy: Ideal over pragmatic" / "Design discipline" / `memory/feedback_ideal-over-pragmatic.md`

### Detect

- `[both]` `// stub:` / `// minimal v1` / `// for now` / `// quick fix` / `// temporary` / `// placeholder` 等 marker (diff: code、plan: 文中 phrase)
- `[diff]` `unimplemented!()` / `todo!()` macro 追加 (existing 除外)
- `[both]` `TODO` / `FIXME` 追加で defer slot 番号 (`#11-*`) の citation 無し
- `[both]` "later" / "future PR" / "M4-13" 等の deferral 言及で defer slot 引用無し
- `[both]` Shim / fallback / backwards-compat wrapper 追加
- `[diff]` Test 内 `.unwrap()` で適切 error context 無く placeholder 風
- `[diff]` 既存 test を delete / `#[ignore]` する diff で削除理由が「broken assumption の root fix が別 PR」OR「duplicate coverage 確認済」のいずれかに帰着していない
- `[diff]` 既存 TODO 痕跡があるのに引き取らず個別 patch (元 TODO が dead code 残存 + SSoT 違反、**Axis 3 専属**)
- `[plan]` plan §scope / §body に "no-op" / "stub" / "session 短縮" / "minimal v1" / "for now" / "scope cut" 等
- `[plan]` plan が「test 簡略化のため scope 削る」/「失敗 test drop」/「IDL gap workaround 経由 test」 — broken assumption の root fix が plan 内 OR 別 PR slot で明示されているか確認
- `[plan]` plan が既存 TODO を引き取らない (該当 area で `grep TODO` 痕跡があるのに plan 内 close 予定無し)
- `[plan]` plan §Defer の新規 slot に `Why deferred` + `Re-evaluation trigger` + `Re-evaluation date` の 3 要素が揃っていない (1 つでも欠落で MIN)

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <marker>: "<excerpt>"
  Required action: (a) 該当 spec gap に defer slot 立てて引用 OR (b) 本 PR / plan 内で proper 実装 OR (c) inline TODO 削除 + accept-as-is justify
```

---

## Axis 4 — Spec citation discipline

**Purpose**: 新規 VM accessor / DomApiHandler / native fn / engine-indep algorithm の docstring に WHATWG / HTML / DOM / URL / CSS / ECMAScript / WebIDL / Fetch / Streams / IndexedDB 等の section/step citation。Spec-faithful 実装の trace 可能性。

**Reference**: CLAUDE.md § "Design philosophy" (spec-faithful, html5ever 非依存) / 各 spec docs

### Verification recipe (webref)

Section number / title / anchor / WebIDL / AO / **algorithm prose** の確認は `.claude/tools/webref` を使う。Data source 自動切替: WHATWG/W3C は w3c/webref machine-readable extracts、tc39 (ECMA-262 / ECMA-402) は `@tc39/<spec>-biblio` (tc39 公式 publish の machine-readable JSON、jsdelivr CDN 経由)、prose 本文は spec multipage chapter HTML (webref `href` / tc39 chapter-file 導出で URL 自動解決、HTTP cache 共有)。WebFetch で spec HTML を取りに行くと長文 truncate されるため citation 整合確認・algorithm prose 確認には不向き — webref / biblio / `body` は per-spec の構造化 JSON / IDL / multipage chapter 単位なので 1 fetch + filter で決定的に効く。

Subcommand (詳細 = `.claude/tools/webref --help`):

- `heading <spec> <number-prefix>` — section number → title + anchor (citation 整合確認の主力)。`<spec>` は WHATWG/W3C shortname (`html` / `dom` / `geometry-1` …) または tc39 shortname (`ecma262` / `ecma402`)
  ```bash
  .claude/tools/webref heading html 4.13.4
  # → §4.13.4 The CustomElementRegistry interface #custom-elements-api
  # (well-known 風 cite「§4.13.4 = upgrade queue」は誤 — §4.13.5 Upgrades が正)

  .claude/tools/webref heading ecma262 14.7.5.9
  # → §14.7.5.9 EnumerateObjectProperties ( obj )  https://tc39.es/ecma262/#sec-enumerate-object-properties
  # (well-known 風 cite「§14.7.5.9 .return() per abrupt completion」は誤 —
  #  実体は EnumerateObjectProperties; ES 改版で section 番号 shift)
  ```
- `aoid <spec> <name>` — **(tc39 専用)** abstract operation 名 → §number + anchor + signature kind。biblio の `op` / `clause` / `built-in function` / `concrete method` entries を `aoid` field で照合、anchor 経由で clause に cross-ref して §number と title を引く。**ECMA-262 cite を書く時は `aoid` を主、`heading` を従** — AO 名は版間で安定、section 番号は不安定
  ```bash
  .claude/tools/webref aoid ecma262 ToNumber
  # → ToNumber §7.1.4 ToNumber ( argument ) (abstract operation) ...#sec-tonumber
  .claude/tools/webref aoid ecma262 OrdinaryGet
  # → OrdinaryGet §10.1.8.1 OrdinaryGet ( obj, propertyKey, receiver ) (abstract operation)
  ```
- `dfn <spec> <term>` — concept dfn → 包含 §heading + anchor (exact 失敗時 substring fallback)。**term-based citation の正規 anchor 確認に最強** (WHATWG/W3C 専用; tc39 biblio にも `type=term` entry が 385 件あるが現 helper は未公開、必要時に拡張)
  ```bash
  .claude/tools/webref dfn html 'reaction queue'
  # → 'custom element reaction queue' → §4.13.6 Custom element reactions
  ```
- `idl <spec> <interface>` — interface IDL fragment (WebIDL 直 grep、属性 / メソッド signature 確認)
  ```bash
  .claude/tools/webref idl html CustomElementRegistry
  ```
- `element <spec> <tag>` — HTML/SVG element → 対応 interface 名 + href (`<canvas>` → `HTMLCanvasElement`)
- `css <spec> <name>` — CSS property / @rule / selector / value の metadata (value syntax / initial / inherited / appliesTo / computedValue / `styleDeclaration` IDL 名 等)。CSS plugin crate 新 property 着手時の正規定義引き
- `body <spec> <anchor-or-AO>` — section / algorithm の **本文 prose** を抽出 (multipage chapter HTML 経由、truncate なし)。tc39 は `sec-X` anchor (e.g. `sec-iteratorclose`) または AO 名 (e.g. `IteratorClose`、自動 anchor 解決) どちらも可。WHATWG/W3C は heading anchor (`the-iframe-element`) または dfn anchor (`concept-upgrade-an-element`) どちらも可 — algorithm dfn は dfn 直後の `<ol>` step を `1. 2. 3.` でレンダリング。section 番号 → title pair の cross-check は `heading` / `aoid`、step の semantic 一致確認は `body`
  ```bash
  .claude/tools/webref body ecma262 IteratorClose           # AO 名 → §7.4.11 step list
  .claude/tools/webref body html concept-upgrade-an-element # custom-elements upgrade algorithm step list
  ```
- `specs <keyword>` — spec catalog 検索 (shortname 不明時、WHATWG/W3C のみ)

**HTTP cache**: 全 fetch は `~/.cache/elidex-webref/` (`XDG_CACHE_HOME` 設定時は `$XDG_CACHE_HOME/elidex-webref/`) に ETag / Last-Modified ベース conditional GET 経由で保存される (930 KB biblio / WHATWG headings/dfns JSON を毎回再 download しない)。bypass は `ELIDEX_WEBREF_NO_CACHE=1`。

**WebFetch との使い分け**:
- **webref (構造化)**: section number / title / anchor / IDL fragment / dfn / AO (aoid) — 決定論的、grep 可、全 spec 一発取得 (構造化 fact)
- **webref `body` (prose)**: algorithm step / 概念定義の prose 本文 — multipage chapter HTML cache、truncate なし、`<ol>` step は `1. 2. 3.` でレンダリング、tc39 AO 名は anchor 自動解決
- **WebFetch**: 上記でカバーできない外形(spec 全体俯瞰、issue thread、blog post、tc39 proposal-stage README 等)。spec.whatwg.org / tc39.es の section prose は `body` で取れるので WebFetch は不要

### Detect

- `[both]` Spec citation が docstring (diff) / plan の新規 fn 説明 (plan) に無い
- `[both]` 既 citation あっても section/step syntax (`§4.2.6 step 3.1`) が不正 / 一貫性無し
- `[both]` Citation で section number ↔ title pair が並記される箇所 (例: `§4.13.4 upgrade queue` / `§4.12.5.1.7 OffscreenCanvas` / `ECMA-262 §14.7.5.9 .return()`) で、agent は **webref `heading <spec> <number>` を実行して number ↔ title 一致を確認**。不一致は **IMP**:
  - **WHATWG/W3C 例**: D-17 で `§4.13.4 upgrade queue` は誤、§4.13.4 = "The CustomElementRegistry interface" / §4.13.5 = "Upgrades" — `consumer_dispatcher.rs:75` の `reaction queue (HTML §4.13.3)` も §4.13.3 = "Core concepts" であって reaction queue 定義は §4.13.6 が候補 → suspect として flag
  - **ECMA-262 例**: `tests_symbol_iter.rs:283` の `Per ECMA-262 §14.7.5.9, .return() is only called for abrupt completions` は誤、§14.7.5.9 = "EnumerateObjectProperties ( obj )" (ES 改版で section shift)。**ECMA-262 cite を見たら必ず `aoid <spec> <name>` で AO 名から番号を引き直す** (AO 名は版間で安定、番号は不安定)
  - **section number だけが書かれていて title が無い citation は対象外** (title-less cite は drift verifier がない、ただし ECMA-262 で AO 名が文中にあれば aoid lookup で逆引き可能 — その場合は AO 名 → 期待番号 → 実 cite 番号で照合)
- `[both]` "per WHATWG" / "per HTML" の prose だけで section 番号無し
- `[both]` Algorithm 実装 (diff) / 実装計画 (plan) に WHATWG step-by-step reference 無く独自実装 (spec drift risk)
- `[plan]` plan §"Docstring requirement" / §"Spec citation table" 欠落 (新規 fn × spec citation 対応表が plan 内に無いと implementation で抜け落ちる)

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <function/handler name> missing/incorrect spec citation
  Expected: "WHATWG <spec> §<section> <step>" or equivalent
  Suggested wording: <one-line citation>
  webref lookup (if applicable): <output line from `.claude/tools/webref heading ...`>
```

---

## Axis 5 — Project-context integrity

**Purpose**: `MEMORY.md` `Active state` / `m4-12-platform-gap-roadmap.md` / active plan-memos / 過去 lesson 整合性。Defer slot reference 整合、resolved anti-pattern 再導入検出、1000-line file growth awareness。

**Reference**: `memory/MEMORY.md` § "Active state" / `memory/m4-12-platform-gap-roadmap.md` § "D-tier3" / recent landing memos (`memory/m4-12-pr-*-landing.md`) の "Lessons" section

### Detect

- `[both]` 引用 defer slot 番号 (`#11-*`) が memory ledger で actually open 中か (旧名 slot ref 引用なし、recent landing memo で renamed/closed されていないか)
  - **Acceptable exception (FP, not IMP)**: 引用 slot が未登録でも、PR の plan-memo 内に "**D-N ship 時に登録**" / "**at landing memo time register**" 等の **pre-agreed ship-time registration commitment** が明示されている場合 (例: `m4-12-pr-d29-form-submission-plan.md:169` "NEW slot `#11-form-navigation` — D-29 ship 時に登録、+1")。Plan-memo の該当行を citation として finding に添えること。Agent はこの場合 **FP-defer** で record、landing memo §"Defer ledger" で slot definition 必須 reminder を Step 3.5 で fold。Diff review (`elidex-review`) でも同 ledger を読んで同一 exception を適用 (plan-stage 合意の admin debt を ship-stage で重複 flag しない)
- `[both]` §D-tier3 phase plan / `MEMORY.md` `Active state` との整合 (scope creep / batch 違反検出)
- `[plan]` **roadmap slot の前提がコード実態で検証されているか** (premise-correction): plan-memo が roadmap/MEMORY の slot framing (scope / blocker / 「未実装」「未着工」前提) を **現状コードに照合せず**そのまま採っていないか。slot は (a) 既に VM 実装済 (D-23 `document.cookie` 既実装で close-as-done)、(b) 記載 blocker が stale (D-21 「paint pipeline 未着工」誤 — 実は構築済)、(c) 記載依存が不要 (D-17 「after_attribute_change hook 必要」誤 — 既存 `AttributeChange` で足る) のことがある。plan-memo に "現状コード照合済 (file:line)" の痕跡が無く、roadmap one-liner を premise にしているなら **IMP** (着手後に scope が崩れる)。Acceptable = plan が premise-correction を明記 or 既存 surface を file:line で確認済
- `[both]` 過去 lesson 違反: lesson #235 trait extension cascade 等 (**lesson #276 は Axis 2 専属、ここでは検出しない**)
- `[both]` 同時並行で他 PR が変更している file での convention drift (`git branch -r` で active branches 確認)
- `[both]` 過去 PR R-loop で flagged & resolved した anti-pattern の再導入 (recent landing memos "Lessons" section 参照)
- `[both]` 1000-line file growth: 該当 file の現在行数を実 bash で測定 (`git diff --name-only --diff-filter=AMR main -- 'crates/**/*.rs' | while IFS= read -r f; do wc -l "$f"; done` — filter は A/M/R 全部含める; portable form per `xargs -r` 非互換回避)、1000 行 push OR 既 1000+ file への substantive add (>50 LoC) を **MIN finding** (gate NOT、author 判断で split / sweep) として surface

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <project-context violation>
  Reference: <memory/<file>:<line> citation>
  Recommended action: <fix or scope adjustment>
```

---

## Maintenance

Axis 番号は安定。新 sub-check 追加時はインライン引用 (例: Sub-check 2b は 2026-05-19 D-29 trial 由来)。詳細 change log は `./workflow.md` § "Change log"。

### Future expansion candidates (field-test 待ち、premature な追加は避ける)

- **Sub-check 2c — New ECS component data-flow (read/write balance)**: 新規追加 ECS component の read site / write site 両側存在確認。D-29 trial で必要性未確認、実需 PR 発生時に flesh out (具体 detect 手段 + plan-memo `§Components added` template 要件)。
- **Test coverage discipline axis (Axis 6 candidate)**: engine-indep test と JS-level test の covering matrix 検証。`/elidex-review` 運用後 false positive 率を見て要判断。
