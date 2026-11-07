//! Live-range update (`adjust_ranges_*`) unit tests — split out from
//! `range/tests.rs` per the ~1000-line file convention.

#![allow(unused_must_use)]

use super::*;
use elidex_ecs::{Attributes, EcsDom};

#[test]
fn adjust_ranges_for_removal_basic() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let c0 = dom.create_element_ownerless_fixture("a", Attributes::default());
    let c1 = dom.create_element_ownerless_fixture("b", Attributes::default());
    let c2 = dom.create_element_ownerless_fixture("c", Attributes::default());
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let child = dom.create_text_ownerless_fixture("hello");
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let section = dom.create_element_ownerless_fixture("section", Attributes::default());
    let p = dom.create_element_ownerless_fixture("p", Attributes::default());
    let inner_text = dom.create_text_ownerless_fixture("hello");
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let c0 = dom.create_element_ownerless_fixture("a", Attributes::default());
    let c1 = dom.create_element_ownerless_fixture("b", Attributes::default());
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let other = dom.create_element_ownerless_fixture("section", Attributes::default());
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
    let t = dom.create_text_ownerless_fixture("hello");

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
    let t = dom.create_text_ownerless_fixture("aaaaa");

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
    let t = dom.create_text_ownerless_fixture("hello");

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
    let t = dom.create_text_ownerless_fixture("hello");
    let other = dom.create_text_ownerless_fixture("world");

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
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let node = dom.create_text_ownerless_fixture("hello world");
    let new_node = dom.create_text_ownerless_fixture("");
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
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let n0 = dom.create_text_ownerless_fixture("hello");
    let n1 = dom.create_text_ownerless_fixture("world");
    dom.append_child(parent, n0);
    dom.append_child(parent, n1);
    let new_node = dom.create_text_ownerless_fixture("tail");
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
    let node = dom.create_text_ownerless_fixture("hello");
    let new_node = dom.create_text_ownerless_fixture("");

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
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let prev = dom.create_text_ownerless_fixture("helloworld"); // post-merge state
    let merged = dom.create_text_ownerless_fixture(""); // post-merge empty, pre-detach
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
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let prev = dom.create_text_ownerless_fixture("helloworld");
    let merged = dom.create_text_ownerless_fixture("");
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
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let prev = dom.create_text_ownerless_fixture("helloworld");
    let merged = dom.create_text_ownerless_fixture("");
    let trailing = dom.create_element_ownerless_fixture("span", Attributes::default());
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
    let root = dom.create_element_ownerless_fixture("div", Attributes::default());
    let t = dom.create_text_ownerless_fixture("hello");
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
