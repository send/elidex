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
    dom.append_child(root, a);
    dom.append_child(root, b);
    dom.append_child(a, c);

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
    dom.append_child(root, a);
    dom.append_child(root, b);

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
    dom.append_child(root, child);

    assert_eq!(dom.find_by_id(root, "target"), Some(child));
}

#[test]
fn find_by_id_not_found() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    dom.append_child(root, child);

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
    dom.append_child(root, mid);
    dom.append_child(mid, deep);

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
