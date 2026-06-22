# /external-converge project overlay — elidex

Loaded by `~/.claude/skills/external-converge/SKILL.md` (the full multi-round convergence loop) when invoked from this repo. Provides project-specific calibration — `reviewer`, `wakeup_median`, `layering`, `fix_discipline`, historical `failure_modes`. Reviewer = **OpenAI Codex on ChatGPT Pro** (loop-affordable, no per-credit cost). For the routine single-pass, see `external-review/project.md` (same reviewer).

## repo

`send/elidex`

## build_verify

`cargo fmt --all && mise run ci`

Per CLAUDE.md "Push 前: mise run ci". cargo task は `--all-features` で gate されているので feature-gated code (`#![cfg(feature = "engine")]` 等) も含めて回る (CLAUDE.md "Workflow" 参照)。

## layering

Reference: CLAUDE.md § "Layering mandate (2026-05-04 incident 由来)".

### paths

- `crates/script/elidex-js/src/vm/host/`

### api_names

Beyond marshalling use of these APIs triggers downward drift signal:

- `EcsDom::traverse_descendants`
- `EcsDom::find_by_id`
- `EcsDom::with_attribute`

Acceptable marshalling use: prototype install / brand check / `JsValue` ↔ `Entity` marshalling / 単純 attribute read / wrapper 生成。NOT acceptable: DOM mutation / selector matching / form validation / live-collection walker / label association / constraint validation algorithm bodies inside `vm/host/`.

### incident_memo

`m4-12-architectural-drift-incident.md` (PR #151, 2026-05-04 — 4R × 17 IMP findings before downward drift detected; lesson #145).

## fix_discipline

Applied at SKILL.md Step 5.1 (fix planning) — per-fix lens. SSoT for both: `<repo>/.claude/skills/elidex-review/workflow.md` (Step 3.5 + Step 4.5).

**Step 5.1 (per-fix)**: apply Step 3.5 "Philosophy alignment" — symptom vs root through CLAUDE.md "ideal over pragmatic" + "設計優先" + "ECS-native first". The reviewer's obvious patch is usually the smallest symptom-fix (sort / guard / cast); prefer a structural fix where the invariant holds by construction. Polish-domination smell → re-derive. **Convergence rule**: real → fully fix (incl. real MINs — don't defer as "edge"); FP → reject. Stop on real-gap exhaustion, not round-count → `memory/feedback_review-loop-convergence-merit-not-fatigue.md`.

