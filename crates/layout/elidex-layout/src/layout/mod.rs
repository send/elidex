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
use elidex_plugin::{ComputedStyle, Display, LayoutBox, Position};
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
        let intrinsic = crate::intrinsic::compute_intrinsic_sizes(
            dom,
            entity,
            input.font_db,
            dispatch_layout_child,
            input.depth,
        );
        let stf = crate::intrinsic::shrink_to_fit_width(&intrinsic, input.containing_width);
        adjusted_input = LayoutInput {
            containing_width: stf,
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
            content: elidex_plugin::Rect::new(input.offset_x, input.offset_y, 0.0, 0.0),
            padding: elidex_plugin::EdgeSizes::default(),
            border: elidex_plugin::EdgeSizes::default(),
            margin: elidex_plugin::EdgeSizes::default(),
            first_baseline: None,
        }
        .into(),
        Display::Flex | Display::InlineFlex => {
            elidex_layout_flex::layout_flex(dom, entity, effective_input, dispatch_layout_child)
                .into()
        }
        Display::Grid | Display::InlineGrid => {
            elidex_layout_grid::layout_grid(dom, entity, effective_input, dispatch_layout_child)
                .into()
        }
        Display::Table | Display::InlineTable => {
            elidex_layout_table::layout_table(dom, entity, effective_input, dispatch_layout_child)
                .into()
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
            input.containing_width,
            input.containing_height,
        );
        let dx = offset_lb.content.x - lb.content.x;
        let dy = offset_lb.content.y - lb.content.y;
        let _ = dom.world_mut().insert_one(entity, offset_lb);
        if dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON {
            let children: Vec<_> = dom.children_iter(entity).collect();
            elidex_layout_block::block::shift_descendants(dom, &children, (dx, dy));
        }
    }

    outcome
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
pub fn layout_tree(
    dom: &mut EcsDom,
    viewport_width: f32,
    viewport_height: f32,
    font_db: &FontDatabase,
) {
    let roots = find_roots(dom);
    for root in roots {
        layout_root(dom, root, viewport_width, viewport_height, font_db);
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
fn layout_root(
    dom: &mut EcsDom,
    root: Entity,
    viewport_width: f32,
    viewport_height: f32,
    font_db: &FontDatabase,
) {
    let root_display = dom
        .world()
        .get::<&ComputedStyle>(root)
        .map(|s| s.display)
        .ok();

    let root_input = LayoutInput {
        containing_width: viewport_width,
        containing_height: Some(viewport_height),
        containing_inline_size: viewport_width,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some((viewport_width, viewport_height)),
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
