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

## ⚠ Draft 5, and what a reader of draft 4 must un-learn

Draft 4's §3–§9 planned a collapse in which **a block's ship-with is the file it lives in, by construction**.
R4 falsified that premise, and draft 4 recorded the falsification in a bolt-on `§3.0` while leaving §3–§9
standing against the design it had just withdrawn. Draft 5 does not patch that step list — patching a
superseded plan is the edifice-building this program exists to stop. §2 through §9 are **re-derived** against
the design that replaced it, and `§3.0` is gone because it is now the body.

**The replacement design is landed, not proposed.** `90e1429b` implements it: `# ships-with: <group>` written
**inside a block's body** is the authority, the four tiers compute a second answer, and a disagreement is
named with the tier that produced it and raises `SystemExit`. It landed with **zero declarations** (34
undeclared, reported as such) because attaching them to blocks is PR-1's decision. Two defects in that
mechanism were found by planting declarations and **not** by inspection — the search region read a
neighbour's declaration, and a `$`-anchored pattern silently dropped a declaration with a trailing comment.
Both are fixed there. Every design claim below that could be run was run before it was written; the ones that
could not are marked UNCHECKED in §8.

## §0.5 / §3. Spec coverage map

PR-2 authorises the `citations` comparison, which is a comparison of spec §-titles, so this memo carries the
pairs. **All four resolve in webref** (`heading --exact`, rc=0 each); three of four are additionally mapped by
`preflight.py`'s pinned label table. Those are different resolvers and the distinction is load-bearing.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | §-title compare | fixture `labelled` / `dedup` / `malformed` | `citations` | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | §-title compare | fixture `labelled` | `citations` | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | §-title compare | fixture `alias` | `citations` | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | §-title compare | fixture `allunmapped` / `malformed` | `citations` | ✓ | no |

**Breadth**: K=3 specs, M=4 entries — the complete set, and the command finds the call site rather than this
table naming a line number PR-1 is about to move:

```bash
grep -n 'webref heading --exact' docs/plans/2026-07-citation-hygiene-A-rederive-*.sh   # → 4
```

⚠ **`CSSOM View 1` resolves in webref and is absent from `preflight.py`'s pinned map** — two different
things, and draft 1 of this memo conflated them. `webref heading --exact cssom-view-1 4.2` returns
`§4.2 The MediaQueryList Interface`, rc=0. What has no key is `preflight.SPEC_LABEL_REVERSE`, which is why
the `citations` block picks `CSSOM VIEW` for the `allunmapped` fixture — *"absent from the 24-key pinned map,
so this is all-unmapped AFTER A"*. ⚠ That map measures **15** keys on this branch —
`(cd .claude/skills/elidex-plan-review && python3 -c "import preflight; print(len(preflight.SPEC_LABEL_REVERSE))")`
— and A-i is the slice that moves it, so re-run this on whichever tree PR-2 lands in. The quote is the
harness's; the count is not re-derived by it.
**Correcting the harness's own comment is PR-2's, listed in §5** — this memo may not silently leave a
measured falsehood in the artifact it is planning.

