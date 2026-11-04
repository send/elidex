//! Tests that the `EcsDom::fire_*` helpers other crates call
//! (`fire_split_text` / `fire_normalize_merge` / `fire_clone_created`) route to the installed
//! `MutationDispatcher`, using the recorder fixture in `mutation_hook.rs`.

use super::mutation_hook::{install_mock, MockEvent};
use super::*;
use crate::dom::CloneScope;

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
fn fire_clone_created_helper_routes_to_hook() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");

    let events = install_mock(&mut dom);
    dom.fire_clone_created(root, CloneScope::Subtree);
    dom.fire_clone_created(root, CloneScope::SelfOnly);

    let log = events.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            MockEvent::CloneCreated {
                root,
                scope: CloneScope::Subtree,
            },
            MockEvent::CloneCreated {
                root,
                scope: CloneScope::SelfOnly,
            },
        ]
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

    let children_before: Vec<_> = dom.children_iter(parent).collect();
    let parents_before = (dom.get_parent(text), dom.get_parent(new_text));

    // No hook installed — helpers must be no-ops, not panic.
    dom.fire_split_text(text, new_text, 1, Some(parent), Some(0));
    dom.fire_normalize_merge(new_text, text, 2, Some(parent), Some(1));
    dom.fire_clone_created(parent, CloneScope::Subtree);

    // ... and must not change the slot or the tree.
    assert!(matches!(dom.dispatcher, DispatcherSlot::Absent));
    assert_eq!(
        dom.children_iter(parent).collect::<Vec<_>>(),
        children_before
    );
    assert_eq!(
        (dom.get_parent(text), dom.get_parent(new_text)),
        parents_before
    );
}
