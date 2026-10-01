# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits and false premises. **Review
record**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`. Its §D holds plan-review
rounds 1–9 with their dispositions and terminators (split out of the companion unchanged); §S this
memo's status narrative (moved from its preface, and each draft's status is added there since); §8 the
code splits, the parent memo's banner and the memo references, and §13 the implementation results
before PR #527 (both moved from this memo). PR #527's record, its
external review and plan-review rounds 10–14, is in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md` ("`…-pr527.md`"), and from round 15 on,
with the implementation results, in `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527-r15.md`
("`…-pr527-r15.md`"). **Corpus**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` ("corpus §6"), the corpus cells and the
mutation records (split out of this memo's §6 unchanged). **Landing**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` ("`…-landing.md`"), §9 (the commits,
the land order and the ledger text) and §11 (the exit criteria), split out of this memo unchanged.
**Residuals**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-residuals.md` ("`…-residuals.md`"),
§5 (class (c), the residuals R1–R9 and the pre-existing defect), split out of this memo unchanged at
`f8ea983a`; extended since by D17-A and D18-A (§5.1). This memo holds only the live decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (`…-reviews.md` §8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 21**, implemented (§9.1: C6…C12, then the `/code-review max` fix round
`976e1430`, `15d3e704`, `6ed414b2`, the /simplify round `92221297`, review round 2 `96eea58c`, `3dde656e` (a window that lost its own directory, named by the parent) and Stage 5's `8f4ad3c8` (the orderly-end flag; unreadable markers untrusted), which is T;
results in `…-pr527-r15.md` §Q). Draft 21 answers plan-review round 19 (`…-pr527-r15.md` §Q). What
each draft answered goes to `…-reviews.md` §S, not here: drafts 6–21 are there, so this preface does not grow with the drafts.

⚠ **The parent's rule applies throughout.** No quantity here moves with a commit; each figure is a
measurement on a named artifact, given with its command.
- `$S` means a scratch directory.
- The real `HOME` is never written.
- `/usr/bin/grep` is spelled out, because the default `grep` on this machine is ugrep, which can
  silently return 0.
- Disposition labels in the companions never collide with this memo's section numbers: F…, R2-…, R3-…,
  U1–U5 (round 4), V…, W1–W5 (round 6's §D.6 dispositions, not the postconditions W, W2, W3, W4), D…,
  E1–E4 (round 8), A1–A3 (round 9), K10-1…K10-6 (round 10), D13-A…D13-C (round 13), D14-A…D14-D
  (round 14), D15-A…D15-C (round 15), D16-A…D16-D (round 16), D17-A…D17-D
  (round 17), D18-A…D18-C (round 18), D19-A…D19-C (round 19) and PX1 (the base `~+/bin` experiment).
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
resolved before the window started. A wrapper that execs "the next `git` on `PATH`" cannot reach the
pin again: every exec-through shim (the pin and the fixtures' shims, one builder) drops its own
directory from the front of `PATH` before it execs, in any idiom, `env -i PATH=…` included (§4, P-j;
measured, `…-pr527-r15.md` §Q, review R2). With `PATH='~+/bin:…'` and a `git` wrapper in the repository's
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
| **(b)** | **accidental fixture-authoring mistakes that persist an outside git input or leave the build incomplete**: a `git config` naming an outside file, an include appended to `.git/config`, an accidental `--separate-git-dir`, an arithmetic error that skips a line, `errexit`, `nounset` or `pipefail` left off at the end (a `set +e`), the window's stderr left redirected (an `exec 2>/dev/null`) | **this PR** | **caught, fail-safe on shape**: P-g over every git dir's configuration and shape (§4); W, with the options and the window's stderr re-checked after the file; W3 (a shell diagnostic). An object store outside the repository (`objects/info/alternates`): P-k (§4) |
| **(c)** | **fixture code that deliberately evades**, and **any fixtures-file write to verifier state that construction leaves reachable** (below) | **out of scope**: code review | not closed, not detected, not owed (§5.1) |

**What separates (b) from (c) is a property, not intent.** A class-(b) mistake leaves **persisted,
observable state** at the end of the build **and leaves the verifier's own state (below) as written**: a
configuration line, a git dir of an unexpected shape, a missing marker, an option left off, a
redirected stderr, a shell diagnostic, each read by the postconditions and the parent as this memo describes them. Class (c) is
everything else: what leaves no such state, and what writes the verifier's own state.

**The verifier's own state, by property.** It is what the verification is made of, not what it reads
as the build's output:
- every name the prelude defines: its data assignments, the labels, and its functions
  (`_fgit_postconditions`, `_fw_opts_on`, `_seal_apply`, …);
- every file the parent or the postconditions write and read back as their own record: the window's
  return channel in `$_FW_DIR` (`built`, `done`, `cause`, `post_ok`, `post_bad`, `machine_limits`, …; `built`
  is written by the fixtures file's own last line by design, §3), the postconditions' directory (the
  references and working files) and the parent's directory (its working files), both made after the
  build (§4, "The references and the working files");
- the pin `$_FGIT_BIN` and the wrapper in it.

The shell options and the window's fd 2 are **not** in it. `errexit`, `nounset`, `pipefail` and where
fd 2 points are the build's own observable state, and W's re-check reads them after the fixtures file
as it reads any output of the build (§3). So an option left off, or stderr left redirected, at the end
is class (b), red by W; switched off and back on in the middle, it leaves nothing to read, class (c) by
the first clause. With that, no mistake falls in both classes.

**The design rule (D16-B): the verifier's files are out of reach by construction.** Every file the
verifier writes and reads back after the build lives in a directory the verifier makes after the build
with `mktemp -d`, never under `$CTL` (the fixtures' tree) or `$_FW_DIR`: the postconditions' directory
(`_pq`, §4) and the parent's (§4, "The parent's directory"). Class (c) is then **declared** only for
what construction cannot close, each with its measured cell. The last item below qualifies the rule
itself: a changed resolution of `mktemp` can hand either directory back to the fixtures (`mkfn`,
`staleshim`), so "out of reach" holds except for that declared class (c):
- the window's return channel, which both sides must name (`xdone`);
- the names the prelude defines, in the shell the fixtures file shares (`xredef`);
- the pin `$_FGIT_BIN`;
- a fixture process that outlives the build, which can still find a directory made after it (`xrace`);
- **which executable or code the verifier — the postconditions in the window and the parent after
  it — runs** (D16-C, stated by property in draft 19, D17-A, extended to the parent in draft 20, D18-A,
  and from name resolution to the code itself in draft 21, D19-B). Anything the fixtures file does that
  changes it writes the verifier's own state. Examples, not a list: a function (`mkfn`), a `PATH` entry
  after `$_FGIT_BIN` (`shimafter`), `hash -p` (`shimhash`), `enable -n`, an alias under
  `expand_aliases` for a name on the window's own lines after the fixtures file (an alias does not
  reach a function body the prelude parsed before it), an executable written into a directory on the
  caller's `PATH` (`staleshim`), and the verifier's own source in the checkout, `.claude/tools/**`,
  which the window can write (`srcrw`: the fixtures append `_mut_correspondence() { return 0; }` to the
  checkout's `mutations.sh`, which the parent sources after the window). The last two are the
  filesystem route: no `PATH` assignment, a file that persists after the window, so they reach the
  **parent's** code as well as the window's. It needs only a writable file the verifier runs or
  sources: the checkout always is one, and on this machine so is `/opt/homebrew/bin`, owned by the
  caller (`ls -ld /opt/homebrew/bin`) and before `/usr/bin` in the caller's `PATH`, which also covers
  overwriting a resolved file in place. `staleshim` redirected the `mktemp` that makes the parent's
  `_VFY` to a directory the fixtures prepared; the parent's other commands (`sed`, `cmp`, `grep`, `wc`,
  the controls' children) are reachable the same way, which is inferred, not measured. Construction
  cannot close this route: resolving the parent's commands to absolute paths before the window only
  narrows it, because the fixture can overwrite the resolved file too, and the verifier's source has to
  live somewhere the checkout can write. A fixture
  that writes outside the fixture root into the caller's environment is itself outside anything class
  (b) produces. Which executable runs is also R1's (§0.1; `#11-trip-wire-launch-environment`): this is
  the same channel, written by the fixtures instead of the caller, and declared here as class (c).
  A function shadows a name whatever it names: an external command, a builtin (`unset`, `builtin` and `command` included) and a name spelled as an absolute path
  (`/usr/bin/mktemp() { …; }`), each measured on bash 5.3 and 3.2 (`…-pr527-r15.md` §Q). One form is
  red, not class (c): a `PATH` entry put **before** `$_FGIT_BIN` fails P-j's first-entry clause
  (`shimprepend`; C6 keeps that clause). The set is every executable and every piece of code the verifier runs
  after the fixtures file, in the window or in the parent, not a list.

  **No in-shell reset is adopted.** Every in-shell construction is defeated by one fixtures-file line.
  `POSIXLY_CORRECT=1` with `$(exec /usr/bin/mktemp -d …)` runs the real binary despite `mktemp`, `exec`
  and `unset` functions, on both shells; one fixtures-file line, `enable -n exec; exec() { …; }`, makes
  the same substitution return the function's output, rc 0 (`…-pr527-r15.md` §Q). So a deliberate
  defeat is class (c). What a reset could still buy is harmlessness for an accidental shadow, and the
  measured direction says it is not needed there (D18-C): the most plausible accidental shadow, a
  fixtures helper `git() { command git -c user.name=k2 -c user.email=… "$@"; }`, is red on both shells
  by P-a (`command line:` origins) and stops the run before any control (`gitfn`, `…-pr527-r15.md` §Q).
  An accidental shadow of `git`, the window's one pinned command, changes what the postconditions ask
  git and so shows; the silent shapes above are deliberate constructions that name a prepared
  directory.

A fixtures-file write to verifier state that construction leaves reachable is class (c) whatever else
the build leaves: the state is read by a verifier that the same command changed, and a file does not say
who wrote it. A postcondition that resets shell state before relying on it (P-c's `set +f` and
`GLOBIGNORE`, §4) makes an accidental leftover harmless; a deliberate `readonly` that defeats the reset
writes the verifier's state, class (c).

The completion markers are **a protocol, not a seal**: they make an accidental `return`, `exit` or
abort, an option switched off and a redirected stderr loud (W); they cannot authenticate their writer. The measured boundary (`…-pr527.md`, round 14 and D14-D): a key
persisted in `clean` alone is red by P-g (`xkey`); the key plus `exit 0` is W (`exit0`, bash 5.3); the key plus
`built` and `done` written by the fixtures file (`xdone`), or plus a `_fgit_postconditions` redefined
to return 0 (`xredef`), is rc 0, PASSED, and so is the key in every fixture plus a background process
that the fixtures file leaves running and that writes the reference made after the build (`xrace`,
round 15, D15-C), and so is a `mktemp` made to name a directory the fixtures prepared, by a
fixtures-file function (`mkfn`, round 16, D16-C), by a `PATH` entry after the pin (`shimafter`) or by
`hash -p` (`shimhash`, both round 17, D17-A), and so is a diagnostic hidden by a stderr redirect scoped
to a group around the failing line (`xgroup`, round 17), and so is a stale mutation record hidden from
the parent's anchor check by an executable the fixtures wrote into a caller-`PATH` directory
(`staleshim`, round 18, D18-A; its control `stalectl`, the stale record alone, is rc 1, "its anchor is
stale"), and so is the same stale record hidden by a `_mut_correspondence` the fixtures appended
to the checkout's `mutations.sh` (`srcrw`, round 19, D19-B). The last nine are this boundary, not
defects. `xkey`, `exit0`
and `xredef` ran on `ebc90bc1`'s tool code, which is `8413a4db`'s except three comment-only path lines
(Fable; `…-pr527.md` round 14); `xdone` on that tree and on the draft-16 prototype (`…-pr527.md` round
14); `xrace` on the draft-16 prototype (round 15's Ax2 cell, as relayed) and on the draft-17 and
draft-18 prototypes, `mkfn` on the draft-17 (round 16's Ax3 cell, as relayed) and draft-18
prototypes, and `shimafter`, `shimhash` and `xgroup` on the draft-18 prototype (round 17's cells, as relayed:
`shimafter` and `shimhash` Ax3's, `xgroup` Ax2's) and the draft-19 prototype, `staleshim` on the
draft-19 prototype (round 18's Ax2+Ax3 cell, as relayed) and the draft-20 prototype, and `srcrw` on the
draft-20 prototype (round 19's cell, as relayed) and the draft-21 prototype (`…-pr527-r15.md` §Q).

Examples of (c), which illustrate the property and are **not a list to complete**:
- a transient per-command input: `-c`, `--config-env`, an environment assignment on one command,
  `--template`, an injecting `PATH` shim;
- persist-then-revert (write a config entry, run git, remove the entry);
- `--git-dir` / `--work-tree` pointing outside the fixture root;
- sourcing an outside file (`. <file>`);
- switching an option off and back on in the middle of the file;
- a stderr redirect scoped to a group or function around a failing line (`{ : $((1/0)); } 2>/dev/null`,
  `xgroup`): fd 2 is the captured file again at the end, so C12's canary arrives and W3 has nothing to
  read;
- writing the verifier's own state that construction leaves reachable: a marker, a prelude variable or
  function, the pin, a file in a directory made after the build written by a background process that
  outlives the build (`xrace`), or which executable or code the verifier runs — the window's or the
  parent's after it (`mkfn`, `shimafter`, `shimhash`, `staleshim`, `srcrw`; above).

A mistake that happens to take a class-(c) shape is not caught either, and this memo does not claim
otherwise.

**Why (c) is out of scope.** P is a function of the fixture script (§1). A script that names an outside
input has made it part of itself, and no in-process check can tell a deliberate transient input from a
legitimate one without re-parsing shell. Draft 7's seed S tried that and failed both ways (§5.1). The
fixtures file already has an owner for deliberate content: review of repository code. The fixtures
file and the harness are two files of one repository at one trust level, changed and reviewed in the
same PRs, so a boundary between them would separate no trust levels. Running the postconditions in a
second process was considered when `xdone` was dispositioned (D14-D) and rejected: `built` stays
forgeable by construction (the fixtures file *is* the build), `$_FGIT_BIN/git` stays writable, and
P-a…P-j assert about the window's environment, so they would then describe a process other than the
one that built (§4).

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
# harness at `96eea58c`, abridged; the harness is the source
_FGIT_VOID="$SCRATCH/fgit-void"             # mkdir, checked
_FGIT_BIN="$SCRATCH/fgit-bin"                # holds `git` (`_fgit_shim`): drops its own dir from PATH, exec <this shell's `type -P git`> (§0.1)
_FGIT_PATH="$_FGIT_BIN:$PATH"               # the caller's PATH verbatim, behind the pin (§0.1)
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
  printf "K2-WINDOW-STDERR-CANARY\n" >&2         # C12: the parent requires it in the capture
  _fgit_postconditions && : > "$_FW_DIR/post_ok" || : > "$_FW_DIR/post_bad"   # trust needs post_ok
  _seal_apply                                   # mode restrictions, after the census
  printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
  : > "$_FW_DIR/done"' _ "$_FW_DIR" 2> "$_FW_DIR/stderr" || _fw_rc=$?
# then, still in _fgit_window: canary check, replay, W3 scan, and _fw_trusted (below)
```

**What the child gets, kept minimal.** The prelude contains:
- `set -euo pipefail`;
- plain `name=%q` assignments. These cover the data the fixtures file reads (`CTL`, the five
  `CONTROL_*` samples, `_REAL_GIT`, `_REAL_GREP`, `_fifo_ok`) and the window's own `_FGIT_VOID`,
  `_FGIT_ENVBIN`, `_FGIT_ENV_REQ`, `_FGIT_WIRE_EXEC`, `_FGIT_WIRE_LC`, `_FGIT_BIN` (P-j) and `_FW_DIR`, and the
  postcondition labels, passed by name rather than as positional arguments: the names are the ones
  `_fgit_postconditions`' body uses, each of which the controls file (where the ratchet counts them) must
  define non-empty, or the window refuses. The fixtures' part of the list is **derived** by the census
  command below;
- `declare -f` bodies of `_fixture_failed`, `_shq`, `_fgit_shim`, `_fgit_canon`, `_fw_opts_on`, `_seal_refuse`,
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
`$SHELLOPTS` after the prelude, and it stops with W's "refused to start" cause otherwise. Its record drops `nounset`
from the prelude and deletes the re-check after the fixtures file, which would red first (corpus §6.1, review R2).

Assignments are **plain**, never `declare -p`. `declare -p` carries the `export` attribute into the
window; P-f caught that in p6's first run (companion §A.9).

**What comes back.** Three things return through files: `_FIX_FAILED`, the fact of completion
(`done`), and the postconditions' verdict: `post_ok` when they returned 0, `post_bad` otherwise. The window's own `CONTROL …` lines are captured
with the rest of its stderr (the file `stderr`) and replayed to the wire's stderr by the parent after
the window ends, the canary taken out (C12). The markers are a protocol, not a seal: a fixtures-file write to them is class (c)
(§0.3). Absent and unreadable are different answers: a marker that exists but cannot be read (`seal_failed`, `machine_limits`, `fix_failed`, `cause`; mode 000, a directory, a dangling link) is collected in `_fw_unread` and makes the window untrusted (below), never "nothing" (Stage 5 E-3; no record, its trigger is class (c); measured by hand, `…-pr527-r15.md` §Q).

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

The parent adds a fourth (D16-D, C12): **the fixtures file left the window's stderr redirected, so a
shell diagnostic could not be read.** After the options re-check the child writes a fixed canary line
to fd 2, and the parent finds it in the captured `stderr` (anywhere in a record, as W3 matches) before
it counts the window complete; a missing canary makes the window incomplete, W. The canary is taken out
of the replay. Without this, `exec 2>/dev/null` followed by `: $((1/0))` in the fixtures file sent the
diagnostic W3 reads to `/dev/null`, rc 0, PASSED on both shells (round 16's `xexecarith`); with it, rc 2,
W with this sentence (`…-pr527-r15.md` §Q). The check is in the parent, so no fixtures-file function
can shadow it; a canary the fixtures file writes itself is a write to the return channel, class (c).

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
run. The job's budget is read from **a run of the job on GitHub's runner**, never from local runs
scaled by a factor (D15-B, #510's convention at `106387e4`). #527 is stacked and gets no runner run.
X9 is route (b), decided by the user when #527 was opened: #501's CI after #527 is squashed into it. So
#527's job comment names the runner method, with no local method beside it and no figure, and the
citation-hygiene lane writes the verdict with the runner figure on #501's branch after that run, in
`…-pr527-r15.md` §Q and never in `ci.yml` (`…-landing.md` §9, "Where the runner figure comes from"; land order step 7). X8's local runs at T are a
sanity record only. T is the last commit that changes the tool code or the job (`…-landing.md` §9.1).

**A red postcondition ends the run before any control.** A red P-a…P-k, or W3, means the build is not
the one the fixtures file describes. A control over it asserts nothing, and it can block. In Fable's
M-SHAPE prototype log (`fable/e1/e4e.log` in the orchestrating session's scratchpad; one run, not
re-measured), a FIFO `.git/commondir` made P-g red, and then the first `_control` was killed at 30 s.
The Codex R26③ cells of draft 13, with a stand-in stop, ended in 5–8 s with no control run
(`…-pr527.md`).

So the window's verdict is reported in **one place**, one harness function,
`_fgit_window_verdict_exit` (C8 renames `_fgit_window_incomplete_exit`). The controls file calls it
after `ctl_ok=0`, and W3's report moves into it from the controls file. W4's report stays in the
controls file, because W4 does not stop the run. **Trust is a property of the window**: the one
writer of **`_fw_trusted`** is `_fgit_window` itself, at its end (1 only for a complete build whose
postconditions wrote `post_ok`, positive evidence: no `post_bad` is not one; and no W3, and no unreadable marker, `_fw_unread` empty), and the gate
`_fw_built_or_w2` decides by `_fw_trusted` alone; the verdict function only reports (C8's version computed the flag, so a verdict
call below the first control left a clean build untrusted: ROg red; fixed, `…-pr527-r15.md` §Q). The
function continues only over a build that is **complete and trusted**:
- incomplete: W alone, exit 2 (above);
- complete, but a postcondition or W3 reported, a marker unreadable, or the postconditions reached no verdict (neither
  marker): those reports (the last two each its own line under W's label, `$_fw_lbl`; W3 under `$_fwd_lbl`), then exit 1. A red is a decided verdict;
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
  that W2 is reported when the exit is gone**. One removes both of `_fgit_window_verdict_exit`'s exits
  with the window kept incomplete (`done` renamed). The other two each remove its untrusted-build
  `exit`, one with a postcondition forced red (`post_bad`), the other with W3 forced (`_fw_diag` set).
  Each survives with its clause (`done`, `post_bad`, W3) removed from `_fgit_window`'s trust computation
  (measured, `…-pr527-r15.md` §Q); `post_ok`'s two records (review R2, corpus §6.1) pin the fourth.
  A gate satisfies all three. ⚠ **Declared gap:** removing any one of the four gates, `_control`'s own included,
  survives the mutation set, because another gate reports W2 first (measured for `_control`'s:
  `…-pr527-r15.md` §Q, C8); the block gates were pinned by the traced `w2rec` cell (companion §A.14),
  re-anchored for T as X13 (`…-landing.md` §11), not by a record.
- Moving the exit below a control is an edit to the controls file, which is not a mutation target. That
  shape is pinned by the corpus RO/ROg cells (X5, re-anchored for T in `…-landing.md` §11), not by a
  record. ROg is green again since the fix round (measured on both shells, `…-pr527-r15.md` §Q).
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
diagnostic, and C12's canary makes sure that diagnostic reached the captured stderr (above).

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
corpus §6 (`HOME` and `GIT_ATTR_NOSYSTEM` by records of their own since the fix round, §6.1).

| entry | without it |
|---|---|
| `PATH=$_FGIT_PATH` | BSD `env -i` runs `/usr/bin/git` rather than `PATH`'s git (INFO cell). The first entry is `$_FGIT_BIN`, a wrapper that execs the `git` **this shell's own lookup** resolves. The lookup is `_fgit_resolve`, the one resolver for every command that crosses the window boundary by path, `$_REAL_GIT` and `$_REAL_GREP` included. After it comes the caller's `PATH`, **verbatim**. No entry is reinterpreted, dropped or emulated, because which executable runs is R1's (§0.1), and what P needs is only that the window's `git` is the pinned one. An entry means whatever it means from inside the window (relative, empty, `~`: §0.1). Since `git` is pinned, that decides only which other tools are found, and a missing one fails the window: red. The fixtures' other commands come from these entries (§1, "Outside P"). P-j pins the construction. History: `…-pr527.md` |
| `HOME=$VOID` | an unset `HOME` also closes this on 2.55. The void is chosen because a future HOME-relative default then lands where P-c looks. Pinned by P-b (`GIT_CONFIG_GLOBAL` outside the void when `HOME` is not the void) and P-c (`$HOME/.config/git/ignore` written) |
| `GIT_CONFIG_NOSYSTEM=1` | the system layer is live here (`/opt/homebrew/etc/gitconfig`) |
| `GIT_ATTR_NOSYSTEM=1` | undocumented at 2.55: 0 hits in all 207 man pages of 2.55.0 (command below; positive control: `CONFIG_NOSYSTEM` hits 2 pages). So it is pinned by `git var` (P-b): dropping it alone reds `GIT_ATTR_SYSTEM=[…]` |
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

**errexit and state.** Every window state name (`_fw_rc`, `_fw_done`, `_fw_post_ok`, `_fw_post_bad`, `_fw_why`, and
`_fw_trusted`, 0 until `_fgit_window` sets it at its end; `_fw_unread`) is
assigned at harness top level, before anything reads it (`_fw_diag` and the W2 flag included). The
child runs under `set -euo pipefail` with **no EXIT trap**. Each of those options is checked separately
before the fixtures file (one record per option, corpus §6, which the re-check also kills; the check before
alone by review R2's record, §3 above), and all three are checked again after it (one record). So an abort there
is a non-zero exit, which the parent reads as data. The verdict is written after `ctl_ok=0`
(`controls.sh:738` at base), and a window that is not complete and trusted ends the run right there.

**The parent's reliance on `set -u`, measured (IMP-2).** With the wire's `set -euo pipefail` changed to `set -eo pipefail` (nounset OFF in the parent), the
clean tree and eight cells (sealfail, env0, garbagehead, w2rec, lblrename, w3ar, sealdotdot,
reftable; six red, and env0 and reftable green at `8413a4db`) gave the same exit status, the same NE/CF counts and the same first four `!!` lines as with it on:
- on bash 5.3 at the `/elidex-review` head (companion §A.14);
- **on bash 3.2 and 5.3 at `8413a4db`**, 36 runs. Bash 3.2 is where it mattered: there the wire's EXIT trap turned an unbound-variable abort into rc 0
  (`#11-k2-wire-exit-trap-masks-set-u-abort`, closed by `8f4ad3c8`: the **orderly-end flag**, below and `…-residuals.md` §5.2).

The cell script is verbatim in `…-pr527.md` ("X13"), and X13 re-runs it at T (`…-landing.md` §9.1). That
is the measured claim: on those runs, no parent-side verdict depended on `set -u`. It is
not a proof over every path, and the "every state name is assigned before it is read" argument is a
reading of the code, not a measurement. The child's `set -u` is a backstop
behind the static census, and it is pinned by its own record. **The orderly-end flag (`8f4ad3c8`).** A run may leave with status 0 only through its verdict: `_K2_ORDERLY=1` is set once, just before the wire's PASSED line, and the wire's EXIT trap body `_k2_exit` (one function, also called by the mutation run's trap, which replaces the wire's) turns a status of 0 without the flag into `CONTROL FAILED (the run ends only through a verdict)`, rc 1. Capturing `$?` first does not help: on 3.2 the trap is entered with `$?` = 0 (measured). The flag's assignment has a record; removing the check has none (bash 5 exits 1 by itself, so it could die only on 3.2); the evidence is the R2-3 cell, 3.2 rc 0 before and rc 1 after (`…-pr527-r15.md` §Q).

## §4 Postconditions — run INSIDE the window

Every git in the window whose inputs **no fixtures-file command altered** inherits the same
environment and the same persisted state. So git's own answers inside the window describe every such
git (§5.1 covers the rest). Each producer has its own label (defined in the controls file and passed
in), and each label has its own record (corpus §6).

| id | label | assertion (inside the window) | liveness (its own label) |
|---|---|---|---|
| W | `the fixture build window completed` | the fixtures file's last line wrote `built`, the options were still on after it, the child's canary reached the captured stderr (C12), and the child wrote `done`. Otherwise NE with the sentence the child wrote to `cause`, or the exit status (§3), **reported alone**; exit 2; no control runs | — |
| W2 | `no control runs over an incomplete or untrusted fixture build window` | `_control`'s first statement, and the first thing each non-`_control` block asks: the window is complete and trusted (`post_ok` written, no postcondition or W3 reported). Its records: the exits removed, with the window incomplete or untrusted (§3). The exit placed below a control: the RO cells. Trust is decided by `_fgit_window`, never by the verdict function (§3) | — |
| W3 | `the fixture build window ran without a shell diagnostic` (renamed in the fix round: `prelude.sh` is not the fixtures file) | no record of the child's stderr contains `./fixtures.sh:` or `./prelude.sh:` **anywhere in it** (both are sourced by those relative names from the window's directory; bash appends a diagnostic to a record the fixtures left without a newline, §3). A failed scan is red. A record pins each prefix, `./prelude.sh:` by an expansion error in `_fgit_postconditions` | — |
| W4 | `every mode restriction a fixture sealed was applied` | every `_seal` was accepted (a path under `$CTL`, with no newline or TAB; an octal mode; a fixture name of `[A-Za-z0-9_-]`) and its `chmod` succeeded; otherwise red, not a machine limitation. `_seal_apply`'s refusal of a path through a symlink is asserted at load by a standing self-check (`_seal_apply_probe`) under this label | — |
| P-a | `a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file` | every `git config --list --show-origin` line in **one probe repo** has the origin `file:.git/config` | `this git reports a configuration origin outside the repo's file`: `-c a.b=c` must show with the origin `command line:`. `--show-origin` (git 2.8) alone, not `--show-scope` (git 2.26): the origin column answers the same question, so the check has no version floor and no limitation arm |
| P-b | `the fixture git has no system or global layer outside the void` | asked **inside P-a's probe repo** (`cd` there, `git --git-dir=.git`: no discovery), so no caller repository's local configuration is read. A git older than 2.42, whose `git var` cannot name these four (exit 129), is a machine limitation: `⚠ NOT EXERCISED on this machine`, green. Otherwise `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is an empty directory, tested by the shell's own globs (`*`, `.[!.]*`, `..?*` with `-e`/`-L`) — not by `$(ls -A …)`, which loses a name made only of newlines and reads a failed `ls` as empty. A void that is not a directory, or cannot be read and searched, is red: globs over it would expand to nothing. The globs run with `set +f` and `GLOBIGNORE` unset, whatever the fixtures file accidentally left (a deliberate `readonly GLOBIGNORE` is threat-model class (c)). Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `no exec-path override reaches the fixture git` | the window's `git --exec-path` equals the same git's answer with only `PATH` and `HOME=$_FGIT_VOID` in its environment (taken in the parent at source time), both canonical (`pwd -P`). `HOME` is there because a caller's `git` wrapper that reads `$HOME` under `set -u` aborts without it, which gave an empty reference and a red with no cause (fix round #3); no record pins it, since dropping it reds only on a machine with such a wrapper. **Re-scoped by `/code-review`:** which executable runs is outside P (§0.1, R1); P-e pins only that nothing in the window overrides where that git runs its commands from. So a caller's `GIT_EXEC_PATH` or `DEVELOPER_DIR`, or one directory spelled two ways, is not a red | — |
| P-f | `the fixture build window's environment holds only its allowlist` | the name of **every** `env -0` record (the text before the first `=`, so non-identifier names such as `BASH_FUNC_f%%` are included) is an allowlist name (derived from `_FGIT_ENV` itself, so the two cannot drift) or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction, and so is an allowlist name missing from the records (`missing …`: a name exported away inside the window, which P-h, P-i and P-j read as shell variables and still see; pinned by review R2's `export -n LC_ALL` record). A listing that is not NUL-separated reads as no record (the NUL-list policy, "P-g's population" below). p6's `sed` parser skipped non-identifier names; p7 parses every record (companion §A.10). `env -0` goes through a file whose status is checked: a failed `env -0`, or no record at all, is NOT EXERCISED. **One** outcome is a machine limitation instead (`⚠ NOT EXERCISED on this machine`, green — the treatment P-b and the FIFO and file-permission controls give a machine that cannot run them): its test is the definition — **this** `env` runs but refuses `-0`: `env -0` fails, writes nothing to stdout and says why on stderr, while the same `env` runs a command. Every other outcome stays red, so an unknown one falls on the fail-safe side. Which `env` builds lack `-0` is not measured here (this machine's macOS 26 `env` has it; PR #527 Codex R4 reported a macOS one without it); the no-`-0` case is exercised with a shim. Both limitations reach the run's summary through one channel (`$_FW_DIR/machine_limits`), naming the postcondition by ID only, since a record's needle is its label. History: `…-pr527.md`. | — |
| P-g | `every fixture repo persists only the configuration a plain git init writes` | for **every git dir under the fixture root** (population below), the listing of `git --git-dir=.git config --list --show-origin` run in that repo (no discovery, so a `.git` git does not recognise is an error, never an ancestor's listing; no `--show-scope`: the origin already names the file) **equal, as a set,** the lines of a **reference** `git init` made in the same window (P-a's probe repo, which is that init, made after the build in the postconditions' directory, which no fixture command that finished by the end of the build can name: "The references and the working files", below), compared by origin, key and value. An extra entry is an input the fixture persisted; a missing one (`git config --unset core.filemode`) hands that setting to the platform default — either way P would depend on more than the fixture. The comparison is of **records**, and it runs first: the `-z` listing, each origin paired with its entry into one record written as one line by `printf %q`, sorted (no `sort -z`), byte-identical to the reference's — so a value holding a newline cannot forge a line, and a repeated entry is red. Only when the records differ are the lines listed (`grep -vxF -f`, both directions), to name the extra and missing entries; that runs on a red path only, so its failure cannot turn a red green. This catches a persisted include (its origin is not `.git/config`), a `commondir` naming another git dir (the origin names that dir's config), and **any** persisted key beyond init's, `core.excludesFile` included. **Census first (M-SHAPE, drafts 14–15):** the census classifies and shape-checks every git dir it names, and runs its per-link search, and its one verdict comes before any postcondition runs git (population below). An entry under a `.git` that is not a directory or a regular file with one link is red: a symlink of any target, a FIFO, a socket, a device, a hard link. Git reads through a link (a `.git/config` pointing outside still reports `file:.git/config`) and blocks opening a FIFO, whatever the entry's name. **A red census runs no git anywhere**, because git reads across git dirs (`commondir`, `alternates`, `include.path`). A scan that fails or writes to stderr is red. The census takes `HEAD` and `.git` entries of **any** type and letter case — a symlink `HEAD`, or `head` on a case-insensitive filesystem, is a shape git reads — and every git dir not reached by a `.git` entry is red. **Both directions:** a git dir whose listing is EMPTY or fails (a `.git` git does not recognise, e.g. a garbage `HEAD`) is red | — |
| P-i | `the fixture repos use the files ref format` | the window's `GIT_DEFAULT_REF_FORMAT` is `files`, and a plain init's `git rev-parse --show-ref-format` answers `files` (a git before 2.45 has no reftable; its `rev-parse` echoes the unknown option back with exit 0, and that literal echo is what counts as `files` — a failed call or any other format name stays red). It pins the allowlist entry on every git, including the files-default ones where dropping it changes nothing else | — |
| P-j | `the fixture build window runs the pinned git first on its PATH` | the window's `PATH` starts with `$_FGIT_BIN`, `type -P git` inside the window is `$_FGIT_BIN/git`, and a `git` run through the pin is handed a `PATH` without `$_FGIT_BIN` (an alias, `-c 'alias.k2p=!printf %s "$PATH"'`, prints it). That last clause pins the shim builder `_fgit_shim`, which writes the pin and every fixtures exec-through shim so that each drops its own directory from the front of `PATH` before it execs: a "next `git` on `PATH`" wrapper then cannot reach it again in any idiom (§0.1). Review R2 replaced the fix round's depth counter (`K2_FGIT_PIN_DEPTH`, exit 125, its own label), which an `env -i PATH=…` re-exec reset, so the cycle hung. It pins the `PATH` construction the way P-h and P-i pin their entries: without the pin, the window's first `git` is the file the pin execs anyway, so nothing else changes, and P-e's reference moves with it. It asserts nothing about the caller's entries (§0.1) | — |
| P-k | `no fixture repo reads objects from a store outside it` | for every `.git` P-g compares, `git count-objects -v` (run as P-g's listing is) exits 0 and prints no `alternate:` line; a `grep` for that line that fails (exit above 1) is red. An `objects/info/alternates` naming another store makes git read an object from there instead of writing it locally, so the content the controls read is an outside input even though P's ids do not move (measured, `…-pr527.md`). Git's own answer, not a file name: it reports the stores git will read (measured for the `alternates` file). A failed `count-objects` is red, and **so is any stderr**: a store that does not exist, or an `alternates` naming a directory or a mode-000 path, gives rc 0, no `alternate:` line and a warning or error on stderr (round 12; `…-pr527.md`), and such a store could appear later and be read. It runs only over a clean census (§4, population) | `this git reports alternate object stores`: a third probe repo in the postconditions' directory (so P-d's two inits are untouched), given an `alternates` file naming P-a's object store, must exit 0 with nothing on stderr and print exactly one `alternate:` line. The file holds the path in double-quoted C form, so a newline in the scratch path stays one line. `gitrepository-layout` says only "one pathname per line". git(1) documents C-style quoting for `GIT_ALTERNATE_OBJECT_DIRECTORIES` (`man git`, that variable), and that the `alternates` file accepts the same form is measured: on 2.55.0 a path holding a newline gives one `alternate:` line, rc 0 (`…-pr527.md`, round 12's P-k commands). **Anything else is red**, NOT EXERCISED: there is no machine-limitation arm, because "prints none" cannot tell a git without the line from a broken probe (git 2.55.0 and Apple 2.54.0 both print it, measured), so the unknown falls on the fail-safe side |
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
so no git runs. It is **one** census: P-g and P-k iterate pass 1's own list and act on its `.git`
entries (the fix round deleted a second population scan). It covers the git dirs under `$CTL` it names. It does not cover what git
reaches through a reference ("The boundary", below), nor the reference repos, which are outside fixture
reach by construction ("The references and the working files", below) except through §0.3's declared
class (c): they are made in a `mktemp -d` directory too, so a changed `mktemp` (`mkfn`, `shimafter`,
`staleshim`) can hand them to the fixtures:
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
   - "Cannot search" is whatever `find` reports: any report on stderr (one check, after both passes),
     or a non-zero exit, fails the census, and so does a list that does not end in a NUL byte. A BSD `find` reports nothing for a directory it cannot enter (rc 0, no stderr, the repo beneath unlisted; measured, review R2), so the same `find` also lists every directory that is not `u+rx` (`! -perm -u=rx`), red as `[a directory the census cannot search]`; a GNU `find` reports one itself (read from its manual, not measured). That
     is one policy for every `read -d ''` reader (the census's two passes, P-f's `env -0`, `_pg_z`):
     after the loop, the remainder `read` left in its variable must be empty, else the list is
     red under the reader's own label. No command is run to ask. A list with no `.git` entry is red too, by P-g's count over it (`no fixture git dir was found`).
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
  `file:.git/../../zzi`). A read that fails is red by one of two routes. An include git cannot open
  (a mode-000 file; `/dev/tty` with no controlling terminal) fails the listing, git exit 128, red. An
  include naming a file that does not exist is skipped silently (rc 0, nothing on stderr), but the
  listing keeps the `include.path` line itself, an entry a plain init does not write, so P-g reds it
  as an extra key (X6's include; `…-pr527-r15.md` §Q). An `alternates` naming an
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

**The references and the working files, in verifier directories made after the build (drafts 16–18).** At
`8413a4db` P-a's probe repo `a` and P-d's second init `b` are made at the fixed path `$_FW_DIR/pq`,
which the fixtures can write, and `git init` over an existing repo keeps its configuration. Round 14's
`xrefpoison` made both in advance with `core.excludesFile /nonexistent-k2` and wrote the same key into
every fixture repo: rc 0, PASSED, on both shells. Every reference a postcondition compares against, and
every file it writes and reads back, by location:
- **the postconditions' directory**, made at the start of the postconditions, **after the build**, by
  `mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX"`, beside `$_FW_DIR` and never under it or `$CTL`. No fixture
  command that finished by the end of the build can know its name, unless the fixtures file changed
  what the `mktemp` the postconditions run resolves to (`mkfn`, `shimafter`, `shimhash`: class (c),
  §0.3). It holds `a`, `b`, P-g's reference
  listings and P-k's probe repo (C10), and every working file below. `a` and `b` are made by `mkdir`
  without `-p` after a clean census, so one that already exists fails the reference, red (P-g). P-a,
  P-b, P-d, P-g and P-i read `a`. A fixture command still running after the build (a background
  process) can find and write the directory; that writes the verifier's own state, class (c) (§0.3,
  `xrace`).
- **P-e's `_FGIT_WIRE_EXEC`, P-h's `_FGIT_WIRE_LC`, P-f's `_FGIT_ENV_REQ`**: values the parent takes
  before the window starts and passes as prelude assignments, not files. The fixtures file shares the
  postconditions' shell, so it could assign one, or redefine a postcondition function; that is a write
  to the verifier's own state, class (c) (§0.3). No fixture names any of them (`/usr/bin/grep -c -e _FGIT_WIRE_EXEC -e
  _FGIT_WIRE_LC -e _FGIT_ENV_REQ -e _fgit_postconditions` over the fixtures file → 0).
- **P-j's `$_FGIT_BIN/git`** is compared by path. Its content decides which executable runs, R1's; a
  fixtures-file write to it is class (c) (§0.3).
- **P-c's void** is the subject, not a reference: P-c reds any entry in it, and both reference inits
  read it as their template, so a template planted there is red through P-c (X6).

**The working files, by property (D15-A).** Draft 16 left them at fixed names in `$_FW_DIR`, and a
fixture-placed file there *can* make a comparison equal: a pass-2 link list or per-link search result
replaced by a link to `/dev/null` reads as empty, so a link to a tree holding a git dir passed
(round 15's `lnblind2`, rc 0, PASSED on both shells; red with the working files moved). So the set is
defined by property, not listed by name: **every file a postcondition writes and then reads back** is
in the postconditions' directory. Derived from the prototype by that property (`…-pr527-r15.md` §Q
gives the command), it is the census's two lists with their stderr, pass 2's per-link result, P-f's
`env -0` output, P-g's population with its stderr and failure mark and in-loop per-link result
(both deleted since, by C9 and the fix round) and its current listings. `$_FW_DIR` keeps only what both sides name:
the two sourced files, the seal manifest `seal` (fixture output by design, "sealing" below) and the
return channel the parent reads (`built`, `done`, `cause`, `post_ok`, `post_bad`, `machine_limits`,
`seal_failed`, `fix_failed`, `stderr`). A fixtures-file write to the return channel is class (c) (§0.3,
§5.1; `xdone`, D14-D).

**The parent's directory (D16-B, C11).** `$CTL` is the fixtures' tree, and after C11 the parent keeps
none of its own working files there (what a control's run does inside a fixture repo is that control's
subject, not a working file). At `8413a4db` the parent wrote its own working files at fixed names directly under
`$CTL`, and a fixture-placed link there could turn a check green: `ln -s /dev/null "$CTL/.bare"` with the
label ratchet at 0 (`xbare`), and `ln -s /dev/null "$CTL/.anchor"` with a stale record (`xanchor`), each
rc 0, PASSED on both shells (round 16's cells, as relayed); with the files moved, rc 1 with the ratchet's own message
(`…-pr527-r15.md` §Q). C11 makes one directory right after `_fgit_window` returns,
`mktemp -d "$SCRATCH/verifyXXXXXX"`, outside `$CTL` and `$_FW_DIR`, and a failure to make it ends the run
"decided nothing" (exit 2), as a failed `$CTL` does. The directory is out of the fixtures' reach by
construction **except** through the declared class (c) of §0.3: an executable the fixtures wrote into a
caller-`PATH` directory can answer that `mktemp` with a directory they prepared (`staleshim`, D18-A). The mutation set's entry guard (`mutations.sh` line 90 at
`8413a4db`) and the two "WHAT IT CONSUMES" headers (`mutations.sh` line 29, `mutgen.sh` line 12) name
`$_VFY` instead of `$CTL`: after C11 neither file reads `$CTL` outside the records' text, and the
controls file sets `_VFY` before it sources the mutation set (the draft-19 prototype, `…-pr527-r15.md`
§Q). C11
also makes **an empty generated set red** (D17-D), and **always on** (D18-B): after the move no
fixture plant can empty the set, but any other cause would print `0 mutant(s)` beside `0 neither
killed nor argued equivalent`, which X3 reads as a pass, the silent direction. It takes **two
layers** (D19-A), because the count X3 reads is not the generator's output but what `_mut_gen_run`
made of it (written to `.genmutants`, read back, split on TAB):
- **the generator's output**, a static property of `_mut_regex_mutants` over the running wire's
  `$K2RE` and `$K2RE_PATH`: by `_mut_correspondence`'s own rule (static properties go where every run
  sees them) it is checked there, always on, by `_mut_gen_floor` in `mutgen.sh`, under its own label
  (`the boundary-mutant generator derives a non-empty set from the wire's regexes`), and a record pins
  it (corpus §6.1). That needs `mutgen` as a record target beside `harness` and `fixtures`;
- **`_mut_gen_run`'s path**, checked at run time in the opt-in run: `_mut_run`, right after it calls
  `_mut_gen_run`, requires `_mut_gen_n` to equal the floor's count (`_mut_gen_floor_n`, which
  `_mut_gen_floor` sets) and to be non-zero, and otherwise prints `!! the generated boundary set ran N
  mutant(s), but the generator derives M from the wire's regexes: this run did not test every rule.`
  and counts one more in `_mut_gen_bad`. It sits in the caller, so an early `return` inside
  `_mut_gen_run` does not skip it. No record can reach it (a trial runs with `WEBREF_WIRE_MUTANTS`
  unset), so X3's floor cell pins it (`…-landing.md` §11). Draft 20 had only the first layer, and
  round 19's cells `RC` (the generator's output sent to `/dev/null` inside `_mut_gen_run`) and `RA`
  (`_mut_gen_run` returning at once) passed with `0 mutant(s)`; with both layers both are red
  (`…-pr527-r15.md` §Q). What each moved file did, at `8413a4db`:
- a plant turned a check green: `.bare` and `.anchor` (the always-on ratchet and anchor check), the
  generator's `.genmutants`, `.genequiv` and `.genseen` (opt-in mutation mode; `ln -s /dev/null
  "$CTL/.genmutants"` makes the generator read no mutant, and the run reports `0 mutant(s)` and
  passes: measured at `8413a4db`, rc 0, PASSED, `…-pr527-r15.md` §Q), and `.fsmonitor_ran`
  (the fsmonitor control's mark: a pre-planted mark stands in for the hook's run under a plain git
  call, so where the hook does not run that control's NOT EXERCISED, a red, becomes a pass);
- a plant made a check fail, red: `.mutants`, `.control_out`, `.umask_out`;
- display only: `.fsm_out`;
- not reachable: `.fifoprobe`, made and removed at source time before the window runs. It moves to
  `$SCRATCH` so that the checker below lists no working file without an exception;
- `fsmhook`, the hook the fsmonitor control installs, moves with its mark.

The checker is static, over the four parent parts: every write, `mkfifo` or assignment of a path under
`$CTL`:

```sh
/usr/bin/grep -nE '(>|>>|[A-Za-z_]=) *"\$CTL/|mkfifo "\$CTL/' .claude/tools/webref-generic-core-trip-wire.{controls,harness,mutations,mutgen}.sh
```

At `8413a4db` it lists 16 lines; after C11 one, `controls.sh`'s relative-scratch control,
`PATH="$CTL/fakerelmktemp:$PATH"`, which names a fixture directory to read; since `6ed414b2` also
`_seal_apply_probe`'s `: > "$CTL/real/f"`, where `CTL` is the probe's own, rebound in its subshell. It sees only literal
`"$CTL/…"` paths, so a path built another way is outside it. No fixture needs these names: every `ln -s`
in the fixtures file makes a link inside a fixture directory, and none names a dot-file directly under
`$CTL` (`/usr/bin/grep -nE '"\$CTL/\.'` over the fixtures file → nothing). So class (b) never reached
them; the move removes the class-(c) form of a command that finished during the build, and leaves
`xrace`'s, a process that outlives it, declared.

**Nothing is unsearchable at census time, by construction (sealing after the census).** Three
fixtures need a mode restriction: `walk/sub` and `d5root` mode 000, `d2red/sub` mode 0444 (and the file
`err/control.py` mode 000). They ask for it through `_seal <path> <mode> <fixture>`, which only appends
to a manifest in the window's directory — the path relative to `$CTL`, and refused (red, W4, and the
fixture marked failed) if it lies outside `$CTL`, holds a newline or TAB, or has a `.` or `..`
component, or if its mode is not octal or its fixture name not `[A-Za-z0-9_-]+` (refused as `(seal)`); `_seal_apply` also refuses a path with a symlink anywhere on it (`chmod` follows symlinks),
so no seal can reach outside the scratch root, and it skips a fixture whose chain already failed, so
one failure is reported once; the window applies the manifest **after** the postconditions,
and a `chmod` that fails marks that fixture failed **and is red under W4**: the controls gated on the
mode having taken effect would otherwise be skipped as a machine limitation (and they gate on an
independent probe, `_perm_ok`, never on a fixture's state). So the census
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
  mutation mode would have paid about 115 times. X8 records the whole cost locally; the budget verdict
  is X9's (§3, "`ci.yml`").
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

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-residuals.md` §5 (touch-time split, text
unchanged at `f8ea983a`; extended since by D17-A and D18-A; §5.1 what the window does not close, class (c); §5.2 the other residuals R1–R9 and the
pre-existing defect, closed by `8f4ad3c8`), so "§5.1", "§5.2" and "§5.2 Rn" in this memo resolve there.

## §6 The corpus — evidence, and the source of the records

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` §6 ("the corpus file";
touch-time split, text unchanged). It holds the recast oracle, the prototypes p6–p11, the re-run
recipe, every focused cell table, and the mutation records with their totals and the ratchet;
its §6.1 holds the record changes of drafts 11–21 (C6…C12) and of the fix round.

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
unchanged): the commits, the cost and `ci.yml`, X9's routes, the land order, the ledger text, §9.1,
the commits made on top of `8413a4db`, and §9.2, the two ledger texts (moved from §5.2, draft 18's
split).

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | the window's `env -i` closes every variable, and four relocations close default files: class (a). That holds for every git in the window **whose inputs no fixtures-file command removed, overrode or added**, however it is spelled. A command that does is class (b) if it leaves persisted, observable state (P-g, W, W3) and class (c) otherwise, which is out of scope | §0.3, §3, §5.1; corpus G, R |
| 2 | A×D | the postconditions run in the window, so they describe every such git. P-g extends that to the persisted configuration and the shape of every git dir under the fixture root: an unknown git-dir shape is red, and so is any entry inside a git dir that is not a directory or a regular file with one link. The census's one verdict comes before any postcondition runs git, and the references it is compared against, with every file a postcondition writes and reads back, are in a directory made after the build, out of the fixtures' reach by §0.3's design rule (what that rule cannot close, `xrace` and a changed command resolution among it, is declared class (c)); after the window the parent's own working files are likewise in a directory it makes then, out of reach by the same rule and with the same declared exception (a changed resolution of the parent's `mktemp`, `staleshim`), and it keeps none of them under `$CTL` (C11; §4, "The parent's directory"). P-k adds that no fixture repo reads objects from a store outside it | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read. Completion needs the fixtures file's own last line, with the options still on and the window's stderr still the captured file (C12); the markers are a protocol, not a seal (a fixtures-file write to them is class (c), §0.3); an incomplete window ends the run with W alone, a complete but untrusted one ends it after its reports, and `_control` itself refuses unless the build is complete and trusted (W2). Each child option is checked and recorded. With the parent's nounset off, the clean tree and eight red cells give the same verdicts (measured, §3; not a proof over every path) | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | producers live in the harness and labels in the controls file. A record targets `fixtures` when what it plants is something a fixture writes (the W early return and options re-check, W3, and the P-c, P-g and P-k shapes); a record that changes a producer targets `harness`, and one that changes the mutant generator targets `mutgen` (D18-B) | §4, corpus §6 |
| 9 | D | one label per producer, one record per label (the generator floor's label included); the ratchet is exact (20 since review R2: fewer bare labels is red too, so a slack ratchet cannot hide the next one) | corpus §6 |
| 10 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it, so the window's `PATH` is the caller's, verbatim, behind the pin, and P-j checks only the pin (first, the `git`, and off the `PATH` it hands on) | §0.1, §5.2 |
| 11 | F | still one build, now in a child; the budget verdict is read from a run of the job on GitHub's runner (X9), never from scaled local runs, and half the budget or more means STOP | §3; `…-landing.md` §9 |

## §11 Exit criteria

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` §11 (touch-time split, text
unchanged), beside the commits they verify.

## §12 Plan-review

Plan-review closed for draft 10 after round 9 (`…-reviews.md` §D.0 records the ground). Rounds 10–14
reviewed drafts 11–15 (`…-pr527.md`), and rounds 15–18 reviewed drafts 16–19 (`…-pr527-r15.md` §Q).
Draft 19 stated D16-C by property (D17-A); made land order step 7 return to a new runner run whenever
T moves, with step 8 reached only when X9's run at the current T is SUCCESS and under the threshold,
or, over the threshold, the user has decided at 7.3 for that T (D17-B); added CLAUDE.md's trip-wires
paragraph and the job's "RE-DERIVED" comment to the final-head rewrite (D17-C); and corrected X3's
opt-in expectation (D17-D).

Draft 20 extended D17-A's property to the parent's commands after the window and declared the
filesystem route class (c) (D18-A, `staleshim`); moved the empty-generated-set check to always-on with
its own label, a `mutgen` record target and a record (D18-B); closed the C8 open item (corpus §6.1);
and replaced the no-reset argument with the measured `gitfn` direction (D18-C). Round 19 reviewed it
(`…-pr527-r15.md` §Q).

Draft 21 restores the run-time layer of the empty-set check beside the always-on floor (D19-A), states
the class-(c) property as which executable or code the verifier runs, with the verifier's own source
among the examples (D19-B), and fixes round 19's MINs (D19-C).

**No further prose plan-review round (the orchestrating session's decision, 2026-09-30).** Draft 21 is
verified by (a) the reviewer's failing cells re-run on the draft-21 prototype, `RC` and `RA`, which
must be red on both shells, and (b) one narrow check of the draft-21 delta. Then the implementation
(C6…C12) starts; its owner is the orchestrating session. The remaining risk is carried by the
executable gates: X3's mutation run at T, `/pre-push`, and the Codex converge. MINs left after the
narrow check are fixed in the branch during the implementation and recorded in `…-pr527-r15.md` §Q,
not re-reviewed as a plan.

## §13 Implementation results

The results before PR #527 are in `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §13
(touch-time split; history). PR #527's results through plan-review round 14 are in `…-pr527.md` §P,
which is closed. Every result from round 15 on, the planned commits included when they are made, goes to
one place: `…-pr527-r15.md` §Q (`…-landing.md` §9.1, step 2).

**Rollover (D19-C).** When an entry would take `…-pr527-r15.md` past 800 lines (`wc -l`), that entry
and every later one go to a new file, `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl.md`,
§R, with the same no-spec-surface §0, and §Q ends with a pointer to it. This memo's next split seam, when
an edit would take it past 950 lines: §4 from "P-g's population, and the census before any git" to the
end of §4 (the census, the boundary, the references and working files, the parent's directory and
sealing), moved unchanged to `…-census.md` with a pointer left in §4.
