# Landing file 3 — `docs/plans/2026-10-python-pin-umbrella-records.md`

The measurements and rejected alternatives behind the python-pin umbrella's decisions (body:
`docs/plans/2026-10-python-pin-umbrella.md`). Section R§n supports body §n; each row carries its date, its mise
version where mise ran, and the command that reproduces it. A bump appends here; the body is not re-measured.
A plain §n is the body's section; §12–§14 are the rounds record's.

## R§0 Bases, decisions, probe record

**Decisions — the full text** (record locations as in body §0; the body lists them in short):

- **PM decision 41** (2026-10-26, `PORTFOLIO:59`): (a) the line is 3.15, aligned with Rust's latest-stable pin
  (ADR #36), provided mise, setup-python and every suite hold on it, else 3.14 — pinned to an exact patch (§4.1);
  (b) A-iii's D3, §8 and §12(1) are amended (§5); (c) an umbrella and per-PR plans (§6), with the entry-point
  population derived from a property, not from a shebang grep; (d) the global hooks are measured end to end
  (§7.2).
- **PM decision 62** (2026-10-28, `PORTFOLIO:149`, under "## 2026-10-28 (PM pass)"): Linux `mise install` is
  accepted unmeasured (only the setup-python build is measured on Linux); S3 stays rejected (§4.2, §11). With
  decision 41 it is also the ground of §7.2 (3) and §11's Linux bullet, which stay as accepted.
- **PM decision 117** (2026-10-31, `PORTFOLIO:217`): (1) the pin is **3.15.2** (§4.1, a correction of the 3.15.1
  record); (2) #510's floor-motivated code branches and two-leg statements, and #510's §8 timeout rule, go into
  §5's amendment table, with every VM P4 file and line PR-1 touches and its manifest impact listed, for the PM to
  obtain the VM P4 lane's consent (§5.2); (3) PR-1 ordered after the #506 split — **withdrawn by decision 127**.
  The approval of the R2 lens dispositions F5–F9 is in the PM's message carrying decision 117 (cited as "PM
  message, decision 117"; also in the lane SSoT's top block). Folding R2's MINs was draft 3's own processing.
- **PM decision 124** (2026-10-31, `PORTFOLIO:228`; lane SSoT top block): draft 2's PR-3 folded into PR-1 (two
  PRs, accepted); **#514** (citation-hygiene slice memos A-ii/B/C) is ordered **after PR-1**, re-derives once
  against the pinned world, and **starts right after PR-1 lands**; the §5.2 table goes to the VM P4 lane after R3.
- **PM decision 127** (2026-10-31, `PORTFOLIO:232`; lane SSoT top block; PM message): **G1** — decision 117's
  "PR-1 after the #506 split" is withdrawn: the split reaches `main` only with #506's merge, which is at converge
  R95+ with no known end (the record), so waiting would chain PR-1 and the M4-12 exit-gate program (decision 118)
  to #506's converge; PR-1 goes first and #506 absorbs it. Conditions: (a) the VM P4 lane consents to PR-1
  touching its files — the record names "the V1–V14 table from draft 4"; V15 (draft 4), V16–V18 (draft 7,
  decision 205), V19–V22 (decision 211) and V23 (decision 213) are handed over with the rest; (b) the memo lists every line PR-1 and #506 both touch, with the
  absorption step. **(b) is moot since #506 landed first** (`3ec0ae5c`, §0): §5.3 states the landed lines with
  one owner each, and (a) stands. **G2** = option (ii): decision 124's order stands (#514's re-derive
  between PR-1 and PR-2); §6 states what holds and what regresses in the PR-1 → PR-2 interval, the regression
  bounded to *CI measurement of direct launches outside the supported path*; #514 cites only PR-1's artifacts.
  **G3–G6**: the lens dispositions of `r3-disposition.md` are accepted.
- **PM decision 141** (2026-10-31, `PORTFOLIO:244`; lane SSoT top block): draft 4's PR-0 accepted and the order
  PR-0 → PR-1 → #514's re-derive → PR-2; §5.2 and §5.3 go to the PM after R4, for the VM P4 lane's consent; R4 to
  consider a reader that turns red when `MISE_PYTHON_GITHUB_ATTESTATIONS` (or the compile override) is set in
  the environment. PR-0's *content* is rebuilt by decision 150 (§6); the order stands. The env-var request is
  answered by decision 150 (4).
- **PM decision 142** (2026-10-31, `PORTFOLIO:245-246`; memory `reference_session-rustup-toolchain-env-stale.md`):
  the session shell keeps the `RUSTUP_TOOLCHAIN` mise exported at session start, and whether bare `cargo` meets
  mise's shim depends on the session's PATH order (measured in the L3 session: `~/.cargo/bin` ahead of mise's
  shims). The ground of §7.1's supported-path definition and §11's per-session PATH bullet (H6). The cause and
  remedy are **PM decision 149** (`PORTFOLIO:250`): the PATH order depends on when the session's parent shell
  started; the fix is restarting old sessions, no dotfiles change.
- **PM decision 150** (2026-10-31, `PORTFOLIO:251`; lane SSoT top block; PM message): (1) **change the tool —
  ask mise, don't model it.** I2 is protected by admission (the running interpreter compared with the pin); T5's
  job is reduced to exactly two things — CI and local must not disagree silently (CI never runs mise), and the
  refusal's remedy is never a dead end. Locally the refusal asks mise what it resolves and from which file; in CI
  one small fail-safe tracked-tree rule (§4.2). PR-0 is rebuilt accordingly (§6). (2) **Terminator for R5**: if
  R5 finds a T5-class IMP in the parts draft 5 adds, the driver sends it to the PM **unfixed** (§12.4). (3)
  **Order (H5)**: the M4-12 exit-gate program after PR-2 (its umbrella pre-research may start after PR-1); the
  cargo-deny PR after PR-1 and before PR-2 unless PR-2 also edits `ci.yml`, then after PR-2; sweep 4 anywhere if
  it does not touch `ci.yml` (§6). (4) **Decision 141's env vars**: a warning suffices — the refusal path and
  `mise run` warn; install-time overrides stay a §11 stated limit; no CI red (§4.2, §11). (5) **H2, H3, H4, H6
  approved** as in `r4-disposition.md` (§2, §4.2, §4.3, §7.1).
  **Departure from the record, ratified (J3).** The record of (1) names the oracle commands `mise current` /
  `mise config ls`. Drafts 5 and 6 use **`mise ls --current --json python`** and, when installed, **`mise which
  python3`** instead, for the measured reason in §4.2: `mise current python` prints the version only, and `mise
  config ls` lists loaded files without a winner and omits both the variable and the rc file. The PM ratified
  the substitution in decision 167.
