use super::*;
use elidex_ecs::Attributes;

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

/// Assert a mutation produced exactly one record and return it (the common
/// single-record case; a childList move yields two — see the move tests).
fn expect_one(records: Vec<MutationRecord>) -> MutationRecord {
    assert_eq!(records.len(), 1, "expected exactly one record");
    records.into_iter().next().unwrap()
}

#[test]
fn apply_append_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    let m = Mutation::AppendChild { parent, child };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::ChildList);
    assert_eq!(record.target, parent);
    assert_eq!(record.added_nodes, vec![child]);
    assert!(record.removed_nodes.is_empty());
    assert_eq!(record.previous_sibling, None);
    assert_eq!(dom.children(parent), vec![child]);
}

#[test]
fn apply_append_child_records_previous_sibling() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let first = elem(&mut dom, "span");
    let second = elem(&mut dom, "p");
    dom.append_child(parent, first);

    let m = Mutation::AppendChild {
        parent,
        child: second,
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.previous_sibling, Some(first));
    assert_eq!(record.added_nodes, vec![second]);
}

#[test]
fn apply_insert_before() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    dom.append_child(parent, b);

    let m = Mutation::InsertBefore {
        parent,
        new_child: a,
        ref_child: b,
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::ChildList);
    assert_eq!(record.added_nodes, vec![a]);
    assert_eq!(record.next_sibling, Some(b));
    assert_eq!(dom.children(parent), vec![a, b]);
}

// --- B1.2a: move-record childList (already-parented node → two records) ---

#[test]
fn apply_append_child_same_parent_move_two_records() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    dom.append_child(parent, a);
    dom.append_child(parent, b); // [a, b]

    // Move `a` to the end: [a, b] -> [b, a].
    let records = super::apply_append_child(&mut dom, parent, a);
    assert_eq!(
        records.len(),
        2,
        "a move emits source-removal + destination"
    );
    // Source-removal on the (same) parent: a left its old slot (prev None, next b).
    let src = &records[0];
    assert_eq!(src.target, parent);
    assert_eq!(src.removed_nodes, vec![a]);
    assert!(src.added_nodes.is_empty());
    assert_eq!(src.previous_sibling, None);
    assert_eq!(src.next_sibling, Some(b));
    // Destination prev = b = parent's last child captured pre-adopt (step 6).
    // (`a` is not the last child, so the self-sibling case does not arise here —
    // see apply_append_child_move_last_child_dest_prev_is_self_sibling.)
    let dst = &records[1];
    assert_eq!(dst.target, parent);
    assert_eq!(dst.added_nodes, vec![a]);
    assert_eq!(dst.previous_sibling, Some(b));
    assert_eq!(dst.next_sibling, None);
    assert_eq!(dom.children(parent), vec![b, a]);
}

#[test]
fn apply_append_child_cross_parent_move_two_records() {
    let mut dom = EcsDom::new();
    let p1 = elem(&mut dom, "div");
    let p2 = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let child = elem(&mut dom, "span");
    dom.append_child(p1, a);
    dom.append_child(p1, child); // p1 = [a, child]

    let records = super::apply_append_child(&mut dom, p2, child);
    assert_eq!(records.len(), 2);
    // Source-removal on the OLD parent p1.
    assert_eq!(records[0].target, p1);
    assert_eq!(records[0].removed_nodes, vec![child]);
    assert_eq!(records[0].previous_sibling, Some(a));
    assert_eq!(records[0].next_sibling, None);
    // Destination insertion on the NEW parent p2.
    assert_eq!(records[1].target, p2);
    assert_eq!(records[1].added_nodes, vec![child]);
    assert_eq!(records[1].previous_sibling, None);
    assert_eq!(records[1].next_sibling, None);
}

