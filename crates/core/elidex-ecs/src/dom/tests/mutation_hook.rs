//! Tests for the `MutationDispatcher` trait + `EcsDom` fire sites.
//!
//! Verifies that every mutation primitive
//! (`append_child` / `insert_before` / `remove_child` / `replace_child` /
//! `destroy_entity` / `set_text_data` / `replace_text_data` /
//! `set_attribute` / `remove_attribute` / `fire_split_text` /
//! `fire_normalize_merge`) fires the correct [`MutationEvent`] variant.
//! Uses a mock `MutationDispatcher` impl that records each event into a
//! `Vec<MockEvent>` (variant-equivalent shape).

use std::sync::{Arc, Mutex};

use super::*;
use crate::dom::{MutationDispatcher, MutationEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
enum MockEvent {
    Insert {
        node: Entity,
        parent: Entity,
        index: usize,
    },
    Remove {
        node: Entity,
        parent: Entity,
        index: usize,
    },
    TextChange {
        node: Entity,
        new_utf16_len: usize,
    },
    ReplaceData {
        node: Entity,
        offset: usize,
        count: usize,
        new_data_len: usize,
    },
    SplitText {
        node: Entity,
        new_node: Entity,
        offset: usize,
        parent: Option<Entity>,
        node_index: Option<usize>,
    },
    NormalizeMerge {
        merged_child: Entity,
        prev: Entity,
        prev_old_len: usize,
        parent: Option<Entity>,
        merged_child_index: Option<usize>,
    },
}

#[derive(Default, Clone)]
struct MockHook {
    events: Arc<Mutex<Vec<MockEvent>>>,
}

impl MutationDispatcher for MockHook {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &crate::EcsDom) {
        // Pattern-match every variant, recording into the same
        // MockEvent shape the legacy tests assert against.  The
        // generic AttributeChange variant is ignored — dedicated
        // attribute tests use a separate fixture.  The Remove arm
        // records (node, parent, removed_index) only — the
        // descendants snapshot is asserted by `DescendantSnapshotHook`
        // in this same file.
        match *event {
            MutationEvent::Insert { node, parent, index } => {
                self.events.lock().unwrap().push(MockEvent::Insert {
                    node,
                    parent,
                    index,
                });
            }
            MutationEvent::Remove {
                node,
                parent,
                removed_index,
                ..
            } => {
                self.events.lock().unwrap().push(MockEvent::Remove {
                    node,
                    parent,
                    index: removed_index,
                });
            }
            MutationEvent::TextChange { node, new_utf16_len } => {
                self.events.lock().unwrap().push(MockEvent::TextChange {
                    node,
                    new_utf16_len,
                });
            }
            MutationEvent::ReplaceData {
                node,
                offset_utf16,
                count_utf16,
                new_data_len_utf16,
            } => {
                self.events.lock().unwrap().push(MockEvent::ReplaceData {
                    node,
                    offset: offset_utf16,
                    count: count_utf16,
                    new_data_len: new_data_len_utf16,
                });
            }
            MutationEvent::SplitText {
                node,
                new_node,
                offset_utf16,
                parent,
                node_index,
            } => {
                self.events.lock().unwrap().push(MockEvent::SplitText {
                    node,
                    new_node,
                    offset: offset_utf16,
                    parent,
                    node_index,
                });
            }
            MutationEvent::NormalizeMerge {
                merged_child,
                prev,
                prev_old_len_utf16,
                parent,
                merged_child_index,
            } => {
                self.events.lock().unwrap().push(MockEvent::NormalizeMerge {
                    merged_child,
                    prev,
                    prev_old_len: prev_old_len_utf16,
                    parent,
                    merged_child_index,
                });
            }
            MutationEvent::AttributeChange { .. } => {
                // Generic MockHook does not record attribute events.
            }
        }
    }
}

fn install_mock(dom: &mut EcsDom) -> Arc<Mutex<Vec<MockEvent>>> {
    let hook = MockHook::default();
    let events = hook.events.clone();
    dom.set_mutation_dispatcher(Box::new(hook));
    events
}

#[test]
fn append_child_fires_after_insert_with_post_link_index() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");
    assert!(dom.append_child(parent, c0));

    // Install hook AFTER c0 is already a child so the first event we record
    // is from appending c1 (index 1).
    let events = install_mock(&mut dom);
    assert!(dom.append_child(parent, c1));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::Insert {
            node: c1,
            parent,
            index: 1
        }]
    );
}

