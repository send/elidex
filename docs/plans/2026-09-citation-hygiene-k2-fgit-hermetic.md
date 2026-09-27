# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit**, which this
draft registers there (§0.2).
**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`. It is #519's landed
record, and this slice does **not edit** it (§8.3).
**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896`. That base is #501's head `webref-cite-audit-tool`
with #519 squashed in (`gh pr view 501 --json headRefOid`).
**Replaces**: `k2-wire-fgit-scrub-complement` @ `ff6b99a3` (14 commits, never pushed).
**Decision**: user, 2026-09-27, option **(a)**: rebuild, and do not fall back to closing
`GIT_TEMPLATE_DIR` alone.
**Status**: **draft 2**, answering plan-review round 1 on draft 1 plus an uncommitted §2.5: 0 CRIT,
13 IMP (11 unique), 25 MIN. Nothing is implemented yet. Plan-review round 2 is owed, and it also covers
this draft's umbrella amendment (§0.2).

⚠ **This memo follows the parent's rule: no quantity that moves with a commit.** Every figure is one
of two things. Either it is a measurement at a **named SHA or a named scratch prototype**, given with
the command that produced it, or it is replaced by that command. `$S` is any scratch directory. Hostile
files are never planted in the real `HOME`. The memo writes `/usr/bin/grep` because the default `grep`
on this machine is ugrep, which can return a silent 0.

---

## §0 Why this exists

Codex raised a P2 on #501. `_fgit`, the helper the K2 wire's controls use to build their fixture repos,
did not neutralise `GIT_TEMPLATE_DIR`. A caller's template `info/exclude` therefore hid the fixtures'
own files from `git add -A`, and the controls reported `CONTROL NOT EXERCISED`.

That was the **second** failure of the list approach. #501 R97 had fixed `GIT_CONFIG_COUNT` by unsetting
three names. `ff6b99a3` then swept every `GIT_*` name and kept a keep-set
(`GIT_CONFIG_GLOBAL`/`_SYSTEM=/dev/null`), which it justified as "git's config layers".

That failed a **third** time, on `$XDG_CONFIG_HOME/git/ignore` / `$HOME/.config/git/ignore`. That file
is the default value of `core.excludesFile`. It is neither a `GIT_*` name nor a config layer (§2).

This memo grounds the mechanism on a **property** instead of a list (§1). It also chooses the direction
in which a channel nobody listed fails (§3.1).

### §0.1 The acceptance criterion changes, on purpose

The parent's §11.1 judged every environment input by one test: **"unreachable or loud, never silent"**.
Under that test, `GIT_TEMPLATE_DIR` (D6) counted as **loud** (a red run with `CONTROL NOT EXERCISED`) and
was accepted.

This slice **replaces that criterion for the fixture build with property P** (§1): the fixtures must not
depend on the caller **at all**. A loud failure caused by the caller now counts as a defect too.

The reason is that the loud failures §2 measures come from **ordinary developer configuration**, not
from contrived input. A `~/.config/git/ignore` holding `*.py` makes the gate red, with 11 or 12 controls
`NOT EXERCISED`. A `~/.config/git/attributes` makes it red with 17. In a **required** gate, a false red
from ordinary configuration means a valid checkout can never pass. Gates in that state get switched off,
not fixed.

The failures stay loud (§2: rc 1). That is exactly why the old criterion never objected, and why it was
the wrong criterion for the build.

The **read** side keeps the old criterion. `_git` preserves the caller's configuration on purpose (#501
R97), so fixture reads are unchanged (§7). The parent's §11.1 D6 row stays as #519's record. It is
superseded **here**, not rewritten (§8.3).

### §0.2 Why this is one slice, and where it is registered

CLAUDE.md's edge-dense rule applies: §10 lists at least three intersecting axes. The rule requires two
things: (a) the work is registered in the umbrella, and (b) it passes plan-review before any code. The
base case the rule accepts is a narrowly-scoped per-PR slice, under an approved umbrella, that has passed
plan-review. Such a slice is a terminal unit.

**Registration.** This draft adds a slice row **A-i-wire-fgit** to the umbrella's slice table and a row
to its memo table. It follows the A-i-wire row's pointer style: this memo is the one list, and the
umbrella row does not restate it. ⚠ The umbrella is a **ratified surface**, so this amendment is itself
under review in plan-review round 2.

