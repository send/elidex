//! Tree-level layout entry point.
//!
//! Walks the DOM tree and assigns [`LayoutBox`] components to each element.
//! The public API is [`layout_tree`], which takes a styled DOM and produces
//! layout boxes for the entire document.

mod anonymous_table;

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::block::stack_block_children;
use elidex_layout_block::positioned;
use elidex_layout_block::LayoutInput;
use elidex_plugin::{
    ComputedStyle, CssSize, Display, LayoutBox, PageSelector, Position, PreparedPages, Size,
};
use elidex_text::FontDatabase;

/// Dispatch child layout based on the element's display type.
///
/// This is the [`ChildLayoutFn`](elidex_layout_block::ChildLayoutFn) provided
/// to all layout algorithms, routing flex/grid containers to their respective
/// crates and everything else to block layout.
///
/// **Provenance (terminal-Z C-3a §2): this fn does NOT touch the phase.** An earlier
/// revision invalidated at this bracket to cover the `LayoutBox`-**component** half of
/// the seam's guard. That was the wrong site in *both* directions — **bypassable**,
/// because the algorithms below it (`layout_block_only`, `stack_block_children`,
/// `empty_container_box`, `layout_positioned_children`, the shifters, …) are `pub` and
/// are called directly cross-crate between the layout crates; and **over-eager**,
/// because it demoted on the `Display::Contents` arm, which writes neither geometry
/// source. Both are one root: the guard was not where the write is (Codex PR#488 R3+R4,
/// raising the two directions independently).
///
/// The guard now lives at [`EcsDom::set_layout_box`](elidex_ecs::EcsDom::set_layout_box)
/// / [`layout_box_mut`](elidex_ecs::EcsDom::layout_box_mut) — the write chokepoint every
/// producer goes through, enforced by the D4 trip-wire's wire #5. So the phase rule is
/// once again ONE sentence at ONE granularity: *any write to either geometry source
/// invalidates; only `layout_tree` publishes, at completion.*
///
/// A continuation in `input.break_token` belongs to the same immutable input.
/// Start a new run with empty progress after changing the DOM or computed style.
pub fn dispatch_layout_child(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    let style = elidex_layout_block::get_style(dom, entity);

    let mut input = input.resolve_item(&style);
    if input.depth == 0 && input.scroll_limits.x.is_none() {
        if let Some(size) = input.viewport {
            input.scroll_limits = input.scroll_limits.for_scrollport(
                &style,
                elidex_plugin::Rect::from_origin_size(elidex_plugin::Point::ZERO, size),
                true,
            );
        }
    }
    if !input.scroll_limits.measuring
        && (style.overflow_x.allows_programmatic_scroll()
            || style.overflow_y.allows_programmatic_scroll())
    {
        // Establish the scroll origin from this pass, before descendants are placed.
        let mut probe = input;
        probe.scroll_limits.measuring = true;
        probe.is_probe = true;
        probe.layout_generation = 0;
        let frame = dispatch_inner(dom, entity, &probe, &style)
            .layout_box
            .padding_box();
        input.scroll_limits = input.scroll_limits.for_scrollport(&style, frame, false);
    }
    dispatch_inner(dom, entity, &input, &style)
}

fn shrink_to_fit_input<'a>(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'a>,
    style: &ComputedStyle,
) -> Option<LayoutInput<'a>> {
    // CSS 2.1 §10.3.5: Inline-level containers (inline-flex, inline-grid,
    // inline-table) shrink to fit on their own inline axis.
    let horizontal = style.writing_mode.is_horizontal();
    let (inline_size, assigned_inline, available_inline) = if horizontal {
        (
            style.width,
            input.assigned.content.width,
            input.containing.width,
        )
    } else {
        (
            style.height,
            input.assigned.content.height,
            input
                .available_height()
                .unwrap_or(input.containing_inline_size),
        )
    };
    if matches!(
        style.display,
        Display::InlineBlock | Display::InlineFlex | Display::InlineGrid | Display::InlineTable
    ) && matches!(input.assigned.fit, elidex_layout_block::FitTarget::None)
        && assigned_inline.is_none()
        && inline_size == elidex_plugin::Dimension::Auto
    {
        let env = elidex_layout_block::LayoutEnv::from_input(input, dispatch_layout_child);
        let inner = elidex_layout_block::intrinsic::inner_intrinsic_sizes(
            dom,
            entity,
            &env,
            elidex_layout_grid::compute_grid_intrinsic,
        );
        let mut fitted = *input;
        let extent =
            elidex_layout_block::block::fit_inline_content(style, input, available_inline, &inner);
        fitted.assigned.fit = elidex_layout_block::FitTarget::ShrinkToFit(Size {
            width: horizontal.then_some(extent),
            height: (!horizontal).then_some(extent),
        });
        Some(fitted)
    } else {
        None
    }
}

