# Plan — Slice A: one spec-label map, landed fail-closed, with a scheduler that runs its suites

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A**. Under that umbrella's approval
boundary this is a **terminal unit** — not re-split for touching the same subsystem as B/C.
**Branch**: `webref-cite-audit-tool`, after the §4.0 re-carve. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`.
**Nature**: developer tooling + CI topology + one plan-memo authoring-contract change (§4.2.5). Zero
`crates/**` diff. **Status**: plan-memo, **draft 6**. `/elidex-plan-review` **required before implementation**.

**Base, lane state and every quantity in this memo are re-derived by §15, not asserted here.** Five review
rounds have produced a stale-coordinate finding four times running (§14: C1, D-e, F7, and round 5's whole
G6 cluster). Round 5's proximate cause was that `origin/main` moved twice in the thirteen minutes before
draft 5 was committed; its *structural* cause is that the memo carried coordinates — shas, line numbers, PR
lists — that rot faster than a draft cycle. Draft 6 stops carrying them: `origin/main` is cited by **symbol
plus the command that locates it**, the re-carve commit by **subject, never sha** (it has moved three
times, each under a rebase this memo required), and every count lives in §15.

### §0.1 What Slice A is

`origin/main` carries the same enumeration three times — `preflight.SPEC_LABEL_REVERSE` (15 keys),
`coverage_map._SPEC_LABEL_MAP` (12) and `cli.COMMON_SHORTNAMES` (a help block). Slice A collapses them onto
one `.claude/tools/_webref/spec_labels.py`, and **because that import is the first thing that can make the
plan-review gate's label resolution *fail*, lands it fail-closed from the start** — then gives the
resulting suites a scheduler, because today nothing runs them. Three things the one-line summary must not
hide:

1. **A's spec *set* is unchanged (the same 12 pinned specs), but 9 additional *spellings* of those specs
   resolve** — `fetch`, `xhr`, `webidl`, `streams`, `webcrypto`, `ecma262`, `ecma402`, `selectors-4`,
   `geometry-1`. Draft 5 claimed "A changes no resolution outcome"; that was false (§5 row 10).
2. **A changes the plan-memo authoring contract** (§4.2.5): a §3 section may declare no spec surface, which
   edits `.claude/skills/elidex-plan-review/SKILL.md` Pre-condition #1 — a repo-wide contract every lane's
   memos are written against.
3. A ships **no detector** (B) and **retires no review policy** (C).

⚠ **Draft 5's scope change stands; its stated reason did not.** A takes the deduplication; the 948-entry
catalog fall-through goes to Slice B, which owns the lookup semantics that make it correct. The reason is
§10-Q1 and is now the measured one. Draft 4's reason — inherited from B §4.1.8 — was that a catalog
level-collision makes verification *"silently run against the wrong document"*. Re-derived (§15 block 2):
**every level-collision pair B names returns byte-identical heading data**, because webref's `ed/` extracts
are keyed to the series' current spec. Of 203 non-round-tripping shortnames, **195 resolve to the same
document and 8 to a different one** — all cross-series or fork cases. The identity criterion is *what
`webref heading` returns*, stated because it is the one under which 195/8 holds; by catalog `shortname`
field the split is 190/13. Two of the 8 (`DOM-Level-2-Style`, `DOM-Style`) carry the label `DOM`, which
elidex memos write constantly — they are harmless not because the label is exotic but because **the pinned
map wins before the catalog is consulted**. The ambiguity B reports is real; the danger attributed to it is
not. §13 hands the correction to B.

---

## §0.5 Spec citation table

This slice implements no spec logic. The two citations below are the rows the new `test_preflight.py`
fixture memos carry; both looked up with `.claude/tools/webref` (§15 block 1), nothing from memory.

| Cite | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| the labelled fixture row (P2/P3 — a row whose spec label maps) | HTML §4.10.21 | Constraints | `#constraints` | `heading --exact html 4.10.21` |
| the second labelled row, so `seen_pairs` dedup is exercised | HTML §4.10.21.2 | Constraint validation | `#constraint-validation` | `heading --exact html 4.10.21.2` |

**P4 needs a *separate* fixture memo, not a third row of the one above.** Its §3 rows are **all**
label-less (`| §4.10.21 Constraints | … |`, each cell opening with `§`). A memo containing *any* labelled
row hard-fails under both the correct placement and the mis-sited one, so P4 would pass vacuously. Two
fixture memos ship — `labelled.md` and `unlabelled.md`. Neither row is a citation defect; the label-less
shape is the input that falsifies §4.2.2's placement.

⚠ **This table certifies fixtures, not the slice.** `preflight` prints `citation verify: ok (2 unique
citation(s) checked)` for a slice with **zero spec surface**, because `origin/main` hard-fails a §3 section
with no heading, with no table, and with a header but no data rows — all three measured (§15 block 5). So
there is no accepted input shape that declares "no spec surface" and passes. That is §1's anchor violated
in A's own file, and **A fixes it** (§4.2.5). A's own memo cannot use the fix: `SKILL.md`'s Step 0 runs
`preflight.py` **from the worktree the memo lives in** (`REPO_ROOT` derives from `__file__`), so this memo
is currently certified by the *carve's* build — visible in its own output, which says "label not in the
shared spec-label map" where `origin/main` says "label not in `SPEC_LABEL_REVERSE`". Either way the fix is
unimplemented, so this ⚠ stands for the life of this memo. (Draft 5 said "plan-review runs against
`origin/main`'s `preflight.py`", which is false and contradicted its own §13 item 1.)

---

## §1 Ideal anchor — a gate reports on the thing it audited, or it reports on itself

Two failures, one shape. A gate's output is a claim about the artifact under review. When the gate's own
infrastructure is missing, the honest output is a claim about the **gate**, not a verdict on the artifact.

1. **Landing the shared map naively introduces exactly that inversion.** Replacing a module-local dict with
   an `import` makes label resolution *failable* for the first time, and the carve's guard
   (`except Exception: _shortname_for = None`) routes that failure into the per-row *unmapped* bucket — a
   documented soft-warn. Result: 21 of 21 rows classified as *author cited a spec I do not know*, and the
   gate **exits 0 having verified nothing** (§4.2.1, measured). The tool blames the memo for a fact about
   the tool.
2. **Nothing runs the suites.** On `origin/main` there are 47 tests across 4 files (verified 2026-07-29 by
   §15 block 3) under no `mise` task, no CI job, no hook (§4.3.1).
3. **A third instance is already live on `origin/main`, and four drafts missed it.** A memo whose §3 rows
   are *all* unmapped produces `citations == []`, so the verify block never runs and `elif seen_pairs:`
   never fires: the gate prints **no `citation verify:` line at all** and exits 0, with both capabilities
   present. Measured (§15 block 5). Not hypothetical — the in-flight `elidex-wt-c3-plan` memo's 18 rows
   take exactly this path today, and `unlabelled.md` is an instance A ships. A fixes it (§4.2.3 item 5).

The corollary that drives the edit set: **a capability is a process-level fact and must be established
once, before the data loop.** "I cannot map *this* label" is a datum about one row. "I cannot map *any*
label" is a fact about this process. Discovering the second by watching the first makes the failure look
like data — and, as §4.2.2 measures, makes the fix's correctness depend on the *content* of the memo being
reviewed.

---

## §2 Coupled invariants

- **J1 — capability ≠ datum.** A row is *unmapped* only if the mapper ran and declined. If the mapper is
  absent, no row is unmapped; the run is uncertified. One return value (`None`) must not carry both.
  ⚠ J1 forbids the two questions sharing a *return value*; it does **not** require them to share a *site*.
  Draft 5's item 6 read it the second way and broke J3 — see §4.2.3 item 3.
- **J2 — the two capabilities must degrade the same direction.** Verifying a citation needs the `webref`
  CLI *and* the label map. Measured on the naive carve, one hard-fails and the other exits 0 (§4.2.1); its
  in-code comment claims they "degrade the same way". They do not.
- **J3 — one degradation must survive.** `--no-verify --no-grep-pass` (structure + breadth only) must keep
  working with the tools tree absent.
- **J4 — one enforcement mechanism, not two.** If `mise` and `ci.yml` each spell the suite invocation, a
  later suite is added to one and not the other. `trip-wires` already answers this: the script is the SoT
  and each runner is a caller.
- **J5 — A adds no network dependency.** The plan-review gate is run by every lane and `mise run ci` is the
  mandatory pre-push gate. Both causes in J1 are static process facts; neither requires a fetch. J5 is
  carried by **P9 *and* §12 (3)'s grep together** — neither alone suffices, because the gate's only fetch
  happens in a *child* process (§4.3.3).

J1–J3 live in `preflight.main`'s control flow and cannot be applied one at a time without transiently
breaking each other, which is why §5 measures the configuration matrix rather than a sample. J4 is
independent. J5 is a property of the whole slice.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | fixture | the labelled `§3` row a fail-closed run must still map | §4.4 — `test_preflight.py` P2/P3 fixture memo | ✓ — the fixture set is authored, not discovered | no |
| WHATWG HTML §4.10.21.2 Constraint validation | fixture | a second citation, so `seen_pairs` dedup is exercised | §4.4 — same fixture | ✓ | no |

**Breadth**: K=1 spec (`html`), M=2 rows → preflight verdict **ok (single PR scope)**.

**Why two rows and not padded**: this slice ships no spec algorithm. Rows here are test fixtures, and a
fixture set larger than the property under test is padding. See §0.5's ⚠ for what this table does *not*
certify.

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** Nothing here is reachable from page content, script, or a network peer's
data. The inputs are the plan-memo's path *and its content*: `parse_spec_cell` extracts a label and a
section number from memo cell text, and `verify_citation` passes **both** to a subprocess. §4.2.2's finding
is that memo content steers control flow, so listing only the path omits the input that section proves is
load-bearing. (Symbols, not line numbers — §15 block 4 locates each; draft 5 carried eight line cites, all
branch-relative, four found by review and four more by re-derivation.)

**Both argv elements stay bounded, and A moves neither bound outside the pinned set.** `section` is bounded
by `SECTION_REF_RE`, untouched. `shortname` is bounded on `origin/main` by the 15-key `SPEC_LABEL_REVERSE`
and after A by the 24-key pinned `LABEL_TO_SHORTNAME` — a **strict superset over the same 12 specs**: 9 new
keys, 0 changed values, 0 lost, and **no new spec enters the set** (§15 block 6). Draft 4 replaced that
bound with a 948-entry third-party document fetched at gate time on every plan review in every lane; that
exposure delta — outbound *and* inbound — leaves with the widening.

**Discovery method.** Every number is produced by executing §15, against `origin/main` and never the branch
(§14 C1 is what happens otherwise); branch/lane facts use **three-dot** ranges (C4); a proposed patch is
*measured*, not read (§4.2.2 was found by applying draft 1's own fix in a sandbox); and a claim inherited
from another slice's memo is re-derived before being relied on (§0's ⚠ is what that produced).

---

## §4 The edit set

### §4.0 Step 0 — re-carve on the seam the umbrella already draws

The re-carve commit is identified by **subject**, never sha (§0). §15 block 7 prints its hash and the
per-file numstat; the split it produces:

| File | Half |
|---|---|
| `_webref/spec_labels.py` | **split** — see the region map below |
| `skills/elidex-plan-review/preflight.py` | **A** — drops local `SPEC_LABEL_REVERSE`, imports the map |
| `_webref/commands/coverage_map.py` | **split** — the delegation to `label_for` is A; the changed last-resort is B |
| `_webref/cli.py` | **split** — the blurb derivation is A; the `cite-audit` subparser + import + example line are B |
| `_webref/DESIGN.md` | **split** — the `spec_labels.py` bullet is A minus its catalog sentence; the `cite_audit.py` adapter bullet, CLI examples and three-bucket paragraph are B |
| `_webref/test_cite_audit.py` | **split** — `TestSharedSpecLabelMap`'s first **8** tests become A's `test_spec_labels.py`; `test_coverage_map_fallback_round_trips` + the `coverage_map_label` helper are **B** (they assert the catalog round-trip); the remaining 10 classes are B's |
| `_webref/commands/cite_audit.py` | **B** — the detector |
| `_webref/sources/webref_data.py` | **B** — `@lru_cache` motivated by the detector's per-section loop |

**`spec_labels.py`, by region** — the split is inside the file, so it is mapped rather than assigned.
Regions are named by content, not line number; §15 block 8 prints their current ranges. Draft 5's line map
left the docstring's closing `"""` unassigned, which A's half cannot compile without.

| Region | Half |
|---|---|
| module docstring's drift rationale **and its closing `"""`** | **A** (with "Four sites" → **three**; the `cite_audit.py` bullet is B's) |
| the docstring's *"`SPECS` is a fallback, not the source"* paragraph | **B** |
| `SPECS` and the three derived dicts | **A** |
| `_catalog()` and its `from .sources.webref_data import _data_index` | **B** |
| `label_for` — the `SHORTNAME_TO_LABEL` lookup is **A**; its catalog branch | **B** |
| `shortname_for` — the pinned lookup is **A**; its catalog branches and the CSS-module docstring paragraph | **B** |

`coverage_map._spec_label` is the same shape: A ships `return label_for(shortname) or
shortname.upper().replace("-", " ")` — **`origin/main`'s last-resort verbatim**. The branch's changed
last-resort (`or shortname`) is only correct *together with* the catalog and B §4.1.8's round-trip rules.

⚠ **The prose needs its own pass.** Five sites in the A column describe `commands/cite_audit.py` as extant
(the `spec_labels.py` docstring bullet, the `DESIGN.md` bullet, `preflight.py`'s new comment, and the moved
tests' docstrings). `cite_audit.py` is **absent from `origin/main`**. A filename-only purity check passes
while every one of these is present, which is why §12 (3) carries content assertions.

Result: `webref-cite-audit-tool` = `origin/main` + the A column + A's edits; a new branch for B = A's
landed head + the B column. **B's memo does not currently describe that base** — §13 enumerates the edits A
makes to it in the same commit.

**Why A takes the map's pinned half rather than leaving the whole carve to B**: the import is what
*creates* the failable capability. If B lands it, `main` carries a fail-open plan-review gate — a gate
every lane runs — for the duration of B.

### §4.1 Slice routing

Rows marked **A** are A's; the rest name where the concern went. (Draft 5 titled this "What A deliberately
does not touch" while one row assigned work *to* A.)

| Concern | Slice | Why |
|---|---|---|
| **the catalog fall-through, the discriminated `_catalog()`, the reverse index and the round-trip rules** (B §4.1.2 / §4.1.7 / §4.1.8) | **B** | §10-Q1's boundary. A lands the map's *shape*; B owns its lookup semantics — and the fall-through **is** lookup semantics, which is what draft 4 contradicted by shipping it |
| the **generic/adapter siting of the parse-alias policy** (`"WHATWG "` prefix, memo abbreviations) | **B** | §7 row 1: A *does* move elidex policy into the generic core. B replaces that lookup wholesale, so B is the slice that can site it — routed rather than waved away |
| `coverage_map`'s changed last-resort, and `test_coverage_map_fallback_round_trips` | **B** | the property it asserts is only true once the catalog and B §4.1.8's rules are in |
| `cite_audit.py`, `test_cite_audit.py`, the `cite-audit` subparser, `webref_data.py`'s memo | **B** | the detector |
| `spec_labels`'s public-surface reduction (`project_pr-a0-review-ledger` #25) | **B** | its stated root is `cite_audit.py` indexing `LABEL_TO_SHORTNAME` directly instead of calling `shortname_for`. Reducing the surface in A would also trip the shipped `test_module_leaves_no_temporaries_to_delete` guard |
| `SKILL.md` — `Preflight semantics`' **Hard-fail** bullet, the **Soft-warn** bullet (it lists "unrecognized spec labels" as unconditionally exit-0, the sentence J1 is about), the **Flags** bullet's `--no-verify`, and Pre-condition #1 | **A** | A adds a hard-fail cause, gives `--no-verify` a second role, and adds the no-spec-surface declaration. No other slice claims this file |
| one shared `SECTION_NUMBER_RE` across `preflight` / `cite_audit` / `section_sort` | **B** | `preflight.SECTION_REF_RE` is A's file but B's grammar unification; A leaves it byte-identical |
| `axes.md` (2)/(4) and its Axis 4 `§"Spec citation table" 欠落` detect (which a no-spec-surface memo will draw); `CLAUDE.md` § "Spec citation"; `DESIGN.md`'s reported-class contract | **C** | amending or retiring a review requirement is C's charter |
| the `crates/**` citation repairs and the 8 newly-authored wrong citations | **D** | content, not plumbing |
| `grep_pass.py` reporting a wrong repo root as one HARD finding *per referenced path* | **C** | §1's class in a neighbouring gate. Draft 5 recorded it with no home at all, so it had no re-evaluation trigger; C already edits review tooling |

### §4.2 A1 — land the capability fail-closed

#### §4.2.1 The measured asymmetry

Measured against the carve as authored, in a sandbox repo skeleton so `REPO_ROOT` resolves, with
`--no-grep-pass` throughout (the sandbox's `REPO_ROOT` is the sandbox, so grep-pass reports 44 hard
findings for `crates/**` paths that do not exist there — an artifact). **Input**:
`elidex-wt-submittable/docs/plans/2026-07-form-submittable-category-repair.md` (21 data rows, 15 unique
pairs) — draft 5 gave the numbers without naming the memo they came from.

| Case | Removed | Result | Exit |
|---|---|---|---|
| **A** | nothing | 21 rows, 21 parsed citations, **15 unique citations verified** | **0** |
| **B** | `.claude/tools/webref` (pre-existing check) | `❌ HARD FAIL — citation verification: 15 failure(s)` | **1** |
| **C** | `.claude/tools/_webref` (the new import) | `parsed citations: 0`, `unmapped-label rows: 21`, **no verify section at all** | **0** |

`15`, not `21`: `seen_pairs` dedups 21 data rows to 15 unique `(shortname, section)` pairs. Case C also
emits a **wrong-cause remedy** — "add the spec to `spec_labels.py::SPECS`", the file that failed to import.

**Case C does not exist on `origin/main`**: there, `shortname_from_label` reads a module-local dict with no
import to fail. The asymmetry is created by moving the map, which is why the slice that moves it owns it.

#### §4.2.2 The tri-state cannot live in `shortname_from_label`

Applied verbatim in the sandbox (a `TOOLS_UNAVAILABLE` sentinel returned from that function, propagated to
`main`, hard-failing there), with `_webref` removed:

| Fixture memo | Result |
|---|---|
| §3 rows carrying spec labels (`WHATWG HTML §4.10.21 …`) | **EXIT 1** — fails closed ✓ |
| §3 rows opening with `§` (no label) | **EXIT 0** — still fails **open** ✗ |

Cause is the function's first line:

```python
def shortname_from_label(label: str | None) -> str | None:
    if not label:
        return None          # ← taken before any availability check below
    ...
```

`parse_spec_cell` returns `cell[:m.start()].strip()`, so a cell beginning with `§` yields `""`. Every such
row short-circuits out before the capability is consulted. The gate's fail-closed property becomes **a
function of the reviewed memo's cell formatting** — J1 restated as a defect.

#### §4.2.3 The fix — two static causes, one verdict, computed early and acted on late

**Why this is smaller than draft 4's.** Draft 4 had a *third* cause: with the catalog fall-through, an
offline lookup could die inside the row loop (B §4.1.7's `SystemExit` escape, re-derived in §15 block 9).
Because that cause can only materialise *during* lookup, draft 4 had to aggregate the verdict across the
loop, add an `UNCERTAIN` arm and two accumulators, and thread a cause out of `spec_labels`. Dropping the
widening deletes the cause and all of that machinery.

1. **Two causes, both static process facts, evaluated once at `main`'s top**: `WEBREF.is_file()` and
   `_shortname_for is None`. The capability verdict is their union.
2. **`shortname_for` stays `str | None`.** No tri-state, no `resolve_label`, no discriminated `_catalog()`.
3. **`shortname_from_label` keeps returning `None` when the map is absent**, exactly as the carve does, and
   the row loop keeps its two arms — MAPPED and UNKNOWN.
   ⚠ **Draft 5's item 6 said this branch becomes "a hard precondition … so no second site answers the
   capability question". That was wrong and broke J3**: under `--no-verify` the hard-fail is suppressed by
   construction, so the loop still calls `shortname_from_label`, a raising precondition fires, and §5 row 8
   (exit 0) becomes a traceback. J1 forbids one *return value* carrying both questions; it does not forbid
   two *sites*. Classification is the row's question, the verdict is the process's, and they are answered
   in different places on purpose. Keeping the loop also avoids draft 2's regression, measured on a
   7-spec/7-row fixture: skipping it took `K` 7 → **0**, `⚠ SPLIT-DEFAULT` → `ok (single PR scope)`, and
   `--strict-breadth` exit 1 → **0** — silently disabling the split gate `SKILL.md` makes a
   stop-and-ask-user step.
4. **Where the verdict is *acted on*, which draft 5 never said.** At the verification stage, not at
   `main`'s top — acting at the top would hard-fail a no-spec-surface memo, which §4.2.5 forbids. And the
   existing trigger is insufficient: `if not args.no_verify and citations:` never fires when the map is
   absent, because every row goes UNKNOWN and `citations` is empty — that *is* §4.2.1's case C. So the
   condition widens to **`not args.no_verify and (citations or (unavailable and data_rows))`**.
   Unavailable + verification requested → HARD FAIL in the same `❌ HARD FAIL — …` shape as the other
   three, naming each absent cause and `--no-verify` as the suppressor. Unavailable + `--no-verify` →
   exit 0 (J3). A no-spec-surface memo has no `data_rows`, so the third arm cannot fire there (§5 row 14).
5. **Every summary line states its basis, and the verify line stops going silent.** Three changes to one
   print block:
   - `citation verify: n/a (0 of N rows resolvable)` when the verify stage ran and resolved nothing —
     today that line is simply **absent** (§1 item 3, measured). The live case, not an edge: the in-flight
     c3-plan memo's 18 rows hit it.
   - the breadth line reads `K=<n> (n of N counted by label spelling)` whenever `unmapped_rows > 0`.
     Draft 5 wrote "(unresolved — counted by label spelling)", which misdescribes the *partial* case
     (`unique_specs` mixes shortname and label keys), and added an "or the capability is absent" disjunct
     that is unreachable — capability-absent with rows implies `unmapped_rows > 0` already.
   - the soft-warn remedy stops naming `SPEC_LABEL_REVERSE`, a symbol A deletes.
6. No third key space is introduced, so `K` and the spec list it prints cannot disagree — each MAPPED
   shortname and each distinct unrecognized label contributes exactly one entry to both, as on
   `origin/main`.

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is currently re-tested
inside `verify_citation` on **every unique citation** — 15 times in case A — reporting one process-level
fact as 15 per-citation failures. After the hoist, case B's exit code is unchanged (1) and its diagnostic
is one line. The guard inside `verify_citation` becomes an **explicit raise**, not an `assert`: under
`python3 -O` an assert is stripped and a direct caller would get exactly the silent non-zero this change
exists to remove.

#### §4.2.4 The remedy text

**Four** strings, currently one, because there are four ways to fail and the author's next action differs
in each. (Draft 4 had five; the catalog-unreachable string leaves with the widening.)

| Condition | Remedy |
|---|---|
| genuinely unmapped label | "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the label spelling" |
| **label-less cell** (`\| §4.10.21 … \|`) | "the Spec section cell must open with a spec label" — measured: today this row prints the `SPECS` advice against `<empty>`, advice that cannot be acted on |
| tools unavailable (import failed) | the import error and the path attempted, plus `--no-verify` |
| CLI missing | the expected path, plus `--no-verify` |

#### §4.2.5 A5 — let a slice declare that it has no spec surface

A slice implementing no spec logic must today author fixture citations and then receives `citation verify:
ok (2 unique citation(s) checked)` as its headline — a verdict about fixtures presented as a verdict about
the slice. §1's anchor, in A's own file. **Ownership**: draft 2 routed this to B (the umbrella forbids B
editing review policy); draft 3 to C on the ground that `axes.md` holds the authoring contract — also
wrong, the contract is `SKILL.md` Pre-condition #1, which §4.1 assigns to A. Two owners who could not
perform the fix is the signal that the deferral was the error.

**Specification** — round 4 found this declared but unspecified; round 5 found the recognition rule unstated
on the three axes every other §3 scanner already resolves.

- **Accepted shape**: the `## §3. Spec coverage map` heading stays **required**. Its body may contain, in
  place of a table, one marker line.
- **Recognition** — the same three properties `find_coverage_map_section` and `find_table` already thread,
  because a rule weaker than theirs turns the marker into the silent bypass this section argues it is not:
  (a) **line-anchored** — the line's first non-whitespace content is the literal `**No spec surface**`;
  (b) **fence-aware** — `fence_state`-gated, so a memo quoting the marker inside a fenced block (as this
  one does) does not trigger it; (c) **§3-scoped** — only between the section's `body_start` and `body_end`.
- **Hard-fail on ambiguity**: marker **and** a table (with or without data rows); or the marker twice.
- **Verdict**: `citation verify: n/a (no spec surface declared)` and `breadth: n/a (no spec surface
  declared)` — not `ok`, not `0`. Draft 4's version printed `ok`/`0` vacuously.
- **Every other gate runs unchanged**: fence-state parsing, the structural checks, and the grep-pass.
- **Capability interaction** (§5 row 14): the verdict is computed and printed but cannot hard-fail, because
  `data_rows` is empty — nothing was requested and nothing was suppressed. This falls out of §4.2.3 item 4's
  condition rather than being a special case.
- **Residual, stated rather than argued away**: an author can declare no spec surface to skip citation
  verification, and unlike `--no-verify` — an *invoker* decision visible in the command every reviewer
  types — the marker lives in the *artifact*, so one author's edit suppresses verification for every later
  reviewer. Mitigations: the ambiguity hard-fail is mechanical; the marker is greppable in one command
  (§15 block 10); the gate prints `n/a`, not `ok`; and Axis 4 reads the memo regardless. §10-Q4 puts the
  residual to review rather than closing it.
- `SKILL.md` Pre-condition #1 gains the same sentence, and `axes.md`'s Axis 4 detect for a missing spec
  citation table is routed to **C** (§4.1) — otherwise every adopting memo draws a standing finding no
  slice is scheduled to answer.

### §4.3 A2 — give the suites a scheduler

#### §4.3.1 The hole, measured on `origin/main` across all three workflows

- `ci.yml`'s `changes` filter has two sets, `rust` and `config`; **`.claude/**` is in neither**, and all
  three jobs (`check`, `doc`, `deny`) are gated on one of the two.
- `ci.yml` **never invokes `mise`** — the single `mise` string in the file is `mise.toml` as a filter entry.
- `codeql.yml` analyses `[actions, rust]` on push-to-main + a weekly cron — **no Python, no `pull_request`
  trigger**. `audit.yml` is `cargo audit` on a weekly cron.

⇒ a `.claude/**`-only pull request triggers **zero jobs**. §15 block 3 prints the filters and the suite
counts (47 tests across 4 files, verified 2026-07-29). A adds `test_spec_labels.py` (8 tests moved by §4.0) and
`test_preflight.py` (§6).

#### §4.3.2 The mechanism — one script, two callers (J4)

`.claude/tools/python-suites.sh`, `set -euo pipefail`, `cd "$(dirname "$0")/../.."`, then two `discover`
lines rooted at `.claude/tools/_webref` and `.claude/skills/elidex-plan-review`.

- `mise.toml` gains `[tasks.tools-test]` = `bash .claude/tools/python-suites.sh`, added to
  `[tasks.ci].depends`.
- `ci.yml` gains a `tools` path-filter set (`.claude/tools/**`, `.claude/skills/**`,
  `.github/workflows/**`) and a `tools` job on `ubuntu-latest` running the same script under the same
  `|| github.event_name == 'push'` bypass the other three jobs use.
- **The script fails loudly when a `test_*.py` under `.claude/` is not collected by either `discover`
  root.** Draft 5 worded this as "outside the filtered paths", which keys on the CI *filter* — a strictly
  broader set than the two roots, so a suite at e.g. `.claude/skills/elidex-review/` would be inside the
  filter, outside `discover`, and pass the check. The set the assertion must range over is
  `git ls-files '.claude/**/test_*.py'` (§15 block 11).

