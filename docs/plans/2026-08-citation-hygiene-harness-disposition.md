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

## §0.5 / §3. Spec coverage map

PR-2 authorises the `citations` comparison, which is a comparison of spec §-titles, so this memo carries the
pairs. **All four resolve in webref** (`heading --exact`, rc=0 each); three of four are additionally mapped by
`preflight.py`'s pinned label table. Those are different resolvers and the distinction is load-bearing.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | §-title compare | fixture `labelled` / `dedup` / `malformed` | `citations` (`-common.sh:85`) | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | §-title compare | fixture `labelled` | `citations` (`-common.sh:86`) | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | §-title compare | fixture `alias` | `citations` (`-common.sh:87`) | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | §-title compare | fixture `allunmapped` / `malformed` | `citations` (`-common.sh:88`) | ✓ | no |

**Breadth**: K=3 specs, M=4 entries — the complete set
(`grep -n 'webref heading --exact' docs/plans/2026-07-citation-hygiene-A-rederive-common.sh | wc -l` → 4) →
single PR for the citation surface.

⚠ **`CSSOM View 1` resolves in webref and is absent from `preflight.py`'s pinned map** — two different
things, and draft 1 of this memo conflated them. `webref heading --exact cssom-view-1 4.2` returns
`§4.2 The MediaQueryList Interface`, rc=0. What has no key is `preflight.SPEC_LABEL_REVERSE`, which is why
`-common.sh:37-45` picks `CSSOM VIEW` for the `allunmapped` fixture — *"absent from the 24-key pinned map, so
this is all-unmapped AFTER A"* (⚠ that map measures **15** keys today, not 24; the quote is the harness's,
the count is not re-derived by it).

