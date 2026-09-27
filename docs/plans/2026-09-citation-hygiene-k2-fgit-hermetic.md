# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire-fgit** (registered
there, §0.2).

**Companion**: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`. It holds the
at-heads measurements, the fate of `ff6b99a3`'s 14 commits, the premises found false, and every
plan-review round's dispositions, including round 3's terminator. This memo holds only the live
decisions.

**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`, #519's record. §8.3 says
how it stays true.

**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (#501's head: `gh pr view 501 --json headRefOid`).
**Replaces**: `ff6b99a3`. **Decision**: user, 2026-09-27, option **(a)**: rebuild.

**Status**: **draft 3**. It answers plan-review round 2 on draft 2 (`0142f47a`): 0 CRIT / 10 IMP /
19 MIN. 8 of those 10 IMP landed on text that draft 2 had added. So draft 3 **removes** mechanism
instead of adding more. It withdraws the `PATH`-shim guard, `envscrub`, the 12 per-channel probes and
slot `#11-k2-fixture-git-absolute-path`. One whole-build differential replaces all four (§6.3). Round 3
is owed. Its terminator is at the top of the companion's dispositions section.

⚠ **The parent's rule applies: state no quantity that moves with a commit.** Every figure is either a
measurement at a named SHA or scratch prototype, given with its command, or it is replaced by that
command. `$S` is scratch. The real `HOME` is never written. `/usr/bin/grep` is spelled out because the
default `grep` here is ugrep, which can return a silent 0.

---

## §0 Why this exists

Codex raised a P2 on #501. `_fgit`, the helper that builds the K2 wire's fixture repos, did not
neutralise `GIT_TEMPLATE_DIR`. A caller's template `info/exclude` then hid the fixtures' own files from
`git add -A`.

That was the **second** time a name list around `_fgit` failed:

1. The first was #519's Codex R7 finding (parent §10.6: `GIT_CONFIG_COUNT`). It was fixed by unsetting
   three names. ⚠ Draft 2 attributed this to "#501 R97", but that is the `safe.directory` finding cited
   at `_git`.
2. `ff6b99a3` then inverted the list into a `GIT_*` sweep plus a keep-set.
3. That failed a **third** time, on `$HOME/.config/git/ignore`. That file is the **default** value of
   `core.excludesFile`. It is neither a `GIT_*` name nor a config layer (companion §A).

This memo grounds the mechanism on a **property** (§1). It also chooses which way an unlisted channel
fails (§3.1).

### §0.1 The acceptance criterion changes, and where it stops

The parent's §11.1 judged each input by the rule "unreachable or loud, never silent". Under that rule it
classed D6 (`GIT_TEMPLATE_DIR`) as loud, and so acceptable.

**For the fixture build, this slice replaces that criterion with property P.** Loud failures that the
caller causes are now defects too. The reason is where they come from: the measured loud failures come
from **ordinary developer configuration**. In a required gate, a false red caused by an ordinary setup
blocks a valid checkout. Every one of those failures was loud (rc 1; companion §A). That is exactly why
the old criterion never objected.

**The boundary (D3).** P governs the **inputs git reads when it decides what to stage and commit**:
environment, configuration, ignore and attribute files, and the template. P does **not** govern which
executable runs or the platform it runs on:

- the `git` binary and anything wrapping it (R1);
- the filesystem's semantics (R2);
- ownership of the scratch directory (R5).

A failure in that second group means "this git cannot run here", not "the caller changed what the
fixture holds". So those failures stay under the old rule: loud is acceptable. The line sits there
because only inputs can be neutralised from inside the process. An executable or a platform can only be
*chosen*, and the gate does not own that choice; the launch-environment slot does (§5).

The **read** side keeps the old criterion. `_git` preserves the caller's configuration on purpose (#501
R97).

### §0.2 One slice, and where it is registered

CLAUDE.md's edge-dense rule applies, because §10 has three or more intersecting axes. So this work needs
a registration and a plan-review before code, and it is then a terminal unit.

**Registration.** The umbrella gets a slice row **A-i-wire-fgit** and a memo-table row, both pointers.
The umbrella is a ratified surface, so that amendment is itself under review.

**The slice is property P for the fixture build, and nothing else.** It consists of the mechanism (§3),
the postconditions (§6.1), and the whole-build differential (§6.3). The differential also covers P's
"only through `_fgit`" half.

**The other commits each have their own ground:**

- **C1/C2** are standalone prereq splits (§8).
- **C0a** is a standalone prereq split of the parent. It is taken because this slice must touch the
  parent (§8.3).
- **C4** lets records target the harness. It is required, because every §6.2 record edits the harness.
- **C3** adds record comments. ⚠ It is **not** required to write records, contrary to what draft 2 said:
  base has records without it. C3 stands on its own ground. §6.2 requires the machine-dependent records
  (P-d, P-e) to state where they discriminate, **at the record**. Base's format cannot hold a note beside
  a record; its own history records moving such a note 65 lines away.

**CLAUDE.md's clause, in full**, answering Ax4:

> "…**standalone な prereq split** として分割する (feature PR に bundle しない — split は単独 PR / 単独 commit)".

The parenthesis names two admissible forms, and a standalone **commit** is one of them. This memo reads
"bundle" as "split and feature in one commit". The precedent is the parent's §11.7 row 7: #519 landed its
harness split as its own commit inside the PR. If round 3 reads the clause as "never in the feature PR",
then C0a, C1 and C2 move to a prereq PR stacked below this one. That costs nothing, because they are
already standalone commits (§12 Q1).

## §1 The property

The controls assert over two things: staged **content**, and **commits**. Two channels change a fixture
without changing its path set (companion §A):

