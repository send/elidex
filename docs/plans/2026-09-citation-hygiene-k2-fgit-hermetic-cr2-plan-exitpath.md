# K2 fixture git — PR #527 CR2 round: the run's exit (plan, §6)

## §0 Spec coverage map

**No spec surface** — the declaration of the plan's §0 (a shell harness's exit path): `preflight.py` exits 1 here by
design. POSIX behaviour is cited by section name with the measurement that shows it on both shells (§6.0).

**Status: accepted (plan-review TERMINAL 2026-10-21, `…-impl-cr2.md` §T.0), the companion of `…-cr2-plan.md` ("the plan")**, split from it at r3 at
its one edge-dense section (touch-time); it moved with the plan at CR2-M (handoff names re-pointed, plan §11).
**§6.0–§6.6 resolve here; every other bare § is the plan's** (which writes these as x§6.n). The plan's conventions hold.

**Texts** (every (M) names its log and text, §6.6): **[r3f]** = the frame + `cr2/bin/r3c2-proto.py` (`73643c0edab38a84`), `r3c2-fix1.py` (`827ea81458483789`), `r3c2-fix2.py` (`3bb517c9c67accc6`) in sequence, each frozen, + the variant and instrument the log names (`r3-clone.sh`, `r3-instr.py` `cb135107ff6b3e46`); logs `cr2/logs/r3f/`. **[r4p]** = [r3f] + `cr2/bin/r4c2-fix3.py` (`d50440af1288588c`, frozen): r4's feasibility prototype of H1–H4 and H17 (not the slice's text; `r4-clone.sh`); logs `cr2/logs/r4/`. **[r5p]** = [r4p] + `cr2/bin/r5c2-fix4.py` (`1ce73d50492f1353`, frozen): r5's prototype of J1, J2, J8, J11 (`r5-clone.sh`); logs `cr2/logs/r5/`. **[r3]** = `r3c2-proto.py` alone (logs `cr2/logs/r3/`), cited only for the two defects its batch found: the stop's 2 s counted from the watchdog's start (fix1: `SECONDS=0`) and b32's "Killed: 9" notices (fix2: `{ … } 2>/dev/null`). **[r2]** = `r2c2-proto.py` (a comparison arm); **[frame]** = `706049ad` (+ an instrument); **[synthetic]** = a probe's own text. Pre-freeze runs (`logs/r3/r3-ship0.*`, `r3-pre-opt*.*`, `logs/r3f/pre/`, `logs/r3f-aborted/`, `logs/r4/attempt*/`, `logs/r5/pre/`) are not evidence for the frozen text; the ones cited (§6.0 R×L, §6.4 R11d and sig, plan §13) show only the defect or form named there.

---

## §6 CR#11, CR#12 and the watchdog's kept stream — the run's exit

### §6.0 The exit path's coupled invariants

The exit path is edge-dense: several mechanisms meet in the few lines that run when the wire ends. The invariants:

| id | invariant | mechanism (r3) |
|---|---|---|
| S | **status**: a run stopped by HUP/INT/TERM exits 128+n; 0 only through `_K2_ORDERLY`; nothing the exit path does changes the status it was given | `_k2_sig` → `exit 128+n`; `_k2_exit "$1"` exits `$1` or the orderly-end 1 |
| R | **reap**: every process group `_ctl_run` has live is stopped when the run ends through its traps; a group counts as stopped only when `kill -0 -<pgid>` fails | the reap set: one file per live group, holding the `$SECONDS` of its record, in `$_K2_REAP_D` (`$_VFY/.reap`) or one directory inside it (the `_og` check's; H1), each written right after its own launch; `_k2_end` drops an entry when its group is gone, or when its id now names a reused leader (H2, J6); `_k2_reap` reads both levels and hands every entry that is still the run's (K) to the stop procedure before the scratch removal |
| C | **a reaped run's own cleanup**: a group the run stops (at its end, or by the watchdog) gets the chance to run its own exit path — a selftest child removes its scratch dir | **one stop procedure**, `$_K2_STOP`: TERM to each group, then until every group is gone or `SECONDS` reaches 2 (1–2 s: it counts whole seconds from the stop text's `SECONDS=0`), then KILL; run as its own process by `_k2_stop` (the reap) and by the watchdog after its marker. **The exception (J13):** at a call's normal end `_k2_end` KILLs without TERM — the child's leader has ended through its own exit path, and what is left (members that outlived it, the watchdog whose marker was not written) runs none of the run's exit path |
| L | **line**: the exit path's lines reach the stderr the run was started with | stderr saved on fd 9 before any trap (`exec 9>&2 \|\| exec 9>/dev/null`); every exit-path line, and the reap set's failure lines, `>&9` |
| E | **errexit in the trap body**: no command of the exit path can end it early or change its status | `set +e` in `_k2_exit`, `_k2_sig` and the mutation run's trap string |
| X | **a second signal during the exit path** | `trap '' HUP INT TERM` is the first statement of `_k2_exit`, of `_k2_sig` and of the mutation run's trap string (before `_mut_rm_copies`) |
| T | **subshell trap reset**: three standing checks run `_ctl_run` in `( … )` (the fourth in `$( ( … ) )`), which resets traps; a signal kills that subshell by its default action | the reap set is files, so the parent's exit path sees the subshell's groups, the fourth's included (its set is inside the run's, H1) |
| G | **`set -m` groups**: the child and the watchdog are their own process groups, so a signal to the wire's group does not reach them | the reap stops them; on SIGKILL of the run nothing does (R15) |
| D | **landing-point redirection**: a trap runs under the redirections active where the signal landed | lines go to fd 9, which no landing redirects |
| F | **fd inheritance**: a child inherits fd 9 unless it is closed, and a child that does not `exec` also keeps the descriptors bash saves around a redirected group or function call (fds 10–12: copies of the caller's stdout and stderr) | both launches close fd 9; the watchdog `exec`s (`env`, then a fresh bash: bash's saved descriptors are close-on-exec); the stop process closes fd 9; the child `exec`s the wire |
| V | **the environment of a new process**: a bash the run starts imports an exported `SHELLOPTS` (errexit, nounset) and reads `BASH_ENV` | the watchdog and the stop process start through `env -u SHELLOPTS -u BASH_ENV` |
| P | **delivery mode**: the signal reaches the run's whole group (a terminal, `kill -- -<pgid>`, a CI cancel) or its pid alone (`kill <pid>`) | group: the trap runs when the landing's command returns, the groups die or are reaped; pid-only: bash runs the trap when the foreground command it waits on returns (§6.0 r3 axes, P×S) |
| M | **the mutation run's trap** replaces the EXIT trap with one that runs `_mut_rm_copies` then `_k2_exit` | `trap '_k2_rc=$?; trap "" HUP INT TERM; set +e; _mut_rm_copies; _k2_exit "$_k2_rc"' EXIT`; the signal traps are the wire's (unchanged) |

Further invariants met in the same lines: **N** (nounset in the exit path) — every expansion there is assigned before the first trap is installed (`_K2_REAP_D=""`, `_K2_STOP`, `SCRATCH`) or defaulted (`${_K2_ORDERLY:-0}`, `${_ord_lbl:-…}`); `"${@:3}"` and `"${#_kr_g[@]}"` on empty lists pass under nounset on both shells (f2probe3 A, C, L); **I** (a signal ignored at entry to a non-interactive shell cannot be trapped or reset: POSIX `trap`, and `dispprobe` (1)) — the signal control's expectation follows a probe of deliverability (§6.4), and the `_wm` check's TERM assertion does the same; **K** (the reap set's entries are acted on only as numbers > 1) — `kill -<sig> -0` is the run's own group and `-1` every process the user may signal, so an unknown entry falls to "skip": a leftover, never a wrong kill; `$!` is never ≤ 1 (f2probe3 L); and (H3) only while its group exists and is the run's — no live leader (a proof only if the group existed throughout: an id is not reused while its group exists; the group emptying, its id reused by a new leader that dies leaving members, is BS-12's residual, J5) or a leader whose `ps -o etime=` (`_k2_age`) is at least the time since the record less 1 s; an unreadable entry, or a `ps` failing in the reap, is the same "skip"; a `ps -o etime=` that cannot read the run's own pid ends the controls at their start, exit 2 "decided nothing" (J1, plan CR11-J). One clock (J14): only `$_K2_STOP`, in its own process, assigns `SECONDS`, and a subshell inherits it (`/usr/bin/grep -nE 'SECONDS *='` over the wire and the parts: that line alone) (`r5/r5probe.txt`; plan §9 BS-12).

