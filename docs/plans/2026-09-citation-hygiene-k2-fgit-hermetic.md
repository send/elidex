# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits, false premises, every
plan-review round's dispositions, and the round-6 terminator. This memo holds only the live
decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (§8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 6.** The round-5 focused review failed its terminator. The root cause was that
drafts 4–5's bypass *detector* was itself a name list: a git that neutralised the watched names still
read the caller's HOME/XDG (companion §A.9, before-cells).

The design therefore changed (orchestrating session, 2026-09-27). P now holds **by construction of
the whole fixture-build window**. The detector, its four channels, its canaries, its nine labels and
its records are **deleted**. Every claim below is a result from the recast corpus run on prototype
**p6** (§6).

Round 6 is owed: a focused re-review of the draft 5→6 delta.

⚠ **The parent's rule applies throughout.** No quantity here moves with a commit; each figure is a
measurement on a named artifact, given with its command.
- `$S` means a scratch directory.
- The real `HOME` is never written.
- `/usr/bin/grep` is spelled out, because the default `grep` on this machine is ugrep, which can
  silently return 0.
- Disposition labels in the companion (F…, R2-…, R3-…, U…, V…) never collide with this memo's
  section numbers.

---

## §0 Why this exists

Codex raised a P2 on #501: the helper that builds the K2 wire's fixture repos did not neutralise
`GIT_TEMPLATE_DIR`. Name lists failed three times:
1. #519's R7 (`GIT_CONFIG_COUNT`, parent §10.6);
2. this P2;
3. `ff6b99a3`'s `GIT_*` sweep plus keep-set, on the *default* of `core.excludesFile`
   (companion §A.1).

Drafts 4–5 then moved the same shape into a bypass detector, and round 5 broke it the same way. This
draft stops enumerating anything the caller carries. It constructs the build's environment from
nothing (§3).

### §0.1 The acceptance criterion, and where it stops

For the fixture build, the parent's §11.1 rule, "unreachable or loud, never silent", is replaced by
**property P** (§1). A loud failure that the caller causes is a defect too. The measured failures
were loud (rc 1), and they came from ordinary developer configuration (companion §A.1). In a required
gate, a false red from that setup blocks a valid checkout.

**Where P stops.** P governs the **inputs git reads when it decides what to stage and commit**. It
does not govern:
- which executable runs (R1);
- the platform it runs on (R2, R5).

A failure there means "this git cannot run here", so it stays under the old rule.

The **read** side is also out of scope. It keeps the old criterion, because `_git` preserves the
caller's configuration on purpose (#501 R97). The window is a child process, so the parent's
environment, and with it the read side, is untouched (the DO cell in §6).

### §0.2 One slice, and where it is registered

CLAUDE.md's edge-dense rule applies, because §10 has three or more axes. The umbrella carries **one
slice row** (A-i-wire-fgit) and **one memo-table row** naming this memo and its companion. Both are
pointers, and the umbrella amendment is itself under review.

The slice is property P for the fixture build: the window (§3), the postconditions that run inside it
(§4), and nothing else.

- **C1/C2** are standalone prereq splits (§8.1).
- **C4** gives the mutation runner a `harness` target (§6). It is required, because every record edits
  the harness.
- **C3**, record comments, is **not required** for writing records. It stands on its own: records that
  pin machine-dependent behaviour must say so at the record.

CLAUDE.md's clause "(feature PR に bundle しない — split は単独 PR / 単独 commit)" admits a
standalone commit; parent §11.7 row 7 is the precedent (§12 Q1).

## §1 The property

> **P.** For each fixture, the following are a function of the fixture script's git commands and the
> files it writes, and of no git input the caller carries:
> - the index (path, mode, blob);
> - the tree of `HEAD`;
> - the ref `HEAD` names;
> - every ref: a commit by its tree, anything else by value.
>
> "Carried" means environment variables, files at default locations (home, XDG, system prefix,
> compiled-in template), and the configuration those name.

Content and commits belong in P, because an attributes file can re-encode a blob and a template hook
can drop an entry (companion §A.1). Refs belong in P, because `init.defaultBranch` or a template
`HEAD` moves the branch that `headprobe` and `badref` build from.

**Outside P** (§0.1):
- timestamps and reflog identity;
- the build's non-git commands (R4);
- reads at control time.

The corpus checks P **directly**: the recast cells compare every fixture's P to a clean build's P
(§6).

## §2 Measurements

The measurements are in companion §A. The draft-6 corpus (§A.9) supersedes every earlier prototype's
result on the mechanism.

## §2.5 Spec coverage map

**No spec surface** — this slice changes a shell harness, its fixtures, its controls and its mutation
records. It touches no WHATWG / W3C / TC39 / CSS WG behaviour.

The authority for the channel classes is git's own documentation at the version measured:
- `git help gitignore`
- `git help git-config` (FILES)
- `git help gitattributes`
- `git help git-init` (TEMPLATE DIRECTORY)
- `git help git` (ENVIRONMENT VARIABLES)
- `git help git-var`

Webref does not index these. The corpus supplies the rest.

This follows A-iii's shape and the parent's §0.5/§3. The heading has **no table**, so `preflight.py`
fails **by design**. The marker is A-ii's §4.2.5 feature, which has not landed yet. The marker line
matches A-ii's recogniser `^ {0,3}\*\*No spec surface\*\*` (A-iii Codex R13).

```sh
/usr/bin/grep -nE '^ {0,3}\*\*No spec surface\*\*' docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md   # one hit
python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md; echo "rc=$?"   # rc=1, no table
```

## §3 Mechanism — the fixture build runs in a window built from nothing

| shape | a channel nobody listed… | verdict |
|---|---|---|
| (i) unset a list of names (base) | reaches the build | failed twice |
| (ii) `GIT_*` sweep plus keep-set (`ff6b99a3`) | reaches it unless it is a `GIT_*` name **and** a config layer | failed a third time |
| (ii′) per-call `env -i` plus a bypass detector (drafts 4–5) | reaches a git that bypasses the helper and neutralises the watched names | failed in round 5 |
| (iii) **the whole build window from nothing**: `env -i` plus an allowlist around one child shell that sources the fixtures file | **closed for every git in the window, however it is spelled**, because each inherits only the window's environment | **chosen**; corpus §6 |
| (iv) OS sandbox | closed, including compiled-in paths | no portable mechanism; needs privileges |

Git's documentation supports these ingredients. `git help git` (`GIT_CONFIG_NOSYSTEM`) says it can be
used "along with $HOME and $XDG_CONFIG_HOME to create a predictable environment". `git help
git-config` (FILES) says "When the XDG_CONFIG_HOME environment variable is not set or empty,
$HOME/.config/ is used".

```sh
# harness — snapshots taken when the harness is sourced
_FGIT_VOID="$SCRATCH/fgit-void"             # mkdir, checked
_FGIT_ENVBIN="$(command -v env)"; _FGIT_BASH="$BASH"
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID" "LC_ALL=C")
# _fgit_window: write a prelude of PLAIN assignments and function bodies, then
"$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" "$_FGIT_BASH" -c '
  . "$1"; . "$2"; shift 2                       # prelude, then the fixtures file
  printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
  _fgit_postconditions "$@" || : > "$_FW_DIR/post_bad"
  : > "$_FW_DIR/done"' _ "$_FW_DIR/prelude.sh" "$_FIXTURES" <labels…> || _fw_rc=$?
```

**What the child gets, kept minimal.** The prelude contains:
- `set -euo pipefail`;
- plain `name=%q` assignments for the data the fixtures file reads, which a census of the fixtures file
  gives as: `CTL`, the five `CONTROL_*` samples, `_REAL_GIT`, `_REAL_GREP`, `_fifo_ok`, plus the
  window's own `_FGIT_VOID`, `_FGIT_ENVBIN`, `_FGIT_ENV_NAMES`, `_FGIT_WIRE_EXEC`, `_FW_DIR`;
- `declare -f` bodies of `_fixture_failed`, `_shq` and `_fgit_postconditions`.

Assignments are **plain**, never `declare -p`. `declare -p` carries the `export` attribute into the
window; P-f caught that in p6's first run (companion §A.9).

**What comes back.** `_FIX_FAILED` returns through a file. So does the fact of completion (`done`)
and whether any postcondition reported. The window's own `CONTROL …` lines reach the wire's stderr
directly.

**Every git in the window is equal.** The fixtures file calls **`git`**, and the per-call `_fgit`
helper is **gone**. That is one issue, one way. Measured: after the split, no `_fgit` use remains
outside the window. The postconditions, the only other caller, now run inside it.

**What each allowlist entry buys.** Measured on git 2.55.0 and Apple 2.54.0; each is also pinned in
§6.

| entry | without it |
|---|---|
| `PATH=$PATH` | BSD `env -i` runs `/usr/bin/git` rather than `PATH`'s git (INFO cell) |
| `HOME=$VOID` | an unset `HOME` also closes this on 2.55. The void is chosen because a future HOME-relative default then lands where P-c looks |
| `GIT_CONFIG_NOSYSTEM=1` | the system layer is live here (`/opt/homebrew/etc/gitconfig`) |
| `GIT_ATTR_NOSYSTEM=1` | undocumented at 2.55 (`git help -m …` for four pages gives 0 hits), so it is pinned by `git var` (P-b) |
| `GIT_TEMPLATE_DIR=$VOID` | the compiled-in template is copied in |
| `LC_ALL=C` | keeps the window's locale the one the wire exports (wire:319), so the build is like-for-like with base |

**The fixture change.** `notcommitted` gains `mkdir -p .git/info`, because the empty template no
longer creates it (companion §A.2).

**errexit and state.** Every window state name (`_fw_rc`, `_fw_done`, `_fw_post_bad`) is assigned at
harness top level, before anything reads it. The child runs under `set -euo pipefail` with **no EXIT
trap**, so an abort there is a non-zero exit. The parent reads it as data. The verdict is written
after `ctl_ok=0` (`controls.sh:738` at base).

Nothing this slice adds relies on `set -u` to fail. See §5.2's pre-existing defect.

## §4 Postconditions — run INSIDE the window

Any bypass inside the window inherits the same environment. So git's own answers inside the window
describe every git there. Each producer has its own label (defined in the controls file and passed
in), and each label has its own record (§6).

| id | label | assertion (inside the window) | liveness (its own label) |
|---|---|---|---|
| W | `the fixture build window completed` | the child wrote `done`. Otherwise NE with its exit code | — |
| P-a | `the fixture git reads configuration only from the fixture's own config file` | every `git config --list --show-scope --show-origin` line in a probe repo is `local<TAB>file:.git/config<TAB>…` | `this git reports a non-local configuration scope`: `-c a.b=c` must show as scope `command` |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is empty. Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `the fixture git is the git the wire reads with` | the window's `git --exec-path` equals the parent's, which is captured at source time | — |
| P-f | `the fixture build window's environment holds only its allowlist` | every name the window's `env` lists is an allowlist name or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction | — |

## §5 Residuals, the slot, and a pre-existing defect

### §5.1 What the window does not close — stated truthfully

**A git in the window that discards or overrides the allowlist.** The fixtures file itself can remove
an allowlist entry or point it elsewhere, for example `env -i … git`, `env -u GIT_CONFIG_NOSYSTEM
git`, or `GIT_CONFIG_NOSYSTEM=0 git`. Such a git reads what the allowlist switched off: the **system
layer** and the **compiled-in template**.

Those are **caller-writable** on a Homebrew machine, measured here:
- `env -i PATH=… git config --list --show-origin --show-scope` shows
  `system	file:/opt/homebrew/etc/gitconfig	credential.helper=osxkeychain`;
- that file and its directory, and the template directory, are owned by the user
  (`ls -l /opt/homebrew/etc/gitconfig` → `kazuaki admin`;
  `stat -f '%Su' /opt/homebrew/opt/git/share/git-core/templates` → `kazuaki`).

So draft 5's "reads no caller input" was **false**.

The corpus RES cells show these spellings pass and produce the clean P here, but only because this
machine's system file and template happen to be benign. The same fixture on a machine with a hostile
`/opt/homebrew/etc/gitconfig` would leak.

**`#11-k2-fixture-git-invocation-convention`, re-derived by property:**
- **Gap:** a fixtures-file command that **removes or overrides one of the window's allowlist entries
  for a process it starts**. The allowlist entries are `PATH HOME GIT_CONFIG_NOSYSTEM GIT_ATTR_NOSYSTEM
  GIT_TEMPLATE_DIR LC_ALL`. Examples are `env -i`, `env -`, `env -u <name>`, `<name>=… git`, or
  `unset <name>`. Such a process reads machine files the caller can write.
- **Why it stays open:** the window cannot constrain what a fixture deliberately undoes inside itself.
  Closing the gap would need per-process postconditions or an OS sandbox.
- **Trigger:** any change to the fixtures file that names an allowlist entry, or starts a process
  with an emptied environment. A reviewer can see this in the diff, because the name appears.
- **Owner:** citation-hygiene lane. **Re-eval:** 2026-11-30.
- **Create-time audit:** the slot pre-exists and is re-scoped, not newly created, so there are 0 new
  own deferrals.
- **Its original premise** ("no bare `git` in the build region") is **moot**. Bare `git` is now the
  canonical spelling, and the window makes it safe.

`#11-k2-fgit-keepset-depends-on-git-purge-glob` **dissolves**. There is no keep-set and no helper that
calls `_git`. Its premise sentence was absent at `ff6b99a3`:
`git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i 'exempt\|held back\|purge'`
returns rc 1.

`_git` inside the window is **undefined**. A fixture calling it fails loudly: the corpus RLOUD cell gives
NE.

### §5.2 Other residuals, and the pre-existing defect

| # | residual | direction | disposition |
|---|---|---|---|
| R1 | which git executable runs (`PATH`; a wrapper git that needs other variables fails in the window) | loud | outside P (§0.1). `PATH` → `#11-trip-wire-launch-environment` |
| R2 | filesystem-derived config written by `init`; FIFO support, permissions, raw names | either | outside P. P-d's reference `init` shares the filesystem |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any | declared blind spot. None is known at 2.55 |
| R4 | the **launch-environment class**: whatever the caller injects into the wire's own bash at startup (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*%%`, a function named `command`). The window's `env -i` drops these from the child's environment, but the parent that writes the prelude has already run under them. The prelude carries only the listed data and three function bodies | any | `#11-trip-wire-launch-environment` |
| R5 | `$SCRATCH` owned by another UID | loud | outside P |
| R6 | reads through `_git` keep the caller's config, including a caller `GIT_TRACE=1`, which reds at base too (parent D7) | loud, pre-existing | `_git`'s contract |
| R7 | Windows git-bash is not in the trip-wires matrix (`sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep runs-on` → `ubuntu-latest`) | unmeasured | declared |
| R8 | a compiled-in reftable default: `badref` writes `.git/refs/heads/`, and P-d's `diff -r` differs between two reftable inits | loud | declared |

**Pre-existing, not fixed here: `#11-k2-wire-exit-trap-masks-set-u-abort`.** On `/bin/bash` 3.2,
`set -euo pipefail` together with the wire's EXIT trap (since base, wire:405) turns an
unbound-variable abort into exit 0. The orchestrator reproduced it (`…/scratchpad/r4ax2/u.sh`: 3.2
rc=0, 5.3 rc=1) and owns the slot, whose ledger trigger is "code sourced after wire:405".

This PR's harness and controls are exactly such code. So **nothing this slice adds relies on `set -u`
to fail**: every state name is assigned before it is read, and the window child has no EXIT trap.

## §6 The corpus — evidence, and the source of the records

**Oracle, recast.** Under a hostile caller, a bypass spelling no longer has to go red. It has to go
**green with P identical to the clean build's P**. P is dumped per fixture repo by a **prototype-only**
hook (`K2_CORPUS_PDUMP`), which is not part of the design. It is passed into the window as a plain
assignment, so P-f does not see it. Symlink targets are normalised for `$CTL` (companion §A.9).

**Subject.** p6 is a `git clone --local` of `e8f78896` carrying exactly §3–§4.

**Subset and re-run recipe.** Scripts are in companion §E.3. Run them from a directory holding the
tree under test as `p6/` and, for the before-cells, `p5d/`:

```sh
python3 c6/gen6.py <dir>                                         # 190 cells
tr '\n' '\0' < c6/jobs.tsv | xargs -0 -n1 -P2 c6/cell6.sh > c6/raw.tsv   # K2_CORPUS_OUT overrides the output dir
c6/eval6.sh > c6/results.tsv                                     # verdicts, P compared per config
```

For X2/X5 on the implementing head, clone that head as `<dir>/p6`. The scripts take no other paths.

**Cells per configuration** (bash 5.3/3.2 × git 2.55/2.54):
- **G** (11): the hostile callers are
  - HOME `.config/git/ignore` `*.py`;
  - XDG attributes `filter=zap` with `clean=true`;
  - `GIT_CONFIG_GLOBAL` = <excludesFile, hooksPath, attributesFile+filter, gpgSign, defaultBranch>;
  - `GIT_CONFIG_COUNT` with each of excludesFile, hooksPath, gpgSign;
  - `GIT_TEMPLATE_DIR` with config / hooks;
  - the **DO cell**: `GIT_CONFIG_GLOBAL`=<`safe.directory=*`> plus `GIT_TEST_ASSUME_DIFFERENT_OWNER=1`;
  - `GIT_CONFIG_GLOBAL=/dev/null`;
  - caller `GIT_TRACE2_EVENT`.
- **R** (11 spellings at the `add -A` site, each under a HOME `*.py` ignore): `command git`, `env
  git`, absolute path, `eval "$_REAL_GIT"`, `hash -p`, `git -c core.excludesFile=/dev/null`, and
  round 5's `scrub3-sh`, `scrub3-absenv`, `hermetic-look`, `gitsweep` and `t2zero-countempty`. On two
  configurations, `gitsweep` and `scrub3-absenv` also run × every other hostile caller (+16).
- **RLOUD** (1): `_git`, which must be loud.
- **RES** (2): `env -i PATH="$PATH" git` and `env -u GIT_CONFIG_NOSYSTEM git`. The expected result is
  green with the clean P, and the residual is declared (§5.1).
- **P** (9): the records below.
- **INFO** (2): drop HOME; drop PATH.
- **BEFORE** (p5d = p5 + the same P-dump hook, two configs): a p5 clean reference, plus `scrub3-sh`,
  `hermetic-look`, `gitsweep` and `t2zero-countempty` × HOME `*.py` ignore.

**Results** (`c6/eval6.sh`; full table in companion §A.9):

| config | G (11, P equal) | R (P equal) | RLOUD | RES (P equal) | P records (9) | INFO: HOME drop / PATH drop |
|---|---|---|---|---|---|---|
| bash 5.3 · git 2.55 | 11/11 | 27/27 | red | 2/2 | 9/9 | red / red |
| bash 3.2 · git 2.55 | 11/11 | 11/11 | red | 2/2 | 9/9 | red / red |
| bash 5.3 · git 2.54 | 11/11 | 11/11 | red | 2/2 | 9/9 | red / **equivalent** (as predicted) |
| bash 3.2 · git 2.54 | 11/11 | 27/27 | red | 2/2 | 9/9 | red / **equivalent** |

The run covered p6 `863158b1` from 17:49 to 18:27 JST. `raw.err` is empty, and no cell hit a
harness error.

**The DO cell** (caller `GIT_CONFIG_GLOBAL`=<`safe.directory=*`> plus
`GIT_TEST_ASSUME_DIFFERENT_OWNER=1`) is green with P equal on all four configs, and it also passed
× `gitsweep` / `scrub3-absenv` on both FULL configs. So the read side still sees the caller's
configuration.

**Before and after, for round 5's Ax3 IMP-1** (both under a caller HOME `*.py` ignore):

| spelling | before: p5 (`p5d`), bash 5.3·git 2.55 and bash 3.2·git 2.54 | after: p6, all four configs |
|---|---|---|
| `scrub3-sh`, `hermetic-look`, `gitsweep`, `t2zero-countempty` | rc 0, `PASSED`, but P differs from p5's clean build by 50 dump lines: **silently wrong** | rc 0, `PASSED`, **P identical** to the clean build |

**Records: representative only, and all targeting the harness.**

| label | record |
|---|---|
| W | the child exits before sourcing the fixtures file |
| P-a | config appended to `_FGIT_ENV` |
| P-a liveness | the probe loses `-c a.b=c` |
| P-b | drop `GIT_CONFIG_NOSYSTEM` |
| P-b liveness | the probe loses `…NOSYSTEM=0` |
| P-c | plant a file in the void |
| P-d | drop `GIT_TEMPLATE_DIR` |
| P-e | inject `GIT_EXEC_PATH=/nonexistent-k2` (discriminates on both gits) |
| P-f | drop `-i` from the window's `env`, so it inherits |

That is **9 labels and 9 records**. `_MUT_TARGETS="wire harness"`: no `fixtures` target is needed,
since nothing that needs a record lives there, and no `controls` target is used. **`_MUT_UNRECORDED_MAX`
stays at 21**, and `_MUT_RECORDS_MIN` rises by exactly 9. RES cells are not records, because
survival would need the `!survive` needle, which the runner requires exactly once
(`mutations.sh:658–662`). The two INFO cells are informative, not records: HOME drop reds P-b, and PATH
drop is equivalent on git 2.54.

**Mutation-mode cost (X3).** Each run of X3 is (95 base records + 9) record trials plus the generated
population, one control pass each. X3 prints the counts, and X8 gives the per-pass time. This is
opt-in and does not add to the always-run gate.

## §7 What was deleted (draft 5 → 6)

- The four-channel detector: the `git` and `env` `PATH` shims, trace2 by env and by config route, and
  the config poison.
- The detector's canaries and its nine labels, and the records that pinned them.
- The per-call `_fgit` helper. Fixtures call `git`.
- The `fixtures` mutation target.
- Draft 5's slot scope ("a git started from nothing by an absolute-path program") and its claim that
  such a git "reads no caller input".

With the detector gone, round 5's Ax2 unpinned-restore IMP and Ax3's wrapper false-red (IMP-2) go
away, since there is no restore and no `PATH` shim. So do Ax3's M1–M3, which concerned `cmd_name` under
the poison and the detector's messages (companion §D.5).

## §8 Splits, the parent memo, and memo references

### §8.1 Code splits

`wc -l .claude/tools/webref-generic-core-trip-wire*.sh` at `e8f78896`: wire 1259, controls 989, mutations
783, harness 195. `ff6b99a3` broke CLAUDE.md's touch-time rule: its first commit took controls to 1026 lines
(`git show c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l`).

| file | split | seam | ground |
|---|---|---|---|
| controls | **C1** | build vs assert. `…trip-wire.fixtures.sh` takes **base controls lines 83–734**. Lines 735–737 are `ctl_ok=0`'s comment and stay with it. The window sources the new file | this PR grows both halves. `wc -l` of p6's parts is in companion §A.9 |
| mutations | **C2** | the header's two populations. The generated half moves to `…trip-wire.mutgen.sh` | `ff6b99a3` reached 983 lines with the infrastructure C3/C4 carry |
| harness | no | — | stays below the threshold (companion §A.9) |
| wire | **not touched** | — | `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` → `0` |

C1 and C2 each bring the following with them:
- one parts list that feeds the copy, the trap's `rm -f` and the stale-skip `case` in `_mut_run`;
- the sibling guard, which lives only in `_mut_run`;
- the extended entry contracts.

**In-file statements that C2/C4/C5 must rewrite in the same commit**, found by
`/usr/bin/grep -n -i 'every control\|a control means\|controls with no record\|controls have no\|_control. call\|umask and' .claude/tools/webref-generic-core-trip-wire.mutations.sh`:
- **:44–46**: "Every control has to be named by some record".
- **:120**: "ADDING A CONTROL MEANS ADDING A RECORD".
- **:136–138**: the non-`_control` labels named as umask and fsmonitor.
- **:672–673**: "controls with NO record … the third quoted argument of a `_control` call".
- **:684–688**: the ratchet messages, which say "controls".

Each of these becomes "labels": `_control` labels ∪ `_lbl="…"` definitions.

### §8.2 The parent memo: a banner, no split (C0b)

**There is no C0a.** CLAUDE.md requires a split only "real cohesion seam があれば".
- Parent §5 is live substance: §7 criterion 4 relies on it, §9 says "the substance is in §5 and §8",
  and so does header L8.
- §10 is cited by live §11 (`awk '/^### §11/{s=1} s' <parent> | /usr/bin/grep -o '§10\.[0-9]' | sort -u` →
  `§10.4 §10.8`).

**Finding the banner's sites.** The candidate lines come from a seed command, widened per round 5:

```sh
awk '/^#{2,3} /{sec=$0} $0 ~ /trip-wire\.(controls|harness|mutations)\.sh|controls file|control harness|mutation set|_fgit|GIT_TEMPLATE_DIR|D6|\*\*Status\*\*|over the wire|controls with no record|_MUT_UNRECORDED_MAX|ratchet|\.bare|mutation record|every control|unreachable or loud/ {printf "%d\t%s\n", NR, substr(sec,1,40)}' \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md | /usr/bin/grep -v '§10\.'
```

⚠ **The seed is a seed.** Each candidate is read and kept iff it is a **present-tense statement that
one of C1–C5 makes false**. A seed cannot return that population by itself: round 5 found L816 outside
draft 5's seed. At `e8f78896` the kept sites are **eleven**:

| parent line | why it becomes false |
|---|---|
| L5 | the Status |
| L61 | §0's command names the controls file, which loses the build half (C1) |
| L278–280 | §4's file rows (C1/C2) |
| L675–677 | §7 criterion 3, "enumerated in `…mutations.sh`" (C2) |
| L683 | §7 criterion 3, "a `sed` expression over the wire" (C4) |
| L746–747 | §7 criterion 5, "All are now in §4's table" (C1/C2) |
| L816 | Slot 2, "when every control has a mutation record" (C5: labels) |
| L866–868 | §9, "the number of controls with no record" (C5 population) |
| L1212–1214 | §11.1's rule "unreachable or loud, never silent", stated as the rule for inputs (§0.1 replaces it for the build) |
| L1220 | §11.1 D6 (§0.1) |
| L1282 | §11.4 P1's file list (C1/C2) |

### §8.3 Memo references in code

Every `§N` in the parts
(`for f in .claude/tools/webref-generic-core-trip-wire*.sh; do /usr/bin/grep -nE '§[0-9]' "$f"; done`),
plus every prose "the memo" / "plan memo"
(`/usr/bin/grep -n -i 'plan memo\|the memo' …`), is resolved by subject:

| file | lines | memo |
|---|---|---|
| wire | 4, 5, 192 (qualified); 38, 158, 195, 206, 225, 451, 461, 534, 550 (K2 predicate §2; exit criteria §12) | A-i memo (`2026-07-citation-hygiene-Ai-spec-label-map.md`) |
| wire | 307 (qualified); 199, 267, 323, 403, 456, 685, 724, 1164 (slots §8; revision §11) | parent |
| controls | 95, 116 (§2) | A-i memo |
| controls | 10, 273, 614, 906 (§8, §11 D10, §11.2, §11.1) | parent |
| mutations | 15 ("the plan memo"), 20 ("The memo") | parent (§10.5, §8) |

The wire is untouched, so this table is the record. In the files this PR edits, C1/C2 qualify every
reference with a file name, and **X4b** checks that **case-insensitively**.

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, the companion, the umbrella rows | docs | the round-6 focused review |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (eleven sites) | docs | the §8.2 command, re-read |
| C1 | controls → controls + fixtures; one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4, X4b |
| C2 | mutations → mutations + mutgen; references qualified | prereq split | X1, X3, X4, X4b |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | `_MUT_TARGETS="wire harness"` with resolver and restore | infra, required | X3 |
| C5 | the window (§3), with the fixtures calling `git`; `notcommitted`'s `mkdir`; §4's postconditions; the 9 records; the ratchet population; §8.1's in-file rewrites; the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

**Cost, and `ci.yml`.** The base job comment "a wire that adds fixture self-tests re-derives this line
in the same PR" is an in-file rule (`git show e8f78896:.github/workflows/ci.yml | sed -n
'/^  trip-wires:/,/^  [a-z]/p'`). C5 re-derives the line by that rule's own method, and records the
method and the verdict without figures. If the value would change, **STOP and escalate to the user**.
The budget half is unowned (parent §6; umbrella, Cross-lane coordination), and #510 is its other
claimant (`gh pr view 510 --json headRefOid`). There is still **one** build: it runs in a child
process, with the postconditions added.

**Land order:**
1. `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`.
2. User approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. `/external-converge`.
4. User approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella's single status cell.
7. Hand the #501 merge decision to the user.
8. After #501 lands, do the ledger step: in memory `project_open-defer-slots.md`, remove the dissolved
   slot, re-scope the invocation-convention slot to §5.1, and fix the header count. Cite #501's merge
   commit on `main`.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | the window's `env -i` closes every variable, and four relocations close default files. That holds for every git in the window, however it is spelled | §3; corpus G, R |
| 2 | A×D | the postconditions run in the window, so they describe every git there | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read; the verdict comes after `ctl_ok=0`; the child has no EXIT trap; nothing relies on `set -u` | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | producers live in the harness, labels in the controls file, and records target `harness` | §4, §6 |
| 9 | D | one label per producer, one record per label; the ratchet stays at 21 | §6 |
| 10 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it | §0.1, §5.2 |
| 11 | F | still one build, now in a child; the ci.yml line is re-derived by rule, and a change means STOP | §9 |

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | `( $SH $W ) > $S/l 2>&1; echo rc=$?; /usr/bin/grep -c 'CONTROL NOT EXERCISED\|CONTROL FAILED' $S/l; /usr/bin/grep -c 'trip-wire PASSED' $S/l` | `rc=0`, `0`, `1` |
| X2 | the §6 G cells (the recipe above, on the implementing head as `p6/`) | all PASS with P equal, 4 configs |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'` | three hits in every column. C1–C3 are byte-identical to base |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | in each file C1/C2 edit: `/usr/bin/grep -n -i -E '§[0-9]\|plan memo\|the memo' <file> \| /usr/bin/grep -v 'citation-hygiene-[A-Za-z0-9-]*\.md'` | empty |
| X5 | the §6 R, RLOUD, RES and P cells, same recipe | R and RES green with P equal; RLOUD red; P PASS |
| X6 | P-a/P-d planting (a template `config`+`HEAD`; a local `include.path`) | red |
| X8 | the ci.yml rule's derivation (§9) | method and verdict recorded; STOP on change |
| X9 | the PR's `Layering trip-wires` job (ubuntu, GNU) | SUCCESS. This is GNU evidence for `env -i`, the window and the prelude |
| X10 | `$SH -n` and `wc -l` over `ls .claude/tools/webref-generic-core-trip-wire*.sh` | clean; every part below 1000 lines |
| X11 | in a clone, add `_x_lbl="an unrecorded probe"` and an `echo` that uses it | red, and the ratchet lists it |

## §12 Questions for the round-6 focused review

- **Q1.** Are C1/C2 acceptable as standalone commits in this PR? *Draft answer:* yes (parent §11.7 row 7).
- **Q2.** Is the re-scoped invocation-convention slot acceptable, given that it is a fixture
  deliberately undoing an allowlist entry, and so visible in the diff by name? *Draft answer:* yes.
  Closing it needs per-process postconditions or a sandbox.
- **Q3.** Should `LC_ALL=C` stay in the allowlist? *Draft answer:* yes. The build ran under the wire's
  exported `LC_ALL=C` at base, and the window keeps it like-for-like. P-f names it explicitly.
