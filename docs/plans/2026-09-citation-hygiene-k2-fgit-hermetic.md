# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.
**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, the fate of `ff6b99a3`'s commits, the premises that turned out false, and every
plan-review round's dispositions, including round 4's terminator. This memo holds only the live
decisions.
**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (§8.3).
**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).
**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 4, written from a measured artifact.** Round 3 found 7 IMP on draft 3, all of them
on text draft 3 had added, and 4 of them moved the mechanism. That was the second self-introduced reset.
So this draft changes the process:
- An **executable adversarial corpus** was built first.
- A **prototype** of the design was driven to pass it on both shells and both gits.
- Every mechanism claim below is a corpus result. The corpus command is the evidence (§6), and the
  corpus becomes the PR's mutation records and exit criteria.

Round 4 is owed.

⚠ **The parent's rule applies here.** No quantity in this memo moves with a commit; each figure is a
named-artifact measurement given with its command. `$S` means scratch; the real `HOME` is never written;
`/usr/bin/grep` is spelled out because the default `grep` is ugrep, which can silently return 0.

---

## §0 Why this exists

Codex raised a P2 on #501: `_fgit`, which builds the K2 wire's fixture repos, did not neutralise
`GIT_TEMPLATE_DIR`. This was the second time a name list around `_fgit` failed. The first was #519's
Codex R7 (`GIT_CONFIG_COUNT`, parent §10.6).

`ff6b99a3` then replaced the list with a `GIT_*` sweep plus a keep-set. That failed a third time, on
`$HOME/.config/git/ignore`. That file is the **default** of `core.excludesFile`; it is neither a `GIT_*`
name nor a config layer (companion §A.1).

So this memo stops listing names. It grounds the mechanism on a property (§1) and chooses the direction
in which an unlisted channel fails (§3).

### §0.1 The acceptance criterion changes, and where it stops

The parent's §11.1 rule was "unreachable or loud, never silent", and it classed D6 as loud. **For the
fixture build, that criterion is replaced by property P.** A loud failure that the caller causes is also
a defect. The failures measured were all loud (rc 1), and all came from **ordinary developer
configuration** (companion §A.1). In a required gate, a false red caused by that kind of setup blocks a
valid checkout.

**Boundary.** P governs the **inputs git reads when it decides what to stage and commit**. It does not
govern which executable runs, or the platform it runs on (R1, R2, R5). A failure there means "this git
cannot run here". Such failures stay under the old rule (loud is acceptable). The reason: only inputs can
be neutralised from inside the process. An executable or a platform can only be *chosen*, and the
launch-environment slot owns that choice.

The **read** side keeps the old criterion. `_git` preserves the caller's configuration on purpose; the
reason is recorded as #501 R97.

### §0.2 One slice, and where it is registered

CLAUDE.md's edge-dense rule applies (§10 has ≥3 axes). The umbrella carries slice row A-i-wire-fgit and
memo-table rows for this memo and its companion. Both are pointers. The umbrella amendment is itself
under review.

The slice is property P for the fixture build: the mechanism (§3), the postconditions (§5) and the
bypass detector (§4).

- **C1 and C2** are standalone prereq splits (§8).
- **C4** is required, because every record in §6 edits the harness.
- **C3** (record comments) is **not** required to write records, since the base holds records without
  it. It stands on its own ground: the machine-dependent records (§6, the P-d and P-e pins) must say
  where they discriminate *at the record*.

CLAUDE.md's clause reads "(feature PR に bundle しない — split は単独 PR / 単独 commit)". A standalone
commit is one of the two forms it admits. The precedent is the parent's §11.7 row 7. §12 Q1 asks whether
round 4 reads it otherwise.

## §1 The property

The controls assert over staged **content** and over **commits**. An attributes file can re-encode a
blob, and a template hook can drop an entry from the index and from `HEAD`, and neither changes the path
set (companion §A.1).

> **P.** For each fixture, these are a function of the fixture script's git commands and the files it
> writes, and of no git input the caller carries:
>
> - the index (path, mode, blob);
> - the tree of `HEAD`;
> - the ref `HEAD` names;
> - every ref, with a commit compared by its tree.
>
> "Carried" means environment variables, files at default locations (home, XDG, system prefix,
> compiled-in template), and the configuration those name.

