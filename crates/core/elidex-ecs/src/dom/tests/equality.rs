//! Tests for `EcsDom::nodes_equal` + `compare_document_position` —
//! the WHATWG DOM §4.4 equality and position primitives that the
//! `elidex-dom-api` IsEqualNode / CompareDocumentPosition handlers
//! and the `elidex-js` VM-side natives delegate to.

use super::*;
use crate::components::AttrData;
use crate::dom::equality::{
    DOCUMENT_POSITION_CONTAINED_BY, DOCUMENT_POSITION_CONTAINS, DOCUMENT_POSITION_DISCONNECTED,
    DOCUMENT_POSITION_FOLLOWING, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC,
    DOCUMENT_POSITION_PRECEDING,
};

// ---------------------------------------------------------------------------
// nodes_equal
// ---------------------------------------------------------------------------

#[test]
fn nodes_equal_deep_tree_no_overflow() {
    // Pin the iterative-stack contract: a 5000-deep linked nesting
    // must not overflow the Rust call stack.  Recursion would crash
    // at ~2-3k on a default 8 MiB stack.
    let mut dom = EcsDom::new();
    let root_a = elem(&mut dom, "div");
    let root_b = elem(&mut dom, "div");
    let mut current_a = root_a;
    let mut current_b = root_b;
    for _ in 0..5000 {
        let child_a = elem(&mut dom, "span");
        let child_b = elem(&mut dom, "span");
        let _ = dom.append_child(current_a, child_a);
        let _ = dom.append_child(current_b, child_b);
        current_a = child_a;
        current_b = child_b;
    }
    assert!(dom.nodes_equal(root_a, root_b));
}

#[test]
fn nodes_equal_legacy_kind_inferred_distinguishes_payloads() {
    // Two legacy entities (no `NodeKind` component) carrying
    // different payload kinds must NOT compare equal — the
    // `node_kind_inferred` fallback drives this.
    let mut dom = EcsDom::new();
    let text_legacy = dom.world_mut().spawn((TextContent("hi".into()),));
    let comment_legacy = dom.world_mut().spawn((CommentData("hi".into()),));
    assert!(!dom.nodes_equal(text_legacy, comment_legacy));
}

#[test]
fn nodes_equal_per_kind_branches() {
    let mut dom = EcsDom::new();
    // Element: same tag + same attrs (order-independent) -> equal
    let same_attrs_lhs = elem(&mut dom, "p");
    let same_attrs_rhs = elem(&mut dom, "p");
    assert!(dom.set_attribute(same_attrs_lhs, "id", "x".into()));
    assert!(dom.set_attribute(same_attrs_lhs, "class", "y".into()));
    assert!(dom.set_attribute(same_attrs_rhs, "class", "y".into()));
    assert!(dom.set_attribute(same_attrs_rhs, "id", "x".into()));
    assert!(dom.nodes_equal(same_attrs_lhs, same_attrs_rhs));

    // Element: differing attribute value -> not equal
    let differ_lhs = elem(&mut dom, "p");
    let differ_rhs = elem(&mut dom, "p");
    assert!(dom.set_attribute(differ_lhs, "id", "x".into()));
    assert!(dom.set_attribute(differ_rhs, "id", "z".into()));
    assert!(!dom.nodes_equal(differ_lhs, differ_rhs));

    // Element: differing tag -> not equal
    let p_node = elem(&mut dom, "p");
    let div_node = elem(&mut dom, "div");
    assert!(!dom.nodes_equal(p_node, div_node));

    // Text: same payload -> equal; different -> not equal
    let t1 = dom.create_text("hello");
    let t2 = dom.create_text("hello");
    let t3 = dom.create_text("world");
    assert!(dom.nodes_equal(t1, t2));
    assert!(!dom.nodes_equal(t1, t3));

    // Comment: same payload -> equal; different -> not equal
    let c1 = dom.create_comment("note");
    let c2 = dom.create_comment("note");
    let c3 = dom.create_comment("other");
    assert!(dom.nodes_equal(c1, c2));
    assert!(!dom.nodes_equal(c1, c3));

    // DocumentType: all 3 fields must match
    let dt1 = dom.create_document_type("html", "", "");
    let dt2 = dom.create_document_type("html", "", "");
    let dt3 = dom.create_document_type("html", "PUB", "");
    assert!(dom.nodes_equal(dt1, dt2));
    assert!(!dom.nodes_equal(dt1, dt3));

    // DocumentFragment: payload-less, equal when children match
    let f1 = dom.create_document_fragment();
    let f2 = dom.create_document_fragment();
    assert!(dom.nodes_equal(f1, f2));
}

