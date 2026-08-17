# Citation-hygiene harness PR-1a-i-β — one command-position scan, and it says what it does

**Subject**: slice **β** of PR-1a-i, the first of the five the disposition memo's §2 partitions it into.
**Input**: `2026-08-citation-hygiene-harness-disposition.md` — the umbrella. Its §2, §3, §3a and §9 are the
authority; this memo does not re-derive them, and where it contradicts them it says so and says which moves.

**β is one predicate.** `classify()` decides whether a line is a call site by testing **the line's first
word**, though its own comment states the rule as *"a vocabulary token stands in COMMAND POSITION"*. β
replaces the approximation with the rule. Nothing else.

**What β no longer is, and why.** Two deliverables were taken out of β by review, each on a measurement:

- **The `prose` subject test** needs a place vocabulary and a group vocabulary, and both reach `-audit.sh`'s
  payload only through the crossing α builds. The umbrella re-sliced it as **ε**, after α and γ.
- **The coverage gate's content test** is not constructible over a rule cell's text. Every candidate was
  implemented and run; two of the rows each one reds belong to α and δ, so no slice can land it green inside
  its own authorisation. The umbrella withdrew the ask and placed the one surviving half — the reverse
  direction, `ruled - set(byclass)` — with ε, where it first has a job. **§2a records the measurements**
  because this is where they were taken; nothing in §3 implements them.

**Why β is still first.** The umbrella's ordering measurement — a derivation called from an argument position
falls through to `?` and the census reds — is a fact about this predicate. α cannot land into a census that
reds on α's own output.

⚠ **β changes no row in today's census.** Measured: implementing the scan on an unplanted tree leaves
`BY CLASS`, `HOMES:` and `RULED BY THE PLAN` byte-identical to base. β's whole value is prospective, and
§3a is built around that fact rather than in spite of it.

## §0.5 / §3. Spec coverage map

β touches no spec text. It touches the **classifier that decides whether the lines carrying the program's
spec pairs are homes**: at HEAD none of them is a census row, and §3a G2 asserts that this still holds.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | classify the lines carrying it | `fixtures` payload — **materialise it, do not read it off this cell** | `classify` — must stay **not a home** | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | classify the line carrying it | `fixtures` payload | `classify` — must stay **not a home** | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | classify the line carrying it | `fixtures` payload | `classify` — must stay **not a home** | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | classify the lines carrying it | `fixtures` payload | `classify` — must stay **not a home** | ✓ | no |

⚠ **The Branch column names fixtures, and a fixture is not the same string as the spec's own label** — the
umbrella's warning, kept: the Spec column carries each spec's authoritative label while a fixture
deliberately carries a different one, because aliasing is what the fixture set exercises. ⚠ **Row 1's cell
is deliberately not filled with a count.** The umbrella refuses to fill it (*"read the fixtures, not this
cell"*), and a first draft of this memo filled it with a figure that measured false. The multiplicity is a
command:

```bash
bash -c '. docs/plans/2026-07-citation-hygiene-A-rederive-integrity.sh
         . docs/plans/2026-07-citation-hygiene-A-rederive-common.sh
         fixtures /tmp/fx'
grep -lF '§4.10.21 Constraints' /tmp/fx/*.md
grep -n '§4\.10\.21\|§2\.2\.5\|§4\.2 The MediaQueryList\|webref heading' \
     docs/plans/2026-07-citation-hygiene-A-rederive-*.sh
```

**Breadth**: K=3 specs, M=4 entries — the umbrella's complete set, and β adds none (verified 2026-08-16
against `2026-08-citation-hygiene-harness-disposition.md:53-58`).

