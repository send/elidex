//! `globalThis.HTMLElement` constructor — the spec-mandated CE
//! constructor brand (\[C1\] §3.2.3 "HTML element constructors").
//!
//! Provides three pieces.
//!
//! [`VmInner::register_html_element_constructor`] is the global init
//! entry — installs `globalThis.HTMLElement` as a constructable
//! function bound to [`native_html_element_ctor`], wires its
//! `prototype` slot to the existing `html_element_prototype`, and
//! stores the `ObjectId` on [`VmInner::html_element_constructor`]
//! for the \[C1\] step 1 illegal-ctor brand check.
//!
//! [`native_html_element_ctor`] is the constructor body. It
//! implements both spec paths via `NewTarget` discrimination: direct
//! `new HTMLElement()` rejection (\[C1\] step 1), the upgrade path
//! (\[C1\] steps 12-15: peek + replace-with-marker), and the sync
//! construct path (\[C1\] step 9: spawn fresh Element entity).
//!
//! [`set_wrapper_prototype`] is the one-issue-one-way helper that
//! splices `wrapper.[[Prototype]] = proto_id` (\[C1\] step 14 +
//! sync-construct internal-create). Called from three sites (D-17b
//! §5): `invoke_upgrade` Phase 3 pre-publication invariant, this
//! file's upgrade-path branch (\[C1\] step 14, spec-narrative), and
//! this file's sync-construct branch. The helper takes a pre-validated
//! `proto_id` so each caller does its own `Get(ctor, "prototype")` +
//! Object-validation locally — `invoke_upgrade` Phase 2 reuses the
//! already-validated value into Phase 3 instead of re-reading the
//! user's `ctor.prototype` accessor (D-17b R2 G2, prevents duplicate
//! side-effects + the 2nd-read-throws-without-Failed-mark gap).

#![cfg(feature = "engine")]

use std::sync::PoisonError;

use elidex_custom_elements::ConstructionStackEntry;

use super::super::super::shape::PropertyAttrs;
use super::super::super::value::{
    JsValue, NativeContext, ObjectId, PropertyKey, PropertyValue, VmError,
};
use super::super::super::VmInner;

impl VmInner {
    /// Install `globalThis.HTMLElement` as a constructable native
    /// function (\[C1\] §3.2.3 + \[C18\] WebIDL §3.7.1 Interface object).
    /// Called from `register_globals` after
    /// `register_html_element_prototype` (chains the `prototype` slot)
    /// AND after `register_custom_element_registry_global` (CE
    /// registry must exist before `define` calls can land an entry
    /// the HTMLElement ctor can resolve).
    ///
    /// # Panics
    ///
    /// Panics if `html_element_prototype` has not been populated
    /// (means `register_html_element_prototype` was skipped or called
    /// out of order).
    pub(in crate::vm) fn register_html_element_constructor(&mut self) {
        let proto_id = self.html_element_prototype.expect(
            "register_html_element_constructor called before register_html_element_prototype",
        );

        // Allocate the ConstructorOnly function object — bare
        // `HTMLElement()` (no `new`) is rejected by the dispatch-side
        // gate at `vm/interpreter.rs::call_dispatch` (WebIDL §3.7.1
        // step 1.2 / [HTMLConstructor]).  The `name` slot is the
        // well-known interned "HTMLElement" so stack traces and
        // `func.name` reads stay coherent.
        let ctor_id =
            self.create_constructor_only_function("HTMLElement", native_html_element_ctor);

        // `HTMLElement.prototype` — wires the existing prototype as
        // the constructor's `prototype` slot (\[C12\] MakeConstructor:
        // `{ ¬W, ¬E, ¬C }` on the ctor side per spec).
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor_id,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );

        // `HTMLElement.prototype.constructor = HTMLElement` (\[C12\]
        // MakeConstructor — `{ W, ¬E, C }` on the prototype side).
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor_id)),
            PropertyAttrs::METHOD,
        );

        // `globalThis.HTMLElement` (\[C18\] WebIDL Interface object —
        // `{ W, ¬E, C }`). Mirrors how every other constructable
        // native is exposed (`Error`, `Promise`, `Event`, …).
        let global_name = self.well_known.html_element_global;
        self.globals.insert(global_name, JsValue::Object(ctor_id));

        // Stash the ObjectId for the \[C1\] step 1 illegal-ctor check
        // + the §3.2.3 HTMLConstructor chain check in
        // `customElements.define`.
        self.html_element_constructor = Some(ctor_id);
    }
}