fn dispatch_inner(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    style: &ComputedStyle,
) -> elidex_layout_block::LayoutOutcome {
    let adjusted_input = shrink_to_fit_input(dom, entity, input, style);
    let effective_input = adjusted_input.as_ref().unwrap_or(input);

    // CSS 2.1 §17.2.1: wrap orphan table-internal children in anonymous table
    // wrappers before block layout. Only needed for block-level containers that
    // are not themselves table elements.
    if matches!(
        style.display,
        Display::Block
            | Display::ListItem
            | Display::Flex
            | Display::InlineFlex
            | Display::Grid
            | Display::InlineGrid
    ) {
        anonymous_table::ensure_table_wrappers(dom, entity);
    }

    #[cfg(test)]
    if !input.is_probe && tests::TRACK_CHILD.get() == Some(entity) {
        tests::TRACK_WIDTH.set(Some(input.containing.width));
    }
    let mut outcome = match style.display {
        // display: contents — element generates no box (CSS Display Level 3 §2.8).
        // Children are promoted to the parent's formatting context via
        // flatten_contents(). Return a zero-size box at the given position.
        Display::Contents => LayoutBox {
            content: elidex_plugin::Rect::from_origin_size(input.offset, elidex_plugin::Size::ZERO),
            padding: elidex_plugin::EdgeSizes::default(),
            border: elidex_plugin::EdgeSizes::default(),
            margin: elidex_plugin::EdgeSizes::default(),
            first_baseline: None,
            layout_generation: 0,
        }
        .into(),
        Display::Flex | Display::InlineFlex => {
            let formatter = elidex_layout_flex::layout_flex;
            #[cfg(test)]
            tests::record_flex_dispatch(
                entity,
                input,
                effective_input,
                std::any::type_name_of_val(&formatter),
            );
            formatter(dom, entity, effective_input, dispatch_layout_child)
        }
        Display::Grid | Display::InlineGrid => {
            elidex_layout_grid::layout_grid(dom, entity, effective_input, dispatch_layout_child)
        }
        Display::Table | Display::InlineTable => {
            elidex_layout_table::layout_table(dom, entity, effective_input, dispatch_layout_child)
        }
        // CSS Multi-column L1: multicol containers have block display +
        // column-count/column-width. Check before falling through to block.
        _ if elidex_plugin::is_multicol(style) => elidex_layout_multicol::layout_multicol(
            dom,
            entity,
            effective_input,
            dispatch_layout_child,
        ),
        _ => elidex_layout_block::block::layout_block_inner(
            dom,
            entity,
            effective_input,
            dispatch_layout_child,
        ),
    };

    // Stamp layout_generation on the entity's LayoutBox for per-fragment
    // paged media rendering. The walk skips entities whose generation
    // doesn't match the current page. Non-paged paths use generation=0.
    if input.layout_generation > 0 {
        outcome.layout_box.layout_generation = input.layout_generation;
        if let Some(mut lb) = dom.layout_box_mut(entity) {
            lb.layout_generation = input.layout_generation;
        }
    }

    if style.position == Position::Relative {
        publish_relative_offset(dom, entity, input, style, &outcome.layout_box);
    }
    outcome
}