- an attributes file that re-encodes a blob;
- a template `pre-commit` hook that removes an entry from the index and from `HEAD`.

> **P.** For each fixture, the following are a function of the fixture script's git commands and the
> working-tree files it writes, and of no git input the caller carries:
>
> - the **index**: path, mode and blob id of every entry;
> - the **tree of `HEAD`**;
> - **which ref `HEAD` names**;
> - the **set of ref names**.
>
> "Git inputs the caller carries" means: environment variables; files at default locations (home, XDG,
> the system prefix, the compiled-in template); and the configuration those name.

**Refs belong in P.** A hostile `init.defaultBranch`, or a template `HEAD`, changes which branch a
fixture is on. `headprobe` and `badref` build from that name
(`_br="$(_fgit symbolic-ref --short HEAD)"`). Ref **values** do not belong in P, because commit ids vary
with timestamps.

**Outside P, by the boundary in §0.1:**

- timestamps and reflog identity;
- the build's **non-git** commands (`printf`, `ln`, …). These run in the caller's shell and belong to the
  launch-environment class (§5 R4; `BASH_FUNC_printf%%`, measured by the reviewer);
- reads at control time.

## §2 What was measured

This section has moved to the companion (§A). Three findings from it still drive decisions here:

- Attributes are a channel that neither prior head names.
- The ignore channel is already open at base.
- Every measured failure was loud.

## §2.5 Spec coverage map

**No spec surface** — this slice changes a shell helper, its controls and its mutation records. It
touches no WHATWG, W3C, TC39 or CSS WG behaviour.

The authority for the channel classes is git's own documentation at the version measured:

- `git help gitignore`
- `git help git-config`, section FILES
- `git help gitattributes`
- `git help git-init`, section TEMPLATE DIRECTORY
- `git help git`, section ENVIRONMENT VARIABLES
- `git help git-var`

webref does not index these. The behaviour measured in §6 is the rest of the authority.

This section follows A-iii's shape and the parent's §0.5/§3: a heading with **no table**, so
`preflight.py` fails **by design**. The marker is A-ii's §4.2.5 feature, and A-ii has not landed. The
marker line matches A-ii's recogniser `^ {0,3}\*\*No spec surface\*\*`: bold closes before the dash
(A-iii Codex R13).

```sh
/usr/bin/grep -nE '^ {0,3}\*\*No spec surface\*\*' docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md   # one hit
python3 .claude/skills/elidex-plan-review/preflight.py \
  docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md; echo "rc=$?"   # rc=1, "no markdown table follows it"
```

## §3 Mechanism

### §3.1 The choice, from the failure direction

The question for each shape: what happens to a channel nobody listed?

| shape | a channel nobody listed… | verdict |
|---|---|---|
| (i) unset a list of names (base) | reaches the build. Measured outcome: a false red | failed twice |
| (ii) sweep `GIT_*` + keep-set (`ff6b99a3`) | reaches the build if it is not a `GIT_*` name, or not a config layer | failed a third time |
| (iii) **construct the environment from nothing** (`env -i` plus an allowlist), with every default location pointed at an empty directory | **closed**, whether listed or not. Files at default locations close through the allowlist entries that relocate or disable them. Afterwards, git's own answer is asserted (§6.1) | **chosen** |
| (iv) an OS sandbox | closed, including compiled-in paths | no portable mechanism across macOS and the ubuntu runner; needs privileges |

**What the documentation supports.** `git help git` (ENVIRONMENT VARIABLES, `GIT_CONFIG_NOSYSTEM`)
endorses the **ingredients**: *"can be used along with $HOME and $XDG_CONFIG_HOME to create a
predictable environment"*. It does not describe the exact shape used here.

The shape leaves `XDG_CONFIG_HOME` unset. That is safe because of a documented fallback:

- `git help git-config` (FILES): *"When the XDG_CONFIG_HOME environment variable is not set or empty,
  $HOME/.config/ is used"*.
- `gitignore(5)` and `gitattributes(5)` say the same for their default files.

Under (iii), a channel stays closed until somebody allowlists it, and each allowlist entry is pinned by a
record (§6.2).

### §3.2 The allowlist

The last column is measured. It is also §6.2's predicted failure for each entry.

| entry | why it is on the list | without it (git 2.55.0 / Apple 2.54.0) |
|---|---|---|
| `PATH=$PATH`, captured when the harness is sourced | the fixtures must be built by the same git the wire reads them with (parent D12) | BSD `env -i` runs `/usr/bin/git`, which is Apple 2.54, not 2.55. Compare `env -i HOME=$S/v git --version` with `env -i PATH="$PATH" HOME=$S/v git --version` |
| `HOME=$VOID` | relocates every HOME-relative default. `XDG_CONFIG_HOME` stays unset, so the §3.1 fallback puts its default inside `$VOID` | an unset `HOME` also closes the channel on 2.55: `git var GIT_CONFIG_GLOBAL` returns rc 1. The void is chosen instead because a future HOME-relative default then lands where P-c looks. "Unset" relies on git never falling back to the passwd home directory, and that cannot be tested without writing to the real home |
| `GIT_CONFIG_NOSYSTEM=1` | turns off the system config layer, which is live on this machine: `git var -l` under `env -i` prints `credential.helper=osxkeychain`, and Apple git also prints `init.defaultbranch=main` | `git var GIT_CONFIG_SYSTEM` prints a path |
| `GIT_ATTR_NOSYSTEM=1` | turns off the system attributes file | `git var GIT_ATTR_SYSTEM` prints a path. This variable is **undocumented** at 2.55: `for p in git gitattributes git-config gitignore; do git help -m $p \| col -b \| /usr/bin/grep -c GIT_ATTR_NOSYSTEM; done` prints `0 0 0 0`. It is therefore pinned by `git var`'s answer, not by its name |
| `GIT_TEMPLATE_DIR=$VOID` | overrides every template source (`git help git-init`, TEMPLATE DIRECTORY). A template can seed `config` and `HEAD`, not only `info/*` and hooks | the compiled-in default template is copied in |

