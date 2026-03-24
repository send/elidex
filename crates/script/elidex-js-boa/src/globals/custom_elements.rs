//! `customElements` global object registration (WHATWG HTML 4.13.4).
//!
//! Provides `customElements.define()`, `get()`, `whenDefined()`, and `upgrade()`.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};
use elidex_custom_elements::{is_valid_custom_element_name, CustomElementReaction};

use crate::bridge::HostBridge;
use crate::globals::require_js_string_arg;

/// Register the `customElements` global object on the boa context.
#[allow(clippy::too_many_lines)]
// Sequential property/method registration on a single JS object.
pub fn register_custom_elements_global(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();

    let mut init = ObjectInitializer::new(ctx);

    // customElements.define(name, constructor, options?)
    let b_define = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let name = require_js_string_arg(args, 0, "customElements.define", ctx)?;

                // Validate name.
                if !is_valid_custom_element_name(&name) {
                    return Err(JsNativeError::syntax()
                        .with_message(format!("'{name}' is not a valid custom element name"))
                        .into());
                }

                // Extract constructor (must be a constructable object per WHATWG §4.13.4).
                let constructor = args
                    .get(1)
                    .and_then(JsValue::as_object)
                    .filter(JsObject::is_constructor)
                    .ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("customElements.define: argument 1 must be a constructor")
                    })?
                    .clone();

                // Extract options.extends if present.
                let extends = if let Some(opts) = args.get(2).and_then(JsValue::as_object) {
                    let ext_val = opts.get(js_string!("extends"), ctx)?;
                    if ext_val.is_undefined() || ext_val.is_null() {
                        None
                    } else {
                        Some(ext_val.to_string(ctx)?.to_std_string_escaped())
                    }
                } else {
                    None
                };

                // Extract observedAttributes from constructor (static getter).
                let observed_attrs = extract_observed_attributes(&constructor, ctx)?;

                // Register in bridge.
                let pending = bridge
                    .register_custom_element(&name, constructor, observed_attrs, extends)
                    .map_err(|msg| JsNativeError::syntax().with_message(msg))?;

                // Enqueue Upgrade reactions for pending elements (queued via
                // bridge.queue_for_ce_upgrade before define() was called).
                for entity in pending {
                    bridge.enqueue_ce_reaction(CustomElementReaction::Upgrade(entity));
                }

                // Walk the DOM to find parser-created elements with
                // CustomElementState::Undefined that weren't queued via the
                // pending queue (the HTML parser marks them Undefined but
                // cannot queue them because the registry lives in HostBridge).
                //
                // Performance note: O(document size) walk per define(). Acceptable
                // for typical pages (<100 define calls, <10K elements). For extreme
                // cases, consider maintaining a pending-by-name index on the DOM.
                let doc = bridge.document_entity();
                bridge.with(|_session, dom| {
                    walk_and_enqueue_upgrades(doc, bridge, dom);
                });

                // Resolve any pending whenDefined() promises for this name.
                let resolvers = bridge.take_when_defined_resolvers(&name);
                if !resolvers.is_empty() {
                    let ctor_val = bridge
                        .get_custom_element_constructor(&name)
                        .map_or(JsValue::undefined(), JsValue::from);
                    for resolver in resolvers {
                        let resolve_obj: JsObject = resolver.into();
                        let _ = resolve_obj.call(
                            &JsValue::undefined(),
                            std::slice::from_ref(&ctor_val),
                            ctx,
                        );
                    }
                }

                Ok(JsValue::undefined())
            },
            b_define,
        ),
        js_string!("define"),
        2,
    );

    // customElements.get(name)
    let b_get = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let name = require_js_string_arg(args, 0, "customElements.get", ctx)?;
                Ok(bridge
                    .get_custom_element_constructor(&name)
                    .map_or(JsValue::undefined(), JsValue::from))
            },
            b_get,
        ),
        js_string!("get"),
        1,
    );

    // customElements.whenDefined(name) — returns pending Promise if not yet
    // defined, resolved with constructor when define() is called (WHATWG §4.13.4).
    let b_when = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let name = require_js_string_arg(args, 0, "customElements.whenDefined", ctx)?;

                if !is_valid_custom_element_name(&name) {
                    return Err(JsNativeError::syntax()
                        .with_message(format!("'{name}' is not a valid custom element name"))
                        .into());
                }

                if bridge.is_custom_element_defined(&name) {
                    // Return a resolved promise with the constructor.
                    let ctor = bridge
                        .get_custom_element_constructor(&name)
                        .map_or(JsValue::undefined(), JsValue::from);
                    let promise = JsPromise::resolve(ctor, ctx);
                    Ok(promise.into())
                } else {
                    // Create a pending promise; store the resolve function.
                    // When define() is called for this name, it will resolve.
                    let (promise, resolvers) = JsPromise::new_pending(ctx);
                    bridge.store_when_defined_resolver(&name, resolvers.resolve);
                    Ok(promise.into())
                }
            },
            b_when,
        ),
        js_string!("whenDefined"),
        1,
    );

    // customElements.upgrade(root) — walk subtree and enqueue Upgrade reactions
    let b_upgrade = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let root = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("customElements.upgrade: root argument required")
                    })?,
                    ctx,
                )?;

                bridge.with(|_session, dom| {
                    walk_and_enqueue_upgrades(root, bridge, dom);
                });

                Ok(JsValue::undefined())
            },
            b_upgrade,
        ),
        js_string!("upgrade"),
        1,
    );

    let ce_obj = init.build();
    ctx.register_global_property(
        js_string!("customElements"),
        ce_obj,
        boa_engine::property::Attribute::all(),
    )
    .expect("failed to register customElements");
}

