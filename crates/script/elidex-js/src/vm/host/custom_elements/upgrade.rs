//! Custom element prototype-chain upgrade — VM-side glue around the
//! engine-indep state machine in [`elidex_custom_elements::upgrade`]
//! (HTML §4.13.5 "upgrade an element").
//!
//! `invoke_upgrade` returns `Result<(), VmError>` so each caller picks
//! its own error policy:
//!
//! - The reaction-queue drain in [`super::flush`] catches `Err` and
//!   reports it to `Window.onerror` per HTML §4.13.6 "Invoke custom
//!   element reactions" (errors observable, not thrown).
//! - The sync `customElements.upgrade(root)` walker in
//!   [`super::lookup::native_ce_upgrade`] catches `Err` per candidate
//!   and `eprintln!`s it (matches Blink's batch isolation — one bad
//!   constructor doesn't abort the remaining candidates in the
//!   subtree, and the API does not throw to the JS caller).

#![cfg(feature = "engine")]

use std::sync::{Arc, Mutex, PoisonError};

use elidex_custom_elements::{ConstructionStackEntry, CustomElementRegistry, UpgradeResolution};
use elidex_ecs::Entity;

use super::super::super::value::{JsValue, NativeContext, VmError};

/// RAII guard that pushes a construction-stack entry on
/// construction and pops it on `Drop` — including on panic-unwind.
///
/// The pop runs through Drop rather than as an explicit imperative
/// block so a panic anywhere inside `construct_synchronous` (user
/// constructor, GC, debug_assert) does NOT leave a stale `Element`
/// entry that would poison subsequent upgrades of the same
/// definition name. The HTMLElement constructor's upgrade branch
/// also replaces the top with an `AlreadyConstructed` marker
/// ([C1] §3.2.3 step 15) before this guard drops, so the popped
/// entry can be either the marker (normal path) or the original
/// `Element` (failure path before the user-ctor body reached
/// `super()`). The debug-assert below distinguishes only "popped
/// something" from "popped nothing"; an empty stack would surface
/// missing push/pop balance (a programming error in the
/// caller-callee contract).
struct ConstructionStackGuard {
    registry: Arc<Mutex<CustomElementRegistry>>,
    name: String,
}

impl ConstructionStackGuard {
    fn push(registry: Arc<Mutex<CustomElementRegistry>>, name: String, entity: Entity) -> Self {
        {
            let mut reg = registry.lock().expect("CE registry mutex poisoned");
            reg.push_construction_stack(&name, entity);
        }
        Self { registry, name }
    }
}

