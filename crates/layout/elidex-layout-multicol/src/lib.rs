//! CSS Multi-column Layout Level 1 implementation.
//!
//! Computes column geometry, fills columns sequentially or with balanced
//! heights, handles `column-span: all` spanning elements, and attaches
//! [`MulticolInfo`] for column rule rendering.

pub mod algo;
mod fill;

use elidex_ecs::{EcsDom, Entity, InlineFlow};
use elidex_layout_block::inline::reposition_atomic_box;
use elidex_layout_block::{
    adjust_min_max_for_border_box, clamp_min_max, composed_children_flat, resolve_dimension_value,
    resolve_padding, sanitize_border, ChildLayoutFn, LayoutInput, LayoutOutcome,
};
use elidex_plugin::{
    BoxSizing, ColumnFill, ColumnSpan, ComputedStyle, CssSize, Dimension, Display, EdgeSizes,
    Float, LayoutBox, MulticolInfo, Point, Position, Rect, Size, Vector, WritingModeContext,
};

use algo::{compute_column_geometry, ColumnGeometry};
use fill::{fill_columns_balanced, fill_columns_sequential, ColumnFillEnv};

/// Segment of children between `column-span: all` elements.
enum Segment {
    /// Normal children to be laid out across columns.
    Normal(Vec<Entity>),
    /// A `column-span: all` element that spans the full container width.
    Spanner(Entity),
}

/// Shared layout context for column fill functions, reducing argument counts.
#[derive(Clone, Copy)]
pub(crate) struct ColumnLayoutCtx {
    pub(crate) content_origin: Point,
    pub(crate) block_offset: f32,
    pub(crate) wm: WritingModeContext,
}

