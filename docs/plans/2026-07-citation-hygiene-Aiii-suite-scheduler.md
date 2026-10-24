# Plan — Slice A-iii: something runs the Python suites, ungated

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md` (on PR #501's branch, not on `main`), slice
**A-iii** (the 2026-08-01 re-slice). Terminal unit under that boundary (§9).
**Order — amended 2026-10-24 (user decision D1): A-iii lands BEFORE A-ii.** The branch is off `main`, stacked
on neither A-ii nor #501. Reason: PR #510 (VM P4) cannot merge until A-iii's `tools` job exists — under the
user's 2026-09-22 decision ("option A1") a check that needs an interpreter leaves `trip-wires` for this job
(§4.2.1) — while #501, which A-ii stacks on, is blocked on items unrelated to A-iii. The umbrella's slice
order is amended **when PR #514 is reworked after #501 lands** (the umbrella reaches `main` only then); until
that trigger this memo is the record of the reorder.
**Nature**: repo infrastructure — one shell script, one `mise` task, one CI job. Zero `crates/**` diff, zero
`_webref` diff, zero gate-semantics change.
**Status**: plan-memo, **draft 2**. `/elidex-plan-review` **required before implementation**.

**This memo carries no measured digits of its own** → `rederive suites`, `suiteset`, `filters`, `ruleset`,
`budget`, `touchset`; `lanes` is author-local and excluded from `all`. The one exception is §8's wall-clock
figures, labelled as local measurements with their frame.

### §0.1 What A-iii is

A-i lands the shared map and A-ii makes the gate fail closed; **nothing runs any of the program's tests**, nor
the four suites `main` already has. A-iii gives the Python suites a scheduler, and nothing else.

---

## §3. Spec coverage map

A-iii has no spec surface. `main`'s preflight hard-fails a memo whose §3 has no table or no `§<number>` row,
and the `No spec surface` declaration that would replace this is A-ii's (§4.2.5 there), which lands after
A-iii (§0). So, per draft 1's own fallback, this section carries the program's two-row fixture table (the
`labelled.md` rows). **They cite nothing A-iii implements**: the `citation verify: ok` the gate prints on this
memo is a statement about these two rows, not about A-iii.

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | fixture | none — A-iii has no spec surface | none | n/a | no |
| WHATWG HTML §4.10.21.2 Constraint validation | fixture | none — A-iii has no spec surface | none | n/a | no |

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** The inputs are a CI trigger and the tracked file set.

**Discovery method.** Measured against `origin/main`; the CI facts against the live ruleset via `gh`; the
cross-lane facts against the branches that contend, not against a recorded list.

---

## §1 Ideal anchor — a check nobody runs is a claim, not a check

`origin/main` carries 4 Python test files under `.claude/`, and **no `mise` task, no CI job and no hook runs
any of them**. The umbrella's constraint — *a claim is admissible only if something mechanically checks it* —
is unmet for the tooling this program is building. → `rederive suites`

**And a scheduler that can be silently skipped is the same defect with an extra step.** That is why A-iii's
job is ungated (§4.2).

---

## §2 Coupled invariants

- **L1 — one script, two callers.** If `mise` and `ci.yml` each spell the suite invocation, a later suite is
  added to one and not the other.
- **L2 — no suite is silently uncollected.** Every member of `git ls-files ':(glob).claude/**/test_*.py'` is
  run.
- **L3 — the trigger cannot be silently narrowed.** No path filter: see §4.2.
- **L4 — A-iii adds no network dependency.** The suites must run green offline.

| pair | intersection |
|---|---|
| L1 × L2 | the set and every loud failure belong to the **script**, not to either caller — otherwise they are things one caller can skip |
| L1 × L3 | with no filter, the CI caller's trigger is unconditional, so the two callers differ only in *where* they run, never in *whether* |
| L2 × L3 | the set L2 ranges over is a repo fact, not a filter list — and under D2 it is also the set the script **runs**, so L2 holds by construction (§4.2) |
| L3 × L4 | an ungated job runs on every PR, so a network-dependent suite would fail every unrelated PR |

