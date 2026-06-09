//! Pre-order tree walk for display list building.

use std::sync::Arc;

use elidex_ecs::{
    BackgroundImages, BoxFragment, EcsDom, Entity, FragmentContent, ImageData, ListItemMarker,
    TemplateContent, MAX_ANCESTOR_DEPTH,
};
use elidex_form::FormControlState;
use elidex_layout_block::paint_order::{collect_sc_participants, is_float_entity, is_positioned};
use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
use elidex_plugin::transform_math::{resolve_child_perspective, Perspective};
use elidex_plugin::{
    BorderCollapse, BoxDecorationBreak, BoxModel, ComputedStyle, Display, EdgeSizes, EmptyCells,
    LayoutBox, ListStyleType, MulticolInfo, Rect, Visibility, WritingMode,
};
use elidex_plugin::{Point, Vector};
use elidex_style::counter::{apply_implicit_list_counters_from_dom, CounterState};
use elidex_text::FontDatabase;

use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;

use super::form::emit_form_control;
use super::transform::{element_transform, TransformResult};
use super::{emit_background, emit_borders, emit_inline_run, emit_list_marker_with_counter};

/// Shared mutable state threaded through the display list walk.
///
/// Groups the invariant references and mutable buffers that every
/// recursive `walk` call needs, reducing per-call argument counts.
pub(crate) struct PaintContext<'a> {
    pub(crate) dom: &'a EcsDom,
    pub(crate) font_db: &'a FontDatabase,
    pub(crate) font_cache: &'a mut FontCache,
    pub(crate) dl: &'a mut DisplayList,
    pub(crate) caret_visible: bool,
    /// Viewport scroll offset.
    ///
    /// This is the same value used for the root-level `PushScrollOffset`
    /// in `build_display_list_with_scroll()`. Fixed elements re-push this
    /// value after their `PopScrollOffset`/walk/`PushScrollOffset` sequence.
    pub(crate) scroll_offset: Vector,
    /// CSS counter state machine (CSS Lists Level 3 §4 Automatic Numbering).
    ///
    /// Retained ONLY to populate document counters for paged-media margin boxes
    /// (`emit_margin_boxes`): per-page running-header counter values depend on
    /// post-pagination page assignment, so they cannot be precomputed pre-layout.
    /// Document generated content (pseudo `content`, list-item markers) is resolved
    /// once before layout by `elidex-style` and read from components — render no
    /// longer derives it here. Mutated only when `paged` (see [`Self::paged`]).
    pub(crate) counter_state: CounterState,
    /// Whether this walk feeds a paged-media display list.
    ///
    /// Gates the `counter_state` mutation: only the paged builders populate
    /// document counters for margin boxes. Off the paged path the counter machine
    /// would do unused work (markers/pseudo read pre-layout components), so its
    /// reset/increment processing is skipped (scope push/pop stay for balance).
    pub(crate) paged: bool,
    /// Expected layout generation for per-fragment paged media rendering.
    ///
    /// When `Some(gen)`, the walk skips entities whose `LayoutBox.layout_generation`
    /// doesn't match. When `None` (non-paged path), all entities are visited.
    pub(crate) expected_generation: Option<u32>,
    /// Entities that are continuations from a previous page fragment.
    ///
    /// For these entities, `counter-increment` is suppressed per CSS
    /// Fragmentation Level 3 §4.
    pub(crate) continuation_entities: Option<std::collections::HashSet<elidex_ecs::Entity>>,
}

