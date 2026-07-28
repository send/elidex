# Plan — Slice A: the shared spec-label map, landed fail-closed, with a scheduler that runs its suites

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A**. Under that umbrella's approval
boundary this is a **terminal unit** — it is not re-split for touching the same subsystem as B/C.
**Branch**: `webref-cite-audit-tool`, **after the §4.0 re-carve**. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`.
**Base**: `96a8e47b`.
**Nature**: developer tooling + **CI topology**. Zero `crates/**` diff, zero engine behavior change.
**Status**: plan-memo, **draft 3**. `/elidex-plan-review` **required before implementation**.

⚠ **Two review rounds, both of which found a CRIT this memo had introduced.** Round 1 showed draft 1
measured its entire evidence base **on the branch instead of on `origin/main`** — so the defect it existed
to fix was one the carve *introduces*, and the PR it described would have carried Slice B. Round 2 showed
draft 2's own fix opened a **third** capability failure (`SystemExit` escaping `_catalog()`) and that the
row-loop skip it proposed stranded four accumulators. §4.0 is the re-carve; §4.2.3 is the redesign that
subsumes round 2's findings; §14 tabulates both rounds, so the corrections are auditable rather than
silent.

### §0.1 What Slice A is, in one sentence

`preflight.py` on `origin/main` carries its own hand-maintained `SPEC_LABEL_REVERSE` map — the third
copy of one enumeration (`coverage_map._SPEC_LABEL_MAP` and `cli.COMMON_SHORTNAMES` are the other two;
the fourth, `cite_audit`'s, arrives only with Slice B). Slice A replaces the three copies with one shared
`spec_labels.py`, **and,
because that import is what makes the gate's verification capability failable, lands it fail-closed from
the start** — then gives the resulting suites a scheduler, because today nothing runs them. It ships no
detector (Slice B) and edits no review policy (Slice C).

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
would pass vacuously. Two fixture memos ship — `labelled.md` (the two rows above) and `unlabelled.md`.
Neither row is a citation defect; the label-less shape is the input that falsifies §4.2.2's placement.

⚠ **This table certifies fixtures, not the slice.** `preflight` prints `citation verify: ok (2 unique
citation(s) checked)` for a slice with **zero spec surface**. Derivation (`origin/main` line numbers, since
this is pre-existing): the no-§3-heading hard-fail is `:306-314`, the no-table hard-fail `:319`, the
**0-data-rows hard-fail `:334`** — the last is the one that forces a zero-spec slice to author fixture
citations. So there is no accepted input shape that declares "no spec surface" and passes.

That is §1's anchor violated in A's own file, and A owns the fix: expressing "no spec surface" needs an
accepted input shape or an opt-out in `preflight.main` — **not** a result type from `spec_labels._catalog()`,
and not `axes.md`'s authoring contract. Draft 2 routed it to Slice B on both counts and was wrong on both:
capability-vs-artifact is the J1 distinction A itself insists on, and the umbrella forbids B from editing
review policy. It is **out of A's scope by size, not by owner**, and is registered as such in §11 with a
trigger and a date rather than "routed" to nobody.

---

## §1 Ideal anchor — a gate reports on the thing it audited, or it reports on itself

Two failures, one shape. A gate's output is a claim about the artifact under review. When the gate's own
infrastructure is missing, the honest output is a claim about the **gate**, not a verdict on the artifact.

1. **The carve as authored introduces exactly that inversion.** Replacing `SPEC_LABEL_REVERSE` with an
   `import` makes the label map *failable* for the first time, and the carve's guard
   (`except Exception: _shortname_for = None`) routes that failure into the per-row *unmapped* bucket — a
   documented soft-warn. Result: 21 of 21 rows classified as *author cited a spec I do not know*, and the
   gate **exits 0 having verified nothing** (§4.2.1, measured). The tool blames the memo for a fact about
   the tool. A must not land that.
2. **Nothing runs the suites.** On `origin/main` there are **47 tests across 4 files** under no `mise`
   task, no CI job, no hook (verified 2026-07-28; derivation in §4.3.1). An unscheduled suite is a claim with no checker — the
   shape this program exists to remove.

The corollary that drives the edit set, and the one draft 1 got wrong twice:
**a capability is a process-level fact and must be established once, before the data loop.**
"I cannot map *this* label" is a datum about one row. "I cannot map *any* label" is a fact about this
process. Discovering the second by watching the first makes the failure look like data — and, as §4.2.2
measures, makes the fix's correctness depend on the *content* of the memo being reviewed.

---

## §2 Coupled invariants

- **J1 — capability ≠ datum.** A row is *unmapped* only if the mapper ran and declined. If the mapper is
  absent, no row is unmapped; the run is uncertified. One return value (`None`) must not carry both.
- **J2 — the two capabilities must degrade the same direction.** Verifying a citation needs the `webref`
  CLI *and* the label map. Measured on the carve, one hard-fails and the other exits 0 (§4.2.1). The
  carve's in-code comment claims they "degrade the same way". They do not.
- **J3 — one degradation must survive.** `--no-verify --no-grep-pass` (structure + breadth only) must keep
  working with the tools tree absent. It is the property a fail-closed change is most likely to break,
  and the one draft 1 broke (§14 C2).
- **J4 — one enforcement mechanism, not two.** If `mise` and `ci.yml` each spell the suite invocation, a
  later suite is added to one and not the other. `trip-wires` already answers this: the script is the SoT
  and each runner is a caller.

J1–J3 all live in `preflight.main`'s control flow and cannot be applied one at a time without transiently
breaking each other — which is why §5 measures the **full** configuration matrix rather than a sample.
J4 is independent, and is why §4.3 ships a script rather than two copies of two lines.

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
data. Two clarifications draft 1 elided:

- **The inputs are the plan-memo's path *and its content*.** `parse_spec_cell` (`preflight.py:216-229`)
  extracts a label and a section number from memo cell text, and `verify_citation` (`:240-263`) passes
  **both** to a subprocess. §4.2.2's finding is that memo content steers control flow, so listing only the
  path omits the input that section proves is load-bearing.
- **Both argv elements are bounded, and one bound changes under A.** `section` is bounded by
  `SECTION_REF_RE = r"§\s*([\d.A-Z]+)"` (`:77`) — unchanged. `shortname` on `origin/main` is bounded by the
  module-local `SPEC_LABEL_REVERSE`; **after A it is bounded by the 948-entry upstream catalog**, because
  `shortname_for` falls through to `_catalog()`. Measured 2026-07-28 (`python3 -c "import sys;sys.path.insert(0,'.claude/tools');from _webref import spec_labels as s;print(len(s.SPECS), len(s.LABEL_TO_SHORTNAME), len(s._catalog()))"`): `SPECS` = 12 entries, `LABEL_TO_SHORTNAME` = 24
  keys, catalog = 948, and `shortname_for("CSS Text 3")` → `css-text-3`, which is *not* in `SPECS`. Draft 2
  derived A's safety from the bound A removes. The post-A bound is still a closed set of upstream shortnames
  passed as a list element, so the conclusion holds — but on the correct premise.
- **The exposure delta runs both ways, and the inbound half is the larger one.** Outbound: A moves suites
  that fetch from `raw.githubusercontent.com` from manual invocation to every `.claude/**` PR (§4.3.3).
  **Inbound**: after A, *which labels the gate recognises* is decided by a third-party document fetched at
  gate time, on every plan-review run in every lane (`SKILL.md:110` invokes `preflight.py` directly). On
  `origin/main` that path has no network at all. §4.2.3 is what keeps this from being a silent trust
  transfer: an unreachable catalog becomes an explicit capability verdict, not a soft-warn.

**Discovery method.** Every defect and number below was produced by **executing** the code on
2026-07-28, and — after draft 1's failure — **against the correct baseline**, stated per measurement:

1. `origin/main` facts come from `git show origin/main:<path>` or a throwaway `git worktree add
   origin/main`, never from the branch (§14 C1 is what happens otherwise).
2. The gate asymmetry is a three-case sandbox run (§4.2.1), one dependency removed per case.
3. The re-siting defect (§4.2.2) was found by **applying draft 1's own fix in that sandbox** and running
   it against two memos — a measurement of a proposed patch, not a reading of one.
4. Branch/lane facts use **three-dot** ranges (`origin/main...<branch>`); draft 1's two-dot form reported
   `main`'s own commits as a branch's and fabricated one overlap (§14 C4).

---

## §4 The edit set

### §4.0 Step 0 — re-carve `26721cfa` on the seam the umbrella already draws

Measured (identical under `..` and `...`, so the branch is not behind):

```sh
git diff --numstat origin/main...HEAD -- .claude/
```

| File | +/− | Half |
|---|---|---|
| `_webref/spec_labels.py` | 136/0 | **A** — the shared map itself |
| `skills/elidex-plan-review/preflight.py` | 20/30 | **A** — drops local `SPEC_LABEL_REVERSE`, imports the map |
| `_webref/commands/coverage_map.py` | 15/21 | **A** — second consumer |
| `_webref/cli.py` | 43/13 | **split** — the `SHORTNAME_TO_BLURB` blurb derivation is A; the `cite-audit` subparser + its import + one example line are B |
| `_webref/DESIGN.md` | 23/0 | **split** — the `spec_labels.py` bullet is A; the `cite_audit.py` adapter bullet, the CLI examples and the three-bucket paragraph are B |
| `_webref/test_cite_audit.py` | 410/0 | **split** — `TestSharedSpecLabelMap` + `coverage_map_label` (`:197-317`, **121 lines**, and it subclasses `unittest.TestCase` directly rather than the detector's `_TreeCase`) become A's `test_spec_labels.py`; the remaining 8 classes stay B's |
| `_webref/commands/cite_audit.py` | 289/0 | **B** — the detector |
| `_webref/sources/webref_data.py` | 9/0 | **B** — `@lru_cache` motivated by the detector's per-section loop; B's §4.1.6 rewrites this area |

The dependency is one-directional — `cite_audit` imports `spec_labels`, never the reverse — which is why
the seam is real and not administrative. `TestSharedSpecLabelMap` subclassing `unittest.TestCase` is the
same fact expressed in the test tree.

⚠ **The split is at hunk granularity; the *prose* needs its own pass.** Six sites in the A column describe
`commands/cite_audit.py` as extant — `spec_labels.py:3-6` and `:66-68`, the `DESIGN.md` bullet,
`preflight.py:48-51`'s new comment, and the moved test's docstrings at `test_cite_audit.py:198-204`/`:245`.
`cite_audit.py` is **absent from `origin/main`**, and only three copies of the label map exist there
(`coverage_map._SPEC_LABEL_MAP`, `cli.COMMON_SHORTNAMES`, `preflight.SPEC_LABEL_REVERSE`) — so §0.1's
"fourth copy … the four copies" is branch-relative too, and is corrected to **three** below. A filename-only
check passes while every one of these is present, which is why §12 (3) gains a content assertion.

Result: `webref-cite-audit-tool` = `origin/main` + the A column + A's edits; a new branch for B = A's
landed head + the B column. **B's memo already assumes this** ("Branch: new, cut from Slice A's landed
head").

**Why A takes the label-map half rather than leaving the whole carve to B**: the map is what *creates*
the failable capability. If B lands it, `main` carries a fail-open plan-review gate — a gate every lane
runs — for the duration of B. A landing it fail-closed means the defect is **never introduced**, which is
strictly better than introducing and repairing it, and is why §4.2 below is framed as "do not land the
carve's guard as authored" rather than "fix a bug".

### §4.1 What A deliberately does not touch

| Concern | Slice | Why not A |
|---|---|---|
| `cite_audit.py`, `test_cite_audit.py`, the `cite-audit` subparser, `webref_data.py`'s memo | **B** | the detector; §4.0 routes them |
| `spec_labels`'s reverse index and the catalog round-trip rules (B §4.1.2 / §4.1.8) | **B** | A lands the map's *shape*; B fixes its lookup semantics |
| the **discriminated `_catalog()` result** (was B §4.1.7) | **A**, B consumes | moved to A at draft 3: A is the first consumer that can be killed by an unreachable catalog (§4.2.3), and the slice that introduces a failure owns it. B's §4.1.7 is rewritten to consume the type rather than introduce it |
| `.claude/skills/elidex-plan-review/SKILL.md` — the `Hard-fail conditions` list (`:113-114`) and `--no-verify`'s documented meaning (`:116`) | **A** | A adds a hard-fail cause and gives `--no-verify` a second role (capability suppressor). No other slice claims this file, and leaving the gate's contract of record describing the old behaviour is the documentation half of §1's anchor |
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
| **C** | `.claude/tools/_webref` (the carve's new import) | `parsed citations: 0`, `unmapped-label rows: 21`, **no verify section at all** | **0** |

`15`, not `21`: `seen_pairs` (`:382-388`) dedups 21 data rows to 15 unique `(shortname, section)` pairs.

Case C also emits a **wrong-cause remedy**: `(add the spec to .claude/tools/_webref/spec_labels.py::SPECS)`
— the file that failed to import. An author following it edits a file the gate cannot read.

**Case C does not exist on `origin/main`**: there, `shortname_from_label` (`preflight.py:242-244`) reads a
module-local `SPEC_LABEL_REVERSE` dict with no import to fail. The asymmetry is a property of the carve,
which is why A owns it.

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

#### §4.2.3 The fix — **three** causes, an aggregated verdict, and a loop that is never skipped

⚠ Draft 2 enumerated **two** causes and proposed skipping the row loop. Round 2 falsified both halves.

**There is a third cause, and A creates it.** A's new import path is
`shortname_from_label` → `spec_labels.shortname_for` → `_catalog()` → `sources/webref_data._data_index()`
→ `cache.py:131 sys.exit`. `SystemExit` is a `BaseException`, so `_catalog()`'s `except Exception`
(`spec_labels.py:91`) does not catch it — and its docstring's promise, *"an offline run degrades to the
pinned set rather than dying"*, is false. Reproduced:

```sh
python3 - <<'EOF'
import sys, urllib.request, urllib.error
sys.path.insert(0, ".claude/tools")
urllib.request.urlopen = lambda *a, **k: (_ for _ in ()).throw(urllib.error.URLError("offline"))
from _webref import spec_labels
try: print("returned:", spec_labels.shortname_for("CSS Text 3"))
except SystemExit as e: print("SystemExit ESCAPED _catalog():", e)
EOF
```
→ `SystemExit ESCAPED _catalog(): webref: network error fetching …/ed/index.json: offline`

Trigger: offline **and** a §3 row whose label is outside the 24 pinned keys — the catalog has **948**
entries, `CSS Text 3` is the umbrella's own example, and the umbrella records 10 in-flight memos in
`elidex-wt-c3-plan` citing exactly such labels. The result is a bare exit from *inside* the row loop: no
diagnostic, no `--no-verify` escape (it fires before the verify stage). `origin/main` exits 0 on the same
input. That is **J3 broken by the slice written to protect it**, and it is §1's anchor inverted — a fact
about the network killing a run that was auditing a memo.

**And the skip strands the accumulators.** Both writers of `unique_specs` (`:357`, `:361`) are downstream
of the mapping call at `:353`, and `K = len(unique_specs)` (`:368`) drives the split decision (`:415-428`),
which `SKILL.md:118` makes a stop-and-ask-user workflow gate. Measured with the skip applied, on a
7-spec / 7-row fixture: `K` 7 → **0**, `split decision` `⚠ SPLIT-DEFAULT` → **`ok (single PR scope)`**,
`--strict-breadth` exit 1 → **0**. `specs_seen` (`parsed citations`) strands identically. Under §4.6's
wider spelling of the same change, `malformed_rows` (`:343`/`:349`, upstream of `:353`) strands too, which
kills a documented structural hard-fail. So the skip silently disables a *different* gate than the one
being fixed — the exact failure class §1 names, one level up.

**The design that removes both.** Capability is still established as a fact, not inferred from a datum
(J1); what changes is that one of the three causes can only *materialise* during lookup, so the verdict is
**aggregated** rather than pre-computed, and the loop keeps running.

1. **`_catalog()` returns a discriminated result** — *available(entries)* vs *unavailable(cause)* —
   catching `SystemExit` **alongside** `Exception`. This makes the function honour the contract its own
   docstring already advertises.
2. **`spec_labels.resolve_label(label)` returns `MAPPED(shortname)` / `UNKNOWN` / `UNCERTAIN(cause)`**,
   where `UNCERTAIN` = pinned miss **and** catalog unavailable. `shortname_for` becomes a thin view over
   it (`resolve_label(...).shortname_or_none`) so there is one implementation and two call shapes, not two
   resolvers.
3. **The row loop keeps its shape** and gains a third arm. All three arms write `unique_specs` with a
   distinct per-label key, so `K` counts author intent in every configuration; `specs_seen` /
   `parsed_count` move only for genuinely mapped rows; `malformed_rows` is untouched because it is
   upstream and stays upstream.
4. **The capability verdict is the union** of the two static causes (`WEBREF.is_file()`,
   `_shortname_for is None` — both process facts, evaluated once before the loop) and the dynamic one
   (any `UNCERTAIN` row). Unavailable **and** verification requested → HARD FAIL in the same
   `❌ HARD FAIL — …` shape as the other three, naming the cause and `--no-verify` as the suppressor.
   Unavailable **and** `--no-verify` → exit 0 (J3), summary reports *uncertified* rows distinctly from
   *unmapped* ones.
5. `shortname_from_label` keeps exactly one job — classify a label — and its `_shortname_for is None`
   branch is replaced by the `UNCERTAIN` arm rather than deleted, so no second site answers the capability
   question and no path calls `None(label)`.

**Boundary against Slice B.** A lands the `_catalog()` discrimination **because A is the first consumer
that can be killed by its absence** — the slice that introduces a failure owns it. B's §4.1.7 then
*consumes* the same result type for the detector's `UNKNOWN-SPEC` bucket instead of introducing it; B's
memo is updated in the same commit so the ownership is stated once.

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is currently re-tested
inside `verify_citation` on **every unique citation** — 15 times in case A — reporting one process-level
fact as 15 per-citation failures. After the hoist, case B's exit code is unchanged (1) and its diagnostic
is one line naming the missing path. The guard inside `verify_citation` becomes an **explicit raise**, not
an `assert`: under `python3 -O` an assert is stripped and a direct caller would get exactly the silent
non-zero this change exists to remove (draft 2 proposed `assert` and routed the objection to review —
§14 D6).

#### §4.2.4 The remedy text

Four strings, currently one, because there are four ways to fail and the author's next action differs in
each:

| Condition | Remedy |
|---|---|
| genuinely unmapped label | "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the label spelling" |
| **label-less cell** (`\| §4.10.21 … \|`) | "the Spec section cell must open with a spec label" — measured: today this row prints the `SPECS` advice against `<empty>`, advice that cannot be acted on, which is the same wrong-cause class as case C |
| tools unavailable (import failed) | the import error and the path attempted, plus `--no-verify` |
| CLI missing | the expected path, plus `--no-verify` |
| catalog unreachable (`UNCERTAIN`) | the fetch URL and cause, plus `--no-verify`; **not** the `SPECS` advice |

### §4.3 A2 — give the suites a scheduler

#### §4.3.1 The hole, measured on `origin/main` across all three workflows

- `ci.yml`'s `changes` filter has two sets: `rust` (`crates/**`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `.rustfmt.toml`, `clippy.toml`, `mise.toml`, `.github/workflows/**`) and `config`
  (`deny.toml`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/**`). **`.claude/**` is in neither**, and
  all three jobs (`check`, `doc`, `deny`) are gated on one of the two.
- `ci.yml` **never invokes `mise`** — the single `mise` string in the file is `mise.toml` as a filter entry.
- `codeql.yml` analyses `[actions, rust]` on push-to-main + a weekly cron — **no Python, no
  `pull_request` trigger**. `audit.yml` is `cargo audit` on a weekly cron.

⇒ a `.claude/**`-only pull request triggers **zero jobs**, and even the post-merge push runs only cargo.
The suites, **as they exist on `origin/main`** (re-derived in a throwaway `git worktree add origin/main`,
2026-07-28):

| Suite | Tests |
|---|---|
| `.claude/tools/_webref/test_inventory_diff.py` | 6 |
| `.claude/tools/_webref/test_agent_brief.py` | 5 |
| `.claude/tools/_webref/test_refresh.py` | 1 |
| `.claude/skills/elidex-plan-review/test_grep_pass.py` | 35 |
| **total on `origin/main`** | **47, across 4 files** |

```sh
# verified 2026-07-28 — measured in a throwaway worktree so the branch cannot leak in
T=$(mktemp -d); git worktree add -q "$T" origin/main
ls "$T"/.claude/tools/_webref/test_*.py "$T"/.claude/skills/elidex-plan-review/test_*.py | wc -l   # → 4
for f in "$T"/.claude/tools/_webref/test_*.py; do python3 -m unittest discover \
  -s "$T/.claude/tools/_webref" -p "$(basename $f)" -t "$T/.claude/tools" 2>&1 | grep -E '^Ran '; done
python3 "$T/.claude/skills/elidex-plan-review/test_grep_pass.py" 2>&1 | grep -E '^Ran '
git worktree remove --force "$T"
```

A adds `test_spec_labels.py` (9 tests moved by §4.0) and `test_preflight.py` (§6), so A's own landed
figure is ~47 + 9 + P-count. **Draft 1 reported 83 across 5 files** — the *branch* figure (verified 2026-07-28 by running the same
commands without the `origin/main` worktree), which counts the 36 detector tests that are Slice B's
(§14 C1).

#### §4.3.2 The mechanism — one script, two callers (J4)

`.claude/tools/python-suites.sh`, `set -euo pipefail`, `cd "$(dirname "$0")/../.."`, then the two
`discover` lines (both verified to collect their full sets):

```sh
python3 -m unittest discover -s .claude/tools/_webref -p 'test_*.py' -t .claude/tools
python3 -m unittest discover -s .claude/skills/elidex-plan-review -p 'test_*.py'
```

- `mise.toml` gains `[tasks.tools-test]` = `bash .claude/tools/python-suites.sh`, added to
  `[tasks.ci].depends`.
- `ci.yml` gains a `tools` path-filter set (`.claude/tools/**`, `.claude/skills/**`,
  `.github/workflows/**`) and a `tools` job on `ubuntu-latest` running the same script under the same
  `|| github.event_name == 'push'` bypass the other three jobs use.

This is the `trip-wires` shape verbatim (`mise run trip-wires` calls four `.claude/tools/*.sh`), so it
introduces no new pattern — and it is why the local gate and the merge gate cannot drift into two
spellings.

#### §4.3.3 The merge gate takes a live-network dependency — measured, and smaller than draft 1 thought

Measured with a spy on `urllib.request.urlopen`:

- **A's own suite set** — `TestSharedSpecLabelMap`, the 9 tests §4.0 moves into `test_spec_labels.py` —
  fetches exactly **1** URL: `https://raw.githubusercontent.com/w3c/webref/main/ed/index.json`
  (1,572,569 B). The second URL (`ed/headings/html.json`, 293,409 B) belongs to the **detector's** tests
  and leaves with Slice B.
- The three `origin/main` suites and `test_grep_pass.py` fetch **nothing**.
- Re-running any of them in the same process with `urlopen` raising `URLError`: **0 failures** — the
  dependency is that one resource, nothing more.
- With the network blocked **from the start**, the fetching tests fail *even with a warm 101 MB
  `~/.cache/elidex-webref`*, because `cached_fetch_url` (`cache.py:64-85`) always issues a conditional GET
  and `cache.py:130-131` `sys.exit`s on `URLError`. There is no offline mode;
  `ELIDEX_WEBREF_NO_CACHE=1` makes it *more* networked.

**Disposition — accept for A, and give the offline question a real owner.** (a) A's marginal dependency is
one conditional GET to `raw.githubusercontent.com`, the same provider the job's own `actions/checkout`
depends on. (b) `mise run ci` **already** requires the network — `deny` runs `cargo deny check`, which
maintains a fetched advisory database (`~/.cargo/advisory-dbs/` present locally), so `tools-test` adds no
new *class* of requirement to the mandatory local gate. (c) The fix is an offline mode in `cache.py` /
`spec_labels.py`.

⚠ Draft 1 routed (c) to "Slice B's §4.1.7" and stopped. That is wrong: B's §4.1.7 makes the catalog-
unavailable case **more** fatal (*"not survivable without `--no-verify`"*), and neither B's memo nor the
umbrella carries an offline obligation. **A's landing edit therefore adds one line to the umbrella's
"Constraints each slice inherits"**: *the suites must be runnable with the network down by the end of
Slice B; B's `_catalog()` availability contract is where it lands.* An unowned concern is not a
disposition (§14 C5).

`actions/cache` for `~/.cache/elidex-webref` was considered and **rejected on the measurement**: the
conditional GET fires whether or not the body is cached, so a restored cache saves transfer and **zero**
requests.

#### §4.3.4 What "enforced" can honestly mean here

```sh
gh api repos/send/elidex/rulesets --jq '.[] | {name, enforcement, target}'
gh api repos/send/elidex/rulesets/13294991 --jq '{rules: [.rules[].type]}'
```

`main` is governed by an **active** repository ruleset `main-protection` (id 13294991, target
`~DEFAULT_BRANCH`) whose rules are `deletion` / `non_fast_forward` / `pull_request`. There is **no
`required_status_checks` rule**, so a red `tools` job does not block a merge; CLAUDE.md's workflow
("CI 全 pass を目視確認してから squash merge") is the blocking step, and it is a human one.

The claim A may make is therefore: the job makes a regression **visible, attributed, and on the PR page at
review time**, where today it is invisible in every event. That is a strict improvement and is what §12
asserts — no more.

⚠ Draft 1 asserted "**no branch protection and no required status checks**" from
`gh api …/branches/main/protection` → 404. That is the **deprecated legacy endpoint**; 404 there means
"not protected *via the legacy API*", not "unprotected" (§14 C3). The corrected picture also changes the
follow-up's cost: adding `required_status_checks` is one rule on an existing active ruleset, not a
from-scratch settings change — which is why §11 registers it rather than waving at it.

#### §4.3.5 The interpreter floor

Measured: **no `.claude` Python source uses syntax newer than 3.9** (`match`, `except*`, `tomllib`,
`typing.Self`, `ExceptionGroup`, atomic groups — all absent). Local dev is 3.14.6. Nothing in the
repository declares a floor.

`python-suites.sh` asserts `sys.version_info >= (3, 9)` — **A's own measured need** — and the job echoes
`python3 -VV`, so the runner's actual version becomes a measured fact on the first CI run instead of an
assertion here.

⚠ Draft 1 declared **3.11** on the stated ground that *"it is the floor Slice B's atomic-group grammar
will need, so declaring it here means B does not reopen `ci.yml`"* — a slice carrying another slice's
concern, against the umbrella constraint A's own §4.2.4 obeys two sections earlier (§14 C6). B raises the
floor when B lands `(?>...)`; that is one line in a file B is already editing.

The floor is asserted in the script, which is the entry point **both** runners use — but note
`elidex-plan-review/SKILL.md:110` invokes `preflight.py` directly, bypassing it. That path is unaffected
today (A adds no version-dependent syntax) and is recorded in §4.6's claims table as **UNCHECKED**.

### §4.4 A3 — site the label-map tests where they belong, from the start

§4.0 moves `TestSharedSpecLabelMap` (`test_cite_audit.py:197-317`) into A's `test_spec_labels.py`. One
assertion inside it does not belong there either:

```python
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "skills" / "elidex-plan-review"))  # :289-292
preflight = importlib.import_module("preflight")                                                # :296
```

The *generic tools* package's test hard-codes the *elidex skill's* directory layout and module name — the
one **import-time executable** edge that blocks `DESIGN.md`'s goal of keeping the drift-detection core
movable to a standalone repository. (It is not the only elidex reference in that tree —
`cli.py:77` bakes a skill path into `argparse` help text, and `webref:5` / `spec_labels.py:7` name skill
paths in prose. All pre-existing, none A's; recorded so "the one edge" is not overstated.)

**Fix**: the `preflight` half of the assertion goes to `.claude/skills/elidex-plan-review/test_preflight.py`,
beside `preflight.py` and `test_grep_pass.py` — the home exists and the dependency direction is right
(consumer depends on library). `test_spec_labels.py` keeps the `coverage_map` half with a module-top-level
import; `coverage_map_label` (`:313-317`, one caller at `:308`, verified by
`grep -rn 'coverage_map_label' .claude/`) collapses into it. No `sys.path` mutation survives inside any
test method.

Because §4.0 makes this a *split of an unlanded file* rather than an edit to a landed one, the
`TestSharedSpecLabelMap` docstrings that describe the three-consumer guard travel with the assertion
instead of being left behind in the detector's file.

### §4.5 Test-siting constraints the plan must state, not discover

Two facts about `test_preflight.py` that only surface on execution:

1. **`_shortname_for` is bound at module import**, so "make the import fail" cannot be done by removing
   `.claude/tools` from `sys.path` and reloading — `preflight.py:56` **re-inserts that directory on every
   import**, so the module under test re-establishes the capability the test is removing. Working
   mechanisms are a `sys.modules`/`__import__` hook plus `importlib.reload`, or a subprocess; they pin
   different lines. An in-process `preflight._shortname_for = None` pins the new precondition but leaves
   `:56-60`'s `except Exception` **mutation-green**. **P2 uses the reload form** and P2b adds a
   subprocess case, so both the guard and the precondition are pinned.
2. **P1 needs `_shortname_for` bound; P2/P3/P4 need it `None`** — mutually exclusive process-global state
   in one file, and reloading does not restore it. `test_preflight.py` therefore restores the module in
   `tearDown` via `importlib.reload` under the un-patched import, and P1 asserts the bound state at
   `setUp` so a leak fails loudly instead of silently inverting. `unittest` orders methods
   alphabetically, so relying on names is not a plan.

### §4.6 Claims vs checks

Per the umbrella's constraint, **UNCHECKED rows are marked, not omitted**.

| Claim | What mechanically checks it |
|---|---|
| Tools-unavailable hard-fails when verification is requested | `test_preflight.py` P2 (reload form) + P2b (subprocess form) |
| …including for a memo whose §3 rows carry no spec label | P4 — the §4.2.2 regression |
| `--no-verify --no-grep-pass` still exits 0 with the tools tree absent | P3 (J3) |
| The row loop is skipped, not crashed, when the capability is absent | P3 asserts exit 0 **and** that the summary says *uncertified*, not *unmapped* |
| The missing CLI is reported once, not per citation, and still exits 1 | P6 |
| The remedy text names the right cause | P5 |
| Consumers derive from `SPECS` | `test_preflight.py` P1 + `test_spec_labels.py`'s `coverage_map` half |
| The suites run at all | `mise run tools-test`; the GitHub `tools` job |
| The interpreter floor holds on the runner | `python-suites.sh`'s assert — **but only on the script path**; `SKILL.md:110`'s direct `preflight.py` invocation is **UNCHECKED** |
| A carries no part of B | §12 (3), ranged against the re-carve — **UNCHECKED until §4.0 is performed**; it fails at today's head by construction |
| A red `tools` job prevents a merge | **UNCHECKED and false** — no `required_status_checks` rule (§4.3.4). What is checked is visibility, not blocking |
| The one live GET is acceptable risk | **UNCHECKED.** An availability judgement, not a measurement; the measurement is only that it is one request (§4.3.3) |
| The 2026-07-28 counts here | **Re-derivable, not pinned** — each ships its command; they drift with the tree, and §12 depends on none of them |

---

## §5 Behavior deltas

`preflight.py`'s exit code never moves from 1 to 0. **The space is not six cells, and draft 2's claim that
it was "the full 3×2" was arithmetic that did not survive round 2** (§14 D7).

**Axes**: CLI present/missing (2) × label-map module importable/not (2) × catalog reachable/not (2) ×
mode `default` / `--no-verify` (2) = **16**, and the memo's §3 label shape (labelled / label-less)
discriminates wherever the classification consults a label — i.e. it multiplies the rows where a capability
is absent. **Collapse rule** (why 16 is not published as 16): the three capability causes are a *union*
(§4.2.3 item 4), so any combination of absent causes yields one *unavailable* verdict; what differs between
them is the **diagnostic**, not the exit code. The catalog cause additionally only materialises when a
label misses the 24 pinned keys. The outcome-distinct rows are therefore:

| # | CLI | module | catalog | mode | §3 labels | Carve as authored | After A |
|---|---|---|---|---|---|---|---|
| 1 | ✓ | ✓ | ✓ | default | labelled | 0 (15 verified) | **0** |
| 2 | ✓ | ✓ | ✓ | `--no-verify` | labelled | 0 | **0** |
| 3 | ✗ | ✓ | ✓ | default | labelled | 1 (15 per-citation failures) | **1** — one diagnostic line |
| 4 | ✗ | ✓ | ✓ | default | label-less | **0** (`citations` empty ⇒ the verify block is skipped) | **1** |
| 5 | ✗ | ✓ | ✓ | `--no-verify` | either | 0 | **0** — capability unused |
| 6 | ✓ | ✗ | ✓ | default | labelled | **0** (21 unmapped, nothing verified) | **1** |
| 7 | ✓ | ✗ | ✓ | default | label-less | **0** | **1** (§4.2.2) |
| 8 | ✓ | ✗ | ✓ | `--no-verify` | either | 0 | **0** (J3) |
| 9 | ✓ | ✓ | ✗ | default | label outside `SPECS` | **`SystemExit`, no output** | **1**, naming the fetch failure |
| 10 | ✓ | ✓ | ✗ | `--no-verify` | label outside `SPECS` | **`SystemExit`, no output** | **0** (J3), rows reported *uncertified* |
| 11 | ✓ | ✓ | ✗ | either | all labels pinned | 0 | **0** — the catalog is never consulted |
| 12 | ✗ | ✗ | ✗ | default | any | 1 | **1**, diagnostic names every absent cause |

**Flags actually used**: every measured row ran with `--no-grep-pass`, because the sandbox's `REPO_ROOT` is
the sandbox and grep-pass reports 44 artefact hard-findings there. `--no-grep-pass` is **not** the default
(`dest="grep_pass", default=True`, `:275-278`), so the `mode` column above is the *verify* axis only;
grep-pass is held off in all of them. Draft 2 labelled these rows "default" without saying so (§14 D8).

**Measured vs predicted**: the *"Carve as authored"* column is measured — rows 1/3/6/8 in the §4.2.1
sandbox, rows 4/9/10/11 this round, row 7 against draft 1's patch. The *"After A"* column is **predicted**
by construction: A is unimplemented. §6's pins are what convert each prediction into a check.

**Breadth is preserved in every row.** Because §4.2.3 keeps the loop, `K` counts distinct labels in rows
6-12 exactly as in row 1; draft 2's skip collapsed it to 0 (measured: a 7-spec fixture went
`⚠ SPLIT-DEFAULT` → `ok (single PR scope)`, `--strict-breadth` 1 → 0). P3 pins `K`, not just the exit code.

**Newly-red**: rows 4, 6, 7, 9. All four require an absent capability; rows 9-10 are the ones no in-flight
worktree can currently hit *without* going offline, which is why §13's landing checklist re-runs the gate
per worktree rather than arguing from here.

---

## §6 Test plan

Two fixture memos ship (§0.5): `labelled.md` (two labelled rows) and `unlabelled.md` (rows opening with `§`).

**`.claude/skills/elidex-plan-review/test_preflight.py`** (new):

- **P1** the `preflight.shortname_from_label(label) == short` derivation assertion, from
  `test_cite_audit.py:275`, with no `sys.path` mutation in the test body and a `setUp` assertion that the
  module is un-poisoned (§4.5 item 2).
- **P2** `_webref` unimportable → **exit 1** (row 6), via the `importlib.reload`-under-import-hook form
  (§4.5 item 1), `tearDown` restoring both the module **and** `sys.path`.
- **P2b** the same via a subprocess, pinning `preflight.py:56-60`'s `except Exception`. Mutation check:
  deleting that clause must turn P2b red — P2 alone leaves it green.
- **P3** `--no-verify --no-grep-pass`, module absent → **exit 0** (row 8, J3), **and** `K` equals the
  distinct-label count, **and** the summary reports *uncertified* distinctly from *unmapped*. The `K`
  assertion is the pin draft 2 lacked; without it the skip regression is invisible.
- **P4** the **label-shape independence property**: `labelled.md` and `unlabelled.md` produce the *same*
  exit code in every capability state (rows 6/7 and 9/10). This is what the mis-sited placement breaks, and
  it pins the property directly — draft 2 proposed vendoring the rejected patch as a fixture, which would
  have created a second copy of `shortname_from_label`'s control flow in the test tree (§14 D10).
- **P5** each of the five remedy strings (§4.2.4) appears for its own cause and no other — including the
  label-less cell, which today prints the `SPECS` advice against `<empty>`.
- **P6** the missing CLI is reported once, not once per citation, exit code still 1 (row 3).
- **P7** catalog unreachable + default → **exit 1** naming the fetch failure (row 9). `urlopen` patched to
  raise `URLError`; asserts **no `SystemExit` escapes** and that the message is not the `SPECS` advice.
- **P8** catalog unreachable + `--no-verify` → **exit 0** with rows *uncertified* and `K` intact (row 10).
- **P9** all-pinned labels + catalog unreachable → exit 0, and `urlopen` is **never called** (row 11) —
  pins that A did not make the catalog a precondition of every run.

**`.claude/tools/_webref/test_spec_labels.py`** (new by §4.0's split): `TestSharedSpecLabelMap`'s 9 tests
minus the `preflight` assertion, plus the `coverage_map` half at module-level import, plus:

- **S0** `_catalog()` under `urlopen` → `URLError` returns *unavailable(cause)* and **no `SystemExit`
  escapes**; `resolve_label` returns `UNCERTAIN` for a non-pinned label and `MAPPED` for a pinned one.

Slice B appends its catalog round-trip cases (its S1-S5) to this file rather than creating it.

**Enforcement**: `mise run tools-test` and the `tools` CI job — §12 (1).

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** — not applicable; no `crates/**` diff.
**ECS-native** — not applicable; no component, no entity, no system.

**`DESIGN.md` generic-core / elidex-adapter split** — the live boundary:

| Edit | Layer | Placement |
|---|---|---|
| `spec_labels.py`'s catalog reach + the discriminated `_catalog()` (§4.2.3) | **generic** | a property of upstream's catalog and of the fetch layer's failure mode |
| `spec_labels.py`'s `SPECS` table | **generic mechanism + pinned elidex conventions** | ⚠ not unqualified generic: `SPECS` pins *this repo's* `"WHATWG "` display prefix and the parse aliases that exist because "real comments and memos abbreviate". `DESIGN.md`'s closing rule puts elidex policy in adapters or documentation, so this is externalization debt A inherits from the carve and does not increase — recorded rather than asserted away |
| `commands/coverage_map.py` consumer | **generic** | second consumer of the shared map |
| `cli.py`'s blurb derivation | **generic wiring** | consumes `SHORTNAME_TO_BLURB`; adds no elidex policy (the pre-existing elidex path at `cli.py:78` is untouched) |
| `_webref/DESIGN.md`, `spec_labels.py` bullet only | **generic** | describes a generic module; the CI facts do **not** go here (below) |
| `test_spec_labels.py` | **generic** | tests a generic module |
| §4.2 capability verdict, remedy text, `test_preflight.py`, `SKILL.md`'s contract | **elidex skill** | consumes the library, adds no generic behavior |
| §4.3 script + `mise` task + CI job | **elidex repo infrastructure** | `.claude/tools/python-suites.sh`, `mise.toml`, `.github/workflows/ci.yml` |

⚠ Draft 2 planned to record the `mise` task, the GitHub job, the path filter and the interpreter floor in
`_webref/DESIGN.md`. That file says the core should "stay generic enough to move to a standalone repository
later" and to "keep new generic behavior free of elidex-specific file paths" — a section describing
`mise.toml` and `ci.yml` travels with the tree at externalization and is wrong on arrival (§14 D-a). Those
facts live in `python-suites.sh`'s header and the `mise.toml` task comment, where `trip-wires` documents
itself.

⚠ Draft 2 also claimed the moved `preflight` assertion was "the one edge" of elidex coupling in the generic
tree, calling `spec_labels.py:7`'s skill path "pre-existing". Measured on `origin/main`:
`git grep -nE '\.claude/skills|elidex-plan-review' -- .claude/tools/_webref/` returns **exactly one** hit,
`cli.py:78`. `spec_labels.py` does not exist there, so its skill-path prose is **A's own new** coupling into
the module §7 calls generic. The narrow claim survives — the moved assertion is the one *import-time
executable* edge — but the attribution did not (§14 D-b).

**One-issue-one-way**, two collapses: the suite invocation goes from zero canonical sites to one (§4.3.2),
and the `WEBREF.is_file()` question from *n*-per-citation to one verdict (§4.2.3). Two further instances of
§1's class are named rather than silently left: `preflight` still reaches `resolver.lookup_section` through
a subprocess while reaching `spec_labels` in-process (§11 slot), and `grep_pass.py:143-148` reports a wrong
repo root as one HARD finding *per referenced path* — the same shape §4.2.1 measured as "44 hard findings …
an artifact". The latter is **not A's**: `git diff --name-only origin/main...HEAD -- .claude/` shows
`grep_pass.py` untouched, and it is `grep_pass`'s own precondition, not the citation gate's. Named here so
the enumeration matches the evidence this memo generated.

---

## §8 Line-count budget

`wc -l` on **`origin/main`** (2026-07-28; draft 1's figures were branch-relative):

| File | On `origin/main` | After A (est.) | Note |
|---|---|---|---|
| `.claude/skills/elidex-plan-review/preflight.py` | 499 | ~500 | −30 local map, +20 import, +~10 precondition |
| `.claude/skills/elidex-plan-review/test_preflight.py` | — | ~180 | new (P1-P6) |
| `.claude/tools/_webref/spec_labels.py` | — | 136 | from §4.0's A column |
| `.claude/tools/_webref/test_spec_labels.py` | — | ~125 | §4.0's 121-line split, minus the moved assertion |
| `.claude/tools/_webref/commands/coverage_map.py` | 114 | ~108 | from §4.0 |
| `.claude/tools/_webref/cli.py` | 264 | ~272 | the blurb-derivation half only |
| `.claude/tools/_webref/DESIGN.md` | 134 | ~141 | the `spec_labels.py` bullet only |
| `.claude/tools/python-suites.sh` | — | ~25 | new |
| `mise.toml` | 136 | ~142 | `[tasks.tools-test]` + one `depends` entry |
| `.github/workflows/ci.yml` | 126 | ~150 | `tools` filter + job |

**1000-line touch-time check** (cohesion-based): the largest file in the touch set is `preflight.py` at
499 → ~500, half the threshold, and it is one cohesive gate whose seam (structure / breadth / citation /
grep-pass) is already four ordered blocks in `main`. Nothing is near a split.

---

## §9 Edge-dense assessment

The **base case** applies: an approved umbrella's narrowly-scoped, plan-reviewed per-PR slice is a terminal
unit, not re-split for touching the same subsystem as B/C/D.

What makes A *safe* as one slice, stated **without** draft 2's completeness claim (§14 D7): J1-J3 live in
one function's control flow with one primary observable (an exit code) and one secondary one (the summary's
classification of each row). §5 does not claim to publish the whole 16-cell space; it publishes the
outcome-distinct rows **plus the collapse rule that maps the rest onto them**, which is the honest form and
is checkable — every row has a §6 pin. J4 is independent and is three files of configuration.
`git diff --stat -- crates/` is empty and stays empty, so a regression degrades a developer tool and cannot
reach a page, a script, or a user.

The ordering couplings are the umbrella's rules, not exemptions: retiring the grep requirement before the
detector is sound would mandate an under-reporting detector (C after B), and the regression pins are
unenforced until a scheduler exists (A before B).

---

## §10 Open questions for `/elidex-plan-review`

Four questions draft 2 listed are **decided here instead**, because each had one live option and listing
them was decision-surface, not review-surface ([[feedback_no-low-value-choices]]): the `verify_citation`
guard is an **explicit raise**, not an `assert` (an assert is stripped under `-O`, reinstating the silent
non-zero the change removes); the re-carve is **its own commit, first on A's branch**; the offline
obligation goes to the umbrella **unconditionally**; and the interpreter floor is **3.9**, A's measured
need. What remains genuinely open:

- **Q1 — `tools` path-filter breadth, given a measured collateral.** After A, PR #491's
  `.claude/tools/layout-box-reader-allowlist.tsv` regeneration triggers the Python suites — another lane's
  PR inheriting a network-touching job. Three options, not two: (a) the broad filter, accepting it;
  (b) name only the two suite-bearing directories, which silently stops covering a third;
  (c) **`python-suites.sh` derives its own suite set and fails loudly when a `test_*.py` lands outside the
  filtered paths** — the same "script is SoT, runners are callers" shape J4 already mandates, which removes
  both horns. **Recommendation: (c) plus the broad filter.** Draft 2 accepted (a) without considering (c).
- **Q2 — does `required_status_checks` belong in this PR?** §4.3.4's correction makes it one rule on an
  existing active ruleset. But measured, the `pull_request` rule already carries
  `required_approving_review_count: 0` **and** a `RepositoryRole` bypass with `bypass_mode: always`, so
  adding the rule leaves it author-bypassable — the change buys visibility-plus-friction, not enforcement.
  **Recommendation: register, do not implement** (§11), because deciding *which* jobs are stable enough to
  require is entangled with the Layout lane's trip-wire work.
- **Q3 — `#11-layoutbox-trip-wire-not-in-ci`: its trigger has already fired *twice*, and not because of A.**
  PR #381 (`actions/checkout` 6→7, open since 2026-06-21) touches `.github/workflows/ci.yml`, so the "next
  `.github/workflows` touch" happened independently of this slice. `feedback_defer_lifecycle_policy`
  Control D requires a fired trigger to receive one of the five formal dispositions with a new date, not a
  re-defer recorded inside another slice's memo. **Recommendation: A performs the formal re-classification**
  (defer-with-new-date, owner = Layout lane, obstacle text corrected in both files) rather than describing
  one. Whether A should instead *discharge* it is the reviewer's call; the filter-placement ground stands
  (the trip-wires read `crates/**`), but draft 2's second ground — entanglement with C-4 — **inverted the
  Layout lane's own record**, where C-4 is the reason to wire them (§14 D9).
- **Q4 — is the §3 "no spec surface" gap (§0.5 ⚠) sized out of A correctly?** It is pre-existing, it is in
  A's own file, and §11 registers it with a trigger and a date. The alternative is to fix it in A: an
  accepted input shape (`§3` heading + an explicit "no spec surface" marker) is perhaps 20 lines in
  `preflight.main`. **Recommendation: register, do not fix** — it changes what the gate *requires of every
  author*, which is an authoring-contract change, and A's own §4.1 routes authoring-contract changes to C.
  Review should overrule if "in A's own file, 20 lines" outweighs that.

---

## §11 Defer slots + per-PR ≤3 audit

**Two own deferrals** against ≤3, plus **two pre-existing-category entries** which are a separate class
([[feedback_defer_cap_policy]]).

⚠ **Naming/counting rule, settled at umbrella level rather than per-memo.** The registry treats `cleanup-*`
as cap-exempt — both existing entries carry *"non-spec; not a `#11-` cap slot"*. B's memo takes the stricter
line (*"counted against the cap anyway, because the discipline is restraint, not accounting"*). Two memos in
one program cannot answer this differently, so A's landing edit puts **the stricter rule in the umbrella's
"Constraints each slice inherits"**: `cleanup-*` names are kept for non-spec tooling, and they count.

### Own deferrals (2 of ≤3)

| Slot | Audit |
|---|---|
| **`cleanup-webref-preflight-inprocess-resolution`** | `preflight.verify_citation` forks a subprocess **and** an HTTP conditional-GET per unique citation, while the same file reaches `spec_labels` in-process — two ways to reach the shared library in one file. **Create-time**: pragmatic-shortcut ✓. **Category (3-gate)**: category 2, 別 slot 依存 — the collapse decides whether a plan-review gate must be usable offline, and §4.2.3 has just made *catalog* reachability an explicit capability, so the offline policy is now a live, adjacent decision rather than a hypothetical. **Confirming Q2 (middle state)**: fires, and is answered rather than overridden — the middle state is one process boundary, is named in §7, and collapsing it *now* would decide the offline policy by side effect, which is the failure §4.2.3 exists to stop. **Boundary cost**: the collapse direction makes the elidex adapter depend on `resolver`, which `DESIGN.md` does not list among its declared generic surface — part of the deferred decision, not a hidden cost. **Trigger**: the offline-gate policy decision, or Slice B's detector landing. **Re-eval**: 2026-11-30. |
| **`cleanup-webref-suites-offline`** (NEW at draft 3) | §4.3.2 adds `tools-test` to `[tasks.ci].depends`, i.e. to CLAUDE.md's **mandatory pre-push gate**, and A's own suite fetches `ed/index.json` with no offline mode. Draft 2 argued this adds "no new *class*" of requirement because `cargo deny` also fetches — **false, and its own bullet said so**: cargo-deny keeps a persistent local advisory DB with an `--offline` escape, while `ELIDEX_WEBREF_NO_CACHE=1` makes webref *more* networked. **Create-time**: pragmatic-shortcut ✓. **Category**: category 2 — the fix is an offline mode in `cache.py`, whose failure semantics §4.2.3 already discriminates but does not change. **Confirming Q2**: does not fire; there is no second mechanism, only an absent one. **Trigger**: a contributor hitting it, or Slice B's `cache.py` work. **Re-eval**: 2026-11-30. |

### Pre-existing category (not own deferrals, not counted)

| Entry | Why pre-existing, and its trigger |
|---|---|
| **`cleanup-elidex-ci-required-status-checks`** | The `main-protection` ruleset (id 13294991, active) has no `required_status_checks` rule, so every CI job is advisory. Pre-existing repo state; A neither creates nor worsens it. ⚠ The cost is **not** "one rule": the `pull_request` rule carries `required_approving_review_count: 0` and a `RepositoryRole` bypass with `bypass_mode: always`, so the rule alone would remain author-bypassable (§10-Q2). **Trigger**: the Layout lane wiring the trip-wires, or the first job stable enough to require. **Re-eval**: 2026-11-30. |
| **`cleanup-preflight-no-spec-surface-verdict`** (NEW at draft 3) | `preflight` cannot express "this slice has no spec surface": `origin/main:334` hard-fails on 0 data rows, so a zero-spec slice must author fixture citations and then receives `citation verify: ok` as a headline (§0.5 ⚠). Pre-existing on `origin/main`; A inherits it. Draft 2 called this "routed rather than slotted" to Slice B — which left it with **no trigger, no date and no ledger entry**, the open-ended-timing form the cap policy forbids, and mis-targeted besides (§14 D-c). **Trigger**: Slice C's `axes.md` work, which is where the authoring contract lives. **Re-eval**: 2026-11-30. |

**Explicitly NOT deferred**: the re-carve (§4.0), the three-cause capability verdict and the discriminated
`_catalog()` (§4.2.3), the four remedy strings (§4.2.4), the test relocation (§4.4), the test-siting
constraints (§4.5), the script + `mise` task + CI job (§4.3), `SKILL.md`'s contract update (§4.1), and the
umbrella's naming/counting rule.

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

Non-zero, with at least one failure attributable to each of P2, P2b, P3, P4, P6, P7, P8 and S0. **P4 is the
load-bearing one** and is now a property assertion (label-shape independence), so it needs no vendored copy
of the rejected patch to be runnable — which is what draft 2's version required (§14 D10).

**(3) A carries no part of B — filenames *and* prose**, ranged from the re-carve:

```sh
git diff --name-only <re-carve-A-commit>..HEAD -- .claude/tools/_webref/     # only §4.0's A column
git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/         # → empty
```

The second command is not decoration: §4.0's split is at hunk granularity, and six prose sites in the A
column describe `commands/cite_audit.py` as extant. A filename-only check — which is all draft 2 had —
passes while every one of them is present.

**(4) The branch carries only A's own memo.** Measured today: `git diff --numstat origin/main...HEAD` also
lists `2026-07-citation-hygiene-B-detector-correctness.md` (**696**) and `-C-policy-retirement.md`
(**103**) — 799 lines of two other slices' plan-memos, invisible to (3) because it is scoped to
`.claude/tools/_webref/`. Either they move to their own branches before A opens, or A's PR description
states why the program's memos ship together. **Recommendation: they move**, since B's memo is B's PR's
own artifact.

**(5) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page. Verified by observation; today the same observation yields zero jobs (§4.3.1).

Reference figures for (3)/(4), verified 2026-07-28: the `_webref` delta is **7 files, +925 / −34**
(`git diff --numstat origin/main...HEAD -- .claude/tools/_webref/`). Draft 2 said "six files / 945 lines";
945 is the whole-`.claude` insertion sum, which also mis-attributed A's own `spec_labels` / `preflight` /
`coverage_map` lines to Slice B (§14 D-d).

---

## §13 Coordination

Re-derived 2026-07-28 with **three-dot** ranges and a **complete** `gh pr list --state open` — draft 2
tabulated two of the four open PRs (§14 D-e).

| Lane | Overlap with A | Ordering rule |
|---|---|---|
| **Slice B** | total by construction — B branches from A's landed head, takes §4.0's B column, and **consumes** the discriminated `_catalog()` A lands rather than introducing it (B §4.1.7 rewritten in the same commit) | **A first** |
| **Slice C** | `_webref/DESIGN.md` — same file, disjoint sections. Also inherits the §3 no-spec-surface entry (§11) | after B |
| **PR-A0 / D** (`domform-submittable-category` @ `04a771b5`) | identical 8 `.claude/` files (`git diff … -- .claude/` = **0 lines**); drops its `.claude/` half once A/B land | after A/B/C |
| **PR #381** `dependabot/github_actions/actions/checkout-7` — **OPEN since 2026-06-21** | ⚠ **actual `ci.yml` contention.** Introduces `.github/workflows/{audit,ci,codeql}.yml`, bumping `actions/checkout@v6 → @v7` inside the same `steps:` blocks §4.3.2 extends | whichever lands second rebases; A's new `tools` job must use v7 if #381 lands first |
| **PR #491** `layout-c4-classification-fix` | introduces `.claude/tools/layout-box-reader-allowlist.tsv`, inside A's `tools` filter — an allowlist regeneration would run the Python suites (§10-Q1) | none, but state it in the PR description |
| **PR #489** `vm-p4-slice0a` | **none.** `git diff --name-only origin/main...vm-p4-slice0a \| grep -cv '^crates/script/elidex-js/'` → **0** | none |
| **PR #486** `dependabot/cargo/rust-dependencies-…` | none — `crates/**` + lockfile only | none |
| **C-3 plan / turn-completion / slice1** | no file overlap; preflight-behaviour overlap only | landing checklist |

**`#11-layoutbox-trip-wire-not-in-ci` — formal re-classification, not a re-defer** (§10-Q3). Its trigger
("the next `.github/workflows` touch") fired **independently of A**, via #381. Per
`feedback_defer_lifecycle_policy` Control D a fired trigger takes one of the five dispositions with a new
date; A records **defer, owner = Layout lane, new re-eval 2026-10-27**, with the obstacle text corrected.
⚠ That text is **not** in `project_open-defer-slots.md` — that entry is one line with no obstacle prose. It
is at `project_inline-mod-split-owed.md:84-85`. Both files are edited or the stale sentence survives where
the lane reads it. Note also that A's job runs `bash python-suites.sh`, so *"CI invokes no `mise` task"*
stays **literally true** after A; what A establishes is the direct-bash-call route, one of the two options
that sentence already named.

**Landing checklist**:

1. Re-run `preflight.py` on the plan-memos each lane is **authoring** — `elidex-wt-c3-plan`,
   `elidex-wt-vmp4plan`, `elidex-wt-turncomp`, `elidex-wt-slice1`, and `elidex-wt-submittable`
   (specifically `docs/plans/2026-07-form-submittable-category-repair.md`, this memo's §4.2.1 fixture, plus
   its two siblings) — from **each worktree's own copy**, since `REPO_ROOT` derives from `__file__`.
   `elidex-wt-c4fix` is **dropped**: `git diff --name-only origin/main...layout-c4-classification-fix --
   docs/plans/` → 0, so it authors none. Slice C's memo is expected red (no §3 table, by design).
2. Register the 2 own deferrals and the 2 pre-existing entries; add the umbrella's naming/counting rule and
   the offline obligation to "Constraints each slice inherits"; correct the trip-wire obstacle text in
   **both** `project_open-defer-slots.md` and `project_inline-mod-split-owed.md` §B. Memory-file writes, not
   chips ([[reference_spawn-task-chips-not-durable]]).
3. Fix the umbrella's own branch-measured figures at `:44-45` — *"the 48-test `_webref` suite fetches 2
   URLs"* is the branch count; on `origin/main` `_webref` is **12 tests**, and A's own suite fetches **1**
   URL (§4.3.3). Same edit as item 2, since it opens the same file.
4. Correct the SHAs the 2026-07-28 rebase invalidated — `project_citation-hygiene-program.md` (§State's
   `45bd11bc` / `d3173bed`, **and** the same "2 behind" phrasing under §"▶ Next action") and
   `project_slice1-elementstate-cache-deletion-state.md:68`.
5. `MEMORY.md`'s L3 bullet: drop the completed "▶ next = re-slice → plan-review" **now** (the umbrella says
   this record class updates now, not at landing); set A-landed / B-next at landing.
6. PR description states §4.3.3 (one conditional GET per run), §4.3.4 (no `required_status_checks`, and the
   bypass actor that makes adding one weaker than it sounds), §10-Q1 (the #491 effect) and the #381
   contention.

---

## §14 What the two review rounds changed

Recorded because the program's thesis is that a silent correction is the defect it exists to remove. Every
item was found by `/elidex-plan-review` and then **independently re-derived** before being accepted; where
re-derivation contradicted the reviewer, that is recorded too.

### Round 1 → draft 2 (superseded by draft 3; kept as an index, not restated)

Draft 1's root cause was one thing: **it measured its evidence base on the branch, not on `origin/main`**.
Everything else followed. C1 the slice boundary and the 83/5 test count (→ §4.0, §4.3.1) · C2
`shortname_from_label`'s "unreachable" branch (→ §4.2.3) · C3 branch protection via the deprecated endpoint
(→ §4.3.4) · C4 two-dot ranges fabricating a lane overlap (→ §13) · C5 the offline gap routed to a
non-owner (→ §11) · C6 a Python 3.11 floor that was Slice B's need (→ §4.3.5) · C7 a six-cell "complete"
matrix (→ D7) · C8 CI facts headed for the externalizable core (→ §7) · C9 a trip-wire rationale that
inverted the Layout lane's record (→ §10-Q3) · C10 a red-check reading a patch committed nowhere (→ D10) ·
C11 slot obstacle text attributed to the wrong file (→ §13). Verification commands for each are at the
section arrowed.

### Round 2 → draft 3

| # | Draft 2 said | Measured |
|---|---|---|
| **D1** | The capability has **two** causes (CLI, import) | **Three.** A's new import reaches `cache.py:131 sys.exit`, and `SystemExit` escapes `_catalog()`'s `except Exception`. Offline + any label outside the 24 pinned keys ⇒ bare exit from inside the row loop, no `--no-verify` escape. **J3 broken by the slice written to protect it**; `origin/main` exits 0 on the same input. §4.2.3 rebuilt around it. |
| **D2** | `main` skips the per-row mapping call when the capability is absent | The skip strands `unique_specs` (both writers are downstream), so **`K` → 0 and the breadth gate silently passes**: a 7-spec fixture went `SPLIT-DEFAULT` → `ok (single PR scope)`, `--strict-breadth` 1 → 0. `specs_seen` strands identically; under §4.6's wider spelling so does `malformed_rows`, killing a structural hard-fail. The loop is no longer skipped. |
| **D3** | §4.2.3 item 3 ("skips the per-row mapping call") and §4.6 ("the row loop is skipped") | Two incompatible spellings of one change, resolvable only by coin flip. Both removed. |
| **D4** | §3.1: `shortname` "is bounded by the `SPECS` map" | That is `origin/main`'s bound — the dict A *deletes*. After A the bound is the **948-entry catalog**. The conclusion holds on the corrected premise; the audit also covered one of two argv elements. |
| **D5** | §3.1: the exposure delta is outward | The **inbound** half is larger and lands in A's own file: which labels the gate recognises becomes a third-party document fetched at gate time, on every plan-review in every lane. |
| **D6** | `verify_citation`'s guard becomes an `assert`, with the `-O` objection routed to review | Under `-O` the assert is stripped and the caller gets exactly the silent non-zero the change removes. Decided in-plan: explicit raise. |
| **D7** | "the full 3×2 capability × mode space" (stated three inconsistent ways: 6/12, 6/9, 3×2+1) | The space is 2×2×2×2 = **16**, and the label-shape discriminator flips the verdict at cells draft 2 applied it to only one of (measured: CLI-missing + default + label-less exits **0**, contradicting its own row 3). §5 now publishes outcome-distinct rows **plus the collapse rule**, and §9 drops the completeness claim. |
| **D8** | §5 rows labelled "default" | All were measured with `--no-grep-pass`, which is **not** the default (`default=True`). The flag axis is now stated. |
| **D9** | §11: two `cleanup-*` slots, counted against ≤3 | The registry treats `cleanup-*` as cap-exempt (both existing entries: *"not a `#11-` cap slot"*). Two memos in one program cannot answer this differently ⇒ the stricter rule moves to the umbrella. Also re-sorted: 2 own deferrals, 2 pre-existing-category entries. |
| **D10** | §12: ship draft 1's patch as a fixture so P4 can assert against it | That vendors a rejected implementation of `shortname_from_label` into the test tree — a middle state created to make a check runnable. P4 now pins the **property** (label-shape independence) directly. |
| **D-a** | §4.4: `spec_labels.py:7`'s skill path is "pre-existing, none A's" | On `origin/main`, `git grep -nE '\.claude/skills\|elidex-plan-review' -- .claude/tools/_webref/` returns **one** hit (`cli.py:78`). `spec_labels.py` does not exist there, so that coupling is **A's own new** one. The narrow "one import-time edge" claim survives; the attribution did not. |
| **D-b** | §7: `spec_labels.py` is "generic — a property of upstream's catalog" | True of `_catalog()`, false of `SPECS`, which pins this repo's `"WHATWG "` prefix — `DESIGN.md`'s own closing rule calls that elidex policy. Now qualified and recorded as inherited externalization debt. |
| **D-c** | §0.5/§11: the no-spec-surface gap is "routed rather than slotted" to Slice B | Routing to B is a category error twice over (capability-vs-artifact is A's own J1; the umbrella forbids B editing review policy), and "routed" left it with no trigger, no date and no ledger entry. Now a pre-existing-category entry owned toward C. |
| **D-d** | §14 C1 / §12: "six files … 945 lines of Slice B" | **7 files, +925 / −34** (verified 2026-07-28, `git diff --numstat origin/main...HEAD -- .claude/tools/_webref/`). 945 is the whole-`.claude` sum and mis-attributes A's own `spec_labels`/`preflight`/`coverage_map` lines to B. |
| **D-e** | §13: "no branch in the repo currently introduces a `.github/workflows` change" | **4** open PRs, not 2. **#381** (`actions/checkout` 6→7, open since 2026-06-21) touches `ci.yml` — actual contention, and the trip-wire slot's trigger had already fired independently of A. |
| **D-f** | §12 (3) checks the slice boundary | Scoped to `.claude/tools/_webref/`, so it cannot see the **799 lines** of B's and C's plan-memos the branch also carries, nor the six prose sites in the A column that name `cite_audit.py`. Both now checked. |
| **D-g** | §11 and §10-Q5 (draft 2) | Draft 2 silently **reversed** draft 1's Q5 recommendation ("register nothing" → "register it as a slot"), moved the own-deferral count 1 → 2, and **deleted** draft 1's "two slots this slice does not register" paragraph — none recorded in §14, whose own preamble forbids exactly that. Recorded here. |

**One reviewer claim was re-derived and rejected**: round 1 flagged `coverage_map_label` as having more than
one caller. `grep -rn 'coverage_map_label' .claude/` → two hits, the `def` at `:313` and one caller at
`:308`. The memo's count was correct. Round 2's claim that `elidex-wt-c4fix`'s `docs/plans/` "is empty" was
also wrong (verified 2026-07-28 — 69 files, inherited from `origin/main`); the substance — that the branch *authors* no plan-memo — held
(`git diff --name-only origin/main...layout-c4-classification-fix -- docs/plans/` → 0), and §13 item 1 acts
on the substance.