#[test]
fn append_child_with_no_hook_is_silent() {
    // Sanity: append still works without a hook installed.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "a");
    assert!(dom.append_child(parent, child));
}

#[test]
fn insert_before_fires_with_ref_child_index() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");
    let new_child = elem(&mut dom, "c");
    assert!(dom.append_child(parent, c0));
    assert!(dom.append_child(parent, c1));

    let events = install_mock(&mut dom);
    assert!(dom.insert_before(parent, new_child, c1));

    // new_child now occupies the index c1 used to occupy (1).
    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::Insert {
            node: new_child,
            parent,
            index: 1
        }]
    );
}

#[test]
fn remove_child_fires_with_pre_removal_index() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");
    let c2 = elem(&mut dom, "c");
    assert!(dom.append_child(parent, c0));
    assert!(dom.append_child(parent, c1));
    assert!(dom.append_child(parent, c2));

    let events = install_mock(&mut dom);
    assert!(dom.remove_child(parent, c1));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::Remove {
            node: c1,
            parent,
            index: 1
        }]
    );
}

#[test]
fn replace_child_fires_remove_then_insert_at_same_index() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");
    let c2 = elem(&mut dom, "c");
    let new_child = elem(&mut dom, "d");
    assert!(dom.append_child(parent, c0));
    assert!(dom.append_child(parent, c1));
    assert!(dom.append_child(parent, c2));

    let events = install_mock(&mut dom);
    assert!(dom.replace_child(parent, new_child, c1));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            MockEvent::Remove {
                node: c1,
                parent,
                index: 1
            },
            MockEvent::Insert {
                node: new_child,
                parent,
                index: 1
            },
        ]
    );
}

#[test]
fn replace_child_with_earlier_sibling_reports_post_shift_index() {
    // When new_child is an earlier sibling of old_child in the same parent,
    // detach(new_child) shifts old_child down by one. The hook MUST report
    // the post-shift index for the old_child removal (WHATWG DOM §5.5
    // "remove a node" step 4). The implicit detach of new_child also fires
    // its own after_remove at new_child's pre-detach index.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let new_child = elem(&mut dom, "n");
    let a = elem(&mut dom, "a");
    let old_child = elem(&mut dom, "o");
    let b = elem(&mut dom, "b");
    assert!(dom.append_child(parent, new_child));
    assert!(dom.append_child(parent, a));
    assert!(dom.append_child(parent, old_child));
    assert!(dom.append_child(parent, b));

    let events = install_mock(&mut dom);
    assert!(dom.replace_child(parent, new_child, old_child));

    // detach(new_child) fires Remove(new_child, parent, 0).
    // After detach: [a, old, b] → old at index 1.
    // Replace removes old at index 1, inserts new at index 1.
    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            MockEvent::Remove {
                node: new_child,
                parent,
                index: 0
            },
            MockEvent::Remove {
                node: old_child,
                parent,
                index: 1
            },
            MockEvent::Insert {
                node: new_child,
                parent,
                index: 1
            },
        ]
    );
}

#[test]
fn destroy_entity_fires_once_for_entity_only() {
    // Lazy-collapse contract: orphaned descendants do NOT receive
    // individual after_remove calls.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let target = elem(&mut dom, "section");
    let grandchild_a = elem(&mut dom, "p");
    let grandchild_b = elem(&mut dom, "span");
    assert!(dom.append_child(parent, target));
    assert!(dom.append_child(target, grandchild_a));
    assert!(dom.append_child(target, grandchild_b));

    let events = install_mock(&mut dom);
    assert!(dom.destroy_entity(target));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::Remove {
            node: target,
            parent,
            index: 0
        }]
    );
}

#[test]
fn destroy_entity_with_no_parent_does_not_fire() {
    let mut dom = EcsDom::new();
    let orphan = elem(&mut dom, "div");

    let events = install_mock(&mut dom);
    assert!(dom.destroy_entity(orphan));

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn set_text_data_fires_after_text_change() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hello");
    assert!(dom.append_child(parent, text));

    let events = install_mock(&mut dom);
    assert_eq!(dom.set_text_data(text, "Hi"), Some(2));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::TextChange {
            node: text,
            new_utf16_len: 2
        }]
    );
}

