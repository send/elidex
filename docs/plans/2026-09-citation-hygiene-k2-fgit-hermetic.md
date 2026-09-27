# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**.

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
measurements, corpus scripts, provenance of `ff6b99a3`'s commits, false premises, every
plan-review round's dispositions, and the round-7 terminator. This memo holds only the live
decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record (§8.2).

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).

**Decision**: user, 2026-09-27, option (a): rebuild.

**Status**: **draft 7.**

Draft 7 answers round 6, which was a **full five-axis review** of draft 6 (`8b6a4005`). It found 0 CRIT
/ 5 IMP / 18 MIN, and no finding moved the window mechanism. Two of the IMPs were mechanism-adjacent,
and both were fixed on prototype **p7**, which is p6 plus those fixes:
- item 1: an incomplete window is now reported alone;
- item 2: the residual is re-stated by property, plus a seed check.

Every claim about p7 below is a cell result (§6, companion §A.10). What happens next is recorded in
companion §D.0: a **Step-4.5 focused check** of items 1 and 2 only, by Ax2 and Ax3. If it converges,
plan-review closes and implementation starts.

**History.** The round-5 focused review failed its terminator. The root cause was that
drafts 4–5's bypass *detector* was itself a name list: a git that neutralised the watched names still
read the caller's HOME/XDG (companion §A.9, before-cells).

The design therefore changed (orchestrating session, 2026-09-27). P now holds **by construction of
the whole fixture-build window**. The detector, its four channels, its canaries, its nine labels and
its records are **deleted**. Every claim below is a result from the recast corpus run on prototype
**p6** (§6).

(Draft 6 called round 6 "focused". It ran as a full five-axis review; companion §D.0 records it as it
ran.)

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
fails **by design**. The marker is A-ii's §4.2.5 feature, which has not landed yet. The marker line satisfies
A-ii's recogniser, which is line-anchored, fence-aware and scoped to the §3 section. Its line pattern
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
| (iii) **the whole build window from nothing**: `env -i` plus an allowlist around one child shell that sources the fixtures file | **closed for every git in the window that does not itself add an input**, however it is spelled, because each inherits only the window's environment. A git that adds an input (§5.1) is not closed | **chosen**; corpus §6 |
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
- plain `name=%q` assignments. These cover the data the fixtures file reads (`CTL`, the five
  `CONTROL_*` samples, `_REAL_GIT`, `_REAL_GREP`, `_fifo_ok`) and the window's own `_FGIT_VOID`,
  `_FGIT_ENVBIN`, `_FGIT_ENV_NAMES`, `_FGIT_WIRE_EXEC` and `_FW_DIR`. The fixtures' part of the list is
  **derived** by the census command below;
- `declare -f` bodies of `_fixture_failed`, `_shq` and `_fgit_postconditions`.

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
`$SHELLOPTS` after the prelude, and it exits 3 otherwise. The W record "prelude without
`set -euo pipefail`" goes red with W alone on both shells (companion §A.10).

Assignments are **plain**, never `declare -p`. `declare -p` carries the `export` attribute into the
window; P-f caught that in p6's first run (companion §A.9).

**What comes back.** Three things return through files: `_FIX_FAILED`, the fact of completion
(`done`), and whether any postcondition reported. The window's own `CONTROL …` lines reach the wire's
stderr directly.

**An incomplete window is not "no fixture failed" (round-6 item 1).** Suppose the child exits before
writing `done`, whether from an abort, a refused prelude, or the fixtures file's own `exit 2`
("decided nothing"). Nothing was built, so **no `_control` runs**.
`_fgit_window_incomplete_exit` prints the W verdict alone and ends the run with the child's exit class:
- 2 stays 2;
- anything else becomes 1.

Before this fix, the parent read the missing file as an empty `_FIX_FAILED`. Every control then ran
over unbuilt trees and blamed wire arms: pre-creating `$CTL/odd/pipe` gave rc 1 with 82 control lines
on p6, against rc 2 with 0 at base. With the fix it gives rc 2 with exactly one line, on both shells
(companion §A.10).

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
| `LC_ALL=C` | keeps the window's locale the one the wire exports (wire:319), so the build is like-for-like with base |

