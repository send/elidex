# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's task, user-approved 2026-08-02.
Edge-dense ⇒ every PR under this umbrella goes through `/elidex-plan-review` before
implementation, and external review runs `/external-converge` from round 1.

All premises verified against `154bac3f`; every spec quote below was resolved directly with
`.claude/tools/webref`, not carried from a reviewer or from a code comment.

**Revision 5.** Round 4 (11 CRIT / 29 IMP / 22 MIN) confirmed revision 4's re-anchor on
`css-inline-3` — all three of round 3's spec CRITs closed — and then found the mechanism
under-specified. §11 records round 4; §12 records what revision 5 changed and why. The central
change is **M3**: revision 4 gave the marker path its own line-state writes and left the
"entering the line" obligation unowned, which is round 4's CRIT 1. Revision 5 instead extracts
the obligation into a **shared core that `place_item` and the marker path both call**, so there
is one owner and the PR-1b→PR-1c delta is two arguments.

⚠ Authoring rules in force (per [[feedback_findings-cluster-in-self-added-scope]] §11.3, as
refined by round 4): **fix what the mandated dry-run finds; do not add what a reviewer's
framing suggests; re-derive rather than sentence-patch, either way.** Revision 4's blanket
freeze was too broad and cost round 4 three re-discoveries.

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
* CSS 2's **"strut" is the root inline box** in the current module — a real box the block
  container generates, not a special case. That reframes §5.2's gap: elidex is not missing an
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
> inline formatting context — is collapsed to have zero advance width.

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
| 2 × 6 | Markers must carry the enclosing recursion level's `group_key`. | M1 | 1a |
| 7 × (intrinsic) | An inline-axis advance the packer applies must also be visible to the intrinsic-size passes, which never build a `LinePacker`. | M8 | 1b |