⚠ **The Touch column is falsifiable, and its falsifier reads the whole census run** — not `homes | grep`,
which discards the gate's `!!` lines and the `LIMIT:` block, the form the umbrella names as *M3's charter
inverted*:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > /tmp/h; echo "rc=$?"; cat /tmp/h
```

Verified 2026-08-16: the `-common.sh` lines the census reports as homes are 4, 9, 10, 212, 233, 507, 509,
516, 518, 520 and 592, and every line carrying a spec pair is outside that set — because those lines carry
**no** vocabulary token, which is stronger than "fewer than two" and is what makes β safe for them: β widens
where a vocabulary token may stand, not what counts as one. ⚠ **G2's scope is `-common.sh` only because all
ten spec-pair lines are there today**; the assertion re-derives that rather than presuming it.

## §1 Measurements

Numbered **β1…**, not `D<N>`: the harness's memo gate resolves every `D<N>` in any
`2026-08-citation-hygiene-harness-*.md` against the umbrella's §1, so a fresh `D`-number here would dangle —
measured, by writing one and watching this memo's own first gate run red.

⚠ **Provenance convention.** ⊕ marks an item re-measured by this memo's author, **and every ⊕ item carries
its command here or in §3**. A previous draft asserted that and was false on twelve of thirteen items; the
attestation is only as good as the commands, so the commands are the convention and ⊕ is shorthand for
"a command is attached".

⚠ **A figure appears in this memo only where its subject is a tree that does not exist on HEAD, or where it
carries the command that produces it.** That applies to every section, not only this one.

Every measurement was taken in a throwaway clone, never in a worktree:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
```

⚠ **Never `source` the dispatcher** (`…A-rederive.sh`'s last line is `"${1:-all}" "$@"`, so sourcing re-runs
the suite recursively). Plants go on the **same line**. ⚠ **A plant is vacuous unless its tokens are in
`VOCAB` and it carries two of them** — admission to the census needs two vocabulary hits
(`-audit.sh:188`), so a plant without both produces no row and reads as a pass.

- **β1 ⊕ the two spellings.** With `_partset`/`_roster` defined as real functions and planted at a heredoc
  call site: unquoted `python3 - $(_partset) $(_roster) <<'PY2'` → **`?`** at **rc=1**
  (`RULED BY THE PLAN: 9 of 10 -- MISSING: ?`); quoted `python3 - "$(_partset)" "$(_roster)" <<'PY3'` →
  **`mention`** at **rc=0**, `9 of 9` — a genuine home filed under the class whose rule is *nothing to do*,
  silently green. ⚠ **There are three such host sites, not two**: `-audit.sh:53` (`HOMESPY`),
  `-audit.sh:538` (`SELFCHECKPY`) and `-inventory.sh:39` (`INVENTORYPY`) — the umbrella's `roster` row names
  all three (`:488`), and `:538` is the one it actually exercised. §3a O2 plants at all three.

  ```bash
  grep -n "python3 - " docs/plans/2026-07-citation-hygiene-A-rederive*.sh
  ```

- **β2 ⊕ the complement**, measured alongside so that "it is broken" is a comparison and not a selection.
  Correct today: `_partset && _roster` and `_partset > /tmp/x; _roster > /tmp/y` → `callsite`;
  `echo '$(_partset)' '$(_roster)'` → `mention` (single quotes really are inert in bash).
  Wrong today: `if [ -n "$x" ]; then _partset; _roster; fi` → `?`, and β1's quoted crossing.

- **β3 ⊕ a line calling the derivation once is not a census row at all, and that bounds what β buys.**
  Appending two single-token call lines leaves `HOMES:` and `BY CLASS` unchanged; adding a second token to
  the same line moves both. ⚠ **The vocabulary grows by the plant's *definitions*, not by its calls** — an
  earlier draft attributed the `V` move to the call lines. ⚠ **This is the census working, not a hole**: the
  admission rule is *this line enumerates two or more members of the set*, and a single call enumerates
  nothing. β's value is scoped to lines that clear that bar, and α's crossing does, passing two derivations
  on one line.

