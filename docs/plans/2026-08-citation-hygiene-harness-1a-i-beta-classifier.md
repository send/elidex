# Citation-hygiene harness PR-1a-i-β — the classifier: one predicate per class, and it says what it does

**Subject**: slice **β** of PR-1a-i, the first of the four the disposition memo's §2 partitions it into.
**Input**: `2026-08-citation-hygiene-harness-disposition.md` — the umbrella. Its §2 (the partition), §3 (the
rule per class) and §3a (the exit criterion) are the authority; this memo does not re-derive them, and where
it contradicts them it says so and says which one moves.

**What β is.** The census `rederive homes` assigns every line it finds a **class**, and the class decides
which of §3's rules applies to it. β changes **how a line is classified** and nothing about where a block
ships. Its whole surface is `classify()` and the two things that read its output: the census's own `BY CLASS`
tally and the coverage gate over the umbrella's §3 rule rows.

**Why it is first.** The umbrella's §2 measured the order: the crossing slice α builds — a bash derivation
called from inside a python heredoc's argument list — is a line today's classifier cannot place. α cannot
land into a census that reds on α's own output, so β precedes it.

**What this memo adds to the umbrella.** Three things the umbrella left open, each measured here rather than
argued: the **target state** of the `callsite` and `mention` rows (§3 of the umbrella still rules both
*nothing to do*, which is the state β exists to change); the **predicate** β replaces them with; and β's own
**exit criterion**, because the umbrella's is stated for a slice that no longer exists.

## §0.5 / §3. Spec coverage map

β touches no spec text. It touches the **classifier that decides whether the lines carrying the program's
spec pairs are homes**, and those lines are what this table is for: at HEAD none of them is a census row, and
β's obligation is that this is still true afterwards. The pairs are the umbrella's — they are the set PR-2's
`citations` comparison ranges over — and they are reproduced rather than re-derived.

