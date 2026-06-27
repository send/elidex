//! Range unit tests — split out from `range/mod.rs` per the ~1000-line
//! file convention (PR186 R2 #5).

#![allow(unused_must_use)]

use super::*;
use elidex_ecs::{Attributes, EcsDom, TextContent};

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
fn adjust_ranges_for_removal_basic() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let c0 = dom.create_element("a", Attributes::default());
    let c1 = dom.create_element("b", Attributes::default());
    let c2 = dom.create_element("c", Attributes::default());
    dom.append_child(parent, c0);
    dom.append_child(parent, c1);
    dom.append_child(parent, c2);

    let mut r = Range::new(parent);
    r.set_start(parent, 1);
    r.set_end(parent, 3);

    let mut ranges = [r];
    // Remove child at index 1 (c1).
    super::adjust_ranges_for_removal(&mut ranges, c1, parent, 1, &dom);

    // start_offset was 1 (== index), not > index, so unchanged.
    assert_eq!(ranges[0].start_offset, 1);
    // end_offset was 3 (> index 1), so decremented to 2.
    assert_eq!(ranges[0].end_offset, 2);
}

#[test]
fn adjust_ranges_for_removal_container_is_removed() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_text("hello");
    dom.append_child(parent, child);

    let mut r = Range::new(child);
    r.set_start(child, 2);
    r.set_end(child, 4);

    let mut ranges = [r];
    super::adjust_ranges_for_removal(&mut ranges, child, parent, 0, &dom);

    // Both boundaries should collapse to (parent, 0).
    assert_eq!(ranges[0].start_container, parent);
    assert_eq!(ranges[0].start_offset, 0);
    assert_eq!(ranges[0].end_container, parent);
    assert_eq!(ranges[0].end_offset, 0);
}

#[test]
fn adjust_ranges_for_removal_descendant_container_collapses() {
    // WHATWG DOM §5.5 "remove a node" steps 4-6: boundaries whose
    // container is an inclusive descendant of the removed node must
    // collapse to (parent, index), not just direct-equality containers.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let section = dom.create_element("section", Attributes::default());
    let p = dom.create_element("p", Attributes::default());
    let inner_text = dom.create_text("hello");
    dom.append_child(parent, section);
    dom.append_child(section, p);
    dom.append_child(p, inner_text);

    // Range boundaries sit on inner_text (a descendant of `section`).
    let mut r = Range::new(inner_text);
    r.set_start(inner_text, 2);
    r.set_end(inner_text, 4);

    let mut ranges = [r];
    super::adjust_ranges_for_removal(&mut ranges, section, parent, 0, &dom);

    // Both boundaries must collapse to (parent, 0).
    assert_eq!(ranges[0].start_container, parent);
    assert_eq!(ranges[0].start_offset, 0);
    assert_eq!(ranges[0].end_container, parent);
    assert_eq!(ranges[0].end_offset, 0);
}

#[test]
fn adjust_ranges_for_insertion_increments_strict_greater() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let c0 = dom.create_element("a", Attributes::default());
    let c1 = dom.create_element("b", Attributes::default());
    dom.append_child(parent, c0);
    dom.append_child(parent, c1);

    // Boundaries at offset 1 and 2 in `parent`. Inserting at index 1
    // shifts only the offset > 1, leaving offset == 1 in place.
    let mut r = Range::new(parent);
    r.set_start(parent, 1);
    r.set_end(parent, 2);
    let mut ranges = [r];
    super::adjust_ranges_for_insertion(&mut ranges, parent, 1);

    assert_eq!(ranges[0].start_offset, 1);
    assert_eq!(ranges[0].end_offset, 3);
}

#[test]
fn adjust_ranges_for_insertion_leaves_other_containers_alone() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let other = dom.create_element("section", Attributes::default());
    let mut r = Range::new(parent);
    r.set_start(other, 0);
    r.set_end(other, 5);

    let mut ranges = [r];
    super::adjust_ranges_for_insertion(&mut ranges, parent, 0);

    assert_eq!(ranges[0].start_container, other);
    assert_eq!(ranges[0].start_offset, 0);
    assert_eq!(ranges[0].end_container, other);
    assert_eq!(ranges[0].end_offset, 5);
}

