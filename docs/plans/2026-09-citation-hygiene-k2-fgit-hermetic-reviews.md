# K2 fixture git — plan-review record

This file holds every plan-review round's dispositions and the terminators, and (§8 and §13, moved from
the design memo when it neared 1000 lines) the record of the C0b–C2 splits and the implementation results, for
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

---

## §8 Splits, the parent memo, and memo references (moved from the design memo)

Moved here unchanged from the design memo's §8 as a touch-time split, when that memo neared 1000
lines; section numbers are kept, and four references to other sections of the design memo now name it.
"Parent" is `docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md`.

### §8.1 Code splits

`wc -l .claude/tools/webref-generic-core-trip-wire*.sh` at `e8f78896`: wire 1259, controls 989, mutations
783, harness 195. `ff6b99a3` broke CLAUDE.md's touch-time rule: its first commit took controls to 1026 lines
(`git show c8f52724:.claude/tools/webref-generic-core-trip-wire.controls.sh | wc -l`).

| file | split | seam | ground |
|---|---|---|---|
| controls | **C1** | build vs assert. `…trip-wire.fixtures.sh` takes **base controls lines 83–734**. Lines 735–737 are `ctl_ok=0`'s comment and stay with it. The window sources the new file | this PR grows both halves. `wc -l` of p6's parts is in companion §A.9 |
| mutations | **C2** | the header's two populations. The generated half moves to `…trip-wire.mutgen.sh` | `ff6b99a3` reached 983 lines with the infrastructure C3/C4 carry |
| harness | no | — | stays below the threshold (companion §A.9) |
| wire | **not touched** | — | `/usr/bin/grep -c '_fgit' .claude/tools/webref-generic-core-trip-wire.sh` → `0` |

C1 and C2 each bring the following with them:
- one parts list that feeds the copy, the trap's `rm -f` and the stale-skip `case` in `_mut_run`;
- the sibling guard (moved into the always-on `_mut_correspondence` by `/code-review`, `…-reviews.md` §13);
- the extended entry contracts.

**In-file statements that become false, and the commit that rewrites each.**

In `mutations.sh`, found by the stated grep, which returns lines 44, 93, 120, 134, 137, 526, 672, 673
and 684:

```sh
/usr/bin/grep -n -i -e 'every control' -e 'a control means' -e 'controls with no record' -e 'controls have no' -e '_control. call' -e 'umask and' .claude/tools/webref-generic-core-trip-wire.mutations.sh
```

| lines | statement | fate |
|---|---|---|
| **:44–46** | "Every control has to be named by some record" | C5 rewrites it: the population becomes `_control` labels ∪ `_lbl="…"` definitions |
| **:120** | "ADDING A CONTROL MEANS ADDING A RECORD" | C5 rewrites it the same way |
| **:133–138** | the correspondence: labels are a `_control`'s "or the label a block that is not a `_control` prints (the umask and fsmonitor ones)", and "the number of controls with NO record is ratcheted" | C5 rewrites it the same way |
| **:672–673** | "controls with NO record … the third quoted argument of a `_control` call" | C5 rewrites it the same way |
| **:684–688** | the ratchet messages, which say "controls" | C5 rewrites them the same way |
| :93, :526 | read and **kept**: "Every control above proves a verdict is REACHABLE" is still true of `_control`s; "Permission denied for EVERY control" is a runtime description | kept |

In the harness and controls, round-6 item 5. Found by the command below, which returns controls:20,
:22, the build region's `_fgit` calls, and harness:25, :35, :45, :46 and :48; the wire returns 0:

```sh
/usr/bin/grep -n -i -e _fgit -e 'two helpers' -e SCRUB .claude/tools/webref-generic-core-trip-wire*.sh
```