#[test]
fn apply_insert_before_cross_parent_move_two_records() {
    let mut dom = EcsDom::new();
    let p1 = elem(&mut dom, "div");
    let p2 = elem(&mut dom, "div");
    let moved = elem(&mut dom, "span");
    let r = elem(&mut dom, "span");
    dom.append_child(p1, moved);
    dom.append_child(p2, r);

    let records = super::apply_insert_before(&mut dom, p2, moved, r);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].target, p1);
    assert_eq!(records[0].removed_nodes, vec![moved]);
    let dst = &records[1];
    assert_eq!(dst.target, p2);
    assert_eq!(dst.added_nodes, vec![moved]);
    assert_eq!(dst.previous_sibling, None);
    assert_eq!(dst.next_sibling, Some(r));
    assert_eq!(dom.children(p2), vec![moved, r]);
}

#[test]
fn apply_append_child_move_last_child_dest_prev_is_self_sibling() {
    // `appendChild(<current last child>)` — DOM §4.2.3 insert step 6 captures
    // `previousSibling` (= parent's last child) BEFORE the adopt at step 7.1, so
    // the destination record's previousSibling is the moved node itself (spec
    // self-sibling). Codex PR384 R1.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    dom.append_child(parent, a);
    dom.append_child(parent, b); // [a, b]; b is last

    let records = super::apply_append_child(&mut dom, parent, b);
    assert_eq!(records.len(), 2);
    // Source-removal of b from its old slot: prev=a, next=None.
    assert_eq!(records[0].removed_nodes, vec![b]);
    assert_eq!(records[0].previous_sibling, Some(a));
    assert_eq!(records[0].next_sibling, None);
    // Destination: previousSibling == b itself (spec step-6 pre-adopt capture).
    assert_eq!(records[1].added_nodes, vec![b]);
    assert_eq!(records[1].previous_sibling, Some(b));
    assert_eq!(records[1].next_sibling, None);
    assert_eq!(dom.children(parent), vec![a, b]);
}

#[test]
fn apply_insert_before_noop_move_dest_prev_is_self_sibling() {
    // `insertBefore(node, node.nextSibling)` — no-position-change move. Step 6
    // captures previousSibling = refChild's previous sibling = the moved node
    // itself, before the adopt. Codex PR384 R1.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");
    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c); // [a, b, c]; b is c's previous sibling

    let records = super::apply_insert_before(&mut dom, parent, b, c);
    assert_eq!(records.len(), 2);
    // Source-removal of b: prev=a, next=c.
    assert_eq!(records[0].removed_nodes, vec![b]);
    assert_eq!(records[0].previous_sibling, Some(a));
    assert_eq!(records[0].next_sibling, Some(c));
    // Destination: previousSibling == b itself, nextSibling == c.
    assert_eq!(records[1].added_nodes, vec![b]);
    assert_eq!(records[1].previous_sibling, Some(b));
    assert_eq!(records[1].next_sibling, Some(c));
    assert_eq!(dom.children(parent), vec![a, b, c]);
}

#[test]
fn apply_replace_child_cross_parent_move_source_plus_coalesced() {
    let mut dom = EcsDom::new();
    let p1 = elem(&mut dom, "div");
    let p2 = elem(&mut dom, "div");
    let before = elem(&mut dom, "span");
    let newc = elem(&mut dom, "span");
    let oldc = elem(&mut dom, "span");
    dom.append_child(p1, before);
    dom.append_child(p1, newc); // p1 = [before, newc]
    dom.append_child(p2, oldc); // p2 = [oldc]

    let records = super::apply_replace_child(&mut dom, p2, newc, oldc);
    assert_eq!(records.len(), 2);
    // Source-removal on newc's old parent p1.
    assert_eq!(records[0].target, p1);
    assert_eq!(records[0].removed_nodes, vec![newc]);
    assert_eq!(records[0].previous_sibling, Some(before));
    assert_eq!(records[0].next_sibling, None);
    // Coalesced replace record on p2.
    let c = &records[1];
    assert_eq!(c.target, p2);
    assert_eq!(c.added_nodes, vec![newc]);
    assert_eq!(c.removed_nodes, vec![oldc]);
    assert_eq!(dom.children(p2), vec![newc]);
}

