# Slice A-i-wire — the K2 generic-core layering trip-wire

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire**.
**Branch**: `webref-generic-core-trip-wire`, stacked on `webref-cite-audit-tool` (#501).
**PR**: #519. **Status**: implementation carried from #501 `611758ff`; **plan-review round 1
returned 2 CRIT / 25 IMP / 19 MIN**, whose disposition is this memo. The one fork the
disposition left open — where this instrument should live at all — was **decided on 2026-09-21
(§1.1, option (a))**, and this revision answers §5's eight items, §6's interpreter question and
§7's mutation-set criterion. What those sections leave open is booked in §8, and §11 is the
revision that followed them.

⚠ **The rule for this memo is to state no quantity that moves with a commit.** Where a number
matters it appears as the command that produces it. Historical quantities (review-round numbers, gate
tallies) are provenance and are cited to the lane SSoT
`memory/project_citation-hygiene-program.md`, which owns them; they are not re-derived here.
⚠ **Draft 1 broke that rule in its own §1**, with a file count that was false when it was
written and unsatisfiable forever after, and four review axes found it independently. **§9
broke it a second time**, with a process-spawn count of the scan. The rule is easy to state
and was not easy to keep.

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
draft 1 implied: the disproportion — in code, in fixtures and in runtime — is in the
**controls**, not the scanner. Any argument about whether this instrument is too large has to start there.

⚠ **The asymmetry is a separate argument from the size**, and draft 1 conflated them by
ruling the size out of scope right after handing the reader the commands. Both are in scope:
#501's own thesis is enforced by nothing until slice A-iii wires the Python suites into CI,
and a slice carrying a permanent gate for someone else's invariant while its own goes
unenforced is what *"a slice may not carry another slice's concern"* forbids (umbrella,
"Constraints each slice inherits").

## §1 What this slice is, and is not

**Is**: one invariant (K2, over running text and over stored paths), one instrument, one
registration, and the two policy paragraphs that registration needs — the artifacts §4 tables.

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
`DESIGN.md`'s is the modules it names; K2's (#501 §2) is `_webref/` **plus the entry
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
the second sentence at all. That substitution is exactly what made
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
| `.claude/tools/webref-generic-core-trip-wire.controls.sh` | the controls: a fixture tree per assertion, each made by re-invoking the scanner over it — **not** one per verdict site; the sites with none are `#11-k2-wire-verdict-site-controls` in §8. Sourced by the wire; its interface is asserted at entry, and run on its own it exits 2 saying so |
| `.claude/tools/webref-generic-core-trip-wire.harness.sh` | the control harness: how a control runs — the fixture scratch root, the fixture git helper, the shim quoting, the FIFO probe, `_control`. Sourced by the controls file, which asserts the wire's interface first |
| `.claude/tools/webref-generic-core-trip-wire.mutations.sh` | the mutation set (§7 criterion 3) and the correspondence between it and the controls — *is each control about the arm it names?*, which is a different question from the controls' own. Split out at R6; same entry contract. It carries a second population beside the hand-written one: the boundary mutants **derived** from `$K2RE` and `$K2RE_PATH`, with the equivalence table that is their only escape from "must be killed" |
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

**Review added a class nobody had listed**: a `.claude/(skills|tools)/` path with **one**
further segment, of which this package's own entry script is the live case — live rather than
hypothetical, and the derivation is in the wire's block, since the count moves with the
package. Unlike classes
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
⚠ **SUPERSEDED**: the per-fixture precondition described here was replaced by the build-status
mechanism (`_fixture_failed`), which answers the same question for every fixture rather than
re-deriving it from one fixture's end state.

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
**absence of a setup step**: nothing to install, no cache, no network. The `trip-wires` job's own
comment had already reached that formulation (*"What the decision rests on is the ABSENCE OF A
SETUP STEP"*); the other three sites had not.

⚠ **AND THE FIRST REPLACEMENT WAS FALSE IN THE SAME SHAPE.** This item's first attempt wrote
*"the shell, `git` and `grep` a bare checkout already has"* at all four sites. Four reviewers
measured it independently: the wires in that job also call `sed`, `tr`, `cmp`, `readlink`,
`mktemp`, `mkfifo`, `chmod`, `env`, `cut`, `ln`, `cp`, `awk`, `sort`, `comm` and `wc`.
A narrower-than-true enumeration, written by the edit
retiring a narrower-than-true enumeration — which is what "a check defined by the symptom
vocabulary" costs: the residue command below *used to* grep for the retired **phrase**
(`grep-only`), so it could not see the new one.

**The property, settled**: the wires need **nothing installed** — no language runtime, no
package manager, no cache, no network. ⚠ **And that is stated as a property, not a list**,
because the list has now been wrong twice. What makes it checkable is the job's shape:

```sh
# the whole claim, derived: the trip-wires job has ONE `uses:`, the checkout.
sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml | grep -c 'uses:'   # -> 1
# ⚠ A SEED, NOT THE CHECK. The line below names five runtimes; a sixth passes
# it in silence, and silence is this list's green — the wrong side for a rule
# in a required job, and the third time an enumeration has stood in for this
# property in this section. It is kept because it does catch the regression
# that actually happened (#501 R69, python3), and it is NOT widened: this
# wire's own header carries the rule that a predicate which cannot return its
# population is a seed, and that lengthening its regex is the wrong repair.
# What is authoritative is the derivation above — one `uses:`, so there is no
# setup step — and §6, which decides what the ungated set may require.
# ⚠ The second filter is not decoration either: without it the only hit is a
# COMMENT recording that an earlier revision used python3, so the bare grep
# cannot tell the history of the rule from a violation of it.
grep -rnE '\b(python3?|node|ruby|perl|cargo)\b' \
     .claude/tools/*-trip-wire*.sh scripts/trip-wires.sh \
  | grep -vE ':[0-9]+:[[:space:]]*#'          # -> no output, for those five
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
   define it. Corrected to what the file consumes and what it defines — and the list that
   matters is the guard's, in the file, not this one.
2. That list is now **asserted at entry**, name by name, rather than described.
3. A direct invocation therefore says what this file is and what to run instead — **measured,
   `rc=2`** — instead of failing inside `mktemp` with a message about a read-only root.

⚠ The one property a separate process would have bought — running the controls without also
scanning the real tree — is not one this wire wants: its contract is that a green is earned by
both halves of the same run.

⚠ **And no second seam is taken — as judged at the time. That judgement is now SUPERSEDED; see
§8.** Re-derive the sizes rather than reading them (§0's first command): when this was written
the scanner was in the band CLAUDE.md's touch-time discipline says to look at *while writing*.
The reasoning below is kept
because it is what a re-derivation at that size has to argue against, not because it still
concludes. Looked at then, the answer was no: the split rule is a **cohesion** judgement, not a
line count, and what is left is the predicates, one walk, one verdict — a monolithic cohesive
unit, mostly rationale (§0's first command prints the
code/comment split; a previous revision put a fraction here, and it was both a quantity that
moves and wrong). The cut that existed was the one already taken. This is recorded rather than
left implicit, because "the file is in the band and nobody said why it stayed" is how the
discipline degrades into a count.

⚠ **And the same judgement is owed for the CONTROLS file, which a previous revision did not
give.** It carries two subjects — the fixture controls, and the mutation harness the wire gates
behind `WEBREF_WIRE_MUTANTS` — and that looks like a seam. ⚠ **SUPERSEDED: it was taken (§8),
and a second seam with it** — the control *harness* (how a control runs) from the control
*catalogue* (which controls exist), when §11's new controls took the catalogue past the
threshold. The argument recorded here was against a split that moved the *table* and left the
*check* behind; the correspondence check moved with its table, and reds cross-file, which is
what §8 says.

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
for w in .claude/tools/*-trip-wire.sh; do echo "$w"; done   # the driver's glob; controls absent
bash .claude/tools/webref-generic-core-trip-wire.controls.sh; echo "rc=$?"   # 2, and says why
```

⚠ **Round 1's supporting claim does not survive measurement and is withdrawn**: *"this memo's
own §0 command and the driver's disagree"*. They do not — §0's `wc -l .claude/tools/*-trip-wire.sh`
**is** the driver's glob and returns the same files. What §0 does is name the two artifacts
**explicitly** in its first command, because the proportionality question is about the artifact
pair while the driver's question is about the registered set. Two questions, two commands, both
correct; the near-miss name is what made them look like one question with two answers.

⚠ **Mode 755 is kept.** The file is *sourced*, not executed, so the bit decides nothing about
how the wire reads it; after item 5 it is no longer misleading either, since running the file
directly reports what it is.

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
that rewrite is deliberately **silent on the interpreter question**: it states the property the
ungated job rests on (there is no setup step) and does not say what a future wire may not need. The answer below is the memo's, not a paragraph's.

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
| `trip-wires` (ungated, exists today) | no setup step — nothing to install | checks that need no interpreter |
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
   **2** naming what it is, instead of failing inside `mktemp`. The harness split carries the
   same guard in both directions (measured).
3. **The mutation set is enumerated in `…trip-wire.mutations.sh`, machine-readably, and each
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
   is about. ⚠ **Split out of the controls file at R6** — the criterion said "in the controls
   file itself" while §4's table already assigned the records to the new file, i.e. the acceptance
   criterion and the artifact table gave competing accounts of the seam. The property the
   criterion is about is *ships with the thing it is about*, which the split preserves.
   ⚠ **It is a floor, not a census**: nothing can detect an arm that never had a
   record, and the file says so.

   ⚠ **…which is why the two regexes' own population is no longer a list.** The same run also
   generates, from `$K2RE` and `$K2RE_PATH` as the wire assigns them, one mutant per
   bracket-expression member and one per quantifier, and requires each to red the control set or
   to carry an argument in the file's equivalence table. That is the half a hand-written set
   cannot supply — a floor cannot see the entry nobody thought of — and it is what makes
   §11.7's corrected item 6 checkable rather than asserted. The boundary is stated at both
   sites: the hand records name **which** control catches a rule; the generated set asserts
   **that** every rule those two regexes spell is caught by one.

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
   and 6 were answered without adding a file; the design review found #501's memo contradicting
   §8 and the umbrella missing the row its own repo-wide command needs; R6 added
   `…trip-wire.mutations.sh` when the touch-time split landed, and §11's implementation added
   `…trip-wire.harness.sh` when the new controls took that file back to the threshold. All are
   now in §4's table. A version of this criterion that froze the set at five would have forced
   those defects to land.

## §8 Defer slots

**The slots this slice owns are slots 1 and 2 below, plus `#11-k2-wire-verdict-site-controls`** (§11.4; registered in the defer ledger) — named rather than tallied, because the tally is what went wrong. **This section has been wrong in both directions before.** An earlier revision said
**zero** while the loop had added obligations it was never reopened to see; the revision that fixed
that booked a **third** slot for work the rule required to be done, not deferred (R6, below). That was right about the four
declared blind spots and wrong as a total: the loop added obligations §8 was never reopened to
see. The distinction it draws still holds — *a defer slot records work the slice owes; a
declared blind spot records the reach of a predicate* — and it is what sorts the list below.

### Not slots, and why (unchanged)

The classes in the wire's `WHAT THIS WIRE DOES NOT DECIDE` block are reach, not work.
For interpolation and bare top-level names, *"closable here"* is answerable only as **no**, so a
`Re-evaluation trigger` could never fire — worse than the undated trigger the lane's precedent
already rejects. ⚠ Two classes were added after this section was first written, and they are
recorded at §10.3 and §10.4.

### ~~Slot 1 — the touch-time split~~ **WITHDRAWN and DONE (R6).** It was never a slot.

⚠ **The slot inverted the rule it cited, and the external reviewer caught it (P1).** CLAUDE.md's
heading is literally *"1000-line debt = touch-time split (**defer しない**)"*, and *prereq* means
**before**. The slot used the other half of the same sentence — *"split は単独 PR / 単独 commit"* —
as permission to **merge the oversized files first and schedule the split afterwards**, which is
the one ordering the rule exists to forbid. Reading a rule's constraint as a licence for the thing
it constrains is the failure worth naming here; the sentence admits **単独 commit**, so the split
lands *in* this PR, as its own commit, before merge.

**Done**: `.claude/tools/webref-generic-core-trip-wire.mutations.sh`, at the seam the reviewer
named — fixture/control execution versus the opt-in mutation harness.

**And again in §11's implementation**, when the new controls took the controls file back to the
threshold: `.claude/tools/webref-generic-core-trip-wire.harness.sh`, at the seam between **how a
control runs** (the fixture scratch root, the fixture git helper, the shim quoting, the FIFO
capability probe, `_control` itself) and **which controls exist** (each fixture and the assertion
over it). Its own commit, before the fixes that needed it.

⚠ **And the memo's own counter-argument is answered rather than dropped.** §9 had argued against
this seam because *"the two lists must be edited together, so splitting them puts the two halves
of one assertion in two files"*. They must — and the correspondence check is what **enforces**
that instead of hoping for it. Being cross-file is the point: it reads the controls for labels and
the mutation file for records, and reds when they drift. The argument was for a version of the
split that moved the *table* and left the *check* behind; that is not this split.

**The wire is NOT split, and that is a cohesion judgement** (§0's first command prints its
code/comment split; the figures are not carried here, because they move with every commit).
CLAUDE.md's discipline is explicit that the test is cohesion, not line count, and exempts
*一枚岩の cohesive unit*: what is left is **two predicates** — `$K2RE` over running text and
`$K2RE_PATH` over stored paths — over one walk into one verdict, and the length is recorded
incident rationale rather than logic. ⚠ **If that is wrong, the seam to propose is
predicate-vs-walk** —
"what counts as a hit" against "what is read" — and it is named here so the next reviewer argues
against a position rather than a silence.

### Slot 1 — `_match_path`'s `|| return 4` is unpinned

| | |
|---|---|
| **Why deferred** | `_onerec` is parameter expansion, so nothing external is left for a shim to break and the only way to reach the guard is to edit the function — a mutation, not an input. The guard is kept for the shape one refactor away (an `_onerec` that shells out again). |
| **Re-evaluation trigger** | Any change that puts an external command back into `_onerec`. |
| **Re-evaluation date** | 2026-12-31 |

### Slot 2 — the ratchet's `wc -l` path is unpinned

| | |
|---|---|
| **Why deferred** | The bug it fixes (`grep -c .` exiting 1 on an empty file) fires only when `.bare` is empty — when every control has a mutation record. No run reaches that state today. |
| **Re-evaluation trigger** | An edit that takes `_MUT_UNRECORDED_MAX` to 0. ⚠ **Nothing makes that arrive on its own**, and an earlier wording said the ratchet did: the constant is hand-edited, and the harness's own diagnostic offers *raising* it in the same breath as lowering it. So this trigger is a decision someone takes, not a state the instrument drifts into — which is also the argument for closing the slot instead, by pinning the path with a control that empties `.bare`. |
| **Re-evaluation date** | 2026-12-31 |

⚠ **This slice's own deferrals are exactly the slots named at the head of this section, and the policy caps a PR's own deferrals at three** — so one more means closing one of these rather than re-labelling it. ⚠ And the
honest note on both: both are *"a code path the gate's own tests cannot reach"*, which
is a smaller admission than a blind spot but a real one, and the cap policy forbids deleting a
slot to make arithmetic work — so if a fourth arrives, one of these has to be closed rather than
re-labelled.

## §9 What the pre-push gate changed, and what it deliberately did not

Six reviewers ran against the first two commits of this slice: `/code-review high` and
`/elidex-review`'s five axes, then `/simplify`'s four angles. **0 CRIT**; the substance is in
§5 and §8 above. This section records only the decisions a later reader would otherwise
re-derive — including the ones that were *declined*, since an unexplained absence reads as an
oversight.

⚠ **The pre-push gate's Stage 5 `/review` did not run as a separate stage, and that is not a
skip.** Since Claude Code v2.1.223 `/review` is an alias of `/code-review`
(https://code.claude.com/docs/en/code-review.md, "Review a diff locally"), so Stage 5 and the
`/code-review high` above are the same command. The gate's own definition is corrected in a
separate PR (`skills-pre-push-review-alias`).

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

`_esc` and `_onerec` were `printf | sed | tr` pipelines called per entry per source, so each
record cost processes. They are parameter expansion now, output proven byte-identical
on bash 3.2 and 5.x over backslashes, embedded newlines, tabs, `~`-leading strings and the
empty string. ⚠ The form matters: a literal `~` replacement is **tilde-expanded**, and quoting
the variable emits literal quote characters under bash 3.2 — `_T=$'~'` used *unquoted* is the
one spelling correct on both. `_scan`'s five `mktemp` calls are gone too: `$SCRATCH` is already
a per-process directory and `_scan` runs once per process. Derive the result rather than
trusting a figure here — `/usr/bin/time -p bash scripts/trip-wires.sh`.

### Declined, with grounds

| declined | ground |
|---|---|
| **Run the controls concurrently** (the harness's dominant cost) | It restructures the output and ordering of the instrument whose correctness this PR exists to establish, and it would need a review round of its own. **Not owed, so not a slot**: the gate is correct without it, and nobody is committed to doing it. Whoever next finds the runtime a problem starts from `/usr/bin/time -p bash scripts/trip-wires.sh`, not from a figure recorded here. |
| **Skip the HEAD pass when its blob SHA equals the index's** | Sound in principle — comparing object ids is a measurement, not an assumption. But every reproduced defect in this walk's history (#501 R92/R93/R95) came from reading one source and *inferring* another, and the wire's own account commits to reading each source rather than assuming two of them. Declined on that ground alone; the runtime it would save is not an argument this memo can make without a figure, and it records none. |
| **Compute the entry-NAME check once instead of per source** (redundant by construction) | Correct, but it requires restructuring the walk — the part of this instrument with the worst defect history — for a saving too small to trade that risk for. |
| **Collapse `_verdict`'s three passes into one** | The three-way grep-status rule is this file's most-cited invariant. The duplication was real and is fixed by a `_classify` helper (one implementation, three calls); merging the *passes* would trade a decision surface for a subtler one. |
| **A shared shell library for `_control` / `st_probe`** | They are genuinely two implementations of one idiom, and `st_probe` still lacks the watchdog `_control` has. But no `scripts/`-level shell library exists, and creating one is not this slice's to do. **Not owed, so not a slot** — this row is a pointer for whoever writes a third copy, not an obligation anyone holds. |
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

### §10.1 Round 2 — five findings, two of them made by round 1's fixes

| # | What | Disposition |
|---|---|---|
| **P2** | An inventoried worktree path that **vanishes** before it is read matches none of `_entry`'s arms — no `ok`, no `err`, **no record at all** — and the final guard only requires the *aggregate* `SCANNED` to be non-zero, so its siblings carry the run to green. Reproduced: a forbidden untracked file removed just after `ls-files` gave exit 0 | **Fixed.** An absent tree entry that git does **not** report as tracked is now an `err`. ⚠ The membership question is asked of git rather than inferred from absence, because a *tracked* path deleted from the worktree is the legitimate case — the index pass already answered for it. Control: a `git` shim whose `--others` inventory names a path that is not there (the deterministic form of the race). |
| **P2** | `_match_path` **still** ignored `_onerec`'s status after §10's fix: it is called beneath `\|\|` in `_stored`, where `errexit` is suspended, so a failing assignment simply fell through to `grep`, which returned 1 for the empty value — the same misclassification by another route | **Fixed** with `\|\| return 4`. ⚠ **Removing the pipeline was not enough, and round 1 stopped there** — a fix aimed at one channel while the value travelled another. ⚠ **No control pins this**, and the wire says so: `_onerec` is parameter expansion now, so nothing external is left for a shim to break and the only way in is to edit the function. Kept for the shape one refactor away, on the same footing as the `-a` on `_verdict`'s arms. |
| **P2** | Round 1's punctuation terminators bought the false positive back as a **false negative**: excluded from the segment *entirely*, a real path `.claude/tools/team,inc/rule.md` stopped at the comma and read K2 zero | **Fixed.** A segment is now *"any run of path characters that does not END in punctuation"*. ⚠ The green control pins one direction and a red control the other; **neither alone could have caught this**, which is the argument for having both. |
| **P2** | The watchdog's `kill -9 "$_cpid"` reaches only the wire's own bash — a command substitution or `grep` **blocked beneath it**, precisely the FIFO hang the watchdog exists to catch, is reparented and stays blocked, so repeated local runs accumulate permanent orphans | **Fixed.** The child is started under `set -m` so it is its own process group, and the group is killed. Measured on bash 5.3 and 3.2: group kill reaps the descendant, top-PID kill does not. |
| **P2** | The green boundary fixture exercised only the **entry name**, so it pinned `$K2RE_PATH` and not the independently-changed arm in `$K2RE`; removing the boundary from the running-text predicate reintroduced the URL false positive with every control green | **Fixed.** A second fixture carries the URL as file *content*, and the mutation record that targeted only the stored-path regex now has a sibling for the running-text one. |

⚠ **Two of the five were introduced by round 1**, and both are the same mistake: a fix aimed at
the site the finding named rather than at the property. The punctuation repair traded one
direction of wrongness for the other; the pipeline repair removed the channel the status was
being lost through and left the one it was actually travelling. The mutation set caught neither
— they are not reachable by mutating a guard, they *are* the guard being wrong — which is the
clearest statement available of what that instrument does and does not buy.

### §10.2 Round 3 — the root, and why three rounds found it one site at a time

**All four findings were of one shape**: *"fresh evidence after the claimed fix"*. That is the
loop's ≥2-round recurrence trigger, so the root-check ran before any of them was patched.

**Q1 — is a canonical algorithm missing, or are these N ad-hoc paths each carrying a subset
bug?** The latter, and the algorithm has a name now:

> **A non-zero status is not a specific negative.** A command's failure may be read as a
> *particular* negative (*"there is no HEAD"*, *"this path is untracked"*) only when a
> separate, **positive** test establishes that negative. Otherwise it is an ERROR.

The reason it kept biting is that **the wrong reading is always the convenient one** — it turns
*"I could not find out"* into *"there is nothing to find"*, which is the direction that makes a
gate green.

**And the audit is written out**, so *"is there another site?"* has an answer rather than a
guess. Every status this file reads is enumerated in the wire's header with what it concludes:
**two sites inferred a specific negative** (the HEAD probe, the tracked-membership question) —
both are the R3 findings, both fixed — and every other site concludes only *error*, from a
documented contract (`grep`'s 1-vs-≥2) or from nothing at all. A new `git` call joins that list
or it is a defect.

**Q2 — own-ideal test.** The wire's own header says *"ONE CHECK, ABSOLUTE … this wire makes no
un-asserted report"* and *"One authority answers all of it, and it is git."* The walk is not the
anti-pattern; **the scattered interpretation of git's answers was** — the opposite of one
authority. That is why the disposition is the named invariant plus an audit (option A: step
back and collapse) rather than a fourth site-patch.

| # | What | Disposition |
|---|---|---|
| **P2** | `rev-parse --verify --quiet HEAD` exits **1 for a malformed branch ref too**, so R2's "only >1 is an error" still skipped the HEAD inventory and exited 0 while HEAD lookup had failed | **Fixed.** Unborn is established *positively* — `rev-list -n 1 --all` exiting 0 with empty output. Measured: truly unborn = rc 0/empty, malformed ref = **rc 128**. |
| **P2** | The membership check read `$rel` as a **pathspec**: a vanished untracked `foo[1].py` matched a tracked `foo1.py` and was reported tracked, so the entry nothing had answered for was passed over — the hole that arm was added to close | **Fixed** with `--literal-pathspecs`. Measured: rc 0 without it, rc 1 with it. ⚠ Same lesson as `${var#"$prefix"/}` one layer up: **a path is data, not a pattern** — third instance in this file. |
| **P2** | The self-test companion token is a **fixed literal in this file**, so exporting both variables — exactly what copying the two lines out of the controls produces — still redirected a normal run | **Fixed.** The companion value is the parent's **live PID**; the child's own `$PPID` must equal it, which copying cannot satisfy. Verified on bash 5.3 and 3.2, and the reviewer's exact reproduction now refuses. |
| **P2** | R2's punctuation rule over-reached onto **both** segments, so `.claude/tools/team,/rule.md` — whose comma is followed by `/` and therefore cannot be prose — went undetected | **Fixed.** The restriction belongs only on the **final** segment, because that is the only place the end of the reference is ambiguous; an intermediate segment is delimited by `/`, which settles it. |

⚠ **The punctuation predicate was wrong three times in three different directions** — too loose
(prose reddened the gate), too tight (a comma inside a segment), then too tight again (a comma
before a slash). Each repair was aimed at the example in the finding. What finally settled it
was stating the *property* — *"ambiguity exists only where the reference ends"* — and it now has
a control on each side, because **neither direction alone could have caught the other**.

⚠ **And one R2 control had to be rewritten, not just kept.** `headprobe` encoded R2's rule ("a
failed probe is an error") on a fixture that was genuinely unborn — under the corrected rule
that fixture's green is *right*, so the control could no longer distinguish anything. It has a
commit now. **A control written against a rule outlives the rule**, and a passing control is not
evidence that it still asks the question it was named for.

### §10.3 Round 4 — PAUSE, and the claim that was driving the loop

`…trip-wire.sh` has now carried a finding in **four consecutive rounds**, which is this loop's
scope-creep PAUSE. The root-check ran again, and this time it lands somewhere different from
§10.2's.

**Q1 — abstraction coverage.** §10.2's invariant (*a non-zero status is not a specific
negative*) held up: R4's HEAD finding is a violation of it that my own R3 fix introduced — I
established the negative positively, as the invariant demands, **but for the wrong subject**,
asking whether the *repository* had commits rather than whether *this HEAD* did. The invariant
is right; its first application was not. No further abstraction is missing there.

**Q2 — own-ideal test, and this is the one that moved.** The wire's header said
*"ONE CHECK, ABSOLUTE. It is closed and decidable; it is not a heuristic."* Measured against
four rounds of evidence, **that is false of half of it**:

| | subject | status |
|---|---|---|
| `$K2RE_PATH` | a **stored path** — git hands the value over whole, `/` is the only delimiter | genuinely **closed and decidable**. One finding, in R1, stable since |
| `$K2RE` | **running text** — arbitrary bytes in an unknown language | a **bounded heuristic**. Every boundary finding in R1, R2, R3 and R4 was here |

*"Does a path reference start and end here?"* cannot be decided without knowing whether the
bytes are prose, code, Markdown, a URL or a `.pyc`. **The claim was the defect, not the
regex** — it made each counter-example read as "a bug to repair", so each repair was aimed at
the example and the next round found the opposite direction:

> too loose (prose in parentheses reddened the gate) → too tight (a comma inside a segment) →
> too tight again (a comma before a slash) → wrong in kind (`@` treated as a boundary, because
> the rule was written as *"not these few path characters"* instead of *"one of these prose
> delimiters"*).

**Disposition: option A — step back and collapse**, not a fifth boundary patch. The header now
states the two halves' different status, and with it the requirement that follows: **every
boundary rule carries a control in both directions**, because each of those four defects was
invisible to a control that tested only the other way. What the wire *does* is unchanged; what
it *claims* is now true.

| # | What | Disposition |
|---|---|---|
| **P2** | `rev-list -n 1 --all` asks whether the **repository** has commits; on an orphan branch HEAD is legitimately unborn while another branch has one, so a clean fixture reported a read error | **Fixed.** `symbolic-ref -q HEAD` + `show-ref --verify` on the named ref. Measured, all three: orphan = symbolic-ref 0 / ref absent; empty repo = same; **malformed ref = symbolic-ref 128**. Green-direction control (an orphan-branch fixture). |
| **P2** | The leading boundary admitted **path characters**: `foo@.claude/skills/team/rule.md` matched on the suffix of a component | **Fixed.** The list is positive — whitespace, a control character, a quote, a backtick, an opening bracket, or `/`. ⚠ `[:cntrl:]` is load-bearing and the **binary control caught its absence in the same run that introduced it**: that fixture wraps the path in NULs. |
| **P2** | `-f` **followed an ancestor symlink**: a tracked `dir/a.py` with `dir` replaced by a link to an external directory made the gate red over bytes `git add -A` would never stage | **Fixed.** A proper ancestor being a symlink is an `err` — not a skip, because that is how entries get passed over in silence. |
| **P2** | `grep -c .` on an **empty** `.bare` prints 0 and **exits 1**, so `set -e` aborted the gate — and it fires exactly when the ratchet reaches the state it exists to permit | **Fixed** with `wc -l`. Measured: `grep -c .` rc 1 / `wc -l` rc 0 on an empty file. ⚠ No control: reaching an empty `.bare` needs every control to have a record, which is not today's state. Recorded rather than implied. |

⚠ **And two of my own fixtures were wrong in ways a passing control hid.** The `headprobe`
shim matched `--verify` *anywhere*, so it also broke the `show-ref --verify` that now decides
unbornness — the shim had silently become a control for a different arm. Narrowing it to
`rev-parse --verify` then exposed a second defect: `*" rev-parse "*" --verify "*` **can never
match**, because the first half consumes the space the second needs, so the shim matched
nothing and the control exercised an unshimmed git. **A shim shims the one call it names, and
"one call" means the verb and the flag, as one adjacent sequence.**

### §10.4 Round 5 + the design re-gate — the round that judged round 4 wrong

R4 declared *"option A — step back and collapse, not a fifth boundary patch."* Codex's R5 and
the mandated cumulative design re-gate (two axes, run because PAUSE fired and a divergent loop
never reaches TERMINAL) agree it was **both**, and that the patch half was the worst change in
this PR.

**The CRIT: R4's leading-boundary rewrite made a required gate FAIL OPEN.** Replacing the
exclusion class with a positive delimiter list closed one contrived false positive
(`foo@.claude/…`) and opened at least five false negatives. Measured, HEAD-at-R4 vs the
revision before it:

| input | R3 | R4 |
|---|---|---|
| `DEFAULT=.claude/skills/a/b.md` | HIT | **MISS** |
| `--paths=.claude/skills/a/b.md` | HIT | **MISS** |
| `k:.claude/skills/a/b.md` | HIT | **MISS** |
| `` `.claude/skills/a/b.md` `` | HIT | **MISS** |
| `**.claude/skills/a/b.md**` | HIT | **MISS** |

**What was actually missing was not a better list — it was a stated failure DIRECTION.**
Neither form can enumerate its complement; the question is where an unknown character lands:

> an **exclusion** class makes it a *boundary* → over-match → **false positive** → the gate
> reds, somebody looks, somebody fixes it.
> a **positive** list makes it *not* a boundary → under-match → **false negative** → the gate
> is green and nobody ever finds out.

In a required gate those are not symmetric. The class is an exclusion again, `@` is handled
*inside* it (beside `+`, `%`, `-`), and the reasoning is in the wire so the next round cannot
re-derive it wrong. **Six new red-direction fixtures** pin the spellings above — the direction
that had no control at all.

**And the retraction had only reached the reader, not the gate.** Three reviewers independently
found that the line CI actually prints still said `K2: 0 … -- ABSOLUTE`, over a count that is the
**union** of the closed stored-path predicate and the half R4 had just called a heuristic. It now
prints the two separately. ⚠ This is the sweep-three-faces rule: R4 swept the *statement* and
neither the *obligation* (a control in both directions — unmet for the rule R4 itself wrote) nor
the *consequence* (the verdict line).

| # | What | Disposition |
|---|---|---|
| **CRIT** | the leading boundary failed open on five spellings | **Fixed** — exclusion class, direction stated, six red-direction fixtures + a mutation record |
| **IMP** | the printed verdict still claimed ABSOLUTE over the heuristic half | **Fixed** — the two predicates report separately |
| **IMP** | the status audit named `rev-list -n1 --all` as the positive test **after R4 removed it**, and omitted `symbolic-ref`, `show-ref`, `cat-file`, `tr\|cmp`, `_ancestor_link` | **Fixed** — table re-derived at HEAD with a derivation command, and the rule is now "re-derive when a mechanism changes, do not amend around it" |
| **IMP** | `_ancestor_link` was ordered **after** `[ -L "$f" ]`, so an external leaf symlink won and the gate reported a K2 hit on bytes outside the tree | **Fixed** — the ancestor question is asked first |
| **IMP** | *"THE WHOLE LIST … THE ONE PLACE IT IS STATED"* was stale: two rounds declared a non-coverage at its own site and a third created a new undecided class | **Fixed** — items 6 and 7 added, with the rule that a ⚠ beside the code is not the list |
| **IMP** | *"Every other control here proves the wire can RED"* — false; **10 of 53** are green-direction, two added three lines above the claim | **Fixed** — replaced by a derivation, which itself had to be run before being written down (the first spelling printed the wrong awk field) |
| **IMP** | §8 said **zero slots** while the loop had added obligations it was never reopened to see | **Fixed** — three slots, each with a fireable trigger and a date; own-deferral count 3, at the cap |
| **IMP** | both files crossed **1000 lines** during the loop; §5 item 5's cohesion argument was made against a 786-line file | **Slot 1**, which is the discipline's own shape: CLAUDE.md requires the split to be *its own PR*, so doing it here is what the rule forbids |
| **IMP** | `CLAUDE.md` still justified the ungated job with *"~1s"* while the measured figure is an order of magnitude higher — two SSoT statements disagreeing | **Fixed** — the justification is the absence of a setup step, and the stale figure is retired with the reason |

⚠ **The lesson I take from this round is about the loop, not the wire.** I ran the root-check,
wrote both mandated questions, reached a correct diagnosis — *the claim is the defect* — and
then shipped a patch of exactly the kind the diagnosis forbade, in the same commit, under a
heading saying I had not. **A correct root-check does not immunise the round it is written in**,
and the thing that caught it was the mandated design re-gate firing *because the loop was still
diverging* — the one guard that does not depend on my own judgement of my own work.

### §10.5 Round 6 — one finding, and it was about a rule I had inverted

**P1: *"Land the 1000-line split before this feature."*** R5's §8 had booked the touch-time split
as a slot to be done **after** #519 lands. That inverts the rule it cites twice over: CLAUDE.md's
heading is *"1000-line debt = touch-time split (**defer しない**)"*, and *prereq* means **before**.
What the slot did was take the sentence's other half — *"split は単独 PR / 単独 commit"* — and use
it as permission to merge the oversized files first.

⚠ **Reading a rule's constraint as a licence for the thing it constrains** is the failure worth
naming. The sentence admits **単独 commit**, which is exactly the shape available here: the split
lands *in* this PR, as its own commit, before merge.

**Done in this round**, at the seam the reviewer named: `…trip-wire.mutations.sh`, separating
*"can the wire reach every verdict?"* (the controls) from *"is each control about the arm it
names?"* (the mutation set and the correspondence between the two lists).

⚠ **Two things the split had to get right, and the standing negative control caught the second:**
- the **correspondence check moves with the mutation set**, not away from it — §9's argument
  against this seam was aimed at a split that moved the table and left the check behind, and that
  version would indeed have been worse.
- the mutation runner copies the **controls** beside each mutant; it now copies this file too.
  Without that, every mutant exited 2 (*"decided nothing"*) for a reason unrelated to its
  mutation, and the harness reported it as the entry failing. **The `!survive` entry died and
  said so** — a broken harness reporting itself, in the run that broke it.

**The wire is not split**, and that is a cohesion judgement (§0's first command prints the
code/comment split; no figure is kept here): two predicates over one walk into one verdict —
CLAUDE.md's *一枚岩の cohesive unit* exemption, which is a cohesion test and not a line count.
If that is wrong the seam to propose is **predicate-vs-walk**, named in §8 so the next reviewer
argues against a position rather than a silence.

### §10.6 Round 7 — five findings; two made by R6's split, one by the edit that retired a figure

| # | What | Disposition |
|---|---|---|
| **P2** | **A caller's git configuration reached the fixtures.** `_git` deliberately preserves `GIT_CONFIG*` (#501 R97 — a checkout readable only through a caller's `safe.directory`), which is right for the **real scan** and wrong for the **fixtures**. Reproduced here and by the reviewer: `GIT_CONFIG_COUNT=1 core.excludesFile=*.py` made the fixtures' `git add` skip their own `.py` inputs → **ten** `CONTROL NOT EXERCISED` on a clean checkout | **Fixed.** `_fgit` is a subshell that unsets `GIT_CONFIG` / `GIT_CONFIG_PARAMETERS` / `GIT_CONFIG_COUNT`; re-measured **10 → 0**. The asymmetry — preserve for the repository, scrub for the fixtures — is why there are two helpers, and it is now stated at the one that scrubs |
| **P2** | **One shim was failing both `ls-files` inventories.** `" ls-files "` is in the tracked (`--stage`) call *and* the worktree (`--cached --others`) one, so removing either arm's status check left the other's error to mask it — the reviewer deleted the worktree emission and the whole run stayed green. Mutation record 17 rewrote all three `_ls_rc` checks at once, so it only ever proved *at least one* remained | **Fixed.** `_ls_rc` → `_rc_tracked` / `_rc_worktree` / `_rc_head`, so each guard is individually addressable; the shim is narrowed to `--stage`; a `--others` shim, fixture, control and record are new, and the one collapsed record becomes three |
| **P2** | The split fragment's interface omitted `$_MUTATIONS` (which `_mut_run` reads to copy itself) and **wrote the caller's `ctl_ok`** — so a rename in the controls would leave this file assigning an unused global while the caller stayed green | **Fixed.** `_mut_correspondence` **returns** a status and the caller decides; `$_MUTATIONS` joins the checked contract. ⚠ This is the cross-file drift the entry guard exists to prevent, **reintroduced by the split that added the guard** |
| **P2** | `CLAUDE.md`'s `実測は一桁秒台` — a **new figure introduced by the edit that retired a stale one**, in a paragraph that says not to record elapsed time, and already false (the reviewer measured ~40 s) | **Fixed** by removing it, not replacing it. Third time this session a figure was introduced while retiring one; the rule is *derive it or omit it* |
| **P2** | §7 criterion 3 still said the records are *"in the controls file itself"* after R6 moved them, while §4's table already named the new file — the acceptance criterion and the artifact table giving competing accounts of the seam | **Fixed.** The property the criterion is about is *ships with the thing it is about*, which the split preserves |

⚠ **And one finding of my own, from fixing the second**: excluding `lsfail` from the fixture
repository loop had been justified by *"its shim fails `ls-files` whatever the directory is"* —
which **stopped being true the moment that shim was narrowed**. Its worktree inventory then ran
for real against a non-repository and the control got `read 0 stored objects` instead of the
inventory error it names. **An exclusion justified by another mechanism's breadth expires when
that mechanism is narrowed, and nothing links the two but a note** — so the note is now at the
exclusion.

### §10.7 Round 8 — two findings; the self-test entry moved off the environment

| # | What | Disposition |
|---|---|---|
| **P2** | **Self-test mode was reachable from a parent shell.** A shell that exports `WEBREF_WIRE_SELFTEST` and `WEBREF_WIRE_SELFTEST_PPID=$$` is the parent of every wire it later launches, so the PID check passed and an ordinary run skipped every control and scanned the fixture. Reproduced here in a `git clone --local` sandbox with a violation planted in `_webref/`: the wire at `0ba1ed2c` printed `PASSED` over the fixture; the fixed wire scanned the checkout and exited 1 | **Fixed at the mechanism, not the value.** This was the third authorization this entry had carried — a literal token (R1's fix), then the parent's PID (R3's fix), now broken the same way — and the root is that the **environment is inherited by definition**, so no value in it separates "the controls started this" from "a shell with the export started this". Self-test mode is now `--selftest <root> [dir] [extra]`: arguments are not inherited and the driver passes none, and the old names are no longer read at all. The control is R8's reproduction verbatim (both names exported, the companion equal to the real parent, pointing at the clean fixture, while the argument names a violating one); a second control pins the missing-root refusal. One mutation record each |
| **P2** | The umbrella's A-i-wire row **copied** this memo's artifact list, slot verdict and blind-spot count, and all three had moved on | **Fixed by removing the copy**: the row now points at §4 and §8 instead of restating them. The same sweep found no other copy — the A-i memo's §8 passage states the blind-spot/slot argument, which still holds, and its §0 record of what A-i once carried is history |

⚠ **Neither finding came from R7's fixes**, which is the stop condition set for this round. Both
are older self-introduced defects: the PID check was R3's fix, and the row went stale when R5/R6 changed what it copied.


### §10.8 Round 9 — five findings on one head, and the loop stops here

Five threads on `0c0187b4`: three from a review Codex ran on the R8 push by itself (02:34Z), and
two from the R9 trigger. ⚠ **The first three sat unread for six hours**: the landing probe
counted only items newer than the R9 trigger, so a review that arrived before it was invisible
to it. Found by the full thread fetch, which is unscoped.

| # | What | Disposition |
|---|---|---|
| **P2** | §9 still dismissed an optimisation with *"a gate that runs in single-digit seconds"* — the figure R7 retired from `CLAUDE.md`, surviving in this memo | **Removed**, with every other figure in that table (a speed-up factor, a percentage, a duration). The ground each row gives no longer rests on a number |
| **P2** | §9 called two declined items *"the right change … recorded as a follow-up"* and *"recorded so the third copy does not have to rediscover it"* — obligations by their wording, absent from §8 | **Rejected as obligations, and now worded as such**: both rows say *not owed, so not a slot*. Nobody is committed to either; the gate is correct without them |
| **P2** | Under a **relative** `TMPDIR`, GNU `mktemp -d` returns a relative path, the trap's `/*/*` guard matched nothing, and every run left its scratch (a normal run: the whole fixture tree) behind | **Fixed by construction**: the scratch path is resolved to its physical absolute form before the trap is installed. macOS's `mktemp` ignores a relative `TMPDIR`, so the control uses a shim that answers the way GNU does and checks what the run leaves behind. Record + control |
| **P2** | *"Run the 40 controls"* here and *"41 invocations"* in the wire — both stale | **Removed**, and the same sweep found a third (*"the other thirty-nine"*) and a fourth (*"all 18 entries below"*) in the controls and mutation files |
| **P2** | A real tool's path was spliced **unquoted** into every generated shim, so a git or grep under a path with a space made the shims invalid | **Fixed at one helper**: `_shq` single-quotes every path written into shim source (every shim, the `mktemp` one included). The helper is the harness's own part, not an arm of the wire, so it gets no mutation record (the set edits the wire): it is asserted instead, by round-tripping a path holding a space, a quote, `$` and a backtick through `/bin/sh`, and the run refuses to continue if that fails. A first version added a `_control` plus a record aimed at the helper; the record could never match, because the mutation set edits the wire, and the run said so |

⚠ **The stop condition set before R8 fired.** Finding 1 is a sweep miss of R7's own fix, which
makes four consecutive rounds whose findings include one of this loop's own fixes. Per that
condition the Codex loop is **not re-triggered**. What stands in for its remaining rounds is an
enumeration by a fresh agent over the populations these findings came from — figures and
universals in prose, every shim and what it wraps, every status site, every boundary rule in
both directions, and what the fixtures inherit from the caller's environment — with anything
it finds either fixed or listed for the merge decision.

## §11 Design revision — after the enumeration attestation (rev 3, plan-review required before implementation)

### §11.0 How this section got here

The Codex loop stopped at R9 (§10.8). A fresh agent then enumerated the populations defined in §11.4
over a frozen snapshot (`d5dcad1c`) and found the CRIT and IMP items disposed of below, plus the MIN
items listed in §11.4 — nine rounds of external review had been sampling that population one member
at a time.

* **Rev 1** (`470d7fcc`) re-executed this wire under `env -i` and an allowlist. Its plan-review
  measured why that cannot work: bash consumes `BASH_ENV`, `SHELLOPTS` and `BASH_FUNC_*` at startup,
  before a script's first line (`SHELLOPTS=noexec` and an exported `exec` both stopped the re-exec); an allowlisted `GIT_CONFIG*` carries any
  git config key (`core.fsmonitor` ran a command inside this wire's own `ls-files` call); and the
  class reaches the sibling wires too. Rev 1 also proposed *declaring* D3 — the fail-safe error §10.4
  records.
* **Rev 2** (`c94e1744`) carved the whole environment class to a new umbrella slice (user decision,
  2026-09-23). Its plan-review found the premise of that carve false for D1 — `grep` reads its
  environment on every exec, so a wire can close `GREP_OPTIONS` for its own calls — and found the
  carve's ledger wrong (most carved items were this PR's own defects) and the new row at odds with
  the umbrella's forced ordering. It also measured holes in every point fix (folded into §11.2).
* **Rev 3** (this; user decision, 2026-09-23): sort the environment findings **by the direction they
  fail**, close the silent ones here, and carve only what is pre-existing and closable nowhere but
  where bash is started.

### §11.1 The environment findings, by failure direction

The wire's rule for modes reachable from outside (beside `_SELFTEST`) is *unreachable or loud,
never silent*; this section applies the same rule to inputs. It, not a list of variable names,
decides each row.

| input | fails | disposition |
|---|---|---|
| `GREP_OPTIONS` (D1) | **silent** — BSD grep honours it; an exclusion makes a named file read as "no match", so a local run goes green over a violation (CI's grep is GNU, which does not honour it; not measured here) | **closed in #519**: `unset GREP_OPTIONS` at the wire's start. Not a denylist of a guessed population: BSD grep's manual (`man grep`, ENVIRONMENT) names one variable, `GREP_OPTIONS`; locale is already pinned by `LC_ALL=C` |
| `POSIXLY_CORRECT`, `BASHOPTS=inherit_errexit` (cause of D4) | **silent** — the `_scan` subshell dies mid-walk and the verdict is taken over what it had emitted | **closed in #519 by construction** (D4, §11.2): completion is asserted, whatever killed the walk |
| `GIT_TEMPLATE_DIR` (D6) | loud — controls report NOT EXERCISED and the run is red | stays loud; the claim that `_fgit` "neutralises both config layers" is deleted |
| `GIT_TRACE*` / trace2 config (D7) | loud — red, with a misleading "walk reported errors" | stays loud; noted where the stderr rule is |
| `CDPATH` (D8) | loud — exit 1 with a `cd` error | stays loud (the exit code is the violation code, not "decided nothing" — noted, not changed) |
| a restrictive `umask` (half of D5) | loud once D5 makes the guard reachable (exit 2 with a message) | D5, §11.2 |
| `XDG_CONFIG_HOME` / `SUDO_UID` (`safe.directory`) | loud — a refusal (exit 2) | stays loud |
| `PATH` (D12) | **either direction** — the wire names its external tools and does not resolve them, so `PATH` decides which binary each name reaches, and a substituted one can answer wrongly without saying so | **bounded in practice by the controls, not by the wire**: they re-invoke this wire with the caller's `PATH` over fixtures whose answers are known, so a substituted tool that changes an answer a fixture pins reds the gate before the real scan. The residual is closable only where bash is started — slot `#11-trip-wire-launch-environment`. Not closable in the wire: absolute paths would be a second, hand-kept inventory of the same kind this file keeps retiring |
| `BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*`, `core.fsmonitor` via `GIT_CONFIG*` | whatever the person running the gate makes it do | out of this wire's reach (startup) or an environment subverting its own gate; **declared** where bash is started — slot `#11-trip-wire-launch-environment` |

**Own vs pre-existing**, per the defer policy: the rows this wire itself decides are closed here
when they fail silently and accepted under the wire's own rule when they fail loudly — not
deferred. ⚠ **Two rows are not the wire's to reach at all**, and an earlier wording of this
sentence said every row was: the startup row (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*`, read
before this file's first line runs) and the residual half of `PATH`, which the controls bound
in practice and which closes only where bash is started — and both are in the carve the next
sentence names. What **is** carved is pre-existing on `main` and not this PR's to fix: the sibling wires'
ambient `grep`, the driver's environment-entered `TRIP_WIRES_SELFTEST` (D9, from #496), and the
startup declaration, `PATH` included — one slot, `#11-trip-wire-launch-environment`, registered in the defer ledger
with an owner route, trigger and date. It does not count against this PR's own-deferral cap.

### §11.2 Point fixes in #519

| # | defect | change | control | record |
|---|---|---|---|---|
| D4 | `_verdict "$(_scan …)"` discards `_scan`'s status; a subshell killed mid-walk yields a verdict over what it emitted | `_scan` emits a terminal record after its last source **and** on its early `return` (the `no temp file` arm); `_verdict` refuses a stream without exactly one, in last position, **before** the `SCANNED -eq 0` guard | fixture: the `catfail` shim plus a tracked entry that sorts **before** the killed one (e.g. `a.py`), under an injected `POSIXLY_CORRECT=1` — pre-fix this is exit 0 PASSED, post-fix exit 2 "did not complete" (pre-fix status measured: 0) | removing the check |
| D2 | a tracked file under a directory with read but not search permission: `[ -L/-f/-e ]` all fail with EACCES, the path reads as tracked-and-gone, no record, green | "absent" needs a positive test: walk up to the **nearest existing ancestor**; it must be a searchable directory and the next component must be absent from it. A tracked path that is neither a file, a link nor provably absent is an `err`. This keeps a tracked directory deleted wholesale (a routine unstaged state) green | red: tracked file, dirty worktree, parent mode 0444, no untracked sibling (a geometry measured to discriminate; adding an untracked sibling or a violation in the index blob reds the wire without D2). Green: a tracked file whose directory was removed. Reported NOT EXERCISED where permissions are not enforced (the `$_perm_line` pattern) | removing the searchability test |
| D3 | a `]`, a quote or a backtick inside a segment ends it, so `.claude/tools/team]inc/rule.md` is missed | **widen, don't declare**: a segment **followed by `/`** is bounded only by `/` and whitespace; only the segment that **ends the match** keeps running-text terminators. `K2: 0` on the real tree is measured **at implementation**, for this class (rev 2's narrower widening measured 0; this one is wider) | red: each of `]` `"` `'` and backtick, both as the first segment and as a later intermediate one. Green: every existing green control. **New boundary this opens** — a quoted one-segment reference followed directly by a `/`-bearing token (`".claude/tools/webref","/tmp"`) now matches: it fails safe (red); it gets a control and a line in the header's list of what the heuristic half decides | one per widened character |
| D5 | `x=$(…)` under `set -e` exits before the guard that follows, at **both** sites (`SCRATCH`, `_root_p`) | `x=$(…) || x=""` at both | one per site: a restrictive umask for `SCRATCH`; a `--selftest` root that cannot be resolved for `_root_p`. `_control` has no umask channel, so the first is a block of its own (the `relcwd` pattern) | one per site |
| D10 | boundary rules with a control in one direction only | one control per missing direction, **re-derived after D3** (D3 edits the same classes) by P4, not taken from the pre-D3 list | as derived | one each |
| MIN | false current-state claims (§11.4) | **deleted by default**; corrected only where deleting would leave a reader believing something false, and never with a new count or list — a figure that matters is a command | — | — |

**The PR body** names D1's former local false green, its fix, and the carved slot, when #519's head
is next updated.

### §11.3 Coupled invariants

* **D4 terminal record × the sentinel arms** — a sentinel `err` (from `cat`/`readlink`) is a record,
  not the end; the terminal record comes only after every source or at the early return.
* **D4 refusal × the `SCANNED -eq 0` guard** — the refusal runs first, or a walk killed before its
  first record reports "read 0" instead of "did not complete".
* **D4 terminal record × the `forge` fixture** — a path cannot forge the terminal tag (`_esc`/`_onerec`
  map LF; measured); the `forge` fixture is extended to the new tag.
* **D2 positive absence × the untracked vanished-path arms** (`phantom`, `globspec` expect "gone") —
  D2 applies to tracked paths; an untracked path under a 0444 directory already reds (git warns), but
  its message still claims "gone" — corrected to not assert a negative the wire did not establish.
* **D3 widening × D10 × the `$K2RE` mutation needles** — D10's population is re-derived after D3;
  every existing needle must still apply to the widened text — measured at implementation (rev 2's
  narrower widening orphaned none).
* **New controls × the harness budget** — `ci.yml`'s `timeout-minutes` line is re-derived in the same
  PR, as that line requires.
* **New controls × the mutation ratchet** — every new control ships with its record.
* **New controls × the controls file's size** — if this PR takes it past CLAUDE.md's threshold,
  the touch-time split is its own commit first, at a seam named then. It did; §11.7 row 7 records
  the seam. Whether it is past the threshold again is `wc -l` on the file, not a figure here.
* **D5 × trap order** — the trap is installed after `SCRATCH`'s assignment and before its guard; an
  empty value is a no-op in the trap's `/*/*` case, and the guard removes the raw directory itself.

### §11.4 The instrument, recorded so it can be re-run

A fresh agent, on a frozen snapshot, enumerates **every** member of each population — one line per
member, each backed by the command that produced it, marked OK or DEFECT:

* **P1 — quantities and universals in prose**: the comments of this wire's `.sh` files — the wire,
  its controls, the control harness and the mutation set — and of
  `scripts/trip-wires.sh`; this memo's header and §0–§11 (§10 as provenance); every umbrella row and
  paragraph naming this slice; the `CLAUDE.md` and `ci.yml` paragraphs. Every figure describing current state,
  and every "every / all / only / no / none / never / the one place" with its complement measured.
  Enumerated with an unfiltered word grep; context read separately.
* **P2 — shims**: every generated script — what it intercepts, whether its pattern is verb and flag
  as one sequence, whether every embedded path goes through `_shq`, whether the wire makes that call.
* **P3 — status sites and verdict sites**: every consumed exit status — what non-zero is read as,
  whether a positive check establishes it — reconciled against the header's audit table in both
  directions; and every verdict the wire can emit, with the control that reaches it.
* **P4 — boundary rules**: every rule in `$K2RE`, `$K2RE_PATH` and the walk, with its red and its
  green control; a missing direction is proven by a surviving mutant.
* **P5 — caller environment**: every variable or ambient input reaching the scan or the fixtures,
  classified by failure direction as in §11.1.
* **P6 — mode entries**: every way into a non-default mode from outside; each unreachable from an
  ordinary driver run, or loud.

The attestation at `d5dcad1c` found these MIN items (to be deleted or corrected per §11.2's MIN row):
wire — "every boundary rule carries a control in both directions"; "never answers green over
something it did not read"; the status audit table's omissions (`cat`, the `[ -L/-f/-e ]` tests,
`_phys`/`cd`, `: >`, the discarded `$(_scan)`) and overclaims (`ls-files --error-unmatch`'s positive
test; `tr|cmp`'s NUL message on a plain failure); "the whole list … the one place"; "none of these is
a deferred obligation" vs item 7; "every line entering this tree passes review"; item 6's character
list; "two things no control pins"; "every git call goes through `_git`"; "`--opt=<path>` is how
`cli.py` spells its examples"; "two of those shapes are live"; "closing it cannot open anything";
grep stderr "fails the run"; "every other stored value avoids `$( )`"; "an unborn HEAD — every
fixture here"; "`$PIN`"; "`_scan` returns 0 unconditionally and reports every failure"; "five named
modules". Controls — "prove the wire can reach every answer" (the gap itself: slot
`#11-k2-wire-verdict-site-controls`); "`_fgit` neutralises both config layers"; "every shim writes a
real tool's path"; "one call = verb and flag"; "every other red fixture writes its path after a
space or a quote"; "each line below is a spelling live in the scanned tree"; "one name is absent …
cachedir"; "the three ways". Mutations — "`$_CONTROLS` … from the controls file"; "writes nothing
of the caller's"; "every needle must name an actual `_control` label" (a quoted-string grep); deleting
a paired or `!survive` record is silent (a code defect, not prose: the correspondence check gains the
`!survive` record and counts records per needle). Memo §0–§9 and header — the line counts and the
scanner-vs-largest-wire comparison; "carried both files past 1000"; "the list is now five"; the
superseded `check-ignore` precondition; the consumes list; "not taken" for the controls split;
"invoked as `bash "$_CONTROLS"`"; §6's retired tool enumeration; "nothing in §5–§7 is open"; "one
predicate" (at every site: `grep -n 'one predicate'`); §8 slot 2 "monotone-downward" (the slot's
trigger rests on it and is re-derived, not reworded).

**§8's cohesion judgement is re-derived, not patched**: it rested on "one predicate, one walk, one
verdict" and a line count. There are two predicates (`$K2RE`, `$K2RE_PATH`) over one walk into one
verdict; the seam a split would use is still predicate-vs-walk, and the wire stays one file on that
ground — the counts are dropped.

### §11.5 Terminator, and what happens if it is not met

plan-review of this revision → implement → P1–P6 by a fresh agent on the implementing head. **Met**
when no CRIT/IMP is found that the wire or its controls can close; a finding closable only where bash
is started goes to `#11-trip-wire-launch-environment`, and nothing else is routed away. **Not met** →
fix once, then re-run the touched populations on the fixed head. A finding that traces to this
revision's own edits (its diff touches the line the finding cites) stops the loop and is reported
rather than fixed in a second round. MIN findings from the re-run are deleted per §11.2 or listed in
the PR body. Then the merge decision, with the land order against #501, goes to the user.

### §11.6 Rev 3's review — three focused passes, and where they leave the design

Rev 3 was reviewed with a changed instrument, because both earlier plan-review rounds had been reset
by text the previous revision itself introduced: a disposal check of rev 2's findings, an audit of
every claim rev 3 newly wrote, and a prototype of §11.2 in a `git clone --local`, run on bash 5.3 and
3.2 with the mutation set. **No CRIT.** Every mechanism worked as specified; the real tree stays
`K2: 0` under the widened class; each new red control fails on the pre-fix wire. What remains is
implementation detail, most of it caught by the mutation harness itself — which is where this plan
review stops and the implementation's own gates take over.

| from | finding | disposition |
|---|---|---|
| disposal | `core.fsmonitor` (via the deliberately kept `GIT_CONFIG*`) was routed to the launch slot, but it reaches this PR's own `ls-files` calls, and a globally enabled fsmonitor changes which files git reports | **own, closed in #519**: `_git` passes `-c core.fsmonitor=false -c core.untrackedCache=false` on every call; control: an injected fsmonitor hook must not run |
| disposal, audit | §8 still counted two own deferrals | §8 lists `#11-k2-wire-verdict-site-controls` and reads **3 of 3** |
| audit | the ledger routed `POSIXLY_CORRECT`/`BASHOPTS` to the launch slot as "closable only where bash starts" — a script can undo both, and D4 closes their effect here | dropped from the slot; D4 is their owner |
| audit | "the wire's rule … a mode **or input**" — the wire's sentence covers modes | §11.1 says the rule is the wire's rule for modes, applied here to inputs |
| audit | P1 said "both umbrella rows" — more rows name this slice | P1 reads "every umbrella row and paragraph naming this slice" |
| audit | §11.0 counted `POSIXLY_CORRECT`/`BASHOPTS` among what defeats a re-exec; what was measured to defeat it was `SHELLOPTS=noexec` and an exported `exec` | worded to what was measured |
| prototype | D3 re-aims two existing records: one's sed anchor no longer matches the widened assignment; one now dies for the wrong reason (the widened first segment admits `"`, so `quotename` no longer separates the two predicates) | both re-aimed in the implementing commit; the mutation set's own "matched nothing" / "wrong reason" checks are what found them |
| prototype | D3 as worded leaves `…/a/]x/` (a trailing slash) undetermined between "intermediate" and "final" | the fail-safe reading: an alternation — a segment followed by `/`, **or** the old final-segment class — so it matches |
| prototype | D3's later-segment red controls only discriminate with the character **leading** that segment (`a/]inc/…`; `a/team]inc/…` already reds pre-fix) | that geometry |
| prototype | D3 listed controls per character but records per class | one record per control (the ratchet does not move) |
| prototype | D1 had no control and no record; deleting `unset GREP_OPTIONS` survived | control: `_ctl_env` `GREP_OPTIONS=--exclude=*` on a violating fixture (discriminates on BSD grep; on GNU grep it passes trivially — stated at the control); one record |
| prototype | the D5 umask block and mode-000 root cannot fail where permissions are not enforced (root) | both inside the `$_perm_line` gate, NOT EXERCISED otherwise |
| prototype | D2's nearest existing ancestor being a regular file (a tracked directory replaced by a file) is ENOTDIR — a positive absence — but the rule made it an `err` | a non-directory nearest ancestor is absence |
| prototype | D4's "remove the check" record contained the tag's TAB, which splits the record | the record edits the check without the TAB |
| prototype | D4's early-return terminal record has no discriminating control | accepted: both paths exit 2 and differ only in message; stated at the arm |

### §11.7 What the implementation decided where §11 did not say

Implemented over `996ec177`. The dispositions above are the specification; these are the points
it left open, answered in the code and recorded here so the attestation §11.5 calls for reads a
position rather than a silence.

| # | Left open | Decided |
|---|---|---|
| 1 | `_absent`'s remaining shapes: a nearest existing ancestor that is a dangling symlink, and a walk that runs off the top of `$ROOT` | Absence is claimed for a searchable directory, for a non-directory (ENOTDIR) and for a dangling symlink — measured: `_absent` returns absence in that last case, because a path under an unresolvable link is unreachable in the worktree and its content is still read by the index and HEAD passes. A walk that exhausts its components returns "not established", i.e. an `err`: unknown lands on red. ⚠ An earlier wording of this row said the dangling case errs, which the code does not do — the code is the one that is right here |
| 2 | how the two re-aimed records are re-aimed | `quotename`'s mutant now excludes `"` from `$K2RE_PATH`'s segments, which is the claim its label makes; the old `K2RE_PATH="$K2RE"` mutant is kept and re-needled to `stagedlink`, whose staged target holds a space — the shape that still separates the two predicates after D3. `punctslash`'s mutant now adds the closing-punctuation exclusion to the FIRST segment, which is the over-reach it was written against |
| 3 | how the fsmonitor control proves anything where a hook cannot run | It is not a `_control` (the question is whether a command RAN). The hook is first shown to run under a plain `git ls-files` over the same fixture; if it does not, the block reports **CONTROL NOT EXERCISED** and reds, rather than passing because nothing happened |
| 4 | `-c core.untrackedCache=false` has no control | Stated at the site and in the wire's list of what no control pins. It is caller state of the same kind as `core.fsmonitor`, and turning it off costs nothing this gate needs |
| 5 | what "counts records per needle" means for the correspondence check | Two checks rather than a per-needle table, which would be a hand-kept list of the kind this file keeps retiring: the `!survive` record must be present exactly once, and the record count has a floor (`_MUT_RECORDS_MIN`) that an edit must lower deliberately. ⚠ What neither sees is a deletion and an addition in the same edit; the added record still has to kill |
| 6 | D10's population after D3 | ⚠ **The answer written here was that *each* rule of `$K2RE` and `$K2RE_PATH` had been mutated against the control set, and it was false.** The enumeration was by hand, and successive attestations each declared it complete and each missed members of it — the leading `/`, the class's `A-Z` and a one-character final segment first; then, on the head that closed those, the stored first segment's minimum length (under which `.claude/skills/a/rule.md`, an entry whose own NAME is the forbidden hierarchy, reads K2 zero on a required gate) and members of the final segment's middle class, which the existing green lines could not see because they end in `)`. What now stands in place of the claim is a **mechanism**: `_mut_gen_run` in `…trip-wire.mutations.sh` **derives** the population from the wire's own two assignment lines — one mutant per bracket-expression member, one per quantifier — splices each back over that line, and requires every one to red the control set or to be named in the **equivalence table `_mut_equivalent`, in that same file**, beside the argument why it cannot change a verdict. A generated mutant that is neither fails the run; so does an assignment it cannot read back, and so does an equivalence entry naming a mutant the generator no longer produces. It runs with the hand-written set, under the same opt-in. It found rules neither attestation had named, and the one equivalence argument that had been offered is refuted at that table. The hand records still say **which** control catches a rule; the generator says **that** no rule is unwatched, which is the half a list cannot do |
| 7 | the controls file's size | It would have passed the threshold, so the touch-time split landed first, as its own commit: the control **harness** (how a control runs) out of the control **catalogue** (which controls exist). §4 and §8 carry it |

⚠ **What this implementation did NOT do**: the PR body (§11.2's last line) and the defer-ledger
registration of `#11-trip-wire-launch-environment` (§11.1) are not in this diff — they are
written where the PR and the ledger live.
