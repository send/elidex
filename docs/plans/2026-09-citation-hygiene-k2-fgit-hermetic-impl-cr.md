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
plan designer assigned each to a slice or a disposition (plan memo §8, now `…-cr-sweep.md` §F), and the 8 with finder lines only (no verifier verdict) are re-measured in their slice first.

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

### S.2 CR-B1 — k2 is posed on every machine; the plan memo and its ledger move into the repo

The first slice of the B/C plan (`…-cr-plan.md` §2.1 and §7). Three commits: the §10 split (the plan memo's revision-error
bullets → `…-reviews.md` §CR, verbatim, registered in the design memo's header), the move (the memo and the sweep ledger
into `docs/plans/` as `…-cr-plan.md` and `…-cr-sweep.md`; the four instruments into the sweep file's §E; the design memo's
header and the umbrella's row A-i-wire-fgit register both, "All eleven" → thirteen), and the code. Two things about the commits, recorded here because the plan memo's §7 (e) says otherwise or cannot know them.
The move commit (`d5902a94`) also carries the §8 → sweep §F split, although §7 (e) said it "carries no reorganisation": the
driver decided it, because the moved memo was 1010 lines and folding the split into the move keeps every commit under 1000
lines. And the split commit (`d237b117`) has §CR name `…-cr-plan.md` one commit before that file exists (the move commit
creates it); the name resolves from the move commit on.

**Changed.** The harness's `GREP_OPTIONS` probe and its comment (29 lines with the blank after) are gone, and so are k2's `_ctl_need` prefix, its summary
line and its ask in the controls file, k2's `_ctl_need` prefix record, and the comment lines that named them. k2 is posed on every machine through `goptgrep`, a grep shim in the fixtures file beside `fakegrep`: it marks `path`,
answers "no match" whenever a `GREP_OPTIONS` reached it, and execs the real grep otherwise; the control's claim moved into the
summary's static lines. `_MUT_RECORDS_MIN` = 222 (measured, `_mutants | wc -l`); corpus §6.1 has the "remove" row and the new
totals; residuals R10 carries the #519 banner's correction (`…-cr-plan.md` §7). `…-cr-plan.md` §7's R3-B2a-3 pointer to §10 was
already absent from the text moved.

**Measured** (every log under the driver's handoff directory `crb1-logs/`; the claim and arm tables are `crb1-logs/CLAIMS.md`).
Ship PASSED, rc 0, on bash 5.3 and 3.2 under BSD `grep` and under the GNU-like profile (a `grep` first on `PATH` that unsets
`GREP_OPTIONS`). Record 100 (`unset GREP_OPTIONS` → `:`) is KILLED AS NAMED on all four. With the shim's `GREP_OPTIONS` arm
removed it is KILLED on BSD and SURVIVES on the GNU-like profile, both shells (the arm is what the GNU-like profile kills it by).
The shim's `path` mark removed → `CONTROL NOT EXERCISED (<k2>): PATH prefix never reached the run`, both shells. The floor is pinned in isolation:
deleting `records.sh` :288 (its label is shared with :101, so the label census stays green) → `!! the mutation set holds 221
records, against a floor of 222.` as the only `!!` line, rc 1, both shells; with the floor lowered to 221 as well, rc 0, PASSED,
both shells. k2's PATH prefix argument (`"$CTL/goptgrep"`, controls :615) has no shipped row: emptied, ship stays green (the path
demand goes with the argument); only record 100 on the GNU-like profile pins it (emptied + record 100: SURVIVED, rc 0, both
shells; on BSD it is KILLED AS NAMED regardless).
`ledger-check.py B1` passes at the code tree and, at the frame, reproduces the per-slice violation counts (B1 32, B2a 151,
B2b 160, B2c 176, C1 218, C2 221); `sweep.sh` reads 26 files and prints identical output on both shells; `completion.sh`'s
B1 greps: parts and wire 0, design memo / census / landing / residuals 0, corpus 1 (:321, history), `three capability probes` 0;
`scripts/trip-wires.sh` rc 0. harness.sh is 886 lines (expected 887: the blank line after the probe went with it).

**Sweep re-run, the new sites** (every `sweep.sh` hit at the head that the frame's run lacks, by `(identifier, file, text)`, both
runs with §E's `idents.tsv` as the review fixes widened it: 54 entries, counted with this section's own text in the file set). The shim,
its comment, its call, the static summary clause and the comments that name k2's mechanism (controls :600, :604, :605, :613, :615,
:692; fixtures :100, :106, :363-364; mutations :248: 13 entries) are `stays` code. controls :25-26 and records :367 (4) are the
two-owner rows (B1's part gone, B2a's still there). `_MUT_RECORDS_MIN=222` (mutations :249, corpus :336: 2) is the ratchet figure.
The design memo :17 and umbrella :23 and :111 (4: :23 is two entries) are the registration rows. Corpus :322, :328 and :329 (5) are §6.1 history
rows. Residuals :7 and :117, the design memo :19, :31, :56 and :644 (7) are the residual range and R10 (`resid_range`, a group this
slice adds to `idents.tsv`); the widened ERE also finds, at the frame as at the head, `…-pr527.md` :657 (a past review row quoting
the rule as "a bare R1–R9", `hist`, stays) and, as new text, the design memo :19 and umbrella :23 ("plan-review R1–R3": rounds of
a review, not the residuals; it matches the range's spelling only, stays); the two unquoted "R1–R9" of the design memo (:31, :644)
became "R1–R10". reviews :664-689 (7) are the moved bullets, `hist` (a dated record of past rounds). This section's own lines
(12) name the slice's identifiers as a record of it, `hist`. None is unclassified. **The
wrapped-identifier backstop** (`perl -0777`, `[\s#]+` between the words, the 26 files): the same five matches at the frame and at
the head (harness :750, mutations :198 and :740, landing :74, reviews :624), none introduced.

### S.3 CR-B2a — one capability table, read by one reader; the declined-by-prefix mechanism is gone

The second slice of the B/C plan (`…-cr-plan.md` §2.2 and §7). Four commits on the branch `crb2a` (base `0848a850`, CR-B1's head):
the code (`c1ca2a4d`), its doc rows (`fea3a0c6`), the review's code fixes (`55f97949`) and its doc fixes (`4a2c2ca6`). The plan
memo's §7 cannot know three things about them, recorded here. The slice was cut into three coder units by the driver (code; the
claim tables; the doc rows and this entry), because CR-B1's coder had stopped at its turn limit; the code unit stopped once at 80
turns, one "summarize and finish" recovered its report, and the unit's last question (GB2-14, below) went to the driver. The
`4a2c2ca6` commit is titled docs but also edits one message line in `ctlrun.sh` and comments in `records.sh`. And the records' line numbers shift between these four commits, so this entry names a record by its
property.

**Changed (`c1ca2a4d`).** The table: `_CAP_TABLE` in the controls file, after the label check and before the harness is sourced,
rows TAB-separated with no empty field (`group` 6 fields, `member` 3, `limit` 4, `fact` 3); at the head 11 rows: the `fifo` group
with 2 members, the `perm` group with 5, the P-f limit row and the P-b fact row. The reader: `_cap_rows <kind|-> <callback>` in
`ctlrun.sh`, the only code that expands the table, with function-local cursor and fields, fields split by parameter expansion, a
row that does not parse flagged `_cr_bad` for the callback, and the lookups `_cap_group_of` and `_cap_flag_of` (the one indirect
expansion, after the flag field is checked to be a variable name) on top of it. The guard: `ctlrun.sh`'s source-time name guard,
which names a missing consumed name and the table's own flag names with an exit 2 instead of an unbound abort. The gate arm:
`_ctl_gate` now asks, in this order, registration, trust, capability, fixture; a member of a group whose flag is 0 returns 2 (no
run, nothing reported), a member of a group the table does not declare returns 1 under the member's own label. `_cap`, a third
standing check beside `_wm` and `_ev` (opens on `_fw_trusted` in their shape; ten rows M, N, Y, U, the untrusted row, X, F1, F2,
each replacing the table and the synthetic flags `_zz_ok` / `_yy_ok`, its own stderr sent to `$_VFY/.cap_err` so an expected red
never reaches the run's output). `_cap_check`, a table check (not a standing check: it runs no `_ctl_run` row), called once as
`_cap_check || ctl_ok=1`. `_cap_lines`, the summary's per-group line, decided from the table and the flag (a claim line or the
`NOT EXERCISED` line with the member count). Removed: `_cn`, the standing check of the old mechanism, the asks that decided the
summary lines from `.declined`, the `_ctl_need` prefixes and `.declined` itself, and `_fifo_line` / `_perm_line`. The census gained
a membership check (`_mut_cap_member`, reader 4): every `member` row must name a `_control` label this run registered, so a member
that never ran cannot be summarised as run. The records: 222 → 231 (`_mutants | wc -l`; `_MUT_RECORDS_MIN` follows): 2 removed (the
two `_ctl_need=0` records, whose subject went), 11 added (the gate's equality always-true and as a substring match; the decline
arm; the member's own group's flag; trust before capability; the undeclared-group arm; the `_cap` loop's no-arm and parse guards;
the reader's flag-name validation; the table check's misspelt group; the census membership check), and 1 re-aimed (the
`|| return $?` record, now `$_cap_lbl`); the doc coder's report first read "3 removed" from a line diff that counted the re-aim, and "10 added"; the driver recounted `_mutants`.
`_MUT_UNRECORDED_MAX` 18 → 21: the three group labels `$_fifo_lbl`, `$_perm_lbl` and `$_pfl_lbl` arrive with the table and
their records are CR-B2b's, so the ratchet is loosened by their number for two slices; **CR-B2b brings it back to 18** (carried,
below). The doc rows (`fea3a0c6`): the design memo's `_cn` row becomes the CAP, CAPT, FIFO, PERM and PFL rows, and a paragraph
states the marker below; corpus §6.1 five rows and new totals; residuals R11 (the plan's BS-2: a member declared that does not
need the capability) and the range "R1–R11"; six rows added to `idents.tsv` in §E (`cap`, `cap_fn`, `cap_err`, `cr_vars`,
`capflags`, `endmarks`, R3-all-3).

