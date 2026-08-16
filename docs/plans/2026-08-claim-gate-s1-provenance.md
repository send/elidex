# S1 — the `claim` annotation grammar + the re-execution wire

> ## State: answering plan-review round 6
>
> **Numbered by round, not by revision.** "Revision N" had no definition anywhere and the front block
> carried one number while the body's back-references carried another; rounds are counted by something
> outside this file. Body references to earlier revisions stay as they are — they name historical
> states, which is what they are for.
>
> **This block is a pointer, not a second record.** Every withdrawal, correction and bound below is
> stated at the site that owns it, and a front-door summary of them was a duplicated decision surface:
> two places a reader could learn what this memo currently holds, with no rule saying which wins. What
> a round found lives in the section it changed; what it *taught* lives in memory. This block carries
> only what has no other site: the memo's own status and the commands that establish it.
>
> **This memo states no quantity about its own tooling**, and `claim-gate-plan-check.py`'s `TOOLING`
> check is what holds it to that. The artifact is these two commands (D10 form 2), re-run against this
> file at every round:
>
> ```
> $ python3 .claude/tools/claim-gate-plan-check.py --self-test | tail -1
> $ python3 .claude/tools/claim-gate-plan-check.py \
>     docs/plans/2026-08-claim-gate-s1-provenance.md \
>     docs/plans/2026-08-quantitative-claim-staleness-gate.md
> ```
>
> **Rounds 1 and 2 have no row**: their findings were closed by rewrites that left no site of their own.
> The body cites *revisions*, which are drafts and not rounds, and the two do not run in step — round 4
> produced revisions 5 **and** 6 — so a revision number in the body is a pointer into `git log`, not into
> this table.
>
> | Round | What it changed, and where that change is recorded |
> |---|---|
> | 3 | The checker leaves the slice into a prerequisite branch (§0, §5, §9). Q3 closes by measurement: no fence arm (§4.2, §7). |
> | 4 | The fence withdrawal reaches the umbrella as an amendment (umbrella D2/D3/§6). `--one` is withdrawn for a clone fixture (§4.1.1). `TOOLING`, `UMBRELLA`, `FIXTURE` replace three prose rules. |
> | 5 | Rule 1 was refuted by running it under the declared regime (§4.2). Six controls asserted states no stage produced (§8, the self-test). Six citations named the wrong line (§4.6). |
> | 6 | Rules 1 and 4 could not both be implemented, and rule 3's liveness spelling broke I7 (§4.2, §2). §2 was a stale second copy of §8's `$3` model (§2 I6). A third altered quotation, load-bearing (§4.1.1(b), §8). D4 amended in the umbrella. This block collapsed to a pointer. |
>
> → memory `feedback_prose-rules-cannot-fix-unexecuted-claims`,
> `feedback_findings-cluster-in-self-added-scope`,
> `feedback_convention-dependent-figures-are-argument`, `project_stale-claim-gate-v1-review`.


Slice **S1** of the umbrella `docs/plans/2026-08-quantitative-claim-staleness-gate.md`. That file owns
the problem statement, the ten binding decisions (§1 D1–D10) and the measurements (§2). This memo does
not reopen them and does not re-derive their numbers.

**Provenance discipline** (umbrella §1 D10, and the reason v1 and the first two S1 drafts all failed
review): every number, `file:line`, §-reference and statement about tool behaviour below came from a
command run in this worktree and pasted next to it. A figure this memo's own landing would invalidate is
replaced by a **property** (D10 form 2) or by a live annotation the wire re-executes (form 1); counts
that survive as counts are stated against a base **including this file** (form 3). Convention-dependent
quantities — whose value depends on how one chose to count rather than on what `wc -l` / `grep -c` /
`git` report — are **removed**, not re-measured.

**That paragraph is not the mechanism, and round 2 is the proof.** It was already present, in those
words, when a claim read off a source comment landed directly beneath it and carried §6's bound and all
of §7 Q3. A contract stating that commands were run is written by an author who believes they were.
The mechanism is `claim-gate-plan-check.py`: pasted commands are re-executed and their pasted answers
compared, tables are checked for exhaustiveness and for the properties their own preambles assert, and
a fact about how two programs behave is measured by running them rather than by reading either one.

**Where a claim below cannot be put under that checker, the rule this paragraph stated was false of this
file at every revision that stated it.** It said *removed, not restated more carefully*, while the
checker's own report listed pasted commands it declined to re-run — among them the `set -euo pipefail`
pair that overturned the wire's rule 1 (§4.2–§4.2) after five rounds. The rule as written prescribed deleting this memo's
most valuable measurement.

Round 6 first tried the disposition the rule omits — **extend the mechanism** — by admitting a `bash -c`
whose script "only reads", on the ground that the checker was refusing commands by the *name they start
with* while the reason to refuse `git clone` was the side effect. **That was reverted, and the reversal
is the more useful record.** Measured: the pipefail pastes span two physical lines, and the form that
joins continuations joins only on a trailing backslash, so the change admitted neither of them — zero
coverage for its entire stated motivation — while a review that attacked the classifier found
`printf %s $(rm -rf $(echo X))`, `cat <(touch X)`, `grep x & touch X` and `git log --output=X` all
admitted and executed. An allow-list describing *what a shell script does* is the wrong instrument, and
its leaking is the argument. What the attempt did buy is the discovery that the hole was never confined
to it: `sed -i`, an `awk` program calling `system()`, `git grep -O<cmd>` and anything after `&` or `;`
had always been executed, unquoted, from a document. Those are refused now by a deny-list of the shapes
a reader-*named* program uses to write.

So the honest rule is narrower than either version: a claim the checker cannot hold is **removed unless
the mechanism can be extended to hold it *safely***, and extending a gate that executes what it admits
is a change whose failure mode is not a missed finding. The residue is a **bound, not a silence** — the
checker names every command it declined and why, so it is read off the run. **No count is given here.**
The first draft of this paragraph gave one, and it was wrong within the same round and wrong again after
the revert: a count of the un-run set is a quantity about this memo's own tooling that any later paste
falsifies, which is the class D10 form 2 exists for, and `TOOLING` does not catch it because the
sentence names "the checker" rather than the file. The kinds are stable where the numbers are not:
programs the checker does not run, commands needing shell state the paste omits, shapes refused as
side-effecting, and fixture blocks whose subject is not this tree.

Measurement base, stated as a **property** rather than a hash: *the working tree at this branch's head,
with this memo `git add`ed*. It was a hash — `ef18b828` — from revision 3 until round 5 measured it:
`git merge-base --is-ancestor ef18b828 HEAD` is false and `git branch -a --contains ef18b828` is empty.
The prerequisite branch was rewritten underneath it, so the base every D10 form-3 count anchors to had
become a commit no reader could check out. A hash pinning a branch that is still being rewritten is not
a base; it is a claim with the same lifetime as the branch, which is the failure this memo is about.
`git version 2.55.0`, `GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)` — the wire runs under
`bash`, and §4.2's argv rule is a `bash` fact, not a `zsh` one.

---

## §0 Decision + scope

**Decision.** S1 ships one registered trip-wire that re-executes every `claim` annotation in the tracked
tree on every push, the complete grammar it executes, and a self-test carrying a positive and a negative
control for every clause of that grammar. Three verbs — `count`, `files`, `lines` — each with its own
production; D2–D6 become production rules, not prose; registration in `REQUIRED_WIRES`
(`scripts/trip-wires.sh:85-90`) lands in the same commit as the wire.

**In scope** — the list is §5's six commits, read off them rather than recalled, because the previous
form stopped at commit 4 and stayed there through the revisions that added 5 and 6: the grammar; the
three recogniser tiers, defined for every scanned surface; the three verbs; the wire; its invocation of
its own self-test; registration; the two prose sites carrying a stale runtime number; the set-level
label the new wire joins; the two set-level sites §4.6 found that a fifth wire falsifies; the
retirement of the checker's second transcription (below, §5 commit 5); **and the five false numerals in
another lane's landed plan-memo** (§5 commit 6).

That last one is cross-lane scope and is stated here rather than left to §5, because §0 is where a
reader learns the blast radius. It is in this PR for the reason §9 gives — this memo *proves* those
rows false, so shipping the proof while leaving the rows is the hidden deferral §9's own standard
forbids — and it is bounded to replacing five numerals against the base that table declares, touching
no prose and no decision of the lane that owns it.

**Not in scope, and not in this PR: authoring `.claude/tools/claim-gate-plan-check.py`.** It checks
*this memo*; the wire checks *the tree*. Different subject, different lifecycle, different gate — and
revision 3 bundled them, and round 3 charged most of its CRITs to that bundling. It is a **prerequisite
PR** that lands first (`claim-gate-plan-check`); S1 is stacked on it. S1 makes a single edit to that
file, and it is the retirement below: S1's own grammar leaving a file that was only ever holding it on
S1's behalf.

Revision 3 claimed the checker's annotation executor was a self-retiring bootstrap: *"a code path, not
a promise, so the two implementations cannot coexist."* **That was false and is withdrawn.** The
`WIRE.exists()` guard covered one function; the self-test called the classifier and executor directly,
so the second transcription of §4.2/§4.2 survived the wire's arrival — and the checker's `PASTE`
verification silently switched off at the same moment, because it consumes the payload that function
stops returning.

The honest replacement is a **bound, recorded in §6**, not a better-timed retirement: **the checker
carries a second transcription of §4.2's tiers and §4.2's verbs, and will keep carrying it until the
wire exists and delegation to it is implemented *and proven by a control*.** Writing the retirement
before the wire is what produced the false claim — the code path existed, and nothing exercised it.
A probe that plants a wire and asserts the checker's own grammar is unreachable is what closes it.

**Who executes that retirement was routed nowhere, and round 5 returned it.** Revision 6 wrote *"S1's
landing is the trigger; the prerequisite PR owns the work"* — an owner that merges **before** its own
trigger fires. On the day the wire exists there is no such PR left to delete `claim_gate_grammar.py`,
whose own docstring says it *"exists to be deleted"*, or to write the closing control. The only PR that
can hold it is the one that creates the wire, because the delegation is unwritable until the wire is
tracked. So it is **§5's commit 5**, not a deferral. That does not reopen the bundling round 3 charged:
S1 still does not author the checker; it removes the copy of S1's own grammar the checker was carrying
on S1's behalf, in S1's PR, at the first commit where doing so is possible.

