# Citation-hygiene harness — the branch's external state

**These are §4 and §7 of `2026-08-citation-hygiene-harness-disposition.md`**, cut out of it as a standalone
prereq when that memo reached the 700-line authoring band for the second time. Both sections are cited by
number and the numbering is unchanged.

**Why the seam is here**, stated positively rather than by a universal over what stays. ⚠ **An earlier draft
said *"every other section of the disposition states a rule about the block set or about who may change one"*
and that is false.** `grep -n '^## ' docs/plans/2026-08-citation-hygiene-harness-disposition.md` prints
**eleven** headings — §0.5 / §3, §1, §2, §3, §3b, §4, §5, §6, §7, §8, §9 — two of which (§4, §7) are the
stubs left by this cut, leaving nine. Of those nine, **§0.5 / §3 is a spec coverage table and §1 is a
two-line pointer**, and neither rules anything about the block set. ⚠ **The first correction of this sentence
said "nine" while attaching the command that prints eleven**, which is the same failure one layer down: the
count was taken from the argument rather than from the output. The seam does not need the universal: what §4 and §7 have in common is a **subject**, and it is not
the block set. §4 answers where the harness lives and what that decides for **#505**; §7 is the list of
statements this program has made **outside this repository** — in memory files other sessions append to
daily, and in review threads — together with the queries that re-check them. Their subject is the branch's
relationship to things outside its own text, and their lifecycle ends when #505 closes and the transfer
lands, which is not true of anything left behind.

⚠ **This cut is at the band, not past it.** The first prereq (`2abaea1b`) took the disposition from 1007 to
678 by moving §1; the round-4 repairs brought it back to exactly 700, and this is the commit that was writing
when it got there. That is the order `memory/feedback_touch-time-split-means-while-writing.md` prescribes and
the one the previous cut missed.

⚠ **This file carries no §0.5 / §3 spec coverage map and `preflight.py` hard-fails on it, correctly** — the
precondition is scoped to *plan* memos, and this one authorises nothing and assigns no rule to a slice. The
same check the measurements memo states applies:

```bash
grep -c '^## §[0-9]* What this memo authorises\|^## §[0-9]* The work' \
     docs/plans/2026-08-citation-hygiene-harness-external-state.md    # 0
```

⚠ **That check does not discriminate, and three review axes said so.** `predicate-collapse.md` also scores
**0** and `preflight.py` **passes** it, because what preflight reads is the spec-coverage-map heading. The
needle is a negative control this file satisfies, not a predicate separating it from a plan memo. **The
ground is the positive one**: this file authorises nothing and assigns no rule to a slice — a property of
what it says, not of a heading — and preflight's precondition is scoped to memos that do.

## §4 Where the harness lives, and what that decides for #505

**PR-1 does not need the memos edited.** The authority is a comment in the `.sh` file, on this branch, and
the memos are one cross-check tier that no longer binds (§2 I2 × I4) — so "the `declared by` column is not
machine-readable" is not a reason the collapse must happen on the memo branch.

**The constraint that survives is the analysis note's, already verified there**: *"A harness cannot be stacked
before the slice it measures."* M3 measures seven RED blocks on this branch and M4 shows none is a measurement
failure — each correctly reports that what it measures is absent. `couplings`, A-i's own §12(3) exit
criterion, is RED until A-i lands.

⇒ **Close #505; carry its content onto `webref-cite-audit-tool`.**

**The transfer is D10's output plus the two 2026-08 memos** — not "cherry-pick its harness commits": the
branches replayed each other path-restricted, so a plain log over the harness returns replayed-equivalent
commits too, and the two memos are content no harness-path query returns at all.

⚠ **The recovery pointer must stay a pushed ref, and it is the LATEST transfer commit that has to be
checked.** `git branch -r --contains <c>` answers *which remote branches descend from `c`*, so asking it
about the earliest commit returns a hit as soon as any of them is pushed — four unpushed mechanism commits
returned green that way. `git branch -r --contains $(git log -1 --format=%H)` on the transfer set's newest
commit is the check that can fail. **Do not delete the branch when the PR closes.**