/// Lay out a multicol container.
///
/// CSS Multi-column Layout Level 1: computes column geometry, fills columns
/// (balanced or sequential), handles `column-span: all`, and attaches
/// [`MulticolInfo`] for column rule rendering.
#[allow(clippy::too_many_lines)]
pub fn layout_multicol(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutOutcome {
    let style = elidex_layout_block::get_style(dom, entity);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);
    let is_horizontal = wm.is_horizontal();
    let font_db = input.font_db;
    let depth = input.depth;

    // --- Resolve box model ---
    let containing_inline = input.containing_inline_size;
    let padding = resolve_padding(&style, containing_inline);
    let border = sanitize_border(&style);
    let i_pb = elidex_layout_block::inline_pb(&wm, &padding, &border);

    let margin_top =
        elidex_layout_block::block::resolve_margin(style.margin_top, containing_inline);
    let margin_bottom =
        elidex_layout_block::block::resolve_margin(style.margin_bottom, containing_inline);
    let margin_left =
        elidex_layout_block::block::resolve_margin(style.margin_left, containing_inline);
    let margin_right =
        elidex_layout_block::block::resolve_margin(style.margin_right, containing_inline);

    let (margin_inline_start, margin_inline_end) = if is_horizontal {
        (margin_left, margin_right)
    } else {
        (margin_top, margin_bottom)
    };
    let (margin_block_start, margin_block_end) = if is_horizontal {
        (margin_top, margin_bottom)
    } else if wm.is_block_reversed() {
        (margin_right, margin_left)
    } else {
        (margin_left, margin_right)
    };

    let available_inline = if is_horizontal {
        input.containing.width
    } else {
        input.containing.height_or_width()
    };

    // --- Resolve inline-size ---
    let inline_size_dim = if is_horizontal {
        style.width
    } else {
        style.height
    };
    let inline_extra = margin_inline_start + margin_inline_end + i_pb;
    let auto_inline = (available_inline - inline_extra).max(0.0);
    let mut content_inline =
        resolve_dimension_value(inline_size_dim, available_inline, auto_inline).max(0.0);
    // box-sizing: border-box — subtract inline p+b from specified inline-size.
    if style.box_sizing == BoxSizing::BorderBox {
        if let Dimension::Length(_) | Dimension::Percentage(_) = inline_size_dim {
            content_inline = (content_inline - i_pb).max(0.0);
        }
    }
    // Apply min/max inline-size constraints.
    let (min_inline_dim, max_inline_dim) = if is_horizontal {
        (style.min_width, style.max_width)
    } else {
        (style.min_height, style.max_height)
    };
    {
        let mut min_i =
            elidex_layout_block::resolve_min_max(min_inline_dim, containing_inline, 0.0);
        let mut max_i =
            elidex_layout_block::resolve_min_max(max_inline_dim, containing_inline, f32::INFINITY);
        if style.box_sizing == BoxSizing::BorderBox {
            adjust_min_max_for_border_box(&mut min_i, &mut max_i, i_pb);
        }
        content_inline = clamp_min_max(content_inline, min_i, max_i);
    }

    // --- Content position ---
    let content_origin = if is_horizontal {
        Point::new(
            input.offset.x + margin_inline_start + border.left + padding.left,
            input.offset.y + margin_block_start + border.top + padding.top,
        )
    } else {
        let l_padding = elidex_plugin::LogicalEdges::from_physical(padding, wm);
        let l_border = elidex_plugin::LogicalEdges::from_physical(border, wm);
        Point::new(
            input.offset.x + margin_block_start + l_border.block_start + l_padding.block_start,
            input.offset.y + margin_inline_start + l_border.inline_start + l_padding.inline_start,
        )
    };

    // --- Column geometry ---
    let column_gap = resolve_dimension_value(style.column_gap, content_inline, 0.0).max(0.0);
    let geom = compute_column_geometry(
        content_inline,
        style.column_count,
        style.column_width,
        column_gap,
    );

    // --- Height determination ---
    let containing_block = if is_horizontal {
        input.containing.height
    } else {
        Some(input.containing.width)
    };
    let definite_height =
        resolve_definite_block_size(&style, containing_block, &padding, &border, wm);

    // Resolve max-block-size for balanced fill upper bound.
    let max_block_dim = if is_horizontal {
        style.max_height
    } else {
        style.max_width
    };
    let containing_for_minmax = if is_horizontal {
        input.containing.height_or_zero()
    } else {
        input.containing.width
    };
    let max_block_size =
        elidex_layout_block::resolve_min_max(max_block_dim, containing_for_minmax, f32::INFINITY);

    // --- Collect children and split into segments ---
    let children = composed_children_flat(dom, entity);
    let segments = split_segments(dom, &children);

    // Collect static positions for absolutely positioned children.
    let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
        dom,
        &children,
        content_origin,
    );

    // --- Lay out segments ---
    let mut block_cursor: f32 = 0.0;
    let mut multicol_segments: Vec<(u32, f32, f32)> = Vec::new();

    for segment in &segments {
        match segment {
            Segment::Normal(seg_children) => {
                if seg_children.is_empty() {
                    continue;
                }
                let col_ctx = ColumnLayoutCtx {
                    content_origin,
                    block_offset: block_cursor,
                    wm,
                };
                let balance_max = if max_block_size.is_finite() {
                    Some(max_block_size)
                } else {
                    None
                };
                let layout_env = elidex_layout_block::LayoutEnv {
                    font_db,
                    layout_child,
                    depth,
                    viewport: input.viewport,
                    layout_generation: input.layout_generation,
                    is_probe: input.is_probe,
                };
                let env = ColumnFillEnv {
                    input,
                    geom: &geom,
                    env: &layout_env,
                    parent_entity: entity,
                    col_ctx: &col_ctx,
                };
                let (frags, col_height) = layout_normal_segment(
                    dom,
                    seg_children,
                    &env,
                    definite_height,
                    balance_max,
                    &style,
                );

                // Position column fragments.
                #[allow(clippy::cast_possible_truncation)]
                let actual_count = frags.len().min(u32::MAX as usize) as u32;
                if actual_count > 0 {
                    multicol_segments.push((actual_count, block_cursor, col_height));
                }
                block_cursor += col_height;
            }
            Segment::Spanner(spanner_entity) => {
                // Lay out spanner at full container width.
                let (spanner_x, spanner_y) = if is_horizontal {
                    (content_origin.x, content_origin.y + block_cursor)
                } else {
                    (content_origin.x + block_cursor, content_origin.y)
                };
                let spanner_input = LayoutInput {
                    containing: CssSize {
                        width: if is_horizontal {
                            content_inline
                        } else {
                            input.containing.width
                        },
                        height: if is_horizontal {
                            input.containing.height
                        } else {
                            Some(content_inline)
                        },
                    },
                    containing_inline_size: content_inline,
                    offset: Point::new(spanner_x, spanner_y),
                    font_db: input.font_db,
                    depth: input.depth + 1,
                    float_ctx: None,
                    viewport: input.viewport,
                    fragmentainer: None,
                    break_token: None,
                    subgrid: None,
                    layout_generation: input.layout_generation,
                    is_probe: input.is_probe,
                };
                let outcome = layout_child(dom, *spanner_entity, &spanner_input);
                // Use margin_box() for spanner block extent.
                let mb = outcome.layout_box.margin_box();
                let spanner_block_extent = if is_horizontal {
                    mb.size.height
                } else {
                    mb.size.width
                };
                block_cursor += spanner_block_extent;
            }
        }
    }

    // --- Resolve final block size ---
    let content_block = definite_height.unwrap_or(block_cursor);

    // Apply min/max block-size constraints.
    let min_block_dim = if is_horizontal {
        style.min_height
    } else {
        style.min_width
    };
    let min_block = elidex_layout_block::resolve_min_max(min_block_dim, containing_for_minmax, 0.0);
    let final_block = clamp_min_max(content_block, min_block, max_block_size);

    // --- Build LayoutBox ---
    let (phys_width, phys_height) = if is_horizontal {
        (content_inline, final_block)
    } else {
        (final_block, content_inline)
    };

    let margin = if is_horizontal {
        EdgeSizes::new(
            margin_block_start,
            margin_inline_end,
            margin_block_end,
            margin_inline_start,
        )
    } else {
        let (phys_left, phys_right) = if wm.is_block_reversed() {
            (margin_block_end, margin_block_start)
        } else {
            (margin_block_start, margin_block_end)
        };
        EdgeSizes::new(
            margin_inline_start,
            phys_right,
            margin_inline_end,
            phys_left,
        )
    };

    let lb = LayoutBox {
        content: Rect::from_origin_size(content_origin, Size::new(phys_width, phys_height)),
        padding,
        border,
        margin,
        first_baseline: None,
        layout_generation: 0,
    };

    let _ = dom.world_mut().insert_one(entity, lb.clone());

    // Attach MulticolInfo for column rule rendering.
    let info = MulticolInfo {
        column_width: geom.width,
        column_gap: geom.gap,
        writing_mode: style.writing_mode,
        segments: multicol_segments,
    };
    let _ = dom.world_mut().insert_one(entity, info);

    // --- Layout positioned descendants ---
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != Position::Static || is_root || style.has_transform;
    if is_cb {
        let pb = lb.padding_box();
        let pos_env = elidex_layout_block::LayoutEnv {
            font_db,
            layout_child,
            depth,
            viewport: input.viewport,
            layout_generation: input.layout_generation,
            is_probe: input.is_probe,
        };
        elidex_layout_block::positioned::layout_positioned_children(
            dom,
            entity,
            &pb,
            &static_positions,
            &pos_env,
        );
    }

    // Clear any `ColumnFlowSlice` carrier left on the multicol container itself
    // (Codex PR#316 R1, nested-multicol carrier hygiene). The carrier is keyed on
    // the IFC `parent_entity`; `fill` drains it only when that entity is one of THIS
    // multicol's snapshotted direct mid-break children. When the IFC container IS
    // this multicol (direct inline content in a multicol — `parent_entity == entity`,
    // no block direct child to snapshot), `fill` never drains it, so it would
    // otherwise persist on `entity`. If `entity` is then an OUTER multicol's
    // mid-break direct child, the outer `fill` would drain this stale carrier and
    // fold THIS multicol's inner-column lines into an `InlineFlow` at the OUTER
    // column offset — breaking the "render never reads the carrier" invariant in
    // nested multicol. A multicol's own self-carrier is never consumable (the
    // direct-inline mid-break case is committed-next), so clear it here, after this
    // multicol's fill and before returning to any ancestor's `fill` drain.
    let _ = dom
        .world_mut()
        .remove_one::<elidex_ecs::ColumnFlowSlice>(entity);

    lb.into()
}

