# Plan — Slice A-ii: the plan-review gate fails closed, and says which capability it lacked

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-ii** (the 2026-08-01 re-slice).
Terminal unit under that boundary (§9). **Branch**: new, stacked on **A-i's landed head**.
**Nature**: one gate's failure semantics + one gate-contract change. Zero `crates/**` diff, zero CI topology.
**Status**: plan-memo, **draft 1**. `/elidex-plan-review` **required before implementation**.

**This memo carries no measured digits of its own.** Every quantity is printed by a function in
`docs/plans/2026-07-citation-hygiene-A-rederive.sh`; the memo cites the function name and a reviewer runs it.
`§4.2`'s control flow is **executable** — `rederive armmatrix` grafts it onto a copy of `preflight.py` and
runs every state with the competing predicates instrumented side by side.

### §0.1 What A-ii is

A-i landed a shared `spec_labels.py` and made `preflight.py` **import** it. That import is the first thing
that can make the gate's label resolution *fail*, and A-i deliberately left the failure handled the way the
carve left it: `except Exception: _shortname_for = None`, which routes a **process** failure into the
per-row *unmapped* bucket — a documented soft-warn. Result: every row classified as *author cited a spec I
do not know*, and the gate **exits 0 having verified nothing**.

A-ii closes that, and two neighbouring holes of the same shape:

1. **Nothing distinguishes "I cannot map *this* label" from "I cannot map *any* label."** One return value
   carries both questions (J1).
2. **A memo whose §3 rows are *all* unmapped prints no `citation verify:` line at all** and exits 0, with
   both capabilities present. Live, not hypothetical.
3. **A slice implementing no spec logic must author fixture citations** and then receives `citation verify:
   ok` as its headline — the gate reporting on itself. A-ii lets a §3 declare no spec surface.

**A-ii changes no lookup semantics** (B's) and **schedules nothing** (A-iii's).

⚠ **A-ii opens with two CRITs inherited from the merged memo's round 9**, stated here as defects to fix
rather than as history — see §4.2.3 items 6 and 8. Both are cases of a summary line asserting something the
process could not establish, which is §1's own failure shape surviving the fix for it.

---

## §0.5 Spec citation table

A-ii implements no spec logic. The citations below are the ones its **fixtures** carry, all looked up with
`.claude/tools/webref`. → `rederive citations`

| Cite | § | Exact title | Anchor | Which fixture, and why it is load-bearing |
|---|---|---|---|---|
| `WHATWG HTML §4.10.21` | HTML §4.10.21 | Constraints | `#constraints` | row 1 of `labelled.md` / `dedup.md` / `nospec-and-table.md` / `fenced-marker.md` — the mapped row every capability state is measured against |
| `WHATWG HTML §4.10.21.2` | HTML §4.10.21.2 | Constraint validation | `#constraint-validation` | **row 2 of `labelled.md`** — a *second distinct* pair, so P1b checks `2 unique` |
| `HTML §4.10.21` (alias spelling) | HTML §4.10.21 | Constraints | `#constraints` | **row 2 of `dedup.md`** — resolves to the *same* pair as row 1; the only shape that takes `seen_pairs`' dedup `continue` |
| `Fetch §2.2.5` | Fetch §2.2.5 | Requests | `#requests` | the only row of `alias.md` — **P10 asserts this verifies** |
| `CSSOM VIEW §4.2` | CSSOM View §4.2 | The MediaQueryList Interface | `#the-mediaquerylist-interface` | `allunmapped.md` / `malformed.md` — chosen because `CSSOM VIEW` is **absent from A-i's pinned map** |

⚠ **These are fixture citations, not A-ii's own §3.** A-i established the rule that *a slice's §3 may only
cite labels that slice's own resolver maps*; §3 below follows it, and `CSSOM VIEW` appears here only as
fixture *content*, where being unmapped is the property under test.

---

## §1 Ideal anchor — a gate reports on the thing it audited, or it reports on itself

Three failures, one shape. A gate's output is a claim about the artifact under review. When the gate's own
infrastructure is missing, the honest output is a claim about the **gate**.

**The corollary that drives the edit set**: *a capability is a process-level fact and must be established
once, before the data loop.* "I cannot map *this* label" is a datum about one row; "I cannot map *any*
label" is a fact about this process. Discovering the second by watching the first makes the failure look
like data — and, as §4.2.2 measures, makes the fix's correctness depend on the *content* of the memo under
review.

⚠ **And the corollary binds the reporting layer, not only the classification.** That is the lesson the two
inherited CRITs teach: a line that says `unmapped-label rows: 2` when the mapper never ran, or `K=1` against
an empty spec list, is the same inversion one layer out. Every summary line either states its basis or is
not printed.

