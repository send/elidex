# Citation-hygiene harness — the measurements

**This file is §1 of `2026-08-citation-hygiene-harness-disposition.md`**, cut out of it as a standalone
prereq when that memo passed 1000 lines. The disposition and the beta memo cite these entries by `D<N>` and
that citation is unchanged; `rederive homes` resolves every `D<N>` against **this** file's §1 and reds on a
number no bullet here defines.

**Why the seam is here rather than anywhere else.** These entries are a **record of runs**: each says what
was measured, gives the command that re-runs it, and — for D15 through D18 — says explicitly that *the
artifact is not in the tree*. They are provenance, cited by the disposition and deciding nothing on their
own. Everything left in the disposition is a live decision: which slice owns which rule, what each slice
authorises, what the umbrella does not get to claim. The two age differently and are read for different
reasons, which is the same criterion `-audit.sh:9-14` states for its own split and the one `259e12cb` used
to cut `-integrity.sh` on the primitive/consumer seam.

⚠ **The cut is late, and that is recorded rather than argued away.** The disposition entered the 700–800
authoring band at `cdd78dc6` (813 lines) and no seam was cut there; `58643630` added 153 and carried it to
**1007**, past CLAUDE.md's standalone-prereq trigger — in the same commit whose D18 argues that `-audit.sh`
must not be allowed to reach 705, and in the same file whose §9 records `a5fab499` as a violation of exactly
this shape. `memory/feedback_touch-time-split-means-while-writing.md:35` names the collision as evidence of
a miss, not a ground for exemption. Sizes are a command, not a figure here:

```bash
wc -l docs/plans/2026-08-citation-hygiene-harness-*.md
```

⚠ **This file carries no §0.5 / §3 spec coverage map, and `preflight.py` hard-fails on it. That is correct,
not an omission.** The precondition is scoped to *plan* memos, and this one plans nothing: it authorises
nothing, assigns no rule to a slice, and states no scope — check by looking for the two headings every plan
memo in this program has and this one does not:

```bash
grep -c '^## §[0-9]* What this memo authorises\|^## §[0-9]* The work' \
     docs/plans/2026-08-citation-hygiene-harness-measurements.md    # 0
```

The program's spec surface has **one** home, the disposition's `## §0.5 / §3. Spec coverage map`, and the
harness pins the same four citations at `-common.sh:85-88`. Copying that table here would give it a fourth
home to drift in, which is the defect §3's `partset` and `roster` rows exist to close.


## §1 Measurements

The analysis note's M1–M7 are the shared basis; re-run them there rather than restating.

**No expected value is written beside a command in this section.** A figure whose subject is outside this
repository — a memory directory another session appends to, memos on another branch — cannot be stamped with
a commit at all (`memory/feedback_verified-claims-go-stale-under-own-later-edits.md`). The exemption is a
**predicate, not a list of D-numbers**: a figure may be written down here when its subject is a **tree that
does not exist on HEAD** — a `git clone --local` sandbox carrying a planted declaration, a deleted tier, a
widened part set. HEAD carries no declarations at all (`MOVE LIST: 0 of 0`), so no reader can re-derive such
a figure and no commit to this repository can falsify it. Those are recorded falsifications, not values a
reader re-derives today.

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
           | grep -vxF "$(printf "%s\n" $AUTHOR_LOCAL)" | sort' > /tmp/derived
sed -n '/^all() { set --/,/local failed/p' docs/plans/2026-07-citation-hygiene-A-rederive.sh \
  | sed '$d' | tr ' \\' '\n\n' | grep -xE '[a-z]+' | grep -vx set | sort -u > /tmp/literal
comm -23 /tmp/derived /tmp/literal; comm -13 /tmp/derived /tmp/literal
# D9  THE HOMES CENSUS. §3's work list is this command's output.
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes
# D10 the transfer set, DISCRIMINATED rather than asserted: a plain `git log`
#     over the file returns replayed-equivalent commits too.
#     `origin/…`, not the bare name: the bare form is a sibling worktree's local
#     branch and is `fatal: bad revision` in a `git clone --local` sandbox.
git log --oneline --cherry-pick --right-only \
    origin/webref-cite-audit-tool...HEAD -- 'docs/plans/*A-rederive*'
