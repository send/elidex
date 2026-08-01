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
| A-i | The shared spec-label map | `webref-cite-audit-tool` (current) | **(table)** Create `.claude/tools/_webref/spec_labels.py` **pinned-map-only**; collapse the three hand-maintained copies onto it; delete the 8 inert parse aliases; the `DESIGN.md` bullet; move the 8 label-map tests into a new generic-tree `test_spec_labels.py`. **(prose)** Rewrite every consumer list and rationale naming an elidex file path (by role) or a Slice-B artifact — in `spec_labels.py`, `cli.py` incl. its `--help` epilog, `coverage_map.py`, and the moved tests — and correct the copy-count claim at every site asserting it. ⚠ **The import lands hard, without an `except` guard**: draft 1 kept the carve's `except Exception: _shortname_for = None`, which would have put a **fail-open plan-review gate** on `main` for the whole duration of A-ii — the state this table calls disqualifying for Slice B. A hard import fails *loudly*, so no such window exists, and A-ii's job becomes turning a loud failure into a named cause rather than closing a hole it inherited. No CI topology. | A-ii's whole subject is the failure mode this import *creates*. If A-ii landed first there would be nothing to fail closed; if B landed the import, `main` would carry a fail-open plan-review gate for the duration of B. |
| A-ii | The gate's failure semantics | new, stacked on A-i | `preflight.py` fail-closed: the two-cause capability verdict, both act-sites, the four remedy strings, the no-spec-surface declaration, and `SKILL.md`'s contract of record | Every lane runs this gate. It must be correct before anything downstream relies on its verdict. |
| A-iii | The suite scheduler | new, stacked on A-ii | `.claude/tools/python-suites.sh`; `[tasks.tools-test]` in `[tasks.ci].depends`; an **ungated** `tools` job in `ci.yml`; the interpreter floor | Nothing downstream is guarded until the suites actually run. `.claude/**` is in **neither** `ci.yml` path filter today, so a tooling-only PR triggers zero jobs — verified. Landing it means B and C are enforced from their first commit. |
| B | Detector correctness | new | The 948-entry catalog fall-through and its lookup semantics; the nine under-report paths; the gate-bucket and grammar findings from A's plan-review; `AuditResult`; one section-number grammar | C retires a discovery method on a supersession claim. That claim is only admissible once B has **measured** the detector's precision and reach. D re-derives a sweep against B's output — running it against today's detector means redoing it. |
| C | Policy retirement | new | `.claude/skills/elidex-review/axes.md` requirement (2)/(4); `CLAUDE.md` § "Spec citation"; `DESIGN.md` | Retiring the alternative method while the replacement's reach is unproven converts a visible gap into an invisible one. Blocked on B's reach measurement. |
| D | Constraint-validation sweep | `domform-submittable-category` (rebase) | The existing `crates/**` comment repairs, **re-derived** on the fixed detector; the 8 newly-authored wrong citations found by `/elidex-review` | PR-A's blast-radius map is expressed in line anchors and grep counts that D moves. |
| E | `is_submittable` category repair | `domform-submittable-category` → PR-A | Per `docs/plans/2026-07-form-submittable-category-repair.md`, **re-derived** — 17 of its anchors/counts are already falsified by PR-A0's own edits | Slice 1 regresses `<button type=submit>:valid` without it. |
| F | Slice 1 keystone | `domform-slice1` | Delete the `ElementState` form-bit cache | — |

Slices A–C are engine-wide tooling; D–F are the L3 form program. The join is real but one-directional: D's exit criterion is a command that B must make trustworthy.

### ⚠ Slice A re-sliced into A-i / A-ii / A-iii (2026-08-01, user-approved)

**Why**: nine `/elidex-plan-review` rounds on a single Slice-A memo, and round 9 was **worse than round 8**
(3 CRIT / 33 IMP vs 0 CRIT / 30 IMP). Four consecutive rounds produced the same root at ascending levels —
executable-described-in-prose (R6), the fix inverting the predicate (R7), the harness not covering its own
claims (R8), the discharge written in the memo but not executed in the artifact (R9). A loop whose severity
rises is not approaching real-gap exhaustion; per `feedback_defer-accumulation-signals-mis-drawn-slice` the
boundary is the defect.

**Where the seam actually is**: round 9's findings separate by axis almost perfectly — every Axis 1
(layering) finding lives in the map extraction, and every Axis 2 finding, including both CRITs, lives in the
gate's failure semantics. Those two had been sharing one memo because one *enables* the other, which is an
ordering relation, not a cohesion one. The earlier candidate seam (J1-J3 vs J4/J5) was tested and rejected
for the wrong reason — it measured where findings landed rather than whether the slices separate.

**What does not change**: the ordering is still forced (A-i → A-ii → A-iii), the umbrella is still the
approval boundary, and each of the three is a terminal unit once it passes its own plan-review.

### Slice memos (re-sliced 2026-07-28, Slice A further split 2026-08-01)

The 785-line single-PR memo `2026-07-webref-cite-audit-detector.md` was partitioned into A/B/C; the 1196-line Slice-A memo `2026-07-citation-hygiene-A-enforcement-plumbing.md` was then partitioned into A-i/A-ii/A-iii and **deleted** — keeping it would be a second statement of every decision the three now own, which is the duplication this program exists to remove. Each carved memo's §14 carries its provenance. ⚠ **An earlier revision of this line said the nine-round review history lives in `memory/project_citation-hygiene-program.md`; measured, that file stops at round 7 and mentions neither round 8, round 9, nor the A-i/A-ii/A-iii split.** It was written without checking — the defect the constraint above names. That file is brought current in the change that lands this; until then the round-by-round index exists only at `git show ee2d0dc0:docs/plans/…`. Nothing is
summarised across memos — each concern is stated once, in one slice's memo, and the others link to it.

