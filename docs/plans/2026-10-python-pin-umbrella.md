# Umbrella — pin the Python interpreter (draft 8, 2026-11-01)

## §0 Status

**Lane**: CI/tools. **Base**: `origin/main` at **`ee6c7a1a`** (2026-11-01; #577). Between draft 6's base `6aea28da`
and `ee6c7a1a` four PRs landed (`git log --oneline 6aea28da..ee6c7a1a`): **#506 `3ec0ae5c`** (the VM P4 umbrella
and its split; merged 2026-11-01 02:35 JST = 2026-10-31 UTC, `git show -s --format=%ci 3ec0ae5c`, from PR head
`69e2a3aa`: `gh pr view 506 -R send/elidex --json headRefOid,mergeCommit,mergedAt`; PORTFOLIO:293), #575 `f6925e39`
(lints), #576 and #577 (`crates/**` and one plan memo). **After the base** `main` moved to `593aa314` (#579; `git
diff --stat ee6c7a1a 593aa314` lists six files, all under `crates/core/elidex-sandbox/`): no line this memo cites
moved, and #579 is the sandbox-lints slot §6 *Ordering* placed "anywhere" — it has landed (draft 8 keeps the base).
Outside `crates/**` (`git diff --stat 6aea28da ee6c7a1a -- ':!crates'`, 18 files) what moves this memo:
- **#506 landed before PR-1.** §5.3 is restated as the landed state (K1). The statements of #506's memos that PR-1
  makes false are V16–V18 (§5.2), and PR-2's edit set gains P2 (§5.3 (b)) — PM decision 205. V18's class in
  #506's slot ledger is V19–V21, and V22 is a call the pin warns on — PM decision 211.
- **At `ee6c7a1a` the suite set has ten members** (PR-1 adds its own; counts here are the base's) — new:
  `docs/plans/test_vm_p4_family_layout.py` (#506's layout guard; it imports `plan_memo_population` /
  `plan_memo_tables`; its import closure is twelve family modules, §5.3 O4) — and 89 tracked `.py` files (`git ls-tree -r
  --name-only ee6c7a1a | /usr/bin/grep -cE '\.py$'`; PR-1 adds `python_pin.py` and `python_pin_testing.py`). All 89
  compile under `-W error` on 3.15.2 and on 3.9.6; the guard alone passes on both, and the whole set is green on
  3.15.2 with V22's line the only runtime `DeprecationWarning` (measured 2026-11-01, R§0).
- **CLAUDE.md's `## CI` `tools` paragraph is `:100`** (`git grep -nE '両 leg|matrix' ee6c7a1a -- CLAUDE.md` prints
  `:30`, `:54`, `:100`). `:54` was rewritten by #506 and still holds "matrix の両 leg" once (V14).
- **Unchanged since draft 4's base**: `.claude/tools/**`, `scripts/**`, `.github/**`, `mise.toml` (`git diff --stat
  74e15df1 ee6c7a1a -- .claude/tools scripts .github mise.toml` prints nothing). So every family line V1–V11 cites,
  and every driver, `ci.yml` and `mise.toml` line this memo cites, holds at `ee6c7a1a`. #510's memo changed on
  `:663` only (1+/1−), so V12, V13 and V15 hold as well; `CLAUDE.md` did change (#571, #506), so V14 is re-read
  at `ee6c7a1a`. Earlier bases and what moved between them: R§0.
R§4.1's rows were measured at `b051329b`. PR-1 re-runs row 3 at its own base.

**Layout (K3, decided in §6 *This umbrella's own length*).** This memo is four landing files in one draft: the
body (§0–§11, this part), then *Landing file 2* (§5.2–§5.3, the VM P4 coupling), *Landing file 3* (the records
R§…, cited from the body as R§n) and *Landing file 4* (§12–§14, the rounds record). In table cells `\|` is the
Markdown escape of `|`: copy a cell's command from the rendered text, or replace `\|` by `|` (R6 M14).

**Decisions this draft is written on** (record locations: `PORTFOLIO` = `/Users/kazuaki/repos/send.sh/elidex-pm-trial/PORTFOLIO-2026-10-26.md`;
lane SSoT = `project_ci-tools-lane.md`, its top block "HANDOFF NOW (2026-10-31)"):
- **PM decision 41** (2026-10-26, `PORTFOLIO:59`): the 3.15 line, pinned to an exact patch (§4.1); A-iii's D3, §8,
  §12(1) amended (§5); an umbrella and per-PR plans with a property-derived entry-point population (§6); the
  global hooks measured end to end (§7.2).
- **PM decision 62** (2026-10-28, `PORTFOLIO:149`): Linux `mise install` accepted unmeasured; S3 stays rejected
  (§4.2, §7.2 (3), §11).
- **PM decision 117** (2026-10-31, `PORTFOLIO:217`): the pin is 3.15.2 (§4.1); #510's floor code and two-leg
  statements go into the consent table (§5.2); its order clause (3) is withdrawn by decision 127.
- **PM decision 124** (2026-10-31, `PORTFOLIO:228`): PR-3 folded into PR-1; #514 after PR-1, re-deriving once
  against the pinned world, starting right after PR-1 lands (§5.1, §6, §8).
- **PM decision 127** (2026-10-31, `PORTFOLIO:232`): **G1** PR-1 does not wait for #506, on (a) the VM P4 lane's
  consent to PR-1 touching its files (§5.2, V1–V23) and (b) a table of the lines both touch — **(b) is moot**:
  #506 landed first (§0), so §5.3 states the landed lines with one owner each; **G2** the PR-1 → PR-2 interval
  with a bounded regression (§6); **G3–G6** R3's lens dispositions.
- **PM decision 141** (2026-10-31, `PORTFOLIO:244`): PR-0 and the order PR-0 → PR-1 → #514's re-derive → PR-2 (§6).
- **PM decisions 142 / 149** (2026-10-31, `PORTFOLIO:245-246`, `:250`): PATH order differs per session; the fix is
  restarting old sessions — the ground of §7.1's supported path and §11's per-session bullet (H6).
- **PM decision 150** (2026-10-31, `PORTFOLIO:251`): (1) ask mise, don't model it — admission protects I2, T5 keeps
  two jobs, PR-0 rebuilt (§4.2, §6); (2) R5's terminator; (3) the order of the cargo-deny PR, the M4-12 program
  and sweep 4 by their `ci.yml` edits (§6); (4) override variables warn, no CI red (§4.2, §11); (5) H2, H3, H4, H6
  approved. Its oracle commands were replaced by `mise ls --current --json python` / `mise which python3`,
  ratified by decision 167 (J3).
- **PM decision 154** (2026-10-31, `PORTFOLIO:257`): #506's split approved for push; #506 then merged as `3ec0ae5c`
  from head `69e2a3aa` (PORTFOLIO:290, :293) — draft 6's `c032379d` was not its last head.
- **PM decision 167** (2026-10-31, `PORTFOLIO:267`): T3 is a defect, fixed as I7 (§2); T1 / T2 to the user
  (decision 170); J1–J4 accepted.
  The full text of each decision above, as this program records it: R§0 *Decisions*.
- **User decision 170** (2026-10-31, `PORTFOLIO:268`, via the PM). The option text the user saw, as recorded in
  the lane SSoT's "HANDOFF NOW (2026-10-31)" block, bullet "TOMORROW FIRST (PM decisions 166/167, …)" (R6 M13): 「『黙らない』を、
  Python の版が変わった場合に限って保証します。同じ版で出どころが違う場合（_.path や .miserc）は、限界として §11
  に書きます。CI の規則は大きくしません。版の食い違いは、今でも確実に止められます。」 So **the guarantee is: never
  silent when the Python *version* differs from the pin.** An interpreter of the pin's version from another
  source — a tracked or untracked `[env] _.path` entry that `mise run` / `mise exec` put first (T1), an rc file
  that redirects mise's version file (T2), any other `python3` of the same version ahead of mise's shim — is
  admitted, and is a **stated limit** (§11), not a guarantee. §4.2's CI rule keeps clauses 1–5 unchanged.
- **PM decision 178** (2026-10-31, `PORTFOLIO:278`): scheduling only — draft 6 today if room, R6 the next day.
- **PM decision 205** (2026-11-01, `PORTFOLIO:341`; relayed by the CI/tools driver): R6's three IMPs. **Q1 (a)**:
  O5 / O6 — the floor and slower-leg sentences of #506's `docs/plans/2026-10-vm-p4-memo-seam-split.md` — are
  **V16 / V17**, fixed by PR-1 on V1–V15's route (one issue, one way). **Q2 (a)**: P2's two lines go into **PR-2's
  edit set**, so PR-2 is green at its own base. **Q3**: the stale "0 hits" note of #510's memo (`:1097-1104`) is
  **V18** in PR-1; the item moves from the PM (PORTFOLIO:293, :308) to CI/tools. **One consent request** to the VM
  P4 lane covers V16–V18 and P2, sent once draft 7 is settled, by V1–V15's route (§9 step 3). **K3**: the seam set
  is decided now, so the landed body is ≤ 1000 lines (§6). Every decision above is unchanged.
- **PM decision 211** (2026-11-01, `PORTFOLIO:347`): **D7-1 (a)**: the three stale "0 hits" lines of #506's
  `docs/plans/2026-07-vm-p4-slot-ledger.md` are **V19–V21**, V18's class, fixed by PR-1 once both slots' homing is
  re-checked (done: §5.2). **D7-2**: `plan_memo_sibling.py:158`'s positional `maxsplit` is **V22**, fixed by PR-1
  unless the VM P4 lane fixes it itself. Both join decision 205's one consent request (§9 step 3), sent after R7
  converges. **D7-3**: the `ci.yml:167` item goes after PR-2 (§6).
- **PM decision 213** (2026-11-01, `PORTFOLIO:349`): the slot ledger's `:327` is **V23**, in the same consent
  request (§5.2); reviewed by the next round, no clerical skip.
- **PM decision 217** (2026-11-01, `PORTFOLIO:354`): **the refusal lists observed facts only and infers no cause**
  (§2 I5, §4.2 *The refusal*); the cause layer is removed, inside decision 170 — **a version mismatch always
  stops**, unchanged. Reason and R8's terminator: R§0, §12.7.

**Edge-dense: yes** — seven coupled invariants (§2; I7 is draft 6's, decision 167), so the umbrella and every
per-PR plan pass `/elidex-plan-review` before implementation (CLAUDE.md *Edge-dense work* (b)). **PR-1 and PR-2
are each edge-dense** (every pair × half is listed in §2 *Ownership*; §6): each plan carries its own intra-PR §2
pair matrix and its own plan-review (§6, §13 C9, C18). Whether PR-0 is a single intersection is **PR-0's plan's
call** (R5 MIN: besides I1's tracked half × I6's admitted lines it meets `main`'s two legs, the offline regime and
A-iii's `mise.toml` reader; C11). **Landing paths** (each with its PR): with PR-1, the body
`docs/plans/2026-10-python-pin-umbrella.md` and its three companions `…-python-pin-umbrella-vm-p4.md`,
`…-umbrella-records.md` and `…-umbrella-rounds.md` (§6 *This umbrella's own length*); `…-pr0-tracked-mise-config.md`
with PR-0, `…-pr1-pin-and-gate.md` with PR-1, `…-pr2-entry-point-guard.md` with PR-2. Draft 2's PR-3 is folded into
PR-1 (§6, F5); PR-0 is draft 4's split, rebuilt by
draft 5 around T5's new shape (§6, decision 150 (1)).
**No ECS surface**: no `crates/**` file, no component, system or query is touched; the subject is tooling.

**This memo carries no measured digits as live claims.** §4.1 is a **decision record** (dated, with the command
that reproduces each row); counts and timings go to the PR bodies and to the PM report, not here (the A-iii
program rule of 2026-08-23). **Every mise claim names the mise version it was measured with** (2026.10.21 for
the 2026-10-31 rows; R6's A4 re-ran every oracle row on 2026.10.25 with identical results, §12.6; draft 7's
probes used 2026.10.25). **The rule for every mise probe in this program** (draft 5; `feedback_mise-probes-isolate-state-dir`):
`MISE_DATA_DIR`, `MISE_CACHE_DIR`, `MISE_STATE_DIR` and `MISE_CONFIG_DIR` under a scratch directory,
`MISE_GLOBAL_CONFIG_FILE` an empty file, trust only through `MISE_TRUSTED_CONFIG_PATHS` (never `mise trust`),
nothing installed into the user's data dir, and from draft 6 `MISE_OFFLINE=1`, `MISE_AUTO_INSTALL=false` and fake
install directories. **I7** (§2) makes it an invariant of every test that can reach the refusal. Which probes
followed it, and the one that did not (draft 4): R§0.

---

## §1 Ideal anchor

Rust is pinned once (`rust-toolchain.toml`) and every caller — local `cargo`, every CI job — runs that toolchain;
a bump is a deliberate PR. ADR #36 (`docs/design/en/28-adr.md`, row 36: during development, latest stable only)
is applied as *the pin tracks each new stable* — #567 bumped the pin to 1.99.1 on that ground. The Python tooling
uses the standard library only, so the same rule picks the newest bugfix line and moves the pin with it.

Python today has the opposite shape: `mise run tools-test` runs whatever `python3` is first on PATH; CI runs two
legs (`"3.9"`, `"3.x"`); the floor 3.9 is an end-of-life line; direct entry points (`webref`, `preflight.py`, the
`plan-*` tools, `plan-memo-umbrella-check.py`) run on anything; and #510 put code branches on `main` that exist
only for interpreters below 3.13 (§5.2).

**Target: one interpreter, spelled once, admitted by every launcher, measured by the gate; any other interpreter
is refused, by name, before it runs anything.** *Admitted = measured*: nothing is accepted that no run measures,
and no code is kept for an interpreter nothing admits.

**What the program guarantees of that target (user decision 170).** Admission compares the running
interpreter's **version** with the pin. So the guarantee is: **an interpreter whose version differs from the pin
is never admitted silently** — every launcher refuses it, naming it. An interpreter **of the pin's version from
another source** (another build ahead of mise's shim, an `[env] _.path` entry, an rc-redirected version file) is
admitted; that is a stated limit (§11), not part of the guarantee, and §4.2's CI rule is not widened to chase it.

---

## §2 Coupled invariants

Seven invariants, I1–I6 and draft 6's I7 (decision 167); the pair table holds all 21 pairs.

- **I1 — one spelling.** The version is spelled once: the root `.tool-versions`, which is **exactly one line,
  `python X.Y.Z`** (H2). Its readers: mise (shim, `mise run`, `mise install`), `actions/setup-python`
  (`python-version-file`), and one stdlib module, `scripts/python_pin.py` (NEW), whose parser accepts that one
  form and nothing else and which the driver's Step 1 and every entry-point guard use. No second literal, and
  **no second spelling any reader honours**: in CI a `python-version` input (I1 × I3); in the tracked tree,
  **T5** (§4.2) — one fail-safe rule over tracked paths and text that **does not model mise's configuration
  surface** (decision 150 (1)): the root `.tool-versions` is the one line; every other tracked file whose name
  could be a mise / rtx config or rc carries no `python` outside a fixed set of admitted occurrences;
  `.miserc*`, a non-root `.tool-versions` and any `.python-version*` are red outright; whatever the rule cannot
  classify falls to red. What T5 cannot see — mise reading an input outside those names — is not silent **when it
  changes the version**: mise then launches a non-pin interpreter, admission refuses it, and the refusal asks mise
  what it resolved and from which input (I5 × I6). When it leaves the version at the pin (§11 T1, T2), it is a
  stated limit (decision 170).
- **I2 — admission = measurement.** The gate (`scripts/python-suites.sh`, both callers) and every direct entry
  point admit exactly the pin's version; the only measurements are runs on the pin (the `tools` job's one leg;
  local mise); no tracked code keeps a branch for an interpreter the pin excludes. **The supported path is
  defined by what runs** (H6): a launch is on it iff the running interpreter's version is the pin. `mise run`, a
  shims-first PATH and `mise activate` are ways to reach it, not its definition — PATH order differs per session
  (decision 142). Which build of that version runs is not admission's to check (decision 170, §11).
- **I3 — CI job shape.** One leg; the setup step consumes the pin file and nothing overrides it; the runner image
  is pinned for build availability; the readers model exactly this shape and read anything else red.
- **I4 — entry-point population.** Derived on every run from tracked files by **launch evidence**, never from a
  list or a shebang grep; anything that is neither an entry point nor reachable by import falls to **red**; a
  second, independent derivation (launch instructions in tracked text) is a lower bound that ratchets it, and it
  also bounds **every** exemption: a tracked launch instruction whose target is an exempt file (a set member, X1;
  the vendored core, X2) is red.
- **I5 — refusal before behaviour, and a remedy that is never a dead end.** Two halves (H4), as for I2.
  *Gate half (PR-1)*: `python_pin.py`'s admission and refusal, which the driver's Step 1 runs before any suite. A
  refusal exits non-zero and prints **observed facts only, and infers no cause** (PM decision 217): the pin and
  its file, the version found, **`sys.executable` as reported**, and **mise's answer** (§4.2: per element of `mise
  ls --current --json python`, its `version`, `source`, `installed` and `install_path`; `mise which python3`'s
  result) — or that mise gave none, with mise's own error lines. Then one fixed block of generic actions, the
  same in every refusal, none claimed to apply (H6, never a dead end: §4.2 *The refusal*). The oracle is
  diagnostic only; it never admits.
  *Entry-point half (PR-2)*: in each entry point the guard runs the same refusal before any import that can fail
  or behave differently by version.
- **I6 — provisioning.** The pin is installed as an attested precompiled build or not at all. Local: mise, with
  **both** install settings spelled in the tracked `mise.toml [settings]` — `python.compile = false` (no source
  build) and `python.github_attestations = true` (verification on for Python, independent of the global
  `github_attestations`) — and PR-1's reader asserts both values; CI: setup-python's manifest build for the pinned
  image. The first use auto-installs; the landing step installs it everywhere. **From PR-1 on, I6 holds for what
  mise installs from the tracked configuration** — not for which interpreter of the pin's version a launcher
  runs: a tracked or untracked `[env] _.path` entry makes `mise run` / `mise exec` launch another `python3`
  ahead of mise's install (measured, §11 T1), and admission, comparing the version, admits it when it reports the
  pin. That is decision 170's stated limit, not an I6 guarantee. Untracked overrides: the refusal path and the
  driver (the Python step of `mise run tools-test` / `mise run ci`) **warn** when any `MISE_PYTHON_*` variable
  other than `MISE_PYTHON_VERSION`, or `MISE_GITHUB_ATTESTATIONS`, is set (decision 150 (4), R5 MIN); an override
  in force at install time is a stated limit (§11). (`MISE_PYTHON_VERSION` is a version input: the oracle names
  it, §4.2.)
- **I7 — hermetic refusal tests (decision 167, T3).** Every test, and every child process a test starts, that can
  reach the refusal path — PR-1's N1 and the oracle fixtures, PR-2's E1 and any other — runs mise **isolated**:
  the child's `PATH` puts the test's fake `mise` first or holds no `mise` at all (asserted with
  `shutil.which("mise", path=child_PATH)` = the fake, or `None` for the no-mise fixture), and its environment sets
  `MISE_DATA_DIR`, `MISE_CACHE_DIR`, `MISE_STATE_DIR` and `MISE_CONFIG_DIR` to directories under the test's own
  temporary root and `MISE_GLOBAL_CONFIG_FILE` to an empty file there, and drops every other `MISE_*` variable
  of the outer environment. So **CI and local take the same path** (the fake's answer, not "the developer has
  mise, CI has none"), and **the user's real mise state is untouched** (measured: an unisolated `mise ls
  --current --json python` in a project dir adds entries to `$MISE_STATE_DIR/tracked-configs`). Owner: PR-1 —
  one helper, `isolated_mise_env(tmp_root, fake_mise=…, extra_env=…)`, in a **support module that is not a set
  member**, `scripts/python_pin_testing.py` (no `test_` prefix, and no path component starting `mise` / `rtx`, so
  T5's clause 2 does not read it; R6 M7): the driver loads each member under a unique module name
  (`scripts/python-suites.sh:642`), so a helper inside a member would be a second module copy for PR-2's suites.
  **Its shape is fixed by the driver and by PR-2 (R7 A1-1)**: it defines no `unittest.TestCase` subclass and no
  top-level `__main__` guard (driver step 3 (a) / (b), `scripts/python-suites.sh:23-45`), and every test loads it
  by path through **one string constant equal to its root-relative path**, `"scripts/python_pin_testing.py"` —
  PR-2's *named* reachability form; an assembled `os.path.join(…, "python_pin_testing.py")` would leave it an
  orphan. **Child, not in-process (R6 M5)**: every fixture that reaches the oracle or reads the environment for
  the warning runs `python_pin.py` in a **child process** started with the helper's environment; the child's
  launcher is a source string held in `python_pin_testing.py` and run as `<sys.executable> -c <launcher> …` (it
  replaces `sys.version_info` and loads `python_pin.py` by path), so it launches no tracked file and is no entry
  point. The oracle's own `subprocess` inherits the isolated environment, never the test runner's. **The offline
  regime holds in the child (R7 A2-6)**: the helper starts from `os.environ`, drops every `MISE_*` key, sets the
  five variables and `PATH`, and changes no regime key (`PYTHONPATH`, the proxy keys, `NO_PROXY`,
  `XDG_CACHE_HOME`; `scripts/python-suites.sh:66-103`); the child runs without `-I` / `-E` / `-S`, which would put
  it in the driver's unguarded-child class; a control asserts the regime keys equal the parent's. In-process
  tests are kept for pure functions (the one-line parser, the admission predicate, the text built from a given
  oracle answer). PR-2 reuses the helper for E1 and every test that executes an entry point. Pins:
  (a) the fake `mise` records the environment it was started with, and a control asserts each of the five
  variables points under the test's root and the global file is empty; (b) a control runs one refusal through the
  helper and then asserts that **no** entry of the real `tracked-configs` / `trusted-configs` (under the outer
  environment's `MISE_STATE_DIR`, else `$XDG_STATE_HOME/mise`, else `~/.local/state/mise`; read only; an absent directory, as in CI, is an empty set and
  the assertion holds — never a `skipTest`, which the driver reads as red) resolves into the test's temporary root — race-free under parallel sessions, which add
  entries of their own; (c) a mutant that drops the helper turns (a) red everywhere and (b) red locally. (b) compares **real paths on
  both sides** — `os.path.realpath` of each entry's link target against `os.path.realpath(tmp_root) + os.sep` —
  because macOS `tempfile.mkdtemp()` returns `/var/folders/…` while mise records `/private/var/folders/…`; a plain
  prefix test never matches, and the mutant would survive locally (R6 M3, measured by R6's A2; `python3 -I -c
  'import tempfile, os; d = tempfile.mkdtemp(); print(d, os.path.realpath(d))'` prints both forms).

| pair | intersection |
|---|---|
| I1 × I2 | admission compares against the value read from the one spelling **at run time**, so editing the pin moves acceptance at every launcher at once — a second literal would survive the edit (the PR-1 pin that edits the spelling and expects acceptance to follow) |
| I1 × I3 | CI reads the pin only through `python-version-file`; a `python-version` input beside it silently wins (setup-python's documented precedence), so the `ci.yml` reader refuses any such input — CI's second spelling |
| I1 × I4 | the population pins name no version; every guard reads I1 through the one module, so adding an entry point adds no spelling |
| I1 × I5 | the refusal must read the pin on the interpreter it is about to refuse, before any version-sensitive import — so `python_pin.py` is stdlib-only, loadable by path, and compiles on old grammar — **a pin, not a header comment** (J2): a control `ast.parse`s the module with `feature_version=(3, 9)` (the oldest interpreter §11 measures the refusal on), red on the six constructs R§11 *J2 probe* lists as rejected (the one enumeration of the red set, R7 A4-6); because `feature_version` is best-effort (measured on the pin, 3.15.2, against a real 3.9.6: seven constructs that 3.9.6 rejects pass it — R§11's accepted rows), a second control holds the module's recorded SHA-256 beside the date and interpreter of its last real-old-interpreter run (§11), red whenever the module changes without a new record — it forces a new record, not the run (both limits: §11); its oracle uses only `subprocess` and `json` (present in every Python 3), imported on the refusal path only; and the pin file is one line (TOML would need `tomllib`, which a refused interpreter may lack). This binds the pin reader and the refusal only; the `mise.toml` reader runs after admission (ownership line below) |
| I1 × I6 | mise installs exactly the patch it **resolves**, and inputs mise ranks above `.tool-versions` can make it resolve something else — draft 4 measured sixteen project config paths ranked above it, R4 an rc file that renames the version file and a second `.tool-versions` line that wins, and mise can widen the set in any release (the repository does not pin mise). So the tracked tree's share is refused by T5 **by name and text**, not by a model of mise, and the rest — when it changes the version — is refused by admission and named by the oracle (symmetric with I1 × I3: the local second spelling); an input that leaves the version at the pin but changes which file or which `python3` mise uses is §11's limit (decision 170). And a resolved patch with no attested prebuilt **fails** the install instead of compiling source or skipping verification (`python.compile = false`, `python.github_attestations = true`) — so the spelling can only name a patch both installers serve (the bump criterion, §4.1); T5 admits exactly those two `[settings]` lines, whose table and values I6's assertion pins |
| I2 × I3 | one admitted interpreter ⇒ one leg: a second leg would measure an interpreter nothing admits. D3's intent — the heaviest code runs on every interpreter the gate tests — holds on the one leg (§5) |
| I2 × I4 | every entry point admits only the pin, so a missed entry point would admit anything — which is why an unknown falls to red, not to "not an entry point" |
| I2 × I5 | the refusal comes before any behaviour, or "admits only the pin" is false for the prefix that ran — for the gate, Step 1 before the first suite (PR-1); for an entry point, its guard before its first version-sensitive import (PR-2) |
| I2 × I6 | the pin is measurable only where it is installable; a pin with no build fails setup-python in CI and `mise install` locally — red on both callers, never a silent fallback to another interpreter |
| I3 × I4 | no CI step launches an entry point directly; the population check is a set member, so it inherits I3's trigger (every pull request to `main`) and runs on the pin |
| I3 × I5 | the refusal path is executed on the one leg by patching `sys.version_info` inside the pin, not by a second interpreter — so I5 needs no second leg (consistent with I2 × I3). CI has no mise, so the same pin runs the oracle against a fake `mise` on PATH that prints each measured JSON shape, one that hangs, one that prints garbage, one that fails with the untrusted-config error, and none at all — and, by I7, so does every local run, so both callers print the same facts; that it compiles and reaches the refusal on a real old interpreter is not measured in CI (§11; J2's two controls bound the drift between measurements) |
| I3 × I6 | CI provisioning is setup-python's manifest build for the pinned image; `runs-on` stays `ubuntu-24.04` with a new reason: the manifest serves builds per image, so the pinned patch must have one for that image |
| I4 × I5 | one pin proves both: it executes **each derived entry point** (the population, not a list) under a non-pin version with elidex imports poisoned, and expects the refusal and no import |
| I4 × I6 | untracked launchers (the user's global hooks, shell history) and untracked configuration (`mise.local.toml`, `MISE_*` in the environment, a config above the repository, an rc file) are outside the tracked population by definition; the launchers reach the pin through mise's shim when their cwd holds `.tool-versions` and auto-install it (§7.2). An untracked input that wins **on the version** makes mise launch a non-pin interpreter: admission refuses it and the oracle names it (measured for `mise.local.toml`, an rc-renamed version file and `MISE_PYTHON_VERSION`, §4.2). An untracked input that leaves the version at the pin but changes the source — an rc-renamed version file holding the pin, an `[env] _.path` entry, another same-version `python3` first on PATH — is admitted: decision 170's stated limit (§11 T1, T2). An untracked input that overrides an **install setting** is warned about while it is set in the run's environment and met by nothing once the install is done: admission compares the version only, and an installed interpreter carries no record of how its archive was verified or built — a stated limit (§11), an I6 concern, not a guard's |
| I5 × I6 | the refusal prints what I6's provisioning answers — mise's `version`, `source`, `installed`, `install_path` and `mise which python3`, or that mise gave none with its error lines — beside `sys.executable`, and the generic actions, among them I6's `mise install`; it never chooses among them (PM decision 217, §4.2 *The refusal*). Draft 4's remedy (`mise current python`) printed only the pin when another interpreter ran ahead of mise's install — a dead end (H6); printing `sys.executable` beside `mise which python3` shows the two differ, and the generic block holds the fix for each measured case |
| I1 × I7 | the scratch repositories of I7's tests copy `python_pin.py` and write their own `.tool-versions`; the isolated mise reads only those (its global config is the empty file), so a test's verdict never depends on the developer's pin, global `python = "latest"` or installs |
| I2 × I7 | I7 binds the **tests'** children, never the gate's own admission: the driver's Step 1 and a real `mise run tools-test` use the developer's mise as §7.1 describes; only the child processes a pin starts to exercise the refusal are isolated |
| I3 × I7 | CI has no mise and local has one; without I7 the two callers execute different branches of the same pin (T3). With the fake `mise` first on the child's PATH (or none), both execute the fake's branch — the oracle is measured identically on both callers |
| I4 × I7 | PR-2's E1 executes **each derived entry point** under a non-pin version; every such child goes through I7's helper, so the population's size does not multiply writes into the user's mise state |
| I5 × I7 | every oracle answer of the refusal is reached under the fake: the five-variable assertion (I7 (a)) runs inside the same fixtures that check the printed facts, so an answer reached without isolation is red |
| I6 × I7 | the isolated `MISE_DATA_DIR` holds no install, and the fake never installs, so no test downloads or installs Python, and the warning's variables are dropped from the child's environment unless a fixture sets them on purpose |

**Visibility vs admission (#564's shape).** For Rust, #564's composite action
(`.github/actions/pinned-rust-toolchain`) asserts in CI that the pin is the only toolchain installed, because
cargo *selects* a toolchain and nothing after it checks the choice. Python's I3 needs no such step: admission is
the driver's Step 1, run on the interpreter that executes the suites, so another interpreter present on the
runner is admitted by nothing; the setup step only provisions.

**Ownership.** **PR-0** = T5's name and content clauses (§4.2 clauses 2–5), the part of I1's tracked-tree half
that holds on `main` before the pin (§6). **PR-1** = I1's CI half and T5's clause 1 (the one-line pin file,
parsed by `python_pin.py`), I2 (gate half, incl. the removal of floor-motivated code), I3, **I5 (gate half: the
refusal, its oracle and the warning, run by Step 1)** (H4), all of I6 (provisioning, auto-install, CI build,
both install settings and their assertion — I6 as narrowed in §2: what mise installs, not which same-version
`python3` a launcher runs). PR-1 also converges the `mise.toml` reader on `tomllib`. That is the
consequence of **I2 × I3** — one admitted interpreter, one leg, the pin, which has `tomllib` — together with
**I6**: the assertion of the two settings reads their table and values from parsed TOML, which the A-iii text
reader's grammar does not cover (draft 4 also cited T5b's key classification here; T5b is retired, §4.2). It is
safe despite I1 × I5 because of an **ordering coupling**: the `mise.toml` reader is a suite member, run only by
the driver **after Step 1 admitted the pin**, never on an interpreter a guard is about to refuse; I1 × I5's
no-`tomllib` constraint binds `python_pin.py`, which runs before admission. **PR-1** also owns **I7's helper and
its controls** (the gate-half tests). **PR-2** = I2 (entry-point half), I4, **I5 (entry-point half)**, and I7 for
its own tests (it reuses PR-1's helper).

**Every pair × half, by PR** (J1 — after the I5 split, no pair is left to inference; C9, C17, C18 carry these
lists into the per-PR matrices). "Gate" = the half in the driver's Step 1 and its pins; "entry" = the half in the
entry-point guards and their pins.

| pair | PR-0 | PR-1 | PR-2 |
|---|---|---|---|
| I1 × I2 | — | gate (Step 1 reads the one spelling at run time) | entry (every guard reads it through the same module) |
| I1 × I3 | — | whole | — |
| I1 × I4 | — | — | whole (the population pins name no version; a new entry point adds no spelling) |
| I1 × I5 | — | gate (`python_pin.py`'s constraints, J2's controls) | entry (the prelude loads the module by path before any version-sensitive import) |
| I1 × I6 | the two admitted `[settings]` lines (clause 4 (b)), ahead of PR-1 | whole (both lines, their assertion, clause 1) | — |
| I1 × I7 | — | gate tests' scratch repos | entry tests' scratch trees (E3) |
| I2 × I3 | — | gate (one leg = the one admitted interpreter) | entry (the one leg runs E1, the entry-point pins, on the pin) |
| I2 × I4 | — | — | whole |
| I2 × I5 | — | gate (Step 1 before the first suite) | entry (the guard before the first version-sensitive import) |
| I2 × I6 | — | gate (no build → red on both callers) | entry (an entry point off mise's install is refused with I6's remedy, never run on a fallback) |
| I2 × I7 | — | gate tests | entry tests |
| I3 × I4 | — | — | whole (the population check is a set member on the one leg) |
| I3 × I5 | — | gate (`sys.version_info` patched inside the pin; fake `mise`) | entry (the same, per derived entry point) |
| I3 × I6 | — | whole | — |
| I3 × I7 | — | gate (the fake's answer on both callers) | entry (the same for E1) |
| I4 × I5 | — | — | whole |
| I4 × I6 | — | — | whole (untracked launchers and configuration are outside the population by definition; the naming is PR-1's oracle, consumed unchanged) |
| I4 × I7 | — | — | whole |
| I5 × I6 | — | gate (the printed facts and the generic block) | entry (the same text, reached from a guard) |
| I5 × I7 | — | gate | entry |
| I6 × I7 | — | gate | entry |

So PR-1's matrix holds fifteen pairs (I1 × I2, I1 × I3, I1 × I5, I1 × I6, I1 × I7, I2 × I3, I2 × I5, I2 × I6,
I2 × I7, I3 × I5, I3 × I6, I3 × I7, I5 × I6, I5 × I7, I6 × I7) and PR-2's eighteen (the entry halves of I1 × I2,
I1 × I5, I1 × I7, I2 × I3, I2 × I5, I2 × I6, I2 × I7, I3 × I5, I3 × I7, I5 × I6, I5 × I7, I6 × I7, and whole
I1 × I4, I2 × I4, I3 × I4, I4 × I5, I4 × I6, I4 × I7); PR-0's one cell is I1 × I6's admitted lines, whose
sufficiency as PR-0's whole scope is PR-0's plan's call (C11). Every one of the 21 pairs has a row here and in the pair
table above, and at least one owner (`grep -o '^| I[1-7] × I[1-7] |' <this memo> | sort | uniq -c` prints each of
the 21 pairs with count 2).

---

## §3. Spec coverage map

**No spec surface** — the umbrella and its PRs touch a pin file, a driver and its pins, a CI job, entry-point
preludes and the docs that state them. As for A-iii (its §3), `main`'s preflight exits 1 on this declaration
until A-ii's marker recognition lands; acceptance is that exit with exactly the no-table hard fail, plus
grep-pass with both strict flags printing nothing (A-iii §3's one-liner, with this memo's path).

---
## §4 Decisions

### §4.1 The version — 3.15.2 (decision record, 2026-10-31; corrects the 3.15.1 record of 2026-10-26)

**Criterion** (stated so every bump reuses it): *the newest bugfix-status CPython line whose exact patch both
installers serve as a prebuilt — mise's precompiled source (python-build-standalone) and setup-python's manifest
for `ubuntu-24.04` x64 — and on which every suite is green.* Reason for the line: ADR #36's rule as #567 applies
it (§1) and the stdlib-only tooling.

**Decision: 3.15.2.** Measured 2026-10-31 at `b051329b` (mise 2026.10.21): both installers serve 3.15.2 as a prebuilt
(mise with attestations verified on macOS; the Linux artifact exists and verifies, its install unmeasured under
decision 62; setup-python's manifest for `ubuntu-24.04` x64), and every suite is green on mise's 3.15.2 build.
It corrects draft 2's 3.15.1 record, which read an older python-build-standalone release. The rows, their commands
and the listing and compile-off findings are in **R§4.1** (rows 1a, 1b, 2, 3, 3x). A bump runs rows 1a, 1b, 2 and 3
— never the `mise ls-remote` listing alone — and compilation is off (I6) because, with it on, an exact install of
a patch without a prebuilt falls back to an unattested source build (R§4.1).

### §4.2 The spelling — `.tool-versions`

| | Spelling | Local | CI | Verdict |
|---|---|---|---|---|
| **S1** | `mise.toml [tools]` + a CI literal | mise | `python-version:` | rejected: two spellings tied by a pin (`mise.toml`'s own `[tasks.trip-wires]` comment rejects that shape) |
| **S2** | `.tool-versions` (`python X.Y.Z`, the only line) | mise reads it natively | setup-python `python-version-file: .tool-versions` | **chosen** |
| **S3** | `mise.toml [tools]` | mise | `jdx/mise-action` | rejected: a third-party action in the gate's trust path — kept rejected by PM decision 62 ("S3 kept", `PORTFOLIO:149`); also the guard would need a TOML reader on a possibly-old interpreter (I1 × I5) |
| **S4** | `.python-version` | **ignored** unless `idiomatic_version_file_enable_tools` includes python (the user's global sets `["rust"]`) | native | rejected: resolution depends on user config |

S2's fact rows (mise reads `.tool-versions` over the global `python = "latest"`; setup-python v7 reads it, and a
`python-version` input beside it wins): R§4.2. Accepted consequence, stated not hidden: the local build
(python-build-standalone) and the CI build
(actions/python-versions) are different builds of the same CPython release — PR-1 §9's slot
`#11-python-pin-single-builder` covers it.

**T5's two jobs (decision 150 (1)).** What protects I2 is admission, not a model of mise's configuration surface
(why the earlier shapes leaked: R§4.2). T5 keeps exactly two jobs: **(i) CI and local do not disagree silently**
— CI never runs mise, so a tracked input that local mise honours and CI does not must be red in CI or loud
locally; **(ii) the refusal's remedy is never a dead end.** (i) is the CI rule, (ii) the local oracle.

**T5 — the tracked-tree rule (CI).** Over `git ls-files`, stdlib only (no `tomllib`), a suite member run by the
driver after admission. Default home: the readers module and its pins (`scripts/python_suites_readers.py`,
`scripts/test_python_suites_ci.py`). That module reads two files' text (`ci.yml`, `mise.toml`) and T5 is a
predicate over the tracked tree, so **PR-0's plan records why it goes there or names its own module** (R6 M8),
and, if it takes its file list from the driver (§6, the offline-regime branch), states that the check then
reads data its launcher passes:
1. **The pin file (H2; PR-1).** The root `.tool-versions` is tracked and is exactly one line, `python X.Y.Z`
   (a final newline allowed; no comment, no blank line, no second tool), parsed by `python_pin.py`'s own parser —
   one reader, one form; locally Step 1 refuses the same file. Before PR-1 the file does not exist and clause 1
   does not exist either; clauses 2–4 leave the root `.tool-versions` to clause 1.
2. **Members (PR-0).** A tracked path is a member iff some path component, case-folded and with one leading `.`
   dropped, **starts with** `mise` or `rtx` (so `mise.toml`, `.mise.local.toml`, `.config/mise/conf.d/x.toml`,
   `mise/tasks/x`, `mise.ci.toml`, `.miserc.toml`, `rtx.toml`), or its basename is `.tool-versions` or starts
   with `.python-version`. At `ee6c7a1a` the only member is `mise.toml` (`git ls-tree -r --name-only ee6c7a1a
   | /usr/bin/grep -iE '(^|/)\.?(mise|rtx)[^/]*(/|$)'` prints `mise.toml`; `… | /usr/bin/grep -ciE
   '(^|/)(\.tool-versions|\.python-version[^/]*)$'` prints 0). The prefix over-approximates on purpose: a
   future `mise_helper.py` is a member too, red only if clause 3 or 4 fires, and the remedy is a rename.
3. **Red outright (PR-0).** A member with a component starting `miserc` (after the dot drop) — mise's rc file
   chooses which files mise reads, so no content of it is admissible (H1); a `.tool-versions` other than the
   root's (and, through H3, never read by the guard either, §4.3); any `.python-version*` (read by mise once a
   user enables idiomatic files for python, and by other tools).
4. **No Python spelling in any other member (PR-0).** Red on any case-insensitive occurrence of `python` in a
   member's text except (a) inside a substring that is the path of a tracked file (`scripts/python-suites.sh` in
   `mise.toml`'s `run` line and comment), and (b) in the root `mise.toml`, the two lines `python.compile = false`
   and `python.github_attestations = true` exactly — admitted ahead of PR-1, which adds them and pins their table
   and values over parsed TOML (C10). This covers without naming them `[tools] python` / `"core:python"`,
   `[tool_alias…]` entries for python, `tasks.<n>.tools`, a file task's `#MISE tools=` header, `[env]
   MISE_PYTHON_VERSION`, `idiomatic_version_file_enable_tools = ["python"]`, every `MISE_ENV` variant (the R4 MIN
   that draft 4's probe listed five of mise's eight `MISE_ENV` paths is moot: every such path is a member), and
   any other key mise adds later that names python. `main`'s `mise.toml` has one line with occurrences outside (a) and (b):
   the `tools-test` description (`mise.toml:115` at `ee6c7a1a`, "Run every tracked Python suite … (floor:
   Python 3.9)"); PR-0 rewords it truthfully without the word (the interpreter first on PATH, floor 3.9), and
   PR-1 rewrites it to the pin (§4.4).
5. **Diagnostic.** Each red names the path (and line), the clause, and the remedy: the Python version is spelled
   only in the root `.tool-versions`; reword (a description need not say "Python"); for a new install setting,
   extend clause 4 (b) together with I6's assertion in the same PR.

Not modelled, by design: which keys or files mise reads, their precedence, `MISE_ENV`. A `[tools]` entry for a
non-Python tool is **green** (draft 4's T5b made it red, §8 row *every lane*). A second tool cannot go into
`.tool-versions` (clause 1); it goes into `mise.toml [tools]`.

**Residual, stated.** T5 misses an input mise reads whose path has no component starting `mise` / `rtx` and is
not `.tool-versions` / `.python-version*`, and a Python spelling in a member that does not contain the letters
`python`. Examples: a future idiomatic version file; a TOML escape (`"python"`) — the last is deliberate
tampering, the class `#11-aiii-gate-tamper-hardening` holds; and **T1**'s `[env] _.path = ["altbin"]` in a tracked
`mise.toml` (no `python` in the text, so clause 4 is green) — §11. **None that changes the version is silent**
(decision 170): mise then launches a non-pin interpreter, admission refuses it, and the oracle names the winning
input. **One that leaves the version at the pin is not covered** — it is §11's limit (T1, T2), and the CI rule
is not widened for it. What the version-changing residual loses is the CI red: the disagreement surfaces at the
next local run, not on the pull request. Two candidates were measured benign in mise 2026.10.21 — `[env] _.file`
setting `MISE_PYTHON_VERSION`, and `override_tool_versions_filenames` in `mise.toml [settings]` (R§4.2); either
turning version-changing in a later release is caught by admission like any other version input.

**The local oracle (decision 150 (1); PR-1).** On refusal only — admission never consults mise — `python_pin.py`
asks mise at the process cwd (where the shim resolved the interpreter that is running) **two questions and no
other** (PM decision 217 withdrew draft 7's `mise env --json` read):
- `mise ls --current --json python` → a JSON array (R5 MIN); per element, `version`, `source` (`{"type", "path"}`
  for a file, `{"type": "environment", "key", "value"}` for a variable), `installed` and `install_path`, printed as
  mise gives them. `[]` is printed as "mise resolves no Python at <cwd>". Order and count are mise's (requested
  versions in request order, R§4.2); the refusal reads no meaning into them. Fixtures: `[]`, one and two elements;
- `mise which python3` → its output when it exits 0, else its exit status (1 while not installed, R§4.2).

**Measured shapes** (2026-10-31, mise 2026.10.21; R6 re-ran them on 2026.10.25): R§4.2's table — the version and
`source` for each input class, the installed shape, `[]`, two elements, T1, T2 and the untrusted case. From it:
`mise current python` (version only) and `mise config ls` (loaded files, no winner; it omits the variable and the
rc file) are not the oracle, and `mise where python` / `mise which python3` exit 1 while the version is not
installed.

**The refusal — facts, then one generic block (PM decision 217).** Every refusal exits non-zero and prints what it
observed, and nothing it inferred:
1. the pin, the pin file `<root>/.tool-versions`, and the version running;
2. **`sys.executable` as the interpreter reports it**, not resolved (macOS `/usr/bin/python3` reports the Xcode
   path, R§11 *The real old interpreter*);
3. the cwd and the repository root, as two paths (§4.3);
4. **mise's answer** as above — or, when mise is not on PATH, does not answer within a short timeout, exits
   non-zero or prints what does not parse (mise is not pinned; its JSON is its own), that fact and **mise's first
   three stderr lines** verbatim (for an untrusted worktree they include mise's own `mise trust` line, R§4.2);
5. the install-settings warning lines (below), when one of their variables is set.

Then the same block in every refusal, claiming none of its actions applies: "To run on `<pin>`: install it —
`mise install` in `<root>` (mise itself: README / CONTRIBUTING, *Python*); or remove or change the mise input
that decides Python here — the `source` above, a file `mise config ls` lists, or an rc file `.miserc*`, which it
does not list; or launch through mise — `mise -C <root> exec -- <tool>`; or remove what puts `<directory of
sys.executable>` ahead of mise's install on PATH — this shell's PATH, or an `[env] _.path` entry of a mise
config. mise's answer again: `mise -C <root> ls --current --json python`, `mise -C <root> which python3`."
**Never a dead end (H6)** is checked per measured case in R§4.2 *Each case's fix* — by this memo, not by the
refusal, which never says which action applies. The fourth action is draft 8's: decision 217 named three, and
without it T1 (b) is a dead end (`mise exec` runs `altbin/python3` too, R§11 *T1*).

**The install settings, what overrides them, and the warning (G5; decision 150 (4)).** mise knows the key
`python.github_attestations`, and with both tracked settings in place the python-specific key wins over a global
`MISE_GITHUB_ATTESTATIONS=false`, which is why I6 spells it; `MISE_PYTHON_GITHUB_ATTESTATIONS=false`,
`MISE_PYTHON_COMPILE=1` and a `mise.local.toml [settings] python.compile = true` each override the tracked value
(measured 2026-10-31 and at R3, mise 2026.10.21: R§4.2). **Warning**: `python_pin.py` prints one stderr line per variable set —
**every `MISE_PYTHON_*` variable except `MISE_PYTHON_VERSION`** (R5 MIN: `MISE_PYTHON_COMPILE` and
`MISE_PYTHON_GITHUB_ATTESTATIONS` are the two measured; any other `python.*` setting mise has or adds is spelled
the same way, so the prefix covers it without a list; `MISE_PYTHON_VERSION` is a version input, which admission
and the oracle handle), and the global `MISE_GITHUB_ATTESTATIONS` (warned although the tracked python key wins
over it in 2026.10.21, because that precedence is mise's) — saying that it may override the tracked install
setting for any install made in this environment. It is printed on the refusal
path and by the driver's Step 1 on every run, so every `mise run tools-test` / `mise run ci` shows it; it never
changes an exit status. A variable set at run time does not prove what was in force at install time, and CI does
not run mise, so there is **no CI red** (decision 150 (4)); an install already made under an override, and an
override through an untracked config file, stay §11's stated limit.

### §4.3 Admission — exact, everywhere

The gate **and** every direct entry point admit `sys.version_info[:3] == pin` with `releaselevel == 'final'`.
Draft 1 admitted the pin's minor at entry points; R1 noted that this admits unmeasured patches while rejecting
3.11 as "unmeasured" — inconsistent. Exact admission removes the class: admitted = measured (§1). One predicate,
one reader, two kinds of launcher.

**Where the pin file is found — one fixed location (H3).** The pin file is the repository root's
`.tool-versions`, and the root is located from **`python_pin.py`'s own path** (`scripts/python_pin.py` → the
parent of its directory) — never from the cwd, never by walking up. The driver loads `python_pin.py` from its own
directory (`scripts/`); PR-2's guard locates the root from its entry file's own path by that file's fixed tracked
depth (a count of parent hops, so it spells no path — the K2 constraint, §8 #501) and loads
`<root>/scripts/python_pin.py` by path. Draft 4's rule ("the nearest ancestor of the launched file that holds a
`.tool-versions`") would read an untracked nested `.tool-versions` (R4 H3), which T5's clause 3 cannot see because
it reads tracked files only. mise, by contrast, resolves from the cwd and walks up; when an untracked nested
`.tool-versions` makes mise launch another interpreter, the refusal names it as the oracle's `source`.

Consequence, accepted: a tool launched by absolute path from a cwd without the pin runs on that cwd's mise
python and is refused when it differs; the diagnostic names `mise -C <repo> exec -- <tool>` (§7.1).

### §4.4 Interpreter-below-floor wording disappears

There is no floor: one interpreter is admitted. Every statement of "the floor" or of two legs is rewritten to the
pin. **Discovery is authoritative; the list here is examples**: A-iii L5, §4.4, Q5, the driver header,
`mise.toml`'s task description, CLAUDE.md `## CI` (`:100` at `ee6c7a1a`), CLAUDE.md `### Workflow`'s "Plan-memo umbrella
checker" bullet (#510's, "matrix の両 leg", `:54`), README / CONTRIBUTING, and #510's statements and code
branches (§5.2). PR-1 §4.6 gives the discovery command and the classes.

---

## §5 Rules amended

### §5.1 A-iii and #510's memo

The user decided to pin with mise (PM decision 41). With one admitted interpreter the CI job has one leg, so the
landed rules that presuppose two legs are amended **in place** by PR-1, each with a dated "amended
<PR-1's landing date> by <PR-1>" marker (CLAUDE.md's SUPERSEDED-marker precedent).

| Rule | Before (quoted) | After | Reason |
|---|---|---|---|
| **A-iii D3** (§4.4) | "`--mutants`-shaped self-tests run on **both** legs, with no per-leg branching (D3): a check that skipped the floor leg would leave the floor unmeasured for exactly the heaviest code." | "`--mutants`-shaped self-tests run on the `tools` job's one leg — the pinned interpreter — with no interpreter-conditional branching (D3): the heaviest code runs on every interpreter the gate measures, which is exactly one." | D3's intent is "run the heaviest code on every interpreter we test, never skip it per leg"; with one leg, running it on that leg keeps the intent. A consequence of the user's decision, not a change to the rule |
| **A-iii §8** | "The legs run in parallel, so the job's wall clock is the slower leg's." | "The job has one leg, the pinned interpreter; its wall clock is that leg's." (The dated 3.9-vs-3.14 measurement paragraph stays as the record of its date.) | same |
| **A-iii §12(1)** | "`mise run tools-test` and `bash scripts/python-suites.sh`, on both legs." | "… on the pinned interpreter (the `tools` job's one leg; locally, mise's install of the pin)." | same |
| **#510 memo §8**, entry "The always-run self-test's cost grows with REVIEW ROUNDS, not with the program", *contract* (`docs/plans/2026-08-plan-memo-umbrella-checker.md:1116-1117`) | "… the self-test enters that job through `.claude/tools/test_plan_memo_umbrella_check.py` on both matrix legs; the slower leg bounds the job." | "… through `.claude/tools/test_plan_memo_umbrella_check.py` on the job's one leg, the pinned interpreter, which bounds the job." The trigger ("the round whose run measures under 2× against the `tools` job's `timeout-minutes`, or the next round that adds a mutant naming the growth property", `:1123-1125`) is unchanged | the third live two-leg rule (F4); its substance — a ratio against the job's timeout — survives the leg count |
| **#510 memo §1** *Interim connection on `main`* (`:73`) | "… which the `tools` job (`Python suites`) runs on both matrix legs" | "… runs on its one leg, the pinned interpreter" | same |

**Which timeout rule PR-1 follows.** Two live rules govern the `tools` timeout: `ci.yml`'s job comment (A-iii's: a
PR that adds suite cost re-derives `timeout-minutes` from its own run) and #510's §8 trigger (a run under 2×
against the timeout re-opens the self-test's budget). PR-1 adds pins and changes the interpreter, so it
**re-derives under `ci.yml`'s rule** from its own run, and sets the value so that the run is not under #510's 2×
trigger; if no value can satisfy both, PR-1's report to the PM names #510's trigger as fired (for the VM P4 lane).

Also amended — not user-ratified rules, rewritten as statements PR-1 falsifies: A-iii L5 and the five `L× L5`
pair rows, §4.4, Q5, Q7, R2, §6's text-reader rationale and §6.1's M-rows (PR-1 now owns the `tomllib` reader,
§6), §12(6), §13's Slice-B row, and **§6's UNCHECKED floor line** (A-iii `:441-443`, "the interpreter floor on
`SKILL.md`'s direct `preflight.py` path … assigned to Slice B's floor raise"). Its owner is split by stage:
**PR-1 rewrites** it to "the pin on `SKILL.md`'s direct `preflight.py` path — UNCHECKED, assigned to PR-2 of the
python-pin umbrella" (PR-1 §4.6's existing wording); **PR-2 flips** it to CHECKED, naming its pin (§13 C13).

**The CI/tools lane SSoT's hand-off ledger** (`project_ci-tools-lane.md`, "#514 rework checklist") is amended,
via the PM who records it. Under decision 124 #514's re-derive runs **between PR-1 and PR-2**, so every item cites
**only PR-1's artifacts** — the landed umbrella (`docs/plans/2026-10-python-pin-umbrella.md`, which lands with
PR-1), `.tool-versions`, `scripts/python_pin.py`, the driver's Step 1 and the one-leg `tools` job — and names
PR-2 only as the owner of what is still UNCHECKED, through the umbrella (G2):
- **Item 3** ("the UNCHECKED 'interpreter floor on SKILL.md's direct preflight.py path (A-iii's)' → '(Slice
  B's)'") becomes: "→ 'the pin on SKILL.md's direct `preflight.py` path — UNCHECKED, assigned to PR-2 of the
  python-pin umbrella (umbrella §6, landed with PR-1)'". **Later**, PR-2 changes it to "CHECKED by PR-2's
  entry-point pin" (§13 C13).
- **Item 4** ("B's raise moves the script's assertion, the leg's `python-version`, and the direct … invocation
  paths, incl. SKILL.md Step 0 (B owns them)") becomes: "B requires ≥ 3.11, which the pin (`.tool-versions`,
  admitted exactly by the driver's Step 1, PR-1) exceeds; B's edit set has no interpreter change; B's F1 is
  withdrawn — the direct-launch admission it covered is UNCHECKED and assigned to PR-2 of the python-pin umbrella
  (umbrella §2 I4/I5, §6)". **Later**, PR-2 names F1's successor, its E1 (`scripts/test_python_entry_points.py`).
- **Item 5** (delete the seven `A-rederive*.sh`; no harness lands) is unchanged, which also retires draft 1's
  edits to `A-rederive-Aiii.sh`'s `floor()` block.

### §5.2–§5.3 The VM P4 lane's files and lines — in *Landing file 2*

§5.2 (the VM P4 lane's files and lines PR-1 edits, V1–V23, with the manifest rule — the consent subject of
decision 127 (a) and decisions 205, 211) and §5.3 (the lines PR-1 and PR-2 meet on `main` since #506 landed, one owner
and one time per row) land as `docs/plans/2026-10-python-pin-umbrella-vm-p4.md`, numbers kept; their reader is the
VM P4 lane (§6 *This umbrella's own length*).

---

## §6 The split, and why these seams

| PR | Scope | Depends on |
|---|---|---|
| **PR-0 tracked mise config** | T5's clauses 2–5 (§4.2): the member predicate over `git ls-files`, the red-outright clause (`.miserc*`, a non-root `.tool-versions`, `.python-version*`), the no-`python` content clause with its admitted occurrences, the diagnostic, their controls — by default in the readers module and its pins (`scripts/python_suites_readers.py`, `scripts/test_python_suites_ci.py`; PR-0's plan records why, or names its own module, §4.2); and the one truthful reword of `mise.toml:115` that clause 4 needs | the umbrella's and PR-0's own plan-review; nothing else (no VM P4 file, no pin) |
| **PR-1 pin, gate and provisioning** | `.tool-versions` (the one line); `scripts/python_pin.py` (the one reader: the one-line parser, admission, the refusal with its oracle and the warning; root from its own path); the driver's Step 1 → exact; `tools` job → one leg reading the pin file; the `ci.yml` reader and its pins; T5's clause 1; the `mise.toml` reader → `tomllib` (draft 2's PR-3: rows M1–M3 retire, Q4 / M2 restated over parsed data) with `[settings] python.compile = false` + `python.github_attestations = true` and their assertion; `mise.toml`'s task text; the removal of #510's floor-motivated code and the VM P4 statements and lines V1–V23 (§5.2); I7's support module `scripts/python_pin_testing.py` (§2); docs; the amendments (§5) | PR-0; its own plan-review; the VM P4 lane's consent to V1–V23 and its answer on V6 (decision 127 (a), decisions 205, 211) |
| **PR-2 entry-point guard** | the launch-evidence predicate, the guard prelude in every derived entry point (root by the entry file's fixed depth, §4.3), their pins (incl. the X1 bound); `SKILL.md` Step 0's comment; the `tools` timeout re-derive in `ci.yml` (PR-2 §8); the UNCHECKED lines naming it become CHECKED (§5.1, §13 C13); the two VM P4 launch lines P2 (§5.3 (b), decision 205) | PR-1; its plan reviewed before PR-1 lands; the VM P4 lane's consent to P2 (the same request as V16–V23, §9 step 3) |

**Ordering** (decisions 124, 127, 118 and 150 (3)): PR-0 → PR-1 → **the PR-1 → PR-2 interval** (#514's re-derive
starts, CI/tools: it edits #514's memos and launches nothing; the M4-12 exit-gate umbrella's pre-research may
start) → PR-2 → the **cargo-deny PR** → the **M4-12 exit-gate program** → the K2 wire program (decision 118:
"before the K2 program"). Placement of the three items of H5:
- **cargo-deny PR** (CI/tools; pins cargo-deny in `ci.yml`'s `deny` job and `audit.yml`, `deny.toml [graph]
  all-features`, drops rust-cache from the `deny` job — lane SSoT, decision 76): decision 150 (3) puts it between
  PR-1 and PR-2 **unless PR-2 also edits `ci.yml`**. PR-2's plan does not say "edits `ci.yml`" in words (R5 MIN);
  it is **inferred**: its §8 has the new member's cost "join the `tools` timeout re-derive", and `ci.yml`'s own
  timeout rule (the `tools` comment, `ci.yml:224-235` at `ee6c7a1a`, unchanged since `1ed126b4`) makes "a PR that
  adds suite cost" re-derive `timeout-minutes` **in the same PR** and record its run in that comment (as #549 and
  #510 did). C19 makes PR-2's plan state it. So the cargo-deny PR goes **after PR-2**. If PR-2's plan-review
  removes its `ci.yml` edit, the other branch of decision 150 (3) applies and the cargo-deny PR moves to the
  interval.
- **M4-12 exit-gate program** (decision 118, owner CI/tools): after PR-2 (decision 150 (3)) and after the
  cargo-deny PR (decision 118: "after the Python pin and cargo-deny PR"); its umbrella pre-research may start
  after PR-1.
- **Sweep 4** (A-iii post-merge sweep: the driver items K16 / K27 / K28 / K29, the §13 sweep's K13, the spawn
  false red, and G13 — lane SSoT "Next sweeps", §10): it touches `scripts/python-suites.sh`,
  `scripts/test_python_suites_regime.py` and A-iii's memo; the ledger lists no `ci.yml` line for it — an
  inference from its item IDs, not a reading of its plan (R5 MIN): **K27** (the driver's 600 s `LAUNCH_TIMEOUT`
  against the job's `timeout-minutes`) may touch `ci.yml`. So, by decision 150 (3), **anywhere** unless its plan
  finds a `ci.yml` edit (K27 or another), in which case it takes the cargo-deny PR's rule. Its overlap with PR-1
  is textual (the driver's Step 1 region; whichever lands second re-reads); before PR-1 it is green on both legs,
  after PR-1 on the pin.
- **The queued `RUSTUP_TOOLCHAIN` unset in mise tasks** (lane SSoT "Queue", decision 142's follow-up: `mise run
  ci` / `lint` must not let an inherited stale `RUSTUP_TOOLCHAIN` override `rust-toolchain.toml`, e.g. a
  task-level unset) edits `mise.toml` — **textual** with PR-0 (the `:115` reword) and PR-1 (`[settings]`, the
  task text): anywhere, whichever lands second re-reads. It spells no `python`, so T5's clause 4 stays green; if
  it lands after PR-0, its text is under clause 4 like any other member line.
- **The `ci.yml:167` docs item** (lane SSoT "Next sweeps", 10-25, "after #501"; the entry names a line at an
  older base, not its subject): it edits `ci.yml`, so by decision 150 (3)'s `ci.yml` rule it goes **after PR-2**,
  as the cargo-deny PR does (PR-2 edits `ci.yml`; PM decision 211 D7-3). Its plan re-derives the line.
- **The three CI/tools slots registered 2026-10-31 / 11-01** (R6 M12; slot ledger `project_open-defer-slots.md`),
  placed by decision 150 (3)'s `ci.yml` rule; none touches the `tools` job or a Python file, so none widens the
  interval's class. `#11-elidex-sandbox-workspace-lints-missing` (that crate's `Cargo.toml` and code): no `ci.yml`
  edit → **anywhere** — and it landed so, as #579 `593aa314` after this base (§0; slot closed, PM decision 214).
  `#11-private-intra-doc-links-unchecked` and `#11-elidex-js-default-features-build-broken`: each may add or
  change a `ci.yml` job (`--document-private-items` in `doc`; a feature-matrix job, still undecided per its
  row) → **after PR-2 if its plan edits `ci.yml`**, the cargo-deny PR's rule; otherwise anywhere. The
  default-features code fix goes to the lane that owns the failing modules (its row) and touches no file here.

**#506 landed before PR-1** (`3ec0ae5c`, §0): there is no absorption step; PR-1 and PR-2 meet #506's lines on
`main` (§5.3), with the VM P4 lane's consent (§9 step 3). #514's landing and PR-2's landing do not wait for each
other (§5.1, §13 C13). Nothing in this order waits for another lane's PR.

**PR-0 rebuilt, not dropped (decision 150 (1), "decide by the interval-free rule").** The criterion for splitting
PR-1 is an *interval-free* seam: a part that can land before the rest **without weakening any invariant that holds
on `main`** during the interval — and, as draft 4 used it for T5a ("so PR-1 adds the file without editing the
check"), one that the rest **adds to without editing**. It is applied to what PR-0 checks, not to what PR-0
leaves for PR-1 (R5 MIN: drafts 4–5 applied it unevenly — the two `[settings]` lines PR-0 admits ahead are absent
on `main`, and the root `.tool-versions` is unchecked until PR-1; *The PR-0 → PR-1 interval* below states both).
Applied to the new T5, clause by clause:
- **Clauses 2–5 (members, red outright, no `python`, diagnostic)** — interval-free: on `main` the one member is
  `mise.toml`, whose occurrences are tracked-path substrings once `mise.toml:115` is reworded (truthfully: the
  description then says "the interpreter first on PATH", which `main` still runs); stdlib plus `git ls-files`,
  so it runs on both of `main`'s legs; the two `[settings]` lines are admitted ahead, as draft 4 admitted the
  root `.tool-versions` "when present"; the root `.tool-versions` is left to clause 1. PR-1 adds clause 1 and
  the settings lines **without editing** clauses 2–5. **Taken — PR-0**, on one condition: §11's
  offline-regime question (may a suite member run `git ls-files` under A-iii's regime on both of `main`'s legs?)
  is answered yes by PR-0's plan; if no, PR-0 takes its file list from the driver, which already runs `git
  ls-files` (`scripts/python-suites.sh:435`), and PR-0's plan says which (R5 MIN). It also puts H1's red
  (`.miserc*`) on `main` before the pin.
- **Clause 1 (the one-line pin file)** — no: its parser is `python_pin.py`'s (one reader, I1 × I5), which before
  PR-1 reads a file that does not exist (dead code; CLAUDE.md: connect or delete), and a second parser in PR-0
  would be replaced by PR-1 — a seam PR-1 edits. **Stays in PR-1.**
- **The oracle and the warning** — no: they are the refusal's remedy, and before PR-1 nothing refuses. PR-1.

**The PR-0 → PR-1 interval** (R5 MIN — stated, like the PR-1 → PR-2 interval below). Holds: clauses 2–5 on
every tracked mise-named file, on both of `main`'s legs. Not checked: the **root `.tool-versions`** (clauses 2–4
leave it to clause 1, which is PR-1's) — a root `.tool-versions` added in the interval is green in CI whatever
it holds, and local mise reads it; and the two `[settings]` lines are **admitted but unpinned** (clause 4 (b)
admits them as text; their values are pinned only by PR-1's parsed-TOML assertion). Neither weakens an invariant
that holds on `main`: `main` has no pin, so no reader of a pin can disagree, and the admitted lines are the
values PR-1 asserts (a flipped value is not the admitted text, hence red by clause 4). Bounded by order: PR-1 is
the lane's next pin PR after PR-0; PR-1's base re-reads the tree (C5's "red at main").

**PR-1 is itself edge-dense (G3, H4).** It owns I1 (CI half, clause 1), I2 (gate half), I3, I5 (gate half), I6
and I7's helper, whose fifteen gate-half pairs (§2 *Every pair × half*) all meet inside it — the ≥3-axis trigger
of CLAUDE.md *Edge-dense work*. So PR-1's plan carries its **own intra-PR
§2 pair matrix** (each pair with its PR-1 mechanism and the pin that holds it, plus the slice-local pairs —
scratch repos × I1, the oracle × I3 (no mise in CI), the VM P4 removals × I2, the `mise.toml` reader × Step 1's
ordering, and PR-1's own `mise.toml` writes × PR-0's clause 4) and passes **its own plan-review** (§13 C9). It is a terminal unit under CLAUDE.md *Edge-dense work*
(c) — a plan-reviewed, narrowly scoped slice of an approved umbrella. It does **not** fit the narrower wording of
`.claude/skills/elidex-review/axes.md:139`'s exclusion ("scope が単一 invariant-axis 交点に絞られている"), so
PR-1's diff review does not invoke that exclusion. The entry's firing condition — no trace of the split — is unmet
anyway: the umbrella and PR-1's own plan are the trace. **PR-2 is edge-dense too** (R4 MIN): it owns I2's
entry-point half, I4 and I5's entry-point half (and I7 for its tests) — its eighteen pairs × halves are listed in
§2 *Every pair × half*, the entry halves consuming PR-1's module and helper unchanged — so its plan carries its
own pair matrix and plan-review as well (C18). **PR-0**: its one §2 cell is I1 × I6's admitted lines; whether that
is its whole intersection (it also meets `main`'s two legs, the offline regime and A-iii's `mise.toml` reader) is
its plan's call (C11), not asserted here.

**Seams searched, and alternatives rejected** — R§6: the interval-freedom tables before and after PR-1 (only T5's
clauses 2–5 pass, as PR-0; every other candidate fails on `main` or is false before the pin), why draft 2's PR-3
is folded into PR-1 (F5: `python.compile = false` needs the `tomllib` reader, which only PR-1's one leg makes
available, and an interval without I6 is the temporary state *Phase design is not a shortcut license* forbids),
and why draft 1's (c)③ dissolves under an exact pin.

**Why PR-1 is not split further.** Beyond PR-0, every candidate of R§6 fails on `main` or is false before the pin,
and the parts meet in one observable — "which interpreter does the gate run, can anything else be spelled or
installed as it, and is any other version refused with a remedy that works": a pin file without the exact assertion
admits anything; an exact assertion with a two-leg CI job is red on one leg; the readers model the job's shape
and the file that spells the pin; the install settings decide what the spelling can be installed as; the refusal
without the oracle is H6's dead end; and the floor-motivated code is exactly what an exact pin makes unmeasured.
Separating them leaves a state where the pin, the leg, the config and the assertion disagree.

**Why PR-2 is separate from PR-1, and one PR itself.** Separate: a different observable (direct launches, not
the gate), a different population (entry points, not suites), a different cross-lane surface (§8). One PR: its
parts meet in one observable — "which tracked files launch as `python3`, and does each refuse a non-pin before
any import": the predicate without the guards is red on every entry point it derives; the guards without the
predicate are a list; the lower bound without either has nothing to bound. Splitting them leaves one of those
states (R§6's table *after PR-1*).

**The interval between PR-1 and PR-2** (decision 127, G2 (ii)). It is **not** "no regression"; the regression is
bounded to one class.
- **What holds from PR-1 on.** I1 (CI half; T5 whole — clauses 2–5 since PR-0, clause 1), I2's gate half, I3,
  I5's gate half, I6 for what mise installs from the tracked configuration, and I7 for PR-1's tests. On the
  **supported path** — a launch whose running interpreter's version is the pin (H6, §2 I2) — every entry point
  already runs the pin's version, and the gate admits no other version; a gate run on another version is refused
  with the oracle's facts and the generic actions. (Same version, other source: decision 170's limit, §11 — not an interval effect.)
- **What regresses, and only this: CI measurement of direct launches outside the supported path.** On `main` the
  `tools` job measures the entry points' code on two interpreters (`"3.9"` and the newest `"3.x"`); after PR-1 it
  measures one, the pin. An entry point launched outside the supported path — no mise; a session whose PATH puts
  another `python3` ahead of mise's shim (decision 142's per-session lesson, §7.1); by absolute path from a cwd
  without the pin; macOS `/usr/bin/python3` — still runs, unguarded until PR-2, on an interpreter no CI leg
  measures any more. Because the supported path is defined by what runs, not by PATH order, such a session is
  inside this class, not inside "supported". Within the class, the branches PR-1 removes fail loudly rather than
  silently (V5: `re._parser` is missing below 3.11 and the control is red; V6 under outcome (i): an uncaught
  `RuntimeError` on a symlink loop below 3.13); every other behaviour there is *unmeasured*, not known to be
  wrong.
- **Not in the regression**: the gate, either caller of the driver, CI's spelling, provisioning, and anything on
  the supported path.
- **What else runs in the interval** (decision 150 (3)): #514's re-derive (memos; launches nothing); the M4-12
  umbrella's pre-research (a memo); and possibly sweep 4 (the driver and its pins — the gate, measured on the pin,
  inside the supported path), and the two `mise.toml` / `ci.yml` items above if they land there. None of them
  adds an entry point or launches one off the supported path, so none widens the class. **This is measured over
  the CI/tools lane's queue only** (R5 MIN): other lanes' PRs in the interval are not enumerated here; one that
  adds a tracked entry point adds it to the class until PR-2, whose predicate derives the population at its own
  base and guards it (PR-2 §11 also runs the predicate on each open PR's head). The cargo-deny PR is not in the
  interval (it follows PR-2, above).
- **Bounded by order, not by date.** PR-2's plan passes `/elidex-plan-review` before PR-1 lands, and PR-2 is the
  lane's next pin PR after PR-1. No date is set (R4 MIN; decision 127 bounded the interval by order, and a date
  would be a second bound nobody decided); the PM report on PR-1's landing states the interval as open until PR-2
  lands, and PR-2's landing report closes it (§9).
- **What #514 cites in the interval**: PR-1's artifacts only (§5.1's ledger items, §8's #514 row); what PR-2
  changes afterwards is listed there and in §13 C13.

**Base case.** Each per-PR plan is a narrowly-scoped slice under this umbrella; once the umbrella and the slice
pass `/elidex-plan-review`, the slice is the terminal unit (CLAUDE.md *Edge-dense work* (c)); for PR-1 and PR-2
see above.

**1000-line rule.** At touch time each PR measures every file in its edit set and splits first (standalone
prereq) any file the edit would bring past 1000 lines or that already exceeds it, by cohesion:

```sh
git ls-files -z -- <the PR's edit set> | xargs -0 wc -l | sort -n | tail -n 12
```

The family's own bound also applies: the plan-memo self-test's `line_bound_control`
(`plan_memo_selftest_properties.py:688`) holds every family source under 1000 lines on every run, and §5.2's
edits remove lines. PR-0 and PR-1 both grow `scripts/python_suites_readers.py` (678 lines at `ee6c7a1a`) and
`scripts/test_python_suites_ci.py` (378); PR-1's `mise.toml` half shrinks the former (the text grammar retires).
`scripts/test_python_suites_regime.py` (853) is not in any PR's edit set (no construct it holds names the
interpreter: `git grep -nE '3\.9|"3\.x"|matrix|setup-python|python-version' -- scripts/test_python_suites_regime.py`
prints nothing); if an implementation finds it must edit it, its G7/G11 fixtures are the seam for the prereq
split — and sweep 4, which does edit it, applies the rule itself (§10).
**One stated exception** (R4 MIN): #510's memo, `docs/plans/2026-08-plan-memo-umbrella-checker.md`, is 1707 lines
at `ee6c7a1a` and PR-1 amends it in place (§5.1, V12, V13, V15, V18). PR-1 does not split it first: it is a landed
record owned by the VM P4 lane, PR-1's edits are four dated amendment markers that replace text rather than grow
it, and where that memo's cohesion seams lie is the owning lane's call — #506's split is that lane's
restructuring of its family (§5.3). The request goes to the VM P4 lane through the PM with the §5.2 consent; if
the lane wants a split first, it lands as that lane's prereq before PR-1.

**This umbrella's own length — the seam set, decided now** (K3, decision 205; it replaces draft 6's "split at
landing"). PR-1 lands this draft as four files, as written — nothing is restructured at landing:

| landing file | sections | its reader | lines in draft 8 |
|---|---|---|---|
| `docs/plans/2026-10-python-pin-umbrella.md` (the body) | §0–§11 | each PR's implementer, the PM | 999 |
| `…-python-pin-umbrella-vm-p4.md` | §5.2, §5.3 | the VM P4 lane: its files and lines, the consent subject | 167 |
| `…-python-pin-umbrella-records.md` | R§0, R§4.1, R§4.2, R§6, R§7.2, R§11 | whoever re-opens a decision: a bump, a re-measurement, a reviewer | 493 |
| `…-python-pin-umbrella-rounds.md` | §12, §13, §14 | the next round's reviewer; append-only (#510's `…-checker-rounds.md` precedent) | 585 |

Counts: `awk '/^# Landing file 2/{exit} 1' <memo> | wc -l` for the body, and `awk '/^# Landing file N/{f=1}
/^# Landing file N+1/{f=0} f' <memo> | wc -l` for companion N (marker lines included). Why these seams: each
moved block has a reader the body does not, and the body cites it by number and never restates it — the records
are evidence (dated rows with their commands; a bump appends), the VM P4 part is another lane's consent subject,
the rounds part is history. Draft 6's single seam (§5.2–§5.3) could not reach 1000 lines (R6 K3). Section numbers
are kept (§5.2, §5.3, §12–§14) or prefixed (R§n supports body §n), so §12–§13's cross-references resolve as
written. A later draft that grows the body past 1000 lines moves material along these seams in that draft. Each
file joins `symbol_attribution_control`'s corpus (§5.2 last paragraph, O8); the pointers between them are
backticked paths, which add no link edge for the plan-memo checker.

---

## §7 Who runs what, after

### §7.1 Developers and sessions

| Situation | `main` today | After PR-1 / PR-2 |
|---|---|---|
| `mise run tools-test`, `mise run ci` | PATH `python3` (here the global 3.14.7) | mise's install of the pin; `mise run` auto-installs it if missing (scratch dir with `.tool-versions` + a task running `python3 -VV`, empty `MISE_DATA_DIR`: the task installs, then prints the pin — measured 2026-10-26 with 3.15.1); the driver warns when an install-override variable is set (§4.2). **Unless** a mise config adds a PATH entry (`[env] _.path`) holding another `python3`: `mise run` then runs that one (measured, §11 T1) — refused if its version differs, admitted if it reports the pin (decision 170's limit) |
| a shell whose PATH reaches mise's shim first (or `mise activate`), cwd in the repo, bare `python3` or a shebang tool | the global mise python | the pin (auto-installed on first use), unless an untracked input mise reads first wins on the version — then refused, the refusal printing that file or variable as mise's `source` (§4.2) |
| any of the above in a worktree mise does not trust | mise errors already: `main`'s `mise.toml` holds `[bootstrap.packages]` (#535, `a9866498`), which mise does not read untrusted (rc 1; the same file without that table reads, rc 0 — R§4.2) | the same (PR-1's `[settings]` is a second such table); the refusal prints mise's error lines, mise's own naming `mise trust` (§9 step 4) |
| a shell whose PATH puts another `python3` ahead of mise's shim (Homebrew, `/usr/bin`, an install dir a session captured at its start) | that `python3` | refused unless its version is the pin — by the driver from PR-1, by each entry point from PR-2; the refusal prints that interpreter's `sys.executable`, mise's answer and the generic actions, among them `mise -C <repo> exec -- <tool>` (§4.2). A same-version build there is admitted (decision 170, §11) |
| repo tool by absolute path, cwd **outside** the repo | the cwd's python | refused by PR-2 unless that python equals the pin; the refusal prints the cwd and the root, and the generic actions name `mise -C <repo> exec -- <tool>` |
| no mise; macOS `/usr/bin/python3` (3.9) or Homebrew | runs | the driver and every entry point refuse it, printing the pin, `sys.executable`, `.tool-versions`, that mise is not on PATH, and the generic actions, the first of which names installing mise (R6 M2, §4.2) |

**The supported path is what runs, not what PATH says** (H6, §2 I2): a launch is supported iff its running
interpreter's version is the pin (which build, decision 170 leaves to §11). "mise installed, the repo's tools installed" — already true of `mise run ci` — is the
way to get there; README / CONTRIBUTING state both (PR-1). PATH order differs per session (decision 142: in the L3
session `~/.cargo/bin` preceded mise's shims, so bare `cargo` skipped mise and kept a stale `RUSTUP_TOOLCHAIN`;
`reference_session-rustup-toolchain-env-stale.md`). For `python3` in this (CI/tools) session, 2026-10-31,
`which -a python3` lists mise's shim first, then `/opt/homebrew/bin/python3`, `/usr/bin/python3` and
`~/.local/share/mise/installs/python/latest/bin/python3`; the other lanes' sessions are unmeasured — the PM asks
each lane for `which -a python3` with PR-1's landing notice (§9 step 4, §11). Nothing in this program depends on
the answer for the guarantee: a session that puts another `python3` of **another version** first is refused with
the facts and the generic actions, not admitted; one of the pin's version is admitted, as §11 states.

### §7.2 The user's global hooks (measured end to end, PM decision 41 (d))

The three hooks in `~/.claude/settings.json` (`usage-pace.py agent-hook` / `prompt-hook`, 5 s;
`announced-next-step-gate.py`, 10 s) and the status line's `timeout 2 python3 usage-pace.py record` are
`#!/usr/bin/env python3`, so `python3` is mise's shim and resolves the session cwd's pin. Measured 2026-10-26 with
3.15.1 (mise version not recorded, §11); method and detail: R§7.2. Results:
1. **Auto-install passes attestation** at the first hook invocation in a pinned cwd.
2. **Before the pin is installed the three hooks run and give the right result**, each within its timeout; no hang.
   The install is a download, so the margin depends on the network.
3. **The status line's 2 s cap kills the install** (exit 124): fail-open, the record is skipped, and the next
   invocation repairs the empty install dir. **Accepted (PM decisions 41 / 62)**: first run after the pin only.
4. **Installing the pin moves the user's global `python = "latest"`** to the pin in every cwd — **a global
   effect**; the PM takes it to the user (§9 step 4).
Verdict: the hooks neither hang nor fail on the supported path; the only failure window is the first invocation on
a slow network, which the landing step (§9 step 4) closes.

---

## §8 Coordination

| Lane | State at `ee6c7a1a` | Overlap | Ordering |
|---|---|---|---|
| **VM P4 — #510** (plan-memo checker) | **landed** `282f04d6` (2026-10-26) | PR-1: §5.2's files and lines (with the lane's consent, through the PM), the manifest regeneration, #510's memo amendments (§5.1, V15, V18), V22 (`plan_memo_sibling.py:158` and its two mutation rows); the landing memos join `symbol_attribution_control`'s corpus (§5.2 last paragraph, O8). PR-2: `plan-memo-umbrella-check.py` is an entry point on `main` (executable, `__main__` guard); the `plan_memo_*.py` modules, many with a `#!/usr/bin/env python3` line but no executable mode and no guard, are **not** entry points under I4 (shebangs are not launch evidence) and must be reachable (PR-2 §4.2), several by literal name through `importlib` / `__import__` (`git grep -nE 'importlib\|__import__' ee6c7a1a -- '.claude/tools/*.py'`) | satisfied |
| **VM P4 — small checker-family PRs outside #506** | #562 (S0) landed `e44ffce6`; #569 landed `07b5dea7` (moved `plan_memo_population.py`'s docstring, not V11's lines); more may come | the same files as §5.2 and the manifest | textual: whichever lands second re-runs §5.2's derivation at its base and regenerates the manifest — after PR-1, on the pin |
| **VM P4 — #506** (umbrella plan and its split) | **landed** `3ec0ae5c` (2026-11-01 JST; last PR head `69e2a3aa`; §0) | §5.3, the landed state: V14 on `CLAUDE.md:54` (a plain edit), V16–V21 and V23 (#506's split plan and slot ledger, #510's memo), the checker run CLAUDE.md:55 asks for after a family-memo edit (§9 step 3), the timeout re-derive that first measures #506's guard on the pin, the guard as a suite member, the manifest, the attribution control's corpus; for PR-2, P1 (green once guarded) and P2 (in PR-2's edit set). #506 touched `.claude/skills/elidex-review/axes.md` but no `.claude/tools/**` (§5.3) | PR-1 and PR-2 edit #506's lines with the VM P4 lane's consent — one request for V16–V23 and P2 (decisions 205, 211, 213, §9 step 3); no absorption step |
| **#501** (`webref-cite-audit-tool`, Slice A-i) | **landed** `74bc8acf` | PR-1: none beyond `main`'s text. PR-2: `.claude/tools/webref` and `preflight.py` get the guard prelude; the shim's prelude must spell no `.claude/(skills\|tools)/<a>/<b>` string (K2, `docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md` §2, enforced over `_webref/` plus the `webref` entry script by `.claude/tools/webref-generic-core-trip-wire.sh`) — the prelude finds the root by its entry file's fixed tracked depth (§4.3, H3), a count of parent hops, and spells no path | satisfied |
| **#514** (Slice B / A-ii memos, `citation-hygiene-slice-memos`) | open, **base = `main`** (`gh pr view 514 --json baseRefName`), head `cfcb4ae6` | wording only: B's floor raise and F1, A-ii's UNCHECKED floor line, A-ii's "creates `test_preflight.py`" statements where they rest on F1, the umbrella's A-iii statements. The list is **derived from #514's own diff**, not stored: `git diff origin/main...origin/citation-hygiene-slice-memos -- 'docs/plans/2026-07-citation-hygiene-*' \| grep -nE '^\+.*(F1\|test_preflight)'` and the same diff through `grep -nE '^\+.*(3\.9\|3\.11\|\(3, ?9\)\|floor\|[^A-Za-z]legs?([^A-Za-z]\|$)\|matrix\|setup-python\|"3\.x"\|python-version)'`, each hit classified by PR-1 §4.6's classes. **During the PR-1 → PR-2 interval #514 cites only PR-1's artifacts** (§5.1): B's F1 is withdrawn and its direct-launch half is "UNCHECKED, assigned to PR-2 of the python-pin umbrella" through the landed umbrella; A-ii keeps creating `test_preflight.py` for its own rows if it still has them. **After PR-2**, F1's successor is PR-2's E1 in `scripts/test_python_entry_points.py`, and PR-2 (or #514's next rework, if #514 has not landed) flips the UNCHECKED lines (§13 C13) | **decided by decision 124**: #514's re-derive starts right after PR-1 lands, before PR-2, and re-derives once against the pinned world |
| **Dependabot** (#381 `actions/checkout` 6 → 7, #502 `taiki-e/install-action`, cargo bumps) | open | `ci.yml` `steps:` only | textual |
| **stale-claim lane** (parked; `refs/backup/stale-claim-detector`) | parked | its `claim-gate-plan-check.py` would be an entry point under I4 | whoever revives it adds the guard; handed over via the PM ledger |
| **every lane — tracked mise files and `.tool-versions`** | — | T5 (§4.2): from **PR-0**, a tracked `.miserc*`, a non-root `.tool-versions`, a `.python-version*`, or a `python` outside the admitted occurrences in any file whose path has a component starting `mise` / `rtx`, is red; from **PR-1**, the root `.tool-versions` is exactly the one `python` line, so another tool goes into `mise.toml [tools]`. A `[tools]` entry for a non-Python tool is **green** (draft 4's T5b made it red) | PM notice with PR-0's landing (§9 step 2) and with PR-1's (§9 step 3) |
| **CI/tools — cargo-deny PR** (decision 76: pin cargo-deny in `ci.yml`'s `deny` job and `audit.yml`, `deny.toml [graph] all-features`, no rust-cache in `deny`) | not started | `ci.yml` (another job than `tools`), the same file PR-1 and PR-2 edit | **after PR-2** (decision 150 (3): PR-2 edits `ci.yml`, §6); before the M4-12 program (decision 118) |
| **CI/tools — M4-12 exit-gate program** (decision 118: security → WPT → 100 sites, one umbrella) | not started | new CI jobs and gate docs; none of this program's files known yet | after PR-2 and the cargo-deny PR; its umbrella pre-research may start after PR-1 (decision 150 (3)); before the K2 wire program (decision 118) |
| **CI/tools — A-iii sweep 4** (K16, K27, K28, K29, K13, the spawn false red, G13) | not started | `scripts/python-suites.sh` (textual with PR-1's Step 1), `scripts/test_python_suites_regime.py` (853 lines, §6's split rule), A-iii's memo; no `ci.yml` item | anywhere (decision 150 (3)); if its plan finds a `ci.yml` edit, the cargo-deny PR's rule |

The lane facts are derived: before each PR lands, its driver re-runs A-iii §13's sweep with that PR's contended
paths added (`.tool-versions`, `scripts/python_pin.py`, `mise.toml`, `scripts/python_suites_readers.py`, §5.2's
files, and for PR-2 every derived entry point); for PR-0 and PR-1 the sweep also runs T5 on each open PR's head
(a red there gets its hand-off before landing, as PR-2 §11 does for the predicate).

---

## §9 Landing steps (umbrella-level)

1. Each PR passes `/elidex-plan-review` on its per-PR plan before implementation; the umbrella first (R5); PR-2's
   plan before PR-1 lands (§6, the interval's bound).
2. **PR-0** lands (no cross-lane precondition). PM notice to every lane with it (§8 row *every lane*; R4 MIN — a new
   cross-lane red needs its notice): a tracked `.miserc*`, a non-root `.tool-versions`, a `.python-version*`, or
   a `python` outside the admitted occurrences in any mise-named file is red from now on.
3. **PR-1's preconditions and landing report.** Before implementation: the VM P4 lane's consent to §5.2 V1–V23
   and P2 and its answer on V6, obtained by the PM with `…-umbrella-vm-p4.md` (§5.2–§5.3) attached (decision 127
   (a)). V1–V15 went with drafts 4–6; **V16–V23 and P2 go in one request, sent once the umbrella converges**
   (decisions 205, 211, 213; V22 names the lane's option to fix it itself), together with two couplings the lane
   consents to as well: §5.3 O4 (V6 and V22 sit in the guard's import closure) and **PR-1's checker run** — PR-1
   edits a VM-P4 family memo (V19–V21, V23), so it runs `python3 .claude/tools/plan-memo-umbrella-check.py
   docs/plans/2026-07-vm-p4-es-language-completeness.md` on the pin and expects exit 0 (CLAUDE.md:55; exit 0 at
   `ee6c7a1a` on 3.15.2 and 3.9.6, R§0); a 1 or 2 is fixed in PR-1's edit before landing. The landing report to the PM: §5's amendments (A-iii D3 / §8 / §12(1) / §6's UNCHECKED line, #510's memo
   §1 / §8 / §3 row, and the ledger); §5.2's VM P4 edits (V1–V23) and the regenerated manifest; the notice to every
   lane (§8 row *every lane*: the root `.tool-versions` is one line); the slots in
   PR-1 §9 to register; the interval as open (§6); and the **re-evaluation of `#11-aiii-gate-tamper-hardening`**,
   whose trigger (b) PR-1 fires twice — it touches the `tools` job's step composition (the `strategy` block and
   the setup step's input) and mise `[settings]` / how `mise.toml` is read. Quoted from the slot's ledger row
   (`project_open-defer-slots.md`; R5 MIN — draft 5 quoted a question the row does not hold): item (1) "other
   steps in the `tools` job (BASH_ENV written to GITHUB_ENV, overwriting the driver, `git rm --cached`, a second
   setup-python, checkout `ref: main`)"; item (3) "mise `[settings]`/`[env]` (`task.skip`, `[env] BASH_ENV`,
   `_.path`, `PYTHONOPTIMIZE`)". Relation to item (1)'s "a second setup-python": PR-1's N4 makes a second
   `actions/setup-python` step red as part of I1 × I3 (a second spelling), whatever the intent, so that sub-item
   leaves the slot's list. Relation to item (3): T5's clause 4 (PR-0) makes any `python` in a tracked mise file red
   except the two admitted lines — and so, **incidentally**, a tracked `[env] PYTHONOPTIMIZE` (its name contains
   `python`); that is a side effect of the spelling rule, not a tamper check, and the slot decides whether that
   sub-item leaves its list. `_.path` stays the slot's — it is also decision 170's same-version limit (§11 T1) —
   and so do `task.skip`, `BASH_ENV` and deliberate obfuscation (e.g. TOML escapes, §4.2 residual); its other
   items stay the slot's.
4. **PR-1's landing step — through the PM, to every lane: run `mise install` in each worktree after rebasing onto
   PR-1's `main`** — first `mise trust` in that worktree if mise reports it untrusted (`main`'s `mise.toml` already
   holds `[bootstrap.packages]`, #535, and PR-1 adds `[settings]`; mise reads neither untrusted, R§4.2 table) — and report `which -a python3` from that lane's
   session (H6, decision 142; §7.1, §11). With
   both install settings already in that `main`, the install is attested and precompiled or fails; it closes the
   first-use window of §7.2 (3) and — globally — moves the user's `python = "latest"` to the pin (§7.2 (4)); the
   PM tells the user that before the broadcast.
5. **The interval** (§6): #514's re-derive starts (decision 124), citing PR-1's artifacts only (§5.1, §8); the
   M4-12 umbrella's pre-research may start; sweep 4 may land (it may also land earlier or later).
6. **PR-2** lands. Its landing report: the derived entry-point set (PR body), the UNCHECKED → CHECKED lines (§13
   C13), the **close** of the PR-1 → PR-2 interval (§6), the P2 edits made under the consent of step 3 (§5.3 (b)),
   and the stale-claim hand-off.
7. **The cargo-deny PR** (after PR-2, because PR-2 edits `ci.yml`; §6). Then the **M4-12 exit-gate program**
   (decisions 118, 150 (3)).
8. **No post-#506 step remains** (R6 K1). Draft 6 put a manual re-run of §5.2's stage 1 here, triggered by #506's
   merge; #506 merged before PR-1 (§0), so PR-1's own derivation at its base, which now names #506's files (§5.2),
   is that reading, with V16–V21 and V23 as its result.

---

## §10 Defer slots

The umbrella holds none of its own; each PR lists its own (≤ 3, with Why / trigger / date): PR-0 §9 (none),
PR-1 §9 (two: `#11-toolchain-pin-bump-automation`, `#11-python-pin-single-builder`), PR-2 §9 (none). Decision
170's same-version limit is **not** a slot: it is a narrowed guarantee, recorded in §11, with no trigger to wait
for (the user chose not to widen the CI rule). **Routed, not deferred here**: A-iii's G13 pin goes red when `TMPDIR` is long
enough to push its `AF_UNIX` path past the macOS `sun_path` limit (`OSError: AF_UNIX path too long`;
`t=$(mktemp -d)/$(printf '%0100d' 0); mkdir -p "$t"; TMPDIR=$t bash scripts/python-suites.sh` names
`test_G13_af_unix_goes_through_and_af_inet_stays_denied`; green with the default `TMPDIR`). It is an A-iii pin
defect independent of the pin, owned by the CI/tools lane; the PM records it in the lane ledger; its fix is
planned for A-iii sweep 4 (placed by §6's *Ordering*: anywhere), which touches `test_python_suites_regime.py`, so
§6's split rule applies there.

---

## §11 UNCHECKED, marked not omitted

- Linux `mise install python@3.15.2` — unmeasured (R§4.1 row 1b gives the recipe); first measured by a Linux developer.
  **Accepted as unmeasured (PM decisions 41 / 62).** PR-1's CI does not measure it: the `tools` job installs
  setup-python's manifest build, not mise's (PR-1 §N4). So on Linux, the only interpreter that CI measures is that
  build (next bullet). Running `mise install` in CI would put a third-party action in the gate's trust path, the
  option rejected as S3 (§4.2), and that rejection stands.
- setup-python's manifest build of 3.15.2 on the runner — measured only by PR-1's own CI run.
- That the local and CI builds behave alike — same release, different builders (`#11-python-pin-single-builder`).
- The pin's age against new CPython security releases — no mechanical alarm (`#11-toolchain-pin-bump-automation`).
- **The refusal on a real non-pin interpreter** (R4 MIN; owner: the CI/tools lane). After PR-1 no CI leg runs a
  non-pin interpreter, so CI executes the refusal only by patching `sys.version_info` inside the pin (I3 × I5) —
  which proves its logic and printed facts, not that an old interpreter compiles `python_pin.py` and reaches the
  refusal (N3 stays best-effort). Measured once: PR-1 runs the driver's Step 1 under macOS `/usr/bin/python3`
  (3.9) — **with I7's isolated mise environment**, since that run reaches the oracle — and records the refusal in
  its PR body. The environment is R§11's *real old interpreter* recipe (R6 M4): the isolated dirs **plus**
  `MISE_TRUSTED_CONFIG_PATHS=<the worktree>`, because with empty isolated state the tracked `mise.toml` is
  untrusted (rc 1 — `main`'s `[bootstrap.packages]` already, R§4.2) and only "mise gave no answer" could be
  recorded; with it, mise resolves the pin, not installed (isolated data dir). Re-measured whenever
  `python_pin.py` changes, and that is **a pin, not a header rule** (J2,
  §2 I1 × I5): the module's recorded SHA-256, with the date and interpreter of the run, is held by a control that
  is red when the module changes without a new record; `ast.parse(…, feature_version=(3, 9))` bounds the grammar
  between runs. **Both controls have a stated limit** (R6 M1): (1) `feature_version` is best-effort — on the pin,
  3.15.2, it accepts seven constructs a real 3.9.6 rejects (R§11's accepted rows, measured on both 2026-11-01), so
  a module using one passes it and fails only on the real old interpreter; (2) the SHA-256 control forces a **new
  record**, not the run: an editor can write the record without running `/usr/bin/python3`. The run itself is
  enforced by PR review and the landing checklist (C20), not mechanically (mechanising it on CI's own
  `/usr/bin/python3`: considered and not taken, R§11 *The real old interpreter*). Between edits, not re-measured.
- **Untracked overrides of the install settings — a stated limit (G5; decision 150 (4)).**
  `MISE_PYTHON_COMPILE=1`, `MISE_PYTHON_GITHUB_ATTESTATIONS=false`, and a `mise.local.toml` (or any untracked
  config mise ranks above the tracked `mise.toml`) whose `[settings]` override either key make mise compile from
  source or install unverified, and admission still accepts the result, because it compares the version only and
  an installed interpreter carries no record of how its archive was verified or built (§2 I4 × I6). The driver
  and the refusal path **warn** while one of the variables (or the global `MISE_GITHUB_ATTESTATIONS`) is set in
  the run's environment (§4.2); an install made earlier under an override, and an override through an untracked
  file, are invisible to that warning, and CI cannot see either (no CI red). Measured: the compile overrides (env
  and `mise.local.toml`, R3) and the attestation env override (2026-10-31, R§4.2); the attestation override
  through `mise.local.toml` is unmeasured and assumed to follow the same config precedence. The global
  `MISE_GITHUB_ATTESTATIONS=false` does **not** override the tracked python-specific key (measured, R§4.2). A
  *tracked* `mise.local.toml` that sets either key is red by T5's clause 4.
- **T5's residual** (§4.2): an input mise reads outside T5's names, or a Python spelling without the letters
  `python` (e.g. a future idiomatic file; a TOML escape) — not silent **when it changes the version**, since
  admission refuses what mise then launches and the oracle names it; what it loses is the CI red. When it leaves
  the version at the pin, it is the next three bullets' limit (decision 170). (`[env] _.file` and an rc-only
  setting in `mise.toml` were measured benign in 2026.10.21, R§4.2.)
- **Decision 170's limit — same version, different source (stated, not guaranteed).** Admission compares the
  version; nothing in the program checks **which build of the pin's version** runs, or **which file** mise read
  it from. Each case below is admitted when the version is the pin, and refused (not silent) when it is not. The
  CI rule (§4.2 clauses 1–5) is not widened for them (the user's decision).
- **T1 — `[env] _.path` (R5 A2).** A tracked `mise.toml` with `[env] _.path = ["altbin"]` holds no `python`
  text, so T5's clause 4 and CI are green; `mise run <task>` and `mise exec -- python3` then run
  `altbin/python3`, while the oracle reports the root `.tool-versions` and `mise which python3` the install — and
  bare `python3` through mise's shim still runs the install. (a) **Admitted** if `altbin/python3` reports the pin
  (a Homebrew or source build of the same patch: not attested, not compile-off — I6 does not cover it). (b)
  **Refused** otherwise; the refusal prints `sys.executable` (`<P>/altbin/python3`) beside `mise which python3`
  (the install) and mise's `source` (the root `.tool-versions`), and the generic block's fourth action is the fix
  (§4.2). Which config sets the entry is not printed — the oracle does not report it, and draft 7's `mise env
  --json` read is withdrawn (PM decision 217); `mise config ls` lists the candidates, and `mise exec` does not
  help. The same holds for an untracked
  `mise.local.toml` or global config. The tamper slot `#11-aiii-gate-tamper-hardening` already lists `_.path`
  (item (3), §9 step 3). Recipe (mise 2026.10.21 and 2026.10.25, nothing downloaded): R§11 *T1*.
- **T2 — an rc file renames the version file (R5 A4).** With `.miserc.toml` `override_tool_versions_filenames =
  [".alt-versions"]`, the oracle's `source` is `{"type": ".tool-versions", "path": "…/.alt-versions"}`: the
  substitute file, never the rc file. (a) **Admitted** if `.alt-versions` holds the pin (mise reads another file
  than the one the pin's readers read — silent until the pin is bumped, when the version differs and the refusal
  fires). (b) **Refused** otherwise, printing `.alt-versions` as mise's `source`; and if the substitute file is
  deleted, mise resolves `[]` — not the root `.tool-versions` — which the refusal prints as such. In both the rc
  file itself is not printed: the oracle does not report it (the generic block names `.miserc*`). The generic
  "remove or change" includes writing the pin into `.alt-versions`, which moves the case into (a); the refusal
  does not tell the two apart (PM decision 217 withdrew R6 M6's cause-specific "never write the pin"). A
  **tracked** `.miserc*` is red by T5's clause 3; an untracked one (in the worktree or above it) is this limit.
  Recipe, with the benign pair of §4.2: R§11 *T2*.
- **Any other same-version `python3`** ahead of mise's shim (Homebrew's or a source build of the pin's patch) is
  admitted by the same rule (§7.1 row 4). No event re-checks it
  against new mise releases: the repository does not pin mise (R4 MIN — draft 4's "re-checked at each mise bump"
  had no bump to attach to), and T5 does not depend on mise's list, so there is nothing to re-derive. **Why mise
  is not pinned (R7 A3-4)**: no guarantee here rests on mise's version — admission compares `sys.version_info`
  with the pin and never asks mise, CI does not run mise, and a changed mise can only change the printed facts
  (next bullet). Trigger to revisit: any change that makes a mise answer decide an exit status.
- **The oracle depends on mise's output shape**, which is mise's and unpinned. A changed shape, a missing mise or
  a hang degrades the refusal to "mise did not answer" plus mise's stderr lines and the generic block; it
  never changes the exit status (§4.2). The pin holds fixtures of the 2026.10.21 shapes, so a change shows locally
  as the degraded message, not
  as a CI red.
- **Per-session PATH** (H6, decision 142): measured for this session only (§7.1); the other lanes report
  `which -a python3` with PR-1's landing notice (§9 step 4). The program does not depend on the answer.
- A direct run of a set member (`python3 scripts/test_….py`) bypasses the driver's admission (PR-2 §4.4); a
  **tracked** instruction to do so is red (I4's bound on X1), an untracked one is outside the population. At
  `ee6c7a1a` there are three (`git grep -nP 'python3?(\.[0-9]+)? +(-[A-Za-z]+ +)*(-m +unittest +)?(\S*/)?test_[A-Za-z0-9_]+\.py'
  ee6c7a1a -- ':!crates'`): C6's `test_grep_pass.py:5` and #506's two (§5.3 P2), all in PR-2's edit set; PR-2's
  R-b scope includes landed records (PR-2 §4.2: "every tracked text file"). **This program's landing files add
  none** (R7 J5): draft 7 quoted P2's and C6's lines in that shape (two more at PR-2's base); draft 8 describes
  them, and the pattern over the memo through Python's `re` prints 0 hits (R§0; each PR re-checks, C33).
- **Inline Python in tracked text** (`python3 -c '…'`, `python3 - <<'PY'`) runs on whatever `python3` the reader
  has; it launches no tracked file, so no guard can sit in front of it and R-b ignores it (PR-2 E4). Today's
  instances are reproduction recipes in comments and docstrings of the plan-memo family, plus the driver's Step 1
  (`scripts/python-suites.sh:163`, replaced by `python_pin.py` in PR-1); the driver's later inline Python runs on
  the interpreter Step 1 admitted
  (`git grep -nE "python3?( -[A-Za-z]+)* +(-c|-)( |$)" ee6c7a1a -- ':!docs/plans/**' ':!docs/audits/**'`). A stated limit:
  such a recipe's result is not guaranteed to be the pin's.
- The mise version of draft 2's 2026-10-26 measurements (§7.1 row 1, §7.2) was not recorded; PR-1's landing step
  re-runs §7.2 (1)–(2) under the then-current mise and names it, and re-runs `usage-pace.test.py` on 3.15.2
  (§7.2 (4); C20).
- #510's `_resolve` `RuntimeError` arm (§5.2 V6): measured unreachable for a symlink loop on 3.15.2 / macOS
  (R§4.1 row 3x). Whether any other `RuntimeError` (e.g. a `RecursionError`) can leave `Path.resolve()` on the pin
  is the VM P4 lane's call; **both outcomes are stated** under §5.2's table — (i) remove the arm with V7–V11 and
  V15, (ii) keep it re-justified on the pin, keep V7–V8, restate V9–V11 and V15. PR-1's implementation waits for
  the answer through the PM.
- Whether `git ls-files` under A-iii's offline regime is available to a suite member on both of `main`'s legs —
  PR-0's plan confirms (the driver itself runs `git ls-files`, `scripts/python-suites.sh:435`).
- Whether A-iii's existing `mise.toml` reader pins the `tools-test` description's text, which PR-0 rewords
  (`mise.toml:115`) — PR-0's plan reads the reader and its pins first (C11).

---
