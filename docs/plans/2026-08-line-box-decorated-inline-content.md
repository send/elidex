# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's selected next task
(user-approved 2026-08-02). Edge-dense (white-space × box-suppression) ⇒ every PR under this
umbrella goes through `/elidex-plan-review` before implementation, and external review runs
`/external-converge` from round 1.

All premises re-verified against `154bac3f`.

* **Revision 1** — single-slice plan. Round 1 (0 CRIT / 17 IMP / 19 MIN / 6 FP): wrong spec
  section, 5 of 7 coupled intersections left to implementation, restructure bundled with a
  semantic flip.
* **Revision 2** — umbrella + 4 PRs. Round 2 (2 CRIT / 18 IMP / 20 MIN): the CRIT was **a row
  the author added between rounds** (R9) inverting a percentage convention the engine already
  had right; and R6's "route the box item through `place_item`" needed four carve-outs.
* **Revision 3** — re-decides the placement mechanism (§5.1 M1), stops claiming
  §10.8 conformance the engine cannot deliver (M2), fixes the baseline precondition (M3),
  **merges the former PR-2 geometry slice into PR-1b** so no fabricated-or-absent-rect
  half-state ships (M4), and makes PR-1a's neutrality provable instead of vacuous (M5).
  §11 tracks round-2 disposition.
* **⚠ Round 3 (8 CRIT / 24 IMP / 25 MIN) says revision 4 is required, and that the plan is
  anchored on a superseded spec section.** §12 records it. **NOT approved for implementation.**

⚠ Authoring note carried forward per [[feedback_findings-cluster-in-self-added-scope]]:
round 2's only CRIT landed in scope round 1 did not ask for. Revision 3's new material
(M1–M5) is therefore flagged as the highest-risk part of this memo, not the settled part.

---

## §1. The rules, quoted from source

### §1.1 The existence rule — CSS 2 §9.4.2

`body CSS2 inline-formatting`:

> Line boxes that contain no text, no preserved white space, no inline elements with
> non-zero margins, padding, or borders, and no other in-flow content (such as images,
> inline blocks or inline tables), **and do not end with a preserved newline** must be
> treated as zero-height line boxes for the purposes of determining the positions of any
> elements inside of them, and must be treated as not existing for any other purpose.

| # | Clause | elidex status on `154bac3f` |
|---|---|---|
| 1 | no text | ✅ `contributes_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:556`) |
| 2 | no preserved white space | ✅ same site, `Pre`/`PreWrap` arm |
| 3 | **no inline element with non-zero margin/padding/border** | ❌ **this umbrella** |
| 4 | no other in-flow content | ✅ atomic arm (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:607`) |
| 5 | does not end with a preserved newline | ✅ `force_break` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:781`) |

elidex implements suppression as the rule's stronger second half — no `LineBox` pushed, no
cursor advance, tentative rects/runs discarded (`flush_line`,
`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:209`, discard arm at `:423-430`).
Clause 3 turning true must flip the line to a **real** line box.

§9.4.2 also supplies the geometry PR-1b now includes:

> Horizontal margins, borders, and padding are respected between these boxes.

### §1.2 The height rule — CSS 2 §10.8

`body CSS2 line-height` (§10.8 *Line height calculations: the line-height and vertical-align
properties*) — the section revision 1 missed entirely:

> Empty inline elements generate empty inline boxes, but these boxes still have margins,
> padding, borders and a line height, and thus influence these calculations just like
> elements with content.

Step 1: "The height of each inline-level box in the line box is calculated. … for inline
boxes, this is their line-height." Step 3: "The line box height is the distance between the
uppermost box top and the lowermost box bottom. **(This includes the strut, as explained
under line-height below.)**" — where the strut is "a zero-width inline box with the
element's font and line height properties" that "each line box starts with".

§10.8.1 (*Leading and half-leading*) supplies the glyphless case:

> If the inline box contains no glyphs at all, it is considered to contain a strut (an
> invisible glyph of zero width) with the A and D of the element's first available font.

and bounds what decoration may affect:

> Although margins, borders, and padding of non-replaced elements do not enter into the line
> box calculation, they are still rendered around inline boxes.

**So clause 3 decides *existence*, never *height*.** Height comes from line-height, per
§10.8 — and §10.8's step 3 needs a strut elidex does not have. See M2.

## §2. Coupled invariants

| # | Invariant | Site |
|---|---|---|
| 1 | **Line-existence** — §9.4.2's five clauses | `any_rendered_content` (`inline/pack/mod.rs:115`) |
| 2 | **Item-stream integrity** — cross-run collapse lookback; positional iteration | `inline/whitespace.rs:59`; `measure.rs`, `atomic.rs`, `pack/items.rs` |
| 3 | **Commit-on-content** — per-line rects/runs commit or discard | `inline/pack/mod.rs:116`, `:210` vs `:423` |
| 4 | **Baseline provenance** — §10.8.1 strut vs glyphs | `inline/pack/mod.rs:575` |
| 5 | **Line-box height composition** — §10.8 step 1/3 | `inline/pack/mod.rs:696` |
| 6 | **Group keying** — relpos/sticky sub-flows | `inline/collect.rs:286` |
| 7 | **Cursor/advance integrity** — soft wrap, hang, run coalescing | `inline/pack/mod.rs:690`, `:701`, `:772` |