`$VOID` is one fresh directory under the wire's `$SCRATCH`, which is already checked and cleaned up by a
trap. It is empty when created, and P-c asserts it is still empty after the build.

**`_FGIT_ENV` is a snapshot taken when the harness is sourced.** It is never re-evaluated per call,
because an environment built from nothing must not depend on the caller's state at call time.

**Deliberately not on the list:**

- `XDG_CONFIG_HOME`: the fallback covers it.
- locale variables: they only affect messages.
- commit identity: passed per call with `-c`.
- `_git`'s read switches: P-a and P-d already exclude configuration, and carrying the switches would
  re-couple `_fgit` to `_git`.

### §3.3 Shape

```sh
# harness — spelled once, expanded once
_FGIT_VOID="$SCRATCH/fgit-void"          # mkdir, checked
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID")
_fgit() { command env -i "${_FGIT_ENV[@]}" git "$@"; }
```

`command env` is used so that a shell function named `env` cannot intercept the call. `_fgit` no longer
calls `_git`.

**One fixture needs a change: `notcommitted`.** It writes to `.git/info/exclude`, but only the ambient
template ever created `.git/info/`. Without adding `mkdir -p .git/info`, a clean caller gets rc 1 with
1 NE (companion §A, prototype A).

## §4 Channel classes: what closes each

This table is a map, not a checklist. §3 closes these channels by construction. This slice plants only
three hostile inputs: the three members of the §6.3 differential. Draft 2's per-channel roster, and the
probes it fed, are withdrawn; their measurements are in companion §A.

