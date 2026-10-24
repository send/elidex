# Plan — Slice A-iii: something runs the Python suites, ungated

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md` (on PR #501's branch, not on `main`), slice
**A-iii** (the 2026-08-01 re-slice). Terminal unit under that boundary (§9).
**Order — amended 2026-10-24 (user decision D1): A-iii lands BEFORE A-ii.** The branch is off `main`, stacked
on neither A-ii nor #501. Reason: PR #510 (VM P4) cannot merge until A-iii's `tools` job exists — under the
user's 2026-09-22 decision ("option A1") a check that needs an interpreter leaves `trip-wires` for this job
(§4.2.1) — while #501, which A-ii stacks on, is blocked on items unrelated to A-iii. The umbrella's slice
order and A-ii memo's §13 row ("A-iii | after A-ii") are amended **in this lane's rework of PR #514** (A-ii's
rebase onto landed `main`, §13). ⚠ If #501 lands before that rework, `main` carries a stale order statement
for the interval between the two; the window closes at the rework, and until then this memo is the record of
the reorder.
**Nature**: repo infrastructure — one driver script and its pins, one `mise` task, one CI job, and the project
docs that enumerate `mise run ci` and the CI jobs. Zero `crates/**` diff, zero `_webref` diff, zero
gate-semantics change.
**Status**: plan-memo, **draft 3**. `/elidex-plan-review` **required before implementation**.

**This memo carries no measurements and no harness.** The program's rule (2026-08-23): a plan memo holds
neither measured digits nor a self-measuring apparatus; what needs measuring is measured by a test or a CI
job — here, A-iii's own. So every claim below is either a **§6 pin** or a fact stated **with the one-line
command that reproduces it at `origin/main`**. The one exception is §8's wall-clock figures, labelled as local
measurements with their frame.

### §0.1 What A-iii is

A-i lands the shared map and A-ii makes the gate fail closed; **nothing runs any of the program's tests**, nor
the suites `main` already has. A-iii gives the Python suites a scheduler, and nothing else.

---

## §3. Spec coverage map

A-iii has no spec surface. `main`'s preflight hard-fails a memo whose §3 has no table or no `§<number>` row,
and the `No spec surface` declaration that would replace this is A-ii's (§4.2.5 there), which lands after
A-iii (§0). So, per draft 1's own fallback, this section carries the program's two-row fixture table. **They
cite nothing A-iii implements**: the `citation verify: ok` the gate prints on this memo is a statement about
these two rows, not about A-iii.

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | fixture | none — A-iii has no spec surface | none | n/a | no |
| WHATWG HTML §4.10.21.2 Constraint validation | fixture | none — A-iii has no spec surface | none | n/a | no |

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
  is run and contributes at least one test; a tracked `.py` that *is* a suite or a self-test by property, but
  is outside the set or unreferenced by it, fails the run (§4.2 step 3).
- **L3 — the trigger cannot be silently narrowed.** No job-level gate and no workflow-level path filter: the
  job runs on every pull request targeting `main` (§4.2.2).
- **L4 — A-iii adds no network dependency.** The script runs every member under an offline regime, so a green
  run *is* the offline measurement (§4.3).
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
| L2 × L4 | every member, a wrapper included, runs under the regime; a wrapper's child inherits only its environment half (§4.3), which is why the wrapper contract also says "stays offline" |
| L2 × L5 | every member runs on both legs; a member that needs more than 3.9 is red on the floor leg, not skipped — and step 3 parses every tracked `.py` with the leg's interpreter, so an unparsable file is red too |
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
   properties a suite can have without the name. Over every tracked `.py` (`git ls-files '*.py'`):
   **(a)** a file that defines a `unittest.TestCase` subclass (read with `ast`; a base spelled `TestCase` or
   `<x>.TestCase`) and is not in the set fails, naming it; **(b)** an **entry-point self-test** — a file
   with a top-level `if __name__ == "__main__":` guard **and** an `ast.Constant` whose value equals
   `'--self-test'`, both read with `ast`, never a string grep — must be named, by its filename, in the source
   of some set member, else the run fails naming it — the wrapper contract below, made checkable. The
   population is fixed by property: a helper that merely carries `--self-test` as text is not in it (on
   #510's head, 18 of the 19 files containing the literal — `git grep -l -e '--self-test' FETCH_HEAD -- '*.py' |
   wc -l`, verified 2026-10-24), and no self-test that can actually be launched
   is missed. On #510's head (`a322b845`) the predicate selects exactly one file,
   `plan-memo-umbrella-check.py`:
   `git fetch -q origin refs/pull/510/head && git ls-tree -r --name-only FETCH_HEAD | grep '\.py$' | while read f; do git show "FETCH_HEAD:$f" | python3 -c 'import ast,sys; t=ast.parse(sys.stdin.read()); g=any(isinstance(n,ast.If) and isinstance(n.test,ast.Compare) and getattr(n.test.left,"id","")=="__name__" for n in t.body); c=any(isinstance(n,ast.Constant) and n.value=="--self-test" for n in ast.walk(t)); g and c and print(sys.argv[1])' "$f"; done`. A tracked `.py` that does not parse under the running interpreter
   fails, naming it: an unreadable file could hide a suite, so the unknown falls to red. **Limit, stated:**
   there is no count ratchet; a suite renamed away from `test_*.py` is caught by (a), a deleted one is a
   reviewed diff.
4. **The roots** are the set's unique dirnames, derived on every run — no fixed root list to forget to extend.
   **Each root runs in its own interpreter, non-recursively**: it loads exactly the set members whose dirname is
   that root, as top-level modules from an explicit list; the loader never walks a directory. Why: `_webref` is
   a package, so a recursive `discover` rooted at `.claude/tools` would collect `_webref`'s suites a second
   time under a package-qualified import name (Q9 plants that shape), and `.claude/tools` becomes a root the
   day a `test_*.py` lands directly in it. One process per root also keeps two roots' same-named modules apart.
   Every root runs under the offline regime of §4.3.
5. **Every member contributes at least one test**, else the script fails naming the **file** — per file, not
   per root, so a no-test file beside passing siblings is caught. Every member is in the set the script runs
   (step 2), so the residual failure of a scheduler is a member that is *collected* and runs nothing; this step
   is that failure's check.

The script's header comment carries the procedure; `mise.toml`'s task comment points at it (L1).

**Entering the set from a non-`discover`-shaped self-test — the contract.** A self-test that is a command
rather than a unittest module (PR #510's `python3 .claude/tools/plan-memo-umbrella-check.py --self-test
--mutants`) enters as a thin tracked `test_*.py` beside it, which: names the command's file (step 3(b));
runs the command through `sys.executable` (so the matrix leg's interpreter is the one measured); asserts exit
0 **and** nonzero control and mutant counts as the self-test reports them (a self-test that ran no control is
not a pass — step 5, one level down); and stays offline (L4). **The wrapper is #510's to write, not
A-iii's** (the #519 boundary rule); A-iii defines the contract and, through step 3(b), makes it the only way
in. Under A1 the same PR removes that self-test from `trip-wires`. A second known candidate fitting the same
contract: the stale-claim lane's `claim_gate_selftest.py` (`refs/backup/stale-claim-detector`, parked) — no
action here; it meets step 3(b) when it lands.

#### §4.2.1 The job boundary — stated as a property

The user's A1 (2026-09-22) reads: "`trip-wires` keeps a floor of shell, `git` and `grep`; a check that needs an
interpreter lives in A-iii's `tools` job". A1 itself puts `grep` — whose behaviour differs across GNU and BSD
builds — in the floor, so its criterion is not "the tool's version can matter" but **"the check needs a
language runtime"**. Definition: an **interpreter** is a language runtime (`python3`, `node`, …) whose version
the check's result depends on; the POSIX text utilities (`grep`, `sed`, `awk`) are part of the shell floor.
Consequence: **no existing wire moves** — `main`'s `layout-box-reader-trip-wire.sh` and
`wasm-runtime-trip-wire.sh` and #527's K2 wire use `awk` and stay in `trip-wires`
(`git grep -nwE 'awk|python3?|node|perl' origin/main -- '.claude/tools/*trip-wire*.sh'`); #510's `python3` wire is
the one that moves.

The rule, as three properties:

- **`trip-wires` declares no interpreter** — it has no setup step.
- **`tools` declares its interpreter** — `actions/setup-python`, on every leg.
- **A check that needs an interpreter belongs to the job that declares one.**

The `tools` job's comment states these rules **self-contained**: `trip-wires`' no-setup-step property in its
own words, the three properties above, and §8's timeout re-derive rule. PR #501's `trip-wires` comment (which
replaces two tool enumerations with the no-setup-step property) is cited as the source only — when A-iii lands
first, that text is not on `main`.

#### §4.2.2 Why no filter

Earlier drafts of the merged Slice A specified a `tools` path-filter set. A-iii drops it, on #496's argument:
gating would make the tamper path of a gate an allowlist entry someone must keep current — a pull request
editing only a suite's input would skip the job that reads it, the failure this slice exists to remove — and
#496 already answered the same question for `trip-wires` (one issue, one way). The trigger is the workflow's:
`on: pull_request: branches: [main]` with no `paths:` / `paths-ignore:` (`git show
origin/main:.github/workflows/ci.yml`, lines before `jobs:`), and Q8 pins that block as well as the job.
**Accepted behaviour delta**: every pull request targeting `main`, a Dependabot bump included, runs the
suites (§5 row 2).

### §4.3 The network question — answered by construction

The script runs each root's interpreter under an **offline regime**: environment half — `http_proxy` /
`https_proxy` at a closed local port, `NO_PROXY` / `no_proxy` removed (an inherited `NO_PROXY=*` would bypass
the proxy), `XDG_CACHE_HOME` a fresh empty directory; in-process half — `urllib.request.urlopen` replaced by a
raiser before any member loads, and a `subprocess` spawn whose argv names the `webref` entry script refused
(the in-process half catches a lookup the proxy alone does not, Codex R16; `webref`, not every spawn, because a
wrapper legitimately spawns its check). A wrapper's child inherits the environment half only. So **every green
run is the offline measurement**, and T-net proves each half can fire. At the base, no suite calls `urlopen` or
imports `subprocess` (`git grep -nE 'urlopen|import subprocess|from subprocess' origin/main --
':(glob)**/test_*.py'` matches nothing) — a statement about the test files only; what the modules under test
do is the regime's to catch.