/// CSS 2.1 §9.4.3. Publish the offset box while preserving the normal-flow result.
fn publish_relative_offset(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    style: &ComputedStyle,
    lb: &LayoutBox,
) {
    let mut offset_lb = lb.clone();
    positioned::apply_relative_offset(
        &mut offset_lb,
        style,
        input.containing.width,
        input.containing.height,
    );
    let delta = offset_lb.content.origin - lb.content.origin;
    dom.set_layout_box(entity, offset_lb);
    if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
        let children: Vec<_> = dom.children_iter(entity).collect();
        elidex_layout_block::block::shift_descendants(dom, &children, delta, input.is_probe);
    }
}

/// Result of laying out a single page fragment in paged media.
#[derive(Clone, Debug)]
pub struct PageFragment {
    /// The layout box for this page's content.
    pub layout_box: LayoutBox,
    /// 1-based page number.
    pub page_number: usize,
    /// Page selectors that matched this page (from `@page` rules).
    pub matched_selectors: Vec<PageSelector>,
    /// Whether this page is intentionally blank (from a forced break).
    pub is_blank: bool,
}

/// Layout content for paged media, producing one [`PageFragment`] per page.
///
/// CSS Paged Media Level 3: fragments content across pages using the
/// fragmentation engine. Each page's content area is determined by the
/// page size minus margins. Page selectors (`:first`, `:left`, `:right`,
/// `:blank`) are matched for each page to apply page-specific rules.
///
/// The design plan specifies a 2-pass approach, but 1-pass suffices:
/// content fragmentation determines the total page count, and
/// `counter(pages)` is resolved at render time in `build_paged_display_lists`
/// where the total is already known. A second layout pass would only be
/// needed if margin box content could affect page breaks, which the spec
/// does not allow (CSS Paged Media L3 §4.2).
#[must_use]
pub fn layout_paged(
    dom: &mut EcsDom,
    page_ctx: &PreparedPages,
    font_db: &FontDatabase,
) -> Vec<PageFragment> {
    let roots = find_roots(dom);
    if roots.is_empty() {
        return Vec::new();
    }

    // Use the first root for layout (typically the document root).
    let root = roots[0];

    let area = page_ctx.select(1, false).geometry.area();
    let content_width = area.size.width;
    let content_height = area.size.height;

    if content_height <= 0.0 || content_width <= 0.0 {
        return Vec::new();
    }

    let frag_ctx = elidex_layout_block::FragmentainerContext {
        available_block_size: content_height,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };

    let input = LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        item_context: elidex_layout_block::ItemContext::NormalFlow,
        assigned: elidex_layout_block::UsedAssignment::default(),
        containing: CssSize::definite(content_width, content_height),
        containing_inline_size: content_width,
        offset: area.origin,
        font_db,
        depth: 0,
        float_ctx: None,
        scroll_limits: elidex_layout_block::ScrollLimits::default().for_scrollport(
            &elidex_layout_block::get_style(dom, root),
            area,
            true,
        ),
        viewport: Some(area.size),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        medium: elidex_plugin::Medium::Print,
        placement: elidex_layout_block::FragmentPlacement::Normal,
        is_probe: false,
    };

    let fragments = layout_fragmented(dom, root, &input, frag_ctx);

    let mut pages = Vec::with_capacity(fragments.len());
    for (i, frag) in fragments.into_iter().enumerate() {
        let page_number = i + 1;
        let is_blank = is_blank_fragment(&frag);
        let matched = page_ctx
            .select(page_number, is_blank)
            .matched_selectors
            .clone();
        pages.push(PageFragment {
            layout_box: frag.layout_box,
            page_number,
            matched_selectors: matched,
            is_blank,
        });
    }

    pages
}

/// Check whether a layout outcome represents a blank page.
///
/// A blank page has zero or near-zero content height, typically produced
/// by a forced page break with no content between breaks.
fn is_blank_fragment(outcome: &elidex_layout_block::LayoutOutcome) -> bool {
    outcome.layout_box.content.size.height < 0.5 && outcome.layout_box.content.size.width < 0.5
}

/// Maximum number of fragments to prevent infinite loops.
pub const MAX_FRAGMENTS: usize = 1000;