/// Pre-order walk: emit paint commands for this entity, then recurse.
///
/// Children are grouped into "inline runs" (consecutive non-block children)
/// and "block children" (those with a `LayoutBox`). Inline runs have their
/// text collected and rendered as a single item; block children are
/// recursed into normally.
///
/// For stacking contexts, children are painted in CSS 2.1 Appendix E order:
/// Layer 2 (negative z) → Layer 3 (blocks) → Layer 4 (floats) →
/// Layer 5 (inline) → Layer 6 (positioned auto + z:0) → Layer 7 (positive z).
///
/// Recursion is capped at `MAX_ANCESTOR_DEPTH` to prevent stack overflow.
#[allow(clippy::too_many_lines)]
pub(crate) fn walk(
    ctx: &mut PaintContext,
    entity: Entity,
    depth: usize,
    parent_perspective: &Perspective,
    in_transform: bool,
) {
    if depth > MAX_ANCESTOR_DEPTH {
        return;
    }
    // Skip <template> elements — their content is inert.
    if ctx.dom.world().get::<&TemplateContent>(entity).is_ok() {
        return;
    }

    // Per-fragment paged media: skip entities not belonging to this page.
    // Only check entities that HAVE a LayoutBox — text nodes and other
    // non-layout entities don't have one and should be visited normally
    // (their visibility is determined by their parent's generation).
    if let Some(expected_gen) = ctx.expected_generation {
        if let Ok(lb) = ctx.dom.world().get::<&LayoutBox>(entity) {
            if lb.layout_generation != expected_gen {
                return;
            }
        }
    }

    // CSS counter scope (CSS Lists 3 §4.3 Nested Counters and Scope): push scope
    // on entry / pop on exit unconditionally to keep the scope stack balanced.
    // The counter MUTATION (reset/set/increment) runs only on the paged path —
    // its sole remaining consumer is paged-media margin boxes; document content
    // (pseudo text, list markers) is resolved pre-layout and read from components.
    ctx.counter_state.push_scope();
    if ctx.paged {
        if let Some(mut style) = ctx
            .dom
            .world()
            .get::<&ComputedStyle>(entity)
            .ok()
            .map(|s| (*s).clone())
            // CSS Lists 3 §4.5: an element with `display: none` generates no box and
            // cannot set/reset/increment a counter — skip its counter processing
            // (matching the pre-layout pass, so paged document counters agree).
            .filter(|style| style.display != Display::None)
        {
            // Apply implicit list-item counters for <ol>, <ul>, <li> (CSS Lists 3 §4.6).
            apply_implicit_list_counters_from_dom(ctx.dom, entity, &mut style);
            // CSS Fragmentation L3 §4: suppress counter-increment on continuation
            // entities (those that started on a previous page fragment).
            let is_continuation = ctx
                .continuation_entities
                .as_ref()
                .is_some_and(|set| set.contains(&entity));
            ctx.counter_state.process_element(&style, is_continuation);
        }
    }

    // Fetch ComputedStyle once for display/visibility/painting checks.
    let style_ref = ctx.dom.world().get::<&ComputedStyle>(entity).ok();

    // Check for display: none — skip this subtree entirely.
    if let Some(ref style) = style_ref {
        if style.display == Display::None {
            ctx.counter_state.pop_scope();
            return;
        }
        // CSS 2.1 §11.2: visibility: collapse on table-row, table-column,
        // table-row-group, or table-column-group hides the entire row/column
        // (equivalent to display: none for rendering purposes).
        if style.visibility == Visibility::Collapse && style.display.is_table_internal() {
            ctx.counter_state.pop_scope();
            return;
        }
    }

    // Check visibility — hidden elements skip painting but still occupy space
    // and children can override visibility, so we must recurse.
    // For non-table elements, 'collapse' is treated the same as 'hidden'.
    let is_visible = style_ref
        .as_ref()
        .is_none_or(|s| s.visibility == Visibility::Visible);

    // ── Chrome + overflow clip + content, emitted over a per-entity *fragment
    // source* (terminal-Z C-1, the unified fragment-walk). Per-entity concerns
    // (transform / perspective / replaced content / form) run once; the chrome +
    // clip + content emission loops the source. The source is mechanical: a
    // consumable multicol mid-break entity (direct-child IFC, store-flagged) yields
    // its N per-column `BoxFragment`s — so it paints per-column chrome (css-break-3
    // §5.4 slice), per-column overflow clip (css-multicol-1 §8.1), and per-column
    // clipped content; every other entity yields its one `LayoutBox` (N=1, a
    // 1-iteration loop byte-identical to the pre-C-1 display list). Both geometry
    // carriers implement `BoxModel`, so the loop body is geometry-source-agnostic.
    //
    // The geometry is cloned into owned locals so the source borrows neither `ctx`
    // nor the ECS world across the child-dispatch recursion (which needs `&mut ctx`).
    let lb_owned: Option<LayoutBox> = ctx
        .dom
        .world()
        .get::<&LayoutBox>(entity)
        .ok()
        .map(|l| (*l).clone());
    // Store-consumable source (§2.2), non-paged only (§2.8): a paged multicol keeps
    // the per-page LayoutBox path until paged×multicol store unification. The cheap
    // `!is_empty()` arena check skips the per-entity consumable lookup entirely on the
    // common no-multicol page (the store has no fragments at all).
    let store_frags: Vec<BoxFragment> = if ctx.expected_generation.is_none()
        && !ctx.dom.fragment_tree().is_empty()
        && ctx.dom.fragment_tree().is_consumable(entity)
    {
        ctx.dom
            .fragment_tree()
            .fragments_for(entity)
            .map(|n| {
                let FragmentContent::Box(bf) = &n.content;
                bf.clone()
            })
            .collect()
    } else {
        Vec::new()
    };

    // Child dispatch is the "content" emitter for both the box and no-box arms; the
    // children list + stacking-context flag are identical in both (a box arm has
    // `style` in scope, the no-box arm reads it via `style_ref` — same predicate), so
    // compute them once here. Only `child_perspective` differs (a box anchors it to its
    // border box; a box-less element has none), so it stays per-arm.
    let children = elidex_layout_block::composed_children_flat(ctx.dom, entity);
    let is_sc = style_ref
        .as_ref()
        .is_some_and(|s| s.creates_stacking_context())
        || ctx.dom.get_parent(entity).is_none(); // root is always a SC

    let mut has_transform_push = false;
    // The box paint runs only with BOTH a LayoutBox and a ComputedStyle (a LayoutBox
    // without style emitted no chrome/transform/clip pre-C-1 — preserved).
    if let (Some(lb), Some(style)) = (lb_owned.as_ref(), style_ref.as_deref()) {
        // CSS Transforms: per-entity, once (§2.7), from the entity's single LayoutBox.
        match element_transform(style, lb, parent_perspective) {
            TransformResult::BackfaceHidden => {
                // CSS Transforms L2 §5: back-facing → skip entire subtree.
                ctx.counter_state.pop_scope();
                return;
            }
            TransformResult::Affine(affine) => {
                ctx.dl.push(DisplayItem::PushTransform { affine });
                has_transform_push = true;
            }
            TransformResult::None => {}
        }

        // CSS 2.1 §17.5.1: empty-cells: hide suppresses background/border for empty
        // table cells when border-collapse is separate.
        let skip_cell_paint = is_visible
            && style.display == Display::TableCell
            && style.empty_cells == EmptyCells::Hide
            && style.border_collapse == BorderCollapse::Separate
            && is_cell_empty(ctx.dom, entity);

        // The fragment source (BoxModel items, fragmentainer order). Single-box ⇒
        // N=1 (the entity's LayoutBox); consumable multicol ⇒ N store fragments. The
        // loop indexes the source directly (no per-box `Vec` allocation on the common
        // N=1 path).
        let single_box = store_frags.is_empty();
        let n = if single_box { 1 } else { store_frags.len() };
        let clips = style.clips_overflow();
        let slice = style.box_decoration_break == BoxDecorationBreak::Slice;
        // The slice break axis is the FRAGMENTATION CONTEXT's block-flow direction =
        // the multicol container's writing mode (css-break-3: one block-flow direction
        // from the fragmentation root, even if the fragmented child sets a different
        // writing-mode). The consumable entity is the multicol's direct child by
        // construction, so its parent IS the container. Single-box ⇒ no slicing, so the
        // child's own writing mode is harmless there.
        let frag_wm = if single_box {
            style.writing_mode
        } else {
            ctx.dom
                .get_parent(entity)
                .and_then(|p| {
                    ctx.dom
                        .world()
                        .get::<&ComputedStyle>(p)
                        .ok()
                        .map(|s| s.writing_mode)
                })
                .unwrap_or(style.writing_mode)
        };

        // Per-iteration "content" inputs (loop-invariant). `child_perspective` is
        // box-anchored, so it is computed here (not hoisted); `children` / `is_sc` were
        // computed once above the box/no-box split.
        let child_perspective = resolve_child_perspective(style, &lb.border_box());
        let child_in_transform = in_transform || has_transform_push;
        let bg_images = ctx.dom.world().get::<&BackgroundImages>(entity).ok();

        // The single chrome + clip + content loop. `cum_block` accumulates the
        // block-axis extent of prior fragments for the slice background-position
        // offset (css-break-3 §5.4.1). On a fragment with `overflow:hidden` the
        // content is re-emitted under each disjoint per-column clip (each line
        // survives in exactly one column); without a clip it is emitted once.
        let mut cum_block = 0.0_f32;
        // `i` indexes the source but also drives `break_edges` / the single-box branch,
        // and the single-box arm has no `store_frags` to iterate — a range loop is the
        // natural form here.
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            let frag: &dyn BoxModel = if single_box { lb } else { &store_frags[i] };
            // box-decoration-break: slice (css-break-3 §5.4) — at each column break no
            // border/padding is inserted and border-radius applies to the unbroken
            // whole box. Build this fragment's paint geometry: omit the broken border
            // edges, zero their padding (the sliced clip / bg box), square the broken
            // corners, and offset the bg-position for tiling continuity (§5.4.1).
            // clone / single-box (n == 1) ⇒ no slicing → byte-identical to pre-C-1.
            let omit = if slice && n > 1 {
                break_edges(i, n, frag_wm)
            } else {
                [false; 4]
            };
            let sliced = omit.iter().any(|&o| o).then(|| sliced_box(frag, omit));
            let paint_box: &dyn BoxModel = sliced.as_ref().map_or(frag, |s| s as &dyn BoxModel);
            let radii = if sliced.is_some() {
                square_broken_corners(style.border_radii, omit)
            } else {
                style.border_radii
            };
            let bg_offset = if slice && n > 1 {
                slice_bg_offset(frag_wm, cum_block)
            } else {
                Vector::<f32>::ZERO
            };

            if is_visible && !skip_cell_paint {
                emit_background(
                    paint_box,
                    style.background_color,
                    radii,
                    style.opacity,
                    bg_images.as_deref(),
                    style,
                    bg_offset,
                    ctx.dl,
                );
                emit_borders(paint_box, style, omit, ctx.dl);

                // Per-entity replaced content / form / column rules: single-box only
                // (a consumable multicol IFC mid-break carries none of these
                // components). Kept in the pre-C-1 paint order — background, border,
                // then column-rules / image / iframe / form.
                if single_box {
                    if let Ok(info) = ctx.dom.world().get::<&MulticolInfo>(entity) {
                        super::paint::emit_column_rules(lb, style, &info, ctx.dl);
                    }
                    if let Ok(image_data) = ctx.dom.world().get::<&ImageData>(entity) {
                        if style.opacity > 0.0 && image_data.width > 0 && image_data.height > 0 {
                            ctx.dl.push(DisplayItem::Image {
                                painting_area: lb.content,
                                pixels: Arc::clone(&image_data.pixels),
                                image_width: image_data.width,
                                image_height: image_data.height,
                                position: Point::ZERO,
                                size: lb.content.size,
                                repeat: BgRepeat {
                                    x: BgRepeatAxis::NoRepeat,
                                    y: BgRepeatAxis::NoRepeat,
                                },
                                opacity: style.opacity,
                            });
                        }
                    }
                    // Iframe DOM children are fallback content — emit the iframe's own
                    // display list and skip child painting (break the loop; the
                    // after-loop cleanup pops the transform + scope). No clip is pushed
                    // this iteration yet, so nothing is left unbalanced.
                    if let Ok(iframe_dl) = ctx
                        .dom
                        .world()
                        .get::<&crate::display_list::IframeDisplayList>(entity)
                    {
                        ctx.dl.push(DisplayItem::SubDisplayList {
                            offset: lb.content.origin,
                            clip: lb.content,
                            list: Arc::clone(&iframe_dl.0),
                        });
                        break;
                    }
                    if let Ok(fcs) = ctx.dom.world().get::<&FormControlState>(entity) {
                        emit_form_control(
                            lb,
                            &fcs,
                            style,
                            &mut super::form::FontEnv {
                                db: ctx.font_db,
                                cache: ctx.font_cache,
                            },
                            ctx.dl,
                            ctx.dom
                                .world()
                                .get::<&elidex_ecs::ElementState>(entity)
                                .ok()
                                .is_some_and(|s| s.contains(elidex_ecs::ElementState::FOCUS)),
                            ctx.caret_visible,
                        );
                    }
                }
            }

            // overflow clipping → clip content to this fragment's (sliced) padding box
            // (CSS Overflow §3 / css-multicol-1 §8.1: per-column clip; at a slice break
            // the padding is removed, so adjacent columns' clips abut at the content
            // edge).
            if clips {
                ctx.dl.push(DisplayItem::PushClip {
                    rect: paint_box.padding_box(),
                    radii: [0.0; 4],
                });
            }
            // Content: clipping ⇒ once per fragment (under each disjoint column clip);
            // non-clipping ⇒ once (after all chrome — the last/only iteration).
            if clips || i == n - 1 {
                dispatch_children(
                    ctx,
                    entity,
                    &children,
                    depth,
                    &child_perspective,
                    child_in_transform,
                    is_sc,
                );
            }
            if clips {
                ctx.dl.push(DisplayItem::PopClip);
            }

            // Accumulate the (sliced) PADDING-box block extent — the background
            // painting area is the padding box (`emit_background`), and slice inserts
            // no border/padding at a break (§5.4), so consecutive fragments' sliced
            // padding boxes are contiguous in the composite; the bg-position offset is
            // measured in that same box, along the fragmentation block axis.
            cum_block += block_axis_extent(paint_box.padding_box(), frag_wm);
        }

        if has_transform_push {
            ctx.dl.push(DisplayItem::PopTransform);
        }
        ctx.counter_state.pop_scope();
        return;
    }

    // No LayoutBox (or no ComputedStyle): no chrome / transform / clip — recurse
    // children once (the pre-C-1 no-LayoutBox behavior). Perspective defaults with no
    // border box to anchor it; `children` / `is_sc` were computed above the split.
    let child_perspective = Perspective::default();
    dispatch_children(
        ctx,
        entity,
        &children,
        depth,
        &child_perspective,
        in_transform,
        is_sc,
    );

    // CSS counter scope: pop scope on element exit.
    ctx.counter_state.pop_scope();
}

