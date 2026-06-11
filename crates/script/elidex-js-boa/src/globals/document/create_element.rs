//! `document.createElement(tagName, options?)` registration —
//! WebIDL option-flattening for `ElementCreationOptions` (DOM §4.5)
//! plus the per-engine custom-element upgrade routing that runs after
//! the engine-indep `createElement` handler attached the
//! `CustomElementState` component. Split out of `document/mod.rs`
//! (1000-line file rule), mirroring the VM host's
//! `custom_elements/creation.rs` split.

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind};

use super::invoke_doc_handler_returning_ref;
use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::require_js_string_arg;

/// WebIDL ARGUMENT CONVERSION of the createElement
/// `(DOMString or ElementCreationOptions)` union — runs before any
/// method step (so conversion TypeErrors precede the step 1
/// `InvalidCharacterError` in the closure below). Returns
/// `(registry_member, is_value)` where `registry_member` is
/// `Some(is_null)` when the member was present.
fn convert_create_element_options(
    options: Option<&JsValue>,
    bridge: &HostBridge,
    ctx: &mut boa_engine::Context,
) -> JsResult<(Option<bool>, Option<String>)> {
    let mut registry_member: Option<bool /* is null */> = None;
    let mut is_value: Option<String> = None;
    match options {
        // Absent / undefined → no options; `null` → the dictionary
        // arm of the union (an empty `ElementCreationOptions`).
        None => {}
        Some(opts_val) if opts_val.is_undefined() || opts_val.is_null() => {}
        Some(opts_val) if opts_val.as_object().is_some() => {
            let opts = opts_val.as_object().expect("guarded by arm");
            // Dictionary conversion gets AND converts each member
            // immediately, in lexicographic order
            // (`customElementRegistry` before `is`), so a registry
            // conversion TypeError fires before the `is` getter is
            // even invoked, and the `is` ToString (user code)
            // completes before any flatten step.
            let reg = opts.get(js_string!("customElementRegistry"), ctx)?;
            if !reg.is_undefined() {
                // The member is a NULLABLE `CustomElementRegistry?`:
                // null passes, the document's registry singleton
                // passes, anything else is a TypeError. Identity is
                // checked against the handle the bridge captured at
                // global registration — NOT the writable
                // `globalThis.customElements` property, which page
                // script can reassign to smuggle a non-registry past
                // the brand check or to orphan the real registry
                // (Codex PR331 R11).
                let is_document_registry = !reg.is_null() && {
                    bridge
                        .custom_elements_object()
                        .zip(reg.as_object())
                        .is_some_and(|(canonical, given)| canonical == given)
                };
                if !reg.is_null() && !is_document_registry {
                    return Err(JsNativeError::typ()
                        .with_message(
                            "Failed to execute 'createElement' on 'Document': \
                             Failed to convert value to 'CustomElementRegistry'.",
                        )
                        .into());
                }
                registry_member = Some(reg.is_null());
            }
            // `is` is a non-nullable DOMString — member absent only
            // when undefined, `{is: null}` ToString-converts to
            // "null".
            let v = opts.get(js_string!("is"), ctx)?;
            if !v.is_undefined() {
                is_value = Some(v.to_string(ctx)?.to_std_string_escaped());
            }
        }
        Some(opts_val) => {
            // DOMString arm — convert for the observable side effects
            // (Symbol → TypeError), discard the result (flatten step
            // 3 is gated on "If options is a dictionary"; web-compat
            // note under `#dom-document-createelement`).
            opts_val.to_string(ctx)?;
        }
    }
    Ok((registry_member, is_value))
}

