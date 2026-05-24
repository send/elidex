//! Custom element prototype-chain upgrade — VM-side glue around the
//! engine-indep state machine in [`elidex_custom_elements::upgrade`]
//! (HTML §4.13.5 "upgrade an element").
//!
//! `invoke_upgrade` returns `Result<(), VmError>` so each caller picks
//! its own error policy:
//!
//! - The reaction-queue drain in [`super::flush`] catches `Err` and
//!   reports it to `Window.onerror` per HTML §4.13.3 "Invoke custom
//!   element reactions" (errors observable, not thrown).
//! - The sync `customElements.upgrade(root)` walker in
//!   [`super::lookup::native_ce_upgrade`] catches `Err` per candidate
//!   and `eprintln!`s it (matches Blink's batch isolation — one bad
//!   constructor doesn't abort the remaining candidates in the
//!   subtree, and the API does not throw to the JS caller).

#![cfg(feature = "engine")]

use elidex_custom_elements::UpgradeResolution;
use elidex_ecs::Entity;

use super::super::super::value::{JsValue, NativeContext, VmError};

/// Invoke the upgrade algorithm on `entity` per HTML §4.13.5.
///
/// Returns:
/// - `Ok(())` on a successful upgrade (state transitions to
///   `CEState::Custom`), with `attributeChangedCallback` /
///   `connectedCallback` reactions appended to the queue for the
///   element's existing observed attributes + connectedness.
/// - `Err(VmError)` on a constructor throw — caller decides whether to
///   rethrow (sync `customElements.upgrade(root)` path) or report to
///   Window.onerror (reaction-queue flush path).
///
/// The state machine itself (resolve definition → enter Precustomized
/// → mark Custom/Failed → enqueue Connected/AttributeChanged) lives in
/// the engine-indep crate; this function only sequences the
/// engine-bound steps (constructor.prototype Object validation,
/// element-wrapper allocation, `ctx.vm.call`) between
/// [`elidex_custom_elements::prepare_upgrade`] /
/// [`enter_constructor`] / [`finalize_success`] /
/// [`finalize_failure`].
pub(crate) fn invoke_upgrade(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<(), VmError> {
    let Some(host) = ctx.host_if_bound() else {
        return Ok(());
    };

    // Phase 1: resolve via engine-indep prepare_upgrade (early-returns
    // on Custom/Failed/no-def). Drops the registry lock before any VM
    // re-borrow.
    let (constructor_id, observed_attributes) = {
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
        match elidex_custom_elements::prepare_upgrade(host.dom_shared(), &registry, entity) {
            UpgradeResolution::Skip => return Ok(()),
            UpgradeResolution::Proceed {
                constructor_id,
                observed_attributes,
            } => (constructor_id, observed_attributes),
        }
    };
    let Some(constructor) = host.ce_constructors.get(&constructor_id).copied() else {
        return Ok(());
    };

    // Phase 2: validate `constructor.prototype` is an Object — even
    // though we do NOT splice it into the wrapper's prototype chain in
    // v1 (`#11-html-element-constructor-base` slot), the spec requires
    // the property to be an object for the upgrade to proceed. Any
    // throw inside the getter (adversarial proxy / accessor) marks the
    // entity Failed before propagating so re-enqueued Upgrades short-
    // circuit at prepare_upgrade's early-return rather than retrying
    // the same throw.
    let proto_key = super::super::super::value::PropertyKey::String(ctx.vm.well_known.prototype);
    let proto_value = match ctx.vm.get_property_value(constructor, proto_key) {
        Ok(v) => v,
        Err(err) => {
            finalize_failure_shim(ctx, entity);
            return Err(err);
        }
    };
    if !matches!(proto_value, JsValue::Object(_)) {
        finalize_failure_shim(ctx, entity);
        return Err(VmError::type_error(
            "Custom element upgrade failed: constructor.prototype must be an object.",
        ));
    }

    // Phase 3: resolve the element wrapper.
    //
    // Spec (HTML §4.13.5 step 5) sets element's [[Prototype]] to
    // constructor.prototype, but in v1 elidex `class MyEl extends
    // HTMLElement` is unwired (`HTMLElement` is not a global
    // constructor — `#11-html-element-constructor-base` slot). Without
    // an HTMLElement super-class, splicing constructor.prototype onto
    // the wrapper would orphan the wrapper from Element.prototype and
    // break `el.setAttribute` / `el.appendChild` / every Element /
    // Node method. Until that slot lands, the wrapper keeps its
    // standard Element-chain prototype; lifecycle callbacks are looked
    // up on `constructor.prototype` directly by
    // [`super::flush::invoke_callback`].
    let wrapper_id = ctx.vm.create_element_wrapper(entity);

    // Phase 4: state transition + construct.
    {
        let dom = ctx.host().dom();
        elidex_custom_elements::enter_constructor(dom, entity);
    }
    let saved_construct = ctx.vm.in_construct;
    ctx.vm.in_construct = true;
    let result = ctx.vm.call(constructor, JsValue::Object(wrapper_id), &[]);
    ctx.vm.in_construct = saved_construct;
    match result {
        Ok(_) => {
            let host = ctx.host();
            // Clone the queue Arc BEFORE the `&mut EcsDom` re-borrow —
            // the Arc is a disjoint owned field of HostData, so the
            // MutexGuard projected from the clone shares no aliasing
            // with the `dom()` re-borrow.
            let queue_arc = std::sync::Arc::clone(&host.ce_reaction_queue);
            let dom = host.dom();
            let mut queue = queue_arc.lock().expect("CE reaction queue mutex poisoned");
            elidex_custom_elements::finalize_success(dom, &mut queue, entity, &observed_attributes);
        }
        Err(err) => {
            finalize_failure_shim(ctx, entity);
            return Err(err);
        }
    }

    Ok(())
}

/// Thin shim that locks the per-VM reaction queue + calls into the
/// engine-indep [`elidex_custom_elements::finalize_failure`] helper
/// (mark Failed + scrub pending reactions).
fn finalize_failure_shim(ctx: &mut NativeContext<'_>, entity: Entity) {
    let host = ctx.host();
    let queue_arc = std::sync::Arc::clone(&host.ce_reaction_queue);
    let dom = host.dom();
    let mut queue = queue_arc.lock().expect("CE reaction queue mutex poisoned");
    elidex_custom_elements::finalize_failure(dom, &mut queue, entity);
}
