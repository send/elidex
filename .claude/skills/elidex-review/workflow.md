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

## Step 4.5 — Fix-delta re-verification (classify every fix; re-check is conditional)

**The gate detects the *original* input; the *fix* it produces is itself unverified.**  A review proposes a fix (Step 3.5), the orchestrator applies it — and nothing independently re-screens the applied fix.  Self-applied re-checks fail here: the orchestrator is biased toward converging.  This is where manual push-back keeps landing (especially plan-stage, where a "fix" is a one-line *design decision* with the widest blast radius and the cheapest re-read).

**Classification is mandatory for every applied fix; the re-check/re-derivation it triggers is conditional** (clerical fixes classify-then-stop).  Two **orthogonal** triggers screen each fix.  Trigger A screens the *fix's mechanics*; Trigger B screens the *finding's framing* (a surface fix to a real design matter often looks clerical under A — B catches it).

### Trigger A — fix tier (mechanics of the fix)

Classify each applied fix (grey cases → resolve to the *higher* tier; a false-positive re-read is cheaper than a missed drift):

| Tier | Definition | Action |
|---|---|---|
| **clerical** | citation / wording / section-number / comment / cfg-gate / scope-doc — no behavior or design change | apply, no re-review |
| **design-affecting** | touches a **type / data structure / invariant / algorithm / control-flow / scope (defer・prereq) / premise** — localized | **focused re-check** |
| **structural** | changes the input *shape* — plan §section restructure / diff-wide restructure | **full re-review from Step 1** (existing anti-pattern rule) |

**Focused re-check** (cheap, independent — precedent: wrapper-identity-seam "focused Axis 2 re-review = 0/0/0"): only the axis the fix touches × only the changed hunk, a *fresh* sub-agent that did not author the fix, detect-only.  Question: *"is this fix clean on its axis?"*  Zooms **in**.

### Trigger B — symptom-shaped finding (framing of the finding) ⚠ catches the surface-fix-to-design-matter case

