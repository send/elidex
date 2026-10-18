# K2 fixture git — PR #527's implementation record, after `…-impl.md` closed

This file continues `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl.md` ("`…-impl.md`"), PR #527's
implementation record, for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo").
The design memo's §13 (the rollover, D19-C) sends an entry that would take `…-impl.md` past 800 lines, and every
later one, here.
`…-impl.md` closed at 799 lines with CR-S8a (`161ae982`), the last slice of the branch-wide code review's first half
(CR-S1, S2, S3, S5, S8a); §R ends with a pointer to this file. §R holds only CR-S3 (§R7.12) and CR-S5's change
(§R7.13) of that half: the review round's setup, CR-S1, CR-S2, CR-S5's full opt-in and CR-S8a, which §R does not
hold, were recorded here afterwards, as §S.1, ahead of the second half. Every reference to "`…-impl.md` §R" for a
result §R holds still resolves there.

The working files (decisions, coder progress, reviews, runner cases, logs) are in the driver's handoff directory
outside the repo, as for `…-impl.md`. This file states what they measured, so that a citation in the repo resolves in
the repo; every result below is relayed from those files as the coders, reviewers and the driver recorded it, and was
not re-run for this file, except where an entry says "measured for this file".

---

## §0 Spec coverage map

**No spec surface** — this file records review dispositions and measurements only. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §S The branch-wide code review: the first half's backfill (§S.1), then clusters B and C

After §S.1, the B/C plan (the capability / decline path and the label census) and its slices CR-B1, CR-B2a–c,
CR-C1 and CR-C2, then the second-half docs slice and the full opt-in at the new T. Entries are added here as each is made.

### S.1 The first half

#### S.1.1 The review round, its triage and the instrument

**Review.** `/code-review max 815f8d4b...14d0b35d` (2026-10-17): 15 findings kept, 3 IMPORTANT (the census's variable
half keeps a deleted variable-label producer "used", so deleting the `_cn` standing check is invisible even to the
opt-in; P-b's 129 definition test cannot tell a typo or `var --` from an old git; the bare-label ratchet counts only
executed `_control`s), 12 MINOR, all verifier-CONFIRMED. Also recorded: 7 MINOR dropped at the cap, 9 items in
"cleanup/altitude/docs" (3 code, 6 docs) "unverified this pass", the `pr527-r15.md` :236 pipe count.

