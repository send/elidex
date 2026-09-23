---
name: pre-push
description: Run the full elidex pre-push gate in one shot — cargo fmt → mise run ci → /simplify → /code-review → /elidex-review — so no stage gets skipped on a busy PR. Invoke BEFORE `git push` / `gh pr create`. Collapses the 5-stage gate into one decision so the heavy stages (/simplify, /code-review) aren't forgotten.
user-invocable: true
---

# pre-push — one-shot pre-push gate

The elidex pre-push gate is five stages run in a fixed order. The trap is reaching for `/elidex-review` alone and treating it as "the comprehensive one" — it explicitly layers *on top of* `/simplify` and `/code-review` and assumes they ran. This skill makes the whole gate one invocation so the decision surface is "did I run /pre-push?", not five independent "did I remember X?".

## Hard rules

- **No skipping.** Every stage must be invoked. The only exception is the whole-skill Skip-OK clause below (pure **inert** doc PR → don't invoke Stages 3–5; a **review/enforcement-tooling edit is NOT inert** and runs the whole gate — see Skip-OK). Sub-skills have their own internal skip clauses that may fire *during* invocation (e.g. `/elidex-review`'s inert-doc skip, which itself excludes enforcement-tooling edits) — that's the sub-skill's concern, not a reason to skip invoking it from pre-push. Do not invent new per-stage skip conditions here. Record any skip in the landing memo.
- **No substitution.** `/elidex-review` does NOT replace `/simplify` + `/code-review`. Run all of them.
- **Committed input.** Every reviewing stage is given the resolved `$BASE...HEAD` range, and `/elidex-review` reads nothing else. An uncommitted working tree is therefore invisible to the whole gate, and invisibly so — it runs green over an empty or stale diff. **Commit the branch's work before Stage 3**, whether or not a stage produced edits.
- **Fix → re-verify.** If *any* stage produces code edits (a `/simplify` rewrite, a `/code-review` fix, an accepted review finding, etc.) → re-run Stage 1 (fmt) then Stage 2 (ci), then **commit** per the rule above. Stage 1 is needed because Stage 3 (`/simplify`) is the pipeline's only auto-apply stage and its output may not be formatted; `cargo fmt --check` in Stage 2 ci will reject it otherwise. Later stages and the eventual push must see green, formatted, committed code. (This covers Stages 3/4/5 — no per-stage repeat below.)
- This skill stops **before** `git push` / `gh pr create`. Pushing is a separate authorized action — confirm per the usual remote/shared-state rules.

## Stages (fixed order)

### Stage 1 — Format
```sh
cargo fmt --all
```

### Stage 2 — Verify
```sh
mise run ci   # check + lint + test-all + doc + deny + ci-sweep cleanup (no-op without cargo-sweep) — CLAUDE.md "Push 前"
```
All jobs must be green. (`test-all` includes doc-tests; cargo tasks are `--all-features`-gated, so feature-gated code is covered.)

Fallback: if `mise run ci` reports a spurious `target/`-missing failure (the `ci-sweep` cleanup task can race the build and wipe `target/` mid-compile), re-run the verification jobs serially — this sidesteps the race by omitting `ci-sweep`:
```sh
mise run check && mise run lint && mise run test-all && mise run doc && mise run deny
```

### Stage 3 — `/simplify`
Invoke the `simplify` skill **on the resolved `$BASE...HEAD` range** (`$BASE` is defined in `/elidex-review`'s SKILL.md), so the stage is pinned to the PR diff rather than to whatever scope it would otherwise infer. Quality-only pass — reuse / simplification / efficiency / altitude cleanups, read from the range and applied to the working tree. It does not hunt bugs (that's Stage 4); the two are complementary, not redundant. `/simplify` is the pipeline's only auto-apply stage (it mutates the working tree directly), so placing it here ensures both the ci re-verify (Stage 2 re-run) and the bug net (Stage 4) validate its output.

### Stage 4 — `/code-review`
Invoke the `code-review` skill **on the PR's whole diff**, **effort scaled to blast radius**. This is now the primary correctness net: the post-push non-Claude pass **defaults to single-shot** (`/external-review`, currently Codex) — a multi-round loop (`/external-converge`) is **opt-in for high-stakes PRs, not an implicit safety net** — so the depth for routine PRs must come from here.