```sh
find -L /opt/homebrew/opt/git/share/man -type f -exec /usr/bin/grep -l ATTR_NOSYSTEM {} +; echo "rc=$?"   # rc=1 (207 pages)
```

**The fixture change.** `notcommitted` gains `mkdir -p .git/info`, because the empty template no
longer creates it (companion §A.2).

**errexit and state.** Every window state name (`_fw_rc`, `_fw_done`, `_fw_post_bad`,
`_fx_seed_hits`) is assigned at harness top level, before anything reads it. The child runs under
`set -euo pipefail` with **no EXIT trap**, and those options are checked, as above. So an abort there
is a non-zero exit, which the parent reads as data. The verdict is written after `ctl_ok=0`
(`controls.sh:738` at base), and an incomplete window ends the run right there.

**Nothing in the parent relies on `set -u` to fail.** The child's `set -u` is a backstop behind the
static census, and it is pinned by its own record. See §5.2 for the pre-existing defect.

## §4 Postconditions — run INSIDE the window

Any git in the window that **does not itself add an input** inherits the same environment. So git's
own answers inside the window describe every such git (§5.1 covers the ones that do add an input). Each producer has its own label (defined in the controls file and passed
in), and each label has its own record (§6).

| id | label | assertion (inside the window) | liveness (its own label) |
|---|---|---|---|
| W | `the fixture build window completed` | the child wrote `done`. Otherwise NE with its exit code, **reported alone**; the run ends with the child's exit class (2 stays 2) and no control runs | — |
| P-a | `a window git that adds no input of its own reads configuration only from its repo's config file` | every `git config --list --show-scope --show-origin` line in **one probe repo** is `local<TAB>file:.git/config<TAB>…` | `this git reports a non-local configuration scope`: `-c a.b=c` must show as scope `command` |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM`/`GIT_ATTR_SYSTEM` exit non-zero, empty. `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` exit 0, with every line under `$_FGIT_VOID/` | `this git names its system files through git var`: with `…NOSYSTEM=0` both names print a path |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is empty. Runs **last** in the window | — |
| P-d | `the fixture git copies no template` | `diff -r` of `.git` from `git init` against `.git` from `git init --template="$_FGIT_VOID"` is empty | — |
| P-e | `the fixture git is the git the wire reads with` | the window's `git --exec-path` equals the parent's, which is captured at source time | — |
| P-f | `the fixture build window's environment holds only its allowlist` | the name of **every** `env -0` record (the text before the first `=`, so non-identifier names such as `BASH_FUNC_f%%` are included) is an allowlist name or one bash maintains (`PWD OLDPWD SHLVL _`). **An unknown name is red**, which is the fail-safe direction. p6's `sed` parser skipped non-identifier names; p7 parses every record (companion §A.10) | — |
| S | `the fixtures file gives no git an input from outside the window (a seed)` | a **seed** over the fixtures file's text (§5.1): no non-comment line matches the input-adding pattern, except the one allowed form `git -c user.name=w -c user.email=w@e commit` | — |

## §5 Residuals, the slot, and a pre-existing defect

### §5.1 What the window does not close — stated truthfully

**The gap, by property (round-6 item 2).** A fixtures-file command can give a git process it starts
**an input that was not produced inside the window**. It can do this in three ways:
- **remove** an allowlist entry: `env -i … git`, `env -`, `env -u GIT_CONFIG_NOSYSTEM git`, `unset NAME`;
- **override** one: `GIT_CONFIG_NOSYSTEM=0 git`, `HOME=… git`;
- **add** one without touching the allowlist at all:
  - `GIT_CONFIG_GLOBAL=<file> git …`;
  - `git -c include.path=<file> …`, `git -c core.excludesFile=…`, `--config-env`;
  - `git init --template=<dir>` or `GIT_TEMPLATE_DIR=<dir> git init`;
  - `git config` writing a persisted key that names an outside path.