**Triage** (designer, `R7-CR-triage.md`, frame `14d0b35d`). N = 32 (15 + 7 + 9 + 1). FIX 30 (23 code rows, 7 doc rows),
DECLARE 2 (C6 under residuals R4, C7 under the memo's §0.3 (c)), OUT-OF-PR 0. By origin: R7's own commits 20, R3–R6 4,
earlier in the PR 3, base 5. About 30 further CONFIRMED items sit in its appendix; under D9 (driver: triage them now) the
plan designer assigned each to a slice or a disposition (plan memo §8), and the 8 with finder lines only (no verifier verdict) are re-measured in their slice first.

**Clusters.** A: `_ctl_run` launch and watchdog with the `_wm` check (K7 K11 K12 K13 U2 K6 C4 U4). B: the capability /
decline path (K4 K10 K14 K15 K8 K9 U3 U8; three mechanisms). C: the label census and ratchet (K1 K3 C3, C5 U9). D: P-b's
machine limit (K2). E: `_ctl_run`'s argument contract (K5 K9 C1 C2 U1; five local defects, one slice). **Order:** S1 (A)
→ S2 (E) → S3 (D) → S5 (B's kill match) → S8a (docs), then the user's checkpoint. The rest of B (S4, S6, D8) and C (S7)
go to one plan memo, plan-reviewed before implementation (`CR-plan-capability-census.md`, 582 lines, 6 slices, 2 open
driver questions, 3 ratified surfaces reversed; not plan-reviewed at the checkpoint).

**Terminator.** 20 of 32 findings came from R7's own commits, the second consecutive round dominated by self-introduced
findings, so the instrument changed again: R7's substitute for plan-review on design-changing slices (designer memo,
driver approval, fresh reviewer) ends. Redesigns of ratified surfaces, or work on three or more intersecting axes, go
through `/elidex-plan-review` first; bounded local FIX slices keep the R7 instrument. Decisions D1–D9: `DECISIONS-R7.md`
"CR round"; D7 (remove the marker) was reversed by S1's measurement (§S.1.2).

**Instrument, per slice.** A claim table (each added sentence with a command), an arm-mutant table (each guard removed →
red), the battery on the slice's base on both shells (bash 5.3.20 and 3.2.57), and a fresh `reviewer` on a frozen
snapshot (tree plus diff) before commit; a fix round is a new coder.

**Scope checkpoint (user, 2026-10-17).** Do the first half, then stop and review scope with the user before B/C.
Early-stop trigger inside it: a slice's fresh reviewer reports a self-introduced IMPORTANT whose fix needs two or more
further review rounds. Data to bring: per-slice review rounds and IMPORTANT counts, the B/C memo's size and open
questions, battery time per slice.

**Battery cost change (user, 2026-10-18).** Base results are cached by base-tree digest (key = sha256 of tree digest
plus the case's inputs; reused only on an exact key; watchdog-killed runs not cached; `BASE_NOCACHE=1` bypasses). 58
superseded cases moved to `cases/retired/` (ported 18, `*pre` 4, EDITFAIL on both trees 15, subject deleted by S3 21);
`CRS*arm`/`claim` cases take a `_` prefix and leave the battery. 192 → 130 cases (131 with `S4bPbsurv_s3`); wall time is
on `### DONE`. The key excludes the runner scripts, so `logs/basecache` is deleted when they change.

#### S.1.2 CR-S1 `c3c0b74b` — watchdog (cluster A)

**Changed.** The child and the watchdog each run as their own process group under `set -m`; the watchdog is stopped by
`kill -9 -"$_wpid"` (K12), the TERM trap and the pid-kill fallback are gone. The timeout marker is kept, written before
the kill with `true >` (K13). Launches are silenced and read `/dev/null` (K11, whose written reason rests on the 0/2500
launch-shape probe under tostop, labelled as such, not as an equivalent mutant). `_ctl_built` is folded into
`_ctl_gate`; the two standing checks run over `$_VFY` and ask no fixture question. `_wm` pins reaping (the hung run's
descendant dead, the watchdog's group gone) and a row with no arm is red (record 288, `hang)` → `hng)`); its hang-row
bound is 1 s (review M3). 221 records, floor 220 unchanged. 6 files, +111/−79.

**D7 reversed.** The marker-free exit-status fact races (group kill of a watchdog that already fired: b53 3/500 and
3/1000, b32 1/1000 lost under `yes`×20; marker-before-kill 0/2000), so the marker stays and AL5 is refuted. D3's pin
needs the census bijection (cluster C): S1 does not pin standing-check presence and does not claim it.

**Regression and fix.** The battery found `R8nowd`, self-introduced: a foreground `sleep "$_ctl_wd_s"` aborted the
watchdog subshell under `set -u` when the bound was unset, so runs lost the watchdog and still PASSED (base reds every
control). Fixed as `sleep "${_ctl_wd_s-}"`. The battery died at 16:26 at 169/171 (DECISIONS-R7 records the driver appending
"the remaining 3 runs"); other
flips were EDITFAIL on rewritten text (7 ported, base=1 ports red).

**Measured (both shells).** Fresh review (Sonnet): 0 CRIT / 0 IMPORTANT / 5 MINOR, all slice-introduced. Q1: `set -m` →
`:` terminates red after about N×`_ctl_wd_s`, no hang. Fixes M1–M5: docs drift, the K11 reason (the 4/20 base false-red
did not reproduce, 0/20 in all cells), hang-row bound, `_ctl_wd_pg` into WHAT IT DEFINES, re-measure on the final tree
(digest `138e74ce`). Ship PASSED on both shells; `scripts/trip-wires.sh` rc 0. The fractional `sleep` on the CI image
(GNU coreutils) is owed: the first push's `trip-wires` job measures it.

#### S.1.3 CR-S2 `945a814d` — `_ctl_run`'s contract (cluster E)

**Changed.** K5: `_control` compares `! [ "$_rc" -eq "$2" ]`, so a swapped or non-numeric expected status is red (new
record). K9: channel arguments are validated on every call, after the gate's registration (new `_cn` row `0 1 - chan -`,
a record restoring the early return). C1: a NOT EXERCISED report shows the run's exit status and output. C2: recorder
names are `_k2rec_`-prefixed and the names the delivery sets are refused before delivery; other bash-maintained names
fall to NOT EXERCISED (red). U1: `fakerelmktemp` and `fakegitcfg` are built bare in the fixture window. EF2/EF3: the
evidence directory and recorder exist only for calls that deliver something. 224 records, `_MUT_UNRECORDED_MAX` 19 → 18
(the new `pin` record makes that label recorded; the ratchet's own message demands it). 8 files, +163/−82.

**Fix round (review M4).** A new `_ev` standing check pins the evidence directory's freshness (2 rows on one call line,
`$_VFY` stubs; +31 lines controls.sh); record 413 (`mkdir -p`) KILLED on both shells, SURVIVES with the row removed.
"two" → "three standing checks" swept.

**Measured (both shells).** The first coder hit the turn limit and its report was lost; one "summarize and finish"
message recovered it (report-loss 1 of 2). Arm rows 3/3, claim rows 5/5, records 21/21 KILLED (digest `a066d58d`).
Battery (base `c3c0b74b`): 11 flips, all EDITFAIL on text S2 rewrote, no pass↔red flip; 10 ported (`S3relmkdirfail` not:
its subject, the deleted parent build, is gone). Fresh review (Sonnet): 0 / 0 / 5 MINOR (4 slice-introduced, 1
pre-existing guard newly claimed, now pinned); Q1 no deliver-but-skip path, Q2 arms unchanged for valid channels. After
the fix round (digest `dd4133bc`): 8/8 records KILLED, 28/28 re-run cases unchanged, ship and `scripts/trip-wires.sh` rc
0.

#### S.1.4 CR-S5's full opt-in, on its final head

CR-S5 is `5f08bd6b` (§R7.13 describes the change; not repeated). The full opt-in at that head (digest `0024f704f873`) is
recorded here. The triage allowed deferring it to X3 at T; the driver ran it at S5 because S6/S7 do not follow at once.

**Measured (`runners/s5opt.sh`; logs `runners/logs/s5-optin-b53.log` and `-b32.log`; shells in parallel; epoch
1792273957 → 1792292706 = 18749 s, about 5.2 h).** `mutation set: 223 entr(ies), 0 not killed as named, 0 not
exercisable on this machine`; `generated boundary set: 50 mutant(s) from $K2RE and $K2RE_PATH, 0 neither killed nor
argued equivalent`; `webref generic-core layering trip-wire PASSED`; `RC=0` on bash 5.3.20 and 3.2.57. No WRONG REASON
(the reviewer predicted none). Battery on the final head (base `598e1aa2`): 131/131 new == base, identical across
shells; against S3 only `S5apb129`, `S5apb129one`, `S5bpbcount` base 0 → 1 (the base moved to the S3 commit).

#### S.1.5 CR-S8a `161ae982` — docs and comments

**Changed (10 files, +56/−45).** U5: `impl.md` :185 pins the commit in the `cmp` command and states the measured
difference. U6: `census.md` :127 command excludes comment lines; "no fixture command names any of them". U7: residuals
:116 and landing :246-247 → "`_control`, the relcwd and fsmonitor blocks and the three standing checks run through it"
(three since S2's `_ev`). R1: `pr527-r15.md` :236 pipe escaped `\|`, 799 lines kept. D7: `census.md` :202 per-commit
part sets, four at `8413a4db` (controls harness mutations mutgen), five at `e271cb02` (+ records, no ctlrun). K-1:
census and impl registered in the umbrella and memo header ("All ten"). K-2: census provenance corrected. K-3: routes
into the closed r15 §Q rerouted. K-4: the unpinned "950 to 729" restated. SW-2: fsmonitor "preflight" wording at the
in-PR sites, three in code comments (ctlrun.sh:56, controls.sh:681-683, records.sh:152-153). C6 DECLARE: residuals R4
carries the correction.

**Fix round (review I-1, I-2, M-1..M-5).** I-2: the first K-3 reroute named a concrete file that is itself full (K-3's
defect regenerated), so every future-tense route now says "the open implementation record (design memo §13)", and §13
alone names the sequence r15 §Q (closed at 799) → `…-impl.md` §R (open while under 800) → the successor named at
rollover. I-1 is the part-set item above; M-1..M-5: landing :351, census :129, landing :200, memo :508, the `impl.md`
footer count; the driver also fixed umbrella :23 (digest `ee08a2bd`).

**Measured.** Fresh review on `crs8-snap` (`c195d986`): 0 / 2 IMPORTANT / 5 MINOR, all slice-introduced except M-4
(pre-existing). Transplanted onto `5f08bd6b` (clone tree == worktree tree `b76818eb`, digest `ee08a2bd`); the `.sh`
delta is comment-only (checked: no non-comment line); records 223; ship PASSED on bash 5.3 and 3.2;
`scripts/trip-wires.sh` rc 0. Battery skipped (comment-only wire delta). `impl.md` was left at 799/800.

#### S.1.6 The checkpoint

| slice | fresh reviews | CRIT / IMP / MIN | self-introduced | fix rounds | early stop | battery (per shell) |
|---|---|---|---|---|---|---|
| S1 | 1 | 0 / 0 / 5 | all 5; plus `R8nowd` (battery) | 1 | no | not recorded (died at 16:26) |
| S2 | 1 | 0 / 0 / 5 | 4 of 5 | 1 | no | not recorded |
| S3 | 1 | 0 / 1 / 3 | all 4 | 1 + a driver-found follow-up | no | about 2 h (192 cases, 02:43 → 04:41) |
| S5 | 1 | 0 / 0 / 5 | all 5 | 1 | no | X 2133 s; base fresh 1958 s (b53), 1970 s (b32); opt-in 5.2 h |
| S8a | 1 | 0 / 2 / 5 | 6 of 7 | 1 | no | skipped (comment-only wire delta) |

Totals: IMPORTANT 3 (S3's unpinned parse guards; S8a's part sets and K-3's route to a full file), all self-introduced,
each closed in one fix round; MINOR 23. Coder stops: one quantity stop each in S1 and S2 (turn limit), 0 capability.
S3's follow-up (unreachable `:*` deleted, `*[!0-9:]*` pinned by `git version 1 .9`) was found by the driver reading the
fix delta.

**Runner defects found on the way.** (1) `bcase.sh` cloned the base once and never fetched later commits, so every base
run of S5's first battery read "base tree is not at 598e1aa2" (pre-existing; fixed by fetching the full sha and
`checkout -f` when HEAD differs; backup `logs/bcase.sh.bak-crs5`). (2) `cache_put` stored those runner failures as base
results and the restart replayed them (introduced by the battery change; fixed so it stores only an output with a
summary line `… rc=` or `EDIT FAILED`; backup `logs/cachelib.sh.bak-crs5`; 262 poisoned entries purged, 131 per shell).
The broken summaries are kept as `battery-*.out.crs5-basebroken` and `…-xonly`.

The first half is done (S1 `c3c0b74b`, S2 `945a814d`, S3 `598e1aa2`, S5 `5f08bd6b`, S8a `161ae982`; pushed head still
`815f8d4b`). The second half (B/C) proceeds in this PR (user, 2026-10-18).