**The verdict marker (a decision the plan does not have).** The code unit's GB2-14 record, the reader's flag-name validation made
vacuous, came out WRONG REASON on bash 5.3: `${!x-}` on a non-name aborts `_cap`'s subshell before its labelled line, and the
then-branch `ctl_ok=1` reds the run unlabelled, so "removed → something reds (named)" fails. The class is every standing check of
shape `elif ! ( … ); then ctl_ok=1`, which is `_wm`, `_ev` and `_cap`, so the driver gave all three one marker (one issue, one
way): the subshell creates `$_VFY/.<x>_end` when it reaches its verdict, and the then-branch prints `CONTROL FAILED (<label>): the
check ended before its verdict` when the file is absent. It was decided as an implementation detail the slice's instrument found
and went into the claim rows, not back to plan-review. Its measurements: an abort injected first in each subshell
(`: "${_no_such_name_x}"`) → KILLED AS NAMED, one label line, rc 1, both shells (12 of 12 trials in the code unit); the marker's
`then` line removed → WRONG REASON, no label line.

**Measured.** Ship PASSED, rc 0, 0 `!!` lines, ×4 (BSD and GNU-like `grep` × bash 5.3 and 3.2) at `c1ca2a4d`, `fea3a0c6`,
`55f97949` and `4a2c2ca6`; `scripts/trip-wires.sh` rc 0 at each code commit; the completion grep over the seven parts and the wire
for `_ctl_need` and the other removed names 0; `prefix-grep.sh` over all parts and the wire 0 (with its negative control, one
`X=1 _control a b` line, 1 hit). The code unit re-trialled the 26 records anchored in `controls.sh` or killed by `_wm` / `_ev` /
`_cap` labels, 52 trials, all KILLED AS NAMED. Sizes at the head (`wc -l`): `ctlrun.sh` 439 (379 at the frame), `controls.sh` 712
(700), `mutations.sh` 809 (788), `records.sh` 441 (413), `harness.sh` 886 unchanged, the design memo 755 (≤ 760), `cr-sweep` 926.
The claim tables were measured at `fea3a0c6` in two clones, one wire process each, both shells, every log under the driver's
`crb2a-logs/`. **GB2-1…GB2-5** (equality, substring, decline arm, member's own group, trust before capability): each record
KILLED AS NAMED, rc 1, by the row that names it (rows N, N, a decline row, Y, the untrusted row); the survive clauses as in
the next section. **GB2-6** the undeclared-group arm: killed by row U. **GB2-7** the census membership: a member row renamed →
`CONTROL FAILED (CEN): declared capability member "…" is no _control this run made`, rc 1; with the membership pass removed,
rc 0 on a capable machine. **GB2-8** a misspelt member group → `CAPT` red naming the group; **GB2-9** `|| return $?` → row X
`returned 2, not 1`; **GB2-10** no-arm → `no arm`, parse → `did not parse`; **GB2-11** a plain ship holds 0 occurrences of
`CONTROL FAILED ($_cap_lbl)`, and with `2> "$_VFY/.cap_err"` made `2>&2` the five expected reds reach the output; **GB2-13**
a `fact` row cut to 2 fields → `CAPT` red `a fact row holds 2 field(s), not 3`, rc 1; with `|| ctl_ok=1` → `|| :` the same red
line prints and rc is 0; **GB2-14** below; **GB2-15** (a) the table renamed in the controls file only → `missing: $_CAP_TABLE`,
rc 2, (b) plus the guard disabled → `_CAP_TABLE: unbound variable` at `ctlrun.sh` :67, rc 1, (c) a flag name misspelt →
`missing: $_fifo_okk`, rc 2; **GB2-16** (a) a group label's definition renamed → `_lbd` names it, rc 2, (b) the table block
moved before `_lbd` → `controls.sh` :154 `_fifo_lbl: unbound variable`, rc 1; **GB2-17** the nested pass visits 11 of 11 rows
with every field (`_cr_row`, `_cr_nf`, `_cr_bad`, every `_cr_f[i]`) identical, and with the cursor and fields made global it
visits 1, both shells; and the registry check: `_cap`'s call registers once per row, all ten at `controls.sh:386`, `direct`, one
site, both shells. **R3-B2a-1** the first reader after the guard is `_cap_check` (`controls.sh` :251 → :230 → `ctlrun.sh` :67
`local _cr_rest="$_CAP_TABLE$_cr_nl"`); **-2** is GB2-13's plant, a `fact` row, as measured; **-3** the guard asserts every
consumed name except `$_VFY`, which the controls file makes after `ctlrun.sh` is sourced (:205 against :180), the header says so;
**-4** every name `ctlrun.sh` defines (28) is in WHAT IT DEFINES, and `mutations.sh`'s lacks `_mc_nl` (fixed, below); **-5** the
three `_fw_trusted` openings are byte-identical; **-6** below; **-7** is GB2-17. The `mutations.sh` function guard sourced in a
shell that defines none of the three functions: `!! This file is the MUTATION SET … missing: _cap_rows() _cap_group_of()
_cap_flag_of()`, rc 2, both shells. The summary on this machine states both groups as claims (0 `NOT EXERCISED`); with the
fifo probe forced to 0 in a copy, rc 0 and `NOT EXERCISED … 2 control(s) of the fifo group`, with the perm probe forced, `5
control(s) of the perm group`, each leaving the other line a claim. The battery (`BASE_REV=0848a850`): 90 comparable cases, 4 rc differences, each explained: `R9trustaboveunset` red → green (its subject, trust above
`unset _ctl_need`, no longer exists); `S10nokind` green → red (the membership check now also pins the registry's kind
detection, a second pin); `R8nowd` now labelled ×3 (the marker); `R3noask_s2`'s `CF` count 103 → 110 (more labelled
reds). 37 cases' edit targets (`_ctl_need`, `.declined`, `_cn`) no longer apply; `runners/cases/retired/README.txt` retires 38:
those 37 and `R9trustaboveunset`, which applies but whose subject no longer exists (its green above is right). Generic properties carried over: `S3fm1norestore` → `S3capnorestore`
(`_cap`'s trust restore, red under `$_cap_lbl`, "did not name zz" on F1 and F2, both shells); the arm cases (`S3fm1norow_s1`,
`R9cn*`) and `needunset` → the GB2 survive clauses and GB2-15(c).