#[test]
fn apply_replace_child_move_step8_referencechild_adjustment() {
    // [A,B,C].replaceChild(C, B): newC (C) is oldC (B)'s next sibling.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");
    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c); // [A, B, C]

    let records = super::apply_replace_child(&mut dom, parent, c, b);
    assert_eq!(records.len(), 2);
    // Source-removal: C's siblings step over B (removed first) -> prev = A.
    assert_eq!(records[0].target, parent);
    assert_eq!(records[0].removed_nodes, vec![c]);
    assert_eq!(records[0].previous_sibling, Some(a));
    assert_eq!(records[0].next_sibling, None);
    // Coalesced: step-8 next = C's next (None), NOT C itself; prev = B's prev = A.
    let coalesced = &records[1];
    assert_eq!(coalesced.added_nodes, vec![c]);
    assert_eq!(coalesced.removed_nodes, vec![b]);
    assert_eq!(coalesced.previous_sibling, Some(a));
    assert_eq!(coalesced.next_sibling, None);
    assert_eq!(dom.children(parent), vec![a, c]);
}

#[test]
fn apply_replace_child_move_prev_may_equal_new_child() {
    // [A,B,C].replaceChild(A, B): coalesced previousSibling == A == newChild
    // is spec-faithful (replace step 9 has no adjustment).
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");
    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c); // [A, B, C]

    let records = super::apply_replace_child(&mut dom, parent, a, b);
    assert_eq!(records.len(), 2);
    // Source-removal: A's siblings step over B -> next = C, prev None.
    assert_eq!(records[0].removed_nodes, vec![a]);
    assert_eq!(records[0].previous_sibling, None);
    assert_eq!(records[0].next_sibling, Some(c));
    // Coalesced: previousSibling = B's prev = A = newChild (spec-faithful).
    let coalesced = &records[1];
    assert_eq!(coalesced.added_nodes, vec![a]);
    assert_eq!(coalesced.removed_nodes, vec![b]);
    assert_eq!(coalesced.previous_sibling, Some(a));
    assert_eq!(coalesced.next_sibling, Some(c));
}

#[test]
fn apply_replace_child_self_replace_no_records() {
    // replaceChild(X, X): rejected by EcsDom::replace_child -> no record
    // (pre-existing browser-parity no-op, unchanged by B1.2a).
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let x = elem(&mut dom, "span");
    dom.append_child(parent, x);

    let records = super::apply_replace_child(&mut dom, parent, x, x);
    assert!(records.is_empty());
    assert_eq!(dom.children(parent), vec![x]);
}

#[test]
fn apply_set_attribute() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");

    let m = Mutation::SetAttribute {
        entity: e,
        name: "class".into(),
        value: "active".into(),
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::Attribute);
    assert_eq!(record.attribute_name.as_deref(), Some("class"));
    assert_eq!(record.old_value, None);

    let attrs = dom.world().get::<&Attributes>(e).unwrap();
    assert_eq!(attrs.get("class"), Some("active"));
}

#[test]
fn apply_set_attribute_records_old_value() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
        attrs.set("class", "old");
    }

    let m = Mutation::SetAttribute {
        entity: e,
        name: "class".into(),
        value: "new".into(),
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.old_value.as_deref(), Some("old"));
}

#[test]
fn apply_remove_attribute() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
        attrs.set("id", "test");
    }

    let m = Mutation::RemoveAttribute {
        entity: e,
        name: "id".into(),
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::Attribute);
    assert_eq!(record.attribute_name.as_deref(), Some("id"));
    assert_eq!(record.old_value.as_deref(), Some("test"));

    let attrs = dom.world().get::<&Attributes>(e).unwrap();
    assert!(!attrs.contains("id"));
}

#[test]
fn apply_set_text_content() {
    let mut dom = EcsDom::new();
    let text = dom.create_text("hello");

    let m = Mutation::SetTextContent {
        entity: text,
        text: "world".into(),
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::CharacterData);
    assert_eq!(record.old_value.as_deref(), Some("hello"));

    let tc = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(tc.0, "world");
}

