# PR-1 tests and controls — edge matrix, pins and mutants (python-pin program, 2026-11-05)

This is PR-1's test and control specification, split out of its plan,
`docs/plans/2026-10-python-pin-pr1-pin-and-gate.md` ("the plan"), under CLAUDE.md's 1000-line touch-time rule. The
seam is design versus verification: the plan states what PR-1 builds and edits (§0–§4, §7–§11); this memo holds
what can go wrong with it and how each case is pinned — the edge matrix (§5, rows E1–E29, with the behaviour deltas)
and the tests and controls that hold those rows (§6: N1–N23, W1–W3, and the DM / FM / NM / WM mutants). The two
sections keep the plan's numbers, so "§5" / "§6" and their subsections are this memo's and every other `§n` is the
plan's; "body :n", "rounds :n", "vm-p4 :n", "PR-0 :n", R§n and the C-items read as the plan's §0 defines them, and a
bare `file:line` is at `660e594c` (they hold at the base `17b2a84d`, plan §0) unless a revision is named. The
plan's §2 matrices name these controls in their *held by* columns; this memo is in the plan's §7 edit set.

---

## §5 Edge matrix

| # | case | verdict | why |
|---|---|---|---|
| E1 | driver, `python3` = 3.15.2 final | admitted, suites run | Step 1 |
| E2 | `python3` 3.15.1 / 3.15.3 / 3.14.10 / 3.16.0 / 3.9.6 / 3.15.2rc1 (the pin's neighbours, derived by `refused_versions`, plan §4.8) | refused, exit 1, no member runs | exact admission; release level |
| E3 | no `python3` on PATH | `FAIL pin: no python3 on PATH` | Step 1's guard |
| E4 | root `.tool-versions` missing / CRLF / second line / comment / trailing comment / `3.15` / BOM / two spaces | `python_pin.py` exits 2, so the driver prints `FAIL pin` and exits 1 before any suite, locally and in CI; clause 1's parse branch is reached by W1 in-process | one parser, one form |
| E5 | a decoy `.tool-versions` in the cwd holding the running version | still refused | H3: root from the module's path |
| E6 | mise: not on PATH | item 6 "mise not on PATH"; block | R§4.2 *Each case's fix* row 1 |
| E7 | mise: untrusted (rc 1, `mise trust` in stderr) | item 6 + stderr | row 2 |
| E8 | mise: another version wins (`mise.local.toml`; `MISE_PYTHON_VERSION`; second `.tool-versions` line; T2 file) | item 5 (a) shows `version` and `source` | row 3; at the fake layer the four are two answer shapes — a file `source` `{type, path}` (the three file variants differ only in its values, printed verbatim) and an environment `source`; N10 has one fixture of each |
| E9 | mise: `[]` | "mise resolves no Python at <cwd>" | row 4 |
| E10 | mise: the pin, not installed | `installed: false`; (b) `exit status 1` | row 5 |
| E11 | another `python3` ahead of the shim | item 2 beside (b)'s install path; (d) `exit status 1` | row 6 |
| E12 | `[env] _.path` → `["altbin"]` (T1 (b)) | (c) / (d) print the file and its list | row 7 |
| E13 | cwd outside the root (PR-2's launches; the driver `cd`s) | item 3's two paths | row 8 |
| E14 | (a) prints garbage / exits 2 / prints JSON nested past the parser's recursion limit | item 6 ("does not parse", no traceback), stderr; (b)–(d) still asked except (d) when (c) failed | fail-safe |
| E15 | (a) hangs (with a grandchild holding the pipes) | item 6 after `ORACLE_TIMEOUT_S`; rest "not asked"; neither the fake nor its grandchild survives | bounded; the process group is killed |
| E16 | stderr longer than the cap | verbatim to 4000 characters, truncation note | cap |
| E17 | `MISE_PYTHON_COMPILE=1`, `MISE_PYTHON_FOO=x`, `MISE_GITHUB_ATTESTATIONS=false` set; admitted | warning lines, exit 0 | warn, never red |
| E18 | `MISE_PYTHON_VERSION` set | no warning (a version input; (a) shows it) | body :463–466 |
| E19 | every A-iii scratch driver | admitted on the pin (its tree holds the copied pin and reader); one more tracked `.py` per tree | scratch × I1 |
| E20 | `ci.yml`: `python-version` beside the file input / `.python-version` / `./.tool-versions` / second setup step (before or after the driver) / setup after the driver / `strategy` / `runs-on: ubuntu-latest` | red | N4, N6 |
| E21 | `mise.toml`: the two lines under `[tasks.x]` instead of `[settings]` | clause 4 green, **W2 red** | W2 parses |
| E22 | `mise.toml`: `[settings.python]` table form with the same values | W2 green, **clause 4 red** (`[settings.python]` spells python) | text and parse both bind |
| E23 | `mise.toml`: a value flipped | W2 red and clause 4 red | both |
| E24 | `mise.toml`: `[settings] experimental = true` added | **W2 red**, naming the key | no `[settings]` key beyond the two is admitted (plan §4.5) |
| E25 | `[tasks.tools-test]` gains `dir` / `env` / a future key | red naming it | N7 allowlist |
| E26 | a resolve `RuntimeError` at `sibling_path` (e), `plan_memo_population.py:54`, `plan_memo_memo.py:761` | propagates (crash = FAIL) under V6 (i) | consent item 2; O4; at (e), `resolve_runtime_error_control` and its mutant (plan §8.1 row 2) |
| E27 | a mise shim or a `mise` first on the outer `PATH`, or a `BASH_ENV` / `ENV` / `BASH_FUNC_*` in the caller's environment, any pin-file scratch run | none is reached; the run is admitted on the interpreter running the pins | `base_env()` (plan §4.3); N22 |
| E28 | a `mise` in a `CHILD_PATH` directory (a distribution package in `/usr/bin`) | the pins run; that mise, if reached, writes only under the isolation root (it may read system or ancestor config), offline, installing nothing | `base_env()`'s isolation (plan §4.3); N22's reachable-mise case |
| E29 | `python3` exits non-zero before `python_pin.py` runs (a mise shim refusing an untrusted config), or exits 0 without its `python-pin: admitted` line | `FAIL pin: python3 did not end with scripts/python_pin.py's exit line (exit N)`, its stderr shown, no member runs | the exit line (plan §4.2, plan §4.3); the driver pin's F6 case and N1d |

**Behaviour deltas** (C22): (1) the gate on a `python3` of another version: runs on `main`, **refused** after PR-1;
(2) `mise run tools-test` without the pin installed: runs PATH's `python3` → installs the pin (attested, prebuilt)
and runs it; (3) the `tools` job: two legs → one; (4) a `python-version` input or second setup step: green → red;
(5) `mise install` of a patch with no prebuilt: compiles unattested → fails; (6) a same-version `python3` from
another source: runs → still runs (decision 170's limit); (7) **any** `mise run <task>` (`trip-wires`, `lint`, `ci`, …)
in a tree whose pin is not installed: runs → installs the pin first (prebuilt, under the tracked settings) and runs
the task with it on `PATH`; when no such build exists the install fails (plan §10 step 11's record) — whether the task
then runs anyway is **unmeasured**. Measured 2026-11-04, mise 2026.10.25
macos-arm64, in a scratch project holding `python 3.15.2`, the two `[settings]` lines and `[tasks.x] run = "echo
task-ran; command -v python3"`: `$I MISE_TRUSTED_CONFIG_PATHS=$P $M run x` (`$I` / `$M` as plan §4.2 *Shapes the fakes
answer*, without `MISE_OFFLINE=1 MISE_AUTO_INSTALL=false` because the install is the subject; everything under `$d`)
installed `cpython-3.15.2+20261026-aarch64-apple-darwin-install_only_stripped` into `$d/data`
before the task, whose `command -v python3` printed that install; rc 0. plan §10 step 9 tells every lane. (8) The pins'
scratch drivers, `git` and utilities, locally: from the outer `PATH` (which may put a newer `bash` or GNU tools
ahead of `/bin`) → from `CHILD_PATH` (plan §4.3) — on macOS `/bin/bash` 3.2.57 and Apple's `/usr/bin` tools (macOS has
no `/usr/bin/bash`: `ls /usr/bin/bash` fails); on `ubuntu-24.04` the same `/usr/bin` as before (**unverified**: no source for the image is in the tree, as plan §4.3 says). Measured at `660e594c` on 2026-11-04, the
driver and every scratch driver it starts (they take `bash` from that `PATH`) on exactly those:
`env PATH="$(dirname "$(python3 -c 'import sys;print(sys.executable)')"):/usr/bin:/bin" /bin/bash
scripts/python-suites.sh` → `GREEN -- 11 member(s), 130 test(s) passed`. After PR-1 every local pin run is on
them, so plan §10 step 2's run measures PR-1's own driver there.

---

## §6 Tests and controls

"Red at `660e594c`" is re-read at PR-1's base (C5). Each control asserts the clause / item and the path or line it
names, not only "non-empty". Mutants follow PR-0's style: a table of (ID, target, wrong implementation, killing
control); a test applies each and asserts the killing control raises `AssertionError`, and that the control passes
unmutated.

### §6.1 `scripts/test_python_suites_driver.py` (pin file; Q5 → N1)

- **N1** (replaces Q5; I7 (ii)): a scratch tree (via `Scratch`, which now copies the pin and its reader, with one
  passing member) and one directory `B` holding the fake `python3` (`write_fake_python3`) and the fake `mise`
  (`write_fake_mise`, answering `[]`). **Its environment, exactly**: `E = {k: v for k, v in base_env().items() if not
  k.startswith("MISE_")}`, then `E.update(kv for kv in helper.isolated_mise_env(tmp_root, fake_mise=B).items() if
  kv[0].startswith("MISE_"))`, then `E["PATH"] = str(B) + os.pathsep + E["PATH"]` — `base_env()`'s without its own
  `MISE_*` (no outer `GIT_*`, as Q5's `Scratch.run` had), plus the helper's five variables and nothing else of it, so
  `PATH` = `B`, then `CHILD_PATH`, and the fake's recorded `MISE_*` are the helper's alone (N1 asserts them under
  `tmp_root`); each isolation keeps its one home for what it asserts (the `MISE_*` values the helper's, the `GIT_*` /
  `PATH` / start-up-file drop `base_env()`'s). The driver is
  launched as `_launch(["bash", <scratch>/scripts/python-suites.sh], env=E, cwd=<scratch>, purpose="driver")`, not
  through `Scratch.run`. N1 pins that composition itself (N19 (a) runs only in the oracle fixtures): before the
  launch, `shutil.which("python3", path=E["PATH"])` and `shutil.which("mise", path=E["PATH"])` are `B`'s fakes; after
  each refusal, the fake `mise`'s recorded environment holds exactly the five `MISE_*` keys, each under `tmp_root`,
  no `GIT_*` key, and `PATH == E["PATH"]`. For each E2 version: exit ≠ 0, the FAIL line names `pin`, the output names the pin, the
  fake version and `<R>/.tool-versions` with `R = os.path.realpath(<scratch>)` (the driver `cd -P`s, so macOS prints
  `/private/var/…` where `mkdtemp` gave `/var/…`; every path N1 compares is real on both sides), and `member(s) in`
  is absent; the first-branch text `FAIL pin: scripts/python_pin.py refused or failed (exit 1; its output is above)`
  is present and the second branch's absent; and the fake `python3`'s argv log (`read_python3_argv`) holds one more
  call naming `python_pin.py` per version, its first two arguments `-I`, `-S` (Step 1's invocation, DM8). The pin and E2's versions are derived from the `.tool-versions` text (the live file for the pin N1
  names; the scratch tree's own copy for the versions, through the helper's `refused_versions`, plan §4.8): the file
  spells no pin, so a bump edits `.tool-versions` alone. **Spelling control**: the scratch `.tool-versions`
  rewritten to the first refused version, the fake reporting it → Step 1 admits and the run is GREEN. Kills
  DM1–DM8 below.
- **N1b**: `PATH` = a scratch directory holding only symlinks to the utilities the driver calls before Step 1
  (`dirname`, `basename`, `readlink`; no `python3`, which `/usr/bin` holds on both platforms), launched as
  `/bin/bash` → `FAIL pin: no python3 on PATH`. **N1c**: a malformed scratch pin → exit ≠ 0, the output names
  `.tool-versions`, no member ran. **The pre-launch case** (F6; E29): a `python3` first on `base_env()`'s `PATH` that
  prints `mise ERROR untrusted config` and exits 1 → exit ≠ 0, that stderr shown, `FAIL pin: python3 did not end
  with scripts/python_pin.py's exit line (exit 1)`, not the first branch's "refused or failed", no member ran.
  **N1d** (E29's exit-0 half): a `python3` first on `base_env()`'s `PATH` that exits 0 silently → exit ≠ 0, `FAIL
  pin: python3 did not end with scripts/python_pin.py's exit line (exit 0)`, neither "refused or failed" nor
  `member(s) in`. **N1e** (exit 3 end to end): the scratch `python_pin.py` made to raise in `read_pin`, run by the
  fake `python3` through `LAUNCHER`'s `_guarded_main()` → `RuntimeError: boom`, `python-pin: exit 3`, the first
  branch's `(exit 3; its output is above)`, not "did not end with", no member ran. **`PIN_FILE`'s one spelling**
  (`test_PIN_FILE_has_one_spelling`): `python_pin.py`'s one literal `PIN_FILE = …`, read by `ast` (not imported,
  P6), equals `python_suites_readers.PIN_FILE`.
- **N22** (I7 (iii)): under `mock.patch.dict(os.environ, …)` — a directory of `write_sentinel`'s `mise` and
  `python3` first on `PATH`, an outer `MISE_PIN_SENTINEL`, and `BASH_ENV`, `ENV` and an exported function
  `BASH_FUNC_dirname%%` (a function the driver calls; its body writes the marker, then runs the real `dirname`) that would each write a marker file — four parts (`N22_PARTS`): `scratch_run` (a passing
  `Scratch` run is GREEN), `direct_launch` (a driver launch built as the regime file's `:549–558` is,
  `_launch(["bash", …], env=fixtures.base_env(), …)`, is GREEN), `bash_env` (a passing `Scratch` run is GREEN and
  the marker file is absent), `isolation` (`fixtures.base_env()`'s `MISE_*` keys are exactly the helper's
  `MISE_VARS` plus `MISE_OFFLINE` / `MISE_AUTO_INSTALL`; `MISE_DIR_VARS` + `MISE_GLOBAL_VAR` equal the helper's
  `MISE_VARS`; each of those paths resolves under `mise_isolation_root()`; the global file is empty; the two values
  are `1` / `false`; no `BASH_ENV` / `ENV` / `GIT_*` / `BASH_FUNC_*` key). After the parts, `read_sentinel` is `[]`,
  and `shutil.which("python3", path=CHILD_PATH)` resolves (`os.path.realpath`) to `os.path.realpath(sys.executable)`.
  N22 reaches `base_env` only as an attribute of the module object (`import python_suites_fixtures as fixtures`),
  never through the file's by-name import (`:26`), so FM1's patch reaches every use; the `CHILD_PATH` assertion
  pins the constant, which FM1 leaves. **N22's reachable-mise case** (E28): with `CHILD_PATH` patched to put a
  sentinel `mise` directory first, between, or last, `base_env()` does not raise, its `PATH` is that value, and its
  isolation variables are under the root, `1` / `false`.
- **Driver and fixture mutants** (`DRIVER_MUTANTS`, NEW, in this file): DM rows are source mutants over the scratch
  copies of the driver and of `python_pin.py` — (ID, file, find, replace, killing control), the find text occurring
  exactly once in the live file it mutates (a stale row is red), written into the scratch tree after
  `Scratch.__init__`: DM1 Step 1's `python_pin.py` call replaced by the old floor check (`tuple(sys.version_info[:2])
  >= (3, 9)`) → N1's refusal cases; DM2 the call replaced by an exact literal (`tuple(sys.version_info[:3]) ==
  <the pin triple>`, the triple derived as N1's) → N1's refusal cases (the fake reports its version only to
  `python_pin.py`, so a check that bypasses the module sees the interpreter running the pins and admits it); DM3
  the call replaced by `true` → N1's refusal cases; DM4 Step 1's failure branch without its `exit 1` (the check no
  longer precedes the first suite) → N1's refusal cases (the passing member then runs: exit 0); DM6 DM4's mutation
  over a scratch tree whose extra member fails (`MUTANT_EXTRA_FILES`) → the run is non-zero and names `pin`, so only
  N1's `member(s) in` assertion kills it; DM7 the exit-line test replaced by `false` (the second branch's text
  always) → N1's first-branch assertion; DM8 Step 1 without `-I -S` → N1's argv assertion; DM9 the driver's
  admitted-line arm accepting any last line on exit 0 → N1d's control (`silent`); DM5 `python_pin.py`'s `main` comparing against a literal triple (the
  derived pin) instead of `read_pin`'s value → N1's spelling control (I1 × I2: editing the pin must move
  acceptance). FM1 (`mock.patch.object` on the fixtures module) `base_env` restored to `660e594c`'s body (outer
  `PATH`, `MISE_*`, `BASH_ENV` kept) → each of N22's four parts kills it alone. The test asserts each killing
  control passes unmutated and raises `AssertionError` mutated.

### §6.2 `scripts/test_python_suites_ci.py` (pin file)

- **N4** (replaces Q7): live `ci.yml` green under `setup_problems`; red, each with its own message: E20's seven
  variants. Kills: a reader that checks only the first setup step, only steps before the driver, or `in` instead of
  key-set equality.
- **N5** (replaces R2): `python-version-file` reads `.tool-versions`; `branches: [main]` unchanged.
- **N6**: a planted `strategy:` is a job-key red (`strategy` left the allowlist).
- **N7** (Q4 / M2 restated over parsed data): live `mise.toml` green; red: `depends` without `"tools-test"`, `depends
  = { a = 1 }`, `run` other than the driver, `[[tasks.tools-test]]`, each of `sources` / `outputs` / `env` / `dir` /
  a future key, a TOML syntax error (its message named). Kills: a denylist of two keys; a text grep.
- **N8** (parser controls, the surviving M1 / M3 cases): a fake `[tasks.tools-test]` inside a multi-line string
  creates no section; `[tasks.tools-test.env]` and `tasks."tools-test".env.X` are the unknown key `env`; an escaped
  key spelling `tasks` reaches the same check. Kills: a reader that is not the parser.
- Fixtures `TOOLS_HEAD` / `TOOLS_STEPS` lose `strategy` and spell `python-version-file: .tool-versions`; R6's
  multi-line-legs half goes (its subject is gone; its first half keeps the grammar property); R9's other job spells
  the file input. Q4, R1–R9, Q6, Q8, D1 stay.

### §6.3 `scripts/test_python_pin.py` (NEW set member; not a pin file; imports `python_pin` for pure functions)

In-process (pure): **N2** parser — green `python 3.15.2` with and without the final newline; red, each naming what it
found: plan §4.2's list; the live `.tool-versions` parses to the triple the live gate admitted
(`admitted(tuple(sys.version_info), pin)`: the member runs under the pin, after Step 1). **N3** admission — the
pin triple final → true; E2's versions → false; `refusal_text` called in-process with a **plain tuple** version
prints its `running:` line (the index-only rule pinned on the refusal path, which `LAUNCHER`'s tuple subclass cannot
pin); and a fake `mise` on the child `PATH` answering "3.15.2 installed"
is never invoked on an admitted run (`calls.jsonl` empty; a child through the helper). **N18** imports — `ast` over `python_pin.py`: top-level
imports exactly `os`, `sys`; `subprocess` / `json` / `signal` only inside `ask_mise`, `traceback` only inside `_guarded_main`; no `TestCase`, no `"--self-test"`.

Through the helper (children, `LAUNCHER`, isolated): **N9** root — run from a cwd holding a decoy `.tool-versions` with
the running version: refused per the tree's pin. **N10** each case's fix — one fixture per R§4.2 *Each case's fix*
row (E6–E13), asserting its fact lines and that the block equals `GENERIC_BLOCK` byte for byte, plus the
two-element answer body :422 and C16 (e) name — R§4.2's draft-6 row (records :299: `mise.local.toml` `[tools]
python = ["3.14.10", "3.15.2"]`, two elements in that order, both `source` `…/mise.local.toml`, `type: "mise.toml"`):
both elements printed, in mise's order, nothing read into the order. **N11** argv — the
fake's log holds exactly (a), (b), (c) and one (d) per listed path, in order. **N12** compound — another version
wins **and** `mise which` ≠ `sys.executable` **and** a warned variable: all three printed. **N13** warning — E17 /
E18, on refusal and on admission, exit status unchanged. **N14** `sys.executable` — one fixture without the
override: item 2 equals the child's real value (C37 (b)). **N15** limits — in the order E16, E15, E14: E16; E15 in process (`os.environ` patched to
its helper environment): `ask_mise(<cwd>, 1.0)` with `Popen.communicate` wrapped so that it refuses a call without a
timeout and starts only once `hang.pids` holds two pids (the 1 s is never spent on the fake's start-up), its records
printed by `refusal_text`; E14, including JSON nested past the
parser's recursion limit (`"does not parse"`, no `Traceback`); and E15's **no survivor**: after the refusal returns,
`hang.pids` holds exactly two pids (the fake, the group leader, and its `sleep`; absent, one line or a third is red),
and both are gone within 2 s, polled: gone = `os.kill(pid, 0)` raises `ProcessLookupError`, or on Linux
`/proc/<pid>/stat`'s state field is `Z` (a zombie no reaper has collected yet, e.g. under a container's pid 1;
killed is what is asserted). macOS has no `/proc`, so there the poll relies on `launchd` reaping orphans — the
assumption plan §4.2's measurement (`group 1.0 s gone`) observed. `N15_sigint`, in process the same way:
`ask_mise(<cwd>, 60)` waits on the hung fake while a thread waits (bounded, 30 s) until `hang.pids` holds two pids and
then sends `os.kill(os.getpid(), signal.SIGINT)` (also when the wait fails, so the main thread cannot hang) →
`KeyboardInterrupt` out of it, then the same two-pid poll. **N19** I7 pins — (a) inside **every** `run_pin` fixture
(N2, N3, N9–N15, N19, N20, N23): the child's environment as the helper built it (less the `MISE_*` a test sets through
`extra_env`) passes `isolation_problems` under that run's root (`<tmp>/iso`, `<tmp>` the run's own `mkdtemp`): the
five variables, the global file empty, no other `MISE_*`; and `shutil.which("mise", path=child_PATH)` is the
fake (or `None` for E6); N19's own control also runs `isolation_problems` over every call the fake recorded; (b) after one refusal, no entry of the real `tracked-configs` / `trusted-configs` (outer
`MISE_STATE_DIR`, else `$XDG_STATE_HOME/mise`, else `~/.local/state/mise`; read only; absent = empty, never a skip)
resolves under that run's `<tmp> + os.sep` (body :226–234's pin, kept; `Result.tmp` carries it — the earlier
`dirname(dirname(bindir))` was `$TMPDIR` itself, which admitted any directory there in (a) and reddened (b) on any
unrelated real entry under `$TMPDIR`); its red control plants, in a scratch state dir read through
`state_entries(base=…)`, a `tracked-configs` link and a `trusted-configs` line resolving into `<tmp>`, and both are
reported, so (b) cannot be green by reading nothing; (c) fixture-owned evidence, the
one NM-I7 is killed by: for the whole test class a `write_sentinel` directory (`mise`, `python3`) is first on
`os.environ["PATH"]`, and after every child of N10–N15 `read_sentinel` is `[]` — the helper's `PATH` puts the fake
first or drops every directory holding a `mise`, so a child reaches the sentinel only when it bypassed the helper,
and a bypassed child reaches the sentinel instead of the user's mise (no real state is written by the mutant run).
**N20** regime — what is compared is the **observed** child environment, not the helper's dict: the fake's
recorded `environ` (the oracle's child inherits the `LAUNCHER` child's environment unchanged) equals the parent's
`os.environ` on every key except `MISE_*`, `PATH` and `PWD`, `OLDPWD`, `SHLVL`, `_` (the keys a shell maintains;
no shell stands between since the fakes exec by shebang, plan §4.8, so the four are a margin, not a need)
— a superset of the regime keys (C34), so no second list of them is written here. **N21** helper shape — `ast` over
the helper: no `TestCase`, no `__main__` guard, no call in G9's spawner set; the member loads it through the one
constant. **G9's set has one home**, the local `spawners` of `test_G9_every_launch_goes_through_the_accounting`
(`test_python_suites_regime.py:694–695`, a file PR-1 does not touch): N21 reads it from that function's source by
`ast` (the set literal and the `startswith` tuple of the comprehension), and copies nothing. **Fail-closed**: the
read is exactly one `Assign` of a set display to `spawners` (one target), exactly one `spawners |= …`, exactly two
stores of the name, no attribute access on it (`spawners.add(…)` would grow the set unseen), and exactly one
`startswith(<tuple>)`; any other shape is red (`G9's spawner set has another shape than this control reads`),
never a smaller set read silently.

**N23** the exit line (plan §4.2) — a refusal ends `python-pin: exit 1` (exactly one exit line); a malformed pin and a
missing pin file end `python-pin: exit 2`; an uncaught exception (a scratch copy whose `read_pin` raises, run as a
script) ends its traceback with `python-pin: exit 3`, exit 3, one exit line; an admitted run's stderr is exactly
`python-pin: admitted 3.15.2`, no exit line, exit 0.

The `test_<id>` of each `control_<id>` (N2, N3, N9–N20, N15_sigint, N23) is generated from one list (`_control_test`); each records its
unmutated pass in `PinCase.passed`, and `test_mutants_are_killed` (which sorts after them) re-runs unmutated only
the killing controls not recorded in this process — the pre-run is memoised, not dropped.

Grammar and record (J2, C24): **N16** — `ast.parse(<python_pin.py>, feature_version=(3, 9))` succeeds; its six
mutants (NM11a–f) each insert one of R§11's six rejected rows (the `match` row indented) and must fail it. **Stated
limit**: `feature_version` is best-effort; R§11's ten accepted-on-3.15.2 rows are a sample, not a census. **N17** —
`REAL_OLD_RUN` (NEW constant: SHA-256 of `python_pin.py`, the date, `3.9.6 <D>/usr/bin/python3`, and `mise`: the mise
the run reached, or `"none reachable"`) holds a non-empty `mise` and equals the live module's digest; red on any edit
without a new record. **Stated limit**: it forces a record, not the run; the run is plan §10's checklist item. The
record in the file (read it there, not here) says `"none reachable"`, so it covers the refusal's interpreter half
only; the real-mise half (plan §10 step 4's "the real mise's answers parse") is a landing item.

### §6.4 `scripts/test_tracked_tool_config.py` (PR-0's member; plan §4.5 *What the member's fixtures change* says why)

- **W1** clause 1: green — the live line, with and without the final newline, and the root file **is read**; red,
  clause 1 at `.tool-versions` — not tracked, a symlink, not UTF-8, and N2's red texts through `problems`.
- **W2** the install settings over parsed TOML: the live root `mise.toml` green; red — absent table, E21, E23, an
  extra `settings.python` key, E24's extra `settings` key (named). Expected value derived from `ADMITTED_LINES`
  (plan §4.5).
- **W3** PR-1's writes × clause 4: a fixture `mise.toml` holding the post-PR-1 `[settings]` block, description and
  comment is green under `problems` (L1 holds the live file).
- **The composition** (plan §4.5 `problems`): a member path holding a newline, or `: `, is one named clause-4 finding
  (no re-parse); an iterator `tracked` over a valid tree is `[]` (both finders see it); clause 1's finding precedes
  the members'.

**The member's edits** — the one exact list; every hunk of `git diff 660e594c <head> --
scripts/test_tracked_tool_config.py` falls under one item, and every other line, `:427` (L1, the live tree) included, is
unchanged (line numbers at `660e594c`):
1. the module docstring: "T5 clauses 2-5" (`:1`) → clauses 1–5; its plan-of-record line (`:3`) also names plan §4.5
   and this section;
2. imports gain `atexit`, `importlib.util`, `shutil`, `tomllib`;
3. after `MEMBER_PATH` (`:30`): `HELPER` (plan §4.8's one constant) and `H`, the helper loaded by path; `PIN_BYTES`, the
   live `<REPO>/.tool-versions` bytes read at import (a malformed live pin reds every fixture world, the right answer);
4. `_without_git_env()` (`:45–46`): the helper's `isolated_mise_env` under one process-lifetime root (`_ISO_ROOT`,
   removed at exit by `atexit`) minus `GIT_*` — the member's `git` children are in I7 (i) (plan §2.1);
5. `pinned(tracked, read)` (NEW): `tracked` plus `ROOT_PIN`, and a `read` answering `ROOT_PIN` with `PIN_BYTES` and
   delegating every other path (so C3 r4's `read`, which raises if called, is still never called); `probs`
   (`:97–100`) is built on it unless its `files` name `ROOT_PIN` themselves (W1's clause-1 cases); `mise_probs`
   (`:103–104`) inherits it;
6. C0's lists (`:107–109`): `MODULE_IMPORTS`' comment ("none can launch a process" → none launches when imported,
   and from `python_pin` only the parser names are taken) and the tuple gaining `python_pin`; NEW beside
   `CONTROLS_2_TO_5`: `PIN_IMPORTS` (`PinFileError`, `parse_pin`), `CONTROLS_1` (W1, W2, W3,
   `clause1_mutants_are_killed`), `CLAUSE_1_NAMES` (`root_pin_problems`, `_REMEDY_PIN`);
7. C0's checks: the `Import` check (`:143–146`) rejects `import python_pin` (it would bind the module, `ask_mise`
   included); the `ImportFrom` check (`:147–152`) admits from `python_pin` only `PIN_IMPORTS`; the seam comment
   (`:162`, "minus `problems`" → "minus `problems` and clause 1's names") and assertion (`:171–173`) exclude
   `CLAUSE_1_NAMES` beside `problems`; the existence loop (`:175`) runs over `CONTROLS_2_TO_5 + CONTROLS_1`;
8. C3: r4 (`:233`) → `t.problems(*pinned(…))`; g1 (`:234–235`) → W1's green case through `pin_green_and_read` (NEW:
   the root `.tool-versions` holding the live line is green **and read**);
9. the other direct calls bypassing `probs` → `t.problems(*pinned(…))`: C4's `:297`, `:307`, `:317`, `:319`, `:325`,
   C5's `:375` (both calls) and `:384`; C5's expect list is unchanged (a valid pin adds no finding);
10. L2 (`:442`): the scratch repository writes a real `.tool-versions` holding `PIN_BYTES` and adds it in the same
    `git add -f` line; M1 (`_walk_lister`) still dies there;
11. after L2 (`:447`), NEW: W1, W2, W3 (each a `control_` / `test_` pair), the composition test
    (`test_problems_names_a_newline_bearing_member_path`) and `test_clause1_mutants_are_killed`;
12. `test_mutants_are_killed` (`:450–465`): its pre-run and mutant loop move into `_assert_all_killed(table)` (NEW:
    each killing control run unpatched, then the table's mutants — the one home of the pre-run for `MUTANTS` and
    `WMUTANTS`), which it calls; its exact ID assertion is unchanged;
13. after the class (`:466`), NEW: `settings_problems` (W2's check) and `W3_MISE_TOML`; after `MUTANTS` (`:553`):
    `WMUTANTS` (§6.5). PR-0's `MUTANTS` table is unchanged.

### §6.5 Mutants

`NM` (in `test_python_pin.py`) are **source** mutants — (ID, find, replace, killing control) over a copy of
`python_pin.py` written by `pin_tree`; the find text must occur exactly once in the live module (a stale row is red),
so they reach the children too: NM1 minor-only admission → N3; NM2 release level ignored → N3; NM3 `split()`-based
parser → N2 (tab, two spaces); NM4 first-line reader → N2 (second line); NM5 pin read from the cwd → N9; NM6 an added
`env --json` question → N11; NM7 no timeout → N15 (E15's wrapped `communicate` refuses a call without one, an `AssertionError`); NM8 the cap removed →
N15; NM14 the group kill dropped (`os.killpg(proc.pid, 9)` → `proc.kill()`) → N15's no-survivor check; NM9 the warning
skips `MISE_GITHUB_ATTESTATIONS` / warns `MISE_PYTHON_VERSION` → N13; NM10 top-level `import json` → N18; NM11a–f →
N16; NM12 a case-dependent line inside the generic block → N10; NM13 admission asks mise first → N3; NM15 / NM16
the exit line dropped from the refusal / the pin-file error (plan §4.2) → N23; NM17 the `__main__` guard calling
`main` instead of `_guarded_main` (no exit line after a traceback) → N23; NM-I7 the helper
dropped → N19 (c) (and (a)). NM-I7 is not a source row: it replaces the loaded helper module's `isolated_mise_env`
with `mock.patch.object` by a function returning `dict(os.environ, **(extra_env or {}))` (the outer environment passed
through) and counting its calls; its stale-row rule is that the attribute exists before patching and the count is
non-zero after the control (a patch nothing called proves nothing). This replaces body :230's "(b) red locally": that
kill read the user's real `tracked-configs` and, with mise on `PATH`, let the mutant run write real state, and it
survived silently where mise is absent (body :230, amended in place, plan §4.7). `WM` (in `test_tracked_tool_config.py`,
`mock.patch.object` like PR-0's) — WM1 an untracked root pin passes → W1; WM2 clause 1 by a first-line reader (the
module's `parse_pin` name patched) → W1; WM3 clause 1 following symlinks → W1; WM4 W2 by text search → W2 (dies at E22's parse-equal green case); WM5
`ADMITTED_LINES` gains a third line → W2 live red (the expected value comes from it); WM6 W2 admitting **any**
table of the document equal to the admitted one (the two lines under a task pass) → W2 (E21) — the one mutant that
reaches W2's red cases, which WM4 and WM5 die before.
