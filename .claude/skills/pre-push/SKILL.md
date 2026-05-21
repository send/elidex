---
name: pre-push
description: Run the full elidex pre-push gate in one shot — cargo fmt → sequential mise verify (lint/test-all/doc/deny) → /simplify → /review → /elidex-review — so no stage gets skipped on a busy PR. Invoke BEFORE `git push` / `gh pr create`. Collapses the 5-stage gate into one decision so the heavy stages (/simplify, /review) aren't forgotten.
user-invocable: true
---

# pre-push — one-shot pre-push gate

The elidex pre-push gate is five stages run in a fixed order. Run as separate
manual steps they get **skipped on heavy PRs** — the easy trap is reaching for
`/elidex-review` alone and rationalizing it as "the comprehensive one". It is
**not** a substitute: `/elidex-review` explicitly layers *on top of* `/simplify`
(reuse / quality / efficiency) and `/review` (general PR review) and assumes
they ran. This skill makes the whole gate one invocation so the decision
surface is "did I run /pre-push?", not five independent "did I remember X?".

## Hard rules

- **No skipping.** Every stage below is mandatory unless its own skip-OK clause
  fires (each sub-skill defines its own — e.g. doc-only / <30 LoC). Do not
  invent new skip logic here; defer to each sub-skill's clause and record the
  reason in the landing memo.
- **No substitution.** `/elidex-review` does NOT replace `/simplify` + `/review`.
  Run all three.
- **Fix → re-verify.** If any stage produces code edits (a `/simplify` fix, a
  review finding you accept), re-run Stage 2 (verify) before continuing — the
  later stages and the eventual push must see green, formatted code.
- This skill stops **before** `git push` / `gh pr create`. Pushing is a separate
  authorized action (and a remote/shared-state action — confirm per the usual
  rules).

## Stages (fixed order)

### Stage 1 — Format
```sh
cargo fmt --all
```

### Stage 2 — Verify (sequential, NOT `mise run ci`)
Run the gate **sequentially**, not via `mise run ci`:
```sh
mise run lint && mise run test-all && mise run doc && mise run deny
```
Rationale: `mise run ci` runs the `ci-sweep` task in parallel with the build, and
ci-sweep wipes `target/` mid-build → spurious "No such file" failures
(`feedback_ci-sweep-race`). The sequential chain avoids the race. All four must
be green. (`test-all` includes doc-tests; cargo tasks are `--all-features`-gated
so feature-gated code is covered.)

### Stage 3 — `/simplify`
Invoke the `simplify` skill. Reuse / quality / efficiency review of the changed
files; apply the fixes worth taking. **If it edits code → return to Stage 2.**

### Stage 4 — `/review`
Invoke the `review` skill. General PR-level review.

### Stage 5 — `/elidex-review`
Invoke the `elidex-review` skill. The project-specific 5-axis design review
(Layering / ECS-native / pragmatic / spec / project-context). This is the final
design gate and compresses the Copilot R-loop. **If a fix here edits code →
return to Stage 2.**

## On completion

When Stages 1–5 are all green / addressed, the branch is push-ready. Record a
gate-completion marker (cheap; lets a future `git push` hook enforce the gate —
"option B"):
```sh
git rev-parse HEAD > "/tmp/elidex-pre-push-$(git rev-parse --abbrev-ref HEAD | tr '/' '-').done"
```
Then surface the push / PR proposal to the user (do not push autonomously
unless previously authorized).

## Skip-OK (whole skill)

- Pure doc-only PR with no `src/**` changes → Stages 3–5 each trivially 0-finding;
  may note explicit skip in the landing memo. Stages 1–2 still run.
- Otherwise: run the whole gate.
