//! Slot distribution algorithm for Shadow DOM.
//!
//! Distributes light DOM children of a shadow host to `<slot>` elements
//! in the shadow tree according to the WHATWG DOM spec §4.2.2.3.

use std::collections::HashMap;

use elidex_ecs::{
    Attributes, EcsDom, Entity, ShadowHost, SlotAssignment, SlottedMarker, MAX_ANCESTOR_DEPTH,
};

/// Distribute light DOM children of `host` to `<slot>` elements in its shadow tree.
///
/// Algorithm:
/// 1. Find the shadow root from the host's `ShadowHost` component.
/// 2. Collect all `<slot>` elements in the shadow tree (tree order).
/// 3. For each light DOM child of the host (skipping the shadow root entity):
///    - If it has `slot="name"` → assign to the named slot.
///    - Otherwise → assign to the default slot (name="" or no name attribute).
/// 4. Attach `SlotAssignment` to each `<slot>` entity.
pub fn distribute_slots(dom: &mut EcsDom, host: Entity) {
    // Get shadow root.
    let shadow_root = match dom.world().get::<&ShadowHost>(host) {
        Ok(sh) => sh.shadow_root,
        Err(_) => return,
    };

    // B5: Clear stale SlottedMarker from previous distribution.
    let light_children = dom.children(host);
    for child in light_children {
        let _ = dom.world_mut().remove_one::<SlottedMarker>(child);
    }

    // Collect all <slot> elements in the shadow tree (pre-order).
    let mut slots: Vec<(Entity, String)> = Vec::new();
    collect_slots(dom, shadow_root, &mut slots, 0);

    // Build name → slot entity map. First slot with a given name wins.
    // Default slot is name="" (empty string).
    let mut default_slot: Option<Entity> = None;
    let mut named_slots: HashMap<&str, Entity> = HashMap::new();

    for (slot_entity, name) in &slots {
        if name.is_empty() {
            if default_slot.is_none() {
                default_slot = Some(*slot_entity);
            }
        } else {
            named_slots.entry(name).or_insert(*slot_entity);
        }
    }

    // Initialize assignments (HashMap for O(1) lookup).
    let mut assignments: HashMap<Entity, Vec<Entity>> = slots
        .iter()
        .map(|(entity, _)| (*entity, Vec::new()))
        .collect();

    // Distribute light DOM children.
    let children = dom.children(host);
    for child in children {
        let target_slot = if let Ok(attrs) = dom.world().get::<&Attributes>(child) {
            match attrs.get("slot") {
                Some(name) if !name.is_empty() => named_slots.get(name).copied(),
                _ => default_slot,
            }
        } else {
            default_slot
        };

        if let Some(slot_entity) = target_slot {
            if let Some(nodes) = assignments.get_mut(&slot_entity) {
                nodes.push(child);
            }
        }
    }

    // Attach SlotAssignment to each slot entity, and SlottedMarker to assigned nodes.
    for (slot_entity, assigned_nodes) in assignments {
        for &node in &assigned_nodes {
            let _ = dom.world_mut().insert_one(node, SlottedMarker);
        }
        let _ = dom
            .world_mut()
            .insert_one(slot_entity, SlotAssignment { assigned_nodes });
    }
}

