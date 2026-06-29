//! Tests for `clone_attributes` / `clone_character_data` /
//! `clone_subtree` — the ECS helpers WHATWG DOM §4.4 `cloneNode`
//! reduces to.

use super::*;

#[test]
fn clone_attributes_copies_all_keys() {
    let mut dom = EcsDom::new();
    let src = elem(&mut dom, "div");
    let dst = elem(&mut dom, "div");
    assert!(dom.set_attribute(src, "id", "hero").did_set);
    assert!(dom.set_attribute(src, "class", "big").did_set);
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
    let clone = dom
        .clone_subtree(src, &mut Vec::new(), None)
        .expect("clone_subtree");
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
    assert!(dom.set_attribute(a, "data", "1").did_set);
    assert!(dom.set_attribute(b, "data", "2").did_set);
    assert!(dom.set_attribute(c, "data", "3").did_set);
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, b));
    assert!(dom.append_child(root, c));

    let clone = dom
        .clone_subtree(root, &mut Vec::new(), None)
        .expect("clone_subtree");
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
fn clone_subtree_infers_node_kind_from_payload_when_missing() {
    // Legacy entities lacking `NodeKind` must not be misclassified as
    // `NodeKind::Element` when they carry only `TextContent` /
    // `CommentData` / `DocTypeData`.  The clone helper infers the
    // kind from the payload components for this defensive path.
    use hecs::World;

    let mut dom = EcsDom::new();
    // Hand-craft a legacy Text entity: `TextContent` + `TreeRelation`
    // but no `NodeKind` component, mimicking pre-NodeKind creation.
    let legacy_text = dom
        .world_mut()
        .spawn((TextContent("legacy".to_owned()), TreeRelation::default()));
    // `NodeKind` is absent — deliberately not inserted.
    let _ = &dom; // keep the borrow scope explicit
                  // Sanity: confirm node_kind returns None.
    assert!(dom.node_kind(legacy_text).is_none());

    let clone = dom
        .clone_subtree(legacy_text, &mut Vec::new(), None)
        .expect("clone_subtree");
    // The clone must have inferred `NodeKind::Text` from the payload,
    // and must carry a `TextContent` component.
    assert_eq!(dom.node_kind(clone), Some(NodeKind::Text));
    let text = dom
        .world()
        .get::<&TextContent>(clone)
        .expect("cloned text data");
    assert_eq!(text.0, "legacy");

    // Suppress unused-import warning from the `World` import used only
    // for documentation purposes above.
    let _: &World = dom.world();
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
    assert_eq!(dom.clone_subtree(src, &mut Vec::new(), None), None);
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

    let clone = dom
        .clone_subtree(host, &mut Vec::new(), None)
        .expect("clone_subtree");
    // The clone has no shadow root component itself.
    assert!(dom.get_shadow_root(clone).is_none());
    // Light child was cloned.
    let kids = dom.children(clone);
    assert_eq!(kids.len(), 1);
    assert!(dom.has_tag(kids[0], "span"));
}

// -----------------------------------------------------------------------
// Clone-policy copy-set (see the tree_clone module-level table):
// intrinsic copy (Namespace) + derived copy (InlineStyle / IframeData)
// + deliberate non-copy (ElementState) + src↔dst pairs exposure.
// -----------------------------------------------------------------------

#[test]
fn clone_shallow_copies_namespace_inline_style_iframe_data() {
    use crate::components::{ElementState, IframeData, InlineStyle};
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("style", "color: red");
    let src = dom.create_element_ns("my-foo", Namespace::Svg, attrs, None);
    let mut style = InlineStyle::default();
    style.set("color", "red");
    dom.world_mut().insert_one(src, style).unwrap();
    dom.world_mut()
        .insert_one(src, IframeData::default())
        .unwrap();
    dom.world_mut()
        .insert_one(src, ElementState::default())
        .unwrap();

    let clone = dom.clone_node_shallow(src).expect("clone_node_shallow");
    // Intrinsic copy: foreign namespace survives the clone ("clone a
    // single node" step 2.4 passes the source namespace).
    assert_eq!(dom.namespace_of(clone), Namespace::Svg);
    // Derived copy (src-parity): InlineStyle and IframeData ride along
    // with the copied Attributes.
    assert_eq!(
        dom.world()
            .get::<&InlineStyle>(clone)
            .expect("InlineStyle copied")
            .get("color"),
        Some("red")
    );
    assert!(dom.world().get::<&IframeData>(clone).is_ok());
    // Deliberate non-copy: interaction state never crosses a clone.
    assert!(
        dom.world().get::<&ElementState>(clone).is_err(),
        "ElementState must not be copied (clone-policy non-copy row)"
    );
}

#[test]
fn clone_subtree_pairs_cover_every_node_root_first() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let child_a = elem(&mut dom, "span");
    let child_b = elem(&mut dom, "p");
    let grandchild = dom.create_text("hi");
    assert!(dom.append_child(root, child_a));
    assert!(dom.append_child(root, child_b));
    assert!(dom.append_child(child_b, grandchild));

    let mut pairs = Vec::new();
    let clone = dom
        .clone_subtree(root, &mut pairs, None)
        .expect("clone_subtree");
    // Completeness: one pair per cloned node, root first.
    assert_eq!(pairs.len(), 4);
    assert_eq!(pairs[0], (root, clone));
    // Mapping fidelity: every pair's two sides agree on TagType (or
    // both lack one, for the Text node).
    for (s, d) in &pairs {
        let src_tag = dom.world().get::<&TagType>(*s).ok().map(|t| t.0.clone());
        let dst_tag = dom.world().get::<&TagType>(*d).ok().map(|t| t.0.clone());
        assert_eq!(src_tag, dst_tag);
        assert_ne!(s, d, "pair must map source to a fresh entity");
    }
}
