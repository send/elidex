//! Tests for the engine-independent "close the dialog" algorithm.

use super::close_the_dialog;
use crate::test_util::AttrChangeCounter;
use elidex_ecs::{Attributes, DialogReturnValue, EcsDom, Entity, IsModalDialog};

/// Create a `<dialog>` element; if `open`, seed the `open` content
/// attribute directly (bypassing the chokepoint so it does not perturb
/// any installed `AttributeChange` counter).
fn dialog(dom: &mut EcsDom, open: bool) -> Entity {
    let e = dom.create_element("dialog", Attributes::default());
    if open {
        dom.world_mut()
            .get::<&mut Attributes>(e)
            .unwrap()
            .set("open", "");
    }
    e
}

fn return_value(dom: &EcsDom, e: Entity) -> Option<String> {
    dom.world()
        .get::<&DialogReturnValue>(e)
        .ok()
        .map(|drv| drv.0.clone())
}

#[test]
fn close_open_dialog_with_result_sets_return_value_and_removes_open() {
    let mut dom = EcsDom::new();
    let d = dialog(&mut dom, true);

    let closed = close_the_dialog(&mut dom, d, Some("ok"));

    assert!(closed, "an open dialog reports it was closed");
    assert!(!dom.has_attribute(d, "open"), "`open` attribute removed");
    assert_eq!(return_value(&dom, d).as_deref(), Some("ok"));
}

#[test]
fn close_result_none_leaves_return_value_unchanged() {
    let mut dom = EcsDom::new();
    let d = dialog(&mut dom, true);
    // Pre-existing returnValue (e.g. from a prior `dialog.returnValue = …`).
    let _ = dom
        .world_mut()
        .insert_one(d, DialogReturnValue("prev".into()));

    let closed = close_the_dialog(&mut dom, d, None);

    assert!(closed);
    assert!(!dom.has_attribute(d, "open"));
    assert_eq!(
        return_value(&dom, d).as_deref(),
        Some("prev"),
        "a null result must not touch returnValue"
    );
}

#[test]
fn close_result_empty_string_sets_return_value_empty() {
    let mut dom = EcsDom::new();
    let d = dialog(&mut dom, true);
    let _ = dom
        .world_mut()
        .insert_one(d, DialogReturnValue("prev".into()));

    // `Some("")` (submit button with `value=""`) sets returnValue to "".
    let closed = close_the_dialog(&mut dom, d, Some(""));

    assert!(closed);
    assert_eq!(return_value(&dom, d).as_deref(), Some(""));
}

#[test]
fn close_not_open_is_noop() {
    let mut dom = EcsDom::new();
    let d = dialog(&mut dom, false);

    let closed = close_the_dialog(&mut dom, d, Some("ignored"));

    assert!(!closed, "a closed dialog reports nothing was closed");
    assert_eq!(
        return_value(&dom, d),
        None,
        "no returnValue written when the dialog was not open"
    );
}

#[test]
fn close_clears_modal_marker() {
    let mut dom = EcsDom::new();
    let d = dialog(&mut dom, true);
    let _ = dom.world_mut().insert_one(d, IsModalDialog);

    let closed = close_the_dialog(&mut dom, d, None);

    assert!(closed);
    assert!(
        dom.world().get::<&IsModalDialog>(d).is_err(),
        "is-modal marker cleared on close"
    );
}

#[test]
fn close_dispatches_one_attribute_change_for_open_removal() {
    let mut dom = EcsDom::new();
    let d = dialog(&mut dom, true);
    let counter = AttrChangeCounter::default();
    let count = counter.count.clone();
    dom.set_mutation_dispatcher(Box::new(counter));

    let closed = close_the_dialog(&mut dom, d, Some("v"));

    assert!(closed);
    assert_eq!(
        *count.lock().unwrap(),
        1,
        "removing `open` via the chokepoint fires exactly one AttributeChange \
         (so a MutationObserver observes the close)"
    );
}
