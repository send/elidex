# Citation-hygiene harness PR-1a-i-β — the classifier: one predicate per class, and it says what it does

**Subject**: slice **β** of PR-1a-i, the first of the five the disposition memo's §2 partitions it into.
**Input**: `2026-08-citation-hygiene-harness-disposition.md` — the umbrella. Its §2 (the partition), §3 (the
rule per class) and §3a (the exit criterion) are the authority; this memo does not re-derive them, and where
it contradicts them it says so and says which one moves.

**What β is.** The census `rederive homes` assigns every line it finds a **class**, and the class decides
which of §3's rules applies to it. β changes **two predicates** — the one that decides whether a line is a
call site, and the one the coverage gate applies to §3's own rule rows — and nothing about where a block
ships. Its whole surface is `classify()`'s `callsite`/`mention` branches and the gate at `-audit.sh:265-284`.

**What β is not, and why this memo is shorter than its first draft.** β held the `prose` subject test until
β's own `/elidex-plan-review` measured that it cannot: that test needs a place vocabulary and a group
vocabulary, and **both reach `-audit.sh`'s payload only through the argv crossing α builds**. The umbrella
re-sliced it out as **ε**, after α and γ. The reasoning and the falsifications are recorded in §2b below,
because they are ε's input and this is where they were measured — but **nothing in §3 implements them.**

**Why β is still first.** The umbrella's ordering measurement — a derivation called from an argument position
falls through to `?` and the census reds — is a fact about the **command-position predicate**, which is β's.
α cannot land into a census that reds on α's own output.

## §0.5 / §3. Spec coverage map

β touches no spec text. It touches the **classifier that decides whether the lines carrying the program's
spec pairs are homes**, and those lines are what this table is for: at HEAD none of them is a census row, and
β's obligation is that this is still true afterwards — asserted by §3a guard G2, not left to inference.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | classify the lines that carry it | `fixtures` payload (five spellings) and `citations` lookup | `classify` — must stay **not a home** | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | classify the lines that carry it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | classify the lines that carry it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | classify the lines that carry it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |

⚠ **The Branch column names fixtures, and a fixture is not the same string as the spec's own label** — the
umbrella's own warning, which β must not drop: the Spec column carries each spec's authoritative label while
a fixture deliberately carries a different one, because aliasing is what the fixture set exercises. One spec
pair is carried by **more than one line in more than one spelling**, so the Step column says *lines*, and
which lines is a command rather than a cell.

**Breadth**: K=3 specs, M=4 entries — the umbrella's complete set, and β adds none (verified 2026-08-16
against `2026-08-citation-hygiene-harness-disposition.md:53-58`).

```bash
grep -n '§4\.10\.21\|§2\.2\.5\|§4\.2 The MediaQueryList\|webref heading' \
     docs/plans/2026-07-citation-hygiene-A-rederive-*.sh
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep 'common\.sh'
```

⚠ **The Touch column is falsifiable and was run** (verified 2026-08-16): the `-common.sh` lines the census
reports as homes are 4, 9, 10, 212, 233, 507, 509, 516, 518, 520 and 592, and every line carrying a spec
pair is outside that set. They are outside it because they carry **no** vocabulary token — a stronger fact
than "fewer than two", and the one that makes β's change safe for them: β widens where a vocabulary token
may stand, not what counts as one.

## §1 Measurements

Numbered **β1…**, not `D<N>`: `D<N>` is the umbrella's namespace and the harness's memo gate resolves every
`D<N>` in any `2026-08-citation-hygiene-harness-*.md` against the umbrella's §1 bullet list, so a fresh
`D`-number opened here would dangle by construction — **measured, by writing one and watching this memo's own
first gate run red.** Citations of the umbrella's existing D-numbers are its.

⚠ **A figure below is written only where its subject is a tree that does not exist on HEAD** — a planted
spelling, a patched classifier. Where a HEAD quantity is named it carries the command that produces it, so
it is a pointer to a run rather than an expected value, and β's own commit is free to move it.

