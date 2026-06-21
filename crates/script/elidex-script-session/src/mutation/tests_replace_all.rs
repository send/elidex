//! B1.2b replace-all / self-reference advance / shadow-root-rejection record
//! tests — split from `tests.rs` to keep each test file under the 1000-line
//! convention (Codex PR393 R7).

use super::tests::{elem, expect_one, fragment_of};
use super::*;

#[test]
fn apply_replace_all_clears_then_inserts_single_combined_record() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let old1 = elem(&mut dom, "a");
    let old2 = elem(&mut dom, "b");
    dom.append_child(parent, old1);
    dom.append_child(parent, old2);
    let fresh = elem(&mut dom, "c");

    let record = expect_one(super::apply_replace_all(&mut dom, parent, Some(fresh)));
    assert_eq!(record.kind, MutationKind::ChildList);
    assert_eq!(record.target, parent);
    assert_eq!(record.removed_nodes, vec![old1, old2]);
    assert_eq!(record.added_nodes, vec![fresh]);
    assert_eq!(record.previous_sibling, None);
    assert_eq!(record.next_sibling, None);
    assert_eq!(dom.children(parent), vec![fresh]);
}

#[test]
fn apply_replace_all_fragment_added_nodes_are_expanded_children() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let old = elem(&mut dom, "a");
    dom.append_child(parent, old);
    let c1 = elem(&mut dom, "x");
    let c2 = elem(&mut dom, "y");
    let frag = fragment_of(&mut dom, &[c1, c2]);

    // TWO records: the §4.2.3 step-4.2 fragment record (frag emptied — NOT suppressed)
    // THEN the step-7 combined record. (Codex PR393 R1: discarding the fragment record
    // hid `replaceChildren(frag)` emptying frag from an observer attached to frag.)
    let records = super::apply_replace_all(&mut dom, parent, Some(frag));
    assert_eq!(records.len(), 2, "fragment record + combined record");
    // step-4.2 fragment record: target = frag, removedNodes = frag's children.
    assert_eq!(records[0].target, frag);
    assert_eq!(records[0].removed_nodes, vec![c1, c2]);
    assert!(records[0].added_nodes.is_empty());
    // step-7 combined record on parent: addedNodes = frag's children, removedNodes = old.
    assert_eq!(records[1].target, parent);
    assert_eq!(records[1].added_nodes, vec![c1, c2]);
    assert_eq!(records[1].removed_nodes, vec![old]);
    assert_eq!(dom.children(parent), vec![c1, c2]);
    assert!(
        dom.children(frag).is_empty(),
        "fragment emptied by expansion"
    );
}

#[test]
fn apply_replace_all_null_node_clears_with_one_record() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let old = elem(&mut dom, "a");
    dom.append_child(parent, old);

    let record = expect_one(super::apply_replace_all(&mut dom, parent, None));
    assert_eq!(record.removed_nodes, vec![old]);
    assert!(record.added_nodes.is_empty());
    assert!(dom.children(parent).is_empty());
}

#[test]
fn apply_replace_all_empty_parent_null_node_emits_no_record() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    // §4.2.3 replace-all step 7: addedNodes ∪ removedNodes empty ⇒ no record.
    let records = super::apply_replace_all(&mut dom, parent, None);
    assert!(records.is_empty());
}

#[test]
fn apply_replace_all_cross_parent_move_emits_source_removal() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let old = elem(&mut dom, "a");
    dom.append_child(parent, old);
    let other = elem(&mut dom, "div");
    let moved = elem(&mut dom, "m");
    let sib = elem(&mut dom, "s");
    dom.append_child(other, moved);
    dom.append_child(other, sib); // other = [moved, sib]

    // `replaceChildren(moved)` where `moved` belongs to `other`: §4.2.3 replace-all
    // step 6 insert → §4.5 adopt removes `moved` from `other` with the adopt's plain
    // (unsuppressed) "remove" → a source-parent removal record, THEN the step-7
    // combined record on parent. (Codex PR393 R1 finding 4.)
    let records = super::apply_replace_all(&mut dom, parent, Some(moved));
    assert_eq!(records.len(), 2, "source removal + combined record");
    assert_eq!(records[0].target, other);
    assert_eq!(records[0].removed_nodes, vec![moved]);
    assert_eq!(records[0].next_sibling, Some(sib));
    assert_eq!(records[1].target, parent);
    assert_eq!(records[1].added_nodes, vec![moved]);
    assert_eq!(records[1].removed_nodes, vec![old]);
    assert_eq!(dom.children(parent), vec![moved]);
    assert_eq!(dom.children(other), vec![sib]);
}