The outside files are reachable from inside the window. The Homebrew system file and template are
**caller-writable**: `/opt/homebrew/etc/gitconfig` is owned by `kazuaki admin`, and so is the templates
directory, measured with `ls -l` / `stat -f '%Su'`. The caller's real home is computable too, with
`eval "h=~$(id -un)"`.

**Measured.** On p6, `GIT_CONFIG_GLOBAL=<file> git add -A` and `git -c include.path=<file> add -A` at
the `add -A` site are **silently wrong**: rc 0, `PASSED`, and P differs from the clean build by 50 dump
lines, on both shells and both gits. The template spelling is loud on p6: rc 1 with NE, because the
excluded fixtures cannot be exercised. See companion §A.10, BEFORE cells.

**Closures weighed.** Each candidate was measured or argued before deciding.

| candidate | covers | decision and reason |
|---|---|---|
| run P-a over **every** fixture repo, not one probe repo | a **persisted** `include.path` (origin ≠ `.git/config`) | **declined**. It misses a persisted `core.excludesFile` (origin is `.git/config`) and every per-command form. Writing the persisted form needs `git config`, which the seed below already flags. It would cost one extra git exec per fixture repo per run |
| **a seed over the fixtures file's text**. The pattern is `GIT_*=`, `HOME=`, `XDG_*=`, `--config-env`, `--template`, `git -c `, `git config`, and `env`/`unset`/`export`/`eval`/`exec` as words, on non-comment lines. The one allowed form is `git -c user.name=w -c user.email=w@e commit` | every **literal** spelling of remove, override or add, whether or not it names an allowlist entry | **adopted** as check S (§4). Its direction is fail-safe: any new occurrence goes red until someone deliberately edits the allowed form. It is a **seed**: a spelling computed at run time (a variable holding `-c`, an `eval` of a built string that avoids the words) passes it. At p7 the census is exactly the 7 lines of the allowed form; the command is below. p7 makes all three add-spellings red with S on both shells (companion §A.10) |

```sh
awk '!/^[ \t]*#/ {print NR": "$0}' .claude/tools/webref-generic-core-trip-wire.fixtures.sh \
  | /usr/bin/grep -E '(^|[^A-Za-z_])(GIT_[A-Z0-9_]+=|HOME=|XDG_[A-Z_]+=|--config-env|--template|git -c |git config|env( |$)|unset |export |eval |exec )'
# p7: 7 lines, every one `git -c user.name=w -c user.email=w@e commit`
```

⚠ **What S costs.** Some R spellings that the window alone makes harmless now go red on S: `env git`,
`git -c core.excludesFile=/dev/null`, `scrub3-*`, `hermetic-look`, `gitsweep`, `t2zero-countempty`,
and `eval "$_REAL_GIT"`. This is by design, because S flags spellings, not effects. The window's own
guarantee for those spellings is shown on p6, which has no S (§6).

**`#11-k2-fixture-git-invocation-convention`, re-derived by property.**
- **Gap:** a fixtures-file command that gives a git process an input not produced inside the window,
  whether by removing, overriding or adding one. That covers environment variables, `-c`,
  `--config-env`, `--template`, and persisted config naming outside paths.
- **Why it stays open:** the window cannot constrain what a fixture does deliberately. S catches
  literal spellings only, and a computed spelling passes S and P-a alike.
- **Trigger:** S going red, or any fixtures-file change that constructs a git argument or environment
  at run time. Honestly, this is **reviewer attention plus a seed grep**, not a proof.
- **Owner:** citation-hygiene lane. **Re-eval:** 2026-11-30.
- **Create-time accounting:** the gap is **new**, since the allowlist names did not exist before. So
  this is **1 own deferral**, filed under a re-scoped existing name. Together with the dissolved slot
  below, the ledger nets to −1. At landing the Source/PR field is re-attributed to this PR.