---

## §2 Coupled invariants

- **J1 — capability ≠ datum.** A row is *unmapped* only if the mapper ran and declined. One return value
  (`None`) must not carry both questions. ⚠ J1 forbids the two questions sharing a *return value*; it does
  **not** require them to share a *site*.
- **J1b — J1 at the reporting layer.** No summary line may assert a classification the process did not
  make. This is J1's consequence and it is listed separately because the merged memo satisfied J1 and
  violated J1b in seven measured states.
- **J2 — the two capabilities degrade the same direction.** Verifying needs the `webref` CLI *and* the
  label map; measured on A-i's head, one hard-fails and the other exits 0.
- **J3 — one degradation must survive.** `--no-verify --no-grep-pass` must keep working with the tools tree
  absent.

**Pairwise intersections**, because these cannot be applied one at a time:

| pair | intersection |
|---|---|
| J1 × J1b | the row loop keeps two arms (control flow) while the summary keys on the *capability*, not the row count — the two must not be re-derived from each other |
| J1 × J2 | one verdict for both causes, but the **diagnostic** names each absent cause separately |
| J1 × J3 | classification still runs under `--no-verify`, so the loop must not raise — draft 5 of the merged memo made it raise and turned J3's row into a traceback |
| J1b × J2 | with only the *CLI* absent the mapper **ran**, so rows are classified and remedy 1 is correct; keying J1b on the union verdict reports a problem the run does not have |
| J2 × J3 | `--no-verify` suppresses the hard fail by construction, so the verdict must be consulted at the verification stage, not at `main`'s top |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | fixture | the labelled `§3` row a fail-closed run must still map | §4.4 — `test_preflight.py` fixtures | ✓ — authored, not discovered | no |
| WHATWG HTML §4.10.21.2 Constraint validation | fixture | `labelled.md` row 2 — a second distinct pair | §4.4 — same fixture set | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | fixture | the alias-spelling row **P10 asserts verifies** | §4.4 — `alias.md` | ✓ | no |

**Breadth**: measured by the gate on this memo. All three labels are pinned by A-i's map, per A-i's rule.

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** The inputs are the plan-memo's path *and its content*: `parse_spec_cell`
extracts a label and a section number from cell text and `verify_citation` passes **both** to a subprocess,
so memo content steers control flow (§4.2.2). Both argv elements stay bounded — `section` by
`SECTION_REF_RE` (untouched), `shortname` by A-i's pinned map.

**Discovery method.** Measured against `origin/main`, never the branch; a proposed patch is *measured*, not
read; claims about *where* are grepped by concept. **And a check must derive its own coverage, not only its
values** — the umbrella constraint that round 8 and round 9 both forced.

---

## §4 The edit set

### §4.1 Slice routing

| Concern | Slice | Why |
|---|---|---|
| the capability verdict, both act-sites, the four remedy strings, the summary's basis rules | **A-ii** | the gate's failure semantics |
| the no-spec-surface declaration and its recognition rule | **A-ii** | the gate's contract of record |
| `SKILL.md` — Hard-fail, Soft-warn and **Flags** bullets, `--no-verify`'s meaning, Pre-condition #1 | **A-ii** | a gate's contract of record travels with the gate |
| `spec_labels.py`, the three consumers, `DESIGN.md` | **A-i** | landed |
| the catalog fall-through and all lookup semantics | **B** | — |
| `python-suites.sh`, `[tasks.tools-test]`, the `tools` job | **A-iii** | — |
| `axes.md`'s Axis 4 detect, `CLAUDE.md` § "Spec citation" | **C** | review-axis requirements |
| `grep_pass.py` reporting a wrong repo root as one HARD finding *per referenced path* | **C** | §1's class in a neighbouring gate |

**`SKILL.md`'s required coverage, as classes to grep** — not a list read off, which is the shape that was
wrong at most items when the merged memo tried it for B:

| Class | Why it is false after A-ii |
|---|---|
| "unrecognized spec labels" as **one** soft-warn class | item 7b makes it two (unknown label / label-less), with distinct remedies |
| any soft-warn described as **unconditionally exit-0** | item 4 hard-fails when verification is requested and the capability is absent |
| "no table after the heading" as an **unconditional** hard fail | §4.2.5 makes it conditional on the marker |
| `--strict-breadth`'s description | §4.2.5 makes it a no-op on the marker path |
| any statement that the gate **verifies** whatever it does not hard-fail on | items 5 / 7c: it may report `n/a` with a stated basis |

### §4.2 Land the capability fail-closed

