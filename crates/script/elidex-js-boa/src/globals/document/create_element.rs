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

use super::invoke_doc_handler_returning_ref;
use crate::bridge::HostBridge;
use crate::globals::require_js_string_arg;

/// Install `document.createElement` on the document object.
pub(super) fn install_create_element(init: &mut ObjectInitializer<'_>, b: &HostBridge) {
    let b_ce = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let tag = require_js_string_arg(args, 0, "createElement", ctx)?;

                // DOM §4.5 step 3 option-flattening — marshalling
                // only, no validity check; full rationale on
                // `CustomElementState::for_created_element`.
                let mut handler_args = vec![ElidexJsValue::String(tag)];
                if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
                    // CONVERSION PHASE — WebIDL dictionary conversion
                    // gets AND converts each member immediately, in
                    // lexicographic order (`customElementRegistry`
                    // before `is`), before any flatten step runs. So a
                    // registry conversion TypeError fires before the
                    // `is` getter is even invoked, and the `is`
                    // ToString (user code) completes before the
                    // flatten conflict check.
                    let reg = opts.get(js_string!("customElementRegistry"), ctx)?;
                    let registry_member = if reg.is_undefined() {
                        None
                    } else {
                        // The member is a NULLABLE
                        // `CustomElementRegistry?`: null passes, the
                        // document's registry singleton passes,
                        // anything else is a TypeError. Identity is
                        // checked against the handle the bridge
                        // captured at global registration — NOT the
                        // writable `globalThis.customElements`
                        // property, which page script can reassign to
                        // smuggle a non-registry past the brand check
                        // or to orphan the real registry (Codex PR331
                        // R11).
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
                        Some(reg.is_null())
                    };
                    // `is` is a non-nullable DOMString — member absent
                    // only when undefined, `{is: null}` ToString-
                    // converts to "null".
                    let v = opts.get(js_string!("is"), ctx)?;
                    let is_value = if v.is_undefined() {
                        None
                    } else {
                        Some(v.to_string(ctx)?.to_std_string_escaped())
                    };
                    // FLATTEN PHASE (DOM §4.5) — runs on the fully
                    // converted dictionary.
                    if let Some(registry_is_null) = registry_member {
                        // Step 3.2.1: a present `customElementRegistry`
                        // member alongside a non-null `is` is a hard
                        // conflict ("exists" = dictionary presence,
                        // fires for null too).
                        if is_value.is_some() {
                            return Err(JsNativeError::typ()
                                .with_message(
                                    "NotSupportedError: 'is' and 'customElementRegistry' \
                                     cannot both be provided",
                                )
                                .into());
                        }
                        // Step 3.2.2 + 3.3: a null registry creates
                        // elements outside the global registry (never
                        // upgraded) — needs per-element registry
                        // association, deferred to slot
                        // `#11-shadow-scoped-custom-element-registry`;
                        // rejected loudly until then.
                        if registry_is_null {
                            return Err(JsNativeError::typ()
                                .with_message(
                                    "NotSupportedError: a null customElementRegistry \
                                     is not supported",
                                )
                                .into());
                        }
                    } else if let Some(is_value) = is_value {
                        handler_args.push(ElidexJsValue::String(is_value));
                    }
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
                    bridge.with(|_session, dom| {
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
                                return;
                            };
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
                            // Definition exists — enqueue Upgrade.
                            bridge.enqueue_ce_reaction(
                                elidex_custom_elements::CustomElementReaction::Upgrade(entity),
                            );
                        } else {
                            // Queue admission (incl. the invalid-name
                            // gate) is owned by the registry's
                            // `queue_for_upgrade`.
                            bridge.queue_for_ce_upgrade(&name, entity);
                        }
                    });
                }

                Ok(result)
            },
            b_ce,
        ),
        js_string!("createElement"),
        1,
    );
}
