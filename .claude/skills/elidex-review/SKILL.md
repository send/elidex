---
name: elidex-review
description: elidex project-specific pre-push design review (5-agent parallel). Checks Layering mandate / ECS-native lens / pragmatic shortcut detection / spec citation discipline / project-context integrity beyond what generic /simplify + /review cover. Run BEFORE `git push` to compress Copilot R-loop.
user-invocable: true
---

# elidex-review — project-specific pre-push design review

`/simplify` (code reuse / quality / efficiency) と `/review` (一般的 PR review) **の上に重ねる** elidex 専門 review skill。Pre-push triple-pass の 3 段目として走らせて Copilot R-loop で flag される前に違反を検出する。

**Reference**: `CLAUDE.md` の `Design philosophy` + `Design discipline` + `Layering mandate` section、`memory/feedback_ideal-over-pragmatic.md`、`memory/feedback_objectkind-resolution-uniformity.md`、`memory/m4-12-architectural-drift-incident.md`、`memory/m4-12-platform-gap-roadmap.md`。

## When to invoke

- **Pre-push 必須段** (cargo fmt + mise run ci + /simplify + /review + **/elidex-review**): 全 elidex PR で実施推奨
- 既存 `/simplify` + `/review` だけでは elidex-specific design 原則違反が漏れる ([feedback_ideal-over-pragmatic] + Layering mandate 違反は generic review skill では捕捉不可)

## Skip OK

- doc-only PR (no src changes) → 各 agent が trivially 0 finding になる可能性高、明示的 skip 可
- diff < 30 LoC かつ既存 pattern の minor extension のみ → judgment skip 可、ただし PR landing memo に skip 理由明示

## Workflow

### Step 1 — Collect diff

```bash
git diff main > /tmp/elidex-review.diff
git diff main --stat > /tmp/elidex-review.stat
wc -l /tmp/elidex-review.diff
```

Diff size > 5000 行なら user 確認 (5-agent token cost 過大の可能性)。

### Step 2 — Launch 5 agents in parallel

5 agent を **同一 message 内 5 並列** Agent tool call で起動 (sequential 起動 NG、~5x slower)。各 agent に diff path + relevant CLAUDE.md / memory section path を prompt で渡す。

#### Agent 1 — Layering mandate review (`crates/script/elidex-js/src/vm/host/` engine-bound only)