#### §4.2.1 The measured asymmetry, and the instruments that measure it

Removing the CLI hard-fails; removing the **import** leaves every row unmapped, nothing verified, **exit 0**,
and a wrong-cause remedy naming the file that failed to import. **This case does not exist on
`origin/main`** — there `shortname_from_label` reads a module-local dict with no import to fail. The
asymmetry is created by A-i moving the map, which is why the slice stacked on A-i owns it.
→ `rederive remedies`

⚠ **The map axis is flipped by an in-process import block, never by removing the tools tree.** Three
candidate instruments, all three signals measured — the third row is the one the merged memo used for eight
drafts, and it flips **neither** axis:

| instrument | `WEBREF.is_file()` | map import | child `webref` rc | is it a §5 state? |
|---|---|---|---|---|
| in-process `sys.meta_path` block | True | FAIL | **0** | **yes** — the map axis |
| `mv .claude/tools/webref` (the 16-line shim) | **False** | OK | 2 | **yes** — the CLI axis |
| patch `preflight.WEBREF` to a nonexistent path | **False** | OK | n/a — never spawned | **yes** — the CLI axis, in-process; what §4.5's pins use |
| `mv .claude/tools/_webref` (the tree) | True | FAIL | **1** | **NO** — neither axis |

→ `rederive instruments`

⚠ **A fixture is named for a *state*, and the state is a property of the resolver it runs against.**
`allunmapped.md` is all-unmapped under `origin/main`'s 15-key dict and under A-i's pinned map — but **not**
at the merged branch's head, where the catalog resolves `CSSOM VIEW` → `cssom-view-1` and verifies the row
it exists to leave unverified. The harness pins the resolver for every after-A measurement.

#### §4.2.2 The tri-state cannot live in `shortname_from_label`

Applied verbatim in a sandbox (a `TOOLS_UNAVAILABLE` sentinel returned from that function and hard-failing
in `main`), with the map absent: a memo whose §3 rows carry spec labels **exits 1** ✓, and a memo whose rows
open with `§` **still exits 0** ✗. Cause is the function's first line:

```python
def shortname_from_label(label: str | None) -> str | None:
    if not label:
        return None          # ← taken before any availability check below
```

`parse_spec_cell` returns `cell[:m.start()].strip()`, so a cell beginning with `§` yields `""` and every such
row short-circuits before the capability is consulted. The gate's fail-closed property becomes **a function
of the reviewed memo's cell formatting** — J1 restated as a defect.

#### §4.2.3 The fix — two static causes, one verdict, two act-sites

1. **Two causes, both static process facts, evaluated once at `main`'s top**: `WEBREF.is_file()` and
   `_shortname_for is None`. The verdict is their union. **Each cause is also kept separately**, because
   items 7 and 7c key on `map_missing` specifically, not on the union.
