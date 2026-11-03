//! `after_remove` descendants snapshot / re-entry / attribute-change hook tests —
//! split out from `mutation_hook.rs` (~1000-line file convention).

use std::sync::{Arc, Mutex};

use super::*;
use crate::dom::{MutationDispatcher, MutationEvent};

// ---------------------------------------------------------------------------
// after_remove descendants snapshot (PR186 R2 #3 regression)
// ---------------------------------------------------------------------------

type DescendantSnapshotLog = Vec<(Entity, Vec<Entity>)>;

#[derive(Default, Clone)]
struct DescendantSnapshotHook {
    snapshot: Arc<Mutex<DescendantSnapshotLog>>,
}

impl MutationDispatcher for DescendantSnapshotHook {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &mut crate::EcsDom) {
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

/// A deliberately-misbehaving dispatcher that re-enters the EcsDom
/// from inside its `dispatch` body — used to test the re-entry
/// detection `debug_assert!` in [`EcsDom::dispatch_event`].
struct ReEntryHook;

impl MutationDispatcher for ReEntryHook {
    fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut crate::EcsDom) {
        // Only re-enter on Insert; avoid infinite recursion if the
        // assert was disabled (release build).
        if let MutationEvent::Insert { parent, .. } = *event {
            let _ = dom.set_attribute(parent, "data-reentry", "1");
        }
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "re-entry contract")]
fn dispatcher_calling_mutation_primitive_panics_in_debug() {
    // The `MutationDispatcher::dispatch` docstring forbids consumers
    // from calling EcsDom mutation primitives recursively.  The
    // `EcsDom::dispatch_event` debug_assert (re-entry depth counter)
    // catches violations in debug builds.
    let mut dom = crate::EcsDom::new();
    let root = dom.create_document_root();
    let parent = dom.create_element("div", crate::Attributes::default());
    let child = dom.create_element("span", crate::Attributes::default());
    assert!(dom.append_child(root, parent));
    dom.set_mutation_dispatcher(Box::new(ReEntryHook));
    // The append_child below fires an Insert event; ReEntryHook
    // calls set_attribute back, which fires its own dispatch_event
    // with depth > 0 → debug_assert!.
    let _ = dom.append_child(parent, child);
}

/// Records the number of `MutationEvent::AttributeChange` dispatches.
#[derive(Default, Clone)]
struct AttrChangeCountHook {
    count: Arc<Mutex<usize>>,
}

impl MutationDispatcher for AttrChangeCountHook {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &mut crate::EcsDom) {
        if matches!(*event, MutationEvent::AttributeChange { .. }) {
            *self.count.lock().unwrap() += 1;
        }
    }
}

/// Codex #335 R8 F27 / DOM "remove an attribute by name" step 2: an
/// attribute is removed — and a mutation record queued — only when it is
/// non-null. `remove_attribute` on an absent attribute must NOT dispatch
/// `MutationEvent::AttributeChange`, so MutationObserver-style consumers
/// don't observe phantom removals on no-op `removeAttribute` calls.
#[test]
fn remove_absent_attribute_fires_no_mutation_record() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", crate::Attributes::default());
    // Seed an attribute BEFORE installing the hook so the set isn't counted.
    assert!(dom.set_attribute(el, "id", "x").did_set);

    let hook = AttrChangeCountHook::default();
    let count = hook.count.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    // Removing a present attribute fires exactly one record.
    dom.remove_attribute(el, "id");
    assert_eq!(*count.lock().unwrap(), 1);

    // Removing an already-absent / never-present attribute fires none.
    dom.remove_attribute(el, "id");
    dom.remove_attribute(el, "never-existed");
    assert_eq!(
        *count.lock().unwrap(),
        1,
        "phantom AttributeChange fired for an absent removeAttribute"
    );
}
