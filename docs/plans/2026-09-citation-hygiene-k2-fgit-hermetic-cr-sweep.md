# K2 fixture git — PR #527 CR round: the sweep ledger (G1)

Companion of the plan memo `…-cr-plan.md` (the handoff file `CR-plan-capability-census.md` until CR-B1's move; its §5.3 derives each slice's file list and
completion checks from this file). **Frame `2f1ca601`**, as the memo. Every path below is relative to the
handoff directory; the scripts are in `crplan-r3/bin/`, the logs in `crplan-r3/logs/`, all run on the clone
`crplan-r3/work/head` (`git clone --local`, checked out at `2f1ca601`); r4's (plan-review R3) are in `crplan-r4/`.
CR-B1 moves this file to `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-sweep.md` and adds §E, the four
instruments the slices run (`sweep.sh`, `idents.tsv`, `ledger-check.py`, `completion.sh`), as run; the logs stay in
the handoff directory, measured outside the repo (the plan memo's §7, "The move", R3 H1).

## §0 Spec coverage map

**No spec surface** — as the plan memo's §0: this file is the ledger of a shell trip-wire's self-test changes.
`preflight.py` exits 1 here by design, because no table follows this heading.

## §1 How the ledger is built

**The instrument is the mechanism's identifiers, not prose stems** (plan-review R2 G1: two rounds of stem greps
missed sites). `bin/idents.tsv` holds one row per identifier group (key, ERE, case flag); `bin/sweep.sh` runs
each as `/usr/bin/grep -nE` over the 26 files G1 names: the seven parts and the wire, the `docs/plans/*k2-fgit*`
files except this file and the plan memo once moved (eleven at the frame and after the move: `sweep.sh` excludes
`…-cr-plan.md` and `…-cr-sweep.md`, which state the identifiers rather than hold sites, R3 H1; **(M)**
`crplan-r4/logs/M-sweep-excl.txt`: 26 files and byte-identical output at the frame and with both files copied in,
both shells; without the exclusion 28 files), the umbrella, the #519 memo (`2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`),
`CLAUDE.md`, `.github/workflows/ci.yml` and the three `scripts/*`. **(M)** `logs/M-sweep-raw.txt` (raw; its counts in
`logs/M-sweep-summary.txt`): 702 hits,
none in `CLAUDE.md`, `ci.yml`, `scripts/ci-sweep.sh` or `scripts/doc-changed.sh`. One row per `file:line`
(a line that matches several groups is one row, its groups listed): `logs/sweep-sites.tsv`, **580 rows**
(`logs/M-sweep-summary.txt`, with the counts by class and slice below). `bin/classify.py` gives every row exactly one rule (file, line range, class, slice, disposition); a row with no
rule or two is an error, and the run exits 0 (`logs/ledger.tsv`). §3 is that file, rendered by
`bin/mkledger.py`; r4 edited dispositions only (rows 124, 126, 196, 198, 307, 310, 314-316, 549, 550, 559), site,
identifiers, class and slice unchanged (**(M)** `crplan-r4/logs/M-h1-inputs.txt`, its r4 line).

| key | identifiers (ERE) | why it is swept |
|---|---|---|
| `ctl_need` | `_ctl_need` | the prefix, removed (B2a) |
| `declined` | `\.declined` | the second log, removed (B2a) |
| `cn` | `(^\|[^A-Za-z0-9_])_cn([^A-Za-z0-9]\|$)` | `_cn`, `_cn_lbl`, `_cn_seen`, `_cn_declined`, `_cn_state`, the check's locals (B2a) |
| `cndir` | `\$_VFY/\.cn([^A-Za-z0-9]\|$)` | `_cn`'s own `_VFY` (B2a) |
| `gopt_ok` `k2_line` `goptprobe` | `_gopt_ok`, `_k2_line`, `goptprobe` | k2's probe, line and probe file, removed (B1) |
| `fifo_ok` `perm_ok` | `_fifo_ok`, `_perm_ok` | the flags: read through the table (B2a), their 0 declared (B2b) |
| `fifo_line` `perm_line` | `_fifo_line`, `_perm_line` | the summary lines, replaced by `_cap_lines` (B2a) |
| `machine_limits` `fw_limits` `notexh` | `machine_limits`, `_fw_limits`, `NOT EXERCISED on this machine` | P-f's limit declared (B2b); the automatic-decline wording |
| `mskip` `mut_trial` | `_mskip`, `_mut_trial\|_mrc2` | the excuse and `_mut_trial`'s return values (B2c) |
| `cap_table` | `_CAP_TABLE\|_cap_[a-z]\|_capt?_lbl\|_fifo_lbl\|_perm_lbl\|_pfl_lbl\|WEBREF_WIRE_LACKS\|goptgrep` | every name the slices add: 0 hits at the frame but one collision (row 311) |
| `callsite` | `call site\|outermost` (any case) | the census's site vocabulary (C1) |
| `standing` | `standing check` (any case) | the population changes (`_cn` → `_cap`); `_cap_check` is not one (G11) |
| `cen_lbl` | `_cen_lbl` | the census label: its clauses grow (B2a, C1) |
| `lbd` `lblconv` | `(^\|[^A-Za-z0-9_])_lbd([^A-Za-z0-9]\|$)\|\.vlabels`; `` _\*_lbl\|_lbl="…" `` | rule 1: the variable-label population and its prose (C1) |
| `controls_v` | `_CONTROLS` | the census reads it (C1, G4) |
| `registry` | `\.registry\|_ctl_reg\|\.ctllabels\|_mut_census_end` | rules 2 and 3 (C1) |
| `ratchets` | `_MUT_RECORDS_MIN\|_MUT_UNRECORDED_MAX\|_MUT_TARGETS` | the figures every slice sets; `_MUT_TARGETS` derived (C2) |
| `wrongreason` | `wrong reason` (any case) | `_mut_trial`'s wrong-reason arm returns 3 (B2c) |
| `grepopt` | `GREP_OPTIONS` | k2 posed through the shim (B1) |
| `family` | `impl-cr` | the sites that list the memo family: the moved plan memo registers there (B1, G1) |
| `slot` `x9` | `#11-trip-wire-launch-environment`; `X9\|route \(b\)` | BS-3's member (B2b); Q1's route (B2b, G2) |

**A row is the statement that holds the hit**, not the hit line alone: the comment block, paragraph, bullet or
table row it sits in (prose wraps, which is how stem greps missed sites). The disposition names the extent
where it is wider than the line, e.g. row 85 (`controls:403`) covers the comment :395-401 above the code.

