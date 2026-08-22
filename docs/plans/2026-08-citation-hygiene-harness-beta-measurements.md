# Citation-hygiene harness PR-1a-i-β — §1 Measurements

**Cut out of `2026-08-citation-hygiene-harness-1a-i-beta-classifier.md` while writing**, because round 7's
corrections took that memo to 705 lines and §4's rule is to cut the seam at the 700-800 authoring band
rather than let a later split decide a PR's scope. The seam is the one this program has cut three times
already (`2abaea1b`, `9647ba4d`, `9c66be4d`): **measurements leave, the section that reasons over them
stays**. β's §1 is a forwarding stub pointing here; every ⊕ mark and every command below is unchanged, and
the numbering (β1…) is kept so the citations into it still resolve.

⚠ **This file is a sixth `2026-08-citation-hygiene-harness-*.md`, so it enters the memo gate's own glob.**
That is the population §5 β5 records an arity defect for; the cut was run against the gate before landing.

## §1 Measurements

Numbered **β1…**, not `D<N>`: the harness's memo gate resolves every `D<N>` in any
`2026-08-citation-hygiene-harness-*.md` against **the measurements memo's** §1 (`-audit.sh:446-453`), so a
fresh `D`-number here would dangle — ⚠ **an earlier draft named the umbrella's §1**, which is a forwarding
stub with zero bullets, so following the sentence found no definitions —
measured, by writing one and watching this memo's own first gate run red.

⚠ **Provenance convention**, stated as a legend rather than in prose, because a line that *defines* the
mark is not a line that *uses* it and a checker cannot tell the two apart:

```text
⊕   this item was re-measured by this memo's author, and the item carries the command IN ITS OWN BLOCK
```

⚠ **"or in §3" is gone, and its removal is the whole change.** A previous draft let an item point at a
command elsewhere; measured, that escape is where nine of this memo's own attestations ended up — a mark with
no command anywhere near it, indistinguishable to a reader from one that has one. The command travels with
the claim or the claim drops the mark. ⚠ **An item whose artifact is not in the tree** (β4 below) says so and
names the recipe that builds it; it still carries a command for the half that is measurable today.
**D18 of the measurements memo carries the falsifier for this convention** and the block's landing, revert
and re-landing. ⚠ **It no longer carries a yield figure** — the one it recorded came from a prototype on a
tree that was never committed, which D18 states — and
why it is not in the tree.

⚠ **A figure appears in this memo only where its subject is a tree that does not exist on HEAD, or where it
carries the command that produces it.** That applies to every section, not only this one.

