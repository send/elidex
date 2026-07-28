# Plan — Slice A: one spec-label map, landed fail-closed, with a scheduler that runs its suites

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A**. Under that umbrella's approval
boundary this is a **terminal unit** — not re-split for touching the same subsystem as B/C.
**Branch**: `webref-cite-audit-tool`, after the §4.0 re-carve. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`.
**Base**: `db96f231`, 0 behind. **Nature**: developer tooling + CI topology. Zero `crates/**` diff.
**Status**: plan-memo, **draft 5**. `/elidex-plan-review` **required before implementation**.

⚠ **Draft 5 is a scope change, not a patch round.** Four review rounds; round 4's conclusion was that the
*slice boundary* was still wrong. **A takes the deduplication and drops the widening**: `spec_labels.py`
lands **pinned-map-only**, and the 948-entry catalog fall-through moves to Slice B, which already owns the
lookup semantics that make it correct (B §4.1.2 / §4.1.7 / §4.1.8, unchanged — B's memo needs no edit).
§4.2's ground for the change is stated there; the rounds are indexed in §14.

⚠ **Round 4's decisive finding rested on a consequence claim that does not survive measurement, and this
memo says so rather than inheriting it.** The claim (B §4.1.8, repeated into round 4) is that a catalog
level-collision means *"citation verification silently runs against the wrong document"*. Re-derived
2026-07-29 — **every level-collision pair B names resolves to byte-identical heading data**:

```sh
for s in pointerevents3 pointerevents4 cssom cssom-1 selectors selectors-4 \
         wai-aria-1.2 wai-aria-1.3 wai-aria webaudio-1.0 webaudio-1.1 webaudio; do
  printf '%-16s %s\n' "$s" "$(python3 .claude/tools/webref heading $s '' | md5)"; done
```
→ one md5 per *series*, not per level: webref's `ed/` extracts are keyed to the series' current spec, so
`cssom` and `cssom-1` are the same document. Of the 203/948 non-round-tripping shortnames, **195 are
same-document series aliases and 8 land on a different document** — all cross-series or fork cases
(`DOM-Level-2-Style`→`dom`, `wasm-js-api*`→ the CSP fork, `rfc6265bis`→`layered-cookies`, three FIDO CTAP
versions), none a label an elidex memo writes. The resolver ambiguity B reports is real; **the danger
attributed to it is not**. A therefore drops the widening on the boundary argument in §4.2, not on a
scare. B's §4.1.8 consequence sentence is B's to correct — §13 hands it over rather than editing it here.

### §0.1 What Slice A is, in one sentence

`origin/main` carries the same enumeration three times — `preflight.SPEC_LABEL_REVERSE` (15 keys),
`coverage_map._SPEC_LABEL_MAP` (12) and `cli.COMMON_SHORTNAMES` (a 12-line help block). Slice A collapses
them onto one `.claude/tools/_webref/spec_labels.py`, and **because that import is the first thing that
can make the plan-review gate's label resolution *fail*, lands it fail-closed from the start** — then gives
the resulting suites a scheduler, because today nothing runs them. It ships no detector (B), edits no
review policy (C), and **changes no resolution outcome** (§5).

---

## §0.5 Spec citation table

This slice implements no spec logic. The two citations below are the rows the new `test_preflight.py`
fixture memos carry; both looked up with `.claude/tools/webref` on **2026-07-28**, nothing from memory.

| Cite | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| the labelled fixture row (P2/P3 — a row whose spec label maps) | HTML §4.10.21 | Constraints | `#constraints` | `heading --exact html 4.10.21` |
| the second labelled row, so `seen_pairs` dedup is exercised | HTML §4.10.21.2 | Constraint validation | `#constraint-validation` | `heading --exact html 4.10.21.2` |

**P4 needs a *separate* fixture memo, not a third row of the one above.** Its §3 rows are **all**
label-less (`| §4.10.21 Constraints | … |`, each cell opening with `§`). This matters mechanically: a memo
containing *any* labelled row hard-fails under both the correct placement and the mis-sited one, so P4
would pass vacuously. Two fixture memos ship — `labelled.md` and `unlabelled.md`. Neither row is a citation
defect; the label-less shape is the input that falsifies §4.2.2's placement.

⚠ **This table certifies fixtures, not the slice.** `preflight` prints `citation verify: ok (2 unique
citation(s) checked)` for a slice with **zero spec surface**, because `origin/main` hard-fails a memo with
no §3 heading (`:307-314`), no table (`:319`) and **0 data rows (`:334`)** — so there is no accepted input
shape that declares "no spec surface" and passes. That is §1's anchor violated in A's own file, and **A
fixes it** (§4.2.5). A's own memo cannot use the fix — plan-review runs against `origin/main`'s
`preflight.py`, before A is implemented — so this ⚠ stands for the life of this memo.

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
2. **Nothing runs the suites.** On `origin/main` there are **47 tests across 4 files** (verified 2026-07-29) under no `mise` task,
   no CI job, no hook (§4.3.1, measured). An unscheduled suite is a claim with no checker — the shape this
   program exists to remove.

The corollary that drives the edit set: **a capability is a process-level fact and must be established
once, before the data loop.** "I cannot map *this* label" is a datum about one row. "I cannot map *any*
label" is a fact about this process. Discovering the second by watching the first makes the failure look
like data — and, as §4.2.2 measures, makes the fix's correctness depend on the *content* of the memo being
reviewed. Draft 4 could not obey this corollary, because the catalog fall-through it also shipped adds a
cause that can only materialise *during* lookup. Dropping the widening restores it (§4.2.3).

---

## §2 Coupled invariants

- **J1 — capability ≠ datum.** A row is *unmapped* only if the mapper ran and declined. If the mapper is
  absent, no row is unmapped; the run is uncertified. One return value (`None`) must not carry both.
- **J2 — the two capabilities must degrade the same direction.** Verifying a citation needs the `webref`
  CLI *and* the label map. Measured on the naive carve, one hard-fails and the other exits 0 (§4.2.1); its
  in-code comment claims they "degrade the same way". They do not.
- **J3 — one degradation must survive.** `--no-verify --no-grep-pass` (structure + breadth only) must keep
  working with the tools tree absent. It is the property a fail-closed change is most likely to break.
- **J4 — one enforcement mechanism, not two.** If `mise` and `ci.yml` each spell the suite invocation, a
  later suite is added to one and not the other. `trip-wires` already answers this: the script is the SoT
  and each runner is a caller.
- **J5 — A adds no network dependency.** The plan-review gate is run by every lane and `mise run ci` is the
  mandatory pre-push gate. Both causes in J1 are static process facts; neither requires a fetch. This is the
  invariant the widening broke, and it is pinned, not asserted (§6 P9).

J1–J3 live in `preflight.main`'s control flow and cannot be applied one at a time without transiently
breaking each other, which is why §5 measures the configuration matrix rather than a sample. J4 is
independent. J5 is a property of the whole slice and is checked by running A's entire suite set offline.

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
data. The inputs are the plan-memo's path *and its content*: `parse_spec_cell` (`preflight.py:216-229`)
extracts a label and a section number from memo cell text, and `verify_citation` (`:240-263`) passes
**both** to a subprocess. §4.2.2's finding is that memo content steers control flow, so listing only the
path omits the input that section proves is load-bearing.

**Both argv elements stay bounded, and — unlike draft 4 — A does not move either bound.** `section` is
bounded by `SECTION_REF_RE = r"§\s*([\d.A-Z]+)"` (`:77`), untouched. `shortname` is bounded on
`origin/main` by the 15-key `SPEC_LABEL_REVERSE` and after A by the 24-key pinned `LABEL_TO_SHORTNAME`, a
**strict superset of the same closed set** (§5). Draft 4 replaced that bound with a 948-entry
third-party document fetched at gate time, on every plan review in every lane; that exposure delta —
outbound *and* inbound — leaves with the widening.

**Discovery method.** Every number below was produced by **executing** code on 2026-07-28/29, against the
correct baseline, stated per measurement:

1. `origin/main` facts come from `git show origin/main:<path>` or a throwaway `git worktree add
   origin/main`, never from the branch (§14 C1 is what happens otherwise).
2. The gate asymmetry is a three-case sandbox run (§4.2.1), one dependency removed per case.
3. The re-siting defect (§4.2.2) was found by **applying draft 1's own fix in that sandbox** and running it
   against two memos — a measurement of a proposed patch, not a reading of one.
4. Branch/lane facts use **three-dot** ranges (`origin/main...<branch>`); two-dot reports `main`'s own
   commits as a branch's (§14 C4).
5. Claims inherited from another slice's memo are re-derived before being relied on (§0's second ⚠ is what
   that produced this round).

---

## §4 The edit set

### §4.0 Step 0 — re-carve `26721cfa` on the seam the umbrella already draws

```sh
git diff --numstat origin/main...HEAD -- .claude/     # identical under `..`, so the branch is not behind
```

| File | +/− | Half |
|---|---|---|
| `_webref/spec_labels.py` | 136/0 | **split** — see the line map below |
| `skills/elidex-plan-review/preflight.py` | 20/30 | **A** — drops local `SPEC_LABEL_REVERSE`, imports the map |
| `_webref/commands/coverage_map.py` | 15/21 | **split** — the delegation to `label_for` is A; the changed last-resort is B |
| `_webref/cli.py` | 43/13 | **split** — the blurb derivation is A; the `cite-audit` subparser + import + example line are B |
| `_webref/DESIGN.md` | 23/0 | **split** — the `spec_labels.py` bullet is A minus its catalog sentence; the `cite_audit.py` adapter bullet, CLI examples and three-bucket paragraph are B |
| `_webref/test_cite_audit.py` | 410/0 | **split** — `TestSharedSpecLabelMap`'s first **8** tests (`:206-296`) become A's `test_spec_labels.py`; `test_coverage_map_fallback_round_trips` (`:309-314`) + the `coverage_map_label` helper (`:317-321`) are **B**; the remaining 10 classes are B's |
| `_webref/commands/cite_audit.py` | 289/0 | **B** — the detector |
| `_webref/sources/webref_data.py` | 9/0 | **B** — `@lru_cache` motivated by the detector's per-section loop; B §4.1.6 rewrites this area |

**`spec_labels.py`, line by line** — the split is inside the file, which is why it is mapped rather than
assigned:

| Lines | Content | Half |
|---|---|---|
| 1-14, 24-77 | module docstring's drift rationale; `SPECS`; `SHORTNAME_TO_LABEL` / `SHORTNAME_TO_BLURB` / `LABEL_TO_SHORTNAME` | **A** |
| 15-22 | the *"`SPECS` is a fallback, not the source"* docstring paragraph | **B** |
| 80-92 | `_catalog()` and its `from .sources.webref_data import _data_index` | **B** |
| 95-109 | `label_for` — the `SHORTNAME_TO_LABEL` lookup is A; the catalog branch (`:104-109`) is B | **split** |
| 112-136 | `shortname_for` — the pinned lookup (`:122-127`) is A; the catalog branches (`:128-136`) and the CSS-module docstring paragraph are B | **split** |

`coverage_map._spec_label` is the same shape: A ships `return label_for(shortname) or
shortname.upper().replace("-", " ")` — **`origin/main`'s last-resort verbatim**, so `coverage-map` output
is byte-identical for every input. The branch's changed last-resort (`or shortname`) is only correct
*together with* the catalog and B §4.1.8's round-trip rules, which is why it travels with them.

⚠ **The prose needs its own pass.** Five sites in the A column describe `commands/cite_audit.py` as
extant — `spec_labels.py:3-6`, the `DESIGN.md` bullet, `preflight.py:48-51`'s new comment, and the moved
tests' docstrings at `test_cite_audit.py:198-204`/`:245`. `cite_audit.py` is **absent from `origin/main`**,
and only three copies of the map exist there, so the docstring's "Four sites" is branch-relative and
becomes **three**. A filename-only purity check passes while every one of these is present, which is why
§12 (3) carries a content assertion.

Result: `webref-cite-audit-tool` = `origin/main` + the A column + A's edits; a new branch for B = A's
landed head + the B column. **B's memo already assumes this** ("Branch: new, cut from Slice A's landed
head").

**Why A takes the map's pinned half rather than leaving the whole carve to B**: the import is what
*creates* the failable capability. If B lands it, `main` carries a fail-open plan-review gate — a gate
every lane runs — for the duration of B. A landing it fail-closed means the defect is **never introduced**,
which is strictly better than introducing and repairing it.

### §4.1 What A deliberately does not touch

| Concern | Slice | Why not A |
|---|---|---|
| **the catalog fall-through, the discriminated `_catalog()`, the reverse index and the round-trip rules** (B §4.1.2 / §4.1.7 / §4.1.8) | **B** | §4.2's boundary. A lands the map's *shape*; B owns its lookup semantics — and the fall-through **is** lookup semantics, which is what draft 4 contradicted by shipping it |
| `coverage_map`'s changed last-resort, and `test_coverage_map_fallback_round_trips` | **B** | the property it asserts is only true once the catalog and B §4.1.8's rules are in |
| `cite_audit.py`, `test_cite_audit.py`, the `cite-audit` subparser, `webref_data.py`'s memo | **B** | the detector; §4.0 routes them |
| `spec_labels`'s public surface reduction (`project_pr-a0-review-ledger` #25) | **B** | its stated root is `cite_audit.py:36` indexing `LABEL_TO_SHORTNAME` directly instead of calling `shortname_for`. Reducing the surface in A would also trip the shipped `test_module_leaves_no_temporaries_to_delete` guard, which forbids module-level private non-callables |
| `.claude/skills/elidex-plan-review/SKILL.md` — `Hard-fail conditions` (`:113-114`), `--no-verify`'s documented meaning (`:116`), Pre-condition #1 (`:30-32`) | **A** | A adds a hard-fail cause, gives `--no-verify` a second role (capability suppressor), and adds the no-spec-surface declaration. No other slice claims this file |
| one shared `SECTION_NUMBER_RE` across `preflight` / `cite_audit` / `section_sort` | **B** | `preflight.SECTION_REF_RE` is A's file but B's grammar unification; A leaves it byte-identical so B's collapse is one edit, not a merge |
| `axes.md` (2)/(4); `CLAUDE.md` § "Spec citation"; `DESIGN.md`'s reported-class contract | **C** | retiring a discovery method rests on a reach measurement only B can produce |
| the `crates/**` citation repairs and the 8 newly-authored wrong citations | **D** | content, not plumbing |

### §4.2 A1 — land the capability fail-closed

#### §4.2.1 The measured asymmetry

Measured against the carve as authored (`26721cfa`), in a sandbox repo skeleton so `REPO_ROOT` resolves,
with `--no-grep-pass` throughout (the sandbox's `REPO_ROOT` is the sandbox, so grep-pass reports 44 hard
findings for `crates/**` paths that do not exist there — an artifact, not a finding):

| Case | Removed | Result | Exit |
|---|---|---|---|
| **A** | nothing | 21 rows, 21 parsed citations, **15 unique citations verified** | **0** |
| **B** | `.claude/tools/webref` (pre-existing check) | `❌ HARD FAIL — citation verification: 15 failure(s)` | **1** |
| **C** | `.claude/tools/_webref` (the new import) | `parsed citations: 0`, `unmapped-label rows: 21`, **no verify section at all** | **0** |

`15`, not `21`: `seen_pairs` (`:382-388`) dedups 21 data rows to 15 unique `(shortname, section)` pairs.
Case C also emits a **wrong-cause remedy** — `(add the spec to
.claude/tools/_webref/spec_labels.py::SPECS)`, the file that failed to import. An author following it edits
a file the gate cannot read.

**Case C does not exist on `origin/main`**: there, `shortname_from_label` (`:239-249`) reads a module-local
dict with no import to fail. The asymmetry is created by moving the map, which is why the slice that moves
it owns the fix.

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

#### §4.2.3 The fix — two static causes, one verdict, computed before the loop

**Why this is smaller than draft 4's.** Draft 4 had a *third* cause: with the catalog fall-through, an
offline lookup could die inside the row loop (B §4.1.7's `SystemExit` escape). Because that cause can only
materialise *during* lookup, draft 4 had to aggregate the verdict across the loop, add an `UNCERTAIN` arm
and two accumulators, and thread a cause out of `spec_labels`. Dropping the widening deletes the cause, and
with it all of that machinery — leaving the design §1's corollary actually prescribes:

1. **Two causes, both static process facts, evaluated once before the loop**: `WEBREF.is_file()` and
   `_shortname_for is None`. The capability verdict is their union, computed at `main`'s top.
2. **`shortname_for` stays `str | None`.** No tri-state, no `resolve_label`, no discriminated `_catalog()`.
   Everything `spec_labels` does is a dict lookup over a pinned table.
3. **The row loop keeps its shape and its arms** — MAPPED and UNKNOWN, exactly as on `origin/main`. This is
   deliberate: draft 2 proposed *skipping* the loop when the capability was absent, which strands
   `unique_specs` (`:357`/`:361`), `specs_seen`, and — in the wider spelling — `malformed_rows`. Measured
   with the skip applied on a 7-spec/7-row fixture: `K` 7 → **0**, `⚠ SPLIT-DEFAULT` → `ok (single PR
   scope)`, `--strict-breadth` exit 1 → **0**. `SKILL.md:118` makes that split decision a stop-and-ask-user
   gate, so the skip silently disables a *different* gate than the one being fixed.
4. **Unavailable + verification requested → HARD FAIL**, in the same `❌ HARD FAIL — …` shape as the other
   three, naming each absent cause and `--no-verify` as the suppressor. **Unavailable + `--no-verify` →
   exit 0** (J3).
5. **The breadth line states its basis.** With the capability absent every row takes the unmapped arm, so
   `unique_specs` is keyed by label spelling rather than shortname — a memo whose rows alias one spec
   counts *n* instead of 1. Rather than claim `K` is capability-independent (draft 3 did; it is not) or
   invent a third key space (draft 4 did), A **prints the basis**: whenever `unmapped_rows > 0` or the
   capability is absent, the breadth line reads `K=<n> (unresolved — counted by label spelling)`. The
   pre-existing partial-unmapped case gains the same honesty. This is a checkable claim (§6 P3), which is
   what draft 4's open question Q4 was not.
6. `shortname_from_label` keeps exactly one job — classify a label — and its `_shortname_for is None`
   branch is replaced by a hard precondition rather than deleted, so no second site answers the capability
   question and no path calls `None(label)`.

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is currently re-tested
inside `verify_citation` on **every unique citation** — 15 times in case A — reporting one process-level
fact as 15 per-citation failures. After the hoist, case B's exit code is unchanged (1) and its diagnostic
is one line naming the missing path. The guard inside `verify_citation` becomes an **explicit raise**, not
an `assert`: under `python3 -O` an assert is stripped and a direct caller would get exactly the silent
non-zero this change exists to remove.

#### §4.2.4 The remedy text

**Four** strings, currently one, because there are four ways to fail and the author's next action differs
in each. (Draft 4 had five; the catalog-unreachable string leaves with the widening.)

| Condition | Remedy |
|---|---|
| genuinely unmapped label | "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the label spelling" |
| **label-less cell** (`\| §4.10.21 … \|`) | "the Spec section cell must open with a spec label" — measured: today this row prints the `SPECS` advice against `<empty>`, advice that cannot be acted on, the same wrong-cause class as case C |
| tools unavailable (import failed) | the import error and the path attempted, plus `--no-verify` |
| CLI missing | the expected path, plus `--no-verify` |

#### §4.2.5 A5 — let a slice declare that it has no spec surface

`origin/main` hard-fails a §3 table with 0 data rows (`:334`) and a memo with no §3 heading (`:307-314`),
so a slice implementing no spec logic must author fixture citations and then receives `citation verify: ok
(2 unique citation(s) checked)` as its headline — a verdict about fixtures presented as a verdict about the
slice. §1's anchor, in A's own file.

**Ownership.** Draft 2 routed this to B (wrong — the umbrella forbids B editing review policy); draft 3 to
C on the ground that `axes.md` holds the authoring contract (**also wrong** — the "every plan-memo MUST
contain a §3 table" contract is at `.claude/skills/elidex-plan-review/SKILL.md:30-32`, Pre-condition #1,
which §4.1 already assigns to A, and `axes.md` carries no §3-table requirement). Two owners who could not
perform the fix is the signal that the deferral was the error. It stays in A: A already edits both
`preflight.py` and `SKILL.md`, and no other slice will.

**Specification** — round 4's finding was that draft 4 declared this without specifying it, leaving the
verdict and the bypassed table block undefined. Fully:

- **Accepted shape**: the `## §3. Spec coverage map` heading stays **required**. Its body may contain, in
  place of a table, one marker line with a literal greppable prefix: `**No spec surface** — <reason>.`
- **Mutual exclusion**: marker *and* a table both present → **HARD FAIL** (ambiguous declaration). This is
  what stops the marker from silently bypassing the table block.
- **Verdict**: `citation verify: n/a (no spec surface declared)` and `breadth: n/a (no spec surface
  declared)` — not `ok`, not `0`. Draft 4's version printed `ok`/`0` vacuously, which is round 2's D2 one
  level up.
- **Every other gate runs unchanged**: fence-state parsing, structural checks, and the grep-pass.
- **Interaction with the capability verdict**: the verdict is still computed (it is static) and still
  printed, but **cannot** hard-fail here, because no citation was requested and none was suppressed.
- **Residual, stated rather than argued away**: an author can declare no spec surface to skip citation
  verification. It is author-controlled — but deliberate, greppable in one command
  (`git grep -n '^\*\*No spec surface\*\*' -- docs/plans/`), printed prominently by the gate, and
  `/elidex-plan-review`'s Axis 4 reads the memo regardless. It is not a silent bypass, which is the
  property that matters.

### §4.3 A2 — give the suites a scheduler

#### §4.3.1 The hole, measured on `origin/main` across all three workflows

- `ci.yml`'s `changes` filter has two sets: `rust` (`crates/**`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `.rustfmt.toml`, `clippy.toml`, `mise.toml`, `.github/workflows/**`) and `config`
  (`deny.toml`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/**`). **`.claude/**` is in neither**, and
  all three jobs (`check`, `doc`, `deny`) are gated on one of the two.
- `ci.yml` **never invokes `mise`** — the single `mise` string in the file is `mise.toml` as a filter entry.
- `codeql.yml` analyses `[actions, rust]` on push-to-main + a weekly cron — **no Python, no `pull_request`
  trigger**. `audit.yml` is `cargo audit` on a weekly cron.

⇒ a `.claude/**`-only pull request triggers **zero jobs**, and even the post-merge push runs only cargo.

```sh
# verified 2026-07-29 in a throwaway worktree so the branch cannot leak in
T=$(mktemp -d); git worktree add -q "$T" origin/main; cd "$T"
python3 -m unittest discover -s .claude/tools/_webref -p 'test_*.py' -t .claude/tools    # Ran 12, OK
python3 -m unittest discover -s .claude/skills/elidex-plan-review -p 'test_*.py'          # Ran 35, OK
```

**47 tests across 4 files** (verified 2026-07-29 by the block above; `test_inventory_diff` 6, `test_agent_brief` 5, `test_refresh` 1,
`test_grep_pass` 35). A adds `test_spec_labels.py` (8 tests moved by §4.0) and `test_preflight.py` (§6), so
A's landed figure is 47 + 8 + P-count. Draft 1's "83 across 5 files" was the *branch* figure (verified 2026-07-28 by running the same two commands without the `origin/main` worktree), which counts
the 28 detector tests that are Slice B's.

#### §4.3.2 The mechanism — one script, two callers (J4)

`.claude/tools/python-suites.sh`, `set -euo pipefail`, `cd "$(dirname "$0")/../.."`, then the two
`discover` lines above (both verified to collect their full sets).

- `mise.toml` gains `[tasks.tools-test]` = `bash .claude/tools/python-suites.sh`, added to
  `[tasks.ci].depends`.
- `ci.yml` gains a `tools` path-filter set (`.claude/tools/**`, `.claude/skills/**`,
  `.github/workflows/**`) and a `tools` job on `ubuntu-latest` running the same script under the same
  `|| github.event_name == 'push'` bypass the other three jobs use.
- The script **derives its own suite set and fails loudly when a `test_*.py` lands outside the filtered
  paths**, so "script is SoT, runners are callers" is enforced rather than documented.

This is the `trip-wires` shape verbatim (`mise run trip-wires` calls four `.claude/tools/*.sh`), so it
introduces no new pattern.

#### §4.3.3 The network question — answered by construction, not by disposition

Measured with a spy on `urllib.request.urlopen` (same throwaway worktree, both `discover` runs):
**`URLOPEN_CALLS=0`** for all 47 `origin/main` tests.

A's 8 moved tests exercise `spec_labels`'s pinned dicts, `coverage_map._spec_label` and
`preflight.shortname_from_label`; under §4.0's split none of them reaches `sources/webref_data`, because
`spec_labels.py` no longer imports it at all. **A's suite set therefore fetches nothing** — predicted (A is
unimplemented), and pinned by **P9**, which runs A's entire suite set with `urlopen` patched to raise.

This is the concrete payoff of the scope change. Draft 4 measured **1** fetch per run
(`raw.githubusercontent.com/w3c/webref/main/ed/index.json`, 1,572,569 B), had to argue it was acceptable in
`mise run ci` — CLAUDE.md's *mandatory* pre-push gate — and opened a deferral (`cleanup-webref-suites-offline`)
plus an umbrella obligation to make it survivable later. All three disappear. What replaces them is one
forward-binding constraint A adds to the umbrella's "Constraints each slice inherits":

> **No slice may make label resolution require the network without shipping its offline degradation in the
> same slice.** Slice B introduces the catalog fall-through and therefore owns the offline contract for it
> (B §4.1.7).

A constraint, not a deferral — it binds B at authoring time rather than recording an unowned concern.

⚠ **What A does *not* claim**: that the plan-review gate becomes offline-capable. It is not, and was not.
`verify_citation` shells out to `webref heading`, which issues a conditional GET, and `cache.py:130-131`
`sys.exit`s on `URLError` — so **`origin/main`'s gate already requires the network in default mode**,
before and after A. A's claim is narrower and exact: *A adds no network requirement that was not already
there*, and the `--no-verify` degradation (J3) stays offline-clean.

#### §4.3.4 What "enforced" can honestly mean here

```sh
gh api repos/send/elidex/rulesets --jq '.[] | {name, enforcement, target}'
gh api repos/send/elidex/rulesets/13294991 --jq '{rules: [.rules[].type]}'
```

`main` is governed by an **active** ruleset `main-protection` (id 13294991, target `~DEFAULT_BRANCH`) whose
rules are `deletion` / `non_fast_forward` / `pull_request`. There is **no `required_status_checks` rule**,
so a red `tools` job does not block a merge; CLAUDE.md's workflow ("CI 全 pass を目視確認してから squash
merge") is the blocking step, and it is a human one. (`gh api …/branches/main/protection` → 404 is the
**deprecated legacy endpoint** and means "not protected *via the legacy API*", not "unprotected".)

The claim A may make: the job makes a regression **visible, attributed, and on the PR page at review
time**, where today it is invisible in every event. That is what §12 asserts — no more.

#### §4.3.5 The interpreter floor

Measured: **no `.claude` Python source uses syntax newer than 3.9** (`match`, `except*`, `tomllib`,
`typing.Self`, `ExceptionGroup`, atomic groups — all absent). Local dev is 3.14.6. Nothing in the
repository declares a floor.

`python-suites.sh` asserts `sys.version_info >= (3, 9)` — **A's own measured need** — and the job echoes
`python3 -VV`, so the runner's actual version becomes a measured fact on the first CI run instead of an
assertion here. B raises the floor when B lands `(?>...)`; that is one line in a file B is already editing.
Note `SKILL.md:110` invokes `preflight.py` directly, bypassing the script — unaffected today (A adds no
version-dependent syntax) and marked UNCHECKED in §5.

### §4.4 A3 — site the label-map tests where they belong, from the start

§4.0 moves `TestSharedSpecLabelMap`'s 8 A-tests into `test_spec_labels.py`. One assertion inside
`test_all_three_consumers_derive_from_specs` does not belong there either:

```python
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "skills" / "elidex-plan-review"))  # :289-292
preflight = importlib.import_module("preflight")                                                # :296
```

The *generic tools* package's test hard-codes the *elidex skill's* directory layout and module name — the
one **import-time executable** edge that blocks `DESIGN.md`'s goal of keeping the drift-detection core
movable to a standalone repository. (It is not the only elidex reference in that tree; see §7's second ⚠.)

**Fix**: the `preflight` half goes to `.claude/skills/elidex-plan-review/test_preflight.py`, beside
`preflight.py` and `test_grep_pass.py` — the home exists and the dependency direction is right (consumer
depends on library). `test_spec_labels.py` keeps the `coverage_map` half with a module-top-level import. No
`sys.path` mutation survives inside any test method. The `coverage_map_label` helper is **not** collapsed
here — its only caller is `test_coverage_map_fallback_round_trips`, and both go to B (§4.0).

Because §4.0 makes this a *split of an unlanded file* rather than an edit to a landed one, the docstrings
describing the three-consumer guard travel with the assertion instead of being left behind.

### §4.5 Test-siting constraints the plan must state, not discover

1. **`_shortname_for` is bound at module import**, so "make the import fail" cannot be done by removing
   `.claude/tools` from `sys.path` and reloading — `preflight.py:56` **re-inserts that directory on every
   import**, so the module under test re-establishes the capability the test is removing. Working
   mechanisms are a `sys.modules`/`__import__` hook plus `importlib.reload`, or a subprocess; they pin
   different lines. An in-process `preflight._shortname_for = None` pins the new precondition but leaves
   `:56-60`'s `except Exception` **mutation-green**. **P2 uses the reload form** and P2b adds a subprocess
   case, so both the guard and the precondition are pinned.
2. **P1 needs `_shortname_for` bound; P2/P3/P4 need it `None`** — mutually exclusive process-global state
   in one file, and reloading does not restore it. `test_preflight.py` restores the module in `tearDown`
   via `importlib.reload` under the un-patched import, and P1 asserts the bound state at `setUp` so a leak
   fails loudly instead of silently inverting. `unittest` orders methods alphabetically, so relying on
   names is not a plan.
3. **The isolation contract is exactly these two modules — `preflight._shortname_for` and `sys.path`.**
   Draft 4 added `sources/webref_data._INDEX` and `webref_data.try_fetch_data.cache_clear()`; both leave
   with the widening, and the second was wrong anyway (`origin/main`'s `try_fetch_data` carries **no**
   `@lru_cache` — that decorator is a hunk §4.0 routes to B, so draft 4's head could not satisfy its own
   contract without pulling B's file).

---

## §5 Behavior deltas, claims, and their pins — one table

Round 4's finding was that §5 / §6 / §4.6 / §12(2) were four spellings of one table, and that every
section-contradicts-section defect across rounds 3-4 was a fact stated at *N sites*. They are one table
here. **Baseline is `origin/main`**, not the carve: the carve is an intermediate artifact that never lands,
and its measured case C (§4.2.1) is cited once, as the reason the design exists, not as a column.

**Axes**: CLI present/missing × label map importable/not × mode `default` / `--no-verify`, with the memo's
§3 label shape (labelled / label-less) discriminating wherever classification consults a label. The two
capability causes are a **union**, so any combination of absent causes yields one verdict; what differs is
the **diagnostic**, not the exit code. Every measured row ran with `--no-grep-pass`, because the sandbox's
`REPO_ROOT` is the sandbox and grep-pass reports 44 artefact hard-findings there; `--no-grep-pass` is
**not** the default (`dest="grep_pass", default=True`, `:275-278`), so the `mode` column is the *verify*
axis only.

**On `origin/main` the "module" axis does not exist** — the map is a module-local dict with no import to
fail — so those rows read `n/a`. That is the honest baseline statement, and it is why A's newly-red rows
are new capability states rather than regressions of existing ones.

| # | CLI | map | mode | §3 labels | `origin/main` | After A | Pin | Detects the naive carve? |
|---|---|---|---|---|---|---|---|---|
| 1 | ✓ | ✓ | default | labelled | 0 (15 verified) | **0** | P1 | — |
| 2 | ✓ | ✓ | `--no-verify` | labelled | 0 | **0** | P1 | — |
| 3 | ✗ | ✓ | default | labelled | 1 (15 per-citation failures) | **1** — one diagnostic line | P6 | ✓ |
| 4 | ✗ | ✓ | default | label-less | **0** (`citations` empty ⇒ verify block skipped) | **1** | P4 | ✓ |
| 5 | ✗ | ✓ | `--no-verify` | either | 0 | **0** — capability unused | P3 | — |
| 6 | ✓ | ✗ | default | labelled | n/a | **1** | P2, P2b | ✓ |
| 7 | ✓ | ✗ | default | label-less | n/a | **1** (§4.2.2) | P4 | ✓ |
| 8 | ✓ | ✗ | `--no-verify` | either | n/a | **0** (J3) | P3 | ✓ |
| 9 | ✗ | ✗ | default | any | n/a | **1**, diagnostic names both causes | P6 | ✓ |
| 10 | ✓ | ✓ | default | no-spec-surface marker | **1** (0 data rows, `:334`) | **0**, `verify: n/a` | P11 | ✓ |
| 11 | ✓ | ✓ | default | marker **and** table | **1** (table wins, rows verified) | **1** — ambiguous declaration | P11b | ✓ |

**No row moves 1 → 0 except row 10**, and there the red was the gate rejecting a valid input shape, not a
defect being hidden. Draft 3 asserted this as a blanket invariant and draft 4's catalog rows falsified it;
without the catalog the invariant is true again, with the one named exception.

**Newly-red**: rows 4, 6, 7, 9, 11. All require an absent capability or an ambiguous declaration. §13's
landing checklist re-runs the gate per worktree rather than arguing their reachability from here.

**Measured vs predicted**: the `origin/main` column is measured (rows 1/3/5 in the §4.2.1 sandbox, row 4
this round, rows 10/11 against `origin/main`'s hard-fail sites). The *After A* column is **predicted** by
construction; the Pin column is what converts each prediction into a check.

### Claims that are not rows

| Claim | Check |
|---|---|
| **A adds no network dependency (J5)** | **P9** — A's entire suite set under `urlopen` patched to raise, exit 0 |
| A changes no resolution outcome | **P12** — `LABEL_TO_SHORTNAME` is a strict superset of `origin/main`'s `SPEC_LABEL_REVERSE` (15 keys, identical values, verified) and `coverage_map._spec_label` is byte-identical for every input |
| The breadth line states its basis when rows are unresolved | **P3** asserts the `(unresolved — counted by label spelling)` qualifier |
| Consumers derive from `SPECS` | **P1** + `test_spec_labels.py`'s `coverage_map` half |
| The remedy text names the right cause | **P5** (four strings, each for its own cause and no other) |
| The suites run at all | `mise run tools-test`; the GitHub `tools` job — §12 (1)/(5) |
| A carries no part of B | §12 (3), ranged against the re-carve — **UNCHECKED until §4.0 is performed**; fails at today's head by construction |
| The interpreter floor holds on the runner | `python-suites.sh`'s assert — **only on the script path**; `SKILL.md:110`'s direct invocation is **UNCHECKED** |
| A red `tools` job prevents a merge | **UNCHECKED and false** — no `required_status_checks` rule (§4.3.4). What is checked is visibility |
| The 2026-07-28/29 counts here | **Re-derivable, not pinned** — each ships its command; §12 depends on none of them |

---

## §6 Test plan

Two fixture memos ship (§0.5): `labelled.md`, `unlabelled.md`; plus `nospec.md` and `nospec-and-table.md`
for rows 10/11. Each pin below names its *mechanism*; its expected values are §5's row, stated once.

**`.claude/skills/elidex-plan-review/test_preflight.py`** (new):

- **P1** the `preflight.shortname_from_label(label) == short` derivation assertion, moved from
  `test_cite_audit.py:275`, with no `sys.path` mutation in the test body and a `setUp` assertion that the
  module is un-poisoned (§4.5 item 2).
- **P2** map unimportable, via the `importlib.reload`-under-import-hook form (§4.5 item 1), `tearDown`
  restoring both the module **and** `sys.path`.
- **P2b** the same via a subprocess, pinning `preflight.py:56-60`'s `except Exception`. Mutation check:
  deleting that clause must turn P2b red — P2 alone leaves it green.
- **P3** `--no-verify --no-grep-pass` with the map absent, asserting exit 0 **and** the breadth-basis
  qualifier. The qualifier assertion is what makes §4.2.3 item 5 a check rather than a claim; without it
  the draft-2 loop-skip regression is invisible.
- **P4** the **label-shape independence property**: `labelled.md` and `unlabelled.md` produce the *same*
  exit code in every capability state (rows 4/6/7). This is what the mis-sited placement breaks, and it
  pins the property directly rather than vendoring the rejected patch as a fixture.
- **P5** each of the four remedy strings (§4.2.4) appears for its own cause and no other — including the
  label-less cell, which today prints the `SPECS` advice against `<empty>`.
- **P6** the missing CLI is reported once, not once per citation (rows 3, 9).
- **P9** the whole of A's suite set runs with `urllib.request.urlopen` patched to raise `URLError` and
  exits 0 — **J5**. Sited here rather than in `test_spec_labels.py` because it is a property of the slice,
  and it must fail if any later edit reintroduces a fetch on A's half.
- **P11** `nospec.md` → exit 0 with `citation verify: n/a` and `breadth: n/a` (row 10); asserts the strings,
  not just the exit code, so a hard-fail-removal that prints `ok` cannot pass.
- **P11b** `nospec-and-table.md` → exit 1 naming the ambiguity (row 11).
- **P12** `shortname_from_label` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, vendored as a
  literal in the test. This is the no-behaviour-change pin; a vendored literal is correct here precisely
  because the point is to freeze the *old* table.

**`.claude/tools/_webref/test_spec_labels.py`** (new by §4.0's split): `TestSharedSpecLabelMap`'s 8 A-tests
minus the `preflight` assertion, plus the `coverage_map` half at module-level import. Slice B appends its
catalog cases — and `test_coverage_map_fallback_round_trips` — to this file rather than creating it.

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** — not applicable; no `crates/**` diff.
**ECS-native** — not applicable; no component, no entity, no system.

**`DESIGN.md` generic-core / elidex-adapter split** — the live boundary:

| Edit | Layer | Placement |
|---|---|---|
| `spec_labels.py`'s `SPECS` table | **generic mechanism + pinned elidex conventions** | ⚠ not unqualified generic: `SPECS` pins *this repo's* `"WHATWG "` display prefix and the parse aliases that exist because "real comments and memos abbreviate". `DESIGN.md`'s closing rule puts elidex policy in adapters or documentation, so this is externalization debt A inherits from the carve and does not increase — recorded rather than asserted away |
| `commands/coverage_map.py` consumer | **generic** | second consumer of the shared map; last-resort unchanged |
| `cli.py`'s blurb derivation | **generic wiring** | consumes `SHORTNAME_TO_BLURB`; adds no elidex policy (the pre-existing elidex path at `cli.py:78` is untouched) |
| `_webref/DESIGN.md`, `spec_labels.py` bullet only | **generic** | describes a generic module; the CI facts do **not** go here (below) |
| `test_spec_labels.py` | **generic** | tests a generic module |
| §4.2 capability verdict, remedy text, no-spec-surface declaration, `test_preflight.py`, `SKILL.md` | **elidex skill** | consumes the library, adds no generic behavior |
| §4.3 script + `mise` task + CI job | **elidex repo infrastructure** | `.claude/tools/python-suites.sh`, `mise.toml`, `.github/workflows/ci.yml` |

⚠ Draft 2 planned to record the `mise` task, the GitHub job, the path filter and the interpreter floor in
`_webref/DESIGN.md`. That file says the core should "stay generic enough to move to a standalone repository
later" and to "keep new generic behavior free of elidex-specific file paths" — a section describing
`mise.toml` and `ci.yml` travels with the tree at externalization and is wrong on arrival. Those facts live
in `python-suites.sh`'s header and the `mise.toml` task comment, where `trip-wires` documents itself.

⚠ Draft 2 also called `spec_labels.py:7`'s skill path "pre-existing". Measured on `origin/main`:
`git grep -nE '\.claude/skills|elidex-plan-review' -- .claude/tools/_webref/` returns **exactly one** hit,
`cli.py:78`. `spec_labels.py` does not exist there, so its skill-path prose is **A's own new** coupling
into the module §7 calls generic. The narrow claim survives — the moved assertion is the one *import-time
executable* edge — but the attribution did not.

**One-issue-one-way**, three collapses: the label enumeration goes from three sites to one; the suite
invocation from zero canonical sites to one (§4.3.2); the `WEBREF.is_file()` question from *n*-per-citation
to one verdict (§4.2.3). Two further instances of §1's class are named rather than silently left:
`preflight` still reaches `resolver.lookup_section` through a subprocess while reaching `spec_labels`
in-process (§11 slot), and `grep_pass.py:143-148` reports a wrong repo root as one HARD finding *per
referenced path* — the same shape §4.2.1 measured as "44 hard findings … an artifact". The latter is **not
A's**: `git diff --name-only origin/main...HEAD -- .claude/` shows `grep_pass.py` untouched, and it is
`grep_pass`'s own precondition, not the citation gate's.

---

## §8 Line-count budget

`wc -l` on **`origin/main`** (2026-07-29):

| File | On `origin/main` | After A (est.) | Note |
|---|---|---|---|
| `.claude/skills/elidex-plan-review/preflight.py` | 499 | ~510 | −30 local map, +20 import/precondition, +~20 no-spec-surface |
| `.claude/skills/elidex-plan-review/test_preflight.py` | — | ~290 | new (P1-P6, P9, P11, P11b, P12) |
| `.claude/tools/_webref/spec_labels.py` | — | ~100 | §4.0's A lines only (the catalog half is B's) |
| `.claude/tools/_webref/test_spec_labels.py` | — | ~110 | §4.0's split, minus the moved assertion and B's round-trip test |
| `.claude/tools/_webref/commands/coverage_map.py` | 114 | ~108 | delegation only; last-resort unchanged |
| `.claude/tools/_webref/cli.py` | 264 | ~272 | the blurb-derivation half only |
| `.claude/tools/_webref/DESIGN.md` | 134 | ~139 | the `spec_labels.py` bullet minus its catalog sentence |
| `.claude/tools/python-suites.sh` | — | ~30 | new (includes the outside-the-filter check) |
| `mise.toml` | 136 | ~142 | `[tasks.tools-test]` + one `depends` entry |
| `.github/workflows/ci.yml` | 126 | ~150 | `tools` filter + job |

**1000-line touch-time check** (cohesion-based): the largest file in the touch set is `preflight.py` at
499 → ~510, half the threshold, and it is one cohesive gate whose seam (structure / breadth / citation /
grep-pass) is already four ordered blocks in `main`. Nothing is near a split.

---

## §9 Edge-dense assessment

The **base case** applies: an approved umbrella's narrowly-scoped, plan-reviewed per-PR slice is a terminal
unit, not re-split for touching the same subsystem as B/C/D.

What makes A safe as one slice: J1-J3 live in one function's control flow with one primary observable (an
exit code) and one secondary one (the summary's classification of each row). §5 publishes the
outcome-distinct rows plus the collapse rule that maps the rest onto them, and every row has a pin. J4 is
independent and is three files of configuration. J5 is a single offline run.

**Draft 5 removes an axis rather than adding one.** Draft 4's third capability cause was dynamic — it
materialised inside the row loop — which is what forced the aggregated verdict, the tri-state, the extra
accumulators and five of the ten pins. With the widening in B, both causes are static and the intersecting
axes drop from three to two. That is the direction the edge-dense rule asks for.

`git diff --stat -- crates/` is empty and stays empty, so a regression degrades a developer tool and cannot
reach a page, a script, or a user.

The ordering couplings are the umbrella's rules, not exemptions: retiring the grep requirement before the
detector is sound would mandate an under-reporting detector (C after B), and the regression pins are
unenforced until a scheduler exists (A before B).

---

## §10 Open questions for `/elidex-plan-review`

Decided here rather than listed, because each had one live option ([[feedback_no-low-value-choices]]): the
`verify_citation` guard is an **explicit raise**, not an `assert`; the re-carve is **its own commit, first
on A's branch**; the interpreter floor is **3.9**; the `tools` path filter stays **broad**, with the script
failing loudly on a suite outside it; and draft 4's Q4 (`K`'s semantics) is answered by §4.2.3 item 5 —
`K` counts label spellings when rows are unresolved, and **says so in its own output**, which is what makes
it checkable instead of open. What remains genuinely open:

- **Q1 — is the boundary drawn in the right place?** This is the question draft 5 exists to put. A keeps
  the *dedup* and hands B the *widening*. The case is: (a) §4.1 already assigned lookup semantics to B, and
  the fall-through **is** lookup semantics, so draft 4 shipped in A the lines it declared B's; (b) the
  widening is the sole reason A needed a network dependency in `mise run ci`, a dynamic third capability
  cause, an aggregated verdict, a tri-state resolver, a deferral and an umbrella obligation — all of which
  §4.3.3/§4.2.3 delete; (c) with it gone A changes **no resolution outcome at all** (§5 P12), which is the
  smallest possible surface for a change to a gate every lane runs. The cost, stated plainly: the
  in-flight c3-plan memo carries 18 §3 rows (`CSSOM VIEW` ×14, `RESIZE OBSERVER` ×3, `INTERSECTION
  OBSERVER` ×1) that `origin/main` soft-warns as unmapped and skips verification for; the widening resolves
  all three correctly (`cssom-view-1`, `resize-observer-1`, `intersection-observer`), and deferring it to B
  defers that gain by one slice. **It is a gain, not a correctness obligation**, and A's charter is
  fail-closed plumbing, not coverage.
- **Q2 — does `required_status_checks` belong in this PR?** It is one rule on an existing active ruleset.
  But measured, the `pull_request` rule already carries `required_approving_review_count: 0` **and** a
  `RepositoryRole` bypass with `bypass_mode: always`, so adding the rule leaves it author-bypassable — the
  change buys visibility-plus-friction, not enforcement. **Recommendation: register, do not implement**
  (§11), because deciding *which* jobs are stable enough to require is entangled with the Layout lane's
  trip-wire work.
- **Q3 — `#11-layoutbox-trip-wire-not-in-ci`.** The slot was registered by #488 (merged 2026-07-27); the
  only `.github/workflows` touch since is **A's own**, so A is the trigger. `feedback_defer_lifecycle_policy`
  Control D requires a fired trigger to receive one of the five formal dispositions.
  **Recommendation: A performs *extend-with-cause*** — date pushed, owner stays the Layout lane, obstacle
  text corrected. Whether A should instead *discharge* it is the reviewer's call; the filter-placement
  ground stands (the trip-wires read `crates/**`).
- **Q4 — the residual in §4.2.5.** The no-spec-surface marker is author-controlled. §4.2.5 argues the
  mitigations (mutual exclusion with a table, greppable literal, `n/a` not `ok`, Axis 4 still reads the
  memo) make it deliberate rather than silent. If review disagrees, the alternative is to require the
  marker to name the umbrella slice it belongs to, which is checkable but adds a coupling A has no other
  reason for.

---

## §11 Defer slots + per-PR ≤3 audit

**One own deferral** against ≤3, plus **one pre-existing-category entry**, a separate class
([[feedback_defer_cap_policy]]). Draft 4 had two own deferrals; `cleanup-webref-suites-offline` is
**dissolved, not deferred** — the dependency that created it leaves with the widening (§4.3.3), and what
replaces it is a forward-binding umbrella constraint on B.

⚠ **Naming/counting rule, settled at umbrella level rather than per-memo.** The registry treats `cleanup-*`
as cap-exempt — both existing entries carry *"non-spec; not a `#11-` cap slot"*. B's memo takes the
stricter line (*"counted against the cap anyway, because the discipline is restraint, not accounting"*).
Two memos in one program cannot answer this differently, so A's landing edit puts the rule in the
umbrella's "Constraints each slice inherits", stated in the dimension that decides: **what counts against
the cap is own-vs-pre-existing, not the `cleanup-*` prefix.** `cleanup-*` stays the name for non-spec
tooling; an *own* `cleanup-*` deferral counts, a *pre-existing* one does not.

### Own deferral (1 of ≤3)

| Slot | Audit |
|---|---|
| **`cleanup-webref-preflight-inprocess-resolution`** | `preflight.verify_citation` forks a subprocess **and** an HTTP conditional-GET per unique citation, while the same file reaches `spec_labels` in-process — two ways to reach the shared library in one file. Measured **0.092 s per citation** as a subprocess vs **0.0008 s in-process**. **Create-time**: pragmatic-shortcut ✓. **Category (3-gate)**: category 2, 別 slot 依存 — the collapse decides whether the plan-review gate must be usable offline, which is B's contract (§4.3.3), not A's. **Confirming Q2 (middle state)**: fires, and is answered rather than overridden — the middle state is one process boundary, is named in §7, and collapsing it now would decide B's offline policy by side effect. **Boundary cost**: the collapse direction makes the elidex adapter depend on `resolver`, which `DESIGN.md` does not list among its declared generic surface — part of the deferred decision, not a hidden cost. **Trigger**: Slice B's `_catalog()` offline contract landing. **Re-eval**: 2026-11-30. |

### Pre-existing category (not an own deferral, not counted)

| Entry | Why pre-existing, and its trigger |
|---|---|
| **`cleanup-elidex-ci-required-status-checks`** | The `main-protection` ruleset (id 13294991, active) has no `required_status_checks` rule, so every CI job is advisory. Pre-existing repo state; A neither creates nor worsens it. ⚠ The cost is **not** "one rule": the `pull_request` rule carries `required_approving_review_count: 0` and a `RepositoryRole` bypass with `bypass_mode: always`, so the rule alone would remain author-bypassable (§10-Q2). **Trigger**: the Layout lane wiring the trip-wires, or the first job stable enough to require. **Re-eval**: 2026-11-30. |

**Explicitly NOT deferred**: the re-carve (§4.0), the two-cause capability verdict (§4.2.3), the four
remedy strings (§4.2.4), the no-spec-surface verdict (§4.2.5), the test relocation (§4.4), the test-siting
constraints (§4.5), the script + `mise` task + CI job (§4.3), `SKILL.md`'s contract update (§4.1), and the
umbrella's two constraint lines.

**Also named, not slotted**: `grep_pass.py:143-148` reports a wrong repo root as one HARD finding *per
referenced path* — §1's class. It is not A's (`grep_pass.py` is untouched) and it is `grep_pass`'s own
precondition. Recorded without a slot deliberately: inventing a slot for another gate's defect is how
ledgers fill with entries nobody can act on.

---

## §12 Exit criterion

**(1) Green:** `mise run tools-test`

**(2) Red — every new pin detects the defect it names:**

```sh
git worktree add /tmp/citeaudit-pre <the re-carve commit from §4.0>
cp .claude/skills/elidex-plan-review/test_preflight.py /tmp/citeaudit-pre/.claude/skills/elidex-plan-review/
cp .claude/tools/_webref/test_spec_labels.py /tmp/citeaudit-pre/.claude/tools/_webref/
cd /tmp/citeaudit-pre && bash .claude/tools/python-suites.sh; echo "EXPECT NON-ZERO: $?"
```

Non-zero, with at least one failure attributable to each pin whose §5 row is marked ✓ in the *Detects the
naive carve?* column — **P2, P2b, P3, P4, P5, P6, P11, P11b** — plus **P9** (the re-carve's
`spec_labels.py` fetches `ed/index.json`) and **P12** (the re-carve's `coverage_map._spec_label` is not
byte-identical to `origin/main`'s). P1 is correctly absent: it detects no re-carve defect. That mapping is
§5's column, not a second list.

**(3) A carries no part of B — filenames *and* prose**, ranged from the re-carve:

```sh
git diff --name-only <re-carve-A-commit>..HEAD -- .claude/tools/_webref/   # only §4.0's A column
git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/       # → empty
git grep -n '_catalog\|webref_data' -- .claude/tools/_webref/spec_labels.py  # → empty
```

The second and third are not decoration: §4.0's split is at hunk *and line* granularity, five prose sites
in the A column describe `commands/cite_audit.py` as extant, and the third is what mechanically enforces
that the widening did not travel back into A.

**(4) The branch carries only A's own memo and the umbrella.** `git diff --numstat origin/main...HEAD --
docs/plans/` currently lists A, B, C and the umbrella. The umbrella **must** ship with A (§13 item 2 edits
it), so the criterion is "A's memo + the umbrella, and nothing else". **B's and C's memos move to their own
branches**, since each is its own PR's artifact.

**(5) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page. Verified by observation; today the same observation yields zero jobs (§4.3.1).

---

## §13 Coordination

Re-derived 2026-07-29 with three-dot ranges and a complete `gh pr list --state open` (**#489, #486, #381**;
#492 **merged** 2026-07-28).

| Lane | Overlap with A | Ordering rule |
|---|---|---|
| **Slice B** | total by construction — B branches from A's landed head and takes §4.0's B column, **including the catalog fall-through, the discriminated `_catalog()`, `coverage_map`'s changed last-resort and `test_coverage_map_fallback_round_trips`**. B's memo already describes all of these as B's and **needs no edit** | **A first** |
| **Slice C** | `_webref/DESIGN.md` — same file, disjoint sections. Also inherits the §3 no-spec-surface declaration as its first real consumer (C's memo hard-fails preflight today by design, having no §3 table) | after B |
| **PR-A0 / D** (`domform-submittable-category` @ `04a771b5`) | ⚠ the carve-provenance claim is **false at head**: `git diff domform-submittable-category -- .claude/` = **1 file / 4 lines** (`layout-box-reader-allowlist.tsv`), because A rebased onto `db96f231` and PR-A0 did not. **Rebase PR-A0**, then the claim holds again. Drops its `.claude/` half once A/B land | after A/B/C |
| **PR #381** `dependabot/github_actions/actions/checkout-7` — OPEN since 2026-06-21 | ⚠ **actual `ci.yml` contention.** *Modifies* `.github/workflows/{audit,ci,codeql}.yml`, bumping `actions/checkout@v6 → @v7` in the same `steps:` blocks §4.3.2 extends. It is a workflows-only PR, so **after A it triggers the `tools` job** — as will every future dependabot GHA bump | **Both directions**: if #381 lands first, A's new job must use `@v7`. If A lands first the merge is clean but A's job sits at `@v6` beside four `@v7` until the next weekly dependabot run |
| **PRs #489 / #486** | none — `crates/**` only (`git diff --name-only origin/main...vm-p4-slice0a \| grep -cv '^crates/script/elidex-js/'` → 0) | none |
| **C-3 plan / turn-completion / slice1** | no file overlap; preflight-behaviour overlap only | landing checklist |

**Collateral the `tools` filter accepts, re-derived**: `.claude/tools/layout-box-reader-allowlist.tsv` is
on-main since #491, `.github/workflows/**` is in the filter (every dependabot GHA bump), and #492
(`.claude/skills/**` only) is a third class that would have triggered it. §10-Q1's broad filter is chosen
knowing this; the enumeration **drifts by nature and is re-derived at landing**, not pinned here.

**Handed to Slice B** (findings A produced but does not own):

1. **B §4.1.8's consequence sentence is falsified for every pair it names** — §0's second ⚠ has the
   measurement. The ambiguity is real; *"citation verification silently runs against the wrong document"*
   is not, for level collisions. B restates the consequence on the 8 cross-series cases or drops it.
2. **The umbrella's §"Cross-lane coordination" bullet is wrong on both counts**: it says *"10 in-flight
   memos in `elidex-wt-c3-plan` cite those labels"*, but `git diff --name-only origin/main...HEAD --
   docs/plans/` in that worktree lists **1** memo, and the labels it actually carries are `CSSOM VIEW`,
   `RESIZE OBSERVER` and `INTERSECTION OBSERVER`, not `CSSOM`/`Selectors`/`Pointer Events`. A corrects the
   count and the labels in the same edit as item 3 below, since the bullet is B's obligation but the
   umbrella is A's file to land.

**Landing checklist**:

1. Re-run `preflight.py` on the plan-memos each lane is **authoring** — `elidex-wt-c3-plan`,
   `elidex-wt-vmp4plan`, `elidex-wt-turncomp`, `elidex-wt-slice1`, and `elidex-wt-submittable` — from
   **each worktree's own copy**, since `REPO_ROOT` derives from `__file__`. `elidex-wt-c4fix` is dropped
   (`git diff --name-only origin/main...layout-c4-classification-fix -- docs/plans/` → 0, so it authors
   none). Slice C's memo is expected red until it adopts §4.2.5's marker.
2. Register the 1 own deferral and the 1 pre-existing entry; add the umbrella's **two** constraint lines
   (the cap naming/counting rule, §11; the no-network-without-offline-degradation rule, §4.3.3); correct
   the trip-wire obstacle text in **both** `project_open-defer-slots.md` and
   `project_inline-mod-split-owed.md` §B (the prose is only in the latter). Memory-file writes, not chips
   ([[reference_spawn-task-chips-not-durable]]).
3. Fix the umbrella's branch-measured figures at `:44-45` — *"the 48-test `_webref` suite fetches 2 URLs"*
   is the branch count; on `origin/main` `_webref` is **12 tests** and fetches **0**. Same edit as item 2
   and the §13 hand-off item 2.
4. Correct the two live stale strings — `project_slice1-elementstate-cache-deletion-state.md:68` and
   `project_pr-a0-review-ledger.md:11`, both carrying `d3173bed` (re-derived 2026-07-29; the other three
   draft-3 listed no longer exist).
5. `MEMORY.md`'s L3 bullet: set A-landed / B-next, and replace the draft-4 "▶ next" text.
6. PR description states §4.3.3 (A adds no network, and the gate's pre-existing network requirement),
   §4.3.4 (no `required_status_checks`, and the bypass actor), the `tools`-filter collateral, and the #381
   contention.

---

## §14 Review-round index

Four `/elidex-plan-review` rounds. **This section is an index, not a restatement** — every live correction
is stated once, inline, at the section that acts on it.

**Round 1 → draft 2.** Root cause: the evidence base was measured on the branch, not `origin/main`.
C1 slice boundary + test counts (→ §4.0, §4.3.1) · C2 the "unreachable" capability branch (→ §4.2.3) ·
C3 branch protection via the deprecated endpoint (→ §4.3.4) · C4 two-dot ranges (→ §13) · C5 the offline
gap routed to a non-owner · C6 a Python floor that was B's need (→ §4.3.5) · C7 a six-cell "complete"
matrix (→ §5) · C8 CI facts headed for the generic core (→ §7) · C9 a trip-wire rationale that inverted the
Layout lane's record (→ §10-Q3) · C10 a red-check reading an uncommitted patch (→ §12) · C11 slot obstacle
text attributed to the wrong file (→ §13).

**Round 2 → draft 3.** Root cause: draft 2's own fix opened a new failure and disabled a neighbouring gate.
D1 the third capability cause (`SystemExit` escaping `_catalog()`) · D2 the row-loop skip stranding
`unique_specs`/`specs_seen`/`malformed_rows` (→ §4.2.3 item 3) · D4 the `SPECS` bound §3.1 relied on is the
one the widening deletes (→ §3.1) · D5 the inbound exposure delta (→ §3.1) · D6 `assert` under `-O`
(→ §4.2.3) · D7 the matrix arithmetic (→ §5) · D8 rows labelled "default" measured under `--no-grep-pass`
(→ §5) · D9 `cleanup-*` cap accounting (→ §11) · D10 vendoring the rejected patch as a fixture (→ §6 P4) ·
D-a/D-b elidex coupling attributed as pre-existing (→ §7) · D-c the no-spec-surface gap routed to B ·
D-d/D-e count and `gh pr list` corrections (→ §12, §13).

**Round 3 → draft 4.** E1 §4.6 asserting the row-loop skip draft 3 had removed · E2 `K` is not
capability-independent (→ §4.2.3 item 5) · E3 §5 row 11 was two rows · E4/E5 row-value and blanket-invariant
corrections (→ §5) · E6 accumulators read everywhere and written nowhere · E7 the test-isolation contract
one module too shallow (→ §4.5 item 3, now resolved by deletion) · E8 the no-spec-surface gap's second
owner also could not perform it (→ §4.2.5) · E9 the trip-wire trigger is A itself (→ §10-Q3) · E10 stale
memory strings already gone (→ §13 item 4) · E11 #491 merged mid-review.

**Round 4 → draft 5.** Root cause: **the slice boundary, three drafts in**. F1 A hardens a gate on a
resolver whose correctness is B's (→ §4.2, the scope change) · F2 §4.5 cited `try_fetch_data.cache_clear()`,
which does not exist on `origin/main` (→ §4.5 item 3, deleted with the widening) · F3 §4.2.5 declared but
unspecified — no arm in the capability verdict, table block bypassed, `ok`/`0` printed vacuously
(→ §4.2.5, now fully specified with rows 10/11 and P11/P11b) · F4 §4.6 asserting a property the row two
above marked "UNCHECKED — and currently false" (→ §5, one table) · F5 fixtures could not reach the rows
whose pins they were (→ §5's Pin column, and §6's fixture list) · F6 §0's status block naming an earlier
draft than §14 (→ §0) · F7 `gh pr list` stale again (→ §13, now stated as re-derive-at-landing) · F8
PR-A0's carve-provenance claim false at head (→ §13) · F9 the memo's length is restatement, not evidence —
§5/§6/§4.6/§12(2) collapsed to one table; ~59 draft-number mentions and 12 draft-N correction blocks
deleted.

**Reviewer claims re-derived and rejected**: round 1's "`coverage_map_label` has more than one caller" (two
hits: the `def` and one call site); round 2's "`elidex-wt-c4fix/docs/plans/` is empty" (`ls elidex-wt-c4fix/docs/plans/ | wc -l` → 69 files, verified 2026-07-28 — the
substance, that the branch *authors* none, held); round 4's decisive-finding **consequence** — that a
catalog level collision verifies against the wrong document — **falsified by measurement** (§0's second ⚠).
The finding's *conclusion* (move the widening) survives on the boundary argument in §10-Q1; its stated
*reason* did not, and draft 5 does not carry it.