git diff --numstat origin/webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'
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
- **D2 — the rename does change citation surface, and the surface it changes is this memo, not the pair.** No
  slice memo on `webref-cite-audit-tool` cites either helper as a block (the hits there are English prose),
  and the analysis note's only `fixtures` occurrences are the English plural noun, which the needle's own
  discriminator drops. **Every hit is in this memo** — §0.5's command block invokes `fixtures /tmp/fx`
  **bare**, which is the one kind of citation a rename actually breaks and the one a needle matching only
  backticked names and `rederive <name>` misses. **PR-1a-i renames and updates this memo in the same PR**;
  I5 survives because the whole surface is inside that PR, not because there is none.
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
  a one-liner that accepts a declaration today because `HDR='…'` follows it. ⚠ **A comment after it changes
  nothing** — measured, the trim stops at the first non-blank *non-comment* line, so `declared=1 agree=1
  DISAGREE=0` still holds; the predicate is "followed only by blanks and comments", as the preceding clause
  states. **`all` cannot be declared, and its declaration is not even reported as
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
  with no evidence stops answering.** `_misplaced` is one of **three** readers of `PART_SLICE` across five
  sites; the load-bearing one is **T2** (`comp[b], why[b] = PART_SLICE[rows[b]["part"]], "T2 part"`, `-inventory.sh:278`). Re-sourcing T2 off the file
  declaration flips `_wtscan` from
  `A-i` to `kernel` — one of D11's own six movers — because a file declaration is total where `PART_SLICE`
  was partial. **Dropping T2 instead** does not: once `_misplaced` compares block-declaration against
  file-declaration, T2 is literally that same comparison. Measured with T2 removed, D12's file declarations
  in place, and the design-correct declaration on every block a part file defines:

  ```
  MOVE LIST: 6 of 33 declared block(s) / 339 lines     # unchanged; `_wtscan` stays A-i
  declared=33  agree=24  DISAGREE=0 (binding 0)  unverifiable=9  undeclared=2
  undeclared: _measured all        NO VERDICT: 1 (all)        rc=1
  ```

  **`unverifiable` is a rule, not a count.** It is *"no code signal reaches this block"*: D7's six, plus
  `homes`/`inventory`/`selfcheck`, which nothing **calls** — `all` reaches them through its roster, by
  positional parameter. The obvious repair, treating roster membership as a call edge, was measured and
  manufactures **five binding false disagreements** against correct declarations, because a dispatcher calls
  everything and so puts every block in the dispatcher's group. So the rule PR-1a-i implements is: **a
  dispatch edge is not a call edge** — which is what makes the unverifiable set honest rather than a number
  to argue down. Today T3 manufactures `kernel` for exactly those blocks, so a declaration nothing can
  confirm reports as **agreeing**; `unverifiable=` replaces that with no answer, and the two blocks §3 must
  still solve stop being silent — they come out at `rc=1`. The transcript is what the sandbox **printed**,
  which is a tree with no dispatcher file declaration; adding one (§3, `groupvocab`) is what stops `all`'s
  file side resting on an assignment no rule has made, and is what removes the `NO VERDICT` limb. No run here
  attests that edit, so the printed line stands.

