# Slice A-i-wire — the K2 generic-core layering trip-wire

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire**.
**Branch**: `webref-generic-core-trip-wire`, stacked on `webref-cite-audit-tool` (#501).
**PR**: #519. **Status**: implementation carried from #501 `611758ff`; **plan-review round 1
returned 2 CRIT / 25 IMP / 19 MIN**, whose disposition is this memo. The one fork the
disposition left open — where this instrument should live at all — was **decided on 2026-09-21
(§1.1, option (a))**, and this revision answers §5's eight items, §6's interpreter question and
§7's mutation-set criterion. **Nothing in §5, §6 or §7 is open.**

⚠ **This memo states no quantity that moves with a commit.** Where a number matters it
appears as the command that produces it. Historical quantities (review-round numbers, gate
tallies) are provenance and are cited to the lane SSoT
`memory/project_citation-hygiene-program.md`, which owns them; they are not re-derived here.
⚠ **Draft 1 broke that rule in its own §1**, with a file count that was false when it was
written and unsatisfiable forever after, and four review axes found it independently. The
rule is easy to state and was not easy to keep.

---

## §0 Why this slice exists at all

The wire is not new code. It entered #501 at **review round 68** (lane SSoT), after that
slice's own `/elidex-plan-review` had closed, and grew there until #501's fifth design
re-gate. Three review axes reached the same conclusion independently: it does not belong in
a slice whose subject is a spec-label map.

CLAUDE.md § "Design discipline" makes `/elidex-plan-review` a **rule, not a judgment** for
edge-dense work, and its base-case clause — *"承認済 umbrella 配下で plan-review を通った
narrowly-scoped per-PR slice は terminal 単位"* — covers what a plan-review approved. It does
not reach an instrument added afterwards. So nothing has ever reviewed this wire as a design.

⚠ **The lane SSoT considered this carve once and declined it**, and the record is the
starting point rather than a footnote (`project_citation-hygiene-program.md`; the declining
passage follows the **R74** trajectory line, the re-evaluation trigger is R73's — a previous
revision cited "the R73 block" for both, and that block contains neither). ⚠ **The phrases
below are TRANSLATIONS of a Japanese SSoT, not verbatim quotations** — a reader following the
pointer will find 「正準アルゴリズム不在」/「正準機構を手で再実装していた」/「不変条件はここ・
機構はあちら」/「次に wire で finding が出たら option B … を検討する」:
the edge-dense trigger's *"no canonical algorithm"* limb was **falsified by measurement**, so
the findings of that period were evidence of *"a canonical mechanism being re-implemented by
hand"*, not of inherent complexity; and the wire is K2's enforcement, i.e. A-i's own
invariant, so carving it reproduces the *"invariant here, mechanism there"* split that gate 4
raised. **The same block set the re-evaluation trigger** — *"next time the wire draws a
finding, consider option B"* — and it fired repeatedly from R74 on. ⚠ **A previous revision wrote "in every round from R74 to R97",
which the SSoT's own round headers falsify** — R82 records *「今回は 2 件とも memo（wire ではない）」*,
and R83/R84 are likewise about memo text, not the wire. The rounds in which the wire itself drew a
finding are derivable from the SSoT rather than asserted here. So this carve is
what the SSoT scheduled, not a reversal of it. **The second ground is answered in §1.1**: the
separation this carve makes is one stacked PR wide and lands immediately after #501, which is
the narrowest form the objection admits short of not carving at all.

Derive the proportion rather than reading a figure — and **derive the two halves separately**,
because they are not the same artifact:

```sh
# scanner vs controls, code vs comment — draft 1 said "1089 lines" and hid this split
for f in .claude/tools/webref-generic-core-trip-wire.sh \
         .claude/tools/webref-generic-core-trip-wire.controls.sh; do
  awk -v f="$f" '{if($0~/^[[:space:]]*#/)c++; else if($0!~/^[[:space:]]*$/)o++}
                 END{print f, "code", o, "comment", c}' "$f"; done
wc -l .claude/tools/*-trip-wire.sh                      # the registered corpus (driver's glob)
git ls-files -- .claude/tools/_webref .claude/tools/webref | wc -l   # distinct paths scanned
for w in .claude/tools/*-trip-wire.sh; do /usr/bin/time -p bash "$w" >/dev/null; done
# commits that TOUCHED the file (not review rounds; and they live on #501, not here):
git log --oneline origin/main..webref-cite-audit-tool -- \
  .claude/tools/webref-generic-core-trip-wire.sh | wc -l
```

⚠ **Run them before arguing about size.** Review round 1 did, and the result is not what
draft 1 implied: the scanner's own code is smaller than the largest existing wire, and the
disproportion — in code, in fixtures and in runtime — is in the **controls**, not the
scanner. Any argument about whether this instrument is too large has to start there.

⚠ **The asymmetry is a separate argument from the size**, and draft 1 conflated them by
ruling the size out of scope right after handing the reader the commands. Both are in scope:
#501's own thesis is enforced by nothing until slice A-iii wires the Python suites into CI,
and a slice carrying a permanent gate for someone else's invariant while its own goes
unenforced is what *"a slice may not carry another slice's concern"* forbids (umbrella,
"Constraints each slice inherits").

## §1 What this slice is, and is not

**Is**: one predicate, one instrument, one registration, and the two policy paragraphs that
registration needs — the artifacts §4 tables.

**Is not**: any change to the generic core, the spec-label map, or the suite.

```sh
git diff --name-only webref-cite-audit-tool...HEAD -- .claude/tools/_webref   # must be empty
```

⚠ **That predicate, not a file count, is the boundary test.** Draft 1 said "five files" and
made an exit criterion of it; the memo is itself a tracked file in the diff it counted, so
the criterion was false when written and could never pass. The artifact set is §4's table,
and §7 checks against that table — not against a number.

### §1.1 — where this instrument belongs: **decided, (a)**

Review round 1 surfaced a third option that neither #501 nor draft 1 had considered. It was
put to the lane owner and **(a) was chosen on 2026-09-21**: K2 stays a shell wire in
`REQUIRED_WIRES`, carried by this stacked PR. The table is kept as the record of what was
weighed, not as an open question.

| | where K2 is decided | §6's contention | cost |
|---|---|---|---|
| **(a) here** ← **CHOSEN** | `trip-wires` job, grep + git, no interpreter | must be answered — §6 answers it | K2 unenforced until this PR lands, which is one PR after #501 |
| (b) slice A-iii — a Python check in the `tools` job | A-iii already plans an **ungated `tools` job and an interpreter floor** (umbrella, A-iii row) | does not arise — that job is interpreter-bearing by design | K2 unenforced through A-ii *and* A-iii; the git-population semantics this wire learned over ~30 review rounds re-derived in another language |
| (c) withdraw the carve | back in #501 | inherited by #501 | the instrument returns to a slice that never plan-reviewed it |

**Why (a), and not (b) — the ground is "one issue, one way", read the other way round.** (b)
looked attractive because it makes §6's question *not arise*. But §6 does have a candidate
answer that costs no wire movement: **Python checks belong in A-iii's `tools` job and
`trip-wires` stays shell + git**. Under that answer (b) is not a dissolution of §6, it is the
*wrong side* of it — it would put a check needing no interpreter into the interpreter-bearing
job, and leave the always-run job's floor to be decided by whoever else lands there. (a) plus
§6's answer gives each job one rule; (b) gives the `tools` job two kinds of occupant and
leaves `trip-wires` undefended.

**And (b) pays the lane's second ground twice over.** The SSoT's objection when it first
declined this carve was that separating K2's invariant from its mechanism reproduces gate 4's
*"invariant here, mechanism there"*. (a) separates them by one stacked PR that lands
immediately after #501. (b) separates them by two further slices, in another language, with
nothing enforcing K2 in between.

⚠ **What (a) does not dissolve, and this memo therefore owes**: §6's question is now this
slice's to answer rather than to sidestep, and the answer's write path is in **another lane's
PR** (#510). §6 states what can and cannot be executed here.

⚠ **Everything below is written for (a).** No re-derivation is owed.

## §2 The invariant, and the invariants it intersects

**K2** — the generic core names no elidex file path, where *file path* means
`.claude/(skills|tools)/` plus **two further segments**, and *generic core* means
`.claude/tools/_webref/` plus the entry script `.claude/tools/webref`.

⚠ **K2 has a closed part and an open part.** The closed part is the path predicate above —
decidable, and this wire decides it. The open part is #501 §12(3)'s, quoted as written:
*"'no **other** host path is named here' is not something this wire decides"*. #501 §12(3)
owns that split and this memo does not restate it. ⚠ **An earlier revision of this line
widened that quotation to "no other host path *or host policy* is named here"** — the phrase
*"or host policy"* occurs on no ref (`git show webref-cite-audit-tool:docs/plans/
2026-07-citation-hygiene-Ai-spec-label-map.md | grep -c "host policy"` → **0**), and the
widening is what made §5 item 1's "disposes of three" arithmetic look supported. The policy
clause is disposed of in §12(3) **nowhere**; see §5 item 1.

### §2.0 The authority, quoted in full — and K2 is STRICTER than it

⚠ **AND THE CLOSING RULE IS NOT THE STRONGEST TEXT AGAINST THIS WIRE.** The
external reviewer cited `DESIGN.md` **§Architecture** (`:24-47`), which does not
describe the package as a generic core with some adapter prose in it — it draws
an explicit two-column boundary and then **names the modules on each side**:

> - Generic core: upstream fetch/cache; semantic inventory construction; semantic
>   diff classification; stable JSON output schema.
> - **elidex adapter**: repository citation scanning; review/plan workflow wording;
>   **impacted `docs/` / `crates/` path heuristics**; elidex-specific agent briefs.

…and lists `commands/agent_brief.py` as the module that *"scans elidex paths for
citations affected by a semantic diff."*

**So `DESIGN.md`'s "generic core" and K2's "generic core" are different sets.**
`DESIGN.md`'s is five named modules; K2's (#501 §2) is `_webref/` **plus the entry
script** — which contains the adapter `DESIGN.md` deliberately put there. That is
not a reading anyone can reconcile by quoting harder, and this memo's earlier
attempt to do so is withdrawn twice over.

`_webref/DESIGN.md` also opens (`:3-5`, verbatim, both sentences):

> `webref` is maintained inside elidex for now, but **its drift-detection core** should stay
> generic enough to move to a standalone repository later. **elidex specific behavior belongs
> in thin adapter commands.**

and closes with: *"keep new generic behavior free of elidex-specific file paths and put elidex
policy in adapter commands or documentation."*

⚠ **A previous revision of this section quoted only the first half of the first sentence, with
its subject replaced** — "the package *should stay generic enough to move*" — and never quoted
the second sentence at all (`grep -rn "thin adapter\|drift-detection core"` over this memo and
both shell files returned **0** before this revision). That substitution is exactly what made
the conclusion "clause one is unqualified over the package, adapter commands included" appear
to follow from the authority. It does not follow, and the sentence it omitted says the
opposite: `DESIGN.md` deliberately sends elidex-specific behaviour **into** adapter commands,
and `_webref/commands/` and the `webref` entry script are adapter surfaces inside K2's scope.

⚠ **So review round 1's finding stands, and this memo's withdrawal of it is withdrawn.**
*"The predicate forbids something its own authority permits"* is true as far as `DESIGN.md`
goes. What resolves it is not a reading of `DESIGN.md` but the honest statement of what K2 is:

**K2 is a deliberate widening of `DESIGN.md`'s rule, chosen by this program, and the wire
enforces K2 — not `DESIGN.md`.** `DESIGN.md` scopes movability to *the drift-detection core*
and permits elidex *behaviour* in thin adapter commands. #501's §2 scopes K2 to
`_webref/` **plus the entry script**, and forbids one syntactic class — a
`.claude/(skills|tools)/<a>/<b>` string — **everywhere in that scope, adapter commands and the
package's own documentation included**. The ground for the widening is that such a string does
not move whoever writes it, so the distinction `DESIGN.md` draws between core and adapter does
not help a reader deciding whether the package can be lifted out. That is a program decision
with a cost, and it is stated as one rather than dressed as exegesis.

⚠ **The cost is real, is not hidden, and now has a name.** `commands/agent_brief.py` is the
module `DESIGN.md` designates as the elidex-path scanner, it is inside K2's scope, and if it
ever needs to *spell* a `.claude/(skills|tools)/<a>/<b>` path the required gate reds a
legitimate adapter. It does not today — its heuristics use bare `docs/` / `crates/` names,
which are class 3 — so the tree is green for a reason that is a fact about the current code,
not a guarantee.

⚠ **This is raised, not settled here**, for the same reason §6's answer is. Two resolutions
exist and both are somebody else's edit: narrow K2's scope to `DESIGN.md`'s named generic
modules (an amendment to **#501 §2**, whose exit criterion quotes the wider definition), or
amend `DESIGN.md` to say the path ban is package-wide (an edit to `_webref/DESIGN.md`, the one
file §1's boundary predicate genuinely covers). Until one lands, the wire enforces K2 as #501
defines it and says so in its header — the direction that can red a legitimate adapter but
cannot miss a violation.

⚠ **The by-DIRECTORY approximation cuts both ways.** Draft 1 stated only the evidentiary
direction (a green says nothing about `DESIGN.md` compliance) and added that the adapter-work
sites are *"exactly what this wire cannot see"* — a false biconditional on both counts: those
sites **are** read, they simply do not match the predicate; and the wire's declared blind
classes include two (interpolation, whitespace segment) that are not adapter-work sites.

### §2.1 Coupled invariants, and each pair's intersection

The plan-review flagged draft 1 for stating one invariant in prose where the slice satisfies
several at once. Enumerated:

| # | Invariant | Intersection that matters |
|---|---|---|
| I1 | K2 path predicate over the generic core | — |
| I2 | driver contract (`REQUIRED_WIRES` retention, root resolution, controls-absent ⇒ exit 2) | **I2 × I1**: the wire must be discoverable AND registered, and its absence must be loud |
| I3 | git population semantics (index / HEAD / worktree disagreeing; symlink blobs; NUL; a tracked path replaced by a FIFO; locale; routing env) | **I3 × I1**: the predicate must meet *stored* content, not what the filesystem happens to show |
| I4 | CI ungated-job policy (`ci.yml` + `CLAUDE.md`) | **I4 × I2**: the job runs on every PR, so its cost and its dependencies are repo-wide policy |
| I5 | the interpreter floor of the wire set (§6) | **I5 × I4**: what the always-run job may require |
| I6 | touch-time split discipline | **I6 × I2**: a `source`d fragment can skip its own controls, so the split must not weaken I2 |

⚠ I1 × I3 is where ~30 review rounds went, and I5 is the one this slice cannot settle alone.

## §0.5 / §3. Spec coverage map

**No spec surface** — this slice touches no spec-defined behaviour: one shell predicate, its
controls, one line of registration and two policy paragraphs. Its authority is `DESIGN.md`'s
closing rule plus the trip-wire registration convention in `scripts/trip-wires.sh`.
⚠ **Draft 1 also named "CLAUDE.md's layering mandate", which is wrong** — that section carries
two rules, the VM-host one (`crates/script/elidex-js/src/vm/host/`) and the core-vs-compat
split, and **neither** says anything about the generic core; naming it routes a reader to the
wrong review axis.

⚠ **This adopts slice A-iii's shape, and with it A-iii's `preflight` exit.** Measured, both
ways, at this head:

```sh
python3 .claude/skills/elidex-plan-review/preflight.py \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md; echo "rc=$?"
```

* **Before**, with a placeholder row reading `| (none — §3's prose states why) | … |`:
  **rc=0**, `unmapped-label rows: 1`, `unique specs (K): 1 (<(none —>)`, breadth counted, and
  **no webref verification attempted** — the one path through the citation arm that neither
  verifies nor fails. That row was the shape this program's gate exists to prevent, sitting in
  the memo whose subject is a gate.
* **Now**, with no table: **rc=1**, `❌ HARD FAIL — Spec coverage map heading at line N but no
  markdown table follows it`. **That exit is by design.** The declaration is A-ii's §4.2.5
  feature and A-ii has not landed, so the honest states are "hard-fail with a declaration" or
  "parse green over an invented citation"; A-iii chose the first and recorded it in the
  umbrella's memo table, and this slice does not invent a second answer.
* (Also measured: deleting the heading as well is **rc=1** too, `no 'Spec coverage map'
  heading in plan-memo` — i.e. the heading is not what buys the green, so keeping it costs
  nothing and keeps the declaration where its consumer will look.)

⚠ The second advisory draft 1's §3 reported — an enumeration-claim warning on §2's
cross-document citation — goes with the table; there is no longer a citation arm to run.

## §4 Artifacts

| Artifact | What it is |
|---|---|
| `.claude/tools/webref-generic-core-trip-wire.sh` | the scanner: population from git, content from git, verdict |
| `.claude/tools/webref-generic-core-trip-wire.controls.sh` | the controls: one fixture per verdict the scanner can reach, **plus the mutation set** (§7 criterion 3) that shows each control is about the arm it names. Sourced by the wire; its interface is asserted at entry, and run on its own it exits 2 saying so |
| `scripts/trip-wires.sh` | one line in `REQUIRED_WIRES`, plus the two driver comments its arrival falsified |
| `.github/workflows/ci.yml` | the ungated-job rationale for the `trip-wires` job |
| `CLAUDE.md` | the paragraph restating that rationale, which names the job comment as canonical |
| this memo | the plan-review record — **a tracked file in this slice's diff**, which draft 1's boundary count forgot |
| `docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md` | #501's memo, **swept** where it duplicated or contradicted the decisions this slice made: §12(3)'s two restatements of the wire's blind classes and its "its header says so" pointer, and §12(4)'s booking of four defer slots on this PR (§5 item 1). ⚠ Added to this table by the review that found the contradiction; the artifact set is not frozen, and §7.5 says so |
| `docs/plans/2026-07-citation-hygiene-umbrella.md` | the A-i-wire **memo-table row** (without it, the umbrella's own repo-wide preflight command hard-fails on this memo with nothing to explain it), the retired "1089-line wire" figure, and §6's answer recorded where the umbrella books the CI-topology question |

⚠ **What the wire asserts, and why, is stated once — in the wire**, in the comment block
above `_esc`. This table names the files; it does not restate the mechanism.

## §5 What this review had to decide — **answered, all eight**

Review round 1 found that draft 1's agenda was partly mis-posed: some items rested on false
premises, and one had its answer already written into it. Each item below is restated as round
1 left it, then **answered**, and every answer says whether it **changes the implementation**,
so "answered, status quo" is distinguishable from "unanswered" (§7 criterion 4).

---

### 1. Do any of the four declared blind spots become defer slots? — **No. Implementation: changed (comments).**

**The question, as round 1 sharpened it**: draft 1 said the wire *"does neither"* (close nor
file). The disposition that replaced it said #501 §12(3) *"already disposes of three"*, and
**that count does not survive checking either** — §12(3)'s *"Open part, reviewed not gated"*
bullet is about **host paths**, not `DESIGN.md`'s policy clause, which §12 disposes of nowhere;
the count was propped up by a quotation this memo had widened (§2). What §12(3) actually did was
restate two classes (whitespace, bare top-level names) in its own prose while §12(4) booked all
four as **defer slots on this PR** — i.e. the base memo both duplicated the enumeration and
contradicted the answer below. The real question is **whether a permanent, required, ungated
gate may hold a review-only obligation at all**, or whether registering it in CI changes what
"filed" has to mean.

**Answer: the gate holds no obligation it does not assert.** What a blind spot records is the
**reach of a predicate**, not work someone owes later — and the two are different objects. A
defer slot says *"this will be done, here is the trigger and the date"*; a reach declaration
says *"this instrument will never decide that"*. Registering the wire in CI changes **who runs
it and how often**. It does not widen its subject, so it cannot convert a statement about reach
into an obligation.

**What registration *does* change** is the risk that a green gets read as a broader claim than
it is — and the answer to that is the wire's own words, not a slot: its header states the reach
and its failure text names the class. That is where this item changes the implementation.

⚠ **And for two of the four, a slot would be unfileable on its own terms.** Interpolation and
bare top-level names are properties of a grep over arbitrary source text; *"closable here"* is
answerable only as **no**. A slot needs a `Re-evaluation trigger` **and a `Re-evaluation
date`**, and the lane's own precedent records that an undated trigger *"has nothing that forces
a look"*. A trigger that can never fire is worse than undated.

**So the cap arithmetic dissolves**: own deferrals **0**, not 4. §8 is updated.

**The implementation change**: the four classes were stated at **four different sites** in the
wire — the policy half in the opening `DESIGN.md` paragraph, the whitespace segment in the
`$K2RE` comment, bare names and interpolation in the seed's obituary. That is the duplicated
decision surface this instrument spent R76 / R80 / R81 collapsing, one level down. They are now
**one block, `WHAT THIS WIRE DOES NOT DECIDE`**, which says in its first line that it is the
one place the list is stated and that anything added is added there. The other sites point at
it.

**The list is now five, not four.** Review found a class nobody had listed: a
`.claude/(skills|tools)/` path with **one** further segment, of which this package's own entry
script is the live case — **non-zero in four files inside the scanned scope**, the bulk of it
`cli.py`'s `--help` examples (the derivation is in the wire's block; the count moves with the
package, so it is not written here). Unlike classes
3 and 4 it **is** grep-decidable, so the block's own "not closable by any wire" justification
does not reach it, and the honest statement — made in the block — is that the predicate never
looked. It is listed, not closed: widening to one segment would red the package on every
mention of its own entry point.

**And the base memo is swept, not merely raised.** A previous revision declined this on the
ground that *"§1's boundary predicate forbids this diff from making"* the edit — **false**: that
predicate's pathspec is `-- .claude/tools/_webref`, which does not cover
`docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md`, an ordinary file on the branch this
PR is stacked on. Three sites are swept **in this PR**, because this is the PR that decided
them: §12(3)'s two restatements and its *"its header says so"* pointer (whose *"four failed
attempts"* are the deleted seed's four failed **widenings**, not these classes), and §12(4)'s
booking of four defer slots on this PR. Leaving them would land a memo asserting, in the same
tree, that slots exist which §8 refuses to file.

---

### 2. The threat-model paragraph — **restated. Implementation: changed (comment); no arm removed.**

Round 1's finding: *"The threat model is accident, not adversary — and saying so bounds this
file"* is falsified by the file's own growth after it was written, and the NUL arm it was
written to justify **is** a new mechanism; the *"existing fail-closed answer"* it appealed to
did not exist until that mechanism created it.

**Answer: delete the bounding claim, keep the decision rule, and keep every arm.** The three
offered options were restate / remove the arms the model excludes / delete the paragraph. The
middle one is wrong on the facts and is the one worth saying no to loudly: **the NUL arm is not
adversary-hardening.** `$( )` *drops* NUL bytes, so the blob arrived at the matcher as a clean
`.claude/skills//rule.md` and read GREEN. The hazard is a **silent misread**, and detecting it
and erroring is the same fail-closed direction as every other unknown in the file. Removing it
would restore a silent green.

What was false was only the paragraph's self-description. It now states one rule — *everything
git can hold and this reader cannot represent is an ERROR* — and says explicitly that this is
**not a bound on the file**, giving the falsified claim and the reason. The registration-edit
observation survives, in its true form: it explains why the wire does not trade reading
correctness for adversary-resistance it could not have anyway.

---

### 3. Two controls pass without testing what they name — **both fixed, one shape each, and the shapes are now two, not three. Implementation: changed.**

Round 1 corrected the count (two, not draft 1's three) and identified what was actually open:
the two already-guarded sites use **two different shapes** — a precondition that sets `ctl_ok=1`
(`notcommitted`, `replaced`) and a note that does not (`odd`) — and the `odd`/`fifotracked` pair
appeared to need a third.

**Answer: there are exactly two legitimate shapes, distinguished by *what failed*, and the
third is not needed.**

| the fixture could not be **built** | the **machine** cannot exercise it |
|---|---|
| `CONTROL NOT EXERCISED` on stderr, `ctl_ok=1`, run reds | a line in the run's own summary saying so, run stays green |
| `notcommitted`, `replaced`, **now `cachedir`** | `_perm_line` (mode-000 readable by this user), **now `_fifo_line`** |

**`cachedir`** is a build failure and takes the first shape. Its defect was ordering: the shared
`init` + `add -A` loop ran **before** `.gitignore` was written, so the probe was tracked already
and the force-add changed nothing. **Measured: deleting `_fgit add -f __pycache__/probe.txt`
entirely left the wire at `rc=0`.** The fixture is now built outside that loop — `.gitignore`,
then the ordinary add (which must now skip the probe), then the force-add as the only thing that
can track it — with a precondition asserting the probe is **ignored *and* tracked**, which is
the property under test. **Re-measured after the fix: the same deletion now gives `rc=1` with
`CONTROL NOT EXERCISED`.**
⚠ The precondition needs `git check-ignore --no-index`; without it git answers about the
**index**, so an ignored path reads *"not ignored"* the moment it is tracked — i.e. the flag is
off exactly in the state the control requires, and the first version of this precondition
rejected its own correct fixture. Recorded in the controls, because the next person to write a
tracked-and-ignored assertion will hit it too.

**`odd` and `fifotracked`** are one machine capability, so they get **one probe and one report
line**, built beside the decision that produces it. Previously each created its FIFO under
`|| true`, so on a filesystem without FIFOs `odd` printed *"every other control ran"* —
**false in exactly the runs where `fifotracked` had just failed for the same missing
capability** — and `fifotracked` reported a *wire* failure for a *machine* limitation. Now
`mkfifo` is asked once; if it fails, neither control runs and `$_fifo_line` says the run carries
no evidence for either. **Measured: forcing the probe to fail leaves the run green with the
NOT-EXERCISED line; killing the wire's `neither a regular file nor a symlink` arm still reds
`fifotracked`,** so gating it did not blunt it.

⚠ **The unconditional summary lost its FIFO clause**, which it had been printing whether or not
the pair ran — the same defect `_perm_line` was built to cure, at a second site.

---

### 4. The false CI rationale, at four sites — **landed at all four. Implementation: changed.**

Round 1 corrected draft 1 twice: it named one site (there are four) and claimed the item was
unfixed in the carried commit (the carried commit had already written one answer, at
`CLAUDE.md`).

**The property the decision rests on** is not "grep", and it is not a runtime figure. It is the
**absence of a setup step**: the wires run on what a bare checkout already has — the shell,
`git` and `grep` — with nothing to install, no cache and no network. The `trip-wires` job's own
comment had already reached that formulation (*"What the decision rests on is the ABSENCE OF A
SETUP STEP"*); the other three sites had not.

⚠ **AND THE FIRST REPLACEMENT WAS FALSE IN THE SAME SHAPE.** This item's first attempt wrote
*"the shell, `git` and `grep` a bare checkout already has"* at all four sites. Four reviewers
measured it independently: the wire and its controls also call `sed`, `tr`, `cmp`, `readlink`,
`mktemp`, `mkfifo`, `chmod`, `env`, `cut`, `ln`, `cp`, and the four **sibling** wires in the same
job add `awk`, `sort`, `comm`, `wc`. A narrower-than-true enumeration, written by the edit
retiring a narrower-than-true enumeration — which is what "a check defined by the symptom
vocabulary" costs: the residue command below *used to* grep for the retired **phrase**
(`grep-only`), so it could not see the new one.

**The property, settled**: the wires need **nothing installed** — no language runtime, no
package manager, no cache, no network. ⚠ **And that is stated as a property, not a list**,
because the list has now been wrong twice. What makes it checkable is the job's shape:

```sh
# the whole claim, derived: the trip-wires job has ONE `uses:`, the checkout.
sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml | grep -c 'uses:'   # -> 1
# and nothing in the wire set shells out to a language runtime. ⚠ The second
# filter is not decoration: without it the only hit is a COMMENT recording that
# an earlier revision used python3 — i.e. the bare grep cannot tell the history
# of the rule from a violation of it.
grep -rnE '\b(python3?|node|ruby|perl|cargo)\b' \
     .claude/tools/*-trip-wire*.sh scripts/trip-wires.sh \
  | grep -vE ':[0-9]+:[[:space:]]*#'          # -> no output
```

| site | now says |
|---|---|
| `ci.yml`, the ungated-job rationale block | the property, plus both retired enumerations named as retired |
| `ci.yml`, the step comment | *"No setup step — nothing to install"*, pointing at the block |
| `CLAUDE.md` | the same property, with 「道具の列挙で書かない」 and the `uses:` derivation |
| the wire's header — §4's canonical site | *"RUNTIME: NOTHING TO INSTALL"*, with both retired lists and the reason |

⚠ **Including `ci.yml`'s line, which this slice had touched the file of but not the line** —
the case draft 1 recorded as out of scope.

⚠ **One more thing the wire's header was doing**, found by the same pass: it closed with
*"anything needing more than the shell, git and grep belongs in a test, not here"*, sitting
beside a sentence about the **job**. Read there it is a rule for the wire **set** — i.e. an
answer to §6's open question, asserted in a comment. Scoped to this wire, with §6 named.

---

### 5. The controls are a `source`d fragment — **the seam stays; the interface is now asserted, not narrated. Implementation: changed.**

Round 1's finding: the controls file consumes the wire's variables and helpers, writes `ctl_ok`
back, and cannot be executed or tested standalone — run directly it reported a `TMPDIR` error
rather than *"must be sourced"*. It also ruled out draft 1's *"state the split as a line-count
response"*, since CLAUDE.md names that as the wrong basis (*"line-count の機械適用でなく
cohesion 判断"*).

**Answer: keep the split, and reject "a real entry point with explicit parameters" — on a
project rule, not on cost.** A separate program needs its own `_git` (the repository-routing
purge, whose exemption list is itself a reproduced finding), its own `LC_ALL` pin and its own
`GIT_NO_LAZY_FETCH` / `GIT_NO_REPLACE_OBJECTS` exports: **a second statement of how this gate
reads git**, in a file whose whole job is to prove the first statement reaches every verdict.
That is the decision-surface duplication this instrument collapsed at #501 R76, R80, R81 and
R89, and "one issue, one way" forbids re-creating it one level down. The seam itself is not in
question — *answers* versus *proof the answers are reachable* is a subject boundary, and it
predates the line count.

**What was actually wrong was the interface**, and round 1 is right that a narrated contract is
no contract. Three changes:

1. The header's stated interface **was wider than the truth** — it claimed `$ROOT` and `_phys`,
   neither of which this file mentions, and attributed `_fgit` to the wire, which does not
   define it. Corrected to what the file consumes (`$SELF`, `$SCRATCH`, the five `CONTROL_*`
   strings, `_git`) and what it defines.
2. That list is now **asserted at entry**, name by name, rather than described.
3. A direct invocation therefore says what this file is and what to run instead — **measured,
   `rc=2`** — instead of failing inside `mktemp` with a message about a read-only root.

⚠ The one property a separate process would have bought — running the controls without also
scanning the real tree — is not one this wire wants: its contract is that a green is earned by
both halves of the same run.

⚠ **And no second seam is taken.** Re-derive the sizes rather than reading them (§0's first
command): after this revision the scanner is in the 700-line band that CLAUDE.md's touch-time
discipline says to look at *while writing*. Looked at, and the answer is no: the split rule is a
**cohesion** judgement, not a line count, and what is left is one predicate, one walk, one
verdict — a monolithic cohesive unit, mostly rationale (§0's first command prints the
code/comment split; a previous revision put a fraction here, and it was both a quantity that
moves and wrong). The cut that existed was the one already taken. This is recorded rather than
left implicit, because "the file is in the band and nobody said why it stayed" is how the
discipline degrades into a count.

⚠ **And the same judgement is owed for the CONTROLS file, which a previous revision did not
give.** It carries two subjects — the fixture controls, and the mutation harness the wire gates
behind `WEBREF_WIRE_MUTANTS` — and that looks like a seam. It is not taken, for the reason the
harness exists: **the mutation set's whole claim is that each control is about the arm it
names**, so the two lists must be edited together and are checked against each other in the same
run (the correspondence check reds on a record naming no control). Splitting them puts the two
halves of one assertion in two files and makes the check cross-file for no gain. The seam that
would be real — fixtures vs. controls — runs through every entry rather than between two blocks.

---

### 6. Where the controls file lives — **pinned by an assertion that already existed, plus the one from item 5. Implementation: covered by item 5.**

Round 1's finding: it sits in the directory the driver globs to *discover* wires, mode 755,
with a name one token short of the discovery convention.

**Answer: the distinction is not left to the name, and it never was — it is decided in one
place.** `scripts/trip-wires.sh` diffs its `*-trip-wire.sh` glob against `REQUIRED_WIRES` **in
both directions**, and the second direction exists precisely for this: *"a wire that ARRIVES
must be registered here in the same commit"*. A rename that brought this file **into** the
convention would run it and fail twice over — once here, with item 5's contract message, and
once there, with `trip-wire(s) ran but are not registered`. Both are loud and neither depends on
a naming near-miss. Measured:

```sh
for w in .claude/tools/*-trip-wire.sh; do echo "$w"; done   # the driver's glob: 5, controls absent
bash .claude/tools/webref-generic-core-trip-wire.controls.sh; echo "rc=$?"   # 2, and says why
```

⚠ **Round 1's supporting claim does not survive measurement and is withdrawn**: *"this memo's
own §0 command and the driver's disagree"*. They do not — §0's `wc -l .claude/tools/*-trip-wire.sh`
**is** the driver's glob and returns the same five files. What §0 does is name the two artifacts
**explicitly** in its first command, because the proportionality question is about the artifact
pair while the driver's question is about the registered set. Two questions, two commands, both
correct; the near-miss name is what made them look like one question with two answers.

⚠ **Mode 755 is kept.** The file is invoked as `bash "$_CONTROLS"`, so the bit is not
load-bearing either way, and after item 5 it is no longer misleading: the file *is* a program
that reports what it is when run.

---

### 7. §3's placeholder row — **A-iii's shape adopted. Implementation: changed (memo).**

The row is gone and §3 is now a `**No spec surface**` declaration in A-iii's exact shape. The
measured before/after — `rc=0` with an unverified, breadth-counted row versus `rc=1` HARD FAIL
by design — is recorded **in §3**, where a reader who runs preflight and is surprised will look,
rather than here.

**Why the declaration and not "make the empty-map case explicit in the gate's vocabulary"**:
that second option is A-ii's §4.2.5, it is not landed, and extending the gate's vocabulary from
this slice would be a change to `.claude/skills/elidex-plan-review/`, which is outside §1's
boundary and is A-i's one-comment allowance in #501 §12(1). One question, one answer, in the
slice that owns it.

---

### 8. §1.1's fork — **decided: (a).** Implementation: unchanged (the carve as built). The reasoning is in §1.1; §6 carries the question (a) leaves open.

## §6 The interpreter floor — what is contended, and by whom

Open PR **#510** registers a wire requiring `python3` and raises the `trip-wires` job's
`timeout-minutes`. This wire's own header rests on the opposite premise, and `CLAUDE.md`
rests the ungated-job decision on it.

⚠ **Three claimants, not two.** `stale-claim-detector` also registers into `REQUIRED_WIRES`
(a wire that needs no interpreter), and #510's own `ci.yml` comment already books an
obligation against it. The *interpreter* half is #510's alone; the *budget* half — what the
always-run job may cost — has three.

⚠ **The conflict is textual as well as policy-level**: #510 rewrites the same `CLAUDE.md`
paragraph this slice rewrites, in the opposite direction, and adds a second CLAUDE.md site.

⚠ **And this slice's carried commit had already written one answer before the question was
called open** (quoting against **this branch's own predecessor commit**, not against the stacked
base — the PR's diff removes different text at two of the four sites, because the carve rewrote
them once already). It rewrote that paragraph to say the wire set needs *"no toolchain, no cache,
no network"* — the "no" answer, landed on a repo-wide surface. Draft 1 claimed the six items
were *"deliberately not fixed in the carried commit"*; for this one that was not true. §5
item 4 has since rewritten all four sites to the **property** rather than the shorthand, and
that rewrite is deliberately **silent on the interpreter question**: it says what the wires
*do* need (the shell, `git`, `grep`, nothing to install) and does not say what a future wire
may not need. The answer below is the memo's, not a paragraph's.

Derive the contending set rather than listing it — and note what the command can and cannot
say:

```sh
git fetch -q origin
for b in $(git for-each-ref --format='%(refname:short)' refs/remotes/origin); do
  n=$(git diff --name-only origin/main...$b -- \
        scripts/trip-wires.sh .github/workflows/ci.yml CLAUDE.md | tr '\n' ' ')
  [ -n "$n" ] && echo "$b :: $n"; done
```

⚠ **This is a seed, not an inventory.** It returns branches that touch any of three files for
any reason, includes already-merged branches (the diff is merge-base-relative), and — scoped
to `refs/remotes/origin` — cannot see branches that exist only in one clone. Draft 1's
version additionally collapsed each branch to its first path with `head -1`, which hid that
#510 touches all three.

### The decision this slice owes: may the required, ungated wire set require an interpreter?

**This memo's answer is NO — and the ground is that the program already has a job for the
other case.** The umbrella's A-iii row plans an **ungated `tools` job with an interpreter
floor**. With that job in the plan, "one issue, one way" gives each job one rule:

| job | floor | occupant |
|---|---|---|
| `trip-wires` (ungated, exists today) | the shell, `git`, `grep` — nothing to install | checks that need no interpreter |
| `tools` (ungated, A-iii) | an interpreter, declared | checks that need one |

The alternative — letting `trip-wires` acquire an interpreter floor — buys nothing A-iii's job
does not already buy, and costs the property that makes `trip-wires` cheap enough to be ungated
in the first place. It would also leave **two** jobs with an interpreter floor and no rule
saying which takes a new check.

⚠ **This is a finding about another lane's PR, and it is raised, not imposed.** Under this
answer #510's wire has the wrong *home*, not the wrong *content*. The write path is in #510's
files — another branch's, which this diff cannot write whatever §1 says (a previous revision
cited §1's boundary predicate here, and that predicate covers only `.claude/tools/_webref`). So
"no" **cannot be executed here, only agreed**. It is carried to #510 as a design question for that lane, and
whichever way it settles, the losing side's rationale text is the edit, not this wire.

⚠ **The budget half is not settled by this answer and is not this slice's to settle.** What the
always-run job may *cost* has three claimants (above); what it may *require* has one. This memo
answers the second only, and says so rather than letting the first ride along.

⚠ **Nothing in this slice blocks on the outcome.** This wire needs no interpreter under either
answer; the question is about the *set*'s floor. If #510 lands first with the opposite answer,
what changes is the rationale paragraph's wording at the four §5-item-4 sites — not this
instrument.

## §7 Exit criteria

1. `bash .claude/tools/webref-generic-core-trip-wire.sh` → PASSED, and
   `bash scripts/trip-wires.sh` → 0. ⚠ This restates the wire's own verdict; it is a smoke
   check, not an assertion about the wire, and is listed as such. **Met** (both 0 at this head).
2. Removing the controls file ends the run at exit 2, *"decided nothing"* — verified in both
   directions, since a split that can silently skip its own controls is worse than no split.
   **Met**, and §5 item 5 added the other direction: the controls file run on its own now exits
   **2** naming what it is, instead of failing inside `mktemp`.
3. **The mutation set is enumerated in the controls file itself, machine-readably, and each
   entry is shown to red when reverted. Met — the run prints the count and `not killed as
   named`, and the count is ratcheted rather than quoted here.**

   ```sh
   WEBREF_WIRE_MUTANTS=1 bash .claude/tools/webref-generic-core-trip-wire.sh
   ```

   One record per entry, TAB-separated: a `sed` expression over the wire, and the substring the
   run must print. Each entry must (a) actually change the copy — a stale anchor is a **failed**
   entry, not a passing one — (b) exit non-zero, and (c) print **the named control's own**
   diagnostic, so an entry that reds for an unrelated reason is caught. It is **not** in the
   gate: it costs one full control pass per entry, and a required check nobody can afford to run
   is how gates get switched off.

   ⚠ **The population is the point.** Draft 1 pointed at *"each fix this wire's history names"* —
   prose scattered across review commits that live on #501 and are **erased by its squash
   merge**, mixed with refactor commits whose revert reds nothing. This set ships in the file it
   is about. ⚠ **It is a floor, not a census**: nothing can detect an arm that never had a
   record, and the file says so.

   ⚠ **Three things the set found on its first honest run, each of which had been invisible:**
   - **The harness's own probe had the wrong subject.** `sed` writes mode 644 and `_control`
     invokes `"$SELF"` *directly*, so every mutant exited 126 for every control — which reds the
     run and prints every control's diagnostic, satisfying (c) **vacuously**. All 18 entries
     "passed" that way. The negative control that exposed it was a deliberately inert entry (a
     comment-only edit); the fix is `chmod +x` on the copy, recorded beside it.
   - **`-a` on `_content`'s grep was not asserted by the binary control.** Without it this grep
     prints `Binary file … matches` and exits **0**, so the run still reds — with a record naming
     a temp blob path instead of the entry and line. The control's needle was the headline
     (`"K2: a"`), which both spellings print; it is now the **record** (`control.dat
     (worktree):1:`), which only the read produces.
   - **One value stated at two levels, load-bearing at a different level each time.**
     `GIT_NO_REPLACE_OBJECTS` **is** in `git rev-parse --local-env-vars`, so `_git`'s routing
     purge clears it and only the re-export inside survives; `GIT_NO_LAZY_FETCH` is **not**, so
     there the top-level export was the live copy. The mutation aimed at the outer
     `GIT_NO_REPLACE_OBJECTS=1` survived because **the line it changed decided nothing**. Both
     now live in one place, after the purge. (And the variable is **presence-checked**: `=0` does
     not turn replacement back on, which is why the entry removes the assignment.)
   - **And one the set itself introduced**, caught by running the wire under the shell it
     commits to: bash 3.2 (stock macOS) still parses expansions inside a **quoted** here-document
     nested in a command substitution, so `_MUTANTS="$(cat <<'MUTANTS' … )"` aborted the entire
     controls file with `_v: unbound variable` — on **every** run, mutation mode or not — while
     bash 5.3 was green. The table is a function now. ⚠ The lesson is the cheap one: the file
     declares bash 3.2 compatibility, so `/bin/bash` is part of verifying it, not an afterthought.

4. Each of §5's eight items is answered **in this memo**, and each answer says whether it
   changes the implementation, so "answered, status quo" is distinguishable from "unanswered".
   **Met** — §5 is the answers; items 1, 2, 3, 4, 5 and 7 changed the implementation, item 6 is
   covered by item 5's change, item 8 changed nothing.
5. `git diff --name-only webref-cite-audit-tool...HEAD` matches **§4's table** — no count, and
   §4 includes this memo. ⚠ The artifact set is **not frozen**: §5 items 2, 5 and 6 can each
   change it, and draft 1's version forbade exactly those answers by fixing the set first.
   **Met — and the set DID change**, which is the criterion working rather than failing: items 5
   and 6 were answered without adding a file, and criterion 3's mutation set ships **inside the
   controls file**, but the design review found #501's memo contradicting §8 and the umbrella
   missing the row its own repo-wide command needs. Both are now in §4's table. A version of this
   criterion that froze the set at five would have forced those two defects to land.

## §8 Defer slots

**None, and §5 item 1 is why** — not "none yet". A defer slot records work the slice owes; the
four declared blind spots record the **reach of a predicate**, which is a different object.
Registering the wire in CI changes who runs it and how often; it does not widen its subject, so
it cannot turn a statement about reach into an obligation.

⚠ **And for two of the four a slot would be unfileable on its own terms.** Interpolation and
bare top-level names are properties of a grep over arbitrary source text: *"closable here"* is
answerable only as **no**, so a `Re-evaluation trigger` could never fire — worse than the
undated trigger the lane's precedent already rejects (*"has nothing that forces a look"*).

⚠ **The cap arithmetic therefore does not arise.** Own deferrals: **0**, against a per-PR cap of
3. Draft 2 recorded four candidates over the cap and the cap policy's ban on deleting slots to
make the arithmetic work; nothing is deleted here — the candidates were never obligations.

**Where the five are stated**: one block in the wire, `WHAT THIS WIRE DOES NOT DECIDE`. #501's
memo used to restate two of them and to book all of them as slots on this PR; **both are swept
in this diff** (§5 item 1), so the single site is now true rather than asserted.

## §9 What the pre-push gate changed, and what it deliberately did not

Six reviewers ran against the first two commits of this slice: `/code-review high` and
`/elidex-review`'s five axes, then `/simplify`'s four angles. **0 CRIT**; the substance is in
§5 and §8 above. This section records only the decisions a later reader would otherwise
re-derive — including the ones that were *declined*, since an unexplained absence reads as an
oversight.

### The root the quality pass found

Five controls had grown a hand-written **precondition** — a probe re-deriving, from a fixture's
end state, the fact that its build had failed. They were added one at a time, each after a
control was caught passing over a tree that never posed its question, and the comment
introducing the first two declared the class closed at two. It was not: three more followed,
**two of them added by the commit that wrote the sentence.**

The fact all five probes reconstruct is free at the point of failure. Every fixture is now
built as `( … ) || _fixture_failed <name>`, and `_control` refuses to report on a fixture whose
chain did not succeed. **The population is therefore every control, not the five somebody
noticed** — verified by breaking four fixture builds, including `staged`, which never had a
bespoke probe and is now covered. Net −18 lines and five fewer ways to write one check.

⚠ **One of those probes had to be subtle**, which is the second argument against writing them
by hand: the run exports `GIT_NO_REPLACE_OBJECTS=1`, so the obvious probe for the `replaced`
fixture would have read the violating bytes whether or not the replacement took — passing
vacuously for the very reason its control exists. A build status has no such trap.

### The mutation set's bookkeeping moved out of the mutation run

The ratchet and the needle↔label correspondence are **static properties of the shipped file**
that cost milliseconds, and they were sitting inside the opt-in harness that costs minutes — so
a PR deleting a record or renaming a control stayed green until somebody ran it by hand. Both
are now checked on every run.

⚠ **And the ratchet changed shape**: a floor on the record *count* could not distinguish "one
deleted, one added" from "unchanged", and said nothing about *which* control had been left
bare. What is ratcheted now is **the number of controls with no record** — the gap, not the
population — so adding a control without a record and deleting a record both red, by name.
⚠ The direction that had been checked has **never had a violation**; the direction that had not
is where both real gaps lived.

⚠ **The standing negative control's edit is now comment text.** Its first version appended a
space to an `rm -f` argument list — inert, but only *accidentally*, and that `rm` is itself
dead. A negative control whose inertness depends on the current behaviour of the code it edits
stops being one the moment that code changes, silently.

### Taken on cost, since this job runs on every PR

`_esc` and `_onerec` were `printf | sed | tr` pipelines called per entry per source — **306 of
the scan's 592 process spawns**. They are parameter expansion now, output proven byte-identical
on bash 3.2 and 5.x over backslashes, embedded newlines, tabs, `~`-leading strings and the
empty string. ⚠ The form matters: a literal `~` replacement is **tilde-expanded**, and quoting
the variable emits literal quote characters under bash 3.2 — `_T=$'~'` used *unquoted* is the
one spelling correct on both. `_scan`'s five `mktemp` calls are gone too: `$SCRATCH` is already
a per-process directory and `_scan` runs once per process. Derive the result rather than
trusting a figure here — `/usr/bin/time -p bash scripts/trip-wires.sh`.

### Declined, with grounds

| declined | ground |
|---|---|
| **Run the 40 controls concurrently** (measured 3.5–4.4× on the harness's dominant cost) | The right change, at the wrong time. It restructures the output and ordering of the instrument whose correctness this PR exists to establish, and it needs a review round of its own. Recorded as a follow-up rather than smuggled into the PR that is stabilising it. |
| **Skip the HEAD pass when its blob SHA equals the index's** (measured −37% of the scan) | Sound in principle — comparing object ids is a measurement, not an assumption. But every reproduced defect in this walk's history (#501 R92/R93/R95) came from reading one source and *inferring* another, and the wire's own account commits to reading each source rather than assuming two of them. Not worth re-opening for a gate that runs in single-digit seconds. |
| **Compute the entry-NAME check once instead of per source** (redundant by construction) | Correct, but it requires restructuring the walk — the part of this instrument with the worst defect history — for ~0.3 s. |
| **Collapse `_verdict`'s three passes into one** | The three-way grep-status rule is this file's most-cited invariant. The duplication was real and is fixed by a `_classify` helper (one implementation, three calls); merging the *passes* would trade a decision surface for a subtler one. |
| **A shared shell library for `_control` / `st_probe`** | They are genuinely two implementations of one idiom, and `st_probe` still lacks the watchdog `_control` has. But no `scripts/`-level shell library exists, and creating one is not this slice's to do. Recorded so the third copy does not have to rediscover it. |
| **Replace the content scan with `git grep`** | The wire's own header records that `git grep -a --no-index` did **not** match inside a `.pyc` on this machine where plain `grep -a` does. A wire whose reach depends on which git is installed is not an absolute. |

## §10 External review (Codex) — round 1

Ten unresolved threads were already on #519 from a review of the carve commit, and this slice's
own pre-push gate had not seen them. Eight were real at the head they were written against;
**two fail-open classes and two false-positive classes survived to this head** and are fixed
here. The two the loop did not change are answered, not waved away.

| # | What | Disposition |
|---|---|---|
| **P1** | An inherited exported `WEBREF_WIRE_SELFTEST` redirects `ROOT` at an arbitrary fixture **and skips every control** — reproduced by the reviewer: exit 0 over a real tree holding a violation | **Fixed.** Self-test mode now requires a companion token the controls set; a leftover export makes the gate **refuse loudly** instead of answering about the wrong tree. Same class as the `WEBREF_WIRE_MUTANTS` bypass §9 records, on a variable that predates it. Control + record. |
| **P1** | The HEAD probe treated **any** non-zero `rev-parse --verify --quiet` as "unborn", so an operational failure silently disabled the whole HEAD pass | **Fixed.** Measured: unborn is **1**, "git cannot answer" is **128**. `>1` is now an `err` record. Control (a `git` shim failing only `--verify`) + record. |
| **P1** | *"Restrict K2 to the generic layer"* — `DESIGN.md` §Architecture assigns path heuristics to the **adapter**, and `commands/agent_brief.py` is inside K2's scope | **Answered in §2, and the memo's earlier answer withdrawn.** The two definitions of "generic core" genuinely differ; K2 is the wider one by deliberate choice. Raised as a cross-slice question with both resolutions named — not settled here. |
| **P2** | The staged-symlink blob read: the trailing-newline sentinel preserved the value but **not the status**, so a failed `cat` became a successfully-read empty target | **Fixed.** `R%d` carries both. Control (a failing `cat` shim) + record. |
| **P2** | `_match_path`'s pipeline: under `pipefail` a pre-processing failure is reported as `grep`'s 1, i.e. as "no match" | **Fixed.** No pipeline — the value is computed first. ⚠ §9's `_esc`/`_onerec` change had already removed the `tr` this could fail in, which narrowed the class without closing it. |
| **P2** | Closing punctuation counted as a path segment: `See (.claude/tools/foo/) for details` reds the gate on a **one-segment** reference outside K2 | **Fixed.** `)]}>,;` joined the running-text terminators. ⚠ The first attempt put `]` in the middle of the bracket expression, which **closes it** — the predicate silently matched nothing and the `routed` control caught it. Green-direction control + record. |
| **P2** | No leading boundary before `.claude`, so `https://example.claude/skills/team/rule.md` and an entry named `…/example.claude/skills/…` matched on a suffix | **Fixed** in both predicates. Green-direction control + record. |
| **P2** | The `cachedir` control could pass from the **index arm alone**, so a regression in the worktree arm stayed green | **Fixed.** The staged blob is clean and the violation is worktree-only — verified by disabling the worktree content arm, which the control now catches and previously did not. |
| **P1** | *"Complete the mandatory plan review before landing the wire"* | **Already true.** Written against the carve commit, before this memo existed; the memo is §4's first artifact and has had a plan-review round plus a five-axis design gate. |
| **P2** | *"Track or close the host-path classes left outside the gate"* | **Answered in §8**, which the reviewer's head predates: no slots, with the argument (a slot records work owed; a blind spot records the reach of a predicate) and the reason a re-evaluation trigger could never fire for two of the classes. §5 item 1 also adds the **fifth** class the reviewer's list did not have. |

⚠ **Two of these are the same shape as defects this slice found in itself** — a required gate
exiting 0 over something it never read, because an environment variable put it in a mode it
was never asked for. Three instances now (`WEBREF_WIRE_MUTANTS`, `WEBREF_WIRE_SELFTEST`, and
the HEAD probe's collapsed status). The general lesson is recorded where the modes are entered
rather than in a fourth ⚠ here: **a mode this wire can be put into from outside must be
unreachable or loud, never silent.**
