# Plan — the self-test's row registries: no row or case can leave, duplicate or go inert without a red

**Status**: draft 2, revised after plan-review R1 (IMP 29 / MIN 17 / CRIT 0 raw, deduplicated to
S1–S11 / P1–P5 / MIN in the R1 decision record). R1 was STRUCTURAL, so this draft goes to a full re-review.
Branch `vm-p4-plan-memo-checker` (worktree `elidex-wt-vmp4checker`, PR #510), measured at `3edb6c91`.
Parent: `docs/plans/2026-08-plan-memo-umbrella-checker.md` (§7 / §8). **Subject**: the machinery that
decides WHICH rows the self-test runs — case records, mutation rows, and the control table they feed.
**Not the subject**: the checker's gates (no attestation found a live hole in them).

⚠ Every figure carries its command. `T=.claude/tools`, `S=` the session scratchpad
(`/private/tmp/claude-501/-Users-kazuaki-repos-send-sh-elidex/a522c74c-…/scratchpad`, NOT committed).
Probe edits are restated in §6 so they outlive `S`. "Measured" = re-run on 2026-09-22 against `3edb6c91`
on a host that was running other self-test jobs (the absolute times are ±20 %; umbrella §8 (4)).

## §0 Why a plan, and what "deletion" means here

Four fix passes (`79309486`, `b325c668`, `3545f944`, `3edb6c91`) were each attested by a fresh adversary
and IMP findings ran **6 → 5 → 7 → 4**. Every pass ADDED a mechanism, and every later attestation found
holes inside the mechanism the previous pass had added (`memory/feedback_defer-accumulation-signals-mis-drawn-slice.md`).
CLAUDE.md's edge-dense rule applies: §2 has eight invariants and there is no canonical algorithm.
"Deletion" is a count, not a mood: §4's ledger deletes **12** mechanism units and adds **10**.

## §1 The property (one sentence per invariant)

**DECLARED.** A row is declared by the self-test file whose code constructs it, when that file runs in
its OWN child interpreter whose `plan_memo*` imports are served from `SOURCES ∪ disk` text (§4 M2).

- **I1 MEMBERSHIP**: the set of rows run equals the set declared.
- **I2 ONCE**: each declared row runs exactly once. Identity `(co_filename, ordinal)` is unique, and
  mutation labels and control-table keys are unique.
- **I3 KILL SET**: every mutation row names a non-empty set of existing controls, and every case is exactly one
  table entry.
- **I4 ISOLATION**: a file's collected rows are a function of the texts its child imports. Scope, stated
  as exactly what M2 isolates: interpreter state (`sys.modules`, builtins, stdlib module attributes,
  `sys.path`, environment variables) between files. It does NOT isolate the filesystem, a file's own import
  closure within its child, or anything outside the process.
- **I5 ONE VALUE**: the value is a `Rows` whose leaves are all `str` / `int` / `None` / `tuple`, and it is
  computed once per distinct input key (M3).
- **I6 PATCHABLE**: every mechanism of I1–I5 (constructors, collector, child bootstrap, partners) can be
  patched by a mutation row, and the patched TEXT is what executes.
- **I7 PINNED**: each partner drives the real control's own function, and the derived sweeps (M8) leave
  every survivor with an arm or with an in-code structural reason.
- **I8 REACHED** (S7): every file of the self-test population is either collected or reached from a
  root. The roots are the collector's population, and the import graph includes literal-string `import_module` edges.

## §2 Coupled invariants — all C(8,2) = 28 pairs, each naming its mechanism

- I1×I2 **identity key** `(co_filename, ordinal)` recorded by the constructor; a row is kept iff its
  `co_filename` is the collected file (a helper's rows are its own, never its importer's).
- I1×I3 **constructor precondition** (M6): a row cannot be constructed without a non-empty kill set.
- I1×I4 **child per file** (M2): a row's presence depends only on texts that child imports.
- I1×I5 **`rows()` key** (M3): one value per (here, closure texts); readers never collect.
- I1×I6 **text map** (M2): the child imports `SOURCES ∪ disk`, so a patched collector or constructor drops or
  adds rows in the run that patched it.
- I1×I7 **same-function partner** (M7): the partner calls `collect(planted_here)`, the real call is `collect(HERE)`.
- I1×I8 **root = collector population** (S7): a population file is collected or reached; neither is red.
- I2×I3 **refusing merge** (M5): one merge site; a repeated key raises.
- I2×I4 **provenance filter** (M1): a shared module re-executed in N children yields its rows only in its own child.
- I2×I5 **uniqueness on the value**: labels are checked once, on `Rows`.
- I2×I6 **patched-row override**: `patched_module`'s fragment REPLACES keys only for its own row. The
  refusing merge governs the unpatched table.
- I2×I7 **sweep over the merge**: the refusing merge's clauses are in M8's population.
- I2×I8 **one population derivation**: rows and fragments both range over `files()` (S6), so no file is
  counted by two predicates.
- I3×I4 **names as data**: a row carries control NAMES (str) across the process boundary. Names resolve in
  the parent at run time (the existing unknown-control FAIL).
- I3×I5 **leaf-type walk** (S2): the kill set is a tuple of str, so nothing can clear it after collection.
- I3×I6 **merged table resolution**: a row naming a patched module's control resolves against that row's table.
- I3×I7 **precondition swept**: M6's non-empty clause is an M8 clause-drop site.
- I3×I8 **fragment totality** (S6): every non-row population file returns `registry()`, so no control
  module is unmerged.
- I4×I5 **key = closure texts**: the child reports the `plan_memo*` modules it imported. The cache key is
  their texts, so an unrelated patch hits the cache.
- I4×I6 **bootstrap in the map**: the child's bootstrap is collector-module text sent through the same map.
- I4×I7 **builtins arm** (§6): a planted file mutates `builtins.sorted` and `ast.dump`; the others' rows are unchanged.
- I4×I8 **string edges**: a non-collector literal `import_module` of a row file is red. Otherwise it would
  execute a row file in the parent.
- I5×I6 **key includes SOURCES**: a patched run cannot read the unpatched value.
- I5×I7 **leaf-walk arm**: the partner hands the walk a tuple holding a list and a plain object (both refused).
- I5×I8 **per_file**: `Rows.per_file` lists every collected file, including those with zero rows, so I8 reads the same value.
- I6×I7 **sweep rows are data** (S8): generated rows are produced by a generator the collector runs, counted
  in `rows()`, and patched through the ordinary path.
- I6×I8 **call-time import form**: the partners' call-time reach of the harness becomes a function-local
  `import` statement (not a string), so S7's string rule stays total.
- I7×I8 **membership partner**: M7's planted directory carries an unreached file and a string-imported row file.

## §3. Spec coverage map

No web spec governs this subject. The rows cite the Python 3 documentation (docs.python.org; section
numbers NOT webref-verifiable, checked by hand) for the semantics the mechanisms rely on, plus local policy.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Python Language Reference §5.3.4 The meta path | serve text | `plan_memo*` names from `SOURCES ∪ disk`; others fall through | child bootstrap (NEW) | ✓ | no |
| Python Language Reference §5.4.1 Loaders `exec_module` | execute | patched text executes (probe §4 M2) | child bootstrap (NEW) | ✓ | no |
| Python Data Model §3.3.1 `__hash__` | immutability | hash is not immutability (S2 probe) | leaf-type walk (NEW) | ✓ | no |
| Python Data Model §3.2 standard type hierarchy, code objects `co_filename` / `co_name` | attribution | top level vs lambda / genexpr / def | constructor (M1) | ✗ listcomp reads `<module>` (PEP 709) | no |
| Local policy (this plan §1 I1/I2) | declare | construction in an allowed ancestor | constructors; static allowlist | ✓ (§4 M1) | no |
| Local policy (this plan §1 I3) | kill set | empty / unknown names | `mutant` (NEW); `run` | ✓ | no |
| Local policy (this plan §1 I4) | isolation | builtins / stdlib / `sys.modules` | child per file | ✗ filesystem (§8 D-3) | no |
| Local policy (this plan §1 I5) | handoff | every reader; per patched run | `rows()` | ✓ | no |
| Local policy (this plan §1 I7) | criteria | BoolOp / Compare / key tuples | sweep generator (R2) | ✗ residue counted (M8) | no |
| Local policy (this plan §1 I8) | reach | unreached file; string import | membership control | ✓ | no |

**Breadth**: no web spec, and M=10 rows. The slices are set by §2 counts (§7), not by breadth.

### §3.1 User-input touch audit

None. The mechanisms read only `.claude/tools/` text; no memo text reaches them.

## §4 Mechanisms and the deleted/added ledger

Baseline (`python3 $T/plan-memo-umbrella-check.py --self-test --mutants` at `3edb6c91`): 747 controls,
472 rows, rc 0. Row files (verified 2026-09-22, `len(X.MUTANTS)` / `len(X.CASES)` over
`H.registry_modules`): MUTANTS in 7 files (79/101/22/110/38/93/29), CASES in 6 files (164/192/154/59/71/26).

**M2 — collection: chosen by MEASUREMENT (S1).** Draft 1's in-process purge + `import_module` is refuted
by three probes:
- **Disk text, not patched text**: `cd $S/reg-ax2/model && python3 sandbox.py` prints `sink held by
  collector before purge: []`. The rows land in the purged copy's sink, and the helper-lambda row is
  dropped silently.
- **Builtins leak**: `cd $S/reg-ax4/sinkp` then `pm_col.collect('pm_rowb')` gives `(0, 4, False, False)`. That
  is 0 rows in the collector's sink and 4 in the fresh copy's. `collect('pm_evil')` then leaves
  `builtins.sorted` replaced after restore, and `sorted([3,1,2])` returns `[1, 2]`.
- **The subprocess rejection was unsound**: patched texts are data and can be sent to a child.

Prototypes `$S/regplan/s1/{loader,child,cand,toy,cost}.py` implement:
(a) a child per file, fed the `SOURCES ∪ disk` text map through a `sys.meta_path` finder, with rows returned as JSON;
(b) an in-process finder over the same map, with the sink in a module outside the purged prefix;
(b+) = (b) plus a `builtins.__dict__` snapshot and restore.
The four probes are `python3 $S/regplan/s1/toy.py`, and cost is `python3 $S/regplan/s1/cost.py $T` (26 self-test
files, verified 2026-09-22 as `is_selftest` over `files()`; the real tree's full text map, twice):

| candidate | patched text executes | sink owner | builtins leak | stdlib leak (`ast.dump`) | cost, 26 files |
|---|---|---|---|---|---|
| (a) child per file | True (3/3 rows `PATCHED:`) | child; rows as data | False | False | 1.14 s / 1.12 s |
| (b) in-process finder | True | collector's `SINK` | **True** | **True** | 0.36 s / 0.36 s |
| (b+) builtins restore | True | collector's `SINK` | False | **True** | not measured (≈ b) |

**Chosen: (a).** It is the only candidate with no leak, and I4's scope (§1) is written to what (a) isolates.

**Cost of re-collection per patched run.** 66 mutation rows patch a self-test file (loading `mutants()`, counting
`r[1] in SELFTEST`). With the cache keyed on closure texts, each of those rows re-runs only the files whose import
closure holds its file. The measured closures give 236 child runs, about 10 s. That figure is an ESTIMATE:
236 × the 1.12 s / 26 per-file mean. R1a measures it. The budget is the tools job's, not the wire's (P1).

**M3 — the value (S2, S3).** `Rows = (cases, mutants, per_file)`. `cases` holds `Case` tuples whose `files`
is sorted `(name, text)` pairs, which changes one reader: `run_on` iterates pairs. `mutants` holds 5-tuples
whose last element is a tuple of str. Immutability is checked by a leaf-type walk (only str / int / None /
tuple are allowed), because hashability is not immutability.
`python3 $S/reg-ax4/hashprobe.py` shows that a tuple holding a plain object with a list attribute hashes OK,
and so does one holding a function. Today's rows also carry `None` (`sibling=None`, `$T/plan_memo_selftest_cases.py:97`).
`hash` is kept as one partner arm, not as the definition. `rows(here=HERE)` is memoised per (here, closure
texts ∪ SOURCES), on the collector module instance. Readers resolve the collector at CALL time
(a function-local `import`). Partners call `collect(planted_here)` directly, because they test the
collector, while the runner, `run` and `_census_rows` call `rows()`.

**M1 — declaration by construction (S4, S5).** `case` / `acase` / `rcase` / `mutant` (NEW) append
`(co_filename, ordinal, row)` to the child's sink and RETURN the row. Attribution is by `co_filename`,
never `co_name`. Base rows leave the constructor module: `python3 -c` over `$S/reg-ax4/sinkp/pm_selfcons.py`
attributes a constructor-module top-level row to `<frozen importlib._bootstrap>`.

Names derived from rows (`X = CASES[-1].name`: 53 top-level assignments, r26 19 + r42 34) become explicit
`name=` string constants in a NON-row vocabulary module. The 92 names (verified 2026-09-22) the mutants modules import from cases
modules (AST count of `ImportFrom` names, module `plan_memo_selftest_cases*`, in `plan_memo_selftest_mutants*`:
r26 23, inline 2, r30 67) move there too. That vocabulary set is DERIVED by measuring those import edges,
not by a hand list.

Post-construction replacement (`_OWN[-1] = _OWN[-1]._replace(files=…)`, `$T/plan_memo_selftest_cases.py:613`)
is forbidden, because the constructor takes every field.

**Static rule, inverted to an ALLOWLIST**: a constructor call's ancestors may be only `Module`, a
top-level `For`, `Expr` or `Assign`. `python3 $S/reg-ax4/staticrule.py` prints `flagged_by_rule=False`
for all 6 of draft 1's escape shapes (lambda, IfExp, BoolOp, empty for, match, assert).

**Runtime backstop**: the constructor refuses a caller frame whose `co_name != '<module>'`.
`python3 $S/reg-ax4/frameprobe.py` on 3.14.6 reads `<lambda>` and `<genexpr>`, but a list comprehension
reads `<module>` (PEP 709, inlined). The static allowlist is therefore what refuses comprehensions.

*Closes by construction*: IMP-4, MIN-6, MIN-7 D/L/N (the lists and the partition are deleted).

**M4 — row files are import-terminal (not "leaf": the harness already uses "leaf" for a self-test
module without `registry()`).** No file imports a row file except through the collector's child. This is
checked over the full import graph: function-local imports and literal-string `import_module` edges,
6 today (`grep -n 'import_module("plan_memo' $T/plan_memo_selftest_*.py`: population 30/130/131, ratchets 579/580, registry 34).
Four of the six (rows / `CRITERIA`) are deleted with M1/M3. The two call-time harness reaches become
function-local `import` statements.

**M5 — one merge, one population (S6).** Every non-row file of `files()` defines `registry()` returning ONLY
its own keys; a missing `registry` is red. That is totality, not a second predicate. The row files are
`Rows.per_file`'s non-empty entries, so there is one derivation. The runner is the ONE merge site and it refuses
a repeated key. The nested merges are deleted: 57 keys appear in more than one fragment's `registry()` today (verified
2026-09-22: a `Counter` over the 9 fragment modules' `registry()` keys; R1's record said 31, by another count).

**M6 — constructor preconditions**: `mutant` refuses empty or non-str `controls` and an empty `find`.

**M7 — same-function partners over planted directories.** Every population, membership and collection
control is `verdict(here)`. The partner calls the same function over a FRESH temporary directory per arm, so
no stale `__pycache__` answers for a re-planted name (MIN: bytecode). The planted shapes are: a non-glob
`memo_extra.py`, a `from` edge, an unreached file and a string-imported row file.

Backstop: the lines of each mechanism function that the real control executes must be a subset of those its
partner executes (`_count_line_sites`).

**M8 — criteria completeness, derived (R2; S8, S9, S10).** The population is the functions M7's backstop
records as executed by partners, so there is no hand tuple and `_FAMILY` goes. The operators are:
- **clause-drop**: each `BoolOp` operand and `Compare`, forced `True` and `False`;
- **key-projection**: at `x in R` / `x not in R`, drop one component of a tuple `x`, or union over
  `V[k]`; widened to `==` with a tuple operand and to a subscript with a tuple key.

Generated rows are DATA from a generator the collector runs, counted in `rows()`. There is NO equivalence table:
a survivor gets an arm, or the operator is not applied at that site with a structural reason written in the code.
Non-termination (`_membership_verdict` `u in seen` → `False`) gets a cycle-planted arm under `_WorkExceeded`.

Residue outside the widened operator, over the prototype's function set: 10 `==`/`!=` compares and 12
non-constant subscripts with non-tuple keys. One tuple-key subscript (`$T/plan_memo_selftest_ratchets.py:77`)
is inside. Count: AST walk over `_loops … _kind_question_verdict`, `collect`, harness `files … _import_order`
and population `_imports … _roots`.

Prototype survivors (the scripts are not committed; args in full):
- `cd $S/attest3ed && python3 clausedrop2.py plan_memo_selftest_ratchets plan_memo_selftest_ratchets _loops,_header,_is_truncation,_intends_truncation,_credit,_scope_verdict,_qualified_callers,_kind_question_verdict`
  → 48 sites (verified 2026-09-22) / 96 / **11** (57 s wall, host busy);
- the same script over `plan_memo_selftest_registry … collect` → 6 / 12 / **1**;
- harness `files,is_selftest,_assigns,registry_modules,import_name,_import_order` → 19 / 38 / **4**;
- population `_imports,_membership_verdict,_roots` → 5 / 10 / **3**;
- `python3 $S/regplan/keyproj.py $S/attest3ed/t/.claude/tools plan_memo_selftest_ratchets plan_memo_selftest_ratchets <same 8>`
  → 6 / **4**, including IMP-1's union.

Classes neither operator sees: literal edits, argument substitution (M7 covers IMP-2's), statement deletion,
tie order (M/O), the residue above, and data-table coarsening.

**Deleted / added ledger** (units of mechanism; "modify" is neither):

| deleted (12) | added (10) |
|---|---|
| D1 13 per-file `CASES`/`MUTANTS` bindings | A1 child bootstrap + text finder |
| D2 `spellings(into)` target | A2 closure-keyed `rows()` cache |
| D3 `_OWN` / `_SEALED` | A3 child sink |
| D4 both tuple seals | A4 `mutant` constructor |
| D5 `registry_modules` + `_assigns` (content partition) | A5 leaf-type walk |
| D6 `collect` (in-process) | A6 ancestor allowlist + `co_name` backstop |
| D7 `cases()` / `mutants()` | A7 one refusing merge site |
| D8 `CRITERIA` | A8 vocabulary module for row-derived names |
| D9 `criteria_rows_control` + its row | A9 partner line-subset backstop |
| D10 the nested fragment merges (8 `update` sites) | A10 sweep generator (2 operators) |
| D11 `_FAMILY` hand tuple | |
| D12 `patched_module`'s leaf / owner fork | |

Modified, not added: `_roots` (S7), the import graph (string edges), and `_IMPORT_SEAMS` handles (S11: derived from
the harness's exports, and an unlisted importer of an export is red).

**Rejected**: seal harder (E3a' bypasses a seal); static row counting (verified 2026-09-22: 4 of 6 cases files construct in
top-level `for` loops: 174/192, 51/59, 49/71, 20/26 syntactic calls vs rows); pinned counts (stale per commit).

## §5 Finding map — no finding without a mechanism or a stated exclusion

| # | Finding (measured) | Closed by | Slice |
|---|---|---|---|
| IMP-1 | e1 probe A (per-question sanction → union): rc 0 | M8 key-projection | R2 |
| IMP-2 | e1 probe G (`"*.py"` → `H.GLOB`): rc 0 | M7 partner + line backstop | R2 |
| IMP-3 | e23b E3a' 422 rows / E3d' 371 rows, rc 0 with `--mutants`; 472 then 422 on re-collection | M2 + M3 | R1a |
| IMP-4 | e23b E2b (own `CASES = []`, rows via `spellings([])`): rc 0 | M1 | R1b |
| MIN-5 | E3c' (100 rows' controls cleared): rc 0 | M6 + M3 leaf walk | R1b |
| MIN-6 | E2c alias: 510 rows rc 0; no label check | M1 + M5 | R1b |
| MIN-7 | e1 D / L / N: rc 0 | M1 deletes the partition | R1b |
| MIN-8 | e1 F: rc 0 (e1f `--mutants`: rc 1, 110 survived, only incidentally) | M7 `from` edge | R2 |
| MIN-9 | e1 C / E / H: rc 0 | M8 clause-drop (all three among the 11) | R2 |
| MIN-10 | 6 mutants docstrings "file-name rule decides" (`grep -c`) | deleted with D1's headers | R1b |
| MIN-11 | a DATA-TABLE gap (S11): `files` is imported by ratchets / properties / records and by `plan_memo_selftest_mutants.py:93`, outside the seam | `_IMPORT_SEAMS` handles derived | R1b |
| MIN-12 | printable docstring "two report modules" vs output `over 4 report module(s)` | clerical | R1b |
| N-1 | a fragment key shadowing another: rc 0, 747 | M5 | R1b |
| N-2 | 3 more key-projection survivors | M8 | R2 |
| N-3 | population clause-drop 3 / 10 | M8 + S10 cycle arm | R2 |
| N-4 | `work` merge of `document_registry` dropped: 740, rc 0 (also `$S/reg-ax3-p1/p1.out`: `740 control(s)`, `all controls behaved`) | M5 totality | R1b |
| M, O | e1 order reversals: rc 0 | equivalent: `Rows` sorted by `(co_filename, ordinal)`; O is two valid topological orders | — |

## §6 Acceptance — every hole-showing probe becomes a permanent arm or row

- **A** → generated key-projection row; arm: a caller sanctioned for `_claims` that calls `_phrases` is red.
- **C / E / H** → generated clause-drop rows + arms (a credited-and-exempt loop is exempt; a non-`for`
  `[:1]` line does not intend truncation; a module-level `_claims` call is red).
- **D / L / N** → deleted subject; arms: rows built in a top-level `for` are collected; a file binding no list is
  collected by its constructions; a non-self-test file constructing a row is red.
- **F / G** → M7 planted `from` edge / `memo_extra.py`.
- **E2b** → arm: rows constructed with no own list are collected.
- **E2c** → arms: importing a row file is red (M4); one label from two files is red (M5).
- **E3a' / E3b / E3f' / E3d' / E3h** → arm: a string-imported row file is red (M4). Arm: a planted file
  mutating another's module attributes, `builtins.sorted` and `ast.dump` leaves the other's rows equal (M2, from `toy.py`).
- **E3c'** → row: drop M6's precondition; arm: `mutant(…, ())` raises.
- **dupfrag / N-4** → rows: an `update` merge in place of the refusing one; a fragment returning a sibling's keys.
- **ax2 sandbox** → arm: a helper lambda's row is refused by the backstop rather than dropped silently.
- **ax4 hashprobe / selfcons / staticrule** → arms: the leaf walk refuses a plain object and a list; a
  constructor-module row is red; each of the 6 escape shapes is refused.
- **M8 survivors (23)** → arms or in-code structural reasons.
- **Not converted**: `$S/attest3ed/probe3` (rc 0, a permitted edge); e1 B / I / J / K (already red); e23 E2a / E2b' /
  E2d / E2e (crashes; E2d stays loud because an import error in a child is red).

**Gates on each slice head**: `--self-test --mutants` rc 0; `bash $T/plan-memo-umbrella-selftest-trip-wire.sh`
rc 0; `bash scripts/trip-wires.sh` rc 0; every probe above re-applied WITH `--mutants` is rc 1; M8 has 0
unexplained survivors (R2); the tools-job wall clock before and after (P1: M8's cost is an input to
A-iii item 2, the suite scheduler, and does not move to a `--mutants` fallback, because the wire already runs
`--mutants`); and a fresh-agent attestation by enumeration with §2's 28 pairs as its checklist.

## §7 Slices and PR boundary

By CLAUDE.md's edge-dense rule, **R1 and R2 are separate PRs, not #510 commits** (P4).

**Split of R1, decided by counting §2 pairs among the invariants each slice's mechanisms change:**
- R1 whole = {I1, I2, I3, I4, I5, I6, I8}: 7 invariants, **21 pairs**.
- R1a collection (M2, M3 over TODAY's lists: the child returns a file's own list as data) =
  {I1, I2, I4, I5, I6}: **10 pairs**.
- R1b declaration / terminal (M1, M4, M5, M6, S7) = {I1, I2, I3, I6, I8}: **10 pairs**.
- The two slices share 3 pairs (among I1, I2, I6), which is their interface: the row key and the text map.

**Decision: split.** Each part is under half of the whole (10 vs 21). R1a is independently landable (it
closes IMP-3 on the existing lists). R1b then deletes the lists. Under R1a the content partition (D5)
still names the files; R1b deletes it, so the interim lasts exactly one PR.
Order: **R1a → R1b → R2**. Each is plan-reviewed on its own.

- R1a owns the umbrella amendment row (P3): the ratified "`MUTANTS` table" wording at umbrella line 320
  (`grep -n 'a \`MUTANTS\` table' <umbrella>`) and §7's slice list.
- R2 owns umbrella §8 (4) and (16), whose triggers fired (M8 adds rows and a table).

**P2, ordering against the gate-population sub-umbrella** (`elidex-wt-gatepop`,
`docs/plans/2026-09-plan-memo-gate-population.md`): registry R1a/R1b land BEFORE its ρ/α/β/γ slices
(all post-#510). The reason is that β touches `plan_memo_selftest_ratchets.py`, and every slice adds controls
and mutants in new modules that R1b's constructors must serve. The mirror sentence in that memo's §7 is
owed by R1a's PR, because that memo is on another branch and is not edited by this commit.

**The one user question.** If R1a / R1b / R2 are follow-up PRs, #510 merges carrying attest3ed's
IMP-1..4. That is an OWN deferral, moving the umbrella's count **14 → 15**, which the defer policy routes to
a cap PAUSE. Merge #510 under that pause (with this memo as the deferral's owner), or hold #510 until R1a lands?

## §8 Defer and open defects (every blind spot, each with cover or owner)

- **D-1** a control function a module defines but does not return from `registry()`: no cover. Owner: R1b's
  plan-review decides whether totality extends to functions.
- **D-2** prose staleness (MIN-10..12's class): no detector. Open, and not owned by this program. Trigger: the
  next prose-only finding.
- **D-3** I4 outside interpreter state (filesystem writes, a file's own closure): stated scope, open.
- **D-4** a non-literal string import (`import_module("plan_memo_" + x)`): M4's graph cannot see it. Open.
- **D-5** M8's residue (10 `==`, 12 subscripts) and its blind classes: counted in R2, open beyond it.
- M8's population is covered by R2 (derived); M2 / M4's remaining residue is D-3 / D-4.