This is the `trip-wires` shape verbatim, so it introduces no new pattern.

#### §4.3.3 The network question — answered by construction, not by disposition

Measured with a spy on `urllib.request.urlopen`: **0 calls** across all 47 `origin/main` tests (§15 block
3). A's 8 moved tests exercise `spec_labels`'s pinned dicts, `coverage_map._spec_label` and
`preflight.shortname_from_label`; under §4.0's split none reaches `sources/webref_data`, because
`spec_labels.py` no longer imports it at all. **A's suite set therefore fetches nothing** — predicted, and
pinned by **P9**.

⚠ **Draft 5's P9 could not have detected a violation.** It patched `urllib.request.urlopen` in the *parent*
process, but `verify_citation` runs `subprocess.run([sys.executable, WEBREF, "heading", …])` and `urlopen`
is called only inside `cache.py`, in the **child**. A parent patch is structurally blind to the gate's only
fetch. P9 is respecified in §6 as two assertions at the level the fetch actually happens.

This is the concrete payoff of the scope change. Draft 4 measured **1** fetch per run
(`raw.githubusercontent.com/w3c/webref/main/ed/index.json`, ~1.5 MB), had to argue it acceptable in
`mise run ci` — CLAUDE.md's *mandatory* pre-push gate — and opened a deferral plus an umbrella obligation to
make it survivable later. All three disappear. What replaces them is one forward-binding constraint A adds
to the umbrella's "Constraints each slice inherits":