**Out of scope, by the umbrella's own slicing**: the tamper ledger (D8 → S2), the superlative vocabulary
and `grep_pass.py` Check 3 (D7/D9 → S3), and **the whole `CONTRIBUTING.md:19` sentence**, which umbrella
§6 routes to S2: it counts `:19` among the six set-level label sites and then carves it out — *"`CONTRIBUTING.md:19` also asserts that a drifted
trip-wire **allowlist** reds the PR — a mechanism S1 does not ship and S2 does, so that sentence
is S2's to touch, not S1's."* Revision 6 read that as a misrouting and split the
sentence at `:19`/`:20`, giving S1 the label half. **That is withdrawn.** The umbrella considered the
split — the word "also" is it weighing both claims in one line — and decided one sentence has one owner.
A slice cannot reverse that (D2's amendment note states the rule), and amending the umbrella to permit it
has no merit: S2 must rewrite the sentence anyway when it ships the allowlist mechanism, so S1 editing
half of it buys a two-touch sequence on one sentence and nothing else. The residue is real and is a bound
in §6: between S1 and S2, `CONTRIBUTING.md` is the one governing file still calling the set "layering
trip-wires". The forced order makes that window exactly one PR.

S1 therefore lands a gate that is opt-in and whose annotations can be deleted without a record — a bound
in §6, not a half-ledger, since "new seam plus a partial legacy" is the strangler shape CLAUDE.md *One
issue, one way* forbids.

**Why this is a terminal unit.** CLAUDE.md's edge-dense rule makes an approved umbrella's
narrowly-scoped per-PR slice the base case: one wire, one grammar, one self-test, one registration line.

---

## §1 The decisive fact: a helper is not a wire, so nothing runs it

The driver discovers what to run by glob, not by reference:

```
$ sed -n '103p' scripts/trip-wires.sh
WIRE_GLOB='.claude/tools/*-trip-wire.sh'
$ git ls-files -- '.claude/tools/*.sh' ':(exclude).claude/tools/*-trip-wire.sh'
```

The second command *is* the membership test: it lists every tracked script in that directory which the
driver's loop (`scripts/trip-wires.sh:188-205`) and its both-directions retention diff (`:207-237`) will
never see, because `WIRE_GLOB` decides membership **by name, not by directory**. It prints nothing before
S1 and prints `claim-provenance-selftest.sh` after — and that transition, not the set of names it returns
on any one day, is what §1 is about: a helper can sit beside the wires and still not be one.

The command replaces a pasted listing of the four names, and the reason is D10 rather than taste.
Revision 1 pasted them and wrote *"every tracked `*.sh` directly under `.claude/tools/` matches
`WIRE_GLOB` today"*; S1's own commit 1 adds a tracked script there that does not match, so the listing
and the sentence are both falsified by this memo's landing — the preamble's literal prohibition, in the
section whose subject is the file that breaks it. §4.6 keeps its listing stable under the same landing by
excluding what S1 adds (`':(exclude).claude/tools/**'`, stated there); that device is unavailable here,
because the excluded file *is* the argument. So the figure goes and the property stays (D10 form 2).
Consequences, both of which shape S1:

1. **A split-out self-test is not executed by anything.** The first draft specified eleven controls in
   such a file and no invocation for it. `scripts/trip-wires.sh:104-111` already names the outcome:
   *"a check nobody re-runs is a transcript, not a gate."* So the wire must invoke its self-test itself,
   under a recursion guard, in the shape `scripts/trip-wires.sh:113-184` uses — and that invocation needs
   its own control (§8 C18), because an invocation nobody proves is the same transcript one level up.
   That shape's guard is an **environment variable**, set on the command at `scripts/trip-wires.sh:139`,
   and an environment variable is inherited by everything the run spawns. So the guard can govern the
   invocation and cannot govern the *requirement*: §4.1.2 puts the helper's existence check ahead of the
   guard and gives it its own diagnostic, which is what makes both of C18's arms reachable (the self-test).
2. **A split-out helper is outside the retention diff.** `scripts/trip-wires.sh:76-84` records why the
   diff runs in *both* directions. A `test -f` guard reproduces exactly the asymmetry that amendment
   closed — it catches a helper *leaving*, not one *arriving* unreferenced. §4.1.2's existence check
   **is** that asymmetric test, and it is not offered as a fix for this consequence: it discharges
   consequence 1 by turning a missing *named* helper into an execution failure with a diagnostic, and it
   says nothing about a file that arrives unreferenced — which remains the thing nothing catches.

**Those two consequences derive a rule, not a number**, and revisions 1–6 read them as deriving the
number ("Both are closed by one choice: two files, not three"). What they establish is: *every non-wire
file must be invoked by name from the wire, and no mechanism will catch one that is not.* Consequence 1
forces the invocation — absence becomes an execution failure rather than a silently smaller gate.
Consequence 2 says the retention diff cannot help, but a file arriving unreferenced is equally invisible
at two files as at three, so it does not discriminate the count at all; read alone it argues for **one**.
Neither closes "two, not three".

**The count comes from cohesion, and only cohesion**: there is exactly one seam where splitting buys
something — the self-test must call the grammar's stages directly, not only through the wire's exit code.
Hence **two files**, the wire carrying the grammar inline and the self-test split and invoked by name.

The precedent is worth stating accurately, because S1 **departs** from it rather than following it. No
registered wire splits at all — none sources a helper:

```
$ git grep -nE '^[[:space:]]*(source|\.)[[:space:]]' -- '.claude/tools/*-trip-wire.sh'
```

Nothing today; S1's wire after it lands. That is the honest shape of the claim — S1 is the first wire
with a helper, and the paragraph above is the justification for being first. (The per-wire `wc -l`
figures revisions 1–6 cited here are gone for the same reason as the listing above: "all four" is
falsified by S1's own landing, and the four lengths by any other lane's edit to those files.)

**What none of it buys is a prediction of S1's own size**, and the trigger that replaced the withdrawn
projection was wrong twice over. It said *"split it at the seam then"* — naming a seam three lines after
this section argues there is exactly one and it is already taken — and it was a **prose rule**, the
remedy shape this program has now recorded five times as ineffective, because nothing runs it and the
author believes it was followed. Replaced by a bound the self-test asserts: **it fails if either shipped
file exceeds 800 lines.** A bound is not a figure — it survives every legitimate edit and fires exactly
when the discipline is breached — and if it fires, the answer is not "split at the seam" but *re-derive
the decomposition*, because a file over the band is evidence of a seam this analysis did not find.

**Where 800 comes from, because a bound with no provenance is a figure wearing a bound's clothes.** It
is not derived from these files and it is not a prediction of their size: revision 7 arrived at it as
the upper end of the withdrawn `~700–800` band, i.e. by arithmetic on the very projection §1 had just
deleted, which left the bound outliving its own basis. Its ground now is the repository's, not this
memo's: CLAUDE.md sets the touch-time split discipline at **1000 lines**, so a wire and a self-test that
have to stay readable by anyone auditing a gate get the same discipline with headroom. Choosing 800
rather than 1000 is a policy this memo may set for its own deliverable; choosing it because a deleted
projection happened to end there is not a reason at all.

---

## §2 Coupled invariants

Seven invariants hold simultaneously. Each is a way the wire can be green while being wrong.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Total tier assignment** (D3) — every candidate line lands in exactly one of *strict* / *malformed* / *mention*; none may vanish | The lax recogniser is the outer net and strict is a subset of it. A typo cannot fall out of the world. |
| **I2** | **Inertness** (D2) — the mention rule must never swallow a strict annotation, and the gate excludes its own files | One rule on all four surfaces: backtick it (§4.2). It is a **line predicate** — *k* backticks before the `<!--` — so no per-file state exists and no surface needs its own arm. `.claude/tools/**` is dropped by pathspec. |
| **I3** | **Liveness precondition** (D4) — `in=` must resolve to ≥1 tracked file **present in the working tree**; `lines=` to exactly 1, and that one a regular file | A `git ls-files` probe per annotation, before the measurement, using the *same* pathspec string, and the presence question asked **existentially** — first hit wins (the wire's rule 3 (§4.2–§4.2)). The working tree, not the index, because that is the tree all three verbs read. |
| **I4** | **Dialect fidelity** (D5) — the pattern body's ERE meaning is the author's meaning | A parse-time reject-list on the pattern body, with a diagnostic naming the offending escape verbatim. |
| **I5** | **Argv fidelity** (D6) — every parsed value reaches `git grep` as one argv element | No `eval`, no unquoted expansion, no reconstructing a command string. Probeable: §4.2. |
| **I6** | **Provable firing** — every grammar clause has a control that can red on its regression, and where the clause has a passing direction, one that can red on *that* | The self-test, the wire's invocation of it, and the `REQUIRED_WIRES` line, all in one commit. The `$3` discrimination requirement is **§8's**, scoped to the fixture, and is not restated here — this row said "a substring unique to it" from the first draft through the revision that withdrew that requirement in §8, which is how C23 came to assert a trailing space nothing needs. §8 also states which clauses have no passing direction, and therefore which rows are negative-only by construction rather than by omission. |
| **I7** | **Cost is bounded by the annotation count, not by the tree** | One tree-wide `git grep`, then per-annotation work. It is a *structural* bound, not a wall-clock one — see §4.5. |

**Pairwise intersections**, load-bearing first.

| Pair | Intersection |
|---|---|
| I1 × I2 | The inertness rules run *inside* tier assignment, so an over-eager one silently converts strict → mention. This is where "the gate exists but executes nothing" lives; every mention-arm control in §8 therefore pairs a wrong-count inert annotation (must stay green) with an identical non-inert sibling (must red). |
| I1 × I4 | The reject-list runs on a body already parsed as strict. A pattern containing `\d` must be **malformed with a dialect diagnostic**, not "strict, then a count mismatch" — otherwise the author reads a mismatch and edits the count. |
| I2 × I7 | Because inertness is a line predicate, the scan is **one** command: `git grep -n` prints `path:line:content`, and the tier pass is a transform of that output rather than a second pass over anything. Revisions 1–3 were two-stage *because* the fence arm needed per-file state; removing it (§4.2) removed the stage, and I7's tree term is now exactly one invocation. |
| I3 × I4 | Both are "the annotation is unfalsifiable" failures in different halves of one command, and both look like an honest zero. Measured: `git grep -cE 'fn ' -- 'crates/**/*.rss'` exits 1 with no output, exactly like a correct pathspec matching nothing. They must produce *different* diagnostics or the author fixes the wrong half. |
| I3 × I5 | The liveness probe and the measurement must receive the **same** pathspec as one argv element, or the probe validates something the measurement never runs. |
| I5 × I7 | The no-`eval` rule forbids the one shape that would batch several annotations into a single `git grep`. Serial execution is forced; §4.5 states what actually bounds it. |
| I6 × I7 | **The bound is on the wire, not on its self-test.** I7 says the wire's work is one tree-wide `git grep` plus a term per annotation. The self-test is not the wire: C10's positive runs `in="crates/**/*.rs"` and C19's corpus row runs `in="crates/shell/elidex-shell/src/**/*.rs"`, both tree-scale, and both deliberate — the acceptance corpus is only worth having because it runs against the real tree (§7 Q2). Stating I7 without this exemption makes the corpus a violation of it. |
| I6 × I3 | The fixture must be a real git repo: `git ls-files` has no `--no-index`. `git init -q . && git add -A` suffices — both `git ls-files` and `git grep` see added-but-uncommitted files, verified in §4.2's argv measurement, which ran against exactly such a fixture. |
| I1 × I3 | A malformed annotation must fail *before* the liveness probe, or a typo'd verb plus a typo'd pathspec reports the pathspec. Order: tier → dialect → liveness → measure. |
| I1 × I5 | Parsing is what produces the argv elements; a parser returning one joined string re-creates D6's hazard one level in. The parser emits each field separately. |
| I1 × I6 | Every tier needs both directions: strict-that-passes / strict-that-fails, malformed-that-fails, mention-that-is-ignored-*while-wrong*. |
| I1 × I7 | Tier assignment is the one stage touching every candidate line, and it touches **no files at all**: it is a transform of the single `git grep -n` output, so I7's tree term is that one invocation and tier assignment adds nothing to it. Revisions 1–7 read "a single pass over the candidate *files*", which described the two-stage scan revision 4 deleted — the row above records that withdrawal, one line away, and this row was not swept with it. |
| I4 × I5 | Both bound what reaches git, at different sites: I4 rejects at parse time with a visible diagnostic, I5 is invisible-by-construction in the executor. Kept separate for that reason. |
| I4 × I6 | A ban that cannot fire is indistinguishable from a clean tree — the precedent is `layout-box-reader-trip-wire.sh:276-299`'s `ban_control`. One control per rejected escape class, each with its own diagnostic substring. |
| I3 × I7 | The liveness probe is one `git ls-files` per annotation, i.e. inside I7's per-annotation term, not a second tree pass — **and so is the presence test the same rule requires**, because it stops at the first present path. Spelled as a filter over every resolved path it is a tree term wearing a per-annotation label: C10's own pathspec resolves to 1407 of 1706 tracked paths (the wire's rule 3 (§4.2–§4.2)). This is the pair that revision 7 violated while C27, I7's only control, stayed green — an existence test is not a `git` invocation, so the tally C27 reads cannot see it. **Nothing controls the stopping**, and §8 records that as an open gap rather than claiming a row for it; what the `ls-files` itself costs is priced here and was never removed. |
| I2 × I6 | D2's self-exclusion is both a *correctness precondition* and a *two-directional* claim. The self-test's own fixture annotations must be inert in the real repo — they are, by the `.claude/tools/**` exclusion, which is therefore load-bearing for S1 landing rather than a convenience. And the control must assert both directions: an annotation under `.claude/tools/` ignored, and the identical annotation outside it red. Asserting only the first would pass on a wire that scanned nothing at all. |

The remaining four pairs are independent, each for a stated reason: **I2×I3** (inertness decides
*whether* an annotation runs, liveness *whether its target exists*), **I2×I4** and **I2×I5** (the
reject-list and the executor act only on lines already classified strict), **I4×I7** (a parse-time string
test). **I5×I6** was on this list and is not independent: §4.2 makes I5 directly probeable and §8 C12
probes it.

That the table above is exhaustive is not a claim to take on trust — with seven invariants it must
account for all 21 pairs exactly once, orientation-insensitively, and `claim-gate-plan-check.py`'s
`PAIR` check derives the invariant set from this section's own `**I**n` rows and enumerates them. Two
revisions asserted this section was complete while it stood at 20, with `I6×I2` and `I2×I6` present as
separate rows and `I6×I7` — the one pair that turned out to need an *exemption* rather than a note —
missing entirely.

---

## §3 Spec coverage map

**No external spec surface.** S1 touches no WHATWG / W3C / TC39 / CSS-WG algorithm; it ships repository
gate tooling. The rows are the in-repo governing contracts S1 must satisfy — the same substitution
umbrella §3 makes and, as it records, a novel one. Every §-number below was confirmed present by
`grep -nE '^#{1,4} ' <file>`.

⚠ **The Branch column is normative, and revision 6 made it so without saying.** Revisions 1–6 opened this
section with *"This table exists to satisfy the preflight structural gate"* — true of why the table has
this shape, and false of what it now carries. §4.2 defines the three **tiers**; this column is the only
place the classifier's **branches** are enumerated, and revision 6's `TIERS` check reads each branch's
tier from this prose and decides it by running `classify()` on a specimen. That is what caught the fence
arm surviving in the checker after §4.2 deleted it — §4.2 no longer mentions fences at all, so nothing
derived from §4.2 could have caught it. A branch added here without a specimen fails; a tier changed on
one side alone fails.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2 — inertness) | classify a candidate line | (i) odd *k* backticks before the `<!--` → mention, on every surface | §4.2 tier pass, backtick arm | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2) | classify a candidate line | (ii) inside a fenced block with even *k* → **strict, and executed** (there is no fence arm) | §4.2 tier pass | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2) | scan | (iii) annotation under `.claude/tools/**` → not scanned; identical annotation outside → scanned | §4.1 scan pathspec | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D3 — three tiers) | classify a candidate line | (iv) lax-but-not-strict → MALFORMED, fail | §4.2 fall-through arm | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D4 — liveness) | resolve `in=` | (v) no tracked file → fail; `lines=` resolving to ≠1 → fail | §4.2 `git ls-files` probe | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D5 — pattern-body lint) | lint the pattern body | (vi) `\` + `[bBdDsSwW<>]` → MALFORMED, diagnostic names the escape | §4.2 reject-list | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D6 — the pathspec is always quoted) | execute the measurement | (vii) every value as one argv element, no `eval` | §4.2 executor | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 (Decision D4 — the standing gate; its "runs once then rots" hole → standing in `mise run trip-wires` ⊂ `mise run ci` and in the ungated CI job) | register the wire, invoke the self-test | (viii) present in the glob but absent from `REQUIRED_WIRES` → fail; self-test helper missing → fail | §4.1.2 + §5 + `scripts/trip-wires.sh:207-237` | ✓ | no |

**Breadth**: preflight's verdict lines for this memo are pasted verbatim in §8.3.

### §3.1 User-input touch audit

**No user-input surface**, and the touch surface below is **derived from §5's commit list rather than
described** — this section read "two bash files and one line in `scripts/trip-wires.sh`" from the
revision where that was true through four revisions in which §5 grew to six commits, and what the
description omitted was the PR's only edit crossing a tier boundary.

| File | Tier | On the ungated per-push path? |
|---|---|---|
| `.claude/tools/claim-provenance-trip-wire.sh` (new) | repo gate | **yes** |
| `.claude/tools/claim-provenance-selftest.sh` (new) | repo gate | **yes**, via the wire's preamble (§4.1.2) |
| `scripts/trip-wires.sh` — the `REQUIRED_WIRES` line | repo gate | yes |
| `.github/workflows/ci.yml`, `CLAUDE.md`, `mise.toml` | prose / config | label and comment edits only (§4.6) |
| `.claude/tools/claim-gate-plan-check.py`, and the deletion of `claim_gate_grammar.py` | **plan-review stage** | **no** — which is why §7's layer rule lets it keep its Python and its skill coupling, and why it may not move |
| `docs/plans/2026-07-app-mode-turn-completion-drain.md` | another lane's landed memo | no — five numerals, value-only (§5 commit 6, §9) |

Nothing compiles into the engine, so the exposure delta on any input path is zero. The adjacent trust
question is *repository* input: the wire evaluates annotation text drawn from the working tree, in CI,
on fork-PR content (`.github/workflows/ci.yml` triggers on `pull_request`). What executes against that
content is `git grep -E`, `git ls-files` and `awk` per annotation, plus — inside the self-test only, for
the acceptance corpus — one `git clone --local` (§4.1.1). Three structural bounds: **I4** rejects PCRE
escapes at parse time, **I5** forbids `eval` so an annotation body cannot become a shell word, and the
pattern reaches `git grep -E`, a regex engine rather than an interpreter. The residual is regex cost on
attacker-chosen input, where `timeout-minutes: 2` (`.github/workflows/ci.yml:146`) caps a hang and
nothing finer (§4.5).

---

## §4 The design

### §4.1 The scan

The scan is one tree-wide `git grep` for the lax recogniser over the D2 surface:

```
git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' \
  -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**'