**Pairwise intersections of S, R, L, E, T, G, D, F, M** (36 cells; "indep." names why there is none); each instrument
named in a cell and its log are in §6.6 (the exit block under `set -euo pipefail` in 15 cases: `f2probe3`; a signal
at a named landing of the full run: `sigsite3` → `r3-s-*`; a SIGKILL while `_ev` runs: `f3hang`; [synthetic]:
`f3fdleak3`, `f3probe`, `dispprobe`; the ship, opt-in and survive runs `r3-ship*`, `r3-opt*`, `r3-surv*`).

| cell | interaction | mechanism | measured |
|---|---|---|---|
| S×R | a failing `kill` of a dead or empty entry must not change the status | `set +e`; entries numeric > 1; the stop runs as its own process; `_k2_stop` returns 0 | f2probe3 C (stale entry), D (live), B (no reap dir): status kept, both shells |
| S×L | a write to a closed or failed stderr must not change the status | `set +e`; the lines are the last thing before `exit` | f2probe3 E (fd 9 and 2 closed: rc 4 kept), E2 (closed, unordered end: rc 1 = the orderly-end red, kept) |
| S×E | under errexit any failing exit-path command ends the trap with its own status | `set +e` first | f2probe3: [r3f] keeps the status in all 15 cases on both shells; the failing `rm` alone is F (rc 5 kept, its line printed). The frame's own text with a failing `rm`: rc 1 for a PASSED run, both shells (`cr2/logs/cr12-head.*`) |
| S×T | a standing check's subshell dies by the signal's default action; the run's status is its own trap's | `_k2_sig` in the parent | `r3-s-{ev,cap,top}-reap-{TERM,INT}.*`: rc 143 (TERM) / 130 (INT) at `_ev`, `_cap` and top level, both shells |
| S×G | the child's and the watchdog's statuses are `_ctl_run`'s to read, never the wire's | indep.: separate groups | the same runs: rc 143/130 while those groups are live |
| S×D | a redirection at the landing does not touch the status | indep. | f3probe [synthetic]: 143/130 in all 20 cells per shell |
| S×F | descriptors held by children do not touch the status | indep. (a descriptor is not read by the exit path) | — |
| S×M | the mutation run's trap passes `$?` the same way; a failing `_mut_rm_copies` must not change it | `set +e` in the trap string before `_mut_rm_copies` | f2probe3 I: rc 7 kept, both shells |
| R×L | the reap's own messages (b32 job notices for the run's stopped jobs) must not reach the run's output | the stop process (its output on `/dev/null`), its KILL fallback and the `wait`s run inside `{ … } 2>/dev/null`, where bash prints those notices | f2probe3 D, M, N on [r3f]: no notice on either shell; before that grouping, M printed `Killed: 9` on b32 (`cr2/logs/r3f-aborted/f2probe3.proto+fix1.txt`) and [r3]'s `killonly` site two (`cr2/logs/r3/r3-s-top-killonly-TERM.b32.log`); [r3f]'s `killonly` site 0; the sites' counts (§6.4 CR11-N) |
| R×E | an empty reap set or a stale group makes `kill` fail | `set +e`; empty set = no files (the glob literal fails the numeric check) | f2probe3 A, C |
| R×T | a group launched inside a subshell standing check is invisible to a variable in the parent | the reap set is files | `r3-s-{ev,cap}-{reap,noreap}-{TERM,INT}.*`, both shells: with the reap 0 processes left in the run's groups at `_ev` and `_cap`, TERM and INT; with the reap call removed 4 (the stub's child and its `sleep`, the watchdog and its `sleep`); the reap set held two entries just before every signal |
| R×G | the reap must stop whole groups (a blocked `grep` under the child, #501 R92) | `kill -<sig> -<pgid>`; the entries are the group leaders' pids | the same runs: the children of the child included in the count |
| R×D | indep.: the reap writes nothing | — | — |
| R×F | a group the reap misses (SIGKILL of the wire; the launch-to-record instant) must not hold the caller's stream | children close fd 9; the watchdog `exec`s | `f3fdleak3.txt` [synthetic], both shells: the frame's watchdog shape keeps fd 10 (the silencing group's saved stderr) and, under a redirected function call (the standing checks' `_ctl_run … > /dev/null 2> …`), fds 10 and 11, EOF after its 3 s sleep; r3's shape (`exec env -u … "$BASH" -c`) keeps none, EOF ≈ 0.38 s; R11d's mutant shape (`( trap : USR2; env … )`) keeps fds 10–11 again. f3hang [r3f]: a SIGKILL of the run while `_ev` runs → the reader's EOF 0.00 s (b53) / 0.01 s (b32); with fd 9 kept in the children (`nofdclose`) 29.83 / 29.82 s; with a watchdog that stays a bash (`noexecwd`) 29.89 / 29.94 s (`r3-f3hang-{final,keep9,noexecwd}.*`) |
| R×M | the mutation run's trap reaps the parent's own groups; each trial's wire is a separate run with its own `$_VFY` and reap set | `_k2_exit` is the one body | by construction; a TERM during a trial: §6.5 |
| L×E | a line written to a closed fd under errexit ends the trap | `set +e` | f2probe3 E, E2 |
| L×T | a subshell killed by the signal prints nothing of the exit path; bash prints its death notice | the parent's `_k2_sig` prints the line | the `_ev`/`_cap` site runs: the line seen; bash's notice for the standing check's subshell (§6.4 CR11-N) |
| L×G | the children's output goes to `$_out_f` / `/dev/null`, never to the exit path's stream | indep. except through F | — |
| L×D | a line written to fd 2 is lost under the landing's redirection | fd 9 | f3probe [synthetic]: fd 2 loses the signal trap's line at a launch block `{ … } 2>/dev/null` and at a function call under `>/dev/null 2>&1` (b53 both lines, b32 the signal line); fd 9 shows both lines in all 10 of its cells per shell (the probe's other 10 cells are fd 2's) |
| L×F | the selftest child must write its own exit lines to its own output (the signal control reads them) | the child gets fd 9 closed and saves its own at startup | `r3-ship.*` (the rows pass); R11h/i/t killed (§6.4) |
| L×M | a trial's wire writes its lines to its own fd 9 = the trial's captured stream | the trial's wire saves its own fd 9 | `r3-opt*` (the kills read the trial's output) |
| E×T | indep.: the subshells run no exit-path code (their traps are reset) | — | — |
| E×G, T×D, G×M | indep.: the `set -m` groups run none of the exit path (E); a killed subshell has no landing of its own to redirect (D); a trial's groups are its own wire's (M) | — | — |
| E×D | a failed redirection inside the trap body (fd 9 closed) under errexit | `set +e` | f2probe3 E |
| E×F | `exec 9>&2` fails under errexit when stderr is closed at entry, ending the run with 1 | `exec 9>&2 \|\| exec 9>/dev/null` | dispprobe (3) [synthetic]: plain rc 1, the or-form continues, both shells |
| E×M | `_mut_rm_copies` failing under errexit ends the trap before `_k2_exit` (no reap, no scratch removal) | `set +e` in the trap string | f2probe3 I |
| T×G | the subshell's groups outlive the subshell's own death | the file carrier (R×T) | the noreap arms of the subshell sites |
| T×F | a killed subshell's children must not hold the caller's stream, and that subshell redirects `_ctl_run`'s output (bash saves the subshell's own stdout/stderr = the caller's) | fd 9 closed; the watchdog `exec`s | f3fdleak3 `fnredir-*`; f3hang (the SIGKILL lands in `_ev`'s subshell); the `_og` kill row (R11d) |
| T×M | indep.: a trial's standing checks run inside the trial's own wire | — | — |
| G×D | the launch is itself a landing point; under its `{ … } 2>/dev/null` a signal there loses fd-2 lines | fd 9 | f3probe `launchblock` |
| G×F | `set -m` children inherit every open descriptor | `9>&-` on both launches; the watchdog `exec`s | f3fdleak3 |
| D×F | fd 9 escapes every landing's redirection only because it is saved before any | the save precedes the first trap | f3probe (its fd-9 cells) |
| D×M | the mutation run's trap lands at the same points | the same fd 9 | f3probe (one mechanism) |
| F×M | a trial's wire inherits the mutation run's fd 9 (the parent's stderr) until its own startup save re-points it | the save is the wire's first exit-path statement | by construction; `r3-opt*` |

**r3's four axes — C, V, P, X — crossed with every invariant** (the cells r2 did not have):

| cell | interaction | mechanism | measured |
|---|---|---|---|
| C×S | the stop's wait must not change the run's status | `_k2_stop` returns 0; the wait is in another process | f2probe3 D, M, N: rc 3 kept, both shells |
| C×R | the reap is the stop procedure | TERM, 1–2 s, KILL | f2probe3 M (a group that ignores TERM): the exit path takes 1.77 s (b53) / 1.86 s (b32) and the group is gone; N (a group that takes TERM): 0.03 s |
| C×T | the subshell sites' children are stubs with no scratch dir; the top-level site's child is a selftest wire with one | TERM lets it run `_k2_sig` → `_k2_exit` → its own `rm -rf` | top level, TERM, a `_control` child live: the head 0 scratch dirs and 0 processes left; the stop's TERM removed (`killonly`) 1 scratch dir; r2's text 1; the frame 0 scratch dirs only after its unreaped child ran to its end (7 processes left at +1 s), both shells (`r3-s-top-*-TERM.*`) |
| C×G | TERM goes to the whole group, so the child's foreground command dies with it and the child's trap runs | `kill -TERM -<pgid>` | the same runs |
| C×F | the stop process holds nothing | it is a fresh process with fd 9 closed | by construction (f3fdleak3's `execenv` shape) |
| C×M | a trial's wire is in the mutation run's group: the signal reaches it directly and it runs its own exit path | — | §6.5 |
| C×I | a run whose TERM was ignored at entry passes the ignore to its children: the TERM does nothing, KILL follows after 1–2 s, and a stopped selftest child leaves its scratch dir | none (cannot be undone) | f2probe3 M (the bound); `killonly` (the scratch dir, CR11-C); residual R15 |
| C × the watchdog | the watchdog runs the same stop after its marker, so a hung child's own exit path runs before the KILL | `'sleep "$1"; true > "$2" \|\| true; shift 2; '"$_K2_STOP"` | the `_wm` hang row's TERM assertion (R11w); `r3-td-*-wdhang.*`: a real selftest child hung under a 3 s watchdog: the frame's KILL — status 137, 1 scratch dir left; the head — status 143 (the child's own TERM trap ran), 0 left; the TERM step removed — 137, 1; both shells; [r3] (without `SECONDS=0`) — 137, 1 (header) |
| C×X | the run's exit path ignores HUP/INT/TERM while the stop waits, so a second signal does not cut the wait | X | f2probe3 G, J |
| C×V | the stop process does not import the caller's errexit/nounset | `env -u SHELLOPTS -u BASH_ENV` | f2probe3 O (SHELLOPTS exported): rc 3 kept, the group gone |
| C×L, C×E, C×D, C×P | indep.: the stop writes nothing to the run's streams; it runs with no errexit of the parent's; it has no landing; it acts after the trap, whatever the delivery | — | — |
| V×R | the watchdog under an exported SHELLOPTS: errexit makes the unset bound's `sleep ""` end it before its marker and kill (fail-open, R8nowd's case) | `env -u SHELLOPTS -u BASH_ENV` | `g12probe.txt` [synthetic copies of the frame's, [r2]'s and [r3f]'s forms]: r2's `exec "$BASH" -c` under an exported SHELLOPTS: no marker, the target alive, BASH_ENV read, **both shells**; [r3f]'s form: marker, target killed, BASH_ENV not read, both shells; the frame's subshell form (no new bash): marker and kill |
| V×E, V×S | the same channel: errexit in a new process | as V×R | as V×R; the parent's status is untouched (separate process) |
| V × the child | the selftest child is a new bash too and reads an exported SHELLOPTS/BASH_ENV | outside CR2: the wire sets its own options at its first line, and a caller's BASH_ENV reaches the wire itself the same way; no fail-open (the child's verdict is read) | not measured |
| V×L, V×T, V×G, V×D, V×F, V×M, V×P, V×X | indep.: V acts only in a process the run starts (the watchdog, the stop), which writes no exit line (L, D), holds no descriptor (F: `exec`, `9>&-`), is not a subshell of a standing check (T), is its own group or the run's (G, M), and starts after delivery (P) and outside the traps (X) | — | — |
| P×S | a pid-only signal: bash defers the trap until the foreground command it waits on returns; the status is still 128+n | the trap | `r3-s-ev-pidonly-TERM.*`: rc 143 with the line, both shells; the end came 6.23 s (b53) / 6.33 s (b32) after the signal: the `_ev` check's two rows, each stub sleeping 3 s, ran out first |
| P×T | pid-only: the standing check's subshell is not signalled; it finishes its rows, each bounded by its watchdog (`_ctl_wd_s`), so the delay is bounded by the subshell's rows × `_ctl_wd_s` | — | the same runs |
| P×R | pid-only: by the time the trap runs, the subshell's groups have ended normally | — | the same runs: 0 processes left |
| P×M | pid-only during the mutation run: its trap waits for the trial's wire it runs in the foreground, so the end comes one trial later | the trap | argued from P×S (one foreground command); not measured |
| P×G, P×L, P×E, P×D, P×F, P×C, P×V, P×X | indep.: a group signal reaches neither `set -m` group either (G); once the trap runs, the delivery mode is spent, so lines, errexit, landings, descriptors, the stop and a second signal are as for a group signal | — | — |
| X×S, X×M | a TERM during `_mut_rm_copies` in the mutation run's trap | `trap "" HUP INT TERM` before `_mut_rm_copies` | f2probe3 J: [r2]'s string rc 143 and the scratch dir left, both shells; [r3f] rc 7 kept, the scratch dir removed, both shells |
| X×L | `_k2_sig`'s line is printed once | `_k2_sig` ignores the three first | f2probe3 H: one line; G: none (its TERM lands inside `_k2_exit`, which ignores it: rc 6, no line) |
| X×R | the reap's wait is not cut | as C×X | as C×X |
| X×T, X×G, X×D, X×F, X×V, X×P | indep.: X is a property of the trap bodies, which no subshell (T) or `set -m` group (G) runs, which write only to fd 9 (D, F), and which start no new bash before ignoring the three (V); the delivery mode only decides when they start (P) | — | — |