2. **`shortname_for` stays `str | None`.** No tri-state — that machinery existed only to carry a dynamic
   third cause that leaves with the widening (B's).
3. **`shortname_from_label` keeps returning `None` when the map is absent**, and the row loop keeps its two
   arms. Under `--no-verify` the hard fail is suppressed by construction, so a raising branch turns J3's row
   into a traceback (J1 × J3).
4. **Act-site 1 — the hard fail**, at the verification stage, not `main`'s top: acting at the top would
   hard-fail a no-spec-surface memo, which §4.2.5 forbids. The trigger is
   **`not args.no_verify and (citations or (unavailable and data_rows))`**. On §4.2.5's path this arm is not
   merely False — that path **returns before `data_rows` is computed at all**, so it is unreachable.
5. **Act-site 2 — the reporting arm, whose guard is the capability verdict and NOT the stage's entry
   predicate.** Three candidates, measured over every state:

   | candidate | measured True in | verdict |
   |---|---|---|
   | `not no_verify and data_rows and not seen_pairs` | every row where the arm must be silent, plus the ones where it must fire | **false positives throughout** |
   | a `verify_ran` flag set where the loop is entered | **nothing — including the rows it exists for** | **red** |
   | **A-ii ships** `not no_verify and data_rows and not unavailable and not seen_pairs` | exactly the capabilities-present, none-resolvable rows | ✓ |

   The rejected flag is the instructive one: **any flag set inside the verification stage inherits item 4's
   entry predicate, which is False in exactly the row the reporting arm exists for.** The guard must be the
   process-level verdict. The line is `citation verify: n/a (0 of N rows resolvable)`, **N = `len(data_rows)`
   including malformed rows**, because `malformed_hard_fail` is decided separately and the reader is being
   told what the denominator was. → `rederive armmatrix`
6. ⚠ **CRIT inherited — the basis qualifier reads the un-partitioned counter.** Measured: a memo whose only
   row is **label-less** prints `unique specs (K): 1 (1 of 1 counted by label spelling)`. "Counted by label
   spelling" for a row that has no label. `unmapped_rows` is incremented for **both** classes while items 7b
   and 7d partition every other consumer. **A-ii's rule**: the basis names the classes it counted —
   `(<u> unknown-label, <l> label-less, of <N>)` — and is emitted only when the mapper ran.
7. **The per-row soft-warn is suppressed when *the map* is absent — not whenever the verdict is absent.**
   ⚠ Keying this on the union verdict is wrong and only the executable showed it: with only the *CLI*
   missing, the mapper ran and declined, so the row genuinely **is** unmapped and remedy 1 is the correct
   diagnosis. Measured, the two remedies then co-print for two independent causes, which is what P5 asks.
7b. **The row loop partitions the unmapped bucket**, or remedies 1 and 2 cannot be per-cause: `origin/main`
   appends `label or "<empty>"` to **one** list. `unrecognized_labels` keeps only *labelled-but-unknown*; a
   separate `labelless_rows` counts the rest.
7c. **J1b — no summary line asserts a classification the process did not make.** With the map absent the
   counter reads **`unclassified rows: <n>  (label map unavailable)`**, not `unmapped-label rows`.
7d. **The partition reaches the summary**: `unknown-label rows` / `label-less rows` replace the merged
   counter, which said "label" for rows that have none.
8. ⚠ **CRIT inherited — `K` asserts a spec count the process could not make.** Measured in every map-absent
   state: `unique specs (K): 1 (…) (-)` — K=1 against an **empty** list, which is exactly the disagreement
   the merged memo's item 8 declared impossible. `unique_specs` gains an `"unmapped:<label>"` key inside the
   row loop that item 7c declares never ran. **A-ii's rule**: when `map_missing`, `K` is not a number —
   the line reads **`unique specs (K): n/a (label map unavailable)`**. When the mapper ran, `K` and its
   printed list must agree, which requires `labelless_rows` to be routed into the display as `<label-less>`.
   ⚠ **And the instrumentation must reach the branch where the claim can be false** — the merged memo's
   `PROTO-DISPLAY` was emitted inside the mapper-ran branch only, so it was structurally blind to the seven
   states where item 8 still failed. → `rederive armmatrix`

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is re-tested inside
`verify_citation` on **every unique citation**, reporting one process-level fact as *n* per-citation
failures. After the hoist the exit code is unchanged and the diagnostic is one line. The guard inside
`verify_citation` becomes an **explicit raise**, not an `assert` — under `python3 -O` an assert is stripped
and a direct caller would get exactly the silent non-zero this change removes.

#### §4.2.4 The remedy text

**Four** strings, currently one.

| Condition | Remedy |
|---|---|
| genuinely unmapped label | "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the label spelling" |
| **label-less cell** | "the Spec section cell must open with a spec label" |
| map unavailable | the captured import error and the path attempted, plus `--no-verify` |
| CLI missing | the expected path, plus `--no-verify` |

⚠ Remedy 3 names "the import error", which `except Exception: _shortname_for = None` **discards** — A-ii
captures it (`_shortname_for_error`) alongside the sentinel. **Initialised before the `try`**, or it goes
stale: a module global assigned only in the `except` arm keeps its previous value when a later
`importlib.reload` **succeeds**. The symmetric-looking `_shortname_for = None` **is** re-established on
reload, which is what makes the asymmetry easy to miss. → `rederive reloadstale`

⚠ **And remedy 3 needs a degraded form.** §4.5 item 1 names an in-process `preflight._shortname_for = None`
as a precondition-pinning mechanism; that sets the sentinel *without raising*, so the captured error is
`None`. A-ii states the string — *"the spec-label map is unavailable (no import error was captured)"* — and
**P5c asserts the string, not the branch**.

#### §4.2.5 Let a slice declare that it has no spec surface

- **Accepted shape**: the `## §3. Spec coverage map` heading stays **required**; its body may carry one
  marker line in place of a table.
- **Recognition** — the three properties `find_coverage_map_section` and `find_table` already thread:
  **line-anchored**, **fence-aware** (`fence_state`-gated), **§3-scoped**. ⚠ The residual census
  (`rederive marker`) implements all three, not a bare grep — anything weaker makes the marker the silent
  bypass this section argues it is not.
- **Hard-fail on ambiguity**: marker **and** a table, with or without data rows; or the marker twice.
  ⚠ These are **one code path** — `find_table` returns non-`None` for a header-only table — so one
  diagnostic serves both fixtures.
- **The summary is reduced, not re-worded.** The path branches **before the data loop**, so the variables
  the normal summary prints have no value; the verdict is the heading line plus the `n/a` lines, full stop.
  `verify_header_columns` is likewise unreachable, stated rather than discovered.
- **One name for one datum**: `split decision: n/a (no spec surface declared)`. The merged memo used
  `breadth:` here and `split decision:` on the other path.
- **`--strict-breadth` becomes a no-op here**, correctly — a slice with no spec surface has no breadth to
  split on — and `SKILL.md`'s **Flags** bullet says so (§4.1).
- **The marker suppresses citation verification, not grep-pass.** A slice with no *spec* surface still has
  §4-§7 structural references, so grep-pass moves into a `grep_pass_stage(args, plan_path) -> bool` called
  from both paths. This is the one structural change §4.2.5 forces on `main`.
- **Capability interaction**: with the capability absent the verdict cannot hard-fail here, and the printed
  line **names the absent capability**, so a run that *could not have verified* is distinguishable from one
  that *had nothing to verify*.
- **Residual, stated rather than argued away**: unlike `--no-verify`, an *invoker* decision visible in the
  command, the marker lives in the *artifact*, so one author's edit suppresses verification for every later
  reviewer. Mitigations: the ambiguity hard-fail is mechanical; the census implements the same three
  recognition properties; the gate prints `n/a`, not `ok`; Axis 4 reads the memo regardless. §10-Q1 puts the
  residual to review.

### §4.3 Test siting

The 8 A-i tests already live in `test_spec_labels.py`. A-ii creates `test_preflight.py` and takes the
`preflight` half of `test_all_three_consumers_derive_from_specs` as **P1**.

**Constraints the plan states rather than discovers:**

1. **`_shortname_for` is bound at module import**, and `preflight.py` **re-inserts `.claude/tools` on every
   import**, so "remove it from `sys.path` and reload" re-establishes the capability the test is removing.
   Working mechanisms: a `sys.modules`/`__import__` hook plus `importlib.reload`, or a subprocess. An
   in-process `preflight._shortname_for = None` pins the precondition but leaves the `except Exception`
   guard **mutation-green**.
2. **P1 needs `_shortname_for` bound; the map-absent runs need it `None`** — mutually exclusive
   process-global state in one file. `tearDown` restores via `importlib.reload`; P1 asserts the bound state
   at `setUp` so a leak fails loudly. `unittest` orders methods alphabetically, so relying on names is not a
   plan.
3. **The isolation contract is five pieces of process state**: `preflight._shortname_for`,
   `_shortname_for_error`, `sys.path`, `preflight.WEBREF`, and `subprocess.run`.
4. **`verify_citation` is stubbed by a shared `setUp` for every pin that runs `main`**, or T-net is red by
   construction. Measured: a handful of `main` runs in default mode with resolvable rows reach
   `subprocess.run`; with the stub installed at module level the count is **0** while every observable
   assertion survives. `verify_citation` is the single seam between the gate and the CLI — preflight has
   exactly one `subprocess.run` call site — so the stub is complete by enumeration. **No pin loses
   coverage**: P6's "reported once" is about the *hoisted* verdict, which never enters the loop; the
   `python3 -O` explicit-raise guard is pinned by calling `verify_citation` directly with `WEBREF` pointed
   at a nonexistent path, which reaches no subprocess.

---

## §5 Behaviour deltas

**Both columns measured** — baseline by `rederive column` (which varies the CLI axis), *After A-ii* by
`rederive armmatrix` running the grafted control flow. **On `origin/main` the "map" axis does not exist**, so
those rows read `n/a`. The two capability causes are a **union**, so any combination yields one verdict;
what differs is the **diagnostic**.

| # | CLI | map | mode | §3 shape | `origin/main` | After A-ii |
|---|---|---|---|---|---|---|
| 1 | ✓ | ✓ | default | labelled | 0, verified | **0** |
| 2 | ✓ | ✓ | `--no-verify` | labelled | 0 | **0** |
| 2b | ✓ | ✓ | default | `dedup.md` | 0, **1** unique from 2 rows | **0**, unchanged |
| 3 | ✗ | ✓ | default | labelled | 1, one failure per citation | **1** — one diagnostic line |
| 4 | ✗ | ✓ | default | label-less | **0** | **1** |
| 5 | ✗ | ✓ | `--no-verify` | either | 0 | **0** — capability unused |
| 6 | ✓ | ✗ | default | labelled | n/a | **1** |
| 7 | ✓ | ✗ | default | label-less | n/a | **1** (§4.2.2) |
| 8 | ✓ | ✗ | `--no-verify` | either | n/a | **0** (J3) |
| 9 | ✗ | ✗ | default | any | n/a | **1**, diagnostic names **both** causes |
| 10 | ✓ | ✓ | default | **alias spelling** | 0, unmapped soft-warn, no verify line | **0**, mapped and verified |
| 11 | ✓ | ✓ | default | **all rows unmapped** | **0**, **no `citation verify:` line at all** | **0** + `n/a (0 of N rows resolvable)` |
| 11b | ✓ | ✓ | default | **label-less** | 0, no verify line | **0** + the same line, + remedy 2 |
| 12 | ✓ | ✓ | default | **marker, no table** | **1** (no-table hard-fail) | **0**, `n/a` |
| 12b | ✓ | ✓ | default | **marker + header-only table** | **1** (0-data-rows) | **1** — ambiguous declaration |
| 13 | ✓ | ✓ | default | **marker + populated table** | **0** (marker is inert prose) | **1** — *same branch as 12b* |
| 14 | ✓ | ✗ | default | **marker** | n/a | **0**, line names the absent capability |
| 15 | ✓ | ✓ | default | **marker inside a fence** | **0** (table verifies) | **0**, unchanged — fence rule |
| 16 | ✓ | ✓ | default | **one unmapped + one malformed** | 1 (malformed) | **1**, **and** `n/a (0 of 2 rows resolvable)` — item 5's denominator |

**Newly-red**: 4, 6, 7, 9, 13. **1 → 0**: row 12 only. **1 → 1, changed diagnostic**: 3, 12b, 16.
**Exit unchanged, output changed**: 10, 11, 11b, 14.

⚠ **The harness runs further untabulated states, and they are not all agreement.** The merged memo asserted
"none diverges between the candidate predicates" as evidence of row-set completeness; measured, several
untabulated states **do** separate the shipped predicate from the rejected one. The completeness claim is
therefore **not** made here: §5 tabulates the outcome-distinct rows, and `armmatrix` prints its own state
totals for anything else. → `rederive armmatrix`

---

## §6 Pins

Each pin names what it **executes**; §5 owns the expected values, stated once. "Fails at A-i's head?" is what
§12(2) reads — no second list.

**Two suite-level fixtures, stated here rather than inside a pin**, because a per-pin clause is what made the
merged memo's pin set unsatisfiable: a shared `setUp` stubs `preflight.verify_citation → (True, "")` for
every pin that runs `main`, and restores the five pieces of process state in `tearDown`. The capability axes
are flipped by §4.2.1's in-process instruments.

| Pin | What it executes | §5 rows | Fails at A-i's head? |
|---|---|---|---|
| **P1** | `shortname_from_label(label) == short` over `SPECS`; no `sys.path` mutation in the body; `setUp` asserts the module un-poisoned | — | no |
| **P1b** | `main` on `labelled.md`, default **and** `--no-verify` | 1, 2 | no |
| **P1c** | `main` on `dedup.md`; asserts `1 unique citation(s) checked` from 2 rows | 2b | no |
| **P2** | map unimportable via `importlib.reload` under an import hook | 6 | **yes** |
| **P2b** | the same via subprocess; **mutation check** — deleting the `except Exception` clause must turn P2b red while P2 alone stays green | 6 | **yes** |
| **P3** | `--no-verify --no-grep-pass`, map absent — exit 0 and the basis qualifier | 8 | **yes** |
| **P3b** | `--no-verify`, CLI absent, map present — exit 0, capability unused | 5 | no |
| **P4** | label-shape independence: `labelled.md` and `unlabelled.md` give the *same* exit code in every capability state | 4, 7, 11b | **yes** |
| **P5** | each of the four remedies appears for its own cause **and no other** — incl. the soft-warn suppressed when **the map** is absent (item 7) and remedy 1 vs 2 separated by item 7b | 3, 6, 7, 9, 11, 11b | **yes** |
| **P5b** | with the map absent the summary reads `unclassified rows`, not `unmapped-label rows` (item 7c) | 6, 7, 8, 9 | **yes** |
| **P5c** | remedy 3's **string**, in both the captured-error and the degraded (`None`) case | 6, 9 | **yes** |
| **P5d** | the basis qualifier names the classes it counted, and is absent when the mapper did not run (item 6) | 11, 11b, 16 | **yes** |
| **P5e** | `K` is `n/a (label map unavailable)` when `map_missing`, and agrees with its printed list otherwise (item 8) | 6, 9, 11, 11b | **yes** |
| **P6** | CLI missing reported once, not per citation; row 9's diagnostic names both causes | 3, 9 | **yes** |
| **P10** | `main` on `alias.md`; asserts the row is MAPPED and verified | 10 | no |
| **P11** | `nospec.md` → exit 0, asserting the `n/a` strings, not just the code | 12 | **yes** |
| **P11b** | `nospec-and-table.md` and `nospec-and-header.md` → exit 1 naming the ambiguity — **two fixtures, one branch** | 12b, 13 | **yes** |
| **P11c** | `nospec.md` with the map absent → exit 0, and the line names the absent capability | 14 | **yes** |
| **P11d** | `fenced-marker.md` → asserted on `find_markers(...) == []` **and** the absence of any `n/a (no spec surface…)` line — *not* on the exit code | 15 | **yes**, on those assertions |
| **P11e** | a no-spec-surface memo still runs grep-pass: `nospec.md` with a bad `crates/…` path → exit 1 **naming the grep-pass finding** | 12 | **yes**, on the diagnostic |
| **P13** | `allunmapped.md`, `unlabelled.md` and `malformed.md` → the `n/a (0 of N rows resolvable)` line present; **and its negative half** — absent in rows 3/6/9 | 11, 11b, 16, 3, 6, 9 | **yes** |
| **T-net** | across A-ii's whole suite, `subprocess.run` is never called with **the resolved `WEBREF` path** — the path object, *not* a `"webref"` substring, because `grep_pass` also calls `subprocess.run` with author symbols in argv | — | **yes** |

⚠ **An exit-code-only assertion is not a discriminator when the base reaches the same code by another
route.** It bites twice: at A-i's head `fenced-marker.md` exits 0 with the table verified (the marker is
inert prose there — identical to A-ii's expected outcome), and `nospec.md` exits 1 already via the no-table
hard fail. P11d and P11e therefore assert on the mechanism. → `rederive carvecolumn`

**UNCHECKED, marked not omitted**: the interpreter floor on `SKILL.md`'s direct `preflight.py` path (A-iii's);
that `shortname_for` and `origin/main`'s `shortname_from_label` are equivalent *functions* (`shortname_for`
calls `.strip()`; unreachable through the gate because `parse_spec_cell` already strips).

