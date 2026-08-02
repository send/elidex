# Citation-hygiene harness — design note: the predicate collapse

**Subject**: the re-derivation harness on branch `citation-hygiene-harness`
([PR #505](https://github.com/send/elidex/pull/505)) — its validity predicate, its block set, and its own
verification.

**This note is not a description of the diff.** `/elidex-review` on #505 returned 2 CRIT / 31 IMP / 17 MIN, and
the decision taken was **not** to patch them: a retro memo restating the harness line by line reproduces the
anti-pattern this program exists to fight, and the artifact already carries "measured" comments that disagree
with what its own blocks print. So this note answers exactly three questions and stops. Where an answer
dissolves a finding rather than fixing it, that is stated as the outcome.

## §0.5 / §3. Spec coverage map

**No spec surface.** This note settles a design question about a shell harness under `docs/plans/`; it touches
no spec-defined behaviour and cites no spec.

⚠ Under the **pre-A-ii** gate this memo hard-fails (heading, no table) — A-ii's §4.2.5 marker is what makes
"no spec surface" a declarable state. Same position A-iii's memo is in, and deliberate for the same reason:
inventing two citations here would earn a `citation verify: ok` headline for a memo with nothing to verify.

## §1 The measurements this note reasons from

Every quantity below is a command. **Re-run before citing; do not carry a digit forward.** `M1`–`M2` run in
`elidex-wt-citeaudit` (branch `webref-cite-audit-tool`, which is where the memos are); `M3`–`M6` run in
`elidex-wt-harness` at `12281e3b`.

| | command | what it answered |
|---|---|---|
| **M1** | `for m in Ai-spec-label-map Aii-gate-failure-semantics Aiii-suite-scheduler B-detector-correctness C-policy-retirement umbrella; do printf '%-28s ' "$m"; grep -c 'rederive' "docs/plans/2026-07-citation-hygiene-$m.md"; done` | who cites the harness at all |
| **M2** | `sed -n '/^## §15/,/^## /p' docs/plans/2026-07-citation-hygiene-A{i,ii,iii}-*.md` | each memo's **declared** block list (§15 is authoritative) |
| **M3** | `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all` | which blocks are RED on the branch that ships them |
| **M4** | `grep -c 'MEASUREMENT FAILED'` over M3's output | whether those REDs are verdicts or measurement failures |
| **M5** | `git grep -lE 'A-rederive' -- . ':!docs/plans/'` and `grep -rlE 'rederive\|citation-hygiene' mise.toml .github/ scripts/` | what runs the harness outside `docs/plans/` |
| **M6** | `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh selfcheck` | the harness's own view of its size |

Readings taken 2026-08-03 at `12281e3b`:

- **M1** — A-i 27, A-ii 19, A-iii 10, umbrella 4, **B 0, C 0**. The carve's "serves four slices" premise is
  false at the two slices that were supposed to be downstream of it.
- **M2** — A-i: `citations keysets readers regions couplings budget` (+ `lanes`, §13). A-ii: `citations column
  carvecolumn instruments remedies reloadstale armmatrix budget couplings marker lanes`. A-iii: `suites filters
  suiteset ruleset budget couplings lanes`. Umbrella: `suites`, `lanes`.
- **M2 ∖ roster** — six blocks are on `all`'s roster and in **no** memo's §15: `selfcheck partition anchors
  offline timing bmemo`. Cross-checked by an independent proximity census — for each memo × block name,
  ``grep -cE 'rederive.{0,40}\bNAME\b'`` — which returns the same six. ⚠ That regex needs GNU-style `\b`; the
  BSD word classes `[[:<:]]`/`[[:>:]]` return 0 here.
- **M3** — `FAILED BLOCKS: partition(exit 1) keysets(exit 1) regions(exit 2) offline(exit 1) couplings(exit 1)
  bmemo(exit 1)`, run exit 1. **6 of 22.**
- **M4** — **0**. None of the six is a measurement failure; every one is a block correctly reporting that what
  it measures is not there.
- **M5** — no match, both. **0 tests, 0 CI, no `mise` task.** Nothing outside `docs/plans/` names the harness.
- **M6** — `7 harness parts, 32 blocks, 22 on 'all's roster`.

## §2 Q1 — what the canonical validity predicate is

**There is no single one, because the harness holds three kinds of block and only two of them have a predicate
at all.** That is the collapse: not one predicate replacing thirteen shapes, but a partition that shows most of
the shapes have nothing to be a predicate *of*.

| kind | what the block produces | valid iff | primitive | present in |
|---|---|---|---|---|
| **J1 derive** | a quantity a memo cites | **the command ran** | `_measure` | `citations` `keysets` `regions` `budget` `marker` `timing` `partition` `lanes` |
| **J2 assert** | a verdict on a written-down invariant | a **comparison** against a stated expectation ran *and* held | `_measure` + an explicit `[ x = expected ]` | `couplings` `suites` `suiteset` `filters` `ruleset` `selfcheck` |
| **J3 instrument** | evidence a human author reads to *decide* a question no memo has settled | — **malformed question** | none | `armmatrix` `instruments` `reloadstale` `column` `carvecolumn` `remedies` `_proto` `_runner` |

**J1 is closed and needs nothing further.** `_measure` makes "the command did not run" unrepresentable as a
pass by construction — `!FAILED(rc=N)` is not a number, so no `= 0` gate can accept it. A J1 block cannot know
whether 24 is the *right* answer; it can only know 24 was measured. Umbrella `:91` ("counts are commands") asks
for exactly that and no more.

**J2 is where umbrella `:92` bites** — *"a claim is admissible only if something mechanically checks it."* A J2
block that prints two numbers side by side and never compares them is a J1 block wearing a verdict's clothes.
That is the whole content of CRIT-1: `citations` (`-common.sh:78`) prints the authoritative §-title beside the
fixture's and never compares, and its own comment says it exists because nothing else would catch a fabricated
title. **Keep and fix — one comparison, not a rewrite.** A-i cites it, and it is the program's own
anti-fabrication guard.

**J3 has no pass condition, and that dissolves the rest.** `armmatrix` runs three candidate reporting
predicates side by side *because A-ii has not decided which one is right*; `instruments` runs three candidate
capability instruments for the same reason. Asking what verdict such a block should return is asking the
harness to answer the question the author is using it to explore.

This is why the `_measure --ok <status-set>` widening that looks like the obvious fix **cannot** close the three
`rc -le 1` sites (`column:34`, `carvecolumn:57`, `remedies:207`): `preflight.py` returns 1 for a real HARD FAIL,
a missing fixture, an uncaught exception **and** a failed `cd`, so no status set discriminates — the
discriminator is in the child's *stdout*, which those blocks print and never read. But widening was never the
right move anyway. **A J3 block on a roster that exits non-zero is a category error**; the sites stop needing a
predicate when they stop being on a shipped roster.

Same disposition for CRIT-2: `armmatrix` (`-Aii.sh:213`) binds no row's status, 26 of 27 rows printed `EXIT=1`
and the block exited 0. **Do not fix.** A-ii's memo is unwritten; when it is written, the matrix's *result* is a
decision the memo records, and what ships alongside it is a **J2 pin on the decided arm** — one assertion, not
a 60-line exploration.

**The rule that follows**: a block declares its kind, and only J1 and J2 ship. J3 belongs to the authoring
session that produced the memo, not to a shipped artifact.

## §3 Q2 — which blocks earn existence

**Rule**: umbrella `:89` — *"a slice may not carry another slice's concern"* — applied one level deeper. **A
harness part ships in the PR of the slice that cites it, and no earlier.** Same rule that justified the carve.

| disposition | blocks | grounds |
|---|---|---|
| **kernel** — slice-independent | `$REPO_ROOT` `_measure` `_measured` `selfcheck` `say` `$MAIN` `$PF` | belongs to no slice; green on any branch |
| **A-i's** | `citations` `keysets` `readers` `regions` `couplings` `budget` (+ `lanes`, `fixtures`, `_wtscan`) | M2. `fixtures` and `_wtscan` are helpers, placed with their **first** citer |
| **A-ii's** | `column` `carvecolumn` `instruments` `remedies` `reloadstale` `armmatrix` `marker` `_runner` `_proto` | M2; **all J3 except `marker`** |
| **A-iii's** | `suites` `filters` `suiteset` `ruleset` | M2 |
| **no consumer** | `partition` `anchors` `offline` `timing` `bmemo` `staleclaims` | M2 ∖ roster. Routed to B "by the quantity they derive" — a guess; **B cites the harness 0 times** (M1) |

⚠ **One block splits across slices**: `budget` (A-i's, M2) calls `_proto`, and `_proto`'s whole subject is the
growth `preflight.py` takes under **A-ii** — which A-i has not touched since draft 3 and §12(1) forbids. So the
prune moves `_proto` **and `budget`'s "preflight.py's LOGIC growth under A" limb** to A-ii, leaving `budget` a
pure file-size census.

### The measurement that decides the *ordering*, not just the set

M3's six REDs are **not six defects. They are one class**, and the class is the ordering:

| RED block | what it reads | who creates it |
|---|---|---|
| `partition` `keysets` `regions` `offline` | `.claude/tools/_webref/spec_labels.py` — **ABSENT** on this branch | **A-i** |
| `bmemo` | `docs/plans/…-B-detector-correctness.md` — **ABSENT** | **B** |
| `couplings` | K2's two pre-existing sites (`_webref/cli.py:78`, `.claude/tools/webref:5`) | **A-i discharges them** |

*(verify: `ls .claude/tools/_webref/spec_labels.py docs/plans/*B-detector-correctness.md`;
`git grep -nE '\.claude/(skills\|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+' -- .claude/tools/`)*

Every one is RED because it measures an artifact **another slice creates**, or an invariant **another slice
discharges**. Note the worst case: `couplings` is the most-cited block in the program (M1/M2: A-i ×7, A-ii, A-iii)
and §12(3)'s exit criterion, and on the branch that ships it, it is RED and stays RED until A-i lands. Three of
A-i's own six blocks are RED here too.

**A harness cannot be stacked before the slice it measures.** #505's ordering is the defect the review found six
separate times — and it is the same fact Axis 5 reached from the other side: #505 lands 1711 lines whose only
describing document (A-i §8) is in the PR that lands second.

### Two dispositions for #505, for plan-review to adjudicate

- **(a) #505 ships the kernel only.** Every slice's part lands in that slice's PR. ⚠ **Open**: `selfcheck`
  derives its scope from `all`'s roster, so a kernel-only PR ships a check ranging over an empty roster — the
  exact "a check that read no file reports no problem" shape it was written against. Needs an answer before
  this option is viable.
- **(b) #505 is closed; the kernel and `-Ai.sh` return to #501, and A-ii / A-iii / B parts wait for their
  memos.** ▶ **Recommended.** No new PR, no stacking, no merge-back dance, no describing-document inversion,
  and every shipped block is green because A-i's artifacts exist there.

The objection to (b) is that it reinstates what the carve fixed. It does not: the carve's evidence was that
**29 of 45 citations came from other slices' memos** and that three review rounds changed zero lines of A-i's
434-line deliverable. The correct application of `:89` to that evidence is to **split the harness by slice** —
which (b) does — not to hoist all of it ahead of every slice, which is what makes six blocks red on arrival.
Under (b) A-i carries the kernel plus its own six blocks against a 434-line deliverable, which is proportionate;
under #505 as it stands it carries **all** of it, which is not.

## §4 Q3 — what verifies the harness

**Today: nothing** (M5 — 0 tests, 0 CI, no `mise` task, no reference outside `docs/plans/`). The answer that
follows from §2 is that verification is **per kind, and mostly by construction**:

| kind | what verifies it | why that is enough |
|---|---|---|
| **J1** | `_measure`, by type | "did not run" is unrepresentable as a pass. A test cannot add to a type. |
| **J2** | a **`# planted:` provenance line** naming the exact command that reddens the block | an assertion nobody has seen fail is not known to be an assertion (CLAUDE.md 検証の作法) |
| **J3** | nothing — **it does not ship** | |
| harness-level | `selfcheck` | the one property that spans blocks |

**`selfcheck` keeps its property and gains one.** Today it enforces *every roster block ends in an explicit
`return`* — chosen because that was the shared consequence of every un-routed measurement found so far, and
because "is this command a measurement?" is a taste judgement no regex holds. The kind declaration removes that
excuse: each block's header states `# kind: J1` or `# kind: J2`, and `selfcheck` checks the shape implied — a
J2 block must contain a comparison and carry a `# planted:` line. A block with no kind does not ship.

⚠ **One real defect inside the kernel, to fix rather than dissolve**: `selfcheck`'s parser recognises a
definition only when it closes with a bare `}`, so it silently drops `all()`, which closes with `; }`. Its scope
can shrink with no signal — in the one thing that verifies anything. (M6 reports 32 blocks against §8's cited
grep of 33; same cause.)

**Decision: no CI wiring, and this closes the question.** The harness's consumers are memos under review and its
readers are reviewers, who run `all`. Putting a script that creates git worktrees, calls `gh api` and reaches
the network into every lane's gate buys nothing for a `docs/plans/` artifact — and the *supported* scheduling
concern is A-iii's, already planned, over the Python suites rather than this.

## §5 Claims vs checks

| claim | check | status |
|---|---|---|
| B and C cite the harness zero times | M1 | CHECKED |
| six roster blocks are in no memo's §15 | M2, two independent derivations | CHECKED |
| six of 22 roster blocks are RED at `12281e3b` | M3 | CHECKED |
| none of those six is a measurement failure | M4 | CHECKED |
| each RED reads an artifact another slice creates / an invariant another slice discharges | §3 table's `ls` + `git grep` | CHECKED |
| nothing verifies the harness | M5 | CHECKED |
| `selfcheck` drops `all()` | reproduces on both branches (recorded in the program memo) | CHECKED |
| J1/J2/J3 partitions the harness exhaustively | — | **UNCHECKED** — a reading of each block, not a command. Plan-review should test the partition on `marker`, `budget` and `lanes`, which are the least obvious placements. |
| option (b) leaves A-i's blocks green | — | **UNCHECKED** — requires running `all` on `webref-cite-audit-tool` with the pruned roster, which does not exist yet |

## §6 What this note authorises, and what it does not

**Authorises** (after `/elidex-plan-review`): pruning the harness to the kernel + one slice's parts; deleting
the J3 blocks; one comparison added to `citations`; the `# kind:`/`# planted:` declaration and `selfcheck`'s
`; }` fix.

**Does not authorise**: patching the 33 CRIT/IMP findings individually. Two independent gates — the cumulative
design re-gate's own-ideal test and `/elidex-review` Axis 2/3 — reached the same place, that the mechanism
itself is the open question. Patching builds the edifice higher. **New evidence, not re-reading, is what would
reopen that.**
