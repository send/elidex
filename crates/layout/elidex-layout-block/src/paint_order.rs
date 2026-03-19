//! CSS 2.1 Appendix E: stacking context paint order.
//!
//! Collects participants for a stacking context's 7-layer paint order
//! and handles z-index auto positioned element bubble-up.

use elidex_ecs::{EcsDom, Entity, MAX_ANCESTOR_DEPTH};
use elidex_plugin::{ComputedStyle, Display, Float, Position};

use crate::try_get_style;

// ---------------------------------------------------------------------------
// Stacking context layers
// ---------------------------------------------------------------------------

/// CSS 2.1 Appendix E: stacking context paint layers.
pub struct StackingContextLayers {
    /// Layer 2: negative z-index stacking contexts, sorted (z asc, tree order).
    pub negative_z: Vec<Entity>,
    /// Layer 3: in-flow non-positioned blocks (DOM order).
    pub in_flow_blocks: Vec<Entity>,
    /// Layer 4: non-positioned floats (DOM order).
    pub floats: Vec<Entity>,
    /// Layer 5: all children (for inline run reconstruction).
    pub all_children: Vec<Entity>,
    /// Layer 6: z-index: auto positioned (DOM order).
    pub positioned_auto: Vec<Entity>,
    /// Layer 6: z-index: 0 stacking contexts.
    pub zero_z: Vec<Entity>,
    /// Layer 7: positive z-index stacking contexts, sorted (z asc, tree order).
    pub positive_z: Vec<Entity>,
}

impl StackingContextLayers {
    fn new() -> Self {
        Self {
            negative_z: Vec::new(),
            in_flow_blocks: Vec::new(),
            floats: Vec::new(),
            all_children: Vec::new(),
            positioned_auto: Vec::new(),
            zero_z: Vec::new(),
            positive_z: Vec::new(),
        }
    }
}

/// Collect all participants for a stacking context's paint order.
///
/// Walks through z-index: auto positioned elements to collect bubbled-up descendants.
///
/// When `parent_display` is a flex or grid container, all layers use order-modified
/// document order (CSS Flexbox §5.4 / CSS Grid §8.3).
#[must_use]
pub fn collect_sc_participants(
    dom: &EcsDom,
    children: &[Entity],
    parent_display: Option<Display>,
) -> StackingContextLayers {
    let mut layers = StackingContextLayers::new();
    layers.all_children = children.to_vec();

    for &child in children {
        let Some(style) = try_get_style(dom, child) else {
            continue;
        };
        if style.display == Display::None {
            continue;
        }

        if style.position == Position::Static {
            // Non-positioned
            if style.float != Float::None {
                layers.floats.push(child);
            } else if crate::block::is_block_level(style.display) {
                layers.in_flow_blocks.push(child);
            }
            // Bubble up through non-SC static elements.
            if !style.creates_stacking_context() {
                bubble_up(dom, child, &mut layers, 0);
            }
        } else if style.creates_stacking_context() {
            classify_by_z(&style, child, &mut layers);
        } else {
            // z-index: auto → Layer 6
            layers.positioned_auto.push(child);
            bubble_up(dom, child, &mut layers, 0);
        }
    }

    // Sort by z-index (stable: tree order preserved for same z).
    layers.negative_z.sort_by_key(|&e| get_z_index(dom, e));
    layers.positive_z.sort_by_key(|&e| get_z_index(dom, e));

    // CSS Flexbox §5.4 / CSS Grid §8.3: order-modified document order.
    if matches!(
        parent_display,
        Some(Display::Flex | Display::InlineFlex | Display::Grid | Display::InlineGrid)
    ) {
        let order_key = |e: &Entity| get_order(dom, *e);
        layers.in_flow_blocks.sort_by_key(order_key);
        layers.floats.sort_by_key(order_key);
        layers.positioned_auto.sort_by_key(order_key);
        // zero_z: all have z-index=0, so sort by order only (stable sort preserves DOM order).
        layers.zero_z.sort_by_key(order_key);
        layers.negative_z.sort_by(|a, b| {
            get_z_index(dom, *a)
                .cmp(&get_z_index(dom, *b))
                .then_with(|| get_order(dom, *a).cmp(&get_order(dom, *b)))
        });
        layers.positive_z.sort_by(|a, b| {
            get_z_index(dom, *a)
                .cmp(&get_z_index(dom, *b))
                .then_with(|| get_order(dom, *a).cmp(&get_order(dom, *b)))
        });
    }

    layers
}