```

The `:(exclude)` magic was verified to actually exclude: `git grep -lE 'strip_comments' -- '*.sh'` → 4
files; with `':(exclude).claude/tools/**'` appended → 0.

**That is the whole scan — one command, not two.** Revisions 1–3 specified a second stage, a tier pass
over the files the first named, and justified it by the fence arm's per-file state. With the fence arm
gone (§4.2) inertness is a line predicate, and `git grep -n` already prints `path:line:content`, so the
tier pass reads the content it was given. The second stage was a consequence of the rule (c) deleted,
and it goes with it — which is the *downstream* face of a withdrawal, the one a sweep of the statements
alone leaves behind (memory `feedback_sweep-obligations-not-only-statements`).

**The name goes with the stage.** Revisions 1–4 called this command "stage 1", and a numbered stage in a
one-command scan names a sibling that no longer exists — the same leftover, one level down in the prose.
Everywhere below and in §5 it is **the scan**; no argument changes, because every use of the old name was
already about this one command.

⚠ D2's exclusion governs where annotations are *looked for*, not what one may *measure*: one of §5's live
annotations targets a file under `.claude/tools/` and is executed normally.

### §4.1.1 The acceptance corpus runs against a clone, not a second entry point

The scan is the wire's only way to acquire an annotation, and round 3 found that this left the acceptance
corpus with **no execution path at all**. C19–C22 supply a known-false annotation and assert the wire
fails on it against a real tree; every placement the scan can see in *this* worktree is closed:

| Placement | Why it cannot work |
|---|---|
| a tracked file here | the annotations are false by construction, so the real gate reds on every push |
| `claim-provenance-selftest.sh` | D2's `.claude/tools/**` exclusion makes it inert — and §2 I2×I6 depends on exactly that |
| a `mktemp -d` fixture | the corpus's tree-scale pathspecs resolve to 0 there, so the D4 liveness probe fires first with a diagnostic the control does not assert |
| an untracked file here | `git grep` without `--untracked` does not see it (§6) |

Every row of that table objects to *this* worktree, not to a real tree. **So the fixture is a real
tree: `git clone --local` of this repository.** The annotation is planted in a tracked file of the
clone, the wire runs there unmodified, and the real gate never sees it.

Three properties are what this rests on, and **each is asserted by a control rather than pasted here**:
the corpus's tree-scale pathspecs resolve in the clone (C10's positive); C19's historical constant
re-measures in the clone to the value it has in this worktree, so the corpus keeps "known input, known
verdict" (§7 Q2); and — the reason this and not a second entry point — **the scan runs in the clone**,
acquiring the planted annotation the same way it acquires every other (C19–C22).

Revision 5 pasted those three as measured figures, plus the clone's wall-clock, its size and a tracked-file
count. All six are gone. The tracked-file count was already false when round 5 measured it (the
prerequisite's own modules had moved it), which is D10's literal prohibition — a tree-wide count this
program's landing changes; the wall-clock and size figures are §4.5's withdrawn class, restored two
sections after being withdrawn. And the whole block was **invisible to the checker**: `git clone` and
`git -C` are not among the programs it re-runs, and until round 5 a non-runnable arrow line was dropped
rather than reported, so four unverifiable figures sat inside a green run. That hole is closed in the
prerequisite; what it now reports is that these lines cannot be re-run — and the preamble's rule for a
claim the checker cannot hold is *remove it*, which is what this is.

**`--local` is a workstation optimisation, and in CI it is not taken.** `actions/checkout@v6` runs at the
default depth — the property §5's `count=0 pattern="fetch-depth"` annotation keeps true — so the tree the
self-test clones in CI is **shallow**, and `git clone --local` of a shallow repository is not a broken
stage: git prints `warning: source repository is shallow, ignoring --local` and `warning: --local is
ignored`, falls back to its transport path, exits 0, and produces the tracked files at the content the
source has. Measured by cloning this worktree at `--depth 1` and running `git clone --local` against the
result. The clone has carried no such qualifier since revision 5 introduced it, which prices `--local` as
the mechanism rather than as a flag that one of the two environments this wire runs in discards.

Two things follow, decided here rather than left to the implementer. The flag **stays**: where it applies
it avoids copying the object store, and dropping it would give that up for nothing CI gains, since CI
already takes the fallback. The warnings are **explained, not suppressed**: `--quiet` does not reach them
(measured on the same fixture), so suppressing them means discarding the clone's stderr — the one place a
genuine clone failure surfaces — and a fixture that could not be built has to be its own FAIL with its
own diagnostic, the discipline `scripts/trip-wires.sh:117-121` already applies to its `mktemp -d`. So the
self-test prints one line before cloning, naming the two warnings as expected on a shallow source, and
fails with a diagnostic of its own if the clone exits non-zero. That the clone is *usable* after the
fallback is not restated here: it is what C10's and C19's arms assert, on whichever path git took.

**Revision 4 specified `--one '<annotation text>'` instead, and it is withdrawn.** Four things were
wrong with it and only the last is fatal on its own:

- It asserted "it bypasses no rule". It bypasses two — D2's `:(exclude).claude/tools/**` and the
  four-suffix restriction both live in the scan's pathspec and cannot apply to a bare string.
- Its four consumers are all negative-only, so **no arm asserted it can PASS**: a `--one` that always
  failed after echoing the parsed expected value satisfied C19–C22 completely.
- §8's `live` row said the four run in one invocation while §4.1.1 said one annotation per call, and
  §4.1.1's I7 argument depended on the latter.
- It is a code path **in the shipped wire whose only caller is a test** — CLAUDE.md's *dead code は
  接続するか削除*, and a second acquisition path beside the scan, which is *One issue, one way* at the
  wire's own front door.

Cost, stated as a bound rather than a figure: **one clone per self-test invocation**, not per annotation
and not per control, reused by every `clone` control in the run. §2 I6×I7 exempts the self-test from I7
for exactly this class of expense, and the exemption is bounded by that count. No ratio against the
wire's own grep is given: §4.5 withdrew the runtime class for this memo and a ratio is two withdrawn
figures in a trench coat.

**Two rules round 5 found missing, decided here rather than deferred.**

**(a) The wire resolves its scan root from `git rev-parse --show-toplevel`, run in the invoking cwd.**
Nothing said which repository the scan reads, and the default a reader would assume is the driver's:
`scripts/trip-wires.sh:24-56` derives root from `$0` and says *"don't inherit the caller's cwd"*. That is
right **for the driver**, whose job is to find the wire *set* — it must anchor to where the wires are.
It is wrong for this wire, whose subject is *the repository it is pointed at*. Under the driver's rule
the `clone` rows would scan the wire's own tree wherever they ran, and — this is why it had to be found
by asking rather than by testing — they would **still go green**, because C19–C22 assert FAIL with a
historical constant and the real tree produces exactly that diagnostic. The corpus would never touch the
clone and nothing would say so. Two roots, two rules, each stated where its reason lives; that is not a
second entry point (the withdrawn `--one` was one, because it was a second way to *acquire an
annotation* — this is one way, pointed at a different tree).

**(b) The clone is shared for cost, not for isolation: one clone directory, one wire invocation per
control**, with the control's annotation planted and removed between invocations. §8's masking model
decides this, but not by quotation: what §8 says is *"Two controls in **separate fixtures** cannot mask
each other at all, because each invocation can only emit its own diagnostics"*. The reason clause is
about **invocations** while the subject is **fixtures**, so what §8 actually establishes is the
invocation-scoped property, and `own`'s definition spends it that way ("its own `mktemp -d` **and** its
own wire invocation"). Revisions 5–7 quoted the sentence here with *invocations* substituted for
*fixtures* — the third altered quotation this program has produced, and the one where the substitution
was the argument: a shared clone is safe **only** under the invocation-scoped reading, so the quotation
supplied the premise this paragraph exists to establish. §8 now states the invocation scoping in its own
voice, and this paragraph cites it instead of restating it. The alternative reading breaks a stated
property:
one invocation over all six `clone` rows exits non-zero by construction (C19–C22 are false by design), so
C10's and C11's **positive** arms could not be read from exit status, which is the only assertion form §8
gives a positive. Consequence, stated because it retires a requirement rather than adding one: §8's
*"Shared ⇒ the `$3` values of all `clone` rows must be mutually non-substring"* is **withdrawn** — it was
inherited verbatim from revision 4's `live` row, which was negatives-only and therefore never had a
positive to strand.

### §4.1.2 The self-test invocation, and why the guard cannot carry the requirement

The wire's preamble runs three things in this order, and the order is the whole content of C18:

1. **Resolve the scan root** — `git rev-parse --show-toplevel` in the invoking cwd, per (a) above.
2. **Require the helper, by name.** If `.claude/tools/claim-provenance-selftest.sh` is not a readable
   file under that root, the wire fails with a diagnostic containing the words **`self-test helper`** and
   the path it looked for, and exits without scanning. The diagnostic is specified rather than left to
   the shell because C18's negative arm asserts it and no stage was emitting it: handing a missing path
   to `bash` produces `No such file or directory`, which names the file and not what it was for, and
   which any other missing file in a fixture produces just as well.
3. **Invoke it, under the guard.** If the guard variable is unset, the wire runs the helper with that
   variable set on the command — the shape `scripts/trip-wires.sh:139` uses — and a non-zero status from
   the helper ends the run. The helper's own probes invoke the wire; they inherit the variable, so the
   wire they invoke skips this step instead of re-entering the self-test.

**Steps 2 and 3 are deliberately not one check, and no revision until this one specified step 2 at all.**
§1 and §8 specified the guard; C18's negative arm asserted a helper-absence diagnostic that presupposes a
check nothing described, and the guard cannot be that check. The guard is an inherited environment
variable, so inside any control step 3 is silent — a wire copied into a fixture runs no self-test,
whatever its helper is doing. A `test -f` folded into step 3 would therefore be
unreachable in exactly the situation C18 exists to test. Step 2 sits outside the guard, so a missing
helper fails identically whether the guard is set or not; proving step 3 fires needs the guard *cleared*,
which is a fixture regime rather than a code shape, and the self-test's C18 paragraph states and bounds it.

### §4.2–§4.2 The grammar, the tiers and the verbs — **now in the wire**

These three sections transcribed the recogniser tiers (D3), the pattern-body lint (D5) and the three
verbs with their four execution rules. **The transcription is withdrawn. `.claude/tools/claim-provenance-trip-wire.sh`
is the canonical site**, and `claim-provenance-selftest.sh` is what holds it to its clauses.

**Why, stated once, because it is the decision this slice ended up making.** A specification that a
program can execute, written in prose, is held true by an author who believes it is — which is the same
sentence §1 uses to withdraw a prose rule, and the reason §0 gives for building a checker instead of a
contract. Six review rounds of this memo produced clause defects that one execution exposes in seconds:
piping a measurement into `wc -l` dies under the declared `pipefail`; a failing branch that writes
before it reads `$?` leaves the fourth outcome kind unreachable; `git grep` exits 1 on a tree with no
annotation and takes the wire with it; `awk` answers **0 with status 0** on a directory. Each survived
multiple rounds of reading and none survived its first run. The prose was not wrong because it was
careless; it was wrong because prose has no `bash -n`, no exit status and no fixture.

What moved, and where it now lives:

| was | now |
|---|---|
| §4.2's three tiers and the fail-without-ending rule | the wire's classify/measure loop; controls C1, C4–C7, C16, C17, C24, C35, C36 |
| §4.2's ten-escape reject list (D5) | the wire's `case "$pattern"` arm; controls C8, C9, C25, and C37 for the accepting direction |
| §4.2's production, three verbs and four execution rules | the wire's parser and measurement; controls C10, C11, C13–C15, C23, C28–C33 |
| §8/the self-test's control table | `claim-provenance-selftest.sh`, each `fail` arm naming a required diagnostic substring |

**What did NOT move, because code cannot hold it**: D4's amendment (umbrella §1), the two-root rule
(§4.1.1(a) — the tree under test comes from cwd, the wire's helper from `$0`), the clone-fixture
decision (§4.1.1), the guard's placement and why one control must clear it (§4.1.2), the cost bound and
its escalation trigger (§4.5), the edit set (§4.6), and everything in §0, §1, §2, §5, §6, §7 and §9.

**The residue this leaves, stated rather than left to read as covered.** The controls implemented are
the hermetic ones. `C12` (argv fidelity), `C19`–`C22` (the acceptance corpus against a clone), `C27`
(I7's structural half, via a counting `git` shim) and `C38`/`C39` (§8.2's clauses read from the
inventory) are **not yet written**, and `C34` is withdrawn (§4.2's short-circuit is not observable by
any probe shape this design has). Until they land, the corpus rows umbrella §3 routes here are unproved.


### §4.5 What bounds the wire's cost, and what does not

The first draft carried a runtime table and anchored the ungated job's budget to `timeout-minutes: 2`.
Both are withdrawn. **The runtime numbers are removed rather than re-measured**: three independent passes
over the same table disagreed while each stayed internally consistent (two sit side by side in umbrella
§6, one annotated "on a busier machine"), which is not noise around a true value but different
conventions about what to include, and a fourth pass picks a fourth convention. **And
`timeout-minutes: 2` is not a budget** — it is orders of magnitude above anything the wires do, so it
catches a hang and nothing else; presenting it as the anchor replacing "~1s" would read as *stronger*
than the number it replaced while bounding strictly less. Stated plainly: **I7 is not machine-bounded at
a useful granularity, and S1 does not make it so.**

What *is* asserted is structural, not measured: **the wire's work is one tree-wide `git grep` plus a term
per annotation — never a term per tracked file.** That is visible in the code shape (§4.1's single
tree-wide command, §4.2's per-annotation probe and measurement) and it is what keeps the ungated job's
real justification — no toolchain, no cache (`.github/workflows/ci.yml:128-130`) — true as annotations
accumulate. The wire header records a re-runnable escalation trigger rather than a prediction — but
**its action was one the design forbids**, and had been since the first draft: *"batch by `in=` before
adding annotations"*, while §2's I5×I7 row says the no-`eval` rule *"forbids the one shape that would
batch several annotations into a single `git grep`"* and that *"serial execution is forced"*, and §9
records revisions 1–6 having phrased its own audited candidate as the batching this forbids. Three
sections, two of them naming the impossibility and the third prescribing it. A trigger whose remedy
cannot be executed is §2 I4×I6's `ban_control`, one level up: it looks like a disposition and can never
be acted on. (Revision 7 wrote that §9's audit *"calls the same candidate"* by that phrase in its own
voice; §9 attributes the phrase to the revisions it is correcting. The proposition is unchanged and
both sections still assert it — what was wrong was the present tense and the speaker.)

It is not fixable by loosening I5, and the reason is worth stating because it is not the obvious one:
multiple `-e` patterns **are** expressible without `eval`, each being its own argv element — but
`git grep` does not report which `-e` matched, so per-annotation counts are unrecoverable from a batched
run. Batching is unavailable to `count` and `files` structurally, not by convention.

**The condition had the same defect as the action, and revision 7 fixed only the action.** *"Dominates a
local `mise run trip-wires`"* is not a thing anything evaluates: nothing measures it, and this section's
own argument — that the runtime figures were removed as convention-dependent and I7 is not machine-bounded
at a useful granularity — forbids supplying the constant that would make it evaluable. A condition no
one can decide is the same `ban_control` shape as an action no one can perform.

The fix takes no constant, because I7 already states the ratio the trigger is really about: the wire's
work is **one tree-wide `git grep` plus a term per annotation**, so if the per-annotation term has come
to dominate, it has come to dominate *that one scan*. The wire times both — it is running them anyway —
and the trigger is: *when the wire's per-annotation stage takes longer than its own tree-wide scan,
**re-plan** — reduce the annotation set, or change the execution model. Do not batch: I5×I7 forbids the
shape and `git grep` could not attribute the result anyway.* Both operands are the wire's own, measured
in the same run, so the condition is decidable on every push and no figure in this memo can go stale
under it. §9 audits that and still declines to mint a slot, on the ground it states there.

**The bound is on the wire, and the self-test is exempt** (I6×I7). C10's positive and C19's corpus row
both run tree-scale pathspecs, deliberately: the acceptance corpus is only worth having because it runs
against the real tree (§7 Q2), and a hermetic re-enactment would drop the "known input, known verdict"
property that is its whole value. Left unstated, I7 reads as forbidding the corpus that §8 then ships.
The self-test runs once per wire invocation, not once per annotation, so it does not scale with either
the tree or the annotation count.

### §4.6 The stale prose claim

Two sites carry a runtime number nobody re-executed — the failure class this program exists for. Both
lose the number and keep the falsifiable part (no toolchain, no cache), which is what the ungated job's
rationale actually rests on. Neither gains a replacement: per §4.5 there is no honest one, and the
CI-side figure is not measurable from a workstation at all.

```
$ grep -n '~1s' .github/workflows/ci.yml CLAUDE.md
.github/workflows/ci.yml:129:  # are filtered because each pays for a Rust toolchain; the wires need none (~1s for
CLAUDE.md:94:… wire は toolchain 不要 (~1s) なので filter は何も買わない。…
```

The set-level label the new wire joins is the second prose surface: S1's wire gates documentation
provenance, not layering, so either the label widens or the deliverable is mis-filed. Edit set — the
**pre-edit** state, which §5 commit 3 is what changes:

```
$ git grep -niE 'layering trip-wire' -- ':(exclude)docs/plans/**' ':(exclude).claude/tools/**'
.github/workflows/ci.yml:145:    name: Layering trip-wires
.github/workflows/ci.yml:153:      - name: Run layering trip-wires
CLAUDE.md:66:… `trip-wires` (layering trip-wires = `scripts/trip-wires.sh`、CI の同名 job と同一) …
CONTRIBUTING.md:19:   layering trip-wires — the last of these also run as their own CI job, so a drifted
mise.toml:100:description = "Run every layering trip-wire (.claude/tools/*-trip-wire.sh)."
scripts/trip-wires.sh:4:# run every layering trip-wire. The glob below DISCOVERS what is present;
```

`CONTRIBUTING.md:19` is **S2's, whole**, and so is its continuation at `CONTRIBUTING.md:20` ("a drifted
trip-wire *allowlist* reds the PR") — one sentence, one owner. Umbrella §6 lists `:19` among the six
label sites and then carves it out, because the same sentence also asserts the allowlist mechanism S2
ships. This section's criterion (the set-level label) would claim `:19`; the umbrella's disposition
overrides, and a slice does not reverse a hand-off in its own file (§0). So S1 edits the other four
files; each wire's own header keeps saying what that wire is, and only the set-level label moves.
§7 Q1 asks for the wording.

⚠ This is a **reversal within this memo's own history**, recorded rather than smoothed over: revision 6
split the sentence and gave S1 the `:19` half, on the reading that revision 3 had misrouted it. It had
not. The premise was taken from §5's account of the umbrella instead of from the umbrella, which is the
`feedback_ao-name-not-section-number-in-briefs` 追補 4 shape — a claim about another document, sourced
from a third document that describes it. §6 records the one-PR window this leaves.
(The two exclusions are what keep this listing stable under its own landing — the memo lives in
`docs/plans/`, the wire and self-test in `.claude/tools/`.)

**Two more sites, which that command cannot reach.** Round 2 found them and the checker now reports
them, which is the more useful fact: `claim-gate-plan-check.py`'s `SITES` pass enumerates every line in
the five governing files that speaks about the wires *collectively*, and lists the ones this memo does
not cite. Every row it lists is disposed of below — two by an edit, the rest by being named as unaffected — and it fails the run if any is left undisposed, so the population is a property of this section rather than a figure in it. (Revision 3 wrote "It finds 13", which was the *previous* revision of the checker's output; the shipped one reports 7 against the memo that sentence was written from and 0 against this one. Same class as the withdrawn probe count: a claim about the checker, in the paragraph crediting the checker.) The two carrying a claim a fifth wire falsifies:

| Site | Today | Why the greps above miss it | S1's edit |
|---|---|---|---|
| `.github/workflows/ci.yml:152` | "No toolchain step: the wires are grep-only." | contains neither `~1s` nor `layering trip-wire` | keep "no toolchain step" (that is the load-bearing half and stays true); drop "grep-only", which the new wire's `git ls-files` probe, `awk` and self-test invocation falsify |
| `mise.toml:102-103` | "The wires cover disjoint layering / discipline surfaces (wasm-runtime tiers, VM-side wasm, native-ctor guards, the LayoutBox reader audit)" | same | the parenthetical **enumerates the wire set**, so it goes stale on every addition; replace it with the property it was reaching for — each script's own header documents its surface, which `mise.toml:103-104` already says (the range was cited as `:104-105` until round 5 read it: `:105` is the next sentence, about the driver owning the one registry) |

**The other five `SITES` reports, left alone deliberately.** A report is only useful if every row is
disposed of, and a row is disposed of only if the line it names was read at that line. Re-derived from
the report, in the order it emits: `.github/workflows/ci.yml:131-133` carries the ungated job's own
argument — "Gating them would mean listing BOTH `.claude/tools/**` … and `scripts/**` … in a filter" —
which a fifth wire does not touch. (The report emits `:131` and `:132` as two of its rows; the citation
is the quoted *sentence's* span, which runs on into `:133`, and it read `:131-132` until round 5 checked
the quotation against the lines. `:133` is not itself a report row — it speaks about a filter, not about
the wires.) `CLAUDE.md:52` lists `trip-wires` among `mise run ci`'s dependencies,
which stays true; and the two driver internals are

```
$ sed -n '25p;59p' scripts/trip-wires.sh
# `shopt` here: /bin/bash on stock macOS is 3.2, and the wires already record that
# be reported as "the wires are gone".
```

— `scripts/trip-wires.sh:25`, the `bash` 3.2 constraint (no arrays, no `shopt`, because stock macOS ships
3.2), and `scripts/trip-wires.sh:59`, the **resolved-root verification** message: "the root is wrong" must
never surface as "the wires are gone". All five describe the mechanism rather than the membership, so none
is falsified by an addition to the set.

**`:59` was labelled "the both-directions retention message", and it is not that line.** The retention
rationale is at `scripts/trip-wires.sh:76-84`, the passage §1 and §6 both already cite. The listing above
is therefore re-derived from `SITES` and spot-checked with a pasted `sed` rather than hand-corrected a
second time: **relabelling is not re-measuring**, and only one of the two survives being run.

This is the general shape and it is worth naming, because it is why the edit set was wrong twice: a
grep proves a *paste* matches its command, never that the command was the right one. `TRANSCRIPT`
re-runs every listing pasted above and confirms each is complete for what it asks; `SITES` exists because
that is not the same as being complete. Both edited sites were reachable only by asking "what else
describes the *set*", which is a concept, not a string
(memory `feedback_semantic-sibling-selfseed-and-regate-breadth`).

---

## §5 Decomposition and sequencing

Two files, per §1:

| File | Owns |
|---|---|
| `.claude/tools/claim-provenance-trip-wire.sh` | header rationale, the self-test requirement, invocation and guard (§4.1.2), the scan, the tier pass, the field parser, the D5 reject-list, the D4 probe, the three verb executors, the per-candidate inventory, verdict and exit |
| `.claude/tools/claim-provenance-selftest.sh` | the fixture repos and every control in §8, including the `clone` fixture's construction, reuse and teardown on the lifecycle §4.1.1 fixes |

**The `Projected` column is gone, and it is the same class this memo has already removed twice.** It
carried `~350` and `~200`, and neither figure was ever a projection: it was written once and inherited
by every revision since —

```
$ git log --oneline -S'~350' 2656d83d -- docs/plans/2026-08-claim-gate-s1-provenance.md
494e83a9 docs(plan): S1 rewrite — close plan-review round 1 (3 CRIT / 39 IMP)
```

— while the design of the two files it projected changed underneath it three times: revision 4 deleted
the fence arm *and* the second scan stage from the wire, revision 4 added a `--one` entry point, and
revision 5 withdrew `--one` and gave the self-test a `git clone --local` fixture to construct, share and
tear down. A number that survives three changes to its own subject is not tracking that subject. Per
§4.5's runtime table and §5's "five uses" — the two the memo already withdrew for it — a figure whose
value depends on the author's convention rather than on what a command reports is **removed, not
re-measured** (memory `feedback_convention-dependent-figures-are-argument`). What replaces it is §1's
authoring trigger, which is re-runnable and applies while the files are being written rather than before.

**Three live annotations, one per verb**, each chosen so that S1's own landing cannot invalidate it (D10
form 1). Once S1 lands the wire re-executes all three on every push, so each reds the build if its
measured set moves.

The nearest comparable wire is 493 lines
<!-- claim lines=493 in=".claude/tools/layout-box-reader-trip-wire.sh" -->
— `lines=` against a pathspec resolving to exactly one tracked file, and §1's line-count argument made
falsifiable.

<!-- claim files=1 pattern="REQUIRED_WIRES" in="scripts/*.sh" -->
The registered wire set has exactly one home — the property `scripts/trip-wires.sh:10-17` argues for
("One script rather than the same loop inlined at both call sites"). It is also the corrected form of one
of v1's three false annotations (`files=7`, which measured 1 — umbrella §0.1). S1 adds no file under
`scripts/`, so the value survives its landing.

<!-- claim count=0 pattern="fetch-depth" in=".github/workflows/*.yml" -->
No CI job requires a deep checkout — the blocker D1 rests on, and the firing trigger umbrella §5 gives
`#11-claim-annotation-diff-scoped-lint`, whose text is *"the first job **in `.github/workflows/ci.yml`**
that requires `fetch-depth: 0` for any reason"*. As an annotation that trigger stops being a prose
promise: the day a job adds one, the wire reds.

**The annotation is deliberately wider than the slot's trigger, and revisions 2–7 hid that by eliding
the qualifier from the quotation.** The trigger names one file; the annotation watches all three
workflows (below), so it reds on a job in `audit.yml` or `codeql.yml` that the slot's trigger would not
fire on. Widening is right — the sentence above says *no CI job*, not *no job in `ci.yml`* — but it
makes the annotation a **superset** of the trigger rather than the same object, and the previous
phrasing asserted they were the same by quoting the trigger without the words that bound it. The slot
keeps the umbrella's trigger; what §5 ships is a wider watch that will fire no later.

Two corrections to the form this took in revision 2, both of the same kind — **the annotation must
cover what the sentence claims**, and a figure beside it must not be the thing that goes stale first.

- The pathspec was `.github/workflows/ci.yml`, while the sentence says *no CI job*. There are three
  workflow files (`git ls-files -- '.github/workflows/*.yml' | wc -l` → 3), and `audit.yml` and
  `codeql.yml` were outside the gate the sentence promised. Widened to `'.github/workflows/*.yml'`,
  which is still one `QSTR` and therefore still one argv element (I5).
- Revision 2 added "All five `actions/checkout@v6` uses run at the default depth". Five is
  convention-dependent — it is five in `ci.yml` and seven across the workflow directory — and the
  version is a moving target a Dependabot PR rewrites. It bounds nothing the annotation does not
  already bound, so it is **removed** rather than re-measured
  (memory `feedback_convention-dependent-figures-are-argument`).

**Commit sequencing (one PR, ordered commits):**

1. `claim-provenance-trip-wire.sh` + `claim-provenance-selftest.sh` + **the `REQUIRED_WIRES` line**, in
   one commit. The registration cannot trail: the unregistered-wire check (`scripts/trip-wires.sh:231-237`)
   fails the moment the file appears, so any split here is a red intermediate commit.
2. §4.6's `~1s` deletion at `.github/workflows/ci.yml` and `CLAUDE.md`, both sites, one commit.
3. §4.6's set-label widening, one commit — at the **four** files. `CONTRIBUTING.md:19` is not among
   them: umbrella §6 routes that whole sentence to S2 (§0, §4.6). Revision 6 put it here and that is
   withdrawn. The cost is real and is booked as a bound in §6 rather than paid by reversing the
   umbrella — for one PR, the contributor-facing document is the last governing file still calling the
   set "layering trip-wires".
4. §4.6's two set-level *claim* edits — `ci.yml:152`'s "grep-only" and `mise.toml:102-103`'s four-wire
   enumeration — one commit. Revision 3 put both in §0's scope and gave neither a commit, so an
   implementer following §5 literally would have shipped S1 with `ci.yml:152` still false.
5. The second transcription retires (§0, §6): `check_annotations` delegates to the now-tracked
   `.claude/tools/claim-provenance-trip-wire.sh`, and `claim_gate_grammar.py` is deleted whole. The
   deletion has **two** importers, and naming only the entry point would break the other at import
   time: `claim-gate-plan-check.py` (`import claim_gate_grammar as grammar`) and
   `claim_gate_selftest.py` (`from claim_gate_grammar import classify, execute, verdict_over`). The
   self-test's three imported symbols go the way the entry point's do — through the wire — so the
   commit touches three Python files, not one, which §3.1's table now carries.

   **The closing control lands in `claim_gate_selftest.py`, not in the wire's self-test**, and the
   distinction is §7's layer rule rather than a preference. It plants a wire and asserts the checker's
   own grammar path is unreachable — a claim *about the checker*, so it belongs at the checker's tier,
   where a Python interpreter and `.claude/skills/**` internals are already present and permitted. Put
   in `claim-provenance-selftest.sh` instead, which §5's file table gives "every control in §8", it
   would put both on the ungated per-push path: exactly what §7 Q3 rejects option (b) on, and exactly
   what C26 was deleted for. It is therefore **not** a `C`-row in the self-test — the self-test's rows are the wire's
   controls — and `claim_gate_selftest.py` already carries the adjacent probe that stubs the wire and
   replays the `check_annotations`-returns-nothing defect.

   Commit 5 follows commit 1 because the delegation is unwritable until the wire is tracked, and it is
   in this PR because no later PR exists to hold it — revision 6 gave it an owner that merges before
   its own trigger fires (§9).
6. The five false rows of `docs/plans/2026-07-app-mode-turn-completion-drain.md` §5's `Lines at HEAD`
   table (§8.2), corrected against that table's own declared base. A **value-only** edit — five
   numerals replaced in place, no line added or removed — which is what keeps C21's corpus row and
   §8.2's `actual` figure for that file true; `git show --stat` on the commit is the check. Revision 6
   routed this to "the L4 lane", and a lane is not a PR, a slot or a ledger row (§9).

Gate: `claim-gate-plan-check.py` (both memos, and `--self-test`) → `cargo fmt --all` → `mise run ci` →
`/simplify` → `/code-review` → `/review` → `/elidex-review`. The checker runs **first** because every
stage after it reads the memo, and a stale figure in the memo costs a reviewer's round rather than a
second. `mise run ci` is not cargo-only — `mise.toml:116` lists `trip-wires` among its `depends`, so
from this PR onward the local gate exercises the deliverable. `/review` needs a PR, so the order is
push → draft PR → `/review <n>`.

---

## §6 What S1 does not close

- **`CONTRIBUTING.md` keeps the old set label for one PR.** Umbrella §6 gives that whole sentence to
  S2 because it also asserts the allowlist mechanism S2 ships (§0, §4.6), so after S1's commit 3 the
  contributor-facing document is the last governing file still calling the set "layering trip-wires".
  The forced S1 → S2 order bounds the window at one PR. Stated as the cost of not reversing a binding
  hand-off from inside the slice — which is what revision 6 did before this was withdrawn.
- **Annotation deletion is unrecorded.** D8's ledger is S2's. Until it lands, deleting an annotation
  turns the wire green with no trace — the theory `scripts/trip-wires.sh:76-84` explicitly rejects for
  `REQUIRED_WIRES`. S1 ships no partial ledger; the fix is S2, in the forced order.
- **Check 3 rejects the strongest artifact until S3 lands.** `grep_pass.py:76-80`'s
  `VERIFICATION_ARTIFACT_RE` accepts exactly three spellings — a `grep -[rlnEFhio]+` literal,
  `verified YYYY-MM-DD`, and a `lines N/N/N` list — and **not** a `claim` annotation. So between S1 and
  S3 the strongest form is the one form the tool refuses: three spellings are accepted and none of them
  is executed, while the executed one is rejected. (The earlier phrasing, "four accepted forms of which
  the only executed one is the one the tool rejects", counted the rejected form among the accepted ones
  — the window is exactly that it is not.) This memo passes Check 3 by pasting `git grep -n…` literals,
  not by carrying annotations.
  D9 retires the other three and admits the annotation; **S3 closes this window.**
- **No third fence tracker — the whole question is gone** (§7 Q3, §4.2). Revision 2 accepted one on a
  premise that was false: that the two in-repo trackers already behave differently on a mismatched
  marker, sourced from `grep_pass.py:254`'s `— content` comment, which means content *of the still-open
  block*. `claim-gate-plan-check.py`'s `FENCE` check now measures that relation by running both, and
  **forbids this memo from restating the verdict in either polarity** — a boolean the memo's own edits
  can invalidate is what D10 form 2 says to cite rather than write out, and a checker that read English
  polarity would be the same unexecuted-claim medium one level down. What remains is not a bound but a
  deletion: with (c) adopted there is no fence tracking in the wire at all.

- **The checker's second transcription of §4.2 and §4.2 retires inside this PR** (§0, §5's commit 5).
  Not a bootstrap that retires itself — revision 3 asserted that and it was false. Revision 6's
  replacement was a debt whose trigger was S1's own landing and whose owner was the branch S1 is
  stacked on; round 5 read the two halves together and they do not compose, because that owner merges
  **before** its own trigger fires. The owner is now §5's commit 5, the earliest commit at which
  delegation is writable at all. The closing condition is unchanged — a control that plants a wire and
  asserts the checker's grammar path is unreachable — because the untested retirement path is precisely
  what let the false claim stand. What stays a bound is the window: between the stacked branch landing
  and S1 landing, `main` carries the second copy. The forced order makes that window one PR, the same
  shape as `CONTRIBUTING.md`'s above.
- **`in=` is exactly one pathspec element.** One `QSTR` → one argv element (that is I5). Git does accept
  an *exclude-only* pathspec (verified: `git grep -lniE 'layering trip-wire' -- ':(exclude)docs/plans/**'`
  returns hits), so one exclusion is expressible; two are not — which is why §4.6's edit-set listing is a
  pasted command rather than a live annotation. Found while building §5's annotations, per the rule that
  a gap met while writing a control is a finding about the spec, not a detail to complete.
- **No verb expresses a wall-clock, environment or process claim.** All three measure the tracked tree,
  so "~1s for the whole set" is not re-expressible as an annotation — which is why §4.6 deletes it
  instead of converting it.
- **S1 is opt-in** (umbrella §4): it executes the annotations that exist, and a numeric claim written
  without one stays silent. Umbrella §5's `#11-claim-annotation-diff-scoped-lint` owns that, blocked on
  the depth-1 checkout §5's `count=0` annotation now watches.
- **The gate does not scan its own files** (D2) — necessary for S1 to land, and also a tamper path.
- **Untracked files are invisible; tracked ones are read from the working tree.** Revision 3 said
  `git grep` "reads the index, so an annotation is inert until staged". Only the first half is right,
  and the second half is backwards. Measured:

  ```
  git init -q .; printf 'alpha\n' > a.txt; git add -A; git commit -qm x
  printf 'alpha\nalpha\n' > a.txt          # worktree 2, index 1
  git grep -c alpha -- a.txt          → a.txt:2      # working tree
  git grep --cached -c alpha -- a.txt → a.txt:1      # index
  rm a.txt                                 # worktree absent, index present
  git ls-files -- a.txt               → a.txt        # liveness says yes
  git grep -c alpha -- a.txt          → (exit 1, no output)
  awk 'END{print NR}' a.txt           → awk: can't open file a.txt  (exit 2)
  ```

  So the scope is **tracked files, at their working-tree content**. `--untracked` adds untracked files;
  it is not what makes a modified tracked file visible. Three consequences the old wording got wrong:
  an edited annotation is live without staging; the gate does not govern "the committed tree"; and the
  preamble's base is a working tree with this file in it, not an index (D10 form 3 unaffected in
  substance — the base is still declared and still includes this file — but stated correctly).

  The block's last four lines are a fourth consequence, and it is **not a bound — it was a defect, and
  the wire's rule 3 (§4.2–§4.2) fixes it.** The D4 liveness probe read the **index** while all three verbs read the
  **working tree**, so a file deleted in the worktree but still in the index passed liveness and then
  measured nothing. Revision 6 wrote it down here as "recorded rather than fixed", with no owner, no
  trigger, no re-eval date and no control; round 5 read that against what a §6 bullet is for. A bound
  records what S1 does not reach. This was S1's own discriminator — the clause D4 exists to make a
  claim falsifiable — answering about a different tree from the one it is guarding. The probe now
  filters to paths present in the working tree, with a diagnostic distinct from the dead-pathspec one,
  and the self-test C28 controls both directions. What remains a bound is the half no filter reaches: the wire
  measures uncommitted local edits, so a green local run is not a statement about what will be pushed.
- **Single-line annotations only** (§4.2), and **`count` counts matching lines, not matches** — an author
  who meant "occurrences" measures something else, correctly and forever.
- **The wire re-executes; it does not judge.** A pattern that measures the wrong thing stays green. The
  canonical instance is a substring swallowing its own boundary — `console` contains `sole` — where bare
  and boundaried are both "correct" re-executions and only the author knows which was meant. §8 C0 keeps
  a control for it precisely because the wire cannot.
- **A pattern may match its own annotation, and this is permitted.** If `in=` covers the annotating file,
  the annotation's own line is measured and must be counted in; §8 C17 pins both directions. (The first
  draft said both at once — its §6 permitted this and its control 10 failed it.)
- **CI reds the PR; it does not block the merge.** No required-status-check rule on `main` — the same
  caveat `layout-box-reader-trip-wire.sh:60-63` and `.github/workflows/ci.yml:140-143` already record.

---

## §7 Open questions for plan-review

**Q1 — set-label wording**, and only the wording. §4.6 widens the label at four files: is "repository
trip-wires" the right replacement for "layering trip-wires"? The second half this question used to
carry — *or should S1's wire get its own driver?* — is **not open**: §0 books registration in the
existing driver's `REQUIRED_WIRES` as commit 1 and §4.6/§5 book the widening at four files, so the
memo decides it three sections before asking it. Recorded rather than silently dropped, because a
question whose answer is already shipped is a second decision surface for a settled decision. The
ground for that decision, unchanged: a second driver costs a second ungated CI job and a second place a
wire can go missing, and §1 shows the *first* driver's glob is already what decides what runs.

**Q2 — the acceptance corpus runs against a clone of the live tree, and the open half is narrower than
this question used to state.** §8 C19–C22 re-execute v1's three false annotations and one of #504's
`lines=` claims verbatim, asserting FAIL and a diagnostic naming the historical asserted value. The
question was posed as *live repo vs hermetic re-enactment*, and §4.1.1 chose **neither**: a
`git clone --local` built once per self-test run, which keeps "known input, known verdict" while
letting the corpus reach the wire through the scan rather than around it. What remains genuinely open
is the residue that choice does not remove: the clone is a clone *of this repository*, so the corpus
still reads files other lanes own, and C10's and C11's positives read one exit status covering every
live annotation in it — umbrella §2 records an unrelated lane's commit (`658cc302`) moving two of them
already. Recommendation: keep the clone — umbrella §3 rows 4–5 route these exact inputs here, and
umbrella §2's worked instance is the argument that a real firing beats a synthetic one — and read the
residue as the reason C10's and C11's positives assert liveness rather than a value (the self-test).

**Q3 — CLOSED by round 3: the wire has no fence tracker.** The question was whether the wire should
carry a third implementation of fence tracking (a), shell out to the Python one (b), or drop the fence
arm entirely (c). Round 3 measured the quantity the recommendation for (a) had assumed instead of
measured — how many annotations actually rely on the fence arm — and the answer is zero (§4.2). With
(c)'s only stated cost at zero and (a) costing a third copy of an algorithm, *One issue, one way*
decides it. **(c) is adopted**; §4.2 now has a single inertness rule and no per-surface table.

(b) is rejected on a ground worth recording, because C26 was violating it: it puts a Python interpreter
in the path of a wire whose entire justification for running ungated is that it needs no toolchain
(`.github/workflows/ci.yml:128-130`), and couples a repository gate to a skill directory's internals.
That rejection now holds without exception — the self-test's C26, which ran the memo-checker inside the wire's
self-test and did both of those things harder than (b) would, is deleted (the self-test).

**The layer rule that rejection implies, stated because §9 hands a decision across it.** Read alone, the
(b) rejection and §9's routing collide: (b) forbids a Python interpreter and a skill-directory coupling
in a gate's path, while `claim-gate-plan-check.py` does both deeply — `sys.path` into
`.claude/skills/elidex-plan-review/`, `importlib` loads of `preflight.py` and `grep_pass.py` reaching
private names, and a subprocess call to `preflight.py` — and §9 names canonical a note that books putting
exactly that tool on the trip-wires loop. The two are only compatible under a rule neither states:

> **A repository gate** — anything on the ungated per-push path — takes no toolchain and couples to no
> skill directory. **A plan-review-stage tool** — run when a human invokes the gate that reads plan-memos
> — may do both, because its trigger is a person and its dependencies are already in the room.

`claim-gate-plan-check.py` is the second kind, which is why it is a prerequisite PR rather than a wire,
why §5's gate lists it as a step rather than registering it, and why the *"a check nobody re-runs is a
transcript"* objection §1 raises against a split-out helper does not transfer to it unchanged: the memo
checker's re-runner is the author at the gate, not the driver. That is also the whole content of §9's
own-deferral #1 — whoever executes the SKILL.md note must decide **which kind** the loop makes it, and
moving it to the trip-wires loop converts it into the first kind and re-opens (b). Recorded here so the
executor meets the constraint rather than discovering it.

**And the cost this puts on every push, stated rather than left to be noticed.** The self-test runs on
every wire invocation, the wire runs in `mise run trip-wires` ⊂ `mise run ci` and in the ungated CI job,
and §4.1.1 builds one clone per self-test run — so every local gate run and every push clones this
repository. No other wire's self-test leaves hermetic ground. It is justified by §1's own principle
rather than excused: a corpus that runs only when someone remembers to run it is the transcript §1
rejects, and §7 Q2's argument for the corpus is that a real firing beats a synthetic one — which is only
true if it fires on the schedule the gate does. §4.5's escalation trigger covers it if that ever stops
being worth it, and no figure is offered here for the reason §4.5 gives.

---

## §8 Test strategy — **now in `claim-provenance-selftest.sh`**

The control table is withdrawn from this memo for the reason §4.2–§4.2 gives: a control is a program,
and a program's home is a file that runs. The controls exist, the driver runs them on every push (`REQUIRED_WIRES`), and their count is
whatever `bash .claude/tools/claim-provenance-selftest.sh | tail -1` reports — a number stated here
would be a quantity about this program's own tooling that the next control falsifies, and each `fail` arm names a substring the diagnostic must contain — because two
different defects of one fixture produce the same exit code.

**What stays here is the part a self-test cannot assert about itself.**

*Fixture model.* `own` = its own `mktemp -d` and its own wire invocation, so two `own` controls cannot
mask each other whatever their diagnostics. `escapes` and `escapes10` put several arms in **one**
invocation, so within them the diagnostics must discriminate; they exist at all only under the
fail-without-ending rule. `clone` = a `git clone --local` of this repository, built once and reused,
one wire invocation per row.

*Mutation, not assertion.* A first run reporting every control green is a claim about the controls. The
controls were mutation-tested: removing the mention arm reds 4, dropping the `.claude/tools` exclusion
reds 18, spelling `lines` as `wc -l` reds 5, dropping the regular-file test reds 6, dropping the reject
list reds 12, rejecting every backslash reds 12, and swallowing the fourth outcome kind reds 1. **Each
mutation was confirmed to have applied before its result was read** — the first harness silently
changed nothing and reported two clean mutations as controls that cannot fire.

*The one the guard was disabling.* C18 proves the wire requires its helper by name and invokes it. Run
standalone, the self-test has no recursion guard set and both arms passed; invoked by the wire — which
sets it — they inherited it and were exercising a wire that skips the very step C18 exists to prove.
They clear it explicitly now. Only running through the driver revealed it, and §4.1.2 had predicted the
inheritance hazard in the prose the control was written from.

*Gaps, recorded rather than completed.* §4.2's liveness short-circuit has no control: the property is a
count of `test -e` calls, which is neither a diagnostic nor a `git` invocation, so neither probe shape
reaches it. C12, C19–C22, C27, C38 and C39 are unwritten (above).


### §8.2 Live-tree acceptance

The umbrella carries two live annotations and this memo carries three (§5). Once the wire lands it
executes all five on every push, so they are an acceptance criterion rather than a side effect. The
criterion is stated as a **property**, because the tier populations are exactly the figure D10 forbids
asserting:

> Every strict annotation in the tracked tree passes; no candidate line classifies MALFORMED; every
> illustrative annotation in either memo is **backticked** and classifies as *mention*; and at least
> one strict annotation exists for each of the three verbs.

Revisions 1–3 wrote that middle clause as "fenced or backticked", which (c) makes false in the direction
that matters: a fenced annotation is now **strict and executed** (§4.2). The property is also why (c)
costs nothing today rather than merely little — it is satisfied by the tree as it stands, with no
annotation to rewrite.

The last clause stops the criterion being satisfiable by a wire that classifies everything as a mention.

**Two of the four clauses had no producer, and that is what §4.2's inventory is for.** Clauses 1 and 2
are the wire's exit status. Clauses 3 and 4 are not: a mention is *ignored*, and a verb is not visible in
an exit code, so a per-annotation tier-and-verb listing has to exist somewhere or half of a criterion
called *standing* is re-checkable only by programs that were thrown away. It was: the listing below comes
from two throwaway programs — a classifier transcribed from §4.2/§4.2's literal text and an executor for
the three verbs — which are not in the deliverable (§5 ships the wire and the self-test, and no commit
adds anything else), and whose transcript the checker reports among the commands it cannot re-run. With
§4.2's inventory the whole criterion is read off one run of the wire, which is the artifact that survives
the prototypes. Printing an inventory is not D8's ledger — S2 stores and diffs one — so §6's bound stands
unchanged.

**A producer is not a consumer, and until round 6 this criterion had only the first.** The inventory
makes clauses 3 and 4 *readable*; nothing was specified to read them, so the wire would print on every
push a listing no arm compared against anything, and *"every illustrative annotation classifies as
mention"* would hold or fail with no one the wiser — §1's own standard (*a check nobody re-runs is a
transcript, not a gate*) applied to the four-clause property itself. **C39** closes it: run against the
`clone`, so its subject is this repository's real annotation population rather than a fixture's, it
asserts the inventory the wire prints satisfies all four clauses; its negative removes the backticks
from one illustrative annotation, which turns a *mention* into a strict annotation carrying a false
count, so clause 3 fails and the run reds. That is the clause whose violation the tree can actually
produce — clauses 1 and 2 are already the exit status, and clause 4 is a property of the memos' own
annotation set, which §5 fixes.

The property was **executed**, not asserted, at the base declared at the top of this file (a working tree
containing it — D10 form 3):

```
$ git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' \
    -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**' | wc -l
       7
$ python3 <prototype-classifier> <this memo> <the umbrella>
--- {'strict': 5, 'mention': 3}
$ bash <prototype-executor>
OK      lines asserted=493  actual=493   in=.claude/tools/layout-box-reader-trip-wire.sh
OK      files asserted=1    actual=1     in=scripts/*.sh
OK      count asserted=0    actual=0     in=.github/workflows/ci.yml
```

Zero MALFORMED, one strict annotation per verb, C6's malformed specimen correctly inert. That transcript
also shows why it is not the standing form: its `count` row reads `in=.github/workflows/ci.yml`, the
pathspec §5 widened to `'.github/workflows/*.yml'` afterwards, and nothing re-ran the prototypes to
notice. A transcript of a program that no longer exists ages exactly like the claims this slice is
about; the wire's inventory is re-run on every push. The per-line
listing is deliberately **not** pasted: every line of it carries a literal `<!-- claim …`, so pasting it
would add candidate lines to the tree being counted and the block would disagree with itself on the next
run — D10's mechanism reaching the transcript instead of the claim. (The same thing happened once more
here: §8.3's prose originally *named* two glob shapes and so contributed two of the soft findings it was
describing.) Building the two prototypes produced one finding each, both folded back into §4: the
backtick arm must be evaluated against the backticks *before* the `<!--`, and the `lines` executor cannot
share an argument position with `pattern` — the production's own "`lines=` takes no `pattern`" clause
surfacing in the implementation, and why C7 exists.

The same executor over the self-test's corpus rows reproduces all four expected failures:

```
FAIL    count asserted=299   actual=80     in=crates/shell/elidex-shell/src/**/*.rs
FAIL    files asserted=7     actual=1      in=scripts/*.sh
FAIL    lines asserted=1219  actual=1251   in=docs/plans/2026-07-app-mode-turn-completion-drain.md
FAIL    lines asserted=632   actual=647    in=crates/shell/elidex-shell/src/app/drain_host/mod.rs
```

The last row is the one to read twice, and its story is not the one it first looks like. `632` did not
go stale: it was **false at the instant #504 landed**, and it is still false today. Measured —

```
git show 3045f256:crates/shell/elidex-shell/src/app/drain_host/mod.rs | awk 'END{print NR}'   → 647
awk 'END{print NR}' crates/shell/elidex-shell/src/app/drain_host/mod.rs                       → 647
git log --oneline 3045f256..HEAD -- crates/shell/elidex-shell/src/app/drain_host/mod.rs | wc -l → 0
```

The file has not been touched since, so the 15-line gap was there at land time. And the column heading
in that table is literally `Lines at HEAD`.

**Revision 2 said this was the one row the correction left behind. It was not, and the real shape is
worse.** Re-measuring every numeric row of that table against `3045f256` — which is the table's own
declared base — gives the figures below. They are twenty live values about files three other lanes own,
so the derivation is stated rather than the measurement trusted; the `Today` column is the same command
with the `git show` prefix dropped:

```
$ bash -c 'for p in crates/shell/elidex-shell/src/app/drain_host/mod.rs crates/shell/elidex-shell/src/app/drain_host/host.rs crates/shell/elidex-shell/src/app/events.rs crates/shell/elidex-shell/src/app/mod.rs; do printf "%s " "$(git show 3045f256:$p | awk "END{print NR}")"; done'
647 358 230 897
```


| Row | Asserted | At `3045f256` | Today |
|---|---|---|---|
| `app/drain_host/mod.rs` | 632 | **647** | 647 |
| `app/drain_host/host.rs` | 358 | 358 | 358 |
| `app/events.rs` | 230 | 230 | 230 |
| `app/mod.rs` | 897 | 897 | 897 |
| `app_test_support.rs` | 317 | 317 | 317 |
| `app_turn_completion_tests.rs` | 648 | **662** | 662 |
| `app_history_phase_sep_tests.rs` | 503 | **505** | 505 |
| `elidex-script-session/src/engine.rs` | 709 | **718** | 718 |
| `elidex-js/src/engine.rs` | 830 | 830 | 830 |
| `elidex-js/src/tests_engine_s1c.rs` | 430 | **437** | 437 |

**Five of ten were false at land, and every one of the five is an undercount.** That direction is the
finding. A table that had drifted randomly would miss in both directions; a table that misses in one
direction was measured *once, early*, and never re-measured as the branch kept moving — which is
precisely the failure mode this slice exists to make impossible, caught in the artifact that first
described it.

It also corrects what revision 2 drew from this. #504's §5 drift is the origin lesson's failure 3 — it
was found and it was corrected — and the correction fixed the numbers the finding named while leaving
**five** untouched, not one. That is still the
fix-the-cited-site-leave-the-mirror-sites shape (`feedback_findings-cluster-in-self-added-scope`), but
at five out of ten it is no longer a missed mirror site: the review found one instance of a systematic
error and treated it as an isolated one. Nobody re-ran the column.

The acceptance corpus takes one of the five (C22). The other four are the same input shape with
different constants and would add no discriminating power to the self-test, so they are named here
rather than duplicated as C-rows — the corpus is chosen for coverage of *verdict shapes*, not for
counting instances.

Umbrella §2 records the complementary case — its own two annotations moving when `658cc302` landed
(`FAIL    count asserted=62  actual=64   in=crates/**/*.rs`). The wire's verdict format is taken from
that output, which is why every `$3` above is spelled `asserted=`.

### §8.3 Preflight

```
$ python3 .claude/skills/elidex-plan-review/preflight.py \
    docs/plans/2026-08-claim-gate-s1-provenance.md
  total entries  (M):   8  (data rows, breadth basis)
  malformed rows:       0  (no §<number> reference)
  unmapped-label rows:  8  (has §<number>, label not in SPEC_LABEL_REVERSE)
  unique specs (K):     2
  split decision:       ok (single PR scope)
  grep-pass:            0 hard, 5 soft
