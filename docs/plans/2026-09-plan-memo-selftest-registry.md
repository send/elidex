# Plan — the self-test's row registries: no row or case can leave, duplicate or go inert without a red

**Status**: DRAFT for `/elidex-plan-review` — no implementation until it converges. Branch
`vm-p4-plan-memo-checker` (worktree `elidex-wt-vmp4checker`, PR #510), written at local head `3edb6c91`.
Parent: `docs/plans/2026-08-plan-memo-umbrella-checker.md` (§7 / §8). **Subject**: the machinery that
decides WHICH rows the self-test runs — the case records, the mutation rows and the control table they
feed. **Not the subject**: the checker's gates; no attestation has found a live hole in them.

⚠ Every figure below carries the command that produced it. `T=.claude/tools`; `P=` the attestation's
scratch probes (`…/scratchpad/attest3ed/`, NOT committed — each probe's edit is restated in §6 so it
survives the scratch directory). "Measured" means re-run for this memo on 2026-09-22 against `3edb6c91`.

## §0 Why a plan and not a fifth fix pass

Four fix passes on this machinery (`79309486`, `b325c668`, `3545f944`, `3edb6c91`) were each attested
by a fresh adversary; IMP findings ran **6 → 5 → 7 → 4**. Every pass ADDED a mechanism — membership by
content, a sealed base tuple, one `collect` step, the `CRITERIA` table, a population module — and every
next attestation found holes INSIDE the mechanism the previous pass added. That is the signature of
`memory/feedback_defer-accumulation-signals-mis-drawn-slice.md`: the boundary is wrong, not the patches.
By CLAUDE.md's edge-dense rule (≥3 intersecting invariants — §2 has seven — and no canonical algorithm
for "which rows exist"), the design is reviewed before any further code. The direction of this plan is
**deletion**: each mechanism below either makes an invariant true by construction or is not added.

## §1 The property (one sentence per invariant)

**Definition — DECLARED.** A row is *declared* by the self-test file that constructs it, when that file
is executed in isolation (a fresh `plan_memo*` module namespace); construction IS declaration, and no
list binding, file name or import order enters the definition.

- **I1 MEMBERSHIP** — the set of rows run equals the set declared on disk.
- **I2 ONCE** — each declared row runs exactly once: row identity `(file, ordinal)` is unique, mutation
  labels and control-table keys are unique.
- **I3 KILL SET** — every mutation row names a non-empty set of controls, each of which exists; every
  case is exactly one entry of the control table.
- **I4 ISOLATION** — the collected value is a function of each file's own text: no import order and no
  other module's import-time effect can change it.
- **I5 ONE VALUE** — the rows are collected exactly once per process into a hashable (hence deep-
  immutable) value, and every reader receives that one object.
- **I6 PATCHABLE** — the mutation proof can patch each mechanism of I1–I5 (constructors, collector,
  partners) and the named control runs the PATCHED mechanism.
- **I7 PINNED** — every criterion of the mechanism is killable: each partner drives the real control's
  own code path, and the derived clause / key sweeps (§4 M8) leave no unexplained survivor.

## §2 Coupled invariants — every pair, one line each

Intersecting pairs (the mechanism that sits on the intersection):

- **I1×I2** — identity `(constructing file, ordinal)` makes "the set" and "once" the same object; a
  helper that constructs rows and is imported by two files must not count twice → keep a row iff its
  constructing file is the file under collection.
- **I1×I3** — a row with an empty kill set is declared but inert → refused at CONSTRUCTION, so
  "declared" implies "runnable" (MIN-5).
- **I1×I4** — without isolation "declared" depends on who imported whom first (IMP-3: 472 then 422).
- **I1×I5** — the declared set is materialised once; a reader that re-collects may see another set.
- **I1×I6** — a row against the collector must be able to drop a declared row and be killed → the
  collector takes `here`, and its partner plants a directory.
- **I1×I7** — the membership control's own disk read must be what its partner exercises (IMP-2).
- **I2×I3** — a duplicate case name collapses two table entries into one → one case has no entry; the
  control table's merge must refuse a repeated key (N-1, measured in §5).
- **I2×I4** — isolation re-executes shared modules once per sandbox; rows they construct would appear
  once per importer → the provenance filter of I1×I2 is also I2's guard.
