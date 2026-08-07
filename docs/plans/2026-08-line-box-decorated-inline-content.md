# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's task, user-approved 2026-08-02.
Edge-dense ⇒ every PR under this umbrella goes through `/elidex-plan-review` before
implementation, and external review runs `/external-converge` from round 1.

All premises verified against `154bac3f`; every spec quote below was resolved directly with
`.claude/tools/webref`, not carried from a reviewer or from a code comment.

**Revision 8.** CRIT counts by round: 0 / 2 / 8 / 11 / 10 / 11 / 5. The turn came at revision 6,
which changed the *method* before the design; revisions 7–8 applied it. Round 5 named the first
half of the defect and round 6 the second: each
revision fixed the site a finding *quoted* and left every other site restating the same decision
untouched, then recorded the finding closed. Revision 6 therefore changed three things about the
document before changing anything in it:

1. **Review history is out** — it lives in `project_line-box-decorated-inline-content.md`, because
   a past-tense ledger restates the normative decisions and then drifts from them.
2. **One normative site per decision**, in §5.1's M-table; §3 / §5.2 / §5.3 / §6 / §7 / §8 / §9
   *reference* an M-row and add only their own facet.
3. **Two checkers run before every dispatch**, both in `.claude/tools/`:
   `plan-sweep.py <memo>` (word-level, 11 decision families — every hit re-derived, not just the
   quoted one) and `plan-xcheck.py <memo>` (parses the routing tables and cross-validates them:
   pairs ↔ M-rows, PR columns in §2/§3 ↔ §5.3, routed rows ↔ full-enum marks, §6 cells ↔ both
   §5.3's and §8's DoD ranges in both directions, slots ↔ §10 ledger rows, own-deferral count ↔
   tagged slots, superseded line-ranges ↔ their corrections). Round 7 mutation-tested the second and named
   five blind spots. Re-running the extended checker on the immediately preceding commit
   (`git show 344c7b42:… > /tmp/r7.md && python3 .claude/tools/plan-xcheck.py /tmp/r7.md`) prints
   `3 contradiction(s)`: one `[CELL]` and two `[RANGE]`. ⚠ Revision 8 described that as "two
   cell-range disagreements", which the output does not say. Round 8 re-tested and found blind
   spot 4 (§5.3↔§8 ranges) only partly closed — the §5 harvest is a union over the whole section,
   so a per-PR reassignment still passes. Treat the checker as covering the *tabular* surface only.
   **Neither tool is on CI yet** — see §9.

⚠ Authoring rules in force: **sweep, don't patch** · **re-derive every ground taken from a
reviewer, including the scope of its citation** (§9 records what not doing this cost) · **no
finding is marked closed without naming the sweep that closed it**.

---

## §1. The rules, from the current module

### §1.1 The inline layout model — `css-inline-3` §2

`body css-inline-3 model`:

> The block container also generates a **root inline box**, which is an anonymous inline box
> that holds all of its inline-level contents. … The root inline box inherits from its parent
> block container, but is otherwise unstyleable.

> **Inline-axis** margins, borders, and padding are respected between inline-level boxes (and
> their margins do not collapse).

Two things follow immediately, and both correct earlier revisions:

* The edges that take space on a line are the **inline-axis** ones. CSS 2 §9.4.2's
  "Horizontal margins, borders, and padding are respected" is the same rule written before
  writing modes existed; revisions 1–3 read "horizontal" as physical and built a
  physical→logical question that does not exist in the current text.
* CSS 2's **line-box strut** — "an imaginary box … a zero-width inline box with the element's font
  and line height properties", CSS 2 §10.8.1 — **is the root inline box** in the current module — a
  real box the block container generates, not a special case. ⚠ CSS 2 §10.8.1 uses "strut" for a *second*
  object in the same section — the invisible glyph inside a **glyphless inline box**, which is what
  §1.5, M6 and M7 mean. The §-number does not discriminate them; this memo says "root inline box"
  for the line-box one and "strut" only for the glyphless-box one. That reframes §5.2's gap: elidex is not missing an
  ad-hoc height floor, it is missing a box.

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

