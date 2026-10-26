# K2 wire CR#1 — the four `mktemp` sites make every scratch directory through one helper, and the exit path removes recursively only what the run made (plan, PR #501)

Slice of PR #501's loop (branch `webref-cite-audit-tool`). It covers **the four `mktemp` sites**: the wire's scratch
root (CR#1) and the three inner sites (NEW-1: controls :222, harness :175, harness :803). **Frame**: every
`file:line` below is read at `67488c8d`, the branch's head, pushed: its merge of `origin/main` at `b9122157` (#547,
#551), on top of `fdded560` (its merge of `2e7c432c`, #546 and #549), on top of `cdc7c6ec` (its merge of A-iii,
#545). None of the three merges changes a line of the wire or its parts (`git diff --quiet 6afa7d98 67488c8d --
'.claude/tools/webref-generic-core-trip-wire*'` holds, measured 2026-10-26), and none changes a doc this memo cites:
`fdded560` changes `scripts/`, two comment lines of `.github/workflows/ci.yml`'s `tools` job (:224–:225) and the
A-iii plan memo; `67488c8d` changes one line of the `tools` job (`actions/setup-python@v7`, ci.yml :244) and adds
`docs/plans/2026-10-event-handler-attrs-at-creation.md` (`git diff --stat fdded560 67488c8d`); so each such cite
equals `cdc7c6ec`'s and `6afa7d98`'s. A number marked **(r4)** is the r4 prototype's
(`scratchpad/nw1-helper-prototype-r4.patch`, made on `fdded560` and applying to `67488c8d`: `git apply --check`), the
tree after this slice; T re-checks it (the delta runbook §3). The last commit below the fix that moves T by landing
:339–:341's definition is `fdded560` (it changes `scripts/`; `67488c8d` changes neither the tool code, `scripts/`
nor the trip-wires job); the new T is the fix commit on top of `67488c8d`. `origin/main` is merged before the fix
commit (those merges are `fdded560` and `67488c8d`), and not between the fix commit and the end of step 1, X9 and
Codex
(§6). "The wire" is `.claude/tools/webref-generic-core-trip-wire.sh`, "controls" etc. its parts
`…trip-wire.<part>.sh`; "landing", "runbook" (`-cr2-plan-finalhead-runbook`), "finalhead", "impl-d"
(`-impl-cr2-d`), "residuals", "corpus", "census memo" (`-census`) are `2026-09-citation-hygiene-k2-fgit-hermetic-*`
memos, and "design memo" is `2026-09-citation-hygiene-k2-fgit-hermetic.md`. Findings:
`elidex-fgit-handoff/cr2/CLASSIFICATION.md` :16, :36. R1 (2026-10-25): CRIT 0, IMP 7, MIN 16, applied in r2. R2:
CRIT 0, IMP 11, MIN 19, applied in r3 (PM decisions 20, 22, 26: D1, the root's template and one helper mode).
R3 (2026-10-26): CRIT 0, IMP 7 (6 distinct), MIN ≈ 22; r4 applies PM decision 27 and the user's decision of
2026-10-26: the own-dir census leaves CR#1 for the program (IMP A; §4, §5), the helper refuses an answer that is a
symbolic link (IMP B, the user's option b1; §3.1), and every fact, procedure and self-added MIN is fixed. r4 is the
terminal revision: one reviewer attests it by enumeration (PM decision 27).

**Scope decision.** User decision 2026-10-25, relayed by PM session `main-e7`: #501 fixes CR#1 only; every other
base defect goes to the program slot `#11-k2-wire-base-defect-program` (owner CI/tools lane), which the PM registers
**before #501 merges** (M8); its trigger has fired (A-iii landed, #545 `ad80b5ca`), and the lane starts it after
#501 lands. PM decision 15, 2026-10-25: NEW-1 is folded into CR#1 (same root cause, an unchecked `mktemp` answer;
same harm class). PM decision 22: a verdict site CR#1 changes or adds gets its control inside CR#1 (§4). PM decision
27: the always-on own-dir census (r2–r3) is not in CR#1; the defect it addressed is a program member (§5), and the
blind spot CR#1 then leaves is stated (§4). §5 lists what is adjacent and why it stays out.

---

## §1 The defect, measured

**Root cause, all four sites.** Each site takes `mktemp`'s answer as "a directory this run made" and checks at most
its exit status, emptiness and `-d`. Nothing establishes that this run made the directory, or that it lies where the
template put it. The scratch root's answer becomes the EXIT trap's `rm -rf` root (`_k2_exit`, wire :515–:526, `/*/*`
at wire :519–:520: a type check, not an ownership check); the inner answers become the places the run writes its fixture
tree, its verifier files and its postcondition files, and where it removes named entries.