Refs are part of P. A hostile `init.defaultBranch` or template `HEAD` moves the branch that `headprobe`
and `badref` build from. Replace-ref and `badref` values are deterministic, so they are compared as
values. Commit ids vary with timestamps, so a commit is compared by its tree.

Three things are **outside P** by §0.1's boundary:

- timestamps and reflog identity;
- the build's non-git commands (§5 R4);
- reads at control time.

## §2 Measurements

The measurements moved to companion §A. The corpus (§6) supersedes every earlier prototype's
whole-wire result. ⚠ The draft-3 mutant runs (companion §A.4) ran in copies with `.git` removed, so
every whole-wire run also went red for an unrelated reason ("read 0 stored objects"). Their results hold
only at label level, and they are superseded.

## §2.5 Spec coverage map

**No spec surface** — this slice changes a shell helper, its controls and its mutation records. It
touches no WHATWG / W3C / TC39 / CSS WG behaviour.

The authority for the channel classes is git's documentation at the version measured: `git help
gitignore`, `git help git-config` (FILES), `git help gitattributes`, `git help git-init` (TEMPLATE
DIRECTORY), `git help git` (ENVIRONMENT VARIABLES) and `git help git-var`. Webref does not index these.
The corpus supplies the rest of the authority.

This follows A-iii's shape and the parent's §0.5/§3: the heading has **no table**, so `preflight.py`
fails **by design**. The marker is A-ii's §4.2.5 feature, which has not landed. The marker line matches
A-ii's recogniser `^ {0,3}\*\*No spec surface\*\*` (A-iii Codex R13).

```sh
/usr/bin/grep -nE '^ {0,3}\*\*No spec surface\*\*' docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md   # one hit
python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md; echo "rc=$?"   # rc=1, no table
```

## §3 Mechanism, part 1 — `_fgit` is built from nothing

| shape | a channel nobody listed… | verdict |
|---|---|---|
| (i) unset a list of names (base) | reaches the build | failed twice |
| (ii) `GIT_*` sweep + keep-set (`ff6b99a3`) | reaches it unless it is both a `GIT_*` name and a config layer | failed a third time |
| (iii) **`env -i` + allowlist**, with every default location pointed at an empty directory | **closed**, listed or not | **chosen**; corpus G rows (§6) |
| (iv) OS sandbox | closed, compiled-in paths included | no portable mechanism, and it needs privileges |

`git help git` (`GIT_CONFIG_NOSYSTEM`) endorses these *ingredients* ("along with $HOME and
$XDG_CONFIG_HOME to create a predictable environment"). Leaving `XDG_CONFIG_HOME` unset is safe because
`git help git-config` (FILES) says: "When the XDG_CONFIG_HOME environment variable is not set or empty,
$HOME/.config/ is used". gitignore(5) and gitattributes(5) say the same.

```sh
# harness — a snapshot taken when the harness is sourced, never re-evaluated per call
_FGIT_VOID="$SCRATCH/fgit-void"          # mkdir, checked
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID")
_fgit() { command env -i "${_FGIT_ENV[@]}" git "$@"; }
```

What each allowlist entry buys, measured on git 2.55.0 and Apple 2.54.0. Each row is also a corpus pin
(§6).

| entry | without it |
|---|---|
| `PATH=$PATH` | BSD `env -i` runs `/usr/bin/git` (Apple 2.54) rather than `PATH`'s git: compare `env -i HOME=$S/v git --version` with `env -i PATH="$PATH" HOME=$S/v git --version` |
| `HOME=$VOID` | leaving `HOME` unset also closes the channel on 2.55 (`git var GIT_CONFIG_GLOBAL` → rc 1). The void is chosen because a future HOME-relative default then lands where P-c looks |
| `GIT_CONFIG_NOSYSTEM=1` | the system layer is live here (`git var -l` under `env -i` prints `credential.helper=osxkeychain`) |
| `GIT_ATTR_NOSYSTEM=1` | undocumented at 2.55 (`for p in git gitattributes git-config gitignore; do git help -m $p \| col -b \| /usr/bin/grep -c GIT_ATTR_NOSYSTEM; done` → `0 0 0 0`), so it is pinned by `git var`'s answer (P-b), not by its name |
| `GIT_TEMPLATE_DIR=$VOID` | the compiled-in template is copied in. A template can seed `config` and `HEAD` as well as `info/*` and hooks |

`command env` stops a shell function named `env` from intercepting the call. `_fgit` no longer calls
`_git`.

One fixture has to change: `notcommitted` writes into `.git/info/`, which only the ambient template ever
created. It gains `mkdir -p .git/info`. Without that, a clean caller goes red with 1 NE (companion
§A.2).

## §4 Mechanism, part 2 — the bypass detector (replaces draft 3's second build)

**What the corpus forced.** Round 3 measured draft 3's hostile members: they changed only `init` and
`add` reading excludes. Bare-git bypasses of `commit`, `update-index`, `hash-object`, `add -f` and `-c`
overrides stayed green, and `.gitignore` `!*` beats `*`. Any **effect-based** member set has that
problem: it has to be universal across every subcommand and every fixture shape, and nobody can
enumerate that.

The detector is **invocation-based** instead. It does not ask what a leaking git would change. It asks
whether any git ran during the fixture build other than through `_fgit`. That makes it
subcommand-independent by construction, and the corpus R rows measure it across every call-site class
(§6).

**The mechanism.** Around the **one** ordinary fixture build, and only around it, two things are set:

- `GIT_TRACE=<abs file>`. Every git that inherits the build's environment appends a trace line, whatever
  its subcommand, its `-c` options or its repository. `-c` cannot override an environment trace
  variable.
- `PATH="<dir holding a git that appends a marker and exits 97>:$PATH"`. Every git resolved through
  `PATH` hits the marker. That includes `env -i PATH="$PATH" git`, since `$PATH` then contains the shim.

`_fgit` sees neither: `env -i` drops `GIT_TRACE`, and its `PATH` is the source-time snapshot.

After the build, both are restored; the caller's `GIT_TRACE` is saved and put back. A non-empty trace or
an existing marker is `CONTROL FAILED (every fixture git call goes through _fgit)`. The message names the
leaking subcommands, taken from the trace.

**Liveness, on the same instance.** With the detector on and **before** the fixtures file is sourced,
two canaries run:

- a plain `( git --version )` must create the marker;
- `( "$_REAL_GIT_BIN" --version )` (an absolute path, inheriting the environment) must write the trace.

Both markers are then removed. If either canary fails, the result is `CONTROL NOT EXERCISED (the
fixture-build bypass detector sees a bypass)`. The caller cannot turn the detector off: it sets its own
`GIT_TRACE` and prepends its own `PATH` entry.

**Order and verdict state.** The sequence is:

1. detector on, then the canaries;
2. `. "$_FIXTURES"` — sourced in the calling shell, like-for-like with the base build;
3. detector off;
4. record the result in `_fd_bad`/`_fd_dead` **as data**.

The verdict is written after `ctl_ok=0` (`controls.sh:738` at base), so the initialiser cannot clobber
it. The full order: detector verdict, then P-a/P-b/P-d/P-e, then the controls, then P-c, then
`[ "$ctl_ok" -eq 0 ] || exit 1`.

**errexit.** The wire runs `set -euo pipefail` (wire:311). The canaries' deliberately failing commands run
as `( … ) >/dev/null 2>&1 || true`. The detector itself does not change how the build runs: it runs once,
in the same shell, exactly as at base. Its only failing commands are the bypasses it exists to catch,
and those already sit inside the fixtures' `&& … || _fixture_failed` chains.

**Cost.** There is no second build. Measured base vs head in companion §A.6: on a loaded machine the K2 wire alone overlapped run to run, in both wall and CPU time.

**What escapes it** (corpus RES rows): a fixture call that builds its own environment from nothing,
either `env -i PATH=<literal> git` or `env -i /abs/git`. Such a call inherits nothing, so no trace
variable and no `PATH` shim reach it. It reads no caller input, so P with respect to the **caller**
holds. What it does read is the machine's system config and default template. §7 routes this.

## §5 Postconditions and residuals

### §5.1 Postconditions

These are always on, and run once per run in probe repos built by `_fgit`. Each producer has its own
label.

| id | label | assertion | liveness (own label) |
|---|---|---|---|
| P-a | `the fixture git reads configuration only from the fixture's own config file` | every `_fgit config --list --show-scope --show-origin` line is `local<TAB>file:.git/config<TAB>…`. Any other scope or origin is red, which covers a future scope and a local `include.path` | `this git reports a non-local configuration scope`: `-c a.b=c` must show as scope `command` |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero with no output. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` appended, both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is empty. Runs **last**, after the final `_fgit` writer (the P probes) | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `_fgit init` against `.git` from `_fgit init --template="$_FGIT_VOID"` is empty. This compares **content**: a template holding only `config`+`HEAD` leaves the names equal (companion §A.3) | — (discriminates where the default template is non-empty) |
| P-e | `the fixture git is the git the wire reads with` | `_fgit --exec-path` equals `git --exec-path` | — (discriminates only where `env`'s default-path git differs) |

### §5.2 Residuals, and the slots

| # | residual | direction | disposition |
|---|---|---|---|
| R1 | which git executable runs: `PATH`, and a shim git that fails under `env -i` | loud | outside P (§0.1). `PATH` → `#11-trip-wire-launch-environment`; the shim half is this slice's, stated at `_fgit` |
| R2 | filesystem-derived config written by `init` (`core.ignorecase`/`precomposeunicode`/`symlinks`/`filemode`), FIFO, permissions, raw names | either | outside P (§0.1). P-d's reference `init` shares the filesystem, so these cancel |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any | **declared blind spot**, not owed work: none is known at 2.55, and it cannot be planted or enumerated from inside. Stated at `_fgit` |
| R4 | the **launch-environment class**: anything injected into the shell that runs the build (`BASH_FUNC_*%%` replacing `printf`, `BASH_ENV`, `SHELLOPTS`, a function named `command`, non-git tools on `PATH`) | any | `#11-trip-wire-launch-environment` (a class, unchanged) |
| R5 | `$SCRATCH` owned by another UID | loud | outside P (§0.1) |
| R6 | reads through `_git` keep the caller's config | per contract | `_git`, `cfgkept`, fsmonitor |
| R7 | Windows git-bash is not in the trip-wires matrix (`sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep runs-on` → `ubuntu-latest`) | unmeasured | declared |
| R8 | the compiled-in ref backend. Under a reftable default, P-d reds, because two `init --ref-format=reftable` runs differ under `diff -r`, and `badref` writes `.git/refs/heads/` directly | loud | declared |

Draft 3's `#11-k2-fgit-machine-files` is **withdrawn at create time**. Its half (a) is R3, a declared
blind spot. Its half (b), a fixture spelling that bypasses `_fgit`, is the invocation-convention slot's
own gap (§7). Own new deferrals in this PR: **0**.

## §6 The corpus — the evidence, and the source of the records

**Command.** Everything lives under the session scratchpad
`…/scratchpad/fgit/`. The prototype `p4` is a `git clone --local` of `e8f78896` carrying exactly §3–§5.

```sh
python3 corpus/gen.py p4 corpus                       # 780 cells, 195 per config
tr '\n' '\0' < corpus/jobs.tsv | xargs -0 -n1 -P7 corpus/cell.sh > corpus/results.tsv
```

Each cell copies the wire's parts under a unique tag beside the originals, the way the mutation runner
does, applies one edit, and runs the **whole wire**. Two things decide the verdict:

- the exit status;
- the **named label** (R, P) or a clean `PASSED` (G).

A crash for an unrelated reason fails the cell. It is not counted as a kill: rc 126/127 is a harness
error.

Configurations: bash 5.3 or 3.2 × git 2.55 (Homebrew) or 2.54 (Apple). The git is chosen by `PATH`
order.

**The cells.**

- **G, must-green** (29 caller environments, unmutated wire): clean; every row of companion §A.1's
  table; `GIT_CONFIG_GLOBAL=/dev/null`; `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM`=<file with
  `init.defaultBranch`, `core.excludesFile`, `core.hooksPath`, `core.attributesFile`+`filter.*.clean`,
  `commit.gpgSign`+`gpg.program=/usr/bin/false`>; `GIT_CONFIG_COUNT` with each of those; `GIT_CONFIG_PARAMETERS`;
  HOME `.gitconfig` with each; XDG ignore/attributes/config/templateDir; `GIT_TEMPLATE_DIR` with
  exclude/config/HEAD/hooks; `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE`.
- **R, must-red** (13 call-site classes × 10 spellings). The classes are taken from the fixtures file:
  `init`, `add -A`, `add <path>`, `add -f`, `commit`, `checkout`, `hash-object`, `hash-object -w`,
  `hash-object -w --stdin`, `update-index`, `replace`, `rev-parse`, `symbolic-ref`. For each class, one
  site is chosen by regex in `gen.py`. The spellings are:
  - bare `git`
  - `_git`
  - `command git`
  - `env git`
  - `env -i PATH="$PATH" git`
  - `PATH="$CTL/fakegit:$PATH" git`
  - `hash -p <abs> git && git`
  - `eval "$_REAL_GIT"`
  - absolute path
  - `git -c core.excludesFile=/dev/null`

  Each cell expects `CONTROL FAILED (every fixture git call goes through _fgit)`.
- **RES, declared residual** (13 × 2): `env -i PATH=/usr/bin:/bin git` and `env -i <abs git>`. The
  expected result is green, and each such green is named, not silent.
- **P, pins** (10 harness mutants), each expecting its own label:
  - P-a: config added to `_FGIT_ENV`;
  - P-b: drop `NOSYSTEM`; drop `ATTR_NOSYSTEM`; drop `HOME`;
  - P-c: a planted void;
  - P-d: drop `TEMPLATE_DIR`;
  - P-e: drop `PATH`;
  - detector: `_fgit` without `-i`;
  - liveness: no `GIT_TRACE`; no `PATH` shim.

**Results** (`cut -f1-3 corpus/results.tsv | sort | uniq -c`, full table in companion §A.5):

| config | G must-green | R must-red | P pins | RES (declared) |
|---|---|---|---|---|
| bash 5.3 · git 2.55 | 29/29 | 130/130 | 10/10 | 26 green |
| bash 3.2 · git 2.55 | 29/29 | 130/130 | 10/10 | 26 green |
| bash 5.3 · git 2.54 (Apple) | 29/29 | 130/130 | 9/10 | 26 green |
| bash 3.2 · git 2.54 (Apple) | 29/29 | 130/130 | 9/10 | 26 green |

The one surviving pin is **P-e (drop `PATH`) under git 2.54**, as predicted: there, `PATH`'s git *is*
`env`'s default-path git (`/usr/bin/git`), so nothing differs. The record says so (C3). No cell crashed:
rc 126/127 is a harness error, and none occurred; `results.err` is empty.

The corpus ran on p4 `9df1e47e`. The label split for the two liveness halves (`e58fec48`) was re-run
on clean + all 10 pins × 4 configs, with the identical result (`corpus2/results.tsv`).

**These cells become the PR.** The R and P cells are the mutation records (C5). The corpus's own
check — the needle is the label, not merely red — is the mutation runner's rule "(c) print the named
control's own diagnostic".

The G cells become exit criterion X2. The RES cells are named in the records file as the declared
residual, so a future green there is expected, not missed.

**The ratchet (D2), measured on p4.** At base, `_mut_correspondence` takes its unrecorded-label
population from `_control` calls only. Widening it to `_control` ∪ `_lbl="…"`, and counting against p4's
record set, gives **30 bare labels**: the base's 21 plus 9 new ones. Every existing `_lbl` label already
has a record (`/usr/bin/grep -cF "<TAB><label>"` → 1 each).

Seven of the new labels get records from the P cells: P-a, P-b, P-c, P-d, P-e, the detector, and the
detector's liveness. That leaves two unrecorded: `this git reports a non-local configuration scope` and
`this git names its system files through git var`. They are **version guards** with no harness arm to
mutate. So `_MUT_UNRECORDED_MAX` is raised 21 → 23 **visibly, with this reason at the value**, and
`_MUT_RECORDS_MIN` rises by exactly the records added. X11 is the negative control.

## §7 `_fgit` vs `_git`, and the carve slots

`_git` preserves the caller's configuration (#501 R97). `_fgit` constructs its own environment. They
share no policy, and `_fgit` does not call `_git`. A fixture that calls `_git` is caught by the detector
(`_git` runs `exec git` through `PATH`; corpus R rows).

- **`#11-k2-fgit-keepset-depends-on-git-purge-glob`: DISSOLVES.** Its defect, a keep-set passed through
  `_git`'s purge, no longer exists. Its premise "one sentence … (taken in this PR)" was absent at
  `ff6b99a3`: `git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i 'exempt\|held back\|purge'`
  returns rc 1. The `_git_exec` closure the slot named is not built, and it is not needed.
- **`#11-k2-fixture-git-invocation-convention`: NARROWED, not closed.** The corpus closes it for every
  spelling that inherits the build's environment or resolves through `PATH` (R rows). It stays open for
  exactly the two RES spellings.
  - **Gap**: a fixture call that builds its own empty environment (`env -i PATH=<literal> git`, or
    `env -i <abs git>`) reads the machine's system config and default template, and nothing reds.
  - **Why it stays open**: no environment or `PATH` channel reaches such a call. Closing it means running
    P-a/P-d over every fixture repo, which is new mechanism, or an OS sandbox.
  - **Trigger**: any `env -i` or absolute git path added to the fixtures file.
  - **Owner**: citation-hygiene lane.
  - **Re-eval**: 2026-11-30.
  - **Create-time audit**: this is a pre-existing slot, narrowed and not new, so it does not count
    against this PR's own cap.
  - Its premise "Measured 2026-09-26: the build region contains no bare `git ` outside comments" holds at
    `e8f78896`: `awk 'NR>=83 && NR<=733 && !/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -cE '(^|[^_A-Za-z$/-])git [a-z-]'`
    → `0`.

**Ledger step** (the memory file `project_open-defer-slots.md`, outside this diff):

- remove the dissolved slot;
- rewrite the invocation-convention slot to the narrowed gap above;
- fix the header's count and source.

Cite **#501's merge commit on `main`**. This PR's squash SHA on `webref-cite-audit-tool` becomes
unreachable once #501 squash-merges.

## §8 Splits, the parent memo, and memo references

### §8.1 Code splits

`wc -l .claude/tools/webref-generic-core-trip-wire*.sh` at `e8f78896` gives: wire 1259, controls 989,
mutations 783, harness 195. `ff6b99a3` broke CLAUDE.md's touch-time rule: `git show
c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l` → `1026`, with no prereq split.

| file | split | seam | ground |
|---|---|---|---|
| controls | **C1** | build vs assert. `…trip-wire.fixtures.sh` takes base controls lines 83–734 (from `for d in clean pin` up to the line before `ctl_ok=0`) and is **sourced**, which is what the detector wraps | this PR grows both halves. p4 measures the split with `wc -l` over its parts (companion §A.5) |
| mutations | **C2** | the header's two populations. The generated half moves to `…trip-wire.mutgen.sh` | `ff6b99a3` reached 983 lines with the infrastructure C3/C4 carry, and this PR adds records |
| harness | no | — | stays far below the threshold |
| wire | **not touched** | — | `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` → `0` |

What C1 and C2 bring with them:

- **One parts list feeds all three readers in `_mut_run`**: the `cp`, the trap's `rm -f`, and the
  stale-skip `case`. The base versions of these can be seen with `sed -n '/^_mut_run()/,/cp "\$_HARNESS"/p'
  …mutations.sh`.
- **The sibling guard lives only inside `_mut_run`.** In a mutant child, every sibling carries
  `.mutant.`, so an always-on guard would kill every mutant.
- **The entry contracts extend to the new files.**

### §8.2 The parent memo: a banner, no split (C0b)

C0a (splitting the parent) is **withdrawn**. Parent §5 is live substance: parent §7 criterion 4 says
"Met" because §5 is the answers, and parent §9 says "the substance is in §5 and §8". So there is no
record-versus-design seam. CLAUDE.md requires a split only "real cohesion seam があれば".

The touch that remains is a supersession banner in the parent's header. Its site list is **derived**, not
hand-counted:

```sh
awk '/^#{2,3} /{sec=$0} $0 ~ /trip-wire\.(controls|harness|mutations)\.sh|controls file|control harness|mutation set|_fgit|GIT_TEMPLATE_DIR|D6|\*\*Status\*\*/ {printf "%d\t%s\n", NR, substr(sec,1,40)}' \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md
```

Each hit is classified by one rule. It is **current-state** if, after C1/C2/C5, it is false in the
present tense. It is **provenance** if it describes a round, a revision or a measurement already made:
§5's item history, §8's withdrawn slot, §9, §10.x, §11.0, §11.3's discharged conditional, §11.4's MIN
quotes, §11.6, §11.7.

At `e8f78896` the current-state sites are **seven**:

1. the header's Status (L5);
2. §0's derivation command naming the controls file (L61);
3. §4's three file rows (L278–280);
4. §7 criterion 3, "enumerated in `…mutations.sh`" (L675);
5. §7 criterion 5, "All are now in §4's table" (L746–747);
6. §11.1's D6 row (L1220);
7. §11.4 P1's file list (L1282).

⚠ Draft 3 said four sites and round 3 said five; the rule gives seven. The banner names all seven, and
the body stays as it is.

### §8.3 Memo references in code

A command lists every memo reference at base:
`/usr/bin/grep -n 'plan memo\|memo §\|memo'"'"'s §' .claude/tools/webref-generic-core-trip-wire*.sh`.
Each hit is resolved by its subject against the two memos' section maps
(`/usr/bin/grep -n '^## \|^### '` on each).

| file:line | reference | means |
|---|---|---|
| wire:5 | qualified | A-i memo (`2026-07-citation-hygiene-Ai-spec-label-map.md`) §2, §12(3) |
| wire:158 | "the memo's §2" (the K2 predicate) | the A-i memo §2, K2 — the memo the header names |
| wire:192 | "A-i's memo §12(3)" | A-i memo |
| wire:199, 267 | "plan memo's §8" (`\|\| return 4`, `wc -l`, verdict-site slots) | **parent** §8, Defer slots |
| wire:307 | "A-i-wire plan memo §6" | parent §6 |
| wire:323, 403, 456, 685, 724, 1164 | "plan memo §11, D1/D5/D3", "§11.6", "§11, D2/D4" | **parent** §11 |
| controls:10, 273, 614, 906 | §8, §11 D10, §11.2, §11.1 | parent |
| mutations:15 | "the plan memo booked this" | parent (§10.5) |

So the wire's unqualified references mean the parent for 8 lines (199, 267, 323, 403, 456, 685, 724,
1164) and the A-i memo for 1 (158). Draft 3's convention ("the memo the header names") was therefore
wrong for 8 of them, and it is dropped.

- **This table is the record.** The wire is not touched (§8.1).
- In files this PR edits (controls, fixtures, harness, mutations, mutgen), references are qualified with
  the file name in C1/C2. That is comment-only, and X4 checks it.
- This memo is only ever cited by file name.

### §8.4 The umbrella

The umbrella now carries:

- a slice row A-i-wire-fgit;
- memo-table rows for this memo **and** its companion. Both exit preflight 1 by design; the companion has
  a `## §0 Spec coverage map` heading, because preflight hard-fails a tracked plan memo that has none;
