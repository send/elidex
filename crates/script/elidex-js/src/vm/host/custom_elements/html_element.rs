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
//! splices `wrapper.[[Prototype]] = ctor.prototype` (\[C1\] step 14 +
//! sync-construct internal-create). Called from three sites (D-17b
//! §5): `invoke_upgrade` Phase 3 pre-publication invariant, this
//! file's upgrade-path branch (\[C1\] step 14, spec-narrative), and
//! this file's sync-construct branch. The two idempotent writes of
//! the same value across the upgrade path's pre-write + in-ctor-body
//! write are defense-in-depth covering distinct observability
//! windows (pre-vm.construct vs in-ctor-body), NOT a
//! One-issue-one-way violation (R5 Step 4.5 framing correction).

#![cfg(feature = "engine")]

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

        // Allocate the constructable function object. The `name` slot
        // is the well-known interned "HTMLElement" so stack traces
        // and `func.name` reads stay coherent.
        let ctor_id = self.create_constructable_function("HTMLElement", native_html_element_ctor);

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
        // + the §4.3 HTMLConstructor chain check in
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
    // Step 0: `new.target` must be present (constructable-only).
    // `HTMLElement(...)` without `new` is a TypeError per WebIDL.
    let Some(new_target) = ctx.new_target() else {
        return Err(VmError::type_error(
            "Failed to construct 'HTMLElement': Please use the 'new' operator",
        ));
    };

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

    // Step 2 (D-17b §4.3 — JS-object brand reverse lookup): read the
    // `$$elidexCEConstructorId` brand off `new.target`. **Own
    // property only** — a chain-walking read would let an
    // unregistered subclass of a registered CE inherit the parent's
    // brand and silently impersonate the parent's definition
    // (`class Child extends MyEl {}` resolving to `<my-el>`); spec
    // (\[C1\] §3.2.3 step 5) reverse-maps via the realm's CE
    // registry, not via a property-chain read. Absent / wrong-shape
    // own value → not a registered CE constructor → TypeError.
    // Symbol-keyed brand (D-17b §4.3 + N8 fix) — see define.rs
    // for the rationale. User code cannot reach a well-known
    // Symbol via any string-key reflection.
    let brand_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.ce_constructor_id_brand);
    let constructor_id = {
        let obj = ctx.vm.get_object(new_target);
        let own = obj.storage.get(brand_key, &ctx.vm.shapes);
        match own {
            Some((super::super::super::value::PropertyValue::Data(JsValue::Number(n)), _))
                if n.is_finite() && *n >= 0.0 =>
            {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let id = *n as u64;
                // Symmetric guard with the `<= 2^53` debug_assert at
                // the brand-write site (define.rs ~143). If a future
                // change wraps `constructor_id` past the f64 mantissa
                // boundary the reverse-cast silently truncates; this
                // assert surfaces the drift in debug builds before it
                // can alias to a different CE definition.
                #[allow(clippy::cast_precision_loss)]
                let id_as_f64 = id as f64;
                debug_assert!(
                    id_as_f64 == *n,
                    "brand round-trip lost precision (n={n}, id={id})"
                );
                id
            }
            _ => {
                return Err(VmError::type_error(
                    "Failed to construct 'HTMLElement': new.target is not a registered custom element",
                ));
            }
        }
    };

    // Resolve the definition name via the engine-indep registry.
    let Some(host) = ctx.host_if_bound() else {
        return Err(VmError::type_error(
            "Failed to construct 'HTMLElement': host is not bound",
        ));
    };
    let definition_name = {
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
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
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
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
            set_wrapper_prototype(ctx.vm, wrapper_id, new_target)?;
            {
                let host = ctx.host();
                let mut registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
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
            set_wrapper_prototype(ctx.vm, wrapper_id, new_target)?;
            Ok(JsValue::Object(wrapper_id))
        }
    }
}

/// Splice `wrapper.[[Prototype]] = ctor.prototype` (\[C1\] step 14 +
/// the sync-construct internal-create's prototype derivation, \[C8\]
/// `Get(NewTarget, "prototype")` semantics).
///
/// Returns `Err` when `ctor.prototype` is not an Object — matches
/// WebIDL's "must be an object" wording for the
/// `CustomElementConstructor` callback type.
pub(crate) fn set_wrapper_prototype(
    vm: &mut VmInner,
    wrapper_id: ObjectId,
    ctor_id: ObjectId,
) -> Result<(), VmError> {
    let proto_key = PropertyKey::String(vm.well_known.prototype);
    let proto_value = vm.get_property_value(ctor_id, proto_key)?;
    let JsValue::Object(proto_id) = proto_value else {
        return Err(VmError::type_error(
            "Custom element constructor.prototype must be an object",
        ));
    };
    vm.get_object_mut(wrapper_id).prototype = Some(proto_id);
    Ok(())
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
