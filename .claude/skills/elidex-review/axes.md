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

**Purpose**: `vm/host/*` は engine-bound 責務のみ。DOM algorithm は `elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler` 経由。

**Reference**: `CLAUDE.md` § "Layering mandate (2026-05-04 incident 由来)" / `memory/m4-12-architectural-drift-incident.md` (PR #151: 4 R-loop × 17 IMP findings before drift detected)

### Detect

- `[both]` `vm/host/*.rs` に 10+ LoC の loop / walker / state machine / coercion algorithm を新規追加または拡張 (diff: 該当行、plan: §Body/§Implementation 記述)
- `[both]` `EcsDom::traverse_descendants` / `find_by_id` / `with_attribute` 等の direct call が marshalling 用途 (entity 取得 / 単純 attribute read / wrapper 生成) を超える
- `[both]` 新規 fn が DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等を `vm/host/` で実装
- `[both]` 既存 engine-indep crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) に類似 API があるかの確認・経由明示の有無
- `[plan]` plan §"Layering check" / §"Architecture" に既存 crate API への mapping 表が欠落 (MIN)

**Acceptable exceptions**: engine 自体 (boa_engine / hecs 等 low-level) / legacy code 変更無し

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

### Detect

- `[both]` Spec citation が docstring (diff) / plan の新規 fn 説明 (plan) に無い
- `[both]` 既 citation あっても section/step syntax (`§4.2.6 step 3.1`) が不正 / 一貫性無し
- `[both]` "per WHATWG" / "per HTML" の prose だけで section 番号無し
- `[both]` Algorithm 実装 (diff) / 実装計画 (plan) に WHATWG step-by-step reference 無く独自実装 (spec drift risk)
- `[plan]` plan §"Docstring requirement" / §"Spec citation table" 欠落 (新規 fn × spec citation 対応表が plan 内に無いと implementation で抜け落ちる)

### Output format

```
[CRIT|IMP|MIN|FP] <file:line | plan-memo §section> — <function/handler name> missing/incorrect spec citation
  Expected: "WHATWG <spec> §<section> <step>" or equivalent
  Suggested wording: <one-line citation>
```

---

## Axis 5 — Project-context integrity

**Purpose**: `MEMORY.md` `Active state` / `m4-12-platform-gap-roadmap.md` / active plan-memos / 過去 lesson 整合性。Defer slot reference 整合、resolved anti-pattern 再導入検出、1000-line file growth awareness。

**Reference**: `memory/MEMORY.md` § "Active state" / `memory/m4-12-platform-gap-roadmap.md` § "D-tier3" / recent landing memos (`memory/m4-12-pr-*-landing.md`) の "Lessons" section

### Detect

- `[both]` 引用 defer slot 番号 (`#11-*`) が memory ledger で actually open 中か (旧名 slot ref 引用なし、recent landing memo で renamed/closed されていないか)
- `[both]` §D-tier3 phase plan / `MEMORY.md` `Active state` との整合 (scope creep / batch 違反検出)
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
