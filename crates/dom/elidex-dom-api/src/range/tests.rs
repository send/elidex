//! Range unit tests — split out from `range/mod.rs` per the ~1000-line
//! file convention (PR186 R2 #5).

#![allow(unused_must_use)]

use super::*;
use elidex_ecs::{Attributes, EcsDom, ShadowRootMode, TextContent};
use elidex_script_session::MutationKind;

fn build_range_tree() -> (EcsDom, Entity, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("Hello");
    let span = dom.create_element("span", Attributes::default());
    let t2 = dom.create_text(" World");

    dom.append_child(root, t1);
    dom.append_child(root, span);
    dom.append_child(span, t2);

    (dom, root, t1, span, t2)
}

#[test]
fn range_defaults_collapsed() {
    let (dom, root, _, _, _) = build_range_tree();
    let range = Range::new(root);
    assert!(range.collapsed());
    assert_eq!(range.common_ancestor_container(&dom), root);
}

#[test]
fn range_set_start_end() {
    let (dom, _root, t1, _span, t2) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 2);
    range.set_end(t2, 3);
    assert!(!range.collapsed());
    assert_eq!(range.start_offset, 2);
    assert_eq!(range.end_offset, 3);
    // Common ancestor should be root (div).
    let _ca = range.common_ancestor_container(&dom);
}

#[test]
fn range_collapsed() {
    let (_dom, root, _, _, _) = build_range_tree();
    let mut range = Range::new(root);
    range.set_start(root, 0);
    range.set_end(root, 0);
    assert!(range.collapsed());
}

#[test]
fn range_common_ancestor() {
    let (dom, root, t1, _span, t2) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 0);
    range.set_end(t2, 3);
    assert_eq!(range.common_ancestor_container(&dom), root);
}

#[test]
fn range_select_node_contents() {
    let (dom, _root, t1, _, _) = build_range_tree();
    let mut range = Range::new(t1);
    range.select_node_contents(t1, &dom);
    assert_eq!(range.start_offset, 0);
    assert_eq!(range.end_offset, 5); // "Hello" length
}

#[test]
fn range_clone() {
    let (_dom, root, _, _, _) = build_range_tree();
    let mut range = Range::new(root);
    range.set_start(root, 1);
    range.set_end(root, 2);
    let cloned = range.clone_range();
    assert_eq!(cloned.start_offset, 1);
    assert_eq!(cloned.end_offset, 2);
}

#[test]
fn range_compare_boundary_points() {
    let (dom, root, _t1, _span, _t2) = build_range_tree();
    let mut r1 = Range::new(root);
    r1.set_start(root, 0);
    r1.set_end(root, 2);

    let mut r2 = Range::new(root);
    r2.set_start(root, 1);
    r2.set_end(root, 3);

    assert_eq!(r1.compare_boundary_points(START_TO_START, &r2, &dom), -1);
}

#[test]
fn range_to_string_same_text_node() {
    let (dom, _root, t1, _, _) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 1);
    range.set_end(t1, 4);
    assert_eq!(range.to_string(&dom), "ell");
}

#[test]
fn range_delete_contents_same_text() {
    let (mut dom, _root, t1, _, _) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 1);
    range.set_end(t1, 4);
    range.delete_contents(&mut dom);

    let tc = dom.world().get::<&TextContent>(t1).unwrap();
    assert_eq!(tc.0, "Ho");
    assert!(range.collapsed());
}

#[test]
fn range_delete_contents_splits_text() {
    let (mut dom, _root, t1, _, _) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 2);
    range.set_end(t1, 4);
    range.delete_contents(&mut dom);

    let tc = dom.world().get::<&TextContent>(t1).unwrap();
    assert_eq!(tc.0, "Heo");
}

#[test]
fn range_extract_contents() {
    let (mut dom, _root, t1, _, _) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 1);
    range.set_end(t1, 4);
    let (frag, _records) = range.extract_contents(&mut dom);

    // Fragment should contain "ell".
    let children: Vec<_> = dom.children_iter(frag).collect();
    assert_eq!(children.len(), 1);
    let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
    assert_eq!(tc.0, "ell");

    // Original text should be "Ho".
    let tc = dom.world().get::<&TextContent>(t1).unwrap();
    assert_eq!(tc.0, "Ho");
}