Precedent (PR #213, 2026-05-20): R2 flagged nondeterministic `HashMap` callback order; reactive patch was per-site `sort_by_key`. Philosophy-ideal was `BTreeMap` keyed by monotonic observer id — registration-order delivery as a *structural* invariant. Reactive patch shipped through TERMINAL and needed follow-up.

**TERMINAL fix-delta pass (Step 5.5, before surfacing merge proposal)**: apply workflow.md Step 4.5 over the cumulative R-loop delta (`git diff <first-R-loop-commit>^..HEAD`). External-review findings are frequently *symptom-shaped* ("add a guard", "handle this case") so **Trigger B is the acute external-review-specific risk** — Step 5.1's lens is self-applied (convergence-biased) and the R-loop delta never re-enters the pre-push `/elidex-review` philosophy gate. If either trigger fires for any R1..Rn fix, run one cumulative `/elidex-review` pass; new findings → resolve before merge. Code-stage delta is batched at merge (workflow.md "Placement" — code fixes are more independent than plan fixes; irreversible merge is the natural gate). **Skipping this TERMINAL pass** is justified only when every axis has expected yield 0 — argue it per-axis per the expected-yield=0 table in `elidex-review/SKILL.md` "Skip OK" (single home; not restated here).

**Also fire this pass EARLY on persistence — not only at TERMINAL (#396, `feedback_external-review-symptom-patch-accumulates-ungated`)**: a *divergent* loop never reaches TERMINAL, so a TERMINAL-only design re-gate never runs while an ad-hoc edifice accumulates round over round. When the SKILL.md Step-4 ≥2-round self-root-check fires (the *same mechanism* patched across rounds — SKILL.md "Fire the design re-gate HERE, not only at TERMINAL"), run this cumulative fix-delta `/elidex-review` pass **then**, and test the accumulated mechanism against the plan-memo's / CLAUDE.md's OWN stated ideal (e.g. "construction **input**, not reconciled after the fact" / *Concurrency by ownership* = ownership-transfer / single-writer / latest-value), not just abstraction-coverage. PR #396: an 8-finding / 5-round `drain/replay` *was* the reconcile-after-the-fact the plan-memo §2 forbade — a TERMINAL-only re-gate would never have fired (the loop was diverging) and the per-fix lens kept answering "the mechanism is necessary." Distinct from the #374 generator-layer check below (that is *altitude* — B1 detail in a B0 doc; this is a fix *mechanism* that is itself the anti-pattern).

**Generator-layer / altitude check (Step 4 PAUSE precondition — #374 incident, MANDATORY when corner-hopping)**: when the same finding-*shape* recurs across ≥3 rounds AND **hops between sub-areas** of the artifact (corner A fixed → corner B surfaces the *same shape* → corner C …, e.g. move-semantics → direct-delivery → boa-delivery → textContent), do **NOT** keep patching corner-by-corner AND do **NOT** surface a scope-boundary/merge proposal yet. First answer in writing: **"is a whole *layer / altitude* of this artifact generating these findings — and should that layer exist here at all?"** For a B0 *design-audit* doc the generating layer is usually **detailed per-instance behavioral characterization** (per-op event-order tables, per-API coalescing specifics, per-parent/per-arity classification, gap-tables) = **B1-altitude detail that does not belong in B0**. The fix is to **remove that layer** (collapse to named invariants + grep-diff methodology + B1 hand-off), NOT to keep correcting it and NOT to scope-boundary-merge with it intact — removal is *altitude correction* (the finding / two-mechanism model / named invariants / design-questions / methodology survive), not scope-cut. A self-grep for residual behavioral claims (`record|fire|callback|suppress|childList|same-parent|cross-parent|deliver|remove-then|insert-then` asserting per-instance behavior) must return **zero** before re-trigger. Skipping straight to "scope-boundary merge, residual is B1-precision" while the generator layer is still in the doc is the #374 failure (≈10 rounds of corner-hopping before the layer was removed). → `memory/feedback_design-audit-doc-convergence-scope-boundary.md` + `memory/feedback_review-fix-philosophy-first.md` (場当たり禁止 applied to *doc structure*).

**Pre-`AskUserQuestion` written-lens attestation (anti-launder — #374 incident, over-asked 2×)**: before ANY `AskUserQuestion` inside the loop, write one line: *"design-lens conclusion = X; does it converge to a single answer? Y/N"*. If it **converges**, do NOT ask — execute X (state it + act). Only ask when the lens genuinely does not converge (a real fork the user owns: scope/charter/merge-readiness). This mirrors the Step-4 TERMINAL written attestation — the forced written check is what makes the rule fire; the passive `memory/feedback_decide-via-philosophy-before-asking.md` note alone did not (#374 asked move-disposition twice when the lens had already converged on "remove the generator layer" / "stop prescribing the subtle rule"). Merge approval is still the user's, but reach it by *proposing + executing the lens result*, not by laundering the decision into a question.

## failure_modes

Historical **Copilot** R-loop incidents that calibrated the loop's defensive rules (now inherited by `external-converge`; the Codex pitfall-gate is simpler — OpenAI cloud, no workflow-log autofind / no `requestReviews` staleness). Each line: incident → operative rule.

- broker-register-ack (slot #10.6c, lessons #135-141) — 8R on layer-confused goal → **Step 3.5 (1) upward drift**.
- PR #151 (lesson #145, `m4-12-architectural-drift-incident.md`) — 4R × 17 IMP before downward drift detected → **Step 3.5 (2) downward drift**.
- PR #154 R1-R9 (2026-05-05) — ~50% IMP miscalibrated as polish, 2 false scope-creep alerts → **Step 2 severity calibration**.
- PR #163 R1-R17 (2026-05-08) — 5k LoC budget upper-bound exceeded by 2× without scope creep → **Step 4 trigger #4** (LoC-scaled).
- PR #163 R29 (workflow-log misread), R30 (`first: 100` page-2 truncation), R31 (post-TERMINAL over-loop), 2026-05-08 — **Step 1 pitfall gate + Step 4 TERMINAL stop**.
- PR #201 R9 (2026-05-17) — pre-request review counted as fresh round, real R10 with IMP arrived later → **Step 1 request-staleness gate**.
- PR #213 R2 (2026-05-20) — reactive `HashMap`+per-site `sort` patch shipped through TERMINAL; philosophy-ideal was `BTreeMap` (structural delivery order) → **Step 5.1 design-philosophy lens** (`fix_discipline` overlay).
- PR #367 + #374 (2026-06-20) — B0 design-audit doc (ScriptSession mutation-path), Codex ~14R total. Three failures + their new gates: **(a)** ad-hoc patched a finding-*generating* detailed-characterization layer with corner-hopping same-shape findings for ~10 rounds before questioning whether the layer belongs in B0 → **`fix_discipline` generator-layer/altitude check**. **(b)** `AskUserQuestion`'d 2× after the design-lens had converged → **`fix_discipline` pre-Ask written-lens attestation**. **(c)** merge-on-stale: #367 merged while Codex's latest review was on a pre-head commit → 8 real post-merge findings → follow-up PR → now a **mechanical block**: `~/.claude/hooks/gh-pr-merge-head-guard.sh` (PreToolUse Bash `gh pr merge`, denies unless Codex `assessed_commit` == PR head; override `# merge-stale-ok`) + the existing `reviewer.assessed_commit_marker` head-check. Net positive of the loop: it caught a spec-wrong move-record invariant (DOM §4.2.3 `suppressObservers` vs HTML §4.13.6 CE lifecycle). → `memory/feedback_design-audit-doc-convergence-scope-boundary.md`.

## wakeup_poll

`300s` — poll cadence while waiting for Codex's review to land, **NOT a latency prediction**.

**Observed latency (user-confirmed 2026-06-21, #390): a single Codex review normally takes ~15 minutes to land.** (Prior "~2 min" [#288] was a *manual-trigger one-off*; #295's ~30–90 min round gaps were *fix-time between rounds* [Claude fixing], not review latency.) So:

- **~15 min is NORMAL, not stuck** — do **not** re-trigger or "surface as slow" at ~14–15 min. At that point the first review is almost certainly still running, and re-triggering interrupts/duplicates a live review rather than recovering a stuck one (#390 re-triggered at ~14 min into a still-running first review — premature).
- **Poll patiently** — 300s cadence is fine (not-spamming the reviewer matters more than the 300s prompt-cache TTL here; widen toward 600s if preferred). `ScheduleWakeup` IS the poll.
- **Surface / re-trigger threshold = ~25–30 min** of zero Codex activity on head (no formal review, no marker-bearing issue-comment, no inline thread) — NOT the generic SKILL.md "~15 min" sanity cap, which is too aggressive for this reviewer. Only then treat it as queued/slow/trigger-not-taken and surface to the user.

→ `memory/feedback_external-converge-codex-latency.md`.

## reviewer

- `bot_login`: `chatgpt-codex-connector[bot]` (REST form). **GraphQL `reviewThreads` author.login is the BARE `chatgpt-codex-connector`** (no `[bot]`) — the Step-1 fetch must normalize (strip `[bot]`) for GraphQL comparisons or it false-negatives every inline finding (`#316`/`#337`).
- `name`: Codex (OpenAI Codex Cloud, ChatGPT **Pro** — loop-affordable, no per-credit cost)
- `trigger`: `@codex review` (posted as a PR comment to re-trigger each round)
- `assessed_commit_marker`: `Reviewed commit:` — appears in BOTH formal-review bodies AND Codex's dry-verdict issue-comment, followed by `` `<sha>` ``. Step 1 reads the reviewer's latest assessed commit from this marker across reviews + issue-comments (NOT the reviews API alone).
- `dry_verdict_match`: `Didn't find any major issues` — Codex's no-findings verdict, posted as a **plain PR issue-comment** (`Codex Review: Didn't find any major issues`), **not** a formal review. A dry-verdict comment on the current head IS a dry round; keying head-staleness on `pulls/{n}/reviews` alone false-stalls every dry round (`#322`/`#337` — see `memory/feedback_codex-dry-verdict-is-issue-comment.md`).

Lenses reach Codex via `AGENTS.md` (`## Review guidelines` → `axes.md`). The genuine Pro Codex is `chatgpt-codex-connector[bot]`; a bare `@codex[agent]` mention is a Copilot-billed impostor — do **not** use. Background → `memory/project_ai-review-setup.md`.