> **No slice may make label resolution require the network without shipping its offline degradation in the
> same slice.** Slice B introduces the catalog fall-through and therefore owns the offline contract for it.

A constraint, not a deferral — it binds B at authoring time rather than recording an unowned concern.

⚠ **What A does *not* claim**: that the plan-review gate becomes offline-capable. It is not, and was not.
`verify_citation` shells out to `webref heading`, which issues a conditional GET, and `cache.py` `sys.exit`s
on `URLError` — so **`origin/main`'s gate already requires the network in default mode**, before and after
A. A's claim is narrower and exact: *A adds no network requirement that was not already there*, and the
`--no-verify` degradation (J3) stays offline-clean.

#### §4.3.4 What "enforced" can honestly mean here

`main` is governed by an **active** ruleset `main-protection` whose rules are `deletion` /
`non_fast_forward` / `pull_request` (§15 block 12). There is **no `required_status_checks` rule**, so a red
`tools` job does not block a merge; CLAUDE.md's workflow ("CI 全 pass を目視確認してから squash merge") is
the blocking step, and it is a human one. (`gh api …/branches/main/protection` → 404 is the **deprecated
legacy endpoint** and means "not protected *via the legacy API*", not "unprotected".)

The claim A may make: the job makes a regression **visible, attributed, and on the PR page at review time**,
where today it is invisible in every event. That is what §12 asserts — no more.