⚠ **What A-iii does *not* claim**: that the gate becomes offline-capable. `verify_citation` shells out to
`webref heading`, which issues a conditional GET, so **`origin/main`'s gate requires the network in default
mode**. A-iii's claim is exact: *the suites run green offline*.

⚠ **The umbrella's earlier record** — that wiring the suites into CI takes a live-network dependency — was
measured on a branch whose catalog fall-through (Slice **B**'s) was the thing fetching. B ships the offline
contract for what B introduces; A-iii inherits no such dependency.

### §4.4 The interpreter floor — measured by one leg, stated by one assertion

The floor is **3.9**, and the **only measurement of it is the `"3.9"` matrix leg** running the set. Draft 1
also pre-checked it statically (an `ast` read for PEP 604 unions and 3.10+ runtime names); that read is
dropped. It cannot enumerate newer-runtime APIs (Codex R21), and on `main` it reports a false positive —
`plan-xcheck.py`'s `flip | nonflip` is a set union of two lowercase names, read as a PEP 604 union. Two
instruments for one property, one of them wrong in both directions, is the shape *one issue, one way* removes.
The script's assertion and the job's `python3 -VV` echo remain the *statement*. (§4.2 step 3's parse of every
tracked `.py` is a different property — a file the suites' complement check cannot read — not a floor
instrument.)

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

The docs that list `mise run ci`'s parts or the CI jobs change in the same PR (located by
`git grep -nE 'mise run ci|つ目の job' origin/main -- CLAUDE.md README.md .claude/skills/pre-push/SKILL.md`):

