# Umbrella plan: inline box decoration in the IFC (`#11-line-box-decorated-inline-content`)

Slot `#11-line-box-decorated-inline-content`, opened by Codex on
[#497](https://github.com/send/elidex/pull/497). Layout lane's task, user-approved 2026-08-02.
Edge-dense ⇒ every PR under this umbrella goes through `/elidex-plan-review` before
implementation, and external review runs `/external-converge` from round 1.

All premises verified against `154bac3f`; every spec quote below was resolved directly with
`.claude/tools/webref`, not carried from a reviewer or from a code comment.

⚠ **Every `file:line` in this memo is a `154bac3f` coordinate and is evidence, not an
instruction.** Two prereq PRs move code before PR-1a: the seam-3 split relocates a block of
`inline/mod.rs`, and the dead-arm deletion removes a surface spanning **both** `pack/mod.rs` and
`inline/mod.rs` (§8 names it; it is not one contiguous range). The memo does **not** re-anchor
after each: its coordinates exist to prove claims about the code as it stands today. **Each PR's
own plan-memo re-anchors against its actual base**, and §8's DoDs name behaviours and call sites
wherever they can — a DoD that still carries a coordinate carries a `154bac3f` one and inherits
this note.

**Citation convention**: every *spec* section number is written with its module
(`css-inline-3 §2.3`, `CSS 2 §9.4.2`). A bare `§N` is always this memo's own section. CSS 2 is
verifiable with webref under the shortname **`CSS2`** (`webref heading CSS2 8.3`); the five CSS 2
section↔title pairs this memo carries were checked that way.

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
  container generates, not a special case. That reframes §5.2's gap — elidex is not missing an
  ad-hoc height floor, it is missing a box.
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
For **margins** this is independently confirmed by CSS 2 §8.3 ("vertical margins will not have any
effect on non-replaced inline elements"). ⚠ CSS 2 §8.3 is titled *Margin properties* and says
nothing about padding or border; the authority for **those** not entering line-box sizing is
`css-inline-3` §5.3, whose layout-bounds inflation by margin/border/padding applies only "when
`line-fit-edge` is not `leading`", and `css css-inline-3 line-fit-edge` gives `initial: leading`.

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
| 1 | **Line existence** — `css-inline-3` §2.3's five clauses | `any_rendered_content` (`inline/pack/mod.rs:115`) |
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
| 3 × 7 | The box's rect must be a *content* span, with edges carried separately, or the border box double-counts. | M4 | 1b |
| 1 × 3 | Flipping existence moves whole lines from discard to commit, affecting **every** entity on the line. | M5 | 1c |
| 1 × 5 | A line kept only by decoration needs a height source, and elidex has no root inline box. | M6 | 1c |
| 1 × 4 | `css-inline-3` §5.3 gives a strut only to a glyphless (or fallback-only) box, so the baseline source depends on the whole line. | M7 | 1c |
| 2 × 6 | Markers carry the enclosing recursion level's `group_key` — read only by the split rule, so the field arrives with its reader (M1 does not carry it in PR-1a). | `#11-inline-box-decoration-splits` | `#11-inline-box-decoration-splits` |
| 7 × (intrinsic) | An inline-axis advance the packer applies must also be visible to the intrinsic-size passes, which never build a `LinePacker`. | M8 | 1b |

Each **pair** is answered by exactly one M-row. M-rows are **not** partitioned by PR — M1's payload
is split across PR-1a (`entity`), PR-1b (the three `EdgeSizes` + the `WritingModeContext`), PR-1c
(`line_height` and font identity) and the splits slot (`group_key`) by §8's dead-field rule, and
pair 2×6 routes to a slot — and the invariant *sets* overlap by construction, because PR-1c consumes PR-1b's geometry.
What §5.3's slicing rests on is narrower and true: each **coupling** has one owning PR, and each PR
is behaviour-scoped (item stream / geometry / existence).

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
| CSS 2 §9.4.2 Inline formatting contexts | box that **cannot** be split | overflows the line box | M3 — no wrap check on the marker path; §6 cell 15 | ✓ | yes |
| CSS Break 3 §5.4 Fragmented Borders and Backgrounds: the box-decoration-break property | `slice` (initial) and `clone` | which side of an inline fragment is the broken edge is set by the **parent element's** inline progression direction — "neither the element's own direction nor its containing block's"; "no visual effect where the split occurs" is CSS 2 §9.4.2's phrasing, not this section's | **`#11-inline-box-decoration-splits`** — the source of a fragment's edge attribution differs from M1's own-direction side mapping, so it belongs with the rule that owns it | ✗ (deliberate, §5.3) | yes |
| CSS Inline 3 §2.2 Layout Within Line Boxes | Note on empty inline boxes | they still have a line-height and influence the calculation | M6 — **PR-1c** | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence (a) | zero-height for positioning descendant content (abspos) | `static_positions` (`inline/pack/mod.rs:97`) — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | layout-bounds inflation by edges | applies only when `line-fit-edge` ≠ `leading`; initial is `leading` | no code touch; grounds §1.2's block-axis exclusion | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 3 | non-zero **inline-axis** margin/padding/border | clause-3 predicate → `has_inline_axis_edge` (M5); the predicate lands in **PR-1b** with its M3 consumers, **PR-1c** substitutes it for the constant | ✓ | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | clause 5 | forced line break | `force_break` (`inline/pack/mod.rs:781`) — untouched | ✓ (pre-existing) | yes |
| CSS Inline 3 §2.3 Phantom Line Boxes | consequence | line box *and its in-flow content* do not exist | commit/discard seam (`inline/pack/mod.rs:210` vs `:423`) — **PR-1c** | ✓ | yes |
| CSSOM View 1 §6 Extensions to the Element Interface | `getClientRects()` step 3, single-fragment case | one `DOMRect` "describing its **border area**" — padding + border, never margin | the `border_box()` fallback (`element/layout_query.rs:236`), correct once M4's edges are real — **PR-1b** | ✓ | yes |
| CSSOM View 1 §6 Extensions to the Element Interface | `getClientRects()` step 3, multi-fragment case | one `DOMRect` per fragment, in **content order** | `InlineClientRects` keeps content spans; inflating them needs fragment identity and the parent's direction — **`#11-inline-box-decoration-splits`**; §6 cell 17d pins the divergence | ✗ (deliberate, §5.3) | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | glyphless / fallback-only box | strut with first-available-font metrics | tentative baseline — **PR-1c** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | half-leading | `A′ = A + L/2` | existing formula (`inline/pack/mod.rs:581`) — unchanged | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | break opportunities | inline box boundary is **not** one | M3 — the marker path calls the shared core without a wrap check — **PR-1b** | ✓ | yes |
| CSS Text 3 §5.5 Line Breaking Details | intra-word shaping | "the characters must still be shaped … as if the word were still whole" | the coalescing `pack/mod.rs:744` documents; its comment's citation is corrected in **PR-1b** (§3.1) | ✓ (pre-existing) | yes |
| CSS Text 3 §5.5 Line Breaking Details | adjacent soft wrap opportunity | break lands at the box's **margin edge** | **`#11-inline-box-decoration-splits`** | ✗ (deliberate, §5.3) | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | max-content | inline-axis edges occupy space | M8 (`inline/measure.rs:44`) — **PR-1b** | ✓ | yes |
| CSS Sizing 3 §5.2 Intrinsic Contributions | min-content | no accumulator to attach edges to | **`#11-inline-min-content-box-edges`** | ✗ (deliberate, M8) | yes |
| CSS Text 3 §7.3 Shaping Across Element Boundaries | shaping break | trigger 1 of 3: non-zero inline-axis edge | `last_placed_entity` coalescing (`inline/pack/mod.rs:744`) — **PR-1b** | ✗ (no shaping-break handling for trigger 2, `vertical-align` ≠ `baseline`, or trigger 3, "the boundary is a bidi isolation boundary" — set by `unicode-bidi`. Both properties resolve; the packer does not read either. Neither is this umbrella's subject) | yes |
| CSS Text 3 §4.1.2 Phase II: Trimming and Positioning | steps 3–4 | a collapsible space is line-final only if nothing follows it on the line | `current_line_last_hang` (`inline/pack/mod.rs:701`) — **PR-1b** | ✓ | yes |
| CSS Text 3 §4.1.1 Phase I: Collapsing and Transformation | step 4 | collapsing crosses inline box boundaries | `collapse_inline_whitespace` (`inline/whitespace.rs:41`) — **PR-1a** | ✓ | yes |
| CSS Inline 3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | Quirks Mode | an inline box fragment with **zero borders and padding** and no direct text is ignored when sizing the line box | not implemented — no quirks-mode layout switch exists; **§9** | ✗ (deliberate, §9) | yes |
| CSS 2 §10.8.1 Leading and half-leading | glyphless inline box | strut with first-available-font metrics (superseded by CSS Inline 3 §5.3, kept as the historical anchor) | M7 — **PR-1c** | ✓ | yes |
| CSS 2 §8.3 Margin properties: margin-top, margin-right, margin-bottom, margin-left, and margin | non-replaced inline elements | vertical margins have no effect | consistent with CSS Inline 3 §2.3 clause 3; no code touch | ✓ | yes |
| CSS 2 §9.4.3 Relative positioning | relpos inline in flow | decorated relpos inline; sub-flow keying | `collect.rs:286` — **`#11-inline-box-decoration-splits`** (the `group_key` field arrives with its only reader, §2 pair 2×6) | ✗ (deliberate, §5.3) | yes |
| CSS Box Model 3 §3.1 / §4.1 | percentage margin/padding | logical width (= inline size) basis | `resolve_box_model` (`helpers.rs:116`) — **PR-1a** | ✓ | yes |
| CSS Writing Modes 4 §6.1 Abstract Dimensions | inline size ≡ logical width | basis identity in vertical modes | same | ✓ | yes |
| CSS Writing Modes 4 §3.2 Block Flow Direction: the writing-mode property | box whose `writing-mode` differs from its **parent box** | an otherwise-`inline` box's display computes to `inline-block` | M1's emit test — such a box is an atomic and gets no marker; §6 cell 12c — **PR-1b** | ✓ | yes |
| CSS Writing Modes 4 §6.4 Abstract-to-Physical Mappings | side mapping | "based on the **used** `direction` and `writing-mode`" of the box being mapped | M1's `WritingModeContext` source; §6 cells 12b/12e — **PR-1b** | ✓ | yes |
| CSS Backgrounds 3 §3.2 Line Patterns: the border-style properties | `none` / `hidden` | width ignored ⇒ 0 | already zeroed at computed-value time (`elidex-style resolve/box_model/mod.rs:260`) | ✓ | yes |

**Breadth**: K=9 specs (CSS Inline 3, CSS Text 3, CSS 2, CSS Break 3, CSS Box Model 3,
CSS Sizing 3, CSS Writing Modes 4, CSS Backgrounds 3, CSSOM View 1), M=32 entries (`Split decision` below restates K; both are recomputed). Both figures are recomputed
from the table above by `python3 .claude/tools/plan-xcheck.py <memo>`, which prints them and fails
on drift — that command is the verification artifact, and it is re-runnable rather than dated.
⚠ `preflight.py` reports `parsed citations: 0` here: its `SPEC_LABEL_REVERSE` has no CSS-module
labels, so its citation hard-gate is **vacuous for this memo** and every §-number below was
verified by hand with `.claude/tools/webref` instead. Closing that gap is
the plan-checker tooling task's (§9), not this umbrella's.

**Split decision**: K=9 ⇒ SPLIT-DEFAULT. The plan **is** split into three shipping PRs, each
behaviour-scoped, with one owning PR per coupling (§2, §5.3); the breadth verdict and the
invariant-axis verdict agree.

### §3.1 User-input touch audit

Adjacent pre-existing laxity:

* `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs:82` hard-codes
  `EdgeSizes::default()` on every inline `LayoutBox`. **PR-1b fills it** (§5.1 M4) — for
  *every* decorated inline, not only the empty ones this slot names, because that is the gap
  §4.3 identifies and it is what makes the PR-1c flip honest rather than half-true.
* **Root inline box absent** (`grep -rni strut crates/` → nothing in inline layout):
  pre-existing, newly *depended on* by PR-1c. Disclosed in M6 with its own slot.
* **`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:726` cites "CSS Text 3 §5.6
  Shaping Across Intra-word Breaks"** — the section number and the title are **both**
  fabricated (`heading css-text-3 5.6` → no headings). The correct target is **css-text-3 §5.5**,
  not css-text-3 §7.3: that comment documents *intra-word* coalescing so render shapes whole
  words, and css-text-3 §5.5 is where that rule lives ("the characters must still be shaped … **as if the word were still
  whole**"). CSS Text 3 §7.3 is the opposite-direction rule M3 uses (shaping must **break** at a
  decorated boundary). PR-1b rewrites the comment, so both land: css-text-3 §5.5 for the
  surviving intra-word coalescing, css-text-3 §7.3 for the new boundary break.
* **A nonexistent "CSS Box Model L3/Level 3 §5.3" is cited across `elidex-layout-block`**
  (css-box-3 §5 is *Borders*, no subsections). The class is defined by the **concept** grep

  ```
  grep -rEn "Box Model (L3|Level 3)[^a-z]*(§)?5\.3" crates/
  ```

  The correct target was established first, not inferred: `webref heading css-box-3 3` → `§3.1 Page-relative (Physical) Margin Properties`, `webref heading css-box-3 4` → `§4.1 Page-relative (Physical) Padding Properties`, `webref body css-box-3 margin-physical` → "Percentages: refer to logical width of containing block". The class is then whatever this grep returns; its hits on `154bac3f` are `lib.rs:178`, `helpers.rs:59`, `helpers.rs:114`,
  `positioned/layout.rs:90`, `block/mod.rs:162`, `block/mod.rs:182`,
  `block/children/helpers.rs:213`. PR-1a corrects **every hit of that grep** to css-box-3
  §3.1/§4.1 in one commit per [[feedback_semantic-sibling-selfseed-and-regate-breadth]]; the DoD
  states the property, not a count, so it cannot go stale against the grep.
  ⚠ **Why a string grep must not be used here.** `grep -rn "Box Model L3" crates/` misses the
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
  `entity != self.parent_entity` at `:703`), the flow-member bucket (`:736-769`, requiring a
  `FlowMember` — `inline/pack/items.rs:49`), `last_placed_entity` (`:772`).
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

`assign_inline_layout_boxes` hard-codes zero edges (`inline/pack/boxes.rs:82`) and the packer
never advances for an inline box's edges. CSS Inline 3 §2's "Inline-axis margins, borders, and
padding are respected between inline-level boxes" is unimplemented engine-wide. **This is why
the slot is an umbrella, and why §5.3 puts geometry before the existence flip**: the geometry
gap is visible today on ordinary `<span style="padding:10px">text</span>`, independently of
whether any line is phantom.

## §5. Design

### §5.1 Mechanism table

| # | Question | Decision | Grounds |
|---|---|---|---|
| **M1** | What enters the item stream, carrying what? | Two `InlineItem` variants, `InlineBoxStart` / `InlineBoxEnd`, emitted around the recursion at `inline/collect.rs:291` for every inline with **at least one non-zero edge on any side**. Payload: `entity` + the **three physical `EdgeSizes`** `resolve_box_model(&style, containing_inline_size)` returns (`helpers.rs:116`) + the `WritingModeContext` they were resolved under (`logical.rs:27`), built from **the decorated inline's own `style`** (`inline/collect.rs:217`) — `WritingModeContext::new(style.writing_mode, style.direction)`. Grounds: <br>• **The axis is the IFC's by construction, so its source cannot matter.** css-writing-modes-4 §3.2 *Block Flow Direction*: "If a box has a different `writing-mode` value than **its parent box** … If the box would otherwise become an in-flow box with a computed display of `inline`, **its display computes instead to `inline-block`**." (The trigger is the *parent box*, not the containing block, and the "establishes an independent … formatting context" clause of the same rule applies only to a box that is a block container — an inline reaches `inline-block` by the display change, not by that clause.) A decorated *inline box* therefore always shares the IFC's writing mode; a `<span style="writing-mode:vertical-rl">` computes to `inline-block`, i.e. an atomic, which M1 emits no marker for. <br>• **The direction is the box's own.** css-writing-modes-4 §6.4 gives the abstract-to-physical mappings "based on the **used** `direction` and `writing-mode`" — of the box whose sides are being mapped. `direction` *can* differ on an inline box without forcing an independent context, so it is the one component that varies, and css-writing-modes-4 §6.2/§6.4 say it is the box's own. <br>• **Why the two competing readings fail**: css-writing-modes-4 §2.1's "`direction` has no effect on bidi reordering … when `unicode-bidi` is `normal`" is about reordering *content*, not about mapping a box's own sides — it does not license taking the direction from elsewhere (the proposition *is* verbatim in css-writing-modes-4 §2.1, and cell 12b's only `[dir]` element is the `<p>`, so its `<span>` really is `unicode-bidi: normal`; the citation is sound and still does not reach). css-break-3 §5.4's "the **parent element's** inline progression direction … neither the element's own direction nor its containing block's direction is used" is scoped to *which side of a fragment is the broken edge*, and its own example is an element that "breaks across two lines" — an unfragmented box has no broken edge. §3 routes that whole section — both `slice` and `clone` — to `#11-inline-box-decoration-splits`, which is also where the parent-direction source lives, so the two direction sources never meet inside one PR. <br>Only the physical edge *values* come from the element's own `ComputedStyle`. Logical facts are *derived* at the point of use via `LogicalEdges::from_physical` (`logical.rs:186`), applied to each set separately: their inline-start/inline-end components summed across the three sets give M3's advance and M4's content offset; the same components tested *per set* give M5's predicate. **Both are derived in `pack/inline_box.rs` beside `has_inline_axis_edge`, not cached on the marker** — the sums' inputs already sit on the payload, and §5.2 designates that module the one derivation site for everything read off a marker, so a stored total would be a second representation of a fact the same struct already determines. `helpers.rs`'s `inline_pb` (`:148`) is **not** reusable here — it covers padding + border only, and the advance must include margin. The `PackItem` forms are `InlineBoxStart { item_index }` / `InlineBoxEnd { item_index }` — **an index, not a copy of the payload**, per §4's `PackItem` idiom; markers never become `FlowMember`s. ⚠ **Split across PR-1a and PR-1b by §8's dead-field rule.** PR-1a emits `InlineBoxStart { entity }` / `InlineBoxEnd { entity }`: the emit *test* resolves the edges at collect time and then discards them, because the variants' only PR-1a readers are the exhaustive matches. The three `EdgeSizes` + `WritingModeContext` join the payload in **PR-1b**, with their first readers M3/M4/M5. The rule the DoD states therefore reaches this payload too, not only `line_height` / font identity / `group_key`. | **Three sets, not one**: `LayoutBox` has independent `padding`/`border`/`margin` (`crates/core/elidex-plugin/src/layout_types/boxes.rs:88-91`) consumed separately by `padding_box`/`border_box`/`margin_box` (`:134-150`), and `resolve_box_model` already returns the triple — one `LogicalEdges` cannot fill three. **Physical, not logical, across the boundary**: `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) writes *physical* fields and receives `is_vertical: bool` with **no `direction`**, so a logical payload would need a return trip whose context is not present there. Carrying physical + the ctx keeps one conversion direction and no reconstruction. **Index, not copy**: a copied payload would duplicate the item stream's data against the file's own `item_index` idiom, and would give M5 two derivation sites — the pre-pack gate holds `&InlineItem` and the packer holds a `PackItem`, so only an index lets both read the *same* payload through one function. Emit on **any** side because M4 must fill all four for a block-axis-only inline to paint; the *existence* test is inline-axis-only (M5) — see §6 cell 6 for the pair. `resolve_box_model` is mandatory: `sanitize_padding` (`:96`) resolves percentages against 0, `sanitize_border`/`sanitize_edge_values` (`:102`/`:48`) clamp non-negative. Basis = logical width = inline size (css-box-3 §3.1/§4.1; css-writing-modes-4 §6.1), `resolve_padding`'s documented contract (`:57-62`). |
| **M2** | Collapse-pass arm | **Transparent** — same shape as `InlineItem::Placeholder` (`inline/whitespace.rs:53`), not the `Atomic` barrier (`:45`). Verified: the `Placeholder` arm touches neither `prev_collapsible_space` nor `prev_text_idx`, and the `:59` lookback indexes a recorded *text* index, so interleaved markers are skipped by construction — including an adjacent `Start`/`End` pair. | §1.6 (css-text-3 §4.1.1 step 4): collapsing crosses "the boundary of the inline containing that space". A barrier would change `a<span style="padding:10px"> </span>b`, currently-correct markup. |
| **M3** | How does anything occupy a line? | **One owner, two callers.** Extract `LinePacker::note_line_occupancy(inline_advance, block_advance, hang: Option<f32>, contributes_content, occupant: LineOccupancy)` (NEW, private) writing exactly the five line-state fields `place_item` writes today: `current_inline +=` (`:695`), `current_line_height = max(..)` (`:696`), the line-occupancy raise that replaces `on_line = true` (`:697`, see the ⚠ below), `any_rendered_content \|=` (`:698`), and — **only when `hang` is `Some`** — `current_line_last_hang =` (`:701`). `place_item` calls it **after** its soft-wrap check (`:690`) and **after** snapshotting `seg_inline_start = self.current_inline` (`:694`, which five downstream sites consume), passing `Some((full-trimmed).max(0.0))` and `occupant = Content` — behaviour unchanged. The marker path calls it with **no** soft-wrap check, `occupant = BoxEdgeOnly`, `contributes_content` from M5, and `hang = Some(0.0)` when M5's `has_inline_axis_edge` holds, **`None` otherwise** — a zero-advance marker (a `padding-top`-only box, cell 6) leaves the hang alone, because the space before it *is* still line-final. A marker for which `has_inline_axis_edge` holds also clears `last_placed_entity` (`:772`). | Without a shared core, *entering the line* is unowned: the marker path would either duplicate `place_item`'s five-write sequence or silently skip part of it. `Option<f32>` rather than an `f32`: the field is a *conditional* write for the marker and an unconditional one for `place_item`, and an `f32` parameter cannot express "leave it alone" — `place_item` assigns unconditionally today (`:701`). No wrap check: §1.3, a box boundary is not a break opportunity. `hang = Some(0.0)`: css-text-3 §4.1.2 keys on "at the end of **a line**", the engine encodes that at `:264-279`, and `place_item` already zeroes it for an atomic. Shaping break on non-zero **components** (§1.4's wording), so a negative or compensating pair still breaks. **One 3-valued field, not two bools**: the two readers want different questions of the same fact ("has anything entered this line?" for `finish()`, "has *content* entered it?" for the wrap guard), which is one ordered state, and CLAUDE.md's "one issue, one way" prefers encoding it once over an invariant maintained across three sites. ⚠ **`on_line` has a second reader** — the soft-wrap guard's `&& self.on_line` (`:690`) — and a marker at the head of a line would arm it, so the first content segment could soft-wrap where `on_line == false` protects it today. The guard has always meant *do not flush a line with nothing on it*, and "nothing" turns out to be the absence of **content**, not of a box boundary — so the line now has **three** states, not two. **Mechanism**: `on_line: bool` becomes `line_occupancy: LineOccupancy` (`Empty` / `BoxEdgeOnly` / `Content`), written in exactly two places — `note_line_occupancy` **raises** it monotonically from an occupant argument (`place_item` passes `Content`, the marker path `BoxEdgeOnly`), and `flush_line`'s per-line reset block (`:432-439`) sets `Empty`. The soft-wrap guard (`:690`) tests `== Content`; `finish()` (`:787`) tests `!= Empty`. ⚠ Adding it to the reset block is **behaviour-neutral today** and removes the asymmetry §4 records: `flush_line`'s three callers are `place_item`'s soft-wrap (which raises to `Content` at `:697` immediately after), `force_break` (whose `on_line = false` at `:783` becomes the same reset), and `finish()` (after which the value is never read). A second bool would instead need three write sites, an implicit `content_on_line ⇒ on_line` invariant, and a reset asymmetry no single field pays. §6 cell 15b pins it. |
| **M4** | Where does the box's rect come from, and how do the edges reach `LayoutBox`? | **Rect**: an open-box **stack** records each `InlineBoxStart`'s cursor; `InlineBoxEnd` pops it and pushes one `current_line_entity_rects` entry (`:706`) for `entity` spanning **content-start → end-cursor** (read **before** the end marker's own advance) — the box's **content** span, never inflated by edges. Pushed by an explicit branch, not the `entity != parent_entity` guard (`:703`), which does not suppress a nested inline. ⚠ **Two channels, one buffer — and PR-1b fixes only the one it can.** `getClientRects` returns `InlineClientRects` **early**, never touching `border_box()` (`crates/dom/elidex-dom-api/src/element/layout_query.rs:219-232`), and cssom-view-1 §6 step 3 requires "one for each box fragment, describing its **border area**". `LayoutBox.content` must stay the *content* union or `border_box() = content + padding + border` double-counts. Those are contradictory demands on one value: `commit_aligned_entity_rects` builds a single `painted` rect and feeds it to **both** `EntityBounds`'s min/max bounds (→ `LayoutBox.content`, `boxes.rs:80-81`) and `EntityBounds.line_rects` (→ `InlineClientRects`, `boxes.rs:102-126`) — verified at `pack/mod.rs:488-509`. **All three buffers keep *content* spans, exactly what they hold today.** PR-1b therefore makes the **single-fragment** channel correct and nothing else: no `InlineClientRects` is stored below `len() > 1`, so `getClientRects` falls back to `LayoutBox.border_box()`, which M4's real edges make right. ⚠ **The multi-fragment channel stays content-span and is a disclosed divergence, not a fix PR-1b withholds** — inflating stored fragments is `#11-inline-box-decoration-splits`'s work, on three grounds PR-1b cannot discharge: (a) **content-order identity** — `slice_and_rebase_fragment` does `b.line_rects.retain(…)` (`pack/fragment.rs:69`) immediately before the consumer (`inline/mod.rs:377` then `:380`), so under paging/multicol `line_rects[0]` is the first *kept* rect in this fragmentainer, not the box's first fragment in content order; (b) **which edge survives a break is the *parent's* inline progression direction** per css-break-3 §5.4 — "neither the element's own direction nor its containing block's" — which is a different source from M1's own-direction side mapping and belongs with the rule that owns it. ⚠ Reachability is deliberately **not** a third reason: the derivation would run in `assign_inline_layout_boxes`, which `continue`s on an existing `LayoutBox` (`boxes.rs:60-62`), but §8 records that limit for *every* geometry PR-1b writes — M4's own edge write included — so it cannot discriminate between what stays and what leaves. §6 cell 17d pins the divergence as accepted, in the shape cells 23 and 24 already use. The stack entry stores the box's **content-start cursor** (already past the marker's inline-start advance), so the rect is `content-start → end-cursor` with no edge re-added — the same content-span meaning `place_item`'s rects already carry. Its `block_start` is snapshotted from `current_block_offset` at the same moments `place_item` snapshots it, so all of a line's rects share one value — the invariant `commit_aligned_entity_rects` relies on. `flush_line`, at the top before any arm runs, walks the **whole** open-box stack: it **emits** a partial rect for each entry whose span is non-empty, and **rebases every entry's content-start to 0 unconditionally** — the two scopes differ, and binding the rebase to the emitted set would leave a box opened at the end of line N (span 0, no rect) holding a line-N cursor into line N+1 and yielding an inverted rect. §6 cell 17c pins it — the start edge was consumed on the earlier line and must not be applied again. The end edge is symmetric and needs no handling: an open box has not reached its end marker, so line N's partial rect reserves nothing for it, so a box straddling a break yields one rect per line and a box opened exactly at a break yields none. ⚠ **The marker's rect is a second producer for the same entity**: a decorated inline with text already has a `place_item` rect on that line (`:706`, its runs carry `entity == span`). The **persisting** arm folds per entity before committing (`commit_aligned_entity_rects`, `:479-487`), so one fragment per line survives — correct, and the only arm that runs. The non-persisting arm — the whole `else` clause at `pack/mod.rs:393-421`, whose unmerged rect loop is `:401-420` — does **not** fold, but it is **dead code**: `FragmentationType` has exactly `Page` and `Column` (`crates/layout/elidex-layout-block/src/lib.rs:37-42`) and `InlineFragConstraint.fragmentation_type` is non-optional (`inline/mod.rs:94`), so `persist_candidate = frag_constraint.is_none() \|\| frag_is_paged \|\| frag_is_column` (`:239`) is **identically true** and `flow_align` is always `Some`. No cell is written against that arm: a cell no markup can construct is exactly what M8's grounds refuse. The dead arm is deleted by a prereq PR (§9). **Edges**: **no side-store.** `assign_inline_layout_boxes` (`inline/pack/boxes.rs:48`) gains one `containing_inline_size: f32` parameter — already in scope at its only call site (`inline/mod.rs:380`, inside `layout_inline_context_fragmented`) — and calls `resolve_box_model(&style, containing_inline_size)` itself, on the entity's `ComputedStyle` — which it already touches at `boxes.rs:57`, today only as an `is_err()` guard, so the change is to *bind* it and copy the three `EdgeSizes` out before the `&mut EcsDom` borrow the same loop takes for `set_layout_box`. Note this is **two call sites of one pure function** (collect-time for the packer, box-assign-time for the `LayoutBox`), not a de-duplication: the packer has no `ComputedStyle` in hand and the box-assigner has no marker. The marker still carries the triple for M3/M4/M5, which run in the packer where no `ComputedStyle` fetch is in hand. | `assign_inline_layout_boxes` writes bounds into `LayoutBox.content` (`:81`) and `border_box() = content + padding + border`, so an edge-inflated rect double-counts — which is why the border area is derived at the write rather than stored in the accumulator both consumers read. Reading the end-cursor before the end advance is what keeps the end side from double-counting too. The flush-time **emit** is what a rebase-only version misses: `InlineBoxEnd` has not run at line N's flush, so line N's fragment would simply be lost. **Derive at the point of use, do not thread a map**: the box-assigner already fetches the `ComputedStyle` it would need (`boxes.rs:57`) and `resolve_box_model` is a pure intra-crate function (`helpers.rs:116`), so a `&HashMap<Entity, (EdgeSizes, EdgeSizes, EdgeSizes)>` parameter buys nothing and adds a producer/consumer pair to keep in sync. ⚠ CLAUDE.md's side-store→component rule is **not** the ground here and is not invoked: its subject is a persistent entity-keyed registry and its prescribed remedy is a component, not point-of-use derivation — and `assign_inline_layout_boxes` already takes `entity_bounds: &HashMap<Entity, EntityBounds>` (`boxes.rs:50`), which the rule would condemn equally if it reached. Not an `EntityBounds` member either — that type is a per-line geometry accumulator whose `or_insert`/`and_modify` pair would not re-set an element-constant on a multi-line inline. ⚠ **Both derivation sites must share one `containing_inline_size`** or the percentage edges disagree between the marker and the `LayoutBox`; they do today because both calls (`inline/mod.rs:154` and `:380`) read the one parameter of `layout_inline_context_fragmented` (`:144`). A second `assign_inline_layout_boxes` caller would break that silently, so the parameter carries the invariant as a docstring contract. |
| **M5** | What makes a line non-phantom, and what else commits? | **The clause-3 predicate is a function of the marker's edges, not a per-PR constant**, and it is a **disjunction, not a sum**: `contributes_content = has_inline_axis_edge(marker)`, true iff **any one** of the three `LogicalEdges` (padding, border, margin — each converted separately by `LogicalEdges::from_physical`) has a non-zero `inline_start` or `inline_end`. **M5 owns this predicate, and it lands in PR-1b** — M3's shaping break and hang gate are PR-1b deliverables and both call it, so the predicate cannot wait for PR-1c. What PR-1c adds is the one-argument *substitution* below, not the predicate. M1's emit test (any side) is a different test on the same payload and stays in PR-1a. Evaluated per marker as a **free function over the marker's `InlineItem` payload**, in `pack/inline_box.rs` beside the stack — deliberately *not* an `impl LinePacker` method, because the pre-pack gate at `inline/mod.rs:200` must call it too and runs before `LinePacker::new` (`:251`). One derivation site, two callers: the gate holds `&InlineItem` directly, the packer resolves its `PackItem`'s `item_index` into the same slice (M1). ⚠ The gate's escape is **qualified**, not a bare "marker escapes beside `Atomic`": `contributes_content` is a pure text predicate with no font dependence (`pack/mod.rs:556`), so a block-axis-only marker escaping `:200` would let font-less text commit a line that §1.2 says stays phantom. In PR-1b the marker path passes a constant `false` (geometry only); **PR-1c replaces that constant with this expression** — that one-argument substitution *is* the existence flip. Once a line is non-phantom, css-inline-3 §2.3's "the line box **and its in-flow content**" applies: every entity on it commits through the existing seam (`:210`), none withheld. | A literal `true` for PR-1c would keep a `padding-top`-only inline's line alive — contradicting §1.2, M1 and cells 5/6. Emit and existence are different predicates over the same payload, so the existence one needs its own site. **Disjunction, not sum**: css-inline-3 §2.3 lists "non-zero **inline-axis** margins, padding, or borders" — three separately-named quantities, so `margin-left:-10px; padding-left:10px` — which sums to zero — still keeps the line. This is the one place the three sets must stay separate; M3's *advance* and M4's *content offset* both take the sum, because geometry adds up and a negative margin really does pull content back. §6 cell 3b pins the cancelling pair. |
| **M6** | Height of a line kept only by decoration | `InlineBoxStart` carries the element's resolved `line_height` and font identity; M3's shared core takes `current_line_height = max(block_advance)` with `block_advance` following the packer's existing vertical convention (`if is_vertical { font_size } else { line_height }`, `:539-543`). **The memo does not claim css-inline-3 §5.3/§2.2 conformance**: the block container's **root inline box** (§1.1) is unimplemented, so the line's height floor is missing. The text path already diverges identically — `<p style="line-height:40px"><span style="line-height:5px">x</span></p>` yields 5px today, with no marker involved. New slot **`#11-inline-root-inline-box`**, pre-existing class; §6 cell 23 pins the divergence so it stays distinguishable from a bug. | The direct authority is `body css-inline-3 line-layout` (§2.2) Note: "Empty inline boxes still have margins, padding, borders, and a **line-height**, and thus influence these calculations just like boxes with content." css-inline-3 §5.3 defines the strut per *box* and does not address the line-level question. Disclosing a pre-existing divergence the new code depends on is the §4.3 pattern. |
| **M7** | Which line gets a strut baseline | `InlineBoxStart` records a tentative `current_line_box_baseline: Option<f32>` from the box's first-available-font metrics via `FontDatabase::query` + `font_metrics` (`crates/text/elidex-shaping/src/database.rs:60`, `:101`), keeping the `!is_vertical` guard (`:575`). `flush_line` promotes it into `first_baseline` **inside** the `if self.any_rendered_content` arm (`:210`) — never on a suppressed line — and only if `first_baseline.is_none()`. The field joins `flush_line`'s per-line reset block (`:432-439`). §4 enumerates that block's current contents; **no site states a running total** — a count restated away from the enumeration it summarises drifts from it. | §1.5: a strut exists only for a glyphless box; css-inline-3 §2.2 owns the line-level composition. No second flag: the text arm sets `first_baseline` at pack time (`:575`), so `is_none()` at flush already answers "did any glyph-bearing segment land here or earlier". Traced against all three orderings. `query`+`font_metrics` rather than `measure_text`, because a glyphless box has no string to shape and the metrics are string-independent anyway (`elidex-shaping/src/measurement.rs:55`). ⚠ **Known residual**: if a line's text has no usable font, `measure_text` returns `None`, `first_baseline` stays `None`, and a co-resident box's tentative promotes on a line that does have glyphs. §6 cell 24 pins it as accepted. |
| **M8** | Intrinsic sizing | **`max_content_inline_size` only** (`inline/measure.rs:44`): each marker adds its inline-axis edge sum to the running total, matching that pass's existing `total +=` shape. **`min_content_inline_size`'s accumulator is out of scope and gets a slot** — `#11-inline-min-content-box-edges` (own deferral): the pass is `max_word = max_word.max(m.width)` per word per item (`:15-37`) with **no running candidate and no cross-item joining**, so a box's edges have nothing to attach to; giving them one is an accumulator restructure of a pass that also already ignores run boundaries (`a<b>b</b>c` yields `max(\|a\|,\|b\|,\|c\|)` today, never `\|abc\|`). Trigger: PR-1c landing, or any shrink-to-fit correctness work. Re-eval: 2026-11-01. **PR-1b**, cell 25 scoped to max-content. Grounds: css-sizing-3 §5.2 defines the *contribution* but notes it "does not define precisely how to determine these sizes", so splitting the two passes is an engine choice, not a spec deviation. | Any rule that adds a box's edges to a *running candidate* names an accumulator this site does not have. Scoping to the pass that *can* host it, and slotting the one that cannot with its real cost stated, is the honest split — the alternative ships a DoD cell that cannot pass. |

### §5.2 Layer ownership

| Layer | Owns |
|---|---|
| `elidex-style` computed values | `border-*-width` already zeroed for `border-style: none`/`hidden` (`crates/css/elidex-style/src/resolve/box_model/mod.rs:260`), so `ComputedStyle.border_*.width` **is** the used width. |
| `crates/layout/elidex-layout-block/src/helpers.rs` | Edge resolution — `resolve_box_model` (`:116`) against `containing_inline_size`. |
| the bogus-cite sweep (`helpers.rs`, `lib.rs`, `positioned/layout.rs`, `block/mod.rs`, `block/children/helpers.rs`) | PR-1a's one-commit class fix across every hit of §3.1's concept grep. Listed as its own row because four of the five files it edits are not otherwise this program's (`helpers.rs` is, one row above). |
| `crates/layout/elidex-layout-block/src/inline/styled_run.rs` | **`InlineItem` (`:9`) — where M1's two marker variants are declared**, beside `Text`/`Atomic`/`Placeholder`. `StyledRun` (`:43`) itself is unchanged; M1 explicitly does not widen it (§5.3 Rejected). |
| `crates/layout/elidex-layout-block/src/inline/collect.rs` | Emits `InlineItem`, incl. the marker pair, with the payload M1 specifies, using the `parent_style` already in scope. Gains **one** new parameter: `containing_inline_size` on `collect_inline_items` (`:136`) / `collect_inline_items_inner` (`:194`). `root_horizontal` (`:211`) is unchanged. Every `collect_inline_items` caller — enumerated by `grep -rn 'collect_inline_items(' crates/` minus the definition — takes the new argument: `inline/mod.rs:154` (has the value), `inline/measure.rs:22` and `:51` (the intrinsic passes, where a containing inline size is definitionally unavailable — see §9), and the test helper `inline/tests/mod.rs:17`. |
| `crates/layout/elidex-layout-block/src/inline/pack/items.rs` | `PackItem` (`:18`) and `FlowMember` (`:49`). Markers get `PackItem` forms carrying `item_index` only (M1); they never become `FlowMember`s. |
| `crates/layout/elidex-layout-block/src/inline/pack/mod.rs` | `LinePacker` line state. **M3's `note_line_occupancy` lives here**, beside `place_item` (`:679`), its first caller. `flush_line` (`:209`) **calls** the open-box hook that `pack/inline_box.rs` owns (M4), promotes M7's tentative baseline inside its `:210` arm, and grows its per-line reset block (M3's `line_occupancy` in PR-1b, M7's tentative in PR-1c). The fabricated shaping citation at `:726` is rewritten by M3. |
| `crates/layout/elidex-layout-block/src/inline/mod.rs` | The IFC entry point. Sites this program writes, by PR: `collect_inline_items`'s call (`:154`, PR-1a, one argument); `items.is_empty()` (`:161`, held in PR-1a, flipped in PR-1c); the `any_font` closure's exhaustiveness arm (`:192-199`, PR-1a, behaviour-neutral); the **outer early-return condition** (`:200`, PR-1c, gains an escape for markers **that satisfy `has_inline_axis_edge`** (M5), beside the existing `Atomic` one); `assign_inline_layout_boxes`'s call (`:380`, PR-1b, one argument); and §7's `clear_inline_flows` gating (`:637` — ⚠ inside the seam-3 range, so this site moves to the extracted module before PR-1a). The dead-arm prereq PR writes three further sites here; they have their own row below. |
| the dead-arm prereq PR's surface | `flush_line`'s `else` arm and its unmerged rect loop (`pack/mod.rs:393-421`) **and** the `inline/mod.rs` half the reachability argument kills: `persist_candidate` (`:239`), `flow_align`'s `Option` construction (`:240-251`), `persist_flow`'s now-redundant conjunct (`:322`) and the comments that explain the two-path model (`:227-230`, `:309-320`, `:329-330`). Listed as a layer of its own because the PR spans two files, which no other row does, and because every one of its six `inline/mod.rs` items lies **above** seam 3 — the fact §8 uses to conclude the two prereqs are independent rather than ordered. |
| `crates/layout/elidex-layout-block/src/inline/whitespace.rs` | `collapse_inline_whitespace` (`:41`) — M2's transparent arm. |
| `crates/layout/elidex-layout-block/src/inline/measure.rs` | `max_content_inline_size` (`:44`) — M8's contribution. `min_content_inline_size`'s **accumulator** — `max_word` at `:23`/`:31` — is not touched (see `#11-inline-min-content-box-edges`); the function itself (`:15-37`) is, because its `collect_inline_items` call (`:22`) takes the new argument like every other caller. |
| `crates/core/elidex-plugin/src/layout_types/boxes.rs` | `InlineClientRects` (`:106-111`) — the cross-crate contract type. **Semantics not changed by this program**: "per-line client rects … single-line inlines use `LayoutBox.border_box()`" stays true, and PR-1b makes the fallback half *correct* rather than redefining the component. ⚠ Its docstring nonetheless cites **"CSSOM View §5"** for `getClientRects()`, and §5 is *Extensions to the Document Interface* — `getClientRects()` is on `Element`, i.e. §6 (`webref heading cssom-view-1 5` / `6`). The sibling `elidex-dom-api/src/element/layout_query.rs:29` repeats it while `:355` has §6, so the file is internally inconsistent. Same class as §3.1's two swept cites, and **PR-1a's sweep takes it** rather than this row vouching for it. The multi-fragment redefinition belongs to `#11-inline-box-decoration-splits`, which owns the docstring edit too. |
| `crates/core/elidex-render/src/builder/slice.rs` + `walk.rs:296` (render) and `crates/layout/elidex-layout-block/src/block/mod.rs:369-372` (layout) | **Existing `box-decoration-break` implementations, and why this program does not extend them.** `walk.rs:296` reads `style.box_decoration_break`; `slice.rs`'s `break_edges` (`:22`) computes per-fragment slice geometry for **column** fragments and takes `(i, n, wm)` with **no `direction`**, its own docstring saying "the inline-axis edges are never 'at a break'"; `block/mod.rs:369-372` handles `Slice`/`Cloned` for **block** fragments off `block_start_pb`/`block_end_pb`. All three are block-axis, while a line break is an **inline-axis** break whose surviving edge is set by the *parent's* inline progression direction (css-break-3 §5.4) — a different axis and a different direction source, so none generalises as written. ⚠ Two crates, and the dependency runs **render → layout** (`elidex-render/Cargo.toml` depends on `elidex-layout-block`, not the reverse), so a layout-side producer cannot call `break_edges`, which is `pub(super)` in `elidex-render::builder`. `#11-inline-box-decoration-splits` owns the inline case and must first decide **which layer produces** the attribution, since it has both a paint consumer and a CSSOM consumer. |
| `crates/core/elidex-plugin/src/logical.rs` | `LogicalEdges::from_physical` (`:186`) + `WritingModeContext::new` (`:27`) — used at the *point of derivation* (M1), never as a round trip. |
| `crates/layout/elidex-layout-block/src/inline/pack/inline_box.rs` (NEW) | **The marker's own derived facts and the stack that holds them**: push on `InlineBoxStart`, pop-and-emit on `InlineBoxEnd`, the flush-time hook `flush_line` calls (emit + rebase, M4), and M5's `has_inline_axis_edge` plus the inline-start/inline-end sums M3 and M4 consume — one derivation site for everything read off a marker. ⚠ **Two shapes in one module, deliberately**: the stack and its `flush_line` hook are an `impl LinePacker` in a sibling module (the idiom `pack/fragment.rs:10` already uses), while `has_inline_axis_edge` and the sums are **free functions over the marker payload**, because the pre-pack gate at `inline/mod.rs:200` calls them before any `LinePacker` exists (M5). The line-state core (M3) and the baseline promotion (M7) stay with their existing owner in `pack/mod.rs`: cohesion, not `pack/mod.rs`'s line count, decides the split. |
| `crates/layout/elidex-layout-block/src/inline/pack/boxes.rs` | `assign_inline_layout_boxes` (`:48`) — writes `LayoutBox.content` from bounds and, per M4, the three edge fields from `resolve_box_model` on the `ComputedStyle` it already fetches (`:57`), given one new `containing_inline_size` parameter. `EntityBounds` (`:30-41`) and the `InlineClientRects` write (`:102-126`) are **unchanged**: both keep content spans, and the multi-fragment border-area inflation is `#11-inline-box-decoration-splits`'s (M4). |
| `crates/layout/elidex-layout-block/src/inline/tests/mod.rs` | The test helper: a `collect_inline_items` caller (`:17`), an exhaustive `match item` (`:20-23`) that gains marker arms in PR-1a, the `mod decorated_inline;` declaration, and **PR-1a's `setup_inline_test` (`:54`) change** giving the harness a deterministic way to force `any_font == false` (cell 12d). |
| `inline/tests/decorated_inline/{stream,geometry,existence}.rs` (NEW) | The three per-PR test modules §6 routes cells to. |
| `elidex-render` tests | PR-1b's paint assertions for cell 13(c) — the background-colour and border rects that `paint/mod.rs:68`/`:382` derive from `border_box()`. The crate depends on `elidex-layout-block` (`crates/core/elidex-render/Cargo.toml`), so a cell there can run layout. §7 lists the family; §8 makes dispositioning it a PR-1b DoD item. |
| `elidex-layout-block/src/inline/tests/` — **not `elidex-dom-api`** | Cells 17b/17c/17d. ⚠ `elidex-dom-api` has **no** dependency on any layout crate and no `[dev-dependencies]` at all, so a cell there cannot run inline layout — its existing `layout_query.rs` tests hand-insert `LayoutBox` literals, which would assert the marshalling and nothing about M4's producer. Every existing `InlineClientRects` assertion already lives under `elidex-layout-block/src/inline/tests/inline_flow/`, and that is where M4's two channels are jointly observable. §8's PR-1b `getBoundingClientRect` obligation is discharged the same way — against the `LayoutBox` border box the DOM API reads — not by a cell in `elidex-dom-api`. |
| the seam-3 module (NEW, created by the prereq PR) | `layout_inline_context_fragmented`'s reconcile block, moved out of `inline/mod.rs`. §7's `clear_inline_flows` gating lands here, so `:637` is a pre-move coordinate. |
| `elidex-text` (facade over `elidex-shaping`) | `FontDatabase::query` (`crates/text/elidex-shaping/src/database.rs:60`) + `font_metrics` (`:101`) — M7's strut A/D, taken without shaping a string. |

### §5.3 Program slicing

Three shipping PRs, each **behaviour-scoped**, with one owning PR per coupling (§2), plus two
booked slots. Each PR gets its own plan-memo and `/elidex-plan-review`.

* **PR-1a — item stream. Behaviour-neutral.** Owns couplings 2×(all) and 1×2: M1 + M2. Marker
  variants, all consumers updated — **the same set §8's PR-1a DoD enumerates**, stated there once
  rather than in two lists — `containing_inline_size` threaded, every bogus `CSS Box Model L3 §5.3` cite fixed
  (§3.1). The packer gets a **no-op** `match pi` arm (`:528`). The two pre-pack gates are held at
  current behaviour: `items.is_empty()` (`inline/mod.rs:161`) excludes markers, and the `any_font`
  closure gains its **exhaustiveness arm** at `:192-199` returning `false` — behaviour-neutral,
  since a marker is neither `Text` nor `Atomic`. The **outer condition** at `:200` is untouched
  until PR-1c. **Characterization tests for §6 cells 1, 2, 5, 6b, 7–12 and 12d land here
  asserting today's behaviour** — the cells that pin *line suppression*, which is observable with
  `entity`-only markers, since `grep -rn padding
  crates/layout/elidex-layout-block/src/inline/tests/` → no hits means the existing suite pins
  nothing about decorated inlines. Cell 12d's harness change lands here too, with the cell it
  makes constructible. PR-1a opens none.
* **PR-1b — geometry.** Owns couplings 7×2, 3×7 and 7×(intrinsic): M3 + M4 + M8, **plus M5's
  predicate** (`has_inline_axis_edge`), which ships here because M3's shaping break and hang gate
  both call it; PR-1c owns only its *substitution* for the constant. The shared line-occupancy
  core with markers passing `contributes_content = false`, inline-axis advance, shaping break at a
  decorated boundary, real `LayoutBox` edges, content-span rects, intrinsic-size contribution.
  **This is the engine-wide correction**: it changes painted output for every decorated inline
  with content. §6 cells 3, 3b, 4, 6, 10b, 12b, 12c, 12e, 13, 14, 14b, 15, 15b, 16, 16b, 17, 17b, 17c, 17d and
  25 land here — cells 3, 3b, 4 and 6 among them because each asserts a *payload* fact (negative
  margin, the cancelling pair, the percentage basis, the emit/existence split) that M1's PR-1a
  variants, carrying `entity` only, give no channel to observe.
  ⚠ **Scope of "neutral", stated precisely**: PR-1b leaves the **phantom-line predicate**
  unchanged, so no line's *existence* flips. It does **not** leave `line_count` unchanged — the
  advance moves the cursor, so following text reaches the wrap guard (`:690`) earlier and
  content-overflowing paragraphs gain lines. That is the correct consequence of §1.1's
  "inline-axis … respected between inline-level boxes", not a regression, and cell 13's
  displacement half plus cell 15 pin it. The two shapes stay lineless in PR-1b for **two different
  reasons**:
  * **Shape A** reaches the packer (its collapsed-to-`""` `StyledRun` keeps `items` non-empty),
    the shared core sets `on_line`, `finish()` (`:787`) flushes, and the flush takes the
    **discard** arm (`:423`) because `contributes_content` is `false` in PR-1b — no `LineBox`,
    and `current_block_offset` untouched (`:422` is in the commit arm).
  * **Shape B never reaches the packer at all**: PR-1a holds `items.is_empty()`
    (`inline/mod.rs:161`) at "excludes markers" and PR-1b does not flip it, so the early return
    fires first. That is a **held gate — an inert shim that PR-1c removes**, not a property of
    M3/M4.

  PR-1b opens 2 (`#11-inline-box-decoration-splits`, `#11-inline-min-content-box-edges`).
* **PR-1c — existence.** Owns couplings 1×3, 1×5 and 1×4: M5's *flip* + M6 + M7. The clause-3
  predicate per §1.2, the strut baseline, the commit consequence, and the two pre-pack gates
  flipped. **The delta to PR-1b's marker call is `contributes_content` (M5's expression replacing
  the constant `false`) plus `block_advance` (M6);** everything else is already in place. §6
  cells 18–24 land here, plus the flip set §8 states. Closes
  `#11-line-box-decorated-inline-content` with its original DoD — line *and* rectangle — met,
  because PR-1b already made the rectangle real. PR-1c opens none.
* **`#11-inline-box-decoration-splits`** (new slot, **own** deferral): css-inline-3 §2.1 box
  splitting across line boxes and bidi fragments, and **the whole of css-break-3 §5.4** — both `slice` and `clone`, and with them the
  **multi-fragment `getClientRects` channel** (cssom-view-1 §6 step 3): inflating a stored
  content-span fragment into a border area needs the fragment's identity in *content* order, which
  `slice_and_rebase_fragment`'s `retain` (`pack/fragment.rs:69`) destroys for a paged or multicol
  IFC, and it needs the **parent's** inline progression direction, which §5.4 specifies and M1's
  own-direction mapping deliberately does not supply. ⚠ Like every geometry this program writes it is
  first-layout-only until `#11-inline-relayout-box-staleness` lands (§8, §9) — an ordering note,
  **not** a blocking dependency, since that limit applies equally to PR-1b's own edge write and
  the memo accepts it there. Also fragmentation across the marker pair;
  **css-text-3 §5.5's adjacent-soft-wrap rule** (a break next to a decorated boundary lands at the
  box's *margin edge*, which M3's unconditional start-edge advance does not honour — §3's css-text-3 §5.5 row
  whose Step cell reads "adjacent soft wrap opportunity" and §6's note after cell 15 route here);
  and the `group_key` / relpos sub-flow keying that §2's pair 2×6 and §3's CSS 2 §9.4.3 row defer
  here, the field's only reader being the split rule. Why deferred: box continuity across line,
  bidi and sub-flow boundaries is a distinct invariant axis, reachable only once a box has
  geometry; folding it in would put a third axis into PR-1b. Trigger: PR-1c landing.
  Re-eval: 2026-11-01.
* **`#11-inline-root-inline-box`** (new slot, **pre-existing** class): the block container's
  root inline box (css-inline-3 §2), which sets every line box's height floor. Why deferred: it
  changes the height of *every* line box in the engine — verified pre-existing because the
  divergence is observable today on ordinary text (M6's example), with no marker involved, so
  `origin/main` already fails it. Trigger: any line-height correctness work, a compat-survey hit,
  **or any work needing font-fallback provenance** (css-inline-3 §5.3's "only glyphs from fallback
  fonts" strut condition folds here, §8). Re-eval: 2026-11-01.

Own deferrals **per PR** (the policy's unit): PR-1a opens none, PR-1b opens 2, PR-1c opens none,
the seam-3 prereq opens none, the dead-arm prereq opens none. `#11-inline-root-inline-box` and the dead arm's own disposition are pre-existing class. All
within ≤3; `.claude/tools/plan-xcheck.py` cross-checks these against §10's own-tagged rows.

**Rejected**: widening `StyledRun` with edge fields (box-level data on a per-segment
measurement type; N copies for an N-segment span; reaches neither shape per §4.2); reading
`run.entity`'s `ComputedStyle` at pack time (reaches neither shape, same reason). Neither is
rejected on component-lookup cost — `collect.rs:36` uses borrowed component reads in the same
per-child loop, a normal idiom here.

## §6. Edge matrix

**PR-1a characterization (assert today's behaviour; PR-1c's flip set is stated once, in §8's PR-1c DoD):**

1. `padding` alone (inline-axis) — line currently suppressed.
2. `border` alone, plus `border-style: none` + `border-width: 5px`. css-backgrounds-3 §3.2:
   "No border. Color and width are ignored (i.e., the border has width 0)". Applied at
   computed-value time (`elidex-style resolve/box_model/mod.rs:260`).
5. **Block-axis edges only** (`padding-top`, `margin-top`) — **line stays suppressed in
   PR-1c too**, per css-inline-3 §2.3's inline-axis restriction (§1.2). For the *margin* half
   CSS 2 §8.3 independently agrees; it says nothing about padding or border, whose authority is
   css-inline-3 §5.3 + `line-fit-edge: leading` (§1.2). A non-regression cell in every PR.
6b. **The emit predicate's two arms** (M1) — all edges zero ⇒ **no marker in the item stream**;
    block-axis-only (`padding-top`) ⇒ a marker **is** emitted. Asserted on the `Vec<InlineItem>`
    through the test helper's exhaustive `match item` (`inline/tests/mod.rs:20-23`, which PR-1a
    extends), because in PR-1a that is the only channel where a marker is observable at all — the
    packer's `match pi` arm is a no-op. M1's own deliverable, pinned in the PR that ships it.
7. Shape A (decorated inline containing only collapsible white space).
8. Shape B (completely empty decorated inline).
9. `a<span style="padding:10px"> </span>b` — the M2 cell: the space must collapse against its
   neighbours exactly as today.
10. Nested: inner decorated / outer not, and the reverse — both suppressed today. The *stack*
    half is cell 10b, in the PR that ships M4.
11. Two inlines on one line, one decorated.
12. Decorated inline as the only IFC content — the **`items.is_empty()` gate**
    (`inline/mod.rs:161`) only, held in PR-1a and flipped in PR-1c. ⚠ This cell does **not**
    exercise the `any_font` probe: `has_text` (`:190`) is false for a marker-only stream, so the
    whole `if has_text` block (`:191-211`) is skipped.
12d. **The `any_font` early-out with a decorated inline** — text plus a decorated inline in an IFC
    where **no font is usable**: `has_text` is true, `any_font` false, so `:200`'s early return
    fires and the IFC returns `line_count: 0` today, even though clause 3 is font-independent.
    PR-1c adds a marker escape beside the existing `Atomic` one in that **outer condition**
    (`:200`); PR-1a only adds the exhaustiveness arm at `:192-199`.
    ⚠ **Harness**: `setup_inline_test` (`inline/tests/mod.rs:54`) returns `None` when the test
    families are unavailable and every caller early-returns, so the suite currently *skips* rather
    than *exercises* the no-font path. **PR-1a's DoD carries the harness change** — a deterministic
    way to force `any_font == false` — because PR-1a is where this cell is first asserted; without
    it the cell is unconstructible, which is the defect class this memo refuses to ship: a cell no markup can construct.
**PR-1b geometry:**

6. **The emit/existence split's geometric half** (M1/M5) — a block-axis-only marker has cursor
   advance 0 and its line stays phantom. ⚠ On a phantom line the box gets **no rect and no
   paint**: css-inline-3 §2.3 makes the line box "and its in-flow content" non-existent, and the
   discard arm (`:428`) clears the tentative rects. The marker earns its keep only on a line that
   exists for another reason — `<p>text <span style="padding-top:10px">x</span></p>`, where M4
   must fill all four `LayoutBox` sides. Two sub-cells: phantom ⇒ no box; co-resident ⇒ full
   four-sided box.
3. `margin` alone, **including negative** — requires M1's `resolve_box_model` sourcing.
3b. **Cancelling pair** — `<span style="margin-left:-10px;padding-left:10px">`: the inline-start
    components sum to zero but each is non-zero, so css-inline-3 §2.3 keeps the line (M5's
    disjunction). The cursor advance for the same box is zero (M3's sum).
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
    even though the IFC root and the span's parent are both `rtl`. Pins css-writing-modes-4 §6.4's
    "based on the used `direction`… of the box". Cell 12b cannot discriminate this, because there
    the span has no `dir` of its own.

13. `<p>a<span style="padding:10px">text</span>b</p>` — the common case §4.3 is about. Three
    assertions: (a) the span's `LayoutBox` carries real edges and its border box is
    `content + padding` **once**, not twice; (b) **`b` is displaced by 20px** — the
    user-visible half of §1.1's "respected *between* inline-level boxes"; (c) the painted
    background-colour and border rects (`crates/core/elidex-render/src/builder/paint/mod.rs:68`
    and `:382`, both `lb.border_box()`) match the border box. (`:85` is `padding_box()`, the
    background-*image* area, and is not this cell's subject.)
14. **Shaping breaks at the boundary** (§1.4): in `<p>a<span style="padding:1px"></span>b</p>`
    the two same-entity texts must **stop** coalescing into one `InlineFlowRun`. Contrast: with a
    span whose edges are zero on **every** side, no marker is emitted (M1) and they still coalesce.
    ⚠ A `padding-top`-only span **does** emit a marker but fails `has_inline_axis_edge`, so
    coalescing there is the gate's doing, not automatic — cell 14b. Neither contrast holds for
    `vertical-align: super` or `dir`-isolated spans, which css-text-3 §7.3 also breaks on; §3's row
    records those two triggers as unimplemented.
14b. **The gate's negative arm** — `<p>a<span style="padding-top:1px"></span>b</p>`: a marker
    exists, `has_inline_axis_edge` is false, so the runs still coalesce, the hang is untouched
    (cell 16's contrast, and M3's `hang: None` path) and the line stays phantom (cell 6).
15. **The boundary is not a wrap opportunity** (§1.3):
    `<p style="width:100px">aaaaaaaaaaaa<span style="padding:20px"></span>b</p>` — the box does
    not move to the next line at its own edge. Paired: `b` **does** wrap, and earlier than
    today, because the cursor is 40px further along (the PR-1b `line_count` change §5.3 states).
*(A soft wrap opportunity adjacent to a decorated boundary — css-text-3 §5.5's other bullet puts
the break at the box's **margin edge** — is **not** a cell of any PR here. M3 advances the start
edge unconditionally, so that rule is unmet; it is folded into `#11-inline-box-decoration-splits`
(§5.3's Why names it). §3's css-text-3 §5.5 row is marked ✗ accordingly.)*
15b. **A leading marker must not let the first segment soft-wrap** (M3's ⚠): `<p style="width:10px">`
    `<span style="padding-left:20px">verylongword</span></p>` — with `on_line` now armed by the
    marker, the first content segment reaches `:690` with an inflated cursor; the line must not be
    flushed-and-discarded out from under it.
16. **`text-align` with a box after the space** (css-text-3 §4.1.2): `<p style="text-align:center">abc
    <span style="padding:1px"></span></p>` — the space is **no longer at the end of the line**,
    so css-text-3 §4.1.2 **step 3** ("A sequence of collapsible spaces at the end of a line is removed") no
    longer applies to it, and css-text-3 §4.1.2 step 4's hanging — which acts on what step 3 leaves — is not reached
    either. The premise is css-inline-3 §2's "inline-axis margins, borders, and padding are
    respected between inline-level boxes": the marker's advance *is* line content after the space.
    M3 therefore zeroes `current_line_last_hang`, matching the engine's `top_group_is_line_last`
    convention (`inline/pack/mod.rs:264-279`).
16b. **Paired contrast** — the same markup **without** the span: the trailing space must still
    hang, exactly as today. Pins that M3's zeroing is scoped to the marker and does not leak.
17. **A decorated inline whose content wraps** — start marker on line N, end marker on line
    N+1; M4's per-line rebase must yield one rect per line, each with that line's
    `block_start`.
10b. **Nested boxes, stack depth > 1** (M4) — `<p>a<span style="padding:5px">b<span
    style="padding:5px">c</span>d</span>e</p>`: each span gets its own rect and its own
    `LayoutBox` edges, and the inner box's span lies within the outer's. This is why M4 is a
    **stack** rather than one open slot, and no other PR-1b cell has depth > 1.
17b. **Two producers, one fragment** (M4's ⚠): a single-line `<span style="padding:10px">text</span>`
    must yield exactly **one** `line_rects` entry — the persisting arm's per-entity fold
    (`commit_aligned_entity_rects`, `:479-487`) absorbing `place_item`'s rect and the marker's.
17c. **A box opened at a line end** — `<p style="width:100px">aaaaaaaaaa<span style="padding:5px">bbbb</span></p>`:
    the marker opens with zero span on line N, so no partial rect is emitted, but its content-start
    **must** still be rebased to 0 or line N+1's rect comes out inverted (M4).
17d. **`getClientRects`, both channels, one asserting a divergence** (M4's ⚠). Two markups:
    (a) **single fragment** — `<span style="padding:10px">text</span>` on one line stores no
    `InlineClientRects` (`boxes.rs:102`'s `len() > 1` guard), so `getClientRects` takes the
    `border_box()` fallback (`element/layout_query.rs:236`) and returns a **border area** —
    padding + border, never margin, per cssom-view-1 §6 step 3. Correct after PR-1b, and this is
    the cell that pins it. (b) **multiple fragments** —
    `<p style="width:60px">aaa <span style="padding:10px">bbb ccc</span></p>` returns per-line
    **content** spans, i.e. edges missing. **Asserted as an accepted divergence**, in the shape
    cells 23 and 24 use, and routed to `#11-inline-box-decoration-splits` (§5.3 gives the three
    reasons PR-1b cannot discharge it). The two markups therefore disagree after PR-1b, and the
    cell records that rather than hiding it.
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
21. **Co-resident entity on a newly-existing line** — css-inline-3 §2.3's "and its in-flow
    content" means that when the line exists, an undecorated inline sharing it commits its rect
    too (M5).
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

**Test placement** (§9): PR-1a's cells land in `inline/tests/decorated_inline/stream.rs`, PR-1b's
in `…/geometry.rs`, PR-1c's in `…/existence.rs` — a new `decorated_inline` module split per PR
from the start, per [[feedback_touch-time-split-means-while-writing]], rather than one file grown
to hold every cell. None of them go in `tests/text_height/basic.rs` or `relpos_subflow.rs`. The
relpos facet of cells 10/11 lands with the new module too.

## §7. Downstream

* `line_count`, IFC `height`, block cursor — **PR-1b moves all three** (the advance pushes
  following text past the wrap guard earlier; §5.3, cells 13(b)/15). PR-1b changes no line's
  *existence*; that is PR-1c.
* `first_baseline` — moves only on a glyphless line (M7); cell 22 asserts both directions.
* `entity_bounds` / `getClientRects` — **split by channel** after M4. Every decorated inline gets
  a real `LayoutBox` border box, so `getBoundingClientRect` and the *single-fragment*
  `getClientRects` fallback are correct in PR-1b. A **multi-fragment** inline still answers
  `getClientRects` from stored **content** spans (cell 17d(b)), a disclosed divergence routed to
  `#11-inline-box-decoration-splits`. ⚠ And its `LayoutBox` is the union of every line's content
  span, so `border_box()` expands that union on all four sides — inline-start padding appears at
  the last line and inline-end padding at the first. Under-inflated in one channel,
  over-inflated in the other; both are the same missing per-fragment attribution, and both go to
  that slot. PR-1c adds the co-resident commit (cell 21).
* **Painted output** — PR-1b changes background and border rendering for **every** decorated
  inline (`crates/core/elidex-render/src/builder/paint/mod.rs:68` background colour, `:382`
  borders; `:85` is the background-image area and follows the padding box). Largest delta in the
  program; cell 13 is its pin.
* `last_placed_entity` / persisted run shape — deliberately changed by M3 (§1.4); cell 14.
* `current_line_last_hang` — **zeroed at a marker with an inline-axis edge** (M3), left alone
  otherwise; cells 16 and 14b.
* **Every reader of an inline `LayoutBox`'s edges.** M4 flips `padding`/`border`/`margin` on inline
  entities from a hard-coded `EdgeSizes::default()` to real values, so consumers that were reading a
  constant now read data. Enumerated by `grep -rn "border_box()\|padding_box()\|margin_box()" crates/`
  — re-run it rather than trusting this list — and dispositioned by family:
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
    `inline/atomic.rs`, `elidex-css/src/page.rs` — block/atomic/abspos paths, unreached for a
    non-replaced inline. `elidex-ecs/src/dom/geometry.rs` likewise, though for a different reason:
    `union_border_boxes` has no non-test consumer yet.
  None is wrong-after-M4 **for a single-fragment inline**; for a multi-fragment one every reader
  in this list inherits the over-inflated union above. The point is that PR-1b's DoD must reach
  past paint, which §8 says.
* Static positions of following abspos (`inline/pack/mod.rs:97`).
* `current_line_runs` / `flow_lines` — markers are never `FlowMember`s, so member-less buckets
  keep being dropped by design; no ordinal dependency (§2's non-invariant).
* `clear_inline_flows` probe gating — a decoration-only IFC stops taking the early return at
  `inline/mod.rs:161` (unconditional clear) and starts taking the full path (`:637` — a pre-move
  coordinate; that line lives in the seam-3 module after the prereq PR). **PR-1c**, per §5.3.
* Fragmentation (`inline/pack/fragment.rs`) and `ColumnFlowSlice`.

## §8. Definition of done

**PR-1a** (item stream, behaviour-neutral): both marker variants reach `pack/items.rs`; every
exhaustive match handles both (`inline/whitespace.rs:41`, `inline/mod.rs:192-199`,
`inline/pack/items.rs:67`, `inline/tests/mod.rs:20-23` — `atomic.rs:39`, `measure.rs` and
`collect.rs:181` are `if let` and need none); **the payload exactly as M1 specifies it, not
restated here**; `containing_inline_size` threaded through every `collect_inline_items` caller
(§5.2 enumerates them from the grep that defines the set); the two pre-pack gates hold current
behaviour; **every hit of §3.1's concept grep** (the bogus `CSS Box Model L3 §5.3` class) corrected; new variants carry docstring citations
to their §3 rows; **`setup_inline_test` (`inline/tests/mod.rs:54`) gains a deterministic way to
force `any_font == false`**, without which cell 12d is unconstructible; cells 1, 2, 5, 6b, 7–12 and 12d land as
characterization tests; zero behaviour change. **Dead-field rule**: fields
whose first reader is a later PR are added by that PR — M6/M7's `line_height` and font identity by
**PR-1c**, `group_key` by `#11-inline-box-decoration-splits` — so nothing ships unread and no
`#[allow(dead_code)]` is needed. The rule reaches M1's own payload too: PR-1a's variants carry
**`entity` only**, since the three `EdgeSizes` and the `WritingModeContext` have no PR-1a reader —
the emit test resolves them at collect time and discards them, and the packer's `match pi` arm is a
no-op. They arrive in **PR-1b** with M3/M4/M5.

**PR-1b** (geometry): cells 3, 3b, 4, 6, 10b, 12b, 12c, 12e, 13, 14, 14b, 15, 15b, 16, 16b, 17,
17b, 17c, 17d and 25 — ⚠ **all of them assert a first-layout property**, because `assign_inline_layout_boxes`
skips any entity that already carries a `LayoutBox` (`boxes.rs:60-62`) and nothing anywhere removes
one (`grep -rnE "remove(_one)?::<(elidex_plugin::)?LayoutBox>" crates/` → one hit,
`elidex-js/src/vm/tests/tests_resize_observer.rs:293`, test-only), so no geometry
this PR writes is refreshed on a relayout. That is total and pre-existing — the same skip already
freezes `LayoutBox.content` — and `#11-inline-relayout-box-staleness` owns it (§9); the DoD states
it rather than implying a JS-observable-after-mutation guarantee it cannot give. **§7's full
`LayoutBox`-edge reader list dispositioned, not just paint** (a `getBoundingClientRect` assertion
included, made through the layout-level channel §5.2 names, not in `elidex-dom-api`); no
double-counted edges on either side; the `inline/pack/mod.rs:726` shaping citation corrected to
css-text-3 §5.5 + §7.3 (§3.1); the `InlineClientRects` write path is **untouched** — PR-1b's
whole CSSOM effect is that the `border_box()` fallback becomes correct once `LayoutBox` carries
real edges (cell 17d(a)), so the cssom-view-1 §6 and css-break-3 §5.4 citation obligations travel
with the derivation to `#11-inline-box-decoration-splits`; `note_line_occupancy`,
`pack/inline_box.rs` and M8's contribution carry docstring citations to their §3 rows.

**PR-1c** (existence): cells 18–24, plus the flip set — **cells 1, 2, 7, 8, 10, 11, 12 and 12d flip; 5, 6b and 9 do not**; this is the one normative statement of it — every §7
consumer checked; the ⚠ caveat at `inline/pack/mod.rs:110` retires, naming the root-inline-box
divergence and pointing at `#11-inline-root-inline-box`; M7's promotion carries docstring
citations to their §3 rows; slot closes.

**This memo and the branch's tooling files** — enumerate them with
`git diff --name-only 154bac3f..HEAD` rather than from a list here, which this branch's own next
commit invalidates — ship with the **seam-3 prereq PR**, the first of the five to open *and* to
land, per
[[feedback_plan-memo-author-in-worktree]]. No later PR re-ships them.

**Seam-3 prereq PR**: `layout_inline_context_fragmented`'s reconcile block (§9's measured range)
moves to its own module, **byte-identical modulo the extracted signature**. The block reads a set
of enclosing-fn bindings that become parameters — that is the **only** permitted edit. The set is
**not enumerated here**; the PR's own plan-memo enumerates it against its actual base, because an
enumeration written here is measured against a base the PR does not have. Proof obligation: a diff
of the moved body against the same block extracted from that base differs only in those bindings.
`cargo test -p elidex-layout-block --all-features` green with no test touched;
`#[allow(clippy::too_many_lines)]` on the residue (`inline/mod.rs:140`) re-evaluated. Ledger
actions in §10.

**Dead-arm prereq PR**: the whole surface the reachability argument kills, not half of it —
`flush_line`'s `else` arm, the `persist_candidate` branch, and everything downstream of
`flow_align` being unconditionally `Some`: the `Option` wrapper on the field and on
`LinePacker::new`'s parameter, the `if let Some(fa)` and `is_some()` guards, and the comments that
explain the two-path model. §5.2's row for this PR enumerates the surface across both files.
Removing only the arm would leave the same dead code half-alive, which is what CLAUDE.md's rule
forbids; `#11-inline-align-clientrects-nonpersist-path` closes. Stacked beside the seam-3 split,
not folded into it, so that PR's byte-identical criterion stays provable.

**Ordering: land the seam-3 PR first, because the cost is asymmetric — not because the PRs are
coupled.** They are not: the dead arm is in `pack/mod.rs`, and its `inline/mod.rs` half (`:239`,
`:240-251`, `:322` and their comments, all ≤ `:330`) lies entirely **above** seam 3's `:413-639`
and touches nothing inside it (`awk 'NR>=413 && NR<=639' inline/mod.rs | grep -c
"persist_candidate\|flow_align"` → 0). A *coordinate* shift is no ground for an order, since the
front matter already requires every PR to re-anchor against its actual base. The ground that does
survive is that the two directions cost differently: **seam-3-first costs the dead-arm PR nothing**
(deleting `:413-639` renumbers nothing *above* it, and every dead-arm `inline/mod.rs` site is
≤ `:330`; its `pack/mod.rs` half is in a file seam 3 never touches), while **dead-arm-first forces the seam-3 PR to re-measure the one criterion in this
program that *is* a byte range** — "byte-identical modulo the extracted signature", already
approved at a measured range. Free in one direction, not the other. Both branch from `main`;
whichever lands second takes `git merge origin/main` (⚠ **not** `rebase` — an opened PR branch
cannot be rebased without a force-push, which `~/.claude/hooks/` denies). **Branch topology**: the
two prereqs are siblings off `main`; PR-1a branches off `main` after both have *landed*; PR-1b
stacks on PR-1a and PR-1c on PR-1b, since each consumes the previous one's mechanism — and because
CLAUDE.md mandates **squash** merge, a stacked PR's base commits are rewritten when its parent
lands, so each re-cuts from `origin/main` once its parent is in rather than rebasing.

**Explicitly not covered, recorded rather than dropped**: css-inline-3 §5.3's "or if it contains
only glyphs from fallback fonts" strut condition — elidex has no fallback-provenance signal.
Folded into `#11-inline-root-inline-box`, whose trigger in §5.3 names font-fallback provenance
explicitly so the fold can surface.

## §9. Out of scope, with disposition

* **`#11-inline-fragmented-fn-decomposition` — trigger fires, and this program honours it.** The
  slot's trigger is "the next change that touches `layout_inline_context_fragmented`'s body";
  PR-1a touches `:154`/`:161`/`:192-199` and PR-1c `:200` and the persist block, the slot memo's
  **seam 3**. **Disposition**: a standalone prereq split PR at seam 3, before PR-1a.
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
  letting this drift"). That claim is not adopted here, and the disagreement is recorded rather
  than resolved silently: it is the counterweight to narrowing the prereq PR to seam 3 alone, so if
  it is right the residue is under-cut. The disposition stands on the fired touch disjunct either
  way; what changes is whether seams 1 and 2 should also be discharged now, and §10's successor
  slot is where that is booked.
  **Scope**: the prereq PR discharges **seam 3 only** — `mod.rs:413-639` on `154bac3f` (the slot
  memo's `:411-637` predates #497's two-line shift). Seams 1 (`:266-303`) and 2 (`:388-411`),
  both re-measured on `154bac3f`, remain, so §10 records a **partial** close. PR-1a's own touch
  sites lie in the residue, not in seam 3.
  **Cold gate** ([[feedback_split-on-touch-prereq-workflow]]): re-run `gh pr list --state open` and
  check each for `elidex-layout-block` before the prereq PR opens. As of the last run no **open PR**
  touched it; note `origin/layout-css2-cite-sweep` is a live remote branch that does, but it is
  #497's unsquashed history, already in `main`.
  **Its own gate**: `/elidex-plan-review`, like every PR here — CLAUDE.md makes that a rule, not a
  judgment, and §10 routes non-mechanical ledger actions to this PR.
* **File growth**: measure with `wc -l` at each PR rather than against a number written here, which
  this program's own prereq PRs invalidate. Per §5.2 the **open-box stack** goes to a new
  `pack/inline_box.rs`; M3's line-state core and M7's promotion stay in `pack/mod.rs`; new tests to
  a new module (§6). `inline/mod.rs`'s growth across the three PRs is a signature change, two gate
  predicates and the persist-block touch — no new seam decision needed. `pack/inline_box.rs` is the one **non-test** source file this program authors (the seam-3 module
  is a move, and §6 already applies the band to the new test modules), so
  [[feedback_touch-time-split-means-while-writing]]'s 700–800 band applies to it while it is being
  written, not afterwards — it accumulates M4's stack, the flush hook, M5 and the sums across
  PR-1b. `inline/mod.rs` and `pack/mod.rs` are re-checked against the ~1000-line convention
  **at every PR**, on their
  then-current size — `pack/mod.rs` takes its largest add in PR-1b (`note_line_occupancy`, the
  `flush_line` hook call, the reset-block growth), so a first check at PR-1c is the shape
  [[feedback_touch-time-split-means-while-writing]] calls already-too-late.
* **`#11-inline-relayout-box-staleness`** (`inline/pack/boxes.rs:96`) — pre-existing, and the
  statement PR-1b owes it is a **write-path** one, not a cosmetic widening. M4 adds a *derived*
  field whose SoT is the entity's `ComputedStyle` padding/border/margin; every mutation path to
  that SoT (`element.style.*`, a `style`/`class` attribute change, a stylesheet insertion) triggers
  restyle + relayout, and on relayout `assign_inline_layout_boxes` `continue`s for any entity that
  already carries a `LayoutBox` (`boxes.rs:60-62`) while nothing anywhere removes one. So the new
  write has **no reconciliation hook**. The staleness is **total and pre-existing** — the same skip
  already freezes `LayoutBox.content`, so every inline geometry assertion in the engine is already
  first-layout-scoped — which is why PR-1b does not own the refresh and §8 instead scopes its cells
  explicitly. What M4 changes is *which fields* go stale, not *whether* they do. The fix belongs
  where the skip is: a `layout_generation` comparison (the component already carries one,
  `boxes.rs:86`) instead of a presence test, which is this slot's work.
  (`#11-inline-align-clientrects-nonpersist-path` was ledger-marked to fold into terminal-Z
  C-3/C-4 alongside it; the dead-arm prereq **closes** it instead, so that pairing no longer holds
  and its SoT entry needs the same correction.)
* **`#11-layoutbox-absence-unreachable`** (#488, registered in the SoT) — no longer relevant, and
  §10 carries the disposition row: M5 keeps no entity's box withheld, so no truthful box-absent
  signal is required.
* **Intrinsic sizing** — split by pass, per M8: the max-content contribution is **in PR-1b**; the
  min-content one is slotted (`#11-inline-min-content-box-edges`) because that pass has no
  accumulator to attach edges to. The intrinsic passes pass `0.0` as the containing inline size
  (§5.2) — the standard treatment for percentages.
* **`#11-css2-spec-label-normalisation`** — this memo cites css-inline-3 for the model, so its
  remaining CSS 2 cites are §8.3, §9.4.2, §9.4.3, §10.8.1 and §16.6.1 (`grep -o 'CSS 2 §[0-9.]*'
  <memo> | sort -u`; each section↔title pair verified with `webref heading CSS2 <n>`). Adjacent
  `CSS 2.1 §` lines in touched files are left alone; the slot requires a single cross-crate commit.
* **css-inline-3 §5.3's Quirks-Mode rule** — "any inline box fragment that has zero borders and
  padding and that does not directly contain text or preserved white space is ignored when sizing
  the line box" — is the quirks analogue of clause 3 and is unimplemented. elidex has no
  quirks-mode layout switch at all, so this is not a gap this umbrella can carve; recorded because
  the §3 rows citing css-inline-3 §5.3 do not cover it.
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
  `.claude/tools/` and are invoked by hand; nothing in `scripts/trip-wires.sh`, `mise.toml`,
  `.github/workflows/ci.yml` or `.claude/skills/**` references them. Per
  [[feedback_every-triggered-pr-must-be-on-the-loop]] a checker reachable only by remembering it
  is the habit that failed six rounds with a tool attached. **Disposition**: they are **not**
  generic yet — `plan-xcheck.py` hard-codes `PR-1[abc]`, a memo-specific superseded-range dict and
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
  checkers in `elidex-plan-review`'s Step 1.5 or the ungated `trip-wires` job, and extend
  `preflight.py`'s `SPEC_LABEL_REVERSE` to CSS-module labels — without that last one the preflight
  citation gate is vacuous for any CSS-module plan (§3). ⚠ Until it is done, **this memo's own
  remaining rounds depend on running the checkers by hand** — the habit the task exists to end.
  Trigger: **this memo's next plan-review round**. Re-eval: 2026-11-01.

## §10. Slot ledger actions at landing

| Action | PR |
|---|---|
| Register this umbrella's slot in `project_open-defer-slots.md` (the SoT per MEMORY.md), together with the two other unregistered Layout-lane slots — `#11-css2-spec-label-normalisation` (a #497 carve, **pre-existing** class) and `#11-inline-fragmented-fn-decomposition` (carved from #495, **pre-existing** class). This umbrella's own slot is **pre-existing** class too: Codex opened it on #497, not this program. Each of the three gets **re-eval 2026-11-01** — a slot registered against a PR that may slip needs a date, not only a trigger. | seam-3 prereq PR |
| Register **and** close `#11-inline-fragmented-fn-decomposition` in one row — it is registered as closed-on-landing, not registered then closed — **as a partial close**, naming the seams §9 measures as still open, in the successor slot `#11-inline-fragmented-fn-seams-1-2` (**pre-existing** class — the seams predate this umbrella; Why: the prereq PR discharges seam 3 only; trigger: **either** the first change after **any** of this umbrella's five PRs that touches the residue — self-exempted for all five PRs, on **two different grounds**: the two prereq PRs *are* the decomposition work, so counting them would make the successor fire on its own predecessor; and PR-1a/1b/1c are the feature work whose touch sites (§5.2's `inline/mod.rs` row) are enumerated and reviewed *here*, which is what the source slot's disjunct exists to force — a slot cannot demand a review it is already receiving — **or** `inline/mod.rs` growing back toward 1000 lines, the source slot's second disjunct, which has not fired and which this program's own growth could trip; re-eval 2026-11-01). | seam-3 prereq PR |
| Open `#11-inline-box-decoration-splits` (own) with the Why / trigger / date in §5.3 | PR-1b |
| Open `#11-inline-min-content-box-edges` (own) with the Why / trigger / date in M8 | PR-1b |
| Close `#11-inline-align-clientrects-nonpersist-path` — the arm it books work against is deleted | dead-arm prereq PR |
| Note on `#11-layoutbox-absence-unreachable` (#488) that M4/M5 remove this umbrella's dependence on a truthful box-absent signal | PR-1b |
| Open `#11-inline-root-inline-box` (pre-existing) with the Why / trigger / date in §5.3 | PR-1c |
| **Close `#11-line-box-decorated-inline-content`** — §5.3 and §8 both assert it, and until now no ledger row carried it | PR-1c |
| The plan-checker standing maintenance note is **already written** into `.claude/skills/elidex-plan-review/SKILL.md` on this branch, not booked for landing — its trigger ("the next plan-review round that runs them by hand") fires every round, so a landing-scoped row would leave it unowned in exactly the window it matters. **Not a `#11-` slot** — skill infra, per that file's own slot-fit precedent. This row records it; the seam-3 prereq PR ships it with the branch's other two tooling files | seam-3 prereq PR |
| Split the joint "fold into terminal-Z C-3/C-4" parenthetical shared by `#11-inline-align-clientrects-nonpersist-path` and `#11-inline-relayout-box-staleness` — this PR closes the first, so the pairing stops holding **here**, and leaving it to a later PR would strand the SoT asserting a fold against a closed slot | dead-arm prereq PR |
| Enrich `#11-inline-relayout-box-staleness`'s SoT entry with M4's write-path statement (§9), and record that `#11-inline-box-decoration-splits` now depends on it (§5.3) | PR-1b |
| Rewrite `project_line-box-decorated-inline-content.md`, `MEMORY.md`'s Layout-lane entry and `active-lane-detail.md`, all of which still carry a superseded framing of this slot | seam-3 prereq PR |
| Record the successor program the close hands off to: `#11-inline-box-decoration-splits` and `#11-inline-min-content-box-edges` **arm at PR-1c landing** (their stated trigger), and `#11-inline-fragmented-fn-seams-1-2` does **not** — neither of its disjuncts fires at the close, so it stays dormant until a later touch or a size trip. The Layout lane's next task is a choice between the first two, not an empty slot | PR-1c |