| Spec section | Step | Branch | Touch (call site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | classify the line that carries it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | classify the line that carries it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | classify the line that carries it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | classify the line that carries it | `fixtures` payload / `citations` lookup | `classify` — must stay **not a home** | ✓ | no |

**Breadth**: K=3 specs, M=4 entries — the umbrella's complete set, and β adds none (verified 2026-08-16
against the umbrella's own §0.5/§3 table, `2026-08-citation-hygiene-harness-disposition.md:53-58`). The Branch column names
the two blocks that carry the pairs; **which lines those are is a command, not a cell**, because β is about
to change what the census says about lines:

```bash
grep -n '§4\.10\.21\|§2\.2\.5\|§4\.2 The MediaQueryList\|webref heading' \
     docs/plans/2026-07-citation-hygiene-A-rederive-*.sh
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep 'common\.sh'
```

⚠ **The claim in the Touch column is falsifiable and was run** (verified 2026-08-16 at `cdd78dc6`): the
`-common.sh` lines the census reports as homes are 4, 9, 10, 212, 233, 507, 509, 516, 518, 520 and 592, and
every line carrying a spec pair — the `fixtures` payload and `citations`' four `webref heading --exact` calls
— is outside that set. They are not homes because they carry no two vocabulary tokens and match no shape
rule, **not** because anything exempts them; a predicate change that made a block name visible on one of
them would pull it in, which is exactly what this row is a guard against.

## §1 Measurements

Numbered **β1…**, not `D<N>`: `D<N>` is the umbrella's namespace and the harness's own memo gate resolves
every `D<N>` in any `2026-08-citation-hygiene-harness-*.md` against the umbrella's §1 bullet list, so a
fresh `D`-number opened here would dangle by construction — **measured, by writing one and watching this
memo's own first gate run red.** Citations of the umbrella's existing D-numbers below are its.

**No expected value is written beside a command whose subject is HEAD.** Where a figure appears it describes
a **sandbox tree that does not exist on HEAD** — a planted spelling, a patched classifier — which is the
umbrella's own exemption (`§1`), and it is recorded as a falsification rather than as a value a reader
re-derives today.

Every measurement below was taken in a throwaway clone, never in a worktree:

```bash
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
```

⚠ **Never `source` the dispatcher.** `docs/plans/2026-07-citation-hygiene-A-rederive.sh`'s last line is
`"${1:-all}" "$@"`, so sourcing it re-runs the whole suite recursively; a previous session reached hundreds
of processes before the run was killed. Plants go on the **same line** — inserting one shifts every line
number the run is about to report.

- **β0 the baselines**, so that everything below is a delta and not a reading:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes            # rc, BY CLASS, RULED BY THE PLAN
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh selfcheck
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory \
     /Users/kazuaki/repos/send.sh/elidex-wt-citeaudit/docs/plans        # needs the A-i/A-ii/A-iii memos
python3 .claude/skills/elidex-plan-review/preflight.py <one memo>       # one path; a glob exits 2
bash scripts/trip-wires.sh
```

- **β1 the two spellings** — the umbrella's measured case, re-run: unquoted `?` at rc=1, quoted `mention` at
  rc=0. §2a.
- **β2 the complement** — which spellings today's classifier already places correctly, measured alongside the
  ones it does not, so that "it is broken" is a comparison rather than a selection. §2a.
- **β3 the coverage gate's content blindness**, in **both** directions: an emptied rule cell, and a rule row
  for a class the census does not emit. §2a.
- **β4 the `prose` predicate, re-implemented from the letter of the umbrella's text** rather than from its
  intent, so that the under-specification surfaces instead of being filled in silently. §2b.
- **β5 the third memo.** Adding a `2026-08-citation-hygiene-harness-*.md` file to the tree — which is what
  landing *this* memo does — makes the memo gate print `LIMIT: 3 of the 2 memos are present; the checks
  ranged over those`, at rc=0, with the new file scanned. Measured in a clone at `cdd78dc6` by writing a
  three-line memo at this file's name and re-running `homes`. The arity is a literal in the gate
  (`docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh:385-386`; the glob it counts is at `:302`),
  and the umbrella schedules five more
  per-PR memos, so the literal is a stated quantity in mechanism that every later slice falsifies again.
  **Disposition in §5.**

## §2 Coupled invariants

Required by `/elidex-plan-review` Pre-condition #3. β is small but not simple: its subject is a classifier
whose source lives inside the files it classifies, so several of these intersect through β's own text.

- **C1 totality** — every censused line receives a class, and an unplaceable line is `?`, which is **RED**.
  This is what makes a classifier change falsifiable at all.
- **C2 every class is a subject test** — each terminal class is reached by a **positive** predicate.
  A fallthrough class silently absorbs whatever the predicates above it missed, and the census's own comment
  records that happening twice (`-audit.sh:135-140`, `:147-150`).
- **C3 self-reference** — the classifier is inside the census's glob, so a predicate spelled as a literal
  **becomes a home of the fact it censuses**. The umbrella records the measured instance: a first version of
  the `prose` test wrote the part stems out and the census reported the test itself at `?`.
- **C4 class set ↔ rule rows** — the coverage gate compares the census's class names against the umbrella's
  §3 rule-row keys (`-audit.sh:265-284`). Splitting a class is simultaneously a mechanism change and a memo
  change.
- **C5 population vs classification** — β must move lines **between** classes, not into or out of the census.
  Every figure in the umbrella keyed to `HOMES:` is downstream of the population.

| pair | intersection | disposition |
|---|---|---|
| C1 × C2 | If every class is a positive test, the residue must be `?` and RED — so **making a predicate stricter can turn a green line red**, and that is the intended direction. β's two subject-test fixes are exactly this: the lines they stop claiming must be claimed by another predicate, or the census reds. Both spellings must therefore be re-homed, not merely evicted. | §3 β-3 |
| C2 × C3 | The `prose` test's place vocabulary must be **derived from the part set**, or the test is a home. But the part-set derivation is **α's**, which lands *after* β — so β derives from what exists at β's HEAD (`PARTS`, `-inventory.sh:45`) and α's collapse re-points it. Stating that is β's job; discovering it in α is the many-homes shape. | §3 β-1, §4 |
| C1 × C4 | A class split adds class names the gate has no rows for, so the census reds **until the umbrella's §3 gains the rows**. Mechanism and memo therefore land in **one commit**, not two. | §3 β-1 |
| C4 × C5 | The gate ranges over the **observed** class set, so retiring a class removes it from numerator and denominator alike and the gate stays green. `N of N` can therefore never be β's exit criterion. | §3a |
| C3 × C5 | β's own new source is inside the glob, so β **moves the population by its own text** — `HOMES:` is expected to rise. C5 is not "the population is unchanged"; it is "the population changes only by lines β itself wrote, and by a measured amount". | §3a |
| C2 × C4 | `mention` and `callsite` are ruled *nothing to do* in the umbrella's §3 while its §2 and §3a make them β's obligations. The gate cannot see the contradiction: it checks that a key has a row, never what the row says. So the contradiction is **β's to close**, in both directions — the predicate and the row. | §3 β-2, β-3 |

⚠ **This is the base case, not a new umbrella.** β is a plan-reviewed slice under an approved umbrella whose
§2 drew the partition, and every intersection above is internal to "how a line is classified". None of them
reaches a question about where a block ships; that is γ's set, and the fact that this table can be written
without naming a tier is the evidence the seam is in the right place.

## §2a Measured — the classifier as it stands

All rows below are same-line plants in a `git clone --local` sandbox at `cdd78dc6`, with `_partset` and
`_roster` defined as real functions (otherwise they are not in `VOCAB` and the census cannot see the call at
all — the test would be vacuous). **Marked ⊕ = re-measured by me after being reported; unmarked = reported
and not independently re-run, and flagged as such rather than absorbed.**

| # | plant | class | rc |
|---|---|---|---|
| β1 ⊕ | `python3 - $(_partset) $(_roster) <<'PY2'` | `?` | **1** — `RULED BY THE PLAN: 9 of 10 -- MISSING: ?` |
| β1 ⊕ | `python3 - "$(_partset)" "$(_roster)" <<'PY2'` | `mention` | **0** — `9 of 9`, silently green |
| β2 ⊕ | `if [ -n "$x" ]; then _partset; _roster; fi` | `?` | 1 |
| β2 ⊕ | `PARTS="$(_roster)"` | **`partset`** | 0 — the class comes from the *variable name*, not the content |
| β2 | `_partset && _roster`, `_partset > /tmp/x; _roster > /tmp/y` | `callsite` | 0 — correct |
| β2 | `'$(_partset)'` (single quotes — genuinely an inert string) | `mention` | 0 — correct |
| β2 ⊕ | `python3 - "$(_partset)" <<'PY2'` and `python3 - $(_partset) <<'PY2'` — **one** token | **no row at all** | 0 |

Four things follow, and the fourth is the one that changes β's shape.

1. **The two spellings differ only in quoting, and that flips the build between RED and silently green.**
   For a whitespace-free value the quoting carries no semantic weight, and the quoted form is the likelier
   one to be written.
2. **`mention`'s stated premise is false for a command substitution.** The umbrella's row says *"every
   vocabulary token on the line sits inside a quoted string, so it talks about a block"*
   (`-disposition.md:485`). `hits_outside_quotes` (`-audit.sh:112-115`) deletes `'…'` and `"…"` spans by flat,
   non-nesting alternation with no shell awareness, so `"$(f)"` is deleted **whole** — the predicate decides
   on quote *characters* and can never distinguish *named in a string* from *called through a substitution*.
3. **The `then ` / `do ` / `else ` alternatives in the `callsite` regex are dead.** The outer `re.search`
   admits them, but the subject is `WORD.search(ln.lstrip())` — always the line's **first** word — so a line
   beginning `if` / `for` / `while` / `case` can never be `callsite`. β2 ⊕ measures one.
4. ⊕ **A line calling the derivation ONCE is not a census row at all, and that bounds what β-3 buys.**
   `rows.append` is gated on `if kinds:`, and the enumeration kind needs `len(hits) >= 2` (`-audit.sh:188`).
   Measured with a discriminating control: appending two single-`_partset` call lines takes `V` 41 → 43 — so
   the token *is* in the vocabulary, and the test is not vacuous — while `HOMES:` stays **70** and `BY CLASS`
   is unchanged; adding a second token to the same line takes `V` to 44 and `HOMES:` to **71**.
   ⚠ **This is the census working, not a hole.** The admission rule is *this line enumerates two or more
   members of the set*, and a single call enumerates nothing; `callsite` exists precisely to say that a line
   which **does** clear that bar is a call rather than an enumeration. So β-3's value is scoped exactly to
   lines that clear it — and the crossing α actually builds does, because it passes **two** derivations on
   one line, which is the umbrella's measured case. Stating the bound is what stops β-3 from being read as a
   promise about every call site.

**The coverage gate is blind to a rule's content in both directions** (β3, both ⊕, same-line memo edits):

| plant | `RULED BY THE PLAN` | rc |
|---|---|---|
| `mention`'s rule cell **emptied**, key kept | `9 of 9` | **0** |
| a rule row for `ghostclass`, a class the census never emits | `9 of 9` | **0** |

The first is the umbrella's claim, and it is **weaker than the umbrella states**: not only `TBD` — a blank
cell passes too. The second is the mirror the umbrella does not mention, and **it is the one that bites β**:
the gate is a one-directional subset test (`set(byclass) - ruled`, `-audit.sh:281`), so **a rule for a
retired class is invisible** — and β retires `prose`.

## §2b Measured — the `prose` predicate, built from the letter

The umbrella's `prose` row states a written predicate and a measured outcome for it. The predicate was
re-implemented **from its literal text**, deliberately not from its evident intent, so that every place the
letter under-determines the test surfaces as a gap rather than as a silent choice
(`memory/feedback_self-authored-test-verifies-intent-not-text.md`). ⚠ **This run is delegated and I did not
re-implement it; the two figures I did re-measure are marked ⊕.**

**Its structural claims reproduce exactly**: the five straddling rows by **identity**, not merely by count
(`-Aii.sh:7`, `-B.sh:7`, `-audit.sh:92`, `-common.sh:10`, `A-rederive.sh:47`), stable under all three
sentence-terminator rules tried; ⊕ the control population of **872** (911 comment lines across the censused
files, less the 39 census rows — `grep -h '^\s*#' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | wc -l`);
`HOMES:` 70 → 73 with the three new homes being exactly the three the umbrella names; rc=1 RED until the new
classes get rows; and `-audit.sh` crossing into the 700–800 authoring band.

⚠ **Its quantitative claims do not reproduce, and that is this slice's most consequential finding.** The
umbrella states **19 placement / 15 rationale**, and `rationale` for 849 of the 872 control lines. Across the
**six readings measured** — two readings of *"part preamble"* × three sentence-terminator rules — the split
came out **14/20** under the most literal reading and **21/13** under the reading that makes the umbrella's
own surrounding prose true. No reading reached 19/15, and no treatment of straddlers reached 849. The
straddling count was stable across all six; the placement/rationale split was not.

⚠ **Two of the fourteen gaps found are not ambiguities but contradictions, and β must decide them:**

1. **"part preamble" excludes the dispatcher, while the same sentence includes it.** ⊕ Verified by reading:
   `PARTFILES` filters on `"A-rederive-" in f.name` (`-audit.sh:59`) and `PARTS` derives from it (`:65`), so
   a place vocabulary *"derived from `PARTS`"* cannot contain the dispatcher — yet the same row lists *"the
   dispatcher header's placement table"* as placement. **Swing: 7 rows**, which is most of the 14-vs-21
   spread, and it is the whole distance between the two readings.
2. **The test's own `GROUPS` literal lands at `?` and nothing says to admit it.** ⊕ Verified by reading:
   `CLASSES` (`-audit.sh:108-109`) has five keys and no `GROUPS`, so the test's own group vocabulary is an
   ALL-CAPS literal the shape rules place nowhere. The umbrella's consequence (iii) predicts the home and
   does **not** say to add the key — so implemented to the letter, the census reds **permanently**.

The remaining twelve are under-specifications with named assumptions — the colon disjunct has no object
constraint (it alone fires on 15 of 36 control placements), the relators are given in one inflection each
while the tree writes others, *"a file-name copula"* has no example, no rule says which straddler is refused
versus deferred, and verdict inheritance across non-census lines is never stated. Three of them —
the colon, the copula and the straddler disposition — are the ones that would let two competent implementers
land materially different classifiers while both claiming to follow §3.

## §3 The work

β lands **one commit** containing the mechanism and the umbrella-memo rows it makes true, because C1 × C4
means the census is red between them.

### β-1 — split `prose` on a written subject test

The umbrella's four consequences — three rows instead of one, `proseunsettled` RED while non-empty, the
census moving, the seam cut while writing — are adopted unchanged; §2b reproduces all four structurally. What
β adds is the part the umbrella left to the letter of a sentence, and §2b shows that letter does not
determine a test. **β's first act is therefore to make the predicate decidable, and to say so in the memo
rather than in the implementer's head.**

Three decisions, each forced by a measurement rather than chosen:

1. **The place vocabulary is the part set *plus the dispatcher*, and the dispatcher is named as an addition
   rather than assumed into `PARTS`.** The literal reading excludes it (§2b gap 1) and would rule the
   dispatcher's own placement table `rationale` — the single largest block of genuine placement prose in the
   tree. The umbrella's sentence wants both; only one of them can come from `PARTS`. ⚠ **α re-points this**:
   once the part-set derivation lands, "the part set plus the dispatcher" is the *text set* α's `partset`
   rule already defines, so β writes it as one expression that α replaces with a call — β does **not** spell
   a second list (C2 × C3).
2. **Every disjunct carries an object constraint, including the colon.** The umbrella gives the relator one
   (*"whose object is a part or a group"*) and gives the colon, the copula and the `holds` form none.
   Measured, the unconstrained colon is the dominant false positive — it alone accounts for 15 of the 36
   control-set placements, on lines like `# Rationale: five review rounds…`. **A disjunct with no object
   constraint is a shape rule with no subject test**, which is the defect the census's own comments record
   twice; β applies the same constraint to all six.
3. **Every straddling row is `proseunsettled`.** The umbrella says the test *"refuses one loudly and rules
   four `rationale`"* without saying which or why. A rule that cannot be stated cannot be implemented, and
   the failure direction matters: `proseunsettled` is RED while non-empty, so refusing all five **stops the
   build** until a human dispositions them, whereas silently ruling four `rationale` leaves four unexamined
   rows in the class whose rule is *do not touch*. The umbrella's own consequence (ii) — *"a refusal that
   exits 0 accumulates exactly like the class it came out of"* — chooses this direction.

⚠ **`GROUPS` is admitted to `CLASSES` in the same edit** (§2b gap 2). Otherwise the test's own group
vocabulary is an unplaceable ALL-CAPS literal and the census reds permanently — the test would be unable to
land, which is a hard blocker the umbrella's consequence (iii) predicts without resolving. The key maps to
`groupvocab`, which is the class whose rule already owns the group vocabulary.

⚠ **No placement/rationale figure is written into this memo.** The umbrella's 19/15 is not reproducible from
the text that is supposed to produce it (§2b), and repeating either measured alternative would make β's memo
the second site of a number whose subject is an implementation that does not exist yet. The implementing
commit reports what its own test returned; §3a asserts the test's *properties*, never its counts.

### β-2 — the coverage gate gets a content test

The gate today asserts that a class name has a row (`-audit.sh:277`), and §2a measures that it notices
neither an emptied cell nor a row for a class that no longer exists. β closes both directions, because β
creates an instance of each: it writes three new rows, and it **retires `prose`**.

- **Content**: a rule cell must **cite something the harness can resolve** — a `path:N` inside the harness
  glob, or a backticked identifier in the census's vocabulary. This is structural rather than a spelling
  test, which is what §3a's second property demands; a blank cell, a `TBD` and a cell of boilerplate that
  says nothing about its class all fail it, and no wording is prescribed.
- ⊕ **Measured against the nine rows as they stand**: seven pass and **exactly two fail — `mention` and
  `callsite`**, the two rows the umbrella rules *nothing to do* and the two β-4 rewrites. That the predicate
  fires on precisely the known-bad pair and on nothing else is the evidence it is the right predicate rather
  than a threshold picked to pass.

```bash
# the measurement, re-runnable at any head: extract §3's rule rows, resolve each cell's citations
python3 - <<'PY'
import re, subprocess, pathlib
P = pathlib.Path("docs/plans/2026-08-citation-hygiene-harness-disposition.md")
s3 = re.search(r"^## §3 .*?(?=^## §|\Z)", P.read_text(), re.S | re.M).group(0)
files = sorted(pathlib.Path("docs/plans").glob("2026-07-citation-hygiene-A-rederive*.sh"))
parts = [f for f in files if "A-rederive-" in f.name]
r = subprocess.run(["bash","--norc","--noprofile","-c",
                    "set -e; %s; declare -F" % "; ".join('. "%s"' % f for f in parts)],
                   capture_output=True, text=True)
V = {l.split()[-1] for l in r.stdout.splitlines()} | {f.name.split("A-rederive-")[1][:-3] for f in parts}
for name, body in re.findall(r"^\| \*\*([a-z]+)\*\* \|(.*)$", s3, re.M):
    t = re.findall(r"`([^`]+)`", body)
    ok = any(re.search(r"\.(sh|md|py):\d+", x) for x in t) or any(x in V for x in t)
    print("  %-12s -> %s" % (name, "PASS" if ok else "FAIL"))
PY
```

- **Direction two**: the gate additionally reports `ruled - set(byclass)` — a rule row for a class the
  census does not emit — as a finding rather than dropping it. Today that set is empty; the moment β lands
  it would hold `prose` if β forgot the row, which is exactly the regression the assertion exists for.

### β-3 — one command-position scan, replacing the first-word approximation

⚠ **The `callsite` predicate does not do what its own comment says, and the `$( )` hole is one instance of
that, not the defect itself.** The comment states the rule — *"a line is a call site because a vocabulary
token stands in COMMAND POSITION"* (`-audit.sh:138-140`). The code (`-audit.sh:141-146`) does something
narrower in three steps: an outer `re.search` establishes only that *some* command position exists on the
line — true of almost any line beginning with a word — and then the body tests **the line's first word**
(`WORD.search(ln.lstrip())`), plus a special case for `_measure` after a separator. So the implemented
predicate is *is the first word a block*, and every other command position on the line is invisible. A
derivation called from an argument list is unreachable by construction, and so is one called after `&&`, or
inside `$( )`, or as the second command of a `;`-joined line.

**β replaces the three steps with one scan.** A command begins at line start, after `;` `&&` `||` `|` `(`
`{`, after `then` / `do` / `else`, and **after `$(` or a backtick**; the token at each such position is
taken, and the line is `callsite` if any of them is in the vocabulary. That is a single predicate, stated
once, and it is the same sentence the comment already carries — the change is that the code now means it.

Three things follow, and each is a reason this is the right altitude rather than a patch:

1. **Both spellings land on `callsite`, and neither reaches `mention`.** `$(_partset)` and `"$(_partset)"`
   differ only in whether the substitution sits inside a quoted span, and a command substitution is *code*
   in both. Because `callsite` is tested before `mention`, closing it in `callsite` closes the `mention`
   symptom too, without widening `mention` — which matters, because widening `mention` would also strip the
   protection it gives to genuine prose-inside-quotes.
2. ⚠ **`mention`'s predicate is therefore left alone, and the umbrella's §2 phrasing —
   *"β owns `mention`'s command-substitution hole as well"* — is right about the ownership and misleading
   about the site.** β's memo says the site: the hole manifests at `mention` and is fixed at `callsite`.
   §3a's observable survives intact (*"unquoted must classify `callsite`, quoted must not classify
   `mention`"*), and β's stronger form is that **both** spellings classify `callsite`.
3. **The `_measure` special case retires into the scan, and it is dead already.** Measured in a clone at
   `cdd78dc6`: neutralising `-audit.sh:145-146` on the same line leaves `rederive homes` **byte-identical**,
   so it decides no row in today's population. Its one live shape in the tree —
   `-Aii.sh:268`, `local n_fx; _measure n_fx ls "$F" || rc=1` — is not a census home (one vocabulary token,
   no shape rule), which is why the branch has no witness. The general scan covers that shape anyway, so
   the special case is removed rather than carried: a second home of the command-position rule, with nothing
   depending on it, is exactly what this program collapses.

```bash
# the measurement, re-runnable: neutralise the branch on its own line and diff the census
d=$(mktemp -d); git clone -q --local --no-hardlinks . "$d"
bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes > /tmp/base.txt
python3 - "$d"/docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); L = p.read_text().splitlines()
assert '_measure' in L[144] and 're.search' in L[144]
L[144] = L[144].replace('if re.search(', 'if False and re.search(')
p.write_text("\n".join(L) + "\n")
PY
bash "$d"/docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | diff /tmp/base.txt -
```

### β-4 — the umbrella rows β falsifies

β's mechanism makes four statements in the umbrella untrue, and they land in **β's own commit**, because the
coverage gate is red in between (C1 × C4) and because a memo edit deferred to a later PR is the many-homes
shape this program exists to close. This is the same discipline the umbrella's `roster` row already states
for its own rename (*"and this memo is updated in the same PR"*).

| the umbrella says | why β falsifies it | β's edit |
|---|---|---|
| §3's `callsite` row: *"nothing to do."* | β rewrites the predicate the row rules on | the row states the command-position scan and that `_measure`'s special case retires into it |
| §3's `mention` row: *"nothing to do, and it is a subject test…"* | the row describes the **behaviour β exists to correct** — a quoted command substitution reaching `mention` is precisely the silent-green failure §2 measured | the row keeps the subject test and gains its precondition: command substitutions are claimed by `callsite` first, so what reaches `mention` really is text |
| §3's `prose` row | β splits the class in three | the row is **replaced by three rows**, per the umbrella's own consequence (i) |
| §3a: *"It carries **eight obligations**: seven of the nine class rules, plus `covgate` … as the eighth"* | ⚠ **the arithmetic contradicts its own apportionment two paragraphs later.** The apportionment gives β `prose` + `covgate` + the two subject-test rows, α four, γ one, δ one — which is **all nine** class rules plus `covgate` = **ten**, not eight. "Seven of the nine" was written while `mention` and `callsite` were regression guards, and the ⚠ paragraph that promoted them to obligations did not move the count. The two measured figures beside it (*"at HEAD it fails with 8 of 8"*, *"it fails with 7 of 8"*) were taken against the eight-obligation script, so they describe a criterion that is no longer the criterion — and that script exists in no tree | β states **its own** criterion (§3a below) and corrects the umbrella's count to the apportionment it already carries. β does not restate α's, γ's or δ's obligations; each states its own |

⚠ **β does not claim authority it lacks.** §9 of the umbrella authorises *"PR-1a-i's and PR-1a-ii's scopes as
stated in §3"*, and §3's two rows say *nothing to do*. The authority for β's wider reading is the umbrella's
**§2**, which is in the same ratified memo and says in terms that `callsite` and `mention` *stop being
"nothing to do" rows* — and §3a, which names the observable. So the umbrella already decided this; §3's table
is the site that did not get the edit. β transcribes a decision, and does not take one.

## §3a β's exit criterion

**β writes its own, and the umbrella's is not inherited.** §3a of the umbrella apportions obligations to β
but its criterion is scored out of eight while its apportionment names ten (§3 β-4), and the script that
produced its two measured figures exists in no tree — it was written in a session-keyed sandbox and is gone.
Re-deriving it is cheaper than reconciling it, and the umbrella's three *properties* survive unchanged and
are adopted here verbatim: **no expected value is written in it**; **a collapse is two facts, and a
literal-gone needle tests only a spelling**; **vacuity guards compare against base, not against a literal**.

Two shapes are ruled out before anything is written, both by the umbrella's own measurements:

- **`RULED BY THE PLAN: N of N` cannot be it** (C4 × C5). It ranges over the *observed* class set, so
  finishing a class removes it from numerator and denominator alike. β **retires `prose`**, so the gate would
  go green for the work being done, which is the failure mode inverted.
- **`homes`'s own raises cannot be it.** They fire on *adding* — a row with no class, a class with no rule —
  and β's work is a *replacement*. The one exception is genuinely useful and is kept: after the split,
  a `placement`/`rationale`/`proseunsettled` class with no §3 row reds, which is C1 × C4 doing its job.

⚠ **And the criterion must not be scored by the mechanism β lands.** β-2 *is* the coverage gate's content
test; scoring β with it would let β pass by writing a test that agrees with whatever β wrote. The content
test is therefore treated as an **edit site with a planted falsification** — does it exist, and does it fail
on a `TBD`-ed row — never as the scorer.

So β's criterion asserts over **edit sites and planted behaviour**, and it carries **four obligations**, one
per deliverable, plus the shared vacuity guards:

| # | obligation | how it is asserted (planted, not read) |
|---|---|---|
| 1 | `prose` is split on a written subject test | `BY CLASS` no longer lists `prose` and does list all three successors; `proseunsettled` non-empty ⇒ **rc=1**; the umbrella's §3 carries a row for each successor and none for `prose` |
| 2 | the coverage gate tests content, not keys | plant `TBD` into one §3 rule cell in a clone ⇒ the gate must red; today it prints `9 of 9` at rc=0 (β-2) |
| 3 | a derivation call is a call site in every command position | plant **both** spellings, at **two distinct host lines** each ⇒ all four rows classify `callsite`; and the `_measure` special case is **gone**, asserted structurally (`-audit.sh:145-146` no longer exists as a second command-position test) rather than by grepping for a spelling |
| 4 | `mention` still means *text about a block* | the one live `mention` row at HEAD — `-B.sh:87` — still classifies `mention`; and a quoted **command** that is not a substitution is not silently claimed (β-3 states why `mention`'s own predicate does not move) |
| G | vacuity guards, against base | every home present in the base census is present after; `defined=` and the roster are unchanged; `HOMES:` rises **only** by lines β itself wrote, and the delta is read from the run rather than predicted here (§4) |

**Guard G is the one that catches the failure the others cannot.** The umbrella measured it: a file can fall
out of the part set with `defined=` dropping by one and every gate still green at rc=0. A count cannot see
that; *no home left the census* can.

## §4 What β does not do, and what it cannot claim

- **β changes no answer about where a block ships.** No tier, no `route`, no `PART_SLICE`, no declaration.
  That is γ's set, and the §2 table above is writable without naming a tier, which is the evidence.
- **β does not build the part-set derivation** (α) and does not use it. β's `prose` place vocabulary is
  derived from what exists at β's HEAD — `PARTS` at `-inventory.sh:45` — and **α re-points it** when the
  derivation lands. Saying so here is the point; the alternative is α discovering a second home.
- **β does not predict the new census figures.** `HOMES:` will rise because β's own source is inside the
  glob (C3 × C5), and the umbrella records a sandbox figure for a tree that no longer matches β's design.
  The number is read from the run in the implementing commit's message, not written here.
- **β does not touch the memo-gate arity** (β5) — see §5.
- ⚠ **What β cannot test, stated rather than left as a silent gap**: that the `prose` class was split on
  **this** subject test rather than some other defensible one — the criterion can only check that a test
  exists, that its verdicts are total and that its refusals are loud; and the *quality* of a `rationale`
  verdict, which is a human read of the rows the test declines to touch. Both are inherited from the
  umbrella's §8 and are not improved on here.

## §5 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: β as a single commit containing the classifier change,
the `prose` subject test with its seam cut, the coverage gate's content test, and the four umbrella edits
§3 β-4 enumerates — and nothing else.

**Does not authorise**: any change to a tier, a declaration or a routing answer (γ); the part-set or roster
derivations (α); the `reads` named-failure rule (δ); widening the census's population beyond the lines β
writes; or predicting a figure §4 assigns to the run.

⚠ **One further classifier defect is measured, and β deliberately does not take it.** ⊕ Planting
`PARTS="$(_roster)"` classifies the line **`partset`**, and `ROSTER="$(_roster)"` classifies `?`: the R3
shape rule takes a literal's class from its **name**, through the five hardcoded keys of `CLASSES`
(`-audit.sh:108-109`), never from what the literal holds. That is a classification defect and therefore
β-shaped by predicate — but it has **no witness in the population at HEAD**; it appears only under a plant.
Taking it would mean β writing a content rule for literals whose content α has not yet changed, which is β's
exit criterion predicting α's implementation, the thing §3a's first property forbids. **It is recorded here
and belongs to α's plan-review**, whose rules are the ones that turn those literals into derivations.

⚠ **β5 — the memo gate's hardcoded `2` — is named, not fixed here.** Landing this memo makes the gate print
`3 of the 2 memos are present`, so β is the commit that falsifies it, and the umbrella schedules five more
memos that each falsify it again. It is nonetheless **not a classification**, which is β's predicate, so
taking it would widen the slice by exactly the reasoning the partition exists to prevent. **It belongs to
the umbrella to place** — the natural home is α's `memoset` rule, whose subject is *one derivation, every
reader calls it*, extended to the second memo set; the literal is a stated quantity in mechanism, and the
correct form is the one this program already uses everywhere else, a **derived count with a raise only for
the case that makes the checks vacuous**. β's plan-review is asked to confirm the assignment, not the fix.

**This memo is the base case** under CLAUDE.md's edge-dense rule: a narrowly-scoped slice under an approved
umbrella, taking its own `/elidex-plan-review`. Passing that review is what discharges terminality; this
memo does not claim it in advance.
