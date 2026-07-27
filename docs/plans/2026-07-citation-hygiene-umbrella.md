# Citation hygiene — umbrella plan

**Status**: umbrella. Governs the slices below; each slice carries its own plan-memo and its own `/elidex-plan-review`.
**Owner lane**: L3 (DOM/form) — but slices A–C are engine-wide tooling and are *not* L3-specific.

## Why this program exists

Two carves, both forced by the same rule, both discovered by a gate rather than at authoring time.

1. **PR-A0** (`domform-submittable-category`) bundled a WHATWG-HTML constraint-validation citation sweep with a general-purpose detector (`webref cite-audit`), a shared spec-label refactor, and a behaviour change to `preflight.py` — the hard gate every plan review runs. `/code-review max` returned 15 confirmed findings and `/elidex-review` 0 CRIT / 26 IMP / 20 MIN; the decisive one was the shape, not any single defect. Tooling is 41% of that diff, the sweep 14%, and the dependency runs one way only.
2. **The carve** (`webref-cite-audit-tool`) then produced a plan-memo whose own §9 concedes the edge-dense trigger fires on both counts and that the base case does not apply — and proceeded as a single PR anyway. `/elidex-plan-review` returned 0 CRIT / 22 IMP / 19 MIN.

CLAUDE.md § "Design discipline": *"Edge-dense work = multi-PR program + 実装前 plan-review 必須 … (judgment でなく rule)"*. Its **base case** — *"承認済 umbrella 配下で plan-review を通った narrowly-scoped per-PR slice は terminal 単位"* — is what terminates the recursion. What was missing at recurrence 2 was not more slicing; it was this document.

**This umbrella is the approval boundary.** A slice below that passes `/elidex-plan-review` is a terminal unit and is not re-split for touching the same subsystem.

## Slices — the ordering is forced, not preferred

| # | Slice | Branch | Scope | Why it must precede the next |
|---|---|---|---|---|
| A | Enforcement plumbing | `webref-cite-audit-tool` (current) | `mise` task + CI path-filter/job; `preflight.py` fail-closed; move the consumer-derivation assertion off the tools package | Nothing downstream is guarded until the suites actually run. `.claude/**` is in **neither** `ci.yml` path filter today, so a tooling-only PR triggers zero jobs — verified. Landing A first means B and C are enforced from their first commit. Touches no detector semantics. |
| B | Detector correctness | new | The nine under-report paths; the gate-bucket and grammar findings from A's plan-review; `AuditResult`; one section-number grammar | C retires a discovery method on a supersession claim. That claim is only admissible once B has **measured** the detector's precision and reach. D re-derives a sweep against B's output — running it against today's detector means redoing it. |
| C | Policy retirement | new | `.claude/skills/elidex-review/axes.md` requirement (2)/(4); `CLAUDE.md` § "Spec citation"; `DESIGN.md` | Retiring the alternative method while the replacement's reach is unproven converts a visible gap into an invisible one. Blocked on B's reach measurement. |
| D | Constraint-validation sweep | `domform-submittable-category` (rebase) | The existing `crates/**` comment repairs, **re-derived** on the fixed detector; the 8 newly-authored wrong citations found by `/elidex-review` | PR-A's blast-radius map is expressed in line anchors and grep counts that D moves. |
| E | `is_submittable` category repair | `domform-submittable-category` → PR-A | Per `docs/plans/2026-07-form-submittable-category-repair.md`, **re-derived** — 17 of its anchors/counts are already falsified by PR-A0's own edits | Slice 1 regresses `<button type=submit>:valid` without it. |
| F | Slice 1 keystone | `domform-slice1` | Delete the `ElementState` form-bit cache | — |

Slices A–C are engine-wide tooling; D–F are the L3 form program. The join is real but one-directional: D's exit criterion is a command that B must make trustworthy.

## Constraints each slice inherits

- **A slice may not carry another slice's concern.** Specifically: A may not change detector semantics; B may not edit review policy; C may not repair citations.
- **Per-PR ≤3 own deferrals** (`feedback_defer_cap_policy`). Gate-uncovered pre-existing defects are a separate category.
- **Counts are commands.** No slice memo carries a quantity it did not derive; every quantity ships its derivation.
- **A claim is admissible only if something mechanically checks it.** A slice memo's "claims vs checks" table must mark unchecked rows UNCHECKED rather than omitting them.

## Cross-lane coordination

- **Slice A changes `preflight.py`'s failure semantics** from silent-exit-0 to fail-closed. Every lane runs that gate. A's landing checklist must re-run preflight on the in-flight plan-memos in `elidex-wt-c3-plan`, `elidex-wt-vmp4plan`, `elidex-wt-turncomp`, `elidex-wt-slice1`, `elidex-wt-c4fix` and record the result.
- **Slice B re-points spec labels to their current level** (`CSSOM`→`cssom-1`, `Selectors`→`selectors-4`, `Pointer Events`→`pointerevents4`). 10 in-flight memos in `elidex-wt-c3-plan` cite those labels; B's landing checklist must re-verify them.
- **Slice B moves the numbers slice D's exit criterion reads** (UNATTRIBUTED and per-spec counts). D re-baselines rather than carrying A/B-era figures.

## Records this program owns

`MEMORY.md` L3 lane bullet and `memory/project_slice1-elementstate-cache-deletion-state.md` both record a 3-PR chain (PR-A0 → PR-A → Slice 1). That shape is superseded by the table above and is updated at the same time as this document, not at landing — the fourth branch already exists, so the stale form is stale now.

## Derivation

```sh
# diff composition that forced carve 1
git diff --numstat origin/main...domform-submittable-category -- docs/plans/ '.claude/**' crates/
# the CI hole that makes slice A first
sed -n '/filters:/,/^  check:/p' .github/workflows/ci.yml
# detector state at any point
.claude/tools/webref cite-audit html --summary
```
