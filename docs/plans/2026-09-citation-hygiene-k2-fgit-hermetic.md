# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits and false premises. **Review
record**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`, plan-review rounds 1–9
with their dispositions and terminators (split out of the companion unchanged). PR #527's record, its
external review and plan-review rounds 10 onward, is in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md` ("`…-pr527.md`"). **Corpus**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` ("corpus §6"), the corpus cells and the
mutation records (split out of this memo's §6 unchanged). **Landing**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` ("`…-landing.md`"), §9 (the commits,
the land order and the ledger text) and §11 (the exit criteria), split out of this memo unchanged. This
memo holds only the live decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (`…-reviews.md` §8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 16**; the implementation (§9.1) has not been made. Draft 16 answers plan-review
round 14 (`…-pr527.md` §P). What each draft answered goes to `…-reviews.md` §S, not here: drafts 6–16
are there, so this preface does not grow with the drafts.

⚠ **The parent's rule applies throughout.** No quantity here moves with a commit; each figure is a
measurement on a named artifact, given with its command.
- `$S` means a scratch directory.
- The real `HOME` is never written.
- `/usr/bin/grep` is spelled out, because the default `grep` on this machine is ugrep, which can
  silently return 0.
- Disposition labels in the companions never collide with this memo's section numbers: F…, R2-…, R3-…,
  U1–U5 (round 4), V…, W1–W5 (round 6's §D.6 dispositions, not the postconditions W, W2, W3, W4), D…,
  E1–E4 (round 8), A1–A3 (round 9), K10-1…K10-6 (round 10), D13-A…D13-C (round 13), D14-A…D14-C
  (round 14) and PX1 (the base `~+/bin` experiment).
  In this memo a bare "R1"–"R9" is a §5.2 residual. Any other review round names its source: "Codex Rn"
  or "PR #527 Rn" for #527's external review, and "#501 Rn" or "#519's Rn" for the parent PRs' rounds.
  A finding within a Codex round is "Codex R26③". In `…-pr527.md`, whose subject is the Codex rounds, a
  bare "Rn" is a Codex round, and a residual is written "§5.2 Rn".

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
  (`…-pr527.md`, the `~login` measurement).

A tool that only such an entry provided may then not be found inside the window. The window fails, and
that is red, the old rule's "loud". The caller's `git` itself is never lost this way, because it was
resolved before the window started. With `PATH='~+/bin:…'` and a `git` wrapper in the repository's
`bin/`, the wrapper **is** the fixture git under M-PATH: rc 0, PASSED, and every fixture git call goes
through it (plan-review round 10, K10-3, reproduced in `…-pr527.md`). That is R1 working as
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
# harness at `8413a4db`, except the line marked PLANNED, which C6 (§9.1) changes; the harness is the source
_FGIT_VOID="$SCRATCH/fgit-void"             # mkdir, checked
_FGIT_BIN="$SCRATCH/fgit-bin"                # holds `git`: exec <this shell's `type -P git`, absolute>
_FGIT_PATH="$_FGIT_BIN:$PATH"               # PLANNED (C6): the caller's PATH verbatim, behind the pin (§0.1)
_FGIT_ENVBIN=<this shell's `type -P env`, absolute>; _FGIT_BASH="$BASH"
_FGIT_ENV=("PATH=$_FGIT_PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID" GIT_DEFAULT_REF_FORMAT=files "LC_ALL=C")
# _fgit_window: write a prelude of PLAIN assignments (labels by name) and function bodies, then
"$_FGIT_ENVBIN" -i "${_FGIT_ENV[@]}" "$_FGIT_BASH" -c '
  cd "$1" || <cause; exit>                     # $_FW_DIR, a directory the window owns
  . ./prelude.sh                                # both copied here by fixed relative names (Codex R2)
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
stderr directly.

**An incomplete window is not "no fixture failed" (round-6 item 1; D4).** Completion needs two
markers:
- **`built`**: the fixtures file's **own last line** writes it. So a top-level `return` or `exit` inside
  the file leaves the window incomplete, and "the child script finished" is not enough.
- **`done`**: written after the postconditions run.

Suppose either marker is missing. Nothing was built, so **no `_control` runs**.
The incomplete-window exit (`_fgit_window_incomplete_exit` at `8413a4db`, `_fgit_window_verdict_exit`
after C8) prints W alone with the recorded cause and **exits 2**.

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

**No time bound (declared, §5.2 R9).** A child can still block on what the fixtures built where no
shape check sees it: git reads, outside every census git dir, something whose read does not complete
(a FIFO; `/dev/tty` under a controlling terminal), through a reference the census does not follow (§4,
"The boundary"). The run then waits: in CI until the job timeout, which is red; locally,
indefinitely. It is never green. The one watchdog is `_control`'s own (#501 R92), unchanged. Draft 12
bounded the whole run; round 11 found seven defects in that design, and the user carved it out of this
PR into `#11-trip-wire-liveness-bound`.

**`ci.yml`.** C9 and C10 add one `find` and one `git count-objects` per fixture git dir to a clean
run; X8 measures the result against §9's threshold. The trip-wires job comment's verdict line, which
C5 wrote as the method plus one verdict naming the commits, names base `e8f78896` and **the tool code
at T**, where T is the last commit that changes `.claude/tools/**` or `scripts/**` (§9.1). After C6–C10
that line is stale until X8 measures T and rewrites it.

**A red postcondition ends the run before any control.** A red P-a…P-k, or W3, means the build is not
the one the fixtures file describes. A control over it asserts nothing, and it can block. In Fable's
M-SHAPE prototype log (`fable/e1/e4e.log` in the orchestrating session's scratchpad; one run, not
re-measured), a FIFO `.git/commondir` made P-g red, and then the first `_control` was killed at 30 s.
The Codex R26③ cells of draft 13, with a stand-in stop, ended in 5–8 s with no control run
(`…-pr527.md`).

So the window's verdict is reported in **one place**, one harness function,
`_fgit_window_verdict_exit` (C8 renames `_fgit_window_incomplete_exit`). The controls file calls it
after `ctl_ok=0`, and W3's report moves into it from the controls file. W4's report stays in the
controls file, because W4 does not stop the run. The function is the one writer of **`_fw_trusted`**
(1 only for a build that is complete, with no postcondition and no W3 reported), and the gate
`_fw_built_or_w2` is its one reader, deciding by `_fw_trusted` alone. The function continues only over a
build that is **complete and trusted**:
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
- **W** fires when the window is incomplete.
- **W2** fires if any control is reached over a build that is not complete and trusted. W2 is `no
  control runs over an incomplete or untrusted fixture build window` (draft 10's label said
  "incomplete" only). Its predicate is one function, `_fw_built_or_w2`. **`_control`
  calls it as its first statement** and refuses unless the window is complete and trusted (E3), and each of the
  three blocks that are not `_control`s (relative scratch, fsmonitor, umask) asks it before it runs.
  So no control runs over such a build, wherever the exit sits. **Its three records pin
  that W2 is reported when the exit is gone**. One removes the incomplete-window `exit`. The other two
  each remove the untrusted-build `exit` inside `_fgit_window_verdict_exit`, one with a postcondition
  forced red (`post_bad`), the other with W3 forced (`_fw_diag` set), so both clauses of "trusted" are
  pinned. `_control`'s gate alone satisfies all three. ⚠ **Declared gap:** removing
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
| `PATH=$_FGIT_PATH` | BSD `env -i` runs `/usr/bin/git` rather than `PATH`'s git (INFO cell). The first entry is `$_FGIT_BIN`, a wrapper that execs the `git` **this shell's own lookup** resolves. The lookup is `_fgit_resolve`, the one resolver for every command that crosses the window boundary by path, `$_REAL_GIT` and `$_REAL_GREP` included. After it comes the caller's `PATH`, **verbatim**. No entry is reinterpreted, dropped or emulated, because which executable runs is R1's (§0.1), and what P needs is only that the window's `git` is the pinned one. An entry means whatever it means from inside the window (relative, empty, `~`: §0.1). Since `git` is pinned, that decides only which other tools are found, and a missing one fails the window: red. The fixtures' other commands come from these entries (§1, "Outside P"). P-j pins the construction. History: `…-pr527.md` |
| `HOME=$VOID` | an unset `HOME` also closes this on 2.55. The void is chosen because a future HOME-relative default then lands where P-c looks |
| `GIT_CONFIG_NOSYSTEM=1` | the system layer is live here (`/opt/homebrew/etc/gitconfig`) |
| `GIT_ATTR_NOSYSTEM=1` | undocumented at 2.55: 0 hits in all 207 man pages of 2.55.0 (command below; positive control: `CONFIG_NOSYSTEM` hits 2 pages). So it is pinned by `git var` (P-b) |
| `GIT_TEMPLATE_DIR=$VOID` | the compiled-in template is copied in |
| `GIT_DEFAULT_REF_FORMAT=files` | a git whose compiled-in default is reftable (git 3.0's planned default, or a breaking-changes build) makes every init differ from every other and `badref` write refs reftable does not read (R8). Gits before 2.45 ignore it. Pinned by P-i |
| `LC_ALL=C` | keeps the window's locale the one the wire exports (wire:319), so the build is like-for-like with base. Pinned by P-h |

```sh
find -L /opt/homebrew/opt/git/share/man -type f | wc -l                                         # 207 (git 2.55.0)
find -L /opt/homebrew/opt/git/share/man -type f -exec /usr/bin/grep -l ATTR_NOSYSTEM {} +; echo "rc=$?"   # rc=1
find -L /opt/homebrew/opt/git/share/man -type f -exec /usr/bin/grep -l CONFIG_NOSYSTEM {} + | wc -l     # 2 (the positive control)
```

**The fixture change.** `notcommitted` gains `mkdir -p .git/info`, because the empty template no
longer creates it (companion §A.2).

**errexit and state.** Every window state name (`_fw_rc`, `_fw_done`, `_fw_post_bad`, `_fw_why`, and
C8's `_fw_trusted`, 0 until the verdict function sets it) is
assigned at harness top level, before anything reads it (`_fw_diag` and the W2 flag included). The
child runs under `set -euo pipefail` with **no EXIT trap**. Each of those options is checked separately
before the fixtures file (one record per option, corpus §6), and all three are checked again after it (one
record). So an abort there
is a non-zero exit, which the parent reads as data. The verdict is written after `ctl_ok=0`
(`controls.sh:738` at base), and a window that is not complete and trusted ends the run right there.

**The parent's reliance on `set -u`, measured (IMP-2).** With the wire's `set -euo pipefail` changed to `set -eo pipefail` (nounset OFF in the parent), the
clean tree and eight cells (sealfail, env0, garbagehead, w2rec, lblrename, w3ar, sealdotdot,
reftable; six red, and env0 and reftable green at `8413a4db`) gave the same exit status, the same NE/CF counts and the same first four `!!` lines as with it on:
- on bash 5.3 at the `/elidex-review` head (companion §A.14);
- **on bash 3.2 and 5.3 at `8413a4db`**, 36 runs. Bash 3.2 is where it matters: there
  `#11-k2-wire-exit-trap-masks-set-u-abort` turns an unbound-variable abort into rc 0.

The cell script is verbatim in `…-pr527.md` ("X13"), and X13 re-runs it at the final head. That
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
| W | `the fixture build window completed` | the fixtures file's last line wrote `built`, the options were still on after it, and the child wrote `done`. Otherwise NE with the sentence the child wrote to `cause`, or the exit status (§3), **reported alone**; exit 2; no control runs | — |
| W2 | `no control runs over an incomplete or untrusted fixture build window` | `_control`'s first statement, and the first thing each non-`_control` block asks: the window is complete, and no postcondition or W3 reported. Its records: each of the two exits removed (§3). The exit placed below a control: the RO cells | — |
| W3 | `the fixtures file ran without a shell diagnostic` | no record of the child's stderr contains `./fixtures.sh:` or `./prelude.sh:` **anywhere in it** (both are sourced by those relative names from the window's directory; bash appends a diagnostic to a record the fixtures left without a newline, §3). A failed scan is red | — |
| W4 | `every mode restriction a fixture sealed was applied` | every `_seal` was accepted (a path under `$CTL`, with no newline or TAB) and its `chmod` succeeded; otherwise red, not a machine limitation | — |
| P-a | `a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file` | every `git config --list --show-origin` line in **one probe repo** has the origin `file:.git/config` | `this git reports a configuration origin outside the repo's file`: `-c a.b=c` must show with the origin `command line:`. `--show-origin` (git 2.8) alone, not `--show-scope` (git 2.26): the origin column answers the same question, so the check has no version floor and no limitation arm |
| P-b | `the fixture git has no system or global layer outside the void` | asked **inside P-a's probe repo** (`git -C`), so no caller repository's local configuration is read. A git older than 2.42, whose `git var` cannot name these four (exit 129), is a machine limitation: `⚠ NOT EXERCISED on this machine`, green. Otherwise `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is an empty directory, tested by the shell's own globs (`*`, `.[!.]*`, `..?*` with `-e`/`-L`) — not by `$(ls -A …)`, which loses a name made only of newlines and reads a failed `ls` as empty. A void that is not a directory, or cannot be read and searched, is red: globs over it would expand to nothing. The globs run with `set +f` and `GLOBIGNORE` unset, whatever the fixtures file accidentally left (a deliberate `readonly GLOBIGNORE` is threat-model class (c)). Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `no exec-path override reaches the fixture git` | the window's `git --exec-path` equals the same git's answer with nothing but `PATH` in its environment (taken in the parent at source time), both canonical (`pwd -P`). **Re-scoped by `/code-review`:** which executable runs is outside P (§0.1, R1); P-e pins only that nothing in the window overrides where that git runs its commands from. So a caller's `GIT_EXEC_PATH` or `DEVELOPER_DIR`, or one directory spelled two ways, is not a red | — |
| P-f | `the fixture build window's environment holds only its allowlist` | the name of **every** `env -0` record (the text before the first `=`, so non-identifier names such as `BASH_FUNC_f%%` are included) is an allowlist name (derived from `_FGIT_ENV` itself, so the two cannot drift) or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction. p6's `sed` parser skipped non-identifier names; p7 parses every record (companion §A.10). `env -0` goes through a file whose status is checked: a failed `env -0`, or no record at all, is NOT EXERCISED. **One** outcome is a machine limitation instead (`⚠ NOT EXERCISED on this machine`, green — the treatment P-b and the FIFO and file-permission controls give a machine that cannot run them): its test is the definition — **this** `env` runs but refuses `-0`: `env -0` fails, writes nothing to stdout and says why on stderr, while the same `env` runs a command. Every other outcome stays red, so an unknown one falls on the fail-safe side. Which `env` builds lack `-0` is not measured here (this machine's macOS 26 `env` has it; PR #527 Codex R4 reported a macOS one without it); the no-`-0` case is exercised with a shim. Both limitations reach the run's summary through one channel (`$_FW_DIR/machine_limits`), naming the postcondition by ID only, since a record's needle is its label. History: `…-pr527.md`. | — |
| P-g | `every fixture repo persists only the configuration a plain git init writes` | for **every git dir under the fixture root** (population below), the lines of `git -C <repo> config --list --show-origin` (no `--show-scope`: the origin already names the file) **equal, as a set,** the lines of a **reference** `git init` made in the same window (P-a's probe repo, which is that init, made after the build in a directory no fixture can reach: "The references", below), compared by origin, key and value (`grep -vxF -f`, both directions). An extra line is an input the fixture persisted; a missing one (`git config --unset core.filemode`) hands that setting to the platform default — either way P would depend on more than the fixture. A `grep` that fails (exit above 1) is a failed comparison, red — never an empty difference. The authoritative comparison is of **records**: the `-z` listing, each origin paired with its entry into one record written as one line by `printf %q`, sorted (no `sort -z`), byte-identical to the reference's — so a value holding a newline cannot forge a line, and a repeated entry is red. This catches a persisted include (its origin is not `.git/config`), a `commondir` naming another git dir (the origin names that dir's config), and **any** persisted key beyond init's, `core.excludesFile` included. **Census first (M-SHAPE, drafts 14–15):** the census classifies and shape-checks every git dir it names, and runs its per-link search, and its one verdict comes before any postcondition runs git (population below). An entry under a `.git` that is not a directory or a regular file with one link is red: a symlink of any target, a FIFO, a socket, a device, a hard link. Git reads through a link (a `.git/config` pointing outside still reports `file:.git/config`) and blocks opening a FIFO, whatever the entry's name. **A red census runs no git anywhere**, because git reads across git dirs (`commondir`, `alternates`, `include.path`). A scan that fails or writes to stderr is red. The census takes `HEAD` and `.git` entries of **any** type and letter case — a symlink `HEAD`, or `head` on a case-insensitive filesystem, is a shape git reads — and every git dir not reached by a `.git` entry is red. **Both directions:** a git dir whose listing is EMPTY or fails (a `.git` git does not recognise, e.g. a garbage `HEAD`) is red | — |
| P-i | `the fixture repos use the files ref format` | the window's `GIT_DEFAULT_REF_FORMAT` is `files`, and a plain init's `git rev-parse --show-ref-format` answers `files` (a git before 2.45 has no reftable; its `rev-parse` echoes the unknown option back with exit 0, and that literal echo is what counts as `files` — a failed call or any other format name stays red). It pins the allowlist entry on every git, including the files-default ones where dropping it changes nothing else | — |
| P-j | `the fixture build window runs the pinned git first on its PATH` | the window's `PATH` starts with `$_FGIT_BIN`, and `type -P git` inside the window is `$_FGIT_BIN/git`. It pins the `PATH` construction the way P-h and P-i pin their entries: without the pin, the window's first `git` is the file the pin execs anyway, so nothing else changes, and P-e's reference moves with it. It asserts nothing about the caller's entries (§0.1) | — |
| P-k | `no fixture repo reads objects from a store outside it` | for every `.git` P-g compares, `git count-objects -v` exits 0 and prints no `alternate:` line. An `objects/info/alternates` naming another store makes git read an object from there instead of writing it locally, so the content the controls read is an outside input even though P's ids do not move (measured, `…-pr527.md`). Git's own answer, not a file name: it reports the stores git will read (measured for the `alternates` file). A failed `count-objects` is red, and **so is any stderr**: a store that does not exist, or an `alternates` naming a directory or a mode-000 path, gives rc 0, no `alternate:` line and a warning or error on stderr (round 12; `…-pr527.md`), and such a store could appear later and be read. It runs only over a clean census (§4, population) | `this git reports alternate object stores`: a third probe repo in the references' directory (so P-d's two inits are untouched), given an `alternates` file naming P-a's object store, must exit 0 with nothing on stderr and print exactly one `alternate:` line. The file holds the path in double-quoted C form, so a newline in the scratch path stays one line. `gitrepository-layout` says only "one pathname per line". git(1) documents C-style quoting for `GIT_ALTERNATE_OBJECT_DIRECTORIES` (`man git`, that variable), and that the `alternates` file accepts the same form is measured: on 2.55.0 a path holding a newline gives one `alternate:` line, rc 0 (`…-pr527.md`, round 12's P-k commands). **Anything else is red**, NOT EXERCISED: there is no machine-limitation arm, because "prints none" cannot tell a git without the line from a broken probe (git 2.55.0 and Apple 2.54.0 both print it, measured), so the unknown falls on the fail-safe side |
| P-h | `the fixture build window reads in the wire's locale` | the window's `LC_ALL` equals the wire's (`C`). It pins the allowlist's `LC_ALL=C`, which P-f cannot, because P-f derives its names from the same list | — |

**How P-g's key set is derived (D2).** It is **not** a list of path-typed keys. Git exposes no per-key
type: `--type=path` is chosen by the caller, not declared per key, and `git help config` is prose. A
list of path keys would therefore be vocabulary, and a key missing from it would pass silently.
Instead, the allowed set is **what git itself writes on `init`**, measured in the window on every run.
That is the fail-safe direction: any persisted entry beyond it is red, whether or not it names a path.

**P-g's population, and the census before any git (drafts 14–15).** The population is the git dirs
the census can **name**: a `.git` entry, or a directory holding a `HEAD` next to `objects` or
`commondir`. That is a classification by name and `HEAD`, not git's whole read set. What git reaches
through a reference is §5.2 R9's boundary (below). The census runs **in two passes, and writes its
verdict, one variable `_pg_census`, only after both.** The postconditions start with the census, and
when `_pg_census` is red, `_fgit_postconditions` reports P-g and returns before any other postcondition,
so no git runs. The census covers the git dirs under `$CTL` that it names. It does not cover what git
reaches through a reference ("The boundary", below), nor the reference repos, which are outside fixture
reach by construction ("The references", below):
1. **Classification and shape.** One `find` from `$CTL`, with no `-L` and no per-directory fork, lists
   every `.git` entry and every `HEAD` entry, of **any type and any letter case** (a case-insensitive
   filesystem lets git read `head` and `.GIT`, so a case variant is red). It descends everywhere:
   hidden and nested directories, and the inside of `.git` directories.
   - A `.git` that is a gitfile, a symlink or anything else but a directory is red: `[.git is not a
     directory]`.
   - A `HEAD` whose directory is not a `.git` and holds an `objects` directory **or** a `commondir`
     file is a git dir not named `.git`, and it is red. That covers a bare repo, a
     `--separate-git-dir` target, a submodule's git dir under `.git/modules/`, and a linked
     worktree's entry under `.git/worktrees/`.
   - A `.git` that is a directory has its **shape** checked: every entry under it must be a directory
     or a regular file with **one** link (`find <.git> \( \( -type f -links +1 \) -o \( ! -type f ! -type d \) \) -print`).
     A symlink of any target, a FIFO, a socket or a device is red, because git reads through a link
     and blocks opening a FIFO. So is a hard link, which shares its content with a path outside the
     git dir. A scan that fails or writes to stderr is red.
   - "Cannot search" is whatever `find` reports: any report on stderr, or a non-zero exit, fails the
     census. A census that finds no git dir is red too.
2. **The per-link search**, only if pass 1 found nothing red. A second `find` from `$CTL` lists every
   symlink. A link that resolves to a directory is searched through (`find -L`, any depth) whatever its name, and a `HEAD` or `.git` anywhere under it
   is red, as is a search that exits non-zero. BSD find lists a loop link without descending (rc 0,
   hiding nothing); GNU find is expected to exit 1, which is not measured here. A link *named* `HEAD`
   to a directory is red whatever it holds, because the search's start matches its own name; the
   record for that shape pins the routing to the search, and the two-levels-down record pins the
   depth.

   A link inside a git dir that pass 1 classified is already red by pass 1, so it is never searched.
   That holds for a `.git` by its shape, and for a bare dir by its name.

**If the census is red, no postcondition runs git**, on any repo: the census reports P-g, and the
build is untrusted (§3). Git reads **across** git dirs, through a `commondir`, an `alternates` chain or
an `include.path`, so both verdicts must be in before git runs anywhere.

Drafts 13 and 14 let git run before a verdict was in (round 12's `xcommon`, round 13's `xlink`); with
one verdict after both passes, both cells are red in 3–5 s on both shells (`…-pr527.md`, rounds 12–13).

**The boundary, by property (§5.2 R9).** Shape covers every git dir the census names. The census does
**not** follow `commondir`, `include.path` or `alternates`: that would emulate git's read set, the
anti-pattern this memo removed for `PATH` (§0.1: the window does not emulate the lookup). What git reads
outside a census git dir is therefore either **compared** or, if the read does not complete, **waited
on**; it is never green:
- **compared**, when the read completes. A reference that reaches configuration (`commondir`,
  `include.path`) puts what it read into `config --list --show-origin`, so P-g's origin comparison reds
  it (`xincreg`, an include naming a regular file with a key: red in 6–8 s, origin
  `file:.git/../../zzi`), and a read that fails fails that listing (an include naming `/dev/tty` with
  no controlling terminal: `fatal: bad config line`, git exit 128, red). An `alternates` naming an
  ordinary store outside leaves that listing unchanged (measured, `…-pr527.md` round 14), so P-g does
  not see it; **P-k** does (`count-objects -v` prints its `alternate:` line);
- **waited on**, when the read does not complete. The run waits, in CI until the job timeout (red).
  Measured on both shells (inserts in `…-pr527.md`, rounds 13–14). Four cells reached `cell15.sh`'s
  120 s alarm with the P-g `git … config --list` blocked in the run's group: `hdless` (a `commondir`
  naming a directory with `objects` and `refs` but no `HEAD`, whose `config` is a FIFO), `xinclude` (an
  include naming a FIFO in the fixture root), `xinclnk` (an include naming a link in the fixture root to
  a FIFO outside it) and `outroot` (a `commondir` naming a git dir outside the fixture root whose
  `config` is a FIFO). `ttyinc` (an include naming `/dev/tty`, under a pseudo-terminal that is the
  wire's controlling terminal) reached that harness's 120 s limit, and `ps` showed the `config --list`
  blocked (bash 5.3). A chained `alternates` whose store's own `alternates` is a FIFO blocks
  `count-objects -v`, P-k's read (a git-only measurement, its setup verbatim in `…-pr527.md` round 14);
- the per-link search over an otherwise clean census reads whatever a link reaches, so a link to a huge
  tree is **slow**: `find` ends when it has walked what the link reaches. Round 12 reported 455 s for
  its `outsidein` cell, whose search went through a link to `/` (`…-pr527.md`, round 12). Draft 15's
  census reds that cell by pass 1 in seconds; a link to `/` over a clean pass 1 has not been
  re-measured.

**The references, outside fixture reach by construction (draft 16).** At `8413a4db` P-a's probe repo
`a` and P-d's second init `b` are made at the fixed path `$_FW_DIR/pq`, which the fixtures can write,
and `git init` over an existing repo keeps its configuration. Round 14's `xrefpoison` made both in
advance with `core.excludesFile /nonexistent-k2` and wrote the same key into every fixture repo: rc 0,
PASSED, on both shells. Every reference a postcondition compares against, by location:
- **`a`, `b`, P-g's reference listings, and P-k's probe repo (C10)**: moved into a directory made
  **after the build** by `mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX"`, beside `$_FW_DIR` and never under it
  or `$CTL`, whose name no fixture can know. `a` and `b` are made by `mkdir` without `-p`, so one that
  already exists fails the reference, red (P-g). P-a, P-b, P-d, P-g and P-i read `a`.
- **P-e's `_FGIT_WIRE_EXEC`, P-h's `_FGIT_WIRE_LC`, P-f's `_FGIT_ENV_NAMES`**: values the parent takes
  before the window starts and passes as prelude assignments, not files. The fixtures file shares the
  postconditions' shell, so an assignment to one there is not checked, any more than one that redefines
  a postcondition function. No fixture names any of them (`/usr/bin/grep -c -e _FGIT_WIRE_EXEC -e
  _FGIT_WIRE_LC -e _FGIT_ENV_NAMES -e _fgit_postconditions` over the fixtures file → 0).
- **P-j's `$_FGIT_BIN/git`** is compared by path. Its content decides which executable runs, R1's.
- **P-c's void** is the subject, not a reference: P-c reds any entry in it, and both reference inits
  read it as their template, so a template planted there is red through P-c (X6).

What stays in `$_FW_DIR` is the subject side. A fixture-placed file there (the census lists, `pgcur`,
`env0`) can make a postcondition red or wait (a FIFO), but it cannot make a comparison equal: the
reference is at a name it cannot know. ⚠ **Open, not dispositioned by draft 16:** the window's return
channel (`built`, `done`, `post_bad`, `machine_limits`, …) is in `$_FW_DIR` too, because both sides
name it, and it can be forged. `xdone` (a fixture persists `core.excludesFile` in `clean`, writes
`built` and `done` itself, and exits 0 before the postconditions) gives rc 0, PASSED, on both shells
(`…-pr527.md` round 14). It leaves persisted state, so §0.3 does not put it in class (c).

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
  prototype's clean run is green on both shells, `…-pr527.md`), and the three declared directories are at their modes, so the clean and G cells
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

**Accounting:** 1 own deferral, `#11-trip-wire-liveness-bound` (§5.2 R9, draft 13). With
`#11-k2-fixture-git-invocation-convention` closed and the keep-set slot dissolved below, the ledger nets to **−1**.

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
| R9 | **no time bound**: a child whose read does not complete makes the run wait. By property: git reads, outside every census git dir and through a reference the census does not follow (`commondir`, `include.path`, `alternates`), something whose read does not complete (§4, "The boundary": a FIFO, or `/dev/tty` under a controlling terminal); a read that completes is compared instead, by P-g's origin or by P-k. The per-link search over a huge tree is slow (`find` ends when it has walked it). The one watchdog is `_control`'s (#501 R92), and it covers a control's run only | never green: red at CI's job timeout; a local run waits | **slot `#11-trip-wire-liveness-bound`**, text below. C9 rewrites the harness's declared blind spot ("There is no time bound…") to name it. The user carved draft 12's bound out (2026-09-28) |

R7's command, together with a negative case that shows it discriminates:

```sh
sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml | /usr/bin/grep runs-on   # runs-on: ubuntu-latest
sed -n '/^  check:/,/^  [a-z]/p' .github/workflows/ci.yml | /usr/bin/grep runs-on        # runs-on: ${{ matrix.os }}
```

**R9's slot, `#11-trip-wire-liveness-bound`: the text §9's ledger step writes.**
- **Gap**: the K2 wire's harness bounds only `_control`'s child (30 s; #501 R92, present at base
  `e8f78896`). Every other child that runs over fixture state is unbounded, so a block there gives no
  green, but the gate reaches no verdict: CI reds at the job timeout, and a local run waits.
  - **Pre-existing at `e8f78896`**:
    - the plain `--selftest` calls of the relative-scratch and umask blocks, and the fsmonitor
      block's `git ls-files` and `--selftest`;
    - the fixture build, then run in the parent through `_fgit`;
    - the mutation runner's trials.
  - **New in #527**:
    - the fixture build window, which replaces the parent build;
    - C9's two-pass census (the classification and shape pass, then the per-link `find -L` search,
      which #527's census already ran at `8413a4db`);
    - C10's `count-objects` per repo and its probe.
  - **Measured blocks**: a read that does not complete, which git makes through a reference outside
    every census git dir. Each ran as its own process group and reached its 120 s limit on both
    shells; the inserts and the cell scripts are verbatim in `…-pr527.md`, rounds 13–14:
    - `outroot`: a `commondir` naming a git dir outside the fixture root, its `config` a FIFO;
    - `hdless`: a `commondir` naming a directory with `objects` and `refs` but no `HEAD`, its `config` a
      FIFO;
    - `xinclude`: an `include.path` naming a FIFO in the fixture root;
    - `xinclnk`: an `include.path` naming a link in the fixture root to a FIFO outside it;
    - `ttyinc`: an `include.path` naming `/dev/tty`, under a controlling terminal (without one, git
      exits 128 and the run is red).

    A chained `alternates` whose store's own `alternates` is a FIFO blocks `git count-objects -v`, P-k's
    read: rc 142 at a 10 s alarm on git 2.55.0 (a git-only measurement; `pkmeas.sh`, verbatim in
    `…-pr527.md` round 14).
- **Defects measured in the two withdrawn designs** (`…-pr527.md`, rounds 10–11; rounds 12–14 then
  measured the blocks above):
  1. nested process groups escape an outer group kill, and `$(…)` then waits on the pipe;
  2. a call-site list misses sites (fsmonitor's `git ls-files`);
  3. a re-run of the wire as a new group becomes a background job on a tty, so `stty tostop` stops
     it: a false red;
  4. with trials nested, the INT/TERM exit path's `kill -9` does not reach the trial groups;
  5. a failing process-group probe re-runs the wire without end;
  6. a join decided by an environment variable alone drops every bound, against the wire's rule
     `git show 8413a4db:.claude/tools/webref-generic-core-trip-wire.sh | sed -n 350,364p` ("ENTERED BY
     ARGUMENT, NEVER BY ENVIRONMENT");
  7. a check for `set -m` by spelling misses `set -o monitor` and `-eum`;
  8. a 0-bound phase, or a top-level cap of 0, leaves call sites unbounded;
  9. a parent that relays a child's rc relays bash 3.2's masked rc 0, which widens
     `#11-k2-wire-exit-trap-masks-set-u-abort`.
- **Why deferred**: the legitimate reason is **L3**. A bound over nested children is a load-bearing
  change of its own: edge-dense, so it needs its own plan and plan-review under CLAUDE.md's rule. L2
  also holds, since #527's memo records two designs measured and withdrawn. The confirming questions:
  1. spec faithfulness: no spec surface;
  2. one issue, one way: the only bounded child, `_control`, predates #527, and #527 adds no second
     mechanism;
  3. anti-justification: no, it is not size or session;
  4. **repeat signal: yes.** Codex R25/R26 and plan-review rounds 10–14 raised it. By the lens that
     means fix-in-PR, and only the **user's explicit carve (2026-09-28)** overrides it. That decision
     is the ground here, not the lens.
- **Trigger**: #501's squash merge into `main`, which carries #527's changes (#527 is squashed into
  #501's branch, not into `main`, and its commits are not reachable from `main`) and opens the
  dedicated slice; or, before that, the next PR that adds
  a child to the K2 harness or the mutation runner, or any report of a K2 run that waited instead of
  reaching a verdict.
- **Owner**: the citation-hygiene lane. **Timing**: a slice of its own, planned and plan-reviewed after
  #501's squash merge. **Re-eval**: 2026-11-30.
- **Accounting**: #527's own deferrals, 1.

**Pre-existing, not fixed here: `#11-k2-wire-exit-trap-masks-set-u-abort`.** On `/bin/bash` 3.2,
`set -euo pipefail` together with the wire's EXIT trap (since base, wire:405) turns an
unbound-variable abort into exit 0. The orchestrating session reproduced it (`…/scratchpad/r4ax2/u.sh`:
3.2 rc=0, 5.3 rc=1). The slot's owner is the **citation-hygiene lane**, and its ledger trigger is
"code sourced after wire:405".

This PR's harness and controls are exactly such code. **The ledger text — the one text; §9's ledger
step writes exactly this** — **replaces**, in place, the entry's sentence "(its memo U5: state
initialised at harness top level, setup failure a labelled verdict)" and the amendment written at PR
creation. It does not append to them:
- **measured, not argued:** with the wire's nounset off, the clean tree and eight cells (six red) give the
  same exit status and verdict lines as with it on (bash 3.2 and 5.3; the cell script is verbatim in
  `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md`, "X13", and X13 re-runs it at this PR's final head).
  On those runs no parent-side verdict depends on `set -u`; that is a measurement over those runs, not a
  proof over every path;
- an incomplete fixture-build window is reported by the **W verdict alone**, and no control runs:
  `_control` asks `_fw_built_or_w2` first, and so does each of the three blocks that are not
  `_control`s (relative scratch, fsmonitor, umask). That covers a child that refuses a prelude missing
  any one of `errexit`, `nounset` or `pipefail`, a fixtures file that stops before its last line, and
  one that switches an option off. A build that is complete but untrusted (a red postcondition, or W3) ends
  the run after its reports, and no control runs over it either;
- a mode restriction a fixture sealed and the window could not apply, or refused, is red (W4);
- pinned by the W records (one per prelude option, one for the early return, one for an abort, one for
  an option switched off by the fixtures file), the three W2 records and the W4 record. The three block gates
  are pinned by the traced `w2rec` cell only (§3's declared gap).

## §6 The corpus — evidence, and the source of the records

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` §6 ("the corpus file";
touch-time split, text unchanged). It holds the recast oracle, the prototypes p6–p11, the re-run
recipe, every focused cell table, and the mutation records with their totals and the ratchet;
its §6.1 holds drafts 11–16's planned record changes.

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

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` §9 (touch-time split, text
unchanged): the commits, the cost and `ci.yml`, X9's routes, the land order, the ledger text, and §9.1,
the commits planned on top of `8413a4db`.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | the window's `env -i` closes every variable, and four relocations close default files: class (a). That holds for every git in the window **whose inputs no fixtures-file command removed, overrode or added**, however it is spelled. A command that does is class (b) if it leaves persisted, observable state (P-g, W, W3) and class (c) otherwise, which is out of scope | §0.3, §3, §5.1; corpus G, R |
| 2 | A×D | the postconditions run in the window, so they describe every such git. P-g extends that to the persisted configuration and the shape of every git dir under the fixture root: an unknown git-dir shape is red, and so is any entry inside a git dir that is not a directory or a regular file with one link. The census's one verdict comes before any postcondition runs git, and the references it is compared against are made after the build where no fixture can reach them. P-k adds that no fixture repo reads objects from a store outside it | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read. Completion needs the fixtures file's own last line, with the options still on; an incomplete window ends the run with W alone, a complete but untrusted one ends it after its reports, and `_control` itself refuses unless the build is complete and trusted (W2). Each child option is checked and recorded. With the parent's nounset off, the clean tree and eight red cells give the same verdicts (measured, §3; not a proof over every path) | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | producers live in the harness and labels in the controls file. A record targets `fixtures` when what it plants is something a fixture writes (the W early return and options re-check, W3, and the P-c, P-g and P-k shapes); a record that changes a producer targets `harness` | §4, corpus §6 |
| 9 | D | one label per producer, one record per label; the ratchet stays at 21 | corpus §6 |
| 10 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it, so the window's `PATH` is the caller's, verbatim, behind the pin, and P-j checks only the pin | §0.1, §5.2 |
| 11 | F | still one build, now in a child; the ci.yml line is re-derived by rule, and a change means STOP | §9 |

## §11 Exit criteria

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` §11 (touch-time split, text
unchanged), beside the commits they verify.

## §12 Plan-review

Plan-review closed for draft 10 after round 9 (`…-reviews.md` §D.0 records the ground). Rounds 10–14
reviewed drafts 11–15 (`…-pr527.md`). Draft 16 makes the references by construction, restates R9's
boundary by read completion, and gives the final-head sequence a terminator, so **round 15**, a focused
re-check of the draft-16 delta, reviews it before C6. It has not run.

## §13 Implementation results

The results before PR #527 are in `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §13
(touch-time split; history). Every result from PR #527 on, drafts 11–16's commits included when they
are made, goes to one place: `…-pr527.md` §P (`…-landing.md` §9.1).
