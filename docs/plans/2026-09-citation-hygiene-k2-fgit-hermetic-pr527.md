# K2 fixture git — PR #527's review record

This file holds the record of PR #527, from its external review through plan-review rounds 10–12, for
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). It was split out of
`…-k2-fgit-hermetic-reviews.md` §13, where it was the subsection "PR #527", as a touch-time split
before round 12's record would have taken that file past 1000 lines. The text is unchanged. The
references follow `…-reviews.md`'s convention: to the design memo as of the draft the paragraph belongs
to, unless a file is named.

---

## §0 Spec coverage map

**No spec surface** — this file records review dispositions and measurements only. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §P PR #527 — external review and the fix-delta reviews (2026-09-28), and plan-review rounds 10–11

Round history moved here from the memo, which keeps only the live decision. The scenario and result
of each claim below are in the commit messages named (a command is quoted only where it is given
here); the verification runs were made in `git clone --local` sandboxes with a scratch `HOME`,
on bash 5.3 (`/opt/homebrew/bin/bash`) and 3.2 (`/bin/bash`, `PATH=/bin:/usr/bin` first). Commit SHAs are PR-branch commits: after the squash merge
they are reachable through the PR.

- **Codex R1** (`36484cbb`): P-g compares as a set in both directions; the removal record (`git config
  --unset core.filemode`) added.
- **Codex R2** (`b445e02f`): prelude and fixtures copied into the window dir and sourced by fixed relative
  names, so W3's prefixes do not depend on the checkout path.
- **P-f, Codex R4 → fix-delta re-check #2** (`478c7a5f`, `8614507c`, `173d286b`, `1fe79a26`, `da2730d4`):
  R4 added a line-based fallback for an `env` without `-0`; R6 showed it reading a `PATH` holding
  `\nINJECT=foo` as a second name (red on a clean run); R6's replacement probe sent "anything but one
  exact record" to green and shared the `-i` record's anchor (the fix-delta `/elidex-review`); the next
  form read an `env` that could not be started as "no `-0`" (re-check #1); re-check #2 found the stdout
  clause unpinned, an exported function `env`, and a last record without NUL. The shipped test is the
  definition (this `env` runs a command but refuses `-0`), one record per clause; each record dies at
  head and survives with its clause removed (16 runs, `da2730d4`'s message).
- **Codex R11–R13 → PATH** (`0fc775c0`, `dce8cb8c`, `661833f9`): relative entries, then `~`, then
  `~login` — an emulation of the lookup growing a case per round. Measured tilde behaviour: bash 5.3 and
  3.2 expand `~` (`dce8cb8c`) and `~login` in command lookup; 5.3 `--posix` and dash do not; execvp does
  not. `~login` was measured after `661833f9` (whose message says it could not be) with the invoking
  user's own login: `env -i HOME=/nonexistent PATH="~$(id -un)/.local/bin:/usr/bin:/bin" <shell> -c
  'command -v claude'` — found by bash 5.3, 3.2 and 3.2 `--posix`; not by 5.3 `--posix` or dash. Replaced
  by pinning the `git` this shell resolves and dropping non-absolute entries. Hand measurements: a
  `tools/bin` git wrapper was P-e red on `da2730d4` and PASSED after; a `~/bin` git wrapper saw 97 calls
  from inside the window.
- **Codex R12** (`dce8cb8c`): P-a / P-g read `--show-origin` only (git 2.8), not `--show-scope` (2.26);
  a `git` shim rejecting `--show-scope` was red before, PASSED after.
- **fix-delta `/elidex-review` of `8614507c..661833f9`** and **Codex R14** (`18035018`): one resolver
  (`$_REAL_GIT`/`$_REAL_GREP` through `_fgit_resolve`; with an exported function `git` the old
  `command -v` form made 7 controls recurse into their own shims, both shells); P-j with two records; the
  last-record-without-NUL record; P-g's `grep` status (Codex R14: `|| true` turned a grep error into a
  pass); P-c by globs (Codex R14: a newline-only name read as empty).
- **fix-delta `/elidex-review` of `661833f9..18035018`** and **Codex R16** (the commit after `18035018`):
  P-c also reds a void it cannot list (`chmod 300`: globs expand to nothing — the fail-open R14 named
  for `ls`, left open by the glob form); `git` resolved once (`$_FGIT_GIT`) for the wrapper and
  `$_REAL_GIT`; records for P-j's `git` clause and P-c's not-a-directory and not-readable clauses; P-g's census takes
  `HEAD` of any type (Codex R16: a bare repo with a symlink `HEAD` was left out); citation and wording
  fixes (`~login` provenance above, §1 "Outside P" instead of R4).
- **focused re-check #4 of `18035018..33627692`** and **Codex R17** (the commit after `33627692`):
  P-c resets `set -f`/`GLOBIGNORE` before its globs (a fixtures file that left either made P-c
  green with an entry in the void); P-c's not-searchable clause gets its record (`chmod 600`); the
  census matches `.git`/`HEAD` in any letter case (APFS lets git read `head` and `.GIT`); P-g adds a
  multiplicity check (Codex R17: a repeated line passed the set comparison); the mutation run skips
  records whose postcondition is a machine limitation here (Codex R17: P-f's records "survived" on an
  `env` without `-0`); the timeout verdict in `ci.yml` names reachable commits (Codex R17 P3).
- **focused re-check #5 of `33627692..5a6f4367`**: the skip ran before the trial, so the four P-f
  records that pin the limitation test's narrowness — and do die on such a machine — went unrun; now
  every record runs and only a survival is reported as not exercisable. The census also takes
  symlinks: a link under the root to a git dir outside it was never examined. The `ci.yml` verdict
  describes the PR's final tool code, so X8 is re-run at the final head before the merge.
- **focused re-check #6 of `5a6f4367..a650b146`** (0 IMP): a link was checked at its target's top
  level only — now searched through (`find -L`, through a file so a SIGPIPE cannot read as a failed
  search); the mutation summary counts excused entries; why a 2 is never excused and why the excuse
  is per label are stated in corpus §6.
- **Codex R19** (on `a650b146`): a link to a bare repo whose unborn `HEAD` is a dangling symlink
  escaped the top-level `-e HEAD` test; the `find -L` search (which lists a dangling link by its
  name) closes it — record added. The umbrella's status cell now names #527, which stays true after
  the squash, instead of a branch and a future landing step.
- **focused re-check #7 of `a650b146..8190c684`**: a link NAMED `HEAD` to a directory went to the
  HEAD arm and was never searched — now any link resolving to a directory is searched; the census
  comment states GNU/BSD loop behaviour, the absent time bound (the job timeout ends it, red) and the
  fail-safe red for a link to an in-root repo.
- **focused re-check #8 of `8190c684..0f65e2d4`** (0 IMP; a name × target matrix of links, both
  shells: no shape lets git read a git dir through a link while P-g is green): three statements
  narrowed to what was measured — a `HEAD` link to a directory is red by its own name; GNU find's
  loop exit is unmeasured; only CI has a job timeout. Clerical, so the re-check chain ends here.
- **Codex R23** (on `cb8b0e09`): dropping every non-absolute `PATH` entry (R13) also dropped what a
  `git` wrapper found through `tools/bin` needs — its `#!/usr/bin/env` interpreter beside it — so every
  fixture `git` failed. R11–R13's real trouble was the `~` forms, whose meaning depends on the shell's
  mode; relative and empty entries have one meaning given the directory. Those are now resolved
  against the wire's directory; only `~` entries are dropped — and relative/empty ones when that
  directory's path holds a `:`, which `PATH` cannot carry (focused re-check #9: a `co:lon` checkout
  split the entry and reddened P-j; `cb8b0e09` had been green there).
- **Codex R24** (on `0116209d`): `~` entries are expanded as the wire's bash expands them (`$HOME`, a
  validated login's home) — with `git` pinned, a differing reading can only add or miss a helper
  directory, so a `~/bin` wrapper's helper is found again; P-g's authoritative comparison is of NUL
  records from `--show-origin -z` (a newline-bearing value forged a matching line listing); a symlink
  inside a `.git` dir is red (a `.git/config` link outside reported `file:.git/config`). Records +2.
  Focused re-check #10: 0 IMP; three wording fixes (the login-name pattern admits `~-`/`~0`, which
  bash expands too; the memo's symlink rule narrowed to non-directory links; a stale line-sort
  sentence removed).