/// HTMLElement constructor body (\[C1\] §3.2.3). Read `new.target`,
/// dispatch the upgrade / sync-construct / illegal-direct paths.
pub(crate) fn native_html_element_ctor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Step 0: `new.target` is guaranteed present by the dispatch-side
    // `CallShape::ConstructorOnly` gate at `vm/interpreter.rs::call_dispatch`
    // (the bare-call rejection in WebIDL §3.7.1 step 1.2 fires before
    // this body runs).  The `expect` documents the invariant rather
    // than masking a real `None` branch.
    let new_target = ctx
        .new_target()
        .expect("ConstructorOnly dispatch gate guarantees NewTarget is present");

    // Step 1 (\[C1\] step 1): reject direct `new HTMLElement()`.
    // Defense-in-depth: surface a clean TypeError rather than a panic
    // if a future embedder snapshot/restore path reaches the ctor
    // before `register_html_element_constructor` ran.
    let html_element_ctor = ctx.vm.html_element_constructor.ok_or_else(|| {
        VmError::internal(
            "HTMLElement constructor invoked before register_html_element_constructor",
        )
    })?;
    if new_target == html_element_ctor {
        return Err(VmError::type_error(
            "Failed to construct 'HTMLElement': Illegal constructor",
        ));
    }

    // Step 2 (\[C1\] §3.2.3 step 5 — reverse-map new.target → registered
    // CE definition): read constructor_id from the host-side reverse
    // map [`HostData::ce_constructor_to_id`]. The map is host Rust
    // state with no JS-visible counterpart, so spoofing is impossible
    // by construction: user code cannot synthesize an entry pointing
    // at an unregistered ctor (D-17b R2 G1 — replaces the earlier
    // symbol-keyed brand, which was discoverable via
    // `Object.getOwnPropertySymbols` and copyable onto an arbitrary
    // ctor object). An unregistered or unrelated ObjectId → not a
    // registered CE constructor → TypeError.
    let Some(host) = ctx.host_if_bound() else {
        return Err(VmError::type_error(
            "Failed to construct 'HTMLElement': host is not bound",
        ));
    };
    let Some(&constructor_id) = host.ce_constructor_to_id.get(&new_target) else {
        return Err(VmError::type_error(
            "Failed to construct 'HTMLElement': new.target is not a registered custom element",
        ));
    };
    let definition_name = {
        let registry = host
            .ce_registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match registry.lookup_by_constructor(constructor_id) {
            Some(def) => def.name.clone(),
            None => {
                return Err(VmError::type_error(
                    "Failed to construct 'HTMLElement': constructor was unregistered",
                ));
            }
        }
    };

    // Step 3 (\[C1\] step 12 — peek construction stack): upgrade path.
    let stack_top = {
        let registry = host
            .ce_registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        registry.peek_construction_stack(&definition_name).cloned()
    };
    match stack_top {
        Some(ConstructionStackEntry::AlreadyConstructed) => {
            // \[C1\] step 13 — re-entrant construct against the same
            // upgrade slot throws TypeError.
            Err(VmError::type_error(
                "Custom element constructor invoked re-entrantly on the same instance",
            ))
        }
        Some(ConstructionStackEntry::Element(entity)) => {
            // \[C1\] steps 14-15 — upgrade path. invoke_upgrade has
            // already pre-allocated the wrapper (D-17 + D-17b §6
            // pre-publication invariant), so we look it up by entity,
            // splice prototype (\[C1\] step 14, idempotent confirmation
            // with §6 pre-write — both writes derive the same
            // proto_id from the same constructor), and replace the
            // stack top with the AlreadyConstructed marker (\[C1\]
            // step 15). SameValue check (\[C4\] step 9.4) is the
            // upgrade caller's responsibility — the returned wrapper
            // equals the upgrade-allocated wrapper by construction
            // here.
            let Some(wrapper_id) = ctx.host().get_cached_wrapper(entity) else {
                return Err(VmError::internal(
                    "HTMLElement ctor upgrade branch: construction-stack entity has no cached wrapper",
                ));
            };
            let proto_id = resolve_validated_prototype(ctx.vm, new_target)?;
            set_wrapper_prototype(ctx.vm, wrapper_id, proto_id);
            {
                let host = ctx.host();
                let mut registry = host
                    .ce_registry
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                let popped = registry.replace_construction_stack_top_with_marker(&definition_name);
                debug_assert_eq!(
                    popped,
                    Some(entity),
                    "replace marker did not return the pushed entity"
                );
            }
            Ok(JsValue::Object(wrapper_id))
        }
        None => {
            // Step 4 (\[C1\] step 9 — empty construction stack): sync
            // construct path. Spawn a fresh Element entity at the
            // definition's local name, allocate a wrapper around it,
            // splice the prototype from new.target.prototype. The
            // `owner` argument plumbs the current document so the
            // new element reports a non-null `ownerDocument` — same
            // shape that `document.createElement('my-el')` produces.
            // Without this, `new MyEl()` and `createElement('my-el')`
            // diverge on the DOM §4.4 "node document" invariant.
            let owner = ctx.host().document_entity_opt();
            let entity = elidex_custom_elements::spawn_custom_element_entity(
                ctx.host().dom(),
                &definition_name,
                &definition_name,
                owner,
            );
            // Allocate (or retrieve) the primary node wrapper via
            // the existing seam — matches how every other
            // sync-constructed Element is exposed to JS.
            let wrapper_id = ctx.vm.create_element_wrapper(entity);
            let proto_id = resolve_validated_prototype(ctx.vm, new_target)?;
            set_wrapper_prototype(ctx.vm, wrapper_id, proto_id);
            Ok(JsValue::Object(wrapper_id))
        }
    }
}