EXIT=0
```

All 5 soft findings are one shape: a `crates/…**/…` pathspec read as a filesystem path by Check 1's
matcher, which cannot resolve a glob. The `⚠ unrecognized labels` line for the two in-repo documents §3
substitutes for spec rows is printed *outside* that tally and is not one of them — a decomposition the
first draft got wrong in both directions. Zero Check 3 findings: every enumeration claim in this memo has
a `git grep -n…` literal within its vicinity, which is the only artifact form Check 3 accepts today (§6).

The count moved from 5 to 7 when revision 3 added two `crates/…**/…` pathspecs, **both of them in §2's
I6×I7 row** — the two tree-scale controls that row exempts from I7, named there in one sentence. The
attribution stood as "§2's I6×I7 row and §4.5's exemption" until round 5 read §4.5: that paragraph names
C10 and C19 in prose and carries no pathspec literal at all, so it contributes nothing to this tally.
The sentence also said "this revision", inherited unchanged from the revision where it was true.

It moved to 8 in the round-5 pass: giving the self-test's C10 a positive that states the verb it exercises put an
eighth tree-scale pathspec in that row. It moved to 9 in the round-6 pass, at **one** site: the wire's rule 3 (§4.2–§4.2)
had to show what C10's pathspec resolves to in order to price the liveness test that broke I7, and that
measurement is a pasted `git ls-files` whose argument is the pathspec itself. §2's I3×I7 row states the
same `1407 of 1706` and contributes nothing here, because it cites §4.2 rather than repeating the
argument — the distinction this paragraph got wrong about §4.5 one paragraph above, established by
locating every such literal rather than by recalling where they are.

**And this paragraph keeps doing it.** The round-6 draft of the two sentences above spelled
the pathspec inline while explaining which site spells it, `PREFLIGHT` reported 9 against a freshly
written 9 that was already 10, and the sentence naming the hazard was the sentence committing it. The
rule this yields is narrower than "no literal here": a paragraph whose subject is *where a token
appears* may not contain the token, in any role, including as evidence for its own claim.

Each draft of this very paragraph has *named the pathspec inline* while explaining that naming it would
add to the count — the paragraph forbidding the act performing it — and each time `PREFLIGHT` reported a
freshly-written figure that was already one higher. **No count of those firings is given**, because a
paragraph whose subject is a self-invalidating figure had, until round 6 measured it, three different
tallies of its own history within thirteen lines (*three times* / *the second demonstration* / *twice*).
That is the convention-dependent class §0 removes rather than re-measures, and it survived here because
the sentence's subject is this memo rather than its tooling, so `TOOLING` never looked. No literal
appears here now, for the reason the sentence gives. The self-invalidation §8.2's per-line listing
declines is the same shape, and §8.2 declines it by construction where this block has to be told.
**This block is a
pasted transcript and therefore a claim**, so it is one `claim-gate-plan-check.py` re-derives: its
`PREFLIGHT` check re-runs preflight against this memo and compares M, K, the malformed and
unmapped-label counts, the hard/soft split and the exit code with what is pasted here. It caught this
exact drift within the revision that caused it — which is the whole difference between a figure and a
figure with a checker.

---

## §9 Defer ledger

**The standard this section states, applied to this section.** A paragraph below disqualifies a
destination with "no PR, no plan-memo and no ledger row" as *routed nowhere*. Round 5 applied that test
to §9's own routings and four failed it: two named owners that cannot hold what they were given, one
named a *lane*, and one was written into §6 with no row here at all. So every row below carries three
things — an **owner** (a commit, or a file that exists), a **trigger** (an event that can be observed
to have happened) and a **home** (something a reader can open). Where no such triple exists, the row is
an own deferral and is counted as one.

**Three of revision 6's routings were dischargeable here, and two of them became commits.** The third
became a design fix inside commit 1 rather than a commit of its own — stated because this section's own
rule is that an owner is *a commit, or a file that exists*, and "fixed in the wire's rule 3 (§4.2–§4.2)" is neither until
the row says which commit carries it.

| Routed to, revision 6 | Why that owner could not hold it | Disposition |
|---|---|---|
| the branch S1 is stacked on, triggered by S1's landing (§0, §6) — retire the checker's second transcription of §4.2/§4.2 | it merges **before** its own trigger fires; on the day the wire exists there is no such PR left to delete `claim_gate_grammar.py` or write the closing control | **§5's commit 5.** The PR that creates the wire is the only one that can delegate to it |
| nothing — "recorded here rather than fixed" (§6): the D4 probe reads the index while all three verbs read the working tree | no owner, no trigger, no re-eval date, no §8 control, and no row in this section | **fixed in the wire's rule 3 (§4.2–§4.2)** (inside §5's commit 1, which specifies the grammar), controlled by the self-test C28 and C34. A bound records what S1 does not reach; this was S1's own discriminator answering about the wrong tree |
| "the **L4 lane**" — the five false rows of #504's `Lines at HEAD` table (§8.2) | a lane is not a PR, a slot or a ledger row, and that lane has nothing in flight to receive it | **§5's commit 6**, value-only, so C21 and §8.2 stay true |

**One candidate was audited and is not a slot** — the wire's cost as the annotation count grows.
Revisions 1–6 phrased the candidate as *"batch annotation execution by `in=`"* while stating in the same
sentence that *"I5 × I7 forbids [it] structurally"*, so the audited item was an action the design does not
permit and the escalation trigger §4.5 rests this verdict on prescribed it. §4.5 now carries the
re-plan action instead, and the candidate is restated as what it actually is: *the per-annotation stage
coming to dominate the wire's own tree-wide scan*. **Round 6 found the condition had the same defect as
the action** — *"dominates a `mise run trip-wires` run"* named no measurement and §4.5's own argument
forbade supplying the constant that would make it one — so §4.5 now states it as a ratio between two
things the wire times in the same run. *(1) platform gap?* No, a performance property of one
script. *(2) repeat signal?* No. *(3) existing home?* Yes, the wire's own header, where the sibling wires
keep comparable notes. *(4) second tracking home?* Yes. → **Not a slot**: a header note with a
re-runnable trigger, the disposition `.claude/skills/elidex-plan-review/SKILL.md:91` records for its own
deferred parser ("deliberately NOT a `#11-*` platform slot … skill-infra not a platform gap") — including
its **re-eval date**, which that precedent supplies with a reason this trigger shares: a conditional
trigger that may never fire needs a date more than one that fires every round. **Re-eval: 2026-11-01.**
The verdict is unchanged; what changed is that both halves of the thing it disposes of can now be acted on.

