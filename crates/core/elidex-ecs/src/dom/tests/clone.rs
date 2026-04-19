//! Tests for `clone_attributes` / `clone_character_data` /
//! `clone_subtree` — the ECS helpers WHATWG DOM §4.5 `cloneNode`
//! reduces to.

use super::*;

#[test]
fn clone_attributes_copies_all_keys() {
    let mut dom = EcsDom::new();
    let src = elem(&mut dom, "div");
    let dst = elem(&mut dom, "div");
    assert!(dom.set_attribute(src, "id", "hero".to_owned()));
    assert!(dom.set_attribute(src, "class", "big".to_owned()));
    dom.clone_attributes(src, dst);
    assert_eq!(dom.get_attribute(dst, "id"), Some("hero".to_owned()));
    assert_eq!(dom.get_attribute(dst, "class"), Some("big".to_owned()));
}

#[test]
fn clone_character_data_text_to_text() {
    let mut dom = EcsDom::new();
    let src = dom.create_text("hello");
    // Pre-allocate dst with blank text to exercise the overwrite path.
    let dst = dom.create_text("");
    dom.clone_character_data(src, dst);
    let text = dom
        .world()
        .get::<&TextContent>(dst)
        .expect("clone should populate TextContent");
    assert_eq!(text.0, "hello");
}

#[test]
fn clone_character_data_comment_to_comment() {
    let mut dom = EcsDom::new();
    let src = dom.create_comment("note");
    let dst = dom.create_comment("");
    dom.clone_character_data(src, dst);
    let c = dom
        .world()
        .get::<&CommentData>(dst)
        .expect("clone should populate CommentData");
    assert_eq!(c.0, "note");
}

#[test]
fn clone_character_data_mismatched_kinds_noop() {
    // Mismatched kinds must not pollute the destination with the
    // wrong component — e.g., Text → Comment would otherwise leave
    // the Comment entity with both CommentData and TextContent.
    let mut dom = EcsDom::new();
    let src = dom.create_text("hello");
    let dst = dom.create_comment("original");
    dom.clone_character_data(src, dst);
    // Destination still has its original CommentData.
    let c = dom.world().get::<&CommentData>(dst).unwrap();
    assert_eq!(c.0, "original");
    // And no TextContent was smuggled in.
    drop(c);
    assert!(dom.world().get::<&TextContent>(dst).is_err());
}

#[test]
fn clone_subtree_shallow_root_has_no_parent() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "section");
    let src = elem(&mut dom, "div");
    assert!(dom.append_child(parent, src));
    let clone = dom.clone_subtree(src).expect("clone_subtree");
    assert!(dom.get_parent(clone).is_none());
    assert!(dom.get_next_sibling(clone).is_none());
    assert!(dom.get_prev_sibling(clone).is_none());
}

#[test]
fn clone_subtree_deep_copies_children_order() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "ul");
    let a = elem(&mut dom, "li");
    let b = elem(&mut dom, "li");
    let c = elem(&mut dom, "li");
    assert!(dom.set_attribute(a, "data", "1".to_owned()));
    assert!(dom.set_attribute(b, "data", "2".to_owned()));
    assert!(dom.set_attribute(c, "data", "3".to_owned()));
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, b));
    assert!(dom.append_child(root, c));

    let clone = dom.clone_subtree(root).expect("clone_subtree");
    let kids = dom.children(clone);
    assert_eq!(kids.len(), 3);
    let values: Vec<String> = kids
        .iter()
        .map(|&e| dom.get_attribute(e, "data").expect("cloned attr"))
        .collect();
    assert_eq!(values, vec!["1", "2", "3"]);
    // Clone children are distinct entities from originals.
    assert!(!kids.iter().any(|&e| e == a || e == b || e == c));
}

#[test]
fn clone_subtree_on_destroyed_src_returns_none() {
    // The returned handle must never alias the original.  Prior to
    // the Option return type, a destroyed `src` echoed back as the
    // "clone", which let JS callers observe a node that was already
    // despawned.
    let mut dom = EcsDom::new();
    let src = elem(&mut dom, "div");
    dom.destroy_entity(src);
    assert_eq!(dom.clone_subtree(src), None);
}

#[test]
fn clone_subtree_skips_shadow_root() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let _shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");
    // Light-tree child that must be cloned.
    let light = elem(&mut dom, "span");
    assert!(dom.append_child(host, light));

    let clone = dom.clone_subtree(host).expect("clone_subtree");
    // The clone has no shadow root component itself.
    assert!(dom.get_shadow_root(clone).is_none());
    // Light child was cloned.
    let kids = dom.children(clone);
    assert_eq!(kids.len(), 1);
    assert!(dom.has_tag(kids[0], "span"));
}
