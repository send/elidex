---
name: pre-push
description: Run the full elidex pre-push gate in one shot — cargo fmt → mise run ci → /code-review → /simplify → /review → /elidex-review — so no stage gets skipped on a busy PR. Invoke BEFORE `git push` / `gh pr create`. Collapses the 6-stage gate into one decision so the heavy stages (/simplify, /review) aren't forgotten.
user-invocable: true
---

# pre-push — one-shot pre-push gate

The elidex pre-push gate is six stages run in a fixed order. The trap is reaching for `/elidex-review` alone and treating it as "the comprehensive one" — it explicitly layers *on top of* `/code-review`, `/simplify`, and `/review` and assumes they ran. This skill makes the whole gate one invocation so the decision surface is "did I run /pre-push?", not six independent "did I remember X?".

## Hard rules

- **No skipping.** Every stage must be invoked. The only exception is the whole-skill Skip-OK clause below (pure-doc PR → don't invoke Stages 3–6). Sub-skills have their own internal skip clauses that may fire *during* invocation (e.g. `/elidex-review`'s doc-only / <30 LoC) — that's the sub-skill's concern, not a reason to skip invoking it from pre-push. Do not invent new per-stage skip conditions here. Record any skip in the landing memo.
- **No substitution.** `/elidex-review` does NOT replace `/code-review` + `/simplify` + `/review`. Run all four.
- **Fix → re-verify.** If *any* stage produces code edits (a `/code-review` fix, a `/simplify` rewrite, an accepted review finding, etc.) → re-run Stage 2 before continuing. Later stages and the eventual push must see green, formatted code. (This covers Stages 3/4/5/6 — no per-stage repeat below.)
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

### Stage 3 — `/code-review`
Invoke the `code-review` skill, **effort scaled to blast radius**. This is now the primary correctness net: the post-push non-Claude pass is single-shot (`/external-review`, currently Codex), not a convergence loop, so the depth that used to come from looping a reviewer must come from here instead.

- **Routine PR** → `/code-review high`. Broad coverage is the floor now — `low`/`medium` were calibrated for when a looping post-push reviewer was the backstop, which it no longer is (the post-push reviewer is single-shot).
- **High blast-radius PR** (layout / inline / whitespace / parser / edge-matrix-dense subsystems, large diff, or touching a `vm/host/` layering path) → `/code-review ultra`. The deep multi-agent cloud pass is the functional successor to the old multi-round post-push review loop — run it once here rather than relying on post-push looping. (Billed under Claude usage; reserve `ultra` for genuinely high-risk PRs so it stays one pass per PR.)

Apply the fixes worth taking.

### Stage 4 — `/simplify`
Invoke the `simplify` skill. Quality-only pass — reuse / simplification / efficiency / altitude cleanups, applied to the working tree. It does not hunt bugs (that's Stage 3); the two are complementary, not redundant.

### Stage 5 — `/review`
Invoke the `review` skill. General PR-level review.

### Stage 6 — `/elidex-review`
Invoke the `elidex-review` skill. The project-specific 5-axis design review (Layering / ECS-native / pragmatic / spec / project-context). This is the final design gate. The post-push non-Claude pass is now a single-shot second opinion (`/external-review`, currently Codex), not a convergence loop, so this gate plus Stage 3's blast-radius effort carry the bulk of the design review — they are no longer "compressing" a loop that will catch the rest.

## On completion

When Stages 1–6 are all green / addressed, the branch is push-ready. Record a gate-completion marker so a future `git push` hook can enforce the gate:
```sh
git rev-parse HEAD > "/tmp/elidex-pre-push-$(git rev-parse --abbrev-ref HEAD | tr '/' '-').done"
```
Then surface the push / PR proposal to the user (do not push autonomously unless previously authorized).

## Skip-OK (whole skill)

- Pure doc / non-code PR with no `**/*.rs` changes (Rust sources live under `crates/**/src/**`; no top-level `src/`) → skip Stages 3–6; note the skip in the landing memo. Stages 1–2 still run.
- Otherwise: run the whole gate.

(This isn't a *new* skip — `/code-review`, `/simplify`, `/review`, and `/elidex-review` each independently treat doc-only as skip-OK and would return 0 findings. This clause hoists it to the top level to avoid four no-op invocations.)
