# K2 fixture git — build the fixtures from nothing (rebuild of the `_fgit` fix)

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i-wire** (follow-up).
**Parent memo**: `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md` (over 1000 lines, so
this is a new memo and not a new section there; see §8.3 for the two rows there that go stale).
**Branch**: `k2-wire-fgit-hermetic`, base `e8f78896` (= `origin/webref-cite-audit-tool`, #501's branch,
with #519 squashed in). **Replaces**: `k2-wire-fgit-scrub-complement` @ `ff6b99a3` (14 commits,
never pushed). **Decision**: user, 2026-09-27, option **(a)**: rebuild rather than ship, and do not
fall back to closing `GIT_TEMPLATE_DIR` alone. **Status**: draft 1. Not implemented. CLAUDE.md's
edge-dense rule makes `/elidex-plan-review` **mandatory before any code**, and §10 shows ≥3 axes
intersecting.

⚠ **The rule for this memo is the parent's: state no quantity that moves with a commit.** Every
figure below is either (a) a measurement at a **named SHA**, given with the command that produced
it, or (b) replaced by that command. `$S` is any scratch directory. Never plant files in the real
`HOME`. `/usr/bin/grep` is spelled out because the default `grep` here is ugrep, and ugrep can
return a silent 0.

---

## §0 Why this exists

Codex P2 on #501: `_fgit` (the helper the K2 wire's controls use to build their fixture repos) did
not neutralise `GIT_TEMPLATE_DIR`. A caller's template `info/exclude` then hid the fixtures' own
files from `git add -A`, and the controls reported `CONTROL NOT EXERCISED`. That was the **second**
time a name list around `_fgit` had missed a channel. The first was #501 R97, where
`GIT_CONFIG_COUNT` was fixed by unsetting three names.

`ff6b99a3` inverted the list: sweep every `GIT_*` and keep a keep-set
(`GIT_CONFIG_GLOBAL`/`_SYSTEM=/dev/null`), justified as "git's config layers". That is a list too.
It failed a **third** time, on a channel that is neither a `GIT_*` variable nor a config layer:
`$XDG_CONFIG_HOME/git/ignore` / `$HOME/.config/git/ignore`, which is where `core.excludesFile`
points when nothing sets it (§2).

Three failures of the same kind show that the thing enumerated was never the thing that mattered.
This memo re-grounds the mechanism on a **property**, not on a list, and it chooses the direction in
which an unenumerated channel fails.

## §1 The property, stated correctly

The brief phrased it as *"the set of paths `git add -A` stages is a function of the fixture's working
tree alone"*. **That is too narrow, and measurably so.** The controls assert over staged **content**
(the violation string is inside a blob) and over **commits** (`committed`, `orphan`, `headprobe`,
`badref`, `replaced`, `lstreefail` all commit). Two channels change a fixture without changing its
path set:

| channel (under an otherwise empty git environment) | effect on `victim.py` (`git ls-files -s`) |
|---|---|
| none (baseline) | blob `7d4290a1…` |
| `$HOME/.config/git/attributes`: `*.py working-tree-encoding=UTF-16LE` | blob `b8b71b2a…`: **content re-encoded**, path unchanged |
| template `hooks/pre-commit` running `git rm -q --cached victim.py` | **absent from index and HEAD**, and `commit` exits 0 |

(Measured on git 2.55.0 (Homebrew) with the probe in §11 E6: a fresh dir holding `victim.py` and
`ok.txt`, then `init` / `add -A` / `commit`, reading the result with a clean environment.)

So the property this design guarantees is:

> **P.** Every object, index entry (path, mode, blob), commit and ref that a fixture build produces
> is a function of the fixture script (the commands and files the controls write) and of nothing the
> caller carries.

"The caller carries" means the process environment, files at default locations outside the fixture
(home, XDG, system prefix, compiled-in template directory), and the configuration any of those name.
P is about the **build**. The **read** of a fixture at control time goes through `_git`, which
preserves the caller's configuration on purpose (#501 R97, `safe.directory`). That is `_git`'s
contract, and this memo does not change it (§7).

## §2 What was measured at the two heads

Invocation: `( cd <checkout> && env <ASSIGNMENTS> <shell> .claude/tools/webref-generic-core-trip-wire.sh ) > log 2>&1`,
then `/usr/bin/grep -c 'CONTROL NOT EXERCISED' log` (NE) and `/usr/bin/grep -c 'CONTROL FAILED' log`
(CF). Checkouts are `git clone --local` at the SHA, verified with `git rev-parse HEAD`. Results are
identical under GNU bash 5.3.20 and `/bin/bash` 3.2.57 wherever both were run.

| caller environment (all files under `$S`) | `ff6b99a3` | `e8f78896` (base) |
|---|---|---|
| `HOME` with `.gitconfig` setting `core.excludesFile` = a file holding `*.py` | rc 0, NE 0 | not run |
| `HOME` with an **empty** `.config/git/ignore` (isolating control) | rc 0, NE 0 | not run |
| `HOME` with `.config/git/ignore` = `*.py` | rc 1, **NE 12**, CF 0 | rc 1, NE 11, CF 0 (bash 3.2) |
| `XDG_CONFIG_HOME=<dir>` with `git/ignore` = `*.py` | rc 1, **NE 11, CF 1** | not run |
| `XDG_CONFIG_HOME=<dir>` with `git/attributes` = `*.py working-tree-encoding=UTF-16LE` | rc 1, NE 17, CF 5 (bash 3.2) | rc 1, NE 17, CF 4 (bash 3.2) |
| `XDG_CONFIG_HOME=<dir>` with `git/config` setting `init.templateDir` = a hook template | rc 0 (bash 3.2) | not run |
| `GIT_TEMPLATE_DIR=<dir>` with `info/exclude` = `*.py` | rc 0 (lane SSoT) | rc 1, NE 11, CF 0 (bash 3.2) |

