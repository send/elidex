# K2 fixture git — PR #527's implementation record, after `…-impl-cr.md` closed (the CR2 round)

This file continues `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-impl-cr.md` ("`…-impl-cr.md`"), PR #527's
implementation record, for `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic.md` ("the design memo").
The design memo's §13 (the rollover, D19-C) sends every entry from PR #527's CR2 round on here: `…-impl-cr.md` reached
796 lines with CR-S8b's entry and its X3 run, and closed at 797 with the pointer to this file that ends its §S. Its body
is not edited; a correction to it is recorded here, in the entry of the slice that makes it.
The round's plan is `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan.md` ("the plan") with its companion
`…-cr2-plan-exitpath.md` (the plan's §6; the plan writes its sections as x§6.n); its sweep and the instruments its
claim, guard and blind-spot rows cite are in `…-cr2-sweep.md` (§E; its index names the three instrument companions,
`…-cr2-instruments-*.md`, that hold §E.2 and §E.3).

The working files (decisions, the review reports, the authors' progress, runner cases, logs) are in the driver's
handoff directory outside the repo, as for `…-impl-cr.md`. This file states what they hold and measured, so that a
citation in the repo resolves in the repo; every result below is relayed from those files as the authors, reviewers
and the driver recorded it, and was not re-run for this file, except where an entry says "measured for this file".

---

## §0 Spec coverage map

**No spec surface** — this file records review dispositions and measurements only. It has the same
by-design shape as the design memo's §2.5: `preflight.py` exits 1 here because no table follows this
heading, and the repo-wide preflight command runs over every tracked plan memo.

---

## §T PR #527's CR2 round: the pre-push gate's eight own items

§T.0 holds the round's setup and the plan's review; §T.1 onward one entry per slice, in the plan's order (CR2-M,
CR2-0, CR2-B, CR2-A, CR2-C1, CR2-D, CR2-C2); §T.Z the round's closing entry (the plan's §11 and §12).

### T.0 The round's setup and the plan's review

**The gate.** `/pre-push` ran at `706049ad` on 2026-10-20; the user approved the push, and it waits for this round
(`DECISIONS-R7.md` "## Push gate"). Stage 4, `/code-review max` over `origin/main...k2-wire-fgit-hermetic`, confirmed
15 findings, CR#1–CR#15; the handoff keeps the review text, one record per finding (file, line, summary, failure
scenario). Stage 5, `/elidex-review` over the same diff, raised ER-A3-1/2, ER-A4-1/2/3 and ER-A5-1/2/3. Axis 5 found
that PR #527's base is #501's branch `webref-cite-audit-tool` (tip `e8f78896`, #519's squash), so the diff also held
#519's and #501's code. Each item was therefore classified by its origin (blame against that branch). This is the
classification the plan cites, with each defect as the review stated it (abridged); its first seven rows are the items
it lists under "#527's own" (the eighth is the plan's O-3):

| item | site at `706049ad` | defect (the review's statement, abridged) | origin |
|---|---|---|---|
| CR#4 | mutations.sh :562 | the census's rule 1 strips `'…'` and cuts at ` #` / `(#` without tracking double quotes, so a label used only inside `"…'…'…"` or after `" #` is not counted: a false green, against the "every error is a false red" statement (mutations :552, `…-impl-cr.md` :623–625, residuals :120) | `c46fc6c1` (C1) |
| CR#7 | records.sh :81; the anchor check | the always-on anchor check asks only "exactly one changed line": records 45/46/81, which rewrite the whole `K2RE_PATH=` line, can never go stale, and records 89–92/97, anchored on the first of two `[^/[:space:]]+/`, move to another segment silently | `04466154` (moved); the check in mutations.sh |
| CR#9 | controls.sh :708 | the fsmonitor block's plain-git retry unsets a hand list of three variables, not `$_GIT_LOCAL_VARS` (wire :669): red with a wrong diagnosis under an unreachable `GIT_OBJECT_DIRECTORY` or `GIT_COMMON_DIR` | `d976f239` (S1') |
| CR#11 | wire :425 | on SIGTERM the EXIT trap enters `_k2_exit` with `$?` = 0 and prints "CONTROL FAILED (the run ends only through a verdict)" though no control failed; the watchdog (`set -m`, its own group, `sleep 30` then `kill -9`) outlives the wire by up to 30 s | `8f4ad3c8` (pushed) |
| CR#12 | wire :424 | the EXIT trap's `rm -rf "$SCRATCH"` runs under `set -e` without `\|\| true`: a failed removal stops the trap before `exit "$1"`, so every status becomes 1 | `8f4ad3c8` (pushed) |
| CR#14 | ctlrun :307 | `_rc` and `_out` are globals not reset on an early return, and `$_VFY/.control_out` is not cleared before the launch: a child whose output redirect fails can be judged by the previous control's verdict | `16efb423` (S2) |
| ER-A5-1 (IMP) | the #519 memo :28–:36 | the banner (lines #527 added) names `_gopt_ok` and `_ctl_need` (0 hits in the wire) and a ratchet population older than C1's census; deferred to "the next PR touching this memo" with no slot | #527's lines |
| the watchdog's kept stream | ctlrun's watchdog | found by plan-review R1's F3 measurement, not by the gate: the watchdog keeps the caller's stream (the plan's O-3) | `c3c0b74b` (not an ancestor of `origin/webref-cite-audit-tool`) |

ER-A5-3 and ER-A3-2 (MIN, ci.yml :180–186, the timeout comment) are landing-tracked: `…-landing.md` §9.1 step 2
rewrites that comment.

**The base PR's items go to #501's loop, not this round**: CR#1 (wire :401: `mktemp` output unchecked and `_phys ""`,
so on bash < 5.3 `cd ""` succeeds, SCRATCH becomes the caller's cwd and the EXIT trap removes it — data loss,
measured; it blocks #501's merge), CR#2 (controls :567/:511, bnd/seg), CR#3 (wire :1061/:1123), CR#5 (wire :1067),
CR#6 (wire :762), CR#8 (wire :1251/:1199), CR#10 (wire :929), CR#13 (wire :341, `CDPATH`), CR#15
(`_webref/spec_labels.py` :118), ER-A5-2, ER-A3-1, ER-A4-1/2/3. Wire :424 and :425 are #527's (`8f4ad3c8`) though the
wire is otherwise #519's.

**The plan's review** (`/elidex-plan-review`, frame `706049ad`; driver decisions in `DECISIONS-R7.md` "## CR2
plan-review (2026-10-21)"):

| round | text | findings after de-duplication | outcome |
|---|---|---|---|
| R1, five axes | r1 | F1–F34 (the report counts IMP 13, MIN 19, FP 8) | r2; the scope became eight items (O-3), F4's adaptation accepted (the signal control's expectation from a deliverability probe, BS-11) |
| R2, five axes | r2 | G1–G40 (IMP 10, MIN 30) | r3, which split §6 into the companion; tiering decided (CR2-B's and CR2-C2's coders start at `opus`) |
| R3, five axes in full (the split) | r3 | H1–H28 (IMP 8, MIN 20) | its terminator test fired (IMPORTANTs the plan itself introduced, two rounds running; input +48 %), so r4 changed altitude: a mechanism finding became a decision plus a required claim or record row, and the two files' net line count could not grow (≤ 1383) |
| R4, a focused re-check (two reviewers) | r4 | J1–J29 (IMP 4, MIN 25) | terminator met: no design-level IMPORTANT outside r4's own additions; J1, inside them, fixed with a claim row (CR11-J); the wire's stop line base + 90 accepted (J22) |
| the driver's diff read | r5 | none at design level | **TERMINAL, 2026-10-21** (no R5 round) |

Every finding was decided by lens; the plan's change maps name the sections each one changed. The accepted r5 text is
961 + 419 lines. The diff read recorded that the line cap held (1383 → 1380) while the characters grew (r3 154.7k →
r4 162.0k → r5 170.7k), partly by joining wrapped lines: a cap on lines bound the wrong unit. It was not reopened.
The r5 text moved into the repo at CR2-M (§T.1) with the re-points the plan's CR2-M row names and the amendments
§T.1 records.

### T.1 CR2-M — the move, the sweep and the instruments, the record rollover

This commit (on `706049ad`; one commit, as the CR2-M row says). The accepted r5 plan and its companion moved into
`docs/plans/` as `…-cr2-plan.md` (962 lines) and `…-cr2-plan-exitpath.md` (419). The plan changed by the CR2-M row's
re-points (:7–:8, :10, :63, and the row's own parenthetical, which named two handoff files and is counted as a re-point
the row requires) and by the amendments below; the companion by its status line. The handoff-name grep gives 0 over
both files.

**Reviews.** A first author committed the row as written; a fresh review of that text found CRIT 0 / IMP 2 / MIN 6.
A second author applied the driver's decisions; the driver read the diff, and a focused re-check of that text found
CRIT 0 / IMP 0 / MIN 3, applied in this commit: `…-cr2-sweep.md`'s §E index excepts `§E.sweep.sh` from its
byte-identity sentence (MIN-A), its header names the plan companion's Texts line as the source of the frozen texts'
prefixes (MIN-B), and this entry exists, so that §T.0's "the amendments §T.1 records" resolves (MIN-C).

**Plan amendment, made in this commit (the driver's decision on IMP-1; the first author's D1).** The row put every §14
instrument verbatim into `…-cr2-sweep.md`, which made it 2725 lines where every other k2-fgit file is under 1000; the
archive has real seams and a verbatim instrument archive is not a generated table (CLAUDE.md touch-time split). It is
split at its seams: `…-cr2-sweep.md` (746) keeps the header, §0, `sweep.sh` and `idents.tsv`, an §E index naming the
file of every block, §E.1 and §E.4; `…-cr2-instruments-probes.md` (788) holds §E.2, `…-cr2-instruments-texts-r2-r3.md`
(783) §E.3's [r2]/[r3]/[r3f] half and `…-cr2-instruments-texts-r4-r5.md` (479) its [r4p]/[r5p] half, each with one
no-spec-surface `## §0`. 65 blocks before and after: 64 identical from heading through fence, `sweep.sh` changed by its
exclusion (below); the 63 file blocks equal the files as run. The split falsified the plan's counts, amended here: CR-U
(seventeen → twenty; the four → the seven), §7's umbrella paragraph, and the CR2-M row (its files; its completion:
each of the seven with one `## §0` and under 1000 lines, §E reached through its index). Every §E cite in the plan, its
companion, the sweep and this record resolves (58 of 58). §14's "copies them into `…-cr2-sweep.md` §E" resolves through
the index and is unchanged; the change map's H5 row is r4's history.

**Exclusions (IMP-2: FP by §10's J23 preamble).** J23 already left CR2's own plan, companion and sweep out of CR4-2,
CR-D2, CR-S and CR-P; it and §11's sweep exclusion now name every `…-cr2-*.md` (the instrument companions in,
`…-impl-cr2.md` not), and `sweep.sh` excludes `docs/plans/…-cr2-*`. Both value sets, measured for this file at the
fix round's text (before this entry), each over its row's population (`docs/plans/*k2-fgit*` with the parts for
CR4-2, CR-S and CR-P, case-insensitive; with the umbrella for CR-D2): verbatim, CR4-2 20, CR-D2 30, CR-S 32, CR-P 6;
with J23's exclusion, 17, 24, 27, 4. CR-S's 27 is its own ERE, `three standing` (§1 (7)); `three standing checks`
gives 26, because harness :750 wraps after `standing`. The excluded hits are the plan's own statements of the claims (J23: they state the identifiers, they are
not sites); the instrument companions hold none, so the split moved no hit.