#[test]
fn apply_remove_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "p");
    dom.append_child(parent, a);
    dom.append_child(parent, b);

    let m = Mutation::RemoveChild { parent, child: a };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::ChildList);
    assert_eq!(record.target, parent);
    assert_eq!(record.removed_nodes, vec![a]);
    assert_eq!(record.previous_sibling, None);
    assert_eq!(record.next_sibling, Some(b));
    assert_eq!(dom.children(parent), vec![b]);
}

#[test]
fn apply_replace_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let old = elem(&mut dom, "span");
    let new = elem(&mut dom, "p");
    dom.append_child(parent, old);

    let m = Mutation::ReplaceChild {
        parent,
        new_child: new,
        old_child: old,
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_eq!(record.kind, MutationKind::ChildList);
    assert_eq!(record.added_nodes, vec![new]);
    assert_eq!(record.removed_nodes, vec![old]);
    assert_eq!(dom.children(parent), vec![new]);
    assert_eq!(dom.get_parent(old), None);
}

#[test]
fn apply_append_child_does_not_leak_shadow_root_as_previous_sibling() {
    // PR201 Copilot R4 / F3 regression: `apply_append_child` was
    // capturing `prev_sibling` via raw `get_last_child(parent)`,
    // which returns the internal ShadowRoot when the host has no
    // light-tree children yet. The fix walks via
    // `children_iter_rev` (which skips ShadowRoot entities).
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let host = elem(&mut dom, "div");
    let _ = dom.append_child(root, host);
    let shadow_root = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
        .expect("attach closed shadow");
    // Sanity: raw `get_last_child(host)` IS the shadow root —
    // confirms the helper would leak without the fix.
    assert_eq!(
        dom.get_last_child(host),
        Some(shadow_root),
        "shadow root is the only sibling at this point"
    );
    let new_child = elem(&mut dom, "span");
    let m = Mutation::AppendChild {
        parent: host,
        child: new_child,
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_ne!(
        record.previous_sibling,
        Some(shadow_root),
        "MutationRecord.previous_sibling must not leak shadow root"
    );
    assert_eq!(
        record.previous_sibling, None,
        "no exposed prev sibling (shadow root skipped)"
    );
}

#[test]
fn apply_remove_child_does_not_leak_shadow_root_as_previous_sibling() {
    // Pre-existing apply_remove_child path now uses
    // `prev_exposed_sibling` too. Lock the no-leak invariant.
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let host = elem(&mut dom, "div");
    let _ = dom.append_child(root, host);
    let shadow_root = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
        .expect("attach closed shadow");
    let child = elem(&mut dom, "span");
    let _ = dom.append_child(host, child);
    assert_eq!(dom.get_prev_sibling(child), Some(shadow_root));
    let m = Mutation::RemoveChild {
        parent: host,
        child,
    };
    let record = expect_one(apply_mutation(&m, &mut dom));
    assert_ne!(record.previous_sibling, Some(shadow_root));
    assert_eq!(record.previous_sibling, None);
}

/// Codex #335 R10 F31: a buffered `style` attribute mutation applied via
/// the deferred flush (which bypasses `EcsDom::set_attribute`) must
/// still invalidate a lazily-hydrated `InlineStyle` cache, else a later
/// CSSOM read resurrects stale declarations.
#[test]
fn apply_style_attribute_invalidates_inline_style_cache() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
        attrs.set("style", "color: red");
    }
    // Simulate a prior `el.style.*` read that hydrated the cache.
    let mut style = elidex_ecs::InlineStyle::default();
    style.set("color", "red");
    dom.world_mut().insert_one(e, style).unwrap();
    assert!(dom.world().get::<&elidex_ecs::InlineStyle>(e).is_ok());

    // A buffered SetAttribute("style", …) must drop the stale cache.
    let m = Mutation::SetAttribute {
        entity: e,
        name: "style".into(),
        value: "color: blue".into(),
    };
    expect_one(apply_mutation(&m, &mut dom));
    assert!(
        dom.world().get::<&elidex_ecs::InlineStyle>(e).is_err(),
        "buffered SetAttribute('style') left a stale InlineStyle cache"
    );

    // Re-hydrate, then a buffered RemoveAttribute must also drop it.
    let mut style = elidex_ecs::InlineStyle::default();
    style.set("color", "blue");
    dom.world_mut().insert_one(e, style).unwrap();
    let m = Mutation::RemoveAttribute {
        entity: e,
        name: "style".into(),
    };
    expect_one(apply_mutation(&m, &mut dom));
    assert!(
        dom.world().get::<&elidex_ecs::InlineStyle>(e).is_err(),
        "buffered RemoveAttribute('style') left a stale InlineStyle cache"
    );
}

