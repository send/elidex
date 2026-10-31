# K2 fixture git — PR #527's final-head sequence, step 1 narrowed to the instruments that exist (plan delta, option B)

**Status: accepted** (draft r6, by `/elidex-plan-review` rounds R1–R3, R3's focused re-check after r5 and its narrow re-check after r6, 2026-10-24). r6 applies the focused re-check
after r5 (`cr2/PLAN-REVIEW-FH-R3.md`, its last section: IMPORTANT 2 / MINOR 4): landing :339–:344's new text takes
§1.3's scope and keeps X8's exception; the new slot is #527's second own deferral, so landing :271 and residuals
:90–:92 are restated (§5); L3's trigger names the §6.1 records of its clauses (§8); a stopped run's logs and clones
are handled (runbook §6.0, §6.1). r5 applied plan-review R3's decisions
(the handoff's `cr2/PLAN-REVIEW-FH-R3.md`: CRITICAL 0 / IMPORTANT 3 / MINOR 18): §1.3's one stop rule now also covers
a docs-only fix that changes a RUN cell's definition (IMP-A); L1–L5 get owners through two defer slots (IMP-B, the PM
session's decision); the runbook gets a status line and a §0 and is counted in the umbrella (IMP-C). r4 applied R2's
(`cr2/PLAN-REVIEW-FH-R2.md`), above all R2-A: the narrowing holds for T = `01fb2924` only, and a move of T stops the
sequence for the user (§1.3); r3 applied R1's, r2 the driver's decisions on r1's open questions. It
changes surfaces that the design memo's plan reviews ratified (landing §9.1 and its steps 1–4 and terminator, land
order steps 7.4 and 9, the ledger text of §9, and the §11 rows X2, X3, X5, X14, X16; a dated note in `…-cr2-plan.md`'s
banner), so it is reviewed as a design
change before step 1 runs (`feedback_plan-ratified-surface-is-a-design-change`). On acceptance, step 2's commit moves it into
`docs/plans/` as `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md`, with its runbook (§6, moved out at r4) as
`…-cr2-plan-finalhead-runbook.md` and its checker as the companion
`2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-check.sh` (§9), as `R29-PLAN-DELTA-2.md` was moved in as
`…-cr2-plan-liveness-2.md`.

**Names.** As delta 2: design memo = `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md`; landing, corpus,
residuals, provenance (PROV), census, umbrella = `…-landing.md`, `…-corpus.md`, `…-residuals.md`, `…-provenance.md`,
`…-census.md`, `docs/plans/2026-07-citation-hygiene-umbrella.md`; impl-d = `…-impl-cr2-d.md`; PR527, R15 =
`…-pr527.md`, `…-pr527-r15.md`. `W` = the worktree `/Users/kazuaki/repos/send.sh/elidex-wt-fgit-hermetic` (branch
`k2-wire-fgit-hermetic`, head `3b7ff6d5`, read only); `H` = the handoff `/Users/kazuaki/repos/send.sh/elidex-fgit-handoff`.
**Every doc line cite is at `3b7ff6d5`, the frame. A tool cite "fixtures :N" (wire, controls, mutations, mutgen,
records alike) is at T = `01fb2924`**, whose tool code `3b7ff6d5` equals (§7 C1). `PROV@ff1322bb` and `PROV@453b7b0f`
= `git show <commit>:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md`.
**Evidence** for §1, runbook §6, §7: the checker `H/cr2/logs/fhd/check.sh` (static reads with `git show`; runbook §6.0's
definitions file, its frame guard, its extraction and patch block, the gate's negative control, §6.3's insert block
and §6.5–§6.6's build and generation lines taken from the runbook's text and run on scratch copies; the umbrella's
preflight count simulated on a scratch clone; it runs no wire) and its output `check.out`,
which it regenerates; the runbook a verifier built, `H/cr2/FINALHEAD-RUNBOOK.md` (its ambiguities A1–A14
are the questions this memo answers, §4).

---

## §0 What changed, and what this memo reviews

Landing §9.1 step 1 (landing :319–:320) runs, at T, X2, X3 (with corpus §6.1's survive check), X5, X6, X8, X11, X13,
X14, X15 and X16. The runbook (2026-10-24) found that several of them cannot run with what exists: the P-dump hook
`K2_CORPUS_PDUMP` that "P equal" reads is unreachable and five of its facts are recorded nowhere (landing :450–:491);
the hostile files under `corpus/h/` that `gen6.py` reads are recorded nowhere; the survive check's population is not
defined and it has no runner; X14's three setups are prose; X16's shim cells start with a clause list (`mkctl`) that is
not delimited. Several others run only after a stated re-anchoring (X16's tree edits, X3's plant line) or need a bound
the row does not state (X3: the runbook's A1).

**The user's decision (2026-10-24): option B.** Narrow step 1 to what runs with the instruments that exist, and record
the rest as stated defects. **Why** (the driver's rationale, recorded as given): code regressions at T are already
covered by the battery, 88 of 88 cases equal to base on both shells (`H/runners/logs/dg-r29e-{b53,b32}.txt`, 88 lines
each, 88 `new` = `base`, §7 C10), and by ship ×4 and `scripts/trip-wires.sh` at `9588df28`, whose tool code is T's
(`H/cr2/logs/cr2m-ship/ship-{b53,b32}-{bsd,gnulike}.log`, `trip-wires.log`, all `rc=0`, dated 10-24 01:45;
`H/DECISIONS-R7.md` :930). Rebuilding the lost instruments means writing new tool code (a hook in the harness, cell
scripts, fixtures), which becomes its own review subject and re-opens the self-introduced-findings loop that #527 has
hit round after round.

**The decision rule (the driver's; applied here, not re-decided).** A cell **RUNS** if it can run with an existing
script, or one extracted verbatim from the docs, plus at most a re-anchoring of a documented insert or edit to T's
lines (re-anchoring = the same clause at a new line or value, stated explicitly). A cell is **RETIRED** if it needs a
new or reconstructed instrument: a hook, a script with no verbatim copy, an undefined population, a setup that exists
only as prose. Glue that only creates directories, clones and logs, or bounds and cleans up a run, is not an
instrument (runbook §6.0).

This memo applies the rule cell by cell (§1), writes the runbook for every RUN cell (runbook §6), states every RETIRED cell as
a defect and records it (§8: in the open implementation record's new §T.L, and in place in each retired §11 row; not
as a residual, §4 Q4; its trigger tracked by a defer slot, #527's second own deferral), states that all of this holds for T = `01fb2924`
and the RUN cells' definitions at `3b7ff6d5` only, so that a move of T or a changed RUN definition stops the sequence
for the user (§1.3), and lists the edits step 2's commit makes (§5, §9). It does not run anything.

---

## §1 The rule, applied

### §1.1 Per check

| check | cells (runs = cells × shells) | decision | why (the rule's clause) |
|---|---|---|---|
| X2 | corpus §6 REF + 11 G × 4 configs = 48 runs | **RETIRED** (L1) | needs the hook (P equal), the hostile files `corpus/h/` (recorded nowhere), and a parts list for `gen6.py` (5 of 9, no documented re-anchor) |
| X3 main | 1 × 2 shells = 2 | **RUN** | the row's own command; the bound is "Cell runners at T"'s form (§1.2 D-F2) |
| X3 plant | 1 × 2 = 2 | **RUN** | the row's insert, re-anchored: before fixtures :705 (s8b's :698 at `21526e80`) |
| X3 floor | 1 × 2 = 2 | **RUN** | the row's edit, at mutgen :304 (the same line as at `21526e80`) |
| X3 survive check | "each §6.1 record": 305 / 211 / 203 / 129 / 130 by five readings | **RETIRED** (L2) | undefined population, no runner; it does not split (§1.2 D-F8) |
| X5 corpus §6 R, RLOUD, RES, P | 128 runs (4 REF shared with X2) | **RETIRED** (L3) | R, RES: hook + `corpus/h/`; RLOUD: `corpus/h/home_ign`; P: 5 of 9 edit anchors absent at T, no documented re-anchor; all: `gen6.py`'s parts list (§1.2 D-F1) |
| X5 draft-10 cells | 38 AFTER jobs (`gen11.py` at T, §7 C17) | **RUN** 32, **RETIRED** 6 (L3) | 32 judged without P equal and needing no file under `corpus/h/`; ROg ×2 and G ×4 need the hook, G also `corpus/h/` (§1.2 D-F6) |
| X5 drafts 11–21's cells | the six corpus §6.1 names (§7 C16) | `xrace` **RUN** (1 × 2 = 2); `lnblind2`, `xbare`, `xanchor`, `xexecarith` run by X16; `Jm` **RETIRED** (L3; 2 trees × 2 = 4) | `xrace`: a verbatim insert, `cell15.sh`; `Jm`: its second edit is prose (§1.2 D-F7) |
| X6 | 2 inserts × 2 = 4 | **RUN** | inserts verbatim (landing :397); runner `cell15.sh` |
| X8 | 3 base + 3 branch = 6 | **RUN** | the row's command; `bash` = the one `PATH` resolves (§1.2 D-F3) |
| X11 | 1 × 2 = 2 | **RUN** | lines verbatim (landing :401), after controls :123 (§1.2 D-F4) |
| X13 | 9 cells × NOU {unset, 1} × 2 = 36 | **RUN** | `crc.sh` verbatim (PR527 :333–:396) + its documented patch (landing :536–:551); every cell anchor present at T (§7 C5) |
| X14 | PX1, R23, R24 × 2 = 6 | **RETIRED** (L4) | setups prose only |
| X15 | 1 × 2 = 2 | **RUN** | insert verbatim (landing :406); runner `cell15.sh` |
| X16 | 23 cells: 20 × 2 = 40 runs | **RUN** (20 cells), three re-anchored (§1.2 D-F5) | inserts verbatim; tree edits re-anchored to T |
| X16 shim cells | `shimprepend`, `shimafter`, `shimhash` × 2 = 6 | **RETIRED** (L5) | each starts with `mkctl`, a setup stated only as prose ("`lnblind2`'s `zzy` link plus …", R15 :192–:195) |

**RUN: 130 runs** (X3 6, X5 34 = 32 + 2, X6 4, X8 6, X11 2, X13 36, X15 2, X16 40). **RETIRED, enumerated: 198 runs**
(X2 48; X5 128 + 6 + 4; X14 6; X16 6; the 4 REF runs X2 and X5 share counted in each) **plus one unenumerated set**,
X3's survive check. §7 C3, C4. L1–L5 are the five entries of impl-d's new §T.L, one per retired check (§8).

### §1.2 Where a check splits, or the rule needed a reading

- **D-F1, X5's corpus §6 rows retire whole.** The driver expected X5's RLOUD rows (red, no P compare) might run. They
  do not: RLOUD is a `gen6.py` kind (`PROV@ff1322bb` :743–:826), its one cell sets `HOME=corpus/h/home_ign`, a file
  recorded nowhere, and `gen6.py` must also be given T's nine parts (it lists five; landing's re-anchor, :503, is for
  `gen11.py`, not `gen6.py`) and a `p5d/` tree (it reads one unconditionally at its end). The P rows' verdict needs no
  P compare, but 5 of their 9 edit anchors are absent from T's harness (§7 C6), and re-anchoring them is not moving a
  clause to a new line: the clauses were rewritten (P-a's probe, P-b's probe, P-d's allowlist entry, P-e, the window's
  source line).
- **D-F2, X3's bound (A1).** The row (landing :393) gives the command with HUP, INT and TERM at their default and no
  bound; "Cell runners at T" (landing :423–:441) gives every listed runner an alarm and a `kill -9 -<pgid>` watchdog
  3 s after it, X3's survive-check runner with the signal defaults "before its `setpgrp`" (:430–:431). The three X3
  runs take that runner's form with a new value: `perl -e '$SIG{INT}="DEFAULT"; $SIG{TERM}="DEFAULT";
  $SIG{HUP}="DEFAULT"; setpgrp; alarm 18000; exec @ARGV'` and a watchdog at 18003 s that kills the run's process
  group. **The value:** measured at `21526e80`, two runs at a time, main 4274 / 4274 s, plant 4174 / 4173 s, floor
  3290 / 3289 s (b53 / b32; `H/runners/logs/s8b-x3/*.start,*.end`, §7 C8), for 251 records. At T there are 305
  (×1.22), and a ship run takes 35 s against ≈ 20–21 s then (`…-impl-cr2.md` :343, CR7-T's base, two at a time;
  impl-d :275, R29-E's two lanes): ×1.71. Scaled, ≈ 8.9 ks per pair (an **estimate**, not a measurement). 18000 s is
  ≈ 2.0 × that estimate and 4.2 × the measured 4274 s. Under bash 3.2 the alarm does not end a hung wire (landing
  :423–:427), so the watchdog is the bound there.
- **D-F3, X8's `bash` (A7).** X8's row (landing :398) copies the CI job's command, `bash scripts/trip-wires.sh`, and
  `scripts/trip-wires.sh` starts every wire as `bash "$w"` (`git show 01fb2924:scripts/trip-wires.sh`, line 204): the
  wires' shell is `PATH`'s `bash` whatever runs the script. So X8 runs as written, `bash` resolved by the driver's
  `PATH` (in this session `/opt/homebrew/bin/bash`, 5.3.20), recorded with `command -v bash` and its version. It is
  one set of six, not one per shell: X8 times the job's command, and a 3.2 set would need a `PATH` built to make
  `bash` resolve to `/bin/bash`, which the row does not state. X8 is a record that cannot fail (landing :343, §9.1
  step 3; :398, :87–:90). Its bound is a watchdog that kills the run's process group at 600 s (`/usr/bin/time` is the
  group's leader, so an alarm in it would not end the timed `bash`; runbook §6.8). Settled by the driver (§4, Q2).
- **D-F4, X11's line (A11).** "After the other `_lbl` definitions": controls :82–:123 is the label block (33 `_lbl="`
  definitions, the first at :82, the last, `_af_lbl`, at :123; no other part defines one, §7 C2). The two lines go
  after :123. Runner: `cell15.sh` with an empty insert, on a clone carrying the edit (an existing script; it gives the
  run its bound and process-group cleanup).
- **D-F5, X16's re-anchors (A9).** Same clause, new line or value:
  - `U` (`xbare`): R15 :175 says `_MUT_UNRECORDED_MAX=21` → `0`; at T the line is mutations :271
    `_MUT_UNRECORDED_MAX=18` → `_MUT_UNRECORDED_MAX=0`.
  - `A` (`xanchor`): R15 :176's record, in mutations.sh on the draft-18 prototype, is records :196 at T (`04466154`
    split records out of mutations, landing :400); its anchor `built"$` → `builtZZSTALE"$`, the one occurrence on that
    line.
  - `stale` (`stalectl`, `staleshim`): R15 :426–:427's record goes at the end of the `MUTANTS` here-document, which at
    T is in records.sh, ending at records :595: the line `s/K2NEVERMATCHES/x/` TAB `the fixture build window
    completed` is inserted before :595. `_MUT_RECORDS_MIN` is a floor (correspondence :407, `-ge`), so the 306th
    record reds nothing by count.
  - The plant cell (X3): s8b's edit at fixtures :698 (`21526e80`) is before fixtures :705 at T.
- **D-F6, X5's draft-10 cells split (A3; R1 IMP-8, replacing r2's Q1).** The instruments exist: `cell9.sh` is
  `cell8.sh` with one documented edit (PROV :825: "on the `env` line: `HOME="$OUT/home"` after `TMPDIR=…`"), and
  `cell8.sh` is `cell6.sh` unchanged (`PROV@453b7b0f` :811), verbatim at `PROV@ff1322bb` :832–:845; `eval9.sh` is
  `eval8.sh` with three arms added before `BEFORE-D2|BEFORE-D4)` (PROV :827), the arms verbatim in the fence PROV
  :829–:833, and `eval8.sh` verbatim at `PROV@453b7b0f` :866–:889 (24 lines; the anchor occurs once); `gen11.py` (PROV
  §E.7) with landing's re-anchoring patch (landing :496–:515) completes at T: `38 jobs`, 342 part copies (nine parts
  each), every copy clean under `bash -n` on 3.2 and 5.3 (§7 C17; at `6ed414b2`, six parts: 228, landing :447–:448). The rule
  then applies per job (§7 C17): `eval9.sh`'s arm for REF is green alone; for AFTER-E2, AFTER-E3 and AFTER-SE1 the
  expected text found (`hit`); for P, `hit` (the one P label, `d9 W2 record`, is not a `W:` label); for AFTER-E3g
  (`d9 ROg`) and G, green **and** `same`, the P compare, which needs the hook. G also sets `HOME` or
  `GIT_CONFIG_GLOBAL` to files under `corpus/h/`. **RUN, 32 jobs:** REF ×2, AFTER-E2 ×24 (`arnoise`, `noise1k`, `ar`,
  `rc5`, `nr`, `wtmeta` on two configurations; `m2h` on four; `d9` cachedir, nested, hidden and bare on two), AFTER-E3
  ×2 (`d9 RO`), AFTER-SE1 ×2, P ×2. **RETIRED, 6:** AFTER-E3g ×2, G ×4. *A reading the driver should see:* twelve of
  the 32 (the four `m2h` and eight `d9` AFTER-E2 jobs) write `core.excludesFile` with a path under `corpus/h/` or
  `r9ax3/outside/` as its value (§7 C17). The rule's criterion is a cell that needs a file there; these need the path
  string only: P-g's verdict is on the persisted key, read from `config --list` against the reference (harness :494),
  not on the file, so the absent file does not decide it. `gen11.py` builds those strings and opens neither directory
  at generation. Glue (runbook §6.5): the directories `cell6.sh` writes into (`logs/`, `pd/`) and `HOME`'s, and `chmod +x`.
- **D-F7, X5's drafts 11–21 cells (A6; R1 IMP-7).** r2 retired them as "not enumerated". Corpus §6.1 (:234–:490)
  enumerates them (§7 C16): the runs it names that are neither a record's kill arm (X3's main run) nor its survive arm
  (the survive check, L2) are `Jm` (:262, "the `Jm` cells"), `xrace` (:276), `lnblind2` (:277), `xbare` and `xanchor`
  (:283, "X16 cells"), `xexecarith` (:285). The rest of §6.1's named runs are a record's arms (`m14lc`, `m14grep`,
  `m14surv`, `floorkill`, `floorsurv`, `r422noarm`, `r423noarm`, `hFF*`, `hPk2`, `hsvR8`, `hsurvA`, `hR3nvr`, `hsurvB`,
  the `S*` and `RVusageonly245` cases), and `lnblind` (:277) is draft 16's measurement of an in-loop search C9 deleted,
  so it has no subject at T. Per cell: `lnblind2`, `xbare`, `xanchor`, `xexecarith` are X16 cells and run there;
  **`xrace` RUNS**: its insert is verbatim (R15 :80–:81: `xrefctl`'s insert, PR527 :721, then the loop), its runner
  `cell15.sh`, and its subject is present at T (the reference directory `mktemp -d "${_FW_DIR%/*}/pgrefXXXXXX"`,
  harness :175, which the loop's `pgref*` names); **`Jm` RETIRES**: its trees are the draft-19 and draft-18 prototypes
  with C6's record applied and "the record itself … pointed at the new line" (R15 :290–:292), an edit stated only as
  prose; its kill arm is at T X3's main-run trial of C6's re-anchored record, its survive arm (the clause removed) is
  the survive check's (L2).
- **D-F8, X3's survive check does not split (R1 IMP-9).** Survive arms of some records were measured by handoff
  instruments at their slices (R29-E's `rows2.sh` rows and `c4-unit`, impl-d :278, §T.K C4). Running those at T would
  give survive results for some records, but the check's subject is "each §6.1 record", whose population has five
  readings (305 / 211 / 203 / 129 / 130, §7 C12); a partial run decides nothing about it. The check stays RETIRED as a
  whole (L2).
- **Not a cell at T: X3's "C1–C3 are byte-identical to base"** (landing :393, expected column). It compares the
  mutation run at the split commits C1–C3 with base; at T it has no subject. It is left as the row states it (§4, Q3).

### §1.3 If T moves, or a RUN cell's definition changes: stop

**The rule.** §1's application of the rule and the runbook hold for T = `01fb2924` and for the RUN cells' definitions
as `3b7ff6d5` states them, and for nothing else. In either of two events the narrowed step 1 is **not** re-run: the
sequence stops and goes to the user. (1) **T moves**: by a step-3 fix that changes the tool code or the job (landing
:337–:338), or on #501's branch after the squash, by a fix or a merge, the CI/tools lane's change to
`.github/workflows/ci.yml` or `scripts/trip-wires.sh` included (land order 7.4, landing :152–:156). (2) **A RUN cell's
definition changes**: a docs-only fix under step 3's docs-only rule (landing :339–:344) changes the command, insert,
tree edit or expected value of a cell §1.1 marks RUN, wherever it is defined: its §11 row (X6's inserts, X11's lines,
X15's insert, X16's inserts and edits, for example), a corpus §6.1 row (`xrace`'s expected value, corpus :276), or a
record the runbook copies (`xrace`'s insert, R15 :80–:81 with PR527 :721). One RUN row is excepted: X8 is a record
that cannot fail (landing :343, :398, :87–:90), so a docs-only change to its row is recorded, not judged, and does not
stop the sequence (D-F3, Q2). A red RUN cell is a finding (§9), and its expected value is never rewritten to the
observed result: a change of a RUN row's expected value is a definition change, event (2). §5 restates landing :318,
:337–:344, :349–:351 and 7.4 to say so.

**Why.** Running step 1 at another T, or a changed definition at this one, needs a new plan delta. The runbook reads
`3b7ff6d5`'s frame (every doc read is `DOC`, the blob at `3b7ff6d5`) and carries its inserts byte-copied from it, so
it would run the old definition, and it is an accepted body, not edited (§9 item 2). A new T needs more: new anchors
(§1.2's re-anchors and every assert of the runbook name `01fb2924`'s lines, and its frame guard refuses another T or
another head of the branch on `origin`); the run/retire rule re-applied cell by cell, since a fix that moves T can
change what a cell's instrument needs and so its status; and sources that may be gone, since after the squash `W` and
its branch `k2-wire-fgit-hermetic` are cleaned up on merge, so the clones' source and the reads the runbook makes
through `W` may not exist at 7.4. A procedure for doing that inside the sequence would itself be an unreviewed
instrument; r2's and r3's versions of one drew findings in both review rounds. The L entries' **Triggers** (§8)
therefore route nothing in this sequence: they serve future PRs, tracked through two defer slots (§8).

**Two known events will likely fire the stop after the squash** (information for the user and the PM session, not a
plan): #501's fix of CR#1 in the wire (`…-impl-cr2.md` :58: the wire's unchecked `mktemp` and `_phys ""`, assigned to
#501's loop), a change to `.claude/tools/**`; and the CI/tools lane's work on `.github/workflows/ci.yml` and
`scripts/trip-wires.sh`. Either moves T on #501's branch, so 7.4 stops for the user.

---

## §2 Coupled invariants

The sequence's invariants (landing §9.1): **TD** T's definition and command (:314–:318, :353–:360); **TM** the
terminator, step 3 dry at a T unchanged since step 1 ran (:349–:351; restated: a move of T stops the sequence, §1.3);
**DR** step 3's docs-only rule (:339–:344; as written, a changed check definition is run at T in the fix commit;
restated, a changed RUN cell definition stops the sequence (X8's row excepted: a record that cannot fail; step 3), §1.3); **RB** every run at T is bounded, alarm plus
watchdog (:423–:441); **IS** runs are isolated: a clone per edit variant, ≤ 2 wire processes at a time, and after a
batch nothing of it left but what ends by itself (the documented runners' orphaned `sleep`, ≤ 123 s; the `_og`
probes, ≤ 10 s) and, on a run that reaches the bound, the wire's own groups, on which `stopgate` stops; **EV** step 2 records every result in the open record (:321); **X9** route (b), on #501 after the squash
(:361–:362); **LG** the ledger step precedes the squash approval and writes no commit (:346); **RL** the residuals'
definition, quoted: "a residual (§5.1, and §5.2's R1–R9, R11, R14, R15 and R17–R23) is a measured boundary of what the
gate decides: a declared limitation with a stated direction (…), not deferred planned work" (residuals :93–:96). *This
memo's reading* (not the ledger's words): a retired check is neither; it is verification the PR did not carry out at
T, a statement about the PR's evidence, not a boundary of the gate (§4 Q4).

**How IS holds** (R1 IMP-3). Every runner sweeps its own process group after its `wait`: `cell15.sh` and `cellw.sh`
list the group and `kill -9` it (PR527 :644–:645), `cellT.sh` and the patched `crc.sh` `kill -9 -<pgid>` it (landing
:522, :549), and the new glue (`x3`, `x8`, runbook §6.7, §6.8) calls the same listing-and-kill (`sweep`, runbook §6.0). The new glue's
watchdog is one `perl` process in its own process group, killed by group after the `wait`, so nothing of it is left.
The documented runners' watchdog (`( sleep 123; kill -9 -"$g" ) &`, landing :440, :521, :548) is sent TERM after the
`wait`; the TERM ends the subshell before its `kill` runs, so only the orphaned `sleep` remains, which ends within
123 s and runs nothing after it (not changed here: the documented patch).

**What the runner's sweep does not reach.** The wire makes process groups of its own with `set -m`: each control's
child (ctlrun :348–:353, `$BASH "$SELF" --selftest`) and its watchdog (ctlrun :371–:372, a `bash -c` whose `$0` is
`k2wd`), and the `_og` check's probes (controls :551, a `sleep 10` ignoring TERM; :559, `sleep 0.2` and two
`sleep 5`). A run that ends normally stops them itself (`_k2_reap`; the `_og` check's own stops). On a run that reaches the bound, the runner's kill and
its `sweep` name the run's group only, so these groups are outside them (§7 C23). `stopgate` sees the first two:
the child's command line names the wire (`trip-wir[e]`) and the watchdog's names `k2wd` (`k2w[d]`, added to `gate`'s
patterns in r4, its negative control covering it); the `_og` probes match no pattern and end by themselves within
10 s, running nothing after. Each batch ends with `stopgate`, which stops the runbook on any line from `gate`; `gate`'s
negative control runs before B1 (runbook §6.0). The driver reads the line before anything is killed and attributes it
(command line, working directory; a path under `$K` is this run's, and a process not clearly another session's
counts as this run's): another session's process is waited out, and the batch re-runs from a fresh `$K`; this run's
means a run reached its bound, a finding in that RUN cell (§9; runbook §6.0).

| pair | intersection | mechanism | check |
|---|---|---|---|
| TD×TM | what a move of T does to step 1 | nothing re-runs: the sequence stops for the user, at step 3 and at 7.4 alike (§1.3) | §5's rows for landing :318, :337–:338, :349–:351 and 7.4 (:152–:156, :162–:164) |
| TD×DR | a docs-only fix that changes a RUN cell's definition, wherever it is defined (§11 row, corpus §6.1 row, or a record the runbook copies) | it stops the sequence for the user, as a move of T does (§1.3, event (2)): the runbook reads `3b7ff6d5`'s frame and byte-copied inserts and is not edited, so it cannot run the new definition; X8's row excepted (a record that cannot fail: recorded, not judged) | §5's row for landing :339–:344 |
| TM×DR | a docs-only fix and the terminator | a change to a retired row's text runs nothing (no runner; re-opening one needs its instrument, the user's decision) and returns to step 3; a RUN definition change stops (TD×DR); so no docs-only fix carries a re-run at T | §5's rows for landing :339–:344, :349–:351 |
| DR×EV | a retired check's text changes after step 2; a RUN cell is red | the retired row is restated in place (still pointing to §T.L); §T.L, a dated entry, is not edited, and the change is recorded in the fix commit's own record entry. A red RUN cell is recorded as observed and is a finding; its expected value is never rewritten to the result (that is a DR event, §1.3) | §8, §9 |
| RB×IS | X3's 18000 s runs and the X5 batch hold the cap | X3 pairs run alone (b53 ∥ b32, the measured shape); B4 runs two jobs at a time (`xargs -P2`, landing :526); the cell batches B1–B3 run one cell at a time | runbook §6.0's batches |
| RB×TD | the bound is a runner property, not tool code | it lives in landing's "Cell runners at T" and in this memo; T does not move | §5 |
| EV×RL | a retired check has no result to record, and its defect needs a home | impl-d §T.L records each retired check's defect (L1–L5) in the record's own idiom for an unpinned gap; the residuals' accounting sentence :90–:92 is restated (Q4) and nothing else in the residuals is edited | §8, §9; §5 (residuals :54 in the hit list; :90–:92, the own-deferral accounting, restated for the new slot, no row added; :93–:96's words left, :93 rewrapped) |
| LG×EV | the ledger now names what #527 did not run | the step-4 text gains a sentence pointing to §T.L L4, L5 and the registration of `#11-k2-fixture-postcondition-coverage-at-t` (L1–L3), #527's second own deferral, so §9.2's `#11-trip-wire-liveness-bound` text it registers verbatim carries the restated **Accounting** line (2); the session that owns the shared ledger (on 2026-10-24, the PM session) writes it, before the squash approval; the head does not move; step 9's update after #501 lands is handed over the same way | §5 (landing :170, :173–:177, :191–:200, :271, :346) |
| X9×all | X9's route and row | unchanged; no retired check is handed to X9 | §5 (no edit to :399) |

Unchanged pairs: TD×RB, TD×EV, TM×EV, TM×X9, DR×X9 (X9's row stays X9's route, landing :342–:343), DR×RL (no retired
check is in the ledger, so a change to one touches no residual), IS×EV, IS×X9, LG×TM (step 4 follows a dry step 3).

---

## §3 Spec coverage map

**No spec surface** (as delta 2 §3: each row maps one of this memo's sections, or, for the row "runbook §6", the
section moved out of it into the runbook, the form `preflight.py` accepts; the siblings' §0, the runbook's included,
make it exit 1 by design).

| Spec section | Step | Branch | Touch | Full enum | User-input flow |
|---|---|---|---|---|---|
| none (this memo) §1 | the rule, per check | RUN / RETIRED / split | landing §9.1 step 1, §11 rows | ✓ for the ten checks landing :319–:320 lists, and every cell of each (§1.1; §7 C3, C4, C16, C17) | no |
| none (this memo) runbook §6 | the runbook | X3, X5, X6, X8, X11, X13, X15, X16 | step 1 at T | ✓ for the 130 runs of §1.1 | no |
| none (this memo) §8 | the defects | L1–L5 | impl-d §T.L; landing §11's retired rows | ✓ for the retired checks of §1.1 | no |

---

## §4 Decisions, and where `3b7ff6d5` stands

| id | decision | at `3b7ff6d5` |
|---|---|---|
| D-F1 | X5's corpus §6 rows retired whole (§1.2) | X5's row (landing :396) plans them at T: **restated** |
| D-F2 | X3's runs bounded at 18000 s + a group watchdog at 18003 s, signals at their default, `setpgrp` (§1.2) | the row has no bound; "Cell runners at T" lists the survive-check runner only: **change** |
| D-F3 | X8 with `PATH`'s `bash`, recorded; one set of six; a 600 s group watchdog | the row names `bash` only: **stated** |
| D-F4 | X11 after controls :123, by `cell15.sh` with an empty insert | "after the other `_lbl` definitions": **stated** |
| D-F5 | X16's `U`, `A` and `stale` re-anchored; the plant line re-anchored (§1.2) | written for the draft-18/20 layout: **restated** |
| D-F6 | X5's draft-10 cells: 32 run with `cell9.sh` and `eval9.sh` built from the companion, 6 retire (§1.2) | "X5 at T" (:444–:527) plans all 38 and names the hook as open: **restated** |
| D-F7 | X5's drafts 11–21 cells: the six of corpus §6.1; `xrace` runs, `Jm` retires, four run under X16 (§1.2) | X5's row: "drafts 11–21's cells (corpus §6.1)", not enumerated: **stated** |
| D-F8 | X3's survive check retired whole; no partial run (§1.2) | planned at T: **change** |
| D-F9 | X2, X3's survive check, X5's retired cells, X14 and X16's shim cells: each a stated defect, impl-d §T.L L1–L5 and in place in its §11 row, its trigger tracked by a defer slot owned by the CI/tools lane (§8) | planned at T: **change** |
| D-F10 | every run in a `git clone --local` of `W` detached at `3b7ff6d5`, made by `mkclone`; every doc read by `git show 3b7ff6d5:<path>`; `PATH` logged per batch (runbook §6.0) | the runners on disk run in `W` (A14): **change** |
| D-F11 | step 2's commit carries the narrowing: this memo, its runbook and its checker moved in, landing, corpus, design memo, umbrella edited, a dated note in `…-cr2-plan.md`'s banner, impl-d's new entry §T.L (§5, §9); the residuals ledger gains no row, its own-deferral accounting :90–:92 restated for the new slot (Q4, §5) | step 2's scope sentence (landing :330–:333) names comment lines, `CLAUDE.md` and the open record only: **restated** |
| D-F12 | the narrowing holds for T = `01fb2924` and the RUN cells' definitions at `3b7ff6d5` only; a move of T, at step 3 or at 7.4, or a docs-only fix that changes a RUN cell's definition, stops the sequence for the user (X8's row excepted: a record that cannot fail; step 3) (§1.3; R2-A, R3 IMP-A) | "the checks re-run only when T moves" (:318), step 3 sends a T-moving fix "back to step 1" and runs a changed definition at T in the fix commit (:339–:341), and 7.4 runs step 1's checks at the new T: **change** |
| D-F13 | the ledger step and step 9's update hand their text to the session that owns the shared ledger (on 2026-10-24, the PM session), which writes it; the amend names what #527 did not run under M-PATH and the slot's owner as routed, and the text registers `#11-k2-fixture-postcondition-coverage-at-t`, #527's second own deferral, with §9.2's `#11-trip-wire-liveness-bound` text restated to count it (landing :271; §5, §8) | step 4, step 9 and "Ledger text" have the driver's session write it: **restated** |
| D-F14 | one stamp form, "(restated YYYY-MM-DD)"; the memo named once, in landing §9.1 step 1's new text, and registered at design memo :22 (§5) | no edited file but residuals has a stamp convention of its own (§7 C22): **stated** |

**Where this memo departs from the driver's expected starting point.** (1) X5's RLOUD and P rows are retired, not run
(D-F1). (2) X5 runs in part: 32 draft-10 jobs and `xrace` (D-F6, D-F7). (3) X16 splits: 20 cells run, three
re-anchored; the three shim cells retire (D-F5, D-F9). (4) X11 runs through `cell15.sh` rather than a bare wire call
(D-F4), so that it has a bound; X6 and X15 likewise (A11). (5) X8 is one set (D-F3).

**The runbook's ambiguities, answered.** A1 → D-F2. A2, A4, A12 → X2 and X5's corpus §6 rows retired (L1, L3). A3 →
D-F6. A5 → L2, D-F8. A6 → D-F7. A7 → D-F3. A8 → L4. A9 → D-F5. A10 → L5. A11 → D-F4, runbook §6.2. A13 → runbook §6.0 (extraction and
patches, rehearsed: §7 C5). A14 → D-F10.

**Where the rule did not decide cleanly, and the driver's decisions** (r1 stated these as open questions; the driver
settled them for r2; R1 withdrew Q1):
- **Q1, `cell9.sh`: withdrawn by plan-review R1 (IMP-8)**: its premise, `eval9.sh` only as prose, is false (PROV
  :827–:833, `PROV@453b7b0f` :866–:889); replaced by the rule applied per job, D-F6.
- **Q2, X8 and "both shells": one set of six, `PATH`'s `bash`, logged** (D-F3, as r1): §11's preface (landing :387)
  says both shells, X8's row names `bash`; X8 is a record that cannot fail (landing :343), and `scripts/trip-wires.sh`
  runs every wire as `bash "$w"`, so `PATH`'s `bash` is the job's own shell, recorded (`logenv`, runbook §6.8).
- **Q3, X3's "C1–C3 are byte-identical to base".** A per-commit statement in a T row; the rule has nothing to apply to.
  Left as written.
- **Q4, the record of the retired checks: impl-d §T.L and the retired §11 rows, not the residuals ledger.** The
  decision (the home) stands. **Its reason** is the residuals definition (quoted in §2, RL; residuals :93–:96): a
  residual is a measured boundary of what the gate decides, and a retired check is not one. **Its field set** is the
  target record's own idiom for a gap: impl-d's "Declared, not pinned" paragraphs, **Gap**, **Consequence**, **Why
  here, not a residual**, **Trigger**, **Owner** (§T.J, impl-d :255; reused by §T.K, :282). L1–L5 (§8) use it; each
  **Owner** is the CI/tools lane through a defer slot (R3 IMP-B, the PM session's decision). Each retired §11 row of
  the landing is restated in place, pointing to §T.L (§5). The residuals ledger gains no row and no range change. Its
  accounting sentence (:90–:92) is restated, since the slot that tracks L1–L3 is #527's own deferral (verification
  #527 did not carry out at T, registered at its landing): 1 → 2 own deferrals, the ledger's net −2 → −1, as
  landing :271 and :199–:200 are restated (§5's rows).
  The three costs r1 named stay as §T.L content: the RO/ROg shape's green side pinned by nothing at T (L3; the design
  memo's sentence :517–:519, whose lines :518–:519 would become false, is restated, §5), `lnblind2`'s survive arm
  unmeasured (L2; corpus :277), and the class-(c) `mktemp` boundary for `shimafter` and `shimhash` unrun at T (L5).
- **Q5, step 2's scope sentence (landing :330–:333): r1's restatement accepted.** Step 2's commit carries the
  narrowing, so it also changes `docs/plans/` files; its invariant (T does not move) holds. §5 restates it.
- **Q6, decided (driver, 2026-10-24): the sweep's file list.** `…-cr2-sweep.md`'s `sweep.sh` takes
  `docs/plans/*k2-fgit*` (:64) and drops every `…-cr2-*` (:72). This memo, its runbook and its checker are therefore
  named in the cr2-plan namespace, `…-cr2-plan-finalhead.md`, `…-cr2-plan-finalhead-runbook.md` and
  `…-cr2-plan-finalhead-check.sh`, so that the case at :72 excludes them, like the earlier plan deltas (`…-cr2-plan-liveness.md`, `…-cr2-plan-liveness-2.md`; §7 C20). No sweep script
  changes; the exclusion comment (:68) gains the three names (§5), since its list of plan deltas is incomplete without
  them. The sweep is not run in this sequence (impl-d :290: not re-run at R29-E).

---

## §5 Ratified surfaces this delta changes

**Searches.** By vocabulary, `/usr/bin/grep -n -E '\bX2\b|\bX5\b|\bX14\b|\bX16\b|survive check|PDUMP|step 1'`; by
property (a statement that a retired check, or a retired cell's measurement, holds at T), `/usr/bin/grep -n -E
'shimafter|shimhash|shimprepend|mkctl|PX1|ROg|RO/ROg'`; both over the current-stating docs at `3b7ff6d5` (§7 C19 lists
the hits); and by the accounting the new slot changes (#527's own deferrals and the ledger's net), `/usr/bin/grep -n -i -E
'own deferral|own slots|nets? to|net change|−2([^0-9]|$)'` over every `k2-fgit-hermetic` doc and the umbrella at `3b7ff6d5` (§7 C28; its
hits under "Read and left" and in the rows for landing :199–:200, :271 and residuals :90–:93). Every hit, with its disposition:
- **Design memo.** :518–:519, edited (below). Left: :56 (PX1 in the decision list), :109 (PX1's citation, withdrawn
  by draft 12: `…-pr527.md` :140, :144, §7 C14), :184, :206, :234–:235, :264 (the class-(c) boundary and `shimprepend`'s clause, stated by property, no
  measurement at T), :246–:247 (the class-(c) cells, each dated at its prototype), :483 (ROg red with C8 and fixed,
  dated by `…-pr527-r15.md` §Q).
- **Landing.** Edited (below): :153 (7.4), :319–:320, :338, :341 (in :339–:344), :349–:350, :392, :393, :396, :403, :405, :428, :444.
  Left: :450–:451, :460, :491 (the hook's specification and the "Open" note inside "X5 at T", which the :444 prefix
  marks partly retired and keeps as the record a rebuild would start from); :31 (C5's verification, at C5,
  `PROV` §A.14); :378 (the land order's step 1, not §9.1's: no subject).
- **Corpus.** Edited (below): :54, :243 and :248 (in :242–:255), :484, :487 (in :486–:488). Left: :30 (the hook as a
  prototype instrument); :104, :123 (corpus §6's expected columns for draft 6's and draft 9's cells, under the recipe
  :54's new text retires at T); :262, :277, :283, :315 (dated §6.1 rows; :277's unmeasured survive result is named by
  the new final-head row); :481 (ROg red with C8, dated history).
- **Residuals.** :54 only (class (c): `shimafter` and `shimhash` give rc 0, PASSED, measured on the prototypes
  `…-pr527-r15.md` §Q records; a boundary of the gate dated by its source, not a claim at T; L5 states that it is
  unrun at T): left. **Census.** :39, :119 (class (c) by property, no measurement claim): left. **Umbrella.** No hit.
- **The accepted plan bodies** (`…-cr2-plan.md`, `…-cr2-plan-exitpath.md`, `…-cr2-plan-liveness.md`,
  `…-cr2-plan-liveness-2.md`, `…-cr-plan.md`): no hit of either pattern. Two of their sentences point at landing §9.1's
  step 1 through `X3` (`/usr/bin/grep -n -E '\bX3\b'`): `…-cr2-plan.md` :773 ("T moves with CR2, so X3 and the rest of
  `…-landing.md` §9.1 run at the new final T as already planned") and `…-cr-plan.md` :822 ("then X3: the full opt-in
  at the new T on both shells (`…-landing.md` §9.1)"). `…-cr-plan.md` :822 is about X3's main run at the final T,
  which runs: true as written, not edited. `…-cr2-plan.md` :773 becomes false for the retired checks ("the rest of
  §9.1 run … as already planned"): it gets a dated corrective note in `…-cr2-plan.md`'s banner, the form that plan
  sets for a line a later slice falsifies (its :18, :276–:278 and :538–:542; the liveness deltas' notes at :19–:32), not a body
  edit (row below). Their other `X3` hits (`…-cr2-plan.md`
  :660, :661, :664, :772, :883; `…-cr-plan.md` :736, :740) are about X3's main run, which runs.
- **The implementation records** (`…-impl.md` :753, `…-impl-cr.md` :793, impl-d :102) state step 1's list at their
  own frames: `hist`, not edited.

**File kinds** (by each file's own convention): landing, corpus, design memo, umbrella and residuals (its restatements
:127, :130 are in place) are current-stating (impl-d
:261's list), as is `…-cr2-sweep.md` (impl-d :92, :98); they are **edited in place**. The corpus's §6.1 **rows** are
dated history, so a new row is added and no row is edited (delta 2 §5 (c)). impl-d is an implementation record: a
**new dated entry**, §T.L, holds the retired checks' defects L1–L5 (§8) and cites :102 "as written at its frame
(§T.Z, `f4fb2466`)" without editing it (the convention impl-d :271 and :290 state: "§T.J gains no edit", "Not edited
… every earlier entry").

**Stamps (R2-B).** One form, "(restated YYYY-MM-DD)", the date being step 2's commit's (2026-10-24 if it commits that
day; any date has the width C18 measures). No edited file but residuals has a stamp convention of its own (`restated 20…` occurs 0
times in landing, corpus, design memo, umbrella and `…-cr2-sweep.md` at `3b7ff6d5`, §7 C22); the form is residuals'
(:127, :130: "restated 2026-10-24 (R29-E; …)") without its parenthesis. **Where the memo is named:** once in the
landing, in §9.1 step 1's new text, the pointer through which the landing's other stamps and the corpus's resolve
(with impl-d §T.L, which every retired row cites), and in the design memo by its registration at :22. The other
choice, naming the memo at each edited file's first stamp, wraps design memo :518–:519's new text to four lines at
that file's width, 761 lines, over its 760 budget; the short form keeps three, 760 (§7 C18 measures both).
Residuals has a form of its own (:127, :130), with a parenthesis naming the slice; its restated accounting takes the
short form, as the other files do (the slot it names resolves it, with impl-d §T.L), so that the sentence and :93, rewrapped with it, keep their four lines, :90–:93 (residuals stays 152 lines, so none of
the cites of its later lines moves, design memo :634's and `…-cr2-sweep.md` :90's among them; §7 C28).
**Rows without a stamp**, each by its site's precedent: `…-cr2-plan.md`'s banner note carries the date and slice in
its own form, as the notes above it do; umbrella :23 and :111 register a plan file, as the `…-cr2-plan-liveness-2.md` registrations there do
without one; `…-cr2-sweep.md` :68 extends a code comment's file list, whose liveness entries carry none; design memo
:22 extends the file list, whose plan-delta registrations carry none. Every other row's new text carries the stamp.

**T-specific values** in the landing's new text are written "at T = `01fb2924`": the narrowing holds for that T only
(§1.3).

| site (at `3b7ff6d5`) | old text | new text |
|---|---|---|
| landing :152–:156 (land order 7.4, its first sentence) | "4. **When T moves on #501's branch after the squash, by a fix or by a merge** (a merge of `main`, or 7.5's conflict resolution, that changes the tool code or the job), §9.1 step 1's local checks run at the new T on #501's branch, because they measure the tool code at T whatever moved it, and the sequence **returns to 7.1** (D17-B): the job runs again on the runner at the new T, and the new run's verdict is written in the open record (§13), beside the old one, which it supersedes." | "4. **When T moves on #501's branch after the squash, by a fix or by a merge** (a merge of `main`, or 7.5's conflict resolution, that changes the tool code or the job; the CI/tools lane's change included), the sequence **stops and goes to the user** (restated YYYY-MM-DD): §9.1 step 1's checks are narrowed for #527's T only, and running them at a new T needs a new plan for that T (§9.1 step 1). If the user decides to continue, that plan is written and reviewed and its step 1 runs at the new T first; only then does the sequence **return to 7.1** (D17-B): the job runs again on the runner at the new T, and the new run's verdict is written in the open record (§13), beside the old one, which it supersedes." The rest of 7.4 unchanged but its terminator (next row) |
| landing :162–:164 (7.4's terminator) | "a finding whose fix moves T returns to 7.1, and any other fix is docs-only and returns to the Codex round." | "a finding whose fix moves T stops the sequence for the user, as above (restated YYYY-MM-DD), and any other fix is docs-only and returns to the Codex round." |
| landing :170 (land order step 9) | "9. After #501 lands, update the ledger entries with #501's merge commit on `main`." | "9. After #501 lands, the driver hands #501's merge commit on `main` to the session that owns the shared ledger (on 2026-10-24, the PM session), which updates the ledger entries with it (restated YYYY-MM-DD)." The next sentence unchanged |
| landing :173–:177 ("Ledger text", the lead-in) | "… so the ledger is written **once more, in one step**, in the final-head sequence (§9.1, step 4). That step leaves the ledger holding exactly this:" | "… in the final-head sequence (§9.1, step 4): the driver hands this text to the session that owns `project_open-defer-slots.md` (on 2026-10-24, the PM session), which writes it (restated YYYY-MM-DD). That step leaves the ledger holding exactly this:" |
| landing :191–:198 (the amend of `#11-trip-wire-launch-environment`) | "**Amend** `#11-trip-wire-launch-environment` in two ways." … "The PM lane stays the owner." | "in three ways."; "The PM lane stays the owner." replaced by "Its owner is the CI/tools lane, as the PM session routed it on 2026-10-24 (restated YYYY-MM-DD)."; and, before that sentence: "(c) Not run under M-PATH at a commit of #527 (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L4, L5): X14's three callers (PX1's `~+/bin` `PATH`; Codex R23's `tools/bin` wrapper with its interpreter beside it; Codex R24's `~/bin` wrapper with a helper) and the `shimafter` and `shimhash` cells. Their last runs: R23's shape red on `cb8b0e09` and PASSED after it (corpus §6); a `tools/bin` wrapper P-e red on `da2730d4` and PASSED after it, and a `~/bin` wrapper with 97 calls from inside the window, its commit not recorded (`…-pr527.md`, Codex R11–R13): all before M-PATH; PX1 under M-PATH on the prototype `$A/sb` only (`…-pr527.md`, K10-3); the shim cells on the draft-19 prototype." |
| landing :199–:200 (after the amend: a new bullet, then "Fix the header count") | "- **Fix the header count.** The net change is −2 (one slot dissolved, two closed, one registered), and it **replaces** the −2 banner written at PR creation." | a new bullet before it: "- **Register** `#11-k2-fixture-postcondition-coverage-at-t` (restated YYYY-MM-DD) with this text:" followed by the slot text of §8 ("The slot L1–L3 register"), verbatim; the count: "The net change is −1 (one slot dissolved, two closed, two registered), and it **replaces** the −2 banner written at PR creation." |
| landing :271 (§9.2, the last line of `#11-trip-wire-liveness-bound`'s text, which step 4 registers verbatim, landing :190) | "- **Accounting**: #527's own deferrals, 1." | "- **Accounting**: #527's own deferrals, 2: this and `#11-k2-fixture-postcondition-coverage-at-t` (restated YYYY-MM-DD)." One line, as at :271 (§9.2's lines are not all wrapped; :270 is 298 columns), so the count does not change. Step 4 hands §9.2's text as step 2 leaves it, this line included (§9) |
| landing :318 (§9.1, the lead-in to the steps) | "The command is below. The checks re-run only when T moves:" | "The command is below. The checks run at T; when T moves, or a fix changes the definition of a cell step 1 runs, the sequence stops for the user (X8's row excepted: a record that cannot fail; step 3) (step 3; land order 7.4; restated YYYY-MM-DD):" |
| landing :319–:320 (§9.1 step 1) | "At T, unless already recorded for this T: X2, X3 (with corpus §6.1's survive check), X5, X6, X8, X11, X13, X14, X15 and X16." | "At T, unless already recorded for this T: the cells `…-cr2-plan-finalhead.md` §1.1 marks RUN (X3's main run, plant cell and floor cell; X5's draft-10 cells but ROg and G, and its `xrace` cell; X6; X8; X11; X13; X15; X16 but round 17's three shim cells), by its runbook `…-cr2-plan-finalhead-runbook.md`, which holds for T = `01fb2924` and the definitions as `3b7ff6d5` states them (the commit whose docs and inserts the runbook reads) only. X2, X3's survive check, X5's corpus §6 cells, ROg, G and `Jm`, X14 and X16's shim cells are retired at T, each a stated defect (`…-impl-cr2-d.md` §T.L, L1–L5) (restated YYYY-MM-DD). This file's restatements of that date are the final-head narrowing (option B), `…-cr2-plan-finalhead.md`." |
| landing :330–:333 (step 2's scope) | "It changes comment lines, `CLAUDE.md` and the open implementation record only:" | "It changes comment lines, `CLAUDE.md` and files under `docs/plans/` only (the open implementation record; at #527's final head also the narrowing `…-cr2-plan-finalhead.md` §9 lists) (restated YYYY-MM-DD):" — the rest of the sentence (`git diff --name-only <T> HEAD -- .claude/tools scripts` empty, the job's non-comment lines equal) unchanged |
| landing :337–:338 (step 3, the T-moving bullet) | "A fix that changes the tool code or the job (its `timeout-minutes` included) moves T: back to step 1." | "A fix that changes the tool code or the job (its `timeout-minutes` included) moves T: the sequence stops, and the user decides (restated YYYY-MM-DD); step 1's narrowed checks do not re-run at a new T (step 1)." |
| landing :339–:344 (step 3, the docs-only bullet, whole) | "A docs-only fix that changes a check's definition (a row of §11, or corpus §6.1) runs the changed check at T and records the result in the fix commit itself. T does not move: back to step 3. A red result is itself a finding and is handled as one: a fix that moves T goes back to step 1. Two rows are exceptions: … so its re-run is recorded, not judged." | "A docs-only fix that changes the command, insert, tree edit or expected value of a cell `…-cr2-plan-finalhead.md` §1.1 marks RUN, wherever it is defined (its §11 row, a corpus §6.1 row, or a record the runbook copies), stops the sequence, and the user decides (restated YYYY-MM-DD): step 1 runs the definitions as `…-cr2-plan-finalhead-runbook.md` holds them at `3b7ff6d5`, its inserts byte-copied, so a changed definition needs a new plan to run. One RUN row is an exception: X8 is a record that cannot fail, so a docs-only change to its row is recorded, not judged, and does not stop the sequence. A change to a retired check's text (X2, X3's survive check, X5's retired cells, X14, X16's shim cells) runs nothing: it has no runner; re-opening one needs its instrument, which is the user's decision (option B). A change to X9's row is run by X9's route, on #501 after the squash (land order step 7), not at T. In these three cases T does not move: back to step 3." |
| landing :346 (step 4) | "The one ledger step (§9, "Ledger text"). It writes no commit, so the head does not move." | "The one ledger step (§9, "Ledger text"): the driver hands the ledger text to the session that owns the shared ledger (on 2026-10-24, the PM session), which writes it (restated YYYY-MM-DD). It comes before step 5 and writes no commit, so the head does not move." |
| landing :349–:351 (§9.1's terminator) | "**Terminator:** step 3 is dry on a head whose T is unchanged since step 1 last ran. Only a Codex finding whose fix moves T returns the sequence to step 1; the sequence's own commit (step 2) never does, and neither does a docs-only fix, which carries its own re-run." | "**Terminator:** step 3 is dry on a head whose T is unchanged since step 1 ran. A Codex finding whose fix moves T, or whose docs-only fix changes the definition of a cell step 1 runs (X8's row excepted, step 3), stops the sequence for the user (restated YYYY-MM-DD); the sequence's own commit (step 2) never moves T, and no other fix runs anything at T." |
| landing :392 (X2) | the row | command cell prefixed "**Retired at T** (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L1): the P-dump hook, the hostile files and a nine-part `gen6.py` do not exist. As planned:"; expected cell prefixed "not run at T." |
| landing :393 (X3) | "…; then corpus §6.1's survive check, by hand: each §6.1 record run with the clause it names removed; then two opt-in cells: …"; expected "… Each §6.1 record survives with its clause removed; a FIFO record reaches the bound instead (§6.1's procedure; …"Cell runners at T" below)." | command: the survive-check clause replaced by "(corpus §6.1's survive check is retired at T (restated YYYY-MM-DD): `…-impl-cr2-d.md` §T.L L2)" and, after "Each of these runs on bash 5.3 **and** 3.2 at T", "under "Cell runners at T"'s X3 runner (alarm 18000 s, watchdog 18003 s)"; the plant insert "just before the fixtures' `built` line (fixtures :705 at T = `01fb2924`)"; expected: the two survive sentences removed. The rest unchanged |
| landing :396 (X5) | the row | command prefixed "**Partly retired at T** (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L3): the corpus §6 R, RLOUD, RES and P cells, the draft-10 ROg and G jobs and the `Jm` cells are retired (the P-dump hook, the hostile files under `corpus/h/`, a nine-part `gen6.py`, and `Jm`'s pointed record, stated only as prose, do not exist). The other draft-10 jobs (32 at T = `01fb2924`) run by "X5 at T" below, with `cell9.sh` built from `cell6.sh` as `git show ff1322bb:docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-provenance.md` holds it and `eval9.sh` from `eval8.sh` as the same file at `453b7b0f` holds it, each with the companion's §E.6 edit or arms (`…-cr2-plan-finalhead-runbook.md` §6.5–§6.6). Twelve of them (the `m2h` and `d9` AFTER-E2 jobs, `m0015`–`m0022` and `m0029`–`m0032` at T = `01fb2924`) set `core.excludesFile` to a hostile ignore file under `corpus/h/` or `r9ax3/outside/` that is absent, so their fixture differs from the measured one; their verdict, P-g on the persisted key, does not read the file. Drafts 11–21's cells are the six corpus §6.1 names: `xrace` here, by `cell15.sh`, and `lnblind2`, `xbare`, `xanchor`, `xexecarith` under X16. As planned:"; expected prefixed "Retired cells: not run at T. `xrace`: class (c), not owed (corpus §6.1); recorded, not judged, unless it reaches the bound." |
| landing :403 (X16) | "round 16's `xbare`, `xanchor` and `xexecarith` (C11, C12; each insert and tree edit verbatim in `…-pr527-r15.md` §Q), round 17's `shimprepend`, `shimafter` and `shimhash` (inserts verbatim in `…-pr527-r15.md` §Q), round 18's `stalectl` and `staleshim` (below)" | "… (C11, C12; each insert and tree edit verbatim in `…-pr527-r15.md` §Q; at T = `01fb2924` the tree edits are re-anchored: `U` is `_MUT_UNRECORDED_MAX=18` → `0` at mutations :271, `A` edits the record at records :196, and the `stale` record ends records.sh's `MUTANTS` here-document, before :595) (restated YYYY-MM-DD), round 18's …"; round 17's three cells moved to: "Round 17's shim cells are retired at T (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L5): each starts with `mkctl`, which §Q states only as prose." Expected: round 17's sentence ("Round 17's: `shimprepend` … the memo must say how.") removed |
| landing :405 (X14) | the row | command prefixed "**Retired at T** (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L4): no setup of the three is recorded beyond prose. As planned:"; expected prefixed "not run at T." |
| landing :427–:431 ("Cell runners at T", the sentence "So every runner …") | "So every runner used at T (X3's survive check, X5, X13, X14, X16, corpus §6.1's FIFO procedure) keeps its alarm … X3's survive-check runner also starts the wire with HUP, INT and TERM at their default, as X3's command does (`$SIG{INT}="DEFAULT"; …` before its `setpgrp`, BS-11)." | "So every runner used at T (X3's three runs, X5's draft-10 jobs by `cellT.sh` and `xrace` by `cell15.sh`, X6, X11, X13, X15, X16) keeps its alarm … X3's runner also starts the wire with HUP, INT and TERM at their default, as X3's command does (… before its `setpgrp`, BS-11), with `alarm 18000` and a watchdog at 18003 s that kills the run's group: above the 4274 s measured at `21526e80` for 251 records, scaled for the 305 records and the ship time of T = `01fb2924` (`…-cr2-plan-finalhead.md` §1.2, D-F2). X8 is bounded otherwise: it has no alarm (`/usr/bin/time` leads its group, and an alarm there would end `time` alone), and a watchdog at 600 s kills the group, `time` included. X3's survive check, X14 and the FIFO procedure are retired at T (restated YYYY-MM-DD)." |
| landing :444 ("X5 at T", through :527) | "**X5 at T.** The companion's §E.7 `gen11.py` …" | prefixed "**Partly retired at T** (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L3). The block below runs at T for the jobs that need neither the hook nor a file under `corpus/h/` (32 at T = `01fb2924`; its last line's input filtered to them, `…-cr2-plan-finalhead-runbook.md` §6.6); ROg and G, which need the hook, are retired. The hook's specification is kept as the record a rebuild would start from." |
| corpus :54 | "For X2/X5 on the implementing head, clone that head as `<dir>/p6`. The scripts take no other paths." | adds: "X2 and X5's corpus §6 cells are retired at T (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L1, L3): the recipe also reads `<dir>/corpus/h/`, hostile files recorded nowhere, and `gen6.py` copies five of the wire's nine files." |
| corpus :242–:255 (the survive check and the FIFO procedure) | "Each record is planned to be **killed** when X3 runs at T …, and to **survive** with the clause it names removed; that **survive check** is run by hand as part of X3 …" and the FIFO procedure | "… and to **survive** with the clause it names removed. The survive check is retired at T (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L2): its population is not defined and it has no runner. A survive arm measured at a slice stands at that slice's tree, as its row records." The FIFO procedure's sentences are kept, prefixed "As planned (retired at T):" |
| corpus §6.1 table, after its last row (:462) | — | a new row: "\| final head \| **no change**: the survive check is retired at T (`…-impl-cr2-d.md` §T.L L2). A row whose survive result was left to it (C9's `lnblind2`, :277: "expected, not measured; X3's survive check measures it") stays unmeasured \| — \| — \| — \|" (totals unchanged: 211 / 305); a dated row, so no stamp |
| corpus :484 | "**Not records, by the caller's `PATH`:** the X14 outcomes." | adds "X14 is retired at T (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L4)." |
| corpus :486–:488 (Cost (X3)) | "Only X3's survive check runs a FIFO record with its clause removed; there it waits, by the FIFO procedure above." | "No run at T runs a FIFO record with its clause removed: the survive check is retired (restated YYYY-MM-DD; `…-impl-cr2-d.md` §T.L L2)." |
| residuals :90–:93 (the accounting sentence :90–:92, #527's own deferrals and the ledger's net; :93 rewrapped with it) | "**Accounting:** 1 own deferral, `#11-trip-wire-liveness-bound` (§5.2 R9, draft 13). With `#11-k2-fixture-git-invocation-convention` closed, the keep-set slot dissolved below and `#11-k2-wire-exit-trap-masks-set-u-abort` closed (§5.2, `8f4ad3c8`), the ledger nets to **−2**." | "**Accounting:** 2 own deferrals (restated YYYY-MM-DD), `#11-trip-wire-liveness-bound` (§5.2 R9, draft 13) and `#11-k2-fixture-postcondition-coverage-at-t`. With `#11-k2-fixture-git-invocation-convention` closed, the keep-set slot dissolved below and `#11-k2-wire-exit-trap-masks-set-u-abort` closed (§5.2, `8f4ad3c8`), the ledger nets to **−1**." Wrapped with :93's words, unchanged ("The count is of defer slots (`#11-*`) only. The defer policy counts work a PR postpones; a residual (§5.1,"), in four lines of at most 115 columns (the paragraph's own maximum, :94 and :96): :93 still ends "a residual (§5.1,", residuals stays 152 lines, and :94 onward do not move (§7 C28). No row added, no range changed (§4 Q4) |
| design memo :518–:519 (the sentence :517–:519; its two last lines, in place) | "shape is pinned by the corpus RO/ROg cells (X5, re-anchored for T in `…-landing.md` §11), not by a record. ROg is green again since the fix round (measured on both shells, `…-pr527-r15.md` §Q)." | "shape is pinned by the corpus RO/ROg cells, not by a record; at T by the RO cell alone (restated YYYY-MM-DD): ROg needs the P-dump hook and is retired (`…-impl-cr2-d.md` §T.L L3). ROg was green at `6ed414b2` (measured on both shells, `…-pr527-r15.md` §Q)." Wrapped at :518's width (100 columns), three lines: the design memo goes 759 → **760** lines, at its budget (≤ 760; this exact text measured on a scratch copy, §7 C18) |
| design memo :22 (the file list; one unwrapped line, so the count does not change) | its last sentence: "… its successor for slice R29-E (the stop watches its groups as one set), accepted by plan-review rounds 1 and 2 and the r3 fix pass, is `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-liveness-2.md` ("`…-cr2-plan-liveness-2.md`"), the authority for the lines its §5 changes." | appends, after that sentence: "The final-head narrowing of PR #527's step 1 (option B), accepted by plan-review (its status line names the rounds), is `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md` ("`…-cr2-plan-finalhead.md`"), the authority for the lines its §5 changes, with its runbook `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md` and its checker `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-check.sh`." No stamp (see Stamps) |
| `…-cr2-sweep.md` :68 (`sweep.sh`'s exclusion comment, restored from r2; the code, :64 and :72, unchanged) | "… its plan deltas (`…-cr2-plan-liveness.md`, R29-D; `…-cr2-plan-liveness-2.md`, R29-E), this file and its" | adds "`…-cr2-plan-finalhead.md` with its runbook `…-cr2-plan-finalhead-runbook.md` and its checker `…-cr2-plan-finalhead-check.sh` (the final-head narrowing)," to the list of plan deltas; the case at :72 already drops all three (§4 Q6, §7 C20). No stamp (see Stamps) |
| umbrella :23, :111 | "… and its successor for slice R29-E, `…-cr2-plan-liveness-2.md` (…)"; :111's file list, ending "… and its residuals `2026-09-citation-hygiene-k2-fgit-hermetic-residuals.md`", and "Of the twenty-three files, twenty-one have `preflight` exit **1** … `…-cr2-plan-liveness.md` and `…-cr2-plan-liveness-2.md` exit 0" | :23 registers `…-cr2-plan-finalhead.md` (accepted by plan-review, the authority for the lines its §5 changes), its companion `…-cr2-plan-finalhead-runbook.md` (its §6, moved out) and its checker `…-cr2-plan-finalhead-check.sh`; :111's list gains the memo and the runbook by their full names, the checker staying out (a script, not a plan memo; the command's `*.md` does not take it); "Of the twenty-five files, twenty-two have `preflight` exit **1** by design …; `…-cr2-plan-liveness.md`, `…-cr2-plan-liveness-2.md` and `…-cr2-plan-finalhead.md` exit 0", the runbook's exit 1 from its §0's declaration, as `…-cr2-plan-exitpath.md`'s (measured, §7 C25). No stamp (see Stamps) |
| `…-cr2-plan.md` banner (a new last item of its "Corrective notes" list, after :32) | — | "- **:757 (CR2-Z's row)**, "X3 and the rest of `…-landing.md` §9.1 run at the new final T as already planned" (YYYY-MM-DD, final head; `…-cr2-plan-finalhead.md` §5, `…-impl-cr2-d.md` §T.L): at #527's final head step 1 runs the cells `…-cr2-plan-finalhead.md` §1.1 marks RUN; X2, X3's survive check, X5's retired cells, X14 and X16's shim cells are retired (§T.L L1–L5)." The line number is the body's as accepted (:773 at `3b7ff6d5` less the banner's 16 lines, :18–:33, as the banner's own rule states; §7 C26). One line, the notes' form |

**Read and left**, the sites outside the hit list above (each true as written): landing :399 (X9); design memo :32,
:57, :652 (the residual range "R17"–"R23": no residual is added); corpus :192 (R23's shape at `cb8b0e09`, dated) and
its retirement bullet :489; corpus :276 (`xrace`, class (c) and not owed: true, and the run at T records it);
residuals :7 (origins) and the residual definition :93–:96, its words unchanged (:93 rewrapped by the row above; a
retired check is not a residual, Q4, so no row is added). The accounting search's other hits (§7 C28), each a dated statement or a rule, not a current count: landing
:174 (the −2 banner written at PR creation, which :199–:200 replaces) and :184 (the heading of the section step 4
deletes, "2 own slots"); `…-pr527.md` :572, `…-reviews.md` :115, :272 (review rounds' dispositions at their drafts);
umbrella :132 (the per-PR cap of 3 own deferrals: 2 is within it); landing :400, `…-cr-plan.md` :817, `…-impl-cr.md`
:340 (a file's line delta, "−2", not a slot count).

---

## §6 The runbook (step 1, the RUN cells)

The runbook, §6.0–§6.8 (the common setup, B1–B8 and how each cell is judged), is in the companion
`…-cr2-plan-finalhead-runbook.md` (`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md`
after step 2), moved out at r4 by the touch-time split (§9); step 2 drops the handoff name here (§9 item 2).
It keeps the section numbers §6.0–§6.8, so a cite "runbook §6.x" in this memo names its sub-section there.

---

## §7 Claim rows

Every count or "only/none" statement of this memo, with its command. "check" = `bash H/cr2/logs/fhd/check.sh` (its
section named in brackets); its output is `H/cr2/logs/fhd/check.out`.

| id | claim | measured by | result |
|---|---|---|---|
| C1 | the frame `3b7ff6d5` resolves; T is `01fb2924`; the tool code at `3b7ff6d5` is T's; the runbook's frame guard passes (`origin`'s branch at `3b7ff6d5`) | check [frame], [T] (landing :353–:360's loop), [tool diff] (`git diff --name-only 01fb2924 3b7ff6d5 -- .claude/tools scripts \| wc -l`), [frame guard] (runbook §6.0's guard fence, taken from the runbook and run with `K` a scratch directory) | `3b7ff6d5816a…`; `01fb29249ba3…`; 0; `frame guard: passed`, rc 0 |
| C2 | the anchors of §1.2 and runbook §6 at T: built fixtures :705; generator append mutgen :304, inside `_mut_gen_run` :287–:348; mutations :271 `=18`, :275 `=305`; the xanchor record records :196; `MUTANTS` records :595; 33 `_lbl="` in controls, :82 first, :123 last, 0 `_*_lbl=` definitions in the other parts | `git show 01fb2924:<part>` then `/usr/bin/grep -n` (check [anchors], [_lbl …]) | as stated |
| C3 | RUN = 130 runs; RETIRED enumerated = 198 runs | the sums of §1.1: 6+34+4+6+2+36+2+40; 48+128+6+4+6+6 | 130; 198 |
| C4 | X16 is 23 cells; 20 run, 3 retire | landing :403's list counted: 4 FIFO + 7 round 13 + 2 round 14 + 1 round 15 + 3 round 16 + 3 round 17 + 2 round 18 + `gitfn` | 23 = 20 + 3 |
| C5 | the scripts extract and patch by runbook §6.0's script block, taken from the runbook and run as written (`cell15.sh` = PR527 :627–:646, the fence :626–:647); every extraction passes `xt`; the twenty insert files of runbook §6.3 equal the docs' text; every `crc.sh` cell anchor and every runbook §6 edit anchor is present once at T | check [scripts] (runbook §6.0's `defs.sh` block and its script block, both taken from the runbook's text and run with `K` a scratch directory; then the fence lines printed), [r4 inserts] (the runbook's §6.3 block run on a scratch `K`; each file compared with the text extracted from PR527 and R15 at the frame), [crc.sh cell anchors], [X16/X3/X11 edit anchors] | every `xt` passed, patches applied, `bash -n` clean on 3.2 and 5.3; `H/cell15.sh` = :627–:646 plus its line-2 comment; 20 inserts, 0 differ; each anchor count 1 (`xrace`'s reference directory included) |
| C6 | `gen6.py` lists 5 parts; 4 of its 9 P-row anchors are in T's harness | check [gen6.py …]: the REC list of `PROV@ff1322bb` :743–:826, each `old` counted in T's harness | `['', '.controls', '.harness', '.fixtures', '.mutations']`; present 4 of 9 (P-a env config, P-b NOSYSTEM, P-c void, P-f `-i`) |
| C7 | `mutgen.sh` and the `K2RE` lines are unchanged `21526e80..01fb2924`; records 251 → 305 | `git diff --name-only 21526e80 01fb2924 -- .claude/tools/webref-generic-core-trip-wire.mutgen.sh \| wc -l`; `git diff 21526e80 01fb2924 -- .claude/tools/webref-generic-core-trip-wire.sh \| /usr/bin/grep -c '^[-+]K2RE'`; `_MUT_RECORDS_MIN` at each (check [generator inputs], [records]) | 0 files, 0 lines; `_MUT_RECORDS_MIN=251`, `=305` |
| C8 | `21526e80`'s X3 wall times | check [s8b-x3] (`.end − .start`) | main 4274 / 4274, plant 4174 / 4173, floor 3290 / 3289 s (b53 / b32) |
| C9 | the hostile files `gen6.py` reads are not on disk under `H` | check: `find "$H" -maxdepth 4 -type d -path '*corpus/h'`; the files `gen6.py` names: `all5.cfg`, `callertrace2`, `do/`, `home_ign`, `hooks`, `plain`, `pyglob`, `tpl_cfg`, `tpl_hook`, `xdg_filter` | 0 dirs (the runbook also searched the session scratchpad, its A4) |
| C10 | battery 88 of 88 equal to base, both shells | check [battery]: per line, `new=` equal to `base=` | 88 lines, 88 equal, each shell. The logs carry no tree id: that the battery read T's parts rests on `H/DECISIONS-R7.md` :925, :930 and the log time (10-24 01:44, after `01fb2924`'s 00:51) |
| C11 | no run of X2 at a commit of #527 is recorded | check [X2 hits]: `/usr/bin/grep -n -E '\bX2\b'` over every `…k2-fgit-hermetic*` doc at the frame: landing :319, :392; corpus :54; `…-impl.md` :753; `…-impl-cr.md` :793; impl-d :102, :122, :126, :142 (:102 §T.Z's step-1 list; :122–:142 a review item also named "X2"); `…-pr527-r15.md` :793 (step 1's plan); `…-reviews.md` :186, :239 — each read | none records an X2 run; X2's last run is the draft-6 corpus on p6 (PROV :344–:365) |
| C12 | §6.1's survive-check population has five readings | corpus :476 (the command `bash -c '. .claude/tools/webref-generic-core-trip-wire.records.sh; _mutants \| wc -l'` and its value, 305 on bash 5.3 and 3.2) and :468 ("**211 records of this PR, 305 in all**"); `sed -n 234,490p corpus \| /usr/bin/grep -c '^\| '` 205 lines = 203 rows; `… -c -E '^\| [^\|]*\| \*\*add'` 129; the table lines piped to `/usr/bin/grep -c -i surviv` 130 (check [corpus §6.1 table], [corpus :468, :476]) | 305 / 211 / 203 / 129 / 130 |
| C13 | line counts at `3b7ff6d5`: landing 552, corpus 490, impl-d 294, design memo 759; the landing's edited prose (:152–:200, :318–:351, :423–:433, :444–:452) is hard-wrapped, at most 123 columns; the corpus's edited prose (:54, :242–:255, :484, :486–:488), at most 105 | check [line counts], [landing prose width], [corpus prose width] (`git show 3b7ff6d5:<file> \| wc -l`; `awk` max `length`) | 552, 490, 294, 759; 123; 105 |
| C14 | the quoted lines: landing :343 "X8 is a record that cannot fail"; residuals :93–:96 hold the residual definition; impl-d :278 holds §T.K's C4; impl-d :255 holds §T.J's "Declared, not pinned" with its five fields and :282 §T.K's reuse ("§T.J's idiom"); landing :152 7.4's T-moving sentence, :163 its "returns to 7.1", :170 step 9, :173 "Ledger text", :191 the amend "in two ways", :318 "re-run only when T moves", :337 the T-moving bullet, :341 "goes back to step 1", :346 step 4, :349 the terminator, :198 "The PM lane stays the owner.", :199 the header count "−2", :339 the docs-only bullet; PR527 :140 "withdrawn in draft 12" and :144 "draft 12 withdrew that" (design memo :109's PX1); `…-impl-cr2.md` :58 CR#1 "(wire :401: `mktemp` output unchecked" | check [quoted lines] (`sed -n` then `/usr/bin/grep`) | 1; 3; 1; the five field names; the reuse; thirteen lines, each 1; 1, 1; 1 |
| C15 | no residual row R24–R28 is planned or cited by this memo | check [residual rows …], over every line but this row: the patterns `**R2[4-8] `, `residuals.md` R2[4-8]`, `residual(s) R2[4-8]`, `(R2[4-8])`, `R24–R28` (Codex R23/R24 and PR527's R26 cell are review rounds) | 0 |
| C16 | X5's drafts 11–21 cells are six: `Jm` :262, `xrace` :276, `lnblind2` :277, `xbare` and `xanchor` :283, `xexecarith` :285 | check [r4 X5 drafts 11-21]: the lines of corpus :234–:490 with the word `cell(s)` (`/usr/bin/grep -n -i -w -E 'cells?'`: :234, :252, :262, :283, :284; :234 and :284 name X3's floor cell, :252 the runner section), and the backticked names on :262, :276, :277, :283, :285; every other parenthesised run name of §6.1 read and classed as a record's kill or survive arm (§1.2 D-F7) | `Jm`; `xrace`; `lnblind`, `lnblind2`; `xanchor`, `xbare`; `xexecarith` (`lnblind`: draft 16's, no subject at T) |
| C17 | the draft-10 instruments build and generate at T: `cell9.sh` from `PROV@ff1322bb` :832–:845 (`cell6.sh`; = `cell8.sh`, `PROV@453b7b0f` :811) plus PROV :825's edit; `eval11.sh` from `PROV@453b7b0f` :866–:889 (anchor once) plus PROV :830–:832; `gen11.py` with landing :496–:525: 38 jobs, 342 part copies, 0 failing `bash -n` on 3.2 or 5.3; kinds AFTER-E2 24, AFTER-E3 2, AFTER-E3g 2, AFTER-SE1 2, G 4, P 2, REF 2; 32 after the filter; T's parts name `K2_CORPUS` 0 times; twelve run jobs (`m0015`–`m0022`, `m0029`–`m0032`) carry a `corpus/h` or `r9ax3/outside` path, each only as the value of `core.excludesFile`; no run job's environment names `corpus/h` | check [r4 X5 draft-10] (runbook §6.5's block and §6.6's lines before `logenv B4`, taken from the runbook's text and run on scratch copies, generation only, `p11` a `git archive` of the frame's `.claude/tools`; the fence lines printed; `eval11.sh`'s arms listed; `cell9.sh`, `eval11.sh`, `cellT.sh` under `bash -n`) | as stated; every `xt` passed; `bash -n` clean on 3.2 and 5.3 |
| C18 | the design memo after the :518–:519 edit is 760 lines; naming the memo in that stamp instead would make it 761 | check [design memo after the :518-:519 edit]: §5's new text, with YYYY-MM-DD = 2026-10-24, wrapped at :518's width (100) on a scratch copy, `wc -l`; the same with ", `…-cr2-plan-finalhead.md`" after the date | 760 (budget ≤ 760); 761 |
| C19 | §5's hit lists are complete; the accepted plan bodies hold no hit of §5's two patterns, and their `X3` hits are the nine §5 names | check [§5 searches]: both patterns over design memo, landing, corpus, residuals, census and umbrella (line numbers listed) and over the five plan bodies; `/usr/bin/grep -n -E '\bX3\b'` over the five | the lists of §5's bullets; 0 and 0 in each body; `…-cr2-plan.md` :660 :661 :664 :772 :773 :883, `…-cr-plan.md` :736 :740 :822 |
| C20 | `sweep.sh`'s list takes this memo, its runbook and its checker and drops them as it drops the earlier plan deltas | check [sweep.sh's file list]: :64's glob `docs/plans/*k2-fgit*`, :72's case, all three names matched with `fnmatch` | glob true, dropped true, for all three |
| C21 | the last runs of X14's callers: corpus :192–:193 (R23's shape red on `cb8b0e09`, PASSED after); PR527 :50–:51 (a `tools/bin` wrapper P-e red on `da2730d4`, PASSED after; a `~/bin` wrapper, 97 calls, no commit); PR527 :93, :100 (R23 raised on `cb8b0e09`, R24 on `0116209d`); R23's fix `0116209d` and R24's fix `f8a5f3e6` by their subjects (`git log -1 --format=%s`: "fix(k2-wire): PR527 Codex R23 — …", "fix(k2-wire): PR527 Codex R24 — …"); PX1 under M-PATH on the prototype `$A/sb` (PR527 :290–:296); `cb8b0e09`, `da2730d4`, `0116209d` ancestors of `3b7ff6d5`, i.e. commits of #527 before M-PATH (draft 11, PR527 :130), and `f8a5f3e6` an ancestor too | check [X14 callers' last runs] (`sed -n`, `/usr/bin/grep -c`; `git log -1 --format=%s`; `git merge-base --is-ancestor`) | as stated; four ancestors |
| C22 | no edited file but residuals has a stamp convention: `restated 20` occurs 0 times in landing, corpus, design memo, umbrella and `…-cr2-sweep.md` at `3b7ff6d5`; residuals :127 and :130 hold "restated 2026-10-24 (R29-E; …)" | check [stamps] | 0 in each of the five; residuals 2 |
| C23 | the wire's own process groups at T: ctlrun :348 `set -m`, :352 the child `"$BASH" "$SELF" --selftest`, :372 the watchdog's `k2wd`; controls :551 a TERM-ignoring `sleep 10`, :559 `sleep 0.2` and `sleep 5` | check [wire groups] (`git show 01fb2924:<part> \| sed -n`, then `/usr/bin/grep -c`) | each 1 |
| C24 | this memo's and its runbook's line counts (the memo was 1025 before the r4 split, 628 at r4 and 688 at r5, the runbook 415 and 455) | check [memo line count], [runbook line count] (`wc -l`) | 720; 467 |
| C25 | the umbrella row :111's count after step 2: of the A-i-wire-fgit files (the twenty-three of :111 at `3b7ff6d5` plus the memo and the runbook), twenty-five, twenty-two have `preflight` exit 1 and three exit 0 (`…-cr2-plan-liveness.md`, `…-cr2-plan-liveness-2.md`, `…-cr2-plan-finalhead.md`); the checker is not taken by the command | check [umbrella count]: a `git clone --local` of `W` at `3b7ff6d5` on scratch, the memo and the runbook copied in under their repo names and the checker under its name, `git add`, then the umbrella's command (umbrella :176, `git ls-files 'docs/plans/*.md' \| xargs -n1 python3 .claude/skills/elidex-plan-review/preflight.py`) run one file at a time with its exit code, counted over the `k2-fgit-hermetic` names | 25 files: 22 exit 1 (the runbook among them), 3 exit 0, the three named; 99 tracked plan memos in all, the checker taken 0 times |
| C26 | `…-cr2-plan.md`'s CR2-Z row is :773 at `3b7ff6d5` and :757 in the body as accepted (`b85b67e3`), the banner's 16 lines (:18–:33: heading, 14 notes, a blank line) between | check [cr2-plan banner] (`/usr/bin/grep -n` at both commits; `sed -n 18p`, `33p`) | 773; 757; 16 |
| C27 | the ledger: `#11-trip-wire-launch-environment` routed to the CI/tools lane by the PM session on 2026-10-24, re-eval 2026-11-30, naming the sibling `#11-k2-fixture-postcondition-coverage-at-t` | check [the ledger slot] (memory `project_open-defer-slots.md` :307–:308) | as stated |
| C28 | the accounting the new slot changes: the search of §5 (`/usr/bin/grep -n -i -E 'own deferral\|own slots\|nets? to\|net change\|−2([^0-9]\|$)'`) over every `k2-fgit-hermetic` doc and the umbrella at `3b7ff6d5` hits exactly the sites §5 disposes of; residuals :90 "1 own deferral", :92 "nets to **−2**", landing :271 "own deferrals, 1."; residuals :90–:92's new text, YYYY-MM-DD = 2026-10-24, with :93's words after it, is four lines of at most 115 columns (the paragraph's own maximum, :94, :96), the fourth ending as :93 does, so residuals stays 152 lines and :94 onward are unchanged; landing :271's new text is one line | check [accounting sites] (`git show`, the search; `sed -n`; the new texts taken from §5's rows, residuals' wrapped with :93 at the break points that minimise the widest line and measured, landing's measured) | umbrella :132; `…-cr-plan.md` :817; `…-impl-cr.md` :340; landing :174 :184 :199 :200 :271 :400; `…-pr527.md` :572; residuals :90 :92; `…-reviews.md` :115 :272; the ASCII `-2` (after a space, `(`, `,`, `*`, a backtick or a line start) hits only identifiers and list suffixes (`GB2b-1, -2, -4`, GB2-13's `-2`, the slice suffix `-2`) and `head -2`: `…-cr-plan.md` :332 :697, `…-impl-cr.md` :302 :327 :395, `…-impl.md` :320, `…-pr527.md` :643 :813, no slot count; 1, 1, 1; 4 lines with :93's words, at most 114 columns, the fourth ending as :93 does, :94 onward unchanged, 152 lines; one line, 119 columns |
| C29 | L3's trigger clauses are named by §6.1 records: corpus §6.1 rows :285, :307, :312 name clause W (their clause column), and :288 names "P-a liveness, P-b liveness"; at T the records labelled `the fixture build window completed` (`_fw_lbl`, controls :82) are records :141–:146, :258, :306, :307, :326, and records :174 and :176 carry P-a's and P-b's liveness labels (`_pal_lbl`, `_pbl_lbl`) | check [L3 trigger clauses] (`git show` of corpus at `3b7ff6d5` and of records and controls at T; the clause column and `/usr/bin/grep -n`) | as stated |

---

## §8 The retired checks' defects (impl-d §T.L, and each retired §11 row)

**Where they are recorded** (the driver's decision, §4 Q4). (a) impl-d's new §T.L (§9 item 5) carries one entry per
retired check, L1–L5 below, in impl-d's own idiom for an unpinned gap, its "Declared, not pinned" paragraph (§T.J,
impl-d :255; §T.K, :282): **Gap** (what no run at T shows, what does run at T on the same subject, the last head or
prototype that verified it, and why its instrument is gone); **Consequence**; **Why here, not a residual**;
**Trigger**; **Owner**. (b) Each retired §11 row of the landing is restated in place, stamped "(restated
YYYY-MM-DD)", pointing to its §T.L entry (§5's rows for landing :392, :393, :396, :403, :405). "Retired" means: not
run at T, by the user's decision (option B, 2026-10-24). The T-specific line numbers below are at T = `01fb2924`.
**The Triggers serve future PRs only**, tracked through two defer slots, both owned by the CI/tools lane
(`ci-tools-lane-44`, which keeps the trip-wire tooling) and re-evaluated together on 2026-11-30 (R3 IMP-B, decided by
the PM session on 2026-10-24): L1–L3 through `#11-k2-fixture-postcondition-coverage-at-t`, which step 4's ledger text
registers (below), and L4–L5 through `#11-trip-wire-launch-environment`, whose owner the PM session routed to the
CI/tools lane that day (memory `project_open-defer-slots.md` :308, §7 C27). In this sequence any move of T, or of a RUN
definition, stops it for the user (X8's row excepted: a record that cannot fail; step 3) (§1.3), so no trigger routes anything here, and none needs to.

**The slot L1–L3 register** (landing §9's "Ledger text", a new **Register** bullet, §5; impl-d §T.J's idiom; the
driver hands it to the session that owns the shared ledger, on 2026-10-24 the PM session, which registers it):
`#11-k2-fixture-postcondition-coverage-at-t`. **Gap:** the fixture-git postconditions were not verified at #527's
final T, `01fb2924`, by the instruments that compare them: no run at T compares a fixture repo's P under a hostile
caller or a bypass spelling with a clean build (X2; X5's corpus §6 R, RES and P cells, ROg and G), and no record is
run with the clause it names removed (X3's survive check); the P-dump hook, the hostile files under `corpus/h/` and a
nine-part `gen6.py` do not exist, and the survive check's population is not defined (`…-impl-cr2-d.md` §T.L L1–L3).
**Consequence:** a caller environment or a fixture spelling that changes P with the run green, an edit that moves
the verdict call and changes a complete build's P, or a record that stays killed with its named clause removed,
would not show at T. **Why here, not a residual:** the residuals hold the gate's decision boundaries, each with its
direction (`…-residuals.md` :93–:96); this is verification #527 did not carry out at T, a gap in its evidence.
**Trigger:** a PR that changes any of P-a…P-g, the window's environment allowlist, the verdict call's position, or a
clause named by a §6.1 record; or a decision to rebuild the hook. **Owner:** the CI/tools lane (`ci-tools-lane-44`),
which keeps the trip-wire tooling. **Re-eval:** 2026-11-30 (with `#11-trip-wire-launch-environment`).

- **L1 — X2, the corpus §6 G cells.** Eleven hostile callers (a `HOME` ignore of `*.py`, an XDG attributes filter, a
  five-key `GIT_CONFIG_GLOBAL`, three `GIT_CONFIG_COUNT` injections, two `GIT_TEMPLATE_DIR`s, the DO cell,
  `GIT_CONFIG_GLOBAL=/dev/null`, a caller `GIT_TRACE2_EVENT`) on four configurations, each to stay green with every
  fixture repo's P equal to the same configuration's clean build (landing :392).
  **Gap:** nothing at T compares the fixtures' P under a hostile caller with a clean build. What runs at T on the same
  subject: the window's postconditions, always on (P-f "the fixture build window's environment holds only its
  allowlist", P-a, P-b, P-g; controls :85–:97), and their records in X3's main run. Last verified: never at a commit
  of #527 (§7 C11); the last run is the draft-6 prototype p6, 190 cells (PROV :344–:365). The instrument: "P equal"
  reads the dump of `K2_CORPUS_PDUMP`, a prototype-only hook whose commits are unreachable (`7c07415c`, `50f9e31a`,
  `22ea58f1`) and five of whose facts are recorded nowhere (landing :450–:491); the hostile files under `corpus/h/`
  are recorded nowhere (§7 C9); `gen6.py` copies five of the wire's nine files (§7 C6).
  **Consequence:** a caller environment that changes a fixture repo's P with the run green would be a false green for
  that caller, and no evidence at T excludes one beyond the postconditions' allowlist.
  **Why here, not a residual:** the residuals hold the gate's decision boundaries, each with its direction (residuals
  :93–:96); this is verification #527 did not carry out at T, a statement about its evidence, recorded with the gaps
  of this record (§T.J's idiom).
  **Trigger:** a PR that changes the window's environment allowlist or the postconditions P-a, P-b, P-f, P-g; or a
  decision to rebuild the hook (members of the slot's trigger).
  **Owner:** the CI/tools lane, through `#11-k2-fixture-postcondition-coverage-at-t` (above).
- **L2 — corpus §6.1's survive check.** Each §6.1 record was to survive with the clause it names removed (corpus
  :242–:243), the FIFO records reaching the bound instead (:247–:255).
  **Gap:** that each record pins the clause it names: no record is run at T with its clause removed. X3's main run
  still requires each record killed with its own label as needle (corpus :236–:237), so a kill by an unrelated check
  under another label is caught; a kill under the same label by another clause is not. A row that left its survive
  result to this check is unmeasured: C9's `lnblind2` (corpus :277, "expected, not measured; X3's survive check
  measures it"). Last verified: never as one check over §6.1; individual survive arms at the slices that measured
  them, each at that slice's tree, as its row records (e.g. R29-E's arms at `8a1f746a`, impl-d :278, §T.K C4); a
  partial re-run of those instruments decides nothing about "each §6.1 record" (§1.2 D-F8). The instrument never
  existed as a runner: the population has five readings (305 / 211 / 203 / 129 / 130, §7 C12), several rows name no
  clause (removals, relabels, re-anchors), and no runner applies "a record with its clause removed" in bulk.
  **Consequence:** a record that stays killed with its named clause removed (the clause unpinned) would not show.
  **Why here, not a residual:** as L1: unverified verification, not a decision boundary of the gate.
  **Trigger:** a PR that changes a clause a §6.1 record names (a member of the slot's trigger).
  **Owner:** as L1.
- **L3 — X5's retired cells.** The corpus §6 R (eleven bypass spellings, two of them under every hostile caller),
  RLOUD, RES and P cells (128 runs); the draft-10 ROg jobs (×2) and G jobs (HOME ignore, DO; ×4); the `Jm` cells
  (draft-19 and draft-18 trees, ×2 shells).
  **Gap:** no cell at T compares a bypass spelling's P with a clean build. The P rows (`gen6.py`'s nine:
  the window's completion, P-a and its liveness probe, P-b and its liveness probe, P-c, P-d, P-e, P-f; `PROV@ff1322bb`
  :743–:826) each edit the harness, and the run must turn red with that row's needle; RLOUD (`_git` under a `HOME`
  ignoring `*.py`) must give `CONTROL NOT EXERCISED (`. No cell at T builds those edits or that caller; the shipped
  records of the same postconditions are killed with their labels in X3's main run (corpus §6's records, :130–:166,
  as §6.1 changes them), which is what runs at T on that subject. Twelve of the 32 draft-10 jobs that run (`m2h` ×4
  and `d9` ×8, `m0015`–`m0022` and `m0029`–`m0032`) set `core.excludesFile` to a hostile ignore file, `corpus/h/pyglob`
  or `r9ax3/outside/ign-py`, absent at T, so their fixture differs from the one the companion measured; their verdict
  arm, P-g on the persisted key (harness :494), reads the key, not the file, and is unaffected (§1.2 D-F6). The green side of the RO/ROg shape (the
  verdict call moved below the first control with the window complete, P equal) is pinned by no record (design memo
  :517–:519, restated by step 2, §5) and by no run at T, while its red side runs (`d9 RO`, B4). `Jm`'s kill arm is X3's
  main-run trial of C6's re-anchored record; its survive arm is L2's. What does run of X5: the 32 draft-10 jobs and
  `xrace` (runbook §6). Last verified: corpus §6 on p6 (draft 6; PROV :344–:365); the draft-10 cells 38 of 38 PASS at C5
  `d7812ffe` on a clone carrying the hook (PROV :576–:578, :597, :600–:601); ROg rc 0, PASSED on both shells at
  `6ed414b2` (R15 :675–:677); `Jm` on the draft-19 and draft-18 prototypes, bash 5.3 (R15 :302–:303). The instrument:
  R, RES, ROg and G need the hook (L1), RLOUD and G files under `corpus/h/`; the P rows' edits anchor on harness text
  rewritten since (5 of 9 absent, §7 C6); `Jm`'s trees need "the record itself … pointed at the new line" (R15
  :290–:292), an edit stated only as prose.
  **Consequence:** an edit that moves the verdict call and changes a complete build's P, or a bypass spelling that
  changes P with the run green, would not show at T.
  **Why here, not a residual:** as L1.
  **Trigger:** an edit that moves the verdict call (`_fgit_window_verdict_exit "$_fw_lbl"`, controls :243) relative to
  the first control; an edit to a clause the P rows edit (the window's completion check, P-a, P-b and their liveness
  probes, P-c's void, P-d's allowlist entry, P-e, P-f's `-i`); or a decision to rebuild the hook (members of the
  slot's trigger: the completion check is clause W, named by the §6.1 rows of corpus :285, :307 and :312, whose
  records are records :258, :306–:307 and :326 at T, label `the fixture build window completed`; P-a's and P-b's
  liveness probes are named by corpus :288, which re-anchors their records, records :174 and :176 at T; so each is
  "a clause named by a §6.1 record", and the slot's text needs no extension; §7 C29).
  **Owner:** as L1.
- **L4 — X14.** PX1's caller `PATH` (`~+/bin` first, a logging `git` in the repository's `bin/`), Codex R23's
  `tools/bin` wrapper with its interpreter beside it, Codex R24's `~/bin` wrapper with a helper, each to end within
  120 s (landing :405).
  **Gap:** for those three callers, a wait (bounded only by CI's job timeout, residual R9) or a red is not excluded at
  T; a green with the wrapper pinned is what M-PATH predicts (design memo §0.1). None of the three has run under
  M-PATH at a commit of #527. Last runs (§7 C21): PX1 under M-PATH on the M-PATH prototype `$A/sb`, rc 0, PASSED, both
  shells (PR527 :290–:296); R23's shape red on `cb8b0e09` and PASSED after it (corpus :192–:193; R23's fix is
  `0116209d`, by its subject "fix(k2-wire): PR527 Codex R23 — …", §7 C21); a `tools/bin` wrapper P-e red on `da2730d4` and PASSED after it, and a `~/bin` wrapper
  with 97 calls from inside the window, its commit not recorded (PR527 :50–:51, Codex R11–R13): commits of #527
  before M-PATH (draft 11, PR527 :130). R24's own case, the `~/bin` wrapper's helper, is stated by its fix (PR527 :100–:102;
  the fix is `f8a5f3e6`, by its subject "fix(k2-wire): PR527 Codex R24 — …", §7 C21), with no run recorded. The instrument: no setup is recorded beyond prose: PX1's run line is verbatim (PR527 :295–:296)
  but its wrapper's text is not (:290–:291); R23's and R24's wrappers are described only (PR527 :93–:106, :50).
  **Consequence:** a caller with such a wrapper could see a wait or a red that no run at T has excluded.
  **Why here, not a residual:** as L1; and not a new slot: the class is member (3) of the open slot
  `#11-trip-wire-launch-environment` (`PATH`: every external tool, `git` and `mktemp` among them, arrives through it),
  which step 4's amend now tells that these three callers were not run under M-PATH at a commit of #527, naming
  their last runs (§5, landing :191–:198).
  **Trigger:** a report from a caller with a `tools/bin` or `~/bin` git wrapper or a `~+` `PATH` entry; a change to the
  pin (harness, `_FGIT_PATH`); or the slot's own trigger. (PR527 :151–:153 names X14 as where these outcomes would be
  recorded.)
  **Owner:** the CI/tools lane, through `#11-trip-wire-launch-environment` (routed by the PM session on 2026-10-24,
  memory `project_open-defer-slots.md` :308; re-evaluation 2026-11-30).
- **L5 — X16's three shim cells.** `shimprepend` (a `mktemp` shim first on the window's `PATH`), `shimafter` (after the
  pin) and `shimhash` (`hash -p`) (landing :403).
  **Gap:** for `shimafter` and `shimhash`, that the declared class-(c) boundary (design memo §0.3, residuals §5.1 :54)
  sits where it is stated: no run at T plants a `mktemp` shim behind the pin. `shimprepend`'s subject, P-j's
  first-entry clause, is pinned at T by C6's re-anchored record (corpus :262), killed in X3's main run. `staleshim`
  (X16, run) measures the parent's side of class (c), not the postconditions' `mktemp`. Last verified: the draft-19
  prototype (R15 :298–:300). The instrument: each insert starts with `mkctl`'s two lines, stated as "`lnblind2`'s `zzy`
  link plus …" (R15 :192–:195), and which of `lnblind2`'s five clauses (R15 :79) is "the `zzy` link" is not stated;
  the reviewer's insert files are gone (R15 :282–:283, a `/tmp` path).
  **Consequence:** a move of the class-(c) boundary (a red where rc 0 is declared, or a green where P-j should red)
  would not show at T.
  **Why here, not a residual:** as L4: the `mktemp` subject is the same slot's member (3); the amend names these two
  cells.
  **Trigger:** a change to how the window or the postconditions resolve `mktemp`, or to P-j; or the slot's own
  trigger.
  **Owner:** as L4.

---

## §9 The slice

**Step 1** (after acceptance), by CLAUDE.md's subagent tiering rule ("サブエージェント振り分け"): B1–B4, the cell
batches, are procedure with output to tabulate, so a **verifier** runs them; B5–B8, background runs judged by a
fixed-marker grep (the three X3 hit strings, `real`), are the **driver's**, run in place (the rule's "背景実行 + 固定
マーカーの grep で判定できる実行 … はドライバーがその場で行う"). Results to `$K/logs/`, and a stopped run's to its
moved-aside `$K.stop<n>/logs/` (runbook §6.0, §6.1). A red or a bound reached in a
RUN cell, or a `stopgate` stop on a process of this run, is a finding for the driver, handled as §9.1's step 3 handles
one. A red RUN cell's result is recorded as observed, and its expected value is never rewritten to it: a change of a
RUN row's expected value is a definition change (§1.3, event (2): stop, the user decides). A `stopgate` stop on
another session's process is waited out, and the batch re-runs from a fresh `$K` (runbook §6.0). A fix that moves T,
or that changes a RUN cell's definition, stops the sequence for the user (X8's row excepted: a record that cannot fail; step 3) (§1.3).

**Step 2's commit** (one commit, a designer for the docs, on #527's branch in a dedicated worktree; T does not move):
1. The step-2 work landing :321–:333 already names (the open record's results entry; the trip-wires comments and
   `CLAUDE.md` rewritten to the runner method; the complement check and its output).
2. This memo into `docs/plans/` as `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md`, status
   "accepted", `preflight` exit 0 on it (its §3 maps its own sections, as delta 2's does).
   Its runbook (§6, moved out at r4) goes in beside it as
   `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-runbook.md`, `preflight` exit 1 by design (its §0),
   unchanged but for three replacements in its Status paragraph: "draft r6, the companion of … reviewed with it" →
   "accepted (<the rounds>), the companion of `…-cr2-plan-finalhead.md` ("the memo")"; "(in the handoff
   `H/cr2/FINALHEAD-PLAN-DELTA.md`)" → "(`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead.md`)";
   "Step 2 moves it" → "Step 2 moved it". In this memo, §6's "; in the handoff `H/cr2/FINALHEAD-PLAN-DELTA-RUNBOOK.md`"
   is dropped and the status line set as above. The fences' `$H` paths (`defs.sh`, `K`) name step 1's run directory,
   which stays in the handoff, and are not pointers.
3. Its checker into `docs/plans/` as `2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-check.sh`,
   **unchanged**: its default memo, runbook and output directory follow its own location (in the handoff, the handoff
   memo and runbook and `H/cr2/logs/fhd/out`; in `docs/plans/`, the moved memo and runbook beside it and a fresh
   `mktemp -d`), its second parameter
   names another output directory, and it clears an output directory only when that directory is empty or carries
   its marker `.fhd-check-out`, and never the handoff's `out/` when it runs from the repository. Its first output line
   is its own sha256; impl-d §T.L records the sha256 of the file as run beside the result (the precedent of
   `…-cr2-sweep.md` :3–:14: "each block's sha256 prefix is of the file as run"). **What it still depends on** outside
   the repository: the absolute paths `W` and `H`; the handoff's logs C8–C10 read (`H/runners/logs/s8b-x3/`, `H` itself
   for C9's search, `H/runners/logs/dg-r29e-*.txt`); and the per-user memory directory
   (`~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/`, its [quoted lines] section's ledger reads). Its
   run output and step 1's run logs stay in the handoff: `H/cr2/logs/fhd/` (`check.out`, `out/`), `H/cr2/work/fh/logs/`
   and, for each run that stopped, the moved-aside `H/cr2/work/fh.stop<n>/logs/` (runbook §6.0); impl-d §T.L is written
   from both, a stopped run's batches that completed standing as recorded there.
4. Every edit of §5's table, stamped "(restated YYYY-MM-DD)" where §5 says so, `…-cr2-plan.md`'s banner note
   included; the memo named in landing §9.1 step 1 and at design memo :22; the retired §11 rows point to impl-d §T.L (§8 (b)). The residuals ledger gains no row;
   its accounting sentence :90–:92 is restated, rewrapped with :93 in the same four lines (§5, §4 Q4). The design memo goes to 760 lines (≤ 760, measured, §7 C18): :22 and :518–:519 are edited in place.
5. **impl-d §T.L, "The final-head sequence at T"** (new, dated): the route (option B, the user, 2026-10-24; the plan
   delta's review rounds); the narrowing, with §1.1's table; §T.Z :102's step-1 list cited as written at its frame
   (`f4fb2466`), superseded for the final T by this entry (§T.Z is not edited); step 1's results, one row per RUN cell
   and shell (rc, time, the expected text found or not), B4's 32 verdict lines, and the six X8 times; the retired
   checks' defects, §8's entries L1–L5 in their five fields; `logs/env.txt`'s `PATH` and versions; T's line numbers as
   runbook §6 used them (at T = `01fb2924`); the checker's sha256 as run; the cites this commit moves: the landing's edits at
   :152–:156, :162–:164, :170, :173–:177, :191–:200, :318–:320, :330–:333, :337–:344, :346, :349–:351, :427–:431 and
   :444, and the corpus's at :54, :242–:255, :484 and :486–:488, are to hard-wrapped prose (the landing's at most 123
   columns, the corpus's at most 105, §7 C13) and add lines, and the corpus gains one table row after :462, so every landing cite after :152 and every
   corpus cite after :54 moves; §T.L gives the old → new map measured on the committed files (`/usr/bin/grep -n`), and
   this memo's cites stay at `3b7ff6d5`, its frame (the table-row prefixes, :392–:405, add no line). Budgets after:
   impl-d < 800 (294 lines at `3b7ff6d5`), landing and corpus < 1000.
6. Not edited: the residuals ledger but its accounting sentence (§5); every accepted plan body (`…-cr2-plan.md`'s banner gains one note, its own
   convention, §5); every earlier dated note and implementation-record
   entry; every corpus §6.1 row; every `.sh` part. `…-cr2-sweep.md` is edited in its comment :68 only (§5, §4 Q6).

**Step 4, the ledger step** (landing :346 as restated), **and step 9** (landing :170 as restated): the driver hands
the text of landing §9's "Ledger text" (with the amend's new sentence (c) and owner sentence, the registration of
`#11-k2-fixture-postcondition-coverage-at-t` with §8's slot text, and §9.2's `#11-trip-wire-liveness-bound` text as
step 2 restates it, its **Accounting** line counting 2 own deferrals, landing :271; §5), and after #501 lands, #501's merge commit, to
the session that owns `project_open-defer-slots.md`; on 2026-10-24 that is the PM session (memory
`project_pm-and-lane-driver-structure.md`: "PM owns … `project_open-defer-slots.md`"; "Drivers own: only their lane
SSoT … They report landings, and the PM transcribes them"), which writes both. Step 4 comes before the user's squash
approval and writes no commit, so the head does not move.

**Effect on the terminator** (landing :349–:351, restated, §5): step 3 is dry on a head whose T is still `01fb2924`.
A Codex finding against a retired row's text is a docs-only fix that runs nothing (§5, the step-3 bullet) and returns
to step 3; a finding whose fix moves T, or changes a RUN cell's definition (X8's row excepted: recorded, not judged), stops the
sequence for the user, as does a move of T on #501's branch at 7.4 (§1.3). A user decision to continue at 7.4 needs a new plan delta and its step 1 at
the new T before 7.1 (§5's 7.4 row): no path reaches 7.1 without it. **X9 is unchanged**: route (b), on #501 after the squash (landing :361–:362, :399, step 7); no retired check
is handed to it, and its row is not edited.

**Cohesion**: no tool file is touched. The landing (552 lines), corpus (490) and impl-d (294) stay below 1000; no split
is triggered. The design memo reaches its 760 budget (§7 C18): the next edit that adds a line to it owes the touch-time
split judgement first. **This memo** moves into `docs/plans/` as an accepted body (`wc -l` at r6: §7 C24). The split is
done at r4: §6 (the runbook) moved into the companion `…-cr2-plan-finalhead-runbook.md`, which the sweep's `…-cr2-*`
case drops as it drops this memo (§4 Q6), with the checker's block extractions pointed at it; §0–§5 and §7–§9 (the
decisions, their evidence and the slice) stay together, since §7's rows are the claims of those sections.
