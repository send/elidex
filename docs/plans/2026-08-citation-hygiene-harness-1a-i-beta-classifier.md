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
- **The coverage gate's content test** leaves this memo for **ε**, *both* directions. ⚠ **An earlier draft
  of this bullet said the test was "not constructible over a rule cell's text" and that the umbrella had
  withdrawn it. Both halves of that are now false and the umbrella retracted them.** What is not constructible
  is the *per-class* family — predicates asking a cell to cite its own class's evidence, which red on four
  rows, two of them α's and δ's. The complement was never measured and it is constructible: a non-per-class
  predicate separates a rule cell from a stub and reds on the umbrella's own example of the defect. So the ask
  was **narrowed, not withdrawn**, and ε takes the reverse direction (`ruled - set(byclass)`) and the
  non-triviality clause together, because they are two clauses of one gate. **§2a records the measurements
  this memo took**; the umbrella's §3 records the one that overturned them, and nothing in §3 implements
  either.

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

**Moved** to `2026-08-citation-hygiene-harness-beta-measurements.md`, as a seam cut taken while writing
when round 7's corrections put this memo inside the 700-800 authoring band. Every β<N> label and every ⊕
command is unchanged there; this stub keeps the section number so citations into §1 still resolve.

## §2 Coupled invariants

Required by `/elidex-plan-review` Pre-condition #3.

- **C1 totality** — every censused line receives a class; an unplaceable line is `?`, which is **RED**.
- **C2 every class is a subject test** — each terminal class is reached by a **positive** predicate.
  ⚠ The census's comment at `-audit.sh:179-185` records one measured instance of a fallthrough swallowing a
  home; `-audit.sh:189-192` asserts the same principle for `mention` but records **no** instance, so this is
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

## §2a Measured — the classifier as it stands, and what this memo hands off

⚠ **The `callsite` predicate does not do what its own comment says.** The comment states the rule —
*"a line is a call site because a vocabulary token stands in COMMAND POSITION"* (`-audit.sh:183-185`). The
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
   (`-audit.sh:157-160`) deletes `'…'` and `"…"` spans by flat, non-nesting alternation with no shell
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

## §3 The work — one command-position scan, over shell text, with one home

⚠ **REDRAFTED at round 8's input. The statement this replaces was three sentences about WHERE a token may
stand, and round 7 measured that the defect was never there.** It is kept, struck through in prose rather
than deleted, in the ⚠ below.

**SCOPE.** The rule ranges over **shell text**. A here-document body opened with a **quoted** delimiter is
not shell text: `bash(1)`, *Here Documents* — *"If any part of word is quoted … the lines in the
here-document are not expanded"* — and POSIX XCU, *Here-Document*, to the same effect. ⊕ Measured,
**1623 of 2850 corpus lines (57%)** sit inside `<<'DELIM'` bodies, and `classify` itself is one of them.
A line outside the scope is not `callsite`, not `mention`, and not `?` — the rule does not reach it.

**POSITION.** Within shell text, a command begins at line start; after the control operators `;` `;;` `&&`
`||` `|` and `&`; after `(`; after `{` **as a complete word**; after a case-item's `)`; after `!`; and after
the reserved words `then` `do` `else` `elif` `if` `while` `until` `time`. It also begins after an assignment
or redirection **prefix** (`FOO=1 cmd`, `> /dev/null cmd`). It does **not** begin after `${` … `}` used as
parameter expansion, nor inside `$(( … ))`. Each is cited and probed in the clause table below.

**HOME.** The rule has **one** home, reachable from every payload that needs it. β does not add a second.

⚠ **The three withdrawn sentences, and why.** They said *"a command begins at line start, after `;` `&&`
`||` `|` `(` `{`, after then/do/else/elif/if/while/until, and after `$(` or `<(`"*, with three exclusions
and one declared gap — and the section reasoned about which positions to add. Round 7 measured three things
that make that framing the defect rather than the rule:

1. **The position list was never the binding constraint.** ⊕ 91 code lines are a call site under the landed
   rule and only **3** reach the `callsite` branch, because `classify` runs only when `kinds` is non-empty
   (`len(hits) >= 2`, `-audit.sh:230`, computed from the **raw** line). Every position arm this section
   argued over — `;` `(` `{` and the keywords — fires **zero** times on the population `classify` sees.
2. **The clause the gate closure was withdrawn over cannot be adjudicated on its own evidence.** ⊕ The two
   corpus lines that discriminate the single/double-quote reading are `-audit.sh:94` (a Python alternation
   read as a shell pipe) and `-common.sh:462` (an f-string) — **both are here-document payload**. Under the
   SCOPE clause above neither is shell text, so the clause's population is empty and the question it was
   arguing does not arise. The missing layer was the defect; the clause was its symptom.
3. **The rule cites no specification, in a program whose subject is citation hygiene.** ⊕
   `grep -niE 'posix|ieee|1003\.1|man bash|shell command language' …-harness-*.md …A-rederive*.sh` returned
   nothing. Every clause below now carries one, and three of them were **wrong** when checked against the
   shell this harness itself invokes — see the table.

⚠ **CLAUSES, each cited and each probed on `bash --norc --noprofile` (GNU bash 5.3.15, the interpreter
`-audit.sh:67` invokes).** `f(){ echo RAN; }` throughout:

