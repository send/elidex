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
git log --oneline origin/main..HEAD -- 'docs/plans/*A-rederive*'
```

Each of those changes was falsified by planting into it or by running it against an acceptance list, not by
inspection — two had defects inspection did not show and the acceptance list did. Where a falsification still
constrains a future action it is stated beside the rule it supports (§3), not kept in a ledger, because a
ledger of past changes is a hand-written set with the failure mode of the site lists it replaced. Some of
those commits moved the census output §3 calls the work list, and a commit that moves the work list decides
part of PR-1a's scope. **From this memo forward no further mechanism lands on this branch** (§9).

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

The analysis note's M1–M7 are the shared basis; re-run them there rather than restating.

**No expected value is written beside a command in this section.** A figure whose subject is outside this
repository — a memory directory another session appends to, memos on another branch — cannot be stamped with
a commit at all (`memory/feedback_verified-claims-go-stale-under-own-later-edits.md`). D11–D13 are the one
exception, and they are exempt for a stated reason: on HEAD there are no declarations, so `MOVE LIST` is
`0 of 0`, and their figures exist only inside the `git clone --local` sandbox they describe. They are
recorded falsifications, not values a reader re-derives today.

```bash
# D1  is part SOURCE ORDER load-bearing? (a glob sorts alphabetically); then,
#     in a sandbox, reverse the dispatcher's loop and run `… selfcheck`
for p in integrity audit common Ai Aii Aiii B; do printf '%-10s ' "$p"
  grep -cE '^[A-Za-z_][A-Za-z0-9_]*=' docs/plans/2026-07-citation-hygiene-A-rederive-$p.sh; done
# D2  who cites the two helpers the collapse renames? The needle must match a
#     BARE INVOCATION as well as a backticked name; the scope must include the
#     two memos on THIS branch; `--include='*.md'` keeps the harness's own
#     comments from reading as a memo citation.
N='`(say|fixtures)`|rederive (say|fixtures)\b|(^|[;|(] *)(say|fixtures) [^ ]'
grep -rnE "$N" --include='*.md' ../elidex-wt-citeaudit/docs/plans/
grep -rnE "$N" docs/plans/2026-08-citation-hygiene-harness-*.md
# D3  is the derived roster set-identical to `all`'s literal? Diff BOTH WAYS --
#     a one-sided `comm` reads a superset as agreement.
bash -c 'for p in integrity audit common Ai Aii Aiii B; do
           . docs/plans/2026-07-citation-hygiene-A-rederive-$p.sh; done
         declare -F | awk "{print \$3}" | grep -v "^_" | grep -vx all \
           | grep -vE "^(lanes|staleclaims)$" | sort' > /tmp/derived
sed -n '/^all() { set --/,/local failed/p' docs/plans/2026-07-citation-hygiene-A-rederive.sh \
  | sed '$d' | tr ' \\' '\n\n' | grep -xE '[a-z]+' | grep -vx set | sort -u > /tmp/literal
comm -23 /tmp/derived /tmp/literal; comm -13 /tmp/derived /tmp/literal
# D9  THE HOMES CENSUS. §3's work list is this command's output.
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes
# D10 the transfer set, DISCRIMINATED rather than asserted: a plain `git log`
#     over the file returns replayed-equivalent commits too.
git log --oneline --cherry-pick --right-only \
    webref-cite-audit-tool...HEAD -- 'docs/plans/*A-rederive*'