/// Lay out a normal (non-spanner) segment across columns.
///
/// Returns the column fragments and the resolved column height.
fn layout_normal_segment(
    dom: &mut EcsDom,
    children: &[Entity],
    env: &ColumnFillEnv<'_>,
    definite_height: Option<f32>,
    balance_max_height: Option<f32>,
    style: &ComputedStyle,
) -> (Vec<fill::ColumnFragment>, f32) {
    let fill_and_position =
        |dom: &mut EcsDom, frags: Vec<fill::ColumnFragment>| -> Vec<fill::ColumnFragment> {
            // `env.input.is_probe` is true iff an ANCESTOR multicol/intrinsic pass
            // is probing (this multicol's OWN balanced search probes never reach
            // `fill_and_position` — they call `fill_columns_sequential` directly,
            // the F1 own-probe invariant). When true, suppress the store write so
            // an ancestor probe leaves no garbage fragments (P1).
            position_column_fragments(
                dom,
                &frags,
                env.geom,
                env.col_ctx.wm,
                env.input.is_probe,
                env.input.layout_generation,
            );
            frags
        };

    match (definite_height, style.column_fill) {
        // Definite height + auto fill → sequential
        (Some(h), ColumnFill::Auto) => {
            let frags = fill_columns_sequential(dom, children, env, h, u32::MAX, false);
            let frags = fill_and_position(dom, frags);
            (frags, h)
        }
        // Definite height + balance → balanced with max_height
        (Some(h), ColumnFill::Balance) => {
            // Use min(definite_height, max-block-size) as balance upper bound.
            let max_h = balance_max_height.map_or(h, |m| h.min(m));
            let (frags, col_height) = fill_columns_balanced(dom, children, env, Some(max_h));
            let frags = fill_and_position(dom, frags);
            (frags, col_height.min(h))
        }
        // Auto height → always balance (CSS Multi-column L1 §7)
        (None, _) => {
            let (frags, col_height) = fill_columns_balanced(dom, children, env, balance_max_height);
            let frags = fill_and_position(dom, frags);
            (frags, col_height)
        }
    }
}