impl Drop for ConstructionStackGuard {
    fn drop(&mut self) {
        // `Mutex::lock` returns `Err` if the mutex is poisoned, i.e.
        // a prior holder panicked. The CE registry is per-VM and
        // single-threaded, so the only realistic poison source is a
        // panic inside one of our own lock scopes — exactly the
        // scenario this guard exists to recover from (running the pop
        // during unwind so a subsequent upgrade of the same definition
        // does not see a stale entry). `unwrap_or_else(|e|
        // e.into_inner())` consumes the `PoisonError` and recovers the
        // underlying guard, letting the pop run regardless of poison
        // state. The recovered data is still consistent: only the CE
        // registry's interior is observed, and the registry is local
        // to this VM, so no cross-VM invariant rides on the poison
        // flag.
        let mut reg = self.registry.lock().unwrap_or_else(PoisonError::into_inner);
        let popped = reg.pop_construction_stack(&self.name);
        debug_assert!(
            matches!(
                popped,
                Some(
                    ConstructionStackEntry::AlreadyConstructed
                        | ConstructionStackEntry::Element(_)
                )
            ),
            "ConstructionStackGuard pop returned {popped:?} (expected Element or AlreadyConstructed) \
             for definition '{}'",
            self.name
        );
    }
}

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
    let (constructor_id, observed_attributes, definition_name) = {
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
        match elidex_custom_elements::prepare_upgrade(host.dom_shared(), &registry, entity) {
            UpgradeResolution::Skip => return Ok(()),
            UpgradeResolution::Proceed {
                constructor_id,
                observed_attributes,
            } => {
                // Definition name needed for the construction-stack
                // push/peek/replace plumbing (D-17b §6 + \[C4\] step 6).
                // Resolve via reverse lookup so we don't grow
                // `UpgradeResolution` with this engine-bound detail.
                // `prepare_upgrade` already resolved the definition,
                // so a None here means the registry was mutated
                // concurrently — surface as VmError::internal so the
                // invariant violation is visible at the upgrade
                // boundary rather than silently abandoned (which
                // would leave the entity in CEState::Undefined,
                // re-enqueued indefinitely).
                let name = match registry.lookup_by_constructor(constructor_id) {
                    Some(def) => def.name.clone(),
                    None => {
                        return Err(VmError::internal(
                            "invoke_upgrade: lookup_by_constructor returned None after \
                             prepare_upgrade succeeded — registry invariant violated",
                        ));
                    }
                };
                (constructor_id, observed_attributes, name)
            }
        }
    };
    let Some(constructor) = host.ce_constructors.get(&constructor_id).copied() else {
        return Ok(());
    };

    // Phase 2: validate `constructor.prototype` is an Object. The
    // wrapper-prototype splice happens via
    // [`super::html_element::set_wrapper_prototype`] in Phase 3
    // (pre-publication invariant per D-17b §6) + again inside the
    // HTMLElement constructor's upgrade branch (\[C1\] §3.2.3 step 14)
    // — both writes derive the same `proto_id` so the SoT is single
    // by construction. The early validation here mirrors the spec's
    // §4.13.5 prep check so adversarial accessor throws mark the
    // entity Failed before any state transition.
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

    // Phase 3: resolve / pre-publish the element wrapper.
    //
    // (\[C1\] §3.2.3 step 14 + D-17b §6 pre-publication invariant) —
    // splice the wrapper's `[[Prototype]]` to `constructor.prototype`
    // BEFORE pushing the construction stack so any synchronous
    // observer between this point and the HTMLElement ctor's
    // matching write inside the user-ctor body (cached-wrapper
    // lookup, document.querySelector, construction-stack peek) sees
    // the correct prototype chain. This is a defense-in-depth
    // idempotent write — the HTMLElement ctor's upgrade branch also
    // calls `set_wrapper_prototype` and the proto_id derives from
    // the same `constructor.prototype`, so SoT divergence is
    // impossible.
    let wrapper_id = ctx.vm.create_element_wrapper(entity);
    super::html_element::set_wrapper_prototype(ctx.vm, wrapper_id, constructor)?;

    // Phase 4: state transition + construction-stack push + construct.
    {
        let dom = ctx.host().dom();
        elidex_custom_elements::enter_constructor(dom, entity);
    }
    // Push the construction-stack entry (\[C2\] field + \[C4\] §4.13.5
    // step 6) inside an RAII guard so the matching pop fires on
    // Drop regardless of how this scope exits — `Ok(())`, early
    // `Err`, or a downstream panic. Without the guard, a panic
    // unwinding past the explicit pop block (e.g. a debug_assert
    // tripping inside `construct_synchronous` or the GC trace)
    // would leave a stale `Element` entry that poisons subsequent
    // upgrades of the same definition name.
    let _stack_guard = {
        let host = ctx.host();
        ConstructionStackGuard::push(
            std::sync::Arc::clone(&host.ce_registry),
            definition_name.clone(),
            entity,
        )
    };
    // Construct-mode invocation (\[C11\] [[Construct]]) — drives
    // construct_synchronous so the HTMLElement ctor's upgrade branch
    // sees `new_target = constructor` (the originally-invoked CE
    // class) + `native_construct_stack` top = Some(constructor) +
    // `is_construct() == true`. Routed via the Stage 3 helper
    // (single SoT for JS-side + native-side construct dispatch).
    let result = ctx.vm.construct_synchronous(
        constructor,
        JsValue::Object(wrapper_id),
        &[],
        constructor,
        Some(wrapper_id),
    );
    // `_stack_guard` drops here (whether `result` is Ok or Err) —
    // its Drop impl pops the construction stack. The matching-shape
    // assertion lives in the guard's Drop body.
    match result {
        Ok(value) => {
            // HTML §4.13.5 "upgrade an element" step 12.2 — if
            // SameValue(constructResult, element) is false, throw a
            // "NotSupportedError" DOMException AND mark the element
            // Failed. D-17b §5 routes the user ctor through
            // `construct_synchronous`, which DOES apply the
            // `[[Construct]]` return-substitution (non-Object return
            // → `Object(pre_alloc_instance)` — dispatch_class.rs
            // ~225). With `pre_alloc_instance = Some(wrapper_id)`
            // (passed from Phase 3), a non-Object return from the
            // user body now arrives here as `Object(wrapper_id)` —
            // matching wrapper_id, SameValue passes, the primitive-
            // return branch is dead. Op::ReturnUndefined for class-
            // ctor frames was also patched (dispatch.rs) to return
            // Undefined explicitly rather than the trailing
            // ExpressionStatement's completion value, so a
            // user body like `constructor() { super(); ({}); }`
            // returns Undefined → substituted to `Object(wrapper_id)`
            // → SameValue passes. The check still fires only for an
            // EXPLICIT `return otherObj;` whose otherObj differs
            // from the wrapper — the spec-mandated guard against
            // `constructor() { return otherObj; }`.
            if matches!(value, JsValue::Object(id) if id != wrapper_id) {
                finalize_failure_shim(ctx, entity);
                let not_supported = ctx.vm.well_known.dom_exc_not_supported_error;
                return Err(VmError::dom_exception(
                    not_supported,
                    "Failed to upgrade custom element: constructor returned a \
                     different object than the element being upgraded.",
                ));
            }
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

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use elidex_custom_elements::CustomElementDefinition;
    use elidex_ecs::EcsDom;

    use super::*;

    #[test]
    fn construction_stack_guard_pops_under_poisoned_mutex() {
        // Regression for `ConstructionStackGuard::drop` previously
        // early-returning on `Mutex::lock()` poison — the exact
        // panic-inside-our-own-lock-scope case the guard exists to
        // recover from would skip the pop, leaving a stale entry that
        // would corrupt a subsequent upgrade of the same definition.
        let registry = Arc::new(Mutex::new(CustomElementRegistry::new()));
        let def = CustomElementDefinition::new("test-el".to_string(), 1, Vec::new(), None);
        registry.lock().unwrap().define(def).unwrap();

        let mut dom = EcsDom::new();
        let entity = dom.create_element("test-el", elidex_ecs::Attributes::default());

        // Push BEFORE poisoning (`push` uses `.expect` on the lock;
        // the invariant under test is `Drop`, not `push`).
        let guard =
            ConstructionStackGuard::push(Arc::clone(&registry), "test-el".to_string(), entity);

        // Poison via panic-while-holding-lock under `catch_unwind`.
        // `MutexGuard::drop` during the unwind flips the poison flag,
        // mirroring the production-relevant case where a `debug_assert!`
        // panic inside a guarded scope poisons the per-VM registry.
        let r_clone = Arc::clone(&registry);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _g = r_clone.lock().unwrap();
            panic!("intentional poison for ConstructionStackGuard regression test");
        }));
        assert!(
            registry.is_poisoned(),
            "registry mutex should be poisoned after panic-while-holding"
        );

        // Drop the guard. The Drop impl MUST run pop despite poison.
        drop(guard);

        let reg = registry.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(
            reg.peek_construction_stack("test-el").is_none(),
            "ConstructionStackGuard::drop must pop even when mutex is poisoned"
        );
    }
}
