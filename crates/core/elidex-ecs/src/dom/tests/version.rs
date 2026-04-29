use super::*;

#[test]
fn version_initial_zero() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert_eq!(dom.inclusive_descendants_version(e), 0);
}

#[test]
fn version_propagates_to_ancestors() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = elem(&mut dom, "html");
    dom.append_child(doc, html);
    let body = elem(&mut dom, "body");
    dom.append_child(html, body);

    // All got bumped during append_child
    let v_doc = dom.inclusive_descendants_version(doc);
    let v_html = dom.inclusive_descendants_version(html);
    let v_body = dom.inclusive_descendants_version(body);
    assert!(v_doc > 0);
    assert!(v_html > 0);

    // Now add a child to body — body, html, doc should all bump
    let div = elem(&mut dom, "div");
    dom.append_child(body, div);
    assert!(dom.inclusive_descendants_version(body) > v_body);
    assert!(dom.inclusive_descendants_version(html) > v_html);
    assert!(dom.inclusive_descendants_version(doc) > v_doc);
}

#[test]
fn version_propagates_to_doc_root() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = elem(&mut dom, "html");
    dom.append_child(doc, html);

    let v = dom.inclusive_descendants_version(doc);
    let child = elem(&mut dom, "p");
    dom.append_child(html, child);
    assert!(dom.inclusive_descendants_version(doc) > v);
}

#[test]
fn version_sibling_not_affected() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "p");
    dom.append_child(parent, a);
    dom.append_child(parent, b);

    let v_a = dom.inclusive_descendants_version(a);
    let v_b = dom.inclusive_descendants_version(b);

    // Add child to a — b should NOT be affected
    let c = elem(&mut dom, "em");
    dom.append_child(a, c);
    assert!(dom.inclusive_descendants_version(a) > v_a);
    assert_eq!(dom.inclusive_descendants_version(b), v_b);
}

#[test]
fn version_failed_mutation_no_rev() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    dom.append_child(parent, child);
    let v = dom.inclusive_descendants_version(parent);

    // Attempt to create a cycle — should fail, version should NOT bump
    let ok = dom.append_child(child, parent);
    assert!(!ok);
    assert_eq!(dom.inclusive_descendants_version(parent), v);
}

#[test]
fn version_tree_mutation_auto() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let v0 = dom.inclusive_descendants_version(parent);

    let child = elem(&mut dom, "span");
    dom.append_child(parent, child);
    let v1 = dom.inclusive_descendants_version(parent);
    assert!(v1 > v0);

    dom.remove_child(parent, child);
    let v2 = dom.inclusive_descendants_version(parent);
    assert!(v2 > v1);
}

#[test]
fn version_detached_node() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    // Detached node: rev_version still bumps its own version
    dom.rev_version(e);
    assert_eq!(dom.inclusive_descendants_version(e), 1);
}

#[test]
fn version_find_tree_root_connected() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = elem(&mut dom, "html");
    dom.append_child(doc, html);
    let body = elem(&mut dom, "body");
    dom.append_child(html, body);
    assert_eq!(dom.find_tree_root(body), doc);
}

#[test]
fn version_find_tree_root_detached() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert_eq!(dom.find_tree_root(e), e);
}

#[test]
fn tree_order_cmp_siblings() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "p");
    dom.append_child(parent, a);
    dom.append_child(parent, b);
    assert_eq!(dom.tree_order_cmp(a, b), std::cmp::Ordering::Less);
    assert_eq!(dom.tree_order_cmp(b, a), std::cmp::Ordering::Greater);
}

#[test]
fn tree_order_cmp_ancestor_descendant() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    dom.append_child(parent, child);
    assert_eq!(dom.tree_order_cmp(parent, child), std::cmp::Ordering::Less);
    assert_eq!(
        dom.tree_order_cmp(child, parent),
        std::cmp::Ordering::Greater
    );
}

#[test]
fn tree_order_cmp_cousins() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "p");
    dom.append_child(root, a);
    dom.append_child(root, b);
    let a_child = elem(&mut dom, "em");
    let b_child = elem(&mut dom, "strong");
    dom.append_child(a, a_child);
    dom.append_child(b, b_child);
    assert_eq!(
        dom.tree_order_cmp(a_child, b_child),
        std::cmp::Ordering::Less
    );
}

#[test]
fn version_propagates_through_shadow_boundary() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(doc, host);
    let sr = dom
        .attach_shadow(host, crate::ShadowRootMode::Open)
        .unwrap();
    let shadow_child = elem(&mut dom, "p");
    dom.append_child(sr, shadow_child);

    let v_host = dom.inclusive_descendants_version(host);
    let v_doc = dom.inclusive_descendants_version(doc);

    // Mutate inside shadow tree — version should propagate through
    // ShadowRoot → host → doc.
    let grandchild = elem(&mut dom, "span");
    dom.append_child(shadow_child, grandchild);

    assert!(dom.inclusive_descendants_version(host) > v_host);
    assert!(dom.inclusive_descendants_version(doc) > v_doc);
}

#[test]
fn is_element_true() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert!(dom.is_element(e));
}

#[test]
fn is_element_false_text() {
    let mut dom = EcsDom::new();
    let t = dom.create_text("hello");
    assert!(!dom.is_element(t));
}

#[test]
fn is_element_false_document() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    assert!(!dom.is_element(doc));
}

#[test]
fn version_bumped_by_set_attribute() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let body = elem(&mut dom, "body");
    dom.append_child(doc, body);
    let div = elem(&mut dom, "div");
    dom.append_child(body, div);

    let v_doc = dom.inclusive_descendants_version(doc);
    let v_body = dom.inclusive_descendants_version(body);
    let v_div = dom.inclusive_descendants_version(div);

    // First set on a fresh entity (no `Attributes` component yet) —
    // exercises the `insert_one(Attributes)` branch.
    assert!(dom.set_attribute(div, "class", "foo".into()));
    assert!(dom.inclusive_descendants_version(div) > v_div);
    assert!(dom.inclusive_descendants_version(body) > v_body);
    assert!(dom.inclusive_descendants_version(doc) > v_doc);

    // Second set on the same entity — exercises the `&mut Attributes`
    // branch.  Must bump again so that downstream caches keyed against
    // an attribute-mutation-sensitive root never wedge to a stale value.
    let v_div2 = dom.inclusive_descendants_version(div);
    assert!(dom.set_attribute(div, "class", "bar".into()));
    assert!(dom.inclusive_descendants_version(div) > v_div2);
}

#[test]
fn version_bumped_by_remove_attribute_even_when_absent() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let body = elem(&mut dom, "body");
    dom.append_child(doc, body);

    let v_doc = dom.inclusive_descendants_version(doc);

    // No `Attributes` component yet — `remove_attribute` is a logical
    // no-op on the attribute storage but must still bump version so
    // attribute-filtered caches converge to "no match" deterministically
    // (e.g. a `getElementsByName` cache populated before the no-op
    // removal must re-walk after it, since the cache invariant is
    // version-keyed, not attribute-mutation-keyed).
    let v_body = dom.inclusive_descendants_version(body);
    dom.remove_attribute(body, "name");
    assert!(dom.inclusive_descendants_version(body) > v_body);
    assert!(dom.inclusive_descendants_version(doc) > v_doc);

    // After a real attribute is set + removed, version must bump on
    // the remove leg too.
    assert!(dom.set_attribute(body, "id", "x".into()));
    let v_body2 = dom.inclusive_descendants_version(body);
    dom.remove_attribute(body, "id");
    assert!(dom.inclusive_descendants_version(body) > v_body2);
}
