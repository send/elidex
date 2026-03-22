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
    ComputedStyle, CssSize, Display, LayoutBox, PageSelector, PagedMediaContext, Position, Size,
};
use elidex_text::FontDatabase;

/// Dispatch child layout based on the element's display type.
///
/// This is the [`ChildLayoutFn`](elidex_layout_block::ChildLayoutFn) provided
/// to all layout algorithms, routing flex/grid containers to their respective
/// crates and everything else to block layout.
pub fn dispatch_layout_child(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    let style = elidex_layout_block::get_style(dom, entity);

    // CSS 2.1 §10.3.5: Inline-level containers (inline-flex, inline-grid,
    // inline-table) use shrink-to-fit width. Compute intrinsic sizes and
    // replace containing_width with shrink-to-fit.
    let adjusted_input;
    let effective_input = if matches!(
        style.display,
        Display::InlineBlock | Display::InlineFlex | Display::InlineGrid | Display::InlineTable
    ) && style.width == elidex_plugin::Dimension::Auto
    {
        let env = elidex_layout_block::LayoutEnv::from_input(input, dispatch_layout_child);
        let intrinsic = crate::intrinsic::compute_intrinsic_sizes(dom, entity, &env);
        let stf = crate::intrinsic::shrink_to_fit_width(&intrinsic, input.containing.width);
        adjusted_input = LayoutInput {
            containing: CssSize {
                width: stf,
                height: input.containing.height,
            },
            containing_inline_size: stf,
            ..*input
        };
        &adjusted_input
    } else {
        input
    };

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

    let outcome = match style.display {
        // display: contents — element generates no box (CSS Display Level 3 §2.8).
        // Children are promoted to the parent's formatting context via
        // flatten_contents(). Return a zero-size box at the given position.
        Display::Contents => LayoutBox {
            content: elidex_plugin::Rect::from_origin_size(input.offset, elidex_plugin::Size::ZERO),
            padding: elidex_plugin::EdgeSizes::default(),
            border: elidex_plugin::EdgeSizes::default(),
            margin: elidex_plugin::EdgeSizes::default(),
            first_baseline: None,
        }
        .into(),
        Display::Flex | Display::InlineFlex => {
            elidex_layout_flex::layout_flex(dom, entity, effective_input, dispatch_layout_child)
        }
        Display::Grid | Display::InlineGrid => {
            elidex_layout_grid::layout_grid(dom, entity, effective_input, dispatch_layout_child)
        }
        Display::Table | Display::InlineTable => {
            elidex_layout_table::layout_table(dom, entity, effective_input, dispatch_layout_child)
        }
        // CSS Multi-column L1: multicol containers have block display +
        // column-count/column-width. Check before falling through to block.
        _ if elidex_plugin::is_multicol(&style) => elidex_layout_multicol::layout_multicol(
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

    // CSS 2.1 §9.4.3: relative offset.
    // Return the original LayoutBox (without offset) so siblings use the
    // unshifted space. The ECS LayoutBox is updated with the offset.
    if style.position == Position::Relative {
        let lb = &outcome.layout_box;
        let mut offset_lb = lb.clone();
        positioned::apply_relative_offset(
            &mut offset_lb,
            &style,
            input.containing.width,
            input.containing.height,
        );
        let delta = offset_lb.content.origin - lb.content.origin;
        let _ = dom.world_mut().insert_one(entity, offset_lb);
        if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
            let children: Vec<_> = dom.children_iter(entity).collect();
            elidex_layout_block::block::shift_descendants(dom, &children, delta);
        }
    }

    outcome
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
/// Returns a two-pass result: if any margin box content references
/// `counter(pages)`, the total page count is known after the first pass.
#[must_use]
pub fn layout_paged(
    dom: &mut EcsDom,
    page_ctx: &PagedMediaContext,
    font_db: &FontDatabase,
) -> Vec<PageFragment> {
    let roots = find_roots(dom);
    if roots.is_empty() {
        return Vec::new();
    }

    // Use the first root for layout (typically the document root).
    let root = roots[0];

    let content_width = page_ctx.content_width();
    let content_height = page_ctx.content_height();

    if content_height <= 0.0 || content_width <= 0.0 {
        return Vec::new();
    }

    let frag_ctx = elidex_layout_block::FragmentainerContext {
        available_block_size: content_height,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };

    let input = LayoutInput {
        containing: CssSize::definite(content_width, content_height),
        containing_inline_size: content_width,
        offset: elidex_plugin::Point::new(page_ctx.page_margins.left, page_ctx.page_margins.top),
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(page_ctx.page_width, page_ctx.page_height)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };

    let fragments = layout_fragmented(dom, root, &input, frag_ctx);

    let mut pages = Vec::with_capacity(fragments.len());
    for (i, frag) in fragments.into_iter().enumerate() {
        let page_number = i + 1;
        let is_blank = is_blank_fragment(&frag);
        let matched = match_page_selectors(&page_ctx.page_rules, page_number, is_blank);
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

/// Collect all page selectors from `@page` rules that match a given page.
fn match_page_selectors(
    page_rules: &[elidex_plugin::PageRule],
    page_number: usize,
    is_blank: bool,
) -> Vec<PageSelector> {
    let mut matched = Vec::new();
    for rule in page_rules {
        for sel in &rule.selectors {
            if sel.matches(page_number, is_blank) && !matched.contains(sel) {
                matched.push(*sel);
            }
        }
    }
    matched
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
    let mut fragments = Vec::new();
    let mut current_token: Option<elidex_layout_block::BreakToken> = None;

    for _ in 0..MAX_FRAGMENTS {
        let frag_input = LayoutInput {
            fragmentainer: Some(&fragmentainer),
            break_token: current_token.as_ref(),
            ..*input
        };
        let mut outcome = dispatch_layout_child(dom, entity, &frag_input);
        current_token = outcome.break_token.take();
        let has_more = current_token.is_some();
        fragments.push(outcome);

        if !has_more {
            break;
        }
    }
    fragments
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
    let roots = find_roots(dom);
    for root in roots {
        layout_root(dom, root, viewport, font_db);
    }
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
        containing: CssSize::definite(viewport.width, viewport.height),
        containing_inline_size: viewport.width,
        offset: elidex_plugin::Point::ZERO,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(viewport),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
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