Every measurement was taken in a throwaway clone, never in a worktree:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
```

⚠ **Never `source` the dispatcher.** `docs/plans/2026-07-citation-hygiene-A-rederive.sh`'s last line is
`"${1:-all}" "$@"`, so sourcing it re-runs the whole suite recursively. Plants go on the **same line**.
⚠ **A plant is vacuous unless the tokens it names are in `VOCAB`**: admission to the census needs two
vocabulary hits (`-audit.sh:188`), so every plant below defines `_partset`/`_roster` as real functions first,
and every plant carries **two**. A plant without the definitions produces no row and reads as a pass.

- **β1 the two spellings**, planted at two host lines each (`-audit.sh:53` and `-inventory.sh:39`, the two
  real heredoc call sites, and a non-executing function appended to `-Aiii.sh`): unquoted
  `python3 - $(_partset) $(_roster) <<'PY2'` classifies **`?`** at **rc=1** (`RULED BY THE PLAN: 9 of 10
  -- MISSING: ?`); quoted `python3 - "$(_partset)" "$(_roster)" <<'PY3'` classifies **`mention`** at
  **rc=0**, `9 of 9` — a genuine home filed under the one class whose rule is *nothing to do*, silently
  green. The quoted spelling is the likelier one.
- **β2 the complement**, measured alongside so that "it is broken" is a comparison and not a selection.
  Correct today: `_partset && _roster` and `_partset > /tmp/x; _roster > /tmp/y` → `callsite`;
  `echo '$(_partset)' '$(_roster)'` → `mention` (single quotes really are inert in bash).
  Wrong today: `if [ -n "$x" ]; then _partset; _roster; fi` → `?`, and the quoted crossing above.
- **β3 the coverage gate is blind to a rule's content in both directions.** `mention`'s rule cell **emptied**
  with its key kept → `9 of 9`, rc=0; a rule row for `ghostclass`, a class the census never emits → `9 of 9`,
  rc=0. The first is the umbrella's claim and is **weaker than the umbrella states** — not only `TBD`. The
  second is the mirror it does not mention, and it is the one that bites this program: the gate is a
  one-directional subset test (`set(byclass) - ruled`, `-audit.sh:281`), so a rule for a retired class is
  invisible, and ε retires `prose`.
- **β4 a line calling the derivation once is not a census row at all, and that bounds what β buys.**
  Measured with a discriminating control: appending two single-`_partset` call lines takes `V` 41 → 43 — the
  token *is* in the vocabulary, so the test is not vacuous — while `HOMES:` stays at its base value and
  `BY CLASS` is unchanged; adding a second token to the same line moves both. ⚠ **This is the census
  working, not a hole.** Admission means *this line enumerates two or more members of the set*, and a single
  call enumerates nothing. β's value is scoped to lines that clear that bar, and the crossing α builds does,
  because it passes **two** derivations on one line.
- **β5 the memo gate's arity is a literal.** Landing a third `2026-08-citation-hygiene-harness-*.md` makes
  the gate print `LIMIT: 3 of the 2 memos are present; the checks ranged over those` at rc=0, with the new
  file scanned. The arity is at `-audit.sh:385-386`; the glob it counts is at `:302`. **Disposition in §5.**
  ⚠ This is already live at HEAD — this memo is the third file — so it is a present state, not a
  consequence of β's implementation commit.

## §2 Coupled invariants

Required by `/elidex-plan-review` Pre-condition #3.

- **C1 totality** — every censused line receives a class, and an unplaceable line is `?`, which is **RED**.
- **C2 every class is a subject test** — each terminal class is reached by a **positive** predicate.
  A fallthrough silently absorbs whatever the predicates above it missed; the census records one measured
  instance of that happening (`-audit.sh:134-140`, the `blocks="citations budget lanes"` home swallowed by
  `callsite` as a fallthrough). ⚠ `-audit.sh:147-150` asserts the same principle for `mention` but records
  **no** instance — so this is grounded once, and β's own β1 supplies the second.
- **C3 self-reference** — the classifier is inside the census's glob, so a predicate spelled as a literal
  becomes a home of the fact it censuses.
- **C4 class set ↔ rule rows** — the coverage gate compares the census's class names against the umbrella's
  §3 rule-row keys (`-audit.sh:265-284`).
- **C5 population vs classification** — β must move lines **between** classes, not into or out of the census.

| pair | intersection | disposition |
|---|---|---|
| C1 × C2 | Making a predicate stricter turns a green line red, and that is the intended direction — so the lines a predicate stops claiming must be claimed by another, or the census reds. β widens `callsite` and therefore **takes** lines from `mention` and `?`; it must be shown to take only the right ones. | §3 β-b, §3a O2/G3 |
| C2 × C3 | The gate's content predicate is itself a rule about rules, so it must not be satisfiable by boilerplate — a predicate with no object constraint is a shape rule with no subject test, which is the defect C2 names. | §3 β-a |
| C1 × C4 | A class split adds names the gate has no rows for. ⚠ **β's first draft claimed this forces mechanism and memo into one commit. Measured, that is false**: keeping the `prose` row while adding three successor rows leaves the gate at `9 of 9`, rc=0, because the gate is a one-directional subset test. Row *additions* and *rewrites* are green in either order; only a row **removal** is ordered, and only to "not before the mechanism". | §3 β-c |
| C4 × C5 | The gate ranges over the **observed** class set, so retiring a class removes it from numerator and denominator alike. `N of N` can never be an exit criterion here. | §3a |
| C3 × C5 | β's own new source is inside the glob, so β moves the population by its own text. C5 is not "the population is unchanged"; it is "no line leaves, and the lines that arrive are ones β wrote". | §3a G1 |
| C2 × C4 | `mention` and `callsite` are ruled *nothing to do* in the umbrella's §3 while its §2 and §3a make both β's obligations. The gate cannot see the disagreement: it checks that a key has a row, never what the row says. β closes it in both directions — the predicate and the row. | §3 β-b, β-c |

⚠ **This is the base case, not a new umbrella.** Every intersection is internal to "how a line is classified",
and none reaches a question about where a block ships. The one that did — the subject test needing two
vocabularies — is why ε exists.

## §2a Measured — the classifier as it stands

**Marked ⊕ = re-measured by the author after being reported; unmarked = reported and not independently
re-run, and flagged rather than absorbed.** Every ⊕ item's command is in §1 or §3.

⚠ **The `callsite` predicate does not do what its own comment says, and the `$( )` hole is one instance of
that rather than the defect itself.** The comment states the rule — *"a line is a call site because a
vocabulary token stands in COMMAND POSITION"* (`-audit.sh:138-140`). The code (`:141-146`) does something
narrower in three steps: an outer `re.search` establishes only that *some* command position exists on the
line — true of almost any line beginning with a word — and the body then tests **the line's first word**
(`WORD.search(ln.lstrip())`), plus a special case for `_measure` after a separator. So the implemented
predicate is *is the first word a block*, and every other command position is invisible.

Three consequences, all ⊕:

1. **The `then ` / `do ` / `else ` alternatives in the outer regex are dead.** A line beginning `if` / `for`
   / `while` / `case` can never be `callsite`, because the subject is the line's first word.
2. **The `_measure` special case decides no row today.** Neutralising `-audit.sh:145-146` on its own line
   leaves `rederive homes` **byte-identical**. Its one live shape in the tree — `-Aii.sh:268`,
   `local n_fx; _measure n_fx ls "$F" || rc=1` — is not a census home (one vocabulary token).
3. **`mention`'s stated premise is false for a command substitution.** `hits_outside_quotes`
   (`-audit.sh:112-115`) deletes `'…'` and `"…"` spans by flat, non-nesting alternation with no shell
   awareness, so `"$(f)"` is deleted **whole**. The predicate decides on quote *characters* and cannot
   distinguish *named in a string* from *called through a substitution*.

## §2b Recorded — the `prose` predicate, and why it is ε's

⚠ **Nothing in §3 implements this section.** It is retained because it is where the measurement was taken
and because it is ε's input; ε's own plan-review owns the decisions.

The umbrella's `prose` row states a written predicate and a measured outcome. The predicate was
re-implemented **from its literal text**, deliberately not from its evident intent
(`memory/feedback_self-authored-test-verifies-intent-not-text.md`). ⚠ **That run is delegated and the author
did not re-implement it; the figures re-measured independently are marked ⊕.**

**Its structural claims reproduce exactly**: the five straddling rows by **identity** (`-Aii.sh:7`,
`-B.sh:7`, `-audit.sh:92`, `-common.sh:10`, `A-rederive.sh:47`), stable under all three sentence-terminator
rules tried; ⊕ the control population of 872 (`grep -h '^\s*#' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l`
less the census's `prose` rows); the census moving with the three new homes the umbrella names; rc=1 RED
until the successors get rows.

⚠ **Its quantitative claims do not reproduce.** Across the six readings measured — two readings of *"part
preamble"* × three sentence-terminator rules — the placement/rationale split came out **14/20** under the
most literal reading and **21/13** under the reading that makes the umbrella's surrounding prose true. The
umbrella states 19/15; no reading reached it, and no treatment of straddlers reached its control figure. The
straddling count was stable across all six; the split was not.

**Two of the fourteen gaps are contradictions rather than ambiguities, and ε must decide them:**

1. ⊕ **"part preamble" excludes the dispatcher, while the same sentence includes it.** `PARTFILES` filters
   on `"A-rederive-" in f.name` (`-audit.sh:59`) and `PARTS` derives from it (`:65`), so a place vocabulary
   *"derived from `PARTS`"* cannot contain the dispatcher — yet the same row lists *"the dispatcher header's
   placement table"* as placement. ⚠ **And the dispatcher's *prose* token is a different object from its
   census stem**, which the umbrella's `partset` row decides against the census; ε must not conflate them.
2. ⊕ **The test's own group vocabulary has no in-scope home.** `GROUPS` exists at `-inventory.sh:168`
   (from `ORDER`, `:46`) inside `INVENTORYPY`, while the test lives in `-audit.sh`'s `HOMESPY`. `CLASSES`
   (`-audit.sh:108-109`) has five keys and no `GROUPS`. **This is the measurement that produced ε**: both
   escapes fail, and the crossing that resolves it is α's.

The remaining twelve are under-specifications — the colon disjunct has no object constraint, the relators are
given in one inflection each, *"a file-name copula"* has no example, no rule says which straddler is refused,
and verdict inheritance across non-census lines is never stated. ⚠ The umbrella's own bar is that three of
them *"would let two competent implementers land materially different classifiers"*; **that bar is not met
here and ε is where it must be**.

## §3 The work

Two deliverables. They are independent of each other and of every memo edit (C1 × C4), so the commit order
is β's to choose; this memo does not manufacture a constraint it measured to be absent.

### β-a — the coverage gate gets a content test with a subject test

The gate asserts that a class name has a row (`-audit.sh:277`), and §1 β3 measures that it notices neither
an emptied cell nor a row for a class that no longer exists.

⚠ **The first draft's predicate — *"the cell cites something the harness can resolve"* — was measured to be
a shape rule with no subject test, which is the defect C2 names.** A cell reading
`` TBD. Refer to `citations`. `` **passes** it. Any cell that mentions any block passes it, so it tests
whether the author typed a backtick.

**The predicate therefore carries an object constraint**: a rule row must cite something **whose class is
that row's own subject** — the class name itself, or an identifier the census assigns to that class. This is
the same constraint β would demand of any other disjunct, and it is derivable rather than listed, because
the census already computes which identifier belongs to which class.

⚠ **The path term is dropped.** It cannot be evaluated where the gate lives: resolving a path reference needs
`TRK` (`-audit.sh:391`), and deciding "inside the harness glob" needs `HN` (`:307`) — both bound *after* the
gate at `:265`. Re-deriving either at the gate re-spells the artifact, which is what `HN`'s own comment
(`:304-306`) exists to prevent. ⊕ Measured: the vocabulary term alone gives the identical verdict on all
nine rows today, so the path term buys nothing and costs a second home.

**Direction two**: the gate additionally reports `ruled - set(byclass)` — a rule row for a class the census
does not emit — as a finding rather than dropping it. That set is empty today; it is what would hold `prose`
if ε forgot the row.

### β-b — one command-position scan, replacing the first-word approximation

**A command begins at line start, after `;` `&&` `||` `|` `(` `{`, and after `then` / `do` / `else`; the
token at each such position is taken, and the line is `callsite` if any of them is in the vocabulary.**
`$(` is included — a command substitution is a command position. That is a single predicate, stated once,
and it is the sentence the code's own comment already carries.

⚠ **A backtick is NOT a command position for this predicate, and the first draft's inclusion of it was
measured to be a regression.** A regex cannot separate a shell backtick substitution from a markdown
backtick, and this harness writes block names in backticks inside strings routinely. ⊕ Measured: a code line
`plantdoc="the \`citations\` block and the \`budget\` block"` classifies `mention` at HEAD — correct, it talks
about blocks — and **`callsite` at rc=0, `9 of 9`** under the backtick disjunct: silently green, the exact
failure class β exists to remove, reproduced one class over. ⊕ The harness contains **zero** legacy backtick
substitutions, so the disjunct buys nothing:

```bash
grep -nE '=[[:space:]]*`|\|\|[[:space:]]*`|;[[:space:]]*`' \
     docs/plans/2026-07-citation-hygiene-A-rederive*.sh | grep -vE ':[0-9]+: *#'
```

⊕ **Validated end to end** in a clone: both crossing spellings, the `then` branch and the `&&` form all
classify `callsite`; the markdown-backtick line and `-B.sh:87` stay `mention`; rc=0, `9 of 9`.

Two consequences:

1. **`mention`'s own predicate does not move.** Because `callsite` is tested first, closing the hole in
   `callsite` closes the `mention` symptom without widening `mention` — which matters, since widening it
   would strip the protection it gives to genuine prose-inside-quotes. ⚠ **The umbrella's §2 phrasing —
   *"β owns `mention`'s command-substitution hole as well"* — is right about the ownership and misleading
   about the site**, and β's §3 rows say the site.
2. **The `_measure` special case retires into the scan.** §2a measures that it is dead; the general scan
   covers its shape (`local n_fx; _measure …` reaches command position after `;`). ⚠ **Dead and subsumed
   are two claims** — only the first is measured today, and §3a O3 asserts the second by planting.

### β-c — the umbrella rows β falsifies

β's mechanism makes three statements in the umbrella untrue. They land in β's commits because they are
bookkeeping β's own change makes true — the discipline the umbrella's `roster` row already states for its
own rename (*"and this memo is updated in the same PR"*).

| the umbrella says | why β falsifies it | β's edit |
|---|---|---|
| §3's `callsite` row: *"nothing to do."* | β rewrites the predicate the row rules on | the row states the command-position scan, that `$(` is a position and a backtick is not, and that `_measure`'s special case retires into it |
| §3's `mention` row: *"nothing to do, and it is a subject test…"* | the row describes the behaviour β corrects — a quoted command substitution reaching `mention` is the silent-green failure §1 β1 measures | the row keeps the subject test and gains its precondition: command substitutions are claimed by `callsite` first, so what reaches `mention` really is text |
| §3's coverage-gate paragraph, which owes *"the content test"* | β lands it, with an object constraint the paragraph does not state | the paragraph states the predicate and that the path term was measured redundant and unevaluable at the gate |

⚠ **§3a's obligation count is NOT β's edit, and the first draft was wrong to take it.** That arithmetic was
false at HEAD *before* β — β discovers it, does not falsify it — and its subject is the line apportioning
obligations to α, γ, δ and ε. The umbrella corrected it in its own commit, together with the re-slice.

⚠ **β does not claim authority it lacks.** The umbrella's §9 authorises *"scopes as stated in §3"*, and §3's
two rows say *nothing to do*. The authority for β's wider reading is §2 — same ratified memo — which says in
terms that `callsite` and `mention` *stop being "nothing to do" rows*, and §3a, which names the observable.
⚠ **That leaves §9 resolving through a section β amends.** β does not repair that itself: an authorisation
clause naming which of its own sections is authoritative is an umbrella-level decision, and it is raised in
§5 for the umbrella rather than taken here.

## §3a β's exit criterion

**β writes its own**; the umbrella apportions obligations to each slice and leaves each slice's criterion to
it. The umbrella's three *properties* are adopted verbatim: **no expected value is written in it**; **a
collapse is two facts, and a literal-gone needle tests only a spelling**; **vacuity guards compare against
base, not against a literal.**

Two shapes are ruled out first. **`RULED BY THE PLAN: N of N` cannot be it** (C4 × C5) — it ranges over the
observed class set. **`homes`'s own raises cannot be it** — they fire on *adding*, and β replaces.

⚠ **And the criterion must not be scored by the mechanism β lands.** β-a *is* the content test; scoring β
with it would let β pass by writing a test that agrees with whatever β wrote. It is therefore an **edit site
with a planted falsification**, never the scorer.

| # | obligation | asserted by planting, not by reading |
|---|---|---|
| O1 | the gate tests content with an object constraint | ⊕ against the **pre-edit** rows, where it is known to fail on exactly two: plant a cell citing a block of the wrong class (`` TBD. Refer to `citations`. `` in the `callsite` row) ⇒ must **fail**; today it passes |
| O2 | a derivation call is a call site in every command position | plant **both** spellings at **two distinct host lines** each ⇒ all four rows classify `callsite`. ⚠ Each plant defines `_partset`/`_roster` as real functions and carries **two** tokens, or it produces no row and reads as a pass (§1) |
| O3 | the scan subsumes `_measure`, and the special case is gone | plant `: ; _measure <two blocks>` ⇒ `callsite` **under the scan** (subsumption), and `-audit.sh`'s second command-position test no longer exists — asserted as *the branch decides no row when neutralised*, which is behaviour, not a line anchor β's own commit invalidates |
| O4 | `mention` still means *text about a block* | two witnesses, because the live one is not enough: `-B.sh:87` (no backtick) **and** a planted code line carrying block names in **markdown backticks** ⇒ both `mention`. The second is the regression the first cannot see |
| G1 | population | every home present in the base census is present after; arrivals are lines β wrote. Read from the run, not predicted |
| G2 | the spec-pair lines stay non-homes | the §0.5 claim, asserted: re-run the `-common.sh` home list and require no spec-pair line in it |
| G3 | nothing else moved class | diff `BY CLASS` against base; the only classes that may move are `callsite`, `mention` and `?` |
| G4 | no block left the table | ⚠ **the umbrella's guard is over `inventory`'s block table, not over `homes`'s rows** — `defined=` unchanged across the change. The first draft transposed "block set" into "census homes"; measured, a part file can leave the part set with every home still present and only `defined=` moving |

⚠ **What β cannot test**, stated rather than left as a gap: whether the object constraint in β-a is the
*right* constraint rather than a defensible one — the criterion can only check that it rejects a
wrong-class citation; and whether the scan's position list is exhaustive for shells this harness does not
write.

## §4 What β does not do

- **β changes no answer about where a block ships.** No tier, no `route`, no `PART_SLICE`, no declaration.
- **β does not split `prose`, write a subject test, or touch a place or group vocabulary.** That is ε, after
  α and γ. β adds **no** key to `CLASSES`.
- **β does not build or use the part-set derivation** (α), and needs neither `PARTS`.
- **β cuts no seam.** With ε carved out, β's growth is a predicate rewrite and a gate term; the authoring
  band is not reached. ⚠ **The first draft authorised a seam cut inside β's commit** — against the branch's
  own two standalone precedents — on the strength of a growth figure that belonged to ε's deliverable.
- **β does not predict the new census figures.** The delta is read from the run in the implementing commit.

## §5 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: the coverage gate's content test, the command-position
scan, and the three umbrella edits §3 β-c enumerates — and nothing else.

**Does not authorise**: any change to a tier, declaration or routing answer (γ); the part-set or roster
derivations (α); the `prose` subject test or any vocabulary it needs (ε); the `reads` named-failure rule (δ);
adding a key to `CLASSES`; cutting a seam; or predicting a figure §4 assigns to the run.

**Raised for the umbrella, not taken here** — each is a decision whose subject is outside β:

- ⚠ **§9 authorises by reference to §3, and β amends §3** (§3 β-c). Naming §2 and §3a alongside §3 in the
  authorisation clause is an umbrella edit; without it, a reader checking whether β was authorised resolves
  §9 into text β wrote.
- ⚠ **The memo gate's hardcoded `2`** (§1 β5). It is **not a classification**, so taking it would widen β by
  the reasoning the partition exists to prevent. ⊕ Measured, the two sites (`-audit.sh:302`, `:385-386`) are
  **not census rows**, so α's `memoset` rule — whose work list is the census — does not reach them either;
  the first draft's assignment to α would have been a drop. The correct form is this program's own: a derived
  count with a raise only where the checks become vacuous. **The umbrella places it.**
- ⚠ **`CLASSES` takes a literal's class from its name** (`-audit.sh:108-109`), so `PARTS="$(_roster)"`
  classifies `partset`. ⊕ Measured: real, and with **no witness in the population at HEAD** — it appears only
  under a plant. It is classification and therefore β-shaped by predicate, but a content rule for literals
  whose content α has not yet changed would make β's criterion predict α's implementation. **α's
  plan-review owns it**, and α is the slice that turns those literals into derivations.

**This memo is the base case** under CLAUDE.md's edge-dense rule: a narrowly-scoped slice under an approved
umbrella, taking its own `/elidex-plan-review`. Passing that review discharges terminality; this memo does
not claim it in advance.
