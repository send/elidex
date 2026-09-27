# K2 fixture git — plan-review record

This file holds every plan-review round's dispositions and the terminators for
`docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo"). It was split out of the
provenance companion (`…-k2-fgit-hermetic-provenance.md`) unchanged, as a touch-time split before
round 9's record was added. Section numbers (§D.0–§D.8) are kept, so earlier references to "companion
§D.n" now resolve here. Section references follow the companion's convention: to the design memo as of
the draft the row belongs to, unless a file is named. `§A.n` refers to the companion.

---

## §0 Spec coverage map

**No spec surface** — this file records review dispositions only. It has the same by-design shape as
the design memo's §2.5 and the companion's §0: `preflight.py` exits 1 because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §D Plan-review dispositions

### §D.0 Terminators and the close-out plan

**Plan-review CLOSED after round 9** (orchestrating session, 2026-09-27).

The ground:
- Round 9's two IMPs (Ax2 0/1/1, Ax3 0/1/3) were **implementation bugs in the prototype, not design
  defects**. Draft 10 fixes both and verifies each with its cells (companion §A.13).
- They are clerical-level under the skill's Step 5, so the re-review may be skipped.
- Every finding since round 7 has been code-level review of the prototype. The design has not moved
  since draft 6 (the window), draft 8 (P-g) and draft 9 (the threat model).

⚠ **The formal terminator was not met.** "No in-scope IMP with a failing cell" was last unmet in round
9, by those two implementation bugs. Draft 10 fixes them and shows their cells passing. No round
confirms that. The close rests on the orchestrating session's decision and the ground above, not on
the terminator.

**Round 9 (as it ran; not met): a Step-4.5 focused check of E2 and E3 only.**
- **Ax2** takes E3 (`_control`'s W2 check, the exit causes) together with AR (W3) and SE1 (the exit-5
  re-check).
- **Ax3** takes E2 (P-g's census: its population, its unknown shapes, the declared unsearchable list).
- **Reviewers are told that class (c) (design memo §0.3) is out of scope.** A class-(c) finding is
  not an IMP.
- **Converges** if and only if there is no in-scope IMP with a failing cell.
- Result: Ax2 0/1/1, Ax3 0/1/3; see §D.9.

**Round 8 (as it ran; not met).** Round 8 was the Step-4.5 focused check of draft 8 (`453b7b0f`) on D2
and D4. It returned E1–E4 with cells (§D.8). The window behaviour held again; the IMPs were in the
threat-model boundary, P-g's population and the placement of the incomplete-window exit.

**Round 7 (as it ran; not met).** Round 7 was the Step-4.5 focused check of draft 7:
- Ax2 (item 1): 0 CRIT / 2 IMP / 4 MIN;
- Ax3 (item 2): 0 CRIT / 3 IMP / 1 MIN.

The behaviour was sound, and the IMPs were in the secondary instruments.

⚠ **Draft 8 removes an instrument that round 7 was asked to check.** Seed S was draft 7's answer to
item 2, and the round-7 check measured it to be a vocabulary deny-list: it let literal spellings through
silently and red-flagged harmless ones. So D1 deletes it rather than extending it. This is recorded as
a removal, not as a fix of S.

**Round 6 (as it ran).** Round 6 was a **full five-axis review** of draft 6 `8b6a4005`, not the
focused one that draft 6 had announced:
- Ax1: 0.
- Ax2: 0/1/3.
- Ax3: 0/1/1.
- Ax4: 0/1/8.
- Ax5: 0/2/6.

In total, 0 CRIT / 5 IMP / 18 MIN. Its terminator was "no IMP with a failing command", and it was not
met, although **no finding moved the window mechanism**. Draft 6 had recorded it as "focused,
Ax2/3/5". That came from the orchestrating session's instruction before it decided on a full round,
and it is corrected here.

**Round 5 (not met).** Ax2 raised 1 IMP and Ax3 raised 3, all with cells. The root cause was that the
detector was a name list, and the design changed as a result (§D.5).

**Round 4 (not met: 5 unique IMP, each with a cell).** Round 4 converges if no IMP lands on draft-4-added text, and no IMP moves a mechanism
**without a corpus cell that shows the defect**. A finding that comes with a failing cell is fixed by
driving the prototype to pass it, and then the memo is re-derived from the result. It is not fixed in
prose first.

**Round 3 (not met).** Round 3 would have converged if no IMP had landed on draft-3-added text and no
IMP had moved a mechanism. An IMP of
either kind is a reset. If that reset is self-introduced, the next draft shrinks mechanism again rather
than adding to it. Round 2 was the first self-introduced reset: 8 of its 10 IMP landed on draft-2 text.

### §D.1 Round 1 (on draft 1 + §2.5; 0 CRIT / 13 IMP, 11 unique / 25 MIN / 14 FP) → draft 2 `0142f47a`

| F | disposition in draft 2 | fate in draft 3 |
|---|---|---|
| F1 | P-d by content, P-a by origin | kept |
| F2 | umbrella slice and memo rows | kept, pointers corrected (R2-4) |
| F3 | marker spelling; memo-row "by design"; ENVIRONMENT VARIABLES | kept |
| F4 | sites enumerated; parent not edited | **reversed** by R2-4 |
| F5 | ci.yml not edited; STOP rule | **reversed** by R2-1's cost clause: re-derive per the in-file rule |
| F6 | criterion change stated | kept; boundary added (R2-3) |
| F7 | `PATH` shim + absolute-path slot | **withdrawn** (R2-1) |
| F8 | ratchet widened to `_lbl` | kept; records for every label (R2-2) |
| F9 | separate labels for draft-2 §6.3/§6.4 | superseded: one differential, one label, one record |
| F10 | source-time snapshot; measured predictions | kept |
| F11 | one parts list, three readers | kept; guard placement fixed |

### §D.2 Round 2 (on draft 2 `0142f47a`; 0 CRIT / 10 IMP / 19 MIN / 17 FP) → draft 3

| id | disposition | verifying command / evidence |
|---|---|---|
| R2-1 | the guard, canary, marker, absolute-path slot, `envscrub` and 12 probes are replaced by **one** whole-build differential. Three universal members, each proven live alone before the hostile build. Verdict order after `ctl_ok=0`. Errexit specified. Refs are in P. Cost re-derived by the ci.yml rule, with STOP on change | prototype D (§A.4): unmutated green; `command env`, call-time `HOME`, `_git`, `PATH=… git` and absolute path all red; `env -i git` green, booked. `git show e8f78896:.github/workflows/ci.yml \| sed -n '/^  trip-wires:/,/^  [a-z]/p'` for the rule. Driver timing, three runs per side |
| R2-2 | six labels, six records; `_MUT_UNRECORDED_MAX` unchanged; X11 | P-a/P-c records measured (§A.3). Existing `_lbl` records: `/usr/bin/grep -cF` → 1 each |
| R2-3 | §0.1 boundary: P governs git's inputs, not executable or platform (R1, R2, R5). R3 booked as `#11-k2-fgit-machine-files`, which is also R3(b). Own deferrals: **1** | design memo §0.1, §5 |
| R2-4 | parent touched: C0a (move §5 and §10 to a rounds companion, stub headings) + C0b (banner), under CLAUDE.md's quoted text. Umbrella pointers name files and sections exactly. The code-reference convention is stated; references are qualified in edited files | `/usr/bin/grep -n '^## '` over the parent (section map); the 17-line reference grep |
| R2-5 | this companion | `wc -l` of both files |
| R2-6 | C3's "required" claim withdrawn and re-grounded. Full clause answered, with Q1 | design memo §0.2 |