| clause | authority | probe | previous statement |
|---|---|---|---|
| `${` opens a command position when followed by a blank | bash 5.3 **function substitution**, `bash(1)` | `x=${ f; }` → **RAN** | ⚠ **wrong** — *"parameter expansion is not a command"* is true of `${name}` and false of `${ cmd; }` |
| `{` opens one only as a **complete word** | POSIX XCU *Reserved Words*; `bash(1)` RESERVED WORDS | `{ f; }` → RAN; `{f; }` → **syntax error**; `echo a{b,c}` → `ab ac` | ⚠ **wrong** — stated as *"`{` not preceded by `$`"*, with no delimiter test |
| `!` opens one | POSIX XCU *Pipelines* | `! f` → **RAN** | ⚠ **omitted** |
| a case-item `)` opens one | POSIX XCU *Case Conditional Construct* | `case x in x) f ;; esac` → **RAN** | ⚠ **omitted**, and `case` is live at `-Aii.sh:227`/`:240` |
| an assignment or redirection **prefix** precedes one | POSIX XCU *Simple Commands* (`cmd_prefix cmd_word`) | `FOO=1 f` → RAN; `> /dev/null f` → RAN | ⚠ **omitted**, live at `-Aii.sh:150`/`:203`/`:233`/`:236` |
| `time` opens one | `bash(1)` RESERVED WORDS | `time f` → **RAN** | ⚠ **omitted** |
| a reserved word is recognised only as a complete word | POSIX XCU *Reserved Words* | `--no-if f` → *command not found* | ⚠ **wrong** — `KWEND`'s `\b` matches `--no-if` and `foo-do` |
| an unquoted `#` begins a comment; no position follows it | POSIX XCU *Token Recognition*; `bash(1)` | — | ⚠ **omitted**, live at 5 sites (⊕ `grep -nE 'return \$\?.*#.*;' …A-rederive*.sh` → `-Ai.sh:36`, `-Aii.sh:318`, `-Aii.sh:344`, `-B.sh:54`, `-B.sh:66`); `uncomment()` (`-audit.sh:652`) already decides it, quote-aware |
| an escaped newline continues the logical line | POSIX XCU *Escape Character* | `echo one \` + `f h` → `one f h` | ⚠ **omitted** — line start after a continuation is an **argument** position |
| `fname () compound_command` is a definition, not a call | POSIX XCU *Function Definition Command* | — | ⚠ **omitted**; **33 of the 62** lines the landed rule newly calls call sites are definition lines |

⚠ **An empty population is why a clause is STATED, not why it is skipped** — this section's own standard,
now applied to the clauses it had omitted rather than only to the ones it had listed.
⚠ **This harness already contains a command-position predicate, and this memo did not name it for three
plan-review rounds.** `at_command` (`-inventory.sh:227`) decides the same question for the **call graph**,
and its docstring states the same rule this section states — *COMMAND POSITION, not "appears anywhere"*. Its
lead is `(?m)(?:^\s*|[;&|(]\s*|\b(?:then|do|else|if|while|until)\s+)` plus a `_measure` third-word arm,
i.e. the two things §3 lands. ⊕ Measured with
`grep -n 'def at_command' -A8 docs/plans/2026-07-citation-hygiene-A-rederive-inventory.sh`, the two diverge
**in both directions**. ⚠ **A first draft of this list got the divergence wrong in three places, and this
list is the reconciliation spec, so the error mattered.** `[;&|(]` is a **character class** — `;` or `&` or
`|` or `(` singly — so:

⚠ **Derived by probing the regex, not by reading it** — two earlier drafts of this table read the two
patterns side by side and both got it wrong. Each row below was run:

```python
lead = r"(?m)(?:^\s*|[;&|(]\s*|\b(?:then|do|else|if|while|until)\s+)"
re.findall(lead + "_partset" + r"(?![\w-])(?!\))", probe)     # at_command's own predicate
```

| probe | `at_command` | β §3 | |
|---|---|---|---|
| `{ _partset; }` | no | `callsite` | β's addition |
| `elif _partset …` | **no** (its group is `then do else if while until`) | `callsite` | β's addition |
| `'…_partset…'` in a single-quoted span | not stripped | stripped before the scan | β's addition |
| `x=$(_partset arg)` | **match** — `[;&\|(]` already admits the `(` | `callsite` | ⚠ **not β's addition**; a draft listed it as one |
| `x=$(_partset)` | **no** — the trailing `(?!\))` suppresses the argument-less form | `callsite` | ⚠ **β1's own headline crossing, and `at_command` rejects it**. Unlisted in either direction until now |
| `diff <(_partset) b` | **no**, same guard | `callsite` | as above |
| `n=$(( _partset + _roster ))` | **match** — arithmetic is a command position to it | **not** `callsite` (clause 4) | ⚠ **the one case the two demonstrably contradict**, and it was missing from the list §5 calls the reconciliation spec |
| `cmd & _partset arg` | **match** — the class holds a bare `&` | **no** — β spells `&&` only | ⚠ still β's gap |
| `if` / `while` / `until` | yes | adopted in this draft | was β's gap |

⚠ **The table is disagreement-by-construct, and two measured disagreement *sites* are not in it.** ⊕ Running both predicates over every line × every block name in the harness —
`grep -c '' docs/plans/2026-07-citation-hygiene-A-rederive*.sh` for the corpus, then each regex applied per
line in a throwaway clone — gives exactly four disagreeing lines:
`-common.sh:212` and `:233` (`_measure n_head _wtscan …`, the third-word arm below), and — in the other
direction — `-B.sh:35` and `-common.sh:462`, which β's rule calls call sites and `at_command`'s `(?!\))`
suppresses. ⚠ **`-B.sh:35` is verbatim the site `at_command`'s own docstring records the guard for**:
*"`partition`'s own docstring ends a sentence with \"all)\"; bare occurrences made … the second a caller of
the dispatcher."* So the cheapest reconciliation direction §5 names — widening `at_command`'s lead — would
**re-introduce a false caller edge that guard was landed to remove**, unless β's rule gains a right-boundary
clause. §3 states four clauses measurement forced and none of them is a right boundary; **that is a fifth,
and it is undecided here.**

⚠ **`at_command` also carries a `_measure` third-word arm** that resolves a block name in **argument**
position (`_measure n_head _wtscan`). β's rule declines that — it widens *where* a token may stand, not
*what counts as one* — so a reconciler taking §3 as canonical would delete an arm whose own comment records
that missing it made `_wtscan` read as uncalled. Whether that arm survives reconciliation is **undecided
here** and is part of what the raise below owes.

⚠ **β adopts the three it lacked rather than diverging further, and that is a fix, not symmetry.** ⊕ **At least three** live shell
lines put a command after one of them, and the command below is a **filter**, not the population —
`grep -nE '^[[:space:]]*(if|while|until)[[:space:]]+[a-z_]' docs/plans/2026-07-citation-hygiene-A-rederive*.sh | grep -E '(then|do)[[:space:]]*$' | grep -vE '(if|while|until)[[:space:]]+\['`
returns `-B.sh:131` and `-common.sh:565`, and the second is `if _measure n git …`, exactly the shape this
section says retires into the scan. ⚠ **Dropping the trailing-keyword filter surfaces a third**,
`-common.sh:579`, the same shape ending in a line continuation rather than `then`; the `[a-z_]` class also
excludes any command beginning with a capital. None of the three carries two vocabulary tokens, so none is a
census row — which is why the conclusion survives and the count does not. A derivation call spelled `if _partset && _roster; then`
would have been missed.

⚠ **Two homes for one rule is owed work, not a documented state — and this paragraph twice said otherwise.**
It said *"§5 raises it with a trigger"*; §5 contained no such raise (three review axes measured `at_command`
appearing **zero** times there), which is the failure §5's own closing ⚠ names one section away, repeated in
the commit that added the rule against it. It also gave an ordering reason that does not hold: *"no function
is shared between them … converging them needs the argv crossing α builds"*. ⊕ Measured — the argv transport
already exists at **both** payloads (that command is an **enumerator, not a filter** —
`grep -n 'python3 - "$REPO_ROOT' docs/plans/2026-07-citation-hygiene-A-rederive*.sh` returns **four**
payloads, of which `-audit.sh:53` and `-inventory.sh:39` are the two this sentence is about); what α builds is the roster
**payload**, not the transport. And `_runner` (`-Aii.sh:77-78`) already writes a Python module to a file and
runs it, so Python is shareable across payloads now. **Nothing orders this after α.** The cheapest direction
was never costed either: widening `at_command`'s own lead at `-inventory.sh:234` is one edit.

⚠ **IT IS β's, and the deferral is withdrawn.** This paragraph said *"it is still not β's, and the reason
is scope rather than order"*, one paragraph after measuring that **nothing orders the reconciliation after
α** and that widening `at_command`'s lead is one edit. Round 7 measured the rest, and the deferral does not
survive any of it:

1. **The ε precedent carries, a fortiori.** The disposition's ε row refuses a second `GROUPS` in `-audit.sh`
   as *"the many-homes defect γ exists to close"*, and the only distinction offered was vocabulary-vs-
   predicate. ε had an excuse β lacks: ε **cannot** share until α's crossing lands, so its choice was
   duplicate-or-go-red. β **can** share today — ⊕ `_runner` (`-Aii.sh:77`, four callers) already writes a
   Python module to a file and runs it, and argv reaches every payload host. β declined the option ε was
   denied.
2. **β's copy is worse on ε's own stated axis.** ε's parenthetical condemns the `GROUPS` copy for reporting
   *"as ruled at rc=0 rather than `?`"*. β's copy produces **no census signal at all** — §3a's G1/G3 assert
   byte-identity and it holds. Silently green is the class this slice exists to remove.
3. **There are THREE lead spellings, not two.** ⊕ `grep -nE '\[;&\||\(\?:then' …A-rederive*.sh` returns
   `-audit.sh:144` (β's), `-audit.sh:618` (`RETURNS`, *"`^` or after a `;`/`&&`/`||`"*, in this file's other
   payload and untouched by the implementation) and `-inventory.sh:234`; `GUARD` (`-audit.sh:93`) embeds a
   fourth partial. §5 calls the divergence table *"the work list"* and the work list was short by two.
4. **The two predicates range over different NAME SETS, which this section never stated.** ⊕ `VOCAB` = 42
   (35 `declare -F` ∪ 8 part stems, **`all` absent** — `all()` is the dispatcher's and no part glob sources
   it); `at_command`'s `known` = 36 (the same 35 ∪ `{all}`). So `-B.sh:35` disagrees because **`all ∈ known`
   and `all ∉ VOCAB`** — a name-set fact, not a position fact — and the *"fifth clause, undecided"* that
   this section built on it rests on the wrong cause. A single home must settle whose names it ranges over.

**So β lands one predicate with one home, reachable from every payload that needs it.** The right-boundary
clause and the `_measure` third-word arm are part of that home's rule rather than of a reconciliation owed
to a later slice, and §5's raise is re-scoped accordingly.

⚠ **The list is a rule about shell syntax, and four of its clauses are decisions measurement forced, not
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
4. **`$((` does not open one either; `elif`, `if`, `while`, `until` and `<(` do.** These are the same clause
   as (3) — a two-character sequence whose prefix is in the list, and keywords the `then` / `do` / `else`
   group was written without — and they are decided here so that an implementer does not. ⚠ **Three of the six — `$((`, `elif`, `<(` — have an
   empty population, and that is a measurement rather than a reason to skip them** (`if`/`while`/`until` do
   not; their population is the paragraph above). ⚠ **Both figures below went stale under this memo's own
   implementing commit, which wrote the rule comment that now holds them** — the ⊕ mark survived, the
   numbers did not. Re-derive with `grep -cE '\$\(\('` and `grep -cE '<\('` over `…A-rederive*.sh`. When
   decided, `$((` occurred three times on two lines (`-Aii.sh:240` twice, `:267` once), all arithmetic over
   `_n`/`_tab` with no vocabulary token, and `<(` zero times; `473b9d56` added one of each **to the rule
   comment itself**, so the population is now the clause's own prose. ⚠ An earlier draft said `$((` occurred
   *"twice"*, reporting `grep -n`'s line count as an occurrence count;
   ⊕ **fifteen** of the sixteen `elif` hits are Python and the sixteenth (`-Aii.sh:281`) is a single-quoted `grep` alternation inside a shell line, so the harness writes no shell `elif` **keyword** at all. ⚠ **An earlier draft said all sixteen were Python**, which the umbrella's own D19 lists among round 4's false completeness claims; the exception matters because it is the one place clause 4 meets clause 2's single-quote strip; ⊕ `<(`
   occurred zero times when decided. So no line changes class under any answer, which is the same status §3a already
   states for the guards — the discriminating power is in the **O-rows**, and this clause buys the rule being
   complete rather than a fix:

   ```bash
   grep -nE '\$\(\(' docs/plans/2026-07-citation-hygiene-A-rederive*.sh
   grep -nE '(^|[^_A-Za-z])elif ' docs/plans/2026-07-citation-hygiene-A-rederive*.sh
   grep -cE '<\(' docs/plans/2026-07-citation-hygiene-A-rederive*.sh
   ```

   ⚠ **An empty population is why the clause is stated and not why it is skipped**: a rule the harness does
   not exercise today is exactly the rule the next writer resolves by guessing, and the guess is invisible —
   `homes` stays at rc=0 either way.

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

⚠ **Nothing else in the umbrella is β's to edit — no other *claim*, that is.** The obligation count, the
narrowed content test, the `19/15` figure, D16's owner and §9's authorisation path were all corrected by the
umbrella in its own commits — each was false at HEAD before β, discovered rather than falsified by it.
⚠ **Anchors are the stated exception**, and they are not a claim β is taking over: §4's last bullet requires
every `-audit.sh:N` at or below `classify` to be re-derived in β's own commit, in this memo and in the
umbrella, because β's edit is what moved them. A row's *rule* is the umbrella's; the *line number* under a
citation is whoever last moved the line.

⚠ **§9 now authorises through §3, §2 and §3a — which narrows the circularity and does not remove it.**
An earlier draft of this paragraph said β's authority *no longer* resolves through a section β amends; adding
two sections to the authorising set does not take the third out of it, and §9 says so where it is stated. The
residue closes only when a slice's own rows are quoted into its slice memo, which is not β's to do. **What
was discharged is the narrowing**, before β implements rather than recorded as an owner without a trigger.

## §3a β's exit criterion

The umbrella's three properties are adopted: **no expected value is written in it**; **a collapse is two
facts, and a literal-gone needle tests only a spelling**; **vacuity guards compare against base**.

⚠ **Two shapes are ruled out.** `RULED BY THE PLAN: N of N` ranges over the observed class set. `homes`'s own
raises fire on *adding*, and β replaces.

⚠ **And the guards cannot evidence the work.** β4 measures that β changes no row on the unplanted tree, so
**every guard below passes under a no-op**. That is stated rather than discovered later: the guards bound the
blast radius, and all the discriminating power is in the **O-rows**. ⚠ **Two sites said "O1–O4" after O5 was
added**, and the table's order is O1, O2, O3, O5, O4, so the range was not even contiguous.

⚠ **EVERY row below inherits three preconditions. An earlier draft stated two and the first was WRONG in a
way that made the criterion vacuous** — it said *"a plant must not define a new function; plant inside an
existing function's body"* while O1 says *"each plant defines `_partset`/`_roster`"*. ⊕ `bash --norc
--noprofile -c 'outer() { inner() { echo hi; }; }; declare -F'` lists **only `outer`**: a nested definition
reaches `declare -F` only once the outer function is CALLED, and the census only SOURCES. Literally read, no
plant token enters `VOCAB`, no row is admitted, `homes` is byte-identical to base, and **every O row passes
while verifying nothing** — §1's *"reads as a pass"*, committed inside the paragraph written to prevent it.

(a) **DEFINITIONS at top level**, so `declare -F` sees them and the tokens are in `VOCAB`. Tokens not in
`VOCAB` ⇒ no row ⇒ reads as a pass. (b) **PROBE LINES inside an existing body, and no probe line BEGINS
with a vocabulary token** — the trap the implementing commit hit three times: probes named `_o2a()`/`_o5b()`
made each probe line a call site **on its own name**, three obligations read as passing, diff clean; (b) is
what the earlier draft was reaching for. (c) **At a payload host** (`-audit.sh:53`, `:597`,
`-inventory.sh:39`); elsewhere the plant is off the census's input and reports nothing, which also reads as
a pass. O1 stated (c); O2 and O5 stated none. ⚠ **(a) forced a repair to the scan's own comment**, which
named both plant tokens on one line and so became a two-token `prose` home under any plant satisfying (a);
§5 carries it.

| # | obligation | asserted by planting, not by reading |
|---|---|---|
| O1 | a derivation call is a call site in every command position | plant **both** spellings at the payload host of **each of `all`'s three roster readers** (`-audit.sh:53` `homes`, `-audit.sh:597` `selfcheck`, `-inventory.sh:39` `inventory`). ⚠ **`:538` was this list's spelling until the parts grew, and *"heredoc host sites"* was its label** — the harness has four argv-plus-heredoc payloads and a reader applying the phrase rather than the criterion counts them ⇒ all six rows `callsite`. Each plant defines `_partset`/`_roster` and carries two tokens |
| O2 | the position list's first three decided clauses hold | `echo '$(_partset)' '$(_roster)'` ⇒ `mention`; `echo "${_partset} ${_roster}"` ⇒ `mention`; a code line with two block names in **markdown backticks** ⇒ `mention`. Each is a case a plausible implementation gets wrong |
| O3 | the scan **subsumes** `_measure`, and the special case is gone | plant `local n; _measure <two blocks>` ⇒ `callsite`. ⚠ **Not `: ; _measure …`** — measured, that shape passes under a bare deletion of the special case with no scan written, because `WORD.search` skips `:` and `;` and finds `_measure` as the first word. The `local` shape is the one that discriminates |
| O5 | clause 4's decisions hold, and an empty population is not a reason to skip the plant | plant `elif _partset && _roster; then :` ⇒ `callsite` (the keyword opens a position), and `n=$(( citations + budget ))` ⇒ **not** `callsite` (arithmetic does not). ⊕ Measured at HEAD, both land at `?` and red the census at rc=1, so neither guess was ever invisible. ⚠ **A first draft of this row used `n=$(( $(_partset) + $(_roster) ))` as the negative plant, which is wrong**: the inner `$(` *is* a command position by clause 1, so β must classify that line `callsite` and the plant tested the opposite of what it named. The negative plant has to put bare vocabulary tokens inside the arithmetic, with no substitution. ⚠ **The stated reason for having no obligation here was also wrong** — every O1–O4 plant is synthetic too, so an empty population never separated this clause from the guarded ones |
| O4 | `mention` still means *text about a block* | `-B.sh:87`, the one live `mention` row, still `mention` — and the whole `BY CLASS` line unchanged (G3), which is the general form of the same assertion |
| G1 | population | every home in the base census is present after, and none arrives |
| G2 | the spec-pair lines stay non-homes | re-derive the `-common.sh` home list from a full census run and require no spec-pair line in it; re-derive that all spec-pair lines are in `-common.sh` rather than presuming it |
| G3 | nothing moved class | the **whole** `rederive homes` output diffed against base on the unplanted tree, not three lines chosen from it. ⚠ **An earlier draft named `BY CLASS` alone** — the same three-name form that let a mechanism landing read as census-neutral when `V` and a roster row had moved (§9). Line-number shifts are expected and are not a class move; what G3 forbids is a row changing class or appearing |
| G4 | no block left the table | ⚠ **the named block set of `inventory`'s table, diffed against base — not `defined=`.** The umbrella measured the count form false: a part can leave the table with `defined=` 35 → 35 because an addition elsewhere masks the removal. `defined=` may be reported, never relied on |

⚠ **What β cannot test**: whether the position list is exhaustive for shells this harness does not write, and
whether ε's later work re-opens a case β closed — β's guards range over today's population, which is the
population β leaves unchanged.

## §4 What β does not do

- **β changes no answer about where a block ships.** No tier, no `route`, no `PART_SLICE`, no declaration.
- **β does not split `prose` or touch any vocabulary** (ε), **does not build or use the part-set derivation**
  (α), and **adds no key to `CLASSES`**.
- **β lands no coverage-gate change.** Both directions of it are ε's — the reverse direction and the non-triviality clause the umbrella measured constructible after this memo concluded it was not.
- ⚠ **β's effect on `-audit.sh`'s length is an obligation, not an assertion.** An earlier draft asserted the
  authoring band was not reached, unmeasured, in a phrasing the harness's own `BAND` needle cannot read.
  `-audit.sh` is **698** lines at HEAD (`wc -l docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh`; the glob spelling an earlier draft carried prints nine rows and a total, of which this figure is one), and the
  umbrella's precondition is that a permitted mechanism commit must not be the commit that crosses the size
  trigger. **The implementing commit measures its own tree and cuts the seam while writing if it enters the
  700–800 band**; this memo predicts no number. ⚠ **That figure is itself β's to re-derive, and §5 authorises
  it.** ⊕ It is the **only** claim the gate's `LEN1` needle (`-audit.sh:377`) holds across all five memos —
  `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep POPULATION` prints `stated length=1` —
  so β's implementing commit falsifies it the moment it adds or removes a line of `classify`. ⚠ **An earlier
  draft named `638` as that claim.** `638` survives only in the disposition's `prose` row as *"from 638 lines
  to 754"*, which neither `LEN1` nor `LEN2` matches, so the gate never reads it; the figure it does hold is
  the one `LEN1` holds, above. ⚠ **An earlier draft restated it as *"this bullet's own 655, four lines
  above"* — a second copy of the figure, stale the moment the implementing commit moved it, and nine lines
  away rather than four. Umbrella §8: a second copy of a claim is not a check on the first, it is a second
  thing to keep true. The restatement is deleted; the gate holds the figure.** It is bookkeeping β's own
  change makes true, the same footing §3 β-c puts the two umbrella rows on. ⚠ **An earlier draft added a second, opposite instruction here and it is withdrawn.** It said that if β's
  scan needed more than `-audit.sh`'s headroom, β should *not* cut a seam but report a collision for α to
  dissolve — which contradicted the sentence above it, and rested on D18's claim of a deadlock that the
  umbrella has since retracted as a false universal. There is one instruction: **measure the tree, cut the
  seam while writing if it is **inside the band**.** ⚠ **An earlier draft of this
  sentence said *"enters the band"*, and round 7 graded that a defect on the ground that `BAND`
  (`-audit.sh:385`) reads *past/in/inside/within/below/under* and not *enters*. ⊕ Measured, the grade was
  wrong and the verb is not why: `BAND` is **line-scoped** (`-audit.sh:498`, `w and here`), so a band
  sentence is a claim only when the harness file it is about is named on the SAME line. This sentence
  INSTRUCTS rather than asserts, so `band claim=0` is correct and there is nothing here for the gate to
  read. The verb list binds only where this memo asserts a position, which is the shape §4's first bullet
  records for a still earlier draft. The phrasing is corrected anyway, so the two spellings do not diverge.
  The same bullet diagnoses exactly this for a still earlier draft and then repeated it. ⊕ The umbrella's own prereq cuts (`2abaea1b`, `9647ba4d`) are
  the worked examples, and D18 records what makes a cut census-neutral: place the block in an **existing**
  part, since a new stem enters `VOCAB` and moves the work list.
- ⚠ **β's own citations into `-audit.sh` move when β edits `classify`, and THE POPULATION IS NOT THE SET OF
  STRINGS THAT LOOK LIKE ONE.** The memo gate's path check is range-only, so a stale anchor stays green — an
  in-range anchor pointing at the wrong line is invisible to it. ⚠ **An earlier draft of this bullet scoped
  the obligation to "every `-audit.sh:N` anchor below `classify`'s definition in this memo and in the
  umbrella", and `473b9d56` discharged exactly that and reported it as complete — 24 of them. Measured after
  the fact, the scope was the defect.** `-audit.sh:N` is a **filter**, not the population, and this same
  bullet's neighbour at `-audit.sh:361-366` already names the form it misses. The population is **every
  citation INTO `-audit.sh`, whatever its spelling and whichever file it sits in** — the `path:N` form, the
  **bare `:N`** form the memos write freely, `sed -n 'A,Bp'` and `sed -n 'Np'` commands, and the harness's
  **own self-citations** (the `-audit.sh` comment that offers `-audit.sh:325` as its worked example of a
  name-suffix reference, which `473b9d56` broke). ⊕ Over that population, and naming the sites by what they
  cite rather than by a line number that this bullet's own edits move, the commit **broke six anchors and one
  prose figure that were correct before it**, in four files, none of them visible to the gate: the β memo's
  `hits_outside_quotes` anchor, its `sed -n` command over the two `classify` branches and the *"ten lines
  apart"* that went with them; the disposition's `ruled`-extraction anchor and its `rosterspan` consumer; the
  measurements memo's `selfcheck` payload host; and the self-citation above. Two of them are the sharpest
  demonstration that the scope was wrong — the disposition re-derived the `_s3` extraction and left the
  `ruled` anchor **in the same parenthetical**, and the β memo's bare `hits_outside_quotes` anchor sits one
  line below two anchors the commit did update. ⚠ **It also renumbered a PAST-TENSE record that must not
  move** (the disposition's *"the `all` route's line was … when it was written"*), and it left standing one
  anchor that was **already wrong before it** (the disposition's `selfcheck` host, stale since before this
  slice). ⚠ **And the enumerator that produced this list needed two verdicts overturned by hand**: a bare
  `:273` that cites `-inventory.sh` rather than this file, and a `:538` that is a record of a *former*
  spelling rather than a citation. That failure rate is the evidence for the next bullet, not a footnote to
  it.
- ⚠ **And the widened population is a SEED, not an inventory — which is the real finding, and it is §3's
  neighbour rather than β's bookkeeping.** A bare `:N` carries no path, so no enumerator can decide which
  file it cites; the harness declares exactly this a LIMIT (`-audit.sh:361-366`). Widening a regex cannot
  close it. What closes it is a **spelling rule** — every citation carries its path, or pins a revision the
  way `:209` and `:218` now do — and that is a change to the memos' citation convention and to this
  obligation's shape, so it is **round 7's**, not this commit's. ⚠ **The method claim is withdrawn too**:
  `473b9d56`'s message said the anchors were *"mapped by line CONTENT rather than by shifting a delta"*, and
  all 24 moved by exactly **+42** while the commit's own shift was +45 above `classify` and +42 below. ⚠ **An
  earlier draft of this sentence named the disposition's `-audit.sh:527` as a case where content and delta
  disagreed and the delta won; measured, they agree there** — the referent does sit 42 lines lower. The real
  defect at that site is the opposite one and worse: the number is inside *"the `all` route's line **was** …
  **when it was written**"*, so it is a record rather than a reference, and re-deriving it to the current head
  destroyed what it recorded. A method that cannot tell a citation from the record of a former citation
  cannot discharge this obligation, which is the seed/inventory point again, at the other end. ⚠ **The site set is not "the umbrella's two rows", and an
  earlier draft said it was.** ⊕ Measured — `classify` is defined at `-audit.sh:163`
  (`grep -n 'def classify' docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh`), and the umbrella cites
. ⚠ **Which sections hold them is not written here, because an earlier draft wrote it and was wrong on
  both counts** — it said §1, §3 and §7, and §1 and §7 are now forwarding stubs holding **zero** anchors, and of the rest
  **§3 holds ten of the twelve**, §2, §6 and §9 one each — ⚠ the correction that removed §1 and §7 also
  removed §3, the one the first list had right; and it said *"only one of which (`:151-152`) is in a row §3 β-c names"*, where that
  anchor is in **§2's slice table** and §3's `mention` and `callsite` rows carry no `-audit.sh:N` anchor at
  all, so the count is **zero**.
  Enumerate them rather than copying the list, which would give it a second home and one that goes stale as
  soon as either memo cites another line:

  ```bash
  grep -oE '\-audit\.sh:[0-9]+' docs/plans/2026-08-citation-hygiene-harness-disposition.md \
    | sort -u -t: -k2 -n | awk -F: '$2+0 >= 118'
  ```

  ⚠ **A range's anchor is its FIRST line, and `awk -F'[:-]'` cannot read one** — measured, that spelling
  splits on the leading dash of `-audit.sh`, so `$2` is the string `audit.sh`, every comparison is a string
  comparison against `118`, and the filter passes **every** anchor including `:9-14`. It reported a filtered
  list that had filtered nothing. The form above splits on `:` alone and coerces with `+0`.

  ⚠ **This is the one place §3 β-c's *"Nothing else in the umbrella is β's to edit"* is too strong**, and the
  two statements are reconciled there rather than left to collide: β edits no umbrella **claim** beyond those
  two rows, and re-derives umbrella **anchors** wherever its own edit moved them.

## §5 What this memo authorises

**Authorises**, after `/elidex-plan-review` passes: the command-position scan — **now including the scope
clause that excludes here-document payload, and the single home §3 requires**, since round 7 measured that
the scan is not correct without either; the two umbrella row edits
§3 β-c enumerates; the anchor re-derivation §4 requires, **over the population §4 now defines**; **the
re-derivation of §4's own `-audit.sh` length figure**, which β's edit falsifies and the memo gate reds on;
**§6's record of what happened to this memo's own gate**; **§3a's plant preconditions**; **the scan's own
comment where it names the plant tokens**; and **the §1 seam cut §4's band rule forces**, which lands §1 in
`2026-08-citation-hygiene-harness-beta-measurements.md` and leaves a forwarding stub — and nothing else. ⚠ **That fourth item
was asserted by §4 as *"§5 authorises it"* while this list did not contain it**, and the claim reached a
commit message before it reached this section; a memo may not cite a sibling section it has not opened.
⚠ **The fifth item is on this list because the same thing happened again, one commit later.** `473b9d56`
wrote §6 — a new section, thirty lines — while this list ended *"and nothing else"* and did not contain it.
The failure this section names is *asserting* an authorisation without opening the section; the repair is to
open it, which is what this edit is. It is a standalone commit for that reason, and it carries no
implementation. ⚠ **And it happened a THIRD time, at `3f55da2a`, which amended §3a's exit criterion under a
commit whose subject was the umbrella's owed items** — reachable because §6 listed O5's precondition as owed
*by the umbrella* when O5 is a row of this memo. The sixth and seventh items are that amendment and the
comment repair it forces, opened here rather than asserted elsewhere.

**Does not authorise**: any change to a tier, declaration or routing answer (γ); the part-set or roster
derivations (α); the `prose` subject test or any vocabulary (ε); the coverage gate (ε); the `reads` rule (δ);
adding a key to `CLASSES`; or predicting a figure §4 assigns to the implementing run.

**Raised for the umbrella, with a trigger** — an owner without a trigger is a drop, which this memo has
already done once:

- ⚠ **WITHDRAWN as a raise, and taken into the slice.** This bullet deferred the two-predicate reconciliation
  to α with *"Trigger: α … if α declines it, the next slice to touch either predicate takes it"* — an owner
  with a decline branch and no terminal owner, which is the shape this section itself calls a drop. §3 now
  answers it: the ε precedent carries a fortiori, β can share today where ε could not, and the divergence
  table was short by **two** spellings (`-audit.sh:618` `RETURNS`, `-audit.sh:93` `GUARD`) and blind to the
  **name-set** difference (`VOCAB` 42 vs `known` 36) that produces its `-B.sh:35` row. β lands one predicate
  with one home; the right-boundary clause and the `_measure` third-word arm are that home's rule.
  ⚠ **This bullet did not exist while §3 claimed it did, and then it deferred what §3 had already measured
  was not deferrable.**

- ⚠ **`callsite` and `mention` decide quoted spans by two different rules, and β leaves both.** Clause 2
  strips single-quoted spans only; `mention` keeps `hits_outside_quotes` (`-audit.sh:157-160`), which strips
  both by flat alternation. §2a measures that predicate's premise false — a `"$(f)"` span is deleted whole,
  so it cannot tell *named in a string* from *called through a substitution* — and §3 closes the `mention`
  **symptom** by testing `callsite` first rather than by fixing it. ⊕ Both rules live in `classify`, seven lines
  apart — the `callsite` branch, now `-audit.sh:186-188`, and the `mention` branch at `-audit.sh:193` calling
  `hits_outside_quotes`, defined at `-audit.sh:157` — by
  `sed -n '186,193p' docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh`. ⚠ **An earlier draft of this
  bullet said "four lines apart" and attached a `grep` that returns `:112` and `:151`, measuring neither the
  distance nor the claim.**
  **Trigger: ε**, which owns `mention`'s siblings once `prose` splits, and is the first slice that must state
  a quote rule of its own.

- ⚠ **The memo gate's arity literal — DISCHARGED, not carried** (§1 β5). The trigger this memo set was
  *the slice that lands the fourth `2026-08-citation-hygiene-harness-*.md`*, and the standalone prereq that
  cut §1 out of the disposition landed it and took the fix in the same commit. ⊕ The sites were never census
  rows, which is why α's `memoset` rule would not have reached them — still measurable by
  `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep -cE 'audit\.sh +(344|43[0-9]) '`,
  returning **0**. ⚠ **The needle used to say `(302|39[0-9])`, and those sites moved +42 at `473b9d56`.** It
  still returned `0`, so the conclusion never wobbled — which is the defect: a command that runs but measures
  a *different* claim passes unconditionally, and its `0` was guaranteed regardless of the fact. It is also a
  line-number citation inside a command, a species §4's widened population names and `9f0fe33d`'s sweep did
  not reach. Recorded rather than deleted, because a raise that fires is the evidence the trigger
  form works.
- ⚠ **`CLASSES` takes a literal's class from its name** (`-audit.sh:108-109`), so `PARTS="$(_roster)"`
  classifies `partset`. ⊕ Measured with `sed -n '108,109p' docs/plans/2026-07-citation-hygiene-A-rederive-audit.sh`
  for the map and `bash docs/plans/2026-07-citation-hygiene-A-rederive.sh homes | grep partset` for the
  population: real, and with **no witness at HEAD**. ⚠ **A draft said *both `partset` rows are literals*; the
  umbrella's D19 lists that among round 4's nine false claims and it survived two sweeps.** The census's own
  `why` column settles it: `-inventory.sh:45` prints `literal \`PARTS\`` and `A-rederive.sh:52` prints no
  literal marker at all, being `for _part in …`. One row is a literal and one is a loop, and no line yet has a
  derivation classified by its literal's name. **Trigger: α**, the
  slice that turns those literals into derivations — a content rule for them now would predict α's
  implementation.

**This memo is the base case** under CLAUDE.md's edge-dense rule. Passing its `/elidex-plan-review`
discharges terminality; this memo does not claim it in advance.

## §6 The gate: closed on a distribution, and reopened when that distribution was falsified

**Closed at round 6, on merit, on a stated ground — and the ground was distributional, not a judgement that
the memo was finished.** Six rounds of `/elidex-plan-review`, five axes each; rounds 5 and 6 answered
`executability = YES` unanimously; and of round 6's fourteen CRITs, **zero targeted §3** — eight were in the
umbrella's §9, and seven of the fourteen were defects the previous round's own corrections had introduced.
Round-count and fatigue are not inputs to a stop decision; the target distribution is. Closing on that
ground was `memory/feedback_review-loop-convergence-merit-not-fatigue.md` applied, not evaded.

⚠ **`/elidex-review` on the implementing commit put two CRITs on §3, so the ground is false and the closure
is WITHDRAWN.** A distributional ground is falsifiable by measurement, which is what makes it a ground; this
is the measurement. The two:

1. **§3's third decided clause is inverted by its own corpus.** §3 says the single/double quote asymmetry is
   what *"satisfies every observable"* among three readings. ⊕ Run over the whole harness: strip-single
   against strip-**neither** differs on **0** lines; against strip-**both** on **2**, and on both of those the
   landed reading is the one that is wrong (`-audit.sh:94`, a Python alternation read as a shell pipe, in the
   `GUARD` regex of the file the scan runs over; `-common.sh:462`, an f-string reached through a `(` inside a
   double-quoted span). The two examples that discriminate are **planted** — `_partset` and `_roster` are not
   in `VOCAB`. So the clause is decided by plants and contradicted by the tree.
2. **§3's "it is still not β's" ruling on the second home is contested by a ratified sibling.** §3 defers
   reconciling `at_command` (`-inventory.sh:227`) to α *"scope rather than order"*, having already measured
   that *"nothing orders this after α"*. The disposition's ε row rejects the same shape for `GROUPS`:
   *"spelling a second `GROUPS` in `-audit.sh` is the many-homes defect γ exists to close"*. Whether that
   precedent transfers from a **vocabulary** to a **predicate** is the open question, and it is §3's.

**So the next event for this slice is round 7 of `/elidex-plan-review`, not α.** The implementation stands
at `473b9d56` and is not reverted. ⚠ **An earlier draft gave the reason as *"reverting it would destroy the
measurements above"*, and that is false** — every measurement here was taken in a `git clone --local` and
`git show 473b9d56:…` reproduces the tree entire, so a revert destroys no history. The real reason is scope:
a revert is a mechanism commit, §5 authorises none, and the landed scan is what makes round 7 a review with
something to run rather than a re-reading. Stated as the scope decision it is. What round 7 owns is §3's rule cell.

⚠ **Three things the closure never covered, and one it created:**

1. **The right-boundary clause is undecided** (§3, the fifth clause). It blocks the *reconciliation* §5
   raises, and the raise says so.
2. **The umbrella items round 6 named are still owed**, by the umbrella: §9's nine pre-clause commits, D19's
   probes 1–3 carrying no runnable command against §8's rule, `CSSOM View 1` failing the pinned map in all
   three plan memos, O5's missing plant precondition, and §2's I4 × I5 PR cell pointing at a stub.
3. **The bookkeeping §4 and §5 enumerate** — the length figure, and the anchors, over the population §4 now
   defines rather than the one its enumerator could see.
4. ⚠ **One of the five items this list called *"owed by the umbrella"* was β's own, and the misattribution
   let a β-memo edit ride under an umbrella authorisation.** *"O5's missing plant precondition"* names a row
   of **§3a, in this memo**. `3f55da2a` amended §3a under a commit whose subject was the umbrella's items,
   and §5's closed list did not contain it — the third occurrence of the failure §5 documents twice, by the
   same author who wrote §5's repair one commit earlier. §5 now carries it; this item is corrected rather
   than deleted, because the misattribution is what made the violation reachable.
4. ⚠ **Two items this slice must NOT take, each raised with an owner and a trigger** — an owner without a
   trigger is a drop, which this memo has already done once:
   - **The ordering premise in the disposition** (`2026-08-citation-hygiene-harness-disposition.md:156`)
     stated, present-tense, the predicate `473b9d56` replaced — and it is the stated reason for
     **β before α**, i.e. the order of the whole eight-slice program. *Owner*: the umbrella. *Trigger*: the
     next umbrella commit, and no later than round 7's dispatch. ✅ **FIRED and DISCHARGED at `819d9eed`**,
     which put the sentence in the past tense and marked the constraint spent; §5 β5's convention is that a
     fired trigger is recorded rather than deleted, so this bullet stays as the record.
   - **The harness is executed by no CI job.** ⊕ `scripts/trip-wires.sh` discovers only
     `.claude/tools/*trip-wire.sh`, and `[tasks.ci]` names no harness task — so §4's *"the memo gate reds on
     it"* describes a **manual** gate, and every claim resting on `findings=0` rests on a run somebody chose
     to do. *Owner*: not β — §5 does not authorise touching `mise.toml` or `scripts/`. *Trigger*: before any
     slice cites the gate as evidence that a class of defect cannot land.