/// Dispatch an entity's children: stacking-context elements use CSS 2.1 Appendix E
/// 7-layer order, others paint in DOM order. The single "content" emitter shared by
/// the N=1 and the per-column multicol arms of the fragment-walk (`walk`).
fn dispatch_children(
    ctx: &mut PaintContext,
    entity: Entity,
    children: &[Entity],
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
    is_sc: bool,
) {
    if is_sc {
        paint_stacking_context_layers(
            ctx,
            entity,
            children,
            depth,
            child_perspective,
            in_transform,
        );
    } else {
        paint_non_sc(
            ctx,
            entity,
            children,
            depth,
            child_perspective,
            in_transform,
        );
    }
}

/// Physical `[top, right, bottom, left]` "this edge is at a fragmentation break"
/// flags for fragment `i` of `n` along the writing-mode block axis (css-break-3
/// §5.4: no border/padding is inserted *at a break*). A continuation fragment
/// (`i > 0`) breaks at its block-start; a fragment that continues (`i < n - 1`)
/// breaks at its block-end. The block axis maps to physical edges by writing mode;
/// the inline-axis edges are never "at a break" (they paint on every fragment).
/// `n <= 1` ⇒ no breaks (single box).
fn break_edges(i: usize, n: usize, wm: WritingMode) -> [bool; 4] {
    if n <= 1 {
        return [false; 4];
    }
    let at_block_start = i > 0;
    let at_block_end = i < n - 1;
    // (block-start, block-end) physical indices into [top, right, bottom, left].
    let (start_idx, end_idx) = match wm {
        WritingMode::HorizontalTb => (0, 2), // top / bottom
        WritingMode::VerticalRl | WritingMode::SidewaysRl => (1, 3), // right / left
        WritingMode::VerticalLr | WritingMode::SidewaysLr => (3, 1), // left / right
    };
    let mut e = [false; 4];
    e[start_idx] = at_block_start;
    e[end_idx] = at_block_end;
    e
}