#[test]
fn set_text_data_on_non_text_entity_returns_none_and_does_not_fire() {
    let mut dom = EcsDom::new();
    let element = elem(&mut dom, "div");

    let events = install_mock(&mut dom);
    assert_eq!(dom.set_text_data(element, "ignored"), None);

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn set_text_data_utf16_length_counts_surrogate_pair_as_two() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("a");
    assert!(dom.append_child(parent, text));

    let events = install_mock(&mut dom);
    // "A<emoji>B" = 3 chars but 4 UTF-16 code units (surrogate pair = 2).
    let len = dom.set_text_data(text, "A\u{1F600}B");
    assert_eq!(len, Some(4));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::TextChange {
            node: text,
            new_utf16_len: 4
        }]
    );
}

#[test]
fn set_text_data_empty_string() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hello");
    assert!(dom.append_child(parent, text));

    let events = install_mock(&mut dom);
    assert_eq!(dom.set_text_data(text, ""), Some(0));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::TextChange {
            node: text,
            new_utf16_len: 0
        }]
    );
}

#[test]
fn take_mutation_dispatcher_round_trip() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");

    let events = install_mock(&mut dom);

    // First mutation fires while hook is installed.
    assert!(dom.append_child(parent, c0));
    assert_eq!(events.lock().unwrap().len(), 1);

    // Take the hook out: subsequent mutations do NOT fire.
    let taken = dom.take_mutation_dispatcher();
    assert!(taken.is_some());
    assert!(dom.append_child(parent, c1));
    assert_eq!(events.lock().unwrap().len(), 1);

    // Re-install: mutations fire again.
    let c2 = elem(&mut dom, "c");
    dom.set_mutation_dispatcher(taken.expect("hook was taken"));
    assert!(dom.append_child(parent, c2));
    assert_eq!(events.lock().unwrap().len(), 2);
}

#[test]
fn clear_mutation_dispatcher_drops_hook() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "a");

    let events = install_mock(&mut dom);
    dom.clear_mutation_dispatcher();

    assert!(dom.append_child(parent, child));
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn set_mutation_dispatcher_returns_previous_hook() {
    let mut dom = EcsDom::new();
    let _ = install_mock(&mut dom);
    let prev = dom.set_mutation_dispatcher(Box::new(MockHook::default()));
    assert!(prev.is_some());
    let none = dom.take_mutation_dispatcher();
    assert!(none.is_some());
    let none2 = dom.take_mutation_dispatcher();
    assert!(none2.is_none());
}

#[test]
fn index_in_parent_walks_prev_sibling_chain() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");
    let c2 = elem(&mut dom, "c");
    assert!(dom.append_child(parent, c0));
    assert!(dom.append_child(parent, c1));
    assert!(dom.append_child(parent, c2));

    assert_eq!(dom.index_in_parent(c0), Some(0));
    assert_eq!(dom.index_in_parent(c1), Some(1));
    assert_eq!(dom.index_in_parent(c2), Some(2));
}

#[test]
fn set_text_data_bumps_inclusive_descendants_version() {
    // `set_text_data` is the canonical Text/CData write path and MUST
    // bump `inclusive_descendants_version` itself so callers (and
    // downstream caches: live collections / layout / render) see the
    // mutation without needing a redundant `rev_version` call. Use a
    // document-rooted tree so `rev_version`'s global-counter path is
    // exercised and ancestor versions propagate as live collections
    // expect.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hello");
    assert!(dom.append_child(doc, parent));
    assert!(dom.append_child(parent, text));

    let before_doc = dom.inclusive_descendants_version(doc);
    let before_text = dom.inclusive_descendants_version(text);

    dom.set_text_data(text, "world");

    assert!(
        dom.inclusive_descendants_version(text) > before_text,
        "text node version did not advance after set_text_data"
    );
    assert!(
        dom.inclusive_descendants_version(doc) > before_doc,
        "doc-root version did not advance — live collections rooted at \
         document would miss the text mutation"
    );
}

#[test]
fn cross_parent_move_bumps_old_parent_version() {
    // When a node is implicitly detached from its old parent during an
    // append/insert/replace to a new parent, the old parent's child
    // list changed — its version must advance so cached queries rooted
    // at the old subtree see the mutation.
    let mut dom = EcsDom::new();
    let old_parent = elem(&mut dom, "section");
    let new_parent = elem(&mut dom, "article");
    let child = elem(&mut dom, "p");
    assert!(dom.append_child(old_parent, child));

    let before_old = dom.inclusive_descendants_version(old_parent);
    let before_new = dom.inclusive_descendants_version(new_parent);

    assert!(dom.append_child(new_parent, child));

    assert!(
        dom.inclusive_descendants_version(old_parent) > before_old,
        "old parent version did not advance on cross-parent move"
    );
    assert!(
        dom.inclusive_descendants_version(new_parent) > before_new,
        "new parent version did not advance"
    );
}