/// Position column fragment children by shifting them to their column's
/// physical position (writing-mode aware).
fn position_column_fragments(
    dom: &mut EcsDom,
    frags: &[fill::ColumnFragment],
    geom: &ColumnGeometry,
    wm: WritingModeContext,
    is_probe: bool,
    layout_generation: u32,
) {
    // THIS multicol's own mid-break children — their box fragments are committed
    // born-absolute below (`bf.content.origin += delta`), so the per-column shift
    // must NOT re-move them. Everything else in a column subtree (incl. a NESTED
    // multicol's own fragments, committed at this multicol's column-0 base) DOES
    // shift with the column delta.
    let own: std::collections::HashSet<Entity> = frags
        .iter()
        .flat_map(|f| f.box_snapshots.iter().map(|s| s.entity))
        .collect();
    // Axis the atomic reposition projects inline↔physical onto (C-2), from the
    // multicol element's own `wm` (`style.writing_mode`). This is the SAME axis the
    // existing per-column `InlineFlow` line-fold below uses (`inline_offset` added on
    // the inline axis), so the atomic path stays consistent with the text path. Both
    // assume the mid-break IFC's writing mode equals the multicol's — a pre-existing
    // shared assumption (the carried `inline_abs`/`block_abs` were folded with the
    // IFC's own `is_vertical` in-line); an orthogonal IFC writing mode is a
    // pre-existing limitation of this seam, not introduced or widened by C-2.
    let is_vertical = !wm.is_horizontal();
    // Every mid-break atomic (static + relpos) across ALL columns: its `LayoutBox`
    // is repositioned wholesale to a born-absolute target below (the folded
    // `inline_start` already carries the column inline offset), so its WHOLE subtree
    // must be PRUNED from the per-column generic shift — otherwise the column offset
    // is double-applied (root) and the content double-shifted (descendants). Global
    // (not per-column) because the per-column continuation re-lay can leave one
    // atomic in several columns' `frag.children`, so it must be pruned from every
    // column's shift, not just its own (C-2 §2.3).
    let atomic_exclude: std::collections::HashSet<Entity> = frags
        .iter()
        .flat_map(|f| f.box_snapshots.iter())
        .flat_map(|s| s.atomic_repositions.iter().map(|(e, _, _, _)| *e))
        .collect();
    // Drop any prior-lay store fragments for this multicol's direct children before
    // re-committing (Codex PR#321 R4-F4 + R6-F1): a same-pass definitive re-lay can
    // SHRINK an entity's span (3 columns → 2 — orphaned higher-column nodes), or
    // COLLAPSE it from spanning to whole-in-one-column (it then drops out of
    // `box_snapshots`/`own` entirely). `push_box`'s upsert can't clear either, and the
    // render router ORs over `fragments_for`, so a stale node paints a phantom column.
    // Removing every direct child (the superset of `own` + collapsed children;
    // `remove_entity` is a cheap no-op for the non-store ones) rebuilds each spanning
    // entity from scratch and de-indexes any that collapsed. Deeper (nested-multicol)
    // fragments are keyed on THEIR own children, not these direct children, so they are
    // untouched. (`!is_probe` only — a probe never commits, so it must not disturb the
    // live store.)
    if !is_probe {
        for frag in frags {
            for &child in &frag.children {
                dom.fragment_tree_mut().remove_entity(child);
            }
        }
    }
    // Per-run-start accumulator for the mid-break IFC lines (Z-1b, Option D): each
    // column's drained `flow_groups` are folded here, offset to the column's inline
    // position, then written as ONE `InlineFlow::single` per run-start AFTER the
    // column loop — so the per-column `shift_descendants_excluding_own_fragments`
    // below (which moves a persisted `InlineFlow`) runs while the run-start has NO
    // flow yet (cleared each column by the IFC's `clear_inline_flows`), making these
    // born-absolute lines excluded from the column shift by construction (timing).
    let mut flows: std::collections::HashMap<Entity, Vec<elidex_ecs::InlineFlowLine>> =
        std::collections::HashMap::new();
    for (i, frag) in frags.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let inline_offset = i as f32 * (geom.width + geom.gap);
        let delta = if wm.is_horizontal() {
            Vector::x_only(inline_offset)
        } else {
            Vector::y_only(inline_offset)
        };

        // Commit this column's spanning-child box fragments to the standalone
        // fragment tree (§15.4.1), offset to the column's inline position. This
        // is the ONLY store-write site, and `position_column_fragments` runs
        // ONLY via `fill_and_position` on the definitive pass (this multicol's
        // OWN balanced-fill probes call `fill_columns_sequential` directly, never
        // positioning), so it never accumulates THIS multicol's own probe garbage.
        //
        // P1 (Z-1b-0): the definitive-pass-only structure protects against a
        // multicol's OWN probes but NOT against an ANCESTOR's. A nested multicol
        // laid inside a balanced outer multicol's probe (or any intrinsic-sizing
        // probe) re-enters this child's full `layout_multicol` → `fill_and_position`,
        // which would write here even though the outer pass is throwaway. The
        // `is_probe` flag — set by `LayoutInput::probe` / the balanced search probe
        // and inherited by every child input — is true exactly then, so we skip the
        // commit and leave no ancestor-probe garbage. Column 0 commits at delta 0.
        if !is_probe {
            for snap in &frag.box_snapshots {
                // Box store (Z-1a box geometry; render fragment-walk consumes it,
                // C-1). `consumable` = this column drained an IFC line carrier
                // (`flow_groups` non-empty) ⟺ a direct-child IFC mid-break — the one
                // category render paints per-fragment (per-column chrome + clip +
                // content). A nested-block / deeper-IFC mid-break pushes box geometry
                // but no carrier (`flow_groups` empty) ⇒ `consumable = false` ⇒ render's
                // single `LayoutBox` arm (today's behavior). The store OR-latches the
                // flag across this entity's per-column pushes.
                let mut bf = snap.box_fragment.clone();
                bf.content.origin += delta;
                #[allow(clippy::cast_possible_truncation)]
                dom.fragment_tree_mut().push_box(
                    snap.entity,
                    i as u32,
                    bf,
                    !snap.flow_groups.is_empty(),
                );
                // IFC lines (Z-1b live): fold this column's per-run-start lines into
                // the accumulator, offset to the column's inline position. The line's
                // `inline_start` is the inline-axis-projected physical coord, so the
                // column offset (purely inline-axis) adds to it directly (block_start
                // — the cross-axis — is unchanged); this matches the box `delta`
                // (`x_only`/`y_only(inline_offset)`).
                for (run_start, lines) in &snap.flow_groups {
                    let entry = flows.entry(*run_start).or_default();
                    for line in lines {
                        let mut l = line.clone();
                        for run in &mut l.runs {
                            *run.inline_start_mut() += inline_offset;
                        }
                        entry.push(l);
                    }
                }
                // Reposition this column's mid-break atomics' `LayoutBox`es to their
                // per-column on-line positions (C-2). `!is_probe`-gated by this commit
                // block (definitive pass only) ⇒ probe-mutation-free; the atomics are
                // pruned from the per-column shift (`atomic_exclude`), so each is moved
                // exactly once, here, to its born-absolute target.
                reposition_midbreak_atomics(dom, snap, inline_offset, is_vertical);
            }
        }

        if i == 0 {
            continue; // Column 0's LayoutBoxes need no inline shift.
        }

        // Shift this column fragment's children (and descendants) to the column's inline
        // offset via block layout's canonical subtree shifter — which moves `LayoutBox`
        // AND a persisted `InlineFlow` (see its docstring). Slice 4 / I-multicol: before
        // this, multicol's own LayoutBox-only shifter left a converged whole-in-column
        // run's inline text painted at column 0. One-issue-one-way — reuse the single
        // project-wide shifter instead of a second InlineFlow-aware copy.
        //
        // EXCLUDING this multicol's OWN fragments: they are born-absolute (the
        // column offset was baked above at `bf.content.origin += delta`), so
        // re-shifting them here would double-apply it. But a NESTED multicol that
        // is whole-in-this-column has already committed ITS spanning-child
        // fragments at this multicol's column-0 base — those are NOT in `own`, so
        // they DO shift with the column delta (else they would paint back in
        // column 0 once consumed). Ancestor shifts of this whole subtree (relpos /
        // margin-collapse) move every fragment via `shift_descendants` (P2).
        //
        // `is_probe` (Z-1b-0.5): when an ANCESTOR pass is a throwaway probe this
        // re-runs over a subtree the prior definitive pass already persisted, so the
        // shifter must NOT move the persisted `InlineFlow` / box store (it is not
        // rebuilt by the probe). This mirrors the `push_box` suppression above
        // (`if !is_probe`) — the col-shift now honors the same flag the col-commit
        // already does, so a probe neither pushes nor shifts persisted render state.
        elidex_layout_block::block::shift_descendants_excluding_own_fragments(
            dom,
            &frag.children,
            delta,
            &own,
            &atomic_exclude,
            is_probe,
        );
    }

    // Build the mid-break run-starts' `InlineFlow` AFTER the column loop (the
    // double-shift guard, Z-1b Option D): each run-start now carries all columns'
    // lines at their baked per-column inline offsets, as ONE `InlineFlow::single`
    // (multicol columns coexist on one surface, discriminated by the baked offsets,
    // NOT by per-column generations — `InlineFragment` doc). `generation` =
    // `layout_generation` exactly like the whole-in-column `InlineFlow::single`
    // persist (`inline/mod.rs`): `0` on the screen path (so `emit_inline_flow`'s
    // `expected == None` paints the fragment's full line set = every column), or the
    // page number under paged media (so the flow gates to its page). The existing
    // `emit_inline_flow` consumes it unchanged.
    //
    // Built here (after the loop), NOT per column, is load-bearing for the
    // double-shift guard: the per-column `shift_descendants_excluding_own_fragments`
    // above shifts any `InlineFlow` it finds on a run-start in the subtree
    // (`shift.rs` InlineFlow arm — it does NOT honor the `own` box-fragment
    // exclusion, which is keyed on the container, not the run-start text node). The
    // run-start carries no flow during that loop because each column's IFC re-run
    // cleared it (`clear_inline_flows`; mid-break ⇒ not persisted ⇒ removed), and
    // this `insert_one` (replace) writes the freshly-baked lines last — so the
    // baked column offset is applied exactly once. (Moving this build INTO the loop
    // would let a column's flow be re-shifted by a later column's shift = double
    // offset; the assert below pins the cleared precondition.) `!is_probe`: `flows`
    // is empty under a probe (the commit block is guarded), so this is a no-op there.
    for (run_start, lines) in flows {
        debug_assert!(
            dom.world().get::<&InlineFlow>(run_start).is_err(),
            "mid-break run-start must carry no InlineFlow at build time (cleared each \
             column by clear_inline_flows) — else the per-column shift double-applied \
             the column offset before this build"
        );
        let _ = dom
            .world_mut()
            .insert_one(run_start, InlineFlow::single(layout_generation, lines));
    }
}