**The review's minor items.** Both status lines read "accepted (plan-review TERMINAL 2026-10-21, `…-impl-cr2.md`
§T.0)", on the CR round's precedent (`…-cr-plan.md` :3, rewritten at its move) (MIN-4; D3). Plan :8's re-point says
§T.0 "records each finding's statement, abridged" (MIN-2). §T.0 labels its first seven rows "#527's own", the label
plan :7 and :63 cite (MIN-1), and CR#9's row reads "wire :669", where `_GIT_LOCAL_VARS` is defined (MIN-3). Two
corrections are recorded here, the frame-tagged text unchanged: §11's "cr-sweep :781–:782" is, at `706049ad`, the
list's end :779–:781 and the exclusion :782–:784 (comment :782–:783, `case` :784) (MIN-5; D5); §11's B2a I-2 list
names no group for CR2-M, which changes the referent of `family` ("the open record"), so this slice re-read every
`family` hit (below) (MIN-6).

**The first author's other items, as the driver disposed of them.** D4: the sweep found one current route outside
the row's files, landing :132 (the open record named as `…-impl-cr.md` §S); its re-point to "the open record (§13)" is
accepted, by the design memo's :744 ("Future-tense routes say 'the open record (§13)'") and §11's present-tense rule.
D6: §13's existing format is kept ("(closed at 797 lines)"). D7: §11's "the phrases of §10" enter `idents.tsv` in the
slice that introduces each phrase; none is introduced here, so `idents.tsv` is the CR round's as committed at
`706049ad`. D8: the CR round's `…-cr-plan.md` and `…-cr-sweep.md` stay in the sweep's file set (fail-safe: more hits,
nothing hidden); their hits are classified `hist`.

**The rollover.** `…-impl-cr.md` closed at 797 lines with one pointer; this file opened with §T.0; the design memo (757)
names it in its header and §13, and its header names the seven CR2 files; the umbrella registers the seven (All twenty
files).