**Why it is one slice.** The slice is exactly **property P for the fixture build**: the mechanism (§3),
the postconditions (§6.1), the probes (§6.3–§6.4), and the guard that fixtures are built **only** through
`_fgit` (§7, which is P's "only through" half and covers `_git` too). C1 and C2 are the standalone prereq
splits CLAUDE.md's touch-time rule requires. C3 and C4 are enabling infrastructure without which this
slice's own mutation records cannot be written.

**Squash merge.** Commit boundaries vanish when the PR is squashed, and that is acceptable. CLAUDE.md
admits a prereq split as "単独 PR **/ 単独 commit**". The parent's §11.7 row 7 is the precedent: #519
landed its harness split as a commit inside the PR. The boundaries matter for review and bisection
**before** the merge, and C1/C2 are shown to be behaviour-neutral at their own commits (X4).

## §1 The property

The controls assert over staged **content**: the violation string sits inside a blob. They also assert
over **commits**: `committed`, `orphan`, `headprobe`, `badref`, `replaced` and `lstreefail` all commit.
So a property about "the paths `git add -A` stages" is not enough.

Two channels change a fixture while leaving its path set unchanged:

| channel (under an otherwise empty git environment) | `victim.py` afterwards |
|---|---|
| none (baseline) | blob `7d4290a1…` |
| `$HOME/.config/git/attributes`: `*.py working-tree-encoding=UTF-16LE` | blob `b8b71b2a…`: **content re-encoded**, path unchanged |
| template `hooks/pre-commit` running `git rm -q --cached victim.py` | **absent from the index and from HEAD**; `commit` exits 0 |

(Measured with the §11 X6 probe on git 2.55.0 from Homebrew and Apple git 2.54.0.)

> **P.** For each fixture, two things are determined only by the fixture script's git commands and the
> working-tree files it writes:
>
> - the **index**: every entry's path, mode and blob id;
> - the **tree of `HEAD`**, where the fixture commits.
>
> Neither depends on any git input the caller carries. That means the process environment, files at
> default locations outside the fixture (home, XDG, system prefix, compiled-in template directory), and
> any configuration those name.

**Outside P, on purpose:**

- **Commit metadata.** Timestamps, reflog identity and the resulting SHAs vary from run to run, and no
  control reads them
- **The non-git commands of the build** (`printf`, `ln`, `mkfifo`, …). They run in the caller's shell,
  so a function exported into that shell changes what they write. The reviewer measured
  `BASH_FUNC_printf%%` replacing blob content, and no postcondition went red. That belongs to the
  **launch-environment class** (§5 R4); it is not a git channel
- **Reads at control time.** Those are `_git`'s (§0.1, §7)

P is exactly what §6.3 compares (§10 row 11).

## §2 What was measured at the two heads

**How each run was made.** Invocation:

```sh
( cd <checkout> && env <ASSIGNMENTS> <shell> .claude/tools/webref-generic-core-trip-wire.sh ) > log 2>&1
```

NE is `/usr/bin/grep -c 'CONTROL NOT EXERCISED' log` and CF is `/usr/bin/grep -c 'CONTROL FAILED' log`.
Checkouts are `git clone --local` at the SHA, checked with `git rev-parse HEAD`. GNU bash 5.3.20 and
`/bin/bash` 3.2.57 give identical results wherever both were run.

| caller environment (all files under `$S`) | `ff6b99a3` | `e8f78896` (base) |
|---|---|---|
| `HOME` with a `.gitconfig` setting `core.excludesFile` to a file holding `*.py` | rc 0, NE 0 | not run |
| `HOME` with an **empty** `.config/git/ignore` (isolating control) | rc 0, NE 0 | not run |
| `HOME` with `.config/git/ignore` = `*.py` | rc 1, **NE 12**, CF 0 | rc 1, NE 11, CF 0 (3.2) |
| `XDG_CONFIG_HOME=<dir>` with `git/ignore` = `*.py` | rc 1, **NE 11, CF 1** | not run |
| `XDG_CONFIG_HOME=<dir>` with `git/attributes` = `*.py working-tree-encoding=UTF-16LE` | rc 1, NE 17, CF 5 (3.2) | rc 1, NE 17, CF 4 (3.2) |
| `XDG_CONFIG_HOME=<dir>` with `git/config` setting `init.templateDir` | rc 0 (3.2) | not run |
| `GIT_TEMPLATE_DIR=<dir>` with `info/exclude` = `*.py` | rc 0 (lane SSoT) | rc 1, NE 11, CF 0 (3.2) |

**Why the HOME row and the XDG row differ.** Both rows fail the same 12 controls. The difference is how
one of them, `envscrub`, is reported.

- `ff6b99a3`'s liveness probe `_envlive` passes `"HOME=${HOME:-}"` into its own `env -i` (line 814: `git
  show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -n
  'HOME=\${HOME'`)
- When `HOME` is hostile, it contaminates the probe's **baseline** half, so `envscrub` is reported NE
- When `XDG_CONFIG_HOME` is hostile, the probe drops it and passes. `_fgit` does not drop it, so
  `envscrub` is reported CF

The only label that differs between the two logs is `a hostile caller git environment cannot change what
a fixture holds`. To reproduce, diff the `NOT EXERCISED (…)` labels of the two logs.

⚠ **The lane SSoT's "12" for the XDG row is inferred, not known.** It probably counted NE and CF
together, but that measurement's log was not recorded, so this is a guess about it.

**Three more findings from this table:**

- **Attributes are a fourth channel.** Neither head names it, and it rewrites blob content (§1)
- **The ignore channel was not opened by `ff6b99a3`.** Base is already red on it. Git reads the default
  ignore path whenever no layer it reads sets `core.excludesFile`. Nulling the global layer only
  guarantees that no layer sets it
- **Every failure measured was loud (rc 1), not silent.** That is why §0.1 changes the criterion rather
  than arguing from silence

## §2.5 Spec coverage map

**No spec surface** — this slice changes a shell helper, its controls and its mutation records. It
touches no WHATWG, W3C, TC39 or CSS WG behaviour.

The channel roster rests on two authorities. The first is git's own documentation at the version
measured, which webref does not index:

- `git help gitignore`
- `git help git-config`, FILES
- `git help gitattributes`
- `git help git-init`, TEMPLATE DIRECTORY
- `git help git`, ENVIRONMENT VARIABLES
- `git help git-var`

The second is the behaviour measured in §2 and §6.

**Why `preflight.py` fails here, by design.** This section follows the shape used by slice A-iii and by
the parent's §0.5/§3: the heading is kept and **no table** follows it. A placeholder row would parse
green without verifying anything. The declaration marker belongs to A-ii's §4.2.5, and A-ii has not
landed.

The marker line matches A-ii's recogniser `^ {0,3}\*\*No spec surface\*\*`, with the bold closed before
the dash. That follows A-iii's Codex R13 precedent.

```sh
/usr/bin/grep -nE '^ {0,3}\*\*No spec surface\*\*' docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md   # one hit
python3 .claude/skills/elidex-plan-review/preflight.py \
  docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md; echo "rc=$?"   # rc=1, "no markdown table follows it"
```

## §3 Mechanism

### §3.1 The choice, argued from which way an unlisted channel fails

| shape | a channel nobody listed… | verdict |
|---|---|---|
| (i) **unset a list of names** (base) | **stays open** and reaches the fixture build. Measured outcome: a false red (§2) | failed twice |
| (ii) **sweep `GIT_*` + keep-set** (`ff6b99a3`) | stays open if it is **not a `GIT_*` name** or **not a config layer**: `HOME`, `XDG_CONFIG_HOME`, default files, a template-seeded `config` | failed a third time (§2) |
| (iii) **construct the environment from nothing** (`env -i` + an allowlist), with every default location pointed at a known-empty place | **closed**, for every environment variable whether or not anyone listed it. File channels at default locations close through the few allowlist entries that relocate or disable them. A **postcondition asks git itself** what it would read (§6.1) | **chosen** |
| (iv) OS sandbox (scratch user, `sandbox-exec`, container) | closed, including compiled-in paths | no portable mechanism across macOS and the ubuntu runner; needs privileges this gate does not have |

**What the documentation supports.** The `git help git` entry for `GIT_CONFIG_NOSYSTEM` (ENVIRONMENT
VARIABLES) says it *"can be used along with $HOME and $XDG_CONFIG_HOME to create a predictable
environment for a picky script"*. That endorses the **ingredients**, not the exact shape of (iii), which
leaves `XDG_CONFIG_HOME` **unset**. What makes "unset" safe is documented separately:

- `git help git-config`, FILES: *"When the XDG_CONFIG_HOME environment variable is not set or empty,
  $HOME/.config/ is used"*
- `git help gitignore` and `git help gitattributes` say the same for the `ignore` and `attributes`
  defaults

Draft 1 also claimed that "git's test suite builds its environment the same way". That claim is
unsourced here and is withdrawn.

**What (iii) changes is the default.**

- Under (i) and (ii), a channel is open until somebody lists it
- Under (iii), a channel is closed until somebody allowlists it
- Every allowlist entry is pinned by a record that goes red without it (§6.2)

⚠ **(iii) does not, by itself, close files at compiled-in paths.** An empty environment still lets git
read `$(prefix)/etc/gitconfig`, `$(prefix)/etc/gitattributes` and the compiled-in template directory.
Each of these has exactly one git-provided switch or relocation, and those switches are the allowlist.
§5 declares the channels that have no such switch. §6.1 is what makes a switch that stops working
visible.

### §3.2 The allowlist

Each row gives the entry, why it is there, and what happens without it. All effects were measured on
git 2.55.0 and on Apple git 2.54.0. They are also §6.2's measured predictions.

| entry | why it is in | without it |
|---|---|---|
| `PATH=$PATH`, with the value **captured at harness-source time** | The fixtures must be built by **the same git the wire reads them with**. The wire resolves `git` through `PATH` (the parent's D12 row) | BSD `env -i` falls back to a default path and runs **`/usr/bin/git` (Apple 2.54)** instead of `PATH`'s 2.55. Compare `env -i HOME=$S/v git --version` with `env -i PATH="$PATH" HOME=$S/v git --version` |
| `HOME=$VOID` | Relocates **every** HOME-relative default: `~/.gitconfig` and `~/.config/git/{config,ignore,attributes}`. `XDG_CONFIG_HOME` stays unset and so falls back into `$VOID` (the documented fallback in §3.1) | Leaving `HOME` unset also closes it on 2.55: `git var GIT_CONFIG_GLOBAL` exits 1. The void is still chosen, for two reasons. First, any HOME-relative default added in a later git lands in a directory this harness owns and asserts empty (P-c). Second, "unset" relies on git never falling back to the passwd home, which cannot be tested without planting files in the real home |
| `GIT_CONFIG_NOSYSTEM=1` | Turns off the system config layer, which is live on this machine: `git var -l` under `env -i` prints `credential.helper=osxkeychain`, and Apple git also prints `init.defaultbranch=main` | `git var GIT_CONFIG_SYSTEM` prints a path instead of nothing |
| `GIT_ATTR_NOSYSTEM=1` | Turns off the system attributes file | `git var GIT_ATTR_SYSTEM` prints a path. ⚠ The variable is **undocumented at 2.55**: `for p in git gitattributes git-config gitignore; do git help -m $p \| col -b \| /usr/bin/grep -c GIT_ATTR_NOSYSTEM; done` prints `0 0 0 0`. Git still honours it, and `git help git-var` documents `GIT_ATTR_SYSTEM` as the path *"if one is enabled"*. So the entry is pinned by git's own answer (P-b), not by trusting the name |
| `GIT_TEMPLATE_DIR=$VOID` | Fixes the template directory, whichever of its four sources would otherwise supply it (`git help git-init`, TEMPLATE DIRECTORY). A template can seed `info/exclude`, `info/attributes`, `hooks/*`, **`config`** and **`HEAD`**. A template holding only `config` and `HEAD` hid `victim.py` and set `HEAD` to `ref: refs/heads/hostile` | The compiled-in default is copied in. On both gits here it contains `description`, `hooks/*.sample` and `info/exclude` |

**About `$VOID`.** It is one fresh directory under the wire's own `$SCRATCH`, which is already checked by
`mktemp -d` and cleaned up by a trap. It starts empty, and P-c asserts that it is still empty after the
build. There is one "nothing", and one assertion that it stayed nothing.

An empty string, `GIT_TEMPLATE_DIR=`, also copies nothing on both gits here, but that behaviour is
undocumented. An empty directory is documented: *"files … in the template directory … will be copied"*.

**`_FGIT_ENV` is evaluated once, at harness-source time (F10 decided).** It is expanded when the harness
is sourced, never per call. The environment `_fgit` constructs must not depend on the caller's state at
call time; a snapshot of constants is the whole point. §6.2 therefore predicts, and the prototype
measures, two different mutants for `HOME`:

- capturing the real `HOME` at **source time** reds P-b only
- passing `HOME` through at **call time** reds P-b and the H1–H4 probes

**Not on the allowlist:**

- `XDG_CONFIG_HOME`: the `HOME` fallback already covers it
- `LANG` and `LC_*`: they change messages, not what gets staged
- Commit identity: supplied per call with `-c user.name/email`. Reflog identity falls back to passwd and
  only affects SHAs, which are outside P
- `_git`'s read switches: `GIT_NO_LAZY_FETCH`, `GIT_NO_REPLACE_OBJECTS`, `-c core.fsmonitor=false` and
  `-c core.untrackedCache=false`. A build has no remote. Replacement does not affect `add` or `commit`.
  fsmonitor and the untracked cache are configuration, which P-a and P-d exclude. Carrying them anyway
  would make `_fgit` depend on `_git` again (§7)

### §3.3 Shape

```sh
# in the harness — spelled once, expanded once
_FGIT_VOID="$SCRATCH/fgit-void"          # mkdir, checked; empty by construction
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID")
_fgit() { command env -i "${_FGIT_ENV[@]}" git "$@"; }
```

- It uses `command env`, so a shell function named `env` cannot intercept the call
- No subshell is needed, because nothing in the shell's own state changes
- `_fgit` **no longer calls `_git`**

**Prototype A: the wire.** Setup: a `git clone --local` of `e8f78896`, with this change and one fixture
fix (`mkdir -p .git/info` in `notcommitted`). It gave **rc 0, NE 0, CF 0, `PASSED`** on both shells under each of these caller environments: a clean
caller; `HOME/.gitconfig` setting `core.excludesFile`; `HOME/.config/git/ignore`; an `XDG_CONFIG_HOME`
ignore file; an `XDG_CONFIG_HOME` attributes file; a `GIT_TEMPLATE_DIR` exclude; the `GIT_CONFIG_COUNT`
trio; `GIT_CONFIG_SYSTEM=<hostile>`; `GIT_DIR=<other repo>`.

**Without the fixture fix**, even a clean caller gives rc 1 with NE 1 (`per-clone info/exclude cannot
hide an entry`). That fixture wrote into a `.git/info/` directory that only the **ambient template** had
been creating (§10 row 2).

**Prototype B: the checks.** A standalone script implementing §6.1's P-a (with origin), P-b, P-d (by
content) and P-e, plus §6.3's three-way probe over roster rows E1–E8 and H1–H4. It was run unmutated and
once per §6.2 row, and its results are §6.2's table. It is re-runnable as §11 X5/X6.

## §4 The channel roster: a seed that checks the mechanism, not the mechanism

⚠ **This table does not close anything.** §3 closes channels by construction, and a roster can never be
complete. Its job is to supply **hostile inputs** to the §6.3 probes, so that every allowlist entry has
something that goes red when the entry is removed. Two consequences:

- A channel missing from this table is still closed, provided §3 holds
- A channel that §3 fails to close is caught by §6.1's property-shaped postconditions, if it carries
  configuration or template content

"Live" means that adding the channel **inside** the constructed environment changes the §1 probe's
result in prototype B.

| # | channel | class | closed by | live? |
|---|---|---|---|---|
| E1 | `GIT_TEMPLATE_DIR` (template `info/exclude`) | env | `env -i` | yes |
| E2 | `GIT_CONFIG_COUNT`/`_KEY_0`/`_VALUE_0` (`core.excludesFile`) | env | `env -i` | yes |
| E3 | `GIT_CONFIG_PARAMETERS` | env | `env -i` | yes |
| E4 | `GIT_CONFIG_GLOBAL=<hostile file>` | env | `env -i` | yes |
| E5 | `GIT_CONFIG_NOSYSTEM=0 GIT_CONFIG_SYSTEM=<hostile file>` | env | `env -i` (and `NOSYSTEM=1` also wins over `GIT_CONFIG_SYSTEM`, measured) | yes |
| E6 | `XDG_CONFIG_HOME` → `git/ignore` | env → default file | `env -i` | yes |
| E7 | `XDG_CONFIG_HOME` → `git/config` with `core.excludesFile` | env → config | `env -i` | yes. ⚠ **Draft 1's E7 is replaced.** It set `init.templateDir` via XDG config, which is **not live inside the constructed environment**: `GIT_TEMPLATE_DIR` outranks `init.templateDir`, and prototype B reported it `NOT-LIVE` on the unmutated run |
| E8 | `GIT_INDEX_FILE`, targeting a dedicated `$CTL/hostile/otherrepo` (**not** `routeddecoy`) | env (routing) | `env -i` | yes |
| H1 | `$HOME/.gitconfig` (`core.excludesFile`) | default file | `HOME=$VOID` | yes |
| H2 | `$HOME/.config/git/ignore` | default file | `HOME=$VOID` | yes |
| H3 | `$HOME/.config/git/attributes` (`working-tree-encoding`) | default file | `HOME=$VOID` | yes (content) |
| H4 | `$HOME/.gitconfig` `filter.*.clean` + attributes `filter=` | default file → config | `HOME=$VOID` | yes (blob becomes `e69de29b`, empty) |
| S1 | `$(prefix)/etc/gitconfig` | compiled-in path | `GIT_CONFIG_NOSYSTEM=1` | cannot be planted without writing into the machine's prefix; pinned by P-b |
| S2 | `$(prefix)/etc/gitattributes` | compiled-in path | `GIT_ATTR_NOSYSTEM=1` | same as S1 |
| S3 | the compiled-in template directory | compiled-in path | `GIT_TEMPLATE_DIR=$VOID` | pinned by P-d (content) |
| R1 | a template's `config`, `HEAD`, `info/exclude`, `info/attributes`, `hooks/pre-commit` | repository-local, seeded by the template | the empty template. P-d compares **content**, because a template holding only `config`/`HEAD` leaves the **names** under `.git` identical (measured, §6.1) | yes (each) |
| K | any configuration key: `core.excludesFile`, `core.attributesFile`, `core.autocrlf`, `filter.*`, `core.fsmonitor`, `core.hooksPath`, `init.*`, `commit.gpgSign`, `index.*`, `safe.directory`, `include.*`, … | configuration | **not by key.** Non-local layers are emptied. A template cannot seed the local layer, because P-d compares by content. A local-layer `include.path` is caught by its origin (P-a) | via E2–E5, E7, H1 |
| U | `GIT_ATTR_SOURCE`, `GIT_*_PATHSPECS`, `GIT_TEST_*`, `GIT_EXEC_PATH`, `GIT_DEFAULT_HASH`, `GIT_DEFAULT_REF_FORMAT`, `GIT_OBJECT_DIRECTORY`, loader variables (`DYLD_*`, `LD_PRELOAD`), … | env | `env -i` | **not probed.** These are closed without anyone having to list them. The row illustrates the class; it is not a census |

**Where the hostile inputs are written.** The controls file writes them during its assertion phase,
under `$CTL/hostile/`, next to the §6.3 probes that use them. They are never written in the fixtures
file (§7) and never under the caller's `HOME`.

## §5 Declared residuals: what no process-level mechanism here closes

| # | residual | direction | owner |
|---|---|---|---|
| R1 | **Which git binary runs.** This is `PATH`, captured at source time. This slice also adds a residual of its **own**: a **shim** git (for example a version-manager wrapper that needs its own variables) fails under `env -i` | loud: the fixtures fail to build, so the run is red with NE | The `PATH` half is the parent's D12, owned by `#11-trip-wire-launch-environment`. The **shim-under-`env -i`** half is new in this slice. Because it is loud, the wire's own rule accepts it. It is stated at `_fgit` and **not** routed to that slot |
| R2 | **Repository config derived from the filesystem**, written by `init`: `core.ignorecase` and `core.precomposeunicode` are `true` here and absent on ext4, plus `core.symlinks` and `core.filemode`. Also FIFO support, permission enforcement and raw filename bytes | either | Not closable in-process. `_fifo_ok` and `_perm_line` report two of these. P-d's reference `init` runs on the same filesystem, so these keys cancel out of the comparison |
| R3 | A **compiled-in path that no variable governs** and that `git var` does not report | silent, if one exists | **None is known at 2.55**: §4 found none, and none can be proven absent from inside. If such a path carries configuration or template content, P-a or P-d catches it |
| R4 | The **launch-environment class**: anything the caller can inject into the shell that runs the build. Examples: exported functions (`BASH_FUNC_*%%`, which override `printf`, `ln` and so on; measured by the reviewer), `BASH_ENV`, `SHELLOPTS`, a function named `command`, and non-git tools resolved through `PATH`. This row names a class; the examples are not a list | whatever the caller makes it | `#11-trip-wire-launch-environment` (unchanged); see also §10 row 15 |
| R5 | `$SCRATCH` owned by another UID. This is `safe.directory`: with the global layer emptied, no caller entry can authorise the directory | loud: the build fails | stated at `_fgit` |
| R6 | Fixture **reads** go through `_git`, which keeps the caller's config by design | as `_git`'s contract says | `_git`, and the `cfgkept` and fsmonitor controls (§7) |
| R7 | **Windows git-bash.** It is not in the trip-wires matrix: `sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep runs-on` prints `ubuntu-latest`. `env -i` there also drops `SYSTEMROOT` | unmeasured | declared, not claimed |
| R8 | **Ref-backend and object-hash defaults**, whether compiled in or set by `GIT_DEFAULT_REF_FORMAT`/`GIT_DEFAULT_HASH`. The environment half is closed by `env -i`. The `badref` fixture writes `.git/refs/heads/<br>` directly | loud: the build fails, so NE | Stated at `badref`. A compiled-in reftable default (Git 3.0) makes that fixture fail loudly rather than silently. Nothing here closes this |
| R9 | An **absolute-path** git invocation inside the fixtures file, such as `/usr/bin/git` or `"$_REAL_GIT"` | silent | The one spelling §7's guard cannot see. Booked as an **open defect** in §7 |

## §6 Self-verification

Every check is a **pair**. A liveness half first shows that the thing being asserted about can actually
happen; the assertion comes second. If the liveness half fails, the check reports
`CONTROL NOT EXERCISED` and never a pass. This is the lesson of the `_envlive` misspelling
(`ff6b99a3` `009e93db`).

Each check has **its own label** (F9). `_mut_trial` matches its needle anywhere in the output, so a
shared label would credit a record to whichever check happened to fire.

### §6.1 Postconditions: git's own answer about what it would read (always on)

Each runs once per wire run, in probe repos built by `_fgit`. Each is a *"Not a `_control`"* block with
its own `_lbl`, defined in the controls file (§10 row 8).

| id | label | assertion | liveness half | pins |
|---|---|---|---|---|
| P-a | `the fixture git reads configuration only from the fixture's own config file` | Every line of `_fgit config --list --show-scope --show-origin` in a probe repo starts with `local<TAB>file:.git/config<TAB>`. **Any other scope or origin is red.** That includes a scope git adds in a later version, and a local `include.path` pointing to an outside file | The same command with `-c a.b=c` prints a `command` line, which proves the parser can see a non-local scope | the layers and includes |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM` and `git var GIT_ATTR_SYSTEM` exit non-zero and print nothing. `git var GIT_CONFIG_GLOBAL` and `git var GIT_ATTR_GLOBAL` exit 0, and every line they print is under `$_FGIT_VOID/` | Both `SYSTEM` names, asked again with `GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0` appended, **print a path**. This keeps a git that does not know these `git var` names from passing as "disabled" | `GIT_CONFIG_NOSYSTEM`, `GIT_ATTR_NOSYSTEM`, `HOME=$VOID` |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` holds no entries **after the whole build** | The same listing sees an entry planted in a throwaway directory | that the void stays empty |
| P-d | `the fixture git copies no template` | `diff -r` between the `.git` of a probe made by `_fgit init` and that of `_fgit init --template="$_FGIT_VOID"` is empty. This compares **content, not names**. The `--template` flag outranks every other template source, so equality means the **effective** template was the void | none needed for the equality itself. The record discriminates only where the default template is non-empty, which is true on both gits here | `GIT_TEMPLATE_DIR` (S3, R1) |
| P-e | `the fixture git is the git the wire reads with` | `_fgit --exec-path` equals `git --exec-path` | — | `PATH`. Discriminates only where `env`'s default-path git differs, as on this Mac; the record says so |

**Evidence behind the F1 disposition.** Measured on git 2.55.0 and Apple 2.54.0; the commands are in §11
X6.

A template holding only `config` (`core.excludesFile`, `core.hooksPath`) and `HEAD`
(`ref: refs/heads/hostile`) gives these results:

- `find .git | sort` is **identical** to a void-template init, so draft 1's P-d would have **passed** it
- `diff -r` shows a **difference**, so draft 2's P-d reds
- The copied keys report scope `local` and origin `file:.git/config`. **No origin check can see them.**
  P-d by content is the only thing that does

A local `include.path=<outside file>` gives these results:

- It reports scope `local` with origin `file:<outside path>`, so draft 1's P-a would have passed it
- Draft 2's P-a reds on it

The clean probe passes both checks on both shells. Draft 1's P-a label and the "include" claim in its §4
row K were false as stated. Both are corrected above.

### §6.2 Pins: one mutation record per allowlist entry, with measured predictions

Prototype B ran each mutant on both shells, and §11 X5 re-runs them. The second column is **what was
measured**, not what was expected.

| mutant | prototype B result | record's needle |
|---|---|---|
| none | every probe and P-check passes | — |
| `command env -i` → `command env` (inherit) | §6.3 scrub goes red on **E2 E3 E4 E6 E7 E8**. E1, E5 and H1–H4 stay equal, because the allowlist's own `GIT_TEMPLATE_DIR`, `NOSYSTEM` and `HOME` override the inherited values. The P-checks pass | §6.3 label |
| `HOME=$VOID` dropped | P-b (`GIT_CONFIG_GLOBAL` exits 1) | P-b |
| `HOME=$VOID` → `HOME=$HOME`, captured at **source time** | P-b only (global paths under the real home) | P-b |
| `… "HOME=$HOME"` appended at **call time** | P-b, plus §6.3 scrub red on **H1–H4** | §6.3 label (one record); P-b is also printed |
| `GIT_CONFIG_NOSYSTEM=1` dropped | P-b (`/opt/homebrew/etc/gitconfig`) | P-b |
| `GIT_ATTR_NOSYSTEM=1` dropped | P-b (`/opt/homebrew/etc/gitattributes`, printed even though the file does not exist) | P-b |
| `GIT_TEMPLATE_DIR=$VOID` dropped | P-d | P-d. Discriminates where the default template is non-empty |
| `PATH=$PATH` dropped | P-e | P-e. Discriminates where `env`'s default-path git differs. **On the runner it may survive.** The mutation run is local-only today (`WEBREF_WIRE_MUTANTS`), and the record says so |

The P-a and P-d mutants were not run against the whole wire in the standalone prototype. X5 runs every
one of them through the real mutation runner.

### §6.3 Hostile-channel probes: one per roster row E1–E8 and H1–H4

A helper in the harness takes one channel as an **array** of assignments, never as a word-split string.
That is the `_ctl_env` lesson from #501 R96: `$CTL` can contain a space.

The helper runs the §1 probe build three times:

1. **Baseline.** `command env -i "${_FGIT_ENV[@]}" git …`, using the **same array** `_fgit` uses (§10
   row 10). It must track `victim.py` with the baseline blob.
2. **Liveness.** The same, with the channel appended **inside** the constructed environment. `env` keeps
   the last assignment (`env -i A=1 A=2 env` prints `A=2` on BSD `env`; the GNU case is in §6.5). The
   result must **differ** from the baseline. If it does not, the helper reports
   `CONTROL NOT EXERCISED (… <channel>)`.
3. **Scrub.** `( export "${channel[@]}"; _fgit … )`. The result must **equal** the baseline. If it does
   not, the helper reports
   `CONTROL FAILED (a hostile caller git environment cannot change what _fgit builds): <channel>`.

It compares `ls-files -s` plus `ls-tree -r HEAD`, both read through `_fgit` from a clean shell. That is
exactly property P.

### §6.4 End-to-end: once through the real path

**The fixture and its control.** The `envscrub` fixture is built in the fixtures file, with the E2E
channel set exported around its build subshell. It is checked by

`_control "$CTL/envscrub" 1 "(staged)" "a fixture built under a hostile caller git environment still poses its question"`

This check has **its own label and its own record** (F9). The record's mutant is the same `env -i` → `env`
edit as §6.2 row 2, but it carries this needle. That costs one extra control pass.

**The channel set: E1, E2, E3, E5 and E6.** They are chosen so that no member shadows another.

- HOME-class channels are left out. In a union, `GIT_CONFIG_GLOBAL` shadows `~/.gitconfig`, and an
  `XDG_CONFIG_HOME` shadows `~/.config/git`
- Routing (E8) is left out. If it leaked, the build would fail, and the result would be NE rather than
  the question the fixture is meant to pose

Per-channel coverage is §6.3's job. §6.4 goes through the real path once.

**Ordering (MIN fixed).** Fixtures are built **before** the assertion phase, and §6.3's liveness runs in
the assertion phase. So `envscrub`'s `_control` is placed **after** the §6.3 block and consults its
verdicts. If any member of the E2E set was not live, `envscrub` reports `CONTROL NOT EXERCISED` instead
of asserting.

**Staged only**, as in `ff6b99a3`. The wire's worktree inventory uses `--exclude-per-directory`, so
staging is the only thing a leak can change.

### §6.5 Portability

**Bash 3.2.57 and 5.3.** All of the following were measured on both versions this session:

- arrays, and `"${a[@]}"` of a non-empty array under `set -u`
- `command env -i`
- `< <( … )`, used by P-a
- the `PATH` shim still catching `git` after bash had already hashed it, because assigning `PATH` resets
  the hash table (§7)

Nothing uses `mapfile`, `declare -A` or `${!prefix@}`. Nothing enumerates the environment, so the
`GIT_X-Y` / `${!GIT_@}` divergence that `ff6b99a3` spent three commits on **does not arise**.

**BSD and GNU `env -i`.** Both follow POSIX. "The last duplicate assignment wins" is measured on BSD only.
X9 runs `env -i` on GNU, but no fixture or probe on the runner **checks** the duplicate-assignment order
in isolation. If GNU differed, the §6.3 liveness halves would report `NOT EXERCISED`. That is loud, so the
risk is bounded but unproven.

**sed prefixes on mutation records.** A target prefix must not parse as a sed command.

- `harness:` is an error on BSD sed (measured)
- **The GNU half is unverified.** The claim that a `controls:` prefix would be valid under GNU sed (`c`
  with inline text) rests on the GNU manual's *a/i/c* text. It was not executed, because there is no GNU
  sed on this machine. X9 does not exercise it either, because the mutation run is local-only

So no record targets the controls file. Everything a record must pin lives in the harness, including §7's
guard.

**Git versions.**

- P-b's liveness half turns a `git var` too old to know these names into a refusal
- P-d compares against a second `init` rather than against a skeleton list, so a git that changes what
  `init` creates does not go red

## §7 `_fgit` vs `_git`, and the build guard

`_git`, in the wire, **preserves** the caller's configuration. It purges only
`git rev-parse --local-env-vars` minus `GIT_CONFIG*`. That is correct for reading a checkout that only a
caller's `safe.directory` makes readable (#501 R97).

`_fgit` **constructs** its environment instead. The two policies share nothing, and `_fgit` does not call
`_git`.

### §7.1 The guard: every `git` the build resolves through `PATH` goes red (F7)

**The guard helpers.** The harness defines `_fgit_guard_on` and `_fgit_guard_off`.

`_fgit_guard_on` does three things, in order:

1. It saves `PATH` and prepends `$CTL/.guard/bin`, which holds a `git` that writes a marker file and
   exits 97.
2. It runs a **canary**: `( git --version )` and `_git --version` must both create the marker. If they
   do not, it reports `CONTROL NOT EXERCISED (a fixture is built only through _fgit)`.
3. It **removes the marker**, so the build starts from a clean marker.

The controls file calls `_fgit_guard_on` immediately before it **sources** the fixtures file, and
`_fgit_guard_off` immediately after. The fixtures file is sourced, not executed, because it appends to
`_FIX_FAILED` in the calling shell. `_fgit_guard_off` restores `PATH`, then requires that the marker is
absent, under the label `a fixture is built only through _fgit`.

**What it catches.** Every spelling that resolves `git` through `PATH` hits the marker: bare `git`,
`command git`, `env git`, `exec git`, `xargs git`, `$( git … )`, and `_git` (which runs `exec git`).
This was measured on bash 3.2.57 and 5.3, including after `git` had already been hashed before the shim.
`_fgit` is **unaffected**, because its `PATH` was captured at source time; the measurement shows it as
"missed", as intended. After `PATH` is restored, a bare `git` leaves no marker.

**Why this, and not a function shadow or a grep.** Draft 1 shadowed `git` with a function. A function
shadow misses `command`, `env`, `exec`, `xargs` and `_git`, and `_git` is the whole defect class, since it
keeps the caller's config. A `PATH` shim closes all of them with one mechanism, and it is a property over
*executions*. A grep only samples *spellings*.

**The one gap, booked as an open defect (R9).** An **absolute path** in the fixtures file bypasses
`PATH`: `/usr/bin/git`, `"$_REAL_GIT"`, or a `$(command -v git)` captured earlier. The slot is
`#11-k2-fixture-git-absolute-path`:

- **Gap:** nothing reds on such a spelling
- **Why deferred:** no in-process mechanism can intercept an absolute exec. The candidates are a seed
  grep, which cannot return the whole population, or an OS sandbox (§3.1 (iv))
- **Trigger:** any absolute path to a git binary added to the fixtures file, or `_REAL_GIT` used outside
  a generated shim
- **Owner:** citation-hygiene lane
- **Re-eval:** 2026-11-30

This is 1 own deferral for this PR (cap ≤3). The comment at the site names the slot. ⚠ **No seed census
can stand in for it.** The fixtures legitimately **write** `"$_REAL_GIT"` and `…/git` paths into generated
shims, so a text search for absolute git paths cannot tell writing from invoking. Measured at base: over
the build region (controls lines 83–733, non-comment),
`/usr/bin/grep -nE '/git[ "]|_REAL_GIT'` matches the shim-writing lines (`"$_REAL_GIT" > "$CTL/fakegit/git"`
and others). That inability is why this is a slot and not a check.

### §7.2 The two existing carve slots

**`#11-k2-fgit-keepset-depends-on-git-purge-glob`: DISSOLVES.**

- Premise re-measured. The slot says, verbatim: *"the cheap closure is one sentence at the keep-set
  naming the dependency (taken in this PR)"*. At `ff6b99a3`, this command finds no such sentence (rc 1):

  ```sh
  git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i 'exempt\|held back\|purge'
  ```

- Create-time audit. The slot's defect is a keep-set passed **into `_git`**. There is now no keep-set
  and no call into `_git`. The structural closure the slot names is exactly §3.3.

**`#11-k2-fixture-git-invocation-convention`: TAKEN (C6), and widened to cover `_git` (§7.1).**

- Premise re-measured. The slot says, verbatim: *"Measured 2026-09-26: the build region contains no bare
  `git ` outside comments"*. At `e8f78896` that is true (X7's census over the build region matches 0
  lines). At `ff6b99a3` it is false: 5 lines match, all in `_envlive`, all `env -i … git`, which §7.1's
  shim **does** catch.
- The slot's own trigger (*"any change to `_fgit`'s contract"*) fires in this PR

**Ledger step (outside this diff).** The defer registry is a memory file: `project_open-defer-slots.md`
in the project memory directory. At landing, update it as follows:

- remove the dissolved slot
- close the taken slot, citing the landing SHA
- add `#11-k2-fixture-git-absolute-path`
- fix the file header's running count and source line

## §8 The 1000-line split plan

### §8.1 Measured sizes, and which rule was broken

Sizes from `wc -l .claude/tools/webref-generic-core-trip-wire*.sh`:

| file | `e8f78896` | `ff6b99a3` |
|---|---|---|
| wire | 1259 | — |
| controls | 989 | 1135 |
| mutations | 783 | 983 |
| harness | 195 | 364 |

At `ff6b99a3`'s first commit, `c8f52724`, controls was already 1026 lines:
`git show c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l`.

⚠ **Draft 1 blamed the wrong rule.** The parent's §11.3 bullet was **conditional**: "if this PR takes it
past … the split is its own commit first". #519 **discharged** it with the harness split (parent §11.7
row 7). What `ff6b99a3` broke was **CLAUDE.md's touch-time rule itself**. It touched a file at the
threshold, crossed the threshold in its first commit, and took no prereq split.

### §8.2 Decisions

| file | split? | seam | why |
|---|---|---|---|
| **controls** (989) | **yes: prereq commit C1** | **Build vs assert.** A new `…trip-wire.fixtures.sh` takes the build phase: from the fixture-dir `mkdir` loop (`for d in clean pin …`) up to the line before `ctl_ok=0`. That is every tree, every shim and every index/worktree disagreement. The controls file **sources** it (§7.1). The controls file keeps the entry contract, the sourcing, every `_control`, every *"Not a `_control`"* block (relcwd, fsmonitor, umask, FIFO, perm, and the new §6 blocks), the mutation hook-up and the summary | This PR adds to both halves, so the file would cross 1000 lines. The seam already exists as phase order in the file. It also gives §7.1's guard an exact scope |
| **mutations** (783) | **yes: prereq commit C2** | **The two populations** the file's own header names (`TWO POPULATIONS, AND THE BOUNDARY IS WHAT EACH ONE'S UNIT IS`). The generated half moves to `…trip-wire.mutgen.sh`: `_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`, `_mut_splice` and `_mut_gen_run`. The hand records, `_mut_correspondence`, `_mut_trial` (shared) and `_mut_run` stay | `ff6b99a3` took this file to 983 lines with the same target infrastructure the rebuild keeps (§9). The rebuild then adds records for §6.1, §6.2, §6.3, §6.4 and §7, which takes it past 1000. So the split is taken now rather than promised conditionally |
| harness (195) | no | — | It stays far below the threshold even after §3.3, the §6.3 helper and §7.1's guard are added |
| **wire** (1259) | **no, and it is not touched** | — | At base, `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` prints 0, and `_git` is unchanged. The parent's §10.5 cohesion judgement stands |

### §8.3 What each split drags with it, and the parent and umbrella sites (F4, F11)

**One parts inventory, read by all three places in `_mut_run` (F11).** Today `_mut_run` handles the
sibling files in three separate places:

- one `cp` line per sibling
- a trap: `trap 'command rm -f "$_mut_wire" "$_mut_ctl" "$_mut_mut" "$_mut_hns"; …'`
- a stale-file `case` that skips only `"$_mut_wire"|"$_mut_ctl"`

To see all three:
`sed -n '/^_mut_run()/,/cp "\$_HARNESS"/p' .claude/tools/webref-generic-core-trip-wire.mutations.sh`.

C1 replaces them with **one** list of parts, and the `cp`, the trap's `rm -f` and the stale-skip `case`
all walk that list. If one place were missed, a copy that no trap removes would be left in
`.claude/tools/`, and `git add -A` would stage it. C1 also adds a guard: `ls "${SELF%.sh}".*.sh`, minus
`*.mutant.*`, must equal the list. An unlisted sibling is red.

**Entry contracts extend to the new files.** The controls file's entry contract, including its refusal on
a missing sibling ("decided nothing", exit 2), extends to the fixtures file. The mutations file's
`_mut_missing` contract extends to the mutgen file.

**Every parent and umbrella site that describes the wire's file set or `_fgit`**, found by command:

```sh
/usr/bin/grep -nE 'trip-wire\.(controls|harness|mutations)\.sh|controls file|control harness|mutation set|_fgit|GIT_TEMPLATE_DIR' \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md docs/plans/2026-07-citation-hygiene-umbrella.md
git grep -lE 'trip-wire\.(controls|harness|mutations)\.sh' -- docs/plans   # only the parent (this memo excluded)
```

Run at `bd3dc513`, this finds 36 lines in the parent and 0 in the umbrella. The umbrella's A-i-wire row
points **into** the parent (§4, §8, §11.1) without naming files. The parent's 36 lines fall into two
kinds.

*Current-state descriptions of the file set* become false after C1, C2 or C5:

- §0's derivation command, which names the controls file
- §4's artifact table: the controls row (*"a fixture tree per assertion"*), the harness row, and the
  mutations row, which still holds the generated population
- §7 criterion 3 (*"The mutation set is enumerated in `…trip-wire.mutations.sh`"*)
- §11.1's D6 row (*"stays loud"*)
- §11.4 P1's hand-named file list (*"the wire, its controls, the control harness and the mutation set"*)

*Provenance* is everything else:

- §5's items
- §7 criterion 5's history
- §8's withdrawn slot
- §9
- every §10 round, including **§10.6's R7 P2 row**, which draft 1 miscalled "§10.x"
- §11.3, §11.6 and §11.7

**Decision: the parent is not edited (F4, and §13 Q3).**

- Provenance is not rewritten. §10.6 R7 records what #519 believed at the time, and correcting it would
  falsify the record
- The current-state sites are superseded **by a pointer from the umbrella**, not by editing them. This
  draft amends the umbrella's A-i-wire row to say two things: its pointers describe the file set **as of
  #519**, and this memo holds the file set, `_fgit`'s design and the D6 criterion after this slice
  (§0.1, §3, §8)
- §11.4 P1's instrument is superseded here by a glob instead of a hand list: `ls
  .claude/tools/webref-generic-core-trip-wire*.sh`, which is X10's population
- **Q3 is answered.** CLAUDE.md's touch-time discipline applies to **any** touch, docs included, and it
  asks for a cohesion judgement and for avoiding a touch when the edit is avoidable. Here it is
  avoidable: the umbrella is the index, and a one-cell amendment there carries the supersession. So no
  parent split is owed. If round 2 finds a parent edit unavoidable, that edit's split becomes a
  standalone prereq **before** C1
- F6's constraint is honoured without touching the parent. The D6 row is **not** reduced to a bare
  pointer: it stays as #519 wrote it, and §0.1 here states the scope change and the reason for it

## §9 What survives from `ff6b99a3`, and the commit plan

### §9.1 The 14 commits

| commit | subject (short) | fate |
|---|---|---|
| `c8f52724` | scrub the whole `GIT_*` environment | The mechanism is **superseded**. Its mutation-target plumbing **survives** into C4: the `harness:` prefix, `_mut_restore_copies` and the `cmp` fix |
| `4ad051ea` | one target list and a resolver | **survives** → C4 |
| `c5820bd5` | a comment syntax for mutation records | **survives** → C3 |
| `a1007bfb` | ground the keep-set in config layers | **superseded**. Its premise was false: a template's `config` is copied into the local layer (§6.1 evidence) |
| `162edf98` | prove the scrub against `env` | **superseded**: with no sweep, there is nothing to prove |
| `328dc22a` | measure the disjointness behind the refusal | **superseded** |
| `c39c7989` | bound the claim about an unknown target prefix | **survives** → C4 |
| `f8c74c6d` | fix two comments | The harness half is **superseded**. The mutations half **survives** → C4 |
| `6b4064fa` | `env -0` per record | **superseded** |
| `009e93db` | liveness before asserting (`_envlive`) | The **principle survives** in §6.3. The code is **superseded**: its second spelling of the environment leaked `HOME` (§2) |
| `4aced879` | restore/resolve order; say why nothing was applied | **survives** → C4 |
| `2dea82ff` | retire the hostile-build count | **superseded**. Its principle, "name a seed as a seed", survives in §4 |
| `6ffe2d5d` | one spelling for the pattern and the target | The **principle survives** in §6.3 |
| `ff6b99a3` | say what the order measurement ran over | **survives** → C4 |

**How the carried-forward code gets reviewed.** C3 and C4 carry `ff6b99a3` code. `ff6b99a3`'s Stage 5
reported "15 IMP / 8 MIN", but those findings are **not itemised** in the lane SSoT:
`/usr/bin/grep -n '15 IMP'` over it returns the tally line and one unrelated R4 row (`2 CRIT / 15 IMP / 15 MIN`), and no itemisation. So they cannot be recovered. Instead,
the rebuilt C3 and C4 are reviewed **fresh**. The rebuild's `/pre-push` Stage 4 runs over the whole
`e8f78896...k2-wire-fgit-hermetic` range, which includes C3 and C4. None of the old findings is assumed
closed.

### §9.2 Commit plan and land order for `k2-wire-fgit-hermetic` (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo (drafts 1 and 2) plus the umbrella rows (§0.2, §8.3) | docs | `/elidex-plan-review` round 2 |
| — | **plan-review gate** | | no code is written before it |
| C1 | Split controls into controls plus **fixtures** (sourced). Add one parts inventory feeding all three `_mut_run` readers, plus the sibling guard | prereq, behaviour-neutral | X1, X4 |
| C2 | Split mutations into mutations plus **mutgen** | prereq, behaviour-neutral | X1, X3, X4 |
| C3 | Let mutation records carry a `#` comment | infra | X3 (same verdict lines) |
| C4 | Let mutation records target the harness (`_MUT_TARGETS="wire harness"`, `_mut_target`, `_mut_restore_copies`). ⚠ Nothing exercises it until C5's records, and C4's message says so. C5 is where "matched nothing" must be seen to fire on a stale `harness:` anchor | infra | X3 |
| C5 | `_fgit` built from nothing (§3.3); `notcommitted`'s `mkdir -p .git/info`; P-a to P-e (§6.1); the §6.3 helper and probes; `envscrub` (§6.4); one record per §6.2 row, plus the §6.3 and §6.4 records; the ratchet population widened (F8, §10 row 12) | feature | X1, X2, X3, X5, X6, X11 |
| C6 | The `PATH` guard and its canary (§7.1). Its `harness:` record replaces the shim's `exit 97` with `exit 0` and drops the marker write, and must red with the canary's `NOT EXERCISED` label | feature (takes the slot) | X3, X7 |

**`ci.yml` is not edited (F5).** This PR does **not** change `timeout-minutes`. X8 measures the
always-run cost of base and head on both shells, and the delta is recorded. If the head's measured driver
time would exceed the base's configured `timeout-minutes`, the work **stops and goes to the user**; the
timeout is not edited to make room.

The budget half is **nobody's** job: see the parent's §6 and the *"budget half is nobody's"* sentence
under Cross-lane coordination in the umbrella. It has three claimants:

- #510, whose head at authoring time was `def01d3c…` (from `gh pr view 510 --json headRefOid`; re-read
  it, because it moves)
- `stale-claim-detector`
- A-i-wire, which this slice is part of

**Land order:**

1. Run `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`. Name the branch, not `HEAD`, because the
   review skills resolve `HEAD` in the session's cwd (lane SSoT).
2. Get **user approval** to push and open the PR, stacked on `webref-cite-audit-tool`.
3. Run `/external-converge`.
4. Squash into `webref-cite-audit-tool`, with user approval.
5. Resolve **#501's Codex P2 thread** on `_fgit`, citing the landed SHA.
6. Hand the **#501 merge decision to the user**.
7. Do the §7.2 ledger step.

## §10 Coupled invariants

The axes:

- **A**: enumeration direction
- **B**: git's config, exclude, attribute, template and hook layers
- **C**: shell and userland portability
- **D**: the gate's self-verification: controls, records, correspondence and the ratchet
- **E**: file structure
- **F**: the always-run budget

| # | axes | invariant | enforced at |
|---|---|---|---|
| 1 | A×B | `env -i` closes **variables**. **Files** at default locations close only through `HOME`, `NOSYSTEM`, `ATTR_NOSYSTEM` and `TEMPLATE_DIR`. A template-seeded **local** config is visible only to a **content** comparison (P-d), and an include only to an **origin** check (P-a). A scope check sees neither (§6.1 evidence). An unknown non-config file is R3 | §3.2, §6.1 |
| 2 | B×D | An empty template removes `.git/info/`, so a fixture that writes into `$GIT_DIR` must create its own directories. Measured: without that, `notcommitted` gives NE | C5, X1 |
| 3 | A×D | Every allowlist entry has a record that reds without it. System layers cannot be planted, so they are pinned by `git var`, never by a hostile file | §6.2 |
| 4 | C×D | Some pins discriminate only on some machines (`PATH`, `TEMPLATE_DIR`). Each such record says where, and the mutation run is local | §6.2 |
| 5 | C×A | The §7.1 guard sees every `PATH` resolution, `_git`'s included. It does not see an absolute path (R9; slot booked). No probe lives in the fixtures file | §7.1 |
| 6 | C | `env` keeps the last duplicate assignment. Measured on BSD; on GNU a violation would be loud (§6.5) | §6.3 |
| 7 | E×D | Every sourced sibling is in one parts list, which feeds `cp`, the trap's `rm` and the stale-skip `case`. An unlisted sibling is red | §8.3 |
| 8 | D×E | `_mut_correspondence` finds labels only in the **controls** file. So every new label is defined there, even when its helper lives in the harness | §6.1 |
| 9 | C×D | A mutation target's name must not parse as a sed command. BSD is measured; GNU comes from the manual and is unverified (§6.5). So the only targets are `wire` and `harness` | §6.5 |
| 10 | D×A | Every liveness baseline is built from `_FGIT_ENV` itself, never from a second spelling of it | §6.3 |
| 11 | B×D | What is compared is exactly P: index path, mode and blob, plus the `HEAD` tree | §1, §6.3 |
| 12 | D | **The ratchet's label population is widened (F8).** At base, `_mut_correspondence` builds the unrecorded-label list only from `_control` calls: `awk -F'"' '/^ *_control /{print $6}'` in `mutations.sh`. So `*_lbl="…"` labels are invisible to it. C5 builds the population as `_control` labels ∪ every `_lbl="…"` definition in the controls file. That costs nothing today: the three existing `_lbl` labels (relcwd, fsmonitor, umask) each have exactly one record (`/usr/bin/grep -cF "<TAB><label>"` prints 1 for each). `_MUT_RECORDS_MIN` rises by exactly the number of records added. `_MUT_UNRECORDED_MAX` does not rise | X3, X11 |
| 13 | F×D | §6.1, §6.3 and §7.1 add git executions to every always-run wire run. X8 measures that. `ci.yml` is not edited here; if the head exceeds the timeout, STOP and escalate | §9.2, X8 |
| 14 | A×read side | The build is hermetic and the read is not, by design. Controls that need the caller's configuration at read time (`cfgkept`, `routed`, fsmonitor) still get it through `_git` and `_ctl_env` | §7, X1 |
| 15 | A×C | P covers git's inputs only. The **non-git** commands of the build run in a shell the caller can inject into (R4's class, for example `BASH_FUNC_printf%%`), and no postcondition here can see that | §1, §5 R4 |

## §11 Exit criteria X1–X11 (run under `/opt/homebrew/bin/bash` 5.x and `/bin/bash` 3.2)

Here `W=.claude/tools/webref-generic-core-trip-wire.sh`. Judge each result by the text of the log, not
by the `$?` of a trailing `echo`.

| id | command | expected |
|---|---|---|
| X1 | `( $SH $W ) > $S/l 2>&1; echo rc=$?; /usr/bin/grep -c 'CONTROL NOT EXERCISED\|CONTROL FAILED' $S/l; /usr/bin/grep -c 'trip-wire PASSED' $S/l` | `rc=0`, `0`, `1` |
| X2 | X1, once per caller environment in §2's table and once per roster row E1–E8 and H1–H4, with the hostile files under `$S` | every run gives `rc=0`, `0`, `1` |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W > $S/m 2>&1`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED' $S/m` | three hits. For C1 to C3, the verdict lines are byte-identical to base's |
| X4 | For C1 and C2: take X1's log at the parent commit and at the split commit. Normalise the per-run scratch paths under `$TMPDIR` with one `sed` in each, then `diff` them | an empty diff, on both shells |
| X5 | For each §6.2 row, apply that row's edit to a `git clone --local` copy and run X1. Do the same for the P-a/P-d planting cases: a template holding `config`+`HEAD`, and a local `include.path` | `rc=1`, with that row's needle in the log. For rows marked machine-dependent, record the result; red is not required |
| X6 | The probes by hand. Run the §1 probe (a fresh dir holding `victim.py` and `ok.txt`; `init`, `add -A`, `commit` under `command env -i "${_FGIT_ENV[@]}"`) with and without each channel. Also run P-d's `diff -r` and P-a's origin listing over the F1 planting cases | the liveness halves differ; the scrub halves are equal; the planting cases red |
| X7 | the bare-word **seed** census over the fixtures file: `awk '!/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.fixtures.sh \| /usr/bin/grep -cE '(^\|[^_A-Za-z$/-])git [a-z-]'` (at base, over the build region, it prints `0`), plus the guard's canary line in X1's log | `0`. This is a seed, not the enforcement, and absolute paths are out of its reach (R9). The guard's label must not appear among the failures |
| X8 | Run `/usr/bin/time -p bash scripts/trip-wires.sh` three times each at base and at head, on both shells. Read base's `timeout-minutes` with `sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep timeout-minutes` | the driver gives `rc=0`, and the delta is **recorded in the PR body**. If the head's time would exceed the configured timeout, **STOP and escalate to the user**; do not edit anything |
| X9 | The PR's CI `Layering trip-wires` job (ubuntu, GNU `env` and `grep`) | SUCCESS. This is the GNU evidence for `env -i` and the probes. It is **not** evidence for the sed claim (§6.5) |
| X10 | `$SH -n` on every part in `ls .claude/tools/webref-generic-core-trip-wire*.sh`, and `wc -l` over the same glob | syntax is clean on both shells; every part is below 1000 lines |
| X11 | Ratchet negative control: in a clone, add `_x_lbl="an unrecorded probe"` and a matching `echo "!! CONTROL FAILED ($_x_lbl)"` to the controls file, then run X1 | red, and the ratchet lists `an unrecorded probe` among the bare labels |

## §12 Findings and premises that turned out false or imprecise

**Carried over from draft 1** (premises of the original brief; each is argued where cited): (1) "the
paths `git add -A` stages" is too narrow (§1); (2) "nulling global config opens the channel" overstates
it, since base is already red (§2); (3) the XDG "12" is 11 NE + 1 CF, and that the SSoT counted both is
an inference (§2); (4) `_envlive`'s 5 lines reach git through `env`, which §7.1's shim catches; (5)
`ff6b99a3`'s "local layer belongs to the repo `_fgit` creates" is false (§6.1 evidence); (6) the brief's
`env -i` shape changed four ways (§3.2); (7) base `_fgit` already nulled the global and system layers.

**New in draft 2 (round-1 findings, re-measured):**

8. **F1 is true.** Both parts reproduced (§6.1 evidence). One refinement: the origin check alone does
   **not** see a template-copied config, because that config really does live in `.git/config`. P-d by
   content is what covers it.
9. **F10 is true.** In the prototype, a source-time `HOME` capture reds only P-b.
10. **Draft 1's own E7 roster row was dead inside the constructed environment.** No reviewer reported
    this; prototype B's liveness half found it. The row was replaced.
11. **F7's premise needed refining.** `_git` is not a function that needs shadowing. It runs `exec git`,
    and the `PATH` shim catches that. Shadowing `_git` as well would have been a second mechanism.
12. Draft 1 misattributed three things:
    - §8.1's promise: the rule `ff6b99a3` broke is CLAUDE.md's touch-time rule, not the parent's §11.3.
    - The "§10.x" row: it is §10.6 R7, which is provenance and is left untouched.
    - `git help git`'s section: it is ENVIRONMENT VARIABLES.

These all measured **as stated**: the line counts, `ff6b99a3`'s 14 commits, `c8f52724`'s 1026 lines, both
slots' false premises, and the roles of `4ad051ea` and `c5820bd5`.

## §13 Questions for plan-review round 2

- **Q1: are P-a to P-e worth their always-on cost?** Draft answer: yes. They are the only pins for the
  system layers and the template. X8 measures their cost, and F5's STOP rule, not this memo, decides
  whether that cost is acceptable
- **Q2: should C4 fold into C5?** Draft answer: no. C4 is infrastructure with its own exit criterion,
  and its message records that it is first exercised in C5
- **Q3:** answered in §8.3. The parent is not edited; the umbrella carries the supersession
- **Q4 (new): does the umbrella amendment need its own review pass?** The amendment (§0.2, §8.3) touches
  a ratified surface. It is a pointer-only change to the A-i-wire row plus a new slice row. Does round 2
  accept that as is, or does it need its own review pass?