/// Get order for sorting (defaults to 0).
fn get_order(dom: &EcsDom, entity: Entity) -> i32 {
    try_get_style(dom, entity).map_or(0, |s| s.order)
}

/// Classify a stacking context entity by z-index into the appropriate layer.
fn classify_by_z(style: &ComputedStyle, entity: Entity, layers: &mut StackingContextLayers) {
    let z = style.z_index.unwrap_or(0);
    match z.cmp(&0) {
        std::cmp::Ordering::Less => layers.negative_z.push(entity),
        std::cmp::Ordering::Equal => layers.zero_z.push(entity),
        std::cmp::Ordering::Greater => layers.positive_z.push(entity),
    }
}

/// Recursively scan non-SC descendants to bubble up positioned elements.
///
/// Depth-limited to `MAX_ANCESTOR_DEPTH` to prevent stack overflow on
/// pathologically deep DOM trees.
fn bubble_up(dom: &EcsDom, entity: Entity, layers: &mut StackingContextLayers, depth: usize) {
    if depth >= MAX_ANCESTOR_DEPTH {
        return;
    }
    for child in dom.children_iter(entity) {
        let Some(style) = try_get_style(dom, child) else {
            continue;
        };
        if style.display == Display::None {
            continue;
        }
        if style.position != Position::Static {
            if style.creates_stacking_context() {
                classify_by_z(&style, child, layers);
                // SC isolates — don't descend further.
            } else {
                layers.positioned_auto.push(child);
                bubble_up(dom, child, layers, depth + 1);
            }
        } else if !style.creates_stacking_context() {
            bubble_up(dom, child, layers, depth + 1);
        }
    }
}

/// Get z-index for sorting (defaults to 0 for auto).
fn get_z_index(dom: &EcsDom, entity: Entity) -> i32 {
    try_get_style(dom, entity)
        .and_then(|s| s.z_index)
        .unwrap_or(0)
}

/// Check if an entity is positioned (CSS `position` is not `static`).
#[must_use]
pub fn is_positioned(dom: &EcsDom, entity: Entity) -> bool {
    try_get_style(dom, entity).is_some_and(|s| s.position != Position::Static)
}