**One own deferral survives the audit.**

- **A third checker belongs in `SKILL.md`'s checkers-loop note, and cannot be put there from here.**
  Owner: S1's author, at S1's landing. **Re-eval: 2026-11-01** — the calendar fallback the defer policy
  requires alongside a trigger, and which this row omitted while the precedent it cites two paragraphs
  down carries one, for the reason that applies here more sharply: the trigger names a push that may
  never happen, since §9 measures below that neither branch is on a remote.
  Trigger: the first push at which both that checker and the note
  are on `main`. Home: memory `project_stale-claim-gate-v1-review`, the lane's SoT, which exists today
  — discharged into the note itself once the note reaches `main`. Audit: *(1) platform gap?* No —
  plan-review skill infra, the class the note's own neighbour entry already records as "deliberately
  NOT a `#11-*` platform slot … skill-infra not a platform gap". *(2) repeat signal?* Yes; the note
  calls itself the "second of this kind". *(3) existing home?* Yes, the note. *(4) second tracking
  home?* Minting a `#11-*` slot would create one. → **Not a slot; a memory row, and this PR's own
  deferral.**

**Four things revision 6 said in routing that row were wrong, and the corrections are why the row is
this small.**

*There are not two answers to one question.* Revision 6 wrote that "two decision surfaces for *the same
question* already exist" and that "the two answers are opposite". They answer different questions. The
docstring answers **when the two checkers collapse into one**; the note answers **where the plan-memo
consistency checkers run**, and says nothing about collapsing anything. What is shared is a *fact*, not
a question: the note enumerates the checkers by name and does not know a third exists. That makes S1's
contribution an enumeration edit rather than a decision — which is why nothing here needs deciding, and
why the row above is a registration rather than a hand-off. **Withdrawn**, in both halves.

