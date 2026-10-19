# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits and false premises. **Review
record**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md`. Its §D holds plan-review
rounds 1–9 with their dispositions and terminators (split out of the companion unchanged); §S this
memo's status narrative (moved from its preface, and each draft's status is added there since); §8 the
code splits, the parent memo's banner and the memo references, and §13 the implementation results
before PR #527 (both moved from this memo); §CR the revision errors of the CR round's plan memo (moved from its §10). PR #527's record, its
external review and plan-review rounds 10–14, is in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527.md` ("`…-pr527.md`"), and from round 15 on,
with the implementation results, in `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527-r15.md`
("`…-pr527-r15.md`"; closed at 799 lines by the §13 rollover), and the implementation record from review round 3 on in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl.md` ("`…-impl.md`", §R; closed at 799 lines, §13), continued, after it closed, in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl-cr.md` ("`…-impl-cr.md`", §S; its §S.1 backfills the
code review's first-half results §R lacks; the open record while under 800 lines, §13). **CR round**: the plan
of PR #527's CR round (clusters B and C of the branch-wide `/code-review max`), approved by plan-review R1–R3 and a
focused re-check, in `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-plan.md` ("`…-cr-plan.md`"), with its
sweep ledger and the instruments the slices run (its §E) in
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-sweep.md` ("`…-cr-sweep.md`"). **Census**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-census.md` ("`…-census.md`"), the second half of §4 (P-g's
population, the census, the boundary and the working files), split out of this memo unchanged at `20615255`.
**Corpus**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` ("corpus §6"), the corpus cells and the
mutation records (split out of this memo's §6 unchanged). **Landing**:
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-landing.md` ("`…-landing.md`"), §9 (the commits,
the land order and the ledger text) and §11 (the exit criteria), split out of this memo unchanged.
**Residuals**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-residuals.md` ("`…-residuals.md`"),
§5 (class (c), the residuals R1–R13 and the pre-existing defect), split out of this memo unchanged at
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
  In this memo a bare "R1"–"R13" is a §5.2 residual. Any other review round names its source: "Codex Rn"
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
pin again: every exec-through shim (the pin, the fixtures' shims and `cfgkept`'s, one builder) drops its own
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
  build (`…-census.md` §4, "The references and the working files");
- the pin `$_FGIT_BIN` and the wrapper in it.

The shell options and the window's fd 2 are **not** in it. `errexit`, `nounset`, `pipefail` and where
fd 2 points are the build's own observable state, and W's re-check reads them after the fixtures file
as it reads any output of the build (§3). So an option left off, or stderr left redirected, at the end
is class (b), red by W; switched off and back on in the middle, it leaves nothing to read, class (c) by
the first clause. With that, no mistake falls in both classes.

**The design rule (D16-B): the verifier's files are out of reach by construction.** Every file the
verifier writes and reads back after the build lives in a directory the verifier makes after the build
with `mktemp -d`, never under `$CTL` (the fixtures' tree) or `$_FW_DIR`: the postconditions' directory
(`_pq`, `…-census.md` §4) and the parent's (`…-census.md` §4, "The parent's directory"). Class (c) is then **declared** only for
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
# harness at `e271cb02`, abridged; the harness is the source
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
  printf "%s\n" "$_FW_CANARY" >&2               # C12: the parent requires it in the capture
  _fgit_postconditions && : > "$_FW_DIR/post_ok" || : > "$_FW_DIR/post_bad"   # trust needs post_ok
  _seal_apply                                   # mode restrictions, after the census
  printf "%s" "$_FIX_FAILED" > "$_FW_DIR/fix_failed"
  : > "$_FW_DIR/done"' _ "$_FW_DIR" 2> "$_FW_DIR/stderr" || _fw_rc=$?
# then, still in _fgit_window: W3 scan, canary check (a failed scan is held apart, so the order of the two cannot erase it), replay, and _fw_trusted (below)
```

**What the child gets, kept minimal.** The prelude contains:
- `set -euo pipefail`;
- plain `name=%q` assignments. These cover the data the fixtures file reads (`CTL`, the five
  `CONTROL_*` samples, `_REAL_GIT`, `_REAL_GREP`, `_fifo_ok`) and the window's own `_FGIT_VOID`,
  `_FGIT_ENVBIN`, `_FGIT_ENV_REQ`, `_FGIT_WIRE_EXEC`, `_FGIT_WIRE_LC`, `_FGIT_BIN` (P-j), `_FW_DIR` and `_FW_CANARY`, and the
  postcondition labels, passed by name rather than as positional arguments: the names are the ones
  `_fgit_postconditions`' body uses, each of which the controls file (where the ratchet counts them) must
  define non-empty, or the window refuses. The fixtures' part of the list is **derived** by the census
  command below;
- `declare -f` bodies of `_fixture_failed`, `_fixture_did_fail`, `_shq`, `_fgit_shim`, `_fgit_canon`, `_fw_opts_on`, `_seal_refuse`,
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
it counts the window complete; a missing canary makes the window incomplete, W. One `awk` finds the canary
(exit 0 found, 1 not found, 2 or more: the scan itself failed, held apart and red under `$_fwd_lbl`, never "not found"); a second cuts it out of the replay, and its write failure is ignored (a closed parent stderr is not a redirected window). Without this, `exec 2>/dev/null` followed by `: $((1/0))` in the fixtures file sent the
diagnostic W3 reads to `/dev/null`, rc 0, PASSED on both shells (round 16's `xexecarith`); with it, rc 2,
W with this sentence (`…-pr527-r15.md` §Q). The check is in the parent, so no fixtures-file function
can shadow it; a canary the fixtures file writes itself is a write to the return channel, class (c).

With no `cause`, W says the window exited with its status before completing: an `exit` in the fixtures
file (the fixtures' own `exit 2` reads "the window exited 2 before completing"), an abort, or an aborted
postcondition.

A parent-side failure to create the window directory or write the prelude has its own reason text.

**No time bound (declared, §5.2 R9).** A child can still block on what the fixtures built where no
shape check sees it: git reads, outside every census git dir, something whose read does not complete
(a FIFO; `/dev/tty` under a controlling terminal), through a reference the census does not follow
(`…-census.md` §4, "The boundary"). The run then waits: in CI until the job timeout, which is red; locally,
indefinitely. It is never green. The one watchdog is `_ctl_run`'s (#501 R92; `_control` and the two non-`_control` blocks run through it; its seconds are `_ctl_wd_s`, 30, set where the run primitives are sourced, and the `_wm` standing check runs a wire that sleeps past a 1 s watchdog to pin the kill's report and marker). Draft 12
bounded the whole run; round 11 found seven defects in that design, and the user carved it out of this
PR into `#11-trip-wire-liveness-bound`.

**`ci.yml`.** C9 and C10 add one `find` and one `git count-objects` per fixture git dir to a clean
run. The job's budget is read from **a run of the job on GitHub's runner**, never from local runs
scaled by a factor (D15-B, #510's convention at `106387e4`). #527 is stacked and gets no runner run.
X9 is route (b), decided by the user when #527 was opened: #501's CI after #527 is squashed into it. So
#527's job comment names the runner method, with no local method beside it and no figure, and the
citation-hygiene lane writes the verdict with the runner figure on #501's branch after that run, in
the open implementation record (§13) and never in `ci.yml` (`…-landing.md` §9, "Where the runner figure comes from"; land order step 7). X8's local runs at T are a
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
controls file, because W4 does not stop the run. **Trust is a property of the window**: the run's
flag **`_fw_trusted`** is set to 1 by `_fgit_window` alone, at its end (1 only for a complete build whose
postconditions wrote `post_ok`, positive evidence: no `post_bad` is not one; and no W3, and no unreadable marker, `_fw_unread` empty); it starts at 0 where the harness assigns its state, and the controls file's writes to it (`_cap`'s untrusted row, `_fw_trusted` set to 0 and restored) are in a subshell, so the run's flag never sees them; and the gate
`_ctl_gate` decides by `_fw_trusted` alone; the verdict function only reports (C8's version computed the flag, so a verdict
call below the first control left a clean build untrusted: ROg red; fixed, `…-pr527-r15.md` §Q). The
function continues only over a build that is **complete and trusted**:
- incomplete: W alone, exit 2 (above);
- complete, but a postcondition or W3 reported, a marker unreadable, or the postconditions reached no verdict (neither
  marker): those reports (the last two each its own line under W's label, `$_fw_lbl`; W3 under `$_fwd_lbl`), then exit 1. A red is a decided verdict;
- otherwise: the controls run.

W4 does not stop the run. A seal that failed marks its fixture failed, `_control` already refuses that
fixture's controls, and the other fixtures are sound. The one gate, `_ctl_gate <label> <fixture>…`, asks both
questions, "complete and trusted" and "the trees the run is aimed at built" — its root and its cwd, which `_ctl_run` derives from its own arguments —, and is asked by the ONE run primitive `_ctl_run`, which also owns the watchdog (`_ctl_wd_s`, 30 s); `_control` and the two non-`_control` blocks are built on it, so no control runs over such a
build wherever the exit sits. The fsmonitor block's plain `git ls-files`, a follow-up made outside `_ctl_run` and after it has returned (only when the gate passed and the hook did not run under the wire), sits inside the branch that its `_ctl_run`'s ask opened, so the block asks the gate once.

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
  "incomplete" only). Its predicate is one function, `_ctl_gate`. **`_ctl_run`
  asks it right after registering its label (its channel arguments are validated after that, whatever the gate answered, and before any delivery)** and refuses unless the window is complete and trusted (E3) and the root and cwd it is aimed at built. The other trees a run names are not asked: every PATH-prefix shim (the fixtures file builds them all, `catfail`'s `cat` among them; the `catfail` tree itself is the root of its own control and gated there) and the routed control's decoy are built bare in the window, so a failed build of any of them ends the window (W) before any control, and the `committed` control's decoy is `$CTL/clean`, the root of other controls; `_control` and
  the two blocks that are not `_control`s (relative scratch, fsmonitor) run through it (the fsmonitor block's plain-git follow-up call, made after `_ctl_run` returns, sits inside the branch its `_ctl_run`'s ask opened, so it asks once; the umask run is a `_control`, its `$8`). The gate's fixture half is its last loop, inside `_ctl_gate` (there is no separate `_ctl_built`): the three standing checks in the controls file that run `_ctl_run` run it over `$_VFY`, a root no fixture build makes, so they ask no fixture question (their stubs read no tree), and each reads `_fw_trusted` itself before its rows (untrusted: red in silence — W2 is the gate's report, printed once by the first gated run).
  Each of the three creates `$_VFY/.<x>_end` (`.wm_end`, `.ev_end`, `.cap_end`) when it reaches its verdict, green or red, and the `then` branch reds `the check ended before its verdict` under the check's label when the file is absent (added in CR-B2a: the record for the reader's flag-name validation was WRONG REASON on bash 5.3, where `${!x-}` aborted `_cap`'s subshell and left the run red with no label). Each marker is pinned by a record that injects an abort into its check's subshell before the verdict (killed through the check's label, `the check ended before its verdict`).
  So no control runs over such a build, wherever the exit sits. **Its three records pin
  that W2 is reported when the exit is gone**. One removes both of `_fgit_window_verdict_exit`'s exits
  with the window kept incomplete (`done` renamed). The other two each remove its untrusted-build
  `exit`, one with a postcondition forced red (`post_bad`), the other with W3 forced (`_fw_diag` set).
  Each survives with its clause (`done`, `post_bad`, W3) removed from `_fgit_window`'s trust computation
  (measured, `…-pr527-r15.md` §Q); `post_ok`'s two records (review R2, corpus §6.1) pin the fourth.
  A gate satisfies all three. `_ctl_run`'s ask is pinned always-on, not by a record: removing it reds the run under `$_cap_lbl` (six of the standing check's ten rows: the two that want a decline, `M 0 1 yes` and `Y 1 0`, get a run; the three that want the gate's red, U, F1 and F2, and the untrusted row `M 0 1 no` come back 0; row X stays green, since `_ctl_run` validates its channels itself) and `$_cen_lbl` (the gate is the registry's writer: every declared member is then no `_control` this run made), measured at CR-B2a, both shells — records.sh, beside W2's record. There is one ask per run, no second gate (the fsmonitor block's former ask before its preflight went when the plain-git check moved inside the branch of its `_ctl_run`'s ask, as a follow-up after the run) and one producer of W2's line (`_ctl_gate`; the three standing checks that read `_fw_trusted` print nothing), so W2's records separate the run path — the gate's trust arm bypassed, record 160's needle is not printed: W alone, the controls run over the untrusted build and the standing checks red in silence (measured, R7 S3 review-1 fixes, both shells, when there were two) — and the earlier declared gap — two producers each killing the other's record — has no subject.
- Moving the exit below a control is an edit to the controls file. That
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
behind the defined-labels check (`$_lbd_lbl`, a static census of the label names the wire and the parts spell), and it is pinned by its own record. **The orderly-end flag (`8f4ad3c8`).** A run may leave with status 0 only through its verdict: `_K2_ORDERLY=1` is set once, just before the wire's PASSED line, and the wire's EXIT trap body `_k2_exit` (one function, also called by the mutation run's trap, which replaces the wire's) turns a status of 0 without the flag into `CONTROL FAILED (the run ends only through a verdict)`, rc 1. Capturing `$?` first does not help: on 3.2 the trap is entered with `$?` = 0 (measured). The flag's assignment has a record; removing the check has none (bash 5 exits 1 by itself, so it could die only on 3.2); the evidence is the R2-3 cell, 3.2 rc 0 before and rc 1 after (`…-pr527-r15.md` §Q).

## §4 Postconditions — run INSIDE the window

Every git in the window whose inputs **no fixtures-file command altered** inherits the same
environment and the same persisted state. So git's own answers inside the window describe every such
git (§5.1 covers the rest). Each producer has its own label (defined in the controls file and passed
in), and each label has its own record (corpus §6).

| id | label | assertion (inside the window unless the row says **in the parent**) | liveness (its own label) |
|---|---|---|---|
| W | `the fixture build window completed` | the fixtures file's last line wrote `built`, the options were still on after it, the child's canary reached the captured stderr (C12), and the child wrote `done`. Otherwise NE with the sentence the child wrote to `cause`, or the exit status (§3), **reported alone**; exit 2; no control runs | — |
| W2 | `no control runs over an incomplete or untrusted fixture build window` | asked by `_ctl_run` (through `_ctl_gate`, after its label is registered; the run's environment travels as trailing channel arguments, there is no env global), which every `_control` and every non-`_control` block runs through: the window is complete and trusted (`post_ok` written, no postcondition or W3 reported). The three standing checks that read the flag before their rows print nothing: the line has one producer. Its records: the exits removed, with the window incomplete or untrusted (§3). The exit placed below a control: the RO cells. Trust is decided by `_fgit_window`, never by the verdict function (§3) | — |
| W3 | `the fixture build window ran without a shell diagnostic` (renamed in the fix round: `prelude.sh` is not the fixtures file) | no record of the child's stderr contains `./fixtures.sh:` or `./prelude.sh:` **anywhere in it** (both are sourced by those relative names from the window's directory; bash appends a diagnostic to a record the fixtures left without a newline, §3). A failed scan is red. A record pins each prefix, `./prelude.sh:` by an expansion error in `_fgit_postconditions` | — |
| W4 | `every mode restriction a fixture sealed was applied` | every `_seal` was accepted (a path under `$CTL`, with no newline or TAB; an octal mode; a fixture name of `[A-Za-z0-9_-]`) and its `chmod` succeeded; otherwise red, not a machine limitation. `_seal_apply`'s refusal of a path through a symlink is asserted at load by a standing self-check (`_seal_apply_probe`) under this label | — |
| P-a | `a window git whose inputs no fixtures-file command altered reads configuration only from its repo's config file` | every `git config --list --show-origin` line in **one probe repo** has the origin `file:.git/config` | `this git reports a configuration origin outside the repo's file`: `-c a.b=c` must show with the origin `command line:`. `--show-origin` (git 2.8) alone, not `--show-scope` (git 2.26): the origin column answers the same question, so the check has no version floor and no limitation arm |
| P-b | `the fixture git has no system or global layer outside the void` | asked **inside P-a's probe repo** (`cd` there, `git --git-dir=.git`: no discovery), so no caller repository's local configuration is read. A git older than 2.42 (its `git var` does not name these four) is a machine limitation, worded as the fact `P-b — <git version string> is below 2.42, so … is not asserted (all four or none)` (`⚠ NOT EXERCISED on this machine`, green: a `fact` row of the capability table, so no declaration is asked for it, `…-cr-plan.md` §2.3), and the limit is **keyed on the documented boundary, not on the shape of a refusal**: `git --version` below 2.42 limits all four names at once (one line, all or none; only the documented usage status 129 is excused there); from 2.42 on a 129, or any non-0 GLOBAL call, is red under this label. A version string that does not parse (no `git version <digits>.<digit>…` prefix, or a major/minor part that is not all digits; `2.39.3 (Apple Git-146)` parses) counts as 2.42 or later: the fail-safe direction is a red on a refusal, never a green limit line. The limit line is printed last and never outranks a red. (P-f keeps a definition test, as `env` has no version contract: a decision, not a second way.) Otherwise `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is an empty directory, tested by the shell's own globs (`*`, `.[!.]*`, `..?*` with `-e`/`-L`) — not by `$(ls -A …)`, which loses a name made only of newlines and reads a failed `ls` as empty. A void that is not a directory, or cannot be read and searched, is red: globs over it would expand to nothing. The globs run with `set +f` and `GLOBIGNORE` unset, whatever the fixtures file accidentally left (a deliberate `readonly GLOBIGNORE` is threat-model class (c)). Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `no exec-path override reaches the fixture git` | the window's `git --exec-path` equals the same git's answer with only `PATH` and `HOME=$_FGIT_VOID` in its environment (taken in the parent at source time), both canonical (`pwd -P`). `HOME` is there because a caller's `git` wrapper that reads `$HOME` under `set -u` aborts without it, which gave an empty reference and a red with no cause (fix round #3); no record pins it, since dropping it reds only on a machine with such a wrapper. **Re-scoped by `/code-review`:** which executable runs is outside P (§0.1, R1); P-e pins only that nothing in the window overrides where that git runs its commands from. So a caller's `GIT_EXEC_PATH` or `DEVELOPER_DIR`, or one directory spelled two ways, is not a red | — |
| P-f | `the fixture build window's environment holds only its allowlist` | the name of **every** `env -0` record (the text before the first `=`, so non-identifier names such as `BASH_FUNC_f%%` are included) is an allowlist name (derived from `_FGIT_ENV` itself, so the two cannot drift) or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction, and so is an allowlist name missing from the records (`missing …`: a name exported away inside the window, which P-h, P-i and P-j read as shell variables and still see; pinned by review R2's `export -n LC_ALL` record). A listing that is not NUL-separated reads as no record (the NUL-list policy, `…-census.md` §4, "P-g's population"). p6's `sed` parser skipped non-identifier names; p7 parses every record (companion §A.10). `env -0` goes through a file whose status is checked: a failed `env -0`, or no record at all, is NOT EXERCISED. **One** outcome is a machine limitation instead (`⚠ NOT EXERCISED on this machine`, green **only where its caller declared it**, `WEBREF_WIRE_LACKS=P-f`: the limit is a `limit` row of the capability table, and an undeclared one is red under `$_pfl_lbl`, naming the declaration): its test is the definition — **this** `env` runs but refuses `-0`: `env -0` fails, writes nothing to stdout and says why on stderr, while the same `env` runs a command. Every other outcome stays red, so an unknown one falls on the fail-safe side. Which `env` builds lack `-0` is not measured here (this machine's macOS 26 `env` has it; PR #527 Codex R4 reported a macOS one without it); the no-`-0` case is exercised with a shim. On a declaring machine both limitations reach the run's summary through one channel (`$_FW_DIR/machine_limits`), naming the postcondition by ID only, since a record's needle is its label; `_cap_check` reads that channel and decides whether each line is acceptable (P-b's: always, a fact; P-f's: declared); undeclared, the P-f line reds and the run exits (controls file, the `ctl_ok` exit) before the summary. History: `…-pr527.md`. | — |
| P-g | `every fixture repo persists only the configuration a plain git init writes` | for **every git dir under the fixture root** (population in `…-census.md` §4), the listing of `git --git-dir=.git config --list --show-origin` run in that repo (no discovery, so a `.git` git does not recognise is an error, never an ancestor's listing; no `--show-scope`: the origin already names the file) **equal, as a set,** the lines of a **reference** `git init` made in the same window (P-a's probe repo, which is that init, made after the build in the postconditions' directory, which no fixture command that finished by the end of the build can name: `…-census.md` §4, "The references and the working files"), compared by origin, key and value. An extra entry is an input the fixture persisted; a missing one (`git config --unset core.filemode`) hands that setting to the platform default — either way P would depend on more than the fixture. The comparison is of **records**, and it runs first: the `-z` listing, each origin paired with its entry into one record written as one line by `printf %q`, sorted (no `sort -z`), byte-identical to the reference's — so a value holding a newline cannot forge a line, and a repeated entry is red. Only when the records differ are the lines listed (`grep -vxF -f`, both directions), to name the extra and missing entries; that runs on a red path only, so its failure cannot turn a red green. This catches a persisted include (its origin is not `.git/config`), a `commondir` naming another git dir (the origin names that dir's config), and **any** persisted key beyond init's, `core.excludesFile` included. **Census first (M-SHAPE, drafts 14–15):** the census classifies and shape-checks every git dir it names, and runs its per-link search, and its one verdict comes before any postcondition runs git (population in `…-census.md` §4). An entry under a `.git` that is not a directory or a regular file with one link is red: a symlink of any target, a FIFO, a socket, a device, a hard link. Git reads through a link (a `.git/config` pointing outside still reports `file:.git/config`) and blocks opening a FIFO, whatever the entry's name. **A red census runs no git anywhere**, because git reads across git dirs (`commondir`, `alternates`, `include.path`). A scan that fails or writes to stderr is red. The census takes `HEAD` and `.git` entries of **any** type and letter case — a symlink `HEAD`, or `head` on a case-insensitive filesystem, is a shape git reads — and every git dir not reached by a `.git` entry is red. **Both directions:** a git dir whose listing is EMPTY or fails (a `.git` git does not recognise, e.g. a garbage `HEAD`) is red | — |
| P-i | `the fixture repos use the files ref format` | the window's `GIT_DEFAULT_REF_FORMAT` is `files`, and a plain init's `git rev-parse --show-ref-format` answers `files` (a git before 2.45 has no reftable; its `rev-parse` echoes the unknown option back with exit 0, and that literal echo is what counts as `files` — a failed call or any other format name stays red). It pins the allowlist entry on every git, including the files-default ones where dropping it changes nothing else | — |
| P-j | `the fixture build window runs the pinned git first on its PATH` | the window's `PATH` starts with `$_FGIT_BIN`, `type -P git` inside the window is `$_FGIT_BIN/git`, and a `git` run through the pin is handed a `PATH` without `$_FGIT_BIN` (an alias, `-c 'alias.k2p=!printf %s "$PATH"'`, prints it). That last clause pins the shim builder `_fgit_shim`, which writes the pin and every fixtures exec-through shim so that each drops its own directory from the front of `PATH` before it execs: a "next `git` on `PATH`" wrapper then cannot reach it again in any idiom (§0.1). Review R2 replaced the fix round's depth counter (`K2_FGIT_PIN_DEPTH`, exit 125, its own label), which an `env -i PATH=…` re-exec reset, so the cycle hung. It pins the `PATH` construction the way P-h and P-i pin their entries: without the pin, the window's first `git` is the file the pin execs anyway, so nothing else changes, and P-e's reference moves with it. It asserts nothing about the caller's entries (§0.1) | — |
| P-k | `no fixture repo reads objects from a store outside it` | for every `.git` P-g compares, `git count-objects -v` (run as P-g's listing is) exits 0 and prints no `alternate:` line (read by one builtin helper, `_fgit_alternates`, which assigns `_fgit_alt` and is shared with the liveness probe; no external `grep`). An `objects/info/alternates` naming another store makes git read an object from there instead of writing it locally, so the content the controls read is an outside input even though P's ids do not move (measured, `…-pr527.md`). Git's own answer, not a file name: it reports the stores git will read (measured for the `alternates` file). A failed `count-objects` is red, and **so is any stderr**: a store that does not exist, or an `alternates` naming a directory or a mode-000 path, gives rc 0, no `alternate:` line and a warning or error on stderr (round 12; `…-pr527.md`), and such a store could appear later and be read. It runs only over a clean census (`…-census.md` §4, population) | `this git reports alternate object stores`: a third probe repo in the postconditions' directory (so P-d's two inits are untouched), given an `alternates` file naming P-a's object store, must exit 0 with nothing on stderr and print exactly one `alternate:` line. The file holds the path in double-quoted C form, so a newline in the scratch path stays one line. `gitrepository-layout` says only "one pathname per line". git(1) documents C-style quoting for `GIT_ALTERNATE_OBJECT_DIRECTORIES` (`man git`, that variable), and that the `alternates` file accepts the same form is measured: on 2.55.0 a path holding a newline gives one `alternate:` line, rc 0 (`…-pr527.md`, round 12's P-k commands). **Anything else is red**, NOT EXERCISED: there is no machine-limitation arm, because "prints none" cannot tell a git without the line from a broken probe (git 2.55.0 and Apple 2.54.0 both print it, measured), so the unknown falls on the fail-safe side |
| P-h | `the fixture build window reads in the wire's locale` | the window's `LC_ALL` equals the wire's (`C`). It pins the allowlist's `LC_ALL=C`, which P-f cannot, because P-f derives its names from the same list | — |
| C | `every label the run used is named by a mutation record` | **in the parent, after the last control, not inside the window** (the census, `_mut_correspondence`): each record's needle equals a label the run used — one a `_control` / `_ctl_run` / `_ctl_gate` / `_fgit_window_verdict_exit` call registered (the run registry, one call site per label: the outermost controls-file frame; a `_control` label is literal, never a `_*_lbl` value — `_control`'s rule only: a direct `_ctl_run` / `_ctl_gate` / verdict call may pass one, so a direct block under an echo-only producer's variable label is a residual, as two calls on one line are) or the value of a `_*_lbl` name the wire or a part EXPANDS (`$name`, or `${name` and a non-identifier character, on a line outside a comment (the text from a `#` that begins a word, at the line's start or after whitespace, to the line's end, cut on every line before matching; `$#` and `${x#y}` are no comment; a `#` after whitespace inside a quoted string is cut too, which can only drop a real expansion, so a false red and never a false green) and outside the record table, whose here-document is quoted; a definition and an indirect `${!x}` are no use: `$_cen_used`) — the labels no record names number exactly `_MUT_UNRECORDED_MAX`, and the registry has not grown by the controls file's last statement (`_mut_census_end`). **The registry is bijected with the text:** the call lines of the controls file (a non-comment line naming one of those four primitives as a word; kind `control` when it names `_control`, else `direct`) are exactly the registered sites, kinds included, so a call that never runs, a call registered at another line and a call the text hides (`eval`, a split word, a helper defined on another line) are each red, so the registry's sites are the same on every machine (its rows and labels per site too: every call site passes a fixed label, measured identical on three PATH profiles); a registry row holds exactly three TAB fields (one parse). Every declared member of the capability table is a `control`-kind label of this run (the membership check). Pinned by the 21 records whose needle is this label (the registry's writer removed, a literal label at the window verdict and at the fsmonitor run, a `( … )` call after the census, the variable half emptied, a name's one expansion deleted, a recorded control deleted, a bare one deleted, a bare label on a second control, a second verdict call, a controls-file helper called from two lines, a `_*_lbl` value as a `_control` label, relcwd's call behind `if false &&`, a `_control` behind a branch (on a machine with a FIFO the kill is the call line with no site; on a no-FIFO declaring machine the branch runs and the kill is the ratchet, 19 against 18), the one expansion deleted with the name left in a trailing comment on another line, each standing check's row loop emptied (three), a `_control` folded into a two-line command substitution, the registry's writer adding a fourth field, a `member` row's label changed to `…closeX`, which no `_control` of the run carries) | — |
| CAP | `the gate declines exactly the members of a group whose flag is 0` | **in the parent** (the standing check `_cap`, ten rows through one `_ctl_run` call line over `$_VFY`; each row replaces `_CAP_TABLE` and the synthetic flags `_zz_ok` / `_yy_ok` in its subshell and sends its stderr to `$_VFY/.cap_err`, so an expected red never reaches the run's output): the gate's capability arm (`_ctl_gate`, after the trust arm and before the fixture loop, reading the table through ctlrun's `_cap_rows`, `_cap_group_of` and `_cap_flag_of`) returns 2, with no run, exactly when the member's OWN group's flag is 0; any other value runs the control. The member's label is compared whole (a proper prefix and a proper superstring of it are not it); an untrusted window answers 1 before the capability arm, printing nothing beyond W2's one line (already said in the row); a group no `group` row declares, and a flag field that is not a name (`a-b`, `1a`), are red naming the group, before any indirect expansion; a malformed channel argument under a decline returns 1 naming it (`_ctl_run` validates its channels whatever the gate answered). A row the loop cannot parse, or no arm asserts, is red naming the row. Pinned by the records whose needle is this label (the lookup's equality always true and a substring match, the flag lookup's equality always true, the decline arm's `return 2` removed, the trust arm placed after the capability arm, the no-flag arm turned into a pass, the reader's name validation removed, the `_cap` loop's `U` arm renamed, a decline row made unparseable (the parse guard), an abort injected into the check's subshell before its verdict (killed through the verdict marker), and the gate's answer taken as an early return again before `_ctl_run`'s channel validation, `|| return $?`, which dies at the malformed-under-a-decline row) | — |
| CAPT | `the capability table is well formed and names every absence it is asked about` | **in the parent** (`_cap_check`, a table check and not a standing check: it runs no `_ctl_run` row; called once after the window verdict, before the first gated run, its call line the writer): every table row parses; group names are unique; every `member` row names a declared group; member labels are unique; every group's flag is set and 0 or 1; every word of `WEBREF_WIRE_LACKS` names a group or a `limit` row (an unknown word, a misspelt `fifoo`, is red under this label: a stale or mistyped declaration is never silently accepted); and every ID the window reported in `$_fw_limits` has a `limit` or a `fact` row (an unknown limit ID is red under this label). Each failure is red under this label, naming the row; the two absences nobody declared are red under the group's or the limit's own label (rows FIFO, PERM, PFL). The census adds the membership (`$_cen_lbl`: every member label is a `_control` this run made; pinned, under that label, by the record that makes a `member` row's label `…closeX`, which no `_control` makes). Pinned by four records, each killed by this label alone: a `member` row's group misspelt (the clause that every member names a declared group), a probe's flag set to 2 (the 0-or-1 clause), a member label listed twice, and a group declared twice; the flag, member-label and group clauses each survive with their own clause disabled (measured, CR-B2a review fixes) | — |
| FIFO | `this machine holds a FIFO, or its caller declared that it cannot` | the table's `fifo` group label (flag `_fifo_ok`). `_cap_check` reds a flag of 0 under this label, naming the group and the declaration (`WEBREF_WIRE_LACKS=fifo`), unless the caller declared the group; a declared 0 makes the gate decline the group's members (row CAP), and a stale declaration on a machine that holds a FIFO declines nothing (the probe still decides). Pinned by the probe-flip record (`_fifo_ok=1` assigned 0), killed with this label on a machine that holds a FIFO; it survives with the group's absence clause removed. **Cost:** a no-FIFO contributor's declaration does not make the opt-in mutation run green: four records whose needle is `$_pg_lbl` plant a FIFO in the window, and on a machine with none their mutant ends at "decided nothing" (WRONG REASON, mutant rc 2), which nothing excuses (`…-cr-plan.md` §6 BS-7, R3-B2b-2). The probe-flip record, on a no-FIFO declaring machine, is excused by the table's flag (CR-B2c: it survives and is reported as not exercisable, the `fifo` group named; measured on a no-FIFO `PATH` shim, both shells) | — |
| PERM | `this user cannot read a mode-000 file, or its caller declared that it can` | the table's `perm` group label (flag `_perm_ok`): as FIFO, with the group `perm` (`WEBREF_WIRE_LACKS=perm`: this user reads a mode-000 file, i.e. root; the red names it). Pinned by the probe-flip record (`_perm_ok=1` assigned 0), killed with this label on a machine that enforces modes, and surviving with the group's absence clause removed. On a declaring machine whose probe also answers 0 (a stale `LACKS=perm` on a capable machine excuses nothing: the flip mutant survives unexcused, a red, the fail-safe side) the opt-in run excuses the records whose needle is a `perm` member and the probe flip (CR-B2c: reported as not exercisable, the `perm` group named; a record that survives or is killed for another reason by a mutant that exited 1 is excused, a 2 never). Measured under a root-simulating `PATH` shim, both shells; a real root user is unreachable here, BS-7, so a real root contributor's opt-in is not measured, and the records whose plant needs mode enforcement (the `chmod` plants whose needle is no member, BS-7) may still red it, not measured | — |
| PFL | `this env lists its environment NUL-separated, or its caller declared that it cannot` | the table's `limit` row for P-f (the `env -0` machine limit): a `P-f — …` line in the window's `$_fw_limits` is red under this label, naming the declaration (`WEBREF_WIRE_LACKS=P-f`), unless the caller declared the ID; declared, it prints the green `⚠ NOT EXERCISED on this machine: P-f — …` (the slot rule: the ID, not the label; `…-corpus.md` §6, "What counts as a kill"). Pinned by the definition-test record (the test forced to the limit branch on an `env` that accepts `-0`), killed with this label and surviving with the limit clause removed. On an `env` that lacks `-0` with `P-f` declared, the flip record is excused by the P-x half (`_pfl_lbl` is the limit label; CR-B2c, simulation, both shells) | — |
| G | `the boundary-mutant generator derives a non-empty set from the wire's regexes` | **in the parent** (mutgen's `_mut_gen_floor`, always on, called by `_mut_correspondence`): generating the mutants of `$K2RE` and of `$K2RE_PATH` succeeds, and the two together yield at least one; otherwise red under this label. Pinned by the mutgen record that makes `_mut_regex_mutants` return at once | — |
| R | `a relative scratch dir is removed on exit` | **in the parent** (the controls file's relative-scratch block, through `_ctl_run`): a run from `$CTL/relcwd` whose `mktemp` is the `fakerelmktemp` prefix (it answers a relative path, and `_ctl_run` demands the prefix's `path` mark back) passes and leaves `$CTL/relcwd` empty; otherwise red. Pinned by two records: the wire's scratch dir kept relative (`_phys` dropped), and the relcwd fixture's build failing (`_fixture_failed relcwd`) | — |
| F | `a caller's fsmonitor hook does not run` | **in the parent** (the controls file's fsmonitor block, through `_ctl_run`): a `core.fsmonitor` hook injected through the caller's `GIT_CONFIG*` does not run under the wire (exit 0, no mark), and does run under a plain `git ls-files` over the same fixture — else NOT EXERCISED, a red, because the wire's not running it would prove nothing. Pinned by two records: the wire's `-c core.fsmonitor=false` dropped, and the fsmon fixture's build failing (`_fixture_failed fsmon`) | — |
| L | `every label the parts use is defined` | **in the parent** (the controls file, asked before anything else runs): every `_*_lbl` name spelled in the wire or a part of `_MUT_PARTS` is defined and non-empty, and at least one name was found; otherwise exit 2, naming the missing names (or that none was found). Pinned by the record that renames the `_fws_lbl` definition | — |
| O | `the run ends only through a verdict` | **in the parent** (the wire's EXIT trap body `_k2_exit`): a status of 0 without `_K2_ORDERLY=1`, which is set once just before the PASSED line, becomes `CONTROL FAILED`, rc 1. Pinned by the record that replaces the flag's assignment; removing the check itself has no record (bash 5 exits 1 by itself, so it could die only on 3.2) | — |
| WM | `only the watchdog's marker makes a signal status a timeout` | **in the parent** (a standing check in the controls file, two rows run through `_ctl_run` with `SELF` pointed at a script): a wire that signals itself at once is `_rc` >= 128 with `_ctl_run` returning 0 and no "killed after" line; a wire that hangs (a script that sleeps 5 s) past the row's 1 s watchdog (`_ctl_wd_s=1`, set in the check's subshell) makes `_ctl_run` return 1 with the marker written and "killed after 1s" reported, its background descendant dead and the watchdog's process group (`_ctl_wd_pg`) gone after each row; otherwise red. Pinned by the ctlrun record that drops the marker clause of the timeout test and by the controls record that renames the hang row's arm (`hang)` to `hng)`, red as "no arm"), both through the check's label (the marker write, the message and the reaping are pinned by the rows, not by a record) | — |
| EV | `a previous run's channel evidence never answers for the next` | **in the parent** (a standing check in the controls file, two rows on one call line run through `_ctl_run` over `$_VFY`, with `SELF` a stub that calls `k2evprobe` and exits 0): row `ran` delivers a PATH prefix whose `k2evprobe` shim writes `path` — `_ctl_run` returns 0 with `path` present; row `stale` then delivers a prefix holding no `k2evprobe`, so the run calls no shim — `_ctl_run` must return 1 naming `PATH prefix never reached the run`; otherwise red, as is a row with no arm. It pins the evidence directory being made afresh for every delivering call (`rm -rf` then `mkdir`). Pinned by the ctlrun record that weakens that to `mkdir -p` (KILLED through this label, both shells; it SURVIVES with the check's rows removed: measured, CR-S2) | — |

**How P-g's key set is derived (D2).** It is **not** a list of path-typed keys. Git exposes no per-key
type: `--type=path` is chosen by the caller, not declared per key, and `git help config` is prose. A
list of path keys would therefore be vocabulary, and a key missing from it would pass silently.
Instead, the allowed set is **what git itself writes on `init`**, measured in the window on every run.
That is the fail-safe direction: any persisted entry beyond it is red, whether or not it names a path.

**P-g's population, and the census before any git (drafts 14–15); the boundary; the references and the
working files; the parent's directory; the sealing after the census; what P-g cannot see.** Moved to
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-census.md` §4 (touch-time split S4c-0, text
unchanged at `0d411df2`), so "§4, \"The boundary\"", "§4, \"The references and the working files\"",
"§4, \"The parent's directory\"" and "P-g's population" in this memo resolve there.

## §5 Residuals, the slot, and a pre-existing defect

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-residuals.md` §5 (touch-time split, text
unchanged at `f8ea983a`; extended since by D17-A and D18-A; §5.1 what the window does not close, class (c); §5.2 the other residuals R1–R13 and the
pre-existing defect, closed by `8f4ad3c8`), so "§5.1", "§5.2" and "§5.2 Rn" in this memo resolve there.

## §6 The corpus — evidence, and the source of the records

Moved to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md` §6 ("the corpus file";
touch-time split, text unchanged). It holds the recast oracle, the prototypes p6–p11, the re-run
recipe, every focused cell table, and the mutation records with their totals and the ratchet;
its §6.1 holds the record changes of drafts 11–21 (C6…C12), of the fix round and of the review rounds after it.

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
| 2 | A×D | the postconditions run in the window, so they describe every such git. P-g extends that to the persisted configuration and the shape of every git dir under the fixture root: an unknown git-dir shape is red, and so is any entry inside a git dir that is not a directory or a regular file with one link. The census's one verdict comes before any postcondition runs git, and the references it is compared against, with every file a postcondition writes and reads back, are in a directory made after the build, out of the fixtures' reach by §0.3's design rule (what that rule cannot close, `xrace` and a changed command resolution among it, is declared class (c)); after the window the parent's own working files are likewise in a directory it makes then, out of reach by the same rule and with the same declared exception (a changed resolution of the parent's `mktemp`, `staleshim`), and it keeps none of them under `$CTL` (C11; `…-census.md` §4, "The parent's directory"). P-k adds that no fixture repo reads objects from a store outside it | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read. Completion needs the fixtures file's own last line, with the options still on and the window's stderr still the captured file (C12); the markers are a protocol, not a seal (a fixtures-file write to them is class (c), §0.3); an incomplete window ends the run with W alone, a complete but untrusted one ends it after its reports, and `_control` itself refuses unless the build is complete and trusted (W2). Each child option is checked and recorded. With the parent's nounset off, the clean tree and eight red cells give the same verdicts (measured, §3; not a proof over every path) | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | labels live in the controls file (every `_*_lbl=` definition is there), and a record's prefix names the part its edit is in — a word of `_MUT_TARGETS`, or none for the wire. A record targets `fixtures` when what it plants is something a fixture writes (the W early return and options re-check, W3, and the P-c, P-g and P-k shapes) | §4, corpus §6 |
| 9 | D | one label per producer; every `_*_lbl` label the wire or a part expands, and every label a controls-file call line passes, is named by a record (the generator floor's included) except the `_MUT_UNRECORDED_MAX` (18) unrecorded ones, which an exact ratchet counts (fewer bare labels is red too, so a slack ratchet cannot hide the next one); the registry's sites are exactly the controls file's call lines | corpus §6 |
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
narrow check are fixed in the branch during the implementation and recorded in the open implementation record (§13; `…-pr527-r15.md` §Q is closed),
not re-reviewed as a plan.

## §13 Implementation results

The results before PR #527 are in `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md` §13
(touch-time split; history). PR #527's results through plan-review round 14 are in `…-pr527.md` §P,
which is closed. Every result from round 15 on, the planned commits included when they are made, goes to
one place at a time, **the open implementation record**: `…-pr527-r15.md` §Q (closed at 799 lines) → `…-impl.md` §R (closed at 799 lines) → `…-impl-cr.md` §S (open
while under 800 lines) → at §S's rollover, this section names the successor file before the first entry goes there. Future-tense routes say "the open record (§13)", not a file
name (`…-landing.md` §9.1, step 2).

**Rollover (D19-C).** When an entry would take `…-pr527-r15.md` past 800 lines (`wc -l`), that entry
and every later one go to a new file, `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl.md`,
§R, with the same no-spec-surface §0, and §Q ends with a pointer to it. `…-impl.md` reached 799 lines with
CR-S8a (`161ae982`); every entry from then on goes to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl-cr.md`,
§S, by the same rule, and §R ends with a pointer to it. This memo's split at 950 lines is
done (S4c-0): §4 from "P-g's population, and the census before any git" to the end of §4 (the census,
the boundary, the references and working files, the parent's directory, sealing and what P-g cannot
see) was moved unchanged to `…-census.md`, with a pointer left in §4; the memo went from 950 to 729
lines at `20615255` (`git show 20615255~1:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md | wc -l` → 950;
the same path at `20615255` → 729), and no further seam is named.