- **D15 — the T0 sentinel is not a check that reproduces a declaration, and neither of the two branches
  earlier drafts posed is right.** Two reviewers argued independently that with the dispatcher declaring
  `# group: kernel`, T0 would reproduce that declaration by construction and so could never disagree.
  Measured, false twice: T0 **can** disagree — a wrong `# ships-with: A-ii` on `all` prints
  `!! all declares A-ii but the tiers compute kernel (T0 dispatcher)`, `DISAGREE=1 (binding 1)`, `inventory`
  rc=1 — and T0 **reads no declaration at all**, being `if rows[b]["part"] == "(disp)": comp[b] = "kernel"`
  (`-inventory.sh:273-274`), so a wrong `# group: A-ii` on the dispatcher leaves `agree=1`, rc=0. Deleting it
  is worse: `all` falls onto the Python regex inside the quoted `INVENTORYPY` heredoc (`-inventory.sh:82`),
  which the dispatch-edge rule correctly calls non-evidence, and the same wrong-declaration plant goes
  **rc=1 → rc=0**. The tier that answers each part's own `# group:` binds on both sides, catches the file
  declaration keeping T0 misses, moves all ten of `-Aii.sh`'s blocks onto the identical tier with no `ships`
  value changing, and removes the string `(disp)` from the harness. ⚠ **Its rank is below T1** — the
  measurement that suggested otherwise was taken with one part declaring; with every part declaring (which
  γ's own RED rule forces) six of thirty-five blocks change group, because the file tier answers *which file
  holds it* and T1 answers *whose concern it is*. Reproduce, in a throwaway clone, one edit per run:

  ```bash
  git clone -q --local --no-hardlinks . /tmp/d15 && git -C /tmp/d15 checkout -q HEAD
  # (i) admit the dispatcher to the part set as stem `disp`; (ii) plant `# group: kernel` on its
  # preamble and `# ships-with: kernel` on `all`; (iii) vary one declaration per run.
  perl -e 'alarm 240; exec @ARGV' bash /tmp/d15/docs/plans/2026-07-citation-hygiene-A-rederive.sh \
      inventory /Users/kazuaki/repos/send.sh/elidex-wt-citeaudit/docs/plans
  ```

  ⚠ **The sandbox is not committed and the tier is not in the tree.** γ builds it; this entry records what was
  measured and how, not an artifact anyone can run today.

- **D16 — the `prose` class has a subject test, its unit is the sentence, and the two kinds this memo named
  are not exhaustive.** Over the class's 39 census rows the split was recorded as **19 placement / 15
  rationale / 5 straddling**. ⚠ **The first two of those three do not reproduce.** Re-implemented from the
  letter of §3's `prose` row across six readings — two readings of *"part preamble"* × three
  sentence-terminator rules — the split came out 14/20 under the most literal reading and 21/13 under the
  reading that makes the surrounding prose true; no reading reached 19/15, and no treatment of straddlers
  reached the 849 control figure. **The straddler count of 5 reproduces, by identity.** The two figures are
  retained as a record of the run that produced it, not as ε's specification; ε measures its own. The test
  refuses one row loudly and defers four to a human. Two structural results survive the non-reproduction. **Five rows carry the tail
  of one sentence and the head of another** (`-Aii.sh:7`, `-B.sh:7`, `-audit.sh:92`, `-common.sh:10`,
  `A-rederive.sh:47`), stable under every terminator rule tried, so a per-line needle is wrong on them by
  construction. And the class holds a **third kind** beyond residence and block-as-subject-of-an-event —
  measured quantities (`A-rederive.sh:47-48`), a source-order invariant (`:46`, the sentence §3's `partset`
  row cites as `:44-49`) and a tier-algorithm specification (`-inventory.sh:266`) — none of which the rule
  may touch. ⚠ **The place vocabulary must be derived from `PARTS`, not spelled**: the first draft of the test
  wrote the stems out and the census reported *the test itself* at `?`, rc=1 — the needle had become a home
  of the fact it censuses. Out of sample, over the 911 harness comment lines (39 census rows + 872 others),
  it returns `rationale` for 849 of the 872 and a placement verdict only for continuations of the same
  placement sentences. Reproduce:

  ```bash
  bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | awk '$5=="prose"'   # the 39 rows
  cat docs/plans/2026-07-citation-hygiene-A-rederive*.sh | grep -cE '^\s*#'          # 911
  ```

  ⚠ **The test is not in the tree.** **ε** builds it (the re-slice moved it out of β), and §3's `prose` row
  states what it must carry — carrying the straddler count, which reproduces, and not the two that do not.

- **D17 — PR-1a-i's exit criterion cannot read the census, because the census shrinks when the work lands.**
  `RULED BY THE PLAN: N of N` ranges over the **observed** class set, so finishing a class removes it from
  numerator and denominator alike: measured, replacing the dispatcher's roster literal with a derivation
  prints `RULED BY THE PLAN: 8 of 8` at **rc=0** with `roster` gone from `BY CLASS` entirely. And `homes`
  never raises on a failure to collapse — its raises are part-count, sourcing, vocabulary, memo-parse,
  zero-homes, unclassified-home, unruled-class and unexecuted-claim, every one of them firing on *adding*
  something. So the criterion asserts over **edit sites**, and uses the census only where the observable is
  per row and cannot shrink into a false green. Two traps measured while building it: **a literal-gone needle
  tests a spelling** — `^MEMOS = \[` still matched `MEMOS = [(g, s) for g, s in MEMOSET …]`, reporting the
  enumeration present when a comprehension over the derivation was what was present — and **a census class
  count of 1 can be right for the wrong reason**, since after a real `memoset` collapse the surviving row is
  the *consumer* and the derivation's own body is not classified `memoset` at all. Reproduce the shrink:

  ```bash
  git clone -q --local --no-hardlinks . /tmp/d17 && git -C /tmp/d17 checkout -q HEAD
  # replace the dispatcher's `all() { set -- …` literal with a `$(_roster)` call, then:
  perl -e 'alarm 240; exec @ARGV' bash /tmp/d17/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes
  ```

  ⚠ **The criterion is not in the tree either**, so §3a's measured-behaviour figures describe a checker that
  does not exist here. Each slice's own plan-review lands its share.

- **D18 — the ⊕ provenance checker is LANDED, in `-inventory.sh`, and the "deadlock" an earlier draft of this
  entry declared was false**, which `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh attest` shows by
  running. ⚠ **What that draft got wrong is recorded before what it got right**, because
  three review axes measured it independently and the error was load-bearing: it deferred the checker to α on
  a stated impossibility.
  - ⚠ **"Every available cut moves the census" was a universal over two enumerated cases.** Measured, placing
    the block in an **existing** part moves nothing: `BY CLASS`, `HOMES:` and `RULED BY THE PLAN` are
    byte-identical to base with the block wired onto `all`'s roster. Two reviewers found this independently,
    on `-inventory.sh` and on `-integrity.sh`. Measuring N cases and writing *every* is a second claim, and it
    needs its own command (`memory/feedback_universal-claims-need-the-complement-measured.md`).
  - ⚠ **The reason it gave for the new-part case was also false, and the true one is already ruled in §3.**
    It said a new part "relocates the gate's own homes"; the gate region has **zero** census rows. What
    actually moves the census is the **stem**: `VOCAB` is the block names union `PARTS` (`-audit.sh:73`), so a
    new stem turns a previously-invisible line into a home — measured for three stems, a different line each
    time. §3's `partset` row already states the remedy (*the stem is chosen against the census*), which that
    draft did not apply. **So a new part is available too, once a stem is chosen that way**; it is not
    available *cheaply*, because it must also join the part set by hand in both hardcoded homes until
    `partset` lands.
  - ⚠ **The band figure was taken at a placement the same entry had ruled out.** It measured 705 for
    `-audit.sh` — the file whose own seam (`-audit.sh:9-14`) excludes memo-subject checks — and then made
    705-in-the-band the blocker. The blocker was never size.

  **The placement decision, which lives here because §3's rows are where placement is ruled and a second copy
  in the harness would be one more home to drift**: the block's subject is the memos, which is the criterion
  by which `inventory` is not in `-audit.sh` either, so it goes on `inventory`'s side of that seam. ⚠ **Its
  true home is a part of its own** — `-inventory.sh`'s own charter is the call graph and the sibling branch's
  memos, not this one's — and that part is the slice which collapses the part-set literals and can pick a
  census-safe stem. An earlier draft of the block's comment argued this inline, and the census correctly filed
  the line as a `prose` placement home: the rule that placement belongs to the memo has a checker, and it
  fired on the violation while it was being written.

  **What the block is** is stated where it runs, with its four blind spots — two declared when it was written
  and two measured by the review that read it — and is not restated here. Run it, and its controls:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh attest              # rc=0, per-memo population
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
printf '\n⊕ A claim with no command near it.\n' >> "$d"/docs/plans/2026-08-citation-hygiene-harness-disposition.md
bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh attest         # rc=1, findings=1
```

  ⊕ Its yield on the pair when it was first run, against the tree
  `git show d9a353a0:docs/plans/2026-08-citation-hygiene-harness-1a-i-beta-classifier.md` returns —
  **a record of that run, not a figure this entry predicts**:
  eleven attestations carried no command anywhere in their block, nine in the beta memo and two in the
  disposition, and **two of the eleven were in a paragraph written twenty minutes earlier in the same
  session** (`memory/feedback_findings-cluster-in-self-added-scope.md`, live and self-inflicted). All eleven
  were fixed. ⚠ **Today's population and yield are not written here** — this entry's own marks move both, so
  the figure is whatever the block prints
  (`memory/feedback_document-landing-invalidates-its-own-measurements.md`).

  ⚠ **A third control matters more than the two above and is the reason the block reds on an empty
  population**: the check's subject set is *lines carrying the mark*, i.e. the author's own vocabulary rather
  than the property being checked, so deleting every mark would otherwise report clean
  (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`). ⊕ Measured — strip the mark
  from every memo in a clone and run the block:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
for m in "$d"/docs/plans/2026-08-citation-hygiene-harness-*.md; do
  python3 -c 'import sys,pathlib;p=pathlib.Path(sys.argv[1]);p.write_text(p.read_text().replace(chr(0x2295),""))' "$m"
done
bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh attest    # attestation=0, rc=1
```

  That closes silencing-by-deletion
  and closes nothing else — **a false measured claim written without the mark is still invisible**, and that
  is the larger class. The checker for *that* is not this one and is not in the tree.
