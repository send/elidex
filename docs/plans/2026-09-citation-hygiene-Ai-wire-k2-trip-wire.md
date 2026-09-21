# Slice A-i-wire — the K2 generic-core layering trip-wire

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire**.
**Branch**: `webref-generic-core-trip-wire`, stacked on `webref-cite-audit-tool` (#501).
**PR**: #519. **Status**: implementation carried from #501 `611758ff`; **plan-review round 1
returned 2 CRIT / 25 IMP / 19 MIN**, and this revision is its disposition. The review's one
unresolved fork — where this instrument should live at all — is §1.1.

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
starting point rather than a footnote (`project_citation-hygiene-program.md`, the R73 block):
the edge-dense trigger's *"no canonical algorithm"* limb was **falsified by measurement**, so
the findings of that period were evidence of *"a canonical mechanism being re-implemented by
hand"*, not of inherent complexity; and the wire is K2's enforcement, i.e. A-i's own
invariant, so carving it reproduces the *"invariant here, mechanism there"* split that gate 4
raised. **The same block set the re-evaluation trigger** — *"next time the wire draws a
finding, consider option B"* — and it fired in every round from R74 to R97. So this carve is
what the SSoT scheduled, not a reversal of it. **The second ground, however, is still
unanswered**, and §1.1 is where it is answered or the carve is withdrawn.

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

### §1.1 — the open fork: where this instrument belongs

Review round 1 surfaced a third option that neither #501 nor draft 1 considered, and it
dissolves this slice's hardest question instead of answering it. **This is not decided.**

| | where K2 is decided | §6's contention | cost |
|---|---|---|---|
| **(a) here** — a shell wire in `REQUIRED_WIRES` | `trip-wires` job, grep + git, no interpreter | must be answered | K2 unenforced until this PR lands |
| **(b) slice A-iii** — a Python check in the `tools` job | A-iii already plans an **ungated `tools` job and an interpreter floor** (umbrella, A-iii row) | **does not arise** — that job is interpreter-bearing by design | K2 unenforced through A-ii *and* A-iii; the git-population semantics this wire learned over ~30 review rounds are re-derived in another language |
| **(c) withdraw the carve** | back in #501 | inherited by #501 | the instrument returns to a slice that never plan-reviewed it |

⚠ **(b) is the one that makes §6 disappear.** The question *"may the required, ungated wire
set require an interpreter?"* exists because this wire chose the `trip-wires` job as its home.
A-iii's job answers it by construction.

⚠ **And (b) is not free of the SSoT's second ground**: it separates the invariant (A-i's K2)
from its mechanism by two more slices, which is the objection the lane recorded when it
declined the carve. (a) separates them by one PR that lands immediately after.

Whichever is chosen, §5–§7 below are written for (a) and must be re-derived if (b) or (c)
wins. Nothing in this memo should be read as having settled it.

## §2 The invariant, and the invariants it intersects

**K2** — the generic core names no elidex file path, where *file path* means
`.claude/(skills|tools)/` plus **two further segments**, and *generic core* means
`.claude/tools/_webref/` plus the entry script `.claude/tools/webref`.

