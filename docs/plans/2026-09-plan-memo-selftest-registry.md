# Decision record — the self-test's row registries

**Status**: WITHDRAWN as a plan; kept as the record of the decision that replaced it (2026-09-23).
Branch `vm-p4-plan-memo-checker` (PR #510). Parent: `docs/plans/2026-08-plan-memo-umbrella-checker.md`.
The full plan text, both drafts, is in history: `git show 43865000:docs/plans/2026-09-plan-memo-selftest-registry.md`
(draft 1) and `git show cf18de7e:docs/plans/2026-09-plan-memo-selftest-registry.md` (draft 2, after R1).

## The decision (user, 2026-09-23)

The self-test registry's **threat model** is: guard against **accidental drift** and against **gaps in the
proof** — a check whose change would go undetected. It does **not** guard against deliberate tampering by
the self-test's own code (a module reassigning or clearing another module's list, binding rows to a list
that is not its own, mutating builtins or the standard library at import): that code is visible in a diff
and in review.

This replaces the plan's **by-construction isolation** design. There is no child process per registry file
and no row sink.

## Why the plan-review did not converge

Draft 1 (`43865000`) went to `/elidex-plan-review`; R1 reported **IMP 29** (MIN 17, CRIT 0 raw — the count
in draft 2's status line). Draft 2 (`cf18de7e`) answered R1 by moving collection into a child interpreter
per file, fed `SOURCES ∪ disk` text through a meta-path finder, so that builtins and stdlib state could not
leak between files. R2 reported **IMP 32** (the R2 review's own count, not re-derived here) — more, not
fewer. The isolation draft 2 bought was against a threat — deliberate tampering — that review already
catches.

## What replaced it — the golden manifest (user-approved, 2026-09-23)

`679b6634` first fixed the in-model findings with SHAPE DETECTORS (merge sweeps, bind-time checks, an
unmerged-fragment detector). Its attestation found 6 IMP, each a spelling the detectors did not know
(`reg |= fragment`, `{**a, **b}`, a mid-file `CASES = list(CASES)`, `MUTANTS += [` edited to `MUTANTS = [`, a
control `.pop()`-ing live rows). The class is open-ended by construction, so the instrument changed: a
committed, machine-generated manifest (`.claude/tools/plan_memo_selftest_manifest.txt`) holds one line per
mutation row, control and case; `--self-test` compares the collection it is about to execute against it and
fails on any difference; `--write-manifest` regenerates it from the same collection call; the collected rows
are frozen, so a control cannot change them in place. The shape detectors it subsumes were deleted. The
proof-gap fixes of `679b6634` that are not shape detectors (the sanction-key arms, the bounded
reachability, the membership partner driving the real function) stand. **Workflow rule**: adding, removing
or changing a row, control or case requires regenerating the manifest and committing its diff.

## Threat model

Its one home is the docstring of `.claude/tools/plan_memo_selftest_registry.py` ("THE THREAT MODEL"); it is
not copied here.