/// Codex PR418 R1 — `deleteContents` over a cross-container range spanning a
/// nested subtree must emit **one** childList removal record per top-level
/// contained node (DOM §5.5 omit-nested), NOT an extra record per descendant.
/// Before the `contained_top_level_nodes` fix, `<wrap><inner/></wrap>` produced
/// a spurious second removal record targeting the already-detached `wrap`.
#[test]
fn range_delete_contents_omits_nested_removal_records() {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("a");
    let wrap = dom.create_element("wrap", Attributes::default());
    let inner = dom.create_element("inner", Attributes::default());
    let t2 = dom.create_text("b");
    dom.append_child(root, t1);
    dom.append_child(root, wrap);
    dom.append_child(wrap, inner);
    dom.append_child(root, t2);

    // Cross-container range: start in t1, end in t2 → spans `wrap` (and its
    // descendant `inner`) fully.
    let mut range = Range::new(root);
    range.set_start(t1, 1);
    range.set_end(t2, 0);
    let records = range.delete_contents(&mut dom);

    // §5.5 deleteContents cross-container order = step 9 start-trunc →
    // step 10 removals → step 11 end-trunc. Both boundary nodes are
    // CharacterData, so steps 9/11 each queue a characterData record
    // (B1.3-ii) — unconditionally per the spec, even though the splice
    // counts here are zero-width. `wrap` is the only top-level removal
    // (`inner` is omitted: its parent `wrap` is also contained).
    assert_eq!(records.len(), 3, "start-trunc + wrap removal + end-trunc");
    // Step 9: start-trunc on t1 (characterData).
    assert_eq!(records[0].kind, MutationKind::CharacterData);
    assert_eq!(records[0].target, t1);
    // Step 10: childList removal of the top-level `wrap`.
    assert_eq!(records[1].kind, MutationKind::ChildList);
    assert_eq!(records[1].target, root);
    assert_eq!(records[1].removed_nodes, vec![wrap]);
    // Step 11: end-trunc on t2 (characterData).
    assert_eq!(records[2].kind, MutationKind::CharacterData);
    assert_eq!(records[2].target, t2);
    // `wrap` (and its subtree) is gone from `root`.
    let kids: Vec<_> = dom.children_iter(root).collect();
    assert_eq!(kids, vec![t1, t2]);
}

/// Codex PR418 R2 — a cross-container range that starts on an **element** with
/// `start_offset > 0` must not remove (or emit records for) the siblings before
/// that offset. The preorder candidate walk starts at the container's first
/// child, so without the DOM §5.5 containment filter `c0` (index 0) would be
/// wrongly removed when the range starts at `root[1]`.
#[test]
fn range_delete_contents_excludes_nodes_before_element_start_offset() {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let c0 = dom.create_element("a", Attributes::default());
    let c1 = dom.create_element("b", Attributes::default());
    let c2 = dom.create_element("c", Attributes::default());
    let tail = dom.create_text("z");
    dom.append_child(root, c0);
    dom.append_child(root, c1);
    dom.append_child(root, c2);
    dom.append_child(root, tail);

    // Range [root:1 .. tail:0] → cross-container (root != tail); contains c1, c2
    // (indices 1,2) but NOT c0 (index 0, before the start offset).
    let mut range = Range::new(root);
    range.set_start(root, 1);
    range.set_end(tail, 0);
    let records = range.delete_contents(&mut dom);

    // Start container `root` is an Element (not CharacterData) → no step-9
    // record; c1, c2 are the contained top-level removals (step 10); the
    // end container `tail` is a Text (CharacterData) → one step-11
    // end-trunc characterData record (B1.3-ii), zero-width but
    // unconditional per §5.5 step 11.
    assert_eq!(records.len(), 3, "c1 + c2 removals + tail end-trunc");
    assert_eq!(records[0].kind, MutationKind::ChildList);
    assert_eq!(records[0].removed_nodes, vec![c1]);
    assert_eq!(records[1].kind, MutationKind::ChildList);
    assert_eq!(records[1].removed_nodes, vec![c2]);
    assert_eq!(records[2].kind, MutationKind::CharacterData);
    assert_eq!(records[2].target, tail);
    // c0 (before the start offset) and tail (end container) survive.
    let kids: Vec<_> = dom.children_iter(root).collect();
    assert_eq!(kids, vec![c0, tail]);
}