- **Codex R25** (on `f8a5f3e6`): `sort -z` is not in every supported `sort` (P1) — records are
  now one `printf %q` line each and sorted as lines; git blocked opening a FIFO `HEAD` (hang) — a
  `HEAD`/`config` that is not a regular file is red before git runs; W3 matched its prefix only at
  a record's start, missing a diagnostic appended to newline-less stderr — now anywhere. Records +2.

Records: **145** (`_MUT_RECORDS_MIN=145`), labels 16, `_MUT_UNRECORDED_MAX=21`. Of the 50 records
after the 95 base: 20 from the implementation, `/simplify` −1, `/code-review` +2, `/elidex-review` +1
(22 when the PR opened at `dccce513`), then Codex R1 +1, P-f clauses +5, P-j +3, P-g status +2,
P-g census +8, P-g multiplicity/records +2, P-g HEAD FIFO +1, W3 +1, P-c +5.

**Codex R26 and the pause (2026-09-28).** R26 (on `8413a4db`) was the third round in a row with an IMP
in the harness (R24, R25, R26), so the loop hit PAUSE. Among its findings:
- **②** a symlink `.git/objects` pointing to a directory outside the fixture root PASSED: fail-open.
  R24's rule inside a `.git` reddens non-directory links only, and a directory link is searched for a
  `HEAD` or `.git`, which an object store does not hold;
- **④** memo §3's W3 definition and §4's W3 row still said "line-start prefix", but the code matches
  anywhere in a record since R25.

