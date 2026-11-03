# PR-0 plan — the tracked-tree rule T5, clauses 2–5 (python-pin program, draft 2, 2026-11-03)

## §0 Status

**Lane**: CI/tools. **Program**: the Python interpreter pin, umbrella `docs/plans/2026-10-python-pin-umbrella.md`
(landed by #581, `ea12ce7f`) with its companions `…-umbrella-records.md` (R§n), `…-umbrella-rounds.md` (§12–§14,
C-items) and `…-umbrella-vm-p4.md`. "Body :n" below is a line of the umbrella body, "rounds :n" of the rounds file.
**Base**: `origin/main` at **`115a667c`** (#585). Between #581 and the base one PR landed (`git log --oneline
ea12ce7f..115a667c`): #585, which added the `check-default` mise task and so moved the `tools-test` description from
`mise.toml:115` to `mise.toml:122` (`git show ea12ce7f:mise.toml | grep -n -F 'Run every tracked'` prints 115; the same
at `115a667c` prints 122). #585 also edited the umbrella body and rounds (`git diff -U0 ea12ce7f 115a667c --
'docs/plans/2026-10-python-pin-umbrella*.md'`): four `mise.toml:115` → `:122` cites (body :563, :621, :999, rounds
:417) and four `ci.yml:167` → `:186` cites (body :84, :597, rounds :170, :209). It missed one: body :594's base-less
"(the `:115` reword)" has been false since #585; it sits in the bullet body :592–596, which PR-0 deletes (§7). Body :393 keeps `:115` "at `ee6c7a1a`",
which is still true of that base.

**Order and preconditions** (body :563 row, :567–568; PM decisions 141, 233): #581 merged (done) → **PR-0** →
PR-1 → the PR-1 → PR-2 interval → PR-2. PR-0 needs this plan's `/elidex-plan-review` before implementation (body
§9 step 1). PR-0 touches **no pin**, and a VM P4 file only if its timeout measurement raises the value (§7): then
it edits vm-p4 O3, deletes "no VM P4 file, " from body :563, and VM P4's consent goes through the PM before PR-0
lands.

**Edge-dense: yes — PR-0 is not a single intersection** (decision D1, §4.1). It carries its own pair matrix (§2)
and its own plan-review, and it is a terminal unit under CLAUDE.md *Edge-dense work* (c).

**No ECS surface, no spec surface** (§3). **Measured digits**: every line number below is re-measurable at
`115a667c` with the command beside it; no count here is a live claim beyond its command.

**PM decisions applied in draft 2** (PM 289, 294; plan-review R1): Q1 — PR-0 re-derives the `tools` timeout by
measurement (§7); Q2 — the narrowed clause 4 (a) and the unclassifiable red stay in this plan, the umbrella gains no
text (§4.2, §7). Design change of draft 2 (R1 A1-IMP, A2-IMP): the member is **not a pin-layer file** and lists the
tree itself, stdout only (§4.1 D2, D3).

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
the two `[settings]` lines **without editing** clauses 2–5 (body :613–633, *PR-0 rebuilt*): C0 keeps only the
names; that PR-1 edits no clause-2–5 function body is a condition checked in PR-1's review, per function span
(§4.3). The ideal this serves (body §1): the version is spelled once, and a tracked second spelling that local mise
honours and CI does not is red in CI, not discovered at the next local run (T5 job (i), body :358–361).

---

## §2 Invariants and the intra-PR pair matrix

PR-0's one cell of the umbrella's §2 table is **I1 × I6's admitted lines** (body :291: "the two admitted
`[settings]` lines (clause 4 (b)), ahead of PR-1"). PR-0 also meets three invariants that hold on `main` and that
the umbrella names as the reason the question is open (body :100–102): A-iii's offline regime, `main`'s two
`tools` legs and A-iii's `mise.toml` reader. The axes:

- **T** — T5 clauses 2–5 (I1's tracked half before the pin).
- **S** — I6's two admitted lines, `python.compile = false` / `python.github_attestations = true` (clause 4 (b)).
- **G** — the driver's member contract, as its header states it (`scripts/python-suites.sh:15-62`, `:66-103`): the
  population (`MEMBER_PATHSPEC`, `:191`), the step-3 complement (a) / (b) over every tracked non-member `.py`, the
  per-root loader, and the offline regime with its stated unguarded-child limit (`:96-103`). **The step-3
  complement is folded into G, not its own axis**: it is a clause of the same contract with the same owner (the
  driver header), and it constrains the same choice as the rest of G — the two new files' shape — so a separate
  axis would count one intersection twice. A-iii's *pin layer* (`PIN_FILES`, `_launch`, P6, G9) is **not** part of
  this axis: PR-0's files stay outside it (D2), and P6 is what keeps them outside (§6).
- **L** — `main`'s two legs, `"3.9"` and `"3.x"` (`.github/workflows/ci.yml`, `git grep -n -F 'python-version: ["3.9"'
  115a667c -- .github/workflows/ci.yml` prints `:266`): every member runs on both, so T5's code is 3.9 stdlib.
- **M** — A-iii's `mise.toml` reader (`mise_sections` / `mise_problems` in `scripts/python_suites_readers.py`, pins
  Q4 / M1–M3 in `scripts/test_python_suites_ci.py`), which reads `[tasks.tools-test]` — the section the reword
  edits.

| pair | intersection in PR-0 | mechanism | held by |
|---|---|---|---|
| T × S | clause 4 admits the two lines as exact text in the root `mise.toml` only, ahead of PR-1 | `ADMITTED_LINES`, exact-line equality (§4.2) | C4-g3, C4-r6, C4-r7, mutant M5 (§6) |
| T × G | T5 is a member under the driver's contract: in the population, its non-member module clean under step 3 (a) / (b), run under the regime; its list is its own `git ls-files`, stdout only | the member's `tracked_paths` (§4.3), outside the pin layer (D2) | L1, L2, C0, C6, mutants M1, M8, M9, M10 (§6); P6 unchanged |
| T × L | T5 must compile and run on 3.9 | stdlib only, no construct newer than 3.9 (no `match`, no `itertools.pairwise`, no PEP 604 unions at run time) | the `"3.9"` leg running the new member |
| T × M | the reword stays inside the reader's grammar (a one-line basic string, no escape) and clause 4 reads the same line | one edited line; nothing pins its text (D4) | Q4 live (existing), L1, C4-r1 |
| G × L | a member launching `git ls-files` under the regime, on both legs | precedent: P6 (`git grep -n -F 'def test_P6_pins_file_never_credits' 115a667c -- scripts/test_python_suites_driver.py` prints `:259`); `main`'s CI at `115a667c` is green (`gh run list -R send/elidex --branch main --workflow ci.yml --limit 1`) | L1 on both legs |
| S × M | — in PR-0 (PR-0 writes no `[settings]` line); PR-1's writes × clause 4 is PR-1's slice-local pair (C9, C21) | — | PR-1 |
| S × G, S × L, G × M, L × M | — (S has no text on `main`; the reader already runs on both legs; the reader launches nothing) | — | — |

**Five axes (T, S, G, L, M), five non-empty pairs** (T × S, T × G, T × L, T × M, G × L), so the ≥3-axis trigger
holds: D1 below.

---

## §3 Spec coverage map

**No spec surface** — a tracked-tree rule over file names and text, a test member, one task description. As for
the umbrella (body §3), `main`'s preflight exits 1 on this declaration until A-ii's marker recognition lands.

---

## §4 Design

### §4.1 The four delegated decisions

**D1 — single intersection? No.** Delegated by body :100 ("Whether PR-0 is a single intersection is **PR-0's
plan's call**"), :313 and :656–658 (C11, rounds :425–428). Answer: PR-0 meets five axes and five non-empty pairs
(§2), each of which constrains a design choice in this plan — the exact admitted text (T × S), the module pair and
its list source (T × G), the language level (T × L), the edited line (T × M), the launch on both legs (G × L). That
is CLAUDE.md *Edge-dense work*'s trigger "≥3 intersecting invariant axes を束ねる", so PR-0 is edge-dense, carries
§2 as its own matrix and passes its own plan-review. It is terminal under (c): a plan-reviewed, narrowly scoped
slice of the approved umbrella. Its diff review does not invoke the base-case exclusion of
`.claude/skills/elidex-review/axes.md` ("scope が単一 invariant-axis 交点に絞られている"; `git grep -n -F 'Base case
除外' 115a667c -- .claude/skills/elidex-review/axes.md` prints `:139`); the entry's firing condition (no trace of the
split) is unmet, because the umbrella and this plan are the trace — as body :650–653 says for PR-1. Reason against
"single": calling it single would let the diff review skip the driver contract and the 3.9 leg, which are exactly
where a member that launches `git` and reads the tree can break `main` or go silently green.

**D2 — T5's home: its own non-pin module pair.** Delegated by body :366 ("PR-0's plan records why it goes there or
names its own module"), :563 and rounds :551 (C29). Answer: a new non-member module **`scripts/tracked_tool_config.py`**
(the predicate, pure) and a new set member **`scripts/test_tracked_tool_config.py`** (its listing, controls,
mutants and the live run). Neither imports `scripts/python_suites_readers.py` or `scripts/python_suites_fixtures.py`,
and no pin file imports either. The pair is compared against **both** homes it could have joined:

- **Against the readers module** (the umbrella's default):
  1. *Different contract.* `scripts/python_suites_readers.py` *models* a grammar subset and raises `ReaderError` on
     anything outside it (its docstring, `:1-7`). T5 by decision 150 (1) models nothing; it is fail-safe over names
     and text. Co-locating a non-modelling rule beside two modelling readers puts the retired shape (draft 4's T5b
     key classification) one function away — the decision surface CLAUDE.md *One issue, one way* asks to remove.
  2. *PR-1 rewrites the readers module and must not edit clauses 2–5.* PR-1 moves the `mise.toml` reader onto
     `tomllib` (the text grammar retires; body :729–730) and adds clause 1. In its own module, with clauses 2–5 in
     named functions (§4.3), C0 keeps only the names; that PR-1 edits no clause-2–5 function body ("PR-1 adds
     clause 1 and the settings lines without editing clauses 2–5", body :624–625) is a condition checked in PR-1's review, per function span.
  3. *Different subject.* The readers read two named files (`ci.yml`, `mise.toml`); T5 is a predicate over the
     tracked tree (body :365–366).
- **Against the pin helpers** (`scripts/python_suites_fixtures.py`, `_launch` / `base_env` / `PIN_FILES`):
  4. *P6's closure would make T5 pin-layer, and PR-1's clause 1 would drag the pin's parser in with it.* P6 holds
     `PIN_FILES` closed under imports both ways (`scripts/test_python_suites_driver.py:302`, "which is not a pin
     file"; `:306`, "imports a pin helper but is not a pin file"). A member importing the fixtures would force itself
     and the predicate module into `PIN_FILES`; then PR-1's clause 1, whose parser is `python_pin.py`'s (body
     :630–631), would force `scripts/python_pin.py` into `PIN_FILES` and under G9's launch scan — a coupling no
     invariant asks for. Outside the pin layer, the same P6 lines are what keep the pair outside: either import
     direction turns P6 red. PR-1's `python_pin` import keeps the pair out of P6 only while no `PIN_FILES` member
     imports `python_pin` (its tests load it by path, body :207–217).
  5. *`_launch` is fail-open for this use.* It merges stderr into stdout (`stderr=subprocess.STDOUT`,
     `scripts/python_suites_fixtures.py:116`) and decodes with `"replace"` (`:119`). A git warning on exit 0 would fuse
     onto the first NUL-separated path, so the first record is contaminated: a non-path in the list, and its path
     missing from it. T5's list is taken stdout only (D3).
  The precedent for a member that launches its own process without the fixtures is the non-pin member
  `.claude/tools/test_plan_memo_umbrella_check.py` (`subprocess.run` at its `:39`), which imports neither helper.
- **Names**: `tracked_tool_config` names the subject — the tracked tool-configuration files (mise's config, rc and
  version files) — and carries no `python_suites_` prefix, which is the A-iii pin family's (`PIN_FILES` lists only
  `python_suites_*` and `test_python_suites_*` files), so the pair does not read as one of its pieces. Neither path
  has a component that folds to a `mise` / `rtx` prefix, so neither is a T5 member (clause 2) — required, since
  both hold `python` throughout. A `t5_` prefix is not used: T5 is the umbrella's label, not a stable name.
- Size is not a reason: every home stays under 1000 lines (§7).

**Launcher-passed data**: does not arise — under D3 the member takes its own list, so T5 reads no data its
launcher passes.

**D3 — the file list: the member's own `git ls-files`, stdout only.** Delegated by body :625–628 ("answered yes by
PR-0's plan; if no, PR-0 takes its file list from the driver … and PR-0's plan says which"), :997 and rounds :425
(C11). Answer: **yes** — a suite member may run `git ls-files` on both legs. Grounds:
- (i) **`git` is in the regime's unguarded-child class, so the regime's measurement does not cover it.** The
  driver header's stated limit includes a child that "is not Python" (`scripts/python-suites.sh:100`); the socket
  guard is a `sitecustomize.py`, installed only in Python processes. `git ls-files` is offline because of what it
  is — a read of the local index and nothing else (no remote, no fetch, no credential helper) — not because the
  regime measured it. (In a partial clone with a sparse index, `ls-files` may lazily fetch trees; CI's checkout has
  no filter.) Its environment is the regime's on every key (the member passes `os.environ` minus `GIT_*`,
  changing no regime key), so its Python descendants, if any, would be guarded; it has none.
- (ii) **It already runs.** P6 (`scripts/test_python_suites_driver.py:259-262`) launches `git -C REPO ls-files -z -- *.py`
  on the live tree, on both legs of every `tools` run since it landed.
- (iii) **The driver's own list cannot serve**: it lists only the set and the `.py` population (`git grep -n -F
  'members = git_lines(root, "ls-files"' 115a667c -- scripts/python-suites.sh` prints `:435`; `ALL_PY_PATHSPEC` at
  `:439`), not every tracked path, so taking it would mean a new driver channel and an edit of
  `scripts/python-suites.sh`, the file PR-1's Step 1 edits.

**How it is taken** (§4.3 `tracked_paths`): `git -C <root> ls-files -z` with stdout piped and **stderr piped
separately**; a non-zero exit **or any stderr byte** is red, with the stderr text in the failure; stdout is decoded
`"utf-8", "surrogateescape"` and split on NUL, as the driver's `git_lines` does (`scripts/python-suites.sh:398-406`),
and `GIT_*` is dropped from the environment, as its `without_git_env` does (`:390-395`). The driver's Python half
is a heredoc that only the pin fixtures load (`_load_driver_module`), so the member restates these two lines rather
than import them; L1's index cross-check (§6) is what catches a listing that drifts from the index.

**D4 — the reword of `mise.toml:122`.** Delegated by body :394, :563, :621–622, :998–999 and rounds :414–417 (C11);
its successor is PR-1's (rounds :490, C21). **The reader first** (body :999): `mise_problems` checks
`[tasks.tools-test]` for its key set (`description`, `run`; `sources` / `outputs` red) and for `run == "bash
scripts/python-suites.sh"`; it never reads the description's value (`scripts/python_suites_readers.py:659-677`). Its
pins Q4 / M1–M3 (`scripts/test_python_suites_ci.py:104-110`, `:315-374`) build their own fixtures with
`description = "suites"` and read the live file only through `mise_problems`. So **neither the reader nor its pins
read or quote the description**, and the reword breaks no pin. Beyond them, three fragments of the line were
measured over the whole tree (`git grep -n -F '<fragment>' 115a667c`): `Run every tracked` prints `mise.toml:122`
and body :393 (a record of `ee6c7a1a`); `floor: Python` prints `mise.toml:122` and rounds :491 (C21's quote of the
pre-PR-0 text as the "before" PR-0 replaces); `first on PATH (floor` prints `mise.toml:122` only. Neither record
reads the live line, so both stay true as records. Its text, the whole line:
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
  `scripts/python-suites.sh:445-451`). **Red, unclassifiable** — the rule cannot see the text mise would read (body
  :158, I1: "whatever the rule cannot classify falls to red") — in six kinds (the strings `member_problems` emits: `missing`, `symlink`, `not-a-file`, `unreadable: <errno>`, `non-UTF-8`, `contains NUL`): the path is **missing** from the
  worktree; it is a **symlink** (any path component); it is **unreadable** (an `OSError` on the read); it is not a regular file (a **gitlink** directory, or another non-file); its bytes,
  or its name, are **not UTF-8** or the bytes contain **NUL** (a name `git` printed that decodes only with surrogate escapes).
- **Lines**: the text split on `\n`, numbered from 1; one trailing `\r` is removed before comparison.
- **Occurrences**: every match of `python` under `re.IGNORECASE` in a line.
- **(4b) admitted lines**: if the path is exactly `mise.toml`, a line equal to an entry of `ADMITTED_LINES` —
  `python.compile = false` or `python.github_attestations = true`, no indentation, no trailing space or comment —
  admits every occurrence in it. Where the line stands in the file (its table) is not read: PR-1 pins the table and
  values over parsed TOML (C10, from the same constant, §11), and the interval states it (§8).
- **(4a) tracked-path substrings**: an occurrence at `[s, s+6)` is admitted iff some tracked path `P` that
  **contains `/`** occurs in the line at `[a, a+len(P))` with `a ≤ s` and `s+6 ≤ a+len(P)`, and
  the character before `a` is absent or not one of `A–Z a–z 0–9 _ . - / @ : + ~`, and
  the character after the span is absent, or not one of `A–Z a–z 0–9 _ - / @ + ~`, and, if it is `.`, the
  character after that `.` is absent or not one of `A–Z a–z 0–9 _`.
  The `/` requirement keeps a root-level tracked file named `python…` from admitting a bare `python` key; the
  boundaries keep `ubi:scripts/python-suites.sh`, `./scripts/python-suites.sh` and `scripts/python-suites.sh.bak`
  out while admitting the path in backticks, quotes, parentheses, before `.` / `,` at a sentence end and before a
  `:<line>` locator. Both are narrower than body :385's "inside a substring that is the path of a tracked file" —
  the fail-safe direction, decided to stay in this plan (PM 289 Q2): what the narrowing turns red is fixed by the
  reword remedy.
- Any occurrence admitted by neither: **red**, one finding per (path, line).

**Clause 5 — diagnostic.** Findings are sorted by (path, line) and each is one string:
`T5 clause <n>: <path>[:<line>]: <what>[: <the line, stripped, cut at 120 chars>] -- <remedy>`, with the path and
line rendered `backslashreplace` so a non-UTF-8 name prints. Remedies:
- 3a: "a tracked mise rc file is never admitted (it chooses which files mise reads); remove it from the index";
- 3b / 3c: "the Python version is spelled only in the root .tool-versions; remove this file (another tool goes
  into mise.toml [tools])";
