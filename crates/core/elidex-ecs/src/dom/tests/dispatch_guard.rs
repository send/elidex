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
    assert!(dom.dispatcher.is_some(), "dispatcher not restored");

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
        dom.dispatcher.is_some(),
        "dispatcher not restored after unwind"
    );
}