#### §4.3.5 The interpreter floor

Measured: **no `.claude` Python source uses syntax newer than 3.9** (`match`, `except*`, `tomllib`,
`typing.Self`, `ExceptionGroup`, atomic groups — all absent). Local dev is 3.14.6. Nothing in the repository
declares a floor.

`python-suites.sh` asserts `sys.version_info >= (3, 9)` — **A's own measured need** — and the job echoes
`python3 -VV`. B raises the floor when B lands `(?>...)`. Note `SKILL.md`'s Step 0 invokes `preflight.py`
directly, bypassing the script — unaffected today (A adds no version-dependent syntax), marked UNCHECKED in
§5.

### §4.4 A3 — site the label-map tests where they belong, from the start

§4.0 moves `TestSharedSpecLabelMap`'s 8 A-tests into `test_spec_labels.py`. One assertion inside
`test_all_three_consumers_derive_from_specs` does not belong there: it inserts the *elidex skill's*
directory onto `sys.path` and imports `preflight` — the one **import-time executable** edge that blocks
`DESIGN.md`'s goal of keeping the drift-detection core movable to a standalone repository.

**Fix**: the `preflight` half goes to `.claude/skills/elidex-plan-review/test_preflight.py`, beside
`preflight.py` and `test_grep_pass.py` — the home exists and the dependency direction is right (consumer
depends on library). `test_spec_labels.py` keeps the `coverage_map` half with a module-top-level import. No
`sys.path` mutation survives inside any test method. The `coverage_map_label` helper is **not** collapsed
here — its only caller is `test_coverage_map_fallback_round_trips`, and both go to B (§4.0).

### §4.5 Test-siting constraints the plan must state, not discover

1. **`_shortname_for` is bound at module import**, so "make the import fail" cannot be done by removing
   `.claude/tools` from `sys.path` and reloading — `preflight.py` **re-inserts that directory on every
   import**, so the module under test re-establishes the capability the test is removing. Working
   mechanisms are a `sys.modules`/`__import__` hook plus `importlib.reload`, or a subprocess; they pin
   different lines. An in-process `preflight._shortname_for = None` pins the new precondition but leaves the
   `except Exception` guard **mutation-green**. **P2 uses the reload form** and P2b adds a subprocess case.
2. **P1 needs `_shortname_for` bound; P2/P3/P4 need it `None`** — mutually exclusive process-global state in
   one file, and reloading does not restore it. `test_preflight.py` restores the module in `tearDown` via
   `importlib.reload` under the un-patched import, and P1 asserts the bound state at `setUp` so a leak fails
   loudly. `unittest` orders methods alphabetically, so relying on names is not a plan.
3. **The isolation contract is three pieces of process state**: `preflight._shortname_for`, `sys.path`, and
   **`preflight.verify_citation`** — P1b stubs it and P9 asserts against it, so an unrestored stub would
   make P1b's neighbours pass vacuously. Draft 4 listed `sources/webref_data._INDEX` and
   `try_fetch_data.cache_clear()`; both leave with the widening, and the second never existed on
   `origin/main` (that `@lru_cache` is a hunk §4.0 routes to B). Draft 5 dropped both and then added P9's
   `urlopen` patch without adding it here; P9's respecification (§6) moves that perturbation into a child
   process, so the parent-side contract stays these three.

---

## §5 Behavior deltas, claims, and their pins — one table

Round 4's finding was that §5 / §6 / §4.6 / §12(2) were four spellings of one table. They are one table
here. **Baseline is `origin/main`**; the carve is an intermediate artifact that never lands, and its
measured case C (§4.2.1) is cited once as the reason the design exists, not as a column. **On `origin/main`
the "map" axis does not exist** — the map is a module-local dict with no import to fail — so those rows read
`n/a`.

Every measured row ran with `--no-grep-pass`, because the sandbox's `REPO_ROOT` is the sandbox and grep-pass
reports 44 artefact hard-findings there; `--no-grep-pass` is **not** the default, so the `mode` column is
the *verify* axis only. The two capability causes are a **union**, so any combination of absent causes
yields one verdict; what differs is the **diagnostic**, not the exit code.

