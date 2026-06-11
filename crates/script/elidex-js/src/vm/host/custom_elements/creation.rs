//! createElement-time custom-element marshalling: WebIDL
//! option-flattening for `ElementCreationOptions.is` and the per-VM
//! upgrade-reaction routing that runs after the engine-indep
//! `createElement` handler attached the `CustomElementState`
//! component. Split out of the (1000+-line) `host/document.rs` so the
//! custom-elements-facing host surface stays in one place.

use crate::vm::value::{JsValue, NativeContext, PropertyKey, StringId, VmError};

const PREFIX: &str = "Failed to execute 'createElement' on 'Document'";

/// The WebIDL-converted (but not yet flattened)
/// `(DOMString or ElementCreationOptions)` argument — output of
/// [`convert_create_element_options`], input to
/// [`flatten_converted_options`]. Split in two because WebIDL
/// argument conversion happens BEFORE the createElement method steps
/// (so conversion TypeErrors precede the step 1 `InvalidCharacterError`
/// localName check), while the flatten algorithm's NotSupportedError
/// gates are step 3 (AFTER step 1).
pub(in crate::vm) struct ConvertedCreateOptions {
    registry: Option<super::RegistryMember>,
    is: Option<StringId>,
}

/// The flattened creation options the binding forwards to the
/// engine-indep `createElement` handler. `is` and `null_registry` are
/// mutually exclusive (flatten step 3.2.1 conflict), so the handler
/// receives them in one positional slot (`String` = is, `Null` =
/// null registry).
pub(in crate::vm) struct FlattenedCreateOptions {
    pub is: Option<StringId>,
    pub null_registry: bool,
}

/// WebIDL ARGUMENT CONVERSION of the createElement `options` union —
/// runs before any method step:
///
/// - absent / `undefined` → no members.
/// - `null` → the dictionary arm of the union (an empty
///   `ElementCreationOptions`) → no members.
/// - object → dictionary conversion: members get AND convert
///   immediately, in lexicographic order (`customElementRegistry`
///   before `is`), so a registry conversion TypeError fires before
///   the `is` getter is invoked, and the `is` ToString (user code)
///   completes here, before any flatten step.
/// - anything else → the DOMString arm: the conversion itself is
///   observable (`Symbol` throws the standard ToString TypeError) but
///   the resulting string is ignored — flatten step 3 is gated on "If
///   options is a dictionary" (web-compat note under
///   `#dom-document-createelement`).
pub(in crate::vm) fn convert_create_element_options(
    ctx: &mut NativeContext<'_>,
    options: Option<&JsValue>,
) -> Result<ConvertedCreateOptions, VmError> {
    const EMPTY: ConvertedCreateOptions = ConvertedCreateOptions {
        registry: None,
        is: None,
    };
    let options_id = match options {
        None | Some(JsValue::Undefined | JsValue::Null) => return Ok(EMPTY),
        Some(JsValue::Object(id)) => *id,
        Some(other) => {
            // DOMString arm — convert for the observable side effects
            // (Symbol → TypeError), discard the result.
            crate::vm::coerce::to_string(ctx.vm, *other)?;
            return Ok(EMPTY);
        }
    };
    let registry_key = PropertyKey::String(ctx.vm.strings.intern("customElementRegistry"));
    let registry_raw = ctx.vm.get_property_value(options_id, registry_key)?;
    // WebIDL dictionary semantics: a member is absent only when the
    // property is undefined. `customElementRegistry` is NULLABLE
    // (`CustomElementRegistry?`), so an explicit null is a present
    // member carrying a null registry.
    let registry = if matches!(registry_raw, JsValue::Undefined) {
        None
    } else {
        Some(super::convert_custom_element_registry_member(
            ctx,
            registry_raw,
            PREFIX,
        )?)
    };
    let is_key = PropertyKey::String(ctx.vm.strings.intern("is"));
    let is_raw = ctx.vm.get_property_value(options_id, is_key)?;
    // `ElementCreationOptions.is` is a plain (non-nullable) DOMString,
    // so an explicit `{is: null}` converts via ToString(null) = "null"
    // — a NON-null is value downstream.
    let is = if matches!(is_raw, JsValue::Undefined) {
        None
    } else {
        Some(crate::vm::coerce::to_string(ctx.vm, is_raw)?)
    };
    Ok(ConvertedCreateOptions { registry, is })
}

/// DOM §4.5 "flatten element creation options" ALGORITHM steps on the
/// fully converted dictionary — method step 3, so the binding must run
/// this AFTER step 1's localName validation (`InvalidCharacterError`
/// beats these NotSupportedErrors; conversion TypeErrors beat both).
/// No validity check on the is value itself (rationale on
/// `CustomElementState::for_created_element`).
pub(in crate::vm) fn flatten_converted_options(
    ctx: &mut NativeContext<'_>,
    converted: ConvertedCreateOptions,
) -> Result<FlattenedCreateOptions, VmError> {
    if let Some(member) = converted.registry {
        // Step 3.2.1: a present `customElementRegistry` member
        // alongside a non-null `is` is a hard conflict —
        // NotSupportedError. "Exists" is dictionary presence, so this
        // fires even when the registry member is null.
        if converted.is.is_some() {
            return Err(VmError::dom_exception(
                ctx.vm.well_known.dom_exc_not_supported_error,
                format!("{PREFIX}: 'is' and 'customElementRegistry' cannot both be provided"),
            ));
        }
        // Step 3.2.2 + 3.3: bind the supplied registry — the
        // document's registry is a no-op, foreign throws, null
        // threads through as a null-registry element (never
        // upgraded).
        super::reject_foreign_registry_member(ctx, &member, PREFIX)?;
        return Ok(FlattenedCreateOptions {
            is: None,
            null_registry: matches!(member, super::RegistryMember::Null),
        });
    }
    Ok(FlattenedCreateOptions {
        is: converted.is,
        null_registry: false,
    })
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
            // A null-registry element (created with
            // `{customElementRegistry: null}`) is outside every
            // registry — no sync upgrade, no pending-queue admission
            // (DOM §4.9: definition lookup in a null registry is
            // always null).
            Ok(state)
                if matches!(
                    state.registry,
                    elidex_custom_elements::RegistryAssociation::Null
                ) =>
            {
                return;
            }
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