⚠ **The deletion exposure is derived from the authorised command, not listed.** `gh pr close` takes
`--delete-branch` — the sibling flag of the `--comment` §9 requires — and it deletes **the local and the
remote branch on close**. Read `gh pr close --help`, then the two gates that do *not* fire: the
`cleanup-branch` post-hook gates on `gh pr merge` **and** a `MERGED` state
(`~/.claude/hooks/post-merge-cleanup-branch.sh`), so `gh pr close` matches neither; and GitHub's
`delete_branch_on_merge` is likewise merge-time (`gh api repos/send/elidex --jq .delete_branch_on_merge`).
The manual `git push origin --delete` / `git branch -D` paths remain, but they were never the ones adjacent
to the authorised action.

**Ordering.** Closing #505 unblocks #501 immediately — `webref-cite-audit-tool` already carries the harness,
so there is no merge and no rebase. ⚠ That retracts a **public commitment**: #501's 2026-08-02 comment ends
*"This PR will rebase once #505 lands, at which point its diff is A-i's deliverable and memos only."* §7
carries it. #501 then lands on its merits; PR-1a stacks after it, where the memos T1 reads are present and
`couplings` is green.

## §7 Registers, swept by command rather than by anchor

**Line anchors are not usable here** — the memory files other sessions append to daily are stale before the
next reader arrives. The register list is therefore a **query**, and the table says what changes, not where.

```bash
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn '#505' "$MEMORY"/*.md
gh pr view 501 --json comments --jq '.comments[]|select(.body|test("505"))|.body'
gh pr view 505 --json body --jq .body
git grep -n 'DISCHARGED by A-i' origin/webref-cite-audit-tool \
  -- docs/plans/2026-07-citation-hygiene-umbrella.md
git grep -n 'Still owed'        origin/webref-cite-audit-tool \
  -- docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md
```

⚠ **The query above is narrow on purpose.** A wider sweep (`-e 'blocked on' -e 'carve' -e 'CARVED'`) returns
hundreds of hits across a hundred files; it is not evidence for a short table, and a short table built on it
is a hand-written set.

| register | class | what changes |
|---|---|---|
| `MEMORY.md`, L3 bullet | invalidated | #505 open / #501 blocked on it / the `git merge origin/main` re-join |
| `active-lane-detail.md`, the 2026-08-02 carve note | invalidated | pre-split figures and the carve's standing |
| `project_citation-hygiene-program.md` — its `#505` regions | invalidated | the regions are whatever `grep -n '#505'` returns at execution; "region" is undefined and the file gains sections between sessions, so no count is carried. What this row carries instead is the one obligation no other row does: the *previous* register audit lives in this file, and it holds two commitments still open — both memos saying "two owed harness edits" when only `suites` remains, and the umbrella slice table having no row for #505 |
| any other file the query returns | invalidated or provenance | the set **grows during a session**. Classify at execution; do not carry the list |
| **PR #501's 2026-08-02 comment** | invalidated | **read the statements from the query above.** The *class* is what this row states: every sentence that asserts #505's standing, its stacking order, or where a Codex finding gets fixed is invalidated by §4 and §5 — including the comment's own heading (*"the harness is carved out to #505"*, reversed by §4) and *"Your R1–R3 harness findings are its opening review record"* (PR-2 discharges R3-F2/F3 under this umbrella, not #505) |
| **PR #505's own body** | invalidated, and **the most public register there is** | it states the harness's file/line/block counts (all now false); that the PR is stacked before A-i; the carve rationale *"29 of 45 harness citations come from memos other than A-i's"*, whose premise M1 falsifies; and — **the statement §4 most turns on** — that the harness here is *"byte-identical to the harness at #501's head … so rebasing #501 onto this branch leaves zero harness delta"*, which `git diff --stat origin/webref-cite-audit-tool -- 'docs/plans/*A-rederive*'` now contradicts by hundreds of lines. The body also carries a `FAILED BLOCKS` line and a block-by-block classification table that `bash …A-rederive.sh all` no longer reproduces. Closing freezes all of it as the public record, so **the close needs a comment**, not a state change |
| **umbrella, the `DISCHARGED by A-i` bullet** | **restored** | it records the harness split as discharged by A-i naming three SHAs; those SHAs are on `webref-cite-audit-tool`, so the harness returning there makes the bullet true again and retires the owed "add a #505 row" amendment |
| **A-i §8** and its layout figures | **restored / re-derived** | §8 is true again once the harness is on A-i's branch. Its figures — part count, per-part sizes, the line total, the block count, the `_measure` call-site census — are already false at HEAD. ⚠ **Re-derive each with §8's own command, not with a harness block that answers a nearby question.** `wc -l docs/plans/2026-07-citation-hygiene-A-rederive*.sh` gives the sizes and the total; §8 publishes its own block count (`cat …-A-rederive*.sh \| grep -cE '^[A-Za-z_][A-Za-z0-9_]*\(\)'`) and its own per-file `_measure` census (`grep -cE '(^\|[^_A-Za-z])_measure(d)? '`, comments included). Both differ in **subject** from the harness's: measured, §8's census and `inventory`'s `meas` column disagree, and the two censuses must be re-derived side by side with the command above rather than compared against a figure carried here — `a5fab499` moved the part set, so any figure this row states is stale by construction, and §8's block count and `selfcheck`'s differ — which is exactly the pair the analysis note's M6 says must not be carried. Re-deriving them is A-i's at landing |
| **A-i §13's owed `suites` relocation** | **discharged at a different destination, and its rule retired** | §13 owes the move *"to `-common.sh`"* **and states the rule it follows** — *"cited by more than one memo → `-common.sh`"*. D11 measures `-common.sh` surviving as a kernel file and `suites` declaring `umbrella`, so the owed move is discharged **to a destination §13 does not name**, and what retires is the citation-count rule, whose other home is the dispatcher header (§3, prose class) |
| **A-i §15's `AUTHOR_LOCAL` quotation and its `readers` note** | invalidated | §3's `authorlocal` class replaces the remote list with adjacent registration and puts `readers` on it, with its own reason (D3) |