- 4, occurrence: "the Python version is spelled only in the root .tool-versions; reword (a description need not
  say Python; a tracked path is admitted), or rename the file so no path component starts with mise / rtx; for a
  new install setting, extend clause 4 (b) together with I6's assertion in the same PR";
- 4, unclassifiable, missing: "the file is tracked but absent from the worktree (or, under a sparse checkout, outside the cone); restore it (git checkout -- <path>)
  or remove it from the index (git rm <path>)";
- 4, unclassifiable, symlink / unreadable / gitlink or non-file / non-UTF-8 or NUL: "track the file itself as UTF-8 text under a UTF-8
  name (not a symlink or a submodule), or rename it out of the member names".
The live test's failure message is the joined list; the driver prints a failing test's detail to stdout
(`_Result._bad`, `scripts/python-suites.sh`), so both callers show every finding, not the first.

### §4.3 The two files' interfaces

**`scripts/tracked_tool_config.py`** — stdlib only, imports limited to the allow-list `errno`, `os`, `re`, `stat`, `string` (no `as` alias), no process launch, no `TestCase`, no top-level
`__main__` guard, no `"--self-test"` constant (driver step 3 (a) / (b)), no import of any `python_suites_*` module:
- `fold(c)`, `is_member(path)` — clause 2;
- `red_outright(path)` — clause 3 (a finding or `None`);
- `content_findings(path, text, tracked)` — clause 4 over one member's text, through `admitted_line(path, line)`
  (4b) and `admits_occurrence(line, s, tracked)` (4a);
