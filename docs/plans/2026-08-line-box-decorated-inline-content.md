# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's task, user-approved 2026-08-02.
Edge-dense ⇒ every PR under this umbrella goes through `/elidex-plan-review` before
implementation, and external review runs `/external-converge` from round 1.

All premises verified against `154bac3f`; every spec quote below was resolved directly with
`.claude/tools/webref`, not carried from a reviewer or from a code comment.

⚠ **Every `file:line` in this memo is a `154bac3f` coordinate and is evidence, not an
instruction.** The program has **six** prereq PRs (§8), and **two** of them move code: the
seam-3 split relocates a
block of
`inline/mod.rs`, and the dead-arm deletion removes a surface spanning **both** `pack/mod.rs` and
`inline/mod.rs` (§8 names it; it is not one contiguous range). Both were required before PR-1a,
and both have landed. **The other four relocate no code**, and each touch set is its own
plan-review's to determine, so this note makes no claim about them:
the **predicate** PR (§9) is the third required before PR-1a, the **min-content** and
**end-of-line white-space** PRs are required before PR-1b, and the **reconciler** PR before PR-1c. The memo does **not** re-anchor
after each: its coordinates exist to prove claims about the code as it stands today. **Each PR's
own plan-memo re-anchors against its actual base**, and §8's DoDs name behaviours and call sites
wherever they can — a DoD that still carries a coordinate carries a `154bac3f` one and inherits
this note. ⚠ Both `elidex-layout-block` prereqs have since landed — #508 (`7e256029`) and #511
(`22de3078`) — so the ranges named above no longer exist in those files; §5.2 and §8 carry the
landed state, and each later PR re-anchors against its own base (which includes `22de3078`).

**Citation convention**: every *spec* section number is written with its module
(`css-inline-3 §2.3`, `CSS 2 §9.4.2`). A bare `§N` is always this memo's own section. **Four**
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
section, §1.3, §3, §5.1, §5.2, §5.3, §6 cell 12e and §9. ⚠ **That sweep was keyed on the three
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
collide with are the eight of them a bare spec token actually lands on — §1, §2, §3.1, §5,
**§5.3**, §5.4, §6, §7. ⚠ **`§5.3` was missing from this list until R6, and it is the worst
omission the list could have**: it is this memo's *Program slicing* section **and**
css-inline-3 §5.3, the single most-cited spec section in the document, so the two readings are
both frequent and both plausible at the same token — exactly the class
[[feedback_enumerated-exemptions-leave-the-next-class-authoritative]] names, landing on the list
that cites it. R6 read every `§5.3` in the file: the large majority are this memo's own section,
and the thirteen bare ones that meant the spec are qualified in place. ⚠ An earlier drafting
listed nine, adding §8 and §10, which no swept site uses, and
omitting §3, §4, §9 and every subsection but §3.1 from the memo's own set — it matched neither
the sections the memo has nor the numbers the sweep found (round 26, gate 2). `§16.6.1`, also
swept, collides with nothing: it is a bare *use*, not a collision. CSS 2 is
verifiable with webref under the shortname **`CSS2`** (`webref heading CSS2 8.3`); the **seven** CSS 2
section↔title pairs this memo carries were checked that way — §8.3, §8.3.1, §9.2.1.1, §9.2.2.1,
§9.4.2, §9.4.3, §10.8.1. ⚠ **Seven, not the six an earlier drafting counted, which was itself a
correction of a five** (round 26, Axis 4): rev 33's own §1.2 edit added the CSS 2 §8.3.1
*Collapsing margins* pair and the count was not re-derived. §9's CSS 2 cite list runs to **ten**
sections because it counts every CSS 2 section this memo *cites*, produced by its own grep; the
other three, CSS 2 §9.5 (§3's css-inline-3 §2.1 row, in a cross-reference), CSS 2 §16.6.1 (§1.6)
and CSS 2 §10.2 (quoted since rev 59's A59, and by rev 60's M4 outcome), are cited without a title, so they carry no section↔title pair to
check — the two figures count different things and each is stated where it is produced.
⚠ **"Eight, the eighth being §16.6.1" until R22**, where the list's own grep returns nine: `§9.5`
entered with rev 55's own §3 row, so the enumeration was one short of its definition in the very
commit that made it so (`git show 46a48641~1:<memo> | grep -o 'CSS 2 §[0-9.]\+' | sort -u` →
eight; the same command on `46a48641` itself → nine). The
**seven** above is unaffected: seven titled pairs, three untitled cites (CSS 2 §10.2 from rev 59), ten sections.

## Terminator

⚠ **This program ran nineteen plan-review rounds with a reason-to-continue and no stopping rule.**
That is the mechanism that produced this length ([[feedback_loop-needs-a-terminator-not-a-reason-to-continue]]).
The rule, from here:

> **TERMINAL** = two consecutive rounds in which **no finding changes a §5.1 M-row *Decision* cell
> or a §6 cell**. At TERMINAL the plan is approved and implementation starts, whatever the
> IMP/MIN count stands at.

## ⚠⚠ DESIGN FREEZE (2026-09-20, rev 35) — the rule above is **RETIRED, not satisfied**

**§5.1's eight mechanism *Decision* columns and §6's cells — 49 of them at rev 34 (`9a17506d`) —
are frozen. Implementation starts.** The round-by-round ledger that tracked the rule — which
round reset the counter, and on what — is **not carried here**: it gated nothing the moment the
rule was retired, and `git log -p` on this file holds it. What a round *decided* survives in the
**Acceptance ledger** below, one row per rejected position or withdrawn claim.

**Why the rule was retired — three grounds, measured at the time rather than asserted.**

1. **The loop was generating its own resets.** Classified by origin, every counter reset after
   round 21 (which reached TERMINAL legitimately, and was reset by an *external* reviewer's three
   genuine findings) was either noise or a defect the previous revision's own fix had introduced:
   round 23 = two **wording** fixes; round 24 = one **`file:line`** fix inside text the previous
   revision had just written; round 25 = a dead branch **introduced by rev 32**; round 26 = an
   equality guard against an extended ordering **introduced by rev 33, the fix for round 25**;
   rev 34's gate = a smaller defect **introduced by rev 34**. Three consecutive self-introduced
   resets. Nothing made the counter likely to reach two.
2. **The rule keyed on a surface that grew every round, and accelerated.** The memo more than
   doubled between round 18 and this freeze. No series of figures is carried for that, and the
   one that used to stand here is the reason: a memo that states its own size invalidates the
   statement by writing it down, and the entry for the revision doing the writing was the one
   entry in the series that did not reproduce
   ([[feedback_document-landing-invalidates-its-own-measurements]]). The shape is the ground, not
   the numbers — requiring *zero* §6-cell-changing findings from a matrix this wide, while the
   prose arguing it gains another round's worth of argument every round, is a fixed-point search
   over an input that grows while it is searched.
3. **⚠ The decisive ground is detector mismatch, not cost.** Rounds 25 and 26 both found
   *code-level predicate* defects — a dead branch, and an equality test against an extended
   ordering. `cargo test` catches that class at implementation: this crate already carries five
   multi-line assertions
   (`grep -rnE 'lines\.len\(\), *[2-9]|line_count, *[2-9]' crates/layout/elidex-layout-block/src`
   → `inline/tests/inline_flow/{persist,vertical,transform,justify}.rs`), and a guard that never
   fires turns all of them red on the first run. Round 26's own Axis 2 said so and offered to
   downgrade its finding to IMP on that ground. Five agents over a memo this long were pointed at
   defects the compiler and the existing suite find in seconds.

**What is frozen**: the **Decision** column of §5.1's eight rows, and the **markup + expected
behaviour** of §6's cells — 49 at rev 34, **58** after the amendments recorded next, **62** at rev 60, **63** at rev 65 and **65** at rev 66 by `plan-xcheck.py` (R22's 10c, R26's 25b, rev 60's 10d and 17g, rev 65's 25c, rev 66's 3c and 25d — ledger A54, A59, A60, A70, A71, A72); rev 63 adds none (R2 adds
three cells; R3 changes three markups and adds none; R4 adds one cell and edits six; R5 adds none,
edits three and withdraws one cell's second arm; R6 adds one cell and edits two; R7 adds one cell
and edits three, and is the **first amendment to change a Decision column** — four of them; R8
adds one cell, **6i**, and edits **6e**, and changes no Decision column; R9 adds one cell, **24f**,
and edits **24e**, and changes no Decision column; R10 **removes 24f again** and edits **24e**,
and is the **second** amendment to change a Decision column — **M4** and **M7**, the two that own
the half of the flush replay it withdraws — so the figure is 56 across R9 and R10 together and
was **57** between them; R11 adds two cells, **12f** and **13d**, edits **12c**, and is the
**third** amendment to change a Decision column — **M1**, whose writing-mode source moves to the
IFC root with the Grounds bullet that assumed css-writing-modes-4 §3.2 is implemented here; R12
adds no cell, edits none and changes no Decision column — its subject is §3's `Full enum?`
column, which touches neither frozen surface, so the figure stays **58**. ⚠ The
figure and the parenthetical both stood at R8 until R10, which is
the same one-site-behind drift the amendment count above records).
⚠ **The command over-reports, and R7 is the instance**: splitting on `^\d+[a-z]?\. ` attributes
§6's trailing matter (the non-regression pair, the test-placement paragraph) to whichever cell is
**last**, so inserting a new last cell reports the previous one as changed. R7's run named
**24d**, whose own text is byte-identical as a **prefix** of the old segment — the check is that
prefix relation, and it is part of the basis rather than a caveat on it.
⚠ **Each round's edit set is stated as the diff
measures it, not as the round's own summary described it** (R5): R4's read "removes one cell's
third assertion and adds one cell", and the diff shows it also edited **6**, **6g**, **13b** and
**24c** — the same under-count the attestation disclaimer below carried. The basis is one command,
re-runnable: extract `## §6` from each revision, split it on `^\d+[a-z]?\. `, and compare the
per-cell texts. **Not frozen**
(still correctable without reopening anything): Grounds columns, citations, coordinates, §7–§10,
and the ledgers.

**⚠⚠ Reopened for M4 only — rev 60, 2026-09-22, by user decision** (ledger **A60**). The user
decided what cell 10c had left open: an inline box's content rect **encloses its descendants on
the inline axis** —
CSS 2 §10.2, "The content width of a non-replaced inline element's boxes is that of the rendered content within them (before any relative offset of children)" (`body CSS2 the-width-property`), and css-box-4 §2, whose content area
"contains its content—text, descendant boxes, an image or other replaced element content, etc." (`body css-box-4 box-model`) — a ground for the **inline** axis only: in the block axis css-inline-3 §5.3/§6.4 size an inline box from its own font and not its children, and `commit_aligned_entity_rects` gives every rect the line's `block_size` (`pack/mod.rs:488-494`). A freeze amendment to **one** Decision column, M4's, which
gains an *outcome* paragraph ahead of its rev-59 text, and to cells **10b** (containment re-worded
as class-wide), **10c** (asserts the outcome), **17c** (the start-edge requirement re-stated as the
registered deviation it is), **17d** (prose only) and **13c** (one sentence), plus two new
PR-1c cells, **10d** and **17g**. Every other M-row and cell stays frozen. The rev-60 draft went through a scoped
`/elidex-plan-review` (5 axes); freeze discipline 5 holds — the outcome is stated here, the
producer that achieves it is PR-1c's.

**DECIDED at rev 60 — the three questions the scoped plan-review was given** (ledger **A60**):

* **OQ-1, a box opened at a soft wrap: no rect on line N.** css-text-3 §5.5: "For soft wrap opportunities before the first or after the last character of a box, the break occurs immediately before/after the box (at its margin edge) rather than breaking the box between its content edge and the content"
  (`body css-text-3 line-break-details`), so
  the whole box is on line N+1 (cell 17c). A **forced** break inside a box does fragment it
  (css-inline-3 §2.1; css-break-3 §2 *box fragment*), so there it has a rect on both lines (cell
  17g). elidex's value on line N+1 is the start-edge deviation's, pinned by 17c as accepted.
* **OQ-2, M1's emit predicate: each pass evaluates it on the edges it resolves; no M1 change.**
  css-sizing-3 §5.2.1 rule 4: "For the min size properties, as well as for margins and paddings
  (and gutters), a cyclic percentage is resolved against zero for determining intrinsic size
  contributions", and the section's closing bullet accepts that layout then
  disagrees ("the contents might thus overflow or underflow the containing block"). The **used**
  value is non-zero whenever the percentage and the containing block's size are; the zero is for
  contributions only — so a percentage-only box has no marker
  and no shaping break in the intrinsic passes and has both in layout, and a later per-pass
  reader of marker presence is correct by that definition.
* **OQ-3, descendant margins: inside the rect, on the inline axis.** M4's outcome is the
  inline-axis hull of the box's own content-start and end-cursor points with its descendants'
  border boxes, so a descendant's margin between those points is enclosed, as the cursor span
  always did.

**⚠⚠ Reopened for M3's hang gate — rev 63, 2026-09-23, by user decision** (Codex R29-F3; ledger
**A63**). The user adopted Codex's reading of css-text-3 §4.1.2: a trailing collapsible space stays
line-final across inline-box markers whatever their edges, and stops being line-final only when
content follows it. **What then happens to the space is not decoration's**: css-text-3 §4.1.2's
end-of-line processing is a pre-existing engine defect, carved by a second user decision the same
day into the **end-of-line white-space prereq** (§8; ledger **A66**), and a third hands that
domain's outcomes to the per-PR plans — narrowed by a fourth to the **mechanism and the
non-`normal` `white-space` values** (ledger **A67**). M3 keeps two things: a marker does not make
a trailing space non-line-final, whatever its edges, and under `normal` step 3 then removes it. A freeze amendment to **M3**'s
Decision (that outcome paragraph, and the side-specific `hang = Some(0.0)` withdrawn) and,
consequentially, to the one clause of **M5**'s Decision that counted the hang gate among the
predicate's readers; to cells **16** (back to its (a)/(b) contrast), **3b** (its arm re-pinned: the
marker does not end line-finality), **14b** (an unobservable clause dropped), **16b** (one sentence) and **17c** (its
precondition re-derived); and to cell **3**'s count of M5's readings. **M4**'s Decision is touched too, but by §2's re-enumeration (ledger **A65**), not by
this decision: its list of §2's rows and its pairs (now 3×7 and 2×3). The 2026-09-20 attestation
table's **M3** and **M5** rows gain a pointer here. No cell id is added, and none is removed. The rev-63 draft went through a scoped `/elidex-plan-review`;
the mechanism is PR-1b's.

**⚠ Amended eleven times, by the external channel this freeze deliberately keeps and by the
cumulative design re-gate it does not silence either** (Codex R2, R3 then R4 on #515, 2026-09-20 —
revs 36, 37 and 38; R5 — rev 39, the re-gate plus a property sweep; R6 — rev 40, three of whose
four findings are defects in rev 39's own structural fix; R7 — rev 41, whose first finding is a
**re-raise of an R5 finding this memo never applied while its commit message said it had**;
R8 — rev 42, whose subject is a hand-off with nothing behind it; R9 — rev 43, three
findings, one of them refused; R10 — rev 44, which **reverses that refusal** on a measurement and withdraws the
cell it had been refused with; R11 — rev 45, three findings, the first of which
falsifies a **Grounds bullet** of §5.1 M1 and not merely a cell, and all three of which are
places this memo asserted a *spec conclusion* where the engine's behaviour was the question;
R12 — rev 47, the re-gate again, whose subject is §3's `Full enum?` column: it was never defined,
and the two rows its undefined reading had left false include **R11's own sweep miss** (ledger
**A34**–**A36**; rev 46 between them swept R11's count correction and opened no finding of its
own, so it is not a round);
⚠ the count read "seven" and the parenthetical stopped at R8 until
R10, R9 having extended the round disclaimer below and this sentence neither; the
freeze declares #515
unblocked, not the reviewer silenced, and the front matter keeps `/external-converge` on the
approval PR). **R2 — four P2 findings, two roots:**

* **R2-F3** — M3's hang gate was written against M5's **whole-box** `has_inline_axis_edge`, which
  is true at *both* markers of an asymmetric box, so an `InlineBoxEnd` that contributes no
  inline-end advance would zero a hang that is still line-final. The gate — and the shaping break
  beside it — become **side-specific** (M3, M5, §8's PR-1b DoD). §6 cell 16's fixture,
  `abc <span style="padding:1px"></span>`, is symmetric and produces the same output under either
  gate; it is replaced by a start-only / end-only **contrast pair** whose two arms differ from each
  other and one of which differs between the gates. Rejected position: ledger **A1**.
* **R2-F1 / R2-F2 / R2-F4** — one root: elidex has **no per-box layout-bounds model** (css-inline-3
  §5.3), and the slot that owns the gap, `#11-inline-root-inline-box`, had a narrower *stated*
  scope than the divergence the memo already discloses. The slot's subject is widened to the whole
  css-inline-3 §5.3 model with four enumerated facets (§5.3, §10 — ⚠ the pointer read "§5.3, §9, §10" until
  R5 and §9 mentions that slot **zero** times; both R4 slots already say §5.3, §10) — **five from
  R4**, and **narrowed again at R22** from "the whole model" to the model *as composition*, the
  `normal` branch's per-box bounds having acquired their own slot in between (§5.3) — M6's and M7's disclosures are corrected
  to name their facets, and three cells are added to pin what was unpinned: **13b** (PR-1c, the
  committed rect's block extent), **23b** (PR-1d, the vertical block-advance source) and **24c**
  (PR-1d, baseline composition on a mixed line). Rejected positions: ledger **A2**, **A3**.

**R3 — one P2, and the class it belongs to.** Cell 17c's fixture needed a second line that
css-text-3 §5.5 does not license: it contained no space, so the wrap came from the inline-box
boundary — from `#11-inline-item-boundary-soft-wrap`, the **pre-existing bug this memo already
owns as a slot** — and a PR-1c geometry test on it would have required the bug to survive. The
root action is not the cell but the **rule**, installed once in the §6 preamble with a
discriminator that is a measurement, plus the **sweep** it licenses, recorded on the slot in §5.3:
17c is re-fixtured, and 17 and 17f — fixture-less until then, which is how a boundary-dependent
fixture comes to be written at all — take 17d(b)'s markup, so the three PR-1c cells on a wrapping
box become three channels on one geometry. Three markups change and no expected behaviour does,
which is why this is an amendment to the frozen surface and not a correction beneath it. Rejected
positions: ledger **A4**, **A5**. ⚠ **The sweep it licensed was population-scoped and did not
reach the class**; R5's bullet carries what it missed and where the guarantee now lives.

**R4 — three findings, one of them a P1, and the P1 is a whole class rather than a sentence.**

* **R4-b (P1) — §6 cell 13's third assertion asserted a paint no code path produces.** It claimed
  the painted background-colour and border rects match the border box. For a **static** inline
  neither is ever emitted: `paint_non_sc` passes only a *block* child to `walk`
  (`builder/walk.rs:648-676`), `emit_background`/`emit_borders` have no non-test caller but
  `walk.rs:356`/`:366`, and `InlineFlowRun` carries only `Text` and `AtomicBox`. Measured on
  PR-1c's own output shape: 0 red display items for `display:inline`, 1 for `display:block`, 1 for
  `position:relative`. ⚠ **That measurement is a count, and R11 records what a count cannot
  reach**: on the `position:relative` arm PR-1c leaves the count at 1 and moves the rect, so the
  withdrawal here holds for the *static* arm only (ledger **A8** narrowed, **A33**, cell **13d**).
  The assertion is **removed** — cell 13 keeps (a) the real edges and (b) the
  20px displacement, and names the `LayoutBox`-fed readers that do exist. The gap is **pre-existing
  and engine-wide**, so it becomes a slot, `#11-inline-decoration-paint-path` (§5.3, §10), and not
  a render pass bolted onto a layout program. ⚠ **The fix is a sweep, not that cell**: the
  population is every site claiming a painted or user-visible delta (`grep -niE 'paint' <memo>`,
  each hit classified), and it reached §4.3, §5.1 M1, §5.2, §5.3's PR-1b and PR-1c bullets —
  including the PR-1b-before-PR-1c *ordering* argument, made **on paint** and remade on the
  readers that exist — §6, §7 and §8; §1.1's framing and §1.2's clause list are spec statements
  and stand. Rejected positions and the withdrawal itself: ledger **A6**, **A7**, **A8**, where
  the eleven inline `… until R4` markers this sweep left behind are retired, since eleven sites
  restating one withdrawal is one record.
  ⚠ **What replaces the motivation the paint claim carried**: the program's user-visible subject is
  (i) the **glyphs after the box moving**, since a line's `InlineFlowRun::Text` `inline_start` is
  what render paints from (⚠ on the renderer's identity-order path — R7-b, scoped once in §7), and (ii) the box's own rect at the readers that take
  `LayoutBox.border_box()` per entity rather than through the walker — `getBoundingClientRect` /
  `offsetLeft`, **hit-testing** (`elidex-layout/src/hit_test.rs:130-131` + `:165`) and **a11y bounds**
  (`elidex-a11y/src/tree.rs:121-125`). Hit-testing was measured, not assumed: with PR-1c's 10px
  padding real the point (12, 20) in the padding ring hits the span; with the padding zeroed, as
  today, it hits the `<p>`.
* **R4-a — rev 36's own "exact for a decoration-only line (one box, composition trivial)" is
  false.** css-text-3 §5.5 gives an inline box boundary no soft wrap opportunity, so a
  decoration-only line can carry two glyphless boxes, and css-inline-3 §5.3 gives each its own
  strut. The exactness claim is **deleted**: M7's promote is an approximation on every line. The
  case becomes facet **(e)** of `#11-inline-root-inline-box` (which therefore has five, not four)
  and is pinned by new cell **24d**, whose markup holds both line-heights at 10px so that the
  maximal ascent falls on one box and the maximal descent on another. The withdrawn exactness
  claim and the markup R4 first proposed are ledger **A9** and **A10**; ⚠ the composed figure this
  bullet stated is itself withdrawn at R5 (ledger **A14**).
* **R4-c — the `:200` gate's lift is inline-axis-only, and the residual is pre-existing.** Decided
  by measurement at `22de3078` rather than by argument, over the whole configuration space: the
  gate is blind to edges, so it is the measurability guard M5 calls it, and PR-1d's escape raises
  the inline-axis arm alone, making no case worse than today. Disposition therefore **slot +
  pin**, not a wider lift: `#11-inline-fontless-measurability-gate` (§5.3, §10), pinned as a
  second assertion on §6 cell 12d, which carries the measurement. Rejected position: ledger
  **A11**.

**R5 — the cumulative design re-gate and a property sweep. Three findings, each the *class* of a
defect a previous round patched at the site it was named at. No Decision column changes.**

* **The boundary-wrap class was declared closed twice and was not.** R3's sweep was scoped to the
  cells that **state** a second line, but a cell can depend on the item-boundary flush through
  its **width window** (15c) or through a second line the packer **discards** before anyone reads
  it (21's arm (b)). 15c's window is raised to its lower flip (ledger **A16**) and 21's arm (b)
  is withdrawn to `#11-inline-item-boundary-soft-wrap` rather than re-fixtured, measurement
  saying no licensed break can open a line with a collapsible space on it (**A13**). The root
  action is again not the cells: §8's PR-1b DoD gains a **suite-level break invariant** over
  every §6 fixture.
* **Cell 3b carried a copy of cell 16's pre-R2 markup and an assertion false of the live one**
  (**A12**); it states its own markups now and the §6 preamble carries the rule a copy obeys. Two
  smaller members travelled with it: cell 6 claimed cell 14b asserts "the same markup" when 14b's
  differs, and 14b pointed at a no-edge contrast R2 removed from cell 16.
* **Two hand-computed spec figures were wrong in the direction discipline #1 predicts** — 13b's
  border area and 24d's composition (**A14**, **A15**) — and both are removed rather than
  recomputed, neither being a determinate spec quantity. That is the general treatment: rule 1
  honoured rather than restated, every figure the amendments added either deleted for the
  relation it stood for or converted to one its cell's test computes.

**R6 — four findings, three of them defects in what R5 added, and the common cause is that R5's
structural fix was specified in prose and never executed.** The governing rule this amendment
installs, in §8 beside the invariant itself: **a gate is landed by running it, not by writing
it** — the PR that lands the helper routes every §6 cell through it in one execution, in both
regimes, and a green matrix is the acceptance criterion.

* **R6-a — the invariant as written failed nearly every fixture.** "Every offset at which
  `flush_line` is entered" includes the terminal `FlushReason::LastLine` that `finish()` enters
  for any occupied line, at an end-of-text offset `find_break_opportunities` filters out by
  construction. The recorded set becomes the `SoftWrap` and `Forced` entries, with the three
  `FlushReason` variants named so the exclusion is checkable; `Forced`'s licence is the mandatory
  half of the same `find_break_opportunities` result and not a second allowance. Rejected
  position: ledger **A20**.
* **R6-b — cell 15c could not pass the invariant it inherits, at any `W` in its own window.**
  R5 raised 15c's lower bound to `measure_width("aaaab")` (ledger **A16**) and thereby repaired
  the **pre**-PR-1b regime only: with the markers' 40px of advance, `b` overruns `W` throughout
  the raised window, so the fixture breaks at an offset `find_break_opportunities("aaaab")`
  licenses nowhere. **A fixture has two regimes and a repair that reads one is half a repair.**
  The cell drops its trailing `b` — it asserted nothing about where `b` lands — leaving a text
  with no licensed break and no realised one; the window widens rather than narrows, and the
  discriminating upper bound is untouched. The alternative, re-fixturing behind a space so the
  break lands at a licensed offset, is rejected on css-text-3 §5.5 rather than on measurement
  (**A19**), and the raised window itself is **A18**. ⚠ **No exemption list was opened**: an
  "exempt the cells that pin an accepted divergence" carve-out is the enumerated-exemption trap
  [[feedback_enumerated-exemptions-leave-the-next-class-authoritative]] names, and it would leave
  the invariant total in name only.
* **R6-c — the memo's own verification commands name files that are not on `main`.** R5 made
  `plan-xcheck.py` the stated mechanism for §3's coverage map, §5.3's defer count and §10's
  routing, and both checkers are on this branch and on no other. The approval PR, whose DoD is
  one file, would therefore land a document citing commands the repository cannot run. **The
  tooling PR is ordered before or with the approval PR** (§8's 順序 block, §9's task); the diff
  stays one file, the remedy being the order and not a bundle. Rejected position: **A21**.
  ⚠ Discharged 2026-09-22: the checkers landed first, as #518 (`4394af4c`; §8). The
  sweep that finding asks for found no second instance: every other path the memo cites as
  runnable — `.claude/tools/webref`, `.claude/tools/layout-box-reader-trip-wire.sh` and its
  `.tsv` allowlist, `.claude/skills/elidex-plan-review/preflight.py`, `scripts/trip-wires.sh`,
  `.github/workflows/ci.yml` — is on `origin/main` today, and the one remaining external path,
  `~/.claude/hooks/`, is a user-level path outside the repository by design.
* **R6-d — the PR-1c cell *set* could not discriminate `LayoutBox.margin`.** Each cell was fine;
  the set carried padding only, with border reached solely by the end-to-end clause and margin
  by a PR-1b cursor cell. New cell **13c** asserts three pairwise-distinct non-zero values, so a
  swap between fields fails it and not only a drop. The same set-level read over **M5**'s
  disjunction found the margin term undiscriminated there too, and cell **3** gains the markup
  that pins it — M5 having one derivation site, one reading suffices. §8's PR-1c DoD carries both
  arguments at set level, which is where the defect was.

**R7 — three findings, and the round exists because the first of them was reported applied and
was not.** The governing rule this amendment installs: **a round's record is verified against the
artifact before it is written, not derived from what the round set out to do.** R7-a is Codex R5's
second P2, raised on #515 and answered in public as applied while rev 39 carried nothing about it;
what rev 39 *did* carry under the R5 heading was the re-gate's three findings plus R5's **first**
P2 (cell 21's arm (b), ledger A13), so the heading looked complete and the drop had no reader. The
audit this round ran — every review thread on #515, resolved included, `totalCount` asserted
against the fetched count, each finding's fix located **in this file** by grep or section — is the
discharge, and it found exactly one NOT APPLIED. It is recorded here rather than as a rule nobody
executes, because a rule is the thing that failed.

* **R7-a — an open box's metrics were not re-applied after a wrap, and this is a *Decision*
  change, the first the freeze has taken.** M6 applied `block_advance` and M7 recorded its
  tentative only where `InlineBoxStart` is consumed, while `flush_line`'s reset zeroes
  `current_line_height` (`:433`) and clears the tentative and M4's flush hook restored neither —
  so an outer `line-height:100px` box wrapping around an inner `line-height:10px` one contributed
  to its **first** fragment alone, against css-inline-3 §5.3 and §2.2 step 3, which compose
  **each** line from every participating box. **The fix is the mechanism and it is placed by
  ownership**: M4's hook already walks the open-box stack, so the walk is the **trigger** and each
  mechanism keeps its own field — one `note_line_occupancy(0.0, block_advance, None, false,
  BoxEdgeOnly)` per open entry (M3's owner, M6's value) and — until R10 withdrew it — one
  tentative re-record (M7's), run at
  the **bottom** of `flush_line` after the reset rather than at the top with the emit and the
  rebase. ⚠ **M3's single-owner discipline is resolved *through* the owner, not around it**: the
  hook calls `note_line_occupancy` instead of assigning `current_line_height`, so that field still
  has exactly one writer and M3 is amended only in its caller count — "two callers" becomes
  **three**. The alternative, M4 writing the field, is ledger **A22**; re-applying the *inline*
  advance as well is **A23**. `contributes_content` is `false` in the replay — a middle fragment of
  an open box carries neither inline-axis edge, css-break-3 §5.4 attributing a broken edge to one
  side — so the replay restores height and touches **existence** not at all, leaving
  PR-1d's flip set as §8 states it. New cell **24e**, a wrapping unequal-line-height fixture whose
  wrap is a **space**, so it satisfies §6's standing rule by its text; its window drops the
  marker's advance for the reason 15d's drops its 20, keeping `line_count` 2 in both regimes. The
  cell's own coverage limit is stated in it: the **baseline** half of the replay has no oracle on
  that fixture, `first_baseline` being per-IFC and already set from line 1's text — **and, R10
  measured, none on any other fixture either**, which is why that half is no longer PR-1d's
  (ledger **A28**, **A30**; slot in §5.3).
* **R7-b — the bidi reorder discards the gap at paint, and the disposition is decided on a
  measurement rather than on the precedent that predicts it.** `builder/inline_flow.rs:154-190`
  (at `22de3078`) drops every run's baked `inline_start` on a non-identity line, starts at
  `min(inline_start)` (`:169-173`) and advances by shaped widths. Verified by a display-list probe
  rather than by reading: two RTL runs, baked 100px apart and with the gap closed, emit
  **identical** glyph positions, against the crate's own `converged_ltr_identity_no_reorder`
  control where an identity line paints each run at its own baked position. **Routed to
  `#11-bidi-full-uba-fidelity`** (§9) — an existing slot of another program, so no deferral of
  this one's — on three grounds of which the measurement is the first: the branch **already** owns
  this class and names that slot for it in its own comment (`:160-168`); a renderer mechanism is
  the crate boundary A6 keeps; and the loss is reachable today with no marker involved, this
  program adding reach and not the defect. §6 cell **12b** carries the pin and asserts no painted
  position. ⚠ **The scope is stated once, in §7's painted-output bullet, over the property** —
  every site of this memo saying the glyphs after the box move states the identity-order half —
  and the four sites it reaches (the freeze's own user-visible-subject clause, §4.3, cell 13(b),
  §7's reader item 1) carry a pointer, not a copy, per rule 2. Ledger **A24**.
* **R7-c — `clientWidth`/`clientHeight` must stay zero for an inline box, and the fix is the
  class.** cssom-view-1 §6 step 1 is *identical* across all four `client*` members (`body
  cssom-view-1 dom-element-clienttop`, re-verified this round), and §3's CSSOM row and §9's
  predicate bullet both said so — while a PR-1c cell listed `clientWidth`/`clientHeight` among the
  consumers of the newly real edges and a second site had `clientHeight` inheriting the produced
  rect, each carving out only `clientTop`/`clientLeft`. **Swept by the property, not by the two
  sites named**: the population is every line naming any of the four members **or** the family
  token — `grep -niE 'client(Top|Left|Width|Height)|client\*' <memo>`, **34** lines on the pre-R7
  revision `49b702b3`, each read and classified by whether it makes a member a *consumer* of this
  program's geometry rather than by which token it spells. ⚠ The narrower grep on the member names
  alone returns **23** and drops every site written `client*` — including §8's requirement 1, the
  one place the rule is already stated totally — which is why the population is the union
  ([[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]]).
  **Five sites** needed work, two of them the pair the finding names. Cell 13's readback list and
  cell 13b's inheritance list are corrected to `getClientRects()` / `getBoundingClientRect()`.
  §5.3's PR-1c ordering argument stated the rule over two members and now states it over all four,
  since M4's padding also **widens** the pre-existing `clientWidth`/`clientHeight` violation and
  does not only open a new `clientTop`/`clientLeft` one. §8's end-to-end clause — the one geometry
  assertion in this memo that could have pinned the pre-prerequisite behaviour — now asserts all
  four, and §5.2's `elidex-shell` row, which describes that same clause, follows it; that fifth
  site is the finding's own lesson landing, being a **restatement** of a site the finding named.
  The rest are correct as they stand — §9 and §8's requirement 1 already say "all four", and §7's
  two field-read sites name `clientTop`/`clientLeft` because that is the shape of the *code* there
  (`lb.border.*`) and not an enumeration of the rule. Ledger **A25**.

* **R8-b — a disclaimer that handed a case to a section holding no entry for it, and the case is
  reachable.** §6 cell 6e disclaimed correct block-in-inline layout "pre-existing — §9", and §9
  carried nothing for it: its only occurrence of *Anonymous block* is the CSS 2 §9.2.1.1
  **citation-label** correction, not a disposition. The reachability was measured, not argued:
  with the **outer** inline decorated, the collector's recursion arm takes the block child (no
  block-level filter precedes it) and the block's text enters the enclosing IFC as an ordinary
  run, while M1's predicate — a function of the span's own style — holds, so the markers bracket
  it. Option (c), "the predicate does not reach the case", is therefore refuted rather than
  declined. **Disposition**: a real slot, `#11-block-in-inline-anonymous-block-split` (§9, §10),
  **pre-existing** class so PR-1a's own count is untouched; cell **6e** retargeted; new cell
  **6i** pins the outer half against the subtree-gated predicate `has_direct_block_child` already
  makes available in the same file. Ordering behind the split is rejected — ledger **A26**.
  ⚠ The same sweep found one further hand-off with nothing behind it — §8 requirement 5's "§9
  hands the `is_monolithic` consumers to the predicate PR", where the consumers §9 books are the
  `client*` members and M1's emit test and not that one — corrected in place. §9's `<br>`/`<wbr>` bullet already records this defect *shape*
  for rev 32, which is the ground for treating it as a class rather than a site.

⚠ **The attestation table below predates the amendments** and is left as the record of what that
pass checked: it did not see cells 13b / 23b / 24c, and M3's, M5's, M6's and M7's rows were
consistent under the whole-box gate R2 replaces. R3 changes no Decision column and no expected
behaviour, so it does not disturb what that pass asserted; M4's row names 17, 17c and 17f and
their subjects are unchanged. R4 and R5 **do** touch the pass's surface and are flagged rather
than folded in, each **measured against its predecessor rather than summarised**. **R4**: cell
**24d** is new (so M7's row has a second cell beside 24c) and **6**, **6g**, **12d**, **13**,
**13b** and **24c** are edited — 13 losing an assertion, 12d gaining one. ⚠ **This disclaimer
named only 13, 24d and 12d until R5** where the diff names six edits; a disclaimer that
under-states its reach is worse than none, being read as an enumeration. **R5**: **fifteen** cells
change, **three** of them on the frozen surface — **3b**'s markups, **15c**'s width window,
**21**'s second arm withdrawn — and **twelve** beneath it with markup and expected behaviour
unchanged: **6**, **14b** (a false markup identity, a stale cross-cell pointer), **13**, **6g**
(a withdrawn-claim marker retired to the ledger), **13b**, **24d** (two unlicensed spec figures),
**14**, **16** (the whole-box name at a boundary gate, a mis-distributed summary quantity),
**17**, **17c**, **17d** (literal arithmetic replaced by the relation the test computes) and
**20** (css-text-3 §5.5's out-of-flow half). **R6**: cell **13c** is **new** (so M4's row below, left as the pass wrote it, is short one cell,
and the missing one is the row's only non-padding-only cell) and **3** and **15c** are edited —
3 gaining the markup it never had, 15c re-fixtured; the matrix goes **53 → 54**. Measured by the
freeze's own command, not summarised: extract `## §6` from `8804d4f5` and from this revision,
split on `^\d+[a-z]?\. ` and compare the per-cell texts — added `13c`, changed `3` and `15c`,
removed none. ⚠ **No M-row *Decision* column changes at R4, R5 or R6** — verified
by extracting each row's Decision cell (split on unescaped `|`, M1's pipe escaped since rev 34)
and comparing — but at R4 **M1's** Grounds changed as well as M7's, which this disclaimer also
did not say; at R5 the Grounds changes are M4's and M7's, and at R6 no Grounds column changes
either, the R6 work landing in §6, §8 and §9. ⚠⚠ **R7 breaks that run**: the same extraction
reports **four** Decision changes — **M3**, **M4**, **M6** and **M7** — and **no** Grounds change
at any row. That is the shape R7-a required (a mechanism placed by ownership across four rows,
not a note), and the freeze anticipates exactly this: the external channel it keeps is allowed to
amend the frozen surface, and an amendment that touched only §6 would have left the mechanism
unowned. **R7** is therefore the round this disclaimer cannot be read past: the pass's M3, M4, M6
and M7 rows are the pre-R7 ones, and cell **24e** is not in any row's list above. **R8** adds
cell **6i** and edits **6e**, both M1's; **6i** is likewise not in any row's list above, and the
round changes no Decision and no Grounds column. The matrix goes **55 → 56**. **R9** adds
**24f** — M4's and M7's, like 24e, and in no row's list above — and edits **24e**; it changes no
Decision and no Grounds column either, its three findings landing in §8's PR-1b and PR-1d items,
§5.2's `pack/mod.rs` row, §3, §7, §9 and §10. The matrix goes **56 → 57**. **R10 removes
24f again** — the round's single finding is that 24f does not discriminate, and the measurement
generalises to every fixture (ledger **A30**) — and edits **24e**; the matrix goes **57 → 56**.
Unlike R8 and R9 it **does** change Decision columns, **two** of them, **M4** and **M7**, which
between them own the withdrawn half of the flush replay; no Grounds column changes, and the rest
of the round lands in §5.3 (a new pre-existing-class slot and the pre-existing list that had
stopped being the population), §8's PR-1d cell list, §10 and the R7-a amendment bullet.
**R11** adds **12f** (M1's, PR-1b) and **13d** (M4's, PR-1c) and edits **12c**; the matrix goes
**56 → 58**. It changes **one** Decision column, **M1** — the payload's writing mode moves from
the decorated inline's own `style` to the IFC root's — and, unlike R7 and R10, it also changes a
**Grounds** column, M1's, because the withdrawn ground and the changed Decision are the same
finding: the bullet argued the source could not matter *from* a rule elidex does not implement.
The rest of the round lands in §3 (two rows, one of them `✓ → ✗`), §5.3 (a new
pre-existing-class slot, the pre-existing list, and a scope narrowing on
`#11-inline-item-boundary-soft-wrap`), §7's painted-output bullet, §8's PR-1b suite invariant and
both PRs' cell lists, §8's closing paragraph, §10 and ledger rows **A31**–**A33** (with **A8**
narrowed rather than deleted).

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
| M3 | `LineOccupancy`; `note_line_occupancy`; guard `≥ Content`; `finish()` `> Empty`; the reset; `hang: Option<f32>` | M4, M5, M6, M7; cells 3b, 14b, 15, 15b, 15d, 16, 16b | consistent (as of this pass; M3's hang gate was reopened at rev 63, ledger **A63**) |
| M4 | the open-box stack; unconditional `End` push; flush-time partial emit and rebase | cells 6, 10b, 13, 14c, 15c, 17, 17b, 17c, 17d, 17f, 21 | consistent (as of this pass; M4 was reopened at rev 60, ledger **A60**) |
| M5 | `has_inline_axis_edge`; the PR-1d substitution; the `:200` gate escape | M3's hang and shaping break; cells 3b, 5, 6, 12d, 14/14b, 16, 21, 24, 24b | consistent (as of this pass; M3's hang no longer reads M5 since rev 63, ledger **A63**) |
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
   apparatus; measurement lives in tests and CI.* ⚠ **Honoured at R5, not restated**: every
   numeric figure the amendment rounds added is either deleted for the relation it stood for or
   converted to a relation the owning cell's test computes, and the two that were **wrong** —
   ledger A14, A15 — are the rule's own prediction landing.
2. **No new inline `⚠ An earlier drafting …` blocks.** A rejected position gets one line in the
   **Acceptance ledger** (the section below this block) and at most a pointer from the cell.
   Markers that predate this rule stay — deleting them would reopen what they closed — but the
   accretion stops here. ⚠ **The ledger did not exist until R5**, though this rule routed to it
   from rev 35: the string occurred once in the whole file, in this sentence. Three rounds of
   rejected positions and eleven inline `… until R4` markers accumulated in the meantime, and
   they are rows A1–A11 of it.
3. **No further whole-umbrella review.** Each PR gets a narrow plan-review of its own slice, the
   normal pre-push gate, and external review.
4. **The approval PR [#515](https://github.com/send/elidex/pull/515) is unblocked by this
   declaration**, not by a renewed TERMINAL.
5. **The umbrella pins outcomes; each per-PR plan-review owns mechanism** (R26; ledger **A59**).
   Ledger **A44** moved *mechanism* — where and how the code achieves a result — out of this memo,
   and **A48** stopped it writing requirements for the work it hands off; neither licenses handing
   off an **outcome**. A §6 cell's expected result, which PR owns a behaviour, and which spec rule
   governs a case are this memo's to state, because a cell with no expected result is not an
   acceptance criterion and a green matrix is one (R6's rule above). "Its plan-review's" is
   therefore a valid ending only for a sentence about *how*.
   ⚠ **One scoped exception, by user decision of 2026-09-23** (ledger **A67**), and it is
   narrower than it first read: for end-of-line white-space processing this umbrella hands over
   **(a)** the mechanism and **(b)** the outcomes for the `white-space` values other than
   `normal` — `pre-wrap`'s hang and what css-text-3 §8.2 lets block it, `pre`'s preserved spaces,
   and `break-spaces` — to the end-of-line white-space prereq's and PR-1b's own plan-memos and
   plan-reviews, along with the cells that would assert them. **The `normal` outcome stays this
   memo's, under item 5's main rule**: css-text-3 §4.1.2 step 3 removes a trailing collapsible
   space, which fixes cell 16's and 16b's expected results without choosing a mechanism. The
   ground for the hand-off is measured: the umbrella mis-specified the other values in three
   consecutive revisions (A63, A66, A67), each review finding its defects in the previous
   revision's own new text, and each of those outcomes turns on a mechanism the per-PR plan
   chooses. **The exception is those two things and nothing else.**

**What the growth was, and what it was not — the argument, without the arithmetic that rots.**
Across the window the loop was audited over, the two sections anchored **outside** the memo
barely moved: the spec-anchored §1 was byte-frozen and the code-anchored §4 shifted by a few
characters. The memo grew anyway, and the largest single recipient of that growth was §9 — the
section describing what the program does *not* do. But the design was not static either, and a
reviewer's claim that it was is refuted the same way: **M1's** and **M4's** *Decision* cells grew
over that window — the predicate widening and the carrier withdrawal, which is exactly where
rounds 16–19's CRITs were — while every other M-row's *Decision* column stood byte-identical
across it. So the loop was **neither** pure prose churn **nor** open-ended design drift; the
retired rule keyed on the one signal that separates them, and it was retired for the three
grounds above and not because that signal was the wrong one.

**Landing state is written in §5.2, never into a Decision cell**: a Decision cell that describes
a since-landed prereq in the present tense is *frozen*, not stale, and §5.2 carries the state —
otherwise every landing would edit the frozen surface.

**Review history — including which revision decided what, and why each earlier reading failed —
lives in `project_line-box-decorated-inline-content.md`, not here.** A past-tense ledger restates
the normative decisions and then drifts from them. What survives in this file is the *ground* for
each decision, stated affirmatively, plus an explicit refutation of the competing readings, so a
rejected one is closed on its merits rather than by precedent.

## Acceptance ledger

Freeze rule 2 routes a rejected position and a withdrawn claim **here**, one line each, so a cell
carries its decision and not its history. ⚠ **The section did not exist until R5** though rule 2
named it from rev 35: measured, the string `acceptance ledger` occurred exactly **once** in the
file, inside the rule that routes to it, while the amendment bullets grew six *Rejected position*
records and the body grew **eleven** inline `… until R4` markers — all eleven the same withdrawal
restated. Rows are append-only, and one decision is one row however many sites it reached.

| # | Round | Rejected position / withdrawn claim | Ground |
|---|---|---|---|
| A1 | R2-F3 | cell 16's symmetric fixture, `abc <span style="padding:1px"></span>`, as a gate test | it produces the same output under the whole-box and the side-specific gate, so it cannot separate them |
| A2 | R2-F1/F2/F4 | building the css-inline-3 §5.3 layout-bounds model inside this program | it changes the height of every line box in the engine; `#11-inline-root-inline-box` owns it, in five facets |
| A3 | R2-F1/F2/F4 | patching M4 / M6 / M7 into local coherence against a model that does not exist | it would bury the divergences cells 13b, 23, 23b, 24c and 24d pin as accepted |
| A4 | R3 | 17c's fixture `<p style="width:100px">aaaaaaaaaa<span style="padding:5px">bbbb</span></p>` | its second line came from the inline-box boundary, which css-text-3 §5.5 licenses nowhere, so a geometry test on it would require `#11-inline-item-boundary-soft-wrap` to survive |
| A5 | R3 | a **count-only** tagless control for that class | measured, `<p style="width:60px">aaaa<span style="padding:10px">bbb ccc</span></p>` gives two lines tagged *and* tagless while splitting at different points, so the control must compare split **points** |
| A6 | R4-b | adding an `elidex-render` pass or member to this program | it crosses the crate boundary CLAUDE.md's Layering mandate keeps; `#11-inline-decoration-paint-path` |
| A7 | R4-b | deleting cell 13's paint assertion alone | the same claim stood at thirteen other sites — the fix is the sweep, not the cell |
| A8 | R4-b | **that a static inline's background and border are painted at all**, and that PR-1c's largest delta is painted output | measured at `22de3078`: `paint_non_sc` passes only a *block* child to `walk` (`builder/walk.rs:648-676`), `emit_background` / `emit_borders` have no non-test caller but `walk.rs:356` / `:366`, `InlineFlowRun` carries only `Text` and `AtomicBox`, and a `background: red` span yields **0** red display items under `display:inline` against 1 under `display:block` and 1 under `position:relative`. One withdrawal at eleven sites, so one row; the inline markers those sites carried are retired here (R5). ⚠ **Narrowed at R11, not deleted**: the withdrawal stands for the **static** subset, which the same probe re-confirms with the edges real (0 before, 0 after). The measurement is a **count**, and a count does not reach the `position:relative` arm, where PR-1c moves the rect — and, on a span that also carries a `border`, moves the count as well (1 → 5), the probe's background-only span being the one member of the family where the count holds still. A33 |
| A9 | R4-a | that M7's promote is *exact* on a decoration-only line ("one box, composition trivial") | css-text-3 §5.5 gives an inline box boundary no soft wrap opportunity, so such a line can carry two glyphless boxes and css-inline-3 §5.3 gives each its own strut; the promote is an approximation on **every** line |
| A10 | R4-a | R4's markup for cell 24d, `font-size:100px;line-height:10px` beside `font-size:10px;line-height:100px` | measured against the test font, the second box's `A′` **and** `D′` both exceed the first's, so one box dominates and M6's `max(line-height)` already gives the composition |
| A11 | R4-c | widening `inline/mod.rs:200`'s escape to clause 1 here | that return also runs `clear_inline_flows` and `remove_one::<ColumnFlowSlice>`, so it would commit an IFC no run can measure |
| A12 | R5 | cell 3b carrying its edge pair "on cell 14's and cell 16's markups" with an inline copy of each | the copy of cell 16 was the **pre-R2** fixture and survived R2's re-fixturing, so the cell's instruction and its own quotation named different markups and its "the hang is zeroed" was true of the replaced one and false of the live one |
| A13 | R5 | re-fixturing cell 21's arm (b) so its second line comes from a licensed opportunity | not constructible: a UAX #14 opportunity lies inside a run and leaves the space at the **end** of the preceding segment, so no line a licensed break opens can begin with a collapsible space. Withdrawn to `#11-inline-item-boundary-soft-wrap` instead |
| A14 | R5 | stating the css-inline-3 §5.3 / §2.2 composition for cell 24d's line as a figure (it read 41.201) | it omitted the `<p>`'s **root inline box**, which css-inline-3 §2.2 step 2 gives its own bullet and which contributes a `D′` deeper than either span's; and css-inline-3 §5.3 makes that box's half-leading optional under `line-height: normal`, so a conforming UA has a range and not a number |
| A15 | R5 | stating the border area cssom-view-1 §6 asks for on cell 13b as a figure (it read 14px, then ≈22px) | 14px is the span's `line-height` plus padding, the derivation css-inline-3 §5.3's "the layout bounds need not correspond to the box's edges" and css-inline-3 §6.4's "the `line-height` has no impact on the size of an inline box, it only affects its contribution to the logical height of its line box" both exclude; ≈22px is that same **border area** computed from the font instead — the first available font's ascent + descent **plus cell 13b's 2px padding on each side** — which css-inline-3 §6.4 offers only as an example a UA **may** use while saying "this specification does not specify how". ⚠ **The gloss read "≈22px is the first available font's ascent + descent" until R22**, and A + D alone is not ≈22: at the test font's own metrics (Arial, `unitsPerEm` 2048, `hhea` ascender 1854 and descender −434, at 16px — the same font and scale §6 cell 15d's width window is derived from) A = 14.484375 and D = −3.390625, which sum in magnitude to **17.875**; 17.875 + 2 × 2px = **21.875**. The row is a withdrawn-claim row and nothing is built on it; the gloss is corrected so it cannot later be cited as a font-metric fact |
| A17 | round 26 | "the only markup that gains a line does so through the item-boundary flush `#11-inline-item-boundary-soft-wrap` records" (§5.3, PR-1b) | a universal over *markup* that the same bullet refutes two clauses earlier: the advance makes text reach the guard at a genuine UAX #14 opportunity earlier than it did, so both paths gain lines and the slot owns only the second. The universal that survives is the one over **cells** |
| A16 | R5 | cell 15c's width window opening at `measure_width("aaaa")` | its lower part sits below `measure_width("aaaab")`, where today's own layout already breaks at an item boundary `find_break_opportunities` licenses nowhere, so the cell's pre-PR baseline was bug-dependent and the §6 preamble's tagless control fails on it |
| A18 | R6-b | A16's replacement — cell 15c keeping `…<span style="padding:20px"></span>b</p>` with the window raised to `measure_width("aaaab")` | the raise licenses the **pre**-PR-1b baseline only: from PR-1b the two markers add 40px, so `b` overruns `W` at every `W` in the window (`W < measure_width("aaaa") + 40 < measure_width("aaaa") + 40 + measure_width("b")`) and the fixture breaks where `find_break_opportunities("aaaab")` — empty — licenses nothing. A fixture has two regimes; a repair that reads one is half a repair |
| A19 | R6-b | re-fixturing 15c as `aaaa ` + the box + `b`, so the flush lands at the licensed offset the space opens | it satisfies the invariant but decides, silently, a question css-text-3 §5.5 leaves open: that bullet puts the break at the box's margin edge "for soft wrap opportunities before the first or after the last character **of a box**", and an empty decorated inline has no characters, so which side of it the break falls on is undetermined — the cell would pin a contested answer to the very question it exists to assert |
| A20 | R6-a | the suite break invariant recording "every offset at which `flush_line` is entered" | `finish()` enters `flush_line(FlushReason::LastLine)` for any occupied line, at the end-of-text offset `find_break_opportunities` filters out by construction, so the predicate reports the ordinary end of a paragraph as an unlicensed break and fails every fixture whose last line carries content |
| A21 | R6-c | "neither PR waits on the other" for the approval and tooling PRs | true while the memo's claims did not depend on the checkers, and R5 ended that: `plan-xcheck.py` became the stated verification mechanism for §3's coverage map, §5.3's defer count and §10's routing, and neither checker is on `origin/main`, so a memo-only approval PR lands re-runnable commands naming files the repository does not contain |
| A22 | R7-a | M4's flush hook assigning `current_line_height` itself when it re-applies an open box's per-line contribution | M3 makes `note_line_occupancy` the field's single writer and argues its whole three-state design from write sites; a second writer is the shape that row refuses, and the hook calling the owner costs nothing — so M3 is amended in its caller count alone |
| A23 | R7-a | replaying the marker's **inline** advance along with its block one on a continuation line | M4's flush-time rebase already carries the cursor and the start edge was consumed on the earlier line — cell 17c's own rule — so re-adding it is exactly the double-count that rule exists to prevent; the replay passes `inline_advance = 0.0` |
| A24 | R7-b | carrying the markers' gaps through the renderer's bidi reorder path inside this program | it is an `elidex-render` mechanism, the crate boundary A6 already keeps; and measured — two RTL runs baked 100px apart emit the same glyph positions as with the gap closed, while the identity control paints each at its own position — the loss is the non-identity branch's own pre-existing shape, which discards layout's baked justify offsets the same way and names `#11-bidi-full-uba-fidelity` in its comment. Routed there, pinned at cell 12b |
| A25 | R7-c | listing `clientWidth`/`clientHeight` among the consumers of PR-1c's real edges while carving out `clientTop`/`clientLeft` | cssom-view-1 §6 step 1 is identical across all four members, so after the predicate prereq an inline reports zero on all four and a two-member carve-out is the enumerated-exemption shape the front matter refuses; the readback claim is `getClientRects()` / `getBoundingClientRect()` and nothing else |
| A26 | R8-b | ordering the decorated-outer block-in-inline case behind the CSS 2 §9.2.1.1 anonymous-block split, the reviewer's alternative to a disposition | the split is a `block/children/stack.rs` box-tree change that re-parents the content out of this IFC, not an inline-module one; ordering this umbrella behind it blocks every PR on an unrelated unimplemented feature and changes nothing about whether the markers are right for the tree the engine builds. The residual is owned instead, by `#11-block-in-inline-anonymous-block-split`, and pinned by §6 cell 6i |
| A27 | R9-a | §8's PR-1d gate requiring `note_line_occupancy` to have exactly **two** call sites, and the reviewer's alternative of counting only *occupancy-raising* ones | R7-a's replay makes three, so the two-count fails a correct implementation and an implementer who satisfies it drops the replay and regresses continuation-line metrics; and the narrowed predicate excludes nothing, `BoxEdgeOnly` being a rung above the `Empty` the reset just wrote. The item names the three sites instead, so a fourth still fails it, and the property the flip set actually needs — the replay leaving `any_rendered_content` alone — is checked by naming the site |
| A28 | R9-b | that the baseline half of the flush replay is **unreachable**, so PR-1d should mandate only the height half | measured: the unreachability rests on the soft-wrap route alone. `force_break` is reached from a preserved segment break under `white-space: pre*` as well as from `<br>` (`pack/mod.rs:775-784`), and `find_break_opportunities` returns one `Mandatory` entry for `"x\n "` against none for `"x\n"`, so a decorated box stays open across an interior forced break whose continuation line carries no rendered text. A new §6 cell, **24f**, discriminates the half on that route; the DoD keeps both halves. ⚠ **Superseded at R10 — the position is accepted and this verdict is wrong in its conclusion, though not in the clause that refuted the reviewer's third ground**: `force_break` is indeed reached without a `<br>`, and `"x\n "` does return one `Mandatory` entry — but that string is not what the pipeline feeds the packer. Under `pre-line` the collapse drops the space after a preserved break, so the collector yields `"x\n"`, whose only break is the filtered end-of-text one; the R9 measurement ran the helper on a **literal** and so had a different subject from the claim. A30 carries the general result |
| A29 | R9-c | keeping the observer-facing content size at zero for inline boxes inside this program, or ordering PR-1c behind that correction | the reader is `elidex-js`'s `size_fn` over `elidex-plugin`'s `content_rect_local`, outside every crate this program touches (A6's boundary, A24's shape); and measured, an ordinary `<p><span>Hi</span></p>` span already reports a non-empty content rect, so the violation is live on `main` and ordering a layout program behind its fix changes nothing for the targets already affected. Owned by `#11-resize-observer-inline-empty-content-rect` instead, with the `client*` predicate prereq as its input |
| A30 | R10 | this memo's own R9 refutation — that the **forced**-break route makes the baseline half of the flush replay discriminable, pinned by the new cell it adds, **24f** — and, with it, re-fixturing rather than withdrawing | measured, and the general statement rather than one fixture's: a continuation line is opened only at an offset `find_break_opportunities` returned, every such offset is strictly inside its run's collapsed text (the end-of-text mandatory break is filtered, `crates/text/elidex-linebreak/src/lib.rs:28-33`), so the continuation line's first member is always that run's non-empty tail segment, and the tail always satisfies `contributes_content` — raising the `RenderedText` rung M7's promote vetoes on. 24f's own fixture fails one step earlier: under `pre-line` the space after the preserved break is collapsed away, so the text the packer sees has no interior break and no second line at all. The one route outside that argument's scope is the **unlicensed** item-boundary wrap, which §6's standing rule forbids a cell from depending on — so it cannot be the discriminator either. So the DoD keeps the **height** half (cell 24e) and the baseline half is withdrawn to `#11-inline-open-box-strut-on-continuation-line`, with the unreachability recorded there as the measurement and the argument that generalises it |
| A31 | R11 | §5.1 M1's first Grounds bullet — "the axis is the IFC's by construction, so its source cannot matter", concluded from css-writing-modes-4 §3.2 — and with it the payload's `WritingModeContext::new(style.writing_mode, …)` source | the ground assumed a rule elidex does not implement. Measured by the *property*, not the word "blockify" (`grep -rn '\.display = ' --include='*.rs' crates/`, each hit read): exactly **four** sites rewrite a computed `display` toward a block-ish value — `resolve/mod.rs:182` (abs/fixed), `:185` (float), flex `helpers.rs:69`, grid `helpers.rs:53` — and none is keyed on `writing-mode`; the complement, `pseudo.rs:55`, assigns an initial `Display::Inline` and is not a blockification. `display.blockify()` (`resolve/mod.rs:218-219`) would give `block` (`:469`) where §3.2 asks for `inline-block`, so the helper is the wrong rule and not a wiring gap. So `<p><span style="writing-mode:vertical-rl; padding-top:10px">x</span></p>` keeps `Display::Inline` and does get a marker. The Decision changes with the ground — the writing mode is the **IFC root's** (already in scope as `root_horizontal`, `inline/collect.rs:150`/`:211`), the `direction` stays the box's — which is what makes the ground a theorem here rather than an assumption. New cell **12f**, new slot `#11-writing-mode-inline-blockification`, §3's css-writing-modes-4 §3.2 row `✓ → ✗`. Found by Codex on #515 |
| A32 | R11 | §8's PR-1b suite-level break invariant licensing only `find_break_opportunities(<the fixture's concatenated text>)` plus forced breaks, and `#11-inline-item-boundary-soft-wrap`'s scope over *every* item boundary | css-text-3 §5.5 carries **two** adjacent bullets and the memo quoted only the first; the second gives a soft wrap opportunity before and after each atomic inline. Atomic items contribute no text, so `a<img>b` concatenates to `"ab"` with an empty opportunity set, and a width-forced flush before the `<img>` — licensed — would have been reported unlicensed and blamed on the slot: the invariant condemned conforming behaviour, and the slot claimed a defect where the packer is accidentally right. Latent rather than live at rev 44 — §6's only in-line atomic fixture is cell 6c's `<p>a<img style="padding:10px">b</p>`, which is not width-constrained (cell 25's `inline-block` is a *containing* block, not an atomic in a line). The licensed set gains the two boundaries of every atomic item; the slot gains a preservation clause, because deriving the test from concatenated text would remove them. Found by Codex on #515 |
| A33 | R11 | §7's "Painted output — **unchanged by PR-1c**" as a universal, and A8's count as its support on the `position:relative` arm | the support was a **count** — 0 / 1 / 1 red rects — and a count cannot discriminate geometry, which is the only thing that moves on the arm where the program's change is user-visible. Measured directly (probe appended to `crates/core/elidex-render/src/builder/tests/inline_flow/relpos.rs`, harness copied from `consumes_relpos_inline_subflow_with_gap`, run and reverted): a `position:relative` `background: red` span paints `SolidRect (24.0, 0.0) 16.0 x 20.0` today and `SolidRect (12.0, -12.0) 40.0 x 44.0` with padding 10 and border 2 — one rect either way. ⚠ **And the count is not invariant in general**: re-measured with the style's `border` decoupled from the `LayoutBox`'s, the same span with `border: 2px solid` goes **1 → 5** rects, because `emit_borders` takes each side's thickness from `LayoutBox.border` and an inline's is zero today. The probe that supported the original claim had picked the one member of the family whose count holds still — which is the point of this row twice over. The chain is `paint_non_sc` skipping positioned children (`walk.rs:650`), Layer 6/7's `walk` (`:614-629`, `:728`), `emit_background` / `emit_borders` reading `lb.border_box()` (`builder/paint/mod.rs:68`, `:382`), and M4 putting the three real `EdgeSizes` on that box at PR-1c (§5.1 M4; §5.2's `pack/boxes.rs` row). New cell **13d** asserts the rect; A8 is narrowed to the static subset rather than deleted. ⚠ **The class, not only the instance**: this is the second time in this program's review that a control measured a quantity the change does not move — A5's tagless *line count* where only the split point discriminates was the first. Found by Codex on #515 |
| A34 | R12 | that §3's `Full enum?` column needed no stated reading — that its meaning was settled by the marks themselves | the column is the schema's (`.claude/skills/elidex-plan-review/SKILL.md:30-39`), and the question it is mandated to answer is stated there at `:32`: catching "spec-step branch **under-enumeration**". This memo has used it instead as a **conformance-and-routing** mark and nowhere said so — `git grep -c "Full enum" 03393e14 -- docs/plans/2026-08-line-box-decorated-inline-content.md` → **1**, the header row. Three readings were therefore available to every round, and R11's flip of the css-writing-modes-4 §3.2 row came out right **by luck of the reading** rather than by rule: it applied a conformance reading to a row written under "the spec makes this case not arise". §3's preamble now states the two-mark reading, names `plan-xcheck.py` check 4 (`.claude/tools/plan-xcheck.py:135-145`) as the thing that *executes* it rather than a prose rule ([[feedback_prose-rules-cannot-fix-unexecuted-claims]]), and names the residual the checker cannot see — check 4 tests where a Touch **points**, never whether what it points at **delivers**. A35 and A36 are the two rows that residual had left false, neither of them found by the checker |
| A35 | R12 | §3's css-writing-modes-4 §6.4 row's unqualified `✓` — that M1's `WritingModeContext` source satisfies the mapping "based on the **used** `direction` and `writing-mode`" *of the box being mapped* | true of the `direction` component and false of the `writing-mode` one, and it is **R11's own sweep miss**: the measurement that flipped the css-writing-modes-4 §3.2 row left standing the row whose Touch names the very thing R11 changed. After **A31** the payload is `WritingModeContext::new(<the IFC root's writing mode>, style.direction)` (§5.1 M1), and §6 cell 12f says so in its own words — it pins "the *conservative degradation*, **not** conformance" — its closing sentence assigning cells 12b and 12e to the `direction` half. The mark is **split, not flipped**, on the precedent of the cssom-view-1 §6 one-line row (`✓ for a box with one fragment on one line; the residue is the row below`): `✓` for the `direction` half, the `writing-mode` half routed to the css-writing-modes-4 §3.2 row above, whose `#11-writing-mode-inline-blockification` retires 12f. ⚠ The slot is therefore named in **that** row's Touch and not in this one's, which is what keeps check 4 quiet for the right reason rather than by placement — a routed branch is `✗` on the row that owns it |
| A36 | R12 | §3's css-text-3 §5.5 *intra-word shaping* row's `✓ (pre-existing)`, and with it the reading of `pack/mod.rs:744` as the mechanism that delivers "the characters must still be shaped … as if the word were still whole" | `:744` answers the **inverse** question. Its coalescing is scoped within one line **by explicit design**: `git grep -nF 'self.last_placed_entity = None' 154bac3f -- crates/layout/elidex-layout-block/src/inline/pack/mod.rs` returns the reset at `:439`, under the comment at `:436-438` that states why — "New line starts a fresh run even for the same entity … reset explicitly so coalescing can't reach across the line break" (⚠ that sentence is line-wrapped in the source, so a grep for it whole returns nothing; the probe is the assignment) — so it re-joins the engine's own *within-line* segmentation, while the clause is about a word broken **across** lines (§5.5's worked example: "نوشتن" broken between "ش" and "ت", the "ش" keeping its **initial** form). Render then shapes the halves independently, `builder/inline_flow.rs:107-124` iterating per line with one rustybuzz call per run. **Reachable with no unimplemented property**: the clause's four named triggers and hyphenation are absent from the engine's **122** `ComputedStyle` fields, but its opportunities come from UAX #14 whole, and `linebreaks("نوش\u{00AD}تن")` returns `[(8, Allowed), (12, Mandatory)]` — an interior break at a joining-**transparent** SOFT HYPHEN — against `[(10, Mandatory)]` for the unbroken word. New slot `#11-intra-word-shaping-across-line-break` (§5.3, §8, §10), pre-existing class, and the row is `✗ (pre-existing → slot)`. ⚠ **The evidence stops where it stops**: structural plus that probe, no Arabic fixture rendered end-to-end, and the slot carries the limit rather than the row implying a behavioural measurement |
| A37 | R12 | R11's own atomic term — "both boundaries of every atomic item, deliberately *permissive* at the GL/WJ/ZWJ exception", on the ground that a subset assertion is unsound only when it **rejects** a licensed break | the ground inverts for this invariant: over-licensing an **item boundary** is precisely how the flush it exists to catch gets blessed, so a fixture could have pinned an item-boundary bug at an atomic adjacent to a WJ (a control widened until it blesses the defect it was built to catch). And no class modelling is needed to state the rule exactly — measured through `find_break_opportunities` (probe run and reverted): `"a\u{FFFC}b"` → `[(1, Allowed), (4, Allowed)]`; with a WJ (U+2060) or ZWJ (U+200D) on one side that side is suppressed, which **is** css-text-3 §5.5's second sentence; with NBSP (U+00A0) it is also suppressed, which is the **one** case §5.5 licenses by name. So the licensed set is the breaker's own answer over the U+FFFC-encoded text ∪ NBSP-adjacent atomic boundaries. R11's "U+FFFC alone does not encode the rule" is withdrawn with it — it encodes all of it but NBSP. Found by Codex on #515 |
| A38 | R12 | that PR-1c may leave the relayout refresh to `#11-inline-relayout-box-staleness`, on R11's ground that "what M4 changes is *which fields* go stale, not *whether* they do" | true for a wrong **number**, false for a wrong **picture**, and R11's own cell 13d is what made the difference: `InlineFlow` is rebuilt every pass (`git grep -nF 'Reconcile InlineFlow' 154bac3f -- crates/layout/elidex-layout-block/src/inline/mod.rs` → `:413`; the unconditional `insert_one` at `:528`; the candidate-key clear at `:156`) while `assign_inline_layout_boxes` skips any entity that already carries a `LayoutBox` (`boxes.rs:62-64`). After PR-1c a restyle therefore moves the glyphs and leaves the background and border at the first-layout box — a desynchronisation created here, since nothing is painted for an inline today. PR-1c takes the `layout_generation` comparison the slot already prescribes (`boxes.rs:86`), at a file M4 edits anyway; the slot keeps every other frozen `LayoutBox`, `content` included. Found by Codex on #515 ⚠ **Narrowed at R13**: the *promotion* stands, the prescribed mechanism does not — see A39 |
| A39 | R13 | the `layout_generation` comparison as PR-1c's repair — taken from `#11-inline-relayout-box-staleness`'s own prescription when R12 promoted it | **inert off the paged path**, and the engine states it at both sites that already had to solve this: `git show 22de3078:crates/layout/elidex-layout-block/src/inline/reconcile.rs` at `:198-200` → "`layout_generation` is constant 0 off the paged path, so this is an explicit reconcile (insert-or-remove), not a generation comparison", and `…/inline/mod.rs` `:558` → "(F9 — `layout_generation` is constant 0 non-paged, so removal, not comparison)". Stored and current both read `0`, so the comparison skips exactly where the presence test does. ⚠ **The general form**: a deferral puts the *problem* under review and leaves the *remedy* unexamined, so promoting a slot's prescribed fix into a PR is the point at which that fix must be re-derived — R12 verified *which PR should own it*, not *whether it works*. Replaced by the ownership predicate §8 and §9 now state, which is the idiom the engine already uses for `InlineFlow`. Found by Codex on #515 |
| A40 | R15 | PR-1c carrying a box reconciler of its own — the ownership predicate R14 wrote, which specifies the **insert** half and is silent on removal | an entity that *leaves* the producer set keeps its IFC-written box forever: restyle a decorated empty `<span>`'s last edge to zero and M1 emits no marker while no text gives `place_item` bounds either, so nothing overwrites and nothing removes. ⚠ **Third consecutive round on one mechanism, so the Step-4 self-root-check ran instead of a fourth patch**, and both its written questions point the same way. (1) The canonical algorithm is not missing — it is **two-sided and already in the engine**: `reconcile_flows` persists what the pass produced and `clear_inline_flows` removes it from every unpersisted candidate, the latter calling itself "the single staleness reconciler" (`git show 22de3078:crates/layout/elidex-layout-block/src/inline/mod.rs` at `:552-559`). R14 quoted "insert-or-remove" and implemented insert. (2) A second, box-only, insert-only reconciler beside the one that calls itself single is precisely the "N implementations of one rule" CLAUDE.md *One issue, one way* forbids. So the repair is carved into a **prerequisite PR** that extends that reconciler to the IFC-owned `LayoutBox` in both directions — which also repairs `LayoutBox.content` and therefore **discharges** `#11-inline-relayout-box-staleness` rather than narrowing it. Found by Codex on #515 |
| A41 | R15 | §3's css-text-3 §7.3 row quoting **one** of the clause's two normative sentences, marking `✗` on triggers 2 and 3 of the first, and §1.4's reading of that first sentence as the live half | the reviewer's premise — three triggers, one delivered, two unowned — fails on measurement, and so does the row it corrects. A text run carries the **parent element**'s entity (`collect.rs:309` pushes `StyledRun::from_style(parent_entity, …)`; only the pseudo arm at `:263` pushes the child), so any inline box that places a member changes the entity across its boundary and `pack/mod.rs:744`'s `coalesce` is false there by construction — the file's own comment at `:727-735` says so. elidex therefore already breaks shaping at those boundaries, **by over-breaking**, for all three triggers alike; triggers 2 and 3 are not live divergences and nothing in this program implements them. What *is* live is the sentence the memo never quoted: shaping must **not** be broken where formatting does not effectively change (`body css-text-3 boundary-shaping`), and `<p>a<span>x</span>c</p>` with an unstyled span is shaped as three runs — `grep -c 'must not be broken'` and `grep -c 'effective change'` over the memo both returned **0** before this round. New slot `#11-shaping-break-at-unchanged-inline-boundary` (§1.4, §3, §5.3, §8, §10), pre-existing class, and the row takes a split verdict. ⚠ **§1.4's own premise is sharpened, not withdrawn, and the audit's first reading that it was false is itself rejected here**: same-entity text *does* meet across a boundary, in exactly the case where the box places nothing — cells 14 and 15's markup, PR-1b's subject — and that same case is the one the engine already gets right under the second sentence, so the two halves are complements rather than a contradiction. Both are recorded because a row that logged only the fix would lose the reviewer's premise being false. Found by Codex on #515 |
| A42 | R15 | §9's predicate-prereq bullet closing the replaced-inline gap with "closing the gap is separate, pre-existing work neither this PR nor this umbrella takes on" and naming no destination | a hand-off to nobody is not a disposition. The gap is author-reachable today — `is_atomic_inline` (`inline/collect.rs:14-19`) matches four display keywords and not `Inline`, so an `<img>` in a paragraph is taken for an inline box, recursed into, and emits no `InlineItem`; `<p>a<img src=x>b</p>` advances by zero. The inline module reaches no replaced element at all, its one `replaced` / `ImageData` / `get_intrinsic_size` hit being the doc comment at `inline/styled_run.rs:12`, while the sizing exists one directory over in `block/replaced.rs`. New slot `#11-replaced-inline-no-atomic-layout` (§5.3, §9, §10), pre-existing class; the prereq PR's predicate supplies the classification, not the layout. Found by Codex on #515 |
| A43 | R15 | R15's own first disposition of the reviewer's §7.3 finding — that triggers 2 and 3 are **not** live divergences, because an entity change already breaks shaping at their boundaries | true only where the box **places content**. The member-less corner is the same corner for all three triggers and only trigger 1 closes in it: M1 emits a marker for a non-zero edge, so `<p>a<span style="padding:1px"></span>b</p>` breaks at PR-1b, while `<p>a<span style="vertical-align:super"></span>b</p>` gets no marker and keeps its two `<p>`-entity runs coalesced, after PR-1d as much as today. Measured: `git grep -nE 'vertical_align'` over `crates/layout/elidex-layout-block/src/inline/` at `154bac3f` (with `unicode_bidi` and `Isolate` in the same alternation) is empty, and `last_placed_entity` has exactly two write sites, `pack/mod.rs:439` and `:772`, so identity is the sole discriminator. ⚠ The reviewer's finding was **right as stated**, and this side's first two readings of it were wrong in opposite directions — first that the triggers were unowned with no measurement behind the claim, then that they were not live at all. New slot `#11-shaping-break-vertical-align-and-isolation` |
| A44 | R16 | this umbrella prescribing the prerequisite PR's **mechanism** — revs 50–51's candidate-set derivation off the collect walk, the ownership flag "riding the producer's push", and the `clear_inline_flows`-extension sketch | **under-specified as a PR and over-specified as a design, in one move.** R15 carved the repair into a prerequisite and the bookkeeping never followed: it was absent from §5.3's roll-call, from §8's sequence and topology, and from the §5.3 pre-existing-class list, while §8 and §9 each carried two paragraphs designing how it would work. R16 returned five findings and **four are defects in that prescribed mechanism** — a candidate set that is not a superset of previously owned boxes (risk 1, §8), a reconciler that covers `LayoutBox` and not `InlineClientRects` (the obligation, §8), and a removal that wire #5 bans with no `EcsDom` API to route it through (risk 2, §8) — because the memo presented the mechanism as settled and the reviewer read it at umbrella depth, correctly. The measurement that makes this a change of **altitude** and not a scope cut: the memo grew from **507,706** to **607,814** characters across revs 44–51 (`git show b14fcecd:docs/plans/2026-08-line-box-decorated-inline-content.md` against `2815674a`'s copy, both `LC_ALL=en_US.UTF-8 wc -m`) while the per-round new-real count went **3, 2, 1, 3, 5** over R12–R16 — a rising finding rate against a rising input, which is a loop that does not converge. CLAUDE.md's *Edge-dense work* rule already answers it: the prerequisite is a terminal per-PR slice and gets its own memo and its own `/elidex-plan-review`. So the umbrella states the obligation, names the three risks that review must answer, and stops. **Nothing is dropped** — every R16 finding is recorded, three of them as that PR's required plan-review inputs. Found by Codex on #515 |
| A45 | R16 | §3's css-inline-3 §5.3 row reading "two conditions, one delivered", with the *glyphless* condition delivered by M7's tentative baseline at PR-1d | delivered **only where a font resolves**, and elidex does not guarantee one. `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60-84` at `154bac3f`) ends `self.db.query(&query)` and answers `None` when the family list matches nothing, with no last-resort family appended anywhere in the call. The case this program reaches it through is the one line it creates that has nothing else to take a baseline from: `<p><span style="font-family:no-such-family;padding:1px"></span></p>` carries no text, so `inline/mod.rs:200`'s measurability gate — inside `if has_text` at `:191` — never runs, M5 commits the line on the inline-axis edge, M7's tentative gets `None`, and the IFC returns `first_baseline == None` where §5.3 asks for the strut's. ⚠ **The gap is not created here**: the same `None` reaches `measure_text` (`elidex-shaping/src/measurement.rs:54`), so `origin/main` already renders ordinary text in an unavailable family as nothing, with no marker involved — which is why the class is **pre-existing** and the disclosure, not the fix, is owed. New slot `#11-shaping-no-last-resort-font` (§3, §5.3, §8, §10), reachable far past this program. Found by Codex on #515 |
| A46 | R17 | rev 52's own obligation for the reconciler prereq — that the reconciler covers `LayoutBox` **and** `InlineClientRects` "in **both** directions", written as an unconditional pair of components | measured, the pair is not unconditional, and a reconciler built to it would be wrong in both halves on one entity. `atomic::layout_atomic_items` runs at `inline/mod.rs:179` and `pack::assign_inline_layout_boxes` at `:380` (both `22de3078`); the atomic path takes its box from `layout_child` (`inline/atomic.rs:60`), the child's own formatting context, and the tail skips every entity that already carries a `LayoutBox` (`pack/boxes.rs:62-64`). So a decorated inline restyled to `inline-block` holds a **fresh** box written before the tail and a **stale** `InlineClientRects` — whose only producer in the workspace is that tail and which nothing ever removes (`git grep -nF 'InlineClientRects' -- crates` → one `insert_one`, no `remove_one`, 2026-09-21). The obligation is restated over **outcomes** — no earlier pass's IFC geometry survives for an entity the IFC no longer lays out, and no geometry this pass's producer wrote is removed — and the transfer case becomes risk 4, which names distinguishing the current producer **at removal time** as a requirement and leaves *how* to that PR's plan-review. Found by Codex on #515 ⚠ **Form superseded at R18 (A48)**: the invariant and the risks are withdrawn, the transfer measurement stands as direction 3 of §8's defect statement |
| A47 | R17 | the deferral of `#11-inline-min-content-box-edges` past PR-1b, on M8's ground that `min_content_inline_size` has no accumulator for an edge to join | that ground measures what the fix **costs**, not whether PR-1b is correct without it. That is **A38's** shape and not A39's — A38 withdrew a deferral whose ground was true of a wrong *number* and false of the wrong *picture*, which is exactly what happens when the ground is read off the pass in isolation instead of off the pass and the layout together; A39's is the narrower case of a slot's *prescribed remedy* going unexamined. PR-1b is **not** correct without it. The inconsistency is not between the two intrinsic sizes, where css-sizing-3 §5.2's "does not define precisely how to determine these sizes" would leave the engine a choice; it is between intrinsic sizing and the layout PR-1b ships. `shrink_to_fit_width` is `min(max_content, max(min_content, available))` (`crates/layout/elidex-layout/src/intrinsic/mod.rs:134`), fed by `intrinsic/block.rs:39`/`:49` and consumed at `elidex-layout/src/layout/mod.rs:57` for an `auto`-width inline-block, so at a small available width the box settles on the **word** width while PR-1b's line needs word + edges and overflows — and deferring the *max*-content half instead only relocates that. The work therefore lands as a **prerequisite PR ahead of PR-1b** (§8), stated as an obligation with named risks, and the slot is **withdrawn rather than re-tagged**: a gap that never opens is not a deferral, so §10's `(own)` row is deleted, §5.3's PR-1b count goes to none, and §3's min-content row is answered by the prerequisite. Found by Codex on #515 ⚠ **Form superseded at R18 (A48)**: the obligation and its named risks are withdrawn, the ordering and the withdrawal of the slot stand |
| A48 | R18 | this umbrella stating a **requirement** for the reconciler and min-content prereqs at all — rev 52's component pair, rev 53's outcome invariant, and rev 53's invariant plus four named risks: three attempts, three rounds | each had a corner the next round found, and the pattern is four rounds deep — new-real findings **3 / 5 / 3 / 2** over R15–R18, of which **1 / 4 / 2 / 2** were defects in the previous round's own text (R16's four is A44's own figure; R15's one is A40, against R14's repair; R17's two are rev 52's prereq count and rev 52's obligation; R18's two are both rev 53's). R18's pair are instances the requirement did not reach: an `inline-block` restyled to a decorated `inline`, where the **other** producer's stale box survives on an entity the IFC does lay out, and min-content's loss of **joining** across items, which no edge term closes. Both are facts about the defect, so both land inside it rather than beside it. ⚠ **The memo's own idiom for work it hands off is a slot** — the gap, a Why, a trigger, a date, an owner — and its slots state the gap and hand the mechanism over; none writes an invariant for the fix to satisfy, the nearest being `#11-resize-observer-inline-empty-content-rect`'s *What the fix needs*, which names an **input** the fix consumes. The heavier form was invented for these two, and a form that reads as a specification is reviewed as one. §8's two paragraphs therefore become **defect statements** with the extent that bounds them; the obligation / invariant / named-risk framing goes, A46's transfer measurement and A47's shrink-to-fit arithmetic surviving inside them. ⚠ Rev 53's transfer sentence was itself false and is not carried over: it reasoned about an entity "now absent from `entity_bounds`", while `place_item` pushes a rect for every placed item whose entity is not the IFC's own — atomics included (`inline/pack/mod.rs:703-715` at `154bac3f`). **Prediction, recorded so R19 can falsify it: R19's findings will not be about these two prerequisites.** Found by Codex on #515 |
| A49 | R20 | §1.4's and §3's count of css-text-3 §7.3 as **two** normative sentences (the row's `Step` read "both normative sentences"), and §3's css-inline-3 §5.3 half-leading row marking `A′ = A + L/2` `✓ (pre-existing)` as though the formula were the section's whole rule | both are miscounts of a section this memo had open, and `webref` is the truth-maker for each. **§7.3 has three normative sentences** — `body css-text-3 boundary-shaping` lines **9** ("must be broken"), **39** ("must not be broken") and **63** ("should not be broken … otherwise, if it is reasonable and possible for that case given the limitations of the font technology"). The third's case is a boundary that *does* change the glyphs while meeting none of sentence (a)'s three triggers; it is **reachable with zero author CSS** through `crates/css/elidex-style/src/ua.rs:117`'s `b, strong { font-weight: bolder; }` — a text run carries the parent element's entity (`collect.rs:309`) and `pack/mod.rs:744`'s coalesce is entity-equality, so `<p>نو<b>شتن</b></p>` is two runs — and `#11-shaping-break-at-unchanged-inline-boundary` does not reach it, its predicate being this one's complement. New slot `#11-shaping-across-formatting-change-boundary`. **§5.3 has two branches** — `body css-inline-3 inline-height` lines **13** (`normal`: the bounds enclose **all** the box's glyphs, explicitly across fonts) and **15** (not `normal`: the first available font alone, with `L = line-height - (A + D)`) — and the marked formula is the **second**'s, while `normal` is `line-height`'s **initial** value, so the branch left unmarked is the default one. elidex collapses `normal` to `font_size * 1.2` (`crates/core/elidex-plugin/src/computed_style/text.rs:136`) and runs the second for both; **five** §3 rows cite §5.3 and none distinguished the branches. New slot `#11-inline-height-normal-layout-bounds`; both slots are **pre-existing** class. ⚠ The §7.3 undercount dates from R15, the round that rewrote that row **twice** (A41 and A43 are both R15's) without re-reading the section whole, and stood unremarked through R16–R18 — which is the ground for A50's audit rather than for a third patch of the same row |
| A50 | R20 | this memo having run **no** systematic enumeration-completeness audit of §3 — the audit `.claude/skills/elidex-plan-review` Pre-condition #1 describes, which rev 47 set aside when it wrote down that the `Full enum?` column answers a different question (A34) | the re-purposing was recorded and the audit was never run in its place, so the schema's question went unasked across 25 sections. Run by hand at R20 over the **25 distinct spec sections** at `09026f98` (§3's preamble carries the command; the unit is **sections**, not the 42 rows — the two part wherever a section carries several rows, and that is where the reviewer's framing differed), against a stated standard: a branch is covered when a §3 row, **or a place a §3 row explicitly hands off to, names it**, a mention in prose elsewhere being a separate tier. **Nine** reachable branches had no row, no §6 cell, no slot and no disposition; **two** are reachable with zero author CSS and **three** are rows that quoted a truncated normative sentence and then marked against the fragment. Result: six rows added (M 42→48, K unchanged), three existing rows corrected, six new **pre-existing**-class slots, three explicit no-slot dispositions, and one finding recorded as *conformant* rather than as a gap — css-text-3 §5.5 bullet 6's out-of-flow half, where `PackItem::Placeholder` records a static position and reaches no flush (`inline/pack/mod.rs:667`) and `inline/whitespace.rs:53` skips it. ⚠ **Two framings are kept against the reviewer's stronger version**: the unit is sections rather than rows, and "no completeness audit at all" is too strong — §1.2's five-clause table for css-inline-3 §2.3 is a real per-clause enumeration, and the schema's own remedy for an un-enumerated branch (implement it upfront **or** name a defer slot, `feedback_plan-scope-re-evaluation.md:44`) is what this column's `✗` already means. What was absent is a **systematic** pass, and the loss is confined to branches the table never *listed*. ⚠ The audit is the one-time manual discharge of the clause-case check §9's tooling task books (rev 51, R15); the mechanical form stays that task's, and a hand pass does not re-fire when a row is added or a module is renumbered |
| A51 | R22 | `#11-inline-root-inline-box`'s scope as "the whole css-inline-3 §5.3 layout-bounds model", and with it the trigger "any work that gives inline boxes layout bounds of their own" | that scope covers §5.3's **`line-height: normal`** branch, which rev 55 had just given its own slot, `#11-inline-height-normal-layout-bounds` — so two slots held one subject and one triggering event, and either could close leaving the other half undone. Rev 55 declined the *fold* (the five facets are each pinned by a §6 cell, and none turns on the per-glyph provenance the `normal` branch is defined on) and that ground stands; what it left unaddressed is the **overlap**, which the reviewer's second option fixes. The sibling's scope narrows to §5.3 **as composition** — the wording §8's fold-refusal bullet already used for it — its trigger narrows to composing per-box bounds into a line, and each slot carries the other's discharge as a trigger disjunct, the adjacent-slot form the fontless-text slot in §5.3 already uses. Swept at §1.3's R2 record, §1.5, §2's coupled-invariant row 5, §3's `normal`-branch row, §5.3 and the two §10 rows. Beside it, the css-pseudo-4 §4.1 slot's **input** is corrected from the composed inline-box predicate to its **replacedness half**: §4.1 suppresses on replacedness alone, and the composite is false for a non-replaced atomic origin such as `span { display: inline-block }`, whose generated children it does not suppress — an implementer taking the composite would strip them while fixing `<img>`. Checked against the other consumers §9's canonical-predicate bullet names: M1's emit test, the four `client*` members and `#11-resize-observer-inline-empty-content-rect` all classify an *inline box* and take the whole predicate legitimately, so the confusion was one entry's; that slot's "precisely the half §3.3.1 keys on" is corrected in the same edit, resize-observer-1 §3.3.1 reading "non-replaced **inline** Elements" |
| A52 | R22 | three published measurements that do not reproduce from the commands that define them, and twelve coordinate / enumeration items behind them | not one of them changes a conclusion, which is the reason they are one row: the standard they fail is this memo's own — a figure reproduces from the command stated beside it. §9's CSS 2 cite list read **eight** where its grep returns **nine**, `CSS 2 §9.5` having entered in **rev 55's own commit** at §3's css-inline-3 §2.1 row, so the enumeration contradicted its definition from the moment it was written and the round that wrote it was recorded dry; the front matter's "lone title-less cite" becomes two, and its **seven** title pairs are untouched (seven titled, two untitled, nine sections). The `origin/main` checker probe read `grep -c plan-` and returns **5**, matching the `elidex-plan-review` skill's files and `.plan-z1b-consume-delta.md`; the landing-delta probe carried a second disjunct and returns **eight** lines where the sentence claimed two. Both conclusions survive on discriminating forms, which is the point: a transcript that does not discriminate is not evidence, however true its claim. The twelve: `collect.rs:230` (a `22de3078` coordinate in the `154bac3f` frame, beside a `154bac3f` sibling in the same sentence); §5.1's flex/grid `helpers.rs:69`/`:53` stated without the "two above the hit" note §5.3 carries; the end-of-text break filter credited to `pack/items.rs:74` instead of `elidex-linebreak/src/lib.rs:28-33`; "four further sites" for a grep returning six hits; `resize.rs:251-254`; `component.rs:140`; `pseudo.rs:27-50` for a function ending at `:81`; `inline_flow.rs:107-124` described as shaping when it collects; `last_placed_entity`'s "two write sites" counted under a different convention from §4's own; PR-1a's `if let` exemption list missing `pack/mod.rs:534` and `:607` (the universal it guards is true — the list was short, not the claim); `17e`'s unrecorded pre-freeze withdrawal; and §7's `border_box()` family list missing the directory with the grep's largest per-file count, plus a second name-substring artefact beside `page.rs`. Three spec quotations are re-marked: css-break-3 §5.4's `…` under a "verbatim" label, css-inline-3 §6.4's Note truncated at its comma, and css-content-3 §1's issue note whose elided half is its "Presumably … might need an exception" hedge |
| A53 | R22 | the TERMINAL attestation's "roughly eight never-executed numeric claims" as the size of this memo's unexecuted-figure blind spot | R22's probe run executed **27** runtime claims at `22de3078` and **26** agreed, most to the digit, which discharges the pass for the claims it reached and leaves the population unmeasured. The enumeration behind it reports the property carried by the **hundred**, tens of them phrased as a measurement of today — an order of magnitude past the estimate, and no figure is carried in §9 because the property has no command that returns its population. One claim was corrected, and it is a **gloss** rather than a figure: **A15** above, where "≈22px is the first available font's ascent + descent" is false of any font (Arial at 16px sums A and D in magnitude to 17.875) while ≈22 is the **border area** cell 13b's markup asks for, 17.875 + 2 × 2px = 21.875. Three claims stated in the present tense of a frame that cannot falsify them are re-tensed — cells **23** and **23b**, whose markup yields `line_count` 0 today, and cell **13b**'s required test, whose border-box half is false until PR-1c — none of them a change of markup or of expected behaviour, and cell 23's substance is confirmed by §5.1 M6's executable sibling shape. The register §9 now books cannot be keyed on the word "measured": in this memo it labels source-level measurement more often than execution, and misses real runtime figures entirely |
| A54 | R22 | §6's negative-edge coverage — cell 3's `margin:-10px` arm and cell 3b's all-sides cancelling pair — as reaching the geometry M4 produces | neither can invert a rect, so the class was untested where it bites. Cell 3's is a **single** box whose own start edge is consumed before M4 saves content-start, so its span is zero-width; 3b's pair sums to zero on **each** side, so the cursor returns to where it started. Depth > 1 with a net-negative inner advance does invert one, and nothing on the path normalises it: the per-line fold takes `min`/`max` only for an entity that **already** has an entry on that line (`pack/mod.rs:481-486`) and pushes a sole entry verbatim (`:485`), `or_insert` copies it (`:505-511`), `assign_inline_layout_boxes` writes `inline_end - inline_start` with no clamp (`pack/boxes.rs:76`, `:81`) and `Rect::new` stores it as given (`layout_types/rect.rs:141-146`). New cell **10c**; which side encloses what is PR-1c's plan-review's, and the cell pins neither answer |
| A55 | R22 | M4's **ground** for the flush-time emit filter — that emitting a zero-span open box's partial rect "would duplicate the rect its eventual `InlineBoxEnd` will push" | the hook runs at flush over boxes still **open**, so that `InlineBoxEnd` is on a **later** line by construction, and `commit_aligned_entity_rects` folds per line (`pack/mod.rs:479-487`) — two lines' entries become two `line_rects` entries, never one fragment twice. The question the filter actually settles is whether the zero-span presence is a **fragment**, which css-inline-3 §2.1 answers for a box split by an over-long line **or a forced break** and cssom-view-1 §6 step 3 then asks a rect for. The outcome stays pinned at cell 17c; the reason beside it is withdrawn. ⚠ R22-d's own fixture (`<pre>` with the break inside the decorated span) does **not** exhibit the loss: the preserved `"\n"` is its own `PackItem` (`items.rs:74-83`) whose run carries the span's entity, so `place_item` supplies the line-1 rect (`:703-714`) |
| A56 | R22 | rev 53's carve as stated — **both** instances of the min-content prerequisite ordered in `main` before PR-1b, branched off `main` and ordered against nothing else | instance 1 has no input at either base its own ordering admits. §5.1 M1 splits the payload by the dead-field rule: PR-1a emits the two marker variants carrying `entity`, and the three `EdgeSizes` + the `WritingModeContext` join the payload in **PR-1b**. Before PR-1a there are no markers and an empty decorated inline is absent from the stream (§4.2's Shape B); between PR-1a and PR-1b they carry no edge. Instance 2 (cross-item joining) reads `InlineItem::Text` only and is unaffected — and it is the half A47's ground and §8's structural ground are both read off. §8 records the defect and the tension; the split is the prerequisite's plan-review's, the ordering the umbrella's |
| A57 | R22 | R22 as a **two**-finding round, and R23's dry round as discharging what it left | a loop defect, not a memo defect, recorded here because the ledger is where a withdrawn claim about this program's own review goes. As reported by the round's operator, Codex posted R22 in two batches — 2 threads at 13:41:01Z and 3 more at 13:43:23Z — and the poll exited on the first, so three findings went unhandled until the next round's arm reported `unresolved=3 new_threads=0`: unresolved threads **older** than the trigger, the signature of a truncated round. R23 then came back dry, which discharges nothing — a reviewer does not re-raise what it has already raised. The three are A54, A55 and A56 above. ⚠ The two timestamps and the arm's counters are the operator's observation and are not reproducible from this repository |
| A58 | R25 | rev 57's disposition of A56 — the two instances' split recorded as unsettled, handed to the min-content prereq PR's plan-review, with the umbrella to re-derive the ordering from what it settled | an approval that leaves ownership open ratifies a self-contradiction: §5.1 M8 gave the min-content half to the prerequisite while §6 routed cell 25's min-content assertion to PR-1b, and ownership is this umbrella's to state, not a per-PR review's. Settled: **the prerequisite owns instance 2 only** (cross-item joining — pre-existing, decoration-independent, reads no marker, realisable at its own base; both grounds on record for carving it are its), and **PR-1b owns the edge term of both intrinsic sizes** (instance 1): `min_content_inline_size` and `max_content_inline_size` both call the `collect_inline_items` the layout pass calls (`inline/measure.rs:22`, `:51`), so PR-1b's M1 payload reaches them from M4 (i)'s one derivation site with no style read in `measure.rs`, and PR-1b ships the line advance with both edge terms, so no `main` commit has layout and intrinsic sizing disagreeing — A47's ground, met. The ordering (prerequisite in `main` before PR-1b) is unchanged; cell 25 stays PR-1b's and now asserts both halves as its own. The joining mechanism remains the prerequisite's plan-review's. Swept at §3's two css-sizing-3 rows, §5.1 M8, the `measure.rs` index row, §5.3's PR-1b bullet, cell 25, §8's PR-1b item and M8 clause, §8's prerequisite block, §9's intrinsic-sizing and §5.2.1 dispositions and §10's successor row. Found by Codex on #515 |
| A59 | R25, R26-F1/F2 | rev 52's altitude rule (ledger **A44**, narrowed by **A48**) read as covering **outcomes** as well as mechanism — handing a cell's expected result, a behaviour's owner or a governing rule's value to a downstream plan-review | the rule was about *how*, and three findings in two rounds are the same over-extension of it to *what*: R25's min-content ownership split, left open for the prerequisite's review (ledger **A58**, settled in rev 58); R26-F2, §3's css-sizing-3 §5.2.1 row handing the intrinsic passes' containing inline size to PR-1b's plan-review while §9 already stated `0.0`, so the memo carried two instructions for one value; and R26-F1, cell 10c recording that it "pins neither answer" for the inverted rect its own construction makes observable. The line is now stated once, as freeze discipline 5: outcomes here, mechanism there. R26-F2 is decided on that line — `0.0`, css-sizing-3 §5.2.1 rule 4, the percentage being cyclic in both passes (§3's row) — with new PR-1b cell **25b** pinning it and §5.2's `collect.rs` row and §9's two sites pointing at §3's row. ⚠ **R26-F1 is not decided in rev 59**: cell 10c's geometry is an outcome this memo owes, but no text found fixes it (css-box-4, css-sizing-3, css-inline-3, cssom-view-1, CSS 2 searched) — css-sizing-3 §3.3 gives "the inner size of a box cannot be negative" as the ground for a floor, CSS 2 §10.2 defines a non-replaced inline's content width as "that of the rendered content within them", and neither says where a floored extent sits — so the choice is left to an explicit decision, recorded here as open, rather than made by default. ⚠ **Two coordinates in earlier rows are corrected here, not in place** (rows are append-only; A33's treatment of A8): A58's "§9's intrinsic-sizing and §5.2.1 dispositions" means **css-sizing-3** §5.2.1 — this memo has no §5.2.1 — and A55's bare `:703-714`, following `items.rs:74-83`, is `pack/mod.rs`'s (the M4 row now qualifies it). Found by Codex on #515; the two coordinate corrections by an independent attestation of rev 58 |
| A60 | user, 2026-09-22; scoped plan-review of rev 60 | cell 10c's "pins neither answer" (A54's record); rev 59's proposal to clamp the inverted extent at content-start; rev 60's first draft of the outcome (the hull of the box's *rendered content* — own placed content plus descendants' border boxes); and three questions that draft left open | **Decided by the user**: an inline box's content rect encloses its descendants on the inline axis — CSS 2 §10.2, "The content width of a non-replaced inline element's boxes is that of the rendered content within them (before any relative offset of children)", and css-box-4 §2's content area that "contains its content—text, descendant boxes, an image or other replaced element content, etc.". **The design freeze is reopened for M4 only.** The first draft was replaced after the scoped plan-review (5 axes; 1 CRIT): keyed to *rendered content*, it made a decorated box whose text is keyed to a child entity zero-width (`<a style="padding:…"><strong>…</strong></a>`, cells 24e, 6i), excluded descendant margins on a gloss ("margins are never painted") the spec does not state, and quoted §10.2 without its parenthetical. **The outcome as landed**, for a box in M1's emit set: the hull of its own content-start and end-cursor points on the line and its descendant inline boxes' and atomics' border boxes (an atomic's border box as placed on the line, not the `LayoutBox` it holds at that point), positions before relative offset — never negative, enclosing in the inline axis by construction, zero-width at content-start with nothing rendered, equal to the rev-59 cursor span except where that span inverts or a descendant border box leaves it (10c, 10d). **The three questions, decided**: OQ-1 — a soft wrap before a box's first character is at its margin edge (css-text-3 §5.5), so 17c's "no rect on line N" is spec-correct and candidate (b) is withdrawn for that case; 17c's line-N start-edge consumption, stated as a requirement, is the registered ✗ deviation `#11-inline-box-decoration-splits` owns, and 17c now asserts line N+1's **extent** because a hull cannot invert and a stale content-start would widen it; a forced break does fragment the box (css-inline-3 §2.1, css-break-3 §2), new cell **17g** (A55's inner-element shape, content spans and count only — no collision with the slot's border-area routing). OQ-2 — candidate (b): each pass evaluates M1's predicate on the edges it resolves, css-sizing-3 §5.2.1 rule 4's zero being for contributions only while the used value is non-zero; no M1 change, no slot; the draft's "makes the used value zero" was wrong. OQ-3 — descendant margins between the box's two points are inside. **Other changes**: new cell **10d** (an atomic with a negative margin inverts the rev-59 span at depth 1, so 10c's "depth > 1 is what made it reachable at all" was false); 10b's containment scoped to the emit set; §8's PR-1c set-level read and 13c's "one cell whose box is not padding-only", both false since 10c, re-stated as "the one cell asserting the `border`/`margin` fields"; the undecorated-ancestor gap (`<span>a<b style="padding:5px">b</b></span>`, cell 10's inner-decorated arm) booked as a pre-existing defect on `#11-inline-zero-edge-box-in-item-stream`; §9's CSS 2 list and the front matter corrected to ten sections / three untitled (`grep -o 'CSS 2 §[0-9.]\+' <memo> \| sort -u`) — ⚠ **the tenth, §10.2, entered with rev 59** (`88630559`, A59's quote; `git log --oneline -S'CSS 2 §10.2'`), whose own blob already returns ten while its list read nine: the recurrence §9's list records for §9.5, a third time. **No other frozen cell's expected result moves**: §6's nested markups are 6d and 6e (undecorated outer, no marker), 6i and 24e (decorated outer over a child-keyed run — the hull equals the cursor span), cell 10's two arms, 10b (padding-only, spans coincide), 10c and 10d; 3 and 3b are single boxes whose base-case geometry 10c's closing ⚠ records rather than asserts. **Focused re-review of the revised draft (2 agents; 0 CRIT / 5 IMP / 9 MIN), applied**: every enclosure claim qualified as **inline-axis** (freeze paragraph, OQ-3, 10b, 10c's title, 17g, M4, this row) — css-box-4 §2 grounds that axis only, the block extent being the line's (`pack/mod.rs:488-494`, cell 13b), so a 2-D `contains` in 10b would fail under a correct producer; the atomic term re-stated as its border box **as placed on the line**, since its ECS `LayoutBox` is not final at the pop (`reposition_atomic_box`, `inline/mod.rs:496`/`:578`) and carries a baked-in relative offset (`atomic.rs:12-15`), and **10d** re-fixtured with `padding-left`, `border-left` and `position:relative;left` on the atomic so that it rejects hulling the atomic's content box and its relpos-offset box; 17c's line-N+1 start re-stated as M3's placement with the §5.5-conformant `[5, 5 + measure_width("cccc")]` as an accepted divergence on `#11-inline-box-decoration-splits` (its scope clause and the note after cell 15 now name 17c), and its discriminating precondition `measure_width("aaaa ") + 5 > measure_width("cccc")` made test-asserted; 17g's forced break cited to css-text-3 §5 *forced line break* and §5.5, its unmeasured red-under-filter claim dropped, its routing note softened to "the same exposure cell 17 has"; the css-text-3 §5.5 and css-sizing-3 §5.2.1 rule 4 quotes completed, and "the used value is non-zero" qualified; a duplicated `**Rect**:` string in M4 removed; the freeze's cell figure carried to 62; 17d added to the reopened-cell list. ⚠ **A59 quotes CSS 2 §10.2 without its parenthetical** "(before any relative offset of children)"; A59 is append-only, so the full sentence is this row's and M4's. ⚠ **A coordinate correction in the review's disposition was refused, measured**: it asked for `measure.rs:51` → `:50` (fn `:44`), but `git show 154bac3f:crates/layout/elidex-layout-block/src/inline/measure.rs \| grep -n 'collect_inline_items('` returns `22:` and `51:` — the memo's `:51`, and A58's and §3's, are right and stand |
| A61 | R27 | §8's "so no `main` commit has layout and intrinsic sizing disagreeing" (rev 58, A58's words, at §8's min-content prereq paragraph) as a universal, and the ⚠ beside it (rev 57) dismissing `InlineItem::Atomic` contributing zero with no owner | both intrinsic passes skip `InlineItem::Atomic` (`inline/measure.rs:13-14`, `:42-43`), so `<span style="padding:10px"><span style="display:inline-block;width:100px"></span></span>` gets 20px of intrinsic size against a 120px line after PR-1b — the universal was false. It was true only of the **edge terms**, which is what A47's ground covers, and it is narrowed to that at §8 and M8's Grounds; the atomic disagreement exists before and after the program equally, so PR-1b neither introduces nor widens it. The ⚠'s classification stands — not a third instance of the min-content prereq's defect, since it hits both passes alike — but a gap with no owner is a dismissal, so it is **registered**: `#11-inline-atomic-intrinsic-contribution`, pre-existing, decoration-independent, triggered by the min-content prereq PR's plan-review (whose accumulator an atomic term would land on) without being bundled into it; §5.3 definition, pre-existing list (now thirteen added in-revision), §9's intrinsic bullet and a §10 row (`approval PR`, like every Codex-opened pre-existing slot). No cell: the memo pins accepted divergences its program's own cells reach, and none of its fixtures carries an atomic inside a shrink-to-fit IFC. ⚠ A58 (append-only) carries the same universal; this row is its correction. Found by Codex on #515 |
| A62 | attestation of revs 59–61 | rev 61's narrowed universal "no `main` commit has layout and intrinsic sizing disagreeing **on the edge terms**" (§8; A61's "It was true only of the edge terms"); the dating "false since 10c" (§8's set-level read, A60); two "this revision → nine" self-measurements; `atomic.rs:12-15`; the §3 row leaving the `0.0`'s writer to PR-1b's plan-review | **Edge terms**: cell 25b pins an edge term that disagrees after PR-1b **by spec** — css-sizing-3 §5.2.1 rule 4 resolves a cyclic percentage against zero for contributions while layout resolves it — so the claim holds of **non-cyclic** edge terms only; §8 and M8's Grounds now say so and name 25b as the sanctioned exception (A61 is committed; this row corrects it). **Set-level read**: cell 13d has carried `border:2px solid` in the PR-1c list since R11, so "carries padding and nothing else" was false from R11, not from 10c; 13d paints its border from a `LayoutBox` its render harness sets by hand, so it tests the reader, and the read now says "produced by layout" with 13d carved out; the inline "until rev 60" marker is removed (freeze discipline 2). A60's "both false since 10c" is corrected here. **Self-measurements**: both "on this revision → nine" sites now name `46a48641` (rev 55, the commit §9.5 entered with), whose blob returns nine; the working copy returns ten. **Owner of the `0.0`**: PR-1a — its DoD threads `containing_inline_size` through every caller and the dead-field rule is met because PR-1a's emit test reads the resolved edges; PR-1b is where the value first shows (cell 25b). **Hygiene**: `atomic.rs:13-16` at M4 (A60's `:12-15` is the same range off by one, corrected here); CSS 2 §10.2 and css-text-3 §5.5 qualified at the front matter and cell 17c — A60's bare "quoted §10.2" and "the §5.5-conformant" mean CSS 2 §10.2 and css-text-3 §5.5, corrected here since A60 is committed; M4's rebase ground re-stated for the hull (a stale content-start widens, it cannot invert) and 17c added to its divergence list; the 2026-09-20 attestation's M4 verdict points at A60; 25b quotes css-sizing-3 §5.2.1's bullet exactly ("The containing block's size is not re-resolved…"). ⚠ **Lineage**: A60's `88630559` and its `git log --oneline -S'CSS 2 §10.2'` resolve on `layout-decorated-inline`; on the approval lineage the same command returns `179f7165` (rev 60's mirror — rev 59 was never mirrored alone). Both SHAs are reachable from `origin`, so the correction lives here rather than in A60 |
| A63 | R29-F3; user, 2026-09-23; three scoped plan-reviews of rev 63 | M3's side-specific hang gate (R2-F3 to rev 62): `hang = Some(0.0)` at a marker whose own side carries an inline-axis edge, on css-inline-3 §2's "Inline-axis margins, borders, and padding are respected between inline-level boxes" | **The freeze is reopened for M3's hang gate, by user decision, on Codex's reading**: css-text-3 §4.1.2 step 3 names no inline-box edge; §4.1.1's step 4 — inside its `normal | nowrap | pre-line` bullet, so grounding those values only — collapses a space "even one outside the boundary of the inline containing that space, provided both spaces are within the same inline formatting context"; §5.5 gives an inline box boundary no soft wrap opportunity; and css-inline-3 §2's sentence says what those edges do to **spacing**, not whether a space is line-final. **The amendment, as it lands**: a box marker does not make a trailing space non-line-final, whatever its edges; under `white-space: normal` step 3 then removes the space, which is the outcome cells 16 and 16b pin. Mechanism: the marker path passes `hang = None` to `note_line_occupancy`. **Sites**: M3's Decision and Grounds; M5's Decision (the predicate has **two** readings — consequential, and the first review's FP confirmed it); cell **16** as the (a)/(b) contrast, `h > 0` test-asserted; cell **16b** with its value and the existing `justify_excludes_trailing_hang_from_opportunities` (`inline/tests/inline_flow/justify.rs:148`) as the evidence that today's engine already produces it; cell **3b**'s arm re-pinned to "the marker does not end line-finality", its side-is-load-bearing paragraph withdrawn, `sp > 0` test-asserted; **14b**'s unobservable hang clause dropped; **17c**'s precondition re-derived to the weaker of the removed / hung readings; cell **3** and §8's reading counts; §3's css-text-3 §4.1.2 row split into a step-3 and a step-4 row and a css-text-3 §8.2 row added (M 48 → 50, which §3's breadth note explains); §5.3's PR-1b bullet; `current_line_last_hang` stated once, at M3, with §7 pointing there; §8's PR-1b detector item, whose population is now the **property** with its grep named as a seed, not an inventory (it misses `hang` lines carrying none of its four literals). **Not touched, checked**: M7's `RenderedText` rung and the round-26 soft-wrap guard (occupancy, not the hang), M4's flush replay (`hang` already `None`), cells 14, 15, 15b, 15d, 24–24e. Found by Codex on #515 |
| A64 | R29-F2; scoped plan-review of rev 63 | §8's min-content prereq defect stated as "no edge term, and no joining across items" with two instances, the prereq owning instance 2 only; and the first rev-63 draft's "layout breaks where `find_break_opportunities` puts an opportunity and nowhere else" | `min_content_inline_size` segments with `split_whitespace()` (`measure.rs:29`), not **layout's segmentation**, which is two things: the UAX #14 oracle `find_break_opportunities` (`elidex-linebreak/src/lib.rs:26`, called at `pack/items.rs:72`) **and** the trimmed segment width (`:690`) — so a bare segmenter swap would over-report by a space. It is not "nowhere else": the `:690` guard also flushes at item boundaries, `#11-inline-item-boundary-soft-wrap`'s pre-existing defect. **One defect**, widened rather than a slot opened ("one issue, one way"), whose instances are the cross-item join (2) and two per-run mis-splits anchored to the spec, not only the engine: NBSP (3; css-text-3 §5.5 — GL "must be honored") and CJK (4; css-text-3 §5.1 `word-break: normal`), against css-sizing-3 §2.1's min-content inline size ("…if all soft wrap opportunities within the box were taken"). The prereq owns 2–4, PR-1b the edge terms (A58's split); PR-1b is not correct without instance 2, and 3–4 predate the program, neither created nor widened by it (§9's bullet, A61's logic). **Reproduce 3 and 4**: compile and run, with `rustc`, a `main` printing `'\u{A0}'.is_whitespace()`, `"a\u{A0}b".split_whitespace().collect::<Vec<_>>()` and `"日本語".split_whitespace().collect::<Vec<_>>()` → `true ["a", "b"] ["日本語"]`; and append to `crates/text/elidex-linebreak/src/lib.rs`'s `mod tests` the test `#[test] fn nbsp_cjk() { assert!(find_break_opportunities("a\u{A0}b").is_empty()); assert_eq!(find_break_opportunities("日本語"), vec![(3, BreakOpportunity::Allowed), (6, BreakOpportunity::Allowed)]); }`, which passes under `cargo test -p elidex-linebreak` (the crate is byte-identical at `154bac3f` and this branch: `git diff 154bac3f HEAD -- crates/text/elidex-linebreak` is empty). M8's frozen Decision names the joining only — a subset. Found by Codex on #515 |
| A65 | R29-F1; scoped plan-review of rev 63 | §2's ten pair rows with a `2 × (all)` shorthand as the enumeration of seven invariants' couplings | seven invariants make **21** pairs, and the plan-review skill's Pre-condition #3 asks each intersection in one line. The shorthand is expanded — its M1 content is a second `1 × 2` row — and nine coupled pairs are added, each checked against the M-row it names: 1×7 (**two** rows after review: M5's shared predicate, M3's `BoxEdgeOnly` occupancy for the wrap guard), 2×3 (M4), 2×4 (M7), 2×5 (M6), 3×4 (M7), 3×5 (M6), 4×5 (`#11-inline-root-inline-box`), 4×7 and 5×7 (M3): twenty rows. Five pairs — 1×6, 3×6, 4×6, 5×6, 6×7 — have **no coupling**, each with its reason, in a table of their own placed after the §2 prose; review corrected three reasons (3×6: a marker's rect does commit, per entity, while no run enters a bucket; 4×6: the baseline is IFC-wide first-wins, `:570-587`, not per line; 6×7: `top_group_is_line_last`, `:264-275`, is a group-keyed reader of line-finality, unaffected because markers add no run). **No coupled pair was found that no M-row, slot or cell handles.** Re-derived with it: §5.3's four "Owns couplings", M4's enumeration of §2 (2×4 and 2×5 are delivery to the packer, not onto `LayoutBox`) and M4's own pairs (3×7 **and** 2×3). ⚠ **Checker gap, recorded in §9's tooling task**: the *Uncoupled pair* table is outside `plan-xcheck.py`'s §2 Pair population (rows there must route to an M-row or slot), so a pair missing from both tables passes — the unsafe direction. Found by Codex on #515 |
| A66 | user, 2026-09-23 | the end-of-line white-space domain — css-text-3 §4.1.2 for every `white-space` value — settled inside M3 (rev 63's second draft), and `#11-prewrap-forced-break-conditional-hang`, the slot that draft opened for step 4's forced-break branch | **Carved into a sixth prerequisite, the end-of-line white-space prereq PR, in `main` before PR-1b, by user decision**: the domain is a pre-existing, decoration-independent engine defect that decoration only makes visible. Its §8 block follows the min-content prereq's shape — a defect statement with measured instances, an ordering line, no `PR-1x` letter and no §10 row — and it is **not ordered against the min-content prereq**, the two touching one fact from different sides. The slot is **withdrawn, never committed**, absorbed by the carve, and removed from §5.3, the pre-existing list (back to thirteen), §3's step-4 row and §10 (35 actions). Every prereq enumeration is re-derived: the front matter (six prereqs; four relocate no code), the crate-PR count (**ten**, at both sites), §5.3's open-none sentence and its check-9 text (**ten** `opens` statements by owner — the published harvest command returns **eleven** tuples, and §5.3 keeps the two counts apart), §8's PR-1b dependency (two), the ordering and topology paragraphs (six branch from `main`, three pending), the sibling counts (five) and §9's review count (nine). ⚠ **None of the four carves carries a §10 row**; check 13's closed alternation is the *reason* for three of them, the predicate prereq — which the alternation does admit — carrying none anyway, and that is the precedent |
| A67 | user, 2026-09-23 (third and fourth decisions on this topic); scoped plan-review of rev 63c, and an independent reading of memo and code | the umbrella stating the **whole** end-of-line white-space domain's outcomes (rev 63's three attempts: "trimmed or hung"; removal plus a §8.2-bounded hang with cells 16(a)–(h) and a PR-1c border-box cell; the same, carved to a prereq while the umbrella kept the finer outcomes) — **and then handing the whole domain away**, which the fourth decision measured as an over-generalisation | each of the three attempts was reviewed and each review found its defects **in that revision's own new text** — three consecutive self-introduced resets (1 CRIT / 15 IMP; 1 CRIT / 4 IMP; 1 CRIT / ~14 IMP) — because those outcomes cannot be authored without choosing a mechanism, which is the one thing the umbrella must not choose. The hand-off that followed went too far: the defect it generalised from was **placement** (the withdrawn border-box arm asserted a PR-1c value inside a PR-1b cell), and for `white-space: normal` no mechanism has to be chosen at all — css-text-3 §4.1.2 step 3 removes the trailing collapsible space, full stop. **Final shape.** The umbrella keeps four things: the M3 amendment (a box marker does not make a trailing space non-line-final, whatever its edges); the **`normal` outcome** — step 3 removes the space, so it reaches neither the aligned line width nor the position of what follows (cells 16, 16b, under freeze discipline 5's main rule); the **defect**, as measured instances with no invariant; and the **ordering** (the prereq in `main` before PR-1b, not ordered against the min-content prereq). It hands over the **mechanism** and the **non-`normal` values** — `pre-wrap`'s hang and what css-text-3 §8.2 lets block it, `pre`, `break-spaces` — with the cells that would assert them (arms (c) and (e)–(h), withdrawn; the border-box-end cell, whose label `16c` is recorded as withdrawn at cell 16), M4's end-cursor sentence and the §2 3×7 clause, and M8's and §8's "a §8.2-blocked space counts in the trimmed width". The prereq's instance 4 is a **statement of a pre-existing disagreement**, not an invariant: `split_whitespace` drops every `char::is_whitespace` separator (U+00A0, U+3000) where layout's trim strips `' ' | '\t'` alone (`inline/whitespace.rs:162`), and `max_content_inline_size` measures `run.text` whole (`inline/measure.rs:52-58`). Freeze discipline 5 carries the exception, scoped to those two things |
| A68 | R30 | §8's end-of-line white-space block leaving the intrinsic/layout trailing-space mismatch **ownerless** — "pre-existing, stated rather than owned … each plan states what its change does to it" (rev 63's instance 4) — and, with it, the memo carrying **two** `A67` rows | an ownerless defect is a defect no PR closes: an `auto`-width inline-block still takes a used width from intrinsic sizes that disagree with the layout both prerequisites produce, because `max_content_inline_size` sums `measure_text(&run.text)` untrimmed (`inline/measure.rs:52-58` at `154bac3f`) while layout measures a segment by its trimmed width (`pack/mod.rs:690`) and css-text-3 §4.1.2 step 3 removes the trailing collapsible space at the line's end — and a max-content size is the width of a single unbroken line (css-sizing-3 §2.1), so its end is exactly that position. **No new slot**: this is the min-content prereq's own defect seen in the other pass, so that block gains instance **5** and its scope is stated — the name is historical, the PR owns the **segmentation and trimming of both intrinsic passes** (instances 2–5), the end-of-line white-space prereq owns the rule about which trailing spaces layout removes or hangs, and PR-1b owns the edge terms (**A58**). The block is **not** renamed: the label reaches the front matter, §3, §5.2, §5.3, §8 and §9, and a rename buys nothing the scope sentence does not. Swept at the sites that described its scope rather than naming it: §5.2's `measure.rs` row, §9's intrinsic bullet and §8's PR-1b clause; M8's Decision, frozen, still says "cross-item joining" — a subset, not a contradiction. ⚠ The duplicate `A67` row (rev 63 wrote the narrowed row and left the superseded one beside it) is deleted here; the surviving row is the one naming the third **and** fourth decisions. Found by Codex on #515 |
| A69 | R31-F1 | the end-of-line white-space and min-content prereqs as **not ordered against each other** (rev 63's carve, kept through rev 64) | rev 64 gave the min-content prereq the intrinsic passes' **trimming** while the end-of-line prereq owns the **rule** that trimming applies — for `pre`/`pre-wrap` and the non-ASCII separators especially — so with no order between them the min-content PR may implement an undecided rule, and the later PR then moves layout without it, recreating the mismatch the carve exists to remove (**A68**). **Ordered**: end-of-line white-space prereq → min-content prereq → PR-1b. The order is read off the ownership rev 64 already states rather than added to it — trimming applies a rule, so the rule lands first — and the alternative, one PR owning both, would re-merge two defects the memo carved apart (**A66**). Swept: both §8 prereq blocks' ordering lines, the topology paragraph's two sentences and §8's PR-1b dependency, which now names the pair in order. **A66's "not ordered against the min-content prereq" is superseded here**, being committed. Found by Codex on #515 |
| A70 | R31-F2 | §6 cell 25's single-word fixture (`<span style="padding:10px">x</span>`) as the memo's whole statement of the edge terms under shrink-to-fit | with one word there is one min-content segment, so the cell cannot say **where** a decorated inline's two edges land when its content has several: both edges on the widest segment, one edge on each adjacent segment, or the pair on every segment all give the same number on `x`. That is M8's territory and PR-1b's, not the handed-off white-space domain, so freeze discipline 5's main rule applies and the umbrella pins it. **Derived from the spec, not from a reading**: css-sizing-3 §2.1 takes "all soft wrap opportunities within the box", css-text-3 §5.5 adds none at the box's boundaries, and css-break-3 §5.4's initial `box-decoration-break: slice` inserts "no border and no padding … at a break" — so the inline-start edge sits on the first segment and the inline-end edge on the last. New PR-1b cell **25c** asserts `max(10 + |a|, |verylongword|, |b| + 10)` as a relation over `measure_width`, with the long word asserted widest as its precondition, and names the two readings it rejects; §6, §5.3 and §8 route it and the freeze's cell figure goes **62 → 63**. Found by Codex on #515 |
| A71 | R32-F1 | the matrix's coverage of `border` in the **inline-axis advance** and in **both intrinsic sizes** — measured, there was none | cell 2 stops at marker emission and the existence flip, 13c asserts the separate `LayoutBox.border` field (PR-1c), §8's end-to-end clause reads `LayoutBox.border == 5` without an advance or an intrinsic size, and every M8 cell (25, 25b, 25c) is padding — so a producer that drops `border` from M3's inline-axis sum and M8's edge term while M1, M5 and M4 keep it passes the whole matrix, leaving following text over the border and shrink-to-fit 20px narrow. Closed by new PR-1b cell **3c**, two markups of its own — `b` displaced 20px, and both intrinsic sizes 20px wider than the same box without the border — stated as relations over the fixtures, not literals, with the resolved `EdgeSizes` set on `ComputedStyle` as cell 2's ⚠ establishes. Routed in §6, §5.3 and §8; the freeze's cell figure moves with **A72**. Found by Codex on #515 |
| A72 | R32-F2 | M8 and cells 25/25b/25c leaving an edge-only box's **negative** contribution unnormalised — `<span style="margin:-10px">` in an `auto`-width inline-block gives `−20`, which `shrink_to_fit_width` would propagate as a containing width | the spec settles **where** the floor sits, so the umbrella pins it rather than handing it over: css-sizing-3 §2.2 floors a box's max-content contribution by its min-content contribution, "e.g. due to the use of negative margins"; §5.2's Note floors the min-content contribution "by the minimum size in its own axis"; and for this `inline-block` that minimum is `min-width: auto`, which §3.2 gives "a used value of 0". So the floor belongs to **the box whose size is being computed**, and **nothing clamps the inner inline box's own −20** — a producer clamping per marker is wrong in the other direction. New PR-1b cell **25d** asserts both intrinsic sizes non-negative (0 on that markup) and the used width 0, naming the consumer it observes — `elidex-layout/src/intrinsic/mod.rs:134` through `layout/mod.rs:57`, **not** the `positioned/constraints.rs:318` homonym, which reads max-content alone. How the floor is applied is PR-1b's. Found by Codex on #515 |
| A73 | R32-F3 | §6 cell 13d as an `elidex-render` test on that crate's `consumes_relpos_inline_subflow_with_gap` harness | that harness **hand-sets the very `LayoutBox` padding and border PR-1c produces**, so the cell passed before PR-1c and could not catch a positioned inline omitted from marker emission or from the edge assignment — the defect it exists for; 13c covers a static span's fields and §8's end-to-end clause a static `border:5px solid` without looking at painted output, so nothing else reached it. **Re-stated against the real producer**: the markup runs through `elidex-shell`'s pipeline, which drives HTML + CSS to a display list (`crates/shell/elidex-shell/src/tests.rs:41`) and already matches `DisplayItem::SolidRect` (`:127`, both verified at `154bac3f`), and the cell asserts the painted geometry as a relation over the `LayoutBox` layout produced — background at `border_box()`, four border segments of `LayoutBox.border` thickness, five rects where the pipeline emits one today. The crate row in §5.2 moves with it: `elidex-render` tests are now a negative row **without exception**, and `elidex-shell` is also the only site where a producer and the display list are jointly observable. A6 is untouched — the cell still adds no `elidex-render` pass or member. Found by Codex on #515 |

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

`body css-text-3 boundary-shaping` — **three** normative sentences, of which this memo quoted only
the first until R15 and counted as two until R20 (`body css-text-3 boundary-shaping`, lines 9 /
39 / 63):

> Text shaping must be broken at inline box boundaries when any of the following are true for any
> box whose boundary separates the two typographic character units: … Any of
> margin/border/padding separating the two typographic character units in the **inline axis** is
> non-zero. … vertical-align is not its initial value. … The boundary is a bidi isolation
> boundary.

> Text shaping **must not** be broken across inline box boundaries when there is no effective
> change in formatting, or if the only formatting changes do not affect the glyphs (as in applying
> text decoration).

> Text shaping **should not** be broken across inline box boundaries **otherwise**, if it is
> reasonable and possible for that case given the limitations of the font technology.

So a decorated boundary **must break** shaping. elidex currently coalesces same-entity text
across such a boundary (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:744`), which
is correct only while inline decoration takes no space — i.e. only until PR-1b.
⚠ **That sentence is narrower than it reads, and the narrowing is what makes the *other* half the
live one** (R15, prompted by a reviewer finding that read the first sentence's three triggers as
three open gaps). A text run carries the **parent element**'s entity, not the text node's —
`collect.rs:309` pushes `StyledRun::from_style(parent_entity, …)`, and only the pseudo arm
(`:263`) pushes the child — so an inline box that places **any** member changes the entity across
its boundary, `:744`'s `coalesce` is false there by construction, and the file's own comment says
so ("A different-entity … segment starts a fresh run", `:727-735`). The same-entity meeting that
the sentence above is about therefore survives in exactly one shape — the box places **nothing**,
so the two texts around it stay contiguous: `<p>a<span style="padding:1px"></span>b</p>`, cells 14
and 15's markup, where PR-1b's marker is what ends the coalescing. ⚠⚠ **The narrowing does *not* dispose of the other two triggers,
and R15's first reading of it did** (corrected in the same round). With text on both sides a
`vertical-align` or bidi-isolation boundary is an entity change too, so elidex breaks there — by
over-breaking, not by implementing the trigger. But **the member-less corner is the same corner
for all three triggers, and only the first one closes in it**:
`<p>a<span style="vertical-align:super"></span>b</p>` has both texts under `<p>`, and M1 emits
**no** marker because the span carries no non-zero edge, so the two runs coalesce and no break
occurs — trigger 2 violated, after PR-1d as much as today. Trigger 3 is the same shape with
`unicode-bidi: isolate`. Measured at `154bac3f`: nothing in the inline layer reads either property
(`git grep -nE 'vertical_align|unicode_bidi|Isolate' 154bac3f --
crates/layout/elidex-layout-block/src/inline/` is **empty**), and the run-break state has exactly
**three** write sites — the constructor (`pack/mod.rs:190`), `flush_line`'s reset (`:439`) and
`place_item`'s set (`:772`) — so
entity identity is the sole discriminator. ⚠ **"Two write sites", excluding the constructor,
until R22**: §4's parallel enumeration for `any_rendered_content` names *its* constructor site
(`:184`) among the writes, so the two counts were taken under different conventions. The
convention is §4's — a write is any assignment to the field, the initialiser included — and both
sites here now state it that way; the conclusion is untouched, since an initialiser cannot end a
run either. ⚠ Widening M1 to every inline box
(`#11-inline-zero-edge-box-in-item-stream`) is a **prerequisite, not the fix**: the marker would
exist, but M3's shaping break keys on `has_inline_axis_edge`, which a zero-edge box fails by
definition. Triggers 2 and 3 in that corner therefore get their own slot,
**`#11-shaping-break-vertical-align-and-isolation`** (§5.3, §8, §10). Ledger **A43**.

⚠ **The second sentence is the divergence, and it is engine-wide.** Both measurement and shaping
are per-run: `measure_text` is called on one `StyledRun`'s whole text (`inline/measure.rs:57`) or
on one of its segments (`:78`), and render shapes one `InlineFlowRun::Text` per `rustybuzz` call
(`builder/inline_flow.rs:107-124` → `elidex-shaping/src/shaping.rs:133`).
⚠ **What `:107-124` is, stated once here and inherited by every other site that names it**
(R22): it is the per-line loop that **collects** the line's `Text` runs and its atomics into two
buckets — it calls nothing in `elidex-shaping`. The `rustybuzz` call is reached from
`emit_flow_text_run` (`builder/inline_flow.rs:232-284`), whose `emit_text_segment` at `:272`
(`emit_vertical_text_segment` at `:262` in the vertical arm) is the last render-side
step before `shaping.rs:133`. The `→` in every citation of this shape is that chain, one run at a
time; the range is the enumeration, not the call. So
`<p>a<span>x</span>c</p>` — a span that changes **no** formatting at all, and the "as in applying
text decoration" case beside it — is shaped as three runs where §7.3 requires one. Nothing in this
program creates the gap or narrows it: the entity split is `collect.rs`'s, it predates every
marker, and PR-1b's break runs in the *first* sentence's direction.
**`#11-shaping-break-at-unchanged-inline-boundary`** (§5.3, §8, §10), **pre-existing** class.
Ledger **A41**.

⚠ **The third sentence is a third divergence, and the undercount is what kept it unread from R15
to R20.** It is the *otherwise* case: a boundary where formatting **does** effectively change the
glyphs, which neither sentence above reaches — the first names three triggers and this is none of
them, the second's predicate is *no* effective change and this is its complement — and §7.3 gives
it a `should not` rather than a `must not`. elidex breaks there for the same reason it breaks
everywhere, the entity split at `collect.rs:309`, and the case arrives with **zero author CSS**
because the UA sheet supplies the formatting change: `crates/css/elidex-style/src/ua.rs:117`
ships `b, strong { font-weight: bolder; }`, a text run carries the **parent element**'s entity,
and `pack/mod.rs:744`'s `coalesce` is entity-equality — so `<p>نو<b>شتن</b></p>` is two runs and
two `rustybuzz` calls (`builder/inline_flow.rs:107-124` → `elidex-shaping/src/shaping.rs:133`),
and §7.3's own worked example of what is "reasonable and possible" is Arabic shaping across a
font change, for which the section names the U+200D / U+200C workaround rather than leaving the
term open. ⚠ `#11-shaping-break-at-unchanged-inline-boundary` is sentence (b)'s and does **not**
reach this: the predicates are complements, and so are the two dispositions — sentence (b)'s
divergence is that two runs exist where one should, this one's is that two runs are shaped
without the joining context that would make them read as one. **The evidence stops at the
structure and the UA rule; no Arabic fixture was rendered end to end**, the same limit §3's
css-text-3 §5.5 intra-word row states for its own probe.
**`#11-shaping-across-formatting-change-boundary`** (§3, §9, §10), **pre-existing** class and
engine-wide, untouched by this program. Ledger **A49**.

### §1.5 Height and baseline — `css-inline-3` §5.3

`body css-inline-3 inline-height`:

> If the inline box contains no glyphs at all, **or if it contains only glyphs from fallback
> fonts**, it is considered to contain a "strut" (an invisible glyph of zero width) with the
> metrics of the box's first available font.

with `A′ = A + L/2`, `L = line-height − (A + D)` — the same half-leading arithmetic elidex
already applies at `crates/layout/elidex-layout-block/src/inline/pack/mod.rs:581`. The condition
is broader than CSS 2 §10.8.1's ("no glyphs at all"), and the *root inline box* gets special
treatment in the same section.

⚠ **That arithmetic is one of §5.3's *two* branches, and this memo carried it as the whole rule
until R20** — §3 has five rows citing §5.3 and none of them distinguishes the two. The section
splits on the computed `line-height`:

> When its computed `line-height` is **normal**, the layout bounds of an inline box encloses
> **all its glyphs**, going from the highest A to the deepest D. (Note that glyphs in a single
> box can come from different fonts and thus might not all have the same A and D.)

> When its computed `line-height` is **not** normal, its layout bounds are derived solely from
> metrics of its **first available font** (ignoring glyphs from other fonts), and leading is used
> to adjust the effective A and D to add up to the used `line-height`. Calculate the leading L as
> `L = line-height - (A + D)`. …

`normal` is `line-height`'s **initial** value and survives to the computed value
(`css css-inline-3 line-height` → `initial: normal`, `computedValue: the specified keyword, a
number, or a computed <length> value`), so the first branch is the one a document with no author
`line-height` is in. elidex is in the second for both: `LineHeight::Normal` collapses to
`font_size * 1.2` at `crates/core/elidex-plugin/src/computed_style/text.rs:136`, reached from
`inline/styled_run.rs:96`, and the packer then runs first-available-font metrics plus
half-leading (`:581`) with no glyph provenance in the input at all. Two consequences, neither of
them this program's doing: under `normal` the bounds are a fixed 1.2 ratio of the font size
rather than the font's own A and D, and they cannot span fonts — which is what that branch's
parenthetical is about. It is **upstream of M6's `block_advance` and M7's strut**, both of which
read the already-collapsed `line_height`. ⚠ It is **not** a sixth facet of
`#11-inline-root-inline-box`: that slot's five facets are each pinned by a §6 cell and it records
in its own text that none of them needs **per-glyph** provenance, which is the one thing the
`normal` branch turns on — and that provenance line is where R22 put the **boundary** between the
two slots as well, the sibling's scope and trigger narrowing to the *composition* so the pair no
longer shares one (§5.3, §9). **`#11-inline-height-normal-layout-bounds`** (§3, §9, §10),
**pre-existing** class. Ledger **A49**.

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
| 5 | **Line-box height composition** — `css-inline-3` §5.3 layout bounds, incl. the root inline box; the composition is `#11-inline-root-inline-box`'s, in five facets (the `normal` branch's per-box bounds are `#11-inline-height-normal-layout-bounds`'s — R22's scope split, §5.3) | `inline/pack/mod.rs:696` (the scalar `max`), `:493` (the committed rect's block extent), `:539-543` (the vertical block-advance source), `:586` (first-wins baseline) |
| 6 | **Group keying** — relpos/sticky sub-flows | `inline/collect.rs:286` |
| 7 | **Cursor/advance integrity** — wrap, trailing hang, shaping runs, intrinsic size | `inline/pack/mod.rs:690`, `:701`, `:772`; `inline/measure.rs:15`, `:44` |

| Pair | Coupling | Resolved in | PR |
|---|---|---|---|
| 1 × 2 | **Which** inlines emit markers decides which boxes clause 3 can see at all: a box with no marker cannot keep a line (the zero-edge case is `#11-inline-zero-edge-box-in-item-stream`'s). The emit set bounds every other pair of 2 too — each such row below names its own intersection. | M1 | 1a |
| 1 × 2 | A marker between two text runs must not become a collapse barrier. | M2 | 1a |
| 7 × 2 | The box's edges take space but its boundary is not a wrap opportunity; and a decorated boundary must break shaping. | M3 | 1b |
| 3 × 7 | The box's rect must be a *content* span, with edges carried separately, or the border box double-counts. | M4 | 1c |
| 1 × 3 | Flipping existence moves whole lines from discard to commit, affecting **every** entity on the line. | M5 | 1d |
| 1 × 5 | A line kept only by decoration needs a height source, and elidex has no root inline box. | M6 | 1d |
| 1 × 4 | `css-inline-3` §5.3 gives a strut only to a glyphless box, so the baseline source depends on the whole line. | M7 | 1d |
| 1 × 7 | One inline-axis predicate is read by clause 3 (existence) and by the advance side's shaping break. | M5 | 1b |
| 1 × 7 | A marker entering a line must register in the line's occupancy but not as *content* for the soft-wrap guard (`LineOccupancy::BoxEdgeOnly`; cell 15b). | M3 | 1b |
| 2 × 3 | Markers are the box rect's producer; a marker's rect commits or discards with its line, never outside that decision (cell 6's phantom sub-cell; M4 invariant (v)). | M4 | 1c |
| 2 × 4 | The marker payload must carry the box's font fields for the glyphless strut (dead-field rule: from PR-1d). | M7 | 1d |
| 2 × 5 | The marker payload must carry the box's `line_height` for a decoration-kept line's height. | M6 | 1d |
| 3 × 4 | A marker's tentative strut baseline must be discarded with a discarded line and promoted only on a committed one (the flush reset clears it). | M7 | 1d |
| 3 × 5 | A marker's `block_advance` raises the line's height, which the committed rects take as their block extent (cell 13b) and a discard resets. | M6 | 1d |
| 4 × 5 | Baseline and height compose per box in `css-inline-3` §5.3; elidex's first-wins baseline (`:586`) and scalar `max` height (`:696`) do not — pre-existing, pinned by cells 24c/24d. | `#11-inline-root-inline-box` | `#11-inline-root-inline-box` |
| 4 × 7 | M7's `RenderedText` rung is a state of M3's occupancy ordering, which the soft-wrap guard also reads — both readers are order tests, never equality (cell 15d). | M3 | 1b |
| 5 × 7 | `note_line_occupancy` is the one writer of both the advance and the line height, the marker path and the flush replay included. | M3 | 1b |
| 1 × 4 (fallback-only half) | The same `css-inline-3` §5.3 sentence's **second** condition — "or if it contains only glyphs from fallback fonts" — is the same coupling over a signal elidex does not have at all. ⚠ Rev 33 carved this half out and swept §3, §5.3, §8 and §10; **§2 was missed**, and §5.3 says slicing has "one owning PR per coupling (**§2**)", so §2 is the authority a per-PR memo derives its DoD from — the row is added here rather than left implied (round 26, Axis 5). The idiom is row `2 × 6`'s: the slot is named in both owner columns. | `#11-inline-fallback-font-strut` | `#11-inline-fallback-font-strut` |
| 2 × 6 | Markers carry the enclosing recursion level's `group_key` — read only by the split rule, so the field arrives with its reader (M1 does not carry it in PR-1a). | `#11-inline-box-decoration-splits` | `#11-inline-box-decoration-splits` |
| 7 × (intrinsic) | An inline-axis advance the packer applies must also be visible to the intrinsic-size passes, which never build a `LinePacker`. | M8 | 1b |

Each **row** is answered by exactly one M-row **or by exactly one slot**: of the **twenty** rows
above, seventeen route to M1–M8 and **three** — `1 × 4 (fallback-only half)`, `2 × 6` and `4 × 5` —
route to a slot instead, each naming the same slot in both columns. Three pairs carry two rows, one
per distinct intersection: `1 × 2` (M1's emit set, M2's collapse barrier), `1 × 4` (M7, the
fallback-only slot) and `1 × 7` (M5's shared predicate, M3's occupancy). The five uncoupled pairs
are the table after the next paragraph, each with its reason; the twenty rows are sixteen distinct
invariant pairs, three second rows and `7 × (intrinsic)`, which pairs 7 with the intrinsic passes
rather than with an invariant — 16 + 5 = 21 (ledger **A65**). ⚠ **This sentence
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

| Uncoupled pair | Why no coupling |
|---|---|
| 1 × 6 | Markers never become `FlowMember`s (M1), so an existence flip adds no member to any group bucket, and member-less buckets are dropped by design (§7). |
| 3 × 6 | Rects commit per entity — a marker's rect included (`2 × 3`) — and runs per group bucket; markers push no run into any bucket (M1), so a group's commit or discard does not depend on a marker. |
| 4 × 6 | The baseline is IFC-wide first-wins, `first_baseline.is_none()` over placed text segments (`inline/pack/mod.rs:570-587`), whatever their group. |
| 5 × 6 | The line height is per line (`:696`'s `max`), group-independent. |
| 6 × 7 | The cursor is one per line across groups — `group_key` buckets runs and never positions them — so an edge advance reaches every group alike. The one group-keyed reader on the advance side, `top_group_is_line_last` (`inline/pack/mod.rs:264-275`), decides a trailing space's line-finality for justification from the groups' run positions; markers add no run, so it reads the same runs with or without them, and line-finality across markers is M3's outcome. Keying the markers themselves into a relpos sub-flow is `2 × 6`'s slot. |

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

⚠ **What the `Full enum?` column means here, stated because the schema's question and this memo's
answer are not the same question** (R12). The column is the one
`.claude/skills/elidex-plan-review` Pre-condition #1 mandates, and its purpose *there* is catching
*spec-step branch under-enumeration* — an **enumeration-completeness** audit. This memo has used
it instead as a **conformance-and-routing** mark and, until rev 47, nowhere said so:
`git grep -c "Full enum" 03393e14 -- docs/plans/2026-08-line-box-decorated-inline-content.md`
→ **1**, the header row. ⚠ **The commit is named in the command for a reason**: this paragraph
and the ledger row recording it are themselves hits, so the same grep against the working tree
answers a different question than the one that found the defect
([[feedback_document-landing-invalidates-its-own-measurements]]). The operative reading is
— **`✓`**: the branch is enumerated, and elidex either conforms **today** or the behaviour is
**delivered by this program** at the PR or M-row the Touch names; **`✗`**: enumerated, and
*routed* — to a `#11-` slot, or as a deliberate divergence naming §9. ⚠ **That reading is
executable rather than prose**, which is the only kind this memo trusts
([[feedback_prose-rules-cannot-fix-unexecuted-claims]]): `plan-xcheck.py` check 4
(`.claude/tools/plan-xcheck.py:289-302` at `4394af4c`) fails a row whose Touch names a `#11-` slot while the
mark starts `✓` (`:293`), and a `✗` whose Touch names no slot, no `PR-`, no defined `M<n>` and no §9
(`:301-302`). That second rule is also why no row below can be read as the *enumeration*
question: under that reading a `✗` would mean this memo had failed to enumerate the branch,
whereas the checker requires every `✗` to name an **owner**. ⚠ **Where the next defect lands is
what check 4 cannot see**: it tests *where the Touch points*, never *whether what it points at
delivers the quoted clause*. A `✓` whose Touch names a PR that does not in fact deliver the clause
passes check 4 and is still false — the css-text-3 §5.5 *intra-word shaping* row and the
css-writing-modes-4 §6.4 row below were exactly that until R12, and the checker found neither.
⚠ **No column is added for the schema's own question**: `preflight.py` validates the header
against a fixed six-name `EXPECTED_COLUMNS` list (`:137`) and `plan-xcheck.py` reads
`c[3]`/`c[4]` positionally, and the schema is mandated for every plan-memo in the repo — widening
it is the plan-checker tooling task's (§9), not this memo's. Ledger **A34**.

⚠ **The schema's own question was therefore never asked of this table until R20, and §9 carries
the answer.** Because the column is used for conformance-and-routing, no mark in it has ever been
an enumeration-completeness verdict. R20 ran that audit by hand over the table's **25 distinct
spec sections** — the audit's unit, and not the 42 rows the table then carried, the two parting
wherever one section carries several rows:
`git show 09026f98:<memo> | sed -n '/^| Spec section/,/^$/p' | cut -d'|' -f2 | grep -Ev '^$|^-+$|^ Spec section $' | sort -u | wc -l`
→ **25** (⚠ the commit is in the command because this revision's own six new rows move the figure
to 26 against the working tree — [[feedback_document-landing-invalidates-its-own-measurements]]).
It found **nine** reachable branches carrying no row, no §6 cell, no slot and no disposition;
**two** of the nine are reachable with zero author CSS, and **three** are rows that quoted a
*truncated* normative sentence and then marked against the fragment quoted. Six are the rows
added above and three are corrections to rows that were already here. §9's audit bullet states
the standard used for "covered", the yield, and that the audit is a one-time manual discharge;
**mechanical closure remains the plan-checker tooling task's clause-case check** (§9).
⚠ **"No completeness audit at all" would be too strong, and is not the claim made here**: §1.2's
clause table is a real 5-of-5 enumeration of css-inline-3 §2.3, and the schema's own remedy for
an un-enumerated branch — implement it upfront **or** name a defer slot
(`feedback_plan-scope-re-evaluation.md:44`, the file `.claude/skills/elidex-plan-review` SKILL.md
cites for Pre-condition #1) — coincides with what this column's `✗` already means. What was
absent is a **systematic** pass, and the loss is confined to branches the table never *listed*.
Ledger **A50**.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS Inline 3 §2 Inline Layout Model | inline-axis edges respected between boxes | start/end edge advance | M3 shared core (`note_line_occupancy`, NEW) — **PR-1b** | ✓ | yes |
| CSS Inline 3 §2 Inline Layout Model | root inline box | block container's anonymous inline box | **NOT implemented — M6, `#11-inline-root-inline-box`** | ✗ (pre-existing, disclosed) | yes |
| CSS Inline 3 §2.1 Layout of Line Boxes | box exceeding the line, or containing a forced break | split into fragments across line boxes | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS Inline 3 §2.1 Layout of Line Boxes | the line box's logical width | "floating boxes or initial letter boxes can come between the containing block edge and the line box edge, reducing the space available to, and thus the logical width of, any such impacted line boxes" (`body css-inline-3 line-boxes`; the section cross-references CSS 2 §9.4.2 and CSS 2 §9.5 for it) | ⚠ **no line box in this engine is ever narrowed by a float, and the cause is at the IFC's entry point rather than in the packer.** `flush_inline_run` (`crates/layout/elidex-layout-block/src/block/children/helpers.rs:37`) computes the IFC's `inline_size` as `input.containing.width` — the whole containing width, `:54-58` — and hands it to `layout_inline_context_fragmented` (`:79-87`) with no float argument of any kind; `git grep -n float_ctx 154bac3f -- crates/layout/elidex-layout-block/src/inline/` returns **eight** hits, seven under `inline/tests/` and the one production hit `atomic.rs:52` being the literal `None`, and `FloatContext` probed on its own returns nothing there. ⚠ **The audit's own reachability caveat, kept because it is the part that is easy to get backwards**: a float *sibling* blockifies (`resolve/mod.rs:185`), so `children_are_block` is true and `stack_block_children` wraps the inline runs in anonymous blocks — but that is not what removes the band, since `flush_inline_run` is the same entry point for those anonymous blocks and passes the same un-narrowed width. The divergence is structural rather than a wrong number on one line. **Pre-existing** and engine-wide, and named here because M3's advance and M4's geometry are computed against that width — **`#11-line-box-float-narrowing`** (§9, §10) | ✗ (pre-existing → slot) | yes |
| CSS 2 §9.4.2 Inline formatting contexts | box that **cannot** be split | overflows the line box | M3 — no wrap check on the marker path; §6 cell 15c (PR-1c; cell 15 asserts the advance) | ✓ | yes |
| CSS Break 3 §5.4 Fragmented Borders and Backgrounds: the `box-decoration-break` property | `slice` (initial) and `clone` | verbatim (apostrophes are the source's): "For inline elements, which side of a fragment is considered the broken edge is determined by the parent element’s inline progression direction. … (Note in particular that neither the element’s own direction nor its containing block’s direction is used.)" — ⚠ **the `…` was unidentified under a "verbatim" label until R22**, against this memo's own convention two rows down (css-display-3 §A, where the elision is named); it drops the section's worked example, "For example, if an inline element whose parent has `direction: rtl` breaks across two lines, the left edge of the fragment on the first line will be the broken edge.", and the trailing "See [CSS3-WRITING-MODES]." — and the section extends itself to two further break kinds in **one** sentence, quoted whole at R20 after the memo had carried its first clause alone: "UAs should also apply `box-decoration-break` to control rendering at bidi-imposed breaks — i.e. when bidi reordering causes an inline to split into non-contiguous fragments — **and/or at display-type–imposed breaks — i.e. when a higher-level display type (such as a block-level box / column spanner) splits an incompatible ancestor (such as an inline box / block container). Otherwise such breaks must be handled as `slice`**". ⚠ **The display-type half is reachable on markup §5.1 M1 already measures for another purpose**: `children_are_block` (`crates/layout/elidex-layout-block/src/block/mod.rs:70`) tests **direct** children only, so `<p>a<span><div style="padding:10px"></div></span>b</p>` takes the IFC path. No destination of its own is opened for it, and the reason is that both halves are already owned: the split that would produce the fragments is `#11-block-in-inline-anonymous-block-split`'s (§9), and decoration at a fragment's broken edges is this row's own slot's. The trailing "Otherwise … `slice`" clause is the one part elidex satisfies today, `slice` being the initial value and `clone` having no implementation to diverge into. ⚠ "no visual effect where the split occurs" is **CSS 2 §9.4.2**'s sentence and occurs nowhere in css-break-3 (checked across every css-break-3 §5.x anchor) | **`#11-inline-box-decoration-splits`** — the source of a fragment's edge attribution differs from M1's own-direction side mapping, so it belongs with the rule that owns it | ✗ (deliberate, §5.3) | yes |
| CSS Inline 3 §2.2 Layout Within Line Boxes | Note on empty inline boxes | they still have a line-height and influence the calculation | M6 — **PR-1d** for a **decorated** empty inline box; a **zero-edge** empty inline (`line-height` only) gets no marker — M1's non-zero-edge conjunct, a slice boundary kept for its presence-change ground — so that case is this program's **own** deferral, **`#11-inline-zero-edge-box-in-item-stream`** (§5.3). ⚠ **And M6 delivers the Note only in a horizontal writing mode** (Codex R2-F2): in a vertical one `block_advance` is `font_size` (`inline/pack/mod.rs:539-543`), so the box's `line-height` never reaches the calculation the Note is about — `#11-inline-root-inline-box` facet (c), pinned by cell 23b | ✗ (deliberate slice boundary: decorated ✓ via M6 **horizontally**, vertical → root-inline-box slot, zero-edge → its own slot; Codex on #515, Codex R2) | yes |
| CSS Inline 3 §2.2 Layout Within Line Boxes | step 1, *Baseline Alignment* | "All in-flow inline-level boxes in the line box are aligned to each other in the block axis according to `dominant-baseline` and `vertical-align`" — the step the other three run after | ⚠ **not performed in the IFC, and the row above covers only the section's Note.** `vertical-align` parses and is stored (`crates/core/elidex-plugin/src/computed_style/mod.rs:362`) and the table layer consumes it (`crates/layout/elidex-layout-table/src/lib.rs`), but `git grep -n vertical_align 154bac3f -- crates/layout/elidex-layout-block/src/inline/` is **empty**, and so is the same probe for `VerticalAlign`; the line's baseline is instead `first_baseline`, captured first-wins from the first text run (`inline/pack/mod.rs:125`, written at `:586`). One author declaration reaches it — `<p>a<span style="vertical-align:super">b</span></p>` — and `dominant-baseline` has no elidex selector at all, which the css-inline-3 §5.3 inflation row measures by the property. **Pre-existing**, and upstream of steps 2–4 and so of M6 and M7 — **`#11-inline-baseline-alignment`** (§9, §10). ⚠ Distinct from `#11-shaping-break-vertical-align-and-isolation`, which owns the same property as one of css-text-3 §7.3's *shaping* triggers and aligns nothing | ✗ (pre-existing → slot) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence (a) | zero-height for positioning descendant content (abspos) | `static_positions` (`inline/pack/mod.rs:97`) — **PR-1d** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | layout-bounds inflation by edges | applies only when `line-fit-edge` ≠ `leading`; initial is `leading` | no code touch; grounds §1.2's block-axis exclusion. ⚠ **The `✓` is vacuous, and R12 writes down what makes it so** rather than leaving the next reader to re-derive it — that re-derivation is the class that produced the css-writing-modes-4 §3.2 row's flip. **elidex has no selector for the branch at all**: `crates/core/elidex-plugin/src/computed_style/mod.rs` carries **122** `pub <name>:` declarations (`git grep -cE '^[[:space:]]*pub [a-z_0-9]+:' 154bac3f -- crates/core/elidex-plugin/src/computed_style/mod.rs`) and none of them is a layout-bounds edge selector — measured by the *property* rather than by the word, `git grep -nEi` over `crates/` whole for `line-fit-edge`, `text-box-edge`, `text-box-trim`, `leading-trim`, `baseline-source`, `dominant-baseline`, `alignment-baseline` and `inline-sizing` (underscored spellings too) returns **nothing**, so the complement is empty and not merely unexamined. The property's initial value is the one that makes the clause not apply (`webref css css-inline-3 line-fit-edge` → `initial: leading`), so an engine that cannot express any other value conforms by construction — and it stays vacuous through this program, which adds no property | ✓ (vacuous — the branch has no elidex selector) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | the composition itself | each inline box contributes **its own** layout bounds, and the resulting line extent is **not** assigned back to its children | **✗ (pre-existing, disclosed)** — elidex keeps a scalar `current_line_height = max(block_advance)` (`inline/pack/mod.rs:696`) and a first-wins `first_baseline` (`:125`), with no per-box `A′`/`D′`. All five facets are owned by the widened `#11-inline-root-inline-box` (§5.3's slot entry) and pinned by cells 23, 23b, 13b, 24c and 24d. ⚠ Added by Codex R2 — the four css-inline-3 §5.3 rows above cover the inflation clause, the strut conditions, half-leading and the Quirks rule, but none covered the composition the divergences are *against* | ✗ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 3 | non-zero **inline-axis** margin/padding/border | clause-3 predicate → `has_inline_axis_edge` (M5); the predicate lands in **PR-1b** with its M3 consumers, **PR-1d** substitutes it for the constant | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 1 | no text | `contributes_content` (`inline/pack/mod.rs:556`) is implemented and is font-**independent**; what stands in front of it is `inline/mod.rs:200`'s measurability gate, which returns `line_count: 0` for any IFC whose text resolves no font, so a clause-1 line can be lost for a reason clause 1 does not name — **`#11-inline-fontless-measurability-gate`** (§5.3), pinned by §6 cell 12d on both sides. ⚠ Added at R5: this table carried rows for clauses 3 and 5 and none for clause 1, though §1.2's table numbers all five and cells 12d and 24 both turn on this one | ✗ (pre-existing → slot; the predicate is implemented, the gate in front of it diverges) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 5 | forced line break | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence | line box *and its in-flow content* do not exist | commit/discard seam (`inline/pack/mod.rs:210` vs `:423`) — **PR-1d** | ✓ | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | `getClientRects()` step 3, the **one-line** case | one `DOMRect` "describing its **border area**" — padding + border, never margin | the `border_box()` fallback (`element/layout_query.rs:237`), correct once M4's edges are real — **PR-1c** | ✓ for a box with **one fragment on one line**; the residue is the row below | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | `getClientRects()` step 3, one `DOMRect` **per box fragment** in content order | ⚠ **the engine has no fragment unit at all.** `commit_aligned_entity_rects` folds to one entry per entity per **line** (`pack/mod.rs:479-487`) and `boxes.rs:102` gates on `line_rects.len() > 1`, so *line* is the only partition it can express; and runs persist in **logical** order, the UAX #9 L2 reorder being render's, not layout's (`inline/mod.rs:215-217`, `collect.rs:303-305`). So a **multi-line** box answers one content span per line (edges missing), and a box the spec fragments **within one line** — css-inline-3 §2.1's Note, "Inline boxes can also be split into several fragments within the same line box due to bidirectional text processing" — answers with *one* rect where the step requires one per fragment | **`#11-inline-box-decoration-splits`**, which owns css-break-3 §5.4 whole, and css-break-3 §5.4 names this case itself ("bidi-imposed breaks — i.e. when bidi reordering causes an inline to split into non-contiguous fragments"). Both halves are **pre-existing**: the count is already wrong on `154bac3f` and no PR here changes it. §6 cell 17d pins the multi-line half; `pack/mod.rs:445-446`'s "one border-box fragment per line …" docstring (the elided tail is "per inline element, in painted coordinates" — §3.1 quotes it in full) — which recurs at `boxes.rs:90-91` and four further sites — is `#11-inline-spec-cite-misattribution`'s (§9) | ✗ (deliberate, §5.3) | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | *get the bounding box* (`#element-get-the-bounding-box`), which `getBoundingClientRect()` returns the result of | step 1 invokes `getClientRects()`; step 4 returns "the smallest rectangle that includes all of the rectangles in list **of which the height or width is not zero**" (step 2 = zeros for an empty list, step 3 = **the first** rect when all are zero-area) | ⚠ elidex derives it from `LayoutBox.border_box()` and **never invokes `getClientRects()`** (`element/layout_query.rs:26-31` → `get_border_box`), so the two derivations are independent — pre-existing. PR-1c makes them *disagree*, at the broken edges only; §6 cell 17f pins it — **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSSOM View 1 §6 Extensions to the `Element` Interface | `clientTop` / `clientLeft` / `clientWidth` / `clientHeight`, step 1 | "If the element has no associated box **or if the box is inline, return zero**" — verbatim, and **identical across all four members** (`body cssom-view-1 dom-element-clienttop`) | ⚠ **step 1 is unimplemented for all four.** `clientTop` / `clientLeft` (`element/layout_query.rs:133-151`) read `lb.border.top` / `lb.border.left` as **direct field reads** and return zero today only because `inline/pack/boxes.rs:82-84` hard-codes `EdgeSizes::default()` — the lines M4 replaces — so PR-1c would turn an accidental correctness into a violation. `clientWidth` / `clientHeight` (`:119-131`) read `get_padding_box` (`:346`) and therefore **already violate step 1 today**, before this program. **Carved out of this umbrella into the predicate prereq PR** (§9), which must land **before PR-1a** — its predicate has two consumers, these four members and M1's emit test (§6 cell 6c). Why it is not a cell here: the predicate is css-display-3 §A Glossary's *inline box* — "A non-replaced inline-level box whose inner display type is flow. …" — the elision marks §A's second sentence, "The contents of an inline box participate in the same inline formatting context as the inline box itself", dropped unmarked at every site until rev 34 (round 26, Axis 4) — i.e. **two** inputs, and elidex answers only one (`is_atomic_inline`, `inline/collect.rs:14`, matches `InlineBlock`/`InlineFlex`/`InlineGrid`/`InlineTable` and **not** the replaced half of css-display-3's *atomic inline*). A guard keyed on `Display::Inline` alone would be a second, wrong answer to a question `collect.rs` already answers | ✗ (deliberate, §9) | yes |
| Resize Observer 1 §3.3.1 *content rect* | *Watching content rect means that:* — the list's third bullet | "non-replaced inline Elements will always have an **empty** content rect" (`dfn resize-observer-1 "content rect"` → §3.3.1 `#content-rect`; `body resize-observer-1 content-rect-h`) | ⚠ **unimplemented, and already violated on `154bac3f`.** The observer's size source is `LayoutBox::content_rect_local` (`crates/script/elidex-js/src/vm/host/resize_observer.rs:404-407` → `crates/core/elidex-plugin/src/layout_types/boxes.rs:204-211`), a total function of `padding` and `content.size` with no inline special case, and an inline `LayoutBox`'s content height is the **line's** block size. Measured, not argued: an ordinary `<p><span>Hi</span></p>` already reports a non-empty `content_rect_local()` for the span today, while an *empty* undecorated span has no `LayoutBox` at all and so reads `(0,0)` — accidentally conformant. **PR-1c** (cell 14c) and **PR-1d** (cell 21) grant that box to previously box-less targets, so the existing violation gains reach and a `ResizeObserver` callback fires with a non-empty `contentRect` where §3.3.1 requires an empty one. **Routed to `#11-resize-observer-inline-empty-content-rect`** (§9, §10), **pre-existing** class | ✗ (deliberate, §9) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | glyphless / fallback-only box | strut with first-available-font metrics | **two conditions, one delivered, and that one only where a font resolves**: the *glyphless* one is M7's tentative baseline — **PR-1d** — but the strut's "first available font" is a font elidex cannot guarantee, `FontDatabase::query` answering `None` with no last-resort family (R16, **`#11-shaping-no-last-resort-font`**, §5.3, §8); the *fallback-only* one ("or if it contains only glyphs from fallback fonts") has no elidex signal at all and is **`#11-inline-fallback-font-strut`**'s (§5.3, §8) | ✗ (pre-existing, disclosed) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | half-leading, in the **`line-height` is not `normal`** branch | `A′ = A + L/2`. ⚠ **The Branch was the bare formula until R20**, and the formula is one branch's: §5.3 splits on the computed `line-height`, and this arithmetic sits inside "When its computed `line-height` is **not** normal, its layout bounds are derived solely from metrics of its first available font (ignoring glyphs from other fonts) …". The `normal` branch is the row below, added in the same round; §1.5 carries both quotations | existing formula (`inline/pack/mod.rs:581`) — unchanged | ✓ (pre-existing, and now for this branch only) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | the **`line-height` is `normal`** branch | "the layout bounds of an inline box encloses **all its glyphs**, going from the highest A to the deepest D. (Note that glyphs in a single box can come from different fonts and thus might not all have the same A and D.)" | ⚠ **unimplemented, and it is the branch a document with no author `line-height` is in** — `normal` is the property's initial value and survives to the computed value (`css css-inline-3 line-height` → `initial: normal`, `computedValue: the specified keyword, …`). elidex never enters it: `LineHeight::Normal` collapses to `font_size * 1.2` at `crates/core/elidex-plugin/src/computed_style/text.rs:136`, reached from `inline/styled_run.rs:96`, and the packer then runs the *other* branch's first-available-font-plus-half-leading shape (`:581`). Two divergences follow — a fixed 1.2 ratio standing in for the font's own A and D, and bounds that cannot span fonts. **Pre-existing**, and upstream of M6's `block_advance` and M7's strut, which both read the already-collapsed value. **`#11-inline-height-normal-layout-bounds`** (§1.5, §9, §10). ⚠ Not a sixth facet of `#11-inline-root-inline-box`: that slot's five facets are each pinned by a §6 cell and it records that none of them turns on **per-glyph** provenance, which is the one input this branch takes. ⚠ **And not the same slot under another name** — that slot's scope read "the whole css-inline-3 §5.3 layout-bounds model" and its trigger "any work that gives inline boxes layout bounds of their own" until R22, so the two shared both; the sibling is narrowed to §5.3's **composition** and this row keeps the branch's own bounds (§5.3, §9) | ✗ (pre-existing → slot) | yes |
| CSS Text 3 §5.5 Line Breaking Details | break opportunities | inline box boundary is **not** one | M3 — the marker path calls the shared core without a wrap check — **PR-1b** | ✓ | yes |
| CSS Text 3 §5.5 Line Breaking Details | soft wrap opportunities *between* inline boxes | those of the text alone — an **inline-box** boundary adds none. ⚠ **Scoped at R11**: the claim read "the boundary between two runs adds none", a universal §5.5's *next* bullet refutes for **atomic inlines** — "For Web-compatibility there is a soft wrap opportunity before and after each replaced element or other atomic inline, even when adjacent to a character that would normally suppress them, including U+00A0 NO-BREAK SPACE", with a GL/WJ/ZWJ exception the invariant now implements by construction (R12: feeding U+FFFC to the engine's own breaker reproduces the whole bullet bar the NBSP case, §8). An atomic item's two boundaries are therefore **licensed unless the adjacent character is GL/WJ/ZWJ, NBSP excepted**, and the *adjacent soft wrap opportunity* row — two rows down, past the intra-word shaping row — is about which edge a break lands at, not about whether one exists | `place_item`'s per-item flush (`pack/mod.rs:658` at `22de3078`) wraps at **every** item boundary with no break-opportunity test — pre-existing, **`#11-inline-item-boundary-soft-wrap`** (§5.3; Codex on #515). Over-breaking is wrong at inline-box boundaries and **accidentally right** at atomic ones, so the slot is scoped to the boundaries §5.5 does not license and its discharge must preserve the others; §8's PR-1b suite invariant carries the atomic term explicitly (R11) | ✗ (pre-existing → slot) | yes |
| CSS Text 3 §5.5 Line Breaking Details | intra-word shaping | "the characters must still be shaped … as if the word were still whole" (the elision is "(their joining forms chosen)") | ⚠ **the clause is not satisfied across a line break, and the cited mechanism answers the inverse question — the row read `✓ (pre-existing)` until R12.** The coalescing `pack/mod.rs:744` (`let coalesce = self.last_placed_entity == Some(entity);`) documents is scoped **within one line by explicit design**: `flush_line` sets `self.last_placed_entity = None` (`:439`) under the comment at `:436-438` — "New line starts a fresh run even for the same entity … reset explicitly so coalescing can't reach across the line break" — and `place_item`'s own comment scopes it to "break-pieces **on this line**" (`:722-735`). So it re-joins the engine's *within-line* segmentation, while the clause is about a word broken **across** lines: §5.5's own worked example is "نوشتن" broken between "ش" and "ت", the "ش" keeping its **initial** form (`body css-text-3 line-break-details`). The two fragments are then shaped independently — `builder/inline_flow.rs:107-124` iterates **per line** and each `InlineFlowRun::Text` carries its own `text`, which `elidex-shaping` shapes in one rustybuzz call per run (`shaping.rs:133`). **`#11-intra-word-shaping-across-line-break`** (§5.3, §8), **pre-existing** and engine-wide: nothing in this program creates or touches it, and `:744`'s within-line coalescing is unaffected. ⚠ **Reachability is measured rather than assumed, because the clause's own trigger list is of properties elidex does not have**: `crates/core/elidex-plugin/src/computed_style/mod.rs` carries **122** `pub <name>:` declarations (`git grep -cE '^[[:space:]]*pub [a-z_0-9]+:' 154bac3f -- crates/core/elidex-plugin/src/computed_style/mod.rs`) and not one of `overflow_wrap`, `word_break`, `line_break` or `hyphens` is among them — each name probed on its own, all zero — so `word-break` / `line-break` / `overflow-wrap` / hyphenation are absent from the engine (the complement outside that file is a crawler alias table, `elidex-crawler/src/analyzer/css.rs:30`, and a `spec_level.rs:57` doc comment — neither in layout). But elidex's break source is **UAX #14 whole** (`find_break_opportunities` → `unicode_linebreak::linebreaks`, `crates/text/elidex-linebreak/src/lib.rs:27`), which returns an **interior** opportunity inside an Arabic word at a SOFT HYPHEN U+00AD — joining-transparent, so the letters still join across it — measured against the workspace's own pin (`unicode-linebreak 0.1.5`, `Cargo.toml:236`): `linebreaks("نوش\u{00AD}تن")` → `[(8, Allowed), (12, Mandatory)]` against `[(10, Mandatory)]` for the unbroken word. So a narrow `<p>نوش&shy;تن</p>` reaches the case with no unimplemented property involved. ⚠ **The measurement is structural plus that probe; no Arabic fixture was rendered end-to-end**, and the row states the limit rather than implying a behavioural measurement. ⚠ The row's second obligation is separate and still live: `pack/mod.rs:726`'s fabricated citation is corrected by **PR-1b** (§3.1), which is the PR that changes what it documents. Ledger **A36** | ✗ (pre-existing → slot) | yes |
| CSS Text 3 §5.5 Line Breaking Details | adjacent soft wrap opportunity | break lands at the box's **margin edge** | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | max-content | inline-axis edges occupy space | M8 (`inline/measure.rs:44`) — **PR-1b** | ✓ | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | min-content | inline-axis edges occupy space in the widest unbreakable segment | M8 — edge term: **PR-1b**; the cross-item segment it is added to: the **min-content prereq PR**, in `main` before PR-1b (§8; ledger **A58**) | ✓ | yes |
| CSS Sizing 3 §5.2.1 Intrinsic Contributions of Percentage-Sized Boxes | rule 4 | "For the min size properties, as well as for margins and paddings (and gutters), a cyclic percentage is resolved against **zero** for determining intrinsic size contributions" (`body css-sizing-3 cyclic-percentage-contribution`) | **Decided here: the two intrinsic passes thread `0.0` as `containing_inline_size`** (R26; ledger **A59**). The callers are four (`git grep -n "collect_inline_items(" 154bac3f -- crates/`), one of them a test helper, and **two are the intrinsic passes** — `min_content_inline_size` (`inline/measure.rs:22`) and `max_content_inline_size` (`:51`). There every inline-axis percentage edge on the marker payload is **cyclic** in the section's sense: an inline box's containing block is its IFC root's content box (css-inline-3 §2, `body css-inline-3 model`), a margin or padding percentage refers to that box's logical width (`css css-box-4 padding-left` / `margin-left`), and that width is the quantity the two passes compute — so rule 4 resolves it against zero, for the min-content and the max-content contribution alike (the section's summary table marks both zero). The layout pass (`inline/mod.rs:154`) threads the real size and resolves the same percentage against it, which the section's closing bullet sanctions: "the percentage is resolved against the containing block's size", which "is not re-resolved", so "the contents might thus overflow". §6 cell 4 pins the layout-pass half and cell **25b** the intrinsic half. Delivered by **PR-1b**, which puts the edge term into both passes (§8; ledger **A58**); the `0.0` itself is written by **PR-1a**, which threads `containing_inline_size` through every `collect_inline_items` caller (§8's PR-1a DoD) and whose emit test is the parameter's first reader, so PR-1b is where the value is first observable (ledger **A62**) | ✓ | yes |
| CSS Text 3 §7.3 Shaping Across Element Boundaries | all **three** normative sentences — ⚠ **the Step read "both" until R20**, and the section has three (`body css-text-3 boundary-shaping`, lines 9 / 39 / 63); §1.4 quotes all three | **(a) must break** on any of three triggers — non-zero inline-axis edge; `vertical-align` not its initial value; a bidi isolation boundary. **(b) must *not* break** "when there is no effective change in formatting, or if the only formatting changes do not affect the glyphs (as in applying text decoration)" — ⚠ **the row carried (a)'s first trigger alone until R15** and read the other two as the gap, which inverted where the divergence is (§1.4, ledger **A41**). **(c) should *not* break** "across inline box boundaries **otherwise**, if it is reasonable and possible for that case given the limitations of the font technology" — the *otherwise* being a boundary where formatting **does** effectively change the glyphs, so (a)'s three triggers and (b)'s no-change predicate both miss it | **(a)**: satisfied today **by over-breaking, for all three triggers** — a text run carries the parent element's entity (`inline/collect.rs:309`), so any inline box that places a member changes it and `:744`'s `coalesce` is false at that boundary by construction; the residue is a box that places **nothing**, and it closes for **trigger 1 only** — `<p>a<span style="padding:1px"></span>b</p>` (cells 14 and 15) is ended by PR-1b's marker, while `<p>a<span style="vertical-align:super"></span>b</p>` and its `unicode-bidi: isolate` twin get **no** marker at all (M1 requires a non-zero edge) and keep coalescing after PR-1d. Triggers **2 and 3 are therefore live in that corner**, and unowned by this program — **`#11-shaping-break-vertical-align-and-isolation`** (§5.3, §8, §10), pre-existing. ⚠ R15 first recorded them as not live, reading the member-less residue as one case rather than three. **(b)**: violated at every boundary of an unstyled inline box **that contains content**, because the entity split keys on identity and never on whether formatting changed (a *member-less* box is the one case the engine gets right, and it is the complement) — **`#11-shaping-break-at-unchanged-inline-boundary`** (§5.3, §8), **pre-existing** and engine-wide, untouched by this program. **(c)**: violated wherever a boundary changes the glyphs without meeting (a)'s triggers, and **reachable with zero author CSS** — `crates/css/elidex-style/src/ua.rs:117` ships `b, strong { font-weight: bolder; }`, so `<p>نو<b>شتن</b></p>` is two entities, two runs and two `rustybuzz` calls. Neither of the other slots reaches it: (b)'s predicate is the complement of this one, and (a)'s residue slot is scoped to the member-less corner. **`#11-shaping-across-formatting-change-boundary`** (§1.4, §9, §10), **pre-existing** and engine-wide | ✗ (split verdict, on the css-inline-3 §5.3 glyphless/fallback-only row's precedent — the section's three sentences kept as one row: **(a)** ✓ pre-existing by over-breaking, with PR-1b for the member-less residue; **(b)** and **(c)** ✗ pre-existing → a slot each. The row takes the worst of the three) | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | step 3 | "A sequence of collapsible spaces at the end of a line is removed, as well as any trailing U+1680 OGHAM SPACE MARK …" — line-final unless **content** (a text run or an atomic inline) follows it; inline-box markers, edges and all, do not end it | this umbrella owns the step's outcome — a marker does not end the space's line-finality (M3, **PR-1b**) and the space is then removed, §6 cells 16, 16b and 3b; **how** it is removed is the **end-of-line white-space prereq**'s (§8), today's engine subtracting it at alignment instead (`inline/pack/mod.rs:234-235`) | ✓ | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | step 4 — its `normal`/`nowrap`/`pre-line`, `pre-wrap` and `break-spaces` bullets, and `pre`, which it does not list | `pre-wrap`: "the UA must (unconditionally) hang this sequence, unless the sequence is followed by a forced line break, in which case it must conditionally hang the sequence instead"; `break-spaces`: spaces "cannot hang nor have their advance width collapsed"; §3's informative table: `pre`'s end-of-line spaces "Preserve" | the **end-of-line white-space prereq** (§8) — the step's outcomes are its plan's, this umbrella states none of them (ledger **A67**); `break-spaces` is vacuous, the parser rejecting it (`declaration/tests/values.rs:161`) | ✓ | yes |
| CSS Text 3 §8.2 Hanging Glyphs | border/padding blocks hanging; a hanging glyph stays in its box | "Non-zero inline-axis borders or padding between a hangable glyph and the edge of the line prevent the glyph from hanging. For example, a period at the end of an inline box with end padding does not hang at the end edge of a line."; "A hanging glyph is still enclosed inside its parent inline box and still participates in text justification: its character advance is just not measured … Effectively, the hanging glyph character advance is re-interpreted as an additional negative margin on the affected edge of its parent inline box; the line is otherwise laid out as usual." (`body css-text-3 hanging`) | the **end-of-line white-space prereq** and **PR-1b**, in their own plans (ledger **A67**): which of a marker's edges block a hang, and how a hanging space sits in its box, are theirs; this umbrella names the section and stops | ✓ | yes |
| CSS Text 3 §4.1.1 Phase I: Collapsing and Transformation | step 4 | collapsing crosses inline box boundaries | `collapse_inline_whitespace` (the `fn`, `inline/whitespace.rs:27`; M2's arm joins the `match` at `:41`) — **PR-1a** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | Quirks Mode | an inline box fragment with **zero borders and padding** and no direct text is ignored when sizing the line box | not implemented — no quirks-mode layout switch exists; **§9** | ✗ (deliberate, §9) | yes |
| CSS 2 §10.8.1 Leading and half-leading | rendering of an inline box's own decoration | "Although margins, borders, and padding of non-replaced elements do not enter into the line box calculation, **they are still rendered around inline boxes**" | **nothing is emitted at all for a *static* inline**: `paint_non_sc` passes only a *block* child to `walk` (`crates/core/elidex-render/src/builder/walk.rs:648-676`) and `InlineFlowRun` (`crates/core/elidex-ecs/src/components/inline_flow.rs:131`) carries only `Text` and `AtomicBox`, so no code path draws an inline box's background or border — pre-existing and engine-wide, **`#11-inline-decoration-paint-path`** (§5.3). ⚠ Added at R5: R4 opened that slot and routed it to §5.3 and §10, and the one table that enumerates this program's spec surface carried no row for the gap its largest withdrawal exposed | ✗ (pre-existing → slot) | yes |
| CSS 2 §10.8.1 Leading and half-leading | glyphless inline box | strut with first-available-font metrics (superseded by CSS Inline 3 §5.3, kept as the historical anchor) | M7 — **PR-1d** | ✓ | yes |
| CSS 2 §8.3 Margin properties: margin-top, margin-right, margin-bottom, margin-left, and margin | non-replaced inline elements | vertical margins have no effect | consistent with CSS Inline 3 §2.3 clause 3; no code touch. ⚠ **The `✓` rested on an unstated premise, and R12 states it — measured at the *producer*, not by enumerating readers.** A **static** inline box gets its `LayoutBox` from exactly one site, `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`), whose `:62` guard `continue`s over any entity that already carries one — which is how an **atomic** inline keeps `layout_child`'s — and that site writes `margin: EdgeSizes::default()` (`:84`), the lines M4 replaces. So today the value is **zero at its source**, and no reader of `.margin` / `margin_box()` can give a static inline's vertical margin an effect whatever it does with it. ⚠ **The reader side is the complement, and it is what has to carry the row after PR-1c**, when M4 makes the value real: the field's consumers are `git grep -nP '\.margin\b' 154bac3f -- crates` (**88** hits; ⚠ `git grep -E` does **not** support `\b` and silently returns **0** for the same pattern — the probe must be `-P`) together with the `margin_box()` callers, and every **non-test** one is a box of another kind — block children (`block/children/stack.rs:399`/`:401`), **atomic** inlines (`inline/atomic.rs:61`, `inline/pack/mod.rs:629`/`:633`), a multicol **spanner** (`elidex-layout-multicol/src/lib.rs:269`), a table box (`elidex-layout-table/src/helpers.rs`), a flex item (`elidex-layout-flex/src/algo.rs`) and grid's own `GridItem.margin` — the rest being the accessor itself (`layout_types/boxes.rs`) and two **carriers** that copy the field without consuming it (`elidex-ecs/src/fragment_tree.rs:194`, `elidex-render/src/builder/slice.rs:110`), which is where a future reader would first appear | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline; sub-flow keying | `collect.rs:286` — **`#11-inline-box-decoration-splits`** (the `group_key` field arrives with its only reader, §2 pair 2×6) | ✗ (deliberate, §5.3) | yes |
| CSS Box Model 3 §3.1 / §4.1 | percentage margin/padding | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract Dimensions | inline size ≡ logical width | basis identity in vertical modes | same | ✓ | yes |
| CSS Writing Modes 4 §3.2 Block Flow Direction: the writing-mode property | box whose `writing-mode` differs from its **parent box** | an otherwise-`inline` box's display computes to `inline-block` | ⚠ **unimplemented, and the row read ✓ until R11 on the spec's conclusion rather than the engine's behaviour.** Measured by the *property* — every site rewriting a computed `display` toward a block-ish value — there are four, keyed on abs/fixed and float (`resolve/mod.rs:182`, `:185`), flex items (`helpers.rs:69`) and grid items (`helpers.rs:53`), and **none on `writing-mode`**; `resolve_writing_mode_properties` (`resolve/mod.rs:235-277`) writes no `display` at all, and `display.blockify()` yields `block` where §3.2 asks for `inline-block`, so wiring the existing helper would not satisfy it either. Such a span therefore stays an inline box and **does** get a marker; M1 sources the payload's writing mode from the IFC root so the mapping degrades conservatively (§6 cell 12f pins that, **PR-1b**), and the gap itself is **`#11-writing-mode-inline-blockification`** (§5.3), whose discharge retires 12f. Cell 12c keeps the `writing-mode`-on-the-containing-block arm, which the rule does not reach | ✗ (pre-existing → slot) | yes |
| CSS Writing Modes 4 §6.4 Abstract-to-Physical Mappings | side mapping | "based on the **used** `direction` and `writing-mode`" of the box being mapped | M1's `WritingModeContext` source; §6 cells 12b/12e for the `direction` half and cell **12f** for the `writing-mode` half — **PR-1b**. ⚠ **The mark was an unqualified `✓` until R12 and is false for half of the clause it quotes — R11's own sweep miss, on the row whose Touch names the very thing R11 changed.** After R11 the payload is `WritingModeContext::new(<the IFC root's writing mode>, style.direction)` (§5.1 M1), so the *used `direction`* component **is** the mapped box's, which is what css-writing-modes-4 §6.2/§6.4 attribute it to, and the *`writing-mode`* component is the **IFC root's** and not the mapped box's. Cell 12f says so in its own words — it pins "the *conservative degradation*, **not** conformance" — and its closing sentence assigns 12b and 12e to the `direction` half. Ledger **A35**. ⚠⚠ **And what M1 takes from the box is the *computed* `direction`, not the used one — R20, on the same clause, and the previous correction stopped one word short.** §6.4's own Note states the difference: "The used `direction` depends on the computed `writing-mode` and `text-orientation`: in vertical writing modes, a `text-orientation` value of `upright` forces the used `direction` to `ltr`." M1's payload is `WritingModeContext::new(<the IFC root's writing mode>, style.direction)` and `style.direction` is the computed value; nothing in the workspace derives a used one — `text_orientation`'s only layout-adjacent reader is `vertical_text_orientation` (`crates/core/elidex-render/src/builder/inline.rs:484`), which maps an orientation and touches no direction. `upright` parses (`crates/css/elidex-css-text/src/lib.rs:81`) and is live in render, so `<p style="writing-mode:vertical-rl; text-orientation:upright; direction:rtl">` hands M1 `(vertical-rl, rtl)` where §6.4's table is to be read at `ltr`, swapping inline-start and inline-end. **Pre-existing** and engine-wide, since every `LogicalEdges::from_physical` caller takes `direction` the same way — **`#11-used-direction-text-orientation-upright`** (§9, §10) | ✗ (split verdict, R20 — unqualified `✓` until R12, a two-way split until now. The **computed `direction`** component is the mapped box's, which cells 12b/12e pin; the **used** `direction` parts from it under `text-orientation: upright` in a vertical writing mode, and that residue is `#11-used-direction-text-orientation-upright`'s; the **`writing-mode`** component is the conservative degradation cell 12f pins, whose residue is the css-writing-modes-4 §3.2 row above, retired by `#11-writing-mode-inline-blockification`. The row takes the worst of the three) | yes |
| CSS Display 3 §A Glossary | *inline box* vs *atomic inline* | "A non-replaced inline-level box whose inner display type is flow. …" (the `…` marks the entry's second sentence, "The contents of an inline box participate in the same inline formatting context as the inline box itself" — restored as an elision in rev 34, the same class as the *atomic inline* third conjunct below; round 26, Axis 4) vs "An inline-level box that is **replaced** (such as an image) **or** that establishes a new formatting context … **and cannot split across lines** (as inline boxes and ruby containers can)" (`webref dfn css-display-3 "inline box"` → `§A Glossary #inline-box`; `body css-display-3 inline-box`) | the **canonical predicate** the prereq PR establishes (§9), consumed by M1's emit test — **PR-1a**, §6 cell 6c — and by the four `client*` members. ⚠ elidex today answers only the formatting-context half (`is_atomic_inline`, `inline/collect.rs:14`; and the `pub` `is_block_level`, `block/mod.rs:46`, is a second partition of the same enum), so the replaced half is unimplemented on the inline path and this program **consumes** the predicate rather than re-deriving one. ⚠ **The definition's third conjunct — "and cannot split across lines" — was dropped unmarked at all three quoting sites** (this row, §6 cell 6c, §9's canonical-predicate bullet) and is restored at each (rev-33 gate). It does not change M1's classification of an `<img>`, which the first conjunct already settles; it matters because this memo uses §A **as a predicate**, and a two-conjunct rendering of a three-conjunct definition is the wrong predicate for whoever implements it | ✓ | yes |
| CSS Pseudo-Elements 4 §4.1 Generated Content Pseudo-elements: ::before and ::after | `content` computes to anything but `none` — a `<content-list>`, `""` included | "generate boxes as if they were immediate children of their originating element"; since the initial `display` is `inline` the box is an inline box in the originating element's IFC (`webref heading css-pseudo-4 4.1` → `§4.1 … #generated-content`) | M1's pseudo routing (the `:265` branch at `22de3078` re-routed through the marker emission) — **PR-1a**; §6 cells 6g, 6h | ✓ | yes |
| CSS Pseudo-Elements 4 §4.1 Generated Content Pseudo-elements: ::before and ::after | the suppression clause | "Also as with regular child elements, the ::before and ::after pseudo-elements are suppressed when their parent, the originating element, is **replaced**" (`body css-pseudo-4 generated-content`) | ⚠ **no replacedness test exists anywhere on the generation path.** `generate_pseudo_entity` (`crates/css/elidex-style/src/pseudo.rs:27-81` — ⚠ **`:27-50` until R22, which brackets the two early returns and not the function**, whose closing brace is `:81`; "and on nothing else" is a claim about the whole body, so the range has to be the whole body) gates on the pseudo cascade having winners and on `content` computing to `Items(_)`, and on nothing else; its call sites add only a `display` test (`crates/css/elidex-style/src/walk.rs:245-248`, the `styles[idx].display != Display::None` guard in front of the `::before`/`::after` pair). No UA rule selects `img` and `Inline` is the `Display` enum's first variant, which the macro gives `#[default]` (`crates/core/elidex-plugin/src/computed_style/display.rs:8`, the convention stated at `computed_style/mod.rs:27`), so `<img>` computes `display: inline` and `img::before { content: "x"; padding: 10px }` yields a styled pseudo entity that reaches M1's pseudo routing — PR-1a emits a marker pair around a box §4.1 says is not generated. **Pre-existing**; what this program changes is that the box becomes observable as an advance, a rect and a line. What its fix reads is the **replacedness half** of the canonical predicate the prereq PR establishes (§9's canonical-predicate bullet separates that half out as the only one needing a component-bearing crate) — this slot's **input**, on `#11-resize-observer-inline-empty-content-rect`'s precedent, and a further consumer beside the two that bullet enumerates. ⚠ **This read "the non-replaced conjunct … is the canonical predicate" until R22**, which names the composite: §4.1 suppresses on replacedness alone, and the composite is false for a non-replaced atomic origin such as `span { display: inline-block }`, whose generated children it does not suppress — **`#11-pseudo-generation-on-replaced-originating-element`** (§9, §10) | ✗ (pre-existing → slot) | yes |
| CSS Content 3 §1 Inserting and Replacing Content: the `content` property | `<content-replacement>` vs `<content-list>` | the pseudo (or element) is a replaced element only under the former — a single `<image>`, which "Makes the element or pseudo-element a replaced element"; an `<image>` inside a list "is an inline anonymous replaced element", a replaced **child**, and the pseudo stays an inline box (`webref heading css-content-3 1` → `§1 … #content-property`; `body css-content-3 content-property`). ⚠ css-content-3 §1's issue note: a bare `<image>` "has historically been treated as `<content-list>` on ::before and ::after. Presumably there's a Web-compat requirement on this, so these pseudo-elements might need an exception. [Issue #2889]" — the operative reading for the only box-generating reader of `ComputedStyle.content` today (the pseudo pipeline), so the flip is element-side (§8 requirement 7). ⚠ **The elision here hid the hedge until R22**: the `…` dropped exactly the "Presumably … might need an exception" sentence, which is what makes the note an open question rather than a rule, and §8's copy of the same note quotes it whole — so the memo's two statements of one quotation disagreed on its modality | the predicate prereq PR's predicate (its mechanism is handed over in §9) — §8 requirement 7 (replacedness of generated content decided from the `content` model, total over `ContentItem`); §6 cells 6g, 6h | ✗ (pre-existing, disclosed: the spec's `<content-replacement>` branch has no elidex counterpart to enumerate — the live `parse_content` (`crates/css/elidex-css/src/declaration/misc.rs:441`) accepts only quoted strings, `attr()`, `counter()`/`counters()` and the keywords, so it rejects `url()` and no `ContentItem` can carry an image. ⚠ The path is named because **two** functions bear that name: the other is `crates/css/elidex-css-box/src/lib.rs:600`, reached only from that crate's own `parse` arm (`:162`) — the crate whose `resolve` `content` arm §8 requirement 7 measures as having no production caller. The claim holds for both — neither has a `url()` branch — but an unqualified name does not say which one was measured (round-24 gate). ⚠ The ✗ is about the *spec* branch, not about §8 requirement 7(c), which argues the opposite for the model: totality over `ContentItem` is what makes a future image variant a compile error rather than a silent gap) | yes |
| CSS Content 3 §1 Inserting and Replacing Content: the `content` property | the box count of a `<content-list>` | "Each value contributes an inline box to the element's contents. For `<image>`, this is an inline anonymous replaced element; for the others, it's an anonymous inline run of text" (`body css-content-3 content-property`) | elidex makes **one** box for the whole list: `resolve_pseudo_content` (`crates/css/elidex-style/src/generated_content.rs:174`) concatenates every `ContentItem` into a single `String` (`:182-206`), which its caller writes as one `TextContent` (`:157-158`), and `collect.rs:260-272`'s pseudo arm pushes one `Text` for that. ⚠ **The `✓` is conformance by an empty complement, and R20 writes down what makes it so** rather than leaving the next reader to re-derive it — the class the css-inline-3 §5.3 inflation row's vacuous mark already records. The clause separates the values only at `<image>`, and `parse_content` (`crates/css/elidex-css/src/declaration/misc.rs:441-534`) admits quoted strings and three `Token::Function` arms — `attr` (`:462`), `counter` (`:476`), `counters` (`:495`) — plus the keywords, and carries no `url` arm anywhere in that span, so every `ContentItem` elidex can build is "an anonymous inline run of text", and adjacent runs of one style are not distinguishable from one. The row above carries the `<content-replacement>` branch, and §8 requirement 7's totality over `ContentItem` is what keeps a future image variant a compile error rather than a silent second box — the row vouches for today | ✓ (vacuous — no `ContentItem` elidex can produce is an `<image>`) | yes |
| CSS Backgrounds 3 §3.2 Line Patterns: the `border-style` properties | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time — the loop at `crates/css/elidex-style/src/resolve/box_model/mod.rs:261-276` sets the width to `0.0` for `BorderStyle::None \| Hidden`, and that crate's own `border_width_zero_when_style_none` (`resolve/box_model/tests.rs:22`) asserts it, so this program adds no cell. ⚠ The row vouches for the **behaviour** only; the two mis-citations at that same site are §3.1's, not this column's. The css-backgrounds-3 §3.2 anchor here is this memo's, established by lookup | ✓ | yes |

**Breadth**: K=13 specs (CSS Inline 3, CSS Text 3, CSS 2, CSS Break 3, CSS Box Model 3,
CSS Sizing 3, CSS Writing Modes 4, CSS Display 3, CSS Pseudo-Elements 4, CSS Content 3, CSS Backgrounds 3, CSSOM View 1, Resize Observer 1), M=50 entries (`Split decision` below restates K; both are recomputed) — R20's enumeration-completeness audit (§9) took M from 42 to 48, and rev 63 to 50: css-text-3 §4.1.2 split into its step-3 and step-4 rows, and a css-text-3 §8.2 row (ledger **A63**). K is unchanged, every section those rows cite belonging to a module the table already carried. Both figures are recomputed
from the table above by `python3 .claude/tools/plan-xcheck.py <memo>`, which prints them and fails
on drift — that command is the verification artifact, and it is re-runnable rather than dated.
⚠ `preflight.py` reports `parsed citations: 0` here: its `SPEC_LABEL_REVERSE` carries no label
for any CSS module this memo cites (it has exactly one CSS label, `CSS Selectors L4` — `grep -n
CSS .claude/skills/elidex-plan-review/preflight.py`; an earlier drafting said "no CSS-module
labels", a universal the complement refutes), nor — since R9 — for the one row whose spec is not
a CSS module at all (`Resize Observer 1`, likewise absent from the dict), so its citation
hard-gate is **vacuous for this
memo** and every §-number below was
verified by hand with `.claude/tools/webref` instead. Closing that gap is
`#11-preflight-css-module-labels`'s (the SoT slot owned by the citation-hygiene lane's Slice B,
after its A-ii migrates the dict) — not this umbrella's, and **not its plan-checker tooling
task's** either (an earlier revision booked it there, a second decision surface for one gap).

**Split decision**: K=13 ⇒ SPLIT-DEFAULT. The plan **is** split into four shipping PRs, each
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
* ⚠ **The css-backgrounds-3 pair, handed off explicitly rather than by a grep.** At
  `crates/css/elidex-style/src/resolve/box_model/mod.rs`, `:262` says only "CSS spec:" with no
  module or section, and `:270` cites "CSS Backgrounds §4.3" for the non-negative rule — that
  anchor is *Corner Clipping*; the rule is css-backgrounds-3 **§3.3** *Line Thickness: the
  `border-width` properties*. Both are pre-existing and fall outside all three concept greps
  above, so `#11-inline-spec-cite-misattribution` (§9) owns them by this explicit hand-off.
  ⚠ It lives here, not in §3's Touch column where it was written until R9: that column states
  where the behaviour is touched, and a slot named inside it reads to `plan-xcheck.py`'s check 4
  as the row routing its own **coverage** away while claiming `✓`. One meaning per column; the
  checker stays strict.
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
gap is observable today on ordinary `<span style="padding:10px">text</span>`, independently of
whether any line is phantom. ⚠ **Observable where, stated rather than left to "visible"** (R4):
the following content is not displaced, which moves painted glyphs (identity-order path, §7's R7-b scope); and the span's own box is
reported without its edges at `getBoundingClientRect` / `offsetLeft`, at hit-testing
(`elidex-layout/src/hit_test.rs:130-131` + `:165`) and at a11y bounds
(`elidex-a11y/src/tree.rs:121-125`).
It is **not** observable as an unpainted background or border, because a static inline's chrome is
never emitted at all — a separate, engine-wide gap this umbrella does not own
(`#11-inline-decoration-paint-path`, §5.3; §5.2's `elidex-render` row carries the measurements).

## §5. Design

### §5.1 Mechanism table

| # | Question | Decision | Grounds |
|---|---|---|---|
| **M1** | What enters the item stream, carrying what? | Two `InlineItem` variants, `InlineBoxStart` / `InlineBoxEnd`, emitted around the recursion at `inline/collect.rs:291` for every **inline box** — css-display-3 §A's defined term, i.e. **non-replaced** ∧ outer `inline` ∧ inner flow — with **at least one non-zero edge on any side**. ⚠ **The non-zero-edge conjunct is kept deliberately** (Codex on #515 asked whether a zero-edge empty inline — `<span style="line-height:100px"></span>` on a line other content keeps — should enter the stream for css-inline-3 §2.2's line-height influence): a marker for *every* inline box would — through M4's `InlineBoxEnd` pop, which pushes a rect **unconditionally**, and the commit arm's fold into `entity_bounds`, which the box-assigner iterates — grant a `LayoutBox` to every **undecorated** empty inline on an existing line: spec-correct (an empty inline box *is* a box; cssom-view-1 §6 returns a fragment for it) but a presence change over every empty `<span>`/`<a>`/`<b>` in every document, which cells 6b, 14's contrast (its no-marker clause), 21 and §7's "a box with all edges zero gains nothing" are all written against. **Widening to every inline box is the end-state; this program's non-zero-edge conjunct is a slice boundary, not a design ground**, and the widening is this program's **own** deferral — `#11-inline-zero-edge-box-in-item-stream` (§5.3), which names the cells it flips; §3's css-inline-3 §2.2 row is ✗ for the zero-edge case. ⚠ **The predicate is the FULL css-display-3 conjunction, and every conjunct is load-bearing — a `non-replaced` conjunct alone is not enough.** ⚠ An earlier drafting added only that one, on the strength of a single `<img>` example; the class is wider, and the arm admits all of it. `collect_inline_items_inner`'s **loop head, its style guard (`:217`, which routes an unstyled child to the text arm) and its four filters** (`:216-272`; ⚠ an earlier drafting labelled that range "the recursion arm" — the `for` body runs to `:317`, and both the recursion at `:291` and the text arm at `:302` are *outside* `:272`, so the range is right for what it is used for and the label was not — round-24 gate) filter exactly **four** things — `Display::None`, `is_absolutely_positioned`, `is_atomic_inline` (`:14`, four display keywords) and `PseudoElementMarker` — and passes every other **styled** child to the inline-element recursion at `:291`. ⚠ Two earlier draftings, both fixed by round 24's §5.1 audit: the range was `:216-251`, which stops at the `is_atomic_inline` arm's `continue` and so does not bracket the fourth filter (`PseudoElementMarker` is `:260-272`); and "passes everything else" has a non-empty complement — a child with **no** `ComputedStyle` never reaches `:291` at all, because `:217`'s `if let Some(style) = crate::try_get_style(…)` sends it to the text arm at `:302` (a text node or a comment). ⚠ **The `PseudoElementMarker` filter is a *routing* special-case, not an exclusion from the class** (Codex on #515): a `::before`/`::after` pseudo entity (`elidex-style`'s `generate_pseudo_entity`, cascaded by `collect_and_cascade_pseudo`, so it carries its own `ComputedStyle` — padding included) with computed outer `inline` **is** a css-display-3 inline box, and the `:265` branch (`:265` at `22de3078`; `:260` at `154bac3f`) pushes its `Text` and `continue`s *before* `:291`, so markers placed only around the recursion would never bracket it. **PR-1a routes that branch through the same `InlineBoxStart`/`InlineBoxEnd` emission** — the predicate evaluated on the pseudo's own style (§8 requirement 7 puts that entity kind — `NodeKind::Text`, no `TagType` — in the predicate's domain), the emission keyed on the predicate **alone** and bracketing **whatever the branch pushes — one `Text`, or nothing**: the branch pushes a `Text` only under `!tc.0.is_empty()` (`:266-274` at `22de3078`), and elidex generates a pseudo entity for `content: ""` too (`elidex-style/src/resolve/box_model/mod.rs:424` maps `CssValue::String(s)` → `ContentValue::Items(vec![String(s)])`, `pseudo.rs:47` returns only when `content` is not `Items(_)` and `:63` then creates the entity, `generated_content.rs:158` writes `TextContent("")`), so an empty decorated pseudo — `p::before { content: ""; padding: 10px }`, the ubiquitous decorative idiom — yields an **adjacent `InlineBoxStart`/`InlineBoxEnd` pair**, cell 14c's shape for generated content — so a decorated pseudo reaches M3/M4/M5 like any element; §6 cells 6g (non-empty) and **6h** (empty) pin the two. ⚠ An earlier drafting keyed the markers to "its single `Text` item", which silently dropped the pair for empty `content` (round 22, Axes 2/3). Three measured members reach the recursion that are **not** inline boxes: <br>• **replaced** — no UA rule selects `img`/`iframe`/`canvas`/`video`/`svg` (`grep -n '\bimg\b' crates/css/elidex-style/src/ua.rs` → no hits), so all compute `display:inline` and `is_atomic_inline` is false for them. css-display-3 §A makes them *atomic inlines* — and the **clause numbering is this memo's own** (§1.2's table), applied to css-inline-3 §2.3's **defining sentence**, which is itself unnumbered (`body css-inline-3 invisible-line-boxes`; ⚠ an earlier drafting called css-inline-3 §2.3 *itself* "a single unnumbered sentence" — the section carries a second normative sentence, "Such boxes must be treated as zero-height line boxes …", and a *What's invisible?* note with a five-item list, so the property belongs to the defining sentence and not to the section — round-24 gate). css-display-3 §A's *atomic inline* entry is likewise unnumbered prose — two sentences, no enumeration (`body css-display-3 glossary`; the second, "Any inline-level box whose inner display type is not flow establishes a new formatting context …", is the shape §A's other multi-paragraph entries use), so "clause 3" / "clause 4" cannot be read against §A at all: read against css-inline-3 §2.3, clause 3 does not reach them and clause **4** does. ⚠ An earlier drafting said the numbering was css-inline-3 §2.3's and described **§A** as "a single prose sentence with no enumeration" — §A is *Appendix A: Glossary*, some twenty definition entries; only the *atomic inline* entry inside it is the single sentence (round 24 audit). <br>• **`display: contents`** — no `Contents` branch exists in `collect.rs`, and `:277` takes **raw** `dom.composed_children`, while the sibling `positioned_subflow_key` takes `composed_children_flat` (`:110`) *for exactly this reason*, its own comment at `:93` saying so. `ua.rs:109` ships `slot { display: contents; }`, so it is live. css-display-3 **§2.5** *Box Generation: the `none` and `contents` keywords*: "The element itself does not generate any boxes, **but its children and pseudo-elements still generate boxes and text sequences as normal**", so it has no outer display type to be inline. ⚠ Two rev-34 corrections here (round 26, Axis 4). (i) The **title** was abbreviated to *Box Generation* while §6 cell 6d spelled it in full, and this memo marks an abbreviation explicitly where it uses one (§1.2's *Margin properties*) — so the two sites disagreed with no marker between them. (ii) The **quotation** stopped at "any boxes" with no ellipsis, dropping the `but` clause; that defect was at *both* sites, and the clause matters because it is what cell 6d's own second assertion ("`x` still reaches the line") rests on. <br>• **block-level reached through an inline** — `children_are_block` (`crates/layout/elidex-layout-block/src/block/mod.rs:70`) tests **direct** children only, so `<p>a<span><div style="padding:10px"/></span>b</p>` takes the IFC path and the `<div>` reaches the arm. <br>M1 therefore **consumes the canonical predicate the prereq PR establishes** (§9) — outer display `inline` ∧ inner display flow ∧ non-replaced ∧ generates a box — rather than re-deriving any of it here. That is the same predicate the four `client*` members consume, which is why that PR lands before **PR-1a**. §6 cells **6c/6d/6e** pin one member each. ⚠ `<br>`/`<wbr>` are **not** in this class: per css-display-3 they *are* inline boxes, so a marker for a decorated one is correct. That elidex implements neither — one probe per tag over `crates/` whole, `grep -rnE '"br"\|BrMarker' crates/` (21 lines) and `grep -rniE '\bwbr\b' crates/` (7), 27 distinct `file:line` between them and **all** of them parsing or DOM, **none** in `crates/layout`, `crates/core/elidex-render` or `crates/core/elidex-ecs`; `force_break()` has one caller, `pack/mod.rs:603`, the preserved-`\n` path — is a pre-existing gap §9 records (⚠ both probes were scoped to those three directories until rev 34 and are rescoped at both sites in one edit — §9's bullet carries the complement; round 26, Axis 1, Gate B). ⚠ **What is unimplemented is each tag's *break* behaviour, not the element**, so this program does reach a decorated one: neither tag is selected by any UA rule, so `<br style="padding:10px">` computes `display:inline`, is styled, and passes all four filters above to the `:291` recursion — M1 emits its marker, and PR-1d flips `<p><br style="padding:10px"></p>` (an instance of Shape B) to a committed line. §9's bullet carries the chain and withdraws rev 32's contrary claim that no cell of this program is constructible with either tag (round 25, Axis 3). ⚠ An earlier drafting quoted a **case-insensitive** single command (`grep -rniE '"br"\|BrMarker' …`) as returning nothing; it returns one hit at `154bac3f` — `elidex-render/src/builder/tests/paged.rs:446`, a paged-media `"BR"` corner label the `-i` matches — and it probed only one of the two tags. The conclusion is unchanged; the transcript was not reproducible, which is the defect (round 24 audit). Payload: `entity` + the **three physical `EdgeSizes`** `resolve_box_model(&style, containing_inline_size)` returns (`helpers.rs:116`) + the `WritingModeContext` they were resolved under (`logical.rs:27`), built from **the IFC root's writing mode and the decorated inline's own `direction`** — `WritingModeContext::new(<the IFC root's writing mode>, style.direction)`, the root value threaded to the recursion beside today's `root_horizontal` (computed at `inline/collect.rs:150`, taken as a parameter at `:211`) rather than read off the box's own `style` at the style guard (`:217`); PR-1a widens that parameter (or adds its sibling) so the payload carries the full `WritingMode` and not only the axis bool, and `root_horizontal`'s existing reader — `:107`, reached from `positioned_subflow_key` at `:287` — is unchanged. ⚠ **This half of the Decision changed at R11, deliberately**: the freeze retired the *internal* plan-review loop, it did not immunise a Grounds bullet external review has measured false, and the Grounds bullet below carries the measurement — + the **five resolved font-and-height fields M6 and M7 read, named rather than glossed**: `line_height`, `families`, `font_size`, `font_weight`, `font_style`. ⚠ **Five, not "`line_height` and font identity"** (round 26, Axis 2): the gloss under-specifies the payload against its own readers. M7 calls `FontDatabase::query(families, weight, style)` (`crates/text/elidex-shaping/src/database.rs:60`) and `font_metrics(id, font_size)` (`:101`), so all four of the font fields are read and not one of them is optional; and M6's `block_advance` follows the packer's own vertical convention `if is_vertical { font_size } else { line_height }` (`pack/mod.rs:539-543`), so **M6 reads `font_size` too** — a field the gloss did not carry at all. The five names are the crate's own for the same five facts: `StyledRun` carries `families` (`inline/styled_run.rs:49`), `font_size` (`:51`), `font_weight` (`:53`), `font_style` (`:55`) and `line_height` (`:61`), and M1 follows that spelling so no reader has to re-derive a mapping. Grounds: <br>• **The axis is the IFC's by construction — and this Decision is what makes that true here, rather than assuming it** (R11). css-writing-modes-4 §3.2 *Block Flow Direction: the `writing-mode` property* (`webref heading css-writing-modes-4 3.2`), the **first** bullet of the different-`writing-mode`-than-parent list: "If a box has a different `writing-mode` value than **its parent box** … If the box would otherwise become an in-flow box with a computed display of `inline`, **its display computes instead to `inline-block`**." (The trigger is the *parent box*, not the containing block, and the "establishes an independent … formatting context" clause of the same rule applies only to a box that is a block container — an inline reaches `inline-block` by the display change, not by that clause.) In a **conforming** engine that rule is inductive over the IFC: every surviving inline box has its parent's writing mode, hence the root's, so the box's own value and the root's are **identical** and sourcing the payload from the root is a no-op. <br>• ⚠⚠ **elidex does not implement §3.2, so on this engine the two sources differ on a reachable markup — and that is why the root is the source.** An earlier drafting read "A decorated *inline box* therefore always shares the IFC's writing mode; a `<span style="writing-mode:vertical-rl">` computes to `inline-block`, i.e. an atomic, which M1 emits no marker for", which is the spec's conclusion asserted of an engine that does not enforce its premise. Measured **by the property rather than by the word "blockify"** — every site that rewrites a computed `display` toward a block-ish value, harvested with `grep -rn '\.display = ' --include='*.rs' crates/ | grep -v tests` and each hit read — there are **four**: `crates/css/elidex-style/src/resolve/mod.rs:182` (abs/fixed) and `:185` (float), both CSS 2.1 §9.7, `crates/layout/elidex-layout-flex/src/helpers.rs:69` (Flex §4.2 items) and `crates/layout/elidex-layout-grid/src/helpers.rs:53` (Grid §6.1 items) — ⚠ **the grep's own hits are the flex and grid *assignments* at `helpers.rs:71` / `:55`; the lines cited are the `.blockify()` calls two above, which is what "each hit read" gives**, and this row did not say so until R22 while §5.3's copy of the same measurement did. None is keyed on `writing-mode`, and `resolve_writing_mode_properties` (`resolve/mod.rs:235-277`) assigns `direction`, `unicode-bidi`, `writing_mode` and `text_orientation` and nothing else. (The one remaining `.display = ` write, `pseudo.rs:55`, sets `Display::Inline` on a pseudo — an initial value, not a blockification; that is the complement, measured rather than left unstated.) **And the existing helper would be the wrong rule even if it were wired**: `blockify_display` is `display.blockify()` (`resolve/mod.rs:218-219`), which takes `inline` to **`block`** (`:469`, CSS 2.1 §9.7) where §3.2 asks for **`inline-block`**. So `<p><span style="writing-mode:vertical-rl; padding-top:10px">x</span></p>` keeps `Display::Inline`, passes M1's predicate and every one of `collect`'s filters, and reaches the recursion arm — and a payload sourced from the box would carry a **vertical** context into a **horizontal** IFC, which M3's advance, M4's content offset and M5's predicate would then map on the wrong axis. The engine already treats this state as live rather than impossible: `positioned_subflow_key` gates a *positioned* inline on exactly this mismatch (`inline/collect.rs:107`), and the parameter's own comment (`:208-210`) says so — "A positioned inline whose writing mode differs gets no sub-flow (render would read it with the wrong axis)" — but a **non-positioned** one takes the `else` arm at `:289` and is not gated at all. Taking the root's writing mode is the conservative degradation: it is the axis every group is already projected with (`:147-150`). §6 cell **12f** pins it, §3's css-writing-modes-4 §3.2 row routes the underlying gap, and the gap itself is `#11-writing-mode-inline-blockification` (§5.3), whose discharge retires 12f. Ledger **A31**. <br>• **The direction is the box's own.** css-writing-modes-4 §6.4 gives the abstract-to-physical mappings "based on the **used** `direction` and `writing-mode`" — of the box whose sides are being mapped. `direction` *can* differ on an inline box without forcing an independent context, so it is the one component that varies, and css-writing-modes-4 §6.2/§6.4 say it is the box's own. <br>• **Why the two competing readings fail**: css-writing-modes-4 §2.1 (*Specifying Directionality: the `direction` property*), verbatim: "The direction property has no effect on bidi reordering when specified on inline boxes whose unicode-bidi value is normal, **because the box does not open an additional level of embedding with respect to the bidirectional algorithm**." Its subject is the *embedding level* of reordered content, not the mapping of a box's own sides, so it does not license taking the direction from elsewhere. ⚠ Earlier revisions paraphrased this as "when `unicode-bidi` is `normal`", dropping both "when specified on inline boxes" and the `because` clause that names the mechanism — the clause is what makes the refutation hold rather than merely assert it. (Cell 12b's only `[dir]` element is the `<p>`, so its `<span>` really is `unicode-bidi: normal`; the citation is sound and still does not reach). css-break-3 §5.4's broken-edge rule — quoted in full in §3's css-break-3 row, and **referred to, not re-quoted, everywhere else including here**, so one edit keeps every site true — is scoped to *which side of a fragment is the broken edge*, and its own example is an element that "breaks across two lines" — an unfragmented box has no broken edge. §3 routes that whole section — both `slice` and `clone` — to `#11-inline-box-decoration-splits`, which is also where the parent-direction source lives, so the two direction sources never meet inside one PR. <br>Only the physical edge *values* come from the element's own `ComputedStyle`. Logical facts are *derived* at the point of use via `LogicalEdges::from_physical` (`logical.rs:186`), applied to each set separately: their inline-start/inline-end components summed across the three sets give M3's advance and M4's content offset; the same components tested *per set* give M5's predicate. **Both are derived in `pack/inline_box.rs` beside `has_inline_axis_edge`, not cached on the marker** — the sums' inputs already sit on the payload, and §5.2 designates that module the one derivation site for everything read off a marker, so a stored total would be a second representation of a fact the same struct already determines. `helpers.rs`'s `inline_pb` (`:148`) is **not** reusable here — it covers padding + border only, and the advance must include margin. The `PackItem` forms are `InlineBoxStart { item_index }` / `InlineBoxEnd { item_index }` — **an index, not a copy of the payload**, per §4's `PackItem` idiom; markers never become `FlowMember`s. ⚠ **Split across PR-1a and PR-1b by §8's dead-field rule.** PR-1a emits `InlineBoxStart { entity }` / `InlineBoxEnd { entity }`: the emit *test* resolves the edges at collect time and then discards them, because the variants' only PR-1a readers are the exhaustive matches. The three `EdgeSizes` + `WritingModeContext` join the payload in **PR-1b**, with their first readers M3/M4/M5, and the five font-and-height fields in **PR-1d**, with their first readers M6/M7. The rule the DoD states therefore reaches this payload too, not only `line_height` / `families` / `font_size` / `font_weight` / `font_style` / `group_key`. | **Three sets, not one**: `LayoutBox` has independent `padding`/`border`/`margin` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:88`, `:90`, `:92` — ⚠ **not** a contiguous `:88-91` range, as an earlier revision wrote: the fields are doc-comment-separated and `margin` is `:92`) consumed separately by `padding_box`/`border_box`/`margin_box` (`:134-149`; ⚠ an earlier drafting wrote `:134-150`, whose `:150` is the `}` closing the `BoxModel` trait, not part of any method — round-24 gate), and `resolve_box_model` already returns the triple — one `LogicalEdges` cannot fill three. **Physical, not logical, across the boundary**: `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) writes *physical* fields and receives `is_vertical: bool` with **no `direction`**, so a logical payload would need a return trip whose context is not present there. Carrying physical + the ctx keeps one conversion direction and no reconstruction. **Index, not copy**: a copied payload would duplicate the item stream's data against the file's own `item_index` idiom, and would give M5 two derivation sites — the pre-pack gate holds `&InlineItem` and the packer holds a `PackItem`, so only an index lets both read the *same* payload through one function. Emit on **any** side because M4 must fill all four for a block-axis-only inline's box to be right at every reader; the *existence* test is inline-axis-only (M5) — see §6 cell 6 for the pair. (the reason all four must be filled is the `border_box()` family, which reads them; a static inline's chrome is painted nowhere — §5.2's `elidex-render` row, ledger A8.) `resolve_box_model` is mandatory: `sanitize_padding` (`:96`) resolves percentages against 0, `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative. Basis = logical width = inline size (css-box-3 §3.1/§4.1; css-writing-modes-4 §6.1), `resolve_padding`'s documented contract (`:57-62`). |
| **M2** | Collapse-pass arm | **Transparent** — same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so interleaved markers are skipped by construction — including an adjacent `Start`/`End` pair. | §1.6 (css-text-3 §4.1.1 step 4): collapsing crosses "the boundary of the inline containing that space". A barrier would change `a <span style="padding:10px"> </span>b`, currently-correct markup: today the `<p>`'s `"a "` leaves `prev_collapsible_space` true (`whitespace.rs:34` seeds it, `collapse_run_text`'s `Normal` arm sets it on the space), so the span's own space collapses to zero advance and the IFC renders **`a b`**; a barrier resetting the flag at each marker — the `Atomic` arm's behaviour (`whitespace.rs:45-48`) — would let that space through and render **`a  b`**. ⚠ An earlier drafting used `a<span style="padding:10px"> </span>b`, on which a barrier is a **no-op**: with the space only inside the span, `prev_collapsible_space` is already false on entry to it, so both readings render `a b` and the markup cannot discriminate them (round 24 audit). |
| **M3** | How does anything occupy a line? | **Outcome at a box boundary (rev 63, freeze amendment by user decision; ledger A63)**: **a box marker does not make a trailing space non-line-final, whatever its edges** — an `InlineBoxStart` or `InlineBoxEnd` standing between a trailing space and the line's end does not, by itself, end that space's line-finality. Grounds: css-text-3 §4.1.2 step 3 names no inline-box edge (§3's row quotes it); §4.1.1's step 4, inside its `normal | nowrap | pre-line` bullet and so grounding those values only, collapses a space "even one outside the boundary of the inline containing that space, provided both spaces are within the same inline formatting context"; §5.5 gives an inline box boundary no break opportunity (§3's row); and css-inline-3 §2's "Inline-axis margins, borders, and padding are respected between inline-level boxes" says what those edges do to spacing, not whether a space is line-final — the premise the withdrawn side-specific gate stood on. Under `white-space: normal` that settles the outcome: css-text-3 §4.1.2 step 3 **removes** the space, so nothing of it reaches the aligned line width or the position of what follows (cells 16, 16b). For the other `white-space` values — `pre-wrap`'s hang, what css-text-3 §8.2 lets block it, `pre` — the outcome is the end-of-line white-space prereq's and PR-1b's own plans' (§8; the freeze's discipline 5 exception, ledger **A67**), and conformance is named by reference: css-text-3 §4.1.2 steps 3 and 4, §5 (*forced line break*: "When a line is broken due to explicit line-breaking controls (such as a preserved newline character), or due to the start or end of a block, it is a forced line break") and §8.2. The mechanism here is one argument: the marker path passes **`hang = None`** to `note_line_occupancy`. <br>**One owner, three callers.** Extract `LinePacker::note_line_occupancy(inline_advance, block_advance, hang: Option<f32>, contributes_content, occupant: LineOccupancy)` (NEW, private) writing exactly **five of** the line-state fields `place_item` writes today (§4 enumerates eight; an earlier drafting wrote "the five", a definite article the enumeration refutes — and this row itself reaches a sixth, `last_placed_entity`, below — round 24 audit): `current_inline +=` (`:695`), `current_line_height = max(..)` (`:696`), the line-occupancy raise that replaces `on_line = true` (`:697`, see the ⚠ below), `any_rendered_content \|=` (`:698`), and — **only when `hang` is `Some`** — `current_line_last_hang =` (`:701`). `place_item` calls it **after** its soft-wrap check (`:690`) and **after** snapshotting `seg_inline_start = self.current_inline` (`:694`, which five downstream sites consume), passing `Some((full-trimmed).max(0.0))` and the occupant the ⚠ below **derives** (`Content` at every PR-1b call site, since the ordering has no higher rung yet) — behaviour unchanged. The marker path calls it with **no** soft-wrap check, `occupant = BoxEdgeOnly`, `contributes_content` from M5, `block_advance = 0.0` in PR-1b (M6 supplies the real value in PR-1d, when `line_height` joins the payload under the dead-field rule — so all five parameters are named at both stages), and `hang = None` — **no marker touches the hang**, whatever its edges (the outcome above; ledger **A63**). ⚠ **The third caller is `flush_line`, and the owner is the whole reason it is a caller** (R7-a): after the per-line reset block (`:432-439`) it replays each **still-open** box's per-line contribution, one call per entry of M4's open-box stack, so `current_line_height` keeps exactly **one** writer and no mechanism reaches that field from outside this function. M4's row states the argument list and the ground for each argument, and M6 and M7 state what the replayed values are; M3 is unamended beyond the caller count — the alternative, M4's hook assigning the field directly, is the second-writer shape this row refuses everywhere else (ledger **A22**). ⚠ **Not M5's whole-box disjunction** (Codex R2-F3) — for the shaping break below; the hang reads no edge at all (rev 63). M5's whole-box disjunction keeps its meaning where the question really is about the **box** rather than a boundary: the clause-3 existence predicate and `inline/mod.rs:200`'s escape. A marker whose **own** side components are non-zero also clears `last_placed_entity` — the shaping break, side-specific on the same ground and in §1.4's own words, since css-text-3 §7.3 breaks on the margin/border/padding "separating the two typographic character units", which at an `InlineBoxEnd` is the box's inline-**end** set — the write `flush_line` already performs at `:439` (⚠ an earlier drafting cited `:772`, which *sets* the field to `Some(entity)` at the end of `place_item`; the clear is `:439` — round-24 gate). **Both readers of the resulting state are order tests over the ordering, never equality**: the soft-wrap guard (`:690`) holds at-or-above `Content`, `finish()` (`:787`) above `Empty`. The two have **different grounds** and the Grounds column separates them: the guard's predicate is exposed to extension and an equality there stops soft wrapping engine-wide at M7's widening, while `finish()`'s `!= Empty` and `> Empty` are the same predicate under any extension — `Empty` is the bottom and every raise is monotone — so that one is written as an order test for uniformity, not for correctness. §6 cell 15d asserts the guard's half; nothing needs to assert `finish()`'s (round 26, Axes 1/2/3). | Without a shared core, *entering the line* is unowned: the marker path would either duplicate `place_item`'s five-write sequence or silently skip part of it. `Option<f32>` rather than an `f32`: the field is never written by a marker (rev 63) and written unconditionally by `place_item`, and an `f32` parameter cannot express "leave it alone" — `place_item` assigns unconditionally today (`:701`). No wrap check: §1.3, a box boundary is not a break opportunity. `hang = None` at every marker: the outcome's grounds (css-text-3 §4.1.2 step 3, §4.1.1 rule 4, §5.5); `place_item` still writes the hang for every text segment and zeroes it for an atomic — the content that ends line-finality. Shaping break on non-zero **components** (§1.4's wording), so a negative or compensating pair still breaks. ⚠ **The shaping break is the one side-specific gate left, and no cell can observe that it is side-specific** (Codex R2-F3; ledger **A63**): coalescing tests `last_placed_entity == Some(entity)` (`:744`), a text run's entity is its nearest styled ancestor (`inline/collect.rs:315` at `22de3078`), so two runs that can coalesce lie outside every box whose markers separate them — a **complete** pair, over which "some marker's own side is non-zero" and "the box has some inline-axis edge" are the same condition; and `flush_line` resets the field at `:439`, so the pair cannot straddle a line either. It is written side-specifically because css-text-3 §7.3 is, not because a cell could tell — and a cell that passes under either reading is the shape §6 refuses. **One 3-valued field, not two bools**: the two readers want different questions of the same fact ("has anything entered this line?" for `finish()`, "has *content* entered it?" for the wrap guard), which is one ordered state, and CLAUDE.md's "one issue, one way" prefers encoding it once over an invariant maintained across three sites. ⚠ **`on_line` has a second reader** — the soft-wrap guard's `&& self.on_line` (`:690`) — and a marker at the head of a line would arm it, so the first content segment could soft-wrap where `on_line == false` protects it today. The guard has always meant *do not flush a line with nothing on it*, and "nothing" turns out to be the absence of **content**, not of a box boundary — so the line now has **three** states, not two. **Mechanism**: `on_line: bool` becomes `line_occupancy: LineOccupancy` (`Empty` / `BoxEdgeOnly` / `Content`), written in exactly two places **after construction** — `note_line_occupancy` **raises** it monotonically from an occupant argument, and `flush_line`'s per-line reset block (`:432-439`) sets `Empty`. ⚠ **The occupant `place_item` passes is *derived inside `place_item`*, from the arguments it already has — not a property of which caller reached it.** `place_item` takes `contributes_content: bool` (`:686`) **and** `member: FlowMember<'_>` (`:687`), and `FlowMember`'s variants (`inline/pack/items.rs:49-58`) separate `Text(&'a str)` from `Atomic` and `PositionedAtomic`. The derivation is a total match on the pair: **`(FlowMember::Text(_), true)` ⇒ M7's `RenderedText` rung; every other pair ⇒ `Content`** — the two atomic variants (whose caller passes a literal `true`, `:658`) and a `Text` segment whose `contributes_content` is `false` (a collapsible-whitespace segment) alike. The marker path, `note_line_occupancy`'s second caller, passes `BoxEdgeOnly`. **In PR-1b the ordering has no rung above `Content`, so the derivation is constant `Content` and behaviour is unchanged**; PR-1d's widening (M7) is what gives the first arm somewhere higher to go. No caller gains a parameter, and the text/non-text distinction is carried by an **enum variant** rather than by a bool two callers feed — which is why `place_item`'s literal-`true` atomic call cannot reach the `RenderedText` rung. ⚠ **The rung is named for what its predicate means, and that is not "glyph"** (round 26, Axis 2): `contributes_content` is `!text.is_empty()` under `pre`/`pre-wrap` (`pack/mod.rs:556-558`), so a lone preserved segment break — `<pre>\n</pre>`, which the code's own comment at `:547-551` names — satisfies it on a line that renders no glyph. M7's Grounds records that corner; the point here is only that "glyph-bearing", which rev 33 used at every site, named a predicate the derivation does not compute. ⚠ **Both readers are *order* tests over that ordering, and neither is an equality**: the soft-wrap guard (`:690`) holds when the line has reached **at least** `Content`; `finish()` (`:787`) when it stands **above** `Empty`. ⚠ **Their grounds are different, and only one of the two is actually about extension** (round 26, Axis 1, Gate A; an earlier drafting wrote "the ground is one and the same at both sites, and it is that the ordering is open to extension", which is true of the guard and vacuous for `finish()`). For **`finish()`** the two spellings coincide and always will: `Empty` is the ordering's **bottom**, `note_line_occupancy` only ever **raises**, and the only writer that lowers is `flush_line`'s reset back to `Empty` — so `!= Empty` and `> Empty` select the same lines under *any* extension, because an extension can only add rungs above the bottom. Writing that one as an order test buys idiom, not correctness, and its ground is uniformity: one ordering, one way of reading it. Only the **guard's** predicate is exposed, and it is exposed precisely because `Content` is *not* the top: M7's PR-1d widening adds a rung *above* `Content` and routes `(FlowMember::Text(_), true)` straight to it, so a segment that raises the top rung **never passes through `Content`**, and `== Content` goes false under the line's feet. ⚠ **And the set it goes false on is not "the lines that carry text", in either direction** (round 26, Axis 1, Gate A): by the derivation above a `Text` segment whose `contributes_content` is `false` — collapsible whitespace — derives `Content`, so a line carrying only that still reads **true**; and an atomic-only line is `Content` as well and must soft wrap. What `== Content` goes false on is *the lines that reached `RenderedText`*, which is **narrower** than "carries text"; what must soft wrap is *every* line at or above `Content`, which is **wider**. The equality therefore stops the flush on the `RenderedText` lines — the ordinary glyph-bearing ones, which is the common case and more than enough — and **soft wrapping stops engine-wide** at PR-1d. ⚠ **Rev 33 wrote the equality** (round 26, Axes 2/3, reached independently): it is the CRIT that an open ordering invites whenever a reader is written against the ordering's current top instead of against the order. Nothing in §6 caught it — cell 15b evaluates the guard exactly once, at `BoxEdgeOnly`, where both predicates agree, and cell 15 asserts that nothing wraps — so **§6 cell 15d** is added to discriminate it and §8's PR-1d gate takes it as a checkable item. ⚠ Adding it to the reset block is **behaviour-neutral today** and removes the asymmetry §4 records: `flush_line`'s three callers are `place_item`'s soft-wrap (which raises to `Content` at `:697` immediately after), `force_break` (whose `on_line = false` at `:783` becomes the same reset), and `finish()` (after which the value is never read). ⚠ **`force_break`'s *other* line-state write is dispositioned here too, and rev 33 left it unstated** (round 26, Axis 2): `any_rendered_content = true` (`:781`) stays exactly where it is. M3 moves only `place_item`'s `\|=` (`:698`) into `note_line_occupancy`, so after the extraction that field has **one writer inside the new owner and one outside it**. There is no behavioural consequence — `force_break`'s write is unconditional and its own, and the core never runs on that path — but the disposition is stated rather than left silent, because this row's whole argument against a bool pair is about write sites, and an unstated second writer leaves standing the shape the row condemns. A second bool would instead need three write sites after construction — **four to this field's three** once the constructor is counted the way §4 counts one for the sibling `any_rendered_content` (`constructor (:184)`) — plus an implicit `content_on_line ⇒ on_line` invariant, and a reset asymmetry no single field pays. ⚠ An earlier drafting compared two against three, counting construction on neither side for `line_occupancy` and on none for the bools; the contrast is stated on one basis now (round 24 audit). §6 cell 15b pins the three-state design and §6 cell 15d the order test. |
| **M4** | Where does the box's rect come from, and how do the edges reach `LayoutBox`? | **Outcome (rev 60, freeze amendment by user decision; ledger A60)** — for a box in M1's emit set, its inline-axis **content** rect on a line is the **hull of its own content-start and end-cursor points on that line and the border boxes of its descendant inline boxes and atomic inlines there** — an atomic counting as its border box **as placed on the line**: the cursor plus its margin-start, with its border-box inline size, not its margin-box advance and not the `LayoutBox` it holds at that point (which `reposition_atomic_box` has not yet moved, `inline/mod.rs:496`/`:578`, and which carries a relative offset baked in, `atomic.rs:13-16`) — every position taken **before any relative offset of children**. Grounds: CSS 2 §10.2, "The content width of a non-replaced inline element's boxes is that of the rendered content within them (before any relative offset of children)"; css-box-4 §2, whose content area "contains its content—text, descendant boxes, an image or other replaced element content, etc.". Consequences: the rect is **never negative** and **encloses every descendant's border box on the inline axis by construction** (css-box-4 §2 grounds that axis only; the block extent stays the line's, cell 13b); with nothing rendered the two points coincide, so the base case — **zero-width at content-start** (cell 14c; cells 3 and 3b's empty spans as 10c's closing ⚠ records them) — needs no rule of its own; it does not depend on which entity a run is keyed to, so a decorated box whose text sits in an undecorated or block-level child (`<a style="padding:…"><strong>text</strong></a>`, cells 24e and 6i) is not zero-width; and it **equals the rev-59 cursor span** wherever that span is not inverted **and** no descendant border box reaches outside it — every frozen cell except 10c, plus the new 10d. **Lines**: a box has a rect on each line it has a fragment on. A forced break inside it fragments it (css-inline-3 §2.1), so it has a rect on both lines (cell 17g); a soft wrap before its first character falls "immediately before … the box (at its margin edge) rather than breaking the box between its content edge and the content" (css-text-3 §5.5), so the whole box belongs to the next line and the line before has no rect (cell 17c) — M3's line-N start-edge advance, and the line-N+1 start it produces, being the registered deviation §3's css-text-3 §5.5 *adjacent soft wrap opportunity* row routes to `#11-inline-box-decoration-splits`, which also owns same-line bidi splits (css-inline-3 §2.1 Note). Boxes **outside** the emit set keep only the rects their own runs give them, a pre-existing gap booked on `#11-inline-zero-edge-box-in-item-stream` (§5.3). Everything below about *how* the rect is formed is the rev-59 mechanism; where it and this outcome diverge (10c, 10d, 17g's first line, and 17c, whose rebase this text grounds on an inversion a hull cannot produce), **the outcome governs** and the producer is PR-1c's plan-review's. <br>**Rect**: an open-box **stack** records each `InlineBoxStart`'s cursor; `InlineBoxEnd` pops it and pushes one `current_line_entity_rects` entry (`:706`) for `entity` spanning **content-start → end-cursor** (read **before** the end marker's own advance) — the box's **content** span, never inflated by edges. Pushed by an explicit branch, not the `entity != parent_entity` guard (`:703`), which does not suppress a nested inline. ⚠ **Two channels, one buffer — and PR-1c fixes only the one it can.** `getClientRects` returns `InlineClientRects` **early**, never touching `border_box()` (`crates/dom/elidex-dom-api/src/element/layout_query.rs:219-233`; ⚠ an earlier drafting wrote `:219-232`, which stops one line short of the `return` at `:233` that makes the exit early — round 24 audit), and cssom-view-1 §6 step 3 requires "one for each box fragment, describing its **border area**". `LayoutBox.content` must stay the *content* union or `border_box() = content + padding + border` double-counts. Those are contradictory demands on one value: `commit_aligned_entity_rects` builds a single `painted` rect and feeds it to **both** `EntityBounds`'s min/max bounds (→ `LayoutBox.content`, `boxes.rs:80-81`) and `EntityBounds.line_rects` (→ `InlineClientRects`, `boxes.rs:102-126`) — verified at `pack/mod.rs:488-509`. **All three buffers keep *content* spans, exactly what they hold today.** PR-1c therefore makes the **one-fragment-on-one-line** channel correct and nothing else: no `InlineClientRects` is stored below `len() > 1`, so `getClientRects` falls back to `LayoutBox.border_box()`, which M4's real edges make right. ⚠ **The more-than-one-line channel stays content-span and is a disclosed divergence, not a fix PR-1c withholds** — inflating stored fragments is `#11-inline-box-decoration-splits`'s work, on two grounds PR-1c cannot discharge: (a) **content-order identity** — `slice_and_rebase_fragment` does `b.line_rects.retain(…)` (`pack/fragment.rs:69`) immediately before the consumer (`inline/mod.rs:377` then `:380`), so under paging/multicol `line_rects[0]` is the first *kept* rect in this fragmentainer, not the box's first fragment in content order; (b) **which edge survives a break is the *parent's* inline progression direction** per css-break-3 §5.4 (§3 carries the sentence in full) — a different source from M1's own-direction side mapping and belongs with the rule that owns it. ⚠ Reachability is deliberately **not** a third reason: the edge **write** happens in `assign_inline_layout_boxes`, which `continue`s on an existing `LayoutBox` (`boxes.rs:62-64`; ⚠ an earlier drafting wrote `:60-62`, which is the `if` plus the two comment lines above it and stops short of the `continue` at `:63` — round 24 audit, applied to every site of the range), but §8 records that limit for *every* geometry PR-1c writes — M4's own edge write included — so it cannot discriminate between what stays and what leaves. §6 cell 17d pins the divergence as accepted, in the shape cell 23 already uses. The stack entry stores the box's **content-start cursor** (already past the marker's inline-start advance), so the rect is `content-start → end-cursor` with no edge re-added — the same content-span meaning `place_item`'s rects already carry. Its `block_start` is snapshotted from `current_block_offset` at the same moments `place_item` snapshots it, so all of a line's rects share one value — the invariant `commit_aligned_entity_rects` relies on. ⚠ **The two emission rules differ on emptiness, deliberately.** `InlineBoxEnd`'s pop pushes **unconditionally**, zero-width span included — an ended box *is* a fragment, and cell 14c's empty decorated inline has no other producer, so a non-empty test there would silently delete the whole presence change. The flush-time hook is the opposite: `flush_line`, at the top before any arm runs, walks the **whole** open-box stack and **emits** a partial rect only for an entry whose span is non-empty — an *open* box with zero span has not become a fragment on this line, and emitting one would duplicate the rect its eventual `InlineBoxEnd` will push (cell 17c is that case). Cells 14c and 17c each exercise one of the two rules; an implementer unifying them breaks exactly one cell. ⚠⚠ **The *ground* stated for the flush-time half is not one, and R22-d is where that surfaces.** The hook runs at flush over the boxes still **open**, so the `InlineBoxEnd` whose push it defers to is by construction on a **later** line, and `commit_aligned_entity_rects` folds per line (`pack/mod.rs:479-487`) — two lines' entries never merge, they become two `line_rects` entries. A rect the hook emitted could therefore never be the same fragment twice, which is what "duplicate" names. What the filter actually settles is the prior question: whether an open box's zero-span presence on a line is a **fragment** there at all. css-inline-3 §2.1 answers it for the box — an inline box that "exceeds the logical width of a line box, or contains a forced line break" is "split … into several fragments … partitioned across multiple line boxes" (`body css-inline-3 line-boxes`) — and cssom-view-1 §6 step 3 asks for "one for each box fragment… (including those with a height or width of zero)". The **outcome** is pinned at 17c and this row does not reopen it; what is withdrawn is the reason recorded beside it, which an implementer builds the branch from. ⚠ **The forced break is the sharp instance, and the finding's own fixture does not reach it.** On `<pre><span style="padding-left:10px">\nx</span></pre>` the preserved break is not transformed to a space, so `find_break_opportunities` marks it `Mandatory`, `force_break` runs (`:781`) and line 1 takes `flush_line`'s **commit** arm (`:209-210`) — a line that exists for no reason but the box. But the `"\n"` segment is its own `PackItem` (`items.rs:74-83`, the `bp > prev_pos` guard keeping only non-empty slices) and its run carries the **span's** entity, so `place_item`'s `entity != self.parent_entity` push (`pack/mod.rs:703-714`) already gives the span a line-1 rect: the fragment is not lost there. Reaching the loss takes an outer box with no run of its own across the break — the inner-element shape — and which of the two instances the hook's branch is written against is PR-1c's plan-review's, on a ground that holds. It also **rebases every entry's content-start to 0 unconditionally** — the two scopes differ, and binding the rebase to the emitted set would leave a box opened at the end of line N (span 0, no rect) holding a line-N cursor into line N+1 — which under the outcome's hull widens line N+1's rect rather than inverting it (cell 17c). §6 cell 17c pins it — the start edge M3 advanced on the earlier line (the registered css-text-3 §5.5 deviation, `#11-inline-box-decoration-splits`) is not applied again, and a stale line-N content-start would otherwise enter line N+1's hull. The end edge is symmetric and needs no handling: an open box has not reached its end marker, so line N's partial rect reserves nothing for it, so a box straddling a break yields one rect per line and a box opened exactly at a soft wrap yields none — which the outcome above now grounds in css-text-3 §5.5 (the break is at the box's margin edge) rather than only in the emit filter. ⚠⚠ **The hook has a third job, and it is not geometry: it *replays* each still-open box's per-line metric contribution** (R7-a). M6 applies `block_advance` and M7 records its tentative baseline only where `InlineBoxStart` is consumed, while `flush_line`'s reset zeroes `current_line_height` (`:433`) and clears the tentative; an outer decorated box that stays open across a soft wrap therefore contributes to its **first** fragment alone, where css-inline-3 §5.3 gives every participating box its own bounds and css-inline-3 §2.2 **step 3** composes **each** line from all of them. The stack this row already walks is the register of exactly which boxes are open, so the walk is the trigger and the mechanisms keep their fields: one `note_line_occupancy(0.0, block_advance, None, false, BoxEdgeOnly)` per open entry (M3's owner, M6's value). ⚠ **R7-a paired that with a second replayed value — M7's tentative baseline — and R10 withdraws it**: the promote it feeds cannot fire on a continuation line, no licensed break opening one that carries no rendered text (measured; `#11-inline-open-box-strut-on-continuation-line`, §5.3). The hook replays the **height** contribution and nothing else, so the per-open-entry call above is the whole of it. ⚠ **Unlike the emit and the rebase this runs at the *bottom* of `flush_line`, after the reset block (`:432-439`)** — a replay before the reset is erased by it — so the hook is two passes over one stack and not one, which is the shape an implementer gets wrong. Each argument is ground, not convention: `inline_advance` is `0.0` because the rebase already carries the cursor and the start edge was consumed on the earlier line (cell 17c's own rule), re-adding it being the double-count that rule exists to prevent (ledger **A23**); `hang` is `None` because the replay follows no space; and `contributes_content` is **`false`** because a *middle* fragment of an open box carries neither of its inline-axis edges — css-break-3 §5.4 attributes a broken edge to one side of the break — so the replay restores the box's **height** without touching **existence**, and PR-1d's flip set is exactly what §8 states. It is staged like every other marker value under §8's dead-field rule: the call site lands with the hook in **PR-1c**, where `block_advance` is `0.0`, so it is a no-op there, and **PR-1d** supplies the value. §6 cell **24e** pins it. ⚠ **The marker's rect is a second producer for the same entity**: a decorated inline with text already has a `place_item` rect on that line (`:706`, its runs carry `entity == span`). The **persisting** arm folds per entity before committing (`commit_aligned_entity_rects`, `:479-487`), so one fragment per line survives — correct, and the only arm that runs. The non-persisting arm — the whole `else` clause at `pack/mod.rs:393-421`, whose unmerged rect loop is `:401-420` — does **not** fold, but it is **dead code**: `FragmentationType` has exactly `Page` and `Column` (`crates/layout/elidex-layout-block/src/lib.rs:37-42`) and `InlineFragConstraint.fragmentation_type` is non-optional (`inline/mod.rs:94`), so `persist_candidate = frag_constraint.is_none() \|\| frag_is_paged \|\| frag_is_column` (`:239`) is **identically true** and `flow_align` is always `Some`. No cell is written against that arm: a cell no markup can construct is exactly what M8's grounds refuse. The dead arm is deleted by a prereq PR (§9). **Edges**: **one derivation site; the box-assigner stays a marshaller; the carrier between them is PR-1c's own memo's choice, not this one's.** ⚠ **Why this half is delegable and the *rect* half above is not**: the rect answers §2's couplings **3×7** and **2×3** and is per *line*; the edges answer **no §2 pair at all** and are a per-*element* constant. Read §2's pair table — **no pair names edge *delivery***, i.e. no pair asks how an element-constant value gets from its derivation site onto `LayoutBox`. **Three kinds of row come close, and the disambiguation is stated rather than assumed** (§2's rows; ledger **A65**). (a) **3×7** — one of M4's two pairs; **2×3**, the other, names a commit decision — reads "The box's rect must be a *content* span, **with edges carried separately**, or the border box double-counts": that names edge ***separation*** as a constraint on the per-line rect (keep the edges *out* of it), not a carrier for the constant; the clause is satisfied by any carrier whatever, which is exactly why it cannot choose one. (b) **2×6** names `group_key` ***delivery*** — the right shape — but it is routed to `#11-inline-box-decoration-splits`, not to an M-row here, so it does not reach this half either. (c) **2×4** and **2×5** name delivery too — of the box's font and `line_height` fields **to the packer**, for M7's strut and M6's height — not of an element-constant onto `LayoutBox`. The other fifteen (1×2 twice, 7×2, 1×3, 1×5, 1×4, 1×4 fallback-only half, 7×intrinsic, 1×7 twice, 3×4, 3×5, 4×5, 4×7, 5×7) name an emit set, a collapse barrier, an advance/shaping break, an existence flip, a height source, a baseline source twice — the glyphless condition and the fallback-only one — an intrinsic-size contribution, a shared predicate, occupancy, discard of a tentative baseline and of a height, a pre-existing composition, and the occupancy ordering and its single writer. ⚠ **"The other seven" of "nine" until rev 34**, because G1's `1 × 4 (fallback-only half)` row was added to §2 in this revision and the enumeration behind the count was not re-run (round 26, Axis 5, Gate B); the conclusion is unchanged, since a *baseline source* is not a delivery shape either. ⚠ Two earlier draftings: the first reached the conclusion by a universal — "every pair names a rect, a predicate or an advance" — which §2's own table refutes (1×4 names a **baseline source**, 1×5 a **height source**, 1×2 a **collapse barrier**, and 2×6 a ***delivery***, the very shape the universal claimed no pair has); the second replaced it with "2×6 is the one delivery-shaped pair", a second universal that left 3×7's own edge-carriage clause unswept. The conclusion is unchanged; the argument is now the enumeration plus the 3×7 disambiguation rather than either universal (round 24 audit; round-24 gate). So M4's two halves differ on lifetime, producer, write site and consumer, and only the rect half carries an umbrella-owned coupling. (§2's "Each **row** is answered by exactly one M-row **or by exactly one slot**" therefore does **not** converse: an M-row may carry a mechanism that answers no pair, and this is the one.) That asymmetry, not "altitude" in the abstract, is why two prescribed carriers were falsified in the same direction — each was an error about the *element-constant* half reasoned through the *per-line* pipeline the paragraph above had just established. <br>`resolve_box_model` runs **once per decorated inline**, at collect time, and its triple rides M1's marker payload (PR-1b). How it gets from the payload onto `LayoutBox` is PR-1c's interior mechanism, and this memo stops at the **invariants** that mechanism must satisfy: <br>(i) exactly **one** `resolve_box_model` call per decorated inline per pass — the marker payload is the single derivation site, so no second site can disagree with it; <br>(ii) `assign_inline_layout_boxes` performs **no `ComputedStyle` fetch** — its `boxes.rs:57` touch stays the `is_err()` guard it is today. A box-assign-time fetch would be a live read *after* the whole packing pass, against M1's collect-time clone (`inline/collect.rs:217`), and nothing pins the two to agree; <br>(iii) the three sets stay **separate** all the way to `LayoutBox`'s three **independent** fields — `padding` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:88`), `border` (`:90`), `margin` (`:92`) — consumed separately by `padding_box`/`border_box`/`margin_box`. ⚠ The ground is those three fields, **not** M5's disjunction: M5 is evaluated on the marker payload in `pack/inline_box.rs` (PR-1b), *upstream* of the carrier, so it would be satisfied even by a carrier that merged the sets. §5.1 M1's grounds column carries the binding ground; an earlier drafting of this clause cited M5, which does not reach here; <br>(iv) the value is an **element constant** and must arrive regardless of which line carries `InlineBoxEnd`; <br>(v) ⚠ **the set of entities receiving a `LayoutBox` must not grow beyond the domain `entity_bounds` already admits** — concretely, `assign_inline_layout_boxes`'s iteration domain stays `entity_bounds` under every carrier, and no carrier writes a `LayoutBox` outside that loop. ⚠ Stated at the **grant**, not at the `entity_bounds` entry: an earlier drafting forbade only "creating an `entity_bounds` entry", which option **C** can satisfy to the letter while still doing the harm — walk the item slice and write each `InlineBoxStart`'s edges straight onto that entity's `LayoutBox`, creating no entry at all and still granting a box on a phantom line. This is §2 invariant 3 (commit-on-content), which M4 **owns** through pairs 2×3 and 3×7, so it is not delegable. Measured: every `entity_bounds` write today is reached from `flush_line`'s commit arm via `commit_aligned_entity_rects` (`pack/mod.rs:496`, in the function at `:468`, called at `:344`; `:404` is the dead non-persist arm — ⚠ neither line is lexically inside `flush_line`, round 24 audit), the discard arm (`:428`) touches `entity_bounds` not at all, and `assign_inline_layout_boxes` iterates `entity_bounds` (`boxes.rs:56`) skipping only entities with no `ComputedStyle` (`:57`) or an existing `LayoutBox` (`:62-64`) — **not** entities with degenerate bounds. A carrier that writes `entity_bounds` from the marker path in `pack()` runs per *item*, outside the commit/discard decision entirely, and would grant a box on a phantom line: that breaks §6 cell 6's "phantom ⇒ no box" and ships PR-1d's presence change inside PR-1c. §5.3's PR-1c bullet ("only on a line that already exists") is the statement (v) protects. <br>⚠ **"and no new parameter" is NOT on that list, and carrying it as though it were is what pre-decided the carrier.** It is a tie-breaker at most; stated as an invariant it silently excluded two of the three options below. <br>⚠ **Two measured facts constrain the choice, and the first of them falsifies the carrier earlier revisions prescribed.** (a) `commit_aligned_entity_rects` (`inline/pack/mod.rs:468`) is handed **no marker/text discriminator**: it drains `current_line_entity_rects`, a `Vec<(Entity, InlineLineRect)>` (`:121`, `:480`), and folds it into `merged: Vec<(Entity, f32, f32, f32)>` (`:479-487`) with no edges slot — so "written on the `InlineBoxEnd` path in `commit_aligned_entity_rects`" names a path that function cannot tell from any other. (b) The fold collapses one entity's per-line entries **before** the `entity_bounds.entry()` that consumes them (`:496`; the file's only other `entry()` is `:404`, the dead non-persist arm — ⚠ an earlier drafting cited `:488` as an `entry()` site, but `:488` is the `for (entity, …) in merged {` head that walks the fold's output, round-24 gate), so two producers on one line cannot order against each other at all; the surviving ordering obstacle is **cross-line** — `or_insert` fires on the first line (`:505`) while `InlineBoxEnd` may be on the last. ⚠ State (a) as a **timing** fact, not a missing-field one: that function runs at **flush, per line**, while markers are consumed in `pack()`, per *item*. "No discriminator" invites adding one, which would push an element constant through `InlineLineRect` and the `merged` tuple and re-bundle the two halves this row just separated. <br>⚠ **Three options, not two, and the reopening is not a re-prescription.** (**A**) widen `EntityBounds` (`inline/pack/boxes.rs:30-41`); (**B**) a second `Entity → (EdgeSizes, EdgeSizes, EdgeSizes)` map threaded to the box-assigner; (**C**) hand `assign_inline_layout_boxes` the `&[InlineItem]` slice and read the marker payload there — `items` is a local `Vec<InlineItem>` (`inline/mod.rs:153`) that every later use only *borrows* (`:161`, `:179`, `:190`, `:192`, `:200`, `:248`, `:255` — the enumeration is `grep -nE '\bitems\b' inline/mod.rs` minus the binding at `:153`; an earlier drafting omitted `:161`'s `items.is_empty()`, an enumeration standing behind an "every", round 24 audit), and `PackItem` has no lifetime parameter (`pack/items.rs:18-35`, `item_index: usize`), so it is live and immutably borrowable at the assign call (`:380`). Costs, stated symmetrically: **A** must reach its write site *through* the fold, so by (a) it is not one widening but three — `InlineLineRect`, the `merged` tuple (`pack/mod.rs:479`) and `EntityBounds` — plus a rule for what an edges slot means when the fold merges two entries for one entity; **and its other implementation, writing `entity_bounds` from the marker path in `pack()`, is barred outright by (v)**. **B** adds one parameter and one structure, satisfies (v) because the assign loop's domain stays `entity_bounds`, and survives `fragment.rs:68`'s `retain` untouched. **C** adds one parameter and no *signature* structure, and satisfies (iv) structurally — the item stream has no line concept at all. ⚠ **Its two real costs are not in the parameter list.** (1) It puts a marker read and a per-entity edge derivation inside `assign_inline_layout_boxes` (`pack/boxes.rs`), which is a **second site reading off a marker** — §5.2 designates `pack/inline_box.rs` the one such site, and M4's own heading says the box-assigner stays a marshaller, so C requires amending one or the other rather than slotting in free. (2) `assign_inline_layout_boxes` iterates a `HashMap` in arbitrary order (`boxes.rs:56`), so reading the slice per entity is either an O(entities × items) rescan or an internally-built entity→edges map — i.e. **option B's structure, moved out of the signature and into the function**. "No structure" is a property of the parameter list, not of the mechanism. ⚠ What is withdrawn is the *prescription*, not replaced by a new one: an earlier revision rejected **B** on the ground that "the carrier exists", meaning `EntityBounds`, and (a) falsifies that premise. PR-1c's memo weighs all three. <br>⚠ Separately withdrawn: the rejection ground earlier revisions gave against `EntityBounds` itself ("`or_insert`/`and_modify` would not re-set an element-constant on a multi-line inline") — `entity_bounds` is keyed **per entity, accumulated across lines** (`:496-511`), which is exactly what an element constant wants. | `assign_inline_layout_boxes` writes bounds into `LayoutBox.content` (`:81`) and `border_box() = content + padding + border`, so an edge-inflated rect double-counts — which is why the border area is derived at the write rather than stored in the accumulator both consumers read. Reading the end-cursor before the end advance is what keeps the end side from double-counting too. The flush-time **emit** is what a rebase-only version misses: `InlineBoxEnd` has not run at line N's flush, so line N's fragment would simply be lost. ⚠ **The carrier ground this column used to carry is withdrawn, not weakened**: "a `&HashMap<Entity, (EdgeSizes, EdgeSizes, EdgeSizes)>` parameter would add a *second* structure beside `entity_bounds` … so the carrier exists and threading a map is the option that buys nothing" rests on `EntityBounds` being writable from the marker, and its write site cannot see the marker (Decision, fact (a)). Nothing replaces it **here** — the choice moves to PR-1c's memo with the measured constraints, which is the altitude a per-PR interior belongs at. `resolve_box_model` (`helpers.rs:116`) is pure and `pub`-exported (`lib.rs:25`, with an existing cross-crate caller at `crates/layout/elidex-layout-grid/src/lib.rs:177`), so calling it once and carrying the result is free of any re-entrancy concern. ⚠ CLAUDE.md's side-store→component rule does **not** reach any of the three options, and the ground is *recorded*, not argued: [[ecs-native-side-store-audit-2026-05-21]] already adjudicated `elidex-layout` **clean** on this axis — "layout 計算の局所 scratch (children / rows / pool / run) のみ". `entity_bounds` is exactly that: a field of a stack-allocated `LinePacker` (`inline/mod.rs:251`), initialised empty (`pack/mod.rs:178`) and dropped when the IFC pass returns, whose *destination* is the `LayoutBox` **component** (`boxes.rs:88`, `dom.set_layout_box`). The rule's subject is a persistent entity-keyed registry and all three payoffs it names — SameObject via component get, GC as one query, despawn cleanup — are persistence properties with no meaning for a value that does not outlive one call. ⚠ Option **B** is *literally* the `HashMap<entity, _>` shape the rule names, so the exemption is stated at that shape and not only at the rule's subject; and `assign_inline_layout_boxes` already takes `entity_bounds: &HashMap<Entity, EntityBounds>` (`boxes.rs:50`), which the rule would condemn on `origin/main` if it reached.  ⚠ **There is no second derivation site to keep in step**, which is the point: percentage edges (cell 4) are resolved once against `layout_inline_context_fragmented`'s own `containing_inline_size` (`inline/mod.rs:144`, reaching `collect_inline_items` at `:154`), and every consumer reads that one result. That holds because the derivation happens **once, at collect time**, whatever carries the result afterwards — it is invariant (i), and it does not depend on `assign_inline_layout_boxes`'s parameter list. ⚠ **This row owns a disclosed divergence it did not name until R5** (and it is the row an implementer builds the mechanism from, which is why naming it only at the cell was the defect — every other site listed 13b and this one did not). M4's rect is the box's *content* span; the **block** extent `commit_aligned_entity_rects` writes for it is the *line's* height (`block_size: line_height`, `inline/pack/mod.rs:493`), where css-inline-3 §5.3 composes the line **from** each box's own bounds and never assigns the line's extent back. That is **facet (b)** of `#11-inline-root-inline-box` (§5.3), pinned by **§6 cell 13b**. Pre-existing — the write is reached through `place_item`'s `entity != parent_entity` push at `:706`, so an undecorated span already gets it — so M4 neither creates nor fixes it; what M4 changes is the population that reaches it and the channels that observe it. |
| **M5** | What makes a line non-phantom, and what else commits? | **The clause-3 predicate is a function of the marker's edges, not a per-PR constant**, and it is a **disjunction, not a sum**: `contributes_content = has_inline_axis_edge(marker)`, true iff **any one** of the three `LogicalEdges` (padding, border, margin — each converted separately by `LogicalEdges::from_physical`) has a non-zero `inline_start` or `inline_end`. **M5 owns this predicate, and it lands in PR-1b** — M3's shaping break is a PR-1b deliverable and reads it, so the predicate cannot wait for PR-1d (ledger **A63**). ⚠ **It reads it per marker side, not as the whole-box disjunction** (Codex R2-F3): the derivation site produces the two side halves — *some component's `inline_start` is non-zero*, *some component's `inline_end` is non-zero* — and `has_inline_axis_edge` **is** their disjunction, so there is still one derivation and **two readings** of it: M3's shaping break takes the half the marker in hand contributes, and the existence flip together with `inline/mod.rs:200`'s escape take the whole. ⚠ **Two *readings*, two *call sites* — the two counts are of different things and R5 says so rather than leaving them beside each other**: the readings are uses of the derived facts, the two are the places the free function is invoked from (the pre-pack gate, holding `&InlineItem`, and the packer, resolving a `PackItem`'s `item_index`); the existence flip and the boundary gate are both reached through the packer's one call. What PR-1d adds is the one-argument *substitution* below, not the predicate. M1's emit test (any side) is a different test on the same payload and stays in PR-1a. Evaluated per marker as a **free function over the marker's `InlineItem` payload**, in `pack/inline_box.rs` beside the stack — deliberately *not* an `impl LinePacker` method, because the pre-pack gate at `inline/mod.rs:200` must call it too and runs before `LinePacker::new` (`:251`). One derivation site, two callers: the gate holds `&InlineItem` directly, the packer resolves its `PackItem`'s `item_index` into the same slice (M1). ⚠ The gate's escape is **qualified**, not a bare "marker escapes beside `Atomic`": an unqualified escape would let a marker that does **not** satisfy clause 3 — a `padding-top`-only box — carry a line by itself, which is exactly what M1, §1.2's clause-3 row and cells 5/6 forbid. What the `:200` early return is doing today is **not** clause enforcement at all: it is an engine **measurability** guard (no usable font ⇒ nothing can be measured ⇒ return `line_count: 0`), and §6 cell 12d records that as a divergence, since clause 3 is font-independent. PR-1d lifts it **only where clause 3 licenses it** — i.e. behind `has_inline_axis_edge` — which is why the escape carries a predicate rather than matching on the variant. ⚠ An earlier drafting grounded this on "font-less text commit[ting] a line that §1.2 says stays phantom": §1.2's clause 1 is `contributes_content`, "a pure text predicate with no font dependence" (`pack/mod.rs:556`) by this row's own words, so a line whose text is font-less is **not** phantom under §1.2 — the attribution ran the wrong way (round 24 audit). In PR-1b the marker path passes a constant `false` (geometry only); **PR-1d replaces that constant with this expression** — that one-argument substitution *is* the existence flip. Once a line is non-phantom, css-inline-3 §2.3's "the line box **and its in-flow content**" applies: every entity on it commits through the existing seam (`:210`), none withheld. | A literal `true` for PR-1d would keep a `padding-top`-only inline's line alive — contradicting §1.2, M1 and cells 5/6. Emit and existence are different predicates over the same payload, so the existence one needs its own site. **Disjunction, not sum**: css-inline-3 §2.3 lists "non-zero **inline-axis** margins, padding, or borders" — three separately-named quantities, so `margin-left:-10px; padding-left:10px` — which sums to zero — still keeps the line. This is the one place the three sets must stay separate; M3's *advance* and M4's *content offset* both take the sum, because geometry adds up and a negative margin really does pull content back. §6 cell 3b pins the cancelling pair. |
| **M6** | Height of a line kept only by decoration | `InlineBoxStart` carries the marker payload's `line_height` **and `font_size`** — two of the five font-and-height fields M1 names, not a "font identity" gloss, because M3's shared core takes `current_line_height = max(block_advance)` with `block_advance` following the packer's existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`), so **both** arms of that convention are payload reads and a payload carrying only `line_height` cannot serve the vertical one (round 26, Axis 2). ⚠⚠ **The contribution is per *line*, not per start marker** (R7-a): css-inline-3 §2.2 step 3 composes **each** line box from the bounds of every box participating in it, and a box that stays open across a soft wrap participates in its continuation lines too. So `block_advance` is applied at `InlineBoxStart` **and re-applied, through the same `note_line_occupancy` owner, on every subsequent line the box is still open on** — M4's flush hook is the trigger and its Decision carries the argument list. Reading this row as "the value the start marker supplies" is what let an outer `line-height:100px` box wrap around an inner `line-height:10px` one and leave the continuation line at 10px; §6 cell **24e** is the pin. **The memo does not claim css-inline-3 §5.3/§2.2 conformance**, and the divergence has **two** facets on this row, not one. (i) The block container's **root inline box** (§1.1) is unimplemented, so the line's height floor is missing; the text path already diverges identically — `<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today, with no marker involved. (ii) ⚠ **In a vertical writing mode the convention above reads `font_size`, so an empty box's `line-height` does not influence the line at all** (Codex R2-F2) — the very calculation css-inline-3 §2.2's Note says it influences, and css-inline-3 §5.3 applies in vertical modes too. This is pre-existing on the text path for the same reason both arms are payload reads. ⚠ **Neither is "fixed" by giving the marker `line_height` in the vertical arm**: `block_advance` feeds one shared `max()`, so a marker reading `line_height` beside co-resident text reading `font_size` on the same vertical line trades a disclosed divergence for an undisclosed incoherence. Slot **`#11-inline-root-inline-box`**, pre-existing class — **whose subject §5.3 widens to the whole css-inline-3 §5.3 layout-bounds model**, these two being its facets (a) and (c); §6 cells 23 and 23b pin them so they stay distinguishable from a bug. | The direct authority is `body css-inline-3 line-layout` (css-inline-3 §2.2) Note: "Empty inline boxes still have margins, padding, borders, and a **line-height**, and thus influence these calculations just like boxes with content." css-inline-3 §5.3 defines the strut per *box* and does not address the line-level question. Disclosing a pre-existing divergence the new code depends on is the §4.3 pattern. ⚠ **This row's Decision says "two facets *on this row*", which is a scope statement and not a count of the slot's** (R5): M6's scalar `max(block_advance)` is also one of the two mechanisms facet **(e)** is about — a decoration-only line carrying more than one glyphless box, where the scalar carries one box's `line-height` and the composition is over all of them — and that facet is disclosed on **M7's** row, with cell 24d headed for both. §5.3's facet list names M6 and M7 as (e)'s owners accordingly. |
| **M7** | Which line gets a strut baseline | `InlineBoxStart` records a tentative `current_line_box_baseline: Option<f32>` from the box's first-available-font metrics via `FontDatabase::query(families, weight, style)` + `font_metrics(id, font_size)` (`crates/text/elidex-shaping/src/database.rs:60`, `:101`) — **the payload's `families` / `font_weight` / `font_style` / `font_size`, named by M1 rather than glossed as "font identity", because all four are arguments of those two calls** (round 26, Axis 2) — keeping the `!is_vertical` guard (`:575`). ⚠⚠ **The tentative is per *line*, and PR-1d records it at `InlineBoxStart` only** (R7-a, narrowed at R10): `flush_line`'s reset clears it, and a box open across a soft wrap has no second `InlineBoxStart` to record it again, so R7-a had M4's flush hook re-record it from the same payload for every still-open box after the reset — on the ground that an open glyphless box otherwise offers its strut to its first fragment and to no continuation line, against css-inline-3 §5.3, which gives the strut to the **box**. **That re-record is withdrawn from PR-1d and routed to `#11-inline-open-box-strut-on-continuation-line`** (§5.3): the promote it would feed cannot fire on a continuation line, because no break css-text-3 §5.5 licenses opens one that carries no rendered text — the continuation line's first member is always the tail segment of the run the break falls inside, and that tail always raises the `RenderedText` rung this row's own second conjunct vetoes on. The argument and the sweep that controls it are the slot's. What M4's hook still replays is M6's **height** contribution, and §6 cell **24e** — which pins that and states in its own text that the baseline half has no oracle on it — is the pin. ⚠ **Until R10 this sentence read that 24e "pins the height half and the baseline half together"**, which R9's own edit to that cell had already falsified. `flush_line` promotes it into `first_baseline` **inside** the `if self.any_rendered_content` arm (`:210`) — never on a suppressed line — and only when **`first_baseline.is_none()` and the line's occupancy never reached the `RenderedText` rung** (the ⚠ in the Grounds column derives the second conjunct: `first_baseline.is_none()` alone does not answer "did a text segment the engine counts as rendered content land here" when no font is usable). ⚠ **The rung is named `RenderedText`, not "glyph-bearing", and the name change is not cosmetic** (round 26, Axis 2): M3 derives it from `(FlowMember::Text(_), contributes_content)`, and `contributes_content` is **not** a glyph predicate — under `pre`/`pre-wrap` it is `!text.is_empty()` (`pack/mod.rs:556-558`), so a lone preserved segment break reaches the rung on a line that renders no glyph. The derivation is unchanged; what changes is that the rung is now named for the predicate that computes it. The Grounds column below states why mirroring `contributes_content` here is *correct* and not merely convenient. ⚠ **That conjunct is not a second bool. It is one more state in M3's monotone `LineOccupancy` ordering** — the line carried *no* occupancy, *non-text* content (an atomic, a marker's edges) or *rendered text* — raised by the **same `note_line_occupancy`** M3 makes the single writer, from **the structural discriminator `place_item` already holds**: M3's derivation of the occupant inside `place_item` from `member: FlowMember<'_>` (`:687`) and `contributes_content` (`:686`) — `(FlowMember::Text(_), true)` ⇒ this rung, every other pair ⇒ `Content`, the marker path ⇒ `BoxEdgeOnly`. PR-1d's whole addition to M3 is giving that first arm a rung above `Content` to raise to. The promote reads that state; nothing reads a flag. ⚠ **An earlier drafting** (rev 33) is withdrawn, and it failed the same way the bool it replaced did (rev-33 gate; ⚠ the marker used to read "What rev 33 wrote here", which was self-referential in the revision that wrote it — round 26, Axis 4 — and the sibling withdrawal below already carried the right form). It said the rung is raised "at the one site where the value means glyph text: the `PackItem::Text` arm's own `contributes_content` (`pack/mod.rs:556-568` → `:597`)". But `place_item` has **exactly two** call sites — `:591` from the `PackItem::Text` arm, passing that predicate at `:597`, and `:652` from the `PackItem::Atomic` arm, passing a literal `true` at `:658` — and they feed **one** parameter (`:686`) and one `note_line_occupancy` call. "Reached only from the Text arm's call" therefore names **no condition the code can test**: at the raise site the two calls are indistinguishable, which is precisely the under-determination of the rev-32 bool, one level down. The fix is not a new parameter or a caller contract but the discriminator that was already in the signature beside it — `FlowMember`, an enum whose variants (`inline/pack/items.rs:49-58`) *are* the Text/atomic distinction. **The state therefore adds no new field to `flush_line`'s per-line reset block (`:432-439`)** — M3 already resets the occupancy there and that reset covers it, so **only the tentative** joins the block. The ground is the encoding, not the fact: a second bool would carry an implicit `any_glyph_text ⇒ any_rendered_content` invariant across three write sites, which is exactly the argument M3's own row makes against an `on_line`/`content_on_line` pair, and CLAUDE.md's *one issue, one way*; and the split encoding is what made the raise-site defect below possible, because a raised **ordering** cannot be claimed by the marker path — that path raises to the *non-text* state by construction. ⚠ **An earlier drafting** (rev 32) instead added a separate per-line `any_glyph_text: bool` "raised by `\|= contributes_content` at the `place_item` site", and said **two** fields join the reset block. Both are withdrawn, and the raise site is why: M3 moves `any_rendered_content \|= contributes_content` (`:698`) **into** `note_line_occupancy`, whose **second caller is the marker path**, and M5 has PR-1d replace that path's constant `false` with `has_inline_axis_edge` — true for every decorated inline. A decoration-only line would have raised the flag, `!any_glyph_text` would be false, the tentative would never promote, and M7's whole mechanism would be **unreachable code**. The other reading is no better: `contributes_content` is a `place_item` **parameter** whose atomic caller passes a literal `true` (`:658`), so an `<img>` would have counted as glyph text (round 25, Axis 2). ⚠ **The state is a fourth `LineOccupancy` variant, decided here and not delegated** (round 26, Axis 2). An earlier drafting left it open — "a fourth variant or a sibling monotone enum written by the same function is mechanism for PR-1d's own memo" — and the memo's own three statements already foreclose the second branch: (i) M3's signature takes `occupant: LineOccupancy` and M7's rung is passed as *that* argument, so a sibling enum is not a value the single writer can accept; (ii) §8's PR-1d DoD closes with "No field joins the `:432-439` reset block for this gate — M3's reset covers the widened state"; and (iii) §5.4 records "the widening is PR-1d's and adds no field". A sibling enum **is** a second per-line field and needs its own reset, so it fails (ii) and (iii) and cannot be handed to the same writer under (i). An open encoding whose two branches differ in *correctness* is a decision surface, not a delegation — CLAUDE.md's *ideal over pragmatic*. What this memo decides is therefore **one monotone-raised ordered state — a fourth `LineOccupancy` variant, `RenderedText` — one writer, never a bool pair**. ⚠ A previous drafting also stated the condition as `first_baseline.is_none()` alone and "the field" as one, leaving the Decision asserting the rule its own Grounds falsify while §6 cell 24 appeals to this Decision as the design authority (round-24 gate). ⚠ **Scope: the occupancy state is per *line* while `first_baseline` is per *IFC* (`pack/mod.rs:125`, initialised at `:187`, set at `:586`/`:631`, and deliberately **not** in the `:432-439` reset block), so this rule binds within a line and not across them.** ⚠ **The cross-line instance that used to be written here does not exist** (R10): it read "with a font-less text line 1 and a decoration-only line 2, `first_baseline` is still `None` at line 2's flush and the tentative promotes", and a decoration-only line 2 is unconstructible — under every break css-text-3 §5.5 licenses, a decoration-only line is the IFC's **only** line, because the run whose text licenses the break puts its first segment on the line before it. What the promote's domain therefore is: a **single** line kept by decoration alone (Shape B, cell 12), where `first_baseline.is_none()` holds by construction and the tentative promotes, giving the IFC a first baseline from that line's box. **That is the intended behaviour, not a residual**: §1.5 makes the strut a property of the *box*, the line genuinely carries no glyphs, so its box's strut is the only baseline it can offer — and what M7's rule exists to prevent is a strut *displacing* a line's own glyph baseline, which that line has none of (⚠ that is the rule's **motivation**, not a conformance claim: on a *mixed* line the veto is a disclosed under-approximation, and the Grounds column states why — Codex R2-F4). What is left undone is the font-less **text** line's own missing baseline — cell 24's line, where the veto holds and `first_baseline` stays `None` — which no signal in the engine can supply and which this program does not create (round 25, Axis 2). §4 enumerates that block's current contents; **no site states a running total** — a count restated away from the enumeration it summarises drifts from it. | §1.5: a strut exists only for a glyphless box; css-inline-3 §2.2 owns the line-level composition. ⚠ **The veto is an approximation, and this row stops arguing it as the spec's rule** (Codex R2-F4). css-inline-3 §5.3 assigns the strut **per glyphless box** and composes the line from every box's aligned layout bounds (css-inline-3 §2.2 step 3), so on a **mixed** line — a glyphless decorated box beside text — the box's A′/D′ must participate; css-inline-3 §5.3 licenses neither M7's veto nor M6's scalar `max`. ⚠ **The promote is an approximation on every line, decoration-only included** (R4): a decoration-only line can hold **more than one** glyphless box — css-text-3 §5.5 makes an inline box boundary no soft wrap opportunity, so `<p><span style="padding:1px;font-size:100px;line-height:10px"></span><span style="padding:1px;font-size:10px;line-height:10px"></span></p>` is one line with two of them — and css-inline-3 §5.3 gives each its own strut, so one tentative `Option<f32>` carries one box's metrics where the line is composed from both. css-inline-3 §5.3's `A′ = A + L/2`, `D′ = D + L/2` over the two spans' first-available-font metrics puts **the maximal ascent on the first and the maximal descent on the second** — the first's `line-height` of 10 is far below its 100px font's `A + D`, so its half-leading is large and negative and its own `D′` goes below zero — so neither a single tentative nor a scalar `max` can carry both. ⚠ **No composed figure is stated, and the one that stood here until R5 was wrong in a way that also shows why the memo should not carry it**: it read "the aligned composition is 41.201 against M6's `max(line-height)` of 10", composed over the **two spans only**. The `<p>` generates a **root inline box** (§1.1), which css-inline-3 §2.2 **step 2 names in its own bullet** and step 3 sizes the line over, and which — glyphless, at the inherited font size, `line-height: normal` — contributes a `D′` deeper than either span's, so the composition is over three boxes and is neither span's figure nor that sum. Its value is not even determinate: css-inline-3 §5.3 says the font's line gap "**may**" be added as half-leading on each side when `line-height` computes to `normal`, so a conforming UA has a range rather than a number. What survives is model-level rather than metric-level, and it is what the cell needs: maximal `A′` and maximal `D′` fall on **different boxes**, and their **sum** — the line's composed extent — exceeds the scalar `max(line-height)` of 10 by the whole of the first span's ascent (⚠ *the sum*, not each of them: the maximal `D′` here is a few pixels and does not exceed 10 on its own, which an earlier phrasing of this correction claimed). Cell 24d reads the metrics in the test. That is facet (e) of `#11-inline-root-inline-box`, not a new mechanism here. On a mixed line elidex has no per-box layout-bounds model to compose with, so it suppresses the strut rather than mis-positioning the line's baseline — a deliberate **under**-approximation, chosen in that direction for the displacement reason the Decision gives, which survives as the motivation and not as conformance. The divergence is accepted for this program and is facet (d) of `#11-inline-root-inline-box`, whose subject §5.3 widens to the whole css-inline-3 §5.3 model; §6 cell 24c pins it. ⚠ **Why `contributes_content` is the right predicate for the rung, and not merely the one at hand** (round 26, Axis 2). The question the second conjunct asks is "did a text segment that *would have* set this line's baseline land here?" — and the engine's own answer to "does this segment set the baseline" is `contributes_content`: the capture at `:575` is gated on exactly `if contributes_content && self.first_baseline.is_none() && !is_vertical`. Mirroring that gate is therefore **consistent by construction**, not an approximation of a glyph test; a glyph predicate would be a *second, different* answer to a question the capture already answers, which is the shape M3's row and CLAUDE.md's *one issue, one way* both refuse. ⚠ **The `<pre>\n</pre>` corner is inherited, not created.** Under `pre`/`pre-wrap` `contributes_content` is `!text.is_empty()` (`pack/mod.rs:556-558`), and the code's comment at `:547-551` names that very markup, so a lone preserved segment break raises `RenderedText` on a line with no glyph — and the same segment already passes `:575` today and would already claim the line's baseline if its font resolved. The rung inherits the engine's existing notion of *a segment that gives the line a baseline*; it introduces no divergence of its own. No second flag: the text arm sets `first_baseline` at pack time (`:586`, under the guard at `:575`; ⚠ an earlier drafting cited `:575` for the *set* — round-24 gate), so `is_none()` at flush already answers "did a rendered-text segment land here or earlier". Traced against all three orderings. `query`+`font_metrics` rather than `measure_text`, because a glyphless box has no string to shape and the metrics are string-independent anyway (`elidex-shaping/src/measurement.rs:55`). ⚠ **Known residual, and it is the one divergence this program *creates*** — today no tentative baseline exists, so the corner cannot occur: if a line's text has no usable font, `measure_text` returns `None`, `first_baseline` stays `None`, and a co-resident box's tentative promotes on a line that does have glyphs. ⚠ It falsifies this row's own "No second flag" ground — `first_baseline.is_none()` does **not** answer "did a rendered-text segment land here" when no font is usable. **Fixed in PR-1d, not deferred** — and ⚠ **an earlier revision folded it into `#11-inline-root-inline-box` on a ground that does not hold**: css-inline-3 §5.3's "only glyphs from fallback fonts" needs **per-glyph provenance** (glyphs are present; which font produced them), while this residual needs only **per-line rendered-text presence** — ⚠ stated as "per-line glyph presence" until rev 34, which named the wrong predicate on this side of the contrast while the fold argument turns on the *other* side (round 26, Axis 2); the argument survives the correction intact, because the fold's own ground was that slot's **font-fallback-provenance trigger disjunct** — struck in rev 33 — while this residual's signal is `contributes_content`, which is not a glyph fact at all. They share the word "font" and nothing else, and that mechanism mismatch is the whole of the argument. ⚠ **A trigger-disjunct clause stood beside it until R5 and is dropped**: it read that **none** of that slot's trigger disjuncts — line-height correctness, per-box layout bounds, a compat-survey hit — can fire for a baseline defect about whether a line carried rendered text. Codex R2's own widening falsifies it: disjunct 2 is "any work that gives inline boxes layout bounds of their own", facet (d) is baseline composition **on a mixed line**, and a mixed line is exactly this residual's scenario — so the disjunct can fire, and the fold's refutation has to rest on provenance alone. §8's parallel paragraph already says so in terms ("no longer holds on its face, because facet (d) is strut composition"), so the two sites now give one ground rather than two of which one is false. ⚠ **Codex R2's widening of that slot does not revive the fold**: not one of its five facets needs per-glyph provenance — composing the layout bounds of a box elidex can already see is glyphless is a line-composition problem, whereas the fallback-only condition asks *which font produced a glyph that is present*. ⚠ **That slot carried a third disjunct, font-fallback provenance, when this argument was first made, and it is struck in rev 33** (round 25, Axis 3): it existed only to make §8's *own* fold — of css-inline-3 §5.3's fallback-only strut condition — surface there, and §8 withdraws that fold to `#11-inline-fallback-font-strut` on this row's identical mechanism-mismatch ground. The disjunct's removal does not weaken the argument here: it was never one that could fire for rendered-text *presence* either. The signal it needs **already exists and is font-independent**: `contributes_content` (`pack/mod.rs:556`), which §5.1 M5 itself calls "a pure text predicate with no font dependence". `measure_text` returns `None` on font *resolution* failure — `db.query(…)?` and `db.font_metrics(…)?` at `crates/text/elidex-shaping/src/measurement.rs:54-55` (⚠ the signature is `:49-53`; an earlier drafting cited it as `:49-51`, which stops two lines short of the return type) — an availability outcome — not a provenance one. So PR-1d does **not** add a second flag beside the tentative: it adds **one more state to M3's `LineOccupancy` ordering**, raised by that ordering's single writer from the occupant `place_item` derives from its own `member`/`contributes_content` pair, and the promote reads the state (Decision). The "second flag" this row gave up on is the right *fact* in the wrong *encoding* — a bool would need an implicit `⇒ any_rendered_content` invariant across three write sites, the shape M3's row refuses for `on_line`/`content_on_line`, and it would have had no raise **condition** that means rendered text once M3 owns `:698` — `contributes_content` alone cannot supply one, since `place_item`'s atomic caller passes a literal `true` for it (`:658`). **The concession is the argument for fixing it**, and CLAUDE.md's *TODO 先送り禁止* points the same way. §6 cell 24 asserts the fixed behaviour, not an accepted divergence. |
| **M8** | Intrinsic sizing | **Both intrinsic passes account for the edges, and both edge terms are PR-1b's; min-content's cross-item *joining* is a prerequisite PR ahead of PR-1b rather than a deferral** (ledger **A47**, **A58**; §8's *Min-content prereq PR*). Each pass calls the same `collect_inline_items` the layout pass does (`inline/measure.rs:22`, `:51`), so the edges it adds are read off PR-1b's marker payload — M4 (i)'s one derivation site — and never derived from style there. `max_content_inline_size` (`:44`): each marker adds its inline-axis edge sum to the running total, matching that pass's existing `total +=` shape. `min_content_inline_size` has **no running candidate and no cross-item joining** — it is `max_word = max_word.max(m.width)` per word per item (`:15-37`), so a box's edges have nothing to attach to, and the same absence already loses run boundaries (`a<b>b</b>c` yields `max(\|a\|,\|b\|,\|c\|)` today, never `\|abc\|`); the joining is the prerequisite's, and PR-1b's edge term lands on the accumulator it builds. ⚠ **That was read here as a ground for deferring it and is not one**: it measures what the fix *costs*, not whether PR-1b is correct without it. | Deferring either intrinsic half past PR-1b ships an inconsistency between **layout and intrinsic sizing** on the non-cyclic edge terms (a cyclic percentage is rule 4's sanctioned exception, cell 25b), not between the two intrinsic sizes (an atomic inline's contribution is a separate, pre-existing disagreement, `#11-inline-atomic-intrinsic-contribution`). `shrink_to_fit_width` is `min(max_content, max(min_content, available))` (`elidex-layout/src/intrinsic/mod.rs:134`), fed by `intrinsic/block.rs:39`/`:49` and consumed at `elidex-layout/src/layout/mod.rs:57` for an `auto`-width inline-block: at a small available width it settles on the **word** width, while PR-1b's line needs word + edges, and the box overflows. css-sizing-3 §5.2's note that the spec "does not define precisely how to determine these sizes" licenses a choice of *procedure*; it does not license an intrinsic size the engine's own layout cannot fit into — save where the spec itself licenses one: a cyclic percentage edge, which css-sizing-3 §5.2.1 rule 4 resolves against zero for contributions while layout resolves it, the content then overflowing (cell 25b; ledger **A62**). |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`) against `containing_inline_size`. |
| **no cite-sweep layer — this program sweeps nothing pre-existing** | §3.1's classes are **pre-existing defects this program merely *found***, and a sweep of them is slot work, not umbrella work (`#11-inline-spec-cite-misattribution`, §9). The single exception is `pack/mod.rs:726`, which **PR-1b corrects because PR-1b changes what that comment documents** — it adds the css-text-3 §7.3 boundary break beside the surviving css-text-3 §5.5 intra-word coalescing. The rule: *a program fixes the citations its own change makes wrong, and hands pre-existing classes to whoever owns them.* Round 16 measured why a coordinate list cannot be that owner — the CSS 2 `§9.2.2.1` misattribution alone has hits at `pack/mod.rs:107`, `:200`, `:425`, `:553` and three test files, and "one border-box fragment per line" recurs at `boxes.rs:90-91` and four more sites. |
| `crates/layout/elidex-layout-block/src/inline/styled_run.rs` | **`InlineItem` (`:9`) — where M1's two marker variants are declared**, beside `Text`/`Atomic`/`Placeholder`. `StyledRun` (`:43`) itself is unchanged; M1 explicitly does not widen it (§5.3 Rejected). |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair, with the payload M1 specifies, using the `parent_style` already in scope. Gains **one** new parameter: `containing_inline_size` on `collect_inline_items` (`:136`) / `collect_inline_items_inner` (`:194`). `root_horizontal` (`:211`) is unchanged. Every `collect_inline_items` caller — enumerated by `grep -rn 'collect_inline_items(' crates/` minus the definition — takes the new argument: `inline/mod.rs:154` (has the value), `inline/measure.rs:22` and `:51` (the intrinsic passes, which pass `0.0` — css-sizing-3 §5.2.1 rule 4, decided at §3's row for it; cell 25b), and the test helper `inline/tests/mod.rs:17`. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get `PackItem` forms carrying `item_index` only (M1); they never become `FlowMember`s. |
| `crates/layout/elidex-layout-block/src/inline/pack/mod.rs` | `LinePacker` line state. **M3's `note_line_occupancy` lives here**, beside `place_item` (`:679`), its first caller — and it is the **single writer** of the line-occupancy ordering, M7's `RenderedText` rung included: the raise sites are `place_item`'s call and the marker path's — and, from **PR-1d**, `flush_line`'s open-box replay (M3's third caller, R7-a), which raises the just-reset line to `BoxEdgeOnly` like the marker path does, so the stage-complete count is **three** and §8's PR-1d item names all three (R9-a). `place_item` **derives** the occupant it passes from the two arguments it already has — `contributes_content` (`:686`) and `member: FlowMember<'_>` (`:687`) — raising the `RenderedText` rung only for `(FlowMember::Text(_), true)` and `Content` for every other pair, its own literal-`true` atomic call (`:652` → `:658`) included; the marker path passes `BoxEdgeOnly` and so raises no higher than the non-text state (M5, M7). ⚠ **Not "the Text arm's call is the only site whose value means glyph text"** — that is what this row said until the rev-33 gate, and `place_item`'s two callers (`:591`, `:652`) feed one parameter and one `note_line_occupancy` call, so a *call site* is not a condition the raise can test (M7's Decision carries the withdrawal). No other function writes the ordering. `flush_line` (`:209`) **calls** the open-box hook that `pack/inline_box.rs` owns (M4, **PR-1c**), promotes M7's tentative baseline inside its `:210` arm and **reads the occupancy state there** (**PR-1d**), and grows its per-line reset block by exactly **one** field per PR — M3's `line_occupancy` in **PR-1b** and M7's tentative in **PR-1d**; M7's `RenderedText` rung is a widening of M3's field, so the `:432-439` reset it already has covers it and no second reset is added. The fabricated shaping citation at `:726` is rewritten by **PR-1b** — the one cite this program corrects, because PR-1b changes what that comment documents (§9). |
| `crates/layout/elidex-layout-block/src/inline/mod.rs` | The IFC entry point. Sites this program writes, by PR: `collect_inline_items`'s call (`:154`, PR-1a, one argument); `items.is_empty()` (`:161`, held in PR-1a, flipped in PR-1d); the `any_font` closure's exhaustiveness arm (`:192-199`, PR-1a, behaviour-neutral); the **outer early-return condition** (`:200`, PR-1d, gains an escape for markers **that satisfy `has_inline_axis_edge`** (M5), beside the existing `Atomic` one); `assign_inline_layout_boxes`'s call (`:380` — ⚠ **whether this call site changes at all depends on the carrier PR-1c's memo picks** (M4): unchanged if the edges ride a widened `EntityBounds`, one added argument if they ride a second entity-keyed map **or** the `&[InlineItem]` slice (M4's three options). This memo does not decide it, so it does not promise the call is untouched either); and — ⚠ **not a write but a path-selection consequence** of the `:161`/`:200` flips (`154bac3f`; `:162`/`:202` at `22de3078`) — §7's `clear_inline_flows` gating (`:637` in the `154bac3f` frame; `inline/reconcile.rs:417-418` at `22de3078`, already `!env.is_probe`-gated: none of PR-1a–1d edits that file; the prereq #511 did, see the dead-arm row). The dead-arm prereq PR (#511, landed) wrote further sites here; they have their own row below. |
| the dead-arm prereq PR's surface | `flush_line`'s `else` arm and its unmerged rect loop (`pack/mod.rs:393-421`) **and** the `inline/mod.rs` half the reachability argument kills: `persist_candidate` (`:239`), `flow_align`'s `Option` construction (`:240-251`), `persist_flow`'s now-redundant conjunct (`:322`) and the comments that explain the two-path model (`:227-230`, `:309-320`, `:329-330`). Listed as a layer of its own because the PR spans two files, which no other row does, and because every one of its six `inline/mod.rs` items lies **above** seam 3 — the fact §8 uses to conclude the two **`elidex-layout-block`** prereqs are independent rather than ordered (the other three — the predicate, min-content and reconciler prereqs — are a different question: each hands its touch set to its own plan-review, so this memo concludes nothing about them). **Landed as #511 (`22de3078`, 2026-09-07).** ⚠ **What landed exceeds this enumeration** (recorded as the delta, per [[feedback_plan-ratified-surface-is-a-design-change]]): the pre-push gate found the same class one level down — with the pre-gate gone, `do_carrier` is definitionally `!persist_flow`, so `reconcile_flows`'s two-`bool` interface re-encoded the deleted third state and its `persist_flow \|\| do_carrier` guard was a tautology — and #511 collapsed it to **one bit**: `reconcile_flows` takes `persist_flow` alone, the caller derives `do_carrier = !persist_flow`, the guard and the redundant `do_carrier` conjuncts are gone. That edit lives in `inline/reconcile.rs`, the seam-3 module, so this row's "two files" is superseded — the landed set is whatever `git show --stat 22de3078` lists (no figure carried here), and the "above seam 3" independence argument held for the *planned* surface only. ⚠ Whether the landed delta disturbs a later obligation is **round 20's question, not this row's** ([[feedback_plan-ratified-surface-is-a-design-change]]: the collapse was applied at #511's pre-push gate and documented after — the class that rule exists to route back through plan-review). What this row can measure: the obligation sweep `grep -n 'do_carrier\|eleven' <memo>` returns landing-record sites only (this row and §8's ordering record — two lines) and no DoD; and the delta **fired the successor slot's disjunct 3** (its trigger exempts no prereq there — disposition in the slot memo and §10's last row). The successor slot's own half (`reconcile_flows`' signature, SoT) counts **ten** parameters and **two** adjacent `bool`s from `22de3078` on. PR-1a re-anchors against its actual base, which includes `22de3078`. |
| `crates/layout/elidex-layout-block/src/inline/whitespace.rs` | `collapse_inline_whitespace` (the `fn` is `:27`) — M2's transparent arm, which joins the per-item `match &mut items[i]` whose head is `:41`. ⚠ Both coordinates name the same function and different things; earlier revisions wrote `:41` here and `:27` in §3 with neither saying which (round 25, Axis 2). |
| `crates/layout/elidex-layout-block/src/inline/measure.rs` | `max_content_inline_size` (`:44`) and `min_content_inline_size`'s edge term — M8's contribution, both PR-1b's. `min_content_inline_size`'s **accumulator** — `max_word` at `:23`/`:31` — is **not this program's**: the min-content prereq PR gives **both passes** layout's segmentation and trimming — cross-item joining, the break oracle, and the trailing space `max_content_inline_size` keeps (§8; ledger **A64**, **A68**) — in `main` before PR-1b, and PR-1b re-anchors against that base. The function itself (`:15-37`) PR-1b does touch, because its `collect_inline_items` call (`:22`) takes the new argument like every other caller. |
| `crates/core/elidex-plugin/src/layout_types/boxes.rs` | `InlineClientRects` (`:106-111`) — the cross-crate contract type. **Semantics not changed by this program**: "per-line client rects … single-line inlines use `LayoutBox.border_box()`" stays true, and PR-1c makes the fallback half *correct* rather than redefining the component. ⚠ Its docstring nonetheless cites **"CSSOM View §5"** for `getClientRects()`; cssom-view-1 §5 is *Extensions to the Document Interface* and both `getClientRects()` and `getBoundingClientRect()` are on `Element`, i.e. cssom-view-1 §6. §3.1 defines that class **by a concept grep** and `#11-inline-spec-cite-misattribution` owns it (§9) — this row routes the docstring, it does not vouch for it. The multi-fragment redefinition belongs to `#11-inline-box-decoration-splits`, which owns the docstring edit too. |
| `crates/core/elidex-render/src/builder/slice.rs` + `walk.rs:296` (render) and `crates/layout/elidex-layout-block/src/block/mod.rs:369-372` (layout) | **Existing `box-decoration-break` implementations, and why this program does not extend them.** `walk.rs:296` reads `style.box_decoration_break`; `slice.rs`'s `break_edges` (`:22`) computes per-fragment slice geometry for **column** fragments and takes `(i, n, wm)` with **no `direction`**, its own docstring saying "the inline-axis edges are never 'at a break'"; `block/mod.rs:369-372` handles `Slice`/`Cloned` for **block** fragments off `block_start_pb`/`block_end_pb`. All three are block-axis, while a line break is an **inline-axis** break whose **broken** edge — css-break-3 §5.4's own term, and the one it defines: "which side of a fragment is considered the broken edge is determined by the parent element's inline progression direction" (⚠ this row wrote "surviving edge" until R5, a term the section does not use and whose sense is the complement of the one it defines) — is set by the *parent's* inline progression direction — a different axis and a different direction source, so none generalises as written. ⚠ Two crates, and the dependency runs **render → layout** (`elidex-render/Cargo.toml` depends on `elidex-layout-block`, not the reverse), so a layout-side producer cannot call `break_edges`, which is `pub(super)` in `elidex-render::builder`. `#11-inline-box-decoration-splits` owns the inline case and must first decide **which layer produces** the attribution, since it has a CSSOM consumer and a **prospective** paint consumer — prospective because a static inline's chrome is not emitted at all until `#11-inline-decoration-paint-path` lands (R4; the `elidex-render` row below carries the measurements). |
| `crates/core/elidex-plugin/src/logical.rs` | `LogicalEdges::from_physical` (`:186`) + `WritingModeContext::new` (`:27`) — used at the *point of derivation* (M1), never as a round trip. |
| `crates/layout/elidex-layout-block/src/inline/pack/inline_box.rs` (NEW) | **The marker's own derived facts and the stack that holds them**: push on `InlineBoxStart`, pop-and-emit on `InlineBoxEnd`, the flush-time hook `flush_line` calls (emit + rebase, M4), and M5's `has_inline_axis_edge` plus the inline-start/inline-end sums M3 and M4 consume — one derivation site for everything read off a marker. ⚠ **Two shapes in one module, deliberately**: the stack and its `flush_line` hook are an `impl LinePacker` in a sibling module (the idiom `pack/fragment.rs:10` already uses), while `has_inline_axis_edge` and the sums are **free functions over the marker payload**, because the pre-pack gate at `inline/mod.rs:200` calls them before any `LinePacker` exists (M5). ⚠ **The file is created by PR-1b and grown by PR-1c**: PR-1b authors the free functions (M5's predicate and M3's sums), PR-1c adds the stack and the hook. §9's 700–800 band therefore applies to it across both PRs, not at one of them. The line-state core (M3) and the baseline promotion (M7) stay with their existing owner in `pack/mod.rs`: cohesion, not `pack/mod.rs`'s line count, decides the split. |
| `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs` | `assign_inline_layout_boxes` (`:48`) — writes `LayoutBox.content` from bounds and, per M4, the three edge fields, **with no `ComputedStyle` fetch** (`:57` stays the `is_err()` guard it is today). ⚠ **Which structure carries the edges to this function — a widened `EntityBounds` (`:30-41`), a second entity-keyed map, or the `&[InlineItem]` slice — is PR-1c's plan-memo's decision, not this memo's** (M4 states the four invariants it must satisfy and the two measured facts that constrain it); this row therefore names the *write target* and not the carrier, and does not promise the signature is unchanged. The `InlineClientRects` write (`:102-126`) is untouched either way: both keep content spans, and the per-fragment border-area inflation is `#11-inline-box-decoration-splits`'s (M4). |
| `crates/layout/elidex-layout-block/src/inline/tests/mod.rs` | The test harness. `collect_styled_runs` (the whole `fn`, `:11-25`) is a `collect_inline_items` caller — the call itself is `:17`, the coordinate the `collect.rs` row above cites — whose `filter_map`'s `match item` (`:20-23`) gains marker arms in PR-1a — but it `filter_map`s to `Vec<StyledRun>`, so those arms are `None` and it **cannot observe a marker**; PR-1a adds a sibling returning the `InlineItem`s (cell 6b). Also the `mod decorated_inline;` declaration and **PR-1a's `setup_inline_test` (`:54`) change** giving the harness a deterministic way to force `any_font == false` (cell 12d). |
| `inline/tests/decorated_inline/{stream,advance,geometry,existence}.rs` (NEW) | The four per-PR test modules §6 routes cells to. |
| `elidex-render` tests — **a negative row from R4, and since rev 66 without exception: no cell of this program lands here** (13d was that exception until R32 moved it to `elidex-shell`, the only host that runs the producer and reads the display list; ledger **A73**) | The crate *could* host one (it depends on `elidex-layout-block`, `crates/core/elidex-render/Cargo.toml`, so a cell there can run layout). There is nothing to assert: **a static inline box's background and border are never emitted**. Three measurements at `22de3078`. (i) `paint_non_sc` (`builder/walk.rs:638`) pushes a non-positioned **inline** child into `inline_run` and passes only a **block** child to `walk` (`:648-676`), so a static inline never reaches the painting function. (ii) `emit_background` / `emit_borders` have exactly two non-test callers, `walk.rs:356` and `:366` — `grep -rn "emit_background(\|emit_borders(" crates/core/elidex-render/src/ \| grep -v "fn emit_"`. (iii) `InlineFlowRun` (`crates/core/elidex-ecs/src/components/inline_flow.rs:131`) has exactly two variants, `Text` and `AtomicBox`, and `builder/inline_flow.rs` consumes only those — no member carries an inline box's chrome. Corroborated end-to-end on the **post-PR-1c shape** (a `<span>` given `background: red` and a `LayoutBox` with real 10px padding under a block `<div>`, display list built by `build_display_list`): red rects **0** for `display:inline`, **1** for `display:block`, **1** for `position:relative` — so the uncovered population is exactly the *static* inline, and it is `#11-inline-decoration-paint-path`'s (§5.3), not this program's. ⚠⚠ **That corroboration is a *count*, and R11 measured what it could not see**: on the `position:relative` arm the **rect moves** — and the count only *looks* invariant because this corroboration used a background-only span: give the same span a `border`, and it goes 1 → **5**, `emit_borders` taking each side's thickness from `LayoutBox.border`. Layer 6/7 reaches such a box through `walk` (`:614-629`, `:728`) and `emit_background` / `emit_borders` read `lb.border_box()` (`builder/paint/mod.rs:68`, `:382`) — the box M4 puts real `EdgeSizes` on at PR-1c (this table's `pack/boxes.rs` row). Measured on this crate's own `consumes_relpos_inline_subflow_with_gap` harness (`crates/core/elidex-render/src/builder/tests/inline_flow/relpos.rs`): `SolidRect (24.0, 0.0) 16.0 x 20.0` today, `SolidRect (12.0, -12.0) 40.0 x 44.0` with padding 10 and border 2; re-measured with the style's `border` decoupled from the `LayoutBox`'s, a `border: 2px solid` span gives **1** rect today (background only — the border paints nothing) against **5** after. So the row is negative for the **static** arm only, cell **13d** lands here to assert the positioned one, and A6 is not crossed — 13d reads this crate's existing output and adds no pass or member (ledger **A8** narrowed, **A33**). What the crate **also** sees is PR-1b moving the `Text` runs' `inline_start`; §7 and §8's PR-1b DoD dispose its existing `border_box()`-reading tests. |
| `elidex-layout-block/src/inline/tests/` — **not `elidex-dom-api`** | Cells 17b/17c/17d. ⚠ `elidex-dom-api` has **no** dependency on any layout crate and no `[dev-dependencies]` at all, so a cell there cannot run inline layout — its existing `layout_query.rs` tests hand-insert `LayoutBox` literals, which would assert the marshalling and nothing about M4's producer. Every existing `InlineClientRects` assertion already lives under `elidex-layout-block/src/inline/tests/inline_flow/`, and that is where M4's two channels are jointly observable. §8's PR-1c `getBoundingClientRect` obligation is discharged the same way — against the `LayoutBox` border box the DOM API reads — not by a cell in `elidex-dom-api`. |
| the seam-3 module — `inline/reconcile.rs`, created by #508 (`7e256029`) | `layout_inline_context_fragmented`'s reconcile block, moved out of `inline/mod.rs`. §7's `clear_inline_flows` gating lives here (`:417-418` at `22de3078`, already `!env.is_probe`-gated) — a **path-selection consequence** of PR-1d's flips in `inline/mod.rs`, **not an edit: none of PR-1a–1d writes this file** (`awk '/^## §6\./,/^## §7\./' <memo> \| grep -c reconcile` → 0; §8's PR-1d DoD → 0; the prereq #511 *did*, this table's dead-arm row), so `:637` is a pre-move coordinate and the successor slot's disjunct 3 is fired by **no PR after #511** (§10's last row; earlier draftings of this row said "PR-1d fires it" and then "no umbrella PR fired it" — #511 did). |
| `elidex-shell` tests — **the only site where a layout producer and a DOM-API reader are jointly observable, and since rev 66 the only site where a layout producer and the *display list* are** | §8's PR-1c end-to-end clause, and §6 cell **13d** (ledger **A73**): its suite drives HTML + CSS to a display list (`src/tests.rs:41`) and already matches `DisplayItem::SolidRect` (`:127`). Ground: `elidex-shell` depends on **both** `elidex-dom-api` and `elidex-layout` (→ `elidex-layout-block`), so it reaches the producer and the reader at one hop — the reachability an earlier revision denied by measuring adjacency instead. `build_pipeline_interactive` (`crates/shell/elidex-shell/src/pipeline.rs:563`) returns a `PipelineResult` carrying **`dom: EcsDom`** (`lib.rs:199-204`), so the test reads the span's real `LayoutBox` from the post-layout world **and** exercises the **four** `client*` members (R7-c widened §8's clause from `clientTop` alone) — either by invoking the registered handlers (`clientTop.get`, `crates/dom/elidex-dom-api/src/registry.rs:168`) or through a `<script>`, which that suite already does (`src/tests.rs:52` mutates the DOM from JS). ⚠ This row exists because §8 took on an obligation no other §5.2 row covers; the memo's other cross-crate rows (the `elidex-render` negative row, and the `— **not** elidex-dom-api` one) answer narrower questions. |
| `elidex-text` (facade over `elidex-shaping`) | `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60`) + `font_metrics` (`:101`) — M7's strut A/D, taken without shaping a string. |

### §5.3 Program slicing

Four shipping PRs, each **behaviour-scoped**, with one owning PR per coupling (§2), plus the
booked slots below — no count here, since the list grows as rounds find them (rev 33 added one).
Each PR gets its own plan-memo and `/elidex-plan-review`.

* **PR-1a — item stream. Behaviour-neutral.** Owns coupling 1×2 (both its rows): M1 + M2. Marker
  variants, all consumers updated — **the same set §8's PR-1a DoD enumerates**, stated there once
  rather than in two lists — `containing_inline_size` threaded — **no citation work at all** (§9). The packer gets a **no-op** `match pi` arm (`:528`). The two pre-pack gates are held at
  current behaviour: `items.is_empty()` (`inline/mod.rs:161`) excludes markers, and the `any_font`
  closure gains its **exhaustiveness arm** at `:192-199` returning `false` — behaviour-neutral,
  since a marker is neither `Text` nor `Atomic`. The **outer condition** at `:200` is untouched
  until PR-1d. **Characterization tests for §6 cells 1, 2, 5, 6b, 6c, 6f, 6d, 6e, 6i, 6g, 6h, 7–12 and 12d land here
  asserting today's behaviour** — the cells that pin *line suppression*, which is observable with
  `entity`-only markers. ⚠ **Cells 6b, 6i, 6g and 6h are the exceptions and are not characterization
  cells**: the marker variants do not exist on `154bac3f`, so all four pin *new* item-stream behaviour
  (6g and 6h the pseudo-element routing M1 records, non-empty and empty `content`; 6i that the
  predicate reads the box's own style and not its subtree). PR-1a stays
  behaviour-neutral in the sense that matters — no layout output moves — but the item stream is
  deliberately not neutral, and 6b, 6i, 6g and 6h are what assert that, since `grep -rn padding
  crates/layout/elidex-layout-block/src/inline/tests/` → no hits means the existing suite pins
  nothing about decorated inlines. Cell 12d's harness change lands here too, with the cell it
  makes constructible. PR-1a opens 1.
* **PR-1b — inline-axis advance.** Owns couplings 7×2, 7×(intrinsic), 1×7, 4×7 and 5×7: M3 + M8, **plus M5's
  predicate** (`has_inline_axis_edge`), which ships here because M3's shaping break reads it
  — **per marker side**, the derivation site producing the two halves of which the
  whole-box name is the disjunction (M5); PR-1d owns only its *substitution* for the constant.
  The shared line-occupancy core with markers passing `contributes_content = false`,
  inline-axis advance, shaping break at a decorated boundary, a marker not ending a trailing
  space's line-finality (ledger **A63**), and the
  max-content contribution. §6 cells 3, 3b, 3c, 4, 12b, 12c, 12f, 12e, 14, 14b, 15, 15b, 15d, 16,
  16b, 25, 25b, 25c and 25d land here — cells 3, 3b, 3c, 4, 12b, 12c, 12f and 12e among them because each asserts a
  *payload* fact
  (negative margin, the cancelling pair, the percentage basis, the physical→logical side mapping)
  that M1's PR-1a variants, carrying `entity` only, give no channel to observe, and that the advance
  makes observable as a cursor position.
  ⚠ **What this PR does not touch is the box's own geometry.** `assign_inline_layout_boxes` still
  hard-codes `EdgeSizes::default()` (`inline/pack/boxes.rs:82-84`), so `border_box()` is still equal
  to `content` and **no `border_box()` reader changes extent** — the box stays as unedged as it is
  on `154bac3f`, now correctly *positioned*. That is the property that makes this PR shippable
  ahead of PR-1c rather than after it; PR-1c's bullet states the asymmetry. (The property is
  about **extent**; a paint-side phrasing of it is vacuous for a static inline — ledger A8.)
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
  ⚠ **The universal is over *cells*, not over markup** (round 26, Axis 3, Gate A; ledger A17):
  the advance makes text reach the guard at a **genuine UAX #14 opportunity** earlier than it
  did, so markup can gain a line legitimately, and the slot owns only the artificial boundary
  path. The two shapes stay lineless in PR-1b for **two different reasons**:
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

  PR-1b opens none. ⚠ **It opened 1 until rev 53** — `#11-inline-min-content-box-edges`, the
  min-content half of M8 — and that slot is **withdrawn, not re-tagged**: its edge term is PR-1b's
  own and the cross-item joining it lands on is a prerequisite *before* PR-1b (§8's *Min-content
  prereq PR*; ledger **A47**, **A58**), so the gap is never open and there is nothing to register. **No close is owed either**: the slot was never in
  the SoT — `project_open-defer-slots.md`'s 2026-08-16 block lists it among the umbrella's names
  that "are not slots yet" — so the withdrawal costs §10 a row and the SoT nothing. Whatever the
  prerequisite itself defers is its own memo's call, like every other prereq here.
* **PR-1c — box geometry.** Owns couplings 3×7 and 2×3: M4. The open-box stack and its flush-time
  emit-and-rebase hook, the marker's content-span rect, and the three real `EdgeSizes` on the
  inline's `LayoutBox`. **This is where the box's own geometry becomes real**, for every decorated
  inline with content — the largest delta in the program. ⚠ **It is not where a *static* inline's
  painted output changes, and R4 corrects the claim that it was**: a static inline never reaches
  `walk`, so its
  background and border are not emitted before or after PR-1c (§5.2's `elidex-render` row carries
  the three measurements and the probe; `#11-inline-decoration-paint-path`). ⚠⚠ **For a
  `position:relative` inline it *is*, and R11 corrects R4's correction** — that box does reach
  `walk` through Layer 6/7, so its painted background and border rect inflates by the real edges.
  On a *background-only* span the count is unchanged and only the geometry moves — which is why a
  count-based measurement missed it — and on a span that also carries a `border` the count moves
  too, 1 → **5**, because `emit_borders` takes each side's thickness from `LayoutBox.border` and an
  inline's is zero until M4 fills it. Cell **13d** asserts it; ledger **A8** (narrowed) and **A33**. The rest of the delta is
  user-visible through the readers that take `LayoutBox.border_box()` per entity rather than
  through the walker — `getBoundingClientRect` / `offsetLeft` (`elidex-dom-api`
  `element/layout_query.rs`), **hit-testing** (`elidex-layout/src/hit_test.rs:130-131` acquires the box, `:165` tests
  `bb.contains`, for any entity with a `LayoutBox` and no display filter beyond `Display::None`) and
  **a11y node bounds** (`elidex-a11y/src/tree.rs:121-125`, an unconditional `border_box()` read).
  Measured for hit-testing on PR-1c's own shape: with the span's 10px padding real, the point
  (12, 20) inside the padding ring hits the **span**; with the padding zeroed, as today, the same
  point hits the `<p>`. §6 cells 6, 10b, 10c, 10d, 13, 13b, 13c, 13d, 14c, 15c, 17, 17b, 17c, 17d,
  17f and 17g land here.
  ⚠ **The predicate prereq PR (§9) is already in `main` by this point** — it lands before PR-1a —
  which is what keeps PR-1c honest: PR-1c makes an inline's `LayoutBox.border` real, and
  cssom-view-1 §6 step 1 requires **all four** `client*` members to stay **zero** for an inline
  box — the step is identical across them (R7-c) — so landing PR-1c into a tree without the guard
  would ship a **new** violation at `clientTop`/`clientLeft` and **widen** the pre-existing one at
  `clientWidth`/`clientHeight`, whose `get_padding_box` source M4's padding makes real too. §3's CSSOM row and §9's
  bullet carry the measurement.
  ⚠ **Why it follows PR-1b, and why the reverse order is not merely less tidy but wrong.** Every
  reader in the family above takes `border_box() = content + padding + border`. Land M4 *first*
  and an inline's real edges are applied to a content span the advance has not yet moved, so the
  reported box **overlaps the preceding content** — on cell 13's own markup,
  `<p>a<span style="padding:10px">text</span>b</p>`, the span's `getBoundingClientRect` and its
  hit area cover the tail of `a`, and `a` is still hit-tested and painted there. That is a defect
  the engine does not have today. The order taken introduces none: after PR-1b the edges are still
  zero, so `border_box()` is still `content` and every one of those readers answers exactly what it
  answers now. Monotone in one direction, a new wrongness in the other — which is the same test
  §5.3 applies to putting geometry before the existence flip. ⚠ **The argument used to be made on
  paint** ("the span's background covers the tail of `a`"), which R4 refutes: there is no such
  background. It survives on the readers that do exist, and those are the ones §7 disposes.
  ⚠ Presence, not only value: M4 adds a second producer of `current_line_entity_rects`, so a
  decorated inline that owns no run gains its **first** `LayoutBox` — but only on a line that
  already exists. The rest of the presence change is PR-1d's; §7 states both stages.
  PR-1c opens 1 (`#11-inline-box-decoration-splits`).
* **PR-1d — existence.** Owns couplings 1×3, 1×5, 1×4, 2×4, 2×5, 3×4 and 3×5: M5's *flip* + M6 + M7. The clause-3
  predicate per §1.2, the strut baseline, the commit consequence, and the two pre-pack gates
  flipped. **The delta to PR-1b's marker call is `contributes_content` (M5's expression replacing
  the constant `false`) plus `block_advance` (M6);** everything else is already in place. §6
  cells 18, 19, 20, 21, 22, 23, 23b, 24, 24b, 24c, 24d and 24e land here, plus the flip set §8 states.
  ⚠ It also grants presence a second time and more widely than PR-1c did: every entity on a
  line that was phantom and now commits gains its first `LayoutBox` — the decorated inline
  itself in Shapes A and B, and every co-resident (cell 21). Closes
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
  same missing attribution seen from the other side. ⚠ It is first-layout-only until
  `#11-inline-relayout-box-staleness` lands (§8, §9) — an ordering note, **not** a blocking
  dependency. ⚠ Its stated ground, "that limit applies equally to PR-1c's own edge write", no
  longer holds: the box repair now lands in a **prerequisite PR** ahead of PR-1c
  (R12 moved it out of the slot, R13 measured the slot's own remedy inert, R15 carved it out as
  the two-sided extension of the engine's single staleness reconciler), so PR-1c's edges *are*
  refreshed. What survives is the narrower ground — this residue is a `#11-inline-box-decoration-splits`
  facet whose per-fragment attribution does not exist at all yet, so there is nothing for a
  refresh to keep current. Also fragmentation across the marker pair;
  **css-text-3 §5.5's adjacent-soft-wrap rule** (a break next to a decorated boundary lands at the
  box's *margin edge*, which M3's unconditional start-edge advance does not honour — §3's css-text-3 §5.5 row
  whose Step cell reads "adjacent soft wrap opportunity" and §6's note after cell 15 route here,
  and §6 cell 17c pins elidex's line-N+1 start as the accepted divergence this slot flips);
  and the `group_key` / relpos sub-flow keying that §2's pair 2×6 and §3's CSS 2 §9.4.3 row defer
  here, the field's only reader being the split rule. Why deferred: box continuity across line,
  bidi and sub-flow boundaries is a distinct invariant axis, reachable only once a box has
  geometry; folding it in would put a second axis into PR-1c, which after the re-slice owns
  exactly one coupling. Trigger: **PR-1d landing, *or* terminal-Z C-3b pinning the `getClientRects` dispatch** — a
  disjunction, matching the sibling #488 slots in the same SoT, which use `or` precisely so an
  unscheduled slice cannot freeze a slot. C-3b co-owns the multi-fragment question (§9) but has no
  PR and no date, so a conjunct would do exactly that. Re-eval: 2026-11-01.
* **`#11-inline-root-inline-box`** (new slot, **pre-existing** class): ⚠ **css-inline-3 §5.3's
  layout-bounds model as *composition*** — how each box's own bounds enter its line's height and
  baseline — widened here from "the block container's root inline box" by Codex
  R2 (F1/F2/F4), which is a correction to the slot's *stated scope*, not new work: every facet
  below was already disclosed somewhere in this memo while the slot claimed only the first.
  ⚠ **The scope read "the whole css-inline-3 §5.3 layout-bounds model" until R22, and under that
  phrasing this slot and `#11-inline-height-normal-layout-bounds` shared a subject and a trigger**
  (Codex R22): §5.3's `normal` branch is how a *single* box's bounds are computed from its glyphs,
  which is that slot's subject, so either could have closed leaving the other half undone. The
  scope stated here is the one §8's fold-refusal bullet already used for this slot — "how a box's
  own metrics compose into its line" — and the line between the two is the same **per-glyph
  provenance** one that split `#11-inline-fallback-font-strut` out at round 25: none of the five
  facets below takes it, and the `normal` branch is defined on it.
  `body css-inline-3 inline-height`: each inline box's contribution to its line's logical height is
  "always calculated with respect to its **own** text metrics", from its own first-available-font
  metrics and used `line-height` (with a strut when it "contains no glyphs at all"), and
  css-inline-3 §2.2 step 3 sizes the line box "to exactly include the aligned layout bounds of all
  its inline-level boxes". ⚠ Edge inflation is **not** part of the default: §2.2 step 2 and css-inline-3 §5.3
  both condition it on `line-fit-edge` ≠ its initial `leading`, which §3's own row already records.
  elidex has none of the model: a scalar `current_line_height = max(block_advance)`
  (`inline/pack/mod.rs:696`) and a first-wins `first_baseline` (`:586`). §2's coupled-invariant
  **row 5** is this slot's invariant. **Five facets, each pinned by a §6 cell rather than fixed
  here**:
  * **(a) the root inline box** (css-inline-3 §2), the line box's height floor — M6, cell 23.
  * **(b) the committed rect's block extent**: `commit_aligned_entity_rects` writes
    `block_size: line_height` — the *line's* height — for every merged entity
    (`inline/pack/mod.rs:493`), where css-inline-3 §5.3 composes the line *from* each box's bounds
    and never
    assigns the line's extent back — M4's channel, cell 13b.
  * **(c) the vertical-mode block-advance source**, `font_size` rather than `line_height`
    (`:539-543`), so an empty box's `line-height` does not influence a vertical line at all — M6,
    cell 23b.
  * **(d) baseline composition on a mixed line**: a glyphless box's strut is suppressed rather
    than composed with the line's other boxes — M7, cell 24c.
  * **(e) composition on a *decoration-only* line carrying more than one box** (R4; **M6 and
    M7**, not M7 alone — ⚠ the owner tag read "M7" until R5 while the bullet's own body invokes
    M6's scalar `max` and cell 24d is headed "(M7/M6)" and asserts the height as well as the
    baseline; M6's Decision says "two facets **on this row**", which is a scope statement about
    that row and not a count of the slot's facets, and its Grounds now says so): css-text-3
    §5.5 gives an inline box boundary no soft wrap opportunity, so two glyphless decorated boxes
    share a line and css-inline-3 §5.3 gives each a strut of its own; M6's scalar `max(block_advance)` and M7's
    single tentative `Option<f32>` between them carry one box's line-height and one box's ascent,
    which is not the aligned composition of two — nor of the **three** boxes css-inline-3 §2.2
    step 2 lists, the `<p>`'s root inline box included. M7's Grounds carries the argument; cell
    24d pins it.
  Why deferred: it changes the height of *every* line box in the engine — verified pre-existing
  because each facet is observable today with no marker involved ((a) M6's example; (b) an
  undecorated `<span>` whose `line-height` differs from its line's, since the write is reached
  through `place_item`'s `entity != parent_entity` push at `:706`; (c) the same span in
  `vertical-rl`, the convention being the *text* path's; (d) and (e) elidex composes no struts at
  all, and holds one scalar line height and one first-wins baseline however many boxes a line
  carries), so `origin/main` already fails all five. Trigger: any line-height correctness work, **any work that
  composes per-box layout bounds into a line box's height or baseline** (the five facets are
  instances of that composition's absence), `#11-inline-height-normal-layout-bounds`'s discharge
  (which settles what a box's bounds *are* under `normal` — the input this slot composes; the
  adjacent-slot form the fontless-text slot two bullets above already uses),
  or a compat-survey hit. Re-eval: 2026-11-01. ⚠ **The second disjunct read "any work that gives
  inline boxes layout bounds of their own" until R22** — that is the bounds *computation*, which
  is the adjacent slot's subject and not this one's, so the two slots stood on one trigger; it is
  narrowed to the composition, and the adjacency is carried as its own disjunct rather than
  dropped, so neither slot closes silently on the other's half. ⚠ **A further disjunct — "any work needing
  font-fallback provenance" — is struck** (round 25, Axis 3): it existed only so that §8's fold of
  css-inline-3 §5.3's "only glyphs from fallback fonts" strut condition would surface here, and
  that fold is withdrawn (§8; the condition has its own slot, the next bullet). It is not
  independently true of *this* slot's subject either, and the widening does not make it so: none of
  the five facets needs **per-glyph** provenance — composing the bounds of a box elidex can already
  see is glyphless is a line-composition problem, while the fallback-only condition asks which font
  produced a glyph that is *present*.
* **`#11-inline-decoration-paint-path`** (new slot, **pre-existing** class; opened by R4): **a
  static inline box's `background` and `border` are never emitted at all.** Three measurements at
  `22de3078`, none of them this program's doing. (i) `paint_non_sc`
  (`crates/core/elidex-render/src/builder/walk.rs:638`) pushes a **non-positioned inline** child
  into `inline_run` and passes only a **block** child to `walk` (`:648-676`), so a static inline
  never reaches the function that paints chrome. (ii) `emit_background` / `emit_borders` have
  exactly two non-test callers, `walk.rs:356` and `:366`
  (`grep -rn "emit_background(\|emit_borders(" crates/core/elidex-render/src/ | grep -v "fn emit_"`).
  (iii) `InlineFlowRun` (`crates/core/elidex-ecs/src/components/inline_flow.rs:131`) has two
  variants, `Text` and `AtomicBox`, and `builder/inline_flow.rs` consumes only those — no member
  carries an inline box's chrome; the inline **pseudo** path is the same (`builder/inline.rs:205-213`
  emits a `StyledTextSegment` and `continue`s). Corroborated on PR-1c's own output shape: a
  `<span>` with `background: red` and real 10px padding on its `LayoutBox` yields **0** red
  display items under `display:inline`, **1** under `display:block` and **1** under
  `position:relative` — the positioned inline reaches `walk` through
  `walk_child_with_fixed_check` (`walk.rs:728`), which is why the slot's subject is the **static**
  case. Why deferred: it is an `elidex-render` walker change — inline chrome needs per-fragment
  paint geometry the walker has no unit for, and css-break-3 §5.4's broken-edge rule decides which
  edges a line-split fragment draws — so it belongs to the render layer and, for the split half, to
  `#11-inline-box-decoration-splits`, which is the other consumer of the same attribution (§5.2).
  Bolting a render pass onto a layout PR crosses the crate boundary CLAUDE.md's Layering mandate
  keeps. **Pre-existing** on the memo's own test: `origin/main` paints no static inline's
  background today, with no marker involved, so it already fails it; **not** an own deferral, so it
  does not enter §5.3's per-PR count. What this program contributes is the *input* the render pass
  would need — PR-1c makes the box's `border_box()` real, which is the rect such a pass would
  draw. Trigger: **PR-1c landing** (the geometry the pass consumes exists from then), any
  `elidex-render` inline-painting work, `#11-inline-box-decoration-splits` picking up the paint
  consumer, or a compat-survey hit. Re-eval: 2026-11-01.
* **`#11-inline-fontless-measurability-gate`** (new slot, **pre-existing** class; opened by R4):
  `inline/mod.rs:200`'s early return gives `line_count: 0` to **any** IFC whose text has no usable
  font, and css-inline-3 §2.3's clause 1 (`contributes_content`, `pack/mod.rs:556`) is a
  font-**independent** text predicate, so the gate is an engine measurability guard standing where
  a clause-1 test would go (M5 says so; §6 cell 12d records the divergence). PR-1d lifts it only
  behind `has_inline_axis_edge`, so a **block-axis-only** decorated inline in a fontless IFC —
  `<p><span style="padding-top:10px;font-family:no-such-family">x</span></p>` — keeps returning 0.
  **Pre-existing, measured rather than argued**: at `22de3078` the gate returns `line_count` **0**
  for that markup, **0** with an inline-axis edge and **0** with no edge at all, so it is blind to
  edges and the program's partial lift makes no case worse than today — it moves the inline-axis
  arm to 1 and leaves the rest. Why deferred: the widening is a *measurability* question, not a
  decoration one — the `:200` return also runs `clear_inline_flows` and
  `remove_one::<ColumnFlowSlice>`, so lifting it commits an IFC in which `measure_text` answers
  `None` for every run, and deciding what such a line's geometry is belongs with whoever gives the
  engine a fontless-text fallback. **Not** an own deferral (the divergence is observable today with
  no marker involved) and it does not enter §5.3's per-PR count. §6 cell 12d is the pin, on both
  sides: today's 0 and PR-1d's surviving 0. Trigger: any fontless-text or font-fallback correctness
  work, `#11-inline-fallback-font-strut` (the adjacent font-availability surface), or a
  compat-survey hit. Re-eval: 2026-11-01.
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
* **`#11-inline-open-box-strut-on-continuation-line`** (new slot, **pre-existing** class, opened
  by R10): css-inline-3 §5.3 gives the strut to the **box**, so a box open across a break offers
  it to every line it participates in; M7 records the tentative where `InlineBoxStart` is consumed
  and `flush_line`'s reset clears it, so a continuation line has none and the promote cannot fire
  there. R7-a had M4's flush hook re-record it; R10 withdraws that half and books it here.
  **Why deferred: nothing can read the value.** Every line after the first is opened at an offset
  `find_break_opportunities` returned for some run's collapsed text, and (i) every such offset is
  **strictly inside** the text — the mandatory break `unicode-linebreak` emits at `text.len()` is
  filtered out (`crates/text/elidex-linebreak/src/lib.rs:28-33`) — so (ii) the continuation line's
  first member is that run's **non-empty tail segment**, `build_pack_items` slicing `text[k..]`
  with `break_after: None` (`inline/pack/items.rs`), and (iii) that tail always satisfies
  `contributes_content` (`pack/mod.rs:556-568`): under `pre`/`pre-wrap` the predicate is
  `!text.is_empty()`, and under `normal`/`nowrap`/`pre-line` a tail that trims to empty would have
  to **begin** with a collapsible space, and no licensed offset in collapsed text is followed by
  one: the css-text-3 §4.1.1 collapse leaves at most a single space (and no tab) in those three
  values, and the opportunity a space run licenses falls **after** it rather than inside it. So
  the continuation line always reaches the `RenderedText` rung M7's promote vetoes on,
  and the re-recorded tentative would be a value with no reader — CLAUDE.md's dead-code rule, and
  the reason this is a withdrawal rather than a TODO. ⚠ **(iii) is a predicate, so it was
  measured against the engine rather than read**: sweeping raw text over `{x, -, SP, TAB, LF,
  NBSP, ZWSP}` up to length 4 × all five `white-space` values, through `collect_inline_items` and
  then `layout_inline_context`, every licensed offset of every collapsed run text — mandatory and
  allowed alike — had a contributing tail, and every case carrying an interior mandatory break
  produced a **committed** second line; the control is that the predicate above and the engine's
  own `line_count` agreed on every one of the forced cases. ⚠ **The universal is carried by
  (i)–(iii) and not by the sweep**, whose alphabet and length are bounds: raw strings are *not* a
  safe superset here — over them the all-space tail does occur (`" \t"`, `"\u{200B}\t"`) and it is
  the collapse that removes it, which is why the sweep is run through the collector and not over
  literals. ⚠ **(i)–(iii) are scoped to *licensed* offsets, and the route outside that scope is
  one no cell may use**: the wrap guard fires inside `place_item`, so a line can also be opened at
  an **item boundary** — the divergence `#11-inline-item-boundary-soft-wrap` owns, a break
  css-text-3 §5.5 licenses nowhere. No *marker* can flush a line (M3's marker path carries no
  soft-wrap check), but a later item's first **text** segment can, and that segment is not a tail
  of the run any break fell inside, so (ii) does not reach it. §6's standing rule forbids a cell
  that depends on the divergence, so it cannot be the discriminator whether or not a fixture for
  it exists — and mandating the re-record to serve it would be mandating support for that bug's
  output. **Pre-existing** class, by a stronger form of the memo's own test
  than the usual one: it is not that `origin/main` already fails it, but that **no input
  distinguishes the two states in either direction**, so PR-1d's omission is not a behaviour this
  program introduces. Trigger: any work that gives the packer a **licensed** line-opening route
  whose continuation line carries no rendered content — implementing `<br>`'s forced break (§9's
  `<br>`/`<wbr>` bullet, where the break is what is missing) is the concrete one — or any change
  to `contributes_content` under which a post-break tail segment can fail it. ⚠ **Not** the
  item-boundary slot: discharging that one *removes* the route above rather than creating one.
  Re-eval: 2026-11-01.
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
  gate then found the sentence missing the existing-line qualifier M1 carries). ⚠ **A pre-existing defect the widening would also reach** (rev 60, ledger **A60**): an undecorated inline has rects only from runs keyed to its **own** entity (`place_item`'s push, `pack/mod.rs:703-714`), so in `<span>a<b style="padding:5px">b</b></span>` the span covers `a` alone and does not enclose its decorated child — the *inner decorated / outer not* arm of §6 cell 10. M4's outcome is scoped to M1's emit set; emitting markers for zero-edge boxes is what would bring them under it. Why deferred: a marker for every inline box grants — through M4's unconditional
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
  payload). Trigger: **PR-1d landing** (the program's one other own slot on that disjunct,
  `#11-inline-box-decoration-splits`, carries it too; §10's successor row names both —
  ⚠ **it was two other slots and a row naming three until rev 53**, when
  `#11-inline-min-content-box-edges` was withdrawn, ledger **A47**),
  `#11-inline-root-inline-box` work
  (the same line-height surface — a different mechanism, so not the same slot), or a compat-survey
  hit on `getClientRects()`/line-height of empty inlines. Re-eval: 2026-11-01. Found by Codex on
  #515 (an earlier drafting folded it into the root-inline-box slot, a mechanism mismatch; a later
  one gave "a presence-axis program of its own under the edge-dense rule" as the Why, a
  classification where the ground above was owed — round 22, Axis 3).
* **`#11-inline-item-boundary-soft-wrap`** (new slot, **pre-existing** class): `place_item`
  flushes the line whenever the next placed item does not fit (`pack/mod.rs:658` at `22de3078`:
  `if self.current_inline + trimmed_width > containing_inline_size && self.on_line`), with no
  break-opportunity test at the item boundary — break opportunities are found only *inside* a
  text run (`find_break_opportunities`, called in `build_pack_items` at `pack/items.rs:72`) — so `aaaaaaaaaaaa<span></span>b` wraps before `b` although
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
  ⚠⚠ **Scope, narrowed at R11, and a preservation clause that is part of the discharge.** The
  subject is the item boundaries css-text-3 §5.5 does **not** license — inline-box boundaries.
  It is **not** every item boundary: the same section's next bullet gives "a soft wrap
  opportunity before and after each replaced element or other atomic inline, even when adjacent
  to a character that would normally suppress them, including U+00A0 NO-BREAK SPACE", so at an
  **atomic** boundary the per-item flush is accidentally right and there is nothing here to fix.
  **A discharge must therefore preserve the atomic boundaries' opportunities**: the obvious
  implementation — deriving the licensed set from the concatenated text, where an atomic
  contributes no characters — would remove them, turning a correct behaviour into a regression
  while closing this slot. §8's PR-1b suite invariant states the licensed set with the atomic
  term for the same reason, and a pure U+FFFC encoding does not substitute for it (U+FFFC is
  UAX #14 class CB, so LB12/LB12a suppress the break beside an NBSP that §5.5 licenses —
  measured through `find_break_opportunities`). Ledger **A32**.
  ⚠ **The class has been declared swept twice and was not, so this is the record of what was
  measured and the guarantee is §8's suite-level invariant, not this list** (R5). Codex R3
  measured every §6 cell whose expected result **states** a second line against the tagless
  control: **15d** and **17d(b)** break at a genuine opportunity; **17c** broke at the boundary
  and **17** and **17f** specified no markup, so all three were re-fixtured; **19** is outside,
  its break being **forced**. That population missed the two members whose dependence is not in a
  *stated* second line — **cell 21**'s arm (b), whose second line the packer discards so it never
  read as a line (withdrawn from §6 to **this slot**, constructible when the slot lands), and
  **cell 15c**, recorded here as outside the class because it "declines to assert where `b`
  lands", true of the *assertion* and false of the *fixture* (its window opened below
  `M("aaaab")`, where today's layout already breaks at an unlicensed boundary). **A cell can be in
  this class through its width and not through its markup, which
  is why per-cell vigilance is not the instrument.** The finding that opened the slot was
  discharged instance-scoped, which is how 17c survived it; the R3 sweep was population-scoped,
  which is how 21(b) and 15c survived that, and R5's own repair of 15c survived it once more
  (ledger **A18**) — 15c now carries no following text at all (§6). The §6 preamble states the
  rule that kept missing it.
  ⚠ **Two cells the R5 sweep read and could not settle, recorded rather than passed over.**
  (i) **Cell 24d** asserts `line_count` **1** for two adjacent glyphless decorated boxes on the
  strength of css-text-3 §5.5, and has **no oracle until PR-1d**: today the markup is Shape B
  twice, `items.is_empty()` fires and `line_count` is 0, so nothing can be measured against it
  now. The argument that it holds is by construction — the marker path calls M3's shared core
  with **no soft-wrap check**, so no marker can flush a line — and by construction is what this
  class keeps falsifying, which is why it is flagged as the likeliest next instance and why the
  suite-level invariant (§8, PR-1b) is what will actually decide it. (ii) **Cell 20's
  `position:absolute` arm** puts an `InlineItem::Placeholder` (`inline/collect.rs:225`) between
  its neighbours, and css-text-3 §5.5's sentence opens with **out-of-flow boxes** before it
  reaches inline box boundaries — the same rule, the other half. ⚠ **Measured, elidex satisfies
  that half by construction and the cell should say so**: `pack()`'s `PackItem::Placeholder` arm
  (`pack/mod.rs:635-642` at `22de3078`) records a static position and returns — it never calls
  `place_item`, so the per-item flush cannot fire at a placeholder and the out-of-flow half of the
  rule is met where the inline-box half is not. That asymmetry is recorded on the cell; no
  behaviour of this program changes on it.
* **`#11-writing-mode-inline-blockification`** (new slot, **pre-existing** class, opened by R11):
  css-writing-modes-4 §3.2's first different-`writing-mode`-than-parent bullet — "If the box would
  otherwise become an in-flow box with a computed display of `inline`, its display computes
  instead to `inline-block`" — is unimplemented. **Measured by the property rather than by the
  word "blockify"**: the sites that rewrite a computed `display` toward a block-ish value are
  harvested with `grep -rn '\.display = ' --include='*.rs' crates/` and each hit read, which
  returns **four** — `resolve/mod.rs:182` (abs/fixed) and `:185` (float), CSS 2.1 §9.7;
  `crates/layout/elidex-layout-flex/src/helpers.rs:69`, Flex §4.2; and
  `crates/layout/elidex-layout-grid/src/helpers.rs:53`, Grid §6.1 (the grep returns the flex and
  grid *assignments* at `helpers.rs:71` / `:55`; the cited lines are the `.blockify()` calls two
  above, which is what reading each hit gives) — none keyed on `writing-mode`,
  with `resolve_writing_mode_properties` (`resolve/mod.rs:235-277`) writing `direction`,
  `unicode-bidi`, `writing_mode` and `text_orientation` and no `display`. The complement is part
  of the measurement: the one further `.display = ` write outside tests, `pseudo.rs:55`, assigns
  the pseudo's *initial* `Display::Inline` and is not a blockification. ⚠ **And the existing
  helper is the wrong rule**, so this is not a wiring job: `display.blockify()`
  (`resolve/mod.rs:218-219`) takes `inline` to `block` (`:469`), where §3.2 asks for
  `inline-block` — an engine-wide display distinction, not a one-line branch. Why deferred:
  implementing it changes the computed display of **every** element whose writing mode differs
  from its parent's, engine-wide — a style-resolution change in `elidex-style` with its own
  invariant axis (percentage bases, intrinsic sizing, the box tree), orthogonal to decoration and
  observable today with no marker involved, so `origin/main` already fails it. It is **not** a
  rider on an inline-layout program. What this program contributes is the conservative
  degradation: M1 sources the payload's writing mode from the IFC root, so a marker emitted for a
  box §3.2 should have made atomic still maps its edges on the axis the IFC is laid out in (§5.1
  M1, §3's css-writing-modes-4 §3.2 row). **Its discharge retires §6 cell 12f**: a conforming
  engine computes that span to `inline-block`, whose inner display type is `flow-root`, so M1's
  **inner-flow** conjunct excludes it and the cell loses its subject. **Not** an own
  deferral and it does not enter §5.3's per-PR count. Trigger: any `writing-mode` or
  block-flow correctness work, any work on the computed-display cascade in `elidex-style`, or a
  compat-survey hit. Re-eval: 2026-11-01. Found by Codex on #515 (R11).
* **`#11-intra-word-shaping-across-line-break`** (new slot, **pre-existing** class, opened by
  R12): css-text-3 §5.5's intra-word shaping clause — "When shaping scripts such as Arabic wrap
  at unforced soft wrap opportunities within words … the characters must still be shaped (their
  joining forms chosen) as if the word were still whole" (`body css-text-3
  line-break-details`) — is unimplemented, and the mechanism §3's row used to cite for it answers
  the **inverse** question. `pack/mod.rs:744`'s `last_placed_entity` coalescing re-joins the
  engine's *within-line* segmentation and is scoped to one line **by explicit design**:
  `flush_line` clears the field (`:439`) under a comment that says why (`:436-438`). Two
  fragments of one word on two lines are therefore shaped independently —
  `builder/inline_flow.rs:107-124` iterates per line, each `InlineFlowRun::Text` carrying its own
  string into its own rustybuzz call (`elidex-shaping/src/shaping.rs:133`) — so the trailing
  fragment's first letter takes an initial form where the spec's own worked example requires a
  medial one. **Reachable without any unimplemented property**: the clause's four named triggers
  (`word-break: break-all`, `line-break: anywhere`, `overflow-wrap: break-word` / `anywhere`) and
  hyphenation are all absent — none of `overflow_wrap`, `word_break`, `line_break`, `hyphens` is
  among the **122** `pub <name>:` declarations of
  `crates/core/elidex-plugin/src/computed_style/mod.rs` — but the engine takes its opportunities
  from UAX #14 whole (`unicode_linebreak::linebreaks`,
  `crates/text/elidex-linebreak/src/lib.rs:27`), which returns an interior `Allowed` break inside
  an Arabic word at a joining-**transparent** SOFT HYPHEN U+00AD (`linebreaks("نوش\u{00AD}تن")` →
  `[(8, Allowed), (12, Mandatory)]`, against `[(10, Mandatory)]` for the unbroken word, on the
  workspace's own `unicode-linebreak 0.1.5`). ⚠ **The evidence is structural plus that probe: no
  Arabic fixture was rendered end-to-end**, and the slot inherits that limit rather than
  implying a behavioural measurement — closing it starts by building one. Why deferred: making a
  fragment shape as part of its whole word is a **shaping-layer** contract — the shaper needs the
  run's cross-line context, or the layout has to shape once and slice the glyph run — which is
  `elidex-shaping` / `elidex-text` work with its own invariant axis, engine-wide and orthogonal
  to inline decoration; nothing in this program creates it or touches it, and `:744`'s
  within-line coalescing is unaffected either way. **Pre-existing** class on the memo's own test:
  the divergence is observable today, on ordinary soft-hyphenated Arabic text with no marker
  involved, so `origin/main` already fails it. **Not** an own deferral and it does not enter
  §5.3's per-PR count. Trigger: any shaping- or line-breaking-fidelity work in `elidex-shaping` /
  `elidex-text`, any work implementing `overflow-wrap` / `word-break` / `line-break` /
  `hyphens` (which widen the population the clause reaches), or a compat-survey hit. Re-eval:
  2026-11-01. Ledger **A36**.
* **`#11-shaping-break-vertical-align-and-isolation`** (new slot, **pre-existing** class, opened
  by R15): css-text-3 §7.3's **first** normative sentence lists three triggers and elidex
  implements **none** of them as triggers — it breaks on entity identity alone, which over-covers
  all three wherever the box places content. In the **member-less** corner, where the two texts
  around the box stay under one entity, only trigger 1 closes, and only because M1 emits a marker
  for a non-zero edge (PR-1b). A box carrying `vertical-align` other than `baseline`, or opening a
  bidi isolation boundary, and **no** edges gets no marker and keeps the runs coalesced:
  `<p>a<span style="vertical-align:super"></span>b</p>` shapes "ab" as one word. Measured at
  `154bac3f`: `git grep -nE 'vertical_align|unicode_bidi|Isolate' 154bac3f --
  crates/layout/elidex-layout-block/src/inline/` is empty, and `last_placed_entity` has exactly
  **three** write sites, `pack/mod.rs:190` (the constructor), `:439` (the `flush_line` reset) and
  `:772`, so no property other
  than identity can end a run (⚠ **"two", the constructor uncounted, until R22** — §1.4's copy of
  this measurement carries the convention, which is §4's: an initialiser is a write). **Why deferred**: the fix is a shaping-break predicate over §7.3's
  three triggers — a line-breaking concern of the packer's core loop, not a decoration one — and
  it needs `#11-inline-zero-edge-box-in-item-stream` first, that widening being a **prerequisite
  rather than the fix**, since a zero-edge marker still fails M3's `has_inline_axis_edge` gate.
  **Not** an own deferral: observable today with no marker involved. **Trigger**:
  `#11-inline-zero-edge-box-in-item-stream`'s discharge. **Re-eval**: 2026-11-01.
* **`#11-shaping-break-at-unchanged-inline-boundary`** (new slot, **pre-existing** class, opened
  by R15): css-text-3 §7.3's **second** normative sentence — "Text shaping must not be broken
  across inline box boundaries when there is no effective change in formatting, or if the only
  formatting changes do not affect the glyphs (as in applying text decoration)" (`body css-text-3
  boundary-shaping`) — is unimplemented, and unimplementable without a mechanism the engine does
  not have. elidex breaks shaping at **every inline box boundary that separates placed content**,
  and it decides on nothing but entity identity: a text run
  takes the **parent element**'s entity (`collect.rs:309`, `StyledRun::from_style(parent_entity,
  …)`; only the pseudo arm at `:263` passes the child), so `<p>a<span>x</span>c</p>` yields three
  runs, and both the measurement and the shaping are per-run — `measure_text` on one run's text
  (`inline/measure.rs:57`) or one of its segments (`:78`), and one `rustybuzz` call per
  `InlineFlowRun::Text` (`builder/inline_flow.rs:107-124` → `elidex-shaping/src/shaping.rs:133`).
  **Reachable with no unimplemented property and no CSS at all**: an unstyled `<span>`,
  `<b>`, `<a>` or `<em>` **with text in it** inside a paragraph is the fixture, which is why the
  class is engine-wide rather than a corner. ⚠ **The negative case is the one the engine gets
  right, and stating it is what keeps the claim from over-reaching**: a *member-less* inline box
  places nothing, so the texts around it stay one run and are shaped together — which is what the
  sentence requires. The divergence is exactly the complement: a box that contributes content
  and changes no formatting. ⚠ **The measurement is structural, on the same terms as the sibling
  above**: the run split and the per-run shaping call are traced, no joining-script fixture was
  rendered end-to-end, and the slot inherits that limit — closing it starts by building one.
  ⚠ **It is not the sibling's class restated.** `#11-intra-word-shaping-across-line-break` is
  about a word split by a **line break** (`:744`'s coalescing is scoped within one line by
  design); this one is about a boundary **within** one line that the engine splits when the spec
  forbids it — the same shaper, the opposite error, and neither implies the other. Why deferred:
  not breaking requires shaping across run boundaries — the shaper needs the neighbouring runs'
  text, or layout has to shape once per *formatting-identical span* and slice, plus an "effective
  change in formatting" comparison the engine has no notion of — which is `elidex-shaping` /
  `elidex-text` work with its own invariant axis, engine-wide and orthogonal to inline decoration.
  **Pre-existing** class on the memo's own test: `origin/main` already fails it, on markup with no
  marker involved, and PR-1b's boundary break runs in the *other* sentence's direction, so nothing
  in this program creates the gap or widens it. **Not** an own deferral and it does not enter
  §5.3's per-PR count. Trigger: any shaping-fidelity work in `elidex-shaping` / `elidex-text`, any
  work that gives layout a formatting-identity comparison across runs (a shaping cache keyed on
  style would be one), or a compat-survey hit. Re-eval: 2026-11-01. Ledger **A41**.
* **`#11-replaced-inline-no-atomic-layout`** (new slot, **pre-existing** class, opened by R15):
  a **replaced** element whose computed `display` is `inline` — the ordinary `<img>` — gets no
  inline layout at all. css-display-3 §A makes it an *atomic inline*, but `is_atomic_inline`
  (`inline/collect.rs:14-19`) answers the question from display keywords only
  (`InlineBlock`/`InlineFlex`/`InlineGrid`/`InlineTable`), so the IFC takes it for a plain inline
  box, recurses into its children, and emits nothing: `<p>a<img src=x>b</p>` advances the cursor
  by zero, the element gets no `LayoutBox` from the inline path, and `b` sits where `a` ends. The
  inline module reaches no replaced element anywhere — `git grep -n
  'replaced\|ImageData\|get_intrinsic_size' 154bac3f --
  crates/layout/elidex-layout-block/src/inline/` returns exactly one hit and it is the doc comment
  at `inline/styled_run.rs:12` — while the sizing the case needs already exists one directory
  over, in `block/replaced.rs`, reachable only from `block/mod.rs`. Why deferred: routing a
  replaced element into the item stream is an **atomic-inline** feature — an `InlineItem::Atomic`
  fed from intrinsic sizing, with baseline alignment and its own `vertical-align` behaviour —
  whose subject is the IFC's treatment of atomics, not the decoration of inline boxes; this
  program neither creates it nor touches it. **Pre-existing** class on the memo's own test:
  `origin/main` already fails it on author-reachable markup with no marker involved. ⚠ **It is
  the gap §9's predicate-prereq bullet already reasons about at length and, until R15, gave no
  token** — the bullet's "closing the gap is separate, pre-existing work neither this PR nor this
  umbrella takes on" named a destination that did not exist (ledger **A42**). What the prereq's
  predicate delivers is the *classification*; the layout is this slot's. **Not** an own deferral
  and it does not enter §5.3's per-PR count. Trigger: any atomic-inline or replaced-element layout
  work, any work on `is_atomic_inline`'s answer (the predicate prereq PR is the first candidate),
  or a compat-survey hit. Re-eval: 2026-11-01.
* **`#11-inline-atomic-intrinsic-contribution`** (new slot, **pre-existing** class, opened by R27):
  both intrinsic passes skip `InlineItem::Atomic` — "Atomic inline-level boxes contribute zero
  (their intrinsic width is not yet computed at this stage)" (`inline/measure.rs:13-14`, `:42-43`
  at `154bac3f`; `collect.rs:246` emits the item with `inline_size: 0.0`, filled only by the
  layout pass's `layout_atomic_items`) — while layout advances by the atomic's margin box. So
  `<span style="padding:10px"><span style="display:inline-block;width:100px"></span></span>` under
  shrink-to-fit gets intrinsic sizes of 20px against a 120px line. The spec outcome is the atomic's
  intrinsic size contribution, "based on the outer size of the box" (css-sizing-3 §2.2, `body
  css-sizing-3 contributions`), as css-sizing-3 §5.2 defines it (`body css-sizing-3
  intrinsic-contribution`). **Pre-existing** on the memo's own test — `origin/main` fails it with
  no marker involved — and decoration-independent; **not** an own deferral, and it does not enter
  §5.3's per-PR count. Why deferred: the fix computes an atomic's intrinsic contribution inside
  the IFC's intrinsic passes, which is atomic-inline sizing, not inline-box decoration; PR-1b's
  edge terms neither create nor widen the disagreement. Trigger: the **min-content prereq PR's
  plan-review** (that PR builds the accumulator an atomic term would land on; not bundled into
  it), any change to `measure.rs`'s treatment of `InlineItem::Atomic`, or a compat-survey hit.
  Re-eval: 2026-11-01.
* **`#11-shaping-no-last-resort-font`** (new slot, **pre-existing** class, opened by R16):
  css-inline-3 §5.3 gives a glyphless inline box a strut "with the metrics of the box's **first
  available font**" (§1.5's quote) — a font the spec takes to exist. elidex has none.
  `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60-84` at `154bac3f`) maps
  the five CSS generic names onto `fontdb`'s generic variants, treats every other name as a
  specific family, and ends `self.db.query(&query)`, so it answers `None` when nothing matches —
  **no last-resort family is appended anywhere in the call** (`git grep -niE
  'last.resort|fallback_font|default_family' 154bac3f -- crates/text/` is empty, and the family
  list is `ComputedStyle.font_family` unextended, `inline/styled_run.rs:90`). **What R16 reached
  it through**: a decoration-only line then has no baseline source at all.
  `<p><span style="font-family:no-such-family;padding:1px"></span></p>` carries no text, so
  `inline/mod.rs:200`'s measurability gate — which sits inside `if has_text` (`:191`) — never
  runs; M5 commits the line on the inline-axis edge; M7's tentative needs `query` and gets
  `None`; and the IFC finishes with `first_baseline == None` where §5.3 asks for the strut's.
  Why deferred: supplying a last-resort font is an `elidex-shaping` decision — which face, how it
  is discovered per platform, what holds when the system offers none — with its own invariant
  axis, and it is reachable from **every** `font-family` that resolves to nothing, far past this
  program's markup. **Pre-existing** class on the memo's own test: the same `None` reaches
  `measure_text` (`db.query(…)?`, `elidex-shaping/src/measurement.rs:54`), so `origin/main`
  already renders ordinary text in an unavailable family as nothing, with no marker involved.
  **Not** an own deferral and it does not enter §5.3's per-PR count. ⚠ **It is neither adjacent
  font slot**: `#11-inline-fontless-measurability-gate` is the **text-driven** early return, which
  this markup never reaches, and `#11-inline-fallback-font-strut` is §5.3's *second* condition,
  which needs glyphs to exist before provenance can classify them. Trigger: any font-fallback or
  font-discovery work in `elidex-shaping`, `#11-inline-fallback-font-strut`'s discharge (the same
  layer), or a compat-survey hit. Re-eval: 2026-11-01. Ledger **A45**.

Own deferrals **per PR** (the policy's unit), for all **ten crate PRs** of the program (§8's two
bookkeeping PRs — neither touches `crates/` — sit outside check 9's roll-call: the docs-only
**approval PR** opens none by construction; the **tooling PR** ships skill infra — checker
generalisation, wiring and two-file awareness, §9 — and its own-deferral count is its own memo's
call, the treatment the predicate prereq gets
below): PR-1a opens 1, PR-1b opens none, PR-1c opens 1, PR-1d opens none,
the seam-3 prereq opens 1, the dead-arm prereq opens none, and the predicate prereq opens none
**here** — what it opens is its own memo's call,
since it is carved precisely because its questions are not this memo's to settle (§9).
⚠ **PR-1b's was 1 until rev 53**; the ground for withdrawing that slot rather than re-tagging it
is in its own bullet above (ledger **A47**).
⚠ **That line is wrapped deliberately**: until rev 34 the wrap fell between `dead-arm` and
`prereq`, which hid the tuple from the checker's own regex — the remediable half of the two
defects the paragraph below records (round 26, Axis 3).
⚠ The **reconciler prereq** (R15's carve, registered here at R16; §8), the **min-content
prereq** (R17's carve, registered here at rev 53; §8) and the **end-of-line white-space prereq**
(the user's 2026-09-23 carve, registered here at rev 63; §8) take the same treatment for the same
reason, and open none here.
⚠ The seam-3 prereq's **1** is `#11-inline-fragmented-fn-seams-1-2`, and the count is the SoT's
landing record for #508, not this memo's: the slot is **mixed** class — seams 1 and 2 are
pre-existing, but `reconcile_flows`' extracted signature is *created* by #508, so its own half
makes it #508's one own slot (≤3 ✓). An earlier drafting wrote "opens none" by counting the
pre-existing half alone; #508's memo recorded the contradiction and this revision resolves it in
the SoT's favour. The dead-arm prereq's "none" is verified at #511's landing (it registered nothing).
⚠ `plan-xcheck.py`'s check 9 — **as #518 landed it (`4394af4c`), the checker this branch now
carries; this paragraph was re-derived then** — reads **seven** of this program's ten `opens`
statements, not ten (six until #518, which added `predicate prereq` to its alternation).
⚠ **It read five until rev 34, and the two it missed failed for *different* reasons — one
of them remediable here** (round 26, Axis 3; an earlier drafting of this very sentence asserted
only the first reason, and the revision that added the second gave the remediation for the first
alone). (a) Its alternation is closed (`PR-1[a-z]|seam-3 prereq|dead-arm prereq` then), so
`predicate prereq` was unmatched by name until #518 — and `reconciler prereq` (R16's carve),
`min-content prereq` (R17's) and `end-of-line white-space prereq` (rev 63's) are the names it still does not reach, which is the
drift the tooling task's widening is now sized against, and it grows once per carve —
**not fixable here**: widening the alternation is the
plan-checker tooling task's (§9). (b) `dead-arm prereq` **is** in the alternation and was
nonetheless unread, because the pattern's literal space fell on a **line wrap** in the sentence
above — **fixed here, by rewrapping that line at zero cost**, which is the disposition (b) was
owed and did not get. The harvest, reproduced by the checker's own regex over this file —

```
grep -noE '(PR-[0-9]+[a-z]|seam-3 prereq|dead-arm prereq|predicate prereq) opens? ([0-9]+|none)' <memo>
```

— now returns eleven tuples: `PR-1a`/`PR-1b`/`PR-1c`/`PR-1d` **twice** each (once from the per-PR
bullets above, once from this sentence) and `seam-3 prereq`, `dead-arm prereq` and
`predicate prereq` **once** each.
Against them §10 carries **three** `(own)`-tagged rows (PR-1a, PR-1c, seam-3 prereq —
`awk '/^## §10\./,0' <memo> | grep -cF '(own)'` → 3), so check 9 is **live in both directions**
for PR-1a, PR-1b, PR-1c, PR-1d and the seam-3, dead-arm and predicate prereqs — PR-1b's "none" against
no row is as live a pair as a "1" against one — while `reconciler prereq`,
`min-content prereq` and `end-of-line white-space prereq` still hold in the reverse direction alone: an `(own)`-tagged §10 row naming
any of them would fail as `§10 tags … which §5.3 does not account for`. ⚠ **It was four rows
until rev 53**, the fourth PR-1b's `#11-inline-min-content-box-edges`; that row is **deleted**,
not re-tagged, and the figure above is re-measured rather than decremented. ⚠ **And check 13's
closed world is the reason three of the four carves take no §10 row of their own**: its prereq
alternation is the closed three (`(seam-3|dead-arm|predicate) prereq PR`), which admits the
**predicate** prereq but not the reconciler, min-content or end-of-line white-space ones, so a row
tagged to a fourth, fifth or sixth prereq routes to nowhere by the checker's own reckoning (`:568` admits only
those three; `:584` is where the fourth tag fails — `4394af4c` coordinates). The precedent is the predicate
prereq's, which the alternation does admit and which carries no §10 row anyway, its registrations
sitting in §5.3, §8 and §9; the reconciler, min-content and end-of-line white-space prereqs follow
it, and for them the closed world makes it the only option; what §10 does carry for the reconciler is the
`#11-inline-relayout-box-staleness` row, whose action names the prerequisite that discharges the
slot. The min-content prereq needs no such row, because the slot it would have discharged is
withdrawn rather than left standing (**A47**) — the two carves differ exactly there: one
discharges a **pre-existing** slot that outlives the program, the other cancels an **own**
deferral that only this program would ever have opened.
⚠ This is a claim *about* the checker that the checker cannot check
([[feedback_prose-rules-cannot-fix-unexecuted-claims]]); the two commands above produced it, and
re-running them is the only thing that keeps it true. Widening the alternation is the plan-checker
tooling task's (§9), not a fix to make here. `#11-inline-root-inline-box`,
`#11-inline-item-boundary-soft-wrap`, `#11-inline-fallback-font-strut` (rev 33),
`#11-inline-decoration-paint-path` and `#11-inline-fontless-measurability-gate` (both R4),
`#11-block-in-inline-anonymous-block-split` (R8), `#11-resize-observer-inline-empty-content-rect`
(R9), `#11-inline-open-box-strut-on-continuation-line` (R10),
`#11-writing-mode-inline-blockification` (R11) and
`#11-intra-word-shaping-across-line-break` (R12),
`#11-shaping-break-at-unchanged-inline-boundary`, `#11-shaping-break-vertical-align-and-isolation` and `#11-replaced-inline-no-atomic-layout`
(all three R15), `#11-shaping-no-last-resort-font` (R16),
`#11-shaping-across-formatting-change-boundary`, `#11-inline-height-normal-layout-bounds`,
`#11-inline-baseline-alignment`, `#11-pseudo-generation-on-replaced-originating-element`,
`#11-used-direction-text-orientation-upright` and `#11-line-box-float-narrowing` (all six from
R20's enumeration-completeness audit, §9), `#11-inline-atomic-intrinsic-contribution` (R27) and the
dead arm's own disposition are pre-existing class, so none of them enters a per-PR count. ⚠ **Three
of them were added at R10, two of those having been missing before it — and the last thirteen, R11's,
R12's, R15's three, R16's, R20's six and R27's, were added to this list in the same revision that opened them, which is the
discipline the record below asks for**: each round that
opened a pre-existing-class slot stated the class at the slot and at its §10 row and left this
list — the one site that turns the class into the *accounting* claim below — unextended,
which is the enumerated-list drift [[feedback_enumerated-exemptions-leave-the-next-class-authoritative]]
names. The claim the list supports was true throughout (neither slot is `(own)`-tagged in §10, so
check 9 never saw them); what was wrong is that the list stopped being the population.
All within ≤3; `.claude/tools/plan-xcheck.py` cross-checks
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

⚠⚠ **Standing rule for every cell of every PR here: a cell that requires a second line must get
it from a genuine soft wrap opportunity *inside* a run — never from an inline-box boundary.**
css-text-3 §5.5 *Line Breaking Details* (§1.3 carries the sentence): "Out-of-flow boxes and
**inline box boundaries do not introduce a forced line break or soft wrap opportunity** in the
flow." A cell whose second line comes from the boundary is pinning
`#11-inline-item-boundary-soft-wrap` (§5.3) — a **pre-existing bug this memo owns as a slot** — as
expected behaviour, so the cell requires the bug to survive and turns red the day the slot is
discharged. ⚠ **The discriminator is a measurement, and it compares split *points*, not line
counts**: lay the same markup out with the span's tags removed; the wrap is genuine only if the
tagless run breaks at the **same place**. Count alone passes fixtures that are still
bug-dependent; ledger A5 carries the measured instance. ⚠ **The R3 sweep of this class was scoped to the cells that *stated* a second line, and that is
not the class** (R5): it missed cell 21's arm (b), whose second line the packer discards, and
cell 15c, whose *width window* rather than its markup reaches the bug-dependent regime. Both are
dispositioned below; the per-cell reading is superseded by the suite-level invariant §8's PR-1b
DoD carries, and §5.3's slot record keeps the members and the non-members. ⚠⚠ **And a fixture has
two regimes — today's and the one the markers' advance creates — so a reading that settles one
settles half** (R6-b; ledger **A18** is the instance, 15c's raised window). That is why the
guarantee is a predicate **run** over every fixture in **both** regimes and not a rule a reader
applies: §8's DoD requires the PR that lands it to execute it over the whole matrix.

⚠⚠ **Second standing rule: a markup belongs to one cell, and a copy of it elsewhere must name
that cell and assert nothing the owning cell does not.** A copy goes stale the day its cell is
re-fixtured, and the assertion built on it then reads as one about the live fixture — ledger A12
is the measured instance. The population is produced, not listed: `grep -noE '<(p|pre|span)'`
over the memo, each markup literal classified as its own cell's or as a copy and each copy
re-read against the cell it names. Checking the property mechanically is the plan-checker tooling
task's (§9).

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
    reader disposition (§8) reaches a second entity class — a decorated `::before`'s **reported
    box** moves from content span to border box, cell 13's delta on generated content — **not** its
    background and border: an inline pseudo is emitted by
    `elidex-render/src/builder/inline.rs:205-213`, which pushes a `StyledTextSegment` from its
    `TextContent` and `continue`s, so it never reaches `walk` and no chrome is drawn for it either
    (`#11-inline-decoration-paint-path`; ledger A8).
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
    is unimplemented on this path, pre-existing — `#11-block-in-inline-anonymous-block-split`, §9);
    it pins only that M1 emits nothing for it. ⚠ **This cell is the inner-decorated half only**; the
    outer-decorated half, where M1 *does* emit, is cell 6i, and the residual both sit on is that
    slot's.
6i. **A decorated inline whose only content is a block-level box still gets markers** (M1, the
    own-style ground; Codex R8-b) — `<p>a<span style="padding:5px"><div>x</div></span>b</p>`: the
    `<span>` is non-replaced, outer `inline`, inner flow, with a non-zero edge, so M1's predicate
    holds and `InlineBoxStart`/`InlineBoxEnd` bracket the recursion at `collect.rs:291` — the
    recursion that collects the `<div>`'s text into **this** IFC, because
    `collect_inline_items_inner`'s four filters (`display:none`, absolutely positioned, atomic
    inline, pseudo-element) do not select a block-level child and the arm is the fall-through.
    ⚠ **What discriminates it**: the competing answer is not hypothetical, it is a function in the
    same file — `has_direct_block_child` (`collect.rs:70`), whose one call is
    `positioned_subflow_key`'s at `:111` — so "suppress the markers when the subtree contains a
    block-level box" is
    the predicate a reader reaches for to make the §9.2.1.1 residual unreachable, and it answers
    this markup **no marker** where M1 answers **markers**. Cell 6e is the converse (the block
    itself gets none); no other cell separates the box's **own** style from its *subtree*, as 6g
    separates it from the entity's origin. Observed through the PR-1a harness and
    **behaviour-neutral** here. ⚠ It asserts **nothing** about correct block-in-inline layout: CSS 2
    §9.2.1.1 breaks the inline around the block into two boxes, and this path produces neither —
    pre-existing, and `#11-block-in-inline-anonymous-block-split` (§9) owns it along with the
    consequence PR-1b then PR-1c and PR-1d define over the flattened shape. Does not flip at
    PR-1d: the IFC's own text already keeps the line.
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
    ⚠ **The escape is `has_inline_axis_edge`-gated, so the divergence survives for a
    block-axis-only box, and that residual is pre-existing** (R4). Second assertion, at PR-1d:
    `<p><span style="padding-top:10px;font-family:no-such-family">x</span></p>` still returns
    `line_count: 0`. Measured today at `22de3078` over the whole configuration space, which is
    what makes it pre-existing rather than created: block-axis edge only **0**, inline-axis edge
    **0**, no edge at all **0** — the gate is blind to edges, so it is the *measurability* guard
    M5 calls it and not a partial clause enforcement this program degrades. PR-1d lifts the
    inline-axis arm to 1 and leaves the other two where they are; nothing gets worse, which is
    why the disposition is a slot and a pin rather than a wider lift —
    `#11-inline-fontless-measurability-gate` (§5.3). Widening the escape to clause 1
    (`contributes_content`) is that slot's, and it is not free: the `:200` return also runs
    `clear_inline_flows` and `remove_one::<ColumnFlowSlice>`, so lifting it commits an IFC whose
    text cannot be measured at all.
    ⚠ **Harness**: `setup_inline_test` (`inline/tests/mod.rs:54`) returns `None` when the test
    families are unavailable and every caller early-returns, so the suite currently *skips* rather
    than *exercises* the no-font path. **PR-1a's DoD carries the harness change** — a deterministic
    way to force `any_font == false` — because PR-1a is where this cell is first asserted; without
    it the cell is unconstructible, which is the defect class this memo refuses to ship: a cell no markup can construct.
**PR-1b inline-axis advance:**

3. `margin` alone, **including negative** — requires M1's `resolve_box_model` sourcing.
   ⚠ **It carries a markup from R6 on, and the markup is what discriminates M5's *margin* term**
   (R6-d): `<p>a<span style="margin:10px"></span>b</p>`, whose only non-zero edges are margins, so
   M5's `padding ∨ border ∨ margin` is true **through its margin term alone** and M3's sum
   advances by it. Two assertions: `b` is displaced by 20px, and the `a`/`b` runs **stop
   coalescing** — M3's side-specific shaping break, which reads M5's derivation, so the cell fails
   if that derivation is blind to margins. No other cell isolates that term — §8's PR-1c DoD
   carries the set-level read — and M5 has one derivation site with two readings, so this one
   markup pins the term for both. The shaping-break
   mechanism itself is cell **14**'s, on `padding:1px`; what is asserted here is only that the
   same gate fires on a margin-sourced edge. The **negative** arm is the same markup with
   `margin:-10px`: the advance is the signed sum, and the break still fires, the gate testing
   non-zeroness and not sign.
3c. **`border` alone, in the advance and in both intrinsic sizes** (M3, M8; R32) — the one
    edge kind no cell of the advance or intrinsic path carries: cell 2 stops at marker emission
    and the existence flip, 13c asserts the separate `LayoutBox.border` **field** (PR-1c), and
    every M8 cell — 25, 25b, 25c — is padding. Two markups of its own, because the two
    observables are not on one:
    * **advance** — `<p>a<span style="border:10px solid"></span>b</p>`: `b` is displaced by
      **20px**, the same relation cell 3 asserts for `margin` and cell 14b's negative arm for
      "no inline-axis component".
    * **intrinsic** — `<div style="display:inline-block"><span style="border:10px solid">x</span></div>`,
      `width` left `auto`: its **max-content** and **min-content** inline sizes are each 20px
      wider than the same box with the span's `border` removed, the test computing both boxes and
      comparing them as cell 25 does rather than stating literals.
    **Rejects** the one producer the rest of the matrix admits: `border` dropped from M3's
    inline-axis sum and from M8's edge term while M1, M5 and M4 keep it — markers still emitted
    (cell 2), the line still kept (cell 2 is the flip set's border member), `LayoutBox.border` still real (13c), and yet
    the following text overlaps the border and shrink-to-fit comes out 20px too narrow. ⚠ The
    fixture sets the resolved `EdgeSizes` on the span's `ComputedStyle` directly, as cell 2's ⚠
    establishes for border (`border-style` resolution is `elidex-style`'s).
3b. **Cancelling pair, the sum/disjunction contrast on the advance side** — the edge pair is
    `margin:-10px;padding:10px`, so that **each** inline-axis side cancels: the side's components
    sum to zero while every component is non-zero, which is M3's **sum** against M5's
    **disjunction**. The cell carries **two markups of its own**, because its two observables are
    not visible on one — a shaping break needs two same-entity text runs, and the line-finality half
    needs a trailing collapsible space that the box's inline-**end** side follows:
    * **shaping** — `<p>a<span style="margin:-10px;padding:10px"></span>b</p>`: the marker's own
      inline-start components are non-zero, so css-text-3 §7.3's break fires at the boundary and
      the `a`/`b` runs stop coalescing, for a box that moves the cursor not at all.
    * **line-finality** — `<p style="text-align:center"><span style="margin:-10px;padding:10px">abc </span></p>`:
      the trailing space is inside the box and the `InlineBoxEnd`'s own inline-end components are
      non-zero, and **the marker does not end the space's line-finality** (M3's outcome), so the
      run's `inline_start` is `(W − measure_width("abc")) / 2` at a width `W` the single line fits;
      the test asserts `sp = measure_width("abc ") − measure_width("abc") > 0` first. It rejects a
      producer that ends line-finality on non-zero **components** at a net-zero advance — the case
      cell 16's arms, whose edges all advance, do not reach (re-pinned, ledger **A63**).
    ⚠ **Both markups are written out here rather than borrowed from cells 14 and 16** (R5, ledger
    A12): the borrowed copy of cell 16 was the pre-R2 one, and the assertion built on it became
    false when R2 re-fixtured that cell. The §6 preamble carries the rule.
    ⚠ The line-existence half of the same disjunction — css-inline-3 §2.3 keeping the line — cannot
    be asserted here, because PR-1b's marker path passes the constant `false`; cell 24b asserts it
    in PR-1d, where the substitution makes it true.
4. **Percentage** `padding: 5%` / `margin: 2%` — against `containing_inline_size`.
12b. **`direction: rtl`** (M1): `<p dir="rtl"><span style="padding-left:10px">x</span></p>` —
    `padding-left` is the inline-**end** edge here, so it advances the cursor *after* the content.
    Pins the physical→logical mapping itself (`padding-left` → inline-end under rtl). It does not
    discriminate *whose* direction is used — `direction` is inherited, so the span's own and its
    parent's agree here. Cell 12e does.
    ⚠⚠ **What this cell asserts is a *layout* cursor, and on an RTL line the painted result does
    not follow it** (R7-b). `builder/inline_flow.rs:154-190` at `22de3078`: where the line's bidi
    order is non-identity the renderer **discards every run's baked `inline_start`**, starts the
    paint cursor at `min(inline_start)` over the line's `Text` runs (`:169-173`) and advances it by
    each run's shaped width alone, so a gap the markers opened **between** two runs is not painted
    at all. Measured on that branch, not read off it: with two RTL runs whose baked
    `inline_start`s differ by a 100px hole and with the hole closed, the emitted glyph positions
    are **identical**; the crate's own `converged_ltr_identity_no_reorder` is the control, an
    identity line painting each run at its own baked position. This cell keeps its layout
    assertion and asserts **no** painted position; the loss is the reorder branch's own
    pre-existing shape — the same branch already discards layout's baked justify offsets and says
    so in its comment at `:160-168` — and is routed, with that precedent's owner, to
    **`#11-bidi-full-uba-fidelity`** (§9). Ledger **A24**.
12c. **`writing-mode: vertical-rl`** (M1): `<p style="writing-mode:vertical-rl">`
    `<span style="padding-top:10px">x</span></p>` — the property is on the **containing block**,
    not on the span. The inline axis is vertical, so `padding-top`/`bottom` become the inline-axis
    pair and `padding-left`/`right` the block-axis pair: cell 6's block-axis-only case inverts
    here, and cell 4's percentage basis is the containing block's inline size (its physical
    height). ⚠ The property is on the **containing block**, so the span inherits it and
    css-writing-modes-4 §3.2 does not fire even in a conforming engine; putting `writing-mode` on
    the **span** is a different cell, **12f**. ⚠ An earlier drafting closed here with "per
    css-writing-modes-4 §3.2 that span computes to `inline-block`, i.e. an atomic, and M1 emits no
    marker for it. Both markups are asserted." — the spec's conclusion asserted of an engine that
    does not implement its premise, and the second markup was never a cell of its own (R11).
12f. **`writing-mode` on the span, and the payload follows the *IFC root's* axis** (M1; R11) —
    `<p><span style="writing-mode:vertical-rl; padding-top:10px">x</span></p>`. css-writing-modes-4
    §3.2 would make that span an `inline-block`, i.e. an atomic that M1 emits no marker for, but
    **elidex does not implement §3.2** (§5.1 M1's Grounds carries the four-site measurement; §3's
    css-writing-modes-4 §3.2 row): the span keeps `Display::Inline`, passes M1's predicate and
    reaches the recursion, and a marker **is** emitted. The cell asserts that its payload's
    `WritingModeContext` carries the **`<p>`'s** writing mode — so `padding-top` stays a
    **block**-axis edge and the marker advances the line by nothing, exactly as cell 6's
    block-axis-only case does — and **not** the span's, under which `padding-top` would become an
    inline-axis edge and the advance would be 10px on an axis the IFC is not laid out in.
    ⚠ **The one-sided padding is what makes the cell discriminate, and a uniform `padding:10px`
    would not**: with all four sides equal the two readings agree numerically — the horizontal
    reading takes `padding-left` = 10 and the vertical one `padding-top` = 10, so the advance is
    10 either way and the cell would pass under the design it is meant to refute. `padding-top`
    alone separates them, 0 against 10. (It separates M5's existence predicate the same way, so
    the cell would discriminate at PR-1d too; it is routed to PR-1b because that is where the
    payload's context first has a reader.)
    ⚠ Three things this cell is, stated rather than left to a reader: **(a)** it is constructible
    **only because §3.2 is unimplemented** — in a conforming engine the markup is an atomic and the
    cell has no subject; **(b)** it pins the *conservative degradation*, not conformance, the same
    idiom cell 12d uses for `inline/mod.rs:200`'s measurability gate; **(c)** it is **retired** by
    `#11-writing-mode-inline-blockification` (§5.3), whose discharge computes the span to
    `inline-block` — an atomic M1's **inner-flow** conjunct already excludes, so the cell loses
    its subject rather than changing its answer. Cell 12b and cell 12e pin the `direction` half,
    which does **not** move to the root — `dir` does not blockify, and css-writing-modes-4
    §6.2/§6.4 attribute the mapping to the box. Ledger **A31**.
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
    ⚠ A `padding-top`-only span **does** emit a marker, but **neither** of its sides contributes
    an inline-axis component, so M3's side-specific break gate does not fire and coalescing there
    is the gate's doing, not automatic — cell 14b. (⚠ Both this cell and 14b read
    `has_inline_axis_edge`, the **whole-box** name, until R5; on these two fixtures the whole-box
    and side-specific readings agree, so the cells did not fail — but a gate *described* by the
    whole-box name is the R2-F3 defect restated, which is why §8's item is by the property.)
    Neither contrast holds for
    `vertical-align: super` or `dir`-isolated spans, which css-text-3 §7.3 also breaks on; §3's row
    records those two triggers as unimplemented.
14b. **The gate's negative arm** — `<p>a<span style="padding-top:1px"></span>b</p>`: a marker
    exists, neither of its sides contributes an inline-axis component (so the whole-box
    `has_inline_axis_edge` is false too, and the two readings agree here), so the runs still
    coalesce and **`b` is not displaced — the cursor advance
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
the break at the box's **margin edge** — has no cell asserting the conformant value; cell 17c
**observes** it as an accepted divergence (rev 60). M3 advances the start edge unconditionally,
so that rule is unmet; it is folded into `#11-inline-box-decoration-splits`
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
    `find_break_opportunities`' own split of **one** run (`pack/items.rs:74-83`), i.e. a genuine
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
16. **`text-align`: a marker does not end the trailing space's line-finality** (M3's outcome,
    ledger **A63**) — a **contrast pair**, both arms centred at a width `W` their single line fits,
    both holding `abc ` and a 10px edge, differing only in which marker carries it. Write
    `h = measure_width("abc ") − measure_width("abc")`; the test asserts `h > 0` first with the
    harness's `measure_width` (`tests/mod.rs:31`), as the crate's justify tests do.
    * **(a) the edge precedes the text** — `<p style="text-align:center"><span
      style="padding-left:10px">abc </span></p>`: the run's `inline_start` is
      `10 + (W − 10 − measure_width("abc")) / 2`.
    * **(b) the edge follows the space** — the same markup with `padding-right:10px`: the
      `InlineBoxEnd`'s edge does **not** end the space's line-finality, so the run's `inline_start`
      is `(W − 10 − measure_width("abc")) / 2` — `h / 2` from what the withdrawn side-specific gate
      gave (`(W − 10 − measure_width("abc ")) / 2`), which is the discriminating quantity.
    ⚠ **Both arms are `white-space: normal`, where the outcome needs no choice of mechanism**:
    css-text-3 §4.1.2 step 3 — "A sequence of collapsible spaces at the end of a line is removed, as well as any trailing U+1680 OGHAM SPACE MARK whose `white-space` property is `normal`, `nowrap`, or `pre-line`" — **removes** the trailing collapsible space, so it
    contributes nothing to the aligned line width and the box's 10px edge sits against
    `measure_width("abc")`, which is what both formulas above say. What a `pre-wrap` or `pre`
    space does, and which of a marker's edges css-text-3 §8.2 lets block a hang, are the
    **end-of-line white-space prereq**'s and **PR-1b**'s own plans' (ledger **A67**); so is where
    the box's border-box **end** falls, which a PR-1b cell cannot observe at all.
    ⚠ **`16c` is a withdrawn label, not a lost cell** (rev 63, the idiom cell 17d uses for `17e`
    and the freeze prose for `24f`): it stood only inside this revision's drafts, as the PR-1c
    border-box-end cell, and the label is not reused. Both arms are LTR `horizontal-tb`,
    where `padding-left`/`padding-right` are the inline-start / inline-end pair; the mapping
    itself is cell 12e's.
16b. **Paired contrast** — `<p style="text-align:center">abc </p>`, arm (a) or (b) of cell 16
    with the span removed: step 3 removes the trailing space, so the run's `inline_start` is
    `(W − measure_width("abc")) / 2` — the baseline every arm of cell 16 is compared against, and
    a value today's engine already produces, its trailing-space exclusion being what
    `justify_excludes_trailing_hang_from_opportunities`
    (`crates/layout/elidex-layout-block/src/inline/tests/inline_flow/justify.rs:148` at
    `154bac3f`) already pins on the justify path.

25. **Shrink-to-fit, both intrinsic sizes** (M8): a `float: left` / `display: inline-block`
    containing `<span style="padding:10px">x</span>` must have a **max-content** inline size 20px
    wider than the same box without the padding **and** a **min-content** size 20px wider — both
    PR-1b's own contribution, the min-content one added on the cross-item accumulator the
    min-content prereq PR put in `main` ahead of PR-1b (§8; ledger **A58**). ⚠ **Both halves,
    because shrink-to-fit reads the pair**
    (`min(max_content, max(min_content, available))`, `elidex-layout/src/intrinsic/mod.rs:134`):
    an assertion on max-content alone passes while the box overflows at a small available width.
    An earlier drafting pinned "min-content knowingly unchanged" against the withdrawn slot
    (ledger **A47**).
25d. **An edge-only box with negative margins does not push an intrinsic size below zero**
    (M8; R32) — `<div style="display:inline-block"><span style="margin:-10px"></span></div>`,
    `width` left `auto`: the span's inline-axis edge sum is `−20`, and **neither intrinsic size
    the passes report is negative** — both are **0** on this markup — so the used width
    `shrink_to_fit_width` derives (`elidex-layout/src/intrinsic/mod.rs:134`, consumed at
    `elidex-layout/src/layout/mod.rs:57`; **not** the `positioned/constraints.rs:318` homonym,
    which reads max-content alone) is 0 and no negative containing width reaches layout.
    Ground, and it fixes **where** the floor sits rather than leaving it to taste: css-sizing-3
    §2.2 floors the box's **max-content contribution by its min-content contribution** ("If the
    ideal max-content contribution would be smaller than the min-content contribution (e.g. due to
    the use of negative margins) the effective max-content contribution is floored by the
    min-content contribution"), and §5.2's Note floors the min-content one by the box's own
    minimum ("The min-content contribution is, as always, also floored by the minimum size in its
    own axis") — which for this `inline-block` is `min-width: auto`, "a used value of 0"
    (css-sizing-3 §3.2). So the floor belongs to **the box whose size is being computed**, here
    the `inline-block`, and **nothing clamps the inner inline box's own −20 contribution**: that
    is what the cell asserts and what a producer clamping per marker would get wrong in the other
    direction. **Rejects** propagating `−20` into `shrink_to_fit_width`. How the floor is applied
    is PR-1b's.
25c. **Where a decorated inline's two edges land when its content has several min-content
    segments** (M8) — `<div style="display:inline-block"><span
    style="padding-left:10px;padding-right:10px">a verylongword b</span></div>` (LTR
    `horizontal-tb`, so those are the inline-start and inline-end edges — the mapping itself is
    cell 12e's; `padding-inline` is not a property this engine parses), `width` left `auto`: its **min-content** inline size is
    `max(10 + measure_width("a"), measure_width("verylongword"), measure_width("b") + 10)`, the
    test computing the three from the harness's `measure_width` (`tests/mod.rs:31`) and comparing
    rather than stating literals, with `measure_width("verylongword")` asserted larger than the
    other two as the precondition that makes the arms distinguishable. Ground: css-sizing-3 §2.1
    makes the min-content inline size the one "that would fit around its contents if all soft wrap
    opportunities within the box were taken", so the three words are three segments; css-text-3
    §5.5 adds none at the box's own boundaries ("Out-of-flow boxes and inline box boundaries do not
    introduce a forced line break or soft wrap opportunity in the flow"); and css-break-3 §5.4's
    initial `box-decoration-break: slice` puts the edges only at the unbroken box's own two ends —
    "no border and no padding are inserted at a break" — so the inline-start edge is on the first
    segment and the inline-end edge on the last. **Rejects the two readings cell 25's single-word
    fixture cannot separate**: both edges on the widest segment
    (`measure_width("verylongword") + 20`), and the pair added to every segment
    (`max(measure_width(w) + 20)`). ⚠ It pins **placement**, not the edge term itself — that is
    cell 25's — and it sits on the min-content prereq's cross-item accumulator like cell 25
    (§8; ledger **A58**, **A70**).
25b. **A percentage edge under shrink-to-fit contributes zero to both intrinsic sizes** (M8;
    css-sizing-3 §5.2.1 rule 4; R26) — `<div style="display:inline-block"><span
    style="padding-left:10%">x</span></div>`, `width` left `auto`: its **min-content** and
    **max-content** inline sizes each **equal** those of the same box with the span's
    `padding-left` removed, the test computing both boxes and comparing them rather than stating a
    literal. The percentage is cyclic here — the span's containing block is the inline-block's
    content box, whose inline size is what the two passes compute — so both passes resolve it
    against the `0.0` §3's row for that rule decides. This is the intersection cells 4 and 25
    leave: 4 resolves a percentage against a real size in the layout pass, 25 has a fixed edge
    under shrink-to-fit. ⚠ **The layout pass still resolves the `10%` against the box's used
    content width**, the section's closing bullet ("The containing block's size is not
    re-resolved based on the resulting size of the box; the contents might thus overflow or
    underflow the containing block"): the content overflows the box by that amount. This cell
    asserts the intrinsic half only.

**PR-1c box geometry:**

6. **The emit/existence split's geometric half** (M1/M5) — the box a block-axis-only marker
   does and does not get. ⚠ On a phantom line the box gets **no rect, and so nothing downstream of
   one**: css-inline-3 §2.3 makes the line box "and its in-flow content" non-existent, and the
   discard arm (`:428`) clears the tentative rects. (The clause used to read "no rect and no
   paint"; paint is not among the downstream readers for a static inline at all — §5.2's
   `elidex-render` row, R4.) The marker earns its keep only on a line that exists
   for another reason — `<p>text <span style="padding-top:10px">x</span></p>`, where M4 must fill
   all four `LayoutBox` sides. Two sub-cells: phantom ⇒ no box; co-resident ⇒ full four-sided box.
   The *cursor* half of the same **predicate** — a block-axis-only marker advances the cursor by
   0 — is cell 14b's, in PR-1b. ⚠ It reads "the same markup" until R5, which is false: 14b's is
   `<p>a<span style="padding-top:1px"></span>b</p>`, a different text and an **empty** span, and
   the shared fact is the predicate, not the fixture.
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
13. `<p>a<span style="padding:10px">text</span>b</p>` — the common case §4.3 is about. Two
    assertions: (a) the span's `LayoutBox` carries real edges and its border box is
    `content + padding` **once**, not twice; (b) **`b` is displaced by 20px** — the
    user-visible half of §1.1's "respected *between* inline-level boxes", and user-visible in the
    strict sense, since `b`'s glyphs are painted from the line's `InlineFlowRun::Text`
    `inline_start` (`elidex-render/src/builder/inline_flow.rs`) and the advance moves it — on the
    identity-order path, §7's R7-b scope; this cell's markup is LTR, so it is on it.
    (a) is what the `LayoutBox`-fed CSSOM readback then answers from — `getClientRects()` and
    `getBoundingClientRect()`, and **those two only**.
    ⚠⚠ **All four `client*` members are out of the readback, not just `clientTop`/`clientLeft`**
    (R7-c): cssom-view-1 §6 step 1 is *identical* across `clientTop`, `clientLeft`, `clientWidth`
    and `clientHeight` — "If the element has no associated box **or if the box is inline, return
    zero**" (`body cssom-view-1 dom-element-clienttop` prints all four; §3's CSSOM row quotes it) —
    so once the predicate prereq is in `main`, which is **before PR-1a** (§9), an ordinary inline
    reports **zero** on all four however real M4 makes its edges. Listing two of them as consumers
    of those edges while carving out the other two is the enumerated-exemption shape this memo's
    own front matter refuses ([[feedback_enumerated-exemptions-leave-the-next-class-authoritative]]),
    and it is also what a geometry cell must not pin: no cell of this matrix may assert the
    **pre**-prerequisite `clientWidth`/`clientHeight` padding-box value, which the prereq closes
    (§9's monotone-direction argument). Ledger **A25**. The cell asserts **no**
    painted rect: nothing emits one for a **static** inline, so PR-1c can neither satisfy nor
    violate such an assertion — `#11-inline-decoration-paint-path` (§5.3) owns that gap
    (ledger A8).
13b. **The committed rect takes the *line's* block extent, not the box's — pinned as accepted**
    (Codex R2-F1, in the shape cell 23 uses): `<p style="line-height:40px">a<span
    style="padding:2px;line-height:10px">x</span></p>` — the span's `LayoutBox.content` takes the
    **line's** block extent and its `border_box()` that plus the padding on each side, where
    cssom-view-1 §6 asks for the box's own **border area**: its content area plus padding and
    border. **Required test**: the cell computes both from the produced `LayoutBox` — asserting
    that the content
    height equals the `<p>`'s used `line-height` and that the border box exceeds it by twice the
    padding — rather than from literals, so the numbers live in the assertion. ⚠ **Those two
    clauses were written as present facts until R22** and only the first is one today: the
    content extent is already what an undecorated span gets (the ⚠ below), while the border-box
    half is false at the frame, `inline/pack/boxes.rs:82-84` writing `EdgeSizes::default()` so
    that `border_box()` equals `content` until PR-1c makes the edges real.
    ⚠ **The size the spec asks for is *not* the span's `line-height`, and no number for it is
    available to this memo** (R5; ledger A15). css-inline-3 §5.3 is scoped to "the contribution of
    an inline box to the logical height of its line box" and says "**The layout bounds need not
    correspond to the box's edges**"; the box's own height is css-inline-3 §6.4 *Inline Box
    Drawing Height: the `inline-sizing` property*'s, whose initial `normal` sizes the content area
    "to fit (possibly hypothetical) text from its **first available font**", with the Note "**the
    `line-height` has no impact on the size of an inline box**, it only affects its contribution
    to the logical height of its line box" (⚠ **truncated at the comma, unmarked, until R22**;
    the dropped clause is the one that says where the `line-height` *does* go, which is the
    relation this cell states) — and then declines to fix it:
    "**this specification does not specify how**", the font's maximum ascender and descender
    offered only as an example a UA *may* use. The cell therefore states the **relation** — elidex
    reports the line's extent where the spec asks for a font-derived one — and asserts only the
    quantity elidex produces. (⚠ The padding is the *padding*, not a css-inline-3 §5.3 inflation: css-inline-3 §5.3
    inflates layout bounds by the edges only when `line-fit-edge` ≠ its initial `leading`, as §3's row
    records; the cell asserts the two quantities separately.)
    `commit_aligned_entity_rects` writes `block_size: line_height` — the *line's*
    height — for **every** merged entity (`inline/pack/mod.rs:493`), and css-inline-3 §5.3 composes the line
    *from* each box's bounds rather than assigning the line's extent back to them; every
    `getClientRects` / hit-test / a11y-bounds reader of that rect inherits it — ⚠ **and not `clientHeight`**, which cssom-view-1 §6 step 1 returns zero for on an inline box whatever this rect holds (R7-c; cell 13's ⚠ carries the step).
    ⚠ **Pre-existing, which is why it is pinned and not fixed**: the write is reached through
    `place_item`'s push under `entity != parent_entity` (`:706`), so that content extent is
    already what an **undecorated** `<span style="line-height:10px">x</span>` gets on
    `154bac3f` — drop the `padding:2px` and only the border-box half changes. What this program
    does is widen the population that reaches it (an empty decorated box gains a rect at all,
    cell 14c) and, through PR-1c's real edges, the channels that observe it. Facet (b) of
    `#11-inline-root-inline-box` (§5.3).
    ⚠ **What discriminates**: the markup holds the span's `line-height` well away from its line's,
    so the two candidate models are far apart on it and the cell fails in both directions — if
    per-box bounds ever land, the asserted content height drops from the line's to the box's own,
    and a cell written on a span whose `line-height` equalled its line's would pass under either.
13c. **The three edge sets stay three fields** (M4's invariant (iii); R6-d) —
    `<p>a<span style="padding:1px;border:3px solid;margin:7px">text</span>b</p>`: the span's
    `LayoutBox` carries `padding == 1`, `border == 3` and `margin == 7` on every side, its
    `border_box()` is the content rect inflated by **4** per side (padding + border, the margin
    **excluded** — cssom-view-1 §6's border area) and its `margin_box()` by **11**. ⚠ **The three
    values are pairwise distinct on purpose**: with one shared value a carrier that merged two
    sets, or routed margin into `padding`, would pass, so the cell would catch a *drop* and not a
    *swap*. This is the one PR-1c cell that asserts the `border` and `margin` fields (10c's and 10d's margins — and 10d's atomic border and padding — act through the cursor; 10d(iii) reads the atomic's own `LayoutBox`, which is `layout_child`'s, not this program's `margin` field), and §8's PR-1c DoD carries
    the set-level argument for why one was needed. ⚠ The fixture sets the three resolved
    `EdgeSizes` on the span's `ComputedStyle` directly, as cell 2's ⚠ establishes for border
    (`border-style` resolution is `elidex-style`'s and `elidex-layout-block` has no edge to it);
    `border:3px solid` is the CSS the fixture stands for, not a string the crate parses. ⚠ It
    asserts no cursor displacement — that is cell 13(b)'s, on its own markup, and the advance
    here would be the three-set sum rather than the padding this cell varies.
13d. **A `position:relative` decorated inline's *painted rect* moves, and this is the one cell
    that reads the renderer's output** (M4; R11) —
    `<p>a<span style="position:relative;background:red;padding:10px;border:2px solid">b</span>c</p>`.
    **Run through the real producer** (R32): the markup goes through `elidex-shell`'s pipeline —
    HTML + CSS to a display list, `crates/shell/elidex-shell/src/tests.rs:41`, whose suite already
    matches `DisplayItem::SolidRect` (`:127`) — so the `LayoutBox` the paint reads is the one
    **PR-1c's M4 produced**, not one the test set. Expected, as a **relation** over that box:
    the background is a `SolidRect` at the span's `border_box()`, i.e. its `LayoutBox.content`
    inflated by its `padding` and `border` on all four sides, and **four** border segments, each
    of the thickness `emit_borders` takes from **`LayoutBox.border`** and not from the computed
    `border-width` — **five** `SolidRect`s where today's pipeline emits **one**, the background at
    the bare content box, the 2px `border-style: solid` painting nothing because the inline's
    `LayoutBox` carries `EdgeSizes::default()`. ⚠ **A hand-set box cannot carry this cell** (R32):
    the earlier drafting built it on `elidex-render`'s `consumes_relpos_inline_subflow_with_gap`
    harness, which sets the `LayoutBox` padding and border PR-1c is meant to produce, so it passed
    **before** PR-1c and could not catch a positioned inline omitted from marker emission or from
    the edge assignment — the defect this cell exists for. 13c covers a *static* span's fields and
    §8's end-to-end clause a static `border:5px solid` without looking at painted output, so
    nothing else reaches it.
    ⚠ **The count moves with the rect on this markup** — the count only looked invariant
    because the earlier claim measured a *background-only* span, the one markup of the family
    where 1 → 1 (measured: with `background` and no border, `(24,0) 16x20` → `(14,-10) 36x40`,
    one rect either way). A count taken on one member of a family is not a measurement of the
    family, which is the whole of ledger **A33** — and **A5** is the same failure shape a round
    earlier. The chain is measured rather than argued: `paint_non_sc` skips positioned children
    (`walk.rs:650`) as Layer 5 does (`:583`), Layer 6/7 reaches them through
    `walk_child_with_fixed_check` → `walk` (`:614-629`, `:728`), and `emit_background`
    (`builder/paint/mod.rs:68`) and `emit_borders` (`:382`) both read `lb.border_box()` =
    `content.expand(padding).expand(border)` (`layout_types/boxes.rs:140-142`) — the box M4 puts
    its three real `EdgeSizes` on at PR-1c (§5.2's `pack/boxes.rs` row).
    ⚠ **A6 does not bar this.** A6 refuses adding an `elidex-render` **pass or member**;
    13d adds a **test** that reads the renderer's existing output, which is what
    CLAUDE.md's *Supported-surface testing* discipline requires of a change that moves a
    user-visible painted rect — the alternative is shipping the move unasserted, which is the
    defect rather than the restraint. This cell is the canonical site for that distinction; A6 is
    not re-litigated elsewhere. ⚠ It asserts **nothing** about a *static* inline, whose 0 rects
    stay 0 (measured, same probe) and whose gap is `#11-inline-decoration-paint-path` (§5.3).
    ⚠ It is also the **one** bounded exception to the test-placement paragraph below: it is an
    `elidex-shell` test, that crate being the only host that runs the producer and observes the
    display list together (`crates/shell/elidex-shell/src/tests.rs:41`, `:127`) —
    `elidex-layout-block` cannot observe a display list, and a layout-side cell could only
    re-assert cell 13c's edges.
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
    overflows"; §1.3) — `<p style="width:W">aaaa<span style="padding:20px"></span></p>` with
    **`measure_width("aaaa") ≤ W < measure_width("aaaa") + 40`**: the span's `LayoutBox` sits on
    line 1 (`block_start` = line 1's) because the marker path calls the shared core with **no wrap
    check** (M3); a wrap check would have flushed before or inside the box. Asserts that the box
    has a `LayoutBox`, that its `block_start` is line 1's, and `lines.len() == 1`. PR-1c because
    the box's rect is M4's: at PR-1b the box has no rect, so the cell's own assertion has no
    channel there (round 22, Axis 2; gate).
    ⚠ **The box is the fixture's last item, and that is what makes the cell invariant-clean**:
    with no content after it nothing can reach the wrap guard, so the realised break set is empty
    at every `W` in the window, matching a licensed set that is empty because `aaaa` carries no
    UAX #14 opportunity at all — the property holds by the text rather than by a width. The two
    fixtures this replaces are ledger **A18** (a trailing `b`, which the markers' own advance
    pushes past `W` throughout the window) and **A19** (that `b` behind a space, which css-text-3
    §5.5 leaves undetermined for a characterless box).
    ⚠ **The upper bound is what makes the cell discriminate**: a wrap check at a marker fires
    only while the box's edges would overrun `W` — at the start marker below
    `measure_width("aaaa") + 20`, at the end marker below `measure_width("aaaa") + 40` — so the
    `+ 40` bound is "some marker's check fires", and the lower bound says only that `aaaa` itself
    fits. Under such a check the flushed second line would carry the markers alone,
    `contributes_content` false (M3 passes the constant at PR-1b), so it would be discarded with
    its tentative rects: what turns the cell red is the box having **no `LayoutBox` at all**,
    which is why presence is asserted beside `block_start`. No literal `W`: the test computes
    both bounds from `measure_width`, as 15d's does.
17. **A decorated inline whose content wraps** —
    `<p style="width:80px">aaa <span style="padding:10px">bbb ccc</span></p>`: start marker on
    line N, end marker on line N+1; M4's per-line rebase must yield one rect per line, each with
    that line's `block_start`. ⚠ **It is 17d(b)'s markup deliberately**, so that the three PR-1c
    cells on a wrapping box read three channels off **one** geometry — `line_rects` here,
    `getClientRects` at 17d(b), `getBoundingClientRect` at 17f — and the width condition is stated
    once, at 17d(b), as a relation over `measure_width`. `line_count` is **2 both today and
    after PR-1b**, so §5.3's PR-1b universal is untouched. ⚠ **The wrap is genuine under the
    preamble's control, measured**: tagged the lines are `"aaa "`+`"bbb "` / `"ccc"`, and with the
    span's tags removed (`<p style="width:80px">aaa bbb ccc</p>`) they are `"aaa bbb "` / `"ccc"`
    — the **same break point**, `find_break_opportunities`' split of one run (`pack/items.rs:74-83`),
    which is cell 15d's ground too, and not the inline-box boundary. The cell carried no markup
    at all until Codex R3 (freeze amendment).
10b. **Nested boxes, stack depth > 1** (M4) — `<p>a<span style="padding:5px">b<span
    style="padding:5px">c</span>d</span>e</p>`: each span gets its own rect and its own
    `LayoutBox` edges, and the inner box's **border box** lies within the outer's content rect on
    the **inline** axis (block extents are the line's, cell 13b).
    This is why M4 is a **stack** rather than one open slot. ⚠ **The containment is class-wide
    over M1's emit set, not a fact about this cell's all-positive edges** (rev 60; ledger **A60**):
    M4's outcome makes such a box's inline-axis content extent a hull that includes every
    descendant's inline-axis border-box extent, whatever the sign of its edges; the cell asserts
    containment on that axis only, since a 2-D `contains` fails under a correct producer. **Cells 10c and 10d are the ones that discriminate** — a
    negative descendant advance, where the rev-59 cursor span does not contain the descendant.
10c. **Nested negative edges: the outer box's rect encloses the inner's border box on the inline
    axis and is never inverted** (M4; R22-c; rev 60 freeze amendment, ledger **A60**) — `<p>a<span
    style="padding-top:1px"><span style="margin-left:-20px;padding-left:1px"></span></span>b</p>`.
    Write `X` for the cursor after `a` (the harness's `measure_width("a")`, `tests/mod.rs:31`). The
    outer span's own inline-axis edges are zero and it places nothing of its own — the `a` and `b`
    runs carry the `<p>`'s entity — so the only rendered content within it is the inner span. The
    inner span places nothing either, so its content rect is M4's **base case**: zero-width at its
    content-start, which M3's **signed** advance puts at `X − 20 + 1`. **Asserted**, as relations
    on the produced `LayoutBox`es, inline axis only (the outer's `padding-top` is the block-axis
    edge that gets it a marker, M1): (i) the inner span's `content` has inline size **0** and starts
    at `X + margin-left + padding-left`; (ii) its `border_box()` starts at `X + margin-left` and has
    inline size `padding-left` (border zero); (iii) the outer span's `content` is the hull of its
    content-start `X`, its end-cursor `X + margin-left + padding-left` and (ii) — it starts at
    `X + margin-left`, ends at `X`, has inline size `−margin-left` (non-negative), and contains
    both `X` and the inner's border box; (iv) `b` starts at `X + margin-left + padding-left`, M3's
    advance, which this cell does not move. ⚠ **What turns it red** is the rev-59 construction: the outer's
    `InlineBoxEnd` entry spans content-start `X` → end-cursor `X − 19`, `inline_end <
    inline_start`, and nothing downstream normalises it — `commit_aligned_entity_rects`' fold takes
    `min`/`max` only where an entity already has an entry on the line (`pack/mod.rs:481-486`) and
    pushes a sole entry verbatim (`:485`), `or_insert` copies it (`:505-511`),
    `assign_inline_layout_boxes` writes `inline_end - inline_start` with no clamp
    (`pack/boxes.rs:76`, `:81`) and `Rect::new` stores it as given (`layout_types/rect.rs:141-146`).
    Where the fix lives is PR-1c's. `LayoutBox`'s doc contemplating a negative extent (`layout_types/boxes.rs:146`)
    speaks of `margin_box()`, derived at the read, and is untouched. The readers of the well-formed
    rect are the `border_box()` family §5.3 enumerates — `getBoundingClientRect` / `offsetLeft`,
    hit-testing (`hit_test.rs:165`'s `bb.contains`), a11y node bounds, a relatively positioned
    inline's painted rect (cell 13d) — and `ResizeObserver`'s content size (§7). The construction
    is an **ordering** plus the markup's own edge values, so it does not turn on the harness's font
    beyond `X`, unlike 17c's.
    ⚠ **Not cell 3b's case, and not cell 3's negative arm.** 3b's pair cancels on **each** side,
    so the cursor returns to where it started and no rect inverts; cell 3's `margin:-10px` arm is
    a single box whose own start edge is consumed *before* M4 saves content-start, so its span is
    zero-width rather than inverted — both are M4's base case. Depth > 1 is **one** way to invert
    the rev-59 cursor span, not the only one: an atomic inline with a negative margin inverts it
    at depth 1 — cell 10d.
10d. **An atomic with a negative margin, depth 1** (M4; rev 60, ledger **A60**) — `<p>a<span
    style="padding-left:1px"><span style="display:inline-block;width:10px;padding-left:2px;border-left:3px solid;margin-left:-20px;position:relative;left:7px"></span></span>b</p>`.
    Write `X` for the cursor after `a`, `C = X + 1` for the span's content-start, and for the
    atomic `m = margin-left = −20`, `bw = 3 + 2 + 10 = 15` (border-box inline size) and `r = 7` (its
    relative offset). The atomic advances the cursor by its margin box, `m + bw = −5`, so the span's
    end-cursor is `C − 5`, behind `C`: the rev-59 cursor span inverts with no nested inline box.
    The atomic's border box **as placed on the line** is `[C + m, C + m + bw] = [C − 20, C − 5]`.
    **Asserted**, inline axis, as relations the test computes from those values: (i) the span's
    `content` is the hull of `C`, `C − 5` and `[C − 20, C − 5]` — start `C + m`, end `C`, inline
    size `−m = 20`; (ii) `b` starts at `C + m + bw`, M3's advance, unmoved; (iii) the atomic's own
    `LayoutBox` border box, after `reposition_atomic_box`, is the placed one shifted by `r`. **Three
    wrong producers no other cell rejects are rejected here**: hulling the atomic's **content**
    box (`[C − 15, C − 5]`) gives start `C − 15` and size 15; hulling its **relpos-offset** box
    (`[C − 13, C + 2]`) gives `[C − 13, C + 2]`, size 15 — and hulling the margin-box advance
    alone gives `[C − 5, C]`, size 5. ⚠ The atomic's `padding`, `border` and `position` are set on
    its `ComputedStyle` directly, as cell 2's ⚠ establishes for border.
17b. **Two producers, one fragment** (M4's ⚠): a single-line `<span style="padding:10px">text</span>`
    must yield exactly **one** `line_rects` entry — the persisting arm's per-entity fold
    (`commit_aligned_entity_rects`, `:479-487`) absorbing `place_item`'s rect and the marker's.
17c. **A box opened at a soft wrap has no rect on the line before it** (M4; rev 60, ledger
    **A60**) — `<p style="width:60px">aaaa <span style="padding:5px">cccc</span></p>`.
    css-text-3 §5.5 puts a soft wrap before a box's first character at the box's **margin edge**,
    so the whole box, start edge included, belongs on line N+1. elidex does not honour that: M3
    advances the 5px start edge on line N and the box's first content opens line N+1 — the
    **registered deviation** §3's css-text-3 §5.5 *adjacent soft wrap opportunity* row marks ✗
    and routes to `#11-inline-box-decoration-splits`, and not a requirement of this cell. The
    shape, as a relation the test computes with the harness's `measure_width` (`tests/mod.rs:31`):
    `measure_width("aaaa ") + 5 ≤ 60`, so the start marker is placed after `"aaaa "` and stays on
    line N, while `measure_width("aaaa ") + 5 + measure_width("cccc") > 60`, so `"cccc"` opens line
    N+1. **Asserted — what this program delivers**: (i) **no rect on line N**, which is
    spec-correct (the box has no fragment there); (ii) **one rect on line N+1**, starting at line
    N+1's content-start **as M3 places it** — 0, the start edge having been consumed on line N —
    with inline size `measure_width("cccc")`. The css-text-3 §5.5-conformant rect would start at the 5px
    start edge, `[5, 5 + measure_width("cccc")]`; **elidex's value is asserted as an accepted
    divergence**, in the shape 17d(b) uses, and routed to `#11-inline-box-decoration-splits`.
    ⚠ **(ii) still discriminates the rebase under M4's outcome**: the hull is never inverted, so
    "not inverted" tests nothing, but a content-start carried over from line N un-rebased is a
    point that **enters the hull** and widens the rect. The stale point is
    `measure_width("aaaa") + 5` if line N's trailing space is removed and
    `measure_width("aaaa ") + 5` if it hangs — the end-of-line white-space prereq's plan decides
    which — so the **precondition the test asserts**, with the harness's `measure_width` like the
    two relations above, is the weaker of the two: `measure_width("aaaa") + 5 >
    measure_width("cccc")`, which implies the other. So the cell asserts the extent,
    not the orientation. `line_count` is **2 before and after PR-1b**: the 5px start edge does
    not move the break, so §5.3's PR-1b universal is untouched. ⚠ **The second line is a genuine
    opportunity, by the preamble's tagless control, measured**: today the markup lays out
    `"aaaa "` / `"cccc"`, and the tags-removed `<p style="width:60px">aaaa cccc</p>` lays out the
    **same split**, so the break is UAX #14's at the space. The fixture this cell carried until
    Codex R3 failed that control (freeze amendment).
17d. **`getClientRects`, both channels, one asserting a divergence** (M4's ⚠). Two markups:
    (a) **single fragment** — `<span style="padding:10px">text</span>` on one line stores no
    `InlineClientRects` (`boxes.rs:102`'s `len() > 1` guard), so `getClientRects` takes the
    `border_box()` fallback (`element/layout_query.rs:237`) and returns a **border area** —
    padding + border, never margin, per cssom-view-1 §6 step 3. Correct after PR-1c, and this is
    the cell that pins it. (b) **multiple fragments** —
    `<p style="width:80px">aaa <span style="padding:10px">bbb ccc</span></p>` returns per-line
    **content** spans, i.e. edges missing. The width condition, as a relation the test computes
    with the harness's `measure_width` (`tests/mod.rs:31`) rather than from literals — this is the
    one site that states it, and cells 17 and 17f read it from here. Write `S` for the span's
    content-start, `measure_width("aaa ") + 10`. `place_item`'s soft-wrap guard tests the
    **trimmed** width (`pack/mod.rs:690`,
    `current_inline + trimmed_width > containing_inline_size`) and then advances the cursor by the
    **full** width (`:695`), so line 1 fits `"bbb"` iff `S + measure_width("bbb") ≤ 80` and then
    leaves the cursor at `S + measure_width("bbb ")`, from which `"ccc"` overruns iff
    `S + measure_width("bbb ") + measure_width("ccc") > 80`. Both hold at 80, so the box owns
    **two** `line_rects` — ⚠ an earlier drafting stated the fit test with the *untrimmed* width,
    which is the cursor position and not the quantity the guard compares; the figures it gave were
    correct and the two halves of the cell nonetheless named different tests (round-24 gate), the
    reason the condition is now written as the guard's own inequality —
    which is what makes this the multi-fragment channel. **Asserted as an accepted divergence**,
    in the shape cell 23 uses, and routed to `#11-inline-box-decoration-splits` (§5.3 gives the two
    reasons PR-1c cannot discharge it). The two markups therefore disagree after PR-1c, and the
    cell records that rather than hiding it. ⚠ An earlier drafting wrote `width:60px`, on which the
    cell constructs a **different** case: the first inequality fails, so the
    box opens exactly at the break, line 1 contributes no rect — correctly, css-text-3 §5.5 putting
    that break at the box's margin edge — `line_rects.len() == 1`, and `getClientRects` takes
    17d(a)'s `border_box()` fallback: cell 17c's phenomenon, not this one's (round 24 audit).
    ⚠ **`17e` is a withdrawn label, not a lost cell** (R22, which found the gap unrecorded while
    `24f`'s withdrawal is recorded in the freeze prose, the amendment disclaimer and the ledger
    row **A30**): it stood through rev 14 as *The
    stale-`InlineClientRects` arm*, pinning the `else` of `boxes.rs:102`'s `len() > 1` branch, and
    rev 15 removed it in the edit that rewrote 17d into its present two-markup form
    (`git show 8da1807a:<memo> | grep -nE '^17[a-z]?\. '` lists it; `ecc2c137` does not). That is
    **before** the rev-34 freeze baseline the amendment record above counts from, which is why no
    amendment line carries it and why the letter was never reused.
17f. **The multi-line `getBoundingClientRect`, pinned as accepted** (§7; cssom-view-1 §6
    *get the bounding box*) — on cell 17's markup,
    `<p style="width:80px">aaa <span style="padding:10px">bbb ccc</span></p>`, the shared
    two-fragment geometry (17d(b), named as this cell's other half at its close, is the third
    channel on it): the wrapping `<span style="padding:10px">` must return the min/max
    **union of its per-line content spans expanded by the padding on all four sides**, which is
    what `LayoutBox.border_box()` is. The two fragments and the genuineness of the break that
    produces them are measured at cell 17 under the §6 preamble's tagless control, and
    `line_count` is 2 both today and after PR-1b. This cell carried no markup at all until Codex
    R3 (freeze amendment). Two divergences from the spec's derivation, both recorded:
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
17g. **A forced break inside a box whose own content has no run across it** (M4; A55's
    inner-element shape; rev 60, ledger **A60**) — `<pre><span style="padding-left:10px"><span>\nx</span></span></pre>`.
    The preserved `"\n"` is a *forced line break* (css-text-3 §5, `dfn css-text-3 'forced line
    break'`; §5.5: "Preserved segment breaks … must be treated as forced line breaks") inside the
    outer span, so css-inline-3
    §2.1 splits the box into a fragment per line (css-break-3 §2 *box fragment*); every run inside
    it is keyed to the **inner**, undecorated span, so the outer owns no run on either line.
    **Asserted**: the outer span's `line_rects.len() == 2`; line 1's rect starts at the outer's
    content-start, `padding-left`; line 2's starts at 0 and encloses the inner span's `x` on the inline axis. The cell
    asserts stored `line_rects` as **content** spans and a count — the same exposure cell 17
    already has; which broken edge keeps its padding, and the border areas `getClientRects`
    reports for more than one fragment, stay `#11-inline-box-decoration-splits`'s (cell 17d(b)).
    How the producer meets line 1 is PR-1c's.

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
    reaches `force_break` for it — the end-of-text break is filtered out **inside**
    `find_break_opportunities` itself (`crates/text/elidex-linebreak/src/lib.rs:28-33`, the
    `offset >= text.len()` return at `:31`; the packer's call is `pack/items.rs:72`, stated at
    `pack/mod.rs:547-551`), and the line
    is kept by **clause 2** instead, through `contributes_content = !text.is_empty()` under `Pre`.
    That is the same predicate §5.1 M7 now names as raising the `RenderedText` rung, so the two
    statements agree. ⚠ **The filter was credited to `pack/items.rs:74` until R22**: the
    packer *calls* the finder at `:72` and splits the run on what comes back at `:74-83`, while the
    filter is a `filter_map` arm inside the finder, in another crate — which
    `#11-inline-open-box-strut-on-continuation-line` (§5.3) already cites correctly, so the memo
    carried one fact at two sites and one of them in the wrong crate. ⚠ **The `<br>` shape is NOT constructible and no cell asserts it**: `<br>`
    carries no break behaviour anywhere in the engine (§9's disposition — a missing feature, not a
    divergence), so there is no `<br>`-adjacent forced break for a marker to sit beside. A
    decorated `<br>` does still pass M1's four filters and take a marker pair; what it cannot do
    is break. When forced-break elements are implemented, this cell gains that shape.
20. `display: none` ⇒ no marker (`inline/collect.rs:218`); `position: absolute` ⇒ out of flow,
    clause 3 does not apply.
    ⚠ **The abspos arm also touches css-text-3 §5.5, on the half elidex gets right** (R5): that
    sentence opens "**Out-of-flow boxes** and inline box boundaries do not introduce a forced line
    break or soft wrap opportunity", and the abspos child does enter the item stream as an
    `InlineItem::Placeholder` (`inline/collect.rs:225`; ⚠ **`:230` until R22 at both sites**, which
    is that push's line at `22de3078` and not at the memo's `154bac3f` frame — the in-sentence
    sibling `:218` above is a `154bac3f` coordinate, so one enumeration carried two frames).
    Measured at `22de3078`, `pack()`'s
    `PackItem::Placeholder` arm (`pack/mod.rs:635-642`) records a static position and returns
    **without calling `place_item`**, so the per-item flush `#11-inline-item-boundary-soft-wrap`
    (§5.3) records cannot fire at a placeholder — the out-of-flow half holds by construction where
    the inline-box half does not. Recorded so the cell is not read as silent about a sentence it
    sits under; no assertion turns on it.
21. **Co-resident entity on a newly-existing line** — css-inline-3 §2.3's "and its in-flow
    content" means that when the line exists, every entity that has a tentative rect on it commits
    (M5). ⚠ **The co-resident must be one the engine can actually give a rect**, and on a line that
    was phantom that is a narrow set: a collapsed-away whitespace run never reaches `place_item`
    (`pack/items.rs:69` skips an empty run), and a box with **all** edges zero gets no marker from
    M1, so it has no rect either. **One case is producible**: a **block-axis-only** decorated box —
    `<p><span style="padding-left:10px"></span><span style="padding-top:5px"></span></p>`, where
    the second span is undecorated *in the inline axis* yet has a marker and so a rect. That
    entity's rect is its **first-ever `LayoutBox`**. This is PR-1d's half of the presence change
    §7 states; PR-1c's half (cell 14c) reaches only lines that already existed.
    ⚠ **A second arm stood here until R5 and is withdrawn to
    `#11-inline-item-boundary-soft-wrap`** (§5.3; ledger A13) — a collapsible space that survived
    collapse, on
    `<p style="width:1px">aaaa<span> </span><span style="padding-left:10px"></span></p>`. It is
    withdrawn and **not** re-fixtured, because measurement says it cannot be: the wrap it needs is
    not one css-text-3 §5.5 licenses (`find_break_opportunities("aaaa")` and `("aaaa ")` are both
    empty, so no width yields an opportunity; what puts the space on a second line is
    `place_item`'s per-item flush at the item boundary), and no licensed break can open a line
    with a collapsible space on it, a UAX #14 opportunity lying inside a run and leaving the space
    at the **end** of the preceding segment. Measured at `22de3078` on the withdrawn markup:
    `line_count` **1** at every width tried (1, 10, 40, 200), and the space span has **no
    `LayoutBox`** at the narrow ones — the whitespace-only second line is flushed and then
    discarded (`any_rendered_content == false`), which is why it never read as a second line and
    why three sweeps passed over it.
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
    5px line **after PR-1d** (`line_count` 0 today, Shape B — the empty span emits no item, so
    `inline/mod.rs:162`'s `items.is_empty()` early return fires at `22de3078`), not the spec's 40px,
    because the block container's root inline box is
    unimplemented (`#11-inline-root-inline-box`, facet (a)). Asserted so the divergence stays
    distinguishable from a bug.
    ⚠ **"Yields a 5px line", present tense, until R22**, where the markup has no oracle at the
    frame at all — the convention cells 24d and 12d already use is the one above. ⚠ **And the
    tense fix is not a doubt about the value**: §5.1 M6's Grounds runs the *same* shape with text,
    `<p style="line-height:40px"><span style="line-height:5px">x</span></p>`, which does yield
    5px today with no marker involved; this cell's contribution is that the decoration-only
    version reaches the same place once a line exists to reach it.
23b. **The vertical facet of the same divergence, pinned as accepted** (M6; Codex R2-F2):
    `<p style="writing-mode:vertical-rl"><span
    style="padding-top:1px;font-size:10px;line-height:100px"></span></p>` yields a **10px** line
    **after PR-1d** (⚠ **present tense until R22**; `line_count` is 0 today for the same reason
    cell 23 gives — one empty span, no items, `inline/mod.rs:162`),
    not the 100px css-inline-3 §2.2's Note requires ("Empty inline boxes still have margins,
    padding, borders, and a **line-height**, and thus influence these calculations just like
    boxes with content") and css-inline-3 §5.3 computes, that section applying in vertical writing modes
    as in horizontal ones. M6 feeds the marker's `block_advance` from the packer's existing
    vertical convention — `if is_vertical { font_size } else { line_height }`
    (`inline/pack/mod.rs:539-543`) — so the box's `line-height` never reaches
    `current_line_height = max(block_advance)` at all. `padding-top` is the **inline-start** edge
    in `vertical-rl` (cell 12c), which is what gives this line its clause-3 existence;
    `writing-mode` is on the `<p>` and merely inherited, so the span does not become the atomic
    cell 12c warns about. Facet (c) of `#11-inline-root-inline-box` (§5.3).
    ⚠ **Not to be "fixed" by giving the marker `line_height` in the vertical arm**: `block_advance`
    feeds one shared `max()`, so a marker reading `line_height` beside co-resident text reading
    `font_size` on the same vertical line would trade a disclosed divergence for an undisclosed
    incoherence — M6's ground that **both** arms of the convention are payload reads stands.
    ⚠ **What discriminates**: swap the marker's vertical arm to `line_height` and the assertion
    goes 10 → 100. Cell 23 cannot catch that — it is horizontal, where the convention already
    reads `line_height` — so the two arms of one convention need one cell each.
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
    ⚠ **The generic-keyword mapping is a fixture constraint here and nothing more** (R15): that
    `sans-serif` reaches one literal family is an `elidex-shaping` / `fontdb`-configuration
    matter, **explicitly no-slot and out of scope** (§9's unrouted-gap sweep names the owner), and
    it is recorded in this cell only because it is why the fixture spells `TEST_FAMILIES` out.
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
24c. **A glyphless decorated box beside differently-sized text: the strut does not compose —
    pinned as accepted** (M7; Codex R2-F4): `<p><span
    style="padding:1px;font-size:100px;line-height:100px"></span>x</p>` — the IFC's
    `first_baseline` is the one the `x` run sets at `inline/pack/mod.rs:586`, its own half-leading
    plus ascent at the inherited 16px, and **not** the far larger `A′` the 100px box's strut would
    contribute under css-inline-3 §5.3, which gives the strut **per glyphless box** and composes
    the line from every box's aligned layout bounds (css-inline-3 §2.2 step 3). elidex keeps a
    scalar `current_line_height = max(…)` and a first-wins `first_baseline`, so M6's `max` can
    raise the line's *height* but cannot *move* its baseline. Facet (d) of
    `#11-inline-root-inline-box` (§5.3).
    ⚠ **The baseline is the observable that discriminates and the height is not**, which is why
    the cell asserts only the first: css-inline-3 §5.3 makes a box's layout bounds `A′ + D′ = line-height`
    exactly, so the span's bounds are 100 and the aligned composition gives the same 100 that
    M6's `max` gives — the two models coincide on the height for this markup and part by the whole
    distance between a 16px run's ascent and a 100px box's `A′` on the baseline.
    ⚠ **It does not exercise M7's `RenderedText` rung and must not be read as doing so**: `x`'s
    family resolves, so `first_baseline.is_none()` is already false at flush and the promote is
    blocked before the occupancy state is consulted. Cell 24 is where the rung is load-bearing;
    this cell pins the divergence that the rung's correctness argument concedes.
24d. **A decoration-only line with *two* glyphless boxes: neither the height nor the baseline
    composes — pinned as accepted** (M7/M6; R4): `<p><span
    style="padding:1px;font-size:100px;line-height:10px"></span><span
    style="padding:1px;font-size:10px;line-height:10px"></span></p>` — css-text-3 §5.5 gives the
    box boundary no soft wrap opportunity, so **one** line with two markers' worth of glyphless
    boxes (`line_count` 1 after PR-1d; 0 today, Shape B twice). Asserts, against the test font's
    own metrics read in the test rather than against constants (the `measure_width`
    convention, `inline/tests/mod.rs:31` at `22de3078`): the line's height is the scalar
    `max(line-height)` over the two spans, and the IFC's `first_baseline` is **one** box's strut
    ascent, where css-inline-3 §5.3 gives each box `A′ = A + L/2` / `D′ = D + L/2` and
    css-inline-3 §2.2 **step 3** sizes the line to enclose the aligned bounds of **all** its
    inline-level boxes — the first span supplying the maximal `A′` and the second a `D′` the first
    cannot (its 10px `line-height` against a 100px font makes its half-leading large and negative,
    so its own `D′` is below zero), so the composition falls on no single box and exceeds the
    scalar `max`. ⚠ **And the composition is over *three* boxes, not two** (R5): the `<p>`
    generates a **root inline box** (§1.1), css-inline-3 §2.2 **step 2** gives it its own bullet,
    and glyphless at the inherited font size with `line-height: normal` it contributes the
    a `D′` deeper than either span's. The cell asserts elidex's two values and the **relation**
    the spec asks for — maximal `A′` and maximal `D′` on different boxes, their sum beyond the
    scalar — and **no composed figure**, because css-inline-3 §5.3 leaves the root inline box's
    half-leading optional ("the font's line gap metric **may** also be incorporated") and a
    conforming UA therefore has a range, not a number. Facet (e) of `#11-inline-root-inline-box` (§5.3).
    ⚠ **Why two boxes and not one**: on a single-box decoration-only line the scalar `max` and
    the single tentative *are* the composition, which is why exactness was once claimed there
    (ledger A9). The cell exists to hold that claim withdrawn, and it discriminates only because the
    two boxes' extremes fall on different boxes — a pair whose `A′`/`D′` both peaked on one box
    would pass under either model (the shape cell 13b's own ⚠ warns about).
    ⚠ **The one-line premise has no oracle before PR-1d, and is the likeliest next instance of
    the boundary-wrap class** (R5): today the markup is Shape B twice, `items.is_empty()` fires
    and `line_count` is 0, so there is nothing to measure "one line" against. The ground is by
    construction (M3's marker path runs the shared core with **no** soft-wrap check), and by
    construction is what that class keeps falsifying; §8's suite-level break invariant is what
    decides it once the cell is constructible.

24e. **An open box's metrics on a *continuation* line** (M4's flush replay, M6's value, M7's
    tentative; R7-a) — `<p style="width:W"><span style="padding-left:1px;line-height:100px"><span
    style="line-height:10px">aaa bbb</span></span></p>` with
    **`1 + measure_width("aaa") ≤ W < measure_width("aaa ") + measure_width("bbb")`**, cell 15d's
    shape, `measure_width` the harness's (`inline/tests/mod.rs:31`). The wrap is the **space**'s —
    the only UAX #14 opportunity the text has — so this cell meets §6's standing rule by its text
    and not by a width, and its realised break offset is one `find_break_opportunities("aaa bbb")`
    returns. Only the **outer** span gets a marker (the inner has no edge, M1); the inner's
    `line-height` is what the two text runs carry. Asserts `lines.len() == 2` and that the two
    lines' `InlineFlowLine.block_size` are **equal**, at the outer box's used `line-height` — both
    read off the produced lines, so no figure is written here.
    ⚠ **What discriminates**: without the replay line 2 collapses to the inner 10px while line 1
    keeps the outer 100px, so the cell fails on an **inequality between two values it computes**
    rather than against a constant — and a fixture whose two `line-height`s were equal would pass
    under either mechanism, the shape cell 13b's own ⚠ warns about.
    ⚠ **The upper bound drops the marker's 1px deliberately**, exactly as 15d's drops its 20:
    it pins `W` below today's own wrap point, so `line_count` is **2 before and after PR-1b** and
    §5.3's PR-1b universal ("no cell pins a `line_count` change") is untouched. The window is
    non-empty iff `1 < measure_width(" ") + measure_width("bbb")`, which the test evaluates rather
    than this memo.
    ⚠ **The baseline half of the replay is not observable on this fixture — and, since R10, not
    on any fixture this memo's own rules allow, which is why this cell is the whole of the replay
    PR-1d mandates**: `first_baseline` is per-**IFC** and first-wins (`inline/pack/mod.rs:125`,
    `:586`), line 1's text resolves a font, so it is already set before line 2 exists and M7's
    tentative could not promote under either mechanism. Separating the halves needs a box open
    across a break whose continuation line carries **no** rendered text, and **no licensed break
    produces one**: the continuation line's first member is always the tail segment of the same
    run, and that tail always contributes (R10, measured — §5.3's
    `#11-inline-open-box-strut-on-continuation-line` carries the sweep and the argument, §8's
    PR-1d item the DoD change). **24f**, the cell R9 added to pin the baseline half on a
    `white-space: pre-line` forced break, is **withdrawn**: its own fixture does not reach the
    state and no other markup does either. This cell pins the replay's height channel, which is
    the whole of what M4's hook replays. Ledger **A28**, **A30**.
    ⚠ **The two-line premise has no oracle before PR-1d**, cell 24d's limitation on its own
    one-line premise: the tagged markup cannot be laid out until the markers exist, so the ground
    is by construction — the space is a licensed opportunity in **both** regimes — and §8's
    suite-level break invariant is what decides it, the treatment that class already gets.

**Non-regression throughout**: `collapsible_whitespace_only_generates_no_line_box`
(`crates/layout/elidex-layout-block/src/inline/tests/text_height/basic.rs:204`) and
`nbsp_only_line_generates_a_box` (`:233`).

**Test placement** (§9): PR-1a's cells land in `inline/tests/decorated_inline/stream.rs`, PR-1b's
in `…/advance.rs`, PR-1c's in `…/geometry.rs`, PR-1d's in `…/existence.rs` — a new
`decorated_inline` module split per PR
from the start, per [[feedback_touch-time-split-means-while-writing]], rather than one file grown
to hold every cell. None of them go in `tests/text_height/basic.rs` or `relpos_subflow.rs`. ⚠ **Cells 10 and 11 have no relpos
variant**, and nothing here needs one: the memo's only relpos content **in the `group_key` sense**
is the sub-flow keying, which §2's pair 2×6 and §5.3 route out to
`#11-inline-box-decoration-splits`, so that question is that slot's. An earlier drafting said "the
relpos facet of cells 10/11 lands with the new module too", naming a facet no cell has (round 24
audit). ⚠ **Cell 13d is not a counter-example to that sentence** (R11; an `elidex-shell` test since rev 66): it is a *paint-geometry*
relpos cell — `position:relative` is what routes the box to `walk`, hence to `border_box()`, hence
to a painted rect at all — and it says nothing about sub-flow keying, which is the facet the
sentence is about. Two different questions share the word; the sentence stands for its own.
⚠ **Every cell is wholly in that module from R4 on, with exactly one bounded exception, and the
exception is named rather than blanket** (R11). Cell 13 used to carry a third, *painted*
assertion that only `elidex-render` could observe, and this paragraph carried the exception for it;
that assertion is withdrawn (freeze amendment, `#11-inline-decoration-paint-path`). The exception
that returns is **13d** and only 13d: it asserts a painted rect this program **moves**, so it
lands in `elidex-render`'s own `builder/tests/inline_flow/relpos.rs`, beside the
`consumes_relpos_inline_subflow_with_gap` harness it reuses — `elidex-layout-block` can observe no
display list, and a layout-side cell could only re-assert cell 13c's edges. §5.2's
`elidex-render` row stays negative for every other cell: 13d reads that crate's existing output
and adds no pass or member there, which is the line ledger **A6** draws.

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
    fragment owning an extreme may carry no edge there. Cell 17f pins it, and every `border_box()`
    consumer below inherits the same box.
  * **One line, more than one fragment** — the case css-inline-3 §2.1's Note names ("split into
    several fragments within the same line box due to bidirectional text processing"). It falls in
    the *first* bullet by line count, and returns one rect where the step asks for one per
    fragment. ⚠ **This is pre-existing and this program does not touch it**: the count is already
    wrong on `154bac3f`, and layout cannot represent the fragments at all. css-break-3 §5.4 asks
    for `box-decoration-break` at "bidi-imposed breaks" too, so it lands with the same owner.

  All three residues route to `#11-inline-box-decoration-splits`; they are one missing per-fragment
  attribution seen from three sides, not three defects.

  PR-1d adds the co-resident commit (cell 21).
* **Painted output — unchanged by PR-1c for a *static* inline (R4 withdraws the claim that it was
  the program's largest delta); for a `position:relative` one the *geometry* changes — and the
  count with it once the span carries a `border` — which R11 measured and R4's count could not
  see.** A static
  inline's background and border are not emitted today and are not
  emitted after PR-1c: `paint_non_sc` (`builder/walk.rs:638`, loop `:648-676`) routes a
  non-positioned inline child to `inline_run`, never to `walk`, and `emit_background` /
  `emit_borders` have no other non-test caller than `walk.rs:356` / `:366`
  (`grep -rn "emit_background(\|emit_borders(" crates/core/elidex-render/src/ | grep -v "fn emit_"`);
  `InlineFlowRun` carries only `Text` and `AtomicBox`. Measured on the post-PR-1c shape: 0 red
  rects for `display:inline`, 1 for `display:block`, 1 for `position:relative` (§5.2's
  `elidex-render` row states the construction). That gap is **pre-existing and engine-wide** —
  `#11-inline-decoration-paint-path` (§5.3) — and this program neither creates nor closes it,
  and `display:inline` stays at 0 with the edges real (measured: 0 red rects both before and
  after, on the probe below with `position: static`).
  ⚠⚠ **But the `position:relative` arm is not unchanged, and a *count* cannot say so** (R11).
  A positioned inline is skipped by `paint_non_sc` (`walk.rs:650`, "painted by the parent SC")
  and by Layer 5 (`:583`), and painted in Layer 6/7 (`:614-629`) through
  `walk_child_with_fixed_check` → `walk` (`:728`), which emits background and borders from a
  `&dyn BoxModel`; both `emit_background` (`builder/paint/mod.rs:68`) and `emit_borders` (`:382`)
  read `lb.border_box()`, which is `content.expand(padding).expand(border)`
  (`layout_types/boxes.rs:140-142`). M4's whole point at PR-1c is that the inline's `LayoutBox`
  carries **three real `EdgeSizes`** instead of `EdgeSizes::default()` (§5.1 M4; §5.2's
  `pack/boxes.rs` row; §6 cell 6's "M4 must fill all four `LayoutBox` sides"),
  and `assign_inline_layout_boxes` iterates `entity_bounds` (`boxes.rs:56`) — so the box
  `border_box()` reads is exactly the one PR-1c changes. Measured, not argued: a probe appended to
  `crates/core/elidex-render/src/builder/tests/inline_flow/relpos.rs` (harness copied from
  `consumes_relpos_inline_subflow_with_gap`, which sets `LayoutBox` directly and so isolates
  render from layout), one `position:relative` span with `background: red`, run and reverted —
  today, padding and border zeroed on the inline `LayoutBox`, gives `SolidRect (24.0, 0.0)
  16.0 x 20.0`; with padding 10 and border 2 on all four sides it gives
  `SolidRect (12.0, -12.0) 40.0 x 44.0`. **Same count (1 → 1), different rect** — and that
  invariant count is an artefact of the *background-only* markup: give the same span a
  `border: 2px solid` and the count moves too, 1 → **5**, because `emit_borders` takes each
  side's thickness from `LayoutBox.border`, which is zero for an inline today, so the border
  paints nothing until M4 fills the box (measured on the same probe; §6 cell **13d** carries all
  four rects). The inflation is
  single and correct, so the new rect is the *right* one — it is simply **unasserted** until §6
  cell **13d**. **PR-1c therefore does change painted output on a supported path**, and the
  universal that said otherwise was supported by a count on an arm where only the geometry
  discriminates — the same shape as ledger **A5**'s control that compared tagless *line counts*
  where only the split **point** separates the cases. Ledger **A8** is narrowed to the static
  subset and **A33** records this.
  What *does* change in painted output is **PR-1b's**, not PR-1c's: the advance moves the
  `InlineFlowRun::Text` `inline_start` of everything after the box, so the glyphs move. Cells 12b
  and 15 pin that advance as a cursor position at PR-1b; cell 13(b) reads it again at PR-1c.
  ⚠⚠ **That holds on the renderer's identity-order path, and this memo states the scope once,
  here** (R7-b): **every** site of this memo saying the glyphs after the box move states the
  identity half, because on a line whose bidi order is non-identity `builder/inline_flow.rs:154-190`
  (at `22de3078`) discards every run's baked `inline_start`, starts the paint cursor at
  `min(inline_start)` over the line's `Text` runs and advances it by shaped widths alone — so the
  gap the markers open **between** two runs is not painted, and an RTL paragraph with a padded
  inline gets the layout advance and the box geometry with its text runs painted contiguous.
  Measured on that branch: with the two runs' baked positions 100px apart and with the gap closed
  the emitted glyph positions are **identical**, against the crate's own
  `converged_ltr_identity_no_reorder` control, where an identity line paints each run at its own
  baked position. **Disposition: routed, not fixed here** — a renderer mechanism is the crate
  boundary CLAUDE.md's Layering mandate keeps, the same ground A6 gives for the paint path, and
  the branch is **already** the owner of this exact loss, discarding layout's baked justify
  offsets and naming `#11-bidi-full-uba-fidelity` in its own comment at `:160-168`. So the
  intersection goes there (§9), cell 12b carries the pin, and the sites this scope reaches keep a
  pointer rather than a copy. Ledger **A24**.
* `last_placed_entity` / persisted run shape — deliberately changed by M3 (§1.4); cell 14.
* `current_line_last_hang` — stated once, at §5.1 M3 (no marker writes it); cells 16 and 3b. ⚠ **This bullet stated the
  whole-box rule R2 replaced until R5** — "zeroed at a marker with an inline-axis edge", citing
  cell 16, the cell R2 re-fixtured to refute exactly that. §8's detector could not see it: §8
  grepped for the *identifier* and this site names none, so §8's item is now stated by the
  property.
* **Every reader of an inline `LayoutBox`.** **Three** distinct changes, in three different PRs:
  1. ⚠ **Position — PR-1b, and the re-slice's shippability argument does not cover it.** M3's
     advance moves `current_inline`; `place_item` snapshots `seg_inline_start` from it
     (`pack/mod.rs:694`, `:706`) and that reaches `LayoutBox.content` (`pack/boxes.rs:81`). So
     `getBoundingClientRect().x`, `offsetLeft`, hit-test area, a11y node bounds and observer
     geometry **shift** in PR-1b, for every decorated inline with content — and so, through the
     line's `InlineFlowRun::Text` `inline_start`, do the painted glyph positions of everything
     after the box — on the identity-order path, §7's R7-b scope. §5.3 and §8 say PR-1b changes no `border_box()` *extent* — true, and not the
     same claim. PR-1b's DoD therefore carries its own reader disposition, `elidex-render`'s
     existing `border_box()`-reading paint tests included. (A **static** box's own rect is never
     painted — the bullet above; what moves in painted output **at PR-1b** is the glyphs after it.
     Ledger A8, narrowed at R11: a `position:relative` box's rect **is** painted, and PR-1c —
     not PR-1b — moves it, cell 13d. Ledger A33.)
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
     `elidex-api-observers` deliberately does *not* skip a box-less target (`resize.rs:250-253`,
     the comment that says so — ⚠ **`:251-254` until R22**, the whole range one line late: `:250`
     is the comment's first line and `:254` the query loop it sits above —
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
     content rect", **which elidex does not implement — and which R9 routes rather than only
     naming**: the gap is `#11-resize-observer-inline-empty-content-rect`'s (§3's Resize Observer
     row, §9, §10), **pre-existing** class, since the violation is live for every inline that owns
     a run and this program only extends its reach. The size source is
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
     the detector reads two. **PR-1d**: cell 21's co-resident block-axis-only box gains its
     first-ever `LayoutBox` — height 0 → the line's, so the callback fires on a **different**
     element than the one whose inline-axis edge the script added. ⚠ **The example was a
     *surviving collapsible space*, with "a non-zero width too", until R5**; that markup is
     withdrawn from cell 21 (its ⚠) because the second line it needed is the item-boundary flush,
     so the width half of the claim has no fixture and the bullet rests on the height half, which
     the surviving arm gives.
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
  and a `removed` row (an edited or moved one), and M4's own edit to `inline/pack/boxes.rs` — an
  allowlisted `producer` — is the only line this program writes that could produce either; the
  `elidex-render` files are untouched by every PR of this program. ⚠ Two wires, and only one is blind here: **wire #1** (the allowlist) *does* see
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
  * `elidex-render` paint (`paint/mod.rs`), `walk.rs`, `slice.rs`, `transform.rs` — ⚠ **not
    reached for a static inline at all, which R4 establishes and the old entry ("the intended
    correction; cell 13(c) pins the paint half") denied.** These are readers of a *block* box's
    `border_box()`; the walker that feeds them never receives a non-positioned inline child
    (`walk.rs:648-676`), so PR-1c changes nothing they observe. Unchanged-behaviour, not
    correct-after-M4; the gap is `#11-inline-decoration-paint-path`'s, and cell 13 no longer has a
    paint half. A `position:relative` inline *does* reach `walk` and *does* paint its chrome
    (measured, §5.2) — that is the same box PR-1c makes real, so this entry is a *static*-inline
    statement and the positioned case is correct-after-M4 like the rest of the list.
  * `elidex-dom-api` `element/layout_query.rs` — `getBoundingClientRect`, `offsetLeft`/`offsetTop`
    and the offsetParent walk. **JS-observable**, and correct once the edges are real. ⚠ **This
    entry read "needs a cell" until R15 and named none; the to-do is withdrawn rather than
    discharged** (§9's unrouted-gap sweep): `getBoundingClientRect` here is cell 13(a)'s border
    box read through the CSSOM, `offsetLeft`/`offsetTop` and the walk add no observable beyond
    that same box, and the divergence that *is* theirs — cssom-view-1's first-box rule against
    elidex's union — is `#11-inline-box-decoration-splits`'s (§9), not a cell of this program.
  * `elidex-layout/src/hit_test.rs` — a decorated inline's hit area grows by its edges. Correct.
  * `elidex-shell/src/content/scroll.rs` — scrollable-overflow extent and scroll-into-view. Correct.
  * `elidex-a11y/src/tree.rs` — node bounds. Correct.
  * `elidex-js` `intersection_observer.rs` / `resize_observer.rs` — observed box sizes. Correct
    **as geometry**, and JS-observable. ⚠ **Not correct as a `ResizeObserver` content rect, and
    calling the entry "correct" without qualification said it was** (R9-c): resize-observer-1
    §3.3.1 requires an **empty** content rect for a non-replaced inline, `content_rect_local`
    returns the real span, and the violation is live for every inline that owns a run — widened,
    not created, by this program's grant to box-less targets. Owned by
    `#11-resize-observer-inline-empty-content-rect` (§3, §9, §10), **pre-existing** class.
  * `elidex-layout-multicol`, `-table`, `-flex`, `-grid`, `block/mod.rs`, `block/children/stack.rs`,
    `inline/atomic.rs` — block/atomic/abspos paths, unreached for a
    non-replaced inline. `elidex-ecs/src/dom/geometry.rs` likewise, though for a different reason:
    `union_border_boxes` has no non-test consumer yet.
  * ⚠ **`elidex-plugin/src/layout_types/` is where the accessors are *defined*, and this list
    omitted it entirely until R22** — while it holds the grep's **largest** per-file count:
    `git grep -c 'border_box()\|padding_box()\|margin_box()' 154bac3f -- crates/` returns **8**
    for `layout_types/mod.rs` — more than any other file, the next being `builder/slice.rs` and
    `builder/paint/mod.rs` at 5 — and 3 for `layout_types/boxes.rs`. Neither is a reader of a decorated inline's box: `boxes.rs:141`/`:148` are the
    bodies of `border_box()` and `margin_box()` themselves (plus a doc line at `:109`), and all
    eight of `mod.rs`'s are the accessors' own unit tests (`layout_box_padding_box` and its
    siblings), each constructing its own `LayoutBox`. PR-1c changes nothing they observe; the
    entry exists because the list is the discovery grep dispositioned, and a file absent from it
    reads as a file the grep did not return.
  * ⚠ **`elidex-css/src/page.rs` is not a reader and is dropped from this family** — a grep
    artefact, in the class the `elidex-ecs` entry above already flags. Its single hit is
    `page.rs:487` `fn parse_page_margin_box() {`, a `#[cfg(test)]` function **name** the discovery
    grep matched as a substring of `margin_box()`; `elidex-css` owns no layout geometry and has no
    `LayoutBox` read at all. ⚠ **And it is not the only one of its kind** (R22):
    `elidex-style/src/resolve/box_model/tests.rs:96` is `fn resolve_box_sizing_border_box() {`,
    the same name-substring artefact, so the "second way" below has **two** instances and the
    single-instance framing was itself the inventory error it describes. Recorded rather than silently deleted, because it is the second way
    this family list fails as an inventory (the first being the field-read blindness above): the
    grep both under-collects and over-collects, so membership must be derived from what the file
    does, not from the hit (round 25, Axis 1; the two additions above, R22).
  None is wrong-after-M4 **for an inline occupying one line**. For a multi-line one every reader
  in this list inherits the over-inflated union above — that is one divergence with many consumers,
  not many divergences, and cell 17f is its single pin. The point is that PR-1c's DoD must reach
  the whole family and not one member of it, which §8 says.
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
`inline/pack/items.rs:67`, `inline/tests/mod.rs:20-23` — `atomic.rs:39`, `measure.rs:25`/`:54`,
`collect.rs:181` and `pack/mod.rs:534`/`:607` are refutable-pattern destructures (`if let`,
`let … else`) and need none; ⚠ **the two `pack/mod.rs` `let … else` sites were missing from this
exemption list until R22.** The universal the list guards is **true** as stated — the four named
matches are exactly the exhaustive ones over `InlineItem` under
`crates/layout/elidex-layout-block/src/`, so the omission is in the exemption list and not in the
claim); **the payload exactly as M1 specifies it, not
restated here**; `containing_inline_size` threaded through every `collect_inline_items` caller
(§5.2 enumerates them from the grep that defines the set); the two pre-pack gates hold current
behaviour; **no citation work** (all of it is `#11-inline-spec-cite-misattribution`'s, §9); new variants carry docstring citations
to their §3 rows; **two test-harness additions, without which cells 12d, 6b, 6g and 6h are
unconstructible**: `setup_inline_test` (`inline/tests/mod.rs:54`) gains a deterministic way to
force `any_font == false`, and a helper beside `collect_styled_runs` (the `fn`, `:11-25`) returns the
`InlineItem`s rather than `filter_map`ping them to `Vec<StyledRun>` — the existing one discards
every non-`Text` variant, so it cannot observe a marker; cells 1, 2, 5, 6b, 6c, 6f, 6d, 6e, 6i, 6g, 6h, 7–12 and 12d land
here — as characterization tests except 6b, 6i, 6g and 6h, which pin new item-stream behaviour
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

**PR-1b** (inline-axis advance): cells 3, 3b, 3c, 4, 12b, 12c, 12f, 12e, 14, 14b, 15, 15b, 15d,
16, 16b, 25, 25b, 25c and 25d. **It owes two dependencies**: it is not cut from `origin/main` until the
end-of-line white-space and min-content prereqs have landed, in that order (ledger **A69**) —
cells 25 and 25c assert the intrinsic sizes on the min-content prereq's base (the cross-item joining the prerequisite's, **both edge terms this PR's**;
ledger **A58**), and cell 16 asserts step 3's removal (`white-space: normal`), which the second makes conformant.
`note_line_occupancy` is the only **occupancy-raising** writer of the five line-state fields,
called **at this stage** by `place_item` and the marker path and by nothing else — the third
caller M3's row names, `flush_line`'s open-box replay, arrives in **PR-1d** with the value it
replays (R7-a, narrowed to M6's height contribution at R10), so the count below is PR-1b's and the PR-1d item states its own. ⚠ **Stated with its complement, because the bare
universal is false and an implementer acting on it would move code M3 forbids moving** (R5):
`any_rendered_content` is also written by `force_break` (`:781`, unconditionally `true`), by
`flush_line`'s per-line reset (`:435`) and by the constructor (`:184`) — §4 enumerates them and
M3's Grounds keeps `:781` **outside** the function on purpose, which is what §6's forced-break non-regression cell
pins (PR-1d's, on `<pre>`). The
checkable form is the one §8's PR-1d item already uses for the rung, with **this PR's** count:
`grep -rn 'note_line_occupancy' crates/layout/elidex-layout-block/src/inline/` returns the
definition plus exactly **two** calls, and no *raise* of the occupancy state exists outside it. `on_line` is gone,
replaced by the three-valued
`line_occupancy` whose reset joins `flush_line`'s per-line block; `has_inline_axis_edge` and M3's
sums live in `pack/inline_box.rs` as free functions, callable from `inline/mod.rs:200` before any
`LinePacker` exists; the shaping break reads the predicate **per marker
side** (ledger **A63**: no marker touches the hang) — the derivation produces the two halves and `has_inline_axis_edge` is their disjunction
(M3, M5). **The check is by the property, not by the identifier** (R5): the R2-F3 defect is *any*
statement — code or prose — that makes a boundary gate turn on the box rather than on the marker's
own side, and a grep for `has_inline_axis_edge` finds only the spellings that name it. The item is
therefore that at the shaping-break gate, and at every site **describing** it, the quantity
named is the marker's own side; and that no site outside the records — the freeze's R2-F3
bullet, its rev-63 reopening, the 2026-09-20 attestation table and the ledger — states a live
hang gate. ⚠ The identifier grep missed the one site that stated the whole-box rule outright —
§7's `current_line_last_hang` bullet, which names no identifier at all, so
`grep -c has_inline_axis_edge` on it returns 0. The population is the **property** — every site that states what a boundary gate turns on, or
what a marker does to a trailing space — and no runnable predicate returns it, so the command
below is a **seed and not an inventory**: `grep -noE 'hang gate|current_line_last_hang|shaping
break|the hang' <memo>` misses every `hang`-bearing line that uses none of its four literals, and
how many that is moves with each edit — which is why the item is the property and the reviewer
reads outward from the seed rather than treating its output as the list (ledger **A63**);
M8 adds **both** edge terms **in this PR**; the min-content prereq's segmentation and trimming of
**both** intrinsic passes (ledger **A64**, **A68**) — is in `main` before it (§8's *Min-content prereq PR*; ledger **A47** — an earlier drafting deferred the
min-content half past PR-1b — and **A58**).
⚠⚠ **One suite-level break invariant over the whole of §6, and it is the item that replaces
per-cell vigilance** (R5). For **every** fixture the `decorated_inline` modules lay out, the set
of realised line-break offsets must be a **subset** of
`find_break_opportunities(<the fixture's text with every atomic item encoded as U+FFFC>)`,
mapped back to item-stream offsets, ∪ **an atomic boundary whose adjacent character is
U+00A0** ∪ any forced break; a shared
helper asserts it and every cell's test runs through it. A fixture whose line structure comes
from `place_item`'s per-item flush — the pre-existing divergence
`#11-inline-item-boundary-soft-wrap` records (§5.3), which this program must not pin as expected
— then turns red the day it is written rather than the day the slot is discharged.
⚠⚠ **The atomic term is a spec licence, not slack, and the invariant condemned conforming
behaviour without it** (R11). css-text-3 §5.5's bullet **after** the one §6's standing rule
quotes: "For Web-compatibility there is a **soft wrap opportunity before and after each replaced
element or other atomic inline**, even when adjacent to a character that would normally suppress
them, including U+00A0 NO-BREAK SPACE. However, with the exception of U+00A0 NO-BREAK SPACE,
there must be no soft wrap opportunity between atomic inlines and adjacent characters belonging
to the Unicode GL, WJ, or ZWJ line breaking classes." Atomic items contribute **no text**, so
`a<img>b` concatenates to `"ab"`, whose opportunity set is empty, and a width-forced flush before
the `<img>` — which §5.5 **licenses** — would be reported as unlicensed and classified as
`#11-inline-item-boundary-soft-wrap`. The invariant therefore runs over the **item stream**,
not over one string — but the *licence* is U+FFFC's, and only one case has to be carved out.
⚠⚠ **R11 first wrote this term as "both boundaries of every atomic, deliberately permissive at
the GL/WJ/ZWJ exception", and that was wrong in the one direction that matters** (R12). A subset
assertion over-licensing is not the safe direction here: over-licensing an *item boundary* is
exactly how the flush this invariant exists to catch gets blessed, so a fixture could pin an
item-boundary bug at an atomic adjacent to a WJ. Read the same bullet's second sentence as
normative and the rule needs no class modelling of its own — feeding U+FFFC to the engine's own
breaker reproduces css-text-3 §5.5 **exactly, except for NBSP**. Measured through
`find_break_opportunities` (probe run and reverted; `crates/text/elidex-linebreak`):

| adjacency | §5.5 | `find_break_opportunities` with U+FFFC | |
|---|---|---|---|
| ordinary letter | break before and after | `"a\u{FFFC}b"` → `[(1, Allowed), (4, Allowed)]` | ✓ agree |
| WJ (U+2060) | **no** break between | before → `[(7, …)]`, after → `[(1, …)]` (that side suppressed) | ✓ agree |
| ZWJ (U+200D) | **no** break between | before → `[(7, …)]`, after → `[(1, …)]` | ✓ agree |
| **NBSP (U+00A0)** | break **is** allowed — the named exception | before → `[(6, …)]`, after → `[(1, …)]` (suppressed) | ✗ **the one carve-out** |

U+FFFC is UAX #14 class **CB**, so LB12/LB12a suppress the break next to a GL/WJ/ZWJ neighbour —
which *is* §5.5's second sentence — and the single disagreement is NBSP, which the bullet's first
sentence licenses by name. Hence the licensed set above: the breaker's own answer over the
U+FFFC-encoded text, plus NBSP-adjacent atomic boundaries, and nothing admitted blindly. Whoever discharges `#11-inline-item-boundary-soft-wrap` by
deriving a break-opportunity test from the concatenated text would **remove** these opportunities
— a regression the slot's own wording used to invite (§5.3). Ledger **A32**.
⚠ **It must also see a line the packer *discards***: a whitespace-only line is flushed and then
dropped on `any_rendered_content == false`, appearing in no `InlineFlow` and no `line_count`,
which is what hid the co-resident cell's withdrawn arm from three sweeps (§6, PR-1d). The offset
set is therefore taken at the **flush** — every offset at which `flush_line` is entered **for a
soft wrap or a forced break**, committed or discarded — not from the persisted lines.
⚠⚠ **The reason is named, and naming it excludes the terminal commit** (R6-a). `FlushReason` has
exactly **three** variants — `SoftWrap`, `Forced`, `LastLine` (`inline/pack/justify.rs:20-28`) —
and `finish()` enters `flush_line(FlushReason::LastLine)` whenever the line is occupied
(`pack/mod.rs:786-789`), at the end-of-text offset, which `find_break_opportunities` **filters out
by construction** (`crates/text/elidex-linebreak/src/lib.rs:28-33` drops the mandatory break
`unicode-linebreak` emits at `text.len()`). A predicate over *every* `flush_line` entry therefore
reports the ordinary end of a paragraph as an unlicensed break, and fails every fixture whose last
line carries content — which is all but the text-less ones. So the recorded set is the `SoftWrap`
and `Forced` entries only. `Forced` needs no separate licence either: the engine reaches
`force_break` from `PackItem::Text`'s `break_after == Some(BreakOpportunity::Mandatory)`, i.e. from
an offset the same `find_break_opportunities` call returned, so "plus any forced break" above is
the **mandatory half of the same result**, not a second allowance. **Checkable as written**: the
helper matches on `FlushReason` and its `LastLine` arm records nothing, so the exclusion is a code
shape a reader can see rather than a description to be trusted.
⚠⚠ **Its DoD is the run, not the sentence** (R6): the PR lands the helper with **every** §6 cell
routed through it and the whole matrix green in one execution. A gate whose behaviour on the real
corpus is unmeasured is the defect and not the fix, which is what the first execution of this one
showed — ledger **A20** (the offset set it recorded) and **A18** (the one cell it failed).
⚠ **And a fixture is read in both regimes when it is written or re-fixtured**, today's layout and
the one the markers' advance creates: a break licensed in one can be unlicensed in the other, and
the landed helper only ever sees the second, so nothing in the suite catches a cell whose *window*
was chosen against the first (A18 is that instance). **Why this rather than another rule in the
preamble**: §6 already carries one, with a measured discriminator, and two members still got
through, because a rule's population is the cells a reader classifies as needing a second line
while the class is the fixtures whose *layout* has an unlicensed break. A predicate over every
fixture has no such population — the shape
[[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]] asks for. It lands here, in the
first PR whose advance can move a break, and the later PRs inherit it as their cells join the
modules. (⚠ A bold PR label is kept off the start of a line here: `plan-xcheck.py`'s check 5
reads one as a DoD heading and would split this paragraph from its own cell list.)
⚠ **The box's own geometry is
untouched**: `inline/pack/boxes.rs:82-84` still writes `EdgeSizes::default()`, so `border_box()` is
still `content` and no `border_box()` reader changes extent — the property §5.3 gives as the reason
this PR can precede PR-1c. `pack/mod.rs:726`'s fabricated citation is rewritten here (§3.1) — the one cite
this program corrects, because this is the PR that changes what that comment documents; no other
citation work (§9). ⚠ **A reader disposition of its own**: PR-1b moves the *position* of
`LayoutBox.content` **for every decorated inline with content** — §7's first numbered change, whose
qualifier this sentence used to drop by writing "every inline" (round 25, Axis 3) — so this PR — not only PR-1c —
re-checks §7's reader family, and `elidex-render`'s existing `border_box()`-reading paint tests are
named and dispositioned here. `note_line_occupancy`, `pack/inline_box.rs` and M8's contribution
carry docstring citations to their §3 rows.

**PR-1c** (box geometry): cells 6, 10b, 10c, 10d, 13, 13b, 13c, 13d, 14c, 15c, 17, 17b, 17c, 17d, 17f and 17g —
⚠⚠ **Cell 13c is new at R6, and the argument for it is at the level of the *set*, not of any
cell** (R6-d). M4's invariant (iii) is that padding, border and margin stay three **independent**
`LayoutBox` fields; the discrimination question is therefore not "does some cell assert an edge"
but "does the set separate the three". Read as a set, **no cell in the list above but 13c
asserts a `LayoutBox.border` or `LayoutBox.margin` field **produced by layout** — the
population is that list, not a second copy of it; 10c's and 10d's margins reach their results
through the cursor, never through the field, and 13d's `border:2px solid` is painted from a
`LayoutBox` its render harness sets by hand, so it tests the reader and not the producer (ledger
**A62**) — while
the border term is exercised only by the end-to-end clause below (`border:5px solid` through
`elidex-shell`) and the margin term by cell **3**, which is PR-1b and observes the *cursor*, never
a produced box. An implementation that dropped `LayoutBox.margin`, or routed margin into
`padding`, therefore satisfied **every** listed cell. 13c closes it with three pairwise-distinct
non-zero values, so a *swap* between two fields fails it and not only a drop. ⚠ **The same
set-level read over M5's disjunction finds one more, and §6's `margin`-alone PR-1b cell is where
it is fixed rather than here**: M5 is `padding ∨ border ∨ margin` with a **single** derivation site and two readings
(§5.1 M5), and **no cell isolates its margin term** — the PR-1d flip set reaches padding through
cell **1** and border through cell **2**, and every remaining cell the attestation's M5 row names
either carries a non-zero padding on the side it reads or is a negative, all-inline-axis-zero
cell. Because the derivation site is one, pinning the term at any one reading pins it for
both, and cell **3**'s markup (§6) is that pin. Nothing else in M4 or M5 has the shape: their
remaining quantities are single-valued (the rect, the advance sum, the rung).
⚠ **all of them assert a first-layout property**, because `assign_inline_layout_boxes` skips any
entity that already carries a `LayoutBox` (`boxes.rs:62-64`) and **no production site removes
one**: wire #5 of `.claude/tools/layout-box-reader-trip-wire.sh` bans the
`remove_one::<LayoutBox>` shape outside test paths, and the `trip-wires` CI job that runs it is
ungated (#496), so the property is machine-enforced rather than a grep count restated here — the one
live hit is under `crates/script/elidex-js/src/vm/tests/` and is test-only. ⚠ State it at the
wire's own reach: that script's header disclaims the stronger reading, so what is enforced is
"no **production** removal", not "no removal". ⚠⚠ **PR-1c therefore makes the write generation-aware, and R12 moves that from the slot into
this PR.** `assign_inline_layout_boxes` skips any entity that already carries a `LayoutBox`
(`boxes.rs:62-64`), so without this the new edges freeze at first layout — and R11's cell 13d is
what makes that *visible* rather than merely stale: `InlineFlow` is rebuilt on **every** pass
(`inline/mod.rs:413`'s "Reconcile `InlineFlow` … every pass", the `insert_one` at `:528` —
probe-gated at `:525`, not unconditional — and the candidate-key clear at `:156`), so after any
restyle the glyphs move to their new
positions while the background and border are painted at the **first-layout** box. Before PR-1c
nothing was painted for an inline at all, so this desynchronisation is **created here**, which is
why "the same skip already freezes `LayoutBox.content`" does not carry it: `content` being stale
is a wrong number, this is a wrong *picture*. The change is the one `#11-inline-relayout-box-staleness`
is **not** the one the slot prescribes. ⚠⚠ **R12 first wrote "a `layout_generation`
comparison"; R13 measured it inert and it is withdrawn.** `layout_generation` is **constant `0`
off the paged path**, and the engine says so at both sites that had to solve this already:
`inline/reconcile.rs:198-200` at `22de3078` — "`layout_generation` is constant 0 off the paged
path, so this is an explicit reconcile (insert-or-remove), **not a generation comparison**" — and
`inline/mod.rs:558` — "(F9 — `layout_generation` is constant 0 non-paged, so **removal, not
comparison**)". Stored and current would both read `0`, the comparison would skip exactly as the
presence test does, and nothing would be repaired. The repair PR-1c depends on is therefore **not a box reconciler of its own — the engine already has the
canonical one, and R14 took half of it twice** (R15's self-root-check). The repair moves to a
**prerequisite PR** that discharges `#11-inline-relayout-box-staleness`, and PR-1c depends on it.

⚠⚠ **Why the root-check landed there rather than on a fourth patch.** Three consecutive rounds
found a defect in the previous round's own repair — R13 said PR-1c may not defer the refresh, R14
measured the deferred remedy inert, R15 found the replacement silent on **removal**. The two
questions the check must answer in writing:

1. *Is a canonical algorithm missing?* **No — it exists and is two-sided.** The IFC's staleness
   reconciler for `InlineFlow` is *insert-or-remove over a candidate set*: `reconcile_flows`
   persists what this pass produced and `clear_inline_flows` removes it from every candidate key
   not persisted, the latter documented at `inline/mod.rs:552-559` (`22de3078`) as "**the single
   staleness reconciler** (F9 — `layout_generation` is constant 0 non-paged, so removal, not
   comparison)". R14's text quoted "explicit reconcile (**insert-or-remove**)" and then specified
   only the insert half, which is exactly the hole R15 found: an entity that *leaves* the producer
   set — a decorated empty `<span>` whose last edge is restyled to zero, so M1 emits no marker and
   no `place_item` bounds exist either — keeps its IFC-written box forever.
2. *Does the mechanism violate the project's own stated ideal?* **Yes.** CLAUDE.md *One issue, one
   way* requires a single canonical form rather than "新 seam + N 個の legacy 実装", and
   `clear_inline_flows` calls itself **the** single staleness reconciler. A second, box-only,
   insert-only reconciler standing beside it is a second implementation of one rule — the thing
   the mandate forbids — and it is what the per-fix lens kept producing because that lens only
   ever asked whether *this* patch is correct.

**So the repair is a prerequisite PR's, and this memo states the defect and stops there** —
the **Reconciler prereq PR** paragraph below. What PR-1c's DoD owes is the dependency: it is not
cut from `origin/main` until that PR has landed, and no cell of PR-1c asserts a second-pass
geometry. Found by Codex on #515; the prerequisite carve is R15's, its scope correction R16's.
Ledger **A40**, **A44**. **§7's full `LayoutBox`-edge reader list
dispositioned, no member of it standing in for the rest** (a `getBoundingClientRect` assertion
included, made through the
layout-level channel §5.2 names, not in `elidex-dom-api`) — **and dispositioned over a second entity
class, generated content**: a decorated `::before`/`::after` (the 6g class, pinned under PR-1a) gets real edges here too, so
every reader listed for elements is re-read for pseudo entities (⚠ **and the reader set is the
same one, not a wider one**: render emits an inline pseudo at `builder/inline.rs:205-213` as a
`StyledTextSegment` from its `TextContent` and `continue`s, so it reaches neither `walk` nor any
chrome; what a pseudo does carry today, through its own `place_item` run, is the `LayoutBox` the
CSSOM / hit-test / a11y readers take — cell 13's delta on a second class, no new reader) — and §7's allowlist reconciliation re-run
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
span's `LayoutBox` carries `border == 5` **and** **all four** `client*` members return **0** — real producer, real
reader, one assertion, and taking the four rather than `clientTop` alone is what stops a geometry test pinning the pre-prerequisite `clientWidth`/`clientHeight` padding box (R7-c); and, in the same clause, a second markup through the same pipeline —
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

**PR-1d** (existence): cells 18, 19, 20, 21, 22, 23, 23b, 24, 24b, 24c, 24d and 24e, plus the flip set — **cells 1, 2, 7, 8, 10, 11, 12 and 12d flip; 5, 6b, 6c, 6f, 6d, 6e, 6i, 6g, 6h and 9 do not**; this is the one normative statement of it — every §7
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
`grep -rn 'note_line_occupancy' crates/layout/elidex-layout-block/src/inline/` must return the
definition plus exactly **three** calls, **named**: `place_item`'s, the marker path's in
`pack/inline_box.rs`, and the open-box **replay** `flush_line` runs after its per-line reset
(M3's third caller, R7-a). A fourth call is a second raise site and fails the item, and a
count of two fails it the other way — an implementer who hits it by dropping the replay
regresses an open box's continuation-line metrics, which §6 cell 24e discriminates.
⚠ **Three, and not "two occupancy-raising ones"** (R9-a): narrowing the predicate to
occupancy-**raising** call sites does not exclude the replay, because `BoxEdgeOnly` is a rung
*above* `Empty` and the line the replay runs on was reset to `Empty` two statements earlier, so
it raises exactly as the other two do. What the replay does not touch is `any_rendered_content`
(it passes `contributes_content: false`, A23's sibling ground), and that — not the raise — is the
property PR-1d's flip set rests on; the named-site form above checks it by naming the site rather
than by counting a class the replay belongs to anyway. Ledger **A27**;
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
matter) and the reviewer's findings are **plan inputs** — #515's first pass reset the counter the
Terminator's rule kept, and the **design freeze**, not a renewed TERMINAL, is what unblocks the
PR (freeze rule 4); the precedent is #416 (`12ebc052`) and
#470 (`283bcc0d`), one-file docs-only landings of an umbrella memo (`git show --stat`; ⚠ #434 is
*not* one — it landed code beside its memo, `git show --stat deb6eaf6`). It carries the memo and the **program bookkeeping**
rows §10 tags `approval PR` — the rows made true by the umbrella's *approval*, which no code
PR's change makes true. **The two plan-checker tools and the SKILL.md standing note have landed
ahead of it, as #518** (`4394af4c`, 2026-09-22 — "deliberately the files only", by its commit
message); **what remains is their own tooling PR** (§9's task — skill infra by §9's own classification, approval-independent, cut
from `origin/main`; **scope = the checkers' wiring, generalisation and two-file awareness as its
memo decides (§9's memo-split booking), and nothing else** — the
`SPEC_LABEL_REVERSE` CSS-label gap is **owned elsewhere**, by the SoT slot
`#11-preflight-css-module-labels` (citation-hygiene Slice B, after its A-ii migrates that dict),
so this program cites the owner and hand-verifies citations meanwhile, as §3 records; ordered
**on its §9 trigger — #510's resolution (landing or closure) or this umbrella's TERMINAL, whichever comes first** —
because #510 puts a generic plan-memo checker and a selftest wire on the same
`trip-wires` registry and the tooling PR's own plan-memo must decide build-on-vs-beside that
substrate under `/elidex-plan-review` (#506 shipped checker tooling without one and paid a tooling-only IMP tail across several external rounds (the SoT's #506
record; no count carried here)); its DoD retires or rewrites the SKILL.md note #518 landed, which
describes the pre-state it ends), tagged `tooling PR` in §10. It gates nothing else in this
program — not the predicate prereq's plan-review, which hand-verifies its citations as every CSS
plan does today. **How the approval PR relates to this branch**: `git merge origin/main` after #518 folded the
landed copies back as rev 57 predicted — the two checkers add/add, resolved to
`origin/main`'s versions (`SKILL.md` merged clean, the branch's copy already matching) — so
`git diff --name-only origin/main...HEAD` lists the memo alone. The approval PR is cut from
`origin/main` and takes the memo with `git checkout layout-decorated-inline -- <memo>`, so its
DoD, **its diff against `origin/main` is exactly one file**, holds by construction; this branch
is retired after it lands. PR-1a
branches off `main` only after the approval PR and the predicate prereq have landed, PR-1b only
after the min-content and end-of-line white-space prereqs have, and PR-1c
only after the reconciler prereq has (topology
below).
**Ordering — done as fixed: the checkers landed first (#518), the approval PR after.** The ground
was a dependency, not a schedule preference (R6-c; the unordered reading is ledger **A21**): §3's
coverage map is produced "from the table above by `python3 .claude/tools/plan-xcheck.py <memo>`",
§5.3's defer count is cross-checked by it and §8 routes §10's routing errors to its check 13, so
an approval PR landed first would have put on `main` a document whose re-runnable verification
commands named files the repository did not contain — the class §3.1 exists to prevent, one
level up. The remedy was the order, not a bundle: the approval PR's diff stays one file, since
bundling would have put skill infra through a plan review scoped to a layout umbrella, the shape
#506 paid for. The order is now a property of any approval head cut from `origin/main`, and
re-runnable: `git merge-base --is-ancestor 4394af4c HEAD` → exit 0, and
`git ls-tree -r HEAD --name-only | grep -cE '\.claude/tools/plan-(sweep|xcheck)\.py'` → 2.
What remains of the tooling PR is ordered before neither the approval PR nor any crate PR of this program. The predicate, reconciler,
min-content and end-of-line white-space prereqs are
ordered against neither the approval PR nor the tooling PR (nor was round 21, now complete). No later PR re-ships any of them — the shipper is defined by
the *event* that makes the row true (approval; the tooling task), not by a PR letter.
⚠ Earlier revisions routed all of this to the seam-3 prereq PR, and rev 25's first draft to
PR-1a. #508 shipped only its own plan-memo, on the user-ratified rule (2026-08-16, recorded in
`docs/plans/2026-08-inline-seam3-reconcile-split.md` — whose preamble also says the tooling would
"travel with the umbrella", a binary framing the tooling-PR model supersedes): *a PR carries the bookkeeping its own
change makes true and hands the program's bookkeeping to the program* — and the same rule rejects
PR-1a: its change makes none of these rows true either, and "the first PR after approval" is a
temporal accident, not a truth-maker (this memo does not fix which PR lands first after TERMINAL;
the predicate prereq is approval-independent like its five siblings — an earlier drafting called
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
   case for one canonical answer rather than weakening it. ⚠ The `is_monolithic` consumers are a
   **routing note of this requirement**, not a disposition §9 carries: §9's canonical-predicate
   bullet books exactly the two consumers **inside** this program — the four `client*` members and
   M1's emit test — and hands every other question about that PR's scope to its own memo, so
   whether they become its second consumer is that memo's call. ⚠ **Why presence is the wrong answer, with the
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

**Reconciler prereq PR** — **the gap**: `assign_inline_layout_boxes` skips every entity that
already carries a `LayoutBox` (`inline/pack/boxes.rs:62-64`, identical at `22de3078`) and the IFC
has no removal half, so the two components written past that skip — the box at `:88` and
`InlineClientRects` at `:124-126` — are **first-layout-only artefacts**; the function says so
itself, "neither its LayoutBox nor a stale `InlineClientRects` is refreshed here" (`:95-101` at
`22de3078`). `InlineFlow`, the same pass's other per-entity output, is reconciled every pass
(`inline/mod.rs:413`'s "Reconcile `InlineFlow` … every pass", the `insert_one` at `:528`, the
candidate clear at `:156`), so one pass's outputs age differently. Measured, the skip runs in
**three** directions.

1. **Stays.** An entity the IFC lays out in both passes keeps the first pass's geometry — `:62-64`
   fires on the box the IFC wrote itself — and `getClientRects` returns **early** off the stale
   component (`element/layout_query.rs:219`), never reaching the `border_box()` fallback at
   `:236-238`, so a multi-line inline that relayouts to one line still answers with its old
   per-line rects. After PR-1c the glyphs move on a restyle and the chrome does not, at cell 13d's
   measured scale on `elidex-render`'s relpos harness: the box the edges give is
   `(12,-12) 40x44` and 5 `SolidRect`s against the `(24,0) 16x20` and 1 the first pass left.
2. **Leaves.** An entity that drops out of `entity_bounds` keeps geometry no producer will write
   again and no remover will take: restyle it to `display:none` and `collect_inline_items_inner`
   `continue`s before recursing (`inline/collect.rs:218-220`), so it contributes no item,
   `place_item` records no rect, and its IFC box stands unchanged. Ledger **A40**'s decorated
   empty `<span>` whose last edge goes to zero is this direction one PR later.
3. **Transfers — either way, with the entity still in `entity_bounds`.** `place_item` pushes a
   rect for every placed item whose entity is not the IFC's own (`inline/pack/mod.rs:703-715`),
   **atomics included** — which is why the skip's own comment names `layout_child`'s
   inline-blocks. So the entity is present and the geometry on it is another producer's.
   *inline → `inline-block`*: `atomic::layout_atomic_items` runs at `inline/mod.rs:179` and the
   tail at `:380` (both `22de3078`), so a **fresh** atomic box from `layout_child`
   (`inline/atomic.rs:60`) stands before the tail; the skip keeps the tail off the entity, and the
   **stale** `InlineClientRects` of its inline pass survives beside the fresh box — current box,
   dead rects, one entity, and direction 1's early return serves the dead half.
   *`inline-block` → inline*: the mirror. `layout_atomic_items` matches `InlineItem::Atomic` and
   nothing else (`inline/atomic.rs:38-44`), so it no longer visits the entity; the **stale
   atomic** box survives, nothing removing one; and `:62-64` sees that box and skips the fresh IFC
   bounds. The other producer's geometry stands on an entity the IFC does lay out this pass.
   ⚠ rev 53 called such an entity "now absent from `entity_bounds`"; `place_item`'s push is
   blind to item kind (`:703`), so it is present and a rule keyed on that absence never reaches
   it.

**Extent**, measured at `22de3078` on 2026-09-21: `InlineClientRects` has **one** producer in the
workspace and **no** remover (`git grep -nF 'InlineClientRects' -- crates` → the `insert_one` at
`pack/boxes.rs:124-126`, no `remove_one` anywhere), and `LayoutBox` has no removal path either —
`crates/core/elidex-ecs/src/dom/geometry.rs` exposes `set_layout_box` (`:140`) and
`layout_box_mut` (`:169`) and nothing that removes, while wire #5 of
`.claude/tools/layout-box-reader-trip-wire.sh` bans the `remove_one::<LayoutBox>` shape outside
that file and outside test paths, its one live hit being test-only.

Extending the engine's single staleness reconciler (`reconcile_flows` + `clear_inline_flows`,
`inline/mod.rs:552-559` at `22de3078`) is a terminal per-PR slice under CLAUDE.md's *Edge-dense
work* rule, so the mechanism, the candidate set, the removal chokepoint, the producer
discrimination and the touch set are that PR's plan-memo and `/elidex-plan-review` (ledger
**A44**). ⚠⚠ **Three revisions stated a requirement here instead, each with a corner the next
round found; all three are withdrawn** (ledger **A48**). The repair discharges
`#11-inline-relayout-box-staleness` rather than narrowing it — the same edit repairs
`LayoutBox.content`.

Ordered **in `main` before PR-1c**, against nothing else here. Found by Codex on #515 — the carve
is R15's, its scope correction R16's, the transfer directions R17's and R18's. No `PR-1x` letter;
it is a prerequisite, spelled like its five siblings.

**Min-content prereq PR** — **the gap**: `min_content_inline_size` (`inline/measure.rs:15-37`) is
`max_word = max_word.max(m.width)` over `run.text.split_whitespace()` (`:29`) for each
`InlineItem::Text` and nothing else — **a segmentation that is not layout's**. Layout's has two
parts: it breaks where `find_break_opportunities` (`crates/text/elidex-linebreak/src/lib.rs:26`,
UAX #14 through the `unicode_linebreak` crate; called at `pack/items.rs:72`) puts an opportunity —
and, pre-existing, also at item boundaries, where the `:690` guard flushes, which is
`#11-inline-item-boundary-soft-wrap`'s defect and not this oracle's — and it measures a segment by
its **trimmed** width (`:690`, trailing collapsible spaces excluded), so a bare swap of the
segmenter without the trim would over-report by a space. `split_whitespace` splits at every `char`
whose `is_whitespace()` is true — the Unicode `White_Space` property — one item at a time. So the
function returns the maximum over **per-item, per-whitespace-word** widths, with no edge term,
where css-sizing-3 §2.1's *min-content inline size* is "… the inline size that would fit around
its contents if all soft wrap opportunities within the box were taken" (`body css-sizing-3
auto-box-sizes`; ledger **A64**). ⚠ **The block's name is historical; its scope is both intrinsic
passes** (ledger **A68**): `max_content_inline_size` carries the same defect in its own direction
(instance 5), so this PR owns the segmentation **and the trimming** of both, the end-of-line
white-space prereq owning the rule about which trailing spaces layout removes or hangs and PR-1b
the edge terms (**A58**). PR-1b widens the line by the edges, and shrink-to-fit is
`min(max_content, max(min_content, available))` (`elidex-layout/src/intrinsic/mod.rs:134`),
reached from `elidex-layout/src/layout/mod.rs:57` for an inline-level atomic display
(`InlineBlock` / `InlineFlex` / `InlineGrid` / `InlineTable`) with `width:auto`. Five measured
instances, owned apart (below; ledger **A58**, **A64**, **A68**) — 2–5 are the one defect, the
segmentation, seen across items (2), inside one run (3, 4) and in the other pass (5); 1 is the
edge term PR-1b adds on top of it.

1. **No edge term.** `<span style="padding:10px">x</span>` in such a box: with PR-1b's
   max-content term in place and min-content still edge-less, `max(min_content, available)` never
   reaches word + 20px at any available width below it, so the box settles under the advance
   PR-1b's line takes and the content overflows it. §6 cell 25 asserts the 20px on **both**
   intrinsic sizes for that reason.
2. **No join across items.** `a<b style="padding:10px">b</b>c` in the same box: css-text-3 §5.5 —
   "Out-of-flow boxes and inline box boundaries do not introduce a forced line break or soft wrap
   opportunity in the flow" — leaves `abc` one unbreakable segment across two inline-box
   boundaries, so its min-content contribution is `|abc|` plus the edges while the function
   returns `max(|a|, |b|, |c|)`: three text nodes are three `InlineItem::Text`s and each is walked
   alone. **An edge term alone does not close this** — `max(|a|, |b|+20, |c|)` is still not
   `|abc|+20` — so the joining is what instance 1's edge term needs to land on, not a follow-up.
3. **NBSP splits a segment layout keeps whole.** U+00A0 has the `White_Space` property, so
   `'\u{A0}'.is_whitespace()` is `true` and `"a\u{A0}b".split_whitespace()` yields `["a", "b"]`,
   while `find_break_opportunities("a\u{A0}b")` returns `[]` — no opportunity, as css-text-3 §5.5
   requires: "line breaking behavior defined for the WJ, ZW, GL, and ZWJ Unicode line breaking
   classes must be honored" (U+00A0 is GL). The function reports `max(|a|, |b|)` for a segment
   that has no soft wrap opportunity.
4. **CJK is not split where layout splits it.** `"日本語".split_whitespace()` yields one word, while
   `find_break_opportunities("日本語")` returns `[(3, Allowed), (6, Allowed)]`: the function reports
   `|日本語|` where layout's narrowest line is `max(|日|, |本|, |語|)` — css-text-3 §5.1's
   `word-break: normal`, "Words break according to their customary rules", which the engine's UAX #14
   oracle realises for ideographs (class ID) as a break between any two (`body css-text-3 word-break-property`). The
   commands that reproduce 3 and 4 are in ledger **A64**.

5. **max-content keeps a trailing space layout removes.** `max_content_inline_size` sums
   `measure_text(&run.text)` untrimmed (`inline/measure.rs:52-58`), while layout measures a
   segment by its **trimmed** width (`pack/mod.rs:690`) and the trailing collapsible space at the
   line's end is removed under css-text-3 §4.1.2 step 3 — the rule the end-of-line white-space
   prereq makes conformant. A max-content size is the width of a **single unbroken line**
   (css-sizing-3 §2.1), so its end *is* that position: `<span>abc </span>` in an `auto`-width
   inline-block gives a used width one space wider than the line both prerequisites produce.

⚠ **Extent — a second shrink-to-fit with its own behaviour**: `elidex-layout-block` has a
**homonym** `shrink_to_fit_width` (`positioned/constraints.rs:318`, the absolutely-positioned
path called from `positioned/layout.rs:319`) whose signature differs and which reads
`max_content_width` alone, never min-content at all.

**Why it is a prerequisite and no longer one of this program's own deferrals**: ledger **A47**
carries the ground, withdrawing `#11-inline-min-content-box-edges` — the inconsistency is between
intrinsic sizing and the layout PR-1b ships, not between the two intrinsic sizes, and deferring
the *max*-content half instead only relocates it. `max_word` is the pass's whole state, so
neither an edge nor a join has anywhere to land in it — the change reaches the pass's structure
and not only its arithmetic, which is the shape CLAUDE.md's *Edge-dense work* rule sends to its
own plan-review. What that change is, what it costs, what pins it and what it touches are that
PR's plan-memo's and that review's. ⚠⚠ **Rev 53 stated an obligation and four named risks here
instead; they are withdrawn** (ledger **A48**).

Ordered **in `main` after the end-of-line white-space prereq and before PR-1b** (rev 65; ledger
**A69**) — the trimming this PR gives the intrinsic passes applies the rule that prereq decides,
so implementing it first would implement an undecided rule and let the later PR move layout out
from under it, which is the mismatch the carve exists to remove. Against nothing else here: the
seam-3 and dead-arm prereqs have landed, and it is ordered against neither the predicate nor the
reconciler prereq. Found by
Codex on #515 (R17; instance 2's classification R18's). No `PR-1x` letter, like its five
siblings.

⚠⚠ **Ownership is split, and it is settled** (R25; ledger **A58**). **This PR owns instances 2–5 — the
segmentation and trimming of both intrinsic passes** (ledger **A64**, **A68**). Cross-item joining and the
per-run segmentation read `InlineItem::Text` and no marker, so it is realisable at the
base its ordering admits — before PR-1a, or between PR-1a and PR-1b, where §5.1 M1's dead-field
split leaves the markers carrying `entity` alone — and both grounds above are read off it.
**Instance 1 is PR-1b's**, for both intrinsic sizes: each pass calls the same
`collect_inline_items` the layout pass does (`measure.rs:22`, `:51`), so once PR-1b puts the
edges on the marker payload the passes' own item streams carry them, from M4 (i)'s one
derivation site, and the min-content edge term is arithmetic on the accumulator this PR built.
PR-1b thus ships the line advance and both intrinsic edge terms together, so no `main` commit
has layout and intrinsic sizing disagreeing **on a non-cyclic edge term** — A47's ground, met, for
what it covers — and §6 routes cell 25 to it. A **cyclic** percentage edge is the spec's own
exception: css-sizing-3 §5.2.1 rule 4 resolves it against zero for the intrinsic passes while
layout resolves it, and "the contents might thus overflow" — cell 25b pins it. An atomic inline's contribution disagrees on every commit
before and after the program alike (next ⚠), so PR-1b neither introduces nor widens that one.
The joining mechanism is this PR's plan-review's. ⚠ Rev 57 recorded the split as open and
handed it to that review (R22-e, **A56**); withdrawn, since ownership is this umbrella's to state.
⚠ A third part — `InlineItem::Atomic`
contributing zero — is **not** a third instance of this PR's defect: `measure.rs`'s own doc states
it of `min_content_inline_size` (`:13-14`) and `max_content_inline_size` (`:42-43`) alike, so it is
neither min-content-specific nor decoration-specific, and it is none of §8's four measured
instances. It is **registered**, not dismissed (Codex R27; ledger **A61**): `#11-inline-atomic-intrinsic-contribution` (§5.3, §10),
whose trigger is this PR's plan-review — the accumulator an atomic term would land on is built
here — without being bundled into it.

**End-of-line white-space prereq PR** — **the gap**: elidex's end-of-line white-space processing
is not css-text-3's. One trim serves every case: `measure_segment_widths` strips trailing ASCII
space and tab whatever `white-space` is (`inline/measure.rs:79`, `is_collapsible_space` being
`' ' | '\t'`, `inline/whitespace.rs:162`), `place_item` records `full − trimmed`
(`pack/mod.rs:701`) and alignment subtracts it (`pack/mod.rs:234-235`), while the spec's treatment
depends on the value and on what follows the space. **Pre-existing and decoration-independent**;
decoration only makes it visible, which is why the user carved it out of M3 on 2026-09-23 (ledger
**A66**) and then handed its outcomes to this PR's own plan (ledger **A67**). What conformance
means is named by reference, not restated: css-text-3 §4.1.2 steps 3 and 4, §5 (*forced line
break*, including a break "due to the start or end of a block") and §8.2 (hanging glyphs, its
intrinsic-size sentences included). Measured instances, at `154bac3f`:

1. **A collapsible trailing space keeps its advance.** It is subtracted at alignment rather than
   removed, so `place_item`'s rect for a run ending in it spans it (`inline_end =
   seg_inline_start + full_width`, `pack/mod.rs:706-714`) and whatever the line places after it
   sits one space further on.
2. **The trim has no `white-space` input** (`:79`), so the cases the spec separates are one case
   here — `pre-wrap` before a forced line break and `pre` among them.
3. **The trim covers ASCII space and tab only.** `is_collapsible_space` is `' ' | '\t'`
   (`inline/whitespace.rs:162`), so other space separators and U+1680 OGHAM SPACE MARK, which
   step 3 and step 4 name, are untouched.
4. **The intrinsic passes exclude a different set of trailing spaces than layout does** — the
   same defect seen from the other side, and **owned by the min-content prereq** (its block above;
   ledger **A68**), not left to a description: `min_content_inline_size` segments with
   `split_whitespace()`, which drops every `char::is_whitespace` separator — U+00A0 and U+3000
   included — where layout's trim strips `' ' | '\t'` alone, and `max_content_inline_size` measures
   `run.text` whole (`inline/measure.rs:52-58`), trailing space and all, where layout removes that
   space at the line end. **This PR owns the rule** — which trailing spaces layout removes or hangs
   — and that prereq owns its application in both intrinsic passes.

**How it relates to the min-content prereq**: **ordered before it** (rev 65; ledger **A69**),
and the ownership is stated rather than shared — **this PR owns the rule** (which trailing spaces
layout removes or hangs), the **min-content prereq owns both intrinsic passes' segmentation and
trimming** (this block's instance 4; ledger **A68**), and **PR-1b owns the edge terms** (ledger
**A58**). The order follows from that split: trimming applies a rule, so the rule lands first.
**Ordered in `main` before the min-content prereq and therefore before PR-1b**, against nothing
else here. What the change is, what pins it and what it touches are its own
plan-memo's and `/elidex-plan-review`'s, and so are the cells for everything but the `normal`
outcome this umbrella keeps (cells 16 and 16b; ledger **A67**): the undecorated path (a lone
`<span>abc </span>` at a line end, `pre`, `pre-wrap` before a forced break), where a marker's
edges block a hang, how a hanging space sits in its box, and where the box's border-box end falls
once the space is removed or hung. No `PR-1x` letter, like its five siblings.

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
delta disturbs a later obligation is measured, not inferred (`grep -n do_carrier <memo>`
returns landing-record sites only — §5.2's row and this paragraph — and no DoD; no line figure is
carried, because this ⚠ is itself one of the hits and a count stated here would be falsified by
stating it. ⚠ **The command carried a second disjunct until R22** — `grep -n 'do_carrier\|eleven'
<memo>`, which at `46a48641` returns **eight** lines where the sentence claimed two, six of them
the ordinary English number-word the disjunct spells. It counted something the sentence never
claimed; the conclusion survives on the single-term form, which at that base returns exactly the
two sites named), and in full it is
round 20's question ([[feedback_plan-ratified-surface-is-a-design-change]]). One consequence is
already known: the successor slot's disjunct 3 fired (§5.2, §10).
⚠ **The predicate prereq is ordered against neither of them, but is ordered against PR-1a.** Its
constraint is **in `main` before PR-1a**, because M1's emit test consumes the predicate (§5.1 M1,
§6 cell 6c). ⚠ This memo does **not** claim it is disjoint from the other five: its touch set
follows
from its own plan-review's choice of home (§9), so disjointness is a question it answers, not a
premise this memo may use. What the umbrella owns is the ordering.
⚠ **The reconciler and min-content prereqs are ordered the same way and each against a different
PR**: against none of the other prereqs, and **in `main` before PR-1c** and **before PR-1b**
respectively — PR-1c's M4 write has no reconciliation hook without the first, and PR-1b's line
advance has no intrinsic size that fits it without the second. Each touch set is likewise its own
plan-review's (§8's *Reconciler prereq PR* and *Min-content prereq PR* paragraphs), so this memo
claims no disjointness for either. The **end-of-line white-space prereq** is ordered **before the
min-content prereq**, which is itself before PR-1b: it decides the rule whose application the
min-content prereq gives the intrinsic passes (rev 65; ledger **A69**; its §8 block).
All six branch from `main`;
whichever lands second takes `git merge origin/main` (⚠ **not** `rebase` — an opened PR branch
cannot be rebased without a force-push, which `~/.claude/hooks/` denies; #511 needed no merge —
it was cut from `origin/main` after #508 landed). **Branch topology**: the
**six** prereqs branch off `main`, as does the **tooling PR** (approval-independent, ordered on
its §9 trigger — #510's resolution or TERMINAL — and against nothing else here; its files landed
ahead as #518, §8 above); the **approval PR**
is cut from `origin/main` at TERMINAL carrying the memo alone (§8 above); PR-1a branches off `main` after **three** of them
*and the approval PR* have *landed* — the two `elidex-layout-block` ones (**both landed**) because they move code PR-1a edits, and
the predicate PR (**pending**) because PR-1a's M1 consumes what it establishes. The remaining three
are **pending** and are ordered against a later PR rather than PR-1a: the **end-of-line
white-space** and **min-content PRs** against **PR-1b** — in that order, the first before the
second (ledger **A69**) — and the **reconciler PR** against **PR-1c**, so each may land any time
before its own and after whatever it follows. ⚠ Whether the
predicate prereq is ordered against the other five depended on its touch set, which §9 hands over;
the two `elidex-layout-block` prereqs having landed, that question is now moot for ordering and
survives only as the predicate PR's own re-anchoring against `22de3078`, and the same holds for
the reconciler, min-content and end-of-line white-space PRs. PR-1b,
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
the shape M7's Grounds refuses for its own residual: that slot's **subject** is the css-inline-3
§5.3 *layout-bounds* model — how a box's own metrics compose into its line — which needs no
**per-glyph** provenance anywhere in its five facets (§5.3), whereas the fallback-only condition
asks which font produced a glyph that is already *present*. They share the word "font" and nothing
else. ⚠ **Codex R2's widening of that slot is what this sentence is now stated against**: the
earlier ground — that the slot's subject was only the block container's *root inline box*, which
§1.1 distinguishes from a glyphless box's *strut* — no longer holds on its face, because facet (d)
is strut composition; the mismatch that survives, and the one the rev-33 strike already rested on,
is provenance. Implementing the layout-bounds model would discharge that slot and leave the
fallback condition undone with nothing preventing the close. The disjunct is struck from that
slot's trigger (§5.3) in the same revision, so the two sites cannot drift apart (round 25, Axis 3).

**Likewise not covered** (R11): css-writing-modes-4 §3.2's blockification of an inline box whose
`writing-mode` differs from its parent's. **It gets its own slot,
`#11-writing-mode-inline-blockification`** (pre-existing class; Why / trigger / re-eval in §5.3,
registered by §10's `approval PR` row), and §3's css-writing-modes-4 §3.2 row is
`✗ (pre-existing → slot)` accordingly — it read `✓` until R11, on the spec's conclusion rather
than on the engine. ⚠ **This memo assumed the rule instead of measuring it, and the assumption was
load-bearing for a Grounds bullet, not only for a cell**: §5.1 M1's first Grounds bullet argued
that a decorated inline box "always shares the IFC's writing mode" *because* §3.2 makes the
differing case an atomic, while M1's own payload took the writing mode from the box. On this
engine both halves of that are false at once, so the Decision changes with the ground (§5.1 M1,
ledger **A31**) — the payload's writing mode is the **IFC root's** and only its `direction` stays
the box's. Implementing §3.2 is an engine-wide computed-display change in `elidex-style` and is
**not** a rider on an inline-layout program; what this program owes is the conservative
degradation and the cell that pins it, §6 cell **12f**, which that slot's discharge retires.

**Likewise not covered** (R12): css-text-3 §5.5's intra-word shaping clause — a word broken
across lines must still have its characters shaped "as if the word were still whole". **It gets
its own slot, `#11-intra-word-shaping-across-line-break`** (pre-existing class; Why / trigger /
re-eval in §5.3, registered by §10's `approval PR` row), and §3's css-text-3 §5.5 intra-word row
is `✗ (pre-existing → slot)` accordingly — it read `✓ (pre-existing)` until R12, on a mechanism
that answers the **inverse** question. ⚠ **The row cited `pack/mod.rs:744`'s coalescing as if it
delivered the clause**, and that coalescing is scoped within one line by explicit design
(`flush_line` clears `last_placed_entity` at `:439`, under a comment at `:436-438` saying why),
so it re-joins the engine's own within-line segmentation and never reaches the case the clause is
about. This is the residual §3's preamble names: check 4 tests where a Touch **points**, not
whether what it points at **delivers**, and this row passed it throughout. Nothing in this
program creates the gap or touches it — `:744`'s within-line coalescing is unaffected by PR-1b's
boundary break, which is the css-text-3 §7.3 rule in the opposite direction — so what this
program owes is the disclosure and the routing, not the fix. Ledger **A36**.

**Likewise not covered** (R15): css-text-3 §7.3's **second** normative sentence — shaping must
**not** be broken across an inline box boundary "when there is no effective change in formatting,
or if the only formatting changes do not affect the glyphs (as in applying text decoration)". **It
gets its own slot, `#11-shaping-break-at-unchanged-inline-boundary`** (pre-existing class; Why /
trigger / re-eval in §5.3, registered by §10's `approval PR` row), and §3's css-text-3 §7.3 row
carries a **split verdict** accordingly — its first sentence `✓`, its second `✗`. ⚠ **The row
quoted only the first sentence until R15, and the reviewer read its second and third
triggers as the gap — which was right, though not for the reason either side first gave**.
Measured, all three triggers are satisfied by **over-breaking** wherever the box places content:
a text run takes the parent element's entity (`collect.rs:309`), so such a box changes it and
`:744`'s coalescing cannot reach across that boundary. ⚠ **In the member-less corner only trigger
1 closes**, by PR-1b's marker — a `vertical-align`-only or isolation-only box gets no marker (M1
requires a non-zero edge) and keeps its two same-entity runs coalesced, so triggers 2 and 3 stay
live there and take their own slot, **`#11-shaping-break-vertical-align-and-isolation`**
(pre-existing class; Why / trigger / re-eval in §5.3, registered by §10's `approval PR` row).
Ledger **A43**. And what the memo had never quoted at all was
the sentence the over-breaking violates. This is the same residual as the row above, one step
further on: check 4 saw a Touch that pointed, §3's own preamble says it cannot see whether the
clause is **delivered**, and here it could not see that the clause quoted was **half** of one.
Nothing in this program creates the gap or narrows it — PR-1b's boundary break runs in the *first*
sentence's direction — so what this program owes is the disclosure and the routing, not the fix.
Ledger **A41**.

**Likewise not covered** (R16): css-inline-3 §5.3's strut is taken "with the metrics of the box's
**first available font**", and elidex has no font that is always available —
`FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60-84` at `154bac3f`) ends
`self.db.query(&query)` and answers `None` when the family list matches nothing, with no
last-resort family appended anywhere in the call. **It gets its own slot,
`#11-shaping-no-last-resort-font`** (pre-existing class; Why / trigger / re-eval in §5.3,
registered by §10's `approval PR` row), and §3's css-inline-3 §5.3 glyphless row records the
qualification on M7's half accordingly. ⚠ **What made it visible here is the one line this
program creates that has nothing else to take a baseline from**:
`<p><span style="font-family:no-such-family;padding:1px"></span></p>` has no text, so
`inline/mod.rs:200`'s measurability gate — inside `if has_text` at `:191` — never runs, M5 commits
the line on the inline-axis edge, M7's tentative gets `None`, and the IFC returns
`first_baseline == None`. **The program does not create the gap**: the same `None` already makes
ordinary text in an unavailable family render as nothing on `origin/main`
(`measure_text`'s `db.query(…)?`, `elidex-shaping/src/measurement.rs:54`), with no marker
involved. What it does is reach the gap through a *second* door, which is why the disclosure is
owed here and the fix is not — supplying a last-resort font is `elidex-shaping` work. ⚠ **It is
neither of the two font slots this memo already carries**:
`#11-inline-fontless-measurability-gate` is the text-driven early return that this markup skips,
and `#11-inline-fallback-font-strut` is §5.3's *second* condition, which presupposes glyphs.
Ledger **A45**.

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
    files have since landed as #518 (§8) and PR #501 is live in `.claude/`, but the two touch
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
  (the four shipping PRs, the four pending prereqs and the tooling PR — **nine** reviews), which
  read §8/§10 — a surface a split would not shrink. ⚠ **It said five until rev 53**, a figure
  that predated both the reconciler carve (R16) and the min-content one (R17).
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
  first-layout-scoped. ⚠⚠ **R11's reading — "what M4 changes is *which fields* go stale, not
  *whether* they do", so PR-1c need not own the refresh — was withdrawn at R12.** It holds for a
  wrong *number* and not for a wrong *picture*: `InlineFlow` is rebuilt on every pass
  (`inline/mod.rs:413`, `:528`, `:156`) while this box is not, so after PR-1c and cell 13d the
  glyphs move on a restyle and the background and border stay at the first-layout box — a
  desynchronisation that does not exist today, because nothing is painted for an inline at all.
  **The write is repaired before PR-1c, and R13/R15 corrected *how* twice.** The
  `layout_generation` comparison this slot prescribes — and which R12 promoted on the slot's word —
  is **inert**: the counter is constant `0` off the paged path, as `inline/reconcile.rs:198-200`
  and `inline/mod.rs:558` both state at `22de3078`, so the comparison degenerates into the presence
  test. ⚠ **The slot's own prescribed remedy had never been measured** — a deferral puts the
  *problem* under review and leaves the *remedy* unexamined, and promoting one into a PR is the
  moment to re-derive it. The repair PR-1c depends on is therefore **not a box reconciler of its own — the engine already has the
canonical one, and R14 took half of it twice** (R15's self-root-check). The repair moves to a
**prerequisite PR** that discharges `#11-inline-relayout-box-staleness`, and PR-1c depends on it.

⚠⚠ **R15: this slot is
  therefore not narrowed but *discharged*, as a prerequisite PR ahead of PR-1c.** Extending the
  engine's one staleness reconciler in both directions repairs `LayoutBox.content` — the residue
  R13 left here — by the same edit, so splitting the two would be a second implementation of one
  rule (CLAUDE.md *One issue, one way*), which is the root R15's self-root-check named. ⚠ **R16:
  the *mechanism* by which it extends is that PR's plan-review's, not this memo's, and rev 50's
  second copy of the derivation and of the self-root-check's two written questions is struck from
  here** (ledger **A44**) — §8 is the single site, its **PR-1c** paragraph carrying those two
  questions and its **Reconciler prereq PR** paragraph the defect and the extent that bounds it.
  Nothing about candidate sets, ownership flags or a `clear_inline_flows` extension is settled in
  this memo. Found by Codex on #515; the carve is R15's, the scope correction R16's.
  (`#11-inline-align-clientrects-nonpersist-path` was ledger-marked to fold into terminal-Z
  C-3/C-4 alongside it; the dead-arm prereq #511 **closed** it instead — SoT corrected at landing,
  2026-09-07: the fold note now applies to `#11-inline-relayout-box-staleness` alone.)
* **`#11-layoutbox-absence-unreachable`** (#488, registered in the SoT) — no longer relevant, and
  §10 carries the disposition row: M5 keeps no entity's box withheld, so no truthful box-absent
  signal is required.
* **Intrinsic sizing** — per M8: both edge terms are **in PR-1b**; the intrinsic passes' segmentation and
  trimming — cross-item joining, layout's break oracle and max-content's trailing space — is a
  **prerequisite PR in `main` before PR-1b** (§8; ledger **A58**), because that pass
  has no accumulator to attach edges to and PR-1b is not correct without the joining (instance 2); instances 3 and 4, the per-run mis-splits, predate the program and PR-1b neither creates nor widens them; an atomic inline's own contribution, zero in both passes before and after, is `#11-inline-atomic-intrinsic-contribution`'s (§5.3). ⚠ **It was slotted until
  rev 53** and the slot is withdrawn, not re-tagged (ledger **A47**). The intrinsic passes pass
  `0.0` as the containing inline size — css-sizing-3 §5.2.1 rule 4, decided at §3's row for it (ledger **A59**) and pinned by cell 25b.
* **`#11-css2-spec-label-normalisation`** — this memo cites css-inline-3 for the model, so its
  remaining CSS 2 cites are **§8.3, §8.3.1, §9.2.1.1, §9.2.2.1, §9.4.2, §9.4.3, §9.5, §10.2,
  §10.8.1 and §16.6.1** — ten (§10.2 entered with rev 59's A59, whose list still read nine — the recurrence recorded next; re-derived by the command below). ⚠ **Seven until rev 34, and the eighth was invisible to the very command
  that defines the list** (round 26, Axis 4): `§8.3.1` entered with rev 33's §1.2 edit written
  **bare**, and the grep below requires the `CSS 2 ` prefix, so the module-qualification sweep of
  this revision is what made it countable. The two figures are ordered: qualify first, then re-run.
  ⚠⚠ **And the list read "eight" through rev 55 while its own grep returned nine** (Codex R22):
  **CSS 2 §9.5** entered in that revision's own commit, in §3's css-inline-3 §2.1 row, where the
  section's cross-reference to floats is quoted — so the enumeration contradicted its defining
  command from the moment it was written. `git show 46a48641~1:docs/plans/2026-08-line-box-decorated-inline-content.md | grep -o 'CSS 2 §[0-9.]\+' | sort -u`
  returns eight and the same command on `46a48641` returns nine, which is what "the list is
  the command's output, not a curated subset" below is for: the bullet stated the rule and the
  round that added a cite did not re-run it. A dry round is not a re-run.
  The front matter's **seven** counts something else — the section↔title *pairs* — and excludes
  the three untitled cites, CSS 2 §9.5 (quoted as a cross-reference, no title),
  CSS 2 §16.6.1, which §1.6 cites with no title, and CSS 2 §10.2 (rev 59, quoted). ⚠ **Two untitled, not one** (the R22-era record; three from rev 59): the front matter
  called §16.6.1 the lone one until R22, §9.5 having arrived in the same commit.
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
  sub-slices, not a single PR". Both programs are in the Layout lane. This umbrella's ten crate
  PRs do not block on C-3 (they add no carrier and no consumer), but the splits slot does — its
  trigger is amended to name C-3b alongside PR-1d landing. ⚠ **The reconciler prereq is the one
  whose non-blocking is not this memo's to assert**: §8's extent for that defect includes
  `elidex-ecs/src/dom/geometry.rs` — the file C-3a's seam owns, and the file wire #5 leaves as
  the only sanctioned home for a `LayoutBox` write — so whether it is parallel-safe with C-3 is a
  question for its own plan-review, on its own touch set.
* **`#11-bidi-full-uba-fidelity` — the paint-time reorder drops the gap the markers open**
  (R7-b; **pre-existing** class, an **existing** slot of another program, so no deferral of this
  one's). On a line whose bidi order is non-identity, `builder/inline_flow.rs:154-190` (at
  `22de3078`) discards every `Text` run's baked `inline_start`, starts the paint cursor at
  `min(inline_start)` over the line's runs (`:169-173`) and advances it by each run's shaped width,
  so an RTL paragraph with a padded inline gets PR-1b's advance and PR-1c's box geometry and paints
  its runs contiguously — the decoration gap is in the layout and not on the screen.
  **Measured, not read off the source**: with two RTL runs whose baked positions differ by a gap
  and with that gap closed, the emitted glyph positions are identical; the crate's own
  `converged_ltr_identity_no_reorder` is the control, an identity line painting each run at its own
  baked position, so the loss is the non-identity branch's and not the renderer's generally.
  **Disposition: route and pin, not fix.** Three grounds, and the measurement is the first of them,
  not the precedent: (1) the branch **already owns this class** — it discards layout's baked
  justify offsets the same way and names this very slot for it in its own comment (`:160-168`), so
  a second owner would be the duplicated decision surface CLAUDE.md's *one issue, one way* refuses;
  (2) carrying per-run offsets through the reorder is an `elidex-render` mechanism, the crate
  boundary the Layering mandate keeps and the same ground as ledger A6; (3) it is **pre-existing**
  and reachable today — any line with two runs at non-adjacent baked positions under a non-identity
  order loses the space between them, with no marker involved. What this program adds is
  **reach**, not the defect. §6 cell **12b** carries the pin and asserts no painted position; §7's
  painted-output bullet states the scope for every site of this memo that says the glyphs move.
  Ledger **A24**. ⚠ Not `#11-inline-box-decoration-splits`: that slot owns the **fragment
  attribution** a bidi split needs (one `DOMRect` per fragment, and which edge survives), a
  *layout* representation gap, while this is a renderer discarding positions layout got right — the
  mechanism mismatch the memo applies elsewhere to keep a slot's subject honest.
* **`#11-block-in-inline-anonymous-block-split`** (new slot, **pre-existing** class; Codex R8-b):
  CSS 2 §9.2.1.1 *Anonymous block boxes* — "When an inline box contains an in-flow block-level box,
  the inline box (and its inline ancestors within the same line box) is broken around the
  block-level box …, splitting the inline box into two boxes (even if either side is empty), one on
  each side of the block-level box(es)", with the line boxes on each side enclosed in anonymous
  block boxes (`body CSS2 anonymous-block-level`) — is unimplemented on the inline path.
  `collect_inline_items_inner`'s recursion arm has no block-level filter, so a `display:block` child
  of an inline is recursed into and its text becomes an ordinary run of the **enclosing** IFC; §6
  cell 6i's markup is the reachability witness and `block/mod.rs:70`'s `children_are_block`, testing
  direct children only, is why the `<p>` is on the IFC path at all.
  **Why it is this program's concern and not only a gap it inherits**: when the **outer** inline is
  decorated, M1's predicate is satisfied by that inline's *own* style, so the markers bracket a
  block's content and PR-1b's advance, PR-1c's geometry and PR-1d's line existence are each defined
  over a representation §9.2.1.1 says should not exist.
  **Why it is nonetheless pre-existing and not an own deferral**: nothing here creates the
  flattening. The enclosing inline already wraps that content and the IFC already claims it as its
  own line, with no marker involved; what the markers change is that the wrong representation
  becomes *observable* — an advance, a `LayoutBox`, a committed line — where today it is only
  implied. So it does not enter §5.3's per-PR own count, on the ground §10's other
  pre-existing-class rows state.
  **Why not ordered behind the split instead** (the reviewer's alternative): the fix is a box-tree
  change in `block/children/stack.rs`'s anonymous-box construction reached from `stack_block_children`
  (`block/mod.rs:433`), not in the inline module this program touches, and it re-parents the content
  out of this IFC entirely. Ordering a marker program behind it would block every PR of this
  umbrella on an unrelated unimplemented feature while changing nothing about whether the markers
  are right for the tree the engine actually builds. Ledger **A26**.
  **Trigger**: the next change that implements or touches the §9.2.1.1 break for an inline
  containing an in-flow block-level box — i.e. one that reaches `stack_block_children`'s anonymous
  wrapping from the inline side — or any PR of this umbrella that needs the decorated-outer case to
  produce spec geometry rather than the flattened one. **Re-eval**: at PR-1d's landing, against §6
  cell 6i, which is the cell that would have to change.
* **`#11-resize-observer-inline-empty-content-rect`** (new slot, **pre-existing** class; Codex
  R9-c): resize-observer-1 §3.3.1 *content rect* — "non-replaced inline Elements will always have
  an **empty** content rect" — is unimplemented. The observer-facing size is
  `LayoutBox::content_rect_local` (`crates/script/elidex-js/src/vm/host/resize_observer.rs:404-407`),
  a total function of `padding` and `content.size`
  (`crates/core/elidex-plugin/src/layout_types/boxes.rs:204-211`) with no inline special case, and
  an inline `LayoutBox`'s content height is the **line's** block size
  (`commit_aligned_entity_rects` writes `block_size: line_height`, `pack/mod.rs:493`;
  `assign_inline_layout_boxes` takes `bounds.block_end - bounds.block_start`, `boxes.rs:77`).
  **Why it is pre-existing, and measured so rather than argued so**: an ordinary
  `<p><span>Hi</span></p>` already reports a non-empty `content_rect_local()` for the span, with
  no marker anywhere — every inline that owns a run violates §3.3.1 today. What is accidentally
  conformant is only the *empty* inline, and only because it has **no** `LayoutBox` at all, so the
  detector's `unwrap_or((Rect::default(), Size::ZERO))` (`resize.rs:255-256`) reads it as `(0,0)`.
  **Why it is nonetheless this program's concern**: PR-1c (cell 14c) and PR-1d (cell 21) grant a
  `LayoutBox` to exactly those box-less targets, so the height an already-observed empty inline
  reports moves 0 → the line's, the change detector fires (`resize.rs:259-262`), and a
  `ResizeObserverEntry` is delivered with a non-empty `contentRect` where §3.3.1 requires an empty
  one. §7 states that callback as a consequence of the grant; this slot is what keeps it from
  being read as an *intended* one.
  **Why the fix is not a cell here, and the ground is the crate boundary and not invisibility**:
  the correction belongs to the reader, and every candidate site is outside this program's crates
  — `elidex-js`'s `size_fn` closure, `elidex-api-observers`' change detection, or `elidex-plugin`'s
  `content_rect_local` itself, whose purity §7 already records as that crate's contract. Cell 14c
  states the same boundary from the other side: `elidex-js` depends on no layout crate, so a cell
  here could only hand-insert a `LayoutBox` and would assert the marshalling. This is A6's
  boundary and A24's shape — a reader-side mechanism, reachable today, whose reach this program
  extends without creating it.
  **Why not ordered behind the correction instead** (the reviewer's second option): the violation
  is live for every inline with content, so ordering PR-1c behind its fix blocks a layout program
  on an observer change that has to happen anyway and that no PR here makes more or less true for
  the targets already affected. Ledger **A29**.
  **What the fix needs**: the same canonical *inline box* predicate the `client*` prereq PR
  establishes (§3's cssom-view-1 `client*` row, §9's canonical-predicate bullet) — css-display-3
  §A's three-conjunct definition, of which §3.3.1's "non-replaced **inline** Elements" needs the
  whole, including the **non-replaced** half elidex does not answer today (`is_atomic_inline`,
  `inline/collect.rs:14`). ⚠ **"Precisely the half §3.3.1 keys on" until R22** — §3.3.1's clause
  carries both conjuncts, and this slot spends the composed predicate rather than the
  replacedness test alone (which is what the css-pseudo-4 §4.1 consumer in §9 takes). The prereq PR is this slot's natural **input**, not its owner: it lands
  the predicate in a layout/DOM crate, and this slot spends it in the observer reader.
  **Trigger**: the next change to `elidex-js`'s `resize_observer.rs` size source, to
  `elidex-api-observers`' change detection, or the first resize-observer WPT subset this engine
  declares supported. **Re-eval**: at PR-1d's landing, against §6 cells 14c and 21, the cells
  whose granted box widens the reach.
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
  restatement of cssom-view-1 §6 step 3 at `boxes.rs:90-91` and **five** further sites — defined by
  the command, not this list: `grep -rn 'fragment per line' crates/` → 6 hits / 3 files on
  `154bac3f` (`boxes.rs:91`; `pack/mod.rs:218`, `:343`, `:443`, `:446`; `tests/inline_flow/align.rs:105`)
  — ⚠ **"four further sites" until R22**, one short of the command that defines the class, the
  hit count beside it being right (so the slot receives **seven classes: four with a concept grep** — Box Model,
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
  (`crates/api/elidex-api-canvas/src/component.rs:132-148`), which inserts at `:139` (⚠ **`:140`
  until R22** — that is the `entity` argument; `insert_one` opens at `:139`); both attach
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
  the other five prereqs, not a sibling topology, not a cold-gate width. All three follow from the
  home decision, which is handed over; §8's ordering paragraph and §9's cold gate now say so, and an
  earlier revision asserted all three from a premise it had already delegated.
  **Deferrals**: whatever it opens is its own memo's call, not counted against this program's
  per-PR budget (§5.3) — it is carved precisely because its questions are not this memo's.
  ⚠ **What it does not fix, and must say so**: elidex has no replaced arm on the inline path at all
  (`is_atomic_inline` tests display keywords only, and `collect.rs`/`atomic.rs` reach no replaced
  element), so a replaced inline still gets no correct atomic layout after this PR. The predicate
  makes the *classification* available and cell 6c keeps this program off the gap; closing the gap
  is separate, pre-existing work neither this PR nor this umbrella takes on — **and it now has a
  destination, `#11-replaced-inline-no-atomic-layout`** (pre-existing class; Why / trigger /
  re-eval in §5.3, registered by §10's `approval PR` row). ⚠ **Until R15 that sentence named no
  token**, which made it an unrouted hand-off rather than a disposition: the gap is author-reachable
  on `<p>a<img src=x>b</p>` today — `is_atomic_inline` (`inline/collect.rs:14-19`) is false for
  `Inline`, so the IFC recurses into an `<img>` that has no children and emits nothing, while the
  sizing the case needs sits in `block/replaced.rs`, reachable only from `block/mod.rs`. Ledger
  **A42**.
* **Ruby annotations (css-inline-3 §2.3 clause 4) are unimplemented engine-wide, and that is a
  missing feature rather than a divergence.** ⚠ **The entry was one line, with no Why, no
  trigger and no disposition, until R15** — ⚠ and *not* the only §9 bullet lacking all three, so
  the ground for completing it is not rarity: the file-growth and intrinsic-sizing bullets lack
  them too and need none, being method notes rather than gaps, and the dead-arm bullet states its
  disposition as a deletion in prose. This entry is the one that names an **unimplemented spec
  clause** and then says nothing about who owns it, which is the shape §9 exists to close.
  **Why out of scope**: ruby is a distinct
  inline-level layout feature — an annotation model, `ruby-position`/`ruby-align`, and a second
  interlinear level above the line — whose work is in the parser, the style cascade and a ruby
  layout algorithm, not in the decoration of inline boxes; clause 4 names ruby annotations only as
  one of the in-flow contents that keep a line box non-phantom, and the clause elidex implements
  (`contributes_content`) is indifferent to which content it is. **Disposition**: no slot, on the
  `<br>`/`<wbr>` bullet's own ground — the only trigger such a slot could carry is *implementing
  ruby*, which makes the slot circular, and a slot with no writable trigger is a feature request
  booked against a program whose subject is inline decoration. Nothing here reaches it: with no
  ruby boxes there is no ruby annotation for clause 4 to count, so this program's markers, advance
  and phantom decisions are unchanged the day ruby arrives.
* **`flush_line`'s non-persisting arm (`inline/pack/mod.rs:393-421`) is dead, and this program
  deletes it.** `persist_candidate` (`inline/mod.rs:239`) is identically true — `FragmentationType`
  has only `Page`/`Column` and the constraint's field is non-optional — so `flow_align` is always
  `Some` and `LinePacker::new` has one call site. **A second standalone prereq PR** (stacked beside
  the seam-3 split, not folded into it, so the byte-identical criterion stays provable) removes the
  arm and the `persist_candidate` branch, and **closes
  `#11-inline-align-clientrects-nonpersist-path`**, which books work against the same unreachable
  code. Booking a fold, a DoD cell or a *new slot* against it would all be wrong: CLAUDE.md is
  unconditional on dead code, and deleting collapses two slots instead of opening a third.
* **The enumeration-completeness audit (R20) — the standard it used, what it found, and the six
  slots it opens.** R20 asked for the audit `.claude/skills/elidex-plan-review` Pre-condition #1
  describes and this memo had never run, because the `Full enum?` column is spent here on a
  conformance-and-routing mark instead (§3's preamble, ledger **A34**). **Unit**: the table's
  **25 distinct spec sections** at `09026f98`, not its 42 rows — the two part wherever one
  section carries several rows, and §3's preamble carries the command that produced the figure.
  **Standard for "covered"**: a branch is covered when a **§3 row, or a place a §3 row explicitly
  hands off to, *names* it**. A mention in §1, §5, §6 or §9 prose that no row points at is a
  **separate tier** and does not count — that tier is what the sweep bullet below collects, and
  holding the two apart is what stops a gap reading as owned because some paragraph once
  discussed it. **The precedent for a per-clause enumeration done well is already in this memo**:
  §1.2's five-clause table for css-inline-3 §2.3 numbers every clause of the defining sentence
  and gives each an elidex status, which is the shape the other 24 sections never got.
  **Yield: nine branches** with no row, no §6 cell, no slot and no disposition. **Three of the
  nine are truncated normative sentences marked against the fragment quoted** — css-text-3 §7.3
  (three sentences carried as two, §1.4); css-inline-3 §5.3 (the half-leading row's Branch was
  the bare formula `A′ = A + L/2`, which belongs to one of the section's two branches, §1.5);
  and css-break-3 §5.4 (the bidi clause lifted out of a sentence that also names
  display-type-imposed breaks and ends "Otherwise such breaks must be handled as `slice`").
  **Two of the nine sit in sections this table cites five times and once**, so re-reading a
  section the memo had already opened would have reached them — which is the sense in which this
  audit is the detector the review loop was not.
  **Two are reachable with zero author CSS**: §7.3's third sentence, through the UA sheet's
  `b, strong { font-weight: bolder; }` (`crates/css/elidex-style/src/ua.rs:117`), and §5.3's
  `normal` branch, `normal` being `line-height`'s initial value.
  **Where each goes** — the measurement for every one of them is at its §3 row and is not
  restated here; this list is the routing:
  * **css-text-3 §7.3 sentence (c)** → **`#11-shaping-across-formatting-change-boundary`** (new,
    **pre-existing** class). *Gap*: shaping is broken at a boundary that does change the glyphs
    but meets none of sentence (a)'s three triggers, where §7.3 says it should not be broken "if
    it is reasonable and possible for that case"; the entity split at `inline/collect.rs:309` is
    the whole cause and predates every marker. *Why not the sibling slot*:
    `#11-shaping-break-at-unchanged-inline-boundary` is sentence (b)'s, whose predicate is this
    one's complement. *Trigger*: the next change to run segmentation in `inline/collect.rs` or to
    shaping-run construction in `crates/core/elidex-render/src/builder/inline_flow.rs`, or the
    first css-text-3 §7.3 WPT subset this engine declares supported. *Re-eval*: 2026-11-01.
  * **css-inline-3 §5.3's `line-height: normal` branch** →
    **`#11-inline-height-normal-layout-bounds`** (new, **pre-existing** class). *Gap*:
    `LineHeight::Normal` collapses to `font_size * 1.2`
    (`crates/core/elidex-plugin/src/computed_style/text.rs:136`) and the packer then runs the
    not-normal branch, so `normal` layout bounds are a fixed ratio of the font size rather than
    the font's own A and D, and cannot span fonts. *Why not a sixth facet of
    `#11-inline-root-inline-box`*: that slot's five facets are each pinned by a §6 cell, and it
    records in its own text that none of them turns on per-glyph provenance — the one input this
    branch takes; it is the same mechanism boundary that gave
    `#11-inline-fallback-font-strut` its own slot at round 25 rather than a fold into that one
    (§8). ⚠ **That ground disposes of the fold and left the *overlap* standing** (Codex R22): the
    sibling slot's scope read "the whole css-inline-3 §5.3 layout-bounds model" and its trigger
    "any work that gives inline boxes layout bounds of their own", so the two claimed one subject
    and one event. The sibling's scope and trigger are narrowed to the **composition** side
    (§5.3), which leaves the bounds *computation* under `normal` here, and each carries the
    other's discharge as a trigger disjunct so a close on one side cannot read as a close on both.
    *Trigger*: any line-height correctness work reaching `LineHeight::resolve_px`,
    any work that gives an inline box layout bounds of its own, or
    `#11-inline-root-inline-box`'s discharge (which composes the bounds this branch computes).
    *Re-eval*: 2026-11-01.
  * **css-inline-3 §2.2 step 1, *Baseline Alignment*** → **`#11-inline-baseline-alignment`**
    (new, **pre-existing** class). *Gap*: the IFC performs no baseline alignment at all —
    `vertical_align` occurs at no site under `crates/layout/elidex-layout-block/src/inline/` and
    `dominant-baseline` has no elidex selector, so a line's baseline is `first_baseline`,
    captured first-wins from the first text run. *Why it is named under this umbrella*: step 1
    runs before §2.2 steps 2–4, which is where M6's `block_advance` and M7's strut sit.
    *Trigger*: the next change to `inline/pack/mod.rs`'s baseline capture, or any work
    implementing `vertical-align` outside the table layer. *Re-eval*: 2026-11-01.
  * **css-pseudo-4 §4.1's suppression clause** →
    **`#11-pseudo-generation-on-replaced-originating-element`** (new, **pre-existing** class).
    *Gap*: `generate_pseudo_entity` (`crates/css/elidex-style/src/pseudo.rs:27-81`) carries no
    replacedness test and its call sites (`crates/css/elidex-style/src/walk.rs:245-248`) add only
    a `display` test, so `img::before { content: "x"; padding: 10px }` generates a pseudo §4.1
    suppresses. *Why it is this program's concern and still pre-existing*: nothing here generates
    the pseudo; what PR-1a changes is that it acquires a marker pair and becomes observable —
    `#11-block-in-inline-anonymous-block-split`'s shape exactly. *Its input, not its owner*: the
    **replacedness half** of the canonical predicate the prereq PR establishes — the half that
    bullet already separates out as the only one needing a component-bearing crate — a consumer
    beside the two that bullet enumerates and beside
    `#11-resize-observer-inline-empty-content-rect`, which is the precedent for routing a
    consumer this way. ⚠ **The input is replacedness *alone*, not the composed inline-box
    predicate** (Codex R22; the entry named the composite, with the `non-replaced` conjunct
    identified as the half it keys on, which reads as the composite's negation). `body
    css-pseudo-4 generated-content` suppresses "when their parent, the originating element, is
    **replaced**" and on nothing else, while the composite is false for a **non-replaced** atomic
    origin — `span { display: inline-block }` fails the inner-flow conjunct — whose generated
    children §4.1 does not suppress; an implementer reading the composite would strip them while
    fixing `<img>`. ⚠ **No other consumer named in that bullet has the same confusion, checked
    rather than assumed**: M1's emit test and the four `client*` members are classifying an
    *inline box*, and `#11-resize-observer-inline-empty-content-rect`'s clause is
    resize-observer-1 §3.3.1's "**non-replaced inline** Elements", which is the composite too —
    so all three take the whole predicate legitimately and this entry is the only one keyed on
    replacedness alone. *Trigger*: the predicate prereq PR's landing, or the
    next change to `elidex-style`'s pseudo generation. *Re-eval*: at the predicate prereq PR's
    landing.
  * **css-writing-modes-4 §6.4's Note** → **`#11-used-direction-text-orientation-upright`** (new,
    **pre-existing** class). *Gap*: elidex derives no *used* `direction` anywhere, so
    `text-orientation: upright` in a vertical writing mode leaves `LogicalEdges::from_physical`
    and M1's `WritingModeContext` reading the computed value where §6.4 reads the used one.
    *Scope*: engine-wide rather than M1's, every caller taking `direction` the same way.
    *Trigger*: the next change to `resolve_writing_mode_properties`
    (`crates/css/elidex-style/src/resolve/mod.rs:235-277`) or to `logical.rs`'s mapping, or the
    first `text-orientation` WPT subset this engine declares supported. *Re-eval*: 2026-11-01.
  * **css-inline-3 §2.1's float clause** → **`#11-line-box-float-narrowing`** (new,
    **pre-existing** class). *Gap*: `flush_inline_run`
    (`crates/layout/elidex-layout-block/src/block/children/helpers.rs:37`) hands the IFC the
    whole containing width as its `inline_size` and no float context reaches
    `crates/layout/elidex-layout-block/src/inline/` at any production site, so no line box is
    ever narrowed by a float. *Trigger*: the next change to `flush_inline_run`'s inline-size
    derivation, or any float-interaction work that reaches the IFC. *Re-eval*: 2026-11-01.
  **The remaining three take no token, each for a stated reason.** css-break-3 §5.4's
  display-type half: both halves are owned already — the split that would produce the fragments
  is `#11-block-in-inline-anonymous-block-split`'s and the decoration at their broken edges is
  `#11-inline-box-decoration-splits`'s, and the sentence's own "Otherwise … `slice`" fallback is
  what elidex satisfies today. css-sizing-3 §5.2.1 rule 4: delivered by this program, so no
  platform slot — the two intrinsic passes thread `0.0` (§3's row, which decides it; ledger
  **A59**), PR-1b ships that with both edge terms (ledger **A58**) and cell 25b pins it. css-content-3 §1's per-value box count: conformant today by an empty
  complement, `parse_content` being unable to build an `<image>`.
  ⚠ **One audit finding is not a gap at all, recorded so it is not re-found**: css-text-3 §5.5's
  bullet 6 reads "Out-of-flow boxes **and** inline box boundaries do not introduce a forced line
  break or soft wrap opportunity in the flow", and §3's row carries the inline-box half alone —
  but elidex **conforms** on the half the row drops. `PackItem::Placeholder` records a static
  position and reaches no flush (`inline/pack/mod.rs:667`), and `inline/whitespace.rs:53` skips
  it without touching collapse state. Unenumerated, conformant, no slot.
  ⚠ **This audit is a one-time manual discharge for the scope as it stands, not a mechanism.**
  The mechanism is the clause-case check the tooling bullet below books, and nothing here claims
  the table is now swept clean — the audit read 25 sections once, by hand. Ledger **A50**.
* **Gaps this memo *states* and never gives a token — the class behind R15's two findings, swept
  once and left as a seed.** The slot machinery itself is sound: every `#11-` token this memo
  *opens* carries a §5.3-or-§9 definition and a §10 row, which `plan-xcheck.py`'s checks 4, 8 and
  13 enforce in three directions — ⚠ and the population is "opened here", not "mentioned here":
  `#11-bidi-full-uba-fidelity`, `#11-layoutbox-field-typed-reader-coverage` and
  `#11-preflight-css-module-labels` are named by this memo and owned by other lanes, so they carry
  no §10 row and check 8 exempts them by construction, firing only where a slot's full definition
  sits beside the token. The defect is narrower and invisible to all three — a gap
  **reasoned about at length and never given a destination**, so there is no token for a checker
  to resolve. Both R15 findings were instances: the replaced-inline gap, argued through a whole
  paragraph of the predicate-prereq bullet above and closed with "neither this PR nor this
  umbrella takes on" (now `#11-replaced-inline-no-atomic-layout`), and css-text-3 §7.3's
  must-not-break sentence, which the memo never quoted at all (now
  `#11-shaping-break-at-unchanged-inline-boundary`, §1.4 / §3 / §5.3 / §8). Ruby annotations above
  were the third, and are dispositioned rather than slotted.
  ⚠ **This list is a *seed*, not a swept-clean claim, and the harvest that produced it failed its
  own positive control**: it looked for negation vocabulary ("not implemented", "no … arm",
  "never reaches"), and the R15 instance that mattered most carries none — §3's §7.3 row said
  "trigger 1 of 3", a *positive* phrasing, while the clause it left out appeared nowhere at all
  (`grep -c 'must not be broken'` and `grep -c 'effective change'` over the memo, both **0**
  before this round). A gap the memo never states cannot be found by any wording of a text
  harvest, and a gap it states positively will not be found by that one: the vocabulary is the
  wrong population ([[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]]), and no
  count of "how many were missed" is offered here, because the denominator would be the harvest's
  own output ([[feedback_convention-dependent-figures-are-argument]]). The predicate that *would*
  have found it is booked as a named check on the plan-checker tooling task below, and
  **mechanical closure of this class is that task's, not this revision's**.
  ⚠ **R20's enumeration-completeness audit above is a *different* population, and neither sweep
  reaches the other's**: that one reads the 25 spec sections §3 already lists and asks which of
  their branches no row names; this one reads the memo's own prose and asks which of the gaps it
  states has no destination. A branch in a section §3 never listed is outside both — which is
  what keeps this list a seed rather than a swept-clean claim even now.
  **Measured and found already routed** (each re-read, none needing a token): `content_rect_local`'s
  origin moving from `(0,0)` to `(padding.left, padding.top)` — §8's PR-1c item accepts the
  exposure in as many words ("accepted despite being assertable"), because the value is entailed
  by an input cell 13(a) already pins; `any_rendered_content`'s second writer, `force_break` —
  §8 states the complement explicitly, M3's Grounds keeps that write **outside**
  `note_line_occupancy` on purpose, and §6's forced-break non-regression cell pins it, so M3's
  "one owner" is a *conceded* exception and not a silent one; `assign_inline_layout_boxes`
  iterating `entity_bounds` without asking whether the bounds are degenerate — that is M4's
  invariant (v), and cell 6's phantom sub-cell is what turns red if a carrier ignores it; the
  font-less **text** line whose strut never promotes — cell 24 pins that direction as expected and
  the class is `#11-inline-fontless-measurability-gate`'s; and `offsetTop`/`offsetLeft`'s
  first-box divergence — routed to `#11-inline-box-decoration-splits` in the `client*` bullet
  above. ⚠ The **one** residue in that set is §7's reader-family entry for
  `element/layout_query.rs`, which says "needs a cell" and names none: **disposed of here, not
  slotted** — `getBoundingClientRect` on a decorated inline is exactly cell 13(a)'s assertion
  read through the CSSOM, `offsetLeft`/`offsetTop` and the offsetParent walk add no *new*
  observable beyond the border box those cells already pin, and the divergence that is genuinely
  theirs is the splits slot's. The to-do is withdrawn rather than discharged with a cell.
  **Dead code is not slot-eligible, and routing it would be the error.** Two members of this
  sweep are dead rather than divergent — `union_border_boxes` (`elidex-ecs/src/dom/geometry.rs`,
  whose every caller sits below that file's `#[cfg(test)]`) and `elidex-css-box`'s
  `BoxHandler::resolve` `content` arm (`elidex-css-box/src/lib.rs:303`, no production caller). Per
  CLAUDE.md's unconditional *dead code は接続するか削除*, the disposition is connect-or-delete by
  whoever owns those crates, and a `#11-` slot against either would book platform work against
  code that should not exist — the same ruling the dead-arm bullet above already makes, applied to
  two sites this program only *reads* and therefore does not delete here.
  **Generic font families are an `elidex-text` bug with no connection to this program.** Cell
  24's fixture reasoning records that `FontDatabase::query` maps the generic
  keyword `sans-serif` to `fontdb::Family::SansSerif` (`elidex-shaping/src/database.rs:70`), which
  `fontdb` resolves to the single literal `"Arial"` and which nothing in the workspace re-points —
  measured by the property rather than the one keyword,
  `git grep -nE 'set_(serif|sans_serif|monospace|cursive|fantasy)_family' 154bac3f -- crates/`
  is empty, so the complement is measured and not merely unexamined. ⚠ It reads there as a live
  caveat inside a cell's fixture argument, which is the worst of both: **disposition — no slot,
  out of scope, owner named.** The generic-family mapping belongs to `elidex-shaping`'s font
  database and its `fontdb` configuration; nothing in the IFC, in M1's predicate or in any cell
  depends on it except as a reason those fixtures spell `TEST_FAMILIES` out in full, which they
  now do.
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
  PR**, not folded into this umbrella. ⚠ **A known defect in that task's scope** (ledger **A65**): §2's
  *Uncoupled pair* table is outside `plan-xcheck.py`'s §2 Pair population, so a pair missing from
  **both** tables passes — a false negative, the unsafe direction; deleting the table leaves the
  checker at rc 0. ⚠ **Not a `#11-` slot, and not an own deferral.**
  This is **skill infrastructure, not a platform gap**, and `.claude/skills/elidex-plan-review/SKILL.md`
  already sets the precedent for exactly this class: its own unbuilt §2 preflight hard-gate is held
  as a *standing maintenance note in the skill*, "deliberately NOT a `#11-*` platform slot per
  `feedback_defer-slot-eligibility-audit-at-create`: it fails the slot-fit audit". Registering a
  `#11-` slot here would put a tooling task in the platform SoT against that precedent — so it is
  written instead as a **standing maintenance note in `.claude/skills/elidex-plan-review/SKILL.md`**
  — landed on `main` with the two checkers in #518 (`4394af4c`), in the same durable home the precedent uses and for the same
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
  for both checkers (the memo-split question §9 books to this task, below), add the **clause-case
  check** below, and retire or rewrite the SKILL.md note that
  describes the pre-state.
  ⚠ **The clause-case check, stated as a predicate because the vocabulary form of it is what
  failed** (R15, from the sweep bullet above): the question is **not** "does this sentence contain
  a negation" — the R15 finding's own site carries none, §3's §7.3 row having phrased its gap
  positively as "trigger 1 of 3" while the clause it omitted appeared in the memo zero times, so
  no text harvest over the memo could reach it. The predicate is **"does this §3 row enumerate the cases of a
  spec clause, and is every case either delivered by an M-row or owned by a registered
  destination?"**, keyed on the **spec's** case count rather than on the memo's wording — which
  makes it a *citation-side* check, reading the clause through `webref` as `preflight.py` already
  does for the label, not a text-side one. Its failure mode is the one check 4 is blind to and
  §3's preamble names: a Touch that points somewhere real while the clause it quotes has cases
  the row never mentions. ⚠ It is **not** implemented here and the memo makes no claim of having
  been swept clean by it — the sweep bullet above is a seed produced by the failing harvest, and
  it says so. ⚠ **R20 ran this predicate by hand over the current scope, and that pass is the
  one-time manual discharge for it** — the enumeration-completeness audit two bullets above: 25
  sections, the standard for "covered" stated there, nine branches found, six new rows and three
  corrections. It changes what this task **inherits**, not what the task is: a hand pass is not
  re-runnable, does not re-fire when a row is added or a module is renumbered, and covered the
  scope as it stood at `09026f98` and no later one. ⚠ **And the audit is evidence for the
  citation-side shape rather than an argument for it**: three of its nine were rows whose
  *quotation* was truncated, so a predicate reading only the row's own text would have answered
  "enumerated" on all three — the clause reaches the check through `webref` rather than through
  the row, which is the whole of why it is keyed on the spec's case count.
  ⚠⚠ **The next checkable property beside it is a runtime-claim register, and R22's probe run is
  both the argument for it and the one-time discharge of the pass that named it.** That run
  executed **27** of this memo's runtime claims against `22de3078` — M7's `41.201`, ledger
  **A33**'s relpos rect pair and its `1 → 5` bordered count, A8's 0-before/0-after static count,
  every line-breaker output,
  cell 15d's `46.70 ≤ W < 55.14` width window and the font-resolution list beside it
  (Arial, Helvetica and Hiragino Sans resolving and the list's other three not, exactly as
  recorded) among them — and **26** agreed, most to the digit. The single correction is a *gloss*, not a
  figure: ledger **A15**, above. ⚠ **The blind spot is much larger than the attestation that
  disclosed it said.** The TERMINAL attestation put it at "roughly eight never-executed numeric
  claims"; R22's enumeration reports the memo carrying that property by the **hundred**, with
  tens of them phrased as a measurement of *today*. **No figure is carried here**, and that is
  the finding rather than a gap in it: the property — "this sentence asserts a number an engine
  run would produce" — has no command that returns its population, which is exactly why it is
  booked to a checker and not to another hand pass
  ([[feedback_convention-dependent-figures-are-argument]]).
  ⚠ **And the register cannot be keyed on the word "measured".** In this memo that word labels a
  *source*-level measurement far more often than an execution — a `grep` over `crates/`, a
  function read end to end, an enumeration of write sites — while several of the memo's real
  runtime figures (a `SolidRect` dump, a `line_count`, a `measure_width` window) carry no such
  word at all. A vocabulary harvest therefore over-fires and under-fires at once, the same
  failure mode the clause-case predicate above was rewritten to escape. The shape that survives
  it is **provenance**: a sentence that states an engine output carries the command that
  produced it and the commit it was produced at, and the check is that it does — which is
  testable on the memo's text without deciding what any sentence means. ⚠ The
  `SPEC_LABEL_REVERSE`
  CSS-label gap is **not** this task's: it is `#11-preflight-css-module-labels` in the SoT
  (citation-hygiene Slice B, after A-ii migrates the dict) — an earlier revision booked it here
  too, a second decision surface for one gap, in the direction that lane's open slice deletes;
  until that slot lands every CSS plan hand-verifies its citations (§3). ⚠ Until the wiring is
  done, **the per-PR plan-reviews under this umbrella (and this memo's landing-record revisions)
  depend on running the checkers by hand** — the habit the task exists to end.
  ⚠ **Trigger: #510's resolution — landing or closure — or this umbrella's TERMINAL, whichever
  comes first** (the bound "no later than the approval PR" was the files' dependency, discharged
  by #518 — below; the sibling checker substrate this
  task must decide against; if #510 is still
  open at TERMINAL the memo decides against `origin/main` as it then stands — another lane's PR
  must not hold this task past this program's terminal);
  **discharger: the tooling PR**, cut from `origin/main` (approval-independent skill infra, by this
  bullet's own classification), under `/elidex-plan-review` — not because it is edge-dense but
  because #506 shipped checker tooling without one and paid a tooling-only IMP tail across several external rounds (the SoT's #506
record; no count carried here) before being carved into #510. Earlier revisions keyed the trigger to "the PR that ships
  these files" and named the seam-3 prereq PR (which shipped neither), then PR-1a (which would
  land the checkers unconnected — dead code by CLAUDE.md's rule), then the tooling PR itself (a
  trigger that is the work cannot fire unfired); the event is "#510's resolution or TERMINAL" —
  TERMINAL is the disjunct that fired, 2026-09-07 — and the PR is the discharger. The same-day
  Terminator reset (two clean rounds owed) does **not** unfire it: the trigger is an event, the
  remaining tooling scope (the checkers' wiring, generalisation and two-file awareness — the files
  themselves landed as #518, §8) depends on no design round, and its own plan-review
  decides against `origin/main` as it then stands.
  Before all that, a revision wrote "this memo's next plan-review round", which fires *every*
  round and can discharge nothing, so it expired unheard six times running; a trigger that recurs
  is not a trigger.
  ⚠⚠ **The approval PR waits on this one, and that is new at R6** (R6-c; ledger **A21**): R5 made
  these two checkers the memo's stated verification mechanism — §3's coverage map produced by
  `plan-xcheck.py`, §5.3's defer count cross-checked by it, §10's routing errors caught by its
  check 13 — and neither file was on `origin/main`, so a memo-only approval PR would have landed
  re-runnable commands naming files the repository did not contain. §8's 順序 block carries the
  order; the approval PR's diff stays exactly one file, the remedy being the ordering and not a
  bundle. ⚠ **Discharged**: the files landed first as #518 (`4394af4c`, 2026-09-22), so the
  approval PR waits on nothing here; the remaining scope is ordered against no PR of this program.
  Re-eval: 2026-11-01.

## §10. Slot ledger actions at landing

| Action | PR |
|---|---|
| ✅ **Done 2026-09-07 (memory bookkeeping — no landing gate)**: this umbrella's slot and `#11-css2-spec-label-normalisation` (a #497 carve, **pre-existing** class; its Why/trigger are in `project_css2-spec-label-normalisation.md`, not restated here) are registered in `project_open-defer-slots.md` (the SoT per MEMORY.md), each with **its own recorded date rather than a fresh one** — `#11-css2-spec-label-normalisation` **2026-10-31** from its slot memo; this umbrella's own slot, which has none, takes 2026-11-01. This umbrella's own slot is **pre-existing** class: Codex opened it on #497, not this program. `#11-inline-fragmented-fn-decomposition` (carved from #495, pre-existing, **2026-10-28**) is **no longer part of this row** — #508 registered *and* partially closed it (next row); an earlier drafting of this row would have re-registered a closed slot with its pre-close date. ⚠ A registration is made true by the slot's *existence*, so routing it to a PR (the seam-3 prereq, then PR-1a, in earlier revisions) was deferral dressed as routing; the SoT's "they land with the umbrella" note is struck accordingly. | done (memory) |
| ✅ **Landed with #508 (`7e256029`)** — the one row #508 carried, as the bookkeeping its own change made true. Register **and** close `#11-inline-fragmented-fn-decomposition` in one row — it is registered as closed-on-landing, not registered then closed — **as a partial close**, naming the seams §9 measures as still open, in the successor slot `#11-inline-fragmented-fn-seams-1-2` (**mixed** class per the SoT's landing record — the seams predate this umbrella, but `reconcile_flows`' extracted signature is created by #508, which makes it that PR's one **(own)** deferral, §5.3; an earlier drafting said "pre-existing" on the seams alone; Why: the prereq PR discharges seam 3 only; **trigger: canonical in the slot memo's Trigger section — the disjuncts the slot memo carries, no count here (a count goes stale each time the slot adds one; the per-program memory says so) — and this row no longer restates it**: disjuncts 1–2 are the two this row originally prescribed (their text, the six-PR exemption and its two grounds, and the predicate carve-out now live in the slot memo, not here), and the slot added further disjuncts for the items 1–2 cannot reach. A verbatim restore of this row's old text would drop the disjuncts the slot added (🔴 per-program memory). ⚠ **Disjunct 3 (slot memo) exempts no umbrella PR**: it fired at #511, and no later umbrella PR re-fires it — none of PR-1a–1d edits `inline/reconcile.rs` (last row); re-eval 2026-11-01). | seam-3 prereq PR |
| Open `#11-inline-spec-cite-misattribution` (**pre-existing** class) with the classes §9 enumerates — defined by §9's concept greps and hand-offs, and counted only there, inside the enumeration that determines the figure (a count restated here is a second site that can drift from it; this row carried one until rev 31 added a hand-off and had to move it at three sites in one commit) — Why, trigger and date in §9 (re-tagged under the rule the `#11-inline-root-inline-box` row below states; an earlier drafting left this third pre-existing row at PR-1a — the rule was added and not swept, round 23, Axis 3) | approval PR |
| Open `#11-inline-box-decoration-splits` (own) with the Why / trigger / date in §5.3, **and the §9 note that its carrier choice (`FragmentTree` vs a widened `InlineClientRects`) belongs to terminal-Z C-3/C-4, not to the slot alone** | PR-1c |
| ✅ **Landed with #511 (`22de3078`, 2026-09-07)** — Close `#11-inline-align-clientrects-nonpersist-path` — the arm it books work against is deleted | dead-arm prereq PR |
| Note on `#11-layoutbox-absence-unreachable` (#488) that M5 withholds no entity's box, so this umbrella needs no truthful box-absent signal **for its mechanism** — ⚠ booked to **PR-1d**, not to the PR that first grants a box: the statement is M5's and is only true once the flip lands, whereas under PR-1c box-absence is still a live signal (§6 cell 6 asserts "phantom ⇒ no box", sound only under §8's first-layout scoping) | PR-1d |
| Correct `#11-layoutbox-trip-wire-not-in-ci` **at every live site carrying the falsified premise, not at one** — the class, per [[feedback_semantic-sibling-selfseed-and-regate-breadth]], is whatever `grep -rln 'layoutbox-trip-wire-not-in-ci\|D4 gate runs only in local' <memory-dir>` returns, **and the row states no count** — the command's output is the class, and the sites' *dispositions* differ, so any figure here would be a classification dressed as a measurement ([[feedback_convention-dependent-figures-are-argument]]; an earlier drafting of this row said "three" and the grep returns more). Verified to carry the falsified premise and needing correction: `project_open-defer-slots.md:25` (open + the premise), `active-lane-detail.md:140` (lists it as the Layout lane's **NEXT TASK**, user-approved) and `project_inline-mod-split-owed.md:49`/`:98` (still OPEN + the premise verbatim). Verified **already** correct and needing none: `project_layoutbox-trip-wire-in-ci-next.md` (CLOSED). The remaining hits are triaged at landing by re-running the grep, not from this list. ⚠ That the sweep is *partial* rather than uniformly pending is the whole reason a one-site row would leave a future Layout-lane session picking up a task #496 landed. ⚠ §10's separate `active-lane-detail.md` rewrite row is scoped to *this slot's* framing and does not reach line 140. The premise: "the D4 gate runs only in local `mise run ci`" — PR #496 (`da958ace`) made the `trip-wires` job ungated and unconditional, so the slot is resolved and the premise is false. Found because §8's PR-1c DoD now cites that job's wire #5 instead of restating a grep, i.e. this program **depends on** the fix; a dependency left asserting the opposite in the SoT is the class §10 exists to close. ✅ **Done 2026-09-07 (memory, no landing gate)** — the grep returned seven files; the live falsified-premise sites were corrected (SoT `:30` → CLOSED #496, `active-lane-detail.md:140`, `project_inline-mod-split-owed.md` — the queue line `:49` and section B's heading, `:99` today and `:98` when this row was first written, the same site renumbered — and `project_c3a-impl-pr-ready.md:29`) and the dated historical mentions left as records. ⚠ Routed to the seam-3 prereq PR, then PR-1a, by earlier revisions; its truth-maker (#496) had landed on 2026-08-02, so tying it to a program PR only lengthened the window in which `active-lane-detail.md:140` advertised a landed task as the lane's NEXT TASK. | done (memory) |
| Open `#11-inline-root-inline-box` (pre-existing) with the Why / trigger / date in §5.3 — whose **subject is css-inline-3 §5.3's layout-bounds model as composition, in five facets**, widened from "the root inline box" by Codex R2 (F1/F2/F4) and again by R4 (facet (e)), then narrowed at R22 from "the whole §5.3 model" so that the `normal` branch's per-box bounds stay `#11-inline-height-normal-layout-bounds`'s; the registration must carry the widened subject, since a slot opened on the narrow one would leave facets (b)–(e) owned by nothing; a slot opened on "the whole model" instead shares a subject and a trigger with `#11-inline-height-normal-layout-bounds`, which is the defect R22 found and §5.3 now states. ⚠ A pre-existing slot whose Why / trigger live only in this memo is registered when the memo is on `main` — the approval PR — not at a code PR (§10's first row: a registration is made true by existence; an earlier drafting routed this row to PR-1d and the next to PR-1b, the same deferral-dressed-as-routing that row withdraws). Own deferrals stay on the PR that opens them — the per-PR ≤3 count is measured at that landing | approval PR |
| Open `#11-inline-fallback-font-strut` (**pre-existing** class) with the Why / trigger / date in §5.3 — css-inline-3 §5.3's "only glyphs from fallback fonts" strut condition, which §8's closing paragraph used to fold into `#11-inline-root-inline-box`. Registered at the approval PR on the ground the row above states; **not** an own deferral, so it does not enter §5.3's per-PR count (the divergence is observable today on fallback-rendered text with no marker involved). ⚠ The same revision strikes the font-fallback-provenance disjunct from `#11-inline-root-inline-box`'s trigger, so the two slots do not both claim the condition — a slot registration that left the fold's surfacing disjunct standing would re-create the second decision surface it was opened to remove | approval PR |
| Open `#11-inline-decoration-paint-path` (**pre-existing** class, opened by R4) with the Why / trigger / date in §5.3 — the subject is that a **static** inline box's background and border are never emitted (`walk` never receives a non-positioned inline child; `emit_background`/`emit_borders` have two non-test callers; `InlineFlowRun` carries no chrome), which is why §6 cell 13's paint assertion is withdrawn rather than kept as a PR-1c obligation. Registered at the approval PR on the ground the `#11-inline-root-inline-box` row states; **not** an own deferral, so it does not enter §5.3's per-PR count (`origin/main` paints no static inline's chrome today, with no marker involved). ⚠ Its trigger names **PR-1c landing** as a disjunct — the geometry a render pass would draw is PR-1c's output — which is a readiness statement, not a dependency of this program on the slot | approval PR |
| Open `#11-inline-fontless-measurability-gate` (**pre-existing** class, opened by R4) with the Why / trigger / date in §5.3 — `inline/mod.rs:200`'s early return zeroes `line_count` for any fontless IFC, and PR-1d lifts it only behind `has_inline_axis_edge`, so a block-axis-only decorated inline keeps returning 0. Registered at the approval PR on the same ground; **not** an own deferral and not in §5.3's per-PR count (measured at `22de3078`: 0 with a block-axis edge, 0 with an inline-axis edge, 0 with none — the gate is blind to edges, so the residual predates the program and the partial lift worsens nothing). §6 cell 12d pins both sides | approval PR |
| Open `#11-writing-mode-inline-blockification` (**pre-existing** class, opened by R11) with the Why / trigger / re-eval in §5.3 — css-writing-modes-4 §3.2's first different-`writing-mode`-than-parent bullet, which computes an otherwise-`inline` box's display to `inline-block`, is unimplemented: measured by the property rather than the word, the four sites that rewrite a computed `display` toward a block-ish value are keyed on abs/fixed, float, flex items and grid items and none on `writing-mode`, and `display.blockify()` would give `block` where the rule asks for `inline-block`. Implementing it is an engine-wide computed-display change in `elidex-style`, orthogonal to decoration. Registered at the approval PR on the same ground as the rows above; **not** an own deferral and not in §5.3's per-PR count (the divergence is observable today with no marker involved). ⚠ The registration must carry the **retirement**: its discharge retires §6 cell 12f, whose markup a conforming engine makes an atomic | approval PR |
| Open `#11-intra-word-shaping-across-line-break` (**pre-existing** class, opened by R12) with the Why / trigger / re-eval in §5.3 — css-text-3 §5.5's "the characters must still be shaped … as if the word were still whole" across a break, unimplemented: `pack/mod.rs:744`'s coalescing is scoped within one line by explicit design (`flush_line` clears `last_placed_entity` at `:439`), and render shapes each line's runs independently (`builder/inline_flow.rs:107-124` → one rustybuzz call per run), so the two halves of a broken word take the wrong joining forms. Reachable with no unimplemented property, the engine's opportunities coming from UAX #14 whole: `linebreaks("نوش\u{00AD}تن")` returns an interior `Allowed` break at a joining-transparent SOFT HYPHEN. Closing it is `elidex-shaping` / `elidex-text` work, orthogonal to decoration. Registered at the approval PR on the same ground as the rows above; **not** an own deferral and not in §5.3's per-PR count (the divergence is observable today with no marker involved). ⚠ The registration must carry the **evidence limit**: the measurement is structural plus that probe, with no Arabic fixture rendered end-to-end, so the slot's first step is building one | approval PR |
| Open `#11-inline-item-boundary-soft-wrap` (**pre-existing** class) with the Why / trigger / date in §5.3 (found by Codex on #515; cell 15 stops pinning the divergence; registered at the approval PR on the ground the row above states) | approval PR |
| Open `#11-shaping-break-at-unchanged-inline-boundary` (**pre-existing** class, opened by R15) with the Why / trigger / re-eval in §5.3 — css-text-3 §7.3's **second** normative sentence, "Text shaping must not be broken across inline box boundaries when there is no effective change in formatting, or if the only formatting changes do not affect the glyphs (as in applying text decoration)", unimplemented and unimplementable on the current mechanism: a text run carries the **parent element**'s entity (`collect.rs:309`), so `<p>a<span>x</span>c</p>` is three runs, and both measurement and shaping are per-run (`inline/measure.rs:57`, `:78`; one rustybuzz call per `InlineFlowRun::Text`, `builder/inline_flow.rs:107-124`). Reachable on an unstyled `<span>` with no CSS at all. Closing it is `elidex-shaping` / `elidex-text` work plus a formatting-identity comparison layout does not have, orthogonal to decoration; PR-1b's boundary break runs in the *first* sentence's direction and neither creates nor narrows this. Registered at the approval PR on the same ground as the rows above; **not** an own deferral and not in §5.3's per-PR count. ⚠ The registration must carry the **evidence limit**: the measurement is structural — the run split and the per-run shaping call traced, no joining-script fixture rendered end-to-end — so the slot's first step is building one | approval PR |
| Open `#11-shaping-break-vertical-align-and-isolation` (**pre-existing** class, opened by R15) with the Why / trigger / re-eval in §5.3 — css-text-3 §7.3's first sentence lists three triggers and elidex implements none of them as triggers, over-covering all three wherever the box places content; in the member-less corner only trigger 1 closes, by PR-1b's marker, so a `vertical-align`-only or isolation-only box keeps its two texts coalesced. Registered at the approval PR on the ground the `#11-inline-item-boundary-soft-wrap` row states; **not** an own deferral and not in §5.3's per-PR count (observable today with no marker involved) | approval PR |
| Open `#11-replaced-inline-no-atomic-layout` (**pre-existing** class, opened by R15) with the Why / trigger / re-eval in §5.3 — a replaced element computing `display: inline` (the ordinary `<img>`) gets no inline layout: `is_atomic_inline` (`inline/collect.rs:14-19`) answers from display keywords only, so the IFC recurses into an element with no children and emits nothing, and `<p>a<img src=x>b</p>` advances the cursor by zero with no `LayoutBox` from the inline path. The inline module reaches no replaced element at all (one hit for `replaced` / `ImageData` / `get_intrinsic_size` under `src/inline/`, the doc comment at `inline/styled_run.rs:12`), while the sizing sits in `block/replaced.rs`, reachable only from `block/mod.rs`. Closing it is atomic-inline layout work, orthogonal to decoration. Registered at the approval PR on the same ground as the rows above; **not** an own deferral and not in §5.3's per-PR count. ⚠ The §9 predicate-prereq bullet reasoned about this gap and named no destination until R15 (ledger **A42**); what that PR delivers is the classification, not the layout | approval PR |
| Open `#11-inline-atomic-intrinsic-contribution` (**pre-existing** class, opened by R27) with the Why / trigger / re-eval in §5.3 — both intrinsic passes skip `InlineItem::Atomic` (`inline/measure.rs:13-14`, `:42-43`), so an atomic inline contributes zero to min- and max-content while layout advances by its margin box; css-sizing-3 §2.2 makes the contribution the atomic's outer size. Trigger: the min-content prereq PR's plan-review. | approval PR |
| Open `#11-shaping-no-last-resort-font` (**pre-existing** class, opened by R16) with the Why / trigger / re-eval in §5.3 — css-inline-3 §5.3's strut takes "the metrics of the box's first available font", and elidex has no font that is always available: `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60-84`) ends `self.db.query(&query)` and answers `None` when the family list matches nothing, with no last-resort family appended anywhere in the call. Reachable from any `font-family` that resolves to nothing, text or not — the same `None` reaches `measure_text` (`elidex-shaping/src/measurement.rs:54`). Closing it is `elidex-shaping` font-discovery work, orthogonal to decoration. Registered at the approval PR on the same ground as the rows above; **not** an own deferral and not in §5.3's per-PR count (ordinary text in an unavailable family already renders as nothing, with no marker involved). ⚠ The registration must carry what makes it **distinct from the two adjacent font slots** — `#11-inline-fontless-measurability-gate` is the text-driven early return this case skips, `#11-inline-fallback-font-strut` is §5.3's second condition, which presupposes glyphs — since a slot filed as either would be closed by work that leaves this open (ledger **A45**) | approval PR |
| Open `#11-shaping-across-formatting-change-boundary` (**pre-existing** class, opened by R20) with the Why / trigger / re-eval in §9 — css-text-3 §7.3's **third** normative sentence, "Text shaping should not be broken across inline box boundaries otherwise, if it is reasonable and possible for that case given the limitations of the font technology", which the memo counted out of existence until R20 (the section has three sentences, `body css-text-3 boundary-shaping` lines 9 / 39 / 63). The registration carries that the predicate is the **complement** of `#11-shaping-break-at-unchanged-inline-boundary`'s, so the two slots are not one slot restated, and that the case is reachable with zero author CSS through the UA sheet's `b, strong { font-weight: bolder; }`. Registered at the approval PR on the ground the rows above state; **not** an own deferral | approval PR |
| Open `#11-inline-height-normal-layout-bounds` (**pre-existing** class, opened by R20) with the Why / trigger / re-eval in §9 — css-inline-3 §5.3's **`line-height: normal`** branch, the one an author-`line-height`-free document is in, which elidex never enters because `LineHeight::Normal` collapses to `font_size * 1.2` at computed-value time. The registration carries **why it is not a sixth facet of `#11-inline-root-inline-box`** (that slot's five facets are each pinned by a §6 cell and none turns on per-glyph provenance), so the two are not re-merged later by someone reading only the section number — **and the R22 boundary beside it**: the sibling's scope and trigger are the *composition* of per-box bounds, this one's is their computation under `normal`, each naming the other's discharge as a trigger disjunct (§5.3, §9). Registered at the approval PR; **not** an own deferral | approval PR |
| Open `#11-inline-baseline-alignment` (**pre-existing** class, opened by R20) with the Why / trigger / re-eval in §9 — css-inline-3 §2.2 **step 1**, which the table covered only at the section's Note until R20: the IFC aligns nothing, `vertical_align` occurring at no site under `crates/layout/elidex-layout-block/src/inline/` and `dominant-baseline` having no elidex selector. The registration carries that it is **upstream** of §2.2 steps 2–4 and so of M6 and M7, and that it is distinct from `#11-shaping-break-vertical-align-and-isolation`, which owns the same property as a shaping trigger. Registered at the approval PR; **not** an own deferral | approval PR |
| Open `#11-pseudo-generation-on-replaced-originating-element` (**pre-existing** class, opened by R20) with the Why / trigger / re-eval in §9 — css-pseudo-4 §4.1's suppression clause, unimplemented: `generate_pseudo_entity` and its call sites test the cascade, `content` and `display` and never replacedness. The registration carries that the **replacedness half** of the canonical predicate the prereq PR establishes is this slot's **input** and not its owner — the `#11-resize-observer-inline-empty-content-rect` shape — and that it is a **further** consumer of that predicate beside the two §9's canonical-predicate bullet enumerates. ⚠ The registration reads *replacedness*, not the composed inline-box predicate (R22): the composite is false for a non-replaced atomic origin, whose generated content §4.1 leaves alone. Registered at the approval PR; **not** an own deferral | approval PR |
| Open `#11-used-direction-text-orientation-upright` (**pre-existing** class, opened by R20) with the Why / trigger / re-eval in §9 — css-writing-modes-4 §6.4's Note, under which `text-orientation: upright` in a vertical writing mode forces the **used** `direction` to `ltr`, while elidex derives no used direction anywhere and every `LogicalEdges::from_physical` caller reads the computed one. The registration carries that the scope is **engine-wide** rather than M1's, so it is not discharged by a change to the marker payload alone. Registered at the approval PR; **not** an own deferral | approval PR |
| Open `#11-line-box-float-narrowing` (**pre-existing** class, opened by R20) with the Why / trigger / re-eval in §9 — css-inline-3 §2.1's float clause (CSS 2 §9.4.2 / §9.5): `flush_inline_run` hands the IFC the whole containing width and no float context reaches the inline module at any production site, so no line box is ever narrowed. The registration carries the audit's **reachability caveat** — the anonymous blocks a float sibling produces go through the *same* entry point, so wrapping is not what removes the band — because that is the part a reader is likely to get backwards. Registered at the approval PR; **not** an own deferral | approval PR |
| Open `#11-inline-zero-edge-box-in-item-stream` (own) with the Why / trigger / date in §5.3 — the cells it flips named there (found by Codex on #515) | PR-1a |
| Open `#11-block-in-inline-anonymous-block-split` (**pre-existing** class, opened by R8) with the Why / trigger / re-eval in §9 — CSS 2 §9.2.1.1's break of an inline around an in-flow block-level box, unimplemented on the inline path, which this program makes observable in the decorated-**outer** case (§6 cell 6i) without creating it. Registered at the approval PR on the ground the `#11-inline-root-inline-box` row states; **not** an own deferral, so it does not enter §5.3's per-PR count (the flattening is what the IFC does today with no marker involved). ⚠ Before R8, §6 cell 6e disclaimed this behaviour and handed it to "§9" with no §9 entry behind it — the same defect shape §9's `<br>`/`<wbr>` bullet records for rev 32, which is why this row exists rather than a bare disclaimer | approval PR |
| Open `#11-resize-observer-inline-empty-content-rect` (**pre-existing** class, opened by R9) with the Why / trigger / re-eval in §9 — resize-observer-1 §3.3.1's "non-replaced inline Elements will always have an empty content rect", unimplemented in the observer's size source, which this program does not create (measured: an ordinary `<p><span>Hi</span></p>` span already reports a non-empty `content_rect_local()`) but whose reach it extends to previously box-less targets at PR-1c (§6 cell 14c) and PR-1d (§6 cell 21). Registered at the approval PR on the ground the `#11-inline-root-inline-box` row states; **not** an own deferral, so it does not enter §5.3's per-PR count. ⚠ Before R9 the rule was quoted in §7 and nowhere routed — no §3 row, no slot, no §9 disposition — the same hand-off-to-nothing shape R8-b's row above records | approval PR |
| Open `#11-inline-open-box-strut-on-continuation-line` (**pre-existing** class, opened by R10) with the Why / trigger / re-eval in §5.3 — css-inline-3 §5.3 gives the strut to the **box**, and PR-1d records M7's tentative at `InlineBoxStart` only, so a box open across a break offers no strut to its continuation lines. R7-a had M4's flush hook re-record it and R10 withdraws that half, on the measurement §5.3 carries: no break css-text-3 §5.5 licenses opens a continuation line without rendered text, so the promote it feeds has no input a cell may use and the value would have no reader. Registered at the approval PR on the ground the `#11-inline-fallback-font-strut` row states; **not** an own deferral, and by a stronger form of the test than that row's — not "observable today" but **not distinguishable in either direction**, so it does not enter §5.3's per-PR count. ⚠ Its trigger is deliberately not `#11-inline-item-boundary-soft-wrap`, whose discharge removes the one (unlicensed) route that reaches the state rather than creating one | approval PR |
| **Close `#11-line-box-decorated-inline-content`** — §5.3 and §8 both assert it, and until now no ledger row carried it | PR-1d |
| The plan-checker standing maintenance note is **already written** into `.claude/skills/elidex-plan-review/SKILL.md` — landed on `main` with the two checkers in #518 (`4394af4c`, 2026-09-22) — not booked for landing — an earlier trigger, "the next plan-review round that runs them by hand", fired every round and discharged nothing, and a landing-scoped row would have left it unowned in exactly the window it matters (its trigger is now #510's resolution or TERMINAL, §9). **Not a `#11-` slot** — skill infra, per that file's own slot-fit precedent. This row records it; what it still routes to the **tooling PR** is the note's retirement or rewrite (§8, §9) — the note lives in SKILL.md, a tooling file. ⚠ Routed to the seam-3 prereq PR, then PR-1a, by earlier revisions; #508 shipped neither the note nor the tools, and PR-1a would have landed them unconnected — the retirer is the §9 task's own PR, on its §9 trigger (#510's resolution or TERMINAL) and under its own plan-review. | tooling PR |
| Split the joint "fold into terminal-Z C-3/C-4" parenthetical shared by `#11-inline-align-clientrects-nonpersist-path` and `#11-inline-relayout-box-staleness` — this PR closes the first, so the pairing stops holding **here**, and leaving it to a later PR would strand the SoT asserting a fold against a closed slot — ✅ **landed with #511 (2026-09-07)** | dead-arm prereq PR |
| Enrich `#11-inline-relayout-box-staleness`'s SoT entry with M4's write-path statement (§9), and note that `#11-inline-box-decoration-splits`'s work is first-layout-only until this slot lands — an **ordering** note, not a blocking dependency (§5.3). ⚠ R12/R13/R15: the slot is **discharged by a prerequisite PR** ahead of PR-1c (its own `layout_generation` remedy being inert off the paged path), so this row's scope is the splits facet alone | PR-1c |
| Rewrite `project_line-box-decorated-inline-content.md`, `MEMORY.md`'s Layout-lane entry and `active-lane-detail.md`, all of which still carry a superseded framing of this slot. ⚠ Re-tagged from `seam-3 prereq PR` under the 2026-08-16 narrowing (§8); the per-program memory file is maintained round by round meanwhile, so what remains for the approval PR is the framing in `MEMORY.md` / `active-lane-detail.md` and the final state of the per-program file | approval PR |
| Record the successor program the close hands off to: `#11-inline-box-decoration-splits` and `#11-inline-zero-edge-box-in-item-stream` **arm at PR-1d landing** (the splits slot on the first disjunct of its trigger; §5.3 gives C-3b as the other; the zero-edge slot on the first disjunct of its trigger, §5.3) — ⚠ **two, not three: `#11-inline-min-content-box-edges` was the third until rev 53 and is withdrawn** (ledger **A47**; its edge term is PR-1b's own and the joining it needs a prerequisite ahead of PR-1b, §8, **A58**), so it arms nothing here — and `#11-inline-fragmented-fn-seams-1-2` **does not arm at the close** — none of PR-1a–1d edits `inline/reconcile.rs` (PR-1d's `clear_inline_flows` item is a path consequence, §5.2/§7) and disjunct 1 is self-exempted. ⚠ It is **already armed**, independently of this program: its disjunct 3 fired at #511 (2026-09-07), the program's only `reconcile.rs` touch, so the reshape is unblocked now and ordered against nothing here — a Layout-lane task in its own right with its own plan-review (the signature's ECS question), re-eval 2026-11-01. Two earlier dispositions are withdrawn on the record: "neither of its disjuncts fires" (counted the disjuncts this memo prescribed, not the ones the slot carries) and "scheduled after PR-1d" (anchored to a `reconcile.rs` write PR-1d does not make — round 20, Axes 2/3). The Layout lane's next-task choice is therefore among **three**: the two slots that arm here and the already-armed successor | PR-1d |