| Pair | Coupling | Resolved in |
|---|---|---|
| 1 × 2 | A non-text item between two text runs must not become a collapse barrier, or currently-correct markup regresses. | M6 |
| 1 × 5 | A line kept only by decoration needs a height source. | M2 |
| 1 × 4 | §10.8.1 gives a strut only to a **glyphless** box, so the baseline source depends on what else lands on the line. | M3 |
| 1 × 3 | The flip moves whole lines from discard to commit, affecting **every** entity on the line, not just the decorated one. | M4 |
| 1 × 7 | A zero-advance participant must not perturb soft wrap, trailing-space hang, or run coalescing. | M1 |
| 2 × 6 | Box markers must carry the enclosing recursion level's `group_key`. | M7 |
| 2 × (all) | Which inlines emit markers sets the blast radius of every row. | M7 |
| 3 × 5 | Existence, commit and height must key on **one** predicate, not copies. | M1 |

**Withdrawn in revision 2** (kept for the record): a claimed "flow-line ordinal indexing"
invariant. Code-contradicted — `InlineFlowLine` carries absolute `block_start`/`block_size`
and render iterates by coordinate
(`crates/core/elidex-render/src/builder/inline_flow.rs:111`); `flush_line` already drops
member-less buckets (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:346`) and
`slice_and_rebase_fragment` retains only non-empty ones
(`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs:63`).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS 2 §9.4.2 Inline formatting contexts | zero-height line box rule | clause 3: non-zero margin/padding/border | clause-3 predicate → `any_rendered_content` (`inline/pack/mod.rs:698`) — **PR-1b** | ✓ | yes |
| CSS 2 §9.4.2 Inline formatting contexts | zero-height line box rule | clause 5: preserved newline | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS 2 §9.4.2 Inline formatting contexts | margins/borders/padding respected between boxes | inline-axis advance at box start/end | `note_inline_box_edges` (NEW) — **PR-1b** | ✓ | yes |
| CSS 2 §9.4.2 Inline formatting contexts | inline box overflows when unsplittable | zero-advance box after overflow | M1 — never reaches the wrap guard | ✓ | yes |
| CSS 2 §9.4.2 Inline formatting contexts | split inline box | "no visual effect where the split occurs"; bidi split | **PR-2** | ✗ (deliberate, §5.3) | yes |
| CSS 2 §10.8 Line height calculations | step 1 | inline box contributes its `line-height` | box marker's `block_advance` — **PR-1b** | ✓ | yes |
| CSS 2 §10.8 Line height calculations | step 3 | line box height includes the **strut** | **NOT implemented — M2, new slot** | ✗ (pre-existing, disclosed) | yes |
| CSS 2 §10.8 Line height calculations | empty inline elements | "still have … a line height" | same as step 1 | ✓ | yes |
| CSS 2 §10.8.1 Leading and half-leading | glyphless inline box | strut (A/D of first available font) | M3 tentative baseline — **PR-1b** | ✓ | yes |
| CSS 2 §10.8.1 Leading and half-leading | decoration vs line box height | edges excluded from height, still rendered | height never keys on edges — invariant | ✓ | yes |
| CSS 2 §10.6.1 Inline, non-replaced elements | content area | vertical edges outside line-height | no change | ✓ | yes |
| CSS 2 §8.3 Margin properties | non-replaced inline elements | "vertical margins will not have any effect" | M7 tension, disclosed | ✓ | yes |
| CSS 2 §16.6.1 The white-space processing model | collapsing across boundaries | box markers must be collapse-transparent | `collapse_inline_whitespace` (`inline/whitespace.rs:41`) — **PR-1a** | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline | `collect.rs:286` sub-flow keying — **PR-1a** | ✓ | yes |
| CSS Text 3 §5 Line Breaking and Word Boundaries | soft wrap opportunity | zero-advance participant never wraps | M1 — the guard is not on the path | ✓ | yes |
| CSS Box Model 3 §3.1 / §4.1 (physical margin/padding) | percentage resolution | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract dimensions | "inline size / logical width" | basis identity in vertical modes | same | ✓ | yes |
| CSS Backgrounds 3 §3.2 Line Patterns: the border-style properties | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time (`elidex-style resolve/box_model/mod.rs:260`) | ✓ | yes |

**Breadth**: K=5 specs (CSS 2, CSS Text 3, CSS Box Model 3, CSS Writing Modes 4, CSS
Backgrounds 3), M=18 entries (verified 2026-08-02 — data rows counted directly above).
**Split decision**: K=5 ⇒ SPLIT-RECOMMENDED, and the plan **is** split (§5.3): two shipping
PRs plus one booked follow-on, on invariant-axis grounds. The breadth verdict and the
invariant-axis verdict agree.

### §3.1 User-input touch audit

Every row is reachable from author CSS/HTML; no compiler-internal-only surface. Adjacent
pre-existing laxity, and how revision 3 handles it:

* `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:82` hard-codes
  `EdgeSizes::default()` on every inline `LayoutBox`. Revision 1 would have let a decorated
  inline acquire that zero-edge box (fabricated conformance — the
  [[feedback_narrow-slot-no-deferred-coupling]] #352 shape); revision 2 withheld the box
  instead (absence, also untrue). **Revision 3 fills the edges for real (M4)**, so neither
  wrong value ships and `boxes.rs:82` stops being a lie for this class.
* **Strut absence** (`grep -rni strut crates/` → no strut in inline layout, verified
  2026-08-02): pre-existing, newly *depended on*. Disclosed in M2 with its own slot rather
  than silently inherited.
* `crates/layout/elidex-layout-block/src/helpers.rs:113` cites "CSS Box Model L3 5.3", which
  does not exist (css-box-3 §5 is *Borders* and has no subsections). PR-1a corrects it to
  §3.1/§4.1 as an in-passing fix at a site it already edits.

## §4. Verified current state

* `any_rendered_content` (`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:115`) is
  **written at four sites**: `place_item` `|=` (`:698`), `force_break` `= true` (`:781`),
  `flush_line` reset `= false` (`:435`), and the constructor (`:184`). Only `:698`/`:781`
  write a computed value. (Revision 2 stated "two"; that was the read-set, not the write-set.)
* `place_item` (`:679`) writes **nine** pieces of state: soft-wrap flush (`:690`),
  `current_inline` (`:695`), `current_line_height` (`:696`), `on_line` (`:697`),
  `any_rendered_content` (`:698`), `current_line_last_hang` (`:701`),
  `current_line_entity_rects` (`:702`), the flow-member bucket (`:736-769`, requiring a
  `FlowMember` argument — `crates/layout/elidex-layout-block/src/inline/pack/items.rs:49`),
  and `last_placed_entity` (`:772`). This enumeration is what M1 rests on.
* `StyledRun` (`crates/layout/elidex-layout-block/src/inline/styled_run.rs:43`) carries no
  margin/padding/border member.

### §4.1 ⚠ Correction 1 — `group_key` is the wrong entity

The slot memo proposed reading decoration off `StyledRun.group_key`. `group_key` is the
**render-run-group start** (`crates/layout/elidex-layout-block/src/inline/styled_run.rs:76`,
`crates/layout/elidex-layout-block/src/inline/collect.rs:286`), not the decorated inline. The
field naming the originating element is `StyledRun.entity` (`styled_run.rs:45`) — which does
not rescue the option either, per §4.2.

### §4.2 ⚠ Correction 2 — two shapes, and **neither** reaches `place_item` today

`collect_inline_items_inner` emits **no `InlineItem` for an inline element itself**; it
recurses into the children (`crates/layout/elidex-layout-block/src/inline/collect.rs:291`).

* **Shape A** — `<span style="padding:10px"> </span>`: a `StyledRun` is created, but its text
  collapses to `""` in `collapse_inline_whitespace`, and `build_pack_items` then **skips it
  entirely** (`crates/layout/elidex-layout-block/src/inline/pack/items.rs:69`,
  `if run.text.is_empty() { continue; }`). So `place_item` is never called and
  `contributes_content` is never evaluated. (Revision 2 said the run reached
  `contributes_content` and returned false — wrong mechanism, same outcome.)
* **Shape B** — `<span style="padding:10px"></span>`: **no `InlineItem` at all**;
  `layout_inline_context_fragmented` returns `line_count: 0` at the `items.is_empty()` gate
  (`crates/layout/elidex-layout-block/src/inline/mod.rs:161`) before packing.

Both shapes are invisible to the packer, which is why reading decoration off a run cannot fix
either one, and why the fix must put the **box itself** into the stream.

### §4.3 ⚠ Correction 3 — inline box decoration is not laid out at all

`assign_inline_layout_boxes` hard-codes zero edges (`inline/pack/boxes.rs:82`) and the packer
never advances for an inline box's edges. §9.4.2's "Horizontal margins, borders, and padding
are respected between these boxes" is unimplemented engine-wide. This is why the slot is an
umbrella — and, per M4, why PR-1b now closes the geometry gap together with the flip.

## §5. Design

### §5.1 Mechanism table (revision 3)

| # | Question | Decision | Grounds |
|---|---|---|---|
| **M1** | How does a box marker participate in line state? | **Not through `place_item`** — through two narrow entry points on `LinePacker`, in a new `pack/inline_box.rs` (§9): (a) `note_inline_box(block_advance)` writes exactly `current_line_height = max(block_advance)` (`:696`), `on_line = true` (`:697`), `any_rendered_content = true` (`:698`); (b) `advance_inline_box_edges(inline_advance)` runs the soft-wrap check (`:690`) and then `current_inline += inline_advance` (`:695`). Neither touches `current_line_last_hang` (`:701`), `current_line_entity_rects` (`:702`), the `FlowMember` bucket (`:736-769`), or `last_placed_entity` (`:772`). | Round 2 §11.1: routing through `place_item` needed four to five conditional carve-outs — co-location, not a seam. Placing a *segment* and noting a *box boundary* are different operations; `force_break` (`:781`) is already precedent for a second producer of `any_rendered_content`, so R6's real point — **one predicate, several producers** — survives. ⚠ **Revision 3 initially claimed the box is zero-advance and never reaches the wrap guard; that was wrong once M4 gave it real edges.** A `<span style="padding:20px">` occupies 40px and must be able to push content to the next line (§9.4.2 "Horizontal margins, borders, and padding are respected between these boxes"), so (b) *does* wrap. Only a box whose inline-axis edges are all zero (cells 6/22) is genuinely zero-advance. CSS Text 3 §5 governs (b); revision 2's R8 is superseded, not moot. |
| **M2** | Where does the height come from, honestly? | The box marker's `block_advance` = the element's own resolved `line_height`, **and the memo does not claim §10.8 conformance**. §10.8 step 3 requires the block container's strut, which elidex does not model (verified: no strut in inline layout). The text path already diverges the same way — `<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today, spec says 40px. New slot **`#11-inline-line-box-strut`** (pre-existing class) records it. `block_advance` follows the packer's existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`), not `line_height` unconditionally. | Round 2 §11.2. Disclosing a pre-existing divergence the new code *depends on* is the §4.3 pattern; silently inheriting it while quoting §10.8 is not. |
| **M3** | Which line gets a strut baseline? | **Only a line with no glyphs on it.** Mechanism: `note_inline_box` records a *tentative* `current_line_box_baseline: Option<f32>` (NEW) from the box's own font metrics via the existing `measure_text` probe, keeping the `!is_vertical` guard (`:575`); `flush_line` promotes it into `first_baseline` **only if** `first_baseline.is_none()` and a new per-line `current_line_has_glyphs: bool` (NEW) is false. The flag is set by the text arm and reset in `flush_line`'s per-line reset block alongside `current_inline`/`current_line_height`/`any_rendered_content` (`:432-435`). The flag is required because `first_baseline` is IFC-global first-wins (`:575`) and therefore cannot answer "did text set one **on this line**". | §10.8.1 gives a strut only "if the inline box contains no glyphs at all". Round 2 found revision 2's version broken twice over: markers are emitted *before* recursion (M7), and capture is first-wins (`:575`), so a decorated inline **containing text** would have stolen its own subtree's baseline. Deciding at flush is the earliest point the glyphless question is answerable. |
| **M4** | Rect and edges — fabricate, withhold, or fill? | **Fill.** PR-1b advances the cursor by the box's inline-axis edges at start and end (§9.4.2 "respected between these boxes"), writes the real `padding`/`border`/`margin` onto the inline `LayoutBox` (replacing `EdgeSizes::default()` at `inline/pack/boxes.rs:82`), and produces the span's rect from the **marker pair** — a new per-line tentative entry spanning start-marker cursor position → end-marker cursor position, inflated by the edges, pushed by `advance_inline_box_edges` into the same `current_line_entity_rects` bucket (`:702`) the commit/discard seam already drains. It is a **new producer**, not the existing `place_item` push: neither shape reaches `place_item` (§4.2), so there is no "normal" path to inherit. The former PR-2 is **merged into PR-1b**. | Revision 1 fabricated a zero-edge rect (#352); revision 2 withheld the box entirely — also untrue, and round 2 showed the withhold was insufficient anyway, because the flip moves whole lines from the discard arm (`:423`) to the commit arm (`:210`), committing **every** entity's rect on the line. Filling the edges dissolves the dilemma, the §11.3 finding, and the slot-DoD amendment finding together. It is the option round 2 (Axis 3) noted was "rejected nowhere in the memo" — because it is the right one, and it only becomes available once advance lands. |
| **M5** | How is PR-1a proven neutral? | PR-1a **adds characterization tests first** — Shape A, Shape B, mixed, and `a<span style="padding:10px"> </span>b` — asserting today's behaviour (no line, `line_count: 0`, collapsing as-is). PR-1b then flips those assertions. The two pre-pack gates are explicitly held at their current behaviour in PR-1a: `items.is_empty()` (`inline/mod.rs:161`) becomes "no item that can generate content" (markers excluded in 1a, included in 1b), and the `any_font` arm (`:190-200`) returns `false` in 1a. | Round 2 §11.4: `grep -rn padding crates/layout/elidex-layout-block/src/inline/tests/` → **0 hits** (verified 2026-08-02), so "every existing test passes unmodified" was a vacuous pin for exactly the inputs PR-1a changes. Characterization-then-flip is the standard fix. The meeting points between the two PRs are therefore **three**, not one: the `match pi` arm (`:528`), `:161`, and `:190-200`. |
| **M6** | Collapse-pass arm | **Transparent**, same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified constructible: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so an interleaved marker is skipped by construction. | CSS 2 §16.6.1 — inline box boundaries do not stop collapsing. A barrier would change `a<span style="padding:10px"> </span>b`, currently-correct markup. |
| **M7** | Which inlines emit markers, carrying what, where? | **Start/end marker pair — two distinct `InlineItem` variants** (`InlineBoxStart` / `InlineBoxEnd`, mirrored in `PackItem`), not one variant with a `start: bool`, so PR-2's split rule can match on them separately and every exhaustive match states both cases explicitly. Emitted for every inline with at least one non-zero edge (any side, sign-inclusive). Start emitted immediately **before** recursing into children (`collect.rs:291`), end immediately after. Each carries `entity`, resolved `EdgeSizes`, resolved `line_height`, font identity for M3's strut probe (the six fields `StyledRun::measure_params` needs — `crates/layout/elidex-layout-block/src/inline/styled_run.rs:104`), and the **enclosing recursion level's `group_key`**. Edges resolved with `resolve_box_model` (`helpers.rs:116`) against **`containing_inline_size`** — the value the caller (`inline/mod.rs:142`) already holds. ⚠ Tension, disclosed: CSS 2 §8.3 says "vertical margins will not have any effect on non-replaced inline elements", yet clause 3's literal text counts a non-zero `margin-top`. This memo follows the literal text (marker emitted, line kept) and pins it as §6 cell 22. | A pair is what M4's advance needs (start edges before the content, end edges after) and what PR-2's split rule needs; a single marker would have to be re-split later. `resolve_box_model` is mandatory because `sanitize_padding` (`helpers.rs:96`) resolves percentages against 0 and `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative, so neither can express a percentage or a negative margin. **Percentage basis = logical width = inline size** (`css css-box-4 padding-top`/`margin-top` → "percentages refer to logical width of containing block"; css-writing-modes-4 §6.1 equates inline size and logical width), which is exactly what `resolve_padding`'s docstring already mandates (`helpers.rs:57-62`) and what `elidex-layout-grid/src/lib.rs:177` and `inline/atomic.rs:22` already pass. |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`), against `containing_inline_size`. PR-1a also fixes its non-existent "CSS Box Model L3 5.3" docstring citation to css-box-3 §3.1/§4.1. |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair. Gains a `containing_inline_size` parameter on `collect_inline_items` (`:136`) and `collect_inline_items_inner` (`:194`) — PR-1a's one signature change. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get a `PackItem` form; they never become a `FlowMember` (M1). |
| `LinePacker` | Line state. `note_inline_box` (M1) and the edge advance (M4) are its two new entry points. Its `EdgeSizes` copy is a pass-local snapshot; the persisted used value is `LayoutBox.padding/border/margin` (`inline/pack/boxes.rs:80`), which M4 now fills. |
| `elidex-text` / `measure_text` | Font metrics for M3's strut probe. Ascent/descent are independent of the probed string, so an `"x"` probe from the box's own font identity yields the strut's A/D. |

### §5.3 Program slicing

Two shipping PRs plus one booked follow-on. Each gets its own plan-memo and
`/elidex-plan-review`; under this approved umbrella each is a terminal slice.

* **PR-1a — item-stream restructure, behaviour-neutral (provably, per M5).** Marker pair per
  M7, `PackItem` form with a no-op `match pi` arm (`:528`), all consumers updated (M6's
  `whitespace.rs` arm; `measure.rs` and `collect.rs:181` are `if let` and need none;
  `atomic.rs:39`; `inline/mod.rs:161` and `:190-200` held at current behaviour;
  `inline/tests/mod.rs:21-22`, an exhaustive match that will fail to compile otherwise),
  `containing_inline_size` threaded, edges resolved. Characterization tests land here. No new
  slot.
* **PR-1b — clause 3 + geometry.** `note_inline_box` (M1), tentative strut baseline (M3),
  inline-axis advance and real `LayoutBox` edges (M4). §6's matrix lands here. Closes
  `#11-line-box-decorated-inline-content` **with its original DoD met** — line *and*
  rectangle — so no slot-memo amendment is needed.
* **PR-2 — `#11-inline-box-decoration-splits`** (new slot, **own** deferral): §9.4.2 "no
  visual effect where the split occurs", bidi splits, fragmentation across the marker pair.
  Why deferred: it is a distinct invariant axis (box continuity across line and bidi breaks)
  that only becomes reachable once a box has geometry at all, and bundling it would put three
  axes in one PR. Re-eval trigger: PR-1b landing. Re-eval date: 2026-11-01.
* **`#11-inline-line-box-strut`** (new slot, **pre-existing** class per M2): line box height
  must include the block container's strut (§10.8 step 3). Why deferred: it changes the height
  of *every* line box in the engine, so it cannot ride inside a slot about decorated inlines.
  Re-eval trigger: any line-height correctness work, or a compat-survey hit. Re-eval date:
  2026-11-01.

Own-deferral count for this umbrella: **1** (PR-2). `#11-inline-line-box-strut` is
pre-existing. Both within the per-PR ≤3 policy.

**Rejected options**: widening `StyledRun` with edge fields (box-level data on a per-segment
measurement type; N copies for an N-segment span; reaches neither shape per §4.2); and
reading `run.entity`'s `ComputedStyle` at pack time (reaches neither shape either, for the
same reason). Note the rejection is **not** on component-lookup cost — `collect.rs:36` uses
borrowed component reads in the same per-child loop, so that is a normal idiom here.