| class | examples | closed by | checked by |
|---|---|---|---|
| any environment variable | `GIT_TEMPLATE_DIR`, the `GIT_CONFIG_COUNT` trio, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL`, `XDG_CONFIG_HOME`, routing (`GIT_DIR`, `GIT_INDEX_FILE`), `GIT_ATTR_SOURCE`, `GIT_DEFAULT_*`, `GIT_TEST_*`, loader variables, … | `env -i` | §6.3 (env member), and its §6.2 record |
| HOME-relative default files | `~/.gitconfig`, `~/.config/git/{config,ignore,attributes}` | `HOME=$VOID` | P-b, P-c; §6.3 (home member) |
| compiled-in system files | `$(prefix)/etc/gitconfig`, `$(prefix)/etc/gitattributes` | `GIT_CONFIG_NOSYSTEM`, `GIT_ATTR_NOSYSTEM` | P-b. These files cannot be planted without writing into the machine's prefix |
| template content | `info/exclude`, `info/attributes`, `hooks/*`, `config`, `HEAD` | `GIT_TEMPLATE_DIR=$VOID` | P-d (by content); §6.3 (template member, for leaking callers) |
| any configuration key | `core.excludesFile`, `core.attributesFile`, `core.autocrlf`, `filter.*`, `core.fsmonitor`, `core.hooksPath`, `init.*`, `include.*`, `safe.directory`, … | not by key: non-local layers are emptied, and only `init` writes the local layer | P-a (scope **and** origin), P-d (content) |

## §5 Declared residuals

| # | residual | direction | owner / disposition |
|---|---|---|---|
| R1 | the **git executable**: how `PATH` resolves it, and a shim git that fails under `env -i` | loud | outside P, per §0.1's boundary. The `PATH` half belongs to `#11-trip-wire-launch-environment`. The shim half is new in this slice; it is stated at `_fgit` and accepted under the boundary |
| R2 | config that `init` derives from the **filesystem**: `core.ignorecase` and `core.precomposeunicode` (true here, absent on ext4), `core.symlinks`, `core.filemode`. Also FIFO support, permissions and raw filename bytes | either | outside P, per §0.1. `_fifo_ok` and `_perm_line` report two of these. P-d's reference `init` runs on the same filesystem, so the differences cancel |
| R3 | **machine files that no relocation reaches**. (a) A compiled-in path that no variable governs and that `git var` does not report; none is known at 2.55. (b) A fixture git call that bypasses `_fgit` **without** inheriting the caller's environment (`env -i git`). It reads no caller input, but it does read the system layer and the default template. Measured: §6.3 stays green on it | silent, if the machine's own files are hostile | **booked** as `#11-k2-fgit-machine-files` (below) |
| R4 | the **launch-environment class**: anything the caller injects into the shell that runs the build. Examples: `BASH_FUNC_*%%`, `BASH_ENV`, `SHELLOPTS`, a function named `command`, non-git tools on `PATH` | whatever the caller makes it | `#11-trip-wire-launch-environment`, unchanged |
| R5 | `$SCRATCH` owned by another UID (`safe.directory`) | loud | outside P, per §0.1 |
| R6 | fixture **reads** through `_git` keep the caller's configuration | per `_git`'s contract | `_git`, `cfgkept`, fsmonitor |
| R7 | Windows git-bash is not in the trip-wires matrix: `sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep runs-on` prints `ubuntu-latest` | unmeasured | declared |
| R8 | ref-backend and hash defaults. `env -i` closes the environment half; the compiled-in default stays open. `badref` writes `.git/refs/heads/` directly. Under a reftable default, P-d goes red, because two `init --ref-format=reftable` runs differ under `diff -r`: the table names in `.git/reftable/` are random (re-measured) | loud | declared. A Git 3.0 reftable default would red P-d and `badref` loudly, not silently |

**`#11-k2-fgit-machine-files`** is this slice's one own deferral (cap ≤3).

- **Gap**: git files at machine level (R3 a and b) can reach a fixture without anything going red.
- **Why deferred**: (a) cannot be planted or enumerated from inside the process. (b) could be closed by
  running P-a/P-d over **every** fixture repo instead of one probe. That is new mechanism with a cost,
  and round 2 asked for less mechanism.
- **Trigger**: any of: a git release that adds a system-level default file; a fixture that calls git
  other than through `_fgit`; a hostile system or template file found on a runner.
- **Owner**: citation-hygiene lane.
- **Re-eval**: 2026-11-30.

## §6 Self-verification

Every check comes as a pair: a liveness half must hold before the assertion counts. Every check has
**its own label**, and every label has **its own mutation record** (§6.2).

### §6.1 Postconditions

These are always on. Each runs once per run, in probe repos built by `_fgit`.

| id | label | assertion | liveness |
|---|---|---|---|
| P-a | `the fixture git reads configuration only from the fixture's own config file` | every line of `_fgit config --list --show-scope --show-origin` starts with `local<TAB>file:.git/config<TAB>`. Any other scope or origin fails, which covers both a future scope and a local `include.path` | adding `-c a.b=c` must produce a `command` line |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM` and `GIT_ATTR_SYSTEM` exit non-zero with no output. `GIT_CONFIG_GLOBAL` and `GIT_ATTR_GLOBAL` exit 0, and every line they print is under `$_FGIT_VOID/` | with `GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0` appended, both `SYSTEM` names print a path. So a git that does not know these names refuses instead of passing |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` is empty. It **runs last**: after the differential's hostile build and its reads, which are the final `_fgit` writers (§10 row 8) | a planted entry in a throwaway directory must be seen |
| P-d | `the fixture git copies no template` | `diff -r` between the `.git` from `_fgit init` and the `.git` from `_fgit init --template="$_FGIT_VOID"` is empty. This compares **content**: a template holding only `config` and `HEAD` leaves the names under `find .git` identical, yet differs under `diff -r` (companion §A) | none; it discriminates where the default template is non-empty, which is true on both gits here |
| P-e | `the fixture git is the git the wire reads with` | `_fgit --exec-path` equals `git --exec-path` | none; it discriminates only where `env`'s default-path git differs, which is true on this Mac |

### §6.2 One record per label, with measured predictions

The records edit the harness (C4). The predictions below were measured in scratch prototypes on both
shells (companion §A, prototypes B and D). §11 X5 re-runs them through the real runner.

| label | record (mutant) | measured |
|---|---|---|
| P-a | append `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=a.b GIT_CONFIG_VALUE_0=c` to `_FGIT_ENV` | P-a red (scope `command`); P-c and P-d stay ok |
| P-b | drop `GIT_CONFIG_NOSYSTEM=1` | P-b red (`/opt/homebrew/etc/gitconfig`). Dropping `GIT_ATTR_NOSYSTEM`, dropping `HOME`, or capturing `HOME=$HOME` at source time also reds P-b. These are not separate records, because they would pin the same label |
| P-c | point `_FGIT_VOID` at a non-empty directory | P-c red. P-a stays ok, and **P-d stays ok** because both inits read the same directory. So P-c is the only pin for "the void is empty" |
| P-d | drop `GIT_TEMPLATE_DIR=$VOID` | P-d red. The record notes (C3) that it discriminates only where the default template is non-empty |
| P-e | drop `PATH=$PATH` | P-e red on this Mac. The record notes that it **may survive on the runner**, since the mutation run is local-only (`WEBREF_WIRE_MUTANTS`) |
| §6.3 | change `command env -i` to `command env` (inherit) | differential red on **every** fixture repo, 80 of 80 |

That gives six labels and six records. The ratchet consequence is §10 row 12.

### §6.3 The whole-build differential (D1)

This replaces draft 2's guard, `envscrub` and the per-channel probes.

**What it asserts.** The fixture set is built twice by the same fixtures file. The first build is the
ordinary one, under the caller. The second runs under an exported **hostile** environment, into a second
root, `$SCRATCH/hctl`. P is then compared repo by repo. The label is `a hostile caller environment does
not change what the fixture build produces`.

This is a property over the build's **output**. It holds whatever spelling a fixture uses to call git,
which is why it replaces a guard over spellings. Draft 2's guard missed eight non-absolute spellings.

**The hostile environment has one universal member per closing mechanism.** Each member's effect is
universal in two ways:

- It excludes `*`. A leaking `add -A` then stages nothing, and an explicit-path `add` of an ignored path
  fails loudly.
- It changes the branch `HEAD` names. So even fixtures whose index is built by plumbing (`update-index
  --cacheinfo`) differ in P.

| member | closed by | contents |
|---|---|---|
| env | `env -i` | `GIT_CONFIG_COUNT=2`: `core.excludesFile=<file holding *>`, and `init.defaultBranch=hostile-env` |
| home | `HOME=$VOID` | `HOME=<dir>` with `.config/git/ignore` = `*` and `.gitconfig` `[init] defaultBranch = hostile-home`. Also `XDG_CONFIG_HOME=<dir>/.config`, so that the caller's own XDG setting cannot shadow this member |
| template | `GIT_TEMPLATE_DIR=$VOID` (for a leaking caller) | `GIT_TEMPLATE_DIR=<dir>` with `info/exclude` = `*` and `HEAD` = `ref: refs/heads/hostile-tpl` |

When all three leak at once, for example through a bare `git`, the env member's `core.excludesFile`
suppresses the home member's XDG default (Ax2 measured this). That does not matter: detection needs only
one member to act. Each member is proven live **alone**, so shadowing inside the union cannot hide a dead
member.

**Liveness is proven on the same instance, at the same time.** The steps run in this order:

1. All hostile files are written first.
2. For each member, exported alone, a plain `git init` followed by `git add -A` runs in a canary
   directory. That is the leaking spelling itself, run in the same subshell context the hostile build
   uses next.
3. Each canary must stage nothing **and** leave `HEAD` on the hostile branch. Otherwise the result is
   `CONTROL NOT EXERCISED (<label>): <member>`.
4. Only then does the hostile build run.

This fixes draft 2's `envscrub` fault, where the liveness files were written after the build they
vouched for.

**Verdict state and order (D1, Ax2 IMP-4).** The differential runs in the controls file **after**
`ctl_ok=0`. That initialiser is `controls.sh:738` at base, and it clobbers any earlier value of 1. The
differential runs **before** the first `_control`, and if it fails it sets `ctl_ok=1`. The full order is:

1. the clean build (the fixtures file is sourced);
2. `ctl_ok=0`;
3. the §6.3 differential;
4. P-a, P-b, P-d and P-e;
5. the controls;
6. P-c;
7. `[ "$ctl_ok" -eq 0 ] || exit 1`.

**Errexit.** The wire runs under `set -euo pipefail` (wire:311), so every command that is expected to
fail needs guarding:

- The canary's deliberately failing plain `git` runs as `( … ) >/dev/null 2>&1 || true`.
- The hostile build runs as `( export …; CTL=…; . "$_FIXTURES" ) >/dev/null 2>&1 || _hd_rc=$?`. A
  non-zero `_hd_rc` is itself a failure, reported as `rc=<n>` in the message.
- The P reader ends in an explicit `true`. This was found the hard way: a reader whose final
  `sed … packed-refs` failed, on a repo without packed refs, made `x="$(reader)"` abort the whole run
  with no diagnostic and no verdict (prototype D).

**P, as read.** For each repo, through `_fgit`:

- `ls-files -s`;
- `rev-parse -q --verify 'HEAD^{tree}'`;
- the contents of `.git/HEAD`;
- ref names, from `find .git/refs -type f` plus `packed-refs`.

That is two git calls per repo; the rest are file reads. A four-call version cost measurably more
(companion §A).

**Position independence is now a checked property.** If a fixture's P embeds its own location, it
differs between the two roots. `linkname` did: `ln -s "$CTL/linkname/ok.py"` stores the absolute path as
the blob. It becomes `ln -s ok.py`. The fixture's subject is the link's **name**, not its target, and its
control stays green. Any future fixture that embeds `$CTL` will red the differential, so this failure is
loud.

**What the differential catches.** Measured in scratch prototype D on bash 5.3 and 3.2. Prototype D is a
`git clone --local` of base with §3.3, the fixtures split out, and this block.

| subject | result |
|---|---|
| unmutated | rc 0, `PASSED`; 80 repos compared, 0 differing |
| `_fgit` inherits the environment (`command env`) | red; 80 of 80 differ |
| `_fgit` adds `HOME=$HOME` at call time | red (home member) |
| one fixture step changed to `_git add -A` | red; only `cachedir` differs |
| one fixture step changed to `PATH="$CTL/fakegit:$PATH" git add -A` | red; only `cachedir` |
| one fixture step changed to `/opt/homebrew/bin/git add -A` (absolute path) | red; only `cachedir` |
| one fixture step changed to `env -i PATH="$PATH" git add -A` | **green**. This spelling reads no caller input; it is R3(b), booked |

So the absolute-path spelling that draft 2 booked as a slot is **caught**. The slot is withdrawn, and so
is draft 2's premise that "no in-process mechanism" could catch it.

**Cost.** The differential adds one more build of the fixture set, plus two reads per repo. There is no
measured way to avoid the second full build. The only single-build variant would be to build under the
hostile environment alone, which leaves nothing to compare P against. The existing controls are no
substitute either: when a leak leaves a fixture's files untracked, a green-direction control stays green,
because the worktree inventory still reads those files. The ci.yml derivation is in §9.

### §6.4 Portability

- **bash 3.2.57 and 5.3.** Measured on both: arrays under `set -u`; `command env -i`; `< <( … )`;
  re-sourcing the fixtures file in a subshell with a different `CTL`; and the differential prototype end
  to end. Nothing enumerates the environment.
- **BSD vs GNU `env`.** "Last assignment wins" is measured on BSD. On GNU, the end-to-end evidence is X9.
  A GNU difference would surface as `NOT EXERCISED` from P-b's liveness, which is loud.
- **sed.** A `harness:` target is an error under BSD sed (measured). The claim that `controls:` is valid
  under GNU sed comes from the GNU manual (*a/i/c* inline text) and is **unverified**: there is no GNU sed
  here, and X9 does not exercise it. So the only targets are `wire` and `harness`.
- **git versions.** P-b's liveness refuses a `git var` that does not know the names. P-d compares two
  `init`s rather than checking a list. Reftable is R8.

## §7 `_fgit` vs `_git`, and the two carve slots

`_git` **preserves** the caller's configuration: it purges `git rev-parse --local-env-vars` minus
`GIT_CONFIG*` (#501 R97). `_fgit` **constructs** its environment. The two share no policy, and `_fgit`
does not call `_git`. A fixture that calls `_git` is caught by §6.3 (measured).

**`#11-k2-fgit-keepset-depends-on-git-purge-glob`: DISSOLVES.**

- The slot's defect was a keep-set passed **through** `_git`'s purge. There is now no keep-set and no
  such call.
- Its premise, verbatim: "one sentence … (taken in this PR)". At `ff6b99a3` that sentence is absent:
  `git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.harness.sh | /usr/bin/grep -n -i 'exempt\|held back\|purge'`
  returns rc 1.
- ⚠ Draft 2 said §3.3 "is exactly" the slot's structural closure, which overstated it. The slot named a
  shared `_git_exec` holding both helpers' non-environment policy. That is **not built**, and it is not
  needed, because the helpers now share no policy.

**`#11-k2-fixture-git-invocation-convention`: CLOSED by §6.3.**

- The slot's gap was a fixture that bypasses `_fgit`. §6.3 catches every bypass that inherits the
  caller's environment, whatever its spelling (measured above). The one residual bypass, a call that
  inherits nothing, is R3(b), booked.
- Its premise, verbatim: "Measured 2026-09-26: the build region contains no bare `git ` outside comments".
  - At `e8f78896` it holds:
    `awk 'NR>=83 && NR<=733 && !/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -cE '(^|[^_A-Za-z$/-])git [a-z-]'`
    prints `0`.
  - At `ff6b99a3` it fails: `_envlive` has 5 such lines.
- **The slot's rejected alternative is what §6.3 now is.** The slot rejected "exporting the hostile
  environment around the whole build phase to make every fixture a control" as "diffuse redundancy over a
  uniform mechanism". That objection targets using the **controls** as the detector, which is diffuse,
  indirect and spelling-dependent. §6.3 instead compares P directly, under one label, with universal
  members.
- The slot also noted that `_FIX_FAILED` is lost in a subshell. That does not apply here: the hostile
  build **wants** its own `_FIX_FAILED`, and the comparison is over output.

**Ledger step.** This touches a memory file outside this diff, `project_open-defer-slots.md`:

- remove the dissolved slot;
- close the taken slot;
- add `#11-k2-fgit-machine-files`;
- fix the header's count and source.

Cite **#501's merge commit on `main`** and this PR's number. Do not cite the `k2-wire-fgit-hermetic`
squash SHA on `webref-cite-audit-tool`: it becomes unreachable from `main` once #501 is squash-merged.

## §8 Splits, and keeping every entry point true

### §8.1 Sizes, and which rule was broken

Sizes, from `wc -l .claude/tools/webref-generic-core-trip-wire*.sh`:

| file | `e8f78896` | `ff6b99a3` |
|---|---|---|
| wire | 1259 | — |
| controls | 989 | 1135 |
| mutations | 783 | 983 |
| harness | 195 | 364 |

`git show c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l` prints `1026`.

The rule `ff6b99a3` broke is **CLAUDE.md's touch-time split**. It did not break the parent's §11.3,
which was conditional and which #519 discharged (parent §11.7 row 7).

### §8.2 Code splits

| file | split | seam | ground |
|---|---|---|---|
| controls (989) | **C1** | **build vs assert.** A new `…trip-wire.fixtures.sh` takes base controls lines 83–733: from `for d in clean pin …` up to the line before `ctl_ok=0`. The controls file keeps the contract, the sourcing, every `_control`, the *"Not a `_control`"* blocks, the mutation hook-up and the summary | this PR grows both halves. §6.3 also **requires** the build to be a separately sourceable unit, because it is sourced twice. Prototype D measured the extraction at 651 + 346 lines |
| mutations (783) | **C2** | the two populations the header names. The generated half (`_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`, `_mut_splice`, `_mut_gen_run`) moves to `…trip-wire.mutgen.sh` | `ff6b99a3` reached 983 lines with the infrastructure C3/C4 carry forward, and this slice adds six records |
| harness (195) | no | — | stays far below the threshold |
| wire (1259) | **no, not touched** | — | `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` prints `0`, and `_git` is unchanged |

**What C1/C2 bring with them:**

- **One parts list feeds all three readers in `_mut_run`**: the `cp`, the trap's `rm -f`, and the
  stale-skip `case`. Today the trap names four files, and the `case` skips only the wire and controls:
  `sed -n '/^_mut_run()/,/cp "\$_HARNESS"/p' .claude/tools/webref-generic-core-trip-wire.mutations.sh`.
  A copy that nothing removes stays in `.claude/tools/`, where `git add -A` would stage it.
- **The sibling guard** checks that `ls "${SELF%.sh}".*.sh`, minus `*.mutant.*`, equals the list. It
  runs **inside `_mut_run`, before its loop, and nowhere else.** Inside a mutant child, `SELF` is
  `….mutant.$$.sh`, so every sibling carries `.mutant.` and the filtered glob is empty. An always-on
  placement would kill every mutant for the wrong reason (Ax2 R2).
- **Entry contracts extend to the new files.** The controls file refuses a missing fixtures file ("decided
  nothing", exit 2), and `_mut_missing` covers mutgen.

### §8.3 The parent memo, the umbrella, and the code's memo references (D4)

**What CLAUDE.md actually says** (the 1000-line bullet, quoted): "…>1000行 file を触る際、real cohesion
seam があれば feature 着手前に **standalone な prereq split** として分割する (feature PR に bundle しない —
split は単独 PR / 単独 commit)。line-count の機械適用でなく **cohesion 判断** … 本 discipline は
**touch-time の proactive な振る舞い** (any-size touch / source・test 問わず)。"

Draft 2's claim that the touch was "avoidable" rested on a rule CLAUDE.md does not contain, and it is
withdrawn.

**Why the parent must be touched.** After landing, these parts of the parent still read as live but are
false:

- the header, L3–10 (*"Status: …"*);
- §4's file table;
- §7 criterion 5;
- §11.1's D6 row;
- §11.4 P1's file list.

A reader who arrives from the umbrella, from a code comment, or directly, sees them as current. The only
honest fix at that entry point is a **supersession banner** in the parent's header. It names the four
superseded sites and this memo. The body stays as #519 wrote it, because it is provenance.

**The cohesion judgement for that touch.** The parent is 1384 lines (`wc -l`). Its section map comes from
`/usr/bin/grep -n '^## ' docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`:

- Two blocks are pure record, with no live decision: **§5** (round 1's eight dispositions, L291–591) and
  **§10** (the external-review rounds, L900–1185).
- The live parts are §1–§4; §6 (the interpreter floor); §7; §8 (A-i-wire's defer slots); §9; and §11
  (the wire's design revision, which the wire cites at §11 and §11.6).

That is a real seam, record against design, and it is the same seam this memo applies to itself in D5.
Two commits follow from it:

- **C0a** (a standalone prereq, before the banner) moves the bodies of §5 and §10 to
  `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire-rounds.md`. The `## §5` and `## §10` headings
  stay in the parent as one-line stubs that point there, so the section numbers and every inbound `§10.x`
  or `§5 item N` reference still resolve.
  - Check: the move is byte-identical. The companion's body, concatenated in order, equals the removed
    ranges: `diff <(git show HEAD~:<parent> | sed -n '<ranges>') <(sed -n '<body>' <companion>)`.
  - The parent's resulting size is re-measured with `wc -l` at that commit; no figure is written here.
  - The rounds companion carries a `## §0 Spec coverage map` heading with the no-spec-surface marker. Like
    this memo's companion, it therefore exits preflight 1 by design, and the umbrella memo-table row says so.
- **C0b** adds the banner.

**The umbrella.**

- The A-i-wire row names the parent file explicitly. Its pointers (§4, §8, §11.1) are now marked "as of
  #519".
- The A-i-wire-fgit row points here, naming each file and section exactly: this memo's §0.1
  (criterion), §3 (`_fgit`) and §8.2 (file set), and the parent's §8 for the A-i-wire defer slots, which
  stay there.
- A **landing step** updates the umbrella's memo-table status cell for this slice (§9).

**Memo references in code.** At base,
`/usr/bin/grep -n 'plan memo\|memo §\|memo'"'"'s §' .claude/tools/webref-generic-core-trip-wire*.sh`
returns 17 lines: most in the wire, 4 in controls. Some of them cite the A-i memo, which the wire's
header names. An unqualified "plan memo §N" could land on this memo's section numbers. The rule:

- **In files this PR edits** (controls, fixtures, harness, mutations, mutgen), every memo reference is
  qualified with the memo's **file name**. This is done in C1/C2 as a comment-only change inside the
  behaviour-neutral commits, and X4 checks it.
- **In the untouched wire**, a convention applies, stated in the parent banner and here: an unqualified
  "plan memo" in a K2 wire file means the memo named in that file's header. It never means this memo,
  which is always cited by file name. The wire is not edited to enforce this, because touching it would
  be a 1000-line touch (§8.2).

## §9 Commit plan and land order (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo, its companion and the umbrella rows (drafts 1–3) | docs | plan-review round 3 |
| — | **plan-review gate** | | no further commits until it passes |
| C0a | parent: move the §5 and §10 bodies to the rounds companion, leaving stub headings | prereq split (docs) | byte-identity `diff` (§8.3) |
| C0b | parent: supersession banner and the reference convention | docs | the banner names the four sites |
| C1 | split controls into controls + fixtures (sourced); one parts list in `_mut_run`; sibling guard inside `_mut_run`; memo references qualified | prereq split | X1, X4 |
| C2 | split mutations into mutations + mutgen | prereq split | X1, X3, X4 |
| C3 | record comments (`#` in column 1) | infra, on its own ground (§0.2) | X3 |
| C4 | records may target the harness (`_MUT_TARGETS="wire harness"`). `wire` is the default and is **never stripped**, because `w` is a valid sed command | infra, required by §6.2 | X3; first exercised by C5 |
| C5 | §3.3; `mkdir -p .git/info` in `notcommitted`; `ln -s ok.py` in `linkname`; P-a…P-e; §6.3; the six records; the ratchet population; the `ci.yml` comment re-derived | feature | X1, X2, X3, X5, X6, X8, X11 |

**Cost, and `ci.yml` (D1).**

The base `trip-wires` job comment carries an in-file rule: *"a wire that adds fixture self-tests
re-derives this line in the same PR"*. To see it:
`git show e8f78896:.github/workflows/ci.yml | sed -n '/^  trip-wires:/,/^  [a-z]/p'`. Draft 2's "ci.yml
is not edited" contradicted that rule and is withdrawn.

C5 re-derives the line by the comment's own method. It runs `/usr/bin/time -p bash scripts/trip-wires.sh`
three times on the branch and three times on its base. It then records the **method and verdict** in the
comment, without figures, as the comment itself requires. If the derived value differs from the current
`5`, **STOP and escalate to the user**. Nobody owns the budget question (parent §6; umbrella, Cross-lane
coordination), and three lanes hold different values.

A prototype measurement already exists (prototype D, companion §A). It includes the differential but not
the P-checks:

- On a loaded Mac, three runs on each side, the driver was materially slower at head than at base.
- Both sides stayed far inside the configured budget.
- The run-to-run spread overlapped the difference, which matches the comment's own previous
  re-derivation.

**X8 repeats this on the real C5 head.**

**Land order:**

1. Run `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`, naming the branch.
2. Get user approval to push and open the PR, stacked on `webref-cite-audit-tool`.
3. Run `/external-converge`.
4. Get user approval to squash.
5. Resolve #501's Codex P2 thread.
6. Update the umbrella's memo-table status cell.
7. Hand the #501 merge decision to the user.
8. After #501 lands on `main`, do the §7 ledger step, citing #501's merge commit.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's layers · **C** portability · **D** self-verification ·
**E** file structure · **F** budget.

| # | axes | invariant | where |
|---|---|---|---|
| 1 | A×B | `env -i` closes variables. Files at default locations close only through the four relocations. A template-seeded local config is visible only to a **content** check (P-d), and an include only to an **origin** check (P-a) | §3.2, §6.1 |
| 2 | B×E | an empty template removes `.git/info/`, so a fixture that writes there must create it | C5 |
| 3 | A×D | every allowlist entry is pinned. System layers are pinned by `git var`, because they cannot be planted | §6.2 |
| 4 | C×D | the P-d and P-e pins depend on the machine, and each record says where (C3) | §6.2 |
| 5 | A×D | each of §6.3's hostile members has a universal effect, and each is proven live **alone** before the build that uses them, in the same subshell context | §6.3 |
| 6 | E×D | the fixtures file is position-independent: P must not embed `$CTL`, and §6.3 enforces this | §6.3 |
| 7 | E×D | one parts list feeds `cp`, the trap and the `case`. The sibling guard lives only in `_mut_run` | §8.2 |
| 8 | D | verdict order: clean build → `ctl_ok=0` → §6.3 → P-a/b/d/e → controls → **P-c last** → exit | §6.3 |
| 9 | C×D | mutation targets are `wire` (the default, never stripped, since `w` is a sed command) and `harness`. `controls:` is unverified under GNU sed and not used | §6.4 |
| 10 | C | every command expected to fail runs under `\|\| true` or `\|\| rc=$?`, and every `$(…)` reader ends in `true`. wire:311 is `set -euo pipefail` | §6.3 |
| 11 | B×D | what is compared is P, refs included | §1, §6.3 |
| 12 | D | **the ratchet.** At base, `_mut_correspondence` builds its population of unrecorded labels only from `_control` calls (`awk -F'"' '/^ *_control /{print $6}'`), and `_MUT_UNRECORDED_MAX=21` is already reached (`mutations.sh:165`). C5 widens the population to `_control` labels **plus** every `_lbl="…"` definition. Every label that enters gets a record. The three existing `_lbl` labels each already have one (`/usr/bin/grep -cF "<TAB><label>"` prints 1 for each), and the six new labels get the six §6.2 records. So `_MUT_UNRECORDED_MAX` stays at 21, and `_MUT_RECORDS_MIN` rises by exactly six | X3, X11 |
| 13 | F×D | §6.3 roughly doubles the build. The ci.yml line is re-derived by its own rule, and a different value means STOP | §9 |
| 14 | A×read | the build is hermetic and the read is not, by design | §7 |
| 15 | A×C | P covers git's inputs only. The build's non-git commands, the executable and the platform all lie outside it (§0.1; R1, R2, R4, R5) | §0.1, §5 |

## §11 Exit criteria X1–X11 (run under `/opt/homebrew/bin/bash` 5.x and under `/bin/bash` 3.2)

`W` is `.claude/tools/webref-generic-core-trip-wire.sh`. Judge each result by the text of the log, not by
`$?`.

| id | command | expected |
|---|---|---|
| X1 | `( $SH $W ) > $S/l 2>&1; echo rc=$?; /usr/bin/grep -c 'CONTROL NOT EXERCISED\|CONTROL FAILED' $S/l; /usr/bin/grep -c 'trip-wire PASSED' $S/l` | `rc=0`, `0`, `1` |
| X2 | X1 under each caller environment in companion §A's table | each gives `rc=0`, `0`, `1` |
| X3 | `WEBREF_WIRE_MUTANTS=1 $SH $W > $S/m 2>&1`, then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED' $S/m` | three hits. For C1–C3, the verdict lines are byte-identical to base's |
| X4 | for C1 and C2: X1's log at the parent commit and at the split commit, with scratch paths normalised by one `sed`, then `diff` | empty, on both shells |
| X5 | every §6.2 record, run through the real runner; plus the §6.3 table's fixture-spelling mutants, applied by hand to a clone | each goes red with its own label. The `env -i git` spelling stays green (R3(b)) |
| X6 | P-a/P-d planting: a template holding only `config` + `HEAD`, and a local `include.path` | each goes red |
| X7 | — | withdrawn along with the guard |
| X8 | the ci.yml rule's own derivation (§9): three runs per side, on both shells | method and verdict written into the comment. If the value would change, STOP |
| X9 | the PR's CI job `Layering trip-wires` on ubuntu | SUCCESS. This is the GNU `env` evidence. It is **not** evidence for the sed claim |
| X10 | `$SH -n` over `ls .claude/tools/webref-generic-core-trip-wire*.sh`, and `wc -l` over the same set | clean, and every part below 1000 lines |
| X11 | ratchet negative control: in a clone, add `_x_lbl="an unrecorded probe"` and an `echo` that uses it to the controls file, then run X1 | red, with `an unrecorded probe` listed among the bare labels |

## §12 Questions for plan-review round 3

- **Q1.** Are C0a, C1 and C2 acceptable as standalone commits inside this PR, under "(feature PR に bundle
  しない — split は単独 PR / 単独 commit)"? Or must they be a separate prereq PR?
  *Draft answer:* commits, following the parent's §11.7 row 7 precedent. Moving them is free.
- **Q2.** Is R3(b), a fixture calling `env -i git` directly, acceptable as a booked slot?
  *Draft answer:* yes. It reads no caller input, and closing it means running P-a/P-d on every fixture,
  which is new mechanism.
- **Q3.** Is doubling the build acceptable in principle before X8 measures the real head?
  *Draft answer:* the ci.yml rule decides that, not this memo. The STOP is the guard.