/// Install `document.createElement` on the document object.
pub(super) fn install_create_element(init: &mut ObjectInitializer<'_>, b: &HostBridge) {
    let b_ce = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let tag = require_js_string_arg(args, 0, "createElement", ctx)?;
                let (registry_member, is_value) =
                    convert_create_element_options(args.get(1), bridge, ctx)?;

                // DOM §4.5 createElement method step 1 — localName
                // validity (InvalidCharacterError) BEFORE the flatten
                // NotSupportedError gates below. The engine-indep
                // handler re-validates (it stays self-contained); this
                // pre-check exists purely for the spec-mandated error
                // ORDER, sharing the handler's predicate.
                if !elidex_dom_api::document::is_valid_element_tag_name(&tag) {
                    return Err(dom_error_to_js_error(DomApiError {
                        kind: DomApiErrorKind::InvalidCharacterError,
                        message: format!("Invalid tag name: {tag}"),
                    })
                    .into());
                }

                // FLATTEN PHASE (DOM §4.5 step 3) — runs on the fully
                // converted dictionary, after the localName check.
                let mut handler_args = vec![ElidexJsValue::String(tag)];
                if let Some(registry_is_null) = registry_member {
                    // Step 3.2.1: a present `customElementRegistry`
                    // member alongside a non-null `is` is a hard
                    // conflict ("exists" = dictionary presence, fires
                    // for null too).
                    if is_value.is_some() {
                        return Err(JsNativeError::typ()
                            .with_message(
                                "NotSupportedError: 'is' and 'customElementRegistry' \
                                 cannot both be provided",
                            )
                            .into());
                    }
                    // Step 3.2.2 + 3.3: an explicit null creates a
                    // null-registry element (never upgraded) — the
                    // handler marks `RegistryAssociation::Null` off
                    // the `Null` positional slot (mutually exclusive
                    // with `is` per the conflict above). A foreign
                    // registry already TypeError'd at conversion
                    // (identity-as-brand, boa exposes one registry).
                    if registry_is_null {
                        handler_args.push(ElidexJsValue::Null);
                    }
                } else if let Some(is_value) = is_value {
                    handler_args.push(ElidexJsValue::String(is_value));
                }

                let result =
                    invoke_doc_handler_returning_ref("createElement", &handler_args, bridge, ctx)?;

                // Per-engine upgrade routing off the `CustomElementState`
                // the engine-indep handler derived (presence read — no
                // name derivation here). The component's
                // `definition_name` is the local name for autonomous
                // custom elements and the *is* value for customized
                // built-ins; the entity's canonical (folded) `TagType`
                // discriminates the two and feeds the extends/local-name
                // lookup.
                if let Ok(entity) = crate::globals::element::extract_entity(&result, ctx) {
                    // Phase 1 (inside `with`): read the routing
                    // decision + run the DOM-side mutations. Phase 2
                    // (outside `with`): the synchronous upgrade —
                    // `run_upgrade_reaction` re-enters `bridge.with`
                    // internally, so it must not run under this
                    // closure's borrow.
                    let defined = bridge.with(|_session, dom| {
                        // Upgrade-routing discrimination: the
                        // definition is keyed by the is value exactly
                        // when it differs from the local name (the
                        // autonomous branch keys on the tag). NB this
                        // is the ROUTING question — the serialization
                        // is-value slot is `CustomElementState::
                        // is_value()` and is independent of it.
                        let (name, local_name) = {
                            let world = dom.world();
                            let Ok(state) =
                                world.get::<&elidex_custom_elements::CustomElementState>(entity)
                            else {
                                return false;
                            };
                            // A null-registry element is outside every
                            // registry — no sync upgrade, no queue
                            // admission (DOM §4.9: definition lookup
                            // in a null registry is always null).
                            if matches!(
                                state.registry,
                                elidex_custom_elements::RegistryAssociation::Null
                            ) {
                                return false;
                            }
                            let local_name = world
                                .get::<&elidex_ecs::TagType>(entity)
                                .map(|t| t.0.clone())
                                .unwrap_or_default();
                            (state.definition_name.clone(), local_name)
                        };
                        let defined = if name == local_name {
                            bridge.is_custom_element_defined(&name)
                        } else {
                            bridge.ce_lookup_by_is(&name, &local_name)
                        };
                        if defined {
                            // DOM §4.9 step 5.1.3.10: defined-at-
                            // creation autonomous elements null the
                            // is value (async-created keep theirs).
                            bridge.ce_clear_is_value_for_sync_autonomous(dom, entity);
                        } else if !bridge.is_custom_element_defined(&name) {
                            // Queue admission (incl. the invalid-name
                            // gate) is owned by the registry's
                            // `queue_for_upgrade`. An already-defined
                            // name that merely failed the local-name
                            // match (e.g. `is: 'x-foo'` on the wrong
                            // base tag) is NOT queued — define() can
                            // never run again for it, so the bucket
                            // would be undrainable (Codex PR331 R15).
                            bridge.queue_for_ce_upgrade(&name, entity);
                        }
                        defined
                    });
                    if defined {
                        // DOM §4.5 invokes *create an element* with
                        // the synchronous custom elements flag set —
                        // the element is constructed before
                        // createElement returns (Codex PR331 R13; VM
                        // parity with `invoke_upgrade`).
                        crate::runtime::ce::run_upgrade_reaction(entity, bridge, ctx);
                    }
                }

                Ok(result)
            },
            b_ce,
        ),
        js_string!("createElement"),
        1,
    );
}
