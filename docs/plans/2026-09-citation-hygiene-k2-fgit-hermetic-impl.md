# K2 fixture git — PR #527's implementation record, from review round 3

This file continues `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-pr527-r15.md` ("`…-pr527-r15.md`"),
PR #527's record, for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). The design
memo's §13 (the rollover, D19-C) sends an entry that would take `…-pr527-r15.md` past 800 lines, and every later
one, here. `…-pr527-r15.md` was at 799 lines when round 7's record was written, so round 7 (R7) was written here and
`…-pr527-r15.md` §Q ends with a pointer to this file. Rounds 3–6 (R3–R6), which §Q does not hold, were recorded
afterwards, ahead of R7. The seam is chronological: every reference to "`…-pr527-r15.md` §Q" for an earlier result
still resolves there.

Rounds 3–7's working files (decisions, coder progress, reviews, runner cases, logs) are in the driver's handoff
directory outside the repo. This file states what they measured, so that a citation in the repo resolves in the
repo; the logs themselves are not copied. Every result below is relayed from those files as the coders, reviewers
and the driver recorded it, and was not re-run for this file, except where an entry says "measured for this file".

---

## §0 Spec coverage map

**No spec surface** — this file records review dispositions and measurements only. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §R Review round 3 onward, and the implementation results

### R3–R6: scope, sources and conventions

**Scope.** `…-pr527-r15.md` §Q records the PR from plan-review round 15 up to Stage 5. Its last two entries are
review R2 (`96eea58c`; its docs `9b61d51c`) and Stage 5 (`3dde656e`, `8f4ad3c8`; its docs `a3260410`, that entry's
"this commit"); its one later bullet (`116a9b8b`'s commit message) was added by `e585daaf` (R6 below). Not in §Q, and recorded here: `815f8d4b`, the push
and Codex R27/R28 (R3's opening), and rounds R3–R6, `2b56c33c`…`e585daaf` (`git log --oneline 815f8d4b..e585daaf`: 10
commits, measured for this file). Before this entry the repo held these rounds only as corpus §6.1's record rows;
those rows stay the record of each record, and the entries below point there instead of restating them.

**Sources, and what they lack.** The driver's handoff directory: `DECISIONS-R3.md`…`DECISIONS-R6.md` (the
dispositions), `PROGRESS.md` (its sections "R3" to "R6 Slice D2"), `NEXT.md` (the sections written 2026-10-02:
"State", "What went wrong", "Findings to fix", "UPDATE (round 6)", "RESULT of step 3"), `final-r6.json` (the review
that closed R6); the lane's memory file (the 2026-10-01/02 bullets: the push, Codex R27/R28, the /simplify counts);
and the commit messages. The runners and logs these sources cite (scratchpad `r2b/`, `rv/`, `rv5/`, `r6a/`…`r6d2/`,
`cr/`) lost their logs in the 2026-10-02→13 pause (`NEXT.md`: purged by the OS; `cr/` remains, holding only sandbox
git objects), so no figure below can be re-read from a log: every result is relayed as written, none re-run, except where an entry says "measured for this file". Where the
sources give no review counts, or disagree, the entry says so.

**Names.** "R3"…"R6" are the handoff's round names (its `DECISIONS-R<n>.md`); "review R2" in §Q is round 2. Codex
rounds (R26, R27, R28) are a separate count. Record numbers are the record table's line numbers in that round's
tree. The runs used R7's conventions (both shells, own process group, perl `alarm` plus a second `kill -9 -<pgid>`
watchdog, at most two at a time; each round's decisions file says so); single records were applied to a copied
tree, and no full opt-in mutation run was made in R3–R6 (DECISIONS.md: "that is the driver's, at the final head").

| round | commits | input | closing review | how it ended |
|---|---|---|---|---|
| R3 | `2b56c33c` | review R2's items dropped at its 15-finding cap (D-1…D-10) | `/simplify` on `815f8d4b..2b56c33c` | its findings became R4 |
| R4 | `fd85e29f` | that `/simplify` (S-1…S-9) | `/code-review max 815f8d4b...fd85e29f`: 15 CONFIRMED + 4 cut | top 3 = R4's regressions; instrument changed |
| R5 | `116a9b8b`, `d5601d5e` | that review (items 1–15, (a)–(d)) | `/simplify` (2 MIN, `d5601d5e`), then a `/code-review max` | its findings K1–K8 / D1–D7 became R6; method changed |
| R6 | `04466154`…`e585daaf` (6) | K1–K8, D1–D7 | `/code-review max 815f8d4b...` at `e585daaf`: 15 CONFIRMED | terminator fired; stopped, reported to the user |

No Codex round ran in R3–R6: nothing was pushed, and the pushed head stayed `815f8d4b` throughout (`NEXT.md`).

### R3 — `2b56c33c`

**Before it.** `815f8d4b` (one landing line: X10 says why the wire, touched by `8f4ad3c8`, is not split) was pushed
on 2026-10-01 after the five pre-push stages, R26's four threads answered and resolved. Codex R27 (triggered
13:22:11Z on `815f8d4b`): DRY at 13:27:59Z, threads 27 of 27 resolved; R28 (14:29:04Z, same head): DRY at
14:36:28Z. Two dry in a row, but the TERMINAL attestation failed: review R2 had dropped CONFIRMED items at its
15-finding cap, undisposed (memory file; `DECISIONS-R3.md` "Why now").

**Changed** (`DECISIONS-R3.md`; the commit message). D-1: `_control` copies `_ctl_env` into a local and clears the
global as its first statement, so no return path leaves it to the next control. D-8: one gate, `_ctl_gate
<fixture> <label>`, for `_control` and the three blocks that are not one (relcwd, fsmon, umask); fixtures gain
`mkdir relcwd … || _fixture_failed relcwd`. D-3: the correspondence's per-record and per-label `grep` forks → one
`awk` each; P-k's `grep '^alternate:'` → the builtin `_fgit_alternates` (the exit-above-1 arm and its record go).
D-4: one `git var` per name. D-5: one alternates helper for P-k and its liveness probe. D-6: the canary spelled once
(`_FW_CANARY`, through the prelude). D-7: 34 `built`-anchored fixtures records write their anchor once (`&`). D-2,
D-9: comments (P-h pins the window's `LC_ALL` equal to the wire's, not `C`; `_mut_run`'s machine-limit excuse keys
on the label naming, the fail-safe side). D-10: rejected, no change (the shape scan's and P-g's cost is the check).
Records +4 +1 +1 −1 → 195 (corpus §6.1, the `2b56c33c` rows); `_MUT_UNRECORDED_MAX` 20. The commit carries
`Co-Authored-By` Sonnet 5.5.

**Measured** (`PROGRESS.md` "R3"; both shells unless stated):
- D-1, on an instrumented copy (a refused control with `_ctl_env=(D1PROBE=1)`): `_ctl_env` left with 1 element at
  `815f8d4b`, 0 after.
- D-8: `_fixture_failed relcwd; fsmon; clean` injected in a copy: the clean control and the three blocks each
  `CONTROL NOT EXERCISED (<own label>)`. The four new records KILLED, 4 of 4; with the block's `_ctl_gate` call
  removed, 3 of 3 survive.
- D-3: the whole wire on 5.3, 3 alternated runs: before 17.13 / 17.23 / 17.30 s (mean 17.22), after 17.07 / 16.91 /
  16.84 s (mean 16.94). `_umask_lbl` renamed in a copy: both `appears nowhere` lines and `21 labels have no mutation
  record, against 20`.
- D-4: `git var` calls in a full run (a logging `_fgit_in` in copies): 10 → 6.
- 74 records (the 37 `built`-anchored fixtures records, and every record near the D-1…D-8 code) on a copy of the
  final tree: 74 of 74 KILLED with their own needle.
- Ship green on both shells (18 s / 24 s); the `K2P\|K2REF` grep and X4b empty; `_MUT_RECORDS_MIN=195` = the count.
  Stage 2 (`mise run ci`) green except `deny` (memory file).

**Review.** `/simplify` on `815f8d4b..2b56c33c`: reuse 1, simplification 4 (F1: the correspondence awk fails open),
efficiency 1, altitude 5 (A-1, IMPORTANT: the three blocks have no watchdog) (memory file; no other severity
stated). Its findings are R4's input.

### R4 — `fd85e29f`

**Changed** (`DECISIONS-R4.md`; the commit message). S-1: one run primitive, `_ctl_run` (gate, `set -m`, the 30 s
watchdog with a group kill, capture, the timeout report; it owns the `_ctl_env` take-and-clear); `_control` and the
three blocks run through it. S-2: `_ctl_gate <label> <fixture>…`, its names derived from root, cwd and the PATH-prefix
dirs; relcwd's fixture lines wrapped in one `( … ) || _fixture_failed relcwd`. S-3: `_fixture_did_fail`, shared by the
gate and `_seal_apply`. S-4: one parsed label set, one awk for both directions, its exit the verdict. S-5: `&` in 6
records. S-6: one canary scan, `_FW_CANARY` by `ENVIRON`. S-7: `_fgit_alternates` assigns `_fgit_alt`. S-8:
`_ctl_env`'s copy in one idiom. S-9: comments. Records 197 (+`_fixture_failed fakerelmktemp`, + the canary scan's exit
3). A first coder run stopped at its turn limit with nothing committed; the round then landed as one commit
(`Co-Authored-By` Sonnet 5.5). An untracked empty file came from a coder's BSD `sed -i` (`-i` took the script as its
suffix); from R5 on the rules say "never `sed -i` in the worktree".

**Measured** (`PROGRESS.md` "R4", both entries; both shells unless stated):
- S-1: a hang injected in each of pin, relcwd, fsmon and umask: after, `killed after 30s` under the block's own
  label, rc 1, 46–52 s in all; before (`2b56c33c`), relcwd, fsmon and umask: no label, the alarm's rc 142 at 80 s.
- S-2: `_fixture_failed fakegit` / `fakerelmktemp`: before rc 0 PASSED, after rc 1 NOT EXERCISED naming the
  control. 59 fixture trees sit outside a `|| _fixture_failed` group (plain lines under errexit), not wrapped.