// ---------------------------------------------------------------------------
// DocumentFragment expansion (B1.2-fragment) — WHATWG DOM §4.2.3 insert/replace
// ---------------------------------------------------------------------------

/// Build a detached `DocumentFragment` holding `children` (in order).
fn fragment_of(dom: &mut EcsDom, children: &[Entity]) -> Entity {
    let frag = dom.create_document_fragment();
    for &c in children {
        dom.append_child(frag, c);
    }
    frag
}

#[test]
fn append_fragment_two_children() {
    // appendChild(frag[a, b]) — §4.2.3 insert: expand the fragment's children,
    // emit the step-4.2 fragment record THEN the step-8 destination record.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let first = elem(&mut dom, "span");
    dom.append_child(parent, first); // parent = [first]
    let a = elem(&mut dom, "i");
    let b = elem(&mut dom, "u");
    let frag = fragment_of(&mut dom, &[a, b]);

    let records = super::apply_append_child(&mut dom, parent, frag);
    assert_eq!(records.len(), 2, "fragment record + destination record");
    // step 4.2 fragment record: on the fragment, removedNodes = its children.
    assert_eq!(records[0].kind, MutationKind::ChildList);
    assert_eq!(records[0].target, frag);
    assert_eq!(records[0].added_nodes, Vec::<Entity>::new());
    assert_eq!(records[0].removed_nodes, vec![a, b]);
    assert_eq!(records[0].previous_sibling, None);
    assert_eq!(records[0].next_sibling, None);
    // step 8 destination record: addedNodes = the expanded children, previousSibling
    // = parent's last child before the move (= `first`), nextSibling null (append).
    assert_eq!(records[1].target, parent);
    assert_eq!(records[1].added_nodes, vec![a, b]);
    assert!(records[1].removed_nodes.is_empty());
    assert_eq!(records[1].previous_sibling, Some(first));
    assert_eq!(records[1].next_sibling, None);
    // Tree: children moved out of the fragment into the parent.
    assert_eq!(dom.children(parent), vec![first, a, b]);
    assert!(
        dom.children(frag).is_empty(),
        "fragment emptied after expand"
    );
}

#[test]
fn append_fragment_one_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let only = elem(&mut dom, "span");
    let frag = fragment_of(&mut dom, &[only]);

    let records = super::apply_append_child(&mut dom, parent, frag);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].target, frag);
    assert_eq!(records[0].removed_nodes, vec![only]);
    assert_eq!(records[1].added_nodes, vec![only]);
    assert_eq!(records[1].previous_sibling, None);
    assert_eq!(dom.children(parent), vec![only]);
}

#[test]
fn append_empty_fragment_no_record() {
    // §4.2.3 insert step 3: count 0 → return, NO record at all.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let existing = elem(&mut dom, "span");
    dom.append_child(parent, existing);
    let frag = dom.create_document_fragment();

    let records = super::apply_append_child(&mut dom, parent, frag);
    assert!(records.is_empty(), "empty-fragment append yields no record");
    assert_eq!(dom.children(parent), vec![existing], "parent unchanged");
}

