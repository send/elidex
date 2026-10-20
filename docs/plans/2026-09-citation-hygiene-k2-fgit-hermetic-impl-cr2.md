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
