---
name: pre-push
description: Run the full elidex pre-push gate in one shot — cargo fmt → mise run ci → /code-review → /review → /elidex-review — so no stage gets skipped on a busy PR. Invoke BEFORE `git push` / `gh pr create`. Collapses the 5-stage gate into one decision so the heavy stages (/code-review, /review) aren't forgotten.
user-invocable: true
---

# pre-push — one-shot pre-push gate

The elidex pre-push gate is five stages run in a fixed order. The trap is reaching for `/elidex-review` alone and treating it as "the comprehensive one" — it explicitly layers *on top of* `/code-review` and `/review` and assumes they ran. This skill makes the whole gate one invocation so the decision surface is "did I run /pre-push?", not five independent "did I remember X?".

## Hard rules

- **No skipping.** Every stage must be invoked. The only exception is the whole-skill Skip-OK clause below (pure-doc PR → don't invoke Stages 3–5). Sub-skills have their own internal skip clauses that may fire *during* invocation (e.g. `/elidex-review`'s doc-only / <30 LoC) — that's the sub-skill's concern, not a reason to skip invoking it from pre-push. Do not invent new per-stage skip conditions here. Record any skip in the landing memo.
- **No substitution.** `/elidex-review` does NOT replace `/code-review` + `/review`. Run all three.
- **Fix → re-verify.** If *any* stage produces code edits (a `/code-review` fix, an accepted review finding, etc.) → re-run Stage 2 before continuing. Later stages and the eventual push must see green, formatted code. (This covers Stages 3/4/5 — no per-stage repeat below.)
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
Invoke the `code-review` skill at default effort. Correctness review of the changed diff; apply the fixes worth taking.

### Stage 4 — `/review`
Invoke the `review` skill. General PR-level review.

### Stage 5 — `/elidex-review`
Invoke the `elidex-review` skill. The project-specific 5-axis design review (Layering / ECS-native / pragmatic / spec / project-context). This is the final design gate and compresses the Copilot R-loop.

## On completion

When Stages 1–5 are all green / addressed, the branch is push-ready. Record a gate-completion marker so a future `git push` hook can enforce the gate:
```sh
git rev-parse HEAD > "/tmp/elidex-pre-push-$(git rev-parse --abbrev-ref HEAD | tr '/' '-').done"
```
Then surface the push / PR proposal to the user (do not push autonomously unless previously authorized).

## Skip-OK (whole skill)

- Pure doc / non-code PR with no `**/*.rs` changes (Rust sources live under `crates/**/src/**`; no top-level `src/`) → skip Stages 3–5; note the skip in the landing memo. Stages 1–2 still run.
- Otherwise: run the whole gate.

(This isn't a *new* skip — `/code-review`, `/review`, and `/elidex-review` each independently treat doc-only as skip-OK and would return 0 findings. This clause hoists it to the top level to avoid three no-op invocations.)