**Clause 3 counts inline-axis edges only.** A `padding-top`-only inline leaves the line
phantom. For **margins** this is independently confirmed by CSS 2 §8.3 ("vertical margins will
not have any effect on non-replaced inline elements") — so revision 3's recorded "tension" was
an artefact of reading the 1998 clause as axis-agnostic. ⚠ §8.3 is titled *Margin properties*
and says nothing about padding or border; the authority for **those** not entering line-box
sizing is `css-inline-3` §5.3, whose layout-bounds inflation by margin/border/padding applies
only "when `line-fit-edge` is not `leading`", and `css css-inline-3 line-fit-edge` gives
`initial: leading`. Revision 4 over-extended §8.3 to all three; corrected here.

### §1.3 Line breaking — `css-text-3` §5.5

`body css-text-3 line-break-details`:

> Out-of-flow boxes and **inline box boundaries do not introduce a forced line break or soft
> wrap opportunity** in the flow.

An inline box's edges take inline-axis space, but the boundary is **not** a break opportunity.
What happens to a box that does not fit is **split**, not overflow — `body css-inline-3
line-boxes` (§2.1): "When an inline box exceeds the logical width of a line box, **or contains a
forced line break, it is split** (see CSS Text 3 §5 …) into several fragments, which are
partitioned across multiple line boxes." Splitting happens at opportunities *inside* the box,
never at its edges. Overflow is the **exception**, for a box with no internal opportunity —
CSS 2 §9.4.2: "If an inline box **cannot be split** … then the inline box overflows the line
box." ⚠ Revision 4 first asserted the unconditional overflow form; corrected here.

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
already applies at `crates/layout/elidex-layout-block/src/inline/pack/mod.rs:581`. Note the
condition is broader than CSS 2 §10.8.1's ("no glyphs at all"), and the *root inline box* gets
special treatment in the same section.

### §1.6 White-space collapsing across boundaries — `css-text-3` §4.1.1

`body css-text-3 white-space-phase-1`, Phase I step 4:

> Any collapsible space immediately following another collapsible space — **even one outside
> the boundary of the inline containing that space**, provided both spaces are within the same
> inline formatting context — is collapsed to have zero advance width. (It is invisible, but
> retains its soft wrap opportunity, if any.)

This is the direct statement that an inline box boundary does not stop collapsing. Revision 4
grounded M2 on CSS 2 §16.6.1, which supports it only by implication and is the superseded text.

## §2. Coupled invariants

| # | Invariant | Site |
|---|---|---|
| 1 | **Line existence** — §2.3's five clauses | `any_rendered_content` (`inline/pack/mod.rs:115`) |
| 2 | **Item-stream integrity** — cross-run collapse lookback; positional iteration | `inline/whitespace.rs:59`; `measure.rs`, `atomic.rs`, `pack/items.rs` |
| 3 | **Commit-on-content** — per-line rects/runs commit or discard | `inline/pack/mod.rs:116`, `:210` vs `:423` |
| 4 | **Baseline provenance** — §5.3 strut vs glyphs | `inline/pack/mod.rs:575` |
| 5 | **Line-box height composition** — §5.3 layout bounds, incl. the root inline box | `inline/pack/mod.rs:696` |
| 6 | **Group keying** — relpos/sticky sub-flows | `inline/collect.rs:286` |
| 7 | **Cursor/advance integrity** — wrap, trailing hang, shaping runs, intrinsic size | `inline/pack/mod.rs:690`, `:701`, `:772`; `inline/measure.rs:15`, `:44` |

| Pair | Coupling | Resolved in | PR |
|---|---|---|---|
| 2 × (all) | Which inlines emit markers sets the blast radius of everything else. | M1 | 1a |
| 1 × 2 | A marker between two text runs must not become a collapse barrier. | M2 | 1a |
| 7 × 2 | The box's edges take space but its boundary is not a wrap opportunity; and a decorated boundary must break shaping. | M3 | 1b |
| 3 × 7 | The box's rect must be a *content* span, with edges carried separately, or the border box double-counts. | M4 | 1b |
| 1 × 3 | Flipping existence moves whole lines from discard to commit, affecting **every** entity on the line. | M5 | 1c |
| 1 × 5 | A line kept only by decoration needs a height source, and elidex has no root inline box. | M6 | 1c |
| 1 × 4 | §5.3 gives a strut only to a glyphless (or fallback-only) box, so the baseline source depends on the whole line. | M7 | 1c |
| 2 × 6 | Markers carry the enclosing recursion level's `group_key` — read only by the split rule, so the field arrives with its reader. | M1 | `#11-inline-box-decoration-splits` |
| 7 × (intrinsic) | An inline-axis advance the packer applies must also be visible to the intrinsic-size passes, which never build a `LinePacker`. | M8 | 1b |

Each **pair** is answered by exactly one M-row. M-rows are **not** partitioned by PR — M1's payload
is split across PR-1a, PR-1c and the splits slot by §8's dead-field rule, and pair 2×6 routes to a
slot — and the invariant *sets* overlap by construction, because PR-1c consumes PR-1b's geometry.
What §5.3's slicing rests on is narrower and true: each **coupling** has one owning PR, and each PR
is behaviour-scoped (item stream / geometry / existence). ⚠ Revisions 6–8 claimed "three PRs,
disjoint invariant sets"; the table above never supported it.

**Withdrawn in revision 2**, kept for the record: a claimed "flow-line ordinal indexing"
invariant. Code-contradicted — `InlineFlowLine` carries absolute `block_start`/`block_size` and
render iterates by coordinate (`crates/core/elidex-render/src/builder/inline_flow.rs:111`);
`flush_line` already drops member-less buckets
(`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:346`) and `slice_and_rebase_fragment`
retains only non-empty ones (`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs:63`).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS Inline 3 §2 Inline Layout Model | inline-axis edges respected between boxes | start/end edge advance | M3 shared core (`note_line_occupancy`, NEW) — **PR-1b** | ✓ | yes |
| CSS Inline 3 §2 Inline Layout Model | root inline box | block container's anonymous inline box | **NOT implemented — M6, `#11-inline-root-inline-box`** | ✗ (pre-existing, disclosed) | yes |
| CSS Inline 3 §2.1 Layout of Line Boxes | box exceeding the line, or containing a forced break | split into fragments across line boxes | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS 2 §9.4.2 Inline formatting contexts | box that **cannot** be split | overflows the line box | M3 — no wrap check on the marker path; §6 cell 15 | ✓ | yes |
| CSS Break 3 §5.4 Fragmented Borders and Backgrounds: the box-decoration-break property | `box-decoration-break: slice` | "no border and no padding are inserted at a break" (the "no visual effect where the split occurs" phrasing is CSS 2 §9.4.2's, not this section's) | **`#11-inline-box-decoration-splits`** | ✗ (deliberate) | yes |
| CSS Inline 3 §2.2 Layout Within Line Boxes | Note on empty inline boxes | they still have a line-height and influence the calculation | M6 — **PR-1c** | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence (a) | zero-height for positioning descendant content (abspos) | `static_positions` (`inline/pack/mod.rs:97`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | layout-bounds inflation by edges | applies only when `line-fit-edge` ≠ `leading`; initial is `leading` | no code touch; grounds §1.2's block-axis exclusion | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 3 | non-zero **inline-axis** margin/padding/border | clause-3 predicate → `any_rendered_content` (`inline/pack/mod.rs:698`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 5 | forced line break | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence | line box *and its in-flow content* do not exist | commit/discard seam (`inline/pack/mod.rs:210` vs `:423`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | glyphless / fallback-only box | strut with first-available-font metrics | tentative baseline — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | half-leading | `A′ = A + L/2` | existing formula (`inline/pack/mod.rs:581`) — unchanged | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | break opportunities | inline box boundary is **not** one | M3 — the marker path calls the shared core without a wrap check — **PR-1b** | ✓ | yes |
| CSS Text 3 §5.5 Line Breaking Details | adjacent soft wrap opportunity | break lands at the box's **margin edge** | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | max-content | inline-axis edges occupy space | M8 (`inline/measure.rs:44`) — **PR-1b** | ✓ | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | min-content | no accumulator to attach edges to | **`#11-inline-min-content-box-edges`** | ✗ (deliberate, M8) | yes |
| CSS Text 3 §7.3 Shaping Across Element Boundaries | shaping break | trigger 1 of 3: non-zero inline-axis edge | `last_placed_entity` coalescing (`inline/pack/mod.rs:744`) — **PR-1b** | ✗ (triggers 2 `vertical-align` ≠ baseline and 3 bidi isolation boundary unimplemented; neither is this umbrella's subject) | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | steps 3–4 | a collapsible space is line-final only if nothing follows it on the line | `current_line_last_hang` (`inline/pack/mod.rs:701`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §4.1.1 Phase I: Collapsing and Transformation | step 4 | collapsing crosses inline box boundaries | `collapse_inline_whitespace` (`inline/whitespace.rs:41`) — **PR-1a** | ✓ | yes |
| CSS 2 §10.8.1 Leading and half-leading | glyphless inline box | strut with first-available-font metrics (superseded by css-inline-3 §5.3, kept as the historical anchor) | M7 — **PR-1c** | ✓ | yes |
| CSS 2 §8.3 Margin properties: margin-top, margin-right, margin-bottom, margin-left, and margin | non-replaced inline elements | vertical margins have no effect | consistent with §2.3 clause 3; no code touch | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline; sub-flow keying | `collect.rs:286` — **`#11-inline-box-decoration-splits`** (the `group_key` field arrives with its only reader, §2 pair 2×6) | ✗ (deliberate, §5.3) | yes |
| CSS Box Model 3 §3.1 / §4.1 | percentage margin/padding | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract Dimensions | inline size ≡ logical width | basis identity in vertical modes | same | ✓ | yes |
| CSS Backgrounds 3 §3.2 Line Patterns: the border-style properties | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time (`elidex-style resolve/box_model/mod.rs:260`) | ✓ | yes |

**Breadth**: K=8 specs (CSS Inline 3, CSS Text 3, CSS 2, CSS Break 3, CSS Box Model 3,
CSS Sizing 3, CSS Writing Modes 4, CSS Backgrounds 3), M=26 entries (verified 2026-08-02 — data rows counted
directly above).
**Split decision**: K=8 ⇒ SPLIT-DEFAULT. The plan **is** split into three shipping PRs on
disjoint invariant sets (§2's rightmost column, §5.3); the breadth verdict and the
invariant-axis verdict agree.

### §3.1 User-input touch audit

Every row is reachable from author CSS/HTML. Adjacent pre-existing laxity:

* `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:82` hard-codes
  `EdgeSizes::default()` on every inline `LayoutBox`. **PR-1b fills it** (§5.1 M4) — for
  *every* decorated inline, not only the empty ones this slot names, because that is the gap
  §4.3 identifies and it is what makes the PR-1c flip honest rather than half-true.
* **Root inline box absent** (`grep -rni strut crates/` → nothing in inline layout, verified
  2026-08-02): pre-existing, newly *depended on* by PR-1c. Disclosed in M6 with its own slot.
* **`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:726` cites "CSS Text 3 §5.6
  Shaping Across Intra-word Breaks"** — the section number and the title are **both**
  fabricated (`heading css-text-3 5.6` → no headings). ⚠ The correct target is **css-text-3 §5.5**,
  not §7.3: that comment documents *intra-word* coalescing so render shapes whole words, and §5.5
  is where that rule lives ("the characters must still be shaped … **as if the word were still
  whole**"). §7.3 is the opposite-direction rule M3 uses (shaping must **break** at a decorated
  boundary). Revisions 5 and 6 both aimed the correction at §7.3. PR-1b rewrites the comment, so
  both land: §5.5 for the surviving intra-word coalescing, §7.3 for the new boundary break.
* **A nonexistent "CSS Box Model L3/Level 3 §5.3" is cited at seven sites** (css-box-3 §5 is
  *Borders*, no subsections). Verified 2026-08-02 via the **concept** grep
  `grep -rEn "Box Model (L3|Level 3)[^a-z]*(§)?5\.3" crates/` → 7 hits, all in
  `elidex-layout-block`: `lib.rs:178`, `helpers.rs:59`, `helpers.rs:114`,
  `positioned/layout.rs:90`, `block/mod.rs:162`, `block/mod.rs:182`,
  `block/children/helpers.rs:213`. PR-1a corrects **all seven** to css-box-3 §3.1/§4.1 in one
  commit per [[feedback_semantic-sibling-selfseed-and-regate-breadth]].
  ⚠ This class has been undercounted three rounds running — R2 said 2, R3 said 4, revision 4
  said 4 "verified" — each time because a **string** grep (`"Box Model L3"`) was run where the
  lesson being cited in the same sentence prescribes a **concept** grep. The three missed sites
  spell `Level 3`.

## §4. Verified current state

* `any_rendered_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:115`) is
  written at four sites: `place_item` `|=` (`:698`), `force_break` `= true` (`:781`),
  `flush_line` reset `= false` (`:435`), constructor (`:184`).
* `place_item` (`:679`) writes nine pieces of state: soft-wrap flush (`:690`), `current_inline`
  (`:695`), `current_line_height` (`:696`), `on_line` (`:697`), `any_rendered_content` (`:698`),
  `current_line_last_hang` (`:701`), `current_line_entity_rects` (**`:706`**, guarded by
  `entity != self.parent_entity` at `:703`), the flow-member bucket (`:736-769`, requiring a
  `FlowMember` — `inline/pack/items.rs:49`), `last_placed_entity` (`:772`).
* `flush_line`'s per-line reset (`:432-439`) covers `current_inline`, `current_line_height`,
  `current_line_last_hang`, `any_rendered_content`, `last_placed_entity` — **not `on_line`**,
  which is reset only in `force_break` (`:783`). `place_item` is safe only because `:697-698`
  re-establish state immediately after its own flush. Any new caller of `flush_line` inherits
  that obligation.
* `StyledRun` (`inline/styled_run.rs:43`) carries no margin/padding/border member.

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

`assign_inline_layout_boxes` hard-codes zero edges (`inline/pack/boxes.rs:82`) and the packer
never advances for an inline box's edges. css-inline-3 §2's "Inline-axis margins, borders, and
padding are respected between inline-level boxes" is unimplemented engine-wide. **This is why
the slot is an umbrella, and why §5.3 puts geometry before the existence flip**: the geometry
gap is visible today on ordinary `<span style="padding:10px">text</span>`, independently of
whether any line is phantom.

## §5. Design

### §5.1 Mechanism table

| # | Question | Decision | Grounds |
|---|---|---|---|
| **M1** | What enters the item stream, carrying what? | Two `InlineItem` variants, `InlineBoxStart` / `InlineBoxEnd`, emitted around the recursion at `inline/collect.rs:291` for every inline with **at least one non-zero edge on any side**. Payload: `entity` + the **three physical `EdgeSizes`** `resolve_box_model(&style, containing_inline_size)` returns (`helpers.rs:116`) + the `WritingModeContext` they were resolved under (`WritingModeContext::new(root.writing_mode, root.direction)`, `logical.rs:27`) built from the **IFC root's** style. ⚠ It is **not** the `parent_style` in scope at the emission site: `collect_inline_items_inner` passes the child's own `&style` as the next level's `parent_style` (`inline/collect.rs:291`), so at depth ≥ 1 that binding is the enclosing *inline's*, not the root's. The root ctx is therefore **threaded like `root_horizontal`** (`inline/collect.rs:211`, seeded at `:150`) — and in fact **replaces** it, since `root_horizontal: bool` is the same IFC-root projection fact in narrower form; its one consumer, `positioned_subflow_key(…, root_horizontal)` (`inline/collect.rs:287`), takes the bool from `ctx.is_horizontal()`. Source: the root's `parent_style`, bound at `inline/mod.rs:150`, the same one feeding `:173` and `:242`. ⚠ Revision 6 said the *decorated inline's own* style; that is wrong. The box's edges are placed along the **line's** inline axis, which is the IFC's, and `direction` on an inline box with `unicode-bidi: normal` has no bidi effect (css-writing-modes-4 §2.1). Only the physical edge *values* come from the element's own `ComputedStyle`. Logical facts are *derived* at the point of use via `LogicalEdges::from_physical` (`logical.rs:186`), applied to each set separately: their inline-start/inline-end components summed across the three sets give M3's advance and M4's content offset (one quantity, computed once and carried on the marker as `inline_start_total` / `inline_end_total`); the same components tested *per set* give M5's predicate. `helpers.rs`'s `inline_pb` (`:148`) is **not** reusable here — it covers padding + border only, and the advance must include margin. Mirrored in `PackItem` (`inline/pack/items.rs:18`); never a `FlowMember`. | **Three sets, not one**: `LayoutBox` has independent `padding`/`border`/`margin` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:88-91`) consumed separately by `padding_box`/`border_box`/`margin_box` (`:134-150`), and `resolve_box_model` already returns the triple — one `LogicalEdges` cannot fill three. **Physical, not logical, across the boundary**: `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) writes *physical* fields and receives `is_vertical: bool` with **no `direction`**, so a logical payload would need a return trip whose context is not present there. Carrying physical + the ctx keeps one conversion direction and no reconstruction. Emit on **any** side because M4 must fill all four for a block-axis-only inline to paint; the *existence* test is inline-axis-only (M5) — see §6 cell 6 for the pair. `resolve_box_model` is mandatory: `sanitize_padding` (`:96`) resolves percentages against 0, `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative. Basis = logical width = inline size (css-box-3 §3.1/§4.1; css-writing-modes-4 §6.1), `resolve_padding`'s documented contract (`:57-62`). |
| **M2** | Collapse-pass arm | **Transparent** — same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so interleaved markers are skipped by construction — including an adjacent `Start`/`End` pair. | §1.6 (css-text-3 §4.1.1 step 4): collapsing crosses "the boundary of the inline containing that space". A barrier would change `a<span style="padding:10px"> </span>b`, currently-correct markup. |
| **M3** | How does anything occupy a line? | **One owner, two callers.** Extract `LinePacker::note_line_occupancy(inline_advance, block_advance, hang, contributes_content)` (NEW, private) writing exactly the five line-state fields `place_item` writes today: `current_inline +=` (`:695`), `current_line_height = max(..)` (`:696`), `on_line = true` (`:697`), `any_rendered_content \|=` (`:698`), `current_line_last_hang =` (`:701`). `place_item` calls it **after** its soft-wrap check (`:690`) and **after** snapshotting `seg_inline_start = self.current_inline` (`:694`, which five downstream sites consume), passing its own `(full-trimmed).max(0.0)` hang — behaviour unchanged. The marker path calls it with **no** soft-wrap check and `contributes_content` from M5. `hang = 0.0` **only when `has_inline_axis_edge` holds** (M5) — the same predicate as the shaping break; a zero-advance marker (a `padding-top`-only box, cell 6) leaves the hang alone, because the space before it *is* still line-final. A marker for which **M5's `has_inline_axis_edge`** holds also clears `last_placed_entity` (`:772`). | Round 4's CRIT 1: rev 4 left *entering the line* unowned. A shared core answers that and round 3's "the split duplicates `place_item`'s sequence". No wrap check: §1.3, a box boundary is not a break opportunity. `hang = 0.0`: css-text-3 §4.1.2 keys on "at the end of **a line**", the engine encodes that at `:264-279`, and `place_item` already zeroes it for an atomic. Shaping break on non-zero **components** (§1.4's wording), so a negative or compensating pair still breaks. ⚠ **`on_line` has a second reader** — the soft-wrap guard's `&& self.on_line` (`:690`) — and a marker at the head of a line would arm it, so the first content segment could soft-wrap where `on_line == false` protects it today. **Mechanism**: the guard keys instead on a new `content_on_line: bool` written only by `place_item`, reset with the other per-line fields (`:432-439`) **and cleared in `force_break` (`:783`) alongside `on_line`** — a `<br>` mid-line must not leave the next line thinking content was already placed. That is what the guard has always meant — *do not flush a line with nothing on it* — and "nothing" is the absence of **content**, not of a box boundary. `on_line` keeps its `finish()` role (`:787`) unchanged. §6 cell 15b pins it. |
| **M4** | Where does the box's rect come from, and how do the edges reach `LayoutBox`? | **Rect**: an open-box **stack** records each `InlineBoxStart`'s cursor; `InlineBoxEnd` pops it and pushes one `current_line_entity_rects` entry (`:706`) for `entity` spanning **content-start → end-cursor** (read **before** the end marker's own advance) — the box's **content** span, never inflated by edges. Pushed by an explicit branch, not the `entity != parent_entity` guard (`:703`), which does not suppress a nested inline. The stack entry stores the box's **content-start cursor** (already past `inline_start_total`), so the rect is `content-start → end-cursor` with no edge re-added. `flush_line`, at the top before any arm runs, walks the **whole** open-box stack: it **emits** a partial rect for each entry whose span is non-empty, and **rebases every entry's content-start to 0 unconditionally** — the two scopes differ, and revision 8's wording bound the rebase to the emitted set, leaving a box opened at the end of line N (span 0, no rect) holding a line-N cursor into line N+1 and yielding an inverted rect. §6 cell 17c pins it — the start edge was consumed on the earlier line and must not be applied again. The end edge is symmetric and needs no handling: an open box has not reached its end marker, so line N's partial rect reserves nothing for it, so a box straddling a break yields one rect per line and a box opened exactly at a break yields none. ⚠ **The marker's rect is a second producer for the same entity**: a decorated inline with text already has a `place_item` rect on that line (`:706`, its runs carry `entity == span`). The **persisting** arm folds per entity before committing (`commit_aligned_entity_rects`, `:479-487`), so one fragment per line survives — correct, and the only arm that runs. The non-persisting arm (`:401-420`) does **not** fold, but it is **dead code**: `FragmentationType` has exactly `Page` and `Column` (`crates/layout/elidex-layout-block/src/lib.rs:37-42`) and `InlineFragConstraint.fragmentation_type` is non-optional (`inline/mod.rs:94`), so `persist_candidate = frag_constraint.is_none() || frag_is_paged || frag_is_column` (`:239`) is **identically true** and `flow_align` is always `Some`. ⚠ Revisions 6–7 booked a fold for that arm and a DoD cell to pin it; both are **withdrawn** — a cell no markup can construct is the defect M8's round-5 finding was written to prevent. The dead arm is recorded in §9. **Edges**: **no side-store.** `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) gains one `containing_inline_size: f32` parameter — already in scope at its only call site (`inline/mod.rs:380`, inside `layout_inline_context_fragmented`) — and calls `resolve_box_model(&style, containing_inline_size)` itself, on the `ComputedStyle` it **already fetches per entity** (`boxes.rs:57`). ⚠ Revisions 6–8 threaded a `&HashMap<Entity, (EdgeSizes, EdgeSizes, EdgeSizes)>` sibling map and argued it only against an `EntityBounds` member; the third option — derive at the point of use from the component the consumer already holds — was never considered, and it is what CLAUDE.md's side-store rule requires (`EdgeSizes` is `Send + Sync` and is not a per-VM identity handle). The marker still carries the triple for M3/M4/M5, which run in the packer where no `ComputedStyle` fetch is in hand. | `assign_inline_layout_boxes` writes bounds into `LayoutBox.content` (`:81`) and `border_box() = content + padding + border`, so an edge-inflated rect double-counts (round 3 CRIT). Reading the end-cursor before the end advance is what keeps the end side from double-counting too (round 4). The flush-time **emit** is what revision 4's rebase-only version missed: `InlineBoxEnd` has not run at line N's flush, so line N's fragment was simply lost. A sibling map rather than an `EntityBounds` member because `EntityBounds` is a per-line geometry accumulator built by struct literals with `and_modify` arms — **one live pair** (`:498`/`:505`); the other (`:406`/`:413`) is inside the dead arm §9 deletes that would not re-set an element-constant on later lines — cf. [[feedback_writesite-audit-includes-struct-literal-ctors]]. |
| **M5** | What makes a line non-phantom, and what else commits? | **The clause-3 predicate is a function of the marker's edges, not a per-PR constant**, and it is a **disjunction, not a sum**: `contributes_content = has_inline_axis_edge(marker)`, true iff **any one** of the three `LogicalEdges` (padding, border, margin — each converted separately by `LogicalEdges::from_physical`) has a non-zero `inline_start` or `inline_end`. **M5 owns this predicate, and it lands in PR-1b** — M3's shaping break and hang gate are PR-1b deliverables and both call it, so the predicate cannot wait for PR-1c. What PR-1c adds is the one-argument *substitution* below, not the predicate. M1's emit test (any side) is a different test on the same payload and stays in PR-1a. Evaluated per marker in `pack/inline_box.rs` and handed to M3's core. In PR-1b the marker path passes a constant `false` (geometry only); **PR-1c replaces that constant with this expression** — that one-argument substitution *is* the existence flip. Once a line is non-phantom, §2.3's "the line box **and its in-flow content**" applies: every entity on it commits through the existing seam (`:210`), none withheld. | Rev 5 wrote `contributes_content` as a literal `true` for PR-1c, which would have kept a `padding-top`-only inline's line alive — contradicting §1.2, M1 and cells 5/6 (round 5 CRIT). Emit and existence are different predicates over the same payload, so the existence one needs its own site. **Disjunction, not sum**: css-inline-3 §2.3 lists "non-zero **inline-axis** margins, padding, or borders" — three separately-named quantities, so `margin-left:-10px; padding-left:10px` — which sums to zero — still keeps the line. This is the one place the three sets must stay separate; M3's *advance* and M4's *content offset* both take the sum, because geometry adds up and a negative margin really does pull content back. §6 cell 3b pins the cancelling pair. |
| **M6** | Height of a line kept only by decoration | `InlineBoxStart` carries the element's resolved `line_height` and font identity; M3's shared core takes `current_line_height = max(block_advance)` with `block_advance` following the packer's existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`). **The memo does not claim §5.3/§2.2 conformance**: the block container's **root inline box** (§1.1) is unimplemented, so the line's height floor is missing. The text path already diverges identically — `<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today, with no marker involved. New slot **`#11-inline-root-inline-box`**, pre-existing class; §6 cell 23 pins the divergence so it stays distinguishable from a bug. | The direct authority is `body css-inline-3 line-layout` (§2.2) Note: "Empty inline boxes still have margins, padding, borders, and a **line-height**, and thus influence these calculations just like boxes with content." Revision 4 cited §5.3, which defines the strut per *box* and does not address the line-level question — round 4's finding. Disclosing a pre-existing divergence the new code depends on is the §4.3 pattern. |
| **M7** | Which line gets a strut baseline | `InlineBoxStart` records a tentative `current_line_box_baseline: Option<f32>` from the box's first-available-font metrics via `FontDatabase::query` + `font_metrics` (`crates/text/elidex-shaping/src/database.rs:60`, `:101`), keeping the `!is_vertical` guard (`:575`). `flush_line` promotes it into `first_baseline` **inside** the `if self.any_rendered_content` arm (`:210`) — never on a suppressed line — and only if `first_baseline.is_none()`. The field joins `flush_line`'s per-line reset block (`:432-439`). **No M-row states that block's field count** — it is five on `154bac3f`, M3 adds one in PR-1b and M7 one in PR-1c; stating a running total is what desynced M3, M7 and §5.2 across rounds 7–8. | §1.5: a strut exists only for a glyphless box; §2.2 owns the line-level composition. No second flag: the text arm sets `first_baseline` at pack time (`:575`), so `is_none()` at flush already answers "did any glyph-bearing segment land here or earlier". Traced against all three orderings. `query`+`font_metrics` rather than `measure_text`, because a glyphless box has no string to shape and the metrics are string-independent anyway (`elidex-shaping/src/measurement.rs:55`) — round 4 flagged the two rows naming different entry points. ⚠ **Known residual**: if a line's text has no usable font, `measure_text` returns `None`, `first_baseline` stays `None`, and a co-resident box's tentative promotes on a line that does have glyphs. §6 cell 24 pins it as accepted. |
| **M8** | Intrinsic sizing | **`max_content_inline_size` only** (`inline/measure.rs:44`): each marker adds its inline-axis edge sum to the running total, matching that pass's existing `total +=` shape. **`min_content_inline_size` is out of scope and gets a slot** — `#11-inline-min-content-box-edges` (own deferral): the pass is `max_word = max_word.max(m.width)` per word per item (`:15-37`) with **no running candidate and no cross-item joining**, so a box's edges have nothing to attach to; giving them one is an accumulator restructure of a pass that also already ignores run boundaries (`a<b>b</b>c` yields `max(|a|,|b|,|c|)` today, never `|abc|`). Trigger: PR-1c landing, or any shrink-to-fit correctness work. Re-eval: 2026-11-01. **PR-1b**, cell 25 scoped to max-content. | Round 5, three axes independently: rev 5's rule (and the "no adjacent word" clause added just before dispatch) both named an accumulator the site does not have. Scoping to the pass that *can* host it, and slotting the one that cannot with its real cost stated, is the honest split — rev 5's version would have shipped a DoD cell that cannot pass. |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`) against `containing_inline_size`. Two of the seven bogus §5.3 cites live here (`:59`, `:114`); the other five are in `lib.rs:178`, `positioned/layout.rs:90`, `block/mod.rs:162`, `block/mod.rs:182`, `block/children/helpers.rs:213` — PR-1a fixes all seven in one commit (§3.1). |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair, with the payload M1 specifies. Gains `containing_inline_size` on `collect_inline_items` (`:136`) / `collect_inline_items_inner` (`:194`), and **widens the existing `root_horizontal: bool` (`:211`) into the IFC root's `WritingModeContext`** — same fact, enough of it for M1. ⚠ `collect_inline_items` has **four** callers (verified 2026-08-02 via `grep -rn 'collect_inline_items(' crates/` minus the definition): `inline/mod.rs:154` (has the value), `inline/measure.rs:22` and `:51` (`min_content_inline_size`/`max_content_inline_size`, where a containing inline size is definitionally unavailable — see §9), and the test helper `inline/tests/mod.rs:17`. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get `PackItem` forms; they never become `FlowMember`s. |
| `crates/layout/elidex-layout-block/src/inline/pack/mod.rs` | `LinePacker` line state. **M3's `note_line_occupancy` lives here**, beside `place_item` (`:679`), its first caller. `flush_line` (`:209`) **calls** the open-box hook that `pack/inline_box.rs` owns (M4), promotes M7's tentative baseline inside its `:210` arm, and grows its per-line reset block (M3 in PR-1b, M7 in PR-1c; no site states a running count). The fabricated shaping citation at `:726` is rewritten by M3. |
| `crates/layout/elidex-layout-block/src/inline/mod.rs` | Three sites: `items.is_empty()` (`:161`, held in PR-1a, flipped in PR-1c); the `any_font` closure's exhaustiveness arm (`:192-199`, PR-1a, behaviour-neutral) and its **outer early-return condition** (`:200`, PR-1c, gains a marker escape beside `Atomic`); and §7's `clear_inline_flows` gating (`:637` — ⚠ inside the seam-3 range, so this cite moves to the extracted module before PR-1a). |
| `crates/layout/elidex-layout-block/src/inline/whitespace.rs` | `collapse_inline_whitespace` (`:41`) — M2's transparent arm. |
| `crates/layout/elidex-layout-block/src/inline/measure.rs` | `max_content_inline_size` (`:44`) — M8's contribution. `min_content_inline_size` (`:15`) is **not** touched; see `#11-inline-min-content-box-edges`. |
| `crates/core/elidex-plugin/src/logical.rs` | `LogicalEdges::from_physical` (`:186`) + `WritingModeContext::new` (`:27`) — used at the *point of derivation* (M1), never as a round trip. |
| `crates/layout/elidex-layout-block/src/inline/pack/inline_box.rs` (NEW) | **The marker's own derived facts and the stack that holds them**: push on `InlineBoxStart`, pop-and-emit on `InlineBoxEnd`, the flush-time hook `flush_line` calls (emit + rebase, M4), and M5's `has_inline_axis_edge` — one derivation site for everything read off a marker. An `impl LinePacker` in a sibling module, the idiom `pack/fragment.rs:10` already uses (one algorithm, one invariant). ⚠ Revision 4 made this a four-concern bucket justified by `pack/mod.rs`'s line count; round 4 was right that this is the mechanical application CLAUDE.md subordinates to cohesion. The line-state core (M3) and the baseline promotion (M7) therefore stay with their existing owner in `pack/mod.rs`. |
| `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs` | `assign_inline_layout_boxes` (`:48`) — writes `LayoutBox.content` from bounds and, per M4, the three edge fields from `resolve_box_model` on the `ComputedStyle` it already fetches (`:57`), given one new `containing_inline_size` parameter. |
| `crates/layout/elidex-layout-block/src/inline/tests/mod.rs` | The test helper: a `collect_inline_items` caller (`:17`, one of four) and an exhaustive `match item` (`:20-23`) that gains marker arms in PR-1a. |
| the seam-3 module (NEW, created by the prereq PR) | `layout_inline_context_fragmented`'s reconcile block, moved out of `inline/mod.rs`. §7's `clear_inline_flows` gating lands here, so `:637` is a pre-move coordinate. |
| `elidex-text` (facade over `elidex-shaping`) | `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60`) + `font_metrics` (`:101`) — M7's strut A/D, taken without shaping a string. |

### §5.3 Program slicing

Three shipping PRs on **disjoint invariant sets** (§2's rightmost column), plus two booked
slots. Each PR gets its own plan-memo and `/elidex-plan-review`.

* **PR-1a — item stream (invariant 2). Behaviour-neutral.** M1 + M2. Marker variants, all
  consumers updated (`whitespace.rs` per M2; `pack/items.rs`; the exhaustive
  match at `inline/tests/mod.rs:21-22`; `measure.rs` and `collect.rs:181` are `if let` and need
  none), `containing_inline_size` threaded, all seven bogus §5.3 cites fixed (§3.1). The packer gets a **no-op**
  `match pi` arm (`:528`). The two pre-pack gates are held at current behaviour:
  `items.is_empty()` (`inline/mod.rs:161`) excludes markers, and the `any_font` closure gains its
  **exhaustiveness arm** at `:192-199` returning `false` — behaviour-neutral, since a marker is
  neither `Text` nor `Atomic`. The **outer condition** at `:200` is untouched until PR-1c. **Characterization tests for §6 cells 1–12, 3b, 12b, 12c and 12d land here asserting
  today's behaviour**, since `grep -rn padding crates/layout/elidex-layout-block/src/inline/tests/`
  → **0 hits** (verified 2026-08-02) means the existing suite pins nothing about decorated
  inlines. No new slot.
* **PR-1b — geometry (invariants 3, 7).** M3 + M4 + M8. The shared line-occupancy core with
  markers passing `contributes_content = false`, inline-axis advance, shaping break at a
  decorated boundary, real `LayoutBox` edges, content-span rects, intrinsic-size contribution.
  **This is the engine-wide correction**: it changes painted output for every decorated inline
  with content. §6 cells 13, 14, 14b, 15, 15b, 16, 16b, 17, 17b, 17c and 25 land here.
  ⚠ **Scope of "neutral", stated precisely** (round 4 CRIT 3): PR-1b leaves the **phantom-line
  predicate** unchanged, so no line's *existence* flips. It does **not** leave `line_count`
  unchanged — the advance moves the cursor, so following text reaches the wrap guard (`:690`)
  earlier and content-overflowing paragraphs gain lines. That is the correct consequence of
  §1.1's "inline-axis … respected between inline-level boxes", not a regression, and cell 13's
  displacement half plus cell 15 pin it. The two shapes stay lineless in PR-1b for **two different
  reasons**, and neither is the one rev 4 or rev 5 gave:
  * **Shape A** reaches the packer (its collapsed-to-`""` `StyledRun` keeps `items` non-empty),
    the shared core sets `on_line`, `finish()` (`:787`) flushes, and the flush takes the
    **discard** arm (`:423`) because `contributes_content` is `false` in PR-1b — no `LineBox`,
    and `current_block_offset` untouched (`:422` is in the commit arm).
  * **Shape B never reaches the packer at all**: PR-1a holds `items.is_empty()`
    (`inline/mod.rs:161`) at "excludes markers" and PR-1b does not flip it, so the early return
    fires first. That is a **held gate — an inert shim that PR-1c removes**, not a property of
    M3/M4, and it is stated here rather than attributed elsewhere.
* **PR-1c — existence (invariants 1, 4, 5).** M5's *flip* (the predicate itself ships in PR-1b with its M3 consumers) + M6 + M7. The clause-3 predicate per §1.2,
  the strut baseline, the commit consequence, and the two pre-pack gates flipped. **The delta to PR-1b's marker call is
  `contributes_content` (M5's expression replacing the constant `false`) plus `block_advance`
  (M6);** everything else is already in place. §6 cells 18–24 plus the flipped 1–4, 3b, 7, 8, 10, 11, 12 and 12d land here. Closes
  `#11-line-box-decorated-inline-content` with its original DoD — line *and* rectangle — met,
  because PR-1b already made the rectangle real.
* **`#11-inline-box-decoration-splits`** (new slot, **own** deferral): css-inline-3 §2.1 box
  splitting across line boxes and bidi fragments, and **css-break-3 §5.4** `box-decoration-break`
  (the "no visual effect where the split occurs" rule — that string is CSS 2 §9.4.2 verbatim and
  appears nowhere in css-inline-3 §2.x; revision 4 mis-attributed it), plus fragmentation across
  the marker pair; **css-text-3 §5.5's adjacent-soft-wrap rule** (a break next to a decorated
  boundary lands at the box's *margin edge*, which M3's unconditional start-edge advance does not
  honour — §3's second §5.5 row and §6's note after cell 15 route here); and the `group_key` /
  relpos sub-flow keying that §2's pair 2×6 and §3's §9.4.3 row defer here, the field's only
  reader being the split rule. Why deferred: box continuity across line, bidi and sub-flow
  boundaries is a distinct invariant axis, reachable only once a box has geometry; folding it in
  would put a third axis into PR-1b. Trigger: PR-1c landing. Re-eval: 2026-11-01.
* **`#11-inline-root-inline-box`** (new slot, **pre-existing** class): the block container's
  root inline box (css-inline-3 §2), which sets every line box's height floor. Why deferred: it
  changes the height of *every* line box in the engine — verified pre-existing because the
  divergence is observable today on ordinary text (M6's example), with no marker involved, so
  `origin/main` already fails it. Trigger: any line-height correctness work, a compat-survey hit, **or any work needing
  font-fallback provenance** (css-inline-3 §5.3's "only glyphs from fallback fonts" strut
  condition folds here, §8). Re-eval: 2026-11-01.

Own-deferral count: **3** (`#11-inline-box-decoration-splits`, `#11-inline-min-content-box-edges`, `#11-plan-memo-consistency-checkers-on-ci`); `#11-inline-root-inline-box` is pre-existing class. Within the per-PR ≤3 policy.

**Rejected**: widening `StyledRun` with edge fields (box-level data on a per-segment
measurement type; N copies for an N-segment span; reaches neither shape per §4.2); reading
`run.entity`'s `ComputedStyle` at pack time (reaches neither shape, same reason). Neither is
rejected on component-lookup cost — `collect.rs:36` uses borrowed component reads in the same
per-child loop, a normal idiom here.

## §6. Edge matrix

**PR-1a characterization (assert today's behaviour; PR-1c flips 1–4, 3b, 7, 8, 10, 11 and 12 — cells 5, 6, 9, 12b and 12c are non-flipping):**

1. `padding` alone (inline-axis) — line currently suppressed.
2. `border` alone, plus `border-style: none` + `border-width: 5px`. css-backgrounds-3 §3.2:
   "No border. Color and width are ignored (i.e., the border has width 0)". Applied at
   computed-value time (`elidex-style resolve/box_model/mod.rs:260`).
3. `margin` alone, **including negative** — requires M1's `resolve_box_model` sourcing.
3b. **Cancelling pair** — `<span style="margin-left:-10px;padding-left:10px">`: the inline-start
    components sum to zero but each is non-zero, so §2.3 keeps the line (M5's disjunction). The
    cursor advance for the same box is zero (M3's sum).
4. **Percentage** `padding: 5%` / `margin: 2%` — against `containing_inline_size`.
5. **Block-axis edges only** (`padding-top`, `margin-top`) — **line stays suppressed in
   PR-1c too**, per css-inline-3 §2.3's inline-axis restriction (§1.2). For the *margin* half
   CSS 2 §8.3 independently agrees; it says nothing about padding or border, whose authority is
   css-inline-3 §5.3 + `line-fit-edge: leading` (§1.2). A non-regression cell in
   every PR; revisions 1–3 pinned the opposite.
6. All zero ⇒ no marker emitted at all (M1). **Block-axis-only** (`padding-top`) ⇒ marker **is**
   emitted, cursor advance 0, and the line stays phantom — the emit/existence split in M1/M5.
   ⚠ On a phantom line the box gets **no rect and no paint**: §2.3 makes the line box "and its
   in-flow content" non-existent, and the discard arm (`:428`) clears the tentative rects. The
   marker earns its keep only on a line that exists for another reason — `<p>text
   <span style="padding-top:10px">x</span></p>`, where M4 must fill all four `LayoutBox` sides.
   Two sub-cells: phantom ⇒ no box; co-resident ⇒ full four-sided box.
7. Shape A (decorated inline containing only collapsible white space).
8. Shape B (completely empty decorated inline).
9. `a<span style="padding:10px"> </span>b` — the M2 cell: the space must collapse against its
   neighbours exactly as today.
10. Nested: inner decorated / outer not, and the reverse — exercises M4's open-box stack.
11. Two inlines on one line, one decorated.
12. Decorated inline as the only IFC content — the **`items.is_empty()` gate**
    (`inline/mod.rs:161`) only, held in PR-1a and flipped in PR-1c. ⚠ This cell does **not**
    exercise the `any_font` probe: `has_text` (`:190`) is false for a marker-only stream, so the
    whole `if has_text` block (`:191-211`) is skipped. Revisions 6–8 claimed it did.
12d. **The `any_font` early-out with a decorated inline** —
    `<p><span style="font-family:NoSuchFont">x</span><span style="padding:10px"></span></p>`:
    `has_text` is true and no font is usable, so `:200`'s early return fires and the IFC returns
    `line_count: 0` today — even though clause 3 makes the line exist, which is font-independent.
    PR-1c adds a marker escape beside the existing `Atomic` one in that **outer condition**
    (`:200`); PR-1a only adds the exhaustiveness arm at `:192-199`.

12b. **`direction: rtl`** (M1): `<p dir="rtl"><span style="padding-left:10px">x</span></p>` —
    `padding-left` is the inline-**end** edge here, so it advances the cursor *after* the content.
    Pins the physical→logical mapping itself (`padding-left` → inline-end under rtl). It does
    **not** discriminate whose `direction` is used — `direction` is inherited, so both sources
    agree here. M1's IFC-root rule is asserted separately by `<p dir="rtl">` containing
    `<span dir="ltr" style="padding-left:10px">`, where the two disagree and the line's axis wins.
12c. **`writing-mode: vertical-rl`** (M1): the inline axis is vertical, so `padding-top`/`bottom`
    become the inline-axis pair and `padding-left`/`right` the block-axis pair — cell 6's
    block-axis-only case inverts here, and cell 4's percentage basis is the containing block's
    inline size (its physical height).

**PR-1b geometry:**

13. `<p>a<span style="padding:10px">text</span>b</p>` — the common case §4.3 is about. Three
    assertions: (a) the span's `LayoutBox` carries real edges and its border box is
    `content + padding` **once**, not twice; (b) **`b` is displaced by 20px** — the
    user-visible half of §1.1's "respected *between* inline-level boxes", which revision 4's
    cell asserted only for the box itself; (c) the painted background-colour and border rects
    (`crates/core/elidex-render/src/builder/paint/mod.rs:68` and `:382`, both `lb.border_box()`)
    match the border box. (`:85` is `padding_box()`, the background-*image* area — revisions 6–8
    cited it here by mistake.)
14. **Shaping breaks at the boundary** (§1.4): in `<p>a<span style="padding:1px"></span>b</p>`
    the two same-entity texts must **stop** coalescing into one `InlineFlowRun`. Contrast: with a
    span whose edges are zero on **every** side, no marker is emitted (M1) and they still coalesce.
    ⚠ A `padding-top`-only span **does** emit a marker but fails `has_inline_axis_edge`, so
    coalescing there is the gate's doing, not automatic — cell 14b. Neither contrast holds for
    `vertical-align: super` or `dir`-isolated spans, which §7.3 also breaks on; §3's row records
    those two triggers as unimplemented.
14b. **The gate's negative arm** — `<p>a<span style="padding-top:1px"></span>b</p>`: a marker
    exists, `has_inline_axis_edge` is false, so the runs still coalesce, the hang is untouched
    (cell 16's contrast) and the line stays phantom (cell 6).
15b. **A leading marker must not let the first segment soft-wrap** (M3's ⚠): `<p style="width:10px">`
    `<span style="padding-left:20px">verylongword</span></p>` — with `on_line` now armed by the
    marker, the first content segment reaches `:690` with an inflated cursor; the line must not be
    flushed-and-discarded out from under it.
17c. **A box opened at a line end** — `<p style="width:100px">aaaaaaaaaa<span style="padding:5px">bbbb</span></p>`:
    the marker opens with zero span on line N, so no partial rect is emitted, but its content-start
    **must** still be rebased to 0 or line N+1's rect comes out inverted (M4).
17b. **Two producers, one fragment** (M4's ⚠): a single-line `<span style="padding:10px">text</span>`
    must yield exactly **one** `line_rects` entry — the persisting arm's per-entity fold
    (`commit_aligned_entity_rects`, `:479-487`) absorbing `place_item`'s rect and the marker's.
15. **The boundary is not a wrap opportunity** (§1.3):
    `<p style="width:100px">aaaaaaaaaaaa<span style="padding:20px"></span>b</p>` — the box does
    not move to the next line at its own edge. Paired: `b` **does** wrap, and earlier than
    today, because the cursor is 40px further along (the PR-1b `line_count` change §5.3 states).
*(A soft wrap opportunity adjacent to a decorated boundary — css-text-3 §5.5's other bullet puts
the break at the box's **margin edge** — is **not** a cell of any PR here. M3 advances the start
edge unconditionally, so that rule is unmet; it is folded into `#11-inline-box-decoration-splits`
(§5.3's Why names it). §3's §5.5 row is marked ✗ accordingly.)*
16. **`text-align` with a box after the space** (§4.1.2): `<p style="text-align:center">abc
    <span style="padding:1px"></span></p>` — the space is **no longer at the end of the line**,
    so §4.1.2 **step 3** ("A sequence of collapsible spaces at the end of a line is removed") no
    longer applies to it, and step 4's hanging — which acts on what step 3 leaves — is not reached
    either. The premise is css-inline-3 §2's "inline-axis margins, borders, and padding are
    respected between inline-level boxes": the marker's advance *is* line content after the space.
    M3 therefore zeroes `current_line_last_hang`, matching the engine's `top_group_is_line_last`
    convention (`inline/pack/mod.rs:264-279`). ⚠ Revision 6 pinned the opposite; revisions 7–8
    argued it from step 4 when step 3 governs.
16b. **Paired contrast** — the same markup **without** the span: the trailing space must still
    hang, exactly as today. Pins that M3's zeroing is scoped to the marker and does not leak.
17. **A decorated inline whose content wraps** — start marker on line N, end marker on line
    N+1; M4's per-line rebase must yield one rect per line, each with that line's
    `block_start`.

25. **Shrink-to-fit, max-content only** (M8): a `float: left` / `display: inline-block` containing
    `<span style="padding:10px">x</span>` must have a **max-content** inline size 20px wider than
    the same box without the padding. Its **min-content** size is knowingly unchanged — see
    `#11-inline-min-content-box-edges` in §5.3.

**PR-1c existence:**

18. `white-space: pre` — `<pre> <span style="padding:5px"></span></pre>`: the line is already
    kept by clause 2 via the *preserved space text*; height must not double-count (M6 takes a
    `max`).
19. Decorated inline adjacent to a forced break, incl. `<pre>\n</pre>` (clause 5).
20. `display: none` ⇒ no marker (`inline/collect.rs:218`); `position: absolute` ⇒ out of flow,
    clause 3 does not apply.
21. **Co-resident entity on a newly-existing line** — §2.3's "and its in-flow content" means
    that when the line exists, an undecorated inline sharing it commits its rect too (M5).
22. **Decorated inline with text must not take a strut baseline** (§1.5); a glyphless one must
    (M7).
23. **Root-inline-box divergence, pinned as accepted** (M6):
    `<p style="line-height:40px"><span style="padding:1px;line-height:5px"></span></p>` yields a
    5px line, not the spec's 40px, because the block container's root inline box is
    unimplemented (`#11-inline-root-inline-box`). Asserted so the divergence stays
    distinguishable from a bug.
24. **No-usable-font residual, pinned as accepted** (M7): on a line whose text has no usable
    font, a co-resident glyphless decorated box's tentative baseline promotes even though the
    line has glyphs.

**Non-regression throughout**: `collapsible_whitespace_only_generates_no_line_box`
(`crates/layout/elidex-layout-block/src/inline/tests/text_height/basic.rs:204`) and
`nbsp_only_line_generates_a_box` (`:233`).

**Test placement** (§9): PR-1a's cells land in `inline/tests/decorated_inline/stream.rs`, PR-1b's in `…/geometry.rs`, PR-1c's in `…/existence.rs` — a new `decorated_inline` module split per PR from the start, per [[feedback_touch-time-split-means-while-writing]], rather than one file grown to ~34 cells. None of them go in
`tests/text_height/basic.rs` (303) or `relpos_subflow.rs` (915). The relpos facet of cell 10/11
lands with the new module too, not in the 915-line file.

## §7. Downstream

* `line_count`, IFC `height`, block cursor — **PR-1b moves all three** (the advance pushes following text past the wrap guard earlier; §5.3, cells 13(b)/15). PR-1b changes no line's *existence*; that is PR-1c.
* `first_baseline` — moves only on a glyphless line (M7); cell 22 asserts both directions.
* `entity_bounds` / `getClientRects` — PR-1b gives every decorated inline a real border box;
  PR-1c adds the co-resident commit (cell 21).
* **Painted output** — PR-1b changes background and border rendering for **every** decorated
  inline (`crates/core/elidex-render/src/builder/paint/mod.rs:68` background colour, `:382`
  borders; `:85` is the background-image area and follows the padding box). Largest delta in the
  program; cell 13 is its pin.
* `last_placed_entity` / persisted run shape — deliberately changed by M3 (§1.4); cell 14.
* `current_line_last_hang` — **zeroed at a marker** (M3): the space is no longer line-final; cell 16.
* Static positions of following abspos (`inline/pack/mod.rs:97`).
* `current_line_runs` / `flow_lines` — markers are never `FlowMember`s, so member-less buckets
  keep being dropped by design; no ordinal dependency (§2 withdrawn row).
* `clear_inline_flows` probe gating — a decoration-only IFC stops taking the early return at
  `inline/mod.rs:161` (unconditional clear) and starts taking the full path (`:637`,
  `!env.is_probe`-gated). **PR-1c**, per §5.3.
* Fragmentation (`inline/pack/fragment.rs`) and `ColumnFlowSlice`.

## §8. Definition of done

**PR-1a** (item stream, behaviour-neutral): both marker variants reach `pack/items.rs`; every
exhaustive match handles both (`inline/whitespace.rs:41`, `inline/mod.rs:192-199`,
`inline/pack/items.rs:67`, `inline/tests/mod.rs:20-23` — `atomic.rs:39`, `measure.rs` and
`collect.rs:181` are `if let` and need none); **the payload exactly as M1 specifies it, not
restated here**;
`containing_inline_size` threaded through all four `collect_inline_items` callers; the two
pre-pack gates hold current behaviour; **all seven** bogus §5.3 cites fixed; new variants carry
docstring citations to their §3 rows; cells 1–12, 3b, 12b, 12c and 12d land as characterization tests; zero behaviour
change. **Dead-field rule**: fields whose first reader is a later PR are added by that PR — M6/M7's
`line_height` and font identity by **PR-1c**, `group_key` by `#11-inline-box-decoration-splits`
— so nothing ships unread and no `#[allow(dead_code)]` is needed. ⚠ PR-1a's marker fields are
nonetheless read only by the *edge resolution* it performs, not by the packer (whose `match pi`
arm is a no-op); if that leaves `field is never read` under `-D warnings`, the characterization
tests in §6 are the intended reader, and PR-1a's DoD includes making that so rather than
silencing it.
**PR-1b** (geometry): cells 13, 14, 14b, 15, 15b, 16, 16b, 17, 17b, 17c and 25; §7's paint delta covered; no double-counted edges on
either side; the `inline/pack/mod.rs:726` shaping citation corrected to css-text-3 §5.5 + §7.3
(§3.1); `note_line_occupancy`, `pack/inline_box.rs` and M8's contribution carry docstring
citations to their §3 rows.
**PR-1c** (existence): cells 18–24 plus the flipped 1–4, 3b, 7, 8, 10, 11, 12 and 12d (cells 5, 6, 9, 12b and 12c are non-flipping); every §7 consumer checked; the ⚠ caveat
at `inline/pack/mod.rs:110` retires, naming the root-inline-box divergence and pointing at
`#11-inline-root-inline-box`; M5's clause-3 predicate and M7's promotion carry docstring
citations to their §3 rows; slot closes.

**Seam-3 prereq PR**: `layout_inline_context_fragmented`'s reconcile block (§9's measured range)
moves to its own module, **byte-identical modulo the extracted signature**. The block captures at
least `packer`, `static_positions`, `is_vertical`, `do_carrier`, `persist_flow` and
`persisted_keys` from the enclosing fn (measured on `154bac3f`); those become parameters, and that
is the **only** permitted edit. Proof obligation: a diff of the moved body against
`git show 154bac3f:crates/layout/elidex-layout-block/src/inline/mod.rs | sed -n '413,639p'` shows
only those bindings. ⚠ Revision 8 wrote "no line of moved code edited", which the capture set makes
unachievable. `cargo test -p elidex-layout-block --all-features` green with no test touched;
`#[allow(clippy::too_many_lines)]` on the residue (`inline/mod.rs:140`) re-evaluated. Ledger
actions in §10.

**Dead-arm prereq PR**: `flush_line`'s `else` arm (`inline/pack/mod.rs:393-421`) and the
`persist_candidate` branch (`inline/mod.rs:239`) removed, on §9's reachability argument;
`#11-inline-align-clientrects-nonpersist-path` closes. Stacked beside the seam-3 split, not folded
into it, so that PR's byte-identical criterion stays provable.

**Explicitly not covered, recorded rather than dropped**: css-inline-3 §5.3's "or if it contains
only glyphs from fallback fonts" strut condition — elidex has no fallback-provenance signal.
Folded into `#11-inline-root-inline-box`, whose trigger in §5.3 names font-fallback provenance
explicitly so the fold can surface (round 4's finding; rev 5 asserted the extension here without
making it at the slot's own definition).

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires, and this program honours it.** The
  slot's trigger is "the next change that touches `layout_inline_context_fragmented`'s body";
  PR-1a touches `:161`/`:190` and PR-1c the persist block, the slot memo's **seam 3**.
  **Disposition**: a standalone prereq split PR at seam 3, before PR-1a.
  ⚠ **Grounds, re-derived.** Revision 5 took round 4's framing *and its citation* without checking
  the citation's scope. CLAUDE.md's prereq-split clause is scoped to **">1000行 file を触る際"**;
  `inline/mod.rs` is **785 lines**, so that clause does not reach here, and #500's precedent
  (1125 / 1037) does not either. The grounds that do reach: (a) the slot's **own** trigger, which
  carries no size qualifier and has fired. That ground alone carries it. A second ground — "785 is in the
  700–800 cut-while-writing band" — is **withdrawn**: that lesson is about files being newly
  *authored*, and its own text routes pre-existing files to the >1000 clause, so reaching for it
  repeated the very citation-scope error this paragraph exists to record. The disposition is
  unchanged.
  **Scope**: the prereq PR discharges **seam 3 only** — `mod.rs:413-639` on `154bac3f`
  (re-measured; the slot memo's `:411-637` predates #497's two-line shift). Seams 1 (`:266-303`, re-measured on `154bac3f` like seam 3, not carried from the slot memo's pre-#497 numbers)
  and 2 (`:388-411`) remain, so §10 records a **partial** close. PR-1a's own touch sites lie in
  the residue, not in seam 3.
  **Cold gate** ([[feedback_split-on-touch-prereq-workflow]]): checked 2026-08-02 — no open PR or
  remote branch touches `elidex-layout-block`.
  **Its own gate**: `/elidex-plan-review`, like every PR here. ⚠ Revision 6–8 waived it as "a
  mechanical move"; that is withdrawn on two counts — CLAUDE.md makes the gate a rule not a
  judgment, and §10 routes five non-mechanical ledger actions to this PR, so the premise was false
  even on its own terms.
* **File growth**: `pack/mod.rs` 791, `inline/mod.rs` 785, `relpos_subflow.rs` 915,
  `tests/text_height/basic.rs` 303 (all verified 2026-08-02). Per §5.2 the **open-box stack** goes
  to a new `pack/inline_box.rs`; M3's line-state core and M7's promotion stay in `pack/mod.rs`; new tests to a new module (§6). `inline/mod.rs`'s growth across
  the three PRs is a signature change, two gate predicates and the persist-block touch — no new
  seam decision needed, and re-checked at PR-1c. **`pack/mod.rs` (791) is re-checked at the same
  point**: it takes `note_line_occupancy`, the `flush_line` hook call, M7's promotion and two reset
  fields, while only the open-box stack leaves for `pack/inline_box.rs`.
* **`#11-inline-relayout-box-staleness`** (`inline/pack/boxes.rs:96`) — pre-existing, but
  **PR-1b widens its observable surface**: today an inline's stale `LayoutBox` is
  indistinguishable from a fresh one because the edges are always zero, and after M4 a stale
  box carries stale edges. Recorded here rather than claimed non-widening (revision 3 claimed
  the latter). Both this slot and `#11-inline-align-clientrects-nonpersist-path` are already
  ledger-marked to fold into terminal-Z C-3/C-4; PR-1b's widening is a note on that fold.
* **`#11-layoutbox-absence-unreachable`** (#488) — no longer relevant: M5 keeps no entity's box
  withheld, so no truthful box-absent signal is required.
* **Intrinsic sizing** — split by pass, per M8: the max-content contribution is **in PR-1b**; the
  min-content one is slotted (`#11-inline-min-content-box-edges`) because that pass has no
  accumulator to attach edges to. The four `collect_inline_items` callers (§5.2) pass `0.0` as the
  containing inline size in the intrinsic passes — the standard treatment for percentages.
* **`#11-css2-spec-label-normalisation`** — this memo now cites css-inline-3 for the model, so
  its remaining CSS 2 cites are **§8.3, §9.4.2, §9.4.3, §10.8.1 and §16.6.1** (grep the memo for `CSS 2 §`; the §16.6.1 hit is §1.6's note on what css-text-3 §4.1.1 superseded, not a live grounding — but the grep returns five, so the inventory says five). Adjacent `CSS 2.1 §` lines in
  touched files are left alone; the slot requires a single cross-crate commit.
* Ruby annotations (§2.3 clause 4) — unimplemented engine-wide.
* **`flush_line`'s non-persisting arm (`inline/pack/mod.rs:393-421`) is dead, and this program
  deletes it.** `persist_candidate` (`inline/mod.rs:239`) is identically true — `FragmentationType`
  has only `Page`/`Column` and the constraint's field is non-optional — so `flow_align` is always
  `Some` and `LinePacker::new` has one call site. **A second standalone prereq PR** (stacked beside
  the seam-3 split, not folded into it, so the byte-identical criterion stays provable) removes the
  arm and the `persist_candidate` branch, and **closes `#11-inline-align-clientrects-nonpersist-path`**,
  which books work against the same unreachable code. ⚠ Revisions 6–7 booked a per-entity fold and a
  DoD cell against it, and revision 8 booked a *slot* — both wrong. The slot's stated ground ("touches
  the fragmentation seam the prereq PR moves") was **false**: the arm is in `pack/mod.rs`, seam 3 is in
  `inline/mod.rs`. CLAUDE.md is unconditional here, and deleting collapses two slots instead of opening
  a third.
* **The two plan checkers are not on any loop.** `plan-sweep.py` / `plan-xcheck.py` live in
  `.claude/tools/` and are invoked by hand; nothing in `scripts/trip-wires.sh`, `mise.toml`,
  `.github/workflows/ci.yml` or `.claude/skills/**` references them (verified 2026-08-02). Per
  [[feedback_every-triggered-pr-must-be-on-the-loop]] a checker reachable only by remembering it
  is the habit that failed six rounds with a tool attached. **Disposition**: they are generic
  over any plan-memo with this section shape, so they belong in `elidex-plan-review`'s Step 1.5
  or in the ungated `trip-wires` job — as **its own PR**, not folded into this umbrella. New slot
  `#11-plan-memo-consistency-checkers-on-ci` (own deferral). Trigger: this umbrella's first PR
  landing, or any other plan-memo adopting the tools. Re-eval: 2026-11-01.

## §10. Slot ledger actions at landing

| Action | PR |
|---|---|
| Register this umbrella's slot in `project_open-defer-slots.md` (the SoT per MEMORY.md), together with the two other unregistered Layout-lane slots — `#11-css2-spec-label-normalisation` (a #497 carve) and `#11-inline-fragmented-fn-decomposition` (carved from #495, not #497). Verified 2026-08-02: 0 hits each. | seam-3 prereq PR |
| Register **and** close `#11-inline-fragmented-fn-decomposition` in one row — it is registered as closed-on-landing, not registered then closed — **as a partial close**, naming the seams §9 measures as still open, in the successor slot `#11-inline-fragmented-fn-seams-1-2` (**pre-existing** class — the seams predate this umbrella; Why: the prereq PR discharges seam 3 only; trigger: any later change touching `layout_inline_context_fragmented`'s residue; re-eval 2026-11-01) — the prereq PR discharges only seam 3. | seam-3 prereq PR |
| Open `#11-inline-box-decoration-splits` (own) with the Why / trigger / date in §5.3 | PR-1b |
| Open `#11-inline-min-content-box-edges` (own) with the Why / trigger / date in M8 | PR-1b |
| Open `#11-plan-memo-consistency-checkers-on-ci` (own) with the Why / trigger / date in §9 | seam-3 prereq PR |
| Close `#11-inline-align-clientrects-nonpersist-path` — the arm it books work against is deleted | dead-arm prereq PR |
| Open `#11-inline-root-inline-box` (pre-existing) with the Why / trigger / date in §5.3 | PR-1c |
| Enrich `#11-inline-relayout-box-staleness`'s SoT entry with PR-1b's widening (§9) | PR-1b |
| Rewrite `project_line-box-decorated-inline-content.md`, `MEMORY.md`'s Layout-lane entry and `active-lane-detail.md`, all of which still carry a superseded framing of this slot | seam-3 prereq PR |

---

**Review history lives in `project_line-box-decorated-inline-content.md`** (transcribed there;
rounds 1–3 remain in git at `906f5f5d`). Rounds 1–5 produced ~250 lines of past-tense ledger
inside this memo, and those ledgers restated the normative decisions above — the source of four
of round 5's findings. ⚠ **Revision 6 first asserted this sentence before doing it**: the
history was deleted from the plan (`f6724844`, 301 deletions, one file) and written only to a
session-temp path, leaving `project_line-box-decorated-inline-content.md:72` and `MEMORY.md`
pointing at a `§13` this revision had removed. Both were repaired and the history transcribed
after round 6 caught it. Recorded rather than quietly fixed, because asserting a state of the
world one has not created is a worse failure than the stale claims this revision was written to
eliminate.