*The competing sentence was already withdrawn, on this branch, before §9 prescribed withdrawing it.*
Revision 6 quoted the docstring as holding "collapse them when a **third** memo wants one" and
prescribed that it "withdraws its competing sentence rather than restating it". Read at this branch's
head, the docstring labels that disposition the old one, records that it was priced off an undercount,
and replaces it with *collapse once both have landed*. The prescription had nothing left to prescribe.

*The note was mis-described, and one description was a quotation of a sentence that is not in the
file.* In the order the errors matter:

- The phrase §9 presented in quotation marks — *"put the plan-memo consistency checkers on a loop"* —
  **does not occur in that file**; `grep -c` for it returns 0. What the note says is *"the plan-memo
  consistency checkers are not on any loop"*. This is the **second** fabricated quotation this program
  has produced. The first is recorded in the stacked branch's own docstring: a sentence attributed to
  `plan-xcheck.py` that is not in it and that says the opposite of what that file says. Two instances
  is a class, not a slip, and the class has a shape — both quoted a *nearby* document from a
  recollection of it rather than from it, which is §4.6's `CONTRIBUTING.md:19` misrouting and the
  `feedback_ao-name-not-section-number-in-briefs` 追補 4 pattern in a third dress. The remedy is not a
  rule: a quotation is the one claim form that is trivially checkable, and `grep -c` on the quoted span
  is the check.
