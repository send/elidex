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

⚠ **The cut is late, and the first version of this record understated how late — in the flattering
direction.** It said the disposition *"entered the 700–800 authoring band at `cdd78dc6` (813 lines)"*. ⊕
Measured, `for c in b4a9a2a4 d85abd5e cdd78dc6 73ba71db 5090a95a 58643630; do git show $c:docs/plans/2026-08-citation-hygiene-harness-disposition.md | wc -l; done`
gives **734 / 743 / 813 / 831 / 854 / 1007**: it **entered** the band at `b4a9a2a4` and **left** it at
`cdd78dc6`. So 813 is the exit, not the entry, and four cut points were missed rather than one. §3 states the
same record correctly for `-audit.sh` (`979e5426` 778, `49b4f645` 808), so the form was understood and
misapplied only where the miss was the author's. `58643630` then added 153 and carried it to **1007**, past
CLAUDE.md's standalone-prereq trigger — in the same commit whose D18 argues that `-audit.sh`
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

⚠ **That check does not discriminate, and three review axes said so.** `predicate-collapse.md` also scores
**0** and `preflight.py` **passes** it, because what preflight reads is the spec-coverage-map heading. The
needle is a negative control this file satisfies, not a predicate separating it from a plan memo. **The
ground is the positive one**: this file authorises nothing and assigns no rule to a slice — a property of
what it says, not of a heading — and preflight's precondition is scoped to memos that do.