**Where the plan memo says otherwise** (the plan is not edited; the measured facts are these). §6 GB2-3: "that row removed →
SURVIVES" is KILLED AS NAMED by the other decline row (`Y 1 0`); it survives with both decline rows removed. GB2-6: with row U
removed the record is killed by F1; U, F1 and F2 all removed survives. GB2-8: the clause removed alone is WRONG REASON as the
plan says, but "clause and arm both removed → SURVIVES" is also WRONG REASON (row U under `$_cap_lbl`, whether the arm is `return 0`
or the block deleted); it survives only with rows U, F1 and F2 removed too. The plan's survive clauses for GB2-3, -6 and -8 name too few rows; the design memo
states no GB2 survive clause, so none needed correcting there. GB2-10: the first shape of the parse record (`M 0 1 no - 1 -` → `nop`) did not
pin the parse guard: with the guard's arm removed the row still ran and was killed as `returned 2, not 1`, so the record's survive clause did not
survive; re-aimed in the review fix to a row only the guard catches (`M 0 1 yes - 2 -` → `… 2 -x`), whose
survive clause (the guard's arm made `*) ;;`) measures SURVIVED, both shells. GB2-12: with `_cap_check || ctl_ok=1` deleted and
GB2-8's record applied, the run is WRONG REASON (the gate's F5 arm reds the member under its own label), not a survival, so the
call is not pinned by GB2-8's record in this slice; the records the plan names for it, GB2b-1, -2 and -4, are CR-B2b's and do
not exist here. GB2-14: the plan's "b53 aborts, b32 differs" held, and the record was WRONG REASON on b53; the plan's 5.3
arm is now killed by the marker (`the check ended before its verdict`) and the 3.2 arm by row F1 (`returned 0, not 1`); the
plan's survive clause (rows F1 and F2 removed) is a plain green on both, since the abort needs a row to abort on. GB2-15(b),
GB2-16(b) and R3-B2a-1's unguarded arm: the plan says unlabelled on both shells, and it is on 5.3 only; on 3.2 the wire's exit
trap adds `CONTROL FAILED (the run ends only through a verdict): the run ended before reaching its verdict` after the `set -u`
line. R3-B2a-5 cites the `_wm` and `_ev` openings at `controls.sh` :223 and :264; they are :280 and :326 at the code commit
(`_cap`'s :364). R3-B2a-6: a callback that returns non-0 is swallowed silently wherever the reader runs under `||` or `if !` (the
gate, `_cap_check`, the census: the wire stays PASSED, rc 0) and aborts the wire unlabelled, rc 1, where errexit is live (the
source-time guard's final-operand call; the bare `_cap_lines`): no red names it; the contract (every callback returns 0, the
pass is stopped by `_cr_stop=1`) is load-bearing and enforced by nothing, and every shipped callback ends in a status-0
command by inspection, recorded, no change. §7's "controls.sh (shrinks ≥ 40)" is a projection that did not hold: 700 at the
frame, 712 at the head (695 at the first code pass, before the marker ×3 and the review fixes); ctlrun's "≤ +60 net" holds at
its limit (379 → 439), and records.sh's "−2, +~10" is −2, +11 and 1 re-aimed. §7 also listed no claim row for three of
`_cap_check`'s five clauses (a group declared twice, a flag that is not 0 or 1, a member label listed twice; only the parse and
the member's-group clauses were pinned, by GB2-13 and GB2-8), though §2.2's reader 1 puts the fail-safe direction on the flag
clause: the review found them unpinned (below).

