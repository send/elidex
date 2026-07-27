# Plan — Slice A: enforcement plumbing (fail-closed gate + a scheduler that runs the suites)

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A**. Under that umbrella's approval
boundary this is a **terminal unit** — it is not re-split for touching the same subsystem as B/C.
**Branch**: `webref-cite-audit-tool`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`.
**Base**: `96a8e47b` (rebased 2026-07-28; the three carry commits replayed clean, and
`git diff domform-submittable-category -- .claude/` is still **0 lines**, so the carve's
provenance-preserving property survived the rebase).
**Nature**: developer tooling + **CI topology**. Zero `crates/**` diff, zero engine behavior change.
**Status**: plan-memo. `/elidex-plan-review` **required before implementation** (umbrella rule).

### §0.1 What this slice is, in one sentence

Two gates in this repository make a claim they cannot currently support: `preflight.py` reports success
when its own label-map dependency is missing, and the 83 tests that pin both gates' behavior are run by
**no scheduler at all**. Slice A fixes the exit code and gives the suites a runner. It changes **no
detector semantics** — that is Slice B — and **no review policy** — that is Slice C.

---

## §0.5 Spec citation table

This slice implements no spec logic. The two citations below are the rows the new `test_preflight.py`
fixture memos carry; both were looked up with `.claude/tools/webref` on **2026-07-28**, nothing quoted
from memory.

| Cite | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| the labelled fixture row (P2/P3 — a row whose spec label maps) | HTML §4.10.21 | Constraints | `#constraints` | `heading --exact html 4.10.21` |
| the second labelled row, so the fixture memo has >1 citation to dedup | HTML §4.10.21.2 | Constraint validation | `#constraint-validation` | `heading --exact html 4.10.21.2` |

The third fixture row is deliberately **label-less** (`| §4.10.21 Constraints | … |`, the cell opening
with `§`). It is not a citation defect — it is the input shape that falsifies the placement §4.1.2
rejects, and it carries no spec label by construction.

---

## §1 Ideal anchor — a gate reports on the thing it audited, or it reports on itself

Two failures, one shape. A gate's output is a claim about the artifact under review. When the gate's own
infrastructure is missing, the honest output is a claim about the **gate**, not a verdict on the artifact.
Today both halves get this backwards:

1. `preflight.py` cannot import `_webref`, so every §3 row's label fails to map, so the row loop
   classifies 21 of 21 rows as *author cited a spec I do not know* — a documented soft-warn — and the
   gate **exits 0 having verified nothing** (§4.1.1, measured). The tool blames the memo for a fact about
   the tool.
2. The 83 tests that pin that behavior run under no `mise` task, no CI job, no hook. They pass only while
   an author remembers to invoke them by hand (§4.3.1, measured). An unscheduled suite is a claim with
   no checker — the exact shape this program exists to remove.

The corollary that drives the whole edit set, and the one the superseded draft got wrong:
**a capability is a process-level fact and must be established once, before the data loop.**
"I cannot map *this* label" is a datum about one row. "I cannot map *any* label" is a fact about this
process. Discovering the second by watching the first is what makes the failure look like data — and, as
§4.1.2 measures, it makes the fix's correctness depend on the *content* of the memo being reviewed.

---

## §2 Coupled invariants

- **J1 — capability ≠ datum.** A row is *unmapped* only if the mapper ran and declined. If the mapper is
  absent, no row is unmapped; the run is uncertified. Today one return value (`None`) carries both.
- **J2 — the two capabilities must degrade the same direction.** `preflight` needs two things to verify a
  citation: the `webref` CLI and the label map. Measured, one hard-fails and the other exits 0 (§4.1.1).
  The in-code comment claims they "degrade the same way". They do not.
- **J3 — one degradation must survive.** `--no-verify --no-grep-pass` (structure + breadth only) must keep
  working with the tools tree absent; that is the only degradation the comment was right to want, and it
  is the property a fail-closed change is most likely to break.
- **J4 — one enforcement mechanism, not two.** If `mise` and `ci.yml` each spell the suite invocation, a
  later suite is added to one and not the other. The repo already has the answer in `trip-wires`: the
  script is the SoT and the runner is a caller.

J1–J3 are all inside `preflight.main`'s control flow and cannot be applied one at a time without
transiently breaking each other — which is why §5 measures all six configurations rather than asserting
them. J4 is independent and is the reason §4.3 ships a script rather than two copies of two lines.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | fixture | the labelled `§3` row a fail-closed run must still map | §4.4 — `test_preflight.py` P2/P3 fixture memo | ✓ — the fixture set is authored, not discovered | no |
| WHATWG HTML §4.10.21.2 Constraint validation | fixture | a second citation, so `seen_pairs` dedup is exercised | §4.4 — same fixture | ✓ | no |

**Breadth**: K=1 spec (`html`), M=2 rows → preflight verdict **ok (single PR scope)**.

**Why the table is two rows and not padded**: this slice ships no spec algorithm. Rows here are test
fixtures, and a fixture set larger than the property under test is padding. CLAUDE.md's
"Supported-surface testing" asks what guards the surface; here the guard is §6's suite, not spec breadth.

### §3.1 User-input touch audit + discovery method

**No user-input flow.** Nothing in this slice is reachable from page content, script, or network. The
inputs are a developer-supplied plan-memo path and the repository's own tree.

**Discovery method.** Every defect and every number below was produced by **executing** the shipped code
on 2026-07-28 at branch head, not by reading it:

1. The gate asymmetry is a three-case sandbox run (§4.1.1) — a repo skeleton with the skill + tools trees
   so `REPO_ROOT` resolves, with one dependency removed per case.
2. The re-siting defect (§4.1.2) was found by **applying the superseded draft's own fix in that sandbox**
   and running it against two memos. It is a measurement of a proposed patch, not a reading of one.
3. The CI hole (§4.3.1) is read off `ci.yml`, `codeql.yml`, `audit.yml` and `mise.toml` — all four
   workflow-relevant files, not just `ci.yml`.
4. The network dependency (§4.3.3) was measured by counting `urlopen` calls through a spy, then re-running
   the same suite in the same process with the network blocked.

Three numbers in the superseded draft did **not** survive re-derivation; each is flagged **⚠ CORRECTS**
at its site (§4.1.2, §4.3.3, §4.3.4).

---

## §4 The edit set

### §4.0 The evidence base

All measurements: **2026-07-28**, branch `webref-cite-audit-tool` rebased onto `96a8e47b`, macOS,
`python3` = 3.14.6. The sandbox used by §4.1 and §5:

```sh
SB=$(mktemp -d)/sb; mkdir -p $SB/.claude/skills $SB/.claude/tools
cp -R .claude/skills/elidex-plan-review $SB/.claude/skills/
cp -R .claude/tools/_webref $SB/.claude/tools/; cp .claude/tools/webref $SB/.claude/tools/
M=../elidex-wt-submittable/docs/plans/2026-07-form-submittable-category-repair.md
python3 $SB/.claude/skills/elidex-plan-review/preflight.py $M --no-grep-pass; echo "EXIT=$?"
```

`--no-grep-pass` throughout: the sandbox's `REPO_ROOT` is the sandbox, so grep-pass reports 44 hard
findings for `crates/**` paths that do not exist there. That is an artifact of the sandbox, not a finding.

### §4.1 A1 — `preflight.py` must fail closed

#### §4.1.1 The measured asymmetry

`preflight.py:56-60` sets `_shortname_for = None` on any import failure; `shortname_from_label`
(`:232-237`) then returns `None` for every row, which `main`'s row loop (`:353-358`) classifies as
*unmapped* — a documented soft-warn. `citations` stays empty, the verify loop never runs, and the gate
exits 0. The in-code comment at `:52-55` claims this "degrade[s] the same way the pre-existing
`WEBREF.is_file()` check does". Measured, the two behave **oppositely**:

| Case | Removed | Result | Exit |
|---|---|---|---|
| **A** | nothing | 21 rows, 21 parsed citations, **15 unique citations verified** | **0** |
| **B** | `.claude/tools/webref` (pre-existing check) | `❌ HARD FAIL — citation verification: 15 failure(s)` | **1** |
| **C** | `.claude/tools/_webref` (the carried import) | `parsed citations: 0`, `unmapped-label rows: 21`, **no verify section at all** | **0** |

Case C also emits a **wrong-cause remedy** on stderr:
`(add the spec to .claude/tools/_webref/spec_labels.py::SPECS)` — the file that failed to import. An
author following it edits a file the gate cannot read.

`15` is the correct figure, not `21`: `seen_pairs` (`:382-388`) dedups 21 data rows to 15 unique
`(shortname, section)` pairs. The asymmetry itself is exactly as the case table shows.

#### §4.1.2 ⚠ CORRECTS — the tri-state cannot live in `shortname_from_label`

The superseded draft's fix part 1 put a tri-state in `shortname_from_label`. Applied verbatim in the
sandbox (a `TOOLS_UNAVAILABLE` sentinel returned from that function, propagated to `main`, hard-failing
there), and run against two memos with `_webref` removed:

| Fixture memo | Result |
|---|---|
| §3 rows carrying spec labels (`WHATWG HTML §4.10.21 …`) | **EXIT 1** — fails closed ✓ |
| §3 rows opening with `§` (no label: `\| §4.10.21 Constraints \| … \|`) | **EXIT 0** — still fails **open** ✗ |

Cause is the function's first line:

```python
def shortname_from_label(label: str | None) -> str | None:
    if not label:
        return None          # ← taken before the availability check below
    if _shortname_for is None:
        return None
    return _shortname_for(label)
```

`parse_spec_cell` (`:216-229`) returns `cell[:m.start()].strip()` as the label, so a cell beginning with
`§` yields `""`. Every such row short-circuits out before the availability branch. The gate's
fail-closed property therefore becomes **a function of the reviewed memo's cell formatting** — a memo
whose §3 rows happen to omit spec labels certifies a run that verified nothing, exactly as today.

This is J1 restated as a defect: the draft kept the capability check inside the per-row data classifier,
so a row that never reaches the classifier never reaches the capability check.

#### §4.1.3 The fix — one precondition, before the loop

Establish the verification capability **once in `main()`**, before the row loop, as a fact about the
process:

1. A `verification_capability()` returning *available* or *unavailable(cause)*, where the two causes are
   the missing CLI (`WEBREF.is_file()`) and the failed import (`_shortname_for is None`). Both are
   already single process-level facts; neither is a property of any row.
2. Unavailable **and** verification requested (i.e. not `--no-verify`) → HARD FAIL, printed in the same
   `❌ HARD FAIL — …` shape the other three hard-fails use so a CI grep catches all of them, with the
   cause named and `--no-verify` named as the suppressor.
3. Unavailable **and** `--no-verify` → no failure (J3), but the summary says *tools unavailable*, not
   *unrecognized labels*. The breadth half still works: `K` counts `unmapped:<label>` keys, which remain
   distinct per distinct label, so the split decision is unchanged.
4. `shortname_from_label` goes back to one job — `label → shortname | None`, a pure data classification.
   Its `_shortname_for is None` branch is no longer reachable from `main` and is deleted rather than left
   as a second site that answers the same question.

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is currently re-tested
inside `verify_citation` on **every unique citation** — 15 times in case A — and reports one process-level
fact as 15 per-citation failures. After the hoist, case B's exit code is unchanged (1) and its diagnostic
becomes one line naming the missing path. One question, one site, one answer (§10-Q1 asks review to
confirm this is in A's scope rather than adjacent to it).

#### §4.1.4 The remedy text

Two strings, currently one:

- genuinely unmapped label → "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the
  label spelling"
- tools unavailable → the import error and the path it was attempted from, plus `--no-verify`

Slice B inherits a third cause here (catalog unreachable, its §4.1.7). A must not pre-build a branch for
it — B's `_catalog()` result type does not exist yet, and a placeholder would be a slot B has to find.

### §4.2 What A deliberately does not touch

Recorded so the boundary is auditable rather than inferred:

| Concern | Slice | Why not A |
|---|---|---|
| `_CITE_RE`, `_attribute`, the comment scanner, `--strict`'s gated classes | **B** | detector semantics; A changes none |
| one shared `SECTION_NUMBER_RE` across `preflight` / `cite_audit` / `section_sort` | **B** | `preflight.SECTION_REF_RE` is A's file but B's grammar unification; A leaves it byte-identical so B's collapse is one edit, not a merge |
| `_catalog()` discriminated result; catalog-unavailable as a distinct preflight cause | **B** | A's precondition covers the import; the catalog's own availability is a `spec_labels` contract B is rewriting |
| `axes.md` requirement (2)/(4); `CLAUDE.md` § "Spec citation"; `DESIGN.md`'s reported-class contract | **C** | retiring a discovery method rests on a reach measurement only B can produce |
| the `crates/**` citation repairs and the 8 newly-authored wrong citations | **D** | content, not plumbing |

### §4.3 A3 — give the suites a scheduler

#### §4.3.1 The hole, measured across all three workflows

```sh
grep -n 'depends' mise.toml | grep ci          # [tasks.ci].depends = check lint test-all doc deny trip-wires ci-sweep
grep -n 'claude' .github/workflows/ci.yml      # → no matches (exit 1)
sed -n '/filters:/,/^  check:/p' .github/workflows/ci.yml
```

- `ci.yml`'s `changes` filter has two sets: `rust` (`crates/**`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `.rustfmt.toml`, `clippy.toml`, `mise.toml`, `.github/workflows/**`) and `config`
  (`deny.toml`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/**`). **`.claude/**` is in neither**, and
  all three jobs (`check`, `doc`, `deny`) are gated on one of the two.
- `ci.yml` **never invokes `mise`** — `check` runs `cargo fmt` / `clippy` / `nextest` / doc-tests directly.
  The single `mise` string in the file is `mise.toml` as a path-filter entry.
- `codeql.yml` analyses `[actions, rust]` and triggers on push-to-main + a weekly cron — **no Python, no
  `pull_request` trigger**. `audit.yml` is `cargo audit` on a weekly cron.

⇒ a `.claude/**`-only pull request today triggers **zero jobs**, and even the post-merge push runs only
cargo. The 83 tests are:

| Suite | Tests |
|---|---|
| `.claude/tools/_webref/test_cite_audit.py` | 36 |
| `.claude/tools/_webref/test_inventory_diff.py` | 6 |
| `.claude/tools/_webref/test_agent_brief.py` | 5 |
| `.claude/tools/_webref/test_refresh.py` | 1 |
| `.claude/skills/elidex-plan-review/test_grep_pass.py` | 35 |
| **total** (all green, re-derived 2026-07-28) | **83, across 5 files** |

```sh
# the 5 files, and the per-file counts above (verified 2026-07-28)
ls .claude/tools/_webref/test_*.py .claude/skills/elidex-plan-review/test_*.py | wc -l   # → 5
for f in .claude/tools/_webref/test_*.py; do python3 -m unittest discover \
  -s .claude/tools/_webref -p "$(basename $f)" -t .claude/tools 2>&1 | grep -E '^Ran '; done
python3 .claude/skills/elidex-plan-review/test_grep_pass.py 2>&1 | grep -E '^Ran '
```

Both `discover` invocations were verified to collect the full set: `-s .claude/tools/_webref -p 'test_*.py'
-t .claude/tools` → `Ran 48 tests … OK`; `-s .claude/skills/elidex-plan-review -p 'test_*.py'` →
`Ran 35 tests … OK`.

#### §4.3.2 The mechanism — one script, two callers (J4)

`.claude/tools/python-suites.sh`, `set -euo pipefail`, `cd` to the repo root, the two `discover` lines.
Then:

- `mise.toml` gains `[tasks.tools-test]` whose `run` is `bash .claude/tools/python-suites.sh`, added to
  `[tasks.ci].depends`.
- `ci.yml` gains a `tools` path-filter set (`.claude/tools/**`, `.claude/skills/**`,
  `.github/workflows/**`) and a `tools` job on `ubuntu-latest` that runs the same script under the same
  `|| github.event_name == 'push'` bypass the other three jobs use.

This is the `trip-wires` shape verbatim (`mise run trip-wires` calls four `.claude/tools/*.sh`), so it
introduces no new pattern — and it is the reason the local gate and the merge gate cannot drift into two
spellings of the same suite.

#### §4.3.3 ⚠ CORRECTS — the merge gate would take a live-network dependency

Not present in the superseded draft, and it is load-bearing: **the `_webref` suite requires the network on
every run.** Measured with a spy on `urllib.request.urlopen`:

- A full 48-test run fetches exactly **2 distinct URLs**:
  `https://raw.githubusercontent.com/w3c/webref/main/ed/headings/html.json` and
  `.../ed/index.json`.
- Re-running the identical suite **in the same process** with `urlopen` raising `URLError`:
  **48 tests, 0 failures** — so the dependency is those two resources, nothing more.
- Running with the network blocked **from the start**: **15 failures + 3 errors**, *even with the warm
  101 MB `~/.cache/elidex-webref`*, because `cached_fetch_url` (`cache.py:64-85`) always issues a
  conditional GET and `cache.py:130-131` `sys.exit`s on `URLError`. There is no offline mode:
  `ELIDEX_WEBREF_NO_CACHE=1` makes it *more* networked, not less.
- Cold cache with the network up: **48 tests in 0.24 s**, writing **4 files** — the two bodies above
  (293,409 B + 1,572,569 B) and their two 80 B `.meta` siblings, **1.79 MB total**. Derived by running
  under `XDG_CACHE_HOME=$(mktemp -d)` and mapping each cache key back through
  `hashlib.sha1(url).hexdigest()`, so the file set is *identified*, not just counted (verified
  2026-07-28). The cache-key mapping is what makes this the same two resources the spy found, rather
  than a coincidence of counts.

**Disposition — accept, and route the offline question to B.** Three facts carry it: (a) the dependency is
two small conditional GETs to `raw.githubusercontent.com`, the same provider the job's own
`actions/checkout` depends on, so the marginal availability risk on a GitHub-hosted runner is close to
zero; (b) `mise run ci` **already** requires the network — `deny` runs `cargo deny check`, which maintains
a fetched advisory database (`~/.cargo/advisory-dbs/`), so `tools-test` adds no new class of requirement
to the mandatory local gate; (c) the fix — serving the disk cache without revalidation — is a change to
`cache.py`/`spec_labels.py` availability semantics, which is precisely Slice B's §4.1.7. Building it in A
would be A changing library semantics to satisfy A's own CI job.

`actions/cache` for `~/.cache/elidex-webref` was considered and **rejected on the measurement**: the
conditional GET fires whether or not the body is cached, so a restored cache saves 1.8 MB of transfer and
**zero** requests. It would add configuration that buys nothing.

#### §4.3.4 ⚠ CORRECTS — what "mechanically checked" can honestly mean here

```sh
gh api repos/send/elidex/branches/main/protection   # → 404 "Branch not protected"
```

`main` has **no branch protection and no required status checks**. A red `tools` job therefore does not
block a merge by itself; CLAUDE.md's workflow ("CI 全 pass を目視確認してから squash merge") is the
blocking step, and it is a human one. The claim this slice may make is therefore: the job makes a
regression **visible, attributed, and on the PR page at review time**, where today it is invisible in
every event. That is a strict improvement and it is what §12's exit criterion asserts — no more.
Adding branch protection is a repository-settings change, not a file in this diff (§10-Q5).

#### §4.3.5 The interpreter floor

Measured: no `.claude` Python source uses syntax newer than 3.9 (`match`, `except*`, `tomllib`,
`typing.Self`, `ExceptionGroup`, atomic groups — all absent). Local dev is 3.14.6. Nothing in the
repository declares a floor.

The script asserts `sys.version_info >= (3, 11)` and the job echoes `python3 -VV`, so the runner's actual
version becomes a **measured fact on the first CI run** rather than an assertion in this memo. 3.11 is a
declaration with headroom, not a discovery; it is the floor Slice B's atomic-group grammar will need, so
declaring it here means B does not reopen `ci.yml`. If a runner image is below it, the failure names the
version and the fix (`actions/setup-python`) — which is why that action is **not** added pre-emptively
(§10-Q2).

### §4.4 A2 — move the consumer-derivation assertion off the tools package

`test_cite_audit.py:275` `test_all_three_consumers_derive_from_specs` asserts, for every `SPECS` entry,
both `coverage_map._spec_label(short) == label` and `preflight.shortname_from_label(label) == short`. To
reach the second it does, **inside the test method**:

```python
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "skills" / "elidex-plan-review"))  # :289-292
preflight = importlib.import_module("preflight")                                                # :296
```

So the *generic tools* package's test hard-codes the *elidex skill's* directory layout and module name —
the one edge that blocks `DESIGN.md`'s stated goal of keeping the drift-detection core movable to a
standalone repository. Two further defects at the same site: the `sys.path` mutation is never undone and
runs on every invocation, and `coverage_map_label` (`:313-316`) re-does an `importlib.import_module`
already performed at `:295`.

**Fix**: the `preflight` half moves to a new `.claude/skills/elidex-plan-review/test_preflight.py`, beside
`preflight.py` and `test_grep_pass.py` — the home already exists and the dependency direction is right
(consumer depends on library). `test_cite_audit.py` keeps the `coverage_map` half with a module-top-level
import beside `spec_labels` at `:27`, and `coverage_map_label` collapses into it. No `sys.path` mutation
survives inside any test method.

`test_preflight.py` is also where §6's P2/P3/P4 land, so A's fail-closed change ships with its own pins in
the file that A creates — and Slice B adds its two preflight cases to a file that already exists.

---

## §5 Behavior deltas

Nothing in `crates/**` changes. `preflight.py`'s exit code changes in exactly one direction — never from
1 to 0. All six configurations, measured at HEAD and stated for after:

| # | Config | Now | After | Note |
|---|---|---|---|---|
| 1 | everything present | 0 (15 verified) | **0** | unchanged; the common case |
| 2 | `webref` CLI missing | 1 (15 per-citation failures) | **1** | same code, one diagnostic line instead of 15 |
| 3 | `_webref` unimportable | **0** (21 unmapped, nothing verified) | **1** | the defect |
| 4 | `_webref` unimportable, §3 rows carry no label | **0** | **1** | §4.1.2 — the placement the draft chose leaves this at 0 |
| 5 | `_webref` unimportable, `--no-verify --no-grep-pass` | 0 | **0** | J3, the degradation that must survive |
| 6 | everything present, `--no-verify --no-grep-pass` | 0 | **0** | unchanged |

Rows 1, 2, 3, 5, 6 measured at HEAD; row 4 measured against the draft's patch applied in the sandbox.

**Newly-red configurations for other lanes**: only 3 and 4, both of which require a broken or absent
`.claude/tools/_webref` — a state no in-flight worktree is in. §13 makes re-running the gate on each
in-flight memo a landing-checklist item anyway, because a claim about five other worktrees is not a claim
this memo is entitled to make from here.

---

## §6 Test plan

Every new test must **fail before this slice's fix** — §12 makes that runnable rather than promised.

**`.claude/skills/elidex-plan-review/test_preflight.py`** (new):

- **P1** the `preflight.shortname_from_label(label) == short` derivation assertion, moved from
  `test_cite_audit.py:275`, with no `sys.path` mutation inside the test body.
- **P2** `_webref` unimportable → **exit 1** (delta row 3, inverted). Import made to fail by construction,
  not by moving a directory, so the test is hermetic.
- **P3** `--no-verify --no-grep-pass` with the tools tree absent → **exit 0** (J3, delta row 5). This is the
  pin that a future tightening of the precondition cannot silently break.
- **P4** `_webref` unimportable **and** a fixture memo whose §3 rows carry no spec label → **exit 1**
  (delta row 4). This is the §4.1.2 regression; it is the one test that distinguishes the correct siting
  from the draft's, and it fails against the draft's patch.
- **P5** the tools-unavailable diagnostic does **not** say "add the spec to … `SPECS`" (§4.1.4).
- **P6** the hoisted precondition reports the missing CLI once, not once per citation (§4.1.3), while the
  exit code stays 1 — pins that the collapse did not weaken case B.

**`.claude/tools/_webref/test_cite_audit.py`** (410 today):

- the `preflight` half of `test_all_three_consumers_derive_from_specs` removed; the `coverage_map` half
  kept, renamed to what it now checks, with a module-level import.
- `coverage_map_label` (`:313-316`) deleted, its one caller using the module-level import.
- net: −1 test in this file, +1 in `test_preflight.py`; the total stays 83 + the new pins.

**Enforcement itself**: `mise run tools-test` and the `tools` CI job. The suite that proves the scheduler
works is the scheduler running the suite — §12 (1).

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** — not applicable; no `crates/**` diff.

**`DESIGN.md` generic-core / elidex-adapter split** — the live boundary here:

| Edit | Layer | Placement |
|---|---|---|
| §4.1 capability precondition, remedy text | **elidex skill** | `preflight.py` — consumes the library, adds no generic behavior |
| §4.4 assertion relocation | **elidex skill** ← moved **out of generic** | this is the layering *improvement*: it removes the generic package's only hard-coded dependency on an elidex skill's directory layout and module name |
| §4.3 script + `mise` task + CI job | **elidex repo infrastructure** | `.claude/tools/python-suites.sh`, `mise.toml`, `.github/workflows/ci.yml` — no `_webref` module is touched |

**Zero edits to `.claude/tools/_webref/**` except `test_cite_audit.py`'s halved assertion** — which is the
mechanical check that A carries no part of B: `git diff --stat -- .claude/tools/_webref/` must list that
one file and nothing else (§12).

**One-issue-one-way**, two collapses: the suite invocation goes from zero canonical sites to exactly one
(§4.3.2), and the `WEBREF.is_file()` question goes from *n*-per-citation to one precondition (§4.1.3).
One is deliberately **not** collapsed and is slotted instead: `preflight` still reaches
`resolver.lookup_section` through a subprocess while reaching `spec_labels` in-process (§11 D-1).

---

## §8 Line-count budget

`wc -l`, verified 2026-07-28 **after the rebase** (`mise.toml` is 136, not the 131 the superseded draft
recorded — #488 added the fourth trip-wire):

| File | Now | After (est.) | Note |
|---|---|---|---|
| `.claude/skills/elidex-plan-review/preflight.py` | 489 | ~515 | precondition + two remedy strings; `shortname_from_label` loses a branch |
| `.claude/skills/elidex-plan-review/test_preflight.py` | — | ~150 | new (P1-P6) |
| `.claude/tools/_webref/test_cite_audit.py` | 410 | ~395 | assertion halved, `coverage_map_label` deleted |
| `.claude/tools/python-suites.sh` | — | ~20 | new |
| `mise.toml` | 136 | ~142 | `[tasks.tools-test]` + one `depends` entry |
| `.github/workflows/ci.yml` | 126 | ~150 | `tools` filter + `tools` job |
| `.claude/tools/_webref/DESIGN.md` | 157 | ~168 | how the suites run, and the declared interpreter floor |

**1000-line touch-time check** (cohesion-based, not count-based): the largest file in the touch set is
`preflight.py` at 489 → ~515, less than half the threshold, and it is a single cohesive gate whose seam
(structure / breadth / citation / grep-pass) is already expressed as four ordered blocks in `main`.
Nothing here is near a split. `test_cite_audit.py` **shrinks**.

---

## §9 Edge-dense assessment

CLAUDE.md's trigger fires on the *program*, which is why the umbrella exists. For this slice the
**base case** applies and is the whole argument: an approved umbrella's narrowly-scoped, plan-reviewed
per-PR slice is a terminal unit, and re-splitting it for touching the same subsystem as B/C is the
infinite regress the base case exists to stop.

Checked honestly rather than asserted: J1-J3 do intersect, but all three live inside one function's
control flow (`preflight.main`) with one output (an exit code), and §5's six-row table is the complete
edge matrix — enumerable in one test file, which is the property the rule protects. J4 is independent of
them and is three files of configuration. There is no cross-crate, cross-thread, or cross-process
invariant; `git diff --stat -- crates/` is empty and stays empty.

What was separable **has** been separated, twice, and both separations were forced by a gate rather than
chosen: the detector left the content sweep (`26721cfa`), and A/B/C left each other (the umbrella).
§4.2 is the auditable statement of the second.

---

## §10 Open questions for `/elidex-plan-review`

- **Q1 — is hoisting `WEBREF.is_file()` inside A's scope?** It is the same precondition as the import
  check and leaving it per-citation keeps two shapes for one question, but it is not literally "fail
  closed" — case B already fails. **Recommendation: include it**, because A is the slice that establishes
  where a capability is checked, and a slice that establishes the rule while leaving one exception is the
  coexistence CLAUDE.md's "One issue, one way" forbids. Review should push back if the diagnostic change
  (15 lines → 1) is considered a behavior change needing its own pin — §6-P6 pins it either way.
- **Q2 — `actions/setup-python`, or the preinstalled interpreter plus an asserted floor?** Pinning is
  reproducible but encodes the version in YAML where nothing checks it; asserting makes the floor
  mechanically checked and lets the first CI run *establish* the runner's version rather than this memo
  asserting it. **Recommendation: assert the floor, do not pin.** Review should overrule if a red CI on an
  image change is considered worse than an unpinned interpreter.
- **Q3 — the two live GETs in the merge gate (§4.3.3).** Accept, or make the suite hermetic in A?
  **Recommendation: accept**, on the three measured grounds in §4.3.3 — and note that making it hermetic
  requires editing `cache.py`'s availability semantics, which is Slice B's §4.1.7. Review should decide
  whether B must therefore carry "the suites run offline" as an explicit obligation rather than a
  by-product; if yes, that is a line in the umbrella, not a new slot.
- **Q4 — `tools` path-filter breadth.** `.claude/skills/**` runs the job on a markdown-only skill edit
  (cheap, ~1 s) but is honest about the blast radius; the narrow alternative names only the two
  suite-bearing directories and silently stops covering a third. **Recommendation: the broad filter.**
- **Q5 — branch protection (§4.3.4).** The job is visible but not blocking. Recording this as a
  repository-settings follow-up is honest; adding it to `docs/` risks a claim nothing checks.
  **Recommendation: state it in the PR description and in `DESIGN.md`, register nothing** — but review
  should say if it wants a slot instead.
- **Q6 — `#11-layoutbox-trip-wire-not-in-ci` (§13).** A is a `.github/workflows` touch, which is one of
  that slot's two triggers. **Recommendation: A dissolves the slot's stated obstacle** ("CI invokes no
  `mise` task, so it needs `mise` wired into the workflow or a direct bash call") and **updates the slot
  text**, but does not wire the trip-wires: they read `crates/**` and so belong under the `rust` filter,
  and their CI verdict is taken against C-4's delete decision, which is the Layout lane's open question.
  Review should overrule if "the trigger fired, discharge it" is read as unconditional.

---

## §11 Defer slots + per-PR ≤3 audit

**One own deferral**, against a budget of ≤3 ([[feedback_defer_cap_policy]]).

| Slot | 4-question audit |
|---|---|
| **`cleanup-webref-preflight-inprocess-resolution`** (NEW) | `preflight.verify_citation` (`:240-263`) forks a Python subprocess **and** an HTTP conditional-GET per unique citation, while the same file reaches `spec_labels` in-process through the carved `sys.path` seam (`:56-58`) — two ways to reach the shared library in one file, ~15 lines from collapse. (1) Real gap? Yes, one-issue-one-way, and A's §4.1.3 hoist makes the asymmetry more visible by removing the per-citation `WEBREF.is_file()` that partially disguised it. (2) Blocked by structure? **Yes, and this is the substantive reason**: in-process resolution means `cache.py`'s `sys.exit` on network failure aborts the *whole gate* mid-run instead of failing one citation. Whether a plan-review gate should be usable offline is a policy question A does not settle and Slice B's §4.1.7 owns the mechanism for; answering it inside a fail-closed PR smuggles a second policy in. (3) Non-regressing to defer? Yes — pre-existing, and §4.1.3 makes the gate's *correctness* independent of its speed. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: Slice B's `_catalog()` availability contract landing, or a §3 table large enough that the gate's runtime is noticed. **Re-eval**: 2026-11-30. |

**Explicitly NOT deferred**, so the absence is deliberate: the fail-closed fix (§4.1), the correct siting
of it (§4.1.2 — the whole point), the remedy strings (§4.1.4), the assertion relocation (§4.4), the
script + `mise` task + CI job (§4.3), and the `WEBREF.is_file()` hoist (§4.1.3, subject to Q1).

**Slot updated, not created**: `#11-layoutbox-trip-wire-not-in-ci`'s "⚠ heavier than it looks: CI invokes
**no** `mise` task" obstacle is dissolved by §4.3.2. The slot stays open and stays the Layout lane's; only
its obstacle text changes (§13, Q6). Updating an existing slot is not an own-deferral.

**Two slots this slice does *not* register** because they are Slice B's, recorded here so the excision is
auditable rather than a loss: `cleanup-webref-agent-brief-attribution` and
`cleanup-webref-audited-set-provenance` (both in the Slice B memo's §11 — the second's deferral reason,
"changing it changes every count in the memo", is a statement about B's delta table, not A's).

---

## §12 Exit criterion

Four runnable checks. None depends on any count in this memo.

**(1) Green — the enforcement task exists and passes, from the task CI depends on:**

```sh
mise run tools-test
```

**(2) Red — every new pin detects the defect it names:**

```sh
git worktree add /tmp/citeaudit-pre 26721cfa
cp .claude/skills/elidex-plan-review/test_preflight.py /tmp/citeaudit-pre/.claude/skills/elidex-plan-review/
cd /tmp/citeaudit-pre && python3 -m unittest discover -s .claude/skills/elidex-plan-review -p 'test_*.py'
echo "EXPECT NON-ZERO: $?"
```

Must exit non-zero with at least one failure attributable to each of P2, P3, P4 and P6. **P4 is the
load-bearing one**: it must also fail against the superseded draft's patch, not merely against unpatched
HEAD, or the §4.1.2 re-siting is unpinned.

**(3) A carries no part of B:**

```sh
git diff --name-only origin/main..HEAD -- .claude/tools/_webref/ | grep -v '^\.claude/tools/_webref/test_cite_audit\.py$'
```

Must print nothing.

**(4) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page. Verified by observation on the PR, not by a local command, because it is a claim about
GitHub's event routing. Today the same observation yields zero jobs (§4.3.1).

---

## §13 Coordination

Re-derived 2026-07-28 from `git worktree list` and `git diff --name-only origin/main..<branch>`, not from
memory. Both branches in this program were rebased onto `96a8e47b` at the start of this session and
replayed clean.

| Lane | State | Overlap with A | Ordering rule |
|---|---|---|---|
| **Slice B** (new branch, from A) | not started | Total by construction — B rebases onto A and adds P4/P5-equivalents to the `test_preflight.py` A creates | **A lands first** (umbrella) |
| **Slice C** (new branch) | not started | none in A's touch set | after B |
| **PR-A0 / D** (`domform-submittable-category` @ `04a771b5`) | 19 commits, unpushed, rebased | carries the identical 8 `.claude/` files (`git diff … -- .claude/` = **0 lines**); must drop its `.claude/` half once A lands | after A/B/C |
| **Layout lane** (`#11-layoutbox-trip-wire-not-in-ci`, `elidex-wt-c4fix` / PR #491) | slot open, re-eval 2026-10-27 | **`ci.yml` — direct contention.** A's `.github/workflows` touch is one of the slot's two triggers | see below |
| **VM P4** (`elidex-wt-vmp4`, PR #489 open) | open | `mise.toml` only; `[tasks.tools-test]` is a new block | whichever lands second rebases |
| **C-3 plan** (`elidex-wt-c3-plan`) | in-flight memos | no file overlap; preflight-behavior overlap only | see the landing checklist |

**`#11-layoutbox-trip-wire-not-in-ci` disposition** (Q6): A **dissolves the obstacle** and **updates the
slot**, and does not discharge it. The slot's own text names the obstacle as "CI invokes **no** `mise`
task, so it needs `mise` wired into the workflow or a direct bash call" — §4.3.2 answers exactly that
question once, for the whole repository. What A does not take is the decision: the four trip-wires read
`crates/**`, so they belong under the `rust` filter and not A's `tools` filter, and whether their verdict
should block is entangled with C-4's delete decision, which is the Layout lane's. After A, wiring them is
a job block that mirrors A's; the slot text is edited to say so, so the Layout lane inherits the dissolved
obstacle instead of re-deriving it.

**Landing checklist** (A changes a gate every lane runs):

1. Re-run `preflight.py` on the in-flight plan-memos in `elidex-wt-c3-plan`, `elidex-wt-vmp4plan`,
   `elidex-wt-turncomp`, `elidex-wt-slice1`, `elidex-wt-c4fix` — **from each worktree's own copy**, since
   `REPO_ROOT` derives from `__file__` — and record the exit codes. Expected: unchanged, because all five
   have an intact `.claude/tools/_webref`; the point is to have measured it rather than argued it.
2. Register `cleanup-webref-preflight-inprocess-resolution` in `project_open-defer-slots.md` and edit
   `#11-layoutbox-trip-wire-not-in-ci`'s obstacle text — a memory-file write, not a chip
   ([[reference_spawn-task-chips-not-durable]]).
3. `MEMORY.md`'s L3 lane bullet names the 6-slice program; update it to A-landed / B-next.
4. PR description states §4.3.3 (the merge gate now makes two conditional GETs to
   `raw.githubusercontent.com`) and §4.3.4 (no branch protection, so the job is visible, not blocking).
