# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's selected next task
(user-approved 2026-08-02). Edge-dense (white-space × box-suppression) ⇒ every PR under
this umbrella goes through `/elidex-plan-review` before implementation, and external
review runs `/external-converge` from round 1.

All premises are re-verified against `154bac3f`.

* **Revision 2** — `/elidex-plan-review` round 1 (0 CRIT / 17 IMP / 19 MIN / 6 FP) found the
  first draft's spec basis was the wrong section, that it left 5 of 7 coupled intersections
  to implementation time, and that slice 1 bundled a structural restructure with a semantic
  flip. Revision 2 became the umbrella + a 4-PR program.
* **Revision 3 (current)** — round 2 (2 CRIT / 18 IMP / 20 MIN / 0 FP) is only partly
  applied. The CRIT is fixed (R9, see below). **§11 records the round-2 findings that are
  still open, including a root-level one: R6's "route the box item through `place_item`"
  does not hold up.** This memo is NOT approved for implementation.

⚠ **The round-2 CRIT was self-inflicted.** R9 was added by the author between rounds, as a
"fix" to a round-1 finding, and it inverted a convention the engine had already got right.
Two of the three axes that flagged it needed only the existing `resolve_padding` docstring
to refute it. Consistent with [[feedback_findings-cluster-in-self-added-scope]]: the round-2
CRIT lands in scope that round 1 did not ask for.

---

## §1. The rule, quoted from source

`body CSS2 inline-formatting` (§9.4.2) — the slot memo quoted this with an elision that
dropped a clause:

> Line boxes that contain no text, no preserved white space, no inline elements with
> non-zero margins, padding, or borders, and no other in-flow content (such as images,
> inline blocks or inline tables), **and do not end with a preserved newline** must be
> treated as zero-height line boxes for the purposes of determining the positions of any
> elements inside of them, and must be treated as not existing for any other purpose.

Five clauses, all of which must hold for suppression:

| # | Clause | elidex status on `154bac3f` |
|---|---|---|
| 1 | no text | ✅ `contributes_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:556`) |
| 2 | no preserved white space | ✅ same site, `Pre`/`PreWrap` arm |
| 3 | **no inline element with non-zero margin/padding/border** | ❌ **this umbrella** |
| 4 | no other in-flow content | ✅ atomic arm (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:607`) |
| 5 | does not end with a preserved newline | ✅ `force_break` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:781`) |

elidex implements suppression as "no `LineBox` pushed, cursor not advanced, tentative
rects/runs discarded" (`flush_line`,
`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:209`) — the rule's stronger
second half. Clause 3 turning true must therefore flip the line to a **real** line box.

### §1.1 The section the first draft missed — §10.8

`body CSS2 line-height` (§10.8 *Line height calculations: the line-height and
vertical-align properties*) is the section that actually governs a decoration-only line,
and it says so directly:

> Empty inline elements generate empty inline boxes, but these boxes still have margins,
> padding, borders and a line height, and thus influence these calculations just like
> elements with content.

with step 1 of the line-box height algorithm:

> The height of each inline-level box in the line box is calculated. … for inline boxes,
> this is their line-height.

and §10.8.1 (*Leading and half-leading*) supplying the baseline:

> If the inline box contains no glyphs at all, it is considered to contain a strut (an
> invisible glyph of zero width) with the A and D of the element's first available font.

Three consequences the first draft got wrong, all now load-bearing:

1. **Height source.** A decoration-only line's height is the empty inline box's own
   `line-height` — not "whatever the text run happened to provide". This is what makes the
   Shape-B case (§4.2) well-defined at all.