## §6. Edge matrix (PR-1b unless noted)

**Decoration axis** — non-zero on any side, sign-inclusive:

1. `padding` alone.
2. `border` alone, plus `border-style: none` + `border-width: 5px`. CSS Backgrounds 3 §3.2:
   "No border. Color and width are ignored (i.e., the border has width 0)". **Verified
   non-issue** — applied at computed-value time (`elidex-style resolve/box_model/mod.rs:260`).
3. `margin` alone, **including negative** — requires M7's `resolve_box_model` sourcing.
4. **Percentage** `padding: 5%` / `margin: 2%` — resolved against `containing_inline_size`.
5. All zero ⇒ unchanged suppression, and per M7 no marker is emitted.
6. Block-axis edges only (`padding-top`) — clause 3 does not distinguish axes.

**Content axis**:

7. Shape A (decorated inline containing only collapsible white space).
8. Shape B (completely empty decorated inline).
9. `a<span style="padding:10px"> </span>b` — the M6 cell: the space must still collapse
   against its neighbours exactly as today. Lands in **PR-1a** as a characterization test.
10. Nested: inner decorated / outer not, and the reverse.
11. Two inlines on one line, one decorated ⇒ line kept.
12. `<p>a<span style="padding:1px"></span>b</p>` — **run coalescing must not change**: both
    texts have `entity == p` and coalesce into one `InlineFlowRun::Text` today (CSS Text 3
    §5.6 shaping across intra-word breaks, `inline/pack/mod.rs:744`). M1 keeps
    `last_placed_entity` untouched, so they still coalesce. PR-1a characterization + PR-1b
    non-regression.