- a note on the A-i-wire row that its parent pointers describe the wire as of #519, naming the file and
  sections for each pointer.

Landing updates the memo-table status cells (§9).

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, the companion, the umbrella rows | docs | plan-review round 4 |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (seven derived sites) | docs | the §8.2 command |
| C1 | controls → controls + fixtures (sourced); one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4 |
| C2 | mutations → mutations + mutgen | prereq split | X1, X3, X4 |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | records may target the harness (`_MUT_TARGETS="wire harness"`; `wire` is the default, **never stripped**, because `w` is a sed command) | infra, required | X3 |
| C5 | §3–§5 as in p4; the §6 R/P cells as records, with RES named; the ratchet (§6); the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

**Cost, and `ci.yml`.** The base job comment is an in-file rule: "a wire that adds fixture self-tests
re-derives this line in the same PR" (`git show e8f78896:.github/workflows/ci.yml | sed -n
'/^  trip-wires:/,/^  [a-z]/p'`). C5 re-derives it by that rule's own method (three runs per side) and
records the method and verdict, with no figures. If the derived value differs from `5`: **STOP and
escalate to the user**, because the budget half is unowned (parent §6; umbrella, Cross-lane
coordination). The prototype measurement is in companion §A.6.

**Land order:**

1. `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`.
2. User approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. `/external-converge`.
4. User approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella status cells.
7. Hand the #501 merge decision to the user.
8. After #501 lands, the §7 ledger step.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | `env -i` closes variables; four relocations close default files. A template-seeded local config is visible only to P-d (content), and an include only to P-a (origin) | §3, §5.1 |
| 2 | B×E | an empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 3 | A×D | the detector is invocation-based: any git that inherits the build's environment or resolves through `PATH` is caught, whatever the subcommand | §4; corpus R |
| 4 | C×D | the detector's two channels (`GIT_TRACE`, the `PATH` shim) are each proven live on the same instance, before the build | §4 |
| 5 | D | verdict data is recorded during the build and written after `ctl_ok=0`; P-c runs last | §4, §5.1 |
| 6 | D×E | the detector must be off before any `_control` runs, because controls re-invoke the wire under the caller's environment | §4 |
| 7 | E×D | one parts list feeds `cp`, the trap and the `case`; the sibling guard lives only in `_mut_run` | §8.1 |
| 8 | C×D | mutation targets are `wire` (the default, never stripped) and `harness`. `controls:` is unverified under GNU sed and is not used | §9 |
| 9 | D | one label per producer. Each has a record, except the two version guards, which get a declared, visible raise to the ratchet | §6 |
| 10 | C×D | the P-d and P-e pins are machine-dependent (corpus P rows per git), and each record says where | §6 |
| 11 | F | a single build; the ci.yml line is re-derived by its own rule, and a changed value means STOP | §9 |
| 12 | A×C | P covers git's inputs only; the executable, the platform and the non-git commands are outside it | §0.1, §5.2 |

## §11 Exit criteria (both shells; both gits where the corpus has a column)

| id | command | expected |
|---|---|---|
| X1 | `( $SH $W ) > $S/l 2>&1; echo rc=$?; /usr/bin/grep -c 'CONTROL NOT EXERCISED\|CONTROL FAILED' $S/l; /usr/bin/grep -c 'trip-wire PASSED' $S/l` | `rc=0`, `0`, `1` |
| X2 | the corpus G cells on the implementing head | all PASS in all four configurations |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W` → `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED'` | three hits. C1–C3 are byte-identical to base |
| X4 | C1/C2: X1's log at the parent and at the split commit, scratch paths normalised by one `sed`, then `diff` | empty |
| X5 | the corpus R, RES and P cells on the implementing head | R and P PASS (P-e/P-d as noted per git). RES green, named |
| X6 | P-a/P-d planting (template `config`+`HEAD`; local `include.path`) | red |
| X8 | the ci.yml rule's derivation (§9) | method and verdict recorded; STOP on change |
| X9 | the PR's `Layering trip-wires` job (ubuntu, GNU) | SUCCESS. This is GNU evidence for `env -i`/`GIT_TRACE`, not for the sed claim |
| X10 | `$SH -n` and `wc -l` over `ls .claude/tools/webref-generic-core-trip-wire*.sh` | clean; every part < 1000 |
| X11 | in a clone, add `_x_lbl="an unrecorded probe"` and an `echo` that uses it | red, and the ratchet lists it |

## §12 Questions for plan-review round 4

- **Q1.** Are C1/C2 acceptable as standalone commits inside this PR, or do they need a prereq PR? Draft
  answer: commits, per the parent's §11.7 row 7. Moving them is free.
- **Q2.** Is narrowing the invocation-convention slot to the two from-nothing spellings acceptable?
  Draft answer: yes. Closing them needs new mechanism, and they read no caller input.
- **Q3.** Is `GIT_TRACE` as a detector channel acceptable on the runner's git? Draft answer: yes, and X9
  measures it; the canary refuses if it is not live.