- **The original premise**, verbatim from the ledger: "Measured 2026-09-26: the build region contains
  no bare `git ` outside comments". It is **moot**: bare `git` is now the canonical spelling, and the
  window makes it safe.

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
| R1 | which git executable runs (`PATH`; a wrapper git that needs other variables fails in the window) | loud | outside P (§0.1). `PATH` → `#11-trip-wire-launch-environment` |
| R2 | filesystem-derived config written by `init`; FIFO support, permissions, raw names | either | outside P. P-d's reference `init` shares the filesystem |
| R3 | a compiled-in path that no variable governs and `git var` does not report | silent, if any | declared blind spot. None is known at 2.55 |
| R4 | the **launch-environment class**: whatever the caller injects into the wire's own bash at startup (`BASH_ENV`, `SHELLOPTS`, `BASH_FUNC_*%%`, a function named `command`). The window's `env -i` drops these from the child's environment, but the parent that writes the prelude has already run under them. The prelude carries only the listed data and three function bodies | any | `#11-trip-wire-launch-environment` |
| R5 | `$SCRATCH` owned by another UID | loud | outside P |
| R6 | reads through `_git` keep the caller's config, including a caller `GIT_TRACE=1`, which reds at base too (parent D7) | loud, pre-existing | `_git`'s contract |
| R7 | Windows git-bash is not in the trip-wires matrix (command below) | unmeasured | declared |
| R8 | a compiled-in reftable default: `badref` writes `.git/refs/heads/`, and P-d's `diff -r` differs between two reftable inits | loud | declared |

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

This PR's harness and controls are exactly such code. The ledger text should say this, and it should
replace the entry's stale reference to the deleted detector ("U5 … setup failure a labelled verdict"):
- nothing in this PR's parent-side code relies on `set -u` to fail, because every state name is
  assigned before it is read;
- an incomplete fixture-build window, including a child that refuses a prelude without
  `set -euo pipefail`, is reported by the **W verdict alone** and ends the run with the child's exit
  class;
- that is pinned by two W records: the child aborting, and the prelude without its options.

## §6 The corpus — evidence, and the source of the records

**Oracle, recast.** Under a hostile caller, a bypass spelling no longer has to go red. It has to go
**green with P identical to the clean build's P**. P is dumped per fixture repo by a **prototype-only**
hook (`K2_CORPUS_PDUMP`), which is not part of the design. It is passed into the window as a plain
assignment, so P-f does not see it. Symlink targets are normalised for `$CTL` (companion §A.9).

**Subjects.** Both are `git clone --local`s of `e8f78896`.
- **p6** carries §3–§4 as in draft 6. It has no S, so it shows the window's own guarantee.
- **p7** is p6 plus round 6's item-1 and item-2 fixes: W alone, the options pin, P-f parsing every
  record, and S. It was measured on the focused cells below.

**Subset and re-run recipe.** Scripts are in companion §E.3. Run them from a directory holding the
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

**Results** (`c6/eval6.sh`; full table in companion §A.9):

| config | G (11, P equal) | R (P equal) | RLOUD | RES (P equal) | P records (9) | INFO: HOME drop / PATH drop |
|---|---|---|---|---|---|---|
| bash 5.3 · git 2.55 | 11/11 | 27/27 | red | 2/2 | 9/9 | red / red |
| bash 3.2 · git 2.55 | 11/11 | 11/11 | red | 2/2 | 9/9 | red / red |
| bash 5.3 · git 2.54 | 11/11 | 11/11 | red | 2/2 | 9/9 | red / **equivalent** (as predicted) |
| bash 3.2 · git 2.54 | 11/11 | 27/27 | red | 2/2 | 9/9 | red / **equivalent** |

The run covered p6 `863158b1` from 17:49 to 18:27 JST. `raw.err` is empty, and no cell hit a
harness error.

**The DO cell** is a caller `GIT_CONFIG_GLOBAL`=<`safe.directory=*`> with
`GIT_TEST_ASSUME_DIFFERENT_OWNER=1`. It is green with P equal on all four configurations, so the read
side still sees the caller's configuration.

