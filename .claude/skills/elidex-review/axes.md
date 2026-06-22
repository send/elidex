# elidex review axes — SSoT

5 axis 定義、`elidex-review` (post-impl diff) + `elidex-plan-review` (pre-impl plan-memo) 共有。

Lifecycle (Step 1.5 / 2 agent prompt template / 3 / 3.5 / 4 / 4.5 + anti-patterns) は `./workflow.md`。SKILL.md は axes.md + workflow.md を参照する thin wrapper。

Agent prompt は「Read axes.md Axis N → apply `Detect` の `[diff]`/`[plan]`/`[both]` context tag に該当する entry を input に適用 → output format で結果」。Section-header tag (e.g. `Sub-check 2b [both]`) は section 内 entry の **default**、per-entry tag (e.g. `- [plan] ...`) が付いている entry はそちらが override する。Axis 番号は安定 (renumbering NG、SKILL.md が参照)。

**Fix 提案禁止 (Step 3.5 で行う) — output format の `Suggested fix` field の意味**: 各 axis の Output format には `Suggested fix` / `ECS-native alternative` / `Recommended action` 等の field が含まれるが、これらは **agent が input を読んで発見した raw suggestion** であって philosophy-aligned な user-facing 推奨 fix ではない。User-facing 推奨 fix は Step 3.5 (philosophy alignment re-evaluation) で構築される。Agent は raw suggestion を input として記録するに留め、「これが推奨 fix です」と endorse しない (smallest-patch bias 防止)。

**`memory/...` reference convention**: 本 file で `memory/X.md` と書かれている path は **CLAUDE Code per-user memory dir** (`~/.claude/projects/<encoded-repo-path>/memory/X.md`) を指し、git tracked な repo file ではない。Project member は CLAUDE memory dir で該当 file を read 可能、cross-dev reader (memory dir なし) は inline context summary に依拠する形。

## Common: severity calibration

- **CRIT** — merging で immediate damage / spec contract 違反 / Layering mandate 違反 (incident-derived) / build break。push gate / plan gate で mandatory fix
- **IMP** — fix-or-something-tangible-breaks: design 原則違反 (ECS-native lens / ideal-over-pragmatic) で observable consequence、single-pass external review (Codex) で flag される候補
- **MIN** — preferable but no concrete consequence: 軽微 style / 雑然
- **FP** — agent 過剰反応 / context 不足 / 既 design 決定 / user 既明示意図あり (Step 3.5 では skip)

## MECE 整理ルール (axis 重複の所有権)