#[test]
fn insert_before_fragment() {
    // insertBefore(frag[a, b], ref) — expand before ref; dest prev = ref's old
    // prev sibling, dest next = ref.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let x = elem(&mut dom, "x");
    let r = elem(&mut dom, "r");
    dom.append_child(parent, x);
    dom.append_child(parent, r); // [x, r]
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    let frag = fragment_of(&mut dom, &[a, b]);

    let records = super::apply_insert_before(&mut dom, parent, frag, r);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].target, frag);
    assert_eq!(records[0].removed_nodes, vec![a, b]);
    assert_eq!(records[1].target, parent);
    assert_eq!(records[1].added_nodes, vec![a, b]);
    assert_eq!(records[1].previous_sibling, Some(x));
    assert_eq!(records[1].next_sibling, Some(r));
    assert_eq!(dom.children(parent), vec![x, a, b, r]);
    assert!(dom.children(frag).is_empty());
}

#[test]
fn insert_before_empty_fragment_no_record() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let r = elem(&mut dom, "r");
    dom.append_child(parent, r);
    let frag = dom.create_document_fragment();

    let records = super::apply_insert_before(&mut dom, parent, frag, r);
    assert!(records.is_empty());
    assert_eq!(dom.children(parent), vec![r]);
}

#[test]
fn insert_before_fragment_bad_reference_is_failure() {
    // A reference child not in `parent` must NOT emit records (else a bad ref
    // would surface records for children that were never inserted). Empty list
    // ⇒ the handler maps it to a hierarchy error.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let other = elem(&mut dom, "div");
    let stray = elem(&mut dom, "span");
    dom.append_child(other, stray); // stray ∈ other, NOT parent
    let a = elem(&mut dom, "a");
    let frag = fragment_of(&mut dom, &[a]);

    let records = super::apply_insert_before(&mut dom, parent, frag, stray);
    assert!(records.is_empty(), "bad reference child yields no records");
    // The fragment children were not moved.
    assert_eq!(dom.children(frag), vec![a]);
    assert!(dom.children(parent).is_empty());
}

#[test]
fn replace_child_fragment() {
    // replaceChild(frag[a, b], old) — §4.2.3 replace: fragment record THEN one
    // coalesced record (added = expanded children, removed = [old]).
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let x = elem(&mut dom, "x");
    let old = elem(&mut dom, "old");
    let y = elem(&mut dom, "y");
    dom.append_child(parent, x);
    dom.append_child(parent, old);
    dom.append_child(parent, y); // [x, old, y]
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    let frag = fragment_of(&mut dom, &[a, b]);

    let records = super::apply_replace_child(&mut dom, parent, frag, old);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].target, frag);
    assert_eq!(records[0].removed_nodes, vec![a, b]);
    let coalesced = &records[1];
    assert_eq!(coalesced.target, parent);
    assert_eq!(coalesced.added_nodes, vec![a, b]);
    assert_eq!(coalesced.removed_nodes, vec![old]);
    assert_eq!(coalesced.previous_sibling, Some(x)); // old's prev (step 9)
    assert_eq!(coalesced.next_sibling, Some(y)); // old's next (step 7)
    assert_eq!(dom.children(parent), vec![x, a, b, y]);
    assert!(dom.children(frag).is_empty());
}

#[test]
fn replace_child_fragment_at_end() {
    // old is the last child → referenceChild = None → children appended after
    // old's previous sibling; coalesced nextSibling = None.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let x = elem(&mut dom, "x");
    let old = elem(&mut dom, "old");
    dom.append_child(parent, x);
    dom.append_child(parent, old); // [x, old]
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    let frag = fragment_of(&mut dom, &[a, b]);

    let records = super::apply_replace_child(&mut dom, parent, frag, old);
    assert_eq!(records.len(), 2);
    let coalesced = &records[1];
    assert_eq!(coalesced.added_nodes, vec![a, b]);
    assert_eq!(coalesced.removed_nodes, vec![old]);
    assert_eq!(coalesced.previous_sibling, Some(x));
    assert_eq!(coalesced.next_sibling, None);
    assert_eq!(dom.children(parent), vec![x, a, b]);
}