#[test]
fn nodes_equal_attr_payload_distinguishes_local_name_and_value() {
    // Pin Copilot R1 vLFr: distinct Attr nodes must NOT vacuously
    // equal — local_name and value must contribute to the payload
    // comparison.
    let mut dom = EcsDom::new();
    let id_x = dom.create_attribute("id");
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(id_x) {
        a.value = "x".into();
    }
    let id_x_clone = dom.create_attribute("id");
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(id_x_clone) {
        a.value = "x".into();
    }
    let id_y = dom.create_attribute("id");
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(id_y) {
        a.value = "y".into();
    }
    let class_x = dom.create_attribute("class");
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(class_x) {
        a.value = "x".into();
    }
    assert!(dom.nodes_equal(id_x, id_x_clone));
    assert!(!dom.nodes_equal(id_x, id_y));
    assert!(!dom.nodes_equal(id_x, class_x));
}

#[test]
fn nodes_equal_non_dom_entities_return_false() {
    // Pin Copilot R1 vLFr second leg: two bare entities without a
    // NodeKind component or DOM payload must NOT compare equal —
    // node_kind_inferred returns None for both, and the per-kind
    // arms would otherwise no-op into a vacuous true.
    let mut dom = EcsDom::new();
    let bare_a = dom.world_mut().spawn(());
    let bare_b = dom.world_mut().spawn(());
    assert!(!dom.nodes_equal(bare_a, bare_b));
}

#[test]
fn compare_document_position_attr_same_owner_direction_follows_this_lt_other() {
    // Pin Copilot R1 vLGI: this.compareDocumentPosition(other) flags
    // describe OTHER's position relative to THIS, so when this < other
    // in entity order, the result must include FOLLOWING (other
    // follows this), not PRECEDING.
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let first = dom.create_attribute("id");
    let second = dom.create_attribute("class");
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(first) {
        a.owner_element = Some(host);
    }
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(second) {
        a.owner_element = Some(host);
    }
    // first.to_bits() < second.to_bits() because Attr entities
    // allocate in attribute insertion order.
    assert!(first.to_bits() < second.to_bits());
    let cmp = dom.compare_document_position(first, second);
    assert_eq!(
        cmp,
        DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC | DOCUMENT_POSITION_FOLLOWING
    );
    let rev = dom.compare_document_position(second, first);
    assert_eq!(
        rev,
        DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC | DOCUMENT_POSITION_PRECEDING
    );
}

#[test]
fn nodes_equal_children_count_must_match() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "div");
    let span = elem(&mut dom, "span");
    let _ = dom.append_child(a, span);
    assert!(!dom.nodes_equal(a, b));
}

// ---------------------------------------------------------------------------
// compare_document_position
// ---------------------------------------------------------------------------

#[test]
fn compare_document_position_self_is_zero() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    assert_eq!(dom.compare_document_position(a, a), 0);
}

#[test]
fn compare_document_position_contains_and_contained_by() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "section");
    let child = elem(&mut dom, "p");
    let _ = dom.append_child(parent, child);
    // child.compareDocumentPosition(parent) -> CONTAINS|PRECEDING
    assert_eq!(
        dom.compare_document_position(child, parent),
        DOCUMENT_POSITION_CONTAINS | DOCUMENT_POSITION_PRECEDING
    );
    // parent.compareDocumentPosition(child) -> CONTAINED_BY|FOLLOWING
    assert_eq!(
        dom.compare_document_position(parent, child),
        DOCUMENT_POSITION_CONTAINED_BY | DOCUMENT_POSITION_FOLLOWING
    );
}

#[test]
fn compare_document_position_preceding_and_following_siblings() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let _ = dom.append_child(parent, a);
    let _ = dom.append_child(parent, b);
    // a.compareDocumentPosition(b) -> FOLLOWING (b comes after a)
    assert_eq!(
        dom.compare_document_position(a, b),
        DOCUMENT_POSITION_FOLLOWING
    );
    // b.compareDocumentPosition(a) -> PRECEDING
    assert_eq!(
        dom.compare_document_position(b, a),
        DOCUMENT_POSITION_PRECEDING
    );
}

#[test]
fn compare_document_position_parent_child_xor_antisymmetric() {
    // Per WHATWG §4.4, parent.cmp(child) ^ child.cmp(parent) must
    // include both containment bits AND both directional bits — the
    // XOR equals (CONTAINS | CONTAINED_BY | PRECEDING | FOLLOWING)
    // = 0x1E.  Pins the invariant that swapping operands flips
    // CONTAINS↔CONTAINED_BY together with PRECEDING↔FOLLOWING.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "section");
    let child = elem(&mut dom, "p");
    let _ = dom.append_child(parent, child);
    let pc = dom.compare_document_position(parent, child);
    let cp = dom.compare_document_position(child, parent);
    assert_eq!(
        pc ^ cp,
        DOCUMENT_POSITION_CONTAINS
            | DOCUMENT_POSITION_CONTAINED_BY
            | DOCUMENT_POSITION_PRECEDING
            | DOCUMENT_POSITION_FOLLOWING
    );
}