git diff --numstat webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'
# D8  the register set (§7). NO expected value: its subject is a directory
#     outside this repository that other sessions append to every day.
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn '#505' "$MEMORY"/*.md
# D11 PR-1a's OWN CONTENT, materialised. In a `git clone --local` sandbox,
#     insert `# ships-with: <the block's computed group>` under EVERY block
#     definition -- not only the ones a prediction expects to succeed, or the
#     run measures the prediction rather than the harness
#     (`memory/feedback_self-authored-test-verifies-intent-not-text.md`) --
#     then read the WHOLE run, not just `DISAGREE=` and the MOVE LIST: a plant
#     that corrupts a definition surfaces in `selfcheck`. This is the only way
#     to see PR-1b's input before PR-1a lands, which is §3b's seam.
# D12 THE FILE'S OWN GROUP: add `# group: <g>` to each part's preamble, route
#     `_misplaced` off it instead of `PART_SLICE`, then break it two ways (a
#     wrong value, a missing one).
# D13 T3's boundary test over GROUPS rather than ORDER, so a `kernel` caller
#     counts as a boundary crossing like any other.
# D14 drop T2 and make the no-caller T3 case return NO ANSWER; re-read the
#     move list, the tally and the exit status.
```

- **D1** — no part makes a source-time statement another part reads; measured in a sandbox with the loop
  reversed, `selfcheck` is GREEN and `inventory` produces the same table. **Order is not load-bearing, so a
  sorted glob is a sound replacement.**
- **D2 — the rename does change citation surface, and the surface it changes is this memo pair.** No slice
  memo on `webref-cite-audit-tool` cites either helper as a block (the hits there are English prose), but
  §0.5's command block invokes `fixtures /tmp/fx` **bare** and both 2026-08 memos name `say` and `fixtures`
  throughout. A needle matching only backticked names and `rederive <name>` misses a runnable invocation,
  which is the one kind of citation a rename actually breaks. **PR-1a-i renames and updates these two memos
  in the same PR**; I5 survives because the whole surface is inside that PR, not because there is none.
- **D3 — the derived roster is a superset of the literal, and §3's `roster` exclusion list is short by one
  term.** `declare -F` minus `_`-prefixed, minus `all`, minus `$AUTHOR_LOCAL` derives 27 names against the
  literal's 24; the extras are `fixtures`, `readers` and `say`, and the `_`-prefix rename covers two.
  **`readers` is covered by nothing** — not `_`-prefixed, not in `AUTHOR_LOCAL="lanes staleclaims"`
  (`-common.sh:592`). It is off the roster because it takes a **required argument**: `rederive readers` with
  none prints a usage line and returns 2, so a derived roster would have `all` invoke it argument-less and go
  RED. The rule must exclude it on that ground (§3, `authorlocal`), and the proof is the two-way `comm`; a
  one-sided one reads a superset as identity.
- **D4 — a pure rename moves the computed ship-with.** Renaming the shared files to group-named files and
  extending `PART_SLICE` so the new stems are known shrinks the routing disagreement **with no block moved**,
  because group-named files are *slice-like*, so T2 fires for everything in them and shadows T3. The metric
  improves because the measurement changed. **This is why the declaration, not the computation, is the
  authority.**
- **D5 — replacing `all`'s roster literal with an expression breaks its readers, loudly and wrongly.** With
  the roster derived in place, `selfcheck` accuses `"all|$(printf`, `$(declare`, `$AUTHOR_LOCAL` and `'%s|'`
  of being *"dispatched by `all` but defined nowhere"*, and `inventory`'s roster column reads `no` for every
  block. **The derivation must be a function every reader calls, not an expression they each re-parse.**
- **D6 — the `AUTHOR_LOCAL` read is unguarded and hardcodes a filename**, so a rename kills `inventory` with
  an uncaught `FileNotFoundError` traceback rather than its own diagnostic, which every other input has.
- **D7 — some blocks have no signal independent of their filename.** Dropping the T2 branch, `anchors`
  `bmemo` `offline` `partition` `staleclaims` `timing` fall to *"T3 no caller"*, so their declaration is
  unfalsifiable — the condition under which the analysis note's §2 rejected a declaration for `kind`. The
  evidence is their `declared by` column, `-` for all six; `inventory`'s *"on the roster, declared by no
  memo"* line does **not** reach `staleclaims`, which is author-local and so not on the roster at all.
- **D9 — the census** finds every site five review axes have named, including the indirect reader that names
  nothing, and prints a `guard` column plus the two things it cannot see. §3 is written against it.
- **D10 — the transfer set.** `--cherry-pick --right-only` drops commits whose patch is already present on
  the other branch, which is what makes the answer derived. A plain path-restricted `git log` over the harness
  returns replayed-equivalent commits too, and so returns more than the transfer set contains.
- **D11 — the move list exists, and it is not a list of guesses.** With every block's computed group planted
  as its declaration, `inventory` reports `DISAGREE=0` and a MOVE LIST of **six** blocks in **two**
  destinations: three declare `A-i` and sit in `-common.sh`; three declare `umbrella` (`budget`, `lanes` from
  `-common.sh`, `suites` from `-Aiii.sh`). The blocks §3 once worried about placing — `_proto`, `fixtures`,
  `say` — are **not on the list**: they declare `kernel`, already live in kernel files, and after the six
  moves `-common.sh` holds nothing else.

  **Two blocks refuse the plant: `_measured` and `all`.** The predicate is **what follows the definition**,
  not one-liner-ness: the region trim walks back over blanks and comments, so a definition followed *only* by
  those cannot carry an in-body declaration. `_measured` is the last definition in `-integrity.sh`; `say` is
  a one-liner that accepts a declaration today because `HDR='…'` follows it, and would stop the moment a
  comment is added after it. **`all` cannot be declared, and its declaration is not even reported as
  unread** — §3's `partset` row carries the cause and the ordering that fixes it. That inverts this harness's
  charter (`-audit.sh`: *written-and-unread must never come out as "undeclared"*) against the block §3's own
  preamble calls **the single most important home**. `all()` is additionally a **line-continuation**
  definition (`all() { set -- … \`), so an inserted line lands *inside the roster literal* and `selfcheck`
  then reports three block names that do not exist — visible only if the whole run is read.
- **D12 — a part file can declare its group, and doing so reproduces the move list exactly.** Routing
  `_misplaced` off a `# group:` line in each part's preamble, with `PART_SLICE` out of the predicate
  entirely, prints the **same six blocks and the same 339 lines**. Falsified both ways: flipping `-Ai.sh`'s
  declaration to `B` puts its three blocks on the list (9 / 513), and deleting `-Aii.sh`'s prints
  `NO GROUP DECLARED` beside all ten of its blocks rather than silently placing them.
- **D13 — the `kernel` caller was being dropped from the boundary test, and today it changes nothing.**
  `cs = [route[c] for c in callers[b] if route[c] in ORDER]` filters `kernel` out because `ORDER` is the
  slice sequence. Widening it to `GROUPS` leaves every block's group unchanged — the block table diffs to the
  single line the edit itself added — because no block currently has both a `kernel` caller and exactly one
  slice caller, the only shape the filter can misroute. It is latent, not inert: the declarations PR-1a
  writes are what `route` reads. ⚠ **The one-token swap does not run**: two lines below,
  `min(cs, key=ORDER.index)` raises `ValueError` the moment `cs == ["kernel"]` (`say`'s case). The widening
  therefore carries a **second, non-obvious edit — a tie-break total over `GROUPS`** — which encodes where
  `kernel` sits in the ordering.
- **D14 — retiring `PART_SLICE` for real means dropping T2, and that is behaviour-neutral only if a tier
  with no evidence stops answering.** `_misplaced` is one of four readers of `PART_SLICE`; the load-bearing
  one is **T2** (`comp[b] = PART_SLICE[part]`). Re-sourcing T2 off the file declaration flips `_wtscan` from
  `A-i` to `kernel` — one of D11's own six movers — because a file declaration is total where `PART_SLICE`
  was partial. **Dropping T2 instead** does not: once `_misplaced` compares block-declaration against
  file-declaration, T2 is literally that same comparison. Measured with T2 removed, D12's file declarations
  in place, and the design-correct declaration on every block a part file defines:

  ```
  MOVE LIST: 6 of 33 declared block(s) / 339 lines     # unchanged; `_wtscan` stays A-i
  declared=33  agree=24  DISAGREE=0 (binding 0)  unverifiable=9  undeclared=2
  undeclared: _measured all        rc=1
  ```

  **`unverifiable` is a rule, not a count.** It is *"no code signal reaches this block"*: D7's six, plus
  `homes`/`inventory`/`selfcheck`, which nothing **calls** — `all` reaches them through its roster, by
  positional parameter. The obvious repair, treating roster membership as a call edge, was measured and
  manufactures **five binding false disagreements** against correct declarations, because a dispatcher calls
  everything and so puts every block in the dispatcher's group. So the rule PR-1a-i implements is: **a
  dispatch edge is not a call edge** — which is what makes the unverifiable set honest rather than a number
  to argue down. Today T3 manufactures `kernel` for exactly those blocks, so a declaration nothing can
  confirm reports as **agreeing**; `unverifiable=` replaces that with no answer, and the two blocks §3 must
  still solve stop being silent — they come out at `rc=1`. The run above assumes the dispatcher's own file
  declaration (§3, `groupvocab`); without it `all`'s file side rests on an assignment no rule has made.

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
| I1 × I3 | The roster must be **derived from the definitions**, not listed; deleting parts with the roster untouched makes `selfcheck` accuse blocks that no longer exist. | **PR-1a-i** |
| I1 × I5 | Excluding helpers by `_`-prefix requires renaming `say`/`fixtures`; D2 shows the citation surface that breaks is this memo pair's, so the rename and the memo edits are one PR. | **PR-1a-i** |
| I2 × I5 | A part **rename** must not change a block name. The dispatcher resolves by name across all sourced parts (D1), and once I2 holds a rename can no longer change the routing answer either. | **PR-1a-i** |
| I2 × I3 (files) | A file's group must be **declared, not read off its name**, or the move list compares a declaration against an inference and the two roles the group vocabulary plays — *whose concern a block is* and *which file holds it* — stay fused in `PART_SLICE`. That fusion is what left `umbrella` with no destination and dropped `kernel` from T3's boundary test. D12/D13. | **PR-1a-i** |
| — | PR-1a-ii writes declarations into checks PR-1a-i has already made correct. It opens no intersection of its own. | **PR-1a-ii** |
| I4 × I5 | The tiers still read the memos (T1), so `inventory` still needs a checkout that has them. That is **pre-existing** — M3 lists `inventory(exit 1)` among seven REDs of one class — and §4 turns on it. | §4 |

⇒ **The intersections marked `done` are already discharged, and that is why PR-1 can be split.** The seam is
**step 1's output**: the move list does not exist until the declarations do. So PR-1a collapses and **PR-1b
moves**; and one step earlier, the declarations cannot be *read* until the part set, the tiers and the
classifier are what §3 says they are, which is the PR-1a-i / PR-1a-ii seam.

⚠ **PR-1a-i does not meet CLAUDE.md's single-intersection base case, and this memo does not argue that it
does.** All four open intersections above are mechanism, so all four land in PR-1a-i and none in PR-1a-ii;
§3 additionally gives PR-1a-i a tier-algorithm change (D13/D14), a census classifier it does not yet have
(the `prose` subject test), and a failure-path change §3 itself flags as not behaviour-neutral. The base case
is *a slice that has **passed** its own plan-review*, not one that is required to take one, so this umbrella
cannot discharge it in advance. **PR-1a-i's own `/elidex-plan-review` is where that is settled**, and it may
split PR-1a-i further; §9 authorises the scope, not its terminality. (`axes.md`'s wording —
*scope が単一 invariant-axis 交点に絞られている場合* — is narrower on **scope** than CLAUDE.md's and omits
the passed-review condition, so the two are not ordered; both must hold.)

## §0 PR-0 — the `-audit.sh` touch-time split

`-audit.sh` is above the 700–800 authoring band (`wc -l docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh`)
and PR-1a-i is the PR that grows it. CLAUDE.md is explicit that a touch-time split is a **standalone prereq
PR, not part of the feature PR** (*real cohesion seam があれば feature 着手前に standalone な prereq split
として分割する — split は単独 PR / 単独 commit*). **PR-0 is therefore its own PR and lands before PR-1a-i.**

It is a split, not a redistribution of §3's work: §3's rules are not all in this file. `roster` edits the
dispatcher's `all()` literal and `-common.sh:592`; `authorlocal` is `-common.sh:592`; `partset` has sites at
`-common.sh:516,520` and the dispatcher's `:52` as well as `-audit.sh`. `groupvocab`, the tiers, the
classifier and `reads` are `-audit.sh`'s.

The precedent is on this file class: `fc47cde1` added `homes` and pushed `-integrity.sh` to 768 lines;
`259e12cb` split it on the primitive/consumer seam and took it back to 114. That seam is settled and PR-0
does not disturb it — PR-0 finds a seam **inside `-audit.sh`**, whose blocks are `homes`, `inventory` and
`selfcheck` plus their parsers. Its exit criterion is behavioural identity: every block's stdout, stderr and
exit code byte-identical either side, which is how `259e12cb` was falsified.

## §3 PR-1a-i — the mechanism, and PR-1a-ii — the declarations

**Goal**: after PR-1a, the block set has exactly one home per class, and file-granular action is sound.
**PR-1a-i** makes every derived fact below correct with zero declarations written; **PR-1a-ii** writes them.
The rules are stated together because they are one design; the split is where they land. Elsewhere in this
memo **"PR-1a" names the pair**.

**The work list is `rederive homes`, and so is the list of classes.** The mapping from home to class lives in
the census, it is **total**, and an unclassified home is **RED** — falsified by planting an unruled literal.
Below is the *rule* per class, and the class names are the census's.

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes   # `BY CLASS:` names every class
```

⚠ **What the coverage gate buys is narrower than `RULED BY THE PLAN: 9 of 9` reads.** Two things are wrong
with it and both are PR-1a-i's. **(a) It gates on a row's key, not its content**: replacing all nine rule
cells with `TBD` still prints `9 of 9` at `rc=0` — the same defect as *a class with no rule*, one altitude
up, and the `prose` row is the live instance, counting toward `9 of 9` while its own cell says the rule may
not be applied. **(b) It is not scoped to §3**: `-audit.sh:268` runs
`re.findall(r"^\| \*\*([a-z]+)\*\* \|", PLAN.read_text())` over the **whole memo**, so a rule row moved out of
§3 into any other table still satisfies it. What the gate proves today is that every class the census emits
has a row **somewhere in this file**. PR-1a-i gives it a content test and a section scope. (A machine check
over a plan-memo's own claims is a decision surface this program does not own: the canonical site is
`.claude/tools/claim-gate-plan-check.py` on branch `claim-gate-plan-check`, and a second instance here would
violate CLAUDE.md's *one issue, one way*.)

| class (the census assigns it) | rule |
|---|---|
| **partset** | one derivation from the glob; every reader calls it — **and it lands FIRST in PR-1a-i, before either declaration rule**, because both of them read it. **This is the rule that makes `all` declarable, and the site is `inventory`'s hardcoded `PARTS = ["integrity","audit","common","Ai","Aii","Aiii","B"]` (`-audit.sh:325`)**, from which `srcs`, `defline` and the stray counter are built inside the `INVENTORYPY` heredoc: the dispatcher is not in that list, so `all` is in neither `srcs` nor `defline` and a declaration on it is read by nobody and reported by nobody. (`homes`'s `PARTFILES` needle at `-audit.sh:59` is a *different block in a different process*; it governs the census, not declarability.) ⚠ **The part-set fact has two incompatible consumer kinds and the rule must state both.** `homes` and `inventory` *source* each part file (`bash -c "set -e; . <file>; declare -F"`, `-audit.sh:66-68`), and the dispatcher's last line is `"${1:-all}" "$@"`, unguarded — sourcing it runs the whole suite recursively (measured: hundreds of processes before the run was killed). So the derivation is a **text set that includes the dispatcher** and a **source set that excludes it**; the alternative, and the cheaper one, is a `[ "${BASH_SOURCE[0]}" = "$0" ]` guard on the dispatcher as a **prerequisite**, after which one set serves both. Two further consequences: the stem derivation `f.name.split("A-rederive-")[1][:-3]` (`-audit.sh:65`) raises `IndexError` on the dispatcher, so admitting it to the text set means changing the stem derivation too; and **T0's predicate is a part-set sentinel** — `part["all"]` is force-set to the literal `"(disp)"` at `-audit.sh:386` and read at `:553` — so giving the dispatcher a real stem silently turns `all`'s routing from a T0 code signal into a T3 call-graph inference, which must be handled in the same edit. ⚠ **One spelling, not two**: every glob site already writes `…A-rederive*.sh`, which matches the dispatcher, so `selfcheck`'s *"N harness parts"* counts it; the exclusions are in the **derivations**, not the glob. |
| **roster** | `_roster`, a **function**, returning every non-`_`-prefixed, non-author-local, non-`all`, **non-argument-taking** definition; `all`, `inventory` and `selfcheck` **call it** (D5: an inline expression makes its readers parse its own shell tokens as block names). The argument-taking exclusion is not optional — D3 measures the derived set at three names above the literal, and `readers` is excluded by no other term; `all` would invoke it argument-less and it returns 2. Two of the three readers are python inside a heredoc and cannot call a bash function directly; the harness already crosses that boundary with `subprocess.run([… "declare -F"])` and PR-1a-i uses it rather than keeping a regex. `say` → `_say`, `fixtures` → `_fixtures`, **and the two 2026-08 memos are updated in the same PR** (D2). |
| **authorlocal** | a registration **adjacent to each definition**, carrying its reason, as a **shell statement** — `_roster` is a bash function and needs it as runtime data, and bash cannot read comments. (`declare -f` is not the reason: the ship-with needle is a Python regex over raw source and never passes through it.) `readers` is registered here too, with *takes a required argument* as its reason, closing the defect A-i §15 records. |
| **groupvocab** | one `GROUPS` set, validated — **and the part files join the blocks in declaring against it**: each part's preamble carries `# group: <g>`, **and so does the dispatcher** (`kernel`; without it `all`'s file side is an assignment no rule has made). `_misplaced` reads that instead of `PART_SLICE`, and `PART_SLICE` retires **here, in PR-1a-i**. ⚠ **The retirement is not scoped to `_misplaced`**, which is one of four readers and not the load-bearing one. The load-bearing reader is **T2**. Re-sourcing T2 off the file declaration is *not* behaviour-neutral (it flips `_wtscan` A-i → kernel, one of D11's own movers); **T2 is dropped**, because once `_misplaced` compares block-declaration against file-declaration, T2 is literally that same comparison. Four further consequences, all PR-1a-i's: **a tier with no evidence must stop answering** (D14 — `unverifiable=`, without which dropping T2 turns nine unconfirmable declarations into silent agreements), on the rule that **a dispatch edge is not a call edge**; T3's boundary test ranges over `GROUPS`, **with the `min()` tie-break widened in the same edit** or it raises (D13); a T3 disagreement must **name the caller declaration it rests on**, because `route` is seeded from the declarations and one wrong entry otherwise reports as a binding disagreement against its correct neighbour; and **a part file with no `# group:` line is RED**. Today such a file is *reported* but its blocks simply stop being compared — `MOVE LIST` and rc are unchanged — which would make PR-1b's exit criterion satisfiable by an undeclared file (§3b). |
| **memoset** | there are **two disagreeing homes** — `inventory`'s `MEMOS` and `budget`'s `for m in …` loop, which do not carry the same memo set. One derivation; both readers call it. |
| **reads** | every read of the harness's or a memo's text must have a **named failure**. The class's size and its guarded/unguarded split are read from `homes`, not restated here; note that `_measure … \|\| failed=1` **is** a named failure, so a `guard` column that does not know the validity primitive under-reports it. |
| **prose** | a prose home becomes a pointer to `rederive homes` / `rederive inventory`, or is deleted — ⚠ **but this class has not got the subject test its siblings got, so that rule may not be applied to its rows as they stand.** It is the largest class, and read line by line it holds two kinds: lines that assert *where* blocks live (a file, a part or a group token beside them — the dispatcher header's placement table, each part's preamble) and lines that name blocks as the *subject of an event* (`marker` became a caller of `_measure`; `suiteset` returned an `echo`'s status). The second kind is mechanism rationale, and deleting it is what the rule does if applied to the class as printed. **PR-1a-i's first task is therefore to split this class on that test**, exactly as the other shape rules were split; until it is split the class is not a work list. |
| **mention** | nothing to do, and it is a subject test rather than a fallthrough: every vocabulary token on the line sits inside a **quoted string**, so it talks *about* a block instead of enumerating the set. Strip the quoted spans and if any token survives, the line is unclassified and RED. |
| **callsite** | nothing to do. A line naming two blocks because one *calls* the other is not a place the set is written down. The class exists so that saying so is a rule rather than an omission. |

**"One file per group" is withdrawn, and so is the rename it implied — the rename is not deferred to PR-1b,
it is dropped.** It was wrong twice over. **(a) It is a repartition, not a rename**: `-common.sh` alone holds
three groups, so renaming files to groups requires deciding per block where each lands — which *is* PR-1b's
move list, so the seam PR-1a defends would collapse. **(b) `-integrity.sh` and `-audit.sh` both hold only
kernel blocks**, so one-file-per-group merges them back into a single file inside the authoring band, undoing
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

## §3b PR-1b — the moves

**Input**: `rederive inventory`'s `MOVE LIST` — the declared blocks whose file contradicts their declaration.
**It does not exist until PR-1a lands**, which is the seam: on a tree with no declarations the harness prints
`MOVE LIST: 0 of 0` and a separate `NO VERDICT` list of undeclared blocks, because a tier guess is not a work
list. PR-1b moves each block to the file matching its declaration, its exit criterion is that `MOVE LIST` is
empty, and it discharges A-i §13's owed `suites` relocation (§7). Nothing else is in it.

⚠ **That criterion is only meaningful if PR-1a-i landed the part-set derivation and retired `PART_SLICE`.**
Both failure modes were simulated end to end. With `PARTS` left as the hardcoded seven (`-audit.sh:325`), the
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
kernel files, so after the six moves `-common.sh` holds them and nothing else and the kernel is four
kernel-pure files. **No block's destination requires choosing between `-integrity.sh` and `-audit.sh`**, so
the primitive/consumer seam `259e12cb` took stays where it is and needs no vocabulary of its own. PR-1b adds
nothing to `-audit.sh`; the band overrun is PR-0's.

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
carries it. #501 then lands on its merits; PR-0 and PR-1a stack after it, where the memos T1 reads are
present and `couplings` is green.

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

This memo therefore creates **no defer slot and defers nothing of its own**: nothing is discarded, and the
question PR-3 answers is blocked on nothing but PR-1. (Checked against
`memory/feedback_defer-slot-eligibility-audit-at-create.md`'s four questions: zero yes. The question has an
owner, a trigger and a place in the forced order, which is what distinguishes it from a slot.)

## §7 Registers, swept by command rather than by anchor

**Line anchors are not usable here** — the memory files other sessions append to daily are stale before the
next reader arrives. The register list is therefore a **query**, and the table says what changes, not where.

```bash
MEMORY=/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory
grep -rn '#505' "$MEMORY"/*.md
gh pr view 501 --json comments --jq '.comments[]|select(.body|test("505"))|.body'
gh pr view 505 --json body --jq .body
git grep -n 'DISCHARGED by A-i' webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-umbrella.md
git grep -n 'Still owed'        webref-cite-audit-tool -- docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md
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
| **PR #505's own body** | invalidated, and **the most public register there is** | it states the harness's file/line/block counts (all now false); that the PR is stacked before A-i; the carve rationale *"29 of 45 harness citations come from memos other than A-i's"*, whose premise M1 falsifies; and — **the statement §4 most turns on** — that the harness here is *"byte-identical to the harness at #501's head … so rebasing #501 onto this branch leaves zero harness delta"*, which `git diff --stat webref-cite-audit-tool -- 'docs/plans/*A-rederive*'` now contradicts by hundreds of lines. The body also carries a `FAILED BLOCKS` line and a block-by-block classification table that `bash …A-rederive.sh all` no longer reproduces. Closing freezes all of it as the public record, so **the close needs a comment**, not a state change |
| **umbrella, the `DISCHARGED by A-i` bullet** | **restored** | it records the harness split as discharged by A-i naming three SHAs; those SHAs are on `webref-cite-audit-tool`, so the harness returning there makes the bullet true again and retires the owed "add a #505 row" amendment |
| **A-i §8** and its layout figures | **restored / re-derived** | §8 is true again once the harness is on A-i's branch. Its figures — part count, per-part sizes, the line total, the block count, the `_measure` call-site census — are already false at HEAD; the check is `bash …A-rederive.sh selfcheck` against `wc -l docs/plans/2026-07-citation-hygiene-A-rederive*.sh`, and re-deriving them is A-i's at landing |
| **A-i §13's owed `suites` relocation** | **discharged at a different destination, and its rule retired** | §13 owes the move *"to `-common.sh`"* **and states the rule it follows** — *"cited by more than one memo → `-common.sh`"*. D11 measures `-common.sh` surviving as a kernel file and `suites` declaring `umbrella`, so the owed move is discharged **to a destination §13 does not name**, and what retires is the citation-count rule, whose other home is the dispatcher header (§3, prose class) |
| **A-i §15's `AUTHOR_LOCAL` quotation and its `readers` note** | invalidated | §3's `authorlocal` class replaces the remote list with adjacent registration and puts `readers` on it, with its own reason (D3) |

**Three classes, not two**: some registers need a pointer repaired, some need an amendment **retracted**, and
some need a **rule** retired.

## §8 What this memo does not get to claim

**A claim is made once, in §1, beside the command that produces it; a claim with no runnable command is not
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

**Authorises**, after `/elidex-plan-review` passes: closing #505 **with a closing comment** (§7) and its
branch **retained**; carrying the transfer set (D10) and this memo pair onto `webref-cite-audit-tool`; and
**PR-0's, PR-1a-i's and PR-1a-ii's scopes as stated in §0 and §3**. It does **not** certify PR-1a-i as a
terminal slice under CLAUDE.md's base case (§2).

**Every PR under this umbrella takes its own `/elidex-plan-review` before implementation** — PR-0, PR-1a-i,
PR-1a-ii, PR-1b, PR-2 and PR-3, per CLAUDE.md's edge-dense rule *(b) 各 PR は実装前に `/elidex-plan-review`
必須*. PR-1a-i's inherits a landed mechanism as ground rather than as a proposal, and should be told so.

**Does not authorise**: changing any block's **subject** on the success path for any block other than `all`,
`homes`, `inventory` and `selfcheck`, whose subject is the block set (behaviour fixes are PR-2, §5); any
removal (PR-3, §6); deleting the `citation-hygiene-harness` branch; editing a status register before §7's
query is re-run; creating a defer slot; **or landing one more line of mechanism on this branch.**

**That last clause is this memo's stopping rule**, and it carries the command that falsifies it. The second
command must print nothing:

```bash
BASE=$(git log -1 --format=%H -- docs/plans/2026-08-citation-hygiene-harness-disposition.md)
git log --oneline "$BASE"..HEAD
git diff --stat "$BASE"..HEAD -- . ':!docs/plans/2026-08-citation-hygiene-harness-*.md'
```

Its scope is the branch, matching the clause: any commit after this memo that touches anything but this memo
pair falsifies it, whether or not its path matches a harness glob.

The reasons mechanism landed ahead of the design, and why they are now spent: the order — land the mechanism
and falsify it before writing the design against it — was **user-ratified as the method for these drafts**,
and a working-tree edit is invisible to review agents, who clone HEAD and whose plants found holes the author
had missed. Neither survives the stopping rule, because an **uncommitted** mechanism can be exercised in a
`git clone --local` sandbox (§1's preamble), which is how D4–D7 and D11–D14 were measured and the form every
further measurement takes. And landing was never scope-free: a commit that moves the census output §3 calls
the work list decides part of PR-1a's scope, and several did. **From here the design is fixed and PR-0 and
PR-1a implement it.**