13. **`text-align`** — `<p style="text-align:center">abc <span style="padding:1px"></span></p>`:
    the trailing collapsible space must still hang (CSS Text 3 §4.1.2), i.e.
    `current_line_last_hang` unperturbed by M1.
14. `white-space: pre` — `<pre> <span style="padding:5px"></span></pre>`: the line is already
    kept by clause 2 via the *preserved space text*, not by the empty inline; height must not
    double-count (M2 takes a `max`, not a sum).
15. Decorated inline adjacent to a forced break, incl. `<pre>\n</pre>`.
16. Decorated inline as the only IFC content — the `items.is_empty()` gate
    (`inline/mod.rs:161`) and the `any_font` probe (`:190-200`), both per M5: current
    behaviour in PR-1a, flipped in PR-1b.
17. `display: none` ⇒ no marker (`collect.rs:218`).
18. `position: absolute` ⇒ out of flow, clause 3 does not apply.
19. `position: relative`/`sticky` ⇒ sub-flow keying (M7); lands with the relpos suite.
20. **Wrapping on the box's own edges** — `<p style="width:100px">aaaa <span style="padding:20px"></span></p>`:
    the box's 40px of inline-axis padding participates in wrapping (M1(b), CSS Text 3 §5).
    Paired sub-cell: a box whose inline-axis edges are all zero (`padding-top` only) never
    wraps, because `advance_inline_box_edges` is called with 0.
