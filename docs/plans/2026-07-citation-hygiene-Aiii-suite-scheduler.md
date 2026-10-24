# Plan — Slice A-iii: something runs the Python suites, ungated

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md` (on PR #501's branch, not on `main`), slice
**A-iii** (the 2026-08-01 re-slice). Terminal unit under that boundary (§9).
**Order — amended 2026-10-24 (user decision D1): A-iii lands BEFORE A-ii.** The branch is off `main`, stacked
on neither A-ii nor #501. Reason: PR #510 (VM P4) cannot merge until A-iii's `tools` job exists (under the
user's 2026-09-22 "option A1", a check that needs an interpreter leaves `trip-wires` for this job, §4.2.1),
while #501, which A-ii stacks on, is blocked on items unrelated to A-iii. The umbrella's slice order and A-ii
memo's §13 row ("A-iii | after A-ii") are amended **in this lane's rework of PR #514**; until then this memo
is the record of the reorder. ⚠ If #501 lands first, `main` carries the umbrella's stale order for the
interval — accepted because its close (the #514 rework, this lane's next action after #501 lands) is owned
by this lane.
**Nature**: repo infrastructure — one driver script and its pins, one `mise` task, one CI job, and the project
docs that enumerate `mise run ci` and the CI jobs. Zero `crates/**` diff, zero `_webref` diff, zero
gate-semantics change.
**Status**: plan-memo draft 6 — **implemented in this PR** (`scripts/python-suites.sh`, `scripts/test_python_suites.py`,
`mise.toml`, `ci.yml`, §4.6's docs), with the ten first-fix items the user carried from the draft-6 re-check
into the implementation applied below (2026-10-24).

**This memo carries no measurements and no harness.** The program's rule (2026-08-23): a plan memo holds
neither measured digits nor a self-measuring apparatus; what needs measuring is measured by a test or a CI
job — here, A-iii's own. So every claim below is either a **§6 pin** or a fact stated **with the one-line
command that reproduces it at `origin/main`**. The one exception is §8's wall-clock figures, labelled as local
measurements with their frame.

**Altitude (draft 6).** The memo states **properties**; the implementation PR chooses the mechanism and
proves each property with a pin in `scripts/test_python_suites.py`. Mechanism detail earlier drafts wrote in
prose (the text readers' grammar, the guard's internals, the list of guarded functions) is not specified
here: each such detail is a row of **§6.1 Property → pin**, which is what the pre-push review and Codex check
the implementation against, row by row.

### §0.1 What A-iii is

A-i lands the shared map and A-ii makes the gate fail closed; **nothing runs any of the program's tests**, nor
the suites `main` already has. A-iii gives the Python suites a scheduler, and nothing else.

---

## §3. Spec coverage map

**No spec surface** — A-iii touches no spec-defined behaviour: a shell script, its pins, a `mise` task, a CI
job, and the docs that enumerate them.

⚠ **`main`'s preflight exits 1 on this memo, by design.** `main`'s preflight hard-fails a §3 with no table;
the declaration above is the form recognised by the marker recognition A-ii §4.2.5 adds, and A-ii lands after
A-iii (§0). This is the program's recorded position, not a local exception — the umbrella on #501 records
"A-iii … `preflight` EXIT **1 is by design**" (its slice-memo status table), C declares **No spec surface**
"exactly as A-iii does — not fixture rows" (C memo on #514, §3 item 4), and A-i-wire (merged into #501 by
#519) does the same. Fixture rows would make the gate print `citation verify: ok` for behaviour A-iii does not
implement — the fabricated-citation shape §4.2.5 exists to retire. When A-ii lands, preflight recognises this
declaration with no edit here.

**Preflight stops there**, before citation verify (vacuous here: no citations) and grep-pass, so its exit 1
says nothing about grep-pass. The memo's acceptance is preflight's exit 1 with exactly the no-table hard fail
**and** grep-pass, run directly from the repository root with both strict flags, printing **nothing** (no
HARD and no SOFT finding; its reach on this memo: counted-enumeration claims only — it checks Rust source
paths under `crates/` and Rust `::` symbols, of which the memo names none):

```sh
python3 -c 'import sys; sys.path.insert(0, ".claude/skills/elidex-plan-review"); from pathlib import Path; from grep_pass import run_grep_pass; f = run_grep_pass(Path(sys.argv[1]), Path("."), strict_enum=True, strict_symbols=True); [print(s, m) for s, m in f]; sys.exit(bool(f))' docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md
```

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** The inputs are a CI trigger and the tracked file set.

**Discovery method.** Repo facts by `git` commands against `origin/main`, each written beside its claim; CI
facts against the live ruleset via `gh`; cross-lane facts by §13's sweep over every open PR's diff, not over a
recorded list.

---

## §1 Ideal anchor — a check nobody runs is a claim, not a check

`origin/main` carries Python test files (`git ls-files ':(glob)**/test_*.py'` — all of them under `.claude/`
today), and **no `mise` task, no CI job and no repo script runs any of them**:
`git grep -nE 'unittest|pytest|test_[a-z_]*\.py|python' origin/main -- mise.toml .github scripts` matches
nothing, and the repo carries no hook (`git ls-tree --name-only origin/main .claude/` is `skills` and `tools`).
The umbrella's constraint — *a claim is admissible only if something mechanically checks it* — is unmet for
the tooling this program is building.

**And a scheduler that can be silently skipped is the same defect with an extra step.** That is why A-iii's
job is ungated (§4.2).

---

## §2 Coupled invariants

- **L1 — one script, two callers.** If `mise` and `ci.yml` each spell the suite invocation, a later suite is
  added to one and not the other.
- **L2 — no suite is silently uncollected.** Every member of `git ls-files ':(glob)**/test_*.py'` (repo-wide)
  is run and contributes at least one test; a tracked non-member `.py` that is a suite by property (a
  `TestCase`, step 3(a)) fails the run whether or not a member names it, and one that is an entry-point
  self-test (step 3(b)) fails it unless a member names it by path (§4.2 step 3).
- **L3 — the trigger cannot be silently narrowed.** No job-level gate and no workflow-level path filter: the
  job runs on every pull request targeting `main` (§4.2.2).
- **L4 — A-iii adds no network dependency.** The script runs every member under an offline regime: network
  access through Python's socket layer — connection and name resolution — fails in the loader and in every
  child outside the **unguarded-child class**, and those children see the regime's environment; a green run
  is the offline measurement **for that coverage**, and the class — with the socket-layer paths T-net does not
  plant — is the stated limit (§4.3, defined once there).
- **L5 — the interpreter floor is 3.9, measured by the `"3.9"` leg** running the whole set; the script's
  assertion is the statement (§4.4).
- **L6 — the job boundary.** `trip-wires` declares no interpreter; `tools` declares one; a check that needs a
  language runtime goes to the job that declares it (§4.2.1).