- **Routine PR** → `/code-review high`. Broad coverage is the floor now — `low`/`medium` were calibrated for when a looping post-push reviewer was the routine backstop, which it no longer is (the post-push default is single-shot; `/external-converge` is opt-in, not automatic).
- **High blast-radius PR** (layout / inline / whitespace / parser / edge-matrix-dense subsystems, large diff, or touching a `vm/host/` layering path) → `/code-review ultra`. The deep multi-agent cloud pass is the functional successor to the old multi-round post-push review loop — run it once here rather than relying on post-push looping. (Billed under Claude usage; reserve `ultra` for genuinely high-risk PRs so it stays one pass per PR.)
- ⚠ **Reaching the whole diff** (https://code.claude.com/docs/en/code-review.md):
  - **non-`ultra`**: resolve `$BASE` (defined in `/elidex-review`'s SKILL.md) and pass the resolved range — e.g. `/code-review high origin/main...HEAD`; skill arguments are not shell-expanded, so a literal `$BASE` reaches it as text. With no target, `/code-review` reviews the commits ahead of the branch's upstream plus uncommitted changes — on a branch pushed once, the unpushed commits, not the PR.
  - **`ultra`**: **only the user can launch it.** A model-issued `/code-review ultra` does not reach the cloud pass; it degrades to a local review, silently, on exactly the PRs that asked for the deepest one. So **ask the user to type `/code-review ultra`** themselves — they pass no target, since it would read a single word as a base branch or PR number and its own scope is already the branch against the default branch. If they decline, run `/code-review max <resolved range>` and say that is what ran.
- `/review` is an alias of `/code-review` since Claude Code v2.1.223 (same page), so the former Stage 5 is this stage.

Apply the fixes worth taking — then fmt, ci, **commit**, and only then **run this stage again on the same resolved range**. The commit is what puts the fixes inside the range; re-running before it re-reads the diff the first pass already read.

**Two passes are the floor**, not a fix-triggered extra: the folded Stage 5 was a second sample of this same reviewer on the same input, and this project has measured byte-identical re-runs producing findings the first pass missed (#515) — folding the stages removed a duplicate *skill*, not a duplicate *sample*. Stop at the first pass **from the second onward** that yields no behavioural edit (formatting and prose do not count). If a third pass still yields behavioural edits, stop and surface it: the churn is saying something about the change, not asking for another round.

### Stage 5 — `/elidex-review`
Invoke the `elidex-review` skill. The project-specific 5-axis design review (Layering / ECS-native / pragmatic / spec / project-context). This is the final design gate. The post-push non-Claude pass defaults to a single-shot second opinion (`/external-review`, currently Codex); `/external-converge` (multi-round to real-gap exhaustion) is opt-in for high-stakes PRs, not an implicit safety net — so this gate plus Stage 4's blast-radius effort carry the bulk of the design review.

## On completion

When Stages 1–5 are all green / addressed, the branch is push-ready. Surface the push / PR proposal to the user (do not push autonomously unless previously authorized).

## Skip-OK (whole skill)

- Pure inert doc / non-code PR — no `**/*.rs` change **and** no change to review/enforcement behavior — → skip Stages 3–5; note the skip in the landing memo. Stages 1–2 still run.
  - **EXCEPT a review/enforcement-tooling edit**, which is NOT inert → it runs the **whole gate (Stages 3–5)**, not this skip (no per-stage carve — that would violate the "No skipping" hard rule above). For *what counts* as such an edit (which paths, executable-vs-inert, why) → **`/elidex-review` "Skip OK" is the single source**; not restated here.
- Otherwise: run the whole gate.

(This isn't a *new* skip — `/simplify`, `/code-review`, and `/elidex-review` each independently treat *inert* doc-only as skip-OK and would return 0 findings. This clause hoists it to avoid three no-op invocations. The carve-out — review/enforcement-tooling edits aren't inert — lives in `/elidex-review`'s "Skip OK"; this references it, doesn't restate it.)
