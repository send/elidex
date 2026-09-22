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
leak between files. R2 reported **IMP 32** (the R2 review's own count, not re-derived here) — more, not fewer — and most of them arose from the child-process
boundary that draft 2 had introduced (bootstrap, text serving, patchability of the child, cost). A design
whose review load grows with each answer is re-drawing its own boundary each round; the isolation it bought
was against a threat — deliberate tampering — that review already catches.

## What was fixed instead

The commit that withdrew this plan (`git log -1 --format=%h -- docs/plans/2026-09-plan-memo-selftest-registry.md`)
fixed every in-model finding in the existing mechanism, each with a partner arm and a killing mutation row:

- the kind ratchet's key projections and clause drops (IMP-1, N-2, MIN-9) and the truncation criteria;
- the membership partner now drives the SAME function the real control calls, over a planted directory
  (IMP-2), and the reachability walk is a bounded fixed point (N-3);
- row shape — a row naming no control, a repeated label or edit (MIN-5, MIN-6); control-name shadowing
  and unmerged fragments (N-1, N-4) through one refusing `Registry`;
- registry-module partition criteria and the `from X import` order edge (MIN-7, MIN-8);
- the classes the threat-model review found an ACCIDENT can reach: rows bound to a throwaway list
  (`spellings([])`, IMP-4), an alias of another module's list, and an emptied registry list;
- docstring accuracy (MIN-10..12).

## Declared out-of-model classes

The in-model and out-of-model classes — each with the reasoning for whether an accidental edit reaches it,
and the probe that shows it — have ONE home: the docstring of `.claude/tools/plan_memo_selftest_registry.py`
("THE THREAT MODEL"). They are not copied here.