#[test]
fn index_in_parent_returns_none_for_orphan() {
    let mut dom = EcsDom::new();
    let orphan = elem(&mut dom, "div");
    assert_eq!(dom.index_in_parent(orphan), None);
}

#[test]
fn append_child_fires_after_remove_for_old_parent_on_move() {
    // Moving a node between parents must fire `after_remove` on the
    // implicit detach from the old parent (per WHATWG insert algorithm
    // step "If node has a parent, then remove node") so Range live-
    // tracking sees the §5.5 step 4-6 adjustment.
    let mut dom = EcsDom::new();
    let old_parent = elem(&mut dom, "section");
    let new_parent = elem(&mut dom, "article");
    let child = elem(&mut dom, "p");
    assert!(dom.append_child(old_parent, child));

    let events = install_mock(&mut dom);
    assert!(dom.append_child(new_parent, child));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            MockEvent::Remove {
                node: child,
                parent: old_parent,
                index: 0
            },
            MockEvent::Insert {
                node: child,
                parent: new_parent,
                index: 0
            },
        ]
    );
}

#[test]
fn insert_before_fires_after_remove_for_old_parent_on_move() {
    let mut dom = EcsDom::new();
    let old_parent = elem(&mut dom, "section");
    let new_parent = elem(&mut dom, "article");
    let anchor = elem(&mut dom, "a");
    let child = elem(&mut dom, "p");
    assert!(dom.append_child(old_parent, child));
    assert!(dom.append_child(new_parent, anchor));

    let events = install_mock(&mut dom);
    assert!(dom.insert_before(new_parent, child, anchor));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            MockEvent::Remove {
                node: child,
                parent: old_parent,
                index: 0
            },
            MockEvent::Insert {
                node: child,
                parent: new_parent,
                index: 0
            },
        ]
    );
}

#[test]
fn replace_child_fires_after_remove_for_new_child_old_parent() {
    // WHATWG "replace a child" step 4: "If node's parent is not null,
    // remove node". This implicit removal must fire `after_remove` on
    // the new_child's old parent, in addition to the
    // remove(old_child) + insert(new_child) pair on `parent`.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let other_parent = elem(&mut dom, "section");
    let new_child = elem(&mut dom, "n");
    let old_child = elem(&mut dom, "o");
    assert!(dom.append_child(other_parent, new_child));
    assert!(dom.append_child(parent, old_child));

    let events = install_mock(&mut dom);
    assert!(dom.replace_child(parent, new_child, old_child));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            MockEvent::Remove {
                node: new_child,
                parent: other_parent,
                index: 0
            },
            MockEvent::Remove {
                node: old_child,
                parent,
                index: 0
            },
            MockEvent::Insert {
                node: new_child,
                parent,
                index: 0
            },
        ]
    );
}

