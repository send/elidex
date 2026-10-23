# K2 fixture git — PR #527 CR2 round: the pre-push gate's eight own items (plan)

**Status: accepted (plan-review TERMINAL 2026-10-21, `…-impl-cr2.md` §T.0), revision r5** (moved into
`docs/plans/` as `…-cr2-plan.md` at CR2-M, §11, with its companion `…-cr2-plan-exitpath.md`,
which holds §6; handoff names were re-pointed at that move). It plans the fixes for the **eight
items of PR #527's own code** that the pre-push gate raised at `706049ad`: the seven findings of `/code-review max` and
`/elidex-review` that the classification recorded in `…-impl-cr2.md` §T.0 lists under "#527's own" with their origin commits
(§T.0 of that file also records each finding's statement, abridged), and the watchdog's kept stream that R1-F3's measurement found (`c3c0b74b`,
#527's own; scope record: O-3 below). Driver push-gate decisions: `DECISIONS-R7.md` "## Push gate". **The base-PR
section of that classification (CR#1, #2, #3, #5, #6, #8, #10, #13, #15, ER-A5-2, ER-A3-1, ER-A4-*) is assigned to PR
#501's loop and is not planned here.** The CR round's settled conventions hold (`…-cr-plan.md` approved r4,
`…-cr-sweep.md`, `…-impl-cr.md` §S): bash 3.2, awk values through `ENVIRON` only, fail-closed, P-x matched exactly as
`_p<x>_lbl|_p<x>l_lbl`, no 8-hex string in mutations.sh's comments (and, from CR2-0, in the new part's: §8), never an
edit of a record-anchored line to simulate a profile, no tool installs. r2 implemented plan-review R1's F1–F34, r3
R2's G1–G40, r4 R3's H1–H28, r5 R4's J1–J29, each as the driver decided them; the change maps at the end name the sections.
**Section references:** `x§6.n` is the companion's; a § after a file name is that file's; any other § is this memo's.

⚠ **Corrective notes** (dated; one line per falsified line, added by the slice that falsifies it, under G36's rule, §7 and `…-cr-plan.md`'s notes: the body below is left as accepted, because it is provenance; the line numbers are the body's as accepted, before the first note, which shifts it by the notes' own lines; the authority for each new text is `…-cr2-plan-liveness.md` §5, and `…-impl-cr2-d.md` §T.J records the slice):
- **:90 (CR#11's row) and :460**, "(TERM, 1–2 s, KILL …)" (2026-10-23, R29-D): TERM, a 1–2 s grace, then KILL until gone (`_k2_kill_gone`).
- **:562**, "`_k2_reap_add` and `_k2_end` (12 lines)" (2026-10-23, R29-D): 5 + 6 = 11 lines at R29-D's head (`_k2_reap_add`, the wire :502–:506; `_k2_end`, :507–:512).
- **:555 (the wire's budget) and :704 (CR-B)**, **1376** = base + 90 (2026-10-23, R29-D): the wire's stop line is **1389** (`…-cr2-plan-liveness.md` §5, its Q8: the R29-D mechanism keeps its code in the wire, by its cohesion re-judgment); 1386 at R29-D's head.
- **:625 (G11-2c)**, "TERM, then 1–2 s (whole `SECONDS`), then KILL" (2026-10-23, R29-D; "then KILL" already corrected by `…-impl-cr2-d.md` §T.I D3): TERM, then `_k2_kill_gone` (a grace, then ≤ 21 KILLs while the group reads live, with an identity latch); pinned by R11w, R29d and the `_og` deaf row (R29i, R29j).
- **:635 (G11-10)**, "stopped only when `kill -0 -<pgid>` fails: `_k2_end` re-KILLs … (bounded)" (2026-10-23, R29-D; first corrected by `…-impl-cr2-d.md` §T.H (1)): … when `_k2_live -<pgid>` fails (a member that is not a zombie, and two table reads gone); `_k2_kill_gone <g> 1`; pinned by zomb, R29a–R29h and R29k–R29p.
- **:636 (G11-11)**, "unknown → skip; a `ps -o etime=` that cannot read the run's own pid refuses" (2026-10-23, R29-D): unknown → skip and named on fd 9 (R29q); either `ps` read failing on the run's row refuses (controls :230).
- **:649 (BS-12)**, (4) "… is skipped and a signal leaves that group running", the refusal by `etime` only, items (1)–(4) (2026-10-23, R29-D): (4) is skipped and named; the refusal needs both `ps` reads; (5) is residual R19 (`…-residuals.md`).
- **:698 (CR11-K)**, "dropped only when `kill -0 -<pgid>` fails (H2)" (2026-10-23, R29-D; first corrected by `…-impl-cr2-d.md` §T.H (1)): … when `_k2_live -<pgid>` fails; its probe (`cr2/bin/cr2c2-r4race.sh`) extracts the stop's three functions first.
- **:431**, `_ctl_left`'s groups "could not confirm empty" (2026-10-23, R29-D): could not confirm ended (a group of zombies has ended).
- **:637 (G11-12) and :700 (CR11-A)**, "stops the call's groups, then removes their entries"; "restores `set +m`, stops, then removes (H17)" (2026-10-23, R29-D): … stops, names and keeps the groups not confirmed gone, and removes the rest (R29r).
- **:701 (CR11-J)**, the J1 text, "the reap's identity check needs `ps -o etime=`" (2026-10-23, R29-D): controls :230's text: the reap needs both `ps` reads (`etime` and the process table).
- **:819**, "`kill -0 -<pgid>` and `ps -o etime=` are read here on macOS only" (2026-10-23, R29-D; first corrected by `…-impl-cr2-d.md` §T.H (4)): the table read too, refused where it cannot list the run's own row.

**Frame: `706049ad`** (the git clone `cr2-wt`, branch `cr2`). Every line number is at `706049ad` unless a commit is
named beside it. **(M)** = measured in `git clone --local` copies of it under `cr2/work/` (never in `cr2-wt` or the
worktree); scripts in `cr2/bin/`, logs in `cr2/logs/` (paths relative to the handoff directory); b53 =
`/opt/homebrew/bin/bash` 5.3.20, b32 = `/bin/bash` 3.2.57; BSD `/usr/bin/grep`, `/usr/bin/sed`; each wire run its own
process group under `perl -e 'setpgrp; alarm …; exec @ARGV'` with a `kill -9 -<pgid>` watchdog beside it (bash 3.2
ignores the alarm), at most two at once; a signal measurement starts the run with HUP, INT and TERM reset to their
default by `perl` (as `cr2/bin/cr11.sh`). A **mechanism prototype** is a marked edit in a clone made by a
`cr2/bin/*-proto.py` or `*-fix<n>.py` script (each frozen, never overwritten), not the slice's text; the exit path's
texts — [r3f], r4's feasibility text [r4p] — and their logs are named in the companion's header and x§6.6.
The frame's ship run: PASSED, rc 0, about 20 s, both shells (`cr2/logs/ship-base.{b53,b32}.log`).

---

## Decided before r3 (was "Open for the driver")

**O-1 — decided (A), by the driver on 2026-10-20, by lens** (`DECISIONS-R7.md`:843: "split the 1414-line #519 memo
inside #527 as its own commit (CV2 / CLAUDE.md touch-time split: the PR that touches it splits it first), then fix the
banner (#527's own text; (B) would defer an own defect; (C) violates touch-time split)"). Asked because ER-A5-1 touches
that memo, whose next touch CV2 (`…-cr-sweep.md` §F row 12) ratifies to split first, and D5 forbade the touch in the CR
round. The form: in #527, a standalone split commit (CLAUDE.md allows "単独 PR / 単独 commit": §10–§11, :930–:1414,
485 lines, moved unchanged to a companion, pointer left), then one commit that rewrites the banner (ER-A5-1 + R10 +
R12 + R13), deletes those residual rows and re-points every referrer into the moved text (§7). Not taken: (B) keep D5
and file a residual (defers an own defect); (C) the banner without the split. Slice CR2-D; ratified surface: §1 (5).

**O-2 — coordination with #501's loop (no decision needed; a merge rule).** Three files of this plan are also edited
by #501's loop:
- **The wire.** CR2-C2 edits the exit path (:405–:431) and CR2-D re-points six comments (:323, :403, :482, :711, :750,
  :1190, §7). #501 fixes CR#1 (:401–:404) and CR#13 (:324–:341). CR2-D's :323 and :403 are inside or beside those
  hunks.
- **controls.sh.** #501's CR#2 edits the bnd/seg controls (controls :511, :567). CR2 edits controls at :76–:77 and
  :98–:118 (CR2-A comments, `_af_lbl`, `_og_lbl`), :124 (`_MUT_PARTS`, CR2-0), :216–:219 (the reap directory, C2),
  :335–:367 (the `_wm` check's hang stub and row, C2), :475 (the fourth standing check after it, C2), :677 (a
  re-pointed comment, CR2-D), :690 (the two new controls after it, C2) and :691–:712 (C1). The nearest pair is :567 vs
  :677, 110 lines apart.
- **The umbrella.** #501's ER-A4-3 edits `docs/plans/2026-07-citation-hygiene-umbrella.md`; CR2 edits its rows :22,
  :23, :110 and :111 (CR2-M, CR2-D; §7, §10 CR-U).
**The rule:** if #501's fix tip lands on #527's base while CR2 is in flight, the next CR2 slice to start re-frames onto
it first, by a **merge** of that tip into #527's branch in the worktree (#527 is pushed: a rebase would need the
force-push the hooks forbid), made before the slice's clone is taken, so its transplant stays a fast-forward; the
wire's stop line is re-derived from the merged frame (x§6.3); line cites are re-read on it (a hunk #501 moved is
re-located by its text), and an edit inside a #501 hunk (CR2-D's :323 and :403 comments) is re-done by hand on #501's
text. No CR2 slice starts on a base older than the last #501 tip that landed.
CR2-C2 is ordered last among the code slices so that, if #501's fixes land first, C2 is written on them.

**O-3 — the scope is eight items (recorded by the driver, push-gate lens).** CR2 was planned in r1 for the seven items
the classification (`…-impl-cr2.md` §T.0) lists as "#527's own". R1-F3's descriptor measurement found an eighth defect of #527's own code:
`_ctl_run`'s watchdog keeps the caller's stream (`c3c0b74b`, not an ancestor of `origin/webref-cite-audit-tool`, so
#527's). The driver folded it into CR2-C2 by the push-gate lens ("this unmerged PR's code → fix before push"), recorded
in `DECISIONS-R7.md` "## CR2 plan-review (2026-10-21)" ("Scope +1"). §1's table lists it as the eighth row.

Every other choice below is decided by the lenses (fail-safe direction, one issue one way, altitude) and stated with
its reason.

---

## §0 Spec coverage map

**No spec surface** — this file plans changes to a shell trip-wire's self-test harness. It has the same by-design
shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this heading. The POSIX shell
behaviour the exit path relies on (x§6.0) is cited by section name with the measurement that shows it on both shells;
POSIX is outside `.claude/tools/webref`'s data sources, so no number is given that was not read.

---

## §1 Scope

| item | origin | what is wrong (one line) | fix (one line) | slice |
|---|---|---|---|---|
| CR#4 | `c46fc6c1` (CR-C1) | rule 1's expansion scan feeds both the orphan check and the ratchet; its under-counts are false reds for the first and false greens for the second | the ratchet and the literal rule read the spelled names (`$_lbd_names`, an over-approximation by construction); the orphan check keeps the scan, its over-counts declared and backstopped by the opt-in | CR2-A |
| CR#7 | `04466154`; the anchor check | "fresh" = "changes exactly one line", blind to an anchor that occurs twice on its line and to a replacement that restates text it does not read | fresh = one line, **position-unique** (applied twice = once) and **faithful** outside the changed span (any character there, perturbed, either stops the match or is carried through); 24 records re-anchored, one removed; a self-test pins the rule | CR2-B |
| CR#9 | `d976f239` (S1') | the fsmonitor block's plain-git retry unsets three hand-named variables | it purges git's own list, `$_GIT_LOCAL_VARS` | CR2-C1 |
| CR#14 | `16efb423` (S2) | `$_VFY/.control_out` is not cleared before a launch, so a child whose redirect fails is judged on the previous control's output | the file is removed with the timeout marker before the launch, fail-closed | CR2-C1 |
| CR#11 | `8f4ad3c8` | a signal reaches the EXIT trap with `$?` = 0 (false "ended before its verdict"; statuses 143/130/129); `_ctl_run`'s groups outlive the wire | HUP/INT/TERM become `exit 128+n` with a line written to the stderr saved at startup; the exit path stops every group `_ctl_run` has live (TERM, 1–2 s, KILL: one stop procedure, shared with the watchdog), from a reap set carried in files (visible from the subshell standing checks; every set the run makes inside the run's own), each group recorded right after its launch with the time of its record, acted on only while it is the run's, and dropped once its group is gone (or its id names a reused leader); a reap set that cannot be written ends the call with exit 2; the exit-path bodies ignore the three signals and run with errexit off | CR2-C2 |
| CR#12 | `8f4ad3c8` | the EXIT trap's `rm -rf "$SCRATCH"` under `set -e`: a failed removal turns every status into 1 | errexit off in every exit-path body (one mechanism, x§6.2); the removal reports its failure and the run's status stands | CR2-C2 |
| (new, F3's measurement) | `c3c0b74b` | `_ctl_run`'s watchdog is a bash subshell that never `exec`s, so it keeps the copies of the caller's stderr (and stdout) bash saves around the silencing group and around a standing check's redirected `_ctl_run` call: a run killed by SIGKILL holds its caller's stream open for up to 30 s | the watchdog `exec`s (through `env -u SHELLOPTS -u BASH_ENV`) a fresh bash running the stop procedure after its sleep and marker (bash's saved descriptors are close-on-exec); both children close fd 9 | CR2-C2 |
| ER-A5-1 | `0d411df2`, `173b81dc` | the #519 memo's banner (:28–:36) names `_gopt_ok` / `_ctl_need` and a pre-C1 ratchet population | O-1 (A), decided: split, then the banner | CR2-D |
| ER-A5-3 / ER-A3-2 (MIN) | — | ci.yml :180–186's timeout comment | **landing-tracked (§9.1 step 2)**: step 2 rewrites that comment to the runner method; verified at step 2 | none |

**Prereq split (CR2-0)** before CR2-A/B: mutations.sh is 899 lines against its 900 stop line, and both CR#4 and CR#7
edit it (§8).

**Ratified surfaces this memo changes** (why it goes to plan-review): (1) CR-C1's rule 1 (`…-cr-plan.md` §3.2) — its
population stops feeding the ratchet and the literal rule (§2); (2) the always-on anchor rule's definition of
"fresh", in force since `976e1430` (corpus §6.1) — two clauses added (§3); (3) `_MUT_PARTS` gains a part (§8);
(4) landing X10's statement that the wire is edited by #527 only in `8f4ad3c8` (x§6.3: rewritten from the measured
`git log` of the wire on #527's range, which then holds `8f4ad3c8`, CR2-D's banner commit and CR2-C2's commit);
(5) **CV2 / D5**: `…-cr-sweep.md` §F row 12 assigns the #519 memo's split to "the PR that next touches the memo … as a
standalone prereq PR", DECISIONS-R7 D5 forbade the touch in the CR round, `…-cr-plan.md` :819–:835 records R10, R12
and R13's trigger as "the PR that next touches the #519 memo", and residuals.md R10/R12/R13 carry the same gap and
trigger. O-1 (A) does the split inside #527 as a standalone commit; CR2-D deletes the three residual rows. §F row 12
(its column is "state at `2f1ca601`", a frame-tagged record, not a live forward ledger) and `…-cr-plan.md` :819–:835 are
closed records: each gets the dated corrective note §7 describes, not a body rewrite (G36);
(6) **CR-S1's watchdog** (`c3c0b74b`, the silenced launch of `_ctl_run`'s two groups and its tostop evidence,
ctlrun :329–:343): C2 rewrites the watchdog's body (it `exec`s the stop procedure through `env -u SHELLOPTS -u
BASH_ENV`) and the launch lines gain `9>&-` and the reap-set calls; the silencing group and `< /dev/null` stay, so the
tostop evidence is about the same launch shape (claim row CR11-S); (7) **the standing checks**: "the three standing
checks" (G11 of the CR round) become four (`_og`, x§6.4); the current-tense sites, from `/usr/bin/grep -rniE 'three
standing'` over the K2 docs, the umbrella and the parts, each read: controls :103, :310, ctlrun :136, harness :750,
records :153, residuals :116, landing :259, design memo :507, :515 and :606 (W2's row); the closed plan `…-cr-plan.md`
:203, :450, :548 gets the corrective note (§7); the dated corpus row :363, the CR round's ledger rows (`…-cr-sweep.md`
:131, :147, :250, :298, :393, :579, :652, :680, :684) and the records (`…-impl.md` :503, `…-impl-cr.md` :114, :140,
:659) are provenance (27 hits in all). The `_wm` check's hang row gains one assertion (TERM before KILL), and its
"outlived" judgment reads the call's own confirmation (`_ctl_left`, x§6.2) instead of its own poll.

**The boundary with the user's carve `#11-trip-wire-liveness-bound` (2026-09-28; `…-pr527.md` :409–:419, design memo
:455–:457, residuals R9).** The carve took draft 12's whole-run **time bound** out of #527: a run-wide process group,
a parent that re-runs the wire under it, and its seven defects. CR2 adds **no time bound, no re-run and no run-wide
group**: what it changes is how the existing run ends when it is signalled or when its scratch removal fails (CR#11,
CR#12, existing code of `8f4ad3c8`) and what `_ctl_run`'s existing groups hold (`c3c0b74b`). Three of the carve's known
edges touch the same lines, and each is measured or argued in x§6.5: trial nesting × the exit
path's kill (measured: a TERM during an opt-in trial), `stty tostop` (claim row CR11-S on the unchanged launch
shape), and bash 3.2's masked rc 0 (the signal's status now comes from `_k2_sig`'s own `exit`, measured 129/130/143 on
b32).