**MINs:**
- draft-2 §6.4's "already live": a false ordering, fixed by draft-3 §6.3's liveness order.
- The union shadowing: handled by proving each member live alone.
- Hostile files are written before the build.
- The dedicated E8 repo: moot, since there are no per-channel probes.
- The guard marker reset: moot.
- C1 states the fixtures file is sourced.
- R8 now names P-d's reftable failure.
- Refs are in P.
- The non-git commands are named as the class R4 plus draft-3 §10 row 15.
- P is narrowed to what draft-3 §6.3 compares.
- C6 is withdrawn.
- The Stage-5 coverage claim is corrected (§B).
- draft-2 §8.1's attribution is fixed.
- The test-suite claim is removed.
- "Recipe" is narrowed to ingredients.
- The GNU sed claim is stated as unverified.
- The slot quotes are verbatim.
- The SSoT "12" is marked as inference.
- R1 is stated as this slice's own residual.
- The ledger step and the landing SHA are corrected.
- The land order gains user approvals, the #501 P2 thread, the status-cell update, and the #501
  decision.
- The sibling guard lives only in `_mut_run`.
- P-c runs last.
- `#501 R97` is corrected to `#519 R7`.
- The `wire` qualifier is restored.
- The draft-3 §7 closure claim is corrected.
- The umbrella pointers are fixed.

