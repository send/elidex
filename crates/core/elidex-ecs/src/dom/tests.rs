use super::*;
use crate::components::{Attributes, ShadowRootMode, SlotAssignment, TextContent};

fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

#[test]
fn create_element() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let tags = dom.query_by_tag("div");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], div);
}

#[test]
fn create_text_node() {
    let mut dom = EcsDom::new();
    let text = dom.create_text("Hello, world!");
    let content = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(content.0, "Hello, world!");
}

#[test]
fn append_and_children() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child1 = elem(&mut dom, "span");
    let child2 = elem(&mut dom, "p");

    dom.append_child(parent, child1);
    dom.append_child(parent, child2);

    let children = dom.children(parent);
    assert_eq!(children, vec![child1, child2]);
}

#[test]
fn parent_relation() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    assert_eq!(dom.get_parent(child), Some(parent));
}

#[test]
fn remove_child_from_middle() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c);

    dom.remove_child(parent, b);

    let children = dom.children(parent);
    assert_eq!(children, vec![a, c]);
    assert_eq!(dom.get_parent(b), None);
}

#[test]
fn remove_first_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.remove_child(parent, a);

    assert_eq!(dom.children(parent), vec![b]);
}

#[test]
fn remove_last_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.remove_child(parent, b);

    assert_eq!(dom.children(parent), vec![a]);
}

#[test]
fn remove_only_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    dom.remove_child(parent, child);

    assert!(dom.children(parent).is_empty());
}

#[test]
fn query_by_tag_multiple() {
    let mut dom = EcsDom::new();
    let _div1 = elem(&mut dom, "div");
    let _span = elem(&mut dom, "span");
    let _div2 = elem(&mut dom, "div");

    assert_eq!(dom.query_by_tag("div").len(), 2);
    assert_eq!(dom.query_by_tag("span").len(), 1);
    assert!(dom.query_by_tag("p").is_empty());
}

#[test]
fn text_node_as_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("Hello");

    dom.append_child(parent, text);

    assert_eq!(dom.children(parent), vec![text]);
    let content = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(content.0, "Hello");
}

#[test]
fn reparenting_detaches_from_old_parent() {
    let mut dom = EcsDom::new();
    let parent_a = elem(&mut dom, "div");
    let parent_b = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent_a, child);
    assert_eq!(dom.children(parent_a), vec![child]);

    dom.append_child(parent_b, child);
    assert!(dom.children(parent_a).is_empty());
    assert_eq!(dom.children(parent_b), vec![child]);
}

#[test]
fn self_append_rejected() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert!(!dom.append_child(e, e));
    assert!(dom.children(e).is_empty());
    assert_eq!(dom.get_parent(e), None);
}

#[test]
fn remove_non_child_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let unrelated = elem(&mut dom, "span");
    assert!(!dom.remove_child(parent, unrelated));
}

#[test]
fn double_append_same_parent() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    dom.append_child(parent, child);

    assert_eq!(dom.children(parent), vec![child]);
}

#[test]
fn insert_before_first() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");

    dom.append_child(parent, b);
    assert!(dom.insert_before(parent, a, b));

    assert_eq!(dom.children(parent), vec![a, b]);
}

#[test]
fn insert_before_middle() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, c);
    assert!(dom.insert_before(parent, b, c));

    assert_eq!(dom.children(parent), vec![a, b, c]);
}

#[test]
fn insert_before_invalid_ref() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let unrelated = elem(&mut dom, "span");

    assert!(!dom.insert_before(parent, a, unrelated));
}

#[test]
fn replace_child_basic() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "p");

    dom.append_child(parent, a);
    dom.append_child(parent, b);

    assert!(dom.replace_child(parent, c, b));
    assert_eq!(dom.children(parent), vec![a, c]);
    assert_eq!(dom.get_parent(b), None);
}

#[test]
fn replace_only_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let old = elem(&mut dom, "span");
    let new = elem(&mut dom, "p");

    dom.append_child(parent, old);
    assert!(dom.replace_child(parent, new, old));

    assert_eq!(dom.children(parent), vec![new]);
}

#[test]
fn replace_child_invalid() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let unrelated = elem(&mut dom, "span");

    assert!(!dom.replace_child(parent, a, unrelated));
}

