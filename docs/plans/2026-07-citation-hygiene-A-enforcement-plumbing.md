# Plan — Slice A: one spec-label map, landed fail-closed, with a scheduler that runs its suites

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A**. Under that umbrella's approval
boundary this is a **terminal unit** (§9). **Branch**: `webref-cite-audit-tool`, after the §4.0 re-carve.
**Nature**: developer tooling + CI topology + one gate-contract change (§4.2.5). Zero `crates/**` diff.
**Status**: plan-memo, **draft 8**. `/elidex-plan-review` **required before implementation**.

**This memo carries no measured digits of its own.** Every quantity is printed by a function in
`docs/plans/2026-07-citation-hygiene-A-rederive.sh`, which ships on this branch; the memo cites the function
name (`→ rederive keysets`) and a reviewer runs it. Six review rounds produced a stale-or-underived
coordinate finding four times; draft 6's answer was a §15 *describing* commands, and round 6 found three of
its blocks were prose, two hard-coded their own answer, and one re-introduced a stored coordinate. Round 6's
Axis 4 stated the diagnosis exactly — *every measured value re-derived correct; the defects were numbers
whose stated derivation does not derive them*. A description of an executable is not a check. So the
derivation is now an executable, and the §6 fixture bodies live in it, so the memos a reviewer measures are
byte-identical to the ones the test plan ships.

⚠ **Draft 8 extends that from the memo's *digits* to its *control flow*, because the digits were never the
whole defect.** §4.2.3 specifies the control flow of code that does not exist yet. Prose was the only
medium, so a **review round was the only thing executing it** — and it inverted twice consecutively: round
6 found draft 6's reporting arm False in the one row it exists for, and round 7 found that draft 7's fix
made it True in six rows where it must be False. Two inversions in a row is a method failure, not an
attention failure. **The fix**: `rederive armmatrix` grafts §4.2.3 and §4.2.5 onto a copy of `preflight.py`
in a scratch worktree and runs all 17 §5 rows *plus 7 untabulated states* with **three candidate predicates
instrumented side by side**. Every §4.2.3 claim below now cites a measured row rather than an argument. The
implementation PR lands that control flow; the harness is not shipped code. **This changed four decisions
and added three edits nobody had proposed** — items 5, 7b, 7c and §4.2.5's grep-pass sentence.

### §0.1 What Slice A is

`origin/main` carries the same enumeration three times — `preflight.SPEC_LABEL_REVERSE`,
`coverage_map._SPEC_LABEL_MAP` and `cli.COMMON_SHORTNAMES`. Slice A collapses them onto one
`.claude/tools/_webref/spec_labels.py`, and **because that import is the first thing that can make the
plan-review gate's label resolution *fail*, lands it fail-closed from the start** — then gives the resulting
suites a scheduler, because today nothing runs them. Two things the one-line summary must not hide:

1. **A's spec *set* is unchanged (the same 12 pinned specs), but 9 additional *spellings* of those specs
   resolve** — the shortnames themselves (`fetch`, `xhr`, `webidl`, `streams`, `webcrypto`, `ecma262`,
   `ecma402`, `selectors-4`, `geometry-1`). Draft 5 claimed "A changes no resolution outcome"; false.
   Draft 6 attributed the delta to a widened alias list; **also false** — see §4.0's alias deletion.
   → `rederive keysets`
2. **A changes the gate's contract of record** (§4.2.5): a §3 section may declare no spec surface, which
   edits `SKILL.md` Pre-condition #1.

A ships **no detector** (B) and **retires no review policy** (C).

⚠ **The scope boundary stands; the reason draft 4 gave for it does not.** A takes the deduplication; the
948-entry catalog fall-through goes to Slice B. Draft 4's reason, inherited from B §4.1.8, was that a
level-collision makes verification *"silently run against the wrong document"*. **Falsified**: every
level-collision pair B names returns byte-identical `webref heading` output, because webref's `ed/` extracts
are keyed to the series' current spec. Of 203 non-round-tripping shortnames, **195 resolve to the same
document and 8 to a different one** — all cross-series or fork cases; two of the 8 carry the label `DOM`,
harmless not because the label is exotic but because **the pinned map wins before the catalog is
consulted**. §10-Q1 carries the real grounds. §13 hands the correction to B. → `rederive partition`

---

## §0.5 Spec citation table

This slice implements no spec logic. The two citations below are rows the `test_preflight.py` fixtures
carry; both looked up with `.claude/tools/webref`, nothing from memory. → `rederive citations`

| Cite | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| the labelled fixture row (P1b/P2/P3) | HTML §4.10.21 | Constraints | `#constraints` | `heading --exact html 4.10.21` |
| the second row of `dedup.md`, so `seen_pairs`' dedup arm is taken | HTML §4.10.21.2 | Constraint validation | `#constraint-validation` | `heading --exact html 4.10.21.2` |

**P4 needs a *separate* fixture, not a third row.** Its §3 rows are **all** label-less (each cell opening
with `§`); a memo containing *any* labelled row hard-fails under both the correct placement and the mis-sited
one, so P4 would pass vacuously. Neither row is a citation defect; the label-less shape is the input that
falsifies §4.2.2's placement.

⚠ **Draft 6 justified its second labelled row as exercising `seen_pairs` and it did not** — both rows were
distinct `(shortname, section)` keys, so the dedup `continue` was taken zero times and the 21→15 figure it
underwrites was unpinned. `dedup.md` now carries two rows resolving to **one** pair; measured, 2 rows →
`1 unique citation(s) checked`. → `rederive column`

⚠ **This table certifies fixtures, not the slice.** `origin/main` hard-fails a §3 section with no heading,
with no table, and with a header but no data rows, so a zero-spec-surface slice must author fixture
citations and then receives `citation verify: ok` as its headline. That is §1's anchor in A's own file, and
**A fixes it** (§4.2.5) — but A's own memo cannot use the fix, because `SKILL.md`'s Step 0 runs
`preflight.py` from the worktree the memo lives in, so this memo is certified by the *carve's* build until A
lands. → `rederive column`

---

## §1 Ideal anchor — a gate reports on the thing it audited, or it reports on itself

Three failures, one shape. A gate's output is a claim about the artifact under review. When the gate's own
infrastructure is missing, the honest output is a claim about the **gate**.

1. **Landing the shared map naively introduces exactly that inversion.** Replacing a module-local dict with
   an `import` makes resolution *failable* for the first time, and the carve's guard
   (`except Exception: _shortname_for = None`) routes that failure into the per-row *unmapped* bucket — a
   documented soft-warn. Result: every row classified as *author cited a spec I do not know*, and the gate
   **exits 0 having verified nothing**. → `rederive remedies`
2. **Nothing runs the suites.** No `mise` task, no CI job, no hook. → `rederive suites`
3. **A memo whose §3 rows are *all* unmapped prints no `citation verify:` line at all** and exits 0, with
   both capabilities present — `citations` is empty, so the verify block never runs and `elif seen_pairs:`
   never fires. Live, not hypothetical: the in-flight `elidex-wt-c3-plan` memo's 18 rows take this path
   today, and `allunmapped.md` reproduces it. Four drafts missed it. → `rederive column`

The corollary that drives the edit set: **a capability is a process-level fact and must be established once,
before the data loop.** "I cannot map *this* label" is a datum about one row; "I cannot map *any* label" is a
fact about this process. Discovering the second by watching the first makes the failure look like data — and,
as §4.2.2 measures, makes the fix's correctness depend on the *content* of the memo under review.

---

## §2 Coupled invariants

- **J1 — capability ≠ datum.** A row is *unmapped* only if the mapper ran and declined. One return value
  (`None`) must not carry both questions. ⚠ J1 forbids the two questions sharing a *return value*; it does
  **not** require them to share a *site*. Draft 5 read it the second way and broke J3 (§4.2.3 item 3).
- **J2 — the two capabilities must degrade the same direction.** Verifying needs the `webref` CLI *and* the
  label map; measured on the carve, one hard-fails and the other exits 0, while its in-code comment claims
  they "degrade the same way".
- **J3 — one degradation must survive.** `--no-verify --no-grep-pass` must keep working with the tools tree
  absent.
- **J4 — one enforcement mechanism, not two.** If `mise` and `ci.yml` each spell the suite invocation, a
  later suite is added to one and not the other. ⚠ Draft 6 grounded this on "the `trip-wires` shape
  verbatim"; measured, `trip-wires` inlines four `bash` lines in `mise.toml` and has exactly **one** runner,
  so it establishes *script-in-`.claude/tools`*, not the two-caller SoT property. A establishes the second
  caller; the ground is the invariant itself, not a precedent.