/// Lay out an element across multiple fragmentainers.
///
/// Returns a `Vec` of `LayoutOutcome`, one per fragment. Each fragment's
/// `layout_box` represents the portion of the element in that fragmentainer.
pub fn layout_fragmented(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    fragmentainer: elidex_layout_block::FragmentainerContext,
) -> Vec<elidex_layout_block::LayoutOutcome> {
    layout_fragmented_with_tokens(dom, entity, input, fragmentainer).0
}

/// Lay out one fixed-geometry page, including the CSS Break 3 §4.4 last resort.
pub fn layout_page_fragment(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    use elidex_layout_block::{source_progress::fragment_advanced, FragmentEnd, FragmentPlacement};
    let made_progress = |dom: &EcsDom, outcome: &elidex_layout_block::LayoutOutcome| {
        outcome.end.token().is_none_or(|next| {
            fragment_advanced(
                dom,
                input.break_token,
                next,
                &outcome.contribution,
                elidex_layout_block::FragmentationType::Page,
                outcome.propagated_break_before,
                None,
            )
        })
    };
    let first = dispatch_layout_child(dom, entity, input);
    if made_progress(dom, &first) {
        return first;
    }
    let retry = LayoutInput {
        placement: FragmentPlacement::FreshPageFallback,
        ..*input
    };
    let mut outcome = dispatch_layout_child(dom, entity, &retry);
    if !made_progress(dom, &outcome) {
        outcome.end = FragmentEnd::Incomplete;
    }
    outcome
}

/// Like [`layout_fragmented`], but also returns the break token chain.
///
/// Each entry in `tokens[i]` is the break token that was used as INPUT to
/// produce fragment `i` (i.e., `tokens[0] = None` for the first page).
/// The chain is needed for Phase 2 of interleaved paged media rendering:
/// each token is replayed to re-layout the corresponding page fragment.
pub fn layout_fragmented_with_tokens(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    fragmentainer: elidex_layout_block::FragmentainerContext,
) -> (
    Vec<elidex_layout_block::LayoutOutcome>,
    Vec<Option<elidex_layout_block::BreakToken>>,
) {
    // Provenance (terminal-Z C-3a §2): NO explicit mark here, and none needed. Whatever
    // this pass writes invalidates itself — `LayoutBox` through
    // `EcsDom::set_layout_box`/`layout_box_mut`, store contents through a `FragmentTree`
    // mutator — and nothing on this path publishes. A paged attempt that writes NOTHING
    // (the early returns in `layout_paged`) correctly leaves a published screen pass
    // standing; an earlier revision marked this entry unconditionally and got that wrong,
    // which is what led a reviewer to ask for the mark to be pushed even earlier (see
    // `a_zero_write_paged_early_return_leaves_the_phase_alone`).
    let mut fragments = Vec::new();
    let mut tokens = Vec::new();
    let mut current_token: Option<elidex_layout_block::BreakToken> = None;

    for _ in 0..MAX_FRAGMENTS {
        tokens.push(current_token.clone());
        let frag_input = LayoutInput {
            fragmentainer: Some(&fragmentainer),
            break_token: current_token.as_ref(),
            ..*input
        };
        let outcome = if matches!(
            fragmentainer.fragmentation_type,
            elidex_layout_block::FragmentationType::Page
        ) {
            layout_page_fragment(dom, entity, &frag_input)
        } else {
            dispatch_layout_child(dom, entity, &frag_input)
        };
        current_token = outcome.end.token().cloned();
        let has_more = current_token.is_some();
        fragments.push(outcome);

        if !has_more {
            break;
        }
    }
    (fragments, tokens)
}