Every measurement was taken in a throwaway clone, never in a worktree:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
```

⚠ **Never `source` the dispatcher** (`…A-rederive.sh`'s last line is `"${1:-all}" "$@"`, so sourcing re-runs
the suite recursively). Plants go on the **same line**. ⚠ **A plant is vacuous unless its tokens are in
`VOCAB` and it carries two of them** — admission to the census needs two vocabulary hits
(`-audit.sh:231`), so a plant without both produces no row and reads as a pass.

- **β1 ⊕ the two spellings.** With `_partset`/`_roster` defined as real functions and planted at a heredoc
  call site: unquoted `python3 - $(_partset) $(_roster) <<'PY2'` → **`?`** at **rc=1**
  (`RULED BY THE PLAN: 9 of 10 -- MISSING: ?`); quoted `python3 - "$(_partset)" "$(_roster)" <<'PY3'` →
  **`mention`** at **rc=0**, `9 of 9` — a genuine home filed under the class whose rule is *nothing to do*,
  silently green. ⚠ **There are three such host sites, not two**: `-audit.sh:53` (`HOMESPY`),
  `-audit.sh:598` (`SELFCHECKPY`) and `-inventory.sh:39` (`INVENTORYPY`) — the umbrella's `roster` row names
  all three (§3's `roster` row — cited by section, because ⚠ a draft cited it by line and this memo's own next edit moved the row two lines down), and `:538` is the one it actually exercised. §3a O2 plants at all three.

  ```bash
  grep -n "python3 - " docs/plans/2026-07-citation-hygiene-A-rederive*.sh
  ```

- **β2 ⊕ the complement**, measured alongside so that "it is broken" is a comparison and not a selection.
  Correct today: `_partset && _roster` and `_partset > /tmp/x; _roster > /tmp/y` → `callsite`;
  `echo '$(_partset)' '$(_roster)'` → `mention` (single quotes really are inert in bash).
  Wrong today: `if [ -n "$x" ]; then _partset; _roster; fi` → `?`, and β1's quoted crossing.

- **β3 ⊕ a line calling the derivation once is not a census row at all, and that bounds what β buys.**
  Appending two single-token call lines leaves `HOMES:` and `BY CLASS` unchanged; adding a second token to
  the same line moves them to `mention=2` and `HOMES: 71 (32 code, 39 prose)`. ⚠ **The plant must define the
  two names as real functions**, or sourcing the part runs them and the census produces nothing at all —
  measured, and it looks like a clean diff:

  ```bash
  d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"; cd "$d"
  P=docs/plans/2026-07-citation-hygiene-A-rederive-B.sh
  r() { bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes 2>&1 | grep -E 'BY CLASS|HOMES:'; }
  r > base
  printf '_partset() { echo p; }\n_roster() { echo r; }\n_p() {\n  _partset\n  _roster\n}\n' >> $P
  r | diff base -                       # empty: two single-token calls are not homes
  git checkout -- $P
  printf '_partset() { echo p; }\n_roster() { echo r; }\n_p() {\n  echo "$(_partset) $(_roster)"\n}\n' >> $P
  r | diff base -                       # mention 1 -> 2, HOMES 70 -> 71
  ```

  ⚠ **The vocabulary grows by the plant's *definitions*, not by its calls** — an
  earlier draft attributed the `V` move to the call lines. ⚠ **This is the census working, not a hole**: the
  admission rule is *this line enumerates two or more members of the set*, and a single call enumerates
  nothing. β's value is scoped to lines that clear that bar, and α's crossing does, passing two derivations
  on one line.

- **β4 ⊕ the scan changes no row today.** Implementing it on an unplanted clone leaves `BY CLASS`, `HOMES:`
  and `RULED BY THE PLAN` byte-identical to base. ⚠ **The artifact is not in the tree** — the scan is what β
  lands, so this half was measured in a throwaway clone that no longer exists and §3's β4 fence is the recipe
  that rebuilds it. What is runnable today is the base it is compared against, and the implementing commit
  takes it first:

  ```bash
  bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep -E 'BY CLASS|HOMES:|RULED'
  ```

- **β5 the memo gate's arity was a literal, and it is now derived — β no longer raises it.** The gate
  announced a count of the memos that existed when the line was written while ranging over the glob at
  `-audit.sh:345`, so it reported its own staleness as a LIMIT. β raised it in an earlier draft with the
  trigger *the slice that lands the fourth `2026-08-citation-hygiene-harness-*.md`*; the standalone prereq
  that cut §1 out of the disposition **is** that slice, and it took the fix. ⊕ Verified at HEAD by
  `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep -A1 'memo(s)'`: the gate now prints
  its arity from the glob **and** names the files it read on the next line — ⚠ an earlier draft attached the
  same command without `-A1`, showing only the first half of what the sentence claims. **Nothing here is β's**, and §5's raise is discharged
  rather than carried.

## β6 — why SCOPE and HOME were withdrawn one round after they were written

Cut out of the β memo's §3 while writing, because the withdrawal took that section across the authoring
band. §3 holds the decision and a one-line summary of each ground; this holds the measurements.


**SCOPE said**: the rule ranges over shell text, and a quoted here-document body is not shell text, so a
line outside the scope is *"not `callsite`, not `mention`, and not `?` — the rule does not reach it"*.
Four measurements retire it:

1. **It turns the RED backstop off for a third of the census.** ⊕ **25 of the 70 homes (16 of the 31 code
   homes)** sit inside a quoted here-document body. §2 **C1** says every censused line receives a class and
   an unplaceable one is `?`, which is RED; SCOPE names no class for the residue and §4 forbids adding a key
   to `CLASSES`. Silently green is what §3 says this slice exists to remove.
2. **It makes the many-homes defect structurally invisible.** ⊕ **Every** command-position decider in the
   harness is inside a quoted payload — `-audit.sh:93 143 145 159 187 223 231 617 653`, `-inventory.sh:137
   234`. Under SCOPE none of them is shell text, so a duplicate predicate can never be a `callsite` row.
3. **It would be the THIRD home of *which lines are payload*, and the harness has already been bitten by the
   second.** ⊕ `HEREDOC` (`-audit.sh:617`) and `_noheredoc`'s inline regex (`-inventory.sh:137`) are
   **byte-identical**, and `-inventory.sh:179-183` records the incident: *"`selfcheck`'s parser already drops
   payload bodies for exactly this reason; **this one did not**"* — a planted declaration entered the MOVE
   LIST at rc=0. This memo named neither: ⊕ `grep -c 'HEREDOC\|blocks()\|_noheredoc\|SELFCHECKPY'` over all
   six memos returns **0**. §3 records, one paragraph away, that it *"did not name"* `at_command` for three
   rounds; the same failure, for the shell-text predicate, one round later.
4. **Its cited authority does not carry it.** `bash(1)` *Here Documents* — *"the lines in the here-document
   are not expanded"* — is about **expansion**. An **unquoted** here-doc body is also not shell text and IS
   expanded, so *not expanded* is neither necessary nor sufficient for the property SCOPE needs.

⚠ **The one thing SCOPE was credited with, it did not do.** §3 said the two lines that discriminate the
quote clause become out of population under it. ⊕ Measured: `-audit.sh:94` and `-common.sh:462` each carry
**one** vocabulary token, so `len(hits) >= 2` (`-audit.sh:231`) excludes them and `classify` is never called
on either — with or without SCOPE. **The admission gate already did it**, which §3 states two paragraphs
earlier and never joined up. W1 is therefore still open and is NOT resolved by this section.

**HOME said**: the rule has one home reachable from every payload that needs it. It named no part, no
function and no transport, and round 8 measured why none was available. ⊕ The dispatcher's own placement
rule (`A-rederive.sh:41-42`) sends a helper with callers in two files to `-common.sh`; §3 cited neither.
⊕ The only concrete direction it gave — widening `at_command`'s lead — leaves **both** spellings in place,
i.e. convergent duplication rather than one home. ⊕ The transport it cited, `_runner` (`-Aii.sh:77`), is a
**slice** part, so reusing it makes A-ii a dependency of the three whole-harness checks — the inverse of the
seam both preambles state. ⊕ And a new emitter function enters `VOCAB` via `declare -F`, taking it 42 → 43,
which reds §3a's G1/G3 byte-identity. **HOME is restored as a raise, and its owner is the umbrella** (below).

## β7 — the classifier as it stands, and what this memo hands off

Cut out of the β memo's §2a while writing, at the band. The stub there keeps the section number.

⚠ **The `callsite` predicate does not do what its own comment says.** The comment states the rule —
*"a line is a call site because a vocabulary token stands in COMMAND POSITION"* (`-audit.sh:184-185`). The
code did something narrower — `git show 9c66be4d:docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh | sed -n '141,146p'`, since **β has now replaced those six lines** and they resolve to nothing at HEAD: an outer `re.search` establishes only that *some* command position
exists on the line, and the body then tests **the line's first word** (`WORD.search(ln.lstrip())`), plus a
special case for `_measure` after a separator.

Three consequences, each ⊕ against the same clone recipe — take the base first with
`bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes`, then neutralise the named lines and re-run:

1. **The `then ` / `do ` / `else ` alternatives in the outer regex are dead** — the subject is the first word,
   so a line beginning `if` / `for` / `while` / `case` can never be `callsite`.
2. **The `_measure` special case decided no row, and β has retired it.** Neutralising it on its own line (`git show 9c66be4d:…-audit.sh | sed -n '145,146p'`, gone at HEAD)
   leaves `rederive homes` byte-identical. ⚠ **Dead on the population, live as a predicate**: it is the only
   thing that would make `local n_fx; _measure …` (`-Aii.sh:268`) a `callsite`, which is why §3a O3 plants
   *that* shape and not one whose first word is already a block.
3. **`mention`'s stated premise is false for a command substitution.** `hits_outside_quotes`
   (`-audit.sh:158-160`) deletes `'…'` and `"…"` spans by flat, non-nesting alternation with no shell
   awareness, so `"$(f)"` is deleted **whole**. The predicate decides on quote *characters* and cannot
   distinguish *named in a string* from *called through a substitution*.

**Recorded for ε — the `prose` predicate.** Re-implemented from the literal text of the umbrella's `prose`
row rather than from its intent. ⚠ **That run was delegated and is not re-measured here.** Its structural
claims reproduce exactly: the five straddling rows by identity (`-Aii.sh:7`, `-B.sh:7`, `-audit.sh:92`,
`-common.sh:10`, `A-rederive.sh:47`); ⊕ **the control population is whatever
`grep -h '^\s*#' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l` returns today, less the 39
`prose` census rows** — ⚠ an earlier draft wrote `= 911` beside that very command, and it has returned 925
and 970 since, because this program keeps adding comment lines to its own parts. The figure had three homes
and none reproduced. Its quantitative claims do not: across six readings the placement/rationale split came out
14/20 and 21/13, never the umbrella's 19/15, and no treatment of straddlers reached its 849 control figure.
**The umbrella now records that at both sites that carried the figure**; ε measures its own.

⚠ **Two of the fourteen gaps found are contradictions rather than ambiguities, and they are ε's**: the place
vocabulary cannot be *"derived from `PARTS`"* and contain the dispatcher (`PARTFILES` filters it out at
`-audit.sh:59`; `PARTS` derives from it at `:65`), and the test's group vocabulary has no in-scope home
(`GROUPS` is `-inventory.sh:168`, inside `INVENTORYPY`, while the test lives in `HOMESPY`). ⚠ **The bar these
are measured against is this memo's own, not a quotation of the umbrella** — an earlier draft attributed the
sentence to the umbrella, where it does not appear.

**Recorded — which family of content test is not constructible, and which is.** Every candidate predicate
that keys a rule cell to **its own class's evidence** was run over the umbrella's nine rows. Keying the
citation to the class the row rules fails on **four** (`authorlocal`, `reads`, `mention`, `callsite`),
because `CLASSES` (`-audit.sh:108-109`) is the only identifier→class map and covers four classes; keying it
to a census *row* of that class fails on **six**; and admitting "the class name itself" makes it satisfiable
by typing the class name. Two of the four are α's and δ's rows. ⚠ **This memo then generalised that to "no
predicate over a rule cell's text is constructible" and the umbrella carried the generalisation into a
withdrawal. The complement was never measured, and it is constructible** — see the umbrella's §3, which
carries the predicate and both of its controls. What this memo measured stands; what it concluded from it
did not, and the correction is the umbrella's because the gate is.
