//! createElement-time custom-element marshalling: WebIDL
//! option-flattening for `ElementCreationOptions.is` and the per-VM
//! upgrade-reaction routing that runs after the engine-indep
//! `createElement` handler attached the `CustomElementState`
//! component. Split out of the (1000+-line) `host/document.rs` so the
//! custom-elements-facing host surface stays in one place.

use crate::vm::value::{JsValue, NativeContext, PropertyKey, StringId, VmError};

/// DOM §4.5 createElement step 3 "flatten element creation options" —
/// pure marshalling: extract `options.is` from the dictionary arm and
/// ToString it. NO validity check (rationale on
/// `CustomElementState::for_created_element`).
///
/// The DOMString arm of the `(DOMString or ElementCreationOptions)`
/// union is deliberately NOT treated as an is value: flatten step 3 is
/// gated on "If options is a dictionary", so a string `options` is
/// accepted for web compatibility and ignored (is stays null — spec
/// note under `#dom-document-createelement`).
pub(in crate::vm) fn flatten_is_option(
    ctx: &mut NativeContext<'_>,
    options: Option<&JsValue>,
) -> Result<Option<StringId>, VmError> {
    let Some(JsValue::Object(options_id)) = options else {
        return Ok(None);
    };
    let is_key = PropertyKey::String(ctx.vm.strings.intern("is"));
    let raw = ctx.vm.get_property_value(*options_id, is_key)?;
    // WebIDL dictionary semantics: the member is absent only when the
    // property is undefined. `ElementCreationOptions.is` is a plain
    // (non-nullable) DOMString, so an explicit `{is: null}` converts
    // via ToString(null) = "null" — a NON-null is value downstream.
    if matches!(raw, JsValue::Undefined) {
        return Ok(None);
    }
    // Flatten step 3.2.1: a dictionary carrying BOTH a non-null `is`
    // and a `customElementRegistry` member is a hard conflict —
    // NotSupportedError. (Scoped-registry support itself is deferred,
    // slot `#11-shadow-scoped-custom-element-registry`; the conflict
    // check is part of the flatten algorithm regardless.)
    let registry_key = PropertyKey::String(ctx.vm.strings.intern("customElementRegistry"));
    let registry_member = ctx.vm.get_property_value(*options_id, registry_key)?;
    if !matches!(registry_member, JsValue::Undefined) {
        let not_supported = ctx.vm.well_known.dom_exc_not_supported_error;
        return Err(VmError::dom_exception(
            not_supported,
            "Failed to execute 'createElement' on 'Document': \
             'is' and 'customElementRegistry' cannot both be provided"
                .to_string(),
        ));
    }
    Ok(Some(crate::vm::coerce::to_string(ctx.vm, raw)?))
}

/// Per-VM upgrade-reaction routing for a freshly created element —
/// pure marshalling off the `CustomElementState` component the
/// engine-indep `createElement` handler attached (presence read; no
/// name derivation happens here, that is
/// `CustomElementState::for_created_element`'s job in
/// elidex-custom-elements).  No-op when the handler attached nothing
/// (ordinary built-in without `is`).
///
/// Routing per DOM §4.9 "create an element" (synchronous custom
/// elements flag set for `createElement`):
/// - definition already registered → SYNCHRONOUS upgrade (the
///   createElement call returns the element in `Custom` state so
///   subsequent appendChild / setAttribute observe the post-upgrade
///   reactions). Errors during the synchronous upgrade are eprinted
///   (matching the reaction-flush path) — createElement still returns
///   the wrapper in `Failed` state.
/// - otherwise → pending-upgrade queue, drained by the next
///   `customElements.define('<name>', ...)`.
///
/// The queue / lookup key is the component's `definition_name` — the
/// local name for autonomous custom elements, the *is* value for
/// customized built-ins (extends/local-name matching happens inside
/// the engine-indep upgrade machinery, `prepare_upgrade` +
/// `upgrade_matches_local_name`).
pub(in crate::vm) fn route_custom_element_upgrade(
    ctx: &mut NativeContext<'_>,
    entity: elidex_ecs::Entity,
) {
    let name = {
        let host = ctx.host();
        let dom = host.dom();
        match dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
        {
            Ok(state) => state.definition_name.clone(),
            Err(_) => return,
        }
    };
    let is_defined = ctx
        .host()
        .ce_registry
        .lock()
        .expect("CE registry mutex poisoned")
        .is_defined(&name);
    if is_defined {
        // DOM §4.9 step 5.1.3.10: the sync autonomous branch nulls the
        // is value (async-created elements keep theirs through the
        // later define()-walk upgrade).
        {
            // Phased (read → decide → clear) because the registry
            // mutex guard and the mutable DOM borrow cannot overlap;
            // the decision itself is the engine-indep
            // `sync_autonomous_definition_matches`.
            let host = ctx.host();
            let names = {
                let dom = host.dom();
                let local = dom
                    .world()
                    .get::<&elidex_ecs::TagType>(entity)
                    .ok()
                    .map(|t| t.0.clone());
                let def = dom
                    .world()
                    .get::<&elidex_custom_elements::CustomElementState>(entity)
                    .ok()
                    .map(|s| s.definition_name.clone());
                local.zip(def)
            };
            if let Some((local_name, def_name)) = names {
                let matches = {
                    let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
                    elidex_custom_elements::sync_autonomous_definition_matches(
                        &registry,
                        &def_name,
                        &local_name,
                    )
                };
                if matches {
                    if let Ok(mut state) =
                        host.dom()
                            .world_mut()
                            .get::<&mut elidex_custom_elements::CustomElementState>(entity)
                    {
                        state.is_value = None;
                    }
                }
            }
        }
        if let Err(err) = super::upgrade::invoke_upgrade(ctx, entity) {
            eprintln!("[CE Upgrade Error] {}", err.message);
        }
    } else {
        // Queue admission (incl. the invalid-name gate) is owned by
        // `CustomElementRegistry::queue_for_upgrade` itself.
        ctx.host()
            .ce_registry
            .lock()
            .expect("CE registry mutex poisoned")
            .queue_for_upgrade(&name, entity);
    }
}