---

## §7 Layering check

**VM host/ / ECS-native** — not applicable; no `crates/**` diff.

**Generic core vs elidex adapter.** Every edit A-ii makes is in `.claude/skills/elidex-plan-review/`, the
adapter — `preflight.py`, `test_preflight.py`, `SKILL.md`. **A-ii touches the `_webref` generic core
nowhere**, which is a stronger statement than the merged memo could make and is a direct consequence of A-i
having taken the core half first. → `rederive couplings`

**One-issue-one-way**: the `WEBREF.is_file()` question collapses from *n*-per-citation to one verdict. The
one remaining instance of §1's class inside A-ii's file — `preflight` reaching `resolver.lookup_section`
through a subprocess while reaching `spec_labels` in-process — is §11's registered slot.

---

## §8 Line-count budget

→ `rederive budget`. `preflight.py` is the largest file in the touch set; A-ii's edit set is roughly
**statement-neutral** because the hoisted verdict deletes the per-citation `WEBREF.is_file()` re-test and
§4.2.5's branch replaces work rather than adding it. Nothing is near a split.

---

## §9 Edge-dense assessment

**(i) An approved umbrella's per-PR slice, explicitly.** The umbrella names A-ii and states its scope, and
was amended **before** this memo's plan-review.

**(ii) Scope narrowed to a single invariant-axis intersection.** J1/J1b/J2/J3 live in one function's control
flow with one primary observable (an exit code) and one secondary (the summary's lines); §5 publishes the
outcome-distinct rows with a pin apiece, and §2 states each pair's intersection. The gate-contract change
(§4.2.5) is additive — one input shape, five rows, five pins.

⚠ **The honest qualification.** This is the densest of the three A slices, and it inherits two CRITs. What
makes it terminal is not that the intersection is small but that its state space is **enumerable and run in
one command**, and that the two things that were sharing this memo's review surface — the map extraction and
the scheduler — are now elsewhere. If a round finds a *new* inversion in this control flow (not a
reporting-layer omission, which §4.2.3 items 6/8 now pin), the honest response is re-slicing, not a better
harness.

---

## §10 Open questions

Decided rather than listed: the `verify_citation` guard is an **explicit raise**; `shortname_from_label`
keeps returning `None`; the reporting arm's guard is the capability verdict; `K` is `n/a` when the map is
absent.

- **Q1 — the §4.2.5 residual.** The marker is artifact-resident. Four mitigations, one mechanical. The
  alternative is to require the marker to name its umbrella slice — checkable, but a coupling A-ii has no
  other reason for. Put to review rather than closed.

---

## §11 Defer slots + per-PR ≤3 audit

**One own deferral.**

| Slot | `#11-webref-preflight-inprocess-resolution` |
|---|---|
| **Why deferred** | the collapse is small, but it decides the offline contract for the resolver, which is Slice B's. Folding it in would settle B's policy by side effect — the failure §4.2.3 exists to stop |
| **Re-evaluation trigger** | Slice B landing the catalog fall-through |
| **Re-evaluation date** | 2026-10-31 |
| **Confidence** | High — the consumer is named and the trigger is a slice already planned |

⚠ It is an **own** deferral, not a pre-existing one: `origin/main`'s `preflight.py` has **no `_webref`
import**, so the in-process reach is created by A-i and inherited here. The umbrella carries a
forward-binding constraint as the **pointer**; this table is the record.

**Pre-existing, not counted**: `#11-elidex-ci-required-status-checks` — the ruleset has no
`required_status_checks` rule, so every CI job is advisory, and a bypass actor makes the rule alone
author-bypassable. Registered by A-iii, which is the slice that adds a job.

**Explicitly NOT deferred**: the two-cause verdict and both act-sites, all four remedy strings and the
degraded form, the soft-warn suppression, the partition and its summary consumers, both inherited CRITs, the
no-spec-surface verdict and its recognition rule, the verify-line silence, the five-piece isolation contract,
and `SKILL.md`'s contract.

---

## §12 Exit criterion

**(1) Green:** `test_preflight.py` passes; `git diff -- crates/` empty; `git diff -- .claude/tools/_webref/`
**empty** (A-ii touches no generic-core file).

**(2) Red at A-i's head:** copy `test_preflight.py` onto A-i's landed head and run it. Non-zero, with at
least one failure attributable to **every pin whose §6 row says "yes"**. No second list: §6's column is the
criterion.

**(3) The two inherited CRITs are pinned, not just fixed:** P5d and P5e are red at A-i's head **and** red
against a build where only the *other* one is fixed — the instrumentation must reach the branch where each
claim can be false, which is what the merged memo's version did not do.

**(4) J3 survives:** `--no-verify --no-grep-pass` exits 0 with the tools tree absent.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-i** | A-ii branches from its landed head and replaces the import guard | **A-i first** |
| **A-iii** | none — A-ii schedules nothing | after A-ii |
| **Slice B** | B collapses §11's slot in the same slice it lands the fall-through | after A-ii |
| **Slice C** | inherits the no-spec-surface declaration as its first real consumer, plus `axes.md`'s Axis 4 detect and `grep_pass.py`'s per-path finding | after B |
| **PR #496 / #497 (Layout lane)** | none — no `ci.yml`, no `mise.toml` | none |

⚠ **A-ii changes `preflight.py`'s failure semantics, and every lane runs that gate.** The landing checklist
re-runs preflight from each worktree that authors a plan-memo and records the result; the worktree set is
**derived, not listed** → `rederive lanes`.

**Landing checklist**

1. Re-run preflight from each plan-memo-authoring worktree → `rederive lanes` derives the set.
2. Register `#11-webref-preflight-inprocess-resolution` in `project_open-defer-slots.md`. Measured, it
   exists in no ledger today, so no sentence may describe it as already recorded.
3. Update `project_citation-hygiene-program.md` with A-ii's outcome.
4. PR description: §4.2.1's instrument table, §4.2.5's contract change, and the two inherited CRITs.

---

## §14 Provenance

A-ii is carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (drafts 1–9, nine
`/elidex-plan-review` rounds). Corrections that originated there and are carried here as **settled**: the
capability-instrument table (R7/R8), the reporting arm's guard and the rejected `verify_ran` flag (R7/R8),
the `map_missing` re-key (R9), item 7b's partition (R8), the marker's three recognition properties and the
grep-pass stage (R8), the `_shortname_for_error` reload asymmetry (R8), P11d/P11e asserting on mechanism
(R8), and the suite-level stub (R7 H2).

Carried here as **open defects**, not history: §4.2.3 items 6 and 8, both round-9 CRITs, both of the form
*a summary line asserting what the process could not establish*.

---

## §15 Re-derivation

`docs/plans/2026-07-citation-hygiene-A-rederive.sh`. Blocks A-ii cites: `citations column carvecolumn
instruments remedies reloadstale armmatrix budget couplings marker lanes`. ⚠ `lanes` and `staleclaims` are
author-local and excluded from `all`.