| pair | intersection |
|---|---|
| L1 × L2 | the set and every loud failure belong to the **script**, not to either caller — otherwise they are things one caller can skip |
| L1 × L3 | with no gate, the CI caller's trigger is unconditional, so the two callers differ only in *where* they run, never in *whether* |
| L1 × L4 | the offline regime is set by the script, so a local `mise run ci` is no more permissive than CI |
| L1 × L5 | the floor assertion is the script's, so both callers state it; only the CI caller *measures* it (the local interpreter is whatever is installed) |
| L1 × L6 | the `tools` job's one driving step is the script (Q6), so a check moving in under L6 enters the script's set — the job has no second invocation site to add it to |
| L2 × L3 | the set L2 ranges over is a repo fact, not a filter list — and under D2 it is also the set the script **runs**, so L2 holds by construction |
| L2 × L4 | every member, a wrapper included, runs under the regime, and a wrapper's `sys.executable` child is outside the unguarded-child class because the contract lets the wrapper change only non-regime keys (§4.2); a child in the class (§4.3) is outside the measurement, which is why the contract also says "stays offline" — and a test that skips is red (step 5), so skipping is no way out of the regime |
| L2 × L5 | every member runs on both legs; a member that needs more than 3.9 is red on the floor leg — a `skipIf` on the version included, because a skipped test is red (step 5) — and step 3 parses every tracked `.py` with the leg's interpreter, so an unparsable file is red too |
| L2 × L6 | a check leaving `trip-wires` under L6 enters through L2's one door (a tracked `test_*.py`, a wrapper if command-shaped), and step 3(b) makes that door mandatory for a `--self-test` command |
| L3 × L4 | an ungated job runs on every pull request targeting `main`, so a network-dependent suite would fail every unrelated one |
| L3 × L5 | both legs fire on every such pull request, so the floor is measured on every one, not only those touching `.claude/` |
| L3 × L6 | moving a check across the boundary must not narrow *when* it runs; both jobs are ungated (`trip-wires` by #496, `tools` by Q8), so it does not |
| L4 × L5 | they meet at `setup-python`: the interpreter is fetched by the setup step, before the script applies the regime — that network use is the action's, not a suite's |
| L4 × L6 | the regime is a property of the `tools` job's script only; a check staying in `trip-wires` does not acquire it and the memo does not claim it does |
| L5 × L6 | the floor is declared by the job that declares the interpreter; `trip-wires` has no floor because it declares none (L6's first clause) |

---

## §4 The edit set

### §4.1 The hole

`ci.yml`'s `changes` filter has two sets, `rust` and `config`, and **`.claude/**` is in neither**
(`git show origin/main:.github/workflows/ci.yml`, the `filters:` block). `check` / `doc` / `deny` are each gated
on one of them: `git grep -nE '^[[:space:]]+(needs|if):' origin/main -- .github/workflows/ci.yml` shows
`needs: changes` plus an `if:` on `rust` or `config` for exactly those three jobs (the one other `if:` is a
step's `runner.os`). The two ungated jobs are `changes` (the filter itself) and `trip-wires` (PR #496,
landed), which runs `scripts/trip-wires.sh` — no Python suite (§1's `git grep`). The only non-comment `mise`
token in the workflows is the filter entry `'mise.toml'` (`git grep -n mise origin/main -- .github/workflows/`).
`codeql.yml` triggers on `push` to `main` and a `schedule`, `audit.yml` on a cron
(`git show origin/main:.github/workflows/codeql.yml`). ⇒ **a `.claude/**`-only pull request runs no job that
runs the Python suites.**

### §4.2 The mechanism — one script, two callers, no filter

`scripts/python-suites.sh` (where `main` puts CI-invoked drivers: `ci-sweep.sh`, `doc-changed.sh`,
`trip-wires.sh`), `set -euo pipefail`; its Python half (loader, counts, complement checks) is a heredoc in
the same file, so the procedure has one home. `mise.toml` gains `[tasks.tools-test]` running it, added to
`[tasks.ci].depends`; `ci.yml` gains a `tools` job that is **deliberately ungated** — no `needs:`, no `if:`, no
path-filter entry — with a two-leg `python-version` matrix consumed by `actions/setup-python` (§4.4). A-iii's
pins ship beside the driver as `scripts/test_python_suites.py` (§6), which is therefore itself a set member and
runs on every pull request targeting `main`.

**The procedure (decision D2 — one way for a suite to enter):**

1. **The floor.** Assert `sys.version_info >= (3, 9)` before anything else, and echo `python3 -VV` (§4.4).
2. **The set** is `git ls-files ':(glob)**/test_*.py'` — tracked files, repo-wide. The `:(glob)` spelling is
   what makes a top-level `test_x.py` count: plain `'**/test_*.py'` does not match a file at the root (in a
   scratch repository, `git ls-files '**/test_*.py'` vs `git ls-files ':(glob)**/test_*.py'`). An empty set
   fails loudly.
3. **The complement, by property** — the set is defined by a name, so the script also checks the two
   properties a suite can have without the name, over every tracked `.py` **that is not a set member**
   (members are run by the loader, steps 4–5; checking them here would make every member — a wrapper, the
   pins file — its own complement):
   **(a)** a non-member defining a class whose base resolves, **within that file**, to
   `unittest.TestCase` or `unittest.IsolatedAsyncioTestCase` fails, naming it (rows P1–P2);
   **(b)** a non-member **entry-point self-test** — a top-level `__main__` guard **and** the constant
   `'--self-test'`, both read syntactically, never by a string grep — must be **named by path** in some set
   member, else the run fails naming it: the wrapper contract below, made checkable (rows P3–P6). *Named by
   path*: one string constant equal to the entry point's path relative to the repository root or to the
   member's own directory; a bare filename elsewhere, or a path assembled from several constants, is not a
   name. **3(b) checks the naming clause only**; the contract's other clauses are the wrapper's own test body,
   reviewed in the wrapper's PR. The population is fixed by property: a helper that merely carries
   `--self-test` as text is not in it (on #510's head, 18 of the 19 files containing the literal —
   `git grep -l -e '--self-test' FETCH_HEAD -- '*.py' | wc -l` → 19, verified 2026-10-24).
   **`--self-test` is the contract's normative spelling**; a `--selftest` spelling or a guard-less
   `__main__.py` is non-conforming, decided by the contract, not chased by the predicate. On #510's head
   (`a322b845`) the predicate (§13's command, `n=510`) selects exactly
   `.claude/tools/plan-memo-umbrella-check.py`. A tracked `.py` — member or not — that does not parse under
   the running interpreter fails, naming it: an unreadable file could hide a suite (row P7). **Limits,
   stated:** a base reaching `TestCase` only through another file (`from base import MyCase`) is outside
   (a); there is no count ratchet; a suite renamed away from `test_*.py` is caught by (a), a deleted one is a
   reviewed diff.
4. **The roots** are the set's unique dirnames, derived on every run — no fixed root list to forget to extend.
   **Each root runs in its own interpreter, non-recursively**: it loads exactly the set members whose dirname is
   that root, as top-level modules from an explicit list; the loader never walks a directory. Why: `_webref` is
   a package, so a recursive `discover` rooted at `.claude/tools` would collect `_webref`'s suites a second
   time under a package-qualified import name (Q9 plants that shape), and `.claude/tools` becomes a root the
   day a `test_*.py` lands directly in it. One process per root also keeps two roots' same-named modules apart.
   Every root runs under the offline regime of §4.3.
5. **Every member contributes at least one test that executed and passed**, else the script fails naming the
   **file** — per file, not per root, so a no-test file beside passing siblings is caught (a scheduler's
   residual failure is a member *collected* that runs nothing). **A test counts only if it executed and passed**:
   **any skipped test makes the run red, naming it** (and likewise an expected failure or an unexpected
   success — neither is a pass). A skip is not a way out of the floor leg (a `skipIf` on the version) or of
   the offline regime (a skip on a failed connect). At the base no test file skips:
   `git grep -nE 'expectedFailure|SkipTest|skip(If|Unless|Test)?\(|unittest\.skip' origin/main --
   ':(glob)**/test_*.py'` matches nothing.

The script's header comment is the permanent home of the procedure **and of the wrapper contract below**,
including `--self-test` as the normative spelling; `mise.toml`'s task comment points at it (L1).

**Entering the set from a non-`discover`-shaped self-test — the contract.** A self-test that is a command
rather than a unittest module (PR #510's `python3 .claude/tools/plan-memo-umbrella-check.py --self-test
--mutants`) enters as a thin tracked `test_*.py` beside it. The contract: the command's self-test is
reached by the flag **`--self-test`** from a top-level `__main__` guard (the repo's one spelling); the
command's own file is **not named `test_*.py`** (a member is excluded from 3(b), so its self-test would never
be required to run) and **defines no `TestCase`** (3(a) would keep it red); the wrapper
names the command's file by path (step 3(b)); runs the command through `sys.executable` (so the matrix leg's
interpreter is the one measured), without `-I` / `-E` / `-S`, **in an environment equal to the regime on
every regime key** (§4.3) — it may add or change only non-regime keys, and `PYTHONPATH` only by appending
after the guard's entry, whether by an `env=` it builds or by mutating its own `os.environ` / `os.putenv`
before spawning — so the command is outside the unguarded-child class; asserts exit 0 **and** nonzero control
and mutant counts as the self-test reports them (a self-test that ran no control is not a pass — step 5, one
level down); and stays offline (L4). **The wrapper is
#510's to write, not A-iii's** (the #519 boundary rule); A-iii defines the contract and, through step 3(b),
makes its naming clause the only way in. Under A1 the same PR removes that self-test from `trip-wires`. The predicate
also selects `.claude/tools/claim-gate-plan-check.py` on the parked stale-claim lane (`origin`'s backup ref
`refs/backup/stale-claim-detector`, not a branch — §13's predicate one-liner with its first command replaced
by `git fetch -q origin refs/backup/stale-claim-detector`, verified 2026-10-24 at `01b645e6`); step 3(b)
holds it red until it fits the contract.

#### §4.2.1 The job boundary — stated as a property

The user's A1 (2026-09-22) reads: "`trip-wires` keeps a floor of shell, `git` and `grep`; a check that needs an
interpreter lives in A-iii's `tools` job". A1 itself puts `grep` — whose behaviour differs across GNU and BSD
builds — in the floor, so its criterion is not "the tool's version can matter" but **"the check needs a
language runtime"**. Definition: an **interpreter** is a language runtime (`python3`, `node`, …) whose version
the check's result depends on; the POSIX text utilities (`grep`, `sed`, `awk`) are part of the shell floor.
Consequence: **no existing wire moves** — `main`'s `layout-box-reader-trip-wire.sh` and
`wasm-runtime-trip-wire.sh` use `awk` and stay in `trip-wires`
(`git grep -nwE 'awk|python3?|node|perl' origin/main -- '.claude/tools/*trip-wire*.sh'`); so does the K2 wire
on #501's branch, whose one `python3` hit is a comment (the same command at `origin/webref-cite-audit-tool`);
#510's `python3` wire is the one that moves.

The rule, as three properties:

- **`trip-wires` declares no interpreter** — it has no setup step.
- **`tools` declares its interpreter** — `actions/setup-python`, on every leg.
- **A check that needs an interpreter belongs to the job that declares one.**

The `tools` job's comment states these rules **self-contained** — `trip-wires`' no-setup-step property in its
own words, the three properties, §8's timeout re-derive rule — citing PR #501's `trip-wires` comment as the
source only, since that text is not on `main` when A-iii lands first.

#### §4.2.2 Why no filter

Earlier drafts of the merged Slice A specified a `tools` path filter. A-iii drops it on #496's argument (one
issue, one way): a filter makes a gate's tamper path an allowlist entry someone must keep current — a PR
editing only a suite's input would skip the job that reads it. The trigger is `on: pull_request: branches:
[main]` with no `paths:` / `paths-ignore:` (`git show origin/main:.github/workflows/ci.yml`, lines before
`jobs:`); Q8 pins that block as well as the job.
**Accepted behaviour delta**: every pull request targeting `main`, a Dependabot bump included, runs the
suites (§5 row 2).

### §4.3 The network question — answered in the script, with a stated limit

The script runs each root's interpreter under an **offline regime** with two halves, placed where every
stdlib network client passes — the socket layer — rather than at named clients:

- **The socket guard (behaviour half).** Property: **network access through Python's socket layer —
  connection and name resolution — fails in every guarded process**, installed at interpreter startup
  before any member loads, through the environment (one mechanism for the loader and for every Python
  child, `webref` included). The guard is installed **exactly once per process**, **does not suppress a
  `sitecustomize` that would run without it**, and a driver run nested inside a guarded process (as the pins
  do) is idempotent (G7–G8). **A guard that fails to install fails the process closed** (nonzero exit) —
  CPython's `site` would print a raising `sitecustomize` and carry on unguarded (G7). The functions it
  covers are exactly those T-net plants (G1–G5, the resolvers `gethostbyname` / `gethostbyname_ex` /
  `gethostbyaddr` / `getnameinfo` included beside `getaddrinfo`).
- **The environment half.** The **regime keys**: `PYTHONPATH` (the guard's entry), `http_proxy` /
  `https_proxy` (a closed local port), `NO_PROXY` / `no_proxy` (absent — an inherited `NO_PROXY=*` would
  bypass the proxy), `XDG_CACHE_HOME` (a fresh empty directory). The loader **restores the regime keys before each member** (in
  `os.environ` and the process environment), so one member's mutation cannot silently unguard a later
  member's conforming children (G11).

**The limit, stated once — the unguarded-child class**, by property: **a child whose environment differs
from the regime (of the nearest enclosing driver run — a nested driver sets its own) on any regime key** —
the guard's `PYTHONPATH` entry missing or not first, a proxy key changed, `NO_PROXY` / `no_proxy` present,
`XDG_CACHE_HOME` changed — however it came to differ (an overriding `env={**os.environ, "PYTHONPATH": d}` is
in the class; so is a parent's `os.environ` / `os.putenv` mutation before spawning; `os.execve(p, a,
os.environ)` inherits and is not), **or that runs Python with `-I` / `-E` / `-S`, or is not Python**
(environment half at most; it can ignore proxy settings). Also outside the claim, in a guarded process: a
socket-layer path T-net does not plant (`_socket` used directly, a raw `sendto`). Every other mention of the
limit (L4, §2, the wrapper contract, §5 row 7, §12(4)) refers here. The claim is exactly: **a green run is
the offline measurement for the loader and every child outside the class, over the paths T-net plants**. The
class is stated as a property: every launch the pins make that changes a regime key is a named, intended
member of the class (the T-net no-regime baseline in G9, launched deliberately outside the regime against a
loopback listener, and the override children in G10); every non-Python child the pins start (git, the bash
driver, `/bin/sh`) is a member by definition and is outside the network measurement, not an escape from it. **Rejected: OS-level network isolation** (a network namespace)
covers every child on the Linux runner but has no macOS equivalent for the local `mise` caller, so the
callers would differ in *whether* — breaking L1 (§2 L1 × L4).

At the base, no test file reaches for the network or spawns
(`git grep -nE 'socket|http\.client|urlopen|import subprocess|from subprocess' origin/main --
':(glob)**/test_*.py'` matches nothing) — a statement about the test files only; what the modules under test
do is the regime's to catch.

⚠ **What A-iii does *not* claim**: that the gate becomes offline-capable. `verify_citation` shells out to
`webref heading`, which issues a conditional GET, so **`origin/main`'s gate requires the network in default
mode**. A-iii's claim is exact: *the suites run green offline*.

⚠ **The umbrella's earlier live-network record** was measured on a branch whose fetching was Slice **B**'s
catalog fall-through; B ships the offline contract for what B introduces, and A-iii inherits no such dependency.

### §4.4 The interpreter floor — measured by one leg, stated by one assertion

The floor is **3.9**, and the **only measurement of it is the `"3.9"` matrix leg** running the set (draft
1's static `ast` pre-check is dropped: it missed newer-runtime APIs, Codex R21, and misread a set union as PEP
604 — wrong both ways). The script's assertion and the job's `python3 -VV` echo are the *statement*; §4.2 step 3's parse of
every tracked `.py` is a different property, not a floor instrument.

**The legs.** `"3.9"` and `"3.x"` — the newest the action resolves, so a newest-runtime breakage shows on the
PR that meets it. `setup-python` serves 3.9 on both current Ubuntu runner images: the action's manifest lists a
3.9.x build for linux x64 on 22.04 and 24.04 (`gh api repos/actions/python-versions/contents/versions-manifest.json
--jq .content | base64 -d | jq '[.[] | select(.version|startswith("3.9.")) | .files[] | select(.platform=="linux"
and .arch=="x64") | .platform_version] | unique'`).

Both legs go through `setup-python` (§4.2.1). `--mutants`-shaped self-tests run on **both** legs, with no
per-leg branching (D3): a check that skipped the floor leg would leave the floor unmeasured for exactly the
heaviest code. Slice B raises the floor when B lands `(?>...)`; the leg's `python-version` moves with it.
`SKILL.md`'s Step 0 invokes `preflight.py` directly, bypassing the script; at floor 3.9 that leg is
unaffected, and **it is assigned to Slice B's floor raise** — B names that invocation in its edit set (§6
UNCHECKED, §13).

### §4.5 What "enforced" can honestly mean here

`main` is governed by an **active** ruleset whose rules are `deletion` / `non_fast_forward` /
`pull_request` (`gh api repos/send/elidex/rules/branches/main --jq '[.[].type]'`). There is **no
`required_status_checks` rule**, so a red `tools` job does not block a merge; CLAUDE.md's "CI 全 pass を目視確認
してから squash merge" is the blocking step, and it is human. (The 404 from `…/branches/main/protection` is the
deprecated legacy endpoint, not "unprotected".) The claim A-iii may make: the job makes a regression
**visible, attributed, and on the PR page at review time**.

### §4.6 The project docs that enumerate the gate

**The rule, as a property.** Every tracked Markdown file that enumerates `mise` tasks, `mise run ci`'s parts
or the CI jobs, or states a premise A-iii falsifies (e.g. that `mise run ci` is cargo-only, or that
`trip-wires` is a home for a check that needs an interpreter), is updated in the same PR. Discovery is one
command, repo-wide over `*.md`, not a file list (`mise run [a-z]` and `test-all` catch task-list lines —
on `origin/main` it hits `README.md`'s "Common Tasks" block and `CLAUDE.md`'s `## Commands` block and its
"その他:" line, verified 2026-10-24):

```sh
git grep -nE 'mise run [a-z]|test-all|trip-wires|つ目の job|cargo-only' origin/main -- '*.md'
```

**Decision per class of hit:**

- **An enumeration of `mise` tasks** (`README.md`'s task list, `CLAUDE.md`'s "その他:" list) gains
  `tools-test` (the Python suites = `scripts/python-suites.sh`).
- **An enumeration of `mise run ci`'s parts or of the CI jobs** gains `tools-test` / the `tools` job (and
  `trip-wires` where the same enumeration lacks it); a statement of what the ungated jobs buy (red on the PR,
  not a merge block) extends to `tools`. `CLAUDE.md` `## CI` gains the fifth job's paragraph: ungated, legs
  `"3.9"` / `"3.x"`, `bash scripts/python-suites.sh` = `mise run tools-test`, §4.2.2's reason, §4.2.1's boundary.
