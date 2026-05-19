# elidex review workflow — SSoT

Shared lifecycle for `elidex-review` (post-impl diff) and `elidex-plan-review` (pre-impl plan-memo).  Each skill's `SKILL.md` is a thin wrapper providing:

- Input collection (Step 1 — `git diff` vs plan-memo path)
- Step 2 agent invocation table (axis × axes.md `Detect` entries tagged `[diff]`/`[plan]`/`[both]` × dry-run file path)
- Recommendation phrasing specific to that gate (push gate vs implementation gate)
- Skill-specific extras (e.g., plan-review Step 5)

This file owns the rest: Step 1.5 methodology / Step 3 aggregation / Step 3.5 philosophy alignment / Step 4 confirmation / anti-patterns / change log.

## Step 1.5 — Mental dry-run (mandatory)

Axis 2 sub-check 2b (component data-flow integrity) detects gaps invisible to plain text scan.  Scenario simulation is the only detection vector.  **Skipping makes Step 2 Agent 2 ineffective for sub-check 2b.**

For each **new/changed test case** AND **new code path that reads ECS components** (refactor PR の new caller / new system query 等も対象 — test に限らない) in the input:

1. Mental-execute setup line by line.
2. At each line, identify ECS components / IDL properties / VM state being read.
3. For each read, identify the write-path that populates it (IDL setter / `setAttribute` / parser path / `MutationDispatcher` consumer / bulk-init hook).
4. Verify the write-path is wired by the change (or pre-existing).  If a step assumes state populated by an unwired mechanism, record:

   ```markdown
   ## <scenario identifier> dry-run

   - Read assumption: `<component>.<field>` after `<step>`
   - SoT mutation paths exercised by scenario: <list>
   - Write paths wired by change / existing impl: <list>
   - **Gap**: <description>
   ```

   `<scenario identifier>` は test の場合 `<file>:<test_name>` / non-test の場合 `<file>:<fn_name>` / plan-memo の場合 `plan §<section> §E-<N>` 等、scenario を一意に指す identifier を入れる。

5. Output to `<dry-run-file>` (path specified per skill — `/tmp/elidex-review.dry-run.md` or `/tmp/elidex-plan-review.dry-run.md`).
6. Step 2 Agent 2 prompt receives `<dry-run-file>` path and incorporates gaps into Sub-check 2b findings.

## Step 3 — Aggregate

After 5 agents return, emit the summary in this exact format:

```markdown
## <skill-name> summary

| Agent | CRIT | IMP | MIN | FP |
|---|---|---|---|---|
| 1 Layering mandate | N | N | N | N |
| 2 ECS-native lens | N | N | N | N |
| 3 Pragmatic shortcut | N | N | N | N |
| 4 Spec citation | N | N | N | N |
| 5 Project context | N | N | N | N |
| **Total** | N | N | N | N |

## Findings (severity 順)

[per-finding: `F<N>` ID + agent label + severity + file:line or §section + summary + agent's *raw* suggested fix]
```

### Finding ID rule

- Sequential monotonic counter `F1, F2, …` across ALL findings regardless of severity / source agent.
- Severity-sorted display order (CRIT → IMP → MIN → FP).
- `FP` IDs leave gaps in the fix-tier set (Step 3.5 emits `Fix decision` blocks ONLY for `CRIT`/`IMP`/`MIN`).  Example: with F1=IMP, F2=FP, F3=MIN → fix-tier list is `{F1, F3}`, not `F1..F3` (the range expression contradicts the gap).
- Subsumption refs in Step 3.5 use these IDs (`Subsumption check: F1 + F5`).

⚠️ **Do NOT recommend fixes at Step 3.**  Agent suggestions are raw input only.  The philosophy-aligned fix proposal is composed in Step 3.5.  Treating the agent suggestion as canonical at Step 3 (smallest-patch bias) is exactly the behavior these skills exist to counteract.

## Step 3.5 — Philosophy alignment (mandatory before Step 4)

Agent suggestions skew polish-level (rename / const / doc / accept-as-is).  CLAUDE.md "ideal over pragmatic" demands structural-level fixes when available.

For each fix-tier finding, produce a user-visible decision record using these prompts:

1. **What does CLAUDE.md philosophy demand here?** — ideal over pragmatic / dead code 接続 or 削除 / 後方互換性 NG / TODO 先送り禁止 / lesson #276 ObjectKind uniformity.
2. **Symptom vs root?** — Symptom: rename / const / doc / accept-as-is / debug_assert.  Root: drop dead code / replace with existing abstraction / use ECS-native pattern / restructure caller / **carve out prerequisite PR**.  Default root unless concrete cost overrides.
3. **Subsumption check?** — Can one structural fix close multiple findings?  Look for cross-finding root cause before fixing each in isolation.
4. **Polish-domination smell?** — If your fix-option list is mostly polish with no structural option, **suspect the framing**.  Re-read through ECS-native + ideal-over-pragmatic lens.

Emit one block per fix-tier finding:

```
### Fix decision — F<N>: <one-line finding summary>

- **Demand (philosophy)**: <CLAUDE.md axis cited>
- **Agent suggestion**: <symptom-level | root-level> — <one-line excerpt>
- **Root-level alternative**: <structural fix OR "separate prerequisite PR" OR "none — agent suggestion already structural">
- **Subsumption check**: <other F<M> this fix resolves, OR "none">
- **Polish-domination smell**: <triggered? — if yes the alternative is the proposed fix>
- **Proposed fix**: <chosen, one sentence>
- **Concrete action**: <skill-specific — code-edit OR plan-memo edit OR prerequisite PR carve-out>
```

The aggregate of these blocks IS the philosophy-aligned fix proposal.  Step 4 references these IDs.

**Pattern source**: 2026-05-19 D-31 PR trial of /elidex-review.  F10 initially listed 3 polish options; user pushed back twice and full re-evaluation produced ~8 polish→structural reversals.  Memory: `feedback_review-fix-philosophy-first.md`.

## Step 4 — User confirmation

**Zero fix-tier short-circuit**: if CRIT/IMP/MIN all 0 (FP only or no findings), skip Step 3.5 block emission; confirm "0 fix-tier findings; FP <N> 件 ignore で進行可" and exit.

**≥1 fix-tier**: reference the concrete fix-tier ID list (e.g. `F1, F3, F4` when F2 is FP — NOT `F1..F4`).  Ask:

- 「全 Fix decision (`F<list>`) で進めますか?」 (apply every block's Proposed fix)
- 「特定 Fix decision `F<X>` は accept-as-is でいいですか?」 (override individual block)
- 「gate にどう影響しますか?」 (push gate / impl gate / escalation if CRIT held)

Auto-fix NG — user decision drives.  ≥1 fix-tier without Step 3.5 block = gate violation, return to Step 3.5.

## Anti-patterns (common to both skills)

- **5-agent 同時起動必須**: sequential = ~5x slow.  All 5 Agent tool calls in a single message.
- **Self-review ≠ multi-agent review**: 1 agent / inline self-evaluation skips parallel independent perspectives → single-perspective blind spot remains (2026-05-19 D-29 trial precedent: self-review missed Axis 2 sub-check 2b → 場当たり cascade).
- **Step 1.5 mental dry-run skip NG**: Sub-check 2b uniquely depends on it.
- **Auto-fix NG**: detection only; user-driven (matches `/simplify`).
- **Step 3.5 user-visible mandatory**: blocks cannot live as internal reasoning — Step 4 references IDs.
- **Skip post-Step-3.5 re-review when root fix changes input shape NG**: root fix で input (plan-memo の §section 構造 / diff の大幅 restructure) が変わったら再 review 必須。D-29 plan-review trial precedent — F3 root fix accepted, plan structure shifted, no re-review → Sub-check 2b data-flow gap still missed (両 skill 適用、diff review でも root fix 適用後の diff 変化を再 scan)。
- **Generic `/review` との重複避ける**: `/review` (built-in) は一般 PR 観点。本 skill 群は elidex 専門 axis 限定。重複指摘は context-aware な本 skill finding 優先。
- **/simplify と相補**: cover axis 異なる (reuse/quality/efficiency vs design/project-context)。

## Change log

- **2026-05-20** — Two skill brush-ups from D-29 PR #209 trial:
  - **B**: Step 1 of both SKILL.md gained `rm -f` of the dry-run output path (`/tmp/elidex-review.dry-run.md` / `/tmp/elidex-plan-review.dry-run.md`).  Triggered by stale-residue friction during D-29 R-loop: prior PR's dry-run sat at the fixed path, Write tool's "Read first" guard tripped the agent.  `rm -f` at Step 1 ensures a clean slate per invocation.
  - **C**: axes.md Axis 5 Detect first bullet (orphan defer slot citation) gained an explicit "Acceptable exception (FP, not IMP)" carve-out for slots whose plan-memo carries a quoted **ship-time registration commitment** (e.g. `D-N ship 時に登録`).  Triggered by recurring noise: `#11-form-navigation` was flagged as IMP by both `/elidex-plan-review` (pre-impl) and `/elidex-review` (pre-push) for D-29, despite the plan-memo explicitly scheduling slot registration at ship-time.  Pre-agreed admin debt should fold into Step 3.5's landing-memo reminder, not gate the push.
- **2026-05-19** — Initial extraction (axes.md + workflow.md SSoT).  Triggered by D-29 plan-review trial-run failure: self-review (single perspective, inline) missed Axis 2 sub-check 2b component data-flow integrity → IDL setter patches + dropped tests + TODO punt 場当たり cascade.  Structural fix: (a) axes.md SSoT, (b) Axis 2 sub-check 2b added explicitly, (c) Step 1.5 mental dry-run mandatory, (d) plan-review skill created consuming same axes.md, (e) workflow.md SSoT extracted to dedupe both SKILL.md.
- **2026-05-18** — `/elidex-review` skill initial creation + D-31 PR trial-run (Step 3.5 philosophy-alignment block added after user pushback on polish-dominated F10 fix options).