⇒ **PR-2's comparison covers all four pairs.** The conflated version said it *"must compare only the pairs
whose lookup succeeded"* — which would have excluded exactly the row the comparison exists for: the block
records that this fixture's §-title was corrected **from a fabrication**, and that `verify_citation` checks
only that the number exists, *"so nothing would catch it"*. The real design constraint is the one that was
underneath: **a lookup that fails is a failed measurement** (`_measure`'s rule) and must never be reported as
a matching title — a statement about failure handling, not about which rows participate.

## §1 Measurements

The analysis note's M1–M7 are the shared basis; re-run them there rather than restating. Readings below were
taken at **`90e1429b`**, memos at **`2497eb09`**. **Every one is a command. Re-run before citing.**

```bash
# D1 — is part SOURCE ORDER load-bearing? (a glob sorts alphabetically)
for p in integrity common Ai Aii Aiii B; do printf '%-10s ' "$p"
  grep -cE '^[A-Za-z_][A-Za-z0-9_]*=' docs/plans/2026-07-citation-hygiene-A-rederive-$p.sh; done
#      then, in a sandbox, reverse the dispatcher's loop and run `… selfcheck`
# D2 — do any MEMOS cite the two helpers the collapse renames? (`--include='*.md'`
#      is load-bearing: docs/plans/ also holds the harness, and an unscoped sweep
#      returns the dispatcher's own header comment as if a memo had cited it.)
grep -rnE '`(say|fixtures)`|rederive (say|fixtures)' --include='*.md' \
     ../elidex-wt-citeaudit/docs/plans/
# --- D4..D7 run in a sandbox. `git clone --local` clones HEAD, so an UNCOMMITTED
#     mechanism reads as "rc=0, no difference" when it is simply not there; copy
#     the working files in with `\cp -f` (bare `cp` prompts and hangs).
SB=$(mktemp -d)/sb; git clone --local -q "$PWD" "$SB" -b citation-hygiene-harness
INV="bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory \
     $PWD/../elidex-wt-citeaudit/docs/plans"
# D4 — does a PURE RENAME move the computed answer? Apply steps: rename
#      -integrity.sh -> -kernel.sh and -common.sh -> -umbrella.sh, point the
#      dispatcher's source loop at the glob, derive PARTS from it, and ADD the two
#      new stems to PART_SLICE. Move no block. Then diff the `ships`/`why` columns
#      and the ROUTING-UNIT tally against the same two on the untouched clone.
# D5 — does deriving `all`'s roster in place break the parsers that read its
#      literal? Replace the `all() { set -- <23 names>` literal with a `declare -F`
#      pipeline and run `… selfcheck` and `$INV`.
# D6 — is the `AUTHOR_LOCAL` read guarded? Do D4's rename WITHOUT patching the
#      `-common.sh` filename at `-integrity.sh`'s AUTHOR_LOCAL read, and run $INV.
# D7 — how many blocks have a computed signal INDEPENDENT of their filename?
#      Delete the two-line `elif rows[b]["part"] in PART_SLICE:` branch (T2) from
#      `-integrity.sh`, run $INV, and take the blocks whose `why` becomes
#      `T3 no caller`; cross them with `on the roster, declared by no memo`.
# D8 — does draft 4's §7 grep support its §8 "the register list is complete"?
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn -e '#505' -e 'blocked on' -e 'carve' -e 'CARVED' "$MEMORY"/*.md | wc -l   # → 490
grep -rl -e '#505' -e 'blocked on' -e 'carve' -e 'CARVED' "$MEMORY"/*.md | wc -l   # → 108
grep -rn '#505' "$MEMORY"/*.md | wc -l                                            # →  19
```

- **D1** — source-time statements: `integrity` **2**, `common` **4**, every slice part **0**, and none is read
  by another part at source time. Measured in a sandbox with the loop reversed: `selfcheck` GREEN, rc=0;
  `inventory` produces the same table. **Order is not load-bearing, so a sorted glob is a sound replacement.**
- **D2** — **no hits in any memo**, confirming M7's `declared by` = `-` for both: the `_`-prefix rename
  changes no citation surface. ⚠ The first form of this command was scoped to `docs/plans/` and returned two
  hits, both the **harness's own** comments. A grep whose scope is wider than its claim reads as a citation
  that does not exist; the scope is now in the command.
- **D3** — the roster derived from `declare -F` (minus `_`-prefixed, minus `all`, minus the author-local
  registrations) is **set-identical** to today's 23-name literal: `comm -23` and `comm -13` both empty. The
  exclusion list in that proof **is** step 4's rule written out, so a different list in the implementation
  means the rule changed and the proof must be re-run. ⚠ **Set-identity is not implementability — see D5.**
- **D4 — a pure rename moves the computed ship-with for seven blocks.** In a sandbox, applying draft 4's
  steps 1+2 *and nothing else* (`-integrity.sh` → `-kernel.sh`, `-common.sh` → `-umbrella.sh`, part list ←
  glob), then extending `PART_SLICE` so the new stems are known: the routing/shipping disagreement falls from
  **10 blocks / 640 lines to 3 blocks / 194 lines with no block moved**, and `_measure` `_measured` go
  kernel, `_wtscan` `fixtures` `say` go umbrella, all by **T2 part**. Renaming the shared files into
  group-named files makes them *slice-like*, so T2 fires for everything in them and **shadows T3 entirely**.
  The metric improves because the measurement changed, not because the layout did.
- **D5 — replacing `all`'s roster literal with an expression breaks both parsers, loudly and wrongly.** Two
  regexes read that literal (`inventory`'s and `selfcheck`'s, same pattern, two sites). With the roster
  derived in place, `selfcheck` accuses `"all|$(printf`, `$(declare`, `$AUTHOR_LOCAL` and `'%s|'` of being
  *"dispatched by `all` but defined nowhere"*, and `inventory`'s roster column reads `no` for every block.
  **The derivation must therefore be a function every reader calls, not an expression they all re-parse.**
- **D6 — the `AUTHOR_LOCAL` read is unguarded and hardcodes a filename.** It reads
  `…-A-rederive-common.sh` by name and takes `.group(1)` with no `if`. After step 2 renames that file,
  `inventory` dies on an uncaught `FileNotFoundError` traceback — not the block's own `SystemExit`
  diagnostic, which every other input to the same block has. This is the same shape as `AUTHOR_LOCAL` itself:
  a fact about blocks, kept in one remote place, keyed by a string.
- **D7 — six of thirty-four blocks have no signal independent of the filename.** Dropping the T2 branch and
  re-running: `_runner` keeps its answer through T3 (four real callers), but `anchors` `bmemo` `offline`
  `partition` `staleclaims` `timing` fall to *"T3 no caller"* and land in kernel. No memo declares them
  either (`inventory` prints `on the roster, declared by no memo` and they are on it). **For those six the
  declaration is unfalsifiable** — the condition under which the analysis note's §2 rejected a declaration
  for `kind`. §2 below carries the consequence.
- **D8 — §7's sweep does not support §8's completeness claim.** The grep in draft 4's §7 returns **490** hits
  over **108** files (`grep -rn -e '#505' -e 'blocked on' -e 'carve' -e 'CARVED' "$MEMORY"/*.md | wc -l`;
  same with `-rl` for the file tally); §7's table named six registers. Narrowed to `#505` it returns **19**
  hits over **5** (`grep -rn '#505' "$MEMORY"/*.md | wc -l`; `-rl` for the tally) — `MEMORY.md`,
  `active-lane-detail.md`, `project_citation-hygiene-program.md`, and two feedback memos that cite #505 as a
  *provenance* rather than a status. A completeness row checked against a grep 26× wider than its own table
  is not checked.

## §2 Coupled invariants

Required because the work is edge-dense (`/elidex-plan-review` Pre-condition #3). ⚠ **Draft 4's I2 —
*"a block's ship-with is the file it lives in, by construction"* — is withdrawn, and it was the premise the
whole step list rested on.** R4 measured two ways it cannot hold, and D4 adds a third: T1 routes by declaring
memo and the memos are prose on another branch; T2 is `ship = PART_SLICE[part]`, which is also the misroute
predicate, so a block moved into the wrong file is silently re-attributed; and a rename with no content
change moves seven blocks' computed answer. **Construction cannot be the authority when the construction is
what is under review.**

- **I1 single home** — one fact (which blocks exist, and whose each is) is written down once.
- **I2 declared, then checked** — a block's group is **stated in its body**; the tiers compute a second
  answer and disagreement is a finding. The declaration is the authority precisely because it is the one
  input a rename, a memo edit and a file split all leave alone.
- **I3 removal safety** — deleting a part removes its blocks from every derived set automatically.
- **I4 derived input** — `inventory`'s own inputs are derived or declared, never parsed out of prose.
- **I5 fixed invocation surface** — every block name keeps resolving through the dispatcher path regardless
  of which part defines it. ⚠ Draft 4 wrote *"six memos cite blocks"*, which the dispatcher's own header also
  says at its `:18`. M1 measures **four** (A-i 27, A-ii 19, A-iii 10, umbrella 4; **B 0, C 0**). The stale
  header is one of the homes §3 collapses, and this memo inherited its digit — the exact transfer this
  program is about.

| pair | intersection | PR |
|---|---|---|
| I1 × I2 | The declaration is the single home only if nothing else can *decide* the answer. So the tiers must be demoted in the same change that introduces declarations — landed at `90e1429b`, ahead of this memo, because the design's viability was the open question (§9). | PR-1 |
| I2 × I3 | A declaration must be **independent of the filename**, or deleting/renaming a file changes what the harness believes about the blocks that were in it. D4 is that failure, measured. | PR-1 |
| I2 × I4 | For six blocks (D7) the only computed answer *is* the filename, so their declaration is unfalsifiable. The harness must therefore **report the strength of each agreement**, not just its existence: a green backed by T1 or T3 is evidence; a green backed by T2 is a restatement. The count of tautologically-confirmed blocks is a number the harness prints, not one a memo asserts. | PR-1 |
| I1 × I3 | The roster must be **derived from the definitions**, not listed. Measured: deleting two parts with the roster untouched makes `selfcheck` accuse **12 blocks that no longer exist**. | PR-1 |
| I1 × I5 | Excluding helpers by `_`-prefix requires renaming `say`/`fixtures`; D2 shows no memo cites either, so I5 survives. | PR-1 |
| I2 × I5 | A part **rename** must not change a block name. The dispatcher resolves by name across all sourced parts (D1), so renaming files is invisible to every memo — *and*, once I2 holds, invisible to the routing answer too, which D4 shows it is not today. | PR-1 |
| I4 × I5 | The four tiers still read the memos (T1), so `inventory` still needs a checkout that has them and still `SystemExit`s without one. That is **pre-existing** — M3 lists `inventory(exit 1)` among seven REDs of one class — and §4 turns on it. | PR-1 / §4 |

⚠ **Seven of seven intersections fall to PR-1, and that is a real concentration, not a bookkeeping artefact.**
Draft 4 left it unremarked. The reason it is nonetheless one PR is CLAUDE.md's base case: PR-1 is a
narrowly-scoped slice under an umbrella that declares the trigger, and every intersection above is one
predicate — *where does the harness keep the fact of which blocks exist and whose each is*. Splitting it
would put the declaration in one PR and the checks that make it falsifiable in another, which is the
strangler state `one-issue-one-way` forbids. PR-2 and PR-3 carry none because they act on blocks *after* the
fact is single-homed; that is what makes the order forced rather than preferred.

## §3 PR-1 — the collapse

**Goal**: after PR-1, the block set has exactly one home, and file-granular action on the harness is sound.

1. **Every block declares its group.** `# ships-with: <group>` in each block's body; groups are `kernel`,
   `umbrella`, `A-i`, `A-ii`, `A-iii`, `B`. 34 declarations, one per block, each reviewable on its own line.
   The mechanism is landed and green with zero declarations, so this step is the content and not the plumbing.
2. **The tiers report the strength of each agreement** (I2 × I4). A block confirmed only by T2 is confirmed
   by its own filename; `inventory` must print that set and its size rather than let it read as agreement.
   ⚠ D7 measures the set today: **`anchors` `bmemo` `offline` `partition` `staleclaims` `timing`**, six of
   thirty-four. It is not a defect to be fixed in PR-1 — it is a fact about six uncited, uncalled blocks that
   PR-3 is the right place to act on — but it must be *visible*, because "the tiers agree" is what makes the
   declaration checkable at all.
3. **The blocks move so that file = declared group.** The move list is `declared ≠ file`, printed by
   `inventory`, and **must be read after step 1 rather than transcribed from draft 4**: D4 shows the current
   10-block / 640-line list is partly an artefact of `PART_SLICE` having no key for the shared files. This
   discharges A-i §13's owed `suites` relocation (§7).
4. **`PART_SLICE` becomes derived, or goes.** It is a **seventh** home of the block-set fact — draft 4 said
   four to six and did not count it — and D4 shows a step list that renames the parts without touching it
   leaves the harness reporting *"lives in umbrella … ships with umbrella"* as a disagreement. Once every
   file is a group, the map from file to group is the identity and the dictionary is dead weight.
5. **Roster ← a function, not an expression.** `_roster` returns every function defined in a sourced part
   that is not `_`-prefixed, not registered author-local, and not `all` itself; `all`, `inventory` and
   `selfcheck` **call it**. ⚠ D5 measured why this cannot be an inline expression: two regexes read the
   literal today, and with a derivation in its place they parse the derivation's own shell tokens as block
   names. `say` → `_say`, `fixtures` → `_fixtures` (D2: no citation surface). The 23-name literal goes.
6. **Author-local registration adjacent to the definition.** `AUTHOR_LOCAL="lanes staleclaims"` — a remote
   list in one file naming a block defined in another — becomes a registration beside each definition,
   carrying its reason, and readable by sourcing rather than by regexing a filename (D6). `readers` joins it,
   closing the defect A-i §15 records. ⚠ Because `declare -f` strips comments, this registration must be a
   **shell statement**, not a `#` declaration like step 1's — the two facts have different readers and only
   one of them is bash.
7. **One glob, not two.** `selfcheck` globs `…A-rederive*.sh` (**7** matches — six parts and the dispatcher)
   while a part list globbed `…A-rederive-*.sh` yields **6**. Draft 4's step 1 would have left the two
   pointing at different sets while M6's *"7 harness parts"* silently changed meaning. Pick one and state
   which, in the block that prints the number.
8. **The dispatcher's header routing prose becomes a pointer** to `rederive inventory`. It is the sixth home
   of the same fact, it is already wrong at its `:18` (I5), and it carries the *"cited by more than one memo
   → `-common.sh`"* rule that step 3 retires — see §7, where that rule's second home is A-i §13.

**What PR-1 does not change: any block's subject.** ⚠ Draft 4's §9 said *"no behaviour change to any block"*,
which forbade its own steps — `all`, `inventory` and `selfcheck` are blocks, and steps 1–7 change all three.
The boundary that actually holds is narrower and states the same intent: **`all`, `inventory` and `selfcheck`
are exempt by definition, because their subject *is* the block set, which is what PR-1 collapses.** Every
other block's inputs, output and verdict are byte-identical across PR-1.

⚠ **PR-1 falsifies A-i §8**, the program's declared single home for the harness's layout. ⚠ **§8 is already
false at HEAD** and PR-1 does not create that: §8 states `-integrity` **238**, **1711** total, **33** blocks;
measured now, **545**, **2018**, **34** — the `inventory` commits falsified it, inside the program whose
theme is that. §8 is A-i's to re-derive at its landing, from `rederive inventory` rather than by editing.

## §4 Where PR-1 happens, and what that decides for #505

⚠ **Draft 4's answer was right and its reason was wrong, and the reason is what a reviewer checks.** It said
PR-1 must edit six memos to make the `declared by` column machine-readable, so PR-1 could not be done on
`citation-hygiene-harness` at all. **The declaration removes that constraint**: the authority is a comment in
the `.sh` file, on this branch, and the memos are demoted to one cross-check tier. The memo edits are no
longer PR-1's prerequisite.

**The constraint that survives is the analysis note's, already verified there**: *"A harness cannot be
stacked before the slice it measures."* M3 measures seven RED blocks on this branch, and M4 shows none is a
measurement failure — each correctly reports that what it measures is absent. `couplings`, A-i's own §12(3)
exit criterion and one of the two blocks carrying an invariant at all, is RED until A-i lands. `inventory` is
RED for the same reason one level up: its T1 tier reads the memos, and they are in #501 (I4 × I5).

⇒ **Close #505; carry its content onto `webref-cite-audit-tool`.** Same destination as draft 3 and draft 4,
on a ground that does not depend on PR-1's step list.

⚠ **"Cherry-pick its harness commits" is not the operation.** The two branches replayed each other
path-restricted, so their commit sets differ while their harness content does not. Measured, the whole delta
is **two files** — `git diff --stat webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'` → `-integrity.sh`
(+313 net) and one line of the dispatcher (`inventory` joining the roster) — produced by **four** commits
(`372d6f52` `ff44f30c` `945dd03a` `90e1429b`). ⚠ **Plus this memo and the analysis note**, which exist only
on `citation-hygiene-harness`: draft 4 authorised carrying the *harness* commits and left its own two files
with no destination. The transfer is those four commits and the two memos.

⚠ **The recovery pointer is a pushed ref, and must stay one.** `git branch -r --contains 372d6f52` resolves
to `origin/citation-hygiene-harness`. **Do not delete the branch when the PR closes** — the `cleanup-branch`
post-hook must be skipped here. That is an explicit instruction, not an assumption; the program has already
paid once for a SHA pointer a rebase destroyed.

⚠ **Draft 3's stated ground was false and is not revived.** *"Kernel-only ships two checks with nothing to
range over"* was R1's own adjudicated `empty-roster` premise re-used with the sign flipped. Measured in a
sandbox: a kernel-only harness has a roster of **2** (`selfcheck inventory`) and `selfcheck` is GREEN. That
premise decides nothing here in either direction.

**Ordering.** Closing #505 unblocks #501 immediately — no merge and no rebase, since `webref-cite-audit-tool`
already carries the harness. #501 lands on its own merits; PR-1 stacks after it, where the memos its T1 tier
reads are present and `couplings` is green.

## §5 PR-2 — the behaviour fixes, and the promises they discharge

PR-2 lands after PR-1, on blocks PR-1 has placed in their final files.

| fix | why it is PR-2's and not deferred |
|---|---|
| **`citations`** compares the authoritative §-title against the fixture's, per §0.5, treating a failed lookup as a failed measurement | `/elidex-review`'s CRIT-1 on #505; the block ships with A-i, so removal never discharges it |
| **`armmatrix`** binds each row's status | 27 rows print `EXIT=` and the block exits 0; A-ii cites it 5×. ⚠ Draft 4 left this in a footnote with no row while its three siblings had rows — the same class, silently ranked lower |
| **`lanes`** routes its remaining seven bypasses through `_measure` | 3 are **unguarded**: a failure yields an empty loop, `failed` stays 0, the block returns 0. `lanes` ships with the umbrella, i.e. in the first harness PR |
| **`column` / `carvecolumn` / `remedies`** read the child's *stdout* instead of `[ "$rc" -le 1 ]` | **Codex R3-F2 on #501, answered publicly with *"They are being fixed in #505, not here"***. #505 closing must not turn a public commitment into slot content |
| **`ruleset`** asserts `conditions.ref_name` selects `main` | **Codex R3-F3**, same public commitment |
| **the `citations` block's own "24-key pinned map" comment** | measured **15** (§0.5). A memo that plans a citation-hygiene fix must not leave a measured falsehood in the block it is fixing |

⚠ The two Codex rows are the obligations R3's Axis 5 found unowned. Draft 3 routed them into branchless
slices via removal; PR-2 discharges them.

## §6 PR-3 — whether the slice parts ship here at all

**Deliberately undecided by this memo.** After PR-1, `-Aii.sh` / `-Aiii.sh` / `-B.sh` are exactly their
slices' blocks and nothing else, so removing them is a `git rm` with no reconciliation. That makes it a
cheap, reversible decision PR-3's own review can take on the evidence then — including M1's standing fact
that **B and C cite the harness zero times**, and §3 step 2's set of six blocks whose group nobody has ever
written down and nothing calls, four of which are Slice B's.

This memo therefore creates **no defer slot and defers nothing of its own**: nothing is discarded, and the
question PR-3 answers is blocked on nothing but PR-1. Draft 3 created three slots for content it was about
to delete.

## §7 Registers, swept by command rather than by anchor

⚠ **Draft 4's table carried line anchors into files that move under it.** Measured: the umbrella bullet is
`:118-137`, not `:117-133`; A-i's owed `suites` sentence is `:676-677`, not `:664`; and the program memo's
regions had drifted 60–100 lines. Two of the three targets are **memory files, appended every session**, so
an anchor into them is stale before the next reader arrives. The register list is therefore a **query**, and
the table says what changes, not where.

```bash
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn '#505' "$MEMORY"/*.md            # the register set; `| wc -l` → 19, `-rl | wc -l` → 5
gh pr view 501 --json comments --jq '.comments[]|select(.body|test("505"))|.createdAt'
git grep -n 'DISCHARGED by A-i' webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-umbrella.md
git grep -n 'Still owed' webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md
```

⚠ **Draft 4's broader grep (`-e 'blocked on' -e 'carve' -e 'CARVED'`) returns 490 hits across 108 files** and
was the evidence for §8's *"the register list is complete — CHECKED"*. It is not evidence for a six-row
table; the narrow query above is, and §8 now says so.

| register | class | what changes |
|---|---|---|
| `MEMORY.md`, L3 bullet | invalidated | #505 open / #501 blocked on it / the `git merge origin/main` re-join |
| `active-lane-detail.md`, the 2026-08-02 carve note | invalidated | pre-split figures and the carve's standing |
| `project_citation-hygiene-program.md` — the next-session pointer, the superseded-draft-3 section, and the carve section | invalidated | three regions, not one |
| **PR #501 comment, 2026-08-02** — *"They are being fixed in #505, not here"* | invalidated, and a **commitment** rather than a status line | discharged by PR-2 (§5), and the comment says so |
| **umbrella, the `DISCHARGED by A-i` bullet** (self-labelled *"⚠ This bullet is a status register"*) | **restored** | it records the harness split as discharged by A-i naming three SHAs; those SHAs are on `webref-cite-audit-tool`, so the harness returning there makes the bullet true again and retires the owed "add a #505 row" amendment |
| **A-i §8** (*"The harness split is A-i's, and it is done"*) and **§8's layout figures** | **restored / re-derived** | the first is true again once the harness is on A-i's branch; the second is already false at HEAD (§3) and is A-i's to re-derive at landing |
| **A-i §13's owed `suites` relocation** | **discharged, and its rule retired** | §13 owes the move *"to `-common.sh`"* and states the rule it follows — *"cited by more than one memo → `-common.sh`"*. PR-1 abolishes `-common.sh`, so this is not a change of destination but a **retirement of the rule**, whose other home is the dispatcher header (§3 step 8) |
| **A-i §15's `AUTHOR_LOCAL="lanes staleclaims"` quotation and its `readers` note** | invalidated | §3 steps 5–6 replace the remote list with adjacent registration and put `readers` on it |

⚠ Three classes, and draft 4 had two: some registers need an amendment **retracted**, and some need a **rule**
retired rather than a pointer repaired.

## §8 Claims vs checks

| claim | check | status |
|---|---|---|
| part source order is not load-bearing | D1 — sandbox, loop reversed → `selfcheck` GREEN rc=0 | CHECKED |
| no memo cites `say` or `fixtures` | D2 → no hits; M7 `declared by` = `-` | CHECKED |
| the derived roster is set-identical to today's 23-name list | D3 — `comm -23` and `comm -13` both empty | CHECKED |
| a pure rename moves seven blocks' computed group; the misroute count falls 10→3 with nothing moved | D4 — sandbox | CHECKED |
| deriving the roster in place breaks both parsers | D5 — sandbox; `selfcheck` names four shell tokens as missing blocks | CHECKED |
| the `AUTHOR_LOCAL` read is unguarded and filename-keyed | D6 — sandbox; uncaught `FileNotFoundError` | CHECKED |
| six blocks have no signal independent of their filename | D7 — sandbox, T2 branch removed | CHECKED |
| §7's narrow query is the register set; draft 4's wide one is not evidence | D8 — `grep -rn '#505'` vs draft 4's four-alternative grep, `wc -l` on both | CHECKED |
| the ship-with declaration mechanism works and its two planted defects are fixed | `90e1429b` — two plants, both `rc=1` with the block named and the tier shown; clean tree rc=0, 34 undeclared | CHECKED |
| deleting parts with the roster untouched breaks `selfcheck` | sandbox → 12 false accusations, `inventory` cannot source | CHECKED |
| draft 3's kernel-only premise is false | sandbox → roster of 2, `selfcheck` GREEN | CHECKED |
| `372d6f52` is reachable from a remote ref | `git branch -r --contains 372d6f52` | CHECKED |
| the transfer is four commits plus two memos, and the content delta is two files | `git diff --stat webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'`; `git log --oneline webref-cite-audit-tool..HEAD -- …-integrity.sh` | CHECKED |
| the four `citations` pairs resolve; the pinned map has 15 keys | `webref heading --exact …` ×4; `len(preflight.SPEC_LABEL_REVERSE)` | CHECKED |
| A-i §8's figures are already false at HEAD | §8 says 238 / 1711 / 33; `wc -l` and `rederive inventory` say 545 / 2018 / 34 | CHECKED |
| four memos cite the harness, not six | M1 | CHECKED |
| PR-1 leaves `inventory`'s misroute list empty | — | **UNCHECKED** — PR-1's own exit criterion. ⚠ D4 shows it is **unreachable** with `PART_SLICE` unchanged, which is why step 4 exists; the criterion is now reachable, not met |
| the six tautologically-confirmed blocks stay six after PR-1 | — | **UNCHECKED** — step 1's declarations do not change what computes them, but step 3's moves change T2's input. Read it from the block after PR-1, do not predict it |
| the register list is complete | §7's narrow query, re-run **at execution** | **UNCHECKED at authoring** — memory files are appended every session, so a completeness claim made here expires. ⚠ Draft 4 marked this CHECKED against a grep 26× wider than its table |

## §9 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: closing #505 with its branch **retained**; carrying the
four `inventory` commits and this memo pair onto `webref-cite-audit-tool`; and **PR-1's scope as stated in
§3**, which is a structural change and gets its own plan-review before implementation under this umbrella's
base case.

**Does not authorise**: changing any block's **subject** — its inputs, its output or its verdict — for any
block other than `all`, `inventory` and `selfcheck`, whose subject is the block set (that is PR-2, §5); any
removal (that is PR-3, §6); deleting the `citation-hygiene-harness` branch; editing a status register before
§7's query is re-run; or creating a defer slot — this memo has nothing to defer.

⚠ **One thing already happened ahead of this memo, and it is recorded rather than smuggled.** `90e1429b`
landed the declaration mechanism on this branch before any review authorised it, because R4's three
converging CRITs made the design's *viability* the open question, and the two defects that mattered — a
declaration read as its neighbour's, and a declaration silently dropped for having a trailing comment —
appeared only when declarations were planted. Neither was visible by inspection. That order is the program's
own lesson applied to itself; it is not a licence, and §3's remaining steps are not implemented.