2. **Baseline.** The line *has* a baseline (the strut's). The first draft's DoD asserted
   `first_baseline` must not move; that is backwards.
3. **Decoration does not affect line height** but *is* rendered: §10.8.1 — "Although
   margins, borders, and padding of non-replaced elements do not enter into the line box
   calculation, they are still rendered around inline boxes." So clause 3 decides
   *existence*, never *height*.

## §2. Coupled invariants

Six invariants intersect; each pairwise row names the mechanism that answers it (§5.1
holds the full resolution table — this section states the coupling, §5 states the answer).

1. **Line-existence** — §9.4.2's five clauses decide whether a `LineBox` is pushed.
2. **Item-stream integrity** — `whitespace.rs` collapses across run boundaries with an
   `items[j]` lookback (`crates/layout/elidex-layout-block/src/inline/whitespace.rs:59`);
   `measure.rs`, `atomic.rs`, `pack/items.rs` iterate the same stream.
3. **Commit-on-content** — per-line tentative `entity_bounds` rects and `current_line_runs`
   commit on a rendered-content flush, discard on suppression
   (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:116`).
4. **Baseline provenance** — `first_baseline` is captured from a box-generating segment
   (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:575`).
5. **Line-box height composition** — §10.8 step 1: every inline-level box on the line
   contributes its own height.
6. **Group keying** — relpos/sticky inlines open sub-flows
   (`crates/layout/elidex-layout-block/src/inline/collect.rs:286`).

| Pair | Coupling | Resolved in |
|---|---|---|
| 1 × 2 | A non-text item between two text runs must not become a collapse barrier, or existing correct markup regresses. | §5.1 R1 |
| 1 × 5 | A line kept only by decoration needs a height source; `place_item` is the only writer of `current_line_height`. | §5.1 R2 |
| 1 × 4 | §10.8.1 gives the glyphless box a strut, so the line *has* a baseline — the gate must not simply stay text-keyed. | §5.1 R3 |
| 1 × 3 | Shape B never calls `place_item`, so `entity_bounds` gets nothing unless the design puts it there. | §5.1 R4 |
| 2 × 6 | The box item must carry the enclosing recursion level's `group_key` or sub-flow ordering desyncs. | §5.1 R5 |
| 3 × 5 | Commit and height both key on "rendered content" today; clause 3 adds a second reason to keep a line — one predicate, not two copies. | §5.1 R6 |
| 2 × (all) | Which inlines emit an item at all sets the blast radius of every row above. | §5.1 R7 |

**Withdrawn from revision 1**: an "invariant 5 = flow-line ordinal indexing" row claimed
render indexes `InlineFlowLine` by position and that empty entries must be preserved. That
is code-contradicted — `InlineFlowLine` carries absolute `block_start`/`block_size` and
render iterates by coordinate (`crates/core/elidex-render/src/builder/inline_flow.rs:111`),
while `flush_line` already drops member-less buckets
(`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:346`) and
`slice_and_rebase_fragment` retains only non-empty ones
(`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs:63`). No ordinal
dependency exists; the row is removed rather than reworded.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS 2 §9.4.2 Inline formatting contexts | zero-height line box rule | clause 3: non-zero margin/padding/border | clause-3 predicate → `any_rendered_content` feed (`inline/pack/mod.rs:698`) — **PR-1b** | ✓ | yes |
| CSS 2 §9.4.2 Inline formatting contexts | zero-height line box rule | clause 5: preserved newline | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS 2 §9.4.2 Inline formatting contexts | margins/borders/padding respected between boxes | inline-axis advance | **PR-2** | ✗ (deliberate, §5.3) | yes |
| CSS 2 §9.4.2 Inline formatting contexts | split inline box | "no visual effect where the split occurs"; bidi split | **PR-3** | ✗ (deliberate) | yes |
| CSS 2 §10.8 Line height calculations | step 1 | inline box contributes its `line-height` | box item's `block_advance` — **PR-1b** | ✓ | yes |
| CSS 2 §10.8 Line height calculations | empty inline elements | "still have … a line height" | same | ✓ | yes |
| CSS 2 §10.8.1 Leading and half-leading | glyphless inline box | strut (A/D of first available font) | `first_baseline` (`inline/pack/mod.rs:575`) — **PR-1b** | ✓ | yes |
| CSS 2 §10.8.1 Leading and half-leading | decoration vs line box height | edges excluded from height, still rendered | height unchanged by edges — invariant | ✓ | yes |
| CSS 2 §10.6.1 Inline, non-replaced elements | content area height | vertical edges outside line-height | no change | ✓ | yes |
| CSS 2 §16.6.1 The white-space processing model | collapsing across boundaries | new item must be collapse-transparent | `collapse_inline_whitespace` (`inline/whitespace.rs:41`) — **PR-1a** | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline | `collect.rs:286` sub-flow keying — **PR-1a** | ✓ | yes |
| CSS Backgrounds 3 §3.2 Line Patterns: the border-style properties | `none` / `hidden` | width ignored ⇒ 0 | already applied at computed-value time (`elidex-style resolve/box_model/mod.rs:260`) | ✓ | yes |

**Breadth**: K=2 specs (CSS 2, CSS Backgrounds 3), M=12 entries (verified 2026-08-02 —
data rows counted directly above).
**Split decision**: breadth alone says single PR, but the **invariant-axis** criterion in
CLAUDE.md governs and says otherwise — §5.3 splits into 4 PRs.

### §3.1 User-input touch audit

Every row is reachable from author CSS/HTML; no compiler-internal-only surface. Adjacent
pre-existing laxity and how this umbrella handles it:

* `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:82` writes
  `EdgeSizes::default()` for an inline element's `padding`/`border`/`margin`. Revision 1
  planned to let a decorated inline acquire that zero-edge `LayoutBox` — turning a
  wrong-but-unreachable value into a wrong-and-reachable one, i.e. the
  [[feedback_narrow-slot-no-deferred-coupling]] #352 shape (a newly-observable value
  reporting conformance the engine has not computed). **Revision 2 withholds it** — see
  §5.1 R4. Exposure does not increase.
