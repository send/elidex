//! Shared test & benchmark helpers for engine-backed VM scenarios.
//!
//! These helpers consolidate the `install_host_data` + [`Vm::bind`]
//! boilerplate together with common element-wrapper and event-construction
//! patterns that were previously duplicated across several
//! `vm::tests::*` modules and `benches/event_dispatch.rs`.
//!
//! The module is published as `#[doc(hidden)] pub` under the `engine`
//! feature so that:
//!
//! 1. The bench crate (`benches/event_dispatch.rs`, which runs in a
//!    separate compilation unit and therefore cannot see `#[cfg(test)]`
//!    items) can reuse them.
//! 2. Library consumers do not see them in rustdoc output.
//!
//! Not part of the stable public API — do not rely on these from
//! downstream crates.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::{EventPayload, EventPhase};
use elidex_script_session::event_dispatch::DispatchEvent;
use elidex_script_session::{EventListeners, SessionCore};

use super::host_data::HostData;
use super::value::JsValue;
use super::Vm;

/// Install a fresh [`HostData`] and bind `vm` against the given
/// `session` / `dom` / `document`.
///
/// # Safety
///
/// The raw pointers derived from `session` and `dom` outlive this
/// call.  The caller must keep both allocations live and non-aliased
/// until [`Vm::unbind`] is invoked.  This mirrors [`Vm::bind`] directly.
#[allow(unsafe_code)]
pub unsafe fn bind_vm(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom, document: Entity) {
    vm.install_host_data(HostData::new());
    unsafe {
        vm.bind(session as *mut _, dom as *mut _, document);
    }
}

/// Create an element of the given `tag`, bind `vm` against
/// `session` / `dom` / `doc`, install the wrapper as `globalThis.el`,
/// and return the underlying entity.
///
/// Convenience for listener-integration tests that want a ready-to-use
/// `el` handle in JavaScript plus the Rust-side entity for direct DOM
/// inspection (`listeners_on`, etc).
///
/// # Safety
///
/// Inherits the full safety contract of [`bind_vm`] (to which this
/// function delegates): the raw pointers derived from `session` and
/// `dom` outlive this call, and the caller must keep both allocations
/// live and non-aliased — no outstanding Rust references to either —
/// until [`Vm::unbind`] is invoked.  Exposing this as a safe `fn`
/// would allow UB from purely safe code (e.g. resuming use of
/// `session` / `dom` while the VM still holds raw pointers into them).
#[allow(unsafe_code)]
pub unsafe fn setup_with_element(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
    tag: &str,
) -> Entity {
    let el = dom.create_element(tag, Attributes::default());
    // SAFETY: forwarded from the caller — `bind_vm`'s contract is
    // identical to our own, and the two are documented together.
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let wrapper_id = vm.inner.create_element_wrapper(el);
    vm.set_global("el", JsValue::Object(wrapper_id));
    el
}

/// Build a minimal [`DispatchEvent`] whose `target == current_target ==
/// entity`, with `bubbles = false`, `phase = AtTarget`, and
/// `dispatch_flag = true`.
///
/// Used by `create_event_object` tests and the event-dispatch micro
/// benchmark where propagation / capturing is not under test.
pub fn make_event(
    event_type: &str,
    cancelable: bool,
    payload: EventPayload,
    entity: Entity,
) -> DispatchEvent {
    let mut ev = DispatchEvent::new(event_type, entity);
    ev.bubbles = false;
    ev.cancelable = cancelable;
    ev.payload = payload;
    ev.phase = EventPhase::AtTarget;
    ev.current_target = Some(entity);
    ev.dispatch_flag = true;
    ev
}

/// Snapshot the [`EventListeners`] component for `entity`, returning
/// an owned clone (or [`EventListeners::default`] if the component is
/// absent).  Callers can drop the world borrow before resuming VM work.
pub fn listeners_on(dom: &EcsDom, entity: Entity) -> EventListeners {
    match dom.world().get::<&EventListeners>(entity) {
        Ok(r) => (*r).clone(),
        Err(_) => EventListeners::default(),
    }
}
