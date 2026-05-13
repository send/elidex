//! Tests for the `MutationHook` trait + `EcsDom` fire sites.
//!
//! Verifies that every mutation primitive
//! (`append_child` / `insert_before` / `remove_child` / `replace_child` /
//! `destroy_entity` / `set_text_data`) fires the correct callback with the
//! correct index / length. Uses a mock `MutationHook` impl that records
//! every callback into a `Vec<MockEvent>`.

use std::sync::{Arc, Mutex};

use super::*;
use crate::dom::MutationHook;

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
}

#[derive(Default, Clone)]
struct MockHook {
    events: Arc<Mutex<Vec<MockEvent>>>,
}

impl MutationHook for MockHook {
    fn after_remove(&mut self, node: Entity, parent: Entity, removed_index: usize) {
        self.events.lock().unwrap().push(MockEvent::Remove {
            node,
            parent,
            index: removed_index,
        });
    }
    fn after_insert(&mut self, node: Entity, parent: Entity, index: usize) {
        self.events.lock().unwrap().push(MockEvent::Insert {
            node,
            parent,
            index,
        });
    }
    fn after_text_change(&mut self, node: Entity, new_utf16_len: usize) {
        self.events.lock().unwrap().push(MockEvent::TextChange {
            node,
            new_utf16_len,
        });
    }
}

fn install_mock(dom: &mut EcsDom) -> Arc<Mutex<Vec<MockEvent>>> {
    let hook = MockHook::default();
    let events = hook.events.clone();
    dom.set_mutation_hook(Box::new(hook));
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
    assert_eq!(dom.set_text_data(text, "Hi".to_string()), Some(2));

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
    assert_eq!(dom.set_text_data(element, "ignored".to_string()), None);

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
    let len = dom.set_text_data(text, "A\u{1F600}B".to_string());
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
    assert_eq!(dom.set_text_data(text, String::new()), Some(0));

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
fn take_mutation_hook_round_trip() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c0 = elem(&mut dom, "a");
    let c1 = elem(&mut dom, "b");

    let events = install_mock(&mut dom);

    // First mutation fires while hook is installed.
    assert!(dom.append_child(parent, c0));
    assert_eq!(events.lock().unwrap().len(), 1);

    // Take the hook out: subsequent mutations do NOT fire.
    let taken = dom.take_mutation_hook();
    assert!(taken.is_some());
    assert!(dom.append_child(parent, c1));
    assert_eq!(events.lock().unwrap().len(), 1);

    // Re-install: mutations fire again.
    let c2 = elem(&mut dom, "c");
    dom.set_mutation_hook(taken.expect("hook was taken"));
    assert!(dom.append_child(parent, c2));
    assert_eq!(events.lock().unwrap().len(), 2);
}

#[test]
fn clear_mutation_hook_drops_hook() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "a");

    let events = install_mock(&mut dom);
    dom.clear_mutation_hook();

    assert!(dom.append_child(parent, child));
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn set_mutation_hook_returns_previous_hook() {
    let mut dom = EcsDom::new();
    let _ = install_mock(&mut dom);
    let prev = dom.set_mutation_hook(Box::new(MockHook::default()));
    assert!(prev.is_some());
    let none = dom.take_mutation_hook();
    assert!(none.is_some());
    let none2 = dom.take_mutation_hook();
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