#[test]
fn destroy_entity_removes_from_world() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    dom.destroy_entity(child);

    assert!(dom.children(parent).is_empty());
    assert!(!dom.contains(child));
}

#[test]
fn destroy_detached_entity() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    dom.destroy_entity(e);
    assert!(dom.query_by_tag("div").is_empty());
}

#[test]
fn circular_append_rejected() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "span");

    dom.append_child(a, b);
    assert!(!dom.append_child(b, a));
    assert_eq!(dom.children(a), vec![b]);
    assert!(dom.children(b).is_empty());
}

#[test]
fn circular_deep_rejected() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "div");
    let c = elem(&mut dom, "div");

    dom.append_child(a, b);
    dom.append_child(b, c);
    assert!(!dom.append_child(c, a));
    assert_eq!(dom.children(b), vec![c]);
}

#[test]
fn circular_insert_before_rejected() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "p");

    dom.append_child(a, b);
    dom.append_child(a, c);
    assert!(!dom.insert_before(b, a, c));
}

#[test]
fn circular_replace_child_rejected() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "p");

    dom.append_child(a, b);
    dom.append_child(b, c);
    assert!(!dom.replace_child(b, a, c));
    assert_eq!(dom.children(b), vec![c]);
}

#[test]
fn append_destroyed_parent_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.destroy_entity(parent);
    assert!(!dom.append_child(parent, child));
}

#[test]
fn append_destroyed_child_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.destroy_entity(child);
    assert!(!dom.append_child(parent, child));
}

#[test]
fn remove_destroyed_child_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    dom.destroy_entity(child);
    assert!(!dom.remove_child(parent, child));
}

#[test]
fn replace_child_validates_before_detach() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let existing = elem(&mut dom, "span");
    let new_child = elem(&mut dom, "p");
    let unrelated = elem(&mut dom, "em");

    dom.append_child(parent, existing);
    dom.append_child(parent, new_child);

    assert!(!dom.replace_child(parent, new_child, unrelated));
    assert_eq!(dom.children(parent), vec![existing, new_child]);
}

#[test]
fn destroy_entity_returns_false_for_already_destroyed() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert!(dom.destroy_entity(e));
    assert!(!dom.destroy_entity(e));
}

#[test]
fn sibling_links_consistent() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c);

    assert_eq!(dom.get_next_sibling(a), Some(b));
    assert_eq!(dom.get_prev_sibling(a), None);
    assert_eq!(dom.get_prev_sibling(b), Some(a));
    assert_eq!(dom.get_next_sibling(b), Some(c));
    assert_eq!(dom.get_prev_sibling(c), Some(b));
    assert_eq!(dom.get_next_sibling(c), None);
}

#[test]
fn deep_tree() {
    let mut dom = EcsDom::new();
    let mut parent = elem(&mut dom, "div");
    let root = parent;

    for _ in 0..50 {
        let child = elem(&mut dom, "div");
        dom.append_child(parent, child);
        parent = child;
    }

    assert!(dom.children(parent).is_empty());
    assert_eq!(dom.children(root).len(), 1);
}

#[test]
fn helper_methods() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c);

    assert_eq!(dom.get_parent(a), Some(parent));
    assert_eq!(dom.get_parent(parent), None);
    assert_eq!(dom.get_first_child(parent), Some(a));
    assert_eq!(dom.get_last_child(parent), Some(c));
    assert_eq!(dom.get_first_child(a), None);
    assert_eq!(dom.get_last_child(a), None);
    assert_eq!(dom.get_next_sibling(a), Some(b));
    assert_eq!(dom.get_next_sibling(b), Some(c));
    assert_eq!(dom.get_next_sibling(c), None);
    assert_eq!(dom.get_prev_sibling(a), None);
    assert_eq!(dom.get_prev_sibling(b), Some(a));
    assert_eq!(dom.get_prev_sibling(c), Some(b));
}

#[test]
fn contains_method() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert!(dom.contains(e));
    dom.destroy_entity(e);
    assert!(!dom.contains(e));
}

