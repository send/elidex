//! `EcsDom::dispatch_event`'s take-and-restore guard.
//!
//! The old raw-pointer guard was undefined behaviour under Stacked Borrows.
//! The normal runner checks only the restore behaviour; the borrow-validity
//! half shows only under `cargo +nightly miri test` (red on the raw-pointer
//! guard, green on this one).

use std::sync::{Arc, Mutex};

use super::*;

/// Counts every dispatch.
#[derive(Default)]
struct CountHook {
    count: Arc<Mutex<usize>>,
}

impl MutationDispatcher for CountHook {
    fn dispatch(&mut self, _event: &MutationEvent<'_>, _dom: &mut EcsDom) {
        *self.count.lock().unwrap() += 1;
    }
}

/// Panics on every dispatch.
struct PanicHook;

impl MutationDispatcher for PanicHook {
    fn dispatch(&mut self, _event: &MutationEvent<'_>, _dom: &mut EcsDom) {
        panic!("intentional dispatcher panic");
    }
}

#[test]
fn dispatch_event_restores_dispatcher_and_depth() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    let hook = CountHook::default();
    let count = hook.count.clone();
    dom.set_mutation_dispatcher(Box::new(hook));

    assert!(dom.set_attribute(el, "id", "a").did_set);
    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(dom.dispatch_depth, 0);
    assert!(
        matches!(dom.dispatcher, DispatcherSlot::Installed(_)),
        "dispatcher not restored"
    );

    // The restored dispatcher keeps firing.
    dom.remove_attribute(el, "id");
    assert_eq!(*count.lock().unwrap(), 2);
    assert_eq!(dom.dispatch_depth, 0);
}

/// Same restore contract when the callback panics: unwinding drops the
/// guard, which restores the slot and the depth counter.
#[test]
fn dispatch_event_restores_dispatcher_and_depth_after_panic() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    dom.set_mutation_dispatcher(Box::new(PanicHook));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dom.set_attribute(el, "id", "a");
    }));
    assert!(result.is_err(), "dispatcher panic should propagate");
    assert_eq!(dom.dispatch_depth, 0, "depth not restored after unwind");
    assert!(
        matches!(dom.dispatcher, DispatcherSlot::Installed(_)),
        "dispatcher not restored after unwind"
    );
}

/// (w1) A window left open (take without restore) makes `fire_clone_created`
/// assert: the event would be dropped.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "dispatcher is taken out")]
fn fire_clone_created_asserts_while_dispatcher_is_taken() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    dom.set_mutation_dispatcher(Box::new(CountHook::default()));
    let _window = dom.take_mutation_dispatcher();
    dom.fire_clone_created(el, CloneScope::Subtree);
}

/// (w2) A take that finds nothing opens no window: the unbound `EcsDom` stays a
/// silent no-op.
#[test]
fn fire_clone_created_is_silent_after_empty_take() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    dom.fire_clone_created(el, CloneScope::Subtree);
    assert!(dom.take_mutation_dispatcher().is_none());
    dom.fire_clone_created(el, CloneScope::Subtree);
    assert!(!matches!(dom.dispatcher, DispatcherSlot::Taken));
}

/// (w3) Take, then restore: the window is closed and the event dispatches.
#[test]
fn fire_clone_created_dispatches_after_take_and_restore() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    let hook = CountHook::default();
    let count = hook.count.clone();
    dom.set_mutation_dispatcher(Box::new(hook));
    let saved = dom.take_mutation_dispatcher().expect("installed");
    assert!(dom.set_mutation_dispatcher(saved).is_none());
    dom.fire_clone_created(el, CloneScope::SelfOnly);
    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(dom.dispatch_depth, 0);
}

/// `clear_mutation_dispatcher` closes a pending window too.
#[test]
fn clear_closes_a_taken_window() {
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    dom.set_mutation_dispatcher(Box::new(CountHook::default()));
    let _window = dom.take_mutation_dispatcher();
    dom.clear_mutation_dispatcher();
    dom.fire_clone_created(el, CloneScope::Subtree);
}

/// A clone fired from inside a dispatch is the re-entry case: the depth
/// assertion fires (the slot is `Absent` there, so the window check is quiet).
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "violates re-entry contract")]
fn fire_clone_created_from_inside_dispatch_hits_reentry_assertion() {
    struct Reenter(Entity);
    impl MutationDispatcher for Reenter {
        fn dispatch(&mut self, _event: &MutationEvent<'_>, dom: &mut EcsDom) {
            dom.fire_clone_created(self.0, CloneScope::Subtree);
        }
    }
    let mut dom = EcsDom::new();
    let el = dom.create_element("div", Attributes::default());
    dom.set_mutation_dispatcher(Box::new(Reenter(el)));
    dom.fire_clone_created(el, CloneScope::Subtree);
}