/// A box with the fragmentation-break edges' padding **and** border zeroed
/// (css-break-3 §5.4: no border or padding is inserted at a break, so adjacent
/// fragments' content abuts). The border is also painted-omitted via the `emit_*`
/// `omit_edges`; this zeroing makes the geometry (border/padding boxes used for the
/// bg fill area and the overflow clip) match. `omit` is the physical
/// `[top, right, bottom, left]` break-edge mask.
struct SlicedBox {
    content: Rect,
    padding: EdgeSizes,
    border: EdgeSizes,
    margin: EdgeSizes,
}

impl BoxModel for SlicedBox {
    fn content(&self) -> Rect {
        self.content
    }
    fn padding(&self) -> EdgeSizes {
        self.padding
    }
    fn border(&self) -> EdgeSizes {
        self.border
    }
    fn margin(&self) -> EdgeSizes {
        self.margin
    }
}

/// Build the [`SlicedBox`] for a fragment: zero the padding + border on each broken
/// edge (css-break-3 §5.4). The non-broken (outer) edges keep their real values.
fn sliced_box(frag: &dyn BoxModel, omit: [bool; 4]) -> SlicedBox {
    let z = |v: f32, o: bool| if o { 0.0 } else { v };
    let (p, b) = (frag.padding(), frag.border());
    SlicedBox {
        content: frag.content(),
        padding: EdgeSizes::new(
            z(p.top, omit[0]),
            z(p.right, omit[1]),
            z(p.bottom, omit[2]),
            z(p.left, omit[3]),
        ),
        border: EdgeSizes::new(
            z(b.top, omit[0]),
            z(b.right, omit[1]),
            z(b.bottom, omit[2]),
            z(b.left, omit[3]),
        ),
        margin: frag.margin(),
    }
}