// Codex PR418 R2 (fragment wider than MAX_ANCESTOR_DEPTH → uncapped new_offset)
// is fixed by-construction: `insert_node` now reads `nodes` / `new_offset`
// through `child_list_uncapped`, the SAME uncapped accessor `apply_*` uses to
// expand the fragment, so the move count and the offset count cannot diverge.
// No dedicated test — exercising it needs >10,000 appended children, an O(n²)
// sibling-walk that costs ~24s and only re-asserts the cap constant, which the
// supported-surface testing policy does not warrant for a by-construction fix.

/// Codex PR418 R1 — `Range.insertNode(shadowRoot)` at a Text boundary must be
/// rejected (returns `None`) **before** the step-7 text split, leaving the DOM
/// unmutated. A `ShadowRoot` is not an insertable node (DOM §4.8); the
/// rejection previously happened in the `apply_*` layer AFTER the split, so the
/// text node was split while the native reported `HierarchyRequestError`.
#[test]
fn range_insert_node_rejects_shadow_root_before_split() {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t = dom.create_text("hello");
    dom.append_child(root, t);
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(root, host);
    let shadow = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();

    let mut range = Range::new(t);
    range.set_start(t, 2);
    range.set_end(t, 2);
    let outcome = range.insert_node(&mut dom, shadow);

    assert!(
        outcome.is_none(),
        "inserting a ShadowRoot must be rejected, not inserted"
    );
    // The text node must NOT have been split — still the original single node.
    let tc = dom.world().get::<&TextContent>(t).unwrap();
    assert_eq!(tc.0, "hello");
    // `t` is still root's first child with no spurious split-tail sibling.
    let kids: Vec<_> = dom.children_iter(root).collect();
    assert_eq!(kids, vec![t, host]);
}

#[test]
fn range_to_string_utf16_offsets() {
    // Test that Range offsets are treated as UTF-16 code units.
    // U+1F600 is 4 bytes in UTF-8 but 2 UTF-16 code units.
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t = dom.create_text("A\u{1F600}B"); // "A<emoji>B" = 3 chars, 4 UTF-16 units
    dom.append_child(root, t);

    let mut range = Range::new(t);
    // UTF-16: A(1) + surrogate pair(2) + B(1) = 4 units
    // offset 1..3 should extract the emoji (surrogate pair)
    range.set_start(t, 1);
    range.set_end(t, 3);
    assert_eq!(range.to_string(&dom), "\u{1F600}");

    // offset 3..4 should extract "B"
    range.set_start(t, 3);
    range.set_end(t, 4);
    assert_eq!(range.to_string(&dom), "B");
}

#[test]
fn range_delete_utf16_offsets() {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t = dom.create_text("A\u{1F600}B");
    dom.append_child(root, t);

    let mut range = Range::new(t);
    range.set_start(t, 1);
    range.set_end(t, 3);
    range.delete_contents(&mut dom);

    let tc = dom.world().get::<&TextContent>(t).unwrap();
    assert_eq!(tc.0, "AB");
}

#[test]
fn range_select_node_contents_utf16() {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t = dom.create_text("A\u{1F600}B");
    dom.append_child(root, t);

    let mut range = Range::new(t);
    range.select_node_contents(t, &dom);
    assert_eq!(range.start_offset, 0);
    // UTF-16 length: A(1) + surrogate(2) + B(1) = 4
    assert_eq!(range.end_offset, 4);
}

#[test]
fn range_insert_node() {
    let (mut dom, root, t1, _, _) = build_range_tree();
    let mut range = Range::new(t1);
    range.set_start(t1, 2);
    range.set_end(t1, 2);

    let new_elem = dom.create_element("b", Attributes::default());
    let outcome = range.insert_node(&mut dom, new_elem);
    let (parent, new_offset, _records) =
        outcome.expect("insert_node into attached text node must succeed");
    assert_eq!(parent, root);
    // root pre-call: [t1].  After split: [head, tail].  After insert
    // before tail: [head, new_elem, tail].  Spec step 10-11 newOffset
    // = tail's post-insert index (= 2) per `Range::insert_node` doc.
    assert_eq!(new_offset, 2);

    // t1 should be "He", then <b>, then "llo".
    let children: Vec<_> = dom.children_iter(root).collect();
    assert!(children.len() >= 3);
    let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
    assert_eq!(tc.0, "He");
    assert_eq!(children[1], new_elem);
}