---

## §4 The edit set

### §4.1 The hole

`ci.yml`'s `changes` filter has two sets, `rust` and `config`; **`.claude/**` is in neither**, and `check` /
`doc` / `deny` are each gated on one of them. **Every `mise` token in `ci.yml` is the path-filter entry
`mise.toml`** — measured over the whole file text, comment lines skipped; an invocation through a variable or
built at runtime is the block's stated limit (Codex R17/R25). The two ungated jobs are `Detect changes` (the
filter itself) and `trip-wires` (PR #496, landed): `trip-wires` runs `scripts/trip-wires.sh`, which runs no
Python suite. `codeql.yml` has no `pull_request` trigger; `audit.yml` is `cargo audit` on a cron. ⇒ **a
`.claude/**`-only pull request runs no job that runs the Python suites.** → `rederive filters`, `rederive
suites`

### §4.2 The mechanism — one script, two callers, no filter

`scripts/python-suites.sh` (where `main` puts CI-invoked drivers: `ci-sweep.sh`, `doc-changed.sh`,
`trip-wires.sh`), `set -euo pipefail`. `mise.toml` gains `[tasks.tools-test]` running it, added to
`[tasks.ci].depends`; `ci.yml` gains a `tools` job that is **deliberately ungated** — no `needs:`, no `if:`, no
path-filter entry — with a two-leg `python-version` matrix consumed by `actions/setup-python` (§4.4).

**The procedure (decision D2 — one way for a suite to enter):**

1. **The set** is `git ls-files ':(glob).claude/**/test_*.py'` — tracked files only, the `:(glob)` spelling so
   a file directly under `.claude/` would count without leaning on `.gitignore:28` (Codex R45/R48). An empty
   set fails loudly.
2. **The roots** are the set's unique dirnames, derived on every run. There is no fixed root list to forget to
   extend.
3. **Each root runs in its own interpreter, non-recursively**: it loads exactly the set members whose dirname
   is that root, as top-level modules from an explicit list — the loader never walks a directory. Why: `_webref`
   is a package, so a recursive `discover` rooted at `.claude/tools` collects `_webref`'s suites under a second
   import name (`rederive suiteset` reproduces it), and `.claude/tools` becomes a root the day a `test_*.py`
   lands directly in it. One process per root also keeps two roots' same-named modules apart.
4. **A root that runs zero tests fails loudly**, naming the root. This replaces draft 1's
   "uncollected-by-either-root" check, which D2 makes unrepresentable: every member is in the set the script
   runs, so the residual failure is a member that is *collected* and runs nothing.
5. The script asserts `sys.version_info >= (3, 9)` before loading anything and echoes `python3 -VV` (§4.4).

The script's header comment carries the procedure; `mise.toml`'s task comment points at it (L1).

**Entering the set from a non-`discover`-shaped self-test — the contract.** A self-test that is a command
rather than a unittest module (PR #510's `python3 .claude/tools/plan-memo-umbrella-check.py --self-test
--mutants`) enters as a thin tracked `test_*.py` beside it, which: runs the command through `sys.executable`
(so the matrix leg's interpreter is the one measured); asserts exit 0 **and** nonzero control and mutant
counts as the self-test reports them (a self-test that ran no control is not a pass — step 4, one level down);
and stays offline (L4). **The wrapper is #510's to write, not A-iii's** (the #519 boundary rule); A-iii defines
only this contract. Under A1 the same PR removes that self-test from `trip-wires`. A second known candidate
fitting the same contract: the stale-claim lane's `claim_gate_selftest.py` (`refs/backup/stale-claim-detector`,
parked) — no action here.

#### §4.2.1 The job boundary — stated as a property

PR #501's `trip-wires` comment retires two enumerations ("grep-only"; "the shell, `git` and `grep`") for a
property: *the job has NO SETUP STEP*. A list of tools is not what A-iii's boundary needs either, and a
"no setup step" criterion alone would admit a wire calling the runner's preinstalled `python3`. The
complementary rule, stated in the `tools` job's comment (A-iii does not edit #501's text):

- **`trip-wires` declares no interpreter** — no setup step.
- **`tools` declares its interpreter** — `actions/setup-python`, on every leg.
- **A check whose correctness depends on an interpreter belongs to the job that declares one.**

#### §4.2.2 Why no filter

Earlier drafts of the merged Slice A specified a `tools` path-filter set. A-iii drops it, on #496's argument:
gating would make the tamper path of a gate an allowlist entry someone must keep current — a PR editing only
a suite's input would skip the job that reads it (the failure this slice exists to remove); the filter buys
nothing, since only Rust-toolchain jobs pay for a filter; and #496 already answered the same question for
`trip-wires` (one issue, one way). **Accepted behaviour delta**: every pull request, a Dependabot bump
included, runs the suites (§5 row 2).

### §4.3 The network question — answered by construction

Measured: **0 `urlopen` calls** in-process across `origin/main`'s suites, and none of them imports
`subprocess` or names the `webref` entry script, so no child can fetch on their behalf. → `rederive suites`

⚠ **What A-iii does *not* claim**: that the gate becomes offline-capable. `verify_citation` shells out to
`webref heading`, which issues a conditional GET, so **`origin/main`'s gate requires the network in default
mode**. A-iii's claim is exact: *the suites run green offline* (T-net).

⚠ **The umbrella's earlier record** — that wiring the suites into CI takes a live-network dependency — was
measured on a branch whose catalog fall-through (Slice **B**'s) was the thing fetching. B ships the offline
contract for what B introduces; A-iii inherits no such dependency.

### §4.4 The interpreter floor — measured by one leg, stated by one assertion

The floor is **3.9**, and the **only measurement of it is the `"3.9"` matrix leg** running the suites. Draft 1
also pre-checked it statically (an `ast` read for PEP 604 unions and 3.10+ runtime names); that read is
dropped. It cannot enumerate newer-runtime APIs (Codex R21), and on `main` it reports a false positive —
`plan-xcheck.py`'s `flip | nonflip` is a set union of two lowercase names, read as a PEP 604 union. Two
instruments for one property, one of them wrong in both directions, is the shape *one issue, one way* removes.
The script's `sys.version_info >= (3, 9)` assertion and the job's `python3 -VV` echo remain the *statement*.

Both legs go through `setup-python` (§4.2.1). `--mutants`-shaped self-tests run on **both** legs, with no
per-leg branching (D3): a check that skipped the floor leg would leave the floor unmeasured for exactly the
heaviest code. Slice B raises the floor when B lands `(?>...)`; the leg's `python-version` moves with it.
`SKILL.md`'s Step 0 invokes `preflight.py` directly, bypassing the script — unaffected today, UNCHECKED in §6.

⚠ Not measured by this memo: that `setup-python` provides `"3.9"` on `ubuntu-latest`. The implementation PR's
first run is that measurement; if it cannot, the floor itself is the question, not the leg (§10 Q3).

### §4.5 What "enforced" can honestly mean here

`main` is governed by an **active** ruleset whose rules are `deletion` / `non_fast_forward` /
`pull_request`. There is **no `required_status_checks` rule**, so a red `tools` job does not block a merge;
CLAUDE.md's "CI 全 pass を目視確認してから squash merge" is the blocking step, and it is human. (The 404 from
`…/branches/main/protection` is the deprecated legacy endpoint, not "unprotected".) The claim A-iii may make:
the job makes a regression **visible, attributed, and on the PR page at review time**. → `rederive ruleset`

---

## §5 Behaviour deltas

| # | Input | `origin/main` | After A-iii |
|---|---|---|---|
| 1 | a `.claude/**`-only PR | `Detect changes` and `trip-wires` only; no suite runs | + the `tools` job |
| 2 | a `crates/**`-only PR | `check`/`doc`/`deny` + `trip-wires` | + the `tools` job (ungated) |
| 3 | `mise run ci` locally | does not run the Python suites | runs them via `[tasks.tools-test]` |
| 4 | a new tracked `test_*.py` anywhere under `.claude/` | runs nowhere | runs; a failure is red |
| 5 | a tracked `test_*.py` that loads no test, alone in its directory | runs nowhere | **script fails loudly** naming the root |
| 6 | the suites, offline | n/a — nothing runs them | green (L4) |

**Newly-red**: 4 (a failing suite) and 5 — A-iii adds a job; it does not change what any existing job asserts.

---

## §6 Pins

Every pin is a test A-iii ships; "Red at main" says whether it fails on `origin/main`, the base.

| Pin | What it executes | §5 rows | Red at main? |
|---|---|---|---|
| **Q1** | `bash scripts/python-suites.sh` at HEAD exits 0, and its output names **every derived root** with a nonzero test count — the roots compared against the dirnames of `git ls-files ':(glob).claude/**/test_*.py'`, not against a list | 3, 4 | **yes** — the script does not exist |
| **Q2** | a **failing** `test_planted.py` at `.claude/skills/elidex-review/` (no suite there today; staged `git add -N`, since the set is tracked files) makes the script exit non-zero naming it — a failing plant, because a passing one would also pass by being skipped | 4 | **yes** |
| **Q3** | a `test_empty.py` defining no test, alone in a new directory under `.claude/`, makes the script exit non-zero naming that root; and the script run in a scratch repository with no tracked `test_*.py` exits non-zero (the empty set) | 5 | **yes** |
| **Q9** | non-recursion: with a passing `test_planted.py` staged directly under `.claude/tools/` (which makes `.claude/tools` a root), every `_webref` test id appears **exactly once** in the run, and `_webref`'s count equals Q1's | 4 | **yes** |
| **Q4** | `mise run ci` reaches `tools-test` (the `depends` edge exists) | 3 | **yes** |
| **Q5** | the floor assertion: the script exits non-zero naming the floor when the `python3` it finds reports a version below 3.9 (fixture: a PATH-shadowing `python3` shim) | — | **yes** |
| **Q6** | `ci.yml`'s `tools` job region — read with the job-boundary rule `rederive filters` uses (`_job_region`: from `  tools:` to the next two-space-indented non-comment, non-list line; restated in the test, which cannot source the harness) — contains a **`run:` step that executes the driver**: after shell word-splitting, some command line of a `run:` scalar has an argv token resolving to `scripts/python-suites.sh`, or is `mise run tools-test`. The token *anywhere* in the region is not enough: `env: DRIVER: scripts/python-suites.sh` runs nothing (Codex R48). Without it an empty `tools` job satisfies §12(3) (Codex R14/R36) | 1, 2 | **yes** — the job does not exist |
| **Q7** | the same region carries a `python-version:` matrix with a `"3.9"` leg **and a `uses: actions/setup-python` step whose `python-version:` input is `${{ matrix.python-version }}`, before Q6's `run:` step** — a matrix no step consumes runs one interpreter on every leg (Codex R48), and a `setup-python` after the driver step measures nothing (Codex R49) | — | **yes** |
| **Q8** | the `tools` job is **present and ungated**: its region is non-empty and carries no `needs:` and no `if:` line — the predicate `rederive filters` states for `trip-wires`, restated in the test; deliberately *not* `_gated`'s complement, which would read a job gated on a *new* `.claude` output as ungated. **Negative control**: the same predicate reads `check` (`needs: changes` + `if:` on `rust`) as gated (Codex R49) | 2 | **yes** |
| **T-net** | `bash scripts/python-suites.sh` runs green in a child whose `XDG_CACHE_HOME` is a fresh empty directory, with `http_proxy`/`https_proxy` at a closed port and `NO_PROXY`/`no_proxy` unset (an inherited `NO_PROXY=*` bypasses the poisoned proxy); **and**, in-process across the suite set, `urllib.request.urlopen` is never called and no `subprocess` call spawns the `webref` entry script — the in-process half catches a lookup the proxy alone does not (Codex R16); `webref`, not every spawn, because a §4.2 wrapper legitimately spawns its check | 6 | **yes** |

⚠ **UNCHECKED, marked not omitted**: that a red `tools` job **blocks** a merge — **false**, §4.5; the
interpreter floor on `SKILL.md`'s direct `preflight.py` path, which bypasses the script.

⚠ **Q2's plant site is chosen to discriminate.** A plant inside a directory that already has suites would pass
under a fixed-root design too; `.claude/skills/elidex-review/` is reached only because the roots are derived.

---

## §7 Layering check

**VM host/ / ECS-native** — not applicable.

**Generic core vs elidex adapter.** A-iii touches **no `_webref` file and no `test_*.py`** — `scripts/`,
`mise.toml`, `.github/workflows/ci.yml`, and this memo with its harness. → `rederive touchset`

The `mise` task, the CI job and the floor are recorded in `python-suites.sh`'s header and the `mise.toml` task
comment, not in `_webref/DESIGN.md`: that file's core is meant to move to a standalone repository, where a
section about elidex's `ci.yml` is wrong on arrival.

**One-issue-one-way**: the suite invocation goes from **zero** canonical sites to one, and a suite enters it
one way — by being a tracked `test_*.py` (§4.2).

---

## §8 Budget — lines and wall clock

**Lines** → `rederive budget`. One new script; `mise.toml` and `ci.yml` gain a task and a job. Nothing near a
split.

**Wall clock.** The legs run in parallel, so the job's wall clock is the slower leg's. `timeout-minutes` is
derived from the implementation PR's own CI run, and **A-iii adopts #501's rule** (its `trip-wires` comment):
*a PR that adds suite cost re-derives the timeout in the same PR* — first of all #510's wrapper. `ci.yml`
carries the rule and the command to re-derive, **no runtime figure**.

*Local measurements — claims, machine-dependent, for sizing only:* PR #510 at `06926607`,
`plan-memo-umbrella-check.py --self-test` 8.7 s (Python 3.14.7) / 23.6 s (Python 3.9.6); with `--mutants`
53 s / 131 s. The 3.9 leg is the slower one, by the ratio these imply on that machine.

---

## §9 Edge-dense assessment

**(i)** The umbrella (on #501) names A-iii and its scope; the order change is §0's, carried into the umbrella
at #514's rework.

**(ii)** L1–L4 are **configuration** invariants with one observable (does the job run, and is it green). This
is the least dense of the three A slices and would not trip the edge-dense trigger on its own; it is a
separate slice because it was **invisible** inside the merged memo — the one cross-lane collision that memo
missed lived here.

---

## §10 Open questions

- **Q1 — does `required_status_checks` belong in this PR?** The `pull_request` rule carries
  `required_approving_review_count: 0` **and** a `RepositoryRole` bypass with `bypass_mode: always`, so adding
  it leaves it author-bypassable: visibility-plus-friction, not enforcement. **Recommendation: register a slot,
  do not implement** (§11).
- **Q2 — the zero-test check's grain.** D2 fixes it per root (§4.2 step 4). A file that loads no test beside
  siblings that do still passes. Per-file is one counter more in the same loader. **Not settled here.**
- **Q3 — the non-floor leg's `python-version` value.** §4.2.1 requires both legs to declare their interpreter,
  so draft 1's "the runner's `python3`" needs a value: `"3.x"` (newest the action resolves) or the runner
  image's version pinned. And if `setup-python` cannot serve `"3.9"` on `ubuntu-latest` (§4.4), whether the
  floor rises or the runner is pinned. **Not settled here.**

---

## §11 Defer slots + per-PR ≤3 audit

**Zero own deferrals.**

**Registered here because A-iii adds a CI job**: **`#11-elidex-ci-required-status-checks`** — the ruleset has
no `required_status_checks` rule, so every CI job is advisory. A-iii neither creates nor worsens it. **Why
deferred**: the rule alone is author-bypassable given the bypass actor, so it is not a one-line fix.
**Trigger**: the first job stable enough to require, or the bypass actor being removed. **Re-eval**:
2026-11-30. **Confidence**: Medium. It is in no ledger today; §13's checklist registers it.

---

## §12 Exit criterion

**(1) Green:** `mise run tools-test` and `bash scripts/python-suites.sh`, on both legs.

**(2) Red at main:** every §6 pin whose last column says "yes". No second list.

**(3) The job fires on every PR class:** Q8 green (the `check` control read as gated by the same predicate),
and a `.claude/**`-only commit shows the `tools` job on the PR page. Stated as a positive observation.

**(4) Offline:** T-net.

**(5) The job runs the driver:** Q6. (3) alone is satisfiable by a job that runs nothing.

**(6) The floor is measured:** Q7 green and the `"3.9"` leg green on the PR page — the one measurement (§4.4).

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** (on #501 / #514) | none in A-iii's files | **A-iii first** (§0, D1); A-ii rebases, and its §4.2.5 declaration can then replace §3's fixture table |
| **[PR #510](https://github.com/send/elidex/pull/510)** (VM P4) | `ci.yml`, `mise.toml`, `scripts/trip-wires.sh` | **A-iii first**; #510 then adds its §4.2 wrapper, drops its self-test from `trip-wires` (A1), and re-derives `tools`' timeout (§8) |
| **[PR #501](https://github.com/send/elidex/pull/501)** | `ci.yml` (the `trip-wires` comment and timeout); adds `test_spec_labels.py`, which §4.2 picks up with no edit | either order; the umbrella amendment rides #514's rework after #501 lands (§0) |
| **[PR #527](https://github.com/send/elidex/pull/527)**, **[#381](https://github.com/send/elidex/pull/381)**, **[#502](https://github.com/send/elidex/pull/502)** | `ci.yml` (#527 also `scripts/trip-wires.sh`; the two Dependabot bumps `steps:` only) | whichever lands second adapts textually |
| **Slices B / C** | after A-iii these are scheduled from their first commit | after |

⚠ **The lane facts are derived, not listed** → `rederive lanes`, which ranges over the files A-iii contends on
(`.github/workflows/`, `mise.toml`, `scripts/`, every `.claude/**/test_*.py`) in committed ranges and working
trees, not over `docs/plans/` or `gh pr list` alone — the shape that hid #496 from the merged memo.

**Landing checklist**

1. Register `#11-elidex-ci-required-status-checks` in `project_open-defer-slots.md`.
2. Update `project_citation-hygiene-program.md` (order: A-iii before A-ii).
3. PR description: §4.2's ungated decision and D2's derived roots; §4.2.1's boundary; §4.5's honest
   "enforced".

---

## §14 Provenance

- Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` §4.3 (drafts 1–9): filter hole (R1),
  `trip-wires` precedent (R6), two-caller invariant, `git ls-files` scoping of L2 (R8), honest enforcement
  (R5), ungated decision and `scripts/` siting (R9).
- Draft 1 = PR #514 head `cfcb4ae6`, carried verbatim as this branch's first commit.
- Draft 2 (2026-10-24): D1 reorder (§0, §13); D2 derived non-recursive roots + wrapper contract (§2, §4.2,
  §5, §6 Q1–Q3/Q9) replacing the two fixed `discover` roots and the uncollected-suite check; D3 both legs run
  `--mutants`, timeout rule adopted from #501 (§4.4, §8); A1 boundary as a property (§4.2.1); static floor
  pre-check and `rederive floor` dropped (§4.4); #496 landed and `#11-layoutbox-trip-wire-not-in-ci` closed
  2026-09-07 (§4.1, §5, §11, §13); §3 switched to the fixture table main's preflight accepts.

---

## §15 Re-derivation

`docs/plans/2026-07-citation-hygiene-A-rederive.sh`. Blocks A-iii cites: `suites suiteset filters ruleset
budget touchset` (all on `all`, plus `selfcheck`) and `lanes` (author-local, excluded from `all`).
`REDERIVE_PY=<interpreter>` runs `suites` under another interpreter — the local stand-in for the floor leg.
