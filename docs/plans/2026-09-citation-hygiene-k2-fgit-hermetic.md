# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits and false premises. **Review
record**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`, every plan-review round's
dispositions and the terminators (split out of the companion unchanged). This memo holds only the live
decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (§8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 10. Plan-review CLOSED** (orchestrating session, 2026-09-27; ground in
`…-reviews.md` §D.0).

Draft 10 answers round 9, a Step-4.5 focused check of draft 9 (`d5dacd56`): Ax2 0/1/1, Ax3 0/1/3. Both
IMPs were **implementation bugs in the prototype, not design defects**, and prototype **p11** fixes
them with their cells (§6, companion §A.13):
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
**p6** (§6).

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
standalone commit; parent §11.7 row 7 is the precedent (`…-reviews.md` §D.2, row R2-6).

### §0.3 Threat model

The fixtures file is repository code, written and reviewed like the rest of the repository. Three
classes of input can reach the fixture build, and this slice treats them differently:

| class | what | owner | how it is handled |
|---|---|---|---|
| **(a)** | **implicit caller-environment leaks**: anything the caller's environment, home, XDG directory, system prefix or compiled-in defaults carry into git without the fixtures file asking for it | **this PR** (its purpose) | **closed** by the window (§3); pinned by P-a…P-f |
| **(b)** | **accidental fixture-authoring mistakes that persist an outside git input or leave the build incomplete**: a `git config` naming an outside file, an include appended to `.git/config`, an accidental `--separate-git-dir`, an arithmetic error that skips a line, a `set +e` left at the end | **this PR** | **caught, fail-safe on shape**: P-g over every git dir (§4); W, with the options re-checked after the file; W3 (a shell diagnostic) |
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
# harness — snapshots taken when the harness is sourced (the shape at head; the harness is the source)
_FGIT_VOID="$SCRATCH/fgit-void"             # mkdir, checked
_FGIT_ENVBIN="$(command -v env)"; _FGIT_BASH="$BASH"
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID" GIT_DEFAULT_REF_FORMAT=files "LC_ALL=C")
# _fgit_window: write a prelude of PLAIN assignments (labels by name) and function bodies, then
"$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" "$_FGIT_BASH" -c '
  . "$1"; cd "$_FW_DIR"                        # prelude; a directory the window owns
  _fw_opts_on || <cause; exit>                 # errexit, nounset, pipefail in force
  . "$2"                                        # the fixtures file (last line writes `built`)
  [ -e "$_FW_DIR/built" ] || <cause; exit>; _fw_opts_on || <cause; exit>
  _fgit_postconditions || : > "$_FW_DIR/post_bad"
  _seal_apply                                   # mode restrictions, after the census
  printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
  : > "$_FW_DIR/done"' _ "$_FW_DIR/prelude.sh" "$_FIXTURES" 2> "$_FW_DIR/stderr" || _fw_rc=$?
```

**What the child gets, kept minimal.** The prelude contains:
- `set -euo pipefail`;
- plain `name=%q` assignments. These cover the data the fixtures file reads (`CTL`, the five
  `CONTROL_*` samples, `_REAL_GIT`, `_REAL_GREP`, `_fifo_ok`) and the window's own `_FGIT_VOID`,
  `_FGIT_ENVBIN`, `_FGIT_ENV_NAMES`, `_FGIT_WIRE_EXEC`, `_FGIT_WIRE_LC` and `_FW_DIR`, and the
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
stderr directly.

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

**Why exit 2, and why it is not pinned.** Exit 2 follows the wire's own convention that 2 means
"decided nothing". Nothing consumes the distinction between 1 and 2, though: the driver runs
`bash "$w"` under `set -euo pipefail` and aborts on any non-zero status
(`git show e8f78896:scripts/trip-wires.sh`, lines 22 and 204). The mutation runner's kill test is
"rc ≠ 0 plus a needle" (`_mut_trial`), and it cannot see the number either. So the number is **not**
pinned, and draft 7's "2 stays 2" / "carry the child's class" claims are withdrawn.

What **is** pinned are the properties a record can observe:
- **W** fires when the window is incomplete.
- **W2** fires if any control is reached over an unbuilt tree. W2 is `no control runs over an
  incomplete fixture build window`. Its predicate is one function, `_fw_built_or_w2`. **`_control`
  calls it as its first statement** and refuses when the window is incomplete (E3), and each of the
  three blocks that are not `_control`s (relative scratch, fsmonitor, umask) asks it before it runs.
  So no control runs over an unbuilt tree, wherever the incomplete-window exit sits. **Its record pins
  that W2 is reported when the exit is gone** (it removes the `exit` from
  `_fgit_window_incomplete_exit`), which `_control`'s gate alone satisfies. ⚠ **Declared gap:** removing
  one of the three block gates survives the mutation set (W2 is already reported by the first
  `_control`); those gates are pinned by the traced `w2rec` cell (companion §A.14), not by a record.