**The review** (a fresh reviewer on the snapshot `fea3a0c6`): 0 CRITICAL, 2 IMPORTANT, 5 MINOR, all introduced by the slice, one
fix round (the early stop did not fire). Checked and confirmed: the 12 record anchors (12 of 12), 4 of 12 record trials KILLED
AS NAMED, the reader's field split / cursor / lookup, the gate order, the ten `_cap` rows, the markers, and 65 of the 261 new
sweep hits sampled with none misclassified. **I-1** `_cap_check`'s clauses for a group declared twice, a flag not 0 or 1, and a
member label listed twice were pinned by nothing; with the flag clause defanged and `_fifo_ok=2` the run was green and the summary
said `NOT EXERCISED` while the gate had run both fifo controls. Fix (`55f97949`): one record per clause, needle `$_capt_lbl`, each
killed by its clause alone (14 of 14 trials KILLED AS NAMED, one `!!` line each) and each with its survive arm measured (8 of 8
SURVIVED, with the re-aimed parse record), and `_cap_lines` decides claim against `NOT EXERCISED` by the gate's own predicate
(`!= 0`), so the summary and the gate cannot disagree. **I-2** the design memo still said R1–R10 at three sites (:31, :56, :649).
The sweep method had a gap: the doc coder classified only hits new by `(identifier, file, text)`, and an unchanged line the
slice's own edit made false was never read. Fix (`4a2c2ca6`): R1–R11 at the three; and every hit of every identifier group
whose referent the slice changed was re-classified over the whole head (the full re-classification: below), which found three
more sites made false (the CAP and CAPT rows' pin lists, the corpus totals), fixed. **m-1** the three markers had no shipped pin:
records for the three abort injections (6 of 6 marker-line-removed arms WRONG REASON). **m-2** "two rows" in the `_ctl_run` ask's
comment and the design memo → six of the ten rows (`M 0 1 yes`, `Y 1 0`, U, `M 0 1 no`, F1, F2; row X stays green), re-measured
both shells. **m-3** corpus and records comments gave inconsistent first reds: rewritten per record (the no-flag arm, row U first,
F1 and F2 too; `|| return $?`, row X; the validation record, the marker on 5.3 and F1 on 3.2); the driver's brief
called the `|| return $?` record 377, but at `fea3a0c6`'s numbering it is 381 (row X) and 377 is the no-flag arm (row U first),
as the review's m-3 had it; the doc coder followed the review, and the brief was in error. **m-4** `cr-sweep` §1's "`_cr_*` out-globals"
→ they are `_cap_rows`'s function-locals, the out-globals are `_cap_g`, `_cap_f`, `_cap_v`. **m-5** `_cap_rows` visited nothing for
an unknown kind (fail-open): now exit 2, `!! _cap_rows: "<kind>" is no row kind to visit; decided nothing.` (the driver read the
first version, which put a non-label in the label slot, and had it moved out), claim row: the census's `_cap_rows member` →
`membr` gives rc 2 and that line, both shells. `_mc_nl` is in `mutations.sh`'s WHAT IT DEFINES (R3-B2a-4's finding). After
the fixes: records 231 → 237 (+3 clause records, +3 marker records; `_MUT_RECORDS_MIN` 237; `_MUT_UNRECORDED_MAX` stays 21, the
census is exact), 22 of 22 re-trials of the table-anchored records KILLED AS NAMED, ship ×4 PASSED, `ledger-check.py B2a` ok
(576 rows).