#[test]
fn attach_shadow_does_not_fire_hook_events() {
    // Light-tree-only contract: `attach_shadow` plumbs the shadow root
    // into the host's child list (via internal `append_child`), but per
    // WHATWG §5.5 light-tree consumers (e.g. `LiveRangeRegistry`) must
    // NOT see `after_insert(shadow_root, host, ...)`. Asserting that the
    // event log stays empty here pins the suppression contract in tree.rs.
    use crate::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");

    let events = install_mock(&mut dom);
    let _shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div>");

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn append_child_into_shadow_root_does_not_fire_hook_events() {
    // Light-tree-only contract: a mutation whose **parent** is a shadow
    // root is a shadow-tree mutation and must not surface to light-tree
    // consumers like Range live-tracking.
    use crate::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div>");
    let shadow_child = elem(&mut dom, "span");

    let events = install_mock(&mut dom);
    assert!(dom.append_child(shadow, shadow_child));

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn remove_child_from_shadow_root_does_not_fire_hook_events() {
    use crate::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div>");
    let shadow_child = elem(&mut dom, "span");
    assert!(dom.append_child(shadow, shadow_child));

    let events = install_mock(&mut dom);
    assert!(dom.remove_child(shadow, shadow_child));

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn destroy_shadow_root_does_not_fire_hook_events() {
    use crate::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div>");

    let events = install_mock(&mut dom);
    assert!(dom.destroy_entity(shadow));

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn index_in_parent_skips_shadow_root_siblings() {
    // The host's shadow root is a `prev_sibling`-reachable entity but
    // NOT exposed in `children_iter` / `children`. Counting it would
    // make MutationHook indices diverge from the light-tree indices
    // Range live-tracking depends on.
    use crate::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let _shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div>");
    let light_child = elem(&mut dom, "p");
    assert!(dom.append_child(host, light_child));

    // Despite the shadow-root sitting in the prev_sibling chain,
    // the light-DOM `p` is at exposed index 0.
    assert_eq!(dom.index_in_parent(light_child), Some(0));
}

#[test]
fn append_child_after_attach_shadow_reports_light_tree_index() {
    // Regression: appending a light-DOM child to a shadow host must
    // fire `after_insert` with the light-tree index, not the raw
    // prev_sibling count that would include the shadow root.
    use crate::ShadowRootMode;
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let _shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div>");
    let light_child = elem(&mut dom, "p");

    let events = install_mock(&mut dom);
    assert!(dom.append_child(host, light_child));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::Insert {
            node: light_child,
            parent: host,
            index: 0
        }]
    );
}

#[test]
fn replace_text_data_fires_after_replace_data_with_offset_count_replacement_len() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hello");
    assert!(dom.append_child(parent, text));

    let events = install_mock(&mut dom);
    // Replace "ell" (offset=1, count=3) with "XYZ" (replacement_len=3) →
    // "hXYZo" (new total length 5).
    assert_eq!(dom.replace_text_data(text, 1, 3, "XYZ"), Some(5));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::ReplaceData {
            node: text,
            offset: 1,
            count: 3,
            new_data_len: 3,
        }]
    );
    let tc = dom.world().get::<&TextContent>(text).expect("TextContent");
    assert_eq!(tc.0, "hXYZo");
}

#[test]
fn replace_text_data_clamps_count_silently() {
    // WHATWG §11.2 step 6: "if offset + count is greater than length,
    // end at length". `replace_text_data` is the engine primitive, so
    // it applies the clamp internally; bounds-validation (IndexSizeError
    // for offset > len) is the caller's responsibility.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("abcde");
    assert!(dom.append_child(parent, text));

    let events = install_mock(&mut dom);
    // count=99 past end → clamped to 4 ("bcde"). "abcde" with
    // offset=1 + delete "bcde" + insert "Z" → "aZ" (length 2). Hook
    // reports the CLAMPED count (4), per WHATWG §11.2 step 6 — Range
    // live-tracking math depends on the actual spliced span, not the
    // caller's possibly-overflowing argument (PR186 R3 #1).
    assert_eq!(dom.replace_text_data(text, 1, 99, "Z"), Some(2));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::ReplaceData {
            node: text,
            offset: 1,
            count: 4,
            new_data_len: 1,
        }]
    );
    let tc = dom.world().get::<&TextContent>(text).expect("TextContent");
    assert_eq!(tc.0, "aZ");
}

#[test]
fn replace_text_data_on_non_text_entity_returns_none_and_does_not_fire() {
    let mut dom = EcsDom::new();
    let element = elem(&mut dom, "div");

    let events = install_mock(&mut dom);
    assert_eq!(dom.replace_text_data(element, 0, 0, "x"), None);

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn replace_text_data_bumps_inclusive_descendants_version() {
    // Mirrors `set_text_data_bumps_inclusive_descendants_version`: the
    // splice primitive must self-bump rev_version so callers don't
    // need a redundant call.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hello");
    assert!(dom.append_child(doc, parent));
    assert!(dom.append_child(parent, text));

    let before_doc = dom.inclusive_descendants_version(doc);
    let before_text = dom.inclusive_descendants_version(text);

    dom.replace_text_data(text, 1, 2, "X");

    assert!(
        dom.inclusive_descendants_version(text) > before_text,
        "text node version did not advance after replace_text_data"
    );
    assert!(
        dom.inclusive_descendants_version(doc) > before_doc,
        "doc-root version did not advance — live collections rooted at \
         document would miss the splice"
    );
}

#[test]
fn fire_split_text_helper_routes_to_hook() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hello world");
    let new_text = dom.create_text("");
    assert!(dom.append_child(parent, text));
    assert!(dom.append_child(parent, new_text));

    let events = install_mock(&mut dom);
    dom.fire_split_text(text, new_text, 5, Some(parent), Some(0));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::SplitText {
            node: text,
            new_node: new_text,
            offset: 5,
            parent: Some(parent),
            node_index: Some(0),
        }]
    );
}