⚠ **The program's spec surface already has more homes than one, and an earlier draft of this paragraph
said it had one while its own next sentence said a copy would be the fourth.** ⊕ Measured —
`grep -l '^## §0.5' docs/plans/2026-08-citation-hygiene-harness-*.md` returns the disposition,
the beta memo and the predicate-collapse memo, each carrying its own `## §0.5 / §3.` table, and the harness
pins the same four citations a fourth time at `-common.sh:85-88`. Adding one here would make five. **That is
the state, not the design**: it is the same many-homes defect §3's `partset` and `roster` rows exist to
close, one altitude up, and no rule row rules it. It is not this file's to fix and not this file's to join.


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
  charter (`-inventory.sh:376-377`: *written-and-unread must never come out as "undeclared"*; ⚠ an earlier draft attributed it to `-audit.sh`, where `grep -c 'written-and-unread'` returns **0**) against the block §3's own
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
  of the fact it censuses. Out of sample, over the harness comment lines that are not census rows (⚠ **the figures this entry carried — 911 and 872 — moved when this session added comment lines to the parts; re-derive with `grep -h '^\s*#' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l` less the census's `prose` row count**),
  it returned `rationale` for all but a handful of continuations of the same placement sentences. ⚠ **The two
  figures that stood here (849 of 872) are not carried**: both moved with this branch's own comment lines, and
  the warning above says so about the second while a draft left the first three lines below it. ε measures its
  own. Reproduce the population:

  ```bash
  bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | awk '$5=="prose"'   # the 39 rows
  cat docs/plans/2026-07-citation-hygiene-A-rederive*.sh | grep -cE '^\s*#'          # today's total
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

- **D18 — the ⊕ provenance block is written, measured, and REVERTED. It cannot land on this branch, and
  every reason this entry gave for that before round 5 was false.** The block was landed at `84a7bd67`,
  reverted after five review axes measured the landing, and its source is below so the slice that lands it
  does not rewrite it. ⊕ The revert is complete: `git diff 84a7bd67^ -- docs/plans/2026-07-citation-hygiene-A-rederive-inventory.sh docs/plans/2026-07-citation-hygiene-A-rederive.sh`
  prints nothing, and `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh selfcheck` reports 34 blocks
  on 24 roster entries again. ⚠ **The sequence of wrong reasons is recorded because the entry kept producing a new
  one rather than the check that would have settled it.**

  | draft said | measured |
  |---|---|
  | "a new part relocates the gate's own homes" | the gate region has **zero** census rows |
  | "**every** available cut moves the census" | a universal over two enumerated cases — two reviewers each found a third |
  | "a new **stem** moves it — three stems, a different line each time" | ⚠ **the probe was contaminated by itself.** Its part defined a block named `x`, and `-Aii.sh:269` reads *"4 capability states x $n_fx fixtures x 2 modes"*, so the multiplication sign was the vocabulary hit and every stem produced the same row. With `_probeonly`: `attest` **70**, `memoclaims` **70**, `zzq` **70** — census-safe stems exist |
  | "placing it in an **existing** part moves nothing" | ⚠ **measured on three named lines rather than on the output.** `BY CLASS`, `HOMES:` and `RULED` are byte-identical, and the census output is **not**: `V = 41 tokens` → `42`, and `all()`'s roster row moves `enumerates 10 of V` → `11 of V` |

  **What §9 actually says, and what it decides.** The clause forbids *landing mechanism that decides what a
  PR decides*, operationalised in this section as *a commit that moves the census output §3 calls the work
  list*. The output is what `rederive homes` prints, not three lines chosen from it. ⊕ Run the whole thing
  either side and diff it:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
git -C "$d" checkout -q 84a7bd67^ && bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > "$d/before"
git -C "$d" checkout -q 84a7bd67  && bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > "$d/after"
diff "$d/before" "$d/after"        # V 41->42; roster row 10 of V -> 11 of V; POPULATION moves
```

  The roster literal is what this memo's own preamble calls the single most important home, and §2 assigns
  that surface to **α**. §9 grades `a5fab499` a violation on a strictly analogous delta. So the landing fails
  the clause, and reverting rather than recording-and-absorbing is the difference between a **forced**
  violation and an **elective** one: `a5fab499` was a split at 1059 lines with nowhere else to go; this was
  optional and justified by a measurement that did not measure it.

  ⚠ **The placement argument misquoted the seam it cited.** It said `-audit.sh:9-14` *"excludes memo-subject
  checks"*. It does not: it excludes checks that **source the parts** and take their **authority from memos
  on another branch**, and `homes`'s own memo-quantity gate — a memo-subject check over the identical glob
  — is in that file. ⊕ Read it: `sed -n '9,14p' docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh`.
  Measured against both files' stated criteria the block belongs with `homes`, and the reason it was put
  elsewhere was that `-audit.sh` is 655 and would cross the authoring band — a size reason, presented as a
  charter reason. **Where it lives is §3's to rule**, not this entry's; this entry records the runs.

  ⚠ **A review axis read the landing as falsifying β's "all three" heredoc hosts. It does not, and the
  refutation is recorded because the wording invites the reading.** §3's `roster` row scopes the three to the
  three **readers of `all`'s roster** — `homes`, `inventory`, `selfcheck` — not to every argv-plus-heredoc
  payload. ⊕ `grep -n 'python3 - "$REPO_ROOT' docs/plans/2026-07-citation-hygiene-A-rederive*.sh` returns four
  such payloads even after the revert (`-audit.sh:53`, `-audit.sh:597`, `-inventory.sh:39`, `-common.sh:118`), and
  `_wtscan` at `-common.sh:118` reads no roster; neither did `attest`. ⚠ **A first draft of this paragraph
  accepted the axis's finding and added `-common.sh:118` as a fourth site β had missed — which is the same
  error one level down**, since that payload fails the row's criterion too. What is real is the **label**: β
  §3a O1 says *"heredoc host sites"* where the row means *the roster readers' payloads*, and a reader
  applying the phrase rather than the row gets four. The three sites it lists are right; `:538` in that list
  is not, and is corrected in β. An obligation stated as a
  cardinal, and a commit inside the review window that changed it.

  ⚠ **The yield this entry recorded does not reproduce with this block.** It said eleven attestations carried
  no command, *"nine in the beta memo and two in the disposition"*, and attached a `git show` — which prints
  a memo and runs no check, the block's own blind spot (2), in the entry that declares it. ⊕ Run the block
  against the tree it named and it prints `findings=0`: the eleven were found by a prototype patched into
  `homes`, and **fixed before `d9a353a0` existed**, so no tree this entry names carries them. What is true and
  checkable is that the class was real and the prototype found it; the count belongs to a tree that was never
  committed.

  **The block, so the slice that lands it does not re-derive it.** Its unit is the markdown block; a command
  is a backticked span of two or more tokens whose first resolves on `PATH`, which is behaviour rather than a
  list of verb names (`memory/feedback_enumerated-exemptions-leave-the-next-class-authoritative.md`).

```bash
python3 - docs/plans <<'ATT'
import re, shutil, sys
from pathlib import Path

HD = Path(sys.argv[1])
M8 = sorted(HD.glob("2026-08-citation-hygiene-harness-*.md"))
if not M8:
    raise SystemExit("!! no `2026-08-citation-hygiene-harness-*.md` under %s; a population of zero "
                     "would report clean for the reason a needle matching nothing does." % HD)

def scan(md):
    """(lineno, text, fenced) -- a ``` fence line counts as fenced on BOTH sides, so a
    legend DEFINING the mark inside a fence is not read as an item USING it."""
    fen = False
    for i, s in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
        f0 = s.lstrip().startswith("```")
        fen = fen ^ f0
        yield i, s, fen or f0

def blocks(md):
    """Blank-line-delimited blocks, with a following all-fenced block merged into the
    one above it: a fence is nobody's item on its own, and the sentence that
    introduces it is separated from it by a blank line."""
    out, cur = [], []
    for i, s, fen in scan(md):
        if not s.strip() and not fen:
            if cur:
                out.append(cur); cur = []
        else:
            cur.append((i, s, fen))
    if cur:
        out.append(cur)
    merged = []
    for b in out:
        if merged and all(f for _, _, f in b):
            merged[-1] = merged[-1] + b
        else:
            merged.append(b)
    return merged

def runnable(blk):
    for _, s, fen in blk:
        if fen and s.strip() and not s.lstrip().startswith("```"):
            return True
        for span in re.findall(r"`([^`\n]+)`", s):
            tok = span.split()
            if len(tok) > 1 and shutil.which(tok[0]):
                return True
    return False

pop, bad, per = 0, [], {}
for md in M8:
    n = 0
    for blk in blocks(md):
        att = [(i, s) for i, s, fen in blk if not fen and "⊕" in s]
        if not att:
            continue
        ok = runnable(blk)
        for i, s in att:
            for _ in range(s.count("⊕")):
                pop += 1; n += 1
                if not ok:
                    bad.append((md.name, i))
                    print("   !! %s:%d  ⊕ attests a measurement and its item carries no command: %s"
                          % (md.name, i, s.split("⊕", 1)[1].strip().replace("**", "")[:58]))
    per[md.name] = n
print("   PER MEMO: %s" % "  ".join("%s=%d" % (k.replace("2026-08-citation-hygiene-harness-", ""), v)
                                    for k, v in sorted(per.items())))
print("   POPULATION: ⊕ attestation=%d   findings=%d" % (pop, len(bad)))
if pop == 0:
    raise SystemExit("!! %d memo(s) read and not one carries the mark; the convention would then be "
                     "checked by a needle that matches nothing." % len(M8))
if bad:
    raise SystemExit("!! %d attestation(s) with no command in their own block. Do not remove the mark "
                     "to silence this -- the convention is that the command travels with the claim."
                     % len(bad))
ATT
```


  ⚠ **Three controls, and the third is why the block reds on an empty population.** A bare mark reds; a mark
  with a command stays clean; and **stripping every mark from every memo reds rather than reporting clean** —
  ⊕ measured by stripping the mark from a copy of the memos in a throwaway tree —
  `python3 -c 'import pathlib,glob;[p.write_text(p.read_text().replace(chr(0x2295),"")) for p in map(pathlib.Path, glob.glob("docs/plans/2026-08-citation-hygiene-harness-*.md"))]'`
  — and then running the block: it prints `attestation=0` and exits **1**. ⚠ **A first version of this
  attestation wrote the same strip as a `for` loop**, whose first token is a shell builtin and so does not
  resolve on `PATH`; the block read it as no command and reported the line. Blind spot (1), caught by the
  block on the sentence that introduces the block. That third control
  exists because the population is the author's own vocabulary, so without it the convention could be
  silenced by deletion. ⚠ **A draft of this entry inlined a
  source that lacked it** and would have handed the next slice a block that reports clean on zero marks; the
  source below is the one that ran at `84a7bd67`, extracted from the commit rather than retyped.

  ⚠ **Five blind spots, and the fifth was measured by the review that read the landed block**: a shell
  BUILTIN as the first token reads as no command; a command that runs but measures a **different** claim
  passes; **the same false claim written without the mark is invisible** (the population is the author's
  vocabulary, not the property); a mark inside a fence leaves the population while a mark inside an inline
  code span stays in it; and — new — **`blocks()` splits on blank lines, so a markdown table is one block**,
  and one runnable span in any row discharges every mark in every row. β §3a's O5 mark passes on O2's row.
  The legend's *"carries the command IN ITS OWN BLOCK"* is therefore stronger than what is checked.

- **D19 — a checker for false COMPLETENESS claims is not constructible as a needle over prose, and four
  probes say so.** Round 4 measured **nine** false completeness claims across the memo pair in two commits —
  *every available cut*, *only figures that reproduce*, *PR-1a-i no longer exists*, *one item is owed*,
  *sixteen `elif` hits are Python*, *both `partset` rows are literals*, *Every hit is in this memo*,
  *§5 authorises it*, *§9 no longer resolves through §3*. That is the class this program is actually losing
  to, and the `attest` block (D18) does not reach it: its population is *lines carrying the mark*, so a false claim
  written without one is invisible. This entry is the attempt to key a check to the property instead, and its
  result is negative. ⚠ **It is recorded because an unrecorded negative gets re-attempted**, and because each
  probe's population is the evidence for the next design.

  | probe | predicate | population | why it fails |
  |---|---|---|---|
  | 1 | any sentence carrying `every` / `all` / `no` / `none` / `only` / `both` / `each` / `never` | **431 of 835 sentences** | over half the prose. A check this broad gets switched off, which is the reason `-audit.sh` gives for keeping its own length needle narrow |
  | 2 | a bold numeral in the paragraph introducing a fence, compared against the fence's output line count | **5 of 40 fences** | too small to justify executing arbitrary fences, and it reaches none of the nine |
  | 3 | an emphasised token followed by an absence phrase (*no longer exists*, *is gone*, *is retired*) | **5**, of which 4 are grammatical accidents (`subsumes`, `arises at`, `step 1's output`) | the subject of a natural-language absence claim is not recoverable by proximity. ⚠ **It misses `PR-1a-i no longer exists`, the very instance it was written for** — that subject is bolded prose, and widening to bold returns a complement of 13 that is entirely noise (`and is gone`, `the does not exist`) |
  | 4 | `§N` plus *authorises / says / states / lists*, checked against that section | **22**, resolvable 21 | existence is checkable and yields nothing; the one unresolved is a legitimate cross-branch cite of A-i's §13. **It does not reach `§5 authorises it`**, because §5 exists — the defect was its *content*, and what "it" refers to is not recoverable |

  ⚠ **An earlier draft offered a two-command block under the words *"reproduce any of them"*, and it
  reproduced none of the four.** The first command was `grep -c ''`, which prints **line** counts, not the
  sentence, fence or claim populations the table states; the second returned a figure that did not match
  probe 4's row. ⊕ The cause, measured at `d3fcc009`: the needle returns **17** over the memo set and **5**
  over the harness scripts, and probe 4's **22** ranged over both while the reproduce command ranged over the
  memos alone. ⚠ **A first correction of this paragraph asserted the scripts contribute zero** — written
  without running the command it was describing, which is the failure this table is about, two layers down.
  ⚠ **No fixed integer belongs beside these rows in any case**: the population is self-referential, since
  this entry's own prose carries `§N … states` spans, so every edit to it moves the figure. The command is
  given so a reader gets today's, over the memo set alone:

⚠ **Probes 1-3 carried figures and no command, which §8 says is not a claim made** — the rule is *"a claim
with no runnable command is not made"*, and it applies to this table as much as to any other. Each probe now
carries the command that produces its population, and, like probe 4, **no fixed integer**: every population
here is self-referential, so this entry's own prose moves all four.

```bash
cd docs/plans
# probe 1 -- sentences, and those carrying a universal
python3 -c "
import re,glob
t=' '.join(open(f).read() for f in sorted(glob.glob('2026-08-citation-hygiene-harness-*.md')))
s=[x for x in re.split(r'(?<=[.!?])\s+', t) if x.strip()]
u=[x for x in s if re.search(r'\b(every|all|no|none|only|both|each|never)\b', x, re.I)]
print('sentences=%d universal=%d' % (len(s), len(u)))"
# probe 2 -- fences, and those whose introducing paragraph carries a bold numeral
python3 -c "
import re,glob
tot=hit=0
for f in sorted(glob.glob('2026-08-citation-hygiene-harness-*.md')):
    L=open(f).read().splitlines(); i=0
    while i<len(L):
        if L[i].startswith('\`\`\`'):
            tot+=1
            if re.search(r'[*][*][^*]*\b\d+\b[^*]*[*][*]', ' '.join(L[max(0,i-6):i])): hit+=1
            j=i+1
            while j<len(L) and not L[j].startswith('\`\`\`'): j+=1
            i=j+1
        else: i+=1
print('fences=%d bold-numeral=%d'%(tot,hit))"
# probe 3 -- an emphasised token followed by an absence phrase, and the bare phrase
grep -oiE '[*][^*]{1,40}[*][^.]{0,30}(no longer exists|is gone|is retired)' \
     2026-08-citation-hygiene-harness-*.md | wc -l
grep -oiE '(no longer exists|is gone|is retired)' \
     2026-08-citation-hygiene-harness-*.md | wc -l    # the complement probe 3 cannot subject-test
# probe 4 -- memo set only
grep -oE '§[0-9]+[a-z]?[^.]{0,40}(authorises|says|states|lists|names)' \
     2026-08-citation-hygiene-harness-*.md | wc -l
```

⚠ **No figure is quoted beside them, and the first draft of this paragraph quoted four.** Those four were
measured minutes before this entry was written and the writing itself moved every one of them — the defect
the entry is about, committed inside the commit that fixes it. What survives is the checkable statement:
**run the fence and compare against the table's own 431/835, 5/40, 5 and 22; not one of the four still
holds**, and the table keeps them as what was true when it was written rather than as what is true. Probe 3's
second command is the point of its row: the bare absence phrase is several times commoner than the emphasised
form the probe can subject-test, so the probe reaches a fraction of its own class.

  ⚠ **What the four probes agree on**: the failures are not identifiable from the *claim*, only from the
  *subject set*, and prose does not name its subject set in a recoverable way. What closed the attestation class was a
  **convention plus a checker over the author's own mark**, and a second mark for completeness claims would
  inherit the same hole — it catches the claims the author already suspected. ⊕ Measured, that hole is
  control (b) of D18's block: a false measured claim written without the mark passes, verified by planting
  one — `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh attest` stays green.

  **What is left is not a needle but a set of habits, and the first draft of this paragraph got their number
  wrong in the way this entry is about.** It said *"the two habits the nine instances share — every one of
  them either (a) universalised over a hand-enumerated set or (b) asserted a sibling section's content without
  opening it"*. ⚠ **That is a false completeness claim inside the entry about false completeness claims**,
  found by auditing this session's own added prose rather than by a reviewer. Three of the nine fit neither
  branch. Classified against the nine:

  | habit | instances | what actually happened |
  |---|---|---|
  | (a) universalised over a set enumerated **by hand** | *every available cut*; *PR-1a-i no longer exists*; *one item is owed* | two cuts became *every* cut; six swept sites became *the* dissolved owner |
  | (b) asserted a **sibling section's** content without opening it | *§5 authorises it*; *§9 no longer resolves through §3* | §5 lists three items and none is the one claimed. This is the branch that reached a commit message |
  | (c) misread a **member** of a set the machine had enumerated correctly | *sixteen `elif` hits are Python*; *both `partset` rows are literals* | the grep and the census both printed the full set; `-Aii.sh:281` is a shell alternation and `A-rederive.sh:52` is a `for` loop. **The enumeration was right and the per-member judgement was wrong**, which no enumerating command can catch |
  | (d) wrote a clause about an object without **sweeping that object** | *only figures that reproduce* | the clause landed in the same edit as the row it ranges over, and `849` survived three lines below it |
  | (e) attached the command and **did not run it** | *Every hit is in this memo* | the needle was written out; running it returns four hits in the other memo |

  ⚠ **(c) is the one that matters most and the one the first draft erased.** Habits (a), (b) and (e) are all
  *failures to enumerate*, and a reviewer can catch them by enumerating. (c) is a failure **after** correct
  enumeration, so no command reaches it — which is the real reason this class has no checker, and a stronger
  reason than the one the four probes give. ⚠ **"No slice takes this" is not a disposition on its own, and an earlier draft left it as one.** The
  create-time rule requires the four questions before declining a slot, and question 4 — repeat signal — is
  unambiguously **yes** here: this class is in the memory ledger and was escalated from five instances in a
  session to nine. Run against `memory/feedback_defer-slot-eligibility-audit-at-create.md`: (1) is it
  discarded work? **no** — nothing is dropped, the habits are stated. (2) does it block a named PR? **no**.
  (3) is there a mechanism to build? **no** — D19's four probes and a reviewer's fifth all measure that there
  is not. (4) repeat signal? **yes**. One yes, and it is the one that forbids silence.

  **So it is recorded as an accepted cost with its price named, not as a non-slot.** The price: habits (a),
  (b) and (e) are failures to enumerate and a reviewer catches them by enumerating — five axes did, nine
  times in one round. Habits (c) and (d) are failures *after* correct enumeration and no reviewer command
  reaches them either; they are caught, when they are caught, by a second person reading the sentence beside
  the output. **Trigger for re-opening**: a round in which habit (c) or (d) accounts for more findings than
  (a), (b) and (e) together — at that point the reviewing loop has stopped being the mechanism and something
  else has to be. Until then this entry, and the `attest` block's blind spot (3), are where the cost is
  written down.

- **D20 — how §9's stopping rule came to be worded as it is.** Cut out of the disposition's §9 as a
  standalone prereq when that memo re-entered the 700–800 authoring band at **702**. ⚠ **The seam is the same
  one this file was created on**: §9 keeps the clause, the authorisation, the plan-review population and the
  per-commit grades — every forward-binding decision — while the *history of how the wording was arrived at*
  is a record of runs, which is what this file is for. Nothing below binds a slice; §9 cites it as D20.

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

- **D21 — every commit the §9 falsifier prints, graded by the membership test.** Cut out of the disposition's
  §9 while writing, because that section entered the authoring band; §9 holds the rule and the verdicts, this
  entry holds the measurement. ⚠ **§9 previously excused eight of these as *"nine ... not graded"*** — a
  count of eight names plus *"`dae569d4`'s own parent chain"*, which enumerates nothing, and an ungraded
  population under a stopping rule is a compliance claim nobody made. ⊕ Run against each commit's own parent:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
BASE=$(git log --diff-filter=A --format=%H \
       -- docs/plans/2026-08-citation-hygiene-harness-disposition.md | tail -1)
for c in $(git log --format=%h "$BASE"^..HEAD --reverse \
           -- . ':!docs/plans/2026-08-citation-hygiene-harness-*.md'); do
  git -C "$d" checkout -q "$c^" 2>/dev/null && \
    a=$(bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes 2>/dev/null | grep -oE 'HOMES: [0-9]+')
  git -C "$d" checkout -q "$c" 2>/dev/null && \
    b=$(bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes 2>/dev/null | grep -oE 'HOMES: [0-9]+')
  printf '%s %s -> %s\n' "$c" "${a:-none}" "${b:-none}"
done; rm -rf "$d"
```

| commit | homes | verdict |
|---|---|---|
| `90e1429b`, `9a0ff039` | no `homes` block yet | **passes** — vacuously; there is no work list to move |
| `fc47cde1` | (none) → **75** | moves — **it CREATES `homes`** |
| `259e12cb` | 75 → **77** | moves — the integrity/primitive split, before the rules settled |
| `7ad42edd` | 77 → **70** | moves — R3 gains its subject test |
| `979e5426` | 70 → **72** | moves — `homes` starts assigning the CLASS |
| `49b4f645`, `adb8a33b` | 72 → 72 | **passes** |
| `b088dacc`, `96d8fae3`, `2abaea1b`, `84a7bd67`, `dae569d4`, `bd52f296` | unmoved | **passes**, as graded above |
| `a5fab499` | 72 → **70** | **violates**, as graded above |
| `473b9d56`, `9f0fe33d` | 70 → 70 | **passes** — ⚠ **both fall under this clause and neither was graded until now** |

  ⚠ **The count is not pinned here either** — the falsifier's population grows with the branch. It printed
  thirteen commits when §9 was written and **seventeen** when this entry was added, and the two the growth
  brought in (`473b9d56`, `9f0fe33d`) were under the clause and ungraded until this measurement.