#[test]
fn adjust_ranges_for_replace_data_collapse_inside_splice() {
    // WHATWG §4.10 "replace data" step 8: boundary `off ∈ [offset,
    // offset+count]` collapses to `offset`. Test: text="hello", replace
    // (offset=1, count=3) with "XYZ"; boundary at off=2 (inside region)
    // → collapses to 1.
    let mut dom = EcsDom::new();
    let t = dom.create_text("hello");

    let mut r = Range::new(t);
    r.set_start(t, 2);
    r.set_end(t, 3);
    let mut ranges = [r];
    super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 3);

    // Both boundaries fall inside [1, 4] → collapse to 1.
    assert_eq!(ranges[0].start_offset, 1);
    assert_eq!(ranges[0].end_offset, 1);
}

#[test]
fn adjust_ranges_for_replace_data_shift_past_splice() {
    // WHATWG §4.10 step 9: boundary `off > offset+count` shifts by
    // `new_data_len - count`. Replace (offset=1, count=3, new_data=3)
    // → boundary at off=5 stays at 5 (delta=0).
    // Replace (offset=1, count=3, new_data=5) → boundary at off=5
    // shifts to 7 (delta=+2).
    let mut dom = EcsDom::new();
    let t = dom.create_text("aaaaa");

    let mut r = Range::new(t);
    r.set_start(t, 5);
    r.set_end(t, 5);
    let mut ranges = [r.clone()];
    super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 3);
    assert_eq!(ranges[0].start_offset, 5);
    assert_eq!(ranges[0].end_offset, 5);

    let mut ranges = [r.clone()];
    super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 5);
    assert_eq!(ranges[0].start_offset, 7);
    assert_eq!(ranges[0].end_offset, 7);

    // Net-deletion: replace (offset=1, count=3, new_data=0) →
    // boundary at off=5 shifts to 2 (delta=-3).
    let mut ranges = [r];
    super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 0);
    assert_eq!(ranges[0].start_offset, 2);
    assert_eq!(ranges[0].end_offset, 2);
}

#[test]
fn adjust_ranges_for_replace_data_before_splice_unchanged() {
    let mut dom = EcsDom::new();
    let t = dom.create_text("hello");

    let mut r = Range::new(t);
    r.set_start(t, 0);
    r.set_end(t, 0);
    let mut ranges = [r];
    super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 3);

    assert_eq!(ranges[0].start_offset, 0);
    assert_eq!(ranges[0].end_offset, 0);
}

#[test]
fn adjust_ranges_for_replace_data_other_container_unchanged() {
    let mut dom = EcsDom::new();
    let t = dom.create_text("hello");
    let other = dom.create_text("world");

    let mut r = Range::new(other);
    r.set_start(other, 3);
    r.set_end(other, 3);
    let mut ranges = [r];
    super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 5);

    assert_eq!(ranges[0].start_container, other);
    assert_eq!(ranges[0].start_offset, 3);
}

#[test]
fn adjust_ranges_for_split_text_migrates_past_offset() {
    // WHATWG §4.10 "split text" step 8: boundary on node at off >
    // offset migrates to (new_node, off - offset).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let node = dom.create_text("hello world");
    let new_node = dom.create_text("");
    dom.append_child(parent, node);
    dom.append_child(parent, new_node);

    let mut r = Range::new(node);
    r.set_start(node, 3); // "hel|lo world"
    r.set_end(node, 8); // "hello wo|rld"
    let mut ranges = [r];
    // split at offset 5 ("hello" | " world").
    super::adjust_ranges_for_split_text(&mut ranges, node, new_node, 5, Some(parent), Some(0));

    // start_offset 3 ≤ 5 → stays on `node`.
    assert_eq!(ranges[0].start_container, node);
    assert_eq!(ranges[0].start_offset, 3);
    // end_offset 8 > 5 → migrates to (new_node, 3).
    assert_eq!(ranges[0].end_container, new_node);
    assert_eq!(ranges[0].end_offset, 3);
}

#[test]
fn adjust_ranges_for_split_text_parent_boundary_increments() {
    // splitText adds one child between node and node_idx+1; parent
    // boundaries at idx > node_idx → +1.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let n0 = dom.create_text("hello");
    let n1 = dom.create_text("world");
    dom.append_child(parent, n0);
    dom.append_child(parent, n1);
    let new_node = dom.create_text("tail");
    dom.append_child(parent, new_node);

    // Splitting n0 (at index 0) inserts new_node at index 1; boundary
    // on parent at offset > 0 increments.
    let mut r = Range::new(parent);
    r.set_start(parent, 1);
    r.set_end(parent, 2);
    let mut ranges = [r];
    super::adjust_ranges_for_split_text(&mut ranges, n0, new_node, 3, Some(parent), Some(0));

    assert_eq!(ranges[0].start_offset, 2);
    assert_eq!(ranges[0].end_offset, 3);
}

