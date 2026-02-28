use super::*;
use crate::components::{Attributes, TextContent};

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