#[test]
fn attributes_accessors() {
    let mut attrs = Attributes::default();
    assert!(!attrs.contains("class"));
    assert_eq!(attrs.get("class"), None);

    attrs.set("class", "foo");
    assert!(attrs.contains("class"));
    assert_eq!(attrs.get("class"), Some("foo"));

    let old = attrs.set("class", "bar");
    assert_eq!(old, Some("foo".to_string()));
    assert_eq!(attrs.get("class"), Some("bar"));

    let removed = attrs.remove("class");
    assert_eq!(removed, Some("bar".to_string()));
    assert!(!attrs.contains("class"));
}

#[test]
fn many_siblings() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let mut entities = Vec::new();

    for _ in 0..100 {
        let child = elem(&mut dom, "span");
        dom.append_child(parent, child);
        entities.push(child);
    }

    let children = dom.children(parent);
    assert_eq!(children.len(), 100);
    assert_eq!(children, entities);

    dom.remove_child(parent, entities[50]);
    let children = dom.children(parent);
    assert_eq!(children.len(), 99);
    assert!(!children.contains(&entities[50]));
}

#[test]
fn insert_before_adjacent_prev_sibling() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c);

    assert!(dom.insert_before(parent, b, c));
    assert_eq!(dom.children(parent), vec![a, b, c]);
}

#[test]
fn replace_child_adjacent_sibling() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);

    assert!(dom.replace_child(parent, b, a));
    assert_eq!(dom.children(parent), vec![b]);
    assert_eq!(dom.get_parent(a), None);
}

#[test]
fn destroy_entity_orphans_children() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c);

    dom.destroy_entity(parent);

    assert_eq!(dom.get_parent(a), None);
    assert_eq!(dom.get_parent(b), None);
    assert_eq!(dom.get_parent(c), None);
    assert_eq!(dom.get_next_sibling(a), None);
    assert_eq!(dom.get_prev_sibling(b), None);
    assert_eq!(dom.get_next_sibling(b), None);
    assert_eq!(dom.get_prev_sibling(c), None);
}

// --- Shadow DOM tests ---

#[test]
fn attach_shadow_success() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open);
    assert!(sr.is_ok());
    let sr = sr.unwrap();
    assert!(dom.contains(sr));
    assert_eq!(dom.get_shadow_root(host), Some(sr));
}

#[test]
fn attach_shadow_all_valid_tags() {
    for tag in VALID_SHADOW_HOST_TAGS {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, tag);
        let sr = dom.attach_shadow(host, ShadowRootMode::Open);
        assert!(sr.is_ok(), "attach_shadow should succeed for <{tag}>");
    }
}

#[test]
fn attach_shadow_invalid_tag() {
    let mut dom = EcsDom::new();
    let input = elem(&mut dom, "input");
    assert!(dom.attach_shadow(input, ShadowRootMode::Open).is_err());

    let img = elem(&mut dom, "img");
    assert!(dom.attach_shadow(img, ShadowRootMode::Open).is_err());

    let a = elem(&mut dom, "a");
    assert!(dom.attach_shadow(a, ShadowRootMode::Open).is_err());
}

#[test]
fn attach_shadow_custom_element() {
    let mut dom = EcsDom::new();
    let ce = elem(&mut dom, "my-component");
    assert!(
        dom.attach_shadow(ce, ShadowRootMode::Open).is_ok(),
        "custom elements should be valid shadow hosts"
    );

    let ce2 = elem(&mut dom, "x-widget");
    assert!(
        dom.attach_shadow(ce2, ShadowRootMode::Open).is_ok(),
        "custom elements with any hyphen should be valid"
    );
}

#[test]
fn attach_shadow_reserved_custom_element_names_rejected() {
    let mut dom = EcsDom::new();
    // Reserved names per HTML §4.13.2 — contain hyphen but are NOT valid custom elements.
    for name in [
        "annotation-xml",
        "color-profile",
        "font-face",
        "font-face-format",
        "font-face-name",
        "font-face-src",
        "font-face-uri",
        "missing-glyph",
    ] {
        let el = elem(&mut dom, name);
        assert!(
            dom.attach_shadow(el, ShadowRootMode::Open).is_err(),
            "reserved name '{name}' should be rejected as shadow host"
        );
    }
}