21. **Vertical writing mode** — `padding: 5%` resolves against the containing block's inline
    size (= its physical height there), and `block_advance` uses `font_size` per M2.
22. **`margin-top` only** — the §8.3 tension in M7: the margin has no effect on a non-replaced
    inline, but clause 3's literal text keeps the line. Pins the literal reading.
23. **Undecorated inline sharing a newly-kept line** — a `pre-wrap` line where a collapsible
    `<i> </i>` sits beside the decorated span: the flip moves the line from the discard arm
    (`:423`) to the commit arm (`:210`), so `<i>` now commits a rect. Per §10.8 "Empty inline
    elements generate empty inline boxes", a zero-width box for `<i>` is correct — the cell
    pins that it is zero-width and not omitted.
24. **Decorated inline with text** — must **not** take a strut baseline (M3); the baseline
    comes from its glyphs.

**Non-regression**: `collapsible_whitespace_only_generates_no_line_box`
(`crates/layout/elidex-layout-block/src/inline/tests/text_height/basic.rs:204`) and
`nbsp_only_line_generates_a_box` (`:233`) unchanged.

## §7. Downstream of a box-suppression flip

Per [[feedback_inline-whitespace-edge-matrix-dense]], a line that *starts* existing is the
mirror of #266:

* `line_count` (fragmentation, `column-count`).
* IFC `height` / block cursor advance — non-zero per M2, so no zero-height line collides with
  `slice_and_rebase_fragment`'s exact-boundary assumption
  (`crates/layout/elidex-layout-block/src/inline/pack/fragment.rs:49`).