#[test]
fn adjust_ranges_for_split_text_orphan_node_skips_parent() {
    // No parent → only the node-side migration runs.
    let mut dom = EcsDom::new();
    let node = dom.create_text("hello");
    let new_node = dom.create_text("");

    let mut r = Range::new(node);
    r.set_start(node, 4);
    r.set_end(node, 4);
    let mut ranges = [r];
    super::adjust_ranges_for_split_text(&mut ranges, node, new_node, 2, None, None);

    assert_eq!(ranges[0].start_container, new_node);
    assert_eq!(ranges[0].start_offset, 2);
}

#[test]
fn adjust_ranges_for_normalize_merge_migrates_merged_child() {
    // WHATWG §4.5 step 6.4: boundary on merged_child at off migrates to
    // (prev, prev_old_len + off). prev_old_len = 5 ("hello").
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let prev = dom.create_text("helloworld"); // post-merge state
    let merged = dom.create_text(""); // post-merge empty, pre-detach
    dom.append_child(parent, prev);
    dom.append_child(parent, merged);

    let mut r = Range::new(merged);
    r.set_start(merged, 2);
    r.set_end(merged, 4);
    let mut ranges = [r];
    super::adjust_ranges_for_normalize_merge(&mut ranges, merged, prev, 5, Some(parent), Some(1));

    // Boundary migrated to (prev, 5 + off).
    assert_eq!(ranges[0].start_container, prev);
    assert_eq!(ranges[0].start_offset, 7);
    assert_eq!(ranges[0].end_container, prev);
    assert_eq!(ranges[0].end_offset, 9);
}

#[test]
fn adjust_ranges_for_normalize_merge_parent_boundary_at_merged_idx() {
    // Parent boundary AT merged_child's index migrates to (prev,
    // prev_old_len) — the merge splice point.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let prev = dom.create_text("helloworld");
    let merged = dom.create_text("");
    dom.append_child(parent, prev);
    dom.append_child(parent, merged);

    let mut r = Range::new(parent);
    r.set_start(parent, 1);
    r.set_end(parent, 1);
    let mut ranges = [r];
    super::adjust_ranges_for_normalize_merge(&mut ranges, merged, prev, 5, Some(parent), Some(1));

    assert_eq!(ranges[0].start_container, prev);
    assert_eq!(ranges[0].start_offset, 5);
    assert_eq!(ranges[0].end_container, prev);
    assert_eq!(ranges[0].end_offset, 5);
}

#[test]
fn adjust_ranges_for_normalize_merge_parent_boundary_past_merged_idx() {
    // Parent boundary AT idx > merged_child_index → decrement (parent
    // loses one child).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let prev = dom.create_text("helloworld");
    let merged = dom.create_text("");
    let trailing = dom.create_element("span", Attributes::default());
    dom.append_child(parent, prev);
    dom.append_child(parent, merged);
    dom.append_child(parent, trailing);

    let mut r = Range::new(parent);
    r.set_start(parent, 3);
    r.set_end(parent, 3);
    let mut ranges = [r];
    super::adjust_ranges_for_normalize_merge(&mut ranges, merged, prev, 5, Some(parent), Some(1));

    assert_eq!(ranges[0].start_offset, 2);
    assert_eq!(ranges[0].end_offset, 2);
}

#[test]
fn adjust_ranges_for_text_change() {
    let mut dom = EcsDom::new();
    let root = dom.create_element("div", Attributes::default());
    let t = dom.create_text("hello");
    dom.append_child(root, t);

    let mut r = Range::new(t);
    r.set_start(t, 2);
    r.set_end(t, 5);

    let mut ranges = [r];
    // Shorten text to 3 UTF-16 units.
    super::adjust_ranges_for_text_change(&mut ranges, t, 3);

    assert_eq!(ranges[0].start_offset, 2); // still valid
    assert_eq!(ranges[0].end_offset, 3); // clamped from 5 to 3
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