- **A premise A-iii falsifies** is rewritten true, keeping its conclusion where it holds: "`mise run ci` is
  cargo-only" → it runs the Python suites and the trip-wires, which check only what the gate code's own tests
  assert, so the review stages stay the only *review* coverage; "the ungated `trip-wires` job" as a Python
  checker's home → "the ungated `tools` job, entering as a tracked `test_*.py`" (L6).
- **A mention that enumerates nothing, or a statement still true after A-iii** (e.g. nothing runs the
  plan-memo checkers: `git grep -nE 'plan-sweep|plan-xcheck|plan_sweep|plan_xcheck' origin/main --
  ':(glob)**/test_*.py'` matches nothing) is unchanged.

**Present result, by file name**: update targets `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, and the
`pre-push`, `elidex-review`, `elidex-plan-review` `SKILL.md`s; the `external-converge` / `external-review`
`project.md`s are in the unchanged class.

**Negative control — a class of hit that is not an update target.** Most hits are under `docs/plans/**` and
`docs/audits/**`: landed records of their own slice, true of their time. One live plan text repeats the
plan-review note's "trip-wires job" option (`git grep -n 'Step 1.5 or the ungated' origin/main -- '*.md'`), but
assigns the task's durable home to that `SKILL.md` note in its own words, so the note is the one site to fix.

---

## §5 Behaviour deltas

| # | Input | `origin/main` | After A-iii |
|---|---|---|---|
| 1 | a `.claude/**`-only PR targeting `main` | `changes` and `trip-wires` only; no suite runs | + the `tools` job |
| 2 | a `crates/**`-only PR targeting `main` | `check`/`doc`/`deny` + `trip-wires` | + the `tools` job (ungated) |
| 3 | `mise run ci` locally | does not run the Python suites | runs them via `[tasks.tools-test]` |
| 4 | a new tracked `test_*.py` anywhere in the repo | runs nowhere | runs; a failure is red |
| 5 | a tracked `test_*.py` that loads no test, beside a passing sibling | runs nowhere | **script fails loudly** naming the file |
| 6 | a non-member tracked `.py` defining a `TestCase` (step 3(a)), or a non-member `--self-test` entry point (step 3(b)) that no member names by path | nothing | **script fails loudly** naming the file |
| 7 | the suites, offline | n/a — nothing runs them | the loader and every child outside the unguarded-child class under the regime (L4; the class is §4.3's stated limit) |
| 8 | a member with a skipped test beside a passing one | runs nowhere | **script fails loudly** naming the test |

**Newly-red**: 4 (a failing suite), 5, 6 and 8 — A-iii adds a job; it does not change what any existing job
asserts. Rows 5, 6 and 8 are green at `origin/main`: every member contributes tests, no `TestCase` is outside
the set, no tracked `.py` contains `--self-test` (`git grep -l -e '--self-test' origin/main -- '*.py'` matches
nothing; `git grep -l TestCase origin/main -- '*.py'` lists only set members), and no test skips (§4.2
step 5's command).

---

## §6 Pins

**Every pin is a test in `scripts/test_python_suites.py`**; "Red at main" says whether it fails on
`origin/main`, the base. **No pin runs the driver on the live tree** — the driver runs this file, so such a pin
would recurse. Every pin that executes the driver builds a **scratch repository** (`git init` in a temporary
directory, the driver copied in, fixture files staged with `git add`; never the pins file itself; every git call and driver run with
**all `GIT_*` variables removed**, so an inherited `GIT_DIR` / `GIT_INDEX_FILE` cannot point git at another
repository), so the live
tree's properties are the driver's own checks on every run (§4.2 steps 1–5, §4.3), and the pins prove each of
those checks can fire.

**The text readers — one property, stated once for the whole of each reader.** Pins that read `ci.yml` or
`mise.toml` read text (the floor interpreter has neither a YAML parser nor `tomllib`, 3.11+). Property:
**in the regions the pins read** (`ci.yml`'s workflow-level `on:` block and `tools` job region;
`mise.toml`'s `[tasks.ci]` and `[tasks.tools-test]`), **any construct the reader does not model reads red,
naming the line** — the unknown falls to red, never to "absent", because a reader that skipped an unreadable
construct could read a gated job as ungated. The grammar the reader models is the implementation's choice,
stated in the test; §6.1 rows R1–R9 and M1–M2 are the constructs it must provably handle or refuse.

| Pin | What it executes | §5 rows | Red at main? |
|---|---|---|---|
| **Q1** | a scratch repo with passing members in two directories exits 0, and the output names **every member** with a nonzero test count — compared against the scratch repo's `git ls-files ':(glob)**/test_*.py'`, not against a list | 3, 4 | **yes** — the script does not exist |
| **Q2** | a **failing** `test_planted.py` in a directory holding no other member makes the script exit non-zero naming it — a failing plant, because a passing one would also pass by being skipped; a directory with no other member is reached only because the roots are derived | 4 | **yes** |
| **Q3** | a `test_empty.py` defining no test, **beside a passing sibling** in the same directory, makes the script exit non-zero naming `test_empty.py`; and a scratch repo with no tracked `test_*.py` exits non-zero (the empty set) | 5 | **yes** |
| **Q9** | non-recursion: a package directory (`__init__.py` + a passing member) with a passing member in its parent directory — every test id appears **exactly once** in the run | 4 | **yes** |
| **Q10** | complement (a), §6.1 P1–P2 | 6 | **yes** |
| **Q11** | complement (b), §6.1 P3–P7 | 6 | **yes** |
| **Q12** | skip is red: a member holding one passing test and one `@unittest.skip` test makes the script exit non-zero naming the skipped test (a per-file count alone would read the file as contributing) | 8 | **yes** |
| **Q4** | `mise.toml`'s `[tasks.ci].depends` contains `"tools-test"`, and `[tasks.tools-test]` runs `bash scripts/python-suites.sh` and has no `sources` / `outputs` key (a task with `sources` is skipped by `mise` when its sources are unchanged, so a suite could go unrun locally) — §6.1 M1–M2 | 3 | **yes** |
| **Q5** | the floor assertion: the script exits non-zero naming the floor when the `python3` it finds reports a version below 3.9 (fixture: a PATH-shadowing `python3` shim) | — | **yes** |
| **Q6** | `ci.yml`'s `tools` job region (bounds: §6.1 R9) contains a **`run:` step that executes the driver** — the driver is the command, not text in the region, and is not masked (§6.1 D1). Without it an empty `tools` job satisfies §12(3) (Codex R14/R36/R48) | 1, 2 | **yes** — the job does not exist |
| **Q7** | the same region carries a `python-version:` matrix with legs `"3.9"` and `"3.x"` **and a `uses: actions/setup-python` step whose `python-version:` input is `${{ matrix.python-version }}`, before Q6's `run:` step** — a matrix no step consumes runs one interpreter on every leg (Codex R48), and a `setup-python` after the driver step measures nothing (Codex R49) | — | **yes** |
| **Q8** | the `tools` job is **present and ungated**: its region is non-empty and carries no `needs` and no `if` key — deliberately not "not gated on a known output", which would read a job gated on a *new* output as ungated; **negative control**: the same predicate reads `check` (`needs: changes` + `if:` on `rust`) as gated (Codex R49). **And the trigger**: the workflow-level `on:` block's `pull_request` carries no `paths:` / `paths-ignore:` and its `branches` is exactly `[main]` | 1, 2 | **yes** |
| **Q13** | the text readers' property (§6 above), §6.1 R1–R9 and M1–M2 | — | **yes** — the readers do not exist |
| **T-net** | the offline regime, §6.1 G1–G11: each network plant is **green without the regime and red with it**, making the script exit non-zero naming the plant | 7 | **yes** |