**Round 5's Ax3 IMP-1**, under a HOME `*.py` ignore, for `scrub3-sh`, `hermetic-look`, `gitsweep` and
`t2zero-countempty`:
- **p5:** silently wrong, with P differing by 50 lines;
- **p6:** P identical.

The cells are in companion §A.9.

**Draft 7's focused cells**, on p6 (before) and p7 (after), on bash 5.3·git 2.55 and bash 3.2·git 2.54.
Scripts are in companion §E.4, and the full table is in companion §A.10.

| cell | p6 (before) | p7 (after) |
|---|---|---|
| `$CTL/odd/pipe` pre-created, so the fixtures `exit 2` | rc 1, **82** control lines (W plus 81 false arm failures) | **rc 2, exactly one line (W)** |
| W record: the child aborts after the prelude | — | rc 1, W alone |
| W record: prelude without `set -euo pipefail` | — (silent on p6) | rc 1, W alone |
| `GIT_CONFIG_GLOBAL=<file> git add -A` | rc 0, PASSED, P differs by 50 lines: **silently wrong** | red on S |
| `git -c include.path=<file> add -A` | **silently wrong** (50 lines) | red on S |
| `GIT_TEMPLATE_DIR=<dir> git init` (the loop) | rc 1, loud (NE) | red on S (plus the same NE) |
| P-f record plus a caller `BASH_FUNC_k2probe%%` | — | P-f lists `BASH_FUNC_k2probe%%`. p6's `sed` parser finds 0 such names; `env -0` finds 1 |
| G: HOME `*.py` ignore; DO cell | — | green, P equal |
| R: `gitsweep` × HOME ignore | green, P equal (§6 table above) | red on **S** (by design: S flags the spelling) |
| P-a record, with the scoped label | — | red with the new label on both configurations |

**Records: representative only.**

| label | record |
|---|---|
| W | the child exits right after the prelude |
| W | the prelude loses `set -euo pipefail`, so the options pin makes the child exit 3 |
| P-a | config appended to `_FGIT_ENV` |
| P-a liveness | the probe loses `-c a.b=c` |
| P-b | drop `GIT_CONFIG_NOSYSTEM` |
| P-b liveness | the probe loses `…NOSYSTEM=0` |
| P-c | plant a file in the void |
| P-d | drop `GIT_TEMPLATE_DIR` |
| P-e | inject `GIT_EXEC_PATH=/nonexistent-k2` (discriminates on both gits) |
| P-f | drop `-i` from the window's `env`, so it inherits |
| S | `GIT_CONFIG_GLOBAL=<file> git` at the fixtures file's `add -A` site (**`fixtures` target**) |