### §D.3 Round 3 (on draft 3 `2b89ef7c`; 0 CRIT / 7 IMP / 22 MIN / 4 FP) → draft 4

Round 3 did not meet its terminator: all 7 IMP landed on draft-3 text, and 4 of them moved the
mechanism. The process changed as a result: the corpus came first (§A.5), and draft 4 was written from
it.

| finding | disposition | evidence |
|---|---|---|
| Ax2/Ax3 IMP-1: the members are not universal across subcommands and fixture shapes | the differential and its members are **replaced** by an invocation-based detector (`GIT_TRACE` plus a `PATH` shim, around the one build) | §A.5 R: 130/130 × 4 configs, across all 13 call-site classes and 10 spellings, including `commit`, `update-index`, `hash-object -w`, `add -f` and `-c` overrides |
| Ax3: poison members, plus a `PATH` shim for `env -i PATH="$PATH" git` | the `PATH` shim is adopted. Poison was **not** needed: `GIT_TRACE` catches every environment-inheriting git regardless of subcommand or `-c`, and it does not break the build it observes | R rows `env -i PATH="$PATH" git`, `-c override`, `hash -p` |
| Ax2 IMP-2: canaries inherit the caller's environment, so an ordinary caller kills a member | moot: the detector's canaries test channels the gate itself sets. The caller's git configuration cannot switch `GIT_TRACE` or `PATH` off | §A.5 G: 29/29 × 4, including `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_COUNT`/`GIT_CONFIG_PARAMETERS` with `init.defaultBranch` |
| Ax2 MIN: ref values | P now includes deterministic ref values; commits are compared by tree | design memo §1 |
| Ax2 MIN: the hostile `_FIX_FAILED` has no reader; errexit differs between 5.3 and 3.2 in `( . fixtures ) \|\| rc` | moot: there is no second build. The one build is sourced in the calling shell exactly as at base | p4 |
| Ax2 MIN: two producers share one label | one label per producer for the P-a/P-b liveness halves (`e58fec48`). ⚠ **Draft 4's claim that this gave one label per producer was false**: the detector's labels still had several producers (`_fdl_lbl`: setup, PATH canary, trace canary; `_fd_lbl`: PATH arm, trace arm). Round 4 found this, and draft 5 fixes it (§D.4) | §A.5 re-run |
| Ax2 MIN: d1 did not match the design | p4 carries §3–§5 exactly, P-a…P-e included | §A.5 |
| Ax3 harness defect (`rm -rf $d/.git`) | §A.4 is marked invalid; the corpus harness keeps `.git` | §A.4 note |
| Ax3 MIN: R4 ground, errexit wording, "no measured way", cost sentence | R4 is restated as a class, with no ground argued beyond the launch slot. The errexit text now describes only what p4 does. "No measured way" is deleted along with the differential. The cost sentence is rewritten from the §A.6 data | draft-4 design memo §4, §5.2 |
| Ax5 IMP: C0a had no seam (parent §5 is live) | **C0a withdrawn**. Only the banner (C0b) remains; its site list is derived by rule and command, giving **seven** sites (round 3 said five) | design memo §8.2 |
| Ax5 IMP: the wire-reference convention resolves 8 references wrongly | the convention is **dropped**. A derived table resolves each reference (8 parent, 1 A-i, plus the qualified ones) | design memo §8.3 |
| Ax4/Ax5: the umbrella memo table lacked a row for the companion | the companion row was added in draft 3; draft 4 keeps it. The rounds companion is moot, since C0a is withdrawn | umbrella |
| Ax3/Ax5: `#11-k2-fgit-machine-files` held two gaps | **withdrawn at create time**: (a) becomes the declared blind spot R3; (b) is the invocation-convention slot's own gap, which that slot is narrowed to. Own new deferrals: 0 | draft-4 design memo §5.2, §7 |
| R2-2 carried: ratchet | measured: 30 bare labels with a widened population; 7 of the 9 new labels get records; +2 declared raise for the two version guards | §A.7 |