/// Reposition one column's mid-break atomics' `LayoutBox`es to their per-column
/// on-line positions (terminal-Z C-2), reusing the canonical
/// [`reposition_atomic_box`]. `inline_offset` is this column's inline position
/// (added to each atomic's carried column-0-base inline coord ⇒ born-absolute
/// target); `is_vertical` is the multicol element's own writing-mode axis. Called
/// only from the definitive-pass commit block (so it is probe-mutation-free), and
/// each atomic is pruned from the generic per-column shift (`atomic_exclude`), so
/// this is the SOLE mover of the atomic + its subtree.
///
/// One uniform loop over the carried records covers both **static** (also flow
/// members) and **relpos/sticky** (non-member) atomics — the seam moves every atomic
/// the same way. The block coord needs no column offset (columns share a block
/// range); the basis preserves any baked relative offset.
fn reposition_midbreak_atomics(
    dom: &mut EcsDom,
    snap: &fill::FragmentSnapshot,
    inline_offset: f32,
    is_vertical: bool,
) {
    for &(atomic, inline_abs, block_abs, basis) in &snap.atomic_repositions {
        reposition_atomic_box(
            dom,
            atomic,
            inline_abs + inline_offset,
            block_abs,
            is_vertical,
            Some(basis),
            false,
        );
    }
}