⚠ **UNCHECKED, marked not omitted**: that a red `tools` job **blocks** a merge — **false**, §4.5; the
interpreter floor on `SKILL.md`'s direct `preflight.py` path, which bypasses the script — **assigned to Slice
B's floor raise** (§4.4).

### §6.1 Property → pin

Every mechanism detail the memo does not specify is pinned here. Each row reads "a test exists in
`scripts/test_python_suites.py` that …"; "red" = the driver (or the reader) exits non-zero **naming** the
file, test, plant or line. Driver rows run in scratch repos (§6).

| Row | Property kept in the memo | Pin (test shape, with its controls) |
|---|---|---|
| **P1** | 3(a): a non-member whose class's base resolves within the file to `unittest.TestCase` / `IsolatedAsyncioTestCase` is red | red, each in its own non-member `helper.py`: `class A(unittest.TestCase)`, `from unittest import TestCase as T` + `class A(T)`, `class A(unittest.IsolatedAsyncioTestCase)`, and in-file indirect `class B(unittest.TestCase)` + `class C(B)`; **control**: `class A(Foo)` with `Foo` unrelated is green |
| **P2** | 3(a)'s limit: a base reaching `TestCase` only through another file is outside | `helper.py` with `from base import MyCase` + `class A(MyCase)` (where `base.py` defines `MyCase(unittest.TestCase)`) makes the run name `base.py` (P1) and **not** `helper.py` — the test names this as the stated limit |
| **P3** | 3(b)'s predicate is syntactic: a top-level `__main__` guard **and** the constant `'--self-test'`, never a string grep | red: `a/tool.py` with guard + constant, named by no member; **green controls**: constant without guard; guard without constant; guard + `--self-test` only in a comment; guard nested in a function (not top-level) |
| **P4** | *named by path*: one constant, root-relative or member-dir-relative | green: `a/test_tool.py` holding `"tool.py"`; a member elsewhere holding `"a/tool.py"`; **red**: a second `b/tool.py` beside `a/test_tool.py`'s `"tool.py"` stays red naming `b/tool.py` (filename is not a name); a member holding `"a/" + "tool.py"` (assembled) leaves `a/tool.py` red |
| **P5** | S1: set members are excluded from 3(a) and 3(b) | green: a member `a/test_wrap.py` with `if __name__ == "__main__": unittest.main()` **and** the constant `"--self-test"`, named by nothing, defining a passing `TestCase` |
| **P6** | the pins file is a member and never credits a live entry point | a test parses `scripts/test_python_suites.py` itself (no driver run) and, calling **the driver's own** entry-point and named-by-path predicates, asserts it names by path none of the live tree's tracked **non-member entry points** (its fixture paths name files inside scratch repos only); **control**: the same predicate credits a planted root-relative and member-dir-relative constant |
| **P7** | a tracked `.py` that does not parse under the running interpreter is red, member or not | red: non-member `bad.py` with a syntax error; red: member `test_bad.py` with a syntax error |
| **R1** | keys are recognised in any quoting | in the `tools` region, `"needs": changes` and `'if': x` each read as gated |
| **R2** | the modelled values read correctly | green: `branches: [main]`, `python-version: ["3.9", "3.x"]`, `python-version: ${{ matrix.python-version }}` |
| **R3** | value-position constructs not modelled are red | red, in the `tools` region: a value `{a: 1}`, `&x`, `*x` |
| **R4** | key-position constructs not modelled are red | red: `&a needs: x`, `!!str needs: x`, an explicit key `? needs` |
| **R5** | merge keys are red | red: `<<: *x` and `<<: {needs: changes}` in the `tools` region |
| **R6** | a multi-line flow sequence never reads as something else | `branches: [main,` / `  dev]` across two lines is red (never read as `[main]`); a `python-version: [` multi-line list is red (fail closed — no exact multi-line reading is modelled) |
| **R7** | comments, including trailing, are not values and not keys | `needs: changes # x` reads as gated; a `# needs: changes` line reads as no key (job ungated); `tools: # c` is still the region's header; `run: bash scripts/python-suites.sh # c` still satisfies D1 |
| **R8** | block scalars: headers `\|` `>` `\|-` `>-` `\|+` are recognised; the body is text | for each header, a body containing `*.py` and a line `needs: x` is green and reads as no key; an unrecognised header (`\|9x`) is red |
| **R9** | the `tools` region is bounded by the next job | a fixture whose **next** job carries `needs:`, a driver `run:` and a `setup-python` step: none is credited to `tools` (Q6–Q8); a two-space-indent comment inside `tools` does not end its region |
| **M1** | `mise.toml` sections the pins read: unmodelled constructs are red | green: `depends = ["check", "tools-test"]`; red: `depends = { a = 1 }`, a multi-line `depends = [` array, a `run` not beginning a quoted string; a `depends` containing `"tools-test"` in another section is not credited |
| **M2** | S4: `[tasks.tools-test]` has no `sources` / `outputs` | red: the fixture section with `sources = ["scripts/**"]`; red: with `outputs = [...]` |
| **D1** | S3: the driver is the command — argv[0], or the argument after `bash` / `sh` — and is not masked | green: `run: bash scripts/python-suites.sh`, `run: scripts/python-suites.sh`, `run: mise run tools-test`; **red**: `env: DRIVER: scripts/python-suites.sh` alone, `run: echo scripts/python-suites.sh`, `bash scripts/python-suites.sh \|\| true`, `bash scripts/python-suites.sh; true`, `continue-on-error: true` on that step, and on the job |
| **G1** | socket-layer connection fails in-process | a member doing `http.client.HTTPConnection` and `urllib.request.urlopen` to a `127.0.0.1` listener (an `https://` URL included — red at connect, before TLS) |
| **G2** | … in a Python child (`webref` included) | a member's `sys.executable` child doing a raw `socket.socket().connect` to the listener, the member asserting the child's exit 0; the same through an executable with a `#!/usr/bin/env python3` shebang (`webref`'s shape) |
| **G3** | `connect_ex` | in-process `socket.socket().connect_ex` to the listener, asserting a return of 0 |
| **G4** | `AF_INET6` | `connect` to a listener on `::1` (a host without IPv6 loopback fails the unguarded run — red, not a skip) |
| **G5** | name resolution fails | in-process `socket.getaddrinfo('localhost', …)`, `gethostbyname`, `gethostbyname_ex`, `gethostbyaddr('127.0.0.1')` and `getnameinfo` (numeric flags), each red; the guard is active at import time — the `getaddrinfo` call at a member's module top level is red |
| **G6** | the environment half, per regime key | the driver launched with an inherited `NO_PROXY=*`, `no_proxy=*`, a real `http_proxy` / `https_proxy` and a set `XDG_CACHE_HOME`; a member asserts a **non-Python** child (`/bin/sh -c env`) sees each inherited value — red under the regime, once per key |
| **G7** | B2: installed exactly once per process; a nested driver run is idempotent | a member asserts the guard's install marker counts 1 in the loader and in a child; the driver run from inside a guarded process (the way the pins run it) gives the same G1–G5 result and a count of 1; **fail closed**: with a raising `socket.py` shadow later on `PYTHONPATH`, the guard's install fails and the run is red naming the guard, with no test run |
| **G8** | B2: a `sitecustomize` that would run without the guard is not suppressed | with a pre-existing `sitecustomize.py` later on `PYTHONPATH` that sets a marker, a member asserts the marker is set **and** G1 is still red |
| **G9** | B3: the pins' own members of the unguarded-child class are deliberate | every G-row's baseline launches the member outside the regime (regime keys stripped), against a loopback listener only, and is green (so each plant can fire); G6's driver launch with altered regime keys is another (its descendants are under the inner driver's regime, G7); the set of pin launches that change a regime key equals the named set (G9's baseline, G6's driver launch, and G10's override children), and the pin fails if a regime-key-changing launch is not in it |
| **G10** | B1: the class is defined by regime-key difference, not by spawn mechanism | under the regime, the test asserts: a child with `env={**os.environ, "PYTHONPATH": d}` (guard entry dropped) **can** connect, and a child spawned after the parent sets `os.environ["NO_PROXY"] = "*"` sees it — each **in** the class (the limit is real, documented by a passing assertion); "connect" is a raw `socket.connect` to a `127.0.0.1` listener the same test opens (no proxy, no name resolution, no external host), so G10 needs no network and holds offline (L4); any `os.environ` change G10 makes happens inside a scratch-repo member's own process and is restored before the member returns; a child that `os.execve`s `sys.executable` with `os.environ`, and one with `env={**os.environ, "X": "1"}`, **cannot** connect — **outside** the class; **control**: a non-network `sys.executable` child passes |
| **G11** | the loader restores the regime keys before each member | two members in one root: the first mutates `NO_PROXY`, `PYTHONPATH` (via `os.environ`) and `https_proxy` (via `os.putenv`); the second asserts a `/bin/sh -c env` child sees the regime's values and a `sys.executable` child is still refused a loopback connect — green only if the keys were restored |

---

## §7 Layering check

**VM host/ / ECS-native** — not applicable.

**Generic core vs elidex adapter.** A-iii touches **no `_webref` file** and adds **exactly one `test_*.py`,
under `scripts/`** (its pins). The implementation PR's `git diff --name-only origin/main...HEAD` is
`scripts/python-suites.sh`, `scripts/test_python_suites.py`, `mise.toml`, `.github/workflows/ci.yml`,
`CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, `.claude/skills/pre-push/SKILL.md`,
`.claude/skills/elidex-review/SKILL.md`, `.claude/skills/elidex-plan-review/SKILL.md` (§4.6), and this memo.

The `mise` task, the CI job and the floor are recorded in `python-suites.sh`'s header, the `mise.toml` task
comment, the `tools` job comment and §4.6's docs, not in `_webref/DESIGN.md`: that file's core is meant to move
to a standalone repository, where a section about elidex's `ci.yml` is wrong on arrival.

**One-issue-one-way**: the suite invocation goes from **zero** canonical sites to one, and a suite enters it
one way — by being a tracked `test_*.py` (§4.2), which step 3 makes the only way.

---

## §8 Budget — lines and wall clock

**Lines.** Two new files under `scripts/`; `mise.toml` and `ci.yml` gain a task and a job; §4.6's docs gain a
phrase or a paragraph each. No touched file is near a split (`git show origin/main:<file> | wc -l` for each).

**Wall clock.** The legs run in parallel, so the job's wall clock is the slower leg's. `timeout-minutes` is
derived from the implementation PR's own CI run, and the rule is stated in the `tools` job comment (source:
#501's `trip-wires` comment): *a PR that adds suite cost re-derives the timeout in the same PR* — first of all
#510's wrapper. `ci.yml` carries the rule and the command to re-derive, **no runtime figure**.

**The budget half.** A-iii takes **#510's share**: #510's self-test moves into `tools` (A1), so its cost is
`tools`' timeout, re-derived under the rule above. The other claimants — the K2 wire on #501's branch
(`webref-generic-core-trip-wire.sh` and its `.controls` / `.mutations` / `.harness` parts) and the stale-claim
lane's `claim-provenance-trip-wire.sh` — are shell-plus-`awk` wires and stay in `trip-wires`' budget
(`git ls-tree -r --name-only <ref> .claude/tools | grep trip-wire` lists a ref's wires, §4.2.1's `git grep`
classifies them), so the budget half is **closed for `tools`**.

*Local measurements — claims, machine-dependent, for sizing only:* PR #510 at `06926607` (its head has since
moved; the frame is this commit), `plan-memo-umbrella-check.py --self-test` 8.7 s (Python 3.14.7) / 23.6 s
(Python 3.9.6); with `--mutants` 53 s / 131 s. The 3.9 leg is the slower one, by the ratio these imply on that
machine.

---

## §9 Edge-dense assessment

**(i)** The umbrella (on #501) names A-iii and its scope; the order change is §0's, carried into the umbrella
at #514's rework.

**(ii)** §2's six invariants L1–L6 are configuration invariants of one observable (does the job run the
whole set, offline, on the floor, and is it green), and they intersect — all 15 pairs are mapped in §2: edge-dense, hence
mandatory plan-review. It is nonetheless **one PR** — CLAUDE.md's *Edge-dense work* rule (c): a narrowly-scoped
slice under an approved umbrella that has passed plan-review is the terminal unit — and splitting would
separate the script from the job that is its only observable. It is separate from A-i / A-ii because it was
**invisible** inside the merged memo, where the one cross-lane collision that memo missed lived.

---

## §10 Open questions

- **Q1 — does `required_status_checks` belong in this PR?** The `pull_request` rule carries
  `required_approving_review_count: 0` **and** a `RepositoryRole` bypass with `bypass_mode: always`
  (`gh api repos/send/elidex/rulesets --jq '.[] | select(.name=="main-protection") | .id'`, then that ruleset's
  `.bypass_actors`), so adding it leaves it author-bypassable: visibility-plus-friction, not enforcement.
  **Recommendation: register a slot, do not implement** (§11).
- **Q2 / Q3 — closed (draft 3):** zero-test grain per file (§4.2 step 5); legs `"3.9"` / `"3.x"` (§4.4).

---

## §11 Defer slots + per-PR ≤3 audit

**Zero own deferrals.**

**Registered here because A-iii adds a CI job**: **`#11-elidex-ci-required-status-checks`** — the ruleset has
no `required_status_checks` rule, so every CI job is advisory. A-iii neither creates nor worsens it. **Why
deferred**: the rule alone is author-bypassable given the bypass actor, so it is not a one-line fix.
**Trigger**: the first job stable enough to require, or the bypass actor being removed. **Re-eval**:
2026-11-30. **Confidence**: Medium. It is in no ledger today; §13's checklist hands it to the PM.

---

## §12 Exit criterion

**(1) Green:** `mise run tools-test` and `bash scripts/python-suites.sh`, on both legs.

**(2) Red at main:** every §6 pin whose last column says "yes". No second list.

**(3) The job fires on every pull request targeting `main`:** Q8 green (trigger and job; the `check` control
read as gated by the same predicate), and a `.claude/**`-only commit shows the `tools` job on the PR page.
Stated as a positive observation.

**(4) Offline:** T-net, and (1) — every green run is under the regime, with exactly §4.3's coverage and its
stated limit (the unguarded-child class, §4.3).

**(5) The job runs the driver:** Q6. (3) alone is satisfiable by a job that runs nothing.

**(6) The floor is measured:** Q7 green and the `"3.9"` leg green on the PR page — the one measurement (§4.4).

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** ([PR #514](https://github.com/send/elidex/pull/514), stacked on #501) | none in A-iii's files | **A-iii first** (§0, D1); the order amendment rides this lane's #514 rework (§0) |
| **[PR #510](https://github.com/send/elidex/pull/510)** (VM P4) | `ci.yml`, `mise.toml`, `scripts/trip-wires.sh`, `CLAUDE.md` `### Workflow` and `## CI` | **A-iii first**; #510 then enters `tools` through the §4.2 wrapper contract (A1) and re-derives its timeout (§8) |
| **[PR #506](https://github.com/send/elidex/pull/506)** (VM P4 umbrella plan) | adds its own `.claude/tools/plan-memo-umbrella-check.py`, which step 3(b) selects | after A-iii; #506 already waits on #510 |
| **[PR #501](https://github.com/send/elidex/pull/501)** | `ci.yml` (the `trip-wires` comment and timeout), `CLAUDE.md` `## CI`; adds `test_spec_labels.py`, which §4.2 picks up with no edit | either order; the umbrella amendment rides #514's rework (§0) |
| **[PR #527](https://github.com/send/elidex/pull/527)**, **[#381](https://github.com/send/elidex/pull/381)**, **[#502](https://github.com/send/elidex/pull/502)** | `ci.yml` (#527 also `scripts/trip-wires.sh` and `CLAUDE.md`; the two Dependabot bumps `steps:` only) | whichever lands second adapts textually |
| **Slices B / C** | after A-iii these are scheduled from their first commit; **B** owns the floor raise and with it `SKILL.md` Step 0's direct `preflight.py` invocation (§4.4) | after |

**After A-iii lands, #510's and #506's `tools` job is red under step 3(b) until a set member names
`.claude/tools/plan-memo-umbrella-check.py` by path** — the intended forcing function, not a new block.
Follow-ups owed by other lanes are handed to those lanes' drivers and tracked in the CI/tools lane SSoT, not
here.

⚠ **Stacked lanes do not get the job yet.** `ci.yml` triggers on PRs targeting `main` only, so #514 and #527
(base `webref-cite-audit-tool`; `gh pr list --state open --json number,baseRefName`) run no `tools` job until
#501 lands and they retarget `main`.

**The lane facts are derived, not listed.** Before landing, the driver re-runs the sweep over every open PR and
reads it by line, not by file:

```sh
gh pr list --state open --limit 200 --json number --jq '.[].number' | while read n; do git fetch -q origin "refs/pull/$n/head" && git diff --stat origin/main...FETCH_HEAD -- .github mise.toml scripts '*.py' CLAUDE.md CONTRIBUTING.md README.md .claude/skills/pre-push/SKILL.md .claude/skills/elidex-review/SKILL.md .claude/skills/elidex-plan-review/SKILL.md | sed "s/^/#$n /"; done
```

It ranges over A-iii's contended files in every open PR — CI and doc paths, and every `*.py` (step 3's
population) — not over `docs/plans/`, the shape that hid #496 from the merged memo. For a PR whose `*.py`
lines show, step 3 is run over its head (`n` set to the PR number) by **the driver itself** — its own
predicate, not a re-spelling of it — on a detached worktree of that head:

```sh
git fetch -q origin "refs/pull/$n/head" && w=$(mktemp -d) && git worktree add -q --detach "$w/t" FETCH_HEAD && cp scripts/python-suites.sh "$w/t/scripts/" && { bash "$w/t/scripts/python-suites.sh" > "$w/log" 2>&1; true; }; grep 'entry point that no set member names by path' "$w/log"; git worktree remove --force "$w/t"
```

Each entry point it names needs a member naming it by path on that PR's head, or that PR's `tools` job is
red. (Draft 6's `ast` one-liner checked only `left.id == "__name__"`; it is retired for this reason.) On #510's
head `a322b845` this names exactly `.claude/tools/plan-memo-umbrella-check.py` (verified 2026-10-24).

**Landing checklist**

1. Re-run the sweep above; a PR it shows on an A-iii line and not in the table gets a row before landing.
2. §4.6's rule holds: its discovery command is re-run and every update target it shows is updated in the
   same PR.
3. Report the landing — the order change (A-iii before A-ii) and the slot
   `#11-elidex-ci-required-status-checks` to register — to the **PM**, who owns `project_open-defer-slots.md`
   and `project_citation-hygiene-program.md`.
4. PR description: §4.2's ungated decision and D2's derived roots; §4.2.1's boundary; §4.5's honest
   "enforced".

---

## §14 Provenance

- Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` §4.3 (drafts 1–9): filter hole (R1),
  `trip-wires` precedent (R6), two-caller invariant, `git ls-files` scoping of L2 (R8), honest enforcement
  (R5), ungated decision and `scripts/` siting (R9).
- Draft 1 = PR #514 head `cfcb4ae6`, carried verbatim as this branch's first commit.
- Drafts 2–4 (2026-10-24): D1–D3 and the A1 boundary (draft 2, `be559816`); plan-review R1 F1–F16 (draft 3,
  `619f68b7`); plan-review R2 G1–G14 (draft 4, `6c8808ca`) — each commit message carries its list.
- Draft 5 (2026-10-24, subtractive): other lanes' follow-ups moved to the CI/tools lane SSoT; cross-file line
  numbers dropped; §4.6 as a property; reader by value position (Q13); unguarded-child class stated once;
  `sitecustomize` chain-load; §3's grep-pass runnable; T-net `connect_ex` / `AF_INET6`.
- Implementation (2026-10-24): first-fix items 1–10 applied (L2 split, L4 limit, guard fail-closed, per-member
  regime restore + G11, resolver plants in G5, contract clauses, P6 via the driver's predicate, `GIT_*` cleared,
  R6 fail-closed, §13 sweep through the driver); G9's named set gains G6's driver launch.
- Draft 6 (2026-10-24, decision altitude): mechanism prose → §6.1 Property → pin; S1 members out of step 3; B1 class by regime key; §4.6 task lists; S2–S4, B2–B3, C1.