#[test]
fn fire_normalize_merge_helper_routes_to_hook() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let prev = dom.create_text("hello");
    let merged_child = dom.create_text("world");
    assert!(dom.append_child(parent, prev));
    assert!(dom.append_child(parent, merged_child));

    let events = install_mock(&mut dom);
    // prev_old_len = 5 (len of "hello" before absorbing "world").
    dom.fire_normalize_merge(merged_child, prev, 5, Some(parent), Some(1));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::NormalizeMerge {
            merged_child,
            prev,
            prev_old_len: 5,
            parent: Some(parent),
            merged_child_index: Some(1),
        }]
    );
}

#[test]
fn fire_helpers_silent_when_no_hook_installed() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("hi");
    let new_text = dom.create_text("");
    assert!(dom.append_child(parent, text));
    assert!(dom.append_child(parent, new_text));

    // No hook installed — helpers must be no-ops, not panic.
    dom.fire_split_text(text, new_text, 1, Some(parent), Some(0));
    dom.fire_normalize_merge(new_text, text, 2, Some(parent), Some(1));
}

#[test]
fn append_child_orphan_does_not_fire_after_remove() {
    // Sanity: an orphan child (no old parent) generates only the
    // after_insert callback; there is no implicit removal to report.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let orphan = elem(&mut dom, "a");

    let events = install_mock(&mut dom);
    assert!(dom.append_child(parent, orphan));

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![MockEvent::Insert {
            node: orphan,
            parent,
            index: 0
        }]
    );
}

// ---------------------------------------------------------------------------
// after_remove descendants snapshot (PR186 R2 #3 regression)
// ---------------------------------------------------------------------------

type DescendantSnapshotLog = Vec<(Entity, Vec<Entity>)>;

#[derive(Default, Clone)]
struct DescendantSnapshotHook {
    snapshot: Arc<Mutex<DescendantSnapshotLog>>,
}

impl MutationDispatcher for DescendantSnapshotHook {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &crate::EcsDom) {
        if let MutationEvent::Remove {
            node, descendants, ..
        } = *event
        {
            self.snapshot
                .lock()
                .unwrap()
                .push((node, descendants.to_vec()));
        }
    }
}

#[test]
fn destroy_entity_passes_inclusive_descendants_snapshot() {
    // PR186 R2 #3: `destroy_entity` MUST snapshot the light-tree
    // inclusive-descendant set BEFORE orphaning children, so a Range
    // boundary consumer can collapse boundaries on still-live
    // descendants without depending on intact parent links.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let target = elem(&mut dom, "section");
    let child_a = elem(&mut dom, "p");
    let grandchild = dom.create_text("inner");
    let child_b = elem(&mut dom, "span");
    assert!(dom.append_child(parent, target));
    assert!(dom.append_child(target, child_a));
    assert!(dom.append_child(child_a, grandchild));
    assert!(dom.append_child(target, child_b));

    let hook = DescendantSnapshotHook::default();
    let snapshot_handle = hook.snapshot.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    assert!(dom.destroy_entity(target));

    let log = snapshot_handle.lock().unwrap().clone();
    assert_eq!(log.len(), 1, "exactly one after_remove fires for target");
    let (recorded_node, descendants) = &log[0];
    assert_eq!(*recorded_node, target);
    // Snapshot must include target + every light-tree descendant.
    // Order within the snapshot is implementation-defined (DFS via
    // children_iter / explicit stack), but the SET is fixed.
    let mut sorted = descendants.clone();
    sorted.sort();
    let mut expected = vec![target, child_a, grandchild, child_b];
    expected.sort();
    assert_eq!(
        sorted, expected,
        "snapshot includes target + child_a + grandchild + child_b"
    );
}

#[test]
fn remove_child_passes_inclusive_descendants_snapshot() {
    // Sanity: plain `remove_child` also passes the snapshot, in case
    // a hook consumer relies on it uniformly across remove paths.
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let target = elem(&mut dom, "section");
    let child = elem(&mut dom, "p");
    assert!(dom.append_child(parent, target));
    assert!(dom.append_child(target, child));

    let hook = DescendantSnapshotHook::default();
    let snapshot_handle = hook.snapshot.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    assert!(dom.remove_child(parent, target));

    let log = snapshot_handle.lock().unwrap().clone();
    assert_eq!(log.len(), 1);
    let (recorded_node, descendants) = &log[0];
    assert_eq!(*recorded_node, target);
    let mut sorted = descendants.clone();
    sorted.sort();
    let mut expected = vec![target, child];
    expected.sort();
    assert_eq!(sorted, expected);
}