| # | CLI | map | mode | §3 shape | `origin/main` | After A | Pin | Detects the naive carve? |
|---|---|---|---|---|---|---|---|---|
| 1 | ✓ | ✓ | default | labelled, pinned | 0 (15 verified) | **0** | P1b | — |
| 2 | ✓ | ✓ | `--no-verify` | labelled | 0 | **0** | P1b | — |
| 3 | ✗ | ✓ | default | labelled | 1 (15 per-citation failures) | **1** — one diagnostic line | P6 | ✓ |
| 4 | ✗ | ✓ | default | label-less | **0** (`citations` empty ⇒ verify block skipped) | **1** | P4 | ✓ |
| 5 | ✗ | ✓ | `--no-verify` | either | 0 | **0** — capability unused | P3 | — |
| 6 | ✓ | ✗ | default | labelled | n/a | **1** | P2, P2b | ✓ |
| 7 | ✓ | ✗ | default | label-less | n/a | **1** (§4.2.2) | P4 | ✓ |
| 8 | ✓ | ✗ | `--no-verify` | either | n/a | **0** (J3) | P3 | ✓ |
| 9 | ✗ | ✗ | default | any | n/a | **1**, diagnostic names both causes | P6 | ✓ |
| 10 | ✓ | ✓ | default | **alias spelling** (`\| Fetch §2.2.5 \|`) | **0**, unmapped soft-warn, **no verify line** | **0**, mapped and verified | P12b | ✓ |
| 11 | ✓ | ✓ | default | **all rows unmapped** | **0**, **no `citation verify:` line at all** (measured) | **0** with `citation verify: n/a (0 of N rows resolvable)` | P13 | ✓ |
| 12 | ✓ | ✓ | default | **marker, no table** | **1** (no-table hard-fail) | **0**, `verify: n/a` | P11 | ✓ |
| 12b | ✓ | ✓ | default | **marker + header-only table** | **1** (0-data-rows hard-fail) | **1** — ambiguous declaration | P11b | ✓ |
| 13 | ✓ | ✓ | default | **marker + populated table** | **0** (the marker is inert prose; the table verifies — measured) | **1** — ambiguous declaration | P11b | ✓ |
| 14 | ✓ | ✗ | default | **marker** | n/a | **0** — no `data_rows`, so §4.2.3 item 4's third arm cannot fire | P11c | ✓ |

**Newly-red**: rows 4, 6, 7, 9, 12b, 13. **Rows moving 1 → 0**: 12 only, and there the red was the gate
rejecting a valid input shape, not a defect being hidden. **Rows whose exit code is unchanged but whose
output changes**: 10, 11.

**Measured vs predicted**: the `origin/main` column is measured (rows 1/3/5 in the §4.2.1 sandbox; rows 4,
10, 11, 12, 12b, 13 re-measured this round against a throwaway `origin/main` worktree — §15 block 5, which
is why row 13's value is 0 and not draft 5's 1). The *After A* column is **predicted** by construction; the
Pin column converts each prediction into a check.

### Claims that are not rows

| Claim | Check |
|---|---|
| **A adds no network dependency (J5)** | **P9 + §12 (3)** together — §4.3.3 for why neither alone suffices |
| A's **spec set** is unchanged; 9 additional **spellings** resolve | **P12** (the 15 `origin/main` pairs, vendored) **+ P12b** (each of the 9 new spellings resolves to an already-pinned shortname, and `set(LABEL_TO_SHORTNAME.values())` equals `origin/main`'s value set). Reachability recorded, not hypothesised: 4 landed memos carry such cells (§15 block 6); all 4 verify green today |
| `coverage_map._spec_label` is unchanged | **P14** — the 12 pinned shortnames **and** a non-pinned sample exercising the `.upper().replace("-", " ")` last-resort, which is the branch A must not take |
| The summary states its basis when rows are unresolved | **P3** (breadth) + **P13** (verify line) |
| Consumers derive from `SPECS` | **P1** + `test_spec_labels.py`'s `coverage_map` half |
| The remedy text names the right cause | **P5** (four strings, each for its own cause and no other) |
| The suites run at all, and none is uncollected | `mise run tools-test`; the GitHub `tools` job; the script's own `git ls-files` assertion (§4.3.2) |
| A carries no part of B | §12 (3) — **UNCHECKED until §4.0 is performed**; fails at today's head by construction |
| The interpreter floor holds on the runner | `python-suites.sh`'s assert — **only on the script path**; `SKILL.md`'s direct invocation is **UNCHECKED** |
| A red `tools` job prevents a merge | **UNCHECKED and false** — no `required_status_checks` rule (§4.3.4). What is checked is visibility |
| `shortname_for` and `origin/main`'s `shortname_from_label` are equivalent functions | **UNCHECKED, and false in one unreachable respect**: `shortname_for` calls `.strip()`, `origin/main`'s does not, so `"  whatwg html "` differs. Unreachable through the gate (`parse_spec_cell` already strips), so the *gate* claim (P12) holds; the *function* claim is not made |

---

## §6 Test plan

Fixture memos: `labelled.md`, `unlabelled.md`, `allunmapped.md`, `nospec.md`, `nospec-and-table.md`,
`nospec-and-header.md`. Each pin names its *mechanism*; its expected values are §5's row, stated once.

**`.claude/skills/elidex-plan-review/test_preflight.py`** (new):

- **P1** the `shortname_from_label(label) == short` derivation assertion, moved from `test_cite_audit.py`,
  with no `sys.path` mutation in the test body and a `setUp` assertion that the module is un-poisoned.
- **P1b** `main` on `labelled.md` in default **and** `--no-verify` mode with both capabilities present,
  `verify_citation` stubbed to `(True, "")` — rows 1/2. The stub is what lets the most-run configuration be
  pinned without J5 and row 1 competing (draft 5 pinned these rows to P1, which never enters `main`, so the
  happy path was unpinned while §5 presented it as checked).
- **P2** map unimportable, via the `importlib.reload`-under-import-hook form, `tearDown` restoring both the
  module **and** `sys.path`.
- **P2b** the same via a subprocess, pinning the `except Exception` guard. Mutation check: deleting that
  clause must turn P2b red — P2 alone leaves it green.
- **P3** `--no-verify --no-grep-pass` with the map absent — exit 0 **and** the breadth-basis qualifier.
- **P4** the **label-shape independence property**: `labelled.md` and `unlabelled.md` produce the *same*
  exit code in every capability state (rows 4/6/7). This pins the property directly rather than vendoring
  the rejected patch as a fixture.
- **P5** each of the four remedy strings appears for its own cause and no other.
- **P6** the missing CLI is reported once, not once per citation (rows 3, 9).
- **P9** **J5, at the level the fetch happens**: (a) across A's whole suite set, `preflight.verify_citation`
  never reaches a real subprocess — asserted by a spy that fails if `subprocess.run` is called with `WEBREF`
  in argv; and (b) `bash python-suites.sh` runs green in a child with `http_proxy`/`https_proxy` pointed at
  a closed port, so any real fetch fails fast. (b) is a subprocess case precisely so it does not perturb
  parent-process state (§4.5 item 3) and does not re-enter itself.
- **P11** `nospec.md` → exit 0 with `citation verify: n/a` and `breadth: n/a` (row 12); asserts the strings,
  not just the exit code. **P11b** `nospec-and-table.md` and `nospec-and-header.md` → exit 1 naming the
  ambiguity (rows 13, 12b). **P11c** `nospec.md` with the map absent → exit 0 (row 14).
- **P12** `shortname_from_label` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, vendored as a
  literal — correct here precisely because the point is to freeze the *old* table. **P12b** each of the 9
  added spellings resolves to a shortname already in that table, and the value sets are equal (row 10).
- **P13** `allunmapped.md` in default mode → exit 0 **and** the `citation verify: n/a (0 of N rows
  resolvable)` line present (row 11). Mutation check: on `origin/main` this line does not exist, so P13
  fails against the unmodified gate.