**Reference**: `CLAUDE.md` § Layering mandate (2026-05-04 incident 由来)、`memory/m4-12-architectural-drift-incident.md` (PR #151 incident = 4 rounds × 17 IMP findings before drift detected)

**Detect**:
- 新規 / 拡張された `vm/host/*.rs` 内に 10+ LoC loop / walker / state machine / coercion algorithm
- `EcsDom::traverse_descendants` / `find_by_id` / `with_attribute` 等の direct call で marshalling 用途を超えるもの
- 新規 fn が DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation 等の algorithm を host/ で実装している
- 既存 engine-indep crate (`elidex-dom-api` / `elidex-form` / `elidex-css` / `elidex-script-session::DomApiHandler`) に類似 API があるか grep 確認、あれば直 call ではなくそちら経由を推奨

**Output format**:
```
[CRIT|IMP|MIN|FP] vm/host/<file>:<line> — <violation summary>
  Suggested fix: <which engine-indep crate API to use OR new helper to add>
```

#### Agent 2 — ECS-native lens review (OO browser engine pattern 直訳防止)

**Reference**: `CLAUDE.md` § Design philosophy "ECS-native first"、`memory/feedback_ideal-over-pragmatic.md`、`memory/feedback_objectkind-resolution-uniformity.md`

**Detect**:
- 新規 trait 定義に OO observer pattern marker (`*Observer` / `*Listener` / `*Subscriber` で `Vec<Box<dyn>>` registry shape)
- Inheritance hierarchy 風 trait + single concrete impl (= over-abstracted、ECS では component pattern が natural)
- `ObjectKind` variant 追加で **state shape が unique でないもの** (ECS component で識別可能なら HostObject + brand-check 統一が原則、lesson #276)
- Class-owned mutable state を struct member field で持たせている箇所 (ECS では Component-on-Entity が natural)
- OO patterns の ECS idiom 翻訳が抜けている (例: subscriber registry → marker component + system query、observer fan-out → version counter + lazy detection、virtual dispatch → typed entity ECS query)

**Acceptable exceptions**:
- VM 内部 (engine 自体の implementation = boa_engine, hecs などの low-level crate) は ECS-native 適用対象外
- Legacy code 変更無しの存在は flag しない (新規追加 OR 大幅変更のみ対象)

**Output format**:
```
[CRIT|IMP|MIN|FP] <file>:<line> — <OO pattern detected> in <context>
  ECS-native alternative: <suggested component / system / query / marker pattern>
  Reference: <lesson # or feedback memory>
```

#### Agent 3 — Pragmatic shortcut detection ([feedback_ideal-over-pragmatic] enforcement)

**Reference**: `CLAUDE.md` § Design philosophy "Ideal over pragmatic"、`memory/feedback_ideal-over-pragmatic.md`、`CLAUDE.md` § Design discipline "TODO 先送り禁止" + "後方互換性は維持しない"

**Detect** (diff 内 + new files で grep):
- `// stub:` / `// minimal v1` / `// for now` / `// quick fix` / `// temporary` / `// placeholder` 等の pragmatic shortcut marker
- `unimplemented!()` / `todo!()` macro 追加 (existing call は除外)
- `TODO` / `FIXME` 追加で defer slot 番号 (`#11-*`) の citation が無いもの
- "later" "future PR" "M4-13" 等の deferral 言及が **defer slot 引用無しで** code comment に書かれているもの
- Shim / fallback / backwards-compat wrapper 追加 ("// legacy" / "// fallback for old API" 等)
- Test 内の `.unwrap()` で適切 error context 無く placeholder 風になっているもの

**Output format**:
```
[CRIT|IMP|MIN|FP] <file>:<line> — <pragmatic shortcut marker>: "<excerpt>"
  Required action: (a) 該当 spec gap に defer slot 立てて引用 OR (b) 本 PR 内で proper 実装 OR (c) inline TODO 削除して accept-as-is で justify
```

#### Agent 4 — Spec citation discipline

**Detect** (新規 VM accessor / DomApiHandler / native fn / engine-indep algorithm 追加箇所で):
- WHATWG / HTML / DOM / URL / CSS / ECMAScript / WebIDL / Fetch / Streams / IndexedDB 等の spec citation が **docstring に無いもの**
- 既 citation あっても section/step 数の syntax (`§4.2.6 step 3.1`) が不正 / 一貫性無いもの
- "per WHATWG" / "per HTML" の prose だけで section 番号無いもの (= 検証困難)
- Algorithm 実装に WHATWG step-by-step の reference 無く独自実装 (spec drift risk)

**Output format**:
```
[CRIT|IMP|MIN|FP] <file>:<line> — <function/handler name> missing/incorrect spec citation
  Expected: "WHATWG <spec> §<section> <step>" or equivalent
  Suggested wording: <one-line citation>
```

#### Agent 5 — Project-context integrity

**Reference**: `memory/MEMORY.md` § Active state、`memory/m4-12-platform-gap-roadmap.md` § D-tier3、active plan-memo (もし存在すれば)

**Detect**:
- Plan-memo / landing-memo にある defer slot 番号 (`#11-*`) が code citation で正しく引用されているか
  - 例: docstring `defer slot #11-foo-bar` が memory ledger で actually open 中の slot か verify
  - 旧名 slot ref (e.g., 既 merged で名前変更されたもの) を引用していないか
- §D-tier3 phase plan / MEMORY.md `## Active state` との整合 (本 PR の scope が plan 通りか、scope creep が起きていないか)
- 過去 lesson (`#XXX`) 違反 (e.g., lesson #235 trait extension cascade を再発させてないか、lesson #276 ObjectKind variant 追加 over-use)
- 本 PR と **同時並行で他 PR が変更している可能性のある file** で convention drift (例: 同 file の sibling code が既に異なる pattern を採用している)
- 過去 PR の R-loop で flagged & resolved した anti-pattern を再導入してないか (recent landing memos の "Lessons" section 参照)
- **1000-line file growth awareness**: bash `wc -l <changed-files>` で本 PR が source file を 1000 行超えに pushed したか OR 既 1000+ file に substantive add (>50 LoC) を入れたか確認。Surface as **MIN finding** (gate NOT) — author 判断で (a) 本 PR 内で split (明確 seam ありかつ split が design 改善になる場合) OR (b) 既存の periodic sweep tranche に委ねる (split shape に file 全体 + 周辺 PR の文脈必要な場合) を選ぶ。1000 行は magical threshold ではなく "focus 霧散の proxy signal"、author の design judgment 優先。Past tranches: MEMORY.md "1000-line file cleanup tranches 1-3" 参照。

**Output format**:
```
[CRIT|IMP|MIN|FP] <file>:<line> — <project-context violation>
  Reference: <memory/<file>:<line> citation>
  Recommended action: <fix or scope adjustment>
```

### Step 3 — Aggregate + present

5 agent 完了後、findings を集約して以下 format で出力:

```markdown
## /elidex-review summary

| Agent | CRIT | IMP | MIN | FP |
|---|---|---|---|---|
| 1 Layering mandate | N | N | N | N |
| 2 ECS-native lens | N | N | N | N |
| 3 Pragmatic shortcut | N | N | N | N |
| 4 Spec citation | N | N | N | N |
| 5 Project context | N | N | N | N |
| **Total** | N | N | N | N |

## Findings (severity 順)

[per-finding 表示、agent label + severity + 該当 file:line + summary + agent's *raw* suggested fix]

⚠️ **Do NOT yet recommend which fix to apply.**  Agent-suggested fixes are raw input only at this step; the philosophy-aligned fix proposal is composed in Step 3.5 below before the user-confirmation gate.  Treating the agent suggestion as the canonical fix at Step 3 (the smallest-patch bias) is the very behavior this skill exists to counteract.

## Recommendation

- CRIT findings: fix BEFORE push (push しても Copilot R で必ず flag される)
- IMP findings: push 前 fix 推奨 (Copilot R で 80% flag 確率)
- MIN findings: judgment call (defer も可、但し landing memo で justify)
- FP findings: agent 過剰反応、user 確認後 ignore

## Pre-push gate

- 0 CRIT + 0 IMP → push 推奨
- 1+ CRIT → push 前 fix mandatory
- 1+ IMP → user 判断 (fix or defer with justification)
```

### Step 3.5 — Philosophy alignment re-evaluation

⚠️ **MANDATORY before Step 4.**  Findings aggregation has a strong "smallest patch" bias — agents surface symptoms with polish-level suggested fixes (`const X = "literal"` / rename / accept-as-is).  CLAUDE.md "ideal over pragmatic" demands structural-level fixes when available.

For each **fix-tier** finding — `CRIT`, `IMP`, and `MIN` (i.e. every severity except `FP`, which is already excluded from fix consideration) — record this chain-of-thought BEFORE composing the user-facing fix proposal:

1. **What does CLAUDE.md philosophy demand here?**
   - "ideal over pragmatic" — ECS first principles 起点
   - "dead code は接続するか削除" — connect-or-delete (NOT keep-for-future)
   - "後方互換性は維持しない" — no shims, no version bridges
   - Lesson #276 ObjectKind resolution uniformity (ECS component + brand-check 統一)

2. **Is the agent's suggested fix symptom-level or root-level?**
   - Symptom: rename / const-extract / doc-comment / accept-as-is / debug_assert
   - Root: drop dead code / replace with existing abstraction / use ECS-native pattern / restructure caller
   - Default to root unless concrete cost overrides (test-churn scope creep / large refactor).

3. **Can one structural fix subsume multiple findings?**
   - E.g., dropping a dead component (F5) can subsume the short-circuit perf finding that touched it (F6); replacing a private helper with an existing public API (F3) can subsume pragmatic-shortcut findings on its callers.
   - Look for cross-finding root cause before fixing each in isolation.

4. **Polish-domination smell**: if your fix-option list is mostly polish (rename / const / doc / accept-as-is) and no structural option appears, **suspect the framing**.  Re-read the finding through ECS-native + ideal-over-pragmatic lens; the structural answer may not be in any of the agent's suggested options.

**Pattern source**: 2026-05-19 D-31 PR trial-run #1 of /elidex-review.  F10 (stringly-typed "href") initially listed 3 polish options (const HREF / predicate / accept-as-is) — user pushed back twice ("Design philosophy 考慮すると?" → "他の項目も再考する必要は?") and full re-evaluation produced ~8 fix-decision reversals from polish to structural (drop dead field / fix template carve-out in-PR not defer slot / explicit early-return guard not debug_assert / ECS hygiene cleanup not marker component / etc.).  Memory: [`feedback_review-fix-philosophy-first.md`].

### Step 4 — User confirmation

Findings 提示後、user に:
- 「全 fix で進めますか?」
- 「特定 finding は accept-as-is (= FP / 判断保留) でいいですか?」
- 「push にどう影響しますか? (CRIT 残してでも push したい場合 escalation)」

を確認。Auto-fix はしない (user 判断 driven)。

## Severity calibration

[feedback `copilot_review_stop`] と整合:
- **CRIT** — merging で immediate damage / spec contract 違反 / Layering mandate 違反 (incident 由来) / build break
- **IMP** — fix-or-something-tangible-breaks: design 原則違反 (ECS-native lens / ideal-over-pragmatic) で observable consequence あり、CRIT じゃないが Copilot R で必ず指摘される候補
- **MIN** — preferable but no concrete consequence: 軽微 style / 雑然
- **FP** — agent 過剰反応 / context 不足 / 既 design 決定 / user 既明示意図あり

Pragmatic shortcut detection は marker presence で auto-flag = false positive 可能性高 (legitimate "// TODO: defer-slot #11-foo (re-eval 2026-Q4)" 等は適切な inline citation で許容)、agent 内で citation pattern 確認推奨。

## Anti-patterns (skill 自体の運用注意)

- **5-agent 同時起動必須**: sequential では ~5x slow、tool call message 内に全 5 個 Agent 含める
- **Auto-fix NG**: skill は detection のみ、修正は user 判断 driven (`/simplify` も同 pattern)
- **Generic /review との重複避ける**: `/review` (built-in) は一般 PR 観点、本 skill は elidex 専門 axis 限定。重複指摘なら本 skill の finding 優先 (より context-aware)
- **/simplify と相補的**: `/simplify` は reuse / quality / efficiency、本 skill は design 原則 / project context、cover axis 異なる

## Future expansion

実 PR で運用後 false positive 多い agent / 抜けてる axis があれば調整:
- Agent prompt の例 (`Detect` list) を増減
- 新 axis 追加検討 (例: `Agent 6 — Test coverage discipline` = engine-indep test と JS-level test の covering matrix 検証)
- Severity calibration の re-tuning

skill 自体の改善は本 SKILL.md 直接 edit (project-local = git tracked、team 共有可能)。

### Pre-impl review skill 化 (staged path)

Plan-memo / pre-impl 段階にも同 5 axis 適用したいが、drift 防止のため axis 定義切出し前提。順序固定:

1. **本 skill 試運転 2-3 PR** — false positive / 抜け axis 観察、SKILL.md tune
2. **`axes.md` 切出し refactor** — 5 axis 定義 (Purpose / Reference / `Detect at plan-memo` / `Detect at diff` / Output format / Severity calibration) を `axes.md` に SSoT 化。本 SKILL.md は thin lifecycle wrapper 化 (input = git diff、agent prompt は `axes.md` Read + `Detect at diff` subsection 適用指示)
3. **`elidex-plan-review/` 新設** — `Read ../elidex-review/axes.md` する thin wrapper、input = plan-memo path、agent prompt は `Detect at plan-memo` subsection 適用

順序固定の理由: axes.md 切出しを pre-impl skill 化の **前** にやれば、2 skill 間 axis drift が構造的に潰せる (single source of truth + thin wrapper)。axis 自体が未 validated な現状で先に切出すと早すぎる refactor で出戻り risk。