- **I2×I5** — uniqueness is checked once, on the one value; re-collection cannot manufacture duplicates.
- **I2×I6** — `patched_module`'s `table.update(pm.registry())` OVERRIDES keys on purpose; the refusing
  merge governs the unpatched build only, and the override stays confined to the patched row.
- **I2×I7** — the refusing merge and the label check are themselves in the sweep population (M8).
- **I3×I4** — rows come from sandboxes, controls from the main process: a row names controls by
  STRING, resolved at run time (the existing "unknown control" FAIL), never by object.
- **I3×I5** — a kill set frozen as a tuple cannot be cleared at import (MIN-5's probe E3c').
- **I3×I6** — a row naming a control of a patched module resolves against the merged table (existing).
- **I3×I7** — the constructor's non-empty precondition is a clause the sweep drops.
- **I4×I5** — the one value is memoised in the collector module, which no row module can reach in the
  main process (M4: row modules are leaves, executed only inside sandboxes).
- **I4×I6** — the sandbox purges `plan_memo*` modules, which would evict a leaf the harness installed
  for the current row (`_INSTALLED_LEAVES`) → the sandbox reads each file's text through the harness's
  per-row `SOURCES`, so a patched constructor or collector is what the sandbox executes.
- **I4×I7** — the isolation partner plants a module that tampers with another and asserts the victim's
  rows are unchanged.
- **I5×I6** — the memo lives on the module INSTANCE: a patched collector computes its own value and
  the unpatched memo cannot leak into that row.
- **I5×I7** — `rows() is rows()` and `hash(rows())` are partner arms.
- **I6×I7** — the derived sweeps are mutation rows generated in-process through the same patch path;
  a survivor must be killable through the ordinary row path, not through a sweep-only hook.

Non-intersecting pairs: none — the 21 above are all C(7,2) pairs.

## §3. Spec coverage map

No external spec governs this subject; every row is local policy. Input = repository Python source.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Local policy, no external spec (this plan §1 I1) | declare | a row is constructed at import of a self-test file | `plan_memo_selftest_cases.py` `case`/`acase`/`rcase`; `mutant` (NEW) | ✓ (§4 M1) | no |
| Local policy, no external spec (this plan §1 I1) | collect | every self-test file on disk is executed once, in isolation | `plan_memo_selftest_registry.py` collector (rewritten) | ✓ (§4 M2) | no |
| Local policy, no external spec (this plan §1 I1) | provenance | a row constructed by a file outside the self-test population | collector | ✓ red | no |
| Local policy, no external spec (this plan §1 I2) | uniqueness | repeated label; repeated control-table key | collector; `plan_memo_selftest_controls.py` `registry` | ✓ red | no |
| Local policy, no external spec (this plan §1 I3) | kill set | empty / unknown control names | `mutant` (NEW); `plan_memo_selftest_mutants.py` `run` | ✓ red | no |
| Local policy, no external spec (this plan §1 I4) | isolation | a module that imports or tampers with a row module | collector sandbox; leaf rule over the import graph | ✗ dynamic reach (§4 M4) | no |
| Local policy, no external spec (this plan §1 I5) | handoff | every reader of rows | runner, `run`, loop ratchet, partner | ✓ (§4 M3) | no |
| Local policy, no external spec (this plan §1 I7) | fidelity | partner vs real control code path | `plan_memo_selftest_population.py` | ✓ (§4 M7) | no |
| Local policy, no external spec (this plan §1 I7) | criteria | BoolOp / Compare operands; `in`-test key components | ratchets, collector, harness, population | ✗ (§4 M8 blind classes) | no |

**Breadth**: K=0 external specs (preflight counts the local-policy label as K=1), M=9 rows. **Split decision**: two slices (§7), each plan-reviewed.

### §3.1 User-input touch audit

None. The self-test reads only files in `.claude/tools/`; no memo text reaches any mechanism here.

## §4 Mechanisms — each with what it deletes, closes, and leaves

Measured baseline (`python3 $T/plan-memo-umbrella-check.py --self-test --mutants` at `3edb6c91`):
747 controls, 472 mutation rows, rc 0. Row modules (verified 2026-09-22 via `H.registry_modules`):
MUTANTS 7 files (79/101/22/110/38/93/29 rows), CASES 6 files (164/192/154/59/71/26) — per-module
`len(X.MUTANTS)` / `len(X.CASES)` after import.