/// Splice `wrapper.[[Prototype]] = proto_id` per \[C1\] step 14 +
/// the sync-construct internal-create's prototype derivation. Pure
/// mechanical mutation: the caller resolves and validates `proto_id`
/// itself, either via [`resolve_validated_prototype`] or by reusing
/// an already-validated value from an earlier `Get`. `invoke_upgrade`
/// Phase 2 takes the latter path so the user's `ctor.prototype`
/// accessor runs once per upgrade instead of twice (D-17b R2 G2).
pub(crate) fn set_wrapper_prototype(vm: &mut VmInner, wrapper_id: ObjectId, proto_id: ObjectId) {
    vm.get_object_mut(wrapper_id).prototype = Some(proto_id);
}

/// Helper that performs `Get(ctor_id, "prototype")` + Object-validation.
///
/// Returns `Err(TypeError)` when `ctor.prototype` is not an Object —
/// matches WebIDL's "must be an object" wording for the
/// `CustomElementConstructor` callback type. Returns the inner
/// `ObjectId` on success so the caller can pass it directly to
/// [`set_wrapper_prototype`]. Kept separate so callers that already
/// have a validated proto_id — `invoke_upgrade` Phase 2 — can skip
/// the second user-accessor invocation.
pub(crate) fn resolve_validated_prototype(
    vm: &mut VmInner,
    ctor_id: ObjectId,
) -> Result<ObjectId, VmError> {
    let proto_key = PropertyKey::String(vm.well_known.prototype);
    let proto_value = vm.get_property_value(ctor_id, proto_key)?;
    let JsValue::Object(proto_id) = proto_value else {
        return Err(VmError::type_error(
            "Custom element constructor.prototype must be an object",
        ));
    };
    Ok(proto_id)
}

/// Verify that `ctor_id`'s `[[Prototype]]` chain reaches
/// `vm.html_element_constructor` — the HTMLConstructor brand-check
/// semantics per \[C1\] §3.2.3 invoked from \[C3\] §4.13.4 `define`
/// algorithm (HTML §4.13.4 step 10 verifies the constructor extends
/// HTMLElement).
///
/// Bounded by the VM-wide [`coerce::PROTO_CHAIN_LIMIT`] — the same
/// cap used by property lookup, `instanceof`, and the canvas
/// `ImageData` brand walk (`host/canvas/image_data.rs::prototype_chain_has_image_data`).
/// Reusing the shared constant keeps brand-check surfaces consistent:
/// a deep-but-valid subclass chain reachable to any other brand check
/// is also reachable here, and the cap doubles as the acyclicity
/// guard (prototype chains are acyclic in normal operation; user
/// `Object.setPrototypeOf` cycles are bounded by the same budget).
/// Reaching `vm.html_element_constructor` before exhausting the
/// budget = pass; otherwise TypeError so the user observes a clear
/// "must extend HTMLElement" failure rather than a silent
/// succeed-at-define-but-broken-at-upgrade path.
pub(crate) fn validate_html_element_constructor_chain(
    vm: &VmInner,
    ctor_id: ObjectId,
) -> Result<(), VmError> {
    let html_ctor = vm.html_element_constructor.ok_or_else(|| {
        VmError::internal(
            "customElements.define: HTMLElement constructor not registered \
             (register_html_element_constructor must run before define)",
        )
    })?;
    // Reject `customElements.define(name, HTMLElement)` at define-time
    // rather than letting it pass the chain walk (HTMLElement trivially
    // appears in its own prototype chain at hop 0) only to fail later
    // at sync-construct / upgrade time via the [\[C1\] step 1
    // illegal-ctor branch. Surface the constraint where the user can
    // act on it (D-17b R10 G10-1).
    if ctor_id == html_ctor {
        return Err(VmError::type_error(
            "Failed to execute 'define' on 'CustomElementRegistry': \
             constructor must extend HTMLElement, not be HTMLElement itself.",
        ));
    }
    let mut current = Some(ctor_id);
    for _ in 0..super::super::super::coerce::PROTO_CHAIN_LIMIT {
        match current {
            Some(id) if id == html_ctor => return Ok(()),
            Some(id) => {
                current = vm.get_object(id).prototype;
            }
            None => break,
        }
    }
    Err(VmError::type_error(
        "Failed to execute 'define' on 'CustomElementRegistry': \
         constructor must extend HTMLElement",
    ))
}