* The same withhold removes a second newly-created path: because
  `assign_inline_layout_boxes` skips entities that already carry a `LayoutBox`
  (`crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:62`) and nothing clears
  stale ones, toggling `padding` off via CSSOM would otherwise strand a phantom rect. That
  path would have been **new**, not covered by `#11-inline-relayout-box-staleness`.

## §4. Verified current state

* `any_rendered_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:115`)
  is fed from exactly two sites — `place_item`'s `contributes_content` (`:698`) and
  `force_break` (`:781`) (verified 2026-08-02 via
  `grep -rn any_rendered_content crates/` → 7 hits, lines 115/184/201/210/435/698/781; only
  `:698`/`:781` write a computed value).
* `StyledRun` (`crates/layout/elidex-layout-block/src/inline/styled_run.rs:43`) carries no
  margin/padding/border member.
* `place_item` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:679`) is the
  **only** writer of `on_line`, `current_line_height`, `any_rendered_content`, and
  `current_line_entity_rects`; `finish` flushes only when `on_line` (`:787`).

### §4.1 ⚠ Correction 1 — `group_key` is the wrong entity

The slot memo proposed reading decoration off `StyledRun.group_key`'s computed style, on
the premise that it "already names the inline box". It does not: `group_key` is the
**render-run-group start**
(`crates/layout/elidex-layout-block/src/inline/styled_run.rs:76`,
`crates/layout/elidex-layout-block/src/inline/collect.rs:286`). For
`<p>a<span style="padding:10px"> </span></p>` the span's run carries the *top-level*
run-start. The field naming the originating element is `StyledRun.entity`
(`crates/layout/elidex-layout-block/src/inline/styled_run.rs:45`). That does not rescue the
option — see §4.2.

### §4.2 ⚠ Correction 2 — two shapes; only one has a run to read from

`collect_inline_items_inner` emits **no `InlineItem` for an inline element itself**; it
recurses into the element's children
(`crates/layout/elidex-layout-block/src/inline/collect.rs:291`).

* **Shape A** — `<span style="padding:10px"> </span>`: one `StyledRun`, `entity` = span,
  collapsible ⇒ `contributes_content` false ⇒ line suppressed.
* **Shape B** — `<span style="padding:10px"></span>`: **no `InlineItem` at all**, so
  `layout_inline_context_fragmented` returns `line_count: 0` at the `items.is_empty()` gate
  (`crates/layout/elidex-layout-block/src/inline/mod.rs:161`) before packing. Nothing to
  un-suppress and nothing to read decoration from.

Any design reading decoration off an existing run fixes A and leaves B.

### §4.3 ⚠ Correction 3 — inline box decoration is not laid out at all

`assign_inline_layout_boxes` hard-codes zero `padding`/`border`/`margin`
(`crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:82`) and the packer never
advances `current_inline` for an inline box's edges. §9.4.2's "Horizontal margins, borders,
and padding are respected between these boxes" is unimplemented engine-wide for inline
boxes. **This is why the slot is an umbrella, not a single fix**: clause 3 exists because a
decorated inline is visible, and the visibility is PR-2's subject. Per §4.3 the geometry
gap is **pre-existing**, not this umbrella's own deferral.

## §5. Design

### §5.1 Intersection → mechanism resolution table

The design decisions, each answering a §2 row. This table is the artifact revision 1 was
missing; implementation follows it rather than re-deciding.

| # | Question | Decision | Grounds |
|---|---|---|---|
| **R1** | Collapse-pass arm for the new item (1 × 2) | **Transparent** — same arm shape as `InlineItem::Placeholder` (`crates/layout/elidex-layout-block/src/inline/whitespace.rs:53`), *not* the `Atomic` barrier arm (`:45`). It neither resets `prev_collapsible_space` nor breaks the `items[j]` lookback. | CSS 2 §16.6.1: inline box boundaries do not stop collapsing. A barrier would change `a<span style="padding:10px"> </span>b` — currently-correct markup. |
| **R2** | Height of a decoration-only line (1 × 5) | The box item carries the inline element's own resolved `line_height` and is placed through `place_item` with `block_advance = line_height`, `full_width = trimmed_width = 0`. | CSS 2 §10.8 step 1 ("for inline boxes, this is their line-height") + "Empty inline elements … still have … a line height". |
| **R3** | Baseline of a decoration-only line (1 × 4) | The line **does** set `first_baseline`, measured from the box's strut. Mechanism: the item carries the element's **font identity** (per R5) so the existing `measure_text(font_db, &params, "x")`-style probe yields the strut's A/D; `first_baseline = current_block_offset + half_leading + ascent`, the same formula as the text arm (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:575-590`), with the box item as a second source. `line_height` alone is **insufficient** — the strut needs ascent/descent, which are font data, not the resolved line-height. | CSS 2 §10.8.1 strut. Revision 1 asserted the opposite. |
| **R4** | `entity_bounds` / client rect (1 × 3) | **Withheld in PR-1a/1b**: the box item pushes **no** `current_line_entity_rects` entry, as an explicit branch on the item kind — *not* by reusing the `entity != self.parent_entity` guard (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:702`), which does not suppress a **nested** decorated inline (§6 cell 10). So the decorated inline acquires no `LayoutBox` and no `InlineClientRects` until PR-2 computes real edges. Pinned by §6 cell 19. | §3.1 — a zero-edge border box is fabricated conformance (#352); absence is the status quo and is honest. |
| **R5** | Item identity / placement (2 × 6) | Carries `entity` + resolved `EdgeSizes` (padding/border/margin) + resolved `line_height` + **font identity for the strut** (families / size / weight / style — the same fields `StyledRun::measure_params` (`crates/layout/elidex-layout-block/src/inline/styled_run.rs:104`) needs, per R3) + the **enclosing recursion level's `group_key`**, emitted **immediately before** recursing into the element's children (`crates/layout/elidex-layout-block/src/inline/collect.rs:291`). | Matches `InlineItem::Atomic`'s existing keying convention (`crates/layout/elidex-layout-block/src/inline/styled_run.rs:23`); font fields are forced by R3. |
| **R8** | Zero-advance vs the soft-wrap guard | `place_item`'s first statement flushes when `current_inline + trimmed_width > containing_inline_size && on_line` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:690`). With `trimmed_width = 0` this still fires when `current_inline` already exceeds the line — reachable after an unbreakable overflowing segment. A zero-advance box **always fits**, so the box item must bypass the guard rather than inherit it. | CSS 2 §9.4.2: an inline box that cannot be split overflows the line box; the following zero-width box does not start a new line. §6 cell 20. |
| **R9** | Percentage basis | **`containing_inline_size` — the value the caller already holds.** Percentage padding/margin refer to the **logical width** (= inline size) of the containing block: `css css-box-4 padding-top` / `margin-top` → "percentages refer to logical width of containing block". `resolve_padding`'s docstring states the same contract and warns callers explicitly (`crates/layout/elidex-layout-block/src/helpers.rs:57-62`), and every other caller — `crates/layout/elidex-layout-grid/src/lib.rs:177`, `crates/layout/elidex-layout-block/src/inline/atomic.rs:22` — already passes it. §6 cell 21 is a plain non-regression cell, not a design constraint. | ⚠ **Revision 2 got this backwards; corrected in revision 3.** The retracted version cited CSS 2 §8.3/§8.4's "width" and read it as *physical* width — but CSS 2 predates writing modes, so applying it to the vertical case is the same out-of-scope-section error round 1 flagged for §9.2.2.1. It would have made the decorated inline the only box in the engine resolving percentages on a different basis than every other caller. |
| **R6** | One predicate, not two (3 × 5) | Clause 3 feeds the **same** `any_rendered_content` flag through the **same** `place_item` call, via `contributes_content = true` for a decorated box item. Commit-on-content, height, baseline and line existence therefore stay on one seam. | CLAUDE.md "One issue, one way". |
| **R7** | Which inlines emit an item (2 × all) | **Only inlines with at least one non-zero edge** (any of padding/border/margin, any side, sign-inclusive). Undecorated inlines emit nothing, so the stream shape is unchanged for the overwhelming majority of documents. | Keeps R1's blast radius off `items.is_empty()` (`crates/layout/elidex-layout-block/src/inline/mod.rs:161`) and off every undecorated span. PR-2 revisits if advance needs all boxes (it does not — a zero edge advances zero). |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution. **`resolve_box_model` (`:116`)** — *not* `sanitize_padding` (`:96`), whose own docstring says it resolves percentages against 0 and to prefer `resolve_padding` when the containing width is available, and *not* `sanitize_border` (`:102`)/`sanitize_edge_values` (`:48`), which are non-negative and cannot represent a negative margin. |
| `collect.rs` | Emits `InlineItem`, incl. the new box item. **Requires `containing_inline_size` threaded in** (per R9) — `collect_inline_items` (`crates/layout/elidex-layout-block/src/inline/collect.rs:136`) has no such parameter; the caller (`crates/layout/elidex-layout-block/src/inline/mod.rs:142`) already holds exactly that value. This threading is PR-1a's one signature change. |
| `pack/items.rs` | `PackItem` (`crates/layout/elidex-layout-block/src/inline/pack/items.rs:18`) — the second enum between `InlineItem` and `place_item`; the box item needs a `PackItem` form or it never reaches the packer. |
| `LinePacker` | Line state. Its `EdgeSizes` copy is a **pass-local derived snapshot**; the persisted used value is `LayoutBox.padding/border/margin` (`crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:80`), filled by PR-2. |

