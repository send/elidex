# LayoutBox / BoxModel reader inventory — terminal-Z C-3a audit

**Status**: the durable, classified reader inventory C-3a produces (design SoT
`docs/plans/2026-07-terminal-z-c3a-seam-and-audit-plan.md` §4; impl plan
`docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 D4). Downstream slices C-3b–e cite this to
pin each consumer's per-fragment contract; C-4's "zero `LayoutBox` reads outside producers" gate
is checked against it. Produced 2026-07-18 against branch `terminal-z-c3a-impl`.

## Method — the grep is triage, the trip-wire is the proof

This inventory was produced by a **human first-pass grep + classify**, NOT by a grep-completeness
claim. Per the design memo §4 (Codex R1-T6 / R2-U2 / re-gate-V5 / R5-W2), a `git grep` is
structurally non-exhaustive — it cannot follow import aliases, re-exports, or generic
`T: BoxModel` bounds — so it **cannot be the exhaustiveness gate**. The first pass was:

```
git grep -nw LayoutBox -- 'crates/**/*.rs'      # 710 token occurrences, 104 files
git grep -nw BoxModel  -- 'crates/**/*.rs'       # BoxModel occurrences
```

then refined with reader-shape greps (`get::<&(mut )?LayoutBox>`, `query::<(..LayoutBox..)>`,
`fn f(..: &(mut )?LayoutBox)`, `&dyn BoxModel`, `From<&LayoutBox>` / `BoxFragment::from`, plugin
type-defs, and `insert_one`/`&mut` producer-write sites), and classified against the live code.

**Exhaustiveness is proven by the D4 trip-wire (impl §3), not by this document.** The trip-wire
(`.claude/tools/layout-box-reader-trip-wire.sh`, run both by the local `mise run trip-wires`
⊂ `mise run ci` **and** by the ungated `trip-wires` job in `.github/workflows/ci.yml` on every
push and PR — so a lane that adds a reader cannot merge green, see the impl plan §6) greps
bare `git grep -nw LayoutBox` + `git grep -nw BoxModel` (the broadened `-nw BoxModel` grep catches
generic `T: BoxModel` bounds the narrow `dyn|impl` grep misses), diffs live reads against the
committed allowlist sibling of this doc (`.claude/tools/layout-box-reader-allowlist.tsv`), adds a
`LayoutBox`/`BoxModel`-alias name-introduction ban, and **exits non-zero on any read not in the
allowlist**. A reference shape this grep-based first pass missed is caught there; a new reader in a
later slice forces a classification before it can land. This document is the human record; the
trip-wire is the machine gate.

⚠ **What "exhaustive" does and does not mean here** (do not read the sentence above more strongly
than the gate supports — the design memo §4 warned that a grep cannot prove exhaustiveness, and the
four wires narrow that gap rather than closing it):

1. **Token-carrying reads** — wire #1 sees them all, and wire #2 keeps them token-carrying by banning
   the alias forms (with a positive control, so the wire cannot silently stop firing).
2. **Token-LESS reads through a `LayoutBox`-typed BINDING** (struct field or fn param) — the
   *declaration* carries the token, so wire #1 does force a row + a classification for a new binding;
   the *reads through it* carry none and are enumerated **by hand** in the tables below
   (`LayoutOutcome.layout_box` → `builder/mod.rs:367-372`; `InlineRunContext.lb` → six reads at
   `builder/inline.rs:268…456`; `PageFragment.layout_box` → producer-internal; the `&LayoutBox` /
   `&dyn BoxModel` helper params in `paint/mod.rs`, `form.rs`, `transform.rs`). This is the weakest
   link and it is **not machine-checked** — **slot `#11-layoutbox-field-typed-reader-coverage`** owns
   closing it, and C-4 must not read a green gate as covering it.
   ⚠ An earlier revision of this doc claimed a "wire #5" *bounded* this family. It did not — its
   regex missed `&'a LayoutBox` (so `inline.rs:73` went unseen) and a new field in an
   already-listed file passed every wire. The wire was withdrawn rather than left overclaiming.
3. **Duplicate-content reads in one file** — the gate keys on unique `(path, content)`, so a *second*
   line identical to an already-allowlisted one in the same file does not fire (see "Counting basis").
   Per-site precision is this document's job, not the gate's.
   ⚠ Codex PR#488 R4 read this as a defect: a second `dom.world().get::<&LayoutBox>(entity)` in a
   *different function* of the same file collapses to the same key, so it lands unclassified while
   wire #1 stays green. That mechanism is real and is **accepted deliberately**, not overlooked — the
   dedup is what lets a reader MOVE within a file without churning the allowlist, and the residual is
   narrow: the new site is by construction the same reader-shape in the same file as one already
   triaged, so it inherits that row's migration semantics. The dangerous direction — a **novel**
   reader shape, or a reader in a new file — always fires. Widening the key to `(path, line, content)`
   would trade this for allowlist churn on every edit above a reader, which is the failure mode that
   makes a gate get ignored. Recorded here so C-4 does not read "exhaustive" as "per-site exhaustive".
4. **Token-hiding macros** — wire #3 guards against a new `macro_rules!` in a reader-token file;
   verifiably none today.

**Scope of "reader".** Tests (`crates/**/tests/**`, `*test*.rs`, in-file `#[cfg(test)]` modules)
are excluded from the CONSUMER classification. Every NON-test line carrying the token is accounted
for below as one of: `pending-migration:<slice>` (a consumer read C-3b–e must migrate), `seam` (the
C-3a N=1 fallback, a single site), `producer` (an `elidex-plugin`/`elidex-layout-*` write or in-layout
producer read), `type-def` (the `LayoutBox`/`BoxModel`/`BoxFragment` definitions + trait impls), or
`import` (a bare `use` line — it carries the token but reads nothing; safe to exclude from the
consumer set because wire #2 bans the aliased forms, so a non-aliased import always leaves the token
at each real use site).
Producer-crate test-only reads are noted separately in the appendix.

## Classification legend

- **reader-kind**: `get<&LayoutBox>` / `get<&mut LayoutBox>` / `query<(..)>` /
  `helper-param fn f(lb:&LayoutBox)` / `&dyn BoxModel` (trait-erased) / `From<&LayoutBox>` /
  `field-read`.
- **CLASSIFICATION** — this list is the **prose definition**; the machine SoT is `$CLASSES` in
  `.claude/tools/layout-box-reader-trip-wire.sh`, which **wire #4 enforces** against column 1 of every
  allowlist row (so the two cannot drift apart again, and `--regenerate`'s `UNCLASSIFIED` placeholder
  can no longer ride into a green run):
  `producer` | `seam` | `pending-migration:<slice>` | `type-def` | `import` | `test`.
  The last two exist because the allowlist keys on **every token-carrying line**, not only reads: a
  bare `use …{LayoutBox}` line is `import`, and a token line inside an inline `#[cfg(test)] mod tests`
  of a production-named file is `test`. Neither is a consumer read, and neither gates C-4 — only
  `pending-migration:<slice>` rows do.
- **slice** (pending-migration only): C-3b (CSSOM geometry, `elidex-dom-api`) / C-3c (hit-test +
  a11y + flex·grid·inline baseline, in-layout) / C-3d (observers IO·RO + shell scroll·nav) / C-3e
  (render residual: inline-text anchor, paged-gen gate, paint/form helpers) / **C-4** (a `LayoutBox`
  WRITE that no C-3 consumer slice touches — only the delete itself forces it; see the
  `test_helpers.rs` note below).

---

## Per-crate classified reader tables

### `elidex-dom-api` — CSSOM-View geometry → **C-3b** (4 reader sites, one family)

All CSSOM-View handlers in `element/layout_query.rs` source their box geometry from **four**
`LayoutBox` reads: two helpers (`get_border_box`, `get_padding_box`) and two direct field-reads
(`clientTop`/`clientLeft`). The handler bodies (gBCR :26, getClientRects :237, offsetW/H :68/:75,
offsetT/L via `offset_from_parent` :384, clientW/H :122/:129, scrollW/H :161/:168, scrollIntoView
:276) compose frames **above** these reads.

| file:line | reader-kind | feeds | CLASS | slice |
|---|---|---|---|---|
| `element/layout_query.rs:338` `get_border_box` | get<&LayoutBox> → `.border_box()` | gBCR, getClientRects fallback, offsetW/H, offsetT/L, scrollIntoView | pending-migration | C-3b |
| `element/layout_query.rs:348` `get_padding_box` | get<&LayoutBox> → `.padding_box()` | clientW/H, scrollW/H | pending-migration | C-3b |
| `element/layout_query.rs:138` `clientTop` | get<&LayoutBox>, field-read `lb.border.top` | clientTop | pending-migration | C-3b |
| `element/layout_query.rs:148` `clientLeft` | get<&LayoutBox>, field-read `lb.border.left` | clientLeft | pending-migration | C-3b |

(`getClientRects` also reads `InlineClientRects` at :219 — the *line* source — making it two-source,
seed 3. `offsetParent` :108 and `scrollTop/Left` :175/:185 read no `LayoutBox`; not listed.)

### `elidex-a11y` + `elidex-layout` (hit-test) + flex·grid·inline baseline → **C-3c** (6 readers)

| file:line | reader-kind | reads | phase | CLASS | slice |
|---|---|---|---|---|---|
| `elidex-a11y/src/tree.rs:122` | get<&LayoutBox> → `.border_box()` | a11y node bounds | screen-post-layout | pending-migration | C-3c |
| `elidex-layout/src/hit_test.rs:130` | get<&LayoutBox> → `.border_box()` (+ self-composed transform) | point-in-box hit test | screen-post-layout | pending-migration | C-3c |
| `elidex-layout-flex/src/lib.rs:474` | get<&LayoutBox> → `.first_baseline`,`.content.origin.y` | container baseline fallback | **in-layout** | pending-migration | C-3c |
| `elidex-layout-flex/src/baseline.rs:18` `read_item_baselines` | get<&LayoutBox> → `.padding`,`.border`,`.first_baseline` of the CHILD | per-item baseline offset for `align-items: baseline` | **in-layout** | pending-migration | C-3c |
| `elidex-layout-grid/src/position.rs:444` | get<&LayoutBox> → `.first_baseline`,`.content.origin.y` | grid baseline fallback | **in-layout** | pending-migration | C-3c |
| `elidex-layout-block/src/inline/pack/mod.rs:613` | get<&LayoutBox> → atomic `.first_baseline`+edges | inline-run baseline from atomic | **in-layout** | pending-migration | C-3c |

⚠ **`flex/baseline.rs:18` was first classified `producer`** (Codex PR#488 R1) because it lives in a
layout crate — but the `producer` clause is "a write, or an in-layout producer read *of geometry this
algorithm itself produced*", and this reads **another entity's** committed box, exactly like its sibling
`flex/lib.rs:474` two rows up. `producer` is the class that SURVIVES C-4, so the delete-enabling gate
would have gone green over a live read that then silently loses item baselines. It is the third instance
of this same mislabel (after `render/builder/inline.rs:73` and `builder/mod.rs:482`/`:998`): being in a
producer crate is not the test — reading someone else's committed geometry is. A sweep of the 31 rows
classified `producer` *before* this reclassification found no fourth, so the live allowlist holds **30**:
the remainder are writes (`set_layout_box` / `layout_box_mut` / literals), local result-holders, or reads of geometry
the same algorithm just produced (recorded in section (d)).

⚠ The four baseline readers are in **producer crates** and run **in-layout** — `box_fragments` is
by contract unusable mid-pass (memo §2). C-3c's disposition is **not** "route through the seam":
per C-4 gate item 2b / hand-off row 2 (`#11-in-layout-probe-visible-geometry`), they either **keep
live `LayoutBox`** or get a **probe-visible accessor**. They are listed pending-migration/C-3c
because a downstream slice *owns the decision*, not because they migrate onto `box_fragments`. See
note (d).

### `elidex-js` (observer DI closures) + `elidex-shell` → **C-3d** (7 readers)

The IntersectionObserver / ResizeObserver registries live in `elidex-api-observers` (callability
`dom-api=0`), which reads geometry only through injected closures (`RectProvider`/`SizeProvider`,
`intersection/mod.rs:17`, `resize.rs:19` — **no `LayoutBox` token**). The live `LayoutBox` reads
are in the **`elidex-js` host closures** — this is the current live observer-geometry reader
(memo §6.2 phrasing; do NOT say "api-observers is untouched" — C-3d option (c) may add the acyclic
`api-observers → dom-api` edge).

| file:line | reader-kind | reads | phase | CLASS | slice |
|---|---|---|---|---|---|
| `elidex-js/.../host/intersection_observer.rs:489` (`rect_fn`) | get<&LayoutBox> → `.border_box()` | IO target rect (doc-space) | screen-post-layout | pending-migration | C-3d |
| `elidex-js/.../host/resize_observer.rs:405` (`size_fn`) | get<&LayoutBox> → `.content_rect_local()`, `.border_box().size` | RO contentRect + borderBoxSize | screen-post-layout | pending-migration | C-3d |
| `elidex-shell/.../content/scroll.rs:137` `compute_content_extent` | **query<(Entity,(&LayoutBox,&ComputedStyle))>** → `.border_box()` | scroll-extent (max far edge, skip display:none) | screen-post-layout | pending-migration | C-3d |
| `elidex-shell/.../content/scroll.rs:236` `scroll_offset_for_fragment` | get<&LayoutBox> → `.border_box()` | URL-fragment nav scroll target | screen-post-layout | pending-migration | C-3d |
| `elidex-shell/.../content/event_handlers.rs:374` `update_scroll_offset` | get<&LayoutBox> → `.content.size.width` | single-line text caret tracking | screen-post-layout | pending-migration | C-3d |
| `elidex-shell/.../content/event_handlers.rs:807` `try_route_click_to_iframe` | get<&LayoutBox> → `.content.origin` | iframe click-routing offset (parent→child point translation) | screen-post-layout | pending-migration | C-3d |
| `elidex-shell/.../content/iframe/lifecycle.rs:267` `check_lazy_iframes` | get<&LayoutBox> → `.content.origin`/`.size` | lazy-iframe viewport-proximity (200px margin) | screen-post-layout | pending-migration | C-3d |

### `elidex-render` — render residual → **C-3e** (26 reader sites, 18 rows)

Render is a **consumer** (paints from geometry), not a producer. It **already** consumes the
fragment store for consumable multicol on the **screen** path (C-1, `walk.rs:207–218`); the
`LayoutBox` reads below are the N=1 fallback source + per-fragment paint helpers + two non-geometry
reads. ⚠ Render runs on **both** the screen path (`!ctx.paged`) and the **paged** path
(`ctx.paged`, page-relative geometry) — so its readers span `screen-post-layout` **and**
`paged-post-layout` (axis 2). The paged path **cannot** use `screen_geometry` (the phase guard
fails on a paged store, memo §2 I-phase fact 3); the walk comment at `walk.rs:190–192` already keeps
the per-page `LayoutBox` path "until paged×multicol store unification" — so C-3e/C-4 keeps
`LayoutBox` for the paged path (or lands the paged store).

| file:line | reader-kind | reads / role | note | CLASS | slice |
|---|---|---|---|---|---|
| `builder/walk.rs:183-188` `lb_owned` | get<&LayoutBox> → `Option<LayoutBox>` clone | N=1 fragment source for the unified walk | axis 7: cloned owned (no store handle held across child recursion) | pending-migration | C-3e |
| `builder/walk.rs:330` `frag` | &dyn BoxModel | dispatch `single_box ? lb : &store_frags[i]` | geometry-source-agnostic loop body | pending-migration | C-3e |
| `builder/walk.rs:343` `paint_box` | &dyn BoxModel | dispatch `sliced ? SlicedBox : frag` | | pending-migration | C-3e |
| `builder/walk.rs:108` paged-gen gate | get<&LayoutBox> → `.layout_generation` | **NOT geometry** — page-membership gate (seed 6) | needs re-home; `BoxFragment` drops `layout_generation` (hand-off row 4) | pending-migration | C-3e→C-4 |
| `builder/walk.rs:701` `is_block_child` | get<&LayoutBox>`.is_err()` | **NOT geometry** — presence predicate (block vs inline) | axis 5 presence | pending-migration | C-3e |
| `builder/mod.rs:367-372` paged blank-page test + `PageFragment` move | **field-read** `outcome.layout_box.content.size.{h,w}` (TOKEN-LESS — reached through `LayoutOutcome.layout_box`, `elidex-layout-block/src/lib.rs:57`) | is-blank-page predicate; the box then moves into `PageFragment` | screen-post-layout (paged Phase 2) | pending-migration | C-3e |
| `builder/inline.rs:73` `InlineRunContext.lb` | **binding-read** `pub(crate) lb: &'a LayoutBox` — the declaration carries the token, its SIX reads do not (`:268`, `:278`, `:294`, `:438`, `:455`, `:456`, via the `let InlineRunContext { .. lb .. } = *ctx;` destructure at `:251`/`:427`) | inline-run origin/extent + centring (`content.origin.{x,y}`, `content.size.{width,height}`, `content.center().x`) | screen-post-layout | pending-migration | C-3e |
| `builder/mod.rs:482` + `:998` `find_roots{,_mut}` | get<&LayoutBox>`.is_ok()` | **NOT geometry** — a presence predicate (axis 5), the render-walk root filter. Deduped to one allowlist row by identical content | screen-post-layout | pending-migration | C-3e |
| `builder/walk.rs:767` list marker | get<&LayoutBox> → `&child_lb` | passes to `emit_list_marker_with_counter` | | pending-migration | C-3e |
| `builder/paint/mod.rs:789-792` `find_nearest_layout_box` | fn→`Option<LayoutBox>`, get<&LayoutBox> | inline-text anchor (seed 5) | **selection problem, no store signal** — `box_fragments(ancestor)` yields N; nothing maps an inline run to its column (I-lines) | pending-migration | C-3e |
| `builder/transform.rs:19` `element_transform` | helper-param `&LayoutBox` → `.border_box()` | computes the PushTransform basis | render is the transform *producer*; reads pre-transform (correct) | pending-migration | C-3e |
| `builder/paint/mod.rs:56` `emit_background` | helper-param `&dyn BoxModel` | `.border_box()`,`.padding_box()` | | pending-migration | C-3e |
| `builder/paint/mod.rs:376` `emit_borders` | helper-param `&dyn BoxModel` | `.border_box()`,`.border()`,`.padding_box()` | | pending-migration | C-3e |
| `builder/slice.rs:79` `sliced_box` | helper-param `&dyn BoxModel` | slices a fragment's edges | | pending-migration | C-3e |
| `builder/paint/mod.rs:599` `emit_list_marker_with_counter` | helper-param `&LayoutBox` → `.content.origin` | | | pending-migration | C-3e |
| `builder/paint/mod.rs:672` `emit_text_marker` | helper-param `&LayoutBox` → `.content.origin` | | | pending-migration | C-3e |
| `builder/paint/mod.rs:725` `emit_column_rules` | helper-param `&LayoutBox` | multicol column rules | | pending-migration | C-3e |
| `builder/form.rs:93,208,314,367,385,425,502,528` (×8) | helper-param `&LayoutBox` | form-control chrome paint: `emit_form_control`, `emit_text_input`, `emit_password`, `emit_check_indicator`, `emit_button`, `emit_select`, `emit_caret`, `emit_selection_highlight` | shared profile (see 8-axis §C-3e) | pending-migration | C-3e |

### `elidex-ecs` — the seam (single site)

| file:line | reader-kind | CLASS |
|---|---|---|
| `dom/geometry.rs` (N=1 fallback, inside `ScreenGeometry::collect`) + `BoxFragment::from(&*lb)` | get<&LayoutBox> + From<&LayoutBox> | **seam** |

This is the **only** consumer-path `LayoutBox` read that is not a producer: the router's N=1
fallback (memo §1 req 2). Every migrated consumer reads through `screen_geometry().box_fragments()`,
which routes here. The trip-wire allowlists this **one** site as `seam` — NOT a blanket `elidex-ecs`
exclusion — so a future low-level reader still trips (impl §3 hole 2). (`geometry.rs` `use` import
and the `#[cfg(test)] mod tests` are infrastructure/test, not readers.)

### `elidex-plugin` + `elidex-ecs` + `elidex-render` — type-defs (grouped)

| file:line | what | CLASS |
|---|---|---|
| `elidex-plugin/src/layout_types/boxes.rs:84` | `pub struct LayoutBox` | type-def |
| `elidex-plugin/src/layout_types/boxes.rs:124` | `pub trait BoxModel` | type-def |
| `elidex-plugin/src/layout_types/boxes.rs:152` | `impl BoxModel for LayoutBox` | type-def |
| `elidex-plugin/src/layout_types/boxes.rs:170-191` | inherent forwarders `padding_box`/`border_box`/`margin_box` → `BoxModel::*` | type-def |
| `elidex-plugin/src/layout_types/mod.rs:7`, `src/lib.rs:55` | `pub use ... {BoxModel, LayoutBox}` re-exports | type-def |
| `elidex-ecs/src/fragment_tree.rs:151,164` | `pub struct BoxFragment`, `impl BoxModel for BoxFragment` | type-def |
| `elidex-render/src/builder/slice.rs:55` | `impl BoxModel for SlicedBox` (paint-time slice adapter) | type-def |

### Producers — `elidex-layout-*` (grouped, terse)

Writes (`&mut`, construction, the write chokepoint, struct fields) + in-layout producer reads. These
**survive C-4** (LayoutBox stays the producer's working representation). Representative sites; the trip-wire
treats the full set as crate-level producer entries.

⚠ **Every component write now routes through `EcsDom::set_layout_box` / `EcsDom::layout_box_mut`** (the
terminal-Z C-3a write chokepoint, plan §2) so it invalidates the screen-geometry phase. That is **19** sites —
16 whole-value writes + 3 read-modify-write — across 6 crates; an earlier count of "14 `insert_one` sites" was
low, having missed `grid/position.rs:428` and three of the five `table/lib.rs` writes. That the writer
inventory was itself incomplete is why the guard could not stay at a dispatch site: "every writer is reached
through the dispatcher" was not a claim review could check. Trip-wire **wire #5** rejects raw token-bearing
writes; see plan §2 for what it does and does not bound.

| kind | representative sites | CLASS |
|---|---|---|
| `&mut LayoutBox` read-modify-write (**via `EcsDom::layout_box_mut`**) | `block/children/shift.rs:164` (probe-lag shift, I-phase fact 1), `inline/mod.rs:705` (atomic reposition), `layout/mod.rs:157` (layout_generation stamp), `positioned/mod.rs:101` (`apply_relative_offset` param) | producer |
| multicol committer read (feeds store) | `multicol/fill.rs:76` (`snapshot_box` get<&LayoutBox>) + `:77` `BoxFragment::from`, `multicol/fill.rs:421` (monolithic block extent) | producer |
| in-layout presence check | `inline/pack/boxes.rs:62` (`get<&LayoutBox>.is_ok()` — "skip if already laid") | producer |
| in-layout derived-value helper (LOCAL box) | `table/helpers.rs:23` `box_total_height`, `table/lib.rs:61` `cell_baseline` | producer |
| construction + **`EcsDom::set_layout_box`** (one per algorithm) | `block/mod.rs:624`, `block/lib.rs:363`, `block/children/helpers.rs:355` (display:contents/anon — axis 3), `inline/pack/boxes.rs:88`, `positioned/layout.rs:515,523`, `flex/lib.rs:491,504`, `flex/algo.rs:519`, `grid/lib.rs:619`, `grid/position.rs:428`, `multicol/lib.rs:329`, `table/lib.rs:331,394,714,741,785`, `layout/mod.rs:175` | producer |
| struct field / result holder | `block/lib.rs:57` `pub layout_box: LayoutBox`, `layout/mod.rs:189` `PageFragment.layout_box`, `table/lib.rs:493` `Vec<LayoutBox>` | producer |
| producer field-read | `multicol/lib.rs:269` `outcome.layout_box.margin_box()` | producer |

---

## 8-axis classification of the pending-migration readers

The eight axes (memo §4): 1 frame · 2 phase · 3 boxless · 4 source-vs-routing · 5 reduction ·
6 home+shape · 7 identity/lifetime · 8 transform-basis.

### C-3b — CSSOM-View family (`layout_query.rs` :338/:348/:138/:148)

1. **frame** — MIXED, composed *above* the four reads: gBCR/getClientRects = doc-space **−
   `accumulated_scroll_offset`** (viewport, `:30`/`:215`); offsetT/L = **offsetParent-relative**, no
   scroll term (`:384`); client*/scroll*/clientTop/Left = frame-agnostic border-width / padding-size.
   The reads yield doc-space border/padding boxes; do **not** pin the whole family to "subtract
   scroll".
2. **phase** — screen-post-layout (JS runs after layout); MUST use the phase-guarded projection.
3. **boxless** — box-absent today returns a **zero rect** (`map_or((0,0,0,0))`). getClientRects
   step 1 needs the true *"no associated box → empty list"* predicate (cssom-view §6); it currently
   has no connectedness / `display:contents` guard, so a detached or `display:contents` element reads
   a real zero-box (§1 req 5 / hand-off row 12 `#11-find-roots-css-root-predicate`). **C-3b must add
   the predicate**; the seam only reports mechanical presence.
4. **source-vs-routing** — **source-change**: gBCR never consults getClientRects today (seed 4) but
   must union fragments post-migration; getClientRects is two-source (line rects vs column fragments,
   seed 3). Everything is source-change at N>1 (G11 last-column fact).
5. **reduction** — gBCR = **union** (spec 4-step get-the-bounding-box, built ON `union_border_boxes`,
   NOT reusing it); getClientRects = **per-fragment**; offsetW/H = **union + cross-entity** (cssom-view
   §7 also unions block-level *descendant* fragments — see axis 6); client*/clientTop/Left = **first**
   (border widths of the principal box); scrollIntoView = **first** (target box).
6. **home+shape** — `elidex-dom-api` (C-3b directly depends on it). Mostly **per-entity**; **`offsetWidth/Height`
   is CROSS-ENTITY** (aggregates descendant block-level boxes) — `union_border_boxes(entity)` alone
   cannot express it; C-3b's offset algorithm aggregates over the low per-entity fold. scrollIntoView
   shares its target-resolution with shell URL-fragment nav (seed 9).
7. **identity/lifetime** — no retained store handle; each handler returns a scalar/string immediately.
8. **transform-basis** — SPLIT: gBCR/getClientRects contract is **painted (post-transform)** (cssom-view
   §6 getClientRects step 3 applies element+ancestor transforms) — currently reads raw `border_box`
   (pre-transform) = **live gap** (hand-off row 7). offset*/client* are **spec-mandated pre-transform**
   (*"ignoring any transforms"*, §6/§7) — `box_fragments` pre-transform is correct.

### C-3c — a11y bounds (`a11y/tree.rs:122`)

1 **frame** doc-space (`border_box.to_f64_bounds()` → accesskit Rect). 2 **phase** screen-post-layout.
3 **boxless** box-absent → **skips `set_bounds`** (the I-boxless "None → skip" consumer). 4
**source-vs-routing** single box today; N>1 = source-change (whole-element extent), N=1 routing-delta.
5 **reduction** **union** (a11y wants the element's full bounds → `union_border_boxes` at N>1; today it
is the single G11 last-column box — a latent multicol bug). 6 **home+shape** `elidex-a11y`, which does
**NOT** depend on `dom-api` → needs the **LOW** `union_border_boxes` fold on `EcsDom` (memo §1
layering); per-entity. 7 **identity/lifetime** none. 8 **transform-basis** **UNRESOLVED** — memo §2
I-transform produced **no citation** for a11y's contract; `box_fragments` yields pre-transform, and
whether a11y wants painted is a C-4 question. **Do NOT mark "wants painted."**

### C-3c — hit-test (`layout/hit_test.rs:130`)

1 **frame** doc-space `border_box`; hit-test adds scroll (`query.point + query.scroll`) and **composes
the transform chain itself** (`compute_element_transform` + `mul_affine`). 2 **phase**
screen-post-layout (runs on pointer events). 3 **boxless** box-absent → `None` → no hit. 4
**source-vs-routing** **source-change** at N>1: must test **each** column fragment for containment
(today only the last-column box → a real multicol hit-test bug). 5 **reduction** **per-fragment** (hit
if ANY fragment contains the point — NOT union, which would false-hit inter-column gaps). 6 **home+shape**
`elidex-layout`; per-entity, per-fragment iteration. 7 **identity/lifetime** **RETAINS** — returns the
hit entity and C-3d's iframe click-routing consumes the hit **fragment**, so it needs the
`(entity, fragmentainer)` key (memo §1 req 1 — the yielded `fragmentainer` id; `FragmentId` index is
`clear()`-invalidated, so only the key survives). 8 **transform-basis** reads pre-transform `border_box`
and composes the transform itself → `box_fragments` (pre-transform) is **correct**.

### C-3c — flex/grid/inline baseline (`flex/lib.rs:474`, `flex/baseline.rs:18`, `grid/position.rs:444`, `inline/pack/mod.rs:613`)

1 **frame** **LOCAL**, and **not one formula** — the three container-baseline sites take a
container-content-relative *difference* (`lb.content.origin.y − container_origin.y + first_baseline`),
whereas `flex/baseline.rs:18` is **margin-box-cross-start-relative** (`item.margin_cross_start +
padding/border on the cross-start side + first_baseline`) and writing-mode-dependent (`ctx.horizontal`
selects `.top` vs `.left`). Distinct per site (memo §2 I-frame) — do not pin the family to one frame.
2 **phase** **IN-LAYOUT** (the defining constraint — MUST NOT use `box_fragments`; the store is unusable
mid-pass). 3 **boxless** box-absent → `None` baseline → other fallback. 4 **source-vs-routing** reads a
child's `first_baseline`; routing-delta (children laid before the parent's baseline; effectively N=1 in
practice). 5 **reduction** **MIXED** — the three container sites reduce **first** (first flex item / first
row-0 item / first atomic), but `flex/baseline.rs:18` is not a reduction at all: it is a **per-item map**
(one `first_baseline` written per baseline-aligned item), whose consumer `compute_line_baselines` then
reduces **per line with `.fold(0.0, f32::max)` = MAX** (CSS Flexbox §9.4 baseline alignment). A seam that
offered only a first-reduction fold would not serve it. 6 **home+shape** `elidex-layout-flex` / `-grid` /
`-block`; per-entity read of a child during parent layout. 7 **identity/lifetime** none (reads a scalar).
8 **transform-basis** pre-transform (a layout metric); but moot — **stays on live `LayoutBox`** or gets a
probe-visible accessor (C-4 gate 2b / hand-off row 2), not the screen-only seam.

### C-3d — IntersectionObserver (`intersection_observer.rs:489`, `rect_fn`)

1 **frame** **doc-space** — the closure returns `border_box()` in doc-space; elidex hands script
doc-space rects where IO §3.2.7 step 6 maps to **viewport** — a pre-existing deviation, **live on
scrolled pages** (record, don't bless). 2 **phase** screen-post-layout. 3 **boxless** box-absent →
`None` → api-observers short-circuits to **ratio 0** — an **elidex invariant, NOT a spec branch**
(pinned `intersection/tests_core.rs:295-317`). 4 **source-vs-routing** **source-change** (seed 2): must
become the cssom-view §6 get-the-bounding-box fold in doc-space. 5 **reduction** **union**
(get-the-bounding-box) — currently first/single. 6 **home+shape** registry in `elidex-api-observers`
(callability `dom-api=0`); the §6 algorithm stays in `dom-api` (floor) reached via the **DI `rect_fn`
seam** injected from the dom-api-callable `elidex-js` host. That seam is *why* the closure silently
returns `border_box()` (not the §6 box) — uncatchable by any api-observers test. C-3d decides: keep
the seam (b) or add the acyclic `api-observers → dom-api` edge (c). 7 **identity/lifetime** none. 8
**transform-basis** contract = **painted (post-transform)** (viewport-mapped, cited §4 seed 2);
currently reads pre-transform `border_box` = **live gap** (hand-off row 7).

### C-3d — ResizeObserver (`resize_observer.rs:405`, `size_fn`)

Multiple entry fields → **classify separately** (seed 1). 1 **frame** contentRect =
**padding-offset** (`content_rect_local` = `Rect::new(padding.left, padding.top, content.w,
content.h)` — RO §3.3.1 "top is padding top"); borderBoxSize = `border_box().size`. 2 **phase**
screen-post-layout. 3 **boxless** **SPEC-ZERO** — RO fires on `display:none`; `resize.rs:256`
`size_fn(...).unwrap_or((Rect::default, Size::ZERO))` — box-less is **NOT skipped**. RO's `Option` is
the helper signature, never the reader policy. 4 **source-vs-routing** RO §2.3 pins
contentBoxSize/borderBoxSize to a **single first-column size** (per-fragment = a future spec) —
**settled**, not an open fragment-choice; only contentRect height is spec-silent. 5 **reduction**
**first** (first-column). 6 **home+shape** registry in `elidex-api-observers` + DI `size_fn` seam;
the contentRect padding-offset composition needs **no dom-api**, so it belongs engine-side in
`elidex-api-observers::resize` (unlike IO's `rect_fn`) — byte-identical to today's
`content_rect_local()` (memo §1: **not** a `BoxModel` helper below the floor). 7 **identity/lifetime**
none. 8 **transform-basis** **pre-transform** — RO §3.3.1 *"observations will not be triggered by CSS
transforms"* → `box_fragments` correct.

### C-3d — shell scroll-extent (`scroll.rs:137`, `compute_content_extent`)

1 **frame** doc-space (far edges of border boxes → content extent). 2 **phase** screen-post-layout
(after `re_render`; the production `ScrollState` makes `accumulated_scroll_offset` non-zero). 3
**boxless** skips `display:none` via the co-read `ComputedStyle` — box-absent contributes nothing.
4 **source-vs-routing** source-change at N>1. 5 **reduction** — a **CROSS-ENTITY AGGREGATE** (max over
ALL entities' far edges), **not a per-entity projection**: `box_fragments(entity)` cannot express it
(seed 7). 6 **home+shape** `elidex-shell`; **cross-entity aggregate with a `display!=None` co-read** —
needs a `query`, not `box_fragments(entity)` (the axis-6 poster child). 7 **identity/lifetime** none
(accumulates a max scalar). 8 **transform-basis** scroll extent is layout-based → pre-transform
(uncited; do not assert painted).

### C-3d — shell URL-fragment nav (`scroll.rs:236`, `scroll_offset_for_fragment`)

1 **frame** doc-space `border_box.origin` → scroll offset (block:start aligns the target top). 2
**phase** screen-post-layout. 3 **boxless** matched-but-boxless (`.ok()?`) → `None` → leaves scroll
unchanged. 4 **source-vs-routing** source-change at N>1; **same algorithm as ScrollIntoView** (seed 9,
WHATWG HTML §7.4.6.4 step 3 substep 5 = cssom-view "scroll a target into view"). 5 **reduction**
**first** (target's first fragment origin). 6 **home+shape** `elidex-shell`; **shared helper with
dom-api ScrollIntoView** — decided **once** (seed 9). 7 **identity/lifetime** none. 8 **transform-basis**
scroll target position → pre-transform (uncited).

### C-3d — shell caret tracking (`event_handlers.rs:374`, `update_scroll_offset`)

1 **frame** frame-agnostic `content.size.width`. 2 **phase** screen-post-layout. 3 **boxless**
box-absent → `None` → skip. 4 **source-vs-routing** N=1 (form controls are not multicol) →
routing-delta. 5 **reduction** **first/single** (content width). 6 **home+shape** `elidex-shell`;
per-entity. 7 **identity/lifetime** none. 8 **transform-basis** content size → pre-transform
(`box_fragments` correct).

### C-3d — shell iframe click-routing (`event_handlers.rs:807`, `try_route_click_to_iframe`)

1 **frame** doc-space `content.origin`, subtracted from the parent-frame click point to produce the
child-frame-local point. 2 **phase** screen-post-layout (hit-test has already resolved `hit_entity`).
3 **boxless** ⚠ **does not skip** — `.ok().map(…).unwrap_or_default()` yields a **zero offset**, so a
boxless iframe still routes the click, untranslated, to the child frame. That is a silent
wrong-coordinate, not a no-op; C-3 **inherits** it (the seam reports absence faithfully, and
`box_fragments(e).next()` gives C-3d the `Option` it needs to distinguish the two). 4
**source-vs-routing** iframes are replaced elements and are not fragmented → N=1, routing-delta only.
5 **reduction** **first** — and this is the memo §1 req 1 consumer that wants the real
`(entity, fragmentainer)` key. `FragmentView::fragmentainer` is `Option<u32>` precisely for it: a
`None` means the fallback arm supplied the fragment and the column is unknown, so C-3d can detect
that case instead of keying on a fabricated `0`. 6 **home+shape** `elidex-shell`; per-entity. 7 **identity/lifetime** none. 8
**transform-basis** pre-transform — a transformed iframe routes clicks against untransformed
geometry today (inherited, uncited).

### C-3d — shell lazy-iframe proximity (`iframe/lifecycle.rs:267`, `check_lazy_iframes`)

1 **frame** doc-space content-rect corners tested against a viewport-derived visible band (200px
margin, per the fn docstring). 2 **phase** screen-post-layout. 3 **boxless**
`.ok().is_some_and(…)` → box-absent is `false`, i.e. a lazy iframe with no box never loads (a
skip, unlike the click-routing reader above — the two shell iframe readers resolve box-absence in
**opposite** directions, which is why they are separate rows). 4 **source-vs-routing** N=1 (replaced
element) → routing-delta only. 5 **reduction** **any** in principle (does *a* fragment intersect the
band); N=1 collapses it to first/single today. 6 **home+shape** `elidex-shell`; per-entity, evaluated
inside a `filter` over the lazy-pending set. 7 **identity/lifetime** none. 8 **transform-basis**
pre-transform (inherited, uncited).

### C-3e — render residual (walk N=1 source, paint/form helpers, transform)

Shared profile for the geometry paint readers (`walk.rs:183-188/330/343`, `paint/mod.rs:56/376/599/
672/725`, `slice.rs:79`, `form.rs ×8`): 1 **frame** doc-space raw facets (`border_box`/`padding_box`/
`content.origin`); the display list applies scroll/transform wrappers separately. 2 **phase**
**screen-post-layout AND paged-post-layout** — the **paged** path reads page-relative geometry and
**cannot** use `screen_geometry` (memo §2 fact 3); C-3e/C-4 keeps `LayoutBox` for the paged path or
lands the paged store. 3 **boxless** box-absent → text nodes / no-box entities handled via parent
generation; the walk's own `single_box` path. 4 **source-vs-routing** the screen path **already**
fragment-sources consumable multicol (C-1); migration routes the **N=1 fallback** through the seam
too. 5 **reduction** **per-fragment** (the walk already loops `for i in 0..n`, `walk.rs:329`); the
helpers paint one fragment each. 6 **home+shape** `elidex-render` (callability `dom-api=0`) → needs
the **LOW** fold; per-entity, per-fragment. 7 **identity/lifetime** `lb_owned` is **cloned** (owned
`Option<LayoutBox>`) so the source borrows neither `ctx` nor the world across child-dispatch recursion
— the seam's owned `FragmentView` satisfies this exactly. 8 **transform-basis** render **computes** the
transform wrapper from `border_box()` (`transform.rs:19`, perspective `walk.rs:315`) — it reads
**pre-transform** and *produces* the post-transform display-list wrapper, so `box_fragments`
(pre-transform) is **correct** (render is the transform producer, not a painted-geometry consumer).

**Per-reader deltas (C-3e):**
- `walk.rs:108` paged-gen gate — axis 5 **NOT a geometry read** (reads `layout_generation`, which
  `BoxFragment` drops) → needs re-home, not `box_fragments` (seed 6, hand-off row 4). axis 2
  paged-post-layout.
- `walk.rs:701` `is_block_child` — axis 5 **NOT a geometry read**, a presence predicate
  (`get.is_err()`). `box_fragments` emptiness could serve, but it is a presence branch, not geometry.
- `paint/mod.rs:789-792` `find_nearest_layout_box` — axis 5 **a selection problem with no store
  signal** (seed 5): returns one ancestor box for the inline-text anchor; `box_fragments(ancestor)`
  yields N and nothing maps an inline run to its column (I-lines gap, hand-off row 5). axis 7 returns
  an owned `LayoutBox` clone.
- `transform.rs:19` `element_transform` — the transform-basis producer; reads pre-transform (correct);
  this is where the display-list `PushTransform` is derived (memo §2 I-transform).

---

## (d) Producer-crate reads that are in-layout or presence-checks whose meaning flips under a `clear()`ed store

C-4 gate item 1 requires "producers" be defined **precisely**: some producer-crate `LayoutBox` reads
are **in-layout** or **presence checks** (axes 2/5) whose meaning **flips** under a `clear()`ed store
(`clear()` runs at the top of `layout_tree`, `layout/mod.rs:392`; the paged path never clears). These
survive C-4 but their semantics are store-state-dependent — C-4's "zero reads outside producers" proof
must account for them, and must NOT treat them as inert:

| site | why its meaning flips under `clear()` | axis |
|---|---|---|
| `block/children/shift.rs:164` (&mut) | THE probe-lag site (I-phase fact 1): `lb.content.origin += delta` is **unguarded in probes** while `shift_entity`/`push_box` are `!is_probe`-gated — so during a 2-pass flex·grid·table re-measure the component holds the working value and the store holds the prior definitive pass. | 2 (in-layout, probe) |
| `inline/pack/boxes.rs:62` (presence) | "skip entities that already have a `LayoutBox`" — after `clear()` (of the component, were it ever cleared) nothing is skipped → double-lay. A pure control-flow presence read whose meaning is the store's population state. | 5 (presence) |
| `multicol/fill.rs:76` (`snapshot_box`) | the committer read that snapshots `LayoutBox → BoxFragment` into the store; it reads the *component* to *write* the store that `clear()` resets — the read is the store's own source of truth. | 2/5 |
| `multicol/fill.rs:421` (monolithic extent) | reads a child's `content.size` mid-fill to size columns — an in-layout store read. | 2 |
| `layout/mod.rs:157` (&mut) | stamps `layout_generation` only when `> 0` (paged) — a paged-path mutation whose read/write is meaningless on the (never-cleared) paged store. | 2 (paged) |
| `flex/lib.rs:474`, `flex/baseline.rs:18`, `grid/position.rs:444`, `inline/pack/mod.rs:613` (baseline) | in-layout reads of a **child's** store box to compute the parent's baseline — classified pending-migration/C-3c (the slice owns the decision), but they ARE producer-crate in-layout reads that C-4 gate 2b tracks: they need a **probe-visible geometry source** or keep `LayoutBox` (hand-off row 2). All **four** (`baseline.rs:18` was reclassified out of `producer` in Codex PR#488 R1 — see the ⚠ above the C-3c table). | 2 (in-layout) |
| `table/helpers.rs:23`, `table/lib.rs:61` (derived-value helpers) | in-layout, but read a **LOCAL** `LayoutBox`/`Vec<LayoutBox>` (never the ECS store) → `clear()`-**independent**. Listed for completeness; not a store-state hazard. | 2 (in-layout, local) |

⚠ **Axis 3's other half — there is no `LayoutBox` REMOVAL path, so "box-absent" is not reachable for
any previously-laid entity.** The memo §4 axis-3 mandate is to enumerate every producer path that
leaves a `LayoutBox` on an element with no associated CSS box; the sites below are the *commit*
half. The *removal* half is empty: `git grep -nE 'remove(_one)?::<[^>]*LayoutBox'` over `crates/**`
finds exactly **one** site, in a genuinely test-only path
(`.../vm/tests/tests_resize_observer.rs:293`). ⚠ It was two until PR #488: the
`remove_one::<LayoutBox>` in the production-compiled `vm/test_helpers.rs` was dropped when that
writer moved onto `EcsDom::set_layout_box` (the removal was a no-op — `insert_one` already upserts).
So "the removal half is empty" now holds **without** a `#[cfg(feature)]` caveat. Layout **skips**
`display:none` subtrees (`elidex-layout/src/layout/mod.rs:441`,
`elidex-layout-block/src/block/children/helpers.rs:133`, `.../stack.rs:137`) without clearing the
component, whereas `FragmentTree::clear()` drops store fragments every pass. So the seam's two
sources have **different staleness models** — store = fresh-this-pass, component = **ever-laid** —
and an element toggled to `display:none` after a pass keeps its last box forever, which the N=1
fallback then yields as a live fragment inside a `CompletedScreen` view.

Consequence for the per-reader axis-3 rows below: wherever a row says "box-absent → …", that branch
fires only for an element that has **never** been laid out. Concretely, the ResizeObserver row's
`SPEC-ZERO` note describes a state today's engine cannot produce for a once-laid target — RO will
report the stale non-zero size instead. (a11y is incidentally saved because its own `is_hidden`
predicate — `elidex-a11y/src/tree.rs:280`, applied at `:50` — tests `ComputedStyle.display ==
None` at `:293` in addition to `aria-hidden`/`hidden`; shell scroll-extent is saved by its
`ComputedStyle` co-read. IO/RO read `LayoutBox` with **no** style co-read and are **not** saved.)
This is **inherited, not introduced** —
C-3a changes no producer — and fixing it means giving "has a box this pass" a real write path
(per-pass component reconcile, or a laid-this-pass marker/generation the fallback checks), which is
a producer-side change outside a seam slice. Registered as **`#11-layoutbox-absence-unreachable`**
(hand-off; resolution trigger = C-4, or any slice that needs a truthful box-absent signal — it is
the same missing fact as hand-off row 1 `#11-fragment-store-n1-coverage-marker`, from the component
side rather than the store side).

The `display:contents` / anon-box **producer defect** the audit must record (axis 3): the producer
commit sites that leave a `LayoutBox` on a spec-boxless element — `block/children/helpers.rs:355`
(a bare `insert_one(child, lb)` in child-box positioning — see the ⚠ below; it is NOT the
anon-box / flattened-`contents` insert) and `inline/pack/boxes.rs:88` — plus the
**detached-element** path (`find_roots`/`root_entities` re-lays a parentless-but-styled element against
the viewport, hand-off row 12). C-3 **inherits** these (no regression); the seam reports their presence
faithfully — "presence" is a mechanical store fact, not a "has an associated CSS box" verdict (memo §1
req 5). **Four** live comments cite CSS Display 3 **§2.8** — a citation drift the axis records rather
than trusts. §2.8 is "The Root Element's Principal Box" (webref `css-display-3`), so none of them is
about the root element — and the four do **not** all take the same correction:

| site | what the comment actually states | correct anchor |
|---|---|---|
| `elidex-layout/src/layout/mod.rs:116` | `display: contents` generates no box | **§2.5** "Box Generation: the none and contents keywords" |
| `elidex-style/src/resolve/mod.rs:497` | same box-generation rule | **§2.5** |
| `elidex-layout-block/src/helpers.rs:355` (`flatten_contents` docstring; also drops the `§`) | same box-generation rule | **§2.5** |
| `elidex-style/src/resolve/mod.rs:212` | **blockification** — maps inline-level display to block-level when floated / absolutely positioned (the CSS 2.1 §9.7 co-cite confirms) | **§2.7** "Automatic Box Type Transformations" — *not* §2.5 |

⚠ The `:212` row is why this is a table and not one sentence: an earlier draft gave a single
correction (§2.5) covering `:71` / `:497` / `:212`, so a downstream slice applying it verbatim would
have swapped one wrong citation for another.

⚠ Do not confuse the two `helpers.rs:355`. The **`flatten_contents`** one above is
`elidex-layout-block/src/helpers.rs:355` (the 4th drift site, named by the design memo). The producer
commit site this paragraph opens with is `elidex-layout-block/src/block/children/helpers.rs:355` — a
different file: a bare `insert_one(child, lb)` in child-box positioning, carrying no comment and no
citation (so it is *not* "the anon-box / flattened-`contents` insert" either).

---

## Known-hard seed edges (memo §4) — located in live code

| # | seed | live site(s) | classification | slice |
|---|---|---|---|---|
| 1 | RO multi-field | `resize_observer.rs:405-406` — contentRect (`content_rect_local`, padding-offset) + borderBoxSize (`border_box().size`) | axis 5 separate fields, first-column (RO §2.3 settled); spec-zero (axis 3); pre-transform (axis 8) | C-3d |
| 2 | IO doc-space | `intersection_observer.rs:489-490` — `border_box()` in doc-space (viewport deviation) | source-change (axis 4); union §6 (axis 5); painted contract (axis 8) | C-3d |
| 3 | getClientRects two-source | `layout_query.rs:219` (`InlineClientRects`) + `:237→:338` (`get_border_box`) | line vs column dispatch; both-split = I-lines | C-3b |
| 4 | gBCR source-change | `layout_query.rs:26→:338` — never consults getClientRects today | source-change (axis 4) | C-3b |
| 5 | render inline-text anchor | `paint/mod.rs:789-792` `find_nearest_layout_box` | selection problem, no store signal (axis 5, I-lines) | C-3e |
| 6 | render paged-gen gate | `walk.rs:108-109` — `lb.layout_generation` | NOT a geometry read (axis 5); re-home (hand-off row 4) | C-3e→C-4 |
| 7 | shell scroll-extent | `scroll.rs:133-149` `compute_content_extent` (`query<(Entity,(&LayoutBox,&ComputedStyle))>`) | cross-entity aggregate (axis 6) | C-3d |
| 8 | flex/grid baseline in-layout | `flex/lib.rs:474`, `flex/baseline.rs:18`, `grid/position.rs:444` (+ sweep sibling `inline/pack/mod.rs:613`) — **four** sites | in-layout (axis 2) + local frames (axis 1) → stay on `LayoutBox` | C-3c |
| 9 | ScrollIntoView == URL-fragment nav | `layout_query.rs:257-276` `ScrollIntoView` + `scroll.rs:210-246` `scroll_offset_for_fragment` | same algorithm (cssom-view "scroll a target into view") — one shared helper | C-3b + C-3d |

## Appendix — producer-crate test-only reads (noted, not in the CONSUMER classification)

Producer/consumer crates have extensive `#[cfg(test)]` `get::<&LayoutBox>` assertions and `LayoutBox {
… }` fixtures (excluded per method): e.g. `elidex-layout-block/src/**/tests/*.rs` (float_layout,
writing_mode, absolute_fixed, margin_collapse, height_replaced), `elidex-layout-multicol/src/tests/
{geometry,fill,spanner,…}.rs`, `elidex-layout-table/src/tests/*.rs`, `elidex-render/src/builder/
tests/*.rs`, `elidex-dom-api/src/element/layout_query.rs` (`mod tests` fixtures), `elidex-ecs/src/dom/
geometry.rs` (`mod tests`), `elidex-js/src/vm/{test_helpers.rs,tests/tests_resize_observer.rs}`. These
do not gate C-4 (the trip-wire excludes test paths).

---

## Reader-universe exhaustiveness cross-check

13 crates carry the token; **6 crates + api-observers have ZERO** production readers
(`elidex-navigation`, `elidex-form`, `elidex-form-core`, `elidex-style`, `elidex-web-canvas`,
`elidex-wasm-runtime`, and `elidex-api-observers`) — confirming observer geometry is read **only**
via the `elidex-js` DI closures, never in `api-observers` (whose `RectProvider`/`SizeProvider` carry
no token). This is a load-bearing exhaustiveness check for the C-3d layering decision.

⚠ **`elidex-js/src/vm/test_helpers.rs` — a `test_`-prefixed file that is NOT test-only.** It is
`#[cfg(feature = "engine")] #[doc(hidden)] pub mod` (`vm/mod.rs:84`), i.e. production-compiled shipped
code, and `set_layout_box` *writes* `LayoutBox`. The gate originally excluded it by basename — the
same false-exhaustiveness hazard the Method section describes for `hit_test.rs`, reintroduced through
the `test_*` PREFIX rather than the `*_test.rs` suffix. The prefix is no longer excluded (it was the
only token-carrying `test_*.rs` in the tree), and its **two** non-import token lines (the
`LayoutBox { .. }` literal and `..LayoutBox::default()`) are classified
**`pending-migration:C-4`**: they are writes, so no C-3 consumer slice migrates them — only the
delete does.

⚠ **It was four lines until this slice, and the two that went away are the point.** PR #488 routed
this writer through `EcsDom::set_layout_box` but preserved its panic-on-despawned-entity contract with
`assert!(dom.world().get::<&LayoutBox>(entity).is_ok(), …)` — which introduced a **fresh raw
`LayoutBox` read in production-compiled code**, and it was classified `test` (this file's basename
still looks test-shaped), so **C-4's "zero reads outside producers" gate would not have seen it**. That
is the exact failure this ⚠ exists to prevent, reintroduced by the change that strengthened the gate.
The fix is not a re-label: `set_layout_box` now **returns whether it wrote**, so the caller asserts on
the chokepoint's own signal and reads nothing back. A write-signal return is strictly better than a
read-back here — it reports the write rather than inferring it, and it adds no reader for C-4 to
account for. They are deliberately NOT counted in the consumer tally below, which counts
consumer *reads*.

**Pending-migration consumer readers: 43 non-test sites** → C-3b (4), C-3c (6), C-3d (7), C-3e (26).
(C-3e's 26 sites live in 18 table rows: `form.rs` is one row for 8 sites and `builder/mod.rs:482`+`:998`
one row for 2 — the allowlist collapses each group to a single `(path, content)` key, so ROWS and SITES
differ by construction. See "Counting basis".)
**Seam: a single site** (`dom/geometry.rs`), the only non-producer `LayoutBox` read. All 9 memo §4
seed edges located + classified.

⚠ **Counting basis — this doc vs the machine allowlist** (`.claude/tools/layout-box-reader-allowlist.tsv`):
this document counts **logical readers** (a reader whose geometry read spans an `use` import + a `get`
+ a helper-param is ONE reader here); the allowlist counts **unique `(path, content)` token lines**, so
its per-slice tallies differ (a logical reader contributes an import row + a get row; identical-content
lines in one file — e.g. `form.rs`'s eight `lb: &LayoutBox` helpers — dedup to one via `sort -u`). The
allowlist is the **freshness/exhaustiveness gate** (it fires on any *novel-content* reader line and, at
C-4, on any surviving `pending-migration` row); per-**site** precision (form ×8, the offsetParent
handlers) is THIS document's job. A partial migration (7 of 8 identical-content readers) does not clear
the allowlist row until the last copy is gone — correct for C-4's "zero pending-migration" check, which
needs *full* migration. The two artifacts are consistent by design, not by equal counts.