#[test]
fn range_extract_contents_element_children() {
    // Test extracting element nodes (not just text).
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let a = dom.create_element("a", Attributes::default());
    let b = dom.create_element("b", Attributes::default());
    let c = dom.create_element("c", Attributes::default());
    dom.append_child(div, a);
    dom.append_child(div, b);
    dom.append_child(div, c);

    // Range: div children [1..2] -> should extract <b>.
    let mut range = Range::new(div);
    range.set_start(div, 1);
    range.set_end(div, 2);
    let (frag, _records) = range.extract_contents(&mut dom);

    // Fragment should contain <b>.
    let frag_children: Vec<_> = dom.children_iter(frag).collect();
    assert_eq!(frag_children.len(), 1);
    assert_eq!(frag_children[0], b);

    // Original div should have <a> and <c>.
    let div_children: Vec<_> = dom.children_iter(div).collect();
    assert_eq!(div_children.len(), 2);
    assert_eq!(div_children[0], a);
    assert_eq!(div_children[1], c);
}

#[test]
fn range_extract_contents_cross_container() {
    // Range spanning from text node t1 to text node t2 across containers.
    let (mut dom, _root, t1, _span, t2) = build_range_tree();
    // Tree: root -> [t1("Hello"), span -> [t2(" World")]]

    let mut range = Range::new(t1);
    range.set_start(t1, 3); // "Hel|lo" -> extract "lo"
    range.set_end(t2, 3); // " Wo|rld" -> extract " Wo"
    let (frag, _records) = range.extract_contents(&mut dom);

    // t1 should be "Hel".
    let tc1 = dom.world().get::<&TextContent>(t1).unwrap();
    assert_eq!(tc1.0, "Hel");

    // t2 should be "rld".
    let tc2 = dom.world().get::<&TextContent>(t2).unwrap();
    assert_eq!(tc2.0, "rld");

    // Fragment should contain extracted text nodes.
    let frag_children: Vec<_> = dom.children_iter(frag).collect();
    assert!(frag_children.len() >= 2);
}

/// Codex PR426 R1 — a Comment-container `deleteContents` (F5-broadened to all
/// CharacterData) must adjust OTHER live ranges anchored in the same Comment.
/// `EcsDom::replace_comment_data` now fires `MutationEvent::ReplaceData` (the
/// container-agnostic live-range fixup `LiveRangeBridge::after_replace_data`),
/// so a sibling range past the deleted span shifts left by the deleted count
/// per WHATWG DOM §4.10 "replace data" steps 8-11 — previously it was left at a
/// stale offset because the Comment splice fired no event.
#[test]
fn comment_delete_contents_adjusts_other_live_ranges() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let comment = dom.create_comment("hello world");
    let _ = dom.append_child(parent, comment);
    let (mut reg, bridge) = LiveRangeRegistry::new_pair();
    dom.set_mutation_dispatcher(make_range_only_test_dispatcher(bridge));

    // Observer range at (comment, 5) — past the [1,4] deletion span.
    let mut observer = Range::new(comment);
    observer.set_start(comment, 5);
    observer.set_end(comment, 5);
    let id = reg.register(observer);

    // A separate range deletes [1,4] ("ell") → "ho world".
    let mut deleter = Range::new(comment);
    deleter.set_start(comment, 1);
    deleter.set_end(comment, 4);
    deleter.delete_contents(&mut dom);

    let cd = dom
        .world()
        .get::<&elidex_ecs::CommentData>(comment)
        .expect("comment data");
    assert_eq!(cd.0, "ho world");

    reg.with_range(id, &dom, |range, _| {
        assert_eq!(range.start_container, comment);
        assert_eq!(
            range.start_offset, 2,
            "offset 5 past the 3-unit deletion shifts to 2 (not left stale at 5)"
        );
        assert_eq!(range.end_offset, 2);
    })
    .expect("observer range present");
}