- **PM decision 154** (2026-10-31, `PORTFOLIO:257`): #506's split approved for push. History: #506 then merged
  as `3ec0ae5c` from head `69e2a3aa` (PORTFOLIO:290, :293; §0) — draft 6's `c032379d` was not its last head.
- **PM decision 167** (2026-10-31, `PORTFOLIO:267`): R5's T1–T3 arrived unfixed under decision 150 (2). **T3**
  (the tests reach the user's real mise state and are not hermetic) is a defect to fix in draft 6 — **I7** (§2)
  with its owners and controls. T1 / T2 narrow a guarantee and go to the user (decision 170). J1–J4 accepted,
  J3's command substitution ratified (above).
- **PM decision 217** (2026-11-01, `PORTFOLIO:354`), R7's root check, option (a): **remove the generator layer.**
  The refusal lists only observed facts — `sys.executable`, mise's chosen config (the oracle's `source`), its
  install path, `mise which python3`, and the pin — and infers no cause. Removed with it: the "which cause" read
  through `mise env --json` (R6 M6), the cause lines per branch and the remedy chosen per cause. The remedy text
  stays never a dead end: the facts plus generic actions (install the pin; remove or change the winning input;
  launch through mise's `exec`), none claimed to apply. **Reason**: that layer produced findings three rounds
  running (R5 T1 / T2, R6 M6, R7 J4 / J6), each a mechanism designed by reasoning and never measured. Decision
  170 narrowed the guarantee to "never silent when the version differs"; this narrows the diagnostic further,
  inside 170; "a version mismatch always stops" is unchanged. **R8's terminator** (as PM decision 215's): R8 IMP
  0 → converged; IMPs clustered in what draft 8 added → no R9, the memo becomes a PR and goes to
  `/external-converge` (the user's N1 precedent, "change the instrument"); any other IMP → the PM (§12.7).

**Draft 7's measurements** (2026-11-01, base `ee6c7a1a`, by the designer; read-only on the repository):
- *Suite census*: `git ls-tree -r --name-only ee6c7a1a | /usr/bin/grep -E '(^|/)test_[^/]*\.py$'` prints ten paths —
  the nine of `b051329b`'s row 3 plus `docs/plans/test_vm_p4_family_layout.py`; `… | /usr/bin/grep -cE '\.py$'` → 89.
- *Compile*: `git archive --format=tar -o tree.tar ee6c7a1a -- .claude docs/plans scripts` (every tracked `.py` is
  under these three; `cut -d/ -f1` of the list: 83 / 1 / 5), extracted to a scratch dir; then each of the 89
  `compile(<text>, <path>, "exec")` under `-I -W error` with Homebrew's `python3.15` (3.15.2) and macOS
  `/usr/bin/python3` (3.9.6): 0 failures on each.
- *The guard*: in that tree, `<python> -I -m unittest discover -s docs/plans -t docs/plans -p
  test_vm_p4_family_layout.py` → `Ran 2 tests … OK` on both interpreters. Under `-W error` 3.9.6 is green and
  3.15.2 errors in `setUpClass` with `DeprecationWarning: 'maxsplit' is passed as positional argument`, from
  `plan_memo_sibling.py:158` (`re.split(r"[#?]", dest, 1)`; positional `maxsplit` is deprecated since 3.13, so
  `main`'s `"3.x"` leg has the same warning today). The driver runs members without `-W error`, so the warning
  does not change any verdict; it is a VM P4 family line, reported to the PM with draft 7 — **V22** since PM
  decision 211 D7-2 (§5.2, with stages 1c / 2c and the keyword form measured on both interpreters).
- *Ledger rows* (V18–V21, §5.2 stage 1b; V19–V21's pointer rows under the heading "VM P4 slot-ledger pointers",
  re-read 2026-11-01 for decision 211; cited by slot ID, not by line — draft 7's line numbers were off by one and
  the live ledger moves, R7 A5-4): `/usr/bin/grep -c '<slot id>' memory/project_open-defer-slots.md` → 1 for
  each of `#11-plan-memo-acceptance-falsifiability-check`, `#11-plan-memo-spec-field-single-home-check`,
  `#11-dead-opcode-removal`, `#11-d17b-dispatch-expr-file-growth`.

**Draft 8's measurements** (2026-11-01, base `ee6c7a1a`, by the designer; the repository read only through `git
show` / `git grep` / `git archive ee6c7a1a`, extracted under the scratchpad; probes isolated by the program's rule):
- *Base drift*: `git diff --stat ee6c7a1a 593aa314` → six files, all `crates/core/elidex-sandbox/**` (#579).
- *J3, V18's origin*: `/usr/bin/grep -n -- '<slot id>' memory/project_open-defer-slots.md` for the two
  `#11-plan-memo-*` slots → one line each, under *Registered in a plan memo (from MEMO rows)*, reading "added to the
  index at #510's landing, 2026-10-26" / "added at #510's landing, 2026-10-26" and "Re-eval: 2027-01-30 (set
  2026-11-01, PM 191)"; `git show -s --format=%ci 282f04d6` → 2026-10-26 22:45 +0900 (#510), `… 3ec0ae5c` →
  2026-11-01 02:35 +0900 (#506).
- *V23*: `git show ee6c7a1a:docs/plans/2026-07-vm-p4-slot-ledger.md | sed -n 327p` holds the sentence quoted in
  §5.2 V23; stage 1b re-run → the same ten hits.
- *J2, the guard's closure*: an `ast` walk from `docs/plans/test_vm_p4_family_layout.py` over the `import` /
  `from … import` names that resolve to `.claude/tools/<name>.py` → the twelve modules of §5.3 O4.
- *J1, the checker*: `python3 -I .claude/tools/plan-memo-umbrella-check.py
  docs/plans/2026-07-vm-p4-es-language-completeness.md` in the extracted tree → rc 0 ("0 mechanical finding(s)
  gate the exit status") on 3.15.2 and on 3.9.6.
- *A3-3, runtime deprecations*: the extracted tree as a scratch git repo, `PATH=<3.15.2's bin>:/usr/bin:/bin
  PYTHONWARNINGS=always::DeprecationWarning bash scripts/python-suites.sh` → `GREEN -- 10 member(s), 120 test(s)
  passed, offline`; 151 `DeprecationWarning` lines, every one `plan_memo_sibling.py:158` (V22). A child started
  with `-I` / `-E` would not see the variable; that part is unmeasured.
- *J5*: §11's X1 pattern through Python's `re` over this memo → draft 7: 2 hits (P2's quote, C6's quote); draft 8: 0.
- *J4*: in the session's shell `type mise` → "a shell function"; `command -v mise` → `mise`; zsh `whence -p mise` →
  `/opt/homebrew/bin/mise`; in bash with `mise` defined as a function, `type -P mise` → `/opt/homebrew/bin/mise`
  while `dirname "$(command -v mise)"` → `.`. `/usr/bin/python3 -I -c 'import sys; print(sys.executable)'` →
  `/Applications/Xcode.app/Contents/Developer/usr/bin/python3` (3.9.6).
- *A4-7 and the post-check* (mise 2026.10.25, `env -i`, `PATH=/usr/bin:/bin:<type -P mise's dir>`): a
  byte-identical copy of `ee6c7a1a:mise.toml` (`cmp`), untrusted → rc 1, stderr "error parsing config file", "Config
  files in … are not trusted.", "Trust them with `mise trust`. …"; the same file without `[bootstrap.packages]` →
  rc 0 (`[]`); with a `.tool-versions` and `MISE_TRUSTED_CONFIG_PATHS=<project>` → 3.15.2, `installed: false`,
  `install_path` under the isolated data dir, and `$ISO/state/tracked-configs` holds links to `<project>/mise.toml`,
  `<project>/.tool-versions` and the isolated global file. User state: `ls -la ~/.local/state/mise/tracked-configs
  | wc -l` → 144 before and after; entries into `scratchpad/d8` → 0.
- *J2 probe*: R§11's table, on Homebrew 3.15.2 and macOS 3.9.6.
- *mise* (2026.10.25), one probe, isolated by the program's rule: `mise env --json` under T1's recipe (R§11 *M6
  read*). Afterwards no entry of the user's `~/.local/state/mise/tracked-configs` or `trusted-configs` resolves
  into the scratch dir (`ls -la … | /usr/bin/grep -c 'scratchpad/d7'` → 0). No before/after listing was taken.
- *`feature_version`*: R§11 *J2 probe*, on 3.15.2.

**Probe isolation, by draft.** Drafts 3–4 isolated `MISE_DATA_DIR` / `MISE_CACHE_DIR` (and, where noted, an empty
`MISE_GLOBAL_CONFIG_FILE`) but not the state dir, and a draft-4 probe wrote trust entries into the user's real
`~/.local/state/mise/trusted-configs` (removed by the CI/tools lane, decision 150's record). Draft 5's probes
(R§4.2) followed the full rule; the listings of the user's `trusted-configs` and `tracked-configs` were identical
before and after. Draft 6's probes (R§4.2's added oracle rows, R§11's T1 / T2 recipes; mise 2026.10.21) added
`MISE_OFFLINE=1`, `MISE_AUTO_INSTALL=false` and fake install directories; the listing of the user's
`~/.local/state/mise` was identical before and after. The 2026-10-26 measurements of R§7.2 recorded no mise version.

**Bases before `ee6c7a1a`** (draft 6's text; its `origin/main` meant `6aea28da`):

**Base**: `origin/main` at `6aea28da` (2026-10-31; #574). Between draft 5's base `1ed126b4`
and `6aea28da` four PRs landed: #572 (`crates/**` only), #571 `e389fdc7` (CLAUDE.md: the tiering block, old
`:57-73`, becomes a two-line pointer), #573 (`.claude/skills/external-{converge,review}/project.md`) and #574 (a new
`docs/plans/2026-10-fragment-dsd-not-on-context.md`) — `git diff --stat 1ed126b4 6aea28da -- ':!crates'` lists
exactly those four files. The only line this memo cites that moved is CLAUDE.md's `## CI` `tools` paragraph,
**`:114` → `:99`** (`git grep -nE '両 leg|matrix' 6aea28da -- CLAUDE.md` prints `:30`, `:54`, `:99`); `:30`
and `:54` are unchanged. None of the three other files holds a Python launch line or an interpreter construct
(`git grep -nE 'python3?|\.py\b' 6aea28da -- <the three paths>` prints nothing). Every other line this memo
labels "at `1ed126b4`" or "at `74e15df1`" is unchanged at `6aea28da`.
Between draft 4's base `74e15df1` and `1ed126b4` only `crates/**` changed (`git diff --stat 74e15df1 1ed126b4 --
':!crates'` prints nothing), so every line this memo labels "at `74e15df1`" is unchanged at `1ed126b4`, and draft
4's facts carry over, each re-derivable: `rust-toolchain.toml` is `channel = "1.99.1"` (#567, `git show
6aea28da:rust-toolchain.toml`); #510 landed as `282f04d6`, #501 as `74bc8acf`, #564 as `fd4de110`, #562 (S0 of
the VM P4 lane's #506 split) as `e44ffce6` (`git log --oneline de10bb04..74e15df1`); the `tools` job still has
the two legs `"3.9"` / `"3.x"` (`git show ee6c7a1a:.github/workflows/ci.yml | sed -n '/^  tools:/,/run:
bash/p'`). Between draft 3's base `b051329b` and `74e15df1` the only Python file that changed is
`.claude/tools/plan_memo_population.py` (#569, a docstring; `git diff --stat b051329b 74e15df1 --
':(glob)**/*.py'`). R§4.1's rows were measured at `b051329b` and are not re-run; PR-1 re-runs row 3 at its own base.

## R§4.1 The version record

**Correction.** Draft 2 recorded **3.15.1** (2026-10-26), reading python-build-standalone release `20261021`, and
stated that neither installer had a 3.15.2 build. Re-measured on 2026-10-31 at `b051329b`, both installers'
manifests serve 3.15.2 — python-build-standalone release `20261026` and setup-python's `versions-manifest.json` —
so the unchanged criterion picks **3.15.2**. Whether "no 3.15.2 build" held at the instant of the 2026-10-26
measurement is not re-established; it does not hold at this base.

PM condition (decision 41 (a), decision 117 (1)), measured 2026-10-31 at `b051329b`, mise 2026.10.21:

| # | Condition | Result | Command |
|---|---|---|---|
| 1a | 3.15.x installs with mise on macOS, attestation verification on (mise default `github_attestations = true`), precompiled | **holds** for 3.15.2: rc 0, `attestations verified` in the log, no source build, `Python 3.15.2` | `d=$(mktemp -d); MISE_DATA_DIR=$d/data MISE_CACHE_DIR=$d/cache MISE_STATE_DIR=$d/state MISE_CONFIG_DIR=$d/config MISE_LOG_LEVEL=debug mise install python@3.15.2 2>&1 \| grep 'attestations verified'` |
| 1b | … on Linux | **install unmeasured** (decision 62). Measured instead: the artifact mise selects for x86_64 Linux exists in the same release and its attestation verifies (rc 0). Newest per line in release `20261026`: 3.12.16 / 3.13.18 / 3.14.10 / 3.15.2 | `gh api repos/astral-sh/python-build-standalone/releases/tags/20261026 --jq '.assets[].name' \| grep '^cpython-3.15.2+20261026-x86_64-unknown-linux-gnu-install_only_stripped'`; download it, then `gh attestation verify <file> --repo astral-sh/python-build-standalone`. Install recipe for a Linux host: `docker run --rm ubuntu:24.04 sh -c 'apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null && curl -fsSL https://mise.run \| sh && ~/.local/bin/mise install python@3.15.2'` |
| 2 | setup-python's manifest has 3.15.x for `ubuntu-24.04` x64 | **holds**: newest is 3.15.2 (per line: 3.12.16 / 3.13.18 / 3.14.10 / 3.15.2) | `gh api repos/actions/python-versions/contents/versions-manifest.json --jq .download_url \| xargs curl -sL \| jq -r '.[] \| select(.version\|test("^3\\.15\\.[0-9]+$")) \| select(any(.files[]; .platform=="linux" and .arch=="x64" and .platform_version=="24.04")) \| .version' \| sort -V \| tail -1` (replace `15` for the other lines) |
| 3 | every suite green on 3.15.2 | **holds** at `b051329b` on mise's 3.15.2 build: `python-suites: GREEN -- 9 member(s), 118 test(s) passed, offline`, members including #510's `.claude/tools/test_plan_memo_umbrella_check.py` and #501's `.claude/tools/_webref/test_spec_labels.py`; every tracked `.py` (88 files) compiles under `-W error`. Not re-run at `74e15df1` (§0) | `PATH="$(MISE_DATA_DIR=$d/data mise where python@3.15.2)/bin:$PATH" bash scripts/python-suites.sh` (last line); `git ls-files -z ':(glob)**/*.py' \| xargs -0 -n1 "<that install>/bin/python3" -W error -c 'import sys, pathlib; compile(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"), sys.argv[1], "exec")'` (exit 0). The manifest build on the runner: **unmeasured** until PR-1's own CI run |
| 3x | #510's floor fallbacks under the pin (feeds §5.2) | `import sre_parse` → `ModuleNotFoundError`; `from re import _parser` works; a self-referential symlink: non-strict `Path.resolve()` returns, strict raises `OSError` — no `RuntimeError` | `<that install>/bin/python3 -I -c 'import sre_parse'`; in a scratch dir `ln -s loop.md loop.md`, then `python3 -I -c 'import pathlib; p = pathlib.Path("loop.md"); print(p.resolve()); p.resolve(strict=True)'` |

**Which patch both serve.** Measured 2026-10-31, mise 2026.10.21: the newest patch per line agrees between mise's
plain listing, python-build-standalone `20261026` and the manifest — 3.12.16 / 3.13.18 / 3.14.10 / 3.15.2
(`MISE_MINIMUM_RELEASE_AGE=0 mise ls-remote python 3.15` against row 1b's and row 2's commands). Draft 2's
record found the prebuilt sources one patch behind the listing on every line; that was the state of release
`20261021`, and it is why the criterion reads the prebuilt sources, never the listing alone.

**What `mise ls-remote` lists depends on `python.compile`** (mise 2026.10.21). With the setting unset (`main`
today) the listing is pyenv python-build's definitions — it contains `3.15-dev`, a python-build definition name
that is no CPython release (mise's `python.pyenv_repo` setting points at pyenv) — and it includes patches with no
prebuilt. With `MISE_PYTHON_COMPILE=0` (the same as PR-1's `python.compile = false`) the listing is the
precompiled source's: it **gains fourteen pre-releases** (`3.15.0a1` … `3.15.0rc3`) and **loses `3.15-dev`**
(`MISE_MINIMUM_RELEASE_AGE=0 mise ls-remote python 3.15`, once plain and once with `MISE_PYTHON_COMPILE=0`, then
`comm` of the two; re-measured 2026-10-31). So after PR-1 the listing in the repository reflects what mise can
install prebuilt, but it still shows neither the attestation nor the manifest: a bump runs rows 1a, 1b and 2, not
the listing.

**Does the age filter gate an exact install? Does compile-off refuse a source build?** The age filter does not
gate an exact install (draft 2's record, 2026-10-26: `mise install python@3.15.2`, then hidden from the listing,
proceeded — and, with no prebuilt then, **fell back to compiling CPython from source**, unattested, with a warning
only). With compilation off the same kind of install fails immediately — re-measured 2026-10-31, mise
2026.10.21, on a patch the plain listing has and python-build-standalone does not build:
`MISE_PYTHON_COMPILE=0 MISE_DATA_DIR=$d/data MISE_CACHE_DIR=$d/cache MISE_STATE_DIR=$d/state MISE_CONFIG_DIR=$d/config mise install python@3.11.2` → rc 1,
`no precompiled python found for core:python@3.11.2`. (Find such a patch with `comm -23` of the two listings of
the previous paragraph.) This is why I6 turns compilation off, in PR-1.

## R§4.2 The spelling, the oracle and the install settings — measurements

S2's fact rows, re-measured 2026-10-31 with mise 2026.10.21 (scratch dir: write the files, then
`mise -C <dir> current python`, isolated by the program's rule, body §0): `.tool-versions` resolves over the global
`python = "latest"` and beside a `mise.toml` holding only `[settings] python.compile = false` and a task.
setup-python v7 reads the file
(`gh api 'repos/actions/setup-python/contents/docs/advanced-usage.md?ref=v7' --jq .content | base64 -d | grep -n 'tool-versions'`
— the same paragraph states that a `python-version` input beside `python-version-file` makes the file ignored)
and reads nothing from mise's configuration, so CI is unaffected by a local second spelling — which is exactly
why it would diverge silently.

**Why T5 changed shape (decision 150 (1)).** Draft 3 enumerated the inputs mise reads; draft 4 defined them by a
property (T5a over paths, T5b over `mise.toml`'s keys, classified with `mise config explain`). Each round found
the next input: R3 `"core:python"`, `[tool_alias…]` and `conf.d`; R4 a `.miserc.toml` whose
`override_tool_versions_filenames` / `override_config_filenames` renames the files mise reads (outside T5a's
anchor `^\.?(mise|rtx)([^a-z0-9]|$)`, since `miserc` continues with a letter), and a second `.tool-versions`
line (`core:python 3.14.10` after `python 3.15.2`: mise resolves 3.14.10, setup-python and a first-line reader
3.15.2). mise defines that surface and can widen it in any release; a static model of it leaks by construction —
hence body §4.2's two jobs.

Two candidates measured **benign** (R5 MIN; 2026-10-31, mise 2026.10.21, isolated as body §0, recipe R§11 *T2*): an `[env]
_.file = ".env"` in `mise.toml` whose `.env` sets `MISE_PYTHON_VERSION=3.14.10` — the oracle still resolves the
root `.tool-versions` (mise does not apply a `.env`-set `MISE_*` to version resolution); and
`override_tool_versions_filenames` placed in `mise.toml [settings]` instead of an rc file — ignored, the root
`.tool-versions` still wins. Both are behaviours of that mise release, recorded, not relied on: either becoming
version-changing in a later release is caught by admission like any other version input.

**The oracle's shapes.** Measured 2026-10-31, mise 2026.10.21, isolated as body §0 states (no `mise trust`;
offline; auto-install off), in scratch projects; the installed case uses a fake install directory, nothing was
downloaded. R6's A4 re-ran every row on mise 2026.10.25 with identical results (§12.6):

| case | `version`, `source` | `mise config ls` |
|---|---|---|
| `.tool-versions` = `python 3.15.2` | 3.15.2, `.tool-versions` (its path); `installed: false` | that file |
| + `mise.local.toml` `[tools] python = "3.14.10"` | 3.14.10, `…/mise.local.toml` | both files, no winner shown |
| + `.miserc.toml` `override_tool_versions_filenames = [".alt-versions"]`; `.alt-versions` = `python 3.14.10` | 3.14.10, `…/.alt-versions` | `.alt-versions` only — neither listing shows the rc file |
| `.tool-versions` = `python 3.15.2` / `core:python 3.14.10` | 3.14.10, `…/.tool-versions` | that file |
| `MISE_PYTHON_VERSION=3.14.10` | 3.14.10, `{"type": "environment", "key": "MISE_PYTHON_VERSION"}` | the `.tool-versions` only |
| installed (fake dir), asked with `-C <project>` from another cwd | 3.15.2, `installed: true`; `mise which python3` → `<install_path>/bin/python3` | — |
| (draft 6) no Python configured anywhere (empty global), **or** `.miserc.toml` redirecting to a file that does not exist | `[]` — in the second case although the root `.tool-versions` holds the pin | — |
| (draft 6) `mise.local.toml` `[tools] python = ["3.14.10", "3.15.2"]` | two elements, in that order, both with `source` `…/mise.local.toml`, `type: "mise.toml"` | — |
| (draft 6) `.miserc.toml` redirect, substitute file `python 3.15.2` (T2, same version) | 3.15.2, `source` `{"type": ".tool-versions", "path": "…/.alt-versions"}` — the rc file is not named | — |
| (draft 6) tracked `mise.toml` `[env] _.path = ["altbin"]`, installed (T1) | 3.15.2, `.tool-versions`; `mise which python3` → the install; but `mise exec -- python3` and `mise run` run `altbin/python3` | — |
| (draft 6) untrusted `mise.toml` holding `[settings]` or `[env]` | rc 1, nothing on stdout; stderr `mise ERROR error parsing config file: <path>` then `mise ERROR Config files in <path> are not trusted.` (an untrusted file holding only `[tools]` is read: rc 0) | — |
| (draft 8, mise 2026.10.25) a byte-identical copy of `ee6c7a1a:mise.toml`, untrusted | rc 1, the same two lines, then ``Trust them with `mise trust`. …`` — **`main` refuses already**: its `[bootstrap.packages]` (#535, `a9866498`) is such a table; without it the same file reads (rc 0, `[]`) (R7 A4-7) | — |

Every element also carries `install_path` (draft 8: `<data dir>/installs/python/3.15.2` while `installed: false`),
which the refusal prints. So `mise current python` (version only) and `mise config ls` (loaded files, no winner; it
omitted the variable and the rc file) are not the oracle; `mise where python` and `mise which python3` exit 1
while the version is not installed. The probe scripts (`probe.sh`, `probe2.sh`, with the isolation lines) are
recorded in PR-1's plan (C16).

**Each case's fix** (body §4.2 *The refusal*; H6, checked by this memo, never by the refusal). Actions: (1) install
the pin (with mise if needed); (2) remove or change the mise input that decides Python; (3) `mise -C <root> exec
-- <tool>`; (4) remove what puts `sys.executable`'s directory ahead of mise's install on PATH.

| case (this table's rows, §11 T1 / T2) | the facts printed show | an action that fixes it |
|---|---|---|
| no mise on PATH | "mise not on PATH" | (1) |
| untrusted worktree | mise's stderr lines, the third naming `mise trust` | mise's own line |
| another version wins (`mise.local.toml`, `MISE_PYTHON_VERSION`, a second `.tool-versions` line, an rc-renamed file) | that `source` and its `version` | (2) |
| `[]` (an rc file redirecting to a missing file) | "mise resolves no Python here" | (2), its `.miserc*` clause |
| the pin, not installed | `installed: false` | (1) |
| another `python3` ahead of mise's shim (Homebrew, `/usr/bin`) | `sys.executable` ≠ `mise which python3` | (3) — or (4) |
| `[env] _.path` holds another version (§11 T1 (b)) | `sys.executable` under `<P>/altbin` ≠ `mise which python3` | (4) only — (3) runs `altbin/python3` too |
| cwd outside the repository | cwd ≠ root | (3) |

**The install settings and their overrides** (draft 6's text). mise 2026.10.21 knows the key
`python.github_attestations` (`mise settings get python.github_attestations` → "is not set", where an unknown key
gives "Unknown setting"; unset, it follows the global `github_attestations`, default `true`). Measured 2026-10-31
in a scratch project whose tracked `mise.toml` holds `python.compile = false` and `python.github_attestations =
true`, each install into a fresh data dir with `MISE_LOG_LEVEL=debug`: with `MISE_GITHUB_ATTESTATIONS=false` the
install still logs `GitHub artifact attestations verified` — the python-specific key wins over the global one;
with `MISE_PYTHON_GITHUB_ATTESTATIONS=false` it logs `verification disabled for Python` and installs (rc 0). At
R3 the reviewer measured `MISE_GITHUB_ATTESTATIONS=false mise install python@3.15.2` installing with
"verification disabled" when nothing is tracked, and `MISE_PYTHON_COMPILE=1` and `mise.local.toml [settings]
python.compile = true` each overriding the tracked `false`.

## R§6 Seams searched and alternatives rejected

Draft 4's PR-0 (T5a, the anchored path property with its measured lower bound and a probe re-run at each mise
bump) is replaced by this shape: no probe, no lower-bound list, and no event tied to mise releases (mise is not
pinned; R4 MIN). PR-0's plan is the same file (`…-pr0-tracked-mise-config.md`), written new (C11).

**Seams searched for interval-freedom — before PR-1.**

| Candidate | Lands before PR-1 without an interval, and is not edited by PR-1? | Verdict |
|---|---|---|
| T5 clauses 2–5 | **yes** (body §6 *PR-0 rebuilt*) | **taken — PR-0** |
| T5 clause 1 | no — `python_pin.py`'s parser, dead before PR-1 | stays in PR-1 |
| `scripts/python_pin.py` alone (parser, admission, refusal, oracle, warning) | no — it reads a file that does not exist; dead code until PR-1 | stays in PR-1 |
| the two `[settings]` lines alone | no — their pin needs `tomllib` (absent on the `"3.9"` leg) or an extension of the text reader (rejected below); unpinned, they are unmeasured (I2) | stays |
| the `ci.yml` reader's `python-version` refusal | no — red on `main`, whose setup step has `python-version: ${{ matrix.python-version }}` | stays |
| the floor-code removal (§5.2 V5, V6) | no — V5 is red on `main`'s `"3.9"` leg, V6 wrong on 3.9–3.12 | stays |
| statement rewrites (V1–V4, §5.1, docs) | no — false before the pin | stays |
| the one-leg job and Step 1's exact match | the pin itself | the core |

**Seams after PR-1** (R4 MIN — the same question for what follows PR-1):

| Candidate | Lands after PR-1, before the rest of PR-2, without widening the interval's regression? | Verdict |
|---|---|---|
| the launch-evidence predicate alone | no — red on every entry point it derives until the guards exist | in PR-2 |
| the guard preludes alone | no — a list of entry points, the shape I4 forbids | in PR-2 |
| R-b (the lower bound) alone | no — nothing to bound without the predicate | in PR-2 |
| `SKILL.md` Step 0's comment, the CHECKED flips | no — false until the guard exists | in PR-2 |
| a refusal-wording change in `python_pin.py` | not a seam: the module is PR-1's; PR-2 consumes it unchanged, and a change after PR-1 is a PR-1-scope edit that re-runs §11's non-pin refusal measurement | — |

**Why PR-3 is folded into PR-1 (F5).** Draft 2 left an interval from PR-1 to PR-3 in which I6 (attested prebuilt
or nothing) did not hold: the pin was live, `mise install` was broadcast to every lane (§9 step 4), and nothing
but "no bump lands between" stopped a source build from becoming the pin. *Phase design is not a shortcut
license*: an interval guarded by a convention is the temporary state that rule forbids. The candidates were:
- *PR-3's `tomllib` reader as a prerequisite that lands before PR-1* — **impossible**: before PR-1 the `tools`
  job still runs the `"3.9"` leg, which has no `tomllib` (A-iii §6's own reason for the text reader).
- *`MISE_PYTHON_COMPILE=0` or another out-of-tree equivalent in PR-1* — the setting is local only (CI does not use
  mise), so it can live only in the tracked `mise.toml` or outside the tree; outside the tree it is not tracked,
  hence not measured (I2).
- *`python.compile = false` in PR-1, pinned by extending the text reader* — extends a grammar whose only reason
  PR-1 removes; *one issue, one way* forbids leaving a second TOML reader standing beside the one it would
  become.
- ***`python.compile = false` in PR-1 with the reader change it needs — the `tomllib` reader — chosen.*** PR-1 is
  the PR that makes `tomllib` available (one leg, the pin), so it is also the PR in which the text reader's
  reason disappears; converging the reader in the same PR leaves no state with a reason-less grammar, and the
  F6 refusal — an I1 property — lands with I1's CI half instead of after it. I6 and I1 then hold from PR-1 on.
  The same argument carries `python.github_attestations = true` (G5) into PR-1.

**Why not draft-1's (c)③ ("the two items moved out of Slice B").** Under an exact pin both items dissolve:
*raising the floor* is PR-1's pin itself (3.15.2 ≥ 3.11, asserted by the driver) — a separate PR would have
nothing to change; *Step 0's direct `preflight.py` call* is one member of PR-2's derived population
(`preflight.py` carries launch evidence), and carving it out would be a second way to cover one entry point.
B's and A-ii's wording follows from PR-1/PR-2 and is #514's rework (§8), not a PR here.

## R§7.2 The user's global hooks — the measurement

Measured 2026-10-26 with the pin then decided (3.15.1); the mise version was not recorded (§11). The results
depend on the shim and auto-install, not on the patch.

The three hooks are configured in `~/.claude/settings.json` as direct executions —
`usage-pace.py agent-hook` (PreToolUse `Agent|Workflow`, timeout 5 s), `usage-pace.py prompt-hook`
(UserPromptSubmit, 5 s), `announced-next-step-gate.py` (Stop, 10 s) — all `#!/usr/bin/env python3`, so
`python3` is mise's shim and resolves the session cwd's pin. The status line (`statusline-rate-tee.sh`) runs
`timeout 2 python3 usage-pace.py record` on every refresh. `usage-pace.test.py` is run by hand.
Side effects read from source first: `usage-pace.py` writes `~/.claude/state/usage-pace.log` /
`rate-limits.json` / `usage-pace-day.json` (redirected by `USAGE_PACE_STATE_DIR`); the Stop gate reads the
transcript and writes nothing; the test file works in temporary directories.

**How it was measured** (no write outside the scratch directory; nothing installed into the user's mise):
copies of the hook files in a scratch directory; cwd = a scratch directory holding `.tool-versions`
(`python 3.15.1`) — and once a scratch `mise.toml [tools]` instead; `MISE_DATA_DIR` / `MISE_CACHE_DIR` = fresh
empty scratch directories (so the pin is genuinely not installed); `MISE_GLOBAL_CONFIG_FILE` = a scratch copy of
the user's global config reduced to `python = "latest"` and its `[settings]` (with the full global config, a
fresh data dir also makes mise resolve the user's other `latest` tools, which are installed in the real data
dir — noise the real case does not have); `USAGE_PACE_STATE_DIR` / `USAGE_PACE_AGENTS_DIR` = scratch; stdin =
the event's JSON shape (`session_id`, `transcript_path`, `cwd`, `hook_event_name`, plus `prompt` /
`tool_name`+`tool_input` / `stop_hook_active`); each run under `timeout <settings.json value>`. The invocation is
`printf '%s' "$json" | timeout 5 <copy>/usage-pace.py prompt-hook` from that cwd.

**Results** (timings in the PM report, not here):

1. **Auto-install passes attestation.** The first hook invocation in a pinned cwd triggers mise's shim
   auto-install (`not_found_auto_install = true`); the debug log shows the attestation verified.
2. **Before the pin is installed, the three hooks run and give the right result**: prompt-hook and agent-hook
   exit 0 with no output (no usage state — expected), the Stop gate emits its `"decision": "block"` JSON for a
   transcript ending in an announced step, and `usage-pace.test.py` is green — each within its settings.json
   timeout on the measuring machine. **No hang.** The install is a download, so the margin depends on the
   network.
3. **The status line's 2 s cap kills the install** (exit 124): fail-open, the status line still renders and the
   record is skipped; it leaves an empty `installs/python/<pin>` directory, which the next invocation repairs
   (re-run after the kill: installs and runs). A hook killed by a slow network fails the same way (non-blocking)
   and is retried at its next invocation. **Accepted (PM decisions 41 / 62)**: the cut affects only the first
   status-line run after the pin, and it does not block.
4. **Installing the pin moves the user's global `python = "latest"`** from 3.14.7 to the pin everywhere, because
   mise resolves `latest` against installed versions (scratch data dir holding a link to the real 3.14.7 install:
   `mise -C <neutral dir> current python` prints 3.14.7, then the pin once it is present). So after the landing
   step every hook in every cwd runs the pin — on which `usage-pace.test.py` was green (3.15.1, 2026-10-26; PR-1's
   landing step re-runs it on 3.15.2). **This is a global effect**; the PM takes it to the user.

Verdict: the hooks neither hang nor fail on the supported path; the only failure window is the first invocation
on a slow network, which the landing step (§9 step 4) closes.

## R§11 Recipes

Every recipe follows the program's isolation rule (body §0) and downloads nothing.

**T1 — `[env] _.path`** (§11 T1; mise 2026.10.21, re-run on 2026.10.25):
```sh
ISO=$(mktemp -d); P=$ISO/proj; mkdir -p "$ISO"/{data,cache,state,config} "$P/altbin"; : > "$ISO/global.toml"
export MISE_DATA_DIR=$ISO/data MISE_CACHE_DIR=$ISO/cache MISE_STATE_DIR=$ISO/state MISE_CONFIG_DIR=$ISO/config \
       MISE_GLOBAL_CONFIG_FILE=$ISO/global.toml MISE_OFFLINE=1 MISE_AUTO_INSTALL=false \
       MISE_NOT_FOUND_AUTO_INSTALL=false MISE_TRUSTED_CONFIG_PATHS=$P
i=$MISE_DATA_DIR/installs/python/3.15.2/bin; mkdir -p "$i"            # fake install of the pin
printf '#!/bin/sh\necho "install $0"\n' > "$i/python3"; chmod +x "$i/python3"
printf '#!/bin/sh\necho "altbin $0"\n' > "$P/altbin/python3"; chmod +x "$P/altbin/python3"
cd "$P"; printf 'python 3.15.2\n' > .tool-versions
printf '[env]\n_.path = ["altbin"]\n[tasks.t]\nrun = "python3"\n' > mise.toml
mise ls --current --json python; mise which python3   # .tool-versions; the install
mise exec -- python3; mise run -q t                   # both print "altbin …"
```

**M6 read — withdrawn** (draft 7's `mise env --json` "which cause" read; PM decision 217). Its one measurement
(2026-11-01, mise 2026.10.25, T1's recipe: the `PATH` of `mise env --json` begins `<P>/altbin`, the install's
`bin`, mise's shims) stands as a record; R7's Axis 4 then measured the rule failing on a real interpreter (a
symlinked `altbin/python3` whose `sys.executable`'s real directory is no `PATH` entry, §12.7 J6), and the refusal
no longer reads causes.

**T2 — an rc file renames the version file** (§11 T2; mise 2026.10.21), in T1's environment (fresh `ISO`, `P`):
```sh
cd "$P"; printf 'python 3.15.2\n' > .tool-versions
printf 'override_tool_versions_filenames = [".alt-versions"]\n' > .miserc.toml
printf 'python 3.15.2\n' > .alt-versions;  mise ls --current --json python   # source …/.alt-versions
printf 'python 3.14.10\n' > .alt-versions; mise ls --current --json python   # 3.14.10, …/.alt-versions
rm .alt-versions;                          mise ls --current --json python   # []
```
The benign pair of §4.2 runs in the same environment: `printf '[env]\n_.file = ".env"\n' > mise.toml; printf
'MISE_PYTHON_VERSION=3.14.10\n' > .env` (in a project without the rc file), and `printf
'[settings]\noverride_tool_versions_filenames = [".alt-versions"]\n' > mise.toml` — each still resolves the root
`.tool-versions`.

**The real old interpreter** (§11's refusal bullet, C20, C24; R6 M4, R7 J4). Run by PR-1 in its worktree `<W>`
once `python_pin.py` exists; written here, not yet run as a whole (its parts are measured, R§0 *Draft 8*):
```sh
M=$(type -P mise)      # bash; in zsh: M=$(whence -p mise). Never `command -v mise` (below).
ISO=$(mktemp -d); mkdir -p "$ISO"/{data,cache,state,config}; : > "$ISO/global.toml"
env -i HOME="$HOME" PATH="/usr/bin:/bin:${M%/*}" \
  MISE_DATA_DIR=$ISO/data MISE_CACHE_DIR=$ISO/cache MISE_STATE_DIR=$ISO/state MISE_CONFIG_DIR=$ISO/config \
  MISE_GLOBAL_CONFIG_FILE=$ISO/global.toml MISE_OFFLINE=1 MISE_AUTO_INSTALL=false \
  MISE_NOT_FOUND_AUTO_INSTALL=false MISE_TRUSTED_CONFIG_PATHS="<W>" \
  bash -c 'cd "<W>" && bash scripts/python-suites.sh'; echo "rc=$?"
ls -l "$ISO/state/tracked-configs"   # the post-check: links to <W>/mise.toml and <W>/.tool-versions
```
**Why `type -P` / `whence -p`** (R7 J4, measured 2026-11-01): where `mise activate` has run, `mise` is a shell
function (the session's shell: `type mise` → "a shell function"), so `command -v mise` prints `mise` and draft 7's
`dirname "$(command -v mise)"` gave `.` — no mise on the run's PATH, and the record would have been the "mise not
on PATH" refusal. `type -P` (bash) and `whence -p` (zsh) print the binary (`/opt/homebrew/bin/mise`).
`/usr/bin` first, so Step 1 runs macOS `/usr/bin/python3` (3.9.6); mise's directory last, so the oracle finds mise
but no shim precedes `/usr/bin`. `MISE_TRUSTED_CONFIG_PATHS` lets mise read the tracked `mise.toml` without `mise
trust` — without it mise exits 1 (untrusted: `main`'s `[bootstrap.packages]` already, R§4.2) and only "mise gave no
answer" is recorded. **Expected**: rc ≠ 0; the refusal prints the pin, `3.9.6`, `<W>/.tool-versions`,
`sys.executable` = `/Applications/Xcode.app/Contents/Developer/usr/bin/python3` (what the 3.9.6 launcher reports,
not `/usr/bin/python3`; R7 A4-3, measured), mise's answer (3.15.2 from `<W>/.tool-versions`, `installed: false`,
`install_path` under `$ISO/data`), `mise which python3`'s exit 1, and the generic block. **The post-check is
positive evidence** (R7 J4 / A3-2): `$ISO/state/tracked-configs` holds links into `<W>` (measured: `<W>/mise.toml`,
`<W>/.tool-versions` and `$ISO/global.toml`), so mise ran on the isolated state. "No entry of the user's state
resolves into `$ISO`" cannot fail (the project is `<W>`, not under `$ISO`), and a before/after diff of the user's
`tracked-configs` is blind (its names are path hashes, the same for the same path). The PR body records the output
and the listing; C24's record takes the module's SHA-256, the date and `3.9.6`.
**Mechanising this run in CI was considered (R7 A3-1) and not taken**: the runner's `/usr/bin/python3` is the
image's, pinned by nothing here, so the measured subject would drift with the image; it is not the 3.9 bound the
record states; and a test needing a host interpreter has no fail-safe outcome where the host lacks one (a skip is
red, a pass is silent).

**J2 probe — what `feature_version=(3, 9)` rejects** (§2 I1 × I5, §11; R6 M1, R7 A4-4 / A4-5 / A4-6; 2026-11-01,
Homebrew 3.15.2 and macOS `/usr/bin/python3` 3.9.6). For each snippet, `ast.parse(src)` must succeed on 3.15.2,
then `ast.parse(src, feature_version=(3, 9))` is tried there, and `ast.parse(src)` on 3.9.6:

| construct (introduced in) | snippet | 3.15.2, `feature_version=(3, 9)` | 3.9.6 |
|---|---|---|---|
| `match` (3.10) | `match x:` / `····case 1:` / `········pass` (`·` = a space) | rejected | rejected |
| `except*` (3.11) | `try: pass` / `except* ValueError: pass` | rejected | rejected |
| `type` statement (3.12) | `type X = int` | rejected | rejected |
| PEP 695 type parameters (3.12) | `def f[T](x: T) -> T: return x` | rejected | rejected |
| PEP 758 `except` without parentheses (3.14) | `try: pass` / `except ValueError, TypeError: pass` | rejected | rejected |
| PEP 750 t-string (3.14) | `x = t'{y}'` | rejected | rejected |
| unparenthesised walrus in a subscript (3.10) | `a[x := 1]` | **accepted** | rejected |
| PEP 646 star in a subscript (3.11) | `a[*b]` | **accepted** | rejected |
| PEP 646 starred annotation (3.11) | `def f(*args: *Ts): pass` | **accepted** | rejected |
| PEP 701 f-string, nested same quotes (3.12) | `x = f"{'a' + "b"}"` | **accepted** | rejected |
| PEP 810 `lazy import` (3.15) | `lazy import json` | **accepted** | rejected |
| PEP 798 list comprehension unpacking (3.15) | `x = [*y for y in z]` | **accepted** | rejected |
| PEP 798 dict comprehension unpacking (3.15) | `x = {**v for v in z}` | **accepted** | rejected |
| parenthesised context managers (3.10; not a gap) | `with (open('a') as f, open('b') as g): pass` | accepted | **accepted** |
| walrus (3.8; control) | `if (n := 1): pass` | accepted | accepted |

The table is the probe: each snippet as its own source string, `"\n"` for each `/`, run under `-I`. **The red set
is the six rejected rows** — the one enumeration §2 I1 × I5, §11 and C24 cite. **The gaps are the seven rows
accepted on 3.15.2 and rejected on 3.9.6.** Parenthesised context managers are no gap: 3.9.6 parses them (draft 7
listed them as one). PR-1's grammar control takes the red set as its mutants and states the gaps as its limit
(C24).

---