#[test]
fn attach_shadow_invalid_custom_element_format() {
    let mut dom = EcsDom::new();
    // Must start with lowercase ASCII letter.
    let upper = elem(&mut dom, "My-Component");
    assert!(
        dom.attach_shadow(upper, ShadowRootMode::Open).is_err(),
        "uppercase start should be rejected"
    );

    let digit = elem(&mut dom, "1-component");
    assert!(
        dom.attach_shadow(digit, ShadowRootMode::Open).is_err(),
        "digit start should be rejected"
    );
}

#[test]
fn attach_shadow_double_attach_fails() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    assert!(dom.attach_shadow(host, ShadowRootMode::Open).is_ok());
    assert!(dom.attach_shadow(host, ShadowRootMode::Open).is_err());
}

#[test]
fn shadow_root_not_in_children() {
    // M1: ShadowRoot entities are not exposed via children().
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let children = dom.children(host);
    assert!(
        !children.contains(&sr),
        "shadow root should not appear in children()"
    );
    assert!(
        children.contains(&light),
        "light DOM children should still appear"
    );
    // But we can still access via get_shadow_root.
    assert_eq!(dom.get_shadow_root(host), Some(sr));
}

#[test]
fn shadow_root_not_in_children_iter() {
    // M1: ShadowRoot entities are not exposed via children_iter().
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let children: Vec<Entity> = dom.children_iter(host).collect();
    assert!(!children.contains(&sr));
    assert!(children.contains(&light));
}

#[test]
fn get_shadow_root_none() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    assert_eq!(dom.get_shadow_root(host), None);
}

#[test]
fn composed_children_shadow_host() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "p");
    dom.append_child(sr, shadow_child);

    // composed_children of host should return shadow tree content.
    let composed = dom.composed_children(host);
    assert!(composed.contains(&shadow_child));
    // Light DOM children should NOT appear (they're distributed via slots).
    assert!(!composed.contains(&light));
}

#[test]
fn composed_children_slot_assigned() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Assign light child to slot.
    dom.world_mut()
        .insert_one(
            slot,
            SlotAssignment {
                assigned_nodes: vec![light],
            },
        )
        .unwrap();

    let composed = dom.composed_children(slot);
    assert_eq!(composed, vec![light]);
}

#[test]
fn composed_children_slot_fallback() {
    let mut dom = EcsDom::new();
    let slot = elem(&mut dom, "slot");
    let fallback = dom.create_text("fallback");
    dom.append_child(slot, fallback);

    // Empty SlotAssignment — should return slot's own children (fallback).
    dom.world_mut()
        .insert_one(
            slot,
            SlotAssignment {
                assigned_nodes: vec![],
            },
        )
        .unwrap();

    let composed = dom.composed_children(slot);
    assert_eq!(composed, vec![fallback]);
}

#[test]
fn composed_children_normal_element() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    dom.append_child(parent, child);

    // No shadow root, no slot — should return normal children.
    let composed = dom.composed_children(parent);
    assert_eq!(composed, vec![child]);
}

#[test]
fn shadow_root_closed_mode() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Closed).unwrap();
    assert_eq!(dom.get_shadow_root(host), Some(sr));
    // The shadow root entity exists, but JS access would check mode.
    let mode = dom.world().get::<&ShadowRoot>(sr).unwrap().mode;
    assert_eq!(mode, ShadowRootMode::Closed);
}

// --- Destroy + Shadow DOM interaction tests ---

#[test]
fn destroy_shadow_host_orphans_shadow_root() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    dom.destroy_entity(host);

    // Host is gone.
    assert!(!dom.contains(host));
    // Shadow root is orphaned but still exists (despawn only destroys the entity itself).
    assert!(dom.contains(sr));
    // Shadow child is detached from shadow root (orphaned by destroy_entity).
    assert!(dom.get_parent(sr).is_none());
}

#[test]
fn destroy_shadow_root_does_not_crash() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    // Destroying the shadow root: ShadowHost is cleaned up on the host.
    dom.destroy_entity(sr);

    assert!(!dom.contains(sr));
    assert!(dom.contains(host));
    // ShadowHost component should be removed (bidirectional cleanup).
    assert!(dom.world().get::<&crate::ShadowHost>(host).is_err());
    assert_eq!(dom.get_shadow_root(host), None);
}