- **β4 ⊕ the scan changes no row today.** Implementing it on an unplanted clone leaves `BY CLASS`, `HOMES:`
  and `RULED BY THE PLAN` byte-identical to base. Command in §3.

- **β5 the memo gate's arity is a literal** (`-audit.sh:385-386`; the glob it counts is `:302`). ⊕ Verified
  at HEAD: the gate prints `LIMIT: 3 of the 2 memos are present; the checks ranged over those` at rc=0, with
  the new file scanned. This is a **present** state, not a consequence of β's implementing commit.
  **Disposition in §5.**

## §2 Coupled invariants

Required by `/elidex-plan-review` Pre-condition #3.

- **C1 totality** — every censused line receives a class; an unplaceable line is `?`, which is **RED**.
- **C2 every class is a subject test** — each terminal class is reached by a **positive** predicate.
  ⚠ The census's comment at `-audit.sh:134-140` records one measured instance of a fallthrough swallowing a
  home; `-audit.sh:147-150` asserts the same principle for `mention` but records **no** instance, so this is
  grounded once — and β1's quoted crossing is the second.
- **C3 self-reference** — the classifier is inside the census's glob, so a predicate spelled as a literal
  becomes a home of the fact it censuses.
- **C5 population vs classification** — β must move lines **between** classes, not into or out of the census.

| pair | intersection | disposition |
|---|---|---|
| C1 × C2 | Making a predicate stricter turns green lines red, and that is the intended direction — so the lines it stops claiming must be claimed by another. β widens `callsite`, so it **takes** from `mention` and `?`; it must be shown to take only the right ones, in both directions. | §3, §3a O1/O4 |
| C2 × C3 | The scan's position list must be stated as a rule, not spelled as a set of characters, or the classifier acquires a second home of "what a command position is" — and the harness's existing quote helper already decides on characters, which is the defect one class over. | §3 |
| C3 × C5 | β's own new source is inside the glob. ⚠ **Measured, β adds no census row at all** (β4), so C5 here is the strong form: *no line enters, none leaves, and none changes class on the unplanted tree*. | §3a G1/G3 |
| C1 × C5 | β is behaviour-neutral today, so **every guard passes under a no-op**. ⚠ That is measured, not assumed, and it means the criterion's entire discriminating power sits in the planted obligations — the guards bound the blast radius, they do not evidence the work. | §3a |

⚠ **C4 (class set ↔ rule rows) is no longer β's.** It entered §2 when β held the `prose` split and the
content test; both are elsewhere now, and β adds and retires no class. The intersection is ε's.

⚠ **This is the base case.** Every intersection is internal to "how a line is classified", and none reaches
a question about where a block ships.

## §2a Measured — the classifier as it stands, and the two withdrawals

⚠ **The `callsite` predicate does not do what its own comment says.** The comment states the rule —
*"a line is a call site because a vocabulary token stands in COMMAND POSITION"* (`-audit.sh:138-140`). The
code (`:141-146`) does something narrower: an outer `re.search` establishes only that *some* command position
exists on the line, and the body then tests **the line's first word** (`WORD.search(ln.lstrip())`), plus a
special case for `_measure` after a separator.

Three consequences, all ⊕ (commands in §3):

1. **The `then ` / `do ` / `else ` alternatives in the outer regex are dead** — the subject is the first word,
   so a line beginning `if` / `for` / `while` / `case` can never be `callsite`.
2. **The `_measure` special case decides no row today.** Neutralising `-audit.sh:145-146` on its own line
   leaves `rederive homes` byte-identical. ⚠ **Dead on the population, live as a predicate**: it is the only
   thing that would make `local n_fx; _measure …` (`-Aii.sh:268`) a `callsite`, which is why §3a O3 plants
   *that* shape and not one whose first word is already a block.
3. **`mention`'s stated premise is false for a command substitution.** `hits_outside_quotes`
   (`-audit.sh:112-115`) deletes `'…'` and `"…"` spans by flat, non-nesting alternation with no shell
   awareness, so `"$(f)"` is deleted **whole**. The predicate decides on quote *characters* and cannot
   distinguish *named in a string* from *called through a substitution*.