**Classes.** `code`: an executable line the slice edits or removes. `stmt`: a present-tense statement the slice
rewrites. `dod`: a completion or exit criterion, a ledger, slot or residual text. `cons`: a figure, needle or
anchor the change moves. `carried`: a statement of the #519 memo, which D5 keeps untouched, corrected by a
residual row (R10, R12, R13 in the memo's §7). `stays`: true after every slice (the reason is in the row).
`hist`: a dated record of past rounds (`…-impl.md`, `…-impl-cr.md`, `…-pr527*.md`, `…-reviews.md`,
`…-provenance.md` whole; corpus §6.1's change rows; dated history inside a comment). `unrel`: the identifier
matched another mechanism.

**Site names.** `controls`, `ctlrun`, `fixtures`, `harness`, `mutations`, `mutgen`, `records` = the parts;
`wire` = `webref-generic-core-trip-wire.sh`; `design` = `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md`;
`census`, `corpus`, `impl`, `impl-cr`, `landing`, `pr527`, `pr527-r15`, `provenance`, `residuals`, `reviews` =
its `-<name>.md` companions; `519` = the #519 memo; `umbrella` = `2026-07-citation-hygiene-umbrella.md`.

## §2 What each slice owns (derived)

| | rows | by class | files (lines at the frame) |
|---|---|---|---|
| CR-B1 | 34 | code 21, stmt 7, dod 4, carried 2 | controls :25-26, :600-613, :694; harness :10, :830-855; mutations :404-405; records :367, :373; corpus :203; design :16-17; umbrella :23, :111; 519 :34-35 (R10) |
| CR-B2a | 122 | code 90, stmt 32 | controls :24-26, :102, :110, :219, :259, :282-383, :487-489, :508-517, :566-569, :643-657, :675-678, :692-693; ctlrun :19, :24, :39-46, :75, :108-141, :171-178, :222, :351-354; mutations :21, :481, :498; records :157-159, :364-377 (but :373); design :476, :510, :616 |
| CR-B2b | 14 | stmt 5, carried 5, dod 4 | controls :395-403; harness :343-349; design :605, :609; corpus :194-200; landing :121-127, :183-186, :386; residuals :111; 519 :427, :446, :452, :1274, :1391 (R12) |
| CR-B2c | 17 | code 13, stmt 4 | mutations :25, :335-346, :411, :709-747; records :277-279; corpus :194-200 |
| CR-C1 | 44 | stmt 32, code 10, cons 1, carried 1 | controls :18, :76, :98, :136; ctlrun :72-78; mutations :17-22, :52, :92, :180, :195-196, :478-526; records :328-356; design :589, :615, :620, :691; 519 :29 (R13) |
| CR-C2 | 4 | code 3, cons 1 | controls :138, :143; mutations :235, :238-248 |
| every slice | 4 | cons 4 | mutations :237, :248; corpus §6.1 Current totals :334-336 |

233 rows are owned (six by two slices: controls :25, :26 and records :367 by B1 and B2a, mutations :21 by B2a and
C1, mutations :248 by every slice and C2, corpus :194 by B2b and B2c); 347 are `stays` 138, `hist` 199 and
`unrel` 10.

**Each slice's completion** is derived from these rows, by three checks:
1. **`bin/ledger-check.py <clone> <slice>`**: every row owned by the slice or an earlier one (B1 → B2a → B2b →
   B2c → C1 → C2) has its frame text gone from its file; every other row (a later slice's, `stays`, `hist`,
   `unrel`, `carried`) has it still there, counted per identical line. `each` rows are the ratchets' own.
   **(M)** negative control at the frame, `logs/M-ledger-check-frame.txt`: each slice FAILs with exactly its
   owned rows as violations (B1 32, B2a 151, B2b 160, B2c 176, C1 218, C2 221: cumulative, `carried` and `each`
   rows excluded).
2. **`bin/completion.sh <clone>`**: the 0-hit greps of the identifiers a slice removes, and the phrases a slice
   retires. **(M)** at the frame, `logs/M-completion-frame.txt`: B1 over the parts and the wire 18, over the
   design memo, census, landing and residuals 0, corpus 2 (:203, and :321, a §6.1 history row that stays);
   `three capability probes` 1; B2a over the parts and the wire 112 (the previous revision's grep reached 108:
   it missed the four `$_VFY/.cn` lines :285, :319, :320, :363), over the same four docs 3 (design :476, :510,
   :616; corpus is left out: its §6.1 rows :312, :321 are history); B2b's four phrases 5 (controls :396, :400,
   harness :349, design :609, and mutations :407, which B1 removes); B2c's hash 1; C2's `_MUT_TARGETS="` 1 and
   the part path 13 (the ten `${SELF%.sh}.` sites, wire:1235 and two comment lines). The part-path population is
   13 code sites, the ten and the three quoted-stem globs mutations :430, :432, :686; H6's grep, which adds
   `"\$\{SELF%\.sh\}"\.`, prints 16 at the frame (`crplan-r4/logs/M-partpath4.txt`; the plan memo's §3.4).
3. **`bin/sweep.sh` re-run at the slice head**: every site not in this ledger (a new mention) is classified in
   the slice's own record entry.

## §3 The ledger

| # | site | identifiers | class | slice | disposition |
|---|---|---|---|---|---|
| 1 | controls:18 | controls_v | stmt | C1 | interface comment :15-22: `$_CONTROLS` is read by the entry check, the site walk and the census, not by `_lbd` (D13/A3-7) |
| 2 | controls:24 | perm_line | stmt | B2a | :23-31 names `$_perm_line`/`$_fifo_line` among names the wire never read: the names go |
| 3 | controls:25 | k2_line fifo_line | stmt | B1,B2a | :25-26 `$_k2_line` (B1) and `$_fifo_line`/`$_perm_line` (B2a) leave the sentence and its grep command |
| 4 | controls:26 | k2_line fifo_line perm_line | stmt | B1,B2a | :25-26 `$_k2_line` (B1) and `$_fifo_line`/`$_perm_line` (B2a) leave the sentence and its grep command |
| 5 | controls:34 | controls_v | stays | - | history inside a comment (the consumes list was wrong twice); true |
| 6 | controls:65 | controls_v | stays | - | controls entry guard: `_CONTROLS` stays consumed |
| 7 | controls:76 | lblconv | stmt | C1 | :75-78 "the census counts the `_*_lbl` variables": rule 1 counts only the expanded ones |
| 8 | controls:98 | lblconv | stmt | C1 | :98-99 the census's sources, as at :76 |
| 9 | controls:100 | cen_lbl | stays | - | `_cen_lbl` definition; text unchanged (the new census clauses report under it; design memo row C is rewritten, C1) |
| 10 | controls:102 | standing | stmt | B2a | :101-104 the non-`_control` producers: the table check (`$_capt_lbl`) and the group/limit labels join; "three standing checks" stays (G11) |
| 11 | controls:107 | lbd | stays | - | `_lbd_lbl` text kept: the check grows (C1 reads the wire too) and the label states a subset of it; design memo row L is rewritten (C1) |
| 12 | controls:110 | ctl_need cn | code | B2a | `_cn_lbl` removed |
| 13 | controls:136 | lbd | code | C1 | `_lbd_files` gains `$SELF` (rule 1: `_lbd` reads the wire) |
| 14 | controls:137 | lbd | stays | - | loop over `_MUT_PARTS` |
| 15 | controls:138 | lbd | code | C2 | part path spelled here: the one helper (CL10/R-5) |
| 16 | controls:139 | lbd | stays | - | `_lbd` body |
| 17 | controls:142 | lbd | stays | - | `_lbd` body |
| 18 | controls:143 | lbd | code | C2 | the dead empty-`_lbd_files` guard (:141-143, CL6/S-7): with `$SELF` always in the list it is dead by construction |
| 19 | controls:144 | lbd | stays | - | `_lbd` body |
| 20 | controls:145 | lbd | stays | - | `_lbd` body |
| 21 | controls:146 | lbd | stays | - | `_lbd` body |
| 22 | controls:148 | lbd | stays | - | `_lbd` body |
| 23 | controls:149 | lbd | stays | - | `_lbd` body |
| 24 | controls:151 | lbd | stays | - | `_lbd` body |
| 25 | controls:152 | lbd | stays | - | `_lbd` body |
| 26 | controls:198 | standing | stays | - | "THE THREE STANDING CHECKS BELOW": `_wm`, `_ev`, `_cap` (G11: `_cap_check` is a table check, its call sits above this comment) |
| 27 | controls:203 | standing | stays | - | `_wm` header; record 361 anchors it |
| 28 | controls:219 | cn | stmt | B2a | "as `_cn`'s is" -> "as `_cap`'s is" |
| 29 | controls:251 | standing | stays | - | `_ev` header |
| 30 | controls:259 | cn | stmt | B2a | "as `_cn`'s is" -> "as `_cap`'s is" |
| 31 | controls:282 | ctl_need standing | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 32 | controls:285 | cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 33 | controls:286 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 34 | controls:287 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 35 | controls:289 | lblconv | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 36 | controls:290 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 37 | controls:295 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 38 | controls:299 | ctl_need | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 39 | controls:301 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 40 | controls:313 | declined cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 41 | controls:314 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 42 | controls:315 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 43 | controls:316 | declined | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 44 | controls:318 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 45 | controls:319 | cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 46 | controls:320 | cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 47 | controls:322 | cn cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 48 | controls:323 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 49 | controls:324 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 50 | controls:325 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 51 | controls:326 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 52 | controls:327 | ctl_need cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 53 | controls:328 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 54 | controls:329 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 55 | controls:330 | declined cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 56 | controls:331 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 57 | controls:332 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 58 | controls:333 | cn cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 59 | controls:334 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 60 | controls:335 | ctl_need cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 61 | controls:336 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 62 | controls:337 | ctl_need cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 63 | controls:338 | ctl_need cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 64 | controls:339 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 65 | controls:341 | ctl_need declined cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 66 | controls:342 | ctl_need cn cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 67 | controls:343 | ctl_need cn cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 68 | controls:344 | ctl_need declined cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 69 | controls:345 | ctl_need cn cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 70 | controls:346 | ctl_need cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 71 | controls:358 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 72 | controls:361 | cn | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 73 | controls:362 | declined cn standing | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 74 | controls:363 | cndir | code | B2a | `_cn` standing check, its comment and `_cn_state` (:282-363) removed; `_cap` replaces it |
| 75 | controls:365 | declined | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 76 | controls:369 | ctl_need | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 77 | controls:372 | registry | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 78 | controls:373 | cen_lbl | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 79 | controls:374 | cn | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 80 | controls:375 | cn | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 81 | controls:378 | declined | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 82 | controls:379 | cn | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 83 | controls:382 | cn | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 84 | controls:383 | declined | code | B2a | the asks `_cn_seen`/`_cn_declined` and their comment (:364-387) removed |
| 85 | controls:403 | fw_limits notexh | stmt | B2b | extent :395-403. Code :402-403 stays (limit lines printed); comment :395-401 rewritten: P-f declared, P-b a fact, the slot rule as the reason (§9) |
| 86 | controls:443 | callsite | stays | - | "at the call site" = the call's text: the sense rule 2 keeps |
| 87 | controls:487 | ctl_need | stmt | B2a | green-direction count comment :485-491: the prefix clause goes |
| 88 | controls:489 | ctl_need | stmt | B2a | green-direction count comment :485-491: the prefix clause goes |
| 89 | controls:508 | perm_line | code | B2a | FIFO pair: comment :507-514, `_fifo_line` :515-516, the prefix on :517 (record 372 goes with it) |
| 90 | controls:512 | ctl_need fifo_ok | code | B2a | FIFO pair: comment :507-514, `_fifo_line` :515-516, the prefix on :517 (record 372 goes with it) |
| 91 | controls:515 | fifo_line | code | B2a | FIFO pair: comment :507-514, `_fifo_line` :515-516, the prefix on :517 (record 372 goes with it) |
| 92 | controls:517 | ctl_need fifo_ok | code | B2a | FIFO pair: comment :507-514, `_fifo_line` :515-516, the prefix on :517 (record 372 goes with it) |
| 93 | controls:550 | callsite | stays | - | as :443 |
| 94 | controls:566 | fifo_line | code | B2a | FIFO pair: :566 comment, :567 prefix, :568-574 ask |
| 95 | controls:567 | ctl_need fifo_ok | code | B2a | FIFO pair: :566 comment, :567 prefix, :568-574 ask |
| 96 | controls:568 | cn fifo_ok | code | B2a | FIFO pair: :566 comment, :567 prefix, :568-574 ask |
| 97 | controls:569 | fifo_line notexh | code | B2a | FIFO pair: :566 comment, :567 prefix, :568-574 ask |
| 98 | controls:600 | grepopt | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 99 | controls:605 | gopt_ok | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 100 | controls:606 | k2_line | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 101 | controls:608 | k2_line grepopt | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 102 | controls:609 | ctl_need gopt_ok grepopt | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 103 | controls:610 | cn gopt_ok | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 104 | controls:611 | k2_line notexh grepopt | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 105 | controls:612 | grepopt | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 106 | controls:613 | grepopt | code | B1 | k2: comment :600-607 rewritten for the shim, `_k2_line` :608 and :610-614 removed, :609 the shim call (record 373 goes) |
| 107 | controls:643 | ctl_need cn perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 108 | controls:648 | perm_line notexh | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 109 | controls:651 | ctl_need perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 110 | controls:652 | ctl_need perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 111 | controls:653 | ctl_need perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 112 | controls:654 | ctl_need perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 113 | controls:655 | ctl_need perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 114 | controls:656 | cn perm_ok | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 115 | controls:657 | perm_line | code | B2a | perm group: comment :639-647 (the R78 wording :642-643 reversed in letter, §1 item 4), `_perm_line` :648-650 and :656-658, prefixes :651-655 |
| 116 | controls:675 | cn | code | B2a | the last ask and its comment |
| 117 | controls:677 | registry | code | B2a | the last ask and its comment |
| 118 | controls:678 | cn | code | B2a | the last ask and its comment |
| 119 | controls:692 | fifo_line | code | B2a | prints -> `_cap_lines` |
| 120 | controls:693 | perm_line | code | B2a | prints -> `_cap_lines` |
| 121 | controls:694 | k2_line | code | B1 | `_k2_line` print removed (the claim joins the static lines :688-691) |
| 122 | controls:697 | registry | stays | - | `_mut_census_end` unchanged (§3.2 item 4) |
| 123 | controls:700 | registry | stays | - | `_mut_census_end` unchanged (§3.2 item 4) |
| 124 | ctlrun:19 | ctl_need | stmt | B2a | WHAT IT CONSUMES: `_ctl_need` goes; `_CAP_TABLE` and the flags by the table's names join (G3); ctlrun is their only reader (R3 H4) |
| 125 | ctlrun:21 | controls_v | stays | - | `$_CONTROLS` consumed by the site walk |
| 126 | ctlrun:24 | registry | stmt | B2a | WHAT IT DEFINES :24-28 gains `_cap_rows`, `_cap_group_of`, `_cap_flag_of`, `_cap_g`, `_cap_f`, `_cap_v` and the reader's internal names, with their outside readers (G12 A1, R3 H4) |
| 127 | ctlrun:39 | ctl_need | code | B2a | :37-46 reworded (no per-call name; the source-time guard of G3 follows), `unset _ctl_need` removed |
| 128 | ctlrun:46 | ctl_need | code | B2a | :37-46 reworded (no per-call name; the source-time guard of G3 follows), `unset _ctl_need` removed |
| 129 | ctlrun:66 | standing | stays | - | "three standing checks": `_wm`, `_ev`, `_cap` |
| 130 | ctlrun:72 | registry | stmt | C1 | registry header :72-101: the site rule as the bijection (§3.2 item 6) |
| 131 | ctlrun:75 | ctl_need standing | stmt | B2a | :75-77 the `_ctl_need` standing check's own `_VFY` sentence goes (`_cap` registers in the run's registry) |
| 132 | ctlrun:77 | callsite lblconv | stmt | C1 | "THE SITE IS THE OUTERMOST FRAME": kept as the mechanism, the bijection added (item 6) |
| 133 | ctlrun:78 | controls_v | stmt | C1 | "THE SITE IS THE OUTERMOST FRAME": kept as the mechanism, the bijection added (item 6) |
| 134 | ctlrun:100 | registry | stays | - | `_mut_census_end` sentence unchanged |
| 135 | ctlrun:102 | registry | stays | - | `_ctl_reg` unchanged (GC-5 mutates it) |
| 136 | ctlrun:105 | controls_v | stays | - | `_ctl_reg` unchanged (GC-5 mutates it) |
| 137 | ctlrun:106 | registry | stays | - | `_ctl_reg` unchanged (GC-5 mutates it) |
| 138 | ctlrun:108 | ctl_need | stmt | B2a | gate comment :108-127 rewritten for the table lookup |
| 139 | ctlrun:116 | declined cn | stmt | B2a | gate comment :108-127 rewritten for the table lookup |
| 140 | ctlrun:119 | registry | stmt | B2a | gate comment :108-127 rewritten for the table lookup |
| 141 | ctlrun:121 | ctl_need | stmt | B2a | gate comment :108-127 rewritten for the table lookup |
| 142 | ctlrun:123 | cn standing | stmt | B2a | gate comment :108-127 rewritten for the table lookup |
| 143 | ctlrun:127 | standing | stmt | B2a | gate comment :108-127 rewritten for the table lookup |
| 144 | ctlrun:129 | registry | stays | - | `_ctl_reg "$1"`, the gate's first statement; record 353 anchors it |
| 145 | ctlrun:130 | ctl_need | code | B2a | need arms :130-131, :137-142 -> `_cap_group_of` arms |
| 146 | ctlrun:131 | ctl_need | code | B2a | need arms :130-131, :137-142 -> `_cap_group_of` arms |
| 147 | ctlrun:138 | declined | code | B2a | need arms :130-131, :137-142 -> `_cap_group_of` arms |
| 148 | ctlrun:139 | declined | code | B2a | need arms :130-131, :137-142 -> `_cap_group_of` arms |
| 149 | ctlrun:141 | ctl_need | code | B2a | need arms :130-131, :137-142 -> `_cap_group_of` arms |
| 150 | ctlrun:154 | standing | stays | - | `_wm` sets the watchdog |
| 151 | ctlrun:159 | mut_trial | stays | - | mutant trials unbounded (slot `#11-trip-wire-liveness-bound`) |
| 152 | ctlrun:171 | ctl_need | stmt | B2a | RETURN CONTRACT :170-183: 1 and 2 restated (table decline; undeclared group) |
| 153 | ctlrun:177 | ctl_need | stmt | B2a | RETURN CONTRACT :170-183: 1 and 2 restated (table decline; undeclared group) |
| 154 | ctlrun:178 | declined | stmt | B2a | RETURN CONTRACT :170-183: 1 and 2 restated (table decline; undeclared group) |
| 155 | ctlrun:183 | standing | stays | - | `_wm` asserts the watchdog group gone |
| 156 | ctlrun:213 | grepopt | stays | - | the wire's `unset GREP_OPTIONS` (the wire is not edited) |
| 157 | ctlrun:222 | cn standing | stmt | B2a | "a row of the `_cn` standing check" -> `_cap`'s X row |
| 158 | ctlrun:269 | standing | stays | - | `_wm` pins reaping |
| 159 | ctlrun:320 | standing | stays | - | `_wm` pins the timeout clause |
| 160 | ctlrun:351 | ctl_need | stmt | B2a | `_control`'s comment :351-354: "the table declined it"; :361 and its reason :357-360 stay |
| 161 | ctlrun:354 | cn | stmt | B2a | `_control`'s comment :351-354: "the table declined it"; :361 and its reason :357-360 stay |
| 162 | ctlrun:373 | wrongreason | unrel | - | `_control`'s own "wrong reason" diagnostic (not `_mut_trial`'s) |
| 163 | fixtures:17 | fifo_ok | stays | - | the window consumes `$_fifo_ok` (forwarded by the prelude): a reader outside the table, named beside BS-1 (G12 A1) |
| 164 | fixtures:59 | fifo_ok | stays | - | the window's `odd` FIFO: reads the flag, not the table (G12 A1) |
| 165 | fixtures:496 | fifo_ok | stays | - | the window's `fifotracked` build: reads the flag (G12 A1) |
| 166 | harness:10 | grepopt | stmt | B1 | header :8-10 "the three capability probes (FIFO, file permissions, `GREP_OPTIONS`)" -> two |
| 167 | harness:59 | wrongreason | hist | - | PR519 history in a comment |
| 168 | harness:159 | fw_limits | stays | - | `_fw_limits` initialised |
| 169 | harness:173 | machine_limits | stays | - | return channel list |
| 170 | harness:341 | machine_limits | stays | - | P-b limit line: a `fact` row, still automatic |
| 171 | harness:348 | notexh | stmt | B2b | extent :343-349: P-f's limit "green — as for the FIFO and file-permission controls" -> declared |
| 172 | harness:362 | machine_limits | stays | - | P-f limit line printed (its acceptance moves to `_cap_check`) |
| 173 | harness:614 | fw_limits | stays | - | window outputs; prelude forwards `_fifo_ok` |
| 174 | harness:623 | fifo_ok | stays | - | window outputs; prelude forwards `_fifo_ok` |
| 175 | harness:709 | machine_limits | stays | - | markers read; `_fw_limits` from `machine_limits` |
| 176 | harness:729 | machine_limits fw_limits | stays | - | markers read; `_fw_limits` from `machine_limits` |
| 177 | harness:737 | standing | stays | - | "the controls file's three standing checks read [the flag]" |
| 178 | harness:753 | registry | stays | - | the window verdict registers |
| 179 | harness:815 | perm_ok | stays | - | the FIFO and permission probes; their 0 is now declared (§2.3), the probe lines are the GB2b-1/2 anchors |
| 180 | harness:817 | fifo_ok | stays | - | the FIFO and permission probes; their 0 is now declared (§2.3), the probe lines are the GB2b-1/2 anchors |
| 181 | harness:818 | fifo_ok | stays | - | the FIFO and permission probes; their 0 is now declared (§2.3), the probe lines are the GB2b-1/2 anchors |
| 182 | harness:822 | fifo_ok | stays | - | the FIFO and permission probes; their 0 is now declared (§2.3), the probe lines are the GB2b-1/2 anchors |
| 183 | harness:826 | perm_ok | stays | - | the FIFO and permission probes; their 0 is now declared (§2.3), the probe lines are the GB2b-1/2 anchors |
| 184 | harness:828 | perm_ok | stays | - | the FIFO and permission probes; their 0 is now declared (§2.3), the probe lines are the GB2b-1/2 anchors |
| 185 | harness:830 | grepopt | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 186 | harness:831 | grepopt | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 187 | harness:835 | fifo_ok perm_ok | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 188 | harness:840 | grepopt | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 189 | harness:846 | ctl_need | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 190 | harness:848 | gopt_ok | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 191 | harness:849 | goptprobe grepopt | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 192 | harness:852 | goptprobe | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 193 | harness:853 | goptprobe grepopt | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 194 | harness:855 | gopt_ok | code | B1 | `_gopt_ok` probe and comment (:830-857) removed |
| 195 | mutations:17 | lblconv | stmt | C1 | :15-18 "the `_*_lbl` variables": the expanded ones (rule 1) |
| 196 | mutations:21 | cen_lbl registry | stmt | B2a,C1 | WHAT IT CONSUMES :20-31: ctlrun's reader functions `_cap_rows`, `_cap_group_of`, `_cap_flag_of`, not the table or a flag (B2a, R3 H4), `$_CONTROLS` (C1, G4) |
| 197 | mutations:22 | lbd lblconv | stmt | C1 | `$_lbd_names`: the census keeps the expanded subset (rule 1) |
| 198 | mutations:25 | fw_limits | stmt | B2c | `$_fw_limits` for `_mskip`: `_mskip` also asks `_cap_group_of` and `_cap_flag_of` (the functions are named here; the table and the flags are not, R3 H4) |
| 199 | mutations:38 | ratchets | stays | - | WHAT IT DEFINES (names unchanged) |
| 200 | mutations:39 | ratchets | stays | - | WHAT IT DEFINES (names unchanged) |
| 201 | mutations:40 | registry | stays | - | WHAT IT DEFINES (names unchanged) |
| 202 | mutations:42 | mut_trial | stays | - | WHAT IT DEFINES (names unchanged) |
| 203 | mutations:52 | lblconv | stmt | C1 | :49-57 the population's unit "the value of a `_*_lbl` variable": an expanded one |
| 204 | mutations:54 | ratchets | stays | - | `_MUT_UNRECORDED_MAX` ratchets the bare count |
| 205 | mutations:92 | lbd | code | C1 | entry guard gains `_CONTROLS` (G4) |
| 206 | mutations:172 | ratchets | stays | - | prefixes are `_MUT_TARGETS`'s words (derived after C2, same name) |
| 207 | mutations:180 | lblconv | stmt | C1 | :180-181 "a `_control` or an `_lbl=` definition": a call line or an expansion (rule 1) |
| 208 | mutations:195 | lblconv | stmt | C1 | :192-197 the correspondence: `eval`/split-word calls banned, the variable half by expansion |
| 209 | mutations:196 | lbd | stmt | C1 | :192-197 the correspondence: `eval`/split-word calls banned, the variable half by expansion |
| 210 | mutations:200 | ratchets | stays | - | EXACT ratchet |
| 211 | mutations:235 | ratchets | code | C2 | `_MUT_TARGETS` derived: `_MUT_PARTS` minus `records` (C5) |
| 212 | mutations:237 | ratchets | cons | each | `_MUT_UNRECORDED_MAX=18`: set by each slice to its measured bare count |
| 213 | mutations:248 | ratchets | cons | each,C2 | extent :238-248: the floor set by each slice to its record count; the changelog :239-247 goes (C2, CL7) |
| 214 | mutations:250 | ratchets | stays | - | `_MUT_TARGETS` / `_mut_trial` readers |
| 215 | mutations:251 | mut_trial | stays | - | `_MUT_TARGETS` / `_mut_trial` readers |
| 216 | mutations:276 | ratchets | stays | - | `_MUT_TARGETS` / `_mut_trial` readers |
| 217 | mutations:295 | ratchets | stays | - | `_MUT_TARGETS` / `_mut_trial` readers |
| 218 | mutations:319 | ratchets | stays | - | `_MUT_TARGETS` / `_mut_trial` readers |
| 219 | mutations:337 | ratchets | stays | - | `_MUT_TARGETS` / `_mut_trial` readers |
| 220 | mutations:346 | mut_trial | code | B2c | extent :335-346: `_mut_trial` returns 3 for its wrong-reason arm with mutant rc 1 only (G10); header :344-345 restated |
| 221 | mutations:377 | registry | stays | - | `.ctllabels` read by the `!kill` arm (rule 3 produces it from one pass) |
| 222 | mutations:388 | registry | stays | - | `.ctllabels` read by the `!kill` arm (rule 3 produces it from one pass) |
| 223 | mutations:394 | registry | stays | - | `.ctllabels` read by the `!kill` arm (rule 3 produces it from one pass) |
| 224 | mutations:404 | k2_line | stmt | B1 | extent :401-408: the K15 example without `_k2_line`/`_gopt_ok`, and without the clause quoting the controls file's `_limit_lines` reason (B2b rewrites that reason) |
| 225 | mutations:405 | gopt_ok | stmt | B1 | extent :401-408: the K15 example without `_k2_line`/`_gopt_ok`, and without the clause quoting the controls file's `_limit_lines` reason (B2b rewrites that reason) |
| 226 | mutations:411 | wrongreason | code | B2c | wrong-reason arm: 3 when `$_mt_rc` is 1, else 2 (G10); the caller prints it when not excused |
| 227 | mutations:478 | registry | stmt | C1 | census comment :477-510 (item 6): one parse, the bijection |
| 228 | mutations:479 | registry | stmt | C1 | census comment :477-510 (item 6): one parse, the bijection |
| 229 | mutations:481 | ctl_need standing | stmt | B2a | "the `_ctl_need` standing check runs under one of its own" goes |
| 230 | mutations:484 | lbd lblconv | stmt | C1 | census comment, as :478 |
| 231 | mutations:485 | lbd | stmt | C1 | census comment, as :478 |
| 232 | mutations:486 | registry | stmt | C1 | census comment, as :478 |
| 233 | mutations:497 | registry | stmt | C1 | census comment, as :478 |
| 234 | mutations:498 | ctl_need | stmt | B2a | :498-501 "registered all the same (`_ctl_need` …)" -> the table |
| 235 | mutations:501 | lbd | stmt | C1 | "a run that ends before this point is red without it (… `_lbd` …)": stays true; in the rewritten paragraph |
| 236 | mutations:511 | registry | stays | - | `.census` note (item 4) |
| 237 | mutations:512 | lbd | code | C1 | `.vlabels` from the expanded names (rule 1); record 356 anchors this line -> re-aimed |
| 238 | mutations:513 | cen_lbl | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 239 | mutations:517 | lblconv | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 240 | mutations:521 | callsite | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 241 | mutations:522 | callsite | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 242 | mutations:524 | lbd registry | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 243 | mutations:525 | registry | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 244 | mutations:526 | lbd registry | code | C1 | the three registry extractions -> one `awk -F'\t'` pass (rule 3); messages kept |
| 245 | mutations:528 | cen_lbl | stays | - | `.bare` / orphan scan |
| 246 | mutations:534 | lblconv | stays | - | `.bare` / orphan scan |
| 247 | mutations:615 | ratchets | stays | - | floor, ratchet, `_mut_census_end` |
| 248 | mutations:616 | ratchets | stays | - | floor, ratchet, `_mut_census_end` |
| 249 | mutations:627 | ratchets | stays | - | floor, ratchet, `_mut_census_end` |
| 250 | mutations:628 | cen_lbl ratchets | stays | - | floor, ratchet, `_mut_census_end` |
| 251 | mutations:634 | ratchets | stays | - | floor, ratchet, `_mut_census_end` |
| 252 | mutations:635 | cen_lbl ratchets | stays | - | floor, ratchet, `_mut_census_end` |
| 253 | mutations:650 | registry | stays | - | floor, ratchet, `_mut_census_end` |
| 254 | mutations:653 | registry | stays | - | floor, ratchet, `_mut_census_end` |
| 255 | mutations:655 | cen_lbl | stays | - | floor, ratchet, `_mut_census_end` |
| 256 | mutations:656 | registry | stays | - | floor, ratchet, `_mut_census_end` |
| 257 | mutations:674 | wrongreason | hist | - | history inside a comment |
| 258 | mutations:709 | machine_limits | code | B2c | `_mskip` (:708-726): the table half; :716's commit hash goes (CL7) |
| 259 | mutations:715 | lbd | code | B2c | `_mskip` (:708-726): the table half; :716's commit hash goes (CL7) |
| 260 | mutations:717 | mskip | code | B2c | `_mskip` (:708-726): the table half; :716's commit hash goes (CL7) |
| 261 | mutations:721 | lbd | code | B2c | `_mskip` (:708-726): the table half; :716's commit hash goes (CL7) |
| 262 | mutations:722 | mskip | code | B2c | `_mskip` (:708-726): the table half; :716's commit hash goes (CL7) |
| 263 | mutations:725 | fw_limits | code | B2c | `_mskip` (:708-726): the table half; :716's commit hash goes (CL7) |
| 264 | mutations:736 | mut_trial | code | B2c | the caller: a capability excuse accepts 1 or 3, P-x 1; an unexcused 3 is red (F7) |
| 265 | mutations:737 | mut_trial | code | B2c | the caller: a capability excuse accepts 1 or 3, P-x 1; an unexcused 3 is red (F7) |
| 266 | mutations:743 | mskip mut_trial | code | B2c | the caller: a capability excuse accepts 1 or 3, P-x 1; an unexcused 3 is red (F7) |
| 267 | mutations:744 | mskip | code | B2c | the caller: a capability excuse accepts 1 or 3, P-x 1; an unexcused 3 is red (F7) |
| 268 | mutations:747 | mut_trial | code | B2c | the caller: a capability excuse accepts 1 or 3, P-x 1; an unexcused 3 is red (F7) |
| 269 | mutgen:13 | mut_trial | stays | - | consumes `_mut_trial` |
| 270 | mutgen:324 | mut_trial | stays | - | `!kill` trials: that arm returns 0/1/2 only (3 is the needle arm's, G10), so :325-326 stay exhaustive |
| 271 | records:100 | grepopt | stays | - | record 100 (k2), killed on every profile (§2.1) |
| 272 | records:153 | standing | stays | - | "the three standing checks that read `_fw_trusted` print NOTHING" |
| 273 | records:157 | cn standing | stmt | B2a | W2 comment :156-159: what removing `_ctl_run`'s ask reds under, re-measured (`$_cn_lbl` goes) |
| 274 | records:159 | cen_lbl | stmt | B2a | W2 comment :156-159: what removing `_ctl_run`'s ask reds under, re-measured (`$_cn_lbl` goes) |
| 275 | records:278 | perm_ok | stmt | B2c | :277-279 -> "where `_perm_ok` is 1" (the excuse makes the decline a declared state) |
| 276 | records:283 | standing | stays | - | `_wm` record comment |
| 277 | records:328 | cen_lbl | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 278 | records:329 | lbd lblconv | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 279 | records:336 | registry | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 280 | records:338 | lbd lblconv | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 281 | records:342 | callsite | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 282 | records:344 | callsite | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 283 | records:350 | lblconv | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 284 | records:351 | lblconv | stmt | C1 | census records comment :328-352: the GC records join; the site vocabulary stays (the bijection is over the same sites) |
| 285 | records:353 | registry | stays | - | record: the gate's `_ctl_reg` removed |
| 286 | records:356 | lbd | cons | C1 | record anchored on mutations.sh:512, which rule 1 rewrites: re-aimed on the new `.vlabels` line (same needle) |
| 287 | records:361 | standing | stays | - | record anchored on controls.sh:203 |
| 288 | records:364 | ctl_need cn standing | code | B2a | comment :364-371 of records 372/374, removed with them |
| 289 | records:365 | declined | code | B2a | comment :364-371 of records 372/374, removed with them |
| 290 | records:367 | gopt_ok fifo_ok | stmt | B1,B2a | `_gopt_ok` leaves the comment (B1, so B1's grep is 0); the comment goes with records 372/374 (B2a) |
| 291 | records:372 | ctl_need fifo_ok | code | B2a | comment, and record 372 (the fifotracked prefix) removed |
| 292 | records:373 | ctl_need gopt_ok | code | B1 | record 373 (k2's prefix) removed with its subject |
| 293 | records:374 | ctl_need | code | B2a | record 374 (the last ask) removed |
| 294 | records:376 | cn | code | B2a | record 377 re-aimed to `$_cap_lbl` (row X), comment :375-376 |
| 295 | records:377 | ctl_need | code | B2a | record 377 re-aimed to `$_cap_lbl` (row X), comment :375-376 |
| 296 | records:396 | wrongreason | unrel | - | "the wrong reason" of the R3 evidence record (not `_mut_trial`'s return) |
| 297 | records:408 | standing | stays | - | `_ev` record comment |
| 298 | wire:318 | callsite | unrel | - | "per call site" of the locale pin |
| 299 | wire:321 | grepopt | stays | - | the wire's `unset GREP_OPTIONS`: the subject, not edited |
| 300 | wire:322 | grepopt | stays | - | the wire's `unset GREP_OPTIONS`: the subject, not edited |
| 301 | wire:324 | grepopt | stays | - | the wire's `unset GREP_OPTIONS`: the subject, not edited |
| 302 | wire:408 | lbd | stays | - | `_lbd_lbl` guard in the U5 comment |
| 303 | wire:1235 | controls_v | stays | - | `_CONTROLS` set and sourced |
| 304 | wire:1236 | controls_v | stays | - | `_CONTROLS` set and sourced |
| 305 | wire:1237 | controls_v | stays | - | `_CONTROLS` set and sourced |
| 306 | wire:1242 | controls_v | stays | - | `_CONTROLS` set and sourced |
| 307 | umbrella:23 | family | dod | B1 | companion registration: the moved `…-cr-plan.md` and `…-cr-sweep.md` join row A-i-wire-fgit (G1, R3 H5) |
| 308 | umbrella:63 | wrongreason | unrel | - | other slices' prose |
| 309 | umbrella:75 | callsite | unrel | - | other slices' prose |
| 310 | umbrella:111 | family | dod | B1 | companion registration; "All eleven" -> thirteen: the moved `…-cr-plan.md` and `…-cr-sweep.md` (G1, R3 H5) |
| 311 | umbrella:132 | cap_table | unrel | - | `feedback_defer_cap_policy` matches `_cap_[a-z]` |
| 312 | 519:15 | ratchets | stays | - | banner: `_MUT_TARGETS` names the targets (derived after C2, same name) |
| 313 | 519:26 | ratchets | stays | - | banner: `_MUT_TARGETS` names the targets (derived after C2, same name) |
| 314 | 519:29 | lblconv | carried | C1 | banner :28-30 "the value of every `_*_lbl` name the parts spell": rule 1 counts expanded names in the wire and parts -> residual R13 |
| 315 | 519:34 | gopt_ok grepopt | carried | B1 | banner :33-36: k2 asked of the run's grep, declined via `_ctl_need` -> residual R10 |
| 316 | 519:35 | ctl_need notexh | carried | B1 | banner :33-36: k2 asked of the run's grep, declined via `_ctl_need` -> residual R10 |
| 317 | 519:427 | fifo_line perm_line | carried | B2b | :424-428, :446-449, :452: the machine-limit shape "run stays green" for FIFO/permission -> residual R12 |
| 318 | 519:446 | fifo_line | carried | B2b | :424-428, :446-449, :452: the machine-limit shape "run stays green" for FIFO/permission -> residual R12 |
| 319 | 519:452 | perm_line | carried | B2b | :424-428, :446-449, :452: the machine-limit shape "run stays green" for FIFO/permission -> residual R12 |
| 320 | 519:847 | ratchets | stays | - | slot 2 re-evaluation trigger |
| 321 | 519:1233 | grepopt | hist | - | §11.1 environment table and carve (provenance; GNU-grep rows corrected by the banner, which R10 corrects) |
| 322 | 519:1248 | grepopt | hist | - | §11.1 environment table and carve (provenance; GNU-grep rows corrected by the banner, which R10 corrects) |
| 323 | 519:1255 | slot | hist | - | §11.1 environment table and carve (provenance; GNU-grep rows corrected by the banner, which R10 corrects) |
| 324 | 519:1256 | slot | hist | - | §11.1 environment table and carve (provenance; GNU-grep rows corrected by the banner, which R10 corrects) |
| 325 | 519:1266 | slot | hist | - | §11.1 environment table and carve (provenance; GNU-grep rows corrected by the banner, which R10 corrects) |
| 326 | 519:1274 | perm_line | carried | B2b | D2 row: "Reported NOT EXERCISED where permissions are not enforced (the `$_perm_line` pattern)" -> R12 |
| 327 | 519:1343 | controls_v | hist | - | review dispositions / prototype table (provenance) |
| 328 | 519:1349 | controls_v | hist | - | review dispositions / prototype table (provenance) |
| 329 | 519:1362 | slot | hist | - | review dispositions / prototype table (provenance) |
| 330 | 519:1386 | wrongreason | hist | - | review dispositions / prototype table (provenance) |
| 331 | 519:1390 | grepopt | hist | - | review dispositions / prototype table (provenance) |
| 332 | 519:1391 | perm_line | carried | B2b | prototype row: "inside the `$_perm_line` gate, NOT EXERCISED otherwise" -> R12 |
| 333 | 519:1408 | ratchets | hist | - | provenance |
| 334 | 519:1413 | slot | hist | - | provenance |
| 335 | census:147 | machine_limits | stays | - | return channel |
| 336 | census:224 | perm_ok | stays | - | W4 seal hazard: `_perm_ok` is a probe of its own (describes the hazard, not the decline) |
| 337 | census:246 | x9 | stays | - | X9 owns the CI fact |
| 338 | corpus:183 | ratchets | hist | - | totals at `8413a4db` |
| 339 | corpus:186 | ratchets | hist | - | totals at `8413a4db` |
| 340 | corpus:194 | machine_limits | stmt | B2b,B2c | extent :194-200: P-f's limit excused automatically (B2b: declared) and "a 2 is never excused" (B2c: a capability excuse accepts 3) |
| 341 | corpus:199 | wrongreason | stmt | B2c | as :194 ("killed for the wrong reason (2) is never excused") |
| 342 | corpus:201 | mut_trial | stays | - | the slot rule (CR-S5) |
| 343 | corpus:203 | k2_line | stmt | B1 | k2's `_k2_line` example restated |
| 344 | corpus:221 | ratchets | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 345 | corpus:222 | ratchets | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 346 | corpus:241 | wrongreason | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 347 | corpus:258 | ratchets | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 348 | corpus:275 | perm_ok | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 349 | corpus:280 | ratchets | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 350 | corpus:290 | standing | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 351 | corpus:302 | standing | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 352 | corpus:306 | ratchets | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 353 | corpus:307 | cen_lbl lbd ratchets lblconv | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 354 | corpus:309 | registry | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 355 | corpus:310 | callsite | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 356 | corpus:311 | registry lblconv | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 357 | corpus:312 | ctl_need cn fifo_ok | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 358 | corpus:316 | standing | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 359 | corpus:317 | standing | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 360 | corpus:321 | ctl_need cn gopt_ok | hist | - | §6 history / §6.1 change rows (dated by commit) |
| 361 | corpus:334 | ratchets | cons | each | §6.1 Current totals: each slice's record count, floor and bare count |
| 362 | corpus:336 | ratchets | cons | each | §6.1 Current totals: each slice's record count, floor and bare count |
| 363 | impl-cr:38 | cn standing | hist | - | a dated record of past rounds; stays |
| 364 | impl-cr:82 | standing | hist | - | a dated record of past rounds; stays |
| 365 | impl-cr:105 | cn | hist | - | a dated record of past rounds; stays |
| 366 | impl-cr:109 | ratchets | hist | - | a dated record of past rounds; stays |
| 367 | impl-cr:112 | standing | hist | - | a dated record of past rounds; stays |
| 368 | impl-cr:114 | standing | hist | - | a dated record of past rounds; stays |
| 369 | impl-cr:132 | wrongreason | hist | - | a dated record of past rounds; stays |
| 370 | impl-cr:140 | standing | hist | - | a dated record of past rounds; stays |
| 371 | impl:78 | ratchets | hist | - | a dated record of past rounds; stays |
| 372 | impl:93 | ratchets | hist | - | a dated record of past rounds; stays |
| 373 | impl:143 | standing | hist | - | a dated record of past rounds; stays |
| 374 | impl:180 | standing | hist | - | a dated record of past rounds; stays |
| 375 | impl:194 | standing ratchets | hist | - | a dated record of past rounds; stays |
| 376 | impl:196 | registry | hist | - | a dated record of past rounds; stays |
| 377 | impl:202 | standing | hist | - | a dated record of past rounds; stays |
| 378 | impl:204 | ratchets | hist | - | a dated record of past rounds; stays |
| 379 | impl:291 | mut_trial | hist | - | a dated record of past rounds; stays |
| 380 | impl:293 | mut_trial | hist | - | a dated record of past rounds; stays |
| 381 | impl:311 | grepopt | hist | - | a dated record of past rounds; stays |
| 382 | impl:334 | ratchets | hist | - | a dated record of past rounds; stays |
| 383 | impl:346 | standing | hist | - | a dated record of past rounds; stays |
| 384 | impl:352 | registry | hist | - | a dated record of past rounds; stays |
| 385 | impl:353 | cen_lbl registry | hist | - | a dated record of past rounds; stays |
| 386 | impl:354 | lbd lblconv | hist | - | a dated record of past rounds; stays |
| 387 | impl:356 | callsite standing | hist | - | a dated record of past rounds; stays |
| 388 | impl:358 | registry lblconv | hist | - | a dated record of past rounds; stays |
| 389 | impl:359 | ctl_need | hist | - | a dated record of past rounds; stays |
| 390 | impl:360 | ctl_need cn standing | hist | - | a dated record of past rounds; stays |
| 391 | impl:361 | declined cndir | hist | - | a dated record of past rounds; stays |
| 392 | impl:363 | ratchets | hist | - | a dated record of past rounds; stays |
| 393 | impl:369 | ctl_need | hist | - | a dated record of past rounds; stays |
| 394 | impl:372 | cn | hist | - | a dated record of past rounds; stays |
| 395 | impl:373 | standing | hist | - | a dated record of past rounds; stays |
| 396 | impl:382 | ctl_need | hist | - | a dated record of past rounds; stays |
| 397 | impl:384 | registry | hist | - | a dated record of past rounds; stays |
| 398 | impl:385 | cn | hist | - | a dated record of past rounds; stays |
| 399 | impl:392 | cn | hist | - | a dated record of past rounds; stays |
| 400 | impl:405 | ctl_need | hist | - | a dated record of past rounds; stays |
| 401 | impl:406 | standing | hist | - | a dated record of past rounds; stays |
| 402 | impl:407 | cn | hist | - | a dated record of past rounds; stays |
| 403 | impl:413 | standing | hist | - | a dated record of past rounds; stays |
| 404 | impl:415 | cn | hist | - | a dated record of past rounds; stays |
| 405 | impl:440 | lbd | hist | - | a dated record of past rounds; stays |
| 406 | impl:445 | lbd | hist | - | a dated record of past rounds; stays |
| 407 | impl:454 | cen_lbl lbd | hist | - | a dated record of past rounds; stays |
| 408 | impl:486 | ctl_need | hist | - | a dated record of past rounds; stays |
| 409 | impl:497 | standing | hist | - | a dated record of past rounds; stays |
| 410 | impl:499 | ratchets | hist | - | a dated record of past rounds; stays |
| 411 | impl:503 | standing | hist | - | a dated record of past rounds; stays |
| 412 | impl:521 | ratchets | hist | - | a dated record of past rounds; stays |
| 413 | impl:533 | ratchets | hist | - | a dated record of past rounds; stays |
| 414 | impl:538 | gopt_ok fifo_ok perm_ok | hist | - | a dated record of past rounds; stays |
| 415 | impl:539 | grepopt | hist | - | a dated record of past rounds; stays |
| 416 | impl:540 | grepopt | hist | - | a dated record of past rounds; stays |
| 417 | impl:541 | ctl_need cn gopt_ok | hist | - | a dated record of past rounds; stays |
| 418 | impl:542 | ratchets | hist | - | a dated record of past rounds; stays |
| 419 | impl:545 | grepopt | hist | - | a dated record of past rounds; stays |
| 420 | impl:550 | gopt_ok | hist | - | a dated record of past rounds; stays |
| 421 | impl:583 | ratchets | hist | - | a dated record of past rounds; stays |
| 422 | impl:585 | gopt_ok | hist | - | a dated record of past rounds; stays |
| 423 | impl:589 | cn | hist | - | a dated record of past rounds; stays |
| 424 | impl:609 | ratchets | hist | - | a dated record of past rounds; stays |
| 425 | impl:639 | registry | hist | - | a dated record of past rounds; stays |
| 426 | impl:642 | registry | hist | - | a dated record of past rounds; stays |
| 427 | impl:649 | ctl_need | hist | - | a dated record of past rounds; stays |
| 428 | impl:650 | ctl_need fifo_ok | hist | - | a dated record of past rounds; stays |
| 429 | impl:652 | cn | hist | - | a dated record of past rounds; stays |
| 430 | impl:655 | cn | hist | - | a dated record of past rounds; stays |
| 431 | impl:657 | ctl_need | hist | - | a dated record of past rounds; stays |
| 432 | impl:658 | registry | hist | - | a dated record of past rounds; stays |
| 433 | impl:659 | standing | hist | - | a dated record of past rounds; stays |
| 434 | impl:707 | cn | hist | - | a dated record of past rounds; stays |
| 435 | impl:708 | ctl_need gopt_ok | hist | - | a dated record of past rounds; stays |
| 436 | impl:719 | registry | hist | - | a dated record of past rounds; stays |
| 437 | impl:720 | standing | hist | - | a dated record of past rounds; stays |
| 438 | impl:723 | controls_v | hist | - | a dated record of past rounds; stays |
| 439 | impl:727 | controls_v | hist | - | a dated record of past rounds; stays |
| 440 | impl:780 | mut_trial | hist | - | a dated record of past rounds; stays |
| 441 | impl:782 | gopt_ok k2_line | hist | - | a dated record of past rounds; stays |
| 442 | impl:783 | mut_trial | hist | - | a dated record of past rounds; stays |
| 443 | impl:787 | lblconv | hist | - | a dated record of past rounds; stays |
| 444 | impl:789 | cn k2_line | hist | - | a dated record of past rounds; stays |
| 445 | impl:792 | mut_trial | hist | - | a dated record of past rounds; stays |
| 446 | impl:793 | wrongreason | hist | - | a dated record of past rounds; stays |
| 447 | impl:795 | wrongreason | hist | - | a dated record of past rounds; stays |
| 448 | impl:799 | family | hist | - | a dated record of past rounds; stays |
| 449 | landing:59 | x9 | stays | - | budget verdict after X9 |
| 450 | landing:74 | x9 | stays | - | X9 route (b) |
| 451 | landing:121 | x9 | dod | B2b | extent :121-127: route (b) also answers Q1 (FIFO / non-root on the runner); a red there is fixed by a declaration in the job's `env:` (G2) |
| 452 | landing:122 | x9 | stays | - | route (b) and land-order step 7 |
| 453 | landing:135 | x9 | stays | - | route (b) and land-order step 7 |
| 454 | landing:149 | x9 | stays | - | route (b) and land-order step 7 |
| 455 | landing:150 | x9 | stays | - | route (b) and land-order step 7 |
| 456 | landing:183 | slot | dod | B2b | extent :183-186: the launch-environment slot amend gains BS-3's stale-`WEBREF_WIRE_LACKS` member (G12 A5) |
| 457 | landing:238 | callsite | unrel | - | liveness slot text: "call sites unbounded" |
| 458 | landing:247 | standing | stays | - | R9 slot text "the three standing checks" |
| 459 | landing:275 | lbd | hist | - | measurement history |
| 460 | landing:299 | ratchets | stays | - | each slice sets `_MUT_RECORDS_MIN` (the rule) |
| 461 | landing:329 | x9 | stays | - | X9 route |
| 462 | landing:348 | x9 | stays | - | X9 route |
| 463 | landing:360 | ratchets | stays | - | C11 row |
| 464 | landing:380 | wrongreason | stays | - | X3 / X8 rows |
| 465 | landing:385 | x9 | stays | - | X3 / X8 rows |
| 466 | landing:386 | x9 | dod | B2b | X9 row: the run is also the evidence for P1 on the runner (Q1, G2) |
| 467 | pr527-r15:37 | x9 | hist | - | a dated record of past rounds; stays |
| 468 | pr527-r15:67 | machine_limits | hist | - | a dated record of past rounds; stays |
| 469 | pr527-r15:68 | machine_limits | hist | - | a dated record of past rounds; stays |
| 470 | pr527-r15:69 | machine_limits | hist | - | a dated record of past rounds; stays |
| 471 | pr527-r15:73 | machine_limits | hist | - | a dated record of past rounds; stays |
| 472 | pr527-r15:116 | ratchets | hist | - | a dated record of past rounds; stays |
| 473 | pr527-r15:117 | ratchets | hist | - | a dated record of past rounds; stays |
| 474 | pr527-r15:132 | x9 | hist | - | a dated record of past rounds; stays |
| 475 | pr527-r15:136 | x9 | hist | - | a dated record of past rounds; stays |
| 476 | pr527-r15:140 | x9 | hist | - | a dated record of past rounds; stays |
| 477 | pr527-r15:175 | ratchets | hist | - | a dated record of past rounds; stays |
| 478 | pr527-r15:219 | ratchets | hist | - | a dated record of past rounds; stays |
| 479 | pr527-r15:230 | x9 | hist | - | a dated record of past rounds; stays |
| 480 | pr527-r15:325 | ratchets | hist | - | a dated record of past rounds; stays |
| 481 | pr527-r15:346 | wrongreason | hist | - | a dated record of past rounds; stays |
| 482 | pr527-r15:375 | mut_trial wrongreason | hist | - | a dated record of past rounds; stays |
| 483 | pr527-r15:378 | x9 | hist | - | a dated record of past rounds; stays |
| 484 | pr527-r15:441 | ratchets | hist | - | a dated record of past rounds; stays |
| 485 | pr527-r15:443 | ratchets | hist | - | a dated record of past rounds; stays |
| 486 | pr527-r15:470 | ratchets | hist | - | a dated record of past rounds; stays |
| 487 | pr527-r15:480 | ratchets | hist | - | a dated record of past rounds; stays |
| 488 | pr527-r15:486 | ratchets | hist | - | a dated record of past rounds; stays |
| 489 | pr527-r15:493 | ratchets | hist | - | a dated record of past rounds; stays |
| 490 | pr527-r15:494 | ratchets | hist | - | a dated record of past rounds; stays |
| 491 | pr527-r15:547 | ratchets | hist | - | a dated record of past rounds; stays |
| 492 | pr527-r15:548 | ratchets | hist | - | a dated record of past rounds; stays |
| 493 | pr527-r15:549 | ratchets | hist | - | a dated record of past rounds; stays |
| 494 | pr527-r15:550 | ratchets | hist | - | a dated record of past rounds; stays |
| 495 | pr527-r15:566 | callsite | hist | - | a dated record of past rounds; stays |
| 496 | pr527-r15:574 | callsite | hist | - | a dated record of past rounds; stays |
| 497 | pr527-r15:654 | perm_ok | hist | - | a dated record of past rounds; stays |
| 498 | pr527-r15:658 | ratchets | hist | - | a dated record of past rounds; stays |
| 499 | pr527-r15:664 | mut_trial | hist | - | a dated record of past rounds; stays |
| 500 | pr527-r15:731 | lbd lblconv | hist | - | a dated record of past rounds; stays |
| 501 | pr527-r15:735 | ratchets | hist | - | a dated record of past rounds; stays |
| 502 | pr527:113 | ratchets | hist | - | a dated record of past rounds; stays |
| 503 | pr527:181 | ratchets | hist | - | a dated record of past rounds; stays |
| 504 | pr527:189 | callsite | hist | - | a dated record of past rounds; stays |
| 505 | pr527:194 | mut_trial x9 | hist | - | a dated record of past rounds; stays |
| 506 | pr527:393 | notexh | hist | - | a dated record of past rounds; stays |
| 507 | pr527:406 | ratchets | hist | - | a dated record of past rounds; stays |
| 508 | pr527:407 | ratchets | hist | - | a dated record of past rounds; stays |
| 509 | pr527:418 | callsite | hist | - | a dated record of past rounds; stays |
| 510 | pr527:485 | ratchets | hist | - | a dated record of past rounds; stays |
| 511 | pr527:570 | callsite | hist | - | a dated record of past rounds; stays |
| 512 | pr527:572 | x9 | hist | - | a dated record of past rounds; stays |
| 513 | pr527:580 | ratchets | hist | - | a dated record of past rounds; stays |
| 514 | pr527:581 | ratchets | hist | - | a dated record of past rounds; stays |
| 515 | pr527:656 | x9 | hist | - | a dated record of past rounds; stays |
| 516 | pr527:666 | ratchets | hist | - | a dated record of past rounds; stays |
| 517 | pr527:667 | ratchets | hist | - | a dated record of past rounds; stays |
| 518 | pr527:685 | x9 | hist | - | a dated record of past rounds; stays |
| 519 | pr527:738 | notexh | hist | - | a dated record of past rounds; stays |
| 520 | pr527:892 | ratchets | hist | - | a dated record of past rounds; stays |
| 521 | pr527:893 | ratchets | hist | - | a dated record of past rounds; stays |
| 522 | provenance:184 | callsite | hist | - | a dated record of past rounds; stays |
| 523 | provenance:251 | lblconv | hist | - | a dated record of past rounds; stays |
| 524 | provenance:657 | notexh | hist | - | a dated record of past rounds; stays |
| 525 | provenance:659 | notexh | hist | - | a dated record of past rounds; stays |
| 526 | provenance:662 | notexh | hist | - | a dated record of past rounds; stays |
| 527 | provenance:866 | fifo_ok | hist | - | a dated record of past rounds; stays |
| 528 | provenance:937 | fifo_ok | hist | - | a dated record of past rounds; stays |
| 529 | residuals:108 | slot | stays | - | R1 |
| 530 | residuals:111 | slot | dod | B2b | R4 (launch-environment class) names BS-3's member: a caller's stale `WEBREF_WIRE_LACKS` |
| 531 | residuals:116 | standing | stays | - | R9 "three standing checks" |
| 532 | reviews:113 | ratchets | hist | - | a dated record of past rounds; stays |
| 533 | reviews:185 | ratchets | hist | - | a dated record of past rounds; stays |
| 534 | reviews:249 | x9 | hist | - | a dated record of past rounds; stays |
| 535 | reviews:319 | callsite | hist | - | a dated record of past rounds; stays |
| 536 | reviews:384 | x9 | hist | - | a dated record of past rounds; stays |
| 537 | reviews:398 | x9 | hist | - | a dated record of past rounds; stays |
| 538 | reviews:491 | lblconv | hist | - | a dated record of past rounds; stays |
| 539 | reviews:523 | ratchets | hist | - | a dated record of past rounds; stays |
| 540 | reviews:536 | ratchets | hist | - | a dated record of past rounds; stays |
| 541 | reviews:537 | ratchets | hist | - | a dated record of past rounds; stays |
| 542 | reviews:539 | lbd | hist | - | a dated record of past rounds; stays |
| 543 | reviews:540 | lbd | hist | - | a dated record of past rounds; stays |
| 544 | reviews:544 | ctl_need gopt_ok notexh | hist | - | a dated record of past rounds; stays |
| 545 | reviews:598 | x9 | hist | - | a dated record of past rounds; stays |
| 546 | reviews:607 | ratchets | hist | - | a dated record of past rounds; stays |
| 547 | reviews:612 | ratchets | hist | - | a dated record of past rounds; stays |
| 548 | reviews:638 | ratchets | hist | - | a dated record of past rounds; stays |
| 549 | design:16 | family | dod | B1 | header :5-22 companion list: the moved `…-cr-plan.md` and `…-cr-sweep.md` are registered (G1, R3 H5) |
| 550 | design:17 | family | dod | B1 | header :5-22 companion list: the moved `…-cr-plan.md` and `…-cr-sweep.md` are registered (G1, R3 H5) |
| 551 | design:152 | machine_limits | stays | - | return channel |
| 552 | design:196 | slot | stays | - | R1 / slot |
| 553 | design:373 | fifo_ok | stays | - | prelude forwards `_fifo_ok` |
| 554 | design:409 | machine_limits | stays | - | markers |
| 555 | design:451 | standing | stays | - | watchdog; X9 |
| 556 | design:458 | x9 | stays | - | watchdog; X9 |
| 557 | design:476 | ctl_need standing | stmt | B2a | "the `_ctl_need` standing check's `untrusted` row" -> `_cap`'s (G1: design memo :476) |
| 558 | design:494 | mut_trial | stays | - | "rc ≠ 0 plus a needle"; the gate's callers |
| 559 | design:503 | standing | stays | - | "the three standing checks … run it over `$_VFY` … each reads `_fw_trusted` itself before its rows": true of `_wm`, `_ev` and `_cap` (`_cap`'s trust-open shape, the plan memo's §7 R3-B2a-5) |
| 560 | design:510 | cn standing cen_lbl | stmt | B2a | "reds the run under `$_cn_lbl`" re-measured |
| 561 | design:531 | x9 | stays | - | X9 confirms GNU bash |
| 562 | design:589 | lbd | stmt | C1 | "a static census of the label names the parts spell": the wire too |
| 563 | design:601 | standing | stays | - | W2 row: "the three standing checks" |
| 564 | design:605 | notexh | stmt | B2b | P-b row: its limit is a `fact` row (no declaration) |
| 565 | design:609 | machine_limits notexh | stmt | B2b | P-f row: the limit is declared (R1 F10) |
| 566 | design:615 | callsite lbd registry ratchets lblconv | stmt | C1 | row C: rule 1 (by expansion), rule 2 (bijection), rule 3, membership (B2a adds the membership clause first) |
| 567 | design:616 | ctl_need declined cn fifo_ok standing cen_lbl registry | stmt | B2a | row CN -> rows CAP, CAPT and one per group/limit label |
| 568 | design:620 | lblconv | stmt | C1 | row L: the wire's text too |
| 569 | design:622 | standing | stays | - | rows WM, EV |
| 570 | design:623 | standing | stays | - | rows WM, EV |
| 571 | design:672 | x9 | stays | - | split note |
| 572 | design:690 | ratchets lblconv | stays | - | labels live in the controls file (the new ones too) |
| 573 | design:691 | lblconv | stmt | C1 | "every `_*_lbl` label is named by a record": every expanded one |
| 574 | design:693 | x9 | stays | - | X9 budget |
| 575 | design:705 | x9 | stays | - | X9 budget |
| 576 | design:733 | family | stays | - | §13 open-record routing |
| 577 | design:740 | family | stays | - | §13 open-record routing |
| 578 | trip-wires.sh:10 | callsite | unrel | - | driver prose ("call sites", "WRONG reason") |
| 579 | trip-wires.sh:13 | callsite | unrel | - | driver prose ("call sites", "WRONG reason") |
| 580 | trip-wires.sh:155 | wrongreason | unrel | - | driver prose ("call sites", "WRONG reason") |

---

## §F D9 — the triage appendix, every item assigned

*Moved from the plan memo `…-cr-plan.md` §8 (its 1000-line rule); the body below is verbatim. The section numbers it cites (§3.2, §7) are the plan memo's, not this file's.*


The appendix has 26 bullets; three hold items that go to different places and are split (D2 → (a)/(b), the
AL1/AL4/AL5 bullet → three, EF2/EF3/EF-3 → two): **30 items**, each assigned once. IDs from the appendix's finders are
those of the handoff file `codereview-max-14d0b35d/candidates.md` (not the older `candidates.md` at the handoff root,
whose AL1–AL5 and CV2 are other items; R1 F26). Row 31 is R1's F9, not an appendix item.

| # | item | assigned | state at `2f1ca601` / note |
|---|---|---|---|
| 1 | D2(a) harness.sh "every fixture … `\|\| _fixture_failed`" | CR-S2 | **done** `945a814d` (harness.sh:774-777) |
| 2 | D2(b) the `_gopt_ok` comment's "reds the run either way" | B (CR-B1) | reworded by CR-S2 `945a814d` (harness.sh:840-843); removed with the probe |
| 3 | D7 census.md:202 "five parts" at `8413a4db` | CR-S8a | **done** `161ae982`: per-commit part sets |
| 4 | D9/K-1 umbrella + memo header omit census.md, impl.md | CR-S8a | **done** `161ae982` |
| 5 | D10/K-2 `d976f239` 952 lines, census.md:7-8 provenance | CR-S8a | **done** `161ae982` |
| 6 | D11/K-3 routes into the closed r15 §Q | CR-S8a | **done** `161ae982`, rerouted to "the open implementation record (§13)", not to a named file |
| 7 | D12/K-4 memo "950 to 729" | CR-S8a | **done** `161ae982` |
| 8 | D13/A3-7 controls.sh:18-19 `$_CONTROLS` | C (CR-C1) | §3.2 item 6 |
| 9 | SW-2 "preflight" wording + #519 memo :1406 | CR-S8a | **done** `161ae982`: the in-PR docs and three code comments (ctlrun.sh:56, controls.sh:681-683, records.sh:152-153); the #519 memo not touched (D5) |
| 10 | SI6 ctlrun.sh:87-88, mutations.sh:491 list `_ctl_gate` | C (CR-C1) | |
| 11 | A3-8/SI7 fixtures.sh prefix-dir comment | CR-S2 | **done** `945a814d` |
| 12 | CV2 the #519 memo > 1000 lines, touched without a split | **OUT-OF-PR** (no slot: a standing rule carries it) | the 1414-line memo was touched by `0d411df2` and `173b81dc` with no touch-time split. CLAUDE.md's "1000-line debt = touch-time split (defer しない)" carries it: the PR that next touches the memo splits it first, as a standalone prereq PR; D5 forbids a touch this round. That PR also discharges residuals R10, R12 and R13 (§7, the carried corrections): applied in the split's follow-up commit, then removed (R3 H2). Recorded 2026-10-18 (R1 F20, G12 A5) |
| 13 | H-3 positional asks | B (CR-B2a) | |
| 14 | AL1 `.declined` as a second log | B (CR-B2a) | |
| 15 | AL4 the umask evidence arm has no single-edit subject | **DECLARE** | records.sh:391-393: "R5 - the umask withheld at the delivery AND the umask control's verdict flipped …: without the evidence check the control would pass, so only the umask clause of the check sees it" (DECISIONS-R7 S2 OPEN-2) |
| 16 | AL5 the marker duplicates the exit status | CR-S1 | **REFUTED**: D7 reversed by CR-S1's race measurement; the marker stays (ctlrun.sh:294-312) |
| 17 | AL2/H-2 the site rule by line | **DECLARE** | mutations.sh:504-505: "The line is the finest site bash reports: two calls under one label on ONE line are one site" |
| 18 | AL3/H-6 the `_ctl_need` prefix | B (CR-B2a) | |
| 19 | AL4/H-7 the verdict's `$1` | C (CR-C1) | kept, with the reason (§3.2 item 5) |
| 20 | CL2/SI4 P-b `_pbbad` | CR-S3 | **done** `598e1aa2` (0 hits) |
| 21 | CL3/SI8 `_cn` loop bookkeeping | B (CR-B2a) | `_cap` is written without derivable state |
| 22 | CL4/SI5/RU3 summary polarity, indent | B (CR-B2a) | `_cap_lines` |
| 23 | CL5/RU6 grep resolved twice | B (CR-B1) | harness.sh:850 and :866; :850 goes |
| 24 | CL6/S-7 dead empty-`_lbd_files` guard | C (CR-C2) | |
| 25 | CL7/SI7 `_MUT_RECORDS_MIN` hash changelog (and :716) | C (CR-C2) | the changelog :238-247 in CR-C2; :716's hash goes with `_mskip`'s rewrite in CR-B2c, which adds none |
| 26 | CL9/RU2 registry label parsed three times | C (CR-C1) | |
| 27 | CL10/R-5 part-path spelling | C (CR-C2) | |
| 28 | EF2/EF3 recorder + evidence-dir `rm`/`mkdir` on every run | CR-S2 | **done** `945a814d` (ctlrun.sh:241-249: only for calls that deliver) |
| 29 | EF-3 the marker `rm` on every run | CR-S1 | **no change**: the marker stays, so its per-run `rm` (ctlrun.sh:287) stays |
| 30 | RU5 a failed count resets `_cn_seen` | B (CR-B2a) | `_cn_seen` removed |
| 31 | (R1 F9) the #519 memo banner :33-36 states OPEN-3's mechanism as current | B (CR-B1) | carried correction, residual R10 (§7; renumbered in landing order, G12 A5) |

**Counts (rows 1–30):** CR-S1 2 (settled: REFUTED, no change) · CR-S2 3 (done) · CR-S3 1 (done) · CR-S8a 6 (done) ·
B 8 · C 7 · DECLARE 2 · OUT-OF-PR 1 = **30**; row 31 is B's.

---

## §E Appendix — the instruments (as run)

The four instruments a slice's completion runs (the plan memo's §7, "The move"), verbatim, one fenced block per file under its
file name. A slice copies each block to a scratch directory under that name and runs it from there (no extractor: the copy is
the block): `sweep.sh <clone>` (reads `idents.tsv` beside itself), `ledger-check.py <clone> <slice> [<commit>]`,
`completion.sh <clone>`. Two changes were made in the move, against the handoff copies in `crplan-r3/bin/` that measured the
frame: `ledger-check.py` reads §3 of this file as committed at CR-B1 and each row's frame text from `git show 2f1ca601:<file>`
(not the handoff logs `ledger.tsv` / `sweep-sites.tsv`), and `completion.sh`'s C2 part-path grep gains the quoted-stem
alternative (plan memo §3.4, H6). `sweep.sh` is r4's (it excludes the two moved files). Run at the frame, `ledger-check.py`
takes the CR-B1 commit as its third argument (the frame has no sweep file). The sections above name handoff-directory paths
(`crplan-r3/bin/…`, `logs/…`): those are where the frame was measured, and stay outside the repo.

### §E.sweep.sh

`sweep.sh`

```bash
#!/bin/bash
# sweep.sh <clone> : the G1 sweep. For each identifier row of idents.tsv (key, ERE, flags),
# /usr/bin/grep -rnE over the G1 file set of the clone; prints key<TAB>file:line<TAB>text.
set -u
c="$1"; here="$(cd "$(dirname "$0")" && pwd)"
cd "$c" || exit 2
files=( .claude/tools/webref-generic-core-trip-wire.sh .claude/tools/webref-generic-core-trip-wire.{controls,ctlrun,fixtures,harness,mutations,mutgen,records}.sh
        docs/plans/*k2-fgit* docs/plans/2026-07-citation-hygiene-umbrella.md docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md
        CLAUDE.md .github/workflows/ci.yml scripts/* )
# Excluded (plan-review R3 H1): the plan memo and this ledger, once CR-B1 moves them into docs/plans/, match the
# glob above; they are the ledger's own statement of the identifiers, not sites. Absent at the frame: a no-op there.
keep=(); for f in "${files[@]}"; do case "$f" in docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-plan.md|docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-sweep.md) ;; *) keep+=( "$f" ) ;; esac; done; files=( "${keep[@]}" )
echo "### head=$(git rev-parse --short HEAD) files=${#files[@]}"
printf '### file: %s\n' "${files[@]}"
while IFS=$'\t' read -r key re fl; do
  opt=-nE; [ "$fl" = i ] && opt=-niE
  /usr/bin/grep -H $opt -- "$re" "${files[@]}" | while IFS= read -r h; do
    f="${h%%:*}"; r="${h#*:}"; l="${r%%:*}"; t="${r#*:}"
    printf '%s\t%s:%s\t%s\n' "$key" "$f" "$l" "$t"
  done
done < "$here/idents.tsv"
```

### §E.idents.tsv

`idents.tsv`

```text
ctl_need	_ctl_need	
declined	\.declined	
cn	(^|[^A-Za-z0-9_])_cn([^A-Za-z0-9]|$)	
cndir	\$_VFY/\.cn([^A-Za-z0-9]|$)	
gopt_ok	_gopt_ok	
k2_line	_k2_line	
goptprobe	goptprobe	
fifo_ok	_fifo_ok	
perm_ok	_perm_ok	
fifo_line	_fifo_line	
perm_line	_perm_line	
machine_limits	machine_limits	
fw_limits	_fw_limits	
mskip	_mskip	
mut_trial	_mut_trial|_mrc2	
notexh	NOT EXERCISED on this machine	
cap_table	_CAP_TABLE|_cap_[a-z]|_capt?_lbl|_fifo_lbl|_perm_lbl|_pfl_lbl|WEBREF_WIRE_LACKS|goptgrep	
callsite	call site|outermost	i
standing	standing check	i
cen_lbl	_cen_lbl	
lbd	(^|[^A-Za-z0-9_])_lbd([^A-Za-z0-9]|$)|\.vlabels	
controls_v	_CONTROLS	
registry	\.registry|_ctl_reg|\.ctllabels|_mut_census_end	
ratchets	_MUT_RECORDS_MIN|_MUT_UNRECORDED_MAX|_MUT_TARGETS	
wrongreason	wrong reason	i
grepopt	GREP_OPTIONS	
family	impl-cr	
lblconv	_\*_lbl|_lbl="…"	
slot	#11-trip-wire-launch-environment	
x9	X9|route \(b\)	
```

### §E.ledger-check.py

`ledger-check.py`

```python
#!/usr/bin/env python3
"""ledger-check.py <clone> <slice> [<commit>] : a slice's sweep completion, derived from the G1 ledger.
The ledger is §3 of docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr-sweep.md AS COMMITTED at CR-B1 (<commit>;
default: the commit that first added the file on the clone's HEAD), not the working file, so a later slice
editing §3 cannot make its own check pass; each row's frame text is line N of `git show 2f1ca601:<file>`.
For every ledger row: if the row is owned by <slice> or a slice before it (order B1 B2a B2b B2c C1 C2), its frame
text must be GONE from its file at the clone (changed or removed); otherwise (a later slice's row, or
stays/hist/unrel/carried) its frame text must still be there. A `carried` row (the #519 memo, D5) always stays:
the slice adds its residual row. Counted per (file, text), so identical lines are handled. Rows owned by `each`
(figures every slice sets) are skipped: the ratchets check them. Prints the violations; exit 1 if any. New
identifier hits are not judged here: re-run sweep.sh at the clone and classify every site not in the ledger in the
slice's record entry. Run at the frame, pass <commit> explicitly (the frame has no sweep file)."""
import sys, os, subprocess, collections
ORDER = ['B1', 'B2a', 'B2b', 'B2c', 'C1', 'C2']
FRAME = '2f1ca601'
D = 'docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic'
SWEEP = D + '-cr-sweep.md'
W = '.claude/tools/webref-generic-core-trip-wire'
clone, sl = sys.argv[1], sys.argv[2]
def git(*a):
    return subprocess.run(['git', '-C', clone, *a], capture_output=True, text=True, errors='replace').stdout
def path(s):
    if s in ('controls', 'ctlrun', 'fixtures', 'harness', 'mutations', 'mutgen', 'records'): return f'{W}.{s}.sh'
    if s == 'wire': return W + '.sh'
    if s == 'design': return D + '.md'
    if s == '519': return 'docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md'
    if s == 'umbrella': return 'docs/plans/2026-07-citation-hygiene-umbrella.md'
    if s == 'trip-wires.sh': return 'scripts/trip-wires.sh'
    return f'{D}-{s}.md'
commit = sys.argv[3] if len(sys.argv) > 3 else (git('log', '--diff-filter=A', '--format=%H', '--', SWEEP).split() or [''])[-1]
if not commit: sys.exit('no commit adds ' + SWEEP + ' on this clone: pass the CR-B1 commit as the third argument')
md = git('show', f'{commit}:{SWEEP}').split('\n')
i = md.index('| # | site | identifiers | class | slice | disposition |')
rows = []
for l in md[i + 2:]:
    if not l.startswith('| '): break
    n, site, keys, c, s, note = l[2:].split(' | ', 5)
    rows.append((n, site, c, s))
done = set(ORDER[:ORDER.index(sl) + 1])
frame = {}
want = collections.Counter(); gone = collections.Counter(); bad = 0
for n, site, c, s in rows:
    owners = set(s.split(',')) - {'-'}
    if 'each' in owners: continue
    f = path(site.rsplit(':', 1)[0]); ln = int(site.rsplit(':', 1)[1])
    if f not in frame: frame[f] = git('show', f'{FRAME}:{f}').split('\n')
    t = frame[f][ln - 1]
    if owners & done and c != 'carried': gone[(f, t)] += 1  # a carried row stays (D5): its residual row is the edit
    else: want[(f, t)] += 1
for (f, t) in set(want) | set(gone):
    p = os.path.join(clone, f)
    have = sum(1 for x in open(p, errors='replace') if x.rstrip('\n') == t) if os.path.exists(p) else 0
    if have != want[(f, t)]:
        bad = 1
        print(f'!! {f}: frame text present {have}x, expected {want[(f, t)]}x (owned by {sl} or earlier: {gone[(f, t)]}): {t[:110]}')
print(f'checked {sum(want.values()) + sum(gone.values())} rows for slice {sl}: {"FAIL" if bad else "ok"}')
sys.exit(bad)
```

### §E.completion.sh

`completion.sh`

```bash
#!/bin/bash
# completion.sh <clone> : the per-slice 0-hit greps derived from the G1 ledger, run over a clone; prints each
# grep's hits (file:line). At a slice's head its own lines must print 0 hits. At 2f1ca601 they print the
# ledger rows the slice owns for those identifiers.
set -u
c="$1"; cd "$c" || exit 2
P=( .claude/tools/webref-generic-core-trip-wire.sh .claude/tools/webref-generic-core-trip-wire.{controls,ctlrun,fixtures,harness,mutations,mutgen,records}.sh )
D=( docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-{census,corpus,landing,residuals}.md )
DnoC=( docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-{census,landing,residuals}.md )
run() { local name="$1"; shift; local out; out="$(/usr/bin/grep -nE "$@" 2>/dev/null)"; printf '== %s: %s hit(s)\n' "$name" "$(printf '%s' "$out" | /usr/bin/grep -c .)"; [ -z "$out" ] || printf '%s\n' "$out" | cut -c1-140 | sed 's/^/   /'; }
echo "### head=$(git rev-parse --short HEAD)"
run "B1 parts+wire" -e '_gopt_ok|_k2_line|goptprobe' -- "${P[@]}"
run "B1 docs (design memo, census, landing, residuals)" -e '_gopt_ok|_k2_line|goptprobe' -- "${DnoC[@]}"
run "B1 corpus (after B1: only :321, a §6.1 history row)" -e '_gopt_ok|_k2_line|goptprobe' -- docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-corpus.md
run "B1 harness 'three capability probes'" -e 'three capability probes' -- .claude/tools/webref-generic-core-trip-wire.harness.sh
run "B2a parts+wire" -e '_ctl_need|\.declined|(^|[^A-Za-z0-9_])_cn([^A-Za-z0-9]|$)|/\.cn([^A-Za-z0-9]|$)|_fifo_line|_perm_line' -- "${P[@]}"
run "B2a docs (corpus excluded: its §6.1 rows are history)" -e '_ctl_need|\.declined|(^|[^A-Za-z0-9_])_cn([^A-Za-z0-9]|$)|_fifo_line|_perm_line' -- "${DnoC[@]}"
run "B2b phrases" -e 'reported below and not|printed on a green line would|as for the FIFO and file-permission controls|the treatment P-b and the FIFO and file-permission controls' -- "${P[@]}" "${D[@]}"
run "B2c hash in _mskip's comment" -e "dropped-at-cap round, commit \`[0-9a-f]{8}\`" -- .claude/tools/webref-generic-core-trip-wire.mutations.sh
run "C2 _MUT_TARGETS literal" -e '^_MUT_TARGETS="' -- .claude/tools/webref-generic-core-trip-wire.mutations.sh
run "C2 part path spelled (after C2: wire:1235, the helper's definition, comments)" -e '\$\{SELF%\.sh\}\.|"\$\{SELF%\.sh\}"\.' -- "${P[@]}"
```