That is **10 labels and 11 records**. `_MUT_TARGETS="wire harness fixtures"`: S's record edits the
fixtures file. `fixtures:` is a BSD `sed` error (`sed 'fixtures:p' </dev/null` → "invalid command code
f"); the GNU behaviour is unmeasured. No `controls` target is used. **`_MUT_UNRECORDED_MAX`
stays at 21**, and `_MUT_RECORDS_MIN` rises by exactly 11. RES cells are not records, because
survival would need the `!survive` needle, which the runner requires exactly once
(`mutations.sh:658–662`). The two INFO cells are informative, not records: HOME drop reds P-b, and PATH
drop is equivalent on git 2.54.

**Mutation-mode cost (X3).** Each run of X3 is (95 base records + 11) record trials plus the generated
population, one control pass each. X3 prints the counts, and X8 gives the per-pass time. This is
opt-in and does not add to the always-run gate.

## §7 What was deleted (draft 5 → 6)

These were deleted, with details in companion §D.5:
- the four-channel detector: the `PATH` shims, trace2 over two routes, and the poison;
- its canaries, labels and records;
- the per-call `_fgit`;
- draft 5's slot scope, and its claim "reads no caller input".

Round 5's Ax2 restore IMP, Ax3 IMP-2 and Ax3 M1–M3 went away with it. Draft 7 restores only the
`fixtures` mutation target, for S.

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
| C0 | this memo, the companion, the umbrella rows | docs | the Step-4.5 focused check (items 1 and 2) |
| — | **plan-review gate** | | |
| C0b | parent: supersession banner (eleven sites) | docs | the §8.2 command, re-read |
| C1 | controls → controls + fixtures; one parts list; sibling guard in `_mut_run`; references qualified | prereq split | X1, X4, X4b |
| C2 | mutations → mutations + mutgen; references qualified | prereq split | X1, X3, X4, X4b |
| C3 | record comments | infra (§0.2) | X3 |
| C4 | `_MUT_TARGETS="wire harness fixtures"` with resolver and restore | infra, required | X3 |
| C5 | the window (§3), with the fixtures calling `git`; `notcommitted`'s `mkdir`; the incomplete-window exit; the options pin; §4's postconditions and S; the 11 records; the ratchet population; §8.1's in-file rewrites; the `ci.yml` line re-derived by its own rule | feature | X1–X3, X5, X6, X8, X11 |

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
- **Cost shape.** There is still **one** build. It runs in a child process, with the postconditions
  and S added.

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
- **Rewrite** `#11-k2-fixture-git-invocation-convention` to §5.1's gap, why, trigger, owner and
  re-eval.
  - Gap: "a fixtures-file command that gives a git process an input not produced inside the window —
    remove, override or add; env vars, `-c`, `--config-env`, `--template`, persisted config naming
    outside paths".
  - Trigger: "S reds, or a fixtures change constructs a git argument or environment at run time".
  - Source/PR: this PR.
  - Accounting: 1 own deferral under a re-scoped name.
- **Amend** `#11-k2-wire-exit-trap-masks-set-u-abort` with §5.2's text: no parent-side reliance on
  `set -u`; an incomplete window is reported by W alone with the child's exit class; pinned by the two
  W records.
- **Fix the header count.** The net change is −1: one slot dissolved, one re-scoped.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | the window's `env -i` closes every variable, and four relocations close default files. That holds for every git in the window **that does not itself add an input**, however it is spelled. Inputs a fixture adds are §5.1's gap, flagged by S when spelled literally | §3, §5.1; corpus G, R |
| 2 | A×D | the postconditions run in the window, so they describe every git there that adds no input | §4 |
| 3 | A×D | P-f is a complement check: an unknown name in the window is red | §4 |
| 4 | B×E | the empty template removes `.git/info/`, so `notcommitted` creates it | §3 |
| 5 | D | window state is assigned before it is read. An incomplete window ends the run with W alone and the child's exit class, and no control runs over unbuilt trees. The child's options are checked. Nothing in the parent relies on `set -u` | §3, §5.2 |
| 6 | D×E | the window is a child process, so the parent's environment, and with it the read side and every `_control`, is untouched | §0.1; DO cell |
| 7 | E | the prelude passes plain assignments, never `declare -p`, so no `export` attribute enters the window | §3 |
| 8 | E×D | producers live in the harness and labels in the controls file. Records target `harness`, plus `fixtures` for S | §4, §6 |
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
| X5 | the §6 R, RLOUD, RES and P cells, same recipe | R and RES green with P equal; RLOUD red; P PASS |
| X6 | P-a/P-d planting (a template `config`+`HEAD`; a local `include.path`) | red |
| X8 | the ci.yml rule's derivation (§9) | method and verdict recorded; STOP on change |
| X9 | `Layering trip-wires` on ubuntu (GNU), via route (a) or (b) of §9, chosen by the user at push time | SUCCESS. This is GNU evidence for `env -i`, `env -0`, the window and the prelude |
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

## §12 Questions for the Step-4.5 focused check (items 1 and 2 only)

- **Q1 (item 1, Ax2).** Does "an incomplete window is W alone, with the child's exit class" close the
  cascade in every way the child can stop? The ways are an abort, a refused prelude, and the fixtures'
  own `exit 2`. The cells cover the second and third, plus an explicit abort record.
- **Q2 (item 2, Ax3).** Is "remove, override or add an input" the right property for the gap? And is
  the balance right between S (adopted, a seed) and P-a over every repo (declined)?