- **`CLAUDE.md` `### Workflow`**, "Push 前": the `mise run ci (…)` list gains `tools-test` after
  `trip-wires`.
- **`CLAUDE.md` `## Commands`**, "その他:": gains `tools-test` (= `scripts/python-suites.sh`, the CI `tools`
  job's script) beside `trip-wires`.
- **`CLAUDE.md` `## CI`**: after the paragraph that calls `trip-wires` "4 つ目の job", a paragraph for the
  fifth job `tools` — not gated, two `setup-python` legs (`"3.9"` floor, `"3.x"`), `bash
  scripts/python-suites.sh` = `mise run tools-test`, the tamper-path reason (§4.2.2) and the L6 boundary
  (§4.2.1); the "⚠ 買えるものの範囲" paragraph is extended to cover `tools` (red on the PR, not a merge
  block).
- **`.claude/skills/pre-push/SKILL.md`** Stage 2: the `mise run ci` comment and the serial fallback chain gain
  `trip-wires` and `tools-test` (both lack `trip-wires` today — the same enumeration, fixed where it is
  touched).
- **`README.md`**: the `mise run ci` comment's list gains `trip-wires` and `tools-test` (same reason), and the
  task list gains a `mise run tools-test` line.

---

## §5 Behaviour deltas

| # | Input | `origin/main` | After A-iii |
|---|---|---|---|
| 1 | a `.claude/**`-only PR targeting `main` | `changes` and `trip-wires` only; no suite runs | + the `tools` job |
| 2 | a `crates/**`-only PR targeting `main` | `check`/`doc`/`deny` + `trip-wires` | + the `tools` job (ungated) |
| 3 | `mise run ci` locally | does not run the Python suites | runs them via `[tasks.tools-test]` |
| 4 | a new tracked `test_*.py` anywhere in the repo | runs nowhere | runs; a failure is red |
| 5 | a tracked `test_*.py` that loads no test, beside a passing sibling | runs nowhere | **script fails loudly** naming the file |
| 6 | a tracked `.py` defining a `TestCase` outside the set, or a `--self-test` entry point (step 3(b): `__main__` guard + `ast.Constant` `'--self-test'`) named by no member | nothing | **script fails loudly** naming the file |
| 7 | the suites, offline | n/a — nothing runs them | every run is offline (L4) |

**Newly-red**: 4 (a failing suite), 5 and 6 — A-iii adds a job; it does not change what any existing job
asserts. Rows 5 and 6 are green at `origin/main`: every member contributes tests, no `TestCase` is outside the
set, and no tracked `.py` contains `--self-test` (`git grep -l -e '--self-test' origin/main -- '*.py'` matches
nothing; `git grep -l TestCase origin/main -- '*.py'` lists only set members).

---

## §6 Pins

**Every pin is a test in `scripts/test_python_suites.py`**; "Red at main" says whether it fails on
`origin/main`, the base. **No pin runs the driver on the live tree** — the driver runs this file, so such a pin
would recurse. Every pin that executes the driver builds a **scratch repository** (`git init` in a temporary
directory, the driver copied in, fixture files staged with `git add`), so the live tree's properties are the
driver's own checks on every run (§4.2 steps 1–5, §4.3), and the pins prove each of those checks can fire.
Pins that read `ci.yml` or `mise.toml` read text: the floor interpreter has neither a YAML parser nor
`tomllib` (3.11+) in its stdlib.

| Pin | What it executes | §5 rows | Red at main? |
|---|---|---|---|
| **Q1** | a scratch repo with passing members in two directories exits 0, and the output names **every member** with a nonzero test count — compared against the scratch repo's `git ls-files ':(glob)**/test_*.py'`, not against a list | 3, 4 | **yes** — the script does not exist |
| **Q2** | a **failing** `test_planted.py` in a directory holding no other member makes the script exit non-zero naming it — a failing plant, because a passing one would also pass by being skipped; a directory with no other member is reached only because the roots are derived | 4 | **yes** |
| **Q3** | a `test_empty.py` defining no test, **beside a passing sibling** in the same directory, makes the script exit non-zero naming `test_empty.py`; and a scratch repo with no tracked `test_*.py` exits non-zero (the empty set) | 5 | **yes** |
| **Q9** | non-recursion: a package directory (`__init__.py` + a passing member) with a passing member in its parent directory — every test id appears **exactly once** in the run | 4 | **yes** |
| **Q10** | complement (a): a tracked `helper.py` defining a `unittest.TestCase` subclass, no `test_*.py` name, makes the script exit non-zero naming it | 6 | **yes** |
| **Q11** | complement (b): a tracked `tool.py` with a top-level `if __name__ == "__main__":` guard and an `ast.Constant` `'--self-test'`, named by no member, makes the script exit non-zero naming it; **controls**: adding a passing `test_tool.py` whose source names `tool.py` turns the run green, and a tracked `helper.py` carrying the same constant **without** the guard is outside the predicate — green with no member naming it. And a tracked `.py` that does not parse makes the run exit non-zero naming it | 6 | **yes** |
| **Q4** | `mise.toml`'s `[tasks.ci].depends` contains `"tools-test"`, and `[tasks.tools-test]` runs `bash scripts/python-suites.sh` (text read, section-bounded) | 3 | **yes** |
| **Q5** | the floor assertion: the script exits non-zero naming the floor when the `python3` it finds reports a version below 3.9 (fixture: a PATH-shadowing `python3` shim) | — | **yes** |
| **Q6** | `ci.yml`'s `tools` job region — from the line `  tools:` (plain or quoted key) to the next line at two-space indent that is not blank, a comment or a list item; the rule is stated in the test itself — contains a **`run:` step that executes the driver**: after shell word-splitting, some command line of a `run:` scalar has an argv token resolving to `scripts/python-suites.sh`, or is `mise run tools-test`. The token *anywhere* in the region is not enough: `env: DRIVER: scripts/python-suites.sh` runs nothing (Codex R48). Without it an empty `tools` job satisfies §12(3) (Codex R14/R36) | 1, 2 | **yes** — the job does not exist |
| **Q7** | the same region carries a `python-version:` matrix with legs `"3.9"` and `"3.x"` **and a `uses: actions/setup-python` step whose `python-version:` input is `${{ matrix.python-version }}`, before Q6's `run:` step** — a matrix no step consumes runs one interpreter on every leg (Codex R48), and a `setup-python` after the driver step measures nothing (Codex R49) | — | **yes** |
| **Q8** | the `tools` job is **present and ungated**: its region is non-empty and carries no `needs:` and no `if:` line — deliberately not "not gated on a known output", which would read a job gated on a *new* output as ungated; **negative control**: the same predicate reads `check` (`needs: changes` + `if:` on `rust`) as gated (Codex R49). **And the trigger**: the workflow-level `on:` block's `pull_request` carries no `paths:` / `paths-ignore:` and its `branches` is exactly `[main]` | 1, 2 | **yes** |
| **T-net** | in a scratch repo, three plants each make the script exit non-zero naming the plant: a member calling `urllib.request.urlopen`; a member spawning the `webref` entry script; a member asserting its environment — run with an inherited `NO_PROXY=*` and a real proxy setting, it must see the closed-port proxy, no `NO_PROXY`/`no_proxy`, and an empty `XDG_CACHE_HOME`, so it fails if the regime is absent. **Control**: a member spawning a non-`webref` child via `sys.executable` passes | 7 | **yes** |

⚠ **UNCHECKED, marked not omitted**: that a red `tools` job **blocks** a merge — **false**, §4.5; the
interpreter floor on `SKILL.md`'s direct `preflight.py` path, which bypasses the script — **assigned to Slice
B's floor raise** (§4.4).

---

## §7 Layering check

**VM host/ / ECS-native** — not applicable.

**Generic core vs elidex adapter.** A-iii touches **no `_webref` file** and adds **exactly one `test_*.py`,
under `scripts/`** (its pins). The implementation PR's `git diff --name-only origin/main...HEAD` is
`scripts/python-suites.sh`, `scripts/test_python_suites.py`, `mise.toml`, `.github/workflows/ci.yml`,
`CLAUDE.md`, `.claude/skills/pre-push/SKILL.md`, `README.md`, and this memo.

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

**The budget half.** A-iii takes **#510's share** of it: #510's self-test moves into `tools` (A1), so its cost
is `tools`' timeout, re-derived by #510 under the rule above. The other claimants — #527's K2 wire and the
stale-claim lane's `claim-provenance-trip-wire.sh` — are shell-only and stay in `trip-wires`' budget. So the
budget half is **closed for `tools`** and remains `trip-wires`' own question.

*Local measurements — claims, machine-dependent, for sizing only:* PR #510 at `06926607` (its head has since
moved; the frame is this commit), `plan-memo-umbrella-check.py --self-test` 8.7 s (Python 3.14.7) / 23.6 s
(Python 3.9.6); with `--mutants` 53 s / 131 s. The 3.9 leg is the slower one, by the ratio these imply on that
machine.