**`.claude/tools/_webref/test_spec_labels.py`** (new by §4.0's split): `TestSharedSpecLabelMap`'s 8 A-tests
minus the `preflight` assertion, plus the `coverage_map` half at module-level import, plus **P14**. Slice B
appends its catalog cases — and `test_coverage_map_fallback_round_trips` — to this file rather than
creating it.

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** and **ECS-native** — not applicable; no `crates/**` diff, no
component, no entity, no system.

**`DESIGN.md` generic-core / elidex-adapter split** — the live boundary:

| Edit | Layer | Placement |
|---|---|---|
| `spec_labels.py`'s `SPECS` table | **generic mechanism carrying elidex policy** | ⚠ **A increases the externalization debt; draft 5 said it did not, measured against the wrong baseline.** On `origin/main` the generic core carries only the *forward* direction (`coverage_map._SPEC_LABEL_MAP`, self-documented "cosmetic only, not load-bearing for verification"); the *reverse* direction and the memo-abbreviation parse policy live in the elidex adapter (`preflight.py`), which is where `DESIGN.md` puts elidex policy. A moves the reverse direction into the core and widens the alias set from 3 spellings to 8. The carve's own `cite_audit.py` states the rule and obeys it for `DEFAULT_ROOT`/`DEFAULT_GLOB` — *"would park elidex policy in the generic core, against DESIGN.md's generic/adapter split"* — so the program applies it in one file of the carve and waives it in the sibling. **Routed to B** (§4.1): B replaces this lookup wholesale, so B is the slice that can site it. Recorded as a cost A pays, not one A avoids |
| `commands/coverage_map.py` consumer | **generic** | second consumer of the shared map; last-resort unchanged |
| `cli.py`'s blurb derivation | **generic wiring** | consumes `SHORTNAME_TO_BLURB`; adds no elidex policy |
| `_webref/DESIGN.md`, `spec_labels.py` bullet only | **generic** | describes a generic module; the CI facts do **not** go here (below) |
| `test_spec_labels.py` | **generic** | tests a generic module |
| §4.2 capability verdict, remedy text, no-spec-surface declaration, `test_preflight.py`, `SKILL.md` | **elidex skill** | consumes the library, adds no generic behavior |
| §4.3 script + `mise` task + CI job | **elidex repo infrastructure** | `.claude/tools/*.sh` is where the four trip-wire scripts already live |

⚠ Draft 2 planned to record the `mise` task, the CI job, the path filter and the interpreter floor in
`_webref/DESIGN.md`. That file says the core should "stay generic enough to move to a standalone repository
later" — a section describing `mise.toml` and `ci.yml` travels with the tree at externalization and is wrong
on arrival. Those facts live in `python-suites.sh`'s header and the `mise.toml` task comment.

⚠ **A introduces three elidex couplings into the generic tree, not one.** On `origin/main`,
`git grep -nE '\.claude/skills|elidex-plan-review' -- .claude/tools/_webref/` returns **exactly one** hit
(`cli.py`). A's `spec_labels.py` adds: the skill path in its docstring; the alias rationale ("real comments
and memos abbreviate"); and the load-time-consumer list naming the plan-review preflight. Only the *moved
assertion* was import-time executable, which is the narrow claim draft 2 made and draft 5 kept — but the
count was wrong. §12 (3) gains an assertion in this class, which its `cite.?audit` and `_catalog` greps do
not cover.

**One-issue-one-way**, three collapses: the label enumeration goes from three sites to one; the suite
invocation from zero canonical sites to one; the `WEBREF.is_file()` question from *n*-per-citation to one
verdict. The one remaining instance of §1's class inside A's own file — `preflight` reaching
`resolver.lookup_section` through a subprocess while reaching `spec_labels` in-process — is §11's slot.

---

## §8 Line-count budget

`wc -l` on `origin/main` (§15 block 13):

| File | On `origin/main` | After A (est.) |
|---|---|---|
| `.claude/skills/elidex-plan-review/preflight.py` | 499 | ~525 |
| `.claude/skills/elidex-plan-review/test_preflight.py` | — | ~340 |
| `.claude/tools/_webref/spec_labels.py` | — | ~100 |
| `.claude/tools/_webref/test_spec_labels.py` | — | ~115 |
| `.claude/tools/_webref/commands/coverage_map.py` | 114 | ~108 |
| `.claude/tools/_webref/cli.py` | 264 | ~272 |
| `.claude/tools/_webref/DESIGN.md` | 134 | ~139 |
| `.claude/tools/python-suites.sh` | — | ~35 |
| `mise.toml` | 136 | ~142 |
| `.github/workflows/ci.yml` | 126 | ~150 |

**1000-line touch-time check** (cohesion-based): the largest file in the touch set is `preflight.py` at
499 → ~525, half the threshold, and it is one cohesive gate whose seam (structure / breadth / citation /
grep-pass) is already four ordered blocks in `main`. Nothing is near a split.

---

## §9 Edge-dense assessment

The **base case** applies: an approved umbrella's narrowly-scoped, plan-reviewed per-PR slice is a terminal
unit, not re-split for touching the same subsystem as B/C/D.

What makes A safe as one slice: J1-J3 live in one function's control flow with one primary observable (an
exit code) and one secondary one (the summary's lines). §5 publishes the outcome-distinct rows plus the
collapse rule, and every row has a pin. J4 is independent. J5 is a single offline run. The
authoring-contract change (§4.2.5) is a **fourth** surface — additive, one input shape, three rows and three
pins — and draft 5's §9 omitted it from the accounting while §0.1 denied it existed.

**Draft 5 removed a capability cause, not an invariant axis, and said the wrong one.** Draft 4's third
capability cause was dynamic — it materialised inside the row loop — which forced the aggregated verdict,
the tri-state and five of the pins. Dropping it takes the *causes* from three to two. J1/J2/J3 remain three
and remain coupled, exactly as §2 says; draft 5's "the intersecting axes drop from three to two" conflated
the two counts.

`git diff --stat -- crates/` is empty and stays empty, so a regression degrades a developer tool and cannot
reach a page, a script, or a user.

The ordering couplings are the umbrella's rules, not exemptions: retiring the grep requirement before the
detector is sound would mandate an under-reporting detector (C after B), and the regression pins are
unenforced until a scheduler exists (A before B).

---

## §10 Open questions for `/elidex-plan-review`

Decided rather than listed, because each had one live option ([[feedback_no-low-value-choices]]): the
`verify_citation` guard is an **explicit raise**, not an `assert`; the re-carve is **its own commit, first
on A's branch**; the interpreter floor is **3.9**; the `tools` path filter stays **broad**, with the script
failing loudly on an uncollected suite; and draft 4's Q4 (`K`'s semantics) is answered by §4.2.3 item 5 —
`K` states its own basis, which makes it checkable instead of open.

- **Q1 — is the boundary drawn in the right place?** A keeps the *dedup* and hands B the *widening*. Three
  grounds, in order of weight:
  **(a) Correctness, measured.** On the branch, `shortname_for("CSS Text 3")` with `urlopen` raising gives
  `SystemExit ESCAPED _catalog()` (§15 block 9) — `cache.py` calls `sys.exit` and `SystemExit` is a
  `BaseException`, so `_catalog()`'s `except Exception` cannot catch it. Landing the widening as carved
  would put a resolver that `sys.exit`s offline into the gate every lane runs *and*, via
  `[tasks.ci].depends`, into `mise run ci`. Hardening a gate's failure semantics on top of that resolver is
  strictly worse than not landing it. The fix is B §4.1.7's, so the code is B's.
  **(b) The boundary A itself drew.** §4.1 assigns lookup semantics to B; the fall-through *is* lookup
  semantics, so draft 4 shipped in A the lines it declared B's.
  **(c) Cost.** The widening was the sole cause of the network dependency in `mise run ci`, the dynamic
  third capability cause, the aggregated verdict, the tri-state resolver, one deferral and one umbrella
  obligation — all deleted.
  **The cost of deferring it, stated:** the in-flight c3-plan memo carries 18 §3 rows (`CSSOM VIEW` ×14,
  `RESIZE OBSERVER` ×3, `INTERSECTION OBSERVER` ×1) that `origin/main` soft-warns and skips; the widening
  resolves all three correctly. That gain arrives one slice later. **Not** a ground for the boundary:
  draft 4's "wrong document" claim, falsified in §0.
- **Q2 — does `required_status_checks` belong in this PR?** It is one rule on an existing active ruleset.
  But the `pull_request` rule already carries `required_approving_review_count: 0` **and** a
  `RepositoryRole` bypass with `bypass_mode: always`, so adding the rule leaves it author-bypassable — it
  buys visibility-plus-friction, not enforcement. **Recommendation: register, do not implement** (§11).