/// Square the `border-radius` corners adjacent to a fragmentation break (css-break-3
/// §5.4: `slice` applies border-radius to the unbroken whole box, so only its real
/// outer corners are rounded — internal break corners are square). `radii` is
/// `[top-left, top-right, bottom-right, bottom-left]`; `omit` is `[top, right,
/// bottom, left]`. A corner is squared iff either of its two adjacent edges broke.
fn square_broken_corners(radii: [f32; 4], omit: [bool; 4]) -> [f32; 4] {
    let [top, right, bottom, left] = omit;
    [
        if top || left { 0.0 } else { radii[0] },     // top-left
        if top || right { 0.0 } else { radii[1] },    // top-right
        if bottom || right { 0.0 } else { radii[2] }, // bottom-right
        if bottom || left { 0.0 } else { radii[3] },  // bottom-left
    ]
}

/// Block-axis extent of a box under writing mode `wm` (height for a horizontal
/// mode, width for a vertical mode) — accumulated across fragments (over their
/// padding boxes, the bg painting area) to offset the slice background-position so
/// its tiling phase stays continuous.
fn block_axis_extent(box_rect: Rect, wm: WritingMode) -> f32 {
    if wm.is_horizontal() {
        box_rect.size.height
    } else {
        box_rect.size.width
    }
}