⚠ **K2 has a closed part and an open part.** The closed part is the path predicate above —
decidable, and this wire decides it. The open part is *"no other host path or host policy is
named here"*. #501 §12(3) owns that split and this memo does not restate it. ⚠ **What this
memo got wrong in draft 1**: it said no grep decides the open part. True of the *policy*
half; over-broad for the *host path* half, whose live instances (`cli.py`'s `--paths`
default, `refresh.py`'s usage string) a grep does decide. The wire's own header names them.

⚠ **THE PREDICATE FORBIDS SOMETHING ITS OWN AUTHORITY PERMITS**, and this is the sharpest
open question about the instrument. `DESIGN.md`'s closing rule says to *"put elidex policy in
adapter commands **or documentation**"* — and `DESIGN.md` itself, plus adapter-role modules
such as `commands/agent_brief.py`, are **inside the scanned population**. A fixture carrying
that sentence plus one adapter path reds the wire (reproduced in review round 1). The tree is
green today only because no such path happens to be written. A permanent required gate whose
rule contradicts the document it cites is a design defect, not a wording one.

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

## §3 Spec coverage map

**This slice cites no spec section, and that is a property of its subject.** Its predicate is
over repository paths; its authority is `DESIGN.md`'s closing rule, plus the trip-wire
registration convention in `scripts/trip-wires.sh`. ⚠ **Draft 1 also named "CLAUDE.md's
layering mandate", which is wrong** — that section is the VM-host rule (`crates/script/
elidex-js/src/vm/host/`) and says nothing about the generic core; naming it routes a reader
to the wrong review axis.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| (none — §3's prose states why) | n/a | n/a | n/a | n/a | n/a |

⚠ **This row is a placeholder and the gate does not know that.** Measured: preflight parses
it as a row that HAS a `§<number>` (the `§3` in the prose), reports `unmapped-label rows: 1`,
counts it into breadth, and skips webref verification because the label is unrecognised — the
one path through the citation arm that neither verifies nor fails. Strip the section mark and
it hard-fails; use a real label with a fake number and it hard-fails. **Slice A-iii faces the
same situation and takes the honest shape instead**: it declares no spec surface and its
`preflight` exit 1 is by design (umbrella, A-iii's memo row). Adopting that here is §5 item 7.

⚠ **Run it, do not predict it** — draft 1 predicted a hard fail from the shape of the rule
without executing it:

```sh
python3 .claude/skills/elidex-plan-review/preflight.py \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md; echo "rc=$?"
```

It exits 0 with **two** advisories, not one (draft 1 said one): the unrecognised label above,
and an enumeration-claim warning triggered by §2's own cross-document citation. The second is
this memo's citation style colliding with the program's own detector.

## §4 Artifacts

| Artifact | What it is |
|---|---|
| `.claude/tools/webref-generic-core-trip-wire.sh` | the scanner: population from git, content from git, verdict |
| `.claude/tools/webref-generic-core-trip-wire.controls.sh` | the controls: one fixture per verdict the scanner can reach; sourced, never run standalone |
| `scripts/trip-wires.sh` | one line in `REQUIRED_WIRES`, plus the two driver comments its arrival falsified |
| `.github/workflows/ci.yml` | the ungated-job rationale for the `trip-wires` job |
| `CLAUDE.md` | the paragraph restating that rationale, which names the job comment as canonical |
| this memo | the plan-review record — **a tracked file in this slice's diff**, which draft 1's boundary count forgot |

⚠ **What the wire asserts, and why, is stated once — in the wire**, in the comment block
above `_esc`. This table names the files; it does not restate the mechanism.

## §5 What this review must still decide

Review round 1 found that draft 1's agenda was partly mis-posed: some items rested on false
premises, and one had its answer already written into it. Restated:

1. **Which of the four declared blind spots change disposition.** ⚠ Draft 1 said the wire
   *"does neither"* (close nor file). **That premise is false**: #501 §12(3) already disposes
   of three — the policy half as *"Open part, reviewed not gated"*, the whitespace segment as
   *"review-covered"*, and bare top-level names with both live instances named. The real
   question is narrower and harder: **does a permanent, required, ungated gate get to hold a
   review-only obligation at all**, or does registering it in CI change what "filed" has to
   mean? If any becomes a slot it contradicts a memo landing first (#501), so the answer must
   be reconciled with §12(3), not asserted over it.
   ⚠ Also: interpolation and bare top-level names are properties of a grep over arbitrary
   source text. "Closable here" is answerable only as **no** for those two.
   ⚠ And four candidates exceed the per-PR own-deferral cap of 3, which is an input to the
   answer, not an afterthought (see §8).
2. **The threat-model paragraph.** *"The threat model is accident, not adversary — and saying
   so bounds this file"* is falsified by the file's own growth after it was written, and the
   NUL arm it was written to justify **is** a new mechanism; the "existing fail-closed answer"
   it appeals to did not exist until that mechanism created it. **Decide**: restate the model
   to describe what the file covers, remove the arms the model excludes, or delete the
   paragraph rather than let it read as a boundary.
3. **Two controls pass without testing what they name** — not three, as draft 1's heading
   said. `cachedir`'s fixture is tracked before its `.gitignore` is written, so its control is
   green even with the force-add removed entirely (mutation-verified in review round 1); and
   the `odd` note prints *"every other control ran"* unconditionally, in runs where the
   `fifotracked` control has already failed. ⚠ Draft 1 wrote the fix into the item and called
   it a decision. What is actually open: the two already-guarded sites use **two different
   shapes** (a precondition that sets `ctl_ok=1`; a note that does not), and the
   `odd`/`fifotracked` pair needs a third. **Decide** which shape each takes.
4. **The false CI rationale is at four sites, and this slice has already answered half.**
   ⚠ Draft 1 named one site and claimed the item was unfixed in the carried commit; neither
   holds. The claim lives at `ci.yml`'s *"the wires are grep-only"* line (untouched by this
   slice, still false — the wire makes dozens of `git` calls), at the rationale block this
   slice rewrote, at `CLAUDE.md`'s paragraph this slice rewrote, and in the wire's own header
   — the site §4 designates canonical. **Decide** the property the decision rests on, and
   **land it at all four**, including the one this PR has touched the file of but not the line.
5. **The split is a `source`d fragment, not a boundary.** The controls file consumes the
   wire's variables and helpers, writes `ctl_ok` back, and cannot be executed or tested
   standalone — run directly it reports a TMPDIR error rather than "must be sourced".
   ⚠ Draft 1 offered "state the split as a line-count response" as an alternative; **CLAUDE.md
   names that as the wrong basis** (*"line-count の機械適用でなく cohesion 判断"*), so it is
   not an option. **Decide**: a real entry point with explicit parameters, or a re-derived
   cohesion seam, or no split.
6. **Where the controls file lives.** It sits in the directory the driver globs to *discover*
   wires, mode 755, with a name one token short of the discovery convention — so "is this a
   wire?" has two answers depending on which glob is asked, and this memo's own §0 command and
   the driver's disagree. **Decide** whether the distinction is pinned by an assertion or left
   to a naming near-miss.
7. **§3's placeholder row.** Adopt A-iii's declared-no-spec-surface shape, or make the
   empty-map case explicit in the gate's vocabulary. Leaving a row that parses but resolves to
   nothing is the shape the citation arm exists to prevent.
8. **§1.1's fork.** Everything above assumes option (a).

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

⚠ **And this slice's carried commit has already written one answer.** It rewrote that
paragraph to say the wire set needs *"no toolchain, no cache, no network"*, which is the
"no" answer to §6's question, landed on a repo-wide surface while the question is called
open. Draft 1 claimed the six items were *"deliberately not fixed in the carried commit"*;
for this one that is not true.

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

**The decision this slice owes**: may the required, ungated wire set require an interpreter?
⚠ **One candidate answer is already in the program**: umbrella's A-iii row plans an ungated
`tools` job *with* an interpreter floor. Under "one issue, one way" that suggests Python
checks belong in that job and `trip-wires` stays grep + git — which would mean #510's wire
has the wrong home rather than this wire having the wrong rationale. **That is a finding
about another lane's PR and this memo does not decide it unilaterally**; it is raised to
#510 and settled with that lane. ⚠ Note the asymmetry the "no" branch carries: its write path
is in another PR's files, which §7 forbids this diff from touching — so "no" cannot be
executed here, only agreed.

## §7 Exit criteria

1. `bash .claude/tools/webref-generic-core-trip-wire.sh` → PASSED, and
   `bash scripts/trip-wires.sh` → 0. ⚠ This restates the wire's own verdict; it is a smoke
   check, not an assertion about the wire, and is listed as such.
2. Removing the controls file ends the run at exit 2, *"decided nothing"* — verified in both
   directions, since a split that can silently skip its own controls is worse than no split.
3. The mutation set is **enumerated in the controls file itself**, machine-readably, and each
   entry is shown to red when reverted. ⚠ Draft 1 pointed at "each fix this wire's history
   names": that population is prose scattered across commits that live on #501 and will be
   **erased by its squash merge**, and it mixes in refactor commits whose revert reds nothing.
   A criterion whose population disappears when the parent lands is not a criterion.
4. Each of §5's eight items is answered **in this memo**, and each answer says whether it
   changes the implementation, so "answered, status quo" is distinguishable from "unanswered".
5. `git diff --name-only webref-cite-audit-tool...HEAD` matches **§4's table** — no count, and
   §4 includes this memo. ⚠ The artifact set is **not frozen**: §5 items 2, 5 and 6 can each
   change it, and draft 1's version forbade exactly those answers by fixing the set first.

## §8 Defer slots

**None registered, and §5 item 1 decides whether any should be.** The four declared blind
spots are the candidates; #501 §12(3) already disposes of three as review-covered, so a slot
here is a **reversal** of a landing memo, not a first filing.

⚠ **Two constraints the review must carry into that decision.** A slot needs
`Why deferred` + `Re-evaluation trigger` + **`Re-evaluation date`** — draft 1 named only the
first two, and the lane's own precedent records that an undated trigger *"has nothing that
forces a look"*. And four candidates exceed the per-PR own-deferral cap of **3**, while the
cap policy forbids deleting slots to make the arithmetic work — so at least one blind spot
must be genuinely closed, or the cap overflow justified explicitly.
