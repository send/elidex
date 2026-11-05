# PR-1 plan — pin, gate and provisioning (python-pin program, draft 1, 2026-11-04)

## §0 Status

**Lane**: CI/tools. **Program**: the Python interpreter pin, umbrella `docs/plans/2026-10-python-pin-umbrella.md`
(landed by #581, `ea12ce7f`) with its companions `…-umbrella-vm-p4.md` (§5.2–§5.3, "vm-p4"),
`…-umbrella-records.md` (R§n) and `…-umbrella-rounds.md` (§12–§14, the C-items). "Body :n" is a line of the umbrella
body, "rounds :n" of the rounds file, "vm-p4 :n" of the VM P4 companion, "PR-0 :n" of
`docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md`. E-rows, N / W controls and DM / FM / NM / WM mutants are
those of the tests memo, `docs/plans/2026-10-python-pin-pr1-tests.md` (§5 / §6). **Base**: `origin/main` at **`660e594c`** (#586 = PR-0).
Between #581 and the base (`git log --oneline ea12ce7f..660e594c`): #585 `115a667c`, #583, #587 and #586.
Outside `crates/**` they moved, against the umbrella's base `ee6c7a1a` (`git diff --stat ee6c7a1a 660e594c --
.claude scripts .github mise.toml CLAUDE.md README.md CONTRIBUTING.md`): `ci.yml` (#586's timeout record, :255–257),
`mise.toml` (#585's `check-default` task; #586's reword, now `:122`), `CLAUDE.md` / `README.md` / `CONTRIBUTING.md`
(#585), `.claude/skills/{external-converge,external-review,pre-push}` (#585), and PR-0's two new files. **No
`.claude/tools/**` file changed** (the same command lists none). Of the plan memos V1–V23 cite, one VM P4 memo
changed and none of its lines is cited: `git diff --stat ee6c7a1a 660e594c -- 'docs/plans/2026-07-vm-p4-*'
docs/plans/2026-08-plan-memo-umbrella-checker.md docs/plans/2026-10-vm-p4-memo-seam-split.md .claude/tools` lists
only `docs/plans/2026-07-vm-p4-slice-plan.md` (1 line, `:953`, #578), a file no V row names. So every family line
vm-p4's V1–V11 and V22 cite, #510's memo lines (V12, V13, V15, V18), the seam-split lines (V16, V17) and the
slot-ledger lines (V19–V21, V23) hold at `660e594c`; `CLAUDE.md:54` (V14) and `:100` hold too (`git grep -nE '両
leg|matrix' 660e594c -- CLAUDE.md` prints `:30`, `:54`, `:100`).

**Order and preconditions** (body :564 row, :567–568, §9 steps 0 and 3): #581 merged (done) → PR-0 (done) → **PR-1**
→ the PR-1 → PR-2 interval → PR-2. The VM P4 lane's consent to §5.3 (c) was received 2026-11-03 (PM 274; recorded in
`elidex-pm-trial/handoff/ci-tools-python-pin/vm-p4-consent-2026-11-03.md`); its conditions bind this plan and the PR
body (§8.1). PR-1's implementation starts after this plan's `/elidex-plan-review`; **PR-1 lands only after PR-2's
plan has passed its own plan-review** (body :703, the interval's bound).

**Edge-dense: yes** (body :639–653; C9). PR-1 owns the gate halves of fifteen pairs (body :309) plus the slice-local
pairs of §2.3; it carries its own pair matrix and plan-review and is a terminal unit under CLAUDE.md *Edge-dense work*
(c). Its diff review cites the umbrella and this plan as the split's trace, not `axes.md:139`'s exclusion (body :647).

**Ownership (C17, C22).** PR-1 holds I1's CI half and T5's clause 1, I2's gate half (incl. the removal of
floor-motivated code), I3, **I5's gate half** (the refusal, its oracle and the warning, run by Step 1), I6 — what mise
installs from the tracked configuration, **not** which `python3` of the pin's version a launcher runs (decision 170,
body §11 T1/T2) — and I7's helper with its controls.

**No ECS surface** (no `crates/**` file). **No spec surface** (§3). **Measured digits**: every line number below is
re-measurable at `660e594c` with the command beside it; counts and timings of PR-1's own runs go to the PR body.

**Where the umbrella's "PR-1 §n" lands in this plan** (the C-items were written against draft 2's layout): §4.1 the
pin file (C15); §4.2 `python_pin.py` and the refusal (C4, C16, C27, C31, C37); §4.3 the driver; §4.4 CI and its reader
(the pin "N4", body :881); §4.5 `mise.toml` (C10, C21, C36); §4.6 discovery and classes (C1, C2, C26, C30); §4.7 the
landed memos amended in place (C3); §4.8 the I7 helper (C23, C28, C34); §6 the pins (in the tests memo, `docs/plans/2026-10-python-pin-pr1-tests.md`); §7 the edit set and the timeout
("§8" in C1 / C26 / C30); §8 coordination (C14, C32); §9 slots; §10 landing, DoD and exit criterion ("§10" / "§11" /
"§12" in C1, C20, C22, C36).

Provenance of the delegations this plan answers (each prints the line cited):
```sh
U=docs/plans/2026-10-python-pin-umbrella.md; R=docs/plans/2026-10-python-pin-umbrella-rounds.md
git grep -n -F "with PR-1, \`…-pr2-entry-point-guard.md\`" 660e594c -- $U       # :105  this file's name
git grep -n -F "So PR-1's matrix holds fifteen pairs" 660e594c -- $U           # :309
git grep -n -F "up to a fixed cap PR-1's plan sets" 660e594c -- $U             # :444  the stderr cap (§4.2)
git grep -n -F "PR-1 §4.6 gives the discovery command" 660e594c -- $U          # :500  (§4.6)
git grep -n -F "Which timeout rule PR-1 follows" 660e594c -- $U                # :520  (§7)
git grep -n -F "**PR-1 pin, gate and provisioning**" 660e594c -- $U            # :564  the scope row
git grep -n -F "Owner: PR-1 —" 660e594c -- $U                                  # :205  I7's helper (§4.8)
git grep -n -F "PR-1 §9 (two:" 660e594c -- $U                                  # :865  (§9)
git grep -n -F "g1 is the one existing control PR-1 edits" 660e594c -- docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md  # :384
git grep -n -F "**C9 — PR-1 is edge-dense" 660e594c -- $R                      # :401
```

---

## §1 What PR-1 holds

Body §1 (:134–142): **one interpreter, spelled once, admitted by every launcher, measured by the gate; any other
version refused, by name, before it runs anything.** PR-1 makes the **gate** that shape — the root `.tool-versions`
is the one spelling; `scripts/python_pin.py` is its one reader; the driver's Step 1 admits exactly its version and
otherwise prints observed facts and one generic block; the `tools` job has one leg reading the same file; mise
installs that version attested and prebuilt or not at all; every code branch and statement that exists only for
another interpreter goes. Direct entry points stay PR-2's (§11). The guarantee is decision 170's: **an interpreter
whose version is not the pin is never admitted silently**; one of the pin's version from another source is admitted,
a stated limit (body §11 T1, T2, *Any other same-version `python3`*), and the CI rule is not widened for it.

---

## §2 Invariants and the intra-PR pair matrix

### §2.1 The halves PR-1 owns

Body §2's text (:148–234) is the definition; PR-1's share, one line each:
- **I1 (CI half + clause 1)**: `.tool-versions` is exactly `python 3.15.2`; `python_pin.py`'s parser is the one reader
  — Step 1 on every driver run, local and CI (a malformed live pin stops the run there: `FAIL pin`, §4.2
  *Pin-file errors*), and T5's clause 1 over the tracked tree once Step 1 has admitted (what it adds: the file is
  tracked and a regular file; its parse branch is reached in-process by W1); `ci.yml` reads it only through
  `python-version-file`.
- **I2 (gate half)**: Step 1 admits exactly `sys.version_info[:3] == pin` with release level `final`; the `tools`
  job's one leg is the only CI measurement; V5 / V6 (floor-motivated code) and V22 (a call the pin deprecates) go.
- **I3**: one leg, no `strategy`, one `actions/setup-python` step with exactly `python-version-file: .tool-versions`,
  `runs-on: ubuntu-24.04`; the `ci.yml` reader models that and the steps before the driver as §4.4 states, and is
  red on what it models — not on everything else (§4.4 *What the reader does not see*).
- **I5 (gate half)**: the refusal (§4.2) — facts items 1–6 and one generic block, no cause inferred, oracle never
  admits; the warning on every Step 1.
- **I6**: `mise.toml [settings]` holds `python.compile = false` and `python.github_attestations = true`, asserted over
  parsed TOML (W2, tests memo §6.4) — for what mise installs from the tracked configuration only.
- **I7**: the population is defined by **where a child's environment comes from**, not by which name it looks up
  (any `PATH` lookup — `git`, `bash` — can resolve to a mise shim): (i) **every launch whose environment comes from
  `scripts/python_pin_testing.py` (NEW)'s `isolated_mise_env` (NEW)** — the fake first or no `mise` (N3's no-mise-call
  half, N9–N15, N19–N20); (iii) **every launch whose environment derives from `python_suites_fixtures.base_env()`**,
  which isolates its children from the caller's mise (§4.3, the one statement of what it drops and sets; N22) — at `660e594c`, `git grep -n -F 'base_env()'
  -- scripts` prints its definition (`python_suites_fixtures.py:81`) and seven launch sites: `Scratch.git` / `run` /
  `baseline` (`:139`, `:149`, `:160`), `test_python_suites_driver.py:261` (P6's `git ls-files`), and
  `test_python_suites_regime.py:549` (the direct driver launch, `:549–558`), `:681` and `:690` (G9's two
  `sys.executable -c pass` launches, `:681–683`, `:690–691`); PR-1 adds N22's, N1's and the pre-launch case's `base_env()` calls (tests memo §6.1;
  the population is unchanged by `base_env()`'s isolation: it is still every launch whose environment derives from it); (ii) N1,
  in both: its environment is `base_env()`'s without its `MISE_*`, with the helper's five `MISE_*` variables and the
  fakes' directory ahead of its `PATH` (stated exactly in tests memo §6.1). Not in the population: the `git` children of
  `test_tracked_tool_config.py`, whose environment is its own `_without_git_env()` (`:45–46`, the outer `PATH`); the
  member is not a pin file and cannot take `base_env()` (P6), so a `git` that is itself a mise shim on the outer
  `PATH` reaches mise there — in the live root and in L2's scratch tree, which holds a `.tool-versions` from PR-1
  (§4.5) — a stated limit below.

**Stated limits, not guarantees** (C22): same-version-other-source (body §11 T1, T2, the same-version bullet);
untracked install-setting overrides (body §11, warned while set, never red); the oracle's shape (mise unpinned —
degrades to item 6, never changes an exit status); the refusal on a real non-pin interpreter, measured once per
module digest by hand (N17's record, §10); `feature_version` best-effort (N16's limit); on a platform without
`os.killpg` the oracle's timeout kills mise only, not its descendants, and everywhere a descendant that starts its
own session leaves the killed group, and an end of the refusal process that raises no Python exception (SIGKILL,
SIGTERM's default action) kills no group, nor does an exception raised inside `Popen` before its pid reaches
`ask_mise` (§4.2 *Process tree*); unverified, an interpreter whose directory is mise's shims directory (§4.3
`base_env`); a `git` on the outer `PATH` that is a mise shim, under T5's member (I7 above); the fakes need a
`sys.executable` usable as a shebang — no white space, `#!` plus it within 127 bytes (§4.8).

### §2.2 The fifteen umbrella pairs, gate half (body :285–309)

| pair | intersection in PR-1 | mechanism | held by (tests memo §6) |
|---|---|---|---|
| I1 × I2 | Step 1 compares against the value read from `.tool-versions` at run time | `python_pin.read_pin` (NEW) called by `main` (NEW); no literal in the driver | N1's spelling control (edit the scratch pin to the shim's version → admitted; kills DM5), DM2 for a literal in the driver; NM3 |
| I1 × I3 | CI reads the pin only through `python-version-file`; a `python-version` input beside it would win | `setup_problems` (NEW) in the readers: the setup step's `with` is exactly that one key | N4's red controls (added input, other file, second step) |
| I1 × I5 | the reader and the refusal run on the interpreter being refused | `python_pin.py` stdlib-only, `sys` / `os` at top level, `subprocess` / `json` on the refusal path only, 3.9 grammar; index-only reads of `sys.version_info` | N16 (feature_version + the six rejected rows), N17 (digest record), N18 (imports) |
| I1 × I6 | the one spelling names a patch both installers serve prebuilt; mise refuses a source build or an unverified archive | the two `[settings]` lines, W2 over parsed TOML from `tracked_tool_config.ADMITTED_LINES` (one source); clause 1 | W1, W2, W3; a pointer to the bump rule in `python_pin.py`'s header (§4.2 *Header*) |
| I1 × I7 | tests' scratch trees copy `python_pin.py` and write their own `.tool-versions`; the isolated mise reads only those | `pin_tree` (NEW) in the helper; `Scratch` copies both files | N9, N19 (a) |
| I2 × I3 | one admitted interpreter ⇒ one leg | the job's `strategy` leaves; `strategy` leaves the job-key allowlist | N4, N6 |
| I2 × I5 | Step 1 runs before any suite | the driver's Step 1 precedes the heredocs and the first `git` | N1 ("member(s) in" absent on every refusal) |
| I2 × I6 | no build → red on both callers, never a fallback | setup-python fails the job; `python.compile = false` fails `mise install` | W2 holds that the settings are present (§4.5); that mise refuses is a dated measurement, not a pin (mise is unpinned): R§4.1 *Does the age filter gate an exact install? Does compile-off refuse a source build?* (records :250–257, the variable form), this plan's measurement from the tracked file and its re-run at landing (both §10 step 11) |
| I2 × I7 | I7 binds the tests' children, never the gate's own admission | Step 1 uses the caller's mise; only children a pin starts are isolated | N1 runs the scratch driver under the helper; the live Step 1 does not |
| I3 × I5 | the refusal is executed on the one leg by patching `sys.version_info` inside the pin | the helper's `LAUNCHER` (NEW) and `fake_python3` (NEW) | N1, N10 |
| I3 × I6 | CI provisioning is setup-python's manifest build for the pinned image | `runs-on: ubuntu-24.04` with its reason restated | N4 (`runs-on` exact) |
| I3 × I7 | CI has no mise, local has one: both run the fake's branch | the fake `mise` first on the child's `PATH`, or none | N19 (a) asserts `shutil.which("mise", path=…)` |
| I5 × I6 | the refusal prints what provisioning answers (item 5) and the generic actions, `mise install` among them | `ask_mise` (NEW), `refusal_text` (NEW), `GENERIC_BLOCK` (NEW) | N10 (each case's fix row), N11 |
| I5 × I7 | every oracle answer is reached under the fake | N10–N15 run through `isolated_mise_env` | N19 (a) inside the same fixtures |
| I6 × I7 | the isolated data dir holds no install; the warning's variables are dropped unless a fixture sets them | the helper drops every outer `MISE_*` | N13, N19 (a) |

The fifteen rows are body :309's list (`grep -o '^| I[1-7] × I[1-7] |' <this plan> | sort | uniq -c` prints each
once).

### §2.3 Slice-local pairs (C9; body :641–644)

| pair | intersection | mechanism | held by |
|---|---|---|---|
| scratch repos × I1 | every A-iii scratch driver now runs Step 1, so each scratch tree must hold the pin and its reader | `Scratch.__init__` copies `.tool-versions` and `scripts/python_pin.py` from the live tree | every existing scratch pin stays green; N1 |
| scratch repos × I7 | after Step 1 exists, a scratch driver whose `PATH` reaches a mise shim runs the real mise in a tree holding a copied `.tool-versions` (state writes; an unisolated oracle on any refusal) | `base_env()` isolates (§4.3) every launch of I7 (iii) (§2.1): whatever mise a child reaches is confined | N22 (four parts) and its mutant FM1; N22's reachable-mise case |
| the oracle × I3 (no mise in CI) | CI and local must print the same facts | fake `mise` fixtures with the measured shapes (R§4.2) | N10–N12 |
| the VM P4 removals × I2 | V5 / V6 / V22 are correct only on the pin | removed with Step 1's exact admission in the same PR | the guard and #510's self-test green on the pin; C35's census (§10) |
| the `mise.toml` reader × Step 1's ordering | the `tomllib` reader is safe only after admission | the reader is a pin module run by a member, i.e. after Step 1 | N7 / N8 (member runs); I1 × I5 binds `python_pin.py` only |
| PR-1's `mise.toml` writes × PR-0's clause 4 | the `[settings]` lines, the reworded description and the new comment must spell no `python` outside admitted occurrences | exact admitted lines; no word `python` in the new text except tracked paths with `/` | L1 (live), W3 (fixture) |
| clause 1 × PR-0's seam | PR-0 :293 and :384 let PR-1 edit only `problems`, C3 g1 and C0's two lists; why that seam cannot hold is §4.5 *What the member's fixtures change* | clause 1 is `root_pin_problems` (NEW), its own names; `problems` merges it; the valid-world baseline lives in the member's fixture layer (§4.5) | C0 (§4.5's edits), the review condition per function span (PR-0 :293–295, kept), C3–C5 / L2 green through the baseline |
| `python_pin` × the pin layer (P6) | a pin file importing `python_pin` would pull it into `PIN_FILES` | no pin file imports it; `Scratch` copies it by path; `tracked_tool_config` (not a pin file) imports it | P6 unchanged |
| the I7 helper × G9 | a pin file loads `python_pin_testing.py` by path, outside P6's import scan | the helper launches nothing | N21 (no call in G9's spawner set, read from G9's own source, fail-closed on its shape — tests memo §6.3) |
| `base_env()` × the helper (one issue, one way) | the fixtures module (a pin file) cannot import the helper (P6), yet both name mise's isolation variables | `base_env()` copies the variable names (`MISE_DIR_VARS`, `MISE_GLOBAL_VAR`), not the helper's code; the driver pin, which loads the helper by path, asserts the two lists agree | N22's `isolation` part |
| the timeout × #510's 2× trigger | PR-1 re-derives the value from its one leg | §7's rule (N = ceil(4 × the leg); raise-only, PM 353) keeps the ratio ≥ 4 | the `ci.yml` comment's record |
| this plan × O8 | `symbol_attribution_control` reads every `docs/plans/*.md` | attributions here name defining modules only | the set member `.claude/tools/test_plan_memo_umbrella_check.py` green |

---

## §3 Spec coverage map

**No spec surface** — a pin file, a stdlib module, a driver step, a CI job, `mise.toml`, their readers and pins, and
the docs that state them. External references, none of which `webref` covers and none numbered as spec sections:
the Python documentation of `tomllib` (3.11+) and of `ast.parse(feature_version=…)` ("best-effort"); setup-python
v7's `docs/advanced-usage.md` (`python-version-file`, and that a `python-version` input beside it wins — R§4.2's
command); mise's settings `python.compile` / `python.github_attestations` and its `ls` / `which` / `config` commands
(measured, R§4.1, R§4.2). As for the umbrella (body :319–324) and PR-0 (PR-0 §3), `main`'s preflight exits 1 on this
declaration ("no markdown table follows it"); acceptance is that exit with exactly that hard fail. A table is not
written: its rows would need `§<number>` cells these documents do not have.

---

## §4 Design

### §4.1 `.tool-versions` (NEW)

Exactly one line, `python 3.15.2`, with a final newline (body §4.1; C15: no comment — a comment line is red by
clause 1). The bump rule is stated once, body §4.1's criterion (:331–334); `python_pin.py`'s header (§4.2) and
README / CONTRIBUTING (their statement-class rewrite, §4.6's table) carry a pointer to it (C15). A second tool goes
into `mise.toml [tools]` (body :401–402).

### §4.2 `scripts/python_pin.py` (NEW) — the one reader, admission, the refusal

**Shape.** Stdlib only; top-level imports `os` and `sys` and nothing else; `subprocess` and `json` imported inside
`ask_mise` only (I1 × I5; C16 (d)). Grammar: parses with `ast.parse(…, feature_version=(3, 9))` (N16) and holds
none of R§11's ten gap constructs, which N16 cannot see (`feature_version` accepts them) — held by N17's digest
record and §10 step 4's real run on 3.9.6; `%`-formatting, no f-strings; ASCII source (the BOM test spells
`"﻿"`). `sys.version_info` is read by index (`v[0]`…`v[3]`),
never by attribute, so a patched tuple and any Python 3 read alike (N3's plain-tuple `refusal_text` call pins it on
the refusal path, where `LAUNCHER`'s tuple subclass would also answer attributes). No `TestCase`, no `"--self-test"` constant
(driver step 3 (a) / (b)); a top-level `__main__` guard calling `_guarded_main` (the exit line, below; the driver launches it, §4.3).

**Root (H3).** `repo_root()` (NEW) = the parent of the directory of `os.path.realpath(__file__)` — never the cwd,
never a walk up (body :481–489). The pin file is `<root>/.tool-versions`.

**Parser (clause 1's, one form).** `parse_pin(text)` (NEW) accepts exactly the regex
`\Apython (0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\n?\Z` over the decoded text and returns the triple;
anything else raises `PinFileError(why)` (NEW) naming what it found: no file, not UTF-8, empty, a BOM, CRLF, a
second line (`core:python 3.14.10`, R§4.2's second-line case), a comment, a blank line, another tool, `3.15`,
`latest`, a pre-release, a trailing comment (`python 3.15.2 # c`, `python 3.15.2#c` — named as a comment, not as
two versions), two versions, a tab, two spaces, `Python`, leading zeros, trailing space. `read_pin(root)`
(NEW) reads the bytes of `<root>/.tool-versions` and calls it.

**Admission.** `admitted(v, pin)` (NEW) = `(v[0], v[1], v[2]) == pin and v[3] == "final"` (body §4.3). It never
consults mise (the oracle is diagnostic only, body :181).

**The warning (body :458–472; C16 (c)).** `warning_lines(environ)` (NEW): one line per set variable among every
`MISE_PYTHON_*` except `MISE_PYTHON_VERSION`, and `MISE_GITHUB_ATTESTATIONS`, sorted, each
`python-pin: warning: <VAR>=<value> is set: it may override the tracked install setting for any install made in
this environment`. Printed to stderr on every `main` call — admission and refusal — and never changes the exit
status.

**The oracle (body :416–426).** `ask_mise(cwd, timeout)` (NEW), on refusal only, runs in this order and no other:
(a) `mise ls --current --json python`; (b) `mise which python3`; (c) `mise config ls --json`; (d) for each path (c)
listed, `mise config get -f <path> env._.path`. Each runs with `cwd=` the process cwd, `stdin=DEVNULL`, stdout and
stderr captured separately, through `subprocess.Popen(…, start_new_session=GROUP)` and
`communicate(timeout=ORACLE_TIMEOUT_S)` (NEW constant), `GROUP = hasattr(os, "killpg")`. Decisions this plan takes
(delegated by body :442–445):
- **Timeout: `ORACLE_TIMEOUT_S = 10`** seconds per question. A healthy answer is a local read; the value bounds a
  hang without cutting a slow disk, and it is paid only on the refusal path. After the first timeout no further
  question is asked, and the facts say so ("not asked: an earlier question timed out") — a fact about the refusal,
  not a cause. So a hang costs one timeout, not four.
- **Process tree**: each question runs in its own session, so mise and everything it starts form one process
  group; one `try` holds both `Popen` and `communicate`, and once `Popen` has returned and `proc` is bound, **any**
  exception (`TimeoutExpired`, and a `KeyboardInterrupt`: Ctrl-C signals the terminal's foreground group, which mise's
  new session is not in; any `BaseException`) runs `_reap(proc)` (NEW): `os.killpg(proc.pid, 9)`
  (9 = SIGKILL on Linux and macOS; `os` is already imported, so `signal` stays out — body :241), then `proc.wait()`,
  and closes the pipes **without reading them again**; a `TimeoutExpired` then becomes the fact below, anything else
  is re-raised. So on any Python exception after `proc` is bound, no member of the group can block the refusal or
  outlive it. Stated limits (§2.1): a descendant that starts its own session has left the group and is not killed;
  and an exception raised inside `Popen` after its fork but before it returns (a `KeyboardInterrupt` during its
  error-pipe read), or between its return and the binding of `proc`, leaves mise running, because no pid reaches
  `ask_mise` — closing that window would need signals blocked around `Popen`, i.e. the `signal` import the module
  does not take (the module's comment there says so). Measured
  2026-11-04 on macOS `/usr/bin/python3` 3.9.6 by the command below: `group 1.0 s gone` (the background `sleep` is
  dead), `child 1.0 s alive` (`proc.kill()` without a session leaves it). N15 pins both on the pin (no survivor;
  NM14).
  ```sh
  for m in group child; do /usr/bin/python3 -I -c 'import os,subprocess as s,sys,time
  g=sys.argv[1]=="group";f="/tmp/pr1-gc.pid";t=time.time()
  p=s.Popen(["/bin/sh","-c","sleep 30 & echo $! >%s; exec sleep 30"%f],stdout=s.PIPE,stderr=s.PIPE,start_new_session=g)
  try: p.communicate(timeout=1)
  except s.TimeoutExpired: os.killpg(p.pid,9) if g else p.kill(); p.wait()
  e=time.time()-t;time.sleep(1);c=int(open(f).read())
  try: os.kill(c,0);print("%s %.1f s alive"%(sys.argv[1],e));os.kill(c,9)
  except ProcessLookupError: print("%s %.1f s gone"%(sys.argv[1],e))' $m; done
  ```
  Where `os.killpg` does not exist (Windows) `GROUP` is false and the timeout kills mise only — a stated limit (§2.1)
  on a platform no caller uses: the driver is a `bash` script and the `tools` job runs on `ubuntu-24.04`
  (`ci.yml:272`).
- **Stderr cap: `STDERR_CAP = 4000`** characters (NEW constant) of mise's stderr decoded `"utf-8", "replace"`,
  verbatim, then `[truncated: <n> more characters]`. mise's untrusted-config error is five lines at mise 2026.10.25
  (next paragraph); the cap keeps a pathological stream from flooding the refusal while keeping every measured
  message whole. No line position is relied on.

**Shapes the fakes answer.** Measured 2026-11-04, mise 2026.10.25 macos-arm64, in a scratch project `$P` holding
`.tool-versions` = `python 3.15.2` and a `mise.toml` with PR-1's two `[settings]` lines, every call as
`cd $P && $I [MISE_TRUSTED_CONFIG_PATHS=$P] $M <question>`, run by `bash` (in zsh, `M=$(whence -p mise)`), where
`M=$(type -P mise)`, `d=$(mktemp -d)`, `P=$d/p` (inside `$d`, so the post-check below covers it), `: > $d/g.toml` and
`I="env -i HOME=$HOME PATH=/usr/bin:/bin MISE_OFFLINE=1 MISE_AUTO_INSTALL=false
MISE_DATA_DIR=$d/data MISE_CACHE_DIR=$d/cache MISE_STATE_DIR=$d/state MISE_CONFIG_DIR=$d/config
MISE_GLOBAL_CONFIG_FILE=$d/g.toml"` (body :113–117's probe rule); nothing was installed (`$d/data/installs` absent),
mise recorded `$P/mise.toml`, `$P/.tool-versions` and `$d/g.toml` in `$d/state/tracked-configs`, and the user's
`tracked-configs` / `trusted-configs` gained no entry resolving under `$d` (or its `/private` real path):
- (a) trusted, not installed: rc 0, one element with exactly the keys `version`, `requested_version`,
  `install_path` (under `MISE_DATA_DIR` as given, `/var/…`), `source` `{"type": ".tool-versions", "path"}` (the real
  path, `/private/var/…`), `installed` (`false`) and `active`; untrusted: rc 1, empty stdout, five stderr lines (R§4.2's
  two `mise ERROR` lines, ``Trust them with `mise trust`. …``, a version line, a `--verbose` hint);
- (b) not installed: rc 1, ``mise ERROR python3 is not a mise bin. Perhaps you need to install it first.``;
- (c) trusted: rc 0, an array of `{"path", "tools", "override"}` — `$P/mise.toml`, `$P/.tool-versions` and the file
  `MISE_GLOBAL_CONFIG_FILE` names, in that order; untrusted: rc 1, empty stdout (so (d) is not asked);
- (d) `$P/mise.toml` without the key: rc 1, `Key not found: env._.path in <path>`; `$P/.tool-versions`: rc 1, a
  TOML parse error.

The fakes copy these shapes — E7's (the untrusted rows), E10's ((a)'s not-installed row, (b), and the trusted (c) / (d) rows) and E13's (the trusted rows;
its subject is item 3's paths); E8's, E9's and E11's answers copy R§4.2's rows (records :292–299 and :326, mise
2026.10.21 / .25) and E12's `["altbin"]` R§0's; E6 has no fake. E14–E16's answers (garbage or exit 2, a hang, an
over-long stderr) are authored fault injections, not mise shapes. E17's and N14's fakes answer as E10's, E18's as
E8's `MISE_PYTHON_VERSION` row (so (a) shows it), and N12's combine E8's (a) row with E11's (b) row — copies, not
new shapes. §10 step 4's real run checks that a real mise's answers
still parse (its (c) prints one path per line, not item 6).

**The refusal (body :432–456; C31, C37).** `refusal_text(…)` (NEW) is pure; `main` prints it to stderr, then the
exit line (below), and returns 1.
What it prints is body :435–445's items 1–6, in that order — the one enumeration (body :433); this plan fixes only
their spelling. Every line starts `python-pin: `:
1. `REFUSED: the running interpreter is not the pin.`, `pin: <X.Y.Z> (<root>/.tool-versions)`, `running: <a.b.c>
   <releaselevel>`;
2. `sys.executable: <value>` (C37 (b): one fixture runs the child with its real value);
3. `cwd: <cwd>`, `repository root: <root>`;
4. the warning lines (below);
5. one labelled block per question: (a) per element `version=… source=… installed=… install_path=…`, each value
   `json.dumps`-ed as mise gave it, `(absent)` for a missing key, and `[]` as body :421 words it; (b) and (d) the
   stdout, or `exit status <n>`; (c) one path per line (each parsed record carries only the field its printer reads:
   (a) `items`, (c) `paths`);
6. item 6's fact (not on PATH = `FileNotFoundError`; "does not parse" = not a JSON array of objects, or JSON that
   `json.loads` cannot take — a `ValueError` or a `RecursionError` on deep nesting, never a traceback), then `mise stderr
   (verbatim, up to 4000 characters):` and the capped text; (d) is not asked when (c) failed.

Then `GENERIC_BLOCK` (NEW): the text of body :447–453 from "To run on `<pin>`" to `config ls`, without its two
cross-references ("which PR-1 rewrites, §4.4" and "§9 step 4"), formatted with `<pin>` and `<root>` only and
byte-identical in every refusal (its last sentence is C27's three commands).

**Pin-file errors are not the refusal.** When `read_pin` raises, `main` prints `python-pin: pin file <root>/.tool-
versions: <why>` and `the pin file is exactly one line, python X.Y.Z`, then the exit line, and returns **2**: no version can be compared
and the remedy is the file, so neither the oracle nor the generic block — whose first words name `<pin>` — applies.
The driver's Step 1 turns that exit into `FAIL pin` on every run, CI's included, before any suite (so T5's clause 1
never sees a malformed live pin in CI; W1 reaches its parse branch in-process).

**`main()`** (NEW): print the warning lines; read the pin (exit 2 on error); return 0 if `admitted(sys.version_info,
pin)`, else ask mise and print the refusal, return 1.

**The exit line (the one statement of the contract).** Every non-zero exit of the module ends its stderr with exactly
one line `python-pin: exit N` (N = 1, 2 or 3), printed by `_exit(code)` (NEW) as the last write; exit 0 prints none.
It is how a caller tells this module's exit from a failure of `python3` before the module ran (the driver's Step 1,
§4.3). **Exit 3**: the `__main__` guard calls `_guarded_main()` (NEW), which runs `main()` and, on any uncaught
exception other than `SystemExit`, prints its traceback (`traceback`, imported inside `_guarded_main` only) and
then the exit line, returning **3** — so an exception inside the module ends with the marker too, and the driver's
"did not end with `scripts/python_pin.py`'s exit line" message fires only when no exit line appears. Pinned by N23 (its exit-3 case); NM15 / NM16 (`return _exit(1)` →
`return 1`, `return _exit(2)` → `return 2`) and NM17 (`sys.exit(_guarded_main())` → `sys.exit(main())`) are its mutants.

**Header (C15, C16 (f)).** A pointer to the bump rule (body §4.1, :331–334, and its rows; C15, rounds :445–447,
places the pointer, not the rule, here; C16 (f)'s "the header keeps the bump rule", rounds :463, is read as that
pointer — PR-1 corrects that wording by an appended rounds entry, §4.7), and the oracle probe recipe with the program's isolation lines (body :113–117): R§11 *T1*'s
environment block, then questions (a)–(d) — the
"probe scripts" R§4.2 :309–310 says this plan records (the `probe.sh` / `probe2.sh` files are not in the tree; the
recipe is re-derived from R§11). The re-measure rule is N17, not a header sentence (C24); the header names the
measurement's place as "section 10 step 4 of the plan", and states the exit-line contract (above).

### §4.3 The driver (`scripts/python-suites.sh`)

- **Step 1** (`:156–166` at `660e594c`): after `cd "$root"`, `command -v python3` (else `FAIL pin: no python3 on
  PATH`), then `python3 -I -S "$root/scripts/python_pin.py"` with its stderr shown live and teed to a `mktemp` file under
  `${TMPDIR:-/tmp}` (a `mktemp` failure is a `FAIL pin` line), the `EXIT` trap installed before it so an interrupt
  leaves no file. On a non-zero exit `N`, the last captured line decides
  the message (the exit line, §4.2): `python-pin: exit N` → `python-suites: FAIL pin: scripts/python_pin.py refused or failed (exit N; its output is
  above)`; any other last line → `python-suites: FAIL pin: python3 did not end with scripts/python_pin.py's exit line
  (exit N)` (e.g. a mise shim refusing an untrusted config, or a kill with N > 128; an uncaught exception in the
  module is not this case — it ends with `python-pin: exit 3`, §4.2, and takes the first branch); both exit 1 before any suite. `-I -S` stay (the admission is the interpreter's property). `python3 -VV` is echoed
  after it. The cwd mise is asked at is `$root`, where the shim resolved this `python3`. No `(3, 9)` remains.
- **Header** (`:10–12`, `:15–19`, `:129–132`): "Python >= 3.9 standard library" → "the pinned interpreter's standard
  library"; step 1 → the pin (exact, `scripts/python_pin.py`, measured by the `tools` job's one leg, the warning on
  a set `MISE_PYTHON_*` except `MISE_PYTHON_VERSION`, or `MISE_GITHUB_ATTESTATIONS` — §4.2's rule).
  The "pins … live in" sentence (`:129–132`, the pin layer = `PIN_FILES`) is unchanged; after it, one new sentence:
  `scripts/test_python_pin.py` (Step 1's reader and refusal) and `scripts/test_tracked_tool_config.py` (T5) are set
  members outside the pin layer — neither imports a pin file, and no pin file imports them.
- **`Scratch`** (`scripts/python_suites_fixtures.py:126–136`): copies the live `.tool-versions` and
  `scripts/python_pin.py` beside the driver it already copies; the docstring says why (§2.3, scratch × I1). Each
  scratch tree gains one tracked `.py` (P8's `" 0 tracked .py parsed"` half stops discriminating; its red-naming half
  does — tests memo §5 E19).
- **`base_env()`** (`scripts/python_suites_fixtures.py:81–83` at `660e594c`; I7 (iii), §2.3 scratch × I7) **isolates**
  (the one statement of what it does): it returns the caller's environment without any `GIT_*`, `MISE_*` or
  `BASH_FUNC_*` variable and without `BASH_ENV` / `ENV` (a bash start-up file or an exported function reaches a
  scratch bash driver and could rewrite `PATH` past `CHILD_PATH`), with `PATH` replaced by `CHILD_PATH` (NEW
  constant) = `os.path.dirname(sys.executable)`, `/usr/bin`, `/bin` — the admitted interpreter's directory first,
  then the two directories that hold `bash`, `git` and the POSIX utilities the scratch driver calls on both macOS
  and `ubuntu-24.04` — and sets mise's isolation: `MISE_DATA_DIR`, `MISE_CACHE_DIR`, `MISE_STATE_DIR`,
  `MISE_CONFIG_DIR` (`MISE_DIR_VARS`, NEW) as directories and `MISE_GLOBAL_CONFIG_FILE` (`MISE_GLOBAL_VAR`, NEW) as
  an empty file, all under one root `mise_isolation_root()` (NEW: one `mkdtemp` per process, made at first use,
  removed at exit by `atexit`), plus `MISE_OFFLINE=1` and `MISE_AUTO_INSTALL=false`. Every launch of I7 (iii) (§2.1,
  its sites and grep) takes its environment from it, and children of a scratch run inherit it, so `python3`
  resolves to the admitted interpreter, and **whatever mise a child does reach** — a `mise` in any `CHILD_PATH`
  directory (a distribution package in `/usr/bin`, or a package manager's `bin` holding both, e.g. Homebrew's) —
  writes only under that root (it may still read system or ancestor config), offline, and installs nothing; no outer `MISE_*` reaches a child. The
  variable names are a copy of the helper's `MISE_DIRS` / `MISE_VARS` (§2.3 *`base_env()` × the helper*: the fixtures
  module is a pin file and cannot import the helper, P6); the driver pin asserts the two lists agree (N22).
  *Superseded design*: the earlier draft raised `AssertionError` when `CHILD_PATH` held a `mise`, "raise, not
  isolate", on the ground that the isolation's one home is the helper, which a pin file does not load — false: the
  driver pin loads the helper by path (`test_python_suites_driver.py`'s `H`), and the raise made the pins
  unrunnable on any host with a distro mise. Whether `os.path.dirname(sys.executable)` can be mise's shims
  directory is **unverified**; if it can, a child's `python3` is a shim that runs under the isolation above (it
  reaches no user state), and N22's `realpath` assertion is vacuous (both sides resolve to the shim's target). The
  docstring says why (§2.3, scratch × I7). Pinned by N22 (four parts, and its reachable-mise case) and FM1 (tests memo §6.1).

### §4.4 CI — the `tools` job and the `ci.yml` reader

**`ci.yml`** (`:219–283`): `strategy` goes; `actions/setup-python@v7` takes `with: python-version-file:
.tool-versions` and nothing else; the job name stays `Python suites` (check names lose the `(3.9)` / `(3.x)` suffix;
the ruleset has no required check, CLAUDE.md `## CI`); `runs-on: ubuntu-24.04` with the comment "the manifest serves
builds per image; the pinned patch must have one for ubuntu-24.04 x64 (umbrella §4.1, row 2). Re-check at each bump
and when ubuntu-24.04 retires." The comment block: the job-boundary property "`actions/setup-python`, on every leg" →
"…, reading the pin file `.tool-versions`"; the "The legs:" paragraph → one paragraph (one leg, the pin; Step 1
admits only it); the timeout rule "(the slower leg bounds the job)" → "(the job's one leg)"; the dated records of
#549, #510 and #586 stay as records; PR-1's own record is added (§7); the driver step's comment "It asserts the floor"
→ "It admits only the pin (Step 1)".

**The reader** (`scripts/python_suites_readers.py`): `TOOLS_JOB_KEYS` loses `strategy` (N6); `MATRIX_EXPR` and
`q7_problems` go; `setup_problems(job, di)` (NEW) returns red unless: exactly one step whose `uses` starts
`actions/setup-python@` — compared case-insensitively, so `Actions/Setup-Python@v7` counts — exists **in the whole
job**, it precedes the driver step, its keys are within
`SETUP_STEP_KEYS`, its `with` is a mapping whose key set is exactly `{"python-version-file"}` and whose value is the
plain or quoted scalar `.tool-versions`; `runs-on` is exactly `ubuntu-24.04` (`TOOLS_RUNS_ON`, kept); and
`before_driver_problems(steps, di)` (NEW) is empty: every step before the driver is a mapping, its `uses:` (if any)
names `actions/checkout` or `actions/setup-python` (`PRE_DRIVER_USES`; owner/repo before `@`, case-insensitive), and
its `run:` (if any) names none of `.tool-versions`, `GITHUB_PATH`, `GITHUB_ENV` (`PRE_DRIVER_RUN_MARKERS`,
case-insensitive substring) — three names through which a step could change the pin or the `python3` on `PATH`. A second
setup-python step anywhere is red as CI's second spelling (I1 × I3; R§9 takes it off the tamper slot's item (1)).
`tools_problems` calls it in `q7_problems`' place. The YAML subset reader is unchanged (no stdlib YAML parser on any
version — the text half of A-iii's reason stands).

**What the reader does not see** (stated, not guarded): the markers are a substring test over the `run:` text, so a
step that reaches the same effect without naming them (the name assembled or held in a variable, a tracked script
it calls) passes; steps after the driver are not scanned; and a `uses:`
action other than checkout / setup-python is red by name, not by what it does — checkout's own inputs are not
read. These sit with the tamper slot `#11-aiii-gate-tamper-hardening` (§8.3's re-evaluation), not with I3.

### §4.5 `mise.toml`, its reader, and T5's clause 1

**`mise.toml`** (C10, C21, C36):
- a `[settings]` table placed **before `[tasks.check]`** (so no later header can capture the lines), holding exactly
  `python.compile = false` and `python.github_attestations = true` — the text of `tracked_tool_config.ADMITTED_LINES`,
  no indentation, no comment on the line — under a comment that spells no `python` outside a tracked path: the install
  is an attested prebuilt or nothing; `mise` reads neither this table nor `[bootstrap.packages]` in an untrusted
  worktree (`mise trust`; `main` already refuses so since #535 — C36), and the pointer "the interpreter-pin umbrella
  plan memo (docs/plans)" naming I6 (no path in `mise.toml`: D4);
- `[tasks.tools-test]` `description` (`:122`, PR-0's D4 text) → `"Run every tracked suite (test_*.py), offline, on the
  pinned interpreter (`.tool-versions`; `mise run` installs it if missing)."` and its comment's "the interpreter floor"
  (`:124`) → "the interpreter pin (Step 1)". Neither spells `python` (C21).

**The reader → `tomllib`** (C3; body :270–277). `mise_problems(text)` parses with `tomllib.loads` (a
`TOMLDecodeError` is one problem naming its message) and asserts over the document: `tasks` a table;
`tasks.ci` a table with keys within `{description, depends}`, `depends` a list of strings containing
`"tools-test"`; `tasks.tools-test` a table with keys within `{description, run}` (an allowlist — `sources`,
`outputs`, `env`, `dir`, any future key red, naming it; M2 generalised, A-iii D1's direction), `run` exactly
`bash scripts/python-suites.sh`. Deleted, no shim (CLAUDE.md): `_TKEY`, `_THEADER`, `_TPART`, `_toml_key_path`,
`_toml_value`, `_CLOSED`, `_scan_strings`, `mise_sections`. The module docstring: "`ci.yml` is read as text (no
stdlib YAML parser exists on any version); `mise.toml` with `tomllib`, which every interpreter the driver admits has."
The settings are **not** this reader's (next paragraph).

**Where the I6 assertion lives (delegated by PR-0 :560–564; body :187 says "PR-1's reader asserts both values" and
the body :564 row lists "their assertion" with the `tomllib` reader — this plan places it in the member instead, for
the reasons below, and amends both statements in place, §4.7).** In the T5 member,
`scripts/test_tracked_tool_config.py`, as W2, whose expected value is `tomllib.loads("[settings]\n" +
"\n".join(ADMITTED_LINES))["settings"]`, and which asserts that the live root `mise.toml`'s parsed `settings` is a
table **equal to it exactly** — the `python` entry and the key set.
Why there, not in the readers module: (i) **one source** — the member already imports `tracked_tool_config`, so W2
takes the lines from `ADMITTED_LINES` by import; (ii) **P6** — the readers module is a pin file, so its importing
`tracked_tool_config` would pull that module and, through clause 1, `python_pin` into `PIN_FILES` and G9's scan
(PR-0 D2 reason 4), a coupling no invariant asks for; (iii) **cohesion** — clause 4 (b) admits the two lines as text
and W2 pins their table and values: one property (T5 × I6), one member. **No other `[settings]` key is admitted**
(the fail-safe direction): mise's settings surface is unpinned like its configuration surface (body I1 × I6), a key
that does not spell `python` is invisible to clause 4, and whether any such key changes a Python install is not
measured — so an unknown key is red, naming it, and a PR that wants one changes W2's expected table in review
(`main`'s `mise.toml` has no `[settings]` table at `660e594c`: `git show 660e594c:mise.toml | grep -n '^\['`). The
readers module keeps its subject (the gate's wiring: `ci.yml`'s job, `mise.toml`'s tasks).

**Clause 1** (body :369–372; PR-0 §4.3 *Clause 1's place*). In `scripts/tracked_tool_config.py`:
`root_pin_problems(tracked, read)` (NEW) — red, clause 1, when `.tool-versions` is not in `tracked` ("the root pin
file is not tracked"), when `read` raises `Unclassifiable` (its kind; the existing `_errno_kind` / reader is called,
not edited), or when `parse_pin` raises (its `why`); it calls `finding` and nothing else of clauses 2–5.
`problems` becomes `root_pin_problems(tracked, read) + member_problems(tracked, read)` over `tracked = list(tracked)`
(both finders walk it; an iterator would be spent by the first): clause 1 first, then clauses 2–5 in their own path
order — concatenated from the two finders' lists, no finding text parsed back (a sort that re-parsed the formatted
findings broke on a newline or `: ` in a path). `root_pin_problems` reads, decodes and parses in one `try` whose
`except` arms (`Unclassifiable`, `UnicodeDecodeError`, `PinFileError`) each return the one clause-1 finding. Its remedy
`_REMEDY_PIN` (NEW): "the root .tool-versions is exactly one line, python X.Y.Z (the pin); another tool goes into
mise.toml [tools]". The module takes the parser only: `from python_pin import PinFileError, parse_pin`
(`sys.path[0]` is `scripts` under the driver) — `python_pin` also holds `ask_mise`, which launches, so the module
binds no name that can. Restated (statement class): the module docstring's first line "T5 clauses 2-5" (`:1`), its
"clause 1, added later" and "Python 3.9 syntax" (`:9`, `:13`), the `CLAUSES_2_TO_5` comment (`:28`,
"`problems` (the composition point PR-1 edits, with C0's two lists)") → every top-level name except `problems` and
clause 1's names, C0 holding it, without the PR-1 seam (the next paragraph says why it went), and `problems`'
docstring "in PR-0, clauses 2-5" (`:233`) → clauses 1–5.

**What the member's fixtures change** (supersedes PR-0 :293's "edits only `problems` … plus C0's two lists" and
:384's "g1 is the one existing control PR-1 edits, and C0 … nothing else in C0" — the one statement of why; §2.3 and
§4.7 refer here). That seam assumed clause 1 could join `problems` without touching a fixture, and it cannot:
with clause 1 merged, every fixture world whose tracked list lacks a root `.tool-versions` gains a clause-1 finding,
so C3 r1–r4's `only(…)`, C4's `only(…)` and `== []` cases, C5's exact list, L2 and `test_mutants_are_killed`'s
unpatched pre-run all turn red). T5 stays one predicate, `problems` over clauses 1–5; the **valid world is the
fixture layer's baseline**. In `scripts/test_tracked_tool_config.py` at `660e594c` PR-1 touches exactly:
- **NEW** `PIN_BYTES` (the live `<REPO>/.tool-versions` bytes, read at import — a malformed live pin reds every
  fixture world, which is the right answer) and `pinned(tracked, read)` (NEW): returns `tracked` plus `ROOT_PIN`
  and a `read` that answers `ROOT_PIN` with `PIN_BYTES` and delegates every other path — so a clause-3 `read` that
  raises if called (C3 r4) is still never called;
- `probs` (`:97–100`) is built on `pinned` unless its `files` name `ROOT_PIN` themselves (W1's clause-1 cases);
  `mise_probs` (`:103–104`) inherits it;
- the direct calls that bypass `probs` become `t.problems(*pinned(…))`: `:233` (C3 r4), `:297`, `:307`, `:317`,
  `:319`, `:325` (C4), `:375` (both calls) and `:384` (C5); C5's expect list is unchanged (a valid pin adds no
  finding);
- C3 g1 (`:234–235`) is rewritten into W1's green case (the root `.tool-versions` holding the live line is green
  **and read**);
- L2's scratch repository (`:437–443`) writes a real `.tool-versions` holding `PIN_BYTES` and adds it with the same
  `git add -f` line; M1 (`_walk_lister`) still dies there (it lists the untracked files);
- C0: the comment of `MODULE_IMPORTS` (`:107`, "none can launch a process" — restated: none launches when imported,
  and from `python_pin` only the parser is taken) and the tuple gaining `python_pin`; the `Import` check (`:143–146`,
  which admits any `MODULE_IMPORTS` name) rejecting `import python_pin` (it would bind the module, `ask_mise`
  included); the `ImportFrom` check (`:147–152`) admitting from `python_pin` exactly `PinFileError` and
  `parse_pin`; the seam assertion (`:171–173`)
  excluding the clause-1 names (`root_pin_problems`, `_REMEDY_PIN`) beside `problems`, and its comment (`:162`,
  "minus `problems`") restated "minus `problems` and clause 1's names"; the existence loop
  (`:175–176`) running over `CONTROLS_2_TO_5 + CONTROLS_1` (tests memo §6.4).
- the module docstring's "T5 clauses 2-5" (`:1`) → clauses 1–5;
- `:427` (L1, the live tree) and every other line are unchanged; PR-0's `MUTANTS` table and its exact ID assertion
  are untouched — PR-1's mutants are a separate table (tests memo §6.5). PR-0's plan `:293` and `:384` get in-place markers
  (§4.7).

### §4.6 Discovery — found by construct, classes decide the edit (C1, C2, C26, C30; body :494–500)

Discovery, run at PR-1's base and again at its final head (word edges spelled out, never `\b`, C2):
```sh
# D1 — live surfaces, repo-wide (landed records excluded; the VM P4 family has its own stages below)
git grep -nE '3\.(9|1[0-4])([^0-9x]|$)|"3\.x"|\(3, ?9\)|(^|[^A-Za-z])(floor|legs?|matrix)([^A-Za-z]|$)|setup-python|python-version|Python 3\.[0-9]+\+|version_info|except ImportError' -- ':!docs/plans/**' ':!docs/audits/**' ':!crates/**' ':!docs/design/**' ':!*.lock' ':!.claude/tools/plan_memo*' ':!.claude/tools/plan-memo*' ':!.claude/tools/test_plan_memo*'
# D2 — the landed A-iii memo (amended in place, §4.7)
git grep -nE 'floor|legs?|matrix|tomllib|3\.9|UNCHECKED|text readers?' -- docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md
# D3 — the VM P4 family: vm-p4 §5.2's stages 1, 1b, 1c, 2, 2c, verbatim, with the revision 660e594c (vm-p4 :21–39)
```
**Classes** (each hit gets one; the PR body lists every hit with its class):

| class | edit | at `660e594c` (D1 / D2 / D3) |
|---|---|---|
| **statement of the requirement** | rewritten to the pin | driver header `:10–12`, `:15–19` and Step 1 `:156–166`; `mise.toml:122`, `:124`; `ci.yml` `:226`, `:233`, `:239–241`, `:244`, `:270–271`, `:281`, and the job's `strategy` and setup step `:266–269`, `:276–278` (§4.4); README `:72`; CONTRIBUTING `:12–13`, `:56–57`; `CLAUDE.md:100` (CI/tools'), `CLAUDE.md:54` (V14); `scripts/tracked_tool_config.py:13`; the readers' docstrings (`:1–7`, `:436–438`); V1–V4 |
| **floor-motivated code branch** | removed with its coupled rows in the same commit, plus a control and a mutant for the behaviour the removal leaves (CLAUDE.md's checker rule) | V5 (`plan_memo_selftest_properties.py:310–313, 325–329`), V6 under outcome (i) and its coupled V7–V11, V15; V6's control `resolve_runtime_error_control` and mutant "V6 sibling" (§8.1 row 2) |
| **live rule or present-tense description in a landed memo** | amended in place, dated marker | A-iii (§4.7); #510's memo V12, V13, V15; seam-split V16, V17 |
| **a ledger claim tied to an event that has happened** (stage 1b) | amended in place, dated marker | V18 (#510's memo `:1097–1108`), V19–V21, V23 (slot ledger `:58–60`, `:322`, `:323`, `:327`) |
| **a call the pin deprecates** (stage 1c / 2c) | keyword form, every copy in one commit, manifest regenerated | V22 (`plan_memo_sibling.py:158`; `plan_memo_selftest_mutants_codex0927.py:51`, `:52`, `:56`, `:57`) |
| **pin / reader** | tests memo §6's rows | `python_suites_readers.py:385–470`; `test_python_suites_ci.py`'s fixtures and Q7 / R2 / R6 / R9; `test_python_suites_driver.py:79–114` (Q5) |
| **dated measurement** | kept as the record of its date, or re-derived under its rule | `ci.yml:247–257` (kept; PR-1 adds its own, §7); A-iii §8's local measurements (`:535–538`, framed at #510's `06926607`, "The 3.9 leg is the slower one" included); vm-p4 :88–99's kept rows; `plan_memo_emphasis.py:103–104, 113`, `plan_memo_tokens.py:108`, `plan_memo_selftest_harness.py:428`, `plan_memo_selftest_manifest.py:200–204` |
| **not a target** | unchanged; named in the PR body | `ci.yml:62–64` (the `check` OS matrix), A-iii's shell floor (`:231–235`, A1's `trip-wires` floor of shell / `git` / `grep`), "legacy endpoint" (`:340`) and "the text readers' grammar" (`:30`, what earlier drafts wrote), `codeql.yml` (language matrix), `floor` in `.claude/tools/webref-generic-core-trip-wire*`, `Cargo.toml` patch floors, `plan-sweep.py`, the skills' "edge matrix" / "floor" prose, `.plan-z1b-consume-delta.md`, `test_python_suites_regime.py:806`, `CLAUDE.md:30` (the edge matrix), PR-0's fixture data (`test_tracked_tool_config.py:75`, `:83–84`, `:228`, `:247–257`, `:369`, `:378`, `:439–441`: `main`'s old text, file names and fixture versions), T5's file-name class `.python-version*` (`tracked_tool_config.py:7`, `:114`, `:142–143`: a name clauses 2 / 3c match, not an interpreter version), the driver's parse-failure message naming the running interpreter (`python-suites.sh:453`, `sys.version_info`), vm-p4 :88–102's non-interpreter words |

The A-iii UNCHECKED line (`:441–443`) is rewritten by PR-1 to "the pin on `SKILL.md`'s direct `preflight.py` path —
UNCHECKED, assigned to PR-2 of the python-pin umbrella" (body :529–531); PR-2 flips it (C13).

### §4.7 The landed memos — amended in place

Every amendment (the rounds file excepted, below) replaces the sentence (or the clause it names) and appends `(amended <PR-1 landing date> by #<PR-1>, python-pin PR-1)`.
- **A-iii** (`docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md`): the user-ratified D3 (`:327–329`), §8
  (`:523`), §12(1) (`:586`) exactly as body :514–516; and the statements PR-1 falsifies (body :526–531): L5 (`:104`)
  and the five `× L5` rows (`:114`, `:118`, `:121`, `:123`, `:125`) to I2 / I3 wording; §4.4 (`:312–332`) to the pin,
  one leg, the image reason, Step 0's path → PR-2; §4.2 step 1 (`:162`); §6's text-reader rationale and its property (`:416–422`) to
  "`ci.yml` read as text, `mise.toml` with `tomllib` (since PR-1)", the construct property scoped to `ci.yml` and
  `mise.toml` red on what `tomllib` refuses and on an unmodelled key (N7 / N8); Q5 (`:434`) → N1, Q7 (`:436`) → N4, Q13 (`:438`)
  and §6.1's M1–M3 (`:474–476`) marked retired → N7 / N8, R2 (`:466`) → N5; §12(6) (`:599`) → "the pin is
  measured: N4 green and the `tools` job green"; §13's Slice-B row (`:612`) → B has no interpreter edit; the §6
  UNCHECKED line (§4.6); and, to the pin / one leg / the `tomllib` reader: §4.2's job sentence (`:150`, "a two-leg
  `python-version` matrix") and helper list (`:157`, "`ci.yml` / `mise.toml` text readers"), step 5's "floor leg"
  (`:201`), the wrapper contract's "matrix leg" (`:215`), §4.2.1's "on every leg" (`:245`), §4.6's `CLAUDE.md`
  paragraph "legs `"3.9"` / `"3.x"`" (`:362–363`), Q4's "§6.1 M1–M2" (`:433`) → over parsed TOML, N7, Q13's "text readers'" (`:438`), R6's
  `python-version: [` half (`:470`, retired with the matrix, tests memo §6.2), §7's "the floor" (`:507`), §9's "on the floor"
  (`:548`), and §10's Q3 closure (`:563`, the draft-3 record kept, the pin appended). Line numbers: `git grep -n -F` of each quoted sentence at PR-1's base.
- **#510's memo** (V12 `:73`, V13 `:1116–1117`, V15 `:434`, V18 `:1097–1108`) and **#506's** (V16, V17, V19–V21,
  V23): §8.1 (the consent's conditions decide their text and their pointers).
- **PR-0's plan** (`docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md`): in `:291–293`'s sentence only the
  clause from "and edits only `problems`" to the sentence's end ("… plus C0's two lists (§6: …).") is replaced, and
  `:384`'s "g1 is the one existing control PR-1 edits, and C0 the one existing control whose two lists PR-1 extends
  … and nothing else in C0" is replaced, each by the pointer "superseded: PR-1 also edits the module's statements,
  the member's fixture layer and C0 as its plan §4.5 lists" and the marker (the reason is §4.5's). Kept, not edited:
  that sentence's opening (clause 1 as its own function calling nothing of clauses 2–5 but `finding`), and
  `:293–295`'s review condition per function span (as `:58–59` and `:139–140` state it).
  The sweep at `660e594c`: `git grep -n -F "C0's two lists" -- docs/plans scripts` prints PR-0 `:293` and
  `scripts/tracked_tool_config.py:28` (§4.5); `git grep -n -F "PR-1 edits" -- docs/plans scripts` adds PR-0 `:59`,
  `:139` (the kept condition), `:384`, and umbrella / rounds / vm-p4 lines on other subjects (the VM P4 lane's
  lines and checker run, the J1 row, clause 1's placement in the PR-0 split at umbrella `:627`);
  ``git grep -n -F 'minus `problems`' -- scripts docs/plans`` prints `scripts/test_tracked_tool_config.py:162`
  (§4.5), PR-0 `:293` (the kept review condition's sentence) and PR-0 `:367` (C0 as PR-0 lands it, not a claim
  about PR-1: kept); `git grep -n -F 'clauses 2-5' -- scripts docs/plans` prints `scripts/test_tracked_tool_config.py:1`
  and `scripts/tracked_tool_config.py:1`, `:233` (all §4.5).
- **The umbrella** (PM 353; precedent: PR-0 edited the body; C38 decides the landing vehicle, not an edit bar) —
  the statements PR-1 falsifies, each re-read at `660e594c`. The body is 994 lines at the base; if the markers
  take it past 1000, body :752's seam rule applies in PR-1. The rounds file is "append-only" (body :744; PM 355):
  its two corrections are one appended dated entry at its end (§14 *python-pin PR-1 correction (2026-11-04)*), and each corrected entry keeps its text and gains
  only a one-line "superseded by" pointer to that entry:
  - body :187: the clause "and PR-1's reader asserts both values" → "and PR-1's T5 member
    (`scripts/test_tracked_tool_config.py`, W2) asserts both values over parsed TOML"; marker after "pinned image." (:188);
  - the body :564 row: the clause "with `[settings] python.compile = false` + `python.github_attestations = true`
    and their assertion" → "and the `[settings]` lines `python.compile = false` + `python.github_attestations =
    true` (their assertion is in the T5 member, PR-1 plan §4.5)"; marker at the cell's end;
  - body :271–274: the sentences from "That is the consequence" to "T5b is retired, §4.2)." → "That is the
    consequence of **I2 × I3** — one admitted interpreter, one leg, the pin, which has `tomllib` — and of §13 C3
    (PR-1 absorbs PR-3); I6's assertion of the two settings also reads parsed TOML, in the T5 member, not through
    this reader (PR-1 plan §4.5)." and the marker;
  - body :230: the clause "(c) a mutant that drops the helper turns (a) red everywhere and (b) red locally." → "(c) a
    mutant that drops the helper is killed by fixture-owned evidence: a sentinel `mise` / `python3` first on the
    test's `PATH` logs any child that bypassed the helper (PR-1 tests memo
    `docs/plans/2026-10-python-pin-pr1-tests.md` §6.5, N19 (c))." and the marker;
  - body :241: "seven constructs that 3.9.6 rejects pass it" → "ten constructs …" (R§11's gap rows, records
    :559–568); marker after "R§11's accepted rows";
  - rounds :463, C16 (f): the entry carries "the header keeps the bump rule" → "the header keeps the pointer to
    the bump rule (C15)"; the pointer after (f)'s end;
  - rounds :598–599, C38: the entry carries "and lists none of its four files in its edit set;" → "and amends the
    body in place and appends to this file where PR-1 falsifies a statement (PM 353; vm-p4 only on PR-1 plan
    §8.4's contingency);"; the pointer after C38's end (:603).
- **CI/tools ledger items 3 / 4** (body :533–548): through the PM (§10).

### §4.8 The I7 helper — `scripts/python_pin_testing.py` (NEW; body :197–234, C23, C28, C34)

Not a set member (no `test_` prefix), not a T5 member (no component starting `mise` / `rtx`), no `TestCase`, no
`__main__` guard, **no process launch** (N21: no call in G9's spawner set), stdlib only. Every test loads it by
path through the one constant `"scripts/python_pin_testing.py"` joined to the repository root (PR-2's named form;
never an assembled `os.path.join(…, "python_pin_testing.py")`). Contents (all NEW):
- `isolated_mise_env(tmp_root, fake_mise=None, extra_env=None)` — `os.environ` minus every `MISE_*`, plus
  `MISE_DATA_DIR`, `MISE_CACHE_DIR`, `MISE_STATE_DIR`, `MISE_CONFIG_DIR` under `tmp_root` and
  `MISE_GLOBAL_CONFIG_FILE` an empty file there; `PATH` = the fake's directory first (or the outer `PATH` with every
  directory holding a `mise` removed, for the no-mise fixture); no regime key changed; `extra_env` last.
- **The fakes' scripts** (`_wrapper`, `_shim`): the fake `mise` and the fake `python3` are one Python script each
  per process, written on first use into one `mkdtemp` directory removed at exit (`atexit`); its shebang is
  `sys.executable` itself (no `-I` / `-E` / `-S`, and no `/bin/sh` between the caller's environment and the fake —
  a `/bin/sh` drops `DYLD_*` under SIP and some names under dash, which falsified N20's comparison). A shebang holds
  one path, so `sys.executable` must have no white space and `#!` plus it must fit 127 bytes (the conservative bound: old Linux kernels' 128-byte
  buffer; 256 since 5.1), else
  `_wrapper` raises `RuntimeError` — a stated limit of the helper. Each `write_fake_*` call puts a **symlink** to
  that script, named `mise` / `python3`, into `bindir`; the script finds its data through the directory of
  `argv[0]` (the symlink's, not its target's). One script per process instead of one per test removes a fresh
  first exec per run (slow on macOS, and inside E15's 1 s budget, so flaky under load).
- `write_fake_mise(bindir, answers)` — the symlink, and `answers` in `<bindir>/answers.json`; the fake appends
  `{argv, environ}` (its whole environment) to `<bindir>/calls.jsonl`, then answers each argv from `answers` (rc,
  stdout, stderr, or `hang`: start `/bin/sleep 3600` holding the inherited stdout / stderr, write its own pid and
  the sleep's to `<bindir>/hang.pids`, and wait).
- `write_sentinel(bindir, names)` — for each name (`mise`, `python3`) a `#!/bin/sh` script that appends `$0 $*` to
  `<bindir>/<SENTINEL_LOG>` (`"sentinel.log"`, NEW constant) and exits 1; `read_sentinel(bindir)` (NEW) returns its
  lines (`[]` = nothing reached them): the evidence N19 (c) and N22 own (no user state is read).
- `LAUNCHER` — a `-c` source string: load the module at `argv[1]` by path under a fresh name, replace
  `sys.version_info` by a tuple subclass with the `major`…`serial` properties built from `argv[2]`, replace
  `sys.executable` when `argv[3]` is non-empty, set `ORACLE_TIMEOUT_S` when `argv[4]` is non-empty, `sys.exit(main())`.
- `write_fake_python3(bindir, version)` — the symlink, and the five-part version in `<bindir>/fake_python3.version`;
  a `python3` on `PATH` for N1: when its arguments name `python_pin.py` it runs that file under `LAUNCHER`'s
  replacement; `-VV` prints the fake version; anything else `execv`s the real interpreter.
- `refused_versions(pin)` (NEW) — E2's versions for a pin triple, derived, never spelled: `Z-1` (when `Z > 0`),
  `Z+1`, `Y-1`'s `.10` (when `Y > 0`), `Y+1`'s `.0`, `3.9.6` (unless it is the pin), and the pin's
  `candidate 1`, each as (spec, shown, level). The driver pin takes the pin from the `.tool-versions` text (the
  live file at import; a scratch tree's own copy), so no pin file spells the pin a second time and none imports
  `python_pin` (P6).
- `pin_tree(dst, module_source, pin_line)` — `dst/scripts/python_pin.py` and `dst/.tool-versions` (`pin_line=None`:
  no pin file).

Children run as `[sys.executable, "-c", LAUNCHER, …]` with the helper's environment, without `-I` / `-E` / `-S`, so
they stay outside the driver's unguarded-child class (body :219–222). The launcher of those children is
`test_python_pin.py`'s `run_child` (NEW, in the member — the helper launches nothing): `subprocess.run(…,
timeout=60)` (60 s default; `run_pin(child_timeout=…)` passes it on, 5 s for E15's run), a `TimeoutExpired` re-raised as `AssertionError` (so a hang is a red control, and a mutant that hangs is
killed); every fixture that writes `hang.pids` registers a cleanup killing each recorded pid (`os.kill(pid, 9)`,
`ProcessLookupError` ignored; a pid `poll_gone` already saw gone is skipped, since it may have been reused),
so no run — mutated or not — leaves a survivor. Every caller reaches the helper's
functions as attributes of the loaded module object (never a name bound from it), which is what NM-I7 patches.

---

## §5 / §6 Edge matrix; tests and controls — in the tests memo

Both sections live in `docs/plans/2026-10-python-pin-pr1-tests.md` ("the tests memo"), under the same numbers:
§5 the edge matrix (E1–E29) and the behaviour deltas (C22); §6 the tests and controls — §6.1 the driver pin (N1,
N1b, N1c, the pre-launch case, N22, DM / FM), §6.2 the `ci.yml` / `mise.toml` reader pin (N4–N8), §6.3
`scripts/test_python_pin.py` (N2, N3, N9–N21, N23), §6.4 the T5 member (W1–W3), §6.5 the NM / WM mutants. Every
E-row, N / W control and DM / FM / NM / WM mutant this plan names is defined there; "§5" / "§6.x" in this plan
mean the tests memo's sections.

---

## §7 Scope

**Edit set** (`(NEW)` = new file):
- `.tool-versions` (NEW); `scripts/python_pin.py` (NEW); `scripts/python_pin_testing.py` (NEW);
  `scripts/test_python_pin.py` (NEW);
- `scripts/python-suites.sh`, `scripts/python_suites_fixtures.py`, `scripts/python_suites_readers.py`,
  `scripts/test_python_suites_ci.py`, `scripts/test_python_suites_driver.py` (`python_suites_fixtures.py`:
  `Scratch`'s copies, `base_env()` and `CHILD_PATH`, §4.3);
- `scripts/tracked_tool_config.py` (clause 1, `problems`, imports, docstring), `scripts/test_tracked_tool_config.py`
  (W1–W3, WM, and exactly §4.5 *What the member's fixtures change*);
- `mise.toml`, `.github/workflows/ci.yml`, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md` (`:54` V14, `:100`);
- the A-iii memo, PR-0's plan, and the umbrella body and rounds file (§4.7); vm-p4 only on §8.4's contingency;
  the VM P4 lane's V1–V23 files and `.claude/tools/plan_memo_selftest_manifest.txt` (regenerated, §8.1); this plan and its tests memo
  `docs/plans/2026-10-python-pin-pr1-tests.md` (NEW; §5 / §6).
**Not touched**: `scripts/test_python_suites_regime.py` (body :725–728), the records file, PR-2's entry points and
`SKILL.md` Step 0.

**1000-line rule** (body :714–734): `git ls-files -z -- <edit set> | xargs -0 wc -l | sort -n | tail -n 12` at the
base prints `scripts/python-suites.sh` 797, `scripts/python_suites_readers.py` 678, `scripts/test_tracked_tool_config.py`
555, `scripts/test_python_suites_driver.py` 489, `scripts/test_python_suites_ci.py` 378 (and #510's memo 1707, the
stated exception, body :728–734; the umbrella body 994, §4.7). Expected after PR-1: readers shrink (the TOML grammar goes), the driver grows by Step 1, `test_tracked_tool_config.py` grows by W1–W3 and WM (re-measured at touch time; a split first if it
would pass ~900); the new files stay well under 1000. `plan_memo_selftest_controls.py` (997 at the base) changes under (i) by
V9's deletions (the `RuntimeError` arm and its sentence; "(an over-long name)", §8.1 row 2), V10's restatement
within its own lines, and row 2's control (`_resolve_control` + `resolve_runtime_error_control`, folded so the file
grows by two): **999 lines** (`wc -l`), one under the bound consent item 3 names (no prereq split; it fails at ≥
1000). The PR body says so and flags it for the VM P4 lane, whose file it is: the next line added there needs the
split first.

**The `tools` timeout** (body :520–524; ci.yml `:243`). PR-1 changes the interpreter and adds members, so it
re-derives under `ci.yml`'s rule from its **own final-head run**: N = ceil(4 × the one leg's end-to-end minutes),
read with the comment's `gh run view <run-id> --json jobs` command. **Raise-only (PM 353, 2026-11-04)**: PM 302's
"raise only when N > 22", given for PR-0 (`ci.yml:257`), is extended to the program, so the value becomes N only if
N > 22 and otherwise stays 22 (PR-1 drops the slower `"3.9"` leg, so its N may be below 22; PR-0's run gave 13). The
run id, the duration, the computation and this rule go into the comment.
**#510's 2× trigger** (its memo `:1123–1125`): the value is ≥ N ≥ 4 × the duration, so the
trigger cannot fire from PR-1's run; the PR body states the ratio. If the run itself is red on time, that is
reported to the PM as #510's trigger fired (body :524).

**No X1-shaped line** (C33): body §11's pattern over this plan prints nothing (§10 DoD).

---

## §8 Coordination

### §8.1 The VM P4 consent — conditions this plan and the PR body carry

The consent file's lines were given at `432c0190` / `eaa741b5`; each is re-read at `660e594c`:

| # | condition (consent file) | where it binds here | re-read at `660e594c` |
|---|---|---|---|
| 1 | V1–V23 + manifest OK; V12 / V13 / V15 / V18 keep or update the pointers into #510's memo | **Rule**: V12 (`:73`) and V15 (`:434`) are rewritten within their own lines, so lines 1–663 keep their count and seam-split `:733` ("`@ …umbrella-checker.md:663`") and `:778` stay true; PR-1 checks `git grep -n -F 'main memo line 2716 at' -- docs/plans/2026-08-plan-memo-umbrella-checker.md` prints `:663` after its edits, and if not, updates `:733` in the same commit (consented). V18 rewrites `:1097–1108`; the new lines of the two slot IDs (`git grep -n -F -e '#11-plan-memo-acceptance-falsifiability-check' -e '#11-plan-memo-spec-field-single-home-check' -- <#510's memo>`) go to the PM for the ledger (§8.3). Seam-split `:573` also cites `:1098` and `:1103` (the `190d2adb:…` cites), inside V18's range: it is a census whose checker-plan lines are read at `e44ffce6` (seam-split `:12–13`), so a move of either cite by V18 does not make it false and it is not edited; PR-1 runs `git grep -n -F -e '190d2adb:…:1218' -e '190d2adb:…:2710' -- docs/plans/2026-08-plan-memo-umbrella-checker.md` after V18 (prints `:1098`, `:1103` at `660e594c`) and names any move in the PR body | #510's memo `:1098` / `:1101` hold; seam-split `:733` holds; **drift**: the ledger rows are now `project_open-defer-slots.md:1056` / `:1057`, not `:1014` / `:1015` (`/usr/bin/grep -n` of each slot ID, 2026-11-04; their post-V18 numbers are on the PM's landing list, §8.3) |
| 2 | V6 = outcome (i); keep `except OSError` with the true reason "defensive, e.g. `getcwd()` failure"; do not carry "3.13 made that an OSError" or "an over-long name"; add the third caller `plan_memo_memo.py:761` | V6: `except OSError:`; `_resolve`'s docstring: non-strict `resolve()` raises no `OSError` for a missing file or a symlink loop on the pin; the arm is defensive (e.g. a `getcwd()` failure); its callers are `sibling_path` (e) (`plan_memo_sibling.py:168`), `plan_memo_population.py:54` and `plan_memo_memo.py:761`, each of which now propagates a `RuntimeError` (E26, O4). `sibling_path`'s (e) sentence (`:151–153`) → "an `OSError` there makes the sibling UNAVAILABLE". V7 find text follows; V8 deleted; V10 / V11 / V15 restated as history. **V9**: the vm-p4 table's edit only — drop the `RuntimeError` arm and its sentence, a deletion (item 3 calls V9 / V10 moot under (i), i.e. no headroom question). **V9's "(an over-long name)"** (`plan_memo_selftest_controls.py:179`): dropped too, with VM P4's consent of 2026-11-04 (PM 353) on its condition that no other cause replaces it; PR-1 names **no cause** (not V6's wording, which is `_resolve`'s reason, stated there once), so the docstring opens "(e) of `sibling_path`: an `OSError` from `resolve()` is the unavailable-sibling schema miss, rc 2 -- not an exception."; the injected `OSError(36, "File name too long")` (`:188`) is a fixture and stays. **The removal's control (CLAUDE.md:54: lexer / population / assertion touched → control + mutant in the same commit)**: `unavailable_sibling_control`'s body is folded into `_resolve_control(M, name, exc)` (NEW: injects `exc` from `Path.resolve` for `name`, returns the result or the exception `check()` let out), which it and `resolve_runtime_error_control` (NEW: an injected `RuntimeError` propagates out of `check()`) call; the mutant "V6 sibling: `_resolve` guards RuntimeError again" (`plan_memo_selftest_mutants_pr510.py`, restoring `except (OSError, RuntimeError)`) is killed by it; the manifest is regenerated (one CONTROL row added, `unavailable_sibling_control`'s digest changed, one MUTANT row). This touches `unavailable_sibling_control`'s body beyond V9's deletion — within V6's consent, reported to the VM P4 lane through the PM (§8.3) | `plan_memo_sibling.py:151–153`, `:174–183`, callers `:168`, `plan_memo_population.py:54`, `plan_memo_memo.py:761` (`git grep -n '_resolve' -- '.claude/tools/*.py'`) |
| 3 | V9 + V10 moot under (i); no prereq split (997 lines; fails at ≥ 1000) | §7's 1000-line rule | `wc -l .claude/tools/plan_memo_selftest_controls.py` → 997 at `660e594c`; 999 after row 2's control (§7) |
| 4 | V22 as written; the four find / replace strings in codex0927 rewritten in the same commit; `--write-manifest` (MUTANT rows `:2122` / `:2126`) | §4.6 class *a call the pin deprecates*; one commit; `python3 .claude/tools/plan-memo-umbrella-check.py --write-manifest` on the pin | codex0927 `:51`, `:52`, `:56`, `:57` (`git grep -nF 'dest, 1)' -- .claude/tools`); manifest `:2122`, `:2126` (`git grep -n -F -e 'R38 of 2026-09-27 agree' -e 'R38 of 2026-09-27 run' -- .claude/tools/plan_memo_selftest_manifest.txt`) |
| 5 | P2: `:333` PR-2's proposal; `:799` delete the bullet | PR-2's (§11) | seam-split `:333`, `:799`, `:801` hold |
| 6 | O4 OK (12-module closure) | §8.2 O4 | unchanged sources |
| 7 | O9 OK (rc 0 on three interpreters); "the mise-built 3.15.2 is not verified" | §10: PR-1 runs the checker on mise's 3.15.2 (the gate's interpreter) and records rc and interpreter in the PR body | — |
| 8 | O8: before PR-1 merges with a red on a VM-P4-written line, report the line and the interpreter or prelude that exposed it to VM P4 via the PM; the merge waits for VM P4's fix or its consent to a fix; no red merge; no rewrite of VM P4 lines without consent | §8.2 O8; §10 DoD | — |

FYI of the consent (`:327`; `:278` false today): textual, whichever lands second re-reads `:327` (vm-p4 :111–113).

### §8.2 PR-1 × #506's lines on `main` (vm-p4 §5.3 (a); C14)

| # | site | PR-1's edit | owner |
|---|---|---|---|
| O1 | `CLAUDE.md:54` | V14: "matrix の両 leg" → "pin の 1 leg" | PR-1, consented |
| O2 | #510's memo | V12, V13, V15, V18 with dated markers (§8.1 row 1) | PR-1, consented |
| O3 | `ci.yml` `timeout-minutes` | the first re-derive measured with #506's guard on the pin (§7) | PR-1 |
| O4 | the guard's twelve-module closure | V6 (behaviour: `RuntimeError` propagates), V11 (comments), V22 (same result) | PR-1, consented |
| O5 / O6 | seam-split `:356–357`, `:804–806` | V16, V17 | PR-1, consented |
| O7 | family sources + manifest | V1–V11, V22, `--write-manifest` on the pin | PR-1, consented |
| O8 | `symbol_attribution_control`'s corpus | this plan joins it; a red on a line PR-1 wrote is PR-1's; a red on a VM-P4-written line follows consent condition 8 | PR-1 / VM P4 |
| O9 | the slot ledger memo | V19–V21, V23 (cells only), then CLAUDE.md:55's checker on the entry memo, exit 0 (C32) | PR-1, consented |

### §8.3 PM-owned records (PR-1 reports, the PM writes)

The slot-ledger edits are on the PM's landing list (`elidex-pm-trial/handoff/pending-at-landing/python-pin-pr1.md`,
PM 353): the two §9 slots' registrations, the two `#11-plan-memo-*` rows' post-V18 numbers (§8.1 row 1), the vm-p4
:151 consent (§8.4) and V9's (§8.1 row 2). PR-1 reports, besides: the CI/tools lane SSoT's #514 checklist items 3 / 4
(body :533–548); the tamper slot `#11-aiii-gate-tamper-hardening` re-evaluation: trigger (b) fired twice (the
`tools` steps; `[settings]` and how `mise.toml` is read) — R§9's items.

For the VM P4 lane (vm-p4 :114–115, carried here because no file holds it): its manifest-regeneration practice
("byte-identical on 3.9/3.12/3.14", #562's commit body) becomes "regenerated on the pin" from PR-1's landing; and
§8.1 row 2's added control and mutant (V6's removal pinned; `unavailable_sibling_control`'s body folded into
`_resolve_control`), which leave `plan_memo_selftest_controls.py` at 999 lines (§7).

### §8.4 Lanes and the interval

Body §8's rows; for PR-1: Dependabot (`ci.yml` `steps:`, textual); A-iii sweep 4 (the driver's Step 1 region,
textual; whichever lands second re-reads); #514 (its re-derive starts right after PR-1, citing PR-1's artifacts only,
decision 124); every lane (from PR-1, the root `.tool-versions` is exactly one `python` line). **Pre-landing sweep**
(body :816–819; C3): A-iii §13's sweep with `.tool-versions`, `scripts/python_pin.py`, `scripts/python_pin_testing.py`,
`mise.toml`, the readers and §8.1's files added; `git diff --stat origin/main...<head> -- scripts` per open PR; T5
(clauses 1–5) on each open PR's head, as PR-0 §8 runs it. **The interval** (body :676–708) opens at PR-1's landing;
the landing report says so. **The vm-p4 :151 contingency** (PM 353): only if N > 22 (§7's raise-only rule) does
vm-p4 O3's "under the unchanged value 22" (vm-p4 :151) become false; PR-1 then deletes ", under the unchanged value
22" only (PR-0 §7's form), with §4.7's marker, and VM P4's consent is obtained by the PM at that time (the PM's
landing list holds it).

---

## §9 Defer slots

Two (body :865), registered by this plan (ledger rows `:1000` / `:1001` point here from PR-1's landing):
- **`#11-toolchain-pin-bump-automation`** — **Defect**: neither `rust-toolchain.toml` nor `.tool-versions` is bumped by
  anything (`grep -n package-ecosystem .github/dependabot.yml` lists `cargo` and `github-actions` only), so a pin can
  fall behind a CPython security release with no alarm. **Why deferred**: the fix is a bot that opens a bump PR (a
  red date-driven job would redden unrelated PRs, I2 × I3), a separate mechanism with its own trust question.
  **Trigger**: a bump PR opened after a security release on the pinned line was already served prebuilt by both
  installers. **Re-eval**: 2026-12-31.
- **`#11-python-pin-single-builder`** — **Defect**: CI measures actions/python-versions' build, local runs
  python-build-standalone's — one CPython release, two binaries (body :353–356, §11). **Why deferred**: one builder on
  both sides means mise in CI (`jdx/mise-action`), the S3 option kept rejected (PM decision 62). **Trigger**: a suite
  whose result differs between local mise and CI on the same pin. **Re-eval**: 2027-03-31.
Decision 170's limit is not a slot (body :866–867).

---

## §10 Landing and DoD

**Landing steps** (body §9 steps 3–4; C20, C32, C35, C36):
1. This plan passes `/elidex-plan-review`; PR-2's plan passes its own before PR-1 lands.
2. Implementation in a dedicated worktree off `origin/main`; `mise run tools-test` green on mise's 3.15.2; `mise run
   ci` before push. **Push rule** (PM 353): each push is judged by the remote ref via the API — `gh api
   repos/send/elidex/git/ref/heads/<branch> --jq .object.sha` equals the pushed head — with at most 2 retries.
3. R§4.1 row 3 at PR-1's base on mise's 3.15.2 (every suite green; every tracked `.py` compiles under `-W error`);
   C35: the set with `PYTHONWARNINGS=always::DeprecationWarning`, `DeprecationWarning` lines counted by source line
   (expected 0 after V22), in the PR body.
4. The real old interpreter (body :887–904; R§11 recipe, `type -P` / `whence -p`, `MISE_TRUSTED_CONFIG_PATHS=<W>`):
   the refusal under macOS `/usr/bin/python3` recorded with `<D>` as `xcode-select -p` prints it, the post-check
   listing of `$ISO/state/tracked-configs`, and `REAL_OLD_RUN` written from that run. The record also shows that the
   real mise's answers parse: item 5 (a) printed per element and (c) one path per line, with no item 6 for (a) or (c)
   (§4.2 *Shapes the fakes answer*: the fakes' shapes are mise 2026.10.25's; this run names the mise it asked).
   Open at the 2026-11-05 record (tests memo §6.3 N17): that run reached no real mise, so this half is re-run at landing on
   the final `python_pin.py`.
5. CLAUDE.md:55's checker on `docs/plans/2026-07-vm-p4-es-language-completeness.md` on the pin: exit 0 (C32; consent
   item 7). The manifest regenerated on the pin.
6. CI: every job green; the `tools` job on its one leg; the timeout derived (§7) and recorded; CI re-run.
7. The pre-landing sweep (§8.4); reds handed off. The X1 pattern over this plan: 0 hits.
8. **Report to the PM**: §4.7's amendments; V1–V23 and the manifest; §8.3's items not on the PM's landing list and its
   note for the VM P4 lane (vm-p4 :114–115: manifest regeneration "on the pin"); the interval opened;
   the tamper-slot re-evaluation; any O8 red (consent condition 8).
9. **Through the PM to every lane** (body :842–848): after rebasing onto PR-1's `main`, `mise trust` if mise reports the
   worktree untrusted (`main` already does since #535; PR-1's `[settings]` is a second such table — C36), then `mise
   install`; report `which -a python3`. The PM tells the user first that installing the pin moves the global `python =
   "latest"` (body §7.2 (4)). Also: the root `.tool-versions` is exactly one line; and **any** `mise run <task>` in a
   worktree without the pin installed now installs it first (tests memo §5 behaviour delta (7)).
10. Re-run body §7.2 (1)–(2) under the then-current mise (named) and `usage-pace.test.py` and `agent-model-floor.test.py`
    (the fourth global Python hook, `~/.claude/hooks/agent-model-floor.py`, PM 360) on 3.15.2; re-measure R§4.2's
    paragraph *The install settings and their overrides* (records :330–339) (C10).
11. Re-measure compile-off **from the tracked file** under the then-current mise (named), with nothing installed: in
    a scratch project holding a copy of PR-1's root `mise.toml` and `.tool-versions`, `$I
    MISE_TRUSTED_CONFIG_PATHS=$P $M install python@<patch>` (`$I` / `$M` as §4.2 *Shapes the fakes answer*, without
    `MISE_OFFLINE=1`, since mise must look for a prebuilt), where `<patch>` is one the plain listing has and
    python-build-standalone does not build (R§4.1's `comm -23` recipe, records :256) → rc 1, `no precompiled python
    found`. This plan's record of it (the scratch `mise.toml` held the two `[settings]` lines and one task): 2026-11-04,
    mise 2026.10.25 macos-arm64, `python@3.11.2` → rc 1,
    `no precompiled python found for core:python@3.11.2 on aarch64-apple-darwin`, nothing under `$d/data/installs`.
    Negative control (nothing installed either), so the setting the refusal runs under comes from the tracked file: `$I
    MISE_TRUSTED_CONFIG_PATHS=$P $M settings get python.compile` (§4.2's `$I`) prints `false`, rc 0, in that
    project, and `mise ERROR Setting [python.compile] is not set`, rc 1, in an empty one (its own path trusted) —
    measured 2026-11-04, mise 2026.10.25 macos-arm64. The fallback when the setting is unset (a source build) is
    unmeasured. The holders of I2 × I6 are then: W2 = the settings are present; this step = mise refuses (a dated record between
    landings, since mise is unpinned).

**DoD / exit criterion** (C22):
- [ ] every interpreter whose version is not the pin is refused by the gate, naming it (N1, N10); the same-version limit
      stated (§2.1);
- [ ] N1–N23, W1–W3 green; every NM / WM / DM / FM killed; each new control red at the base (N2–N21, N23, W1–W3 have no
      subject there; N1 and N22 run red on the base's driver and `base_env`);
- [ ] N4 green on the live `ci.yml`, every E20 variant red; `git grep -nE '_toml_key_path|_toml_value|_scan_strings|mise_sections|MATRIX_EXPR|q7_problems' -- scripts`
      prints nothing;
- [ ] §4.6's D1–D3 re-run on the final head; every statement-class hit rewritten; §4.7's markers present;
- [ ] consent conditions 1–8 met as §8.1; the manifest regenerated; the checker exit 0;
- [ ] `symbol_attribution_control` and the VM P4 guard green with this plan in `docs/plans/`;
- [ ] the timeout recorded; the PM notices sent.

---

## §11 Deferred to PR-2

PR-2 (body :565 row): the launch-evidence predicate and the guard in every derived entry point, consuming
`python_pin.py` and `python_pin_testing.py` unchanged (R§6: a refusal-wording change is PR-1-scope); `SKILL.md` Step
0; the UNCHECKED → CHECKED flips (C13); P2 (`:333` as PR-2 proposed; `:799` deleted — consent item 5); its own
`tools` timeout re-derive in `ci.yml` (C19). Handed over from this plan: (1) the driver's Step 1 launches
`scripts/python_pin.py`, a tracked launch instruction, so the module is in PR-2's derived population — its guard is
itself (it reads no version-sensitive import before admission); (2) the helper's `LAUNCHER` and `write_fake_python3`
are PR-2's E1 fixtures; (3) PR-2's entry points refuse with exit 1 and pin-file errors exit 2, as `main` does.

**PM decisions** (PM 353, 2026-11-04; none open): (1) the vm-p4 :151 contingency, §8.4; (2) V9's "(an over-long
name)", §8.1 row 2; (3) the timeout is raise-only, §7; the umbrella drifts are PR-1's own amendments, §4.7.