**Recorded for ε — the `prose` predicate.** Re-implemented from the literal text of the umbrella's `prose`
row rather than from its intent. ⚠ **That run was delegated and is not re-measured here.** Its structural
claims reproduce exactly: the five straddling rows by identity (`-Aii.sh:7`, `-B.sh:7`, `-audit.sh:92`,
`-common.sh:10`, `A-rederive.sh:47`); ⊕ the control population of 872
(`grep -h '^\s*#' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l` = 911, less the 39 `prose`
census rows). Its quantitative claims do not: across six readings the placement/rationale split came out
14/20 and 21/13, never the umbrella's 19/15, and no treatment of straddlers reached its 849 control figure.
**The umbrella now records that at both sites that carried the figure**; ε measures its own.

⚠ **Two of the fourteen gaps found are contradictions rather than ambiguities, and they are ε's**: the place
vocabulary cannot be *"derived from `PARTS`"* and contain the dispatcher (`PARTFILES` filters it out at
`-audit.sh:59`; `PARTS` derives from it at `:65`), and the test's group vocabulary has no in-scope home
(`GROUPS` is `-inventory.sh:168`, inside `INVENTORYPY`, while the test lives in `HOMESPY`). ⚠ **The bar these
are measured against is this memo's own, not a quotation of the umbrella** — an earlier draft attributed the
sentence to the umbrella, where it does not appear.

**Recorded — why the content test is withdrawn.** Every candidate predicate over a rule cell's text was run
over the umbrella's nine rows. Keying the citation to the class the row rules fails on **four**
(`authorlocal`, `reads`, `mention`, `callsite`), because `CLASSES` (`-audit.sh:108-109`) is the only
identifier→class map and covers four classes; keying it to a census *row* of that class fails on **six**;
and admitting "the class name itself" makes it satisfiable by typing the class name. Two of the four are α's
and δ's rows. The umbrella withdrew the ask and placed the surviving reverse direction with ε.

## §3 The work — one command-position scan

**A command begins at line start, after `;` `&&` `||` `|` `(` `{`, after `then` / `do` / `else`, and after
`$(`. The token at each such position is taken, and the line is `callsite` if any of them is in the
vocabulary.**

⚠ **The list is a rule about shell syntax, and three of its clauses are decisions measurement forced, not
characters copied from the old regex:**

