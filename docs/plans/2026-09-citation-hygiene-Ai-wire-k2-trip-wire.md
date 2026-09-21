# Slice A-i-wire — the K2 generic-core layering trip-wire

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire**.
**Branch**: `webref-generic-core-trip-wire`, stacked on `webref-cite-audit-tool` (#501).
**PR**: #519. **Status**: implementation carried from #501 `611758ff`; this memo is the
plan-review that instrument never had.

⚠ **This memo states no quantity it did not derive, and no quantity that moves with a
commit.** Where a number matters it appears as the command that produces it. That is the
program's ratified rule, and slice A-i is the reason it is restated here: A-i wrote figures
into its memo and spent four review rounds on figures its own later edits falsified.

---

## §0 Why this slice exists at all

The wire is not new code. It entered #501 at **review round 68**, after that slice's own
`/elidex-plan-review` had closed, and grew there until #501's fifth design re-gate. Three
review axes reached the same conclusion independently: it does not belong in a slice whose
subject is a spec-label map.

CLAUDE.md § "Design discipline" makes `/elidex-plan-review` a **rule, not a judgment** for
edge-dense work, and its base-case clause — *"承認済 umbrella 配下で plan-review を通った
narrowly-scoped per-PR slice は terminal 単位"* — covers what a plan-review approved. It does
not reach an instrument added afterwards. So nothing has ever reviewed this wire as a design.

Derive the proportion rather than reading a figure:

```sh
wc -l .claude/tools/*trip-wire*.sh                       # this wire against the corpus
git ls-files -- .claude/tools/_webref .claude/tools/webref | wc -l   # its whole population
git log --oneline origin/main..webref-cite-audit-tool -- \
  .claude/tools/webref-generic-core-trip-wire.sh | wc -l  # rounds spent stabilising it
for w in .claude/tools/*-trip-wire.sh; do /usr/bin/time -p bash "$w" >/dev/null; done
```

⚠ **The asymmetry is the point, not the size.** #501's own thesis — the shared enumeration —
is enforced by nothing until slice A-iii wires the Python suites into CI. A slice carrying a
permanent gate for someone else's invariant while its own goes unenforced is what "a slice may
not carry another slice's concern" (umbrella §) forbids.

## §1 What this slice is, and is not

**Is**: one predicate, one instrument, one registration, and the two policy paragraphs that
registration needs.

**Is not**: any change to the generic core, the spec-label map, or the suite. Derive:

```sh
git diff --name-only webref-cite-audit-tool...HEAD
```

Five files. If that command returns anything under `.claude/tools/_webref/`, this slice has
grown a second concern and the boundary has failed.

## §2 The invariant

**K2** — the generic core names no elidex file path, where *file path* means
`.claude/(skills|tools)/` plus **two further segments**, and *generic core* means
`.claude/tools/_webref/` plus the entry script `.claude/tools/webref`.

⚠ **K2 has a closed part and an open part, and only the closed part is mechanisable.** The
closed part is the path predicate above — decidable, and this wire decides it. The open part
is *"no other host path or host policy is named here"*, which no grep decides; #501 §12(3)
owns that split and this memo does not restate it.

⚠ **This wire's green is evidence about the by-DIRECTORY approximation**, not about
`DESIGN.md`'s by-RESPONSIBILITY rule. #501 §2 derives the sites inside the directory that do
adapter work; they are exactly what this wire cannot see.

## §3 Spec coverage map

**This slice cites no spec section, and that is a property of its subject, not an omission.**
Its predicate is over repository paths; its authorities are `DESIGN.md`'s closing rule and
CLAUDE.md's layering mandate, neither of which is a web specification.

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| (none — §0 explains why) | n/a | n/a | n/a | n/a | n/a |

⚠ The Step 0 preflight hard-fails a memo with no resolvable `§<number>` row. That is expected
here and is a known shape in this repo, not a defect in this memo — derive the population of
memos in the same position rather than taking the claim:

```sh
for f in $(git ls-files 'docs/plans/*.md'); do \
  python3 .claude/skills/elidex-plan-review/preflight.py "$f" >/dev/null 2>&1 \
  || echo "$f"; done | wc -l
```

The review that matters for this slice is Steps 1–4, not Step 0's citation arm.

## §4 Artifacts

| Artifact | What it is |
|---|---|
| `.claude/tools/webref-generic-core-trip-wire.sh` | the scanner: population from git, content from git, verdict |
| `.claude/tools/webref-generic-core-trip-wire.controls.sh` | the controls: one fixture per verdict the scanner can reach; sourced, never run standalone |
| `scripts/trip-wires.sh` | one line in `REQUIRED_WIRES`, plus the two driver comments its arrival falsified |
| `.github/workflows/ci.yml` | the ungated-job rationale for the `trip-wires` job |
| `CLAUDE.md` | the paragraph restating that rationale, which names the job comment as canonical |

⚠ **What the wire asserts, and why, is stated once — in the wire**, in the comment block above
`_esc`. This table names the files; it does not restate the mechanism. Three rounds of #501
were spent removing duplicate accounts of exactly that.

## §5 The six questions this review must answer

These are #501 design re-gate 5's findings whose subject is this slice. They are deliberately
**not** fixed in the carried commit: fixing them before the review would be the same mistake
that produced this slice.

1. **Declared blind spots are not filed.** The wire names four classes it cannot see (the
   policy half of `DESIGN.md`'s closing rule; bare top-level names; interpolation; a segment
   containing whitespace). A required gate that names a blind spot must close it or file it as
   an open defect; this one does neither, while #501 §11 records *"zero own deferrals"*.
   **Decide**: which of the four are closable here, and which become slots with a trigger.
2. **A threat-model paragraph that does not bound the file.** It says *"the threat model is
   accident, not adversary — and saying so bounds this file"*, and the file grew after it was
   written; the NUL arm it was written to justify **is** a new mechanism, and the "existing
   fail-closed answer" it appeals to did not exist until that mechanism created it.
   **Decide**: restate the model to describe what the file covers, or remove the arms the
   model excludes, or delete the paragraph rather than let it read as a boundary.
3. **Three controls can pass without testing what they name.** `cachedir`'s fallback satisfies
   its own control, because the fixture loop's `add -A` runs before the `.gitignore` write;
   and the `odd` note asserts *"every other control ran"* in a run where `fifotracked` has
   already failed. **Decide**: the fix is the one this file already states —
   *"a control that cannot fail is not a control"* — applied to the sites it missed.
4. **The CI rationale is false of this wire.** `ci.yml` says *"No toolchain step: the wires
   are grep-only"*; this wire takes its whole population and content from `git`. And the
   paragraph headed *"DELIBERATELY NO RUNTIME FIGURE"* carries three ratios.
   **Decide**: the property the decision actually rests on, stated once.
5. **The split is a `source`d fragment, not a boundary.** The controls file consumes the
   wire's variables and helpers, writes back into its scope, and cannot be executed or tested
   standalone. The per-file threshold is satisfied; the debt is not bounded.
   **Decide**: a real entry point with explicit parameters, or state the split as a
   line-count response rather than as a cohesion seam.
6. **The contention with #510 — see §6.**

## §6 The contention with #510, and why it is a decision

Open PR **#510** registers `plan-memo-umbrella-selftest-trip-wire.sh`, which requires
`python3`, and raises the `trip-wires` job's `timeout-minutes`. This wire's own header rests
on the opposite premise — *an earlier revision used `python3` and broke that premise for the
whole wire set* — and `CLAUDE.md` rests the ungated-job decision on it.

Both this program's memos said a second lander *"takes a textual merge, not a decision"*. That
was false and is withdrawn at both sites. Derive the contending set rather than listing it —
the hand list is what missed the only branch with an open PR:

```sh
for b in $(git for-each-ref --format='%(refname:short)' refs/heads refs/remotes/origin); do
  git diff --name-only origin/main...$b -- \
    scripts/trip-wires.sh .github/workflows/ci.yml CLAUDE.md | head -1 | \
    sed "s|^|$b |"; done | sort -u
```

**The decision this slice owes**: may the required, ungated wire set require an interpreter?
If yes, this wire's *"no toolchain"* rationale is wrong and `CLAUDE.md`'s paragraph with it.
If no, #510's wire needs a different home or a different shape. Either answer is a change to a
repo-wide policy surface, which is why it belongs to a plan-review and not to whoever rebases
second.

## §7 Exit criteria

1. `bash .claude/tools/webref-generic-core-trip-wire.sh` → PASSED, and
   `bash scripts/trip-wires.sh` → 0.
2. Removing the controls file ends the run at exit 2, *"decided nothing"* — verified in both
   directions, since a split that can silently skip its own controls is worse than no split.
3. Every mutation the controls claim to catch is caught. Re-derive by reverting each fix this
   wire's history names and observing a reported control failure — not by reading the list.
4. Each of §5's six questions is answered in this memo before the implementation changes.
5. `git diff --name-only webref-cite-audit-tool...HEAD` returns the five files of §4 and
   nothing else.

## §8 Defer slots

**None registered yet, and that is §5 item 1's subject, not a claim of zero.** The four
declared blind spots are candidates; the review decides which are closable here and which
become slots with an owner and a trigger. A memo that reported "zero own deferrals" while its
instrument declared four unfiled blind spots is the shape this slice exists to correct.