A finding is **symptom-shaped** if it prescribes a *local mechanism* ("add a guard / null-check / sort / cast", "handle the empty case", "fix this message") rather than naming the *root defect*.  Responding to it literally produces a surface patch that **looks clerical under Trigger A and would skip** — yet the root may be a design matter elsewhere (the state shouldn't be reachable; a missing abstraction; an invariant that should hold by construction).

When a fix responds to a symptom-shaped finding, run a **root-cause re-derivation** *regardless of its Trigger-A tier* (this **overrides clerical-skip**):
- **Scope**: NOT the changed hunk — the **surrounding design** the symptom lives in (root causes usually sit *outside* the hunk; that's why the surface fix was reachable).  Zooms **out**.
- **Independence + adversarial brief**: a fresh sub-agent told *"the review framed this as `<finding>` and the applied fix was `<fix>`; ignore that framing — what is the root cause, and is this hunk even the right place to fix it, or should the invariant hold by construction upstream?"*
- **Detect-only**: a root-level alternative → surface as a new Step 3.5 Fix decision (orchestrator + user choose root-fix vs. accept-surface-with-explicit-justification).

> v1 heuristic (calibrate with use): when unsure whether a finding is symptom-shaped, run B.  Process can't fully kill the symptom-vs-root miss (root scope is context-dependent, the fixer is convergence-biased) — B reduces it via independence + zoom-out + anti-framing.

### Placement (blast-radius-weighted, NOT uniform)

- **plan-review**: re-check each design-affecting / symptom-shaped fix **immediately** — plan fixes *compound* (later plan decisions build on the corrected one).
- **elidex-review (diff) / copilot-review**: batch into **one cumulative pass at the end** (pre-push / pre-merge) — code fixes are more independent and the irreversible step is the natural gate.

When every fix is clerical under A **and** none answered a symptom-shaped finding under B, the **re-check/re-derivation is skipped** — classification itself is always done (that is how you established the fixes are clerical).

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

- **2026-05-23** — **Step 4.5 Fix-delta re-verification** added (+ wired into all three gates' SKILL.md / copilot overlay `fix_discipline`).  Triggered by a recurring observation across PR #222 and prior work: the gates detect the *original* input but the *applied fix* is never independently re-screened — manual push-back kept landing on review-fixes, **especially plan-stage** (widest blast radius).  **Two orthogonal triggers**: **A (fix mechanics)** — clerical / design-affecting / structural; design-affecting → cheap **focused** re-check (touched axis × hunk, fresh detect-only agent; generalizes the wrapper-identity-seam "focused Axis 2 re-review").  **B (finding framing)** — a *symptom-shaped* finding ("add a guard / sort", "handle X") answered by a literal surface patch that *looks clerical under A but masks a design matter*; → **root-cause re-derivation** (zoom **out** to surrounding design, fresh agent briefed to ignore the surface framing, overrides clerical-skip).  B was added same-PR after noticing A alone misses surface-fixes-to-design-matters (their root sits outside the hunk, so the narrow focused re-check is blind).  Placement **blast-radius-weighted** (plan = immediate; diff / Copilot = cumulative at push/merge).  Subsumes the prior self-assessed "input-shape-change → re-review" anti-pattern (now the "structural" tier).  v1 — calibrate B's symptom-shaped heuristic with use.
- **2026-05-20** — `elidex-review` Step 1 diff range switched 2-dot → 3-dot against a *freshened* base (`git fetch --quiet origin main`, then the base resolves to `origin/main` — falling back to a verified local `main`; `git diff <base>...HEAD`) + staleness preflight added.  Triggered by an observer-refactor PR review: a sibling event-handler PR merged to `main` mid-session while the observer branch (forked from the previous `main`) was under review.  Plain 2-dot `main..HEAD` reported the 13 base-only files as phantom "deleted" files, contaminating the 5-agent run (wasted, had to detect + rebase + re-run).  3-dot uses `merge-base($BASE, HEAD)..HEAD` = the branch's own changes (matching the GitHub PR diff), immune to the base advancing; it stays committed-only so the original working-tree-contamination guard is preserved.  Resolving `$BASE` against `origin/main` (fetched) — not a possibly-stale local `main` — keeps the diff and the `git merge-base --is-ancestor $BASE HEAD` preflight (non-blocking "branch is N commits behind $BASE — consider `git rebase`") accurate against *current* remote main (Copilot PR #214 R1 refinement).  (`elidex-plan-review` Step 1 takes a plan-memo path, not a diff, so it is unaffected.)
- **2026-05-20** — Two skill brush-ups from D-29 PR #209 trial:
  - **B**: Step 1 of both SKILL.md gained `rm -f` of the dry-run output path (`/tmp/elidex-review.dry-run.md` / `/tmp/elidex-plan-review.dry-run.md`).  Triggered by stale-residue friction during D-29 R-loop: prior PR's dry-run sat at the fixed path, Write tool's "Read first" guard tripped the agent.  `rm -f` at Step 1 ensures a clean slate per invocation.
  - **C**: axes.md Axis 5 Detect first bullet (orphan defer slot citation) gained an explicit "Acceptable exception (FP, not IMP)" carve-out for slots whose plan-memo carries a quoted **ship-time registration commitment** (e.g. `D-N ship 時に登録`).  Triggered by recurring noise: `#11-form-navigation` was flagged as IMP by both `/elidex-plan-review` (pre-impl) and `/elidex-review` (pre-push) for D-29, despite the plan-memo explicitly scheduling slot registration at ship-time.  Pre-agreed admin debt should fold into Step 3.5's landing-memo reminder, not gate the push.
- **2026-05-19** — Initial extraction (axes.md + workflow.md SSoT).  Triggered by D-29 plan-review trial-run failure: self-review (single perspective, inline) missed Axis 2 sub-check 2b component data-flow integrity → IDL setter patches + dropped tests + TODO punt 場当たり cascade.  Structural fix: (a) axes.md SSoT, (b) Axis 2 sub-check 2b added explicitly, (c) Step 1.5 mental dry-run mandatory, (d) plan-review skill created consuming same axes.md, (e) workflow.md SSoT extracted to dedupe both SKILL.md.
- **2026-05-18** — `/elidex-review` skill initial creation + D-31 PR trial-run (Step 3.5 philosophy-alignment block added after user pushback on polish-dominated F10 fix options).