- `finding(clause, path, line, what, text, remedy)` — clause 5's one string;
- `member_problems(tracked, read)` — clauses 2–5 over `tracked` (an iterable of paths, deduplicated and sorted
  inside, since an index with unmerged entries lists a path once per stage), with `read(path)` returning the bytes
  of a regular file or raising `Unclassifiable(kind)`, kind ∈ {`missing`, `symlink`, `not-a-file`, `unreadable: <errno name>`};
- `problems(tracked, read)` — **the composition, not a clause**: in PR-0 it returns `member_problems(tracked, read)`;
- `worktree_reader(root)` — the `read` above over a checkout (`os.lstat` decides the kind);
- constants `ROOT_PIN = ".tool-versions"`, `ROOT_CONFIG = "mise.toml"`, `ADMITTED_LINES` (the two lines) and
  `CLAUSES_2_TO_5` — every top-level name of the module except `problems` (the clause functions above, their
  constants and private helpers, `Unclassifiable`, `worktree_reader`, and the tuple itself).

**Clause 1's place** (D2 reason 2): PR-1 adds clause 1 as its own function (`root_pin_problems`,
its parser `python_pin.py`'s) that no clause-2–5 function calls and that calls none of them except `finding`, and
edits only `problems` to merge its findings into the sorted result, plus C0's two lists (§6: it adds its clause-1 names to the names C0 excludes from `CLAUSES_2_TO_5` and, if the module imports `python_pin`, adds it to the import allow-list). C0 (§6) holds that `CLAUSES_2_TO_5` is the module's complete set of top-level names minus `problems`; that PR-1's diff
adds no line to, and removes none from, the span (`ast` line range at PR-0's merge sha) of any function named in
`CLAUSES_2_TO_5` is a condition checked in PR-1's review, per function span.

**`scripts/test_tracked_tool_config.py`** — the set member (stdlib: `ast`, `errno`, `os`, `re`, `subprocess`, `sys`,
`tempfile`, `unittest`, `unittest.mock`). It puts its own directory first on `sys.path` when absent (under the driver, step 4
already has `sys.path[0]` = `scripts`) and imports `tracked_tool_config`. **The driver's member contract, re-checked
for the pair**: the member is in the population (`scripts/test_tracked_tool_config.py` matches `MEMBER_PATHSPEC`,
`:(glob)**/test_*.py`) and runs in the `scripts` root under the offline regime, its `git` child outside the guard's claim as D3 (i) states; the module is
a tracked non-member `.py` that step 3 (a) passes (no `TestCase`) and step 3 (b) passes (no `__main__` guard, no
`"--self-test"` constant, so not an entry point to name); both parse on 3.9 (step 3's parse clause, the `"3.9"`
leg). Its listing:
- `checked_z_output(argv, returncode, stdout, stderr)` — pure: raises `AssertionError` naming `argv`, the exit code
  and the stderr text (decoded `"replace"`, for display only) when the exit code is non-zero or stderr is non-empty;
  otherwise the stdout records, decoded `"utf-8", "surrogateescape"`, NUL-split, empty records dropped;
- `git_z(root, *args, run=subprocess.run)` — `run(["git", "-C", root, *args], env=<os.environ minus GIT_*>,
  stdout=PIPE, stderr=PIPE, timeout=600)` into `checked_z_output`;
- `tracked_paths(root)` — `sorted(set(git_z(root, "ls-files", "-z")))`.

---

## §5 Edge matrix

"Red n" = red by clause n; "skip" = member, left to clause 1; "—" = not a member.

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
| E12 | over-approximated names: `scripts/mise_helper.py` with `python` → red 4; without → green; `Mise/notes.md` → member; `..mise.toml` → —; `crates/css/elidex-css/src/declaration/misc.rs` and the in-memory path `crates/x/src/promise.rs` → —; nested `sub/mise.toml` with `[tools] node = "20"` only → green; `scripts/tracked_tool_config.py`, `scripts/test_tracked_tool_config.py` → — | as stated | clause 2's prefix and the one-dot drop; a non-Python tool is green (body :400–401) |
| E13 | tracked-state cases: an untracked `.miserc.toml` / `mise.local.toml` → —; a staged, uncommitted `.miserc.toml` → red 3a; a tracked member deleted from the worktree → red 4 (unclassifiable, missing); a tracked symlink `mise.ci.toml` → red 4 (symlink); a tracked symlink `.miserc.toml` → red 3a (clause 3 reads no text); a tracked file under a symlinked directory → red 4 (symlink); an unreadable member → red 4 (unreadable); a gitlink at `mise` → red 4 (not-a-file); non-UTF-8 bytes or a NUL byte in `mise.ci.toml` → red 4 (non-UTF-8 / contains NUL); a member whose name is not UTF-8 (`mise\xff.toml`) → red 4 (non-UTF-8); a path listed twice (unmerged) → one finding | as stated | the list is the index (`git ls-files`); the text is the worktree's |
| E14 | the listing: exit 0 with a warning on stderr → red (the stderr text in the message); a non-zero exit → red; a NUL-split stdout with dot-paths first → every record kept | as stated | D3: stdout only, stderr is a finding, never data |

**Outside the rule, stated, no control** (body :404–414, §11): a TOML escape spelling `python` without its letters
(deliberate tampering, `#11-aiii-gate-tamper-hardening`); a tracked `[env] _.path` (T1, decision 170's limit); an
input mise reads under a name clause 2 does not match; every untracked input (I4 × I6). None that changes the
version is silent once PR-1's admission exists; before PR-1 nothing admits, so nothing regresses (§8).

---

## §6 Tests and controls

All in `scripts/test_tracked_tool_config.py`. Pure controls call `member_problems` / `problems` with a path list and
an in-memory `read`; the live and scratch tests take every listing through `git_z`. Each control asserts the **clause
number and the path (and line)** in the finding, not only "non-empty". Each control body is a plain method
`control_<id>(self)` called by its `test_<id>` method, so the mutant test can run it again under a mutant. IDs are
this plan's.

- **L1 (live, green)**: `problems(tracked_paths(REPO), worktree_reader(REPO)) == []`, where `REPO` is the member's
  parent directory's parent. **Non-vacuity**: the list is non-empty, `"mise.toml"` is in it and
  `is_member("mise.toml")` is true. **Index cross-check** (every entry is a real index path): `git_z(REPO,
  "ls-files", "-z", "--stage")` gives one record per index entry; each record must match `[0-7]{6} [0-9a-f]{40,64}
  [0-3]\t([^\0]+)` in full, and the set of their paths must equal the set `tracked_paths` returned. A record fused with
  any other text fails the first, a listed non-path fails the second — the contamination D2 reason 5 describes.
- **L2 (scratch repository, red + green)**: a temporary `git -c init.defaultBranch=main init -q` (setup commands
  run with `GIT_*` dropped and are checked by exit code only, with `git add -f` and `-c core.excludesFile=` so a global gitignore cannot break them; strict stderr is the listing's) with `.miserc.toml`
  staged and `mise.local.toml` and `.python-version` present but untracked;
  `problems(tracked_paths(dir), worktree_reader(dir))` reports exactly `.miserc.toml` (3a). Kills a list taken from
  a directory walk (would add the untracked two) and one taken from `HEAD` (would miss the staged one). The body
  takes the lister as a parameter (`control_L2(self, lister=tracked_paths)`).
- **C0 (layering and the PR-1 seam)**: (i) by `ast` over both new files: no import of a `python_suites_*` module, no
  `subprocess` / `os` launch in `tracked_tool_config.py` (its imports are exactly the allow-list of §4.3, no alias; an import of
  `subprocess` / `multiprocessing` / `pty` / `ctypes` / `importlib` / `shutil` or any alias is red, as is an `os.system` /
  `popen` / `exec*` / `spawn*` attribute), no `TestCase` / `__main__` guard / `"--self-test"`
  constant in it (step 3 (a) / (b) restated where the predicate lives, so a violation names this pair); (ii) the
  names in `CLAUSES_2_TO_5` are exactly the module's top-level names (`ast`: functions, classes, assignments) minus `problems`; (iii) the named control
  set exists: the class defines `test_<id>` for every ID in `CONTROLS_2_TO_5` (C2, C3, C4, C5, L1, L2 and the
  mutant test), a tuple in the member. PR-1 adds clause 1's controls beside them and removes none.
- **C1 (listing)**: `checked_z_output` over fabricated results — `(0, b".miserc.toml\0.mise.toml\0mise.toml\0", b"")`
  → the three paths, dot-paths kept; `(0, same, b"warning: x\n")` → raises, the message containing `warning: x`;
  `(128, b"", b"fatal: y\n")` → raises with `128` and `fatal: y`; `(0, b"mise\xff.toml\0", b"")` → one path that
  `os.fsencode` maps back to `b"mise\xff.toml"`.
- **C6 (the listing's launch)**: `control_C6(self, launch=git_z)` calls `launch(REPO, "ls-files", "-z", run=fake)`
  under `mock.patch.dict(os.environ, {"GIT_DIR": "x"})`; `fake` records its kwargs and returns
  `subprocess.CompletedProcess(argv, 0, b"mise.toml\0", b"warning: x\n")`. Asserts (a) the recorded `stderr` is
  `subprocess.PIPE` (not merged into stdout), (b) no `GIT_*` key in the recorded `env`, (c) the call raises
  `AssertionError` containing `warning: x`.
- **C2 (members)**: red side — every E3/E4/E6/E8 path and `Mise/notes.md` is a member; green side — the E7 paths,
  `..mise.toml`, `misc.rs`, the two new files, and the fixed in-memory path `crates/x/src/promise.rs` are not.
- **C3 (red outright)**: r1 E3's seven paths, empty content, each red 3a; r2 E4 each red 3b; r3 E6 each red 3c; r4
  a `.miserc.toml` whose `read` raises if called (clause 3 reads no text). g1 E5: root `.tool-versions` holding
  `python 3.12` is green under `problems` and is never read (the interval, pinned so PR-1's clause 1 is the change
  that turns it); g1 is the one existing control PR-1 edits, and C0 the one existing control whose two lists PR-1 extends (its clause-1 names; `python_pin` in the import allow-list if imported) and nothing else in C0.
- **C4 (content)**: r1 E2 — `main`'s line 122 text in a `mise.toml` fixture is red 4 at its line (the "red at main"
  of the reword); r2 E8's members, each red 4 at the line; r3 E11's red substrings; r4 E12's `mise_helper.py`;
  r5 E13's unclassifiable cases (missing, symlink, symlinked parent directory, unreadable via a patched `open`, gitlink, non-UTF-8 bytes, NUL bytes), each with its kind's remedy, built in
  a temporary directory and read through `worktree_reader` (so the reader's kinds are what is tested), except that the
  NUL-byte content and the non-UTF-8 name go through `member_problems` (`probs`) with an in-memory `read` (macOS refuses that name on disk, Errno 92);
  r6 `python.compile = true`; r7 E10's near misses. g1 the live `mise.toml` text (also covered by L1); g2
  E11's green substrings, with `scripts/python-suites.sh` in the fixture's tracked list and removed from it (then
  red: the admission depends on the list, not on the string's shape); g3 E9's two lines, LF and CRLF; g4 E12's
  nested `[tools] node` file.
- **C5 (diagnostic)**: one fixture with a 3a, a 3c, a missing member and three clause-4 findings in two files: the
  result is sorted by (path, line), each string carries the clause, the path, the line for clause 4 and its
  clause's (and kind's) remedy; a line longer than 120 characters is cut.
- **Existing pins that must stay green unchanged**: Q4 / M1–M3 (the reader over the reworded live file), P6 (which
  now also asserts the pair stays outside the pin layer: either import direction is red at
  `scripts/test_python_suites_driver.py:302` / `:306`), G9, and every other member.

**Mutation — in the member, on every run.** `MUTANTS` in the member is a table of (ID, target, wrong
implementation, killing control); `test_mutants_are_killed` applies each by `unittest.mock.patch.object` on the
`tracked_tool_config` module (or, for the member's own listing, passes the wrong function as the parameter of
`control_L2(self, lister=tracked_paths)` / `control_C1(self, check=checked_z_output)` / `control_C6(self,
launch=git_z)`: M1, M8, M9, M10), runs the killing control's body and asserts it raises `AssertionError`; it also
asserts the table's IDs are exactly M1–M6 and M8–M10 and that each killing control passes unpatched (so a mutant is
not killed by a broken control). The module's functions call each other through module globals, so a patch
reaches every internal call; the member calls them as `t.f(...)` after `import tracked_tool_config as t` (no
`from … import`), so a patch reaches the member's calls too.
- M1 the list from a directory walk (`os.walk` lister) — L2;
- M2 case-sensitive names (`fold` without `casefold`) — C3 r1's `.MISERC.TOML`;
- M3 4a without the `/` rule — C4 r3's `python.txt`;
- M4 4a without boundaries — C4 r3;
- M5 4b by `in` instead of equality — C4 r7;
- M6 symlinks read through (`worktree_reader` following the link) — C4 r5;
- M8 stderr merged into stdout before the split (`_launch`'s shape) — C1's warning case;
- M9 stdout decoded `"replace"` — C1's non-UTF-8 name case;
- M10 `git_z` merging stderr into stdout (`stderr=subprocess.STDOUT`) — C6 (a).
The mutants are stdlib (`unittest.mock`) and need no second process, so they run on both legs; their cost is a
few re-runs of pure controls.

---

## §7 Scope

**Edit set**:
- `scripts/tracked_tool_config.py` — NEW (non-member module, §4.3);
- `scripts/test_tracked_tool_config.py` — NEW (set member, §6);
- `mise.toml` — the one line of D4;
- `docs/plans/2026-10-python-pin-umbrella.md` — two sentences that are false, each corrected by deletion or a
  number only, nothing added (PM 289 Q2), and body :563's deletion if the timeout rises (below):
  - body :728–729, false once PR-0 lands (PR-0 does not grow the readers module): "PR-0 and PR-1 both grow
    `scripts/python_suites_readers.py`" becomes "PR-1 grows `scripts/python_suites_readers.py`", the rest of the
    sentence unchanged, rewrapped within the paragraph;
  - body :592–596, the bullet "The queued `RUSTUP_TOOLCHAIN` unset in mise tasks", false since that item was
    withdrawn (PM 287/314; it also holds :594's stale `:115`): deleted whole;
  the body stays ≤ 1000 lines (999 at `115a667c`: `wc -l`; every edit removes characters);
- `.github/workflows/ci.yml` — the timeout comment's record of PR-0's run, always; the value, and the vm-p4
  companion, **only if the timeout rises** (below);
- this plan.

**The `tools` timeout (PM 289 Q1, PM 294).** PR-0 adds a member, so it fires `ci.yml`'s rule "a PR that adds suite
cost re-derives `timeout-minutes` in the same PR from its own run of this job" (`git grep -n -F 'a PR that adds
suite cost' 115a667c -- .github/workflows/ci.yml` prints `:243`; the value at `:262`, 22). Counter-precedent: #506
(`3ec0ae5c`) added the member `docs/plans/test_vm_p4_family_layout.py` without a `ci.yml` edit (`git show --stat
3ec0ae5c -- .github/workflows/ci.yml` prints nothing); PR-0 does not follow it. The rule (PM 302): on PR-0's own CI
run, N = ceil(4 × the slower Python-suites leg's end-to-end duration in minutes), read with the comment's `gh run
view <run-id> --json jobs` command; the value is raised to N only if N > 22 and never lowered in PR-0. The run id,
both legs' durations and the computation are recorded in that comment, as #549 and #510 recorded theirs (body
:575–577: "record its run in that comment"), so `ci.yml` is edited even when the value stays 22 — then only the
comment's record of the run.
- **N ≤ 22**: the value stays 22; vm-p4 O3 and body :563 stay true and are not edited.
- **N > 22**: the value becomes N, and vm-p4 O3 (`git grep -n -F 'under the unchanged value 22' 115a667c --
  docs/plans/2026-10-python-pin-umbrella-vm-p4.md` prints `:151`) becomes false: PR-0 deletes ", under the
  unchanged value 22" and adds nothing — before "has been in the job since `3ec0ae5c`, under the unchanged value 22
  (the split", after "has been in the job since `3ec0ae5c` (the split". Body :563's cell "no VM P4 file, no pin"
  becomes "no pin". The vm-p4 file is a VM P4 companion, so the corrected wording goes to the PM for VM P4's
  consent **before PR-0 lands**. O3's other sentence ("PR-1's re-derive … is the first one
  measured with the guard, on the pin") stays true either way — PR-0's run is not on the pin, and rounds :384–385
  states the same claim as "the first measured with the guard on the pin" — so it is not edited.

**Not touched**: `scripts/python-suites.sh` (D3), `scripts/python_suites_readers.py`,
`scripts/test_python_suites_ci.py` and `scripts/python_suites_fixtures.py` (D2; `PIN_FILES` gains nothing),
`scripts/test_python_suites_regime.py` and `scripts/test_python_suites_driver.py`, `.tool-versions` (absent;
PR-1), `scripts/python_pin.py` / `scripts/python_pin_testing.py` (PR-1), every VM P4 family file
(`.claude/tools/plan_memo_*`, `.claude/tools/plan-memo-umbrella-check.py`, `docs/plans/2026-07-vm-p4-*.md`,
`docs/plans/test_vm_p4_family_layout.py`), `CLAUDE.md`, the umbrella's records and rounds files, and the vm-p4
companion and `ci.yml` except as above.

**1000-line rule** (CLAUDE.md; body §7): `git ls-files -z -- mise.toml docs/plans/2026-10-python-pin-umbrella.md
| xargs -0 wc -l` prints 153 and 999 at `115a667c`; the new files are well under 1000; nothing crosses 1000. The
umbrella body's line count is re-measured after its two edits (≤ 1000, its K3 bound).

**No `python3 … test_*.py` launch line** in any landing file (C33, rounds :570): this plan describes the members
and never writes a direct launch of one; `git grep -nP 'python3?(\.[0-9]+)? +(-[A-Za-z]+ +)*(-m +unittest +)?(\S*/)?test_[A-Za-z0-9_]+\.py'
-- docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md` prints nothing.

---

## §8 Coordination and the PR-0 → PR-1 interval

**Lanes** (body §8): the overlap is the *every lane* row (body :816) — from PR-0, a tracked `.miserc*`, a non-root
`.tool-versions`, a `.python-version*`, a `python` outside the admitted occurrences in any file whose path has a
component starting `mise` / `rtx`, or such a file T5 cannot read as UTF-8 text, is red. Textual overlaps: `mise.toml`
(anyone editing `[tasks.tools-test]`; whichever lands second re-reads); `ci.yml`'s timeout comment, and its value
if it rises (§7; textual with PR-1 and Dependabot, which touches `steps:` only; no ordering change — PR-0 precedes
PR-1, the cargo-deny PR is after PR-2).

**Pre-landing sweep** (body :821–824): the driver re-runs A-iii §13's sweep with PR-0's contended paths added
(`mise.toml`, the two new files), and runs T5 on **each open PR's head**: the list from `git ls-tree -r -z
--name-only <head>` (stdout only, as D3), `read` from `git cat-file blob <head>:<path>` for a regular blob and
`Unclassifiable` for modes 120000 / 160000, the head fetched into `FETCH_HEAD` only (no ref writes). Each red is
handed to that PR's lane through the PM before PR-0 lands; the PR body lists the heads checked and the result.

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
4. CI: every job green, the `tools` job on **both** legs (the `"3.9"` leg is the only measurement of T × L); the
   timeout derived from that run (§7) and recorded in the `ci.yml` comment, CI re-run; if N > 22 the value rises and
   O3's corrected wording has VM P4's consent through the PM before landing.
5. **PM notice to every lane** with the landing (body :836–838, §9 step 2; §8 row *every lane*). Text to send:
   "From PR-0 (`<merge sha>`), `mise run tools-test` and the `tools` CI job are red when a **tracked** file is a
   mise rc file (`.miserc*`, any directory), a `.tool-versions` other than the repository root's, or a
   `.python-version*`; or when any tracked file whose path has a component starting `mise` / `rtx` (case-insensitive,
   one leading dot ignored — `mise.toml`, `.mise.local.toml`, `.config/mise/…`, `rtx.toml`, `mise_helper.py`)
   contains `python` in any case, except inside the path of a tracked file and the two `[settings]` lines PR-1
   adds. Two reds go beyond the umbrella's wording: (1) such a file is also red when it cannot be read as text — a
   symlink, a gitlink, unreadable, tracked but missing from the worktree or, under a sparse checkout, outside the cone (restore it or `git rm` it), or not UTF-8 / containing NUL in content
   or name; (2) `python` counts as inside a path only for a tracked path containing `/`, bounded by non-path
   characters — `` `scripts/python-suites.sh` `` passes, `./scripts/python-suites.sh`, `ubi:scripts/python-suites.sh`
   and `scripts/python-suites.sh.bak` are red. Untracked files are not checked. A `[tools]` entry for a non-Python
   tool is green. The root `.tool-versions` is unchecked until PR-1, which makes it exactly one `python X.Y.Z` line
   (a second tool goes into `mise.toml [tools]`). Each finding names the clause, the file and line, and the remedy.
   Open PRs found red at `<date>`: <list or none>."

**DoD**:
- [ ] the two new files as §4.3, outside the pin layer (no `PIN_FILES` edit; P6 green); every §6 control as
      written, each asserting clause and path; L1 green with its index cross-check; the mutant test green with
      M1–M6 and M8–M10 each killed; the existing pins green;
- [ ] `git grep -n -i python -- mise.toml` prints only lines whose occurrences lie in `scripts/python-suites.sh`;
- [ ] the umbrella body's :728 sentence and :592–596 bullet (and :563 if N > 22) corrected as §7, nothing added, body ≤ 1000
      lines;
- [ ] the X1 pattern prints nothing over this plan (§7); the set member `.claude/tools/test_plan_memo_umbrella_check.py`
      green under the driver (its `symbol_attribution_control` reads every `docs/plans/*.md`);
- [ ] the timeout derived and recorded in the `ci.yml` comment (§7); if N > 22, the value, O3 and body :563
      edited, with VM P4's consent for O3 through the PM; the sweep result in the PR body;
- [ ] CI green on both legs; the PM notice sent.

---

## §11 Deferred to PR-1 / PR-2

**PR-1** (body :564 row): clause 1 (the one-line root `.tool-versions`, parsed by `python_pin.py`) — it needs that
parser, dead before PR-1 (body :630–633) — added as §4.3 *Clause 1's place* says; the two `[settings]` lines and
their parsed-TOML assertion (C10) — the pin needs `tomllib`, absent on the `"3.9"` leg (R§6 table); the `tomllib`
reader; the description's rewrite to the pin (body §4.4; C21's "before" is D4's line); the slice-local pair "PR-1's
`mise.toml` writes × clause 4" with a control that the post-PR-1 `mise.toml` is green (C9, C21); the refusal, its
oracle, the warning, I7.

**`ADMITTED_LINES` is the one source of the two lines' text.** PR-1's I6 assertion (C10) takes its keys/values from
`tracked_tool_config.ADMITTED_LINES` or asserts equality with it — one source; where C10 lives, and in which layer, is
PR-1's plan's decision (umbrella body :564, :270–274 place the assertion with the `mise.toml` reader). Stated as a
fact, not a placement: a pin file importing `tracked_tool_config` comes under P6's closure (D2 reason 4). A new install
setting therefore extends `ADMITTED_LINES` and C10 in the same PR (clause 5's remedy).

**PR-2**: nothing of T5. The new module has no `__main__` guard and no launch line names the new member, so neither
is an entry point under I4.