**M1 — Construction is declaration.** `case` / `acase` / `rcase` (existing names) and `mutant` (NEW)
append to a SINK owned by the collector and RETURN the row; the calling file is read from the first
frame outside the constructor module. **Deletes**: every module-level `CASES` / `MUTANTS` list,
`spellings(into)`, `_OWN`, `_SEALED`, both tuple seals, `registry_modules` / `_assigns` (the by-content
partition), `collect`, `cases()`, `mutants()`, and the `CRITERIA` table (M8). A row that references an
earlier one takes the constructor's return value (`R42_BLANK_MARKER = rcase(…).name`), not `CASES[-1]`.
*Closes by construction*: IMP-4 (no list to leave empty; `spellings([])` has no target to rebind),
MIN-6 (no list to alias), MIN-7 D/L/N (the partition they mutate is deleted). *Leaves*: a row constructed
only inside a def or a conditional that never runs at import → M1's static rule: outside the constructor
module, a constructor call under `def` / `if` / `while` / `try` / `with` / a comprehension is red.
Measured today: 0 such calls outside the base spellings' own two bodies (AST walk over
`$T/plan_memo_selftest_cases*.py` flagging those ancestors → `plan_memo_selftest_cases.py` lines 104 /
107 only, both `acase`/`rcase` calling `case`). ⚠ Why not count rows statically instead (verified 2026-09-22): 4 of the 6
cases modules construct rows in top-level `for` loops (syntactic calls vs rows: inline 174 / 192, r26
51 / 59, r42 49 / 71, sibling 20 / 26, by the same AST walk against `len(X.CASES)`), so a static count
is not the declared set.

**M2 — Isolated collection, once.** The collector executes EVERY self-test file of `H.files(here)`
(the one population; 26 at head, `is_selftest` over `files()`) in a sandbox: `sys.modules` stripped of
`plan_memo*`, the file imported, the sink read, the namespace restored. Keep a row iff its constructing
file is the file under collection; a row constructed by any file outside the self-test population is
red; an import error is red (never skipped). Measured cost of the full-purge sandbox (purge +
`importlib.import_module` per module, timed in-process): 0.22 s for the 13 row modules, 0.50 s for all
26 self-test modules (verified 2026-09-22). *Closes by construction*:
IMP-3 (no base list exists to reassign; a module's import touches only its own sandbox's copies of
other modules), M/O-shaped order effects (the value is sorted by `(file, ordinal)`). *Leaves*: a file
that reaches ANOTHER sandbox's objects by introspection (`gc`, frames, closures) — declared out of scope,
not guarded; a file that erases its own rows through the sink — a self-authored edit to that file,
visible in its diff, and M4's seam table makes naming the sink's internals red.