| Slice | Memo | Gate state |
|---|---|---|
| A-i | `2026-07-citation-hygiene-Ai-spec-label-map.md` | **review-ready**; `preflight` EXIT 0, K=2 (`fetch`, `html`), 0 hard / 0 soft grep-pass |
| A-ii | `2026-07-citation-hygiene-Aii-gate-failure-semantics.md` | draft; `preflight` EXIT 0, K=2 |
| A-iii | `2026-07-citation-hygiene-Aiii-suite-scheduler.md` | draft; `preflight` EXIT **1 by design** — A-iii declares **no spec surface**, which is A-ii's §4.2.5 feature and is not landed yet. A-iii is the first real consumer of that declaration, and its plan-review therefore follows A-ii, which the ordering already requires |
| B | `2026-07-citation-hygiene-B-detector-correctness.md` (`git mv` of the 785-line memo, so its provenance survives) | draft; `preflight` EXIT 0. §4.0-§4.1 / §4.6 / §5 carried verbatim; §0-§2 and §7-§13 rewritten to the slice boundary |
| C | `2026-07-citation-hygiene-C-policy-retirement.md` | draft; `preflight` EXIT **1** by design — no `§3` table until C's kickoff, a pre-existing hard-fail mode unrelated to slice A |

**Two corrections the re-slice produced**, both by executing rather than reading, and both recorded at
their site: the fail-closed tri-state does **not** work where the pre-slice memo sited it (a memo whose
`§3` rows carry no spec label still exits 0 — measured against the proposed patch), and the suites' network
behaviour is not what the pre-slice memo assumed.

⚠ **The second correction was itself wrong and is corrected here.** This document said wiring the suites
into CI takes a **live-network dependency** ("the 48-test `_webref` suite fetches 2 URLs"). Measured
(`rederive suites`): **0 `urlopen` calls** across the `origin/main` suites. The figure came from a
branch-measured run, and the branch's catalog fall-through — which the re-slice moved to **B** — was the
thing fetching. A-i ships a pinned-map-only resolver and adds no network requirement; **B owns the offline
contract for the fall-through it introduces**, which is a constraint below.

## Constraints each slice inherits

- **A slice may not carry another slice's concern.** Specifically: A may not change detector semantics; B may not edit review policy; C may not repair citations.
- **Per-PR ≤3 own deferrals** (`feedback_defer_cap_policy`). Gate-uncovered pre-existing defects are a separate category.
- **Counts are commands.** No slice memo carries a quantity it did not derive; every quantity ships its derivation.
- **A claim is admissible only if something mechanically checks it.** A slice memo's "claims vs checks" table must mark unchecked rows UNCHECKED rather than omitting them.
- **A slice memo may only cite spec labels that slice's own resolver maps.** Round 9 found the merged
  Slice-A memo citing `CSSOM View §4.2` in its own coverage map — a label resolving only through the catalog
  fall-through this program routes to Slice B, so the memo was certified by machinery its own slice removes
  and would have soft-warned against itself after landing. ⚠ The rule ranges over **every citation surface**
  (§0.5, §3, prose), not §3 alone — the defect appeared at two surfaces, and a §3-only rule under-covers the
  case it came from. A label the slice does not map may appear only as *fixture content*, where being
  unmapped is the property under test, and must be marked as such.
- **A check must derive its own coverage, not only its values.** Round 8 and round 9 of Slice A's review both found blocks that printed a correct number while their stated derivation ranged over the wrong set — a grep that discarded the lines the memo cited it for, instrumentation sited in the branch where the defect was already fixed. A derivation that cannot witness the claim's negation is not a check.
- **No slice may make label resolution require the network without shipping its offline degradation in the same slice.** Slice B introduces the catalog fall-through and therefore owns the offline contract for it.
- **The plan-review gate reaches its shared library one way.** Slice B, which lands the offline contract, collapses `verify_citation`'s subprocess onto the in-process resolver in the same slice. Tracked as `#11-webref-preflight-inprocess-resolution`.

## Cross-lane coordination

- **Slice A-ii changes `preflight.py`'s failure semantics** from silent-exit-0 to fail-closed. Every lane runs that gate. A-ii's landing checklist must re-run preflight on every worktree that authors a plan-memo and record the result. ⚠ **The worktree set is derived, not listed** — `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh lanes`. The list this bullet used to carry named `elidex-wt-c4fix`, which does not exist, and omitted `elidex-wt-submittable` and `elidex-wt-tripwire-ci`, which do.
- **Slice B re-points spec labels to their current level** (`CSSOM`→`cssom-1`, `Selectors`→`selectors-4`, `Pointer Events`→`pointerevents4`). ⚠ **This bullet said "10 in-flight memos in `elidex-wt-c3-plan`"; measured, it is 1** (carrying `CSSOM VIEW` ×14, `RESIZE OBSERVER` ×3, `INTERSECTION OBSERVER` ×1 — all of which the widening resolves *correctly*). B's landing checklist must re-verify it.
- **CI topology is decided across two lanes, not one.** The Layout lane's [PR #496](https://github.com/send/elidex/pull/496) lands an **ungated** trip-wire job and argues in-file that gating `.claude/tools/**` behind a path filter makes the tamper path of an allowlist gate itself an allowlist entry. Slice A-iii **adopts that shape** rather than adding a competing filter — one question, one answer. Whichever lands second is a textual merge, not a decision.
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