- Moving the exit below a control is an edit to the controls file, which is not a mutation target. That
  shape is pinned by the corpus RO/ROg cells (X5), not by a record.
- **W3** fires when bash reported a diagnostic located in the fixtures file (below).

**A shell diagnostic in the fixtures file is red (W3; E4, AR).** An arithmetic-expansion error such as
`$(( 1/0 ))` at the fixtures file's top level does **not** stop a sourced file under `set -e`, on either
shell. The build completes, and on p8 the run was green (§6). The child's stderr is captured to a file
and replayed. **Any line that begins with `./fixtures.sh:` or `./prelude.sh:`** — the window sources both from its own directory by those relative names (the fixtures file is copied there first), so the prefix does not depend on the checkout path, which could otherwise hold a newline that splits the name across two stderr records (PR #527 Codex R2)
is W3. That is a property of where bash puts a diagnostic about a file, not a wording: it covers `…
line N:`, 3.2's `… command substitution: line N:` and `… eval: line N:`, in any locale. (Draft 10
matched only `<fixtures path>: line N:`, which missed the other forms; `/code-review`, `…-reviews.md` §13.) The scan is
`awk` over the file, stopping at three lines: **no pipe into `head`**, which under `pipefail` SIGPIPEd
grep (141) and read as "no diagnostic" (round 9, Ax2). A non-zero `awk` status is itself red. It was
measured on bash 3.2 and 5.3 on macOS; X9 confirms GNU bash. It is class (b) by the §0.3 property: a skipped line leaves a
diagnostic.

The before/after cells are in §6.

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
§6.

| entry | without it |
|---|---|
| `PATH=$PATH` | BSD `env -i` runs `/usr/bin/git` rather than `PATH`'s git (INFO cell) |
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
before the fixtures file (one record per option, §6), and all three are checked again after it (one
record). So an abort there
is a non-zero exit, which the parent reads as data. The verdict is written after `ctl_ok=0`
(`controls.sh:738` at base), and an incomplete window ends the run right there.

**The parent's reliance on `set -u`, measured (IMP-2).** With the wire's `set -euo pipefail` changed to `set -eo pipefail` (nounset OFF in the parent), the
clean tree and eight red cells (sealfail, env0, garbagehead, w2rec, lblrename, w3ar, sealdotdot,
reftable) gave the same exit status, the same NE/CF counts and the same verdict lines as with it on, on
bash 5.3 at the `/elidex-review` head (`NOU=1 …/scratchpad/impl/crc.sh <head> b53 <cell>`, companion
§A.14). That is the measured claim: on those runs, no parent-side verdict depended on `set -u`. It is
not a proof over every path, and the "every state name is assigned before it is read" argument is a
reading of the code, not a measurement. The child's `set -u` is a backstop
behind the static census, and it is pinned by its own record. See §5.2 for the pre-existing defect.

## §4 Postconditions — run INSIDE the window

Every git in the window whose inputs **no fixtures-file command altered** inherits the same
environment and the same persisted state. So git's own answers inside the window describe every such
git (§5.1 covers the rest). Each producer has its own label (defined in the controls file and passed
in), and each label has its own record (§6).