---

## §2 CR#4 — the census reads one scan for two checks with opposite safe directions

### §2.1 The defect, reproduced

- **The finding's false green, at wire level (M).** `cr2/bin/cr4-edit.py` inserts, after controls.sh's `_ev_lbl=`
  line (an inserted block, no anchored line edited), two labels used only in `if false` echo lines:
  `echo "!! the window's own check ($_v03a_lbl) isn't asked"` and `echo "!! see slot #11-x ($_v03b_lbl) …"`.
  PASSED, rc 0, both shells (`cr2/logs/cr4-q.{b53,b32}.log`). The same lines without the apostrophes and the `#`:
  `CONTROL FAILED (every label the run used is named by a mutation record): 20 labels have no mutation record, against
  a ratchet of 18`, rc 1, both shells (`cr4-plain.*`). At scan level, `cr2/bin/cr4-parse.sh` over the shipped awk
  (extracted mechanically, as `crc1-fix3/oracle.sh` does): the two quoted lines count nothing, the two plain lines
  count their label (`cr2/logs/cr4-parse.txt`).
- **The reverse error (M).** The same scan counts text bash does not expand: an apostrophe in an earlier double-quoted
  string pairs with the opening quote of later single-quoted data (`echo "it's"; echo '$_v03c_lbl'` counts
  `_v03c_lbl`), and a ` #` inside double quotes cuts the line before a quoted here-document's operator, so the body is
  scanned (`echo "a #b" <<'EOF'` then `$_v03d_lbl` counts it) (`cr4-parse.txt`). At wire level
  (`cr2/bin/cr4-over.py`): a new label whose only "use" is such a body, plus one record for it anchored on an inserted
  line → the ship run PASSED, rc 0, both shells (`cr4-ovship.*`: the always-on orphan check misses it), and the opt-in
  run filtered to that record (the `crc2-fix/Bc-t2.sh` method) → `MUTANT 252 (cr2 probe d) SURVIVED`, `252 entr(ies),
  1 not killed as named`, rc 1, both shells (`cr4-ovoptin.*`; its generated-set floor red is the filter's stub).
- **The oracle, extended (M).** `cr2/bin/oracle2.sh` = `crc1-fix3/oracle.sh` plus a generated part C: double-quoted
  strings with prefixes and suffixes drawn from the characters rule 1's steps act on (`'`, ` #`, `\"`, `$'`) around
  each form, then single-quoted data, escaped data and quoted here-documents after such a string. 395 cases at the
  frame (`cr2/logs/oracle2-head.out`): C1's 131 reproduce exactly (114 agree, 13 false red, 4 syntax-excluded,
  0 false green); the 264 generated ones give 201 agree, **45 under-counts and 18 over-counts**. The 18 over-counts are
  two classes only: a ` #` inside an earlier double-quoted string on the line of a quoted here-document operator (12),
  and an apostrophe inside an earlier double-quoted string on the line of later single-quoted data (6).

### §2.2 Root cause

