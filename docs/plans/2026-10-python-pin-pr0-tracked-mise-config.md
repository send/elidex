# PR-0 plan — the tracked-tree rule T5, clauses 2–5 (python-pin program, draft 1, 2026-11-03)

## §0 Status

**Lane**: CI/tools. **Program**: the Python interpreter pin, umbrella `docs/plans/2026-10-python-pin-umbrella.md`
(landed by #581, `ea12ce7f`) with its companions `…-umbrella-records.md` (R§n), `…-umbrella-rounds.md` (§12–§14,
C-items) and `…-umbrella-vm-p4.md`. "Body :n" below is a line of the umbrella body, "rounds :n" of the rounds file.
**Base**: `origin/main` at **`115a667c`** (#585). Between #581 and the base one PR landed (`git log --oneline
ea12ce7f..115a667c`): #585, which added the `check-default` mise task and so moved the `tools-test` description from
`mise.toml:115` to `mise.toml:122` (`git show ea12ce7f:mise.toml | grep -n -F 'Run every tracked'` prints 115; the same
at `115a667c` prints 122). #585 also edited the umbrella body and rounds (the `mise.toml:115` → `:122` cites of §6, §11 and C11, and two
`ci.yml` line cites; `git diff ea12ce7f 115a667c -- 'docs/plans/2026-10-python-pin-umbrella*.md'`);
body :393 keeps `:115` "at `ee6c7a1a`", which is still true of that base.

**Order and preconditions** (body :563 row, :567–568; PM decisions 141, 233): #581 merged (done) → **PR-0** →
PR-1 → the PR-1 → PR-2 interval → PR-2. PR-0 needs this plan's `/elidex-plan-review` before implementation (body
§9 step 1). PR-0 touches **no VM P4 file and no pin** (body :563, *Depends on*), so it needs no consent.

**Edge-dense: yes — PR-0 is not a single intersection** (decision D1, §4.1). It carries its own pair matrix (§2)
and its own plan-review, and it is a terminal unit under CLAUDE.md *Edge-dense work* (c).

**No ECS surface, no spec surface** (§3). **Measured digits**: every line number below is re-measurable at
`115a667c` with the command beside it; no count here is a live claim beyond its command.

Provenance of the delegations this plan answers (each prints the line cited):
```sh
U=docs/plans/2026-10-python-pin-umbrella.md; R=docs/plans/2026-10-python-pin-umbrella-rounds.md
git grep -n -F "Whether PR-0 is a single intersection" 115a667c -- $U                          # :100
git grep -n -F "sufficiency as PR-0's whole scope" 115a667c -- $U                              # :313
git grep -n -F "its plan's call (C11), not asserted here" 115a667c -- $U                       # :658
git grep -n -F "PR-0's plan records why it goes there or names its own module" 115a667c -- $U  # :366
git grep -n -F "is answered yes by PR-0's plan" 115a667c -- $U                                 # :627
git grep -n -F "PR-0's plan confirms" 115a667c -- $U                                           # :997
git grep -n -F "PR-0 rewords it truthfully" 115a667c -- $U                                     # :394
git grep -n -F "PR-0's plan reads the reader and its pins first" 115a667c -- $U                # :999
git grep -n -F "C11 — PR-0's plan is new" 115a667c -- $R                                       # :414
git grep -n -F "C29 — PR-0's plan, T5's home" 115a667c -- $R                                   # :551
git grep -n -F "C21 — PR-1 §4.5" 115a667c -- $R                                                # :490
```

---

## §1 What PR-0 holds

T5 (body §4.2, :363–398) is the tracked-tree half of I1 (*one spelling*): over `git ls-files`, a fail-safe rule on
tracked **names and text** that does not model mise's configuration surface (PM decision 150 (1)). PR-0 lands its
clauses 2–5 — members, red outright, no `python` outside admitted occurrences, the diagnostic — and the one
reword of `mise.toml:122` that clause 4 needs on `main`. PR-1 adds clause 1 (the one-line root `.tool-versions`) and
the two `[settings]` lines **without editing** clauses 2–5 (body :613–633, *PR-0 rebuilt*). The ideal this
serves (body §1): the version is spelled once, and a tracked second spelling that local mise honours and CI does
not is red in CI, not discovered at the next local run (T5 job (i), body :358–361).

---

## §2 Invariants and the intra-PR pair matrix

PR-0's one cell of the umbrella's §2 table is **I1 × I6's admitted lines** (body :291: "the two admitted
`[settings]` lines (clause 4 (b)), ahead of PR-1"). PR-0 also meets three invariants that hold on `main` and that
the umbrella names as the reason the question is open (body :100–102): A-iii's offline regime, `main`'s two
`tools` legs and A-iii's `mise.toml` reader. The axes:

- **T** — T5 clauses 2–5 (I1's tracked half before the pin).
- **S** — I6's two admitted lines, `python.compile = false` / `python.github_attestations = true` (clause 4 (b)).
- **G** — A-iii's offline regime and its launch accounting: every process a pin file starts goes through
  `_launch` (`scripts/python_suites_fixtures.py`, pinned by G9 in `scripts/test_python_suites_regime.py`), with
  every `GIT_*` variable dropped (`base_env`); P6 holds the pin-file list `PIN_FILES` closed under imports.
- **L** — `main`'s two legs, `"3.9"` and `"3.x"` (`.github/workflows/ci.yml`, `git grep -n -F 'python-version: ["3.9"'
  115a667c -- .github/workflows/ci.yml` prints `:266`): every member runs on both, so T5's code is 3.9 stdlib.
- **M** — A-iii's `mise.toml` reader (`mise_sections` / `mise_problems` in `scripts/python_suites_readers.py`, pins
  Q4 / M1–M3 in `scripts/test_python_suites_ci.py`), which reads `[tasks.tools-test]` — the section the reword
  edits.

| pair | intersection in PR-0 | mechanism | held by |
|---|---|---|---|
| T × S | clause 4 admits the two lines as exact text in the root `mise.toml` only, ahead of PR-1 | `ADMITTED_LINES`, exact-line equality (§4.2) | C4-g3, C4-r6, C4-r7 (§6) |
| T × G | T5's tracked list is its own `git ls-files`, launched under the regime and accounted | the member's live test calls `_launch([...git ls-files -z], env=base_env(), purpose="git")`; both new files join `PIN_FILES` | L1, L2 (§6); G9 and P6 unchanged |
| T × L | T5 must compile and run on 3.9 | stdlib only, no construct newer than 3.9 (no `match`, no `itertools.pairwise`, no PEP 604 unions at run time) | the `"3.9"` leg running the new member |
| T × M | the reword stays inside the reader's grammar (a one-line basic string, no escape) and clause 4 reads the same line | one edited line; nothing pins its text (D4) | Q4 live (existing), L1, C4-r1 |
| G × L | `git ls-files` from a member under the regime, on both legs | precedent: P6 (`git grep -n -F 'def test_P6_pins_file_never_credits' 115a667c -- scripts/test_python_suites_driver.py` prints `:259`) already does it on both legs; `main`'s CI at `115a667c` is green (`gh run list -R send/elidex --branch main --workflow ci.yml --limit 1`) | P6, L1 |
| S × M | — in PR-0 (PR-0 writes no `[settings]` line); PR-1's writes × clause 4 is PR-1's slice-local pair (C9, C21) | — | PR-1 |
| S × G, S × L, G × M, L × M | — (S has no text on `main`; the reader already runs on both legs; the reader launches nothing) | — | — |

Four non-empty pairs over four axes (T, S, G, L, M with T in each), so the ≥3-axis trigger holds: D1 below.

---

## §3 Spec coverage map

**No spec surface** — a tracked-tree rule over file names and text, a test member, one task description. As for
the umbrella (body §3), `main`'s preflight exits 1 on this declaration until A-ii's marker recognition lands.

---

## §4 Design

### §4.1 The four delegated decisions

**D1 — single intersection? No.** Delegated by body :100 ("Whether PR-0 is a single intersection is **PR-0's
plan's call**"), :313 and :656–658 (C11, rounds :425–428). Answer: PR-0 meets five axes and four non-empty pairs
(§2), each of which constrains a design choice in this plan — the list source (T × G), the language level (T × L),
the edited line (T × M), the exact admitted text (T × S). That is CLAUDE.md *Edge-dense work* trigger (a), so
PR-0 is edge-dense, carries §2 as its own matrix and passes its own plan-review. It is terminal under (c): a
plan-reviewed, narrowly scoped slice of the approved umbrella. Its diff review does not invoke the base-case
exclusion of `.claude/skills/elidex-review/axes.md` ("scope が単一 invariant-axis 交点に絞られている"; `git grep -n
-F 'Base case 除外' 115a667c -- .claude/skills/elidex-review/axes.md` prints `:139`); the entry's firing condition (no
trace of the split) is unmet, because the umbrella and this plan are the trace — as body :650–653 says for PR-1.
Reason against "single": calling it single would let the diff review skip the regime and the 3.9 leg, which
are exactly where a member that launches `git` and reads the tree can break `main`.

**D2 — T5's home: its own module pair, not the readers module.** Delegated by body :366 ("PR-0's plan records why
it goes there or names its own module"), :563 and rounds :551 (C29). Answer: a new non-member module
**`scripts/python_suites_tracked_config.py`** (the predicate, pure) and a new set member
**`scripts/test_python_suites_tracked_config.py`** (its controls and the live run). Reasons:
1. **Different contract.** `scripts/python_suites_readers.py` *models* a grammar subset and raises `ReaderError`
   on anything outside it (its docstring, `:1-7`). T5 by decision 150 (1) models nothing; it is fail-safe over
   names and text. Co-locating a non-modelling rule beside two modelling readers puts the retired shape (draft 4's
   T5b key classification) one function away — the decision surface CLAUDE.md *One issue, one way* asks to remove.
2. **PR-1 rewrites the readers module and must not edit clauses 2–5.** PR-1 moves the `mise.toml` reader onto
   `tomllib` (the text grammar retires; body :728–729) and adds clause 1. With T5 in its own module, "PR-1 adds
   clause 1 without editing clauses 2–5" (body :625) is checkable from PR-1's diff (`git diff` of
   `scripts/python_suites_tracked_config.py` adds lines only); inside a module PR-1 rewrites, it is not.
3. **Different subject.** The readers read two named files (`ci.yml`, `mise.toml`); T5 is a predicate over the
   tracked tree (body :365–366).
Size is not a reason: either home stays under 1000 lines (§7). Names: neither new path has a component starting
`mise` / `rtx`, so neither is a T5 member (clause 2) — required, since both hold `python` throughout.
**Launcher-passed data**: does not arise — under D3 the member takes its own list, so T5 reads no data its
launcher passes. (Had D3 gone the other way, the driver would have had to pass a list over a new channel, a direct
run of the member would see none, and a control would have had to assert that a missing list is red; none of
that is built.)

**D3 — the file list: the member's own `git ls-files`.** Delegated by body :625–628 ("answered yes by PR-0's plan;
if no, PR-0 takes its file list from the driver … and PR-0's plan says which"), :997 and rounds :425 (C11).
Answer: **yes** — a suite member may run `git ls-files` under A-iii's regime on both legs. Grounds: (i) the
regime's keys are `PYTHONPATH`, the proxy keys, `NO_PROXY` and `XDG_CACHE_HOME`, plus every `GIT_*` dropped
(`scripts/python-suites.sh:66-73`); its behaviour half denies socket connects and name lookups in Python
processes. `git ls-files` is a local index read by a non-Python child; it opens no socket and is not in the
unguarded-child class (it changes no regime key). (ii) **It already runs**: P6 (`scripts/test_python_suites_driver.py:259-262`)
launches `git -C REPO ls-files -z` through `_launch` with `base_env()` on the live tree, on both legs of every
`tools` run since it landed. (iii) The driver's own list cannot serve: it lists only the set and the `.py`
population (`git grep -n -F 'members = git_lines(root, "ls-files"' 115a667c -- scripts/python-suites.sh` prints
`:435`; `ALL_PY_PATHSPEC` at `:439`), not every tracked path, so taking it would mean a new driver channel and an
edit of `scripts/python-suites.sh`, the file PR-1's Step 1 edits.

**D4 — the reword of `mise.toml:122`.** Delegated by body :394, :563, :621–622, :998–999 and rounds :414–417 (C11);
its successor is PR-1's (rounds :490, C21). **The reader first** (body :999): `mise_problems` checks
`[tasks.tools-test]` for its key set (`description`, `run`; `sources` / `outputs` red) and for `run == "bash
scripts/python-suites.sh"`; it never reads the description's value (`scripts/python_suites_readers.py:659-677`). Its
pins Q4 / M1–M3 (`scripts/test_python_suites_ci.py:104-110`, `:315-374`) build their own fixtures with
`description = "suites"` and read the live file only through `mise_problems`. No tracked file quotes the line
(`git grep -n -F 'Run every tracked' 115a667c` prints only `mise.toml:122` and the umbrella's own `:393` quote, a
record of `ee6c7a1a`). So the reword breaks no pin. Its text, the whole line:
```toml
description = "Run every tracked suite (test_*.py), offline, on the interpreter first on PATH (floor 3.9)."
```
Truthful: the driver runs the `python3` first on PATH and asserts ≥ 3.9 (`scripts/python-suites.sh:17-19`, and Step
1); the comment under it (`:123-128`) still points to the driver's header for the procedure. It keeps the reader's
grammar (one-line basic string, no backslash). After it, `git grep -n -i python 115a667c -- mise.toml` would print
only `:125` and `:129`, both inside the tracked path `scripts/python-suites.sh` (clause 4 (a)). This line is C21's
"before" for PR-1.

### §4.2 T5 clauses 2–5, as implemented

All name tests are on root-relative POSIX paths as `git ls-files -z` prints them. `fold(c)` = `c.casefold()`
with **one** leading `.` removed. Case-folding everywhere is the fail-safe direction: macOS's default file system
is case-insensitive, so `.Tool-Versions` is opened by a lookup of `.tool-versions`.

**Clause 2 — members.** A path is a member iff (a) some `/`-component `c` has `fold(c)` starting with `mise` or
`rtx`, or (b) its basename, case-folded, equals `.tool-versions` or starts with `.python-version`. (Body :373–379,
with (b) case-folded as well.) Members at `115a667c`: `mise.toml` only (`git ls-files | /usr/bin/grep -iE
'(^|/)\.?(mise|rtx)[^/]*(/|$)'` prints `mise.toml`; `git ls-files | /usr/bin/grep -ciE
'(^|/)(\.tool-versions|\.python-version[^/]*)$'` prints 0).

**The root `.tool-versions`** — exactly the path `.tool-versions`, case included — is a member that clauses 3 and 4
skip (left to clause 1, PR-1; body :372). Any other spelling at the root (`.TOOL-VERSIONS`) is not it.

**Clause 3 — red outright** (evaluated before any text is read; a member red here gets no clause 4 finding):
- (3a) some component `c` with `fold(c)` starting `miserc` — mise's rc file chooses which files mise reads (H1);
- (3b) basename case-folded `.tool-versions`, path ≠ `.tool-versions`;
- (3c) basename case-folded starting `.python-version`.

**Clause 4 — no `python` in any other member's text.** For each member not skipped and not red by clause 3:
- **Read** its bytes from the worktree at the repository root (the same source as the driver's own parse step,
  `scripts/python-suites.sh:445-451`). If the path is a symlink, is missing, or is not a regular file (a gitlink
  directory), or its bytes do not decode as UTF-8: **red, unclassifiable** — the rule cannot see the text mise
  would read (body :158, I1: "whatever the rule cannot classify falls to red").
- **Lines**: the text split on `\n`, numbered from 1; one trailing `\r` is removed before comparison.
- **Occurrences**: every match of `python` under `re.IGNORECASE` in a line.
- **(4b) admitted lines**: if the path is exactly `mise.toml`, a line equal to `python.compile = false` or
  `python.github_attestations = true` — no indentation, no trailing space or comment — admits every occurrence in
  it. Where the line stands in the file (its table) is not read: PR-1 pins the table and values over parsed TOML
  (C10), and the interval states it (§8).
- **(4a) tracked-path substrings**: an occurrence at `[s, s+6)` is admitted iff some tracked path `P` that
  **contains `/`** occurs in the line at `[a, a+len(P))` with `a ≤ s` and `s+6 ≤ a+len(P)`, and
  the character before `a` is absent or not one of `A–Z a–z 0–9 _ . - / @ : + ~`, and
  the character after the span is absent, or not one of `A–Z a–z 0–9 _ - / @ + ~`, and, if it is `.`, the
  character after that `.` is absent or not one of `A–Z a–z 0–9 _`.
  The `/` requirement keeps a root-level tracked file named `python…` from admitting a bare `python` key; the
  boundaries keep `ubi:scripts/python-suites.sh`, `./scripts/python-suites.sh` and `scripts/python-suites.sh.bak`
  out while admitting the path in backticks, quotes, parentheses, before `.` / `,` at a sentence end and before a
  `:<line>` locator. Both are narrower than body :385's "inside a substring that is the path of a tracked file" —
  the fail-safe direction: what the narrowing turns red is fixed by the reword remedy.
- Any occurrence admitted by neither: **red**, one finding per (path, line).

**Clause 5 — diagnostic.** Findings are sorted by (path, line) and each is one string:
`T5 clause <n>: <path>[:<line>]: <what>[: <the line, stripped, cut at 120 chars>] -- <remedy>`. Remedies:
- 3a: "a tracked mise rc file is never admitted (it chooses which files mise reads); remove it from the index";
- 3b / 3c: "the Python version is spelled only in the root .tool-versions; remove this file (another tool goes
  into mise.toml [tools])";