#[test]
fn compare_document_position_cousins_use_tree_order_cmp() {
    // Cousins = same-tree, neither ancestor of the other, different
    // depths.  Exercises the tree_order_cmp fallback (line ~196 in
    // equality.rs) which sibling tests don't reach.
    //
    //   root
    //   ├─ branch_a
    //   │   └─ leaf_a
    //   └─ branch_b
    //       └─ leaf_b
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "section");
    let branch_a = elem(&mut dom, "div");
    let leaf_a = elem(&mut dom, "p");
    let branch_b = elem(&mut dom, "div");
    let leaf_b = elem(&mut dom, "p");
    let _ = dom.append_child(root, branch_a);
    let _ = dom.append_child(branch_a, leaf_a);
    let _ = dom.append_child(root, branch_b);
    let _ = dom.append_child(branch_b, leaf_b);
    // leaf_a is in document order before leaf_b -> leaf_b FOLLOWS.
    assert_eq!(
        dom.compare_document_position(leaf_a, leaf_b),
        DOCUMENT_POSITION_FOLLOWING
    );
    assert_eq!(
        dom.compare_document_position(leaf_b, leaf_a),
        DOCUMENT_POSITION_PRECEDING
    );
}

#[test]
fn compare_document_position_disconnected_antisymmetric() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "div");
    // No tree edges -> different roots -> DISCONNECTED|IMPL|...
    let ab = dom.compare_document_position(a, b);
    let ba = dom.compare_document_position(b, a);
    let mask = DOCUMENT_POSITION_DISCONNECTED | DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC;
    assert_eq!(ab & mask, mask);
    assert_eq!(ba & mask, mask);
    // Antisymmetric: PRECEDING ↔ FOLLOWING flip when operands swap
    assert_eq!(
        ab ^ ba,
        DOCUMENT_POSITION_PRECEDING | DOCUMENT_POSITION_FOLLOWING
    );
}

#[test]
fn compare_document_position_attr_same_owner_implementation_specific() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let attr_a = dom.create_attribute("id");
    let attr_b = dom.create_attribute("class");
    // Wire owner_element so the Attr-vs-Attr same-owner path fires.
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(attr_a) {
        a.owner_element = Some(host);
    }
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(attr_b) {
        a.owner_element = Some(host);
    }
    let cmp = dom.compare_document_position(attr_a, attr_b);
    // Must include IMPLEMENTATION_SPECIFIC and exactly one of
    // PRECEDING / FOLLOWING (no DISCONNECTED).
    assert_ne!(cmp & DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC, 0);
    assert_eq!(cmp & DOCUMENT_POSITION_DISCONNECTED, 0);
    let dir = cmp & (DOCUMENT_POSITION_PRECEDING | DOCUMENT_POSITION_FOLLOWING);
    assert!(dir == DOCUMENT_POSITION_PRECEDING || dir == DOCUMENT_POSITION_FOLLOWING);
}

#[test]
fn compare_document_position_attr_uses_owner_element_for_tree_compare() {
    // Attr operand whose owner sits inside the tree compares as if
    // it were rooted at its owning Element — so the Attr "follows"
    // a sibling that comes after the owner Element in document
    // order.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "section");
    let owner_el = elem(&mut dom, "p");
    let later_el = elem(&mut dom, "span");
    let _ = dom.append_child(parent, owner_el);
    let _ = dom.append_child(parent, later_el);
    let attr = dom.create_attribute("data-x");
    if let Ok(mut a) = dom.world_mut().get::<&mut AttrData>(attr) {
        a.owner_element = Some(owner_el);
    }
    // Attr-of-owner_el vs later_el: owner is before later -> Attr
    // also "before" -> later FOLLOWS the Attr.
    assert_eq!(
        dom.compare_document_position(attr, later_el),
        DOCUMENT_POSITION_FOLLOWING
    );
}

#[test]
fn compare_document_position_doctype_orphan_disconnected() {
    // DocumentType lives outside any tree by default — pin the
    // disconnected branch on a non-Element kind so the path covers
    // more than the Element-fixture case.
    let mut dom = EcsDom::new();
    let dt = dom.create_document_type("html", "", "");
    let other = elem(&mut dom, "div");
    let cmp = dom.compare_document_position(dt, other);
    assert_eq!(
        cmp & DOCUMENT_POSITION_DISCONNECTED,
        DOCUMENT_POSITION_DISCONNECTED
    );
}