POSIX.1-2017 XCU references for the behaviour relied on, each measured here rather than assumed: "Signals and Error
Handling" (an asynchronous list ignores SIGINT and SIGQUIT when job control is disabled — `dispprobe` (2): a plain
async child ignores INT, a `set -m` one does not); "Shell Execution Environment" (a subshell resets traps that are not
ignored — the noreap arms of the subshell sites); `trap` (signals ignored on entry cannot be trapped — `dispprobe` (1)).
Bash's own behaviour, measured and not POSIX's: a subshell whose body is one simple command `exec`s it (both shells),
and bash 5.3 also `exec`s the last command of a longer subshell; a trap set in the subshell prevents it (f3fdleak3's
`trapstay`, and §6.4 R11d).

### §6.1 Defects, reproduced

- **CR#11 (M) [frame].** The full run, its own process group with default dispositions (`cr2/bin/cr11.sh`; perl resets
  INT, TERM and HUP: a script's background job starts with SIGINT ignored, which made the first INT trial PASS, rc 0,
  `cr2/logs/cr11-INT.*`, an artifact), signalled after 8 s: TERM → rc 143 on both shells; INT → rc 130 on b53, **129**
  on b32; and the run's `_ctl_run` groups alive after it ended — **one `sleep 30` per run** (r1 counted a concurrent
  run, `cr11h-TERM.b53.log`). The pgid-scoped instrument (`cr2/bin/sigsite.sh`: the run's groups taken just before the
  signal, every process still in one 1 s after the end), frame, TERM and INT: **2 processes left per run, both shells —
  the watchdog's subshell and its `sleep 30`** (`cr2/logs/s-frame-TERM.b53.log`: pid 30805 leads its own group, a
  forked bash, with `sleep 30` in it; the child, 30804, has ended). `cr2/bin/trapprobe.sh` [synthetic]: under TERM the
  EXIT trap runs with `$?` = 0 at all three landing points, both shells; under INT it varies (0, 130, 129)
  (`cr2/logs/trapprobe.txt`). So `_k2_exit` takes its 0 branch and prints the false "ended before reaching its
  verdict"; in the wire trials the line was lost under `wait $! 2>/dev/null` (ctlrun.sh:372's shape) in 3 of 4 cells
  (`cr2/bin/trapprobe3.sh`, `cr2/logs/trapprobe3.txt`).