- 4, occurrence: "the Python version is spelled only in the root .tool-versions; reword (a description need not
  say Python; a tracked path is admitted), or rename the file so no path component starts with mise / rtx; for a
  new install setting, extend clause 4 (b) together with I6's assertion in the same PR";
- 4, unclassifiable: "track the file itself as UTF-8 text (not a symlink), or rename it out of the member names".
The live test's failure message is the joined list; the driver prints a failing test's detail to stdout
(`_Result._bad`, `scripts/python-suites.sh`), so both callers show every finding, not the first.

### §4.3 The module's interface

`scripts/python_suites_tracked_config.py`, stdlib only (`os`, `re`), no process launch, no `TestCase`, no
top-level `__main__` guard, no `"--self-test"` constant (driver step 3 (a) / (b)):
- `is_member(path) -> bool` — clause 2;
- `tracked_config_problems(tracked, read) -> list[str]` — clauses 3–5 over `tracked` (an iterable of paths, deduplicated
  and sorted inside, since an index with unmerged entries lists a path once per stage) with `read(path)` returning
  the bytes of a regular file or `None`;
- `worktree_reader(root)` — the `read` above over a checkout (`None` for a symlink, a missing path or a non-file).
Constants `ROOT_PIN = ".tool-versions"`, `ROOT_CONFIG = "mise.toml"`, `ADMITTED_LINES` (the two lines). PR-1 adds
clause 1 here (its parser is `python_pin.py`'s, body :630–631) and leaves these unedited.

---

## §5 Edge matrix

"Red n" = red by clause n; "skip" = member, left to clause 1; "—" = not a member. 52 cases in 13 rows.

| # | case (path: content) | verdict | why |
|---|---|---|---|
| E1 | `mise.toml` live, after the reword | green | only `scripts/python-suites.sh` substrings (4a) |
| E2 | `mise.toml` with `main`'s `:122` line before the reword | red 4 at that line | "Python suite", "`python3`", "Python 3.9" |
| E3 | `.miserc.toml`, `.miserc`, `miserc.toml`, `sub/.miserc.toml`, `.config/mise/miserc.toml`, `.MISERC.TOML`, `x/.miserc.d/y` (any content, empty too) | red 3a | rc file |
| E4 | `sub/.tool-versions` (holding `node 20` or empty), `.Tool-Versions` at the root | red 3b | not the root pin file |
| E5 | `.tool-versions` at the root, holding anything (`python 3.12` included) | skip | clause 1 is PR-1's: the interval (§8) |
| E6 | `.python-version`, `.python-versions`, `sub/.python-version`, `.python-version.bak`, `.PYTHON-VERSION` | red 3c | idiomatic version file |
| E7 | `python-version` (no dot), `tool-versions` (no dot) | — | not read by mise as named; outside the rule (body :404–405 residual class) |
| E8 | python-spelling members: `.config/mise/conf.d/py.toml` / `mise.ci.toml` / `.rtx.toml` / `rtx.toml` with `[tools] python = "3.12"`; `mise/tasks/x` with a `#MISE tools=` python header; `mise.toml` with `[env] MISE_PYTHON_VERSION = "3.12"`, with `"core:python"`, with `python.compile = true`, with `[env] PYTHONOPTIMIZE = "1"`, with a comment `# Python`, with `PYTHON` | red 4 | an occurrence outside (a) and (b); `PYTHONOPTIMIZE` incidentally (R§9) |
| E9 | admitted lines: both exact in the root `mise.toml` (LF and CRLF) | green | 4b |
| E10 | admitted-line near misses: indented, trailing space, trailing `# c`, `python.compile=false`, the same exact line in `sub/mise.toml` or `.mise.toml` | red 4 | 4b is exact text in the root `mise.toml` only |
| E11 | path substrings: `"bash scripts/python-suites.sh"`, `` `scripts/python-suites.sh` ``, `(scripts/python-suites.sh)`, `scripts/python-suites.sh.` at a sentence end, `scripts/python-suites.sh:435` → green; `./scripts/python-suites.sh`, `ubi:scripts/python-suites.sh`, `scripts/python-suites.sh.bak`, `scripts/python-suites.sh@1`, an untracked `scripts/python_nope.sh`, a tracked root-level `python.txt` used as `python.txt = 1` → red 4 | as stated | 4a boundaries and the `/` rule |
| E12 | over-approximated names: `scripts/mise_helper.py` with `python` → red 4; without → green; `Mise/notes.md` → member; `..mise.toml` → —; `crates/css/elidex-css/src/declaration/misc.rs` and the four tracked `*promise*` files → —; nested `sub/mise.toml` with `[tools] node = "20"` only → green | as stated | clause 2's prefix and the one-dot drop; a non-Python tool is green (body :400–401) |
| E13 | tracked-state cases: an untracked `.miserc.toml` / `mise.local.toml` → —; a staged, uncommitted `.miserc.toml` → red 3a; a tracked member deleted from the worktree → red 4 (unclassifiable); a tracked symlink `mise.ci.toml` → red 4 (unclassifiable); a tracked symlink `.miserc.toml` → red 3a (clause 3 reads no text); a gitlink at `mise` → red 4 (unclassifiable); non-UTF-8 bytes in `mise.ci.toml` → red 4 (unclassifiable); a path listed twice (unmerged) → one finding | as stated | the list is the index (`git ls-files`); the text is the worktree's |

**Outside the rule, stated, no control** (body :404–414, §11): a TOML escape spelling `python` without its letters
(deliberate tampering, `#11-aiii-gate-tamper-hardening`); a tracked `[env] _.path` (T1, decision 170's limit); an
input mise reads under a name clause 2 does not match; every untracked input (I4 × I6). None that changes the
version is silent once PR-1's admission exists; before PR-1 nothing admits, so nothing regresses (§8).

---

## §6 Tests and controls

All in `scripts/test_python_suites_tracked_config.py`. Pure controls call `tracked_config_problems` with a path
list and an in-memory `read`; the two live / scratch tests launch only through `_launch`. Each control asserts the
**clause number and the path (and line)** in the finding, not only "non-empty". IDs are this plan's.

- **L1 (live, green)**: `git -C REPO ls-files -z` through `_launch(..., env=base_env(), purpose="git")`, exit 0;
  `tracked_config_problems(list, worktree_reader(REPO)) == []`; **non-vacuity**: the list is non-empty,
  `is_member("mise.toml")` is true and `"mise.toml"` is in the list (a list that silently came back empty must not
  read green).
- **L2 (scratch repository, red + green)**: a temporary `git init` with `.miserc.toml` staged and `mise.local.toml`
  and `.python-version` present but untracked; the same list-and-read path as L1 over it reports exactly
  `.miserc.toml` (3a). Kills a list taken from a directory walk (would add the untracked two) and one taken from
  `HEAD` (would miss the staged one).
- **C2 (members)**: red side — every E3/E4/E6/E8 path and `Mise/notes.md` is a member; green side — the E7 paths,
  `..mise.toml`, `misc.rs`, and the tracked `*promise*` paths of the live list (asserted ≥ 1, so the control is not
  vacuous) are not.
- **C3 (red outright)**: r1 E3's seven paths, empty content, each red 3a; r2 E4 each red 3b; r3 E6 each red 3c; r4
  a `.miserc.toml` whose `read` raises if called (clause 3 reads no text). g1 E5: root `.tool-versions` holding
  `python 3.12` is green and is never read (the interval, pinned so PR-1's clause 1 is the change that turns it).
- **C4 (content)**: r1 E2 — `main`'s line 122 text in a `mise.toml` fixture is red 4 at its line (the "red at main"
  of the reword); r2 E8's members, each red 4 at the line; r3 E11's red substrings; r4 E12's `mise_helper.py`;
  r5 E13's unclassifiable cases (symlink, missing, non-UTF-8, gitlink); r6 `python.compile = true`; r7 E10's near
  misses. g1 the live `mise.toml` text (also covered by L1); g2 E11's green substrings, with
  `scripts/python-suites.sh` in the fixture's tracked list and removed from it (then red: the admission depends on
  the list, not on the string's shape); g3 E9's two lines, LF and CRLF; g4 E12's nested `[tools] node` file.
- **C5 (diagnostic)**: one fixture with a 3a, a 3c and two clause-4 findings in two files: the result is sorted by
  (path, line), each string carries the clause, the path, the line for clause 4 and its clause's remedy; a line
  longer than 120 characters is cut.
- **Existing pins that must stay green unchanged**: Q4 / M1–M3 (the reader over the reworded live file), P6 and G9
  (with `PIN_FILES` extended by the two new files: `scripts/python_suites_fixtures.py`, its `PIN_FILES` tuple), and
  every other member.
- **Fixture text constraints**: no string constant equal to a tracked `--self-test` entry point's path (P6 reads
  every `PIN_FILES` string constant), no `"--self-test"` constant, no `TestCase` in the non-member module.

**Mutation.** The umbrella asks none of PR-0 (its mutation requirements are I7's, PR-1). Each control above is
written to kill a named wrong implementation; the implementer applies each of these once by hand and records in
the PR body which control turned red: M1 the list from a directory walk (L2); M2 case-sensitive names (C3 r1's
`.MISERC.TOML`, r3's `.PYTHON-VERSION`); M3 4a without the `/` rule (C4 r3's `python.txt`); M4 4a without
boundaries (C4 r3); M5 4b by `in` instead of equality (C4 r7); M6 clause 4 skipped for symlinks instead of red
(C4 r5); M7 the root `.tool-versions` compared case-insensitively (C3 r2's `.Tool-Versions`).

---

## §7 Scope

**Edit set**:
- `scripts/python_suites_tracked_config.py` — NEW (non-member module, §4.3);
- `scripts/test_python_suites_tracked_config.py` — NEW (set member, §6);
- `scripts/python_suites_fixtures.py` — `PIN_FILES` gains the two new paths (P6's closure demands it: the member
  imports the fixtures, and it imports the new module);
- `mise.toml` — the one line of D4;
- `docs/plans/2026-10-python-pin-umbrella.md` — the sentence at body :728–729 that PR-0 makes false ("PR-0 and PR-1
  both grow `scripts/python_suites_readers.py` … and `scripts/test_python_suites_ci.py`") becomes "PR-1 grows
  `scripts/python_suites_readers.py` (678 lines at `ee6c7a1a`) and `scripts/test_python_suites_ci.py` (378), and
  its `mise.toml` half shrinks the former (the text grammar retires); PR-0 adds its own module pair (PR-0's plan
  §4.1 D2)", rewrapped inside the same paragraph so the body stays ≤ 1000 lines (999 at `115a667c`: `wc -l`);
- this plan.

**Not touched**: `scripts/python-suites.sh` (D3), `scripts/python_suites_readers.py` and
`scripts/test_python_suites_ci.py` (D2), `scripts/test_python_suites_regime.py` and
`scripts/test_python_suites_driver.py`, `.tool-versions` (absent; PR-1), `scripts/python_pin.py` /
`scripts/python_pin_testing.py` (PR-1), every VM P4 family file (`.claude/tools/plan_memo_*`,
`.claude/tools/plan-memo-umbrella-check.py`, `docs/plans/2026-07-vm-p4-*.md`, `docs/plans/test_vm_p4_family_layout.py`),
`CLAUDE.md`, the umbrella's other three files, and `.github/workflows/ci.yml` unless Q1 (§11) rules otherwise.

**1000-line rule** (CLAUDE.md; body §7): `git ls-files -z -- mise.toml scripts/python_suites_fixtures.py
docs/plans/2026-10-python-pin-umbrella.md | xargs -0 wc -l` prints 153, 232 and 999 at `115a667c`; the new files are
small; nothing crosses 1000. The umbrella body's line count is re-measured after the :728 edit (≤ 1000, its K3 bound).

**No `python3 … test_*.py` launch line** in any landing file (C33, rounds :570): this plan describes the members
and never writes a direct launch of one; `git grep -nP 'python3?(\.[0-9]+)? +(-[A-Za-z]+ +)*(-m +unittest +)?(\S*/)?test_[A-Za-z0-9_]+\.py'
-- docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md` prints nothing.

---

## §8 Coordination and the PR-0 → PR-1 interval

**Lanes** (body §8): the overlap is the *every lane* row (body :816) — from PR-0, a tracked `.miserc*`, a non-root
`.tool-versions`, a `.python-version*`, or a `python` outside the admitted occurrences in any file whose path has a
component starting `mise` / `rtx`, is red. Textual overlaps: `mise.toml` (anyone editing `[tasks.tools-test]`;
whichever lands second re-reads), `scripts/python_suites_fixtures.py` (A-iii sweep 4 may edit it; textual).
Dependabot touches `ci.yml` `steps:` only — no overlap.

**Pre-landing sweep** (body :821–824): the driver re-runs A-iii §13's sweep with PR-0's contended paths added
(`mise.toml`, `scripts/python_suites_fixtures.py`, the two new files), and runs T5 on **each open PR's head**: the
list from `git ls-tree -r -z --name-only <head>`, `read` from `git cat-file blob <head>:<path>` for a regular blob
and `None` for modes 120000 / 160000, the head fetched into `FETCH_HEAD` only (no ref writes). Each red is handed
to that PR's lane through the PM before PR-0 lands; the PR body lists the heads checked and the result.

**The PR-0 → PR-1 interval** (body :635–642, restated for PR-0's shape). Holds: clauses 2–5 on every tracked
member, on both of `main`'s legs. Not checked: the root `.tool-versions` (skip, E5) — a root `.tool-versions` added
in the interval is green in CI whatever it holds, and local mise reads it; and the two `[settings]` lines are
admitted as text anywhere in the root `mise.toml` but unpinned (no table, no value beyond the exact text; a flipped
value is not the admitted text, hence red). Neither weakens an invariant that holds on `main`: `main` has no pin,
so no reader of a pin can disagree. Bounded by order: PR-1 is the lane's next pin PR (body :641–642).

---

## §9 Defer slots

None (body :869: "PR-0 §9 (none)").

---

## §10 Landing and DoD

**Landing steps**:
1. This plan passes `/elidex-plan-review` (body §9 step 1); implementation starts after it.
2. Implementation in a dedicated worktree off `origin/main`; `cargo fmt --all` is a no-op here; `mise run
   tools-test` green locally; `mise run ci` before push (CLAUDE.md *Workflow*).
3. The pre-landing sweep of §8; reds handed off.
4. CI: every job green, the `tools` job on **both** legs (the `"3.9"` leg is the only measurement of T × L).
5. **PM notice to every lane** with the landing (body :836–838, §9 step 2; §8 row *every lane*). Text to send:
   "From PR-0 (`<merge sha>`), `mise run tools-test` and the `tools` CI job are red when a **tracked** file is a
   mise rc file (`.miserc*`, any directory), a `.tool-versions` other than the repository root's, or a
   `.python-version*`; or when any tracked file whose path has a component starting `mise` / `rtx` (case-insensitive,
   one leading dot ignored — `mise.toml`, `.mise.local.toml`, `.config/mise/…`, `rtx.toml`, `mise_helper.py`)
   contains `python` in any case, except inside the path of a tracked file and the two `[settings]` lines PR-1
   adds. Untracked files are not checked. A `[tools]` entry for a non-Python tool is green. The root
   `.tool-versions` is unchecked until PR-1, which makes it exactly one `python X.Y.Z` line (a second tool goes
   into `mise.toml [tools]`). Each finding names the clause, the file and line, and the remedy. Open PRs found red
   at `<date>`: <list or none>."

**DoD**:
- [ ] the two new files and the `PIN_FILES` entries as §4.3 / §7; every §6 control as written, each asserting
      clause and path; L1 green; the existing pins green;
- [ ] `git grep -n -i python -- mise.toml` prints only lines whose occurrences lie in `scripts/python-suites.sh`;
- [ ] the umbrella body's :728 sentence amended, body ≤ 1000 lines;
- [ ] the X1 pattern prints nothing over this plan (§7); the set member `.claude/tools/test_plan_memo_umbrella_check.py`
      green under the driver (its `symbol_attribution_control` reads every `docs/plans/*.md`);
- [ ] the mutation record M1–M7 in the PR body; the sweep result in the PR body;
- [ ] CI green on both legs; the PM notice sent; Q1 answered and applied.

---

## §11 Deferred to PR-1 / PR-2, and open questions

**PR-1** (body :564 row): clause 1 (the one-line root `.tool-versions`, parsed by `python_pin.py`) — it needs that
parser, dead before PR-1 (body :630–633); the two `[settings]` lines and their parsed-TOML assertion (C10) — the
pin needs `tomllib`, absent on the `"3.9"` leg (R§6 table); the `tomllib` reader; the description's rewrite to the
pin (body §4.4; C21's "before" is D4's line); the slice-local pair "PR-1's `mise.toml` writes × clause 4" with a
control that the post-PR-1 `mise.toml` is green (C9, C21); the refusal, its oracle, the warning, I7. PR-1 adds
clause 1 to `scripts/python_suites_tracked_config.py` without editing §4.2's clauses.
**PR-2**: nothing of T5. The new module has no `__main__` guard and no launch line names the new member, so neither
is an entry point under I4.

**Open questions (this plan cannot settle them; to the PM)**:
- **Q1 — does PR-0 fire `ci.yml`'s timeout rule?** The `tools` job's comment says "a PR that adds suite cost
  re-derives `timeout-minutes` in the same PR from its own run of this job" (`git grep -n -F 'a PR that adds suite
  cost' 115a667c -- .github/workflows/ci.yml` prints `:243`); PR-0 adds a member. Precedents on both sides: #549 and
  #510 re-derived and recorded their runs in that comment; #506 (`3ec0ae5c`) added the member
  `docs/plans/test_vm_p4_family_layout.py` and #562 (`e44ffce6`) added self-test cases, both without a `ci.yml`
  edit (`git show --stat 3ec0ae5c e44ffce6 -- .github/workflows/ci.yml` prints nothing; umbrella-vm-p4 O3 records
  the value 22 unchanged since). If the rule fires, PR-0 edits only that comment (its run, and the value if the
  rule's margin requires) and `ci.yml` joins §7's edit set — textual with PR-1 and Dependabot, no ordering change
  (PR-0 precedes PR-1; the cargo-deny PR is after PR-2). If not, §7 stands.
- **Q2 — are the 4a narrowings and the unclassifiable red (§4.2) within the umbrella's T5?** This plan reads them
  as the fail-safe direction body §2 I1 prescribes ("whatever the rule cannot classify falls to red") and states
  both; the plan-review may rule that either belongs in the umbrella's §4.2 text, in which case the umbrella is
  amended in PR-0 beside :728.