- S-4: the awk forced to fail (rc 2) → red; a renamed label → red; a record needle `PASSED` → red (green on the old
  tree). S-6: the scan's exit 3 → red under the shell-diagnostic label. S-7: subshells from the alternates extraction,
  by `BASHPID`: 81 before, none after. S-8: green on 3.2 under `set -u`. S-9: its premise was false (harness :291-292
  at `2b56c33c` held no history sentence); one phrase reworded.
- Records: 42 of 42 near R4's code KILLED with their needle, the six `&` records and the two new ones included.
  Gates (records 483–485 in that tree): with `_ctl_run`'s gate removed, 483 and 485 survive and 484 is killed by
  fsmon's preflight; with the preflight removed, 484 is killed by `_ctl_run`'s; with both removed, 484 survives.
- 5.3 timing, 3 runs each: `2b56c33c` 16 / 18 / 17 s, R4 17 / 16 / 18 s, no measurable change. Ship 17 s / 23 s.

**Review.** `/code-review max 815f8d4b...fd85e29f`: 15 CONFIRMED and 4 dropped at the cap (`DECISIONS-R5.md`; the
findings in scratchpad `rv/final.json`, purged). Its top three were regressions R4's refactors introduced (S-1's
PATH prefix, S-4's `awk -v`, S-6's canary). With R4's own findings mostly on R3's additions, that made two rounds in a
row mostly self-introduced, so the instrument changed (R5) instead of "one more round".

### R5 — `116a9b8b` and `d5601d5e`

**The instrument** (`DECISIONS-R5.md`): corrective only (each fix removes, replaces or restores the base form; no
new helper, primitive or doc section; an over-reach is removed, not built around); docs replace and never add;
rounds named by commit in shipped text; terminator: if the next review is again mostly self-introduced, the driver
stops and reports to the user.

**Changed** (item numbers in the order of that review's `rv/final.json`, not the handoff's `final.json`, which is the
2026-09-30 review's; the commit message): 1, awk output paths by `ENVIRON`; 2, the run primitive exports its PATH
prefix; 3, canary detection is the awk's own exit and the replay `|| true`; 4, a record pinning P-k liveness's
two-line clause; 5, the canary scan's failure held for the trust decision; 6, a status ≥ 128 is a timeout only when
the watchdog's marker exists, relcwd's arm prints rc and output; 7, the `_ctl_env` standing check asks `_ctl_run`; 8,
the gate's PATH-prefix derivation removed (and the `fakerelmktemp` record); 9, a `_control` line with no extractable
label is red; 10–15, docs and comments; (a)–(c) done, (d) rejected (cost below noise). Records 197 (−1
`fakerelmktemp`, +1 second store). `d5601d5e`: the two comment corrections of `/simplify` on `fd85e29f..116a9b8b` (2
MIN; memory file). Both carry `Co-Authored-By` Opus 5.5; who made them is not stated.

**Measured** (`PROGRESS.md` "R5", both entries; tree `h0` = `fd85e29f`, `n1` = edited; both shells):
- A backslash `TMPDIR` and a newline `TMPDIR` (shims): `h0` rc 1, `n1` PASSED. `env -i <bash> <wire>`: `h0` rc 1
  with 11 CONTROL FAILED, `n1` PASSED. `2>&-` on 3.2: `h0` rc 1, `n1` PASSED.
- Item 4's record KILLED; it survives with the clause removed (rc 0, needle absent). Item 5: with the fold removed,
  record 481 survives. Item 6: no record pinned the marker clause ("no fixture can exit 134 at once"): unmeasured,
  K2 of R6. Item 9: a tab-indented unrecorded `_control` → red naming the line. Item 10: the gate variants
  re-measured as in R4.
- 32 of 32 records near the changed code KILLED with their needle. Ship 17 s / 23 s; mutations 997 lines, harness
  963. Not run: an always-on run after the last doc edits (`PROGRESS.md` "NOT DONE"); `NEXT.md` "State" records the
  wire green on both shells at `d5601d5e`, and local CI green except `deny` (PR-external advisories; `Cargo.lock`
  unchanged since base).
- `116a9b8b`'s message says "four records use `&`"; three were re-shaped to `&` and one new record uses it (corpus
  §6.1's `116a9b8b` rows; `PROGRESS.md` says "(b) DONE (4 records to `&`)"). §Q's last bullet notes it.

**Review, and how R5 ended.** A `/code-review max` ran as the terminator's round (the memory file: range
`fd85e29f...d5601d5e`). `NEXT.md` then lists the findings to fix, naming as their sources `rv5/partial.md` and
per-agent notes (purged); no source says in so many words that the list is that review's whole output. The list: K1
(the label parser reads text, not calls: a TAB fails open, a mention reds), K2 (116a9b8b's two guards, the marker
clause and the bare-label red arm, unpinned), K3 (relcwd and cfgkept show no evidence their shim ran; at `fd85e29f`
under `env -i` both green while 11 controls were red; older), K4 (the err-seal record not `&`, a miss of R5 (b)), K5
(the second-store path not C-quoted like its line), K6 (`_ctl_run`'s return-contract comment), K7 (the marker write
can stop the kill), K8 (fsmon's gate name a literal); docs D1–D7 (D1: the per-round totals bullets collapse into one
current statement). No severity counts and no origin split are stated. `NEXT.md` "What went wrong": R3–R5's fixes had
been committed after a verifier pass only, with no driver read of the diff, so the conclusion "the loop does not
converge" was confounded and withdrawn; the method agreed with the user on 2026-10-02: fix every finding, the driver
reads the full `git diff` before every commit, then one `/code-review max 815f8d4b...HEAD` measures whether
self-reviewed fixes converge.

### R6 — `04466154`…`e585daaf`

**Rules** (`DECISIONS-R6.md`): the coder does not commit; the driver reads the diff and commits; awk by `ENVIRON`,
fail-closed, a record or a standing check per new guard (KILLED shipped, SURVIVING without the guard) or a written
reason. Six commits, each diff read by the driver (`NEXT.md` "UPDATE (round 6)"), all `Co-Authored-By` Opus 5.5.

**Slice A, `04466154`** (prereq split). `_mutants` and its comment block (mutations :218-518) moved to
`…trip-wire.records.sh`, sourced beside `…mutgen.sh` with the same missing-sibling guard; `records` joins
`_MUT_PARTS`. Measured: `diff <(git show 04466154~1:…mutations.sh | sed -n 218,518p) <(git show
04466154:…records.sh | sed -n 18,318p)` differs in 8 diff lines (records.sh's added header line, three pointer
rewordings, the closing `}` absent from the range; the commit says "apart from three pointer words"); ship 17 s / 23 s; one record per target (wire 42, fixtures 144, mutgen
244, controls 289, harness 308) KILLED; records.sh removed → rc 2, "missing or unreadable … decided nothing"; a
trial's copy set includes records.sh; `bash -n` clean.