The rounds had been adding one case per finding to three enumerations: the `PATH` normaliser
(R11–R13, R23, re-check #9, R24①), the link and two-name cases in P-g (R24, R25), and the absence of any
time bound (R25's own comment: "nothing here has a watchdog"). An independent verdict (Fable) proposed
replacing all three with properties, and the user accepted it on 2026-09-28. That is design memo
draft 11: M-PATH, M-SHAPE and M-WATCHDOG, the untrusted-build stop, W3's wording, and the declared
`objects/info/alternates`.

**PX1, the base-only `~+/bin` experiment** (named E1 in draft 11, renamed because round 8's items are E1–E4; Fable; the artifacts are in the orchestrating session's scratchpad,
`fable/sb-base` and `fable/e1/`). A `git clone --local` of `e8f78896` held `bin/git`, which logs `$PWD`
and then execs `/usr/bin/git`. The wire ran from the clone's root with `PATH='~+/bin:/usr/bin:/bin'`,
and it PASSED (`e1/base.log`). `wrapper.log` has 668 lines, every one the clone's root
(`sort e1/wrapper.log | uniq -c` → one line, 668). So the wrapper served the wire's own reads and **no**
fixture's git: the fixtures `cd`, and `~+/bin` then names a directory with no `git`. A caller `PATH`
whose meaning depends on the working directory was never a supported surface for the build. ⚠ That
reading was **withdrawn in draft 12** (round 10, U3 below). PX1 describes base, which had no pin; under
M-PATH the wire's shell resolves `~+/bin/git` before the window starts, so the wrapper becomes the
fixture git. M-PATH now rests on R1 alone (design memo §0.1).

**Superseded by draft 11**, on R1 (draft 11 cited PX1; draft 12 withdrew that). These are the fixes the normaliser grew:
- R11–R13: relative entries, then `~`, then `~login`, ending in pinning `git` and dropping every
  non-absolute entry;
- resolving relative and empty entries against the wire's directory (R23), and dropping them when that
  directory holds a `:` (re-check #9);
- expanding `~` and `~login` as this bash does (R24①).

Under M-PATH, the callers those fixes served (a `tools/bin` wrapper with its interpreter beside it; a
`~/bin` wrapper with a helper) may go red, which is loud, when a tool they need is only on an entry
that means something else inside the window; X14 records the outcomes. R24's rule for
non-directory links inside a `.git` and R25's `HEAD`/`config` guard are superseded by M-SHAPE. R25's
"nothing here has a watchdog" is superseded by M-WATCHDOG.

**Measurements for draft 11** (this session; `$A` is `…/scratchpad/author`):
- **M-PATH + M-SHAPE prototype, clean.** `$A/sb` is a `git clone --local` of `35dc1153` with
  `fable/sb-e4`'s tool diff applied (`git -C …/fable/sb-e4 diff -- .claude/tools | git apply`), and a
  timer around `_fgit_window` in `controls.sh` (Perl `Time::HiRes`, printing `K2WINDOW <s>`). Three
  runs per shell, two wires at a time, `HOME=$A/home`:
  - bash 5.3: rc 0, PASSED, window 4.99 / 4.82 / 4.83 s, whole run 18 / 16 / 17 s;
  - bash 3.2 (`PATH=/bin:/usr/bin:$PATH`): rc 0, PASSED, window 7.27 / 7.06 / 7.21 s, whole run 25 / 23 / 23 s.

  So no fixture git dir holds anything but regular files and directories, and the window bounds are
  derived from these figures (design memo §3). X3 has **not** been run on the prototype.
- **The untrusted build reaches the controls.** In `fable/e1/e4e.log`, with a FIFO `.git/commondir` in
  `clean`, P-g reported it by the shape rule, and then `green is reachable` was "killed after 30s"
  (the log ends there).
- **`commondir` and `alternates`.** git 2.55.0, in `$A/cd`, `HOME` set to it, `GIT_CONFIG_NOSYSTEM=1`:
  - with `r/.git/commondir` naming `out/.git`, `git -C r config --list --show-origin` lists `out`'s
    configuration with the origin `file:<…>/out/.git/config`, so P-g's origin comparison already
    reddens it;
  - with `a/.git/objects/info/alternates` naming `out`'s object store, `git -C a count-objects -v`
    prints `alternate: <…>/out/.git/objects`. P-g does not ask it, which is the declared blind spot.
- **X4b at `8413a4db`** (the X4b block over `harness.sh`) gives two lines, 71 ("memo §0.1") and 93
  ("memo §1"), with no file name. C6 deletes both.

**Records planned by draft 11** (superseded by draft 12's plan at the end of this section): −1 (P-j per-entry), +1 (W timeout),
+1 (W2 untrusted), +3 (P-g shape) = **+4**. That makes **54** window records after the 95 base, **149**
in all, `_MUT_RECORDS_MIN=149`, labels 16, `_MUT_UNRECORDED_MAX=21`. None of this is implemented yet.

**Plan-review round 10 (on draft 11, frame `9827363e`) → draft 12.** 0 CRIT / 6 IMP / 22 MIN (its findings are labelled K10-1…K10-6 here: round 4 already used U1–U5, and both families have a set-u item), all six IMPs
on draft 11's own additions. Dispositions:

| finding | disposition |
|---|---|
| K10-1 (Ax2): nested `_with_watchdog` groups escape the outer kill | **accepted, redesigned**. One process group per wire run, phases announce their bound, one timer per group; the only nesting is the mutation runner's trials, and the trial's wire joins its group (design memo §3). Cells (a)–(e) below |
| K10-2 (Ax3): the watchdog population was a call-site list, and fsmonitor's `git ls-files` was outside it | **accepted**: membership replaces the list. An unannounced call site is bounded by the phase before it |
| K10-3 (Ax3): under M-PATH, `~+/bin`'s wrapper is pinned as the fixture git | **accepted**; §0.1 rests on R1 alone, and PX1 (draft 11's "E1") is history. Reproduced below |
| K10-4 (Ax3): alternates declared inside class (b), unmeasured | **accepted, closed**: measured below, and closed by P-k (`count-objects -v`), with a liveness probe |
| K10-5 (Ax5): set-u non-reliance measured on 5.3 only | **accepted**: measured on 3.2 and 5.3 at `8413a4db` (below); the parent-side state is enumerated (design memo §3); X13's script is verbatim below |
| K10-6 (Ax4): the §3 code block said "the shape at head" over planned code | **accepted**: the lines C6/C7 change are marked PLANNED |
| Ax3 MINs (6): the bounds' multipliers; X9 and the window time; `_mut_trial`'s timeout disposition; the corpus's "not records" reason against harness 658–664; R1's slot body and its fired trigger; R9 without a slot | **accepted**: one rule (× 4.3 × 2, up to 30 s); the summary prints the window's time; a killed trial's line is appended and judged by its needle; the #501 R92 contract gets a record (corpus §6.1); §9.1's ledger step amends the launch-environment slot; R9 shrank to the pre-controls code, and the audit says it is not a slot |
| Ax5 MINs (9): the abort path with live groups and timers; §9's "Where the record goes"; the #510 interaction; X12's `pgrep` scope; X13/X14 used per commit; X6/X11 commands; X8's threshold; the ledger's stale K10-5 sentence; where C6–C10's history goes | **accepted**: the exit handler kills the group and the timer first, and the timer exits once its leader or directory is gone; §9 rewritten, with the threshold 34.8 s; X12 is scoped to the run's scratch path; §9.1 separates per-commit from final-head criteria; X6 and X11 are spelled out; the amend replaces the sentence in place; history goes to this §13 |
| Ax4 MINs (4): the E1 collision; the corpus header; the bounds' command; `gitrepository-layout` in §2.5 | **accepted**: PX1; the header is rewritten; the commands are below; §2.5 lists it and `count-objects` |
| Premises carried (3; counted in the relayed 22, so 6 + 9 + 4 + 3 = 22): `_control` has no marker today; the top-level census has no `-L`; C5 edited `ci.yml` | **accepted**, as stated in design memo §3/§4/§9 |

**The watchdog cells** (⚠ history: draft 13 withdrew this design, and the cells now serve as the
evidence for `#11-trip-wire-liveness-bound`'s edges. This session; `$A` is `…/scratchpad/author`). The prototype is `$A/sb`, a
clone of `35dc1153` with fable's M-PATH + M-SHAPE diff and the draft-12 bound. The bound is in
`$A/bound.sh`, sourced from the controls file; there, a wire not already in a group re-runs itself as
the leader of one. Each run had `HOME=$A/home`; bash 3.2 ran with `PATH=/bin:/usr/bin:$PATH`, and two
wires ran at a time. After each cell,
`ps -ax -o pid=,command= | /usr/bin/grep -e 'config --list' -e k2ff -e k2blk -e "$A/sb"` listed nothing
from these runs. Two unrelated `plan-memo-umbrella-selftest-trip-wire.sh` processes of another session
did appear once, which is why X12 filters by the run's own scratch path.

| cell | set-up | bash 5.3 | bash 3.2 |
|---|---|---|---|
| K10-1 repro | round 10's `ax2/outer.sh` (per-child groups, nested, inner bound broken), `perl -e 'alarm 20; …'` | perl rc 142 at 20 s; the inner `bash -c` and `cat ff` left behind (killed by hand) | same |
| (a) | `$A/sba2`: the fixtures file gains `mkfifo "$_FW_DIR/k2ff" && git -C "$CTL/clean" config include.path "$_FW_DIR/k2ff"` before `built` | rc 2 at 90 s, W alone | same |
| (b) | `$A/sbb2`: after `green is reachable`, a `_control` over `clean` whose `PATH` prefix holds a `git` that reads a FIFO | rc 1 at 35 s, CONTROL FAILED (the probe's label) | rc 1 at 38 s, same |
| (c) | `$A/ctrial2.sh`: a trial group (cap 330), output captured with `$(…)`, over `sba2` | returned at 90 s; marker: W label, "phase" | same |
| (c′) | as (c) over `sba3`, whose window announces 100000 s, cap 120 | returned at 120 s; marker: W label, "cap 120" | same |
| (d) | `$A/sb`, clean, three runs | rc 0, PASSED ×3 | rc 0, PASSED ×3 |
| (e) | as (d), wall time | 18.5 / 16.8 / 16.6 s | 24.7 / 22.8 / 22.8 s |

(e) is against draft 11's prototype: 18 / 16 / 17 s and 25 / 23 / 23 s. Before settling, the same
cells ran on a variant that grouped only the controls phase. It passed (a), (b), (c) and (d), and was
replaced because it left the scan and the harness setup outside the bound.

The prototype's bound (`$A/bound.sh`) and the trial driver for (c) (`$A/ctrial2.sh`, run as
`bash ctrial2.sh $A <tag> <tree> <cap>`), verbatim:

```sh
# PROTOTYPE (draft 12): one bounded process group per run level; the running
# phase announces its own deadline; a group may also carry an absolute cap.
_k2_phase() { # $1 bound in s (0 = none), $2 kind (W|C), $3 label
  [ -n "${_K2_PHASE:-}" ] || return 0
  printf '%s %s %s %s\n' "$1" "$(date +%s)" "$2" "$3" > "$_K2_PHASE.tmp" && mv -f "$_K2_PHASE.tmp" "$_K2_PHASE"
  [ -z "${K2_PHASELOG:-}" ] || perl -MTime::HiRes=time -e 'printf "%.3f\t%s\n", time, $ARGV[0]' "$3" >> "$K2_PHASELOG"
}
_k2_pgid() { ps -o pgid= -p "$1" 2>/dev/null | tr -d ' '; }
_k2_joined() {
  [ -n "${_K2_PHASE:-}" ] && [ -s "$_K2_PHASE.pgid" ] && [ "$(cat "$_K2_PHASE.pgid")" = "$(_k2_pgid $$)" ]
}
_k2_timer() { # $1 pgid, $2 phase file, $3 fired marker, $4 cap in s (0 = none), $5 group start
  _tg=$1 _tp=$2 _tf=$3 _tc=$4 _ts=$5
  trap 'kill $(jobs -p) 2>/dev/null; exit 0' TERM
  while :; do
    sleep 1 & wait $!
    _tl=""; { IFS= read -r _tl < "$_tp"; } 2>/dev/null || continue
    _tb=${_tl%% *}; _tr=${_tl#* }; _tt=${_tr%% *}; _tn=$(date +%s); _tw=""
    case "$_tb" in ''|*[!0-9]*) _tb=0 ;; esac
    if [ "$_tb" -gt 0 ] && [ $((_tn - _tt)) -ge "$_tb" ]; then _tw=phase
    elif [ "$_tc" -gt 0 ] && [ $((_tn - _ts)) -ge "$_tc" ]; then _tw="cap $_tc"; fi
    if [ -n "$_tw" ]; then
      printf '%s\n%s\n' "$_tl" "$_tw" > "$_tf"
      kill -9 -"$_tg" 2>/dev/null; exit 0
    fi
  done
}
_k2_group() { # $1 fresh dir, $2 cap and first bound, $3 kind, $4 label, then the command.
  _gd="$1"; rm -rf "$_gd"; mkdir "$_gd" || return 2
  _k2_prev_phase="${_K2_PHASE:-}"
  _K2_PHASE="$_gd/phase"; export _K2_PHASE
  _k2_phase "$2" "$3" "$4"; _gc="$2"; shift 4
  set -m; "$@" & _gpid=$!; set +m
  printf '%s\n' "$_gpid" > "$_K2_PHASE.pgid"
  _k2_timer "$_gpid" "$_K2_PHASE" "$_gd/fired" "$_gc" "$(date +%s)" & _tpid=$!
  _k2_rc=0; wait "$_gpid" || _k2_rc=$?
  kill -TERM "$_tpid" 2>/dev/null || true; wait "$_tpid" 2>/dev/null || true
  _k2_fired=""; [ ! -e "$_gd/fired" ] || _k2_fired="$(cat "$_gd/fired")"
  _K2_PHASE="$_k2_prev_phase"
}
```

```sh
# cell (c): a mutation trial run the way the runner will run it: its own group, output CAPTURED.
# $1 = scratch, $2 = tag, $3 = tree, $4 = cap
. "$1/bound.sh"
s=$(date +%s)
out="$(_k2_group "$1/trial.$2" "$4" C "trial ($2)" env -u WEBREF_WIRE_MUTANTS HOME="$1/home" "$BASH" "$1/$3/.claude/tools/webref-generic-core-trip-wire.sh" 2>&1; printf 'FIRED=[%s] rc=%s\n' "$(printf '%s' "$_k2_fired" | tr '\n' '|')" "$_k2_rc")"
echo "$2: trial returned after $(( $(date +%s)-s ))s"; printf '%s\n' "$out" | /usr/bin/grep -e '^!!' -e FIRED | cut -c1-200
```

**Phase times, which the bounds are derived from** (the prototype's `_k2_phase` appends
`<epoch>\t<label>` to `$K2_PHASELOG` when it is set; three clean runs per shell):

```sh
cd $A/sb && env K2_PHASELOG=$A/e/p53.1 HOME=$A/home /opt/homebrew/bin/bash .claude/tools/webref-generic-core-trip-wire.sh
# END appended from the driver; per-phase duration = next line's epoch − this line's
```

The maxima over the six runs: harness setup 0.61 s; window 6.84 s (draft 11's timer: 7.27 s); the
longest control phase 1.12 s (the umask block, together with everything after it up to the scan); the
scan 1.53 s.

**K10-3, reproduced.** In `$A/sb` (M-PATH), `bin/git` logs `$PWD` and execs `/usr/bin/git`. With
`PATH='~+/bin:/usr/bin:/bin'`, both shells gave rc 0, PASSED. The wrapper logged 660 (5.3) and 640 (3.2)
calls from the repository root, plus 750 over the two runs from inside the window and the fixture
directories. Those went to one log file, because the window's `env -i` dropped the variable naming the
per-shell log, so they are not split by shell. Round 10 measured 375 in one run.

**K10-4, measured** (git 2.55.0, `$A/alt`, `HOME=$A/alt`, `GIT_CONFIG_NOSYSTEM=1`). An outside repo `out`
holds a loose object at the id of the content `A\n` whose body is `B\n`. Two repos, `plain` and `alt`,
each commit a file `f` containing `A\n`; `alt` has `objects/info/alternates` naming `out`'s store.
- `ls-files -s` and `HEAD^{tree}` are identical, so P's ids do not move;
- `alt` stores no local copy of the object, and `git -C alt cat-file -p <id>` and `show HEAD:f` print
  `B`. So the content the controls read comes from outside;
- `count-objects -v` prints `alternate: <out's store>` for `alt` and nothing for `plain`, on git 2.55.0
  and Apple 2.54.0.

No fixture uses alternates, `--shared` or `clone`.

**K10-5, measured** (the script below at `8413a4db`; `NOU=1` for off; 36 runs, two at a time). Each cell
gave the same rc, NE/CF counts and first four `!!` lines with nounset on and off, on both shells:
clean 0/0/0 PASSED · sealfail 1 · env0 0 (the machine limitation, green) · garbagehead 1 · w2rec 1 ·
lblrename 2 · w3ar 1 · sealdotdot 1 · reftable 0 PASSED.

**X13: the cell script** (`crc.sh`, trimmed to these nine cells, with its two paths made parameters;
run as `K2_CELLS=<scratch> K2_REPO=<checkout> [NOU=1] bash crc.sh <commit> <b53|b32> <cell>`). A cell
whose anchor has moved fails its `assert`, loudly, and is re-anchored.

```sh
#!/bin/bash
# crc.sh <commit> <b53|b32> <cell> : one /code-review cell on a fresh clone at <commit>
c=$1; sn=$2; k=$3
V=${K2_CELLS:?set K2_CELLS to a scratch dir}
X=$V/crc/$k.$c.$sn${NOU:+.nou}; rm -rf "$X"; mkdir -p "$X/home"
if [ "$sn" = b32 ]; then SH=/bin/bash; P=/bin:/usr/bin:$PATH; else SH=/opt/homebrew/bin/bash; P=$PATH; fi
d="$X/co"
git clone -q --local "${K2_REPO:?set K2_REPO to the checkout}" "$d" && git -C "$d" checkout -q "$c" || { echo "CELL $k clone failed"; exit 0; }
T="$d/.claude/tools/webref-generic-core-trip-wire"
run_cwd="$d"; env_extra=(); sb="$X/sb"; mkdir -p "$sb"
case $k in
  sealfail)
    printf '#!/bin/sh\nfor a in "$@"; do case "$a" in */err/control.py) echo "chmod: refused (cell)" >&2; exit 1;; esac; done\nexec /bin/chmod "$@"\n' > "$sb/chmod"; chmod +x "$sb/chmod"; P="$sb:$P" ;;
  w3ar) python3 - "$T.fixtures.sh" <<'P'
import sys;p=sys.argv[1];s=open(p).read();a='mkdir -p "$CTL/walk/sub"\n';assert s.count(a)==1
s=s.replace(a,a+'_ar=$(( 1/0 ))\n');open(p,'w').write(s)
P
  ;;
  env0)
    printf '#!/bin/sh\nfor a in "$@"; do [ "$a" = -0 ] && { echo "env: illegal option -- 0" >&2; exit 1; }; done\nexec /usr/bin/env "$@"\n' > "$sb/env"; chmod +x "$sb/env"; P="$sb:$P" ;;
  garbagehead) python3 - "$T.fixtures.sh" <<'P'
import sys;p=sys.argv[1];s=open(p).read();a=': > "$_FW_DIR/built"\n';assert s.count(a)==1
s=s.replace(a,'( mkdir -p "$CTL/zzg" && cd "$CTL/zzg" && git init -q . && git config core.excludesFile /nonexistent-k2 && printf "garbage\\n" > .git/HEAD )\n'+a);open(p,'w').write(s)
P
  ;;
  w2rec) python3 - "$T.harness.sh" <<'P'
import sys,re;p=sys.argv[1];s=open(p).read()
i=s.index('_fgit_window_incomplete_exit() {');j=s.index('\n}\n',i);b=s[i:j];assert b.count('\n  exit 2')==1
s=s[:i]+b.replace('\n  exit 2','\n  :')+s[j:]
a=': > "$_FW_DIR/done"\'';assert s.count(a)==1;s=s.replace(a,': > "$_FW_DIR/notdone"\'');open(p,'w').write(s)
P
  ;;
  lblrename) python3 - "$T.controls.sh" <<'P'
import sys;p=sys.argv[1];s=open(p).read();a='_pa_lbl="';assert s.count(a)==1;s=s.replace(a,'_pa_label="');open(p,'w').write(s)
P
  ;;
  reftable)  # a compiled-in reftable default, simulated: a git that picks reftable unless the caller pins a format
    R="$(PATH=$P command -v git)"; printf '#!/bin/sh\n[ -n "${GIT_DEFAULT_REF_FORMAT:-}" ] || { GIT_DEFAULT_REF_FORMAT=reftable; export GIT_DEFAULT_REF_FORMAT; }\nexec %s "$@"\n' "$R" > "$sb/git"; chmod +x "$sb/git"; P="$sb:$P" ;;
  sealdotdot)
    mkdir -p "$X/tmp/k2-sentinel"; chmod 755 "$X/tmp/k2-sentinel"; env_extra=("TMPDIR=$X/tmp")
    python3 - "$T.fixtures.sh" "$k" <<'P'
import sys;p,k=sys.argv[1],sys.argv[2];s=open(p).read();a=': > "$_FW_DIR/built"\n';assert s.count(a)==1
add={'sealdotdot':'_seal "$CTL/walk/../../../k2-sentinel" 000 walk\n',
     'sealsymlink':'ln -s ../../../k2-sentinel "$CTL/walk/lnk" && _seal "$CTL/walk/lnk" 000 walk\n'}[k]
s=s.replace(a,add+a);open(p,'w').write(s)
P
  ;;
  clean) : ;;
esac
log=$X/log
case $k in sealdotdot|sealsymlink) m0="$(ls -ld "$X/tmp/k2-sentinel" | cut -c1-10)";; esac
# NOU=1: the wire with nounset OFF (IMP-2's measurement: if nothing relies on `set -u` to fail, the
# verdicts are the same with it off).
if [ "${NOU:-}" = 1 ]; then python3 - "$T.sh" <<'P'
import sys;p=sys.argv[1];s=open(p).read();a='\nset -euo pipefail\n';assert s.count(a)==1;s=s.replace(a,'\nset -eo pipefail\n');open(p,'w').write(s)
P
fi
XF=; [ "$k" = w2rec ] && XF=-x
( cd "$run_cwd" && env HOME=$X/home PATH="$P" "${env_extra[@]}" $SH $XF "$T.sh" ) > $log 2>&1; rc=$?
echo "CELL $k $c $sn${NOU:+ nounset-OFF} rc=$rc NE=$(/usr/bin/grep -c 'CONTROL NOT EXERCISED' $log) CF=$(/usr/bin/grep -c 'CONTROL FAILED' $log) PASSED=$(/usr/bin/grep -c 'trip-wire PASSED' $log)"
/usr/bin/grep -e '^!!' -e 'NOT EXERCISED on this machine' $log | cut -c1-230 | head -4 | sed 's/^/    /'
case $k in sealdotdot|sealsymlink) echo "    outside-scratch sentinel mode before=$m0 after=$(ls -ld "$X/tmp/k2-sentinel" | cut -c1-10)";; esac
if [ "$k" = w2rec ]; then echo "    non-_control blocks EXECUTED over the unbuilt tree (trace lines naming relcwd/fsmhook/umask 777): $(/usr/bin/grep -c -e 'cd .*/relcwd' -e 'chmod +x .*/fsmhook' -e 'umask 777' $log)"; fi
chmod -R u+rwX "$X" 2>/dev/null
```

**Records planned by drafts 11–12** (⚠ superseded by draft 13's plan below):
- −1 (P-j per-entry);
- +2 (C7: the W FIFO include, and the wire record for #501 R92's contract);
- +1 (W2 untrusted);
- +3 (P-g shape);
- +3 (P-k).

That is **+8**: 58 records of this PR, **153** in all, `_MUT_RECORDS_MIN=153`, labels 18,
`_MUT_UNRECORDED_MAX=21`. None of it is implemented yet.

**Plan-review round 11 (on draft 12, frame `87b964c7`) → draft 13.** 0 CRIT / 12 IMP (8 unique) / 30
MIN. Seven of the eight unique IMPs are cell-backed defects in draft 12's one-group-per-run bound. The
**user decided (2026-09-28) to carve the time bound out of #527**. Draft 13 removes it, and the
defects become the known edges of `#11-trip-wire-liveness-bound` (design memo §5.2 R9):
- a tty with `stty tostop` stops the re-run: a false red;
- with trial nesting, the INT/TERM exit path's `kill -9` does not reach the trial groups;
- a failing pgid probe re-runs the wire without end;
- the join is entered by environment alone, against `webref-generic-core-trip-wire.sh:350–364`;
- the `set -m` check goes by spelling (`set -o monitor`, `-eum`);
- 0-bound phases, and a top-level cap of 0, leave call sites unbounded;
- the new parent relays bash 3.2's masked rc 0.

The eighth IMP, P-k's machine-limitation arm, which could not tell a broken probe from a git without
the line, is fixed: the arm is dropped, so a probe that does not report its `alternate:` line is red.

| finding | disposition |
|---|---|
| the seven bound IMPs above | **carved out**, by user decision: design memo §3 "No time bound", §5.2 R9 and the slot text; draft 12's C7 is withdrawn |
| P-k liveness arm | **fixed**: no machine-limitation arm; the P-k records are re-derived (corpus §6.1) |
| umbrella L22 → memo §8.1 | **fixed**: it now names `…-reviews.md` §8.1 |
| this file's §8.2, bare `§` references | **fixed**: they name the parent |
| the `U…` collision between round 4 (U1–U5) and round 10 | **fixed**: round 10's findings are now K10-1…K10-6 |
| X4b's range | **fixed**: every commit that edits the harness (C6, C8, C9, C10) |
| the threshold, 34.9 against 34.8, and its direction | **fixed**: product ≥ 150 s, that is a slowest run ≥ 150 / 4.3 = 34.883… s |
| "eight red cells", against K10-5 (env0 and reftable green) | **fixed**: "eight cells (six red)" |
| a "#501 R92 contract" quoted with no such text | **removed**, with C7's record and the bound section |
| "(972 lines)" with no command | **fixed**: `wc -l` given |
| round 10's "22 MIN", not reproducible from the table | **fixed**: per-row counts 6 + 9 + 4 + 3 |
| X11 in neither list; X3 per commit | **fixed**: both at the final head; per commit X1, X4b, X10 |
| X10's glob covers the untouched 1259-line wire | **fixed**: `wc -l` only over the parts this PR edits |
| #510's head | **fixed**: `400086ee`, read 2026-09-29 (round 11's message said `83fc1d05`; the head had moved again) |
| R9's wording | **rewritten**: R9 is now the time-bound residual |
| the other round-11 MINs | about the removed bound; they go with it |

**R26③, measured** (this session). The M-SHAPE prototype `$A/sb` is fable's M-PATH + M-SHAPE diff plus a
C8 stand-in (`exit 1` when a postcondition or W3 reported, before any control). The head control is a
`git clone --local` at `8413a4db`. Each cell plants a FIFO in `clean/.git` just before `built`:

```sh
# R26③ cell: $1 = tree, $2 = name planted as a FIFO in clean/.git, $3 = tag
A=${K2_CELLS:?set K2_CELLS to a scratch dir}
T=$A/r3/t.$3; mkdir -p $T && (cd $1 && tar cf - --exclude=./target .) | (cd $T && tar xf -)
python3 - "$T/.claude/tools/webref-generic-core-trip-wire.fixtures.sh" "$2" <<'P'
import sys;p,n=sys.argv[1],sys.argv[2];s=open(p).read();a=': > "$_FW_DIR/built"\n';assert s.count(a)==1
s=s.replace(a,'rm -f "$CTL/clean/.git/%s" && mkfifo "$CTL/clean/.git/%s"\n'%(n,n)+a);open(p,'w').write(s)
P
echo "$T"
```

| FIFO | prototype, bash 5.3 | prototype, bash 3.2 | `8413a4db`, both shells |
|---|---|---|---|
| `commondir` | rc 1 in 5 s: P-g "not a regular file or directory", no control run | rc 1 in 8 s, same | the window waits in `git … config --list --show-origin` (killed by `alarm 40`, rc 142; the blocked git and the window's bash were then killed by hand) |
| `HEAD` | rc 1 in 5 s, same | rc 1 in 7 s, same | — (R25's guard already reds it) |
| `config` | rc 1 in 5 s, same | rc 1 in 8 s, same | — (R25's guard) |
| `config.worktree` | rc 1 in 5 s, same | rc 1 in 7 s, same | **rc 0, PASSED** in 18 / 24 s: git does not read `config.worktree` unless `extensions.worktreeConfig` is set, so it is not an input there. M-SHAPE reds it anyway, which is the fail-safe direction |

After each prototype cell, no process matched `config --list` or the cell's path.

**P-k, re-derived** (corpus §6.1). Each record is killed, and survives with its clause removed:
- the fixture `alternates` survives with the `alternate:` clause removed;
- the liveness probe without its file survives with the liveness check removed. With no limitation arm,
  no machine excuses it;
- the failed `count-objects` survives with the status check removed.

git 2.55.0 and Apple 2.54.0 both print the probe's line (K10-4 above).

**Records planned by draft 13** (design memo §9.1, corpus §6.1): −1 (P-j per-entry), +1 (W2 untrusted),
+3 (P-g shape), +3 (P-k) = **+6**. That makes 56 records of this PR and **151** in all
(`_MUT_RECORDS_MIN=151`), with 18 labels and `_MUT_UNRECORDED_MAX=21`. The runner's count at
`8413a4db` is 145, from `awk -F'\t' '!/^#/ && NF>1'` over the `MUTANTS` here-document. None of it is
implemented yet.