---

## §9 Edge-dense assessment

**(i)** The umbrella (on #501) names A-iii and its scope; the order change is §0's, carried into the umbrella
at #514's rework.

**(ii)** L1–L6 are **configuration** invariants with one observable (does the job run, and is it green). This
is the least dense of the three A slices and would not trip the edge-dense trigger on its own; it is a
separate slice because it was **invisible** inside the merged memo — the one cross-lane collision that memo
missed lived here.

---

## §10 Open questions

- **Q1 — does `required_status_checks` belong in this PR?** The `pull_request` rule carries
  `required_approving_review_count: 0` **and** a `RepositoryRole` bypass with `bypass_mode: always`
  (`gh api repos/send/elidex/rulesets --jq '.[] | select(.name=="main-protection") | .id'`, then that ruleset's
  `.bypass_actors`), so adding it leaves it author-bypassable: visibility-plus-friction, not enforcement.
  **Recommendation: register a slot, do not implement** (§11).
- **Q2 — closed (draft 3):** the zero-test grain is per file (§4.2 step 5).
- **Q3 — closed (draft 3):** the legs are `"3.9"` and `"3.x"`, and `setup-python` serves 3.9 on the current
  Ubuntu images (§4.4).

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

**(4) Offline:** T-net, and (1) — every green run is under the regime (§4.3).

**(5) The job runs the driver:** Q6. (3) alone is satisfiable by a job that runs nothing.

**(6) The floor is measured:** Q7 green and the `"3.9"` leg green on the PR page — the one measurement (§4.4).

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** ([PR #514](https://github.com/send/elidex/pull/514), stacked on #501) | none in A-iii's files | **A-iii first** (§0, D1). **#514 rework checklist** (this lane): rebase onto landed `main`; replace §3's fixture table here with A-ii's `No spec surface` declaration; amend the umbrella's slice order and A-ii memo's §13 row "A-iii \| after A-ii" (§0) |
| **[PR #510](https://github.com/send/elidex/pull/510)** (VM P4) | `ci.yml`, `mise.toml`, `scripts/trip-wires.sh`, `CLAUDE.md` `## CI` | **A-iii first**; #510 then adds its §4.2 wrapper, drops its self-test from `trip-wires` (A1), and re-derives `tools`' timeout (§8). Step 3(b) **requires the wrapper**: on #510's branch the `tools` job is red until a member names the self-test's file — the intended forcing function |
| **[PR #501](https://github.com/send/elidex/pull/501)** | `ci.yml` (the `trip-wires` comment and timeout), `CLAUDE.md` `## CI`; adds `test_spec_labels.py`, which §4.2 picks up with no edit | either order; the umbrella amendment rides #514's rework (§0) |
| **[PR #527](https://github.com/send/elidex/pull/527)**, **[#381](https://github.com/send/elidex/pull/381)**, **[#502](https://github.com/send/elidex/pull/502)** | `ci.yml` (#527 also `scripts/trip-wires.sh` and `CLAUDE.md`; the two Dependabot bumps `steps:` only) | whichever lands second adapts textually |
| **Slices B / C** | after A-iii these are scheduled from their first commit; **B** owns the floor raise and with it `SKILL.md` Step 0's direct `preflight.py` invocation (§4.4) | after |

⚠ **Stacked lanes do not get the job yet.** `ci.yml` triggers on pull requests targeting `main` only, so #514
and #527 (base `webref-cite-audit-tool`; `gh pr list --state open --json number,baseRefName`) run no `tools`
job until #501 lands and they retarget `main`.

**The lane facts are derived, not listed.** Before landing, the driver re-runs the sweep over every open PR and
reads it by line, not by file:

```sh
gh pr list --state open --limit 200 --json number --jq '.[].number' | while read n; do git fetch -q origin "refs/pull/$n/head" && git diff --stat origin/main...FETCH_HEAD -- .github mise.toml scripts ':(glob)**/test_*.py' CLAUDE.md README.md .claude/skills/pre-push/SKILL.md | sed "s/^/#$n /"; done
```

It ranges over the files A-iii contends on in every open PR, not over `docs/plans/` — the shape that hid #496
from the merged memo.

**Landing checklist**

1. Re-run the sweep above; a PR it shows on an A-iii line and not in the table gets a row before landing.
2. §4.6's docs are updated in the same PR.
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
- Draft 2 (2026-10-24): D1 reorder; D2 derived non-recursive roots + wrapper contract; D3 both legs run
  `--mutants`, timeout rule from #501; A1 boundary as a property; static floor pre-check dropped; #496 landed;
  §3 switched to the fixture table main's preflight accepts.
- Draft 3 (2026-10-24): plan-review R1 findings F1–F16 applied — the re-derivation harness removed (the
  2026-08-23 rule; every claim a §6 pin or a command), pins in `scripts/test_python_suites.py` and a repo-wide
  set, per-file zero-test grain, complement checks by property, offline regime in the script, L5/L6 and all
  15 pairs, §4.6 docs, trigger-level Q8, A1's interpreter definition, legs `"3.9"`/`"3.x"`, PM-owned SoT.