⚠ **Not addressed individually.** Ax4's 7 and Ax5's 7 MINs were not in this session's brief as texts,
and the review agents' output files were unreachable (dangling links under `tasks/`). Their stated
subjects were handled where named: wording/pointer fixes, memory references anchored by heading text,
and the umbrella rows. Any MIN not listed above is **open**, and it should be re-raised against draft 4.


### §D.4 Round 4 (on draft 4 `57e5419f`; 0 CRIT / 9 IMP, 5 unique / 20 MIN / 22 FP) → draft 5

Round 4 did not meet its terminator: 5 unique IMP, each with a failing cell. Nobody broke the detector
core.

| id | disposition | evidence (companion §A.8) |
|---|---|---|
| U1: the record plan | Producers move into harness functions; labels stay in the controls file and are passed in. Targets become `wire harness fixtures`: BSD `sed 'fixtures:p'` gives rc 1; GNU is unmeasured. Records come from representative cells only: 16 new labels, 16 records. `_MUT_UNRECORDED_MAX` stays 21. RES cells are not records, because of the `!survive` exactly-once rule. X2 and X5 now use the subset | P column 14/14 × 4. The record table is in design memo §6 |
| U2+U3: the escape class | Two channels are added. (a) An env shim, with `_fgit` using an absolute `env` captured at source time. (b) Trace2 via `GIT_TRACE2_EVENT` and via `GIT_CONFIG_GLOBAL` (config route), plus poison `GIT_CONFIG_COUNT=bogus`. `GIT_TRACE` is dropped, because it was measured to be shadowed. The residual shrinks to a git started from nothing by an absolute-path program. The slot is narrowed with a property-defined trigger | R column 33/33 × 4, including the round-4 escapes and `env -`; RES green; BEFORE d0234/5/7/8 silent on p4 |
| U4: P-e | The record injects `GIT_EXEC_PATH`, which reds on both gits. The `PATH`-drop mutant becomes an INFO cell and is not a record. No runner mechanism was added | P-e record PASS × 4; INFO red on git 2.55, green on 2.54 |
| U5: uninitialised state | Every detector name is assigned at harness top level. A setup failure is a labelled verdict. Nothing relies on `set -u` | setup record PASS × 4, against BEFORE d0233 (rc 1, unlabelled) and d0236 (**rc 0**) |
| Pre-existing: bash 3.2 EXIT trap plus `set -u` exits 0 | Recorded in design memo §5.2 and not fixed, because the wire is untouched. The orchestrator books the slot | `…/scratchpad/r4ax2/u.sh` (the orchestrator's) |

**MINs:**
- **Ax4, C1 range:** stated exactly as base controls lines 83–734; lines 735–737 are `ctl_ok=0`'s
  comment. The §7 census now uses `NR<=734`.
- **Ax4, the D-label collision:** labels renamed to F…, R2-…, R3-…, U….
- **Ax4, X4 cannot see comment-only qualification:** added X4b, a grep that requires a memo filename on
  every `§N` / "plan memo" line.
- **Ax4, umbrella wording:** now "one slice row, one memo-table row, one status cell".
- **Ax4, bare § references:** qualified as "draft-N §x" throughout §A–§D.
- **Ax4, the RES classifier:** now requires rc 0, 0 control lines and PASSED for RES-GREEN
  (`corpus5/cell.sh`).
- **Ax5, banner scope:** re-derived by property over C1–C5, giving nine sites, including L683 (C4) and
  L866–868 (§9, present tense).
- **Ax5, the §8.3 grep:** now covers every `§[0-9]`, which gives 9 A-i and 8 parent unqualified wire
  references. Draft 4's "A-i for 1" was false.
- **Ax5, stale header:** the pointer now reads §8.2.
- **Ax5, X8:** now names #510, with its head via `gh pr view 510 --json headRefOid`.
- **Ax5, `env -`:** closed by the env shim.
- **Ax5, in-file mutations statements** (`:44–46`, `:136–138`, `:684`): listed as C2/C4/C5
  obligations.
- **Ax5, §8.2's ground:** now states both §5 (live) and §10 (cited by live §11: `§10.4`, `§10.8`).
- **Ax2, detector labels with several producers:** now one per producer; the e58fec48 claim is
  corrected in §D.3.
- **Ax2, caller `GIT_TRACE=1`:** noted as pre-existing on the read side (R6, parent D7).
- **Ax3, §4 cost prose standing in for X8:** that sentence now points to X8.
- **Ax3, mutation-mode cost:** stated as a formula over X3's own counts.

⚠ Round 4's six Ax4 MINs and ten Ax5 MINs are addressed only where this session's brief named their
subject, which was every item listed above. Anything else stays open for round 5.

### §D.5 Round 5 (focused; on draft 5 `e8c1bdb8`; Ax2 0/1/2, Ax3 0/3/4, Ax5 0/0/6) → draft 6

| id | disposition | evidence (§A.9) |
|---|---|---|
| V1, Ax3 IMP-1: the detector is a name list (scrub3, hermetic-look, gitsweep, t2zero-countempty) | **Design change** (the orchestrating session's decision): the whole fixture-build window is built from nothing, and the detector is deleted | BEFORE 8/8 silent-wrong on p5d; AFTER, R PASS with P identical × 4 configs |
| V2, Ax3 IMP-2: a wrapper git false-reds (x005/x010) | **moot**: there is no `PATH` shim | — |
| V3, Ax3 IMP-3: the system prefix is caller-writable | stated as the residual. The claim "reads no caller input" is withdrawn | ownership measurement (§A.9) |
| V4, Ax2 IMP: the restore of `GIT_CONFIG_GLOBAL` is unpinned | **moot**: there is no restore; the window is a child. The DO cell is green with P equal × 4 | G DO cell |
| V5, the decision's own premise: "the one spelling that can still read caller state is `env -i` / `env -u GIT_CONFIG_NOSYSTEM`" | **refined**. The residual is any fixture command that removes or overrides an allowlist entry: `env -i`, `env -`, `env -u`, `NAME=… git`, `unset NAME`. RES cells pass here only because this machine's system file and template are benign | RES 2/2 × 4 with P equal; residual stated by property |
| V6, the decision's own premise: "`_fgit`'s per-call `env -i` may be redundant" | **confirmed and removed**. After the move, no use of `_fgit` remains outside the window (the postconditions moved inside), so the fixtures call `git` | `/usr/bin/grep -c '_fgit' p6/…fixtures.sh` → 0 |

**MINs:**
- **Ax5, X4b case:** now `grep -i` (it catches mutations:20 "The memo").
- **Ax5, "P (14)" vs a 16-row table:** moot; the table and count are now 9 and 9.
- **Ax5, banner seed:** the seed is widened (`.bare|mutation record|every control|unreachable or loud`), giving eleven sites, including L816 and L1212–1214. The memo now says the seed is a seed.
- **Ax5, in-file mutations statements:** now listed as `:44–46`, `:120`, `:136–138`, `:672–673` and `:684–688`, found by a stated grep.
- **Ax5, §5.2:** names `#11-k2-wire-exit-trap-masks-set-u-abort`, and nothing new relies on `set -u`.
- **Ax5, X2/X5 re-run recipe:** the scripts take a directory argument and `K2_CORPUS_OUT`, with the tree under test at `<dir>/p6`.
- **Ax2/Ax3 MINs tied to the detector** (M1 `cmd_name` absent under poison; M2/M3 detector messages and the canary order): **moot**, deleted with the detector.
- ⚠ Any round-5 MIN not named in this session's brief remains open for round 6.

### §D.6 Round 6 (full; on draft 6 `8b6a4005`; 0 CRIT / 5 IMP / 18 MIN) → draft 7

| id | disposition | evidence |
|---|---|---|
| W1 (item 1, Ax2): an incomplete window was read as "no fixture failed", and the controls cascaded | fixed on p7: W alone, the child's exit class carried (2 stays 2), no control over unbuilt trees. The W records now require exactly one control line | §A.10: `odd/pipe` went from rc 1 with 82 lines to rc 2 with 1, on both shells |
| W2 (item 2, Ax3 plus Ax2 MIN-3): add-input spellings silently diverge | the gap is re-stated by property (remove, override or add an input). S is adopted as a seed; P-a over every repo is declined; the over-claims are scoped (§3 (iii), §4, §10 rows 1–2, the P-a label); the slot trigger is honest | §A.10: SILENT-WRONG on p6 becomes S red on p7 |
| W3 (item 3, Ax4): `\|` inside table cells | every command containing a pipe now sits in a fenced block with `-e` alternatives; each was re-run | §A.10, moved commands |
| W4 (item 4, Ax5): X9 cannot run on a stacked PR | both routes, (a) a temporary draft PR to `main` and (b) #501's CI with a stop condition, are written down as a push-time user decision | design memo §9 |
| W5 (item 5, Ax5): statements that become false after `_fgit`'s deletion | harness:22–47, harness:48–49 and controls:20–22 are listed, with the grep that finds them | design memo §8.1 |

**MINs:**
- **Ax2 MIN-1:**
  - the census now has a derivation command, and it matches the prelude by static read-set;
  - `set -u` is pinned by the options check plus a W record;
  - the three sentences now say "nothing in the parent relies on `set -u`".
- **Ax2 MIN-2:** P-f now parses every `env -0` record.
- **Ax4 MIN-1:** the mutations list is `:133–138`, and the grep's full return is listed (93 and 526 are
  read and kept).
- **Ax4 MIN-2:** banner lines corrected to L747–748 and L1281–1282; L279 is noted as changing under C5.
- **Ax4 MIN-3:** 0 of 207 man pages mention `GIT_ATTR_NOSYSTEM`, with a positive control.
- **Ax4 MIN-4:** "in §E" pointers now use the history route.
- **Ax4 MIN-5:** drifted "design memo §" references are qualified by draft, with a stated convention in
  the header.
- **Ax4 MIN-6:** the ledger premise is quoted verbatim.
- **Ax4 MIN-7:** the owner is the citation-hygiene lane, and the correct replacement text for the
  exit-trap ledger entry is given.
- **Ax4 MIN-8:** A-ii's recogniser is described as line-anchored, fence-aware and §3-scoped.
- **Ax4 false premise and Ax5 MIN, round-6 scope:** recorded as it ran (§D.0).
- **Ax5, "0 new own deferrals":** corrected to 1 own deferral under a re-scoped name, with the ledger
  net at −1.
- **Ax5, ledger timing:** the ledger text is planned now (design memo §9) and written at PR creation.
- **Ax5, fixtures:20–28 `exit 2`:** subsumed by W1.
- **Ax5, X8:** where the record goes, and the interaction with #510 (head `94281cd7` at writing).
- **Ax5, umbrella "§5.1: the residual":** now points to §5.1 for the gap and §5.2 for the other
  residuals.

### §D.7 Round 7 (Step 4.5; on draft 7 `ff1322bb`; Ax2 0/2/4, Ax3 0/3/1) → draft 8

| id | disposition | evidence (§A.11) |
|---|---|---|
| D1 (Ax3 IMP-1): S is a vocabulary deny-list, with silent-wrong literal spellings and false reds | **S deleted**, along with its label, record and census, because extending it is the wrong repair | Ax3's r7ax3 cells; design memo §5.1 |
| D2 (Ax3 IMP-2): the P-a-over-every-repo decline rested on a false premise (persisting does not need `git config`) | **P-g adopted**: every fixture repo's configuration must equal a reference init's, line by line. The key set is derived from git per run, not from a path-key list | before p7: silently wrong ×3 per shell; after p8: P-g red ×4 per shell; census clean |
| D3 (Ax3 IMP-3): over-claim scope and the residual | the unit is "every git whose inputs no fixtures-file command altered". The residual is re-scoped to **transient** inputs, with a trigger of reviewer attention | transient cell h0019/h0033: silent, as declared |
| D4 (Ax2 IMPs): pinning | W2 added; one record per prelude option; `built` from the fixtures file's last line; the exit-number claim is dropped as unconsumed (driver `set -euo pipefail` plus `bash "$w"`, lines 22/204) | W2 and option records PASS; early `return 0` goes from 81 lines to W alone |

**MINs:**
- **Ax2 MIN-1 (a top-level `return 0`):** fixed by the `built` marker.
- **Ax2 MIN-2 (the class decided by value):** moot, because the number is not claimed.
- **Ax2 MIN-3 (wording, and parent-side failure mislabelled):** fixed with reason text via `_fw_why`.
- **Ax2 MIN-4 (W alone suppresses static checks):** moot, because S is deleted.
- **Ax3 MIN-1:** moot, because S is deleted.
- **The claim in W1 (§D.6) that "records require exactly one control line":** it was **false** for the
  runner. It holds only for the corpus evaluator (`eval7`/`eval8` check `ctl=1`). The mutation runner
  sees only rc ≠ 0 and a needle. Draft 8 therefore pins W2 as a separate label.

### §D.8 Round 8 (Step 4.5; on draft 8 `453b7b0f`; E1–E4) → draft 9

| id | disposition | evidence (§A.12) |
|---|---|---|
| E1: no stated threat model; sentences gave P-g the reverted forms | design memo §0.3: classes (a) closed, (b) caught on shape, (c) out of scope and owned by code review. The D3 list becomes examples of the (c) property. `#11-k2-fixture-git-invocation-convention` **closes** after the create-time audit (design memo §5.1). §5.1, the ledger text and §10 row 1 are rewritten | — (a scope decision) |
| E2: P-g's population was a top-level directory glob that skipped non-directory `.git` | census over every git dir, by property; unknown shapes red; deliberate unsearchable dirs declared | before p8: `--separate-git-dir` SILENT-WRONG ×2 per shell; nested, hidden, bare unseen. After p10: all red with P-g; clean, G and DO green |
| E3: W2 depended on where the incomplete-window exit sat | `_control`'s first statement is the W2 check | RO: p8 no W2, p10 W2 on both shells; ROg green both |
| E4: wording | §3 row (iii) in the §5.1 unit; the harness comment "however it is spelled" qualified and the exit-class comment corrected (both in C5); `_fw_why` names 3/4/5 | prototype text |
| E4 AR: an arithmetic error skips a line and the build completes | **W3** from the child's stderr, on bash's `<file>: line N:` form | p8 GREEN-SAME, p10 rc 1 W3, both shells |
| E4 SE1: `set +e` in the fixtures file | options re-checked after the file (exit 5, W). A mid-file toggle is class (c) | p10 rc 2, W alone, both shells |
| harness: c8 cells ran under the real `HOME` | `cell9.sh` sets `HOME` for every cell; c8 re-run unchanged | §A.11 note |

### §D.9 Round 9 (Step 4.5; on draft 9 `d5dacd56`; Ax2 0/1/1, Ax3 0/1/3) → draft 10, plan-review closed

| id | disposition | evidence (companion §A.13) |
|---|---|---|
| A1 (Ax2 IMP): W3 fails open under SIGPIPE (grep piped into `head -3` under pipefail exits 141, and the fallback empties the diagnostic) | `grep -m 3 -F`; rc 1 = none, any other status red | arnoise, noise1k: p10 green → p11 W3, both shells; ar unchanged red |
| A2 (Ax3 IMP): the census pruned the declared unsearchable dirs; m2h passed with P differing | the declared dirs are checked at their declared mode, opened for the census, and restored and re-checked | m2h: p10 silent → p11 P-g red on all four shell × git configs |
| Ax3 MIN-1: git-dir shape | `HEAD` + (`objects` or `commondir`); `.git` is no longer pruned, so submodule and linked-worktree git dirs are seen | wtmeta: p10 silent → p11 red |
| Ax3 MIN-2: "cannot search" | defined as what `find` reports; any report fails the census | nr: red on both |
| Ax3 MIN-3: census cost | the per-directory `sh -c` removed; the cost is measured | 1.45 s → 0.76 s |
| Ax2 MIN: W2's record | the sentence now says the record pins "W2 is reported when the exit is gone" and does not tell the two call sites apart | prose |
| Ax2 note: causes 3/4/5 collide with errexit statuses | the cause travels in a marker file | rc5: p10 misnamed → p11 generic, both shells |