/// Recursively collect all `<slot>` elements in the shadow tree (pre-order).
///
/// Recursion is capped at `MAX_ANCESTOR_DEPTH` to prevent stack overflow.
fn collect_slots(dom: &EcsDom, entity: Entity, result: &mut Vec<(Entity, String)>, depth: usize) {
    if depth > MAX_ANCESTOR_DEPTH {
        return;
    }
    if dom.has_tag(entity, "slot") {
        let name = dom
            .world()
            .get::<&Attributes>(entity)
            .ok()
            .and_then(|attrs| attrs.get("name").map(str::to_string))
            .unwrap_or_default();
        result.push((entity, name));
    }

    for child in dom.children(entity) {
        collect_slots(dom, child, result, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom, ShadowRootMode};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        elem_with_attrs(dom, tag, &[])
    }

    fn elem_with_attrs(dom: &mut EcsDom, tag: &str, attrs: &[(&str, &str)]) -> Entity {
        let mut a = Attributes::default();
        for (k, v) in attrs {
            a.set(*k, *v);
        }
        dom.create_element(tag, a)
    }

    #[test]
    fn named_slot_distribution() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let light1 = elem_with_attrs(&mut dom, "span", &[("slot", "header")]);
        let light2 = elem(&mut dom, "p");
        let _ = dom.append_child(host, light1);
        let _ = dom.append_child(host, light2);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let named_slot = elem_with_attrs(&mut dom, "slot", &[("name", "header")]);
        let default_slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, named_slot);
        let _ = dom.append_child(sr, default_slot);

        distribute_slots(&mut dom, host);

        let named_assign = dom.world().get::<&SlotAssignment>(named_slot).unwrap();
        assert_eq!(named_assign.assigned_nodes, vec![light1]);

        let default_assign = dom.world().get::<&SlotAssignment>(default_slot).unwrap();
        assert_eq!(default_assign.assigned_nodes, vec![light2]);
    }

    #[test]
    fn default_slot_collects_unslotted() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let c1 = elem(&mut dom, "span");
        let c2 = elem(&mut dom, "p");
        let _ = dom.append_child(host, c1);
        let _ = dom.append_child(host, c2);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, slot);

        distribute_slots(&mut dom, host);

        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert_eq!(assign.assigned_nodes, vec![c1, c2]);
    }

    #[test]
    fn fallback_content_when_no_assigned_nodes() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        // No light DOM children.

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem(&mut dom, "slot");
        let fallback = dom.create_text("fallback");
        let _ = dom.append_child(sr, slot);
        let _ = dom.append_child(slot, fallback);

        distribute_slots(&mut dom, host);

        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert!(assign.assigned_nodes.is_empty());

        // composed_children should return fallback.
        let composed = dom.composed_children(slot);
        assert_eq!(composed, vec![fallback]);
    }

    #[test]
    fn shadow_root_entity_not_distributed() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let light = elem(&mut dom, "span");
        let _ = dom.append_child(host, light);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, slot);

        distribute_slots(&mut dom, host);

        // Shadow root entity should not appear in any slot.
        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert_eq!(assign.assigned_nodes, vec![light]);
        assert!(!assign.assigned_nodes.contains(&sr));
    }

    #[test]
    fn multiple_nodes_same_named_slot() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let c1 = elem_with_attrs(&mut dom, "span", &[("slot", "items")]);
        let c2 = elem_with_attrs(&mut dom, "p", &[("slot", "items")]);
        let _ = dom.append_child(host, c1);
        let _ = dom.append_child(host, c2);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem_with_attrs(&mut dom, "slot", &[("name", "items")]);
        let _ = dom.append_child(sr, slot);

        distribute_slots(&mut dom, host);

        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert_eq!(assign.assigned_nodes, vec![c1, c2]);
    }

    #[test]
    fn no_matching_slot_means_not_distributed() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let c1 = elem_with_attrs(&mut dom, "span", &[("slot", "missing")]);
        let _ = dom.append_child(host, c1);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem_with_attrs(&mut dom, "slot", &[("name", "other")]);
        let _ = dom.append_child(sr, slot);

        distribute_slots(&mut dom, host);

        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert!(assign.assigned_nodes.is_empty());
    }

    #[test]
    fn redistribute_clears_stale_slotted_markers() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let c1 = elem(&mut dom, "span");
        let c2 = elem(&mut dom, "p");
        let _ = dom.append_child(host, c1);
        let _ = dom.append_child(host, c2);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, slot);

        // First distribution: both c1 and c2 get SlottedMarker.
        distribute_slots(&mut dom, host);
        assert!(dom.world().get::<&SlottedMarker>(c1).is_ok());
        assert!(dom.world().get::<&SlottedMarker>(c2).is_ok());

        // Remove c2 from light DOM so it should no longer be slotted.
        let _ = dom.remove_child(host, c2);

        // Re-distribute: c2's SlottedMarker should be cleared.
        distribute_slots(&mut dom, host);
        assert!(dom.world().get::<&SlottedMarker>(c1).is_ok());
        // c2 is detached so not in host's children anymore — its marker
        // won't be cleared by redistribute (it's no longer a child).
        // But c2 should not appear in the slot's assigned_nodes.
        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert_eq!(assign.assigned_nodes, vec![c1]);
    }

    #[test]
    fn redistribute_clears_markers_when_slot_removed() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let c1 = elem(&mut dom, "span");
        let _ = dom.append_child(host, c1);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, slot);

        // First distribution: c1 gets SlottedMarker.
        distribute_slots(&mut dom, host);
        assert!(dom.world().get::<&SlottedMarker>(c1).is_ok());

        // Remove the slot element from shadow tree.
        let _ = dom.remove_child(sr, slot);

        // Re-distribute: c1 should lose its SlottedMarker since no slot exists.
        distribute_slots(&mut dom, host);
        assert!(
            dom.world().get::<&SlottedMarker>(c1).is_err(),
            "stale SlottedMarker should be cleared"
        );
    }

    #[test]
    fn nested_slot_in_shadow_tree() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let light = elem(&mut dom, "span");
        let _ = dom.append_child(host, light);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let wrapper = elem(&mut dom, "div");
        let slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, wrapper);
        let _ = dom.append_child(wrapper, slot);

        distribute_slots(&mut dom, host);

        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        assert_eq!(assign.assigned_nodes, vec![light]);
    }

    #[test]
    fn text_nodes_distributed_to_slot() {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, "div");
        let text1 = dom.create_text("hello");
        let child = elem(&mut dom, "span");
        let text2 = dom.create_text("world");
        let _ = dom.append_child(host, text1);
        let _ = dom.append_child(host, child);
        let _ = dom.append_child(host, text2);

        let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
        let slot = elem(&mut dom, "slot");
        let _ = dom.append_child(sr, slot);

        distribute_slots(&mut dom, host);

        let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
        // Text nodes should be distributed alongside elements (order preserved).
        assert_eq!(assign.assigned_nodes, vec![text1, child, text2]);
    }
}