| id | label | assertion (inside the window) | liveness (its own label) |
|---|---|---|---|
| W | `the fixture build window completed` | the fixtures file's last line wrote `built`, the options were still on after it, and the child wrote `done`. Otherwise NE with the sentence the child wrote to `cause`, or the exit status (§3), **reported alone**; exit 2; no control runs | — |
| W2 | `no control runs over an incomplete fixture build window` | `_control`'s first statement, and the first thing each non-`_control` block asks: the window is complete. Its record: the exit removed. The exit placed below a control: the RO cells | — |
| W3 | `the fixtures file ran without a shell diagnostic` | no line of the child's stderr begins with `./fixtures.sh:` or `./prelude.sh:` (both sourced by those relative names from the window's directory) | — |
| W4 | `every mode restriction a fixture sealed was applied` | every `_seal` was accepted (a path under `$CTL`, with no newline or TAB) and its `chmod` succeeded; otherwise red, not a machine limitation | — |
| P-a | `a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file` | every `git config --list --show-scope --show-origin` line in **one probe repo** is `local<TAB>file:.git/config<TAB>…` | `this git reports a non-local configuration scope`: `-c a.b=c` must show as scope `command` |
| P-b | `the fixture git has no system or global layer outside the void` | asked **inside P-a's probe repo** (`git -C`), so no caller repository's local configuration is read. A git older than 2.42, whose `git var` cannot name these four (exit 129), is a machine limitation: `⚠ NOT EXERCISED on this machine`, green. Otherwise `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is empty. Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `no exec-path override reaches the fixture git` | the window's `git --exec-path` equals the same git's answer with nothing but `PATH` in its environment (taken in the parent at source time), both canonical (`pwd -P`). **Re-scoped by `/code-review`:** which executable runs is outside P (§0.1, R1); P-e pins only that nothing in the window overrides where that git runs its commands from. So a caller's `GIT_EXEC_PATH` or `DEVELOPER_DIR`, or one directory spelled two ways, is not a red | — |
| P-f | `the fixture build window's environment holds only its allowlist` | the name of **every** `env -0` record (the text before the first `=`, so non-identifier names such as `BASH_FUNC_f%%` are included; on an `env` without `-0` — older macOS and BSDs, PR #527 Codex R4 — plain `env` one line per record, where a multi-line value can only add a spurious, red name, never hide one) is an allowlist name (derived from `_FGIT_ENV` itself, so the two cannot drift) or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction. p6's `sed` parser skipped non-identifier names; p7 parses every record (companion §A.10). `env -0` goes through a file whose status is checked: a failed or unsupported `env -0`, or no record at all, is NOT EXERCISED | — |
| P-g | `every fixture repo persists only the configuration a plain git init writes` | for **every git dir under the fixture root** (population below), the lines of `git -C <repo> config --list --show-scope --show-origin` **equal, as a set,** the lines of a **reference** `git init` made in the same window (P-a's probe repo, which is that init), compared by scope, origin, key and value (`grep -vxF -f`, both directions). An extra line is an input the fixture persisted; a missing one (`git config --unset core.filemode`) hands that setting to the platform default — either way P would depend on more than the fixture (PR #527 Codex R1). This catches a persisted include (its origin is not `.git/config`) and **any** persisted key beyond init's, `core.excludesFile` included. **Both directions:** a git dir whose listing is EMPTY or fails (a `.git` git does not recognise, e.g. a garbage `HEAD`) is red | — |
| P-i | `the fixture repos use the files ref format` | the window's `GIT_DEFAULT_REF_FORMAT` is `files`, and a plain init's `git rev-parse --show-ref-format` answers `files` (a git before 2.45 has no reftable; its `rev-parse` echoes the unknown option back with exit 0, and that literal echo is what counts as `files` — a failed call or any other format name stays red). It pins the allowlist entry on every git, including the files-default ones where dropping it changes nothing else | — |
| P-h | `the fixture build window reads in the wire's locale` | the window's `LC_ALL` equals the wire's (`C`). It pins the allowlist's `LC_ALL=C`, which P-f cannot, because P-f derives its names from the same list | — |

**How P-g's key set is derived (D2).** It is **not** a list of path-typed keys. Git exposes no per-key
type: `--type=path` is chosen by the caller, not declared per key, and `git help config` is prose. A
list of path keys would therefore be vocabulary, and a key missing from it would pass silently.
Instead, the allowed set is **what git itself writes on `init`**, measured in the window on every run.
That is the fail-safe direction: any persisted entry beyond it is red, whether or not it names a path.

**P-g's population, by property (E2; A2, A3).** It is every git dir the fixtures produced. One `find`
from `$CTL`, with no per-directory fork, lists every `.git` entry **of any type** and every file named
`HEAD`. It descends everywhere: hidden and nested directories, and the inside of `.git` directories.
- A `.git` that is a real directory has its configuration compared.
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
and a hidden one. Both were measured (§6).

- **Census:** today every fixture git dir's configuration equals the reference on both shells, `find`
  reports nothing, and the three declared directories are at their modes, so the clean and G cells
  are green (§6). Legitimate fixture state (`badref`'s ref, `notcommitted`'s `info/exclude`, the
  per-call identity) is not configuration, so it stays green.
- **Cost:** measured as the census section's wall time on a clean tree (instrumented copies, companion
  §A.13): **0.76 s** on p11 against 1.45 s on p10, on both shells. Most of p11's figure is the one
  `git config --list` per fixture git dir. p10's extra time came from one `sh -c` per directory, which
  mutation mode would have paid about 115 times. X8 measures the whole cost.
- **What P-g cannot see:**
  - class (c) (§0.3): transient inputs, persist-then-revert, a git dir outside the fixture root;
  - persisted inputs outside configuration: hooks, `info/attributes` and `info/exclude` that a fixture
    writes, which are produced inside the window by definition, and harmless unless configuration
    names them;
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

