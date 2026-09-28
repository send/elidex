# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits and false premises. **Review
record**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`, every plan-review round's
dispositions and the terminators (split out of the companion unchanged). **Corpus**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` ("corpus §6"), the corpus cells and the
mutation records (split out of this memo's §6 unchanged). This memo holds only the live decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (`…-reviews.md` §8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 12**. Draft 10 closed plan-review (2026-09-27; `…-reviews.md` §D.0). Draft 11 revised
it after PR #527's external review paused (user decision, 2026-09-28), and draft 12 answers
plan-review round 10 on draft 11 (0 CRIT / 6 IMP / 22 MIN; dispositions in `…-reviews.md` §13). Three
enumerations become properties:
- **M-PATH**: `git` is resolved once, as the caller's shell resolves it, and pinned; every other `PATH`
  entry reaches the window verbatim. There is no normaliser (§0.1, §3).
- **M-SHAPE**: inside a git dir, anything that is not a regular file or a directory is red (§4).
- **M-WATCHDOG**: every wire run is one process group, and the running phase announces its bound (§3).
  Draft 11's per-child groups escaped a nested kill (round 10, U1).

Also:
- a red postcondition ends the run before any control (§3);
- W3's definition says what the code does (§3, §4);
- P-k closes an outside object store (§4).

The implementation is §9.1 and has not been made. The round history and the evidence are in
`…-reviews.md` §13, "PR #527".

Draft 10 answers round 9, a Step-4.5 focused check of draft 9 (`d5dacd56`): Ax2 0/1/1, Ax3 0/1/3. Both
IMPs were **implementation bugs in the prototype, not design defects**, and prototype **p11** fixes
them with their cells (corpus §6, companion §A.13):
- **A1: W3 failed open under SIGPIPE.** `grep … | head -3` under `pipefail` returned 141 on a long
  stderr, and `|| _fw_diag=""` emptied the diagnostic. It is now `grep -m 3`; only rc 1 means "none",
  and any other status is red (§3).
- **A2: the census pruned the declared unsearchable dirs**, so a nested repo under one, gitlinked into
  `walk`'s index with an outside `core.excludesFile`, passed. p11 opened them for the census and restored
  them; the implementation's `/simplify` pass replaced that with sealing after the census (§4, `…-reviews.md` §13).
- **A3 (MINs):** the git-dir shape is `HEAD` plus `objects` or `commondir`; "cannot search" is what
  `find` reports; the census forks nothing per directory; W2's record is described exactly; the W cause
  travels in a file the child writes, not in the exit status.

Draft 9's round-8 items (E1–E4) stand as written: the threat model (§0.3), the census over every git
dir (§4), `_control`'s W2 check (§3), W3 and the options re-check.

**Implemented** on this branch, C0b–C5 (`26e445fd` … `386febc8`); results in `…-reviews.md` §13, history in
companion §A.14.

**History.** The round-5 focused review failed its terminator. The root cause was that
drafts 4–5's bypass *detector* was itself a name list: a git that neutralised the watched names still
read the caller's HOME/XDG (companion §A.9, before-cells).

The design therefore changed (orchestrating session, 2026-09-27). P now holds **by construction of
the whole fixture-build window**. The detector, its four channels, its canaries, its nine labels and
its records are **deleted**. Every claim below is a result from the recast corpus run on prototype
**p6** (corpus §6).

(Draft 6 called round 6 "focused". It ran as a full five-axis review; `…-reviews.md` §D.0 records it as it
ran.)

⚠ **The parent's rule applies throughout.** No quantity here moves with a commit; each figure is a
measurement on a named artifact, given with its command.
- `$S` means a scratch directory.
- The real `HOME` is never written.
- `/usr/bin/grep` is spelled out, because the default `grep` on this machine is ugrep, which can
  silently return 0.
- Disposition labels in the companion (F…, R2-…, R3-…, U…, V…, W…, D…, E…) never collide with this
  memo's section numbers.

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

**`PATH` is on the R1 side of that line.** Which `git` runs is R1's, so it is decided **once, the
way the caller's own shell decides it**: the wire's bash resolves `git` from the wire's working
directory, with the caller's `HOME`, and the window runs exactly that file through the pin
`$_FGIT_BIN` (§3). Every other entry reaches the window **verbatim**, behind the pin, and nothing
reinterprets it. What such an entry means inside the window is outside P (§1, "Outside P"):
- a relative or empty entry means a directory relative to wherever the window's command runs;
- `~` and `~/…` expand against the window's void `HOME`;
- `~login/…` expands through the password database, not `HOME`, so it still names that login's home
  (`…-reviews.md` §13, the `~login` measurement).

A tool that only such an entry provided may then not be found inside the window. The window fails, and
that is red, the old rule's "loud". The caller's `git` itself is never lost this way, because it was
resolved before the window started. With `PATH='~+/bin:…'` and a `git` wrapper in the repository's
`bin/`, the wrapper **is** the fixture git under M-PATH: rc 0, PASSED, and every fixture git call goes
through it (plan-review round 10, U3, reproduced in `…-reviews.md` §13). That is R1 working as
stated. Draft 11 cited the base-only experiment PX1 here as "never a supported surface"; that described
base, where no pin existed, and it is history now, not the argument. What P needs from `PATH` is
one fact, that the window's `git` is the pinned one, and P-j checks it (§4).

The **read** side is also out of scope. It keeps the old criterion, because `_git` preserves the
caller's configuration on purpose (#501 R97). The window is a child process, so the parent's
environment, and with it the read side, is untouched (the DO cell in corpus §6).

### §0.2 One slice, and where it is registered

CLAUDE.md's edge-dense rule applies, because §10 has three or more axes. The umbrella carries **one
slice row** (A-i-wire-fgit) and **one memo-table row** naming this memo and its companion. Both are
pointers, and the umbrella amendment is itself under review.

The slice is property P for the fixture build: the window (§3), the postconditions that run inside it
(§4), and nothing else.

- **C1/C2** are standalone prereq splits (`…-reviews.md` §8.1).
- **C4** gives the mutation runner a `harness` target (corpus §6). It is required, because every record edits
  the harness.
- **C3**, record comments, is **not required** for writing records. It stands on its own: records that
  pin machine-dependent behaviour must say so at the record.

CLAUDE.md's clause "(feature PR に bundle しない — split は単独 PR / 単独 commit)" admits a
standalone commit; parent §11.7 row 7 is the precedent (`…-reviews.md` §D.2, row R2-6).

### §0.3 Threat model

The fixtures file is repository code, written and reviewed like the rest of the repository. Three
classes of input can reach the fixture build, and this slice treats them differently:

| class | what | owner | how it is handled |
|---|---|---|---|
| **(a)** | **implicit caller-environment leaks**: anything the caller's environment, home, XDG directory, system prefix or compiled-in defaults carry into git without the fixtures file asking for it | **this PR** (its purpose) | **closed** by the window (§3); pinned by P-a…P-f |
| **(b)** | **accidental fixture-authoring mistakes that persist an outside git input or leave the build incomplete**: a `git config` naming an outside file, an include appended to `.git/config`, an accidental `--separate-git-dir`, an arithmetic error that skips a line, a `set +e` left at the end | **this PR** | **caught, fail-safe on shape**: P-g over every git dir's configuration and shape (§4); W, with the options re-checked after the file; W3 (a shell diagnostic). An object store outside the repository (`objects/info/alternates`): P-k (§4) |
| **(c)** | **fixture code that deliberately evades** | **out of scope**: code review | not closed, not detected, not owed (§5.1) |

**What separates (b) from (c) is a property, not intent.** A class-(b) mistake leaves **persisted,
observable state** at the end of the build: a configuration line, a git dir of an unexpected shape, a
missing marker, an option switched off, a shell diagnostic. Class (c) is everything that leaves none.
Examples of (c), which illustrate the property and are **not a list to complete**:
- a transient per-command input: `-c`, `--config-env`, an environment assignment on one command,
  `--template`, an injecting `PATH` shim;
- persist-then-revert (write a config entry, run git, remove the entry);
- `--git-dir` / `--work-tree` pointing outside the fixture root;
- sourcing an outside file (`. <file>`);
- switching an option off and back on in the middle of the file.

A mistake that happens to take a class-(c) shape is not caught either, and this memo does not claim
otherwise.

**Why (c) is out of scope.** P is a function of the fixture script (§1). A script that names an outside
input has made it part of itself, and no in-process check can tell a deliberate transient input from a
legitimate one without re-parsing shell. Draft 7's seed S tried that and failed both ways (§5.1). The
fixtures file already has an owner for deliberate content: review of repository code.

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
- the build's non-git commands (which `mkdir` or `ln` runs is `PATH`'s, as R1 says of `git`: the
  caller's `PATH`, verbatim, behind `$_FGIT_BIN`, §0.1);
- reads at control time.

The corpus checks P **directly**: the recast cells compare every fixture's P to a clean build's P
(corpus §6).

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
- `git help gitrepository-layout` (`commondir`, `objects/info/alternates`)
- `git help count-objects` (`-v`, the `alternate:` line)

Webref does not index these. The corpus supplies the rest.

This follows A-iii's shape and the parent's §0.5/§3. The heading has **no table**, so `preflight.py`
fails **by design**. The marker is A-ii's §4.2.5 feature, which has not landed yet. The marker line satisfies
A-ii's recogniser, which is line-anchored, fence-aware and scoped to the Spec coverage map section. Its line pattern
is `^ {0,3}\*\*No spec surface\*\*` (A-iii Codex R13).

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
| (iii) **the whole build window from nothing**: `env -i` plus an allowlist around one child shell that sources the fixtures file | **closed for every git in the window whose inputs no fixtures-file command removed, overrode or added** (the §5.1 unit), however it is spelled, because each inherits only the window's environment. A command that does is class (b) or (c) (§0.3) | **chosen**; corpus §6 |
| (iv) OS sandbox | closed, including compiled-in paths | no portable mechanism; needs privileges |

Git's documentation supports these ingredients. `git help git` (`GIT_CONFIG_NOSYSTEM`) says it can be
used "along with $HOME and $XDG_CONFIG_HOME to create a predictable environment". `git help
git-config` (FILES) says "When the XDG_CONFIG_HOME environment variable is not set or empty,
$HOME/.config/ is used".

```sh
# harness at `8413a4db`, except the lines marked PLANNED, which C6/C7 (§9.1) change; the harness is the source
_FGIT_VOID="$SCRATCH/fgit-void"             # mkdir, checked
_FGIT_BIN="$SCRATCH/fgit-bin"                # holds `git`: exec <this shell's `type -P git`, absolute>
_FGIT_PATH="$_FGIT_BIN:$PATH"               # PLANNED (C6): the caller's PATH verbatim, behind the pin (§0.1)
_FGIT_ENVBIN=<this shell's `type -P env`, absolute>; _FGIT_BASH="$BASH"
_FGIT_ENV=("PATH=$_FGIT_PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID" GIT_DEFAULT_REF_FORMAT=files "LC_ALL=C")
# _fgit_window: write a prelude of PLAIN assignments (labels by name) and function bodies, then
_k2_phase "$_FW_TIMEOUT" W "$_fw_lbl"       # PLANNED (C7): announce the window's bound (below)
"$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" "$_FGIT_BASH" -c '
  cd "$1" || <cause; exit>                     # $_FW_DIR, a directory the window owns
  . ./prelude.sh                                # both copied here by fixed relative names (R2)
  _fw_opts_on || <cause; exit>                 # errexit, nounset, pipefail in force
  . ./fixtures.sh                               # the fixtures file (last line writes `built`)
  [ -e "$_FW_DIR/built" ] || <cause; exit>; _fw_opts_on || <cause; exit>
  _fgit_postconditions || : > "$_FW_DIR/post_bad"
  _seal_apply                                   # mode restrictions, after the census
  printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
  : > "$_FW_DIR/done"' _ "$_FW_DIR" 2> "$_FW_DIR/stderr" || _fw_rc=$?
```

**What the child gets, kept minimal.** The prelude contains:
- `set -euo pipefail`;
- plain `name=%q` assignments. These cover the data the fixtures file reads (`CTL`, the five
  `CONTROL_*` samples, `_REAL_GIT`, `_REAL_GREP`, `_fifo_ok`) and the window's own `_FGIT_VOID`,
  `_FGIT_ENVBIN`, `_FGIT_ENV_NAMES`, `_FGIT_WIRE_EXEC`, `_FGIT_WIRE_LC`, `_FGIT_BIN` (P-j) and `_FW_DIR`, and the
  postcondition labels, passed by name rather than as positional arguments: the names are the ones
  `_fgit_postconditions`' body uses, each of which the controls file (where the ratchet counts them) must
  define non-empty, or the window refuses. The fixtures' part of the list is **derived** by the census
  command below;
- `declare -f` bodies of `_fixture_failed`, `_shq`, `_fgit_canon`, `_fw_opts_on`, `_seal_refuse`,
  `_seal`, `_seal_apply` and `_fgit_postconditions`. The child first enters `$_FW_DIR`, a directory it owns.

**The census, and why it is complete.** The read-set comes from:

```sh
awk '!/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.fixtures.sh \
  | /usr/bin/grep -oE '\$\{?[A-Za-z_][A-Za-z0-9_]*' | tr -d '${' | sort -u
```

On p7 this gives 19 names:
- nine that are assigned inside the fixtures file itself: `d`, `_c`, `_sha`, `_d3i`, `_d3c`, `_mc`,
  `_good`, `_bad`, `_br`;
- `GIT_CONFIG_COUNT`, which appears only inside a single-quoted shim body that the build writes out
  and never runs;
- the nine prelude names.

So the prelude is complete **by static read-set** today. Completeness does **not** rest on `set -u`,
because a `${X:-}` read would be silent. What `set -u` adds is a runtime backstop, and it is
**pinned**. The child refuses to go on unless `errexit`, `nounset` and `pipefail` are all in
`$SHELLOPTS` after the prelude, and it stops with W's "refused to start" cause otherwise. The W record "prelude without
`set -euo pipefail`" goes red with W alone on both shells (companion §A.10).

Assignments are **plain**, never `declare -p`. `declare -p` carries the `export` attribute into the
window; P-f caught that in p6's first run (companion §A.9).

**What comes back.** Three things return through files: `_FIX_FAILED`, the fact of completion
(`done`), and whether any postcondition reported. The window's own `CONTROL …` lines reach the wire's
stderr directly. The fired marker (below) is written by the run's timer, outside the group.

**An incomplete window is not "no fixture failed" (round-6 item 1; D4).** Completion needs two
markers:
- **`built`**: the fixtures file's **own last line** writes it. So a top-level `return` or `exit` inside
  the file leaves the window incomplete, and "the child script finished" is not enough.
- **`done`**: written after the postconditions run.

Suppose either marker is missing. Nothing was built, so **no `_control` runs**.
`_fgit_window_incomplete_exit` prints W alone with the recorded cause and **exits 2**.

The cause travels in a file, `cause`, into which the child writes the **sentence W prints**, just
before it exits; the exit status carries nothing. A fixtures-file command can end the child with any
status under `errexit` (`sh -c 'exit 5'` does), so a status would name the wrong cause (round 9, Ax2).
The child writes one of three sentences:
- the window refused to start, because a prelude option was not in force;
- the fixtures file returned before its last line;
- the fixtures file switched off `errexit`, `nounset` or `pipefail`. One helper, `_fw_opts_on` (one
  `case` per option, so no pattern depends on the order `$SHELLOPTS` lists them in), is asked before
  **and after** the fixtures file (SE1). Switching an option off and back on mid-file leaves nothing to
  observe, so that is class (c).

With no `cause`, W says the window exited with its status before completing: an `exit` in the fixtures
file (the fixtures' own `exit 2` reads "the window exited 2 before completing"), an abort, or an aborted
postcondition.

A parent-side failure to create the window directory or write the prelude has its own reason text.

**Every wire run is one time-bounded process group (M-WATCHDOG).** A run can block on what the
fixtures built. Examples, which show the property and are not a list: git opening a FIFO that a
persisted `include.path` names outside the fixture root (no shape check inside a git dir can see it,
§4); a census into a huge tree; a control whose tree holds a FIFO. A blocked gate reaches no verdict.
Draft 11 gave each child its own process group, bound by a list of call sites. Plan-review round 10
measured both halves failing. A group nested in another escapes the outer kill: a trial whose inner
bound was broken hung past 20 s on both shells, and `$(…)` waited on the pipe (U1). The list also
missed a call site: the fsmonitor block's plain `git ls-files` (U2). So the bound is by
**membership**, not by list:
- **One group per run.** When the controls file is sourced by a wire that is not inside a bounded group
  (below), it runs the **same wire, with the same arguments,** as the leader of a new process group
  and waits for it. That child runs everything from the harness's setup through the controls to the
  scan of the repository. Every process it starts is in the group unless it re-groups itself, and
  after C7 nothing does: `set -m` appears in one function, `_k2_group`, which runs only where no group
  is in effect (here, and the mutation runner's trials). An always-on check in `_mut_correspondence`
  keeps it so: an uncommented `set -m` anywhere in the wire's parts outside `_k2_group` is red.
- **Each phase announces its own bound.** Before each unit of work, one line goes to the group's phase
  file: bound, start time, kind, label. The units are the harness setup (S), the window (W,
  `_FW_TIMEOUT`), the window's verdict (S), each `_control` and each non-`_control` block (C, with its
  label), and the scan of the repository (S). A call site nobody announces runs under the phase
  before it. It is bounded all the same; what it can lose is attribution, never the bound.
- **One timer per group**, outside it (forked after `set +m`), reads the phase file once a second. When
  the current phase has run past its bound, or the group past its cap (0 = none), it writes the
  **fired marker** (the phase line, and which bound fired) and kills the whole group
  (`kill -9 -<pgid>`). The parent reads the marker, never the status, because a child can exit 137 by
  itself. It prints by kind:
  - W: NOT EXERCISED with W's label, exit 2;
  - C: CONTROL FAILED with that control's label, exit 1. This keeps #501 R92's contract ("a control
    whose run blocks is red with its label"), but the run ends there;
  - S: decided nothing, exit 2.
- **Nothing the run started outlives it.** After the leader exits, the parent kills the group anyway
  (ESRCH if it is empty), then stops the timer. The timer also exits by itself once the leader is gone
  or its directory has vanished. The parent's exit path (EXIT, INT, TERM) kills the group and the timer
  before the wire's scratch cleanup. That handler is composed with the wire's EXIT trap, read by
  `trap -p EXIT`, because `trap` replaces.
- **Fresh per invocation.** Each `_k2_group` creates its own directory (phase file, pgid file, fired
  marker) and refuses one that already exists.
- **Nesting only by the mutation runner, and only where no bound can fire.** Each trial runs as its own
  group under cap `_MUT_TRIAL_CAP`. Meanwhile the enclosing run's phase is unbounded (0), so the
  enclosing timer cannot fire while a trial group exists. The trial's wire finds itself inside a
  bounded group (`_K2_PHASE` names a phase file whose pgid file holds its own process group) and
  **joins**: no second run, no group, no timer. Its phases announce to the trial's phase file, and the
  runner's timer enforces them and the cap. So a mutated harness that announces a huge bound, or none,
  still ends at the cap. `_k2_group`, `_k2_joined`, the timer and `_k2_phase` live in the controls
  file, which no record edits (`_MUT_TARGETS` is harness and fixtures, plus the wire).
- **A trial whose group was killed** gets the verdict line the parent would have printed (from the
  marker), appended to its output. It is then judged by the record's needle like any other kill; a cap
  kill names the phase that was running.

The cells, on bash 5.3 and 3.2, prototype in `…-reviews.md` §13:
- (a) a window blocked on a FIFO gives W alone, rc 2, at 90 s;
- (b) a control whose run blocks gives CONTROL FAILED with its label, rc 1, at 35–38 s;
- (c) a trial whose window blocks returns at 90 s with the W label, and returns at its cap (120 s in
  the cell) when the window's bound is broken;
- (d) a clean run is PASSED;
- (e) its wall time stays within the run-to-run spread of draft 11's prototype.

None of them left a process behind.

Outside the group is only the wire's own code before it sources the controls file: arguments,
scratch creation and its EXIT trap (§5.2 R9).

**The bounds, derived by one rule.** A bound is the slowest measured wall time of the unit × 4.3 × 2,
rounded up to a multiple of 30 s:
- 4.3 is the slowest runner-to-local ratio this repository has recorded: PR #510's trip-wires comment,
  49 s locally against 211 s on Codex's runner (`git show cc05c428:.github/workflows/ci.yml`);
- 2 is headroom for variance.

The measurements are in `…-reviews.md` §13, with the command:
- **`_FW_TIMEOUT=90`**: the window, 7.27 s at most (draft 11's prototype, bash 3.2), gives 62.5;
- **30 s** for every other phase: the harness setup (0.61 s at most), the slowest control or block
  (1.12 s, the phase that runs the umask block and everything up to the scan), and the scan
  (1.53 s). #501 R92's 30 s is the floor;
- **`_MUT_TRIAL_CAP=330`**: `_FW_TIMEOUT` plus the rule applied to the slowest whole run (25 s gives
  240), so a trial that legitimately runs its window to the bound still finishes.

The controls summary prints the window's wall time in whole seconds, so X9 and any later re-derivation
read it from an ordinary run.

**`ci.yml`.** `timeout-minutes` does not move: a clean run costs no more, and the longest bounded
failure (the window, 90 s) ends inside the 5-minute budget. The trip-wires job comment's verdict
paragraph, which C5 replaced with the method and one verdict line, names the tool code of the squash.
After C6–C10 that line is stale until X8 re-runs at the final head and rewrites it (§9).

**A red postcondition ends the run before any control.** A red P-a…P-j, or W3, means the build is not
the one the fixtures file describes. A control over it asserts nothing, and it can block. On the
M-SHAPE prototype, a FIFO `.git/commondir` made P-g red, and then the first `_control` was killed at
30 s (`…-reviews.md` §13). So the window's verdict is reported in **one place**, and the run then
continues only over a build that is **complete and trusted**:
- incomplete: W alone, exit 2 (above);
- complete, but a postcondition or W3 reported: those reports, then exit 1. A red is a decided verdict;
- otherwise: the controls run.

W4 does not stop the run. A seal that failed marks its fixture failed, `_control` already refuses that
fixture's controls, and the other fixtures are sound. The gate `_control` asks first,
`_fw_built_or_w2`, asks the same question, "complete and trusted", so no control runs over such a
build wherever the exit sits.

**Why exit 2, and why it is not pinned.** Exit 2 follows the wire's own convention that 2 means
"decided nothing". Nothing consumes the distinction between 1 and 2, though: the driver runs
`bash "$w"` under `set -euo pipefail` and aborts on any non-zero status
(`git show e8f78896:scripts/trip-wires.sh`, lines 22 and 204). The mutation runner's kill test is
"rc ≠ 0 plus a needle" (`_mut_trial`), and it cannot see the number either. So the number is **not**
pinned, and draft 7's "2 stays 2" / "carry the child's class" claims are withdrawn.

What **is** pinned are the properties a record can observe:
- **W** fires when the window is incomplete, including when it timed out.
- **W2** fires if any control is reached over a build that is not complete and trusted. W2 is `no
  control runs over an incomplete or untrusted fixture build window` (draft 10's label said
  "incomplete" only). Its predicate is one function, `_fw_built_or_w2`. **`_control`
  calls it as its first statement** and refuses unless the window is complete and trusted (E3), and each of the
  three blocks that are not `_control`s (relative scratch, fsmonitor, umask) asks it before it runs.
  So no control runs over such a build, wherever the exit sits. **Its two records pin
  that W2 is reported when the exit is gone**: one removes the incomplete-window `exit`, the other the
  untrusted-build `exit` with a postcondition forced red. `_control`'s gate alone satisfies both. ⚠ **Declared gap:** removing
  one of the three block gates survives the mutation set (W2 is already reported by the first
  `_control`); those gates are pinned by the traced `w2rec` cell (companion §A.14), not by a record.
- Moving the exit below a control is an edit to the controls file, which is not a mutation target. That
  shape is pinned by the corpus RO/ROg cells (X5), not by a record.
- **W3** fires when bash reported a diagnostic located in the fixtures file (below).

**A shell diagnostic in the fixtures file is red (W3; E4, AR).** An arithmetic-expansion error such as
`$(( 1/0 ))` at the fixtures file's top level does **not** stop a sourced file under `set -e`, on either
shell. The build completes, and on p8 the run was green (corpus §6). The child's stderr is captured to a file
and replayed. **Any stderr record that contains `./fixtures.sh:` or `./prelude.sh:`, anywhere in it,** is
W3. The window sources both files from its own directory by those relative names (the fixtures file is
copied there first), so the name does not depend on the checkout path, which could otherwise hold a
newline that splits it across two records. The match is not anchored at the record's start: a
fixtures-file command that writes to stderr without a final newline makes bash append its diagnostic to
that record (`x./fixtures.sh: … division by 0`). A fixture that writes one of those names to stderr
itself is red too, which is the fail-safe direction. That bash names the file it reports on is a
property of the shell, not a wording: it covers `…
line N:`, 3.2's `… command substitution: line N:` and `… eval: line N:`, in any locale. (Draft 10
matched only `<fixtures path>: line N:`, which missed the other forms; `/code-review`, `…-reviews.md` §13.) The scan is
`awk` over the file, stopping at three lines: **no pipe into `head`**, which under `pipefail` SIGPIPEd
grep (141) and read as "no diagnostic" (round 9, Ax2). A non-zero `awk` status is itself red. It was
measured on bash 3.2 and 5.3 on macOS; X9 confirms GNU bash. It is class (b) by the §0.3 property: a skipped line leaves a
diagnostic.

The before/after cells are in corpus §6.

**Two harness comments change in C5 (E4).** Both were measured on the prototype:
- The window comment said every git the fixtures file starts "however it is spelled" inherits only this
  environment. It now adds "unless a fixtures-file command itself altered its inputs (memo §0.3 classes
  b/c)".
- The incomplete-exit comment said the run ends "with the child's exit class". It now says "decided
  nothing (2, the wire's convention; the number is not consumed by the driver)".

**Every git in the window is equal.** The fixtures file calls **`git`**, and the per-call `_fgit`
helper is **gone**. That is one issue, one way. Measured: after the split, no `_fgit` use remains
outside the window. The postconditions, the only other caller, now run inside it.

**What each allowlist entry buys.** Measured on git 2.55.0 and Apple 2.54.0; each is also pinned in
corpus §6.

| entry | without it |
|---|---|
| `PATH=$_FGIT_PATH` | BSD `env -i` runs `/usr/bin/git` rather than `PATH`'s git (INFO cell). The first entry is `$_FGIT_BIN`, a wrapper that execs the `git` **this shell's own lookup** resolves. The lookup is `_fgit_resolve`, the one resolver for every command that crosses the window boundary by path, `$_REAL_GIT` and `$_REAL_GREP` included. After it comes the caller's `PATH`, **verbatim**. No entry is reinterpreted, dropped or emulated, because which executable runs is R1's (§0.1), and what P needs is only that the window's `git` is the pinned one. An entry means whatever it means from inside the window (relative, empty, `~`: §0.1). Since `git` is pinned, that decides only which other tools are found, and a missing one fails the window: red. The fixtures' other commands come from these entries (§1, "Outside P"). P-j pins the construction. History: `…-reviews.md` §13, PR #527 |
| `HOME=$VOID` | an unset `HOME` also closes this on 2.55. The void is chosen because a future HOME-relative default then lands where P-c looks |
| `GIT_CONFIG_NOSYSTEM=1` | the system layer is live here (`/opt/homebrew/etc/gitconfig`) |
| `GIT_ATTR_NOSYSTEM=1` | undocumented at 2.55: 0 hits in all 207 man pages of 2.55.0 (command below; positive control: `CONFIG_NOSYSTEM` hits 2 pages). So it is pinned by `git var` (P-b) |
| `GIT_TEMPLATE_DIR=$VOID` | the compiled-in template is copied in |
| `GIT_DEFAULT_REF_FORMAT=files` | a git whose compiled-in default is reftable (git 3.0's planned default, or a breaking-changes build) makes every init differ from every other and `badref` write refs reftable does not read (R8). Gits before 2.45 ignore it. Pinned by P-i |
| `LC_ALL=C` | keeps the window's locale the one the wire exports (wire:319), so the build is like-for-like with base. Pinned by P-h |

```sh
find -L /opt/homebrew/opt/git/share/man -type f -exec /usr/bin/grep -l ATTR_NOSYSTEM {} +; echo "rc=$?"   # rc=1 (207 pages)
```

**The fixture change.** `notcommitted` gains `mkdir -p .git/info`, because the empty template no
longer creates it (companion §A.2).

**errexit and state.** Every window state name (`_fw_rc`, `_fw_done`, `_fw_post_bad`, `_fw_why`) is
assigned at harness top level, before anything reads it (`_fw_diag` and the W2 flag included). The
child runs under `set -euo pipefail` with **no EXIT trap**. Each of those options is checked separately
before the fixtures file (one record per option, corpus §6), and all three are checked again after it (one
record). So an abort there
is a non-zero exit, which the parent reads as data. The verdict is written after `ctl_ok=0`
(`controls.sh:738` at base), and a window that is not complete and trusted ends the run right there.

The bound (C7) adds parent-side state, assigned before it is read in the same way:
- in the parent that starts the group: `_k2_rc`, `_k2_fired`, the leader's and the timer's PIDs, and the
  group's directory;
- in the run: `_k2_join` and the exported `_K2_PHASE`;
- the constants `_FW_TIMEOUT` and `_MUT_TRIAL_CAP`, and the 30 s phase bound.

The timer is a separate process with no `set -u` dependence: every variable it reads is assigned
first in its own body.

**The parent's reliance on `set -u`, measured (IMP-2).** With the wire's `set -euo pipefail` changed to `set -eo pipefail` (nounset OFF in the parent), the
clean tree and eight red cells (sealfail, env0, garbagehead, w2rec, lblrename, w3ar, sealdotdot,
reftable) gave the same exit status, the same NE/CF counts and the same first four `!!` lines as with it on:
- on bash 5.3 at the `/elidex-review` head (companion §A.14);
- **on bash 3.2 and 5.3 at `8413a4db`**, 36 runs. Bash 3.2 is where it matters: there
  `#11-k2-wire-exit-trap-masks-set-u-abort` turns an unbound-variable abort into rc 0.

The cell script is verbatim in `…-reviews.md` §13 ("X13"), and X13 re-runs it at the final head. That
is the measured claim: on those runs, no parent-side verdict depended on `set -u`. It is
not a proof over every path, and the "every state name is assigned before it is read" argument is a
reading of the code, not a measurement. The child's `set -u` is a backstop
behind the static census, and it is pinned by its own record. See §5.2 for the pre-existing defect.

## §4 Postconditions — run INSIDE the window

Every git in the window whose inputs **no fixtures-file command altered** inherits the same
environment and the same persisted state. So git's own answers inside the window describe every such
git (§5.1 covers the rest). Each producer has its own label (defined in the controls file and passed
in), and each label has its own record (corpus §6).

| id | label | assertion (inside the window) | liveness (its own label) |
|---|---|---|---|
| W | `the fixture build window completed` | the fixtures file's last line wrote `built`, the options were still on after it, and the child wrote `done`. Otherwise NE with the sentence the child wrote to `cause`, the timeout sentence the parent prints from the fired marker (§3), or the exit status (§3), **reported alone**; exit 2; no control runs | — |
| W2 | `no control runs over an incomplete or untrusted fixture build window` | `_control`'s first statement, and the first thing each non-`_control` block asks: the window is complete, and no postcondition or W3 reported. Its records: each of the two exits removed (§3). The exit placed below a control: the RO cells | — |
| W3 | `the fixtures file ran without a shell diagnostic` | no record of the child's stderr contains `./fixtures.sh:` or `./prelude.sh:` **anywhere in it** (both are sourced by those relative names from the window's directory; bash appends a diagnostic to a record the fixtures left without a newline, §3). A failed scan is red | — |
| W4 | `every mode restriction a fixture sealed was applied` | every `_seal` was accepted (a path under `$CTL`, with no newline or TAB) and its `chmod` succeeded; otherwise red, not a machine limitation | — |
| P-a | `a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file` | every `git config --list --show-origin` line in **one probe repo** has the origin `file:.git/config` | `this git reports a configuration origin outside the repo's file`: `-c a.b=c` must show with the origin `command line:`. `--show-origin` (git 2.8) alone, not `--show-scope` (git 2.26): the origin column answers the same question, so the check has no version floor and no limitation arm |
| P-b | `the fixture git has no system or global layer outside the void` | asked **inside P-a's probe repo** (`git -C`), so no caller repository's local configuration is read. A git older than 2.42, whose `git var` cannot name these four (exit 129), is a machine limitation: `⚠ NOT EXERCISED on this machine`, green. Otherwise `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is an empty directory, tested by the shell's own globs (`*`, `.[!.]*`, `..?*` with `-e`/`-L`) — not by `$(ls -A …)`, which loses a name made only of newlines and reads a failed `ls` as empty. A void that is not a directory, or cannot be read and searched, is red: globs over it would expand to nothing. The globs run with `set +f` and `GLOBIGNORE` unset, whatever the fixtures file accidentally left (a deliberate `readonly GLOBIGNORE` is threat-model class (c)). Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `no exec-path override reaches the fixture git` | the window's `git --exec-path` equals the same git's answer with nothing but `PATH` in its environment (taken in the parent at source time), both canonical (`pwd -P`). **Re-scoped by `/code-review`:** which executable runs is outside P (§0.1, R1); P-e pins only that nothing in the window overrides where that git runs its commands from. So a caller's `GIT_EXEC_PATH` or `DEVELOPER_DIR`, or one directory spelled two ways, is not a red | — |
| P-f | `the fixture build window's environment holds only its allowlist` | the name of **every** `env -0` record (the text before the first `=`, so non-identifier names such as `BASH_FUNC_f%%` are included) is an allowlist name (derived from `_FGIT_ENV` itself, so the two cannot drift) or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction. p6's `sed` parser skipped non-identifier names; p7 parses every record (companion §A.10). `env -0` goes through a file whose status is checked: a failed `env -0`, or no record at all, is NOT EXERCISED. **One** outcome is a machine limitation instead (`⚠ NOT EXERCISED on this machine`, green — the treatment P-b and the FIFO and file-permission controls give a machine that cannot run them): its test is the definition — **this** `env` runs but refuses `-0`: `env -0` fails, writes nothing to stdout and says why on stderr, while the same `env` runs a command. Every other outcome stays red, so an unknown one falls on the fail-safe side. Which `env` builds lack `-0` is not measured here (this machine's macOS 26 `env` has it; PR #527 Codex R4 reported a macOS one without it); the no-`-0` case is exercised with a shim. Both limitations reach the run's summary through one channel (`$_FW_DIR/machine_limits`), naming the postcondition by ID only, since a record's needle is its label. History: `…-reviews.md` §13, PR #527. | — |
| P-g | `every fixture repo persists only the configuration a plain git init writes` | for **every git dir under the fixture root** (population below), the lines of `git -C <repo> config --list --show-origin` (no `--show-scope`: the origin already names the file) **equal, as a set,** the lines of a **reference** `git init` made in the same window (P-a's probe repo, which is that init), compared by origin, key and value (`grep -vxF -f`, both directions). An extra line is an input the fixture persisted; a missing one (`git config --unset core.filemode`) hands that setting to the platform default — either way P would depend on more than the fixture. A `grep` that fails (exit above 1) is a failed comparison, red — never an empty difference. The authoritative comparison is of **records**: the `-z` listing, each origin paired with its entry into one record written as one line by `printf %q`, sorted (no `sort -z`), byte-identical to the reference's — so a value holding a newline cannot forge a line, and a repeated entry is red. This catches a persisted include (its origin is not `.git/config`), a `commondir` naming another git dir (the origin names that dir's config), and **any** persisted key beyond init's, `core.excludesFile` included. **Shape first (M-SHAPE):** before git runs on a `.git`, every entry under it that is not a regular file or a directory (`find <.git> ! -type f ! -type d`: a symlink of any target, a FIFO, a socket, a device) is red, and git is **not** run on that repo. Git reads through a link (a `.git/config` pointing outside still reports `file:.git/config`) and blocks opening a FIFO, whatever the entry's name. A scan that fails or writes to stderr is red. The census takes `HEAD` and `.git` entries of **any** type and letter case — a symlink `HEAD`, or `head` on a case-insensitive filesystem, is a shape git reads — and every git dir not reached by a `.git` entry is red. **Both directions:** a git dir whose listing is EMPTY or fails (a `.git` git does not recognise, e.g. a garbage `HEAD`) is red | — |
| P-i | `the fixture repos use the files ref format` | the window's `GIT_DEFAULT_REF_FORMAT` is `files`, and a plain init's `git rev-parse --show-ref-format` answers `files` (a git before 2.45 has no reftable; its `rev-parse` echoes the unknown option back with exit 0, and that literal echo is what counts as `files` — a failed call or any other format name stays red). It pins the allowlist entry on every git, including the files-default ones where dropping it changes nothing else | — |
| P-j | `the fixture build window runs the pinned git first on its PATH` | the window's `PATH` starts with `$_FGIT_BIN`, and `type -P git` inside the window is `$_FGIT_BIN/git`. It pins the `PATH` construction the way P-h and P-i pin their entries: without the pin, the window's first `git` is the file the pin execs anyway, so nothing else changes, and P-e's reference moves with it. It asserts nothing about the caller's entries (§0.1) | — |
| P-k | `no fixture repo reads objects from a store outside it` | for every `.git` P-g compares, `git count-objects -v` exits 0 and prints no `alternate:` line. An `objects/info/alternates` naming another store makes git read an object from there instead of writing it locally, so the content the controls read is an outside input even though P's ids do not move (measured, `…-reviews.md` §13). Git's own answer, not a file name: it reports the stores git will read (measured for the `alternates` file). A failed `count-objects` is red | `this git reports alternate object stores`: a third probe repo (so P-d's two inits are untouched), given an `alternates` file naming P-a's object store, must print one `alternate:` line; if it prints none, that is a machine limitation (`⚠ NOT EXERCISED on this machine`, green; `machine_limits` line `P-k — …`), as for P-b |
| P-h | `the fixture build window reads in the wire's locale` | the window's `LC_ALL` equals the wire's (`C`). It pins the allowlist's `LC_ALL=C`, which P-f cannot, because P-f derives its names from the same list | — |

**How P-g's key set is derived (D2).** It is **not** a list of path-typed keys. Git exposes no per-key
type: `--type=path` is chosen by the caller, not declared per key, and `git help config` is prose. A
list of path keys would therefore be vocabulary, and a key missing from it would pass silently.
Instead, the allowed set is **what git itself writes on `init`**, measured in the window on every run.
That is the fail-safe direction: any persisted entry beyond it is red, whether or not it names a path.

**P-g's population, by property (E2; A2, A3).** It is every git dir the fixtures produced. One `find`
from `$CTL`, with no per-directory fork, lists every `.git` entry and every `HEAD` entry, **of any type and in any letter case** (a
case-insensitive filesystem lets git read `head` and `.GIT`; a case variant is red), and every symlink:
`find` does not follow links, so a link that resolves to a directory is searched through (`find -L`, any
depth) whatever its name, and a `HEAD` or `.git` anywhere under it is red, as is a search that exits non-zero
(BSD find lists a loop link without descending, rc 0, hiding nothing; GNU find is expected to exit 1 — not
measured here). A link *named* `HEAD` to a directory is red whatever it holds, since the search's start
matches its own name; the record for that shape therefore pins the routing to the search, not its depth
(the two-levels-down record pins the depth). The census runs inside the window, so the window's phase bound covers it (§3): a search into a huge tree ends as W with the timeout cause. It descends everywhere: hidden and nested directories, and the inside of `.git` directories.
- A `.git` that is a real directory has its **shape** checked (every entry under it a regular file or a directory, table above), and then its configuration compared. So a link inside a `.git` dir is red by shape, whatever it points at and whatever its name.
- A link elsewhere that does not resolve to a directory is classified by its name: `.git` or `HEAD` as below, anything else passes (it is not part of a git dir).
- A link **inside** a `.git` dir is not searched through (the per-link `find -L`). The shape rule already reds it, and searching behind it could turn a huge tree into a window timeout (W) instead of P-g (round 10, Ax2). The top-level census does not follow links (no `-L`); only the per-link search does.
- A `.git` that is a gitfile, a symlink or anything else is red: `[.git is not a directory]`.
- A `HEAD` whose directory is not a `.git` and holds an `objects` directory **or** a `commondir` file is
  a git dir not named `.git`, and it is red. That covers a bare repo, a `--separate-git-dir` target, a
  submodule's git dir under `.git/modules/`, and a linked worktree's entry under `.git/worktrees/`.
- **"Cannot search" is whatever `find` reports.** Any report on stderr, or a non-zero exit, fails the
  census, and a failed census is red. So is one that finds no git dir.

**Nothing is unsearchable at census time, by construction (sealing after the census).** Three
fixtures need a mode restriction: `walk/sub` and `d5root` mode 000, `d2red/sub` mode 0444 (and the file
`err/control.py` mode 000). They ask for it through `_seal <path> <mode> <fixture>`, which only appends
to a manifest in the window's directory — the path relative to `$CTL`, and refused (red, W4, and the
fixture marked failed) if it lies outside `$CTL`, holds a newline or TAB, or has a `.` or `..`
component; `_seal_apply` also refuses a path with a symlink anywhere on it (`chmod` follows symlinks),
so no seal can reach outside the scratch root, and it skips a fixture whose chain already failed, so
one failure is reported once; the window applies the manifest **after** the postconditions,
and a `chmod` that fails marks that fixture failed **and is red under W4**: the controls gated on the
mode having taken effect would otherwise be skipped as a machine limitation. So the census
reads the whole tree, and a directory it cannot read is a restriction nobody sealed: red. Draft 9
**pruned** these directories, and a nested repo under `walk/sub`, gitlinked into `walk`'s index with an
outside `core.excludesFile`, passed with P differing (round 9, Ax3); p11 opened and restored them
(companion §A.13). Sealing replaces the declared list, the mode check and the open/restore loop.

An accidental `--separate-git-dir` is class (b): it leaves a gitfile, so it is red.

Draft 8's p8 walked `$CTL/*/` and skipped any fixture whose `.git` was not a directory. A
`--separate-git-dir` with a persisted `core.excludesFile` was **silently wrong** there (rc 0, P
differs). This author's intermediate p9 still walked only the top level, and it missed a nested repo
and a hidden one. Both were measured (corpus §6).

- **Census:** today every fixture git dir's configuration equals the reference on both shells, `find`
  reports nothing, no fixture git dir holds anything but regular files and directories (the M-SHAPE
  prototype's clean run is green on both shells, `…-reviews.md` §13), and the three declared directories are at their modes, so the clean and G cells
  are green (corpus §6). Legitimate fixture state (`badref`'s ref, `notcommitted`'s `info/exclude`, the
  per-call identity) is not configuration, so it stays green.
- **Cost:** measured as the census section's wall time on a clean tree (instrumented copies, companion
  §A.13): **0.76 s** on p11 against 1.45 s on p10, on both shells. Most of p11's figure is the one
  `git config --list` per fixture git dir. p10's extra time came from one `sh -c` per directory, which
  mutation mode would have paid about 115 times. X8 measures the whole cost.
- **What P-g cannot see:**
  - class (c) (§0.3): transient inputs, persist-then-revert, a git dir outside the fixture root;
  - **anything outside configuration and shape**, which is P-g's scope; the *content* of a regular
    file inside a git dir is not compared. Examples, not a list: hooks, `info/attributes` and
    `info/exclude` that a fixture writes, which are produced inside the window by definition and
    harmless unless configuration names them. An object store outside the repository is P-k's (table
    above). No fixture uses one today (`/usr/bin/grep -n -i -e alternate -e shared -e 'clone'` over the
    fixtures file gives three lines, 408, 666 and 667 at `8413a4db`, all comments);
  - a fixture that legitimately needs extra config. That one reds, and needs a deliberate change to
    the reference, which is the fail-safe direction.

## §5 Residuals, the slot, and a pre-existing defect

### §5.1 What the window does not close: class (c)

**The unit.** A fixtures-file command can give a git process **an input not produced inside the
window** in three ways: by **removing** an allowlist entry, **overriding** one, or **adding** one.

The outside files are reachable from inside the window:
- the Homebrew system file and template are **caller-writable** (`/opt/homebrew/etc/gitconfig` is
  owned by `kazuaki admin`, and so is the templates directory);
- the caller's real home is computable with `eval "h=~$(id -un)"`.

**A command that leaves the input persisted is class (b), caught by P-g on shape (D2, E2)** when what
it persists is configuration or a git dir's shape, and by P-k when it is an object store outside the
repository; §4 says what lies outside all three. P-g reds, on
both shells:
- `printf '[include]…' >> .git/config`;
- `printf '[core]\n\texcludesFile=…' >> .git/config`;
- `git -C . config core.excludesFile …`;
- `git config include.path …`;
- `--separate-git-dir`, with or without a persisted key; a nested, hidden or bare git dir with a
  persisted key (draft 9, corpus §6).

On p7 the first three were **silently wrong**. The fourth was red only through S (companion §A.11). P-g
sees what is **still persisted when the build ends**. A form that persists and then reverts is class
(c).

**Class (c) is the residual, by property.** It is any fixtures-file command that gives a git an outside
input and leaves no persisted, observable state when the build ends (§0.3 gives examples). Measured on
p8: `git -C . -c include.path=<file> add -A` gives rc 0, `PASSED`, and Pdiff 50 on both shells. That is
silently wrong, and it is **declared**, not closed.

**D1: seed S is deleted.** Round 7 (Ax3) measured literal spellings passing S end to end with Pdiff 50
on both shells:
- `git -C . -c include.path=…`;
- the fixtures' own shim idiom, `printf '#!/bin/sh\nexec %s -c include.path=<file> "$@"\n' "$_REAL_GIT"`
  followed by `PATH=<shim dir>:$PATH git add -A`.

It also measured false reds: `commit --template=/dev/null`, `printf 'export x'` written into data, and
`git config core.autocrlf false`. Widening the regex would be the wrong repair: S's population is a
vocabulary, not the property. And keeping a second, partial surface for the same residual is decision
tax. So S, its label, its record and its census are removed. Draft 7 had adopted it, and this is
recorded as the removal of a round-7 instrument (`…-reviews.md` §D.7).

**`#11-k2-fixture-git-invocation-convention` closes (E1).** Draft 8 re-scoped it to the transient
residual. Under §0.3 that residual is class (c), so the create-time audit is applied to the slot as it
would stand:
1. **Does it name a defect this PR leaves?** No. It names a class the threat model declares out of
   scope. That is a boundary of the mechanism, not a gap in it.
2. **Is work owed?** No. Every candidate remedy is already rejected: an in-process detector is a
   vocabulary (S failed both ways), and an OS sandbox, row (iv) of §3, has no portable mechanism. Review
   of the fixtures file is the standing owner of all repository code, not a deferred task.
3. **Does it have a trigger that brings work back?** No. Draft 8's trigger, "reviewer attention on any
   change to the fixtures file", is a standing policy, not an event.
4. **Would keeping it change any decision?** No. It would only restate §0.3 in the ledger.

So it is **not a slot**. The ledger entry is removed, citing §0.3. If the threat model changes, for
example if fixtures come from untrusted contributors, that is a new decision with its own slot, not this
slot's trigger. The slot's original premise, verbatim from the ledger, "Measured 2026-09-26: the build
region contains no bare `git ` outside comments", is **moot**.

**Accounting:** 0 own deferrals. With the keep-set slot dissolved below, the ledger nets to **−2**.

`#11-k2-fgit-keepset-depends-on-git-purge-glob` **dissolves**. There is no keep-set and no helper that
calls `_git`. Its premise sentence was absent at `ff6b99a3`: the command below returns rc 1.

```sh
git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i -e exempt -e 'held back' -e purge
```

`_git` inside the window is **undefined**. A fixture calling it fails loudly: the corpus RLOUD cell gives
NE.

### §5.2 Other residuals, and the pre-existing defect

| # | residual | direction | disposition |
|---|---|---|---|
| R1 | which git executable runs, and which of the fixtures' other commands: the caller's `PATH`, verbatim, behind the pin (§0.1). A wrapper that needs other variables, or a tool that only a working-directory- or home-dependent entry provides, fails in the window (loud), but a `PATH` git wrapper that injects configuration only into `add` is **silent** | silent | class (a)'s **declared platform boundary** (§0.1), not closed: `PATH` → `#11-trip-wire-launch-environment`, member (3). That slot's body does not name the K2 window, and its trigger ("#519 lands") has fired; §9's ledger step amends both. Draft 10 called this "loud"; `/code-review` showed the silent case |
| R2 | filesystem-derived config written by `init`; FIFO support, permissions, raw names | either | outside P. P-d's reference `init` shares the filesystem |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any | declared blind spot. None is known at 2.55 |
| R4 | the **launch-environment class**: whatever the caller injects into the wire's own bash at startup (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*%%`, a function named `command`). The window's `env -i` drops these from the child's environment, but the parent that writes the prelude has already run under them. The prelude carries only the listed data and the eight function bodies §3 lists | any | `#11-trip-wire-launch-environment` |
| R5 | `$SCRATCH` owned by another UID | loud | outside P |
| R6 | reads through `_git` keep the caller's config, including a caller `GIT_TRACE=1`, which reds at base too (parent D7) | loud, pre-existing | `_git`'s contract |
| R7 | Windows git-bash: unmeasured | unmeasured | **declared residual.** The `trip-wires` job is ubuntu-only in CI (command below), and running `mise run ci` or the wires under Windows git-bash is not a supported surface today; nothing here claims it |
| R8 | a compiled-in reftable default (git 3.0's planned default, or a breaking-changes build): `badref` writes `.git/refs/heads/`, and P-d's `diff -r` differs between two reftable inits (random `reftable/*.ref` names, `tables.list`) — the gate would red on every PR for a non-K2 reason | — | **closed**: `GIT_DEFAULT_REF_FORMAT=files` is in the window's allowlist, pinned by P-i on every git. The `reftable` cell (a `git` that picks reftable unless the caller pins a format) is red before and green after (companion §A.14) |
| R9 | a hang outside the run's process group (§3): only the wire's own code before it sources the controls file (arguments, scratch creation, its EXIT trap). It reads no fixture state | loud in CI (the job timeout ends it), a wait locally | outside P, under R2. **Not a slot**, by the create-time audit: it names no defect this PR leaves (the code reads the platform only), no work is owed, no event would bring work back, and keeping a slot would change no decision |

R7's command, together with a negative case that shows it discriminates:

```sh
sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml | /usr/bin/grep runs-on   # runs-on: ubuntu-latest
sed -n '/^  check:/,/^  [a-z]/p' .github/workflows/ci.yml | /usr/bin/grep runs-on        # runs-on: ${{ matrix.os }}
```

**Pre-existing, not fixed here: `#11-k2-wire-exit-trap-masks-set-u-abort`.** On `/bin/bash` 3.2,
`set -euo pipefail` together with the wire's EXIT trap (since base, wire:405) turns an
unbound-variable abort into exit 0. The orchestrating session reproduced it (`…/scratchpad/r4ax2/u.sh`:
3.2 rc=0, 5.3 rc=1). The slot's owner is the **citation-hygiene lane**, and its ledger trigger is
"code sourced after wire:405".

This PR's harness and controls are exactly such code. **The ledger text — the one text; §9's ledger
step writes exactly this** — **replaces**, in place, the entry's sentence "(its memo U5: state
initialised at harness top level, setup failure a labelled verdict)" and the amendment written at PR
creation. It does not append to them:
- **measured, not argued:** with the wire's nounset off, the clean tree and eight red cells give the
  same exit status and verdict lines as with it on (bash 3.2 and 5.3; the cell script is verbatim in
  `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §13, "X13", and X13 re-runs it at this PR's final head).
  On those runs no parent-side verdict depends on `set -u`; that is a measurement over those runs, not a
  proof over every path;
- an incomplete fixture-build window is reported by the **W verdict alone**, and no control runs:
  `_control` asks `_fw_built_or_w2` first, and so does each of the three blocks that are not
  `_control`s (relative scratch, fsmonitor, umask). That covers a child that refuses a prelude missing
  any one of `errexit`, `nounset` or `pipefail`, a fixtures file that stops before its last line, and
  one that switches an option off. A run that exceeds its running phase's bound is killed as one
  process group and reported from the fired marker: a window, as W. A build that is complete but untrusted (a red postcondition, or W3) ends
  the run after its reports, and no control runs over it either;
- a mode restriction a fixture sealed and the window could not apply, or refused, is red (W4);
- pinned by the W records (one per prelude option, one for the early return, one for an abort, one for
  an option switched off by the fixtures file, one for a window that times out), the two W2 records and the W4 record. The three block gates
  are pinned by the traced `w2rec` cell only (§3's declared gap).

## §6 The corpus — evidence, and the source of the records

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` §6 ("the corpus file";
touch-time split, text unchanged). It holds the recast oracle, the prototypes p6–p11, the re-run
recipe, every focused cell table, and the mutation records with their totals and the ratchet;
its §6.1 holds drafts 11–12's planned record changes.

## §7 What was deleted (draft 5 → 6)

Draft 8 (D1) also deletes seed S. See §5.1 and `…-reviews.md` §D.7.

These were deleted, with details in `…-reviews.md` §D.5:
- the four-channel detector: the `PATH` shims, trace2 over two routes, and the poison;
- its canaries, labels and records;
- the per-call `_fgit`;
- draft 5's slot scope, and its claim "reads no caller input".

Round 5's Ax2 restore IMP, Ax3 IMP-2 and Ax3 M1–M3 went away with it. Draft 8 keeps
the `fixtures` mutation target for the W, W3 and P-g records.

## §8 Splits, the parent memo, and memo references

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §8 (touch-time split, text
unchanged; §8.1 code splits, §8.2 the parent memo's banner, §8.3 memo references in code). C0b–C2 are
implemented, so this is their record.

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, the companion, the umbrella rows | docs | the Step-4.5 focused check (E2 and E3) |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (eleven sites) | docs | the `…-reviews.md` §8.2 command, re-read |
| C1 | controls → controls + fixtures; one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4, X4b |
| C2 | mutations → mutations + mutgen; references qualified | prereq split | X1, X3, X4, X4b |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | the `harness:` and `fixtures:` record targets with resolver and restore; the harness's `_shq` comment, which said the mutation set "has nothing to aim at", rewritten because C4 makes it false (`…-reviews.md` §13) | infra, required | X3 |
| C5 | the window (§3), with the fixtures calling `git` and ending with the `built` line; `notcommitted`'s `mkdir`; the incomplete-window exit with its named causes; W2 as `_control`'s first statement and after the controls; W3; the per-option pins and the options re-check; §4's postconditions including P-g's census; the 20 records (50 at head; how they came to be: `…-reviews.md` §13); §3's two comment texts; the ratchet population; `…-reviews.md` §8.1's in-file rewrites; the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

**Cost, and `ci.yml`.** The base job comment "a wire that adds fixture self-tests re-derives this line
in the same PR" is an in-file rule (`git show e8f78896:.github/workflows/ci.yml | sed -n
'/^  trip-wires:/,/^  [a-z]/p'`). C5 re-derives the line by that rule's own method, and records the
method and the verdict without figures. If the value would change, **STOP and escalate to the user**.

- **Where the record goes.** C5 replaced the job comment's re-derivation paragraphs with the method
  (three runs per side, alternated) and one verdict line naming the commits. That line describes the
  tool code of the squash, so after C6–C10 it is stale until X8 re-runs at the final head and rewrites
  it.
- **The threshold.** Take the slowest of the three head runs, × 4.3 (the runner ratio, §3). If that
  reaches half the 5-minute budget, **STOP**: a head run of 34.9 s or more (150 / 4.3 = 34.88).
- **Interaction with #510.** #510 rewrites the same job. At `cc05c428` (read it at the time with
  `gh pr view 510 --json headRefOid`) it sets `timeout-minutes: 10` and carries no K2 rule. The umbrella
  withdrew "whichever lands second is a textual merge", so the later lander **re-derives** the line
  under both rules. It does not carry a paragraph forward.
- **Ownership.** The budget half is unowned (parent §6; umbrella, Cross-lane coordination).
- **Cost shape.** There is still **one** build. It runs in a child process, with the postconditions and
  P-g added.

**X9 cannot run on this PR as stacked.** `ci.yml` triggers only on `pull_request: branches: [main]`,
and this PR targets `webref-cite-audit-tool` (#519 ran zero checks). The GNU/Linux evidence therefore
needs one of two routes, and **the user decides which at push time**:
- **(a) A temporary draft PR from this branch to `main`**, opened solely to run CI and closed
  afterwards. Its diff would also carry #501's changes, because this branch sits on #501, so it must
  not be reviewed or merged as such.
- **(b) #501's own CI after the squash** serves as the GNU evidence. **Stop condition:** if that CI goes
  red on anything this slice touched, a fix lands before #501 merges.

**Land order:**
1. `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`.
2. User approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. `/external-converge`.
4. User approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella's single status cell — done inside #527 (it names the PR, which stays true after the squash; PR #527 Codex R19).
7. Hand the #501 merge decision to the user.
8. After #501 lands, update the ledger entries with #501's merge commit on `main`. This PR's squash SHA
   is unreachable from `main`.

**Ledger text, planned now.** The ledger convention is registration **before** merge, so the
orchestrating session writes these entries into `project_open-defer-slots.md` at PR creation:
- **Remove** `#11-k2-fgit-keepset-depends-on-git-purge-glob`, citing §5.1 (dissolved: no keep-set, no
  `_fgit`).
- **Remove** `#11-k2-fixture-git-invocation-convention`, citing §0.3 and §5.1: its residual is class
  (c), declared out of scope and owned by code review; the create-time audit finds no owed work and no
  trigger.
- **With both gone, delete their section too:** the heading "Citation-hygiene — K2 wire `_fgit` scrub
  fix: 2 own slots" and its Source paragraph hold nothing else, so they are removed with the two entries
  rather than left orphaned.
- **Amend** `#11-k2-wire-exit-trap-masks-set-u-abort` with §5.2's ledger text, verbatim; it is the only
  text for that amend.
- **Fix the header count.** The net change is −2: one slot dissolved, one closed.

### §9.1 Drafts 11–12: the commits on top of `8413a4db`

Drafts 11–12 are implemented as five commits on the PR branch. Each is green on both shells, and each
sets `_MUT_RECORDS_MIN` to its own record count. The record changes are in corpus §6.1. Per commit:
X1, X3, X4b and X10. At the final head: X2, X5, X6, X8, X9 and X12–X15. The history goes to
`…-reviews.md` §13, not to the provenance companion (972 lines).

| # | commit | what | this PR's / all records |
|---|---|---|---|
| C6 | M-PATH | delete the normaliser (the `_fp_*` loop and its comment block); `_FGIT_PATH="$_FGIT_BIN:$PATH"`; P-j keeps its first-entry and `git` clauses and loses the per-entry one; P-j's label renamed (§4); the harness comments that cite "memo §0.1" and "memo §1" without the file name go with the block (X4b is **not** empty on them at `8413a4db`) | 49 / 144 |
| C7 | M-WATCHDOG | in the controls file: `_k2_group`, `_k2_joined`, `_k2_phase` and the timer, the one-group-per-run start with its verdict from the fired marker, the exit handler composed with the wire's trap, and phase announcements for the setup, each non-`_control` block and the scan. In the harness: the window's and `_control`'s announcements, `_control`'s inline watchdog and its `_rc -ge 128` read deleted, `_FW_TIMEOUT`, and the window's wall time in the summary. In the mutation runner: each trial as its own group under `_MUT_TRIAL_CAP`, its output through a file, a killed trial's verdict line appended, and the run's phase unbounded while trials run. The always-on `set -m` check. Comments that say nothing has a time bound are rewritten | 51 / 146 |
| C8 | untrusted build | the window's verdict reported in one place, and the run ends unless the build is complete and trusted; `_fw_built_or_w2` asks the same; W2's label renamed | 52 / 147 |
| C9 | M-SHAPE | the shape scan before git runs on a `.git`; the non-directory-link arm inside a `.git` and the two-name `HEAD`/`config` guard deleted; no per-link search behind a link inside a `.git` | 55 / 150 |
| C10 | P-k | `count-objects -v` per compared `.git`, its liveness probe and machine-limitation arm; two labels | 58 / 153 |

C7 comes before C9, so no record in C9 runs a git that could block without a bound. After C10 come X8
(by the `ci.yml` rule, with §9's threshold) and the ledger step. That step:
- replaces the exit-trap slot's text, in place, with §5.2's;
- amends `#11-trip-wire-launch-environment`: member (3) gains the K2 window (git pinned as the caller's
  shell resolves it from the wire's directory; the other entries verbatim), and the fired trigger
  "#519 lands" becomes "the next PR that changes how a required wire, or the K2 fixture window,
  resolves or passes `PATH`". The PM lane stays the owner.

The land order then resumes at step 3 (`/external-converge`) on the new head. Draft 12 restructures
§3 and §10, so it goes to a full plan-review, round 11, before C6 (§12).

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | the window's `env -i` closes every variable, and four relocations close default files: class (a). That holds for every git in the window **whose inputs no fixtures-file command removed, overrode or added**, however it is spelled. A command that does is class (b) if it leaves persisted, observable state (P-g, W, W3) and class (c) otherwise, which is out of scope | §0.3, §3, §5.1; corpus G, R |
| 2 | A×D | the postconditions run in the window, so they describe every such git. P-g extends that to the persisted configuration and the shape of every git dir under the fixture root: an unknown git-dir shape is red, and so is any entry inside a git dir that is not a regular file or a directory. P-k adds that no fixture repo reads objects from a store outside it | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read. Completion needs the fixtures file's own last line, with the options still on; an incomplete window ends the run with W alone, a complete but untrusted one ends it after its reports, and `_control` itself refuses unless the build is complete and trusted (W2). Each child option is checked and recorded. With the parent's nounset off, the clean tree and eight red cells give the same verdicts (measured, §3; not a proof over every path) | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | producers live in the harness and labels in the controls file. Records target `harness`, plus `fixtures` for the W early-return, W options re-check, W3 and P-g records | §4, corpus §6 |
| 9 | D | one label per producer, one record per label; the ratchet stays at 21 | corpus §6 |
| 10 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it, so the window's `PATH` is the caller's, verbatim, behind the pin, and P-j checks only the pin | §0.1, §5.2 |
| 11 | F | still one build, now in a child; the ci.yml line is re-derived by rule, and a change means STOP | §9 |
| 12 | D×F | a wire run is one process group, and nothing in it re-groups: `set -m` appears only in `_k2_group`, which runs only where no group is in effect (a top-level run; each mutation trial, while the enclosing phase is unbounded). The running phase announces its bound; one timer outside the group kills the whole group, which reaches every descendant because none left it; the verdict comes from the fired marker, never a status; the group is killed again after its leader exits. Outside it: the wire's code before it sources the controls file (R9) | §3, §5.2 |

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | the X1 block below | `rc=0`, `0`, `1` |
| X2 | the corpus §6 G cells (its recipe, on the implementing head as `p6/`) | all PASS with P equal, 4 configs |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'` | three hits in every column. C1–C3 are byte-identical to base |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | the X4b block below, over each file C1/C2 and C6–C9 edit | empty (at `8413a4db` the harness gives 2 lines, 71 and 93, which C6 deletes) |
| X5 | the corpus §6 R, RLOUD, RES and P cells, same recipe; the draft-10 cells (companion §E.7) on the implementing head as `p11/`; and drafts 11–12's cells (corpus §6.1) | R and RES green with P equal; RLOUD red; P PASS; every AFTER row PASS; each draft-11/12 cell as corpus §6.1 says |
| X6 | two by-hand edits to `…trip-wire.fixtures.sh`, each just before its `: > "$_FW_DIR/built"` line: `mkdir -p "$_FGIT_VOID" && printf 'ref: refs/heads/x\n' > "$_FGIT_VOID/HEAD" && : > "$_FGIT_VOID/config"`, and `git -C "$CTL/clean" config include.path /nonexistent-k2`; one run each | red: the template through **P-c**, the include through **P-g**. Not P-d: P-d compares two inits that both use the void as their template, so a template planted there is on both sides. Not P-a: P-a reads one probe repo, not the fixtures |
| X8 | the ci.yml rule's derivation (§9) | method and verdict recorded; STOP on change |
| X9 | `Layering trip-wires` on ubuntu (GNU), via route (a) or (b) of §9, chosen by the user at push time | SUCCESS. This is GNU evidence for `env -i`, `env -0`, the window, the prelude and the process-group bound. Record the window's wall time from the summary line (§3), for the bounds' next derivation. If a record's sed expression reads differently under GNU sed, the always-on anchor check in `_mut_correspondence` is the first thing to fail, on the ordinary run |
| X10 | `$SH -n` and `wc -l` over `ls .claude/tools/webref-generic-core-trip-wire*.sh` | clean; every part below 1000 lines |
| X11 | in a clone, add to `…trip-wire.controls.sh`, after the other `_lbl` definitions, `_x_lbl="an unrecorded probe"` and `echo "$_x_lbl" >/dev/null` | red, and the ratchet lists it |
| X12 | the cells (a)–(c) of §3, by hand, from the edits and scripts in `…-reviews.md` §13 (each FIFO lives under the run's own scratch directory); after each, `ps -ax -o pid=,command=` filtered by that run's scratch path and by the cells' names (`k2ff`, `k2blk`) | (a) rc 2, W; (b) rc 1, CONTROL FAILED with the probe's label; (c) the trial returns at the window's bound, and at its cap when that bound is broken. No process names that scratch path or those names afterwards. The filter is scoped to this run, so a concurrent session's processes do not count |
| X13 | the `crc.sh` script, verbatim in `…-reviews.md` §13, at the final head: the clean tree and the eight red cells (`garbagehead` is an untrusted build), with the parent's nounset off (`NOU=1`) and on, on bash 3.2 and 5.3 | the same exit status, NE/CF counts and first four `!!` lines both ways, on both shells |
| X14 | at the final head: PX1's caller `PATH` (`~+/bin` first, a logging `git` in the repository's `bin/`); Codex R23's `tools/bin` wrapper with its interpreter beside it; Codex R24's `~/bin` wrapper with a helper | each PASSED, or red with a named cause, and none hangs. For PX1 the expected result is PASSED with the wrapper as the fixture git (§0.1; round 10 measured that on the draft-11 prototype). The outcomes are recorded in `…-reviews.md` §13 |
| X15 | by hand, before `built`: `git init -q "$CTL/zzx" && mkdir -p "$_FW_DIR/zzt/a" && : > "$_FW_DIR/zzt/a/HEAD" && ln -s "$_FW_DIR/zzt" "$CTL/zzx/.git/k2link"` | rc 1, P-g, whose message names `zzx` by the shape rule only; no "symlink to a tree holding a git dir" entry for `k2link`, so the search did not run behind it |

Commands containing `|` are kept out of table cells, because `\|` in a markdown cell is read one way
raw and another way rendered, and that made one of X1/X4b vacuous. They use `-e` alternatives instead.
Each was re-run to show it discriminates (companion §A.10).

```sh
# X1 — a green log gives 0 and 1; a red one gives 1 and 0 (measured on c7 f0011 and f0013)
( $SH $W ) > $S/l 2>&1; echo "rc=$?"
/usr/bin/grep -c -e 'CONTROL NOT EXERCISED' -e 'CONTROL FAILED' $S/l
/usr/bin/grep -c 'trip-wire PASSED' $S/l

# X4b — base mutations.sh gives 2 lines (15, 20); the same file with both qualified gives 0.
# The old in-table form '§[0-9]\|plan memo\|the memo' (raw, no -i) gave 0 on base, i.e. vacuous.
/usr/bin/grep -n -i -E -e '§[0-9]' -e 'plan memo' -e 'the memo' <file> | /usr/bin/grep -v 'citation-hygiene-[A-Za-z0-9-]*\.md'
```

## §12 Plan-review

Plan-review closed for draft 10 after round 9 (`…-reviews.md` §D.0 records the ground). Round 10
reviewed draft 11 (`…-reviews.md` §13). Draft 12 restructures §3 (the bound) and §10 (#12), and adds
P-k, so **round 11 is a full re-review** before C6. It has not run.

## §13 Implementation results

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §13 (touch-time split: this
memo holds the live design; the implementation history lives with the review record).