- **Q3 — `#11-layoutbox-trip-wire-not-in-ci`.** The slot was registered by #488; the only
  `.github/workflows` touch since is **A's own**, so A is the trigger. `feedback_defer_lifecycle_policy`
  **Control C** binds the five dispositions to a calendar re-eval arrival and **Control D** to the
  milestone-close stocktake — neither binds them to a trigger firing, so draft 5's citation of Control D was
  wrong, and the re-eval date (2026-10-27) has not arrived. **Recommendation: record the trigger firing as
  an observation** in both files and leave the disposition to the existing date; if review prefers
  *extend-with-cause*, the policy requires an explicit new date, which A would set to **2027-01-31** (after
  the Layout lane's trip-wire item in `project_inline-mod-split-owed.md` §B).
- **Q4 — the residual in §4.2.5.** The marker is artifact-resident, so unlike `--no-verify` it suppresses
  verification for every later reviewer, not just the invoker. §4.2.5 lists four mitigations, only one of
  which is mechanical. The alternative is to require the marker to name its umbrella slice — checkable, but
  a coupling A has no other reason for. Put to review rather than closed.

---

## §11 Defer slots + per-PR ≤3 audit

**One own deferral** against ≤3, plus **one pre-existing-category entry**, a separate class
([[feedback_defer_cap_policy]]). Draft 4 had two own deferrals; `cleanup-webref-suites-offline` is
**dissolved, not deferred** — the dependency that created it leaves with the widening (§4.3.3), replaced by
a forward-binding umbrella constraint on B.

⚠ **Naming/counting rule, settled at umbrella level.** The registry treats `cleanup-*` as cap-exempt; B's
memo takes the stricter line. Two memos in one program cannot answer this differently, so A's landing edit
puts the rule in the umbrella's "Constraints each slice inherits", in the dimension that decides: **what
counts against the cap is own-vs-pre-existing, not the `cleanup-*` prefix.**

### Own deferral (1 of ≤3)

| Slot | Audit |
|---|---|
| **`cleanup-webref-preflight-inprocess-resolution`** | `preflight.verify_citation` forks a subprocess **and** an HTTP conditional-GET per unique citation, while the same file reaches `spec_labels` in-process — two ways to reach the shared library in one file. Measured **0.092 s per citation** as a subprocess vs **0.0008 s in-process** (§15 block 14). **Create-time audit**: spec-core ✗ / one-way ✗ / pragmatic-shortcut ✓ / repeat-signal ✓ (§1's class, third instance). **Category**: draft 5 called this "category 2, 別 slot 依存", which does not fit — the trigger is a PR slice, not a registered parent slot. It is **category 3** (the collapse decides whether the plan-review gate must be usable offline, which is B's contract); the cap policy's fallback when no category fits is to fold it in, and it is not folded in because collapsing now would decide B's offline policy by side effect. **Boundary cost**: the elidex adapter would import a second generic module (`resolver`) — a direction it already takes for `spec_labels`. (Draft 5 said `DESIGN.md` "does not list `resolver` among its declared generic surface"; that list names 7 of 24 modules — verified 2026-07-29: the "Current generic modules" bullet list has 7 entries, against `git ls-tree -r --name-only origin/main -- .claude/tools/_webref | grep '\.py$' | grep -v 'test_\|__init__' | wc -l` → 24 — and is a drift-tooling inventory, not a surface declaration.) **Trigger**: Slice B's `_catalog()` offline contract landing. **Re-eval**: 2026-11-30. **Resolved by**: Slice B. **Confidence**: High. |

### Pre-existing category (not an own deferral, not counted)

| Entry | Audit |
|---|---|
| **`cleanup-elidex-ci-required-status-checks`** | The `main-protection` ruleset has no `required_status_checks` rule, so every CI job is advisory. Pre-existing repo state; A neither creates nor worsens it. ⚠ The cost is **not** "one rule": the `pull_request` rule carries `required_approving_review_count: 0` and a `RepositoryRole` bypass with `bypass_mode: always`, so the rule alone would remain author-bypassable (§10-Q2). **Trigger**: the Layout lane wiring the trip-wires, or the first job stable enough to require. **Re-eval**: 2026-11-30. **Resolved by**: unassigned — which is what makes it pre-existing rather than own. **Confidence**: Medium. |

**Explicitly NOT deferred**: the re-carve (§4.0), the two-cause capability verdict and its act-site
(§4.2.3), the four remedy strings (§4.2.4), the no-spec-surface verdict and its recognition rule (§4.2.5),
the verify-line silence (§4.2.3 item 5), the test relocation (§4.4), the test-siting constraints (§4.5), the
script + `mise` task + CI job (§4.3), `SKILL.md`'s contract update (§4.1), the edits to B's memo (§13), and
the umbrella's two constraint lines.

---

## §12 Exit criterion

**(1) Green:** `mise run tools-test`

**(2) Red — every pin detects the defect it names.** Build a worktree at the re-carve commit (located by
subject — §15 block 7), copy in `test_preflight.py` and `test_spec_labels.py`, and run
`bash .claude/tools/python-suites.sh`: non-zero, with at least one failure attributable to **every pin whose
§5 row carries ✓ in the last column** — P2, P2b, P3, P4, P6, P9, P11, P11b, P11c, P12b, P13 — plus **P5**
and **P14**, whose claims live in §5's "Claims that are not rows" block and are marked there. P1 and P1b are
correctly absent: neither detects a re-carve defect. (Draft 5's list named P5 as ✓-derived while P5 had no
row, which made §12(2) the second list §5 exists to abolish.)

**(3) A carries no part of B — filenames, prose, and layer**, ranged from the re-carve:

```sh
git diff --name-only <re-carve>..HEAD -- .claude/tools/_webref/            # only §4.0's A column
git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/       # → empty
git grep -n '_catalog\|webref_data' -- .claude/tools/_webref/spec_labels.py  # → empty
git grep -cE '\.claude/skills|elidex-plan-review' -- .claude/tools/_webref/  # → the §7 ⚠ count, not more
```

The last three are not decoration: the split is at region granularity, five prose sites in the A column
describe `cite_audit.py` as extant, the third enforces that the widening did not travel back into A, and the
fourth is the only assertion in the class §7's ⚠ measures.

**(4) The branch carries only A's own memo and the umbrella.** `git diff --numstat origin/main...HEAD --
docs/plans/` currently lists A, B, C and the umbrella. The umbrella **must** ship with A (§13 edits it), so
the criterion is "A's memo + the umbrella, and nothing else". **B's and C's memos move to their own
branches** — after A's edits to B land (§13), so the corrected framing travels with the file.

**(5) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page. Verified by observation; today the same observation yields zero jobs.

---

## §13 Coordination

**Every lane fact below drifts by nature and is re-derived at landing (§15 block 15), not pinned here.**
Draft 5 called its PR list "complete" and was falsified within thirteen minutes — by two merges and one new
PR, one of the merges touching a file A edits.

| Lane | Overlap with A | Ordering rule |
|---|---|---|
| **Slice B** | total by construction — B branches from A's landed head and takes §4.0's B column, **including the catalog fall-through, the discriminated `_catalog()`, `coverage_map`'s changed last-resort and `test_coverage_map_fallback_round_trips`**. ⚠ **B's memo describes a base this re-slice dissolves, and A edits it** (below) | **A first** |
| **Slice C** | `_webref/DESIGN.md` — same file, disjoint sections. Inherits the §3 no-spec-surface declaration as its first real consumer, plus `axes.md`'s Axis 4 detect and `grep_pass.py`'s per-path finding (§4.1) | after B |
| **PR-A0 / D** | ⚠ its carve-provenance claim is **false at head** (`git diff domform-submittable-category -- .claude/` is non-empty) because A rebased and PR-A0 did not. **Rebase PR-A0**, then the claim holds again | after A/B/C |
| **the open `actions/checkout` bump** | ⚠ **actual `ci.yml` contention** — it modifies the same `steps:` blocks §4.3.2 extends, and after A it triggers the `tools` job, as will every future GHA bump | **Both directions**: whichever lands second adapts |
| **anything touching `.claude/skills/**` or `.claude/tools/**`** | after A these trigger the `tools` job. A *class*, not a list — three instances arose during round 5 alone | none |

**Edits A makes to B's memo, in the re-carve commit** (draft 5 asserted "B's memo needs no edit"; measured,
it is wrong at seven sites, one of them a claim A's own §13 measures as false):

1. **§4.1.2 / §4.1.7 / §4.1.8** are framed as fixing defects *extant at B's base*, with present-tense
   measurements and `spec_labels.py` line anchors. After the re-slice B **introduces** that code, so all
   three become "introduce it correctly" and their anchors go.
2. **§4.1.8's consequence sentence** — *"citation verification silently runs against the wrong document"* —
   is falsified for every level-collision pair it names (§0). B restates it on the 8 cross-series cases or
   drops it.
3. **§0.1's provenance paragraph** describes a branch composition this re-slice dissolves, and its
   `git diff domform-submittable-category -- .claude/` → 0 lines claim is stale.
4. **§4.1.2 / §4.1.8 cite `coverage_map`'s changed last-resort as pre-existing**; it is B's own edit.
5. **§4.2's seam list** names two seams A creates and must name the widening as a third.
6. **§4.2 / §13 point at "Slice A §4.1"** for the fail-closed work (now §4.2) and at "A §4.2" for the
   `SECTION_REF_RE` decision (now §4.1) — the two references are swapped.
7. **"A creates `test_preflight.py` with P1-P6"** — A ships P1, P1b, P2, P2b, P3, P4, P5, P6, P9, P11, P11b,
   P11c, P12, P12b, P13.

**Landing checklist**:

1. Re-run `preflight.py` on the plan-memos each lane is **authoring** — from each worktree's own copy, since
   `REPO_ROOT` derives from `__file__`. §15 block 15 *derives* the worktree list rather than fixing it; the
   umbrella's own bullet names a different set and is corrected in the same edit as item 3.
2. Register the 1 own deferral and the 1 pre-existing entry; add the umbrella's **two** constraint lines
   (the cap naming/counting rule, §11; the no-network-without-offline-degradation rule, §4.3.3); record the
   trip-wire trigger firing (§10-Q3) in **both** `project_open-defer-slots.md` and
   `project_inline-mod-split-owed.md` §B — draft 5 said the prose was "only in the latter", which is false
   and negated the instruction it qualified. Memory-file writes, not chips
   ([[reference_spawn-task-chips-not-durable]]).
3. **Correct the "10 in-flight memos in `elidex-wt-c3-plan`" claim at all four sites**, not one: the
   umbrella's Cross-lane bullet, `MEMORY.md`'s L3 bullet, and `project_citation-hygiene-program.md` twice.
   The true figure is **1** in-flight memo, carrying `CSSOM VIEW` / `RESIZE OBSERVER` / `INTERSECTION
   OBSERVER`. Grep the concept, not the string ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]).
   The same edit fixes the umbrella's branch-measured suite figures and its Slice A/B **Scope** cells, which
   never mention the spec-label map and so record no owner for it after the re-slice.
4. **Update `project_citation-hygiene-program.md`** — the program's designated cross-session SoT, which this
   draft falsifies at six points (draft number, head, behind-count, the carve-provenance line, the
   `spec_labels.py` split, the line count). Draft 5's checklist omitted it entirely.
5. Correct the two live stale `d3173bed` strings (`project_slice1-elementstate-cache-deletion-state.md`,
   `project_pr-a0-review-ledger.md`) — re-derived, still the only two.
6. `MEMORY.md`'s L3 bullet: set A-landed / B-next.
7. PR description states §4.3.3 (A adds no network, and the gate's pre-existing network requirement),
   §4.3.4 (no `required_status_checks`, and the bypass actor), §0.1 item 2 (the authoring-contract change),
   and the `tools`-filter collateral class.

---

## §14 Review-round index

Five `/elidex-plan-review` rounds. **This section is an index, not a restatement** — every live correction
is stated once, inline, at the section that acts on it.

**Round 1 → draft 2.** Root: the evidence base was measured on the branch, not `origin/main`. C1 slice
boundary + test counts · C2 the "unreachable" capability branch · C3 branch protection via the deprecated
endpoint · C4 two-dot ranges · C5 the offline gap routed to a non-owner · C6 a Python floor that was B's
need · C7 a six-cell "complete" matrix · C8 CI facts headed for the generic core · C9 a trip-wire rationale
inverting the Layout lane's record · C10 a red-check reading an uncommitted patch · C11 slot obstacle text
attributed to the wrong file.

**Round 2 → draft 3.** Root: draft 2's own fix opened a new failure and disabled a neighbouring gate. D1 the
third capability cause · D2 the row-loop skip stranding the accumulators · D4-D5 the `SPECS` bound and the
inbound exposure delta · D6 `assert` under `-O` · D7-D8 matrix arithmetic and mislabelled modes · D9
`cleanup-*` cap accounting · D10 vendoring the rejected patch as a fixture · D-a/D-b elidex coupling
attributed as pre-existing · D-c the no-spec-surface gap routed to B · D-d/D-e count and PR-list corrections.

**Round 3 → draft 4.** E1 §4.6 asserting the skip draft 3 removed · E2 `K` is not capability-independent ·
E3-E5 row-level corrections · E6 accumulators read everywhere and written nowhere · E7 the isolation
contract one module too shallow · E8 the no-spec-surface gap's second owner also could not perform it · E9
the trip-wire trigger is A itself · E10-E11 stale memory strings and a mid-review merge.

**Round 4 → draft 5.** Root: **the slice boundary, three drafts in**. F1 A hardens a gate on a resolver whose
correctness is B's · F2 §4.5 cited an API absent from `origin/main` · F3 §4.2.5 declared but unspecified ·
F4 §4.6 asserting a property marked UNCHECKED two rows above · F5 fixtures unable to reach their own rows ·
F6-F8 status, PR-list and provenance staleness · F9 length as restatement.

**Round 5 → draft 6.** 1 CRIT / 25 IMP / 22 MIN / 5 FP, in six clusters. **G1 (CRIT)** draft 5's item 6 made
`shortname_from_label` raise, breaking J3 under `--no-verify` — J1 forbids one *return value* carrying both
questions, not two *sites* (→ §2, §4.2.3 item 3) · **G2** *"A changes no resolution outcome"* false: the map
is a strict superset, 9 spellings newly resolve, 4 landed memos carry such cells, and P12 was structurally
blind to it (→ §0.1, §3.1, §5 row 10, P12b) · **G3** *"B's memo needs no edit"* false at seven sites (→ §13)
· **G4** four pins could not check what they claimed — P9 patched the parent process while the fetch is in a
child; rows 1/2 were pinned to a unit assertion that never enters `main`; `_spec_label`'s last-resort lost
its only exerciser to B; §12(2) was a second list (→ §6, §5, §12) · **G5** the marker's recognition rule was
unstated on anchoring, fences and scope, and §5 row 13's `origin/main` value was wrong (measured: 0, not 1)
(→ §4.2.5, §5) · **G6** the staleness class, fourth consecutive round — base, PR list, a dangling carve sha,
eight branch-relative line cites (four found by review, four more by re-derivation), and the program SoT
unlisted in the checklist; structural response: `origin/main` by symbol, the carve by subject, all counts in
§15 (→ §0, §15). Plus §1 item 3 (the verify line goes silent on an all-unmapped memo — §1's own class, live,
missed by four drafts), §7's understated debt and coupling count, §9's conflated axis count, §10-Q3's wrong
control, §11's category and missing ledger fields, and §4.1's mis-titled table.

**Reviewer claims re-derived and rejected**: round 1's "`coverage_map_label` has more than one caller";
round 2's "`elidex-wt-c4fix/docs/plans/` is empty" (the substance — that the branch *authors* none — held);
round 4's decisive-finding **consequence**, falsified by measurement (§0); round 5's "`python-suites.sh`
carrying skill paths is a generic-core violation" (it is repo infrastructure, where four trip-wire scripts
already live) and "the temporaries guard becomes vacuous in A" (equally subject-less before and after, since
`_catalog` is callable and the guard excludes callables).

---

## §15 Re-derivation

Every quantity this memo relies on, as a command. Nothing here is a stored value: a reviewer runs the block
rather than trusting a number, and a later draft re-runs it rather than editing digits. Run from this
worktree.

```sh
# 1  the two fixture citations (§0.5, §3)
.claude/tools/webref heading --exact html 4.10.21 ; .claude/tools/webref heading --exact html 4.10.21.2

# 2  the level-collision identity measurement, and the 195/8 partition by `webref heading` output (§0)
for s in pointerevents3 pointerevents4 cssom cssom-1 selectors selectors-4 \
         wai-aria wai-aria-1.2 wai-aria-1.3 webaudio webaudio-1.0 webaudio-1.1; do
  printf '%-16s %s\n' "$s" "$(.claude/tools/webref heading $s '' | md5)"; done

# 3  suite counts + urlopen calls on origin/main (§1, §4.3.1, §4.3.3)
T=$(mktemp -d); git worktree add -q "$T" origin/main
python3 -m unittest discover -s "$T/.claude/tools/_webref" -p 'test_*.py' -t "$T/.claude/tools"
python3 -m unittest discover -s "$T/.claude/skills/elidex-plan-review" -p 'test_*.py'
#    re-run with urllib.request.urlopen wrapped by a counting spy -> 0

# 4  origin/main anchors, by symbol (§3.1, §4.2, §5) — never stored as line numbers
git show origin/main:.claude/skills/elidex-plan-review/preflight.py | grep -n \
  'SECTION_REF_RE\|^def parse_spec_cell\|^def shortname_from_label\|^def verify_citation\|dest="grep_pass"\|unique_specs\|seen_pairs\|elif seen_pairs'

# 5  the §5 origin/main column — each fixture shape against a REAL origin/main worktree.
#    A sandbox without .claude/tools/webref reports verification failures that are artifacts;
#    that contamination is why draft 5 recorded row 13 as 1 when it is 0.

# 6  the 15->24 superset, the 9 added spellings, the equal value sets, and the landed memos carrying them
git grep -nE '^\| *(WebIDL|XHR|Fetch|Streams|WebCrypto|selectors-4|geometry-1|ecma262|ecma402) §' \
  origin/main -- docs/plans/

# 7  the re-carve commit — by subject, never sha (it has moved three times)
git log --format='%H %s' --grep='carve the cite-audit detector'
git diff --numstat origin/main...HEAD -- .claude/

# 8  spec_labels.py region boundaries (§4.0)
grep -n '^"""\|^SPECS\|^def \|^#: ' .claude/tools/_webref/spec_labels.py

# 9  the SystemExit escape that grounds §10-Q1(a)
python3 - <<'PY'
import sys, urllib.request, urllib.error, os
os.environ["XDG_CACHE_HOME"] = "/tmp/empty-cache"; sys.path.insert(0, ".claude/tools")
urllib.request.urlopen = lambda *a, **k: (_ for _ in ()).throw(urllib.error.URLError("offline"))
from _webref import spec_labels
try: print("returned:", spec_labels.shortname_for("CSS Text 3"))
except SystemExit as e: print("SystemExit ESCAPED _catalog():", e)
PY

# 10 the no-spec-surface marker census (§4.2.5)
git grep -n '^\*\*No spec surface\*\*' -- docs/plans/

# 11 the set J4's check must range over (§4.3.2)
git ls-files '.claude/**/test_*.py'

# 12 branch protection (§4.3.4)
gh api repos/send/elidex/rulesets --jq '.[] | {name, enforcement, target}'
gh api repos/send/elidex/rulesets/13294991 --jq '{rules: [.rules[].type]}'

# 13 line-count budget (§8)
for f in .claude/skills/elidex-plan-review/preflight.py .claude/tools/_webref/commands/coverage_map.py \
         .claude/tools/_webref/cli.py .claude/tools/_webref/DESIGN.md mise.toml .github/workflows/ci.yml; do
  echo "$(git show origin/main:$f | wc -l) $f"; done

# 14 the §11 subprocess-vs-in-process timing
#    100 reps of verify_citation vs resolver.lookup_section on one (shortname, section) pair

# 15 lane state at landing (§13) — base, open PRs, and the worktrees authoring plan-memos
git rev-list --left-right --count origin/main...HEAD ; gh pr list --state open
for w in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
  n=$(git -C "$w" diff --name-only origin/main...HEAD -- docs/plans/ 2>/dev/null | wc -l)
  [ "$n" -gt 0 ] && echo "$n $w"; done
```
