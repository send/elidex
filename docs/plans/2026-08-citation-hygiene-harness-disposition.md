# Citation-hygiene harness — disposition: one home for the block set, then everything else

**Subject**: what to *do* about the re-derivation harness and [PR #505](https://github.com/send/elidex/pull/505).
**Input**: `2026-08-citation-hygiene-harness-predicate-collapse.md` (the analysis note), which settles the
validity predicate and block ownership and explicitly authorises nothing. Its findings are cited here, not
re-derived.

**Why this memo exists separately.** A design analysis and a PR-disposition plan are two slices, and the
execution plan that once rode with the analysis bundled PR topology, a 762-line removal, harness
self-verification semantics and a CRIT detector fix into one authorised action set without declaring
CLAUDE.md's edge-dense trigger. This memo declares it and answers it: **an umbrella with a forced PR
sequence**, each PR reviewed on its own.

**The reframe that makes the sequence short.** A 762-line removal was only ever *necessary* because the
harness cannot be acted on at file granularity — and once it can, removal stops being a design act and
becomes a one-line consequence whose timing its own PR can decide. **So this memo removes nothing, discards
nothing, and creates no defer slot.** It makes the removal possible, and lets the PR that wants it argue for
it.

**No enumeration here is hand-written where a command can derive it.** `rederive homes` derives the sites at
which the block set is written down, and §3 is a rule per *class of home* applied to that command's output
rather than a list of sites that has to be right. Mechanism landed on this branch ahead of review; that set
is likewise derived, never transcribed:

```bash
git log --oneline --cherry-pick --right-only \
    origin/webref-cite-audit-tool...HEAD -- 'docs/plans/*A-rederive*'
```

⚠ **`origin/`, not the bare branch name.** `webref-cite-audit-tool` is a sibling worktree's *local* branch, so
the bare form is `fatal: bad revision` inside a `git clone --local` sandbox — which is the form §9 designates
as the one every further measurement takes. Every rev this memo hands to `git` is spelled `origin/…` for that
reason; the bare name still appears in prose, where it names the branch and not a revision.

It is D10's command, and D10 says why a plain path-restricted log is not. Falsification on this branch is by
**running**, in three forms: planting into the mechanism; re-running an **acceptance list** (which does not
exist before `fc47cde1`); and **behavioural identity** (`259e12cb` — every block's stdout, stderr and exit
code captured either side). Defects a run showed and inspection did not include the declaration needle
matching the examples in its own comment, and the census dropping `all()`'s own definition line — the roster,
the single most important home. Where a falsification still
constrains a future action it is stated beside the rule it supports (§3), not kept in a ledger, because a
ledger of past changes is a hand-written set with the failure mode of the site lists it replaced. Some of
those commits moved the census output §3 calls the work list, and a commit that moves the work list decides
part of PR-1a's scope. **From this memo forward no mechanism that decides what a PR decides lands on this
branch** (§9).

## §0.5 / §3. Spec coverage map

PR-2 authorises the `citations` comparison, which is a comparison of spec §-titles, so this memo carries the
pairs. **All four §-numbers resolve in webref**; that is a different question from whether the *fixture's*
label resolves in the gate's pinned map, and the distinction is load-bearing.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | §-title compare | **read the fixtures, not this cell** (below) | `citations` | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | §-title compare | fixture `labelled` | `citations` | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | §-title compare | fixture `alias` | `citations` | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | §-title compare | fixture `allunmapped` / `malformed` | `citations` | ✓ | no |

**Breadth**: K=3 specs, M=4 entries — the complete set, and the command finds the call site rather than this
table naming a line number PR-1 is about to move:

```bash
grep -n 'webref heading --exact' docs/plans/2026-07-citation-hygiene-A-rederive-*.sh
.claude/tools/webref heading --exact html 4.10.21     # and 4.10.21.2 / fetch 2.2.5 / cssom-view-1 4.2
# WHICH FIXTURES CARRY EACH PAIR -- materialise them; do not read it off the table:
bash -c '. …-A-rederive-integrity.sh; . …-A-rederive-common.sh; fixtures /tmp/fx'
grep -lF '§4.10.21 Constraints' /tmp/fx/*.md
```

⚠ **The Branch column names fixtures, and a fixture is not the same string as the spec's own label.** The
Spec column carries each spec's authoritative label; a fixture deliberately carries a different one, because
aliasing is what the fixture set exists to exercise. The two columns must not be read as one string, and the
first row's fixture set is materialised by the command above rather than named here. Measured on the labels
the fixtures actually write — `WHATWG HTML` and `HTML` resolve, **`Fetch` and `CSSOM VIEW` do not**, which is
the state those fixtures exist for rather than a defect:

```bash
(cd .claude/skills/elidex-plan-review && python3 -c "
import preflight as p
for lab in ['WHATWG HTML','HTML','Fetch','CSSOM VIEW']: print(lab, '->', p.shortname_from_label(lab))
print('pinned-map keys:', len(p.SPEC_LABEL_REVERSE))")
```

⚠ **The harness's own comment beside the `allunmapped` fixture calls it a "24-key pinned map"; the command
above measures a smaller number.** That comment lives in **`fixtures()`**, not `citations()`. Correcting it
is PR-2 (§5), at the block that holds it.

⇒ **PR-2's comparison covers all four pairs**, including the row whose fixture label does not resolve: the
harness records that this fixture's §-title was corrected **from a fabrication**, and that `verify_citation`
checks only that the number exists, *"so nothing would catch it"*. Restricting the comparison to pairs whose
lookup succeeded would exclude exactly the row the comparison exists for. The constraint underneath is
`_measure`'s: **a lookup that fails is a failed measurement** and must never be reported as a matching title.

## §1 Measurements

**Moved.** The D-entries live in `2026-08-citation-hygiene-harness-measurements.md`, cut out as a standalone
prereq when this memo passed 1000 lines; that file's own header states the seam and records that the cut was
late. Citations are unchanged — `D<N>` resolves there, and `rederive homes` reds on a `D<N>` no bullet
defines. This heading is kept so that the section numbering §2 onward, which both memos cite, does not move.

## §2 Coupled invariants

Required because the work is edge-dense (`/elidex-plan-review` Pre-condition #3).

- **I1 single home** — one fact (which blocks exist, and whose each is) is written down once.
- **I2 declared, then checked** — a block's group is **stated in its body**, and a **file's group is stated
  in its preamble**; the tiers compute a second answer and disagreement is reported. The declaration is the
  authority because it is the one input a rename, a memo edit and a file split all leave alone, and because a
  construction cannot be the authority when the construction is what is under review (D4).
- **I3 removal safety** — deleting a part removes its blocks from every derived set automatically.
- **I4 derived input** — the harness's own inputs are derived or declared, never parsed out of prose; and
  **the enumeration of where they live is derived too** (D9).
- **I5 fixed invocation surface** — every block name keeps resolving through the dispatcher path regardless
  of which part defines it. The dispatcher's own header states a memo count that M1 falsifies; that stale
  header is one of the homes §3 collapses.

| pair | intersection | PR |
|---|---|---|
| I1 × I2 | The declaration is the single home only if nothing else can *decide* the answer: `ship` is the declaration where there is one, and the tiers are the check. Before that, the declaration appeared nowhere but the mismatch line while the column, the tally and the move list printed the computed value. | done |
| I2 × I3 | A declaration must be **independent of the filename**, or renaming a file changes what the harness believes about the blocks in it (D4). | done |
| I2 × I4 | A disagreement is only as good as the tier behind it. T0/T3 are code signals and **bind**; T1 is a heuristic parse of prose on a branch under active revision — a failed measurement by `_measure`'s own rule — and **reports**; T2 is the filename, which the move list already checks. This is what stops a memo edit from turning the harness red. | done |
| I1 × I4 | The list of homes must be derived, or a plan against it is complete only by luck. `rederive homes` (D9). | done |
| I1 × I3 | The roster must be **derived from the definitions**, not listed; deleting parts with the roster untouched makes `selfcheck` accuse blocks that no longer exist. | **1a-i-α** |
| I1 × I5 | Excluding helpers by `_`-prefix requires renaming `say`/`fixtures`; D2 shows the citation surface that breaks is this memo's, so the rename and the memo edit are one PR. | **1a-i-α** |
| I2 × I5 | A part **rename** must not change a block name. The dispatcher resolves by name across all sourced parts (D1), and once I2 holds a rename can no longer change the routing answer either. | **1a-i-α** |
| I2 × I3 (files) | A file's group must be **declared, not read off its name**, or the move list compares a declaration against an inference and the two roles the group vocabulary plays — *whose concern a block is* and *which file holds it* — stay fused in `PART_SLICE`. That fusion is what left `umbrella` with no destination and dropped `kernel` from T3's boundary test. D12/D13. | **1a-i-γ** |
| — | PR-1a-ii writes declarations into checks PR-1a-i has already made correct. It opens no intersection of its own. | **PR-1a-ii** |
| I4 × I5 | The tiers still read the memos (T1), so `inventory` still needs a checkout that has them. That is **pre-existing** — M3 lists `inventory(exit 1)` among seven REDs of one class — and §4 turns on it. | §4 |

⇒ **The intersections marked `done` are already discharged, and that is why PR-1 can be split.** The seam is
**step 1's output**: the move list does not exist until the declarations do. So PR-1a collapses and **PR-1b
moves**; and one step earlier, the declarations cannot be *read* until the part set, the tiers and the
classifier are what §3 says they are, which is the PR-1a-i / PR-1a-ii seam.

⚠ **PR-1a-i did not meet CLAUDE.md's single-intersection base case, and it is re-sliced here rather than
sent to its own plan-review to be split there.** Earlier drafts routed the remedy downstream. CLAUDE.md
assigns the partition to the **umbrella**, and its stated purpose — map the edge matrix up front so the
review tail is pre-empted — is not served by sending a four-intersection slice to review expecting the
review to cut it. The accumulation was measured: mentions of PR-1a-i's obligations rose monotonically in
every draft since the carve, which is several times the threshold at which a lane's own notes say the
**boundary** is the thing to suspect.

**The five slices, and the order is measured, not preferred.** ⚠ This table said *three* while listing
four, from the draft that added `δ`; the count is now stated as the row set it heads.

| slice | what it owns | why it is one thing |
|---|---|---|
| **1a-i-β** the classifier | **`classify`'s subject test for a derivation call — every command position, both spellings** (§3's `callsite` and `mention` rows carry the rule) | it is one predicate, it changes **how the census classifies a line**, and it changes no answer about where a block ships. ⚠ **The coverage gate's content test was in this row and leaves it for **ε**, both halves** — β's own `/elidex-plan-review` measured that the *per-class* family of predicates is not constructible without rewriting rows β does not own, and a later measurement found the *non-per-class* half constructible after all (see §3's coverage-gate paragraph, which carries both commands). The two halves are one gate and land in one edit; β lands no coverage-gate change either way. ⚠ **What remains is not "the command-position predicate" in the narrow sense.** Measured, planting α's crossing two ways: unquoted (`… $(_partset) $(_roster) <<'INVENTORYPY'`) it classifies `?` and the census is RED at rc=1 — the failure β exists to fix; **quoted** (`… "$(_partset)" "$(_roster)" …`) it reaches `mention` first (`-audit.sh:151-152`) and files a genuine home under the one class whose rule is *nothing to do*, at `9 of 9`, **rc=0**. The quoted spelling is the likelier one, and it is silently green. So β owns `mention`'s command-substitution hole as well, and **`callsite` and `mention` stop being "nothing to do" rows** |
| **1a-i-α** the set | I1 × I3, I1 × I5, I2 × I5; the part-set derivation and its crossing; `memoset`; `authorlocal` | all of them say **one fact, one home** about *names and sets*; none changes a tier's answer |
| **1a-i-γ** the routing | I2 × I3 (files); the T0/T2 collapse into one file-declaration tier; the undeclared-file RED rule; `unverifiable=`; D13's tie-break; `route = dict(decl)` | all of them change **what answer the harness gives about where a block ships** — and every deferred decision, every conditional cascade and the only change this memo flags as not behaviour-neutral is here |
| **1a-i-ε** the subject test | the `prose` subject test; the three rows that replace `prose` in §3; and **the coverage gate entire** — `covgate`, both directions: `ruled - set(byclass)` reported rather than dropped, **and** the non-triviality clause §3 measured constructible after an earlier draft withdrew it. ⚠ **This row said "reverse direction" alone while §3 said both halves and §3a assigned `covgate` wholesale** — three statements of one scope, two of them created in the commit that made the re-assignment. Placed here because ε's own `prose`-row removal is the first event either direction can catch | it is the only classifier change that needs a **vocabulary** rather than a predicate — the part set for its place tokens and `GROUPS` for its group tokens — and **both reach it only through α's crossing**. ⚠ **It was inside β until β's own `/elidex-plan-review` measured that it cannot be**: `GROUPS` already exists at `-inventory.sh:168`, inside `INVENTORYPY`, while the test lives in `-audit.sh`'s `HOMESPY`, so γ collapsing the group vocabulary does not put it in scope — only the argv crossing the `roster` row assigns to α does. The two escapes were measured and both fail: spelling a second `GROUPS` in `-audit.sh` is the many-homes defect γ exists to close (and filing it under `CLASSES` reports the duplicate as *ruled* at rc=0 rather than `?`), and dropping the group half of the predicate lands every group-object row in `proseunsettled`, which is RED while non-empty — so α would inherit a red census, which is the very thing β's ordering argument exists to prevent |
| **1a-i-δ** the guards | `reads` — a named failure on every read of the harness's or a memo's text | ⚠ **It was assigned to no slice at all when the partition was first drawn, and §3a counted it as an obligation, so the two sections contradicted each other.** It is its own slice because it fits none of the other three predicates: it changes no classification, no set and no routing answer. It is also the only slice with no ordering constraint — nothing reads its output — so it may land at any point, and that independence is the evidence the partition is right rather than an excuse for the leftovers |

**β before α**, because the crossing α builds cannot be classified by the census as it stands: `classify`
inspects only a line's first word for command position, so a derivation called from an argument position
falls through to `?` and the census goes RED — measured, and it is the same edit either way, so it belongs
to the slice that owns the predicate. **α before γ**, because the tier collapse requires the dispatcher to be
in the part set, which is α's work. **γ before ε**, because ε's group tokens are γ's single `GROUPS`, and
**α before ε** because ε reads both its vocabularies across a process boundary that only α's crossing opens.
**δ is unordered** — nothing reads its output.

⚠ **That measurement is what this ordering argument used to be missing, and it is why ε is its own row.**
The `?`-at-rc=1 result above is a fact about the **command-position predicate**, which is β's; it was read as
a fact about the whole of β, and the `prose` subject test was carried along in front of the two slices whose
output it consumes.

Two things fall out of the partition rather than being argued into it. The **two conflicting "firsts"** —
`partset` lands first, and the `prose` split is the first task — stop conflicting, because they are first in
different slices. And the **T0 decision stops being a partition question**: both branches lived inside γ, so
what earlier drafts deferred as scope-deciding is a γ-internal design choice, and it is decided above on
measurement rather than left to an implementer.

The base case is *a slice that has **passed** its own plan-review*, not one that is merely required to take
one, so this umbrella still cannot discharge terminality in advance; each of β, α, γ, ε and δ takes its own
`/elidex-plan-review`. §9 authorises the scope, not its terminality. (`axes.md`'s wording —
*scope が単一 invariant-axis 交点に絞られている場合* — is narrower on **scope** than CLAUDE.md's and omits
the passed-review condition, so the two are not ordered; both must hold.)

## §3 The mechanism, and the declarations

⚠ **This section was headed *PR-1a-i — the mechanism, and PR-1a-ii — the declarations*, and both names were
dissolved by the re-slice.** The rule rows below are apportioned to β, α, γ, δ and ε by §3a; where the prose
still says *PR-1a-i*, it is describing what was decided when that slice existed, and §3a is the live
assignment. The two are not interchangeable: a row's rule is stable, its owner is not.

**Goal**: after PR-1a, the block set has exactly one home per class, and file-granular action is sound.
**PR-1a-i** makes every derived fact below correct with zero **block** declarations written; **PR-1a-ii**
writes them. The rules are stated together because they are one design; the split is where they land.
Elsewhere in this memo **"PR-1a" names the pair**.

Most of that mechanism now lands in the **inventory** part — the part-set derivation, `PART_SLICE`'s retirement and the tier changes all moved there — and **the seam PR-1a-i was assigned has already been cut** —
the memo-quantity gate took that file past 1000, which is CLAUDE.md's standalone-prereq trigger. ⚠ **That is
where the trigger came from, and §9 records it as a violation this branch caused rather than a licence** —
the gate commit is one this memo's stopping rule permits, and it is what crossed the threshold; the cut was
already late when it happened. Sizes are a command, not a figure here:

```bash
wc -l docs/plans/2026-07-citation-hygiene-A-rederive*.sh
```

The record `memory/feedback_touch-time-split-means-while-writing.md:41` asks for, rather than a disposition
that answers it away: the file entered the 700–800 band at `979e5426` and left it at `49b4f645`, both on this
branch, and no seam was cut at either (`for c in 979e5426 49b4f645; do git show $c:docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh | wc -l; done` → 778, 808).
`:39` keys its prescription to **reaching** the band, so the cut was already late when it happened.
⚠ This record is **historical, and `BAND` cannot check it**: the needle measures the working tree and keys to
no revision, so its population over this claim is zero by construction, not by oversight — which is why the
command above is stated beside it rather than a shape the gate would match.

Two things survive the cut and are still PR-1a-i's. **The criterion**, because any further part-file move
faces it — ⚠ **and the form earlier drafts gave it is false.** They said *`defined=` and `all`'s roster
unchanged, read from `rederive inventory` either side*. Measured: split `regions` out of `-Ai.sh` into a new
glob-matching part, update the dispatcher's bootstrap loop and deliberately not `-inventory.sh`'s hardcoded
`PARTS`, and **both halves hold** — `defined=` 35 → 35, `roster=` 24 → 24 — while `regions` has silently left
`inventory`'s table. An addition elsewhere masks the removal in the count, and `all`'s roster is a literal in
the dispatcher that a part-file move does not touch. **The criterion is the named block set, not two
counts**: no block may leave the table. §3's byte-identity exemption cannot serve either — it exempts `all`,
`homes`, `inventory` and `selfcheck` **by name**, and their output is the only witness a split has. Green
gates are not the criterion: omitting the hardcoded-`PARTS` update drops `defined=` by one while every gate
stays green at rc=0. **And the new part's
`# group:` preamble**, which `groupvocab` assigns to PR-1a-i along with every other part file's.

**The work list is `rederive homes`, and so is the list of classes.** The mapping from home to class lives in
the census, it is **total**, and an unclassified home is **RED** — falsified by planting an unruled literal.
Below is the *rule* per class, and the class names are the census's.

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes   # `BY CLASS:` names every class
```

⚠ **What the coverage gate buys is narrower than `RULED BY THE PLAN: 9 of 9` reads.** Two things are wrong
with it and both are PR-1a-i's. **(a) It gates on a row's key, not its content**: replacing all nine rule
cells with `TBD` still prints `9 of 9` at `rc=0` — the same defect as *a class with no rule*, one altitude
up. The `prose` row was the live instance until its subject test was written; the class it splits into
supplies the next one, because `proseunsettled` must be RED while non-empty and a key-only gate cannot say
so. **The section scope this paragraph used to assign alongside it has landed** — `b088dacc`
scopes the extraction to `^## §3 ` (`-audit.sh:273`, extracting at `:277`).
⚠ **The content test is NARROWED, not withdrawn — an earlier draft withdrew it on a claim whose universal
half is false.** β's `/elidex-plan-review` implemented every candidate predicate over a rule cell's *text*
and ran each over these nine rows. What it measured, and what holds, is about **one family**: predicates that
ask a cell to cite **its own class's evidence**. Keying the cell's citation to the class it rules fails on
**four** rows (`authorlocal`, `reads`, `mention`, `callsite`), because the only identifier→class map in the
census is `CLASSES` (`-audit.sh:108-109`), five keys over four classes; keying it to a census *row* of that
class fails on **six**; and admitting "the class name itself" as a satisfier makes the predicate satisfiable
by typing the class name, which is the shape rule with no subject test that the rows themselves are supposed
to avoid. Two of the four failures are rows whose slices are α and δ, so no slice can land **that family**
green within its own authorisation. **The rule rows do not systematically cite their own class's evidence
and there is no reason they should**; a per-class predicate over their text is measuring authorship habits,
not rules.

⚠ **The withdrawal generalised that to "a predicate over a rule cell's text is unconstructible", and the
complement was never measured.** It is constructible, and the constructible form is the one defect (a)
actually names — a cell that carries **no rule at all** — which is not a per-class question. Measured, this
predicate is fifteen lines and separates the two states the paragraph above contrasts:

```bash
python3 - docs/plans/2026-08-citation-hygiene-harness-disposition.md <<'PY'
import re, sys, pathlib
s3 = re.search(r"^## §3 .*?(?=^## §|\Z)", pathlib.Path(sys.argv[1]).read_text(), re.S | re.M).group(0)
fail = []
for name, body in re.findall(r"^\| \*\*([a-z]+)\*\* \|(.*)$", s3, re.M):
    cell = body.rsplit("|", 1)[0] if body.rstrip().endswith("|") else body
    stripped = re.sub(r"\b%s\b" % re.escape(name), "", cell).strip()   # the key is not content
    n = len(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", stripped))
    print("   %-12s words=%d" % (name, n))
    if n < 8:
        fail.append((name, n))
print("rows=%d failing=%d %s" % (len(re.findall(r"^\| \*\*[a-z]+\*\* \|", s3, re.M)), len(fail), fail))
sys.exit(1 if fail else 0)
PY
```

⊕ Measured at this head, by running the fence above: `rows=9 failing=0`, rc=0. ⊕ Measured with the
`callsite` cell replaced by `TBD` in a throwaway clone —
`d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"`, edit the row there, re-run the fence — it gives
`rows=9 failing=1 [('callsite', 1)]` at rc=1, **while
`bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes` on that same tree still prints
`RULED BY THE PLAN: 9 of 9 class(es)` at rc=0.** That is defect (a)'s own example, caught.

⚠ **The floor is anchored to a printed distribution rather than chosen**, which is why the command prints
every row's size and not just the verdict. ⚠ **The distribution is not written here.** Two drafts wrote it out and both were stale on landing — the
`prose` cell alone has read 441, 495 and 547 as this session's own edits touched its row — which is why the
command prints every row's size. A stub of the shape defect (a) names measures **1**, and the smallest real
rule measures well over an order of magnitude more; run the fence for today's spread. The floor sits in that
gap with a factor of four below the smallest real rule, and a rule row that ever reds this gate raises the
floor **with the re-run distribution beside it**, not by argument. ⚠ **What it does not buy**: it cannot tell
a rule from prose of the same size, and it is blind to a cell that is wrong rather than absent. It closes
defect (a) as stated — a key with no content — and nothing wider.

**Both surviving halves are ε's, and they are one gate.** The other direction — reporting
`ruled - set(byclass)`, a rule row for a class the census no longer emits — is empty today and first has a
job when **ε** retires `prose`. The non-triviality clause lands beside it, in the same edit, for the same
reason ε is where it belongs: ε **replaces the `prose` row with three**, which is this program's largest
single injection of new rule cells, and those three are exactly the cells a key-only gate would pass empty.
Splitting one gate's two clauses across two slices is the decision surface CLAUDE.md *One issue, one way*
is about. ⚠ **The ordering cost is stated rather than discovered**: α and γ each rewrite rule cells **before**
ε lands the guard, so for those two slices defect (a) stays open, and their `/elidex-plan-review` is the only
thing standing over their cells. β does not take it either — β's whole slice is one predicate inside
`classify`, and the gate is not a classification. (**This program already owns a machine check over this memo's own claims — the gate
above is one**, and since `b088dacc` it also checks this memo's **quantities**: `-audit.sh:265-281` reads this
memo and fails closed if it can read none of §3's rule rows, citing the same
`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md` the claim-gate lane's checker cites. ⚠ **That gate
had a population defect of its own and `96d8fae3` fixed it; this records the fix, not the state.** The
population print was written on the match branch — `_pop` at `-audit.sh:366-367`, joined at `-audit.sh:486-488` — so a
needle matching nothing contributed **no key** and was absent from the line rather than printed as `=0`, which
is to say the property it was landed for — *"a needle matching nothing reports clean for the wrong reason"*,
`-audit.sh:299-300` — did not hold. Draft 13 declined the fix on the ground that it would shift `homes`'s
line numbers a second time; round 13 falsified that on three axes with the same edit. `96d8fae3` seeds `_POP`
at the initialiser it already shares with `_CBAD`/`_CLIM`/`_rv`, so no cited anchor moves. ⊕ The fix holds at
this head — both keys print, and the command is the gate itself:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep POPULATION
```

⚠ **The two needles' populations are `band claim=0` and `stated length=1`, and a zero is not a clean bill** —
it is this gate reporting that it checked nothing, which is the whole reason the key must print. What remains
owed here is the narrowed content test, and it is **ε's**.)

| class (the census assigns it) | rule |
|---|---|
| **partset** | one derivation from the glob; every reader calls it — **and within **slice α** it lands first, before either declaration rule**, because both of them read it. **This is the rule that makes `all` declarable, and the site is `inventory`'s hardcoded `PARTS = ["integrity", "audit", "inventory", "common", "Ai", "Aii", "Aiii", "B"]` (`-inventory.sh:45`)**, from which `srcs`, `defline` and the stray counter are built inside the `INVENTORYPY` heredoc: the dispatcher is not in that list, so `all` is in neither `srcs` nor `defline` and a declaration on it is read by nobody and reported by nobody. (`homes`'s `PARTFILES` needle at `-audit.sh:59` is a *different block in a different process*; it governs the census, not declarability.) ⚠ **The part-set fact has two incompatible consumer kinds and the rule must state both.** `homes` sources each part file (`bash -c "set -e; . <file>; declare -F"`, `-audit.sh:66-68`) and `inventory` sources them separately, with `declare -f`, at `-inventory.sh:56`, and the dispatcher's last line is `"${1:-all}" "$@"`, unguarded — sourcing it runs the whole suite recursively (measured: hundreds of processes before the run was killed). So the derivation is a **text set that includes the dispatcher** and a **source set that excludes it**. ⚠ **And a third consumer can call neither**: the census's other `partset` home is the dispatcher's own bootstrap loop (`A-rederive.sh:52`), which runs *before anything is sourced*, so it cannot invoke a derivation defined in a part file. **It is bound to the SOURCE set** — it is the loop that does the sourcing, and it excludes the dispatcher trivially because it *is* the dispatcher. It resolves by sourcing `-integrity.sh` **by name** ahead of the loop and taking the derivation from there — which the dispatcher's own header already treats as invariant (*"sourced FIRST because everything else reads it"*, `:44-49`). Saying so is the rule's job: the available wrong reading — inline a second glob at `:52` — re-creates exactly the two-homes defect this row exists to close. ⚠ **The other two consumers are python inside heredocs and cannot call a bash derivation at all**, so the rule must state the crossing — and the **`roster` row states it once, for both rules**: PR-1a-i *builds* one crossing rather than reusing `homes`'s. This row consumes that rule and does not restate it. Two further consequences: the stem derivation `f.name.split("A-rederive-")[1][:-3]` (`-audit.sh:65`) raises `IndexError` on the dispatcher, so admitting it to the text set means changing the stem derivation too — **and the stem it is given is a decision, not a by-product**. `PARTS` feeds `VOCAB` (`-audit.sh:73`), so a new stem is a new vocabulary token, and a new token turns previously-invisible lines into **homes** — a home `classify` cannot place returns `?` and the census goes RED. Measured in a sandbox, one stem per run: `rederive` and `all` both red at `MISSING: ?`, rc=1, by **two different routes** — `rederive` makes `-audit.sh:59`'s own `PARTFILES = […]` visible, an ALL-CAPS literal whose name has no row in `CLASSES` (`-audit.sh:108`), while `all` makes `-inventory.sh:230` visible, a code line the shape rules place nowhere. ⚠ **Both routes were re-run at this head and both still go RED, but every site this sentence once named has moved**: the `all` route's line was `-audit.sh:527` when it was written (the memo carried the *sandbox's* number, which the plant had shifted by one), and the `rederive` route is now **two** homes, `-audit.sh:59` and `-audit.sh:307` (`HN`), the second created by `b088dacc` — so `CLASSES`'s write-site pressure grew by one after this row was written. `disp`, `dispatcher`, `main` and `kernel` each leave the census at `9 of 9`, rc=0. **`CLASSES` is a write site PR-1a-i must consider**, and the stem is chosen **against the census**, not assumed — the two red routes differ, so no single rule about `CLASSES` clears a stem on its own. ⚠ **T0 is decided, and it is neither of the two branches earlier drafts posed.** The question was put as *keep the sentinel or delete it*; measured, **both are wrong**, and the answer is that T0 and T2 are the same tier and collapse into one. `part["all"], prose["all"] = "(disp)", …` at `-inventory.sh:106` (note: **two assignments on one line**) is read at `:273`. Measured in a sandbox, dispatcher admitted to the part set as stem `disp`, `# group: kernel` on its preamble and `# ships-with: kernel` on `all`:

- **Keeping it** catches a wrong *block* declaration (`!! all declares A-ii but the tiers compute kernel (T0 dispatcher)`, `DISAGREE=1 (binding 1)`, `inventory` rc=1) but is blind to a wrong *file* declaration: T0 is `if rows[b]["part"] == "(disp)": comp[b] = "kernel"`, a constant that reads no declaration, so planting `# group: A-ii` on the dispatcher leaves `agree=1`, rc=0. ⚠ **The argument that T0 "reproduces the declaration by construction" is therefore false** — it reproduces nothing; it is a constant. Two reviewers advanced that argument independently and running it refuted both.
- **Deleting it** moves `all` onto `T3 inventory`, whose sole edge is a Python regex inside the quoted `INVENTORYPY` heredoc (`-inventory.sh:82`) — which `at_command`'s own docstring calls shell-opaque. The dispatch-edge rule then correctly silences it, and **that un-gates `all` entirely**: the same wrong-declaration plant goes rc=1 → **rc=0**.
- **The tier reads the file's declaration instead of being a constant, and is keyed on every part rather than on the dispatcher.** Measured: the `part[…]` half of `:106` deleted, `Tfile` answering each part's own `# group:`. `all` reports `kernel Tfile disp declares kernel`, rc=0; a wrong block declaration is binding RED; a wrong **file** declaration is binding RED (`!! all declares kernel but the tiers compute A-ii (Tfile disp declares A-ii)`) — the hole keeping T0 leaves; and planting `# group: A-ii` on `-Aii.sh` moves all ten of its blocks onto the same tier **with no `ships` value changing**. `grep '(disp)'` then returns nothing. ⚠ **So T2 is this tier**: T2 is `PART_SLICE[part]`, a hardcoded filename→slice map, and `Tfile` is the same lookup against a declaration. Collapsing them is the same move `_misplaced` already makes, and `all` stops being a special case.

Three consequences, all measured, all this slice's:

1. ⚠ **`groupvocab`'s "a part file with no `# group:` line is RED" is load-bearing for the tier, not adjacent to it.** With the declaration absent the tier falls through and `all` lands back on the heredoc regex, reporting agreement on non-evidence at rc=0 — measured both without and with the dispatch-edge rule. One deleted comment line silently turns the whole thing off. The RED rule lands in the same edit.
2. ⚠ **The file tier sits BELOW T1, and an earlier draft of this row got that backwards on a measurement taken too early.** The measurement that suggested outranking T1 was taken with **one** part file declaring, where seven blocks moved off `T1 declared` with identical values and nothing visible changed. But γ's own rule makes an undeclared part file RED, so at γ's completion **every** part declares — and measured in that state, **six of thirty-five blocks change their computed group**: `_wtscan`, `citations`, `couplings` from A-i to kernel, `budget` and `lanes` from umbrella to kernel, `suites` from umbrella to A-iii. With PR-1a-ii's declarations planted those are binding disagreements. The reason is the design: the file tier answers **which file holds it**, which is T2's question, and T1 answers **whose concern it is**. Ranking file above concern is exactly the fusion I2 × I3 (files) exists to break, and it is why T2 sits below T1 today. `-common.sh` settles it on its own — it holds umbrella, A-i and kernel blocks, so no single file declaration is right for all of them. **The collapse is unaffected**: T0 and T2 are still the same question and still collapse into one tier; that tier simply keeps T2's rank.
3. **`route = dict(decl)` is orthogonal and survives.** With a wrong declaration on `all`, `say` is silently re-attributed to `A-ii` and the ships-with roll-up moves with it, under every branch above. That hazard is this slice's too, and the collapse does not touch it. ⚠ **One spelling, not two**: every glob site already writes `…A-rederive*.sh`, which matches the dispatcher, so `selfcheck`'s *"N harness parts"* counts it; the exclusions are in the **derivations**, not the glob. |
| **roster** | `_roster`, a **function**, returning every non-`_`-prefixed, non-author-local, non-`all`, **non-argument-taking** definition; `all`, `inventory` and `selfcheck` **call it** (D5: an inline expression makes its readers parse its own shell tokens as block names). The argument-taking exclusion is not optional — D3 measures the derived set at three names above the literal, and `readers` is excluded by no other term; `all` would invoke it argument-less and it returns 2. Two of the three readers are python inside a heredoc and cannot call a bash function directly; ⚠ **the crossing this row used to point at belongs to none of them.** `-audit.sh:66-68` is `homes`’s, and it spells `declare -F` (names); `inventory` crosses at `-inventory.sh:56` with `declare -f` (bodies) and builds its source list **from** the hardcoded `PARTS`, so it consumes the part set and cannot supply it; `selfcheck` has no crossing at all and is the regex this rule replaces. PR-1a-i therefore **builds** the crossing rather than reusing one, and builds it once — a crossing re-spelled inside each heredoc is the many-homes shape this row exists to close. ⚠ **The transport is `argv`, and saying so is the rule's job** (the sibling `partset` row sets that standard): all three payloads open with *quoted* heredocs (`<<'HOMESPY'`, `<<'SELFCHECKPY'`, `<<'INVENTORYPY'`), so no python is shared between them — but each payload's *enclosing bash block* can call `_roster` and pass the result in `argv`, which **all three** already take (`-audit.sh:53`, `:538`, `-inventory.sh:39`). Measured in a sandbox: `selfcheck` ran with its roster supplied that way. ⚠ **And there are three readers of `all`'s roster, not two.** `-inventory.sh:76` and `-audit.sh:566` carry byte-identical needles and both `raise SystemExit` on no match. The third is `-audit.sh:162-168`'s `rosterspan`, consumed at `:221` as `rosterspan.get(f.name)` — a silent `None`, which the `reads` row of this very table forbids, and which is the **only input** to `classify`'s `roster` branch. Measured: with the literal replaced and that reader left behind, `rosterspan` empties silently, the `roster` class disappears from the census, and `homes` still exits 0. ⚠ **`_roster`'s own home is a decision, not a by-product**: it must resolve for `all`, `homes`, `inventory` and `selfcheck`, which puts it in `-integrity.sh` — whose header states its charter as *the measurement primitive and nothing else* (`-integrity.sh:7-12`) and records that whole-harness checks were moved out for being consumers of it. PR-1a-i widens that charter explicitly or picks another home; it does not drift into one. `say` → `_say`, `fixtures` → `_fixtures`, **and this memo is updated in the same PR** (D2 — the note carries no citation of either). |
| **authorlocal** | a registration **adjacent to each definition**, carrying its reason, as a **shell statement** — `_roster` is a bash function and needs it as runtime data, and bash cannot read comments. (`declare -f` is not the reason: the ship-with needle is a Python regex over raw source and never passes through it.) `readers` is registered here too, with *takes a required argument* as its reason, closing the defect A-i §15 records. |
| **groupvocab** | one `GROUPS` set, validated — **and the part files join the blocks in declaring against it**: each part's preamble carries `# group: <g>`, **and so does the dispatcher** (`kernel`; without it `all`'s file side is an assignment no rule has made). `_misplaced` reads that instead of `PART_SLICE`, and `PART_SLICE` retires **here, in PR-1a-i**. ⚠ **The retirement is not scoped to `_misplaced`**, which is one of **three** readers across five sites and not the load-bearing one. The load-bearing reader is **T2** (`-inventory.sh:277-278`); the third is the `home` column of the MOVE LIST report (`-inventory.sh:412`), unnamed until here and needing a replacement source in the same edit or PR-1a-i lands a `NameError`. Re-sourcing T2 off the file declaration is *not* behaviour-neutral (it flips `_wtscan` A-i → kernel, one of D11's own movers); **T2 is dropped**, because once `_misplaced` compares block-declaration against file-declaration, T2 is literally that same comparison. Four further consequences, all PR-1a-i's: **a tier with no evidence must stop answering** (D14 — `unverifiable=`, without which dropping T2 turns nine unconfirmable declarations into silent agreements), on the rule that **a dispatch edge is not a call edge** — ⚠ **and the T0 question this used to pose as an either/or is **decided** in the `partset` row: T0 and T2 collapse into one file-declaration tier, so `all` keeps a real answer, `say` keeps a real `route["all"]`, and neither orphans.** With `-inventory.sh:106` gone, `all` routes `T3 inventory` (measured): its only command-position caller edge is a Python regex string inside `inventory`'s heredoc, which `at_command`'s own docstring calls shell-opaque, so left alone `all` reports as *agreeing* on the strength of a regex — and once the heredoc payload is excluded on the same ground as a dispatch edge, `all` has no evidence and stops answering. **`say`'s only caller is `all`** (`A-rederive.sh:95`; measured, `say` is the sole block whose entire caller set resolves to `kernel`), so `route["all"]` is its whole tier input. When that input stops answering, **`say` must return no answer too** — it must not fall through to `-inventory.sh:304`'s `comp.setdefault(b, "kernel")`, labelled *"T3 caller cycle"*, which manufactures exactly the `kernel` that agrees with `say`'s own declaration. That is verbatim the defect D14 exists to remove, reintroduced at another block by the same edit. **How many blocks `unverifiable=` ends up holding is read from the block afterwards, not predicted here** (§8). ⚠ **And it takes D13's only witness with it**: `cs == ["kernel"]` arises at `say` *because* `route["all"]` answers `kernel`, so if `all` stops answering that instance is gone. The `min()` tie-break total over `GROUPS` is then justified by its **rule** — any caller resolving to `kernel` raises `ValueError`, since `kernel` is not in `ORDER` — rather than by `say` as its one measured case; T3's boundary test ranges over `GROUPS`, **with the tie-break widened in the same edit** or it raises (D13); a T3 disagreement must **name the caller declaration it rests on**, because `route` is seeded from the declarations and one wrong entry otherwise reports as a binding disagreement against its correct neighbour; and **a part file with no `# group:` line is RED**. Today such a file is *reported* but its blocks simply stop being compared — `MOVE LIST` and rc are unchanged — which would make PR-1b's exit criterion satisfiable by an undeclared file (§3b). |
| **memoset** | the census reports **four** homes, not the two this row used to name: `inventory`'s `MEMOS` (`-inventory.sh:51-52`), `budget`'s `for m in …` loop (`-common.sh:507`), and `-inventory.sh:203`'s path builder, which is a consumer the row never reached. One derivation; both readers call it. ⚠ **A "class count is 1" check would be right for the wrong reason**: measured on a landed collapse, the census reports `memoset=1` while the surviving row is the *consumer* and the derivation's own body is not classified `memoset` at all. ⚠ **And a literal-gone needle tests a spelling** — measured, `^MEMOS = \[` still matched `MEMOS = [(g, s) for g, s in MEMOSET …]`, i.e. it reported the enumeration present when what was present was a comprehension over the derivation. The test is structural: exactly one block carries the derivation, and every named reader calls it in command position. |
| **reads** | every read of the harness's or a memo's text must have a **named failure**. The class's size and its guarded/unguarded split are read from `homes`, not restated here; note that `_measure … \|\| failed=1` **is** a named failure, so a `guard` column that does not know the validity primitive under-reports it. |
| **prose** | a prose home becomes a pointer to `rederive homes` / `rederive inventory`, or is deleted — ⚠ **and the subject test its siblings got is now written and measured, so the rule applies to the rows the test admits and to no others.** The test asks the siblings' kind of question — `callsite` asks about command position, `mention` about quoted spans — namely **does this sentence bind block names to a place**: a mapping arrow or colon in a part preamble, a residence relator (`are in`, `lives in`, `placed with`, `have callers in`) whose object is a part or a group, a deictic *lives here*, a `<file> holds`, a file-name copula, or an existing `rederive` pointer. ⚠ **The unit is the sentence, not the line** — measured, **five** of the class's rows carry the tail of one sentence and the head of another (`-Aii.sh:7`, `-B.sh:7`, `-audit.sh:92`, `-common.sh:10`, `A-rederive.sh:47`; the count is stable under every sentence-terminator rule tried, and it is the same five the test refuses or defers), so a per-line needle is wrong on them by construction. ⚠ **And the place vocabulary is derived from `PARTS`, not spelled**: the first version wrote the stems out, and the census reported *the subject test itself* at `?` and went RED — the needle had become a home of the fact it censuses. ⚠ **This row is ε's specification input, so the figures it carries are the ones that reproduce and no others.** Of the split recorded as *19 placement / 15 rationale / 5 straddling*, **only the straddler count of 5 reproduces** (D16 re-implemented the row across six readings and reached 14/20 and 21/13, never 19/15); the other two are retained in D16 as a record of that run and **ε measures its own**. The test refuses one row loudly and rules four `rationale` (do not touch) for a human to confirm; over the comment lines that are **not** census rows it returned `rationale` for all but a handful of continuations of the same placement sentences. ⚠ **The out-of-sample control figure is not carried here either.** D16 records that no treatment of straddlers reproduced it, which puts it in the same state as the 19/15 split this row already refuses; the population it ranged over is a command, not a figure (`grep -h '^\s*#' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l` less the 39 `prose` census rows; ⚠ a draft wrote `911` here and this program has since moved it twice), and ε measures its own verdict count. ⚠ **The two kinds this memo named are not exhaustive** — the class also holds *measured quantities* (`A-rederive.sh:47-48`), a *source-order invariant* (`:46`, the very sentence the `partset` row cites as `:44-49`) and a *tier-algorithm specification* (`-inventory.sh:266`). None is residence and none is a block as the subject of an event, and the rule must not touch any of them; `rationale` is where they land. It is the largest class, and read line by line it holds two kinds: lines that assert *where* blocks live (a file, a part or a group token beside them — the dispatcher header's placement table, each part's preamble) and lines that name blocks as the *subject of an event* (`marker` became a caller of `_measure`; `suiteset` returned an `echo`'s status). The second kind is mechanism rationale, and deleting it is what the rule does if applied to the class as printed. **Splitting the class is therefore ε's first task** (it was PR-1a-i's first task when this row was written,
before the re-slice), exactly as the other shape rules were split, and the split has four consequences it must carry in the same commit. **(i) The `prose` row is replaced by three rows, not amended** — `placement` (the rule as written), `rationale` (nothing to do, a subject test rather than a fallthrough) and `proseunsettled` (the refusals). Without them the census emits three classes with no rule and goes RED at rc=1, which is the coverage gate working. **(ii) `proseunsettled` is RED while it is non-empty**: a refusal that exits 0 accumulates exactly like the class it came out of. **(iii) Writing the test moves the census** — `HOMES` 70 → 73, because the test's own `GROUPS` set, its pointer needle and one comment quoting this memo's `marker` example are themselves homes. That is this slice deciding its own scope, which §9 permits, but **no figure elsewhere may be written as though the split were free**. **(iv) It grows `-audit.sh` from 638 lines to 754** — measured with `wc -l` on the patched part in the sandbox that implemented the test — which reaches the authoring threshold §9's precondition names, so the precondition fires **on this commit**: the seam is cut while writing, not afterwards. (The figure carries its command because it describes a tree that does not exist yet, which is the same reason the band record earlier in this section carries one.) |
| **mention** | nothing to do, and it is a subject test rather than a fallthrough: every vocabulary token on the line sits inside a **quoted string**, so it talks *about* a block instead of enumerating the set. Strip the quoted spans and if any token survives, the line is unclassified and RED. |
| **callsite** | nothing to do. A line naming two blocks because one *calls* the other is not a place the set is written down. The class exists so that saying so is a rule rather than an omission. |

**"One file per group" is withdrawn, and so is the rename it implied — the rename is not deferred to PR-1b,
it is dropped.** It was wrong twice over. **(a) It is a repartition, not a rename**: `-common.sh` alone holds
three groups, so renaming files to groups requires deciding per block where each lands — which *is* PR-1b's
move list, so the seam PR-1a defends would collapse. **(b) `-integrity.sh`, `-audit.sh` and `-inventory.sh` all hold only
kernel blocks**, so one-file-per-group merges them back into a single file — **1209** lines today, by `wc -l` over the three; ⚠ an earlier draft said 1192, which was the sum before this session's parts grew, undoing
the split `259e12cb` took on the primitive/consumer seam.

**The invariant file-granular action actually needs is weaker**: *no file holds blocks from more than one
group*. A group may span files, and the kernel must, since its whole-harness checks alone exceed the band.
That needs neither a separate query nor a layer token in the group vocabulary: once the *file* declares its
group the way a block does (D12), "this block's declaration disagrees with its file's" **is** "this file
mixes groups", and `PART_SLICE` — the hard-coded filename→slice map with an implicit
*everything-else-is-kernel* branch, where the two roles were fused — goes away. A group spanning four files
is then four files declaring it, which is what the kernel is (D11) and what one-file-per-group could not
express. **PR-1b's exit criterion is therefore a single empty `MOVE LIST`**, and no file's name is
load-bearing, so none has to change.

**PR-1a-ii: every block declares its group.** `# ships-with: <group>` in each block's body, one per block,
each reviewable on its own line. Two exceptions, both stated as predicates rather than name lists (D11):

- **The region trim.** A definition followed *only* by blanks and comments up to the next definition cannot
  carry an in-body declaration. Those blocks are un-one-lined, or given a following statement, before
  PR-1a-ii writes their declarations.
- **`all`, which PR-1a-ii cannot fix — PR-1a-i must.** Until the part set admits the dispatcher on the terms
  the `partset` row states, a declaration on `all` is read by nobody and reported by nobody, and an inserted
  line lands inside the roster literal; so *"every block declares its group"* is unsatisfiable and
  `undeclared=` cannot reach 0. That is the ordering the `partset` row carries.

**What PR-1a does not change: any block's subject on the success path.** `all`, `homes`, `inventory` and
`selfcheck` are exempt by definition, because their subject *is* the block set. Every other block's inputs
and output are byte-identical across PR-1a. ⚠ **Its verdict on the FAILURE path is not, and cannot be**: the
`reads` class adds a named failure to `budget`'s four reads, and changing what happens when a read fails is
the entire point of that rule. The exemption is a predicate over the success path, not a list of block names.

### §3a The slices' exit criterion

Every exit criterion in this memo used to be PR-1b's. This one is derived and measured, and its shape is
forced by two facts about the harness. **`homes` cannot serve as the criterion**: its four verdict raises and
seven parser guards all fire on *adding* something — a row with no class, a class with no rule, a false memo
quantity, a check that could not read its subject — and none fires on *failing to collapse*, which is what
every rule here does. And **the census shrinks in response to correct work**: `RULED BY THE PLAN: N of N`
ranges over the *observed* class set, so finishing a class removes it from numerator and denominator alike
and the gate stays green — measured, replacing the roster literal with a derivation prints `8 of 8` at rc=0
with the `roster` class gone from `BY CLASS` entirely.

So the criterion asserts over **edit sites**, and uses the census only where the observable is **per row** (a
guard column, a roster column) and therefore cannot shrink into a false green. ⚠ **It is apportioned across the slices, and the total is read off the apportionment rather than stated
beside it** — a criterion written for a PR that no longer exists reports every slice failing by design, and a
headline count written next to a list is a second home for the list's length. **β** owns the two subject-test
rows, `callsite` and `mention`; **α** owns `partset`, `roster`, `authorlocal` and `memoset`; **γ** owns
`groupvocab`; **ε** owns `prose` — and, once ε splits it, the three rows that replace it — **and `covgate`**;
**δ** owns `reads`. ⚠ **`covgate` moved off β when the coverage gate's content test was narrowed and
re-assigned** (the paragraph in §3): β lands no coverage-gate change, and this row is the surface that
authorises one, so leaving it on β would have this memo and β's §4 saying opposite things.
⚠ **The figure this paragraph used to carry was false before the re-slice and would be false again after it.**
It read *"eight obligations: seven of the nine class rules, plus `covgate`"*, which was written while `mention`
and `callsite` were regression guards; the ⚠ below promoted them to obligations without moving the count, so the
apportionment already summed to ten against a stated eight. After ε splits `prose` the class set itself grows,
so no fixed integer survives. The two figures beside it — *"at HEAD it fails with 8 of 8"* and *"7 of 8"* —
were measured against a script built for the eight and are retained **as a record of that run**, not as this
criterion's expected values; the script is in no tree. Each slice's
exit is its own obligations met **and** the vacuity guards holding — the guards are shared, because the
failure they catch (a block silently leaving the table) is available to any of them.

⚠ **`mention` and `callsite` were listed here as regression guards on the ground that their observable is
already true at HEAD. That was right about HEAD and wrong about β**: β changes both subject tests, so their
observable is exactly what β must move, and a criterion that scores the unchanged predicate as satisfied
would pass β for doing nothing. They are β's obligations, and the observable is the pair of planted
spellings above: unquoted must classify `callsite`, quoted must not classify `mention`.

Three properties it must have, each of which a measurement forced:

- **No expected value is written in it.** Every quantity it compares is measured twice — on the tree under
  test and on the base revision in a throwaway clone — which is this memo's own rule about figures. In
  particular `unverifiable=` is tested for **existence only**, never for a size, because §8 forbids
  predicting the number and the criterion may not become the site that does.
- **A collapse is two facts, and a literal-gone needle tests only a spelling.** Measured: `^MEMOS = \[`
  still matched `MEMOS = [(g, s) for g, s in MEMOSET …]` — it reported the enumeration present when what was
  present was a comprehension over the derivation. Each collapse is therefore asserted structurally: exactly
  one block carries the derivation, **and** every named reader calls it in command position.
- **Vacuity guards, compared against base rather than against a literal.** The failure this memo already
  documents — a file falls out of the part set, `defined=` drops, every gate stays green — is caught by *no
  block left the table*, not by a count. That is the same correction §3's split criterion needed above.

Measured behaviour of the eight-obligation script (recorded, not inherited): at HEAD it failed with
**8 of 8** outstanding, naming each open site. With one rule
genuinely landed (a `memoset` collapse: a derivation globbing the memo directory with a named failure,
`budget`'s loop reading it, and `inventory`'s payload fed through `argv` — the crossing the `roster` row
requires, since that payload is a quoted heredoc) it fails with **7 of 8**, and the guards hold across the
change (`defined=` 35 → 36, no block gone, file count unchanged). On the vacuity tree it fails on the guard
rather than the obligations, which is the distinction the old criterion could not draw.

⚠ **What it cannot test belongs in §8, not in a silent gap**: the *shape* of an `authorlocal` registration
(fixing a spelling would make the criterion predict the implementation); that `prose` was split on **this**
subject test rather than some other; `unverifiable=`'s membership; the T3 diagnostic's format, whose only
witness is a planted declaration and which is therefore PR-1a-**ii**'s; whether `-integrity.sh`'s charter was
widened by argument or drifted into; and the quality of a `reads` named failure, which the criterion
inherits from the census's `guard` column rather than improving on.

## §3b PR-1b — the moves

**Input**: `rederive inventory`'s `MOVE LIST` — the declared blocks whose file contradicts their declaration.
**It does not exist until PR-1a lands**, which is the seam: on a tree with no declarations the harness prints
`MOVE LIST: 0 of 0` and a separate `NO VERDICT` list of undeclared blocks, because a tier guess is not a work
list. PR-1b moves each block to the file matching its declaration, its exit criterion is that `MOVE LIST` is
empty, and it discharges A-i §13's owed `suites` relocation (§7). Nothing else is in it.

⚠ **That criterion is only meaningful if PR-1a-i landed the part-set derivation and retired `PART_SLICE`.**
Both failure modes were simulated end to end. With `PARTS` left as the hardcoded eight (`-inventory.sh:45`), the
moved blocks leave the part set and the criterion passes **vacuously** — `MOVE LIST: 0 of 29`, `defined=`
down by three, and the `umbrella` group absent from the tally altogether. With the part set widened but
`PART_SLICE` left in place, it is **unreachable** — the three `umbrella` blocks sit in `-umbrella.sh` and are
still reported misplaced. So PR-1b's gate is a read whose write path is two of PR-1a-i's rules, and PR-1b
additionally asserts that `defined=` is unchanged across the moves, so a file that fell out of the part set
cannot read as a clean list.

⚠ **`-umbrella.sh`'s own `# group: umbrella` preamble is PR-1b's to write**, in the commit that creates the
file. It has to be said, because §3 assigns file declarations to PR-1a-i and this file does not exist then —
and because a part file with no `# group:` line is today merely *reported*: its blocks stop being compared,
`MOVE LIST` and rc are unchanged, so PR-1b's exit criterion would be satisfiable by a `-umbrella.sh` that
omits its preamble. **PR-1a-i makes an undeclared part file fatal** (§3, `groupvocab`), which is what closes
that hole from the other side.

**Every destination exists or is determined.** D11 measures the whole list: two destinations, both fixed by
the declarations rather than chosen by the mover. The `A-i` three move into `-Ai.sh`, which exists. The
`umbrella` three move into a new `-umbrella.sh` — **creating a file for a group that has none is not a
repartition**, because *which* blocks go in it is precisely the set that declares `umbrella`, written in
PR-1a and reviewed there. What PR-1a was split to avoid is deciding a destination per block; reading one off
a declaration is the opposite of that.

**And the layer question does not arise.** `_proto`, `fixtures` and `say` declare `kernel` and already sit in
kernel files, so after the six moves `-common.sh` holds them and nothing else and the kernel is five
kernel-pure files. **No block's destination requires choosing between `-integrity.sh` and `-audit.sh`**, so
the primitive/consumer seam `259e12cb` took stays where it is and needs no vocabulary of its own.

## §4 Where the harness lives, and what that decides for #505

**Moved** to `2026-08-citation-hygiene-harness-external-state.md`, with §7, as a standalone prereq at the
band. Both sections' subject is this branch's relationship to things outside its own text; every section that
remains states a rule about the block set or about who may change one.

## §5 PR-2 — the behaviour fixes, and the promises they discharge

PR-2 lands after PR-1b, on blocks placed in their final files.

| fix | why it is PR-2's and not deferred |
|---|---|
| **`citations`** compares the authoritative §-title against the fixture's, per §0.5, treating a failed lookup as a failed measurement | `/elidex-review`'s CRIT-1 on #505; the block ships with A-i, so removal never discharges it |
| **`armmatrix`** binds each row's status | 27 rows print `EXIT=` and the block exits 0; A-ii cites it 5× |
| **`lanes`** routes its remaining bypasses through `_measure` | three are **unguarded**: a failure yields an empty loop, `failed` stays 0, the block returns 0 |
| **`column` / `carvecolumn` / `remedies`** read the child's *stdout* instead of `[ "$rc" -le 1 ]` | **Codex R3-F2 on #501, answered publicly with *"They are being fixed in #505, not here"***. #505 closing must not turn a public commitment into slot content |
| **`ruleset`** asserts `conditions.ref_name` selects `main` | **Codex R3-F3**, same public commitment. It ships **A-iii**, not A-ii |
| **`fixtures`' "24-key pinned map" comment** | measured smaller (§0.5). The comment is in **`fixtures()`**, not `citations()` |

## §6 PR-3 — whether the slice parts ship here at all

**Deliberately undecided by this memo.** After PR-1b, `-Aii.sh` / `-Aiii.sh` / `-B.sh` are exactly their
slices' blocks and nothing else, so removing them is a `git rm` with no reconciliation — a cheap, reversible
decision PR-3's own review can take on the evidence then, including M1's standing fact that **B and C cite the
harness zero times**, and D7's set of blocks whose group nobody has written down and nothing calls, four of
which are Slice B's.

This memo therefore creates **no defer slot**: nothing is discarded, and the question PR-3 answers is blocked
on nothing but PR-1. (Checked against `memory/feedback_defer-slot-eligibility-audit-at-create.md`'s four
questions: zero yes. The question has an owner, a trigger and a place in the forced order, which is what
distinguishes it from a slot.) **And it defers nothing of its own.** The coverage gate's content test is
**ε's**, stated in §3 in both its halves. ⚠ **It is not the only item owed, and an earlier draft of this
sentence said it was** — the same commit that swept this paragraph created two more in D18 and named them in
the same breath, and ε additionally holds the `prose` subject test and the three rows that replace `prose`.
The ledger is a query, not a count:

```bash
grep -nE '\bowed\b|Trigger:|DISCHARGED' docs/plans/2026-08-citation-hygiene-harness-*.md
```

⚠ **The needle was unanchored until two axes measured it**: `owed` without word boundaries matched inside
*allowed*, *followed*, *narrowed* and *showed*, so roughly a third of what the ledger returned named no item
at all. ⚠ **And it still cannot see an item that declines to use the vocabulary** — D19's *"No slice takes
this"* matches none of the three needles, which is the ledger's stated blind spot rather than a gap in it. ⚠ **It was PR-1a-i's when this
paragraph was written, and PR-1a-i no longer exists** — the re-slice replaced it with β/α/γ/δ/ε, so the
sentence named an owner that had been dissolved. The check over this memo's own quantities landed as
`b088dacc`, extending the memo gate this program already owns at `-audit.sh:265-281`; §9 judges that commit
against its own clause. Its population defect, which draft 13 recorded as a third owed item, was fixed in
`96d8fae3` rather than carried. ⚠ **The provenance block is landed** (`-inventory.sh`, block `attest`). It was landed at `84a7bd67`, reverted at `dae569d4` on a whole-output reading of §9's stopping rule, and re-landed once three review axes measured that the reading convicts the whole branch and acquits nothing; §9 grades it on membership, where it passes. Nothing is owed for it, and D18 carries the sequence. Every item the
query above returns has a site, an owner and a trigger; nothing here is a slot and nothing is an unowned
check. ⚠ **What this paragraph may not do is state the count**, which is how it went false: a number beside a
list is a second home for the list's length, which is the rule §3a already states for its own apportionment.

## §7 Registers, swept by command rather than by anchor

**Moved** — see §4's note.

## §8 What this memo does not get to claim

⚠ **This clause was written when §1 was in this file and the split did not sweep it** — §1 is now a
forwarding stub and the D-entries are in `2026-08-citation-hygiene-harness-measurements.md`, so *"beside"* has
become a cross-file `D<N>` pointer. The rule is unchanged and its home moved:

**A claim is made once, in the measurements memo's §1, beside the command that produces it; a claim with no runnable command is not
made.** There is no separate place to write `CHECKED`, because a second copy of a claim is not a check on the
first — it is a second thing to keep true. What that leaves without a home is the **negative** space:

| not measured | why, and who measures it |
|---|---|
| PR-1b leaves the `MOVE LIST` empty | its own exit criterion; unevaluable until PR-1a-ii writes the declarations **and** PR-1a-i lands the part-set derivation and the undeclared-file rule (§3b) |
| the set of declarations no code signal can confirm, after the moves | `unverifiable=` is computed from the call graph, which PR-1b's moves do not change — but §3's own class work does. Read it from the block afterwards; do not predict it |
| the `prose` class's rule | there is no command that decides it today; that absence **is** the finding, and PR-1a-i's subject test is what creates one (§3) |
| PR-1a-i's terminality under CLAUDE.md's base case | its own `/elidex-plan-review`, which is the only thing that can discharge a *passed-review* condition (§2) |
| the register list is complete | **unfixably so at authoring** — its subject is a directory other sessions write to, and it grows within a session. §7's query, re-run at execution |

## §9 What this memo authorises

⚠ **Two of this clause's referents moved out of this file and the clause was not swept.** §4 and §7 are
forwarding stubs since `9647ba4d`; the content they held — what the close must carry, and the register
queries — is in `2026-08-citation-hygiene-harness-external-state.md`, whose own header says it authorises
nothing. An authorisation may not resolve into a file that disclaims authority, so the **authorising text is
here and the external-state memo is its evidence**, not its home. ⚠ **And "this memo pair" is now five
files** — the two splits created two of them, so an unswept transfer clause would leave both outside what it
carries.

**Authorises**, after `/elidex-plan-review` passes: closing #505 **with a closing comment whose content is
recorded in the external-state memo's §7** and its branch **retained**; carrying the transfer set (D10) and
**every `2026-08-citation-hygiene-harness-*.md` in this branch's `docs/plans`** onto
`webref-cite-audit-tool`; and
**the scopes of β, α, γ, δ and ε as stated in §3, §2 and §3a**. ⚠ **This clause authorised
"PR-1a-i's and PR-1a-ii's scopes" until it was measured** — a slice pair the re-slice dissolved, so the
authorisation every live slice runs through named nobody, and §6 asserted the dissolution two sections
earlier while this clause carried the old name. ⚠ **The last two sections are named because §3's rule rows
are not the whole statement of a slice's scope** — §2's slice table and §3a's apportionment carry obligations
§3's rows do not, and a slice whose mechanism falsifies a §3 row must edit that row, so an authorisation
resolving through §3 alone would resolve through text the slice itself wrote. ⚠ **Naming §2 and §3a does not
remove §3 from the set**, and β's §3 claims it does; the circularity is narrowed, not discharged, and it
closes only when a slice's own rows are quoted into its slice memo. It does **not** certify any slice as
terminal under CLAUDE.md's base case (§2).

**Every PR under this umbrella takes its own `/elidex-plan-review` before implementation**, per CLAUDE.md's
edge-dense rule *(b) 各 PR は実装前に `/elidex-plan-review` 必須*. ⚠ **The population is every PR this memo forces, and it is larger than §2's slice
table.** A sweep replaced an enumeration (*PR-1a-i, PR-1a-ii, PR-1b, PR-2 and PR-3*) with a pointer to §2 —
and §2's table holds the five slices only, so PR-1b (§3b), PR-2 (§5) and PR-3 (§6) fell out of the mandatory
population in the commit whose subject was *four completeness claims in the umbrella were false*. The
population is **§2's five slices plus every PR §3b, §5 and §6 name**, and it is not compressed to a pointer
again. This clause used to enumerate *PR-1a-i, PR-1a-ii, PR-1b, PR-2 and PR-3*, naming none
of the five slices the re-slice created, so the obligation had two homes with different content and the one
in the authorising section was the stale one. Each slice inherits whatever mechanism has landed as **ground
rather than as a proposal**, and should be told so — which now includes the provenance block (D18).

**Does not authorise**: changing any block's **subject** on the success path for any block other than `all`,
`homes`, `inventory` and `selfcheck`, whose subject is the block set (behaviour fixes are PR-2, §5); any
removal (PR-3, §6); deleting the `citation-hygiene-harness` branch; editing a status register before §7's
query is re-run; creating a defer slot; **or landing mechanism on this branch that decides what a PR
decides.**

**That last clause is this memo's stopping rule, and it is narrower than the wording it replaces.** Earlier
drafts forbade *"landing one more line of mechanism on this branch"*. That scope was wrong, on the review
record's own finding: it made **prose** the only permitted response to findings whose answer was
**execution** — the failure mode `memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md` names, adopted
as a rule. What the clause was always trying to say is the thing that makes landing scope-ful at all, stated
in the preamble (`:44`, before §0.5): **no mechanism that decides what a PR decides**. Mechanism whose only subject is *this
memo's own falsifiability* decides nothing about the harness's design and is therefore permitted; extending
`-audit.sh:265-281`'s memo gate from §3's class-rule row keys to this memo's **quantities** was exactly that,
and it landed as a separate commit, `b088dacc`.

**The clause's operative test is MEMBERSHIP of the work list, and one criterion is applied to every commit
the falsifier prints.** ⚠ **Two drafts of this paragraph got that wrong in opposite directions.** The first
said *"two commits have since been run against this clause"* while the falsifier printed thirteen. The second
graded three more against *"the whole `homes` output"* — and three review axes then measured that the whole
output moves on almost every commit here, so the test as stated convicts nearly the branch and the grades
written under it were false. ⊕ Measured, per commit against its own parent:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
for c in a5fab499 b088dacc 96d8fae3 2abaea1b 84a7bd67; do
  git -C "$d" checkout -q "$c^" && bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > "$d/a"
  git -C "$d" checkout -q "$c"  && bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > "$d/b"
  printf '%s  output-delta=%s  ' "$c" "$(diff "$d/a" "$d/b" | grep -c '^[<>]')"
  grep -ohE 'HOMES: [0-9]+ \([0-9]+ code, [0-9]+ prose\)' "$d/a" "$d/b" | paste -sd' -> ' -
done
```

`b088dacc` moves **69** output lines, `2abaea1b` **21**, `84a7bd67` **6**, `96d8fae3` **2**, and the revert
`dae569d4` **10**. Membership moves for exactly one: **`a5fab499`, 72 homes → 70**. That is the commit §9
convicts, and the two `prose` rows it dropped are what the conviction has always rested on. ⚠ **So the test
is membership**: a commit that changes *what is on the work list* decides what a PR decides; a commit that
renumbers the list does not, or the clause would forbid every edit to any part file.

The grades, under that test:

- **`b088dacc`** — 72 → 72. **Passes**, as it always did, and now for a stated reason rather than by
  subject alone.
- **`96d8fae3`** — membership unchanged. **Passes.** ⚠ A draft graded it *"census unmoved"*; it moved two
  output lines, which is not the test.
- **`2abaea1b`** — 70 → 70. **Passes.** ⚠ A draft graded it *"census unmoved"* and then convicted it anyway
  for discharging a raise β had entered; both halves were wrong — it moved 21 output lines, and discharging a
  raise by *fixing the thing raised* is not deciding a PR's scope.
- **`84a7bd67`** — 70 → 70. **Passes**, and the block is landed. ⚠ **It was convicted and reverted at
  `dae569d4` on the whole-output test, and that was an error**: the same test acquits nothing on this branch,
  and applied consistently it convicts `b088dacc` — permitted by name in this very clause — eleven times
  harder. ⊕ Both figures come from the fence above, and the block's own state from
  `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh attest`. Three review axes measured the
  inconsistency independently. ⊕ The revert is itself undone rather than left standing — verify with
  `git diff 84a7bd67 -- docs/plans/2026-07-citation-hygiene-A-rederive-inventory.sh`, which prints nothing —
  because leaving it would keep the branch in the state the clause exists to prevent: the block gone, its
  cost unbooked, and the only enforcement of the mark convention removed on a false measurement.
- **`a5fab499`** — 72 → 70. **Violates**, unchanged.

⚠ **Nine further commits the falsifier prints pre-date this clause** (`adb8a33b`, `49b4f645`, `979e5426`,
`7ad42edd`, `fc47cde1`, `259e12cb`, `9a0ff039`, `90e1429b`, and `dae569d4`'s own parent chain before the
clause was written); the clause does not reach them and they are not graded. `dae569d4` and the commit
carrying this paragraph do fall under it: both move output lines and neither moves membership, so both pass.

⚠ **Draft 13 excused it with an exception, and that was wrong.** CLAUDE.md keys the standalone-prereq form to
*touching* a file already past 1000. `-audit.sh` was 854 until `b088dacc` — this branch's own commit, which
this clause **permits** — took it to 1059, and that commit's message announced the consequence in advance
(*"which moves the seam cut §3 assigns to PR-1a-i … to its standalone prereq form. The split follows
immediately."*). §3 grades the same event correctly two sections earlier: *"the cut was already late when it
happened."* An exception here would also be **self-composing** — any commit this clause permits could
manufacture the compulsion for the next one — and
`memory/feedback_touch-time-split-means-while-writing.md:35` names this exact shape as **evidence of a miss**,
not a ground for exemption. The branch's own counter-precedent is `259e12cb`, which cut `-integrity.sh` at
768 lines, at band time, as `:39` prescribes.

**The clause therefore stands unamended, and `a5fab499` is recorded as a violation of it caused by the missed
band-time cut.** What follows is not an exception but a **precondition**: a mechanism commit this clause
permits must not be the commit that crosses the size trigger — cut the seam while writing, at the band, so
that no later split has to decide a PR's scope. The scope effects the split did have — two `prose` homes
fewer on the work list — are PR-1a-i's to absorb, not facts PR-1a-i may assume.

⚠ **The split did not meet the position rule draft 12 stated for it, and that is recorded rather than
deleted with the paragraph.** The rule was: the new file joins the part set **by derivation, after
`partset` lands**. `a5fab499` cut first and joined it by hand in **both** hardcoded homes —
`-inventory.sh:45` and the dispatcher's bootstrap loop at `A-rederive.sh:52` — which are the two homes
the `partset` rule exists to collapse, and exactly the two the census names (`rederive homes`, class
`partset`). PR-1a-i collapses both; no third is claimed here, because the work list is the census and not a
count written in this sentence.

The falsifier is correspondingly a **list to judge**, not an emptiness assertion:

```bash
# BASE is the commit that INTRODUCED this memo pair, not the latest commit that
# touches it. Recomputed as the latest, a commit that touches the memo AND
# mechanism becomes BASE itself -- and `BASE..HEAD` excludes BASE, so it hides
# its own mechanism change. `adb8a33b` is exactly that shape.
BASE=$(git log --diff-filter=A --format=%H \
       -- docs/plans/2026-08-citation-hygiene-harness-disposition.md | tail -1)
git log --oneline "$BASE"^..HEAD --name-only \
    -- . ':!docs/plans/2026-08-citation-hygiene-harness-*.md'
```

Its scope is the branch, matching the clause: every commit since the memo pair that touches anything but the
pair is printed, whether or not its path matches a harness glob, and each is **judged against the clause**
rather than counted.

The reasons mechanism landed ahead of the design, and why they are now spent: the order — land the mechanism
and falsify it before writing the design against it — was **user-ratified as the method for these drafts**,
and a working-tree edit is invisible to review agents, who clone HEAD and whose plants found holes the author
had missed. Neither survives the stopping rule, because an **uncommitted** mechanism can be exercised in a
`git clone --local` sandbox (§1's preamble), which is how D4–D7 and D11–D14 were measured and the form every
further measurement takes. And landing was never scope-free: a commit that moves the census output §3 calls
the work list decides part of PR-1a's scope, and several did. **From here the design is fixed and PR-1a
implements it.**