#[test]
fn destroy_slot_clears_assignment() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Manually attach SlotAssignment.
    let _ = dom.world_mut().insert_one(
        slot,
        SlotAssignment {
            assigned_nodes: vec![light],
        },
    );

    dom.destroy_entity(slot);
    assert!(!dom.contains(slot));
    // Light child is still alive — just no longer assigned.
    assert!(dom.contains(light));
}

#[test]
fn destroy_assigned_node_leaves_no_dangling() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    let _ = dom.world_mut().insert_one(
        slot,
        SlotAssignment {
            assigned_nodes: vec![light],
        },
    );
    let _ = dom.world_mut().insert_one(light, crate::SlottedMarker);

    dom.destroy_entity(light);
    assert!(!dom.contains(light));
    // SlotAssignment still references the destroyed entity (stale ref).
    // This is documented behavior — redistribute should be called after mutations.
    let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
    assert_eq!(assign.assigned_nodes.len(), 1);
    assert!(!dom.contains(assign.assigned_nodes[0]));
}

// --- L7: find_tree_root tests ---

#[test]
fn find_tree_root_shadow_root_returns_self() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    // ShadowRoot itself should be returned as its own tree root.
    assert_eq!(dom.find_tree_root(sr), sr);
}

#[test]
fn find_tree_root_shadow_child_returns_shadow_root() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(doc, host);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let child = elem(&mut dom, "span");
    dom.append_child(sr, child);
    // Child in shadow tree should find shadow root as tree root.
    assert_eq!(dom.find_tree_root(child), sr);
}

#[test]
fn find_tree_root_normal_returns_document_root() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let div = elem(&mut dom, "div");
    dom.append_child(doc, div);
    let span = elem(&mut dom, "span");
    dom.append_child(div, span);
    // Normal DOM node should find document root.
    assert_eq!(dom.find_tree_root(span), doc);
}

// --- L3: get_shadow_root stale entity tests ---

#[test]
fn get_shadow_root_returns_none_after_destroy() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert_eq!(dom.get_shadow_root(host), Some(sr));
    dom.destroy_entity(sr);
    // After destroying shadow root, get_shadow_root should return None.
    assert_eq!(dom.get_shadow_root(host), None);
}

// --- L1: Custom element name validation tests ---

#[test]
fn custom_element_uppercase_rejected() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "MyElement");
    assert!(
        dom.attach_shadow(el, ShadowRootMode::Open).is_err(),
        "uppercase custom element names should be rejected"
    );
}

#[test]
fn custom_element_non_ascii_allowed() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "my-élément");
    assert!(
        dom.attach_shadow(el, ShadowRootMode::Open).is_ok(),
        "non-ASCII characters in custom element names should be allowed"
    );
}

#[test]
fn custom_element_invalid_char_rejected() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "my-element!");
    assert!(
        dom.attach_shadow(el, ShadowRootMode::Open).is_err(),
        "invalid PCENChar (!) should be rejected"
    );
}

// --- Shadow DOM destroy cleanup tests ---

#[test]
fn destroy_shadow_root_cleans_up_host() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert!(dom.world().get::<&ShadowHost>(host).is_ok());
    dom.destroy_entity(sr);
    // ShadowHost component should be removed from host.
    assert!(dom.world().get::<&ShadowHost>(host).is_err());
}

#[test]
fn destroy_host_cleans_up_shadow_root() {
    use crate::components::ShadowRoot;

    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert!(dom.world().get::<&ShadowRoot>(sr).is_ok());
    dom.destroy_entity(host);
    // ShadowRoot component should be removed from shadow root entity.
    assert!(dom.world().get::<&ShadowRoot>(sr).is_err());
}

#[test]
fn composed_children_fallback_after_shadow_root_destroy() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light_child = elem(&mut dom, "span");
    dom.append_child(host, light_child);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    // Before destroy: composed_children returns shadow tree (empty here).
    assert!(dom.composed_children(host).is_empty());
    dom.destroy_entity(sr);
    // After destroy: ShadowHost cleaned up, falls through to normal children.
    assert_eq!(dom.composed_children(host), vec![light_child]);
}