- **J5 — A adds no network dependency.** Carried by **T-net** *and* §12 (3) together — neither alone
  suffices, because the gate's only fetch happens in a **child** process (§4.3.3).

J1–J3 live in one function's control flow and cannot be applied one at a time without transiently breaking
each other, which is why §5 measures the configuration matrix rather than a sample. J4 and J5 are
independent.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | fixture | the labelled `§3` row a fail-closed run must still map | §4.4 — `test_preflight.py` fixtures | ✓ — the fixture set is authored, not discovered | no |
| WHATWG HTML §4.10.21.2 Constraint validation | fixture | the second row of `dedup.md`, so the dedup arm is taken | §4.4 — same fixture set | ✓ | no |

**Breadth**: K=1 spec (`html`), M=2 rows → **ok (single PR scope)**. Rows here are test fixtures; a fixture
set larger than the property under test is padding. See §0.5's ⚠ for what this does *not* certify.

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** Nothing here is reachable from page content, script, or a network peer. The
inputs are the plan-memo's path *and its content*: `parse_spec_cell` extracts a label and a section number
from cell text and `verify_citation` passes **both** to a subprocess, so memo content steers control flow
(§4.2.2). Symbols, not line numbers — draft 6 carried eight line cites, all branch-relative, four found by
review and four more by re-derivation. → `rederive anchors`

**Both argv elements stay bounded, and A moves neither outside the pinned set.** `section` is bounded by
`SECTION_REF_RE`, untouched. `shortname` is bounded on `origin/main` by `SPEC_LABEL_REVERSE` and after A by
the pinned `LABEL_TO_SHORTNAME` — a **strict superset over the same 12 specs**, 0 changed, 0 lost, no new
spec. Draft 4 replaced that bound with a 948-entry third-party document fetched at gate time on every plan
review in every lane; that exposure delta leaves with the widening. → `rederive keysets`