/// Split children into segments around `column-span: all` elements.
///
/// CSS Multi-column L1 §5: `column-span: all` only applies to in-flow
/// block-level elements. Out-of-flow and inline-level are excluded.
///
/// Absolutely/fixedly positioned children are excluded from column content
/// entirely — they are laid out separately via `layout_positioned_children`.
fn split_segments(dom: &EcsDom, children: &[Entity]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_normal: Vec<Entity> = Vec::new();

    for &child in children {
        let child_style = dom.world().get::<&ComputedStyle>(child).ok();

        // Skip display:none children — they generate no boxes.
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }

        // Skip absolutely/fixedly positioned children — they're out-of-flow.
        let is_out_of_flow = child_style
            .as_ref()
            .is_some_and(|s| matches!(s.position, Position::Absolute | Position::Fixed));
        if is_out_of_flow {
            continue;
        }

        let is_spanner = child_style.is_some_and(|style| {
            style.column_span == ColumnSpan::All
                && style.display != Display::Inline
                && style.float == Float::None
        });

        if is_spanner {
            if !current_normal.is_empty() {
                segments.push(Segment::Normal(std::mem::take(&mut current_normal)));
            }
            segments.push(Segment::Spanner(child));
        } else {
            current_normal.push(child);
        }
    }

    if !current_normal.is_empty() {
        segments.push(Segment::Normal(current_normal));
    }

    segments
}

/// Resolve a definite block-size for the multicol container.
///
/// Returns `Some(content_height)` for explicit `height`/`width` (depending on
/// writing mode), `None` for `auto`.
fn resolve_definite_block_size(
    style: &ComputedStyle,
    containing_block: Option<f32>,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    wm: WritingModeContext,
) -> Option<f32> {
    let block_size_dim = if wm.is_horizontal() {
        style.height
    } else {
        style.width
    };
    let b_pb = elidex_layout_block::block_pb(&wm, padding, border);

    match block_size_dim {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                Some((px - b_pb).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_block.map(|cb| {
            let resolved = cb * pct / 100.0;
            if style.box_sizing == BoxSizing::BorderBox {
                (resolved - b_pb).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