/// Extract the static `observedAttributes` from a constructor function.
///
/// Reads `Constructor.observedAttributes` and expects an Array of strings.
/// Returns an empty vec if the property is missing or not an array.
fn extract_observed_attributes(
    constructor: &boa_engine::JsObject,
    ctx: &mut Context,
) -> JsResult<Vec<String>> {
    const MAX_OBSERVED_ATTRIBUTES: usize = 1000;

    let val = constructor.get(js_string!("observedAttributes"), ctx)?;
    if val.is_undefined() || val.is_null() {
        return Ok(Vec::new());
    }
    let obj = val.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("observedAttributes must be an array-like object")
    })?;
    let arr = JsArray::from_object(obj.clone())
        .map_err(|_| JsNativeError::typ().with_message("observedAttributes must be an array"))?;
    let len = (arr.length(ctx)? as usize).min(MAX_OBSERVED_ATTRIBUTES);
    let mut attrs = Vec::with_capacity(len);
    for i in 0..len {
        #[allow(clippy::cast_precision_loss)]
        let item = arr.get(i as u32, ctx)?;
        let s = item.to_string(ctx)?;
        attrs.push(s.to_std_string_escaped().to_ascii_lowercase());
    }
    Ok(attrs)
}

/// Walk a subtree and enqueue Upgrade reactions for undefined custom elements.
fn walk_and_enqueue_upgrades(
    root: elidex_ecs::Entity,
    bridge: &HostBridge,
    dom: &elidex_ecs::EcsDom,
) {
    walk_and_enqueue_upgrades_inner(root, bridge, dom, 0);
}

fn walk_and_enqueue_upgrades_inner(
    root: elidex_ecs::Entity,
    bridge: &HostBridge,
    dom: &elidex_ecs::EcsDom,
    depth: usize,
) {
    use elidex_custom_elements::{CEState, CustomElementState};
    if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
        return;
    }

    if let Ok(ce_state) = dom.world().get::<&CustomElementState>(root) {
        if ce_state.state == CEState::Undefined {
            // Check definition exists and, for customized built-ins, that the
            // extends tag matches the element's actual tag.
            let should_upgrade = bridge.with_ce_definition(&ce_state.definition_name, |def| {
                def.extends.as_ref().is_none_or(|ext| {
                    dom.world()
                        .get::<&elidex_ecs::TagType>(root)
                        .ok()
                        .is_some_and(|tag| tag.0.eq_ignore_ascii_case(ext))
                })
            });
            if should_upgrade {
                bridge.enqueue_ce_reaction(CustomElementReaction::Upgrade(root));
            }
        }
    }

    let mut child = dom.get_first_child(root);
    while let Some(c) = child {
        walk_and_enqueue_upgrades_inner(c, bridge, dom, depth + 1);
        child = dom.get_next_sibling(c);
    }
}