- The three files it names — `scripts/trip-wires.sh`, `mise.toml`, `.github/workflows/ci.yml` — are
  where the note says nothing **runs** the checkers. They are its evidence for the gap, not its edit
  set. What it specifies is two parts in one PR: (a) invoke `plan-sweep.py` and `plan-xcheck.py` from
  **Step 1.5**, or from the ungated `trip-wires` job, generalising `plan-xcheck.py`, which hard-codes
  one memo's labels; (b) extend `preflight.py`'s `SPEC_LABEL_REVERSE` to CSS-module labels. Of those,
  `preflight.py` is not an S1 file at all, and the only possible overlap with S1 is
  `scripts/trip-wires.sh` — under the note's parenthetical alternative, not its primary form.
- "three of S1's five edit files" is wrong in both halves — but the correction is not a different
  numeral. §5 states no file count; six is its *commit* count, and enumerating the files those
  commits touch gives a different answer again depending on whether the deletion, this memo and
  another lane's memo are counted. §3.1 now carries the enumeration by name and tier, so the
  refutation is that the note's edit set is wrong about *which* files, not about how many.

*And the note's own trigger cannot be fired from here.* It reads *the first PR that ships* `plan-sweep.py`
*and* `plan-xcheck.py` — "for the umbrella that authored them, the seam-3 prereq PR". S1 ships neither,
and neither does the branch it is stacked on. So revision 6's rule — *whichever of the two branches
lands first adds the third checker to that note* — did not merely lack an owner: the file it edits does
not carry the note on either branch that would do the editing.