**Sweep** (§E.sweep.sh at this commit's text, 29 files, b53 = b32 byte-identical). Base `706049ad` 2276 lines; the
fix round's text 2289; this commit 2296. Every `family` hit (38 before this entry) was re-read: current and
rewritten here (design memo, umbrella, `…-impl-cr.md` :797, landing :132), location cites that still resolve (`stays`),
the CR round's closed plan and ledger (`hist`), and this file's record (`hist`). The 7 lines this entry adds are all
its own (`family` 5, `standing` 1, `resid_range` 1): this record's statements, `hist`; the edits to `…-cr2-sweep.md`
are outside the set, and no other hit moved. The `perl -0777` wrap backstop gives six matches, byte-identical to
§S.8's six. Populations that later slices inherit: CR4-2 17 (the new hit is §T.0's record of CR#4: a closed record),
CR-D2 24 (the design memo header's "plan-review R1–R4": unrelated). With this entry's own lines, CR-D2 is 25 and CR-S
28 (27 for the longer phrase): each added hit is this entry's statement of a population, a record (`hist`).

**Battery: not applicable to CR2-M.** The diff touches `docs/plans/` only, so no PARTS file changes; and the runner's
X tree is the worktree's PARTS (`runners/lib.sh` :2 `WT`, :6 `PARTS`, :12 the copy), so X == BASE by construction.
The b53/b32 run at `706049ad` (88 cases each, X = BASE in every case on both shells) is the frame's battery, kept as
the base cache for later slices, not a measurement of CR2-M. §11's battery clause, unlike its arm table, has no
"written equivalent reason" exemption: a plan defect, recorded here.

### T.2 CR2-0 — the prereq split: the always-on half out of mutations.sh

**CR2-M after its transplant** (`b85b67e3`, the plan's §11 procedure): ship ×4 ({BSD, GNU-like grep} × {b53, b32})
and `scripts/trip-wires.sh`, all rc 0, PASSED, 0 CONTROL FAILED (the handoff's `cr2/logs/cr2m-ship/`).

This commit (on `b85b67e3`; the plan's §8 and its CR2-0 row). The always-on half of mutations.sh, `_mut_cap_member`,
`_mut_correspondence` and `_mut_census_end` (frame :443–:747, 305 lines), moved to
`.claude/tools/webref-generic-core-trip-wire.correspondence.sh` (330 lines: a 25-line header, then the block at
:26–:330). The block differs from the frame's in two comment lines: frame :653's "see "UNLISTED" above" now names
mutations.sh (correspondence :236), and frame :478's "static properties of this shipped file" now says "of the shipped
files" (correspondence :61; review M-1 below). mutations.sh sources the new part after the records (:135–:145, the same
existence check, exit 2 "This run decided nothing"). Its header names the new part (WHAT IT DEFINES, with
`_MUTCORR`), lists it among the parts `_mut_run` copies (:32–:34), and the seven references into the moved code
(frame :16, :158, :189, :199, :201, :251, :750) now name it. `_MUT_PARTS` (controls.sh:124) gained `correspondence`,
and records.sh:360, the only `mutations:` record, is now `correspondence:` (corpus §6.1, row CR2-0). The comments that
say where a moved thing lives name the new part: controls.sh :19, :76, :99, :121, :153–:154 and :771–:772, ctlrun.sh
:34 and :148, records.sh :13, :28 and :34, mutations.sh :10. Two comment fixes outside §8's list were needed by the
split: the prefix list at frame :161 (now :172) gained `correspondence:`, and frame :177–:178 (now :188–:189),
"`mutations` is THIS FILE: the census and the ratchet are its arms", now says that the census and the ratchet belong
to `correspondence`, the file's always-on half. **Kept, true:** mutations :90–:95, which says `_mut_correspondence`
"RETURNS a status now" (the function moved; the entry-guard rationale it serves did not), and records :10–:11, where
"the mutation set holds the READERS … the always-on correspondence and anchor checks" (the umbrella: the readers are
in mutations.sh and correspondence.sh, whose header calls itself the mutation set's always-on half).

**The review** (fresh reviewer on the slice before its fix round): CRITICAL 0 / IMPORTANT 2 / MINOR 4, each disposed
of in this commit:
- **I-1, fixed.** mutations.sh's header listed seven parts `_mut_run` copies beside each mutant, without the new one;
  it lists eight (:32–:34).
- **I-2, fixed: a #527 defect from before CR2.** landing's "X5 at T" gen11 patch (landing :500) gives the cell
  generator a part list that lacked `'.ctlrun'` since `16efb423` split the control run primitives out of the harness
  after the list was written. Every X5 cell at T would exit 2 at the harness's ctlrun existence check (harness.sh
  :879–:883), whatever the cell tests. It is fixed here because this slice edits that line (`'.correspondence'`): the
  list now gains `'.ctlrun'` too and names every part (the wire, controls, harness, fixtures, mutations, ctlrun,
  correspondence, mutgen, records: the nine files `$SH -n` reads).
- **M-1, fixed.** correspondence :61, "static properties of this shipped file" (the record checks, whose subject is
  the record table, records.sh since `04466154`, and the labels), now says "of the shipped files". With :236, the
  moved block now differs from the frame's in two lines (`diff` of frame :443–:747 against correspondence :26–:330).
- **M-2, fixed by one rule:** a comment that says WHERE a moved thing lives names the file that holds it; "the mutation
  set" stays where it means the umbrella (mutations.sh and its parts together) or mutations.sh by the name its header
  gives itself (THE MUTATION SET), which the focused re-check read as broader than the decision's wording and the
  driver ratified as the rule. The per-site readings: controls
  :19 and :76 (the census) now say "in the correspondence part"; mutations :10 says the correspondence is enforced "by
  its always-on half"; records :34 names `…trip-wire.correspondence.sh` for the moved `wc -l`/`awk` notes. Two sites
  split by meaning: controls :121–:122 (the sibling guard is in the correspondence part; the entry check, the per-run
  copies, `_mut_rm_copies` and the `_MUT_TARGETS` derivation stay "the mutation set", all in mutations.sh) and ctlrun
  :34 (the census's membership check "(correspondence)", `_mut_run`'s excuse "(mutations)"). records :10–:11 stays
  (the umbrella reading). The other lines with "mutation set" (`/usr/bin/grep -ci 'mutation set'`: 38 over the six
  parts that have any, controls 4, ctlrun 1, mutations 10, correspondence 10, records 7, mutgen 6; harness and
  fixtures 0; controls :122 and records :10 among them, read above) name the umbrella or mutations.sh itself (its
  header calls it THE MUTATION SET), or are runtime message strings, and are unchanged.
- **M-3, fixed: the headers match the code's non-comment uses** (`/usr/bin/grep` over non-comment lines, at this
  commit). mutations.sh no longer claims the run registry or `$_cen_lbl` (0 uses each), says `$_CONTROLS` and
  `$_lbd_files` are only asserted at entry there (1 use each, :99) and read by the census, and adds `_mut_run`'s read
  of `$_VFY/.ctllabels` (2 uses). correspondence.sh no longer claims `$_lbd_names`, `$_MUT_TARGETS`,
  `_mut_regex_mutants`, `$K2RE` or `$K2RE_PATH` (0 uses each) and adds the run registry `$_VFY/.registry` (4 uses).
- **M-4, carried to CR2-D (owner).** The #519 memo's banner (:21–:23, :37) lists the split files without the new part.
  The 1414-line memo is split at CR2-D first, so CR2-D's banner commit adds `correspondence` to those file lists; the
  CR2-D brief carries it.

**Checks (the coder's, relayed; CR2-0-PROGRESS), on the slice before the fix round.** `$SH -n` over the 9 files × b53,
b32: 18 of 18 clean. `_mutants` 251 on both shells. The new part moved away: rc 2 on both shells, with its "missing or
unreadable … This run decided nothing". Record 360 opt-in (`r3-optin.sh <clone> <sh> <log> 360 default`, index 204):
`mutation set: 251 entr(ies), 0 not killed as named, 0 not exercisable` on b53 and b32 (rc 1 is the method's stubbed
generated set, "ran 0 mutant(s) … 50", the same on both). Ship ×4: rc 0 and PASSED, each log byte-identical to
`cr2m-ship/`'s (the output prints no line numbers). `scripts/trip-wires.sh` PASSED. **The fix round's** (comments
only): every part's non-comment lines are unchanged against the pre-fix copies, and `bash -n` is clean on the 9 files
under b53 and b32, 18 of 18. **Measured for this file:** CR-8 (`awk '/^[[:space:]]*#/' | grep -cE '[0-9a-f]{8}'`)
0 for mutations, 0 for correspondence; `$SH -n` 18 of 18; `wc -l` as below.

**Docs.** Every present-tense cite into lines that moved or shifted was re-pointed or kept as true. mutations.sh's
later lines shift (frame :135–:442 +11, :748–:899 −294; frame :53–:134 keep their numbers, the header above them
netting 0), so the search covered every line, not only :443–:747. Line cites (`mutations(\.sh)?` with a line number,
over the sweep's file set minus `…-cr2-*`): every hit is frame- or commit-tagged (the corpus rows "at `8413a4db`"/"at
`e8f78896`", census :161–:162, §T.0's table at `706049ad`) or in a closed record, so all are `hist`; landing :417's
X4b cite (lines 15 and 20 of base mutations.sh) is at the base, and lines 1–42 did not move; the parts cite no
mutations.sh line. Part words (before this entry): `mutations` as a word, 283 hits. 12 are in code and current: 7
rewritten or new (controls :124, :153; correspondence :3, :236; mutations :172, :188; records :13), 5 unchanged and
true. 10 are in present-tense docs: landing X10 :399 (the part list gains `correspondence` and "CR2-0 splits
correspondence out of mutations") and "X5 at T" :500 (I-2) are rewritten; corpus :368 is new; corpus :385, landing
:417, the design memo :189 and :240, residuals :59 and the #519 memo banner :15 and :25 are true and stay (`srcrw`'s
append to mutations.sh still overrides, because the new part is sourced before the append). The other 261 are `hist`.
The `correspondence` group (before this entry), 81 hits: 38 in code (28 rewritten or new, the fix round's six among
them: controls :19, :76, :121, ctlrun :34, mutations :22, records :34; 10 true) and 43 in docs (3 rewritten or new, 7
true, 33 `hist`: the #519 memo's body, closed records, corpus rows at a commit, landing's C11 row). "below"/"above" in
the split files: 31 at the frame, 26 now (mutations 17, correspondence 9). The 7 references across the seam were
rewritten to name the new part, and :236 now names mutations.sh but keeps its "below" about its own block. Two
"below"s are new and resolve (mutations :136, correspondence :12). The other 23 are on one side, with their targets on
that side.

**`…-cr2-sweep.md` §E.** The new part joined `sweep.sh`'s file set, and its comment line now records that (§11).
`idents.tsv` gained `correspondence` (flag `i`). §11 names `correspondence` as a CR2 group, and this slice adds the
part word. The §E index's byte-identity sentence now excepts the file set and the appended groups. The header says
`idents.tsv` is the CR round's with each slice's groups appended.

**Sweep** (§E.sweep.sh and §E.idents.tsv at this commit's text, 30 files, b53 = b32 byte-identical; the handoff's
`cr2/logs/cr2-0-sweep/`, the fix round's runs under `fix/`). Base `b85b67e3` 2350 lines (45 grep errors there, one per
row: the new part is absent at the base); this tree before this entry 2390. Of the 40 lines, 28 are the
`correspondence` group's (53 → 81), 10 are the new part's header and mutations.sh's new lines repeating consumed and
defined names (`cap_fn` 1, `cap_table` 2, `controls_v` 1, `lbd` 1, `part_path` 2, `ratchets` 1, `registry` 2), and 2
are the corpus row (`cen_vars`, `family`). With file names and line numbers set aside and mutations.sh and the new
part read as one file, the base and this tree differ in 18 hits removed and 58 added. Every one (18 of 18, 58 of 58)
is a line of the slice's diff, one it rewrote or added (the parts' headers and location comments, records :360, corpus
:368, landing :399 and :500), each re-read and true at this commit (the headers by the non-comment greps of M-3); the
hits that moved with the block keep their text. The fix round's own share, against the tree before this entry as the
review read it (2387): `correspondence` +6 (controls :19, :76, :121, ctlrun :34, mutations :22, records :34),
`registry` +1, and `cen_lbl`, `cen_vars`, `lbd`, `ratchets` −1 each (M-3's dropped claims), each a rewritten
statement; landing :500's I-2 edit changes a hit's text, not the count. The `perl -0777` wrap backstop: 6 matches at
both, the same six as CR2-M's, with mutations :206/:842 now :217/:548 (`wrong reason`, both on the staying side). With
this entry the sweep gives 2437 lines. The entry's own 47 lines (`correspondence` 35; `registry` 3; `lbd`, `ratchets`
2 each; `cap_table`, `cen_lbl`, `cen_vars`, `controls_v`, `wrongreason` 1 each) are this record's statements, `hist`.

**Runners (outside the repo).** `runners/lib.sh` `PARTS` gains `correspondence.sh` and its :8 comment says "the 9 wire
files"; `runners/rec.sh`'s target list gains `correspondence` (backups `*.pre-cr2-0`). The closed round's `_crs5f*`
scripts are untouched. `r3-optin.sh` edits its clone's mutations.sh in place, so a clone holds one opt-in run at a time
(two concurrent runs on one clone race and die silently). The coder used two clones, `cr2/work/cr2-0-optin` (b53) and
`cr2-0-optin32` (b32).

**Budgets** (`wc -l`): mutations 605 (stop 900) against the plan's ≈ 598: the source guard is 11 lines and the header
nets 0, where the plan estimated +4, so the result is 7 over. correspondence 330 (≤ 450; estimate ≈ 330). controls 774,
ctlrun 436 (≤ 439), records 470. This file stays under 800.

Review: CRITICAL 0 / IMPORTANT 2 / MINOR 4, all fixed or carried above. Focused re-check of the fix round: CRITICAL 0 /
IMPORTANT 0 / MINOR 2 — M-2's reading (ratified above) and mutations :7–:10 naming the enforcing file only at :16 (kept:
not false; low confidence); it confirmed landing :500's `.mutgen` and `.records` were at the base, so the list now names
every part. Battery (`BASE_REV=b85b67e3`, the worktree holding the final slice diff; handoff `runners/logs/battery-{b53,b32}.cr2-0-full.out`):
88 cases per shell, 81 new == base at the first run; 7 were EDIT FAILED on the new tree only, their edit targets moved
into the new part (M15nonote, M16labels, S11nokindrule, S12noendC1: the census awk, its `.census` write,
`_mut_census_end`) or rewritten (S5aentry, S5apartsdrop: `_MUT_PARTS`'s literal; S5aentryng runs S5aentry). They were
retargeted, not retired (handoff `cases/_ed.py` gains `ed_in`/`ed1_in`/`has_part`: the first part that holds the
anchor, so the base edits its own text), and re-run on both shells: 7 of 7 new == base, the same rc class (1/1, 1/1,
0/0, 0/0, 2/2, 1/1, 1/1). So 88 of 88 new == base on b53 and b32: no behaviour change. (The earlier run on the slice
before its fix round was stopped; the battery runs on the final diff).

### T.3 CR2-B — what a fresh anchor is (CR#7)

**CR2-0 after its transplant** (`f35bfde3`, the plan's §11 procedure): ship ×4 ({BSD, GNU-like grep} × {b53, b32})
and `scripts/trip-wires.sh`, all rc 0, PASSED, 0 CONTROL FAILED (the handoff's `cr2/logs/cr2m-ship/cr2-0/`).

This commit (on `f35bfde3`; the plan's §3, its CR2-B row and claim rows CR7-1…CR7-T; the code as it stands after
the three fix rounds below). The always-on anchor check required only that each record expression change
exactly one line of its target, a rule about the effect's size, not about what the expression reads; an anchor that
occurs twice on the line, or a wildcard that restates text it never read, stayed "fresh" after the text it was written against was
respelled. It now requires the expression to be **fresh**: one line, position-unique (applied twice = applied once) and
faithful outside the changed span. In the correspondence part, `_mut_fresh <expression> <target>` (:55–:125, its header
:33–:54: the definition :33–:42, the variants :43–:48, the blind spots :49–:54) sets `_mf_v` to `stale`, `count:<n>`,
`twice`, `carry` or `fresh`, or returns 1 with the reason when a tool fails (decided nothing). One function judges and
reds: `_mut_fresh_gate <expression> <target> <what>` (:126–:152) prints the verdict's message (the old two for `stale`
and `count:<n>`, two new ones for `twice` and `carry`, one for a tool failure) and reds (`_mut_corr_bad=1`, status 1) on
anything but `fresh`. The anchor check (:338–:390) calls it per expression (:387). The rule's self-test (:346–:364) runs
first, through the same gate, on the fixture `a=1`, `x ab ab`, `k=vx` with eight rows: the plan's §3.3 five, the two
fix round 2 added and the one fix round 3 added (below); it asserts each verdict and the red (`_mut_corr_bad`, status, message) present on every row
but `fresh` and absent on it, and reds a disagreement as `CONTROL FAILED ($_af_lbl)`. It runs above the `*.mutant.*`
skip (:365–:368), so inside every mutant. `_af_lbl` is new in controls.sh's non-`_control` list (:119; the list comment
:102–:107 names it). mutations.sh's format header states the three clauses (:169–:171). In records.sh the 24 non-fresh
records are re-anchored in place, line 81's record is a `#` comment line, and eight records are added at the end of the
table, no existing line moved: R7a–R7d (:471–:474, comment :469–:470) disable one verdict arm each, R7e (:476, comment
:475) the gate's red step, R7f and R7g (:479–:480, comment :477–:478) the exact path and the same-class probe, R7h
(:482, comment :481) the `\001` probe; `_MUT_RECORDS_MIN` is 251 → 258 (mutations :272). Corpus §6.1 has one row per
changed record (24 re-anchors, the remove row for records.sh:81, eight adds; claim row CR-7) and the totals (164 of
this PR, 258 in all; `_mutants | wc -l` 258 on b53 and b32, 251 at `f35bfde3`, measured for this file). The design memo's §4 has row AF, the definition.

**The variant passes' shape (delegated by the plan to the coder, §11's model note; corrected by the review).** The
first draft laid each changed line's variants **in place** of that line in one copy of the target, ran one `sed` over
it and compared the output from the changed line on, and claimed that "an address range still opens and closes around
them". The review showed that wrong (I-1 below): consecutive variants do not preserve what a line-dependent form reads.
A range's second address can match the first variant and close before the rest, which then come out unread (a false
`fresh`); a numeric or `$` address, or `n N D G H x P`, reads line position or a neighbouring line, which K variants
in a row are not. The pass now reads the expression's form first (the awk at :73–:102, which skips the delimited
`/re/`, `s` and `y` parts with `\` escaping as sed reads them and, since fix round 3, reads a regex's bracket
expression to its `]`, sending one that holds the delimiter, or is unclosed, to the exact path): a **line-local** expression, per command one `s`, `y`
or `d` under at most one `/regex/` address, keeps the in-place pass (:118–:122); **any other form** takes the **exact
path** (:105–:116): one whole-target copy per variant with only that line replaced, one `sed` per copy, and each output
compared whole against the target with its variant unchanged (read) or `.anchor` with the variant's image (carried).
Each character outside the changed span gets two probes where it has a class: `\001`, and a same-class substitute (a
lower-case letter `q`, `q` itself `j`; upper-case `Q`, `J`; a digit `7`, `3`), so a class that `\001` stops but a
letter passes (`[a-z]` written back as a fixed letter) is read. Values go through `ENVIRON`; seven processes per
line-local expression (:345). The coder also made one choice the plan did not state, fail-closed: a one-line change is
located by its `NcM` diff header, and a change of one deleted and one added line in two hunks (a move) has none, so
`_mut_fresh` returns 1 ("the one changed line has no `NcM` diff header", :72), which is red. The driver accepts it.

**Checks before the review (the coder's, relayed; the handoff's `cr2/CR2-B-PROGRESS.md`, logs under
`cr2/logs/cr2b/`).** Superseded figures are marked; the final tree's are under "After fix round 2".
- `$SH -n` over the 9 files × b53, b32: 18 of 18 clean. CR-8: 0 for mutations, 0 for correspondence.
- **CR7-2** (the shipped `_mut_fresh`, sourced, over every record expression; both shells): at the base, 25 of 263
  not fresh, the plan's 25 (45, 55, 71, 80, 81, 101, 103, 104, 106–109, 134, 135, 137, 224, 226 `carry`; 89–92, 97,
  323, 329, 382 `twice`); on the slice, 0 of 266 (262 and R7a–R7d; 269 after fix rounds 1–2, 270 after fix round 3).
- **CR7-3:** the 24 re-anchored records' mutants are byte-identical to their old ones on the old targets, needles equal,
  24 of 24, and old 81 == old 80 (both shells).
- **CR7-T:** ship runs, BSD grep, two at a time: base 20 s (b53) and 21 s (b32), slice 24 s and 24 s, so the pass
  costs about 3–4 s a run; CI's `timeout-minutes: 5` is unaffected.
- **Opt-in** (single-record runs, `r3-optin.sh … default`, a fresh `git clone --local` per run with the slice diff):
  72 of 72 as tabled. The 24 re-anchored records × b53, b32: 48 of 48 `0 not killed as named` (rc 1 is the method's
  stubbed generated set, as at CR2-0). R7a–R7d (records.sh 471–474): 8 of 8 killed as named; each with its self-test
  row removed: 8 of 8 `1 not killed as named`, the mutant SURVIVED; with the self-test skipped inside a mutant
  (G7-5): 8 of 8 SURVIVED. **The first launch was aborted:** its R7 list named records.sh 472–475, one line off (475
  is now R7e's comment; it was `MUTANTS` then, which `r3-optin.sh`'s assert does not reject). It was killed, its logs
  moved to `cr2/logs/cr2b/optin-mismapped-aborted/`, and the lane runner now refuses a line with no TAB (a record has
  one); the relaunch used 471–474.
- **CR7-4** (`cr7-edit.py`'s respelling on the slice; function level and two wire runs, BSD grep): rc 1 on both
  shells, 11 reds, all `its anchor is stale`: records 89–97, 120 and 121; 0 twice, carry or count reds, 0 CONTROL
  FAILED.

**CR7-4, a correction of the claim row's expected column (driver decision; the plan stays as accepted, corrections
are recorded here, the G36 rule).** The row expects records 89–92 and 97 to red "(twice / carry)" beside the six the
base reds. On the slice head they red as **stale**, with 93–96, 120 and 121: 11 in all. The re-anchor put them on the
unique `…+/(` context, which the finding's respelling changes, so they no longer match. Under the new rule alone, the
base's records do not red 89–92 and 97 after the respelling (the coder's `cr2-b-cr7f`: the respelling leaves one
`…+/` on the line, so they are position-unique and faithful there). The scenario is closed because the rule forced
their re-anchor at the base, where they were `twice`, not by the rule alone.

**The fresh review: CRITICAL 0 / IMPORTANT 3 / MINOR 3**, each decided by the driver on the fail-safe lens (an exact
check where it is cheap; a declared blind spot is not a fix):
- **I-1**, line- or neighbour-dependent sed forms judged by the in-place pass (above): **fixed** by the exact path, for
  the forms the form parser marks (a range `,`, a numeric or `$` address, any command but `s` `y` `d`). Seven of the
  269 record expressions take it (records.sh 162 ×2, 170, 171: harness; 357, 403: ctlrun; 418, an `h`/`g` pair:
  controls; all `fresh`; measured for this file on b53 and b32; 7 of 270, the same records, after fix round 3).
- **I-2**, `\001` alone misses a class that excludes it but not a letter or digit: **fixed** by the same-class probes.
- **I-3**, the red was spelled in the records' loop and the self-test read only `_mf_v`, so a mutant dropping the red
  survived every check: **fixed** by `_mut_fresh_gate`, which both paths call, the self-test asserting the red through
  it, and record R7e (floor 254 → 255).
- **M-1**, a comment's wording: fixed (the same pass names a pure insertion beside a deletion, the gate in the
  header's DEFINES, and the process count). **M-2**, a `cmp` failure (exit 2) was read as `twice`, a tool failure turned into a verdict: fixed,
  status 1 with the reason (measured: `cmp` shadowed to exit 2 gives rc 1 and "cmp of the output applied twice failed
  (exit 2)", both shells). **M-3**, landing X9 said the self-test reds "a pass that reads differently" under mawk and
  GNU sed, more than a fixed fixture can show: softened to "a pass that reads its fixture differently" (this commit).

**Fix round 1 (coder; `cr2/logs/cr2b/fix/`).** The exact path, the class probes, `_mut_fresh_gate` and R7e
(records.sh:476), M-1 and M-2, and the BLIND paragraph restated by measurement. Function level: the four I-1 probes
and a `$s` form red `carry` on `k=vx`; an in-range control `carry`, a captured-range control `fresh`; `[a-z]`,
`[[:alnum:]]*`, `[0-9]`, `[A-Z]`, `[^ ]` red `carry`; both shells, also with the GNU-like PATH (it swaps grep only:
no GNU sed or mawk here). The function-level pass over every record: 4.4 s → 5.0–5.3 s (the seven exact expressions
0.42 s together; a first draft that sent any `,` to the exact path took 21.5 s and was replaced by the parser).
**What it left unpinned:** no record or self-test row reached the exact path or the class probes (a mutant forcing
the in-place pass, or dropping the substitute, survived).

**Fix round 2 (coder; `cr2/logs/cr2b/fix2/`), driver decision: pin both.** Two self-test rows on the same fixture,
`/^x/,/=/s/^k=.*/k=wx/` carry (laid in place, the first variant still holding `=` closes the range and the later
variants come out unread, `fresh`; only the exact path reds it) and `s/^k=v[a-z]$/k=wx/` carry (`\001` stops the
match; only the `q` probe shows the `x` read through `[a-z]`). The range row needs no second fixture: `k=vx` already
holds the `=` that closes the range on a variant, and the existing rows' verdicts are unchanged by it. Records R7f
(:479, the exact path forced off: `x = 0` before the pass's `END` print) and R7g (:480, the substitute dropped:
`\001` only); floor 255 → 257. Measured before wiring (function level, both shells): unmutated, 7 of 7 rows as
wanted (the range row on the exact path, the class row in place); under R7f only the range row turns `fresh`, under
R7g only the class row; the other six unchanged in each.

**After fix round 2** (the coder's, relayed, logs as named; fix round 3's figures below supersede those it re-measures):
- `$SH -n` 9 files × b53, b32: no failure. CR-8: 0 and 0.
- **Cost:** ship ×4 PASSED, rc 0, 0 CONTROL FAILED, 25 s and 25 s (BSD grep), 29 s and 30 s (GNU-like), against
  24–26 s before the review and 24–26 s after fix round 1: unchanged within noise (`fix2/ship-*.log`).
- **CR7-2:** head 0 of 269 not fresh, the seven rows as wanted; frame 25 of 263, the same record list as before the
  review; b53 = b32 (`fix2/ftest-*.txt`).
- **Opt-in** (`cr2b-lane-fix.sh`, the fix round's slice diff, both shells):

  | run | records (records.sh) | variant | runs | result |
  |---|---|---|---|---|
  | fix round 1 | R7a–R7e (471–474, 476) | plain | 10 | 10 `0 not killed as named` |
  | fix round 1 | R7a–R7d / R7e | its self-test row removed / the red assertion removed (`nored`) | 10 | 10 `1 not killed as named`, `MUTANT 251–255 … SURVIVED` |
  | fix round 1 | R7a–R7e | self-test skipped inside a mutant (G7-5) | 10 | 10 SURVIVED |
  | fix round 2 | R7f, R7g (479, 480) | plain | 4 | 4 `0 not killed as named` |
  | fix round 2 | R7f / R7g | its row removed (`surv-exact` / `surv-class`) | 4 | 4 `1 not killed as named`, `MUTANT 256` / `257 … SURVIVED` |
  | fix round 2 | R7f, R7g | G7-5 | 4 | 4 SURVIVED |
  | fix round 2 | R7a–R7e | plain, re-run | 10 | 10 `0 not killed as named` |

  30 + 22 runs, one log each (`fix/optin/`, `fix2/optin/`). The coder's progress note says 32 for fix round 1; its
  own breakdown (10 + 10 + 10) and the logs give 30. The 24 re-anchored records are not re-run: their mutants and
  targets are unchanged, and inside their mutants only the self-test runs, as at ship.
- **CR7-4 end to end** (`work/cr2-b-cr7f2`, the final diff with `cr7-edit.py`): function level 11 not fresh, the list
  unchanged; wire run rc 1, 11 reds all `its anchor is stale`, 0 twice, count or carry, 0 CONTROL FAILED, both shells.

**The focused re-check of the fix rounds: CRITICAL 0 / IMPORTANT 2 / MINOR 3.** All six earlier findings (I-1–I-3,
M-1–M-3) were verified fixed. Both IMPORTANTs were introduced by the fix rounds themselves; each was decided by the
driver on the same fail-safe lens:
- **I-A**, fix round 2 gave each character a second probe and pinned only it: no row needed `\001`, so a mutant that
  dropped it (the same-class probe alone) survived every check. **Fixed** by a self-test row on the same fixture,
  `s/^x.ab ab$/x ab XY/` carry (the `.` reads the space, which has no same-class probe, so only `\001` shows the read),
  and record R7h (records.sh:482, comment :481); floor 257 → 258.
- **I-B**, fix round 1's form parser read a `[` inside a regex as an ordinary character, so a delimiter inside a
  bracket expression ended the regex early and a line-dependent form could be read as line-local and streamed in
  place: a false green (the re-checker's `/[/s]=]1]/,/d/s/s.x.y./sAxAyZ/` read `fresh` in place, `carry` exactly).
  **Fixed, fail-safe:** in a regex (`/re/` addresses and `s`'s regex; not `s`'s replacement, not `y`) a `[` is read to
  its `]` (a leading `^`, a leading `]` literal, `[:…:]` `[=…=]` `[.….]` skipped); a delimiter inside, or no closing
  `]`, sends the expression to the exact path (seds differ on a delimiter inside a bracket expression).
- **M-A**, R7d's row claimed it SURVIVES with its carry row removed; since fix round 2 three more rows are carry.
  **Restated by measurement:** with the plain carry row alone removed the mutant is KILLED (by the other three), with
  all four removed it SURVIVES; records.sh :469–:470's comment (rewritten in place) and corpus §6.1's R7d row say so.
- **M-B**, the BLIND list was incomplete: a partial range the probe falls outside (`[a-c]`), a quantifier whose match
  length a substitution probe cannot change (`x*`, `x\{1,\}`), the `I` flag and a pure insertion are **added** to the
  comment (:49–:54) and row AF.
- **M-C**, the coder's progress note counted fix round 1's opt-in as 32 runs: **30** (fixed there; this entry already
  said 30).

**The slice's terminator (driver).** Self-introduced findings two rounds running (fix round → re-check) on an
open-ended surface, sed-regex corner cases. The next check is an enumeration attestation of exactly these five fixes
(each applied? measured? does its survive arm prove its subject?), not another adversarial sweep of the heuristic; a
further corner-case class of the probe heuristic goes to BLIND, not to a new round. The probes are a mechanical guard
backstopped by the opt-in run; complete sed-regex semantics is not their goal.

**Fix round 3 (coder; `cr2/logs/cr2b/fix3/`).** The `x.ab` row (the rows' comment :354–:355 now names three
mechanisms, R7f–R7h) and R7h; the bracket-aware parser (`sk(i, d, b)`, +4 code lines and +1 comment line in the form
awk); R7a–R7d's comment restated in place (records.sh :469–:470); BLIND +2 lines; no existing records.sh line moved.
Probes (`fix3/probe-after.txt`, both shells): the re-checker's example `carry` on the exact path (was `fresh` in
place); the new row `carry` in place; the eight rows as wanted; a delimiter inside a bracket expression (`[/]`, `[]/]`,
`:` inside `[[:alpha:]]` under `s:…:`) exact; `[^]a]`, `[[:alpha:]]` with no delimiter inside, a `[` in a replacement,
`\[` and `y` with `[` stay in place. Function level (`fix3/fnlevel.txt`, the gate and the red assertion as shipped,
both shells identical): unmutated, 8 rows red 0; R7a–R7c and R7f–R7h each red exactly their own row and 0 with it
removed; R7d reds the four carry rows, 3 with the plain one removed, 1 with three removed (the new row kept), 0 with
all four removed; R7e reds the 7 rows that are not `fresh` (red:status:message 0:1:1).

**After fix round 3** (the final tree; the coder's, relayed, logs under `fix3/`):
- `$SH -n` 9 files × b53, b32: no failure. CR-8: 0/0/0.
- **Cost:** ship ×4 PASSED, rc 0, 0 CONTROL FAILED, 25 s and 24 s (BSD grep, b53 and b32), 28 s and 27 s (GNU-like):
  24–28 s, unchanged within noise (`fix3/ship-*.log`).
- **CR7-2:** head 0 of 270 not fresh, 8 of 8 rows as wanted; frame 25 of 263, the record list identical to fix round
  2's; b53 = b32 (`fix3/ftest-*.txt`). Exact path 7 of 270, the same records (records.sh 162 ×2, 170, 171, 357, 403,
  418); R7h in place (`fix3/mode-head.*.txt`).
- **Opt-in** (`cr2b-lane-fix.sh` with the variants `surv-dot` and `surv-allcarry` and G7-5's anchor moved to the new
  last row, fix round 3's slice diff, both shells; `fix3/optin/`, one log each):

  | records (records.sh) | variant | runs | result |
  |---|---|---|---|
  | R7a–R7h (471–474, 476, 479, 480, 482) | plain | 16 | 16 `0 not killed as named` |
  | R7a–R7c, R7d, R7e, R7f, R7g, R7h | its row removed (`surv-stale` / `-count` / `-twice`), every carry row removed (`surv-allcarry`), the red assertion removed (`nored`), `surv-exact`, `surv-class`, `surv-dot` | 16 | 16 `1 not killed as named`, `MUTANT 251–258 … SURVIVED` |
  | R7d | the plain carry row alone removed (`surv-carry`) | 2 | 2 `0 not killed as named`: KILLED (the old claim was false) |
  | R7h | G7-5 | 2 | 2 SURVIVED |

  36 runs. Every claim of R7a–R7c and R7e–R7g still holds on the final tree; R7d's holds as restated.
- `wc -l`: correspondence 447, records 484, mutations 606, controls 775.
- Not run here: awk on mawk and sed on GNU (none installed), as before: route (b), landing X9.

**Accepted residues.** A restatement inside the changed span stays blind: on `k=vy` the respelt target of
`s/^k=v[a-z]$/k=wx/` is still `fresh`, because the restated character is inside the span (`vy` → `wx`), the declared
inside-span class of the settled definition (the plan's §3, clause 3 is "outside the changed span"); no design change.
Clause 3 under mawk and GNU sed is not measured here (none installed, none installed for this): route (b), landing
X9, which this commit's sentence now bounds to what the fixture shows. The BLIND comment (:49–:54) and row AF state
the rest: the lines a range's addresses or a hold-space copy read (BS-8), a variable defined elsewhere, a wildcard
inside the span, a class whose characters no probe leaves (`[[:punct:]]`, a space) or a partial range the probe falls
outside (`[a-c]`), a quantifier whose match length a substitution probe cannot change (`x*`, `x\{1,\}`), the `I` flag,
and a pure insertion (the last four since fix round 3, re-check M-B).

**Budgets** (`wc -l`): correspondence 447 (≤ 450) against the plan's ≈ +45: +117 (the gate, the form parser with its
bracket reading and the exact path are the growth past the first draft's +61); records 484 (+14: eight records, six
comment lines; no existing line moved, the 24 re-anchors and line 81 in place); mutations 606 (+1, the header
sentence; the floor 258 at :272); controls 775 (+1). The design memo 758 (≤ 760), the corpus 429, this file under 800.

**Premise error in the driver's brief (the plan's row is right).** The plan's CR2-B row names mutations.sh:260. At the
plan's frame `706049ad` that is `_MUT_RECORDS_MIN=251` (`git show 706049ad:… | sed -n 260p`); CR2-0 moved it to :271
(at `f35bfde3`), and this slice to :272. The brief read :260 at `f35bfde3`, where it is the `done` of the
`_MUT_TARGETS` derivation.

**Docs** (measured for this file). The slice moves lines in two code files: correspondence (base :33–:218 +120, base
:227–:239 +140, base :275 on +117; the rest rewritten) and controls (:119 on, +1); mutations :169–:171 are rewritten
(+1 after) and records.sh moves no line (its additions follow :468). Present-tense cites into those files over the
sweep's file set (`/usr/bin/grep -noE '(controls|correspondence|records|mutations)(\.sh)?`?( lines?)? ?:[0-9]{1,4}'`
and the `line N of` forms) outside this slice's own text are frame- or commit-tagged (corpus :188, :246, :266, :278,
the design memo :582 "at base", census :161–:162, landing :372) or in a closed record (§T.2's correspondence cites are
at `f35bfde3`), so `hist`; records.sh:81's own comment and corpus :368 (records.sh:360) cite lines that did not move.
None needed re-pointing. This slice's own cites into the four files, re-read at the final tree (after fix round 3):
the corpus's 33 CR2-B rows each name a records.sh line whose needle is the row's label (the remove row: a `#` line),
33 of 33, and the CR2-0 row's records.sh:360, 1 of 1; row AF's records.sh:471–474, :476, :479, :480, :482; this
entry's cites (correspondence :33–:42, :33–:54, :43–:48, :49–:54, :55–:125, :72, :73–:102, :105–:116, :118–:122,
:126–:152, :338–:390, :345, :346–:364, :365–:368, :387; controls :102–:107, :119; mutations :169–:171, :272, and the
sweep list below; records :469–:482). The definition is
stated in three clauses at each current-tense site (CR7-1): the correspondence part (:33–:42, :339–:340, :380–:381),
mutations :169–:171 and design memo §4 row AF; records.sh's header does not state the rule. One present-tense
enumeration was incomplete: landing X9 (:398) lists what route (b)'s run is the first mawk evidence for; it now names
the rule's clause 3 under mawk and GNU sed, which the self-test's rows red if a pass reads its fixture differently there.

**`…-cr2-sweep.md` §E.** `idents.tsv` gains `_mut_fresh` (word-bounded), `_mut_fresh_gate` (fix round 1's identifier,
in the slice that adds it), `_mf_v`, `_af_lbl` and the three phrases B2a I-2 re-classifies for CR2-B, "fresh",
"exactly one line" and "anchor" (flag `i`); the header names them.

**Sweep** (measured for this file; §E.sweep.sh and §E.idents.tsv at this commit's text, 30 files, b53 = b32
byte-identical; the handoff's `cr2/logs/cr2-b-sweep/r3/`, after fix round 3, which supersedes `r2/` and the first pass beside them;
the base clone `git clone --local` at `f35bfde3`). Base `f35bfde3` 2703 lines (2672 hits, the same file as `r2/`'s; the
slice's new groups have none there), the tree without this entry 2884 hits. With file names and line numbers set aside,
the base and this tree differ in 33 hits removed and 245 added, each on a line the slice's
diff removed or added (checked against `git diff -U0`, 0 off). B2a I-2, every hit of the three phrase groups, read by
its sentence against the rule as fixed (the gate, the exact path, the class probes):
- `anchor`, 250: 66 rewritten or new (code 32: correspondence 21, records 9, controls 2; docs 34: corpus 32, row
  AF, landing X9), each true at this commit (fix round 3's: R7h's label, its corpus row, R7d's restated row, row AF); 12 code lines unchanged (mutations :135, :168, :184, :218, :253, :284,
  :286, :387, correspondence :11, records :11 true under the new rule; mutations :304 and harness :355 another sense);
  48 unchanged in other docs: 3 current and true (corpus :216–:217, a record whose anchor no longer matches is red
  before `_mut_run`; landing X16 :402, `stalectl` and `xanchor` rc 1 with "its anchor is stale", a message the gate
  keeps: read, not run), 33 `hist` (the corpus's dated rows, :245–:247 and :286 among them, the plan's provenance;
  census :155, :186; the design memo :237–:238; landing's C rows :369, :370, :372; the #519 memo :714, :1386), 12
  another sense (spec anchors in CLAUDE.md, cell-patch re-anchors in landing, a line-anchored regex); 124 in closed
  records, `hist`.
- `fresh`, 107: 46 new (code 36: correspondence 23, records 10, mutations 2, controls 1; docs 10: the corpus's R7 rows,
  row AF, landing X9; fix round 3's: the BLIND comment's third `fresh` line, R7h's label and corpus row, each true); 10 code lines unchanged (mutations :173 true; 9 another sense: the evidence directory made
  afresh, a fresh unborn HEAD, `refresh.py`); 15 in other docs, another sense (a fresh agent, Codex round, clone or
  `mktemp`); 36 in closed records, `hist`.
- `exactly one line`, 8: 1 new (mutations :170, the first clause); corpus :286 (dated, `hist`); 6 in closed
  records, `hist`. No current site states it as the whole definition.
- The identifier groups: `_mut_fresh` 15, `_mut_fresh_gate` 8, `_mf_v` 31, `_af_lbl` 6, all new, each on a line
  that defines, calls or states the current code.
The `perl -0777` wrap backstop (b53 = b32): 7 matches at the base, the six CR2-0 recorded and `exactly one line` at
mutations :169 (the old sentence, split over two lines); 6 on this tree, the same six with mutations :217/:548 now
:218/:549 (`wrong reason`, the header's +1). The old sentence is gone; its successor at :170 is on one line. With this entry the sweep gives 3015 lines; the entry's own 100, in eleven groups (the three phrase groups 59 of
them), are this record's statements, `hist`.

Review: CRITICAL 0 / IMPORTANT 3 / MINOR 3, dispositions above (I-1, I-2, I-3 fixed by fix rounds 1 and 2; M-1, M-2
fixed; M-3 this commit's landing sentence). Focused re-check of the fix rounds: CRITICAL 0 / IMPORTANT 2 / MINOR 3, the
six earlier findings verified fixed, dispositions above (I-A, I-B, M-A, M-B fixed by fix round 3 and this entry; M-C
the handoff's progress note). Attestation (the slice's terminator: these five fixes only, by enumeration): 17 of 17
PASS, both shells, measured on the final tree; its one out-of-scope note (a direct `_mut_fresh` call under a UTF-8
locale fails one controls record's variant sed) does not reach the wire, which exports `LC_ALL=C` before any part
runs. Battery (`BASE_REV=f35bfde3`, the worktree holding the final code; handoff
`runners/logs/battery-{b53,b32}.cr2-b-full.out`): 88 of 88 new == base on b53 and b32, no case edit-failed.

### T.4 CR2-A — the census reads one population per check (CR#4)

**CR2-B after its transplant** (`cdc563d0`, the plan's §11 procedure): ship ×4 ({BSD, GNU-like grep} × {b53, b32})
and `scripts/trip-wires.sh`, all rc 0, PASSED, 0 CONTROL FAILED (the handoff's `cr2/logs/cr2m-ship/cr2-b/`).

This commit (on `cdc563d0`; the plan's §2, its CR2-A row, claim rows CR4-1…CR4-5 and CR4-L). The census fed one scan,
rule 1's expansion scan (`.vlabels`), to three readers whose safe directions are opposite: the orphan check (an
under-count is red, an over-count a miss), the ratchet and the literal rule (an under-count is a miss). Each now reads
the population whose error is red for it. In the correspondence part, `.mvalues` (:270) holds the values of every name
of `$_lbd_names` (the `_*_lbl` tokens of the wire and every part, the record table included, each asserted defined and
non-empty by `_lbd` in controls.sh), by the same indirection as `.vlabels` (:269). The rules 2 and 3 pass (:277–:310)
reads `.vlabels` (ARGV[1], into `.labels`), `.mvalues` (ARGV[2], the literal rule's `isv`), the registry and the controls
file; the needle pass (:317–:330) keeps the orphan check on `.labels` and writes `.bare` from `.mvalues` and `.labels`,
less the needles. Every name the expansion regex can capture is a token of `_lbd`'s regex, so spelled ⊇ expanded, and
an extra spelled token is asserted defined, then counted bare; the one blind spot, a label variable whose name is never
spelled (`printf -v`, `eval`), `_lbd` cannot assert either, and none exists (the plan's instrument, 7 hits at the frame,
each a reader or the word "declared"). The comments whose referent changed state the two populations: the census
heading (:213–:221), rule 1's error statement (:249–:259, now two-sided, with `oracle2.sh`'s figures), the `.bare` line
(:317), the consumes line (:16–:17); controls :75–:77, :98–:100, :145–:153; mutations :15–:18, :23–:25, :56–:61,
:200–:203, :213–:223; records.sh's census block (:330–:332). records.sh's header (:12–:16) states its second rule:
it spells only defined labels, so a record that needs a new label name builds it from its anchor by a sed
backreference. R4a (:486, comment :483–:485) pins the first clause; `_MUT_RECORDS_MIN` is 258 → 259 (mutations :275).
Corpus §6.1 has R4a's row and the totals (165 of this PR, 259 in all; `_mutants | wc -l` 259 on b53 and b32,
258 at `cdc563d0`, measured for this file). The design memo's §4 row C and §10 row 9 state the two populations.

**`…-impl-cr.md` :625–626, corrected** (claim row CR4-3; its body is not edited, the plan's §12). The sentence "every
step only drops text, so the scan can lose a real expansion (its records orphan: a false red) and can never invent
one" is wrong: the scan counts whatever it does not drop, so text it fails to drop is counted. CR2's `oracle2.sh`
(C1's 131 cases plus 264 generated double-quote contexts) measured 18 over-counts in two classes at the frame — a ` #`
inside an earlier double-quoted string on the line of a quoted here-document's operator (12), and an apostrophe inside
an earlier double-quoted string before single-quoted data on the line (6) — and a line of a multi-line single-quoted
string is a third, unmeasured by the oracle. Its :623 and :782 ("0 false green") stay true of C1's 131 cases and false
as a statement of the scan; so do the closed plan's :413 and :733, which carry a dated note under `…-cr-plan.md`'s
Status paragraph (G36's rule). After this commit the orphan check alone reads the scan, and an over-count there
leaves a record non-orphan, its opt-in trial then not killed as named: red, BS-4's class (`…-cr-plan.md` §6).

**The author's checks** (the coder's, on the slice's tree; the handoff's `cr2/logs/cr2a/`):
- `$SH -n` over the 9 files × b53, b32: 18 of 18 clean. CR-8 (8-hex strings in the comments of mutations.sh and the
  correspondence part): 0 and 0.
- CR4-1: `oracle2.sh` on the correspondence part, 395 cases: 315 agree, 58 under, 18 over, 4 syntax-excluded,
  sort-identical to the frame's output (`cr2/logs/oracle2-head.out`), so C1's 131 (114 / 13 / 4 / 0) and the generated
  264 (201 / 45 / 18) split as at the frame and the over-counts are the two classes.
- CR4-4: `pops.sh` over the correspondence part (`cr2a-pops.sh`): expanded 32 = spelled 32 (the frame's 31 and CR2-B's
  `_af_lbl`), so `.bare` stays 18; ship ×4 PASSED.
- The anchor rule (CR2-B's `cr2b-ftest.sh`, b53 and b32): 271 expressions, 0 not fresh.
- Ship ×4 ({BSD, GNU-like grep} × {b53, b32}): PASSED, rc 0, 0 CONTROL FAILED, 26–38 s (the run after the fix below;
  its logs replace the first run's).
- R4a, one-record trials (`cr2a-lane.sh`, `r3-optin.sh … default`, in a fresh `git clone --local` per run): the slice
  as is, `259 entr(ies), 0 not killed as named` on b53 and b32 (rc 1 only from the trial's stubbed generated-set
  floor); the survive arm (the ratchet's and the literal rule's population back to the expansions: `.mvalues` from
  `${_cen_used}`), `MUTANT 259 (every label the run used is named by a mutation record) SURVIVED`, `1 not killed as
  named`, on b53 and b32. A first survive run per shell (spelled `$_cen_used`, see departure (3)) ended rc 1 with no
  mutation-set line; its log was overwritten by the re-run.
- CR4-L (`cr2a-cl.sh`: a ship run with a label inserted after controls.sh's `_cen_lbl=` line, whose value equals a
  `_control` label, `green is reachable`, and whose only use is inside a double-quoted string holding an apostrophe
  pair): with the fix, the line "\"green is reachable\" is a `_*_lbl` value passed to `_control`" present, rc 1, b53 and
  b32; with `.mvalues` from `${_cen_used}`, absent, PASSED rc 0, b53 and b32.
- CR4-2's parts half: 0 hits of the instrument over the parts (2 at `cdc563d0`, correspondence :255 and :256, rule 1's
  old error statement). CR4-5: records.sh spells 10 `_*_lbl` tokens, each defined in controls.sh, the same set as at
  `cdc563d0` (R4a adds none).

**The driver's catch** (diff read): the first cut made records.sh's header line 16 two lines (`@@ -16 +16,2 @@`),
moving every record line +1, which the program forbids, since the documents cite records by their records.sh line
(`…-impl-cr.md` :796, `641239cc`); fixed by folding the rule into line 16 (the header nets 0 lines), so 258 of 258
record lines keep their number and text. The coder's re-measurement after the fix: records.sh `-n` clean on b53 and
b32; `_mutants | wc -l` 259; the anchor rule 0 of 271 not fresh; R4a's opt-in trial killed as named as is and SURVIVED
in its survive arm, both shells; ship ×4 PASSED (the handoff's `cr2/logs/cr2a/ftest2.txt`, `fix-lane{1,2}.out`).

**Departures from the plan, with the driver's disposition:**
1. The mechanism prototype `cr4-proto.py` passed `.mvalues` after the registry, so the rules 2 and 3 pass checked each
   registry row against an `isv` not yet filled: the literal rule saw an empty set. The slice passes `.mvalues` as
   ARGV[2], before the registry. A prototype defect, caught by CR4-L (its fix arm). Accepted.
2. `.bare` is built from `.labels` ∪ `.mvalues`, not the plan's `.mvalues` ∪ the registry's labels. The two sets are
   equal: `.labels` is `.vlabels` ∪ the registry's labels, and `.vlabels` ⊆ `.mvalues` by construction (every expanded
   name is a spelled token, so its value is among the spelled values; CR4-4 measures the name sets equal at this tree).
   Reading `.labels` keeps one file for the orphan check and the ratchet's registry half. Accepted.
3. The survive variant spells `${_cen_used}`, not `$_cen_used`, so that the anchor of the record at records.sh:360
   (`for _cen_v in [$]_cen_used; do`) still matches one line in the variant's tree. Accepted.
4. The correspondence part is at its stop line, 450 of 450 (the plan's §8). **Carried constraint:** a later slice that
   must add a line to it returns to the driver first.
5. The plan's §2.3 item 3 quotes mutations :24–:25 as "`$_lbd_names` … the census reads the values of those expanded
   outside the record table"; that sentence is not in the current text (mutations :23–:25 at `cdc563d0` say only that
   `_mut_run` reads the `_p<x>_lbl` and `_p<x>l_lbl` names among them). The coder added the reader instead: the
   consumes line (:24) now says the correspondence part reads their values for the ratchet and the literal rule.

**Docs** (measured for this file). The slice moves lines in three code files (base numbers): controls :152 on
+1 (:150–:151 rewritten into :150–:152); correspondence :270–:280 +1, :283–:314 +2, :315 on +3 (new lines :270 and
:317; :281–:282 rewritten into :282–:284); mutations :218–:219 +1, :221 on +3. records.sh moves none: its header's
rule is folded into :16, :331–:332 are rewritten in place, and R4a's four lines follow the last record (:483–:486).
Line cites into the four files over the sweep's file set (`/usr/bin/grep -noE '(records|mutations|controls|correspondence)(\.sh)?`?( ?:| line| lines)?
?:?[0-9]+'`), 635 at this tree less this entry: 2 new and correct (row C's and corpus R4a's records.sh:486); 3
unchanged and true at this tree (design memo row AF's R7a–R7h, records.sh:471–474, :476, :479, :480, :482, and its
exact path's list, records.sh 162 ×2, 170, 171, 357, 403, 418, 7 of 271 expressions, re-measured with `mode.sh`,
b53 = b32; records.sh:81's comment, which names the corpus §6.1 remove row for records.sh:81); 630 frame- or
commit-tagged or in a closed record, `hist` (design memo :582 "at base"; census 2, landing 2, corpus 46, the
dated rows and the totals' commit clauses; `…-cr-plan.md` 110, `…-cr-sweep.md` 259, `…-impl-cr.md` 48, §T.0–§T.3 96,
`…-impl.md` 18, `…-pr527-r15.md` 20, provenance 8, reviews 20). `…-cr-plan.md`'s note adds 3 lines above its body, so
its line numbers shift by 3 from :15 on; the note states that its numbers are the approved body's (as at `706049ad`),
and the repo's two line cites into that file (`…-impl-cr.md` :642 to :506, this file :119 to :3) are closed
records, not re-pointed.
CR4-2, over `docs/plans/*k2-fgit*` (less every `…-cr2-*.md`) and the parts: 14 hits at this tree (17 at `cdc563d0`),
0 a current statement of the claim; the closed plan's two (:416, :736, the approved :413, :733) and its note (:17);
the closed record's two (`…-impl-cr.md` :623, :782, corrected above); 9 unrelated (`…-cr-plan.md` :331, :967 and
landing :131, the capability declaration; landing :398, X9; corpus :366, a dated row; `…-impl-cr.md` :608, :614,
:616, the C1 rounds' history; §T.0's finding row, this file :46).

**`…-cr2-sweep.md` §E.** `idents.tsv` gains `.mvalues` (the slice's identifier) and the two phrases B2a I-2
re-classifies for CR2-A, "expand" and "spell" (flag `i`); its other two groups, `.vlabels` and `$_cen_used`, are rows
already (`lbd`, `cen_vars`). The header names them.

**Sweep** (measured for this file; §E.sweep.sh and §E.idents.tsv at this commit's text, 30 files, b53 = b32
byte-identical; the handoff's `cr2/logs/cr2-a-sweep/`; the base clone `git clone --local` at `cdc563d0`). Base
3211 lines (3180 hits; the slice's new groups there: `.mvalues` 0, "expand" 66, "spell" 128), the tree without
this entry 3226 hits. With file names and line numbers set aside, the base and this tree without this entry differ
in 69 hits removed and 115 added, each on a line the slice's diff removed or added (checked against `git diff -U0`,
0 off). B2a I-2, every hit of the plan's four groups and `.mvalues` in this tree without this entry, read by its
sentence against the census as fixed:
- `expand`, 66: 11 new (code 7: controls :99, mutations :18, :60, :203, :223, correspondence :259, records :331; docs 4: row C,
  row 9, R13, the corpus's unrecorded-labels paragraph :419), each true at this commit; 15 code lines unchanged, 5 in
  the census's sense and true of the orphan check's scan (correspondence :218, :249, :254; mutations :216; records
  :375) and 10 another sense (a variable or glob expanded: wire :728, controls :175, ctlrun :20, :56, :178, :224,
  harness :531, :802, records :17, `scripts/trip-wires.sh` :99); 5 in other docs, another sense (design memo :97, :98,
  :185, :612; landing :285); 35 `hist` (the corpus's dated rows :360, :361; closed records 33, the #519 memo's body
  :913 among them).
- `spell`, 148: 25 new (code 18: controls :77, :99, :150, :152, mutations :18, :61, :201, :202, :203, :205 ("spelling",
  another sense, reflowed by the review fixes), :222, correspondence :220, :259, :317, records :16, :331, :484, :485; docs 7: R4a's corpus row, the unrecorded-labels paragraph, `…-cr-plan.md`'s
  note, R13, rows C and 9, and row AF's "respelling", another sense), each true; 40 code lines unchanged, 1 in the
  census's sense and true (mutations :23, `$_lbd_names`, "the names the wire and parts spell") and 39 another sense
  (a text or command spelled or respelled); 13 in other docs, 2 true (design memo :595, `$_lbd_lbl`'s census of the
  names the wire and the parts spell; row L :630) and 11 another sense; 70 `hist` (the corpus's dated rows 12; closed
  records 58, the #519 memo's banner :30 among them, which R13 tracks and CR2-D rewrites).
- `lbd` (`_lbd`, `.vlabels`), 100: 12 new (code 8; docs 4: row C, R13, `…-cr-plan.md`'s note, R4a's corpus row), each
  true; 23 code lines unchanged, each defining or reading `_lbd`, `$_lbd_names`, `$_lbd_files` or `.vlabels` as the
  code does (`.labels` is still `.vlabels` and the registry's labels, correspondence :275); 3 in other docs, true
  (corpus :203, landing :287, design memo :595); 62 `hist`.
- `cen_vars` (`_cen_*`, `$_cen_used` among them), 54: 7 new (code 5; docs 2: row C, R4a's corpus row), each true;
  19 code lines unchanged and true (records :341, the census block's line for the record that empties the `$_cen_used` loop,
  :360, which still dies with its needles orphan: the orphan check reads `.vlabels`); 2 in other docs, true (design memo :516, :623, `$_cen_lbl`);
  26 `hist`.
- `.mvalues`, 7: all new (correspondence :220, :259, :270, :310, :317, :330; R4a's corpus row), each defining,
  reading or stating the code.
The `perl -0777` wrap backstop (b53 = b32): 6 matches at the base and 6 on this tree, the same six with mutations :218/:549
now :219/:552. With this entry the sweep gives 3376 lines; the entry's own 119
hits, in fourteen groups, are this record's statements, `hist`.

**The fresh review** (one reviewer, on the slice's tree after the driver's catch): CRITICAL 0, IMPORTANT 0, MINOR 6,
each fixed, in comments and docs only. The four code files' non-comment lines are unchanged by the fixes (the diff of
their non-`#` lines against the reviewed tree is empty); correspondence, records.sh and mutations each net 0 lines, and
258 of 258 record lines keep their number and text against `cdc563d0`.
1. records.sh :331: "outside this table" now qualifies EXPANDS only; the spelled names (`$_lbd_names`) include the
   record table's ("SPELLS, this table included").
2. mutations :59–:61: the record unit states both populations: a needle must be a registered label or the value of an
   expanded name (the orphan check reads `.labels`), and the ratchet counts the registered labels and the values of the
   spelled names that no record names (the plan's §2.3 item 3).
3. correspondence :257, design memo row C, `…-cr-plan.md` :17: the third over-count class, a line of a multi-line
   single-quoted string, is marked as not measured by `oracle2.sh` (none of its 395 cases has a newline inside single
   quotes: each `${NL}` of `cr2/bin/oracle2.sh` falls outside single quotes). Correspondence nets 0 lines (450 of 450).
4. mutations :202–:203, beside "ADDING A LABEL MEANS ADDING A RECORD": the dead end the two populations create: a label
   rule 1 under-counts (one used only inside a double-quoted message holding an apostrophe) is spelled, so the ratchet
   counts it bare, and is not expanded, so a record for it reds as an orphan; such a label needs its wording changed, or
   the ratchet raised as a visible decision. Latent at this tree (CR4-4: expanded 32 = spelled 32). The next sentence
   (:204–:206) is reflowed from five lines into three, so mutations nets 0 lines.
5. design memo row AF: "7 of the 271 record expressions at the CR2-A slice" (the count's frame).
6. residuals R13: "short of the wire and the record table" was half false. The banner's lines (#519 memo :28–:30) were
   last edited at `173b81dc` (2026-10-15; `git log` of the memo, `git blame -L 28,30`); at that commit controls.sh's
   `_lbd_files` read the parts of `_MUT_PARTS` (`controls harness ctlrun mutations records fixtures mutgen`, records
   among them) and not the wire, which `_lbd_files=("$SELF")` added at `2ac70030` (CR-C1). R13 now says "short of the
   wire" and gives that evidence.
The review's two non-finding notes: (a) the opt-in logs keep R4a's kill count, not its message, so "19 against
18" (corpus R4a row, records.sh:484) is derived, not logged: the survive arm differs from the plain run only in where
`.mvalues` comes from, and R4a's label is no `_control` label, so the literal rule cannot be what killed it; (b)
correspondence.sh's "`.vlabels` feeds the ORPHAN CHECK only" is literally narrow, since `.bare` reads `.labels` (which
holds `.vlabels`) too; the effect is the same because `.vlabels` ⊆ `.mvalues` (the accepted departure 2).
Re-measured after the fixes: `-n` on records.sh, correspondence and mutations × b53, b32: 6 of 6 clean; the anchor rule
(`cr2b-ftest.sh`, b53 and b32): 271 expressions, 0 not fresh (no anchor's uniqueness changed); the doc line cites into
the four files (this entry's and rows C, AF, corpus R4a's), 129 of 129 true: no cited line moved, and of the cited lines whose text the
fixes changed, mutations :59 (the `spell` list) moved to :61 and the comment ranges :56–:60, :200–:201 above became
:56–:61, :200–:203; the rest hold. No wire run.

**Budgets** (`wc -l`): correspondence 450 (its stop line, 450), records 488, mutations 609, controls 776; the design
memo 758 (≤ 760), the corpus 430, `…-cr-plan.md` 977, residuals 141, `…-cr2-sweep.md` 757, this file 742 (< 800).

Review: CRITICAL 0, IMPORTANT 0, MINOR 6, each fixed (above). Battery (`BASE_REV=cdc563d0`, the worktree holding the
final code; handoff `runners/logs/battery-{b53,b32}.cr2-a-full.out`): 88 of 88 new == base on b53 and b32, no case
edit-failed. (Two earlier runs were stopped by the driver before they finished: one on the record-shifting header,
one before the review's comment fixes.)

### T.5 CR2-C1 — the plain-git retry's purge, and the control's stale output (CR#9, CR#14)

**CR2-A after its transplant** (`dd5b86cc`, the plan's §11 procedure): ship ×4 ({BSD, GNU-like grep} × {b53, b32}) and `scripts/trip-wires.sh`, all rc 0, PASSED, 0 CONTROL FAILED (the handoff's `cr2/logs/cr2m-ship/cr2-a/`).

This commit (on `dd5b86cc`; the plan's §4, §5, its CR2-C1 row, claim rows CR9-1, CR14-a/b, CR14-G). **CR#9:** the fsmonitor
control's plain-git retry (controls :713) unset three hand-named variables, so a caller's `GIT_OBJECT_DIRECTORY` or
`GIT_COMMON_DIR` broke the call and the control reported NOT EXERCISED; it now unsets git's own list, `for _v in
$_GIT_LOCAL_VARS; do unset "$_v"; done`, and `env` gives back the one configuration it asks about (reason :710–:712). R9
(records.sh :490, comment :487–:489) pins it; `_MUT_RECORDS_MIN` 259 → 260 (mutations :275); corpus §6.1 has R9's row and
the totals (166 of this PR, 260 in all). **CR#14:** `_ctl_run` removed the timeout marker before the launch but not the
output file, so a launch whose redirect could not open it was judged on the previous control's output; ctlrun :346 now
removes both, fail-closed (`… could not be cleared`, return 1; reason :344–:345). No record (the plan's §5: a read-only file
is the only constructible case, and root writes it). The RETURN CONTRACT (ctlrun :228–:240, rewritten in place, 13 lines)
has a before-launch clause (`_rc`/`_out` untouched, not reset at entry, since the `_cap` check reads "`_rc` is set" as "a
run was made"; the new return is one of its 1s) and an after-launch clause (both assigned); a caller reads them after a 0
return only, the standing checks beside their own red. 259 of 259 record lines keep their number.

**The author's checks** (the coder's, on the slice's tree; re-runnable commands, not kept logs; the PROGRESS file lists them): `-n` on the nine files × b53, b32: clean; the
anchor rule (`cr2b-ftest.sh`, both): 273 expressions, 0 not fresh. R9's opt-in (`r3-optin.sh … default`): 260 entries, 0 not
killed as named, both; the survive arm (the second expression alone): `MUTANT 260 (a caller's fsmonitor hook does not run)
SURVIVED`, 1 not killed, both. CR14-a: the plan's read-only stale file → `CONTROL FAILED (cr2 stale probe): expected exit 1,
got 0` (the probe runs), both; the clearing taken out → no red under its label (it passes unrun), both. CR14-b: a directory
at `$_VFY/.control_out` → `CONTROL FAILED (cr2 stale probe): a previous run's output or timeout marker could not be
cleared`, both. CR14-G (the plan's expression, wire and parts): 19 hits, each read: `_wm` (controls :352), `_cap` (:445,
:451), relcwd (:541, :548–:549), fsmonitor (:706–:708), `_control` (ctlrun :425–:434, after `[ "$_cr" -eq 0 ] || return 1`),
`_ctl_run`'s own (:380–:400), records :286 (a record's text); the returns before `_rc=` (:290, :293, :297, :300, :346) leave
both untouched, those after (:384, :401, :403) have both assigned, none falls between `_rc=` (:374) and `_out=` (:376). Ship
×4: PASSED, rc 0 (31–39 s); the ship with `GIT_OBJECT_DIRECTORY=/nonexistent-k2`, and with `GIT_COMMON_DIR=/nonexistent-k2`:
PASSED, rc 0, b53 and b32. CR9-1: `unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE` in 0 of the parts. CR-8: 0 and 0. The
driver's dispositions, accepted: (a) R9's first expression writes three separate `unset` statements, not one `unset` of
three names, so CR9-1 stays 0 with records.sh in its population; (b) the `_wm` check's `hang` row cannot tell the new
pre-launch return 1 from a timeout by status; its message assertion (controls :357) reds in every case (the red says `could
not be cleared`), its marker assertion (:356) when the output file is what could not be cleared.

**Docs.** Line cites into the moved or rewritten lines (controls :710 on, +3; ctlrun :228–:240 in place, :344 on, +2;
records :487 on, +4), over the sweep's file set (838 cites the instrument attributes to the three parts): 45 there, 18 true
at this tree (this entry's 17, corpus R9's :490), 5 misattributed (not cites into the parts), 22 `hist` (`…-cr-plan.md` 4,
`…-cr-sweep.md` 16, `…-impl-cr2.md` :170, the design memo :582 "at base"); 0 re-pointed. The sweep (b53 = b32), with this
slice's groups (`rc_out`, `control_out`, `control_timeout`, `retcontract`; `…-cr2-sweep.md` §E): base 3396 hits, this tree
before the entry 3403 (final minus 12, no log of its own; 13 removed, 20 added, each on a diff line). B2a I-2, every hit re-classified: `rc_out` 20 (the CR14-G
19; `…-impl-cr.md` :104, `hist`); `control_out` 13 (ctlrun :282, :307, :346, :352, :376, code; census :192, landing :372,
true; `…-impl-cr2.md` :51, the finding; `…-impl.md` :439, `…-pr527-r15.md` :144, :182, :429, :614, `hist`);
`control_timeout` 9 (controls :356, :369, ctlrun :282, :346, :370, :380, records :286, code; `…-impl.md` :439–:440, `hist`);
`retcontract` 5 (ctlrun :228, new; `…-cr-sweep.md` :273–:275, `…-impl.md` :502, `hist`). Wrap backstop: 7 at the base, the
same 7 here (the new group's `…-cr-plan.md` :283, `hist`). With this entry: 3415 (its own 12, in nine groups, `hist`).
**Budgets** (`wc -l`): ctlrun 438 (≤ 439, leaving CR2-C2 exactly +1), controls 779, records 492, mutations 609,
correspondence 450 (untouched); corpus 431, `…-cr2-sweep.md` 761, this file 799 (< 800).

Review: CRITICAL 0 / IMPORTANT 1 / MINOR 3, each fixed in place (net 0 lines). Battery (`BASE_REV=dd5b86cc`): 88/88 new == base, b53, b32.
