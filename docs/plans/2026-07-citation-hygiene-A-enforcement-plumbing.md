# Plan — Slice A: the shared spec-label map, landed fail-closed, with a scheduler that runs its suites

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A**. Under that umbrella's approval
boundary this is a **terminal unit** — it is not re-split for touching the same subsystem as B/C.
**Branch**: `webref-cite-audit-tool`, **after the §4.0 re-carve**. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`.
**Base**: `96a8e47b`.
**Nature**: developer tooling + **CI topology**. Zero `crates/**` diff, zero engine behavior change.
**Status**: plan-memo, **draft 2**. `/elidex-plan-review` **required before implementation**.

⚠ **Draft 1 was reviewed and failed on its own premises.** The 5-agent gate returned 2 CRIT, and
verifying them showed draft 1 measured its entire evidence base **on the branch instead of on
`origin/main`**. The consequences were not cosmetic: the defect draft 1 existed to fix does not exist on
`origin/main` at all, and the PR draft 1 described would have shipped 945 lines of Slice B. §4.0 is the
re-carve that follows; §14 records what changed and why, so the correction is auditable rather than
silent.

### §0.1 What Slice A is, in one sentence

`preflight.py` on `origin/main` carries its own hand-maintained `SPEC_LABEL_REVERSE` map — the fourth
copy of one enumeration. Slice A replaces the four copies with one shared `spec_labels.py`, **and,
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

The third fixture row is deliberately **label-less** (`| §4.10.21 Constraints | … |`, the cell opening
with `§`). It is not a citation defect — it is the input shape that falsifies the placement §4.2.2
rejects, and it carries no spec label by construction.

⚠ **This table certifies fixtures, not the slice.** `preflight` prints `citation verify: ok (2 unique
citation(s) checked)` for a slice with **zero spec surface**, because `preflight.py:296-326` hard-fails a
memo with no §3 heading and no data rows, so there is no way to declare "no spec surface" and pass.
That is the same defect class as §1's anchor, living in A's own file. It is **named and not fixed here**
— see §11's slot, because fixing it needs a verdict *type*, which is Slice B's `_catalog()` work.

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
  extracts a section number from memo cell text and `verify_citation` (`:240-263`) passes it to a
  subprocess. §4.2.2's whole finding is that memo *content* steers control flow, so listing only the path
  would omit the input that section proves is load-bearing. Not a vulnerability — argv is a list, and
  `shortname` is bounded by the `SPECS` map — but the audit must *derive* that, not assume it.
- **Exposure delta is not zero, and it is outward.** §4.3.3 measures that A moves suites which fetch from
  `raw.githubusercontent.com` from manual invocation to *every `.claude/**` PR on a GitHub runner*. Small,
  data-only, defended in §4.3.3 — but a claim of "nothing reachable from the network" would be false.

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
| `spec_labels`'s reverse index and discriminated `_catalog()` (B §4.1.2 / §4.1.7 / §4.1.8) | **B** | A lands the map's *shape*; B fixes its catalog semantics |
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

#### §4.2.3 The fix — one precondition, before the loop, and the loop must respect it

1. `verification_capability()` returns *available* or *unavailable(cause)*, evaluated **once in `main()`**
   before the row loop. The two causes are the missing CLI (`WEBREF.is_file()`) and the failed import.
   Neither is a property of any row.
2. Unavailable **and** verification requested (not `--no-verify`) → HARD FAIL, in the same
   `❌ HARD FAIL — …` shape the other hard-fails use, with the cause named and `--no-verify` named as the
   suppressor.
3. Unavailable **and** `--no-verify` → no failure (J3), and **`main` skips the per-row mapping call
   entirely**, classifying every row as *uncertified* rather than *unmapped*.

   ⚠ Draft 1 said instead that the row loop still runs and `K` still counts `unmapped:<label>` keys, while
   *also* deleting `shortname_from_label`'s `_shortname_for is None` branch as unreachable. Those are
   inconsistent: the loop calls that function at `:353` on every path including `--no-verify`, so with the
   branch gone it is `None(label)` → **`TypeError`**, breaking J3, §5 row 5 and test P3 (§14 C2). Skipping
   the call is the resolution that keeps J1 (uncertified ≠ unmapped) *and* keeps one site answering the
   capability question.
4. `shortname_from_label` therefore keeps exactly one job — `label → shortname | None`, a pure data
   classification — and its capability branch is deleted because `main` no longer calls it when the
   capability is absent, not because the state is unreachable.

**This also collapses a duplication in the other direction.** `WEBREF.is_file()` is currently re-tested
inside `verify_citation` on **every unique citation** — 15 times in case A — reporting one process-level
fact as 15 per-citation failures. After the hoist, case B's exit code is unchanged (1) and its diagnostic
is one line naming the missing path. The guard inside `verify_citation` becomes an `assert`, not a
deletion, so a future direct caller gets a contract violation rather than a silent non-zero (§10-Q1).

#### §4.2.4 The remedy text

Three strings, currently one:

- genuinely unmapped label → "add the spec to `.claude/tools/_webref/spec_labels.py::SPECS`, or check the
  label spelling"
- tools unavailable → the import error and the path it was attempted from, plus `--no-verify`
- CLI missing → the expected path, plus `--no-verify`

Slice B inherits a fourth cause (catalog unreachable, its §4.1.7). A does not pre-build a branch for it.

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

`preflight.py`'s exit code never moves from 1 to 0. The full matrix is 3 capability states × 2 verify
modes; draft 1 published six of these twelve cells and called it complete (§14 C7).

| # | Capability state | Flags | Carve as authored | After A |
|---|---|---|---|---|
| 1 | all present | default | 0 (15 verified) | **0** |
| 2 | all present | `--no-verify --no-grep-pass` | 0 | **0** |
| 3 | CLI missing | default | 1 (15 per-citation failures) | **1** — one diagnostic line |
| 4 | CLI missing | `--no-verify --no-grep-pass` | **0** | **0** — capability irrelevant when unused |
| 5 | `_webref` unimportable | default | **0** (21 unmapped, nothing verified) | **1** |
| 6 | `_webref` unimportable, §3 rows label-less | default | **0** | **1** (§4.2.2) |
| 7 | `_webref` unimportable | `--no-verify --no-grep-pass` | 0 | **0** (J3) |
| 8 | both missing | default | 1 | **1** |
| 9 | both missing | `--no-verify --no-grep-pass` | 0 | **0** |

Rows 1, 3, 5, 7 measured at the carve; row 6 measured against draft 1's patch; rows 2, 4, 8, 9 measured in
the same sandbox. **Newly-red: 5 and 6 only** — both require a broken `.claude/tools/_webref`, a state no
in-flight worktree is in. §13 still makes re-running the gate on each in-flight memo a landing item,
because a claim about six other worktrees is not one this memo can make from here.

---

## §6 Test plan

**`.claude/skills/elidex-plan-review/test_preflight.py`** (new):

- **P1** the `preflight.shortname_from_label(label) == short` derivation assertion, from
  `test_cite_audit.py:275`, with no `sys.path` mutation in the test body and a `setUp` assertion that the
  module is un-poisoned (§4.5 item 2).
- **P2** `_webref` unimportable → **exit 1**, via the `importlib.reload`-under-import-hook form (§4.5
  item 1), with `tearDown` restoring the module.
- **P2b** the same via a subprocess, so `preflight.py:56-60`'s `except Exception` is pinned rather than
  bypassed. Mutation check: deleting that clause must turn P2b red.
- **P3** `--no-verify --no-grep-pass`, tools absent → **exit 0**, **and** the summary reports *uncertified*
  rather than *unmapped* (row 7; pins §4.2.3 item 3's skip, which is where draft 1 crashed).
- **P4** tools absent **and** a fixture memo whose §3 rows carry no spec label → **exit 1** (row 6). The
  one test that distinguishes the correct siting from draft 1's.
- **P5** the tools-unavailable diagnostic does **not** say "add the spec to … `SPECS`".
- **P6** the missing CLI is reported once, not once per citation, exit code still 1 (row 3).

**`.claude/tools/_webref/test_spec_labels.py`** (new by §4.0's split): `TestSharedSpecLabelMap`'s 9 tests
minus the `preflight` assertion, plus the `coverage_map` half at module-level import. Slice B appends its
catalog round-trip cases (its S1–S5) to this file rather than creating it.

**Enforcement**: `mise run tools-test` and the `tools` CI job — §12 (1).

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** — not applicable; no `crates/**` diff.
**ECS-native** — not applicable; no component, no entity, no system.

**`DESIGN.md` generic-core / elidex-adapter split** — the live boundary:

| Edit | Layer | Placement |
|---|---|---|
| `spec_labels.py` + its `coverage_map` consumer | **generic** | a property of upstream's catalog |
| `_webref/DESIGN.md`, `spec_labels.py` bullet only | **generic** | describes a generic module; the CI facts do **not** go here (below) |
| `test_spec_labels.py` | **generic** | tests a generic module |
| §4.2 capability precondition, remedy text, `test_preflight.py` | **elidex skill** | `preflight.py` consumes the library, adds no generic behavior |
| §4.3 script + `mise` task + CI job | **elidex repo infrastructure** | `.claude/tools/python-suites.sh`, `mise.toml`, `.github/workflows/ci.yml` |

⚠ Draft 1 planned to record the `mise` task, the GitHub job, the path filter and the interpreter floor in
`_webref/DESIGN.md`. That file says of itself that the core should "stay generic enough to move to a
standalone repository later" and to "keep new generic behavior free of elidex-specific file paths" — a
section describing `mise.toml` and `.github/workflows/ci.yml` travels with the tree at externalization
and is wrong on arrival (§14 C8). Those facts live in `python-suites.sh`'s header and the `mise.toml`
task comment, which is where `trip-wires` documents itself.

**One-issue-one-way**, two collapses: the suite invocation goes from zero canonical sites to one
(§4.3.2), and the `WEBREF.is_file()` question from *n*-per-citation to one precondition (§4.2.3). One is
deliberately **not** collapsed and is slotted: `preflight` reaches `resolver.lookup_section` through a
subprocess while reaching `spec_labels` in-process (§11).

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

The **base case** applies: an approved umbrella's narrowly-scoped, plan-reviewed per-PR slice is a
terminal unit, not re-split for touching the same subsystem as B/C/D.

What makes A *safe* as one slice, stated without the completeness claim draft 1 over-reached on: J1-J3
live in one function's control flow with one output (an exit code), and §5's **nine-row matrix is the
full 3×2 capability × mode space plus the label-shape discriminator** — enumerable in one test file. J4
is independent and is three files of configuration. `git diff --stat -- crates/` is empty and stays
empty, so a regression degrades a developer tool and cannot reach a page, a script, or a user.

The ordering couplings are the umbrella's rules, not exemptions: retiring the grep requirement before the
detector is sound would mandate an under-reporting detector (C after B), and the regression pins are
unenforced until a scheduler exists (A before B).

---

## §10 Open questions for `/elidex-plan-review`

- **Q1 — `WEBREF.is_file()`: hoist and `assert`, or hoist and delete?** §4.2.3 proposes `assert`, so a
  future direct caller of `verify_citation` gets a contract violation rather than a silent non-zero.
  **Recommendation: assert.** Review should say if an `assert` in a gate that may run under `-O` is
  considered too weak, in which case it is an explicit raise.
- **Q2 — is §4.0's re-carve in A's PR, or its own commit ahead of it?** It is a history rewrite of an
  unpushed branch either way. **Recommendation: its own commit, first on A's branch**, so
  `git log --oneline` shows the seam and §12 (3) can range from it.
- **Q3 — the umbrella line for the offline obligation (§4.3.3).** A adds it to "Constraints each slice
  inherits". **Recommendation: yes, unconditionally** — draft 1's conditional phrasing ("review should
  decide whether B must carry it") is how a concern ends up unowned.
- **Q4 — `tools` path-filter breadth.** Note the measured consequence: PR #491
  (`layout-c4-classification-fix`) introduces `.claude/tools/layout-box-reader-allowlist.tsv`, so after A
  a Layout-lane allowlist regeneration triggers the Python suites — a network-touching job on a change
  unrelated to it. Narrowing to the two suite-bearing directories avoids that but silently stops covering
  a third. **Recommendation: keep the broad filter** (the job is ~1 s and one GET, and the alternative
  fails silently), and state the #491 effect in the PR description.
- **Q5 — register the `required_status_checks` follow-up?** With §4.3.4 corrected it is one rule on an
  existing active ruleset. **Recommendation: register it as a slot** (§11) rather than a PR-description
  aside, because "visible but not blocking" is a claims-table row A cannot close by itself.
- **Q6 — `#11-layoutbox-trip-wire-not-in-ci` (§13).** A is a `.github/workflows` touch, one of that slot's
  two triggers. **Recommendation: A dissolves the obstacle, updates the slot text in *both* files that
  carry it, and does not wire the trip-wires** — they read `crates/**` and so belong under the `rust`
  filter, and the work is the Layout lane's. ⚠ Draft 1 also argued "their verdict is entangled with C-4's
  delete decision"; that **inverts the Layout lane's own record**, where C-4 is the reason to wire them
  (§14 C9). The filter-placement and cross-lane-ownership grounds carry the disposition alone.

---

## §11 Defer slots + per-PR ≤3 audit

**Two own deferrals** against ≤3 ([[feedback_defer_cap_policy]]). Audited in the canonical form: the
create-time eligibility set, then the landing-time legitimacy category, then the four confirming
questions.

| Slot | Audit |
|---|---|
| **`cleanup-webref-preflight-inprocess-resolution`** (NEW) | `preflight.verify_citation` (`:240-263`) forks a subprocess **and** an HTTP conditional-GET per unique citation, while the same file reaches `spec_labels` in-process — two ways to reach the shared library in one file. **Create-time**: pragmatic-shortcut ✓ (≥1 hit ⇒ eligible). **Legitimacy**: L1 precondition-gated — in-process resolution means `cache.py`'s `sys.exit` on network failure aborts the *whole gate* mid-run, and whether a plan-review gate should be usable offline is the policy §4.3.3 hands to B. **Confirming Q2 (one-issue-one-way middle state)**: fires — §7 records the surviving duplication explicitly. Answered: the middle state is bounded by B's landing, which is the same event that answers the policy, so folding it into A would decide the policy by side effect. **Boundary note**: the collapse direction (toward in-process `resolver.lookup_section`) makes the elidex adapter depend on a generic module `DESIGN.md` does not list among its declared generic surface — that is part of the deferred decision, not a hidden cost. **Trigger**: B's `_catalog()` availability contract landing. **Re-eval**: 2026-11-30. |
| **`cleanup-elidex-ci-required-status-checks`** (NEW, §10-Q5) | The `main-protection` ruleset (id 13294991, active) has no `required_status_checks` rule, so every CI job — including the four `trip-wires` and A's new `tools` job — is advisory, blocked only by a documented human step. **Create-time**: one-way ✓ (a claims-table row A cannot close). **Legitimacy**: L1 — it is a repository-settings change, outside any diff, and it interacts with the Layout lane's trip-wire slot and with which jobs are *stable* enough to be required. **Confirming Q2**: does not fire — there is no second mechanism, only an absent one. **Trigger**: the Layout lane wiring the trip-wires into CI, or the first job stable enough to require. **Re-eval**: 2026-11-30. |

**Explicitly NOT deferred**: the re-carve (§4.0), the fail-closed precondition and its correct siting
(§4.2), the row-loop skip (§4.2.3 item 3), the remedy strings, the test relocation (§4.4), the test-siting
constraints (§4.5), the script + `mise` task + CI job (§4.3), and the umbrella's offline line (§4.3.3).

**Named, not fixed, and routed rather than slotted**: §0.5's ⚠ — `preflight` cannot express "no spec
surface", so a zero-spec slice must cite fixtures and receives `citation verify: ok` as a headline. A
verdict *type* is Slice B's `_catalog()` result-type work; A's landing note hands B the requirement. This
is recorded here so the absence is deliberate, and it is **not** counted as an own-deferral because it is
B's mechanism, not A's postponement.

---

## §12 Exit criterion

**(1) Green:**

```sh
mise run tools-test
```

**(2) Red — every new pin detects the defect it names:**

```sh
git worktree add /tmp/citeaudit-pre <the re-carve commit from §4.0>
cp .claude/skills/elidex-plan-review/test_preflight.py /tmp/citeaudit-pre/.claude/skills/elidex-plan-review/
cd /tmp/citeaudit-pre && python3 -m unittest discover -s .claude/skills/elidex-plan-review -p 'test_*.py'
echo "EXPECT NON-ZERO: $?"
```

Non-zero, with at least one failure attributable to each of P2, P2b, P3, P4 and P6. **P4 is load-bearing
and its counter-case is not runnable from this tree** — "it must also fail against draft 1's patch" was a
sandbox experiment committed nowhere (§14 C10). Made runnable: the patch ships as a fixture under
`test_preflight.py`'s test data, so P4 asserts against *both* the unfixed guard and the mis-sited one.

**(3) A carries no part of B**, ranged from the re-carve rather than from `origin/main`:

```sh
git diff --name-only <re-carve-A-commit>..HEAD -- .claude/tools/_webref/
```

Must list only files in §4.0's A column. ⚠ Draft 1 ranged this from `origin/main` and asserted it prints
nothing; at today's head it prints six files, because the detector carve is inside that range (§14 C1).

**(4) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page. Verified by observation on the PR; today the same observation yields zero jobs (§4.3.1).

---

## §13 Coordination

Re-derived 2026-07-28 with **three-dot** ranges (`origin/main...<branch>`) and `gh pr list --state open`.

| Lane | Overlap with A | Ordering rule |
|---|---|---|
| **Slice B** | total by construction — B branches from A's landed head and takes §4.0's B column; B appends to the `test_preflight.py` and `test_spec_labels.py` A creates | **A first** |
| **Slice C** | `_webref/DESIGN.md` — **same file, disjoint sections** (A: the `spec_labels.py` bullet; C: the reported-class contract). C is downstream, so the hazard is nil | after B |
| **PR-A0 / D** (`domform-submittable-category` @ `04a771b5`) | carries the identical 8 `.claude/` files (`git diff domform-submittable-category -- .claude/` = **0 lines**); must drop its `.claude/` half once A/B land | after A/B/C |
| **Layout lane — slot** (`#11-layoutbox-trip-wire-not-in-ci`) | **prospective** `ci.yml` contention: no branch in the repo currently introduces a `.github/workflows` change | see below |
| **Layout lane — PR #491** (`layout-c4-classification-fix`) | **actual**: introduces `.claude/tools/layout-box-reader-allowlist.tsv`, inside A's `tools` filter. After A, an allowlist regeneration runs the Python suites | §10-Q4; state in the PR description |
| **VM P4** (`elidex-wt-vmp4`, PR #489) | **none.** `git diff --name-only origin/main...vm-p4-slice0a \| grep -cv '^crates/script/elidex-js/'` → **0 files** (verified 2026-07-28). ⚠ Draft 1's two-dot form reported a `mise.toml` overlap that is #488's commit on `main`, not #489's (§14 C4) | none |
| **C-3 plan / turn-completion / slice1** | no file overlap; preflight-behavior overlap only | landing checklist |

**`#11-layoutbox-trip-wire-not-in-ci` disposition** (Q6): A **dissolves the obstacle** and **updates the
slot**, and does not discharge it. ⚠ The obstacle sentence ("CI invokes **no** `mise` task, so it needs
`mise` wired into the workflow or a direct bash call") is **not in `project_open-defer-slots.md`** — that
entry is one line with no obstacle text. It is in `project_inline-mod-split-owed.md:84-85`, the Layout
lane's resume pointer, whose own header says state lives there (§14 C11). Both files are edited, or the
stale sentence survives where the lane will read it. Note also that A's job runs `bash
python-suites.sh`, so "CI invokes no `mise` task" stays **literally true** after A; what A establishes is
the direct-bash-call route, one of the two options that sentence already named.

**Landing checklist**:

1. Re-run `preflight.py` on the in-flight plan-memos in `elidex-wt-c3-plan`, `elidex-wt-vmp4plan`,
   `elidex-wt-turncomp`, `elidex-wt-slice1`, `elidex-wt-c4fix` **and `elidex-wt-submittable`** (the last is
   this memo's own §4.2.1 fixture and is Slice D's branch; draft 1 omitted it) — from **each worktree's
   own copy**, since `REPO_ROOT` derives from `__file__` — and record the exit codes. Slice C's memo is
   expected red (no §3 table, by design).
2. Register the two slots in `project_open-defer-slots.md`; edit the trip-wire slot's obstacle text in
   **both** `project_open-defer-slots.md` and `project_inline-mod-split-owed.md` §B; add the offline line
   to the umbrella's "Constraints each slice inherits". Memory-file writes, not chips
   ([[reference_spawn-task-chips-not-durable]]).
3. Correct the branch SHAs the 2026-07-28 rebase invalidated: `project_citation-hygiene-program.md`
   §State (`45bd11bc` / *"2 behind"* / `d3173bed`) and
   `project_slice1-elementstate-cache-deletion-state.md:68`.
4. `MEMORY.md`'s L3 bullet still reads `▶ next = memo を A/B/C に re-slice → Slice A の plan-review` —
   **both halves are already done**, and the umbrella says this record class updates *now*, not at
   landing. Split: drop the completed "▶ next" immediately; set A-landed / B-next at landing.
5. PR description states §4.3.3 (one conditional GET to `raw.githubusercontent.com` per run), §4.3.4 (no
   `required_status_checks` rule, so the job is visible, not blocking) and §10-Q4 (the PR #491 effect).

---

## §14 What draft 1 got wrong

Recorded because the program's thesis is that a silent correction is the defect it exists to remove. Every
item was found by the `/elidex-plan-review` gate and then **independently re-derived** before being
accepted — two agent claims were escalated by that re-derivation, and one agent claim about a helper's
call count was checked and found wrong (the memo's "one caller" was correct).

| # | Draft 1 said | Measured |
|---|---|---|
| C1 | "Zero edits to `_webref/**` except `test_cite_audit.py`"; the branch-relative **83 tests across 5 files** (verified 2026-07-28) | The detector carve is inside `origin/main..HEAD` — 6 `_webref` files, ~945 lines. `origin/main`'s `preflight.py` has a **local `SPEC_LABEL_REVERSE`, no `_webref` import**, so the D2 defect is *introduced by the carve*, not pre-existing. `cite_audit.py` is absent from `origin/main`; the real baseline is **47 tests across 4 files** (derivation in §4.3.1, verified 2026-07-28). Root cause: the evidence base was measured on the branch. |
| C2 | §4.1.3 item 4: the `_shortname_for is None` branch is "no longer reachable from `main`" | `main`'s row loop calls it at `:353` on every path, including `--no-verify`. Deleting it makes the J3 must-survive path a `TypeError`. |
| C3 | "`main` has no branch protection and no required status checks" | `gh api …/branches/main/protection` is the **deprecated** endpoint. An **active** ruleset `main-protection` exists; the conclusion survives (no `required_status_checks` rule) but the premise and the follow-up's cost were both wrong. |
| C4 | §13 derived from `origin/main..<branch>` | Two-dot reports `main`'s commits as the branch's; it fabricated a `mise.toml` overlap with PR #489 (three-dot: 0). |
| C5 | The offline gap is "precisely Slice B's §4.1.7" | B's §4.1.7 makes the offline case *more* fatal; no memo and not the umbrella carried the obligation. Unowned. |
| C6 | Declare a Python 3.11 floor because Slice B will need it | A slice carrying another slice's concern, two sections after A refuses the same move for the catalog cause. A's measured need is 3.9. |
| C7 | "§5's six-row table is the complete edge matrix" | The space is 3 capability states × 2 modes (+ the label-shape discriminator). Six of nine cells were published as complete. |
| C8 | Record the CI facts in `_webref/DESIGN.md` | That file is the externalizable generic core and says so; elidex CI topology travels with it and is wrong on arrival. |
| C9 | The trip-wires are not wired because their verdict is entangled with C-4 | The Layout lane's record says the opposite — C-4 is the *reason* to wire them. |
| C10 | §12: P4 must also fail against draft 1's patch | That patch was committed nowhere; the check was runnable by no one. |
| C11 | Edit the trip-wire slot's obstacle text in `project_open-defer-slots.md` | That entry has no obstacle text; the sentence is in `project_inline-mod-split-owed.md:84-85`. |