#[test]
fn replace_child_empty_fragment() {
    // §4.2.3 replace: even an empty fragment removes oldChild (step 11) and
    // queues ONE coalesced record (added = «», removed = [old]) — no fragment
    // record (the nested insert returns at step 3).
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let x = elem(&mut dom, "x");
    let old = elem(&mut dom, "old");
    let y = elem(&mut dom, "y");
    dom.append_child(parent, x);
    dom.append_child(parent, old);
    dom.append_child(parent, y);
    let frag = dom.create_document_fragment();

    let records = super::apply_replace_child(&mut dom, parent, frag, old);
    assert_eq!(
        records.len(),
        1,
        "only the coalesced record, no fragment record"
    );
    let coalesced = &records[0];
    assert_eq!(coalesced.target, parent);
    assert!(coalesced.added_nodes.is_empty());
    assert_eq!(coalesced.removed_nodes, vec![old]);
    assert_eq!(coalesced.previous_sibling, Some(x));
    assert_eq!(coalesced.next_sibling, Some(y));
    assert_eq!(dom.children(parent), vec![x, y]);
}

#[test]
fn append_fragment_that_is_ancestor_of_parent_rejects_atomically() {
    // Codex PR387 R1 F1: a non-empty fragment that is a host-including inclusive
    // ancestor of `parent` (frag > ancestor > parent) must be rejected ATOMICALLY
    // (§4.2.1 step 2) — no partial move of the non-cyclic children, empty record
    // list (the handler maps it to a hierarchy error).
    let mut dom = EcsDom::new();
    let frag = dom.create_document_fragment();
    let ancestor = elem(&mut dom, "div");
    let other = elem(&mut dom, "span");
    dom.append_child(frag, ancestor);
    dom.append_child(frag, other); // frag = [ancestor, other]
    let parent = elem(&mut dom, "p");
    dom.append_child(ancestor, parent); // ancestor > parent, so frag is an ancestor of parent

    let records = super::apply_append_child(&mut dom, parent, frag);
    assert!(
        records.is_empty(),
        "cyclic fragment append rejected, no records"
    );
    // Nothing moved: `other` (non-cyclic) must NOT have been partially relinked.
    assert_eq!(dom.children(frag), vec![ancestor, other]);
    assert!(dom.children(parent).is_empty());
}

#[test]
fn append_self_fragment_returns_empty() {
    // Codex PR387 R1 F3 (apply side): `frag.appendChild(frag)` — parent == child ==
    // an (empty) fragment is a host-including inclusive ancestor of itself, so
    // apply returns empty (the handler then raises a hierarchy error rather than
    // the prior silent success).
    let mut dom = EcsDom::new();
    let frag = dom.create_document_fragment();
    let records = super::apply_append_child(&mut dom, frag, frag);
    assert!(records.is_empty());
}

// Codex PR387 R1 F2 (the >MAX_ANCESTOR_DEPTH fragment "moves ALL children" case):
// the regression that `expand_fragment` snapshots via `child_list_uncapped` (not the
// 10k-capped `children_iter`) is guarded by (a) `expand_fragment` calling the
// canonically-uncapped helper and (b) `child_list_uncapped`'s own no-truncation
// contract in elidex-ecs. A full-cap apply test here would build/move 10_001 nodes
// — O(n²) via per-node `index_in_parent`, ~60s of permanent CI latency for an
// extreme edge — so it is intentionally NOT in the normal suite (Codex PR387 R5 H1).

#[test]
fn replace_child_fragment_old_not_child_is_failure() {
    // Deferred-flush safety: apply_replace_child's fragment branch re-checks
    // oldChild ∈ parent (the apply_mutation path lacks the dom-api precheck).
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let other = elem(&mut dom, "div");
    let stray = elem(&mut dom, "old");
    dom.append_child(other, stray); // stray ∈ other, not parent
    let a = elem(&mut dom, "a");
    let frag = fragment_of(&mut dom, &[a]);

    let records = super::apply_replace_child(&mut dom, parent, frag, stray);
    assert!(records.is_empty());
    assert_eq!(dom.children(frag), vec![a], "fragment untouched on failure");
}
