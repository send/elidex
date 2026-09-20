# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's task, user-approved 2026-08-02.
Edge-dense ⇒ every PR under this umbrella goes through `/elidex-plan-review` before
implementation, and external review runs `/external-converge` from round 1.

All premises verified against `154bac3f`; every spec quote below was resolved directly with
`.claude/tools/webref`, not carried from a reviewer or from a code comment.

⚠ **Every `file:line` in this memo is a `154bac3f` coordinate and is evidence, not an
instruction.** Two of the program's **three** prereq PRs move code before PR-1a (the third — the
predicate PR, §9 — establishes a predicate rather than relocating code; its touch set is its own
plan-review's to determine, so this note makes no claim about it): the seam-3 split relocates a
block of
`inline/mod.rs`, and the dead-arm deletion removes a surface spanning **both** `pack/mod.rs` and
`inline/mod.rs` (§8 names it; it is not one contiguous range). The memo does **not** re-anchor
after each: its coordinates exist to prove claims about the code as it stands today. **Each PR's
own plan-memo re-anchors against its actual base**, and §8's DoDs name behaviours and call sites
wherever they can — a DoD that still carries a coordinate carries a `154bac3f` one and inherits
this note. ⚠ Both `elidex-layout-block` prereqs have since landed — #508 (`7e256029`) and #511
(`22de3078`) — so the ranges named above no longer exist in those files; §5.2 and §8 carry the
landed state, and each later PR re-anchors against its own base (which includes `22de3078`).

**Citation convention**: every *spec* section number is written with its module
(`css-inline-3 §2.3`, `CSS 2 §9.4.2`). A bare `§N` is always this memo's own section. **Three**
carve-outs, each stated **by its property** rather than by the sites it currently reaches, so the
next instance of the property is exempt by the rule and not by a list that has to be extended
([[feedback_enumerated-exemptions-leave-the-next-class-authoritative]]):

1. the convention does not reach inside a **quotation** — a withdrawn drafting, a code comment,
   a spec heading title or a command's own output is reproduced as written;
2. in an **enumeration, slash-list or arrow pair whose own clause names the module**, the items
   inherit that module rather than repeating it (the seven CSS 2 section↔title pairs this front
   matter lists below, §9's `CSS 2 cites are …` list,
   `css-box-3 §3.1/§4.1`, `css-writing-modes-4 §6.2/§6.4`, and the parenthetical pairs that
   follow a `webref heading <module> <n>` invocation);
3. a bare number naming a **plan-memo section requirement** stated by the tooling — the skill's
   `§2` coupled-invariant hard-gate (`.claude/skills/elidex-plan-review/SKILL.md:87`, "edge-dense
   plan §2 missing coupled-invariant enumeration") denotes *a plan-memo's* §2, which for this memo
   is its own, so the convention holds rather than leaking to a third document (round 26, gate 2
   read it as a third document's section; the skill's own wording refutes that);
4. **use rather than mention**: a bare token written in backticks as the *object* of a sentence
   about bare tokens — this front matter's own `§8.3` / `§8.3.1` / `§9.2.1.1` / `§10.8.1` / `§5.5`,
   §9's `` `§8.3.1` entered with rev 33's §1.2 edit `` and `` a bare `§9.2.1.1` sat one clause
   away `` — cites nothing and so qualifies nothing. ⚠ **Added in rev 34** (round 26, Axis 4):
   rev 34's own sweep sentence introduced **seven** such mentions and rev 33 one, all eight
   outside the two carve-outs that then existed — a rule broken by the edit that installs it,
   which is the class [[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]] names.
   Measured with a `(?![0-9.])` lookahead, without which `§8.3.1` counts as a `§8.3` hit.

⚠ **Everywhere else the memo was swept in rev 34** (round 26, Axis 4): rev 33 fixed one bare instance (§9's
`§9.2.1.1`) and **added eleven** — counting bare `§8.3` / `§8.3.1` / `§10.8.1` across the whole
file, rev 32 (`613470f2`) carried 1 / 0 / 1 and rev 33 (`255e14a0`) 7 / 2 / 4, the growth landing
in §1.2 and its restatement in §6 cell 5 (of which one `§8.3` sits inside a quotation and stays
as written). Older bare sites the same sweep reached: §6 cell 15's `§5.5`, plus the Terminator
ledger, §1.3, §3, §5.1, §5.2, §5.3, §6 cell 12e and §9. ⚠ **That sweep was keyed on the three
CSS 2 tokens it had just counted and therefore stopped at the tokens whose number does not collide
with a section this memo has** (round 26, Axes 4/5 — the same defect class again, a detector
scoped by the symptom's vocabulary). Re-run over **every** `§<digits>` in the file, classified
against the four carve-outs above, it reaches **fifteen** further sites where a bare number
reads as this memo's own section and means a spec's: `§5.4` for css-break-3 §5.4 at §3's
css-break-3 row and in §5.3's splits paragraph; `§5.x` for css-break-3's §5 anchors in the same §3
row; `§5` for cssom-view-1 §5 in §3.1 and in §5.2's `InlineClientRects` row, and `§6` for
cssom-view-1 §6 one clause later in that same row; `§6 → §7` for cssom-view-1's two
interfaces at the two places in §9 **and the one in §3.1** that name the `layout_query.rs:355`
outlier (⚠ the §3.1 occurrence was missed by the rev-34 sweep, which qualified the §9 pair of the
same outlier while editing the §3.1 bullet nine lines above it — round 26, gate 2); `§1` and `§2` for
css-content-3's in §3's css-content-3 row, twice in §8's requirement-7 paragraph, and in §9's
`pseudo.rs:45` hand-off; `§3.1` for resize-observer-1 §3.1 in §7's ResizeObserver paragraph; and
`§16.6.1` for CSS 2 §16.6.1 in §9's cite-list paragraph — the last being a bare *use* the rev-34
sweep left standing one sentence after the carved list it belongs to. All fifteen are qualified
here. ⚠ **The method is stated because the figure is only as good as it**: enumerate every
`§<digits>` token in the file (`grep -o '§[0-9][0-9.]*' <memo> | wc -l`), drop those whose
immediately preceding words are a module label, and triage the rest by hand — first every token
whose number is **not** a section this memo has, which cannot be a self-reference at all, then
every token whose number **is**, reading the passage rather than a window. The tokens the
convention leaves are then self-references by inspection, not by the filter's silence.
The memo has **24** sections (`grep -cE '^#{2,3} §' <memo>`), and the numbers the swept sites
collide with are the seven of them a bare spec token actually lands on — §1, §2, §3.1, §5, §5.4,
§6, §7. ⚠ An earlier drafting listed nine, adding §8 and §10, which no swept site uses, and
omitting §3, §4, §9 and every subsection but §3.1 from the memo's own set — it matched neither
the sections the memo has nor the numbers the sweep found (round 26, gate 2). `§16.6.1`, also
swept, collides with nothing: it is a bare *use*, not a collision. CSS 2 is
verifiable with webref under the shortname **`CSS2`** (`webref heading CSS2 8.3`); the **seven** CSS 2
section↔title pairs this memo carries were checked that way — §8.3, §8.3.1, §9.2.1.1, §9.2.2.1,
§9.4.2, §9.4.3, §10.8.1. ⚠ **Seven, not the six an earlier drafting counted, which was itself a
correction of a five** (round 26, Axis 4): rev 33's own §1.2 edit added the CSS 2 §8.3.1
*Collapsing margins* pair and the count was not re-derived. §9's CSS 2 cite list runs to **eight**
sections because it counts every CSS 2 section this memo *cites*, produced by its own grep; the
eighth, CSS 2 §16.6.1, is cited in §1.6 without a title, so it carries no section↔title pair to
check — the two figures count different things and each is stated where it is produced.

## Terminator

⚠ **This program ran nineteen plan-review rounds with a reason-to-continue and no stopping rule.**
That is the mechanism that produced this length ([[feedback_loop-needs-a-terminator-not-a-reason-to-continue]]).
The rule, from here:

> **TERMINAL** = two consecutive rounds in which **no finding changes a §5.1 M-row *Decision* cell
> or a §6 cell**. At TERMINAL the plan is approved and implementation starts, whatever the
> IMP/MIN count stands at.

## ⚠⚠ DESIGN FREEZE (2026-09-20, rev 35) — the rule above is **RETIRED, not satisfied**

**§5.1's eight mechanism *Decision* columns and §6's 49 cells are frozen at rev 34 (`9a17506d`).
Implementation starts.** The ledger below is kept as the record of how the loop ran; it no longer
gates anything.

**Why the rule was retired — three measurements, not fatigue.**

1. **The loop was generating its own resets.** Classified by origin, every counter reset after
   round 21 (which reached TERMINAL legitimately, and was reset by an *external* reviewer's three
   genuine findings) was either noise or a defect the previous revision's own fix had introduced:
   round 23 = two **wording** fixes; round 24 = one **`file:line`** fix inside text the previous
   revision had just written; round 25 = a dead branch **introduced by rev 32**; round 26 = an
   equality guard against an extended ordering **introduced by rev 33, the fix for round 25**;
   rev 34's gate = a smaller defect **introduced by rev 34**. Three consecutive self-introduced
   resets. Nothing made the counter likely to reach two.
2. **The rule keyed on a surface that grew every round.** `wc -m`: 152,152 at round 18 → 199,416
   → 249,144 → 279,186 → 317,266 → **363,770** at rev 34, the last two increments +38k and +46.5k
   — accelerating. Inline `⚠` markers 101 → 376; `An earlier drafting` markers 2 → 40. Requiring
   *zero* §6-cell-changing findings from a 49-cell matrix that gains ~40k characters of argued
   prose per round is a fixed-point search over an input growing ~15% per iteration.
3. **⚠ The decisive ground is detector mismatch, not cost.** Rounds 25 and 26 both found
   *code-level predicate* defects — a dead branch, and an equality test against an extended
   ordering. `cargo test` catches that class at implementation: this crate already carries five
   multi-line assertions
   (`grep -rnE 'lines\.len\(\), *[2-9]|line_count, *[2-9]' crates/layout/elidex-layout-block/src`
   → `inline/tests/inline_flow/{persist,vertical,transform,justify}.rs`), and a guard that never
   fires turns all of them red on the first run. Round 26's own Axis 2 said so and offered to
   downgrade its finding to IMP on that ground. Five agents over 363k characters of prose were
   pointed at the defects the compiler and the existing suite find in seconds.

**What is frozen**: the **Decision** column of §5.1's eight rows, and the **markup + expected
behaviour** of §6's 49 cells — what an implementer builds from. **Not frozen** (still correctable
without reopening anything): Grounds columns, citations, coordinates, §7–§10, and the ledgers.

**Residual risk and how it is discharged**: cross-mechanism code-level consistency — one mechanism
introducing a value, state or ordering that another's predicate was written without. It is
discharged by the **per-PR test suites**, not by further review of this document; the five
multi-line assertions above are the standing tripwire for the round-26 class specifically.

**Attestation** — a single narrow pass over §5.1 + §6 only (2026-09-20), asking one question: do
the eight Decisions contradict each other or any cell? **Result: clean.** Each mechanism's
introduced values and their readers:

| row | introduces / changes | checked against | verdict |
|---|---|---|---|
| M1 | the two marker items; the emit predicate; the payload (entity, three `EdgeSizes`, `WritingModeContext`, five font fields) | M3, M4, M5, M6, M7, M8; cells 1, 2, 5, 6b, 6c, 6d, 6e, 6f, 6g, 6h, 12b, 12c, 12e, 14, 14b, 20, 21, 25 | consistent |
| M2 | nothing new (a transparent `Placeholder`-shaped arm) | cell 9 | consistent |
| M3 | `LineOccupancy`; `note_line_occupancy`; guard `≥ Content`; `finish()` `> Empty`; the reset; `hang: Option<f32>` | M4, M5, M6, M7; cells 3b, 14b, 15, 15b, 15d, 16, 16b | consistent |
| M4 | the open-box stack; unconditional `End` push; flush-time partial emit and rebase | cells 6, 10b, 13, 14c, 15c, 17, 17b, 17c, 17d, 17f, 21 | consistent |
| M5 | `has_inline_axis_edge`; the PR-1d substitution; the `:200` gate escape | M3's hang and shaping break; cells 3b, 5, 6, 12d, 14/14b, 16, 21, 24, 24b | consistent |
| M6 | the marker's `block_advance` | M3's `max`; cells 18, 23 | consistent |
| M7 | the `RenderedText` rung; the tentative baseline | M3's guard, `finish()` and reset; cells 22, 24 | consistent |
| M8 | the max-content edge sum | cell 25 | consistent |

Reader sets were verified by enumeration rather than asserted (`git grep` at `22de3078`):
`on_line` has exactly two readers (`pack/mod.rs:658`, `:756`); `any_rendered_content` exactly one
(`:211`); `flush_line` exactly three call sites (`:659`, `:751`, `:757`); `place_item` exactly two
(`:559`, `:620`).

⚠ **Coverage limit, stated rather than left implicit**: cells **7, 8, 10, 11** carry no expected
value of their own — they are PR-1a *characterization* cells and the block header supplies it
("assert today's behaviour"), so they are complete by that header. Cell **19** was the one real
gap the pass's disclosure exposed: a **PR-1d** cell with no assertion, now filled, and its
`<pre>\n</pre>` attribution to clause 5 corrected to clause 2 (the engine never reaches
`force_break` for it).

**Discipline from here.**

1. **A code-level predicate defect does not get a prose rewrite** — it gets **one required-test
   line added to the §6 cell that owns the behaviour**. Rewriting a mechanism paragraph adds
   review surface the next reader must audit; a test line is checked by the compiler. Same rule
   the citation-hygiene lane reached: *a plan memo carries no measurements and no self-measuring
   apparatus; measurement lives in tests and CI.*
2. **No new inline `⚠ An earlier drafting …` blocks.** A rejected position gets one line in the
   acceptance ledger and at most a pointer from the cell. Existing markers stay — deleting them
   would reopen what they closed — but the accretion stops here.
3. **No further whole-umbrella review.** Each PR gets a narrow plan-review of its own slice, the
   normal pre-push gate, and external review.
4. **The approval PR [#515](https://github.com/send/elidex/pull/515) is unblocked by this
   declaration**, not by a renewed TERMINAL.

Grounds, measured rather than asserted (`git show <rev>:<memo>`, section extents by the same
`^## §N\.` split `plan-xcheck.py` uses):

| § | round 18 (`0d4d474c`) | rev 23 (`5167ca3f`) | Δ chars |
|---|---|---|---|
| §1 the rules (anchored to spec) | 6,505 | 6,505 | **+0** |
| §2 coupled invariants | 3,326 | 3,326 | **+0** |
| §4 verified current state (anchored to code) | 3,035 | 3,038 | **+3** |
| §9 out of scope | 20,622 | 37,815 | **+17,193** |
| whole memo | 152,152 | 199,416 | +47,264 |

⚠ **The left column is labelled by its commit, and it is *not* rev 18** (round 26, Axis 5). It used
to read "rev 18", and the figures do not reproduce against rev 18. Both readings settle it, and
**each states its convention on the spot rather than claiming to need none**. The **whole-memo**
figure is `git show <rev>:<memo> | LC_ALL=en_US.UTF-8 wc -m` — *characters*, the convention that
matters here, since `wc -c` gives 153,905 bytes where `wc -m` gives 152,152 characters at
`0d4d474c`. It prints **120,651** at
rev 18 (`698f4947`) and **120,709** at its dry-run fix (`a7b6cb8f`), against the column's
**152,152** — which `0d4d474c` reproduces exactly, as `5167ca3f` reproduces the right column's
**199,416**. The **per-section** figures reproduce exactly too, and all four of them:
`plan-xcheck.py`'s own `sect()` — the split the table header already cites — returns
6,505 / 3,326 / 3,035 / 20,622 for §1 / §2 / §4 / §9 at `0d4d474c` and
6,505 / 3,326 / 3,038 / 37,815 at `5167ca3f`, matching both columns cell for cell.
⚠ **Rev 34 conceded that they do not** — "their extents depend on whether the split's trailing
newline is counted" — and the concession is withdrawn (round 26, Axis 5, Gate B): there is no
open convention, because `sect()` slices `s[m.start() : m.end() + nxt.start()]` where `nxt` is
matched in `s[m.end():]`, so the trailing newline **is** included, by the one function the table
already declares as its basis. A figure produced by a named command does not inherit an ambiguity
the command does not have.
`0d4d474c` is titled `round 18 — withdraw three claims measurement refutes` and lands
*after* rev 21 (`198ea823`): a **round** number, not a revision number, and by then the two
numberings had parted. The window the table spans is therefore `0d4d474c`→`5167ca3f`, **four**
revisions (`git log --oneline 0d4d474c..5167ca3f -- <memo>` → rev 22, 22b, 22c, 23).

Of the two sections anchored **outside** the memo, only the spec-anchored one (§1) is byte-frozen
across that window; the code-anchored one (§4) moved **+3**, as its own row says — ⚠ an earlier
drafting called both frozen, a claim the adjacent table refutes (round 26, Axis 5). The memo
meanwhile grew by a third, and the largest single recipient is the section describing what the
program does *not* do. ⚠ **But the design was not static in that window, and a reviewer's claim that it was
is refuted by the same command**: M1's Decision cell grew 4,755 → 7,302 and M4's 7,261 → 13,650 —
the predicate widening and the carrier withdrawal, which is exactly where rounds 16–19's CRITs were.
**Six** of eight M-rows — M2, M3, M5, M6, **M7** and M8 — carry a byte-identical *Decision* column
across all five revisions (`0d4d474c`, `ee6b017d`, `2501c979`, `47a423b2`, `5167ca3f`), measured on
the basis the ledger declares below: split each row on unescaped `|`, take the Decision cell,
compare. ⚠ **Five until rev 34, because the count was taken on the whole row rather than on the
column the rule keys on** (round 26, Axis 5, Gate B), and **M7 is exactly where the two readings
part**: its Decision is 664 characters at all five revisions while the whole row goes
1,431 → 2,223 at `5167ca3f` — a Grounds-only change, the distinction the round-19 ledger row
already draws. On the whole-row reading the figure is five (M2, M3, M5, M6, M8); the rule's
basis is the Decision column, so the figure is six. M1 and M4 change on both readings. ⚠ Beware
the unescaped `|` when re-measuring: at `47a423b2` and `5167ca3f` M1's Decision cell contains a
literal pipe inside `` `grep -rnE '"br"|BrMarker'` `` and splits into two columns, so a naive
4-column split reads its Decision as 2,475 rather than 7,274 at `47a423b2`, and 2,503 rather
than 7,302 at `5167ca3f` — stripped, the basis every other figure in this paragraph uses (⚠ an
earlier drafting gave the single pair "2,504 rather than 7,302" for both revisions, which differ,
and took the 2,504 from the *raw* cell; round 26, gate 2). M1 escapes the pipe as `\|` from
rev 34 (a table cell must; §9's copy of the same command is prose and stays unescaped), but the
historical revisions do not. So the loop
is **neither** pure prose churn **nor** open-ended design drift; the stopping rule keys on the one
signal that separates them.

**Ledger** (a round counts if any applied finding changed an M-row Decision or a §6 cell).
Conventions, stated once (round 20): a **row is a five-axis round, or an external-reviewer pass on
the approval PR** (the front matter's `/external-converge` rule makes those passes plan inputs —
the Codex row below); a clean external pass is recorded as a dash and advances nothing — only
five-axis rounds count toward the two; a finding-bearing one resets, as the Codex row shows; a
Step 4.5 gate on an author revision is *not* a round and **folds into that revision's own round
row** — the row already names the revision and the surfaces it touched, so a gate's findings are
counted there and get no row of their own (⚠ **rewritten in rev 33** — round 25, Axis 5: the rule
used to read "recorded only when it touches an M-row Decision or a §6 cell", and four gates —
rev 28's, rev 30's, rev 31's and rev 32's — ran with no row of their own. ⚠ **An earlier drafting
of this parenthesis added that they "each touched both"; the rev-33 gate re-measured it and the
measurement refutes it.** Basis, per revision: extract every §5.1 M-row's *Decision* column and the
whole `## §6` section from `git show <rev>:<memo>` and diff against the predecessor. Only **rev
28's** gate is separably measurable — it is its own commit pair, `df91d40b`→`c921c86b` — and it did
touch both (M1's Decision; §6). Revs 30–32 fold the gate and the round into **one** commit each, so
attributing a revision's Decision changes to *its gate* rather than to the round double-counts the
round's own findings; what those three commits show is `c921c86b`→`64347064` M1 + §6,
`64347064`→`cc145374` **no M-row Decision at all** + §6, and `cc145374`→`613470f2` M1/M3/M4/M5/M7 +
§6. Rev 31's gate therefore cannot have touched an M-row Decision, because its whole revision did
not — the ledger's **round 23** row below says the same ("no (byte-identical)"). The measured count
is **one** gate demonstrably touching both, not four. That does not restore the old rule: four
gates still ran with no row, and a rule four gates break is a practice; the practice is the one
written above. The two rows that predate the change
— **19-gate** and **rev 25** — are kept as historical records under the superseded convention and
are not evidence of the current one); an author revision that applies landings or bookkeeping (rev 25) is measured — `git
show <rev>:<memo>` extracts of `### §5.1` and `## §6` diffed — and recorded as a dash if
byte-identical. **Landing state is written in §5.2, never into a Decision cell**: a Decision cell
that describes a since-landed prereq in the present tense is *frozen*, not stale, and §5.2 carries
the state — otherwise every landing would cost a round.

| round | M-row Decision touched? | §6 cell touched? | consecutive clean |
|---|---|---|---|
| 19 | no (M7 *grounds* only) | **yes** (cell 24's disposition) | 0 |
| 19-gate (Step 4.5 on rev 23) — *historical row, superseded convention: a gate now folds into its revision's round row* | no | **yes** (6c fixture, 6f, 24) | 0 |
| rev 25 (landings + bookkeeping; four Step 4.5 gates; not a round) — *historical row, same* | no (byte-identical) | no (byte-identical) | — |
| 20 (0 CRIT / 8 IMP / 15 MIN; all applied in rev 26 — routing/ordering, §5.2/§8/§9/§10 only) | no (byte-identical) | no (byte-identical) | **1** |
| 21 (0 CRIT / 2 IMP / 12 MIN; all applied in rev 27 — §3/§3.1/§5.2/§5.3/§5.4 index/§8/§9/§10 + memory) | no (byte-identical) | no (byte-identical) | **2 → TERMINAL** |
| Codex on #515 (external-reviewer pass on the approval PR, same day; applied in rev 28) | **yes** (M1: pseudo routing; the non-zero-edge conjunct restated as a slice boundary) | **yes** (cell 15 rewritten; cell 6g added) | **0** |
| 22 (0 CRIT / 10 IMP / 13 MIN; applied in rev 30) | **yes** (M1: emission independent of the `Text` push; §8 requirement 7's domain) | **yes** (cell 15 rewritten again; 6g's fixture; cells 6h and 15c added) | **0** |
| 23 (0 CRIT / 3 IMP / 11 MIN; applied in rev 31 — §3/§5.3/§8/§9/§10 + memory, and two wording fixes inside cell 15) | no (byte-identical) | **yes** (cell 15: the false "today" baseline; the css-text-3 §5 vs css-text-3 §5.5 attribution) | **0** |
| 24 (0 CRIT / 2 IMP / 9 MIN, **plus the two exhaustive audits dispatched in the same cycle** — every §6 cell and every §5.1 M-row — which returned **23** further findings; all applied in rev 32, the sections measured by `git diff cc145374 -- <memo>`: §2, §3, §5.1, §5.2, §5.3, §6, §7, §8, §9, §10 and the Terminator prose below; memory is updated separately, outside the repo) | **yes** (M1, M3, M4, M5, M7 — *not* M2, whose **Decision** column is byte-identical between `cc145374` and `613470f2`; only its Grounds changed, which the ledger already distinguishes the way the round-19 row does. An earlier drafting wrote the range "M1–M5, M7", which over-enumerates by one — round 25, Axis 5) | **yes** (11 cells, named: 6, 6c, 6d, 6h, 9, 12e, 14c, 15, 17d, 17f, 24 — and two non-cell notes, the test-placement paragraph and the adjacent-soft-wrap parenthetical after 15) | **0** |
| 25 (12 IMP / 27 MIN, the count rev 33's own commit title records; applied in rev 33) | **yes** (M1, M3, M7 — *not* M2, M4, M5, M6, M8, whose **Decision** columns are byte-identical between `613470f2` and `255e14a0`) | **yes** (5, 6b, 6c, 22, 24 — 48 cells in both revisions, five of them changed) | **0** |
| 26 (**1 CRIT** / 10 IMP / 11 MIN, plus two sites the author's own sweep found and the two Step 4.5 gates on rev 34 itself — 2 IMP + 5 MIN and 2 IMP + 6 MIN — which fold into this row by the convention above; all applied in rev 34 except one gate MIN, which measurement refuted: §6's cell order is by owning PR, not by id, so 15d is not "out of sequence" beside 15b, and moving it after 15c would move a PR-1b cell into the PR-1c block — 15d records the refutation) | **yes** (M1, M3, M4, M6, M7 — *not* M2, M5, M8, whose **Decision** columns are byte-identical between `255e14a0` and rev 34. ⚠ **M4 joined on the gates' pass**: §2's `1 × 4 (fallback-only half)` row made M4's "two of the nine pairs" enumeration a count of ten, and the summary sentence M4 quotes from §2 was re-derived with it) | **yes** (11: 5, 6c, 6d, 12e, 15, 15b, 15d, 16, 17f, 22, 24 — **15d is new**, so the matrix goes 48 → 49) | **0** |

**TERMINAL reached at round 21 (2026-09-07) was reset the same day.** The approval PR #515
drew three real P2 findings from the external reviewer — the model diversity the front matter's
`/external-converge` rule exists for — all three of which change M-row / §6 content (M1 now routes
decorated pseudo-elements and restates the non-zero-edge conjunct as a slice boundary; cell 15 no longer pins a css-text-3 §5.5
divergence; cell 6g is new), so by the rule above the count is **0** again and two consecutive
clean rounds are required before approval — round 22 was **not** one (its row above), and neither
was round 23: two of its findings were wording inside cell 15 (a "today" baseline that described a
`b` run today's packer never produces, and a css-text-3 §5 rule attributed to css-text-3 §5.5), and applying them is a
§6-cell change by the conventions above. ⚠ An earlier drafting of rev 31 *carried* both to the
landing record to hold the count at 1 — no convention exempts wording, and a real finding held
back to protect a counter is the "diminishing-returns bucket" the review discipline forbids; a
gate on rev 31 refused it. Nor was round 24: the cell-15 text rev 31 added cited `collect.rs:314`
in no frame (`:310` at `154bac3f`, `:315` at `22de3078`), so round 24's §6 **finding** was once
again a coordinate inside text the previous revision had just written — as round 23's had been.
*That recurrence* is why rev 32 audited **every** §6 cell and **every** §5.1 M-row rather than only
its delta, and the audit returned **23** further findings — which is the evidence that one defect
per round was a sampling artefact of reviewing the delta, not a defect count. They are applied
here rather than left for round 25 because this round's count is already 0: a §6 defect fixed now
costs nothing and one fixed later costs a round. ⚠ Two earlier draftings of this sentence, both
refuted by the ledger row above: "the third … whose **only** §6 touch" (a universal — round 22 also
touched §6, by *adding* cells 6h and 15c, a different class), and "the **second** consecutive round
whose §6 touch was **confined to** text the previous revision had just written" — the *finding* was
so confined, but the round's applied §6 touch is the 11 cells the row now enumerates (round-24
gate). **The deciding window is whatever the ledger's last row leaves, and it is read off the
ledger rather than restated as a fixed pair.** Its last row is round 26, `consecutive clean` **0**,
so the next two five-axis rounds — **27 and 28** — decide. ⚠ **"Rounds 25–26" stood here until
rev 34** (round 26, Axis 3): it was written in rev 32, *before* round 25 ran, and round 25 then
reset the count — a window stated as a constant is a claim that goes stale on the very next round
it describes, so the sentence now names the ledger as its source. ⚠ **And the ledger had no
round-25 row at all**, though rev 33's commit title is `rev 33 — round 25 (12 IMP / 27 MIN)` and
every prior revision added its round's row; both it and round 26's are added in rev 34, each
measured against its own predecessor by the Basis above. #515 stays a **draft** until then and is
reviewed under `/external-converge` (rev 27 ran it single-pass — wrong, corrected). Later
revisions after a renewed TERMINAL are landing records, not design rounds.

**Review history — including which revision decided what, and why each earlier reading failed —
lives in `project_line-box-decorated-inline-content.md`, not here.** A past-tense ledger restates
the normative decisions and then drifts from them. What survives in this file is the *ground* for
each decision, stated affirmatively, plus an explicit refutation of the competing readings, so a
rejected one is closed on its merits rather than by precedent.

---

## §1. The rules, from the current module

### §1.1 The inline layout model — `css-inline-3` §2

`body css-inline-3 model`:

> The block container also generates a **root inline box**, which is an anonymous inline box
> that holds all of its inline-level contents. … The root inline box inherits from its parent
> block container, but is otherwise unstyleable.

> **Inline-axis** margins, borders, and padding are respected between inline-level boxes (and
> their margins do not collapse).

Two things follow immediately:

* The edges that take space on a line are the **inline-axis** ones. CSS 2 §9.4.2's
  "Horizontal margins, borders, and padding are respected" is the same rule written before
  writing modes existed — not a physical-axis statement, and not a physical→logical question.
* CSS 2's **line-box strut** — the box CSS 2 §10.8.1 describes when it says each line box starts
  with "a zero-width inline box with the element's font and line height properties" and names that
  imaginary box a strut — **is the root inline box** in the current module: a real box the block
  container generates, not a special case. That reframes the gap M6 and §4.3 describe — elidex is
  not missing an ad-hoc height floor, it is missing a box.
  ⚠ CSS 2 §10.8.1 uses "strut" for a *second* object in the same section — the invisible glyph
  inside a **glyphless inline box**, which is what §1.5, M6 and M7 mean. The §-number does not
  discriminate them; this memo says "root inline box" for the line-box one and "strut" only for
  the glyphless-box one.

### §1.2 The existence rule — `css-inline-3` §2.3 *Phantom Line Boxes*

`body css-inline-3 invisible-line-boxes`:

> Line boxes that contain no text, no preserved white space, no inline boxes with non-zero
> **inline-axis** margins, padding, or borders, and no other in-flow content (such as atomic
> inlines or ruby annotations), and do not end with a forced line break are phantom line
> boxes. Such boxes must be treated as zero-height line boxes for the purposes of determining
> the positions of any descendant content (such as absolutely positioned boxes), and both the
> line box **and its in-flow content** must be treated as not existing for any other layout or
> rendering purpose.

| # | Clause | elidex status on `154bac3f` |
|---|---|---|
| 1 | no text | ✅ `contributes_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:556`) |
| 2 | no preserved white space | ✅ same site, `Pre`/`PreWrap` arm |
| 3 | **no inline box with non-zero inline-axis margin/padding/border** | ❌ **this umbrella** |
| 4 | no other in-flow content (atomic inlines, ruby) | ✅ atomic arm (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:607`); ruby unimplemented, out of scope |
| 5 | does not end with a forced line break | ✅ `force_break` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:781`) |

elidex implements suppression as the rule's stronger second half — no `LineBox` pushed, no
cursor advance, tentative rects/runs discarded (`flush_line`,
`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:209`, discard arm at `:423-430`).

**Clause 3 counts inline-axis edges only.** A `padding-top`-only inline leaves the line phantom.
**CSS 2 confirms both halves independently, in two different sections.** For **margins**, CSS 2 §8.3
(*Margin properties: margin-top, …, and margin* — full title in §3's row; the short form
*Margin properties* is an abbreviation wherever this memo uses it): "vertical margins will not have
any effect on non-replaced inline elements". For **padding and border**, CSS 2 §10.8.1 *Leading and
half-leading* (`body CSS2 leading`): "Although margins, borders, and padding of non-replaced
elements do not enter into the line box calculation, they are still rendered around inline boxes."
⚠ What each CSS 2 sentence buys is the **block-axis exclusion** — the half clause 3 needs for
`padding-top` — and nothing about the inline axis, which is css-inline-3 §2.3's own contribution.
The modern-module authority for all three is `css-inline-3` §5.3, whose layout-bounds inflation by
margin/border/padding applies only "when `line-fit-edge` is not `leading`", and
`css css-inline-3 line-fit-edge` gives `initial: leading`.
⚠ **Narrowed in rev 33** (round 25, Axis 4): an earlier drafting wrote that CSS 2 §8.3 "says nothing
about padding or border", which the section refutes — CSS 2 §8.3.1 *Collapsing margins*, which
`body CSS2 margin-properties` returns as part of CSS 2 §8.3, contains "no padding and no border separate
them". That is a **margin-collapsing** condition on block boxes, not a line-box-sizing rule, so
CSS 2 §8.3 is not the authority for the padding/border half either way.
⚠ **The asymmetry that narrowing left behind is withdrawn** (rev-33 gate): the replacement read
"for margins CSS 2 confirms it, for padding/border only `css-inline-3` does" — a universal whose
complement is a section this memo already cites elsewhere (§1.1, §1.5, §3's own row) and already
counts among the checked CSS 2 section↔title pairs above. CSS 2 §10.8.1 is that complement,
and it is the sentence quoted above.
⚠ **Every CSS 2 number in this subsection was bare until rev 34** (round 26, Axis 4); the front
matter states the convention, the carve-outs and the sweep's reach.

### §1.3 Line breaking — `css-text-3` §5.5

`body css-text-3 line-break-details`:

> Out-of-flow boxes and **inline box boundaries do not introduce a forced line break or soft
> wrap opportunity** in the flow.

An inline box's edges take inline-axis space, but the boundary is **not** a break opportunity.
What happens to a box that does not fit is **split**, not overflow — `body css-inline-3
line-boxes` (css-inline-3 §2.1): "When an inline box exceeds the logical width of a line box, **or contains a
forced line break, it is split** (see CSS Text 3 §5 …) into several fragments …, which are
partitioned across multiple line boxes." (Both `…` mark an elision: the first the css-text-3 §5 title, the
second the bare `[CSS-BREAK-3]` reference that follows "fragments" — ⚠ an earlier drafting marked
the first and dropped the second unmarked; rev-33 gate.) Splitting happens at opportunities *inside* the box,
never at its edges. Overflow is the **exception**, for a box with no internal opportunity —
CSS 2 §9.4.2: "If an inline box **cannot be split** … then the inline box overflows the line box."

### §1.4 Shaping — `css-text-3` §7.3 *Shaping Across Element Boundaries*

`body css-text-3 boundary-shaping`:

> Text shaping must be broken at inline box boundaries when any of the following are true …
> Any of margin/border/padding separating the two typographic character units in the **inline
> axis** is non-zero.

So a decorated boundary **must break** shaping. elidex currently coalesces same-entity text
across such a boundary (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:744`), which
is correct only while inline decoration takes no space — i.e. only until PR-1b.

### §1.5 Height and baseline — `css-inline-3` §5.3

`body css-inline-3 inline-height`:

> If the inline box contains no glyphs at all, **or if it contains only glyphs from fallback
> fonts**, it is considered to contain a "strut" (an invisible glyph of zero width) with the
> metrics of the box's first available font.

with `A′ = A + L/2`, `L = line-height − (A + D)` — the same half-leading arithmetic elidex
already applies at `crates/layout/elidex-layout-block/src/inline/pack/mod.rs:581`. The condition
is broader than CSS 2 §10.8.1's ("no glyphs at all"), and the *root inline box* gets special
treatment in the same section.

### §1.6 White-space collapsing across boundaries — `css-text-3` §4.1.1

`body css-text-3 white-space-phase-1`, Phase I step 4:

> Any collapsible space immediately following another collapsible space — **even one outside
> the boundary of the inline containing that space**, provided both spaces are within the same
> inline formatting context — is collapsed to have zero advance width. (It is invisible, but
> retains its soft wrap opportunity, if any.)

This is the direct statement that an inline box boundary does not stop collapsing. CSS 2 §16.6.1
states the same case directly (step 4.2: "even a space before the inline") but narrowly — leading
side only, with no same-IFC scoping — and is the superseded text.

## §2. Coupled invariants

| # | Invariant | Site |
|---|---|---|
| 1 | **Line existence** — the five clauses **§1.2's table** numbers in `css-inline-3` §2.3's defining sentence (the numbering is this memo's, per §5.1 M1; §1.2 is its single site) | `any_rendered_content` (`inline/pack/mod.rs:115`) |
| 2 | **Item-stream integrity** — cross-run collapse lookback; positional iteration | `inline/whitespace.rs:59`; `measure.rs`, `atomic.rs`, `pack/items.rs` |
| 3 | **Commit-on-content** — per-line rects/runs commit or discard | `inline/pack/mod.rs:116`, `:210` vs `:423` |
| 4 | **Baseline provenance** — `css-inline-3` §5.3 strut vs glyphs | `inline/pack/mod.rs:575` |
| 5 | **Line-box height composition** — `css-inline-3` §5.3 layout bounds, incl. the root inline box | `inline/pack/mod.rs:696` |
| 6 | **Group keying** — relpos/sticky sub-flows | `inline/collect.rs:286` |
| 7 | **Cursor/advance integrity** — wrap, trailing hang, shaping runs, intrinsic size | `inline/pack/mod.rs:690`, `:701`, `:772`; `inline/measure.rs:15`, `:44` |

| Pair | Coupling | Resolved in | PR |
|---|---|---|---|
| 2 × (all) | Which inlines emit markers sets the blast radius of everything else. | M1 | 1a |
| 1 × 2 | A marker between two text runs must not become a collapse barrier. | M2 | 1a |
| 7 × 2 | The box's edges take space but its boundary is not a wrap opportunity; and a decorated boundary must break shaping. | M3 | 1b |
| 3 × 7 | The box's rect must be a *content* span, with edges carried separately, or the border box double-counts. | M4 | 1c |
| 1 × 3 | Flipping existence moves whole lines from discard to commit, affecting **every** entity on the line. | M5 | 1d |
| 1 × 5 | A line kept only by decoration needs a height source, and elidex has no root inline box. | M6 | 1d |
| 1 × 4 | `css-inline-3` §5.3 gives a strut only to a glyphless box, so the baseline source depends on the whole line. | M7 | 1d |
| 1 × 4 (fallback-only half) | The same `css-inline-3` §5.3 sentence's **second** condition — "or if it contains only glyphs from fallback fonts" — is the same coupling over a signal elidex does not have at all. ⚠ Rev 33 carved this half out and swept §3, §5.3, §8 and §10; **§2 was missed**, and §5.3 says slicing has "one owning PR per coupling (**§2**)", so §2 is the authority a per-PR memo derives its DoD from — the row is added here rather than left implied (round 26, Axis 5). The idiom is row `2 × 6`'s: the slot is named in both owner columns. | `#11-inline-fallback-font-strut` | `#11-inline-fallback-font-strut` |
| 2 × 6 | Markers carry the enclosing recursion level's `group_key` — read only by the split rule, so the field arrives with its reader (M1 does not carry it in PR-1a). | `#11-inline-box-decoration-splits` | `#11-inline-box-decoration-splits` |
| 7 × (intrinsic) | An inline-axis advance the packer applies must also be visible to the intrinsic-size passes, which never build a `LinePacker`. | M8 | 1b |

Each **pair** is answered by exactly one M-row **or by exactly one slot**: of the **ten** rows
above, eight route to M1–M8 and **two** — `1 × 4 (fallback-only half)` and `2 × 6` — route to a
slot instead, and each column of those two names the same slot in both places. ⚠ **This sentence
read "Each pair is answered by exactly one M-row" and "pair 2×6 routes to a slot" until rev 34**
(round 26, Axis 5, Gate B): the universal already had one counterexample it named two clauses
later, and G1's `1 × 4 (fallback-only half)` row — added in this same revision — made it two, so
the summary had to be re-derived from the table rather than patched around. A prose summary of a
table that is edited without re-deriving the summary is how a table and its gloss part.
M-rows are **not** partitioned by PR — M1's payload
is split across PR-1a (`entity`), PR-1b (the three `EdgeSizes` + the `WritingModeContext`), PR-1d
(`line_height`, `families`, `font_size`, `font_weight`, `font_style` — named, per M1) and the
splits slot (`group_key`) by §8's dead-field rule — and the invariant *sets* overlap by
construction, because each PR consumes its predecessor's mechanism.
What §5.3's slicing rests on is narrower and true: each **coupling** has one owning PR, and each PR
is behaviour-scoped (item stream / inline-axis advance / box geometry / existence).

**Not an invariant, though it looks like one**: flow-line ordinal indexing. Code-contradicted —
`InlineFlowLine` carries absolute `block_start`/`block_size` and render iterates by coordinate
(`crates/core/elidex-render/src/builder/inline_flow.rs:111`); `flush_line` already drops
member-less buckets (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:346`) and
`slice_and_rebase_fragment` retains only non-empty ones
(`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs:63`).

## §3. Spec coverage map

Every row below is reachable from author CSS/HTML; §3.1 audits that surface and the pre-existing
laxity adjacent to it. ⚠ The `User-input flow` column reads "yes" on every row, which is **not** a
reason to drop it: the column is part of the schema `.claude/skills/elidex-plan-review`
Pre-condition #1 mandates and `preflight.py` warns on its absence. Its uniformity is the audit's
*result*, and keeping the field is what makes a future row answer the question rather than inherit
the answer.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS Inline 3 §2 Inline Layout Model | inline-axis edges respected between boxes | start/end edge advance | M3 shared core (`note_line_occupancy`, NEW) — **PR-1b** | ✓ | yes |
| CSS Inline 3 §2 Inline Layout Model | root inline box | block container's anonymous inline box | **NOT implemented — M6, `#11-inline-root-inline-box`** | ✗ (pre-existing, disclosed) | yes |
| CSS Inline 3 §2.1 Layout of Line Boxes | box exceeding the line, or containing a forced break | split into fragments across line boxes | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS 2 §9.4.2 Inline formatting contexts | box that **cannot** be split | overflows the line box | M3 — no wrap check on the marker path; §6 cell 15c (PR-1c; cell 15 asserts the advance) | ✓ | yes |
| CSS Break 3 §5.4 Fragmented Borders and Backgrounds: the `box-decoration-break` property | `slice` (initial) and `clone` | verbatim (apostrophes are the source's): "For inline elements, which side of a fragment is considered the broken edge is determined by the parent element’s inline progression direction. … (Note in particular that neither the element’s own direction nor its containing block’s direction is used.)" — and the section extends itself to bidi: "UAs should also apply `box-decoration-break` to control rendering at bidi-imposed breaks". ⚠ "no visual effect where the split occurs" is **CSS 2 §9.4.2**'s sentence and occurs nowhere in css-break-3 (checked across every css-break-3 §5.x anchor) | **`#11-inline-box-decoration-splits`** — the source of a fragment's edge attribution differs from M1's own-direction side mapping, so it belongs with the rule that owns it | ✗ (deliberate, §5.3) | yes |
| CSS Inline 3 §2.2 Layout Within Line Boxes | Note on empty inline boxes | they still have a line-height and influence the calculation | M6 — **PR-1d** for a **decorated** empty inline box; a **zero-edge** empty inline (`line-height` only) gets no marker — M1's non-zero-edge conjunct, a slice boundary kept for its presence-change ground — so that case is this program's **own** deferral, **`#11-inline-zero-edge-box-in-item-stream`** (§5.3) | ✗ (deliberate slice boundary: decorated ✓ via M6, zero-edge → slot; Codex on #515) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence (a) | zero-height for positioning descendant content (abspos) | `static_positions` (`inline/pack/mod.rs:97`) — **PR-1d** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | layout-bounds inflation by edges | applies only when `line-fit-edge` ≠ `leading`; initial is `leading` | no code touch; grounds §1.2's block-axis exclusion | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 3 | non-zero **inline-axis** margin/padding/border | clause-3 predicate → `has_inline_axis_edge` (M5); the predicate lands in **PR-1b** with its M3 consumers, **PR-1d** substitutes it for the constant | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 5 | forced line break | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence | line box *and its in-flow content* do not exist | commit/discard seam (`inline/pack/mod.rs:210` vs `:423`) — **PR-1d** | ✓ | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | `getClientRects()` step 3, the **one-line** case | one `DOMRect` "describing its **border area**" — padding + border, never margin | the `border_box()` fallback (`element/layout_query.rs:237`), correct once M4's edges are real — **PR-1c** | ✓ for a box with **one fragment on one line**; the residue is the row below | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | `getClientRects()` step 3, one `DOMRect` **per box fragment** in content order | ⚠ **the engine has no fragment unit at all.** `commit_aligned_entity_rects` folds to one entry per entity per **line** (`pack/mod.rs:479-487`) and `boxes.rs:102` gates on `line_rects.len() > 1`, so *line* is the only partition it can express; and runs persist in **logical** order, the UAX #9 L2 reorder being render's, not layout's (`inline/mod.rs:215-217`, `collect.rs:303-305`). So a **multi-line** box answers one content span per line (edges missing), and a box the spec fragments **within one line** — css-inline-3 §2.1's Note, "Inline boxes can also be split into several fragments within the same line box due to bidirectional text processing" — answers with *one* rect where the step requires one per fragment | **`#11-inline-box-decoration-splits`**, which owns css-break-3 §5.4 whole, and css-break-3 §5.4 names this case itself ("bidi-imposed breaks — i.e. when bidi reordering causes an inline to split into non-contiguous fragments"). Both halves are **pre-existing**: the count is already wrong on `154bac3f` and no PR here changes it. §6 cell 17d pins the multi-line half; `pack/mod.rs:445-446`'s "one border-box fragment per line …" docstring (the elided tail is "per inline element, in painted coordinates" — §3.1 quotes it in full) — which recurs at `boxes.rs:90-91` and four further sites — is `#11-inline-spec-cite-misattribution`'s (§9) | ✗ (deliberate, §5.3) | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | *get the bounding box* (`#element-get-the-bounding-box`), which `getBoundingClientRect()` returns the result of | step 1 invokes `getClientRects()`; step 4 returns "the smallest rectangle that includes all of the rectangles in list **of which the height or width is not zero**" (step 2 = zeros for an empty list, step 3 = **the first** rect when all are zero-area) | ⚠ elidex derives it from `LayoutBox.border_box()` and **never invokes `getClientRects()`** (`element/layout_query.rs:26-31` → `get_border_box`), so the two derivations are independent — pre-existing. PR-1c makes them *disagree*, at the broken edges only; §6 cell 17f pins it — **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | `clientTop` / `clientLeft` / `clientWidth` / `clientHeight`, step 1 | "If the element has no associated box **or if the box is inline, return zero**" — verbatim, and **identical across all four members** (`body cssom-view-1 dom-element-clienttop`) | ⚠ **step 1 is unimplemented for all four.** `clientTop` / `clientLeft` (`element/layout_query.rs:133-151`) read `lb.border.top` / `lb.border.left` as **direct field reads** and return zero today only because `inline/pack/boxes.rs:82-84` hard-codes `EdgeSizes::default()` — the lines M4 replaces — so PR-1c would turn an accidental correctness into a violation. `clientWidth` / `clientHeight` (`:119-131`) read `get_padding_box` (`:346`) and therefore **already violate step 1 today**, before this program. **Carved out of this umbrella into the predicate prereq PR** (§9), which must land **before PR-1a** — its predicate has two consumers, these four members and M1's emit test (§6 cell 6c). Why it is not a cell here: the predicate is css-display-3 §A Glossary's *inline box* — "A non-replaced inline-level box whose inner display type is flow. …" — the elision marks §A's second sentence, "The contents of an inline box participate in the same inline formatting context as the inline box itself", dropped unmarked at every site until rev 34 (round 26, Axis 4) — i.e. **two** inputs, and elidex answers only one (`is_atomic_inline`, `inline/collect.rs:14`, matches `InlineBlock`/`InlineFlex`/`InlineGrid`/`InlineTable` and **not** the replaced half of css-display-3's *atomic inline*). A guard keyed on `Display::Inline` alone would be a second, wrong answer to a question `collect.rs` already answers | ✗ (deliberate, §9) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | glyphless / fallback-only box | strut with first-available-font metrics | **two conditions, one delivered**: the *glyphless* one is M7's tentative baseline — **PR-1d**; the *fallback-only* one ("or if it contains only glyphs from fallback fonts") has no elidex signal at all and is **`#11-inline-fallback-font-strut`**'s (§5.3, §8) | ✗ (pre-existing, disclosed) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | half-leading | `A′ = A + L/2` | existing formula (`inline/pack/mod.rs:581`) — unchanged | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | break opportunities | inline box boundary is **not** one | M3 — the marker path calls the shared core without a wrap check — **PR-1b** | ✓ | yes |
| CSS Text 3 §5.5 Line Breaking Details | soft wrap opportunities *between* inline boxes | those of the text alone — the boundary between two runs adds none | `place_item`'s per-item flush (`pack/mod.rs:658` at `22de3078`) wraps at **every** item boundary with no break-opportunity test — pre-existing, **`#11-inline-item-boundary-soft-wrap`** (§5.3; Codex on #515) | ✗ (pre-existing → slot) | yes |
| CSS Text 3 §5.5 Line Breaking Details | intra-word shaping | "the characters must still be shaped … as if the word were still whole" | the coalescing `pack/mod.rs:744` documents; its comment's citation is corrected by **PR-1b** (§3.1), which is the PR that changes what it documents | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | adjacent soft wrap opportunity | break lands at the box's **margin edge** | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | max-content | inline-axis edges occupy space | M8 (`inline/measure.rs:44`) — **PR-1b** | ✓ | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | min-content | no accumulator to attach edges to | **`#11-inline-min-content-box-edges`** | ✗ (deliberate, M8) | yes |
| CSS Text 3 §7.3 Shaping Across Element Boundaries | shaping break | trigger 1 of 3: non-zero inline-axis edge | `last_placed_entity` coalescing (`inline/pack/mod.rs:744`) — **PR-1b** | ✗ (no shaping-break handling for trigger 2, `vertical-align` ≠ `baseline`, or trigger 3, "the boundary is a bidi isolation boundary" — set by `unicode-bidi`. Both properties resolve; the packer does not read either. Neither is this umbrella's subject) | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | steps 3–4 | a collapsible space is line-final only if nothing follows it on the line | `current_line_last_hang` (`inline/pack/mod.rs:701`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §4.1.1 Phase I: Collapsing and Transformation | step 4 | collapsing crosses inline box boundaries | `collapse_inline_whitespace` (the `fn`, `inline/whitespace.rs:27`; M2's arm joins the `match` at `:41`) — **PR-1a** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | Quirks Mode | an inline box fragment with **zero borders and padding** and no direct text is ignored when sizing the line box | not implemented — no quirks-mode layout switch exists; **§9** | ✗ (deliberate, §9) | yes |
| CSS 2 §10.8.1 Leading and half-leading | glyphless inline box | strut with first-available-font metrics (superseded by CSS Inline 3 §5.3, kept as the historical anchor) | M7 — **PR-1d** | ✓ | yes |
| CSS 2 §8.3 Margin properties: margin-top, margin-right, margin-bottom, margin-left, and margin | non-replaced inline elements | vertical margins have no effect | consistent with CSS Inline 3 §2.3 clause 3; no code touch | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline; sub-flow keying | `collect.rs:286` — **`#11-inline-box-decoration-splits`** (the `group_key` field arrives with its only reader, §2 pair 2×6) | ✗ (deliberate, §5.3) | yes |
| CSS Box Model 3 §3.1 / §4.1 | percentage margin/padding | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract Dimensions | inline size ≡ logical width | basis identity in vertical modes | same | ✓ | yes |
| CSS Writing Modes 4 §3.2 Block Flow Direction: the writing-mode property | box whose `writing-mode` differs from its **parent box** | an otherwise-`inline` box's display computes to `inline-block` | M1's emit test — such a box is an atomic and gets no marker; §6 cell 12c — **PR-1b** | ✓ | yes |
| CSS Writing Modes 4 §6.4 Abstract-to-Physical Mappings | side mapping | "based on the **used** `direction` and `writing-mode`" of the box being mapped | M1's `WritingModeContext` source; §6 cells 12b/12e — **PR-1b** | ✓ | yes |
| CSS Display 3 §A Glossary | *inline box* vs *atomic inline* | "A non-replaced inline-level box whose inner display type is flow. …" (the `…` marks the entry's second sentence, "The contents of an inline box participate in the same inline formatting context as the inline box itself" — restored as an elision in rev 34, the same class as the *atomic inline* third conjunct below; round 26, Axis 4) vs "An inline-level box that is **replaced** (such as an image) **or** that establishes a new formatting context … **and cannot split across lines** (as inline boxes and ruby containers can)" (`webref dfn css-display-3 "inline box"` → `§A Glossary #inline-box`; `body css-display-3 inline-box`) | the **canonical predicate** the prereq PR establishes (§9), consumed by M1's emit test — **PR-1a**, §6 cell 6c — and by the four `client*` members. ⚠ elidex today answers only the formatting-context half (`is_atomic_inline`, `inline/collect.rs:14`; and the `pub` `is_block_level`, `block/mod.rs:46`, is a second partition of the same enum), so the replaced half is unimplemented on the inline path and this program **consumes** the predicate rather than re-deriving one. ⚠ **The definition's third conjunct — "and cannot split across lines" — was dropped unmarked at all three quoting sites** (this row, §6 cell 6c, §9's canonical-predicate bullet) and is restored at each (rev-33 gate). It does not change M1's classification of an `<img>`, which the first conjunct already settles; it matters because this memo uses §A **as a predicate**, and a two-conjunct rendering of a three-conjunct definition is the wrong predicate for whoever implements it | ✓ | yes |
| CSS Pseudo-Elements 4 §4.1 Generated Content Pseudo-elements: ::before and ::after | `content` computes to anything but `none` — a `<content-list>`, `""` included | "generate boxes as if they were immediate children of their originating element"; since the initial `display` is `inline` the box is an inline box in the originating element's IFC (`webref heading css-pseudo-4 4.1` → `§4.1 … #generated-content`) | M1's pseudo routing (the `:265` branch at `22de3078` re-routed through the marker emission) — **PR-1a**; §6 cells 6g, 6h | ✓ | yes |
| CSS Content 3 §1 Inserting and Replacing Content: the `content` property | `<content-replacement>` vs `<content-list>` | the pseudo (or element) is a replaced element only under the former — a single `<image>`, which "Makes the element or pseudo-element a replaced element"; an `<image>` inside a list "is an inline anonymous replaced element", a replaced **child**, and the pseudo stays an inline box (`webref heading css-content-3 1` → `§1 … #content-property`; `body css-content-3 content-property`). ⚠ css-content-3 §1's issue note: a bare `<image>` "has historically been treated as `<content-list>` on ::before and ::after … [Issue #2889]" — the operative reading for the only box-generating reader of `ComputedStyle.content` today (the pseudo pipeline), so the flip is element-side (§8 requirement 7) | the predicate prereq PR's predicate (its mechanism is handed over in §9) — §8 requirement 7 (replacedness of generated content decided from the `content` model, total over `ContentItem`); §6 cells 6g, 6h | ✗ (pre-existing, disclosed: the spec's `<content-replacement>` branch has no elidex counterpart to enumerate — the live `parse_content` (`crates/css/elidex-css/src/declaration/misc.rs:441`) accepts only quoted strings, `attr()`, `counter()`/`counters()` and the keywords, so it rejects `url()` and no `ContentItem` can carry an image. ⚠ The path is named because **two** functions bear that name: the other is `crates/css/elidex-css-box/src/lib.rs:600`, reached only from that crate's own `parse` arm (`:162`) — the crate whose `resolve` `content` arm §8 requirement 7 measures as having no production caller. The claim holds for both — neither has a `url()` branch — but an unqualified name does not say which one was measured (round-24 gate). ⚠ The ✗ is about the *spec* branch, not about §8 requirement 7(c), which argues the opposite for the model: totality over `ContentItem` is what makes a future image variant a compile error rather than a silent gap) | yes |
| CSS Backgrounds 3 §3.2 Line Patterns: the `border-style` properties | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time — the loop at `crates/css/elidex-style/src/resolve/box_model/mod.rs:261-276` sets the width to `0.0` for `BorderStyle::None \| Hidden`, and that crate's own `border_width_zero_when_style_none` (`resolve/box_model/tests.rs:22`) asserts it, so this program adds no cell. ⚠ The row vouches for the **behaviour**, not for the site's comments: `:262` says only "CSS spec:" with no module or section, and `:270` cites "CSS Backgrounds §4.3" for the non-negative rule, which is *Corner Clipping* — the rule is css-backgrounds-3 **§3.3** *Line Thickness: the `border-width` properties*. Both are pre-existing and outside all three of §3.1's concept greps, so `#11-inline-spec-cite-misattribution` owns them by an explicit hand-off rather than by a grep. The css-backgrounds-3 §3.2 anchor here is this memo's, established by lookup | ✓ | yes |

**Breadth**: K=12 specs (CSS Inline 3, CSS Text 3, CSS 2, CSS Break 3, CSS Box Model 3,
CSS Sizing 3, CSS Writing Modes 4, CSS Display 3, CSS Pseudo-Elements 4, CSS Content 3, CSS Backgrounds 3, CSSOM View 1), M=38 entries (`Split decision` below restates K; both are recomputed). Both figures are recomputed
from the table above by `python3 .claude/tools/plan-xcheck.py <memo>`, which prints them and fails
on drift — that command is the verification artifact, and it is re-runnable rather than dated.
⚠ `preflight.py` reports `parsed citations: 0` here: its `SPEC_LABEL_REVERSE` carries no label
for any CSS module this memo cites (it has exactly one CSS label, `CSS Selectors L4` — `grep -n
CSS .claude/skills/elidex-plan-review/preflight.py`; an earlier drafting said "no CSS-module
labels", a universal the complement refutes), so its citation hard-gate is **vacuous for this
memo** and every §-number below was
verified by hand with `.claude/tools/webref` instead. Closing that gap is
`#11-preflight-css-module-labels`'s (the SoT slot owned by the citation-hygiene lane's Slice B,
after its A-ii migrates the dict) — not this umbrella's, and **not its plan-checker tooling
task's** either (an earlier revision booked it there, a second decision surface for one gap).

**Split decision**: K=12 ⇒ SPLIT-DEFAULT. The plan **is** split into four shipping PRs, each
behaviour-scoped, with one owning PR per coupling (§2, §5.3); the breadth verdict and the
invariant-axis verdict agree.

### §3.1 User-input touch audit

Adjacent pre-existing laxity:

* `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:82-84` hard-codes
  `EdgeSizes::default()` on every inline `LayoutBox`. **PR-1c fills it** (§5.1 M4) — for
  *every* decorated inline, not only the empty ones this slot names, because that is the gap
  §4.3 identifies and it is what makes the PR-1d flip honest rather than half-true.
* **Root inline box absent** (`grep -rni strut crates/` → nothing in inline layout):
  pre-existing, newly *depended on* by PR-1d. Disclosed in M6 with its own slot.
* **`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:726` cites "CSS Text 3 §5.6
  Shaping Across Intra-word Breaks"** — the section number and the title are **both**
  fabricated (`heading css-text-3 5.6` → no headings). The correct target is **css-text-3 §5.5**,
  not css-text-3 §7.3: that comment documents *intra-word* coalescing so render shapes whole
  words, and css-text-3 §5.5 is where that rule lives ("the characters must still be shaped … **as if the word were still
  whole**"). CSS Text 3 §7.3 is the opposite-direction rule M3 uses (shaping must **break** at a
  decorated boundary). **PR-1b** rewrites the comment — the one cite this program corrects, because PR-1b is what changes
  what it documents — so both land: css-text-3 §5.5 for the surviving intra-word coalescing, and
  css-text-3 §7.3 for the boundary break PR-1b adds. Both are PR-1b DoD items.
* **A nonexistent "CSS Box Model L3/Level 3 §5.3" is cited across `elidex-layout-block`**
  (css-box-3 §5 is *Borders*, no subsections). The class is defined by the **concept** grep

  ```
  grep -rEn "Box Model (L3|Level 3)[^a-z]*(§)?5\.3" crates/
  ```

  The correct target was established first, not inferred, with `webref heading css-box-3 3` / `4` / `body css-box-3 margin-physical`. ⚠ Those headings' **full** titles are "§3.1 Page-relative (Physical) Margin Properties: the `margin-top`, `margin-right`, `margin-bottom`, and `margin-left` properties" and "§4.1 Page-relative (Physical) Padding Properties: the `padding-top`, `padding-right`, `padding-bottom`, and `padding-left` properties" — earlier revisions showed them abbreviated while formatting them as literal command output, which is a fabricated transcript even when the abbreviation is harmless. The percentage basis comes from the same lookup: "Percentages: refer to logical width of containing block". The class is then whatever this grep returns; its hits on `154bac3f` are `lib.rs:178`, `helpers.rs:59`, `helpers.rs:114`,
  `positioned/layout.rs:90`, `block/mod.rs:162`, `block/mod.rs:182`,
  `block/children/helpers.rs:213`. `#11-inline-spec-cite-misattribution` (§9) owns **every hit of that grep** to css-box-3
  §3.1/§4.1 in one commit per [[feedback_semantic-sibling-selfseed-and-regate-breadth]]; the DoD
  states the property, not a count, so it cannot go stale against the grep.
* **A second bogus class: "CSSOM View §5" for `Element` members.** `getClientRects()` and
  `getBoundingClientRect()` are both in cssom-view-1 **§6** *Extensions to the Element Interface*;
  cssom-view-1 §5 is *Extensions to the Document Interface* (`webref heading cssom-view-1 5` / `6`). Defined by
  the concept grep

  ```
  grep -rEn "CSSOM[ -]?View[^)|]{0,15}§?\s*5\b" crates/
  ```

  whose hits on `154bac3f` span **three** crates — `elidex-plugin/src/layout_types/boxes.rs:106`,
  `elidex-shell/src/content/mod.rs:274` and `:286`, `elidex-dom-api/src/element/layout_query.rs:29`
  — while `layout_query.rs:355` already spells cssom-view-1 §6, so that file contradicts itself
  (and `:355` is outside this grep, since `offsetParent` is on `HTMLElement` = cssom-view-1 §7,
  not cssom-view-1 §6).
  `#11-inline-spec-cite-misattribution` (§9) owns **every hit of that grep** and that outlier. ⚠ A two-site list would have missed the `elidex-shell` pair, which is
  the same failure the Box Model class below taught; the class is whatever the grep returns.
* **`pack/mod.rs:425` cites CSS 2 §9.2.2.1** ("Anonymous inline boxes") for the phantom
  `getClientRects` geometry the discard arm avoids producing. The rule is CSS 2 §9.4.2, superseded
  by css-inline-3 §2.3 — §1.2's authority. It sits inside the arm §1.2, cell 6 and M4 all cite, and
  `#11-inline-spec-cite-misattribution` owns it (§9): `grep -rn "9\.2\.2\.1" crates/` returns nine
  hits and at least `pack/mod.rs:107`, `:200`, `:553` and three `text_height` test comments carry
  the same misattribution, so a coordinate list is not the class.
* **`pack/mod.rs:445-446` says `getClientRects()` "returns one border-box fragment per line …"** —
  the docstring's full clause is "returns one border-box fragment per line **per inline element, in
  painted coordinates**", and the ellipsis marks that tail rather than dropping it inside the
  quotation marks as an earlier drafting did (round 25, Axis 4; §9's rendering of the same phrase is
  attributed to `boxes.rs:90-91`, where the shorter form matches the source — ⚠ **`:90-91`, not the
  `:91` every site wrote until the rev-33 gate**: `:90` ends "…one border-box" and `:91` opens
  "fragment per line.", so the one-line coordinate cuts the phrase it names in half. Fixed at all
  four sites that carry it — this bullet, §3's CSSOM fragment row, §5.2's no-cite-sweep row and §9.
  The `grep -rn 'fragment per line' crates/` that *defines* the class is unaffected: the searched
  string lies wholly on `:91`). It is a
  restatement of cssom-view-1 §6 step 3 that swaps *fragment* for *line*. That is the engine's own
  fold (one entry per entity per line), not the spec's partition; §3's two CSSOM rows now say so.
  `#11-inline-spec-cite-misattribution` (§9) owns it.
  ⚠ **Why a string grep must not be used for the Box Model class above.** `grep -rn "Box Model L3" crates/` misses the
  three sites spelled `Level 3`, and a `Box Model[^|]{0,20}§` sweep misses a *different* three —
  `lib.rs:178`, `helpers.rs:59` and `helpers.rs:114` write the section number with **no `§`
  sign at all**. Only the concept grep, which makes both the level spelling and the `§` optional,
  returns the whole class.

## §4. Verified current state

* `any_rendered_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:115`) is
  written at these sites: `place_item` `|=` (`:698`), `force_break` `= true` (`:781`),
  `flush_line` reset `= false` (`:435`), constructor (`:184`).
* `place_item` (`:679`) writes this line state: soft-wrap flush (`:690`), `current_inline`
  (`:695`), `current_line_height` (`:696`), `on_line` (`:697`), `any_rendered_content` (`:698`),
  `current_line_last_hang` (`:701`), `current_line_entity_rects` (**`:706`**, guarded by
  `entity != self.parent_entity` at `:703`), **two** flow-member buffers under the
  `flow_align.is_some()` gate (`:736-769`, requiring a `FlowMember` — `inline/pack/items.rs:49`):
  `current_line_relpos_atomics` (`:739`), which takes the `PositionedAtomic` case that
  `place_item`'s own comment at `:718-721` calls "NOT a group flow member", and
  `current_line_runs` (`:745` for `Text`, `:760` for `Atomic`), the render-run-group bucket;
  and `last_placed_entity` (`:772`).
* `flush_line`'s per-line reset (`:432-439`) covers `current_inline`, `current_line_height`,
  `current_line_last_hang`, `any_rendered_content`, `last_placed_entity` — **not `on_line`**,
  which is reset only in `force_break` (`:783`). `place_item` is safe only because `:697-698`
  re-establish state immediately after its own flush. Any new caller of `flush_line` inherits
  that obligation.
* `StyledRun` (`inline/styled_run.rs:43`) carries no margin/padding/border member.
* `PackItem` (`inline/pack/items.rs:18`) refers to its `InlineItem` by **`item_index`**, not by a
  copied payload; the packing loop resolves it (`pack/mod.rs:534`, `:613`). M1 follows that idiom.

### §4.1 ⚠ `group_key` is the wrong entity

The slot memo proposed reading decoration off `StyledRun.group_key`. It is the **render-run-group
start** (`inline/styled_run.rs:76`, `inline/collect.rs:286`), not the decorated inline.
`StyledRun.entity` (`:45`) names the element — and does not rescue the option either, per §4.2.

### §4.2 ⚠ Two shapes, and **neither** reaches `place_item` today

`collect_inline_items_inner` emits no `InlineItem` for an inline element itself; it recurses
into the children (`inline/collect.rs:291`).

* **Shape A** — `<span style="padding:10px"> </span>`: a `StyledRun` exists, but its text
  collapses to `""` (`whitespace.rs:34` seeds `prev_collapsible_space = true`) and
  `build_pack_items` skips it (`inline/pack/items.rs:69`). `place_item` is never called.
* **Shape B** — `<span style="padding:10px"></span>`: no `InlineItem` at all;
  `layout_inline_context_fragmented` returns `line_count: 0` at `items.is_empty()`
  (`inline/mod.rs:161`) before packing.

Reading decoration off a run cannot fix either. The box itself must enter the stream.

### §4.3 ⚠ Inline box decoration is not laid out at all

`assign_inline_layout_boxes` hard-codes zero edges (`inline/pack/boxes.rs:82-84`) and the packer
never advances for an inline box's edges. CSS Inline 3 §2's "Inline-axis margins, borders, and
padding are respected between inline-level boxes" is unimplemented engine-wide. **This is why
the slot is an umbrella, and why §5.3 puts geometry before the existence flip**: the geometry
gap is visible today on ordinary `<span style="padding:10px">text</span>`, independently of
whether any line is phantom.

## §5. Design

### §5.1 Mechanism table

| # | Question | Decision | Grounds |
|---|---|---|---|
| **M1** | What enters the item stream, carrying what? | Two `InlineItem` variants, `InlineBoxStart` / `InlineBoxEnd`, emitted around the recursion at `inline/collect.rs:291` for every **inline box** — css-display-3 §A's defined term, i.e. **non-replaced** ∧ outer `inline` ∧ inner flow — with **at least one non-zero edge on any side**. ⚠ **The non-zero-edge conjunct is kept deliberately** (Codex on #515 asked whether a zero-edge empty inline — `<span style="line-height:100px"></span>` on a line other content keeps — should enter the stream for css-inline-3 §2.2's line-height influence): a marker for *every* inline box would — through M4's `InlineBoxEnd` pop, which pushes a rect **unconditionally**, and the commit arm's fold into `entity_bounds`, which the box-assigner iterates — grant a `LayoutBox` to every **undecorated** empty inline on an existing line: spec-correct (an empty inline box *is* a box; cssom-view-1 §6 returns a fragment for it) but a presence change over every empty `<span>`/`<a>`/`<b>` in every document, which cells 6b, 14's contrast (its no-marker clause), 21 and §7's "a box with all edges zero gains nothing" are all written against. **Widening to every inline box is the end-state; this program's non-zero-edge conjunct is a slice boundary, not a design ground**, and the widening is this program's **own** deferral — `#11-inline-zero-edge-box-in-item-stream` (§5.3), which names the cells it flips; §3's css-inline-3 §2.2 row is ✗ for the zero-edge case. ⚠ **The predicate is the FULL css-display-3 conjunction, and every conjunct is load-bearing — a `non-replaced` conjunct alone is not enough.** ⚠ An earlier drafting added only that one, on the strength of a single `<img>` example; the class is wider, and the arm admits all of it. `collect_inline_items_inner`'s **loop head, its style guard (`:217`, which routes an unstyled child to the text arm) and its four filters** (`:216-272`; ⚠ an earlier drafting labelled that range "the recursion arm" — the `for` body runs to `:317`, and both the recursion at `:291` and the text arm at `:302` are *outside* `:272`, so the range is right for what it is used for and the label was not — round-24 gate) filter exactly **four** things — `Display::None`, `is_absolutely_positioned`, `is_atomic_inline` (`:14`, four display keywords) and `PseudoElementMarker` — and passes every other **styled** child to the inline-element recursion at `:291`. ⚠ Two earlier draftings, both fixed by round 24's §5.1 audit: the range was `:216-251`, which stops at the `is_atomic_inline` arm's `continue` and so does not bracket the fourth filter (`PseudoElementMarker` is `:260-272`); and "passes everything else" has a non-empty complement — a child with **no** `ComputedStyle` never reaches `:291` at all, because `:217`'s `if let Some(style) = crate::try_get_style(…)` sends it to the text arm at `:302` (a text node or a comment). ⚠ **The `PseudoElementMarker` filter is a *routing* special-case, not an exclusion from the class** (Codex on #515): a `::before`/`::after` pseudo entity (`elidex-style`'s `generate_pseudo_entity`, cascaded by `collect_and_cascade_pseudo`, so it carries its own `ComputedStyle` — padding included) with computed outer `inline` **is** a css-display-3 inline box, and the `:265` branch (`:265` at `22de3078`; `:260` at `154bac3f`) pushes its `Text` and `continue`s *before* `:291`, so markers placed only around the recursion would never bracket it. **PR-1a routes that branch through the same `InlineBoxStart`/`InlineBoxEnd` emission** — the predicate evaluated on the pseudo's own style (§8 requirement 7 puts that entity kind — `NodeKind::Text`, no `TagType` — in the predicate's domain), the emission keyed on the predicate **alone** and bracketing **whatever the branch pushes — one `Text`, or nothing**: the branch pushes a `Text` only under `!tc.0.is_empty()` (`:266-274` at `22de3078`), and elidex generates a pseudo entity for `content: ""` too (`elidex-style/src/resolve/box_model/mod.rs:424` maps `CssValue::String(s)` → `ContentValue::Items(vec![String(s)])`, `pseudo.rs:47` returns only when `content` is not `Items(_)` and `:63` then creates the entity, `generated_content.rs:158` writes `TextContent("")`), so an empty decorated pseudo — `p::before { content: ""; padding: 10px }`, the ubiquitous decorative idiom — yields an **adjacent `InlineBoxStart`/`InlineBoxEnd` pair**, cell 14c's shape for generated content — so a decorated pseudo reaches M3/M4/M5 like any element; §6 cells 6g (non-empty) and **6h** (empty) pin the two. ⚠ An earlier drafting keyed the markers to "its single `Text` item", which silently dropped the pair for empty `content` (round 22, Axes 2/3). Three measured members reach the recursion that are **not** inline boxes: <br>• **replaced** — no UA rule selects `img`/`iframe`/`canvas`/`video`/`svg` (`grep -n '\bimg\b' crates/css/elidex-style/src/ua.rs` → no hits), so all compute `display:inline` and `is_atomic_inline` is false for them. css-display-3 §A makes them *atomic inlines* — and the **clause numbering is this memo's own** (§1.2's table), applied to css-inline-3 §2.3's **defining sentence**, which is itself unnumbered (`body css-inline-3 invisible-line-boxes`; ⚠ an earlier drafting called css-inline-3 §2.3 *itself* "a single unnumbered sentence" — the section carries a second normative sentence, "Such boxes must be treated as zero-height line boxes …", and a *What's invisible?* note with a five-item list, so the property belongs to the defining sentence and not to the section — round-24 gate). css-display-3 §A's *atomic inline* entry is likewise unnumbered prose — two sentences, no enumeration (`body css-display-3 glossary`; the second, "Any inline-level box whose inner display type is not flow establishes a new formatting context …", is the shape §A's other multi-paragraph entries use), so "clause 3" / "clause 4" cannot be read against §A at all: read against css-inline-3 §2.3, clause 3 does not reach them and clause **4** does. ⚠ An earlier drafting said the numbering was css-inline-3 §2.3's and described **§A** as "a single prose sentence with no enumeration" — §A is *Appendix A: Glossary*, some twenty definition entries; only the *atomic inline* entry inside it is the single sentence (round 24 audit). <br>• **`display: contents`** — no `Contents` branch exists in `collect.rs`, and `:277` takes **raw** `dom.composed_children`, while the sibling `positioned_subflow_key` takes `composed_children_flat` (`:110`) *for exactly this reason*, its own comment at `:93` saying so. `ua.rs:109` ships `slot { display: contents; }`, so it is live. css-display-3 **§2.5** *Box Generation: the `none` and `contents` keywords*: "The element itself does not generate any boxes, **but its children and pseudo-elements still generate boxes and text sequences as normal**", so it has no outer display type to be inline. ⚠ Two rev-34 corrections here (round 26, Axis 4). (i) The **title** was abbreviated to *Box Generation* while §6 cell 6d spelled it in full, and this memo marks an abbreviation explicitly where it uses one (§1.2's *Margin properties*) — so the two sites disagreed with no marker between them. (ii) The **quotation** stopped at "any boxes" with no ellipsis, dropping the `but` clause; that defect was at *both* sites, and the clause matters because it is what cell 6d's own second assertion ("`x` still reaches the line") rests on. <br>• **block-level reached through an inline** — `children_are_block` (`crates/layout/elidex-layout-block/src/block/mod.rs:70`) tests **direct** children only, so `<p>a<span><div style="padding:10px"/></span>b</p>` takes the IFC path and the `<div>` reaches the arm. <br>M1 therefore **consumes the canonical predicate the prereq PR establishes** (§9) — outer display `inline` ∧ inner display flow ∧ non-replaced ∧ generates a box — rather than re-deriving any of it here. That is the same predicate the four `client*` members consume, which is why that PR lands before **PR-1a**. §6 cells **6c/6d/6e** pin one member each. ⚠ `<br>`/`<wbr>` are **not** in this class: per css-display-3 they *are* inline boxes, so a marker for a decorated one is correct. That elidex implements neither — one probe per tag over `crates/` whole, `grep -rnE '"br"\|BrMarker' crates/` (21 lines) and `grep -rniE '\bwbr\b' crates/` (7), 27 distinct `file:line` between them and **all** of them parsing or DOM, **none** in `crates/layout`, `crates/core/elidex-render` or `crates/core/elidex-ecs`; `force_break()` has one caller, `pack/mod.rs:603`, the preserved-`\n` path — is a pre-existing gap §9 records (⚠ both probes were scoped to those three directories until rev 34 and are rescoped at both sites in one edit — §9's bullet carries the complement; round 26, Axis 1, Gate B). ⚠ **What is unimplemented is each tag's *break* behaviour, not the element**, so this program does reach a decorated one: neither tag is selected by any UA rule, so `<br style="padding:10px">` computes `display:inline`, is styled, and passes all four filters above to the `:291` recursion — M1 emits its marker, and PR-1d flips `<p><br style="padding:10px"></p>` (an instance of Shape B) to a committed line. §9's bullet carries the chain and withdraws rev 32's contrary claim that no cell of this program is constructible with either tag (round 25, Axis 3). ⚠ An earlier drafting quoted a **case-insensitive** single command (`grep -rniE '"br"\|BrMarker' …`) as returning nothing; it returns one hit at `154bac3f` — `elidex-render/src/builder/tests/paged.rs:446`, a paged-media `"BR"` corner label the `-i` matches — and it probed only one of the two tags. The conclusion is unchanged; the transcript was not reproducible, which is the defect (round 24 audit). Payload: `entity` + the **three physical `EdgeSizes`** `resolve_box_model(&style, containing_inline_size)` returns (`helpers.rs:116`) + the `WritingModeContext` they were resolved under (`logical.rs:27`), built from **the decorated inline's own `style`** (`inline/collect.rs:217`) — `WritingModeContext::new(style.writing_mode, style.direction)` — + the **five resolved font-and-height fields M6 and M7 read, named rather than glossed**: `line_height`, `families`, `font_size`, `font_weight`, `font_style`. ⚠ **Five, not "`line_height` and font identity"** (round 26, Axis 2): the gloss under-specifies the payload against its own readers. M7 calls `FontDatabase::query(families, weight, style)` (`crates/text/elidex-shaping/src/database.rs:60`) and `font_metrics(id, font_size)` (`:101`), so all four of the font fields are read and not one of them is optional; and M6's `block_advance` follows the packer's own vertical convention `if is_vertical { font_size } else { line_height }` (`pack/mod.rs:539-543`), so **M6 reads `font_size` too** — a field the gloss did not carry at all. The five names are the crate's own for the same five facts: `StyledRun` carries `families` (`inline/styled_run.rs:49`), `font_size` (`:51`), `font_weight` (`:53`), `font_style` (`:55`) and `line_height` (`:61`), and M1 follows that spelling so no reader has to re-derive a mapping. Grounds: <br>• **The axis is the IFC's by construction, so its source cannot matter.** css-writing-modes-4 §3.2 *Block Flow Direction: the `writing-mode` property* (`webref heading css-writing-modes-4 3.2`): "If a box has a different `writing-mode` value than **its parent box** … If the box would otherwise become an in-flow box with a computed display of `inline`, **its display computes instead to `inline-block`**." (The trigger is the *parent box*, not the containing block, and the "establishes an independent … formatting context" clause of the same rule applies only to a box that is a block container — an inline reaches `inline-block` by the display change, not by that clause.) A decorated *inline box* therefore always shares the IFC's writing mode; a `<span style="writing-mode:vertical-rl">` computes to `inline-block`, i.e. an atomic, which M1 emits no marker for. <br>• **The direction is the box's own.** css-writing-modes-4 §6.4 gives the abstract-to-physical mappings "based on the **used** `direction` and `writing-mode`" — of the box whose sides are being mapped. `direction` *can* differ on an inline box without forcing an independent context, so it is the one component that varies, and css-writing-modes-4 §6.2/§6.4 say it is the box's own. <br>• **Why the two competing readings fail**: css-writing-modes-4 §2.1 (*Specifying Directionality: the `direction` property*), verbatim: "The direction property has no effect on bidi reordering when specified on inline boxes whose unicode-bidi value is normal, **because the box does not open an additional level of embedding with respect to the bidirectional algorithm**." Its subject is the *embedding level* of reordered content, not the mapping of a box's own sides, so it does not license taking the direction from elsewhere. ⚠ Earlier revisions paraphrased this as "when `unicode-bidi` is `normal`", dropping both "when specified on inline boxes" and the `because` clause that names the mechanism — the clause is what makes the refutation hold rather than merely assert it. (Cell 12b's only `[dir]` element is the `<p>`, so its `<span>` really is `unicode-bidi: normal`; the citation is sound and still does not reach). css-break-3 §5.4's broken-edge rule — quoted in full in §3's css-break-3 row, and **referred to, not re-quoted, everywhere else including here**, so one edit keeps every site true — is scoped to *which side of a fragment is the broken edge*, and its own example is an element that "breaks across two lines" — an unfragmented box has no broken edge. §3 routes that whole section — both `slice` and `clone` — to `#11-inline-box-decoration-splits`, which is also where the parent-direction source lives, so the two direction sources never meet inside one PR. <br>Only the physical edge *values* come from the element's own `ComputedStyle`. Logical facts are *derived* at the point of use via `LogicalEdges::from_physical` (`logical.rs:186`), applied to each set separately: their inline-start/inline-end components summed across the three sets give M3's advance and M4's content offset; the same components tested *per set* give M5's predicate. **Both are derived in `pack/inline_box.rs` beside `has_inline_axis_edge`, not cached on the marker** — the sums' inputs already sit on the payload, and §5.2 designates that module the one derivation site for everything read off a marker, so a stored total would be a second representation of a fact the same struct already determines. `helpers.rs`'s `inline_pb` (`:148`) is **not** reusable here — it covers padding + border only, and the advance must include margin. The `PackItem` forms are `InlineBoxStart { item_index }` / `InlineBoxEnd { item_index }` — **an index, not a copy of the payload**, per §4's `PackItem` idiom; markers never become `FlowMember`s. ⚠ **Split across PR-1a and PR-1b by §8's dead-field rule.** PR-1a emits `InlineBoxStart { entity }` / `InlineBoxEnd { entity }`: the emit *test* resolves the edges at collect time and then discards them, because the variants' only PR-1a readers are the exhaustive matches. The three `EdgeSizes` + `WritingModeContext` join the payload in **PR-1b**, with their first readers M3/M4/M5, and the five font-and-height fields in **PR-1d**, with their first readers M6/M7. The rule the DoD states therefore reaches this payload too, not only `line_height` / `families` / `font_size` / `font_weight` / `font_style` / `group_key`. | **Three sets, not one**: `LayoutBox` has independent `padding`/`border`/`margin` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:88`, `:90`, `:92` — ⚠ **not** a contiguous `:88-91` range, as an earlier revision wrote: the fields are doc-comment-separated and `margin` is `:92`) consumed separately by `padding_box`/`border_box`/`margin_box` (`:134-149`; ⚠ an earlier drafting wrote `:134-150`, whose `:150` is the `}` closing the `BoxModel` trait, not part of any method — round-24 gate), and `resolve_box_model` already returns the triple — one `LogicalEdges` cannot fill three. **Physical, not logical, across the boundary**: `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) writes *physical* fields and receives `is_vertical: bool` with **no `direction`**, so a logical payload would need a return trip whose context is not present there. Carrying physical + the ctx keeps one conversion direction and no reconstruction. **Index, not copy**: a copied payload would duplicate the item stream's data against the file's own `item_index` idiom, and would give M5 two derivation sites — the pre-pack gate holds `&InlineItem` and the packer holds a `PackItem`, so only an index lets both read the *same* payload through one function. Emit on **any** side because M4 must fill all four for a block-axis-only inline to paint; the *existence* test is inline-axis-only (M5) — see §6 cell 6 for the pair. `resolve_box_model` is mandatory: `sanitize_padding` (`:96`) resolves percentages against 0, `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative. Basis = logical width = inline size (css-box-3 §3.1/§4.1; css-writing-modes-4 §6.1), `resolve_padding`'s documented contract (`:57-62`). |
| **M2** | Collapse-pass arm | **Transparent** — same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so interleaved markers are skipped by construction — including an adjacent `Start`/`End` pair. | §1.6 (css-text-3 §4.1.1 step 4): collapsing crosses "the boundary of the inline containing that space". A barrier would change `a <span style="padding:10px"> </span>b`, currently-correct markup: today the `<p>`'s `"a "` leaves `prev_collapsible_space` true (`whitespace.rs:34` seeds it, `collapse_run_text`'s `Normal` arm sets it on the space), so the span's own space collapses to zero advance and the IFC renders **`a b`**; a barrier resetting the flag at each marker — the `Atomic` arm's behaviour (`whitespace.rs:45-48`) — would let that space through and render **`a  b`**. ⚠ An earlier drafting used `a<span style="padding:10px"> </span>b`, on which a barrier is a **no-op**: with the space only inside the span, `prev_collapsible_space` is already false on entry to it, so both readings render `a b` and the markup cannot discriminate them (round 24 audit). |
| **M3** | How does anything occupy a line? | **One owner, two callers.** Extract `LinePacker::note_line_occupancy(inline_advance, block_advance, hang: Option<f32>, contributes_content, occupant: LineOccupancy)` (NEW, private) writing exactly **five of** the line-state fields `place_item` writes today (§4 enumerates eight; an earlier drafting wrote "the five", a definite article the enumeration refutes — and this row itself reaches a sixth, `last_placed_entity`, below — round 24 audit): `current_inline +=` (`:695`), `current_line_height = max(..)` (`:696`), the line-occupancy raise that replaces `on_line = true` (`:697`, see the ⚠ below), `any_rendered_content \|=` (`:698`), and — **only when `hang` is `Some`** — `current_line_last_hang =` (`:701`). `place_item` calls it **after** its soft-wrap check (`:690`) and **after** snapshotting `seg_inline_start = self.current_inline` (`:694`, which five downstream sites consume), passing `Some((full-trimmed).max(0.0))` and the occupant the ⚠ below **derives** (`Content` at every PR-1b call site, since the ordering has no higher rung yet) — behaviour unchanged. The marker path calls it with **no** soft-wrap check, `occupant = BoxEdgeOnly`, `contributes_content` from M5, `block_advance = 0.0` in PR-1b (M6 supplies the real value in PR-1d, when `line_height` joins the payload under the dead-field rule — so all five parameters are named at both stages), and `hang = Some(0.0)` when M5's `has_inline_axis_edge` holds, **`None` otherwise** — a zero-advance marker (a `padding-top`-only box, cell 6) leaves the hang alone, because the space before it *is* still line-final. A marker for which `has_inline_axis_edge` holds also clears `last_placed_entity` — the write `flush_line` already performs at `:439` (⚠ an earlier drafting cited `:772`, which *sets* the field to `Some(entity)` at the end of `place_item`; the clear is `:439` — round-24 gate). **Both readers of the resulting state are order tests over the ordering, never equality**: the soft-wrap guard (`:690`) holds at-or-above `Content`, `finish()` (`:787`) above `Empty`. The two have **different grounds** and the Grounds column separates them: the guard's predicate is exposed to extension and an equality there stops soft wrapping engine-wide at M7's widening, while `finish()`'s `!= Empty` and `> Empty` are the same predicate under any extension — `Empty` is the bottom and every raise is monotone — so that one is written as an order test for uniformity, not for correctness. §6 cell 15d asserts the guard's half; nothing needs to assert `finish()`'s (round 26, Axes 1/2/3). | Without a shared core, *entering the line* is unowned: the marker path would either duplicate `place_item`'s five-write sequence or silently skip part of it. `Option<f32>` rather than an `f32`: the field is a *conditional* write for the marker and an unconditional one for `place_item`, and an `f32` parameter cannot express "leave it alone" — `place_item` assigns unconditionally today (`:701`). No wrap check: §1.3, a box boundary is not a break opportunity. `hang = Some(0.0)`: css-text-3 §4.1.2 keys on "at the end of **a line**", the engine encodes that at `:264-279`, and `place_item` already zeroes it for an atomic. Shaping break on non-zero **components** (§1.4's wording), so a negative or compensating pair still breaks. **One 3-valued field, not two bools**: the two readers want different questions of the same fact ("has anything entered this line?" for `finish()`, "has *content* entered it?" for the wrap guard), which is one ordered state, and CLAUDE.md's "one issue, one way" prefers encoding it once over an invariant maintained across three sites. ⚠ **`on_line` has a second reader** — the soft-wrap guard's `&& self.on_line` (`:690`) — and a marker at the head of a line would arm it, so the first content segment could soft-wrap where `on_line == false` protects it today. The guard has always meant *do not flush a line with nothing on it*, and "nothing" turns out to be the absence of **content**, not of a box boundary — so the line now has **three** states, not two. **Mechanism**: `on_line: bool` becomes `line_occupancy: LineOccupancy` (`Empty` / `BoxEdgeOnly` / `Content`), written in exactly two places **after construction** — `note_line_occupancy` **raises** it monotonically from an occupant argument, and `flush_line`'s per-line reset block (`:432-439`) sets `Empty`. ⚠ **The occupant `place_item` passes is *derived inside `place_item`*, from the arguments it already has — not a property of which caller reached it.** `place_item` takes `contributes_content: bool` (`:686`) **and** `member: FlowMember<'_>` (`:687`), and `FlowMember`'s variants (`inline/pack/items.rs:49-58`) separate `Text(&'a str)` from `Atomic` and `PositionedAtomic`. The derivation is a total match on the pair: **`(FlowMember::Text(_), true)` ⇒ M7's `RenderedText` rung; every other pair ⇒ `Content`** — the two atomic variants (whose caller passes a literal `true`, `:658`) and a `Text` segment whose `contributes_content` is `false` (a collapsible-whitespace segment) alike. The marker path, `note_line_occupancy`'s second caller, passes `BoxEdgeOnly`. **In PR-1b the ordering has no rung above `Content`, so the derivation is constant `Content` and behaviour is unchanged**; PR-1d's widening (M7) is what gives the first arm somewhere higher to go. No caller gains a parameter, and the text/non-text distinction is carried by an **enum variant** rather than by a bool two callers feed — which is why `place_item`'s literal-`true` atomic call cannot reach the `RenderedText` rung. ⚠ **The rung is named for what its predicate means, and that is not "glyph"** (round 26, Axis 2): `contributes_content` is `!text.is_empty()` under `pre`/`pre-wrap` (`pack/mod.rs:556-558`), so a lone preserved segment break — `<pre>\n</pre>`, which the code's own comment at `:547-551` names — satisfies it on a line that renders no glyph. M7's Grounds records that corner; the point here is only that "glyph-bearing", which rev 33 used at every site, named a predicate the derivation does not compute. ⚠ **Both readers are *order* tests over that ordering, and neither is an equality**: the soft-wrap guard (`:690`) holds when the line has reached **at least** `Content`; `finish()` (`:787`) when it stands **above** `Empty`. ⚠ **Their grounds are different, and only one of the two is actually about extension** (round 26, Axis 1, Gate A; an earlier drafting wrote "the ground is one and the same at both sites, and it is that the ordering is open to extension", which is true of the guard and vacuous for `finish()`). For **`finish()`** the two spellings coincide and always will: `Empty` is the ordering's **bottom**, `note_line_occupancy` only ever **raises**, and the only writer that lowers is `flush_line`'s reset back to `Empty` — so `!= Empty` and `> Empty` select the same lines under *any* extension, because an extension can only add rungs above the bottom. Writing that one as an order test buys idiom, not correctness, and its ground is uniformity: one ordering, one way of reading it. Only the **guard's** predicate is exposed, and it is exposed precisely because `Content` is *not* the top: M7's PR-1d widening adds a rung *above* `Content` and routes `(FlowMember::Text(_), true)` straight to it, so a segment that raises the top rung **never passes through `Content`**, and `== Content` goes false under the line's feet. ⚠ **And the set it goes false on is not "the lines that carry text", in either direction** (round 26, Axis 1, Gate A): by the derivation above a `Text` segment whose `contributes_content` is `false` — collapsible whitespace — derives `Content`, so a line carrying only that still reads **true**; and an atomic-only line is `Content` as well and must soft wrap. What `== Content` goes false on is *the lines that reached `RenderedText`*, which is **narrower** than "carries text"; what must soft wrap is *every* line at or above `Content`, which is **wider**. The equality therefore stops the flush on the `RenderedText` lines — the ordinary glyph-bearing ones, which is the common case and more than enough — and **soft wrapping stops engine-wide** at PR-1d. ⚠ **Rev 33 wrote the equality** (round 26, Axes 2/3, reached independently): it is the CRIT that an open ordering invites whenever a reader is written against the ordering's current top instead of against the order. Nothing in §6 caught it — cell 15b evaluates the guard exactly once, at `BoxEdgeOnly`, where both predicates agree, and cell 15 asserts that nothing wraps — so **§6 cell 15d** is added to discriminate it and §8's PR-1d gate takes it as a checkable item. ⚠ Adding it to the reset block is **behaviour-neutral today** and removes the asymmetry §4 records: `flush_line`'s three callers are `place_item`'s soft-wrap (which raises to `Content` at `:697` immediately after), `force_break` (whose `on_line = false` at `:783` becomes the same reset), and `finish()` (after which the value is never read). ⚠ **`force_break`'s *other* line-state write is dispositioned here too, and rev 33 left it unstated** (round 26, Axis 2): `any_rendered_content = true` (`:781`) stays exactly where it is. M3 moves only `place_item`'s `\|=` (`:698`) into `note_line_occupancy`, so after the extraction that field has **one writer inside the new owner and one outside it**. There is no behavioural consequence — `force_break`'s write is unconditional and its own, and the core never runs on that path — but the disposition is stated rather than left silent, because this row's whole argument against a bool pair is about write sites, and an unstated second writer leaves standing the shape the row condemns. A second bool would instead need three write sites after construction — **four to this field's three** once the constructor is counted the way §4 counts one for the sibling `any_rendered_content` (`constructor (:184)`) — plus an implicit `content_on_line ⇒ on_line` invariant, and a reset asymmetry no single field pays. ⚠ An earlier drafting compared two against three, counting construction on neither side for `line_occupancy` and on none for the bools; the contrast is stated on one basis now (round 24 audit). §6 cell 15b pins the three-state design and §6 cell 15d the order test. |
| **M4** | Where does the box's rect come from, and how do the edges reach `LayoutBox`? | **Rect**: an open-box **stack** records each `InlineBoxStart`'s cursor; `InlineBoxEnd` pops it and pushes one `current_line_entity_rects` entry (`:706`) for `entity` spanning **content-start → end-cursor** (read **before** the end marker's own advance) — the box's **content** span, never inflated by edges. Pushed by an explicit branch, not the `entity != parent_entity` guard (`:703`), which does not suppress a nested inline. ⚠ **Two channels, one buffer — and PR-1c fixes only the one it can.** `getClientRects` returns `InlineClientRects` **early**, never touching `border_box()` (`crates/dom/elidex-dom-api/src/element/layout_query.rs:219-233`; ⚠ an earlier drafting wrote `:219-232`, which stops one line short of the `return` at `:233` that makes the exit early — round 24 audit), and cssom-view-1 §6 step 3 requires "one for each box fragment, describing its **border area**". `LayoutBox.content` must stay the *content* union or `border_box() = content + padding + border` double-counts. Those are contradictory demands on one value: `commit_aligned_entity_rects` builds a single `painted` rect and feeds it to **both** `EntityBounds`'s min/max bounds (→ `LayoutBox.content`, `boxes.rs:80-81`) and `EntityBounds.line_rects` (→ `InlineClientRects`, `boxes.rs:102-126`) — verified at `pack/mod.rs:488-509`. **All three buffers keep *content* spans, exactly what they hold today.** PR-1c therefore makes the **one-fragment-on-one-line** channel correct and nothing else: no `InlineClientRects` is stored below `len() > 1`, so `getClientRects` falls back to `LayoutBox.border_box()`, which M4's real edges make right. ⚠ **The more-than-one-line channel stays content-span and is a disclosed divergence, not a fix PR-1c withholds** — inflating stored fragments is `#11-inline-box-decoration-splits`'s work, on two grounds PR-1c cannot discharge: (a) **content-order identity** — `slice_and_rebase_fragment` does `b.line_rects.retain(…)` (`pack/fragment.rs:69`) immediately before the consumer (`inline/mod.rs:377` then `:380`), so under paging/multicol `line_rects[0]` is the first *kept* rect in this fragmentainer, not the box's first fragment in content order; (b) **which edge survives a break is the *parent's* inline progression direction** per css-break-3 §5.4 (§3 carries the sentence in full) — a different source from M1's own-direction side mapping and belongs with the rule that owns it. ⚠ Reachability is deliberately **not** a third reason: the edge **write** happens in `assign_inline_layout_boxes`, which `continue`s on an existing `LayoutBox` (`boxes.rs:62-64`; ⚠ an earlier drafting wrote `:60-62`, which is the `if` plus the two comment lines above it and stops short of the `continue` at `:63` — round 24 audit, applied to every site of the range), but §8 records that limit for *every* geometry PR-1c writes — M4's own edge write included — so it cannot discriminate between what stays and what leaves. §6 cell 17d pins the divergence as accepted, in the shape cell 23 already uses. The stack entry stores the box's **content-start cursor** (already past the marker's inline-start advance), so the rect is `content-start → end-cursor` with no edge re-added — the same content-span meaning `place_item`'s rects already carry. Its `block_start` is snapshotted from `current_block_offset` at the same moments `place_item` snapshots it, so all of a line's rects share one value — the invariant `commit_aligned_entity_rects` relies on. ⚠ **The two emission rules differ on emptiness, deliberately.** `InlineBoxEnd`'s pop pushes **unconditionally**, zero-width span included — an ended box *is* a fragment, and cell 14c's empty decorated inline has no other producer, so a non-empty test there would silently delete the whole presence change. The flush-time hook is the opposite: `flush_line`, at the top before any arm runs, walks the **whole** open-box stack and **emits** a partial rect only for an entry whose span is non-empty — an *open* box with zero span has not become a fragment on this line, and emitting one would duplicate the rect its eventual `InlineBoxEnd` will push (cell 17c is that case). Cells 14c and 17c each exercise one of the two rules; an implementer unifying them breaks exactly one cell. So: and **rebases every entry's content-start to 0 unconditionally** — the two scopes differ, and binding the rebase to the emitted set would leave a box opened at the end of line N (span 0, no rect) holding a line-N cursor into line N+1 and yielding an inverted rect. §6 cell 17c pins it — the start edge was consumed on the earlier line and must not be applied again. The end edge is symmetric and needs no handling: an open box has not reached its end marker, so line N's partial rect reserves nothing for it, so a box straddling a break yields one rect per line and a box opened exactly at a break yields none. ⚠ **The marker's rect is a second producer for the same entity**: a decorated inline with text already has a `place_item` rect on that line (`:706`, its runs carry `entity == span`). The **persisting** arm folds per entity before committing (`commit_aligned_entity_rects`, `:479-487`), so one fragment per line survives — correct, and the only arm that runs. The non-persisting arm — the whole `else` clause at `pack/mod.rs:393-421`, whose unmerged rect loop is `:401-420` — does **not** fold, but it is **dead code**: `FragmentationType` has exactly `Page` and `Column` (`crates/layout/elidex-layout-block/src/lib.rs:37-42`) and `InlineFragConstraint.fragmentation_type` is non-optional (`inline/mod.rs:94`), so `persist_candidate = frag_constraint.is_none() \|\| frag_is_paged \|\| frag_is_column` (`:239`) is **identically true** and `flow_align` is always `Some`. No cell is written against that arm: a cell no markup can construct is exactly what M8's grounds refuse. The dead arm is deleted by a prereq PR (§9). **Edges**: **one derivation site; the box-assigner stays a marshaller; the carrier between them is PR-1c's own memo's choice, not this one's.** ⚠ **Why this half is delegable and the *rect* half above is not**: the rect answers §2's coupling **3×7** and is per *line*; the edges answer **no §2 pair at all** and are a per-*element* constant. Read §2's pair table — **no pair names edge *delivery***, i.e. no pair asks how an element-constant value gets from its derivation site onto `LayoutBox`. **Two of the ten pairs come close, and the disambiguation is stated rather than assumed.** (a) **3×7** — M4's own pair — reads "The box's rect must be a *content* span, **with edges carried separately**, or the border box double-counts": that names edge ***separation*** as a constraint on the per-line rect (keep the edges *out* of it), not a carrier for the constant; the clause is satisfied by any carrier whatever, which is exactly why it cannot choose one. (b) **2×6** names `group_key` ***delivery*** — the right shape — but it is routed to `#11-inline-box-decoration-splits`, not to an M-row here, so it does not reach this half either. The other eight (2×all, 1×2, 7×2, 1×3, 1×5, 1×4, 1×4 fallback-only half, 7×intrinsic) name an emit set, a collapse barrier, an advance/shaping break, an existence flip, a height source, a baseline source twice — the glyphless condition and the fallback-only one — and an intrinsic-size contribution. ⚠ **"The other seven" of "nine" until rev 34**, because G1's `1 × 4 (fallback-only half)` row was added to §2 in this revision and the enumeration behind the count was not re-run (round 26, Axis 5, Gate B); the conclusion is unchanged, since a *baseline source* is not a delivery shape either. ⚠ Two earlier draftings: the first reached the conclusion by a universal — "every pair names a rect, a predicate or an advance" — which §2's own table refutes (1×4 names a **baseline source**, 1×5 a **height source**, 1×2 a **collapse barrier**, and 2×6 a ***delivery***, the very shape the universal claimed no pair has); the second replaced it with "2×6 is the one delivery-shaped pair", a second universal that left 3×7's own edge-carriage clause unswept. The conclusion is unchanged; the argument is now the enumeration plus the 3×7 disambiguation rather than either universal (round 24 audit; round-24 gate). So M4's two halves differ on lifetime, producer, write site and consumer, and only the rect half carries an umbrella-owned coupling. (§2's "Each **pair** is answered by exactly one M-row **or by exactly one slot**" therefore does **not** converse: an M-row may carry a mechanism that answers no pair, and this is the one.) That asymmetry, not "altitude" in the abstract, is why two prescribed carriers were falsified in the same direction — each was an error about the *element-constant* half reasoned through the *per-line* pipeline the paragraph above had just established. <br>`resolve_box_model` runs **once per decorated inline**, at collect time, and its triple rides M1's marker payload (PR-1b). How it gets from the payload onto `LayoutBox` is PR-1c's interior mechanism, and this memo stops at the **invariants** that mechanism must satisfy: <br>(i) exactly **one** `resolve_box_model` call per decorated inline per pass — the marker payload is the single derivation site, so no second site can disagree with it; <br>(ii) `assign_inline_layout_boxes` performs **no `ComputedStyle` fetch** — its `boxes.rs:57` touch stays the `is_err()` guard it is today. A box-assign-time fetch would be a live read *after* the whole packing pass, against M1's collect-time clone (`inline/collect.rs:217`), and nothing pins the two to agree; <br>(iii) the three sets stay **separate** all the way to `LayoutBox`'s three **independent** fields — `padding` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:88`), `border` (`:90`), `margin` (`:92`) — consumed separately by `padding_box`/`border_box`/`margin_box`. ⚠ The ground is those three fields, **not** M5's disjunction: M5 is evaluated on the marker payload in `pack/inline_box.rs` (PR-1b), *upstream* of the carrier, so it would be satisfied even by a carrier that merged the sets. §5.1 M1's grounds column carries the binding ground; an earlier drafting of this clause cited M5, which does not reach here; <br>(iv) the value is an **element constant** and must arrive regardless of which line carries `InlineBoxEnd`; <br>(v) ⚠ **the set of entities receiving a `LayoutBox` must not grow beyond the domain `entity_bounds` already admits** — concretely, `assign_inline_layout_boxes`'s iteration domain stays `entity_bounds` under every carrier, and no carrier writes a `LayoutBox` outside that loop. ⚠ Stated at the **grant**, not at the `entity_bounds` entry: an earlier drafting forbade only "creating an `entity_bounds` entry", which option **C** can satisfy to the letter while still doing the harm — walk the item slice and write each `InlineBoxStart`'s edges straight onto that entity's `LayoutBox`, creating no entry at all and still granting a box on a phantom line. This is §2 invariant 3 (commit-on-content), which M4 **owns** through pair 3×7, so it is not delegable. Measured: every `entity_bounds` write today is reached from `flush_line`'s commit arm via `commit_aligned_entity_rects` (`pack/mod.rs:496`, in the function at `:468`, called at `:344`; `:404` is the dead non-persist arm — ⚠ neither line is lexically inside `flush_line`, round 24 audit), the discard arm (`:428`) touches `entity_bounds` not at all, and `assign_inline_layout_boxes` iterates `entity_bounds` (`boxes.rs:56`) skipping only entities with no `ComputedStyle` (`:57`) or an existing `LayoutBox` (`:62-64`) — **not** entities with degenerate bounds. A carrier that writes `entity_bounds` from the marker path in `pack()` runs per *item*, outside the commit/discard decision entirely, and would grant a box on a phantom line: that breaks §6 cell 6's "phantom ⇒ no box" and ships PR-1d's presence change inside PR-1c. §5.3's PR-1c bullet ("only on a line that already exists") is the statement (v) protects. <br>⚠ **"and no new parameter" is NOT on that list, and carrying it as though it were is what pre-decided the carrier.** It is a tie-breaker at most; stated as an invariant it silently excluded two of the three options below. <br>⚠ **Two measured facts constrain the choice, and the first of them falsifies the carrier earlier revisions prescribed.** (a) `commit_aligned_entity_rects` (`inline/pack/mod.rs:468`) is handed **no marker/text discriminator**: it drains `current_line_entity_rects`, a `Vec<(Entity, InlineLineRect)>` (`:121`, `:480`), and folds it into `merged: Vec<(Entity, f32, f32, f32)>` (`:479-487`) with no edges slot — so "written on the `InlineBoxEnd` path in `commit_aligned_entity_rects`" names a path that function cannot tell from any other. (b) The fold collapses one entity's per-line entries **before** the `entity_bounds.entry()` that consumes them (`:496`; the file's only other `entry()` is `:404`, the dead non-persist arm — ⚠ an earlier drafting cited `:488` as an `entry()` site, but `:488` is the `for (entity, …) in merged {` head that walks the fold's output, round-24 gate), so two producers on one line cannot order against each other at all; the surviving ordering obstacle is **cross-line** — `or_insert` fires on the first line (`:505`) while `InlineBoxEnd` may be on the last. ⚠ State (a) as a **timing** fact, not a missing-field one: that function runs at **flush, per line**, while markers are consumed in `pack()`, per *item*. "No discriminator" invites adding one, which would push an element constant through `InlineLineRect` and the `merged` tuple and re-bundle the two halves this row just separated. <br>⚠ **Three options, not two, and the reopening is not a re-prescription.** (**A**) widen `EntityBounds` (`inline/pack/boxes.rs:30-41`); (**B**) a second `Entity → (EdgeSizes, EdgeSizes, EdgeSizes)` map threaded to the box-assigner; (**C**) hand `assign_inline_layout_boxes` the `&[InlineItem]` slice and read the marker payload there — `items` is a local `Vec<InlineItem>` (`inline/mod.rs:153`) that every later use only *borrows* (`:161`, `:179`, `:190`, `:192`, `:200`, `:248`, `:255` — the enumeration is `grep -nE '\bitems\b' inline/mod.rs` minus the binding at `:153`; an earlier drafting omitted `:161`'s `items.is_empty()`, an enumeration standing behind an "every", round 24 audit), and `PackItem` has no lifetime parameter (`pack/items.rs:18-35`, `item_index: usize`), so it is live and immutably borrowable at the assign call (`:380`). Costs, stated symmetrically: **A** must reach its write site *through* the fold, so by (a) it is not one widening but three — `InlineLineRect`, the `merged` tuple (`pack/mod.rs:479`) and `EntityBounds` — plus a rule for what an edges slot means when the fold merges two entries for one entity; **and its other implementation, writing `entity_bounds` from the marker path in `pack()`, is barred outright by (v)**. **B** adds one parameter and one structure, satisfies (v) because the assign loop's domain stays `entity_bounds`, and survives `fragment.rs:68`'s `retain` untouched. **C** adds one parameter and no *signature* structure, and satisfies (iv) structurally — the item stream has no line concept at all. ⚠ **Its two real costs are not in the parameter list.** (1) It puts a marker read and a per-entity edge derivation inside `assign_inline_layout_boxes` (`pack/boxes.rs`), which is a **second site reading off a marker** — §5.2 designates `pack/inline_box.rs` the one such site, and M4's own heading says the box-assigner stays a marshaller, so C requires amending one or the other rather than slotting in free. (2) `assign_inline_layout_boxes` iterates a `HashMap` in arbitrary order (`boxes.rs:56`), so reading the slice per entity is either an O(entities × items) rescan or an internally-built entity→edges map — i.e. **option B's structure, moved out of the signature and into the function**. "No structure" is a property of the parameter list, not of the mechanism. ⚠ What is withdrawn is the *prescription*, not replaced by a new one: an earlier revision rejected **B** on the ground that "the carrier exists", meaning `EntityBounds`, and (a) falsifies that premise. PR-1c's memo weighs all three. <br>⚠ Separately withdrawn: the rejection ground earlier revisions gave against `EntityBounds` itself ("`or_insert`/`and_modify` would not re-set an element-constant on a multi-line inline") — `entity_bounds` is keyed **per entity, accumulated across lines** (`:496-511`), which is exactly what an element constant wants. | `assign_inline_layout_boxes` writes bounds into `LayoutBox.content` (`:81`) and `border_box() = content + padding + border`, so an edge-inflated rect double-counts — which is why the border area is derived at the write rather than stored in the accumulator both consumers read. Reading the end-cursor before the end advance is what keeps the end side from double-counting too. The flush-time **emit** is what a rebase-only version misses: `InlineBoxEnd` has not run at line N's flush, so line N's fragment would simply be lost. ⚠ **The carrier ground this column used to carry is withdrawn, not weakened**: "a `&HashMap<Entity, (EdgeSizes, EdgeSizes, EdgeSizes)>` parameter would add a *second* structure beside `entity_bounds` … so the carrier exists and threading a map is the option that buys nothing" rests on `EntityBounds` being writable from the marker, and its write site cannot see the marker (Decision, fact (a)). Nothing replaces it **here** — the choice moves to PR-1c's memo with the measured constraints, which is the altitude a per-PR interior belongs at. `resolve_box_model` (`helpers.rs:116`) is pure and `pub`-exported (`lib.rs:25`, with an existing cross-crate caller at `crates/layout/elidex-layout-grid/src/lib.rs:177`), so calling it once and carrying the result is free of any re-entrancy concern. ⚠ CLAUDE.md's side-store→component rule does **not** reach any of the three options, and the ground is *recorded*, not argued: [[ecs-native-side-store-audit-2026-05-21]] already adjudicated `elidex-layout` **clean** on this axis — "layout 計算の局所 scratch (children / rows / pool / run) のみ". `entity_bounds` is exactly that: a field of a stack-allocated `LinePacker` (`inline/mod.rs:251`), initialised empty (`pack/mod.rs:178`) and dropped when the IFC pass returns, whose *destination* is the `LayoutBox` **component** (`boxes.rs:88`, `dom.set_layout_box`). The rule's subject is a persistent entity-keyed registry and all three payoffs it names — SameObject via component get, GC as one query, despawn cleanup — are persistence properties with no meaning for a value that does not outlive one call. ⚠ Option **B** is *literally* the `HashMap<entity, _>` shape the rule names, so the exemption is stated at that shape and not only at the rule's subject; and `assign_inline_layout_boxes` already takes `entity_bounds: &HashMap<Entity, EntityBounds>` (`boxes.rs:50`), which the rule would condemn on `origin/main` if it reached.  ⚠ **There is no second derivation site to keep in step**, which is the point: percentage edges (cell 4) are resolved once against `layout_inline_context_fragmented`'s own `containing_inline_size` (`inline/mod.rs:144`, reaching `collect_inline_items` at `:154`), and every consumer reads that one result. That holds because the derivation happens **once, at collect time**, whatever carries the result afterwards — it is invariant (i), and it does not depend on `assign_inline_layout_boxes`'s parameter list. |
| **M5** | What makes a line non-phantom, and what else commits? | **The clause-3 predicate is a function of the marker's edges, not a per-PR constant**, and it is a **disjunction, not a sum**: `contributes_content = has_inline_axis_edge(marker)`, true iff **any one** of the three `LogicalEdges` (padding, border, margin — each converted separately by `LogicalEdges::from_physical`) has a non-zero `inline_start` or `inline_end`. **M5 owns this predicate, and it lands in PR-1b** — M3's shaping break and hang gate are PR-1b deliverables and both call it, so the predicate cannot wait for PR-1d. What PR-1d adds is the one-argument *substitution* below, not the predicate. M1's emit test (any side) is a different test on the same payload and stays in PR-1a. Evaluated per marker as a **free function over the marker's `InlineItem` payload**, in `pack/inline_box.rs` beside the stack — deliberately *not* an `impl LinePacker` method, because the pre-pack gate at `inline/mod.rs:200` must call it too and runs before `LinePacker::new` (`:251`). One derivation site, two callers: the gate holds `&InlineItem` directly, the packer resolves its `PackItem`'s `item_index` into the same slice (M1). ⚠ The gate's escape is **qualified**, not a bare "marker escapes beside `Atomic`": an unqualified escape would let a marker that does **not** satisfy clause 3 — a `padding-top`-only box — carry a line by itself, which is exactly what M1, §1.2's clause-3 row and cells 5/6 forbid. What the `:200` early return is doing today is **not** clause enforcement at all: it is an engine **measurability** guard (no usable font ⇒ nothing can be measured ⇒ return `line_count: 0`), and §6 cell 12d records that as a divergence, since clause 3 is font-independent. PR-1d lifts it **only where clause 3 licenses it** — i.e. behind `has_inline_axis_edge` — which is why the escape carries a predicate rather than matching on the variant. ⚠ An earlier drafting grounded this on "font-less text commit[ting] a line that §1.2 says stays phantom": §1.2's clause 1 is `contributes_content`, "a pure text predicate with no font dependence" (`pack/mod.rs:556`) by this row's own words, so a line whose text is font-less is **not** phantom under §1.2 — the attribution ran the wrong way (round 24 audit). In PR-1b the marker path passes a constant `false` (geometry only); **PR-1d replaces that constant with this expression** — that one-argument substitution *is* the existence flip. Once a line is non-phantom, css-inline-3 §2.3's "the line box **and its in-flow content**" applies: every entity on it commits through the existing seam (`:210`), none withheld. | A literal `true` for PR-1d would keep a `padding-top`-only inline's line alive — contradicting §1.2, M1 and cells 5/6. Emit and existence are different predicates over the same payload, so the existence one needs its own site. **Disjunction, not sum**: css-inline-3 §2.3 lists "non-zero **inline-axis** margins, padding, or borders" — three separately-named quantities, so `margin-left:-10px; padding-left:10px` — which sums to zero — still keeps the line. This is the one place the three sets must stay separate; M3's *advance* and M4's *content offset* both take the sum, because geometry adds up and a negative margin really does pull content back. §6 cell 3b pins the cancelling pair. |
| **M6** | Height of a line kept only by decoration | `InlineBoxStart` carries the marker payload's `line_height` **and `font_size`** — two of the five font-and-height fields M1 names, not a "font identity" gloss, because M3's shared core takes `current_line_height = max(block_advance)` with `block_advance` following the packer's existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`), so **both** arms of that convention are payload reads and a payload carrying only `line_height` cannot serve the vertical one (round 26, Axis 2). **The memo does not claim css-inline-3 §5.3/§2.2 conformance**: the block container's **root inline box** (§1.1) is unimplemented, so the line's height floor is missing. The text path already diverges identically — `<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today, with no marker involved. New slot **`#11-inline-root-inline-box`**, pre-existing class; §6 cell 23 pins the divergence so it stays distinguishable from a bug. | The direct authority is `body css-inline-3 line-layout` (css-inline-3 §2.2) Note: "Empty inline boxes still have margins, padding, borders, and a **line-height**, and thus influence these calculations just like boxes with content." css-inline-3 §5.3 defines the strut per *box* and does not address the line-level question. Disclosing a pre-existing divergence the new code depends on is the §4.3 pattern. |
| **M7** | Which line gets a strut baseline | `InlineBoxStart` records a tentative `current_line_box_baseline: Option<f32>` from the box's first-available-font metrics via `FontDatabase::query(families, weight, style)` + `font_metrics(id, font_size)` (`crates/text/elidex-shaping/src/database.rs:60`, `:101`) — **the payload's `families` / `font_weight` / `font_style` / `font_size`, named by M1 rather than glossed as "font identity", because all four are arguments of those two calls** (round 26, Axis 2) — keeping the `!is_vertical` guard (`:575`). `flush_line` promotes it into `first_baseline` **inside** the `if self.any_rendered_content` arm (`:210`) — never on a suppressed line — and only when **`first_baseline.is_none()` and the line's occupancy never reached the `RenderedText` rung** (the ⚠ in the Grounds column derives the second conjunct: `first_baseline.is_none()` alone does not answer "did a text segment the engine counts as rendered content land here" when no font is usable). ⚠ **The rung is named `RenderedText`, not "glyph-bearing", and the name change is not cosmetic** (round 26, Axis 2): M3 derives it from `(FlowMember::Text(_), contributes_content)`, and `contributes_content` is **not** a glyph predicate — under `pre`/`pre-wrap` it is `!text.is_empty()` (`pack/mod.rs:556-558`), so a lone preserved segment break reaches the rung on a line that renders no glyph. The derivation is unchanged; what changes is that the rung is now named for the predicate that computes it. The Grounds column below states why mirroring `contributes_content` here is *correct* and not merely convenient. ⚠ **That conjunct is not a second bool. It is one more state in M3's monotone `LineOccupancy` ordering** — the line carried *no* occupancy, *non-text* content (an atomic, a marker's edges) or *rendered text* — raised by the **same `note_line_occupancy`** M3 makes the single writer, from **the structural discriminator `place_item` already holds**: M3's derivation of the occupant inside `place_item` from `member: FlowMember<'_>` (`:687`) and `contributes_content` (`:686`) — `(FlowMember::Text(_), true)` ⇒ this rung, every other pair ⇒ `Content`, the marker path ⇒ `BoxEdgeOnly`. PR-1d's whole addition to M3 is giving that first arm a rung above `Content` to raise to. The promote reads that state; nothing reads a flag. ⚠ **An earlier drafting** (rev 33) is withdrawn, and it failed the same way the bool it replaced did (rev-33 gate; ⚠ the marker used to read "What rev 33 wrote here", which was self-referential in the revision that wrote it — round 26, Axis 4 — and the sibling withdrawal below already carried the right form). It said the rung is raised "at the one site where the value means glyph text: the `PackItem::Text` arm's own `contributes_content` (`pack/mod.rs:556-568` → `:597`)". But `place_item` has **exactly two** call sites — `:591` from the `PackItem::Text` arm, passing that predicate at `:597`, and `:652` from the `PackItem::Atomic` arm, passing a literal `true` at `:658` — and they feed **one** parameter (`:686`) and one `note_line_occupancy` call. "Reached only from the Text arm's call" therefore names **no condition the code can test**: at the raise site the two calls are indistinguishable, which is precisely the under-determination of the rev-32 bool, one level down. The fix is not a new parameter or a caller contract but the discriminator that was already in the signature beside it — `FlowMember`, an enum whose variants (`inline/pack/items.rs:49-58`) *are* the Text/atomic distinction. **The state therefore adds no new field to `flush_line`'s per-line reset block (`:432-439`)** — M3 already resets the occupancy there and that reset covers it, so **only the tentative** joins the block. The ground is the encoding, not the fact: a second bool would carry an implicit `any_glyph_text ⇒ any_rendered_content` invariant across three write sites, which is exactly the argument M3's own row makes against an `on_line`/`content_on_line` pair, and CLAUDE.md's *one issue, one way*; and the split encoding is what made the raise-site defect below possible, because a raised **ordering** cannot be claimed by the marker path — that path raises to the *non-text* state by construction. ⚠ **An earlier drafting** (rev 32) instead added a separate per-line `any_glyph_text: bool` "raised by `\|= contributes_content` at the `place_item` site", and said **two** fields join the reset block. Both are withdrawn, and the raise site is why: M3 moves `any_rendered_content \|= contributes_content` (`:698`) **into** `note_line_occupancy`, whose **second caller is the marker path**, and M5 has PR-1d replace that path's constant `false` with `has_inline_axis_edge` — true for every decorated inline. A decoration-only line would have raised the flag, `!any_glyph_text` would be false, the tentative would never promote, and M7's whole mechanism would be **unreachable code**. The other reading is no better: `contributes_content` is a `place_item` **parameter** whose atomic caller passes a literal `true` (`:658`), so an `<img>` would have counted as glyph text (round 25, Axis 2). ⚠ **The state is a fourth `LineOccupancy` variant, decided here and not delegated** (round 26, Axis 2). An earlier drafting left it open — "a fourth variant or a sibling monotone enum written by the same function is mechanism for PR-1d's own memo" — and the memo's own three statements already foreclose the second branch: (i) M3's signature takes `occupant: LineOccupancy` and M7's rung is passed as *that* argument, so a sibling enum is not a value the single writer can accept; (ii) §8's PR-1d DoD closes with "No field joins the `:432-439` reset block for this gate — M3's reset covers the widened state"; and (iii) §5.4 records "the widening is PR-1d's and adds no field". A sibling enum **is** a second per-line field and needs its own reset, so it fails (ii) and (iii) and cannot be handed to the same writer under (i). An open encoding whose two branches differ in *correctness* is a decision surface, not a delegation — CLAUDE.md's *ideal over pragmatic*. What this memo decides is therefore **one monotone-raised ordered state — a fourth `LineOccupancy` variant, `RenderedText` — one writer, never a bool pair**. ⚠ A previous drafting also stated the condition as `first_baseline.is_none()` alone and "the field" as one, leaving the Decision asserting the rule its own Grounds falsify while §6 cell 24 appeals to this Decision as the design authority (round-24 gate). ⚠ **Scope: the occupancy state is per *line* while `first_baseline` is per *IFC* (`pack/mod.rs:125`, initialised at `:187`, set at `:586`/`:631`, and deliberately **not** in the `:432-439` reset block), so this rule binds within a line and not across them.** With a font-less text line 1 and a decoration-only line 2, `first_baseline` is still `None` at line 2's flush and the tentative promotes, giving the IFC a first baseline from line 2's box. **That is the intended behaviour, not a residual**: §1.5 makes the strut a property of the *box*, line 2 genuinely carries no glyphs, so its box's strut is the only baseline that line can offer — and what M7's rule exists to prevent is a strut *displacing* a line's own glyph baseline, which line 2 has none of. The thing line 1 leaves undone is the font-less line's own missing baseline, which no signal in the engine can supply and which this program does not create (round 25, Axis 2). §4 enumerates that block's current contents; **no site states a running total** — a count restated away from the enumeration it summarises drifts from it. | §1.5: a strut exists only for a glyphless box; css-inline-3 §2.2 owns the line-level composition. ⚠ **Why `contributes_content` is the right predicate for the rung, and not merely the one at hand** (round 26, Axis 2). The question the second conjunct asks is "did a text segment that *would have* set this line's baseline land here?" — and the engine's own answer to "does this segment set the baseline" is `contributes_content`: the capture at `:575` is gated on exactly `if contributes_content && self.first_baseline.is_none() && !is_vertical`. Mirroring that gate is therefore **consistent by construction**, not an approximation of a glyph test; a glyph predicate would be a *second, different* answer to a question the capture already answers, which is the shape M3's row and CLAUDE.md's *one issue, one way* both refuse. ⚠ **The `<pre>\n</pre>` corner is inherited, not created.** Under `pre`/`pre-wrap` `contributes_content` is `!text.is_empty()` (`pack/mod.rs:556-558`), and the code's comment at `:547-551` names that very markup, so a lone preserved segment break raises `RenderedText` on a line with no glyph — and the same segment already passes `:575` today and would already claim the line's baseline if its font resolved. The rung inherits the engine's existing notion of *a segment that gives the line a baseline*; it introduces no divergence of its own. No second flag: the text arm sets `first_baseline` at pack time (`:586`, under the guard at `:575`; ⚠ an earlier drafting cited `:575` for the *set* — round-24 gate), so `is_none()` at flush already answers "did a rendered-text segment land here or earlier". Traced against all three orderings. `query`+`font_metrics` rather than `measure_text`, because a glyphless box has no string to shape and the metrics are string-independent anyway (`elidex-shaping/src/measurement.rs:55`). ⚠ **Known residual, and it is the one divergence this program *creates*** — today no tentative baseline exists, so the corner cannot occur: if a line's text has no usable font, `measure_text` returns `None`, `first_baseline` stays `None`, and a co-resident box's tentative promotes on a line that does have glyphs. ⚠ It falsifies this row's own "No second flag" ground — `first_baseline.is_none()` does **not** answer "did a rendered-text segment land here" when no font is usable. **Fixed in PR-1d, not deferred** — and ⚠ **an earlier revision folded it into `#11-inline-root-inline-box` on a ground that does not hold**: css-inline-3 §5.3's "only glyphs from fallback fonts" needs **per-glyph provenance** (glyphs are present; which font produced them), while this residual needs only **per-line rendered-text presence** — ⚠ stated as "per-line glyph presence" until rev 34, which named the wrong predicate on this side of the contrast while the fold argument turns on the *other* side (round 26, Axis 2); the argument survives the correction intact, because that slot's subject is per-*glyph* provenance and this residual's signal is `contributes_content`, which is not a glyph fact at all. They share the word "font" and nothing else, and neither of that slot's trigger disjuncts — line-height correctness, a compat-survey hit — can fire for a baseline defect about whether a line carried rendered text, so the fold would have parked a fixable bug behind a condition that never arrives. ⚠ **That slot carried a third disjunct, font-fallback provenance, when this argument was first made, and it is struck in rev 33** (round 25, Axis 3): it existed only to make §8's *own* fold — of css-inline-3 §5.3's fallback-only strut condition — surface there, and §8 withdraws that fold to `#11-inline-fallback-font-strut` on this row's identical mechanism-mismatch ground. The disjunct's removal does not weaken the argument here: it was never one that could fire for rendered-text *presence* either. The signal it needs **already exists and is font-independent**: `contributes_content` (`pack/mod.rs:556`), which §5.1 M5 itself calls "a pure text predicate with no font dependence". `measure_text` returns `None` on font *resolution* failure — `db.query(…)?` and `db.font_metrics(…)?` at `crates/text/elidex-shaping/src/measurement.rs:54-55` (⚠ the signature is `:49-53`; an earlier drafting cited it as `:49-51`, which stops two lines short of the return type) — an availability outcome — not a provenance one. So PR-1d does **not** add a second flag beside the tentative: it adds **one more state to M3's `LineOccupancy` ordering**, raised by that ordering's single writer from the occupant `place_item` derives from its own `member`/`contributes_content` pair, and the promote reads the state (Decision). The "second flag" this row gave up on is the right *fact* in the wrong *encoding* — a bool would need an implicit `⇒ any_rendered_content` invariant across three write sites, the shape M3's row refuses for `on_line`/`content_on_line`, and it would have had no raise **condition** that means rendered text once M3 owns `:698` — `contributes_content` alone cannot supply one, since `place_item`'s atomic caller passes a literal `true` for it (`:658`). **The concession is the argument for fixing it**, and CLAUDE.md's *TODO 先送り禁止* points the same way. §6 cell 24 asserts the fixed behaviour, not an accepted divergence. |
| **M8** | Intrinsic sizing | **`max_content_inline_size` only** (`inline/measure.rs:44`): each marker adds its inline-axis edge sum to the running total, matching that pass's existing `total +=` shape. **`min_content_inline_size`'s accumulator is out of scope and gets a slot** — `#11-inline-min-content-box-edges` (own deferral): the pass is `max_word = max_word.max(m.width)` per word per item (`:15-37`) with **no running candidate and no cross-item joining**, so a box's edges have nothing to attach to; giving them one is an accumulator restructure of a pass that also already ignores run boundaries (`a<b>b</b>c` yields `max(\|a\|,\|b\|,\|c\|)` today, never `\|abc\|`). Trigger: PR-1d landing, or any shrink-to-fit correctness work. Re-eval: 2026-11-01. **PR-1b**, cell 25 scoped to max-content. Grounds: css-sizing-3 §5.2 defines the *contribution* but notes it "does not define precisely how to determine these sizes", so splitting the two passes is an engine choice, not a spec deviation. | Any rule that adds a box's edges to a *running candidate* names an accumulator this site does not have. Scoping to the pass that *can* host it, and slotting the one that cannot with its real cost stated, is the honest split — the alternative ships a DoD cell that cannot pass. |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`) against `containing_inline_size`. |
| **no cite-sweep layer — this program sweeps nothing pre-existing** | §3.1's classes are **pre-existing defects this program merely *found***, and a sweep of them is slot work, not umbrella work (`#11-inline-spec-cite-misattribution`, §9). The single exception is `pack/mod.rs:726`, which **PR-1b corrects because PR-1b changes what that comment documents** — it adds the css-text-3 §7.3 boundary break beside the surviving css-text-3 §5.5 intra-word coalescing. The rule: *a program fixes the citations its own change makes wrong, and hands pre-existing classes to whoever owns them.* Round 16 measured why a coordinate list cannot be that owner — the CSS 2 `§9.2.2.1` misattribution alone has hits at `pack/mod.rs:107`, `:200`, `:425`, `:553` and three test files, and "one border-box fragment per line" recurs at `boxes.rs:90-91` and four more sites. |
| `crates/layout/elidex-layout-block/src/inline/styled_run.rs` | **`InlineItem` (`:9`) — where M1's two marker variants are declared**, beside `Text`/`Atomic`/`Placeholder`. `StyledRun` (`:43`) itself is unchanged; M1 explicitly does not widen it (§5.3 Rejected). |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair, with the payload M1 specifies, using the `parent_style` already in scope. Gains **one** new parameter: `containing_inline_size` on `collect_inline_items` (`:136`) / `collect_inline_items_inner` (`:194`). `root_horizontal` (`:211`) is unchanged. Every `collect_inline_items` caller — enumerated by `grep -rn 'collect_inline_items(' crates/` minus the definition — takes the new argument: `inline/mod.rs:154` (has the value), `inline/measure.rs:22` and `:51` (the intrinsic passes, where a containing inline size is definitionally unavailable — see §9), and the test helper `inline/tests/mod.rs:17`. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get `PackItem` forms carrying `item_index` only (M1); they never become `FlowMember`s. |
| `crates/layout/elidex-layout-block/src/inline/pack/mod.rs` | `LinePacker` line state. **M3's `note_line_occupancy` lives here**, beside `place_item` (`:679`), its first caller — and it is the **single writer** of the line-occupancy ordering, M7's `RenderedText` rung included: the two raise sites are `place_item`'s call and the marker path's. `place_item` **derives** the occupant it passes from the two arguments it already has — `contributes_content` (`:686`) and `member: FlowMember<'_>` (`:687`) — raising the `RenderedText` rung only for `(FlowMember::Text(_), true)` and `Content` for every other pair, its own literal-`true` atomic call (`:652` → `:658`) included; the marker path passes `BoxEdgeOnly` and so raises no higher than the non-text state (M5, M7). ⚠ **Not "the Text arm's call is the only site whose value means glyph text"** — that is what this row said until the rev-33 gate, and `place_item`'s two callers (`:591`, `:652`) feed one parameter and one `note_line_occupancy` call, so a *call site* is not a condition the raise can test (M7's Decision carries the withdrawal). No other function writes the ordering. `flush_line` (`:209`) **calls** the open-box hook that `pack/inline_box.rs` owns (M4, **PR-1c**), promotes M7's tentative baseline inside its `:210` arm and **reads the occupancy state there** (**PR-1d**), and grows its per-line reset block by exactly **one** field per PR — M3's `line_occupancy` in **PR-1b** and M7's tentative in **PR-1d**; M7's `RenderedText` rung is a widening of M3's field, so the `:432-439` reset it already has covers it and no second reset is added. The fabricated shaping citation at `:726` is rewritten by **PR-1b** — the one cite this program corrects, because PR-1b changes what that comment documents (§9). |
| `crates/layout/elidex-layout-block/src/inline/mod.rs` | The IFC entry point. Sites this program writes, by PR: `collect_inline_items`'s call (`:154`, PR-1a, one argument); `items.is_empty()` (`:161`, held in PR-1a, flipped in PR-1d); the `any_font` closure's exhaustiveness arm (`:192-199`, PR-1a, behaviour-neutral); the **outer early-return condition** (`:200`, PR-1d, gains an escape for markers **that satisfy `has_inline_axis_edge`** (M5), beside the existing `Atomic` one); `assign_inline_layout_boxes`'s call (`:380` — ⚠ **whether this call site changes at all depends on the carrier PR-1c's memo picks** (M4): unchanged if the edges ride a widened `EntityBounds`, one added argument if they ride a second entity-keyed map **or** the `&[InlineItem]` slice (M4's three options). This memo does not decide it, so it does not promise the call is untouched either); and — ⚠ **not a write but a path-selection consequence** of the `:161`/`:200` flips (`154bac3f`; `:162`/`:202` at `22de3078`) — §7's `clear_inline_flows` gating (`:637` in the `154bac3f` frame; `inline/reconcile.rs:417-418` at `22de3078`, already `!env.is_probe`-gated: none of PR-1a–1d edits that file; the prereq #511 did, see the dead-arm row). The dead-arm prereq PR (#511, landed) wrote further sites here; they have their own row below. |
| the dead-arm prereq PR's surface | `flush_line`'s `else` arm and its unmerged rect loop (`pack/mod.rs:393-421`) **and** the `inline/mod.rs` half the reachability argument kills: `persist_candidate` (`:239`), `flow_align`'s `Option` construction (`:240-251`), `persist_flow`'s now-redundant conjunct (`:322`) and the comments that explain the two-path model (`:227-230`, `:309-320`, `:329-330`). Listed as a layer of its own because the PR spans two files, which no other row does, and because every one of its six `inline/mod.rs` items lies **above** seam 3 — the fact §8 uses to conclude the two **`elidex-layout-block`** prereqs are independent rather than ordered (the third, the predicate prereq, is a different question — §9 hands over its touch set, so this memo concludes nothing about it). **Landed as #511 (`22de3078`, 2026-09-07).** ⚠ **What landed exceeds this enumeration** (recorded as the delta, per [[feedback_plan-ratified-surface-is-a-design-change]]): the pre-push gate found the same class one level down — with the pre-gate gone, `do_carrier` is definitionally `!persist_flow`, so `reconcile_flows`'s two-`bool` interface re-encoded the deleted third state and its `persist_flow \|\| do_carrier` guard was a tautology — and #511 collapsed it to **one bit**: `reconcile_flows` takes `persist_flow` alone, the caller derives `do_carrier = !persist_flow`, the guard and the redundant `do_carrier` conjuncts are gone. That edit lives in `inline/reconcile.rs`, the seam-3 module, so this row's "two files" is superseded — the landed set is whatever `git show --stat 22de3078` lists (no figure carried here), and the "above seam 3" independence argument held for the *planned* surface only. ⚠ Whether the landed delta disturbs a later obligation is **round 20's question, not this row's** ([[feedback_plan-ratified-surface-is-a-design-change]]: the collapse was applied at #511's pre-push gate and documented after — the class that rule exists to route back through plan-review). What this row can measure: the obligation sweep `grep -n 'do_carrier\|eleven' <memo>` returns landing-record sites only (this row and §8's ordering record — two lines) and no DoD; and the delta **fired the successor slot's disjunct 3** (its trigger exempts no prereq there — disposition in the slot memo and §10's last row). The successor slot's own half (`reconcile_flows`' signature, SoT) counts **ten** parameters and **two** adjacent `bool`s from `22de3078` on. PR-1a re-anchors against its actual base, which includes `22de3078`. |
| `crates/layout/elidex-layout-block/src/inline/whitespace.rs` | `collapse_inline_whitespace` (the `fn` is `:27`) — M2's transparent arm, which joins the per-item `match &mut items[i]` whose head is `:41`. ⚠ Both coordinates name the same function and different things; earlier revisions wrote `:41` here and `:27` in §3 with neither saying which (round 25, Axis 2). |
| `crates/layout/elidex-layout-block/src/inline/measure.rs` | `max_content_inline_size` (`:44`) — M8's contribution. `min_content_inline_size`'s **accumulator** — `max_word` at `:23`/`:31` — is not touched (see `#11-inline-min-content-box-edges`); the function itself (`:15-37`) is, because its `collect_inline_items` call (`:22`) takes the new argument like every other caller. |
| `crates/core/elidex-plugin/src/layout_types/boxes.rs` | `InlineClientRects` (`:106-111`) — the cross-crate contract type. **Semantics not changed by this program**: "per-line client rects … single-line inlines use `LayoutBox.border_box()`" stays true, and PR-1c makes the fallback half *correct* rather than redefining the component. ⚠ Its docstring nonetheless cites **"CSSOM View §5"** for `getClientRects()`; cssom-view-1 §5 is *Extensions to the Document Interface* and both `getClientRects()` and `getBoundingClientRect()` are on `Element`, i.e. cssom-view-1 §6. §3.1 defines that class **by a concept grep** and `#11-inline-spec-cite-misattribution` owns it (§9) — this row routes the docstring, it does not vouch for it. The multi-fragment redefinition belongs to `#11-inline-box-decoration-splits`, which owns the docstring edit too. |
| `crates/core/elidex-render/src/builder/slice.rs` + `walk.rs:296` (render) and `crates/layout/elidex-layout-block/src/block/mod.rs:369-372` (layout) | **Existing `box-decoration-break` implementations, and why this program does not extend them.** `walk.rs:296` reads `style.box_decoration_break`; `slice.rs`'s `break_edges` (`:22`) computes per-fragment slice geometry for **column** fragments and takes `(i, n, wm)` with **no `direction`**, its own docstring saying "the inline-axis edges are never 'at a break'"; `block/mod.rs:369-372` handles `Slice`/`Cloned` for **block** fragments off `block_start_pb`/`block_end_pb`. All three are block-axis, while a line break is an **inline-axis** break whose surviving edge is set by the *parent's* inline progression direction (css-break-3 §5.4) — a different axis and a different direction source, so none generalises as written. ⚠ Two crates, and the dependency runs **render → layout** (`elidex-render/Cargo.toml` depends on `elidex-layout-block`, not the reverse), so a layout-side producer cannot call `break_edges`, which is `pub(super)` in `elidex-render::builder`. `#11-inline-box-decoration-splits` owns the inline case and must first decide **which layer produces** the attribution, since it has both a paint consumer and a CSSOM consumer. |
| `crates/core/elidex-plugin/src/logical.rs` | `LogicalEdges::from_physical` (`:186`) + `WritingModeContext::new` (`:27`) — used at the *point of derivation* (M1), never as a round trip. |
| `crates/layout/elidex-layout-block/src/inline/pack/inline_box.rs` (NEW) | **The marker's own derived facts and the stack that holds them**: push on `InlineBoxStart`, pop-and-emit on `InlineBoxEnd`, the flush-time hook `flush_line` calls (emit + rebase, M4), and M5's `has_inline_axis_edge` plus the inline-start/inline-end sums M3 and M4 consume — one derivation site for everything read off a marker. ⚠ **Two shapes in one module, deliberately**: the stack and its `flush_line` hook are an `impl LinePacker` in a sibling module (the idiom `pack/fragment.rs:10` already uses), while `has_inline_axis_edge` and the sums are **free functions over the marker payload**, because the pre-pack gate at `inline/mod.rs:200` calls them before any `LinePacker` exists (M5). ⚠ **The file is created by PR-1b and grown by PR-1c**: PR-1b authors the free functions (M5's predicate and M3's sums), PR-1c adds the stack and the hook. §9's 700–800 band therefore applies to it across both PRs, not at one of them. The line-state core (M3) and the baseline promotion (M7) stay with their existing owner in `pack/mod.rs`: cohesion, not `pack/mod.rs`'s line count, decides the split. |
| `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs` | `assign_inline_layout_boxes` (`:48`) — writes `LayoutBox.content` from bounds and, per M4, the three edge fields, **with no `ComputedStyle` fetch** (`:57` stays the `is_err()` guard it is today). ⚠ **Which structure carries the edges to this function — a widened `EntityBounds` (`:30-41`), a second entity-keyed map, or the `&[InlineItem]` slice — is PR-1c's plan-memo's decision, not this memo's** (M4 states the four invariants it must satisfy and the two measured facts that constrain it); this row therefore names the *write target* and not the carrier, and does not promise the signature is unchanged. The `InlineClientRects` write (`:102-126`) is untouched either way: both keep content spans, and the per-fragment border-area inflation is `#11-inline-box-decoration-splits`'s (M4). |
| `crates/layout/elidex-layout-block/src/inline/tests/mod.rs` | The test harness. `collect_styled_runs` (the whole `fn`, `:11-25`) is a `collect_inline_items` caller — the call itself is `:17`, the coordinate the `collect.rs` row above cites — whose `filter_map`'s `match item` (`:20-23`) gains marker arms in PR-1a — but it `filter_map`s to `Vec<StyledRun>`, so those arms are `None` and it **cannot observe a marker**; PR-1a adds a sibling returning the `InlineItem`s (cell 6b). Also the `mod decorated_inline;` declaration and **PR-1a's `setup_inline_test` (`:54`) change** giving the harness a deterministic way to force `any_font == false` (cell 12d). |
| `inline/tests/decorated_inline/{stream,advance,geometry,existence}.rs` (NEW) | The four per-PR test modules §6 routes cells to. |
| `elidex-render` tests | PR-1c's paint assertions for cell 13(c) — the background-colour and border rects that `paint/mod.rs:68`/`:382` derive from `border_box()`. The crate depends on `elidex-layout-block` (`crates/core/elidex-render/Cargo.toml`), so a cell there can run layout. §7 lists the family; §8 makes dispositioning it a PR-1c DoD item. |
| `elidex-layout-block/src/inline/tests/` — **not `elidex-dom-api`** | Cells 17b/17c/17d. ⚠ `elidex-dom-api` has **no** dependency on any layout crate and no `[dev-dependencies]` at all, so a cell there cannot run inline layout — its existing `layout_query.rs` tests hand-insert `LayoutBox` literals, which would assert the marshalling and nothing about M4's producer. Every existing `InlineClientRects` assertion already lives under `elidex-layout-block/src/inline/tests/inline_flow/`, and that is where M4's two channels are jointly observable. §8's PR-1c `getBoundingClientRect` obligation is discharged the same way — against the `LayoutBox` border box the DOM API reads — not by a cell in `elidex-dom-api`. |
| the seam-3 module — `inline/reconcile.rs`, created by #508 (`7e256029`) | `layout_inline_context_fragmented`'s reconcile block, moved out of `inline/mod.rs`. §7's `clear_inline_flows` gating lives here (`:417-418` at `22de3078`, already `!env.is_probe`-gated) — a **path-selection consequence** of PR-1d's flips in `inline/mod.rs`, **not an edit: none of PR-1a–1d writes this file** (`awk '/^## §6\./,/^## §7\./' <memo> \| grep -c reconcile` → 0; §8's PR-1d DoD → 0; the prereq #511 *did*, this table's dead-arm row), so `:637` is a pre-move coordinate and the successor slot's disjunct 3 is fired by **no PR after #511** (§10's last row; earlier draftings of this row said "PR-1d fires it" and then "no umbrella PR fired it" — #511 did). |
| `elidex-shell` tests — **the only site where a layout producer and a DOM-API reader are jointly observable** | §8's PR-1c end-to-end clause. Ground: `elidex-shell` depends on **both** `elidex-dom-api` and `elidex-layout` (→ `elidex-layout-block`), so it reaches the producer and the reader at one hop — the reachability an earlier revision denied by measuring adjacency instead. `build_pipeline_interactive` (`crates/shell/elidex-shell/src/pipeline.rs:563`) returns a `PipelineResult` carrying **`dom: EcsDom`** (`lib.rs:199-204`), so the test reads the span's real `LayoutBox` from the post-layout world **and** exercises `clientTop` — either by invoking the registered handler (`clientTop.get`, `crates/dom/elidex-dom-api/src/registry.rs:168`) or through a `<script>`, which that suite already does (`src/tests.rs:52` mutates the DOM from JS). ⚠ This row exists because §8 took on an obligation no other §5.2 row covers; the memo's other cross-crate rows (`elidex-render` paint, and the `— **not** elidex-dom-api` negative row) answer narrower questions. |
| `elidex-text` (facade over `elidex-shaping`) | `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60`) + `font_metrics` (`:101`) — M7's strut A/D, taken without shaping a string. |

### §5.3 Program slicing

Four shipping PRs, each **behaviour-scoped**, with one owning PR per coupling (§2), plus the
booked slots below — no count here, since the list grows as rounds find them (rev 33 added one).
Each PR gets its own plan-memo and `/elidex-plan-review`.

* **PR-1a — item stream. Behaviour-neutral.** Owns couplings 2×(all) and 1×2: M1 + M2. Marker
  variants, all consumers updated — **the same set §8's PR-1a DoD enumerates**, stated there once
  rather than in two lists — `containing_inline_size` threaded — **no citation work at all** (§9). The packer gets a **no-op** `match pi` arm (`:528`). The two pre-pack gates are held at
  current behaviour: `items.is_empty()` (`inline/mod.rs:161`) excludes markers, and the `any_font`
  closure gains its **exhaustiveness arm** at `:192-199` returning `false` — behaviour-neutral,
  since a marker is neither `Text` nor `Atomic`. The **outer condition** at `:200` is untouched
  until PR-1d. **Characterization tests for §6 cells 1, 2, 5, 6b, 6c, 6f, 6d, 6e, 6g, 6h, 7–12 and 12d land here
  asserting today's behaviour** — the cells that pin *line suppression*, which is observable with
  `entity`-only markers. ⚠ **Cells 6b, 6g and 6h are the exceptions and are not characterization
  cells**: the marker variants do not exist on `154bac3f`, so all three pin *new* item-stream behaviour
  (6g and 6h the pseudo-element routing M1 records, non-empty and empty `content`). PR-1a stays
  behaviour-neutral in the sense that matters — no layout output moves — but the item stream is
  deliberately not neutral, and 6b, 6g and 6h are what assert that, since `grep -rn padding
  crates/layout/elidex-layout-block/src/inline/tests/` → no hits means the existing suite pins
  nothing about decorated inlines. Cell 12d's harness change lands here too, with the cell it
  makes constructible. PR-1a opens 1.
* **PR-1b — inline-axis advance.** Owns couplings 7×2 and 7×(intrinsic): M3 + M8, **plus M5's
  predicate** (`has_inline_axis_edge`), which ships here because M3's shaping break and hang gate
  both call it; PR-1d owns only its *substitution* for the constant. The shared line-occupancy
  core with markers passing `contributes_content = false`, inline-axis advance, shaping break at a
  decorated boundary, the hang gate, and the max-content contribution. §6 cells 3, 3b, 4, 12b,
  12c, 12e, 14, 14b, 15, 15b, 15d, 16, 16b and 25 land here — cells 3, 3b, 4, 12b, 12c and 12e among
  them because each asserts a *payload* fact (negative margin, the cancelling pair, the percentage
  basis, the physical→logical side mapping) that M1's PR-1a variants, carrying `entity` only, give
  no channel to observe, and that the advance makes observable as a cursor position.
  ⚠ **What this PR does not touch is the box's own geometry.** `assign_inline_layout_boxes` still
  hard-codes `EdgeSizes::default()` (`inline/pack/boxes.rs:82-84`), so `border_box()` is still equal
  to `content`, and paint (`paint/mod.rs:68`, `emit_borders` at `:382`) draws exactly the area it
  draws today — the decoration stays as under-painted as it is on `154bac3f`, now correctly
  *positioned*. That is the property that makes this PR shippable ahead of PR-1c rather than after
  it; PR-1c's bullet states the asymmetry.
  ⚠ **Scope of "neutral", stated precisely**: PR-1b leaves the **phantom-line predicate**
  unchanged, so no line's *existence* flips. It does **not** leave `line_count` unchanged — the
  advance moves the cursor, so following text reaches the wrap guard (`:690`) earlier and
  content-overflowing paragraphs gain lines. That is the correct consequence of §1.1's
  "inline-axis … respected between inline-level boxes", not a regression; cells 12b and 15
  pin the advance as a cursor position (cell 3 pins the payload sourcing the advance makes
  observable), and **no cell pins a `line_count` change** (an earlier
  drafting said cells 3, 12b and 15 pin it; round 22, Axes 3/5). ⚠ **Cell 15d is the cell that
  comes closest and the one this universal has to be checked against** (round 26, Axis 3,
  Gate A): its second segment *does* reach the guard, so its width constraint is pinned into the
  regime where **today's count is already 2** — the cell asserts `lines.len() == 2` before and
  after PR-1b and pins no change. Its own ⚠ states the regime and why.
  ⚠ **What is withdrawn is the clause that used to follow the universal** — "the only markup that
  gains a line does so through the item-boundary flush `#11-inline-item-boundary-soft-wrap`
  records, which is not pinned as expected" (round 26, Axis 3, Gate A). That is a universal over
  *markup*, and the sentence two clauses earlier refutes it: the advance moves the cursor, so text
  reaches the guard at a **genuine UAX #14 opportunity** earlier than it did, which is exactly how
  cell 15d's markup gains its second line in the upper regime — a legitimate wrap at an
  opportunity `find_break_opportunities` produced, not the artificial item boundary the slot
  records. Both paths gain lines; what the slot owns is only the second. The universal that
  survives is the one about **cells**, and it survives because 15d is constrained into the
  already-wrapped regime rather than because no markup can gain a line. The two shapes stay lineless in
  PR-1b for **two different
  reasons**:
  * **Shape A** reaches the packer (its collapsed-to-`""` `StyledRun` keeps `items` non-empty),
    the shared core raises the line's occupancy above `Empty`, so `finish()` (`:787`) — which is
    an order test against `Empty`, not a bool read — flushes, and the flush takes the
    **discard** arm (`:423`) because `contributes_content` is `false` in PR-1b — no `LineBox`,
    and `current_block_offset` untouched (`:422` is in the commit arm). ⚠ This bullet said "the
    shared core sets `on_line`" until rev 34 — the field PR-1b **deletes**, as §8's own PR-1b
    paragraph says ("`on_line` is gone, replaced by the three-valued `line_occupancy`"); the
    mechanism is restated in occupancy terms and the conclusion is unchanged (round 26, Axis 5).
  * **Shape B never reaches the packer at all**: PR-1a holds `items.is_empty()`
    (`inline/mod.rs:161`) at "excludes markers" and PR-1b does not flip it, so the early return
    fires first. That is a **held gate — an inert shim that PR-1d removes**, not a property of
    M3/M4.

  PR-1b opens 1 (`#11-inline-min-content-box-edges`).
* **PR-1c — box geometry.** Owns coupling 3×7: M4. The open-box stack and its flush-time
  emit-and-rebase hook, the marker's content-span rect, and the three real `EdgeSizes` on the
  inline's `LayoutBox`. **This is where painted output changes**,
  for every decorated inline with content — the largest user-visible delta in the program. §6
  cells 6, 10b, 13, 14c, 15c, 17, 17b, 17c, 17d and 17f land here.
  ⚠ **The predicate prereq PR (§9) is already in `main` by this point** — it lands before PR-1a —
  which is what keeps PR-1c honest: PR-1c makes an inline's `LayoutBox.border` real, and
  cssom-view-1 §6 step 1 requires `clientTop`/`clientLeft` to stay **zero** for an inline box, so
  landing PR-1c into a tree without the guard would ship a new violation. §3's CSSOM row and §9's
  bullet carry the measurement.
  ⚠ **Why it follows PR-1b, and why the reverse order is not merely less tidy but wrong.** Paint
  reads `border_box() = content + padding + border` (`paint/mod.rs:68` for the background colour,
  `emit_borders` at `:382`). Land M4 *first* and an inline's real edges are applied to a content
  span the advance has not yet moved, so the box's background and border are drawn **over the
  preceding content** — on cell 13's own markup, `<p>a<span style="padding:10px">text</span>b</p>`,
  the span's background covers the tail of `a`. That is a defect the engine does not have today.
  The order taken introduces none: after PR-1b the edges are still zero, so `border_box()` is
  still `content` and the decoration covers exactly what it covers now. Monotone in one direction,
  a new wrongness in the other — which is the same test §5.3 applies to putting geometry before
  the existence flip.
  ⚠ Presence, not only value: M4 adds a second producer of `current_line_entity_rects`, so a
  decorated inline that owns no run gains its **first** `LayoutBox` — but only on a line that
  already exists. The rest of the presence change is PR-1d's; §7 states both stages.
  PR-1c opens 1 (`#11-inline-box-decoration-splits`).
* **PR-1d — existence.** Owns couplings 1×3, 1×5 and 1×4: M5's *flip* + M6 + M7. The clause-3
  predicate per §1.2, the strut baseline, the commit consequence, and the two pre-pack gates
  flipped. **The delta to PR-1b's marker call is `contributes_content` (M5's expression replacing
  the constant `false`) plus `block_advance` (M6);** everything else is already in place. §6
  cells 18, 19, 20, 21, 22, 23, 24 and 24b land here, plus the flip set §8 states. ⚠ It also
  grants presence a second time and more widely than PR-1c did: every entity on a line that was
  phantom and now commits gains its first `LayoutBox` — the decorated inline itself in Shapes A
  and B, and every co-resident (cell 21). Closes
  `#11-line-box-decorated-inline-content` with its original DoD — line *and* rectangle — met,
  because PR-1c already made the rectangle real. PR-1d opens none.
* **`#11-inline-box-decoration-splits`** (new slot, **own** deferral): css-inline-3 §2.1 box
  splitting across line boxes and bidi fragments, and **the whole of css-break-3 §5.4** — both `slice` and `clone`, and with them the
  **per-fragment `getClientRects` channel** (cssom-view-1 §6 step 3), in both the shapes §3's two
  CSSOM rows separate: inflating a stored content-span fragment into a border area needs the
  fragment's identity in *content* order, which `slice_and_rebase_fragment`'s `retain`
  (`pack/fragment.rs:69`) destroys for a paged or multicol IFC, and it needs the **parent's**
  inline progression direction, which css-break-3 §5.4 specifies and M1's own-direction mapping deliberately
  does not supply; and answering *within* one line at all requires a fragment unit the packer does
  not have, since `commit_aligned_entity_rects` folds per entity per **line** and the UAX #9 L2
  reorder is render's. With it travels the `getBoundingClientRect` half (cell 17f), which is the
  same missing attribution seen from the other side. ⚠ Like every geometry this program writes it is
  first-layout-only until `#11-inline-relayout-box-staleness` lands (§8, §9) — an ordering note,
  **not** a blocking dependency, since that limit applies equally to PR-1c's own edge write and
  the memo accepts it there. Also fragmentation across the marker pair;
  **css-text-3 §5.5's adjacent-soft-wrap rule** (a break next to a decorated boundary lands at the
  box's *margin edge*, which M3's unconditional start-edge advance does not honour — §3's css-text-3 §5.5 row
  whose Step cell reads "adjacent soft wrap opportunity" and §6's note after cell 15 route here);
  and the `group_key` / relpos sub-flow keying that §2's pair 2×6 and §3's CSS 2 §9.4.3 row defer
  here, the field's only reader being the split rule. Why deferred: box continuity across line,
  bidi and sub-flow boundaries is a distinct invariant axis, reachable only once a box has
  geometry; folding it in would put a second axis into PR-1c, which after the re-slice owns
  exactly one coupling. Trigger: **PR-1d landing, *or* terminal-Z C-3b pinning the `getClientRects` dispatch** — a
  disjunction, matching the sibling #488 slots in the same SoT, which use `or` precisely so an
  unscheduled slice cannot freeze a slot. C-3b co-owns the multi-fragment question (§9) but has no
  PR and no date, so a conjunct would do exactly that. Re-eval: 2026-11-01.
* **`#11-inline-root-inline-box`** (new slot, **pre-existing** class): the block container's
  root inline box (css-inline-3 §2), which sets every line box's height floor. Why deferred: it
  changes the height of *every* line box in the engine — verified pre-existing because the
  divergence is observable today on ordinary text (M6's example), with no marker involved, so
  `origin/main` already fails it. Trigger: any line-height correctness work, or a compat-survey hit.
  Re-eval: 2026-11-01. ⚠ **A third disjunct — "any work needing font-fallback provenance" — is
  struck** (round 25, Axis 3): it existed only so that §8's fold of css-inline-3 §5.3's
  "only glyphs from fallback fonts" strut condition would surface here, and that fold is withdrawn
  (§8; the condition has its own slot, the next bullet). It is not independently true of *this*
  slot's subject either: the root inline box is the block container's anonymous inline box and the
  line box's **height floor** (§1.1), which needs no per-glyph font provenance to implement.
* **`#11-inline-fallback-font-strut`** (new slot, **pre-existing** class): css-inline-3 §5.3's
  second strut condition — "**or if it contains only glyphs from fallback fonts**", quoted in §1.5 —
  under which an inline box whose glyphs all came from fallback fonts takes a strut with its
  **first available** font's metrics rather than its glyphs'. Unimplemented. Why deferred: elidex
  has no fallback-provenance signal at all — `measure_text` reports font **availability**, not
  provenance (`db.query(…)?` then `db.font_metrics(…)?`,
  `crates/text/elidex-shaping/src/measurement.rs:54-55`, the same measurement M7's Grounds makes for
  its own residual), so there is nothing in the engine that can answer "which font produced this
  glyph"; supplying one is shaping-layer work with its own invariant axis, not inline-decoration
  work. **Pre-existing** class on the memo's own test: the divergence is observable today, on
  ordinary fallback-rendered text with no marker involved, so `origin/main` already fails it. Trigger:
  any work that gives the shaping layer per-glyph font provenance, any baseline- or line-height
  correctness work that needs it, or a compat-survey hit. Re-eval: 2026-11-01.
* **`#11-inline-zero-edge-box-in-item-stream`** (new slot, **own** deferral — PR-1a's): M1 emits
  markers only for inline boxes with a non-zero edge; css-display-3's class has no such condition,
  and css-inline-3 §2.2 Note gives a **zero-edge** empty inline (`<span style="line-height:100px">
  </span>` on a line other content keeps) a line-height contribution this program therefore does
  not deliver. **Own, not pre-existing, on one ground**: the line-height gap predates the program
  (`origin/main` fails it with no marker involved), but the **presence asymmetry** does not — after
  PR-1c a decorated empty inline **on an existing line** has a `LayoutBox` and an undecorated one
  there still has none (the qualifier is M1's and cell 14c's: the sole-content case has no line at
  all until PR-1d flips `items.is_empty()`, §6 cell 12), a
  distinction css-display-3 does not draw and this program introduces by keying M1's emission on
  decoration; the slot retires that asymmetry, which is why it is PR-1a's (the PR that introduces
  the conjunct). ⚠ An earlier drafting tagged it own on the scope ground below — but scope answers
  why the widening is not done *here*, and the memo's own pre-existing test is "`origin/main`
  already fails it with no marker involved", which the asymmetry does not (round 24, Axis 3; a
  gate then found the sentence missing the existing-line qualifier M1 carries). Why deferred: a marker for every inline box grants — through M4's unconditional
  `InlineBoxEnd` push and the commit-arm fold into `entity_bounds` — a `LayoutBox` to every
  undecorated empty inline on an existing line: spec-correct (an empty inline box is a box) but a
  presence change over every empty `<span>`/`<a>`/`<b>` in every document. The ground that
  carries the boundary: the conjunct is what this program's cell surface — **6b** (all edges zero
  ⇒ no marker), **14**'s contrast (its no-marker clause only; coalescing holds either way, M3 keys
  on `has_inline_axis_edge`), **21** and §7's "a box with all edges zero gains nothing" — is
  written against, and widening changes `LayoutBox` presence for every empty undecorated inline;
  the breadth ("every empty `<span>`/`<a>`/`<b>`") is a claim about documents, not a measurement. It is sliced out rather
  than scheduled as PR-1e on a **scope** ground, not a constructibility one: the presence change
  it carries is over **undecorated** empty inlines, outside this program's subject ("decorated
  inline content") and outside §7's reader audit, which disposes presence for decorated inlines
  and for the co-residents of a formerly phantom line only; it arms at PR-1d landing because that
  is when the program's own presence change has landed and the wider one can be measured against
  it. ⚠ An earlier drafting gave constructibility before PR-1c/PR-1d as the ground; 6b's flipped
  form is constructible at PR-1a through the item-stream helper, and constructibility would not
  distinguish a slot from a PR-1e — a PR-1e after PR-1d would find every one of the cells above
  constructible (round 23, Axis 3). Those
  cells **flip** when this slot lands (14c is *not* one of them — a decorated empty inline
  keeps its box either way; an earlier drafting listed it). How: drop the conjunct from M1's emit
  test — the edges are already derived downstream (M3's sums, M5's disjunction; the PR-1a-only
  resolve-then-discard M1 records is already gone by then, PR-1b having put the edges on the
  payload). Trigger: **PR-1d landing** (the program's other two own slots carry the same
  disjunct; §10's successor row names all three), `#11-inline-root-inline-box` work
  (the same line-height surface — a different mechanism, so not the same slot), or a compat-survey
  hit on `getClientRects()`/line-height of empty inlines. Re-eval: 2026-11-01. Found by Codex on
  #515 (an earlier drafting folded it into the root-inline-box slot, a mechanism mismatch; a later
  one gave "a presence-axis program of its own under the edge-dense rule" as the Why, a
  classification where the ground above was owed — round 22, Axis 3).
* **`#11-inline-item-boundary-soft-wrap`** (new slot, **pre-existing** class): `place_item`
  flushes the line whenever the next placed item does not fit (`pack/mod.rs:658` at `22de3078`:
  `if self.current_inline + trimmed_width > containing_inline_size && self.on_line`), with no
  break-opportunity test at the item boundary — break opportunities are found only *inside* a
  text run (`find_break_opportunities` in `build_pack_items`, `pack/items.rs:74`) — so `aaaaaaaaaaaa<span></span>b` wraps before `b` although
  css-text-3 §5.5 gives it no soft wrap opportunity (the boundary adds none). Why deferred:
  carrying break-opportunity state across items is a line-breaking change to the packer's core
  loop, orthogonal to decoration and observable today with no marker involved (`origin/main`
  already fails it: `collapse_inline_whitespace` never merges adjacent same-entity `Text` items,
  `build_pack_items` splits each run by `find_break_opportunities` — `pack/items.rs:74` at
  `22de3078`, UAX #14, no cross-item state — and `place_item` flushes per placed item); this
  program only moves the cursor past the boundary (cell 15 asserts that and nothing about where
  `b` lands), and the marker adds no item boundary the runs did not already have. **The layer**:
  the cross-item opportunity is `elidex-linebreak`'s — UAX #14 over one string today,
  `find_break_opportunities(text)` (`crates/text/elidex-linebreak/src/lib.rs:26` at `22de3078`) —
  so the fix is an API there over the concatenated paragraph (or a stateful one) that the packer
  consumes; packer-side state would re-derive UAX #14 in layout. **The consumption edge** is the
  `elidex-text` facade, not `elidex-linebreak`: `pack/items.rs:66` at `22de3078` is `use
  elidex_text::find_break_opportunities`, re-exported at `crates/text/elidex-text/src/lib.rs:10`
  (`pub use elidex_linebreak::{find_break_opportunities, BreakOpportunity}`), and
  `elidex-layout-block`'s Cargo.toml has `elidex-text` and no `elidex-linebreak` edge — so
  ownership is `elidex-linebreak`'s; whether the new API is re-exported through the facade or
  reached by a new edge is the slot memo's call — this memo records the measurement, not the
  mechanism (round 23, Axis 1; round 24, Axis 3). Trigger: any line-breaking correctness work, a
  compat-survey hit, or `#11-inline-box-decoration-splits` picking up css-text-3 §5.5's margin-edge bullet
  (the same rule's other half). Re-eval: 2026-11-01. Found by Codex on #515.

Own deferrals **per PR** (the policy's unit), for all **seven crate PRs** of the program (§8's two
bookkeeping PRs — neither touches `crates/` — sit outside check 9's roll-call: the docs-only
**approval PR** opens none by construction; the **tooling PR** ships skill infra — checker
generalisation, wiring and two-file awareness, §9 — and its own-deferral count is its own memo's
call, the treatment the predicate prereq gets
below): PR-1a opens 1, PR-1b opens 1, PR-1c opens 1, PR-1d opens none,
the seam-3 prereq opens 1, the dead-arm prereq opens none, and the predicate prereq opens none
**here** — what it opens is its own memo's call,
since it is carved precisely because its questions are not this memo's to settle (§9).
⚠ **That line is wrapped deliberately**: until rev 34 the wrap fell between `dead-arm` and
`prereq`, which hid the tuple from the checker's own regex — the remediable half of the two
defects the paragraph below records (round 26, Axis 3).
⚠ The seam-3 prereq's **1** is `#11-inline-fragmented-fn-seams-1-2`, and the count is the SoT's
landing record for #508, not this memo's: the slot is **mixed** class — seams 1 and 2 are
pre-existing, but `reconcile_flows`' extracted signature is *created* by #508, so its own half
makes it #508's one own slot (≤3 ✓). An earlier drafting wrote "opens none" by counting the
pre-existing half alone; #508's memo recorded the contradiction and this revision resolves it in
the SoT's favour. The dead-arm prereq's "none" is verified at #511's landing (it registered nothing).
⚠ `plan-xcheck.py`'s check 9 — **as the checker stands at this branch's HEAD; the tooling PR
widens the alternation, so this paragraph is re-derived when the tooling PR lands — the approval
PR is cut from `origin/main` and needs no merge, and the checker on this branch is unchanged
until then** — reads **six** of this program's seven `opens` statements, not
seven. ⚠ **It read five until rev 34, and the two it missed failed for *different* reasons — one
of them remediable here** (round 26, Axis 3; an earlier drafting of this very sentence asserted
only the first reason, and the revision that added the second gave the remediation for the first
alone). (a) Its alternation is closed (`PR-1[a-z]|seam-3 prereq|dead-arm prereq`), so
`predicate prereq` is unmatched by name — **not fixable here**: widening the alternation is the
plan-checker tooling task's (§9). (b) `dead-arm prereq` **is** in the alternation and was
nonetheless unread, because the pattern's literal space fell on a **line wrap** in the sentence
above — **fixed here, by rewrapping that line at zero cost**, which is the disposition (b) was
owed and did not get. The harvest, reproduced by the checker's own regex over this file —

```
grep -noE '(PR-1[a-z]|seam-3 prereq|dead-arm prereq) opens? ([0-9]+|none)' <memo>
```

— now returns ten tuples: `PR-1a`/`PR-1b`/`PR-1c`/`PR-1d` **twice** each (once from the per-PR
bullets above, once from this sentence) and `seam-3 prereq` and `dead-arm prereq` **once** each.
Against them §10 carries **four** `(own)`-tagged rows (PR-1a, PR-1b, PR-1c, seam-3 prereq —
`awk '/^## §10\./,0' <memo> | grep -cF '(own)'` → 4), so check 9 is **live in both directions** for
PR-1a, PR-1b, PR-1c, PR-1d, the seam-3 prereq and the dead-arm prereq; only `predicate prereq`
still holds in the reverse direction alone — an `(own)`-tagged §10 row naming it would fail as
`§10 tags … which §5.3 does not account for`.
⚠ This is a claim *about* the checker that the checker cannot check
([[feedback_prose-rules-cannot-fix-unexecuted-claims]]); the two commands above produced it, and
re-running them is the only thing that keeps it true. Widening the alternation is the plan-checker
tooling task's (§9), not a fix to make here. `#11-inline-root-inline-box`,
`#11-inline-item-boundary-soft-wrap`, `#11-inline-fallback-font-strut` (rev 33) and the dead arm's
own disposition are pre-existing class, so none of them enters a per-PR count. All within ≤3; `.claude/tools/plan-xcheck.py` cross-checks
these against §10's own-tagged rows.

**Rejected**: widening `StyledRun` with edge fields (box-level data on a per-segment
measurement type; N copies for an N-segment span; reaches neither shape per §4.2); reading
`run.entity`'s `ComputedStyle` at pack time (reaches neither shape, same reason). Neither is
rejected on component-lookup cost — `collect.rs:36` uses borrowed component reads in the same
per-child loop, a normal idiom here.

### §5.4 ECS-native check (index)

The OO→ECS mapping this design makes is stated where each decision lives; this subsection only
indexes it (round 21, Axis 2 `[plan]` schema entry): **M1** — an index into the pass-local item
stream, not a copied box object; **M1** (pseudo routing, rev 28) — the class is decided by the
box's own `ComputedStyle` component, not by the entity's origin; `PseudoElementMarker` is a
routing marker, not a class marker (cells 6g/6h); **M3** — `LineOccupancy`, a three-state enum that encodes the state once, instead of a bool
pair whose implicit `content_on_line ⇒ on_line` invariant would have to be maintained across
three write sites; **M7** (rev 33, renamed rev 34) — the per-line *rendered-text* fact is the **same** answer applied a
second time: PR-1d gives that same monotone ordering one more rung, raised by M3's single writer
from an occupant `place_item` **derives from an enum variant it is already handed** —
`FlowMember::Text(_)` (`inline/pack/items.rs:49-58`) conjoined with `contributes_content` — rather
than a second per-line bool carrying an implicit
`⇒ any_rendered_content` invariant (cell 24). ⚠ **Same idiom, stated at the raise**: an earlier
drafting located the rung "at the one site whose value means glyph text", i.e. at a *call site*,
which no query can test — the ECS-native form of the same answer is the discriminating **variant**,
which travels with the value (rev-33 gate). The M3 entry above states the ordering as PR-1b
leaves it, three-valued; the widening is PR-1d's **fourth variant, `RenderedText`** (M7 decides
that here rather than delegating it — round 26, Axis 2), and adds no field, no reset, no second
writer and no new parameter;
**M4** (Grounds) — the side-store→component rule is
answered at the `HashMap<Entity, _>` *shape*: an intra-pass scratch map on a stack-local packer
whose destination is the `LayoutBox` component is the audit's "clean" class, so no entity-keyed
registry is introduced by any carrier option; **§9 FragmentTree bullet** — this program adds no
new persisted carrier; `LayoutBox` (a component) is the destination and terminal-Z C-4 retires it
on its own schedule.

## §6. Edge matrix

**PR-1a characterization (assert today's behaviour; PR-1d's flip set is stated once, in §8's PR-1d DoD):**

1. `padding` alone (inline-axis) — line currently suppressed.
2. `border` alone — a real border, so a marker is emitted and the line flips in PR-1d.
   ⚠ The `border-style: none` + `border-width: 5px` arm is **not a cell of this program**: the used
   width is zeroed at computed-value time in `elidex-style`, which `elidex-layout-block` cannot
   exercise (no dependency, no dev-dependencies), and the assertion already exists —
   `crates/css/elidex-style/src/resolve/box_model/tests.rs`'s `border_width_zero_when_style_none`.
   Downstream of it a zero-width border is simply the all-zero case cell 6b already pins.
5. **Block-axis edges only** (`padding-top`, `margin-top`) — **line stays suppressed in
   PR-1d too**, per css-inline-3 §2.3's inline-axis restriction, and CSS 2 confirms **both** halves
   independently — CSS 2 §8.3 for margins, CSS 2 §10.8.1 for padding and border. The authorities
   are §1.2's;
   this cell refers to them rather than restating them. ⚠ It used to restate them, in the two forms
   §1.2 has since withdrawn: "§8.3 says nothing about padding or border" (refuted by CSS 2 §8.3.1) and
   then the asymmetry that only `css-inline-3` covers the padding/border half (refuted by CSS 2 §10.8.1)
   — rev-33 gate. A non-regression cell in every PR.
6b. **The emit predicate's two arms** (M1) — all edges zero ⇒ **no marker in the item stream**;
    block-axis-only (`padding-top`) ⇒ a marker **is** emitted. In PR-1a the item stream is the
    *only* channel where a marker is observable at all (the packer's `match pi` arm is a no-op and
    the variants carry `entity` only), and ⚠ **the existing helper cannot reach it**:
    `collect_styled_runs` (the `fn`, `inline/tests/mod.rs:11-25`; `:17` is its own
    `collect_inline_items` call, which §5.2's `collect.rs` row cites separately — an earlier
    drafting wrote `:17-25` at both sites, a range that starts at the call and means neither the
    function nor its `match`, round 25 Axis 2) `filter_map`s the items to
    `Vec<StyledRun>`, so its `match item` (`:20-23`, the coordinate §5.2's harness row uses; the
    two **arms** are `:21-22` — ⚠ an earlier drafting wrote "arms (`:20-23`)", which names the
    `match` head and its closing brace and neither arm, rev-33 gate) is exhaustive for
    *compilation* but every non-`Text`
    variant — `Atomic`, `Placeholder`, and the two new markers — is mapped to `None` and
    discarded. **PR-1a's DoD therefore carries a second helper** returning the items themselves,
    exactly as it already carries `setup_inline_test`'s no-font change for
    cell 12d. Without it this cell is unconstructible, which is the defect class the memo refuses.
6c. **A decorated *replaced* inline gets no marker** (M1's third arm) —
    `<p>a<img style="padding:10px">b</p>`: the `<img>` computes `display: inline` (no UA rule
    selects it, §9) and is therefore an **atomic inline**, not an inline box
    (css-display-3 §A: *inline box* = "A non-replaced inline-level box whose inner display type is
    flow. …" — the `…` is §A's second sentence, on the contents' formatting context, which this
    cell does not use; *atomic inline* = "replaced (such as an image) **or** … establishes a new formatting
    context … **and cannot split across lines** (as inline boxes and ruby containers can)").
    css-inline-3 §2.3 clause **3** is scoped to inline boxes, so it does not reach it;
    clause **4** (other in-flow content) does. The item stream must therefore contain **no**
    `InlineBoxStart`/`InlineBoxEnd` for it. ⚠ Without this cell M1's predicate emits one — the
    `<img>` passes `Display::None`, abspos and `is_atomic_inline` and falls into the inline-element
    recursion — past the **loop head, its style guard and its four filters** (`inline/collect.rs:216-272`, the same
    span M1 names, and *not* "the recursion arm": the recursion itself is `:291`, outside it, as is
    the text arm at `:302`; the `for` body closes at `:317`) and into the arm at `:291`;
    ⚠ an earlier drafting wrote `:214-251`, whose `:214` is the depth guard's `return`, outside the
    `for` loop that starts at `:216`, and whose `:251` stops at the `is_atomic_inline` arm's
    `continue`, before the `PseudoElementMarker` filter at `:260-272` — round 24 audit) — and PR-1b would advance the line by
    its **edges only**, PR-1c would give it a zero-width-content `LayoutBox`, and PR-1d would keep a
    line alive on the wrong clause. That is a defect this program would create; today an inline
    `<img>` gets nothing from the IFC at all. Asserted here, in PR-1a, because the item stream is
    where the predicate is observable and where the exclusion lands.
    ⚠ It does **not** assert correct replaced-inline layout — elidex has no replaced arm in the IFC
    at all, which is pre-existing and out of scope (§9); the cell pins only that this program does
    not build on top of the gap.
    ⚠⚠ **Fixture stipulation, and it is what makes this cell discriminate**: the `<img>` carries
    **no `ImageData`**. The crate's established idiom does the opposite — `make_dom_with_image`
    (`crates/layout/elidex-layout-block/src/block/tests/mod.rs:106-119`) attaches it
    unconditionally — so a cell reusing that helper would let a **presence-keyed** predicate pass
    while still emitting a marker for every unloaded image, i.e. ship the defect §8 requirement 5
    exists to prevent. This cell is the enforcement lever for that requirement; prose cannot fail.
6f. **The same markup with `ImageData` present** — the other side of the fixture axis. Together
    with 6c it pins that the predicate answers **replaced** from element identity, not from decode
    outcome: both must yield *no marker*. A predicate that passes 6f and fails 6c is exactly the
    presence-keyed answer requirement 5 rules out.
6d. **A decorated `display: contents` element gets no marker** (M1, same predicate, different
    conjunct) — `<p>a<span><span style="display:contents;padding:10px">x</span></span>b</p>`: css-display-3
    **§2.5** *Box Generation: the `none` and `contents` keywords* — "The element itself does not
    generate any boxes, **but its children and pseudo-elements still generate boxes and text
    sequences as normal**" (`body css-display-3 valdef-display-contents`) — so it has no outer
    display type and cannot be an inline box, **and** `x` still reaches the line. ⚠ The quotation
    stopped at "any boxes" with no ellipsis until rev 34 (round 26, Axis 4), dropping the very
    clause this cell's *second* assertion rests on. ⚠ **§A Glossary does not carry this rule**; an earlier
    drafting cited it there, and the box-generation rules §A *does* mention (`body css-display-3 glossary |
    grep -i generat` — no count is stated here, the command's output is the answer) are principal
    box, additional boxes, anonymous block boxes, root inline box and containing block; none of them
    is this one. ⚠ **The nesting is what makes the cell reach M1's predicate at all**: the *inner*
    span is fetched by `collect.rs:277`'s **raw** `dom.composed_children` from the outer inline —
    unlike `positioned_subflow_key` at `:110`, which takes `composed_children_flat` for exactly
    this reason (comment at `:93`) — and `collect.rs` has no `Contents` branch, so it reaches the
    recursion arm. An earlier drafting made the `display:contents` span a **direct** child of the
    `<p>`, where it never reaches the arm: the IFC root's child list is `composed_children_flat`
    (`block/mod.rs:308` → `helpers.rs:348`'s `composed_children_flat`, which calls
    `flatten_contents` — defined at `:363` — at `:350`; passed to the IFC at
    `block/mod.rs:540`), so it is flattened away before `collect_inline_items` runs and a
    predicate *without* the `display:contents` conjunct passes the cell (round 24 audit).
    Live in the engine today — `ua.rs:109` is `slot { display: contents; }`. Asserts: no marker,
    and `x` still reaches the line.
6g. **A decorated pseudo-element inline box gets markers** (M1's routing special-case; Codex on
    #515) — `p::before { content: "x"; display: inline; padding: 10px }` on `<p>ab</p>`: the pseudo
    entity carries its own cascaded `ComputedStyle` (padding 10px) and computed outer `inline`, so
    it is an inline box, and `InlineBoxStart`/`InlineBoxEnd` bracket its one `Text` item (the
    empty-`content` case is cell 6h) (`collect.rs:265`'s branch at `22de3078`, re-routed by PR-1a).
    Observed through the PR-1a harness and
    **behaviour-neutral** here (the markers are inert until PR-1b). Paired with 6c–6e it pins that
    the class is decided by the css-display-3 predicate on the box's *own* style, not by the
    entity's origin (element vs generated content). Does not flip at PR-1d. ⚠ Fixture:
    `elidex-layout-block` has no `elidex-style` dependency, so the pseudo is hand-built — **as
    production builds it** — `dom.create_text("x")` (the node kind `elidex-style/src/pseudo.rs:63` creates:
    `NodeKind::Text`, no `TagType`) with `ComputedStyle` (padding **and** `display: Display::Inline`
    set explicitly) and `PseudoElementMarker` inserted, the insert idiom of
    `inline/tests/inline_flow/persist.rs:134-143` at `22de3078` — ⚠ **not** that test's
    `create_element("span", …)`, which is an element-kind entity carrying `TagType` and would let a
    tag-gated predicate pass (§8 requirement 7); the fixture stipulates the node kind as well as the
    style. That the cascade produces that style
    (`elidex-style/src/pseudo.rs:53-55` defaults `display` to `Inline`, `build_computed_style`
    carries padding) is a **stipulation**, as 6c's `ImageData` is; the stipulation is discharged
    end-to-end by §8 PR-1c's second markup. Consequence for **PR-1c**: its
    reader disposition (§8) reaches a second entity class — a decorated `::before`'s background
    and border move from content span to border box, cell 13's delta on generated content.
6h. **An empty decorated pseudo-element gets an adjacent marker pair** (M1; round 22) —
    `p::before { content: ""; padding: 10px }` on `<p>ab</p>`: the pseudo entity exists
    (`elidex-style/src/pseudo.rs:47` is the early-return **guard** — it returns unless `content` is
    `ContentValue::Items(_)`, which `content: ""` satisfies — and `:63` creates the entity; its
    `TextContent` is `""`, `generated_content.rs:158`) and is an inline box, but the
    `collect.rs:265` branch (at `22de3078`) pushes no `Text` for it (`!tc.0.is_empty()`, `:267`), so
    `InlineBoxStart`/`InlineBoxEnd` are **adjacent** in the item stream with nothing between.
    Observed through the PR-1a harness, behaviour-neutral here; the generated-content analogue of
    cell 14c (its box, at PR-1c, is 14c's case — M4's `InlineBoxEnd` pop pushes unconditionally).
    Fixture: 6g's (`dom.create_text("")` + `ComputedStyle` + `PseudoElementMarker`, the production
    node kind). Does not flip at PR-1d. Like 6b/6g, **not** a characterization cell.
6e. **A block-level element reached through an inline gets no marker** (M1, the outer-display
    conjunct) — `<p>a<span><div style="padding:10px">x</div></span>b</p>`: `children_are_block`
    (`crates/layout/elidex-layout-block/src/block/mod.rs:70`) tests **direct** children only, so this
    `<p>` takes the IFC path and the `<div>` reaches the arm. A block-level box is not an inline box.
    ⚠ It does **not** assert correct block-in-inline layout (CSS 2 §9.2.1.1's anonymous-block split
    is unimplemented on this path, pre-existing — §9); it pins only that M1 emits nothing for it.
7. Shape A (decorated inline containing only collapsible white space).
8. Shape B (completely empty decorated inline).
9. `a <span style="padding:10px"> </span>b` — the M2 cell: the space must collapse against its
   neighbours exactly as today (`a b`). ⚠ **The markup is M2's, and it is chosen to be
   discriminating**: an earlier drafting used `a<span style="padding:10px"> </span>b`, on which a
   marker acting as a collapse barrier would produce `a b` as well — the cell would have passed
   the regression it exists to catch, because `prev_collapsible_space` is already false on entry to
   the span: the `<p>`'s `"a"` takes `collapse_run_text`'s `Normal` arm's non-space branch, which
   sets the flag false (`whitespace.rs:150`). ⚠ Not `whitespace.rs:34` — that line *seeds* the flag
   **`true`** (so that leading collapsible white space at the start of the IFC collapses away), i.e.
   it establishes the opposite value; an earlier drafting cited it here, while M2's own
   parenthetical already names `:34` **and** the `Normal` arm correctly (round-24 gate). With the
   leading space, today's `a b` becomes `a  b` under a barrier, so the assertion has a failing
   counterpart (round 24 audit — the same non-discriminating shape found in M2's own grounds).
10. Nested: inner decorated / outer not, and the reverse — both suppressed today. The *stack*
    half is cell 10b, in the PR that ships M4.
11. Two inlines on one line, one decorated.
12. Decorated inline as the only IFC content — the **`items.is_empty()` gate**
    (`inline/mod.rs:161`) only, held in PR-1a and flipped in PR-1d. ⚠ This cell does **not**
    exercise the `any_font` probe: `has_text` (`:190`) is false for a marker-only stream, so the
    whole `if has_text` block (`:191-211`) is skipped.
12d. **The `any_font` early-out with a decorated inline** — text plus a decorated inline in an IFC
    where **no font is usable**: `has_text` is true, `any_font` false, so `:200`'s early return
    fires and the IFC returns `line_count: 0` today, even though clause 3 is font-independent.
    PR-1d adds a marker escape beside the existing `Atomic` one in that **outer condition**
    (`:200`); PR-1a only adds the exhaustiveness arm at `:192-199`.
    ⚠ **Harness**: `setup_inline_test` (`inline/tests/mod.rs:54`) returns `None` when the test
    families are unavailable and every caller early-returns, so the suite currently *skips* rather
    than *exercises* the no-font path. **PR-1a's DoD carries the harness change** — a deterministic
    way to force `any_font == false` — because PR-1a is where this cell is first asserted; without
    it the cell is unconstructible, which is the defect class this memo refuses to ship: a cell no markup can construct.
**PR-1b inline-axis advance:**

3. `margin` alone, **including negative** — requires M1's `resolve_box_model` sourcing.
3b. **Cancelling pair, the sum/disjunction contrast on the advance side** — the edge pair is
    `margin-left:-10px;padding-left:10px`, carried on **cell 14's and cell 16's markups** rather
    than a bare span, because the two observables this cell names are only visible there: a shaping
    break needs two same-entity text runs (`<p>a<span …></span>b</p>`) and a hang needs a preceding
    collapsible space (`<p style="text-align:center">abc <span …></span></p>`). On both, the
    inline-start components sum to zero
    but each is non-zero, so M3's **sum** gives a cursor advance of zero while M5's
    **disjunction** holds. In PR-1b that disjunction is observable through its two PR-1b
    consumers and only those: the shaping break fires at the boundary and
    `current_line_last_hang` is zeroed, for a box that moves the cursor not at all. ⚠ The
    line-existence half of the same disjunction — css-inline-3 §2.3 keeping the line — cannot be
    asserted here, because PR-1b's marker path passes the constant `false`; cell 24b asserts it in
    PR-1d, where the substitution makes it true.
4. **Percentage** `padding: 5%` / `margin: 2%` — against `containing_inline_size`.
12b. **`direction: rtl`** (M1): `<p dir="rtl"><span style="padding-left:10px">x</span></p>` —
    `padding-left` is the inline-**end** edge here, so it advances the cursor *after* the content.
    Pins the physical→logical mapping itself (`padding-left` → inline-end under rtl). It does not
    discriminate *whose* direction is used — `direction` is inherited, so the span's own and its
    parent's agree here. Cell 12e does.
12c. **`writing-mode: vertical-rl`** (M1): `<p style="writing-mode:vertical-rl">`
    `<span style="padding-top:10px">x</span></p>` — the property is on the **containing block**,
    not on the span. The inline axis is vertical, so `padding-top`/`bottom` become the inline-axis
    pair and `padding-left`/`right` the block-axis pair: cell 6's block-axis-only case inverts
    here, and cell 4's percentage basis is the containing block's inline size (its physical
    height). ⚠ Putting `writing-mode` on the **span** instead constructs a different cell: per
    css-writing-modes-4 §3.2 that span computes to `inline-block`, i.e. an atomic, and M1 emits no
    marker for it. Both markups are asserted.
12e. **Own-vs-inherited direction** (M1): `<p dir="rtl"><span dir="ltr" style="padding-left:10px">x</span></p>`
    — the span's **own** used direction is `ltr`, so `padding-left` is its inline-**start** edge,
    even though the IFC root and the span's parent are both `rtl`. Pins the mapping basis
    css-writing-modes-4 §6.4 states — "based on the used `direction` and `writing-mode`" — read
    per box, which is css-writing-modes-4 §6.2's attribution: "For boxes with a used `direction`
    value of `ltr`, this means the line-left side." ⚠ An earlier drafting quoted the pair as one
    css-writing-modes-4 §6.4 sentence, "based on the used `direction`… of the box"; "of the box" is
    not in css-writing-modes-4 §6.4, which names no owner. §3's own css-writing-modes-4 §6.4 row already renders it correctly, with the
    gloss outside the quotation marks, and this cell now follows it (round 24 audit).
    Cell 12b cannot discriminate this, because there
    the span has no `dir` of its own.

14. **Shaping breaks at the boundary** (§1.4): in `<p>a<span style="padding:1px"></span>b</p>`
    the two same-entity texts must **stop** coalescing into one `InlineFlowRun`. Contrast: with a
    span whose edges are zero on **every** side, no marker is emitted (M1) and they still coalesce.
    ⚠ A `padding-top`-only span **does** emit a marker but fails `has_inline_axis_edge`, so
    coalescing there is the gate's doing, not automatic — cell 14b. Neither contrast holds for
    `vertical-align: super` or `dir`-isolated spans, which css-text-3 §7.3 also breaks on; §3's row
    records those two triggers as unimplemented.
14b. **The gate's negative arm** — `<p>a<span style="padding-top:1px"></span>b</p>`: a marker
    exists, `has_inline_axis_edge` is false, so the runs still coalesce, the hang is untouched
    (cell 16's contrast, and M3's `hang: None` path) and **`b` is not displaced — the cursor advance
    is zero**, because M3's sum is over inline-axis components only. ⚠ This markup's line is **not**
    phantom — `a` and `b` are text, so clause 1 already keeps it (`contributes_content`,
    `pack/mod.rs:556-568`); the phantom case is cell 5's and cell 6's, on markup with no text.
    The advance half is asserted here rather than in cell 6, which after the re-slice is PR-1c's
    and asserts the box rather than the cursor.
15. **The boundary is not a wrap opportunity — the advance** (§1.3):
    `<p style="width:300px">aaaa<span style="padding:20px"></span>b</p>` — nothing wraps
    (`lines.len() == 1` before and after PR-1b), and `b`'s run `inline_start` is
    `measure_width("aaaa") + 40` after PR-1b — expectations computed with the harness's
    `measure_width` (`tests/mod.rs:31`), as the crate's justify tests do. Today there is no `b`
    run at all: `aaaa` and `b` are same-entity text runs (`collect.rs:315` at `22de3078`,
    `parent_entity` — both the `<p>`) that `place_item` coalesces into one `"aaaab"` run at `inline_start` 0
    (`pack/mod.rs:713-717` at `22de3078`: `coalesce = self.last_placed_entity == Some(entity)`);
    the marker's inline-axis edge is what ends the coalescing (cell 14), and the separate `b` run
    is the cell's second assertion. ⚠ What this cell does **not** assert: any wrap. This markup
    has no soft wrap opportunity (letters, no spaces — css-text-3 §5 puts them at word
    boundaries; css-text-3 §5.5: the boundary adds none), and needs none —
    the width is chosen so no item reaches the wrap guard, because the guard is `place_item`'s
    per-item flush (`pack/mod.rs:658` at `22de3078`) which wraps at every item boundary — the
    pre-existing divergence `#11-inline-item-boundary-soft-wrap` (§5.3) records and this program
    must not pin as expected. ⚠ Two earlier draftings: one asserted "`b` **does** wrap" (Codex on
    #515); the next kept `width:100px`, on which `aaaaaaaaaaaa` (≈107px at 16px Arial —
    `TEST_FAMILIES`'s first family, `tests/mod.rs:43-50`) already overflows today, so the advance
    reached no `InlineFlowLine` field (PR-1b's observation channels are `InlineFlowLine` —
    `block_start`, the runs' `inline_start` — `lines.len()` and `InlineLayoutResult.height`; the
    span's rect is M4's, PR-1c) and the cell asserted nothing (round 22, Axis 2).
*(A soft wrap opportunity adjacent to a decorated boundary — css-text-3 §5.5's other bullet puts
the break at the box's **margin edge** — is **not** a cell of any PR here. M3 advances the start
edge unconditionally, so that rule is unmet; it is folded into `#11-inline-box-decoration-splits`
(§5.3's Why names it). §3's css-text-3 §5.5 row **whose Step cell reads *adjacent soft wrap
opportunity*** is marked ✗ accordingly — the same disambiguation §5.3 already uses, because §3
carries four css-text-3 §5.5 rows, two ✓ and two ✗ (round 24 audit).)*
15b. **A leading marker must not let the first segment soft-wrap** (M3's ⚠): `<p style="width:10px">`
    `<span style="padding-left:20px">verylongword</span></p>` — the first content segment reaches
    `:690` with a cursor the marker has already inflated to 20, and the line must **not** be
    flushed-and-discarded out from under it. ⚠ **The mechanism sentence is rewritten in rev 34**
    (round 26, Axis 5): it read "with `on_line` now armed by the marker", which describes the
    **two-state design M3 rejected**, not the design M3 adopted. Under M3 the marker raises only to
    `BoxEdgeOnly` and the guard is an order test at-or-above `Content`, so the marker does not arm
    the guard at all — that *is* the mechanism, and this cell is what M3 cites when it says "§6
    cell 15b pins the three-state design". The **assertion** is unchanged; only its stated cause
    was the discarded alternative's.
    ⚠ This cell evaluates the guard **once**, at `BoxEdgeOnly`, where at-or-above-`Content` and
    `== Content` agree, so it cannot discriminate the two predicates. **Cell 15d does**, and that
    is why 15d exists.
15d. **The guard is an order test, not an equality** (M3's ⚠, the reader half of M7's rung) —
    `<p style="width:W"><span style="padding-left:20px">bbb ccc</span></p>` with
    **`20 + measure_width("bbb") ≤ W < measure_width("bbb ") + measure_width("ccc")`**, the shape
    cell 15c states its width constraint in, and `measure_width` the harness's (`tests/mod.rs:31`).
    The marker raises the line to `BoxEdgeOnly` and advances the cursor to 20 with no wrap check
    (M3); `"bbb "` then fits under the guard's **trimmed** test (the lower bound is exactly that
    condition, since `place_item` compares `current_inline + trimmed_width`, `pack/mod.rs:690`)
    and raises the line to its top
    rung; `"ccc"` does **not** fit, so the guard must fire and the line must wrap —
    `lines.len() == 2`, with `"ccc"` on line 2. ⚠ **What makes this the discriminating cell**: the
    guard is reached a second time with the line already above `Content`, so an `== Content`
    predicate reads false and **nothing wraps**, while the at-or-above test wraps. Cell 15b's
    single content segment never reaches that state, and cell 15 asserts that nothing wraps, so
    neither can fail on it.
    ⚠ **The upper bound is `measure_width("bbb ") + measure_width("ccc")` and *not* that sum plus
    20, and which of the two it is decides whether this cell falsifies §5.3** (round 26, Axis 3,
    Gate A). Because `place_item` advances `current_inline` by the **full** width and tests the
    **trimmed** one, the second segment's guard arithmetic is `M("bbb ") + M("ccc") > W` today and
    `20 + M("bbb ") + M("ccc") > W` from PR-1b. The `20 +` form of the bound therefore admits a
    regime — `M("bbb ") + M("ccc") ≤ W < 20 + M("bbb ") + M("ccc")` — in which **today's
    `line_count` is 1 and PR-1b's is 2**, and §5.3's PR-1b bullet says in terms that "**no cell
    pins a `line_count` change**". Dropping the `20 +` pins `W` **below** today's own wrap point,
    so the count is **2 before and after PR-1b** and §5.3's universal survives untouched, while
    the discrimination is undisturbed: at the second segment the arithmetic is true in *both*
    regimes, so under PR-1d `>= Content` flushes (2 lines) and `== Content` does not (1 line).
    That the cell sits in the already-wrapping regime is the whole of its PR-1b content — it is a
    non-regression assertion there and a discriminator only at PR-1d.
    ⚠ **The range is non-empty, measured against the harness's own font, not assumed**: it is
    non-empty iff `20 < measure_width(" ") + measure_width("ccc")`, and with `TEST_FAMILIES`'
    first resolvable family at `ComputedStyle::default().font_size` the advances are
    `M(" ") ≈ 4.45` and `M("ccc") ≈ 24.0` (Arial, `unitsPerEm` 2048, `hmtx` advances
    569 and 3 × 1024, at 16 px), giving `46.70 ≤ W < 55.14` — a window ~8.4 px wide. The margin
    over the `20` threshold is **1.42×** (measured through `elidex-shaping` as `measure_width`
    calls it: the whole `TEST_FAMILIES` list, Arial and Helvetica all give `28.45`; Hiragino Sans
    `33.70` = 1.69×; the list's other three entries do not resolve on this machine, so no "every
    family" claim is measurable here — ⚠ an earlier drafting asserted "a factor of more than two
    in every sans-serif family on the list", which no family reaches, round 26 gate 2). So no
    literal `W` is written here: the test computes it from `measure_width` the way the crate's
    justify tests do. ⚠ The padding stays `20px` and the words stay `bbb`/`ccc` **because**
    widening the window by changing them was the alternative to loosening the constraint, and the
    constraint did not need loosening.
    ⚠ **Why the wrap here is legitimate and cell 15's ⚠ does not bar it**: the two segments are
    `find_break_opportunities`' own split of **one** run (`pack/items.rs:74`), i.e. a genuine
    UAX #14 opportunity inside the text, not the item boundary
    `#11-inline-item-boundary-soft-wrap` records — that slot's subject is a flush at a boundary the
    break finder never produced.
    ⚠ **It lands in PR-1b, where the behaviour is first constructible and first true, and PR-1d's
    DoD names it as the item that must stay green**: in PR-1b the ordering has no rung above
    `Content`, so the cell passes under either predicate; PR-1d's widening is what turns it into a
    discriminator, and a cell already in the suite turns red there without anyone having to
    remember to write one. ⚠ That is also why it sits **here**, beside 15b, and not after 15c:
    §6 is grouped by **owning PR** — the four headings `PR-1a characterization`,
    `PR-1b inline-axis advance`, `PR-1c box geometry`, `PR-1d existence` — and not by cell id, so
    15c (PR-1c, because the box's rect is M4's) is two groups further down while 15d is PR-1b's.
    Within PR-1b the ids do run 15 → 15b → 15d. ⚠ A round-26 reading called the 15-family
    "out of sequence" on the strength of the file order alone; the ordering it measured against is
    not the one §6 uses, and moving 15d after 15c would move a PR-1b cell into the PR-1c block.
16. **`text-align` with a box after the space** (css-text-3 §4.1.2): `<p style="text-align:center">abc
    <span style="padding:1px"></span></p>` — the space is **no longer at the end of the line**,
    so css-text-3 §4.1.2 **step 3** ("A sequence of collapsible spaces at the end of a line is
    removed, as well as any trailing U+1680 OGHAM SPACE MARK whose `white-space` property is
    `normal`, `nowrap`, or `pre-line`") no
    longer applies to it, and css-text-3 §4.1.2 step 4's hanging — which acts on what step 3 leaves — is not reached
    either. The premise is css-inline-3 §2's "inline-axis margins, borders, and padding are
    respected between inline-level boxes": the marker's advance *is* line content after the space.
    M3 therefore zeroes `current_line_last_hang`, matching the engine's `top_group_is_line_last`
    convention (`inline/pack/mod.rs:264-279`).
    ⚠ The step-3 quotation stopped at "is removed" with no ellipsis until rev 34 (round 26,
    Axis 4); the OGHAM clause does not reach this markup, but a step quoted short is the wrong step
    for whoever implements it.
16b. **Paired contrast** — the same markup **without** the span: the trailing space must still
    hang, exactly as today. Pins that M3's zeroing is scoped to the marker and does not leak.

25. **Shrink-to-fit, max-content only** (M8): a `float: left` / `display: inline-block` containing
    `<span style="padding:10px">x</span>` must have a **max-content** inline size 20px wider than
    the same box without the padding. Its **min-content** size is knowingly unchanged — see
    `#11-inline-min-content-box-edges` in §5.3.

**PR-1c box geometry:**

6. **The emit/existence split's geometric half** (M1/M5) — the box a block-axis-only marker
   does and does not get. ⚠ On a phantom line the box gets **no rect and no paint**:
   css-inline-3 §2.3 makes the line box "and its in-flow content" non-existent, and the discard
   arm (`:428`) clears the tentative rects. The marker earns its keep only on a line that exists
   for another reason — `<p>text <span style="padding-top:10px">x</span></p>`, where M4 must fill
   all four `LayoutBox` sides. Two sub-cells: phantom ⇒ no box; co-resident ⇒ full four-sided box.
   The *cursor* half of the same markup — advance 0 — is cell 14b's, in PR-1b.
   ⚠ **This is the cell PR-1c's carrier choice can break, and the reason M4 states invariant (v)
   rather than leaving the choice unconstrained.** `entity_bounds` is written today from
   `flush_line`'s commit arm **via `commit_aligned_entity_rects`** (`pack/mod.rs:496`, inside the
   function at `:468`, called at `:344`), with `:404` the dead non-persist arm and **never** the
   discard arm (`:428`) — M4's phrasing, so the two agree. ⚠ An earlier drafting said "only inside
   `flush_line`'s commit arm (`:496`)": `:496` is not lexically inside `flush_line` at all, and
   `:496` is not the only `entity_bounds` write (round 24 audit). And
   `assign_inline_layout_boxes` iterates `entity_bounds` (`boxes.rs:56`) without asking whether the
   bounds are degenerate. A carrier writing the edges from the marker path in `pack()` would run per
   *item*, outside that decision, and grant this cell's **phantom** sub-cell a `LayoutBox` — which is
   also PR-1d's presence change arriving a PR early. That sub-cell is what turns it red.
13. `<p>a<span style="padding:10px">text</span>b</p>` — the common case §4.3 is about. Three
    assertions: (a) the span's `LayoutBox` carries real edges and its border box is
    `content + padding` **once**, not twice; (b) **`b` is displaced by 20px** — the
    user-visible half of §1.1's "respected *between* inline-level boxes"; (c) the painted
    background-colour and border rects (`crates/core/elidex-render/src/builder/paint/mod.rs:68`
    and `:382`, both `lb.border_box()`) match the border box. (`:85` is `padding_box()`, the
    background-*image* area, and is not this cell's subject.)
14c. **The empty decorated inline gains a `LayoutBox` it never had** (§7's presence change) —
    `<p>a<span style="padding:1px"></span>b</p>`: today that span has **no** `LayoutBox`
    (`assign_inline_layout_boxes` iterates `entity_bounds`, which only `place_item` populates, and
    the span owns no run), so `getBoundingClientRect` is `0,0,0,0` and `ResizeObserver` never
    observes it. After M4 it has one. Asserts **the box's presence and its geometry**, in
    `elidex-layout-block` where the producer lives. ⚠ It deliberately does **not** assert an
    observer callback: `elidex-js` depends on no layout crate (production or dev), so such a cell
    could only hand-insert a `LayoutBox` and would assert the marshalling rather than M4's producer
    — the ground §5.2 already uses to bar `elidex-dom-api` as a home. §7 records the observer
    consequence, and it is a **callback**: change detection compares the *content* size only
    (`resize.rs:259-262`, the whole `let changed = …;` statement — ⚠ every earlier drafting of this
    range, here and at §7's two sites, wrote `:260-262`, which drops `:259`'s
    `obs.last_size.is_none_or(`, the half of the predicate that makes a **box-less** target
    "changed" — round-24 gate), and this span's content height moves 0 → the line's height, so the
    observation is `changed` and an entry is delivered. ⚠ An earlier drafting called it "a changed
    `border_box_size` on an already-delivered entry": `border_box_size` is written into the entry
    (`resize.rs:271`) only *inside* the `if changed` (`:263`) the content comparison gates, so it is
    a consequence of the callback and never the trigger for one (round 24 audit).
15c. **The box does not move to the next line at its own edge** (CSS 2 §9.4.2's "cannot be split …
    overflows"; §1.3) — `<p style="width:W">aaaa<span style="padding:20px"></span>b</p>` with
    `measure_width("aaaa") < W < measure_width("aaaa") + 40`: the span's `LayoutBox` sits on line 1
    (`block_start` = line 1's) because the marker path calls the shared core with **no wrap check**
    (M3); a wrap check would have flushed before or inside the box. Asserts the box's line only,
    nothing about where `b` lands (that is the item-boundary flush, §5.3's pre-existing slot).
    PR-1c because the box's rect is M4's — at PR-1b the only channel is `b`'s `inline_start` on
    the line the item-boundary flush puts it on (0 with no wrap check, 40 or 20 with one), i.e.
    every PR-1b observation runs through the divergence the slot forbids pinning (round 22,
    Axis 2; gate).
17. **A decorated inline whose content wraps** — start marker on line N, end marker on line
    N+1; M4's per-line rebase must yield one rect per line, each with that line's
    `block_start`.
10b. **Nested boxes, stack depth > 1** (M4) — `<p>a<span style="padding:5px">b<span
    style="padding:5px">c</span>d</span>e</p>`: each span gets its own rect and its own
    `LayoutBox` edges, and the inner box's span lies within the outer's. This is why M4 is a
    **stack** rather than one open slot, and no other PR-1c cell has depth > 1.
17b. **Two producers, one fragment** (M4's ⚠): a single-line `<span style="padding:10px">text</span>`
    must yield exactly **one** `line_rects` entry — the persisting arm's per-entity fold
    (`commit_aligned_entity_rects`, `:479-487`) absorbing `place_item`'s rect and the marker's.
17c. **A box opened at a line end** — `<p style="width:100px">aaaaaaaaaa<span style="padding:5px">bbbb</span></p>`:
    the marker opens with zero span on line N, so no partial rect is emitted, but its content-start
    **must** still be rebased to 0 or line N+1's rect comes out inverted (M4).
17d. **`getClientRects`, both channels, one asserting a divergence** (M4's ⚠). Two markups:
    (a) **single fragment** — `<span style="padding:10px">text</span>` on one line stores no
    `InlineClientRects` (`boxes.rs:102`'s `len() > 1` guard), so `getClientRects` takes the
    `border_box()` fallback (`element/layout_query.rs:237`) and returns a **border area** —
    padding + border, never margin, per cssom-view-1 §6 step 3. Correct after PR-1c, and this is
    the cell that pins it. (b) **multiple fragments** —
    `<p style="width:80px">aaa <span style="padding:10px">bbb ccc</span></p>` returns per-line
    **content** spans, i.e. edges missing. Arithmetic, in the shape its siblings state it and with
    the harness's `measure_width` (`tests/mod.rs:31`, `TEST_FAMILIES`'s first family at
    `ComputedStyle::default().font_size` = 16px Arial): `"aaa "` is 31.14, so the span's
    content-start is **41.14**. `place_item`'s soft-wrap guard tests the **trimmed** width
    (`pack/mod.rs:690`, `current_inline + trimmed_width > containing_inline_size`) and then advances
    the cursor by the **full** width (`:695`), so: line 1 fits `"bbb"` trimmed (26.70) at
    41.14 + 26.70 = **67.84 ≤ 80**, leaving the cursor at **72.28** (the full `"bbb "`, 31.14);
    `"ccc"` (24.00, no trailing space) would reach **96.28 > 80** and wraps, so the box owns **two**
    `line_rects` — ⚠ an earlier drafting stated the fit test with the *untrimmed* width ("line 1
    takes `"bbb "` (31.14) to 72.28 ≤ 80"), which is the cursor position, not the quantity the guard
    compares; the measured values were all correct, but the cell's own ⚠ below already uses the
    trimmed figure, so the two halves stated different tests (round-24 gate) —
    which is what makes this the multi-fragment channel. **Asserted as an accepted divergence**,
    in the shape cell 23 uses, and routed to `#11-inline-box-decoration-splits` (§5.3 gives the two
    reasons PR-1c cannot discharge it). The two markups therefore disagree after PR-1c, and the
    cell records that rather than hiding it. ⚠ An earlier drafting wrote `width:60px`, on which the
    cell constructs a **different** case: `"bbb "` trimmed (26.70) does not fit from 41.14, so the
    box opens exactly at the break, line 1 contributes no rect, `line_rects.len() == 1`, and
    `getClientRects` takes 17d(a)'s `border_box()` fallback — cell 17c's phenomenon, not this one's
    (round 24 audit).
17f. **The multi-line `getBoundingClientRect`, pinned as accepted** (§7; cssom-view-1 §6
    *get the bounding box*) — a wrapping `<span style="padding:10px">` must return the min/max
    **union of its per-line content spans expanded by the padding on all four sides**, which is
    what `LayoutBox.border_box()` is. Two divergences from the spec's derivation, both recorded:
    (a) elidex never invokes `getClientRects()` for this at all (`element/layout_query.rs:26-31`
    → `get_border_box`; ⚠ every earlier drafting of this range, here and at §3's *get the bounding
    box* row and §7, wrote `:29-32`, which contains no part of the `get_border_box` call: that call
    is `:26`, while `:29` is only the **last line** of the `CSSOM View §5` comment — the comment is
    `:27-29`, and §3.1 lists it as a hit of its own misattribution grep — and `:30`/`:31`/`:32` are
    the scroll-offset conversion, the `Ok(dom_rect_value(…))` return and the method's closing brace.
    ⚠ A first correction described `:29-32` as "the `CSSOM View §5` comment", which is wrong in the
    other direction (round 24 audit; round-24 gate). The replacement `:26-31` is right at all three
    sites), whereas *get the bounding box* step 1 does, and its step 4 takes the
    smallest rectangle over the rects "of which the height or width is not zero" — **pre-existing,
    unchanged by this program**; (b) once PR-1c makes the edges real the two derivations stop
    agreeing, and they disagree **only at a broken edge**: expanding a union by a uniform edge is
    the union of the expanded rects, so the arithmetic itself introduces nothing, but under
    `box-decoration-break: slice` (initial) the **geometry** is css-break-3 §5.4's own definition of
    that value — "The effect is as though the element were rendered with no breaks present, and
    then sliced by the breaks afterward: **no border and no padding are inserted at a break**; no
    box-shadow is drawn at a broken edge; and backgrounds, border-radius, and the border-image are
    applied to the geometry of the whole box as if it were unbroken" —
    which is the clause this cell rests on. ⚠ The quotation stopped at "at a break" with no
    ellipsis until rev 34 (round 26, Axis 4); the restored tail is css-break-3 §5.4's own account
    of what *else* `slice` governs — box-shadow, backgrounds, border-radius, border-image — none of
    which this cell asserts, which is precisely why the elision has to be visible rather than
    silent. (CSS 2 §9.4.2's "no visual effect where the split
    occurs" is the paint-side corroboration only; a UA could report a border area it does not
    paint, so it cannot carry a claim about the reported area.) css-break-3 §5.4 then says which
    side of each fragment the broken edge is, and it is always an **inline-axis** side, so
    block-axis edges are on every fragment and the block axis agrees. elidex is therefore never
    *smaller* than the spec's box, and larger whenever a broken edge **carrying a non-zero
    inline-axis edge** owns an extreme — for a block-axis-only decorated inline the two coincide. The cell asserts elidex's value, in the shape cell 23 uses; the comparison
    is `#11-inline-box-decoration-splits`'s to settle, with cell 17d(b) as its other half.

**PR-1d existence:**

18. `white-space: pre` — `<pre> <span style="padding:5px"></span></pre>`: the line is already
    kept by clause 2 via the *preserved space text*; height must not double-count (M6 takes a
    `max`).
19. **Decorated inline adjacent to a forced break — a non-regression cell, and only one of its two
    shapes is constructible.** Assertion: `<pre>` + a decorated inline + a preserved segment break
    yields the **same line count before and after PR-1d**. Clause 5's line already exists
    independently of clause 3: `force_break` sets `any_rendered_content = true` unconditionally
    (`pack/mod.rs:781`), and M3 leaves that write **outside** `note_line_occupancy` (M3's Grounds
    dispositions it), so M5's existence flip adds nothing on such a line and must not double-count
    it. ⚠ **The parenthetical this cell used to carry — "incl. `<pre>\n</pre>` (clause 5)" — was
    spec-true and engine-false** (round 26, the §5.1/§6 consistency pass): css-text-3 §4.1.1
    preserves the segment break, so *by the spec* it is a forced break, but the engine never
    reaches `force_break` for it — the end-of-text break is filtered out of
    `find_break_opportunities` (`pack/items.rs:74`, stated at `pack/mod.rs:547-551`), and the line
    is kept by **clause 2** instead, through `contributes_content = !text.is_empty()` under `Pre`.
    That is the same predicate §5.1 M7 now names as raising the `RenderedText` rung, so the two
    statements agree. ⚠ **The `<br>` shape is NOT constructible and no cell asserts it**: `<br>`
    carries no break behaviour anywhere in the engine (§9's disposition — a missing feature, not a
    divergence), so there is no `<br>`-adjacent forced break for a marker to sit beside. A
    decorated `<br>` does still pass M1's four filters and take a marker pair; what it cannot do
    is break. When forced-break elements are implemented, this cell gains that shape.
20. `display: none` ⇒ no marker (`inline/collect.rs:218`); `position: absolute` ⇒ out of flow,
    clause 3 does not apply.
21. **Co-resident entity on a newly-existing line** — css-inline-3 §2.3's "and its in-flow
    content" means that when the line exists, every entity that has a tentative rect on it commits
    (M5). ⚠ **The co-resident must be one the engine can actually give a rect**, and on a line that
    was phantom that is a narrow set: a collapsed-away whitespace run never reaches `place_item`
    (`pack/items.rs:69` skips an empty run), and a box with **all** edges zero gets no marker from
    M1, so it has no rect either. The two producible cases are (a) a **block-axis-only** decorated
    box — `<p><span style="padding-left:10px"></span><span style="padding-top:5px"></span></p>`,
    where the second span is undecorated *in the inline axis* yet has a marker and so a rect — and
    (b) a collapsible space that survived collapse, which needs a preceding non-space and a wrap:
    `<p style="width:1px">aaaa<span> </span><span style="padding-left:10px"></span></p>`. **Both
    markups are asserted**, and each entity's rect is its **first-ever `LayoutBox`**. This is PR-1d's
    half of the presence change §7 states; PR-1c's half (cell 14c) reaches only lines that already
    existed.
22. **Both clauses are per *box*, and that is now said** (§1.5): a decorated inline **with** text
    contains glyphs, so it has no strut and must not take a strut baseline; a **glyphless** one
    does contain a strut, so M7 records a tentative for it. ⚠ **That pair is not a biconditional
    on glyph presence, and the cell does not claim it is** (round 26, Axis 4): css-inline-3 §5.3
    gives a strut to a box with "no glyphs at all, **or if it contains only glyphs from fallback
    fonts**", so a box *with* text can have one too. The second condition is carved to
    `#11-inline-fallback-font-strut` (§2's `1 × 4 (fallback-only half)` row, §5.3, §8) because
    elidex has no fallback-provenance signal; **this cell asserts the first condition only**, and
    the "with text ⇒ no strut" half is elidex's behaviour under the delivered condition, not
    css-inline-3 §5.3's whole rule. ⚠ Relatedly, M7's rung is `RenderedText`, not "glyph-bearing"
    (M3, M7): the engine's per-line signal here is `contributes_content`, which is not a glyph
    predicate — so nothing in this cell's *mechanism* turns on glyph presence either. ⚠ **Neither clause says which line's
    baseline that strut becomes** — the line-level question is cell 24's, and under M7's rule a
    glyphless box co-resident on a line that reached the `RenderedText` rung does **not** give the
    line its baseline. ⚠ **That clause said "a line that carries glyph text" until rev 34**
    (round 26, Axis 2, Gate A) — the same residue the sentence two clauses above disclaims, two
    sentences after disclaiming it: M7's gate is the rung, and the rung is
    `contributes_content`, so a line whose only text resolves no font still blocks the promote
    (cell 24) while a line at `BoxEdgeOnly` or `Content` does not. An earlier drafting left clause 2 tagged bare "(M7)" and unqualified as to line
    composition, which read as the line-level claim and contradicted cell 24; §1.5's own text
    ("If the inline box contains no glyphs at all … it is considered to contain a strut") is
    per-box, so the authority is §1.5 and M7 is only the mechanism that records it (round 25,
    Axis 2).
23. **Root-inline-box divergence, pinned as accepted** (M6):
    `<p style="line-height:40px"><span style="padding:1px;line-height:5px"></span></p>` yields a
    5px line, not the spec's 40px, because the block container's root inline box is
    unimplemented (`#11-inline-root-inline-box`). Asserted so the divergence stays
    distinguishable from a bug.
24. **No usable font: the tentative baseline must not promote** (M7): on a line whose text has no
    usable font, a co-resident glyphless decorated box's tentative baseline **does not** promote —
    the line carries text, so no strut baseline may stand in for it. The gate is the line's
    **occupancy state**, which reached the `RenderedText` rung because `place_item` was handed a
    `FlowMember::Text(_)` with `contributes_content` (M3's derivation, `pack/mod.rs:686-687`) even
    though `measure_text` returned `None`. ⚠ **The rung is named for `contributes_content`, not for
    glyphs, and this cell is the reason the distinction is not academic** (round 26, Axis 2): its
    whole premise is a line whose text resolves **no font**, so no glyph is rendered on it either —
    a rung actually keyed on glyph presence would not be raised here and the promote would fire,
    which is the defect the cell exists to catch. What the rung means is "a text segment the engine
    counts as rendered content landed on this line", and that is what
    `first_baseline.is_none()` alone cannot answer, because it stays `true` for want of a usable
    font. ⚠ **Not "raised at the `PackItem::Text` arm's call"**, as this cell said until the rev-33
    gate: `place_item`'s two callers feed one parameter, so the arm is not a condition the raise can
    test — M7's Decision carries the withdrawal and the derivation this cell now appeals to.
    ⚠ **Markup: the cell needs *two* `font-family` values, and it is not discriminating with
    one.** **Four** things must hold at once for the promote to be reachable at all — (i) the
    **decorated box's** family must **resolve**, or `FontDatabase::query` returns `None`
    (`crates/text/elidex-shaping/src/database.rs:60`) and no tentative is recorded to promote;
    (ii) the **text run's** family must **not** resolve, or `measure_text` sets `first_baseline` at
    `pack/mod.rs:586` and `is_none()` already blocks the promote with no glyph state consulted;
    (iii) no earlier line may have set `first_baseline`, which is per-IFC (`:125`, initialised
    `:187`, set `:586`/`:631`, **not** in the `:432-439` reset block) — so the cell asserts the
    **first** line of its IFC; and (iv) ⚠ **the packer must run at all.** Conditions (i)+(ii) put
    the markup squarely inside `inline/mod.rs:200`'s measurability gate — `has_text` is true
    (`:190-191`), `any_font` is false by (ii), and there is no atomic — so on `154bac3f` it returns
    `line_count: 0` at `:207` and nothing is packed. The cell exists **only because M5's PR-1d lift
    gives that gate an escape behind `has_inline_axis_edge`**, and the escape applies **only
    because the box's `padding:1px` has inline-axis components**. A plausible tightening to
    `padding-top:1px` — matching cell 5's block-axis-only shape — would leave the cell
    unconstructible while still looking like a valid fixture (rev-33 gate). So: a bogus
    `font-family` on the text run and a resolvable one on the decorated box, e.g.
    `<p><span style="font-family:'no-such-family-xyz'">x</span><span style="font-family:Arial,Helvetica,'Liberation Sans','DejaVu Sans','Noto Sans','Hiragino Sans';padding:1px"></span></p>`,
    with the box's family list being the harness's own `TEST_FAMILIES`
    (`inline/tests/mod.rs:43`) verbatim, so it resolves wherever the suite runs.
    ⚠ **The box's family must be `TEST_FAMILIES`, not the generic keyword `sans-serif`**, which is
    what this fixture wrote until the rev-33 gate. `TEST_FAMILIES` is
    `["Arial", "Helvetica", "Liberation Sans", "DejaVu Sans", "Noto Sans", "Hiragino Sans"]` and
    contains no generic keyword; `FontDatabase::query` maps `"sans-serif"` to
    `fontdb::Family::SansSerif` (`elidex-shaping/src/database.rs:70`), which `fontdb::Database::new`
    resolves to the single literal **`"Arial"`** and which nothing in the workspace re-points
    (`grep -rn 'set_sans_serif_family' crates/` → no hits). On a machine without Arial the box's
    family does not resolve, condition (i) fails, and the cell stops discriminating — exactly what
    the ⚠ above it exists to prevent, and the reason the whole list is written rather than any
    subset of it: the six exist because no proper subset covers every platform the suite runs on.
    ⚠ **Cell 12d's harness switch defeats this cell and must not be used here**: `setup_inline_test`
    (`inline/tests/mod.rs:54`) gives PR-1a "a deterministic way to force `any_font == false`", and
    `any_font` is `items.iter().any(…)` over **text runs** (`inline/mod.rs:192-199`), so forcing it
    false means *no* run resolves — a whole-IFC font-less style under which the **box's** family
    does not resolve either, condition (i) fails, no tentative exists, and the cell passes with or
    without the gate. ⚠ An earlier drafting pinned this as an **accepted divergence** — "the
    tentative promotes even though the line has glyphs" — which contradicted M7's own Decision,
    where the residual is *fixed in PR-1d, not deferred*. M7 is the design authority (a Decision
    cell) and CLAUDE.md's *TODO 先送り禁止* backs it, so the cell asserts the fixed behaviour and
    the divergence reading is withdrawn (round 24 audit). ⚠ And a first version of the *fixed* cell
    named no markup and would have been run under that same switch — non-discriminating, the class
    cell 9 records above (round 25, Axis 2).
24b. **The cancelling pair keeps its line** (M5's disjunction, the half cell 3b cannot assert) —
    `<p><span style="margin-left:-10px;padding-left:10px"></span></p>`: the inline-start
    components sum to zero, so the box advances the cursor not at all, yet css-inline-3 §2.3
    counts "non-zero **inline-axis** margins, padding, or borders" as three separately-named
    quantities, so the line is **not** phantom and a `LineBox` is emitted. ⚠ Shape B, so the gate it
    depends on is `items.is_empty()` (`inline/mod.rs:161`) — cell 12's, not the `any_font` outer
    condition, which `has_text == false` makes unreachable for a marker-only stream. Constructible only here,
    because the substitution of M5's expression for PR-1b's constant `false` is what makes the
    disjunction reach line existence.

**Non-regression throughout**: `collapsible_whitespace_only_generates_no_line_box`
(`crates/layout/elidex-layout-block/src/inline/tests/text_height/basic.rs:204`) and
`nbsp_only_line_generates_a_box` (`:233`).

**Test placement** (§9): PR-1a's cells land in `inline/tests/decorated_inline/stream.rs`, PR-1b's
in `…/advance.rs`, PR-1c's in `…/geometry.rs`, PR-1d's in `…/existence.rs` — a new
`decorated_inline` module split per PR
from the start, per [[feedback_touch-time-split-means-while-writing]], rather than one file grown
to hold every cell. None of them go in `tests/text_height/basic.rs` or `relpos_subflow.rs`. ⚠ **Cells 10 and 11 have no relpos
variant**, and nothing here needs one: the memo's only relpos content is the `group_key` sub-flow
keying, which §2's pair 2×6 and §5.3 route out to `#11-inline-box-decoration-splits`, so that
question is that slot's. An earlier drafting said "the relpos facet of cells 10/11 lands with the
new module too", naming a facet no cell has (round 24 audit).
⚠ **One cell is not wholly in that module, and the blanket sentence above does not reach it**:
cell **13(c)** asserts painted rects, which only `elidex-render` can observe (§5.2 routes it there,
and that crate depends on `elidex-layout-block` so it can run layout). Its (a) and (b) arms stay in
`…/geometry.rs`. Every other cell is where the sentence says.

## §7. Downstream

* `line_count`, IFC `height`, block cursor — **PR-1b moves the cursor and, where following text
  then reaches the wrap guard, all three**; cells 12b and 15 pin the cursor, no cell the count
  (§5.3). PR-1b changes no line's *existence*; that is PR-1d.
* `first_baseline` — a box's tentative promotes into it only while `first_baseline.is_none()`
  **and** the line's occupancy never reached the `RenderedText` rung (M7). ⚠ **Not "only on a
  glyphless line", and not "cell 22 asserts both directions"** — both stood here until rev 34
  (round 26, Axis 2, Gate A). "Glyphless" is the predicate Part B withdrew everywhere else:
  `RenderedText` is derived from `contributes_content`, which is not a glyph test, and M7's own
  Grounds record `<pre>\n</pre>` reaching the rung on a line that renders no glyph — a *glyphless*
  line that nonetheless blocks the promote. And the assertion is **cell 24's**, not cell 22's:
  cell 22 is the **per-box** pair (with text ⇒ no strut / glyphless ⇒ strut) and says so itself
  ("Neither clause says which line's baseline that strut becomes — the line-level question is
  cell 24's"), while cell 24 is the line-level one and pins the negative direction, a co-resident
  box's tentative not promoting on a font-less text line.
* `entity_bounds` / `getClientRects` / `getBoundingClientRect` — **split by LINE count, which is
  the only partition the engine can express.** `commit_aligned_entity_rects` folds to one entry per
  entity per line (`pack/mod.rs:479-487`) and `boxes.rs:102` gates on `line_rects.len() > 1`; there
  is no fragment unit anywhere below, because runs persist in **logical** order and the UAX #9 L2
  reorder belongs to render (`inline/mod.rs:215-217`, `collect.rs:303-305`). cssom-view-1 §6 step 3
  asks for one rect **per box fragment**, so one-per-line is an approximation, exact only when each
  line carries one fragment of the box. Three states, not two:
  * **One line, one fragment** (the common case): no `InlineClientRects` is stored, so
    `getClientRects` takes the `border_box()` fallback and `getBoundingClientRect` reads the same
    box. Both are correct after PR-1c, and cell 17d(a) pins it.
  * **More than one line**: `getClientRects` answers from stored **content** spans — edges missing,
    *under*-inflated (cell 17d(b)). `getBoundingClientRect` does not read that list at all
    (`layout_query.rs:26-31` → `get_border_box`); it reads `LayoutBox.border_box()`, i.e. the
    min/max **union** over lines (`boxes.rs:65-81`) expanded on all four sides. ⚠ Expanding a union
    by a uniform edge *is* the union of the expanded rects, so the arithmetic alone diverges from
    nothing; the divergence PR-1c introduces is at the **broken edges**, where CSS 2 §9.4.2's "no
    visual effect where the split occurs" and css-break-3 §5.4's parent-direction rule mean a
    fragment owning an extreme may carry no edge there. Cell 17f pins it, and every paint and
    hit-test consumer below inherits the same box.
  * **One line, more than one fragment** — the case css-inline-3 §2.1's Note names ("split into
    several fragments within the same line box due to bidirectional text processing"). It falls in
    the *first* bullet by line count, and returns one rect where the step asks for one per
    fragment. ⚠ **This is pre-existing and this program does not touch it**: the count is already
    wrong on `154bac3f`, and layout cannot represent the fragments at all. css-break-3 §5.4 asks
    for `box-decoration-break` at "bidi-imposed breaks" too, so it lands with the same owner.

  All three residues route to `#11-inline-box-decoration-splits`; they are one missing per-fragment
  attribution seen from three sides, not three defects.

  PR-1d adds the co-resident commit (cell 21).
* **Painted output** — PR-1c changes background and border rendering for **every** decorated
  inline (`crates/core/elidex-render/src/builder/paint/mod.rs:68` background colour, `:382`
  borders; `:85` is the background-image area and follows the padding box). Largest delta in the
  program; cell 13 is its pin.
* `last_placed_entity` / persisted run shape — deliberately changed by M3 (§1.4); cell 14.
* `current_line_last_hang` — **zeroed at a marker with an inline-axis edge** (M3), left alone
  otherwise; cells 16 and 14b.
* **Every reader of an inline `LayoutBox`.** **Three** distinct changes, in three different PRs:
  1. ⚠ **Position — PR-1b, and the re-slice's shippability argument does not cover it.** M3's
     advance moves `current_inline`; `place_item` snapshots `seg_inline_start` from it
     (`pack/mod.rs:694`, `:706`) and that reaches `LayoutBox.content` (`pack/boxes.rs:81`). So every
     painted rect, `getBoundingClientRect().x`, `offsetLeft`, hit-test area, a11y node bounds and
     observer geometry **shift** in PR-1b, for every decorated inline with content. §5.3 and §8 say
     PR-1b changes no painted *extent* — true, and not the same claim. PR-1b's DoD therefore carries
     its own reader disposition, `elidex-render`'s existing `border_box()`-reading paint tests
     included.
  2. **Value** — M4 flips `padding`/`border`/`margin` from a hard-coded `EdgeSizes::default()` to
     real values, so consumers that were reading a constant now read data. **PR-1c.**
  3. ⚠ **Presence, and it lands in TWO stages, in two different PRs.** Absence is *observable* at
     the readers below, which use `.ok()?` / `map_or` rather than a default, so each stage is
     JS-visible. Today an empty decorated inline has no `LayoutBox` at all: the only producer of
     `current_line_entity_rects` is `place_item` (`:706`), which needs a run, and
     `assign_inline_layout_boxes` iterates `entity_bounds` (`boxes.rs:56`).
     * **PR-1c (M4) — on lines that already exist.** M4 adds a *second producer* (`InlineBoxEnd`
       pops the stack and pushes an entry), so an inline that owns **no run** gets an
       `entity_bounds` entry and therefore its **first-ever `LayoutBox`** — but only where the line
       commits, which in PR-1c still means a line kept by something other than the decoration. On
       `<p>a<span style="padding:1px"></span>b</p>` (cell 14's markup) `getBoundingClientRect` goes
       `0,0,0,0` → a real box, a11y gains node bounds, and the span becomes hit-testable. Cell 14c.
     * **PR-1d (M5's flip) — on lines that did not exist.** Flipping the predicate moves whole
       lines from discard to commit, and css-inline-3 §2.3's "the line box **and its in-flow
       content**" means *every* entity on such a line commits. So the grant is **wider** here than
       in PR-1c: the decorated inline itself in Shapes A and B — which have no committing line at
       all before the flip, so M4 alone never reaches them — and every co-resident entity **that
       has a tentative rect on the line**. ⚠ Not "every co-resident": on a formerly-phantom line an
       entity has a rect only if `place_item` ran for it (so, a collapsible space that survived
       collapse) or M1 emitted a marker for it (so, at least one non-zero edge on some side). A box
       with all edges zero gains nothing. Cell 21 gives both producible markups, and the flip of
       cells 7 and 8 covers the box itself.
     ⚠ **The observers are the exception — and two earlier readings of them were wrong.**
     `elidex-api-observers` deliberately does *not* skip a box-less target (`resize.rs:251-254`,
     and `intersection/mod.rs:298-300` for the sibling observer), so both already fire for that span
     today. ⚠ **The citation for that was carried from the code comment, which this memo's front
     matter forbids**: the comment says "per Resize Observer §2.1 (observe)", but
     `body resize-observer-1 resize-observer-interface` gives `observe()` four steps with no
     initial-entry statement at all. The mechanism is `lastReportedSizes = [(-1,-1)]`
     (`dfn resize-observer-1 lastReportedSizes` → **resize-observer-1 §3.1 ResizeObservation example struct**, which
     self-declares **non-normative**), which makes `isActive()` true on the first pass. And
     `intersection/mod.rs:298-300` is **not** "the same" citation — that comment cites Intersection
     Observer §2.2, a different spec.
     ⚠ **"No new callback fires" is also not derivable the way it was stated.** Change detection
     compares the **content** size only (`resize.rs:259-262`) — that part holds — but the ground
     offered was resize-observer-1 §3.3.1's "non-replaced inline Elements will always have an empty
     content rect", **which elidex does not implement**: the size source is
     `lb.content_rect_local()` (`crates/script/elidex-js/src/vm/host/resize_observer.rs:404-407`),
     and `content_rect_local` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:204-211`)
     returns the real content span with no inline special case. ⚠⚠ **This memo therefore states the *mechanism* and stops deriving a global "no new callback
     fires" conclusion — three successive revisions asserted one and all three were wrong, each in a
     different direction.** The mechanism, measured: the detector reads
     `size_fn(...).unwrap_or((Rect::default(), Size::ZERO))` (`resize.rs:255-256`), so a **box-less
     target reads as (0, 0)**; it compares **both axes** (`resize.rs:259-262`, `width` **and**
     `height`); and the size it compares is `LayoutBox::content_rect_local()`
     (`crates/script/elidex-js/src/vm/host/resize_observer.rs:404-407`), whose height is
     `content.size.height` verbatim (`layout_types/boxes.rs:204-211`). An inline `LayoutBox`'s
     content height is the **line's** block size, not the box's glyph extent —
     `commit_aligned_entity_rects` writes `block_size: line_height` (`pack/mod.rs:493`) and
     `assign_inline_layout_boxes` takes `bounds.block_end - bounds.block_start` (`boxes.rs:77`).
     <br>**Consequence, per stage, and it is a callback in both.** **PR-1c**: cell 14c's
     `<p>a<span style="padding:1px"></span>b</p>` span has no `LayoutBox` today, so it reads (0, 0);
     M4's `InlineBoxEnd` pop gives it one with `content.size == (0, line_height)`, and the line
     carries `a`/`b` so `line_height > 0` — **height moves and a callback fires**. ⚠ An earlier
     revision said "zero-width content spans by construction", which quantifies over one axis while
     the detector reads two. **PR-1d**: cell 21(b)'s co-resident surviving collapsible space gains
     its first-ever `LayoutBox` with a non-zero *width* too, so a script mutation adding an
     inline-axis edge to *one* span fires a callback on a **different** element.
     <br>Both are new JS-observable behaviour; each PR's DoD carries its own stage through §7's
     reader disposition, which is where the check belongs. What this paragraph does **not** do is
     assert a conclusion the DoDs then inherit unexamined. §10's
     `#11-layoutbox-absence-unreachable` row states only the opposite direction (M5 withholds no
     box); both stages above are this program **granting** a box where absence was the truthful
     signal, which is why that row is booked to PR-1d — the PR whose flip makes its statement true —
     rather than to the PR that first grants one.

  ⚠ **This family is a second method over a set the repo already inventories mechanically.**
  `.claude/tools/layout-box-reader-allowlist.tsv` is the CI-enforced SoT (the ungated `trip-wires`
  job), keyed `<classification>\t<path>\t<content>`, and it already classifies files this program
  writes — `inline/pack/boxes.rs` as `producer`, `inline/pack/mod.rs` as `pending-migration:C-3c`.
  **PR-1c reconciles against it rather than re-deriving**: the grep below is the discovery method,
  the allowlist is what must end up right. PR-1c's DoD is to re-run the wire and record the delta —
  expected empty, per the ⚠ below; the two ways it fails are an `added` row (a novel reader line)
  and a `removed` row (an edited or moved one), and cell 13's paint assertions are the likeliest
  source of either. ⚠ Two wires, and only one is blind here: **wire #1** (the allowlist) *does* see
  `let lb = LayoutBox {` — it is allowlist row 75, for the very file M4 edits — while
  `#11-layoutbox-field-typed-reader-coverage` scopes its blindness to **wire #5**, the
  write-chokepoint ban, which "cannot see `let lb = LayoutBox{…}; insert_one(e, lb)`".
  ⚠⚠ **But both wires key on FILES, never on FIELDS, and so does the grep below — which is how this
  program carried a spec violation past three rounds of its own reader audits.** The allowlist row is
  `<classification>\t<path>\t<content>` where *content* is the **acquisition** line, so a file
  already listed can read any field of `LayoutBox` without moving a single key; and the discovery
  grep `border_box()|padding_box()|margin_box()` cannot see `lb.border.top` at all.
  `crates/dom/elidex-dom-api/src/element/layout_query.rs:133-151` is exactly that shape, and
  cssom-view-1 §6 requires `clientTop`/`clientLeft` to be **zero for an inline box** — which elidex
  satisfies today only because the edges are hard-zeroed. **Neither instrument could have found
  it.** (What the instruments' blindness surfaced is a *defect*; the defect turned out to be
  four-membered, cross-crate and keyed on a defined term elidex only half-implements, so it is
  carved to its own PR — §9 — rather than fixed as a cell here. The audit lesson stands whatever
  owns the fix.) So this family list is **evidence, not an inventory**: the DoD is a reconciliation against
  the allowlist *plus* a hand audit of field reads, and the absence of a mechanical backstop for the
  latter is `#11-layoutbox-field-typed-reader-coverage`'s (already open, from #488) — this program
  does not close it, and says so rather than implying completeness. The field-level readers it must
  therefore carry by hand: `clientTop`/`clientLeft` (`lb.border.*` — the violation above, **carved
  out**: the predicate prereq PR fixes it, landing before PR-1a, §9)
  and `ResizeObserverEntry.contentRect` via `LayoutBox::content_rect_local`
  (`crates/core/elidex-plugin/src/layout_types/boxes.rs:204-211`), whose **origin** moves from
  `(0,0)` to `(padding.left, padding.top)` at PR-1c — a moved value, not a new callback, since
  change detection compares `.size` only. And because
  wire #1 keys on the `(path, content)` of *token-bearing* lines, M4's edit — the field values
  inside that literal — changes no keyed line, so the reconciliation below is expected to resolve
  to **no allowlist delta**; PR-1c's DoD is to confirm that, not to assume it. Discovery grep (misses `BoxModel`
  accessor calls and direct field reads, which is why it is not the SoT):
  `grep -rn "border_box()\|padding_box()\|margin_box()" crates/`. Dispositioned by family:
  * `elidex-render` paint (`paint/mod.rs`), `walk.rs`, `slice.rs`, `transform.rs` — the intended
    correction; cell 13(c) pins the paint half.
  * `elidex-dom-api` `element/layout_query.rs` — `getBoundingClientRect`, `offsetLeft`/`offsetTop`
    and the offsetParent walk. **JS-observable**, and correct once the edges are real; needs a cell.
  * `elidex-layout/src/hit_test.rs` — a decorated inline's hit area grows by its edges. Correct.
  * `elidex-shell/src/content/scroll.rs` — scrollable-overflow extent and scroll-into-view. Correct.
  * `elidex-a11y/src/tree.rs` — node bounds. Correct.
  * `elidex-js` `intersection_observer.rs` / `resize_observer.rs` — observed box sizes. Correct,
    and JS-observable.
  * `elidex-layout-multicol`, `-table`, `-flex`, `-grid`, `block/mod.rs`, `block/children/stack.rs`,
    `inline/atomic.rs` — block/atomic/abspos paths, unreached for a
    non-replaced inline. `elidex-ecs/src/dom/geometry.rs` likewise, though for a different reason:
    `union_border_boxes` has no non-test consumer yet.
  * ⚠ **`elidex-css/src/page.rs` is not a reader and is dropped from this family** — a grep
    artefact, in the class the `elidex-ecs` entry above already flags. Its single hit is
    `page.rs:487` `fn parse_page_margin_box() {`, a `#[cfg(test)]` function **name** the discovery
    grep matched as a substring of `margin_box()`; `elidex-css` owns no layout geometry and has no
    `LayoutBox` read at all. Recorded rather than silently deleted, because it is the second way
    this family list fails as an inventory (the first being the field-read blindness above): the
    grep both under-collects and over-collects, so membership must be derived from what the file
    does, not from the hit (round 25, Axis 1).
  None is wrong-after-M4 **for an inline occupying one line**. For a multi-line one every reader
  in this list inherits the over-inflated union above — that is one divergence with many consumers,
  not many divergences, and cell 17f is its single pin. The point is that PR-1c's DoD must reach
  past paint, which §8 says.
* Static positions of following abspos (`inline/pack/mod.rs:97`).
* `current_line_runs` / `flow_lines` — markers are never `FlowMember`s, so member-less buckets
  keep being dropped by design; no ordinal dependency (§2's non-invariant).
* `clear_inline_flows` probe gating — a decoration-only IFC stops taking the early return at
  `inline/mod.rs:161` (unconditional clear) and starts taking the full path (`:637` — a pre-move
  coordinate; that line lives in `inline/reconcile.rs` since #508 — a *path* change for the IFC,
  not an edit of that file, which is already `!env.is_probe`-gated). **PR-1d**, per §5.3.
* Fragmentation (`inline/pack/fragment.rs`) and `ColumnFlowSlice`.

## §8. Definition of done

**PR-1a** (item stream, behaviour-neutral): both marker variants reach `pack/items.rs`; every
exhaustive match handles both (`inline/whitespace.rs:41`, `inline/mod.rs:192-199`,
`inline/pack/items.rs:67`, `inline/tests/mod.rs:20-23` — `atomic.rs:39`, `measure.rs` and
`collect.rs:181` are `if let` and need none); **the payload exactly as M1 specifies it, not
restated here**; `containing_inline_size` threaded through every `collect_inline_items` caller
(§5.2 enumerates them from the grep that defines the set); the two pre-pack gates hold current
behaviour; **no citation work** (all of it is `#11-inline-spec-cite-misattribution`'s, §9); new variants carry docstring citations
to their §3 rows; **two test-harness additions, without which cells 12d, 6b, 6g and 6h are
unconstructible**: `setup_inline_test` (`inline/tests/mod.rs:54`) gains a deterministic way to
force `any_font == false`, and a helper beside `collect_styled_runs` (the `fn`, `:11-25`) returns the
`InlineItem`s rather than `filter_map`ping them to `Vec<StyledRun>` — the existing one discards
every non-`Text` variant, so it cannot observe a marker; cells 1, 2, 5, 6b, 6c, 6f, 6d, 6e, 6g, 6h, 7–12 and 12d land
here — as characterization tests except 6b, 6g and 6h, which pin new item-stream behaviour
(§5.3); zero layout-output change. **Dead-field rule**: fields
whose first reader is a later PR are added by that PR — M6/M7's `line_height` and font identity by
**PR-1d**, `group_key` by `#11-inline-box-decoration-splits` — so nothing ships unread and no
`#[allow(dead_code)]` is needed. The rule reaches M1's own payload too: PR-1a's variants carry
**`entity` only**, since the three `EdgeSizes` and the `WritingModeContext` have no PR-1a reader —
the emit test resolves them at collect time and discards them, and the packer's `match pi` arm is a
no-op. They arrive in **PR-1b** with their first readers M3 and M5. ⚠ M4 (PR-1c) is a **third**
reader of the same triple, not a second producer of it: it carries the payload's values through to
`LayoutBox`, which is why §5.1 M4's invariant (i) is one `resolve_box_model` call per pass rather
than two. **What carries them is PR-1c's memo's choice**, so PR-1a's DoD says nothing about it.

**PR-1b** (inline-axis advance): cells 3, 3b, 4, 12b, 12c, 12e, 14, 14b, 15, 15b, 15d, 16, 16b and 25.
`note_line_occupancy` is the **only** writer of the five line-state fields, called by both
`place_item` and the marker path; `on_line` is gone, replaced by the three-valued
`line_occupancy` whose reset joins `flush_line`'s per-line block; `has_inline_axis_edge` and M3's
sums live in `pack/inline_box.rs` as free functions, callable from `inline/mod.rs:200` before any
`LinePacker` exists; the shaping break and the hang gate both read the predicate; M8 adds the
max-content contribution and **not** the min-content one. ⚠ **The box's own geometry is
untouched**: `inline/pack/boxes.rs:82-84` still writes `EdgeSizes::default()`, so `border_box()` is
still `content` and no painted rect changes extent — the property §5.3 gives as the reason this PR
can precede PR-1c. `pack/mod.rs:726`'s fabricated citation is rewritten here (§3.1) — the one cite
this program corrects, because this is the PR that changes what that comment documents; no other
citation work (§9). ⚠ **A reader disposition of its own**: PR-1b moves the *position* of
`LayoutBox.content` **for every decorated inline with content** — §7's first numbered change, whose
qualifier this sentence used to drop by writing "every inline" (round 25, Axis 3) — so this PR — not only PR-1c —
re-checks §7's reader family, and `elidex-render`'s existing `border_box()`-reading paint tests are
named and dispositioned here. `note_line_occupancy`, `pack/inline_box.rs` and M8's contribution
carry docstring citations to their §3 rows.

**PR-1c** (box geometry): cells 6, 10b, 13, 14c, 15c, 17, 17b, 17c, 17d and 17f — ⚠ **all of them
assert a first-layout property**, because `assign_inline_layout_boxes` skips any entity that
already carries a `LayoutBox` (`boxes.rs:62-64`) and **no production site removes one**: wire #5 of
`.claude/tools/layout-box-reader-trip-wire.sh` bans the `remove_one::<LayoutBox>` shape outside
test paths, and the `trip-wires` CI job that runs it is ungated (#496), so the property is
machine-enforced rather than a grep count restated here — the one live hit is under
`crates/script/elidex-js/src/vm/tests/` and is test-only. ⚠ State it at the wire's own reach: that
script's header disclaims the stronger reading, so what is enforced is "no **production** removal",
not "no removal". So no geometry this PR writes is refreshed on a relayout. That is total and
pre-existing — the same skip already freezes `LayoutBox.content` — and
`#11-inline-relayout-box-staleness` owns it (§9); the DoD states it rather than implying a
JS-observable-after-mutation guarantee it cannot give. **§7's full `LayoutBox`-edge reader list
dispositioned, not just paint** (a `getBoundingClientRect` assertion included, made through the
layout-level channel §5.2 names, not in `elidex-dom-api`) — **and dispositioned over a second entity
class, generated content**: a decorated `::before`/`::after` (the 6g class, pinned under PR-1a) gets real edges here too, so
every reader listed for elements is re-read for pseudo entities (render's `builder/inline.rs:205`
walks them by `PseudoElementMarker` and `walk.rs` paints on `ComputedStyle` + `LayoutBox`, which
a pseudo already carries today through its own `place_item` run — cell 13's delta on a second
class, no new reader) — and §7's allowlist reconciliation re-run
with its delta recorded — ⚠ **plus the hand audit of *field* reads that neither wire nor the grep
can perform** (§7). ⚠ **That audit gets a population and a command, not a completion clause**: what
both instruments are blind to is a direct edge-field read off a `LayoutBox` binding (`lb.border.top`
— §7 states the blindness), so the audit's enumeration is the output of
`grep -rEn '\.(border|padding|margin)\.(top|right|bottom|left)\b' crates/`
— **over `crates/` whole, not over a hand-written crate list**, **run in the PR with its output
recorded**, each hit classified as a `LayoutBox` read or not. ⚠ **It was scoped to a list until
rev 34, and the list omitted the crate holding the audit's own worked example** (round 26,
Axis 1): the list named `elidex-render`, `elidex-dom-api`, `elidex-layout`, `elidex-shell`,
`elidex-a11y`, `elidex-js`, `elidex-layout-block` and its four siblings, and `elidex-ecs` — of
which four (`elidex-shell`, `elidex-a11y`, `elidex-js`, `elidex-ecs`) return zero hits, while
**four crates that do have hits were not on it at all**. ⚠ **The complement is stated rather
than exemplified, because naming one omission is the very thing this clause forbids** (round 26,
Axis 1, Gate A; rev 34 wrote "`elidex-plugin` was not on it"). Run over `crates/` whole at
`154bac3f` the grep returns **171** lines in twelve crates; subtract the list and **27** lines in
**four** crates remain — `elidex-css-box` 9, `elidex-style` 6, `elidex-plugin` 6,
`elidex-css-anim` 6. Twenty-five of the twenty-seven are `ComputedStyle` reads and writes, the
over-collection this clause accepts by design; the other two are
`elidex-plugin`'s `boxes.rs:206-207`, `content_rect_local`'s body, which the paragraph below
dispositions by name — so the list's omission hid the audit's own worked example inside a crate
the same paragraph calls "already dispositioned". A population defined by a
hand-written list of crates cannot discover a crate; scoping it to `crates/` and discarding
non-`LayoutBox` hits at classification is the shape the clause already argues for one sentence
later. It deliberately
over-collects — `ComputedStyle` edge reads match too — because an audit that under-collects cannot
be reviewed for completeness, and this is the shape that reaches
`element/layout_query.rs:139`/`:149`, the very reads §7 measures both instruments missing. **No
count is written here**: the command's output *is* the population, so it cannot go stale against the
tree, and completeness is reviewable against a re-run rather than against an intention (round 25,
Axis 3; the clause it replaces named two known reads and no artefact, in a DoD whose every sibling
clause is checkable). Of those hits, two are already dispositioned and **neither is a cell of this
PR**: the
`clientTop`/`clientLeft` guard, carved to the predicate prereq PR, which lands before **PR-1a** and
so is already in `main` here (§9) — PR-1c's obligation is the **integration** assertion below, not
the guard itself; and `content_rect_local`'s origin moving from `(0,0)` to
`(padding.left, padding.top)` — **a JS-observable value this PR moves, and the memo accepts the
exposure explicitly rather than pinning it**. What makes it cell-free is not invisibility
(`ResizeObserverEntry.contentRect` reports it) but entailment:
`LayoutBox::content_rect_local` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:204-211`) is a
**total function of `padding` and `content.size`** — `Rect::new(padding.left, padding.top,
content.size.width, content.size.height)`, with no inline special case — and PR-1c changes neither
the function nor any input beyond the `padding` cell 13(a) already pins. ⚠ **That purity is pinned
nowhere, and saying it here does not pin it**: the function is `elidex-plugin`'s and its
correspondence to those two fields is that crate's contract. ⚠ **What does *not* follow, and what
rev 33 wrote, is that a cell here "could only assert it through a hand-inserted `LayoutBox`"** —
**measured false** (round 26, Axis 1): `content_rect_local` is `pub`
(`elidex-plugin/src/layout_types/boxes.rs:204`), `elidex-layout-block`'s `[dependencies]` carry
`elidex-plugin`, and this crate's own inline harness already reads **real post-layout** boxes out
of the world — `dom.world().get::<&LayoutBox>(span)` at `inline/tests/inline_flow/align.rs:58`,
`:87` and `:119` — so a cell could call `lb.content_rect_local()` on a produced box with no
hand-insertion anywhere. The exposure is therefore **accepted despite being assertable**, and that
is the disposition: the value is entailed by an input cell 13(a) already pins, so a cell here would
re-assert `elidex-plugin`'s own contract rather than anything PR-1c decides — a reason of
redundancy, not of impossibility. ⚠ This is the error class §5.2 already records once ("the
reachability an earlier revision denied by measuring adjacency instead"); the rev-34 sweep re-read
the memo's three other unreachability grounds and found them sound — `elidex-dom-api` has no
layout dependency and no `[dev-dependencies]` at all (§5.2), and `elidex-js` depends on no layout
crate in either table (cell 14c), both re-measured against `Cargo.toml`. So the moved origin is
**accepted, on the record**, not absent (round 25, Axis 3;
an earlier drafting said only "a cell here would assert a pure function of an already-asserted
input", which reads as *no exposure* rather than *an accepted one*). §7 records the observer consequence
(a moved `contentRect`, no new callback, since change detection compares `.size` only).
⚠ **The integration assertion the withdrawn `clientTop`/`clientLeft` cell carried needs an owner, and no crate can host it whole.**
Its content was "real `LayoutBox.border` **and** the guard ⇒ `clientTop` is 0" — one claim spanning a
layout producer and a DOM-API reader. ⚠ **An earlier drafting of this paragraph claimed it had nowhere to live, from a command that
measures the wrong thing.** `grep -q` for both crate names over every `crates/**/Cargo.toml` does
return no match — but that is *direct* dependency, not reachability. **`elidex-shell` reaches both
at one hop** (`elidex-dom-api` + `elidex-layout` → `elidex-layout-block`) — ⚠ it is the only
usable joint host: an earlier drafting added "as does `elidex-render`", but that is Cargo-graph
reachability, not nameability — `elidex-render` has no `elidex-dom-api` dependency and
`elidex-form` re-exports one `is_option_disabled` (`git grep 'pub use elidex_dom_api' 22de3078 --
crates/dom/elidex-form/src/`); and `elidex-shell`'s suite already drives
the whole pipeline, HTML + CSS + JS to a display list (`crates/shell/elidex-shell/src/tests.rs:41`,
`:52`). So the end-to-end cell **is** constructible, and PR-1c's DoD takes it rather than a
decomposition. The canonical predicate is still worth taking on its own ground (§9's
one-issue-one-way argument); it is **not** additionally justified by a testability gap that does not
exist. **The obligation, stated once**: for
`<p>a<span style="border:5px solid">text</span>b</p>`, run through `elidex-shell`'s pipeline, the
span's `LayoutBox` carries `border == 5` **and** `clientTop` returns **0** — real producer, real
reader, one assertion; and, in the same clause, a second markup through the same pipeline —
`p::before { content: "x"; padding: 10px }` on `<p>ab</p>`: the pseudo's `LayoutBox` carries
`padding == 10`. That second markup is the joint site that discharges cell 6g's cascade
stipulation (`display` default + padding on a pseudo) — `elidex-shell` is the host whose suite runs
cascade **and** layout end-to-end (it depends on `elidex-style` directly and on `elidex-layout-block`
through `elidex-layout`, `crates/shell/elidex-shell/Cargo.toml`; `elidex-render` also depends on both
but drives no pipeline and has no `elidex-dom-api` edge, above), and `elidex-style`'s own
`src/tests/selectors_pseudo.rs` at `22de3078` asserts only `TextContent`/`color` (and marker
presence) on pseudos, never `display` or an edge. It is a PR-1c DoD clause, not a §6 cell of this umbrella, because §6's cells
are layout-crate cells and this one is not. ⚠ The two in-crate halves (`elidex-layout-block`: the
box carries `border == 5` and its entity satisfies the predicate; `elidex-dom-api`: a hand-inserted
box + the predicate ⇒ 0) remain worth having as fast unit cover, but they are **not** the discharge —
an earlier drafting made them the discharge because it believed the end-to-end cell impossible.
No double-counted edges on either side; the `InlineClientRects` write path
is **untouched** — PR-1c's whole CSSOM effect is that the `border_box()` fallback becomes correct
once `LayoutBox` carries real edges (cell 17d(a)), so the cssom-view-1 §6 and css-break-3 §5.4
citation obligations travel with the derivation to `#11-inline-box-decoration-splits`; the open-box
stack and its `flush_line` hook carry docstring citations to their §3 rows.

**PR-1d** (existence): cells 18, 19, 20, 21, 22, 23, 24 and 24b, plus the flip set — **cells 1, 2, 7, 8, 10, 11, 12 and 12d flip; 5, 6b, 6c, 6f, 6d, 6e, 6g, 6h and 9 do not**; this is the one normative statement of it — every §7
consumer checked, **including the second and wider stage of the presence change §7 states**: every
entity on a line the flip moves from discard to commit gains its first `LayoutBox`, so the same
reader list PR-1c dispositioned is re-checked against a *newly box-bearing* entity rather than a
newly *edged* one, and §10's `#11-layoutbox-absence-unreachable` row lands here for that reason; the ⚠ caveat at `inline/pack/mod.rs:110-114` **is replaced by one** naming the root-inline-box
divergence and pointing at `#11-inline-root-inline-box`. ⚠ **Replaced, not "retires"**: on
`154bac3f` that caveat names the *decorated-empty-inline* drop and points at
**`#11-line-box-decorated-inline-content`** — this umbrella — so what PR-1d discharges is its
current subject, and the sentence above described the caveat as it will be, not as it is
(rev-33 gate). M7's promotion carries docstring
citations to their §3 rows **and its occupancy gate, which is five checkable items and not a
field pair**: (i) the line-occupancy ordering gains its **fourth `LineOccupancy` variant,
`RenderedText`** — the mechanism M7 decides rather than delegates (⚠ an earlier drafting wrote "a
fourth variant or the sibling monotone enum PR-1d's own memo may pick"; a sibling enum is a second
per-line field, which this very DoD's closing clause and §5.4 both forbid, so the two branches
differ in correctness and the choice is not PR-1d's to make — round 26, Axis 2);
(ii) the occupant carrying that rung is **derived inside `place_item`** from the two arguments it
already has — `contributes_content` (`:686`) and `member: FlowMember<'_>` (`:687`) — whose only
`RenderedText` arm is `(FlowMember::Text(_), true)`; **neither `place_item` call site changes and no
caller gains a parameter** (`:591` still passes the `PackItem::Text` arm's predicate, `:652` still
passes a literal `true`). ⚠ **The obligation is the derivation, not the call site**: the earlier
wording, "reached only from `place_item`'s call carrying the Text arm's `contributes_content`",
names nothing a grep or the compiler can check, because both callers feed one parameter and one
`note_line_occupancy` call (rev-33 gate; M7's Decision carries the withdrawal). **Checkable as
written**: `grep -rn 'RenderedText' crates/layout/elidex-layout-block/src/` must
return exactly three things — the variant's declaration, that one arm inside `place_item` in
`inline/pack/mod.rs`, and the promote's read in `flush_line` — and in particular **zero hits in
`inline/pack/inline_box.rs`**, the marker path's module, which passes `BoxEdgeOnly` and raises no
higher than the non-text state. A fourth hit is a second raise site and fails the item. Beside it,
`grep -rn 'note_line_occupancy' crates/layout/elidex-layout-block/src/inline/` must still return
the definition plus exactly **two** calls;
(iii) the promote inside the `:210` arm reads that state, not a bool; (iv) cell 24's test at the
promote site, on the two-`font-family` markup that cell names and **without** the `any_font`
harness switch;
(v) ⚠ **both readers of the ordering are order tests, and §6 cell 15d is the one that can fail if
they are not** (M3; round 26, Axes 1/2/3). The soft-wrap guard (`:690`) must hold at-or-above
`Content` and `finish()` (`:787`) above `Empty` — `== Content` at the guard reads **false** on
every line that reached `RenderedText`, and **stops soft
wrapping engine-wide** the moment this PR adds the rung. ⚠ **Not "on every line carrying text"**,
as this item glossed it until rev 34 (round 26, Axis 1, Gate A): a `Text` segment with
`contributes_content == false` derives `Content`, not `RenderedText`, and an atomic-only line is
`Content` too and must wrap — so the false set is narrower than "carries text" and the
must-wrap set is wider. Only `finish()`'s half is insensitive to the widening, `Empty` being the
ordering's bottom; the guard is the checkable one, which is why it is the item. Cell 15d lands in PR-1b, where both
predicates agree; PR-1d is where it discriminates, so **PR-1d re-runs it and it must stay green**.
It is named here rather than left to the guard's own PR because PR-1d's widening is what makes the
equality wrong. **No field joins the `:432-439` reset block for this gate** — M3's reset covers the
widened state, and only M7's tentative joins the block; slot closes.

**This memo ships as the umbrella's approval artefact** — a docs-only **approval PR**, cut from
`origin/main` at TERMINAL and carrying the memo checked out from this branch
(`layout-decorated-inline`, the worktree that authored it, per
[[feedback_plan-memo-author-in-worktree]]); it is reviewed under `/external-converge` (front
matter) and the reviewer's findings are **plan inputs** — #515's first pass reset TERMINAL
(Terminator), so the PR waits as a draft for the renewed TERMINAL; the precedent is #416 (`12ebc052`) and
#470 (`283bcc0d`), one-file docs-only landings of an umbrella memo (`git show --stat`; ⚠ #434 is
*not* one — it landed code beside its memo, `git show --stat deb6eaf6`). It carries the memo and the **program bookkeeping**
rows §10 tags `approval PR` — the rows made true by the umbrella's *approval*, which no code
PR's change makes true. **The two plan-checker tools and the SKILL.md standing note ship in their
own tooling PR** (§9's task — skill infra by §9's own classification, approval-independent, cut
from `origin/main`; **scope = the checkers' wiring, generalisation and two-file awareness as its
memo decides (§9's memo-split booking), and nothing else** — the
`SPEC_LABEL_REVERSE` CSS-label gap is **owned elsewhere**, by the SoT slot
`#11-preflight-css-module-labels` (citation-hygiene Slice B, after its A-ii migrates that dict),
so this program cites the owner and hand-verifies citations meanwhile, as §3 records; ordered
**on its §9 trigger — #510's resolution (landing or closure) or this umbrella's TERMINAL, whichever comes first** —
because #510 puts a generic plan-memo checker and a selftest wire on the same
`trip-wires` registry and the tooling PR's own plan-memo must decide build-on-vs-beside that
substrate under `/elidex-plan-review` (#506 shipped checker tooling without one and paid a tooling-only IMP tail across several external rounds (the SoT's #506
record; no count carried here)); its DoD retires or rewrites the SKILL.md note it ships, which
describes the pre-state it ends), tagged `tooling PR` in §10. It gates nothing else in this
program — not the predicate prereq's plan-review, which hand-verifies its citations as every CSS
plan does today. **How the two PRs relate to this branch**: today `git diff --name-only
origin/main...HEAD` on this branch lists the memo *and* the three tooling files (`SKILL.md`,
`plan-sweep.py`, `plan-xcheck.py`). **Both PRs are cut from `origin/main` and take their files
from this branch** with `git checkout layout-decorated-inline -- <paths>` — the approval PR the
memo alone, the tooling PR the three tooling files (then the generalisation work §9 names) — so
the approval PR's DoD, **its diff against `origin/main` is exactly one file**, holds by
construction and **neither PR waits on the other** (a first draft had the approval PR wait for the
tooling PR to land and merge back — a schedule pressure with no design ground, the shape under
which #506 shipped tooling without a plan-review). This branch is retired after both land (`git
merge origin/main` folds the landed copies back: the two checkers add/add, `SKILL.md`
modify/modify, all resolved to `origin/main`'s versions). Order: approval PR at TERMINAL; tooling
PR on its §9 trigger (#510's resolution — landing or closure — or TERMINAL, whichever first; #510 is
another lane's open PR, so this program does not wait on it past its own terminal); PR-1a
branches off `main` only after the approval PR and the predicate prereq have landed (topology
below).
⚠ **Consequence stated**: nothing in this program waits on the tooling PR except the tooling task
itself; #510's state affects only the tooling PR's substrate decision. The predicate prereq is
ordered against neither (nor was round 21, now complete). If #510 closes unmerged, that resolution fires the trigger
and the tooling PR's memo decides the substrate question against whatever `origin/main` then
carries. No later PR re-ships any of them — the shipper is defined by
the *event* that makes the row true (approval; the tooling task), not by a PR letter.
⚠ Earlier revisions routed all of this to the seam-3 prereq PR, and rev 25's first draft to
PR-1a. #508 shipped only its own plan-memo, on the user-ratified rule (2026-08-16, recorded in
`docs/plans/2026-08-inline-seam3-reconcile-split.md` — whose preamble also says the tooling would
"travel with the umbrella", a binary framing the tooling-PR model supersedes): *a PR carries the bookkeeping its own
change makes true and hands the program's bookkeeping to the program* — and the same rule rejects
PR-1a: its change makes none of these rows true either, and "the first PR after approval" is a
temporal accident, not a truth-maker (this memo does not fix which PR lands first after TERMINAL;
the predicate prereq is approval-independent like its two siblings — an earlier drafting called
it approval-dependent and cited a §9 that says no such thing). The rows whose truth-makers
had **already landed** — the `#11-layoutbox-trip-wire-not-in-ci` correction (#496) and the two
slot registrations (#497's carve and Codex's own slot) — were memory edits with no landing gate
and were done 2026-09-07; §10 marks them. `plan-xcheck.py` check 13 now fails any §10 row still
tagged to a prereq that §8 records as landed without a ✅ discharge, and any row tagged to an
event label §8 does not define, so the mis-routings of *these* shapes are caught by the checker
rather than by a reviewer (mutation-verified, twelve ways, each loud and none on the live
memo: strip #511's row's ✅ → `LANDED`; rename `approval PR` in §10 → `TAG`; demote §8's
`tagged \`tooling PR\`` to a bare mention → `EVENT`; retag a PR-1d row `done (memory)` without
✅/date → `DONE`; reword the seam-3 landing heading ("landed in", no "as") and strip its row's ✅
→ `LANDED`; rename the heading so it names no prereq → `LANDED` (positive control); strike the
row's ✅ through (`~~✅ …~~`) → `LANDED`; write the row `|**…` with no space after the pipe →
`LANDED`; hard-wrap the heading *inside* its bold span and strip the row's ✅ → `LANDED`; drop
the ✅ from the heading while the row keeps its ✅ → `LANDED` (the reverse direction); move a
`done (memory)` row's date away from its ✅ → `DONE`; leave only a backticked `` `✅` `` in the row
→ `LANDED`. A landing record is a ✅ inside a bold span that *opens a line* of §8 — the heading
form — so prose that merely mentions the glyph, like this sentence or a mid-line `**a** ✅ **b**`,
is not one; probed, no false positive).

**Seam-3 prereq PR — ✅ landed as #508 (`7e256029`, 2026-08-23)**: `layout_inline_context_fragmented`'s
reconcile block (§9's measured range) moved to `inline/reconcile.rs`, **byte-identical modulo the
extracted signature** — the criterion as it was set: the block reads a set of enclosing-fn
bindings that become parameters, the **only** permitted edit; the set was enumerated by the PR's
own plan-memo against its actual base (`658cc302`), not here; the proof obligation (a diff of the
moved body against the same block extracted from that base differs only in those bindings) was
discharged by the PR's two harnesses; the crate's suite stayed green with no test touched; the
`#[allow(clippy::too_many_lines)]` re-evaluation and the ledger rows it carried are in
`docs/plans/2026-08-inline-seam3-reconcile-split.md`. ⚠ The extracted signature is the successor
slot's *own* half (§5.3) — and #511 has since narrowed it by one parameter (§5.2).

**Dead-arm prereq PR — ✅ landed as #511 (`22de3078`, 2026-09-07)**: the whole surface the
reachability argument kills, not half of it — `flush_line`'s `else` arm, the `persist_candidate`
branch, and everything downstream of `flow_align` being unconditionally `Some`: the `Option`
wrapper on the field and on `LinePacker::new`'s parameter, the `if let Some(fa)` and `is_some()`
guards, and the comments that explain the two-path model. §5.2's row enumerates the planned
surface and records the delta #511 carried beyond it (the `reconcile_flows` one-bit collapse).
Removing only the arm would have left the same dead code half-alive, which is what CLAUDE.md's
rule forbids; `#11-inline-align-clientrects-nonpersist-path` closed at landing (SoT, 2026-09-07).
Stacked beside the seam-3 split, not folded into it, so that PR's byte-identical criterion stayed
provable.

**Predicate prereq PR**: cssom-view-1 §6 step 1 holds for all four `client*` members against
css-display-3's *inline box* predicate. **This is the obligation face** — §9 argues these; here they
are requirements. ⚠ **PR-1a's DoD inherits four of the seven, not all of them**, and an earlier
drafting wrote the unbounded form ("PR-1a's DoD inherits them because M1 consumes the result").
Requirements **3, 4, 5 and 7** are assertable from PR-1a — cell 6c pins completeness over
`get_intrinsic_size`'s domain (the `input { display: inline }` case), PR-1a's emit test *is* the
call from `elidex-layout-block`, cells 6c/6f are requirement 5's enforcement lever, and cells
6g/6h evaluate the predicate on a pseudo entity. Requirements **1** (all four `client*` members),
**2** (one canonical answer, none beside `inline/collect.rs:14`) and **6** (the home's
lane-eligibility and layer fit) are **not reachable from PR-1a at all**: the first lives in
`elidex-dom-api`, and the other two are properties of the carved PR's own design and process that a
downstream consumer cannot assert. Those three are discharged by the prereq PR itself, under its
own plan-review (round 25, Axis 3):
1. **All four `client*` members**, against the two-input predicate — not a `Display::Inline` guard.
2. **One canonical answer**, not a second beside `inline/collect.rs:14` (and `block/mod.rs:46`).
3. ⚠ **Complete over what `get_intrinsic_size` covers** (`helpers.rs:405`) — an incomplete
   replacedness half silently falsifies §6 cell 6c for `input { display: inline }`, a
   **replaced form control** that computes `Display::Inline`.
4. ⚠ **Callable from `elidex-layout-block`**, because M1's emit test is a consumer. A predicate
   reachable only from `elidex-dom-api` does not discharge this program's dependency.
5. ⚠ **The emit decision must not be keyed on decode outcome.** The engine's existing replacedness
   answer is component presence, and this program is the **first to use it to decide whether a box
   enters the item stream at all** rather than to *size* a box already in it. ⚠ **An earlier revision's ground
   for that was false on three counts, and the corrected picture is worse, not better.**
   `grep -rn get_intrinsic_size crates` returns **nine** lines at `154bac3f`; minus the three that
   are not calls — the definition (`helpers.rs:405`), the re-export (`lib.rs:24`) and the `use`
   import at `crates/layout/elidex-layout-flex/src/fragment.rs:8` — that is **six**
   call sites, not the three named: `block/mod.rs:224`, `positioned/layout.rs:116` and
   `intrinsic/mod.rs:98` (in `elidex-layout`, not `elidex-layout-block`) size a box; but
   `block/children/stack.rs:319` and `elidex-layout-flex/src/fragment.rs:196`/`:353` feed
   `is_monolithic`, i.e. **whether a box may be split across fragmentainers** — a structural
   decision on presence, not a size. And `elidex-layout-multicol/src/fill.rs:418` is **not a
   `get_intrinsic_size` call at all**: it is a direct `ImageData` read feeding the same
   monolithic-ness test, so "degrading to zero intrinsic size" never described it. ⚠ **The
   exclusion list read "minus the definition and the re-export" until rev 34** (round 26, Axis 1,
   Gate B), which subtracts two from nine and leaves **seven**, not the six the sentence then
   states; the six named call sites were right and the arithmetic was short the `use` import.
   So this program
   is **not** the first to key a non-sizing decision on presence — the class is **pre-existing and
   engine-wide** (a failed image is fragmentable today; a decoded one is not), which strengthens the
   case for one canonical answer rather than weakening it. §9 hands the `is_monolithic` consumers to
   the predicate PR as its natural second consumer. ⚠ **Why presence is the wrong answer, with the
   normalisation's reach measured rather than called cheap**: two of the three components
   are **already decode-independent tag projections** — `attribute_reconcile.rs:54` says so verbatim
   ("Presence-gated: `IframeData` exists ⇔ the entity is an `<iframe>`"), and `FormControlState` is
   attached at insertion by a pure tag dispatch (`elidex-form-core/src/lib.rs:819` `from_element`).
   **Only `ImageData` conflates two facts** — "this element is replaced" and "its pixels are ready".
   ⚠ **But normalising it is not a rename, and an earlier revision called the answer "cheap" on the
   sibling comparison alone.** `ImageData` is also the **size** carrier this requirement's own
   consumers read — `get_intrinsic_size` returns `Size::new(img.width as f32, img.height as f32)`
   from it (`crates/layout/elidex-layout-block/src/helpers.rs:405-410` at `22de3078`) — so splitting
   *replaced* from *has pixels* reaches the component in `elidex-ecs`, **both** of its producers
   (`decode_image`, `sync_dirty_canvases`, below) and every consumer enumerated above, and it must
   answer what an intrinsic size is before decode. The two siblings are no counter-model: they carry
   a size too (`:412-417`, `:419-424`), but from attribute and tag data, which is why presence and
   size may travel together there and may not here. **So "no escape hatch" rests where it already
   rested — CLAUDE.md's *ideal over pragmatic* — and not on a cost claim** (round 25, Axis 1).
   Handed over with it:
   `helpers.rs:399-401`'s doc comment, which *documents* presence as the replacedness answer
   ("Returns `Some(Size)` for replaced elements …, `None` otherwise") and is the root of the
   confusion. Three reachable cases where presence is absent yet the element is replaced:
   a **failed or 404 image** (the insert is gated on fetch *and* decode success, so it is permanent,
   not transient); a **JS-created `<img>`**, since the extraction runs only at document load; and an
   **undrawn `<canvas>`**, whose `ImageData` comes from a *second* production producer,
   `elidex_api_canvas::sync_dirty_canvases` (`crates/api/elidex-api-canvas/src/component.rs:132-148`).
   ⚠ An earlier revision called `decode_image` the "only producer" of `ImageData`; it is not.
   In each case M1 would classify a replaced element as an inline box and emit a marker — the exact
   defect §5.1 M1 exists to prevent. The PR supplies an answer independent of decode outcome. ⚠ **There is
   no "or justify" branch**, and an earlier revision offered one: that route ships the defect §5.1 M1
   exists to prevent, and CLAUDE.md's *ideal over pragmatic* declines it. ⚠ **The enforcement lever
   is §6 cells 6c/6f, not this sentence** — prose here cannot fail, and a fixture choice would
   otherwise defeat the cell (see 6c's ⚠). ⚠ **But those cells are PR-1a's, which puts the only
   assertion of this prereq's own requirement one PR downstream of its landing**: the prereq could
   land keyed on decode outcome and nothing would go red until PR-1a. It can carry its own, and
   must — the predicate answers *replaced* for an `<img>` entity carrying no `ImageData` (a failed
   decode, a JS-created `<img>`) and for an undrawn `<canvas>`, which is a unit assertion in
   whichever crate the home turns out to be and needs no layout at all. Cells 6c/6f then stay as the
   **consumer-side** lever — the second assertion, not the first (round 25, Axis 3).
   ⚠ The three cases are **illustrations of a permanent class, not its extent**: `<video>`,
   `<object>`, `<embed>` and `<svg>` receive **none** of the three components under any condition
   (`FormControlState` attaches on form tags, `IframeData` on `iframe`; nothing attaches for these),
   so presence classifies them non-replaced **always**, not in a corner. M1's own grounds name
   `video` and `svg` in the set it must exclude.
6. ⚠ **Its own lane-eligibility check.** The candidate home for the replacedness half is
   `elidex-form-core`, which is a **live L3-lane target** (`is_submittable` at `src/lib.rs:200` is
   the citation-hygiene program's Slice E subject). §9 declines
   `#11-css2-spec-label-normalisation` on exactly this ground — "parallel-safety is what
   discriminates" — and an earlier revision never applied that rule to the PR the whole program
   blocks on. The verdict travels with the home decision; the check does not.
   ⚠ **And the home must be justified on *layer fit* as well as parallel-safety**, which this
   requirement attached to `elidex-form-core` and this clause does not: the composed predicate is
   css-display-3 §A's ***inline box***, consumed by `elidex-layout-block` (M1's emit test) and
   `elidex-dom-api` (the four `client*` members), so siting it in the form-controls crate would make
   that crate the owner of an **engine-wide display classification** — a wider claim than
   "`FormControlState` lives there". §9's own survey already names the alternative and its
   asymmetry: the *display* half has a home with `pub` spec-cited `Display` categorisation in
   `elidex-plugin` (`computed_style/` — `display.rs`'s `impl Display` and `columns.rs`'s
   `is_multicol`; ⚠ **not** `display.rs`'s `is_scroll_container` / `clips`, which are `impl
   Overflow`, §9) that **both** consumers already depend on with zero
   new edges, and only the *replacedness* half needs a component-bearing crate. The carved PR must
   answer both questions for whichever split it picks — parallel-safety **and** which layer the
   composed answer belongs to — not the first alone (round 25, Axis 1).
7. ⚠ **Defined on every entity that carries a `ComputedStyle`, not on elements only.** A
   `::before`/`::after` pseudo entity is `dom.create_text(String::new())`
   (`elidex-style/src/pseudo.rs:63` at `22de3078`) — `NodeKind::Text` + `TextContent` +
   `TreeRelation` (`elidex-ecs/src/dom/mod.rs:548-563`), then `ComputedStyle` +
   `PseudoElementMarker` inserted (`pseudo.rs:64-65`) — with **no `TagType`, no `Attributes`**, and
   M1 evaluates the predicate on it (§6 cells 6g/6h). Its replacedness is decided from the
   `content` model — `ContentItem` (`elidex-plugin/src/computed_style/box_model.rs:35-56`) has
   `String`/`Attr`/`Counter`/`Counters` and no `url()` variant, so generated content is
   non-replaced today, and the predicate must say so **from that fact, not from tag presence**.
   Three things about that read, all the prereq PR's to keep: (a) **the spec rule the predicate
   tracks** is css-content-3 §1 *Inserting and Replacing Content: the `content` property*: a
   `<content-replacement>` (a single `<image>`) "Makes the element or pseudo-element a replaced
   element", whereas an `<image>` inside a `<content-list>` "is an inline anonymous replaced
   element" — a replaced **child** — and the pseudo itself stays an inline box (`webref heading
   css-content-3 1`; `body css-content-3 content-property`). ⚠ **But css-content-3 §1's own issue note, on the
   one class this predicate actually evaluates**: "This value has historically been treated as
   `<content-list>` on ::before and ::after. Presumably there's a Web-compat requirement on this,
   so these pseudo-elements might need an exception. [Issue #2889]" — and the pseudo pipeline is
   the only **box-generating** reader of `ComputedStyle.content` today. Measured at `22de3078`:
   one resolver writes `content` for elements and pseudos alike (`elidex-style`'s `resolve_content`,
   `resolve/box_model/mod.rs:391`, called from `build_computed_style` at `resolve/mod.rs:133`,
   which `pseudo.rs:43` also runs; `elidex-css-box`'s `BoxHandler::resolve` `content` arm at
   `lib.rs:303` has no production caller — ⚠ **that is about this arm, not about the trait**:
   `CssPropertyHandler::resolve` has exactly one production caller,
   `elidex-style/src/resolve/mod.rs:305`, on a concrete `TransformHandler`); the readers are the
   pseudo pipeline (`pseudo.rs:47`,
   `generated_content.rs:178` under its marker gate) and the CSSOM readback at
   `elidex-css-box/src/lib.rs:701` — no box-generation path reads an element's `content`, and page
   margin boxes consume `content` on a separate path
   (`crates/core/elidex-render/src/builder/mod.rs:549`, off `MarginBoxContent`, a different struct)
   outside the predicate's domain. ⚠ An earlier drafting said "the pseudo is elidex's only
   `content` consumer" and called the css-box arm "element-side" — a universal with a non-empty
   complement and a mislabelled dead path (round 24, Axes 1/2/4). So for a pseudo the operative
   reading is `<content-list>`: an `<image>` — bare or in
   a list — is an anonymous replaced **child**, and the pseudo stays an inline box; the
   `<content-replacement>` flip is an **element-side** condition, reached only when elements
   consume `content`, and the predicate must not flip a pseudo to replaced on css-content-3 §1's un-excepted
   text. The rule reaches elements too, so the read is not pseudo-only; the pseudo/element
   distinction enters only that `<content-replacement>` reading. (b) **Which crate owns the
   read**: `ContentValue`/`ContentItem` live in `elidex-plugin`'s `computed_style`
   (`box_model.rs:60`/`:35`, re-exported at `computed_style/mod.rs:89`) beside `Display`
   (`computed_style/display.rs:7`, re-exported at `computed_style/mod.rs:91`), so this is a
   `ComputedStyle` read from `elidex-plugin` — the read the predicate already makes for `display`;
   whichever half of the split the carved PR assigns it to (§9 leaves the split open), it adds no
   crate edge (`elidex-form-core`, the replacedness half's candidate home, already depends on
   `elidex-plugin`, `crates/dom/elidex-form-core/Cargo.toml:14` at `22de3078`).
   (c) **Totality**: the replacedness answer is derived by a match **total over `ContentItem`**, so
   a new variant fails to compile at the classification site rather than silently staying
   non-replaced. "Today" is a fact about the model, not a deferral: `content: url()` never reaches
   the model — `parse_content` (`elidex-css/src/declaration/misc.rs:441` at `22de3078`) accepts
   `none`/`normal`/strings/`attr()`/`counter()`/`counters()` and rejects every other token
   (`_ => Err(())`, `:516`), so the declaration is dropped before resolve; `CssValue::Url`
   (`elidex-plugin/src/values.rs:71`) exists but is produced by the background parsers and the
   presentational `background=` mapping (`elidex-dom-compat/src/presentational.rs:130`), never by
   `parse_content`, and were
   one to arrive at `content`, `resolve/box_model/mod.rs:444` `_ => ContentValue::Normal` (bare)
   and `:435` `_ => None` (inside a list) would still yield no image item — such a pseudo
   generates no entity. A
   tag-gated predicate (natural for the four `client*` Element members) would put the production
   pseudo silently outside the class while an element-kind fixture passes — requirement 5's
   fixture-defeat shape; cell 6g's fixture therefore builds the production node kind. The tag
   read is a **live idiom in the file the `client*` members live in**: `layout_query.rs:373` at
   `22de3078` reads `get::<&elidex_ecs::TagType>` (`offsetParent`'s body/html fallback) — which
   is why the requirement is written rather than assumed. ⚠ An earlier drafting claimed no such
   site existed on the strength of a `<&TagType>` grep that misses the qualified path
   (`git grep -n TagType 22de3078 -- crates/dom/elidex-dom-api/src/element/layout_query.rs` is
   the command that finds it; gate on rev 30).
The **mechanism** — the home, whether `FormControlState` needs a crate edge or a move, the split
between the `elidex-plugin` display half and the component half, and the disposition of
`client_top_returns_border_width` (`element/layout_query.rs:497`) — stays that PR's plan-memo's (§9).
It is carved because those are not this memo's questions; the seven above are, because this program
depends on them.

**Ordering — done as fixed: seam-3 landed first (#508, 2026-08-23), the dead-arm PR second
(#511, 2026-09-07, cut from `origin/main` after #508). The ground was asymmetric cost, not
coupling.** The two were not coupled *as planned*: the dead arm is in `pack/mod.rs`, and its `inline/mod.rs` half (`:239`,
`:240-251`, `:322` and their comments, all ≤ `:330`) lies entirely **above** seam 3's `:413-639`
and touches nothing inside it (`awk 'NR>=413 && NR<=639' inline/mod.rs | grep -c
"persist_candidate\|flow_align"` → 0). A *coordinate* shift is no ground for an order, since the
front matter already requires every PR to re-anchor against its actual base. The ground that does
survive is that the two directions cost differently: **seam-3-first costs the dead-arm PR nothing**
(deleting `:413-639` renumbers nothing *above* it, and every dead-arm `inline/mod.rs` site is
≤ `:330`; its `pack/mod.rs` half is in a file seam 3 never touches), while **dead-arm-first forces the seam-3 PR to re-measure the one criterion in this
program that *is* a byte range** — "byte-identical modulo the extracted signature", already
approved at a measured range. Free in one direction, not the other.
⚠ **Measured after the fact**: the *landed* dead-arm surface did reach into seam 3 — the
gate-driven one-bit collapse edited `inline/reconcile.rs` (§5.2) — so the independence premise
above held for the planned surface only. The order being seam-3-first cost the *byte-identity*
criterion nothing — which is all the asymmetric-cost ground ever claimed; whether the landed
delta disturbs a later obligation is measured, not inferred (`grep -n 'do_carrier\|eleven' <memo>`
returns landing-record sites only — §5.2's row and this paragraph, two lines — and no DoD), and in full it is
round 20's question ([[feedback_plan-ratified-surface-is-a-design-change]]). One consequence is
already known: the successor slot's disjunct 3 fired (§5.2, §10).
⚠ **The predicate prereq is ordered against neither of them, but is ordered against PR-1a.** Its
constraint is **in `main` before PR-1a**, because M1's emit test consumes the predicate (§5.1 M1,
§6 cell 6c). ⚠ This memo does **not** claim it is disjoint from the other two: its touch set follows
from its own plan-review's choice of home (§9), so disjointness is a question it answers, not a
premise this memo may use. What the umbrella owns is the ordering.
All three branch from `main`;
whichever lands second takes `git merge origin/main` (⚠ **not** `rebase` — an opened PR branch
cannot be rebased without a force-push, which `~/.claude/hooks/` denies; #511 needed no merge —
it was cut from `origin/main` after #508 landed). **Branch topology**: the
**three** prereqs branch off `main`, as does the **tooling PR** (approval-independent, ordered on
its §9 trigger — #510's resolution or TERMINAL — and against nothing else here); the **approval PR**
is cut from `origin/main` at TERMINAL carrying the memo alone (§8 above); PR-1a branches off `main` after **all three** prereqs
*and the approval PR* have *landed* — the two `elidex-layout-block` ones (**both landed**) because they move code PR-1a edits, and
the predicate PR (**pending**) because PR-1a's M1 consumes what it establishes. ⚠ Whether the
predicate prereq is ordered against the other two depended on its touch set, which §9 hands over;
the two `elidex-layout-block` prereqs having landed, that question is now moot for ordering and
survives only as the predicate PR's own re-anchoring against `22de3078`. PR-1b,
PR-1c and PR-1d each depend on their predecessor's mechanism, and because CLAUDE.md mandates **squash** merge
a stacked branch's base commits are rewritten when its parent lands — which cannot be repaired on
an *opened* PR without the force-push the hooks deny. So they are **not opened as a stack**: each
is cut from `origin/main` only after its parent has landed. "PR-1b depends on PR-1a" is an ordering
of landings, not a git parent relation.

**Explicitly not covered, recorded rather than dropped**: css-inline-3 §5.3's "or if it contains
only glyphs from fallback fonts" strut condition — elidex has no fallback-provenance signal.
**It gets its own slot, `#11-inline-fallback-font-strut`** (pre-existing class; Why / trigger /
re-eval in §5.3, registered by §10's `approval PR` row), and §3's css-inline-3 §5.3 glyphless row
is `✗ (pre-existing, disclosed)` on that half accordingly.
⚠ **An earlier revision folded it into `#11-inline-root-inline-box`** on the ground that that
slot's §5.3 trigger named font-fallback provenance — but the trigger named it *only so the fold
could surface*, which makes the ground circular, and the fold is a mechanism mismatch of exactly
the shape M7's Grounds refuses for its own residual: that slot's **subject** is the block
container's *root inline box*, the line box's height floor, which §1.1 goes out of its way to
distinguish from the *strut* of a glyphless inline box — they share the word "font" and nothing
else. Implementing the root inline box would discharge that slot and leave the fallback condition
undone with nothing preventing the close. The disjunct is struck from that slot's trigger
(§5.3) in the same revision, so the two sites cannot drift apart (round 25, Axis 3).

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires, and this program honours it.** The
  slot's trigger is "the next change that touches `layout_inline_context_fragmented`'s body";
  PR-1a touches `:154`/`:161`/`:192-199` and PR-1d `:200` and the persist block, the slot memo's
  **seam 3**. **Disposition**: a standalone prereq split PR at seam 3, before PR-1a —
  **✅ done, #508 (`7e256029`)**; the partial close and the successor slot are in the SoT.
  **Grounds**: the slot's **own** trigger, whose *touch* disjunct has fired. That ground alone
  carries it, and it is the only one stated. ⚠ The slot's trigger has a **second disjunct** —
  "`mod.rs` growing back toward 1000" — which has **not** fired and which §10's successor slot must
  therefore carry forward, since this program grows the file. ⚠ CLAUDE.md's prereq-split clause is
  scoped to **">1000行 file を触る際"** and `inline/mod.rs` is under that gate
  (`wc -l crates/layout/elidex-layout-block/src/inline/mod.rs`), so the clause does not reach
  here; neither does #500's precedent, whose files were both over it. The 700–800
  cut-while-writing band does not reach either — that lesson is about files being newly *authored*,
  and its own text routes pre-existing files to the >1000 clause. ⚠ The slot memo
  (`project_inline-fragmented-fn-decomposition.md`) asserts the opposite **for this exact file**
  ("783 sits at the top of the 700–800 'cut while writing' band, which is the argument for not
  letting this drift"). ⚠ That quotation is accurate as a quotation and **stale as a fact**:
  `wc -l crates/layout/elidex-layout-block/src/inline/mod.rs` was 785 on `154bac3f` and is **570** on `22de3078` (#508 took the reconcile block out, #511 the dead arm; re-run `wc -l`, do not carry this figure), so its number is stale **and so is its band argument — the file now sits below the 700–800 band**; an earlier revision wrote "its band argument is
  unchanged but its number is not — quote it, do not restate it. That claim is not adopted here, and the disagreement is recorded rather
  than resolved silently: it is the counterweight to narrowing the prereq PR to seam 3 alone, so if
  it is right the residue is under-cut. The disposition stands on the fired touch disjunct either
  way; what changes is whether seams 1 and 2 should also be discharged now, and §10's successor
  slot is where that is booked.
  **Scope**: the prereq PR discharges **seam 3 only** — `mod.rs:413-639` on `154bac3f` (the slot
  memo's `:411-637` predates #497's two-line shift). Seams 1 (`:266-303`) and 2 (`:388-411`),
  both re-measured on `154bac3f`, remain, so §10 records a **partial** close. PR-1a's own touch
  sites lie in the residue, not in seam 3.
  **Cold gate** ([[feedback_split-on-touch-prereq-workflow]]): re-run `gh pr list --state open` and
  check each **against that PR's own touch set**, not a fixed crate — **three** sets, not two: the
  seam-3 and dead-arm prereqs are `elidex-layout-block` only; PR-1a–1d are layout plus
  `elidex-render`/`elidex-dom-api` **tests**; and the predicate prereq's set is **not determined
  here** — it follows from its plan-review's choice of predicate home (§9), so this memo runs the
  gate at its *known lower bound* (`elidex-dom-api` production, which it certainly edits) and books
  the re-run to that PR. Local worktree branches count too, since
  `gh pr list` cannot see them. ⚠ **Run it at the width the touch set actually has**, which an
  earlier revision did not: it widened the feature PRs' set to `elidex-render`/`elidex-dom-api` and
  then still checked branches against `crates/layout/elidex-layout-block/` alone. Re-run on
  `658cc302` at all three widths:
  * **Open PRs**: none touches `crates/layout/elidex-layout-block/`. This branch's own tooling
    files ship in the tooling PR (§8) and PR #501 is live in `.claude/`, but the two touch
    **disjoint files** (`comm -12` on the two `--name-only` sets → empty), so there is no
    collision. ⚠ An earlier revision also claimed a *semantic* overlap — that #501 relocates
    `SPEC_LABEL_REVERSE` out of `preflight.py`, the site §3 and §9 book work against. **Measured
    and withdrawn**: `git grep -c SPEC_LABEL_REVERSE origin/webref-cite-audit-tool --
    .claude/skills/elidex-plan-review/preflight.py` → 7, and that branch's whole diff to the file
    is three comment lines. #501 moves the *forward* map inside `.claude/tools/webref`; the
    booking's target does not move — ⚠ and since round 20 nothing here books work against it
    at all (the CSS-label gap is `#11-preflight-css-module-labels`'s, §9 tooling bullet).
  * **Tooling PR width** (measured 2026-09-07, round 20 — the width the touch set actually has,
    which the bullet above did not measure): `.claude/tools/plan-xcheck.py`, `plan-sweep.py`,
    `.claude/skills/elidex-plan-review/SKILL.md` and, under the `trip-wires` option,
    `scripts/trip-wires.sh` — where open PR #510 edits `REQUIRED_WIRES` in the same hunk
    (`git diff origin/main...origin/vm-p4-plan-memo-checker -- scripts/trip-wires.sh`) and the
    driver fails an unregistered wire in both directions. `preflight.py` is **not** in the set,
    which is also what removes the #501 overlap on that file (`gh pr view 501 --json files`; #514
    touches `docs/plans` only). Order: on the task's trigger (§9 — #510's resolution or TERMINAL).
  * **Unmerged local branches**: thirteen touch the widened set (the count is the output of the
    loop in this bullet, not a figure carried in prose). None reaches an edit site this program
    holds. ⚠ An earlier revision recorded `layout-text-height-split` as a "genuine overlap"
    restructuring the `text_height/*` module §6's non-regression line cites. **Measured and
    withdrawn**: `git diff --stat origin/main...layout-text-height-split` and
    `git show --stat 4357cd4c` are identical (11 files, 1393+/1345−) — it is #500's unsquashed
    history, already in `main`, exactly the disposition the next paragraph gives
    `origin/layout-css2-cite-sweep`; and `git show origin/main:…/text_height/basic.rs | sed -n
    '204p;233p'` returns the two named tests, so §6's coordinates are already anchored post-split
    and PR-1a owes no re-anchoring.
  * **The third set's known lower bound — `elidex-dom-api` production**, which the carve added and
    which no earlier run covered. ⚠ This is a measurement at *one* width the predicate PR certainly
    has, **not** at its set: if its plan-review puts the predicate in a shared crate the set is
    wider, and the PR re-runs the gate then. At this width the edit site is
    `crates/dom/elidex-dom-api/src/element/layout_query.rs`, and **nothing reaches it**. Measured on
    `658cc302`: `gh pr diff <n> --name-only | grep -c crates/dom/elidex-dom-api/` is **0** for every
    open PR — ⚠ **a dated measurement, not a present-tense property**, because the open set turns
    over between rounds: **2026-09-07** the set was 506, 505, 503, 502, 501, 381, and
    **2026-09-08** `gh pr list --state open` returns **515, 514, 513, 510, 506, 502, 501, 381**,
    on which the same command is still **0** for every member. The check is the command re-run at
    the round's own date; this enumeration records which set was checked (round 25, Axis 3). And looping
    `git diff --name-only origin/main...<ref>` over every local and `origin/` branch returns **no**
    hit on `element/layout_query.rs`. ⚠ Two stale branches (`feat/m4-1.5-2-plugin-arch-anim`,
    `feat/tags-t2d-interactive`) do touch `elidex-dom-api/src/registry.rs`, which registers
    `clientTop.get` / `clientLeft.get` (`registry.rs:168-169`) — recorded because it is the nearest
    miss, and not a collision: the guard lands in the handler bodies, and whether the PR needs
    `registry.rs` at all is its own memo's question. The general claim in the bullet above ("None
    reaches an edit site this program holds") therefore **held at every width this memo measured,
    on both dates above** — the 2026-09-08 re-run gives **0** for
    `crates/layout/elidex-layout-block/`, `crates/core/elidex-render/` and
    `crates/dom/elidex-dom-api/` across all eight open PRs — which is the thing a widened set most
    often falsifies, and the reason the residual
    width is booked rather than assumed clean. ⚠ Past tense deliberately: the gate is re-run per PR
    ([[feedback_split-on-touch-prereq-workflow]]), so this records what was measured and when, not
    a standing property (round 25, Axis 3).
  ⚠ A round-16 finding held that the widening would surface `domform-submittable-category`.
  **Measured and refuted**: `git diff --name-only origin/main...domform-submittable-category |
  grep -E "crates/layout/|elidex-render|elidex-dom-api"` returns nothing, so it collides with
  neither set. Recorded rather than dropped, because a finding accepted without re-derivation is the
  failure this memo's front matter exists to prevent — and the same standard is why the *breadth*
  defect above was not excused by that refutation being correct.
  Note `origin/layout-css2-cite-sweep` is a live remote branch that does touch the file, but it is
  #497's unsquashed history, already in `main`.
  **Its own gate**: `/elidex-plan-review`, like every PR here — CLAUDE.md makes that a rule, not a
  judgment, and §10 routes non-mechanical ledger actions to this PR.
* ⚠ **This memo's own length, and why the split is booked rather than done.** It is far past 1000
  lines and has grown in every revision since; **no figure is written here** — `wc -l <memo>` is the
  measurement, which is the rule the adjacent **File growth** bullet already states for source
  files, and an earlier drafting instead wrote "past 1000 lines and grew again in rev 20 and rev 21",
  a figure and a window both stale within two revisions (round 25, Axis 3).
  CLAUDE.md's touch-time discipline does not name
  `docs/**`, but its rationale ("cohesion 判断", and the cost paid by every reader) reaches a
  document five review agents re-read in full each round, and the sibling L3 lane already has the
  precedent (an analysis note plus an executing umbrella). The seam exists — §1 (the rules from the
  module) and §4 (verified current state) are grounds, not decisions, and the review history was
  already exported to the slot memo. **What blocks doing it here is this program's own gates**, and
  that is measured, not assumed: `preflight.py`'s hard gate aborts on a missing **§3**, so the
  coverage map cannot leave the umbrella; `plan-xcheck.py` harvests §6's cell routing structurally
  and cross-checks §10's ledger against slot definitions that live in **§9**, so neither can leave
  either; and checks 11b/11c scan **the whole file** for cited paths and cell references, so moving
  §1/§4 out would silently shrink the checked surface — a split that weakens the checker is the
  wrong trade for a program whose last six rounds were saved by it. **Booked to the plan-checker
  tooling task (below), on that task's trigger** (#510's resolution or TERMINAL): making the checkers
  two-file aware is now in that task's scope statement (an earlier revision booked it here without
  ever adding it to the task's scope — a booking to a trigger nothing reached; rev 26 first tried
  to *decline* the split on that same ground, which its own tooling bullet refuted by naming a
  reachable event). The tooling memo decides two-file awareness **on its merits**, under its own
  plan-review and whichever substrate it builds on; the split follows two-file awareness — that
  memo is the decision's home, not this bullet (a first draft pre-scripted "beside ⇒ decline",
  a scope-cut authorised before the deciding memo existed). Until then the umbrella carries its length
  knowingly; the reader cost that remains after TERMINAL is the per-PR plan-reviews
  (five crate PRs and the tooling PR), which read §8/§10 — a surface a split would not shrink.
* **File growth**: measure with `wc -l` at each PR rather than against a number written here, which
  this program's own prereq PRs invalidate. Per §5.2 the **open-box stack** goes to a new
  `pack/inline_box.rs`; M3's line-state core and M7's promotion stay in `pack/mod.rs`; new tests to
  a new module (§6). `inline/mod.rs`'s growth across the four shipping PRs is a signature change, two gate
  predicates and the persist-block touch — no new seam decision needed. `pack/inline_box.rs` is the one **non-test** source file this program authors (the seam-3 module
  is a move, and §6 already applies the band to the new test modules), so
  [[feedback_touch-time-split-means-while-writing]]'s 700–800 band applies to it while it is being
  written, not afterwards — it accumulates M5's predicate and M3's sums in **PR-1b**, then M4's
  stack and the flush hook in **PR-1c**, so the band must be checked at both and not only at the
  second. `inline/mod.rs` and `pack/mod.rs` are re-checked against the ~1000-line convention
  **at every PR**, on their
  then-current size — `pack/mod.rs` takes its largest add in PR-1b (`note_line_occupancy`, the
  `flush_line` hook call, the reset-block growth), so a first check at PR-1d is the shape
  [[feedback_touch-time-split-means-while-writing]] calls already-too-late.
* **`#11-inline-relayout-box-staleness`** (`inline/pack/boxes.rs:96`) — pre-existing, and the
  statement PR-1c owes it is a **write-path** one, not a cosmetic widening. M4 adds a *derived*
  field with **two** SoT inputs — the entity's `ComputedStyle` padding/border/margin **and**, for
  percentage edges (cell 4), the containing block's `containing_inline_size`. Every mutation path
  to the first (`element.style.*`, a `style`/`class` attribute change, a stylesheet insertion)
  triggers restyle + relayout; the second moves on a viewport resize or any ancestor width change
  **with no `ComputedStyle` mutation at all**. In both cases, on relayout `assign_inline_layout_boxes` `continue`s for any entity that
  already carries a `LayoutBox` (`boxes.rs:62-64`) while nothing anywhere removes one. So the new
  write has **no reconciliation hook**. The staleness is **total and pre-existing** — the same skip
  already freezes `LayoutBox.content`, so every inline geometry assertion in the engine is already
  first-layout-scoped — which is why PR-1c does not own the refresh and §8 instead scopes its cells
  explicitly. What M4 changes is *which fields* go stale, not *whether* they do. The fix belongs
  where the skip is: a `layout_generation` comparison (the component already carries one,
  `boxes.rs:86`) instead of a presence test, which is this slot's work.
  (`#11-inline-align-clientrects-nonpersist-path` was ledger-marked to fold into terminal-Z
  C-3/C-4 alongside it; the dead-arm prereq #511 **closed** it instead — SoT corrected at landing,
  2026-09-07: the fold note now applies to `#11-inline-relayout-box-staleness` alone.)
* **`#11-layoutbox-absence-unreachable`** (#488, registered in the SoT) — no longer relevant, and
  §10 carries the disposition row: M5 keeps no entity's box withheld, so no truthful box-absent
  signal is required.
* **Intrinsic sizing** — split by pass, per M8: the max-content contribution is **in PR-1b**; the
  min-content one is slotted (`#11-inline-min-content-box-edges`) because that pass has no
  accumulator to attach edges to. The intrinsic passes pass `0.0` as the containing inline size
  (§5.2) — the standard treatment for percentages.
* **`#11-css2-spec-label-normalisation`** — this memo cites css-inline-3 for the model, so its
  remaining CSS 2 cites are **§8.3, §8.3.1, §9.2.1.1, §9.2.2.1, §9.4.2, §9.4.3, §10.8.1 and
  §16.6.1** — eight. ⚠ **Seven until rev 34, and the eighth was invisible to the very command
  that defines the list** (round 26, Axis 4): `§8.3.1` entered with rev 33's §1.2 edit written
  **bare**, and the grep below requires the `CSS 2 ` prefix, so the module-qualification sweep of
  this revision is what made it countable. The two figures are ordered: qualify first, then re-run.
  The front matter's **seven** counts something else — the section↔title *pairs* — and excludes
  CSS 2 §16.6.1, which §1.6 cites with no title.
  ⚠ Three corrections to how this list is produced. **CSS 2 §9.2.1.1** (*Anonymous block boxes*)
  entered with §6 cell 6e and an earlier revision's list omitted it — ⚠ and it is written with its
  module here because a bare `§9.2.1.1` sat one clause away from the memo-internal `§6` in the same
  sentence, against the front matter's "a bare `§N` is always this memo's own section" (round 25,
  Axis 4; the module spelling adds no new key to the grep below, which already returns this
  section). And the command must be written so it does
  **not match its own literal**: `grep -o 'CSS 2 §[0-9.]\+' <memo> | sort -u` — with `*` the bare
  `CSS 2 §` in this very sentence is itself a hit, so the figure was unstable under re-running as
  written ([[feedback_prose-rules-cannot-fix-unexecuted-claims]]); each section↔title pair verified with
  `webref heading CSS2 <n>` (§9.2.2.1 = *Anonymous inline boxes*, and it appears only where the
  memo names a **code** site's mis-cite, which is why an earlier revision's list omitted it — the
  list is the command's output, not a curated subset). Adjacent `CSS 2.1 §`
  lines in touched files are left alone. ⚠ **The slot's trigger — "a lane already touching ≥1 of the
  9 crates" — has fired**, since this program touches `elidex-layout-block` and `elidex-plugin`, and
  it is declined on the record: the slot's own memo requires "one commit across all 9 crates" and
  declines parallelism because that "collides with the L1 CSS lane, which has per-family increment
  slots open against `elidex-css`/`elidex-style`" — a collision that is live. The commit-shape
  ground no longer discriminates; parallel-safety is what does.
* **css-inline-3 §5.3's Quirks-Mode rule** — "any inline box fragment that has zero borders and
  padding and that does not directly contain text or preserved white space is ignored when sizing
  the line box" — is the quirks analogue of clause 3 and is unimplemented. elidex has no
  quirks-mode layout switch at all, so this is not a gap this umbrella can carve; recorded because
  the §3 rows citing css-inline-3 §5.3 do not cover it. **Disposition**: **no slot** — a slot would
  track a divergence from a rule the engine has no mode to apply, so its trigger could only be "a
  quirks-mode layout switch exists", which is the missing feature itself; recorded here, and §3's
  own Quirks-Mode row carries `✗ (deliberate, §9)` pointing at this bullet. When a quirks mode is
  built, this rule is one of its cells. (⚠ Added in rev 33 — the bullet stated its facts and stopped,
  while both siblings around it carry a **Disposition** line; round 25, Axis 3.)
* **`<br>` and `<wbr>` carry no break behaviour engine-wide.** Both are inline boxes under the
  css-display-3 predicate M1 consumes (non-replaced, outer `inline`, inner flow), so a *decorated*
  one takes a marker like any other inline box. What is missing is each tag's own effect:
  `grep -rnE '"br"|BrMarker' crates/` returns **21** lines at `154bac3f` and
  `grep -rniE '\bwbr\b' crates/` **7**, **27** distinct `file:line` between them (one line,
  `elidex-html-parser-strict/src/tree_builder/modes/in_body.rs:166`, matches both), and **every
  one of them is parsing or DOM**: `elidex-html-parser-strict` 23 (tokenizer and tree-builder
  arms plus html5lib fixtures), `elidex-dom-api` 3 (the `VOID_ELEMENTS` list at
  `element/tree.rs:804-805` and a test), `elidex-js` 1 (`HTMLBRElement` prototype selection,
  `vm/host/elements.rs:286`). **Zero** in `crates/layout`, `crates/core/elidex-render` or
  `crates/core/elidex-ecs`, and `force_break()`'s single caller (`pack/mod.rs:603`) is the
  preserved-`\n` path, not a `<br>`. ⚠ **The scope was those three directories until rev 34, and
  the universal it supports is engine-wide** (round 26, Axis 1, Gate B): a hand-written directory
  list cannot discover a directory, which is the rule this revision installs twice elsewhere —
  §8's PR-1c edge audit and §9's canonical-predicate survey — and broke here in the same edit that
  rewrote this bullet. The complement is measured above rather than asserted absent, and the
  **conclusion survives**: nothing outside the old scope is break behaviour.
  **What is unimplemented is the *break*, not the element**, and
  the program therefore does reach a decorated one. Measured: neither `br` nor `wbr` is selected by
  any UA rule (`git show 22de3078:crates/css/elidex-style/src/ua.rs | grep -niE '(^|[^a-z])br([^a-z]|$)|wbr'`
  → nothing), so `<br style="padding:10px">` computes `display: inline`; it carries a
  `ComputedStyle`, so `collect_inline_items_inner`'s style guard (`collect.rs:217`) does not route
  it to the text arm; and it passes all four filters — `Display::None` (`:218`),
  `is_absolutely_positioned` (`:224`), `is_atomic_inline` (`:243`) and `PseudoElementMarker`
  (`:260`) — reaching the inline-element recursion at `:291`. So **M1 emits a marker for it**,
  PR-1b advances the line by its edges, PR-1c gives it a `LayoutBox`, and PR-1d flips
  `<p><br style="padding:10px"></p>` from `line_count: 0` (Shape B — `<br>` has no children, so the
  recursion emits nothing) to a committed line. Each step is spec-correct **in isolation** — per
  css-display-3 a `<br>` *is* an inline box, which is what §5.1 M1's grounds already say ("a marker
  for a decorated one is correct") — while the **forced break** stays unimplemented, so what the
  program delivers for a decorated `<br>` is its box, never its break.
  **Why out of scope**: each is a distinct inline-level feature —
  a forced-line-break element and a soft-wrap-opportunity element — whose work is in the UA
  stylesheet and the packer's break machinery, not in inline decoration. **Disposition**: no slot — a missing feature, not a divergence
  a slot would track; recorded so that §5.1 M1's attribution names something. ⚠ **The ground is
  restated to its sibling's shape, because the thinner one it carried until rev 34 did not rule
  out a writable trigger** (round 26, Axis 3, low confidence, and the judgement taken is the
  sibling match rather than opening a slot): "a missing feature, not a divergence" alone leaves
  "any work implementing forced-break elements" available as a trigger, and a slot with a writable
  trigger is a slot. The sibling above — the quirks-mode bullet — declines on the sharper form,
  that the only trigger a slot could carry is *the missing feature itself*, which makes the slot
  circular; and that is exactly the case here, since the work that would discharge a `<br>` /
  `<wbr>` slot **is** implementing `<br>` and `<wbr>`. ⚠ It is also **not** the
  `#11-inline-item-boundary-soft-wrap` class, which *is* slotted and is also packer break
  machinery: that slot tracks a wrap the engine **performs** where css-text-3 §5.5 gives no
  opportunity — a divergence with a behaviour on both sides — whereas `<br>` performs nothing at
  all, so there is no behaviour to diverge. Opening a slot here would book a feature request
  against a program whose subject is inline decoration. ⚠ **Two claims rev 32
  made here are withdrawn** (round 25, Axis 3): "no cell of this program is constructible with
  either" — `<br style="padding:10px">` is an instance of Shape B, so it constructs cells 8 and 12
  exactly as they are written — and "M1's predicate admits them unchanged the day they exist, so
  nothing here needs amending then", which read the tags as outside the program when the four
  filters above put a decorated one inside it today. ⚠ An earlier revision
  made that attribution ("a pre-existing gap §9 records") with no §9 entry behind it —
  `git show cc145374:<memo> | awk '/^## §9\./,/^## §10\./' | grep -ciE 'wbr|BrMarker'` → 0
  (round 24 audit). ⚠ The command is anchored to `cc145374` deliberately: quoted against the
  working copy it returns a **non-zero** count that grows with every revision of this bullet (4 at
  rev 32, more at rev 33) and is therefore not written here, because **this bullet** is what makes
  it non-zero — the
  [[feedback_document-landing-invalidates-its-own-measurements]] class, caught by the round-24
  gate.
* **`FragmentTree` / `BoxFragment` and terminal-Z C-3/C-4 — the sibling that owns per-fragment
  geometry, and why this umbrella does not reach for it.** `crates/core/elidex-ecs/src/fragment_tree.rs`
  already carries the ratified per-`(entity, fragmentainer)` store (`BoxFragment` at `:155`; `FragmentContent` today has the single variant `Box(BoxFragment)`, and
  the module doc at `:22` names `InlineLines` as "when it lands", i.e. not yet a variant), and `inline/mod.rs:375-376` marks per-fragment
  inline `LayoutBox`/clientRects as "committed-next (cssom-view store consume)". Terminal-Z C-3
  builds a `client_rects(entity)` two-source dispatch and **C-4 retires `LayoutBox` +
  `InlineClientRects`** outright. **Disposition**: this umbrella writes only to surfaces that exist today
  and adds **no new carrier** — M4 fills `LayoutBox`'s existing edge fields and leaves
  `InlineClientRects` untouched. ⚠ **"Carrier" is used in two senses in this memo and they must not
  be conflated.** Here and in the two bullets below it means a **persisted geometry surface**
  (`LayoutBox` / `InlineClientRects` / `BoxFragment`) — the thing C-4 would have to unwind. In §5.1
  M4 it means the **in-pass transport** from the marker payload to the box-assigner, whose choice is
  explicitly undecided. The claim here holds under all three of M4's options, because every one of
  them is `LinePacker`-local and dropped when the IFC pass returns (§5.1 M4's grounds measure it),
  so none of them persists anything for C-4 to unwind. The ground that this costs C-4 nothing is **in this repo, not in
  C-4's scope**: `impl From<&elidex_plugin::LayoutBox> for BoxFragment` (`fragment_tree.rs:183`)
  projects `content`/`padding`/`border`/`margin` 1:1 and its docstring calls itself "the single
  source of the `LayoutBox`↔`BoxFragment` field correspondence", so M4's three fields carry across
  mechanically. (Asserting instead that "C-4 has nothing to unwind" would be a claim about another
  program's scope made from here — the error this memo's front matter exists to prevent.)
  ⚠ **The live overlap is C-3b, not C-4.** The C-3 consumer-migration memo's `getClientRects()`
  row reads "OPEN → **C-3b** … for an inline split across both lines and columns, today's
  `InlineClientRects` is per-column/G11 state and true per-fragment inline rects are
  committed-next … C-3b pins the dispatch" — the same open question §5.3 books into
  `#11-inline-box-decoration-splits` and cells 17d(b)/17f route there. So that slot **co-owns**
  the multi-fragment dispatch with C-3b and must not decide the carrier alone; §10 amends its Why.
  ⚠ **Which C-3 documents are authoritative**: the C-3a seam-and-audit and impl plans are **merged
  on `main`** (`docs/plans/2026-07-terminal-z-c3a-seam-and-audit-plan.md`,
  `…-c3a-impl-plan.md`); the consumer-migration architecture memo is on the unmerged
  `origin/terminal-z-c3-plan` and self-declares "pre-`/elidex-plan-review` design anchor", so it is
  read as intent, not as ratified fact.
  ⚠ **Lane sequencing**: C-3 states it is "not layout-only and not parallel-safe … coordinated
  sub-slices, not a single PR". Both programs are in the Layout lane. This umbrella's seven PRs do
  not block on C-3 (they add no carrier and no consumer), but the splits slot does — its trigger is
  amended to name C-3b alongside PR-1d landing.
* **`#11-inline-spec-cite-misattribution`** (new slot, **pre-existing** class): the wrong-section
  citations §3.1 records, which this program *found* but did not create. The classes are concept-grep classes and pattern-less hand-offs, **enumerated — and counted — once, at the end of this bullet**, inside the enumeration that determines the figure; §10's row was stripped of the count for exactly that reason and an earlier drafting of this sentence restated it here, giving one figure two sites (round 25, Axis 3). §3.1's three come first, each
  defined by a concept grep because round 16 measured that a coordinate list under-covers every one
  of them: `grep -rEn "Box Model (L3|Level 3)[^a-z]*(§)?5\.3" crates/` (7 hits / 5 files →
  css-box-3 §3.1/§4.1); `grep -rEn "CSSOM[ -]?View[^)|]{0,15}§?\s*5\b" crates/` (4 hits / 3 crates
  → cssom-view-1 §6 for `Element` members, plus `layout_query.rs:355`'s cssom-view-1 §6 → §7, since
  `offsetParent` is on `HTMLElement`); and `grep -rn "9\.2\.2\.1" crates/` (9 hits — CSS 2 §9.2.2.1
  is *Anonymous inline boxes*, not the box-suppression rule, which is CSS 2 §9.4.2 / css-inline-3 §2.3;
  ⚠ **not every hit is wrong** — `text_height/layout_box.rs:74` explicitly says "NOT §9.2.2.1", so
  membership must be derived, not assumed). Also the "one border-box fragment per line"
  restatement of cssom-view-1 §6 step 3 at `boxes.rs:90-91` and four further sites — defined by
  the command, not this list: `grep -rn 'fragment per line' crates/` → 6 hits / 3 files on
  `154bac3f` (so the slot receives **seven classes: four with a concept grep** — Box Model,
  CSSOM View §5, `9.2.2.1` from §3.1, and `fragment per line` from here — **and three pattern-less
  hand-offs**: the css-backgrounds `:262`/`:270` pair, the `layout_query.rs:355` cssom-view-1 §6→§7 outlier
  §3.1 names beside its CSSOM grep, and `crates/css/elidex-style/src/pseudo.rs:45` at `22de3078`,
  whose comment attributes "on pseudo-elements, `content: normal` computes to `none`" to "CSS
  Generated Content §2" — the rule is css-content-3 **§1**, the `content` property definition
  ("For ::before and ::after, this computes to none"), and css-content-3 §2 is *Generated Content Values: the
  `<content-list>` type* (`webref heading css-content-3 1` / `2`). It is a hand-off rather than a
  grep because the wrong label is "CSS Generated Content §2", a name outside §3.1's concept greps;
  and it is not PR-1a's touch set — PR-1a re-routes `collect.rs:265`, not `pseudo.rs` (round 23,
  Axis 4)).
  **Why deferred, and why not folded in**: none of these is self-seeded — the citations were wrong
  before this program and stay wrong after it, so correcting them here would bundle a sweep with
  mechanism work, the shape `docs/plans/2026-07-citation-hygiene-umbrella.md` records as
  force-carved ("PR-A0 bundled a … citation sweep with a … detector … the decisive one was the
  shape, not any single defect" — that memo is on the unmerged `webref-cite-audit-tool` branch of
  open PR #501, so it is read as recorded experience, not ratified fact). The one cite this program
  *does* correct is `pack/mod.rs:726`, in PR-1b, because PR-1b changes what that comment documents.
  Trigger: any lane already sweeping citations in `elidex-layout-block`, `elidex-plugin`,
  `elidex-style` (two of the three hand-offs live there), `elidex-shell` or `elidex-dom-api`, or the citation-hygiene program reaching its `crates/**`
  re-derivation slice. Re-eval: 2026-11-01.
* **The canonical *inline box* predicate, with the `client*` guard as its first consumer — carved
  to a third standalone prereq PR, ordered before PR-1a.** ⚠ **Deliberately larger than "add a guard
  to two getters", on a measured ground.** css-display-3 §A's *inline box* is needed by **two**
  consumers inside this program — the four `client*` members (`elidex-dom-api`) and **M1's emit
  test** (`elidex-layout-block`, PR-1a, §6 cell 6c) — and elidex has no site that answers it. Giving
  each consumer its own answer is the state CLAUDE.md's *one issue, one way* forbids ("新 seam +
  N 個の legacy 実装 が共存する strangler 中間状態を残さない"); the pragmatic alternative — excluding
  replaced elements in `collect.rs` with the in-crate `get_intrinsic_size` proxy (`helpers.rs:405`)
  while the guard picks its own — is the "現実解" *ideal over pragmatic* declines by default, and it
  is **load-state-dependent** besides — measured, not supposed: `get_intrinsic_size` reads the
  `ImageData` **component** (`helpers.rs:407`), which **two** production sites attach and no other
  — `decode_image` (`crates/shell/elidex-navigation/src/loader.rs:392-396`), whose result is
  inserted at `:286`, and `elidex_api_canvas::sync_dirty_canvases`
  (`crates/api/elidex-api-canvas/src/component.rs:132-148`), which inserts at `:140`; both attach
  it only **after** pixels exist, so an `<img>` that has not loaded or
  whose decode failed carries none and reads as **non-replaced**. A predicate whose answer depends
  on network timing is not one css-display-3 §A describes. ⚠ **"Whose only producer is
  `decode_image`" is withdrawn** (round 26, Axis 3): §8 requirement 5 already said so in this same
  memo — "⚠ An earlier revision called `decode_image` the 'only producer' of `ImageData`; it
  is not", naming `sync_dirty_canvases` and the undrawn-`<canvas>` case it makes reachable — and
  this bullet is the face the blocking prereq PR reads for its problem statement, so the universal
  stood refuted at one site and unrefuted at the other. The conclusion never needed the
  universal: what it turns on is that *no* producer attaches the component before pixels exist. ⚠ **But this is a pre-existing
  engine-wide convention, not a defect this program creates or must lift**: component-presence *is*
  how elidex asks "replaced" everywhere — `block/mod.rs:224`, `intrinsic/mod.rs:98`,
  `crates/layout/elidex-layout-multicol/src/fill.rs:418`, and `resolve_block_height`'s `is_replaced: bool`
  (`block/children/helpers.rs:384`) is fed by exactly this `.is_some()`. So the carved PR **inherits**
  the convention; it is not handed an unbudgeted invention. ⚠ **What the handover requires is §8's
  seven requirements and nothing else** — that paragraph is the obligation face (clause (c) below
  says so), and this bullet opens no second list beside it. What the carved PR is *not* asked to
  lift is the pre-existing engine-wide presence convention wherever it **sizes** a box; the
  predicate's **own** answer is a different question and is already settled by **§8 requirement 5**,
  which is unconditional ("the emit decision must not be keyed on decode outcome", with no "or
  justify" branch) and whose enforcement lever is §6 cells 6c/6f. ⚠ An earlier drafting wrote that
  load-independence "is **not on that list** … so the PR's plan-review can decide **whether to lift
  it**" — a second obligation list for one obligation, disarming the requirement this program's
  blocking prereq must satisfy (round 25, Axis 1). So the PR establishes **one** predicate in a home both crates reach, and both
  consumers consume it. It is ordered before **PR-1a** because that is where the first consumer
  lands — not before PR-1c, which an earlier revision wrote when only the guard was in view.
  ⚠ **Its touch set is therefore NOT knowable from here, and this memo asserts none.** The home,
  whether `FormControlState` needs a crate edge or a move, and whether `is_atomic_inline`
  (`inline/collect.rs:14`) is subsumed or left standing are that PR's plan-review questions; it
  re-runs the cold gate at whatever width it lands on
  ([[feedback_split-on-touch-prereq-workflow]]). An earlier revision claimed "`elidex-dom-api` only,
  disjoint from both other prereqs, lands any time" and then derived the branch topology, the
  tooling-ship paragraph and a third cold-gate width from it — a premise asserted at four sites
  while being listed among what this bullet hands over.
  Step 1 is *identical* across `clientTop`, `clientLeft`, `clientWidth` and `clientHeight`
  (`body cssom-view-1 dom-element-clienttop` prints all four; §3's CSSOM row quotes it), and elidex
  implements it for **none** of them: `clientWidth`/`clientHeight`
  (`crates/dom/elidex-dom-api/src/element/layout_query.rs:119-131` → `get_padding_box`, `:346`)
  already return a non-zero padding box for an inline **today**, and `clientTop`/`clientLeft`
  (`:133-151`) read `lb.border.top`/`.left` directly and are zero only because
  `inline/pack/boxes.rs:82-84` hard-codes `EdgeSizes::default()`. ⚠ The two halves rest on
  **different** fields, which one coordinate cannot name: `padding` is `:82`, `border` is `:83`,
  `margin` is `:84`. This memo cites the `:82-84` block wherever the claim is "all three edges are
  zeroed"; the `clientTop`/`clientLeft` half rests on `:83` alone and the
  `clientWidth`/`clientHeight` half on `:82` alone. An earlier drafting cited `:82` for both.
  ⚠ **A shared helper makes the obvious "one place" fix wrong**: `clientWidth`/`clientHeight` and
  `scrollWidth`/`scrollHeight` (`:155-170`) go through the **same** `get_padding_box` (`:346`), and
  `scrollWidth`/`scrollHeight` carry **no** inline clause — `body cssom-view-1
  dom-element-scrollwidth` gives **two** unconditional zero-returns, step 2 ("If document is not the
  active document") and step 6 ("If the element does not have any associated box"), and **neither is
  keyed on inline-ness**, which is the property this argument needs (⚠ an earlier drafting called
  step 6 the *only* zero-return — round 25, Axis 4; the conclusion is unaffected). Guarding inside
  `get_padding_box` would silently zero both scroll members for every inline. Handed over with the
  mechanism, because it is the trap the mechanism decision walks into.
  ⚠ `offsetTop`/`offsetLeft` also carry no inline clause and are **not** in scope: `body cssom-view-1
  dom-htmlelement-offsettop` step 1 zero-returns only for the body element or no associated box, and
  the rule is a **first-box** one ("An inline element that consists of multiple line boxes will only
  have its first box considered"), whereas elidex derives them from the union `border_box()`. That is
  a different divergence from cell 17f's union-vs-`getClientRects` one and belongs to
  `#11-inline-box-decoration-splits` with the rest of the per-fragment attribution; §7's reader
  family names the members without separating the two rules.
  **Why it is a PR and not a slot**: PR-1c fills those edges, so deferring the guard would ship a
  *new* violation on `<span style="border:5px solid">` — CLAUDE.md is unconditional on that
  ("TODO 先送り禁止"), and the correct direction is monotone: landing the guard first changes
  `clientTop`/`clientLeft` for a non-replaced inline from 0 to 0 (now by construction) and
  `clientWidth`/`clientHeight` from the padding-box size to 0 (a pre-existing violation closed),
  and PR-1c then cannot regress it.
  **Why it is not a cell of PR-1c**, which is what an earlier revision made it: (a) the predicate is
  css-display-3's *inline box*, "A non-replaced inline-level box whose inner display
  type is flow. …" (the `…` marks §A's second sentence, unused here) — **two** inputs, not one, so a guard keyed on `Display::Inline` alone is wrong by
  construction. Both the quote and the §-number come from lookup, not recall:
  `.claude/tools/webref dfn css-display-3 "inline box"` → `§A Glossary #inline-box`, and
  `body css-display-3 inline-box` prints the sentence and, beside it, *atomic inline*;
  (b) elidex answers only the formatting-context half — `is_atomic_inline`
  (`inline/collect.rs:14`) matches `InlineBlock`/`InlineFlex`/`InlineGrid`/`InlineTable` and nothing
  about replacedness, while css-display-3 §A's *atomic inline* is "replaced (such as an image)
  **or** … establishes a new formatting context … **and cannot split across lines** (as inline
  boxes and ruby containers can)", so **where** the one canonical answer to "is this
  an inline box" should live is an open question this umbrella must not settle by adding a second
  one. ⚠ **The survey must be a concept grep, and an earlier drafting of this very clause was
  self-seeded**: it wrote `grep -rEn 'fn is_(atomic_inline|block_level)' crates/`, which names the
  two functions it claims to have found and cannot discover a third — the exact failure
  [[feedback_semantic-sibling-selfseed-and-regate-breadth]] names, committed in the sentence citing
  it. ⚠ **Its replacement was self-seeded one level down, and rev 34 replaces that too** (round 26,
  Axis 1): the grep
  `grep -rEn 'fn [a-z_]+\((&|self|display)[^)]*Display\)? *-> *bool' crates/` requires the literal
  `Display` **inside the parameter list**, which is a calling-convention vocabulary, not the
  property being tested. It returns exactly three —
  `elidex-layout/src/layout/anonymous_table.rs:12`, `elidex-layout-block/src/inline/collect.rs:14`
  and `elidex-layout-block/src/block/mod.rs:46` — and **none in `elidex-plugin`**, the crate the
  clause below names as the home, while no `&ComputedStyle`-form answer can match it at all and
  its `self` alternative is dead. The positive control outside its vocabulary is
  `pub fn is_multicol(style: &ComputedStyle) -> bool`
  (`crates/core/elidex-plugin/src/computed_style/columns.rs:28`), whose body matches on
  `style.display`: a real display predicate the grep cannot see. That the grep's three hits are
  non-empty proves nothing about its reach — [[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]].
  **The survey is therefore defined by the property**: *a bool-returning fn that states a
  `Display` categorisation in its own body* — it reads `Display::`, or it is a member of
  `impl Display`, which writes `Self::`. Its **output is the inventory of competing answers**,
  and nothing in that inventory is a curated list:

  ```
  awk 'FNR==1{d=0;o=0;n=""}
  /^impl Display[ {]/{d=1} d&&/^\}/{d=0}
  /(^|[^a-z_])fn [a-z_0-9]+/{n=$0;sub(/^.*fn /,"",n);sub(/[^a-z_0-9].*/,"",n);i=$0;sub(/[^ \t].*/,"",i)}
  !o&&/-> *bool[ \t]*\{[ \t]*$/&&n!=""{ind=i;nm=n;s=FNR;h=d;o=1;n="";next}
  o&&$0==ind"}"{if(h)print FILENAME":"s":"nm;o=0;next}
  o&&/Display::/{h=1}' $(git ls-files 'crates/**/*.rs')
  ```

  Its hits on `154bac3f` are **seven** — `is_multicol` (`elidex-plugin/src/computed_style/columns.rs:28`),
  `is_table_internal` (`elidex-plugin/src/computed_style/display.rs:35`), `is_hidden`
  (`elidex-a11y/src/tree.rs:280`), `is_block_level` (`elidex-layout-block/src/block/mod.rs:46`),
  `establishes_bfc` (`:694`), `is_atomic_inline` (`elidex-layout-block/src/inline/collect.rs:14`)
  and `needs_table_wrapper` (`elidex-layout/src/layout/anonymous_table.rs:12`). This feeds §8
  **requirement 2** ("one canonical answer, not a second beside `inline/collect.rs:14`"), so the
  survey is load-bearing for the blocking prereq and not colour.
  ⚠ **What the instrument finds is *competing definitions*, not every fn that discriminates on a
  `Display` value, and rev 34 claimed the wider set** (round 26, Axis 1, Gate B): at least four
  bool fns discriminate on a `Display` value by **delegating** to `is_block_level` and never write
  `Display::` at all — `elidex-render/src/builder/walk.rs:700` `is_block_child`,
  `elidex-layout-block/src/block/mod.rs:70` `children_are_block`,
  `elidex-layout-block/src/inline/collect.rs:70` `has_direct_block_child` and
  `elidex-layout/src/hit_test.rs:263` `is_block_or_float`. They are **callers** of the canonical
  answer, not rival answers to it, so requirement 2 is untouched and the conclusion survives — but
  the claim has to be the narrow one, because widening the regex to catch them would drag in every
  *consumer* and stop being a survey of answers. The right reading of "its output is the
  inventory" is therefore: of the fns that **state** a `Display` categorisation, these seven are
  all of them; delegating consumers are a different population and are not what requirement 2
  counts.
  ⚠ **And it surfaces the home the parameter-list grep hid**: two of those seven are in
  **`elidex-plugin`**, which already carries `pub` spec-cited `Display` categorisation in
  `computed_style/` — `is_multicol` (`columns.rs:28`, "CSS Multi-column L1 §2") and, in
  `computed_style/display.rs`, `impl Display`'s `is_table_internal` (`:35`, "CSS 2.1 §11.2") and
  `blockify` (`:52`, "CSS 2.1 §9.7 / Flex §4.2 / Grid §6.1") — in the one crate **both**
  consumers already depend on, with **zero** new edges. ⚠ **`is_scroll_container` (`:97`) and
  `clips` (`:109`) are struck from that list: they are `impl Overflow` members, not `impl Display`**
  — `impl Display` opens at `:31` and `impl Overflow` at `:94`, identical in both frames
  (`git show 154bac3f:crates/core/elidex-plugin/src/computed_style/display.rs | grep -n 'impl \|pub fn '`)
  — so a four-precedent claim was two of `Display` and two of a different enum that happens to
  live in the same file (round 26, Axis 1). The conclusion survives on the three above. So the
  *display* half of the predicate
  (outer type, inner type, generates-a-box-at-all) has an existing home and needs no crate work at
  all; only the **replacedness** half needs a component-bearing crate. An earlier drafting said
  "elidex has no site that answers it", which is true of the composed predicate and false of its
  larger half;
  (c) `elidex-dom-api` has no layout dependency (§5.2), so the predicate cannot simply call
  `collect.rs` — and the **replacedness half is not reachable there either**: `get_intrinsic_size`
  (`helpers.rs:405`) tests `ImageData`, `FormControlState` and `IframeData`; the first and third
  live in `elidex-ecs` (`crates/core/elidex-ecs/src/components.rs:405`, `:698`), which
  `elidex-dom-api` depends on, but `FormControlState` lives in `elidex-form-core`
  (`crates/dom/elidex-form-core/src/lib.rs:348`), which it does **not** — measured with
  `sed -n '/^\[dependencies\]/,/^\[/p' crates/dom/elidex-dom-api/Cargo.toml` (plugin, ecs,
  script-session, css, custom-elements, style, hecs, url; and no `[dev-dependencies]` section at
  all). The gap is author-reachable, not theoretical: `input { display: inline }` makes a **replaced
  form control** `Display::Inline`, exactly the case where the guard must not fire.
  So "a four-member fix in `elidex-dom-api`" buys either an **incomplete predicate** or a **new
  crate edge**. ⚠ **The incomplete branch is not open to it**, and an earlier drafting left it open
  while PR-1a was already made to depend on the complete answer: this memo itself names the case it
  breaks — `input { display: inline }` makes a **replaced form control** `Display::Inline`, so an
  incomplete predicate silently falsifies §6 cell 6c and lets M1 emit a marker for a box
  css-inline-3 §2.3 clause 3 does not reach. **§8's *Predicate prereq PR* paragraph is the obligation face** and states seven
  requirements, of which these two are the ones this clause implies: the replacedness half is
  **complete** over what `get_intrinsic_size` covers, and the composed predicate is **callable from
  `elidex-layout-block`**, because M1 is a consumer. ⚠ An earlier revision asserted "§8 states them"
  when §8 stated three *different* things and PR-1a's DoD named no predicate at all. *How* — the home,
  the edge, the split between the `elidex-plugin` display half and the component half — stays the
  carved PR's question, and is ⚠ **not** a settled premise this memo may derive an ordering from
  (see the ⚠ under **Ordering** below);
  (d) it flips a currently-green test — `client_top_returns_border_width`
  (`element/layout_query.rs:497`) inserts a `LayoutBox` with `border 3/2/1/4` and **no**
  `ComputedStyle`, and `Display::default()` is `Inline` (`keyword_enum!` gives the first variant
  `#[default]`, `crates/core/elidex-plugin/src/computed_style/mod.rs:27-42`;
  `computed_style/display.rs:8` lists `Inline` first), so the guard makes it return 0.
  ⚠ **What this memo does *not* assert**, and an earlier revision did: that
  `<img style="border:5px">` "reports 5 today". **Measured false**, in three steps, each with the
  command that establishes it. (1) `<img>` matches **no** UA rule at all:
  `grep -n '\bimg\b' crates/css/elidex-style/src/ua.rs` → **no hits**, so its `display` stays the
  initial `inline` (`Display::default()`). ⚠ Not "`ua.rs` has one `display` rule" — an earlier
  drafting of *this* sentence said that and it is false: `grep -n 'display\s*:' …/ua.rs` returns
  many, and **no number is stated here** because the count depends on whether test comments and
  grouped selectors are counted — a convention-dependent figure is an argument, not a measurement
  ([[feedback_convention-dependent-figures-are-argument]]). The claim that holds is the *absence of
  an `img` selector*, which is the one the argument needs and which is convention-free. (2) `is_atomic_inline` is **false** for `Inline` (`inline/collect.rs:14-19` matches four
  keywords, none of them `Inline`), so the IFC treats it as a plain inline box and recurses into its
  children, of which `<img>` has none — no `InlineItem` is emitted. (3) `assign_inline_layout_boxes`
  iterates `entity_bounds` (`inline/pack/boxes.rs:56`), which only `place_item` populates
  (`pack/mod.rs:706`), so the element gets **no** `LayoutBox` from the inline path and `clientTop`
  returns 0 via `map_or` (`element/layout_query.rs:137-140`). Corroborating grep:
  `grep -rn 'replaced\|ImageData\|get_intrinsic_size' crates/layout/elidex-layout-block/src/inline/`
  returns **exactly one** hit, `inline/styled_run.rs:12` — a doc comment on `InlineItem::Atomic`
  reading "An atomic inline-level box (e.g. `inline-block`, replaced element)". ⚠ It is the *only*
  place the inline module names the concept, and it is prose: no code in that module reaches a
  replaced element. (An earlier drafting of this sentence claimed the grep returned nothing, which
  is the error the sentence exists to prevent.) The spec half stands; the
  "and does today" half does not, and the replaced-inline gap it exposes is that PR's question to
  dispose of, not this one's.
  **Scope handed over**: the predicate's **home** and whether it needs a crate edge or a component
  move; whether `is_atomic_inline` (`inline/collect.rs:14`) is subsumed or left standing; all four
  `client*` members; the reader half of the integration assertion (§8's PR-1c DoD gives the split);
  the shared-`get_padding_box` trap above; the generated-content members of the domain
  (§8 requirement 7); and the disposition of
  `client_top_returns_border_width`. Its own plan-memo and `/elidex-plan-review`, like every PR here.
  **Ordering**: in `main` before **PR-1a**, because M1's emit test is one of its two consumers
  (§5.1 M1, §6 cell 6c). ⚠ **This memo claims nothing about its touch set** — not disjointness from
  the other two prereqs, not a sibling topology, not a cold-gate width. All three follow from the
  home decision, which is handed over; §8's ordering paragraph and §9's cold gate now say so, and an
  earlier revision asserted all three from a premise it had already delegated.
  **Deferrals**: whatever it opens is its own memo's call, not counted against this program's
  per-PR budget (§5.3) — it is carved precisely because its questions are not this memo's.
  ⚠ **What it does not fix, and must say so**: elidex has no replaced arm on the inline path at all
  (`is_atomic_inline` tests display keywords only, and `collect.rs`/`atomic.rs` reach no replaced
  element), so a replaced inline still gets no correct atomic layout after this PR. The predicate
  makes the *classification* available and cell 6c keeps this program off the gap; closing the gap
  is separate, pre-existing work neither this PR nor this umbrella takes on.
* Ruby annotations (css-inline-3 §2.3 clause 4) — unimplemented engine-wide.
* **`flush_line`'s non-persisting arm (`inline/pack/mod.rs:393-421`) is dead, and this program
  deletes it.** `persist_candidate` (`inline/mod.rs:239`) is identically true — `FragmentationType`
  has only `Page`/`Column` and the constraint's field is non-optional — so `flow_align` is always
  `Some` and `LinePacker::new` has one call site. **A second standalone prereq PR** (stacked beside
  the seam-3 split, not folded into it, so the byte-identical criterion stays provable) removes the
  arm and the `persist_candidate` branch, and **closes
  `#11-inline-align-clientrects-nonpersist-path`**, which books work against the same unreachable
  code. Booking a fold, a DoD cell or a *new slot* against it would all be wrong: CLAUDE.md is
  unconditional on dead code, and deleting collapses two slots instead of opening a third.
* **The two plan checkers are not on any loop.** `plan-sweep.py` / `plan-xcheck.py` live in
  `.claude/tools/` and are invoked by hand: nothing in `scripts/trip-wires.sh`, `mise.toml` or
  `.github/workflows/ci.yml` **runs** them. ⚠ Say *runs*, not *references* — an earlier revision
  wrote "nothing … or `.claude/skills/**` references them" and this branch's own
  `.claude/skills/elidex-plan-review/SKILL.md` note references them by name, so the stated grep
  returned a hit and falsified the sentence that offered it. Per
  [[feedback_every-triggered-pr-must-be-on-the-loop]] a checker reachable only by remembering it
  is the habit that failed six rounds with a tool attached. **Disposition**: they are **not**
  generic yet — `plan-xcheck.py` hard-codes a `PR-1[a-z]` label shape, a memo-specific superseded-range dict and
  this umbrella's own slot name — so generalising them is part of the work, not a precondition.
  They belong in `elidex-plan-review`'s Step 1.5 or the ungated `trip-wires` job, as **its own
  PR**, not folded into this umbrella. ⚠ **Not a `#11-` slot, and not an own deferral.**
  This is **skill infrastructure, not a platform gap**, and `.claude/skills/elidex-plan-review/SKILL.md`
  already sets the precedent for exactly this class: its own unbuilt §2 preflight hard-gate is held
  as a *standing maintenance note in the skill*, "deliberately NOT a `#11-*` platform slot per
  `feedback_defer-slot-eligibility-audit-at-create`: it fails the slot-fit audit". Registering a
  `#11-` slot here would put a tooling task in the platform SoT against that precedent — so it is
  written instead as a **standing maintenance note in `.claude/skills/elidex-plan-review/SKILL.md`**
  — already added on this branch, in the same durable home the precedent uses and for the same
  stated reason ("durable *here in the skill*, read every plan-review — so it can't be silently
  lost"). A per-program memory file is opened only while this umbrella is live, whereas this task's
  scope outlives it, and a landing-scoped row would leave a trigger that fires *every round*
  unowned until landing. Scope: put both
  checkers on a loop (`elidex-plan-review`'s Step 1.5 or the ungated `trip-wires` job — ⚠ #510
  registers its own plan-memo checker wire in `scripts/trip-wires.sh`'s `REQUIRED_WIRES`, the same
  hunk, and ships a lexer/blocks/tables substrate; this task lands **on its trigger below** —
  #510's resolution or TERMINAL — and its memo
  decides build-on-vs-beside that substrate), generalise `plan-xcheck.py` off this memo's labels,
  decide — on its merits, whichever substrate — and if adopted implement **two-file awareness**
  for both checkers (the memo-split question §9 books to this task, below), and retire or rewrite the SKILL.md note that
  describes the pre-state. ⚠ The `SPEC_LABEL_REVERSE`
  CSS-label gap is **not** this task's: it is `#11-preflight-css-module-labels` in the SoT
  (citation-hygiene Slice B, after A-ii migrates the dict) — an earlier revision booked it here
  too, a second decision surface for one gap, in the direction that lane's open slice deletes;
  until that slot lands every CSS plan hand-verifies its citations (§3). ⚠ Until the wiring is
  done, **the per-PR plan-reviews under this umbrella (and this memo's landing-record revisions)
  depend on running the checkers by hand** — the habit the task exists to end.
  ⚠ **Trigger: #510's resolution — landing or closure — or this umbrella's TERMINAL, whichever
  comes first** (the sibling checker substrate this task must decide against; if #510 is still
  open at TERMINAL the memo decides against `origin/main` as it then stands — another lane's PR
  must not hold this task past this program's terminal; the approval PR waits only on the renewed
  TERMINAL, §8);
  **discharger: the tooling PR**, cut from `origin/main` (approval-independent skill infra, by this
  bullet's own classification), under `/elidex-plan-review` — not because it is edge-dense but
  because #506 shipped checker tooling without one and paid a tooling-only IMP tail across several external rounds (the SoT's #506
record; no count carried here) before being carved into #510. Earlier revisions keyed the trigger to "the PR that ships
  these files" and named the seam-3 prereq PR (which shipped neither), then PR-1a (which would
  land the checkers unconnected — dead code by CLAUDE.md's rule), then the tooling PR itself (a
  trigger that is the work cannot fire unfired); the event is "#510's resolution or TERMINAL" —
  TERMINAL is the disjunct that fired, 2026-09-07 — and the PR is the discharger. The same-day
  Terminator reset (two clean rounds owed) does **not** unfire it: the trigger is an event, the
  tooling scope (two checkers + a SKILL note) depends on no design round, and its own plan-review
  decides against `origin/main` as it then stands.
  Before all that, a revision wrote "this memo's next plan-review round", which fires *every*
  round and can discharge nothing, so it expired unheard six times running; a trigger that recurs
  is not a trigger.
  Re-eval: 2026-11-01.

## §10. Slot ledger actions at landing

| Action | PR |
|---|---|
| ✅ **Done 2026-09-07 (memory bookkeeping — no landing gate)**: this umbrella's slot and `#11-css2-spec-label-normalisation` (a #497 carve, **pre-existing** class; its Why/trigger are in `project_css2-spec-label-normalisation.md`, not restated here) are registered in `project_open-defer-slots.md` (the SoT per MEMORY.md), each with **its own recorded date rather than a fresh one** — `#11-css2-spec-label-normalisation` **2026-10-31** from its slot memo; this umbrella's own slot, which has none, takes 2026-11-01. This umbrella's own slot is **pre-existing** class: Codex opened it on #497, not this program. `#11-inline-fragmented-fn-decomposition` (carved from #495, pre-existing, **2026-10-28**) is **no longer part of this row** — #508 registered *and* partially closed it (next row); an earlier drafting of this row would have re-registered a closed slot with its pre-close date. ⚠ A registration is made true by the slot's *existence*, so routing it to a PR (the seam-3 prereq, then PR-1a, in earlier revisions) was deferral dressed as routing; the SoT's "they land with the umbrella" note is struck accordingly. | done (memory) |
| ✅ **Landed with #508 (`7e256029`)** — the one row #508 carried, as the bookkeeping its own change made true. Register **and** close `#11-inline-fragmented-fn-decomposition` in one row — it is registered as closed-on-landing, not registered then closed — **as a partial close**, naming the seams §9 measures as still open, in the successor slot `#11-inline-fragmented-fn-seams-1-2` (**mixed** class per the SoT's landing record — the seams predate this umbrella, but `reconcile_flows`' extracted signature is created by #508, which makes it that PR's one **(own)** deferral, §5.3; an earlier drafting said "pre-existing" on the seams alone; Why: the prereq PR discharges seam 3 only; **trigger: canonical in the slot memo's Trigger section — the disjuncts the slot memo carries, no count here (a count goes stale each time the slot adds one; the per-program memory says so) — and this row no longer restates it**: disjuncts 1–2 are the two this row originally prescribed (their text, the six-PR exemption and its two grounds, and the predicate carve-out now live in the slot memo, not here), and the slot added further disjuncts for the items 1–2 cannot reach. A verbatim restore of this row's old text would drop the disjuncts the slot added (🔴 per-program memory). ⚠ **Disjunct 3 (slot memo) exempts no umbrella PR**: it fired at #511, and no later umbrella PR re-fires it — none of PR-1a–1d edits `inline/reconcile.rs` (last row); re-eval 2026-11-01). | seam-3 prereq PR |
| Open `#11-inline-spec-cite-misattribution` (**pre-existing** class) with the classes §9 enumerates — defined by §9's concept greps and hand-offs, and counted only there, inside the enumeration that determines the figure (a count restated here is a second site that can drift from it; this row carried one until rev 31 added a hand-off and had to move it at three sites in one commit) — Why, trigger and date in §9 (re-tagged under the rule the `#11-inline-root-inline-box` row below states; an earlier drafting left this third pre-existing row at PR-1a — the rule was added and not swept, round 23, Axis 3) | approval PR |
| Open `#11-inline-box-decoration-splits` (own) with the Why / trigger / date in §5.3, **and the §9 note that its carrier choice (`FragmentTree` vs a widened `InlineClientRects`) belongs to terminal-Z C-3/C-4, not to the slot alone** | PR-1c |
| Open `#11-inline-min-content-box-edges` (own) with the Why / trigger / date in M8 | PR-1b |
| ✅ **Landed with #511 (`22de3078`, 2026-09-07)** — Close `#11-inline-align-clientrects-nonpersist-path` — the arm it books work against is deleted | dead-arm prereq PR |
| Note on `#11-layoutbox-absence-unreachable` (#488) that M5 withholds no entity's box, so this umbrella needs no truthful box-absent signal **for its mechanism** — ⚠ booked to **PR-1d**, not to the PR that first grants a box: the statement is M5's and is only true once the flip lands, whereas under PR-1c box-absence is still a live signal (§6 cell 6 asserts "phantom ⇒ no box", sound only under §8's first-layout scoping) | PR-1d |
| Correct `#11-layoutbox-trip-wire-not-in-ci` **at every live site carrying the falsified premise, not at one** — the class, per [[feedback_semantic-sibling-selfseed-and-regate-breadth]], is whatever `grep -rln 'layoutbox-trip-wire-not-in-ci\|D4 gate runs only in local' <memory-dir>` returns, **and the row states no count** — the command's output is the class, and the sites' *dispositions* differ, so any figure here would be a classification dressed as a measurement ([[feedback_convention-dependent-figures-are-argument]]; an earlier drafting of this row said "three" and the grep returns more). Verified to carry the falsified premise and needing correction: `project_open-defer-slots.md:25` (open + the premise), `active-lane-detail.md:140` (lists it as the Layout lane's **NEXT TASK**, user-approved) and `project_inline-mod-split-owed.md:49`/`:98` (still OPEN + the premise verbatim). Verified **already** correct and needing none: `project_layoutbox-trip-wire-in-ci-next.md` (CLOSED). The remaining hits are triaged at landing by re-running the grep, not from this list. ⚠ That the sweep is *partial* rather than uniformly pending is the whole reason a one-site row would leave a future Layout-lane session picking up a task #496 landed. ⚠ §10's separate `active-lane-detail.md` rewrite row is scoped to *this slot's* framing and does not reach line 140. The premise: "the D4 gate runs only in local `mise run ci`" — PR #496 (`da958ace`) made the `trip-wires` job ungated and unconditional, so the slot is resolved and the premise is false. Found because §8's PR-1c DoD now cites that job's wire #5 instead of restating a grep, i.e. this program **depends on** the fix; a dependency left asserting the opposite in the SoT is the class §10 exists to close. ✅ **Done 2026-09-07 (memory, no landing gate)** — the grep returned seven files; the live falsified-premise sites were corrected (SoT `:30` → CLOSED #496, `active-lane-detail.md:140`, `project_inline-mod-split-owed.md` — the queue line `:49` and section B's heading, `:99` today and `:98` when this row was first written, the same site renumbered — and `project_c3a-impl-pr-ready.md:29`) and the dated historical mentions left as records. ⚠ Routed to the seam-3 prereq PR, then PR-1a, by earlier revisions; its truth-maker (#496) had landed on 2026-08-02, so tying it to a program PR only lengthened the window in which `active-lane-detail.md:140` advertised a landed task as the lane's NEXT TASK. | done (memory) |
| Open `#11-inline-root-inline-box` (pre-existing) with the Why / trigger / date in §5.3. ⚠ A pre-existing slot whose Why / trigger live only in this memo is registered when the memo is on `main` — the approval PR — not at a code PR (§10's first row: a registration is made true by existence; an earlier drafting routed this row to PR-1d and the next to PR-1b, the same deferral-dressed-as-routing that row withdraws). Own deferrals stay on the PR that opens them — the per-PR ≤3 count is measured at that landing | approval PR |
| Open `#11-inline-fallback-font-strut` (**pre-existing** class) with the Why / trigger / date in §5.3 — css-inline-3 §5.3's "only glyphs from fallback fonts" strut condition, which §8's closing paragraph used to fold into `#11-inline-root-inline-box`. Registered at the approval PR on the ground the row above states; **not** an own deferral, so it does not enter §5.3's per-PR count (the divergence is observable today on fallback-rendered text with no marker involved). ⚠ The same revision strikes the font-fallback-provenance disjunct from `#11-inline-root-inline-box`'s trigger, so the two slots do not both claim the condition — a slot registration that left the fold's surfacing disjunct standing would re-create the second decision surface it was opened to remove | approval PR |
| Open `#11-inline-item-boundary-soft-wrap` (**pre-existing** class) with the Why / trigger / date in §5.3 (found by Codex on #515; cell 15 stops pinning the divergence; registered at the approval PR on the ground the row above states) | approval PR |
| Open `#11-inline-zero-edge-box-in-item-stream` (own) with the Why / trigger / date in §5.3 — the cells it flips named there (found by Codex on #515) | PR-1a |
| **Close `#11-line-box-decorated-inline-content`** — §5.3 and §8 both assert it, and until now no ledger row carried it | PR-1d |
| The plan-checker standing maintenance note is **already written** into `.claude/skills/elidex-plan-review/SKILL.md` on this branch, not booked for landing — an earlier trigger, "the next plan-review round that runs them by hand", fired every round and discharged nothing, and a landing-scoped row would have left it unowned in exactly the window it matters (its trigger is now #510's resolution or TERMINAL, §9). **Not a `#11-` slot** — skill infra, per that file's own slot-fit precedent. This row records it; the **tooling PR** ships it with the two checkers (§8, §9) — the note lives in SKILL.md, a tooling file. ⚠ Routed to the seam-3 prereq PR, then PR-1a, by earlier revisions; #508 shipped neither the note nor the tools, and PR-1a would have landed them unconnected — the shipper is the §9 task's own PR, on its §9 trigger (#510's resolution or TERMINAL) and under its own plan-review; that PR retires or rewrites this note. | tooling PR |
| Split the joint "fold into terminal-Z C-3/C-4" parenthetical shared by `#11-inline-align-clientrects-nonpersist-path` and `#11-inline-relayout-box-staleness` — this PR closes the first, so the pairing stops holding **here**, and leaving it to a later PR would strand the SoT asserting a fold against a closed slot — ✅ **landed with #511 (2026-09-07)** | dead-arm prereq PR |
| Enrich `#11-inline-relayout-box-staleness`'s SoT entry with M4's write-path statement (§9), and note that `#11-inline-box-decoration-splits`'s work is first-layout-only until this slot lands — an **ordering** note, not a blocking dependency (§5.3) | PR-1c |
| Rewrite `project_line-box-decorated-inline-content.md`, `MEMORY.md`'s Layout-lane entry and `active-lane-detail.md`, all of which still carry a superseded framing of this slot. ⚠ Re-tagged from `seam-3 prereq PR` under the 2026-08-16 narrowing (§8); the per-program memory file is maintained round by round meanwhile, so what remains for the approval PR is the framing in `MEMORY.md` / `active-lane-detail.md` and the final state of the per-program file | approval PR |
| Record the successor program the close hands off to: `#11-inline-box-decoration-splits`, `#11-inline-min-content-box-edges` and `#11-inline-zero-edge-box-in-item-stream` **arm at PR-1d landing** (the splits slot on the first disjunct of its trigger; §5.3 gives C-3b as the other; the zero-edge slot on the first disjunct of its trigger, §5.3), and `#11-inline-fragmented-fn-seams-1-2` **does not arm at the close** — none of PR-1a–1d edits `inline/reconcile.rs` (PR-1d's `clear_inline_flows` item is a path consequence, §5.2/§7) and disjunct 1 is self-exempted. ⚠ It is **already armed**, independently of this program: its disjunct 3 fired at #511 (2026-09-07), the program's only `reconcile.rs` touch, so the reshape is unblocked now and ordered against nothing here — a Layout-lane task in its own right with its own plan-review (the signature's ECS question), re-eval 2026-11-01. Two earlier dispositions are withdrawn on the record: "neither of its disjuncts fires" (counted the disjuncts this memo prescribed, not the ones the slot carries) and "scheduled after PR-1d" (anchored to a `reconcile.rs` write PR-1d does not make — round 20, Axes 2/3). The Layout lane's next-task choice is therefore among **four**: the three slots that arm here and the already-armed successor | PR-1d |
