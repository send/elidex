# Citation-hygiene harness — the branch's external state

**These are §4 and §7 of `2026-08-citation-hygiene-harness-disposition.md`**, cut out of it as a standalone
prereq when that memo reached the 700-line authoring band for the second time. Both sections are cited by
number and the numbering is unchanged.

**Why the seam is here.** Every other section of the disposition states a rule about the block set or about
who may change one: which slice owns which class, what each slice authorises, what the memo may not claim.
These two do not. §4 answers where the harness lives and what that decides for **#505**; §7 is the list of
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