**Slice B, `5b5c2ca0`** (K1, K2's parser half). `_mut_labels`: one awk sets quoted strings and comments aside and
reads calls (a `_control` call starts its line in the four-argument shape; an `_lbl=` definition is
`NAME="non-empty"`; a `_ctl_run` call's first argument is `"$_<name>_lbl"`; anything else that is a call is red).
A standing check `$_lp_lbl` over synthetic lines; `_MUT_UNRECORDED_MAX` 20 → 21. Repro at `04466154`: a TAB-separated
`_control` with a new label → green (fail-open); a tab-indented comment mentioning `_control ` → red. After: the
first → red (`22 labels … against 21`), the second → PASSED. `.ctllabels` byte-identical, `.labels` identical plus
`_lp_lbl`. The non-line-start arm removed → red under `$_lp_lbl` end to end; the `_lbl=` and `_ctl_run` arms
removed → the same label, in isolation. The coder stopped on a design question: controls :188's `_ctl_run x`
reds under the new rule; option A (`_ctl_run "$_ce_lbl"`, measured on a copy: ship green, record 277 KILLED) is
what the commit applied (`git show 5b5c2ca0`, measured for this file).

**Slice C, `aa0a501b`** (K2's other half, K6, K7, cleanups). A standing check `$_wm_lbl`: `SELF` points at a
script that signals itself at once, and `_ctl_run` must return 0 with `_rc` ≥ 128 and no `killed after 30s`;
a record deletes the marker test; `_MUT_RECORDS_MIN` 197 → 198. The marker write `{ : > …; } 2>/dev/null || :` (no
record: every caller is an `||`/`if !` operand); the return-contract comment; `_fw_canary_bad` and the inert reset
gone, `_r_names` inlined. Measured: at `5b5c2ca0` the marker clause removed → PASSED (the gap); the record KILLED
(`_ctl_run returned 1, not 0`); the canary exit-3 record KILLED, and survives with the fold removed (as a
`.mutant.99` copy; as a plain copy only the stale-anchor check reds); `set -u` with an empty cwd yields no empty
word. ⚠ `PROGRESS.md` describes the script as `kill -ABRT $$`; the commit's is `kill -TERM $$` ("so no core dump",
its message; `git show aa0a501b:…controls.sh` :201, measured for this file). Which signal the logged runs used is
not stated.

**Slice D+E, `2d5c393f`** (K4, K5, K8). The err-seal record → `&` (old and new outputs `cmp`-identical, KILLED);
P-k's second-store record C-quotes its path by the extended line's `printf | sed`: under a backslash `TMPDIR` the
old record was killed by the stderr clause (git cannot normalise the raw path), the new by the more-than-one-line
clause (two `alternate:` lines, empty stderr), both shells; fsmon's gate, preflight and run derive from
`_fsm_root` (record 282 KILLED). K3 stopped on a false premise: `_VFY` does not exist at fixture build time.

**Slice D2, `e271cb02`** (K3, by the driver's option B: the controls build both shims under `$_VFY`, as the fsmon hook
is). Repro at `2d5c393f`: both PATH-prefix arguments replaced by `""` → PASSED on both shells (neither shim ran). Each
shim touches a mark; absent → NOT EXERCISED. A first record shape spelled its needle in its expression and was
"killed" by the mutant's stale-anchor report alone; the shipped two are tail-anchored. Records 198 → 200. Measured:
ship 17 s / 22 s, and under `env -i` (marks present); both records KILLED; with the mark checks removed neither
control's line is printed (rc 1 from the stale-anchor report only); record 281 KILLED. Two run batches ran while the
machine slept (15–19 min of wall clock, a normal verdict); later runs were made under `caffeinate -i`, and the timings
above are from runs that did not sleep.

**Slice F, `e585daaf`** (D1–D7; the commit message, `PROGRESS.md` has no entry). The per-round totals bullets
collapse into one current statement (105 of this PR, 200 in all, measured at `e271cb02`); residuals R9 and landing
§9.2 corrected; the design memo's sketch labelled by commit; part lists gain `records`; #519's wording at fixtures'
head-probe comment restored; §Q's `116a9b8b` bullet. Ship green on both shells (17 s / 23 s).

**The review that closed R6.** `/code-review max 815f8d4b...k2-wire-fgit-hermetic` at `e585daaf`: 15 CONFIRMED, no
CRITICAL, no false green of the required gate; origin 9 in this round, 2 mixed, 4 older (`NEXT.md` "RESULT of step 3",
which does not say how each finding was classed; `final-r6.json`'s per-finding origins are below). By the terminator,
the driver stopped and reported to the user; no fix round was started. R7 fixes all 15 (`DECISIONS-R7.md`: all real;
the instrument changed, R7 below). `final-r6.json`, in its order (file:line at `e585daaf`; origin as it states):

| # | finding | origin |
|---|---|---|
| 1 | the `_lp_lbl` check tries 3 of the parser's 6 red arms; "asserts every arm" false (mutations :534) | `5b5c2ca0` |
| 2 | env-channel controls (routed, committed, k2, fsmon) never show the environment reached the run (controls :397) | global from `815f8d4b`; `fd85e29f`, `e271cb02` |
| 3 | corpus §6.1 stops at `8f4ad3c8`: no rows for the 11 records of `2b56c33c..e271cb02` | `e585daaf` and others |
| 4 | the tokenizer resets quote state per line: a call after a multi-line string's close fails open | `5b5c2ca0` |
| 5 | the correspondence awk's orphan arm (`bad = 1`) pinned by nothing (mutations :593) | `fd85e29f` |
| 6 | cfgkept's mark is written by the pre-purge `git rev-parse` too, so it shows nothing (controls :385) | `e271cb02` |
| 7 | the literal-label check covers `_ctl_run` only, not `_ctl_gate` or `_fgit_window_verdict_exit` | `5b5c2ca0`, `2b56c33c`, `fd85e29f` |
| 8 | `_wm`'s check drops `_ctl_run`'s stderr and needs `clean` built; cfgkept's mark check is not an `elif` | `aa0a501b`, `e271cb02` |
| 9 | cfgkept's NOT EXERCISED line repeats its label literally; drift reds only in the opt-in run | `e271cb02` |
| 10 | memo "every tree the run reads built" is wider than the gate; `routeddecoy`'s failure record write-only | `fd85e29f`, older |
| 11 | the watchdog marker's write is pinned by nothing (harness :933) | `116a9b8b`, `aa0a501b` |
| 12 | three `_ctl_gate`/`_ctl_run` header comments disagree with the code (harness :850) | `116a9b8b`, `fd85e29f`, `aa0a501b`, `e271cb02` |
| 13 | the `controls:` records' anchor reason is false; docs say controls is no mutation target | `e271cb02`; older (`96eea58c` on), re-exposed |
| 14 | label docs: "the one label no record names" (21 have none), 8 of 27 `_lbl=` without a §4 row | `5b5c2ca0`, `2b56c33c`, `aa0a501b` |
| 15 | umask and fsmon blocks still duplicate `_control`'s assertion and the gate's question | `fd85e29f` (`2d5c393f`) |

### R7: scope, instrument and conventions

**Scope.** Round 7 starts at `e585daaf` (pushed head `815f8d4b`) and fixes the 15 findings of the review that closed
round 6 (`final-r6.json`, numbered #1…#15 in that file's order, listed at R6's end), and the MINORs that round 6 cut
(C1–C5 CONFIRMED, C6 PLAUSIBLE and measure-only). All 15 were dispositioned as real and fixed; none went to a ledger.
Commits, in order: `16efb423` (prereq split), `d976f239` (S1'), `288c857c` (S3), `b164f3ae` (S5a), `c8eb9ab4` (S5b-1),
`0eee71c7` (S5b-2), `8c952d6d` (S2a), `fd90b4b1` (S2b), `0d411df2` (S2c), `20615255` (S4c-0), `1fd70133`
(S4a+S4b), `173b81dc` (S4d), `e0db887a` (S4c), then S4e+S4f, the commit that adds this file (`git log --oneline -6`).

**The instrument (binding for every slice).** Round 6's terminator fired (9 of its 15 findings were introduced by
its own fixes), so the instrument changed (the defect class: "a guard or a claim not pinned as stated", which
removal experiments find and diff-reading does not):
1. **Claim table.** Every new or changed guard, and every sentence a slice writes or keeps that says "pins",
   "asserts", "every", "only", "never", "one", a count or "X reports Y", gets a row: claim | how measured (command,
   both shells) | result. A claim that cannot be measured is deleted. A guard: removed → something reds (named);
   shipped → green.
2. **Altitude.** Removing a mechanism is preferred to adding one; a new check needs a reason the finding cannot be
   fixed by removing or moving code.
3. **A fresh `reviewer`** reviews the slice diff adversarially on a frozen snapshot before the driver commits; its
   CONFIRMED items are fixed in the same slice.

After S1' review 2 (all 8 items self-introduced, the second such review in a row) the instrument gained two rows
(v2, binding from then on): (4) an **arm-mutant table** — every arm of every guard the slice touches gets a mutant
run, red under a named label, or pinned by a record (the arm removed + the record's edit SURVIVES while the record
alone is KILLED), or a written equivalent-mutant reason ("works" rows do not count); (5) a **regression battery** —
every reviewer repro case is kept as a file and run on the new tree and on the slice's base, both shells; any
red→green flip against the base is a finding unless the slice's decision says so.

**Conventions of the measurements.** b53 = bash 5.3 (5.3.20), b32 = `/bin/bash` 3.2 (3.2.57); "both shells" = each
row run on each. Every wire run is in its own process group (`perl -e 'setpgrp; exec'`) with a `kill -9 -<pgid>`
watchdog, except the two full opt-in runs (S2c-2's and the run at T in R7.8; `runners/optin.sh` and
`runners/optfin.sh` run the wire directly, with no watchdog): a full run takes ≈63 min per shell (R7.8). At most two
wire runs at once (one queue per shell). Runners edit copies (`git clone --local` of the worktree, the parts copied
in), never the worktree. A record's or a case's edit is applied by a python `str.replace` whose count is asserted to
be exactly 1. A **`.mutant.99` copy set** runs the edited parts under the names a real `_mut_trial` gives them
(`<wire>.mutant.99.<part>.sh`), so the always-on stale-anchor check is skipped
as in a real trial; cases marked **(m99)** run that way. Verdicts follow `_mut_trial`: **KILLED** = rc ≠ 0 with the
record's needle in a diagnostic's label slot (`CONTROL FAILED (<needle>)` or `CONTROL NOT EXERCISED (<needle>)`, from CR-S5 on; R7.13); **SURVIVES** = rc 0; **WRONG-REASON** = rc ≠ 0 without it (the runners' `rec.sh` still tests the bare needle). Record numbers
below are records.sh line numbers at the time of the measurement, which moved between rounds; records are also
named by their shape. "Ship" = the always-on wire on the slice's final tree.

### R7.0 Slices at a glance

| slice | commit | subject (final-r6 # / cut item) | review rounds, CRIT / IMP / MIN |
|---|---|---|---|
| prereq split | `16efb423` | run primitives out of the harness (touch-time, before S1' grows them) | none recorded |
| S1 | — (discarded) | static label parser extended (#1, #4, #5, #7, #14 part) | pre-commit review: IMP 3 (CRIT, MIN counts not stated) |
| S1' | `d976f239` | label population from what the run executed | port 0/2/5; 2: 0/3/5; 3: 0/1/5; 4: 0/0/3 |
| S3 | `288c857c` | one W2 producer, umask as a `_control`, a pinned watchdog (#8, #11, #12, #15) | 1: 0/1/5; 2: 0/0/1 |
| S5a | `b164f3ae` | P-b one loop, locals, one parts list (cut MINORs) | 1: 0/0/4 |
| S5b-1 | `c8eb9ab4` | P-b's machine limit by a definition test (S5a review's observation) | 1: 0/1/4; 2: 0/0/2 |
| S5b-2 | `0eee71c7` | claims (cut MINORs; C6 closed) | none (blast radius: 2 comment lines) |
| S2a | `8c952d6d` | channels travel with the call; env channels and umask evidenced (#2) | 1: 0/0/6 |
| S2b | `fd90b4b1` | the PATH-prefix channel evidenced (#6) | 1: 0/0/3 |
| S2c | `0d411df2` | k2 asked only where the run's grep honours `GREP_OPTIONS` | 1: 0/1/4; 2: 0/1/1; 3: 0/0/2 |
| S4c-0 | `20615255` | design memo split at its 950-line seam (`…-census.md`) | reviewed with S4c |
| S4a+S4b | `1fd70133` | corpus §6.1 rows for every record change since Stage 5 (#3, #14 part) | 1: 0/1/4; 2: 0/1/2 |
| S4c | `e0db887a` | memo, census and tool comments; `routeddecoy` built bare (#10, #13, #14 rest) | with S4c-0 — 1: 0/2/4; 2: 0/0/3 |
| S4d | `173b81dc` | the #519 memo's banner, reviews §8.2, landing X10 (#13) | with S4e+S4f — 1: 0/2/5; 2 (delta): 0/1/5 |
| S4e+S4f | the commit that adds this file | this file: R7 (S4e), then R3–R6 ahead of it (S4f) | with S4d (above) |

final-r6 #9's subject was gone before S4 (the S4 claim table: the sentence went in `288c857c`; from `fd90b4b1`
cfgkept's evidence reds under `_ctl_run`'s own label).
A slice split in two passes is named `-1` (the code, ship on both shells, greps) and `-2` (records in both directions,
the measurements, the battery): S2a-1/S2a-2, S2b-1/S2b-2, S2c-1/S2c-2, each pair landing as its slice's one commit.
S5b-1 and S5b-2 are two slices, each with its own commit.

S2 was planned by a designer memo (channel evidence, approved 2026-10-02; re-anchored onto `0eee71c7` on
2026-10-14 after S1'/S3/S5 moved its anchors), S1' by a designer memo (the runtime registry, approved
2026-10-02), S4 by a designer claim table on `0d411df2` (30 items). The order run: prereq split, S1', S3, S5, S2, S4.

### R7.1 Prereq split — `16efb423`

**Changed.** A pure move: the harness's `_ctl_gate`, `_ctl_run`, `_control` and their comments (harness :839-966 at
`e585daaf`, 128 lines) moved to a new part `.claude/tools/webref-generic-core-trip-wire.ctlrun.sh` (157 lines with
its header), sourced at the harness's end behind `_CTLRUN`. Harness 966 → 856 lines. The moved body is
`cmp`-identical to the old range; two `harness:` records re-aimed `ctlrun:`; `ctlrun` joins `_MUT_PARTS` and
`_MUT_TARGETS`.

**Measured.** Ship PASSED rc 0, 5.3 22 s / 3.2 28 s. ctlrun.sh removed: rc 2 on both shells ("the control run
primitives beside this harness … are missing or unreadable … decided nothing"). The two re-aimed records (`_ctl_env=()`
deleted; the watchdog-marker test deleted): KILLED on both shells, with their needles. Opt-in run with the table cut
to those two records and `!survive`: the hand set "3 entr(ies), 0 not killed as named, 0 not exercisable" on both
shells; the whole run PASSED on 5.3 (1034 s) and on 3.2 (re-run alone under an 1800 s runner alarm, after a first
attempt hit the runner's 1200 s alarm in the generated half).

### R7.2 S1 (discarded) and S1' — `d976f239`

**S1, discarded.** The first S1 extended the static label parser (`_mut_labels`: quote state across lines, every
label-taking primitive, a table-driven standing check). Its pre-commit reviewer found 3 IMPORTANT — two fail-open
(a `;#` comment with an apostrophe hides the next call; `${_x_lbl:=…}` sets a label past the bare-name arm) and one
of unpinned arms and readings — and MINORs (partial quotes and `eval` hide calls; `"$1"` read as a label; their
count is not stated in the decision record). The driver's lens: a text parser of bash enumerates shapes, and every
unlisted shape falls open. The diff was discarded; the label population is taken from what bash ran.

**S1', changed.** `_ctl_reg`, the first statement of `_ctl_gate` and of the window verdict, appends
`<file>:<line>`, `<kind>`, `<label>` (TAB-separated) to `$_VFY/.registry`. The census (`$_cen_lbl`, "every label
the run used is named by a mutation record") reads that registry and the values of the `_*_lbl` names the parts spell (`$_lbd_names`, a
text derivation, each asserted defined — not `compgen -v`, so a caller's environment adds no label). The static
parser and its standing check `$_lp_lbl` are gone. Rules: one label, one site, the site being the outermost frame
whose `BASH_SOURCE` is the controls file; a `_control` label is literal (kind `control` when `_control` is on the
stack; a `_*_lbl` value passed to `_control` is red). `_mut_census_end`, the controls file's last statement, reds
if the registry grew after the census. `_ctl_need` carries the flag's value (no indirect expansion); a new label
`$_cn_lbl` ("`_ctl_need` declines exactly what this machine cannot do") has a table-driven standing check run under
its own `$_VFY/.cn`; a declined call is noted in `$_VFY/.declined`, each capability group's summary line is decided
from it, and a last ask before the controls' exit reds any decline no group consumed. The wire's diff vs
`16efb423` is empty. Records 200 → 213; `_MUT_UNRECORDED_MAX` 21 → 20, exact.

**Reviews.** Port review (on the port onto the split): 28 claims, 7 falsified, 0 / 2 / 5 — IMPORTANT: (1) the site
walk's harness-frame skip had no record; (2) the census's `compgen -v` counted a caller's exported `_zz_lbl` as a
label (false red, and the inverse could hide an orphan). Review 2: 65 claims/guards, 14 falsified, 0 / 3 / 5 —
(I1) a label call after the census in a `( … )` was refused and the run still passed (a regression from base);
(I2) no arm of the new `_ctl_need` gate was pinned: a one-token edit declined 6 controls in silence on a capable
machine; (I3) one label, one site missed a second producer when the first was not a registered call (`mgreuse`,
`pkreuse`; a regression from base). Review 3: 64 claims/guards, 7 falsified, 0 / 1 / 5 — (I1) a decline outside a
`_cn_declined` window was silent (the control never ran, the run green, also on a capable machine). Review 4: 36
claims/guards, 2 falsified, 0 / 0 / 3 (the standing check's own `$_VFY` was pinned only by its `0 2` row being
last). Review 2's terminator: all 8 items self-introduced, the second review in a row → instrument v2 (above).

**Measured (the final S1' state after review 4's fixes, and the survive pairs measured across its rounds; both shells,
identical unless stated).**
- Ship: rc 0 PASSED, CF 0, NE 0, 20 s.
- Kills (`.mutant.99` copy sets): 17 of 17 KILLED on each shell, each by its own needle — the census block's records
  (registry writer, window literal, late `( … )` call, variable half emptied, `forge` deleted, fsmon literal,
  `ignored` deleted, `ignored` second control, second verdict call, helper called from two lines, `$_mg_lbl` as a
  `_control` label), the fifotracked `_ctl_need` → 0 record, the last-ask record, and four ctlrun/fixtures records.
- Survive pairs (each: the guard removed + the record's edit, m99 → rc 0 PASSED; the record alone KILLED):
  kind detection and the census kind rule (`S10nokind`, `S11nokindrule`); `_mut_census_end` disabled and its call
  line removed (`S12noend`, `S15noendline`); `_cn_declined`'s growth arm and red arm (`S13nogrowth`, `S14nored`);
  the last ask (`S16nolastask`); the window verdict's registry writer (`R3noverdictreg`); the site rule's
  `bad = 1` (`m99nositedupn`, on the first draft's review-fix tree), a walk that skips only ctlrun's frames, under
  which both verdict calls fold to the harness's line (`survA_skipctl`), and a nearest-controls-frame walk
  (`survB_near`). Case edits: R7.10, with the S4ab review's forms at `20615255`: `hR3nvr` and `hsurvA` re-aimed, `hsvR8`
  and `hsurvB` as `svR8` and `survB_near` are.
- Arm table (review 2, 3, 4 rounds): every arm of the gate (`1)`, `0)` with its note, `*)`, `unset`, the failed
  note, the empty value, `|| return $?`) red under `$_cn_lbl` when removed, both shells; `_control`'s 2 → 0 mapping
  is an equivalent mutant on a capable machine and fails safe on an incapable one (fifo flag forced 0 and the
  mapping removed: rc 1, nothing printed), the reason written beside the line.
- Registry dump (review 3): 88 lines = 82 `control` + 6 `direct`, every site in the controls file, no label with two
  sites.
- Battery (55 cases × the new tree and base `16efb423` × 2 shells): red→green flips vs base 0; green→red `latecs`
  and `pkreuse` (intended by I1, I3); digests b53 = b32.
- No opt-in `WEBREF_WIRE_MUTANTS=1` run on the final S1' state; the per-record `.mutant.99` trials stand for it.

### R7.3 S3 — `288c857c`

**Changed.** (#15) The umask run is a `_control` (`$8` = the umask, passed as `_ctl_run`'s `$4`), the fifth call of
the perm group, before its ask; `_umask_lbl` is gone. (#8) `_ctl_gate` keeps base's order — register, `unset
_ctl_need`, the trust arm inline (W2 once, return 1), the need arms, then `_ctl_built` (the fixture loop only, NOT
EXERCISED under the label); the standing checks open `if [ "$_fw_trusted" -ne 1 ] || ! _ctl_built "$<label>" clean;
then ctl_ok=1`, a silent red, so W2 has one producer, the gate; the `_cn` table gained an `untrusted` row, then
column validation and a total `case` (review 2); cfgkept's mark check became the `elif` of its control's `if`.
(#11) `_ctl_wd_s=30` is set at ctlrun's source; the `_wm` check is table-driven, its `hang` row (`_ctl_wd_s=1`, a
script that sleeps 5 s) pins the watchdog marker's write and the `killed after 1s` message. (#12) three ctlrun
comments brought to the code.

**Reviews.** Review 1: 0 / 1 / 5 — (I1) the standing checks' precondition had become the first W2 printer, so W2's
records (160/168/169) no longer showed that `_ctl_run` refuses an untrusted build, and the memo sentence closing
the "declared gap" was false. Review 2: 0 / 0 / 1 (a `_cn` row with a garbage or missing 5th column lost its arm in
silence). Driver decisions on the way: the coder's first gate order put the need arms before trust (`S3declutrust`:
an untrusted window became a decline), restored to base's order; `R8gatenotrust160` (record 160's shape with the
gate's trust arm removed) is WRONG-REASON — rc 1, W only, no W2 line — not SURVIVES, accepted as fail-closed
(`S3fnoprecond160`, the preconditions' red removed too: rc 0 PASSED, shows the rc 1 is theirs).

**Measured (final state; both shells, identical).** Ship rc 0 PASSED, CF 0, NE 0. Kills: 20 of 20 (the census
block, the fifo and last-ask records, the ctlrun/fixtures records, the umask records and the re-aimed cfgkept
record) + the W2 records 160, 168, 169 KILLED. Survive pairs: cfgkept's `elif` (`S3cfgnoelif330`), the fixture arm
(`S3nofix284`); the `hang` row is the marker write's only pin (`S3nohangrownomark`, row and write removed: PASSED).
Arms red under a named label: the marker write (`S3nomark`), the message (`S3nomsg`), the timeout clause
(`S3noclause`), the `sleep` reading `_ctl_wd_s` (`S3wdfixed`), the umask argument (`S3umask8`), the umask message
(`S3umaskmsg`), the trust arm's place and the row's restore (`S3fm1trustbelow`, `S3fm1norestore`), the validation
and the `*)` arm (`R9cn4colnoop`, `R9cn0dir`, each the only case that flips green when its guard alone is removed).
Final-r6 #8's repros: a failed clean fixture → NE under each check's label, no false line (CF 0, NE 6); record
160's shape → W and one W2 line (CF 1); a failed cfgkept fixture → one line. Registry: 89 rows = 83 `control` + 6
`direct`, no label with two sites. Battery (100 cases × 2 trees × 2 shells, base `d976f239`): red→green 0; green→red
9 (`S3nomark`; four "gate's trust arm off" cases, now red through the `untrusted` row; four new cases green at base:
`R9cn5garbage`, `R9cn0dir`, `R9trustaboveunset`, `R9gatenotrust168`); digests b53 = b32. Records 213 unchanged.

### R7.4 S5a — `b164f3ae`

**Changed.** P-b in one loop over the four names (`git var` once per name, then that name's checks; the
`printf -v` indirect names gone); the P-b status record re-anchored to the loop's status line (`_prc=0; _o="$(_fgit_in`
→ `_prc=1; …`). `_out_f`, `_r_wmark`, `_cpid`, `_wpid` local in `_ctl_run`; the `_wm` hang row reads
`$_VFY/.control_timeout`. `_MUT_PARTS` spelled once, in the controls file above `_lbd`; `_lbd_files` derived
from it; the mutation set reads it, red if unset. A coder's `continue` after the 129 add was dropped: removed, it
stayed green and byte-identical (pinned by nothing) and left the machine-limit arm unpinned.
[CR-S3: that claim is corrected. The "discarded" SYSTEM probe was the pin of the arm's ORDER only (limit line before `_pbl`). Under K2 the limit line is printed after the reds, and a 129 on a git below 2.42 `continue`s before the probe, because the probe's empty answer on a name that git cannot give would read as a red (`_CRS3armcont` RED, `_CRS3armcont0` PASSED). The probe, the arm order it pinned and `_pbbad` are removed; the SYSTEM liveness probe stays for a name that answered.]

**Review 1:** 0 / 0 / 4 (wording; the empty `_lbd_files` reading the wire's stdin — fixed by running the grep only
over a non-empty list). The reviewer's observation (any status 129 from `git var` read as a machine limit) went to
S5b as its one code item. Coders: 2 (1 turn-limit handoff).

**Measured (both shells).** Ship PASSED. Kills 20 of 20. The P-b status record KILLED, and SURVIVES with the GLOBAL
`rc != 0` clause removed (`S5arecsurv`). P-b's output byte-identical to base (the coder: 6 cases; the
reviewer: 10 cases × 2 trees × 2 shells). Arms: the machine-limit branch, the 129 add, the GLOBAL scan, the SYSTEM
`rc == 0` check and the SYSTEM liveness check each change the verdict when removed; the `local` line removed stays
green (no claim of locality is written). Battery (130 cases, base `288c857c`): red→green 0, green→red 0, digests equal. After the review fix: ship
PASSED; `S5alfempty` rc 2 under `$_lbd_lbl` with stdin `/dev/null` and with stdin `_cen_lbl`; the status record
KILLED.

### R7.5 S5b-1 — `c8eb9ab4`, and S5b-2 — `0eee71c7`

**S5b-1, changed.** A name's status 129 is a machine limit only by a definition test (P-f's shape): the call exits
129, writes nothing on stdout, the first line of its stderr begins `usage: git var ` (the window runs `LC_ALL=C`),
and the same git answers `git var GIT_PAGER` with 0 (asked at most once per run, only if a name answered 129). Any
other 129 is red under `$_pb_lbl` (`<name>:rc!=0`); a bad 129 is tested before the machine-limit line, so a good
129 cannot hide it. Four records (stdout, a hangup line before git's usage, the control failing, git's top-level
usage); records 213 → 217.

**Superseded by CR-S3 (K2).** The definition test, the `GIT_PAGER` control (`_pbk`), `_pbe`, `_pbu`, `_pbx` and the
usage-text match are gone: the limit is keyed on `git --version` below 2.42 (all four names, one line), and a version that
does not parse counts as capable (red on a refusal). Its four records went with it (see CR-S3 below).

**Reviews.** Review 1: 0 / 1 / 4 — (I1) the design memo's P-b row still said a 129 is a machine limitation, false
after the slice. Review 2: 0 / 0 / 2. Driver design stops: 2 (a HUP-killed non-exec child makes bash print a
`Hangup` line, so "stderr non-empty" let it through → the first-line prefix; `usage: `-only is not equivalent, as
git's own top-level usage is a real 129 → record 245). Coders: 4 (1 turn-limit handoff).

**Measured (both shells).** Ship byte-identical to `b164f3ae`'s output (seconds aside). `S5bpbhup` (non-exec
HUP) green → red. The four records KILLED; each SURVIVES with its clause removed — the `-z "$_o"` test (`S5bsurvA`), the first-line prefix
test (`S5bsurvD`), the control test (`S5bsurvC`), the prefix narrowed to `usage: ` (`RVusageonly245`), and the whole
prefix clause (`S5bsurvE`): rc 0 PASSED with the machine-limit line. Battery (base `b164f3ae`, in three parts):
exactly 6 flips, all base green → new red and intended (`S5bpb129{mixed,nopager,nostderr,stdout}`, `S5bpbhup`,
`S5bpbhupexec`); b53 = b32 except one case per part whose CF differs by shell identically at base (`needunset`,
`R8nowd`).

**S5b-2.** Two comment lines (harness, ctlrun): `_fixture_did_fail` is not "the ONE reader of `$_FIX_FAILED`" (the
reader list measured by `/usr/bin/grep -n _FIX_FAILED`). The other items were already true or had no subject: the
controls file's awk recipe counts its `_control` call lines by expected exit (14 expecting 0, 57 expecting 1, 5
expecting 2: 76 lines) and its grep the 7 `_ctl_need=`-prefixed calls the recipe leaves out, as the comments say; one `SIZE RULE` (not three); no BWK `-v`-newline
claim exists on the tree or in its history; C6 (`_lp` re-run per mutant) closed — `_lp` went with the parser in
`d976f239`. No fresh reviewer. Ship PASSED, 22 s on each shell.

### R7.6 S2a — `8c952d6d`

**Changed (#2).** A run's environment travels with the call: `_ctl_run <for> <root> <cwd> <umask> <pfx> <scope>
<extra> [NAME=VALUE…]` (`_control` forwards `"${@:9}"`); each channel argument is validated (`NAME=…`) after the
gate registers, else `malformed channel argument`, return 1. `_ctl_run` makes `$_VFY/.reached`; an inline
`"$BASH" -c` recorder writes each delivered variable and the `umask`; after the timeout clause the evidence check
demands every delivered env channel back exactly and the umask as `%04o`, else NOT EXERCISED under the label. The
`_ctl_env` global, its standing check and `$_ce_lbl` are deleted. Records: the `_ctl_env=()` record removed; R1
(the routing purge disabled), R2 (`GIT_DIR` withheld before the recorder), R5 (the umask withheld at delivery, the
umask control's verdict flipped) added. `_MUT_UNRECORDED_MAX` 20 → 19 (routed gains a record);
records 217 → 216 → 219.

**Review 1:** 0 / 0 / 6 (comments: R1's survive direction; the RETURN CONTRACT; the PATH prefix not yet demanded;
a hazard premise; "three standing checks"; the removed-record list). The reviewer's real opt-in run, filtered to
R1 + R2 + R5 + `!survive`: 219 entries, 0 not killed; with the evidence check disabled R2 and R5 SURVIVED and R1 was
still killed by routed.

**Measured (both shells).** Ship rc 0 PASSED (21 s / 20 s). Malformed channel (`1BAD=x` on k2's call): rc 1, the one
line `malformed channel argument "1BAD=x"`, no run. R1 KILLED (`expected exit 1, got 0`), SURVIVES with routed's
channel arguments removed (`S2a2R1surv`); R2 KILLED (`GIT_DIR never reached the run`), SURVIVES with the evidence
check disabled (`S2a2R2surv`); R5 KILLED (`umask never reached the run`), SURVIVES with the check disabled
(`S2a2R5surv`). The 24 records anchored in lines S2a changed: 24 KILLED × 2 shells, 0 survived, 0 wrong-reason.
Battery (162 cases, base `0eee71c7`): digests b53 = b32, flips 0; 17 old cases EDIT FAILED on the new tree (anchors
changed; classified by S2b-2 below).

### R7.7 S2b — `fd90b4b1`

**Changed (#6).** The PATH-prefix channel is demanded too: each of the 11 asserted shim arms (9 fixtures shims,
relcwd's `fakerelmktemp`, cfgkept's `fakegitcfg` in its `ls-files` arm) writes `: > "${K2_CTL_REACHED:?}/path"`,
and `_ctl_run` demands `path` when the call's `$5` is non-empty. The relcwd and cfgkept marks, their checks and the
two prefix-drop records are gone; R3 (cfgkept's prefix withheld at delivery) and R4 (the wire's `_git` stepping past
a first `…/fakegitcfg` PATH entry) added. Records 219 → 217 → 219; `_MUT_UNRECORDED_MAX` 19.

**Review 1:** 0 / 0 / 3. The coder's opt-in "hang" was the generated set's duration, not a defect: the reviewer's
unfiltered run completed green on S2a and S2b (b32 957 s, b53 944 s); filtered to R3 + R4 + `!survive`: 219 entries,
0 not killed on both shells; with the evidence check disabled R3 and R4 SURVIVED.

**Measured.** Ship rc 0 PASSED on both shells, output identical to `8c952d6d`'s. Each of the 11 marks removed in
turn (bash 5 only): rc 1, `CONTROL NOT EXERCISED (<label>): PATH prefix never reached the run`. R3 and R4 KILLED on
both shells (R3 by the cfgkept line alone); R3 SURVIVES with the evidence check disabled (`S2b2R3surv`); R4 SURVIVES with
cfgkept's mark moved back to the shim's entry (`S2b2R4surv`), and with the evidence check disabled (the reviewer's
case). Records anchored in lines S2b changed: 0. Battery (164 cases, base `8c952d6d`): digests b53 = b32, no flip
among old cases; 18 EDIT FAILED on the new tree — 8 subject gone by design (the cfgkept `elif` and its record, the
`_ctl_env` global, the `_ce` check) and 10 needing a re-aim (6 anchored on `_MUT_UNRECORDED_MAX=20`; `R3declprenorm`,
`R3noverdictreg`, `S3umask8`, `S5anolocal`), not re-aimed in S2b (`R3noverdictreg` was later, as `hR3nvr`, R7.10).

### R7.8 S2c — `0d411df2`

**Changed.** The harness probes the grep the k2 run resolves (`_gopt_ok`, beside `_fifo_ok` and `_perm_ok`, over a
file of its own, called as the wire calls it): without `GREP_OPTIONS` rc 0 is required; with
`GREP_OPTIONS=--exclude=*` rc 1 → 1 (honoured), rc 0 → 0 (machine limit); anything else → a value the gate reds
under k2's label. k2 is `_ctl_need="$_gopt_ok" _control …`, and `_cn_declined` decides its summary line. One record
(k2's flag → 0, the fifotracked record's shape). Records 220; `_MUT_UNRECORDED_MAX` 19.

**Reviews.** Review 1: 0 / 1 / 4 — (I1) the S2c-2 change running probe call (a) in a subshell with `unset
GREP_OPTIONS` was dead code (the wire unsets it before the harness is sourced); it answered a "defect" the driver
had stated without measuring, and was reverted. Review 2: 0 / 1 / 1 — (I) a fix had edited the body of the #519
memo (`…-Ai-wire-k2-trip-wire.md`), whose banner says the body stays as written; reverted, with banner bullets and
reviews §8.2 rows instead. Review 3 (delta only): 0 / 0 / 2, fixed by the driver.

**Measured (both shells).** Ship rc 0 PASSED; `_gopt_ok` = 1 on this machine (call (a) rc 0, call (b) rc 1). Forced
arms on a clone: 0 → NOT EXERCISED, rc 0; `x` → red under k2's label; the probe file missing → red. The k2 record
KILLED where the flag is 1 (`S2c2gopt1kill`); with the flag forced 0 it SURVIVES, NOT EXERCISED (`S2c2gopt0surv`).
Battery (167 cases, base `fd90b4b1`): digests equal, no flip; EDIT FAILED 18, the same names as S2b.
**The full opt-in run** (`WEBREF_WIRE_MUTANTS=1`, no filter, both shells in parallel, ≈63 min each): rc 0 PASSED on
both; `mutation set: 220 entr(ies), 0 not killed as named, 0 not exercisable on this machine`; `generated boundary
set: 50 mutant(s) from $K2RE and $K2RE_PATH, 0 neither killed nor argued equivalent`. Its tree was `fd90b4b1` plus
the S2c-2 diff (`git diff | shasum` = `c6a12346…`), not `0d411df2`: measured for this file, the run's parts differ
from `0d411df2`'s in the harness (14 diff lines: call (a)'s subshell, the probe comment, the header line) and the
controls file (2 lines, one comment); ctlrun, fixtures, mutations, mutgen, records and the wire are identical. The
review fixes after it reverted call (a) and changed comments only. A full opt-in run on the S4 head, both shells,
was scheduled after the S4 commits (`DECISIONS-R7.md`, S4e note 1). It ran on a clean `git clone --local` at
`e0db887a`, which is T (`…-landing.md` §9.1) at this writing: §9.1's own T command, run on the worktree, prints
`e0db887a0ecbbda0ce790e0ce86c51b37719b24f` (S4e+S4f change docs only). The run is X3's first cell only (landing §11:
the `WEBREF_WIRE_MUTANTS=1` run, on both shells); X3's other cells are R7.11's. rc 0 PASSED on both
shells, 3661 s (bash 5) and 3660 s (bash 3.2); `mutation set: 220 entr(ies), 0 not killed as named, 0 not
exercisable on this machine`; `generated boundary set: 50 mutant(s) from $K2RE and $K2RE_PATH, 0 neither killed nor
argued equivalent`.

### R7.9 S4c-0 — `20615255`, S4a+S4b — `1fd70133`, S4d — `173b81dc`, S4c — `e0db887a`; S4e+S4f

**S4c-0.** The design memo's §4 from "P-g's population, and the census before any git" to its end (lines 619-845 at
`0d411df2`) moved unchanged to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-census.md` (252 lines; `cmp` of
its last 227 lines against `git show 0d411df2:<memo> | sed -n 619,845p` silent); the memo went 950 → 729 lines;
present-tense pointers updated (memo, residuals, controls :174, harness :170). The coder's record says "Wire
PASSED" without naming the shell; the driver's records 3.2 PASSED.

**S4 claim table** (designer, on `0d411df2`): 30 items — 21 false or changed, 2 needing a design decision (#10's
gate vs claim → option C; the §4 rows' shape → one table), 4 with no subject left, 3 already true. Premises it
corrected: the PR's base is `e8f78896` (stacked; 95 base records), so the PR's share is 125 of 220.

**S4a** (corpus §6.1, 15 rows for `2b56c33c`…`16efb423`, from the parsed record sets of each commit; three rows
withheld for want of a stated survive clause). **S4b** (18 rows for `d976f239`…`0d411df2`, the Current-totals
bullet rewritten: 125 / 220 at `0d411df2`, `_MUT_RECORDS_MIN=220` a floor, `_MUT_UNRECORDED_MAX=19` exact; of the
three withheld rows (P-b's status record, NOALT, `_fixture_failed fakerelmktemp`), the first two measured at
`20615255`, below, with the k2 `_gopt_ok` record, which was not withheld; `…-pr527-r15.md` :797 reworded).
Measured at `20615255` on a `--local` clone, both shells: ship PASSED; the P-b status record alone KILLED (`GIT_CONFIG_GLOBAL:rc!=0
GIT_ATTR_GLOBAL:rc!=0`), SURVIVES with the GLOBAL arm's red removed (`S4bPbsurv`); the NOALT record alone KILLED
(`NOT EXERCISED (this git reports alternate object stores)`), SURVIVES with the liveness check's no-line clause
removed (`S4bNoaltsurv`); the k2 record alone KILLED, SURVIVES with `_cn_declined`'s red arm removed (`S4bGoptsurv`,
NE 1). The `fakerelmktemp` row says its survive clause was never stated or measured.

**The S4ab review's re-measurement** ("S4ab" = the S4a+S4b pair; `R7-S4ab-review.md`; S4 decisions, judgment 2), at `20615255`, both shells:
the S1'-era survive citations hold — `svR2`, `svR5`, `svR6`, `svR7`, `svR8` and `survB_near` as they are,
`survA_skipctl` and `R3noverdictreg` only after a re-aim (`hsurvA`, `hR3nvr`). The `2b56c33c` fixture-failure rows'
clause (a block's `_ctl_gate` call) no longer exists (I1; re-measured as `hFFrelcwd`, `hFFfsmon`, `hFFclean`), and the
`2d5c393f` row's citation predated its re-shape (M4; `hPk2`). The fix round and the second review's N3 re-cite the
`2b56c33c`, `2d5c393f` and `d976f239` site-rule rows to those cases, dated `20615255` (R7.10); `S3cfgnoelif330` is
dated "measured at `288c857c`, its subject went in `fd90b4b1`".

**S4c** (design memo, census, and comments in fixtures/ctlrun/controls): #10 by option C — the `routeddecoy`
fixture leaves the fixtures init loop and is built whole and bare on its own line after its content (`git init -q .`,
`git add -A`, no `||`: errexit in the window, as the window's other bare builds), so it has no failure record and a
failed init or add ends the window (W); on success the decoy repo is byte-identical (the commit message: "measured");
six parent-side rows (G R F L O WM) in the memo's §4 table under a reworded head; the other S4 sentences. Measured,
both shells: ship rc 0 PASSED; plant A (the decoy's add → `false`) and plant B (its init → `false`): rc 2, the W line
(`the window exited 1 before completing … nothing was built, so no control was run`); the first pass's plant (the
decoy build → `false`) at `20615255`: rc 0 PASSED (final-r6 #10 reproduced).
**S4d**: the #519 memo's banner (one hunk), reviews §8.2 rows (13 in all), landing X10's part list; the
`_MUT_TARGETS` gains checked against `7f4259f9`, `96eea58c`, `16efb423`, `d976f239`. No runs. **S4e**: this file's
R7. **S4f**: R3–R6, recorded ahead of R7 by the same §13 rule (S4e note 4).

**Reviews.** S4a+S4b (`R7-S4ab-review.md`): 0 / 1 / 4 — (I1) the `2b56c33c` fixture-failure rows cited a survive
clause that no longer exists (above). Second review: 0 / 1 / 2 — (N1, IMPORTANT) an unescaped `||` in a table code
span split the `2d5c393f` row, escaped by the driver; N2, `hsurvA.py`'s comment; N3, above. Not re-reviewed (the
IMPORTANT was a one-token escape, verified by count). S4c-0+S4c (`R7-S4c-review.md`): 0 / 2 / 4 — (I-1) the first
pass left `routeddecoy` in the init loop, so its failure record was still written (via `$d`) and read by nobody (the
loop planted to skip it: rc 0 PASSED, both shells); (I-2) the memo said every PATH-prefix shim directory is built
bare and ends the window, but the controls file's two `$_VFY` ones (`fakerelmktemp`, `fakegitcfg`) are built in the
parent, where a failure ends the wire unlabelled. Second review: 0 / 0 / 3, applied by the driver; ship PASSED on both
shells. S4d+S4e+S4f (`R7-S4def-review.md`): 0 / 2 / 5 — (I-1) R7.10 lacked the S4ab cases; (I-2) sentences here the
S4ab fix round had made false; both fixed in this file. Its delta review: 0 / 1 / 5 — (I-1) R7.8 and R7.11 had the run at `e0db887a` as the
final head's, where landing §9.1's subject is T; M-1…M-5, a re-aim list, a kill-alone source, the S4ab case count,
a missing final-r6 number and the watchdog convention's exception; all fixed in this file, the driver reading the
diff.

### R7.10 The runner cases corpus §6.1 cites

Corpus §6.1 names these runner cases in its `2b56c33c`…`0d411df2` rows. Each lives in the handoff directory as
`runners/cases/<name>.py`, or, for the 8 of the S4ab review's 12 cases that corpus §6.1 cites (the last eight
below), `runners/cases-s4ab/<name>.py` (its edit, applied
by `_ed.py`: `ed(part, old, new)` asserts `old` occurs exactly once; `ed1` tries anchor pairs in turn, the first that
occurs once), and, for all 30 below, a `<name>.m99` marker: every one runs as a `.mutant.99` copy set. Each is a
survive check: one guard removed plus one record's edit; "PASSED" = rc 0, i.e. the record would SURVIVE with that
guard gone. The record alone is KILLED: for a slice's case, its kill row is in that slice's entry above; for the
S4ab review's cases, in the case's own entry below (`hFF…`, `hPk2`). For `hR3nvr`, `hsurvA`, `hsurvB` and `hsvR8`
the record alone at `20615255` is not recorded (neither `R7-S4ab-review.md` nor PROGRESS's S4 sections state it).
Results are both shells, identical; "→" separates the anchor from its replacement.

- **`S12noend`** (S1', the late call). mutations: `_mut_census_end() {` gains `return 0` as its first statement.
  controls: the `_mut_run` line → `_mut_run; ( _control "$CTL/clean" 0 "PASSED" "a late label" ) || ctl_ok=1`.
  PASSED (S1' review 2's arm-mutant table; S1' follow-up, both shells).
- **`S15noendline`** (S1'). controls: the line `_mut_census_end || exit 1` removed; the same `_mut_run` edit as
  `S12noend`. PASSED (S1' follow-up).
- **`S10nokind`** (S1', the kind rule). ctlrun: the line `case " ${FUNCNAME[*]} " in *" _control "*)
  _rg_k=control ;; esac` deleted. controls: the `green is reachable` `_control` line gains `; _control "$CTL/clean"
  0 "PASSED" "$_mg_lbl" || ctl_ok=1`. PASSED (the arm-mutant tables of S1' reviews 2 and 3).
- **`S11nokindrule`** (S1'). mutations: the census awk's `if ($2 == "control" && (l in isv) && !(l in lit)) {` →
  `if (0) {`; the same controls edit as `S10nokind`. PASSED (the same two tables).
- **`S13nogrowth`** (S1', `_ctl_need`). controls: `[ "$_cd_new" -gt 0 ] || return 1` → `return 1`;
  `_ctl_need="$_fifo_ok" _control "$CTL/fifotracked"` → `_ctl_need=0 _control "$CTL/fifotracked"`. PASSED
  (the same two tables).
- **`S14nored`** (S1'). controls: `_cn_declined`'s `if [ "$1" -eq 1 ]; then` before its `CONTROL FAILED ($_cn_lbl)`
  echo → `if false; then`; the same fifotracked edit. PASSED, with the fifo summary NOT EXERCISED (NE 1) (the
  same two tables).
- **`S16nolastask`** (S1', the last ask). controls: the line `_cn_declined 1 "after the last capability group's ask,
  so no summary line was decided from them" || :` → `:`; `_mut_correspondence || ctl_ok=1` →
  `_ctl_need=0 _ctl_run "$_mg_lbl" "$CTL/clean" || :; _mut_correspondence || ctl_ok=1`. PASSED (S1' review 3's arm-mutant table).
- **`R3noverdictreg`** (S1', the site rule's second verdict). harness: the window verdict's `  _ctl_reg "$1"`
  (before `if [ "$_fw_done" -ne 1 ]`) → `  :`. controls: the comment line matching `^# STANDING CHECK of .*one owner
  of .*: a run the gate$` → `_fgit_window_verdict_exit "$_fw_lbl"`. PASSED (S1' review 3 battery; S3 battery, at
  `d976f239` too). ⚠ From S2a's tree on, EDIT FAILED: that comment went with the `_ce` check (S2a's progress row
  says "bare AssertionError, ctlrun edit"; in the case's text the one bare `assert` is the controls comment's — the
  `ed` asserts carry a message). S2b-2 classed it "needs a re-aim"; re-aimed as `hR3nvr` (below), which the corpus cites.
- **`S3cfgnoelif330`** (S3, cfgkept). controls: `elif [ ! -e "$_cfg_mark" ]; then` → `elif false; then`;
  ` "$_VFY/fakegitcfg"; then ctl_ok=1` → ` ""; then ctl_ok=1` (the prefix-drop record). PASSED (S3's arm-mutant table).
  EDIT FAILED from S2a's tree; subject gone by design at `fd90b4b1` (the mark check and the record removed).
- **`S5arecsurv`** (S5a, the P-b status record). harness, the first of three anchor shapes that occurs once: base's
  `_pbr`/`_pbo` line plus `if [ "${!_pbr}" -ne 0 ]; then _pbd="$_pbd $_v:rc!=0"; continue; fi` → the first line
  alone; S5a's `if [ "$_prc" -ne 0 ]; then _pbd="$_pbd $_v:rc!=0"; continue; fi` → nothing; S5b-1's four-line
  `if [ "$_prc" -ne 0 ]; then` block with its `_pbbad`/129 test and `continue` → nothing. Then base's
  `printf -v "_pbr_$_v" '%s' "$_prc"` → `… 1`, or `_prc=0; _o="$(_fgit_in` → `_prc=1; _o="$(_fgit_in`. PASSED (S5a,
  on base and new; re-run after S5b-1's re-aim, new = base).
- **`S5bsurvA`** (S5b-1, the stdout clause). harness: after
  `_prc=0; _o="$(_fgit_in "$_pq/a" var "$_v" 2>"$_pq/var.err")" || _prc=$?` append
  `; echo "usage: git var x" > "$_pq/var.err"; _prc=129` (stdout left as the call wrote it);
  `if [ -z "$_o" ] && [ "$_pbe" = usage ] && [ "$_pbk" = yes ]; then` → the same without `[ -z "$_o" ] &&`.
  PASSED with the machine-limit line, NE 1.
- **`S5bsurvC`** (S5b-1, the control clause). harness: the same status line gains `; _o=""; echo "usage: git var
  x" > "$_pq/var.err"; _prc=129`; the definition `if` loses `&& [ "$_pbk" = yes ]`; `if _fgit_in "$_pq/a" var
  GIT_PAGER >` → `… var NOSUCH_K2 >` (the control fails). PASSED, NE 1.
- **`S5bsurvD`** (S5b-1, the first-line prefix). harness: the status line gains `; _o=""; { echo "Hangup: 1"; echo
  "usage: git var x"; } > "$_pq/var.err"; _prc=129` (record 243's two-line injection); the definition `if` loses
  `&& [ "$_pbe" = usage ]`. PASSED, NE 1 (S5b-1; again after review 1 and after record 245).
- **`RVusageonly245`** (S5b-1, `usage: `-only). harness: the status line gains `; _o=""; echo "usage: git [-v |
  --version] [-h | --help]" > "$_pq/var.err"; _prc=129` (record 245's injection); `case "$_pbe" in "usage: git var
  "*)` → `case "$_pbe" in "usage: "*)`. PASSED, NE 1 (its `.m99` marker added at the commit, re-run then).
- **`S2a2R1surv`** (S2a, R1). wire: `*) unset "$_v" ;; esac` → `*) : ;; esac`. controls: routed's
  `"exported GIT_DIR cannot redirect the scan" "" "" "" "" \` continuation with `"GIT_DIR=$CTL/routeddecoy/.git"
  "GIT_WORK_TREE=$CTL/routeddecoy" || ctl_ok=1` → `"exported GIT_DIR cannot redirect the scan" || ctl_ok=1`.
  rc 0 PASSED.
- **`S2a2R2surv`** (S2a, R2). ctlrun: `K2_CTL_REACHED="$_r_reach" "$BASH" -c` → `K2_CTL_REACHED="$_r_reach" env -u
  GIT_DIR "$BASH" -c`; `  if [ -n "$_r_miss" ]; then` → `  if false; then`. rc 0 PASSED.
- **`S2a2R5surv`** (S2a, R5). ctlrun: `    [ -z "$_r_umask" ] || umask "$_r_umask"` → `    :`;
  `  [ "$_cr" -eq 0 ] || return 1` gains `; case "$4" in "a restrictive umask decides nothing") set -- "$1" 0
  "PASSED" "$4" ;; esac`; the evidence check `if` → `if false`. rc 0 PASSED.
- **`S2b2R3surv`** (S2b, R3). ctlrun: `    export PATH="${_r_pfx:+$_r_pfx:}$PATH"` is preceded by
  `case "$_r_pfx" in */fakegitcfg) _r_pfx= ;; esac; `; the evidence check `if` → `if false`. rc 0 PASSED.
- **`S2b2R4surv`** (S2b, R4). wire: `_git() { ( for _v in $_GIT_LOCAL_VARS; do` → `_git() { ( case "${PATH%%:*}"
  in */fakegitcfg) PATH="${PATH#*:}" ;; esac; for _v in $_GIT_LOCAL_VARS; do`. controls (cfgkept's shim text):
  `case " $* " in *" ls-files "*) : > "${K2_CTL_REACHED:?}/path"; if [ -z` → `: > "${K2_CTL_REACHED:?}/path"; case
  " $* " in *" ls-files "*) if [ -z` (the mark moved to the shim's entry). rc 0 PASSED.
- **`S4bPbsurv`** (S4b, at `20615255`). harness: the line `        if [ "$_pbbad" -eq 1 ] || [ "$_prc" -ne 129 ];
  then _pbd="$_pbd $_v:rc!=0"; fi` deleted (its `continue` kept); `_prc=0; _o="$(_fgit_in` → `_prc=1; _o="$(_fgit_in`.
  rc 0 PASSED.
- **`S4bNoaltsurv`** (S4b, at `20615255`). harness: ` || [ -z "$_fgit_alt" ] || ` → ` || ` (the liveness check's
  no-line clause); `alternate:*) _fgit_alt=` → `NOALT:*) _fgit_alt=`. rc 0 PASSED.
- **`S4bGoptsurv`** (S4b, at `20615255`). controls: `_cn_declined`'s red-arm `if` → `if false` (as in `S14nored`);
  `_ctl_need="$_gopt_ok" _control "$CTL/k2"` → `_ctl_need=0 _control "$CTL/k2"`. rc 0 PASSED, k2 NOT EXERCISED
  (NE 1).
- **`hFFrelcwd`, `hFFfsmon`, `hFFclean`** (the S4ab review's I1: the `2b56c33c` fixture-failure records). ctlrun:
  `_ctl_built`'s `    if _fixture_did_fail "$_b_n"; then` → `    if false; then`. fixtures: the line
  `: > "$_FW_DIR/built"` gains a leading `_fixture_failed relcwd; ` (`fsmon`, `clean` in the other two). rc 0 PASSED;
  each record alone KILLED.
- **`hPk2`** (M4: the `2d5c393f` second-store record). harness: ` || [ "$_fgit_alt" != "${_fgit_alt%%$'\n'*}" ];
  then _pkl=bad` → `; then _pkl=bad` (P-k liveness's more-than-one-line clause); then the expression of records.sh
  line 321 (that record, `harness:`) is applied to the harness by `sed -e`, the case asserting exactly one line
  changes. rc 0 PASSED; the record alone KILLED on both shells (`R7-S4ab-review.md` M4: "Record :321 alone is
  KILLED on both shells"; corpus §6.1's `2d5c393f` second-store row: "the record alone KILLED").
- **`hR3nvr`** (`R3noverdictreg` re-aimed). harness: the same `  _ctl_reg "$1"` → `  :`. controls: the one line
  matching `^# STANDING CHECK of .*timeout clause, both ways\. A status >= 128 is a$` →
  `_fgit_window_verdict_exit "$_fw_lbl"` (the second-verdict record's current shape). rc 0 PASSED.
- **`hsurvA`** (`survA_skipctl` re-aimed). ctlrun: the site walk `  while [ "$_rg_i" -gt 1 ] && [
  "${BASH_SOURCE[$_rg_i]}" != "$_CONTROLS" ]; do _rg_i=$((_rg_i - 1)); done` → `  _rg_i=1; while [
  "${BASH_SOURCE[$_rg_i]}" = "${BASH_SOURCE[0]}" ]; do _rg_i=$((_rg_i + 1)); done` (it skips only ctlrun's own
  frames); controls: the same edit as `hR3nvr`. rc 0 PASSED.
- **`hsurvB`** (`survB_near` at `20615255`). ctlrun: the same walk → `  _rg_i=1; while [ "${BASH_SOURCE[$_rg_i]}" !=
  "$_CONTROLS" ]; do _rg_i=$((_rg_i + 1)); done` (the nearest controls-file frame). controls: the `green is
  reachable` `_control` line → `_rh() { _control "$CTL/clean" 0 "PASSED" "green is reachable"; }; _rh || ctl_ok=1`,
  and `_fsm_root="$CTL/fsmon"` gains `; _rh || ctl_ok=1` (the helper record's two expressions). rc 0 PASSED.
- **`hsvR8`** (`svR8` at `20615255`: the site rule's `bad = 1`). mutations: the `        bad = 1 }` after `would pass
  under that record." > "/dev/stderr"` → `        }`. controls: `_mut_correspondence || ctl_ok=1` is preceded by
  `_control "$CTL/clean" 0 "PASSED" "an IGNORED generated artefact does not fire" || ctl_ok=1; `. rc 0 PASSED.

Where measured: the S1' cases on the S1' working tree over `16efb423` (reviews 2–4) and in the S3/S5 batteries; the
S3, S5 and S2 cases on their slice's final tree; the S4b cases and the 8 of the S4ab review's 12 cases that corpus
§6.1 cites on a clone of `20615255`.
Corpus §6.1's `d976f239` site-rule row cites `hsvR8`, `hsurvA`, `hR3nvr` and `hsurvB`, dated `20615255`; its row for
the census's "more"/"fewer" readings and orphan arm still states its survive measurements by what was removed, naming
no case, dated to the S1' first draft: those ran as survive trees (`svR2`, `svR5`, `svR6`, `svR7`, on `e585daaf` +
the design prototype; R7.2), not as case files; at `20615255` the S4ab review re-measured them as case files
(`hsvR2`, `hsvR5`, `hsvR6`, `hsvR7` in `runners/cases-s4ab/`, which the row does not cite), and they hold as they
are.

### R7.11 Carried out of R7

- `…-pr527-r15.md` §Q's opening sentence ("Every result from plan-review round 15 on goes here") predates this
  rollover; the pointer at §Q's end is the rollover's statement (design memo §13).
- Of the 10 battery cases S2b-2 classed "needs a re-aim" (R7.7), `R3noverdictreg` was re-aimed for the corpus as
  `hR3nvr` (R7.10); the other nine were not.
- The full opt-in run at T (`e0db887a`, R7.8) is X3's first cell only. X3 (`…-landing.md` §11) also holds corpus
  §6.1's survive check, the plant cell and the floor cell, and §9.1 step 2 records the results in the open
  implementation record (design memo §13); none of these is done, so X3 is not "already recorded for this T", and §9.1 step 1 still runs X3 at T,
  with X2, X5, X6, X8, X11, X13, X14, X15 and X16. A later fix that changes the tool code or the `trip-wires` job
  moves T, and the run at `e0db887a` is then no cell of X3 at the new T. The final-head sequence and the branch-wide
  `/code-review max 815f8d4b...` that DECISIONS-R7 schedules after S4: not yet run.

### R7.12 CR-S3 — K2 (cluster D), with CL2/SI4

**Changed.** P-b's machine limit is keyed on `git --version` below 2.42 (all four names, one line); from 2.42 on a 129
or any non-0 GLOBAL call is red (a SYSTEM name's silent failure is the hermetic answer, judged by the liveness probe). A
version string that does not parse counts as capable (red on a refusal, never a green limit line). The limit line is
printed after the reds and never outranks one. Removed: `_pbk` (the `GIT_PAGER` control), `_pbe`, `_pbu`, `_pbx`, the
usage-text match and `_pbbad` (CL2/SI4, folded at no cost). P-f keeps its definition test (`env` has no version
contract): a decision recorded in the comment and the design memo's P-b row. Records: the four definition-test records
(P-b 129 variants) removed with the test; three added (`_prc=129` forced, `var --`, a misspelt name). 224 → 223.

**CL2's pin.** The "discarded" SYSTEM probe pinned only the ORDER of the old arm (limit line before `_pbl`). With the
limit line last, a 129 on an old git must `continue` before the probe (its empty answer would read as red):
`_CRS3armcont` (forced old, SYSTEM 129, probe empty, `continue` removed) → NE under `_pbl_lbl`; `_CRS3armcont0` (same, kept) PASSED.

**Measured (both shells).** Ship PASSED. `var --` and the misspelt-name records KILLED (git 2.56.0); with the version test
forced old (`_CRS3armvar`, `_CRS3armtypo`) both SURVIVE with the limit line. Claim rows by a `git` wrapper on PATH:
`2.41.0` (var answering, and var refusing 129) and `2.39.3 (Apple Git-146)` refusing → one limit line, green;
`2.43.0 (Apple Git-1)` refusing, `garbage` and `2.` refusing → red; `2.41.0` plus a HOME leak (`_CRS3home`) → red.
Each parse guard, refusing: red with it, one green limit line with it removed — the outer prefix by a bare `2.41.0`,
the inner `*:` by `git version 1.x.5`, the inner `*[!0-9:]*` by `git version 1 .9` (`[` takes `"1 "` as 1). `:*` was unreachable: deleted.

### R7.13 CR-S5 — K15

**Changed.** `_mut_trial` (mutations.sh) counts a record killed only where its needle stands in a diagnostic's label
slot: `CONTROL FAILED (<needle>)` (the `!kill` arm's form) or `CONTROL NOT EXERCISED (<needle>)`. Before, the needle
anywhere in the output counted, and `_k2_line` prints k2's label on the green summary: record 100, `_gopt_ok` 0 and any
unrelated late red then "killed as named" with no check under that label failed. The head comment of `_mut_trial`,
the records-format header, `_limit_lines` (controls.sh) and the corpus's notes state it. mutations.sh 778 → 789; 223 records.

**Premise check (static).** 223 records, 93 distinct needles: 92 full labels (each a quoted `_control` argument or a
`_*_lbl` value) plus `!survive` (its own arm, before the slot case); generated mutants use `!kill`. Every needle has a
red path that prints it in a slot. Other printings outside a slot (census awk, `.bare`, the late-calls tail,
`_cn_declined`, the ctlrun `_out` dumps) sit on red paths under another label's slot; `_k2_line` is the one printed on
a green line. The full opt-in (all 223 plus the generated set) is the driver's run.

**Measured (bash 5.3 and 3.2; real `_mut_trial` body, `_crs5ftrial.sh`; logs `runners/logs/crs5f-*`).** K15 repro
`_CRS5k15` (record 100, flag forced 0, a planted late red): rc 2 WRONG REASON; `_CRS5k15arm` (substring restored): rc 0.
Arm rows, one per disjunct, both shells (guard intact: all "killed as named"): arm A (`NOT EXERCISED` disjunct
removed): records 174, 288 (killed only through NOT EXERCISED) rc 2 WRONG REASON, 100 and 353 still rc 0; arm B
(`FAILED` disjunct removed): records 100, 353 (FAILED-killed) rc 2, 174 and 288 still rc 0. Ship PASSED;
`scripts/trip-wires.sh` rc 0. 31 records through `rec.sh` (`_crs5queue.sh`): all KILLED, every needle in a slot.

This file closed at 799 lines (`wc -l`); every later entry, and the code review's first-half results this file lacks, are in `…-impl-cr.md` §S (design memo §13).