**Discovery method.** Every number comes from the script, measured against `origin/main` and never the
branch; branch/lane facts use three-dot ranges; a proposed patch is *measured*, not read (§4.2.2 was found by
applying draft 1's own fix in a sandbox); and a claim inherited from another slice's memo is re-derived
before being relied on (§0's ⚠ is what that produced). **And a claim about *where* something occurs is
grepped by concept, not by string** — three conclusions in this program flipped on that distinction
(§7, §13 item 2, and the B-memo edit list).

---

## §4 The edit set

### §4.0 Step 0 — re-carve on the seam the umbrella already draws

**Two commits, two subjects.** Draft 6 identified the re-carve "by subject, never sha" to end a staleness
class, and created an ambiguity instead: one subject, three referents. They are now distinct —
`carve the cite-audit detector out of the citation sweep` (the existing whole-detector carve, the red
baseline §12(2) builds) and `re-carve the shared spec-label map onto the A/B seam` (Step 0, the A-only
commit §12(3) ranges from). → `rederive lanes`

| File | Half |
|---|---|
| `_webref/spec_labels.py` | **split** — region map below |
| `skills/elidex-plan-review/preflight.py` | **A** — drops the local map, imports the shared one |
| `_webref/commands/coverage_map.py` | **split** — the delegation to `label_for` is A; the changed last-resort is B |
| `_webref/cli.py` | **split** — the blurb derivation is A; the `cite-audit` subparser and its import are B |
| `_webref/DESIGN.md` | **split** — the `spec_labels.py` bullet is A minus its catalog sentence; the rest is B |
| `_webref/test_cite_audit.py` | **split** — `TestSharedSpecLabelMap`'s first **8** tests become A's `test_spec_labels.py`; `test_coverage_map_fallback_round_trips` + the `coverage_map_label` helper are **B** (they assert the catalog round-trip); the other 10 classes are B's |
| `_webref/commands/cite_audit.py`, `sources/webref_data.py` | **B** |

**`spec_labels.py`, by region** — named by content, not line number. → `rederive regions`

| Region | Half |
|---|---|
| docstring drift rationale **and its closing `"""`** | **A** ("Four sites" → **three**; the `cite_audit.py` bullet is B's) |
| the *"`SPECS` is a fallback, not the source"* paragraph | **B** |
| `SPECS` and the three derived dicts | **A**, **minus the parse aliases** (below) |
| `_catalog()` and its `sources.webref_data` import | **B** |
| `label_for` / `shortname_for` — the pinned lookups **A**; their catalog branches | **B** |

**A deletes the 8 parse aliases.** Measured: removing every one leaves `LABEL_TO_SHORTNAME` **byte-identical
at 24 keys**, because each alias (`HTML`, `DOM`, `URL`, `Fetch`, `Streams`, `WebCrypto`, `XHR`, `WebIDL`)
lowercases to its own shortname, which the comprehension already supplies via `entry[0]`. They are dead
weight in a file A creates, and draft 6 built §7's entire debt argument on widening them. The 9
newly-resolving spellings come from the **shortname-as-own-parse-key** rule instead. → `rederive keysets`

`coverage_map._spec_label` keeps `origin/main`'s last-resort `shortname.upper().replace("-", " ")`
**verbatim**; the branch's `or shortname` is only correct together with the catalog and B §4.1.8's rules.

⚠ **The prose needs its own pass.** Sites in the A column describe `commands/cite_audit.py` as extant, and it
is **absent from `origin/main`**. A filename-only purity check passes while every one is present, which is
why §12 (3) carries content assertions. → `rederive couplings`

Result: `webref-cite-audit-tool` = `origin/main` + the A column + A's edits; B's branch = A's landed head +
the B column. **B's memo did not describe that base**; §13 records the edits A applied to it.

**Why A takes the map's pinned half rather than leaving the whole carve to B**: the import is what *creates*
the failable capability. If B lands it, `main` carries a fail-open plan-review gate — a gate every lane runs
— for the duration of B.

### §4.1 Slice routing

**The criterion, stated once so the rows are not answering it differently** (round 6 found §4.1 splitting one
concern by file while justifying by nature): **a gate's contract of record travels with the gate**; a
*review-axis* requirement is C's. So `SKILL.md`, which documents what `preflight` accepts, moves with
`preflight`; `axes.md`'s Axis 4 detect is a review requirement and is C's.

| Concern | Slice | Why |
|---|---|---|
| the catalog fall-through, the discriminated `_catalog()`, the reverse index and the round-trip rules | **B** | §10-Q1's boundary. A lands the map's *shape*; B owns its lookup semantics — and the fall-through **is** lookup semantics |
| `coverage_map`'s changed last-resort, and `test_coverage_map_fallback_round_trips` | **B** | true only once the catalog and B §4.1.8's rules are in |
| `cite_audit.py`, `test_cite_audit.py`, the `cite-audit` subparser, `webref_data.py`'s memo | **B** | the detector |
| `spec_labels`'s public-surface reduction (`project_pr-a0-review-ledger` #25) | **B** | its stated root is `cite_audit.py` indexing `LABEL_TO_SHORTNAME` directly; reducing the surface in A would also trip the shipped `test_module_leaves_no_temporaries_to_delete` guard |
| `SKILL.md` — the Hard-fail bullet, the Soft-warn bullet (it lists "unrecognized spec labels" as unconditionally exit-0, the sentence J1 is about), `--no-verify`'s meaning, Pre-condition #1 | **A** | the criterion above |
| one shared `SECTION_NUMBER_RE` across `preflight` / `cite_audit` / `section_sort` | **B** | A leaves `preflight.SECTION_REF_RE` byte-identical so B's collapse is one edit |
| `axes.md` (2)/(4) and its Axis 4 spec-citation-table detect, which a no-spec-surface memo will draw; `CLAUDE.md` § "Spec citation"; `DESIGN.md`'s reported-class contract | **C** | review-axis requirements |
| `grep_pass.py` reporting a wrong repo root as one HARD finding *per referenced path* | **C** | §1's class in a neighbouring gate; C already edits review tooling. Draft 5 recorded it with no home at all |
| the `crates/**` citation repairs and the 8 newly-authored wrong citations | **D** | content, not plumbing |

### §4.2 A1 — land the capability fail-closed

#### §4.2.1 The measured asymmetry — and the instrument that measures it

Removing the CLI hard-fails; removing the **new import** leaves every row unmapped, nothing verified,
**exit 0**, and a wrong-cause remedy naming the file that failed to import. **Case C does not exist on
`origin/main`** — there `shortname_from_label` reads a module-local dict with no import to fail. The
asymmetry is created by moving the map, which is why the slice that moves it owns it. → `rederive remedies`

⚠ **Drafts 1-7 measured the map axis with the wrong instrument, so every row taken with it is re-derived
here.** `mv .claude/tools/_webref` — what `rederive remedies` used — flips **neither** §5 axis.
`.claude/tools/webref` is a *separate* 16-line shim, so `WEBREF.is_file()` stays **True** while the CLI dies
`rc 1` (`ModuleNotFoundError`) at invocation: a state §5 has no row for, in which A's static verdict names
only the map and the CLI is in fact broken. The two axes are flipped by different things, and only these:

| instrument | `WEBREF.is_file()` | map import | child `webref` rc | is it a §5 state? |
|---|---|---|---|---|
| `mv .claude/tools/_webref` (drafts 1-7) | True | FAIL | **1** | **no** |
| in-process `sys.meta_path` block | True | FAIL | **0** | **yes** — the map axis (rows 6/7/8/14) |
| `mv .claude/tools/webref` (the shim) | **False** | OK | 2 | **yes** — the CLI axis (rows 3/4/5) |

The block leaves the tree on disk, so the *child* process the gate spawns still resolves — which is what
"CLI ✓ / map ✗" means and what tree removal cannot produce. → `rederive instruments`

⚠ **The same class, once more, in the fixture set.** `allunmapped.md` is all-unmapped under `origin/main`'s
15-key dict and under A's pinned map — but **not at the carve**, where `shortname_for` consults the catalog
and resolves `CSSOM VIEW` → `cssom-view-1`, verifying the row it exists to leave unverified. So the harness
pins the resolver to the pinned map for every after-A measurement. A fixture is named for a *state*, and the
state is a property of the resolver it is run against.

#### §4.2.2 The tri-state cannot live in `shortname_from_label`

Applied verbatim in the sandbox (a `TOOLS_UNAVAILABLE` sentinel returned from that function and hard-failing
in `main`), with `_webref` removed: a memo whose §3 rows carry spec labels **exits 1** ✓, and a memo whose
rows open with `§` **still exits 0** ✗. Cause is the function's first line:

```python
def shortname_from_label(label: str | None) -> str | None:
    if not label:
        return None          # ← taken before any availability check below
```

`parse_spec_cell` returns `cell[:m.start()].strip()`, so a cell beginning with `§` yields `""` and every such
row short-circuits before the capability is consulted. The gate's fail-closed property becomes **a function
of the reviewed memo's cell formatting** — J1 restated as a defect.

#### §4.2.3 The fix — two static causes, one verdict, and two independent act-sites

1. **Two causes, both static process facts, evaluated once at `main`'s top**: `WEBREF.is_file()` and
   `_shortname_for is None`. The verdict is their union.
2. **`shortname_for` stays `str | None`.** No tri-state, no `resolve_label`, no discriminated `_catalog()` —
   all of that machinery existed only to carry draft 4's dynamic third cause, which leaves with the widening.
3. **`shortname_from_label` keeps returning `None` when the map is absent**, and the row loop keeps its two
   arms. ⚠ Draft 5 made this branch raise; under `--no-verify` the hard-fail is suppressed by construction,
   so the loop still calls it and §5 row 8 becomes a traceback. Classification is the row's question, the
   verdict is the process's, answered in different places on purpose. Keeping the loop also avoids draft 2's
   regression, which took `K` 7 → 0 and `--strict-breadth` 1 → 0, silently disabling the split gate
   `SKILL.md` makes a stop-and-ask-user step.
4. **Act-site 1 — the hard fail.** At the verification stage, not `main`'s top: acting at the top would
   hard-fail a no-spec-surface memo, which §4.2.5 forbids. The existing trigger is insufficient —
   `if not args.no_verify and citations:` never fires when the map is absent, because every row goes UNKNOWN
   and `citations` is empty, which *is* §4.2.1's case C. So it widens to
   **`not args.no_verify and (citations or (unavailable and data_rows))`**. Unavailable + verification
   requested → HARD FAIL naming each absent cause and `--no-verify` as the suppressor; unavailable +
   `--no-verify` → exit 0 (J3, measured rows 5/8). On a no-spec-surface memo the third arm is not merely
   False — §4.2.5's path **returns before `data_rows` is computed at all**, so the arm is unreachable, which
   is the property §4.2.5 actually needs and a weaker claim than draft 7's. Measured rows 12/14/x3.
5. **Act-site 2 — the reporting arm, whose guard is the capability verdict and NOT the stage's entry
   predicate.** This is the clause that inverted in draft 6 and again in draft 7. It is now **measured**:
   `rederive armmatrix` runs three candidates over 24 states. Required truth set = **exactly rows 11 and
   11b** (capabilities present, no row resolvable).

   | candidate | expression | measured True in | verdict |
   |---|---|---|---|
   | draft 6 | routed through item 4's predicate | — (never, incl. 11) | red, found by 2 axes |
   | draft 7 | `not no_verify and data_rows and not seen_pairs` | 3, 4, 6, 7, 9, **11, 11b**, x1, x2, x4 | **8 false positives** |
   | *flag* | `verify_ran` set where the loop is entered | **nothing, incl. 11 and 11b** | **red** |
   | **A ships** | `not no_verify and data_rows and not unavailable and not seen_pairs` | **11, 11b only** | ✓ |

   Two things the table settles that no amount of prose did. **(a)** Draft 7's arm is not merely noisy: in
   **row 3** it prints `n/a (0 of 2 rows resolvable)` when **2 of 2 rows resolved** and only the CLI was
   missing — a false statement about the memo under review, which is §1's own failure shape. **(b)** The
   obvious repair — a flag set when the verify loop is entered — is **worse**, and for a structural reason
   worth stating once: *any* flag set inside the verification stage inherits item 4's entry predicate, which
   is False in exactly the row the reporting arm exists for. **The guard must be the process-level verdict
   (`unavailable`), never the stage's entry condition.** That is §1's corollary applied to the reporting
   layer, and it is why drafts 6 and 7 failed in opposite directions from the same mistake.

   The line is `citation verify: n/a (0 of N rows resolvable)`, **N = `len(data_rows)`**, malformed rows
   included, because `malformed_hard_fail` is decided separately and the reader is being told what the
   denominator was. → `rederive armmatrix`
6. **Every other summary line states its basis too.** The breadth line reads
   `K=<n> (<u> of <N> counted by label spelling)` whenever `unmapped_rows > 0` — draft 6's
   "(unresolved — counted by label spelling)" misdescribes the *partial* case, where `unique_specs` mixes
   shortname and label keys. The soft-warn remedy stops naming `SPEC_LABEL_REVERSE`, a symbol A deletes.
7. **The per-row soft-warn is suppressed when the capability is absent.** ⚠ Measured: with the map absent
   the existing `if unrecognized_labels:` block still prints *"add the spec to …`::SPECS`"* — remedy 1
   co-printing with remedy 3's case, i.e. the founding wrong-cause defect of §1 item 1 surviving A's own fix.
   Draft 6's P5 asserted each remedy appears "for its own cause and no other" and would have been red.
   → `rederive remedies`
7b. **The row loop must partition the unmapped bucket, or remedies 1 and 2 cannot be per-cause.**
   ⚠ **Unstated through draft 7, and it makes §4.2.4's table unimplementable.** `origin/main` appends
   `label or "<empty>"` to **one** list, so a label-less row and an unknown-label row are indistinguishable
   downstream and any `if unrecognized_labels:` block fires remedy 1 at both. A splits them at the point of
   classification — `unrecognized_labels` keeps only *labelled-but-unknown*, a separate `labelless_rows`
   counts the rest. Measured with the split: row 11 prints **remedy 1 only**, row 11b **remedy 2 only**, and
   P5's "and no other" becomes satisfiable. → `rederive armmatrix`
7c. **J1 binds the reporting layer too, not only the classification.** ⚠ Also unstated through draft 7, and
   it is J1's own words turned on A's own output: *a row is unmapped only if the mapper ran and declined*.
   With the capability absent the mapper never ran, yet the summary still prints `unmapped-label rows: 2`
   (measured, row 6) — a datum the process could not establish, which is precisely what item 1 forbids one
   return value from carrying. Under item 6's standard the counter states its basis: when `unavailable`, the
   line reads **`unclassified rows: <n>  (label map unavailable)`**. Item 3 keeps the loop's two arms for
   *control flow*; this is about what the arms are then allowed to assert.
8. No third key space is introduced, so `K` and the spec list it prints cannot disagree.

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is re-tested inside
`verify_citation` on **every unique citation**, reporting one process-level fact as *n* per-citation
failures. After the hoist the exit code is unchanged and the diagnostic is one line. The guard inside
`verify_citation` becomes an **explicit raise**, not an `assert`: under `python3 -O` an assert is stripped
and a direct caller would get exactly the silent non-zero this change removes.

#### §4.2.4 The remedy text

**Four** strings, currently one. ⚠ Remedy 3 says "the import error", which the guard
`except Exception: _shortname_for = None` **discards** — so A must capture it (`_shortname_for_error`)
alongside the sentinel, or the string cannot be produced. Draft 6 asserted the remedy without a write-path.

⚠ **And the capture must be initialised *before* the `try`, or it goes stale across a reload.** Round 7's
finding, measured: a module global assigned only in the `except` arm keeps its previous value when a later
`importlib.reload` **succeeds** — the arm simply does not run. §4.5 item 2 reloads between tests, so a
map-absent pin would poison every later pin's remedy text, method-order-dependently. One line fixes it
(`_shortname_for_error: Exception | None = None` above the `try`), and it is stated here because the
symmetric-looking `_shortname_for = None` **is** re-established on reload, which is what makes the asymmetry
easy to miss. → `rederive reloadstale`

| Condition | Remedy |
|---|---|
| genuinely unmapped label | "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the label spelling" |
| **label-less cell** | "the Spec section cell must open with a spec label" — today this row prints the `SPECS` advice against `<empty>`, advice that cannot be acted on |
| tools unavailable | the captured import error and the path attempted, plus `--no-verify` |
| CLI missing | the expected path, plus `--no-verify` |

#### §4.2.5 A5 — let a slice declare that it has no spec surface

A slice implementing no spec logic must today author fixture citations and then receives `citation verify:
ok` as its headline — §1's anchor, in A's own file. Draft 2 routed the fix to B (the umbrella forbids B
editing review policy), draft 3 to C; §4.1's criterion settles it as A's, because it is the gate's own
contract.

- **Accepted shape**: the `## §3. Spec coverage map` heading stays **required**; its body may carry one
  marker line in place of a table.
- **Recognition** — the three properties `find_coverage_map_section` and `find_table` already thread, because
  anything weaker turns the marker into the silent bypass this section argues it is not: **line-anchored**
  (first non-whitespace content is the literal `**No spec surface**`), **fence-aware** (`fence_state`-gated),
  **§3-scoped** (between `body_start` and `body_end`). `fenced-marker.md` pins the second.
- **Hard-fail on ambiguity**: marker **and** a table, with or without data rows; or the marker twice. ⚠ §5
  rows 12b and 13 are **one code path**, not two: `find_table` returns non-`None` for a header-only table,
  so `table is not None` covers both and one diagnostic serves both. P11b's two fixtures pin one branch
  against two inputs; draft 7 read the two rows as two behaviours. Measured.
- **The marker suppresses citation verification, not grep-pass.** ⚠ **Unstated through draft 7**, and it
  decides an edit: a slice with no *spec* surface still has §4-§7 structural references, so the
  no-spec-surface path must reach the same grep-pass stage the table path does. Since that path returns
  early, grep-pass moves into a `grep_pass_stage(args, plan_path) -> bool` called from both — the one
  structural change §4.2.5 forces on `main` beyond the branch itself.
- **Verdict**: `citation verify: n/a (no spec surface declared)` and `breadth: n/a (no spec surface
  declared)` — not `ok`, not `0`. ⚠ **The summary is *reduced*, not merely re-worded.** Round 7 found draft 7
  implying the marker path prints the usual block while skipping the writer for ~12 of the variables in it.
  The resolution is that the path branches **before the data loop**, so those variables have no value to
  print and none is printed: the marker verdict is the heading line plus the two `n/a` lines, full stop.
  That is also what makes item 4's third arm unreachable rather than merely False.
- **Capability interaction**: with the capability absent the verdict cannot hard-fail here (`data_rows` is
  empty, so item 4's third arm cannot fire) — but the printed line **names the absent capability** rather
  than reusing the plain string, so a run that *could not have verified* is distinguishable from one that
  *had nothing to verify*. Draft 6 collapsed both onto one string, against item 6's own standard.
- **Residual, stated rather than argued away**: unlike `--no-verify`, an *invoker* decision visible in the
  command every reviewer types, the marker lives in the *artifact*, so one author's edit suppresses
  verification for every later reviewer. Mitigations: the ambiguity hard-fail is mechanical; the census
  (`rederive marker`) implements the same three recognition properties rather than a looser grep; the gate
  prints `n/a`, not `ok`; Axis 4 reads the memo regardless. §10-Q4 puts the residual to review.

### §4.3 A2 — give the suites a scheduler

#### §4.3.1 The hole

`ci.yml`'s `changes` filter has two sets, `rust` and `config`; **`.claude/**` is in neither**, and all three
jobs are gated on one of the two. `ci.yml` never invokes `mise`. `codeql.yml` analyses `[actions, rust]` on
push + a weekly cron, with no `pull_request` trigger; `audit.yml` is `cargo audit` on a cron. ⇒ a
`.claude/**`-only pull request triggers **zero jobs**. → `rederive filters`, `rederive suites`

#### §4.3.2 The mechanism — one script, two callers (J4)

`.claude/tools/python-suites.sh`, `set -euo pipefail`, then two `discover` lines rooted at
`.claude/tools/_webref` and `.claude/skills/elidex-plan-review`. `mise.toml` gains `[tasks.tools-test]`
added to `[tasks.ci].depends`; `ci.yml` gains a `tools` path-filter set (`.claude/tools/**`,
`.claude/skills/**`, `.github/workflows/**`) and a `tools` job under the same push bypass the other jobs use.

**The script fails loudly when a `test_*.py` under `.claude/` is not collected by either `discover` root.**
⚠ Draft 6 worded this as "outside the filtered paths", which keys on the CI *filter* — strictly broader than
the two roots, so a suite at `.claude/skills/elidex-review/` would be inside the filter, outside `discover`,
and pass. The set the assertion ranges over is `git ls-files '.claude/**/test_*.py'`. → `rederive suiteset`

#### §4.3.3 The network question — answered by construction

Measured: **0 `urlopen` calls** across all `origin/main` tests. A's 8 moved tests exercise the pinned dicts,
`coverage_map._spec_label` and `preflight.shortname_from_label`; under §4.0's split none reaches
`sources/webref_data`, because `spec_labels.py` no longer imports it. → `rederive suites`

⚠ **Draft 5's P9 could not have detected a violation, and draft 6's baseline used the same blind
instrument.** `verify_citation` runs `subprocess.run([sys.executable, WEBREF, …])` and `urlopen` is called
only inside `cache.py`, in the **child**; a parent-process patch cannot see it. **T-net** (§6) is specified
at the level the fetch happens, and the `origin/main` baseline above is stated with its limit: it measures
parent-process calls, and what the 47 tests do in child processes is invisible to it. That limit is
acceptable only because those suites spawn no `webref` child — which **T-net**'s first assertion is what
actually establishes.

Draft 4 measured one fetch per run, had to argue it acceptable in `mise run ci` — CLAUDE.md's *mandatory*
pre-push gate — and opened a deferral plus an umbrella obligation. All three disappear. What replaces them
is one forward-binding constraint A adds to the umbrella's "Constraints each slice inherits":

> **No slice may make label resolution require the network without shipping its offline degradation in the
> same slice.** Slice B introduces the catalog fall-through and therefore owns the offline contract for it.

⚠ **What A does *not* claim**: that the gate becomes offline-capable. It is not, and was not —
`verify_citation` shells out to `webref heading`, which issues a conditional GET, and `cache.py` `sys.exit`s
on `URLError`, so **`origin/main`'s gate already requires the network in default mode**. A's claim is exact:
*A adds no network requirement that was not already there*, and the `--no-verify` degradation stays clean.

#### §4.3.4 What "enforced" can honestly mean here

`main` is governed by an **active** ruleset whose rules are `deletion` / `non_fast_forward` /
`pull_request`. There is **no `required_status_checks` rule**, so a red `tools` job does not block a merge;
CLAUDE.md's "CI 全 pass を目視確認してから squash merge" is the blocking step, and it is human. (The 404 from
`…/branches/main/protection` is the **deprecated legacy endpoint** and means "not protected via the legacy
API", not "unprotected".) The claim A may make: the job makes a regression **visible, attributed, and on the
PR page at review time**. → `rederive ruleset`

#### §4.3.5 The interpreter floor

No `.claude` Python source uses syntax newer than 3.9. `python-suites.sh` asserts
`sys.version_info >= (3, 9)` — A's own measured need — and the job echoes `python3 -VV`. B raises the floor
when B lands `(?>...)`. `SKILL.md`'s Step 0 invokes `preflight.py` directly, bypassing the script;
unaffected today, marked UNCHECKED in §6.

### §4.4 A3 — site the label-map tests where they belong

§4.0 moves `TestSharedSpecLabelMap`'s 8 A-tests into `test_spec_labels.py`. One assertion inside
`test_all_three_consumers_derive_from_specs` does not belong there: it inserts the *elidex skill's* directory
onto `sys.path` and imports `preflight` — the one **import-time executable** edge blocking `DESIGN.md`'s goal
of keeping the core movable to a standalone repository. The `preflight` half goes to `test_preflight.py` as
**P1**; `test_spec_labels.py` keeps the `coverage_map` half at module-level import. No `sys.path` mutation
survives inside any test method. ⚠ §4.3.3 therefore must not say the 8 moved tests exercise
`shortname_from_label` — that assertion leaves with P1.

### §4.5 Test-siting constraints the plan must state, not discover

1. **`_shortname_for` is bound at module import**, and `preflight.py` **re-inserts `.claude/tools` on every
   import**, so "remove it from `sys.path` and reload" re-establishes the capability the test is removing.
   Working mechanisms are a `sys.modules`/`__import__` hook plus `importlib.reload`, or a subprocess; an
   in-process `preflight._shortname_for = None` pins the precondition but leaves the `except Exception` guard
   **mutation-green**. **P2** uses the reload form, **P2b** the subprocess form.
2. **P1 needs `_shortname_for` bound; P2/P4 and the map-absent runs need it `None`** — mutually exclusive
   process-global state in one file. `tearDown` restores via `importlib.reload` under the un-patched import,
   and P1 asserts the bound state at `setUp` so a leak fails loudly. `unittest` orders methods
   alphabetically, so relying on names is not a plan.
3. **The isolation contract is four pieces of process state**: `preflight._shortname_for`, `sys.path`,
   `preflight.WEBREF` (perturbed by every CLI-absent row), and `subprocess.run` (**T-net** installs a parent
   spy on it). ⚠ Draft 6 named `verify_citation` on the ground that T-net asserts against it; T-net asserts
   against `subprocess.run`, which `importlib.reload` does **not** restore because it lives in another
   module. Draft 4's `webref_data._INDEX` and `try_fetch_data.cache_clear()` both leave with the widening,
   and the second never existed on `origin/main`.
4. **`verify_citation` is stubbed by a shared `setUp`, for every pin that runs `main`, or T-net(a) is red by
   construction.** ⚠ Round 7's second CRIT, now measured rather than argued. Draft 7 stated the stub in
   **P1b alone** while T-net(a) ranged over the whole suite. `rederive armmatrix`'s spy counts webref
   subprocesses per row: **5 calls across 4 rows** — row 1 ×2, row 2b ×1, row 10 ×1, row 15 ×1 — i.e. pins
   **P1b, P1c, P4, P10, P11d**, every one of them a `main` run in default mode with a resolvable row. Zero
   for all 20 other states. Measured with the stub installed at module level instead: the count is **0**
   while every observable assertion survives (`ok (2 unique citation(s) checked)`, `ok (1 unique …)`), which
   is what makes §12(1) attainable. `verify_citation` is the single seam between the gate and the CLI —
   preflight has exactly one `subprocess.run` call site — so the stub is complete by enumeration, not by
   hope. **No pin loses coverage**: P6's "reported once, not per citation" is about the *hoisted* verdict,
   which never enters the loop, and the `python3 -O` explicit-raise guard (§4.2.3) is pinned by calling
   `verify_citation` directly with `WEBREF` pointed at a nonexistent path — which reaches no subprocess.

---

## §5 Behaviour deltas

**Both columns are now measured, and by different harness blocks.** Baseline = `rederive column`, which
draft 8 extends to vary the CLI axis (rows 3/4/5 need it and draft 6's version never varied it). *After A* =
`rederive armmatrix`, running the grafted control flow; through draft 7 that column was **predicted by
construction**, and prediction is what inverted twice. **On `origin/main` the "map" axis does not exist** —
the map is a module-local dict with no import to fail — so those rows read `n/a`. Every row ran
`--no-grep-pass`. The two capability causes are a **union**, so any combination of absent causes yields one
verdict; what differs is the **diagnostic**.

⚠ **This table has no Pin column.** Rounds 4-6 each found §5's Pin column, §6's prose and §12(2)'s ✓-list
disagreeing — three views of one thing. §6 is now the single pin table and names the rows it covers.

| # | CLI | map | mode | §3 shape | `origin/main` | After A |
|---|---|---|---|---|---|---|
| 1 | ✓ | ✓ | default | labelled | 0, verified | **0** |
| 2 | ✓ | ✓ | `--no-verify` | labelled | 0 | **0** |
| 2b | ✓ | ✓ | default | `dedup.md` | 0, **1** unique from 2 rows | **0**, unchanged |
| 3 | ✗ | ✓ | default | labelled | 1, one failure per citation | **1** — one diagnostic line |
| 4 | ✗ | ✓ | default | label-less | **0** (`citations` empty ⇒ verify block skipped) | **1** |
| 5 | ✗ | ✓ | `--no-verify` | either | 0 | **0** — capability unused |
| 6 | ✓ | ✗ | default | labelled | n/a | **1** |
| 7 | ✓ | ✗ | default | label-less | n/a | **1** (§4.2.2) |
| 8 | ✓ | ✗ | `--no-verify` | either | n/a | **0** (J3) |
| 9 | ✗ | ✗ | default | any | n/a | **1**, diagnostic names **both** causes |
| 10 | ✓ | ✓ | default | **alias spelling** | **0**, unmapped soft-warn, **no verify line** | **0**, mapped and verified |
| 11 | ✓ | ✓ | default | **all rows unmapped** | **0**, **no `citation verify:` line at all** | **0** + `citation verify: n/a (0 of N rows resolvable)` |
| 11b | ✓ | ✓ | default | **label-less** | **0**, no verify line | **0** + the same `n/a` line, + remedy 2 |
| 12 | ✓ | ✓ | default | **marker, no table** | **1** (no-table hard-fail) | **0**, `verify: n/a` |
| 12b | ✓ | ✓ | default | **marker + header-only table** | **1** (0-data-rows hard-fail) | **1** — ambiguous declaration |
| 13 | ✓ | ✓ | default | **marker + populated table** | **0** (the marker is inert prose; the table verifies) | **1** — ambiguous declaration, *same branch as 12b* |
| 14 | ✓ | ✗ | default | **marker** | n/a | **0**, and the line names the absent capability |
| 15 | ✓ | ✓ | default | **marker quoted inside a fence** | **0** (table verifies) | **0**, unchanged — rule (b) |

**Newly-red**: 4, 6, 7, 9, 13. **1 → 0**: row 12 only, where the red was the gate rejecting a valid input
shape. **1 → 1 with a changed diagnostic**: 3, 12b. **Exit unchanged, output changed**: 10, 11, 11b, 14.

⚠ **Row 11b is new in draft 8 and was not deducible from the table it is missing from.** It is the *second*
state in which item 5's line must print, and the only one at the available end that P4 (label-shape
independence) can compare against — draft 7 tabulated the label-less shape only in capability-absent rows
(4, 7), where the hard fail dominates and the arm's correctness is untestable. `rederive armmatrix` also
runs seven further states §5 does not tabulate (x1-x7, both axes × the label-less and all-unmapped shapes ×
`--no-verify`); **none diverges between the three candidate predicates**, which is the evidence that §5's
row set plus 11b is complete for the property, rather than merely large.

---

## §6 Pins — one table

Each pin names what it **executes**; §5 owns the expected values, stated once. "Fails at the carve?" is what
§12 (2) reads — no second list.

**Two suite-level fixtures, stated here rather than inside a pin, because a per-pin clause is what made
draft 7's pin set unsatisfiable** (§4.5 item 4): a shared `setUp` stubs **`preflight.verify_citation` →
`(True, "")`** for every pin that runs `main`, and restores the four pieces of process state in `tearDown`.
The capability axes are flipped by §4.2.1's two instruments — never by removing the tools tree.

| Pin | What it executes | §5 rows | Fails at the carve? |
|---|---|---|---|
| **P1** | `shortname_from_label(label) == short` over `SPECS`, no `sys.path` mutation in the body, `setUp` asserts the module un-poisoned | — | no |
| **P1b** | `main` on `labelled.md`, default **and** `--no-verify`, both capabilities present | 1, 2 | no |
| **P1c** | `main` on `dedup.md`; asserts `1 unique citation(s) checked` from 2 rows — the dedup arm | 2b | no |
| **P2** | map unimportable via `importlib.reload` under an import hook | 6 | **yes** |
| **P2b** | the same via subprocess; mutation check — deleting the `except Exception` clause must turn P2b red, P2 alone leaves it green | 6 | **yes** |
| **P3** | `--no-verify --no-grep-pass`, **map absent** — exit 0 and the breadth-basis qualifier | 8 | **yes** |
| **P3b** | `--no-verify`, **CLI absent, map present** — exit 0, capability unused | 5 | no |
| **P4** | label-shape independence: `labelled.md` and `unlabelled.md` give the *same* exit code in every capability state | 4, 7 | **yes** |
| **P5** | each of the four remedy strings appears for its own cause **and no other** — the per-row soft-warn suppressed when the capability is absent (item 7), **and** remedy 1 vs remedy 2 separated by the partition of item 7b (row 11 → remedy 1 only, row 11b → remedy 2 only) | 3, 6, 7, 9, 11, 11b | **yes** |
| **P5b** | with the capability absent the summary reads `unclassified rows`, not `unmapped-label rows` (item 7c) | 6, 9 | **yes** |
| **P6** | CLI missing reported once, not per citation; **and** that row 9's diagnostic names both causes | 3, 9 | **yes** |
| **P10** | `main` on `alias.md`; asserts the row is MAPPED and verified | 10 | **no** — the carve already resolves it (measured) |
| **P11** | `nospec.md` → exit 0, asserting the `n/a` strings, not just the code | 12 | **yes** |
| **P11b** | `nospec-and-table.md` and `nospec-and-header.md` → exit 1 naming the ambiguity — **two fixtures, one branch** (§4.2.5) | 12b, 13 | **yes** |
| **P11e** | a no-spec-surface memo still runs grep-pass: `nospec.md` with a bad `crates/…` path → exit 1 **naming the grep-pass finding** (§4.2.5) | 12 | **yes**, on the diagnostic |
| **P11c** | `nospec.md` with the map absent → exit 0, and the line names the absent capability | 14 | **yes** |
| **P11d** | `fenced-marker.md` → the fenced quotation is **not** recognised, asserted on `find_markers(...) == []` **and** the absence of any `n/a (no spec surface…)` line — *not* on the exit code | 15 | **yes**, on those assertions |
| **P12** | `shortname_from_label` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, vendored as a literal — correct here precisely because the point is to freeze the *old* table | — | no |
| **P13** | `allunmapped.md` **and** `unlabelled.md`, default, both capabilities present → exit 0 **and** the `n/a (0 of N rows resolvable)` line present; **and its negative half** — the line absent in rows 3/6/9, which is what separates the shipped predicate from draft 7's | 11, 11b, 3, 6, 9 | **yes** |
| **P14** | `coverage_map._spec_label` over the 12 pinned shortnames **and** a non-pinned sample exercising the `.upper().replace("-", " ")` last-resort — the branch A **does** take | — | **yes** |
| **T-net** | J5 at the level the fetch happens: (a) across A's whole suite set `subprocess.run` is never called with **the `WEBREF` path** in argv — the resolved path, *not* a `"webref"` substring, because `grep_pass` also calls `subprocess.run` with author symbols in argv and this memo is full of the string; (b) `bash python-suites.sh` runs green in a child with `http_proxy`/`https_proxy` at a closed port | — | **yes** |

⚠ **An exit-code-only assertion is not a discriminator when the carve reaches the same code by another
route** — round 7's H6, generalised, and it bites exactly twice. `rederive carvecolumn` now runs all nine
fixtures (draft 7's ran three): at the carve `fenced-marker.md` **exits 0** with the table verified, because
the marker is inert prose there — identical to A's expected outcome — so P11d's "fails at the carve = yes"
was **false as written**. Same shape for P11e: the carve exits 1 on `nospec.md` already, via the no-table
hard fail. Both pins now assert on the mechanism (`find_markers`, the grep-pass diagnostic), which is what
makes §12 (2) a real red baseline rather than an accidental one. Every other "yes" re-derives from that
block's exit codes.

Sited in `test_preflight.py` except P12/P14 and `test_spec_labels.py`'s 8 moved tests. **UNCHECKED, marked
not omitted**: the interpreter floor on `SKILL.md`'s direct `preflight.py` path; that a red `tools` job
blocks a merge (**false** — §4.3.4); that `shortname_for` and `origin/main`'s `shortname_from_label` are
equivalent *functions* (`shortname_for` calls `.strip()`, the other does not — unreachable through the gate
because `parse_spec_cell` already strips, so the *gate* claim P12 makes holds and the *function* claim is
not made).

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** and **ECS-native** — not applicable; no `crates/**` diff.

**`DESIGN.md` generic-core / elidex-adapter split.** ⚠ **Draft 6's account of this was built on a string
grep and is corrected here.** It claimed `origin/main`'s generic tree carried "exactly one" elidex coupling.
That is true of the pattern `.claude/skills|elidex-plan-review` and false of the concept: measured, the
generic tree already carries **7 plan-memo / plan-review references across 2 files** (verified 2026-07-29) (`cli.py`,
`commands/coverage_map.py`) — including "plan-memo §3 skeleton", "plan-review gate" and "drop into plan-memo
§0.5". → `rederive couplings`

So the honest picture: **A adds instances to a pre-existing, already-saturated class, not a new class.**
Three consequences, and draft 6 got all three wrong:

- The **display labels** (`"WHATWG HTML"` etc.) are elidex convention and are **already** in the generic core
  on `origin/main`, in `coverage_map._SPEC_LABEL_MAP`. A moves the *reverse direction* of the same table, not
  new policy.
- The **parse aliases** were draft 6's stated debt increase. They are **inert** and A **deletes** them
  (§4.0), so the increase does not occur and there is nothing to route to B — draft 6 routed it to B on the
  ground that "B replaces this lookup wholesale", which B §4.1.8 falsifies (its rule 1 is "`SPECS` pinned map
  wins, verbatim").
- The **shortname-as-own-parse-key** rule, which is what actually produces the 9 new spellings, is a property
  of the catalog namespace rather than elidex policy: a spec's shortname is a valid way to name it in any
  repository. It is generic, and it stays.

What remains is A's own new **prose**: the docstring's skill path, the alias rationale, and the load-time
consumer list. A writes these **by role, not by path** ("the plan-review gate", not
`.claude/skills/elidex-plan-review/preflight.py`), which is what `DESIGN.md`'s closing rule asks — it forbids
elidex-specific *file paths* in generic behaviour, and permits policy in documentation. §12 (3) asserts on
that, by concept.

| Edit | Layer |
|---|---|
| `spec_labels.py`'s `SPECS` (labels + shortnames, no aliases) | **generic mechanism + pre-existing elidex display convention** |
| `commands/coverage_map.py`, `cli.py` blurb derivation, `DESIGN.md` bullet, `test_spec_labels.py` | **generic** |
| §4.2 capability verdict, remedy text, no-spec-surface declaration, `test_preflight.py`, `SKILL.md` | **elidex skill** |
| §4.3 script + `mise` task + CI job | **elidex repo infrastructure** — `.claude/tools/*.sh` is where the four trip-wire scripts already live |

⚠ Draft 2 planned to record the `mise` task, the CI job and the interpreter floor in `_webref/DESIGN.md`.
That file says the core should "stay generic enough to move to a standalone repository later"; a section
describing `mise.toml` and `ci.yml` travels with the tree at externalization and is wrong on arrival. Those
facts live in `python-suites.sh`'s header and the `mise.toml` task comment.

**One-issue-one-way**, three collapses: the label enumeration three sites → one; the suite invocation zero
canonical sites → one; the `WEBREF.is_file()` question *n*-per-citation → one verdict. The one remaining
instance of §1's class inside A's own file — `preflight` reaching `resolver.lookup_section` through a
subprocess while reaching `spec_labels` in-process — is §11's constraint.

---

## §8 Line-count budget

→ `rederive budget`. The largest file in the touch set is `preflight.py`, and it is one cohesive gate whose
seam (structure / breadth / citation / grep-pass) is already four ordered blocks. Nothing is near a split.
The memo and the re-derivation harness are printed by the same block, since draft 6's §8 omitted the largest
file the branch actually touches.

⚠ **Draft 7's "~500 → ~540" was an invented digit, and the honest measure is not line count at all.**
`wc -l` on the `armmatrix` proto comes out *shorter* than the file it grows, because the proto trims argparse
help and abbreviates diagnostics — so a line-count estimate here is noise in either direction. `budget` now
reports **statement count** (`ast`): A's edit set is roughly **statement-neutral**, because the hoisted
capability verdict deletes the per-citation `WEBREF.is_file()` re-test and §4.2.5's branch replaces work
rather than adding it. The block prints its own caveat (collapsed diagnostics make the shipped delta somewhat
larger). The load-bearing claim survives either measure: **A restructures a 499-line file; it does not grow
one toward the threshold.**

⚠ **This memo is 925 lines and is itself the largest thing the branch carries.** It is not split, and the
reason is the discipline's own cohesion test rather than an exemption: a plan-memo's sections are one
decision surface, and CLAUDE.md's *one-issue-one-way* is the rule that would be violated by splitting it —
§4.2.3 and §5 and §6 are three views of one control flow, and rounds 4-6 each produced a defect from those
views drifting apart while they were in the *same* file. The re-derivation harness is the split that
actually applies here, and it has been taken: every executable claim now lives in
`…-A-rederive.sh` rather than in prose beside the decision.

---

## §9 Edge-dense assessment

CLAUDE.md's **base case** has two conjuncts, and draft 6 argued only the first. ⚠ Round 6's finding.

**(i) An approved umbrella's per-PR slice, explicitly.** The umbrella exists, names A, and is the approval
boundary. ⚠ One qualification A must not hide: the umbrella's Slice A **Scope** cell does not mention the
spec-label map, the gate-contract change, or §4.2.3 item 5. §13 has A amend it — a slice amending the
boundary it shelters under. The amendment is a *correction to match A's plan-reviewed scope*, not a
widening, and it lands in the same commit as this memo so the boundary and the slice are reviewed together.

**(ii) Scope narrowed to a single invariant-axis intersection.** J1/J2/J3 are one intersection — they live in
one function's control flow with one primary observable (an exit code) and one secondary (the summary's
lines), and §5 publishes the outcome-distinct rows with a pin apiece. J4 (three files of configuration) and
J5 (one offline run) are **independent surfaces, not additional intersections**, which is what the conjunct
is about: an independent surface adds review area, not edge density. The gate-contract change (§4.2.5) is
additive — one input shape, five rows, five pins. So the conjunct holds on the reading that matters, and the
memo now says which reading.

⚠ **Draft 8's harness is evidence *for* the conjunct, not against it.** Three of §4.2.3's items changed
under measurement, which could be read as "the intersection is denser than claimed". It is not: all three
changes live in the same function's control flow with the same two observables §5 already publishes, and the
harness enumerates that space **exhaustively** — 24 states, four capability combinations × the fixture
shapes × both modes — with no predicate divergence outside it. An axis whose state space can be enumerated
and run in one command is bounded, which is what the conjunct asks for. What the harness removes is the
review-round-as-interpreter cost that made this intersection *look* unbounded across rounds 5-7.

**Draft 5 removed a capability cause, not an invariant axis.** Draft 4's third cause was dynamic and forced
the aggregated verdict, the tri-state and five pins. Dropping it takes *causes* three → two; J1/J2/J3 remain
three and remain coupled, exactly as §2 says. Draft 6's "the intersecting axes drop from three to two"
conflated the two counts.

`git diff --stat -- crates/` is empty and stays empty, so a regression degrades a developer tool and cannot
reach a page, a script, or a user.

---

## §10 Open questions

Decided rather than listed, each having one live option ([[feedback_no-low-value-choices]]): the
`verify_citation` guard is an **explicit raise**; the re-carve is **its own commit, first on A's branch**;
the floor is **3.9**; the `tools` filter stays **broad** with the script failing loudly on an uncollected
suite; and draft 4's Q4 (`K`'s semantics) is answered by §4.2.3 item 6 — `K` states its own basis.

- **Q1 — is the boundary drawn in the right place?** A keeps the *dedup*, B takes the *widening*. Grounds, in
  order of weight. **(a) Correctness, measured**: `shortname_for("CSS Text 3")` with `urlopen` raising gives
  `SystemExit ESCAPED _catalog()` — `cache.py` calls `sys.exit` and `SystemExit` is a `BaseException`, so
  `_catalog()`'s `except Exception` cannot catch it. Landing the widening as carved puts a resolver that
  `sys.exit`s offline into the gate every lane runs *and*, via `[tasks.ci].depends`, into `mise run ci`.
  Hardening a gate's failure semantics on top of that resolver is strictly worse than not landing it, and the
  fix is B §4.1.7's. → `rederive offline` **(b)** A's own §4.1 assigns lookup semantics to B, and the
  fall-through *is* lookup semantics. **(c)** The widening was the sole cause of the network dependency, the
  dynamic third cause, the tri-state, one deferral and one umbrella obligation.
  **The cost of deferring it, stated**: the in-flight c3-plan memo's 18 §3 rows (`CSSOM VIEW` ×14,
  `RESIZE OBSERVER` ×3, `INTERSECTION OBSERVER` ×1) stay soft-warned one slice longer; the widening resolves
  all three *correctly*. **Not** a ground: draft 4's "wrong document" claim, falsified in §0.
- **Q2 — does `required_status_checks` belong in this PR?** One rule on an existing active ruleset — but the
  `pull_request` rule already carries `required_approving_review_count: 0` **and** a `RepositoryRole` bypass
  with `bypass_mode: always`, so adding it leaves it author-bypassable: visibility-plus-friction, not
  enforcement. **Recommendation: register, do not implement** (§11).
- **Q3 — `#11-layoutbox-trip-wire-not-in-ci`.** Its registered trigger is "before C-4 or the next
  `.github/workflows` touch", and **A is that touch**. ⚠ Draft 6 recommended "record the trigger firing as an
  observation", which is not one of `feedback_defer_lifecycle_policy`'s five dispositions — a sixth option
  smuggled in. The trigger having fired, the indicated disposition is **promote-to-PR**, and the owner is the
  **Layout lane**, not A. A records the firing in both files that carry the slot and names the disposition;
  A does not execute another lane's slot. Whether the Layout lane instead extends A's `tools` job or adds a
  second is a real fork A creates and §13 hands over explicitly.
- **Q4 — the residual in §4.2.5.** The marker is artifact-resident. Four mitigations, one mechanical. The
  alternative is to require the marker to name its umbrella slice — checkable, but a coupling A has no other
  reason for. Put to review rather than closed.

---

## §11 Defer slots + per-PR ≤3 audit

**Zero own deferrals.** Draft 4 had two, draft 6 one; both are now **constraints, not deferrals**.

⚠ **Why the last one converted.** `cleanup-webref-preflight-inprocess-resolution` recorded that
`verify_citation` forks a subprocess *and* an HTTP conditional-GET per citation while the same file reaches
`spec_labels` in-process — two ways to reach one library. Round 6's finding: the cap policy admits **three**
categories (別 milestone依存 / 別 slot依存 / genuinely 200+ LoC) and this fits none — the fix is ~15 lines,
and the trigger is a *PR slice*, not a registered parent slot. The policy's instruction when no category fits
is to fold in or narrow scope. Folding in would decide B's offline policy by side effect, which is the
failure §4.2.3 exists to stop. So it becomes the same shape §4.3.3 already uses: a forward-binding umbrella
constraint that binds B at authoring time rather than a ledger entry nobody is obliged to act on.

> **The plan-review gate reaches its shared library one way.** Slice B, which lands the offline contract,
> collapses `verify_citation`'s subprocess onto the in-process resolver in the same slice.

**Pre-existing category** (not an own deferral, not counted):
**`cleanup-elidex-ci-required-status-checks`** — the ruleset has no `required_status_checks` rule, so every
CI job is advisory. A neither creates nor worsens it. ⚠ The cost is not "one rule": the bypass actor above
makes the rule alone author-bypassable. **Trigger**: the Layout lane wiring the trip-wires, or the first job
stable enough to require. **Re-eval**: 2026-11-30. **Confidence**: Medium. ⚠ Its create-time audit scores
zero of four (`feedback_defer-slot-eligibility-audit-at-create`), which by that file's own rule makes it a
**maintenance note, not a slot** — recorded here as such rather than registered, and merged into
§10-Q3's hand-off, which it overlaps.

**Explicitly NOT deferred**: the re-carve, the two-cause verdict and both act-sites, the four remedy strings
and the soft-warn suppression, the no-spec-surface verdict and its recognition rule, the verify-line silence,
the test relocation, the siting constraints, the script + task + job, `SKILL.md`'s contract, the edits
applied to B's memo, and the umbrella's three constraint lines.

---

## §12 Exit criterion

**(1) Green:** `mise run tools-test`

**(2) Red:** build a worktree at the **detector carve** (§4.0's first subject), copy in `test_preflight.py`,
`test_spec_labels.py` and `python-suites.sh` — draft 6 omitted the third, which the carve does not have —
and run it. Non-zero, with at least one failure attributable to **every pin whose §6 row says "yes"**. No
second list: §6's column is the criterion.

**(3) A carries no part of B — filenames, prose, and layer**, ranged from the **Step-0 re-carve** (§4.0's
second subject):

```sh
git diff --name-only <step-0>..HEAD -- .claude/tools/_webref/              # only §4.0's A column
git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/       # → empty
git grep -n '_catalog\|webref_data' -- .claude/tools/_webref/spec_labels.py  # → empty
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh couplings           # → no elidex file PATH in A's half
```

The fourth replaces draft 6's `git grep -c`, which emitted per-file line counts, could not express a total,
and saw one of the three couplings it was offered as the check for.

**(4) The branch carries A's memo, the re-derivation script and the umbrella.** Sequence, because draft 6's
§13 and §12(4) were mutually unsatisfiable: **(i)** A applies B's edits on this branch; **(ii)** B's branch is
cut from this head, carrying the corrected file; **(iii)** B's and C's memos are dropped from A's branch.
Step (iii) is what makes (4) true, and it must follow (ii), not precede it.

**(5) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page; today the same observation yields zero jobs.

---

## §13 Coordination

**Every lane fact drifts by nature and is re-derived at landing** → `rederive lanes`. Draft 5 called its PR
list "complete" and was falsified within thirteen minutes; `origin/main` moved four times during round 6.

| Lane | Overlap | Ordering |
|---|---|---|
| **Slice B** | total by construction — B branches from A's landed head and takes §4.0's B column | **A first**; B's memo edits applied (below) |
| **Slice C** | `_webref/DESIGN.md`, disjoint sections. Inherits the no-spec-surface declaration as its first real consumer, plus `axes.md`'s Axis 4 detect and `grep_pass.py`'s per-path finding | after B |
| **PR-A0 / D** | its carve-provenance claim is **false at head** because A rebased and it did not — rebase it | after A/B/C |
| **the open `actions/checkout` bump** | **actual `ci.yml` contention** — same `steps:` blocks §4.3.2 extends, and after A it triggers the `tools` job | whichever lands second adapts |
| **anything touching `.claude/skills/**` or `.claude/tools/**`** | after A these trigger the `tools` job. A *class*, not a list | none |

**Owed to B's memo, applied at Step 0 alongside the re-carve** (draft 5 asserted B needed no edit; draft 6
enumerated seven items from *reading*, of which review found four wrong or incomplete — so these are stated
as **classes to grep**, not a list to read off → `rederive bmemo`): the present-tense "extant defect" framing and `spec_labels.py:` anchors in §4.1.2 /
§4.1.7 / §4.1.8 / §4.6.1; §4.1.8's falsified consequence sentence; the four sites claiming
`test_spec_labels.py` is new; B's P4/P5 colliding with A's in the same file; §8's line-count column measured
at a base where two of its files do not exist; the swapped `Slice A §4.1` / `A §4.2` references;
`coverage_map`'s changed last-resort cited as pre-existing; §4.2's seam list, which must name the widening as
a third seam; §0.1's stale provenance paragraph; and §11's cap-rule restatement, which becomes a pointer.

**Handed to Slice B**: B restates §4.1.8's consequence on the 8 cross-series cases or drops it (§0).
**Handed to the Layout lane**: the trip-wire slot's trigger has fired (§10-Q3), and A's `tools` job creates a
fork — extend it, or add a second job.

**Landing checklist**

1. Re-run `preflight.py` from each worktree that authors a plan-memo → `rederive lanes` derives the list;
   the umbrella's own bullet names a different set and is corrected in item 3.
2. Add the umbrella's **three** constraint lines: the cap naming/counting rule (own-vs-pre-existing, not the
   `cleanup-*` prefix); no-network-without-offline-degradation (§4.3.3); one-way-to-the-shared-library
   (§11). ⚠ The first is a *pointer* to `feedback_defer_cap_policy`, which declares itself SSoT for that
   distinction — not a second statement of it.
3. Correct the "10 in-flight memos in `elidex-wt-c3-plan`" claim at **all four sites** (umbrella, `MEMORY.md`,
   `project_citation-hygiene-program.md` ×2); true figure **1**. Same edit: the umbrella's branch-measured
   suite figures, its live-network-dependency paragraph (§4.3.3 measures zero), and its Slice A/B **Scope**
   cells, which record no owner for the spec-label map or the gate-contract change (§9).
   → `rederive staleclaims`
4. Update `project_citation-hygiene-program.md`, the program's cross-session SoT — including its record of
   the "wrong document" consequence §0 falsifies, which draft 6's checklist missed.
5. Correct the two live stale `d3173bed` strings, and `MEMORY.md`'s L3 bullet, which carries two
   non-ancestor shas and a superseded next-action beyond the A-landed/B-next flip.
   → `rederive staleclaims`
6. PR description: §4.3.3 (A adds no network; the gate's pre-existing requirement), §4.3.4 (no
   `required_status_checks` and the bypass actor), §0.1 item 2, and the `tools`-filter collateral class.

---

## §14 Review-round index

Seven rounds; every live correction is stated once, inline, at the section that acts on it.

**R1 → d2.** Evidence measured on the branch, not `origin/main`. **R2 → d3.** The fix opened a new failure
and disabled a neighbouring gate. **R3 → d4.** Section-contradicts-section; `K` not capability-independent.
**R4 → d5.** *The slice boundary, three drafts in.* **R5 → d6.** Claims about self and others false;
coordinates rot; the CRIT-class item-6 error. **R6 → d7** (1 CRIT / 46 IMP / 43 MIN / 10 FP): **G1** the
print trigger routed through a predicate that is False in the one row it exists for — found by two axes
independently · **G2** *"A changes no resolution outcome"* false, and draft 6's replacement attributed the
delta to an **inert** alias list · **G3** the B-edit list, written from reading, wrong or incomplete at most
items · **G4** four pins that could not check what they claimed, incl. row 10's ✓ (the carve already resolves
aliases) and P5's "and no other" (the per-row soft-warn survives) · **G5** the marker's recognition rule
unstated on anchoring/fences/scope; row 13's `origin/main` value wrong · **G6** the executable-described-in-
prose class — §15's blocks — answered by the committed script.

**R7 → d8** (Axis 2 alone: 2 CRIT / 7 IMP / 4 MIN / 0 FP; draft 7 is structural, so R7 is a full re-review
from Step 1): **H1** the fix to R6's CRIT **inverted it** — draft 7's reporting arm is True in six rows where
it must be False, incl. one printing `0 of 2 rows resolvable` about a memo whose 2 of 2 rows resolved ·
**H2** the pin set is **mutually unsatisfiable** — T-net(a) ranges over the whole suite while only P1b states
the stub, and four other pins reach `subprocess.run` · **H3** the capability instrument is wrong: tree
removal leaves `WEBREF.is_file()` True, so the diagnostics P5/P6 pin were never in the state they claim ·
**H4** §4.2.5's marker path skips the writer for ~12 variables it then prints · **H5** remedy 3's
`_shortname_for_error` is not cleared by `importlib.reload` · **H6** P11d's "fails at the carve = yes" is
false (measured exit 0) · **H7** remedy 2 has no §5 row.

**What R7 changed about the method, not just the content.** H1 is R6's CRIT returning with the sign
flipped, and H3 means the evidence for several rows was taken in a state that does not exist. Both are
symptoms of one thing: §4.2.3 governs code that does not exist yet, so **prose was the only medium and a
review round was the only interpreter**. Draft 8's answer is `rederive armmatrix` (§0). It confirmed H1,
falsified the obvious repair (a `verify_ran` flag — measured True in *zero* states, including the two it
exists for), and surfaced three edits no round had proposed: items 7b, 7c and §4.2.5's grep-pass sentence.

**Re-derived and rejected**: r1's "`coverage_map_label` has more than one caller"; r2's
"`elidex-wt-c4fix/docs/plans/` is empty" (the substance held); r4's decisive-finding *consequence*,
falsified (§0); r5's "`python-suites.sh` carrying skill paths is a generic-core violation" (repo
infrastructure) and "the temporaries guard becomes vacuous in A" (equally subject-less before and after);
r6's four dry-run gaps of my own, of which the reviewer refuted three on their witnesses.

---

## §15 Re-derivation

`docs/plans/2026-07-citation-hygiene-A-rederive.sh`, on this branch. One function per quantity; the §6
fixture bodies live there so reviewer and test read byte-identical files.

```sh
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all      # or one name
```

`citations partition keysets column carvecolumn instruments remedies reloadstale armmatrix suites anchors
regions offline couplings suiteset marker budget filters ruleset timing lanes bmemo staleclaims`

**Draft 8's four additions, and what each stopped being an argument about.** `instruments` — the three
candidate capability instruments on all three signals (§4.2.1); drafts 1-7 used one that flips neither axis.
`armmatrix` — §4.2.3 and §4.2.5 grafted onto a copy of `preflight.py` in a scratch worktree, run over 24
states with three candidate predicates side by side (§4.2.3 item 5); this is the one that ends the
prose-as-control-flow class. `reloadstale` — the except-arm/reload asymmetry behind §4.2.4. And `column` and
`carvecolumn` now vary the axis and the fixture set their claims range over, rather than a sample of them.
