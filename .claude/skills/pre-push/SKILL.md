---
name: pre-push
description: Run the full elidex pre-push gate in one shot — cargo fmt → mise run ci → /simplify → /review → /elidex-review — so no stage gets skipped on a busy PR. Invoke BEFORE `git push` / `gh pr create`. Collapses the 5-stage gate into one decision so the heavy stages (/simplify, /review) aren't forgotten.
user-invocable: true
---

# pre-push — one-shot pre-push gate

The elidex pre-push gate is five stages run in a fixed order. Run as separate
manual steps, they get **skipped on heavy PRs** — the easy trap is reaching for
`/elidex-review` alone and rationalizing it as "the comprehensive one". It is
**not** a substitute: `/elidex-review` explicitly layers *on top of* `/simplify`
(reuse / quality / efficiency) and `/review` (general PR review) and assumes
they ran. This skill makes the whole gate one invocation so the decision
surface is "did I run /pre-push?", not five independent "did I remember X?".

## Hard rules

- **No skipping.** Every stage below is mandatory unless a skip-OK clause fires.
  Do not invent *new per-stage* skip conditions here — for an individual stage,
  defer to that sub-skill's own clause (e.g. `/elidex-review`'s doc-only / <30
  LoC). The only whole-skill fast-path is the doc-only one below, which is just
  the union of the sub-skills' own doc-only clauses hoisted to avoid three
  no-op invocations — not a new condition. Record any skip in the landing memo.
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

### Stage 2 — Verify
```sh
mise run ci   # check + lint + test-all + doc + deny — the canonical pre-push gate (CLAUDE.md "Push 前")
```
All jobs must be green. (`test-all` includes doc-tests; cargo tasks are
`--all-features`-gated so feature-gated code is covered.)

Fallback: if `mise run ci` reports a spurious `target/`-missing failure — its
`ci-sweep` cleanup task can run in parallel with the build and wipe `target/`
mid-compile, surfacing a misleading "No such file" error — re-run the jobs
sequentially instead, which sidesteps the race:
```sh
mise run check && mise run lint && mise run test-all && mise run doc && mise run deny
```
(These are `ci`'s *verification* jobs run serially. `ci` also depends on the
`ci-sweep` cleanup task — deliberately omitted here, since its build-parallel
`target/` wipe is exactly the race the fallback exists to dodge; it cleans
artifacts, it doesn't verify anything.)

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

This is **not a new skip condition** — `/simplify`, `/review`, and
`/elidex-review` each independently declare doc-only as skip-OK, so on a pure
doc PR all three return 0 findings anyway. This clause only hoists that to the
top level to avoid three no-op invocations (per the "No skipping" Hard rule).

- Pure doc / non-code PR with no `**/*.rs` changes (Rust sources live under
  `crates/**/src/**`, there is no top-level `src/`) → skip Stages 3–5; note the
  skip in the landing memo. Stages 1–2 still run.
- Otherwise: run the whole gate.