`.vlabels` (the scan's names' values) feeds three readers whose safe directions differ:
- the **orphan check** (each record's needle is a label the run used): an under-count is red (the record orphans),
  an over-count is a miss;
- the **ratchet** (`.bare` = labels with no record, exactly `_MUT_UNRECORDED_MAX`): an under-count is a miss (a new
  label with no record is not counted, so "ADDING A LABEL MEANS ADDING A RECORD", mutations.sh:188, fails), an
  over-count is red;
- the **literal rule** (a `_control` label equal to a variable's value is red): an under-count is a miss.

C1's oracle measured only the first direction ("FALSE-GREEN" = counted where bash expands nothing), so "every error is a
false RED" (mutations.sh:552) was true of the orphan check and false of the other two. The argument recorded at
`…-impl-cr.md` :625–626 ("every step only drops text, so the scan can lose a real expansion … and can never invent
one") is also wrong for the orphan direction: the scan counts whatever it does not drop, so text it fails to drop
(the two classes above) is counted. No line-based scan of bash can be made under-approximate by construction: the
multi-line single-quoted strings (the awk programs) need a quote tokenizer, which is the enumerated-shapes parser R7
rejected (DECISIONS-R7 "S1 REDIRECT"). **(M)** the obvious hardening fails the ship: a greedy first-to-last `'` drop
plus raw-line here-document detection (`cr2/bin/scanV.awk`) loses every expansion of `_ph_lbl` and five of
mutations.sh's six, because shipped messages hold apostrophes inside double quotes and rule 1's own comment
(mutations.sh:550) spells `<<'W'` (`cr2/logs/scanV-counts.txt`: harness 20 → 18, mutations 6 → 1, mutgen 2 → 1 expansions; `_ph_lbl` lost).

### §2.3 The fix: each check reads the population whose error is red for it

1. **The ratchet and the literal rule read the spelled names.** `$_lbd_names` (controls.sh:160: `grep -ho
   '_[A-Za-z0-9_]*_lbl'` over the wire and every part, the record table included; `_lbd` asserts each defined and
   non-empty) is already computed and consumed by mutations.sh (`_mut_run`'s excuse). Its values go to
   `$_VFY/.mvalues` by the same indirection as `.vlabels` (mutations.sh:566, `printf '%s\n' "${!_cen_v}"`), over
   `$_lbd_names` instead of `$_cen_used`, one line beside :566 (in the new part after CR2-0; `cr2/bin/cr4-proto.py`);
   the literal rule's `isv` reads `.mvalues`, and `.bare` = (`.mvalues` ∪ the registry's labels) −
   needles. **Its error is red by construction:** every name the expansion regex can capture is a match of the same
   token regex (a match cannot start before the name, which follows `$` or `{`, and the longest one ends where the
   name does, at a non-identifier character), so spelled ⊇ expanded; an extra spelled token is asserted defined by `_lbd` (red if not) and then
   counted bare (red if unrecorded). Its one blind spot is a label variable whose name is never spelled (constructed),
   which `_lbd` cannot assert either; none exists. Instrument: `/usr/bin/grep -nE 'printf -v|eval|declare'` over the
   wire and every part, then `/usr/bin/grep '_lbl'`: 7 hits at the frame, each read — harness.sh:631 (`declare -f
   _fgit_postconditions | grep -o …`, a reader of spelled names, constructing none) and six prose uses of the word
   "declared" (controls :115, :116, :117, :249; mutations :446; records :160), none a construction. **(M)** at the frame both scans name the
   same 31 names (`cr2/bin/pops.sh`, `cr2/logs/pops-head.txt`), so `.bare` stays 18. Mechanism prototype
   (`cr2/bin/cr4-proto.py`): the finding's lines → `20 labels have no mutation record, against a ratchet of 18`, rc 1,
   both shells, naming `cr2 probe a` and `cr2 probe b` (`cr4-pq.*`); ship PASSED, rc 0, both shells (`cr4-pship.*`).
2. **The orphan check keeps rule 1's scan** (`.labels` = `.vlabels` ∪ the registry's labels), unchanged, with its
   records (GC-1, the trailing-comment, single-quoted and escaped records 220–222 by index). Its comment, mutations.sh
   :546–556, is rewritten to the two-sided truth: an under-count is a false red (the record orphans) and, since CR2,
   can no longer hide a label from the ratchet; an over-count (the two classes measured, and a line inside a
   multi-line single-quoted string, unmeasured by the oracle) leaves a record non-orphan here, and that record's trial
   in the opt-in run is then not killed as named, red (§2.1, `cr4-ovoptin.*`) — BS-4's class (`…-cr-plan.md` §6).
3. **The comments whose referent CR2-A changes** (they state the census's population as the names the parts EXPAND,
   which after CR2-A is true of the orphan check only). **Instrument:** `/usr/bin/grep -nE
   '_lbl.{0,40}(EXPAND|expand)|(EXPAND|expand).{0,40}_lbl'` over the wire and every part, at the frame: 10 hits, each
   classified —
   - **rewritten** to the two populations (the orphan check: the expansions; the ratchet and the literal rule: the
     spelled names): controls :99 (in :98–:100, "the expanded `_*_lbl` variables, against the records"); mutations
     :18 (in :16–:18, "the `_*_lbl` variables the text expands, against this file's records, and reds when they
     drift"), :59 (in :58–:61, "a `_*_lbl` variable the text expands … ratcheted"), :189 (in :188–:189, "an expanded
     `_lbl` name … the ratchet below is what makes that true"), :204 (in :200–:207, the correspondence's population
     and the ratchet), :515 (in :510–:518, the census block's heading: "the values of the `_*_lbl` names the TEXT
     EXPANDS"); records :331 (in :330–:333, "the `_*_lbl` names the wire or a part EXPANDS outside this table");
   - **rewritten as §2.3 item 2 says** (rule 1's own comment, two-sided): mutations :546 and :551 (in :546–:556);
   - **unchanged**: records :375 (a record's comment on how the scan treats a single-quoted span: the orphan check's
     scan, which CR2-A keeps).
   Plus mutations :24–:25 ("`$_lbd_names` … the census reads the values of those expanded outside the record table"),
   named by the review: the instrument misses it because the verb and the name are further apart than its 40-character
   window; rewritten. Plus two sites outside the instrument's reach: controls :76–:77 ("the census … counts the
   `_*_lbl` variables the wire or a part EXPANDS", name and verb on two lines) and design memo :700 (a doc) (row 9: "every `_*_lbl` label the wire or a part expands … is named by a
   record except the `_MUT_UNRECORDED_MAX` (18) unrecorded ones"). The mutations lines from :443 on move to the new
   part at CR2-0 and are edited there. They are in CR2-A's files (§11), and §11's sweep rule re-classifies every hit of
   the groups whose referent CR2-A changed, not only new hits.
4. **Not done, with reasons:** no hardening of the scan (§2.2, (M) ship-red); no tokenizer (altitude, R7).

### §2.4 Pins, budget, claims

- **New record R4a** (pins clause 1), **(M) prototyped as a real record** (`cr2/bin/r4a-rec.py`, one line before
  `MUTANTS`):
  `controls:s/^\(_pe\)\(_lbl\)="no exec-path override reaches the fixture git"$/&; \1r4a\2="cr2 r4a probe"; : "it's $\1r4a\2's"/`
  with needle `$_cen_lbl`'s value (`every label the run used is named by a mutation record`). It appends, to
  controls.sh :88 (`_pe_lbl=`, a label line before `_lbd` that no record anchors: the one record anchored on a
  definition line is records.sh :301, on `_fws_lbl=`), a new label and its only use inside a double-quoted string
  holding an apostrophe pair. **The new name is built by sed backreferences from the anchor line** (`\1r4a\2` from
  `\(_pe\)\(_lbl\)`), so records.sh spells no new `_*_lbl` token: records.sh is one of `_lbd_files` (controls :155–:159),
  and a spelled `_per4a_lbl` there would be "used but not defined", exit 2, on every ship run. **(M)** the token set
  `grep -ho '_[A-Za-z0-9_]*_lbl'` over records.sh is identical with and without R4a; the applied expression writes
  `_pe_lbl="…"; _per4a_lbl="cr2 r4a probe"; : "it's $_per4a_lbl's"` (one line); its end is the line's literal end, so
  it is fresh under all three clauses of §3 (`cr2/bin/fresh.sh` over the clone: R4a not listed, `cr2/logs/f6-fresh.txt`).
  With the CR2-A mechanism (`cr4-proto.py`) and R4a: ship PASSED, rc 0, both shells (`cr2/logs/f6-ship.{b53,b32}.log`);
  the opt-in filtered to R4a (`cr2/bin/r2-optin.sh`) → `252 entr(ies), 0 not killed as named`, both shells
  (`f6-opt.*`; the log keeps the count, not the kill's text). Survive arm (the frame's population, no
  `cr4-proto.py`): `MUTANT 252 (every label the run used is named by a mutation record) SURVIVED`, both shells
  (`f6-surv.*`).
  **The rule this states, for records.sh: records.sh spells only defined labels.** A record that needs a new label name
  builds it from its anchor with a backreference. Stated in records.sh's header beside the comment-syntax rule (CR2-A).
- **Claim row CR4-L** (the literal rule's population; latent, the populations are equal at the frame): a `_control`
  whose literal equals the value of a name spelled only inside such a string → `is a \`_*_lbl\` value passed to
  \`_control\`` present with the fix, absent with `isv` read from `.vlabels`; both shells. No record: its needle would
  be `$_cen_lbl`'s, which the ratchet's red kills either way (the same label), so a record cannot separate them.
- **Budget (code and comment lines alike, §8):** in the part CR2-0 creates: about +8 lines (the `.mvalues` line, two
  awk arguments, the comment's net growth); records.sh +1 (R4a) and +1–2 header lines (the spelling rule). Docs edited
  in place: design memo row C (:620) and row 9 (:700), residuals R13 (:120, rewritten: the ratchet's population is the
  spelled names again, see §7). **`…-cr-plan.md` :413 and :733 (BS-5a) are not rewritten**: `…-cr-plan.md` is a closed,
  approved plan, and a current-tense claim in it that a CR2 fix falsifies gets a dated corrective note in the banner
  form D5 set for the #519 memo (a short list under its Status paragraph naming each falsified line, what is now true
  and the commit; +1 line per item), never a body rewrite (§7 lists the note's items). **`…-impl-cr.md` is not edited**: its body
  is a closed record; the correction of its :625–626 sentence is recorded in `…-impl-cr2.md` §T (CR2-A's entry), and
  `…-impl-cr.md`'s single end pointer (CR2-M) names `…-impl-cr2.md` §T as where its corrections are recorded.

---

## §3 CR#7 — what a fresh anchor is

### §3.1 The defect, reproduced

- **Duplicate occurrence (M).** `cr2/bin/cr7-edit.py` respells K2RE's first segment class to the equal set
  (`[^/[:space:]]+/(` → `[^[:space:]/]+/(`, wire:547) and re-anchors record 79 (records.sh:79) onto it, the finding's
  scenario. The always-on anchor check reds six expressions (records 93–96 with flag `2`, and the two other
  `…+/(`-anchored records) and passes records 89–92 and 97 (`cr2/logs/cr7.{b53,b32}.log`), whose mutants now edit the
  intermediate segment while their needles name the first (`cr2/logs/cr7-moved.txt`).
- **Restating replacement (M).** Record 81 (`s#^K2RE_PATH=.*#…#`) matches any value of `K2RE_PATH` and writes a fixed
  one; its mutant is byte-identical to record 80's (`cr2/logs/cr7-reanchor-identity.txt`, last block), so it adds no
  test and would keep "testing" the old value after an edit. Record 45 has the same shape.
- **Population (M).** `cr2/bin/fresh.py` (Python, real `/usr/bin/sed`, `LC_ALL=C`) and `cr2/bin/fresh.sh` (bash, sed,
  diff, awk through `ENVIRON`, the wire's own tools) apply the two rules below to every record expression at the
  frame: **25 of 263 not fresh**, the same 25 in both prototypes and on both shells (`cr2/logs/fresh-head.bsd.txt`,
  `fresh-head.sh.bin-{5,3}.txt`):
  - restating replacements: records 45, 55, 81;
  - an unescaped `.` in text meant literally, which the replacement restates: 71, 80, 101, 103, 104, 106–109, 134,
    135, 137, 224;
  - duplicate occurrence on the line: 89–92, 97 (twice and carry);
  - `g` over three occurrences (a respelled one silently drops out of the edit): 226;
  - an append whose output matches its own pattern again (`.*$` absorbs the appended text): 323, 329, 382 (twice
    only: these are false reds of the twice rule, fixed by anchoring the end on the line's literal text).

### §3.2 Root cause and the definition

The rule "each expression changes exactly one line" (mutations.sh:657–687) defines fresh by the *effect's size*, not by
*what the expression reads*. An expression is fresh only if the text it changes is still the text it was written
against, at the place it was written for. Two clauses, each a property of the expression and the current file, are
added to the one-line clause:
1. **One line** (unchanged): `diff` counts exactly one changed line.
2. **Position-unique:** applied twice (`sed -e x -e x`), it changes nothing more than applied once. Closes the duplicate
   occurrence: the second application reaches the next occurrence.
3. **Faithful (outside the changed span):** for every character of the changed line outside the changed span (the
   common prefix and suffix of the line before and after), the line with that one character replaced by `\001`, run
   through the expression, either is left unchanged (the expression reads that character) or comes out as the changed
   line with the same `\001` at the mapped position (the expression carries it). Anything else means the expression
   reads that character through a wildcard and writes something else in its place: it restates text it does not read,
   so a respelling there would be silently reverted. Closes, **for the characters outside the changed span**,
   whole-line replacement, unescaped dots, and a partial match among several sites. A deletion (`d`) has no changed
   line after it and restates nothing; only clauses 1 and 2 apply.

**Direction:** clauses 2 and 3 can only add reds; a record fresh under them is fresh under clause 1. Their false reds
(an append that re-matches, a coincidental shared character) are cured by anchoring more precisely, never by
loosening the rule. **What they cannot see** (BS-8 in §9, each with a measured example): text outside the edited line
(the other end of an address range, a hold-space source line as in record :418, a variable defined elsewhere), and a
wildcard **inside** the changed span. **(M)** `cr2/bin/f13span.sh` (a one-line target `k=abc` through `fresh.sh`):
`s/^k=a.c$/k=aXc/` is fresh (its `.` reads the character it replaces, inside the span), and on the target respelled to
`k=aYc` it writes `k=aXc`, reverting the respelling silently; `s/^k=.b/k=aX/` (its `.` reads `a`, outside the span,
and restates it) is `CARRY` (`cr2/logs/f13span.txt`). Clause 3 therefore does not claim whole-line replacement in
general: a replacement whose wildcard lies wholly inside what it rewrites is not seen.

**Cost (M):** the whole pass over 263 expressions, 1.0 s with clause 1 alone, 4.5 s with all three, both shells, on the
bash prototype under the load of two concurrent wire runs (`cr2/logs/fresh-countonly-time.{5,3}.txt`,
`fresh-time.{5,3}.txt`); the ship run is about 20 s. The slice re-measures it inside the wire (claim row CR7-T).

### §3.3 The fix

- **One function, `_mut_fresh <expression> <target>`**, in the part CR2-0 creates, setting `_mf_v` to `fresh`, `stale`,
  `count:<n>`, `twice` or `carry` (the first clause that fails, in that order); the anchor loop calls it per expression and keeps its messages for `stale` and
  `count:<n>`, with two new ones (twice: anchor it on text that occurs once; carry: spell what it keeps, `\(…\)` and
  `\1`, and escape a literal `.`). The variants for clause 3 are built in one awk pass (values through `ENVIRON`),
  run through one `sed`, compared in one awk pass: three processes per changed line (as `cr2/bin/fresh.sh`).
- **The records (M):** `cr2/bin/cr7-reanchor.py` rewrites the 24 non-fresh records in place (no records.sh line moves:
  45 and 55 by captures; the dotted ones by `\.`; 89–92 and 97 onto the unique `…+/(` context; 226 as one match with
  three captures; 323, 329, 382 with a literal end). Every re-anchored record's mutant is byte-identical to its old one
  and its needle unchanged, 24 of 24, and line 81's old mutant equals line 80's (`cr7-reanchor-identity.txt`); after the
  edit both prototypes report 0 of 262 not fresh (`fresh-reanchored.{sh,py}.txt`); ship with `_MUT_RECORDS_MIN=250`
  PASSED, rc 0, both shells (`cr7r-ship.*`). Identical mutants mean the 24 trials are the frame's; the slice still runs
  them opt-in (completion).
- **Record 81 is removed and its line kept as a `#` comment** (`_mutants` drops it, so the file keeps 470 lines). The
  reason, stated in the comment: records are identified by their records.sh line across the documents (the corpus
  rows, `…-cr-sweep.md`'s ledger and `…-cr-plan.md` cite records by line; `b3b64aa4` dropped a comment line for exactly
  this, `DECISIONS-R7.md`:831), so a removal leaves its line. The comment says what was removed and why (its mutant
  equals record 80's; it restated `K2RE_PATH`'s value) and names its corpus §6.1 remove row. It is not a shim: it
  documents a removal and is read by no code.
- **The self-test (the pin).** The rule is first applied to a fixed fixture written to `$_VFY` (for example the lines
  `a=1`, `x ab ab`, `k=vx`) with five rows whose verdicts are known: `s/^a=1$/a=2/` fresh, `s/^zz$/y/` stale,
  `s/=/:/` count 2, `s/ab/XY/` twice, `s/^k=.*/k=wx/` carry (the `x` outside the edit is matched by `.*` and
  restated). A disagreement is `CONTROL FAILED ($_af_lbl): …` (a new
  label in controls.sh's non-`_control` list, :102–:118: the comment :102–:107 and the definitions :108–:118). It runs **before** the `*.mutant.*` skip (mutations.sh:644),
  so a record that disables an arm is exercised inside its mutant; the real records' loop stays skipped there. It is a
  table check in G11's sense (it runs no `_ctl_run` row, so the count of standing checks — three, four after CR2-C2 —
  stays true), and its presence
  is pinned by its records (G11's precedent for `_cap_check`). It also pins mawk's and GNU sed's behaviour of clause 3
  on every machine: a variant pass that produced nothing would read every record fresh, and the carry row would red.

### §3.4 Pins, budget, claims

- **Records R7a–R7d**, needle `$_af_lbl`'s value, each disabling one verdict arm of `_mut_fresh` (stale, count, twice,
  carry): KILLED AS NAMED by the self-test's row; survive arm: that row removed → SURVIVED; both shells. The stale and
  count arms are pinned for the first time (no record could be killed by the unlabelled anchor messages, the
  `crplan-r2/logs/M-slot-guard.txt` class).
- `_MUT_RECORDS_MIN` 251 → 250 (line 81) → 254 (R7a–d), "say why and LOWER the floor in the same edit"
  (mutations.sh:705–707) satisfied by the commit message and corpus §6.1's remove row. `_MUT_UNRECORDED_MAX` stays 18
  (`$_af_lbl` has four records).
- **Budget:** about +45 lines in the new part (function, self-test, messages); records.sh +4 lines before `MUTANTS`
  (:469; the lines after it, :469–:470, are cited by no K2 document or part: `/usr/bin/grep -rnE
  'records(\.sh)? ?:4[5-7][0-9]'` over docs/plans and the parts has one hit, `2026-06-b1.2d-transient-observers-plan.md`:37,
  `deliver_pending_mutation_records:469`, another subject); controls.sh +1 (`_af_lbl`). Docs: corpus §6.1 (one remove
  row, four add rows, totals), design memo §4 label table (+1 row `AF`), and the anchor rule's **current-tense**
  descriptions (§10 CR7-1). The corpus's dated rows that describe the rule as it was (:245–:247, a paragraph citing
  `mutations.sh:537–560` "at `8413a4db`", and :286, the `976e1430` re-shape row) are provenance and stay as written.

---

## §4 CR#9 — the plain-git retry derives its purge from git's list

- **Defect (M).** Full run with `GIT_OBJECT_DIRECTORY=/nonexistent-k2` exported: `!! CONTROL NOT EXERCISED (a caller's
  fsmonitor hook does not run): the hook did not run under a plain git call either`, rc 1, both shells
  (`cr2/logs/cr9-god.{b53,b32}.log`); the same with `GIT_COMMON_DIR` (`cr9-gcd.*`).
- **Root cause.** controls.sh:708 `unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE` is the hand list the wire itself
  forbids (wire:653–654), while `$_GIT_LOCAL_VARS` (wire:669, git's own list) is in scope.
- **Fix.** `( for _v in $_GIT_LOCAL_VARS; do unset "$_v"; done; cd "$_fsm_root" && env "${_fsm_cfg[@]}" git ls-files … )`:
  the whole list, configuration included, unlike `_git`'s (wire:675–676, which keeps `GIT_CONFIG*` for R97's
  `safe.directory` case): the plain call's fixture is the run's own, and the one configuration it asks about is given
  back by `env`. The reason goes beside the line. **(M)** mechanism prototype (`cr2/bin/cr9-proto.py`): PASSED, rc 0,
  under both variables and plain, both shells (`cr9p-{god,gcd,ship}.*`).
- **Pin: record R9**, two expressions: the loop put back to the three names, and an `export
  GIT_OBJECT_DIRECTORY=/nonexistent-k2` appended to the unanchored line `_fsm_root="$CTL/fsmon"`; needle `$_fsm_lbl`'s
  value. Expected KILLED AS NAMED; survive arm: the second expression alone → SURVIVED (the purge is what passes);
  both shells. The appended line must pass CR#7's rules.
- **Budget:** controls.sh ±0 code lines, +1–2 comment lines; records +1.

---

## §5 CR#14 — the control's output file is cleared before the launch

- **Defect (M).** `cr2/bin/cr14-edit.py ro`: right after the `k2` control (which leaves `K2: a` in
  `$_VFY/.control_out`), the file is made read-only and a control that would fail if it ran (`_control "$CTL/clean" 1
  "K2: a" "cr2 stale probe"`) follows. Its label appears in no `CONTROL FAILED` line: the child's redirect could not
  open the file, the child exited 1 without running, and `_out` was read from the stale file (the run's only red is the
  ratchet's, which lists the probe's new label) (`cr2/logs/cr14-ro.{b53,b32}.log`). With the file writable the probe
  runs and reds, `expected exit 1, got 0` (`cr14-rw.*`).
- **Root cause.** ctlrun.sh:307/:350/:374: the verdict is read from a file the launch may not have written; the
  timeout marker beside it is removed first (:344), the output is not.
- **Fix.** On :344: `command rm -f "$_r_wmark" "$_out_f" || { echo "!! CONTROL FAILED ($_r_for): a previous run's output
  or timeout marker could not be cleared" >&2; return 1; }`. A failed open now leaves no file, so `_out` is empty and
  the expectation fails. **(M)** prototype (`cr14-edit.py ro fix`): the probe runs and reds as in `rw`, both shells
  (`cr14-rofix.*`).
- **The globals half of the finding: what the readers depend on, stated in the RETURN CONTRACT (:228–:240), not a
  reset.** Every reader of `_rc`/`_out` outside `_ctl_run`, in every expansion form (instrument: `/usr/bin/grep -nE
  '\$\{?_(rc|out)([^A-Za-z0-9_]|$)|unset _(rc|out)'` over the wire and the parts, each hit read), at the frame:
  1. `_control` (ctlrun :423–:432): reads both after `_ctl_run` returned 0 (`[ "$_cr" -eq 0 ] || return 1`, :419).
  2. the relcwd block (controls :538–:547) and the fsmonitor block (:703–:706): read both in the `elif`/`else` of
     `if ! _ctl_run …`, i.e. after a 0 return (`if !` sends every non-zero, 2 included, to the then-branch).
  3. the `_wm` standing check's abort row (controls :350): `${_rc:-0}` is read **whatever the return was**; a
     non-zero return is red there by its own assertion (:349, "the run returned $_wm_r, not 0"), so a stale `_rc` can
     only be read beside a red.
  4. the `_cap` standing check (controls :443, :449): `unset _rc` before each row, then `${_rc+ran}` — "a run was
     made" **is** "`_rc` is set". It depends on `_ctl_run` assigning `_rc` only after a launch (:372) and leaving it
     untouched on every return before one (the gate's 1, a decline's 2, a malformed channel's 1).
  Inside `_ctl_run` the readers are its own, after its own assignments (:378, :380, :397, :398). The one other hit is not
  a reader (records.sh :286, a record's text): 19 hits at the frame, each read.
- **The contract text (r3), one rewrite of :228–:240 covering every way `_ctl_run` ends after C1 and C2** (C1 writes
  the clauses its fix adds, C2 the ones its fix adds; both slices list ctlrun :228–:240 in their files, §11):
  - **before a launch** it returns, leaving `_rc`, `_out` and (C2) `_ctl_left`, `_ctl_wd_pg` as they were: 1 when the gate refused (an untrusted window,
    an undeclared group's member, a failed fixture), a channel argument is malformed or reserved, the evidence
    directory could not be made, or (C1) the previous run's output or timeout marker could not be cleared; 2 when the
    gate declined the run for a capability this machine lacks;
  - **after a launch** every return has `_rc` and `_out` assigned from that launch (:372, :374): 1 when the watchdog
    killed the run (its marker) or a delivered channel is missing from the evidence (NOT EXERCISED); 0 otherwise, the
    caller's comparison being the verdict; and (C2) `_ctl_wd_pg` names the watchdog's group and `_ctl_left` the groups it
    could not confirm empty (their one reader, the `_wm` check, reads them as it reads `_rc`: beside its rows' own reds);
  - **it does not return** (C2) when the reap set cannot record or drop one of its groups: it stops the call's groups,
    writes the cause to fd 9 and `exit 2`s the shell it runs in — at top level the run ends "decided nothing"; in a
    standing check's subshell that subshell ends, and the check reds under its label ("the check ended before its
    verdict"), the cause on the run's own stderr (x§6.2). A failed write therefore never reaches
    the code between `_rc=` and `_out=` (r2's `_k2_reap_del … || return 1` returned there, with `_out` stale: G1).
  This is what the `_cap` check reads (unset ⇒ no run made) and why `_rc`/`_out` are not reset at entry (a reset would
  make "a run was made" unanswerable there); a caller reads them only after a 0 return, except the standing checks
  above, whose own assertions red a non-zero return beside the read. The finding's "a decline returning 2 would reach
  the `elif`" is not so (`if !`, item 2). Claim row CR14-G pins the reader list and the contract **at C1's head and
  again at C2's head**.
- **Pin: claim rows** (no record): CR14-a, the read-only stale file of the repro, the removal taken out → the probe
  passes unrun, with it → red; CR14-b, a directory at `$_VFY/.control_out` → the new red under the control's label;
  both shells. **Unpinned by a record (reason written beside the code):** the only constructible open failure that
  leaves the stale file readable is a read-only file, which root writes anyway, so a record would survive on a root
  machine (BS-7's class) without a capability declaration; EMFILE also breaks the parent's own read, so it does not show
  the hazard.
- **Budget (code and comment lines, §8):** ctlrun.sh 436 → at most 438 at C1 (the fold on :344 is the same line; two
  comment lines for its reason; the contract's C1 clause inside the :228–:240 rewrite); the stop line is 439 for C1 and
  C2 together (§8).

---

## §6 CR#11, CR#12 and the watchdog's kept stream — the run's exit (in the companion)

The exit path is edge-dense, so its section is the companion `…-cr2-plan-exitpath.md` (split at r3 at the section seam,
touch-time; its numbers kept as x§6.0–x§6.6, the 1000-line rule otherwise passed here). The fix in one sentence: HUP,
INT and TERM become `exit 128+n` with a line on the stderr saved at startup (fd 9); the exit bodies ignore them and run
with errexit off; `_ctl_run`'s groups are recorded as files in the run's reap set, reaped by one stop procedure (TERM,
1–2 s, KILL) shared with the watchdog, which `exec`s and so holds no saved descriptor; a failed scratch removal is
reported and the status stands. Its records are R11h, R11i, R11t, R12, R12e, R11w, R11d, R11a, R11x, R11e, R11g,
R11r and R11n (x§6.4); its guard rows are in §9 and its claim rows in §10 of this memo.

---

## §7 ER-A5-1 and the ci.yml MINs

- **ER-A5-1 (O-1 (A), decided), CR2-D.** Defect, read at the frame: the #519 memo's banner :28–:36 (lines `0d411df2` and
  `173b81dc` added) names `_gopt_ok` and `_ctl_need` (0 hits in the wire and the parts: `/usr/bin/grep -c` over
  `.claude/tools/webref-generic-core-trip-wire*.sh`), and states the ratchet's population as "the value of every
  `_*_lbl` name the parts spell". After CR2-A that population is the spelled names of the wire **and** the parts, the
  record table included, so R13's correction (written for C1's expansion population) is itself superseded.
- **The split commit** moves §10–§11 (:930–:1414) verbatim to a companion
  (`docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire-review-rounds.md`, name decided in the slice), leaving a
  pointer at the #519 memo's §10 heading that names both moved sections ("§10 and §11 — every §10.x and §11.x this
  memo cites, e.g. :9, :567, :736, :776, :783, :796, :811 — are in `<companion>`"); it changes nothing else. The companion opens with the no-spec-surface §0 (as
  the umbrella's row :110 says of the #519 memo, its `preflight` exits 1 by design) and **one sentence on how a bare §
  in the moved text resolves**: "§10 and §11 keep their numbers, so a bare §10.x or §11.x resolves here; every other
  section reference without a file name is to `2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`" (the landing's
  header is the precedent). Measured at the frame: the moved block holds 75 `§<n>` occurrences —
  `awk 'NR>=930 && NR<=1414' <memo> | /usr/bin/grep -oE '§[0-9]+' | sort | uniq -c` — 42 to §10/§11 (they resolve in
  the companion) and 33 to §0, §2, §4–§9 (they resolve in the parent); the sentence governs the ones without a file
  name, and one that names its file resolves by that name. The body stays verbatim.
- **The banner commit**: the rewrite — k2 posed on every machine through `goptgrep`, declined nowhere (R10); the FIFO
  and permission controls red unless declared (R12); the census's two populations (§2.3, replacing R13). **Every line
  reference in the banner states its base** (F33): the banner's existing references (L5, L61, L278–280, L675–677,
  L683, L747–748, L816, L866–868) are `e8f78896` numbering and stay so labelled ("the lines are as of `e8f78896`",
  banner :16); the references into the moved text (L1212–1214, L1218, L1220, L1281–1282, L1360, and R12's :1274,
  :1391) become section references into the companion with its own line numbers at the banner commit (`<companion>
  §11.1 :<n>`), stated as such. Then R10, R12 and R13 are deleted from residuals.md and the range follows (below).
- **Referrers into the moved text, re-pointed in the banner commit.** Two instruments, re-run at the commit's head over
  the wire, the parts, `docs/plans/*k2-fgit*`, the umbrella, `scripts/` and `.github/` (read at the frame):
  (1) `/usr/bin/grep -rnE 'Ai-wire-k2-trip-wire|plan memo §1[01]'`; (2) the "parent" forms,
  `/usr/bin/grep -nE "[Pp]arent('s)?[^|.]{0,40}§1[01]|[Pp]arent D[0-9]+"` (9 hits at the frame). Each hit is
  classified **current** (re-pointed to the companion), **hist** (left as written) or **other** (names the #519 memo
  itself, not the moved text: unchanged):
  - current: the wire :323 (§11, D1), :403 (§11, D5), :482 (§11, D3), :711 (§11.6), :750 (§11, D2), :1190 (§11, D4) —
    each "plan memo §11…" names the companion; controls :677 (§11.1); fixtures :224 (§11, D10), :564 (§11.2);
    umbrella :22 (its pointer "§11.1 of `…-Ai-wire-k2-trip-wire.md`"); design memo :67 ("parent §10.6"), :78 ("the
    parent's §11.1 rule"), :132 ("parent §11.7 row 7"); residuals :113 (R6's "(parent D7)"); landing X10's cites into
    the #519 memo (§5 :549, §8 :818: not moved, but shifted if the banner's line count changes, re-measured at the
    banner commit);
  - hist (a closed round's record or the provenance list, which describe what was cited when they were written; they
    resolve through the pointer the split leaves at §10's place): reviews :116 (R2-4's row), :214 ("R6, parent D7"),
    :566 (the C2 round's table of part references, frame-numbered), provenance :775 (item 17, "parent §10.6"); and
    reviews :518–:525 (a closed round's instrument and its finding that "Parent §10 is cited by live parent §11"),
    kept as run with one sentence saying §10–§11 moved to the companion at CR2-D's split commit (the command's file is
    named as of `706049ad`); `…-cr-plan.md` :819 (under its corrective note, below); `…-cr-sweep.md` :24, :780, :877
    and `…-impl.md` :547 (a closed round's text and its instruments as run);
  - other: controls :12 (its §8), design memo :34 and reviews :461 (which file "parent" is), landing :372 (the
    banner's lines 14 and 25, before the banner: re-read at the banner commit), umbrella :110 (registration, below),
    residuals :117, :119, :120 (R10, R12, R13: deleted by this commit).
  The current ones are CR2-D's files (§11). One issue, one way: no referrer is resolved through `…-reviews.md` §8.3's
  table instead.
- **The umbrella:** row :110 (A-i-wire, the #519 memo's row) registers the companion beside the memo, unconditionally
  (both files' `preflight` exit 1 by the same declaration); row :22's pointer is re-pointed (above). Row :23
  (A-i-wire-fgit) enumerates the slice's files by role without a count, and CR2-M adds CR2's seven to it (`…-cr2-plan.md`,
  its companion `…-cr2-plan-exitpath.md`, `…-cr2-sweep.md` with its instrument companions `…-cr2-instruments-probes.md`, `…-cr2-instruments-texts-r2-r3.md` and `…-cr2-instruments-texts-r4-r5.md`
  (split from it at CR2-M under the 1000-line rule), `…-impl-cr2.md`; each opens with the no-spec-surface §0);
  row :111 registers the same seven, "All thirteen files" → "All twenty files" (CR-U). The #519 memo's companion is
  not in :111's count (it is that memo's, row :110).
- **`…-cr-plan.md`'s corrective note (G36's rule, §2.4).** `…-cr-plan.md` is a closed, approved plan; its current-tense
  claims that CR2 falsifies get a dated note under its Status paragraph, one line each, added by the slice that
  falsifies them: :413 and :733 ("never a false green" for the census population: CR2-A); :203, :450 and :548 ("the
  three standing checks": CR2-C2); :819–:835 (R10, R12, R13 carried "until the PR that next touches the #519 memo":
  CR2-D, which is that touch). The body is not edited. `…-cr-sweep.md` gets the same note under its header paragraph
  for §F row 12 (CV2's "the PR that next touches the memo … standalone prereq PR": split inside #527, CR2-D).
- **The residual range.** After CR2-D's deletion it is not contiguous: {R1–R9, R11}, spelled `R1–R9 and R11` (bare)
  and `"R1"–"R9" and "R11"` (quoted) at residuals :7 (quoted), design memo :31, :649 (bare), :56 (quoted). CR2-C2 then
  adds R14 and R15 (the exit path's two windows, §9), and the range becomes `R1–R9, R11, R14 and R15` /
  `"R1"–"R9", "R11", "R14" and "R15"` at the same four sites. Residual numbers are not reused (they are cited by number
  elsewhere), so R10, R12 and R13 stay retired.
- **Order:** CR2-D runs **before** CR2-C2, so C2's X10 sentence is written from a `git log` that already holds CR2-D's
  wire commit.
- Pin: none (prose); its claim rows are CR-D1/D2/D3 (§10).
- **ER-A5-3 (MIN, ci.yml :180–186):** landing-tracked (§9.1 step 2).
- **ER-A3-2 (MIN, ci.yml :180–186):** landing-tracked (§9.1 step 2).

---

## §8 Budgets, and the prereq split CR2-0

Every figure counts **code and comment lines alike** (`wc -l`); a "stop line" is the count a slice may not pass without
returning to the driver.

| part | frame | budget / stop line | CR2's growth | after |
|---|---|---|---|---|
| ctlrun | 436 | **439** | C1 ≤ +2 (§5: the fold on :344 is the same line; two comment lines; the contract's clause inside the :228–:240 rewrite); C2 ≤ +1 (code −1: the watchdog's three lines become two, the reap-set calls on existing lines, the call's end one `_k2_end` line; comments: the `set -m` paragraph :317–:328 and the watchdog paragraph :351–:366 are rewritten in place to the stop procedure, `exec` and `env -u`, net ≤ +2) | ≤ 439 |
| harness | 886 | 919 | none | 886 |
| mutations | 899 | 900 | −305 + 4 (CR2-0); the trap string edited in place | ≈ 598 |
| new part (CR2-0) | — | 450 | 305 + header ≈ 330; §2 +8; §3 +45 | ≈ 383 |
| controls | 774 | 1000 | `_af_lbl` +1; §4 +2; §6: `_og_lbl` +1, the reap directory +1, the signal loop +4, R12's control +1, the `_wm` probe and assertion +2, the fourth standing check ≈ +55 with its four rows and comment, comments ≈ +5 ([r5p] measures +61, J1's probe included) | ≈ 848 |
| fixtures | 698 | 1000 | §6 +5 | ≈ 703 |
| records | 470 | 1000; no cited line moves | +19 records (R7a–d, R4a, R9, R11h/i/t, R12, R12e, R11w, R11d, R11a, R11x, R11e, R11g, R11r, R11n) and their comment lines before `MUTANTS` (:469); +1–2 header lines | ≥ 491 |
| wire | 1286 | **1376** = the slice base's `wc -l` + 90 (O-2, x§6.3; r3's +74 → +90 for code +11 and comments +5, driver-accepted, J22) | the exit block +43 code lines ([r5p]: +45 with its two marker lines), plus the comments that state its reasons (the :405–:421 comment rewritten; ≈ +45) | ≤ 1376 |
| design memo | 755 | 760 | header +1 (CR2 files), §13 +1 (successor), §4 rows AF and OG +2 | 759 |
| `…-cr-plan.md` | 974 | 1000 | the corrective note: a heading line and one line per item (§7), ≈ +7 | ≈ 981 |
| residuals | — | — | R10, R12, R13 deleted (CR2-D); R14, R15 added (CR2-C2) | −1 row |
| `…-impl-cr.md` | 796 | closed | the §S end pointer +1 (CR2-M); body unchanged | 797 |
| #519 memo | 1414 | split (CR2-D) | −485 + pointer; banner rewritten | ≈ 930 |

**Where the reap set's writers live.** `_k2_reap_add` and `_k2_end` (12 lines) are called only from `_ctl_run`, but stay
in the wire beside `_k2_reap`: the format (one file per group, named by its id, read as a number > 1, holding its
record's `$SECONDS`) then has one home with its one reader, which must be in the wire (x§6.3); and ctlrun, at 436 + C1's
+2 under its stop line 439, has no room for 12 lines.

**CR2-0, the seam.** mutations.sh already draws it in its own comments: "The always-on half" (:450) and "The opt-in
half" (:749). The always-on half — `_mut_cap_member` (:443–:449), `_mut_correspondence` (:450–:732) and
`_mut_census_end` (:733–:747), 305 lines — moves **verbatim** to `.claude/tools/webref-generic-core-trip-wire.correspondence.sh`
(named after the function it holds; `_MUT_PARTS` word `correspondence`, so a record can target it as
`correspondence:`). mutations.sh keeps the format's readers (`_mut_parse`, `_mut_src_for`, `_mut_target`,
`_mut_apply`), the trial and the opt-in run, and sources the new part beside records and mutgen (:122, :134) with the
same existence check (exit 2). The split's other edits, all in the same commit:
- `_MUT_PARTS` (controls.sh:124); both parts' WHAT IT CONSUMES / WHAT IT DEFINES;
- the one record that targets mutations.sh, records.sh:360 (its anchor, mutations.sh:566, moves), re-prefixed
  `correspondence:` (**(M)** `cr2/logs/mut-anchors.txt`: it is the only `mutations:` record);
- **mutations.sh's positional references across the seam** are rewritten to name the other file. Instrument:
  `/usr/bin/grep -nE '[Bb]elow|[Aa]bove'` over mutations.sh, each hit's target read at the frame (31 hits):
  - **into the moved code, from what stays** (rewritten to name the new part): :16 "the correspondence check below",
    :158 "the always-on anchor check (below)", :189 "the ratchet below", :199 "each is checked below", :201 "the
    census below", :251 "EXACT, both ways (below)", :750 "its BOOKKEEPING above" (the opt-in half's header, about the
    always-on half);
  - **out of the moved code, to what stays** (rewritten to name mutations.sh): :653 "see "UNLISTED" above" (the
    anchor loop's comment; "UNLISTED" is mutations.sh :239, which stays);
  - **within one side** (unchanged): :29, :53, :61 (`_MUT_UNRECORDED_MAX` stays at :256), :137, :161, :167, :171, :197,
    :230 and the messages :392, :396, :789, :859, :883, :885, :890 (all on the staying side); :516, :531, :533, :544,
    :695, :710 (all inside the moved block, their targets moving with them); :724 is a message's "below the ratchet",
    not a reference;
  each re-read at the split's head;
- the docs' **present-tense** cites of mutations.sh lines in the moved range and the part counts ("seven parts", file
  lists, landing X10's part list), found by the sweep. **Line cites that are provenance stay**: `…-cr-sweep.md`'s ledger
  and `…-cr-plan.md`'s cites are frame-tagged records of the CR round (the reason `b3b64aa4` dropped a records.sh line
  rather than shift them) and are not re-pointed; a cite is present-tense when it describes the current code without a
  commit beside it;
- **the no-8-hex rule** ("no 8-hex string in mutations.sh's comments") is carried to the new part: its comments are
  checked by the same grep (`awk '/^[[:space:]]*#/' <part> | /usr/bin/grep -cE '[0-9a-f]{8}'` → 0) at CR2-0 and every
  later slice;
- **the sweep's file set** (`…-cr-sweep.md` :779's `files=(…)` list, copied into CR2's `sweep.sh`, §11) gains the new
  part (from CR2-0) and the #519 memo's companion (from CR2-D's split commit, §7).
Outside the repo: `runners/lib.sh`'s `PARTS` list gains the part (the battery copies the parts by that list). A stub
alternative (moving only the census block) was rejected: the census's `.bare` is consumed by the ratchet at the
function's end (:710–:727), so that split cuts through one function.

---

## §9 Guard table

The binding rule (`…-impl.md`:271): a guard removed → something reds (named); shipped → green.

| id | guard | KILLED by (needle) | survive arm | pinned | slice |
|---|---|---|---|---|---|
| G4-1 | ratchet + literal rule read the spelled names | R4a (`$_cen_lbl`), (M) | population back to `.labels`, (M) SURVIVED | record | A |
| G4-2 | literal rule's `isv` from `.mvalues` | latent (equal populations at the frame) | — | claim row CR4-L | A |
| G7-1..4 | `_mut_fresh`'s stale / count / twice / carry arms | R7a–d (`$_af_lbl`) via the self-test rows | that row removed | records | B |
| G7-5 | the self-test runs inside mutants | R7a–d themselves (each is killed only if it runs there) | the self-test moved below the `*.mutant.*` skip → all four SURVIVE | records | B |
| G9 | the retry's purge is git's list | R9 (`$_fsm_lbl`) | the export alone | record | C1 |
| G14-1 | the output file cleared before the launch | unpinned (root writes a read-only file; §5) | — | claim row CR14-a | C1 |
| G14-2 | the clearing fails closed | unpinned (as G14-1) | — | claim row CR14-b | C1 |
| G11-1h | HUP exits 129 with a line | R11h (the signal control's label), (M) | HUP's row removed, (M) SURVIVED | record | C2 |
| G11-1i | INT exits 130 with a line | R11i, (M) | INT's row removed, (M) SURVIVED | record | C2 |
| G11-1t | TERM exits 143 with a line | R11t, (M) | TERM's row removed, (M) SURVIVED | record | C2 |
| G11-1p | the rows' deliverability probe | unpinned (an equivalent mutant where all three signals are delivered; x§6.4) | — | claim row CR11-P | C2 |
| G11-1f | the exit lines go to the stderr saved at startup | unpinned (R11f SURVIVED: no selftest landing is redirected; x§6.4) | — | claim row CR11-F | C2 |
| G11-2 | the exit path reaps `_ctl_run`'s groups, from every reap set the run makes (H1: one carrier, `$_K2_REAP_D` and the directories inside it) | R11r and R11n (`$_og_lbl`: the `_og` sig row, the call nested in a subshell that holds the run's traps and reap set and is TERMed with it; its exit path must leave no recorded group; R11n reads one level only), (M) [r5p] | the sig row's assertions removed, (M) [r5p] | record; claim row CR11-R (every call site, the `_og` one included) | C2 |
| G11-2c | a stopped group gets TERM, then 1–2 s (whole `SECONDS`), then KILL (the stop procedure `$_K2_STOP`, one text for the watchdog and the reap) | R11w (`$_wm_lbl`: the `_wm` hang stub traps TERM and the row asserts it was sent when TERM can reach a child), (M) | the row's TERM assertion removed, (M) | record (the watchdog's use); claim row CR11-C (the reap's use: the stopped child's scratch dir) | C2 |
| G11-3 | no child keeps the caller's stream: the watchdog `exec`s | R11d (`$_og_lbl`: the `_og` kill row, the caller SIGKILLed inside a `$( … )`, the substitution must end at once), (M) | the kill row's time assertion removed, (M) | record | C2 |
| G11-3b | the watchdog's launch closes fd 9 | R11e (`$_og_lbl`: the kill row's call sends fd 9 into the substitution, `9>&1` before its `> /dev/null`, so a watchdog that keeps fd 9 holds the substitution open), (M) | the kill row's time assertion removed, (M) | record | C2 |
| G11-4 | errexit off in `_k2_exit` | R12e (`failrm`'s label: its `chmod` fails), (M) killed both shells | no `chmod` shim, (M) | record | C2 |
| G11-4b | errexit off and HUP/INT/TERM ignored first in `_k2_sig` and the mutation run's trap string | unpinned (no control fails a command inside `_k2_sig`; the trap string runs only in the opt-in run) | — | claim row CR12-E | C2 |
| G11-6a | a reap set that cannot record a group ends the call with exit 2, the cause on fd 9, its groups stopped | R11a (`$_og_lbl`: the norec row, the reap directory missing), (M) | the norec row's assertions removed, (M) | record | C2 |
| G11-6b | a reap set that cannot drop a group ends the call with exit 2, the cause on fd 9 | R11x (`$_og_lbl`: the nodel row, the stub turns its own entry into a directory, which `rm -f` cannot remove, root included), (M) | the nodel row's assertions removed, (M) | record | C2 |
| G11-7 | the fourth standing check's rows run | R11g (`$_cen_lbl`, GC-3's class: `for _og_row in kill norec nodel sig` emptied; the census's bijection reds the call line no run registered), (M) | none, as for GC-3's three (the census is always-on) | record | C2 |
| G11-8 | the stop procedure and the watchdog run without the caller's SHELLOPTS and BASH_ENV | unpinned (no control can deliver SHELLOPTS: bash rewrites it, so a channel naming it falls to NOT EXERCISED, ctlrun :261–:265) | — | claim row CR11-V | C2 |
| G11-9 | a reap entry is acted on only as a number > 1 | unpinned (an equivalent mutant on every run: entries come from `$!`, never ≤ 1) | — | claim row CR11-L | C2 |
| G11-10 | a group is stopped only when `kill -0 -<pgid>` fails: `_k2_end` re-KILLs each group of the call (bounded) and drops its entry once the group is gone or its id names a reused leader (J6); the `_wm` check reads that confirmation (`_ctl_left`) | unpinned (a fork racing the group KILL cannot be produced on demand) | — | claim row CR11-K | C2 |
| G11-11 | an entry is acted on only while its group is the run's: no live leader, or a leader no younger than the record (`ps -o etime=`); unknown → skip; a `ps -o etime=` that cannot read the run's own pid refuses at the controls' start (exit 2, J1) | unpinned (an equivalent mutant on every run: no id is reused within one; the refusal, where `ps` works) | — | claim rows CR11-I, CR11-J | C2 |
| G11-12 | a failed record restores `set +m`, stops the call's groups, then removes their entries | R11a (the add arm's last line → `return 0`: the norec row reds; x§6.4), (M); the order and `set +m` unpinned by a record (no control fails the watchdog's record after the child's, or a record under a tty) | — | claim row CR11-A (P10: the order) | C2 |
| G12 | a failed removal keeps the status and says so | R12 (`failrm`'s label), (M) killed both shells | the control expects `PASSED` | record | C2 |

**Declared blind spots (added or rewritten):**

| id | blind spot | why it stays | trigger | owner |
|---|---|---|---|---|
| BS-5a (rewritten) | rule 1's scan errs both ways: an under-count (the false-red classes of C1 plus the 45 generated) is red; an over-count (two measured classes and multi-line single-quoted strings) leaves a record non-orphan always-on | no line scan of bash is under-approximate by construction (§2.2, (M)); the ratchet no longer depends on it | a record whose trial is not killed as named in the opt-in run | the opt-in (X3), (M) it reds the over-count (`cr4-ovoptin.*`); stated at rule 1's comment and design memo row C |
| BS-8 | the anchor rule reads only the edited line and only outside the changed span: an address range's other end, a hold-space source line, a variable defined elsewhere, and a wildcard inside the span can drift with the record "fresh" | the rule is a property of one expression and one line; context drift is what the opt-in's kill tests. **(M) backstop, one drift shape (address-range end):** `cr2/bin/f13drift.py` respells ctlrun :196–:197 (the `return 1` record 403 was written for, and the range's end `fi`; behaviour unchanged): record 403 is fresh under all three clauses (`f13-drift-fresh.txt`: the frame's 25 non-fresh only) and now edits `_fixture_did_fail`'s `return 1` (:205); the opt-in run filtered to it: `MUTANT 228 (the gate declines exactly the members of a group whose flag is 0) SURVIVED`, both shells (`f13-drift.{b53,b32}.log`); at the frame the same record is killed as named (`f13-frame.b53.log`: `0 not killed as named`): the opt-in reds the drift the anchor rule passes. **(M) inside the span:** `f13span.txt` (§3.2) | a record whose trial survives or is killed under another label | the opt-in (X3); stated beside `_mut_fresh` |
| BS-9 → **R14** (narrowed) | the instant between each launch and its own `_k2_reap_add`: a signal in the child's instant leaves the child unrecorded **and with no watchdog yet**, so it runs to its own end unbounded; in the watchdog's instant the child is recorded and stopped, and the unrecorded watchdog sleeps out its seconds and stops a dead group | a group's id exists only after its fork; a recorder in the child would be the same instant moved | a run stopped by a signal in that instant (a leftover selftest child, or a `sleep` of up to 30 s) | **residual R14**, added by CR2-C2. CR2's own (the reap set is CR2's). **Owner:** the citation-hygiene program, k2 trip-wire lane (R11's form: Gap/Why/Trigger/Owner, dated); where its leftover runs unbounded it is R9's class and subordinate to `#11-trip-wire-liveness-bound`, whose run-wide group would bound it. **Direction:** a leftover, never a wrong kill or a changed status |
| BS-10 → **R15** | the run's groups are stopped only when the run ends through its traps with TERM deliverable: (a) SIGKILL of the run and the default-fatal signals it does not trap (QUIT, ALRM, USR1, …) reap nothing; (b) a run whose TERM was ignored at its entry passes the ignore to its children, so the stop's TERM does nothing, the KILL after 1–2 s ends them (`r3f/f2probe3.txt` M: 1.77 s b53 / 1.86 s b32), and a stopped selftest child leaves its scratch dir (the KILL-only stop: `r3f/r3-s-top-killonly-TERM.*`, 1 left; CR11-C) | SIGKILL cannot be trapped; the other signals are not what terminals, CI or `kill` send to stop a run; an ignored-at-entry signal cannot be undone (`dispprobe` (1)) | a report of a leftover selftest child, `sleep`, or scratch dir after such an end | **residual R15**, added by CR2-C2: (a) pre-existing (the frame reaps nothing on any end) and subordinate to `#11-trip-wire-liveness-bound` (R9), whose run-wide group is what reaches a SIGKILLed run's groups; (b) CR2's own (the stop's TERM). **Owner:** the citation-hygiene program, k2 trip-wire lane (R11's form: Gap/Why/Trigger/Owner, dated); the leftovers hold none of the caller's descriptors (G11-3). **Direction:** leftovers, never a wrong kill; the status is the signal's |
| BS-11 | the opt-in kills of R11h, R11i and R11t, and of R11w, R11r and R11n (their assertions stand down via `_wm_tg` when TERM cannot reach a child), need HUP, INT and TERM deliverable at the run's entry: a run that inherits one ignored (a non-interactive `… &`, `nohup`, a caller's `trap "" TERM` — ctlrun :325's case) cannot trap it, the row expects `PASSED`, and the record SURVIVES (red, never green; (M) R11i under INT ignored and R11t under TERM ignored, x§6.4) | bash cannot trap a signal ignored at entry; the ship stays correct there (the probe) | an opt-in run started from a script's background job, under `nohup`, or under `trap "" TERM` | **carried by CR2-C2's files**: landing X3's text and its survive-check runner text start the wire with HUP, INT and TERM at their default (a `perl` reset, as `cr2/bin/cr11.sh`), and the slices' single-record runs do the same (`cr2/bin/r3-optin.sh … default`); stated beside the records |
| BS-12 (narrowed) | a stale entry — a failed drop in a standing check's subshell (it lasts until the run ends), or an entry whose write was cut by a SIGKILL of its recorder or by a default-action TERM/INT of a standing check's subshell (T, J15) — names an id that may since be reused | closed in the fail-safe direction (H3, x§6.2): an entry is acted on only while its group exists and has no live leader or a leader no younger than the record, else skipped; drops continue past a failure. **Residue and its consequence:** (1) a group that emptied, whose id a new leader reused and left with members when it died, has no live leader and is taken as the run's — a wrong kill, if that sequence fits inside one run (J5: "no live leader" proves identity only for a group that existed throughout); (2) a reuse within the record's 1–2 s granularity is taken as the run's; (3) the clocks differ: `SECONDS` counts wall-clock seconds, Linux's `etime` comes from a boot-based start time, so a wall-clock step during the run shifts the comparison by the step (J14); (4) an empty (cut) entry, or a `ps` failing in the reap, is skipped and a signal leaves that group running (a leftover, R15's consequence); a machine whose `ps -o etime=` cannot read the run's own pid refuses at the controls' start, exit 2 "decided nothing" (J1), not a false red | a report of a wrong kill at a run's end, or of a leftover | CR2-C2, stated beside `_k2_reap`; claim rows CR11-I, CR11-J |
---

## §10 Claim table

Every numeric or universal claim the fixes change. "Both" = b53 and b32. Each row is re-measured by the slice on its
own head; the frame's figure is given where measured here. An instrument over `docs/plans/*k2-fgit*` (CR4-2, CR-D2, CR-S,
CR-P) leaves CR2's own plan, companion, sweep and the sweep's instrument companions (every `…-cr2-*.md`; not `…-impl-cr2.md`) out of its population, for H6's reason (§11): they state the identifiers, they are not sites (J23).

| id | claim (site) | measured by | expected |
|---|---|---|---|
| CR4-1 | rule 1's error statement (mutations.sh :552–:554 → the new part) | `cr2/bin/oracle2.sh <part>` | 395 cases: C1's 131 = 114/13/4/0; generated 264 = 201 agree / 45 under / 18 over, over-counts exactly the two classes; a new over-count class is a finding |
| CR4-2 | "never a false green on the oracle" as a current statement | `/usr/bin/grep -rniE 'never a false green\|every error is a false red\|0 false green'` over `docs/plans/*k2-fgit*` and the parts | at the frame **16 hits**, each classified: **the claim, current**: mutations :552, :553, design memo row C :620, residuals :120 (R13); **the claim, closed plan** (the corrective note, §7): `…-cr-plan.md` :413, :733; **the claim, closed record** (corrected in `…-impl-cr2.md` §T): `…-impl-cr.md` :623, :782; **unrelated**: `…-cr-plan.md` :328, :964 and landing :131 (the capability declaration's "never a false green"), landing :398 (X9's row), corpus :366 (a dated record row), `…-impl-cr.md` :608, :614, :616 (the C1 rounds' history). After: 0 current sites stating it; the closed plan's two under its note; the closed record's two stay |
| CR4-3 | `…-impl-cr.md` :625–626 "can never invent one" | the over-count rows of CR4-1 | the correction recorded in `…-impl-cr2.md` §T (CR2-A's entry); `…-impl-cr.md`'s body unchanged; `wc -l` 797 from CR2-M on (796 + the end pointer) |
| CR4-4 | spelled ⊇ expanded; 31 names; `.bare` 18 | `cr2/bin/pops.sh`; ship ×4 | 31 = 31; ratchet 18 |
| CR4-5 | records.sh spells only defined labels | `/usr/bin/grep -ho '_[A-Za-z0-9_]*_lbl'` over records.sh, each token defined in controls.sh | every token defined; R4a adds none (M) |
| CR7-1 | "exactly one line" as the current definition | sweep, current-tense sites only: mutations :657–:687 (→ the new part), the design memo's anchor wording, records.sh's header if it states it | three clauses stated at each; corpus :245–:247 and :286 (dated provenance) unchanged |
| CR7-2 | 25 of 263 not fresh at the frame; 0 of 262 after | `cr2/bin/fresh.sh`, `fresh.py` | as stated, both shells |
| CR7-3 | re-anchored mutants identical | `cr2/bin/cr7-reanchor.py` + the identity script (`cr7-reanchor-identity.txt`) | 24/24, 81 == 80 |
| CR7-4 | the finding's repro under the fix | `cr2/bin/cr7-edit.py` on the slice head | records 89–92 and 97 red (twice / carry) as well as the six the frame reds, both shells |
| CR7-T | the anchor pass's cost in the wire | `time` of two ship runs, before / after, both shells | the growth stated in seconds; the CI budget (`timeout-minutes: 5`) unaffected |
| CR-N | `_mutants` count and `_MUT_RECORDS_MIN` | `_mutants \| wc -l` at each slice head | 251 → 250 → 254 (B) → 255 (A) → 256 (C1) → 269 (C2: R11h, R11i, R11t, R12, R12e, R11w, R11d, R11a, R11x, R11e, R11g, R11r, R11n) |
| CR-R | `_MUT_UNRECORDED_MAX` 18 | ship ×4 | 18 at every slice head |
| CR-C | 89 call lines, census | the census's own count at the head (§S.8's method) | 92 after C2 (the signal loop's call line, R12's control, the `_og` check's call line) |
| CR-6 | corpus §6.1's totals line, :373: "**156 records of this PR, 251 in all**" | `_mutants \| wc -l` + the corpus rows | re-stated at each slice that changes the count (250, 254, 255, 256, 269), with record 81's removal and its origin |
| CR-8 | no 8-hex string in the comments of mutations.sh and the new part | `awk '/^[[:space:]]*#/' <file> \| /usr/bin/grep -cE '[0-9a-f]{8}'` | 0 and 0 at every slice head from CR2-0 |
| CR9-1 | the fsmonitor block's comment (controls.sh :691–:696) and the retry line (:708) | `/usr/bin/grep -c 'unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE'` over the parts | 0 |
| CR14-G | the readers of `_rc`/`_out` (and, from C2, `_ctl_left`/`_ctl_wd_pg`, J10) outside `_ctl_run`, what each depends on, and the contract text (§5) | `/usr/bin/grep -nE '\$\{?_(rc\|out\|ctl_left\|ctl_wd_pg)([^A-Za-z0-9_]\|$)\|unset _(rc\|out)'` over the wire and the parts, each site read; every `return` and `exit` inside `_ctl_run` and the reap-set writers it calls, read against the contract's three clauses | **at C1's head and again at C2's head**: the four reader groups of §5 (`_control`; relcwd and fsmonitor; `_wm` :350; `_cap` :443/:449) plus the fourth standing check's (it reads no `_rc`/`_out`: its rows read the substitution's own status line); at C2's head `_ctl_left`/`_ctl_wd_pg`'s one reader, the `_wm` check (controls :350's line), beside its rows' reds; every return before the launch leaves both untouched, every return after it has both assigned, the reap-set failures `exit 2`; a new reader or a return between `_rc=` and `_out=` is a finding |
| CR11-1 | "the run ends only through a verdict" and the exit comment (wire :405–:421, including :412–:413's "every other end is `exit 1` or `exit 2`") | `cr2/bin/sigsite3.sh` on the head, top level, TERM and INT | 143 / 130 with `stopped by SIG<n>`; the false line gone; :412–:413 and landing :280 restate the ends: 1, 2 and 128+n |
| CR11-2 | ctlrun's watchdog comment ("the killer is itself killed when the child returns", :314) and the `set -m` paragraph (:317–:328: "Both are killed as a group, with SIGKILL") | sigsite3; the `_wm` check | "… and, on a signal, stopped by the run's exit path from the reap set"; "TERM, then after 1–2 s (`SECONDS` counts whole seconds) SIGKILL, which no inherited disposition can ignore"; the call's end re-KILLs until each group is gone (CR11-K) |
| CR11-N | job notices | sigsite3 b32 | no `Killed: 9` / `Terminated: 15` for reaped jobs; one bash notice for a standing check's subshell, stated (x§6.4) |
| CR11-R | the reap, from every call site (H1: every reap set the run makes is read at its end) | `cr2/bin/sigsite3.sh` at `_ev`, `_cap`, top level (`r3-instr.py instr-top`: a `_control` child live) and the `_og` kill and nodel rows (an `instr-og` instrument C2 writes: its stub marks and sleeps) × TERM, INT, both shells; the `_k2_reap` call removed (`noreap`) | 0 left with it at every site; without it, at `_ev`/`_cap` 4 (the stub's child and its `sleep`, the watchdog and its `sleep`) and at top level 7 (the selftest child and its processes, the `grep` shim's `sleep`, the watchdog and its `sleep`), TERM and INT, both shells (`r3f/r3-s-*-noreap-*`); without it the ship reds under `$_og_lbl` ("sig: the exit path left group N running", R11r: `r5/r5-noreapship.*`) |
| CR11-F | the exit lines reach the startup stderr wherever the signal lands | `cr2/bin/f3probe.sh` (a synthetic probe, its own text) | fd 9 seen in all 10 fd-9 cells per shell; fd 2 lost at `launchblock`/`fnredir` (the wire's landing `_verdict "$(_scan …)"` is wire :1250) |
| CR11-P | the signal rows' probe | ship with INT, HUP and TERM each ignored at entry (`cr2/bin/r3-runsig.sh`), with and without the probe, on the one text | PASSED with it (that row expecting 0); that row red without it |
| CR11-D | no child holds the caller's stream after the run | the `_og` kill row (R11d, R11e); `cr2/bin/f3hang.sh` on the head | a SIGKILL while `_ev` runs → EOF under 1 s, both; the two removed arms (fd 9 kept; a watchdog that stays a bash) ~30 s |
| CR11-S | the launch silencing (ctlrun :329–:343's tostop measurement) with the watchdog's new body | the launch-shape probe of that comment, re-run | 0 failed launches under tostop, as stated there |
| CR12-E | errexit and a second signal cannot change the exit path's status (`_k2_sig`, the mutation trap string; `_k2_exit`'s errexit is R12e) | `cr2/bin/f2probe3.sh` on the head's exit-path text | status kept in all fifteen cases, both shells (J: a TERM during `_mut_rm_copies` keeps 7 and removes the scratch dir; r2's string: 143, scratch left) |
| X10 | the wire's touches and line count (landing :399) | `git log --format=%h -- <wire>` on #527's range; `wc -l` at each | `8f4ad3c8`, CR2-D's banner commit, C2's commit; the measured counts |
| CR-P | "seven parts" and part lists (docs, mutations header :5–:57, controls :119–:124) | sweep of the part words | eight, `correspondence` listed |
| CR-U | umbrella :111 "All thirteen files" (the one count); :23's enumeration; :110's registration | `/usr/bin/grep -n 'All [a-z]* files'`; :23 and :110 read | **twenty** at :111 (+ `…-cr2-plan.md`, `…-cr2-plan-exitpath.md`, `…-cr2-sweep.md`, `…-cr2-instruments-probes.md`, `…-cr2-instruments-texts-r2-r3.md` and `…-cr2-instruments-texts-r4-r5.md`, `…-impl-cr2.md`); :23 names the same seven; :110 names the #519 memo's companion; each of the seven has exactly one `## §0` |
| CR-D1 | (O-1 A) the #519 memo's banner :28–:36 | `/usr/bin/grep -c '_gopt_ok\|_ctl_need'` over that memo's banner | 0 |
| CR-D2 | the residual range at residuals :7, design memo :31, :56, :649 | `/usr/bin/grep -rnE '"?R1"?–"?R[0-9]+"?( and "?R11"?)?'` over `docs/plans/*k2-fgit*` and the umbrella (the quoted and bare forms, old and new); each hit classified | at the frame 23 hits: **current** residuals :7, design memo :31, :56, :649 (rewritten); **unrelated** (review rounds, "plan-review R1–R3"): `…-cr-plan.md` :3, design memo :19, umbrella :23; **hist** (closed records of earlier ranges): `…-cr-plan.md` :908, `…-cr-sweep.md` :763–:764, `…-impl-cr.md` :225–:227, :267, :353, :355, :377, :420, :577, :778, :779, :789, pr527 :657. After CR2-D: the four read `R1–R9 and R11` / `"R1"–"R9" and "R11"`; after CR2-C2: `R1–R9, R11, R14 and R15` / `"R1"–"R9", "R11", "R14" and "R15"`; no `R1–R13` left at a current site |
| CR-D3 | every referrer into the moved §10–§11 points at the companion | §7's two instruments at the banner commit's head, each hit read | the §7 current list, each re-pointed; the §7 hist list unchanged, reviews :518–:525 with its note |
| CR11-C | a stopped group's own exit path runs (TERM first) within the stop's 1–2 s: the reap's use of the stop procedure | `cr2/bin/sigsite3.sh` at top level with a `_control` child live, the run's TMPDIR its own and kept, scratch dirs counted; arms: the head, the stop's TERM step removed (`killonly`), r2's text, the frame | the head 0 scratch dirs, 0 processes left, the whole exit path (the child's own cleanup included) under 1 s — [r3f] 0.26–0.37 s, TERM and INT, both shells — so `-lt 2` stays (else `-lt 3`); `killonly`: 1 scratch dir, 1.82 / 1.85 s; r2's text: 1 scratch dir (KILL); the frame: 7 processes at +1 s, 0 scratch dirs once they ended (`r3f/r3-s-top-{reap,killonly,r2,frame}-*`) |
| CR11-W | the watchdog's timeout stop leaves no scratch dir | `cr2/bin/r3-tdrun.sh` with `r3-instr.py wdhang` (a real selftest child hung under a 3 s watchdog), TMPDIR kept | the frame: the hung child dies 137 (KILL) and leaves 1 scratch dir; the head: it dies 143 (its TERM trap) and leaves 0; the stop's TERM step removed (`killonly`): 137 and 1; both shells; the frame without the hang leaves 0 (the negative control) (`r3f/r3-td-{frame-wdhang,r3-wdhang,killonly-wdhang,frame-plain}.*`) |
| CR11-V | the stop procedure and the watchdog ignore an exported SHELLOPTS and BASH_ENV | `cr2/bin/g12probe.sh` (watchdog forms); f2probe3 O (`_k2_stop` with SHELLOPTS exported) | the head's form writes its marker and kills its target with SHELLOPTS exported, BASH_ENV not read, both shells (r2's `exec "$BASH" -c`: no marker, target alive, BASH_ENV read, both shells) |
| CR11-L | a reap entry is acted on only as a number > 1 | f2probe3 L (the stop stubbed to print its arguments; entries 0, 00, 01, 1, 12a, 5) | only 5 reaches the stop, both shells |
| CR11-Q | a signal sent to the run's pid alone (not its group) | `cr2/bin/sigsite3.sh … pidonly` at `_ev` | rc 143 with the line; the trap waits until the standing check's subshell ends (bounded by that subshell's rows × `_ctl_wd_s`): rc 143, the signal's line, 6.23 s (b53) / 6.33 s (b32) from the signal to the end (the `_ev` check's two rows, each stub sleeping 3 s, ran out first), 0 processes left (`r3f/r3-s-ev-pidonly-TERM.*`) |
| CR11-T | a signal during an opt-in trial (the carve's trial-nesting edge) | `cr2/bin/r3-trialsig.sh` (TERM to the run's group once the trial's wire has its own verifier directory) | rc 143 with the line, 0 processes left in the run's groups, 0 scratch dirs, 0 `.mutant.` copies, both shells; on b32 two `_ctl_run` groups of the trial were live at the signal (`r3f/r3-trial-TERM.*`) |
| CR11-K | a group is dropped only when `kill -0 -<pgid>` fails (H2) | `cr2/bin/r4race.sh` (from R3 Axis 2's race probe), its `_k2_end` form taken from the head's exit block as `r4probe.sh` extracts it, not its copy `end4` (C2 makes that change, J9): N ≥ 600 calls (J26) × both shells × both forms (the frame's / [r3f]'s single KILL, and the head's `_k2_end`), 2 × ncpu busy loops | the single-KILL form shows survivors in the watchdog's group (if not, raise N or the load: the probe has no subject); the head's form 0. [r4p]'s small run: N = 600, b32 frame 4 vs 0, b53 [r3f] 6 vs 0 (`r4/race600-*`). A nonzero residue after the bounded stop is a residual row (gap/why/trigger/owner), not a note |
| CR11-I | an entry is acted on only while its group is the run's (H3, BS-12) | `cr2/bin/r5probe.sh` (the head's exit block, synthetic cases) | own live group stopped; a leader started after its record skipped (alive after); a leaderless group stopped; an unreadable entry, or no `ps`/`awk` on `PATH`, skipped; a failed drop does not stop the later drops (rc 2, that group named); five entries reaped in < 0.1 s; both shells ([r5p]: `r5/r5probe.txt`) |
| CR11-A | a failed record restores `set +m`, stops, then removes (H17), with no errexit precondition (J8) | `r5probe.sh` P8 (`_k2_reap_add` failing under `set -m`, called bare under errexit) and P10 (the child recorded, the watchdog's record failing; `_k2_stop` wrapped to say whether the child's entry exists when the stop starts, J7); the `_og` norec row | P8: rc 2, the line, the group gone; P10: rc 2, the watchdog named, both groups gone, "at-stop: child entry present", the child's entry removed after (its swapped-order arm P10s: "absent"); P8r, [r4p]'s bare `wait` under errexit: rc 143, not 2; both shells ([r5p]: `r5/r5probe.txt`); `set +m` before the stop (read in the text: no control has a tty) |
| CR11-J | a `ps -o etime=` that cannot read refuses the run, never reds it (J1) | the ship with a `ps` that fails (`cr2/work/r5-ps-shim/ps`, `exit 1`) first on `PATH` | rc 2, "!! decided nothing: the reap's identity check needs \`ps -o etime=\` (read through awk), …", no other red; both shells ([r5p]: `r5/r5-shipps.*`) |
| CR11-O | the `_og` check's per-row `rm -rf` deletes a live entry only after a red row (J16) | read: each row's assertions against its recorded groups (x§6.4) | kill and sig `_og_gone`, norec its group, nodel `_k2_end`'s confirmation; a new row without one is a finding |
| CR-S | "three standing checks" as a count | the §1 (7) instrument, each hit read | four at every current site; the closed plan under its note; provenance unchanged |
| CR-B | the budgets with comments (§8) | `wc -l` at each slice head | ctlrun ≤ 439, wire ≤ base + 90 (1376 at `706049ad`), the others as §8 |
| CR-7 | every record CR2 changes has a corpus §6.1 row naming its commit | the corpus rows against `git log -p -- <records.sh>` over CR2's range | one row per added, removed, re-anchored or re-prefixed record (records.sh:360 at CR2-0 included); the totals line equals `_mutants \| wc -l` (CR2-Z) |

---

## §11 Slices

Base: `706049ad`, or the last #501 tip merged under it (O-2's rule). Each slice: its author (the model note below; C2's
brief and reviewer get both plan files) in a fresh `git clone --local` → its claim table and arm
table (every arm of every guard it touches: a mutant run red under a named label, or a written equivalent reason) →
the battery (`runners/battery.sh b53` and `b32`, `BASE_REV` = the slice's base, the worktree holding the slice diff
uncommitted, the cases kept current: a case whose edit target the slice rewrote is retargeted or retired with its
reason) → **a fresh reviewer (the reviewer type's default model, no `model` override)** on a frozen snapshot (no wire
run while the battery runs) → a fix round by a new author at the slice's tier → a focused re-check when the review had an IMPORTANT → the
driver's diff read → the record entry (§12) → transplant (the worktree's uncommitted diff verified equal to the slice
diff, backed up, then discarded; the worktree must then be clean, else stop (J18); then a fast-forward of the worktree to the slice's commit, the base being the worktree's head, O-2) → ship ×4 (BSD and GNU-like grep, `crplan-r2/gnulike` first on `PATH`, × b53 and b32)
and `scripts/trip-wires.sh` on the worktree. Single-record opt-in runs use the `crc2-fix/Bc-t2.sh` method (the filter
after `_mut_parse "$_mrec"` in `_mut_run`, the generated set stubbed), **started with HUP, INT and TERM at their
default** (`cr2/bin/r3-optin.sh … default`; BS-11); at most two wire processes at once. **Every record change is a
corpus §6.1 row** (an added, removed, re-anchored or re-prefixed record, records.sh:360's re-prefix at CR2-0 included),
written in the slice that makes it and naming its commit (claim row CR-7).

**The sweep.** Every slice re-runs CR2's copy of the CR round's sweep: `sweep.sh` and `idents.tsv` as printed in
`…-cr2-sweep.md` §E (created by CR2-M from `…-cr-sweep.md` §E; the closed round's instruments stay as run), with
- the file set = `…-cr-sweep.md` :779's list **plus the new part** (from CR2-0) **and the #519 memo's companion** (from
  CR2-D's split commit), **minus** CR2's own plan, its companion, its sweep and the sweep's instrument companions (every `…-cr2-*.md`:
  `…-cr2-plan.md`, `…-cr2-plan-exitpath.md`, `…-cr2-sweep.md`, `…-cr2-instruments-*.md`; not `…-impl-cr2.md`): the ledger's own statement of the identifiers, not sites — cr-sweep
  :781–:782, the CR round's R3 H1 precedent);
- CR2's identifier groups added to that `idents.tsv` in the commit that adds the identifiers (`_K2_REAP_D`, `_k2_reap`,
  `_k2_sig`, `_K2_STOP`, `_k2_stop`, `_k2_reap_add`, `_k2_end`, `_k2_age`, `_k2_traps`, `_ctl_left`, `_og_trap`, `.mvalues`, `_mut_fresh`, `_mf_v`, `_af_lbl`,
  `_og_lbl`, `correspondence`, `.control_out`, the phrases of §10);
- **B2a I-2's rule** (`…-impl-cr.md` :779, §S.8's B2a row: "every hit of a group whose referent changed is
  re-classified"): every hit of every identifier group whose referent the slice changed is re-classified by its whole
  sentence, not only the new hits — CR2-A: the census population words ("expand", `.vlabels`, `$_cen_used`, "spell");
  CR2-B: "fresh", "exactly one line", "anchor"; CR2-C1: `_rc`/`_out`, `.control_out`, "RETURN CONTRACT"; CR2-C2:
  `_K2_ORDERLY`, the exit trap, `_ctl_wd_pg`, `_ctl_left` and the `_wm` check's "outlived" (H2), the reap entry's format (a number > 1 holding its record's `$SECONDS`, H3), "watchdog", "killed when the child returns", "SIGKILL", "standing
  check", `_rc`/`_out` and "RETURN CONTRACT" (its non-return clause); CR2-0: the part words and "below"/"above" in the
  split files; CR2-D: "plan memo §1", "parent §1", "parent D";
- the `perl -0777` wrap backstop.
`ledger-check.py` is the CR round's (its §3 is committed at CR-B1) and is not a CR2 completion condition. **Line cites
that are provenance** (frame- or commit-tagged records: `…-cr-sweep.md`'s ledger, `…-cr-plan.md`, `…-impl-cr.md`'s
body, the corpus's dated rows, a closed round's instruments) are classified `hist` and not re-pointed; present-tense
cites into moved lines are.

| slice | items | depends on | files | completion (besides the above) |
|---|---|---|---|---|
| CR2-M | the move; the record rollover | plan-review passed | this memo and its companion → `…-cr2-plan.md`, `…-cr2-plan-exitpath.md` (§6, its section numbers kept); a companion `…-cr2-sweep.md` with §0 and §E (§E.2 and §E.3 in its instrument companions `…-cr2-instruments-probes.md`, `…-cr2-instruments-texts-r2-r3.md` and `…-cr2-instruments-texts-r4-r5.md`, each with its own §0, named by its §E index: the 1000-line split, made in the move commit) — **every instrument a claim row, a guard row or a BS row of this plan cites, as run, verbatim** (the list is §14's "Instruments", re-checked at the move by grepping both plan files for `cr2/bin/` and for every file name §14 lists), and the sweep's `sweep.sh`/`idents.tsv` — so a cited instrument resolves in the repo (`…-cr-plan.md` §7 "The move", the CR round's R3 H1), each with its inputs (`cr12.sh`'s `work/shim-rm/rm` and its `work/fx/clean`, made by `mkfx-clean.sh`, J19; the frame clone named by commit; `oracle2.sh` carries `oracle.sh`'s text and `r2`/`r3-optin.sh` the Bc-t2 filter inline: no other input); **every handoff name in the two plan files re-pointed** to its repo name (the two plan files; the plan-review reports, the classification and the findings file → `…-impl-cr2.md` §T.0, which records them) or dropped; design memo §13 names `…-impl-cr2.md` §T as the open record (it names none at the frame: §13 :741–:744), the new file with the no-spec-surface §0; `…-impl-cr.md` §S ends with one pointer naming `…-impl-cr2.md` §T as where its corrections are recorded; design memo header, umbrella :23 and :111 register the files (CR-U) | design memo ≤ 760; umbrella "All twenty files" and :23 naming the seven; each of the seven with exactly one `## §0` (J20) and under 1000 lines; every `cr2/bin/` name in the two plan files present in §E (through its index) with its inputs; `/usr/bin/grep -nE 'CR2-plan(-exitpath)?\.md\|cr2/[A-Z][A-Za-z0-9_-]*\.(md\|json)\|findings\.json\|CLASSIFICATION\.md\|PLAN-REVIEW-R[0-9]\|PROGRESS\.md\|NEXT\.md'` over the two moved files: 0 (J3; `DECISIONS-R7.md` is kept: repo docs already cite it, `…-cr-plan.md` :13, `…-impl-cr.md` :58); §13 sequence reads `… → …-impl-cr.md §S (closed at 797) → …-impl-cr2.md §T` |
| CR2-0 | the split (§8) | M | mutations.sh, the new part, controls.sh:124, records.sh:360 (prefix) and its corpus §6.1 row, mutations.sh's positional references and the moved block's :653 (§8), the docs' present-tense cites; `…-cr2-sweep.md` §E (file set); runners `lib.sh` | output of ship ×4 equal to the frame's but for line numbers; `_mutants` 251; record :360 opt-in KILLED AS NAMED, both shells; the new part removed → exit 2 "decided nothing"; `$SH -n` over every part; CR-8; no behaviour change, so the battery's new == base |
| CR2-B | CR#7 | 0 | the new part (`_mut_fresh`, self-test, loop), controls (`_af_lbl`, the label list comment), records.sh (24 in place, line 81's comment, R7a–d), mutations.sh:260, corpus §6.1, design memo §4 row AF and anchor wording | §3.4's records as tabled, both shells; CR7-1..4, CR7-T; opt-in of the 24 re-anchored records KILLED AS NAMED, both shells |
| CR2-A | CR#4 | B (so R4a is checked by the new rules) | the new part (rule 1's block and comment, the two awk passes, the census heading :510–:518, the `.mvalues` line beside :566), records.sh (R4a; the header's spelling rule), mutations.sh :16–:18, :24–:25, :58–:61, :188–:189, :200–:207, :260, controls.sh :76–:77, :98–:100, records.sh :330–:333 (§2.3 item 3's list), design memo row C (:620) and row 9 (:700), `…-cr-plan.md`'s corrective note (created, with :413/:733), residuals R13, corpus §6.1 (R4a's row); `…-impl-cr2.md` §T (the impl-cr :625–626 correction) | R4a as tabled (M: §2.4); CR4-L; CR4-1..5; `oracle2.sh` run on the slice head |
| CR2-C1 | CR#9, CR#14 | A | controls.sh :691–:712, ctlrun.sh :228–:240 (the contract's C1 clause), :344, records.sh (R9), mutations.sh:260, corpus §6.1 (R9's row) | R9 as tabled; CR14-a/b, CR14-G at C1's head; ctlrun ≤ 439 or STOP; ship under `GIT_OBJECT_DIRECTORY` and `GIT_COMMON_DIR` unreachable PASSED, both shells |
| CR2-D | ER-A5-1 (O-1 (A), decided) | C1 | the #519 memo (split commit, then the banner commit), its companion (with its §0 and its bare-§ sentence, §7), residuals.md (R10, R12, R13 deleted; :113's re-point; the range), design memo :31/:56/:649 and :67/:78/:132, umbrella (:22's pointer; :110 registers the companion), `…-cr-sweep.md`'s corrective note (§F row 12), `…-cr-plan.md`'s corrective note (:819–:835's item), the wire :323/:403/:482/:711/:750/:1190, controls :677, fixtures :224/:564, reviews.md :518–:525 (note), landing X10's #519-memo cites, CR2's sweep file set (+ the companion) | the split commit moves text only (`git diff --stat` + a `diff` of the moved block against its old lines: identical); CR-D1, CR-D2, CR-D3; the wire's `wc -l` unchanged by the six re-points (measured on a clone: 1286, the longest re-pointed line 136 characters against the wire's 165, `cr2/logs/r3/g22-wire.txt`); ship ×4 (the wire touched) |
| CR2-C2 | CR#11, CR#12, the watchdog's kept stream | D (X10's `git log`), C1 (ctlrun budget) | wire :405–:431 (+ the exit-path block; the comment :412–:413 "every other end is `exit 1` or `exit 2`"), ctlrun.sh :228–:240 (the contract's non-return clause), :314, :317–:328, :329–:343 (comment), :350, :351–:373 and its header, controls.sh :102–:118 (`_og_lbl`, the list comment), :216–:219, :310 and :335–:367 (the `_wm` hang stub, its probe and assertion), the fourth standing check after :475, the two controls after :690, fixtures.sh (three signal shims, `failrm`'s two), records.sh (R11h, R11i, R11t, R12, R12e, R11w, R11d, R11a, R11x, R11e, R11g, R11r, R11n), mutations.sh :260, :785, corpus §6.1 (thirteen rows), residuals.md (R14, R15; the range; R9's "three standing checks" :116), the "three standing checks" sites (§1 (7)) and `…-cr-plan.md`'s note item, design memo §4 row OG, landing :280 (the ledger text's "every other end … is `exit 1` or `exit 2`": current, written into the ledger at landing, so restated to 1, 2 and 128+n), **landing X3's text and its survive-check runner text** (HUP, INT and TERM at their default, BS-11), landing X10 | the records as tabled in x§6.4, both shells; CR11-1/2/A/C/D/F/I/J/K/L/N/O/P/Q/R/S/T/V/W, CR12-E, CR14-G at C2's head, CR-S, CR-B; TERM and INT into the full run at a top-level `_control` and in `_ev`/`_cap`: 143 / 130 with the line, nothing left in the run's groups and no scratch dir left, both shells; the rm shim selftest rc 0 with the line; the census's call lines 92 |
| CR2-Z | the round's closing entry | all | `…-impl-cr2.md` §T | every figure the docs and part comments state as current re-measured at the head (§S.8's method); T moves with CR2, so X3 and the rest of `…-landing.md` §9.1 run at the new final T as already planned |

Order: M → 0 → B → A → C1 → D → C2 → Z. **Type (CLAUDE.md, H8, J4):** in any slice, a doc edit that re-points a citation or revises a definition → `designer`: CR2-0 (the docs' mutations.sh cites, §8), A, B (the design memo's anchor wording), C2, D and M; code and code-comment edits → `coder` (D's wire, controls and fixtures comments included). A slice with both runs its coder first and its designer after, in the same clone (the docs cite the code's final lines). **Model (decided):** CR2-B's and CR2-C2's coders start one tier up, with `model: opus` (CLAUDE.md tiering rule (a)): CR2-B's brief delegates a how-to judgment (the variant awk's shape in `_mut_fresh`), and so does CR2-C2's (its code is the prototype's, but the comments' account of x§6.0 and the in-place rewrite of ctlrun's paragraphs under the 439 stop line are the coder's to write). Every other slice's coder runs on the agent type's default model, with no `model` override; reviewers likewise (above). A fix round starts at its slice's tier (B/C2: `opus`), and rule 5's stop counts are per unit (the slice), across its fix rounds.

---

## §12 The record

`…-impl-cr.md` ends §S.8 at 796 lines and says step 2's entry "goes to the successor §13 names"; **§13 names none at
the frame** (design memo :741–:744: "at §S's rollover, this section names the successor file before the first entry
goes there"). CR2-M names it: `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl-cr2.md`, §T, by the D19-C rule,
and every CR2 slice's entry goes there (§T.0 the round's setup and this plan's review, §T.1… one per slice, §T.Z the
closing entry). Future-tense routes keep saying "the open record (§13)". `…-impl-cr.md` is closed: CR2 adds its one end
pointer and nothing else; a correction to its body (CR4-3) is recorded in §T.

---

## §13 Premise errors in the inputs

- **ER-A5-1's "fix in this PR"** met CV2's ratified split-first rule; decided O-1 (A).
- **"the successor file that design memo §13 names"**: §13 names none yet (§12).
- **CR#14's latent half** ("relcwd and fsmon would read the previous `_rc`/`_out` if they returned 2"): `if !` routes
  2 to the then-branch, so no `elif` reads them (§5); but the r1 statement that every reader reads after a 0 return was
  itself false (the `_wm` and `_cap` standing checks, §5).
- **CR#11's measured message** is seen only where the signal lands unredirected (lost in 3 of 4 cells,
  `trapprobe3.txt`), though the status is 0 at every landing under TERM; its "watchdog outlives the wire" is one
  `sleep 30` per run, r1's "two" counted a concurrent run (x§6.1).
- **CR#7's "records 45/46/81 never go stale"**: 46 is not a defect. It replaces the value with `"$K2RE"`, restates
  nothing it does not read (it reads only `K2RE_PATH=`), and its mutant means the same whatever the value; clause 3
  passes it (§3.1 list).
- **The first INT trial passed** (`cr2/logs/cr11-INT.*`): a script's background job starts with SIGINT ignored (the
  measurement resets it, x§6.1); so do `cr2/bin/run.sh`'s ship runs, where the INT row rightly expects `PASSED` (x§6.4).
- **Plan-review R1's cites, re-read:** the mutations header's "below" references are :199 and :201 (R1: :198, :200);
  :16, :158, :189 and :251 are the same class (§8). The `_cap` subshell spans :423–:471 (R1: :423–:444).
- **r2's own errors, corrected in r3 (found by plan-review R2):** a frame run leaves the watchdog's subshell and its
  `sleep 30`, not the child (`s-frame-TERM.b53.log`); f3probe's fd-9 cells are 10 per shell, not 20; r1's per-cause
  figures were confounded (the `rm` cause is `cr12-head.*`); the f3hang runs are on instrumented clones; r2's R12e text
  and prose disagreed; r2's (M) spanned three texts; CR11-F's line is wire :1250, the #519 memo's base note :16, CR-6's
  phrase corpus :373, CR4-2's list its instrument's output, and §14 says who runs f3hang (X9 runs only the wire).
- **Plan-review R2's premises, re-read:** Axis 2's "BASH_ENV sourced on b53": r2's exec'd bash reads BASH_ENV on
  **both** shells (`cr2/logs/r3f/g12probe.txt`). G6's "ends the run with exit 2": a failed reap-set write `exit 2`s the
  shell `_ctl_run` runs in, which is the run only at top level; in a standing check's subshell it is that subshell,
  and the run ends red (1) through the check's "ended before its verdict" (§5; x§6.2). G2's watchdog fold-in needed
  the stop text's own `SECONDS=0`: without it ([r3]) the watchdog's TERM was followed by KILL at once and the hung
  child's scratch dir was left exactly as at the frame (x§ header).
- **Plan-review R3's inputs, re-measured:** the race behind H2 is rare: [r4p]'s first runs (N = 300, `r4/race-*`) showed
  no survivor on either form, so CR11-K's frame arm needs N ≥ 600 or more load; the `_og` kill row of [r3f] could
  SIGKILL its caller before the watchdog's record was written ([r4p] at first red "the reap left group N running",
  `r4/attempt2/`): its stub now waits for both records' content (x§6.4).

---

## §14 Facts not read here, and the instruments

- **mawk and GNU sed** for clause 3's variant pass and sed runs, and for the census's two new awk arguments: not
  installed here (no installs); route (b) (`…-landing.md` X9) gains them. Direction: the self-test's carry and twice
  rows red if either tool makes the clauses vacuous (§3.3).
- **Real root** for CR14-a (a root user writes the read-only file): unreachable here; the claim row says simulated
  where it is. (R11x's directory entry fails `rm -f` for root too, so it needs no root caveat.)
- **GNU/Linux bash and coreutils** for the exit path: the `exec`ing watchdog depends only on the close-on-exec flag of
  bash's saved descriptors (measured on b53 and b32 here); `env -u` (GNU and BSD `env` have it), `sleep 0.1` (GNU
  `sleep` takes it; a `sleep` that rejects it falls to `sleep 1`, so the stop's bound becomes 2–3 s, never shorter)
  `kill -0 -<pgid>` and `ps -o etime=` are read here on macOS only (a `ps` without a readable `etime` refuses the run: CR11-J). **Who runs f3hang and g12probe on Linux:** route (b) runs the
  wire, not these instruments, so CR2-C2's files add them to landing X9's evidence list (a step of §9.1's route (b)
  run, beside the wire: `f3hang.sh` needs a FIFO, `perl` and `lsof` (informational line only); `g12probe.sh` needs
  `perl`), and X9's text names the expected figures (EOF under 1 s; marker and kill under an exported SHELLOPTS).
- **Instruments (every one a claim, guard or BS row cites; CR2-M copies them into `…-cr2-sweep.md` §E):**
  `oracle2.sh`, `pops.sh`, `scanV.awk`, `cr4-edit.py`, `cr4-over.py`, `cr4-parse.sh`, `cr4-proto.py`, `r4a-rec.py`,
  `fresh.py`, `fresh.sh`, `fresh-countonly.sh`, `cr7-edit.py`, `cr7-reanchor.py`, `f13drift.py`, `f13span.sh`,
  `cr9-proto.py`, `cr14-edit.py`, `cr11.sh`, `cr12.sh`, `trapprobe.sh`, `trapprobe2.sh`, `trapprobe3.sh`,
  `dispprobe.sh`, `sigsite.sh`, `sigsite2.sh`, `sigsite3.sh`, `f2probe.sh`, `f2probe3.sh`, `f3probe.sh`,
  `f3fdleak.sh`, `f3fdleak3.sh`, `f3hang.sh`, `g12probe.sh`, `r2-optin.sh`, `r2-runsig.sh`, `r3-optin.sh`,
  `r3-runsig.sh`, `r3-tdrun.sh`, `r3-trialsig.sh`, `r3-clone.sh`, `r3-batch.sh`, `r3-batch2.sh`, `r3f-batch.sh`,
  `run.sh`, the prototypes `r2c2-proto.py` and `r2c2-proto.fd8-rejected.py` (r2's, cited as comparison arms), **the
  frozen text [r3f]: `r3c2-proto.py`, `r3c2-fix1.py`, `r3c2-fix2.py`**, with `r3-instr.py`, and r4's: `r4c2-fix3.py`,
  `r4-clone.sh`, `r4-batch.sh`, `r4probe.sh`, `r4race.sh`, `r4race-batch.sh`, and r5's: `r5c2-fix4.py`, `r5-clone.sh`,
  `r5-batch.sh`, `r5probe.sh`. Inputs outside `cr2/bin/`: `cr2/work/shim-rm/rm` and `cr2/work/fx/clean` (`cr12.sh`; the
  latter made by `mkfx-clean.sh`, J19), `cr2/work/r5-ps-shim/ps` (CR11-J) and the frame clone `cr2-wt` at `706049ad`
  (every `*-clone.sh`).

---

## Change maps

**r2 (plan-review R1's F-ids → sections):** **F1** x§6.0 (T, R×T), x§6.2 (the reap set in files, `_K2_REAP_D`, three subshell sites), x§6.4 CR11-R, §10 CR11-R;
**F2** x§6.0 (E and its cells), x§6.1 CR#12 (the wider root), x§6.2 (`set +e` in `_k2_exit`, `_k2_sig`, mutations
:785), x§6.4 R12e, §9 G11-4/4b, §10 CR12-E; **F3** x§6.0 (L, D, F), x§6.1 (the watchdog's kept stream, found by F3's
measurement), x§6.2 (fd 9 saved, `9>&-`, the `exec`ing watchdog; the fd-8 form rejected by measurement), x§6.4 CR11-F,
§10 CR11-F/D/S; the STOP condition is not met: the leak is closed (f3fdleak, f3hang); **F4** x§6.2 (three trap lines),
x§6.4 (one control, three rows, R11h/i/t; probe; R11f measured), §9 G11-1h/i/t/p, BS-11; **F5** x§6.0 (invariants, 36
cells, POSIX references); **F6** §2.4 (R4a by backreference, the records.sh rule, prototyped), §10 CR4-5; **F7** §5
(the readers enumerated, the contract text), §10 CR14-G, §13; **F8** §2.3 item 3, §11 (B2a I-2's rule), CR2-A's files;
**F9** §7 (referrers, order), x§6.3 (X10 from `git log`), §11 (D before C2), CR-D3; **F10** §1 item (5), CR2-D's files
(§F row 12); **F11** §8 (file set), §11 (the sweep's copy in `…-cr2-sweep.md` §E, exclusions); **F12** §11 (reviewer:
default model); **F13** §3.2 (clause 3 narrowed, inside-span example), §9 BS-8 (measured backstop), BS-9..BS-12;
**F14** §10 CR-D2 (instrument for both forms); **F15** §10 CR-U (sixteen), CR2-M; **F16** x§6.2 (`_k2_reap`'s numeric
check; empty set) — with F2; **F17** x§6.4 R12 (the shim's scope); **F18** x§6.1 (one per run, the pgid-scoped
instrument), §10 CR11-R; **F19** x§6.4 CR11-R (claim row, measured reason, the `_wm` precedent considered); **F20** §9
(unpinned (reason) labels); **F21** §3.3 (line 81's comment and its reason); **F22** x§6.4 CR11-N (decided: `wait`,
measured); **F23** §10 CR7-4, CR2-B completion; **F24** §2.4, §10 CR4-3, §12 (`…-impl-cr.md` body unchanged; §T);
**F25** §8, §10 CR-8; **F26** §8 (provenance cites stay; "below" references), §11; **F27** O-2 (controls.sh); **F28**
§9 (no HUP claim row cited; HUP is a record); **F29** §7 (the range {R1–R9, R11} and its spellings); **F30** §3.4
(`MUTANTS` :469; the one unrelated hit); **F31** §3.3 (:102–:118); **F32** §10 CR4-2 (case-insensitive; the sites and
the unrelated hits); **F33** §7 (each L-reference's base); **F34** §3.4, §10 CR7-1 (dated corpus rows stay).

**r3 (R2's G-ids):** **G1** §5 (the contract text: before a launch / after it / the reap-set failure that does not return); x§6.2 (each
record before `wait`; a failed write `exit 2`s); §10 CR14-G (at C1's and C2's heads); §11 (ctlrun :228–:240 in C1's
and C2's files; "RETURN CONTRACT", `_rc`/`_out` in C2's sweep groups); **G2** x§6.0 invariant C and its cells; x§6.1
(the KILL that skips a child's exit path); x§6.2 (`$_K2_STOP`, one stop procedure for the reap and the watchdog);
x§6.4 R11w; §9 G11-2c, R15 (C under an ignored TERM); §10 CR11-C, CR11-W; **G3** §9 BS-11 (TERM; carried by C2's
files); x§6.4 (R11t under an ignored TERM; CR11-P for TERM); §11 C2's files (landing X3's text and its runner text);
§11's single-record runs; **G4** x§6.4 (the `_og` kill row, R11d and R11e, the equivalent first form of R11d); §9
G11-3, G11-3b (records); §10 CR11-D; **G5** §7 (the two instruments; current and hist, the rule); §11 CR2-D's files;
**G6** x§6.2 (per-launch records; a failed write ends the call); §9 BS-9 → R14, BS-10 → R15, BS-12 closed, G11-6a/b
(R11a, R11x); §7 (the range with R14, R15); §10 CR-D2; **G7** §11 (every record change is a corpus §6.1 row; each
slice's files name its rows); §10 CR-7; **G8** §1 (the carve's boundary); x§6.5 (its three edges); §10 CR11-T; **G9**
x§6.2 (the exit block verbatim; the `chmod` line and why `|| true` is dead); x§6.4 R12e on that text; **G10** §2.3
item 3 (the instrument, its 10 hits classified, :24–:25 and two sites outside its reach); §11 CR2-A's files; **G11**
x§6.0 invariant X and its cells; x§6.2 (mutations :785); §10 CR12-E (f2probe3 J); **G12** x§6.0 axis V and its cells;
x§6.2 (`env -u SHELLOPTS -u BASH_ENV`); §9 G11-8; §10 CR11-V; **G13** x§6.0 (K: entries > 1, the fail-safe direction);
x§6.2; §9 G11-9; §10 CR11-L; **G14** header (the frozen text [r3f]: `r3c2-proto.py` then its two frozen corrections);
x§ header (the text labels, and why the corrections are separate frozen files); x§6.4 (every record and arm on [r3f]);
x§6.6 (the index); **G15** x§6.1 (the watchdog's subshell and its `sleep 30`); §13; **G16** x§6.0 L×D (10 fd-9 cells
per shell); §10 CR11-F; §13; **G17** x§6.1 (the `rm` cause from `cr12-head.*`; r1's per-cause figures not cited);
x§6.0 S×E; §13; **G18** x§6.1 (instrumented clones); §13; **G19** x§6.4 CR11-P (with and without the probe on the one
text, three signals); **G20** x§6.0 axis P and its cells; §10 CR11-Q; **G21** §14 (route (b) runs the wire; C2 adds
f3hang and g12probe to X9's evidence list); **G22** §8 (comment-inclusive figures; stop lines ctlrun 439, wire 1360;
the reap-set writers' placement argued); x§6.3; §11 CR2-D (the wire's line count, measured); §10 CR-B; **G23** title
and header (eight items); O-3; §1 (6); **G24** §7 (the companion's bare-§ sentence, measured); §8 and §11 (the sweep's
file set gains the companion); **G25** §7 (umbrella :110, :22, :23, :111); §10 CR-U; §11 CR2-M; **G26** §11 C2's files
(landing :280, wire :412–:413); §10 CR11-1; **G27** §11 (model decided: CR2-B and CR2-C2 `model: opus`; the default
copy removed); **G28** O-2 (the umbrella; the re-frame rule); §11 (base); **G29** §10 CR11-F; x§6.4 (wire :1250);
**G30** §10 CR4-2 (the instrument's 16 hits, classified); **G31** §7 (banner :16); **G32** §10 CR-D2 (23 hits
classified); **G33** §10 CR-6 (corpus :373); **G34** §2.3 item 1 (the instrument, 7 hits classified); **G35** §14
"Instruments"; §11 CR2-M (§E copies every cited instrument; checked by grep); **G36** §2.4, §1 (5), §7
(`…-cr-plan.md`'s corrective note; no body rewrite); §11 (A, D, C2 add their items); **G37** §10 CR11-R (the top-level
noreap figure); **G38** x§6.0 R×T and the sites (every cited arm run on [r3f]); **G39** §2.3 item 1 (`.mvalues` beside
mutations :566); **G40** §8 (the 31 positional references classified, both directions across the seam); **Axis 2 MIN
(not named in G1–G36): the reap-set failure's cause hidden in a standing check's `2>` file; `_k2_reap_add`'s partial
write and no wait** x§6.2 (the cause on fd 9; recorded entries removed, then the groups stopped and waited).

**r4 (R3's H-ids; the record of r4's changes, replacing per-line retraction notes):**

| H | where |
|---|---|
| H1 | x§6.0 R, T; x§6.2 (one carrier: `$_K2_REAP_D` and the directories inside it; `_k2_reap` reads both levels); x§6.4 `_og` set; §9 G11-2; §10 CR11-R (the `_og` site) |
| H2 | x§6.2 root (7), `_k2_end`, `_ctl_left`, the `_wm` judgment; x§6.4 R11w; §1 (7); §5 (the contract's `_ctl_left`); §9 G11-10; §10 CR11-K, CR11-2 |
| H3 | x§6.0 K (identity: no live leader, or `ps -o etime=` no younger than the record); x§6.2; §9 G11-11, BS-12 narrowed (residue + consequence); §10 CR11-I. Fallback not needed (measured) |
| H4 | x§6.4 the `sig` row and record R11r; §9 G11-2 (a record); §10 CR11-R, CR-N (268); §8 records; §11 C2's files |
| H5 | x§0; §7 and §10 CR-U (umbrella :23/:111 register four CR2 files, seventeen); §11 CR2-M |
| H6 | §11 "The sweep" (exclusion list) |
| H7 | §11 CR2-M (re-pointing step and its grep, completion 0) |
| H8 | §11 "Type" (CR2-M, CR2-D, the citation-bearing doc parts → `designer`) |
| H9 | x§6.0 r3's axes: every "indep." with its reason; P×M's own cell |
| H10 | §9 BS-10 → R15 ([r3f] 1.77/1.86 s; the scratch evidence = `killonly`, CR11-C) |
| H11 | §9 BS-11 (R11w, and r4's R11r) |
| H12 | §9 BS-9 → R14, BS-10 → R15 (owner, own vs pre-existing, relation to R9 and the carve) |
| H13 | §1 (5), §7, §11 CR2-D (§F row 12 is a frame-tagged record → G36 corrective note) |
| H14 | O-2 (merge, not rebase; stop line re-derived); §11 (transplant = fast-forward); x§6.3, §8 (stop line = base + 90) |
| H15 | §11 (C2's brief and reviewer get both plan files) |
| H16 | §11 (a fix round at its slice's tier; rule 5 counted per unit) |
| H17 | x§6.2 (`set +m`, stop, then remove); §9 G11-12; §10 CR11-A; x§6.4 R11a re-anchored |
| H18 | x§6.0 C, C×R, C×I; x§6.2 (1–2 s; `-lt 2` kept: cleanup 0.26–0.37 s); §10 CR11-2, CR11-C |
| H19 | x§6.2 (the line's "decided nothing" is the call's; the run ends 1) |
| H20 | x§6.0 X×L (G no line, H one) |
| H21 | §2.4 (the log's own text cited) |
| H22 | §7 (instrument (1)'s extra hits classified: hist / other) |
| H23 | header (one rule: `x§6.n`, a file-named §, else this memo); every bare §6.n here now `x§6.n`; x§ header |
| H24 | change maps (G38: [r3f]) |
| H25 | x§6.6 (`cr2/logs/r3/f3fdleak3.txt`) |
| H26 | §9, §10 (`r3f/`, `r4/` on every log cite) |
| H27 | §7 (the split's pointer names §10 and §11 and the body's bare cites) |
| H28 | §14 (inputs outside `cr2/bin/`); §11 CR2-M (each copied script with its inputs) |

**r5 (R4's J-ids; [r5p] = [r4p] + `r5c2-fix4.py`, logs `r5/`):**

| J | where |
|---|---|
| J1 | x§6.0 K; x§6.2 (`_k2_age`; controls :216–:219's probe, exit 2); §9 G11-11, BS-12; §10 CR11-J; §14 (procps/busybox dropped) |
| J2 | x§6.4 `_og` call line (outer/inner, `9>&-`), sig row, R11n; x§6.6 (`r5/`); §9 G11-2, BS-11; §10 CR-N 269, CR-6; §8, §6, §11 C2's files (R11n) |
| J3 | §11 CR2-M (completion grep; `DECISIONS-R7.md` kept) |
| J4, J29 | §11 "Type" (the routing rule; order and clone) |
| J5 | x§6.0 K; §9 BS-12 residue (1) |
| J6 | x§6.0 R; x§6.2 `_k2_end` note; §1 CR#11; §9 G11-10 |
| J7 | §10 CR11-A (P10, P10s); §9 G11-12 |
| J8 | x§6.2 (`wait … \|\| :`; the precondition note dropped); §10 CR11-A (P8 bare) |
| J9 | §10 CR11-K (the head's `_k2_end` extracted) |
| J10 | §5 contract; §10 CR14-G |
| J11 | x§6.2 (`_k2_traps`); x§6.4 R11h/i/t, sig (`_og_trap`) |
| J12 | §9 G11-7 |
| J13 | x§6.0 C (the exception) |
| J14 | x§6.0 K (one clock, grep); §9 BS-12 residue (3) |
| J15 | §9 BS-12 (the cut write) |
| J16 | x§6.4 (`rm -rf …/og`: a red-run-only leftover; option one); §10 CR11-O |
| J17 | x§ header (pre-freeze runs: not evidence for the frozen text) |
| J18 | §11 transplant (clean worktree, else stop) |
| J19 | §11 CR2-M, §14 (`mkfx-clean.sh`) |
| J20 | §11 CR2-M (:23, four `## §0`) |
| J21 | §11 sweep, CR2-M ("the CR round's R3 H1") |
| J22 | §8 wire row (+90's reason); x§6.3 |
| J23 | §10 header (the CR2 files out of CR4-2, CR-D2, CR-S, CR-P) |
| J24 | §9 BS-9, BS-10 (direction) |
| J25 | x§6.4 norec/nodel (R11a once); §9 G11-12 |
| J26 | §10 CR11-K (N ≥ 600) |
| J27 | change maps H25; x§6.6 ("§6.0–§6.5") |
| J28 | §11 B2a I-2's C2 group |