**M3 — One frozen value.** `rows()` returns a memoised `Rows(cases, mutants, per_file)`; every row is a
tuple of `str` / `int` / tuples (a case's `files` dict becomes sorted pairs), so `hash(rows())` succeeds
iff nothing mutable remains. Every reader — runner, `run`, `_census_rows`, the partner — calls `rows()`
and nothing else. *Closes by construction*: IMP-3's "re-collects per caller (472 then 422)" (measured:
`mutants()` twice in one process with probe E3a' applied → 472, then 422), MIN-5's clear-at-import
(E3c'). *Leaves*: nothing of I5 that M4 does not cover.

**M4 — Row modules are leaves.** No module imports a file that constructs rows (the full import graph
`registry_membership_control` already builds, function-local imports included); shared vocabulary
(`build`, `Case`, the file-name constants, `run`) moves to non-row modules, so the main process never
executes a row module's code. The sink's private name joins `_IMPORT_SEAMS` with the collector as its
only importer. *Closes*: the main-process half of IMP-3 (a row module cannot reach the memo).
*Leaves (declared blind, shared with the membership and seam controls)*: dynamic import by string
(`importlib.import_module("plan_memo_" + …)`, `__import__`, `sys.modules[…]`).

**M5 — Count and compare at the consumers.** The control table is built by ONE flat, refusing merge
over the fragment modules DERIVED from disk (every self-test file whose top level defines `registry`),
replacing the nested `update` chains (`controls` → records / work / ratchets / population, records →
properties → invariants, work → growth / pipeline — the `import registry as` lines): `len(table) == Σ len(fragment) +
len(rows().cases)`; mutation labels are unique; `run` asserts its verdict count equals
`len(rows().mutants)`. *Closes by construction*: MIN-6's missing label check, N-1 (a fragment key that
shadows another's), N-4 (a fragment nobody merges). *Leaves*: a control function its own module
defines but does not return from `registry()` — blind, as today.

**M6 — Constructor preconditions.** `mutant(name, file, find, replace, controls)` refuses an empty
`controls`, a non-`str` control name, an empty `find`; `case` refuses a measure outside the
harness's `measure` domain. *Closes by construction*: MIN-5.

**M7 — Partner = the control's own function over a planted directory.** Every population / membership
control becomes `verdict(here)`; its control calls `verdict(HERE)`, its partner calls the SAME
`verdict(planted_dir)` — never a hand-built `{file: text}` dict. The planted directory carries the
shapes each criterion excludes: a non-glob `memo_extra.py` imported by the population (IMP-2), a
`from plan_memo_b import x` edge (MIN-8), an `import` edge (existing). **Derived backstop**: the lines
of each mechanism function executed by the real control must be a subset of those executed by its
partner (`_count_line_sites`, the existing witness); a line only the real control runs is red — that is
IMP-2's class, not its instance. *Leaves*: a line both run whose VALUE the partner does not discriminate
(M8's subject).

**M8 — Criteria completeness is DERIVED; `CRITERIA` is deleted.** Two operators, generated from the AST
of the mechanism functions (the functions M7's backstop records as executed by the self-test's
mechanism controls — ratchets, collector, harness population, partners), each applied through the
ordinary patch path (I6). ⚠ "The mechanism controls" is a STATED set (today's `_FAMILY` plus the
collector's partner); UNREVIEWED whether it is complete — a mechanism whose control is outside it is
swept by nothing:
  * **clause-drop** — each `BoolOp` operand and each `Compare` forced `True` and `False`;
  * **key-projection** — at each `x in R` / `x not in R` whose `x` is a tuple, drop one component
    (`x' in {e[…] for e in R}`); whose `R` is a subscript `V[k]`, union over the dimension
    (`x in set().union(*V.values())`).
Each generated mutant must turn a control red; a survivor gets a partner arm, or an entry in a stated
EQUIVALENCE complement keyed by (function, operand source text) that is itself checked to name an
existing site. Cost: the ratchets clause-drop alone took 57 s wall for 96 mutants (`time`), so the
five rows below (162 mutants) extrapolate to ~1.6 min — an EXTRAPOLATION, measured at R2. Measured
survivors today (prototypes `$P/clausedrop2.py` and
`…/scratchpad/regplan/keyproj.py`, not committed, against the unmodified tree):

| module (operator) | sites | mutants | survivors |
|---|---|---|---|
| ratchets core (clause-drop) | 48 | 96 | 11 |
| ratchets core (key-projection) | 6 | 6 | 4 — incl. IMP-1's union |
| collector `collect` (clause-drop) | 6 | 12 | 1 |
| harness population fns (clause-drop) | 19 | 38 | 4 |
| population verdict fns (clause-drop) | 5 | 10 | 3 |

**Classes neither operator sees (the complement, stated)**: literal / constant edits (a glob string,
`[:1]` → `[:2]`); argument substitution (IMP-2's `"*.py"` → `H.GLOB` — M7's backstop covers it);
statement deletion and early `return`; ordering and tie-breaks (M, O — equivalent under I4's sort);
a key compared by `==` or looked up by `d[k]` rather than `in`; coarsening in a DATA table rather than in
code; non-termination (the survivor `_membership_verdict` `u in seen` → `False` is a loop that revisits
— terminating on the acyclic probe, unbounded on a cycle); anything outside the mechanism functions.

**Rejected candidates.** *Seal harder* (a module `__setattr__` guard, `MappingProxyType`) — adds
mechanism, and E3a' shows a seal is bypassed by reassignment; *subprocess per module* — breaks I6 (a
patched collector is not what a child process runs); *static row counting* — measured infeasible (M1);
*pinned expected counts* (a committed "472") — a figure that every row-adding commit must edit, the
shape the umbrella's §8 records going stale three times.

## §5 Finding map — no finding without a mechanism or a stated exclusion

| # | Finding (measured) | Closed by | By construction? | Residue |
|---|---|---|---|---|
| IMP-1 | e1 probe A (sanction keyed per question → union): rc 0, 747 controls | M8 key-projection (and its 3 sibling survivors, N-2) | derived detector | keys compared by `==` / `d[k]` |
| IMP-2 | e1 probe G (`here.glob("*.py")` → `H.GLOB`): rc 0 | M7 same-function partner + line-subset backstop | yes (partner) + derived | a line both run, value undiscriminated (M8) |
| IMP-3 | e23b E3a' (reassign base, 422 rows) and E3d' (clear sibling, 371 rows): rc 0 with `--mutants` | M1 (no lists) + M2 (isolation) + M3 (once) + M4 (leaves) | yes | introspective reach; dynamic import (M2/M4) |
| IMP-4 | e23b E2b (own `CASES = []`, rows via `spellings([])`): rc 0 | M1 | yes | construction under a conditional (M1 static rule) |
| MIN-5 | E3c' (100 base rows' controls cleared): rc 0 | M6 + M3 | yes | — |
| MIN-6 | E2c (alias of r26's list): 510 rows, rc 0; no label check | M1 (no lists) + M5 (labels unique) | yes | — |
| MIN-7 | e1 D / L / N: rc 0 | M1 deletes the partition; arms in §6 | yes (deleted) | — |
| MIN-8 | e1 F: rc 0 without `--mutants`; e1f with `--mutants`: rc 1, 110 survived — caught only incidentally | M7 plants the `from` edge | yes (partner) | — |
| MIN-9 | e1 C / E / H: rc 0 | M8 clause-drop (all three are in the 11 ratchet survivors) | derived | — |
| MIN-10 | "file-name rule decides" in 6 mutants docstrings (verified 2026-09-22: `grep -c 'file-name rule decides' $T/plan_memo_selftest_mutants_*.py` → 6 files) | deleted with the headers M1 rewrites | yes (deleted) | prose: no detector (below) |
| MIN-11 | `_IMPORT_SEAMS` label "module-set handles" omits `files` — the population handle since `b325c668`, imported by ratchets / properties / records (`grep -n 'from plan_memo_selftest_harness import' $T/*.py`) | clerical, slice R1 | no | prose |
| MIN-12 | printable control says "the two report modules"; its own output says 4 (`--self-test` line `41 emit site(s) over 4 report module(s)`) | clerical, slice R1 | no | prose |
| N-1 | NEW: a fragment key equal to another fragment's replaces it silently — planted in the population fragment over a ratchets key: rc 0, still 747 controls | M5 refusing merge | yes | — |
| N-2 | NEW: key-projection survivors beyond IMP-1 — `(mod, caller) in sites[name]` drop-caller; `(name, mod, caller) not in seen` drop-name and drop-module | M8 + partner arms | derived | as IMP-1 |
| N-3 | NEW: population verdict clause-drop 3 / 10 survive (`_imports` `node.module`; `_membership_verdict` `r in inside`, `u in seen`) | M8 + arms, or equivalence entries with reasons | derived | non-termination class |
| N-4 | NEW: the control table is merged through hand-written `update` chains; deleting `reg.update(document_registry())` from `plan_memo_selftest_work.py` drops 7 controls (740) at `--self-test` rc 0. Dropping the ratchets / population / records (imported as `property_registry`) merge in `controls` IS red, but only through `property_family_control`'s hand tuple `_FAMILY` (`--mutants` not run for these probes) | M5's refusing merge ranges over the fragment modules DERIVED from disk (self-test files defining a top-level `registry`) | yes | — |
| M, O | e1 M (collection order reversed), O (checker import ties reversed): rc 0 | declared equivalent: I4 sorts by `(file, ordinal)`; O is two valid topological orders | — | — |

**Prose staleness (MIN-10..12) is not closed by any mechanism here** — it is the class
`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md` names; this plan deletes the stale
sentences it can and corrects the other two by hand. No detector is proposed: UNREVIEWED whether one
belongs in this subject at all.

## §6 Acceptance — every probe that showed a hole becomes a permanent control or kill row

Each line: probe (edit) → what it becomes. "arm" = an assertion in the partner over a planted
directory; "row" = a mutation row whose control is named.

- **A** (`if (mod, caller) in sites[name]` → `… in set().union(*sites.values())`) → generated
  key-projection row; arm: a caller sanctioned for `_claims` that calls `_phrases` is red.
- **C** (bucket order: pinned decided before exempt) → generated clause-drop row + arm: a loop both
  credited and exempt is counted exempt.
- **D / L / N** (partition read by `ast.walk` / Assign-only / any module) → the partition is deleted;
  arms: rows constructed inside a top-level `for` are collected; a file binding no list is collected by
  its constructions alone; a non-self-test file constructing a row is red.
- **E** (`_intends_truncation` any line with `[:1]`) → generated clause-drop row + arm.
- **F** (drop the `from X import` order edge) → arm: the planted directory's `from` edge orders.
- **G** (`here.glob("*.py")` → `here.glob(H.GLOB)`) → row, killed by the M7 partner's `memo_extra.py`.
- **H** (module-level calls not counted) → generated clause-drop row + arm: a module-level `_claims`
  call is red.
- **E2b** (own `CASES` empty, rows bound to a throwaway list) → arm: a planted module whose rows reach no
  list of its own still contributes them.
- **E2c** (`MUTANTS = _r.MUTANTS`) → arms: a planted module importing a row module is red (M4); two files
  constructing one label is red (M5).
- **E3b** (`setattr` extends the base: 473 rows) went red (verified 2026-09-22, e23 re-run) only through the MODULES-map
  control over its NEW file — the existing-module variant is E3a''s shape → same arm as below.
- **E3a' / E3f'** (an existing module reassigns the base list: 422 rows rc 0; for CASES 747 rc 0 —
  no loss only because `collect` read the base first, i.e. order luck) and **E3d' / E3h** (clears a
  sibling's: 371 rows rc 0; the CASES variant rc 1 only incidentally, through unknown-control names) → arm: a planted module that imports another row module is red (M4), and, reached by string through
  `sys.modules` inside its own sandbox, leaves the victim's collected rows equal (M2).
- **E3c'** (clear 100 rows' controls) → row: drop M6's non-empty precondition; arm: `mutant(…, [])`
  raises.
- **dupfrag** (a fragment key shadowing another) → row: replace the refusing merge with `update`.
- **M8 survivors** (the §4 table's 23) → each a generated row killed by an arm, or an equivalence entry
  with its reason; the equivalence list is checked to name existing sites.
- **Not converted**: `$P/probe3/` (a module-level `from plan_memo_selftest_harness import files` appended
  to the controls module) is rc 0 at head and adds an edge every stated rule permits — no hole shown;
  e1 **B / I / J / K** were already rc 1 through a control; e23 **E2a / E2b' / E2d / E2e** were rc 1 by
  CRASH (unpack / index / import / attribute error) — loud, and in the new design M2's "an import error
  is red" keeps E2d loud; E2a / E2b' / E2e test list shapes that M1 deletes. e23's other rc 1s were the
  MODULES-map control over the planted file, which is why e23b re-ran them mapped.

**Gates (all, on the slice head)**: `python3 $T/plan-memo-umbrella-check.py --self-test --mutants` rc 0
with every arm and row above present; `bash $T/plan-memo-umbrella-selftest-trip-wire.sh` rc 0;
`bash scripts/trip-wires.sh` rc 0; M8 reports 0 unexplained survivors; the wire's wall clock before and
after, stated with its command (the sweeps run under `--mutants` only if the always-run cost exceeds
the umbrella §8 (4) budget); every §6 probe whose target text still exists, re-applied to the slice head through a
`$P/mut.py`-shaped runner WITH `--mutants`, must be rc 1; a fresh-agent attestation by ENUMERATION
(`memory/feedback_attestation-by-enumeration-not-assertion.md`) with the §2 pairs as its checklist.

## §7 Slices

- **R1 — declaration, collection, value** (M1–M6 + MIN-10..12 clerical). Touches every row module
  (mechanical: tuple rows → `mutant(…)`, base rows moved to leaf modules) plus the collector, the
  runner, `run` and the partner. Its per-slice plan fixes the leaf split's file names and the frozen
  `Case` shape.
- **R2 — pinning** (M7–M8). Depends on R1 (the sweep population includes R1's collector).
Each slice is plan-reviewed on its own before implementation; this memo is their shared design.
⚠ **Open (user)**: whether R1 / R2 land as commits on #510 or as a follow-up PR. The edge-dense rule
forbids bundling unreviewed work; it does not decide the PR boundary.

## §8 Defer

None proposed by this plan. The blind classes of M2 / M4 / M8 are stated scope limits, not deferrals;
if the review judges any of them in scope, it becomes a slice here rather than an entry in the
umbrella's §8.