**Sweep re-run, the new sites** (`sweep.sh`, 26 files, §E's `idents.tsv` with the six rows this slice adds, run on bash 5.3 and
3.2 with identical output). At `fea3a0c6`: 812 lines against 737 at the base; 261 hits new by `(identifier, file, text)`, all
classified by rule (the new mechanism's code and comments, this record, ratchets, the corpus and design rows, `stays`), 0
unclassified; the review sampled 65 and found none wrong. At `4a2c2ca6`: 828 lines (821 before the doc fixes, the 7 added being
those fixes' own rewritten sites). The full re-classification the review asked for, of the groups whose referent the slice
changed: `resid_range` 10 hits (4 history, 2 unrelated "plan-review R1–R3", 4 true or fixed, of which the design memo's three
were false and are fixed); `ctl_need` 23, `declined` 2, `cn` 17, `cndir` 2, `fifo_line` 3, `perm_line` 5, the removed
mechanism: all history, the #519 memo (residual R10) or this slice's removal rows; `standing` 58: the population is `_wm`, `_ev`,
`_cap`, still three, every non-history hit read and true; `notexh` 12 true or history (the limit line is still produced where it was); the ratchet figures ("222", "231", "136") false at the head only in the corpus totals and a
`mutations.sh` comment, fixed; `fifo_ok` / `perm_ok` 18 / 12, true; `cen_lbl` 17, true except that the census row of the design
memo does not list the membership record (carried below, stated in the CAPT row's census sentence meanwhile); the remaining groups'
referents are unchanged or the slice's own names. A concept sweep (`declin|capability flag|summary line|last ask|ask`) over the
parts and the non-history docs finds no stale description of the removed mechanism. **The wrapped-identifier backstop**
(`perl -0777`, `[\s#]+`): the same five matches at the base and at the head (harness :750, mutations :205 and :761, landing :74,
reviews :624), none introduced. **R3-all-1, the two-owner rows, at the head:** controls :25-26 and records :367 (B1/B2a): both
owners' parts gone (`_fifo_line`, `_perm_line`, `_ctl_need` and `_gopt_ok` 0 in both files); mutations :21 (B2a/C1): the consumes list
(:20-31) holds `_cap_rows` (B2a's edit) and not `_CONTROLS` (the check C-REPORT used for C1's part); mutations :248 (every slice/C2) and corpus :194 (B2b/B2c):
still there. This section's own lines name the slice's identifiers as a record of it (98 hits, all in this section, `hist`: a
dated record of the slice); the sweep reads 926 lines with this section's text in the file set (828 without it), on both shells with
identical output, and every added hit is this section's own, none removed. None is
unclassified.

**Carried to later slices.** (1) **CR-B2b**: `_MUT_UNRECORDED_MAX` back from 21 to 18, with the probe-flip records for `$_fifo_lbl`,
`$_perm_lbl` and `$_pfl_lbl`, the three bare labels the loosened ratchet counts (a required claim row); GB2-12's named pins GB2b-1, -2, -4 (the table check's
call is pinned by those, not by GB2-8's record); the four labels `$_capt_lbl`, `$_fifo_lbl`, `$_perm_lbl`, `$_pfl_lbl` state
the declaration's enforcement (absence red) before B2b builds it, true from B2b on. (2) **CR-B2b**: the P-x excuse in
`_mut_run` (`mutations.sh` :742, `case "$_mlv" in "_p${_mid#P-}"*`) keys on the `_p<x>*_lbl` naming, which `$_perm_lbl` (`_pe…`)
matches for P-e; harmless while P-e has no limit row, to be closed where B2b builds the excuse on the declaration (a required
claim row). (3) **CR-C1**: the design memo's census row (:620, a ledger `stays` row for B2a, C1's row in §7) does not yet list
the membership record among its pins; editing it here would fail `ledger-check.py B2a`, so the pin is stated in the CAPT row's
census sentence and C1 rewrites that row.

### S.4 CR-B2b — an undeclared absence is red; the declaration is checked, not trusted

The third slice of the B/C plan (`…-cr-plan.md` §2.3 and §7). Base `de05191c` (CR-B2a's head), branch `crb2b`. Commits: the code
(`567b22e8`), its doc rows (`ea5e5a4c`), the review's fixes (`17c6cd13` for the part comments, `cd95ba25` for the memo and landing),
and the re-check's (`1f5a8f4d` for the part comments, `d9c20d4c` for the memo and landing; `d75be6b6` added this entry; the
branch was fast-forwarded onto `k2-wire-fgit-hermetic`, so these hashes are the branch's). The slice was cut into three coder units by the driver (code; the
claim table; the doc rows), as CR-B2a's was. Records are named by their property, not by line number: the line numbers of
`records.sh` shift between these commits.

**Changed (`567b22e8`).** `_cap_check` (controls file) gained three clauses, all under `$_capt_lbl`: every word of
`WEBREF_WIRE_LACKS` must be a group or a limit ID of the table (a word that is neither is red, naming it), a group whose flag is 0
and which the caller did not declare is red under the group's own label with the declaration hint, and every ID the window
reports in `$_fw_limits` must have a `limit` or `fact` row (a limit ID the caller did not declare is red under its row's label).
`ctlrun.sh` is unchanged (439, at its budget). Three probe-flip records (the FIFO and permission probes assigned 0, and P-f forced
to its limit branch) pin the new clauses; the records are 237 → 240, `_MUT_RECORDS_MIN` 240, and `_MUT_UNRECORDED_MAX` is back from
21 to 18 (`mutations.sh` :246, :261). The doc rows (`ea5e5a4c`): the design memo's P-b fact and P-f declared rows, the FIFO, PERM,
PFL and CAPT rows current, the BS-7 cost; residuals R12 and the R4 member, the range R1–R12 at four sites; the landing's route (b)
for Q1, the slot amendment for BS-3 and X9; corpus :194 (B2b's part), §6.1's three rows and the totals (145 / 240 / 18); three
rows in `idents.tsv`.

**Measured: the claim table** (B-CLAIMS, code commit, in a clone, the real `_mut_run` and `_mut_trial` with a filter to one record
and the generated set stubbed, both shells, at most two wire processes at once; logs under the driver's `crb2b-logs/`). **Rows
1–6, every claim as expected on both shells (nothing differed):** (1) `LACKS=fifoo` red under `$_capt_lbl` naming the word,
rc 1; a TAB or a newline separates words (green), a CR does not (red, one word); (2) a limit ID injected into the harness copy that
no row names red naming it, and green with that clause made `:`; (3) the probe-flip record under `LACKS=fifo` on a no-FIFO
simulation SURVIVES unexcused (`_mut_exc` 0: the table-flag excuse is CR-B2c's), and with `-u WEBREF_WIRE_LACKS` on `_mut_trial`'s
env line it is KILLED AS NAMED, so the difference between the two is the inheritance; (4) the four records whose needle is
`$_pg_lbl` and that plant a FIFO end at "decided nothing", WRONG REASON, mutant rc 2, the opt-in red, 8 of 8; (5) the three new
records KILLED AS NAMED, 6 of 6, and the permission-flip record on a root simulation SURVIVES unexcused; (6) P-f undeclared on a
`env -0`-refusing simulation red under `$_pfl_lbl` with the hint, declared green with the `NOT EXERCISED` line. **Fix round, N of
M:** R3-B2b-3's P4 cites row 4; the P7 FIFO half (both groups declared) 8 of 8 WRONG REASON; GB2-12's opt-in, the table check's call
deleted with GB2b-1 applied, SURVIVED 2 of 2 and with the call present KILLED AS NAMED 2 of 2 (the always-on run with the call
deleted stays green: the call is pinned only by the opt-in). 6 of 6 fix-round rows done; the P6 mode half is not measured (below).

**Measured: the battery** (`BASE_REV=de05191c`): 90 comparable cases, 2 rc differences, both shells, explained: `incap` and
`M5incap` went red under `$_fifo_lbl` (the intended undeclared-absence red). Both simulated no FIFO by editing the probe line,
which is a record's anchor (the base was red there too, from the stale anchor), so the cases broke the rule that a case must
not edit an anchored line; they are retired and retargeted by behaviour with a `nofifo` PATH shim: `incapB2b` (declared: green, 2
`NOT EXERCISED`), `incapnodecl` (undeclared: red under `$_fifo_lbl`) and `M5incapB2b` (the 2 → 0 mapping removed: rc 1, no
`CONTROL FAILED`), all as expected on both shells.

**The reviews.** First review (a fresh reviewer on the snapshot `ea5e5a4c`): 0 CRITICAL, 3 IMPORTANT, 5 MINOR, the code correct,
every finding wording. Origins: I-1 (the FIFO "Cost:" row named records by line numbers that had drifted by two), I-2
(`mutations.sh` said a probe-flip mutant reds on a declaring machine; it SURVIVES there) and M-1 (X9's "no verdict depends on it"
against `_wm_gone`'s `sleep 0.05`) were false statements this slice wrote; I-3 (R3-B2b-2's cost sentence missing beside
`_cap_check`) and M-3 (R3-B2b-3's P6 and P7, GB2-12) were carried requirements unmet; M-2 was two old comments whose referent the
slice changed; M-4 was the planned §S entry (this one); M-5 was the sweep method (hit-line-only classification missed wrapped
sentences). Re-check (a focused reviewer on `cd95ba25`, no wire runs): 0 / 0 / 3, I-1, I-2, I-3, M-1 and M-3 closed, M-2 open (the
fix wrote that the decline depends on the declaration; the gate declines on the flag alone, `ctlrun.sh` :202, and never reads
`LACKS`), and 3 MINOR in the fix's own wording (N-1 a sentence with no verb in the FIFO "Cost:", N-2 X9's sleep census omitting
the hang fixture's `sleep 5`, N-3 the P-f row's "one channel" unconditional). All four and the queued X9 clause (a `sleep` that
refuses fractions leaves a process not yet reaped red and a reaped one green: a false red at worst) are this round's fix. **The M-5
method fix, measured:** all 125 new hits were re-classified reading the whole sentence: 72 true, 50 code, 1 imprecise (N-3, fixed
here), 0 false, and 2 settled as CR-B2c's (`records.sh`'s comment that the probe-flip records are excused by the table's own flag,
which is false until B2c builds that excuse). 125 of 125, remaining 0.

**Ledger rows 85 and 171 fail, an instrument limit, with a manual backstop.** `ledger-check.py B2b` reports 574 of 576 rows ok;
the two failures are row 85 (`controls.sh` :483, `[ -z "$_fw_limits" ] || _limit_lines=…`) and row 171 (`harness.sh` :348, the
line `# outcome is a MACHINE LIMITATION instead (…NOT EXERCISED on this machine`,`). The checker judges the hit line of a
multi-line statement whose rewrite landed elsewhere in its extent; both statements are true at the head. Row 171: `harness.sh`
:346-349 says P-f's outcome is green "only where its caller declared it, WEBREF_WIRE_LACKS=P-f", the rewrite being on :349. Row 85:
`controls.sh` :472-481 states the slot rule and P-f's declared green, and the row's own disposition says the hit line stays.
Rewriting true lines to satisfy the checker would bless the instrument, so neither is edited.

**Not measured.** (1) The `_wm_gone` poll path: reaching it needs a process that outlives its kill, which no PATH shim arranges
without editing an anchored line; measured instead, with an integer-only `sleep` shim, that the run is green and `0.05` is never
called. (2) P6's mode half (the records that plant a `chmod`): real root is unreachable on this machine, the root simulation
shims the effect only.

**Measured: completion.** Ship PASSED, rc 0, 0 `!!` lines, ×4 (BSD and GNU-like `grep` × bash 5.3 and 3.2) at `567b22e8`
and `cd95ba25`, and at this round's commits (log paths in the driver's `crb2b-logs/`, `crb2b-fix/logs/` and `crb2b-fix2/`);
`scripts/trip-wires.sh` rc 0. Sizes at the head (`wc -l`): `ctlrun.sh` 439, `controls.sh` 763 (712 at CR-B2a's head),
`mutations.sh` 813, `records.sh` 448, `harness.sh` 886 unchanged, the design memo 755 (≤ 760), `cr-sweep` 930.

**Carried to later slices.** (1) **CR-B2c**: the table-flag excuse in `_mut_run`: `records.sh`'s comment beside the probe-flip
records already says each is excused "by the table's own flag or the P-x half", and B2c makes that true; until then
the FIFO and permission probe-flip records SURVIVE unexcused under a declaration (row 3 and row 5). (2) **CR-B2c**: R3-B2b-1 and
GB2b-6 re-measured in the excused form (`_mut_exc` 1, "survived, and is not exercisable"); B2b measured the inheritance by its two
arms. (3) **CR-B2c**: the `_p<x>*_lbl` excuse naming in `_mut_run` matches `$_perm_lbl` (`_pe…`) for P-e, to be closed where the
excuse reads the declaration (a required claim row). (4) **CR-B2c**: the P6 permission flip's excuse (it SURVIVED unexcused, row
5). (5) **CR-C1**: the design memo's census row (a ledger `stays` row for B2a, C1's row in §7) does not yet list the membership
record among its pins.

### S.5 CR-B2c — the opt-in excuse reads the capability table

The fourth slice of the B/C plan (`…-cr-plan.md` §2.3 and §7). Base `a567cc13`, branch `crb2c`. Commits: the code (`4df27614`), its
part comments (`e1e764f7`), the doc rows (`61d442a9`), the review's fixes (`32c89a05` for the part comments, `567b22fe` for the
corpus and the design memo), the re-check's (`17ca651d`, one wording in `mutations.sh`),
and the commit that adds this entry; the branch was fast-forwarded onto `k2-wire-fgit-hermetic`, so these hashes are the branch's.
The slice was cut into three coder units by the driver (code; the claim table; the doc rows and sweep), as CR-B2b's was. Records
are named by their property, not by line number.

**Changed (`4df27614`).** `_mut_trial` returns 3 for a wrong reason only when the mutant exited 1 (its wrong-reason arm decides;
`_mut_wrong_reason` only prints the diagnostic; a mutant that exited 2, as a harness break also does, still returns 2 and is never
excused). `_mut_run`'s excuse has two halves. The capability half: a
record whose needle is a member of a group whose flag is exactly 0, or that group's own label (`_mut_skip_glabel`), is excused
when it survives or was killed for another reason, and the report names the group. The P-x half: a record whose needle is the
postcondition label of an ID in `$_fw_limits`, or that ID's limitation-test label, exactly (`_p<x>_lbl` or `_p<x>l_lbl`), and only a survival is excused; the exact naming
was the driver's fail-safe decision for the item carried from §S.4 (the `_p<x>*` glob also matched `$_perm_lbl` for P-e, which
would excuse wrongly). An unexcused 3 is red. `records.sh` comments name which excuse covers which probe flip. The doc rows
(`e1e764f7`, `61d442a9`): the part comments, the corpus excuse text and §6.1's flip rows, the design memo's FIFO, PERM and PFL rows,
the `mut_skip` identifier row in the sweep.

**Measured: the claim table** (B-CLAIMS, code commit, in a clone, the real `_mut_run` and `_mut_trial` with a filter to one record
and the generated set stubbed, both shells, at most two wire processes at once; logs under the driver's `crb2c-logs/`), 66 wire
runs plus the row 7 script. **Rows 1–8, every claim as expected on both shells (nothing differed):** (1) P6, root simulation with
`LACKS=perm`: the err-seal, delete, umask, self-test-root and not-absent records and the permission flip are excused naming the
perm group (16 of 16); (2) P4, no-FIFO simulation with `LACKS=fifo`: the FIFO flip excused naming `fifo`, KILLED AS NAMED with
`-u WEBREF_WIRE_LACKS` on `_mut_trial`'s env, and the four `$_pg_lbl` FIFO plants WRONG REASON, exit 2, not excused (12 of 12);
(3) P7, both groups declared: as P4 and P6 (8 of 8); (4) P1, no declaration: the same records and the CAPT-label record KILLED AS
NAMED (8 of 8); (5) a planted exit-2 record red and not excused, and with the exit-1 test removed excused (6 of 6; a planted
anchor that matches nothing is red earlier, by the always-on anchor check); (6) a planted exit-1 record with a non-member needle
red with one diagnostic, green with the unexcused-3 branch removed (4 of 4); (7) P-e naming: the shipped `case` over the
shipped label derivation matches `_pe_lbl` alone for P-e, and `_pf_lbl`, `_pfl_lbl` for P-f (2 of 2, plus the reading of why no
P-e line reaches `_mut_run`); (8) the excuse reads the member's own group flag (8 of 8), the P-f flip is excused by the P-x half
only where the machine really lacks it (4 of 4), and the return-value range of `_mut_trial` read. **Fix round, 22 of 22:** the
shipped derivation and `case` over all 16 `_p*_lbl` names (16 of 16 on both shells), the two standing-check records under the
capability absent (needles `$_cap_lbl`, the standing check `_cap`, and the W label `$_wm_lbl`, killed as named, 2 of 2 each) and a stale `LACKS=perm` on a capable
machine leaving the permission flip surviving unexcused and red (2 of 2).

**Measured: the battery** (`BASE_REV=a567cc13`): 91 cases, both shells, rc, PASSED and CONTROL FAILED differences 0, 0 EDIT FAILED.

**The reviews.** First review (a fresh reviewer on `61d442a9`): 0 CRITICAL, 3 IMPORTANT, 4 MINOR, the code correct (a unit test of
the shipped excuse block, 7 of 7 cases on both shells). Origins: I-1 (the corpus said a declaring machine "cannot read" a mode-000
file, backwards: it is root that reads it), I-2 ("the gate reds", where the red is `_cap_check`'s) and M-1 (the err-seal clause in
`records.sh` stated unconditionally) were false or imprecise statements this slice wrote; I-3 (`ctlrun.sh`'s list of outside
`_cap_rows` readers missing `_mut_run`, plan §2.2) was a requirement owed; M-2 (the PERM row needed the probe's answer as well),
M-3 (`_p<x>l_lbl` is the limitation-test label; the P-x source includes a fact row) were wording this slice changed; M-4 was the claim
rows (row 7 hand-copied, the capability-absent label records unmeasured as a standing check). Re-check (a focused reviewer on
`567b22fe`, no wire runs): 7 of 7 closed, 0 / 0 / 1, the MINOR being the P-x clause of `mutations.sh` ("a line, a limit or a fact
row" set a line in apposition to a table row), fixed here (`17ca651d`); 14 fix-added statements, 13 true, 1 imprecise, 0 false.
**The sweep:** 62 new hits on 35 lines, each read as a whole sentence: 3 imprecise (fixed in the doc-rows commit), 0 false.

**Ledger: 12 of 576 rows fail, an instrument limit, with a manual backstop.** `ledger-check.py B2c` fails the same 12 rows before
and after the fix round: the two settled CR-B2b rows (`harness.sh` and `controls.sh`, §S.4) and 10 retained hit lines in
`mutations.sh` of rows whose extent `4df27614` rewrote (the checker judges the hit line, not the extent). The re-check confirmed
that each extent changed and each statement is true at the head. Rewriting true lines to satisfy the checker would bless the
instrument, so none is edited.

**Not measured.** (1) The apply-time MATCHED NOTHING path (the always-on anchor check reds first, so it is unreachable without
removing that check). (2) Real root and a real no-FIFO filesystem: the simulations shim the effect only.

**Measured: completion.** Ship PASSED, rc 0, 0 `!!` lines, ×4 (BSD and GNU-like `grep` × bash 5.3 and 3.2) at `4df27614`, at the
fix round and at this round's commits; `scripts/trip-wires.sh` rc 0. Sizes at the head (`wc -l`): `ctlrun.sh` 439 unchanged,
`controls.sh` 763, `mutations.sh` 846, `records.sh` 449, the design memo 755, `cr-sweep` 932.

**Carried from §S.4, discharged here.** All four CR-B2c items: the table-flag excuse in `_mut_run`, the excused form
(`_mut_exc` 1) for R3-B2b-1 and GB2b-6, the P-e naming, and the P6 permission flip's excuse. **Carried onward.** (1) **CR-C1**: the
design memo's census row (a ledger `stays` row for B2a, C1's row in §7) does not yet list the membership record among its pins;
still open. The reviews marked nothing else for a later slice.

### S.6 CR-C1 — the label census is bijected with the text

The fifth slice of the B/C plan (`…-cr-plan.md` §3.2 and §7). Base `c6e69356`, branch `crc1`. Commits: the code (`2ac70030`), its
part comment (`63893e7b`), the doc rows (`505ad593`), the first review's fixes (`82776f96` for the wire, `2b9489b6` for the docs),
the driver's restoration of a guard the fix deleted (`666eed96`), the re-check's (`c2ce1ebf` for the wire, `bbe5f30f` for the docs),
the instrument change's (`c46fc6c1` for the wire, `4070c49d` for the docs), and the commit that adds this entry; the branch was
fast-forwarded onto `k2-wire-fgit-hermetic`, so these hashes are the branch's. The slice was cut into three coder units by the
driver (the code; the claim table; the doc rows and sweep), as CR-B2c's was. Records are named by their property, not by line number.

**Changed (`2ac70030`).** Rules 1 to 3 of `…-cr-plan.md` §3.2. Rule 1: the census's used names are the `_*_lbl` names the wire or a
part EXPANDS (`$name`, or `${name` and a non-identifier character), outside the record table; a definition and an indirect `${!x}`
are no use, and `.vlabels` holds their values. Rule 2: the call lines of the controls file (a non-comment line naming a run
primitive as a word) are exactly the registered sites, kinds included, in both directions. Rule 3: one `awk -F'\t'` pass reads the
registry and the controls file together, and a row that does not hold exactly three TAB fields is red. The registry's sites are the
same on every machine, and its rows and labels per site were measured equal (100 rows, 89 sites, byte-identical across both shells
and the P1, P4 and P6 profiles). Eight records pin the new checks (GC-1 to GC-5, and the re-aim of the old record 360), so the
records and the ratchets went 240 → 248 with `_MUT_UNRECORDED_MAX` unchanged at 18 (measured 18 on P1, P4 and P6); the fix
round's record made it 249 (`82776f96`) and the third round's 250 (`c46fc6c1`). `ctlrun.sh` shrank from 439 to 436. `mutations.sh` is
890 at `2ac70030`, 894, 897 and 900 at the head: **900 is the plan's stop line** (the plan stops when the measured length passes
900), reached and not passed, carried to CR-C2 as a measured fact below. The doc rows (`63893e7b`, `505ad593`): the part comment
(`_lbd` reads the record table too, every `_*_lbl` token), the design memo's census row (the pins of rule 1, the bijection, the
kinds, the three-field parse and the membership record, which discharges §S.4's carried item and §S.5's), corpus §6.1's re-aim and
four add rows, residual R13 and the range R1–R13, landing X9's census clause, and the `cen_vars` row of the sweep's identifiers.

**Measured: the claim table** (B-CLAIMS, code commit, in clones, the real `_mut_run` and `_mut_trial` with a filter to one record
and the generated set stubbed, both shells, at most two wire processes at once; logs under the driver's `crc1-logs/`), rows 1 to 10,
about 46 pairs, about 92 wire runs. As expected, N of M: GC-1 2 of 2, GC-2's K3 record 2 of 2, GC-3 6 of 6, GC-4 4 of 4 (it SURVIVES
with the bijection removed, and at the base with the record planted last), GC-5 2 of 2, A's kills re-run 4 of 4; the label-defined
guard (`_CONTROLS` removed from the word list: unlabelled rc 1, a shell diagnostic) 2 of 2 and its `missing:` red 2 of 2; the K1
harness case 4 of 4, `if false` around three calls 6 of 6, the unregistered branch call red on P1 and P4 4 of 4; the registry
dumps and the profiles (row 5, 6 of 6); `_rh` 2 of 2; the rule-2 case with the call line itself wrapped 4 of 4; the trailing-comment scan (the
population: 0); the 11 older census records killed as named, 22 of 22. **Four deviations, read by the driver from DECISIONS:** (1)
the relcwd record with the bijection removed is killed for the WRONG REASON, not SURVIVED (relcwd's own else-branch reds when the
call is wrapped); accepted, and the docs state the measured form: the bijection is what makes the kill AS NAMED. (2) GC-7's arm
deleting only the red line of `_cap_chk_absent` is also WRONG REASON (`_cc_bad=1` remains); the SURVIVED form needs the whole body
replaced by `:`; accepted. (3) the trust simulation combined with the opt-in is red before any mutant, so it is not measurable;
its parts were measured separately; accepted. (4) the second GC-8 arm (the word removed from the guard's list) aborts unlabelled
with an unbound variable, rc 1, which is exactly the plan's "aborts unlabelled" and was misread by B as a difference: as expected.
The brief's "about 12" old census records is 11.

**The reviews and the instrument change.** First review (a fresh reviewer on `505ad593`): 0 CRITICAL, 1 IMPORTANT, 6 MINOR, the
census code correct on every path exercised. Origins: I-1 (rule 1 counted a name in a trailing comment as a use, the fail-unsafe
direction, while three sites said a comment is no use) was a declared-blind-spot class this slice left open; M-1 (the K3 record's
kill path stated for P1 only), M-4 (row 9's universal against its ratchet's 18) and M-5 and M-6 (R13's cite, "the same on every
machine" derived from the bijection) were statements this slice wrote; M-2 (the dead `_lbd_files` guard) and M-3 (the one-site
paragraph) were referents the slice changed. The open items: O-1 (mawk) is X9's route (b), below; O-2 and O-3 were measured in the
fix round; O-4 (the docs state relcwd's measured form) is this entry and corpus §6.1. **Fix round (`82776f96`, `2b9489b6`):** rule 1
cuts a comment from a `#` at the line start or after whitespace; the 33 lines it cuts that hold `_lbl` are all whole-line comments;
a new record (idx 220) is killed as named and SURVIVES with the cut removed, both shells; the K3 record is killed on P4 by the
ratchet there (0 not killed as named on P4 and on P1, 4 of 4). O-2: deleting the relcwd block reds under `$_cen_lbl` (2 of 2); deleting `_cap`'s call line
reds under `_cap`'s own label first, and `$_cap_lbl` is still expanded elsewhere in the check, so it is red either way and D3's "no
expansion" holds only for the whole block. **Re-check (a focused reviewer on `666eed96`, no wire runs): 0 / 1 / 2.** N-1 was
IMPORTANT: bash begins a comment after any metacharacter, so `;# $_x_lbl` (and `&#`, `|#`, `(#`) stayed a use, a false green, and
"never a false green" was false at five sites. N-2 (three siblings still derived "machine-independent" from the bijection) and N-3
(D3 true only when the whole check block is deleted) were MINOR.

**The terminator, and the oracle.** I-1 and N-1 were two consecutive IMPORTANTs, both introduced by this slice's own work, on one
mechanism, so the instrument changed instead of another reviewer round: a bash oracle that runs each case under `set -u` with the
name unset (bash reports the use as an unbound variable if and only if it expands) on bash 5.3 and bash 3.2 and compares it with
the shipped awk cut. Convergence was set as 0 false green on both shells with every false-red class declared. Fix round 2
(`c2ce1ebf`, `bbe5f30f`) widened the cut to `(^|[[:space:];&|()<>])#`: 41 cases, 32 agree, 5 false red (three declared classes), 4
syntax errors (`<#` and `>#` are not comments in bash), 0 false green on both shells; the negative control, the old cut, gave 17
false green. **The driver then read the oracle's table and found it hand-picked:** it had no single-quoted `'$x_lbl'` and no
escaped `\$x_lbl`, neither of which expands, and the scan counted both (a false green; population 0 at head, the one single-quote
hit being an apostrophe in a comment). The table was regenerated over contexts and forms, which found a third class, a quoted
here-document's body (`<<'W'`, `<<"W"`, `<<\W`), also counted though bash expands nothing there. The negative control, the fix-round-2
head against the regenerated table, gave 24 false green. Fix round 3 (`c46fc6c1`, `4070c49d`): after the cut, a quoted
here-document's body is skipped to its terminator, each backslash-escaped character is dropped, then each single-quoted span; the
oracle, 131 cases: 114 agree, 13 false red, 4 syntax errors, 0 false green on both shells. The order is the point: the comment cut
comes FIRST, because a quote stripped before it would pair across a `#` and turn a comment's text into code, a false green. The
argument that nothing false-green remains is that every step only drops text, so the scan can lose a real expansion (its
records orphan: a false red) and can never invent one; the false-red classes are declared in rule 1's comment, BS-5a, row C and R13
(a `#` or `'` inside double quotes, a `#` after a single-quoted ` #` on the line, an unquoted here-document body's `#`, a bare
arithmetic name `$((name))`). The oracle was run outside the repo and the figures are stated in the memo, the logs are not copied (the
convention of this file's header). Two new two-expression records are killed as named, both shells: the third round's (250, the name in a single-quoted span) SURVIVES
with the single-quoted-span step removed and with all three steps removed; the fourth round's (251, the name escaped, ` ; : \$_pkl_lbl`
appended to the `_pkl=bad` line) SURVIVES with the escape step removed, and with that step removed the 250 record is still killed as
named (0 not killed), so 251 is the escape step's only pin by a record (one-record opt-in trials, both shells; `_MUT_RECORDS_MIN=251`,
`_MUT_UNRECORDED_MAX=18` unchanged).

**Ledger: 22 of 576 rows fail, an instrument limit, with a manual backstop.** `ledger-check.py C1` reports the same 22 rows at
`505ad593` and at the head: the 12 settled CR-B2b and CR-B2c rows (§S.4, §S.5) and 10 retained hit lines of rewritten extents (the
registry header and the "reds if it grew" statement in `ctlrun.sh`, two in `mutations.sh`, six in `records.sh`), each read whole
by the first review and true. (At `82776f96` the set held 3 more lines, the dead guard's frame text, until `666eed96` restored
the guard.) Rewriting true lines to satisfy the checker would bless the instrument, so none is edited.

**The `_lbd_files` guard.** The fix round deleted `controls.sh`'s `[ "${#_lbd_files[@]}" -eq 0 ] || …` guard, dead since
`$SELF` starts the list (M-2). The driver restored it (`666eed96`): `…-cr-plan.md` :506 folds CL6/S-7 into CR-C2, and the sweep's
rows 18 and 24 give it to C2, so deleting it here would change a ratified slice boundary. It is carried to C2 with its message,
whose "no label was found in the parts" should name the wire too (C1 made `_lbd_files` start with `$SELF`). CR-C2 did both: the guard went and the message names the wire and the parts.

**Not measured.** (1) mawk: none is installed here and none was installed; landing X9's route (b) says the census's awk program
and rule 1's scan (`[[:space:]]`, the word-start cut, the dynamic `(^|…)` match) are the first mawk evidence of the CI run, and
that the CI `awk` being mawk is the plan's premise. (2) The trust simulation combined with the opt-in (row 6; red before any
mutant). (3) The apply-time MATCHED NOTHING path (the always-on anchor check reds first).

**Measured: completion.** Ship PASSED, rc 0, 0 `!!` lines, ×4 (BSD and GNU-like `grep` × bash 5.3 and 3.2) at `2ac70030`, at
`82776f96`, at `666eed96`, at `c2ce1ebf` and at `c46fc6c1`; `scripts/trip-wires.sh` rc 0 at the same points. Sizes at the head
(`wc -l`): `ctlrun.sh` 436, `controls.sh` 765, `mutations.sh` 900, `records.sh` 469, `harness.sh` 886 unchanged, the design memo
755, `cr-sweep` 934.

**Measured: the battery** (`BASE_REV=c6e69356`, on the tree of `4070c49d`, i.e. before the escaped-name record and the ratchet
comment, which change no always-on behaviour; both shells, identical): 88 comparable cases, 12 rc, PASSED or CONTROL FAILED
differences and 3 cases whose edit no longer applies, all explained. Intended: the four trust and window cases gain the census
red listing the three standing checks' call lines (§3.2's "Trust": a second red on a red run), and the gate-ask case one more
census red. Made stale by this slice: two cases now red on a record's anchor (the row loop the GC-3 record anchors), two whose
late unregistered call rule 2 now catches without `_mut_census_end` (retargeted onto a late duplicate registry row, the class
only that guard sees: red with the guard, green without it), one already stale at the base, the two empty-parts cases (the run
reds through orphaned records, rc 1, no longer the defined-labels check's exit 2, since that check reads the wire too), and the
three whose edited text moved (the `.labels` build, the literal-label test, the entry guard's list: needles fixed, each re-run
red or green as before). A control with the mutation copy set and no edit is green on both shells and on the base (the census
does not depend on the copies' names). The cases are maintained outside the repo (the measured-outside convention).

**Carried from §S.5, discharged here.** The design memo's census row now lists the membership record among its pins (row C).
**Carried onward to CR-C2.** (1) The `_lbd_files` guard and its message (above). (2) `mutations.sh` is at 900: C2's edits to it must
stay at or under 900, or the census half (`_mut_correspondence` and `_mut_census_end`) is split out first as a standalone commit,
per the plan's C1 row; this is the driver's carry, as a measured fact and not a plan clause of C2's. The reviews marked nothing
else for a later slice.