| site | statement | fate |
|---|---|---|
| **harness:22–47** | "`_fgit` is what the fixtures are built with; the channels it closes are named at its definition below"; "PER CALL, NOT `export`"; "AND IT SCRUBS THE ENVIRONMENT-PROVIDED CONFIG TOO …"; "there are two helpers: PRESERVE for the repository, SCRUB for the fixtures" | C5 **rewrites** the block. The fixtures are built in a window constructed from nothing (design memo §3). The per-call rationale (#501 R94: the real scan must keep `safe.directory`) is kept, and it now holds because the window is a child process. PRESERVE (`_git`) remains; the fixture half is the window |
| **harness:48–49** | the `_fgit` definition | C5 **deletes** it |
| **controls:20–22** | the list of names this file defines ("`$CTL`, `_fgit`, `_control`, …") and its `grep -c` command | C5 drops `_fgit` from both, because the file no longer defines it |
| controls build-region `_fgit` calls | code | move with C1 to the fixtures file; C5 changes `_fgit` to `git` |

### §8.2 The parent memo: a banner, no split (C0b)

**There is no C0a.** CLAUDE.md requires a split only "real cohesion seam があれば".
- Parent §5 is live substance: §7 criterion 4 relies on it, §9 says "the substance is in §5 and §8",
  and so does header L8.
- §10 is cited by live §11 (`awk '/^### §11/{s=1} s' <parent> | /usr/bin/grep -o '§10\.[0-9]' | sort -u` →
  `§10.4 §10.8`).

**Finding the banner's sites.** The candidate lines come from a seed command, widened per round 5:

```sh
awk '/^#{2,3} /{sec=$0} $0 ~ /trip-wire\.(controls|harness|mutations)\.sh|controls file|control harness|mutation set|_fgit|GIT_TEMPLATE_DIR|D6|\*\*Status\*\*|over the wire|controls with no record|_MUT_UNRECORDED_MAX|ratchet|\.bare|mutation record|every control|unreachable or loud/ {printf "%d\t%s\n", NR, substr(sec,1,40)}' \
  docs/plans/2026-09-citation-hygiene-Ai-wire-k2-trip-wire.md | /usr/bin/grep -v '§10\.'
```

⚠ **The seed is a seed.** Each candidate is read and kept iff it is a **present-tense statement that
one of C1–C5 makes false**. A seed cannot return that population by itself: round 5 found L816 outside
draft 5's seed. At `e8f78896` the kept sites are **eleven**:

| parent line | why it becomes false |
|---|---|
| L5 | the Status |
| L61 | §0's command names the controls file, which loses the build half (C1) |
| L278–280 | §4's file rows (C1/C2). L279, the harness row ("the fixture git helper"), also changes under C5 |
| L675–677 | §7 criterion 3, "enumerated in `…mutations.sh`" (C2) |
| L683 | §7 criterion 3, "a `sed` expression over the wire" (C4) |
| L747–748 | §7 criterion 5, "All are now in §4's table" (C1/C2) |
| L816 | Slot 2, "when every control has a mutation record" (C5: labels) |
| L866–868 | §9, "the number of controls with no record" (C5 population) |
| L1212–1214 | §11.1's rule "unreachable or loud, never silent", stated as the rule for inputs (design memo §0.1 replaces it for the build) |
| L1220 | §11.1 D6 (design memo §0.1) |
| L1281–1282 | §11.4 P1's file list (C1/C2) |

### §8.3 Memo references in code

⚠ **This table is as of `e8f78896`**: the line numbers are the base files', before C1/C2 split the
controls and mutation files (the parent memo's banner sends readers here for exactly that resolution).

Every `§N` in the parts
(`for f in .claude/tools/webref-generic-core-trip-wire*.sh; do /usr/bin/grep -nE '§[0-9]' "$f"; done`),
plus every prose "the memo" / "plan memo"
(`/usr/bin/grep -n -i -e 'plan memo' -e 'the memo' …`), is resolved by subject:

| file | lines | memo |
|---|---|---|
| wire | 4, 5, 192 (qualified); 38, 158, 195, 206, 225, 451, 461, 534, 550 (K2 predicate §2; exit criteria §12) | A-i memo (`2026-07-citation-hygiene-Ai-spec-label-map.md`) |
| wire | 307 (qualified); 199, 267, 323, 403, 456, 685, 724, 1164 (slots §8; revision §11) | parent |
| controls | 95, 116 (§2) | A-i memo |
| controls | 10, 273, 614, 906 (§8, §11 D10, §11.2, §11.1) | parent |
| mutations | 15 ("the plan memo"), 20 ("The memo") | parent (§10.5, §8) |

The wire is untouched, so this table is the record. In the files this PR edits, C1/C2 qualify every
reference with a file name, and **X4b** checks that **case-insensitively** (design memo §11).

---

## §13 Implementation results (moved unchanged from the design memo §13; the PR #527 subsection at its end was written here)

Branch `k2-wire-fgit-hermetic`: C0b `26e445fd`, C1 `b94518ee`, C2 `339b7137`, C3 `5bae4b4e`, C4
`a257d8f9`, C5 `386febc8`. Every run: both shells, `HOME` a scratch dir, fresh `git clone --local` per
commit, at most two wire runs at once. Scripts and logs: `…/scratchpad/impl/` (companion §A.14).

⚠ **X3, the per-record runs and X5 ran on `d7812ffe`, the first C5; X1 ran on `386febc8`.**
`git diff d7812ffe 386febc8 --stat` → `harness.sh | 0` (mode 100644 → 100755, restored) and
`.github/workflows/ci.yml | 6 ++++++` (X8's paragraph); nothing else.

```text
X1   x1.sh <clone> $SH                    C1..C5, 386febc8: rc=0 CTL=0 PASSED=1, both shells
X4   diff <(norm.sh base.log) <(norm.sh Cn.log)   C1..C4: empty, both shells (norm.sh drops bash 5.3's
     intermittent "child setpgid (N to N): Operation not permitted" line and collapses scratch paths)
X4b  grep block of §11 over controls, fixtures, mutations, mutgen, harness: 0 lines each
X3   WEBREF_WIRE_MUTANTS=1 $SH $W; grep -F the three §11 strings
     base e8f78896: 95 entries / 50 generated, 3 hits, rc 0
     C2, C3, C4:    3 hits, rc 0; log equal to base's minus the X3 status and setpgid lines
     C5 d7812ffe:   "115 entr(ies), 0 not killed as named", "50 … 0 neither killed", PASSED, 0 `!!`
     each of the 20 new records alone (rectest.sh): 40/40 killed with its needle
X5   gen11 AFTER + G + m2h + draft-9 set on the head (P-dump hook on the verification clone only): 38/38 PASS
X6   template in the void: rc 1, P-c; include.path in `clean`: rc 1, P-g
X8   /usr/bin/time -p bash scripts/trip-wires.sh, alternated: base 21.63 20.85 21.75 s,
     head 26.76 27.86 26.44 s; budget unchanged (5 min), paragraph added to ci.yml
X10  $SH -n on every part: clean; parts at C5 `386febc8` 382/677/383/688/337 lines; the wire 1259, untouched
X11  _x_lbl added: rc 1, "22 labels have no mutation record, against a ratchet of 21", lists it
X9   not run: user's route choice at push time (§9)
```

**Design statements that changed in implementation:** the W2 record shape (corpus §6 table, `…-k2-fgit-hermetic-corpus.md`), X6's producers
(§11), and C4's scope (§9). The other deviations are procedural; companion §A.14 lists all seven.

**The `/simplify` pass** (after `/pre-push` Stage 3; one commit on top of C5). Same verdicts; the
mechanism is simpler:
- mode restrictions are sealed after the census (`_seal`, §4), which deletes the declared list, the
  mode check, the open/restore loop and the `_pg_declared` record: **19 records, `_MUT_RECORDS_MIN=114`**;
- one `_fw_opts_on` helper before and after the fixtures file, and the child writes the cause sentence
  itself (§3); the old after-file pattern `*:errexit:*:nounset:*` also failed when the two options sat
  side by side in `$SHELLOPTS`;
- the postcondition labels reach the window by name, not positionally;
- `_mut_target` derives every non-wire pair from the part name, `_MUT_TARGETS="harness fixtures"`, and
  `_mut_splice` writes through the resolved pair; the unreachable "no arm" branches are gone;
- `_FGIT_ENV_NAMES` is derived from `_FGIT_ENV`; P-g's reference is P-a's init; P-e's reference is
  `_git --exec-path` (⚠ superseded by the `/code-review` pass below: P-e re-scoped, same-env reference); the copies are removed by one `_mut_rm_copies`; history-narrating comments and the
  two `ci.yml` re-derivation paragraphs are collapsed to the method plus one verdict line.

Its verification lines are in companion §A.14.

**The `/code-review` pass** (`/pre-push` Stage 4; one commit on top of the `/simplify` pass). Fifteen
findings, each fixed with its cell (companion §A.14):
- the window runs in its own directory, and P-b asks `git var` inside P-a's probe repo (a caller cwd in
  a repository with local configuration was a false red); a git older than 2.42 is NOT EXERCISED on
  this machine, not red;
- P-e re-scoped (§4 table); P-f checks `env -0`'s status and population; P-g reds an empty listing;
  P-h pins `LC_ALL=C`; W3 matches by path prefix; W4 makes a failed or refused seal red, and the
  manifest stores `$CTL`-relative paths only;
- the three non-`_control` blocks ask `_fw_built_or_w2` before they run, and the dead check after the
  controls is gone;
- the record-anchor check and the sibling guard are always on (in `_mut_correspondence`, skipped inside
  a mutant, whose target is edited on purpose); the guard tests the part name, not the full path;
- mutants and control children run under the driver's `$BASH`, so a 3.2 driver exercises 3.2;
- entry guards: the mutation set requires only what it reads and reports a missing `mutgen.sh` as a
  missing sibling; the fixtures file refuses to run unsourced or without `$CTL`/`$_FW_DIR`;
- the forwarded postcondition labels are derived from `_fgit_postconditions`' body, and a missing one
  refuses the window.
Not taken: P-g's per-repo `git config` forks and `_mut_correspondence`'s greps (efficiency only).
Records: **116**, `_MUT_RECORDS_MIN=116`, `_MUT_UNRECORDED_MAX=21`.

### PR #527 — external review and the fix-delta reviews (2026-09-28)

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

**E1, the ground for M-PATH** (Fable; the artifacts are in the orchestrating session's scratchpad,
`fable/sb-base` and `fable/e1/`). A `git clone --local` of `e8f78896` held `bin/git`, which logs `$PWD`
and then execs `/usr/bin/git`. The wire ran from the clone's root with `PATH='~+/bin:/usr/bin:/bin'`,
and it PASSED (`e1/base.log`). `wrapper.log` has 668 lines, every one the clone's root
(`sort e1/wrapper.log | uniq -c` → one line, 668). So the wrapper served the wire's own reads and **no**
fixture's git: the fixtures `cd`, and `~+/bin` then names a directory with no `git`. A caller `PATH`
whose meaning depends on the working directory was never a supported surface for the build.

**Superseded by draft 11**, with E1 as the citation. These are the fixes the normaliser grew:
- R11–R13: relative entries, then `~`, then `~login`, ending in pinning `git` and dropping every
  non-absolute entry;
- resolving relative and empty entries against the wire's directory (R23), and dropping them when that
  directory holds a `:` (re-check #9);
- expanding `~` and `~login` as this bash does (R24①).

Under M-PATH, the callers those fixes served (a `tools/bin` wrapper with its interpreter beside it; a
`~/bin` wrapper with a helper) may go red, which is loud; X14 records the outcomes. R24's rule for
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

**Records planned by draft 11** (design memo §9.1, corpus §6.1): −1 (P-j per-entry), +1 (W timeout),
+1 (W2 untrusted), +3 (P-g shape) = **+4**. That makes **54** window records after the 95 base, **149**
in all, `_MUT_RECORDS_MIN=149`, labels 16, `_MUT_UNRECORDED_MAX=21`. None of this is implemented yet.