- **CR#12 (M) [frame].** `--selftest` over a one-file clean repo with a `rm` first on `PATH` that fails `rm -rf`
  (`cr2/bin/cr12.sh`, `cr2/work/shim-rm/rm`): `webref generic-core layering trip-wire PASSED` printed, rc 1, both
  shells (`cr2/logs/cr12-head.{b53,b32}.log`); without the shim rc 0. That is the failing removal alone, on the frame's
  text. **The root is wider than the removal:** errexit is live in the whole EXIT-trap body, so any failing command
  there changes the status, and the mutation run's trap string runs `_mut_rm_copies` under errexit before `_k2_exit`
  is reached (mutations.sh:785). (r2's f2probe also listed per-cause figures for r1's text; they are confounded, since
  r1's unguarded `kill` fails first in every case, and are not cited.)
- **(new) The watchdog keeps the caller's stream (M).** `_ctl_run`'s launches are silenced as
  `{ ( … ) … & _cpid=$!; } 2>/dev/null` (ctlrun :346–:350, :367–:369, `c3c0b74b`): bash saves the caller's stderr
  (fd 10) for the group, and under a redirected call (the standing checks' `_ctl_run … > /dev/null 2> …`) its stdout
  too (fds 10, 11). The child `exec`s the wire, which drops them (close-on-exec); the watchdog subshell never `exec`s
  and keeps them (`f3fdleak3.txt` [synthetic], both shells), so after a SIGKILL of the run, or a missed reap, a reader
  of the run's output sees EOF only when its `sleep 30` ends. Measured on instrumented clones (the `_ev` stub sleeps
  3 s so the SIGKILL lands while a group is live; the frame's watchdog shape is r3's `noexecwd` arm, §6.4).
- **(found by plan-review R2) The watchdog's timeout kill and the reap's kill skip a child's own exit path (M).** A
  KILL gives a selftest child no chance to run `_k2_exit`, so the scratch dir it made at wire :401 is left in the
  caller's TMPDIR. On the frame this happens on every watchdog timeout (a hung child); on r2's text also on every reap.
  **(M)** a real selftest child hung under a 3 s watchdog (`r3-instr.py wdhang`, the run's TMPDIR its own and kept, `cr2/bin/r3-tdrun.sh`): [frame] the child dies 137 and 1 scratch dir is left, both shells (`r3-td-frame-wdhang.*`); without the hang, 0 (`r3-td-frame-plain.*`). A TERM at top level while a `_control` child runs: [r2] 1 scratch dir left (`r3-s-top-r2-TERM.*`). The frame leaves none there only because it reaps nothing: its child runs on to its own end (7 processes left at +1 s, `r3-s-top-frame-TERM.*`).

### §6.2 Root causes and the fix

**The roots.** (1) An untrapped signal reaches the EXIT trap with a status that is not the signal's. (2) The exit path
knows nothing of the groups `_ctl_run` made its own (`set -m`, ctlrun.sh:345–:369), and three of the six `_ctl_run`
call sites are inside subshells (the `_wm` check, controls :339–:363; `_ev`, :385–:399; `_cap`, :423–:471), whose
assignments the parent cannot see and whose traps are reset. (3) errexit is live in every exit-path body. (4) The exit
path writes to whatever stderr the interrupted command had. (5) A child that outlives the run and never `exec`s keeps
the descriptors bash saved around it. (6) A group is stopped by KILL alone, which skips its own exit path. (7) One
group KILL can miss a member forked while it lands (CR11-K: [r3f]'s single KILL left a `sleep` 6 times in 600 loaded
calls on b53, the frame's 4 on b32; `r4/race600-*`), and a reap entry outlives its group's id (BS-12).

**The wire, beside the EXIT trap (:422–:431), verbatim** (the code lines of [r5p]'s block; the slice adds the comments
that state the reasons, and the records of §6.4 anchor on these lines):

```sh
exec 9>&2 || exec 9>/dev/null
_K2_ORDERLY=0; _K2_REAP_D=""
_K2_STOP='SECONDS=0
for p; do kill -TERM -"$p"; done
for p; do while kill -0 -"$p" && [ "$SECONDS" -lt 2 ]; do sleep 0.1 || sleep 1; done; done
for p; do kill -9 -"$p"; done; exit 0'
_k2_stop() { local _ks_p
  { env -u SHELLOPTS -u BASH_ENV "$BASH" -c "$_K2_STOP" k2stop "$@" < /dev/null > /dev/null 2>&1 9>&- \
      || for _ks_p in "$@"; do kill -9 -"$_ks_p"; done
    for _ks_p in "$@"; do wait "$_ks_p" || :; done; } 2>/dev/null; return 0; }
_k2_age() { ps -o etime= -p "$1" 2>/dev/null | _K2_AGE="$2" awk '{ n = split($1, f, /[-:]/)
    split("1 60 3600 86400", u, " "); for (i = n; i > 0; i--) s += f[i] * u[n - i + 1]; ok = n >= 2 && n <= 4 && $1 ~ /^[0-9:-]+$/ }
    END { exit !(ok && s >= ENVIRON["_K2_AGE"]) }'; }
_k2_reap() { local _kr_f _kr_p _kr_t; local -a _kr_g=()
  [ -n "$_K2_REAP_D" ] || return 0
  for _kr_f in "$_K2_REAP_D"/* "$_K2_REAP_D"/*/*; do _kr_p="${_kr_f##*/}"
    case "$_kr_p" in ''|*[!0-9]*|0*|1) continue ;; esac
    kill -0 -"$_kr_p" 2>/dev/null || continue
    if kill -0 "$_kr_p" 2>/dev/null; then _kr_t=""; read -r _kr_t 2>/dev/null < "$_kr_f"
      case "$_kr_t" in ''|*[!0-9]*) continue ;; esac
      _k2_age "$_kr_p" $((SECONDS - _kr_t - 1)) || continue; fi
    _kr_g+=("$_kr_p"); done
  [ "${#_kr_g[@]}" -eq 0 ] || _k2_stop "${_kr_g[@]}"; }
_k2_reap_add() { # $1 = the label, $2 = a group just launched, $3... = the call's groups already recorded
  local _ka_p; [ -n "$_K2_REAP_D" ] && printf '%s\n' "$SECONDS" 2>/dev/null > "$_K2_REAP_D/$2" && return 0
  set +m; echo "!! $1: the reap set could not record process group $2, so a signal would leave it running; decided nothing" >&9
  _k2_stop "${@:2}"; for _ka_p in "${@:3}"; do command rm -f "$_K2_REAP_D/$_ka_p" 2>/dev/null; done; exit 2; }
_k2_end() { # $1 = the label, $2 = the child's group (its leader waited), $3 = the watchdog's: each KILLed until empty, then dropped
  local _ke_p _ke_i _ke_f=0; _ctl_left=""; kill -9 -"$3" 2>/dev/null || true; wait "$3" 2>/dev/null || true
  for _ke_p in "${@:2}"; do _ke_i=0
    while kill -0 -"$_ke_p" 2>/dev/null && ! kill -0 "$_ke_p" 2>/dev/null; do
      [ "$_ke_i" -lt 20 ] || { _ctl_left="$_ctl_left $_ke_p"; continue 2; }
      kill -9 -"$_ke_p" 2>/dev/null || true; sleep 0.05 || sleep 1; _ke_i=$((_ke_i + 1)); done
    command rm -f "$_K2_REAP_D/$_ke_p" 2>/dev/null && continue; _ke_f=1
    echo "!! $1: the reap set could not drop process group $_ke_p, so the run's end could stop another's; decided nothing" >&9; done
  [ "$_ke_f" -eq 0 ] || exit 2; }
_k2_exit() { # $1 = the status the shell was leaving with
  trap '' HUP INT TERM; set +e
  _k2_reap
  case "$SCRATCH" in /*/*) chmod -R u+rwX "$SCRATCH" 2>/dev/null
    rm -rf "$SCRATCH" || echo "!! could not remove the scratch dir $SCRATCH (the exit status is the run's own)" >&9 ;; esac
  if [ "$1" -eq 0 ] && [ "${_K2_ORDERLY:-0}" -ne 1 ]; then
    echo "!! CONTROL FAILED (${_ord_lbl:-the run ends only through a verdict}): the run ended before reaching its verdict" >&9
    exit 1
  fi
  exit "$1"
}
_k2_sig() { trap '' HUP INT TERM; set +e; echo "!! the run was stopped by SIG$1, so nothing was decided" >&9; exit "$2"; }
_k2_traps() { trap '_k2_rc=$?; _k2_exit "$_k2_rc"' EXIT
trap '_k2_sig HUP 129' HUP
trap '_k2_sig INT 130' INT
trap '_k2_sig TERM 143' TERM
}; _k2_traps
```

Notes on that text:
- **The `chmod` line has no `|| true`** (the frame's has one): errexit is off in the body, so the one mechanism that
  keeps a failing `chmod` from changing the status is the `set +e` (R12e, measured on this text, §6.4). **The reap runs
  before the scratch removal**: the reap set lives inside the scratch.
- **The stop's bound** starts where the stop text starts: `SECONDS=0` is its first statement, so it does not depend on
  how long the process running it has lived (the watchdog has slept its seconds by then: [r3] without it killed at once,
  see the header). `SECONDS` counts whole wall-clock seconds, so the grace is 1–2 s (f2probe3 M: 1.77 s on b53, 1.86 s
  on b32); `-lt 2` stays because a selftest child's own cleanup ends well inside 1 s (CR11-C: the whole top-level exit
  path 0.26–0.37 s); a `sleep` that rejects `0.1` falls to `sleep 1` (up to 3 s). The poll is `kill -0 -<pgid>` over
  each group; bash reaps its own finished children as they end, so a stopped leader of the run's own does not linger
  as a zombie in the poll (f2probe3 N: 0.03 s).
- **The job notices of a stop are silenced where bash prints them**: bash 3.2 reports a stopped job of the run's own
  while it waits on the stop process, before any later `wait` could collect it quietly, so the stop process, its KILL
  fallback and the `wait`s run inside `{ … } 2>/dev/null` ([r3] printed "Killed: 9" twice, header).
- **A failed reap-set write ends the call** (H17): `_k2_reap_add` restores `set +m` (the child's record runs under `set -m`), writes the cause to fd 9, stops its groups, then removes the recorded entries, and `exit 2`s; `_k2_end` names each entry it cannot drop and `exit 2`s after trying all (H3). At top level the run ends 2; in a standing check's subshell that subshell ends 2, its check reds and the run ends 1 — the line's "decided nothing" is the call's, the run's status its check's red (H19); the cause is on the run's own stderr. `_ctl_run`'s return contract (plan §5) gains that clause. `_k2_stop`'s `wait`s are `|| :` (J8), so neither needs its caller's errexit off (CR11-A).
- **The call's end, `_k2_end`** (H2): the watchdog KILLed and waited (unreaped until then, so its id is not reused), then each group KILLed again every 0.05 s, ≤ 20 times, while it has members and no leader; its entry is dropped when the group is gone, or when its id now names a reused leader (its own was waited, J6), and a group still there goes to `_ctl_left` and keeps its entry for the run's end. This also KILLs what survived the watchdog's TERM when `_ctl_run` cut its poll; the `_wm` check reads `_ctl_left`.
- **One carrier and the identity (H1, H3):** every reap set the run makes is `$_K2_REAP_D` or a directory inside it, and `_k2_reap` reads both levels; an entry's content is its record's `$SECONDS`, and K's identity test is one `_k2_age` per entry with a live leader, at the run's end only (five entries reaped in < 0.1 s, `r5/r5probe.txt` P9), and once on the run's own pid at the controls' start (J1).

**controls.sh :216–:219:** the `_VFY` condition gains `|| ! mkdir "$_VFY/.reap"` (the same "decided nothing", exit 2), and `_K2_REAP_D="$_VFY/.reap"` follows it, then (J1) `_k2_age $$ 0 || { echo "!! decided nothing: the reap's identity check needs \`ps -o etime=\` (read through awk), which this machine does not give" >&2; exit 2; }` (plan CR11-J).

**ctlrun.sh** (code lines −1; the reap-set calls on existing lines):

```sh
    < /dev/null > "$_out_f" 2>&1 9>&- & _cpid=$!; } 2>/dev/null; _k2_reap_add "$_r_for" "$_cpid"
…
  { ( exec env -u SHELLOPTS -u BASH_ENV "$BASH" -c 'sleep "$1"; true > "$2" || true; shift 2; '"$_K2_STOP" \
      k2wd "${_ctl_wd_s-}" "$_r_wmark" "$_cpid" ) < /dev/null > /dev/null 2>&1 9>&- & _wpid=$!; } 2>/dev/null
  set +m
  _ctl_wd_pg="$_wpid"; _k2_reap_add "$_r_for" "$_wpid" "$_cpid"
  wait "$_cpid" 2>/dev/null; _rc=$?
  _k2_end "$_r_for" "$_cpid" "$_wpid"
```

- **Each group is recorded right after its own launch** (G6), before `wait`: the child's instant is the one between its
  fork and its record, and the watchdog's the same; neither record can come before its group's fork (R14).
- **The watchdog keeps its three steps in their order** — sleep, the marker (`true`, not `:`, for the reason :357–:359
  gives), the stop — and adds the stop's TERM and wait before its KILL. The unset bound still fails closed: `sleep ""`
  fails, and in the fresh bash (no errexit: `env -u SHELLOPTS`) the marker and stop follow (R8nowd; g12probe). It
  holds none of bash's saved descriptors (`exec`), and its `9>&-` closes fd 9 before the `exec`.
- **The silencing group stays as it is**, so the tostop evidence (ctlrun :329–:343) is about the same launch shape;
  the slice re-runs its launch-shape probe once on the watchdog's new body (claim row CR11-S).
- **(M) rejected forms:** fd 2 saved on fd 8 around both launches (`cr2/bin/r2c2-proto.fd8-rejected.py`): fds 10–11
  still kept under a redirected call (`cr2/logs/f3fdleak.txt` `fnredir-explicit8`), the stream held 29.8–29.9 s
  (`cr2/logs/b3-f3hang-closed.*`, [r2]-era); r2's `exec "$BASH" -c` without `env -u` (fail-open, V×R).

**mutations.sh:785:** `trap '_k2_rc=$?; trap "" HUP INT TERM; set +e; _mut_rm_copies; _k2_exit "$_k2_rc"' EXIT`.

**CR#12** is the `set +e` (one stated mechanism for every exit-path body: the root) plus the removal's `|| echo`, which
carries only its message. The status stands: a leftover directory does not change what the run proved, and the line
names it.

**Windows left** (plan §9): the instant between each launch and its record (residual R14); the signals the run does not
trap, SIGKILL, and a TERM ignored at the run's entry (residual R15); the opt-in kills under an ignored signal (BS-11,
carried by C2's landing text). A signal ignored at entry is **closed for the ship run** (§6.4: each signal row's
expectation follows deliverability; measured green with INT, HUP and TERM each ignored at entry). A stale entry (r2's
BS-12) is skipped unless still the run's (K); its residue is plan §9 BS-12.

### §6.3 The wire's touch-time question (1286 lines) and its budget

Landing X10 records the wire as "a monolithic cohesive unit … whose named fallback seam is predicate-vs-walk". **The
judgment stands for this touch.** The edited lines are the exit half of one run lifecycle: its other half is the
wire's last statement (`_K2_ORDERLY=1`, :1285), the mutation run replaces the trap with one calling the same
`_k2_exit` (mutations.sh:785), and the parts write its state (`_ord_lbl`, `_K2_REAP_D`, the reap set through
`_k2_reap_add`/`_k2_end`). The exit lifecycle cannot move to a part: its trap is installed before the parts are
found (the wire spells the parts' path itself, controls :132–:133), and a missing part must still end through it. The
named seam would leave the edited code in the wire on either side, and no other seam bounds it.

**Budget, code and comments alike (G22):** [r5p]'s wire is 1331 lines (+45: 43 code lines, two markers); the slice
adds the comments that state the reasons (the :405–:421 comment rewritten; fd 9, the stop, the reap set's format,
identity and failures, the call's end, the traps): ≈ +45. **Stop line = the slice base's `wc -l` + 90** (1376 at
`706049ad`; re-derived after an O-2 merge; its reason: plan §8): a head over it returns to the driver (trimming vs X10's judgment). X10's sentence is written by CR2-C2 from the measured `git log --format=%h --
.claude/tools/webref-generic-core-trip-wire.sh` over #527's range and `wc -l` at each commit it lists — at that point
`8f4ad3c8`, CR2-D's banner commit (six re-pointed comments; the line count unchanged, measured: `cr2/logs/r3/g22-wire.txt`, [frame])
and CR2-C2's commit — not from a pre-written sentence. (Coordination, plan O-2.)

### §6.4 Pins

All (M) here are [r3f] on both shells (logs `cr2/logs/r3f/`), or [r4p] where named (`cr2/logs/r4/`), opt-in runs
filtered to one record (`cr2/bin/r3-optin.sh`, HUP/INT/TERM at default unless named); [r4p]'s records sit at records.sh
:469–:480 (`MUTANTS` follows; [r3f]'s end at :479, and its `r11f` variant's record takes :478).

- **Records R11h, R11i, R11t** pin the three trap lines of the one installer `_k2_traps` (J11: defined and called by the
  wire, called by the `_og` sig row), one each. One new control, **one call line, three rows**,
  after the `k2` control (controls :690):
  `for _sg in HUP:129 INT:130 TERM:143; do … _control "$CTL/clean" "$_sg_e" "$_sg_m" "a signal ends the run red and says so" "" "" "$CTL/sig$_sg_n" || ctl_ok=1; done`,
  label literal (the census's rule), whose row expectation is `128+n` and `stopped by SIG<n>` **unless** a probe shows
  the signal is not deliverable to a child of this run: `{ sh -c "kill -<n> \$\$; exit 0"; } 2>/dev/null` exits 0 only
  when `<n>` was ignored at the wire's entry (then the run cannot be stopped by it, and the row expects 0 and
  `PASSED`). The probe's group redirection silences bash's report of the probe's death. The fixtures file builds three
  PATH-first `git` shims (`$CTL/sig{HUP,INT,TERM}/git`, beside `goptgrep`, fixtures :363–:364), each marking `path`
  and sending its signal to its own group (`kill -<n> 0`) on the HEAD probe (`rev-parse --verify`, wire :1086). The
  child is `_ctl_run`'s, launched under `set -m`, so its SIGINT is not ignored (POSIX "Signals and Error Handling";
  `dispprobe` (2)). Each record replaces its trap line with `:`; expected KILLED AS NAMED (the child dies by the default
  action: 128+n, no `stopped by` line, wrong reason). Survive arm: that row removed from the list. **(M)** each killed,
  each survive arm SURVIVED, ship PASSED with rows 129/130/143; R11i under INT ignored and R11t under TERM ignored
  SURVIVE (BS-11); both shells (§6.6: `r3-opt*`, `r3-surv*`, `r3-ship.*`, `r3-optR11{i,t}-ign*`); on [r5p] each killed
  again, both shells (`r5/r5-optR11{h,i,t}.*`).
- **The probe** is pinned where it can be: removed, the rows expect 128+n always, which on a machine that delivers the
  three signals changes nothing (an equivalent mutant there); its subject is a run that inherits a signal ignored.
  **Claim row CR11-P (M), one text, all three signals:** with INT, HUP or TERM ignored at entry, the ship PASSED with
  the probe, that row expecting 0 (`r3-shipign{INT,HUP,TERM}.*`); without it: `CONTROL FAILED (a signal ends the run red and says so): expected exit 130 (129, 143), got 0`, rc 1, both shells (`r3-noprobe-ign*.*`).
- **The fd choice (line to fd 9) is a claim row, CR11-F, with a measured reason.** The record that sends `_k2_sig`'s
  line to fd 2 (R11f, the `r11f` variant): `MUTANT 261 … SURVIVED` on both shells (`r3-optR11f.*`): every landing a PATH shim can reach in a selftest
  child is inside a command substitution (`_verdict "$(_scan …)"`, wire :1250; `$(git rev-parse --local-env-vars
  2>/dev/null)`, :669), whose subshell dies by the default action while the main shell lands at a top-level
  substitution, where fd 2 is not redirected. The landings where fd 2 loses the line (a launch block, a function call
  under a redirection) exist only in a controls-mode run, which no control runs. CR11-F = f3probe on the slice head.
- **Records R12 and R12e** pin the removal's report and the exit path's errexit-off. One new control,
  `_control "$CTL/clean" 0 "could not remove the scratch dir" "a scratch dir that cannot be removed does not change the verdict" "" "" "$CTL/failrm"`,
  after the signal control. Its prefix holds two shims (fixtures, beside the signal shims): an `rm` that marks `path`
  and **fails exactly the call `rm -rf <dir>`** — the scratch removal, the only `rm -rf` a selftest child makes (wire
  :424; :1144's `rm -f` and every other call pass through to the real `rm`) — after removing `<dir>` with the real `rm`,
  so no run leaks a scratch dir (`[ "${1:-}" = -rf ]`); and a `chmod` that fails `chmod -R …` (the exit path's only
  `chmod`) after running the real one. Under r3's exit path the run still exits 0 and prints the removal's line. **R12**
  drops the `||` clause: the removal fails silently, status 0, the line absent → KILLED AS NAMED (wrong reason).
  **R12e** drops `_k2_exit`'s `set +e` on the verbatim text of §6.2 (the `chmod` line without `|| true`): the failing
  `chmod` ends the trap under errexit with status 1 → KILLED AS NAMED (expected exit 0, got 1). **(M)** both killed;
  survive arms (R12: the control expecting `PASSED`; R12e: no `chmod` shim) SURVIVED (§6.6). Removing the control's
  call is not a survive arm: it orphans both needles and the correspondence check reds (`cr2/logs/b4-survR12.b53.log`).
- **Record R11w** pins the stop procedure's TERM step, through the watchdog: the `_wm` check's hang stub becomes
  `trap ': > "$0.term"; exit 143' TERM; sleep 5 & echo $! > "$0.pid"; wait`, and its hang row asserts
  `[ "$_wm_tg" -eq 0 ] || [ -e "$_VFY/.wm_hang.term" ]` ("the hung run was sent no TERM before its KILL, so its own exit
  path could not run"), where `_wm_tg` is the same deliverability probe for TERM, run once before the check (a run that
  inherits TERM ignored cannot deliver it, and the assertion stands down: ship (M) PASSED with TERM ignored,
  `r3-shipignTERM.*`). R11w turns `$_K2_STOP`'s TERM line into `:`. **(M)** R11w `0 not killed as named`; survive arm (the row's TERM assertion removed) `MUTANT 257 … SURVIVED`; both shells (`r3-optR11w.*`, `r3-survR11w.*`). The one red seen outside a record ([r3f], `r3-survR11i.b53.log`: "the watchdog's process group outlived the call") is root (7) of §6.2, a member forked while the group KILL landed; [r4p]'s `_k2_end` re-KILLs until the group is gone and the check reads `_ctl_left` (CR11-K).
- **Records R11d, R11e, R11a, R11x, R11r, R11n** pin the fourth standing check's subjects (label `_og_lbl`, "a call's process groups end with it and hold none of its caller's streams"; a non-`_control` label beside `_wm_lbl` in controls :108–:118). The check, after `_cap`'s (controls :475), runs `_ctl_run` over `$_VFY` in **four rows, one call line**, each inside `$( ( … ) )`, the call in an inner `( _K2_REAP_D=…/og; _ctl_run … ) 9>&-` (J2; the set inside the run's, H1) with a 5 s watchdog; the call sends fd 9 into the substitution (`9>&1`, written before its `> /dev/null`), and the rows read its fd-9 line from the substitution's output. The inner starts with fd 9 closed: bash 3.2 saves a redirected fd 9 at fd 10 without close-on-exec, and the `exec`ing watchdog kept it (`r5/pre/b32-sig-hold.*`). Each row starts, and the check ends, with `rm -rf "$_K2_REAP_D/og"`, which deletes a live entry only after a red row (each row confirms its groups gone first: kill and sig by `_og_gone`, norec its one group, nodel by `_k2_end`'s confirmation, without which it does not end 2): a red-run-only leftover (J16, plan CR11-O).
  - **kill**: the stub waits until both groups' entries hold their content, then `kill -KILL $PPID` — it SIGKILLs the
    shell running the call (waiting makes the watchdog certain to exist and the recorder never cut mid-write, §13). The substitution must end at
    once (in under 3 s by `SECONDS`), not when the watchdog's sleep ends; then the check calls `_k2_reap` on what the
    killed call left and asserts each group gone. **R11d** makes the watchdog a bash that stays: `( trap : USR2; env …
    )` instead of `( exec env … )`. Dropping `exec` alone is an equivalent mutant on both shells, which `exec` a
    subshell's only command themselves (measured before the freeze: that form SURVIVED, `cr2/logs/r3/r3-pre-opt475.b53.log`);
    a trap set in the subshell stops the implicit `exec` (f3fdleak3 `trapstay`), which is the frame's watchdog shape.
    **R11e** drops `9>&-` from the watchdog's launch: the watchdog then holds fd 9, which in this row is the
    substitution's pipe.
  - **norec**: the reap set points at a missing directory, so the first `_k2_reap_add` fails. The call must end its shell with 2, name the group on fd 9, and leave no process in it. **R11a** turns the add arm's last line (`_k2_stop …; … exit 2; }`, re-anchored on [r4p]'s text) into `return 0; }`.
  - **nodel**: the stub waits for its own entry and replaces it with a directory, which `rm -f` cannot remove (root included), so `_k2_end`'s drop fails. The call must end with 2 and name the group. **R11x** drops `_k2_end`'s `exit 2` (re-anchored on [r4p]'s text).
  - **sig** (H4, J2): the outer holds the wire's traps (`_og_trap`: a scratch dir of its own, `exec 9>&1`, its pid in `.og_outer`, then `_k2_traps`, J11) and the run's `_K2_REAP_D`; the stub, once both entries hold content, TERMs the outer, then the inner (`$PPID`, which dies by the default action, T), so the outer's exit path must read the nested set. It must end 143 with "stopped by SIGTERM", its scratch dir gone and both recorded groups gone — CR#11's core. **R11r** turns `_k2_exit`'s `_k2_reap` into `:`; **R11n** shrinks `_k2_reap`'s glob to one level. The row stands down when TERM cannot reach a child (`_wm_tg`, BS-11). **(M) [r5p]:** R11r, R11n `0 not killed as named`, R11n's survive arm (the row's assertions removed) `MUTANT 264 … SURVIVED`, both shells (`r5/r5-{opt,surv}R11n.*`, `r5/r5-optR11r.*`); the ship without the reap call reds there (`r5/r5-noreapship.*`); ship PASSED (`r5/r5-ship.*`); TERM ignored at entry: [r4p] (`r4/r4-shipignTERM.*`).
  **(M)** R11d, R11e, R11a, R11x `0 not killed as named`; their survive arms (that row's assertions removed) `MUTANT 258`, `261`, `259`, `260 … SURVIVED`; both shells (`r3-opt{R11d,R11e,R11a,R11x}.*`, `r3-surv{R11d,R11e,R11a,R11x}.*`); ship PASSED with the check (`r3-ship.*`); on [r4p] R11d, R11e, R11a, R11x, R11g, R11w again `0 not killed as named`, both shells (`r4/r4-opt*.*`), and on [r5p] (the nested call) R11d, R11e, R11a, R11x (`r5/r5-opt*.*`). The check's own row loop is pinned as GC-3 pins the other three: **R11g** (`for _og_row in kill norec
  nodel sig` emptied; the census's bijection reds a call line no run registered, needle `$_cen_lbl`'s value): `0 not killed as named`, both shells (`r3-optR11g.*`).
- **CR11-R** (the reap from every call site, the `_og` rows included) is plan §10's; its records are R11r and R11n (above).
- **CR11-N, decided:** the stop process and the `wait` on each stopped job run inside `{ … } 2>/dev/null`
  (`_k2_stop`). Measured: no `Killed: 9` / `Terminated: 15` notice for a reaped job (f2probe3 D, M, N; the top-level
  sites 0; with the stop's KILL forced, `killonly`, 0 on [r3f] against 2 on [r3]); the one line at `_ev`/`_cap` under
  TERM is bash's own notice for the standing check's subshell, both shells. That notice (`Terminated: 15`) stays: it reports a true event of a run being stopped, and silencing it would mean
  a redirection around every standing check. Stated beside the trap lines.
- **Budget** (plan §8): wire +43 code lines and ≈ +45 comment lines, stop line base + 90 (§6.3); ctlrun −1 code line,
  comments in place, stop line 439 with C1's +2; controls ≈ +65 ([r5p]: +61); fixtures +5; records +13 (R11h, R11i,
  R11t, R12, R12e, R11w, R11d, R11a, R11x, R11e, R11g, R11r, R11n); census call lines 89 → 92; `_MUT_UNRECORDED_MAX` stays 18.

### §6.5 The carve's known edges (plan §1)

`#11-trip-wire-liveness-bound` lists seven defects of draft 12's time bound (`…-pr527.md` :413–:419). CR2 builds no
bound, so four do not arise (the failing pgid probe, the join entered by environment, the `set -m` check by spelling,
0-bound phases). The three that touch the lines CR2 edits:
- **Trial nesting × the exit path's kill (M).** A TERM to the opt-in run's group during R12's trial (`r3-trialsig.sh`,
  [r3f]): rc 143 with the line, nothing left (plan CR11-T). The trial's wire is in the mutation run's group, so it
  runs its own exit path over its own reap set, and the mutation run's trap then runs its own.
- **`stty tostop` (argued; claim row CR11-S).** The launch shape the tostop evidence covers is unchanged: the same
  `{ … & } 2>/dev/null` group and `< /dev/null`; the watchdog's new body runs inside the same forked subshell after
  bash's `setpgid`, so it adds no write before the child's own redirect. The slice re-runs the launch-shape probe.
- **bash 3.2's masked rc 0 (M).** The draft-12 parent relayed the child's masked 0; r3 relays nothing: a signal's
  status is `_k2_sig`'s own `exit 128+n` (b32: rows 129/130/143 in `r3-ship.b32.log`; the sites 143/130).

### §6.6 Evidence index

Every (M) of §6.0–§6.5, with its text and its log (paths under `cr2/logs/`):

| log (`cr2/logs/`) | text | instrument | result, both shells unless named |
|---|---|---|---|
| `r3f/f2probe3.txt` | [r3f], [r2] | `f2probe3.sh` | [r3f] keeps the status in all 15 cases; J: [r2] 143 + scratch left, [r3f] 7 kept; M 1.77 / 1.86 s; L: only `5` reaches the stop; no job notice |
| `r3f/g12probe.txt` | [synthetic] copies of [frame], [r2], [r3f] forms | `g12probe.sh` | exported SHELLOPTS: [r2] no marker, target alive, BASH_ENV read; [r3f] marker, kill, BASH_ENV not read |
| `r3/f3fdleak3.txt` | [synthetic] | `f3fdleak3.sh` | the frame's shape keeps fd 10 (fds 10–11 under a redirected call); `execenv` keeps none; `trapstay` (R11d's shape) keeps them |
| `f3probe.txt`, `dispprobe.txt`, `trapprobe.txt`, `trapprobe3.txt` | [synthetic] (r2's runs) | `f3probe.sh`, `dispprobe.sh`, `trapprobe*.sh` | as cited in §6.0 and §6.1 |
| `r3f/r3-ship.*`, `r3-shipign{INT,HUP,TERM}.*` | [r3f] + `instr-sigshow` | `r3-runsig.sh` | PASSED, rc 0; rows 129/130/143, the ignored one 0 |
| `r3f/r3-noprobe-ign{INT,HUP,TERM}.*` | [r3f] `noprobe` | `r3-runsig.sh` | rc 1: that row "expected exit 130 / 129 / 143, got 0" |
| `r3f/r3-noreapship.*` | [r3f] `noreap` | `r3-runsig.sh` | PASSED, rc 0 (r3's unpinned reap call; [r4p]'s R11r pins it) |
| `r3f/r3-opt{R11h,R11i,R11t,R12,R12e,R11w,R11d,R11e,R11a,R11x,R11g}.*` | [r3f] | `r3-optin.sh … default` | `0 not killed as named`, each |
| `r3f/r3-surv{R11h,R11i,R11t,R12,R12e,R11w,R11d,R11e,R11a,R11x}.*` | [r3f] + the arm's variant | `r3-optin.sh … default` | `MUTANT 252–261 … SURVIVED`, each (R11i on b53: `r3-survR11i-rerun.b53.log`, see §6.4 R11w) |
| `r3f/r3-optR11f.*` | [r3f] `r11f` | `r3-optin.sh` | `MUTANT 261 … SURVIVED` (CR11-F's reason) |
| `r3f/r3-optR11i-ignINT.*`, `r3-optR11t-ignTERM.*` | [r3f] | `r3-optin.sh … ign*` | SURVIVED (BS-11) |
| `r3f/r3-s-{ev,cap,top}-{reap,noreap}-{TERM,INT}.*` | [r3f] (+ `noreap`) + `instr-ev` / `instr-cap` / `instr-top` | `sigsite3.sh` | rc 143/130; reap: 0 left, 0 scratch dirs; noreap: 4 left at `_ev`/`_cap`, 7 at top level |
| `r3f/r3-s-top-{frame,r2,killonly}-TERM.*` | [frame], [r2], [r3f] `killonly`, each + `instr-top` | `sigsite3.sh` | frame 7 left, 0 scratch after 40 s; r2 0 left, 1 scratch; killonly 0 left, 1 scratch, 1.82–1.85 s |
| `r3f/r3-s-ev-pidonly-TERM.*` | [r3f] + `instr-ev` | `sigsite3.sh … pidonly` | rc 143, 6.23 / 6.33 s, 0 left |
| `r3f/r3-f3hang-{final,keep9,noexecwd}.*` | [r3f] (+ `nofdclose` / `noexecwd`) + `instr-ev` | `f3hang.sh` | EOF 0.00–0.01 s; 29.82–29.83 s; 29.89–29.94 s |
| `r3f/r3-td-{frame-plain,frame-wdhang,r3-wdhang,killonly-wdhang}.*` | [frame], [frame] + `wdhang`, [r3f] + `wdhang`, [r3f] `killonly` + `wdhang` | `r3-tdrun.sh` | 0; 137 and 1 scratch; 143 and 0; 137 and 1 |
| `r3f/r3-trial-TERM.*` | [r3f] | `r3-trialsig.sh` | rc 143, 0 left, 0 scratch, 0 copies |
| `r4/r4-ship.*`, `r4-shipignTERM.*`, `r4-noreapship.*` | [r4p] (+ `noreap`) | `r3-runsig.sh` | PASSED, rc 0 (TERM ignored: the sig row stands down); `noreap`: rc 1, `$_og_lbl` "sig: the exit path left group N running" |
| `r4/r4-opt{R11r,R11a,R11x,R11g,R11d,R11e,R11w}.*`, `r4-survR11r.*` | [r4p] (+ `og-nosig`) | `r3-optin.sh … default` | `0 not killed as named`, each; `MUTANT 263 … SURVIVED` |
| `r4/r4probe.txt` | [synthetic] harness, [r4p]'s exit block | `r4probe.sh` | P1–P9 as CR11-I / CR11-A state |
| `r5/r5-ship.*`, `r5-shipps.*`, `r5-noreapship.*` | [r5p] (+ `noreap`; `shipps`: `work/r5-ps-shim/ps`, `exit 1`, first on PATH) | `r5-batch.sh` | PASSED; `shipps` rc 2 with J1's line; `noreap` rc 1, `$_og_lbl` "sig: the exit path left group N running" |
| `r5/r5-opt{R11n,R11r,R11h,R11i,R11t,R11d,R11e,R11a,R11x}.*`, `r5-survR11n.*` | [r5p] (+ `og-nosig`) | `r3-optin.sh … default` | `0 not killed as named`, each; `MUTANT 264 … SURVIVED` |
| `r5/r5probe.txt` | [synthetic] harness, [r5p]'s exit block ([r4p]'s for P8r) | `r5probe.sh` | P1–P10 as CR11-I / CR11-A state |
| `r5/pre/b32-sig-hold.*` | pre-freeze [r5p] without the inner's `9>&-` | `r3-runsig.sh` + a stub dump | b32: the watchdog kept fd 10 = the substitution, the sig row ended after 5 s (that defect only) |
| `r4/race600-*.txt` (`race-*.txt`: N = 300) | [synthetic] (Axis 2's `race.sh` forms) | `r4race.sh`, `r4race-batch.sh` | N = 600, load: b32 frame 4 / `_k2_end` 0; b53 [r3f] 6 / 0 (N = 300: 0 / 0, non-discriminating) |
| `r3/g22-wire.txt` | [frame] + CR2-D's six re-points | python `str.replace` in a clone | 1286 lines, the longest 136 characters |
| `r3/r3-*` (all) | [r3] | the same instruments | the record of [r3]'s two defects (header); not cited otherwise |
| `cr11-*`, `s-frame-*`, `cr12-head.*`, `b3-f3hang-closed.*`, `b4-survR12.b53.log` | [frame], r2-era prototypes | `cr11.sh`, `sigsite.sh`, `cr12.sh`, `f3hang.sh`, `r2-optin.sh` | as cited in §6.1 and §6.2 (the defects and the rejected forms) |
