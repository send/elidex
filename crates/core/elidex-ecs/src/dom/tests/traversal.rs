use super::*;

// ---------------------------------------------------------------------------
// traverse_descendants
// ---------------------------------------------------------------------------

#[test]
fn traverse_descendants_pre_order() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    let c = elem(&mut dom, "c");
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, b));
    assert!(dom.append_child(a, c));

    let mut visited = Vec::new();
    dom.traverse_descendants(root, |e| {
        visited.push(e);
        true
    });
    // Pre-order: a, c (child of a), b
    assert_eq!(visited, vec![a, c, b]);
}

#[test]
fn traverse_descendants_early_stop() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, b));

    let mut visited = Vec::new();
    dom.traverse_descendants(root, |e| {
        visited.push(e);
        false // stop after first
    });
    assert_eq!(visited, vec![a]);
}

#[test]
fn traverse_descendants_empty() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");

    let mut count = 0;
    dom.traverse_descendants(root, |_| {
        count += 1;
        true
    });
    assert_eq!(count, 0);
}

// ---------------------------------------------------------------------------
// find_by_id
// ---------------------------------------------------------------------------

#[test]
fn find_by_id_found() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let child = dom.create_element("span", {
        let mut a = Attributes::default();
        a.set("id", "target");
        a
    });
    assert!(dom.append_child(root, child));

    assert_eq!(dom.find_by_id(root, "target"), Some(child));
}

#[test]
fn find_by_id_not_found() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    assert!(dom.append_child(root, child));

    assert_eq!(dom.find_by_id(root, "nonexistent"), None);
}

#[test]
fn find_by_id_nested() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let mid = elem(&mut dom, "section");
    let deep = dom.create_element("p", {
        let mut a = Attributes::default();
        a.set("id", "deep");
        a
    });
    assert!(dom.append_child(root, mid));
    assert!(dom.append_child(mid, deep));

    assert_eq!(dom.find_by_id(root, "deep"), Some(deep));
}

#[test]
fn find_by_id_ignores_non_descendants() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    // orphan is NOT a descendant of root
    let _orphan = dom.create_element("span", {
        let mut a = Attributes::default();
        a.set("id", "orphan");
        a
    });

    assert_eq!(dom.find_by_id(root, "orphan"), None);
}

// ---------------------------------------------------------------------------
// first_element_child / last_element_child
// ---------------------------------------------------------------------------

#[test]
fn first_and_last_element_child_skip_text() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let text_head = dom.create_text("lead");
    let a = elem(&mut dom, "a");
    let text_mid = dom.create_text("middle");
    let b = elem(&mut dom, "b");
    let text_tail = dom.create_text("trail");
    assert!(dom.append_child(root, text_head));
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, text_mid));
    assert!(dom.append_child(root, b));
    assert!(dom.append_child(root, text_tail));

    assert_eq!(dom.first_element_child(root), Some(a));
    assert_eq!(dom.last_element_child(root), Some(b));
}

#[test]
fn first_element_child_empty_and_text_only() {
    let mut dom = EcsDom::new();
    let empty = elem(&mut dom, "div");
    assert_eq!(dom.first_element_child(empty), None);
    assert_eq!(dom.last_element_child(empty), None);

    let text_only = elem(&mut dom, "p");
    let t = dom.create_text("x");
    assert!(dom.append_child(text_only, t));
    assert_eq!(dom.first_element_child(text_only), None);
    assert_eq!(dom.last_element_child(text_only), None);
}

// ---------------------------------------------------------------------------
// next_element_sibling / prev_element_sibling
// ---------------------------------------------------------------------------

#[test]
fn element_siblings_skip_non_elements() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    let t1 = dom.create_text("t1");
    let t2 = dom.create_text("t2");
    let b = elem(&mut dom, "b");
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, t1));
    assert!(dom.append_child(root, t2));
    assert!(dom.append_child(root, b));

    assert_eq!(dom.next_element_sibling(a), Some(b));
    assert_eq!(dom.prev_element_sibling(b), Some(a));
    assert_eq!(dom.next_element_sibling(b), None);
    assert_eq!(dom.prev_element_sibling(a), None);
}

// ---------------------------------------------------------------------------
// first_child_with_tag
// ---------------------------------------------------------------------------

#[test]
fn first_child_with_tag_case_insensitive() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, b));

    assert_eq!(dom.first_child_with_tag(root, "a"), Some(a));
    assert_eq!(dom.first_child_with_tag(root, "A"), Some(a));
    assert_eq!(dom.first_child_with_tag(root, "b"), Some(b));
}

#[test]
fn first_child_with_tag_missing() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    assert!(dom.append_child(root, a));

    assert_eq!(dom.first_child_with_tag(root, "p"), None);
}

#[test]
fn first_child_with_tag_does_not_recurse() {
    // Spec: first_child_with_tag only scans direct children.
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let mid = elem(&mut dom, "section");
    let deep = elem(&mut dom, "p");
    assert!(dom.append_child(root, mid));
    assert!(dom.append_child(mid, deep));

    assert_eq!(dom.first_child_with_tag(root, "p"), None);
    assert_eq!(dom.first_child_with_tag(mid, "p"), Some(deep));
}