### §5.3 Program slicing

Per CLAUDE.md "Edge-dense work = multi-PR program", four PRs. Each gets its own plan-memo
and `/elidex-plan-review`; under this approved umbrella each is a terminal slice.

* **PR-1a — item-stream restructure, behaviour-neutral.** New `InlineItem` + `PackItem`
  variant per R5/R7, all consumers updated (`whitespace.rs` per R1, `measure.rs`,
  `atomic.rs`, `pack/items.rs`, the `any_font` probe at
  `crates/layout/elidex-layout-block/src/inline/mod.rs:190`), `containing_inline_size`
  threaded into `collect_inline_items`, edges resolved via `resolve_box_model`. The packer
  **ignores** the item: `pack_item`'s `match pi`
  (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:528`) gets an explicit no-op
  arm — that arm is the seam PR-1b replaces with the `place_item` call, and it is the only
  place the two PRs meet. Content test unchanged. **Every existing test passes unmodified —
  that is the pin.** New slot: none.
* **PR-1b — clause-3 flip.** Box item goes through `place_item` per R2/R3/R6 with zero
  advance; `entity_bounds` withheld per R4. §6's matrix lands here. Closes
  `#11-line-box-decorated-inline-content`.
* **PR-2 — `#11-inline-box-decoration-geometry`** (new slot, **pre-existing** class per
  §4.3, not this umbrella's own deferral): inline-axis advance for start/end edges, real
  `EdgeSizes` on the inline `LayoutBox`, border-box client rects (retires R4's withhold).
  Re-eval trigger: PR-1b landing. Re-eval date: 2026-11-01.
* **PR-3 — `#11-inline-box-decoration-splits`** (new slot, **own** deferral of this
  umbrella): §9.4.2 "no visual effect where the split occurs", bidi splits, fragmentation.
  Re-eval trigger: PR-2 landing. Re-eval date: 2026-11-01.

Own-deferral count for this umbrella: **1** (PR-3) — within the per-PR ≤3 policy.

**Rejected options** (for the record): widening `StyledRun` with edge fields (box-level data
on a per-segment measurement type; N copies for an N-segment span; never reaches Shape B),
and reading `run.entity`'s `ComputedStyle` at pack time (never reaches Shape B). Note the
latter's rejection is *not* on component-lookup cost — `collect.rs:36` deliberately uses
borrowed component reads in the same per-child loop, so that is a normal idiom here.

## §6. Edge matrix (PR-1b unless noted)

**Decoration axis** — non-zero on *any* side, sign-inclusive:

1. `padding` alone.
2. `border` alone, plus `border-style: none` + `border-width: 5px`. CSS Backgrounds 3 §3.2:
   "No border. Color and width are ignored (i.e., the border has width 0)". **Verified
   non-issue** — applied at computed-value time
   (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`); test cell, not a design
   constraint.
3. `margin` alone, **including negative** — "non-zero" is literal. Requires R4's
   `resolve_box_model` sourcing (the `sanitize_*` helpers clamp to non-negative).
4. **Percentage** `padding: 5%` / `margin: 2%` — requires the `containing_inline_size`
   threading in §5.2; without it the used value is 0 and the bug is silently un-fixed.
5. All zero ⇒ unchanged suppression (non-regression), and per R7 no item is emitted at all.
6. Block-axis edges only (`padding-top`) — clause 3 does not distinguish axes.

**Content axis**, crossed with the above:

7. Shape A (decorated inline containing only collapsible white space).
8. Shape B (completely empty decorated inline).
9. **`a<span style="padding:10px"> </span>b`** — the R1 cell: the space must still collapse
   against its neighbours exactly as today. *This cell was absent from revision 1 and is the
   one that guards against regressing currently-correct markup.* (PR-1a already covers the
   behaviour-neutral half.)
10. Nested: inner decorated / outer not, and the reverse.
11. Two inlines on one line, one decorated ⇒ line kept.
12. Decorated inline plus real text on the same line ⇒ unchanged.
13. `white-space: pre` × decorated empty inline ⇒ already kept by clause 2; height must not
    double-count (R2 supplies the box's own line-height; the text run supplies its own).
14. Decorated inline adjacent to a forced break, incl. `<pre>\n</pre>`.
15. Decorated inline as the only IFC content. **Verified**: the gate is `items.is_empty()`
    (`crates/layout/elidex-layout-block/src/inline/mod.rs:161`); the box item makes `items`
    non-empty. The `has_text`/`any_font` early-out (`:190`) is entered only `if has_text`, so
    a decoration-only IFC skips it — but the *mixed* case (decorated inline + text whose font
    is unusable) does reach it. **Decision**: the box item counts as usable content for that
    probe (same standing as the existing `Atomic` escape at
    `crates/layout/elidex-layout-block/src/inline/mod.rs:200`), so the IFC is not early-returned
    to `line_count: 0`. Its line height comes from `line-height`, which needs no font; only
    R3's strut baseline needs font metrics, and that degrades to "no baseline captured" exactly
    as the text path already does when `measure_text` returns `None`.
16. `display: none` ⇒ no item (`crates/layout/elidex-layout-block/src/inline/collect.rs:218`).
17. `position: absolute` ⇒ out of flow, clause 3 does not apply.
18. `position: relative`/`sticky` ⇒ sub-flow keying (R5; test lands with the relpos suite).
19. **No phantom rect** — R4: the decorated inline has no `LayoutBox` / `InlineClientRects`
    after PR-1b, and toggling decoration off does not strand one. Includes the **nested**
    case (cell 10), where the `entity != parent_entity` guard does not apply.
20. **Overflowing line then a decorated inline** — R8: after an unbreakable segment has
    pushed `current_inline` past `containing_inline_size`, a following zero-advance box item
    must not trigger a soft wrap.
21. **Vertical writing mode × percentage edges** — R9 non-regression: `padding: 5%` on a
    decorated inline inside a `writing-mode: vertical-rl` block resolves against the
    containing block's **inline size** (= its physical height there), matching every other
    box in the engine.

**Non-regression**: `collapsible_whitespace_only_generates_no_line_box`
(`crates/layout/elidex-layout-block/src/inline/tests/text_height/basic.rs:204`) and
`nbsp_only_line_generates_a_box` (`:233`) unchanged.

## §7. Downstream of a box-suppression flip

Per [[feedback_inline-whitespace-edge-matrix-dense]], a line that *starts* existing is the
mirror of #266 and needs the same consumer sweep:

* `line_count` (fragmentation, `column-count`).
* IFC `height` / block cursor advance — non-zero per R2, so no zero-height line collides
  with `slice_and_rebase_fragment`'s exact-boundary assumption
  (`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs:49`).
* `first_baseline` — **does** move, per R3; asserted positively, not negatively.
* `entity_bounds` / `getClientRects` — withheld per R4, asserted by cell 19.
* Static positions of following abspos
  (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:97`).
* `current_line_runs` / `flow_lines` — no ordinal dependency (§2 withdrawn row); member-less
  buckets are dropped by design.
* Fragmentation (`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs`) and
  `ColumnFlowSlice`.

## §8. Definition of done (per PR)

**PR-1a**: new variant reaches `pack/items.rs`; all consumers handle it; edges resolved via
`resolve_box_model` with a real containing size; **zero behaviour change** — the existing
suite passes unmodified, plus cells 5/9 as explicit neutrality pins.
**PR-1b**: clause 3 accounted for in both shapes; every §6 cell tested or explicitly argued
unnecessary; every §7 consumer checked; every §5.1 row realised at the named call site; the
⚠ caveat at `crates/layout/elidex-layout-block/src/inline/pack/mod.rs:110` retires, its
replacement naming the remaining divergence and pointing at
`#11-inline-box-decoration-geometry`; slot closes.

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires.** `layout_inline_context_fragmented`
  is `crates/layout/elidex-layout-block/src/inline/mod.rs:141-648` (508 lines,
  `#[allow(clippy::too_many_lines)]`) and PR-1a touches it at `:161`/`:190`, PR-1b at the
  persist block. **Disposition**: PR-1a's touch is a signature change plus one match arm —
  below the substantive-growth bar — so the decomposition is *not* bundled; it is
  re-affirmed as the lane's standing slot and re-checked at PR-1b, whose persist-block work
  is the larger touch. Recorded here because a fired trigger left silent is the defect.
* **File sizes** (touch-time split awareness, `154bac3f`): `inline/tests/relpos_subflow.rs`
  915, `inline/pack/mod.rs` 791, `inline/mod.rs` 785. None over 1000, but two are in the
  700–800 cut-while-writing band per [[feedback_touch-time-split-means-while-writing]].
  PR-1b's ~19 matrix cells must land in a **new** test module rather than growing
  `tests/text_height/basic.rs` or `relpos_subflow.rs`; cell 18 is the one that would
  otherwise land in the 915-line file.
* **`#11-inline-relayout-box-staleness`** — pre-existing
  (`crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:96`); R4's withhold means
  this umbrella does not widen it.
* **`#11-css2-spec-label-normalisation`** — this memo uses `CSS 2 §` throughout (the label
  the slot converges on; webref titles the spec "Cascading Style Sheets (CSS) Level 2").
  Adjacent `CSS 2.1 §` lines in touched files are **left alone** — the slot requires a
  single cross-crate commit, so in-passing normalisation is out.
* **`#11-inline-align-clientrects-nonpersist-path`** (`inline/pack/mod.rs:399`) and
  **`#11-justify-subflow-line-unified`** (`:57`, `:247`) sit in the seams R4/R5 touch;
  neither is modified — R4 withholds rather than routing through the clientRects path, and
  R5 reuses the existing keying without changing sub-flow line unification.
* Vertical writing mode beyond keeping existing `is_vertical` paths correct.

## §10. Slot ledger actions at landing

Register in `project_open-defer-slots.md` (MEMORY.md names it the slot SoT; it currently
carries none of the Layout lane's inline slots): close
`#11-line-box-decorated-inline-content` at PR-1b, open
`#11-inline-box-decoration-geometry` (pre-existing class) and
`#11-inline-box-decoration-splits` (own), each with the Why / trigger / re-eval date from
§5.3.

## §11. Round-2 findings still open (revision 3 must resolve before implementation)

Round 2 = 2 CRIT / 18 IMP / 20 MIN. The CRIT (R9's inverted percentage basis, both Axis 4
and Axis 5) is **fixed above**. The rest are open. Grouped by root:

### §11.1 ROOT — R6's "one seam through `place_item`" does not hold

`place_item` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:679`) is not a
neutral placement primitive; it owns four further concerns that a zero-width, text-less box
item corrupts. Each was found independently:

* **`current_line_last_hang`** is written unconditionally at `:701`; with
  `full_width == trimmed_width == 0` the box item **resets the trailing-space hang to 0**,
  and `flush_line` aligns on `current_inline - current_line_last_hang` (`:235`). Failure:
  `<p style="text-align:center">abc <span style="padding:1px"></span></p>` centres on
  `w("abc ")` instead of `w("abc")` (CSS Text 3 §4.1.2). **No §6 cell covers text-align at all.**
* **`last_placed_entity`** is written at `:772` and drives same-entity run coalescing
  (`:744`). Failure: `<p>a<span style="padding:1px"></span>b</p>` — both texts have
  `entity == p` and coalesce today; the interposed box item splits them into two persisted
  runs, losing cross-boundary shaping (CSS Text 3 §5.6). **§6 cell 12 asserts this case is
  "unchanged"; it is not.**
* **`member: FlowMember<'_>`** is a required parameter (`inline/pack/items.rs:48`) and all
  three variants misbehave: `Atomic` persists an `InlineFlowRun::AtomicBox` whose render path
  `walk()`s a `LayoutBox` that R4 deliberately withholds; `Text(&str)` has no string; there is
  no fourth variant. R6 never says which is passed.
* **R8's soft-wrap bypass** is a fifth in-body carve-out.

Four-to-five conditional carve-outs inside one function is co-location, not a shared seam.
**Revision 3 must re-decide the placement mechanism** — a dedicated minimal path that touches
only line-existence, height and baseline, while keeping R6's genuine point (one *predicate*,
`any_rendered_content`, not two).

### §11.2 ROOT — the strut model is missing, so R2/R3 are both under-derived

§1.1 derived the line height from §10.8 step 1 alone and dropped **step 3**: "The line box
height is the distance between the uppermost box top and the lowermost box bottom. (This
includes the strut …)", where §10.8's `line-height` prose defines the block container's own
strut — "each line box starts with a zero-width inline box with the element's font and line
height properties". So:

* Height is `max(container strut, box line-height)`, not the box's line-height. Failure:
  `<p style="line-height:40px"><span style="padding:1px;line-height:5px"></span></p>` →
  5px under R2, 40px per spec. `grep -rni strut crates/` finds **no strut in inline layout**
  — this is a pre-existing gap that R2 newly depends on, and §4.3-style disclosure is absent.
* R3 conflates "the box's ascent" with "the line box's baseline" (§10.8 steps 2–3 align all
  inline-level boxes incl. the strut). It also drops the existing `&& !is_vertical` guard
  (`:575`), and R2's `block_advance = line_height` ignores the packer's own vertical
  convention `if is_vertical { font_size } else { line_height }` (`:539-543`).
* R3's precondition is unmodelled: §10.8.1 gives a strut only to a box with **no glyphs**,
  but R7 emits for every decorated inline and R5 emits *before* recursion, so a decorated
  inline that *does* contain text would take a strut baseline it must not have (first-wins
  capture at `:575`).

### §11.3 R4's withhold is necessary but not sufficient

The withhold correctly suppresses the *box item's own* rect. But the clause-3 flip also moves
whole lines from the discard branch (`:423-431`) to the commit branch (`:210`), which commits
**every** entity's tentative rect on that line. Failure: a `pre-wrap` line where a collapsible
`<i> </i>` sits beside the decorated span — the undecorated `<i>` gains a `LayoutBox` and a
`getClientRects()` rectangle it did not have, and it has no §6 cell. Additionally, open slot
`#11-layoutbox-absence-unreachable` (#488) records that "box-absent is unreachable for a
once-laid entity" — so §6 cell 19 is false on the **relayout** path as written.

### §11.4 PR-1a is not behaviour-neutral as specified

R7's emit predicate fires at **collect** time, upstream of the pack-time seam §5.3 claims is
"the only place the two PRs meet". Two pre-pack gates move in PR-1a:

* `items.is_empty()` (`crates/layout/elidex-layout-block/src/inline/mod.rs:161`) — a Shape-B
  IFC stops taking the early return, whose `clear_inline_flows` is unconditional, and starts
  taking the full path where the same clear is `!env.is_probe`-gated (`:637`).
* the `any_font` probe (`:190-200`) — §6 cell 15 makes a semantic decision there, but §6 is
  headed "PR-1b unless noted" and cell 15 is not noted.

Also: `grep -rn padding crates/layout/elidex-layout-block/src/inline/tests/` → **0 hits**, so
"every existing test passes unmodified" is a vacuous pin for exactly the inputs PR-1a changes.

### §11.5 Open MINs (carried, not yet applied)

Axis 1: no layer row owns font-metric/strut measurement; `containing_inline_size` named
inconsistently across §5.2/§5.3/§6/§8. Axis 2: §4 bullet 3's write-set claim is
self-contradictory and omits five fields `place_item` writes; R5's `group_key` and
`EdgeSizes` have no reader through PR-1a/1b (`dead_code` under `-D warnings`); §4.2's Shape-A
mechanism is misstated (the run collapses to `""` and `build_pack_items` skips it at
`inline/pack/items.rs:69`, so `place_item` is never reached); `inline/tests/mod.rs:21-22` is
an omitted exhaustive match site. Axis 3: PR-3's `Why deferred` missing; R8's mechanism
unnamed; §2's pair table does not enumerate R8/R9. Axis 4: R8's grounds should be css-text-3
§5 (soft wrap opportunity), not §9.4.2; §3 lacks rows for R8/R9's sections; no docstring
citation requirement in §8; §8.3's "vertical margins have no effect on non-replaced inline
elements" vs R7's any-side rule; cell 13's clause-2 attribution; `helpers.rs:113` cites a
non-existent "CSS Box Model L3 5.3". Axis 5: §10's claim about the slot ledger's contents is
false (it carries three inline slots); §9's decomposition disposition uses the wrong bar;
production-file growth (pack/mod.rs 791, inline/mod.rs 785) has no seam decision; the slot
memo's own DoD ("keeps its line **and rectangle**") needs an explicit amendment if PR-1b
closes the slot with the rectangle withheld.