Each pair is answered by exactly one M-row, and each M-row belongs to exactly one PR. That
mapping is what §5.3's slicing is derived from — three PRs, disjoint invariant sets.

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
| CSS Break 3 §5.4 Fragmented Borders and Backgrounds | `box-decoration-break: slice` | no visual effect where the split occurs | **`#11-inline-box-decoration-splits`** | ✗ (deliberate) | yes |
| CSS Inline 3 §2.2 Layout Within Line Boxes | Note on empty inline boxes | they still have a line-height and influence the calculation | M6 — **PR-1c** | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence (a) | zero-height for positioning descendant content (abspos) | `static_positions` (`inline/pack/mod.rs:97`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Layout Bounds | layout-bounds inflation by edges | applies only when `line-fit-edge` ≠ `leading`; initial is `leading` | no code touch; grounds §1.2's block-axis exclusion | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 3 | non-zero **inline-axis** margin/padding/border | clause-3 predicate → `any_rendered_content` (`inline/pack/mod.rs:698`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 5 | forced line break | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence | line box *and its in-flow content* do not exist | commit/discard seam (`inline/pack/mod.rs:210` vs `:423`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Layout Bounds | glyphless / fallback-only box | strut with first-available-font metrics | tentative baseline — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Layout Bounds | half-leading | `A′ = A + L/2` | existing formula (`inline/pack/mod.rs:581`) — unchanged | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | break opportunities | inline box boundary is **not** one | M3 — the marker path calls the shared core without a wrap check — **PR-1b** | ✓ | yes |
| CSS Inline 3 §2 Inline Layout Model | intrinsic contribution | inline-axis edges occupy space | M8 (`inline/measure.rs:15`, `:44`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §7.3 Shaping Across Element Boundaries | shaping break | non-zero inline-axis edge separates the units | `last_placed_entity` coalescing (`inline/pack/mod.rs:744`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | trailing collapsible space | hangs; excluded from alignment | `current_line_last_hang` (`inline/pack/mod.rs:701`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §4.1.1 Phase I: Collapsing and Transformation | step 4 | collapsing crosses inline box boundaries | `collapse_inline_whitespace` (`inline/whitespace.rs:41`) — **PR-1a** | ✓ | yes |
| CSS 2 §8.3 Margin properties | non-replaced inline elements | vertical margins have no effect | consistent with §2.3 clause 3; no code touch | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline | `collect.rs:286` sub-flow keying — **PR-1a** | ✓ | yes |
| CSS Box Model 3 §3.1 / §4.1 | percentage margin/padding | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract Dimensions | inline size ≡ logical width | basis identity in vertical modes | same | ✓ | yes |
| CSS Backgrounds 3 §3.2 border-style | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time (`elidex-style resolve/box_model/mod.rs:260`) | ✓ | yes |

**Breadth**: K=7 specs (CSS Inline 3, CSS Text 3, CSS 2, CSS Break 3, CSS Box Model 3,
CSS Writing Modes 4, CSS Backgrounds 3), M=23 entries (verified 2026-08-02 — data rows counted
directly above).
**Split decision**: K=7 ⇒ SPLIT-DEFAULT. The plan **is** split into three shipping PRs on
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
  fabricated (`heading css-text-3 5.6` → no headings; css-text-3's only shaping section is
  §7.3). PR-1b rewrites that comment anyway (M3 changes the coalescing rule it documents), so
  the citation is corrected there.
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
| **M1** | What enters the item stream, carrying what? | Two `InlineItem` variants, `InlineBoxStart` / `InlineBoxEnd`, emitted around the recursion at `inline/collect.rs:291` for every inline with **at least one non-zero edge on any side**. ⚠ The *emit* predicate is deliberately wider than clause 3's *existence* predicate: §1.2 counts inline-axis edges only, but M4 must fill `LayoutBox.padding/border/margin` on all four sides for a block-axis-only decorated inline to paint correctly, and a marker is the only carrier. M5's clause-3 test therefore reads **only the inline-axis components** of the marker's `LogicalEdges`; a `padding-top`-only box emits a marker, paints correctly, advances the cursor by 0, and leaves its line phantom. Payload: `entity` + **`LogicalEdges`** (`crates/core/elidex-plugin/src/logical.rs:172`) — all four flow-relative sides, not just the inline pair. Built as `LogicalEdges::from_physical(resolve_box_model(&style, containing_inline_size), WritingModeContext::new(style.writing_mode, style.direction))` (`logical.rs:186`, `:27`) where **`style` is the decorated inline's own** (`inline/collect.rs:217`), not `parent_style`. `containing_inline_size` threaded into `collect_inline_items` (`:136`). Mirrored in `PackItem` (`inline/pack/items.rs:18`); never a `FlowMember`. | Splitting emit from existence is what lets one carrier serve both: §1.2 clause 3 is inline-axis-only, while painting needs all four sides. Revision 5's first draft made the emit predicate inline-axis-only too, which silently left `<span style="padding-top:10px">text</span>` with a zero `LayoutBox.padding` — the block-axis pair must be carried **and** emitted: M4 fills `LayoutBox.padding/border/margin`, which `border_box()` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:140`) consumes on all four sides — revision 4's inline-axis-only payload could not fill it. `LogicalEdges::from_physical` is the engine-independent mapping this crate already uses (`block/mod.rs:158`, `positioned/layout.rs:88`), and those sites take the **element's own** style, which is also what `direction` requires (it selects inline-start vs inline-end for that box). `resolve_box_model` is mandatory: `sanitize_padding` (`helpers.rs:96`) resolves percentages against 0 and `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative. Basis = logical width = inline size (`css css-box-4 padding-top`; css-writing-modes-4 §6.1), `resolve_padding`'s documented contract (`helpers.rs:57-62`). |
| **M2** | Collapse-pass arm | **Transparent** — same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so interleaved markers are skipped by construction — including an adjacent `Start`/`End` pair. | §1.6 (css-text-3 §4.1.1 step 4): collapsing crosses "the boundary of the inline containing that space". A barrier would change `a<span style="padding:10px"> </span>b`, currently-correct markup. |
| **M3** | How does anything occupy a line? | **One owner, two callers.** Extract `LinePacker::note_line_occupancy(inline_advance, block_advance, hang, contributes_content)` (NEW, private) writing exactly the five line-state fields `place_item` writes today: `current_inline +=` (`:695`), `current_line_height = max(..)` (`:696`), `on_line = true` (`:697`), `any_rendered_content \|=` (`:698`), `current_line_last_hang =` (`:701`). `place_item` calls it **after** its soft-wrap check (`:690`), unchanged in behaviour. The marker path calls it **without** any soft-wrap check, with `hang = 0.0`. Marker arguments: PR-1b passes `(edge_advance, 0.0, 0.0, false)`; **PR-1c changes exactly two of them** to `(edge_advance, block_advance, 0.0, true)`. Separately, a marker whose `LogicalEdges` has a **non-zero inline-start or inline-end component** sets `last_placed_entity = None` (`:772`). | Round 4's CRIT 1: revision 4 gave the marker its own writes and left *entering the line* (`on_line`, `any_rendered_content`) unowned, so `finish()` (`:787`) and `flush_line` (`:210`) could never fire for Shape B. A shared core answers that **and** round 3's "the split duplicates `place_item`'s sequence" — there is now one copy, one owner. No soft-wrap check on the marker path: §1.3, an inline box boundary is not a break opportunity. `hang = 0.0`: round 4 showed leaving the hang stale is *affirmatively wrong* — css-text-3 §4.1.2 keys on "at the end of **a line**", the engine already encodes that at `pack/mod.rs:264-279`, and `place_item` already zeroes it for an atomic (`:701`, `full == trimmed`). Shaping break gated on **non-zero components, not the signed advance**: §1.4 says "non-zero", so `margin-left:-5px` and a compensating `-5px/+5px` pair both break, though both sum to an advance of 0 or less. |
| **M4** | Where does the box's rect come from, and how do the edges reach `LayoutBox`? | **Rect**: an open-box **stack** records each `InlineBoxStart`'s cursor; `InlineBoxEnd` pops it and pushes one `current_line_entity_rects` entry (`:706`) for `entity` spanning **start-cursor + inline-start edge → end-cursor** (read **before** the end marker's own advance) — the box's **content** span, never inflated by edges. Pushed by an explicit branch, not the `entity != parent_entity` guard (`:703`), which does not suppress a nested inline. `flush_line` **emits a partial rect for every still-open box before draining** and rebases its start-cursor to 0, so a box straddling a break yields one rect per line. **Edges**: carried to `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) as a **sibling `&HashMap<Entity, LogicalEdges>` parameter**, not as an `EntityBounds` member. | `assign_inline_layout_boxes` writes bounds into `LayoutBox.content` (`:81`) and `border_box() = content + padding + border`, so an edge-inflated rect double-counts (round 3 CRIT). Reading the end-cursor before the end advance is what keeps the end side from double-counting too (round 4). The flush-time **emit** is what revision 4's rebase-only version missed: `InlineBoxEnd` has not run at line N's flush, so line N's fragment was simply lost. A sibling map rather than an `EntityBounds` member because `EntityBounds` is a per-line geometry accumulator built by two struct literals (`pack/mod.rs:413`, `:505`) with `and_modify` arms (`:406`, `:498`) that would not re-set an element-constant on later lines — cf. [[feedback_writesite-audit-includes-struct-literal-ctors]]. |
| **M5** | What else changes when a line stops being phantom? | §2.3's "the line box **and its in-flow content**" is the rule: when the line exists, every entity on it commits its tentative rect through the existing seam (`:210`). No entity is specially withheld. §6 cell 21 pins the co-resident case. | Revision 1 fabricated a zero-edge rect; revision 2 withheld the box entirely; both were untrue because the geometry was missing. With M4 landed in PR-1b, by PR-1c the geometry is real and the ordinary commit path is simply correct. This also removes revision 2's collision with `#11-layoutbox-absence-unreachable` (#488) — no truthful box-absent signal is needed. |
| **M6** | Height of a line kept only by decoration | `InlineBoxStart` carries the element's resolved `line_height` and font identity; M3's shared core takes `current_line_height = max(block_advance)` with `block_advance` following the packer's existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`). **The memo does not claim §5.3/§2.2 conformance**: the block container's **root inline box** (§1.1) is unimplemented, so the line's height floor is missing. The text path already diverges identically — `<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today, with no marker involved. New slot **`#11-inline-root-inline-box`**, pre-existing class; §6 cell 23 pins the divergence so it stays distinguishable from a bug. | The direct authority is `body css-inline-3 line-layout` (§2.2) Note: "Empty inline boxes still have margins, padding, borders, and a **line-height**, and thus influence these calculations just like boxes with content." Revision 4 cited §5.3, which defines the strut per *box* and does not address the line-level question — round 4's finding. Disclosing a pre-existing divergence the new code depends on is the §4.3 pattern. |
| **M7** | Which line gets a strut baseline | `InlineBoxStart` records a tentative `current_line_box_baseline: Option<f32>` from the box's first-available-font metrics via `FontDatabase::query` + `font_metrics` (`crates/text/elidex-shaping/src/database.rs:60`, `:101`), keeping the `!is_vertical` guard (`:575`). `flush_line` promotes it into `first_baseline` **inside** the `if self.any_rendered_content` arm (`:210`) — never on a suppressed line — and only if `first_baseline.is_none()`. The field is reset in `flush_line`'s per-line block (`:432-439`), which thereby goes from five fields to six. | §1.5: a strut exists only for a glyphless box; §2.2 owns the line-level composition. No second flag: the text arm sets `first_baseline` at pack time (`:575`), so `is_none()` at flush already answers "did any glyph-bearing segment land here or earlier". Traced against all three orderings. `query`+`font_metrics` rather than `measure_text`, because a glyphless box has no string to shape and the metrics are string-independent anyway (`elidex-shaping/src/measurement.rs:55`) — round 4 flagged the two rows naming different entry points. ⚠ **Known residual**: if a line's text has no usable font, `measure_text` returns `None`, `first_baseline` stays `None`, and a co-resident box's tentative promotes on a line that does have glyphs. §6 cell 24 pins it as accepted. |
| **M8** | Intrinsic sizing | `min_content_inline_size` / `max_content_inline_size` (`inline/measure.rs:15`, `:44`) gain marker arms: each marker adds its `inline_start + inline_end` edge sum to the running measure — to the **current word's** width in the min-content pass — a box boundary is not a soft wrap opportunity (§1.3), so it cannot start a new min-content candidate. When there is no current word (a marker pair with no adjacent text, i.e. Shape B inside a shrink-to-fit box) the edge sum becomes a candidate **of its own**, since the box still occupies that much inline space and nothing can wrap inside it and to the total in the max-content pass. Their `collect_inline_items` calls (`:22`, `:51`) pass `0.0` as the containing inline size, the standard intrinsic-sizing treatment for percentages. **PR-1b**, with cell 25. | M3's advance makes a decorated inline occupy inline-axis space, so a shrink-to-fit `float`/`inline-block` containing one under-measures without this. Round 4: revision 4 asserted this "is in PR-1b's scope" in §9 with no M-row, no §3 row, no owner and no DoD line — which is relabelling, not scope. It gets a row because it is a real mechanism decision (which pass gets the edges, and how min-content treats a boundary). |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`) against `containing_inline_size`. Two of the seven bogus §5.3 cites live here (`:59`, `:114`); the other five are in `lib.rs:178`, `positioned/layout.rs:90`, `block/mod.rs:162`, `block/mod.rs:182`, `block/children/helpers.rs:213` — PR-1a fixes all seven in one commit (§3.1). |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair; selects the inline-axis edge pair from the physical `EdgeSizes` using `parent_style`'s writing mode and direction. Gains `containing_inline_size` on `collect_inline_items` (`:136`) / `collect_inline_items_inner` (`:194`). ⚠ `collect_inline_items` has **four** callers (verified 2026-08-02 via `grep -rn 'collect_inline_items(' crates/` minus the definition): `inline/mod.rs:154` (has the value), `inline/measure.rs:22` and `:51` (`min_content_inline_size`/`max_content_inline_size`, where a containing inline size is definitionally unavailable — see §9), and the test helper `inline/tests/mod.rs:17`. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get `PackItem` forms; they never become `FlowMember`s. |
| `crates/layout/elidex-layout-block/src/inline/pack/mod.rs` | `LinePacker` line state. **M3's `note_line_occupancy` lives here**, beside `place_item` (`:679`) which becomes its first caller. `flush_line` (`:209`) gains M4's open-box emit+rebase, M7's promotion, and a sixth per-line reset. The fabricated shaping citation at `:726` is rewritten by M3. |
| `crates/layout/elidex-layout-block/src/inline/mod.rs` | The two pre-pack gates PR-1a holds and PR-1c flips (`items.is_empty()` `:161`; the exhaustive `any_font` match `:190-200`), and §7's `clear_inline_flows` gating (`:637`). |
| `crates/layout/elidex-layout-block/src/inline/whitespace.rs` | `collapse_inline_whitespace` (`:41`) — M2's transparent arm. |
| `crates/layout/elidex-layout-block/src/inline/measure.rs` | `min_content_inline_size` (`:15`) / `max_content_inline_size` (`:44`) — M8. |
| `crates/core/elidex-plugin/src/logical.rs` | `LogicalEdges::from_physical` (`:186`) + `WritingModeContext::new` (`:27`) — the physical→logical mapping M1 routes through rather than hand-rolling. |
| `crates/layout/elidex-layout-block/src/inline/pack/inline_box.rs` (NEW) | **One cohesive concern: the open-box stack** — push on `InlineBoxStart`, pop-and-emit on `InlineBoxEnd`, flush-time partial emit + rebase. An `impl LinePacker` in a sibling module, the idiom `pack/fragment.rs:10` already uses (one algorithm, one invariant). ⚠ Revision 4 made this a four-concern bucket justified by `pack/mod.rs`'s line count; round 4 was right that this is the mechanical application CLAUDE.md subordinates to cohesion. The line-state core (M3) and the baseline promotion (M7) therefore stay with their existing owner in `pack/mod.rs`. |
| `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs` | `assign_inline_layout_boxes` (`:48`) — writes `LayoutBox.content` from bounds and, per M4, `padding`/`border`/`margin` from a **sibling `&HashMap<Entity, LogicalEdges>` parameter**. |
| `elidex-text` (facade over `elidex-shaping`) | `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60`) + `font_metrics` (`:101`) — M7's strut A/D, taken without shaping a string. |

### §5.3 Program slicing

Three shipping PRs on **disjoint invariant sets** (§2's rightmost column), plus two booked
slots. Each PR gets its own plan-memo and `/elidex-plan-review`.

* **PR-1a — item stream (invariants 2, 6). Behaviour-neutral.** M1 + M2. Marker variants, all
  consumers updated (`whitespace.rs` per M2; `atomic.rs:39`; `pack/items.rs`; the exhaustive
  match at `inline/tests/mod.rs:21-22`; `measure.rs` and `collect.rs:181` are `if let` and need
  none), `containing_inline_size` threaded, four bogus cites fixed. The packer gets a **no-op**
  `match pi` arm (`:528`). The two pre-pack gates are held at current behaviour:
  `items.is_empty()` (`inline/mod.rs:161`) excludes markers, and the `any_font` arm
  (`:190-200`) returns `false`. **Characterization tests for §6 cells 7–12 land here asserting
  today's behaviour**, since `grep -rn padding crates/layout/elidex-layout-block/src/inline/tests/`
  → **0 hits** (verified 2026-08-02) means the existing suite pins nothing about decorated
  inlines. No new slot.
* **PR-1b — geometry (invariants 3, 7).** M3 + M4 + M8. The shared line-occupancy core with
  markers passing `contributes_content = false`, inline-axis advance, shaping break at a
  decorated boundary, real `LayoutBox` edges, content-span rects, intrinsic-size contribution.
  **This is the engine-wide correction**: it changes painted output for every decorated inline
  with content. §6 cells 13–17 and 25 land here.
  ⚠ **Scope of "neutral", stated precisely** (round 4 CRIT 3): PR-1b leaves the **phantom-line
  predicate** unchanged, so no line's *existence* flips. It does **not** leave `line_count`
  unchanged — the advance moves the cursor, so following text reaches the wrap guard (`:690`)
  earlier and content-overflowing paragraphs gain lines. That is the correct consequence of
  §1.1's "inline-axis … respected between inline-level boxes", not a regression, and cell 13's
  displacement half plus cell 15 pin it. Shapes A and B stay lineless in PR-1b, but **not** because
  `on_line` stays false — the shared core sets `on_line` unconditionally, so `finish()` (`:787`)
  does flush. The flush takes the **discard** arm (`:423`) because `any_rendered_content` is
  false, so no `LineBox` is pushed and `current_block_offset` is not advanced (`:422` is in the
  commit arm). Neutrality comes from the discard arm. Consequence to note: a decoration-only IFC
  now runs a flush it did not run before — traced harmless, but stated rather than assumed.
* **PR-1c — existence (invariants 1, 4, 5).** M5 + M6 + M7. The clause-3 predicate per §1.2,
  the strut baseline, the commit consequence, and the two pre-pack gates flipped. **The delta to
  PR-1b's marker call is two arguments** (`block_advance`, `contributes_content`); everything
  else is already in place. §6 cells 1–12 and 18–24 land here. Closes
  `#11-line-box-decorated-inline-content` with its original DoD — line *and* rectangle — met,
  because PR-1b already made the rectangle real.
* **`#11-inline-box-decoration-splits`** (new slot, **own** deferral): css-inline-3 §2.1 box
  splitting across line boxes and bidi fragments, and **css-break-3 §5.4** `box-decoration-break`
  (the "no visual effect where the split occurs" rule — that string is CSS 2 §9.4.2 verbatim and
  appears nowhere in css-inline-3 §2.x; revision 4 mis-attributed it), plus fragmentation across
  the marker pair. Why deferred: it is a distinct invariant axis (box
  continuity across breaks) that only becomes reachable once a box has geometry, and folding it
  in would put a third axis into PR-1b. Trigger: PR-1c landing. Re-eval: 2026-11-01.
* **`#11-inline-root-inline-box`** (new slot, **pre-existing** class): the block container's
  root inline box (css-inline-3 §2), which sets every line box's height floor. Why deferred: it
  changes the height of *every* line box in the engine — verified pre-existing because the
  divergence is observable today on ordinary text (M6's example), with no marker involved, so
  `origin/main` already fails it. Trigger: any line-height correctness work, or a compat-survey
  hit. Re-eval: 2026-11-01.

Own-deferral count: **1** (`#11-inline-box-decoration-splits`). Within the per-PR ≤3 policy.

**Rejected**: widening `StyledRun` with edge fields (box-level data on a per-segment
measurement type; N copies for an N-segment span; reaches neither shape per §4.2); reading
`run.entity`'s `ComputedStyle` at pack time (reaches neither shape, same reason). Neither is
rejected on component-lookup cost — `collect.rs:36` uses borrowed component reads in the same
per-child loop, a normal idiom here.

## §6. Edge matrix

**PR-1a characterization (assert today's behaviour; PR-1c flips 1–12):**

1. `padding` alone (inline-axis) — line currently suppressed.
2. `border` alone, plus `border-style: none` + `border-width: 5px`. css-backgrounds-3 §3.2:
   "No border. Color and width are ignored (i.e., the border has width 0)". Applied at
   computed-value time (`elidex-style resolve/box_model/mod.rs:260`).
3. `margin` alone, **including negative** — requires M1's `resolve_box_model` sourcing.
4. **Percentage** `padding: 5%` / `margin: 2%` — against `containing_inline_size`.
5. **Block-axis edges only** (`padding-top`, `margin-top`) — **line stays suppressed in
   PR-1c too**, per §1.2's inline-axis restriction and CSS 2 §8.3. A non-regression cell in
   every PR; revisions 1–3 pinned the opposite.
6. All zero ⇒ no marker emitted at all (M1). Block-axis-only (`padding-top`) ⇒ marker **is** emitted (it must paint), cursor advance 0, and the line stays phantom — the emit/existence split in M1.
7. Shape A (decorated inline containing only collapsible white space).
8. Shape B (completely empty decorated inline).
9. `a<span style="padding:10px"> </span>b` — the M2 cell: the space must collapse against its
   neighbours exactly as today.
10. Nested: inner decorated / outer not, and the reverse — exercises M4's open-box stack.
11. Two inlines on one line, one decorated.
12. Decorated inline as the only IFC content — the `items.is_empty()` gate
    (`inline/mod.rs:161`) and the `any_font` probe (`:190-200`), held in PR-1a, flipped in
    PR-1c.

**PR-1b geometry:**

13. `<p>a<span style="padding:10px">text</span>b</p>` — the common case §4.3 is about. Three
    assertions: (a) the span's `LayoutBox` carries real edges and its border box is
    `content + padding` **once**, not twice; (b) **`b` is displaced by 20px** — the
    user-visible half of §1.1's "respected *between* inline-level boxes", which revision 4's
    cell asserted only for the box itself; (c) the painted background/border rect
    (`crates/core/elidex-render/src/builder/paint/mod.rs:68`, `:85`) matches the border box.
14. **Shaping breaks at the boundary** (§1.4): in `<p>a<span style="padding:1px"></span>b</p>`
    the two same-entity texts must **stop** coalescing into one `InlineFlowRun`. Contrast:
    with an all-zero-inline-axis-edge span they still coalesce (no marker is emitted at all
    per M1, so this is automatic).
15. **The boundary is not a wrap opportunity** (§1.3):
    `<p style="width:100px">aaaaaaaaaaaa<span style="padding:20px"></span>b</p>` — the box does
    not move to the next line at its own edge. Paired: `b` **does** wrap, and earlier than
    today, because the cursor is 40px further along (the PR-1b `line_count` change §5.3 states).
16b. **A soft wrap opportunity adjacent to a decorated boundary** — css-text-3 §5.5's other
    bullet puts the break at the box's **margin edge**, so the start edge must not be stranded
    on the previous line. ⚠ Not yet mechanised; see §12.2.
16. **`text-align` unaffected by the box** (§4.1.2): `<p style="text-align:center">abc
    <span style="padding:1px"></span></p>` — the trailing collapsible space still hangs;
    `current_line_last_hang` is not touched by M3.
17. **A decorated inline whose content wraps** — start marker on line N, end marker on line
    N+1; M4's per-line rebase must yield one rect per line, each with that line's
    `block_start`.

25. **Shrink-to-fit** (M8): a `float: left` / `display: inline-block` containing
    `<span style="padding:10px">x</span>` must be 20px wider than the same box without the
    padding — `min_content_inline_size` and `max_content_inline_size` both.

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

**Test placement** (§9): PR-1a/1c cells land in a **new** test module, not in
`tests/text_height/basic.rs` (303) or `relpos_subflow.rs` (915). The relpos facet of cell 10/11
lands with the new module too, not in the 915-line file.

## §7. Downstream

* `line_count`, IFC `height`, block cursor — PR-1c only; PR-1b changes no line's existence.
* `first_baseline` — moves only on a glyphless line (M7); cell 22 asserts both directions.
* `entity_bounds` / `getClientRects` — PR-1b gives every decorated inline a real border box;
  PR-1c adds the co-resident commit (cell 21).
* **Painted output** — PR-1b changes background/border rendering for **every** decorated
  inline (`crates/core/elidex-render/src/builder/paint/mod.rs:68`, `:85`). Largest delta in the
  program; cell 13 is its pin.
* `last_placed_entity` / persisted run shape — deliberately changed by M3 (§1.4); cell 14.
* `current_line_last_hang` — deliberately unchanged (M3); cell 16.
* Static positions of following abspos (`inline/pack/mod.rs:97`).
* `current_line_runs` / `flow_lines` — markers are never `FlowMember`s, so member-less buckets
  keep being dropped by design; no ordinal dependency (§2 withdrawn row).
* `clear_inline_flows` probe gating — a decoration-only IFC stops taking the early return at
  `inline/mod.rs:161` (unconditional clear) and starts taking the full path (`:637`,
  `!env.is_probe`-gated). **PR-1c**, per §5.3.
* Fragmentation (`inline/pack/fragment.rs`) and `ColumnFlowSlice`.

## §8. Definition of done

**PR-1a** (item stream, behaviour-neutral): both marker variants reach `pack/items.rs`; every
exhaustive match handles both (`inline/whitespace.rs:41`, `atomic.rs:39`, `inline/mod.rs:190-200`,
`inline/tests/mod.rs:21-22`); `LogicalEdges` built per M1 from the element's own style;
`containing_inline_size` threaded through all four `collect_inline_items` callers; the two
pre-pack gates hold current behaviour; **all seven** bogus §5.3 cites fixed; new variants carry
docstring citations to their §3 rows; cells 1–12 land as characterization tests; zero behaviour
change. **Dead-field rule, resolved**: PR-1a's payload is `entity` + `LogicalEdges` only. M6/M7's
`line_height` and font identity are added by **PR-1c**, and `group_key` by
`#11-inline-box-decoration-splits` — neither ships unread, so no `#[allow(dead_code)]` is needed
and revision 4's rule and payload no longer contradict. (Consequence, accepted: with identical
payloads the two variants are distinguished only by position in the stream; that is the
distinction PR-1b's stack and PR-2's split rule both need, so it is a real one — cf. lesson #276,
which forbids variants whose *state shape* is redundant, not variants whose *role* differs.)
**PR-1b** (geometry): cells 13–17 and 25; §7's paint delta covered; no double-counted edges on
either side; the `inline/pack/mod.rs:726` shaping citation corrected (§3.1).
**PR-1c** (existence): cells 18–24 plus the flipped 1–12; every §7 consumer checked; the ⚠ caveat
at `inline/pack/mod.rs:110` retires, naming the root-inline-box divergence and pointing at
`#11-inline-root-inline-box`; slot closes.

**Explicitly not covered, recorded rather than dropped**: css-inline-3 §5.3's "or if it contains
only glyphs from fallback fonts" strut condition — elidex has no fallback-provenance signal.
Folded into `#11-inline-root-inline-box`, **and that slot's trigger is extended to name it**
(round 4: revision 4's fold had a destination whose trigger would never surface it).

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires, and revision 5 honours it.** The
  trigger is "the next change that touches `layout_inline_context_fragmented`'s body"; PR-1a
  touches `:161`/`:190` and PR-1c the persist block — which is the slot memo's **seam 3**
  (`mod.rs:411-637`), the seam it describes as having a ready-made module boundary.
  **Disposition**: a **standalone prereq split PR at seam 3, before PR-1a** — the third option
  CLAUDE.md prescribes ("feature 着手前に standalone な prereq split として分割する — feature PR に
  bundle しない"), and the lane's own #500 precedent. ⚠ Revision 4 instead proposed *amending the
  slot's trigger* to carry a size qualifier; round 4 was right that this is trigger-weakening —
  it answers the file-size question the trigger never asked, converts a cohesion trigger into
  the line-count mechanism CLAUDE.md subordinates, and mutates another slot's ratified surface
  from inside this umbrella's ledger. Withdrawn.
* **File growth**: `pack/mod.rs` 791, `inline/mod.rs` 785, `relpos_subflow.rs` 915,
  `tests/text_height/basic.rs` 303 (all verified 2026-08-02). New packer surface goes to
  `pack/inline_box.rs` (§5.2); new tests to a new module (§6). `inline/mod.rs`'s growth across
  the three PRs is a signature change, two gate predicates and the persist-block touch — no new
  seam decision needed, and re-checked at PR-1c.
* **`#11-inline-relayout-box-staleness`** (`inline/pack/boxes.rs:96`) — pre-existing, but
  **PR-1b widens its observable surface**: today an inline's stale `LayoutBox` is
  indistinguishable from a fresh one because the edges are always zero, and after M4 a stale
  box carries stale edges. Recorded here rather than claimed non-widening (revision 3 claimed
  the latter). Both this slot and `#11-inline-align-clientrects-nonpersist-path` are already
  ledger-marked to fold into terminal-Z C-3/C-4; PR-1b's widening is a note on that fold.
* **`#11-layoutbox-absence-unreachable`** (#488) — no longer relevant: M5 keeps no entity's box
  withheld, so no truthful box-absent signal is required.
* **Intrinsic sizing** — `measure.rs` is `if let InlineItem::Text`, so markers are invisible to
  `min_content_inline_size`/`max_content_inline_size`. After M3 a decorated inline contributes
  inline-axis size, so shrink-to-fit widths would under-measure. **This is in PR-1b's scope**,
  not deferred: cell 13 must include a `float`/`inline-block` shrink-to-fit assertion, and the
  two `measure.rs` callers of `collect_inline_items` (§5.2) resolve percentages against 0 there
  — the standard intrinsic-sizing treatment, stated rather than assumed. The fourth caller,
  the test helper at `inline/tests/mod.rs:17`, takes 0 for the same reason.
* **`#11-css2-spec-label-normalisation`** — this memo now cites css-inline-3 for the model, so
  its remaining CSS 2 cites are §8.3, §9.4.3 and §16.6.1 only. Adjacent `CSS 2.1 §` lines in
  touched files are left alone; the slot requires a single cross-crate commit.
* Ruby annotations (§2.3 clause 4) — unimplemented engine-wide.

## §10. Revision history and ledger actions

| Rev | Round | Result | What it bought |
|---|---|---|---|
| 1 | R1: 0 CRIT / 17 IMP | rejected | The slot is an umbrella (§4.3); both shapes are invisible to the packer (§4.2). |
| 2 | R2: 2 CRIT / 18 IMP | rejected | `place_item` cannot simply host the box (its nine writes, §4). CRIT was an author patch inverting the percentage basis. |
| 3 | R3: 8 CRIT / 24 IMP | rejected | The plan was on superseded text. Two CRITs were author patches; the `flush_line`/`on_line` and double-count defects were real. |
| 4 | R4: 11 CRIT / 29 IMP | rejected | Re-anchor confirmed (all three spec CRITs closed). **No CRIT was an author patch** — the no-patching rule held. Mechanism gaps surfaced. |
| 5 | R5 pending | — | One shared line-occupancy owner (M3); `LogicalEdges` carrier; flush-time open-box emit; intrinsic sizing as M8; seam-3 prereq split honoured. |

Ledger actions, each attributed:

| Action | PR |
|---|---|
| Register `#11-line-box-decorated-inline-content` in `project_open-defer-slots.md` (the SoT per MEMORY.md; it carries `#11-inline-align-clientrects-nonpersist-path`, `#11-inline-relayout-box-staleness`, `#11-justify-subflow-line-unified` but not this one) | seam-3 prereq PR |
| Register the #497 carves, also absent: `#11-line-box-decorated-inline-content`, `#11-css2-spec-label-normalisation`, `#11-inline-fragmented-fn-decomposition` (verified 2026-08-02, 0 hits each) | seam-3 prereq PR |
| Close `#11-inline-fragmented-fn-decomposition` | seam-3 prereq PR |
| Open `#11-inline-box-decoration-splits` (own) with Why / trigger / date from §5.3 | PR-1b |
| Open `#11-inline-root-inline-box` (pre-existing) with Why / trigger / date from §5.3, **trigger extended to name the fallback-font strut condition** (§8) | PR-1c |
| Enrich `#11-inline-relayout-box-staleness`'s SoT entry with PR-1b's widening (§9) | PR-1b |
| Rewrite `project_line-box-decorated-inline-content.md`, which still records the rev-3 program shape and the CSS 2 §10.8 anchor as current fact | PR-1c |

## §11. Round-4 outcome — revision 5 required

Round 4 = **11 CRIT / 29 IMP / 22 MIN**. Two things changed in character from round 3.

**The no-inter-round-patching rule held, and it worked.** Axis 3 verified it mechanically
(revision 4 is a single rewrite commit, no patch commits after it). **None of round 4's CRITs
is an author patch** — the class that produced the top-severity finding in both R1→R2 and
R2→R3 is absent. The mandated Step 1.5 dry-run's five findings were handed to the round
unpatched, and three of them were independently confirmed (M6's missing entry point, M4's
missing flush-time emit, M1's wrong style for the physical→logical selection), one confirmed
as a non-finding (M2 collapse transparency), one confirmed with a sharper reading (§7 asserts
the opposite of what Shape A/B actually do).

**The findings moved from framing to mechanism.** Round 4 closed, with verification rather
than assertion: all three round-3 spec CRITs (§7.3 reversal, §5.5 soft-wrap, §2.3 inline-axis
re-anchor); the border-box double-count; the `last_placed_entity` direction; the
`current_line_has_glyphs` redundancy; the `:702`/`:706` cite; the caller count; the
`#11-inline-root-inline-box` pre-existing classification; `inline/mod.rs` growth; cell 19's
destination; the "approved umbrella" phrasing; the relayout-staleness non-widening claim; and
the 4→2→3 PR reslice (§2's eight pairs each resolve to exactly one M-row, and M1–M7 partition
1a{M1,M2} / 1b{M3,M4} / 1c{M5,M6,M7}).

### §11.1 Open CRITs

1. **PR-1c has no write site for the existence flip** (Axes 1, 2; dry-run). Revision 3 had
   `note_inline_box`; revision 4 dropped the name when the concern moved to PR-1c and never
   replaced it. M6 cites `:696` and §3 cites `:698`, both `place_item`-internal lines the
   marker path provably cannot reach — and `finish()` (`:787`) flushes only `if on_line`,
   `flush_line` emits only `if any_rendered_content` (`:210`). As written, cells 8 and 12
   cannot pass and the slot cannot close.
2. **§1.3's "an inline box that does not fit overflows" is overbroad** (Axis 4). css-inline-3
   §2.1 says it is **split**; overflow is the unsplittable exception. **Corrected in §1.3
   above.** Consequence still open: splitting is now owned by nobody — §3 row 3 defers it to
   "PR-2", a unit §5.3 does not define.
3. **PR-1b is not `line_count`-neutral** (Axis 3). M3 inflates `current_inline` without arming
   `:690`, but the *next* text segment evaluates `:690` against the inflated cursor, so
   following text wraps earlier: `line_count`, IFC height and the block cursor move in PR-1b.
   §5.3's neutrality claim and §7's "PR-1c only" bullet are both false as written.
4. **M3's shaping-break gate is `inline_advance > 0`; §7.3's rule is "non-zero"** (Axis 2). A
   negative margin, or a compensating pair summing to 0, emits markers but does not break
   shaping.
5. **M3 leaves `current_line_last_hang` stale, and that is affirmatively wrong** (Axis 2).
   css-text-3 §4.1.2 keys on "at the end of **a line**", not on the last text segment; the
   engine already encodes the distinction at `pack/mod.rs:264-279` and already zeroes the hang
   for an atomic (`:701`). §6 cell 16 pins the wrong behaviour.
6. **M4's edges have no carrier** (Axes 1, 2). `current_line_entity_rects` is
   `Vec<(Entity, InlineLineRect)>` and `InlineLineRect` (`pack/boxes.rs:18-27`) is four
   geometry `f32`s; `EntityBounds` is built only from those rects at `:413`/`:505`. The
   inline-axis-only payload M1 specifies also cannot fill a physical four-sided
   `LayoutBox` edge set.
7. **M4's straddling box loses line N's fragment** (Axis 2; dry-run). A rebase is necessary but
   not sufficient — the stack needs a flush-time *emit*, since `InlineBoxEnd` has not run.
8. **The bogus-citation class is seven sites, not four** (Axis 5). **Corrected in §3.1 above**,
   including why the undercount recurred three rounds running.

### §11.2 What revision 5 must do

1. Name the PR-1c entry point that writes `on_line` / `current_line_height` /
   `any_rendered_content`, and discharge §4's `flush_line` obligation for it.
2. Restate PR-1b's neutrality honestly: phantom-predicate-neutral, **not** `line_count`-neutral.
   Add the displacement cells Axis 3 names (following text wraps earlier; `b` shifts by the
   edge sum) — the user-visible half of §1.1 that cell 13 does not assert.
3. Gate the shaping break on **non-zero components**, not on the signed advance.
4. Zero `current_line_last_hang` at a marker with non-zero inline-axis edges, matching the
   atomic treatment; re-pin cell 16.
5. Give the edges a real carrier (Axis 1 suggests `LogicalEdges`, `crates/core/elidex-plugin/src/logical.rs:167`,
   which also preserves the block-axis pair `LayoutBox` needs and is already used by
   `block/mod.rs:158`); add the flush-time emit for open boxes.
6. Own splitting: point §3 row 3 at `#11-inline-box-decoration-splits`, and re-anchor that
   slot's "no visual effect where the split occurs" on **css-break-3 §5.4**, not css-inline-3
   §2 (Axis 4: the string is CSS 2 §9.4.2 verbatim and appears nowhere in css-inline-3 §2.x).
7. Give intrinsic sizing an M-row, a §3 row, a `measure.rs` layer row and a DoD line, or a slot
   — §9's relabelling is not scope (Axis 3).
8. Resolve §8's dead-field rule against M1's payload; `group_key` has no reader in 1a/1b/1c.
9. Re-anchor M2 on **css-text-3 §4.1.1** (the current statement of cross-boundary collapsing),
   fix §3 row 12's malformed "CSS Text 3 §16.6.1", and add rows for css-inline-3 §2.1 and §2.2.
10. Re-derive `#11-inline-fragmented-fn-decomposition`'s disposition: Axis 5 is right that
    amending another slot's trigger from this memo is a design change owed its own review, and
    that CLAUDE.md's third option — a standalone prereq split PR at seam 3 — was never
    considered.
11. Update the slot memo itself (Axis 5): it still records the rev-3 program shape and the
    CSS 2 §10.8 anchor as current fact.

### §11.3 One rule refinement, accepted

Axis 3 is right that this revision's "no inter-round patching" was too broad. The lesson it
came from targets **reactive** patches to reviewer findings; it should not have frozen the
author's own mandated dry-run findings, which cost this round three re-discoveries. The rule
for revision 5: **fix what the dry-run finds, do not add what a reviewer's framing suggests** —
and re-derive, never sentence-patch, either way.

## §12. Revision 5 — what changed, and what is knowingly still open

### §12.1 Changes, each answering a round-4 CRIT or IMP

1. **M3 restructured: one owner, two callers.** Round 4's CRIT 1 was that revision 4 gave the
   marker its own writes and left *entering the line* unowned. Rather than name a second
   function, revision 5 extracts `note_line_occupancy` — the five line-state writes
   `place_item` already makes — and has both `place_item` and the marker path call it. This
   also closes round 3's "the split duplicates `place_item`'s sequence", makes the PR-1b→1c
   delta **two arguments**, and puts the hang zeroing in the shared signature (round 4 CRIT 5).
2. **Shaping break gated on non-zero components** (CRIT 4), matching §1.4's wording rather than
   the signed advance.
3. **`LogicalEdges` as the payload** (CRIT 6): keeps the block-axis pair alive for
   `LayoutBox.padding/border/margin`, and routes physical→logical through the existing
   engine-independent API instead of hand-rolling it at the collect site — from the element's
   **own** style, which is what `direction` requires.
4. **Edges reach `assign_inline_layout_boxes` as a sibling map**, not an `EntityBounds` member
   (CRIT 6): `EntityBounds` is a per-line accumulator with two struct-literal ctors and
   `and_modify` arms that would not carry an element-constant.
5. **Flush-time emit for open boxes** (CRIT 7) — the rebase alone lost line N's fragment.
6. **PR-1b's neutrality restated precisely** (CRIT 3): phantom-predicate-neutral, **not**
   `line_count`-neutral, with the displacement pinned by cells 13(b) and 15.
7. **Splitting owned** (CRIT 2 residual): §3 points at `#11-inline-box-decoration-splits`, and
   the "no visual effect" rule is re-anchored on **css-break-3 §5.4** — the string is CSS 2
   §9.4.2 verbatim and appears nowhere in css-inline-3 §2.x.
8. **Seven cite sites, not four** (CRIT 8), with the concept grep as provenance.
9. **M8 for intrinsic sizing** — a real row, §3 row, layer row, DoD line and cell 25, replacing
   revision 4's relabelling.
10. **M2 re-anchored on css-text-3 §4.1.1 step 4**; **M6 on css-inline-3 §2.2's Note**; §1.2's
    block-axis exclusion re-grounded on §5.3 + `line-fit-edge: leading` rather than over-reading
    CSS 2 §8.3; §1.4 gains its provenance line; §3 gains rows for §2.1, §2.2, §2.3(a), §5.3,
    css-break-3 §5.4 and the intrinsic contribution, and its malformed "CSS Text 3 §16.6.1" row
    is replaced. K/M recounted **by script** (23 rows, 7 specs).
11. **Layer table completed** — `pack/mod.rs`, `inline/mod.rs`, `whitespace.rs`, `measure.rs`,
    `logical.rs` added; `pack/inline_box.rs` narrowed from a four-concern bucket to the one
    cohesive concern (the open-box stack), since round 4 was right that a line-count rationale
    is the mechanical application CLAUDE.md subordinates to cohesion.
12. **Dead-field rule resolved against the payload** (§8): PR-1a carries `entity` + `LogicalEdges`
    only; the later fields arrive with their readers.
13. **`#11-inline-fragmented-fn-decomposition` honoured, not weakened** (§9): a standalone
    prereq split PR at seam 3 — the option CLAUDE.md prescribes and revisions 1–4 never
    considered. Revision 4's proposal to amend another slot's trigger is withdrawn.
14. **§10's ledger actions all attributed**, carves enumerated, and the slot memo's own rewrite
    added as an action.

### §12.2 Knowingly open — for round 5 to judge

* **Cell 16b has no mechanism.** css-text-3 §5.5's other bullet puts a break adjacent to a box
  at the box's **margin edge**; M3 advances the start edge unconditionally at the marker, so a
  wrap triggered by the *following* text leaves that edge stranded on the previous line. The
  cell is written; the mechanism is not. Fixing it plausibly needs the start edge to be
  provisional until the first content after it is placed — which touches invariant 7 and may
  belong with `#11-inline-box-decoration-splits`.
* **Nested boxes and the shared core's `block_advance`.** With M3 taking a `max`, a nested pair
  contributes `max`, not a sum — correct per §2.2 (each box contributes its own layout bounds),
  but §6 cell 10 asserts only the rect stack, not the height.
* **`#11-inline-box-decoration-splits` now carries two specs** (css-inline-3 §2.1 splitting,
  css-break-3 §5.4 decoration-break). It may be two slots.

## §13. Round-5 outcome — the failure mode is now identified, and it is procedural

Round 5 = **10 CRIT / 33 IMP / 28 MIN**. CRIT counts across the five rounds: 0 → 2 → 8 → 11 → 10.
**The plan is not converging**, and round 5 identified why in a form precise enough to fix.

### §13.1 The mechanism: fix-at-the-quoted-site, then record closed

Axis 3 named it: *"rev 5 rewrote the site a round-4 finding quoted and left the mirror sites
carrying the pre-fix statement, while §12.1 records the finding as closed."* Four of round 5's
findings are that one defect:

| Decision changed in rev 5 | Site fixed | Mirror site left stating the old decision |
|---|---|---|
| Payload is four-sided `LogicalEdges` from the element's own style | §5.1 M1 | §5.2's `collect.rs` row still says "inline-axis edge pair … using `parent_style`" |
| PR-1b moves `line_count` | §5.3 | §7's first bullet still routes `line_count`/height/cursor to "PR-1c only" |
| Intrinsic sizing became M8 with cell 25 | §5.1, §6 | §9's rev-4 paragraph survives verbatim, still pinning it to cell 13 |
| Program is M1–M8 / nine pairs | §2, §5.1 | §11 still says "eight pairs … M1–M7" |

This is [[feedback_verified-claims-go-stale-under-own-later-edits]] **inside a single revision**.
The corrective is mechanical and was not being done: after changing a decision, **grep the memo
for every site that restates it** and re-derive each — a concept sweep, not a patch at the line
the finding quoted. Same lesson as the seven-vs-four citation class, applied to the memo itself
rather than to the codebase.

### §13.2 I violated §11.3's rule in the revision that introduced it

§9's disposition for `#11-inline-fragmented-fn-decomposition` adopted round 4's framing **and its
citation** verbatim: "a standalone prereq split PR at seam 3 — the third option CLAUDE.md
prescribes". Verified 2026-08-02: CLAUDE.md's clause is scoped to **">1000行 file を触る際"**, and
`inline/mod.rs` is **785 lines**. #500, cited as precedent, split files of 1125 and 1037 — both
over the gate. The disposition may still be right on grounds the memo does not state (the slot's
own trigger; the 700–800 band per [[feedback_touch-time-split-means-while-writing]]), but the
ground it does state is out of scope. §11.3 said "do not add what a reviewer's framing suggests";
this is precisely that, one section later.

### §13.3 All three pre-dispatch dry-run fixes carried new findings

The refined rule (fix what the dry-run finds) was applied honestly — Axis 3 verified rev 5 is a
single commit with the fixes disclosed inline at each site. But each fix was a patch, not a
sweep:

* **Widened emit predicate** → the *existence* half was left unassigned (`contributes_content` is
  a constant `true` in M3, so a `padding-top`-only inline would keep its line, contradicting M1,
  §1.2 and cells 5/6), and widening to four sides collided with `LayoutBox`'s **three separate**
  edge sets, which one `LogicalEdges` cannot fill.
* **§5.3's neutrality reason** → correct for Shape A, wrong for Shape B (which never reaches the
  packer, because PR-1a's held `items.is_empty()` gate excludes markers). **Third consecutive
  wrong mechanism account of the same claim** (R3 `on_line`, R4 the same, R5 the discard arm).
  Also missed `on_line`'s second reader — the soft-wrap guard at `:690`, now armed on a fresh
  line by a leading marker.
* **M8's no-adjacent-word rule** → `min_content_inline_size` (`inline/measure.rs:15-37`) is
  `max_word = max_word.max(m.width)` per word per item, with **no running candidate and no
  cross-item joining** (verified). Neither M8 rule has a site; cell 25's min-content half cannot
  pass. Implementing it is an accumulator restructure the memo does not disclose.

### §13.4 What revision 6 must do differently

Not another patch round. The method changes:

1. **Sweep, don't patch.** For every decision revision 6 changes, grep the memo for each site
   restating it and re-derive all of them in the same edit. Record the grep, as §3.1 now does for
   the citation class.
2. **Re-derive every ground taken from a reviewer**, including its citation's scope. §13.2 is the
   cost of not doing this once.
3. **Do not mark a round-N finding closed without naming the sweep** that closed it. §12.1's
   fourteen "closed" items produced four re-openings.

Substantive worklist, on top of that method: give the clause-3 existence predicate an owner
distinct from the emit predicate; carry three edge sets, not one; state the logical→physical
return leg into `LayoutBox` (`assign_inline_layout_boxes` has `is_vertical` but no `direction`);
scope M8 to max-content and give min-content its own disposition or name the restructure;
re-derive §9's prereq-split ground; address the non-persist arm's missing per-entity fold
(unaddressed since round 4); split §5.3's neutrality account per shape and credit the held gate;
add `direction: rtl` and `writing-mode: vertical-rl` cells; route cell 16b out of PR-1b's DoD;
and re-scope the seam-3 PR (it closes one of the slot's three seams and PR-1a's touch sites lie
outside it).