⇒ **PR-2's comparison covers all four pairs.** The conflated version said it *"must compare only the pairs
whose lookup succeeded"* — which would have excluded exactly the row the comparison exists for: `-common.sh`
records that this fixture's §-title was corrected **from a fabrication**, and that `verify_citation` checks
only that the number exists, *"so nothing would catch it"*. The real design constraint is the one that was
underneath: **a lookup that fails is a failed measurement** (`_measure`'s rule) and must never be reported as
a matching title — which is a statement about failure handling, not about which rows participate.

## §1 Measurements

The analysis note's M1–M7 are the shared basis; re-run them there rather than restating. Two further
measurements belong to this memo because they establish that its central move is possible.

```bash
# D1 — is part SOURCE ORDER load-bearing? (the collapse replaces the hardcoded
#      part list with a glob, and a glob sorts alphabetically)
for p in integrity common Ai Aii Aiii B; do printf '%-10s ' "$p"
  grep -cE '^[A-Za-z_][A-Za-z0-9_]*=' docs/plans/2026-07-citation-hygiene-A-rederive-$p.sh; done
#      then, in a `git clone --local` sandbox, reverse the dispatcher's loop and run:
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh selfcheck
# D2 — do any MEMOS cite the two helpers the collapse renames?
#      `--include='*.md'` is load-bearing: `docs/plans/` also holds the harness
#      itself, and an unscoped sweep returns the dispatcher's own header comment
#      as if a memo had cited the block. That is the finding this program is
#      about, so the scope is written down rather than assumed.
grep -rnE '`(say|fixtures)`|rederive (say|fixtures)' --include='*.md' \
     ../elidex-wt-citeaudit/docs/plans/
#      the harness's OWN references, which PR-1 step 4 has to rewrite:
grep -rnE '(^|[^_A-Za-z])(say|fixtures)([^_A-Za-z]|$)' --include='*.sh' docs/plans/ \
  | grep -vE '^\S+:[0-9]+: *(say|fixtures)\(\)' | wc -l
```

- **D1** — source-time statements: `integrity` **2**, `common` **4**, every slice part **0**, and none is read
  by another part at source time (`REPO_ROOT` and `MAIN`/`PF`/`HDR`/`AUTHOR_LOCAL` are read at *call* time;
  the dispatcher's `cd "$REPO_ROOT"` runs after all sourcing). Measured in a sandbox with the loop reversed to
  `B Aiii Aii Ai common integrity`: `selfcheck` → `7 harness parts, 33 blocks, 23 on 'all's roster` /
  `VERDICT: GREEN`, **rc=0**; `inventory` runs and produces the same table. **Order is not load-bearing, so a
  sorted glob is a sound replacement.**
- **D3** — the derived roster is **byte-identical** to today's 23-name literal, so PR-1 step 4 is a pure
  refactor and can be proved so *before* implementation:

  ```bash
  bash --norc --noprofile -c '
  for p in integrity common Ai Aii Aiii B; do . "docs/plans/2026-07-citation-hygiene-A-rederive-$p.sh"; done
  derived=$(declare -F | sed "s/^declare -f //" | grep -v "^_" \
            | grep -vxE "say|fixtures|lanes|staleclaims|readers" | sort)
  today=$(sed -n "/^all() { set -- /,/local failed/p" docs/plans/2026-07-citation-hygiene-A-rederive.sh \
          | sed "s/all() { set -- //;s/\\\\$//;s/local failed.*//" | tr " " "\n" | grep -v "^$" | sort)
  comm -23 <(printf "%s\n" $derived) <(printf "%s\n" $today)   # in derived, not on the roster
  comm -13 <(printf "%s\n" $derived) <(printf "%s\n" $today)'  # on the roster, not in derived
  ```
  Both empty; 23 = 23. The exclusion list in that command **is** step 4's rule (`_`-prefix) plus step 5's
  registrations, written out — so if PR-1's implementation needs a different list, the rule changed and the
  proof has to be re-run.
- **D2** — **no hits in any memo**, confirming M7's `declared by` = `-` for both: the `_`-prefix rename
  changes no citation surface. ⚠ The first form of this command was scoped to `docs/plans/` and returned
  two hits — `A-rederive.sh:42` and `-common.sh:7`, both the **harness's own** comments. A grep whose scope
  is wider than its claim reads as a citation that does not exist; the scope is now in the command.
  Inside the harness there are **17** non-definition references to the two names, and rewriting them is
  PR-1 step 4's work, not a citation-surface cost.

## §2 Coupled invariants

Required because the work is edge-dense (`/elidex-plan-review` Pre-condition #3). Five invariants hold
simultaneously; the intersections are where every earlier draft failed.

- **I1 single home** — one fact (which blocks exist, and whose each is) is written down once.
- **I2 file = group** — a block's ship-with is the file it lives in, by construction rather than by table.
- **I3 removal safety** — deleting a part removes its blocks from every derived set automatically.
- **I4 derived input** — `inventory`'s own inputs are derived, not parsed out of prose.
- **I5 fixed invocation surface** — six memos cite blocks through the dispatcher path; every block name keeps
  resolving regardless of which part defines it.

| pair | intersection |
|---|---|
| I1 × I2 | "one home" and "file = group" agree **only if** a block's home *is* its group's file. The 10 misrouted blocks / 640 lines are not bookkeeping — moving them is what makes I1 true. |
| I1 × I3 | The roster must be **derived from the definitions**, not listed. Measured: deleting two parts with the roster untouched makes `selfcheck` accuse **15 blocks that no longer exist**. |
| I1 × I5 | Excluding helpers by `_`-prefix requires renaming `say`/`fixtures`; D2 shows no memo cites either, so I5 survives. |
| I2 × I5 | A part **rename** must not change a block name. The dispatcher resolves by name across all sourced parts (D1: reverse order works), so renaming files is invisible to every memo. |
| I2 × I4 | Ship-with is computed from declarations. While declarations are **prose**, a purely typographic rewrite of A-i's §15 moves A-i from 7/415 to 3/171 — a file's worth of blocks. I2 is meaningless until I4 holds. |
| I3 × I4 | The machine-readable declaration lives in the **memo**, and the memos live on `webref-cite-audit-tool`. **This is what forces the harness onto that branch** — see §4. |
| I1 × I4 | `AUTHOR_LOCAL` and "cannot run unattended" are the same category (`lanes`, `staleclaims` are machine-local; `readers` takes a required `<symbol>` and A-i §15 already flags it as being in neither list). One marker, registered adjacent to the definition, not a remote list. |

## §3.0 ⚠ SUPERSEDING — ship-with is DECLARED at the definition, not computed from prose

**Three of R4's CRITs converge on one answer, and it is not in the step list below.** Recorded here in full,
because it changes PR-1's core and the steps in §3 are **not yet re-derived against it** — that is draft 5's
subject and it needs its own review round.

What R4 measured:

1. **"file = ship-with group" cannot hold, because ship-with is computed from memo prose.** T1 is *earliest
   declaring memo*, and the memos are `.md` on another branch. Measured: adding two A-ii block names to A-i's
   §15 — **no `.sh` touched** — moves `anchors` and `timing` to A-i and takes the misroute list from 10/640 to
   12/677. Worse: blanking the umbrella's three `rederive` citations **dissolves the entire `-umbrella.sh`
   group**, redistributing six blocks / 396 lines and taking `_measure` with it. Machine-readability (§4)
   changes the *parser*; it does not change the *authority location*.
2. **The check is tautological for everything T2 routes.** `T2` is `ship = PART_SLICE[part]` and the misroute
   predicate is `PART_SLICE[part] != ship` — the same expression. Measured: moving `anchors` verbatim from
   `-Aii.sh` into `-B.sh` is silently re-attributed to B and the list stays at 10/640. After PR-1 makes every
   file a group, **15 of 34 blocks would route by reading their own filename**, and the caller-derived answer
   is discarded for exactly the blocks `inventory` was built to place.
3. **§4's forcing constraint is overstated, and §2 already contains the alternative.** `inventory`'s own
   docstring provides for cross-branch memos (*"pass a sibling worktree's when the harness and the memos are
   on different branches"*), which is how file A's M7 invokes it. And §2's `I1 × I4` row already adopts the
   right pattern for the sibling fact: `AUTHOR_LOCAL` becomes *"One marker, registered adjacent to the
   definition, not a remote list."*

⇒ **Apply `I1 × I4`'s pattern to ship-with itself.** Each block declares its group at its definition site;
`inventory`'s four tiers stop being the authority and become a **cross-check** — declared ≠ computed is RED.
That is the "declaration ↔ machine signal cross-check" shape rejected for `kind` in file A §2, and it is right
here for the opposite reason: for `kind` no computed signal existed, so a declaration would have been
unfalsifiable. For ship-with a computed answer *does* exist (T0–T3), so the declaration is checkable and the
disagreement is a finding.

What it dissolves, at once: a memo edit can no longer relocate a file (1); the filename stops being its own
authority (2); the memo edits stop being PR-1's prerequisite, so **§4's inference and therefore #505's fate
must be re-derived** (3); and `_measure` can stay in the kernel with an explicit declaration instead of being
exiled into a shipping cohort — R4's third Axis-1 CRIT, that PR-1 relocates the harness's entire validity
primitive into a file named for PR ordering.

⚠ **Everything from §3 to §9 below is written against the superseded design.** R4's remaining findings against
that step list — `PART_SLICE`, the two roster parsers and the `AUTHOR_LOCAL` filename read as unnamed readers;
`selfcheck`'s undashed glob; §9's clause forbidding §3 step 6; PR-1 carrying 7 of 7 coupled intersections;
the two memos having no destination; §7's line anchors being 60–100 lines off — are **not patched into it**.
They are re-derived against the declaration design in draft 5, because patching a superseded step list is the
edifice-building this program exists to stop.

## §3 PR-1 — the collapse

**Goal**: after PR-1, the block set has exactly one home and file-granular action on the harness is sound.

1. **Part list ← sorted glob.** The dispatcher's `for _part in integrity common Ai Aii Aiii B` and
   `inventory`'s `PARTS` both go; both become `…-A-rederive-*.sh`, which `selfcheck` already uses. Justified
   by D1.
2. **Parts renamed to ship-with groups**: `-kernel.sh`, `-umbrella.sh`, `-Ai.sh`, `-Aii.sh`, `-Aiii.sh`,
   `-B.sh`. `-common.sh` and `-integrity.sh` cease to exist as concepts — they were "shared" and "not on the
   slice seam", which M7 shows is not a partition of anything.
3. **The 10 misrouted blocks move** to the file matching their ship-with, per M7's routing-vs-shipping report:
   `_measure` `_measured` `_proto` `budget` `lanes` `suites` → `-umbrella.sh`; `_wtscan` `citations`
   `couplings` `fixtures` → `-Ai.sh`. This discharges A-i §13's owed `suites` relocation.
4. **Roster ← definitions.** `all` dispatches every function defined in a sourced part that is not
   `_`-prefixed, not registered as unattendable, and not `all` itself. `say` → `_say`, `fixtures` →
   `_fixtures` (D2: no citation surface). The 23-name literal list goes.
5. **One marker for "`all` cannot run this".** `AUTHOR_LOCAL="lanes staleclaims"` — a remote list in
   `-common.sh` naming a block defined in `-B.sh` — becomes a registration adjacent to each definition,
   carrying its reason. `readers` joins it, closing the defect A-i §15 records.
6. **`inventory`'s ship-with becomes a check, not a report.** Computed routing ≠ the file the block lives in
   → **RED**. After step 3 the list is empty; it stays empty by enforcement.
7. **The dispatcher's header routing prose becomes a pointer** to `rederive inventory`. It is the sixth home
   of the same fact, and it was already wrong once in the same commit that changed the answer.

**Not in PR-1**: any behaviour change to any block, and any removal.

⚠ **PR-1 falsifies A-i §8**, the program's declared single home for the harness's layout (part count,
per-part sizes, total, block count, `_measure` call-site census). §8 is on the same branch and is amended in
the same PR — that is the point of landing where the memos are, not a side effect.

## §4 PR-1 needs the memos, and that decides #505

I4 says the `declared by` column must stop parsing prose. The declaration it replaces prose with is a line in
each memo's §15 that a parser can read exactly — for example a fenced `rederive-blocks:` line — and
`inventory` must **fail** when a memo has a §15 and no such line, in place of today's heuristic.

That is an edit to six memos. All six are on `webref-cite-audit-tool` (#501). **PR-1 therefore cannot be done
on `citation-hygiene-harness` at all**, and the harness has to be where the memos are.

⇒ **Close #505; cherry-pick its harness commits onto `webref-cite-audit-tool`.**

This is the same destination draft 3 chose, on a sound argument instead of a false one. ⚠ Draft 3's stated
ground — *"kernel-only ships two checks with nothing to range over"* — is **false**, and was R1's own
adjudicated `empty-roster` premise re-used with the sign flipped. Measured in a sandbox: a kernel-only
harness has a roster of **2** (`selfcheck inventory`), and `selfcheck` prints
`3 harness parts, 12 blocks, 2 on 'all's roster / VERDICT: GREEN`. That premise decides nothing here, in
either direction.

⚠ **The recovery pointer is now a pushed ref.** `citation-hygiene-harness` was unpushed when R3 ran, so the
slots draft 3 wrote named a SHA reachable from one worktree's reflog — a defect the program had already paid
for once (an umbrella revision pointed at a commit a rebase destroyed). The branch is pushed;
`git branch -r --contains 372d6f52` resolves. **Do not delete the branch when the PR closes** — the
`cleanup-branch` post-hook must be skipped here, and that is an explicit instruction, not an assumption.

**Ordering.** Closing #505 unblocks #501 immediately (no merge, no rebase — the harness is byte-identical
there but for the `inventory` commits). #501 then lands on its own merits, and PR-1 stacks after it. PR-1 is
not a prerequisite for A-i's correctness: `couplings`, A-i's exit criterion, is green on that branch today.

## §5 PR-2 — the four behaviour fixes, and the promises they discharge

PR-2 lands after PR-1, on blocks that PR-1 has placed in their final files.

| fix | why it is PR-2's and not deferred |
|---|---|
| **`citations`** compares the authoritative §-title against the fixture's, per §0.5, treating a failed lookup as a failed measurement | `/elidex-review`'s CRIT-1 on #505; the block ships with A-i, so removal never discharges it |
| **`lanes`** routes its remaining seven bypasses through `_measure` | 3 of them are **unguarded**: a failure yields an empty loop, `failed` stays 0, the block returns 0. `lanes` ships with the umbrella, i.e. in the first harness PR |
| **`column` / `carvecolumn` / `remedies`** read the child's *stdout* instead of `[ "$rc" -le 1 ]` | **Codex R3-F2 on #501, answered publicly with *"They are being fixed in #505, not here"***. #505 closing must not turn a public commitment into slot content |
| **`ruleset`** asserts `conditions.ref_name` selects `main` | **Codex R3-F3**, same public commitment |

⚠ These are the two obligations R3's Axis 5 found unowned. Draft 3 routed them into branchless slices via
removal; PR-2 discharges them instead. `armmatrix`'s unbound row status (27 rows print `EXIT=`, block exits 0)
is the same class and rides with them.

## §6 PR-3 — whether the slice parts ship here at all

**Deliberately undecided by this memo.** After PR-1, `-Aii.sh` / `-Aiii.sh` / `-B.sh` are exactly their
slices' blocks and nothing else, so removing them is a `git rm` with no reconciliation. That makes it a cheap,
reversible decision that PR-3's own review can take on the evidence then — including M1's standing fact that
**B and C cite the harness zero times**.

This memo therefore creates **no defer slot and defers nothing of its own**: nothing is discarded, and the
question PR-3 answers is not blocked on anything but PR-1. Draft 3 created three slots for content it was
about to delete; the collapse removes the need for both.

## §7 Registers, swept rather than recalled

Every site asserting *#505 is open* / *#501 is blocked on #505* / *the harness is carved*:

```bash
cd /Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn -e '#505' -e 'blocked on' -e 'carve' -e 'CARVED' *.md
gh pr view 501 --json comments --jq '.comments[]|select(.body|test("505"))|.createdAt'
git grep -n -i 'harness' webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-umbrella.md
```

| register | class | what changes |
|---|---|---|
| `MEMORY.md` L3 bullet | invalidated | #505 open / #501 blocked / `git merge origin/main` re-join |
| `active-lane-detail.md:97-98` | invalidated | the 2026-08-02 carve note |
| `project_citation-hygiene-program.md` — **three regions**, not one: `:49-` (next-session pointer), `:146-204` (superseded decision + carve debt), `:206-239` (the carve section, whose `:238` prescribes the retired `git merge`) | invalidated | |
| **PR #501 comment, 2026-08-02** — *"They are being fixed in #505, not here"* | invalidated, and it is a **commitment**, not a status line | discharged by PR-2 (§5), and the comment says so |
| **umbrella `:117-133`** (self-labelled *"⚠ This bullet is a status register"*) | **restored** | it records the harness split as DISCHARGED by A-i naming three SHAs; those SHAs are on `webref-cite-audit-tool`, so the harness returning there makes the bullet true again and retires the owed "add a #505 row" amendment |
| **A-i `:320` / `:353` / `:664`** | **restored / discharged** | *"The harness split is A-i's, and it is done"*; `:664`'s owed `suites` relocation is discharged by PR-1 step 3 |

⚠ Two classes, and draft 3 had only one. Some registers need an amendment **retracted**, not written.

## §8 Claims vs checks

| claim | check | status |
|---|---|---|
| part source order is not load-bearing | D1 — sandbox, loop reversed → `selfcheck` GREEN rc=0 | CHECKED |
| no memo cites `say` or `fixtures` | D2 → no hits; M7 `declared by` = `-` | CHECKED |
| 10 blocks / 640 lines are misrouted; the list is M7's | `rederive inventory` routing-vs-shipping report | CHECKED |
| deleting parts with the roster untouched breaks `selfcheck` | sandbox → 15 false accusations, `inventory` cannot source | CHECKED |
| draft 3's kernel-only premise is false | sandbox → roster of 2, `selfcheck` GREEN | CHECKED |
| `372d6f52` is reachable from a remote ref | `git branch -r --contains 372d6f52` → `origin/citation-hygiene-harness` | CHECKED |
| the six memos are all on `webref-cite-audit-tool` | `ls ../elidex-wt-citeaudit/docs/plans/2026-07-citation-hygiene-*.md` → 6 | CHECKED |
| #501's harness = this branch's minus the `inventory` commits | `git diff --stat webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'` | CHECKED |
| the four `citations` pairs, 3 of 4 resolvable | `.claude/tools/webref heading --exact …` ×4 | CHECKED |
| the register list is complete | §7's three greps | CHECKED — **re-run at execution**, not at authoring |
| PR-1 leaves `inventory`'s misrouted list empty | — | **UNCHECKED** — it is PR-1's own exit criterion, and cannot be measured before PR-1 |
| the derived roster equals today's 23-name list | D3 (below) — `comm -23` and `comm -13` both empty, sets identical | **CHECKED** — and ⚠ this row said UNCHECKED with *"the intended answer is no: `readers` joins it"* until it was run. That prediction contradicted §3 step 5, which registers `readers` as unattendable, so it does **not** join. Reasoning about a set is not measuring it — in this memo of all places |
| A-i §8 after PR-1 | — | **UNCHECKED** — amended in PR-1 (§3), re-derived from `rederive inventory` rather than edited |

## §9 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: closing #505 with its branch **retained**;
cherry-picking the harness commits onto `webref-cite-audit-tool`; and **PR-1's scope as stated in §3**, which
is a structural change and gets its own plan-review before implementation under this umbrella's base case.

**Does not authorise**: any behaviour change to any block (that is PR-2, §5); any removal (that is PR-3, §6);
deleting the `citation-hygiene-harness` branch; editing a status register before the sweep in §7 is re-run;
or creating a defer slot — this memo has nothing to defer.
