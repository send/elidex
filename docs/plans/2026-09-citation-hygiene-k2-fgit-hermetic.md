# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds:
- the measurements and the corpus scripts;
- the fate of `ff6b99a3`'s commits;
- the premises that turned out false;
- every plan-review round's dispositions and the round-5 terminator.

This memo holds only the live decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (see §8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 5**, which answers round 4. Round 4 found 0 CRIT, 9 IMP (5 unique) and 20 MIN.
Nobody broke the detector core; every IMP came with a failing corpus cell. As with draft 4, every
mechanism claim below is a corpus result. The prototype **p5** was driven to pass a representative
corpus subset on both shells and both gits (§6). Round 5 is owed: a **focused** re-review of the
draft 4 → 5 delta.

⚠ **The parent's rule applies throughout:** no quantity moves with a commit, and each figure is a
named-artifact measurement given with its command. In this memo:
- `$S` means a scratch directory;
- the real `HOME` is never written;
- `/usr/bin/grep` is spelled out, because the default `grep` here is ugrep, which can silently return 0.

**Disposition labels.** The companion's tables use round-prefixed labels (F1…, R2-1…, R3-…, U1–U5),
so they cannot collide with this memo's section numbers.

---

## §0 Why this exists

Codex raised a P2 on #501. `_fgit`, which builds the K2 wire's fixture repos, did not neutralise
`GIT_TEMPLATE_DIR`. That was the second failure of a name list around `_fgit`. The first was #519's
Codex R7 (`GIT_CONFIG_COUNT`, parent §10.6).

`ff6b99a3` swapped the list for a `GIT_*` sweep plus a keep-set. That failed a third time, on the
**default** of `core.excludesFile`, `$HOME/.config/git/ignore` (companion §A.1).

This memo therefore grounds the mechanism on a property (§1) and on the direction in which an
unlisted channel fails (§3).

### §0.1 The acceptance criterion, and where it stops

For the fixture build, the parent's §11.1 rule ("unreachable or loud, never silent") is replaced by
**property P**. A loud failure that the caller causes counts as a defect too. The measured failures
were all loud (rc 1), and all came from ordinary developer configuration (companion §A.1). In a
required gate, a false red caused by the caller's setup blocks a valid checkout.

**Where P stops.** P governs the **inputs git reads when it decides what to stage and commit**. It
does not govern two other things:
- which executable runs (R1);
- the platform it runs on (R2, R5).

A failure in those means "this git cannot run here", so it stays under the old rule. Only inputs can
be neutralised from inside the process; the executable and the platform can only be chosen, and that
choice belongs to the launch-environment slot.

**The read side** keeps the old criterion. `_git` preserves the caller's configuration on purpose, for
the reason recorded as #501 R97.

### §0.2 One slice, and where it is registered

CLAUDE.md's edge-dense rule applies, since §10 has three or more axes. The umbrella carries:
- **one slice row**, A-i-wire-fgit;
- **one memo-table row** naming this memo and its companion.

Both are pointers. The umbrella amendment is itself under review.

**The slice** is property P for the fixture build. It covers the mechanism (§3), the bypass detector
(§4) and the postconditions (§5).

**The other commits:**
- **C1 and C2** are standalone prereq splits (§8.1).
- **C4** gives the mutation runner the targets that the §6 records edit. It is required.
- **C3** (record comments) is **not required** to write records, since base holds records without it.
  It stands on its own ground: the records that pin machine-dependent behaviour must say so at the
  record.

CLAUDE.md's clause "(feature PR に bundle しない — split は単独 PR / 単独 commit)" admits a standalone
commit, with parent §11.7 row 7 as precedent. §12 Q1 asks whether round 5 reads it otherwise.

## §1 The property

The controls assert over staged **content** and over **commits**. An attributes file can re-encode a
blob, and a template hook can drop an entry from the index and from `HEAD`; neither changes the path
set (companion §A.1).

> **P.** For each fixture, the following are a function of the fixture script's git commands and
> the files it writes, and of no git input the caller carries:
> - the index (path, mode, blob);
> - the tree of `HEAD`;
> - the ref `HEAD` names;
> - every ref, with a commit compared by its tree and anything else by value.
>
> "Carried" means environment variables, files at default locations (home, XDG, system prefix,
> compiled-in template), and the configuration those name.

Refs are in P because a hostile `init.defaultBranch` or a template `HEAD` moves the branch that
`headprobe` and `badref` build from. Replace-ref and `badref` values are deterministic, so they are
compared by value. Commit ids vary with timestamps, so commits are compared by tree.

**Outside P**, by §0.1's boundary:
- timestamps and reflog identity;
- the build's non-git commands (§5.2 R4);
- reads at control time.

## §2 Measurements

The measurements live in companion §A. The corpus (§6) supersedes every earlier prototype's
whole-wire result.

## §2.5 Spec coverage map

**No spec surface** — this slice changes a shell helper, its controls and its mutation records. It
touches no WHATWG, W3C, TC39 or CSS WG behaviour.

The authority for the channel classes is git's own documentation at the version measured:
- `git help gitignore`;
- `git help git-config` (FILES);
- `git help gitattributes`;
- `git help git-init` (TEMPLATE DIRECTORY);
- `git help git` (ENVIRONMENT VARIABLES);
- `git help git-var`.

webref does not index these. The corpus supplies the rest.

This follows A-iii's shape and the parent's §0.5/§3: the heading has **no table**, so `preflight.py`
fails **by design**. The marker is A-ii's §4.2.5 feature, which has not landed yet. The marker line
matches A-ii's recogniser `^ {0,3}\*\*No spec surface\*\*` (A-iii Codex R13).

```sh
/usr/bin/grep -nE '^ {0,3}\*\*No spec surface\*\*' docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md   # one hit
python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md; echo "rc=$?"   # rc=1, no table
```

## §3 Mechanism, part 1 — `_fgit` is built from nothing

| shape | a channel nobody listed… | verdict |
|---|---|---|
| (i) unset a list of names (base) | reaches the build | failed twice |
| (ii) `GIT_*` sweep plus keep-set (`ff6b99a3`) | reaches it unless it is a `GIT_*` name and a config layer | failed a third time |
| (iii) **`env -i` plus an allowlist**, with every default location pointed at an empty directory | **closed**, whether listed or not | **chosen**; corpus G cells (§6) |
| (iv) OS sandbox | closed, compiled-in paths included | no portable mechanism; needs privileges |

The ingredients come from git's own documentation. `git help git` (`GIT_CONFIG_NOSYSTEM`) says the
variable can be used "along with $HOME and $XDG_CONFIG_HOME to create a predictable environment".
`git help git-config` (FILES) says: "When the XDG_CONFIG_HOME environment variable is not set or
empty, $HOME/.config/ is used". gitignore(5) and gitattributes(5) say the same.

```sh
# harness — a snapshot taken when the harness is sourced, never re-evaluated per call
_FGIT_VOID="$SCRATCH/fgit-void"          # mkdir, checked
_FGIT_ENVBIN="$(command -v env)"         # resolved ONCE, so the build-time env shim (§4) never sees _fgit
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID")
_fgit() { "$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" git "$@"; }
```

What each allowlist entry buys, measured on git 2.55.0 and Apple 2.54.0. Each entry is also a pin in
§6.

| entry | without it |
|---|---|
| `PATH=$PATH` | BSD `env -i` runs `/usr/bin/git` rather than `PATH`'s git (`env -i HOME=$S/v git --version` vs `env -i PATH="$PATH" HOME=$S/v git --version`) |
| `HOME=$VOID` | an unset `HOME` also closes this on 2.55 (`git var GIT_CONFIG_GLOBAL` → rc 1). The void is chosen because a future HOME-relative default then lands where P-c looks |
| `GIT_CONFIG_NOSYSTEM=1` | the system layer is live on this machine (`git var -l` under `env -i` prints `credential.helper=osxkeychain`) |
| `GIT_ATTR_NOSYSTEM=1` | undocumented at 2.55 (`for p in git gitattributes git-config gitignore; do git help -m $p \| col -b \| /usr/bin/grep -c GIT_ATTR_NOSYSTEM; done` → `0 0 0 0`), so it is pinned by `git var`'s answer (P-b) |
| `GIT_TEMPLATE_DIR=$VOID` | the compiled-in template is copied in. A template can seed `config` and `HEAD` as well as `info/*` and hooks |

**One fixture change.** `notcommitted` writes into `.git/info/`, which only the ambient template ever
created. It gains `mkdir -p .git/info`. Without that, a clean caller gets rc 1 with 1 NE (companion
§A.2).

## §4 Mechanism, part 2 — the bypass detector

**Why this shape.** It is invocation-based rather than effect-based. Draft 3 used members with
effects; round 3 measured that no set of effects is universal across subcommands and fixture shapes.
This detector does not ask what a leaking git would change. It asks whether **any git ran during the
fixture build other than through `_fgit`**. So it is subcommand-independent, and the corpus R cells
measure it across every call-site class (§6).

**Channels.** The channels are set **only** around the one ordinary fixture build, and restored
afterwards; the caller's own values are saved and put back. There are **four channels**, and each
route has its own canary and its own label:

| channel | what it sees | canary (after setup, before the build; markers reset after each) |
|---|---|---|
| `PATH="$_FD_DIR/bin:$PATH"` with a `git` shim (appends a marker, exits 97) | any `git` resolved through `PATH`: bare, `command`, `_git` (its `exec git`), `xargs`, `sh -c git` | `( git --version )` must mark |
| an `env` shim in the same directory (marker, exit 97) | any `env` resolved through `PATH`: `env -i …`, `env - …`, `env -u …`. `_fgit` calls `$_FGIT_ENVBIN`, captured at source time | `( env true )` must mark |
| trace2, set by **two routes**: `GIT_TRACE2_EVENT=<abs file>`, and `GIT_CONFIG_GLOBAL=<file setting trace2.eventTarget to the same file>` | any git that inherits the build's environment. trace2 writes `start`/`cmd_name`/`error`/`exit` **before** a configuration error kills the process (re-measured on both gits), so it also records a git that the poison then stops. The config route catches a git that dropped the trace variables (`GIT_TRACE2_EVENT=` or `GIT_TRACE=` spellings, `sh -c 'unset …; exec …'`) | two canaries, one per route: with `GIT_CONFIG_GLOBAL` unset, `hash-object --stdin` must write the file (env route); with `GIT_TRACE2_EVENT` unset, the same (config route) |
| `GIT_CONFIG_COUNT=bogus` (poison) | a git that dropped both trace2 routes. It dies rc 128 on any subcommand that reads configuration; the fixture's chain fails and its control reports NE | with `GIT_TRACE2_EVENT` and `GIT_CONFIG_GLOBAL` unset, `hash-object --stdin` must exit 128 |

**The build sets `GIT_TRACE2_EVENT` itself** because a caller's `GIT_TRACE2_EVENT` **overrides**
`trace2.eventTarget`. Measured: the G cell with a caller `GIT_TRACE2_EVENT=<file>` went NE on the
config canary until the build set its own value (companion §A.8, run 1).

**`GIT_TRACE` is not a channel.** Draft 4 relied on it. Measured in run 2 of the corpus (companion
§A.8): once the poison is set, a config-reading git dies **before** GIT_TRACE's `built-in` line is
written. The absolute-path, `hash -p` and `eval` cells went red on the trace2 label alone. A channel
that never produces its own red cell is dropped (§12 Q3).

**Labels, one per producer** (each producer is one line in `_fgit_detect_verdict`, harness):
- setup: `the fixture-build bypass detector is installed`.
- five liveness labels:
  - `the build's PATH shim for git records a call`;
  - `… for env records a call`;
  - `the build's GIT_TRACE2_EVENT records an inheriting git`;
  - `the build's trace2 target, set through its global configuration, records a git that drops the trace variables`;
  - `the build's poisoned configuration stops a git that drops the trace variables and the global configuration`.
- three hit labels:
  - `no fixture calls git through PATH instead of _fgit`;
  - `no fixture calls env through PATH instead of _fgit`;
  - `no fixture git writes the build's trace2 events`. Its message names the subcommands, from trace2's `cmd_name` events.

The poison has no hit label. When it fires, the verdict is the affected fixture's own NE, and the
trace2 hit also fires unless both routes were dropped. What is pinned is its liveness.

**Where the code lives (U1).** Every producer is a harness function:
- `_fgit_detect_on`, `_fgit_detect_canaries`, `_fgit_detect_off`, `_fgit_detect_verdict`;
- `_fgit_postconditions`, `_fgit_void_empty`.

The **labels** are strings defined in the controls file and passed as arguments, because the
correspondence check looks for labels in the controls file. So every producer's arm can be mutated by
a `harness:` record, and every label is visible to the ratchet.

**Order and state (U5).** Every detector state name is assigned at harness top level, before any
function reads it. Nothing this slice adds relies on `set -u` to fail (see the pre-existing defect in
§5.2). The build sequence is:
1. `if _fgit_detect_on; then _fgit_detect_canaries; fi`
2. `. "$_FIXTURES"` (sourced in the calling shell, exactly as at base)
3. `_fgit_detect_off`

Then, after `ctl_ok=0` (`controls.sh:738` at base):
4. `_fgit_detect_verdict … || ctl_ok=1`
5. the postconditions
6. the controls
7. P-c
8. `[ "$ctl_ok" -eq 0 ] || exit 1`

If setup fails, `_fd_on` stays 0. In that case `_fgit_detect_off` does nothing, and the verdict is
`CONTROL NOT EXERCISED (the fixture-build bypass detector is installed)` on both shells (cells in §6).

**errexit.** The wire runs `set -euo pipefail` (wire:311). Every canary's deliberately failing command
is written `( … ) … || true` or `|| rc=$?`. The build itself runs once, as at base.

**What escapes.** Only a git started **from an empty environment by an absolute-path program**,
e.g. `/usr/bin/env -i PATH=… git` or `/usr/bin/env -i "$_REAL_GIT_BIN"` (corpus RES cells, §6). Such
a process inherits neither `PATH` nor any variable, so no channel can see it. By the same token it
reads **no caller input**, so P holds literally. What it reads instead are machine defaults: the
system layer and the compiled-in template. §7 routes this.

**Cost.** There is still one build. It gains five canaries and the P probes. For the numbers, see
exit criterion X8, not this sentence.

## §5 Postconditions and residuals

### §5.1 Postconditions

These are always on, and run once per run in probe repos built by `_fgit`.

| id | label | assertion | liveness (its own label) |
|---|---|---|---|
| P-a | `the fixture git reads configuration only from the fixture's own config file` | every `_fgit config --list --show-scope --show-origin` line is `local<TAB>file:.git/config<TAB>…` | `this git reports a non-local configuration scope`: `-c a.b=c` must show as scope `command` |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero with no output; `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` appended, both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is empty. It runs **last** | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `_fgit init` against `.git` from `_fgit init --template="$_FGIT_VOID"` is empty (a **content** comparison) | — |
| P-e | `the fixture git is the git the wire reads with` | `_fgit --exec-path` equals `git --exec-path` | — |

### §5.2 Residuals, slots and a pre-existing defect

| # | residual | direction | disposition |
|---|---|---|---|
| R1 | which git executable runs: `PATH`, and a shim git that fails under `env -i` | loud | outside P (§0.1). `PATH` goes to `#11-trip-wire-launch-environment`; the shim half is this slice's own, stated at `_fgit` |
| R2 | filesystem-derived config written by `init`; FIFO support, permissions, raw names | either | outside P. P-d's reference `init` shares the filesystem |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any exists | **declared blind spot**, not owed work. None is known at 2.55 |
| R4 | the **launch-environment class**: anything injected into the shell that runs the build (`BASH_FUNC_*%%`, `BASH_ENV`, `SHELLOPTS`, a function named `command`, non-git tools on `PATH`) | any | `#11-trip-wire-launch-environment` |
| R5 | `$SCRATCH` owned by another UID | loud | outside P |
| R6 | reads through `_git` keep the caller's config. This includes a caller `GIT_TRACE=1` or `GIT_TRACE_SETUP`/`_PERFORMANCE`, which turn trace lines on the inventories' stderr into `err` records. That goes red at base too (Ax3's x0011/x0014 cells report 17 control failures; parent §11.1 D7) | loud, pre-existing | `_git`'s contract; parent D7 |
| R7 | Windows git-bash is not in the trip-wires matrix (`sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep runs-on` → `ubuntu-latest`) | unmeasured | declared |
| R8 | a compiled-in reftable default. `badref` writes `.git/refs/heads/`, and P-d's `diff -r` differs between two reftable inits | loud | declared |

**Pre-existing defect, not this PR's.** On `/bin/bash` 3.2, `set -euo pipefail` combined with the
wire's EXIT trap (present since base, wire:405) turns an unbound-variable abort into **exit 0**. The
orchestrator reproduced it (`…/scratchpad/r4ax2/u.sh`: 3.2 rc=0, 5.3 rc=1). The wire is untouched by
this PR, so it is **not fixed here**; the orchestrator books it as a slot. This slice's own code never
depends on `set -u` to fail (§4, U5).

## §6 The corpus — evidence, and the source of the records

**Subject.** `p5` is a `git clone --local` of `e8f78896` carrying exactly §3–§5. Everything lives
under the session scratchpad `…/scratchpad/fgit/`.

**Subset.** Under the load limit (at most 2 wire runs at once), the representative subset replaces the
full 780-cell run:

```sh
python3 corpus5/gen5.py .                               # 238 cells: 4 configs × (8 G + 33 R + 2 RES + 14 P + 1 INFO) + 6 before-cells on p4
tr '\n' '\0' < corpus5/jobs.tsv | xargs -0 -n1 -P2 corpus5/cell.sh > corpus5/results.tsv
```

**Per configuration** (bash 5.3/3.2 × git 2.55/2.54):
- **G** (8): clean; HOME `.config/git/ignore` `*.py`; `GIT_CONFIG_GLOBAL=/dev/null`;
  `GIT_CONFIG_COUNT` `core.excludesFile`; `GIT_CONFIG_GLOBAL=<all five hostile keys>`;
  `GIT_CONFIG_COUNT` `init.defaultBranch`; caller `GIT_TRACE=<file>`; caller `GIT_TRACE2_EVENT=<file>`.
  The last two are new. The build overrides `GIT_TRACE2_EVENT`, and `GIT_TRACE=<file>` shows that a
  file-target caller trace leaves the read side green.
- **R** (33):
  - one spelling (bare `git`) at each of the 13 call-site classes;
  - every spelling at the `add -A` site. That is draft 4's ten, plus the round-4 escapes:
    - `env -` and `env -i PATH=/usr/bin:/bin git`;
    - `env -i <abs git>`;
    - `GIT_TRACE= <abs>`, `GIT_TRACE=0 eval`, `GIT_TRACE=/dev/null <abs>`;
    - `env -u GIT_TRACE <abs>`;
    - `sh -c 'unset GIT_TRACE GIT_TRACE2_EVENT; exec'`;
    - the poison case, which also unsets `GIT_CONFIG_GLOBAL`;
  - `GIT_TRACE= <abs>` under a caller `*.py` ignore. That is round 4's silently-emptied index.
- **RES** (2): `/usr/bin/env -i PATH=… git` and `/usr/bin/env -i <abs git>`. The classifier now
  requires rc 0, 0 control lines and `PASSED` for RES-GREEN (fixed per round 4).
- **P** (14): the record set below.
- **INFO** (1): P-e with `PATH` dropped, which is machine-conditional (U4).
- **BEFORE** (6, on p4): the U5 setup failure, and two escapes under a caller `*.py` ignore.

Every verdict requires the **named label**, not just a red exit.

**Results** (`cut -f1-3 corpus5/results.tsv | sort | uniq -c`; the full table and every cell are in
companion §A.8):

| config | G (8) | R (33) | P records (14) | RES (2) | INFO: P-e with PATH dropped |
|---|---|---|---|---|---|
| bash 5.3 · git 2.55 | 8 PASS | 33 PASS | 14 PASS | 2 green, named | red (discriminates) |
| bash 3.2 · git 2.55 | 8 PASS | 33 PASS | 14 PASS | 2 green, named | red |
| bash 5.3 · git 2.54 | 8 PASS | 33 PASS | 14 PASS | 2 green, named | green: equivalent, as predicted (U4) |
| bash 3.2 · git 2.54 | 8 PASS | 33 PASS | 14 PASS | 2 green, named | green |

Run 3, on p5 `067c0c40`, 16:20–17:21 JST. `results.err` is empty, and no cell crashed.

**The cells that are only one channel's**, measured on every configuration (the arms column in
`results.tsv`):
- `env -i PATH=/usr/bin:/bin git` → env hit only;
- `GIT_TRACE= <abs>` → trace2 hit only;
- the poison cell (both trace2 routes dropped) → the fixture NEs only, with no hit label.

**BEFORE, on p4, which lacks these fixes** (6 cells):

| cell | bash 5.3 · git 2.55 | bash 3.2 · git 2.54 |
|---|---|---|
| setup failure (U5) | rc 1, **no** label | **rc 0**, no label |
| `GIT_TRACE= <abs>` under a caller `*.py` ignore | rc 0, silent green | rc 0, silent green |
| `env -i PATH=/usr/bin:/bin git` under a caller `*.py` ignore | rc 0, silent green | rc 0, silent green |

**AFTER, on p5:**
- the setup record reds with `CONTROL NOT EXERCISED (the fixture-build bypass detector is installed)`,
  rc 1, on all four configurations;
- both escapes red with their own hit label.

Runs 1 and 2 were aborted, and each exposed a defect that p5 then fixed (companion §A.8):
- run 1: a caller's `GIT_TRACE2_EVENT` overrode the config route;
- run 2: `GIT_TRACE` was shadowed by the poison, and the trace2 canaries shared a file.

**The record set (U1).** Records are drawn from **representative** cells only. The user ruled on
2026-09-27 that making records from the whole corpus is 過大. Each new label gets exactly one record:

| label | record | target |
|---|---|---|
| detector setup | `_FD_DIR` under a non-existent parent (U5) | harness |
| git-shim liveness | the git shim writes no marker | harness |
| env-shim liveness | the env shim writes no marker | harness |
| trace2 env-route liveness | drop `export GIT_TRACE2_EVENT` | harness |
| trace2 config-route liveness | drop `export GIT_CONFIG_GLOBAL` | harness |
| poison liveness | drop `export GIT_CONFIG_COUNT=bogus` | harness |
| git hit | the `add -A` site spelled bare `git` | fixtures |
| env hit | `env -i PATH=/usr/bin:/bin git` there | fixtures |
| trace2 hit | `"$_REAL_GIT_BIN"` there | fixtures |
| P-a | config added to `_FGIT_ENV` | harness |
| P-a liveness | the probe loses `-c a.b=c` | harness |
| P-b | drop `GIT_CONFIG_NOSYSTEM` | harness |
| P-b liveness | the probe loses `…NOSYSTEM=0` | harness |
| P-c | plant a file in the void | harness |
| P-d | drop `GIT_TEMPLATE_DIR` | harness |
| P-e | inject `GIT_EXEC_PATH=/nonexistent-k2` (U4) | harness |

That is **16 new labels and 16 records**. The version-guard labels do have arms, the probe itself, so
draft 4's "no arm, 21 → 23" reasoning is withdrawn. **`_MUT_UNRECORDED_MAX` stays 21** and
`_MUT_RECORDS_MIN` rises by exactly 16. The P and hit cells above *are* these records, measured.

**What the runner needs (U1).**
- `_MUT_TARGETS="wire harness fixtures"`. `wire` is the default and is never stripped, because `w` is a
  sed command.
- `fixtures:` is a BSD `sed` error, like `harness:` (`sed 'fixtures:p' </dev/null` → "invalid command
  code f", rc 1). GNU is **unmeasured**: `f` is not in GNU sed's command list per its manual, but that
  has not been run.
- The controls file stays excluded, because a `c` prefix parses under GNU sed. Nothing that needs a
  record lives there any more; the producers moved to the harness.
- The parts list (§8.1) feeds the runner's copies. Since C1 adds `fixtures` and C2 adds `mutgen`, and
  both are copied beside each mutant, a `fixtures:` record edits the copy that the mutant's controls
  source.

**RES and `!survive`.** RES cells are **not** records. A record whose expected result is survival
would need the `!survive` needle, and the runner requires that needle exactly once
(`mutations.sh:658–662` at `e8f78896`). So RES stays a corpus cell (X5), not a record.

**U4: P-e.** The record is now `GIT_EXEC_PATH` injected into `_FGIT_ENV`, which reds P-e on **both**
gits. The `PATH`-drop mutant is kept only as the INFO cell: it reds on git 2.55 and is equivalent on
Apple 2.54, where `env`'s default-path git *is* the `PATH` git. It is **not** a record, so X3's
"0 not killed" holds in every column. No runner mechanism was added.

**Mutation-mode cost (X3).** One run is (95 base records + 16) record trials plus the generated
population, each trial being one full control pass. The records-and-generated count is measured by
X3 itself (`… N entr(ies)` / `… N mutant(s)`). The per-pass time is X8's wire time. The product is the
opt-in run's cost; it does not add to the always-run gate.

## §7 `_fgit` vs `_git`, and the carve slots

`_git` preserves the caller's configuration. `_fgit` constructs its environment. They share no policy.
A fixture calling `_git` is caught: `_git`'s `exec git` resolves through `PATH` (corpus R).

- **`#11-k2-fgit-keepset-depends-on-git-purge-glob`: DISSOLVES.**
  - The keep-set no longer exists.
  - Its premise sentence was absent at `ff6b99a3`:
    `git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i 'exempt\|held back\|purge'`
    returns rc 1.
  - The `_git_exec` it named is not built, and is not needed.
- **`#11-k2-fixture-git-invocation-convention`: NARROWED to one property-defined gap.**
  - **Gap**: a fixture starts git **from an empty environment through an absolute-path program**
    (`/usr/bin/env -i …`, or `exec -c` / `env -i` via an absolute path, or any non-shell tool doing
    the same). It reads the system layer and the default template, and no channel sees it.
  - **Why it stays open**: nothing inherited reaches such a process. Closing the gap needs P-a/P-d per
    fixture repo, or an OS sandbox.
  - **What it costs**: the process reads no caller input, so P holds. The exposure is machine files,
    the same class as R3.
  - **Trigger**, by property rather than vocabulary: any fixtures-file change that starts a process
    by absolute path with an emptied environment. That includes `env -` and `$_REAL_GIT` through
    `eval`, since the latter is an absolute path. Reviewers see it because that line's command word is
    not `_fgit` and it names a path.
  - **Owner**: citation-hygiene lane. **Re-eval**: 2026-11-30.
  - **Create-time audit**: pre-existing slot, narrowed, not new; own new deferrals in this PR: **0**.
  - **Why it does not close**: its gap is not empty; the RES cells demonstrate it.
  - Its original premise, "Measured 2026-09-26: the build region contains no bare `git ` outside
    comments", holds at `e8f78896`:
    `awk 'NR>=83 && NR<=734 && !/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -cE '(^|[^_A-Za-z$/-])git [a-z-]'`
    → `0`.

**Ledger step** (the memory file `project_open-defer-slots.md`, outside this diff):
- remove the dissolved slot;
- rewrite the narrowed one as above;
- fix the header's count and source;
- cite **#501's merge commit on `main`**, since this PR's squash SHA becomes unreachable.

## §8 Splits, the parent memo, and memo references

### §8.1 Code splits

`wc -l .claude/tools/webref-generic-core-trip-wire*.sh` at `e8f78896` gives wire 1259, controls 989,
mutations 783, harness 195. `ff6b99a3` broke CLAUDE.md's touch-time rule: its first commit took
controls to 1026 lines (`git show c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l`)
without a prereq split.

| file | split | seam | ground |
|---|---|---|---|
| controls | **C1** | build vs assert. `…trip-wire.fixtures.sh` takes **base controls lines 83–734**: from `for d in clean pin` up to the blank line before `ctl_ok=0`'s three-line comment (lines 735–737), which stays with `ctl_ok=0`. The file is **sourced**, and the detector wraps it | this PR grows both halves. `wc -l` over p5's parts is in companion §A.8 |
| mutations | **C2** | the two populations named in the header. The generated half moves to `…trip-wire.mutgen.sh` | `ff6b99a3` reached 983 lines with the infrastructure C3/C4 carry, and this PR adds 16 records |
| harness | no | — | stays below the threshold with the §4/§5 functions. `wc -l` in companion §A.8 |
| wire | **not touched** | — | `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` → `0` |

**What C1/C2 bring with them:**
- **One parts list feeds all three readers in `_mut_run`**: the copy, the trap's `rm -f`, and the
  stale-skip `case`.
- **The sibling guard lives only inside `_mut_run`.**
- The entry contracts extend to the new files.

**In-file statements C2/C4/C5 must rewrite in the same commit**, because they become false:
- `mutations.sh:44–46`: "Every control has to be named by some record". The population becomes
  `_control` labels plus `_lbl` labels.
- `:136–138`: the correspondence text names the umask and fsmonitor blocks as the non-`_control`
  labels. There are now 16 more.
- `:684`: the ratchet message says "controls have no mutation record". It becomes "labels".

### §8.2 The parent memo: a banner, no split (C0b)

**C0a is withdrawn.** CLAUDE.md requires a split only "real cohesion seam があれば". The parent has no
record-vs-design seam, for two reasons:
- **§5 is live substance.** Parent §7 criterion 4 is "Met" because §5 is the answers; parent §9 says
  "the substance is in §5 and §8"; header L8 cites §5's answers.
- **§10 is cited by live §11**:
  `awk '/^### §11/{s=1} s' docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md | /usr/bin/grep -o '§10\.[0-9]' | sort -u`
  → `§10.4`, `§10.8`.

**The banner's sites** are derived by **property**, not by section name. Candidates come from:

```sh
awk '/^#{2,3} /{sec=$0} $0 ~ /trip-wire\.(controls|harness|mutations)\.sh|controls file|control harness|mutation set|_fgit|GIT_TEMPLATE_DIR|D6|\*\*Status\*\*|over the wire|controls with no record|_MUT_UNRECORDED_MAX|ratchet/ {printf "%d\t%s\n", NR, substr(sec,1,40)}' \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md | /usr/bin/grep -v '§10\.'
```

Each candidate is read and kept if and only if it is a **present-tense statement that one of C1–C5
makes false**. At `e8f78896` that gives **nine sites**:

| parent line | why it becomes false |
|---|---|
| L5 | the Status |
| L61 | §0's derivation command names the controls file, which loses the build half (C1) |
| L278–280 | §4's file rows (C1/C2) |
| L675–677 | §7 criterion 3, "enumerated in `…mutations.sh`" (C2) |
| L683 | §7 criterion 3, "a `sed` expression over the wire" (C4 targets) |
| L746–747 | §7 criterion 5, "All are now in §4's table" (C1/C2) |
| L866–868 | §9, "What is ratcheted now is the number of controls with no record" (C5 population) |
| L1220 | §11.1 D6 (§0.1) |
| L1282 | §11.4 P1's file list (C1/C2) |

Everything else the command prints is either still true or a report of a past round. ⚠ Draft 3 said
four sites, round 3 said five, draft 4 said seven; the property rule gives nine. The banner names all
nine, and the body stays as it is.

### §8.3 Memo references in code

The grep for memo references now needs no "memo" token. It lists every `§N`:
`for f in .claude/tools/webref-generic-core-trip-wire*.sh; do /usr/bin/grep -nE '§[0-9]' "$f"; done`.
Each hit is resolved by subject against the section maps of the A-i memo
(`2026-07-citation-hygiene-Ai-spec-label-map.md`: §2 = the K2 invariant, §12 = exit criteria) and the
parent (§6 interpreter floor, §8 defer slots, §11 design revision):

| file | lines | memo |
|---|---|---|
| wire | 4, 5, 192 (qualified); 38, 158, 195, 206, 225, 451, 461, 534, 550 (unqualified: K2 predicate §2, exit criteria §12) | **A-i memo** |
| wire | 307 (qualified); 199, 267, 323, 403, 456, 685, 724, 1164 (unqualified: slots §8, revision §11) | **parent** |
| controls | 95, 116 (§2, the K2 predicate) | A-i memo |
| controls | 10, 273, 614, 906 (§8, §11 D10, §11.2, §11.1) | parent |
| mutations | 15, 20 (a prose "the plan memo" with no §) | parent (§10.5, §8) |

- **The wire** (untouched): 9 unqualified references mean the A-i memo and 8 mean the parent. Draft 4
  said "A-i memo for 1", which was false. **This table is the record.**
- **Files this PR edits:** every reference is qualified with the file name in C1/C2. The real check
  is X4b, not X4's behaviour diff.
- **This memo** is only ever cited by file name.

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, the companion, the umbrella rows | docs | the round-5 focused review |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (nine derived sites) | docs | the §8.2 command, re-read |
| C1 | controls → controls + fixtures (sourced); one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4, X4b |
| C2 | mutations → mutations + mutgen; references qualified | prereq split | X1, X3, X4, X4b |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | `_MUT_TARGETS="wire harness fixtures"` with resolver and restore | infra, required | X3 |
| C5 | §3–§5 as in p5; the 16 records; the ratchet population; §8.1's in-file rewrites; the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

**Cost, and `ci.yml`.** The base job comment is an in-file rule: "a wire that adds fixture self-tests
re-derives this line in the same PR". It can be read with `git show e8f78896:.github/workflows/ci.yml | sed -n '/^  trip-wires:/,/^  [a-z]/p'`.
C5 re-derives the line by that rule's own method and records the method and verdict, with no figures.
If the derived value differs from `5`: **STOP and escalate to the user**. The budget half is unowned
(parent §6; umbrella, Cross-lane coordination), and #510 is its other claimant (head via
`gh pr view 510 --json headRefOid`).

**Land order:**
1. `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`.
2. User approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. `/external-converge`.
4. User approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella's single status cell.
7. Hand the #501 merge decision to the user.
8. After #501 lands, the §7 ledger step.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | `env -i` closes variables, and four relocations close default files. A template-seeded local config is visible only to P-d; an include only to P-a | §3, §5.1 |
| 2 | B×E | an empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 3 | A×D | the detector is invocation-based, over four channels (five routes). Each route is proven live alone. Its only gap is a git started from nothing by an absolute-path program, which reads no caller input | §4; corpus R, RES |
| 4 | A×C | `_fgit`'s `env` and `PATH` are snapshots taken before the detector's shims exist, so `_fgit` is never a hit | §3, §4 |
| 5 | D | detector state is assigned before any read. Verdicts are written after `ctl_ok=0`. P-c runs last. Nothing added relies on `set -u` | §4, §5.2 |
| 6 | D×E | the detector is off, with the caller's `PATH`/`GIT_TRACE2_EVENT`/`GIT_CONFIG_GLOBAL`/`GIT_CONFIG_COUNT` restored, before any `_control` re-invokes the wire | §4; corpus G |
| 7 | E×D | producers live in the harness; labels live in the controls file; records target `harness` or `fixtures` | §4, §6 |
| 8 | C×D | targets are `wire` (default, never stripped), `harness` and `fixtures`. `controls:` is not used. The GNU behaviour of `fixtures:` is unmeasured | §6 |
| 9 | D | one label per producer, and one record per label. The ratchet stays at 21 | §6 |
| 10 | C×D | no always-counted record is machine-dependent: P-e's record uses `GIT_EXEC_PATH` | §6 (U4) |
| 11 | F | a single build; the ci.yml line is re-derived by its own rule, and a change means STOP | §9 |
| 12 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it | §0.1, §5.2 |

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | `( $SH $W ) > $S/l 2>&1; echo rc=$?; /usr/bin/grep -c 'CONTROL NOT EXERCISED\|CONTROL FAILED' $S/l; /usr/bin/grep -c 'trip-wire PASSED' $S/l` | `rc=0`, `0`, `1` |
| X2 | the §6 G cells (`gen5.py` subset) on the implementing head, at `-P2` | all PASS, 4 configs |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'` | three hits in every column. C1–C3 byte-identical to base |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | in every file C1/C2 edit, each `§[0-9]` / "plan memo" / "the memo" reference names a memo file: `/usr/bin/grep -nE '§[0-9]\|plan memo\|the memo' <file> \| /usr/bin/grep -v 'citation-hygiene-[A-Za-z0-9-]*\.md'` | empty |
| X5 | the §6 R, RES, P and INFO cells on the implementing head, at `-P2` | R and P PASS; RES green (named); INFO red on git 2.55 and green on 2.54 |
| X6 | P-a/P-d planting (a template `config`+`HEAD`; a local `include.path`) | red |
| X8 | the ci.yml rule's derivation (§9) | method and verdict recorded; STOP on change |
| X9 | the PR's `Layering trip-wires` job (ubuntu, GNU) | SUCCESS. This is GNU evidence for `env -i`, `GIT_TRACE`, trace2 and the shims; it is not evidence for the sed claim |
| X10 | `$SH -n` and `wc -l` over `ls .claude/tools/webref-generic-core-trip-wire*.sh` | clean; every part below 1000 lines |
| X11 | in a clone, add `_x_lbl="an unrecorded probe"` and an `echo` that uses it | red, and the ratchet lists it |

## §12 Questions for the round-5 focused review

- **Q1.** Are C1/C2 acceptable as standalone commits in this PR? *Draft answer:* yes, per parent §11.7
  row 7.
- **Q2.** Is the narrowed invocation-convention slot, one property-defined gap that reads no caller
  input, acceptable? *Draft answer:* yes. Closing it needs per-fixture postconditions or a sandbox.
- **Q3.** Four channels are more mechanism than draft 4's two. Is each one earning its place? *Draft
  answer:* each has a cell that only it turns red (§6: the env hit, the trace2 hit, and the poison NE
  cell), plus its own liveness record. `GIT_TRACE` had no such cell once the poison was in, so it was
  dropped (measured, §4).