**A command that leaves the input persisted is class (b), caught by P-g on shape (D2, E2).** P-g reds,
on both shells:
- `printf '[include]…' >> .git/config`;
- `printf '[core]\n\texcludesFile=…' >> .git/config`;
- `git -C . config core.excludesFile …`;
- `git config include.path …`;
- `--separate-git-dir`, with or without a persisted key; a nested, hidden or bare git dir with a
  persisted key (draft 9, §6).

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
| R1 | which git executable runs (`PATH`). A wrapper that needs other variables fails in the window (loud), but a `PATH` git wrapper that injects configuration only into `add` is **silent** | silent | class (a)'s **declared platform boundary** (§0.1), not closed: `PATH` → `#11-trip-wire-launch-environment`. Draft 10 called this "loud"; `/code-review` showed the silent case |
| R2 | filesystem-derived config written by `init`; FIFO support, permissions, raw names | either | outside P. P-d's reference `init` shares the filesystem |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any | declared blind spot. None is known at 2.55 |
| R4 | the **launch-environment class**: whatever the caller injects into the wire's own bash at startup (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*%%`, a function named `command`). The window's `env -i` drops these from the child's environment, but the parent that writes the prelude has already run under them. The prelude carries only the listed data and the eight function bodies §3 lists | any | `#11-trip-wire-launch-environment` |
| R5 | `$SCRATCH` owned by another UID | loud | outside P |
| R6 | reads through `_git` keep the caller's config, including a caller `GIT_TRACE=1`, which reds at base too (parent D7) | loud, pre-existing | `_git`'s contract |
| R7 | Windows git-bash: unmeasured | unmeasured | **declared residual.** The `trip-wires` job is ubuntu-only in CI (command below), and running `mise run ci` or the wires under Windows git-bash is not a supported surface today; nothing here claims it |
| R8 | a compiled-in reftable default (git 3.0's planned default, or a breaking-changes build): `badref` writes `.git/refs/heads/`, and P-d's `diff -r` differs between two reftable inits (random `reftable/*.ref` names, `tables.list`) — the gate would red on every PR for a non-K2 reason | — | **closed**: `GIT_DEFAULT_REF_FORMAT=files` is in the window's allowlist, pinned by P-i on every git. The `reftable` cell (a `git` that picks reftable unless the caller pins a format) is red before and green after (companion §A.14) |

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
step writes exactly this** — replaces the entry's stale reference to the deleted detector ("U5 … setup
failure a labelled verdict"):
- **measured, not argued:** with the wire's nounset off, the clean tree and eight red cells give the
  same exit status and verdict lines as with it on (bash 5.3, at this PR's head; the command is in §3).
  On those runs no parent-side verdict depends on `set -u`; that is a measurement over those runs, not a
  proof over every path;
- an incomplete fixture-build window is reported by the **W verdict alone**, and no control runs:
  `_control` asks `_fw_built_or_w2` first, and so does each of the three blocks that are not
  `_control`s (relative scratch, fsmonitor, umask). That covers a child that refuses a prelude missing
  any one of `errexit`, `nounset` or `pipefail`, a fixtures file that stops before its last line, and
  one that switches an option off;
- a mode restriction a fixture sealed and the window could not apply, or refused, is red (W4);
- pinned by the W records (one per prelude option, one for the early return, one for an abort, one for
  an option switched off by the fixtures file), the W2 record and the W4 record. The three block gates
  are pinned by the traced `w2rec` cell only (§3's declared gap).

## §6 The corpus — evidence, and the source of the records

**Oracle, recast.** Under a hostile caller, a bypass spelling no longer has to go red. It has to go
**green with P identical to the clean build's P**. P is dumped per fixture repo by a **prototype-only**
hook (`K2_CORPUS_PDUMP`), which is not part of the design. It is passed into the window as a plain
assignment, so P-f does not see it. Symlink targets are normalised for `$CTL` (companion §A.9).

**Subjects.** All three are `git clone --local`s of `e8f78896`.
- **p6** carries §3–§4 as in draft 6. It shows the window's own guarantee under hostile callers.
- **p7** is p6 plus draft 7's fixes, including S, which is now deleted.
- **p8** is p7 minus S, plus P-g, the `built` marker, the W reason text, W2 and the per-option checks.
  Draft 8's claims are measured on p8.
- **p9** is p8 plus `_control`'s W2 guard, W3, the options re-check, the named exit causes, the comment
  fixes, and a top-level P-g shape check. It is superseded by p10 on P-g only.
- **p10** is p9 with P-g's census over every git dir (§4). Draft 9's claims are measured on p10.
- **p11** is p10 plus round 9's fixes: W3 without a pipe into `head`, the census opening the declared
  directories, the `HEAD` + `objects`/`commondir` shape, no per-directory fork, and the cause marker.
  Draft 10's claims, and the implementation, are measured against p11.

**Subset and re-run recipe.** Scripts: `git show ff1322bb:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` §E.3 (scratch `c6/`). Run them from a directory holding the
tree under test as `p6/` and, for the before-cells, `p5d/`:

```sh
python3 c6/gen6.py <dir>                                         # 190 cells
tr '\n' '\0' < c6/jobs.tsv | xargs -0 -n1 -P2 c6/cell6.sh > c6/raw.tsv   # K2_CORPUS_OUT overrides the output dir
c6/eval6.sh > c6/results.tsv                                     # verdicts, P compared per config
```

For X2/X5 on the implementing head, clone that head as `<dir>/p6`. The scripts take no other paths.

**Draft 6's recast corpus** covered 190 cells on p6, across all four configurations. The cell list is
in companion §A.9. It contains:
- **G:** 11 hostile callers, including the DO cell;
- **R:** 11 bypass spellings under a HOME `*.py` ignore, with two of them also run × every hostile
  caller;
- **RLOUD:** `_git`;
- **RES:** 2 cells;
- **P:** the records;
- **INFO:** 2 cells;
- **BEFORE:** cells on p5d.

**Results.** Draft 6's corpus results are in companion §A.9:
- G, R and RES were green with P equal on all four configurations;
- the DO cell was green with P equal, so the read side still sees the caller's configuration;
- round 5's four bypass spellings went from silently wrong on p5 to P-equal on p6;
- the one INFO cell that stayed green is PATH-drop on git 2.54, which is equivalent there, as
  predicted.

**Draft 8's focused cells.** They ran on p7 (before) and p8 (after), on bash 5.3·git 2.55 and bash
3.2·git 2.54. The scripts are in the history (companion §E, `453b7b0f` §E.5), and the full table is
in companion §A.11, which also records their re-run with `HOME` set: unchanged.

| cell | p7 (before) | p8 (after) |
|---|---|---|
| D2: `printf '[include]\n\tpath = <file>' >> .git/config` | rc 0, PASSED, P differs: **silently wrong** | rc 1, P-g |
| D2: `printf '[core]\n\texcludesFile = <file>' >> .git/config` | **silently wrong** | rc 1, P-g |
| D2: `git -C . config core.excludesFile <file>` | **silently wrong** | rc 1, P-g |
| D2: `git config include.path <file>` | red only via S | rc 1, P-g |
| D4: a top-level `return 0` early in the fixtures file | rc 1, **81** control lines: `done` certified the child, not the file | **rc 2, W alone** |
| D4: `$CTL/odd/pipe` pre-created (the fixtures exit 2) | (draft 7: rc 2, W alone) | rc 2, W alone |
| D4 record W2: incomplete-window `exit` removed, plus `odd/pipe` | — | rc 1, W2 printed |
| D4 records: the prelude drops `nounset` / `pipefail` / `errexit` (one each) | — | each rc 2, W alone |
| D3 residual, now a class-(c) example (§0.3): `git -C . -c include.path=<file> add -A` | — | rc 0, PASSED, Pdiff 50: **silent, declared** |
| G: HOME `*.py` ignore; the DO cell | — | green, P equal |
| P-a record with the §5.1-unit label | — | red with the label |

**Draft 9's focused cells.** They ran on p8 (before) and p10 (after), on bash 5.3·git 2.55 and bash
3.2·git 2.54, **with `HOME` set for every cell**. Every row is the same on both configurations. The
scripts are in companion §E.6, and the full table is in companion §A.12.

| cell | p8 (before) | p10 (after) |
|---|---|---|
| E2: `cachedir` built with `--separate-git-dir` plus a persisted `core.excludesFile` | rc 0, PASSED, P differs: **silently wrong** | rc 1, P-g: `cachedir:[.git is not a directory] .gd-cachedir:[a git dir not named .git]` |
| E2: `cachedir` built with `--separate-git-dir` only (a gitfile) | rc 0, PASSED, P differs: **silently wrong** | rc 1, P-g, same two entries |
| E2: a nested repo `zz/inner` with a persisted `core.excludesFile` | green: unseen | rc 1, P-g: `zz/inner:[local file:.git/config core.excludesfile=…]` |
| E2: a hidden top-level repo `.hid`, same key | green: unseen | rc 1, P-g |
| E2: a bare git dir `zbare`, same key | green: unseen | rc 1, P-g: `zbare:[a git dir not named .git]` |
| E3 RO: the exit moved below the first control, plus `odd/pipe` | rc 2: W, plus the first control **misreported** as `green is reachable`; no W2 | **rc 2: W2, then W** |
| E3 ROg: the exit moved below the first control, window complete | green, P equal | green, P equal |
| AR: `_ar=$(( 1/0 ))` after the fixtures file's first line | rc 0, PASSED: **silent** | rc 1, W3 |
| SE1: `set +e` before the fixtures file's `built` line | — | rc 2, W alone: "the fixtures file switched off errexit, nounset or pipefail" |
| W2 record: the incomplete-window `exit` removed, plus `odd/pipe` | — | rc 1, W2 |
| G: HOME `*.py` ignore; the DO cell | — | green, P equal |
| clean | green | green, P equal |

**Draft 10's cells.** They ran on p10 (before) and p11 (after), with `HOME` set for every cell. The
scripts are in companion §E.7, and the full table is in companion §A.13.

| cell | p10 (before) | p11 (after) |
|---|---|---|
| arnoise: `$(( 1/0 ))` plus 200 `[` diagnostics | green, PASSED: **W3 failed open** | rc 1, W3 |
| noise1k: 1000 `[` diagnostics | green, PASSED: **W3 failed open** | rc 1, W3 |
| ar: `$(( 1/0 ))` alone | rc 1, W3 | rc 1, W3 |
| m2h: nested repo under `walk/sub`, outside `core.excludesFile`, gitlinked into `walk` (4 shell × git configs) | rc 0, PASSED, P differs where a reference exists: **silently wrong** | rc 1, P-g `walk/sub/inner:[local file:.git/config core.excludesfile=…]` |
| wtmeta: a linked worktree removed, its `.git/worktrees/` entry left | rc 0, P differs: **silently wrong** | rc 1, P-g `zzw/.git/worktrees/zzw2:[a git dir not named .git]` |
| nr: a `chmod 300` directory under `walk` | rc 1, P-g (census failed) | rc 1, P-g (census failed) |
| rc5: `sh -c 'exit 5'` at the fixtures file's top level | rc 2, W **misnamed** "switched off errexit…" | rc 2, W "the window exited 5 before completing" |
| draft 9's set: `--separate-git-dir`, nested, hidden, bare, RO, ROg, SE1, G, DO, W2 record | — | as draft 9: all PASS |

Both shells gave the same verdict in every row; m2h also ran on bash 5.3·git 2.54 and bash 3.2·git 2.55.

**Records: representative only.**

| label | record | target |
|---|---|---|
| W | the child exits right after the prelude | harness |
| W | the prelude drops `nounset` (`set -eo pipefail`) | harness |
| W | the prelude drops `pipefail` (`set -eu`) | harness |
| W | the prelude drops `errexit` (`set -uo pipefail`) | harness |
| W | a top-level `return 0` early in the fixtures file | **fixtures** |
| W2 | one `harness:` expression with two substitutions: the `exit` removed from `_fgit_window_incomplete_exit`, and the child's `done` marker renamed so the window stays incomplete. A record edits one file, so the draft's "plus the fixtures exit 2" is not expressible (`…-reviews.md` §13) | harness |
| P-a | config appended to `_FGIT_ENV` | harness |
| P-a liveness | the probe loses `-c a.b=c` | harness |
| P-b | drop `GIT_CONFIG_NOSYSTEM` | harness |
| P-b liveness | the probe loses `…NOSYSTEM=0` | harness |
| P-c | plant a file in the void | harness |
| P-d | drop `GIT_TEMPLATE_DIR` | harness |
| P-e | inject `GIT_EXEC_PATH=/nonexistent-k2` | harness |
| P-h | drop `"LC_ALL=C"` from `_FGIT_ENV` | harness |
| W4 | `_seal_apply`'s `chmod` replaced by `false` | harness |
| P-i | drop `GIT_DEFAULT_REF_FORMAT=files` from `_FGIT_ENV` | harness |
| P-f | drop `-i` from the window's `env` | harness |
| W | `set +e` before the fixtures file's `built` line | **fixtures** |
| W3 | `_ar=$(( 1/0 ))` after the fixtures file's first line | **fixtures** |
| P-g | `printf '[include]…' >> .git/config` in a fixture | **fixtures** |
| P-g | one fixture's `git init` gains `--separate-git-dir` (a gitfile) | **fixtures** |
| P-g | a nested repo with a persisted `core.excludesFile` | **fixtures** |
| P-g | a fixture unsets a key a plain init writes (`git config --unset core.filemode`) | **fixtures** |

- **Totals:** **15 labels and 23 records** (P-h and W4 added by `/code-review`, P-i by `/elidex-review`, P-g's removal record by Codex R1; `…-reviews.md` §13). `_MUT_TARGETS="harness fixtures"`: the prefix parts; a record
  with no prefix edits the wire.
- **The `fixtures:` prefix:** it is a BSD `sed` error ("invalid command code f"); GNU is unmeasured.
- **The ratchet:** **`_MUT_UNRECORDED_MAX` stays at 21**, and `_MUT_RECORDS_MIN` rises by exactly 23 (95 → 118).
- **What is not a record:**
  - RES cells, because the runner requires the `!survive` needle exactly once (`mutations.sh:658–662` at `e8f78896`);
  - the exit number, which is unpinnable (§3);
  - the two INFO cells, which are informative.

**Mutation-mode cost (X3).** Each run of X3 is (95 base records + 23) record trials plus the generated
population, one control pass each. X3 prints the counts, and X8 gives the per-pass time. This is
opt-in and does not add to the always-run gate.

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
- the sibling guard (moved into the always-on `_mut_correspondence` by `/code-review`, `…-reviews.md` §13);
- the extended entry contracts.

**In-file statements that become false, and the commit that rewrites each.**

In `mutations.sh`, found by the stated grep, which returns lines 44, 93, 120, 134, 137, 526, 672, 673
and 684:

```sh
/usr/bin/grep -n -i -e 'every control' -e 'a control means' -e 'controls with no record' -e 'controls have no' -e '_control. call' -e 'umask and' .claude/tools/webref-generic-core-trip-wire.mutations.sh
```

| lines | statement | fate |
|---|---|---|
| **:44–46** | "Every control has to be named by some record" | C5 rewrites it: the population becomes `_control` labels ∪ `_lbl="…"` definitions |
| **:120** | "ADDING A CONTROL MEANS ADDING A RECORD" | C5 rewrites it the same way |
| **:133–138** | the correspondence: labels are a `_control`'s "or the label a block that is not a `_control` prints (the umask and fsmonitor ones)", and "the number of controls with NO record is ratcheted" | C5 rewrites it the same way |
| **:672–673** | "controls with NO record … the third quoted argument of a `_control` call" | C5 rewrites it the same way |
| **:684–688** | the ratchet messages, which say "controls" | C5 rewrites them the same way |
| :93, :526 | read and **kept**: "Every control above proves a verdict is REACHABLE" is still true of `_control`s; "Permission denied for EVERY control" is a runtime description | kept |

In the harness and controls, round-6 item 5. Found by the command below, which returns controls:20,
:22, the build region's `_fgit` calls, and harness:25, :35, :45, :46 and :48; the wire returns 0:

```sh
/usr/bin/grep -n -i -e _fgit -e 'two helpers' -e SCRUB .claude/tools/webref-generic-core-trip-wire*.sh
```

| site | statement | fate |
|---|---|---|
| **harness:22–47** | "`_fgit` is what the fixtures are built with; the channels it closes are named at its definition below"; "PER CALL, NOT `export`"; "AND IT SCRUBS THE ENVIRONMENT-PROVIDED CONFIG TOO …"; "there are two helpers: PRESERVE for the repository, SCRUB for the fixtures" | C5 **rewrites** the block. The fixtures are built in a window constructed from nothing (§3). The per-call rationale (#501 R94: the real scan must keep `safe.directory`) is kept, and it now holds because the window is a child process. PRESERVE (`_git`) remains; the fixture half is the window |
| **harness:48–49** | the `_fgit` definition | C5 **deletes** it |
| **controls:20–22** | the list of names this file defines ("`$CTL`, `_fgit`, `_control`, …") and its `grep -c` command | C5 drops `_fgit` from both, because the file no longer defines it |
| controls build-region `_fgit` calls | code | move with C1 to the fixtures file; C5 changes `_fgit` to `git` |

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
| L278–280 | §4's file rows (C1/C2). L279, the harness row ("the fixture git helper"), also changes under C5 |
| L675–677 | §7 criterion 3, "enumerated in `…mutations.sh`" (C2) |
| L683 | §7 criterion 3, "a `sed` expression over the wire" (C4) |
| L747–748 | §7 criterion 5, "All are now in §4's table" (C1/C2) |
| L816 | Slot 2, "when every control has a mutation record" (C5: labels) |
| L866–868 | §9, "the number of controls with no record" (C5 population) |
| L1212–1214 | §11.1's rule "unreachable or loud, never silent", stated as the rule for inputs (§0.1 replaces it for the build) |
| L1220 | §11.1 D6 (§0.1) |
| L1281–1282 | §11.4 P1's file list (C1/C2) |

### §8.3 Memo references in code

⚠ **This table is as of `e8f78896`**: the line numbers are the base files', before C1/C2 split the
controls and mutation files (the parent memo's banner sends readers here for exactly that resolution).

Every `§N` in the parts
(`for f in .claude/tools/webref-generic-core-trip-wire*.sh; do /usr/bin/grep -nE '§[0-9]' "$f"; done`),
plus every prose "the memo" / "plan memo"
(`/usr/bin/grep -n -i -e 'plan memo' -e 'the memo' …`), is resolved by subject:

| file | lines | memo |
|---|---|---|
| wire | 4, 5, 192 (qualified); 38, 158, 195, 206, 225, 451, 461, 534, 550 (K2 predicate §2; exit criteria §12) | A-i memo (`2026-07-citation-hygiene-Ai-spec-label-map.md`) |
| wire | 307 (qualified); 199, 267, 323, 403, 456, 685, 724, 1164 (slots §8; revision §11) | parent |
| controls | 95, 116 (§2) | A-i memo |
| controls | 10, 273, 614, 906 (§8, §11 D10, §11.2, §11.1) | parent |
| mutations | 15 ("the plan memo"), 20 ("The memo") | parent (§10.5, §8) |

The wire is untouched, so this table is the record. In the files this PR edits, C1/C2 qualify every
reference with a file name, and **X4b** checks that **case-insensitively** (§11).

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, the companion, the umbrella rows | docs | the Step-4.5 focused check (E2 and E3) |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (eleven sites) | docs | the §8.2 command, re-read |
| C1 | controls → controls + fixtures; one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4, X4b |
| C2 | mutations → mutations + mutgen; references qualified | prereq split | X1, X3, X4, X4b |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | the `harness:` and `fixtures:` record targets with resolver and restore; the harness's `_shq` comment, which said the mutation set "has nothing to aim at", rewritten because C4 makes it false (`…-reviews.md` §13) | infra, required | X3 |
| C5 | the window (§3), with the fixtures calling `git` and ending with the `built` line; `notcommitted`'s `mkdir`; the incomplete-window exit with its named causes; W2 as `_control`'s first statement and after the controls; W3; the per-option pins and the options re-check; §4's postconditions including P-g's census; the 20 records (23 at head: `/simplify` removed one, `/code-review` added two, `/elidex-review` one; `…-reviews.md` §13); §3's two comment texts; the ratchet population; §8.1's in-file rewrites; the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

**Cost, and `ci.yml`.** The base job comment "a wire that adds fixture self-tests re-derives this line
in the same PR" is an in-file rule (`git show e8f78896:.github/workflows/ci.yml | sed -n
'/^  trip-wires:/,/^  [a-z]/p'`). C5 re-derives the line by that rule's own method, and records the
method and the verdict without figures. If the value would change, **STOP and escalate to the user**.

- **Where the record goes.** A new paragraph in the `trip-wires` job comment block, beside the
  existing "RE-DERIVED AGAIN when the K2 wire's rev-3 controls landed" paragraph, stating the method
  (three runs per side) and the verdict (unchanged or changed).
- **Interaction with #510.** #510 rewrites that same comment block and raises the timeout. Its head
  moves, so read it at the time with `gh pr view 510 --json headRefOid`; it was `94281cd7` when this
  draft was written. Whichever PR lands second carries the other's paragraph forward in its rebase.
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
6. Update the umbrella's single status cell.
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

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | the window's `env -i` closes every variable, and four relocations close default files: class (a). That holds for every git in the window **whose inputs no fixtures-file command removed, overrode or added**, however it is spelled. A command that does is class (b) if it leaves persisted, observable state (P-g, W, W3) and class (c) otherwise, which is out of scope | §0.3, §3, §5.1; corpus G, R |
| 2 | A×D | the postconditions run in the window, so they describe every such git. P-g extends that to the persisted configuration of every git dir under the fixture root; an unknown shape is red | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read. Completion needs the fixtures file's own last line, with the options still on; an incomplete window ends the run with W alone, and `_control` itself refuses over an unbuilt tree (W2). Each child option is checked and recorded. With the parent's nounset off, the clean tree and eight red cells give the same verdicts (measured, §3; not a proof over every path) | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | producers live in the harness and labels in the controls file. Records target `harness`, plus `fixtures` for the W early-return, W options re-check, W3 and P-g records | §4, §6 |
| 9 | D | one label per producer, one record per label; the ratchet stays at 21 | §6 |
| 10 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it | §0.1, §5.2 |
| 11 | F | still one build, now in a child; the ci.yml line is re-derived by rule, and a change means STOP | §9 |

## §11 Exit criteria

These run on both shells, and on both gits wherever the corpus has a column.

| id | command | expected |
|---|---|---|
| X1 | the X1 block below | `rc=0`, `0`, `1` |
| X2 | the §6 G cells (the recipe above, on the implementing head as `p6/`) | all PASS with P equal, 4 configs |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'` | three hits in every column. C1–C3 are byte-identical to base |
| X4 | C1/C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty |
| X4b | the X4b block below, over each file C1/C2 edit | empty |
| X5 | the §6 R, RLOUD, RES and P cells, same recipe; and the draft-10 cells (companion §E.7) on the implementing head as `p11/` | R and RES green with P equal; RLOUD red; P PASS; every AFTER row PASS |
| X6 | planting: a template `config`+`HEAD` in the void; a local `include.path` in a fixture | red: the template through **P-c**, the include through **P-g**. Not P-d: P-d compares two inits that both use the void as their template, so a template planted there is on both sides. Not P-a: P-a reads one probe repo, not the fixtures |
| X8 | the ci.yml rule's derivation (§9) | method and verdict recorded; STOP on change |
| X9 | `Layering trip-wires` on ubuntu (GNU), via route (a) or (b) of §9, chosen by the user at push time | SUCCESS. This is GNU evidence for `env -i`, `env -0`, the window and the prelude. If a record's sed expression reads differently under GNU sed, the always-on anchor check in `_mut_correspondence` is the first thing to fail, on the ordinary run |
| X10 | `$SH -n` and `wc -l` over `ls .claude/tools/webref-generic-core-trip-wire*.sh` | clean; every part below 1000 lines |
| X11 | in a clone, add `_x_lbl="an unrecorded probe"` and an `echo` that uses it | red, and the ratchet lists it |

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

## §12 Plan-review: closed

Plan-review closed after round 9 (`…-reviews.md` §D.0 records the ground). No question is open.
Implementation follows §9.

## §13 Implementation results

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §13 (touch-time split: this
memo holds the live design; the implementation history lives with the review record).