#[test]
fn apply_insert_before_self_reference_advances_to_noop_move() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    dom.append_child(parent, a);
    dom.append_child(parent, b); // [a, b]

    // `insertBefore(a, a)`: §4.2.3 pre-insert step 3 advances referenceChild to a's
    // next sibling (b), so it is a no-position-change move (2 records), NOT the
    // self-reference rejection `EcsDom::insert_before` would otherwise return as an
    // empty list. (Codex PR393 R1 finding 3.)
    let records = super::apply_insert_before(&mut dom, parent, a, a);
    assert_eq!(
        records.len(),
        2,
        "self-reference is a no-position-change move"
    );
    assert_eq!(records[0].removed_nodes, vec![a]);
    assert_eq!(records[1].added_nodes, vec![a]);
    assert_eq!(dom.children(parent), vec![a, b], "a stays in place");
}

#[test]
fn apply_insert_before_self_reference_last_child_appends() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    let b = elem(&mut dom, "b");
    dom.append_child(parent, a);
    dom.append_child(parent, b); // [a, b]; b is last

    // `insertBefore(b, b)` with b last → advance to b.nextSibling = null → append b
    // (still a no-position-change same-parent move, 2 records). (Codex PR393 R1 finding 3.)
    let records = super::apply_insert_before(&mut dom, parent, b, b);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].removed_nodes, vec![b]);
    assert_eq!(records[1].added_nodes, vec![b]);
    assert_eq!(dom.children(parent), vec![a, b], "b stays last");
}

#[test]
fn apply_insert_before_self_reference_with_orphan_ref_is_rejected() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "a");
    dom.append_child(parent, a); // parent = [a]
    let orphan = elem(&mut dom, "orphan"); // not a child of parent

    // `insertBefore(orphan, orphan)`: referenceChild (orphan) ∉ parent → §4.2.3
    // "ensure pre-insertion validity" step 3 NotFound (empty list → handler error).
    // The pre-insert step-3 self-reference advance must NOT fire before ref ∈ parent is
    // established, else it would append the orphan. (Codex PR393 R2 regression of R1 finding 3.)
    let records = super::apply_insert_before(&mut dom, parent, orphan, orphan);
    assert!(records.is_empty(), "ref ∉ parent must fail, not append");
    assert_eq!(dom.children(parent), vec![a], "tree unchanged");
    assert!(
        dom.get_parent(orphan).is_none(),
        "orphan not moved into parent"
    );
}

#[test]
fn apply_insert_before_self_reference_with_other_parent_ref_is_rejected() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let other = elem(&mut dom, "div");
    let x = elem(&mut dom, "x");
    dom.append_child(other, x); // x ∈ other, not parent

    // `parent.insertBefore(x, x)` where x belongs to `other` → ref ∉ parent → reject.
    let records = super::apply_insert_before(&mut dom, parent, x, x);
    assert!(records.is_empty());
    assert_eq!(
        dom.get_parent(x),
        Some(other),
        "x stays under its real parent"
    );
}

#[test]
fn apply_layer_rejects_shadow_root_as_inserted_node() {
    use elidex_ecs::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");
    let other = elem(&mut dom, "div");
    let kid = elem(&mut dom, "kid");
    dom.append_child(other, kid); // other = [kid]

    // A ShadowRoot is bound to its host by the immovable host edge (§4.8) — inserting
    // it into the light tree is a HierarchyRequestError. The engine-independent apply_*
    // layer rejects it for ALL runtimes (Codex PR393 R5: boa's arg path lacks the VM's
    // `normalize_mixin_arg` ShadowRoot rejection, so the shared layer must enforce it).
    assert!(super::apply_append_child(&mut dom, other, sr).is_empty());
    assert!(super::apply_insert_before(&mut dom, other, sr, kid).is_empty());
    assert!(super::apply_replace_child(&mut dom, other, sr, kid).is_empty());
    assert!(super::apply_replace_all(&mut dom, other, Some(sr)).is_empty());

    // The shadow root stays attached to its host, and `other` is untouched —
    // crucially `apply_replace_all` did NOT clear `other`'s children before rejecting.
    assert!(dom.is_shadow_root(sr), "shadow root still a shadow root");
    assert_eq!(dom.children(other), vec![kid], "other left intact");
}
