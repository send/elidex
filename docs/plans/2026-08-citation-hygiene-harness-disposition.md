# Citation-hygiene harness — disposition: one home for the block set, then everything else

**Subject**: what to *do* about the re-derivation harness and [PR #505](https://github.com/send/elidex/pull/505).
**Input**: `2026-08-citation-hygiene-harness-predicate-collapse.md` (the analysis note), which settles the
validity predicate and block ownership and explicitly authorises nothing. Its findings are cited here, not
re-derived.

**Why this memo exists separately.** Three plan-review rounds converged on the analysis and shredded the
execution plan that rode with it: R3's nine CRITs all landed in the analysis note's §6, which bundled PR
topology, a 762-line removal, harness self-verification semantics and a CRIT detector fix into one authorised
action set without declaring CLAUDE.md's edge-dense trigger. This memo declares it and answers it: **an
umbrella with a forced PR sequence**, each PR reviewed on its own.

**The reframe that makes the sequence short.** Draft 3 argued for removing 762 lines. That removal was only
ever *necessary* because the harness cannot be acted on at file granularity — and once it can, removal stops
being a design act and becomes a one-line consequence whose timing its own PR can decide. **So this memo
removes nothing, discards nothing, and creates no defer slot.** It makes the removal possible, and lets the
PR that wants it argue for it.

## ⚠ Draft 6, and the one thing a reader of draft 5 must un-learn

**Draft 5's §3 was a hand-written step list, and round 5's five axes between them found six sites it did not
name** — the dispatcher's source loop, `PARTS`, `ORDER`, `MEMOS`, a parser that names nothing and reads a
variable, `budget`'s two globs — **plus a rename step that had vanished from the list while three other steps
still depended on it.** That is the fourth consecutive round in which a hand-written enumeration was completed
by a reviewer. R3 said the block set has "four to six" homes; R4 found a seventh; R5 found six more.

The answer is the one this program has already reached three times: **the enumeration stops being written.**
`rederive homes` derives it — and §3 below is a rule per *class of home*, applied to that command's output,
rather than a list of sites that has to be right.

**Seven mechanism changes landed before this draft was written, each falsified by planting or by running.**
⚠ **The count is itself a finding**: 1 → 2 → 0 → 1 → 4 → 3 across draft windows, each under the same
disclaimer. §9 records the rate, not just the exception.

| commit | what it fixes | how it was falsified |
|---|---|---|
| `9a0ff039` | the declaration was the authority in the prose only — the column, the tally and the move list all printed the **computed** value | a typographically null §15 edit turned 4 correct declarations into 4 DISAGREEs and rc=1; after, 2 reported non-binding and rc=0, with the move list byte-identical either side |
| `9a0ff039` | three parse holes fell through to "undeclared", and `claimed` counted the wrong pattern's hits | planted a one-liner declaration, a duplicate, and a valueless one — all three now named, rc=1 |
| `fc47cde1` | `homes` — the census | run against an acceptance list of every site round 5's axes named (`git show fc47cde1`, which carries the list): all found |
| `259e12cb` | the split `homes` forced, on the primitive/consumer seam | every block's stdout+stderr and exit code, before and after: exit codes identical, output identical but for the four differences the split *is* |
| `7ad42edd` | `homes`'s R3 was a shape rule with no subject test | five noise literals dropped, acceptance list re-run intact. ⚠ It **moved the census output**, which §3 says *is* the work list — so it is not, as §9 claimed of its siblings, a commit that decides nothing PR-1a decides |
| `979e5426` | the census assigns the CLASS; an unclassified home is RED. Three more shape rules got the subject test R3 got, and `guard` learned `_measure … \|\| failed=1` | planted an unruled literal → named, rc=1. ⚠ **My first subject test for R4 was wrong** and dropped the indirect reader R4 exists for; the acceptance list caught it, inspection did not |
| `49b4f645` | a declaration in a heredoc **payload** was authoritative; and T3 made the measurement primitive's own layer unrepresentable | planted payload declaration: entered the MOVE LIST at rc=0 before, reported written-unread at rc=1 after. `# ships-with: kernel` on `_measure`: binding-RED before, `agree=2` after |

⚠ **Two of those four fixes had a defect of their own that inspection did not show.** The declaration needle,
once it admitted a post-brace form, matched the *examples in its own comment* and turned a clean tree red;
and the census, excluding definition lines, dropped `all()`'s own definition line — which **is** the roster,
the single most important home. Both were found by running the fix against an acceptance list, not by reading
it. That order is this memo's method, not an anecdote (§9).

## §0.5 / §3. Spec coverage map

PR-2 authorises the `citations` comparison, which is a comparison of spec §-titles, so this memo carries the
pairs. **All four §-numbers resolve in webref**; that is a different question from whether the *fixture's*
label resolves in the gate's pinned map, and the distinction is load-bearing.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | §-title compare | fixture `labelled` / `dedup` / `malformed` | `citations` | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | §-title compare | fixture `labelled` | `citations` | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | §-title compare | fixture `alias` | `citations` | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | §-title compare | fixture `allunmapped` / `malformed` | `citations` | ✓ | no |

**Breadth**: K=3 specs, M=4 entries — the complete set, and the command finds the call site rather than this
table naming a line number PR-1 is about to move:

```bash
grep -n 'webref heading --exact' docs/plans/2026-07-citation-hygiene-A-rederive-*.sh
.claude/tools/webref heading --exact html 4.10.21     # and 4.10.21.2 / fetch 2.2.5 / cssom-view-1 4.2
```

⚠ **The Spec column is each spec's authoritative label; the Branch column names a fixture that carries a
different one on purpose.** Aliasing is what the fixture set exists to exercise, so the two columns must not
be read as one string. Measured on the labels the fixtures actually write:

```bash
(cd .claude/skills/elidex-plan-review && python3 -c "
import preflight as p
for lab in ['WHATWG HTML','HTML','Fetch','CSSOM VIEW']: print(lab, '->', p.shortname_from_label(lab))
print('pinned-map keys:', len(p.SPEC_LABEL_REVERSE))")
```

`WHATWG HTML` and `HTML` resolve; **`Fetch` and `CSSOM VIEW` do not.** ⚠ Draft 5 said *"three of four are
additionally mapped"* and it is **two** — it counted the memo's spelling, not the fixture's, in the one table
whose subject is that they differ. The count is not this page's to carry, so it is the command above.

⚠ **The harness's own comment beside the `allunmapped` fixture calls it a "24-key pinned map"; the command
above measures 15.** That comment lives in **`fixtures()`**, not `citations()` — ⚠ draft 5's §5 row named the
wrong block, in a table whose whole content is which block to change. Correcting the comment is PR-2 (§5).

⇒ **PR-2's comparison covers all four pairs.** An earlier draft said it *"must compare only the pairs whose
lookup succeeded"* — which would have excluded exactly the row the comparison exists for: the harness records
that this fixture's §-title was corrected **from a fabrication**, and that `verify_citation` checks only that
the number exists, *"so nothing would catch it"*. The real constraint is underneath: **a lookup that fails is
a failed measurement** (`_measure`'s rule) and must never be reported as a matching title.

## §1 Measurements

The analysis note's M1–M7 are the shared basis; re-run them there rather than restating.

⚠ **No expected value is written beside a command in this section, and that is a change from draft 5.** Four
of draft 5's figures did not reproduce five days later, and one of them — a count over the project's memory
directory — **moved during the review itself**, because another session writes to that directory. A digit
whose subject is not in this repository cannot be stamped with a commit. `memory/feedback_verified-claims-go-stale-under-own-later-edits.md`.

```bash
# D1  is part SOURCE ORDER load-bearing? (a glob sorts alphabetically)
for p in integrity audit common Ai Aii Aiii B; do printf '%-10s ' "$p"
  grep -cE '^[A-Za-z_][A-Za-z0-9_]*=' docs/plans/2026-07-citation-hygiene-A-rederive-$p.sh; done
#     then, in a sandbox, reverse the dispatcher's loop and run `… selfcheck`
# D2  do any MEMOS cite the two helpers the collapse renames? (`--include='*.md'`
#     is load-bearing: docs/plans/ also holds the harness, and an unscoped sweep
#     returns the dispatcher's own header comment as if a memo had cited it)
grep -rnE '`(say|fixtures)`|rederive (say|fixtures)' --include='*.md' ../elidex-wt-citeaudit/docs/plans/
# D9  THE HOMES CENSUS. §3's work list is this command's output.
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes
# D10 the transfer set, DISCRIMINATED rather than asserted. A plain `git log`
#     over the file returns replayed-equivalent commits too, which is why draft
#     5's cited check returned five for a four-commit claim.
git log --oneline --cherry-pick --right-only \
    webref-cite-audit-tool...HEAD -- 'docs/plans/*A-rederive*'
git diff --numstat webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'
# D8  the register set (§7). NO expected value: its subject is a directory
#     outside this repository that other sessions append to every day.
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn '#505' "$MEMORY"/*.md
```

- **D1** — no part makes a source-time statement another part reads; measured in a sandbox with the loop
  reversed, `selfcheck` is GREEN and `inventory` produces the same table. **Order is not load-bearing, so a
  sorted glob is a sound replacement.**
- **D2** — **no hits in any memo**, so the `_`-prefix rename changes no citation surface. ⚠ The first form of
  this command was scoped to `docs/plans/` and returned two hits, both the **harness's own** comments. A grep
  whose scope is wider than its claim reads as a citation that does not exist.
- **D3** — the roster derived from `declare -F` (minus `_`-prefixed, minus `all`, minus author-local) is
  **set-identical** to the literal: `comm -23` and `comm -13` both empty. The exclusion list in that proof
  **is** the rule written out, so a different list in the implementation means the rule changed and the proof
  must be re-run. ⚠ Set-identity is not implementability — see D5.
- **D4 — a pure rename moves the computed ship-with.** In a sandbox, applying draft 4's rename and part-list
  steps *and nothing else*, then extending `PART_SLICE` so the new stems are known: the routing disagreement
  shrinks **with no block moved**, and several blocks change group by `T2 part`. Renaming the shared files
  into group-named files makes them *slice-like*, so T2 fires for everything in them and shadows T3. The
  metric improves because the measurement changed. **This is why the declaration, not the computation, is the
  authority** — and why `9a0ff039` was needed before this could be planned.
- **D5 — replacing `all`'s roster literal with an expression breaks its readers, loudly and wrongly.** With
  the roster derived in place, `selfcheck` accuses `"all|$(printf`, `$(declare`, `$AUTHOR_LOCAL` and `'%s|'`
  of being *"dispatched by `all` but defined nowhere"*, and `inventory`'s roster column reads `no` for every
  block. **The derivation must be a function every reader calls, not an expression they each re-parse.**
- **D6 — the `AUTHOR_LOCAL` read is unguarded and hardcodes a filename**, so a rename kills `inventory` with
  an uncaught `FileNotFoundError` traceback rather than its own diagnostic — which every other input to that
  block has.
- **D7 — some blocks have no signal independent of their filename.** Dropping the T2 branch and re-running,
  `anchors` `bmemo` `offline` `partition` `staleclaims` `timing` fall to *"T3 no caller"*. For those the
  declaration is unfalsifiable — the condition under which the analysis note's §2 rejected a declaration for
  `kind`. ⚠ Draft 5 cited `inventory`'s *"on the roster, declared by no memo"* line as covering all six; it
  does not cover `staleclaims`, which is author-local and therefore not on the roster. The claim holds (their
  `declared by` column is `-`); the evidence cited for it did not reach one of them.
- **D9 — the census** finds every site round 5's five axes named, including the indirect reader that names
  nothing, and prints a `guard` column plus the two things it cannot see. §3 is written against it.
- **D10 — the transfer set.** `--cherry-pick --right-only` drops commits whose patch is already present on
  the other branch, which is what makes the answer derived. ⚠ Draft 5 asserted "four commits" and cited a
  command that returns five; the fifth is a path-restricted replay whose content is already there.

## §2 Coupled invariants

Required because the work is edge-dense (`/elidex-plan-review` Pre-condition #3). ⚠ Draft 4's I2 —
*"a block's ship-with is the file it lives in, by construction"* — is **withdrawn**, and it was the premise
its whole step list rested on: T1 routes by declaring memo and the memos are prose on another branch; T2 is
the misroute predicate restated; and D4 shows a rename with no content change moves the computed answer.
**Construction cannot be the authority when the construction is what is under review.**

- **I1 single home** — one fact (which blocks exist, and whose each is) is written down once.
- **I2 declared, then checked** — a block's group is **stated in its body**; the tiers compute a second
  answer and disagreement is reported. The declaration is the authority precisely because it is the one input
  a rename, a memo edit and a file split all leave alone.
- **I3 removal safety** — deleting a part removes its blocks from every derived set automatically.
- **I4 derived input** — the harness's own inputs are derived or declared, never parsed out of prose; and
  **the enumeration of where they live is derived too** (D9). This is the invariant draft 5 lacked.
- **I5 fixed invocation surface** — every block name keeps resolving through the dispatcher path regardless
  of which part defines it. ⚠ Draft 4 wrote *"six memos cite blocks"*, which the dispatcher's own header also
  says. M1 measures **four** (B and C cite it zero times). The stale header is one of the homes §3 collapses,
  and the memo inherited its digit — the exact transfer this program is about.

| pair | intersection | PR |
|---|---|---|
| I1 × I2 | The declaration is the single home only if nothing else can *decide* the answer. Landed at `9a0ff039`: `ship` is the declaration where there is one, and the tiers are the check. Before it, the declaration appeared nowhere but the mismatch line while the column, the tally and the move list printed the computed value. | done |
| I2 × I3 | A declaration must be **independent of the filename**, or renaming a file changes what the harness believes about the blocks in it (D4). | done |
| I2 × I4 | A disagreement is only as good as the tier behind it. T0/T3 are code signals and **bind**; T1 is a heuristic parse of prose on a branch under active revision — a failed measurement by `_measure`'s own rule — and **reports**; T2 is the filename, which the move list already checks. Landed at `9a0ff039`, and it is what stops a memo edit from turning the harness red. | done |
| I1 × I4 | The list of homes must be derived, or a plan against it is complete only by luck — four rounds running, it was not. `rederive homes` (D9). | done |
| I1 × I3 | The roster must be **derived from the definitions**, not listed; deleting parts with the roster untouched makes `selfcheck` accuse blocks that no longer exist. | PR-1a |
| I1 × I5 | Excluding helpers by `_`-prefix requires renaming `say`/`fixtures`; D2 shows no memo cites either, so I5 survives. | PR-1a |
| I2 × I5 | A part **rename** must not change a block name. The dispatcher resolves by name across all sourced parts (D1), and once I2 holds a rename can no longer change the routing answer either. | PR-1a |
| I4 × I5 | The tiers still read the memos (T1), so `inventory` still needs a checkout that has them. That is **pre-existing** — M3 lists `inventory(exit 1)` among seven REDs of one class — and §4 turns on it. | §4 |

⚠ **Four of eight intersections are already discharged, and that is why PR-1 can now be split.** Draft 5
placed 7 of 7 in one PR and argued the base case; round 5's Axis 3 answered that the seam it defended was not
the seam that exists. The seam that exists is **step 1's output**: the move list does not exist until the
declarations do — §3 said so itself while putting both in one PR. So **PR-1a collapses, PR-1b moves**, and
§2's stated reason for not splitting (declaration and its checks must not be in different PRs) is satisfied,
because both are in PR-1a. ⚠ `axes.md`'s base-case exclusion is stricter than CLAUDE.md's phrasing —
*scope が単一 invariant-axis 交点に絞られている場合* — and PR-1a meets it: every remaining intersection is
one predicate, **where the block set is written down**.

## §3 PR-1a — the collapse

**Goal**: after PR-1a, the block set has exactly one home per class, and file-granular action is sound.

**The work list is `rederive homes`, and so is the list of classes.** ⚠ Draft 6 carried the class table here
and claimed it was *"complete by construction: every row the census prints falls into one of these"*. **Five
reviewers measured that false on 31 of 70 rows** — exactly what rounds 3–5 measured against the hand-written
list of *sites* the class table replaced. An enumeration written by hand is incomplete at whatever altitude
it is written. So the mapping moved into the census (`979e5426`), it is **total**, and an unclassified home
is **RED** — falsified by planting an unruled literal. Below is the *rule* per class; the *coverage* is the
command's exit status, and the class names are the census's:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes   # `BY CLASS:` names every class
```

⚠ **Every class the census emits must have a row here, and the count is not this page's to carry.** Two
classes reached draft 6 with no rule at all (`memoset`, `callsite`) though its own un-learn paragraph named
`MEMOS` as one of round 5's discoveries.

| class (the census assigns it) | rule |
|---|---|
| **partset** | one derivation from the glob; every reader calls it. ⚠ **One spelling, not two**: every glob site in the harness already writes `…A-rederive*.sh`, which matches the dispatcher too, so `selfcheck`'s *"N harness parts"* counts it. Draft 6 said "the two globs must become one"; no second spelling exists in code — the hazard is real, the state it described was not. |
| **roster** | `_roster`, a **function**, returning every non-`_`-prefixed, non-author-local, non-`all` definition; `all`, `inventory` and `selfcheck` **call it** (D5: an inline expression makes its readers parse its own shell tokens as block names). ⚠ Two of those three are python inside a heredoc and cannot call a bash function directly; the harness already crosses that boundary with `subprocess.run([… "declare -F"])` and PR-1a uses it rather than keeping a regex. `say` → `_say`, `fixtures` → `_fixtures` (D2). |
| **authorlocal** | a registration **adjacent to each definition**, carrying its reason, as a **shell statement** — `_roster` is a bash function and needs it as runtime data, and bash cannot read comments. ⚠ Draft 5 gave `declare -f` as the reason, which is wrong: the ship-with needle is a Python regex over raw source and never passes through `declare -f` either. `readers` joins it, closing the defect A-i §15 records. |
| **groupvocab** | one `GROUPS` set, validated (landed at `9a0ff039`). ⚠ It is currently *derived from* `ORDER`, so the validation landed and the collapse has not; `PART_SLICE` retires with it, **in PR-1b** — its stated ground is *"once every file is a group"*, which is PR-1b's output, not PR-1a's. |
| **memoset** | ⚠ **A class draft 6 had no rule for.** There are **two disagreeing homes** — `inventory`'s `MEMOS` and `budget`'s `for m in …` loop, which do not carry the same memo set — and the analysis note flagged the second a draft ago. One derivation; both readers call it. |
| **reads** | every read of the harness's or a memo's text must have a **named failure**. ⚠ Draft 6 sized this class from a `guard` column that did not know `_measure … \|\| failed=1` — *the* validity primitive — and so reported `budget`'s four reads as unguarded; fixed at `979e5426`. ⚠ It also hand-counted the rows and the split, and both were wrong. Read them from `homes`; do not restate them here. |
| **prose** | ⚠ **Draft 6's rule named one site** (the dispatcher's header) and the census reports prose homes in every part: each states which blocks it holds and where the shared ones live, and PR-1b's renames falsify many of them. The rule is: a prose home becomes a pointer to `rederive homes` / `rederive inventory`, or is deleted. |
| **mention** | ⚠ **Also nothing to do**, and also a subject test rather than a fallthrough: every vocabulary token on the line sits inside a **quoted string**, so it talks *about* a block instead of enumerating the set. Strip the quoted spans and if any token survives, the line is unclassified and RED. ⚠ This class was introduced by the classifier work below and the plan-coverage checker caught its missing rule within a minute of the checker existing. |
| **callsite** | ⚠ **Also unruled in draft 6, and the rule is: nothing to do.** A line naming two blocks because one *calls* the other is not a place the set is written down. The class exists so that saying so is a rule rather than an omission — the harness's own comment said it and §3 did not. |

⚠ **The rename is NOT in PR-1a, and "one file per group" is withdrawn.** Draft 6 put a rename here and it was
wrong twice over. **(a) It is a repartition, not a rename**: `-common.sh` alone holds three groups, and the
kernel group spans four files, so renaming files to groups requires deciding per block where each lands —
which *is* PR-1b's move list, so the seam PR-1a defends would collapse. Measured, ≥14 of 35 blocks change
file. **(b) `-integrity.sh` and `-audit.sh` both hold only kernel blocks**, so one-file-per-group merges them
back into a single file inside the 700–800 authoring band, undoing the split `259e12cb` took on the
primitive/consumer seam — **by construction**, because the group vocabulary has no token for a layer.

**The invariant file-granular action actually needs is weaker**: *no file holds blocks from more than one
group*. A group may span files, and the kernel must, since its three whole-harness checks alone exceed the
band. ⚠ That is a weaker rule and it needs its own check or it is an escape hatch: `inventory` prints
`part` beside `ships`, so **"no file mixes groups" is a query, and PR-1b's exit criterion is that query plus
an empty `MOVE LIST`** — not a seam an author may declare. Renaming to `<group>[-<seam>].sh` is a consequence
of the moves and belongs with them, in PR-1b.

**Also in PR-1a: every block declares its group.** `# ships-with: <group>` in each block's body, one per
block, each reviewable on its own line — except the two **one-liner** definitions (`_measured`, `say`), which
must be un-one-lined or the declaration is read only by the post-brace form. The mechanism is landed and
green with zero declarations, so this is content, not plumbing.

**What PR-1a does not change: any block's subject on the success path.** ⚠ Draft 4's §9 said *"no behaviour change to any block"*,
which forbade its own steps. The boundary that holds: **`all`, `homes`, `inventory` and `selfcheck` are
exempt by definition, because their subject *is* the block set.** Every other block's inputs and output are
byte-identical across PR-1a. ⚠ **Its verdict on the FAILURE path is not, and cannot be**: the `reads` class
adds a named failure to `budget`'s four reads, and changing what happens when a read fails is the entire
point of that rule. Draft 6 narrowed the exemption *list* and left the *predicate* unscoped, which is draft
4's self-forbidding-step defect at a new site.

## §3b PR-1b — the moves

**Input**: `rederive inventory`'s `MOVE LIST` — the declared blocks whose file contradicts their declaration.
**It does not exist until PR-1a lands**, which is the seam. On a tree with no declarations the harness prints
`MOVE LIST: 0 of 0` and a separate `NO VERDICT` list of undeclared blocks, because a tier guess is not a work
list — ⚠ draft 4 and draft 5 both quoted that guess list as the move list.

PR-1b moves each block to the file matching its declaration, and its exit criterion is that `MOVE LIST` is
empty. This discharges A-i §13's owed `suites` relocation (§7). Nothing else is in it.

## §4 Where the harness lives, and what that decides for #505

⚠ **Draft 4's answer was right and its reason was wrong.** It said PR-1 must edit six memos to make the
`declared by` column machine-readable, so it could not be done on `citation-hygiene-harness`. **The
declaration removes that constraint**: the authority is a comment in the `.sh` file, on this branch, and the
memos are one cross-check tier that no longer binds (§2 I2 × I4).

**The constraint that survives is the analysis note's, already verified there**: *"A harness cannot be stacked
before the slice it measures."* M3 measures seven RED blocks on this branch and M4 shows none is a measurement
failure — each correctly reports that what it measures is absent. `couplings`, A-i's own §12(3) exit
criterion, is RED until A-i lands.

⇒ **Close #505; carry its content onto `webref-cite-audit-tool`.** Same destination as drafts 3–5, on a
ground that does not depend on any step list.

**The transfer is D10's output plus the two 2026-08 memos.** ⚠ Not "cherry-pick its harness commits": the
branches replayed each other path-restricted, so a plain log over the harness returns replayed-equivalent
commits too — draft 5 cited such a command for a four-commit claim and it returns five. ⚠ And draft 4
authorised carrying the *harness* commits and left **its own two files with no destination at all**.

⚠ **The recovery pointer must stay a pushed ref.** `git branch -r --contains <the earliest transfer commit>`
must resolve. **Do not delete the branch when the PR closes.** ⚠ Draft 5 attributed this hazard to the
`cleanup-branch` post-hook; measured, that hook gates on `gh pr merge` **and** a `MERGED` state
(`~/.claude/hooks/post-merge-cleanup-branch.sh`), so `gh pr close` matches neither, and GitHub's
`delete_branch_on_merge` is likewise merge-time. The real exposure is a manual `git push origin --delete` or
`git branch -D`. A sentence that asserted its own non-assumption status while naming a mechanism not in the
path is this memo's charter inverted.

**Ordering.** Closing #505 unblocks #501 immediately — `webref-cite-audit-tool` already carries the harness,
so there is no merge and no rebase. ⚠ That retracts a **public commitment**: #501's 2026-08-02 comment ends
*"This PR will rebase once #505 lands, at which point its diff is A-i's deliverable and memos only."* §7
carries it. #501 then lands on its merits; PR-1a stacks after it, where the memos T1 reads are present and
`couplings` is green.

## §5 PR-2 — the behaviour fixes, and the promises they discharge

PR-2 lands after PR-1b, on blocks placed in their final files.

| fix | why it is PR-2's and not deferred |
|---|---|
| **`citations`** compares the authoritative §-title against the fixture's, per §0.5, treating a failed lookup as a failed measurement | `/elidex-review`'s CRIT-1 on #505; the block ships with A-i, so removal never discharges it |
| **`armmatrix`** binds each row's status | 27 rows print `EXIT=` and the block exits 0; A-ii cites it 5×. ⚠ Draft 4 left this in a footnote with no row while its three siblings had rows — same class, silently ranked lower |
| **`lanes`** routes its remaining bypasses through `_measure` | three are **unguarded**: a failure yields an empty loop, `failed` stays 0, the block returns 0 |
| **`column` / `carvecolumn` / `remedies`** read the child's *stdout* instead of `[ "$rc" -le 1 ]` | **Codex R3-F2 on #501, answered publicly with *"They are being fixed in #505, not here"***. #505 closing must not turn a public commitment into slot content |
| **`ruleset`** asserts `conditions.ref_name` selects `main` | **Codex R3-F3**, same public commitment |
| **`fixtures`' "24-key pinned map" comment** | measured 15 (§0.5). ⚠ Draft 5 billed this to `citations`; the comment is in **`fixtures()`**. A memo planning a citation-hygiene fix must not misattribute the site it is fixing, in the table whose only content is which block to change |

## §6 PR-3 — whether the slice parts ship here at all

**Deliberately undecided by this memo.** After PR-1b, `-Aii.sh` / `-Aiii.sh` / `-B.sh` are exactly their
slices' blocks and nothing else, so removing them is a `git rm` with no reconciliation — a cheap, reversible
decision PR-3's own review can take on the evidence then, including M1's standing fact that **B and C cite the
harness zero times**, and D7's set of blocks whose group nobody has written down and nothing calls, four of
which are Slice B's.

This memo therefore creates **no defer slot and defers nothing of its own**: nothing is discarded, and the
question PR-3 answers is blocked on nothing but PR-1. (Checked against
`memory/feedback_defer-slot-eligibility-audit-at-create.md`'s four questions: zero yes. The question has an
owner, a trigger and a place in the forced order, which is what distinguishes it from a slot.)

## §7 Registers, swept by command rather than by anchor

⚠ **Draft 4's table carried line anchors into files that move under it** — measured, 60–100 lines off — and
two of the three targets are **memory files that other sessions append to daily**. An anchor into them is
stale before the next reader arrives. The register list is therefore a **query**, and the table says what
changes, not where.

```bash
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn '#505' "$MEMORY"/*.md
gh pr view 501 --json comments --jq '.comments[]|select(.body|test("505"))|.body'
gh pr view 505 --json body --jq .body
git grep -n 'DISCHARGED by A-i' webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-umbrella.md
git grep -n 'Still owed'        webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md
```

⚠ **Draft 4's wider grep (`-e 'blocked on' -e 'carve' -e 'CARVED'`) returns hundreds of hits across a hundred
files** and was the evidence for §8's *"the register list is complete — CHECKED"*. It is not evidence for a
short table; the narrow query is.

| register | class | what changes |
|---|---|---|
| `MEMORY.md`, L3 bullet | invalidated | #505 open / #501 blocked on it / the `git merge origin/main` re-join |
| `active-lane-detail.md`, the 2026-08-02 carve note | invalidated | pre-split figures and the carve's standing |
| `project_citation-hygiene-program.md` — **four** `#505` regions | invalidated | the next-session pointer, the superseded-draft-3 section, the carve section, **and the superseded 2026-08-03 section**, which is the *previous register audit* and carries two obligations no other row here does (both memos still saying "two owed harness edits" when only `suites` remains; the umbrella slice table having no row for #505). ⚠ Draft 5 named three and its own query returns four |
| any other file the query returns | invalidated or provenance | ⚠ the set **grows during a session** — it gained a file while round 5 was running. Classify at execution; do not carry the list |
| **PR #501's 2026-08-02 comment** | invalidated — **three statements, and the row has grown once per round** | *"They are being fixed in #505, not here"* → discharged by PR-2 (§5). *"This PR will rebase once #505 lands…"* → retracted by §4's "no merge and no rebase". ⚠ *"the harness is now its own PR, **stacked before this one**"* → **reversed** by §4, which lands #501 first. The comment also repeats the *"29 of 45"* premise M1 falsifies. ⚠ Draft 5 named one, draft 6 two; that the count keeps rising is the finding, not the count |
| **PR #505's own body** | invalidated, and **the most public register there is** | ⚠ No draft's query read it. It states the harness's file/line/block counts (all now false, in the same way §3 says A-i §8 is); that the PR is *"stacked before #501"*; the carve rationale *"29 of 45 harness citations come from memos other than A-i's"*, whose premise M1 falsifies; and — ⚠ **the statement §4 most turns on** — that the harness here is *"byte-identical to the harness at #501's head … so rebasing #501 onto this branch leaves zero harness delta"*, which `git diff --stat webref-cite-audit-tool -- 'docs/plans/*A-rederive*'` now contradicts by hundreds of lines. Closing freezes all of it as the public record, so **the close needs a comment**, not a state change |
| **umbrella, the `DISCHARGED by A-i` bullet** | **restored** | it records the harness split as discharged by A-i naming three SHAs; those SHAs are on `webref-cite-audit-tool`, so the harness returning there makes the bullet true again and retires the owed "add a #505 row" amendment |
| **A-i §8** and its layout figures | **restored / re-derived** | the first is true again once the harness is on A-i's branch; the second is already false at HEAD (§3) and is A-i's to re-derive at landing, from `rederive inventory` |
| **A-i §13's owed `suites` relocation** | **discharged, and its rule retired** | §13 owes the move *"to `-common.sh`"* **and states the rule it follows** — *"cited by more than one memo → `-common.sh`"*. PR-1a abolishes `-common.sh`, so this is not a change of destination but a **retirement of the rule**, whose other home is the dispatcher header (§3, prose class) |
| **A-i §15's `AUTHOR_LOCAL` quotation and its `readers` note** | invalidated | §3's author-local class replaces the remote list with adjacent registration and puts `readers` on it |

⚠ Three classes, and draft 4 had two: some registers need an amendment **retracted**, and some need a **rule**
retired rather than a pointer repaired.

## §8 Claims vs checks

⚠ **No row in this table carries an EXPECTED VALUE.** ⚠ Draft 6 said *"no digit"* and two of its own rows
carry one — the planted-edit falsification records. The values are right and the blanket claim was not, in
the section whose subject is claims-versus-checks. What a row may carry is a **recorded falsification**
stamped to the commit that produced it; what it may not carry is a figure a reader would re-derive today.
Every one of draft 5's numeric rows that round 5 re-derived failed to reproduce, including two marked CHECKED
against a command that does not return the claim.

| claim | check | status |
|---|---|---|
| part source order is not load-bearing | D1 — sandbox, loop reversed → `selfcheck` GREEN rc=0 | CHECKED |
| no memo cites `say` or `fixtures` | D2 | CHECKED |
| the derived roster is set-identical to the literal | D3 — `comm -23` and `comm -13` both empty | CHECKED |
| a pure rename moves the computed group, so the computation cannot be the authority | D4 — sandbox | CHECKED |
| deriving the roster in place breaks its readers | D5 — sandbox | CHECKED |
| the `AUTHOR_LOCAL` read is unguarded and filename-keyed | D6 — sandbox, uncaught traceback | CHECKED |
| some blocks have no signal independent of their filename | D7 — sandbox, T2 branch removed | CHECKED |
| the census finds every site round 5's axes named | D9 vs the acceptance list in `git show fc47cde1` | CHECKED |
| the declaration is the authority in every downstream report, and a memo edit cannot move the move list | `9a0ff039` — planted §15 edit: 4 DISAGREE/rc=1 before, 2 reported/rc=0 after, move list identical either side | CHECKED |
| the three declaration parse holes are closed and named | `9a0ff039` — one-liner, duplicate, valueless: each named, rc=1 | CHECKED |
| the split changed no behaviour | `259e12cb` — every block's stdout+stderr and exit code before/after; and made to fail on purpose | CHECKED |
| the transfer set, discriminated | D10 — `--cherry-pick --right-only` | CHECKED |
| the four `citations` §-numbers resolve in webref; two of the four fixture labels resolve in the pinned map | §0.5's two commands | CHECKED |
| A-i §8's figures are false at HEAD | §8's own figures vs `wc -l` and `rederive inventory` | CHECKED |
| four memos cite the harness, not six | M1 | CHECKED |
| the `cleanup-branch` hook cannot fire on a PR close | its two gates, read | CHECKED |
| PR-1b leaves the `MOVE LIST` empty | — | **UNCHECKED** — PR-1b's own exit criterion, and it cannot be evaluated before PR-1a writes the declarations |
| the set of filename-only-confirmed blocks after the moves | — | **UNCHECKED** — the declarations do not change what computes them, but PR-1b's moves change T2's input. Read it from the block afterwards; do not predict it |
| the register list is complete | §7's query, re-run **at execution** | **UNCHECKED at authoring, and unfixably so** — its subject is a directory other sessions write to, and it grew during round 5. ⚠ Draft 4 marked this CHECKED against a grep two orders of magnitude wider than its table |

## §9 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: closing #505 **with a closing comment** (§7) and its
branch **retained**; carrying D10's commits and this memo pair onto `webref-cite-audit-tool`; and **PR-1a's
scope as stated in §3**.

**Every PR under this umbrella takes its own `/elidex-plan-review` before implementation** — PR-1a, PR-1b,
PR-2 and PR-3, per CLAUDE.md's edge-dense rule *(b) 各 PR は実装前に `/elidex-plan-review` 必須*. ⚠ Draft 5
named plan-review for PR-1 only, which left PR-2 — six behaviour fixes across six blocks, two of them public
commitments — reading as pre-authorised.

**Does not authorise**: changing any block's **subject** for any block other than `all`, `homes`, `inventory`
and `selfcheck`, whose subject is the block set (behaviour fixes are PR-2, §5); any removal (PR-3, §6);
deleting the `citation-hygiene-harness` branch; editing a status register before §7's query is re-run; or
creating a defer slot.

⚠ **Mechanism has landed on this branch ahead of review in every draft window but one, and the rate is
rising**: 1 → 2 → 0 → 1 → 4 → 3, each time under a disclaimer saying it is not a licence. Draft 5's stated
justification is **withdrawn**: it said the defects "appeared only when declarations were planted" and could
not be found by inspection — true — and concluded that landing was therefore necessary. It is not; §1's own
preamble explains how to exercise an **uncommitted** mechanism in a `git clone --local` sandbox, and D4–D7
are all done that way. The reasons that do hold:

1. **The order is user-ratified.** *"設計を書く前に機構を landing して反証する"* was given as the method for
   these drafts, and repeated for any new mechanism in them.
2. **A working-tree edit is invisible to review.** Round 5's and round 6's agents each cloned HEAD; several
   planted declarations into the mechanism and found holes in it — including two the author had missed.
   Uncommitted, none of that would have been reachable, which is the failure mode §1's preamble warns about
   one level up.
3. **It decides nothing PR-1a must decide** — ⚠ **true of six of the seven, and false of `7ad42edd` and
   `979e5426`**, which changed the census output §3 calls the work list. A commit that moves the work list
   decides part of PR-1a's scope, and saying otherwise of all of them was the same over-claim this memo keeps
   finding elsewhere.

**The rate is the datum a reader should weigh, not the disclaimer.** Each PR under this umbrella takes its own
`/elidex-plan-review` before implementation — PR-1a, PR-1b, PR-2, PR-3 — and PR-1a's inherits a landed design
as ground rather than as a proposal, which it should be told.
