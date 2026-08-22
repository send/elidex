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
`2026-08-citation-hygiene-harness-*.md` against **the measurements memo's** §1 (`-audit.sh:445-453`), so a
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
(`-audit.sh:230`), so a plant without both produces no row and reads as a pass.

- **β1 ⊕ the two spellings.** With `_partset`/`_roster` defined as real functions and planted at a heredoc
  call site: unquoted `python3 - $(_partset) $(_roster) <<'PY2'` → **`?`** at **rc=1**
  (`RULED BY THE PLAN: 9 of 10 -- MISSING: ?`); quoted `python3 - "$(_partset)" "$(_roster)" <<'PY3'` →
  **`mention`** at **rc=0**, `9 of 9` — a genuine home filed under the class whose rule is *nothing to do*,
  silently green. ⚠ **There are three such host sites, not two**: `-audit.sh:53` (`HOMESPY`),
  `-audit.sh:597` (`SELFCHECKPY`) and `-inventory.sh:39` (`INVENTORYPY`) — the umbrella's `roster` row names
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
  `-audit.sh:344`, so it reported its own staleness as a LIMIT. β raised it in an earlier draft with the
  trigger *the slice that lands the fourth `2026-08-citation-hygiene-harness-*.md`*; the standalone prereq
  that cut §1 out of the disposition **is** that slice, and it took the fix. ⊕ Verified at HEAD by
  `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep -A1 'memo(s)'`: the gate now prints
  its arity from the glob **and** names the files it read on the next line — ⚠ an earlier draft attached the
  same command without `-A1`, showing only the first half of what the sentence claims. **Nothing here is β's**, and §5's raise is discharged
  rather than carried.