/// Layout the entire DOM tree.
///
/// Each element that participates in layout receives a [`LayoutBox`] ECS
/// component. Elements with `display: none` are skipped entirely.
///
/// # Prerequisites
///
/// `elidex_style::resolve_styles()` must have been called first so that
/// every element has a [`ComputedStyle`] component.
pub fn layout_tree(dom: &mut EcsDom, viewport: Size, font_db: &FontDatabase) {
    // The standalone fragment tree (§15.4.1) is layout output, rebuilt from
    // scratch each pass — clear it before the full-from-root relayout (the
    // rebuild is the reconcile; no incremental / staleness model). Z-1a's only
    // populator is multicol box fragments, and this screen pass is the only path
    // the Z-1b render fragment-walk consumer reads; screen render always runs
    // `layout_tree` first (shell pipeline), so it sees a freshly-rebuilt tree.
    // The paged render path (`build_paged_display_lists_interleaved` →
    // `layout_fragmented_with_tokens` + per-page `dispatch_layout_child`) does
    // NOT clear here and may leave incidental dark fragments; folding paged media
    // into the store (and its hygiene) is committed-next, per the
    // `FragmentNode::fragmentainer` docstring.
    // Provenance (terminal-Z C-3a §2): a stale `CompletedScreen` from a prior pass must
    // not be readable while this pass's store is empty/partial (the re-entrant-screen
    // soundness hole). `screen_layout_pass` owns the whole window — it clears (which
    // invalidates by construction: an emptied store is definitionally not a completed
    // pass), runs the closure, and publishes at completion. There is deliberately no
    // entry mark here or on the paged entry: the phase is driven by the WRITERS, under
    // one rule — any write to either geometry source invalidates (`clear()` + the
    // `FragmentTree` mutators for the store, `EcsDom::set_layout_box`/`layout_box_mut`
    // for the `LayoutBox` component). Probes run inside this window, so they read
    // `Invalid`. The bracket is also the ONLY path that can publish, since
    // `FragmentTree::publish_completed_screen` is `pub(crate)` to `elidex-ecs`
    // (Codex PR#488 R6).
    dom.screen_layout_pass(|dom| {
        let roots = find_roots(dom);
        for root in roots {
            layout_root(dom, root, viewport, font_db);
        }
    });
}

/// Find root entities for layout: parentless entities with styles or children.
fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    dom.root_entities()
        .into_iter()
        .filter(|&e| {
            dom.world().get::<&ComputedStyle>(e).is_ok() || dom.get_first_child(e).is_some()
        })
        .collect()
}

/// Layout starting from a root entity.
///
/// If the root has a `ComputedStyle` (is an element), layout it directly
/// via the display-type dispatcher. Otherwise (document root), layout its
/// children as block-level elements.
fn layout_root(dom: &mut EcsDom, root: Entity, viewport: Size, font_db: &FontDatabase) {
    let root_display = dom
        .world()
        .get::<&ComputedStyle>(root)
        .map(|s| s.display)
        .ok();

    let root_input = LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        item_context: elidex_layout_block::ItemContext::NormalFlow,
        assigned: elidex_layout_block::UsedAssignment::default(),
        containing: CssSize::definite(viewport.width, viewport.height),
        containing_inline_size: viewport.width,
        offset: elidex_plugin::Point::ZERO,
        font_db,
        depth: 0,
        float_ctx: None,
        scroll_limits: elidex_layout_block::ScrollLimits::default().for_scrollport(
            &elidex_layout_block::get_style(dom, root),
            elidex_plugin::Rect::from_origin_size(elidex_plugin::Point::ZERO, viewport),
            true,
        ),
        viewport: Some(viewport),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        medium: elidex_plugin::Medium::Screen,
        placement: elidex_layout_block::FragmentPlacement::Normal,
        is_probe: false,
    };

    if let Some(display) = root_display {
        if display == Display::None {
            return;
        }
        if display == Display::Contents {
            // display: contents at root — skip box, layout children directly.
            let children = elidex_layout_block::composed_children_flat(dom, root);
            // Root-level always establishes a BFC.
            let _ = stack_block_children(
                dom,
                &children,
                &root_input,
                dispatch_layout_child,
                true,
                root,
            );
            return;
        }
        dispatch_layout_child(dom, root, &root_input);
        return;
    }

    // Document root: layout children as top-level blocks with margin collapse.
    // Root always establishes a BFC.
    let children = elidex_layout_block::composed_children_flat(dom, root);
    let _ = stack_block_children(
        dom,
        &children,
        &root_input,
        dispatch_layout_child,
        true,
        root,
    );
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