**Three classes, not two**: some registers need a pointer repaired, some need an amendment **retracted**, and
some need a **rule** retired.

## The plan-review gate's pinned map, and why the fix is carved

**Cut out of the disposition's §9 while writing**, because that section entered the 700-800 authoring band.
Its subject is a file outside this branch's own text, which is this memo's subject; §9 keeps the fact that
the carve exists and this holds the record.

⚠ **Raised with an owner and a trigger, and CARVED rather than fixed here: `CSSOM View 1` is not in the
plan-review gate's pinned map.** ⊕ Reproduce with
`python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-08-citation-hygiene-harness-1a-i-beta-classifier.md`,
which prints `⚠ unrecognized labels: ['CSSOM View 1']` and counts the spec as `<CSSOM View 1>`; all three
memos carrying the coverage map warn. The label resolves in webref (`.claude/tools/webref specs cssom-view`
→ `cssom-view-1`), so the fix is **two** entries, not one: `SPEC_LABEL_REVERSE` at
`.claude/skills/elidex-plan-review/preflight.py:51`, whose own comment at `:44-50` requires the sync, and
`_SPEC_LABEL_MAP` at `.claude/tools/_webref/commands/coverage_map.py:13`, which has no CSSOM entry either.
⚠ **An earlier draft of this carve sited the second map in `.claude/tools/webref`** — a 16-line shim holding
neither the map nor the comment — so a PR executing the carve as written would have landed the one-sided edit
the same sentence forbids. ⚠ **It is not landed here,
and the reason is this program's own origin.** PR-A0 bundled a citation sweep with a general-purpose detector,
a shared spec-label refactor **and a behaviour change to `preflight.py`** — that bundle is why this umbrella
exists. Adding a `preflight.py` behaviour change to a docs-only branch would repeat it, and it is a
`.claude/**` edit, whose blast radius is every lane and whose landing needs the full pre-push gate rather
than this branch's memo gate. *Owner*: its own PR. *Trigger*: **any dispatch that reads a memo carrying the
coverage map**, not the next slice boundary. ⚠ **The boundary form was already violated when it was written**:
round 7 is a `/elidex-plan-review`, it read all three memos, and the gate verified three of the map's four
pairs and skipped the CSSOM row — the criterion and its stated reason did not name the same event, so the
carve could be honoured indefinitely while its purpose was missed every round.