/// Background-position shift for fragment `i`'s `box-decoration-break: slice`
/// painting (css-break-3 §5.4.1): `-cumulative_block_extent` projected onto the
/// physical block-flow direction, so the image is painted as if on the unbroken
/// composite box. Horizontal-tb flows down (−y); vertical-rl/sideways-rl flow left
/// (+x toward the composite origin on the right); vertical-lr/sideways-lr flow right
/// (−x).
fn slice_bg_offset(wm: WritingMode, cum_block: f32) -> Vector {
    match wm {
        WritingMode::HorizontalTb => Vector::new(0.0, -cum_block),
        WritingMode::VerticalRl | WritingMode::SidewaysRl => Vector::new(cum_block, 0.0),
        WritingMode::VerticalLr | WritingMode::SidewaysLr => Vector::new(-cum_block, 0.0),
    }
}

/// Paint children in CSS 2.1 Appendix E stacking context layer order.
#[allow(clippy::similar_names)] // layer6 vs layers — intentional CSS layer numbering
fn paint_stacking_context_layers(
    ctx: &mut PaintContext,
    entity: Entity,
    children: &[Entity],
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let parent_display = elidex_layout_block::try_get_style(ctx.dom, entity).map(|s| s.display);
    let layers = collect_sc_participants(ctx.dom, children, parent_display);

    // Layer 2: negative z stacking contexts (z ascending).
    for &child in &layers.negative_z {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 3: in-flow non-positioned blocks (DOM order).
    for &child in &layers.in_flow_blocks {
        maybe_emit_list_marker(ctx, child);
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 4: non-positioned floats (DOM order).
    for &child in &layers.floats {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 5: inline content (DOM order, positioned excluded).
    {
        let mut inline_run = Vec::new();
        for &child in &layers.all_children {
            // Positioned children are painted in Layer 6/7, not here — skip them
            // WITHOUT flushing, so they do not split the inline run. This matches
            // `paint_non_sc` (which `continue`s on positioned) and layout's IFC
            // grouping (`stack_block_children` keeps inline content contiguous
            // across an out-of-flow sibling); a split here would diverge from the
            // run-start key layout persisted `InlineFlow` under and from layout's
            // line geometry (CSS 2 §9.2.1.1 anonymous block boxes).
            if is_positioned(ctx.dom, child) {
                continue;
            }
            if is_block_child(ctx.dom, child) || is_float_entity(ctx.dom, child) {
                if !inline_run.is_empty() {
                    emit_inline_run(
                        ctx,
                        entity,
                        &inline_run,
                        depth,
                        child_perspective,
                        in_transform,
                    );
                    inline_run.clear();
                }
            } else {
                inline_run.push(child);
            }
        }
        if !inline_run.is_empty() {
            emit_inline_run(
                ctx,
                entity,
                &inline_run,
                depth,
                child_perspective,
                in_transform,
            );
        }
    }

    // Layer 6: positioned auto + z:0 SC — DOM-order interleave.
    let mut layer6: Vec<Entity> = layers
        .positioned_auto
        .iter()
        .chain(layers.zero_z.iter())
        .copied()
        .collect();
    layer6.sort_by(|&a, &b| ctx.dom.tree_order_cmp(a, b));
    for &child in &layer6 {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 7: positive z stacking contexts (z ascending).
    for &child in &layers.positive_z {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }
}

/// Paint children of a non-SC element in DOM order, skipping positioned
/// children (they are painted by the parent stacking context).
///
/// The `in_transform` flag is propagated to all children so that
/// `position: fixed` descendants inside a transform ancestor are
/// correctly treated as absolute (CSS Transforms L1 §2).
fn paint_non_sc(
    ctx: &mut PaintContext,
    entity: Entity,
    children: &[Entity],
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let mut inline_run = Vec::new();

    for &child in children {
        // Skip positioned children — they're painted by the parent SC.
        if is_positioned(ctx.dom, child) {
            continue;
        }

        if is_block_child(ctx.dom, child) {
            // Flush any pending inline run before the block child.
            if !inline_run.is_empty() {
                emit_inline_run(
                    ctx,
                    entity,
                    &inline_run,
                    depth,
                    child_perspective,
                    in_transform,
                );
                inline_run.clear();
            }

            maybe_emit_list_marker(ctx, child);

            // Recurse into block child.
            walk(ctx, child, depth + 1, child_perspective, in_transform);
        } else {
            // Text node or inline element — add to current run.
            inline_run.push(child);
        }
    }

    // Flush trailing inline run.
    if !inline_run.is_empty() {
        emit_inline_run(
            ctx,
            entity,
            &inline_run,
            depth,
            child_perspective,
            in_transform,
        );
    }
}

/// Check whether a child entity is a block-level child.
///
/// Block children are recursed into separately; non-block children (text
/// nodes and inline elements) are collected into inline runs.
///
/// An entity is block-level if it has a `LayoutBox` AND a block-level
/// display type. Inline elements may also have a `LayoutBox` (assigned
/// during inline layout for background/border rendering) but should
/// still be treated as part of an inline run.
pub(crate) fn is_block_child(dom: &EcsDom, entity: Entity) -> bool {
    if dom.world().get::<&LayoutBox>(entity).is_err() {
        return false;
    }
    // Check display type — inline elements with LayoutBox are NOT block children.
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .is_some_and(|style| elidex_layout_block::block::is_block_level(style.display))
}

/// Walk a child entity, wrapping `position: fixed` (viewport-attached) elements
/// with `PopScrollOffset`/`PushScrollOffset` so they remain visually unscrolled.
///
/// The `PopScrollOffset`/`PushScrollOffset` pair must always be balanced:
/// both are emitted unconditionally when `is_viewport_fixed` is true,
/// and `walk()` never early-returns after the Pop has been emitted.
fn walk_child_with_fixed_check(
    ctx: &mut PaintContext,
    child: Entity,
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let is_fixed_vp = is_viewport_fixed(ctx.dom, child, in_transform);
    if is_fixed_vp {
        ctx.dl.push(DisplayItem::PopScrollOffset);
    }
    walk(ctx, child, depth + 1, child_perspective, in_transform);
    if is_fixed_vp {
        ctx.dl.push(DisplayItem::PushScrollOffset {
            scroll_offset: ctx.scroll_offset,
        });
    }
}

/// `position: fixed` with no transform ancestor → viewport-attached (scroll excluded).
///
/// CSS Transforms L1 §2: a transform ancestor establishes a containing block
/// for fixed descendants, so they scroll with the transform ancestor.
#[must_use]
fn is_viewport_fixed(dom: &EcsDom, entity: Entity, in_transform: bool) -> bool {
    if in_transform {
        return false;
    }
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .is_some_and(|s| s.position == elidex_plugin::Position::Fixed)
}

/// Emit a list marker for a block child if it is a `list-item` with a visible marker.
///
/// The marker text is resolved pre-layout (CSS Lists 3 §4.6 implicit `list-item`
/// counter, §4.7 `counter()` formatting) and stored in the [`ListItemMarker`]
/// component by `elidex-style`'s generated-content pass — the single source of
/// marker text. Render reads it here, owning only the `visibility` paint guard.
fn maybe_emit_list_marker(ctx: &mut PaintContext, child: Entity) {
    if let Ok(child_style) = ctx.dom.world().get::<&ComputedStyle>(child) {
        if child_style.display == Display::ListItem
            && child_style.list_style_type != ListStyleType::None
            && child_style.visibility == Visibility::Visible
        {
            let marker_text = match ctx.dom.world().get::<&ListItemMarker>(child) {
                Ok(m) => m.0.clone(),
                Err(_) => return,
            };
            if let Ok(child_lb) = ctx.dom.world().get::<&LayoutBox>(child) {
                emit_list_marker_with_counter(
                    &child_lb,
                    &child_style,
                    &marker_text,
                    ctx.font_db,
                    ctx.font_cache,
                    ctx.dl,
                );
            }
        }
    }
}

/// Check if a table cell is empty (CSS 2.1 §17.5.1).
///
/// A cell is considered empty if it has no children or all children are
/// whitespace-only text nodes.
fn is_cell_empty(dom: &EcsDom, entity: Entity) -> bool {
    let children: Vec<_> = dom.children_iter(entity).collect();
    if children.is_empty() {
        return true;
    }
    children.iter().all(|&child| {
        dom.world()
            .get::<&elidex_ecs::TextContent>(child)
            .is_ok_and(|text| text.0.trim().is_empty())
    })
}

#[cfg(test)]
mod c1_helper_tests {
    use super::{
        block_axis_extent, break_edges, slice_bg_offset, sliced_box, square_broken_corners,
    };
    use elidex_plugin::{BoxModel, EdgeSizes, Rect, Vector, WritingMode};

    #[test]
    fn break_edges_single_box_has_no_breaks() {
        assert_eq!(break_edges(0, 1, WritingMode::HorizontalTb), [false; 4]);
    }

    #[test]
    fn sliced_box_zeros_padding_and_border_on_broken_edges_only() {
        // A middle fragment (top+bottom broken, horizontal-tb). The block-axis
        // (top/bottom) padding + border are removed; the inline (left/right) keep theirs.
        let base = TestBox {
            content: Rect::new(0.0, 0.0, 100.0, 40.0),
            padding: EdgeSizes::new(5.0, 6.0, 7.0, 8.0),
            border: EdgeSizes::new(1.0, 2.0, 3.0, 4.0),
            margin: EdgeSizes::default(),
        };
        let s = sliced_box(&base, [true, false, true, false]);
        assert_eq!(
            (s.padding().top, s.padding().bottom),
            (0.0, 0.0),
            "broken block-axis padding removed (§5.4: no padding at a break)"
        );
        assert_eq!(
            (s.padding().left, s.padding().right),
            (8.0, 6.0),
            "inline padding preserved (not at a break)"
        );
        assert_eq!(
            (s.border().top, s.border().bottom),
            (0.0, 0.0),
            "broken block-axis border removed"
        );
        assert_eq!((s.border().left, s.border().right), (4.0, 2.0));
    }

    #[test]
    fn square_broken_corners_squares_corners_touching_a_break() {
        // top broken ⇒ both top corners square; bottom unbroken ⇒ stay rounded.
        assert_eq!(
            square_broken_corners([5.0, 6.0, 7.0, 8.0], [true, false, false, false]),
            [0.0, 0.0, 7.0, 8.0]
        );
        // top+bottom broken (a middle fragment) ⇒ all four corners square.
        assert_eq!(
            square_broken_corners([5.0, 6.0, 7.0, 8.0], [true, false, true, false]),
            [0.0; 4]
        );
        // no break ⇒ radii unchanged.
        assert_eq!(
            square_broken_corners([5.0, 6.0, 7.0, 8.0], [false; 4]),
            [5.0, 6.0, 7.0, 8.0]
        );
    }

    /// A minimal [`BoxModel`] for the sliced-geometry unit tests.
    struct TestBox {
        content: Rect,
        padding: EdgeSizes,
        border: EdgeSizes,
        margin: EdgeSizes,
    }
    impl BoxModel for TestBox {
        fn content(&self) -> Rect {
            self.content
        }
        fn padding(&self) -> EdgeSizes {
            self.padding
        }
        fn border(&self) -> EdgeSizes {
            self.border
        }
        fn margin(&self) -> EdgeSizes {
            self.margin
        }
    }

    #[test]
    fn break_edges_horizontal_omits_block_axis_at_breaks() {
        // [top, right, bottom, left]. horizontal-tb: block axis = top/bottom.
        // First fragment of 3: breaks at its block-END only (bottom).
        assert_eq!(
            break_edges(0, 3, WritingMode::HorizontalTb),
            [false, false, true, false]
        );
        // Middle: breaks at both block-START (top) and block-END (bottom).
        assert_eq!(
            break_edges(1, 3, WritingMode::HorizontalTb),
            [true, false, true, false]
        );
        // Last: breaks at its block-START only (top). Inline edges never break.
        assert_eq!(
            break_edges(2, 3, WritingMode::HorizontalTb),
            [true, false, false, false]
        );
    }

    #[test]
    fn break_edges_vertical_maps_block_axis_to_inline_physical_edges() {
        // vertical-rl: block-start = right, block-end = left.
        // First of 2 breaks at block-END (left).
        assert_eq!(
            break_edges(0, 2, WritingMode::VerticalRl),
            [false, false, false, true]
        );
        // Last of 2 breaks at block-START (right).
        assert_eq!(
            break_edges(1, 2, WritingMode::VerticalRl),
            [false, true, false, false]
        );
        // vertical-lr: block-start = left, block-end = right (mirror of -rl).
        assert_eq!(
            break_edges(0, 2, WritingMode::VerticalLr),
            [false, true, false, false]
        );
    }

    #[test]
    fn slice_bg_offset_projects_negative_block_flow_per_mode() {
        // horizontal flows down ⇒ shift up (−y).
        assert_eq!(
            slice_bg_offset(WritingMode::HorizontalTb, 40.0),
            Vector::new(0.0, -40.0)
        );
        // vertical-rl flows left, composite origin on the right ⇒ +x.
        assert_eq!(
            slice_bg_offset(WritingMode::VerticalRl, 40.0),
            Vector::new(40.0, 0.0)
        );
        // vertical-lr flows right ⇒ −x.
        assert_eq!(
            slice_bg_offset(WritingMode::VerticalLr, 40.0),
            Vector::new(-40.0, 0.0)
        );
    }

    #[test]
    fn block_axis_extent_is_height_horizontal_width_vertical() {
        let bb = Rect::new(0.0, 0.0, 100.0, 40.0);
        assert_eq!(block_axis_extent(bb, WritingMode::HorizontalTb), 40.0);
        assert_eq!(block_axis_extent(bb, WritingMode::VerticalRl), 100.0);
    }
}