**The scratch root** (wire :400–:404; only the exit status is checked at :401). Measured end to end on this machine at
the base (victims under the scratchpad), re-measured 2026-10-26 at `fdded560` (wire and parts identical to `67488c8d`'s) by the N5 cell's script
(`scratchpad/cr1r4/bin/n5.sh`; rows in `cr1r4/log/batchX.<sh>.out`, logs `cr1r4/w/n5/base.*`):

| mktemp answer (exit 0) | `_phys` gives | bash 3.2.57 | bash 5.3.20 |
|---|---|---|---|
| empty | `cd ""` | the cwd, rc 0 → **cwd removed** (rc 2) | rc 1 → "could not resolve" (rc 2, kept) |
| `.` | the cwd | **cwd removed** (rc 1) | **cwd removed** (rc 1) |
| `-`, `OLDPWD` exported | 3.2: `cd -` fails (a 3.2 shell does not inherit `OLDPWD`); 5.3: `cd -` prints the directory, so the value is two lines, not a directory | "could not resolve the scratch dir (-)", rc 2; nothing removed | the run continues and fails at harness :54 (`mkdir "$_FGIT_VOID"`: no such directory), rc 2; nothing removed |
| `rel`, `CDPATH` exported, `rel` a directory on it | the `cd` prints the directory: two lines, not a directory | fails at harness :54, rc 2; nothing removed | same |
| an existing directory outside, holding `sentinel` | its physical path | the run works in it, and the EXIT trap removes it: **outside directory removed** (rc 2) | same |

With cwd = the checkout root, the in-tree refusal (wire :547–:554) exits 2 *after* the trap is installed, so the trap
removes the root, `.git` included. The empty answer destroys only under bash < 5.3; the `.` answer and an existing
outside directory under both. `-` and a `CDPATH` hit destroy nothing at the base: each is loud (rc 2) for a
misleading reason (NEW-2, §5). A failing `mktemp` (exit non-zero) is refused at wire :401 (`no scratch dir`, rc 2).

**The inner sites at the base**, from the cell script of §6 N6 (a full run from a victim cwd holding `sentinel`; the
shim answers badly only for its site's template; `out` is an existing directory holding `sentinel`):

| site | (a) empty | (b) `.` | (c) `out` |
|---|---|---|---|
| controls :222 `_VFY` | refused (rc 2, `[ -z ]`) | rc 1; the victim cwd gains `.reap`, `.registry`, `.census`, `fsmhook` and 7 more | **rc 0 PASSED**; `out` gains 12 such files (`ls -A out \| wc -l` − 1, verified 2026-10-25) |
| harness :803 `CTL` | refused (rc 2) | rc 1; nothing outside the scratch | **rc 0 PASSED**; `out` gains the whole fixture tree (97 entries, `ls -A out \| wc -l` − 1, verified 2026-10-25), sealed modes included (`rm -rf` alone cannot remove `walk/sub`) |
| harness :175 `_pq` | refused (rc 1) | rc 1; the window's own directory (in the scratch) | **rc 0 PASSED**; `out` gains the postconditions' 17 working files (same count, verified 2026-10-25) |

Identical on both shells (verified 2026-10-25 at `6afa7d98`, whose wire and parts are `67488c8d`'s). So (a) is
already refused at every inner site; (b) and (c) write outside the run's own directory, and (c) ends green over a
run that wrote into a directory it did not make.

## §2 Coupled invariants

Edge-dense: four sites × answer shapes × two shells, across the exit path (CR#11/CR#12's surface, #527), the
fixture window's prelude, the record census (ratchet, floor, anchor rule) and the control registry.

| # | invariant | where it is checked |
|---|---|---|
| I1 | **One way**: outside the run's own directories, the wire and its parts name a place from a temporary-name source (`mktemp`, a pid, `$RANDOM`, a clock) or under a temporary root (`TMPDIR`, `/tmp`, `/var/`, `/dev/shm`) only through `_k2_own_dir` (the four sites call it; no site checks an answer itself). One place is named otherwise: the opt-in run's copies of the parts, by pid, beside the wire (`_mut_part_path "mutant.$$"`, mutations :487). Names inside the run's own directories are not this invariant's: the `_og` nodel stub's pid-named reap entry (controls :513 (r4)) and `_og_trap`'s `.og_scr` (I2) | **no always-on check** (PM decision 27; §4's blind spot): a raw `mktemp` put back at a site stays green on every run. Measured once, at r4: an over-inclusive token grep over the wire and its parts, comment lines out, each of its 47 lines read (`scratchpad/cr1r4/log/i1-tokens.txt`); the other hits are message texts and labels, pid reads for `kill`/`ps`, channel values a child's own helper call reads, two fixture file contents and the records' expressions. Containment is checked at T (N5, N6, `staleshim`); the new form is fenced by the helper's records (R1–R10, R13, R14); the call sites are review's |
| I2 | **Removal** (the exit path, `_k2_exit`): its recursive removal reaches only `SCRATCH`, a `run` directory this run's own `mkdir` (no `-p`) made, and the inner directories inside it; its one other removal is the `rmdir` of the scratch root's answer, and only of an answer that passed the containment test (I3). Qualifier: the `_og` sig row's `_og_trap` (controls :517) points `SCRATCH` at `$_VFY/.og_scr`, which it makes with `mkdir -p` at a fixed name inside the verifier's directory, and the traps it installs remove that recursively: a directory inside the run's own, not one an answer named | C-b, C-c, relcwd; R5, R6, R7, R8; the `_og` sig row (I11) |
| I3 | **Containment, all four sites**: `<dir>` is resolved physically once; an answer is used only if its text is `<that dir>/<prefix>` plus a non-empty name with no `/`, and it is not a symbolic link (so it is physical too); checked before anything is touched | C-d, U (`dot`, `old`), R4; C-f, R14; U `unres`, R10; N5, N6 (b), (c), `staleshim` (at T) |
| I4 | **Fail direction**: an answer the helper cannot place is refused (empty variable, a reason) and nothing is removed for it; the one residual is §3.1's `rmdir` | C-a, C-d, C-e, C-f, U; §1's, §3.1's and §4's victim runs |
| I5 | **Status**: nothing in the exit path changes the status it was given (CR#11/12): errexit off, every test under `${…:-}`; the helper runs only as an `\|\|`/`if` operand or inside a command substitution (U's call), neither of which inherits errexit: the wire (`set -euo pipefail`, :311) never sets `inherit_errexit` (M3) | C-a..C-f pose the new branches under the wire's own `set -u`; R2 (the root call bare) is killed; X13 (§6) poses the changed exit path under nounset off and on (M16) |
| I6 | **Line**: every exit-path line goes to fd 9 (:427–:429's rule, fd 9 opened at :438) | review of the diff |
| I7 | **Order**: the scratch root's helper call happens before the trap is installed (:393–:399's PR519 rule, restated §3.1), its refusal after it | relcwd (controls :701–:712, re-derived), R9 |
| I8 | **Window**: the helper reaches the fixture window (a shell started under `env -i`, harness :653, whose site is harness :175) through the prelude's `declare -f` list (harness :640), with `_phys` (the helper resolves `<dir>` with it); the window has no `SCRATCH`, so its site passes `${_FW_DIR%/*}` | N1 (P-* green), N6 `pgref` |
| I9 | **Census of records**: every new label has a record (`_MUT_UNRECORDED_MAX=18` stays exact, mutations :271), every record is fresh, the floor moves 305 → 316 (mutations :275) | the always-on half, every run |
| I10 | **Needles**: each new message text occurs on exactly one wire line (the helper's, or the exit path's) besides its controls' lines and the record lines that quote it | §4's grep, at implementation |
| I11 | **Shared trap**: `_og_trap` (controls :517) installs the wire's traps for its own scratch, so it states both names the exit path reads | the `_og` sig row's assertion (M1), R12 |
| I12 | **Fixture space**: every new control's child or call runs from a fixture cwd or is handed an absolute fixture path, and every fixture it reads is asked of a gate (M2); the helper's block runs after C-b | `_ctl_run`/`_ctl_gate` (§4); the order: review |

**Pairwise** (M7): the pairs that share a line or a value, and what checks the pair, not each member.

| pair | where they meet | check |
|---|---|---|
| I1 × I8 | the window's copy of the helper is `declare -f` text, not a part line | N6 `pgref` (its wiring), at T only |
| I2 × I4 | a root call refused after the containment test leaves `_K2_SCR_TOP` set (a taken `run`, an unwritable answer) | C-c and the umask control (the line), the exit path's `rmdir` (a non-empty answer is kept and said) |
| I2 × I7 | the trap's `rmdir` reads `_K2_SCR_TOP`, set before the trap | relcwd, R7, C-b |
| I2 × I11 | the `_og` sig row's trap must not reach the wire's own top | M1's assertion, R12 |
| I3 × the in-tree check | the in-tree check (wire :539–:554) compares `SCRATCH` as text with the root resolved; `SCRATCH` is physical only because `<dir>` is resolved and the answer is not a symbolic link | C-f, R14; the in-tree control; the symbolic-link run (§3.1) |
| I3 × I8 | `<dir>` is `${_FW_DIR%/*}`, empty if `_FW_DIR` is; bash < 5.3 would resolve an empty `<dir>` to the cwd | the helper refuses an empty `<dir>` before `_phys`: U's `unres` row, R10 |
| I4 × I5 | a refusal returns 1 under errexit | C-a..C-f (status and line), R2, X13 |
| I5 × I6 | the kept line goes to fd 9 inside the trap | C-b reads it in the child's output; X13 |
| I2 × I12 | C-b leaves `odtmp/k2scratchnamed` holding `keep`, which U then reads | **review only** (the order in the text) |
| I6 alone | fd 9 for the new line | **review only** |
| I10 alone | needle uniqueness | **review only** (§4's grep, run once at implementation, not always on) |
| I1 alone | the form of the four sites | **no always-on check** (§4's blind spot) |

## §3. Spec coverage map

No web spec. The behaviour rows are POSIX/bash facts this memo measures (§1, §3.1, §4); the table form is the one
`preflight.py` accepts (every row carries a `§`).

| Spec section | Step | Branch | Touch | Full enum | User-input flow |
|---|---|---|---|---|---|
| POSIX/bash utility behaviour — this memo §1 | `mktemp`'s answer at the root | `""`, `.`, `-`, relative (`CDPATH`), an outside directory, a non-zero exit; a symbolic link inside `TMPDIR` | wire :400–:404 | ✓ six answers, both shells, end to end, at the base and the fix (§1, §4); the link at the fix and without its check (§3.1) | yes: mktemp's answer, the caller's `PATH`/`CDPATH`/`OLDPWD`/`TMPDIR` |
| POSIX/bash utility behaviour — this memo §3.1 | `cd` in `_phys` on `<dir>` (`${TMPDIR:-/tmp}` at the root) | absolute, relative, relative on `CDPATH`, `-`, absent, empty | the helper | ✓ measured both shells (§3.1) | yes: the caller's `TMPDIR`, `CDPATH`, `OLDPWD` |
| POSIX/bash utility behaviour — this memo §1 | an inner answer that ignores its template | empty / `.` / a path outside `<dir>` | controls :222, harness :175, :803 | ✓ three shapes × three sites, both shells | yes: the caller's `PATH` |
| POSIX/bash utility behaviour — this memo §3.1 | `[ ! -L ]` on the answer, and `mkdir` without `-p` inside it: the proof of creation | a link / created / EEXIST / EACCES (umask 777) | the helper | ✓ | no |
| POSIX/bash utility behaviour — this memo §3.1 | `rmdir` of the root's answer: the exit path's second removal | removed / ENOTEMPTY (kept, said) / absent / `run` not removed (silent: the removal said it) | `_k2_exit`, wire :519–:520 | ✓ | no |
| none (this memo) §4 | tests | C-a..C-f, U, relcwd, in-tree, umask; R1..R10, R12..R14 | controls, fixtures, records | ✓ every verdict site CR#1 changes or adds has a control (§4's table); the form of the sites has none (§4) | no |
| none (this memo) §6 | T and landing | N1..N6, the RUN cells at the new T, X9, Codex | landing §9.1, 7.4; the delta runbook | ✓ every cell of finalhead §1.1 re-ruled (§6's table) | no |

### §3.1 The fix

**One helper, one mode, defined in the wire beside `_phys` (after :400):** `_k2_own_dir <var> <dir> <prefix>`
(wire :411–:421 (r4)).

1. Its first `printf -v` empties `$<var>`: the one writer of the refused value (M4).
2. `<dir>` is resolved physically, once: `[ -n "$2" ] && _od_d="$(_phys "$2")" && [ -n "$_od_d" ]`, else refused
   (`could not resolve the directory to make it in ([<dir>]) to a physical path`). The empty test comes first:
   bash < 5.3 would answer `_phys ""` with the cwd.
3. The one `mktemp` call of the wire and its parts: `mktemp -d "$_od_d/${3}XXXXXX"`. A failed call, or an empty
   answer, is refused (`no directory from mktemp (TMPDIR/disk?)`; `mktemp exited 0 but printed no path`).
4. The answer's **text** must be `<resolved dir>/<prefix>` followed by a non-empty name with no `/` (`case
   "${_od_a#"$_od_d/$3"}" in "$_od_a"|""|*/*) refuse`), else `mktemp answered [<a>], not a new name inside <resolved
   dir>`.
5. **Then the answer must not be a symbolic link** (b1, the user's decision 2026-10-26): `[ ! -L "$_od_a" ] || refuse`
   with `mktemp answered [<a>], a symbolic link, not a new directory`. Nothing is touched before this test; only then
   is `_K2_OD_TOP` the answer. **Why `-L`, not `_phys "$answer"` equal to the text**: `<resolved dir>` is physical and
   the name has no `/`, so the last component is the only one that can be a link, and `-L` tests exactly it; it needs
   no fork and no search permission on the answer. A `_phys` comparison needs both: under umask 777 `mktemp` makes a
   mode-000 directory, `cd` cannot enter it, and `_phys` answers nothing (measured, both shells:
   `scratchpad/cr1r4/log/umask-phys.txt`), so it would refuse the umask control's answer for the wrong reason.
6. `mkdir "$_K2_OD_TOP/run"` (no `-p`) or refuse with `could not make this run's own directory inside <top>`; then
   `$<var>` = `<top>/run`. A refusal leaves `$<var>` empty, the reason in `_K2_OD_WHY`, and removes nothing.

So `SCRATCH` is physical: `<dir>` is resolved by `pwd -P`, the answer's name has no `/` and is not a link, and `run`
is a directory this run's own `mkdir` made. **The symbolic-link answer, measured** (`scratchpad/cr1r4/bin/sym.sh`,
both shells, 2026-10-26: `TMPDIR` holds `k2scratchLNK`, a link to a directory inside the checkout the run scans, and
a shim answers the scratch root's template with it): at r4, rc 2, `!! mktemp answered [<TMPDIR>/k2scratchLNK], a
symbolic link, not a new directory, so nothing here was proved`, the link and its target (`sentinel` only) untouched
(`cr1r4/w/sym.{b32,b53}/log`); with the check removed, the parent's in-tree check passes on the link's text (no
`INSIDE`), every child gets the same answer and the controls go red (rc 1), nothing is lost, and the exit path keeps
the link (`left … in place`) (`cr1r4/w/symnochk.{b32,b53}/log`).

Called only as an `||`/`if` operand or inside `$( … )` (I5), so errexit is off inside it; D5's `|| SCRATCH=""` guard
(wire :402–:404, review-rounds §11) has no job left: the root call is `_k2_own_dir SCRATCH "${TMPDIR:-/tmp}" k2scratch ||
:` (the `||` keeps it an operand, which records :98, retargeted, pins: R2), and the helper is the writer.

**The sites** (each message keeps its site's own text and appends the reason):

| site | becomes |
|---|---|
| wire :401–:404 | `_k2_own_dir SCRATCH "${TMPDIR:-/tmp}" k2scratch \|\| :`; `_K2_SCR_TOP="$_K2_OD_TOP"` — before the trap (I7) |
| wire :534–:538 | one guard after the trap: `if [ -z "$SCRATCH" ]; then echo "!! $_K2_OD_WHY, so nothing here was proved" >&2; exit 2; fi` |
| controls :222–:223 | `if ! _k2_own_dir _VFY "$SCRATCH" verify \|\| ! mkdir "$_VFY/.reap"; then` … `(${_K2_OD_WHY:-its .reap could not be made})` |
| harness :803–:804 | `if ! _k2_own_dir CTL "$SCRATCH" ctl; then` … `($_K2_OD_WHY)` |
| harness :175–:176 | `_k2_own_dir _pq "${_FW_DIR%/*}" pgref \|\| { echo "!! CONTROL FAILED ($_pg_lbl): … made fresh ($_K2_OD_WHY)" …` |
| harness :640 | `declare -f _phys _k2_own_dir _fixture_failed …` (I8) |

(`\|` is markdown escaping.) **Comments made false by the fix, re-pointed in the same commit**: "by `mktemp -d`" at
controls :216, harness :168 and harness :800–:802 names the helper; the wire's PR519 comment (:393–:399) is restated
(the template's directory is `${TMPDIR:-/tmp}` resolved physically, before the trap, so the answer is absolute and
physical); controls :219–:221's class-(c) clause becomes "an executable written into a caller-`PATH` directory still
changes which `mktemp` and `mkdir` run (an answer outside `$SCRATCH` is refused: `staleshim`)" (controls :222–:224
(r4)); `_scan`'s "`$SCRATCH` is already this PROCESS's own `mktemp -d`" (wire :1116) names the helper (at r4,
wire :1139 (r4)); and the fixtures' comment on the deleted `fakemktemp` shim (fixtures :335–:339, "macOS's `/usr/bin/mktemp`
ignores `TMPDIR` when given no template, which is why the control shims the command") is deleted with it.

**Exit path** (`_k2_exit`, after wire :520; at r4, wire :540–:544 (r4)):

```sh
  # ACCEPTED DEFECT (CR#1; residuals R24): this `rmdir` takes the scratch root's `mktemp` answer, which passed the
  # containment test, so a broken `mktemp` naming an existing EMPTY `<physical TMPDIR>/k2scratch…` loses it; never
  # removing it would leave one empty directory per run. Silent while `run` is still there: the line above said why.
  case "${_K2_SCR_TOP:-}" in /*/*) rmdir "$_K2_SCR_TOP" 2>/dev/null || [ ! -d "$_K2_SCR_TOP" ] || [ -e "${SCRATCH:-}" ] \
    || echo "!! left $_K2_SCR_TOP in place: the directory mktemp named holds more than this run made" >&9 ;; esac
```

The recursive removal at wire :519–:520 is unchanged in text; what changed is what can reach it. The inner directories lie
inside `SCRATCH`, so its removal takes them. `[ -e "${SCRATCH:-}" ]` keeps the line from giving a wrong reason: when
`rm -rf "$SCRATCH"` fails, `run` is still in the top, the `rmdir` fails for that, and the removal's own line
(`could not remove the scratch dir …`) has already said why; a refused root call leaves `SCRATCH` empty, so
`[ -e "" ]` is false and C-c's line stays. Measured on both shells with the exit path's lines (wire :538–:544 (r4))
run alone (`scratchpad/cr1r4/bin/rmfail.sh`, results `cr1r4/log/rmfail.txt`): `run` removable → the top removed, no
line; `run` holding a user-immutable file → only `could not remove the scratch dir …`, the top kept; the top holding
another file → only `left … in place: … holds more than this run made`. **`_og_trap`** (controls :517) adds
`_K2_SCR_TOP=""` beside its `SCRATCH=` (I11), and the sig row asserts it (M1): `case "$_og_st" in *" in place: "*)
_og_bad=…` — without the reset the row's subshell tries the wire's own `mktemp` directory and prints the kept line
into `_og_st`, and the row's existing match (`*"stopped by SIGTERM"*"ended 143"`) still passes.

**One mode, all four sites (D1, PM decision 22).** The root's template is explicit, `${TMPDIR:-/tmp}/k2scratchXXXXXX`
with the directory resolved physically, so the root has a `<dir>` like the inner sites and the containment test
applies to all four; the r2 helper's no-`<dir>` mode, its resolution of the answer, its own `rmdir`, R10's old
target (the M6 guard) and U's `nodir` row are deleted. What changes, measured on this machine (GNU coreutils
`mktemp` first on `PATH`, BSD `/usr/bin/mktemp` behind it):

- **`TMPDIR` unset**: today `mktemp -d` picks `/tmp` (GNU) or the per-user `/var/folders/…/T` (BSD); with D1 both
  use `/tmp` (resolved: `/private/tmp`), a sticky directory where `mktemp -d` still makes a fresh 0700 name.
  Measured at r4 (`scratchpad/cr1r4/bin/final2.sh`; each run made alone, with no other run of the wire live, from an
  empty cwd outside the checkout): a full run with `TMPDIR` unset PASSED on both shells and
  left no `k2scratch…` in `/private/tmp` (none before, none after) and nothing in its cwd (`cr1r4/log/unset.{b32,b53}.log`); a run
  with BSD `mktemp` first on `PATH` PASSED on both shells (`cr1r4/log/bsd.{b32,b53}.log`).
- **A relative `TMPDIR`** is read against the cwd and resolved before `mktemp` runs, on every `mktemp` (BSD's
  template-less call ignored it; GNU's answered a relative path, PR519). The PR519 rule (wire :393–:399) stands in its
  stronger form: the template is absolute and physical before the trap is installed. The relcwd control poses it
  with the real tool (below). **A caller's relative `TMPDIR` reaches every child too**, and a child run from its own
  cwd resolves it there: measured on the first r3 tree, a full run with `TMPDIR=rel` went red, rc 1, `CONTROL NOT
  EXERCISED` for C-a and C-d (each child resolved `rel` in its own cwd, found nothing, and was refused before its
  shim ran), where the base PASSED on both shells. C-a and C-d therefore hand `TMPDIR=.` (their gated cwd); with it
  the same run PASSED on both shells, and so did the plain run (`scratchpad/r3log/batchD.{b32,b53}.out`). **Every
  child, enumerated** (controls at r4): run in the parent's cwd, so `rel` resolves where the parent resolved it —
  every `_control` not named below (ctlrun :349), the standing checks' `_ctl_run`s (`_wm`, `_ev`, the capability
  rows, the `_og` rows: controls :363, :407, :464, :525 (r4)), the fsmonitor block (controls :919 (r4)) and the opt-in
  mutation trials (`$BASH "$_mut_wire"` with no `cd`, mutations :402); run in their own cwd and handed their own
  `TMPDIR` — relcwd (`reltmp`), C-a and C-d (`.`); run in the parent's cwd and handed their own `TMPDIR` — C-b, C-c,
  C-f (`$CTL/odtmp`) and in-tree (`$CTL/inscope/scr`); in-process with an explicit `<dir>`, so `TMPDIR` is not read —
  U (`$CTL/odpar` or empty), the verifier's and the controls' directories (`$SCRATCH`) and the window's
  (`${_FW_DIR%/*}`).
- **The relcwd (controls :701–:712) and in-tree (controls :810) controls** lose their shims: `fakerelmktemp` answered a
  relative name and `fakemktemp` a path under `inscope`, whatever the template, and with D1 both answers are refused
  by the text test, for a reason that is not the control's question. Each is handed a `TMPDIR` channel instead and
  runs the real `mktemp`: relcwd `TMPDIR=reltmp` (relative, inside `relcwd`): rc 0 and `reltmp` left empty; in-tree
  `TMPDIR=$CTL/inscope/scr` (inside the scanned tree): rc 2, `INSIDE the tree`. `_ctl_run` demands each channel back
  (`env.TMPDIR`), as it demanded the shim's `path`.
- **The in-tree check** (wire :539–:554) still runs, on `SCRATCH`, which is physical (above).
- **The umask control** (controls :903): under umask 777 `mktemp` makes a directory this run cannot enter, so `mkdir
  run` fails: the needle becomes `could not make this run's own directory`, and the exit path's `rmdir` removes the
  empty mode-000 answer (`rmdir` needs only the parent's permission). The base needle `could not resolve the scratch
  dir` stays only in wire :543's message.

**Why not the trap guard.** Tightening `/*/*` (refuse the cwd, ancestors of `ROOT`, `/`-ish) is a list of known bad
values: the next one (`$HOME`, a sibling checkout, any existing directory a broken `mktemp` names) falls to "delete".
Only the producer knows what it made, at the moment it makes it.

**`_phys` itself is not changed** (F2). It is reached by `ROOT` (wire :541) and by each site's `<dir>` (the scratch
root's `${TMPDIR:-/tmp}`; `$SCRATCH` for the verifier's and the controls' directories; `${_FW_DIR%/*}` for the
window's), never by an answer. An odd `<dir>` gives two lines (a relative `TMPDIR` found on an exported `CDPATH`;
`TMPDIR=-` under 5.3, through `OLDPWD`) or nothing (`-` under 3.2; an absent directory); the inner `<dir>`s are this
run's own absolute, physical directories, so only the root's can be odd. Measured at r4, both shells
(`scratchpad/cr1r4/bin/final2.sh`, the cases of `scratchpad/d1root/tmpdir.sh`; results `cr1r4/log/tmpdir.txt`): the
two-line value (`TMPDIR=rel` with `rel` on an exported `CDPATH`, both shells; `TMPDIR=-` with `OLDPWD` under 5.3)
makes `mktemp` fail, rc 2, `no directory from mktemp (TMPDIR/disk?)` (misleading, loud); the empty one (`TMPDIR=-`
under 3.2) and an absent directory are refused, rc 2, `could not resolve the directory to make it in ([…])`; nothing
is removed. An answer of `-` or `rel`
no longer reaches `_phys` at all: it is refused by the text test (§4's victim runs). The misresolution of `<dir>`
stays NEW-2 (§5).

**Accepted defect: the residual `rmdir`** (driver, 2026-10-25; M5; re-derived for D1). The helper removes nothing.
The exit path `rmdir`s `_K2_SCR_TOP`, which is set only for an answer that passed the containment test, a name
`<physical TMPDIR>/k2scratch<name>` that is not a link; a recursive removal reaches only the `run` the run's own
`mkdir` made. So the one way to lose something the run did not make is a broken `mktemp` that names an existing
**empty** directory of exactly that form: it is `rmdir`ed. The cwd, `.`, `-`, a relative name, an outside directory
and a link are refused before anything is touched (§3.1, §4, the victim runs). The alternative, never `rmdir`ing the
answer, leaves an empty directory in `TMPDIR` after every normal run, since the run's own top is what that `rmdir`
removes; so the harm is bounded to one empty directory under `TMPDIR` with this wire's prefix, and the alternative
costs every run. **Its durable homes**: the code comment at the exit path (above), and a residual row the fix commit
adds to residuals §5.2, after R23:

> | R24 | **the exit path's `rmdir` of the scratch root's `mktemp` answer** (`_k2_exit`, the wire; the CR#1 plan §3.1). The answer passed the containment test (a new name `<physical TMPDIR>/k2scratch<name>`, not a symbolic link), so a broken `mktemp` that names an existing EMPTY directory of exactly that form loses it; anything else is refused before the run touches it, and a non-empty answer is kept and said (`left … in place`). Origin: #501's CR#1 | silent: one empty directory under `TMPDIR` with the wire's prefix | **accepted defect** (the CR#1 plan §3.1): never removing the answer would leave one empty directory per normal run |

The same commit extends the memo's residual ranges to R24: `"R17"–"R23"` in its header (residuals :7) and `R17–R23`
twice in §5.1's **Accounting** (residuals :94, :96).

## §4 Tests

**Fixtures** (after fixtures :359; none joins the two fixture-name lists at fixtures :36 / :462, like `relcwd`):
`inscope/scr` and `relcwd/reltmp` (the two channels' `TMPDIR`s), `mtcwd/keep`, `mtdot/keep`, `odcwd/keep`,
`odtmp/k2scratchnamed/keep` (one loop), `odtmp/k2scratchcoll/run/`, `odlnk/` and `odtmp/k2scratchlink` (a symbolic
link to `$CTL/odlnk`; one line with `k2scratchcoll/run`, `|| _fixture_failed odtmp`), `odpar/` (empty), each `||
_fixture_failed <name>`; six `mktemp` shims built in one loop, each marking `path` (`${K2_CTL_REACHED:?}/path`) as
the shims do: `fakenomktemp` prints nothing, `fakefailmktemp` exits 1, `fakedotmktemp` prints `.`, `fakeoldmktemp`
prints `$CTL/odtmp/k2scratchnamed`, `fakecollmktemp` prints `$CTL/odtmp/k2scratchcoll`, `fakelnkmktemp` prints
`$CTL/odtmp/k2scratchlink` (through `_shq`). The shims `fakemktemp` and `fakerelmktemp` (fixtures :348–:359) are
deleted with their only callers' shim arguments, and `fakemktemp`'s comment (fixtures :335–:339) with them.

**Controls** (C-a..C-f after the controls relcwd block, :712, in this order; relcwd, in-tree and umask in place):

| id | site, shape | call | expect | label |
|---|---|---|---|---|
| relcwd | root, relative `TMPDIR` (re-derived) | `_ctl_run "$_rel_lbl" "$CTL/clean" "$CTL/relcwd" "" "" "" "" "TMPDIR=reltmp"` + own check | rc 0; `relcwd` holds only `reltmp`, which is empty | `_rel_lbl` (unchanged) |
| C-a | root, (a) | `_ctl_run "$_mtn_lbl" "$CTL/clean" "$CTL/mtcwd" "" "$CTL/fakenomktemp" "" "" "TMPDIR=."` + own check | rc 2, `printed no path`, `ls -A mtcwd` = `keep` | `_mtn_lbl` "a mktemp that prints no path decides nothing and removes nothing" |
| C-d | root, (b) | `_ctl_run "$_dot_lbl" … "$CTL/mtdot" "" "$CTL/fakedotmktemp" "" "" "TMPDIR=."` + own check | rc 2, `not a new name inside`, `ls -A mtdot` = `keep` | `_dot_lbl` "a mktemp that answers the cwd costs the cwd nothing" |
| C-e | root, non-zero exit | `_control "$CTL/clean" 2 "no directory from mktemp" … "$CTL/fakefailmktemp"` | rc 2, the reason | "a mktemp that fails decides nothing" |
| gate | — | `if ! _ctl_gate "$_odu_lbl" odcwd odpar odtmp; then ctl_ok=1; else` C-b, C-c, C-f, U | — | — |
| C-b | root, an answer inside `TMPDIR` that the run did not make | `_control "$CTL/clean" 0 "holds more than this run made" … "$CTL/fakeoldmktemp" "" "TMPDIR=$CTL/odtmp"` | rc 0, the line | "the exit path removes only the directory this run made" |
| C-c | root, taken `run` | `_control "$CTL/clean" 2 "could not make this run's own directory" … "$CTL/fakecollmktemp" "" "TMPDIR=$CTL/odtmp"` | rc 2 | "a scratch dir whose run directory exists decides nothing" |
| C-f | root, an answer inside `TMPDIR` that is a symbolic link (b1) | `_control "$CTL/clean" 2 "a symbolic link, not a new directory" … "$CTL/fakelnkmktemp" "" "TMPDIR=$CTL/odtmp"` | rc 2, the reason | "a mktemp that names a symbolic link decides nothing" |
| U | the helper in-process: (a), (b), (c), an empty `<dir>`, real | per row a subshell from `odcwd` with the row's shim first on `PATH` and `K2_CTL_REACHED="$_VFY/.odu"` calling `_k2_own_dir _odu_d <odpar, or "" for unres> p` | refused with `printed no path` / `not a new name inside` ×2 / `could not resolve the directory` (real `mktemp`); `odcwd` = `keep`, `odpar` empty, `odtmp/k2scratchnamed` = `keep`; real: `odpar/p…/run` exists | `_odu_lbl` "the own-dir helper refuses an answer outside its directory and makes nothing anywhere" |
| in-tree | root, `TMPDIR` inside the tree (re-derived) | `_control "$CTL/inscope" 2 "INSIDE the tree" … "" "" "" "" "TMPDIR=$CTL/inscope/scr"` | rc 2 | (unchanged) |
| umask | root, umask 777 (needle re-derived) | `_control "$CTL/clean" 2 "could not make this run's own directory" … "" "" "" 777` | rc 2 | (unchanged) |

C-a and C-d need their own cwd (I12): a `_control` child runs in the caller's cwd (ctlrun :349), and `_ctl_run` asks
the gate about root and cwd. C-b, C-c and C-f read `odtmp` through their shims and `TMPDIR`, which no gate derived
from root and cwd asks, so they run inside the own-dir block's gate (M2): a fixture that did not build is `CONTROL NOT
EXERCISED`, never C-c's vacuous green (an absent `k2scratchcoll` answers "could not make" too). C-f needs `odlnk` as
well as the link: with a dangling link the check removed would fail at `mkdir run` instead, so both are built on the
line that fails `odtmp`. U checks the **reason**, so a shim that never ran cannot pass it. The registry accepts a
direct `_ctl_gate` (kind `direct`, correspondence :214–:218). Three labels join the label block, before `_af_lbl`
(controls :123), so `_af_lbl` stays the last definition (X11's anchor, D-F4): `_mtn_lbl`, `_dot_lbl`, `_odu_lbl`; C-e,
C-b, C-c and C-f are `_control`s with literal labels, as every `_control` has. The prose counts of the non-`_control`
blocks change in the same commit: controls :105–:106 (relcwd, the two cwd-mktemp, the own-dir and fsmonitor
blocks), ctlrun :120 (ONE GATE: plus the own-dir block's direct ask), ctlrun :123–:125 ("four blocks that run the
wire … all five paths") and ctlrun :214 ("the four blocks that are not one"); the `k2` control's comment's mention of
`fakerelmktemp` and of relcwd's comment (controls :846–:847, :850) goes with the shim.

**Every verdict site CR#1 changes or adds, and its control inside CR#1** (PM decision 22; the program absorbs the
sites CR#1 does not touch, PM decision 20):

| site (at `67488c8d`, or new) | CR#1 | control | record |
|---|---|---|---|
| wire :401, `no scratch dir` (a failing `mktemp`) | moved into the helper | C-e (a shim that exits 1); none reached it before | R13 |
| the helper's empty-answer refusal | new | C-a, U `no` | R3 |
| the helper's `<dir>` refusal | new | U `unres` | R10 |
| the helper's text test | new | C-d, U `dot`, `old` | R4 |
| the helper's symbolic-link refusal | new (b1) | C-f | R14 |
| the helper's `mkdir run` refusal | new | C-c, umask | R6 |
| the `<dir>` resolution | new | relcwd | R1 |
| the `run` level (`$<var>` = `<top>/run`) | new | C-b, U `real` | R5 |
| the root call's operand form (`\|\| :`) | changed (was D5's `\|\| SCRATCH=""`) | umask, C-a, C-c, C-d, C-e, C-f | R2 |
| wire :534–:538, the post-trap guard | changed | C-a, C-c, C-d, C-e, C-f, umask | R9 |
| the exit path's `rmdir` and its "left … in place" line | new | C-b, relcwd | R7, R8 |
| controls :222–:223, the verifier's directory | changed | **parent-only**: N6 `verify` (3 shapes × 2 shells) and X16 `staleshim`, at T | — |
| harness :803–:804, the controls' scratch dir | changed | **parent-only**: N6 `ctl`, at T | — |
| harness :175–:176, the postconditions' directory | changed | **parent-only** (the window): N6 `pgref`, at T; X5 `xrace` | — |
| `_og_trap`'s reset and the sig row's assertion | new | the `_og` sig row | R12 |

**Blind spots: no always-on control (decided, PM decisions 26 Q1 and 27).** Two properties of this slice have **no
always-on control**, and both are declared:

- **the form of the four sites**: a raw `mktemp` put back at any of the four sites, or a new one added elsewhere,
  stays green on every run. r2–r3 carried an always-on census of the text for it; R3 measured its drop rules failing
  open (a `-p <pid>` after any command, a `"` read as a word boundary, a global `\$` drop) and its per-line red
  branches without kill records, so it left CR#1 with its record (r3's R11) and its allowlist (PM decision 27), and
  its defect is a program member (§5);
- **the containment of the three parent-only sites** (controls :222 `_VFY`, harness :803 `CTL`, harness :175 `_pq`):
  they run only in the parent wire, and every `_control` child runs `--selftest` (ctlrun :352), which never sources
  the controls (wire :1330–:1344).

What CR#1 has instead: containment is checked at T (N5 for the root; N6 and `staleshim` for the three parent-only
sites, recorded in §T.N); the new form is fenced by the helper's records (R1–R10, R13, R14: each kills a mutation of
one of the helper's checks, on every opt-in run); the call sites are review's. An always-on reach (a nested
normal-mode run of the wire, ≈ 35 s per site, needs a full run from inside the controls) and an always-on check of
the form are the program's (§5; slot (ii)).

**Records** (records.sh; the needle is the label; R1 and R2 replace records :84 and :98; R3..R10, R12, R13 and R14
follow records :99 in this order, records :100–:110 (r4), which is the records' order and the evidence logs' numbering; r3's R11,
the census's record, left with the census and its number is not reused; 11 new, 2 retargeted):

| id | expression (target: the wire unless prefixed) | killed by (measured, both shells) | survives with (measured, both shells) |
|---|---|---|---|
| R1 (records :84) | `s/_od_d="[$](_phys "[$]2")"/_od_d="${2%\/}"/` | relcwd, in-tree (an unresolved `TMPDIR` is relative, or symlinked: `/var` → `/private/var`, which the in-tree check's physical comparison then misses) | relcwd's two arms and the in-tree control's verdict |
| R2 (records :98) | `s/^_k2_own_dir SCRATCH "[$]{TMPDIR:-\/tmp}" k2scratch \|\| :$/_k2_own_dir SCRATCH "${TMPDIR:-\/tmp}" k2scratch/` | umask, C-a, C-c, C-d, C-e, C-f (a refusal under errexit ends the run rc 1, before the trap, silently) | the six controls' verdicts (C-a's and C-d's assertions; C-c, C-e, C-f and umask through `_control`) |
| R3 | `s/^  \[ -n "[$]_od_a" \] \|\| {/  true \|\| {/` | C-a, U `no` | C-a's assertion and U's `no` row |
| R4 | `s/not a new name inside [$]_od_d"; return 1 ;; esac/not a new name inside $_od_d" ;; esac/` | C-d, U `dot`, `old` (and `unres`, through the `run` the `dot` row leaves in `odcwd`) | C-d's assertion and U's `dot`, `old`, `unres` rows |
| R5 | `s/printf -v "[$]1" %s "[$]_K2_OD_TOP\/run"/printf -v "$1" %s "$_K2_OD_TOP"/` | C-b, U `real` (and U's refusal rows, which read the `keep` C-b's run removed) | C-b's verdict and all five U rows |
| R6 | `s/mkdir "[$]_K2_OD_TOP\/run" 2>\/dev\/null \|\|/mkdir -p "$_K2_OD_TOP\/run" 2>\/dev\/null \|\|/` | C-c | C-c's verdict |
| R7 | `s/rmdir "[$]_K2_SCR_TOP" 2>\/dev\/null \|\|/false \|\|/` | relcwd (an empty `k2scratch…` left in `reltmp`) | relcwd's two arms |
| R8 | `s/rmdir "[$]_K2_SCR_TOP" 2>\/dev\/null/rm -rf "$_K2_SCR_TOP"/` | C-b, and U's refusal rows (the `keep` is gone) | C-b's verdict and all five U rows |
| R9 | `s/^if \[ -z "[$]SCRATCH" \]; then$/if false; then/` | umask, C-a, C-c, C-d, C-e, C-f (each refusal falls through to the message at wire :543 (the in-tree block's resolution check), rc 2, the wrong reason) | the same six verdicts |
| R10 | `s/\] \|\| { _K2_OD_WHY="could not resolve the directory/] \|\| : \|\| { _K2_OD_WHY="could not resolve the directory/` | U `unres` | U's `unres` row |
| R12 | `controls:s/ _K2_SCR_TOP=""; mkdir -p / mkdir -p /` | the `_og` sig row's new assertion | that assertion |
| R13 | `s/XXXXXX")" \|\| { _K2_OD_WHY="no directory from mktemp/XXXXXX")"; : \|\| { _K2_OD_WHY="no directory from mktemp/` | C-e | C-e's verdict |
| R14 | `s/^  \[ ! -L "[$]_od_a" \] \|\| {/  true \|\| {/` | C-f (the link passes; `run` is made in its target; the run goes on, rc 0) | C-f's verdict |

(`\|` is markdown escaping; the record text has a bare `|`.) Labels: R1, R7 `_rel_lbl`; R2 the umask control's; R3
`_mtn_lbl`; R4 `_dot_lbl`; R5, R8 C-b's; R6, R9 C-c's; R10 `_odu_lbl`; R12 `_og_lbl`; R13 C-e's; R14 C-f's. R6 is
`mkdir -p`, not `true`: with the proof removed outright every normal run fails at harness :54 with rc 2 and no label
(measured in r2). **Method**, all on the final r4 tree, 2026-10-26, each shell in its own clone and one run at a time
per shell (the two shells side by side): each kill is the real opt-in path filtered to one record
(`scratchpad/cr1r4/bin/optin.sh`, a copy of `elidex-fgit-handoff/cr2/bin/r3-optin.sh`; HUP/INT/TERM at their
default; `scratchpad/cr1r4/log/kill.R<n>.<sh>.log`); each survive arm is the same trial with the named arms removed
(`cr1r4/edits/R<n>.ed`, written by `cr1r4/bin/r4-survive-gen.py` from `r4-survive.spec`; `survive.R<n>.<sh>.log`);
the labels a mutant reds were read from a plain run with the record applied in place (`inplace.R<n>.<sh>.log`; that
method also prints the record gate's "its anchor is stale" line for the edited file, an artefact). Results:
every kill log reads `316 entr(ies), 0 not killed as named` and every survive log `1 not killed as named` (the one
record trialled), 13 records × 2 shells each; the in-place runs red exactly the labels of the table's "killed by"
column, on both shells (`cr1r4/log/batchI.{b32,b53}.out`). **R1's first form**, `_od_d="$2"`, died for the WRONG REASON at r3 (`scratchpad/r3log/kill.R1.*`); diagnosed at
r4 (`cr1r4/bin/r1diag.sh`, `cr1r4/log/r1diag.*.log`): this machine's `TMPDIR` ends in `/`, so that mutant's template
and `SCRATCH` hold `//`, and P-b (`_pb_lbl`, "the fixture git has no system or global layer outside the void") reds
first, because git reports the void's `GIT_CONFIG_GLOBAL`/`GIT_ATTR_GLOBAL` paths with the `//` collapsed; with the
trailing `/` stripped from `TMPDIR`, the same mutant reds relcwd and in-tree, R1's labels, on both shells. The record's
form `"${2%/}"` strips it, so it dies for R1's reasons whatever `TMPDIR`'s shape. Records :502 (the exit path's `rm -rf` line) is untouched. Floor:
`_MUT_RECORDS_MIN` 305 → 316 (`_mutants | wc -l` = 316 at r4, 305 at `67488c8d`); corpus §6.1's "Current totals"
(corpus :472–:485) and its table gain the CR#1 rows, one per record with its kill and its survive arm (corpus §6.1
:245's requirement), written from this table. The ratchet stays 18: three new labels (`_mtn_lbl`, `_dot_lbl`,
`_odu_lbl`) and four new literal labels (C-b, C-c, C-e, C-f), each recorded.

**Needles (I10).** `/usr/bin/grep -n -F -e 'printed no path' -e 'not a new name inside' -e "could not make this
run's own directory" -e 'holds more than this run made' -e 'could not resolve the directory to make it in' -e 'no
directory from mktemp' -e 'a symbolic link, not a new directory' <wire and parts>` at r4 (`scratchpad/cr1r4/log/i10-grep.txt`)
gives: in the wire, one line per text, the helper's (wire :413, :414, :415, :416, :417, :419 (r4)) and, for `holds more
than this run made`, the exit path's (wire :544 (r4)); in controls, the control lines that read them (controls
:717–:718, :724–:725, :729, :739–:742, :942 (r4)); in records, R4's and R13's expressions (records :101, :109 (r4),
which quote `not a new name inside` and `no directory from mktemp`); and harness :801 (r4), the comment at the
controls' site ("an answer that is not a new name inside `$SCRATCH`"). R10's expression holds `could not resolve
the directory`, not the whole needle, so the grep does not list it. The site texts stay on one line each.

**Negative control (red without the fix).** The new controls need the helper, so they cannot run over the base; the
red evidence is §1's victim runs at the base and N6's nine inner cells per shell. At the fix, the same victim runs
with every `mktemp` call answered by one bad value (N5 at r4: `scratchpad/cr1r4/bin/n5.sh`, rows in
`cr1r4/log/batchX.<sh>.out`, both shells): every row rc 2; empty → `mktemp exited 0 but printed no path`; `.`,
`-` (with `OLDPWD`), `rel` (with `CDPATH`) and an outside directory → `mktemp answered [<a>], not a new name inside
<TMPDIR resolved>`; a non-zero exit → `no directory from mktemp (TMPDIR/disk?)`; in every row the victim, the outside
directory and `cdp/rel` keep `sentinel` only. The symbolic-link answer: §3.1. Inner: N6 (§6).

## §5 Not in this slice, and the boundaries it moves

- **Touch-time split, a decided exception.** The wire is 1388 lines (1411 (r4) after this slice); CLAUDE.md's
  touch-time rule would split it first. The PM decision gives the split to the program as its first slice: this is
  a merge-blocking safety fix inside one cohesive block (scratch setup + exit path) plus three call sites, and the
  split is a structural change whose seams the program's umbrella plan has to choose and review. Every edited *part*
  stays under 1000 lines (§6 N2); controls reaches 993 (r4), so the program's split names it (M14). The split
  re-keys every record target (keyed by part); the program's first slice moves them with the lines.
- **The own-dir census (R3 IMP A; PM decision 27)**: not in CR#1. Its defect is a program member, stated as a defect
  only: "no always-on check that temporary places are created only through the helper"; R3's census findings (drop
  rules that fail open, per-line red branches with no kill record) are that member's evidence (slot draft). The
  blind spot it leaves in CR#1 is §4's.
- **S3** (controls :903's needle is a substring of two wire messages, :536 and :543): with D1 the umask control's
  needle becomes the helper's `mkdir` refusal, one wire line, and deleting the post-trap guard reds it (R9's kill
  set). S3 as classified no longer describes the tree after CR#1; decided, PM decision 26: resolved by CR#1, moved to
  the program's Not members (§7).
- **NEW-2** (not in CLASSIFICATION): `_phys "-"` reads `$OLDPWD` under 5.3 and a bare relative operand consults an
  exported `CDPATH` (both shells). At the base it reaches the root's answer (§1); after CR#1 it reaches only the
  root's `<dir>`, `${TMPDIR:-/tmp}`: a two-line value, `mktemp` fails, rc 2 for a misleading reason (`no directory
  from mktemp`), and nothing is removed (§3.1). PM decision 20 item 3: one member of
  `#11-trip-wire-launch-environment` (both `CDPATH` and `-`→`$OLDPWD`), a sibling of CR#13; the slot draft carries
  the amendment text.
- **ER-A5-1** was closed in #527 (`…-impl-cr2-d.md` §T.6, CR2-D), so it is correctly not a program member.

**Boundaries CR#1 moves** (F6; texts for the implementer's commit and, where marked, for the PM):

- **(a) impl-d §T.L L5's trigger fires.** Its trigger is "a change to how the window or the postconditions resolve
  `mktemp`, or to P-j" (impl-d :579–:580); harness :175 now goes through `_k2_own_dir`, so it fires. **Disposition
  (Ax5 IMP 3): re-homed, not run.** `shimafter` and `shimhash` cannot run at the new T: each insert starts with
  `mkctl`'s two lines, stated only as prose, and the reviewer's insert files are gone (impl-d :572–:574; R15
  :282–:283); writing them now would be a new instrument, not a run of the recorded one. They stay with
  `#11-trip-wire-launch-environment` member (3), as #527's amendment (c) put them, and the consequence is restated:
  whatever the postconditions' `mktemp` resolves to, its answer only names a place inside `${_FW_DIR%/*}`, and the
  directory they work in is the run's own `mkdir`'s (U, N6 `pgref`); a shim behind the pin still changes which
  `mktemp`, and which `mkdir`, runs. The implementer's commit adds to impl-d's L5 a dated note: "[added <date>: CR#1
  (#501, the CR#1 plan) fired this trigger: the postconditions' `mktemp` answer is now refused outside
  `${_FW_DIR%/*}` and never names the working directory itself; a shim behind the pin still changes which `mktemp`
  (and `mkdir`) runs; `shimafter`/`shimhash` were not run at CR#1's T, their instrument being prose only.]"
- **(a2) `#11-k2-fixture-postcondition-coverage-at-t` (impl-d L1–L3; the defer ledger, memory
  `project_open-defer-slots.md` :376, outside the repo) fires.** Its trigger names "a clause named by a §6.1 record":
  corpus :280 (C9's `xrefpoison` row) names the postconditions' directory as "the directory `mktemp -d` has just
  made", and landing :409 (C9's row) "a fresh `mktemp -d` beside `$_FW_DIR`"; CR#1 makes that directory the run's own
  `mkdir` of `run` inside the answer. What it means: L1–L3's gap (X2, the survive check's population, X5's retired
  cells) is unchanged; CR#1 runs none of them, but measures the survive arm of each of its own thirteen records (§4).
  The two stale texts are restated in this slice's commit, each "(restated <date>, the CR#1 plan)": landing :409's
  clause becomes "a fresh `run` directory, made by `mkdir` inside a `mktemp -d` answer beside `$_FW_DIR` (the CR#1
  plan §3.1)"; corpus :280's "the directory `mktemp -d` has just made holds nothing" becomes "the `run` directory the
  run's own `mkdir` (no `-p`) has just made holds nothing". **Ledger text for the PM**: "Fired at PR #501's CR#1
  (<fix commit>, <date>): the postconditions' directory is now a `run` directory made by the run's own `mkdir` inside
  a `mktemp` answer beside `$_FW_DIR` (harness :175; the CR#1 plan §3.1), a clause corpus §6.1's C9 `xrefpoison` row
  names (restated there and in landing's C9 row). The gap is unchanged: X2, the survive check and X5's retired cells
  were not run at CR#1's T; CR#1 measured the survive arms of its own thirteen records only."
- **(b) `#11-trip-wire-launch-environment` member (3)**, ledger text for the PM: "Amended at PR #501's CR#1 (<fix
  commit>, <date>): of member (3)'s tools, `mktemp` no longer has a wrong-but-successful answer in the K2 wire: every
  answer is either refused (an empty or failed answer, one that is not a new name inside its physically resolved
  directory, `TMPDIR` at the root, or a symbolic link) or only names the place where the run's own `mkdir` makes the
  directory it works in, which is all a recursive removal reaches (the CR#1 plan §3.1; U, C-a..C-f). This amendment
  adds `mkdir` to the member's tools, which did not list it: since CR#1 it makes that directory, so a `mkdir` that a
  caller's `PATH` resolves elsewhere is now this member's channel. The other tools (`grep`, `git`, `cat`, `readlink`,
  `tr`, `cmp`) stand as stated." The members it gains (E1, CR#13, NEW-2) are the slot draft's amendment (i).
- **(c) residuals §5.1's class-(c) paragraph** (residuals :47–:59; the clause is :53–:59), rewritten in this slice's
  commit in its own lines: the clause on `mkfn`, `shimafter`, `shimhash` and `staleshim` says that they change which
  `mktemp` the postconditions or the parent run, and that since CR#1 an answer only names a place: one that is not a
  new name inside the run's directory is refused (`staleshim` at the CR#1 T: rc 2, `mktemp answered [<prep>], not a
  new name inside <SCRATCH>`, the prepared directory untouched; red at #527's T, and rc 0 on its prototypes), and
  one inside it yields a fresh `run` made by the run's own `mkdir`; the property (which executable or code the
  verifier runs) stands, and `mkdir` itself is now the tool that property names for this step. Each clause stamped
  "(restated <date>, the CR#1 plan)".
- **(c2) the same clause where else it is stated**, restated in this slice's commit, each "(restated <date>, the CR#1
  plan)". The sweep: `/usr/bin/grep -n -E 'mktemp|staleshim|_VFY|pgref|verifier.s directory'` over the
  `2026-09-citation-hygiene-k2-fgit-hermetic*.md` memos at `67488c8d`, each hit read; a hit that records a past
  round's measurement or a closed memo's history (the round tables, `…-pr527*.md`, `…-impl*.md`, `…-provenance.md`,
  `…-cr2-instruments-texts-*.md`, `…-cr2-sweep.md`, corpus's dated rows) is history and stays; finalhead and its
  runbook are accepted bodies and are not edited (§6); these present-tense statements are made false by CR#1:
  - **design memo :170–:175** (D16-B): "lives in a directory the verifier makes after the build with `mktemp -d`"
    becomes "lives in a `run` directory the verifier makes after the build with its own `mkdir` (no `-p`) inside a
    `mktemp -d` answer (`_k2_own_dir`, the CR#1 plan §3.1)", and "a changed resolution of `mktemp` can hand either
    directory back to the fixtures (`mkfn`, `staleshim`)" becomes "a changed resolution of `mktemp` or `mkdir` can
    still hand either directory to the fixtures (`mkfn`, `staleshim`): since CR#1 not by naming a place outside the
    directory it was asked for (refused; `staleshim` at the CR#1 T, rc 2), but by naming one inside it that they
    prepared, or by a `mkdir` that does not make `run`".
  - **design memo :233–:235** (the measured boundary, "and so is a `mktemp` made to name a directory the fixtures
    prepared, by a fixtures-file function (`mkfn`…) … `shimafter` … `shimhash`"): the measurements stand at their
    rounds (16, 17); a dated note follows them: "[CR#1 (restated <date>): not re-run at CR#1's T (the CR#1 plan §5
    (a)); since CR#1 such an answer is refused unless it is a new name inside `${_FW_DIR%/*}`, in which the
    postconditions' own `mkdir` makes `run`.]"
  - **design memo :508** (W2's gate): "the two blocks that are not `_control`s (relative scratch, fsmonitor) run
    through it" becomes "the four blocks that run the wire and are not `_control`s (relative scratch, the two
    cwd-`mktemp` blocks, fsmonitor) run through it, and the own-dir block asks `_ctl_gate` directly for the fixtures
    its helper calls and its three `_control`s' shims read" (the same change as ctlrun :120, :123–:125, §4).
  - **design memo :629** (the `R` row, `a relative scratch dir is removed on exit`): "a run from `$CTL/relcwd` whose
    `mktemp` is the `fakerelmktemp` prefix (it answers a relative path, and `_ctl_run` demands the prefix's `path`
    mark back) passes and leaves `$CTL/relcwd` empty" becomes "a run from `$CTL/relcwd` handed the relative
    `TMPDIR=reltmp` (`_ctl_run` demands the channel back) passes and leaves `$CTL/relcwd` holding only `reltmp`,
    empty", and "Pinned by two records: the wire's scratch dir kept relative (`_phys` dropped), and the relcwd
    fixture's build failing" becomes "Pinned by three records: the scratch root's `<dir>` left unresolved (R1), the
    exit path's `rmdir` dropped (R7), and the relcwd fixture's build failing".
  - **census memo :39–:40**: "they are made in a `mktemp -d` directory too, so a changed `mktemp` (`mkfn`,
    `shimafter`, `staleshim`) can hand them to the fixtures" becomes "they are made in the postconditions' directory
    too (since CR#1 a `run` directory the run's own `mkdir` makes inside a `mktemp -d` answer), so a changed `mktemp`
    or `mkdir` (`mkfn`, `shimafter`, `staleshim`) can still hand them to the fixtures, by naming a place inside
    `${_FW_DIR%/*}` that they prepared".
  - **census memo :116–:120**: "made … by `mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX"`" becomes "made … by `_k2_own_dir _pq
    "${_FW_DIR%/*}" pgref`: a `run` directory its own `mkdir` (no `-p`) makes inside a `mktemp -d` answer that must be
    a new name inside `${_FW_DIR%/*}`, not a link", and "unless the fixtures file changed what the `mktemp` the
    postconditions run resolves to" becomes "unless the fixtures file changed what the `mktemp` or `mkdir` the
    postconditions run resolves to (since CR#1, to name only a place inside `${_FW_DIR%/*}`)".
  - **census memo :157–:161**: "`mktemp -d "$SCRATCH/verifyXXXXXX"`" becomes "`_k2_own_dir _VFY "$SCRATCH" verify`
    (a `run` directory inside a `mktemp -d` answer)", and "can answer that `mktemp` with a directory they prepared
    (`staleshim`, D18-A)" becomes "can answer that `mktemp` with a directory they prepared inside `$SCRATCH`, or
    change which `mkdir` makes `run`; one outside `$SCRATCH` is refused since CR#1 (`staleshim`, D18-A; rc 2 at the
    CR#1 T)".
  - **landing :411** (C11's row, its last sentence): "(an executable written into a caller-`PATH` directory,
    `staleshim`, D18-A)" gains "; since CR#1 it can only change which `mktemp` and `mkdir` run: an answer outside
    `$SCRATCH` is refused, `staleshim` rc 2 at the CR#1 T".
  - **the tool comments** (controls :219–:221 and the others): §3.1, "Comments made false by the fix".
- **(c3) the accepted `rmdir`'s residual row R24** and the residual ranges: §3.1.
- **The landing memo** is edited by this slice for two ratified definition changes of §6 (`staleshim`, `xrace`),
  each a dated restatement written as the 2026-10-25 staleshim one was (impl-d :474; landing X16's "(restated
  2026-10-25: the expected value was …)"), and for (a2)'s C9 clause and (c2)'s C11 clause. `sealdotdot`'s restated
  definition is the delta runbook's `crc.sh` patch (§2 there), not a landing edit: `crc.sh` is extracted from
  `…-pr527.md`, closed at round 14, and the patch is the instrument's one statement.

## §6 T and landing

The fix changes `.claude/tools/**`, so it moves T (landing :339–:341's definition, :392–:397's command) to the fix
commit. Landing 7.4 (:152–:165) stops the sequence for the user on a T move and requires a reviewed plan whose step 1
runs at the new T first; the user's 2026-10-25 decision is the "continue", and this memo is that plan. The finalhead
runbook is bound to `01fb2924` and `3b7ff6d5` (its frame guard, runbook :88–:102; the binding lines :96–:99) and is
not edited; **the new-T instrument frame is the delta runbook**
`docs/plans/2026-10-citation-hygiene-k2-wire-cr1-scratch-plan-runbook.md` (F5): it names T and replaces the guard
(T's parent is `67488c8d`), reads every doc at T from `W` = this branch's worktree (`k2-wire-fgit-hermetic` is merged
and gone from `origin`), byte-compares each extraction with `3b7ff6d5`'s, re-checks every RUN cell's anchors at T in
a dry pass of its own (finalhead §7 C5), lists each fence of runbook §6.0–§6.8 that it replaces, and adds the fences
of this plan's own cells (BN: N1, N2, N5, N6).

**The sequence, and its commits** (`origin/main` is merged before the fix commit: those merges are `fdded560` and `67488c8d`; no merge
of `origin/main` happens between the fix commit and the end of step 1, X9 and Codex: a later merge that touches
`.claude/tools/**` or `scripts/**` moves T and step 1 runs again at it):

1. **The fix commit**, on `67488c8d`: the tool code of §3.1/§4, and the doc restatements of §5 ((a), (a2), (c),
   (c2), (c3), the landing edits, corpus §6.1's CR#1 rows). It is T.
2. **Push** (user approval). The delta runbook's guard needs `origin`'s `webref-cite-audit-tool` to equal `W`'s
   HEAD, so the push comes before step 1; X9 is that push's CI run.
3. **Step 1 at T** (below), by the delta runbook.
4. **The verdict commit** (docs only; T unchanged): §T.N in impl-d with step 1's results and X9's verdict.
5. **The R6 erratum** (PM decision 20 item 4), a separate docs commit: residuals R6's row (residuals :118) gains
   the one line "E1 (caller `core.ignoreCase`) exists, so not loud → `#11-trip-wire-launch-environment`" (no
   rewrite). Docs only; T unchanged.
6. **One fresh Codex round** on the head after 4 and 5; its terminator is landing 7.4's.

**(a) Step 1 at the new T: the per-cell result of finalhead's run/retire rule** (finalhead §1.1, re-applied cell by
cell; bash 5.3.20 and 3.2.57 unless noted; two runs at a time at most; verdicts by log grep; the fences: the delta
runbook):

| cell | at the new T | why (the rule's clause) | expected | runs |
|---|---|---|---|---|
| N1 plain run | **RUN** | the gate; runs C-a..C-f, U, relcwd, in-tree and umask | PASSED, rc 0, from the checkout root (the clone clean after) and from an empty cwd outside it (still empty) | 4 |
| N2 = X10 | **RUN** | parts edited | `$SH -n` clean over every part; `wc -l`: controls 993, correspondence 450 (unedited), ctlrun 440, fixtures 705, harness 887, records 607, mutations 623 (all < 1000; mutgen 348 unedited; the wire 1411, §5) (r4) | — |
| N5 root answers | **RUN** | the root site's evidence | base (`67488c8d`): §1's table, its `outside` row, and `no scratch dir` for a failing `mktemp`; T: §4's victim rows | 24 (6 answers × 2 shells × 2 trees) |
| N6 inner sites | **RUN** | NEW-1's evidence; the parent-only sites' control (§4) | base: §1's table; T: rc 2 (`verify`, `ctl`) or 1 (`pgref`), the site's text and the reason in the output, `victim` and `out` = `sentinel` | 36 (18 per tree) |
| X3 main | **RUN** | finalhead's clause stands (the row's own command); 316 records, 11 new and 2 retargeted | landing :431: three hits per shell, PASSED | 2 |
| X3 plant | **RUN** | finalhead's clause stands (the row's insert, before the fixtures' `built` line, fixtures :705, present at T); its subject is `$_VFY/.genmutants`, and CR#1 changes `_VFY` (controls :222), a parent-only site | landing :431, quoted at runbook §6.7: the plant has no effect, the run reports its normal non-zero `N mutant(s)` (`generated boundary set: N mutant(s)`, N > 0) and PASSED | 2 |
| X3 floor | **RUN** | finalhead's clause stands (the row's edit at mutgen :304, unedited); the same subject | landing :431, quoted at runbook §6.7: red, `!! the generated boundary set ran 0 mutant(s), but the generator derives` and, on the next line, `50 from` (`/usr/bin/grep -A1`), and `1 neither killed nor argued equivalent` | 2 |
| X3 survive check | retired (L2) | stands; CR#1 measures its own records' arms (§4) | — | 0 |
| X2, X5 corpus R/RLOUD/RES/P, X14, X16 shim cells | retired (L1, L3, L4, L5) | stand (impl-d §T.L); L5's trigger fired, §5 (a) | — | 0 |
| X5 draft-10 | **RUN** | the window's prelude and `_fgit_postconditions` changed | 32 `PASS`, no `HARNESS-ERROR` | 32 |
| X5 `xrace` | **RUN**, insert re-derived | its glob had lost its subject (below) | recorded, not judged (class (c), corpus :280); at r4, rc 0, PASSED, both shells | 2 |
| X6 | **RUN** | P-c / P-g read the postconditions' directory, now `pgref…/run` | landing :435 | 4 |
| X8 | **RUN** | a record that cannot fail; cheap (M15) | the six times, recorded | 6 |
| X11 | **RUN** | finalhead's clause stands (its lines verbatim, after the last label, D-F4); CR#1 adds three labels before `_af_lbl`, which stays the last | red, the ratchet lists `an unrecorded probe` | 2 |
| X13 | **RUN**, `sealdotdot` re-derived | `_k2_exit`, the pre-trap block and the guard changed (I5) | landing :442, unchanged; the sentinel line recorded | 36 |
| X15 | **RUN** | P-g reads the postconditions' directory | landing :444 | 2 |
| X16, 19 cells | **RUN** | P-g reads the references, now under `pgref…/run` | landing :441 as restated 2026-10-25 | 38 |
| X16 `staleshim` | **RUN**, expected restated | the helper refuses its answer (below) | rc 2, `could not create the verifier's directory (mktemp answered [<S>/prep], not a new name inside <SCRATCH>)`; `prep` holds only `.anchor` | 2 |

**194 runs = 130 (finalhead's RUN cells, finalhead §1.1 :96: X3 6 (main, plant, floor), X5 34, X6 4, X8 6, X11 2, X13
36, X15 2, X16 40) + 64 (this plan's own cells: N1 4, N5 24, N6 36).** r3's table gave 126 + 64 = 190, with X3 plant
and floor "not run"; that was not the rule's result (finalhead rates both RUN, and their subject runs through `_VFY`,
which CR#1 changes), and r4 corrects the count, not the decision (PM decision 27). Wall: X3 three pairs ≈ 7.4 h
(runbook §6.7's estimate, ≈ 8.9 ks a pair); the rest ≈ 1.5 h. D1, the census's removal and b1 change no other cell's
class: the helper is inside cells that already run (N1, X3, X11), and no retired cell's reason reads it.

**Definition changes this plan ratifies** (each a RUN cell's definition, finalhead §1.3 event (2); the driver's
report surfaces them to the user):

- **`staleshim`** (X16, F3): its shim answers `*verifyXXXXXX*` with `$S/prep`, outside `$SCRATCH`; at the new T the
  helper refuses it. Expected: **rc 2**, `!! could not create the verifier's directory (mktemp answered [<S>/prep],
  not a new name inside <SCRATCH>); decided nothing.`, `prep` holding only `.anchor`, no control run (was: rc 1,
  CF=8, NE=4, restated 2026-10-25; before that rc 0, PASSED). Measured at r4, both shells (`scratchpad/cr1r4/bin/r4-batchX.sh`,
  `cr1r4/log/batchX.{b32,b53}.out`): rc 2, that line, `prep` = `.anchor`, on both shells. Landing X16 gets the dated restatement.
- **`xrace`** (X5, F4): its loop globs `"${_FW_DIR%/*}"/pgref*/a/.git` and `…/b/.git`, which the `/run` level leaves
  matching nothing; re-derived: `pgref*/run/a/.git`, `pgref*/run/b/.git`, the rest verbatim (R15 :80–:81). Measured:
  at the r3 prototype the old insert gave rc 1, red (`xrefctl`'s verdict: its loop wrote nothing), the re-derived
  one rc 0, PASSED (`scratchpad/r3log/extras.{b32,b53}.out`); at r4 the re-derived one rc 0, PASSED on both shells (b53
  `cr1r4/log/batchX.b53.out`, 78 s; b32 `cr1r4/log/xrace2.b32.out`, 66 s, after a first b32 try under a load of ≈
  50–80 ended at the cell's 120 s alarm, `batchX.b32.out`), the window's bash and a `sleep` left in the group and
  killed by group, as R15 records for the base. Class (c), not owed (corpus :280); recorded, not judged. Landing X5 gets the
  dated restatement.
- **X13 `sealdotdot`** (F4): `_seal "$CTL/walk/../../../k2-sentinel"` reached the sentinel beside `$SCRATCH`
  (`TMPDIR=$X/tmp`); with `/run` under both the scratch root and `$CTL` it names `$SCRATCH/k2-sentinel`, which does
  not exist, so the cell's "outside-scratch sentinel" line would watch nothing. Re-derived:
  `$CTL/walk/../../../../../k2-sentinel` (two more levels). Measured at r4 (`crc.sh` with this patch,
  b53 `cr1r4/log/batchX.b53.out`, b32 `cr1r4/log/seal2.b32.out`, re-run once a
  first b32 try at a load of ≈ 50–80 ended at the cell's 120 s alarm): rc 1, NE=1, CF=1, the same first lines with
  nounset off and on, on both shells, and the sentinel's mode unchanged; an instrumented run at the r3 prototype, whose `/run` levels r4 keeps,
  printed `$CTL/walk/../../../../..` as `$X/tmp` (three `..` give `$SCRATCH`). The cell's judged value (the same
  status, NE/CF and first four `!!` lines with nounset off and on) does not depend on it: `_seal` refuses the text's
  `..` component before any `chmod`. Its restatement is the delta runbook's `crc.sh` patch (§2 there).

N5's and N6's setups (the shim, the victim, the outside and `CDPATH` directories, `OLDPWD`, each `$D` inside the
step-1 `K` and expanded when the shim is generated, M11), their verdict greps and their expected lines are the delta
runbook's BN fence (§4 there).

**(b) X9** on #501's CI at the new T (the push of step 2): `Layering trip-wires` SUCCESS and its time under half the
budget, 150 s (`timeout-minutes: 5`, ci.yml :185 at `67488c8d`), and the **Python suites** job (id `tools`, A-iii
#545: it runs every tracked `test_*.py`, among them `.claude/tools/_webref/test_spec_labels.py`, which this PR adds;
measured green by the driver, 13 tests on Python 3.9.6) green on its "3.9" leg too; written in the open record
(`…-impl-cr2-d.md`, a new §T.N, the verdict commit). **(c)** One fresh Codex round on the head after the verdict
and erratum commits; its terminator is landing 7.4's. Results of (a) go to the same §T.N.

## §7 Prototype (evidence for review; not the implementation), and decisions

`scratchpad/nw1-helper-prototype-r4.patch` (`git diff fdded560` of a scratch clone at `fdded560`, whose tool code is `67488c8d`'s; 148 insertions, 74 deletions over seven files by `git diff --stat` (verified 2026-10-26): the wire and six parts;
correspondence and mutgen are unedited; it applies to
`67488c8d` with `git apply --check`): the plain run on the final tree PASSED on both shells (39 s / 36 s, side by
side, `scratchpad/cr1r4/log/plain2.{b53,b32}.log`; a first try at a 1-minute load of ≈ 50–80 from other sessions took
400 s and went red in the `_og` check's `zomb` row, a timing-bound row; the batch was stopped and re-run by
`cr1r4/bin/final2.sh` once the load was below 10, and the re-run's logs replace the first try's, `cr1r4/log/final2.out`); R1..R10, R12..R14, the root answers, the symbolic link, the exit path's
removals, the `TMPDIR` shapes, `staleshim`, `xrace` and `sealdotdot` as above; N6's nine cells per shell at r4 (`scratchpad/nw-cell.sh`;
`cr1r4/log/batchX.{b32,b53}.out`): rc 2 for `verify` and `ctl`, rc 1 for `pgref`, the site's text and the reason in
the output, `victim` and `out` holding `sentinel` only. One D1 consequence was found by measuring, not by design: a caller's relative `TMPDIR` reached C-a's and C-d's
children (§3.1); their `TMPDIR=.` channel was accepted by the driver with PM decision 26 and reviewed in R3. The
patch goes to the implementer through the driver; this memo is the specification.

**Decisions** (all decided):

- **Parent-only sites and the form of the sites** (§4): PM decisions 26 Q1 and 27. N6 and `staleshim` at T are the
  controls of controls :222, harness :803 and harness :175; neither their containment nor the form of the four sites
  has an always-on control (a declared blind spot), and the program builds both.
- **The census** (§4, §5): PM decision 27. Out of CR#1; its defect is a program member.
- **The symbolic-link answer** (§3.1): the user's decision 2026-10-26, option b1: one check after the text test.
  If the closing attestation finds a defect in it, it is not fixed: b2 applies (the link shape joins §3.1's accepted
  residual and moves to the program) and the rounds stop (PM decision 27).
- **S3** (§5): PM decision 26. Resolved by CR#1 and moved to the program's Not members.