1. **`$(` is a command position; a backtick is not.** A regex cannot separate a shell backtick substitution
   from a markdown backtick, and this harness writes block names in backticks inside strings. ⊕ Measured: a
   code line `plantdoc="the \`citations\` block and the \`budget\` block"` is `mention` at HEAD — correct,
   it talks about blocks — and **`callsite` at rc=0, `9 of 9`** if a backtick is admitted: silently green,
   the failure class β exists to remove, one class over. ⊕ The harness contains **zero** legacy backtick
   substitutions, so admitting one buys nothing:

   ```bash
   grep -nE '=[[:space:]]*`|\|\|[[:space:]]*`|;[[:space:]]*`|^[[:space:]]*`|\([[:space:]]*`' \
        docs/plans/2026-07-citation-hygiene-A-rederive*.sh | grep -vE ':[0-9]+: *#'
   ```

2. **Single-quoted spans are removed before the scan; double-quoted spans are not.** ⚠ **This is the clause
   an earlier draft omitted, and omitting it makes the memo self-contradictory.** β2 lists
   `echo '$(_partset)' '$(_roster)'` → `mention` as *correct today*, and a scan with no quote handling
   reclassifies it. But removing **double**-quoted spans as well would kill β1's quoted crossing — β's
   headline case. Of the three readings available, exactly one satisfies every observable this memo states:
   strip `'…'`, then scan. The rule is stated because it is not derivable from the position list.
3. **`{` opens a command position; `${` does not.** ⊕ Measured: with a bare `{` in the list,
   `echo "${_partset} ${_roster}"` becomes `callsite` (`mention` at HEAD) — parameter expansion is not a
   command. The clause is *`{` not preceded by `$`*.

⚠ **`mention`'s own predicate does not move.** Because `callsite` is tested first, closing the hole in
`callsite` closes the `mention` symptom without widening `mention`, which would strip the protection it
gives to genuine prose-inside-quotes. **The umbrella's §2 phrasing — *"β owns `mention`'s
command-substitution hole as well"* — is right about the ownership and names the wrong site**; §3 β-c
corrects the row.

⚠ **The `_measure` special case retires into the scan.** §2a measures it is dead on today's population; the
scan covers its shape, because `local n_fx; _measure …` reaches command position after `;`. **Dead and
subsumed are two claims**, and §3a O3 asserts the second by planting the shape that discriminates.

```bash
# β4's command: implement the scan on a clone, diff the census against base
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > /tmp/base
# …patch classify()…  then:
bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | diff /tmp/base -
```

### §3 β-c — the umbrella rows β falsifies

β's mechanism makes two statements in the umbrella untrue. They land in β's commit because they are
bookkeeping β's own change makes true — the discipline the umbrella's `roster` row states for its own rename.

| the umbrella says | why β falsifies it | β's edit |
|---|---|---|
| §3's `callsite` row: *"nothing to do."* | β replaces the predicate the row rules on | the row states the rule — a call site is a vocabulary token in command position — and that `_measure`'s special case retires into it. ⚠ **The row states the rule, not the position list**: the list is the mechanism's, stated once in the code, or the umbrella becomes a second home for it |
| §3's `mention` row: *"nothing to do, and it is a subject test…"* | the row describes the behaviour β corrects — β1's quoted crossing reaching `mention` is the silent-green failure | the row keeps the subject test and gains its precondition: command substitutions are claimed by `callsite` first, so what reaches `mention` really is text |

⚠ **Nothing else in the umbrella is β's to edit.** The obligation count, the withdrawn content test, the
`19/15` figure, D16's owner and §9's authorisation path were all corrected by the umbrella in its own
commits — each was false at HEAD before β, discovered rather than falsified by it.

⚠ **§9 now authorises through §3, §2 and §3a**, so β's authority no longer resolves through a section β
amends. That was a **precondition**, not a raise, and it is discharged before β implements rather than
recorded as an owner without a trigger.

## §3a β's exit criterion

The umbrella's three properties are adopted: **no expected value is written in it**; **a collapse is two
facts, and a literal-gone needle tests only a spelling**; **vacuity guards compare against base**.

⚠ **Two shapes are ruled out.** `RULED BY THE PLAN: N of N` ranges over the observed class set. `homes`'s own
raises fire on *adding*, and β replaces.

⚠ **And the guards cannot evidence the work.** β4 measures that β changes no row on the unplanted tree, so
**every guard below passes under a no-op**. That is stated rather than discovered later: the guards bound the
blast radius, and all the discriminating power is in O1–O4.

| # | obligation | asserted by planting, not by reading |
|---|---|---|
| O1 | a derivation call is a call site in every command position | plant **both** spellings at **all three** heredoc host sites (`-audit.sh:53`, `:538`, `-inventory.sh:39`) ⇒ all six rows `callsite`. Each plant defines `_partset`/`_roster` and carries two tokens |
| O2 | the position list's three decided clauses hold | `echo '$(_partset)' '$(_roster)'` ⇒ `mention`; `echo "${_partset} ${_roster}"` ⇒ `mention`; a code line with two block names in **markdown backticks** ⇒ `mention`. Each is a case a plausible implementation gets wrong |
| O3 | the scan **subsumes** `_measure`, and the special case is gone | plant `local n; _measure <two blocks>` ⇒ `callsite`. ⚠ **Not `: ; _measure …`** — measured, that shape passes under a bare deletion of the special case with no scan written, because `WORD.search` skips `:` and `;` and finds `_measure` as the first word. The `local` shape is the one that discriminates |
| O4 | `mention` still means *text about a block* | `-B.sh:87`, the one live `mention` row, still `mention` — and the whole `BY CLASS` line unchanged (G3), which is the general form of the same assertion |
| G1 | population | every home in the base census is present after, and none arrives |
| G2 | the spec-pair lines stay non-homes | re-derive the `-common.sh` home list from a full census run and require no spec-pair line in it; re-derive that all spec-pair lines are in `-common.sh` rather than presuming it |
| G3 | nothing moved class | `BY CLASS` byte-identical to base on the unplanted tree |
| G4 | no block left the table | ⚠ **the named block set of `inventory`'s table, diffed against base — not `defined=`.** The umbrella measured the count form false: a part can leave the table with `defined=` 35 → 35 because an addition elsewhere masks the removal. `defined=` may be reported, never relied on |

⚠ **What β cannot test**: whether the position list is exhaustive for shells this harness does not write, and
whether ε's later work re-opens a case β closed — β's guards range over today's population, which is the
population β leaves unchanged.

## §4 What β does not do

- **β changes no answer about where a block ships.** No tier, no `route`, no `PART_SLICE`, no declaration.
- **β does not split `prose` or touch any vocabulary** (ε), **does not build or use the part-set derivation**
  (α), and **adds no key to `CLASSES`**.
- **β lands no coverage-gate change.** Withdrawn by the umbrella; the surviving half is ε's.
- ⚠ **β's effect on `-audit.sh`'s length is an obligation, not an assertion.** An earlier draft asserted the
  authoring band was not reached, unmeasured, in a phrasing the harness's own `BAND` needle cannot read.
  `-audit.sh` is **638** lines at HEAD (`wc -l docs/plans/2026-07-citation-hygiene-A-rederive*.sh`), and the
  umbrella's precondition is that a permitted mechanism commit must not be the commit that crosses the size
  trigger. **The implementing commit measures its own tree and cuts the seam while writing if it enters the
  700–800 band**; this memo predicts no number.
- ⚠ **β's own citations of `-audit.sh:N` move when β edits `classify`.** The memo gate's path check is
  range-only, so a stale anchor stays green. The implementing commit re-derives every `-audit.sh:N` anchor in
  this memo and in the umbrella's two rows, in the same commit.

## §5 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: the command-position scan, the two umbrella row edits
§3 β-c enumerates, and the anchor re-derivation §4 requires — and nothing else.

**Does not authorise**: any change to a tier, declaration or routing answer (γ); the part-set or roster
derivations (α); the `prose` subject test or any vocabulary (ε); the coverage gate (ε); the `reads` rule (δ);
adding a key to `CLASSES`; or predicting a figure §4 assigns to the implementing run.

**Raised for the umbrella, with a trigger** — an owner without a trigger is a drop, which this memo has
already done once:

- ⚠ **The memo gate's arity literal** (§1 β5). It is not a classification, so β does not take it. ⊕ Measured,
  the two sites are **not census rows**, so α's `memoset` rule — whose work list is the census — does not
  reach them either. **Trigger: the slice that lands the fourth `2026-08-citation-hygiene-harness-*.md`**,
  which on the umbrella's own schedule is α.
- ⚠ **`CLASSES` takes a literal's class from its name** (`-audit.sh:108-109`), so `PARTS="$(_roster)"`
  classifies `partset`. ⊕ Measured: real, with **no witness in the population at HEAD**. **Trigger: α**, the
  slice that turns those literals into derivations — a content rule for them now would predict α's
  implementation.

**This memo is the base case** under CLAUDE.md's edge-dense rule. Passing its `/elidex-plan-review`
discharges terminality; this memo does not claim it in advance.