* `first_baseline` — moves only on a glyphless line (M3); cells 8 and 24 assert both directions.
* `entity_bounds` / `getClientRects` — the decorated box gets a **real** border box (M4);
  co-resident entities on newly-kept lines get theirs too (cell 23).
* Static positions of following abspos (`inline/pack/mod.rs:97`).
* `current_line_runs` / `flow_lines` — markers are never `FlowMember`s (M1), so member-less
  buckets keep being dropped by design; no ordinal dependency exists (§2 withdrawn row).
* `current_line_last_hang` / `last_placed_entity` — explicitly untouched (M1); cells 12/13.
* `clear_inline_flows` probe gating — a decoration-only IFC stops taking the early return at
  `inline/mod.rs:161` (whose clear is unconditional) and starts taking the full path (whose
  clear at `:637` is `!env.is_probe`-gated). Per M5 this happens at **PR-1b**, not PR-1a.
* Fragmentation (`inline/pack/fragment.rs`) and `ColumnFlowSlice`.

## §8. Definition of done

**PR-1a**: marker pair reaches `pack/items.rs`; every consumer handles it; edges resolved via
`resolve_box_model` against `containing_inline_size`; the two pre-pack gates hold current
behaviour; `helpers.rs:113`'s citation corrected; **characterization tests for cells 7/8/9/12
land asserting today's behaviour**; no behaviour change.
**PR-1b**: clause 3 accounted for in both shapes; every §6 cell tested or explicitly argued
unnecessary; every §7 consumer checked; every §5.1 M-row realised at the named site; new
`InlineItem`/`PackItem` variants and the clause-3 predicate carry docstring citations to their
§3 rows; the ⚠ caveat at `inline/pack/mod.rs:110` retires, its replacement naming the strut
divergence and pointing at `#11-inline-line-box-strut`; slot closes with its **original** DoD
(line *and* rectangle) met.

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires.** Its trigger is "the next change
  that touches `layout_inline_context_fragmented`'s body" — no growth qualifier — and PR-1a
  touches it at `:161`/`:190`, PR-1b at the persist block (the slot memo's named seam 3,
  `mod.rs:411-637`). **Disposition**: not bundled, because `inline/mod.rs` is **785 lines**
  (verified 2026-08-02), below CLAUDE.md's >1000 prereq-split mandate. Re-checked at PR-1b,
  whose persist-block touch is the larger one and lands on seam 3. (Revision 2 grounded this
  on axes.md Axis 5's >50 LoC bar, which is a review-time backstop scoped to files that are
  already over the line-count threshold, not this discipline's detector — corrected here.)
* **Production-file growth.** `pack/mod.rs` 791 and `inline/mod.rs` 785 (verified) are both in
  the 700–800 cut-while-writing band per [[feedback_touch-time-split-means-while-writing]].
  PR-1b adds `note_inline_box`, the edge advance, the tentative-baseline field and a `match pi`
  arm to `pack/mod.rs`. **Decision**: the box-marker packer surface (`note_inline_box` + edge
  advance + tentative baseline) lands in a **new `pack/inline_box.rs`** module from the start,
  not appended to `pack/mod.rs`. PR-1b's ~24 matrix cells land in a **new test module**, not in
  `tests/text_height/basic.rs` (303) or `relpos_subflow.rs` (915) — cell 19 is the one that
  would otherwise land in the 915-line file.
* **`#11-layoutbox-absence-unreachable`** (#488): recorded that "box-absent is unreachable for a
  once-laid entity". Revision 2's withhold collided with it; **M4 removes the collision** — the
  decorated inline now gets a box rather than needing a truthful absence signal.
* **`#11-inline-relayout-box-staleness`** — pre-existing (`inline/pack/boxes.rs:96`). M4 does
  not widen it: `assign_inline_layout_boxes` still skips entities that already carry a box
  (`:62`), which is the same staleness this slot inherits rather than creates.
* **`#11-css2-spec-label-normalisation`** — this memo uses `CSS 2 §` throughout. Adjacent
  `CSS 2.1 §` lines in touched files are left alone: the slot requires a single cross-crate
  commit, so in-passing normalisation is out.
* **`#11-inline-align-clientrects-nonpersist-path`** (`inline/pack/mod.rs:399`) and
  **`#11-justify-subflow-line-unified`** (`:57`, `:247`) sit in seams M4/M7 touch; neither is
  modified — M4 commits through the existing aligned-rect path rather than adding one, and M7
  reuses the existing keying.
* Intrinsic sizing: `measure.rs` is `if let InlineItem::Text`, so markers are ignored there.
  Correct for PR-1a; **PR-1b must revisit** `min_content_inline_size`/`max_content_inline_size`
  because M4's advance makes a decorated inline contribute inline-axis size.

## §10. Slot ledger actions at landing

`project_open-defer-slots.md` (the slot SoT per MEMORY.md) currently carries three Layout
inline slots — `#11-inline-align-clientrects-nonpersist-path`,
`#11-inline-relayout-box-staleness`, `#11-justify-subflow-line-unified` — but **not** this
umbrella's slot nor the #497 carves, which live only in `active-lane-detail.md` and their own
project memos. At landing: close `#11-line-box-decorated-inline-content` (PR-1b), open
`#11-inline-box-decoration-splits` (own) and `#11-inline-line-box-strut` (pre-existing), each
with the Why / trigger / re-eval date from §5.3, and register this umbrella's slot in the SoT.

## §11. Round-2 disposition

Round 2 = 2 CRIT / 18 IMP / 20 MIN. Disposition:

* **CRIT ×2** (R9's inverted percentage basis) — **fixed**, M7. Basis is the containing
  block's logical width = inline size, per css-box-4 and `helpers.rs:57-62`.
* **§11.1 root — `place_item` routing** — **fixed**, M1 (dedicated entry point; the four
  carve-outs and R8 all dissolve).
* **§11.2 root — strut** — **fixed by disclosure**, M2 + new slot `#11-inline-line-box-strut`;
  M2 also restores the packer's `is_vertical` convention and M3 the `!is_vertical` guard.
* **§11.3 — withhold insufficient / other entities' rects** — **fixed**, M4 + cell 23.
* **§11.4 — PR-1a not neutral** — **fixed**, M5 (characterization tests; three meeting points
  named, not one).
* **MINs** — applied: write-set correction (§4), Shape-A mechanism correction (§4.2),
  `inline/tests/mod.rs:21-22` added to PR-1a's consumer list, R8 re-grounded on CSS Text 3 §5
  (§3) and then made moot by M1, §8.3 tension disclosed (M7 + cell 22), cell 13/14 rewritten,
  §3 rows added for every new section with K/M recounted, docstring-citation requirement added
  to §8, `helpers.rs:113`'s non-existent citation scheduled for PR-1a, §10's false claim about
  the ledger corrected, §9's decomposition disposition re-grounded on the 785 < 1000 fact,
  production-file seam decided, PR-2's `Why deferred` written, and PR-2's own-vs-pre-existing
  classification split (the geometry half is gone — merged into PR-1b by M4 — so what remains
  is genuinely this umbrella's own).
* **Not carried**: revision 2's `group_key`-has-no-reader MIN is resolved by M7 (the marker
  pair is a `PackItem`, and `group_key` is read by PR-2's split handling) — but PR-1a must
  still `#[allow(dead_code)]` or otherwise account for fields whose first reader is PR-1b, and
  §8's PR-1a DoD does not yet say which. **Open, low-severity, for round 3.**

## §12. Round-3 outcome — revision 4 required

Round 3 = **8 CRIT / 24 IMP / 25 MIN**, up from round 2. Two distinct causes, and they need
different responses.

### §12.1 The plan is anchored on a superseded section — re-anchor on css-inline-3 §2.3

`body css-inline-3 invisible-line-boxes` (§2.3 *Phantom Line Boxes*) is the current-module
restatement of the CSS 2 §9.4.2 rule this entire memo is built on, and it is **narrower**:

> Line boxes that contain no text, no preserved white space, no inline boxes with non-zero
> **inline-axis** margins, padding, or borders, and no other in-flow content (such as atomic
> inlines or ruby annotations), and do not end with a forced line break are phantom line
> boxes. Such boxes must be treated as zero-height line boxes for the purposes of determining
> the positions of any descendant content … and both the line box **and its in-flow content**
> must be treated as not existing for any other layout or rendering purpose.

Consequences — all verified 2026-08-02 by direct webref lookup, not taken from the reviewer:

* **Clause 3 counts inline-axis edges only.** §6 cells 6 and 22 (block-axis-only `padding-top`
  / `margin-top` keeps the line) pin the **opposite** of the current spec, and M7's "any side,
  sign-inclusive" emit predicate is wrong. This also dissolves the §8.3 "tension" M7 disclosed
  — CSS 2 §8.3 says vertical margins have no effect on non-replaced inlines, and §2.3 agrees by
  excluding them from the rule. The tension existed only because the memo read a 1998 section
  in isolation.
* §2.3 also modernises two other clauses ("forced line break" not "preserved newline"; "atomic
  inlines or ruby annotations" not "images, inline blocks or inline tables") and states the
  in-flow-content consequence explicitly.
* Net effect on the design: **simpler**. The emit predicate narrows, two matrix cells flip to
  non-regression, and the axis-mapping question M7 raised (physical `EdgeSizes` → logical
  advance) becomes load-bearing in one place instead of two.

Two further spec findings, both verified directly and both contradicting revision 3:

* **`body css-text-3 line-break-details` (§5.5)**: "Out-of-flow boxes and **inline box
  boundaries do not introduce a forced line break or soft wrap opportunity** in the flow." So
  M1(b) must **not** arm the soft-wrap guard (`inline/pack/mod.rs:690`). The box's edges
  advance the cursor; an unsplittable box **overflows** (CSS 2 §9.4.2). §6 cell 20 pins the
  wrong behaviour, and revision 3's citation of css-text-3 §5 for M1(b) repeated round 2's R8
  error one level up — §5 exists, but §5.5 is the rule and it says the opposite.
* **`body css-text-3 boundary-shaping` (§7.3 *Shaping Across Element Boundaries*)**: "Text
  shaping must be broken at inline box boundaries when … Any of margin/border/padding
  separating the two typographic character units in the **inline axis** is non-zero." So §6
  cell 12 has it backwards: for `<p>a<span style="padding:1px"></span>b</p>` the two runs must
  **stop** coalescing, not keep coalescing. `last_placed_entity` must therefore be broken by a
  marker with non-zero inline-axis edges — the opposite of M1's "leaves it untouched".
* **`css-text-3 §5.6` does not exist** (`heading css-text-3 5.6` → "no headings under §5.6";
  §5 ends at §5.5). Cell 12 cited it, and the citation was inherited verbatim from
  `crates/layout/elidex-layout-block/src/inline/pack/mod.rs:726` ("CSS Text 3 §5.6 Shaping
  Across Intra-word Breaks"), where the section number **and** the title are both fabricated —
  css-text-3's only shaping section is §7.3. **Pre-existing defect in main**, the class #497
  swept; needs its own carve.

### §12.2 The authoring loop is the other cause — three CRITs were self-inflicted, again

Round 3's `current_line_entity_rects` contradiction was found independently by Axes 1, 2 and 3:
M1 lists the field among those the new entry points "neither touches", while M4 pushes the box
rect into exactly that field via exactly that function. M4's rect-source sentence was added by
the author **between rounds 2 and 3**, and M1's write-set list was not re-derived — only its
wrap-guard sentence was. This is the **third consecutive round** where the highest-severity
finding is in material added between rounds (round 2: R9; round 3: M4's rect push, and the
`css-text-3 §5` grounds for M1(b)).

Both remaining code-level CRITs are real and independent of that:

* **M1(b) leaves line state inconsistent after a wrap.** `flush_line` does **not** reset
  `on_line` — verified: its reset block (`inline/pack/mod.rs:432-439`) covers `current_inline`,
  `current_line_height`, `current_line_last_hang`, `any_rendered_content`,
  `last_placed_entity`, and nothing else; `on_line` is reset only in `force_break` (`:783`).
  `place_item` is safe only because `:697-698` re-establish state immediately after its own
  flush. Splitting the flush away from the state writes turns an internal invariant into an
  unstated call-order contract between two functions.
* **M4 double-counts the edges.** `assign_inline_layout_boxes` writes the accumulated bounds
  into `LayoutBox.content` (`inline/pack/boxes.rs:81`), and `border_box() = content + padding +
  border` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:140`). M4 pushes a rect already
  "inflated by the edges" **and** fills `LayoutBox.padding/border`, so background/border paint
  (`crates/core/elidex-render/src/builder/paint/mod.rs:68`, `:85`) sees the edges twice. The two
  readings coincide today only because the edges are hard-zero — which is precisely what M4
  changes.

### §12.3 What revision 4 must do

1. Re-anchor §1/§3/§5/§6 on **css-inline-3 §2.3**, keeping CSS 2 §9.4.2 as the historical
   reference. Narrow M7's emit predicate to non-zero **inline-axis** edges; flip cells 6/22 to
   non-regression; drop the §8.3 "tension" as resolved.
2. Fix M1(b): no soft-wrap arming (css-text-3 §5.5); overflow instead. Re-derive M1's write-set
   as the union across M1+M3+M4 rather than as "`place_item` minus four", and settle the
   call-order contract — or keep the flush at a single owner.
3. Fix M4: the new producer's rect is the box's **content** span; edges ride in the `EdgeSizes`
   only. Name the carrier that gets those edges to `assign_inline_layout_boxes` (its signature
   has neither the edges nor `containing_inline_size`), the open-box start-cursor **stack**
   (§6 cell 10 is nested), and the straddling-box case.
4. Break run coalescing at a decorated boundary (css-text-3 §7.3); re-pin cell 12.
5. Re-judge PR-1b's slice boundary: after M4's merge it spans 5 of §2's 8 coupling pairs, and
   §6 has no cell for the most common shape it now changes —
   `<p>a<span style="padding:10px">text</span>b</p>` — nor does §7/§8 cover the engine-wide
   repaint M4 causes for every decorated inline.
6. Carry the MINs: `current_line_entity_rects` push is `:706` not `:702` (three sites);
   "CSS Box Model L3 5.3" occurs at **four** sites (`helpers.rs:59`, `:114`,
   `positioned/layout.rs:90`, `block/mod.rs:162`), not one; §3 needs rows for css-text-3 §7.3
   and §4.1.2; the docstring-citation DoD sits on PR-1b but the variants land in PR-1a; §6 cell
   19's test destination contradicts §9; `#11-inline-line-box-strut`'s own/pre-existing
   classification needs deciding on the `origin/main` predicate; and PR-1a's dead-field question
   (`#[allow(dead_code)]` vs a smaller PR-1a payload) is still open.