/// Check if an entity is a float.
#[must_use]
pub fn is_float_entity(dom: &EcsDom, entity: Entity) -> bool {
    try_get_style(dom, entity).is_some_and(|s| s.float != Float::None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{ComputedStyle, Position};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    fn set_positioned(dom: &mut EcsDom, entity: Entity, pos: Position, z: Option<i32>) {
        let _ = dom.world_mut().insert_one(
            entity,
            ComputedStyle {
                display: Display::Block,
                position: pos,
                z_index: z,
                ..Default::default()
            },
        );
    }

    fn set_block(dom: &mut EcsDom, entity: Entity) {
        let _ = dom.world_mut().insert_one(
            entity,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
    }

    fn set_float(dom: &mut EcsDom, entity: Entity) {
        let _ = dom.world_mut().insert_one(
            entity,
            ComputedStyle {
                display: Display::Block,
                float: Float::Left,
                ..Default::default()
            },
        );
    }

    fn set_inline(dom: &mut EcsDom, entity: Entity) {
        let _ = dom.world_mut().insert_one(
            entity,
            ComputedStyle {
                display: Display::Inline,
                ..Default::default()
            },
        );
    }

    #[test]
    fn classify_static_block() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "div");
        dom.append_child(parent, child);
        set_block(&mut dom, child);

        let layers = collect_sc_participants(&dom, &[child], None);
        assert_eq!(layers.in_flow_blocks.len(), 1);
        assert!(layers.floats.is_empty());
        assert!(layers.positioned_auto.is_empty());
    }

    #[test]
    fn classify_static_float() {
        let mut dom = EcsDom::new();
        let child = elem(&mut dom, "div");
        set_float(&mut dom, child);

        let layers = collect_sc_participants(&dom, &[child], None);
        assert_eq!(layers.floats.len(), 1);
        assert!(layers.in_flow_blocks.is_empty());
    }

    #[test]
    fn classify_static_inline() {
        let mut dom = EcsDom::new();
        let child = elem(&mut dom, "span");
        set_inline(&mut dom, child);

        let layers = collect_sc_participants(&dom, &[child], None);
        // Inline is neither block nor float nor positioned.
        assert!(layers.in_flow_blocks.is_empty());
        assert!(layers.floats.is_empty());
        assert!(layers.positioned_auto.is_empty());
    }

    #[test]
    fn classify_positioned_negative_z() {
        let mut dom = EcsDom::new();
        let child = elem(&mut dom, "div");
        set_positioned(&mut dom, child, Position::Absolute, Some(-1));

        let layers = collect_sc_participants(&dom, &[child], None);
        assert_eq!(layers.negative_z.len(), 1);
    }

    #[test]
    fn classify_positioned_auto_z() {
        let mut dom = EcsDom::new();
        let child = elem(&mut dom, "div");
        set_positioned(&mut dom, child, Position::Relative, None);

        let layers = collect_sc_participants(&dom, &[child], None);
        assert_eq!(layers.positioned_auto.len(), 1);
    }

    #[test]
    fn classify_positioned_zero_z() {
        let mut dom = EcsDom::new();
        let child = elem(&mut dom, "div");
        set_positioned(&mut dom, child, Position::Absolute, Some(0));

        let layers = collect_sc_participants(&dom, &[child], None);
        assert_eq!(layers.zero_z.len(), 1);
    }

    #[test]
    fn classify_positioned_positive_z() {
        let mut dom = EcsDom::new();
        let child = elem(&mut dom, "div");
        set_positioned(&mut dom, child, Position::Fixed, Some(1));

        let layers = collect_sc_participants(&dom, &[child], None);
        assert_eq!(layers.positive_z.len(), 1);
    }

    #[test]
    fn bubble_up_through_auto() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let auto_pos = elem(&mut dom, "div");
        let inner_sc = elem(&mut dom, "div");
        dom.append_child(parent, auto_pos);
        dom.append_child(auto_pos, inner_sc);
        set_positioned(&mut dom, auto_pos, Position::Relative, None);
        set_positioned(&mut dom, inner_sc, Position::Absolute, Some(5));

        let layers = collect_sc_participants(&dom, &[auto_pos], None);
        // auto_pos is in positioned_auto
        assert_eq!(layers.positioned_auto.len(), 1);
        // inner_sc (z:5) bubbles up to positive_z
        assert_eq!(layers.positive_z.len(), 1);
        assert_eq!(layers.positive_z[0], inner_sc);
    }

    #[test]
    fn bubble_up_stops_at_sc() {
        let mut dom = EcsDom::new();
        let sc = elem(&mut dom, "div");
        let inner = elem(&mut dom, "div");
        dom.append_child(sc, inner);
        set_positioned(&mut dom, sc, Position::Absolute, Some(0));
        set_positioned(&mut dom, inner, Position::Absolute, Some(5));

        let layers = collect_sc_participants(&dom, &[sc], None);
        // sc (z:0) is in zero_z. inner is INSIDE sc's stacking context.
        assert_eq!(layers.zero_z.len(), 1);
        // inner should NOT bubble up.
        assert!(layers.positive_z.is_empty());
    }

    #[test]
    fn bubble_up_stops_at_non_positioned_sc() {
        let mut dom = EcsDom::new();
        let opacity_elem = elem(&mut dom, "div");
        let positioned_child = elem(&mut dom, "div");
        dom.append_child(opacity_elem, positioned_child);
        // opacity < 1.0 creates SC even without positioning
        let _ = dom.world_mut().insert_one(
            opacity_elem,
            ComputedStyle {
                display: Display::Block,
                opacity: 0.5,
                ..Default::default()
            },
        );
        set_positioned(&mut dom, positioned_child, Position::Absolute, Some(5));

        let layers = collect_sc_participants(&dom, &[opacity_elem], None);
        // opacity element creates SC → positioned child should NOT bubble up
        assert!(layers.positive_z.is_empty());
    }

    #[test]
    fn bubble_up_through_static_children() {
        let mut dom = EcsDom::new();
        let static_parent = elem(&mut dom, "div");
        let static_wrapper = elem(&mut dom, "div");
        let positioned_child = elem(&mut dom, "div");
        dom.append_child(static_parent, static_wrapper);
        dom.append_child(static_wrapper, positioned_child);
        set_block(&mut dom, static_parent);
        set_block(&mut dom, static_wrapper);
        set_positioned(&mut dom, positioned_child, Position::Absolute, Some(3));

        let layers = collect_sc_participants(&dom, &[static_parent], None);
        // positioned_child should bubble up through static_parent and static_wrapper
        assert_eq!(layers.positive_z.len(), 1);
        assert_eq!(layers.positive_z[0], positioned_child);
    }
}