**The 11-vs-12 difference is explained, not residual.** In both rows 12 controls fail. With `HOME`
hostile, the (3') liveness probe `_envlive` in `ff6b99a3`'s controls passes `"HOME=${HOME:-}"` into its
own `env -i` (`git show ff6b99a3:.claude/tools/webref-generic-core-trip-wire.controls.sh | /usr/bin/grep -n 'HOME=\${HOME'`).
So its **baseline** half is contaminated by the very channel under test, it reports "not
discriminating", and the `envscrub` control becomes **NE**. With `XDG_CONFIG_HOME` hostile, the probe's
`env -i` drops that variable, so the probe passes. The fixture is then built under `_fgit`, which
does not drop it, and `envscrub` becomes **CF** (`expected exit 1, got 0`). The lane SSoT's "12" for
the XDG row counted both kinds together.
Reproduce: diff the `NOT EXERCISED (…)` labels of the two logs. The only difference is
`a hostile caller git environment cannot change what a fixture holds`.

Two more findings from this table:

* **The attributes row is a fourth channel that neither head enumerates.** It is open at base and at
  `ff6b99a3` alike. It is not an exclude at all: it rewrites blob content (§1).
* **The ignore channel is not something `ff6b99a3` opened.** Base is already red on it. What nulling
  the global config changes is only that `core.excludesFile` is then guaranteed unset, so the default
  path is guaranteed to be read. A caller whose global config does not set `core.excludesFile` hits
  the default path anyway.

## §3 Mechanism

### §3.1 The choice, argued from the failure direction

| shape | an unenumerated channel… | verdict |
|---|---|---|
| (i) **unset a list of names** (base) | stays **open**, silently | failed twice |
| (ii) **sweep `GIT_*` + keep-set** (`ff6b99a3`) | open if it is **not a `GIT_*` name** or **not a config layer**: `HOME`, `XDG_CONFIG_HOME`, default files, template-seeded `config` | failed a third time (§2) |
| (iii) **construct the environment from nothing** (`env -i` + an allowlist) and point every default location at a known-empty place | **closed** for every environment variable, whether or not anybody named it. File channels at default locations close through the few allowlist entries that relocate them, and a **postcondition asks git itself** what it would read (§6.1) | **chosen** |
| (iv) run the build in an OS sandbox (a scratch user, `sandbox-exec`, a container) | closed, including compiled-in paths | out of reach: no portable mechanism across macOS and the ubuntu runner, and it needs privileges this gate does not have |

(iii) is git's own documented recipe. `git help git`, `GIT_CONFIG_NOSYSTEM`: *"can be used along
with $HOME and $XDG_CONFIG_HOME to create a predictable environment for a picky script"*. Git's test
suite builds its environment the same way. What (iii) changes in kind is the **default**. Under (i)
and (ii) a channel is open until somebody lists it. Under (iii) it is closed until somebody puts it
on the allowlist, and every allowlist entry is pinned by a record that reds when the entry goes
(§6.2). This is the fail-safe direction that `feedback_fail-safe-direction-before-list-completeness`
calls for.

⚠ **(iii) does not close channels that are files at compiled-in paths by itself.** An empty
environment still lets git read `$(prefix)/etc/gitconfig`, `$(prefix)/etc/gitattributes` and the
compiled-in template directory. Each of these has one git-provided off switch or relocation, and
those switches are the allowlist. Section 5 declares what has no such switch. Section 6.1 is what
makes a switch that stops working visible.

### §3.2 The allowlist

`_fgit`'s whole environment is five entries. Nothing else reaches the fixture git.

| entry | why it is in | what removing it does (measured, git 2.55.0 / Apple 2.54.0) |
|---|---|---|
| `PATH=$PATH` | the fixtures must be built by **the same git the wire reads them with**, and the wire resolves `git` through `PATH` (the parent memo's D12 row) | BSD `env -i` without `PATH` falls back to a default path and runs **`/usr/bin/git` (Apple 2.54)** instead of `PATH`'s 2.55: `env -i HOME=$S/v git --version` vs `env -i PATH="$PATH" HOME=$S/v git --version`. A different git builds the fixtures, and nothing reds |
| `HOME=$VOID` | relocates **every** HOME-relative default: `~/.gitconfig`, `~/.config/git/{config,ignore,attributes}`. `XDG_CONFIG_HOME` is not on the list, so `env -i` leaves it unset and git falls back to `$HOME/.config`, which is inside `$VOID` | `HOME` unset is **also** closed on git 2.55 (`git var GIT_CONFIG_GLOBAL` → rc 1). The void is chosen because it is the documented recipe, and because with it any future HOME-relative default lands in a directory this harness owns and asserts empty (§6.1 P-c). "Unset" instead depends on git not falling back to the passwd home, and that fallback cannot be tested without planting files in the real home |
| `GIT_CONFIG_NOSYSTEM=1` | the system config layer. Live on this machine: `git var -l` under `env -i` prints `credential.helper=osxkeychain`, and for Apple git also `init.defaultbranch=main` | `git var GIT_CONFIG_SYSTEM` prints a path instead of nothing (rc 0 vs rc 1) |
| `GIT_ATTR_NOSYSTEM=1` | the system attributes file | `git var GIT_ATTR_SYSTEM` prints a path instead of nothing. ⚠ **Undocumented at 2.55**: `for p in git gitattributes git-config gitignore; do git help -m $p \| col -b \| /usr/bin/grep -c GIT_ATTR_NOSYSTEM; done` → `0 0 0 0`. It is honoured, and `git help git-var` documents `GIT_ATTR_SYSTEM` as the path *"if one is enabled"*, so it is pinned by git's own answer (§6.1 P-b) and never by trusting the name |
| `GIT_TEMPLATE_DIR=$VOID` | the template directory, in all four of its sources (`--template`, this variable, `init.templateDir`, the compiled-in default; `git help git-init`, TEMPLATE DIRECTORY). A template can seed `info/exclude`, `info/attributes`, `hooks/*` **and `config`**: a template holding `config` with `core.excludesFile` hid `victim.py` (measured) | the compiled-in default is copied in: `description`, `hooks/*.sample`, `info/exclude` on both gits here |

`$VOID` is one fresh directory under the wire's own `$SCRATCH` (already `mktemp -d`-checked and
trapped). It is empty by construction and asserted empty after the build. Using one empty directory
for both roles is deliberate: there is one "nothing", and one assertion that it stayed nothing.
An empty `GIT_TEMPLATE_DIR=` string also copies nothing on both gits here, but that behaviour is
undocumented. An empty directory is documented (*"files … in the template directory … will be
copied"*).

**Not on the list, with the reason:**
- `XDG_CONFIG_HOME`: see `HOME`. Setting it would be a second relocation of the same thing.
- `LANG`/`LC_*`: they decide messages, not what gets staged. The controls match the wire's output,
  not git's.
- Commit identity: supplied per call (`_fgit -c user.name=w -c user.email=w@e commit`). Reflog
  identity falls back to the passwd entry and only affects SHAs.
- `_git`'s read switches (`GIT_NO_LAZY_FETCH`, `GIT_NO_REPLACE_OBJECTS`,
  `-c core.fsmonitor=false -c core.untrackedCache=false`): a build has no remote. Replacement is
  irrelevant to `add`/`commit`. fsmonitor and the untracked cache are configuration, and §6.1 P-a
  asserts that no configuration exists outside the fixture. Carrying them "to be safe" would make
  `_fgit` depend on `_git` again (§7).

### §3.3 Shape

```sh
# in the harness — spelled once
_FGIT_VOID="$SCRATCH/fgit-void"          # mkdir, checked; empty by construction
_FGIT_ENV=("PATH=$PATH" "HOME=$_FGIT_VOID" GIT_CONFIG_NOSYSTEM=1 GIT_ATTR_NOSYSTEM=1 \
           "GIT_TEMPLATE_DIR=$_FGIT_VOID")
_fgit() { command env -i "${_FGIT_ENV[@]}" git "$@"; }
```

`command env` rather than `env`, so that a shell function called `env` cannot intercept the call.
No subshell is needed any more, because nothing in the shell's own state is modified. `_fgit` **no
longer calls `_git`** (§7).

**Prototype evidence.** Apply exactly this change to the base, together with the §4-row-R2 fixture
fix (`mkdir -p .git/info` in `notcommitted`), in a `git clone --local` of `e8f78896`. Both shells then
give **rc 0, NE 0, CF 0, `PASSED`** under each of: a clean caller, `HOME/.gitconfig` excludesFile,
`HOME/.config/git/ignore`, `XDG_CONFIG_HOME` ignore, `XDG_CONFIG_HOME` attributes, `GIT_TEMPLATE_DIR`
exclude, the `GIT_CONFIG_COUNT` trio, `GIT_CONFIG_SYSTEM=<hostile>`, and `GIT_DIR=<other repo>`.
**Without** the fixture fix, the prototype is rc 1 with NE 1 (`per-clone info/exclude cannot hide an
entry`) even for a clean caller: that fixture wrote into a `.git/info/` directory that only the
**ambient template** had created (§10 row 2).

## §4 The channel roster: a seed that checks the mechanism, not the mechanism

⚠ **This table is not what closes anything.** §3 closes channels by construction, and this roster
could never be complete, because git reads what git reads. Its job is to supply **hostile inputs**
for the probes in §6.3, so that each allowlist entry has something that reds when that entry goes.
A channel missing from this table is still closed if §3 holds. A channel that §3 fails to close is
supposed to be caught by §6.1's postconditions, which are phrased as properties, before any roster
entry is needed.

"Live" means that, under the constructed environment with this one channel **added inside it**, the
§1 probe's result differs from baseline. Everything was measured on git 2.55.0 in this session.

| # | channel | class | closed by | live? |
|---|---|---|---|---|
| E1 | `GIT_TEMPLATE_DIR` (template `info/exclude`) | env | `env -i` | yes (victim.py hidden) |
| E2 | `GIT_CONFIG_COUNT`/`_KEY_0`/`_VALUE_0` (`core.excludesFile`) | env | `env -i` | yes |
| E3 | `GIT_CONFIG_PARAMETERS` | env | `env -i` | yes |
| E4 | `GIT_CONFIG_GLOBAL=<hostile file>` | env | `env -i` | yes |
| E5 | `GIT_CONFIG_NOSYSTEM=0 GIT_CONFIG_SYSTEM=<hostile file>` | env | `env -i` (and `NOSYSTEM=1` wins over `GIT_CONFIG_SYSTEM`, measured) | yes |
| E6 | `XDG_CONFIG_HOME` → `git/ignore` | env → default file | `env -i` | yes |
| E7 | `XDG_CONFIG_HOME` → `git/config` `init.templateDir` → template | env → config → template | `env -i` | yes |
| E8 | `GIT_INDEX_FILE`, `GIT_DIR`, `GIT_WORK_TREE` | env (routing) | `env -i` | yes (index empty / not a repo) |
| H1 | `$HOME/.gitconfig` (`core.excludesFile`) | default file | `HOME=$VOID` | yes |
| H2 | `$HOME/.config/git/ignore` | default file | `HOME=$VOID` | yes |
| H3 | `$HOME/.config/git/attributes` (`working-tree-encoding`) | default file | `HOME=$VOID` | yes (content) |
| H4 | `$HOME/.gitconfig` `filter.*.clean` + attributes `filter=` | default file → config | `HOME=$VOID` | yes (blob `e69de29b`, empty) |
| S1 | `$(prefix)/etc/gitconfig` | compiled-in path | `GIT_CONFIG_NOSYSTEM=1` | not plantable without writing the machine's prefix; pinned by `git var` (§6.1 P-b) |
| S2 | `$(prefix)/etc/gitattributes` | compiled-in path | `GIT_ATTR_NOSYSTEM=1` | same |
| S3 | the compiled-in template directory | compiled-in path | `GIT_TEMPLATE_DIR=$VOID` | yes on both gits here (non-empty default); pinned by skeleton equality (§6.1 P-d) |
| R1 | a template's `config` / `info/exclude` / `info/attributes` / `hooks/pre-commit` | repository-local, template-seeded | the empty template | yes (each measured) |
| K | any configuration key: `core.excludesFile`, `core.attributesFile`, `core.autocrlf`/`safecrlf`/`eol`, `filter.*`, `core.fsmonitor`, `core.untrackedCache`, `core.sparseCheckout`, `core.hooksPath`, `init.templateDir`, `init.defaultBranch`, `commit.gpgSign`, `index.*`, `include.*`/`includeIf.*`, `safe.directory` … | configuration | **not by key.** Every non-local layer is emptied, and §6.1 P-a asserts that no entry has a scope other than `local`. A key list here would be the enumeration this memo retires | via E2–E5/H1 |
| U | `GIT_ATTR_SOURCE`, `GIT_*_PATHSPECS`, `GIT_TEST_*`, `GIT_EXEC_PATH`, `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_COMMON_DIR`, `GIT_CEILING_DIRECTORIES`, loader variables (`DYLD_*`, `LD_PRELOAD`) … | env | `env -i` | **not probed**. This is the point: they are closed without anybody listing them, and this row is illustrative, not a census |

Rows E1–E8 and H1–H4 become the §6.3 probes. S1–S3 are pinned by §6.1. K and U are closed by
construction and are not probed per member.

## §5 Declared residuals: what no process-level mechanism here closes

| # | residual | direction | where it is owned |
|---|---|---|---|
| R1 | **which git binary** builds the fixtures: `PATH`. A **shim** git (a version-manager wrapper that needs its own variables or a real `HOME`) fails under `env -i` | loud (every fixture fails to build, so NE and red) | the parent memo's D12 row and slot `#11-trip-wire-launch-environment`. Stated at `_fgit` |
| R2 | **filesystem-derived repository config** that `git init` writes by probing the disk: `core.ignorecase`, `core.precomposeunicode` (both `true` on this APFS, absent on ext4), `core.symlinks`, `core.filemode`. Also FIFO support, permission enforcement, and acceptance of raw filename bytes | either | not closable in-process. Two are already reported (`_fifo_ok`, `_perm_line`). P-a admits `local` scope precisely because these entries are legitimate |
| R3 | a **compiled-in path that no variable governs** and `git var` does not report | silent if it exists | **none is known at git 2.55** (§4 found none). Not provable from inside. §6.1 P-a covers any such path that holds *configuration* |
| R4 | the **launch environment of bash itself** (`BASH_ENV`, `SHELLOPTS`, a function called `command`) | whatever the caller makes it | slot `#11-trip-wire-launch-environment` (unchanged) |
| R5 | ownership of `$SCRATCH` by another UID (`safe.directory`; with the global layer emptied, no caller entry can authorise it) | loud (the build fails) | stated at `_fgit` |
| R6 | the **read** of fixtures through `_git` keeps the caller's configuration by design | per `_git`'s contract | `_git`, the `cfgkept` and fsmonitor controls (§7) |
| R7 | **Windows git-bash**: not in the trip-wires matrix (`sed -n '/^  trip-wires:/,/^  [a-z]/p' .github/workflows/ci.yml \| /usr/bin/grep runs-on` → `ubuntu-latest`). `env -i` there also drops `SYSTEMROOT`, which git for Windows needs | unmeasured | declared, not claimed |

## §6 Self-verification

Four instruments, each with one job. Every check below is a **pair**: a liveness half that shows the
thing it asserts about can actually happen, and then the assertion. A check whose liveness half fails
reports `CONTROL NOT EXERCISED`, never a pass. That is the lesson of the `_envlive` misspelling
(`ff6b99a3`'s own commit `009e93db`).

### §6.1 Postconditions: git's own answer about what it would read (property-shaped, always on)

Each one runs once per wire run in a probe repository built by `_fgit`. Each is a *"Not a
`_control`"* block in the controls file with its own `_lbl`, following the existing fsmonitor block's
pattern. The labels live in the controls file because `_mut_correspondence` looks for labels there
and nowhere else (§10 row 8).

| id | label (the needle) | assertion | liveness half | pins |
|---|---|---|---|---|
| P-a | `the fixture git reads no configuration from outside the fixture` | every line of `_fgit config --list --show-scope` in the probe repo has scope `local`; **any other scope, including one git adds later, is red** | the same command with one `-c k=v` prints a `command` line (the parser can see a non-local scope) | the *layers*, by property. It also backs rows K/U |
| P-b | `the fixture git has no system or global layer outside the void` | `git var GIT_CONFIG_SYSTEM` and `git var GIT_ATTR_SYSTEM` print nothing and exit non-zero. `git var GIT_CONFIG_GLOBAL` and `git var GIT_ATTR_GLOBAL` exit 0, and every line they print is under `$_FGIT_VOID/` | the two `SYSTEM` names, asked with `GIT_CONFIG_NOSYSTEM=0 GIT_ATTR_NOSYSTEM=0` appended, **print a path**. This proves this git knows these names, so an older git that does not know them cannot pass as "disabled" | `GIT_CONFIG_NOSYSTEM`, `GIT_ATTR_NOSYSTEM`, `HOME=$VOID` (dropping `HOME` makes the `GLOBAL` names exit 1; passing it through puts them outside the void) |
| P-c | `nothing is written into the fixture git's void` | `$_FGIT_VOID` holds no entries **after the whole build** | an entry planted in a throwaway directory is seen by the same listing command | the void's emptiness, which is what makes `HOME` and the template mean "nothing" |
| P-d | `the fixture git copies no template` | the `.git` listing (`find .git \| sort`) of a probe made by `_fgit init` equals that of `_fgit init --template="$_FGIT_VOID"`. The flag outranks every other template source, so equality means the **effective** template was the void. This avoids an allowlist of skeleton files, which would red on every git release that changes `init` | none needed for the equality itself. The record's discrimination depends on the machine's default template being non-empty (it is on both gits here, measured) | `GIT_TEMPLATE_DIR` |
| P-e | `the fixture git is the git the wire reads with` | `_fgit --exec-path` equals `git --exec-path` | — | `PATH`. ⚠ This discriminates **only where the default-path git differs from `PATH`'s**: yes on this Mac (Apple vs Homebrew), possibly not on the ubuntu runner. It is stated at the record as machine-dependent |

### §6.2 Pins: one mutation record per allowlist entry (control (i) of the brief)

A mutation record is the differential the brief asks for: remove the entry, and the run must red with
the named label. With the entry present, the ordinary run is green. The hostile environment comes
from §6.3's probes, which export it themselves, so no record depends on the caller's machine
**except** where noted. All records are `harness:`-targeted (§9, C4).

| record removes / weakens | must red with | discriminates |
|---|---|---|
| `env -i` → `env` (inherit everything) | §6.3 label | everywhere (every E and H probe) |
| `HOME=$VOID` → dropped | P-b | everywhere |
| `HOME=$VOID` → `HOME=$HOME` | P-b, and §6.3 (H probes) | everywhere |
| `GIT_CONFIG_NOSYSTEM=1` → dropped | P-b | everywhere (`git var` reports the path whether or not the file exists: measured, `/etc/gitconfig` is absent on this Mac and is still printed) |
| `GIT_ATTR_NOSYSTEM=1` → dropped | P-b | everywhere (same) |
| `GIT_TEMPLATE_DIR=$VOID` → dropped | P-d | where the compiled-in template is non-empty (both gits here). Stated at the record |
| `PATH=$PATH` → dropped | P-e | where `env`'s default path finds a different git (this Mac). **On the runner this record may survive.** The mutation run is local-only today (`WEBREF_WIRE_MUTANTS`), so it runs where it discriminates, and the record says so |

### §6.3 Hostile-channel probes (control (ii)): one per roster row E1–E8, H1–H4

A helper in the harness takes one channel (an **array** of assignments, never a word-split string:
`_ctl_env`'s #501 R96 lesson; `$CTL` can hold a space) and runs the §1 probe build three times:

1. **baseline**: `command env -i "${_FGIT_ENV[@]}" git …`, which spells the constructed environment
   **once**, from the same array `_fgit` uses. It must track `victim.py` with its baseline blob.
   (`_envlive`'s second spelling, with `HOME` passed through, is the defect this replaces: §2.)
2. **liveness**: the same, with the channel appended **inside** the constructed environment
   (`env` takes the last assignment: `env -i A=1 A=2 env` → `A=2`, measured on BSD `env`; the GNU
   case is exit criterion E9). The result must **differ** from baseline, or the probe reports
   `CONTROL NOT EXERCISED (… <channel>)`.
3. **scrub**: `( export "${channel[@]}"; _fgit … )`, i.e. the channel in the **caller's**
   environment. The result must **equal** baseline, or it reports
   `CONTROL FAILED (a hostile caller git environment cannot change what a fixture holds): <channel>`.

The result compared is `ls-files -s` plus `ls-tree -r HEAD`, read with `_fgit` in a clean shell.
Paths **and** blobs **and** the committed tree are compared, per §1. The pattern and the file it must
hide are spelled once each (`ff6b99a3` `6ffe2d5d`'s lesson), and so are the channel arrays: the
end-to-end control in §6.4 exports the **same** arrays.

### §6.4 End-to-end (control (iii) of the brief: once through the real path)

The `envscrub` fixture is built in the fixtures file under the **union** of the §6.3 channel arrays,
exported around its build subshell. Every one of those channels has already been shown live. Then
`_control "$CTL/envscrub" 1 "(staged)" "a hostile caller git environment cannot change what a fixture holds"`.
The violation is **staged only**, as in `ff6b99a3`, because the wire's worktree inventory uses
`--exclude-per-directory`, so only staging can hide it. Routing channels (E8) are included. If they
leaked, the build would fail, which is loud.

### §6.5 Portability (control (iii) of the brief)

- **bash 3.2.57 and 5.3**: arrays, `"${a[@]}"` under `set -u` with a non-empty array, `command env`,
  and a function shadowing `git` inside `( … )` and `$( … )` (§7) are all measured on both in this
  session. No `mapfile`, no `declare -A`, no `${!prefix@}`: the `GIT_X-Y` / `${!GIT_@}` divergence
  that `ff6b99a3` spent three commits on **does not arise**, because nothing enumerates the
  environment any more.
- **BSD vs GNU `env -i`**: both are POSIX. Last-assignment-wins is measured on BSD only, and E9 is
  the GNU evidence.
- **sed** (mutation targets): the target prefix must not parse as a sed command. `harness:` is an
  error under BSD sed (measured). A `controls:` prefix is **valid under GNU sed**, because `c` takes
  inline text as a GNU extension (the GNU manual, *a/i/c*; not executed here since there is no GNU
  sed on this machine; BSD rejects it: `sed 'controls:p' </dev/null` → "command c expects \"). So no
  record targets the controls file, and everything a record must pin lives in the harness.
- **git versions**: P-b's liveness half makes a git without the `git var` names refuse instead of
  passing. P-d uses equality, not a skeleton list, so a git that changes what `init` creates does not
  red.

## §7 `_fgit` vs `_git`, and the two carve slots

`_git` (in the wire) **preserves** the caller's configuration and purges only
`git rev-parse --local-env-vars` minus `GIT_CONFIG*`. That is the right policy for reading a real
checkout, which may only be readable through a caller's `safe.directory` (#501 R97). `_fgit`
**constructs** its environment. The two policies share nothing. In the new shape `_fgit` does not
call `_git`, so neither helper passes through the other's environment policy.

**`#11-k2-fgit-keepset-depends-on-git-purge-glob`: DISSOLVES.**
- Premise re-measured: the slot says *"the cheap closure is one sentence at the keep-set naming the
  dependency (taken in this PR)"*. At `ff6b99a3`, `/usr/bin/grep -n -i 'exempt\|held back\|purge' .claude/tools/webref-generic-core-trip-wire.harness.sh`
  returns nothing (rc 1): no such sentence exists. The lane SSoT was right.
- Create-time audit: the defect the slot describes is "a keep-set passed as prefix assignments
  **into `_git`**". There is no keep-set and no call into `_git`. The structural closure the slot
  names (separate the invocations) is what §3.3 does. Nothing remains to defer.

**`#11-k2-fixture-git-invocation-convention`: TAKEN in this PR (commit C6).**
- Premise re-measured: *"the build region contains no bare `git`"*. At `e8f78896`, the build region
  (from `for d in clean pin` to `ctl_ok=0`) has **0** non-comment lines matching
  `(^|[^_A-Za-z$/-])git [a-z-]` other than `_fgit`. At `ff6b99a3` it has **5**, all in `_envlive`
  (commands in §11 E7). ⚠ Those 5 are `env -i … git`, i.e. through `env`, not a bare shell word. That
  matters for the mechanism below.
- Create-time audit: the slot's own trigger ("any change to `_fgit`'s contract") fires in this PR.
- Mechanism: **property over executions, not a grep over spellings.** Immediately before the
  controls source the fixtures file, define `git() { : > "$CTL/.bare_git"; return 97; }`, and
  `unset -f git` immediately after. Every command word `git` the shell resolves during the build hits
  the function, whatever the quoting, line breaks or indirection (`$cmd` with `cmd=git` resolves
  functions too), including inside `( … )` and `$( … )`. Both measured on 3.2 and 5.3. The fixture's
  `&&` chain fails, so its control reports NE. A liveness canary (`( git --version )` with the
  shadow on must create the marker) runs first, and a check afterwards requires the marker to be
  absent: label `a fixture is built only through _fgit`. `_fgit` itself is unaffected because
  `command env` executes the binary (measured).
- **Declared residual of the guard**: it does not see `command git`, `env … git`, `exec git`, an
  absolute path, or `_git` (whose `exec git` bypasses functions). These are deliberate spellings, and
  the guard's comment names them. The probes in §6.3 use `command env -i …`, which is exactly such a
  spelling. That is why they live in the **controls file's assertion phase**, outside the fixtures
  file, so the fixtures file itself can hold no such spelling. A grep for them may accompany the
  guard, and it is labelled as a **seed**.

## §8 The 1000-line split plan

### §8.1 Measured sizes

`wc -l .claude/tools/webref-generic-core-trip-wire*.sh` at `e8f78896`: wire 1259, controls 989,
mutations 783, harness 195. At `ff6b99a3`: controls 1135, mutations 983, harness 364. At
`ff6b99a3`'s first commit `c8f52724`, controls was already 1026:
`git show c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l`. The parent
memo's §11.3 promised the split *"its own commit first"*, and `ff6b99a3` did not take it.

### §8.2 Decisions

| file | split? | seam | why |
|---|---|---|---|
| **controls** (989) | **yes: prereq commit C1** | **build vs assert.** A new `…trip-wire.fixtures.sh` takes the build phase: from the fixture-dir `mkdir` loop (`for d in clean pin …`) to the line before `ctl_ok=0`. That covers every tree, shim and index/worktree disagreement. The controls file keeps the entry contract, sourcing, every `_control` and every *"Not a `_control`"* block (relcwd, fsmonitor, umask, FIFO, perm), the mutation hook-up and the summary | this PR adds to both halves, which projects the file past 1000. The seam already exists as phase order in the file. It gives §7's guard an exact scope (one file is sourced with the shadow on). The harness keeps "how a control runs" |
| **mutations** (783) | **yes: prereq commit C2** | **the two populations**, which the file's own header names (`TWO POPULATIONS, AND THE BOUNDARY IS WHAT EACH ONE'S UNIT IS`). The generated half (`_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`, `_mut_splice`, `_mut_gen_run`) moves to `…trip-wire.mutgen.sh`. The hand records, `_mut_correspondence`, `_mut_trial` (shared) and `_mut_run` stay | `ff6b99a3` carried this file to 983 with the same target infrastructure the rebuild keeps (§9), and the rebuild adds a record per §6.2 row, per §6.3 and §6.4, and per §7. That is past 1000, so the split is taken now and not promised conditionally, which is the failure §8.1 records |
| harness (195) | no | — | stays far from the threshold even with §3.3, §6.3's helper and §7's guard |
| **wire** (1259) | **no, and it is not touched** | — | `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` → 0 at base. `_git` is unchanged. The parent memo's §10.5 cohesion judgement (two predicates over one walk) stands, and a file this PR does not edit triggers no touch-time split |

### §8.3 What each split drags with it (these are the split commits' own checklists)

- **Every sourced sibling is copied beside each mutant.** `_mut_run` copies controls, mutations and
  harness today. The parent memo's §10.5 records that forgetting one made every mutant exit 2. C1 and
  C2 each add a sibling, so C1 replaces the per-file `cp` lines with **one parts inventory**, and a
  guard compares it against the directory:
  `ls "${SELF%.sh}".*.sh` minus `*.mutant.*`, each part named once. An unlisted sibling reds. This
  follows the fail-safe rule: a new file nobody listed must not be skipped silently.
- The controls file's entry contract and the refusal on a missing sibling ("decided nothing", exit 2)
  extend to the fixtures file. The mutations file's `_mut_missing` contract extends to the mutgen
  file.
- The parent memo: its P2 row (§10.x) and its `GIT_TEMPLATE_DIR` row in §11.1 become false once C5
  lands. Each is **replaced in place by a one-line pointer to this memo, with no net growth**. Whether
  a pointer-only edit of a >1000-line plan document triggers that document's own split is put to
  plan-review (§13 Q3). It is not decided here.

## §9 What survives from `ff6b99a3`, and the commit plan

### §9.1 The 14 commits

| commit | subject (short) | fate |
|---|---|---|
| `c8f52724` | scrub the whole `GIT_*` environment | **superseded** (mechanism). Its mutation-target plumbing (`harness:` prefix, `_mut_restore_copies`, the `cmp` fix) **survives**, folded into C4 |
| `4ad051ea` | one target list and a resolver | **survives** → C4 |
| `c5820bd5` | a comment syntax for mutation records | **survives unchanged in intent** → C3 (re-applied onto the split file) |
| `a1007bfb` | ground the keep-set in config layers | **superseded**. Its premise was false: the template's `config` *is* copied into the "local" layer (§3.2, template row) |
| `162edf98` | prove the scrub against `env` | **superseded**: with no sweep there is nothing to prove against `env` |
| `328dc22a` | measure the disjointness behind the refusal | **superseded** (same reason) |
| `c39c7989` | bound the claim about an unknown target prefix | **survives** → C4 |
| `f8c74c6d` | fix two comments | harness half **superseded**, mutations half **survives** → C4 |
| `6b4064fa` | `env -0` per record | **superseded** |
| `009e93db` | liveness before asserting (`_envlive`) | **principle survives** (§6.3 pairs, one spelling). The code is **superseded**: it had its own spelling of the environment, and that spelling leaked `HOME` (§2) |
| `4aced879` | restore/resolve order; say why nothing was applied | **survives** → C4 |
| `2dea82ff` | retire the hostile-build count | **superseded** (the (3') paragraph is rewritten). The principle, "a seed is named as a seed", survives in §4 |
| `6ffe2d5d` | one spelling for pattern and target | **principle survives** in §6.3 |
| `ff6b99a3` | say what the order measurement ran over | **survives** → C4 |

Nothing is cherry-picked blind. C3 and C4 are re-authored on top of C2's split file, so their line
anchors change, and each is re-verified by its own exit criterion.

### §9.2 Commit plan for `k2-wire-fgit-hermetic` (base `e8f78896`)

| # | commit | kind | verified by |
|---|---|---|---|
| C0 | this memo | docs | `/elidex-plan-review` |
| — | **plan-review gate** | | no code before it |
| C1 | split controls → controls + **fixtures** (build phase); parts inventory + sibling guard in the mutation runner | prereq, behaviour-neutral | E1, E4 |
| C2 | split mutations → mutations + **mutgen** (generated population) | prereq, behaviour-neutral | E1, E3, E4 |
| C3 | mutation records may carry a `#` comment | infra | E3 (same verdict lines) |
| C4 | mutation records may target the harness (`_MUT_TARGETS="wire harness"`, `_mut_target`, `_mut_restore_copies`) | infra | E3. ⚠ C4 alone has no `harness:` record yet, so its machinery is first exercised by C5. That is recorded in C4's message, and C5 is the commit where "matched nothing" must be seen to fire on a stale `harness:` anchor |
| C5 | `_fgit` from nothing (§3.3), `notcommitted`'s `mkdir -p .git/info`, P-a…P-e (§6.1), the §6.3 helper + probes, the `envscrub` fixture + control (§6.4), one record per §6.2 row + §6.3 + §6.4, `_MUT_RECORDS_MIN` raised by exactly the number added, parent-memo pointer rows | feature | E1, E2, E3, E5, E6, E8 |
| C6 | the bare-`git` guard (§7) + canary + record | feature (slot taken) | E3, E7 |
| — | `ci.yml` `timeout-minutes`: re-derived, per that line's own rule, if C5/C6 moved the cost | only if needed | E8 |

Then `/pre-push` over `e8f78896...k2-wire-fgit-hermetic`. Name the branch, not `HEAD`: the lane
SSoT records that the review skills resolve `HEAD` in the session's cwd. After that come the PR
(stacked on `webref-cite-audit-tool`) and `/external-converge`. **The 15 IMP / 8 MIN from `ff6b99a3`'s
Stage 5 are not listed individually in the lane SSoT.** Only three co-occurring items and the XDG
hole are recorded. The rebuild's own Stage 5 re-derives them against the new head. They are not
assumed closed.

## §10 Coupled invariants

Axes: **A** enumeration direction · **B** git's config/exclude/attribute/template/hook layers ·
**C** shell and userland portability · **D** the gate's self-verification (controls, records,
correspondence, ratchet) · **E** file structure · **F** the always-run budget.

| # | axes | invariant | where it is enforced |
|---|---|---|---|
| 1 | A×B | `env -i` closes **variables**. **Files** at default locations close only through `HOME`/`NOSYSTEM`/`ATTR_NOSYSTEM`/`TEMPLATE_DIR`. An unknown *config* source is caught by P-a's scope property, and an unknown non-config file is R3 | §3.2, §6.1 |
| 2 | B×D | an empty template removes `.git/info/`, so any fixture that writes into `$GIT_DIR` creates its own directories. Measured: `notcommitted` → NE without it | C5 fixture fix + E1 |
| 3 | A×D | each allowlist entry has a record that reds without it. System layers, which cannot be planted, are pinned by **git's own answer** (`git var`), not by a hostile file | §6.2 |
| 4 | C×D | some pins discriminate only on some machines (`PATH`, `TEMPLATE_DIR`). Each such record says where, and the mutation run is local | §6.2 |
| 5 | C×A | `command env -i` bypasses shell functions, so §7's guard sees only the command word `git`. The deliberate spellings are listed at the guard, and no probe lives in the fixtures file | §7 |
| 6 | C | `env` takes the last duplicate assignment (BSD measured; GNU is E9), which the liveness halves rely on | §6.3 |
| 7 | E×D | every sibling the controls source is copied beside each mutant: one parts list, with an unlisted sibling red | §8.3 |
| 8 | D×E | `_mut_correspondence` looks for labels in the **controls** file only, so every new label (P-a…P-e, §6.3, §7) is defined there even when its helper is in the harness | §6.1 |
| 9 | C×D | a mutation target's name must not parse as a sed command under BSD **or GNU**, so the targets are `wire` (the default, never stripped) and `harness`. No `controls:` target | §6.5 |
| 10 | D×A | every liveness baseline is built from `_FGIT_ENV` itself and never from a second spelling (§2: `_envlive`'s second spelling leaked `HOME`) | §6.3 |
| 11 | B×D | the compared result is paths **and** blobs **and** the committed tree (§1), not the path set | §6.3 |
| 12 | D | `_MUT_RECORDS_MIN` rises by exactly the records added, and `_MUT_UNRECORDED_MAX` does not rise: every new label is named by a record | E3 |
| 13 | F×D | §6.1 and §6.3 add git executions to every always-run wire run. The `ci.yml` timeout line is re-derived with its own command, not with a figure written here | E8 |
| 14 | A×(read side) | the build is hermetic and the read is not, by design. A control that needs the caller's configuration at read time (`cfgkept`, `routed`, fsmonitor) still gets it through `_git`/`_ctl_env` | §7, E1 |

## §11 Exit criteria (commands and expected outputs; run each under `/opt/homebrew/bin/bash` 5.x and `/bin/bash` 3.2)

`W=.claude/tools/webref-generic-core-trip-wire.sh`. Judge by the text of the log, not by `$?` of a
trailing `echo`.

| id | command | expected |
|---|---|---|
| E1 | `( $SH $W ) > $S/l 2>&1; echo rc=$?; /usr/bin/grep -c 'CONTROL NOT EXERCISED\|CONTROL FAILED' $S/l; /usr/bin/grep -c 'trip-wire PASSED' $S/l` | `rc=0`, `0`, `1` |
| E2 | E1 once per caller environment in §2's table plus each §4 row E1–E8/H1–H4 (hostile files under `$S`; never the real `HOME`) | every run `rc=0`, `0`, `1` |
| E3 | `WEBREF_WIRE_MUTANTS=1 $SH $W > $S/m 2>&1` then `/usr/bin/grep -F -e 'entr(ies), 0 not killed as named' -e ', 0 neither killed nor argued equivalent' -e 'trip-wire PASSED' $S/m` | both verdict lines with `0`. The record count equals `_MUT_RECORDS_MIN` after C5/C6. C1–C3 verdict lines are byte-identical to base's |
| E4 | C1 and C2 are behaviour-neutral: E1's log at the parent commit and at the split commit, each with its per-run scratch paths (under `$TMPDIR`) normalised by one `sed`, then `diff` | empty diff, both shells |
| E5 | pin differentials: for each §6.2 row, apply that row's edit to a `git clone --local` copy and run E1 | `rc=1`, and the log contains that row's label. Rows marked machine-dependent: run and record the result, and do not require red |
| E6 | the §1 probe, standalone: a fresh dir holding `victim.py` and `ok.txt`; `init`/`add -A`/`commit` under `command env -i "${_FGIT_ENV[@]}"` with and without each channel; read `ls-files -s` | liveness halves differ, scrub halves are equal (the §6.3 contract, run by hand once) |
| E7 | bare-git census for the fixtures file: `awk '!/^[ \t]*#/' .claude/tools/webref-generic-core-trip-wire.fixtures.sh \| /usr/bin/grep -cE '(^\|[^_A-Za-z$/-])git [a-z-]'`, plus the guard's canary line in E1's log | `0` (a **seed** census, not the enforcement); the guard label absent from failures |
| E8 | `/usr/bin/time -p bash scripts/trip-wires.sh` at base and at head, then `ci.yml`'s `timeout-minutes` re-derived by the rule written beside it | driver `rc=0` with every wire run. The timeout change, if any, is in C5/C6's diff |
| E9 | the PR's CI `Layering trip-wires` job (ubuntu, GNU `env`/`grep`/`sed`) | SUCCESS. This is the GNU evidence for §6.5 |
| E10 | `$SH -n` on each of the wire's `.sh` parts, and `wc -l .claude/tools/webref-generic-core-trip-wire*.sh` | syntax clean on both shells; every part below 1000 |

## §12 Premises of the brief that turned out false or imprecise

1. **"The set of paths `git add -A` stages"** is too narrow. Attributes change blob content, and
   hooks change the commit (§1, measured).
2. **"Nulling global config is exactly what opens this channel"** overstates it. Base is already red
   on it (§2, NE 11). The default ignore path is read whenever no read layer sets `core.excludesFile`.
   Nulling only guarantees that.
3. **"XDG row: 11 (the lane SSoT recorded 12)"**. Both are 12 failing controls: 11 NE plus 1 CF for
   XDG, and 12 NE for HOME. The difference is `_envlive`'s own `HOME` passthrough (§2). It is
   explained, not unexplained.
4. **"`_envlive` adds 5 bare `git`"**: there are 5 lines, but they are `env -i … git`, through `env`.
   A grep counts them, and a function-shadow guard would not (§7).
5. **The keep-set's ground** (`ff6b99a3`: "local … belong[s] to the repository `_fgit` itself
   creates"). A template's `config` is copied into the local layer (§3.2, measured).
6. **The proposed `env -i` shape**:
   (a) *"explicit empty `--template`"* → `GIT_TEMPLATE_DIR=$VOID` in the constructed environment
   instead: one site, covering every `init`, pinned by P-d.
   (b) *"`GIT_ATTR_NOSYSTEM`"* is honoured but **undocumented** at 2.55 (0 hits in four man pages),
   so it is pinned by `git var`.
   (c) *"`XDG_CONFIG_HOME` unset or scratch"* → neither is set: `env -i` leaves it unset and the
   default lands in the void `HOME`.
   (d) `PATH` is load-bearing beyond finding git: without it BSD `env` runs Apple git 2.54 (§3.2).
7. **"Base `_fgit` = unset 3 names"**: base already had `GIT_CONFIG_GLOBAL=/dev/null
   GIT_CONFIG_SYSTEM=/dev/null` beside the three unsets. `ff6b99a3` kept those two as its keep-set.
8. **Not in the brief, found here**: the attributes channel (§2 row 5) and the fixture that depends
   on the template-created `.git/info/` (§3.3).

The line counts, `ff6b99a3`'s 14-commit composition, `c8f52724`'s 1026, both slots' false premises,
and the roles of `4ad051ea`/`c5820bd5` all measured **as stated**.

## §13 Questions for plan-review

- **Q1**: are P-a…P-e worth their always-on cost, or should P-d/P-e run only in the mutation run?
  Draft answer: always on. They are the only pins for the system layers and the template, and an
  unexercised pin is the defect class §6 exists for.
- **Q2**: should C4's machinery land together with its first `harness:` record (i.e. fold C4 into C5)
  to avoid a commit whose code is exercised only later? Draft answer: keep them separate. C4 is pure
  infrastructure with its own exit criterion, and the ordering is recorded in C4's message.
- **Q3**: does a pointer-only, zero-growth edit of the parent memo (>1000 lines) trigger that memo's
  own touch-time split? CLAUDE.md's discipline covers "source・test"; plan documents are not named.
