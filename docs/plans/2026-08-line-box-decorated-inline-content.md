# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's task, user-approved 2026-08-02.
Edge-dense ⇒ every PR under this umbrella goes through `/elidex-plan-review` before
implementation, and external review runs `/external-converge` from round 1.

All premises verified against `154bac3f`; every spec quote below was resolved directly with
`.claude/tools/webref`, not carried from a reviewer or from a code comment.

**Revision 4 — re-derived from premises, not patched.** Round 3 (8 CRIT) established that
revisions 1–3 were anchored on **superseded CSS 2 text**. `css-inline-3` §2 is the current
home for the whole subject and it settles, in one place, three things the earlier revisions
each got wrong by reading CSS 2 sections in isolation. §10 records the revision history and
what each round bought.

⚠ Authoring rule in force for this revision, per [[feedback_findings-cluster-in-self-added-scope]]:
each of rounds 1→2, 2→3 had its top-severity finding introduced by an author patch applied
*between* rounds. Revision 4 is a rewrite from the re-anchored premise; **no sentence-level
inter-round patching is applied before round 4 dispatches.**

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
phantom. This is consistent with CSS 2 §8.3 ("vertical margins will not have any effect on
non-replaced inline elements") rather than in tension with it — revision 3 recorded a
"tension" that existed only because it read the 1998 clause as axis-agnostic.

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

## §2. Coupled invariants

| # | Invariant | Site |
|---|---|---|
| 1 | **Line existence** — §2.3's five clauses | `any_rendered_content` (`inline/pack/mod.rs:115`) |
| 2 | **Item-stream integrity** — cross-run collapse lookback; positional iteration | `inline/whitespace.rs:59`; `measure.rs`, `atomic.rs`, `pack/items.rs` |
| 3 | **Commit-on-content** — per-line rects/runs commit or discard | `inline/pack/mod.rs:116`, `:210` vs `:423` |
| 4 | **Baseline provenance** — §5.3 strut vs glyphs | `inline/pack/mod.rs:575` |
| 5 | **Line-box height composition** — §5.3 layout bounds, incl. the root inline box | `inline/pack/mod.rs:696` |
| 6 | **Group keying** — relpos/sticky sub-flows | `inline/collect.rs:286` |
| 7 | **Cursor/advance integrity** — wrap, trailing hang, shaping runs | `inline/pack/mod.rs:690`, `:701`, `:772` |

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
| CSS Inline 3 §2 Inline Layout Model | inline-axis edges respected between boxes | start/end edge advance | `advance_inline_box_edges` (NEW) — **PR-1b** | ✓ | yes |
| CSS Inline 3 §2 Inline Layout Model | root inline box | block container's anonymous inline box | **NOT implemented — M6, new slot** | ✗ (pre-existing, disclosed) | yes |
| CSS Inline 3 §2 Inline Layout Model | inline box exceeding the line is split | split fragments across line boxes | **PR-2** | ✗ (deliberate, §5.3) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 3 | non-zero **inline-axis** margin/padding/border | clause-3 predicate → `any_rendered_content` (`inline/pack/mod.rs:698`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 5 | forced line break | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence | line box *and its in-flow content* do not exist | commit/discard seam (`inline/pack/mod.rs:210` vs `:423`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Layout Bounds | glyphless / fallback-only box | strut with first-available-font metrics | tentative baseline — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Layout Bounds | half-leading | `A′ = A + L/2` | existing formula (`inline/pack/mod.rs:581`) — unchanged | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | break opportunities | inline box boundary is **not** one | `advance_inline_box_edges` must not arm `:690` — **PR-1b** | ✓ | yes |
| CSS Text 3 §7.3 Shaping Across Element Boundaries | shaping break | non-zero inline-axis edge separates the units | `last_placed_entity` coalescing (`inline/pack/mod.rs:744`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | trailing collapsible space | hangs; excluded from alignment | `current_line_last_hang` (`inline/pack/mod.rs:701`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §16.6.1 → CSS 2 §16.6.1 White space processing model | collapsing across boundaries | markers must be collapse-transparent | `collapse_inline_whitespace` (`inline/whitespace.rs:41`) — **PR-1a** | ✓ | yes |
| CSS 2 §8.3 Margin properties | non-replaced inline elements | vertical margins have no effect | consistent with §2.3 clause 3; no code touch | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline | `collect.rs:286` sub-flow keying — **PR-1a** | ✓ | yes |
| CSS Box Model 3 §3.1 / §4.1 | percentage margin/padding | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract Dimensions | inline size ≡ logical width | basis identity in vertical modes | same | ✓ | yes |
| CSS Backgrounds 3 §3.2 border-style | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time (`elidex-style resolve/box_model/mod.rs:260`) | ✓ | yes |

**Breadth**: K=6 specs (CSS Inline 3, CSS Text 3, CSS 2, CSS Box Model 3, CSS Writing Modes 4,
CSS Backgrounds 3), M=17 entries (verified 2026-08-02 — data rows counted directly above).
**Split decision**: K≥6 ⇒ SPLIT-DEFAULT. The plan **is** split into three shipping PRs on
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
| **M1** | What enters the item stream, carrying what? | Two `InlineItem` variants, `InlineBoxStart` / `InlineBoxEnd`, emitted around the recursion at `inline/collect.rs:291` for every inline with **at least one non-zero inline-axis edge**. `InlineBoxStart` carries `entity`, the resolved inline-axis `EdgeSizes`, and the enclosing recursion level's `group_key`; `InlineBoxEnd` carries `entity` and its own inline-axis edges only. Edges resolved by `resolve_box_model` (`helpers.rs:116`) against **`containing_inline_size`**, threaded into `collect_inline_items` (`inline/collect.rs:136`). Mirrored in `PackItem` (`inline/pack/items.rs:18`); never a `FlowMember`. | §1.2 clause 3 is inline-axis-only, so the predicate is too. The two payloads **differ** (the end marker needs no `group_key`), which is what justifies two variants rather than one with a `bool` — cf. lesson #276. `resolve_box_model` is mandatory: `sanitize_padding` (`:96`) resolves percentages against 0 and `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative, so neither expresses a percentage or a negative margin. Basis = logical width = inline size (`css css-box-4 padding-top`; css-writing-modes-4 §6.1), which is `resolve_padding`'s documented contract (`helpers.rs:57-62`) and what `elidex-layout-grid/src/lib.rs:177` and `inline/atomic.rs:22` already pass. **Physical→logical**: `resolve_box_model` returns physical edges; the inline-axis pair is selected by writing mode + direction at the collect site, which has `parent_style`. |
| **M2** | Collapse-pass arm | **Transparent** — same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so interleaved markers are skipped by construction — including an adjacent `Start`/`End` pair. | CSS 2 §16.6.1: inline box boundaries do not stop collapsing. A barrier would change `a<span style="padding:10px"> </span>b`, currently-correct markup. |
| **M3** | How do the edges take space, and what do they break? | `advance_inline_box_edges(inline_advance)` does `current_inline += inline_advance` and **nothing else** — in particular it does **not** arm the soft-wrap guard (`:690`), does not call `flush_line`, and does not touch `current_line_last_hang` (`:701`). It **does** clear `last_placed_entity` (`:772`) when `inline_advance > 0`, so the next text segment starts a fresh run. | §1.3: an inline box boundary is not a soft wrap opportunity; a box that does not fit overflows. Not calling `flush_line` also avoids inheriting `place_item`'s unstated obligation to re-establish `on_line`/`any_rendered_content` afterwards (§4). §1.4: shaping **must** break at a boundary with non-zero inline-axis edges — so the coalescing at `:744` must stop there, which is exactly what clearing `last_placed_entity` does. Leaving `current_line_last_hang` alone is correct because §4.1.2's hang belongs to the last *text* segment; the box does not create or consume one. |
| **M4** | Where does the box's rect come from, and what is in it? | `InlineBoxEnd` pushes one entry into `current_line_entity_rects` (`:706`) for `entity`, spanning **start-cursor + start-edges → end-cursor** — i.e. the box's **content** span, *not* inflated by edges — via an explicit branch, not the `entity != parent_entity` guard at `:703` (which does not suppress a nested inline). The edges themselves reach `LayoutBox.padding/border/margin` through a new `EntityBounds` member, since `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) receives neither the edges nor `containing_inline_size`. Open boxes are tracked in a **stack** (nesting, §6 cell 10); `flush_line` rebases each open box's start-cursor to 0 so a box straddling a line break produces one rect per line. | `assign_inline_layout_boxes` writes bounds into `LayoutBox.content` (`inline/pack/boxes.rs:81`) and `border_box() = content + padding + border` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:140`). A rect already inflated by the edges would therefore double-count them in background/border paint (`crates/core/elidex-render/src/builder/paint/mod.rs:68`, `:85`). Today the two readings coincide only because the edges are hard-zero — which is precisely what this row changes. |
| **M5** | What else changes when a line stops being phantom? | §2.3's "the line box **and its in-flow content**" is the rule: when the line exists, every entity on it commits its tentative rect through the existing seam (`:210`). No entity is specially withheld. §6 cell 21 pins the co-resident case. | Revision 1 fabricated a zero-edge rect; revision 2 withheld the box entirely; both were untrue because the geometry was missing. With M4 landed in PR-1b, by PR-1c the geometry is real and the ordinary commit path is simply correct. This also removes revision 2's collision with `#11-layoutbox-absence-unreachable` (#488) — no truthful box-absent signal is needed. |
| **M6** | Height of a line kept only by decoration | `InlineBoxStart` carries the element's resolved `line_height` and font identity; the packer takes `current_line_height = max(block_advance)` (`:696`) with `block_advance` following the existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`). **The memo does not claim §5.3 conformance**: css-inline-3 §2's root inline box is unimplemented, so the line's height floor from the block container is missing. The text path already diverges identically (`<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today). New slot **`#11-inline-root-inline-box`**, pre-existing class. | Disclosing a pre-existing divergence that new code *depends on* is the §4.3 pattern; silently inheriting it while quoting §5.3 is what revision 1 did. |
| **M7** | Which line gets a strut baseline | `InlineBoxStart` records a *tentative* `current_line_box_baseline: Option<f32>` from the box's own first-available-font metrics (existing `measure_text` probe, keeping the `!is_vertical` guard at `:575`); `flush_line` promotes it into `first_baseline` only if `first_baseline.is_none()`. **No second flag is needed**: the text arm sets `first_baseline` at pack time (`:575`), so `first_baseline.is_none()` at flush already means no glyph-bearing segment landed on this line or any earlier one — which is the question. | §1.5: a strut exists only for a glyphless (or fallback-only) box. Markers are emitted *before* recursion, so an eager capture would let a decorated inline steal its own subtree's baseline; promoting at flush is the earliest point the question is answerable. Revision 3 proposed an extra `current_line_has_glyphs` flag; it is redundant with `first_baseline.is_none()` under the correct write condition and wrong under the loose one. **Fallback-only fonts are out of scope** — elidex has no fallback-provenance signal; recorded in §8. |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`) against `containing_inline_size`. PR-1a also fixes the four bogus "CSS Box Model L3 5.3" cites (§3.1). |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair; selects the inline-axis edge pair from the physical `EdgeSizes` using `parent_style`'s writing mode and direction. Gains `containing_inline_size` on `collect_inline_items` (`:136`) / `collect_inline_items_inner` (`:194`). ⚠ `collect_inline_items` has **four** callers (verified 2026-08-02 via `grep -rn 'collect_inline_items(' crates/` minus the definition): `inline/mod.rs:154` (has the value), `inline/measure.rs:22` and `:51` (`min_content_inline_size`/`max_content_inline_size`, where a containing inline size is definitionally unavailable — see §9), and the test helper `inline/tests/mod.rs:17`. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get `PackItem` forms; they never become `FlowMember`s. |
| `crates/layout/elidex-layout-block/src/inline/pack/inline_box.rs` (NEW) | `advance_inline_box_edges`, the open-box stack, the tentative baseline, and the marker rect producer — an `impl LinePacker` in a sibling module, the idiom `pack/fragment.rs:10` already uses. Keeps `pack/mod.rs` (791 lines) out of the >1000 band. |
| `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs` | `assign_inline_layout_boxes` (`:48`) — writes `LayoutBox.content` from bounds and, per M4, `padding`/`border`/`margin` from the new `EntityBounds` member. |
| `elidex-text` / `measure_text` | First-available-font ascent/descent for M7's strut probe. Metrics come from `db.font_metrics(font_id, size)` (`elidex-shaping/src/measurement.rs:55`), independent of the probed string. |

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
* **PR-1b — geometry (invariants 3, 7).** M3 + M4. Inline-axis advance, shaping break at a
  decorated boundary, real `LayoutBox` edges, content-span rects. **This is the engine-wide
  correction**: it changes painted output for every decorated inline, empty or not. Line
  *existence* is untouched — the phantom-line predicate is unchanged, so no line appears or
  disappears. §6 cells 13–17 land here.
* **PR-1c — existence (invariants 1, 4, 5).** M5 + M6 + M7. The clause-3 predicate per §1.2,
  the strut baseline, the commit consequence. §6 cells 1–12 and 18–22 land here. Closes
  `#11-line-box-decorated-inline-content` with its original DoD — line *and* rectangle — met,
  because PR-1b already made the rectangle real.
* **`#11-inline-box-decoration-splits`** (new slot, **own** deferral): css-inline-3 §2 box
  splitting across line boxes and bidi fragments, "no visual effect where the split occurs",
  fragmentation across the marker pair. Why deferred: it is a distinct invariant axis (box
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
6. All zero ⇒ no marker emitted at all (M1).
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

13. `<p>a<span style="padding:10px">text</span>b</p>` — the common case §4.3 is about: the
    cursor advances by 10px at each boundary, the span's `LayoutBox` carries real edges, and
    its border box is `content + padding` **once**, not twice.
14. **Shaping breaks at the boundary** (§1.4): in `<p>a<span style="padding:1px"></span>b</p>`
    the two same-entity texts must **stop** coalescing into one `InlineFlowRun`. Contrast:
    with an all-zero-inline-axis-edge span they still coalesce (no marker is emitted at all
    per M1, so this is automatic).
15. **A box that does not fit overflows, it does not wrap** (§1.3):
    `<p style="width:100px">aaaaaaaaaaaa<span style="padding:20px"></span>b</p>` — the box
    boundary introduces no soft wrap opportunity.
16. **`text-align` unaffected by the box** (§4.1.2): `<p style="text-align:center">abc
    <span style="padding:1px"></span></p>` — the trailing collapsible space still hangs;
    `current_line_last_hang` is not touched by M3.
17. **A decorated inline whose content wraps** — start marker on line N, end marker on line
    N+1; M4's per-line rebase must yield one rect per line, each with that line's
    `block_start`.

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

**PR-1a**: marker variants reach `pack/items.rs`; every exhaustive match handles both; edges
resolved via `resolve_box_model` against `containing_inline_size`; the physical→logical
selection is implemented and tested for one vertical and one RTL case; the two pre-pack gates
hold current behaviour; all seven "CSS Box Model L3/Level 3 §5.3" cites fixed; **new variants and their
fields carry docstring citations to their §3 rows**; cells 1–12 land as characterization tests;
**zero behaviour change**. ⚠ The marker fields whose first reader is PR-1b/1c must not need
`#[allow(dead_code)]` — PR-1a's payload is exactly `entity` + inline-axis `EdgeSizes` +
`group_key` (+ `line_height`/font identity on `Start`), and PR-1a's own edge resolution reads
the `EdgeSizes`; any field with no PR-1a reader is moved to the PR that introduces its reader.
**PR-1b**: cells 13–17; §7's paint delta covered; no double-counted edges.
**PR-1c**: cells 18–22 plus the flipped 1–12; every §7 consumer checked; the ⚠ caveat at
`inline/pack/mod.rs:110` retires, naming the root-inline-box divergence and pointing at
`#11-inline-root-inline-box`; slot closes.

**Explicitly not covered, recorded rather than silently dropped**: css-inline-3 §5.3's
"or if it contains only glyphs from fallback fonts" strut condition — elidex has no
fallback-provenance signal from `measure_text`, so M7 implements the "no glyphs at all" half
only. Folded into `#11-inline-root-inline-box`, which is the slot that will need font
provenance anyway.

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires** ("the next change that touches
  `layout_inline_context_fragmented`'s body", no growth qualifier). PR-1a touches `:161`/`:190`,
  PR-1c the persist block (the slot memo's seam 3, `mod.rs:411-637`). **Disposition**: not
  bundled — `inline/mod.rs` is **785 lines** (verified 2026-08-02), below CLAUDE.md's >1000
  prereq-split mandate. Because the slot's own trigger has no size qualifier and will therefore
  keep firing on every future touch, **§10's ledger actions include amending that trigger** to
  carry one, rather than leaving a trigger that no PR ever honours.
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
| 1 | R1: 0 CRIT / 17 IMP | rejected | Established the slot is an umbrella (§4.3) and that both shapes are invisible to the packer (§4.2). |
| 2 | R2: 2 CRIT / 18 IMP | rejected | Established `place_item` cannot host the box (its nine writes, §4). CRIT was an author patch inverting the percentage basis. |
| 3 | R3: 8 CRIT / 24 IMP | rejected | Established the plan was on superseded text. CRITs: two author patches, plus the `flush_line`/`on_line` and border-box double-count defects now fixed by M3/M4. |
| 4 | R4 pending | — | Re-anchored on css-inline-3 §2/§2.3/§5.3 + css-text-3 §5.5/§7.3. Three PRs on disjoint invariant sets. |

At landing: register this umbrella's slot in `project_open-defer-slots.md` (the SoT per
MEMORY.md, which currently carries `#11-inline-align-clientrects-nonpersist-path`,
`#11-inline-relayout-box-staleness` and `#11-justify-subflow-line-unified` but not this one) at
**PR-1a**; close `#11-line-box-decorated-inline-content` at **PR-1c**; open
`#11-inline-box-decoration-splits` (own) and `#11-inline-root-inline-box` (pre-existing) with
the Why / trigger / re-eval date from §5.3; amend
`#11-inline-fragmented-fn-decomposition`'s trigger per §9; and register the #497 carves, which
are also absent from the SoT.

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