*Where the note is, stated as a property rather than a hash.* The note is on the local branch
`layout-decorated-inline`, and on no remote. Revision 6 pinned it to `0d4d474c`; that commit is no
longer that branch's head, which is the fate `ef18b828` already met in this memo's own preamble — a
hash pinning a branch still being rewritten has the same lifetime as the branch. The re-runnable forms:
`git grep -c 'not on any loop' -- .claude/skills/elidex-plan-review/SKILL.md` finds nothing in this
worktree and nothing at `origin/main`; and `git ls-remote --heads origin` for either branch prints
nothing, which is why neither can carry a PR, a plan-memo or a ledger row.

**One booking is registered at ship time rather than deferred.**
`#11-claim-annotation-diff-scoped-lint` is the umbrella's slot, audited in umbrella §5 with its trigger
(the first `.github/workflows/ci.yml` job requiring `fetch-depth: 0`, now watched by §5's `count=0`
annotation) and its re-eval date. But `grep -rl 'claim-annotation-diff-scoped-lint'` over the project
memory directory returns no file, and `git ls-tree origin/main --` for the umbrella's path prints
nothing: the slot's only home is a document that is not on `main` and is not the project's slot
registry. The registry is memory `project_open-defer-slots`, and the precedent for SoT-homing a
plan-local booking there is explicit. **At S1's landing the slot is registered there**, carrying
umbrella §5's trigger and date. Not a new deferral — the slot is already audited and already owned;
what S1 adds is the registration, which is an action in its landing rather than a promise to someone
else.

**Not S1's deferrals** (owned elsewhere, listed so no one re-books them). Each carries the same triple:
the owner is a slice with a plan-memo named in the umbrella's slice table, the trigger is the forced
S1 → S2 → S3 order, and the home is umbrella §6's per-slice hand-off, which lands with this PR. D8's
ledger → S2; the whole `CONTRIBUTING.md:19` sentence → S2 (umbrella §6, §0, §4.6); D7/D9 and Check 3,
including §6's four-form window → S3.

Revision 2's list ended that S3 item with "and the fence-tracker reconciliation", which was a defer to
a slice that does not accept it: umbrella §6's S3 line reads *"D7's shared word lists and the
cross-implementation control"* and names no fence work. So the routing was a claim about another
document that the other document does not make — the same class as the rest of round 2's findings, in
the one section whose job is to be exact about who owns what. It is withdrawn rather than re-routed,
because there is nothing left to route: the premise that there was a reconciliation to do was §6's
false one, and §7 Q3 is now closed by adopting (c) — there is no third tracker to reconcile.

**The finding this memo makes about a landed document is now a commit, not a routing.** Round 3's point
was that §9's job is to be exact about who owns what, and that a memo which *proves* a landed document
false while booking zero deferrals has hidden one in prose. #504's `Lines at HEAD` table is false in
five of ten rows (§8.2), all undercounts, all still false on `main`. C22 takes one as corpus; revision 6
sent the rest to "the **L4 lane**" with the measurement attached, and observed correctly that S1 is
opt-in (§6) so the wire will never red on them — which is the reason the routing needed an owner and
not a note. A lane is not a PR, a slot or a ledger row, and that lane has nothing in flight to receive
this. Since the measurement is already taken and pasted, the correction is **§5's commit 6**: five
numerals, replaced in place, against the base that table itself declares.
(memory `feedback_duplicated-decision-surface-blocks-converge`.)

**Own deferrals: 1 of the per-PR ≤3 budget.** Revision 6 reported 0, and that 0 was **zero by
construction**: three obligations had been handed to owners that could not execute them, and a fourth
was written into §6 with no row here at all. The audit yields 1 rather than 4 because the other three
turned out to be executable in this PR and became commits — which is the disposition the defer policy
prefers anyway, and the reason 1 is an honest count rather than merely a small one.