- **lesson #276 ObjectKind variant overuse** → Axis 2 専属 (Axis 5 では検出しない)
- **Test data-flow gap (test が依拠する state が実 mutation path で populate されない)** → Axis 2 sub-check 2b 専属 (the underlying problem)
- **Test drop / workaround を pragmatic shortcut として選択** → Axis 3 専属 (the response choice — broken assumption 自体は Axis 2 が catch)
- **Symptom-level patch when root fix is in-context** → Axis 3 専属 (Axis 2 で data-flow gap 検出 → Axis 3 が対応方法を評価)
- **Past lesson 違反** (lesson #235 trait cascade 等) → Axis 5 (lesson #276 を除く)
- **Host-side OO inheritance** → Axis 1 sub-check 1a + Axis 2 sub-check 2a 両 flag (genuine overlap、Step 3.5 で subsumption)
- **docstring-stated 契約 ↔ body 違反** (promise-returning native の `Result` swallow 等) → **Axis 3 専属** (generic "wrong value on error" は /code-review、「自 file 契約違反 + ideal-over-pragmatic」design 角度が Axis 3 領分)

---

## Axis 1 — Layering mandate

**Purpose**: 2 つの layering 軸を扱う (Sub-check 1a + 1b、独立判定)。Axis 2 の 2a/2b と同じ shape。

**Reference**: `CLAUDE.md` § "Layering mandate (2026-05-04 incident 由来)" / `memory/m4-12-architectural-drift-incident.md` (PR #151: 4 R-loop × 17 IMP findings before drift detected) / `memory/reference_elidex-js-core-strict-only.md` (D-17b-r2 由来、core/compat split judgment axis)

### Sub-check 1a: Engine vs Host `[both]`

Rule: CLAUDE.md § "Layering mandate (2026-05-04 incident 由来)" — "VM host/ は engine-bound 責務のみ" 段落 (DOM algorithm は engine-indep crate 経由)。

- `[both]` `vm/host/*.rs` に 10+ LoC の loop / walker / state machine / coercion algorithm を新規追加または拡張 (diff: 該当行、plan: §Body/§Implementation 記述)
- `[both]` `EcsDom::traverse_descendants` / `find_by_id` / `with_attribute` 等の direct call が marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) を超える
- `[both]` 新規 fn が DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等を `vm/host/` で実装
- `[both]` 既存 engine-indep crate に類似 API があるかの確認・経由明示の有無
- `[plan]` plan §"Layering check" / §"Architecture" に既存 crate API への mapping 表が欠落 (MIN)

**Acceptable exceptions (1a)**: engine 自体 (boa_engine / hecs 等 low-level) / legacy code 変更無し

### Sub-check 1b: Core vs Compat `[both]` (design doc §14.1)

Rule: CLAUDE.md § "Layering mandate (2026-05-04 incident 由来)" — "Core vs compat split" 段落 (elidex-js core = ES2020+ strict-only baseline、LegacySemantics は compat plugin 領域 = core VM 実装 NG)。

- `[both]` core VM (`crates/script/elidex-js/src/vm/` 内、`host/` 除く) で LegacySemantics 機能 (sloppy mode coercion / Annex B / sloppy direct-eval scope injection / var hoisting quirks / `arguments.callee` `.caller` / `with` 文 / `__proto__` accessor / RegExp legacy / 文字列HTMLメソッド) を実装 → **CRIT/IMP** (compat plugin 領域に移管 OR drop)
- `[plan]` plan-memo で **direct/indirect eval 区別 / sloppy mode flag / Annex B 専用 dispatch path / var hoisting quirks 専用 opcode** 等の motivation で defer slot 立てる案 → **IMP** (core では不要、compat plugin 着手まで slot 不要)
- `[plan]` plan-memo の `FrameKind` / opcode / dispatch path 拡張案が "direct/indirect eval semantic 区別" "sloppy this auto-boxing" 等 LegacySemantics 機能を motivation にしている → **IMP** (core scope 外、`memory/reference_elidex-js-core-strict-only.md` 参照)

**Acceptable exceptions (1b)**: motivation が "compat plugin 設計時の hand-off note" として明示されている (= "本 slot は LegacySemantics plugin 着手時に再評価" と書かれている) → FP-allowed

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <sub-check 1a/1b short label> in <context>
  Suggested fix: <which engine-indep crate API OR new helper, OR compat-plugin hand-off>
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
- **per-entity 状態を entity-keyed side-store/registry (HostData `*_cache` / `*Registry` / `HashMap<entity, _>`) に持たせる** で `Send + Sync` かつ per-VM identity handle でない → ECS component が正 (CLAUDE.md § "Side-store→component 判定ルール")。boa `HostBridge` side-store の VM 直訳が典型罠 (D-21 R3 CRIT)。**例外 (FP-allowed)**: (a) per-VM identity handle (VM `ObjectId` 保持の wrapper-cache 群 ~25 — `world_id` discriminator 解禁まで HostData が正) / (b) shared cross-cutting state (cookie jar / `NetworkHandle` 等 per-entity でない session 資源)。詳細 = CLAUDE.md 参照
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
- `[both]` **edge-dense work** (≥3 intersecting invariant axes を束ねる / 正準アルゴリズムが無い subsystem を触る) を **単一 PR** に束ねている → **IMP** (CLAUDE.md "Edge-dense work = multi-PR program" rule、judgment でなく rule、#339 incident 由来)。`[plan]`: plan が umbrella plan + per-PR plan 分割を宣言していない (Required action = 分割し plan-review をかけ直す)。`[diff]`: PR diff が edge-dense なのに分割 (umbrella program の per-PR slice) の痕跡が無い — **#339 の失敗 mode が「plan-review skip」ゆえ diff gate が最後の砦** (plan gate を skip した bundled PR が final design gate を素通りするのを止める。Required action = umbrella + stacked per-PR に再構成、不可なら retro plan-review + lesson 記録)。**Base case 除外 (両 context で fire しない)**: 承認済 umbrella program の per-PR slice であることが明示され scope が単一 invariant-axis 交点に絞られている場合 (terminal 単位 = 許容される単一 PR)
- `[diff]` **docstring-contract ↔ body 違反** — `Result`-返す host native (特に promise-returning `*_outcome` family) の body が backend/IO `Result` を `unwrap_or(_)` / `unwrap_or_default()` / `.ok().flatten()` で握り潰すが、file/fn docstring か同 file の sibling op (`open`/`put` 等) が「failure → reject/propagate」契約を明示 → silent-wrong-on-error + self-inconsistency。**Required action**: 契約通り `?`-propagate + `map_err`。(#275 R1 = cache read 8 op; bar = generalizable[promise-returning native family 横断] かつ grep-detectable[`*_outcome` 終端の `Result` swallow] ゆえ axis 化)

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <marker>: "<excerpt>"
  Required action: (a) 該当 spec gap に defer slot 立てて引用 OR (b) 本 PR / plan 内で proper 実装 OR (c) inline TODO 削除 + accept-as-is justify
```

---

## Axis 4 — Spec citation discipline

**Purpose**: 新規 VM accessor / DomApiHandler / native fn / engine-indep algorithm の docstring に WHATWG / HTML / DOM / URL / CSS / ECMAScript / WebIDL / Fetch / Streams / IndexedDB 等の section/step citation。Spec-faithful 実装の trace 可能性。

**Reference**: CLAUDE.md § "Design philosophy" (spec-faithful, html5ever 非依存) / 各 spec docs

### Verification tool

Citation 整合 (§number ↔ title)・WebIDL fragment・dfn anchor・AO 名 → 正規 §number・algorithm prose の確認は `.claude/tools/webref` を使う (subcommand 一覧 + cache + WebFetch との使い分けは `.claude/tools/webref --help`、CLAUDE.md § "Spec citation" も同 tool を mandate)。

要点だけ:
- **ECMA-262** は section 番号が版間で不安定 → `aoid <spec> <name>` で AO 名から正規 §number を引き直す。`heading` は従。
- **WHATWG/W3C** は `heading <spec> <number>` で §number ↔ title 確認、term 引きは `dfn <spec> <term>`。
- **Algorithm prose** 一致確認は `body <spec> <anchor-or-AO>` (multipage chapter cache、truncate なし)。
- WebFetch は spec HTML truncate するので citation/prose 確認には不向き — webref 一択。

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
- `[plan]` **citation-drift sweep PR の initial-audit method 文書化** (citation-sweep authoring discipline、`feedback_citation-sweep-audit-comprehensive.md` #229): plan-memo が *citation drift sweep* (§-number / AO 名 / spec-name の一括修正) を扱うのに initial audit が narrow single-pattern grep のみだと、R-loop で audit-incompleteness が連続発覚し scope が 2x-4x に膨張 + R 数が伸びる。plan-memo §audit に以下が揃っていなければ **MIN**: (1) **真値先確定** = 修正対象 concept を `webref dfn <spec> <term>` で逆引きして正規 anchor + 包含 §heading を取得 (例 `dfn html 'reaction queue'` → §4.13.6)、(2) **≥4 grep pattern** = spec-name 接頭辞付き / 接頭辞無し (`Observer §X.Y`) / synonym・文脈付き / non-existent-section probe (`heading <spec> X` で存否確認)、(3) **engine-indep crate も対象** (`dom/` `api/` `core/` `tests/`、`vm/host/` だけでない)、(4) 各 pattern の件数明記。これは本 axis の "Verification tool" *machinery* (`dfn`/`aoid`、個別 cite の §↔title 検証) の **authoring-discipline twin** = 同軸・別 facet (sweep audit の網羅性)、phase は *initial discovery* (rename 後の propagation = workflow.md Step 1.6、別物)

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
  - **Production-completeness sub-facet** (`feedback_existing-infra-production-completeness-premise.md`, #383): plan が work を「**trivial producer / thin wiring on complete existing infra**」と framing する時、"complete existing infra" は existence でなく **PRODUCTION-completeness** の検証を要する premise。infra の caller/test を grep し (a) multi-consumer fan-out (b) paint/refresh trigger (c) coordinate-system / unit consistency (d) full lifecycle を覆うか確認 — 唯一の exercise が **single-instance synthetic-driver test** なら、その "trivial producer" は実は **edge-dense subsystem** に乗っており diff-local + plan review では density が不可視 (両者は producer の局所正当性 + engine-side data-flow を見て shell/integration coordinate-system を見ない)。premise が existence-only 検証なら **IMP**。Canary (mid-loop で reactive fix が invariant を壊したら corner-fix 継続でなく STOP + re-plan) は external-converge overlay の generator-layer check が既に enforce — そちら参照
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

Axis 番号は安定 (SKILL.md / 5-agent split が依拠)。新 sub-check 追加時はインライン引用 (例: Sub-check 2b は 2026-05-19 D-29 trial 由来)。

### Future expansion candidates (field-test 待ち、premature な追加は避ける)

- **Sub-check 2c — New ECS component data-flow (read/write balance)**: 新規追加 ECS component の read site / write site 両側存在確認。D-29 trial で必要性未確認、実需 PR 発生時に flesh out (具体 detect 手段 + plan-memo `§Components added` template 要件)。
- **Test coverage discipline axis (Axis 6 candidate)**: engine-indep test と JS-level test の covering matrix 検証。`/elidex-review` 運用後 false positive 率を見て要判断。
