//! Element wrapper objects for boa — provides DOM methods on element instances.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::{ComponentKind, JsObjectRef};

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::{
    boa_arg_to_elidex, boa_args_to_elidex, invoke_dom_handler, invoke_dom_handler_ref,
    invoke_dom_handler_void, require_js_string_arg,
};
use crate::value_conv;

/// Hidden property key storing the entity bits on element wrapper objects.
pub(crate) const ENTITY_KEY: &str = "__elidex_entity__";

/// Hidden property key for caching the `style` object on an element wrapper.
const STYLE_CACHE_KEY: &str = "__elidex_style__";

/// Hidden property key for caching the `classList` object on an element wrapper.
const CLASSLIST_CACHE_KEY: &str = "__elidex_classList__";

/// Hidden property key for caching the context2d object on a canvas element.
const CONTEXT2D_CACHE_KEY: &str = "__elidex_ctx2d__";

/// Hidden property key for caching the `dataset` object on an element wrapper.
const DATASET_CACHE_KEY: &str = "__elidex_dataset__";

/// Extract the entity from a JS value that has an `__elidex_entity__` property.
///
/// Used for both `this` (element methods) and argument values (e.g., child nodes).
/// Validates that the stored value is a finite non-negative number before casting.
pub(crate) fn extract_entity(value: &JsValue, ctx: &mut Context) -> JsResult<Entity> {
    let obj = value
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("expected an element object"))?;
    let val = obj.get(js_string!(ENTITY_KEY), ctx)?;
    if val.is_undefined() {
        return Err(JsNativeError::typ()
            .with_message("object is not an element (missing entity reference)")
            .into());
    }
    let n = val.to_number(ctx)?;
    if !n.is_finite() || n < 0.0 {
        return Err(JsNativeError::typ()
            .with_message("invalid entity reference (non-finite or negative)")
            .into());
    }
    let bits = n as u64;
    Entity::from_bits(bits).ok_or_else(|| {
        JsNativeError::typ()
            .with_message("invalid entity reference")
            .into()
    })
}

/// Extract entity bits as f64 for storage in hidden properties.
fn entity_bits_as_f64(entity: Entity) -> f64 {
    entity.to_bits().get() as f64
}

/// Create a boa element wrapper object for the given entity.
///
/// The object has DOM methods (appendChild, setAttribute, etc.) and an
/// internal `__elidex_entity__` property for identity tracking.
///
/// Uses the bridge's JS object cache for identity preservation.
pub fn create_element_wrapper(
    entity: Entity,
    bridge: &HostBridge,
    session_entity_ref: JsObjectRef,
    ctx: &mut Context,
) -> JsValue {
    // Check cache first.
    if let Some(cached) = bridge.get_cached_js_object(session_entity_ref) {
        return cached.into();
    }

    let b = bridge.clone();
    let obj = build_element_object(entity, &b, ctx);

    bridge.cache_js_object(session_entity_ref, obj.clone());
    obj.into()
}

fn build_element_object(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> boa_engine::JsObject {
    let mut init = ObjectInitializer::new(ctx);

    // Store entity reference.
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits_as_f64(entity)),
        Attribute::empty(),
    );

    register_child_mutation_methods(&mut init, bridge);
    register_attribute_methods(&mut init, bridge);

    let realm = init.context().realm().clone();

    register_content_accessors(&mut init, bridge, &realm);
    register_style_accessor(&mut init, bridge, &realm);
    register_class_list_accessor(&mut init, bridge, &realm);
    register_event_listener_methods(&mut init, bridge);
    register_shadow_dom_methods(&mut init, bridge, &realm);
    register_canvas_method(&mut init, bridge);
    super::element_form::register_form_accessors(&mut init, bridge, &realm);
    register_tree_nav_accessors(&mut init, bridge, &realm);
    register_node_info_accessors(&mut init, bridge, &realm);
    register_node_methods(&mut init, bridge);
    register_child_parent_mixin_methods(&mut init, bridge);
    register_element_extra_methods(&mut init, bridge);
    register_element_extra_accessors(&mut init, bridge, &realm);
    register_dataset_accessor(&mut init, bridge, &realm);
    register_char_data_methods(&mut init, bridge, &realm);
    register_attr_node_methods(&mut init, bridge);

    init.build()
}

// ---------------------------------------------------------------------------
// Sub-functions for build_element_object
// ---------------------------------------------------------------------------

/// Register appendChild and removeChild methods.
///
/// Both share the same pattern: extract parent from `this`, extract child
/// from first arg, invoke a `DomApiHandler` via bridge, return the child value.
fn register_child_mutation_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // appendChild(child)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| dom_child_operation(this, args, bridge, ctx, "appendChild"),
            b,
        ),
        js_string!("appendChild"),
        1,
    );

    // removeChild(child)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| dom_child_operation(this, args, bridge, ctx, "removeChild"),
            b,
        ),
        js_string!("removeChild"),
        1,
    );
}

/// Shared implementation for child mutation methods (appendChild, removeChild).
///
/// Extracts parent from `this`, child from first arg, invokes the handler by
/// name via the registry, and returns the child JS value.
fn dom_child_operation(
    this: &JsValue,
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
    handler_name: &str,
) -> JsResult<JsValue> {
    let parent = extract_entity(this, ctx)?;
    let child_val = args.first().ok_or_else(|| {
        JsNativeError::typ().with_message(format!("{handler_name} requires a node argument"))
    })?;
    let child_entity = extract_entity(child_val, ctx)?;
    let handler = bridge.dom_registry().resolve(handler_name).ok_or_else(|| {
        JsNativeError::typ().with_message(format!("Unknown DOM method: {handler_name}"))
    })?;
    bridge.with(|session, dom| {
        let child_ref = session.get_or_create_wrapper(child_entity, ComponentKind::Element);
        handler
            .invoke(
                parent,
                &[ElidexJsValue::ObjectRef(child_ref.to_raw())],
                session,
                dom,
            )
            .map_err(dom_error_to_js_error)?;
        Ok(child_val.clone())
    })
}

/// Register setAttribute, getAttribute, and removeAttribute methods.
fn register_attribute_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // setAttribute(name, value)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "setAttribute", ctx)?;
                let value = require_js_string_arg(args, 1, "setAttribute", ctx)?;
                invoke_dom_handler_void(
                    "setAttribute",
                    entity,
                    &[ElidexJsValue::String(name), ElidexJsValue::String(value)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("setAttribute"),
        2,
    );

    // getAttribute(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "getAttribute", ctx)?;
                invoke_dom_handler(
                    "getAttribute",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("getAttribute"),
        1,
    );

    // removeAttribute(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "removeAttribute", ctx)?;
                invoke_dom_handler_void(
                    "removeAttribute",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("removeAttribute"),
        1,
    );
}

/// Register textContent (getter/setter) and innerHTML (getter) accessors.
fn register_content_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // textContent getter
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("textContent.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    // textContent setter
    let b = bridge.clone();
    let setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            // textContent setter accepts undefined/missing as "" per spec.
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "textContent.set",
                entity,
                &[ElidexJsValue::String(text)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("textContent"),
        Some(getter),
        Some(setter),
        Attribute::CONFIGURABLE,
    );

    // innerHTML getter (read-only for Phase 2)
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("innerHTML.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("innerHTML"),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

/// Register the `style` cached accessor.
fn register_style_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_cached_accessor(
        init,
        realm,
        bridge,
        "style",
        STYLE_CACHE_KEY,
        crate::globals::window::create_style_object,
    );
}

/// Register the `classList` cached accessor.
fn register_class_list_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_cached_accessor(
        init,
        realm,
        bridge,
        "classList",
        CLASSLIST_CACHE_KEY,
        create_class_list_object,
    );
}

/// Register addEventListener and removeEventListener methods.
fn register_event_listener_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // addEventListener(type, listener, capture?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                crate::globals::add_event_listener_for(entity, args, bridge, ctx)
            },
            b,
        ),
        js_string!("addEventListener"),
        2,
    );

    // removeEventListener(type, listener, capture?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                crate::globals::remove_event_listener_for(entity, args, bridge, ctx)
            },
            b,
        ),
        js_string!("removeEventListener"),
        2,
    );
}

/// Register attachShadow method and shadowRoot accessor.
fn register_shadow_dom_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // attachShadow({ mode: "open" | "closed" })
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;

                // Parse init dict: { mode: "open" | "closed" }
                let init_obj = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("attachShadow requires an init dict")
                })?;
                let init_obj = init_obj.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("attachShadow argument must be an object")
                })?;
                let mode_val = init_obj.get(js_string!("mode"), ctx)?;
                let mode_str = mode_val.to_string(ctx)?.to_std_string_escaped();
                let mode = match mode_str.as_str() {
                    "open" => elidex_ecs::ShadowRootMode::Open,
                    "closed" => elidex_ecs::ShadowRootMode::Closed,
                    _ => {
                        return Err(JsNativeError::typ()
                            .with_message("mode must be 'open' or 'closed'")
                            .into())
                    }
                };

                let (sr_entity, sr_ref) = bridge.with(|session, dom| -> JsResult<_> {
                    // WHATWG DOM §4.2.14: should throw NotSupportedError (DOMException).
                    // Boa 0.21 lacks DOMException / WebIDL exception support, so we
                    // use TypeError with the DOMException name prefix. JS code can
                    // detect the error type via `e.message.startsWith("NotSupportedError")`.
                    dom.attach_shadow(entity, mode).map_err(|()| {
                        JsNativeError::typ()
                            .with_message("NotSupportedError: Failed to execute 'attachShadow' on 'Element': This element does not support attachShadow")
                    })?;
                    let sr = dom.get_shadow_root(entity).ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("Shadow root not found after attachShadow")
                    })?;
                    let sr_ref = session.get_or_create_wrapper(sr, ComponentKind::Element);
                    Ok((sr, sr_ref))
                })?;
                Ok(create_element_wrapper(sr_entity, bridge, sr_ref, ctx))
            },
            b,
        ),
        js_string!("attachShadow"),
        1,
    );

    // shadowRoot accessor (read-only)
    let b = bridge.clone();
    let sr_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|session, dom| {
                let Some(sr) = dom.get_shadow_root(entity) else {
                    return Ok(JsValue::null());
                };
                // Check mode — closed mode returns null.
                let mode = dom
                    .world()
                    .get::<&elidex_ecs::ShadowRoot>(sr)
                    .ok()
                    .map(|s| s.mode);
                if mode != Some(elidex_ecs::ShadowRootMode::Open) {
                    return Ok(JsValue::null());
                }
                let sr_ref = session.get_or_create_wrapper(sr, ComponentKind::Element);
                Ok(create_element_wrapper(sr, bridge, sr_ref, ctx))
            })
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("shadowRoot"),
        Some(sr_getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

/// Register getContext method for canvas elements.
fn register_canvas_method(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;

                // getContext is only valid on <canvas> elements.
                let is_canvas = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::TagType>(entity)
                        .is_ok_and(|t| t.0.as_str() == "canvas")
                });
                if !is_canvas {
                    return Ok(JsValue::null());
                }

                let context_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();

                if context_type != "2d" {
                    return Ok(JsValue::null());
                }

                // Check for cached context2d object.
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("expected an element object")
                })?;
                let cached = obj.get(js_string!(CONTEXT2D_CACHE_KEY), ctx)?;
                if !cached.is_undefined() {
                    return Ok(cached);
                }

                // Determine canvas dimensions from width/height attributes.
                let (width, height) = bridge.with(|_session, dom| {
                    let w = dom
                        .world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|a| a.get("width").and_then(|v| v.parse::<u32>().ok()))
                        .unwrap_or(elidex_web_canvas::DEFAULT_WIDTH);
                    let h = dom
                        .world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|a| a.get("height").and_then(|v| v.parse::<u32>().ok()))
                        .unwrap_or(elidex_web_canvas::DEFAULT_HEIGHT);
                    (w, h)
                });

                let bits = entity.to_bits().get();
                if !bridge.ensure_canvas_context(bits, width, height) {
                    // Pixmap allocation failed (dimensions too large).
                    return Ok(JsValue::null());
                }

                let ctx2d =
                    crate::globals::canvas::create_context2d_object(bits, this, bridge, ctx);
                // Cache on the element for identity preservation.
                obj.set(js_string!(CONTEXT2D_CACHE_KEY), ctx2d.clone(), false, ctx)?;
                Ok(ctx2d)
            },
            b,
        ),
        js_string!("getContext"),
        1,
    );
}

// ---------------------------------------------------------------------------
// Helper for read-only ref-returning accessors (tree navigation)
// ---------------------------------------------------------------------------

/// Register a read-only accessor that returns an element ref (or null) via `invoke_dom_handler_ref`.
fn reg_ref_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler_ref(handler, entity, &[], bridge, ctx)
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!(js_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

/// Register a read-only accessor that returns a value (string/number/bool) via `invoke_dom_handler`.
fn reg_val_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler(handler, entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!(js_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

// ---------------------------------------------------------------------------
// Tree navigation accessors
// ---------------------------------------------------------------------------

/// Register 10 read-only tree navigation accessors (parentNode, firstChild, etc.).
fn register_tree_nav_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    static NAV_PROPS: &[(&str, &str)] = &[
        ("parentNode", "parentNode.get"),
        ("parentElement", "parentElement.get"),
        ("firstChild", "firstChild.get"),
        ("lastChild", "lastChild.get"),
        ("nextSibling", "nextSibling.get"),
        ("previousSibling", "previousSibling.get"),
        ("firstElementChild", "firstElementChild.get"),
        ("lastElementChild", "lastElementChild.get"),
        ("nextElementSibling", "nextElementSibling.get"),
        ("previousElementSibling", "previousElementSibling.get"),
    ];
    for &(js_name, handler) in NAV_PROPS {
        reg_ref_accessor(init, bridge, realm, js_name, handler);
    }
}

// ---------------------------------------------------------------------------
// Node info accessors
// ---------------------------------------------------------------------------

/// Register node info accessors (tagName, nodeName, nodeType, etc.) and hasChildNodes method.
#[allow(clippy::similar_names)] // Getter/setter pairs (e.g., tag_getter/tag_setter) intentionally similar
fn register_node_info_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // Read-only value accessors.
    reg_val_accessor(init, bridge, realm, "tagName", "tagName.get");
    reg_val_accessor(init, bridge, realm, "nodeName", "nodeName.get");
    reg_val_accessor(init, bridge, realm, "nodeType", "nodeType.get");
    reg_val_accessor(
        init,
        bridge,
        realm,
        "childElementCount",
        "childElementCount.get",
    );
    reg_val_accessor(init, bridge, realm, "isConnected", "isConnected.get");

    // ownerDocument — returns ref.
    reg_ref_accessor(init, bridge, realm, "ownerDocument", "ownerDocument.get");

    // nodeValue getter/setter.
    let b = bridge.clone();
    let nv_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("nodeValue.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let nv_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "nodeValue.set",
                entity,
                &[ElidexJsValue::String(text)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("nodeValue"),
        Some(nv_getter),
        Some(nv_setter),
        Attribute::CONFIGURABLE,
    );

    // hasChildNodes() method.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler("hasChildNodes", entity, &[], bridge)
            },
            b,
        ),
        js_string!("hasChildNodes"),
        0,
    );
}

// ---------------------------------------------------------------------------
// Node methods (contains, compareDocumentPosition, cloneNode, etc.)
// ---------------------------------------------------------------------------

/// Register node methods that take entity arguments.
#[allow(clippy::too_many_lines)] // Registration boilerplate; splitting would not improve clarity
fn register_node_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // contains(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("contains", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("contains"),
        1,
    );

    // compareDocumentPosition(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("compareDocumentPosition", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("compareDocumentPosition"),
        1,
    );

    // cloneNode(deep?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let deep = args.first().is_some_and(JsValue::to_boolean);
                invoke_dom_handler_ref(
                    "cloneNode",
                    entity,
                    &[ElidexJsValue::Bool(deep)],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("cloneNode"),
        1,
    );

    // normalize()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_void("normalize", entity, &[], bridge)
            },
            b,
        ),
        js_string!("normalize"),
        0,
    );

    // getRootNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_ref("getRootNode", entity, &[], bridge, ctx)
            },
            b,
        ),
        js_string!("getRootNode"),
        0,
    );

    // isSameNode(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("isSameNode", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("isSameNode"),
        1,
    );

    // isEqualNode(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("isEqualNode", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("isEqualNode"),
        1,
    );
}

// ---------------------------------------------------------------------------
// ChildNode / ParentNode mixin methods
// ---------------------------------------------------------------------------

/// Register variadic ChildNode/ParentNode mixin methods (before, after, remove, etc.).
fn register_child_parent_mixin_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // Variadic methods: before, after, replaceWith, prepend, append, replaceChildren.
    static VARIADIC_METHODS: &[&str] = &[
        "before",
        "after",
        "replaceWith",
        "prepend",
        "append",
        "replaceChildren",
    ];
    for &method_name in VARIADIC_METHODS {
        let b = bridge.clone();
        init.function(
            NativeFunction::from_copy_closure_with_captures(
                move |this, args, bridge, ctx| {
                    let entity = extract_entity(this, ctx)?;
                    let elidex_args = boa_args_to_elidex(args, bridge, ctx)?;
                    invoke_dom_handler_void(method_name, entity, &elidex_args, bridge)
                },
                b,
            ),
            js_string!(method_name),
            0,
        );
    }

    // remove() — no args.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_void("remove", entity, &[], bridge)
            },
            b,
        ),
        js_string!("remove"),
        0,
    );
}

// ---------------------------------------------------------------------------
// Element extra methods
// ---------------------------------------------------------------------------

/// Register additional Element methods (matches, closest, insertAdjacent*, etc.).
#[allow(clippy::too_many_lines)] // Registration boilerplate; splitting would not improve clarity
fn register_element_extra_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // matches(selector)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let sel = require_js_string_arg(args, 0, "matches", ctx)?;
                invoke_dom_handler("matches", entity, &[ElidexJsValue::String(sel)], bridge)
            },
            b,
        ),
        js_string!("matches"),
        1,
    );

    // closest(selector)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let sel = require_js_string_arg(args, 0, "closest", ctx)?;
                invoke_dom_handler_ref(
                    "closest",
                    entity,
                    &[ElidexJsValue::String(sel)],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("closest"),
        1,
    );

    // insertAdjacentElement(position, element)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let position = require_js_string_arg(args, 0, "insertAdjacentElement", ctx)?;
                let elem = boa_arg_to_elidex(args.get(1).unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler_ref(
                    "insertAdjacentElement",
                    entity,
                    &[ElidexJsValue::String(position), elem],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("insertAdjacentElement"),
        2,
    );

    // insertAdjacentText(position, text)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let position = require_js_string_arg(args, 0, "insertAdjacentText", ctx)?;
                let text = require_js_string_arg(args, 1, "insertAdjacentText", ctx)?;
                invoke_dom_handler_void(
                    "insertAdjacentText",
                    entity,
                    &[ElidexJsValue::String(position), ElidexJsValue::String(text)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("insertAdjacentText"),
        2,
    );

    // hasAttribute(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "hasAttribute", ctx)?;
                invoke_dom_handler(
                    "hasAttribute",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("hasAttribute"),
        1,
    );

    // toggleAttribute(name, force?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "toggleAttribute", ctx)?;
                let mut elidex_args = vec![ElidexJsValue::String(name)];
                if let Some(v) = args.get(1) {
                    elidex_args.push(ElidexJsValue::Bool(v.to_boolean()));
                }
                invoke_dom_handler("toggleAttribute", entity, &elidex_args, bridge)
            },
            b,
        ),
        js_string!("toggleAttribute"),
        1,
    );

    // getAttributeNames()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let result = invoke_dom_handler("getAttributeNames", entity, &[], bridge)?;
                let s = result.to_string(ctx)?.to_std_string_escaped();
                let array = JsArray::new(ctx);
                if !s.is_empty() {
                    for name in s.split('\0') {
                        array.push(JsValue::from(js_string!(name)), ctx)?;
                    }
                }
                Ok(array.into())
            },
            b,
        ),
        js_string!("getAttributeNames"),
        0,
    );
}

// ---------------------------------------------------------------------------
// Element extra accessors (className, id)
// ---------------------------------------------------------------------------

/// Register className and id getter/setter accessors.
#[allow(clippy::similar_names)] // Getter/setter pairs (e.g., cls_getter/cls_setter) intentionally similar
fn register_element_extra_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // className getter/setter.
    let b = bridge.clone();
    let cn_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("className.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let cn_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "className.set",
                entity,
                &[ElidexJsValue::String(val)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("className"),
        Some(cn_getter),
        Some(cn_setter),
        Attribute::CONFIGURABLE,
    );

    // id getter/setter.
    let b = bridge.clone();
    let id_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("id.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let id_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void("id.set", entity, &[ElidexJsValue::String(val)], bridge)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("id"),
        Some(id_getter),
        Some(id_setter),
        Attribute::CONFIGURABLE,
    );
}

// ---------------------------------------------------------------------------
// Dataset accessor
// ---------------------------------------------------------------------------

/// Register the `dataset` cached accessor.
fn register_dataset_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_cached_accessor(
        init,
        realm,
        bridge,
        "dataset",
        DATASET_CACHE_KEY,
        create_dataset_object,
    );
}

/// Create a dataset proxy object with get/set/delete methods.
fn create_dataset_object(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);
    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    // get(key)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let key = require_js_string_arg(args, 0, "dataset.get", ctx)?;
                invoke_dom_handler("dataset.get", entity, &[ElidexJsValue::String(key)], bridge)
            },
            b,
        ),
        js_string!("get"),
        1,
    );

    // set(key, value)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let key = require_js_string_arg(args, 0, "dataset.set", ctx)?;
                let value = require_js_string_arg(args, 1, "dataset.set", ctx)?;
                invoke_dom_handler_void(
                    "dataset.set",
                    entity,
                    &[ElidexJsValue::String(key), ElidexJsValue::String(value)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("set"),
        2,
    );

    // delete(key)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let key = require_js_string_arg(args, 0, "dataset.delete", ctx)?;
                invoke_dom_handler_void(
                    "dataset.delete",
                    entity,
                    &[ElidexJsValue::String(key)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("delete"),
        1,
    );

    init.build().into()
}

/// Register a cached read-only accessor (style, classList) on an element object.
///
/// The accessor returns a cached sub-object on subsequent accesses (identity
/// preservation: `el.style === el.style`). The `create_fn` builds the object
/// on first access, and it's stored under `cache_key` on the element wrapper.
fn register_cached_accessor(
    init: &mut ObjectInitializer<'_>,
    realm: &boa_engine::realm::Realm,
    bridge: &HostBridge,
    prop_name: &str,
    cache_key: &'static str,
    create_fn: fn(Entity, &HostBridge, &mut Context) -> JsValue,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("expected an element object"))?;
            // Return cached object if available.
            let cached = obj.get(js_string!(cache_key), ctx)?;
            if !cached.is_undefined() {
                return Ok(cached);
            }
            let entity = extract_entity(this, ctx)?;
            let val = create_fn(entity, bridge, ctx);
            // Cache on the element for identity preservation.
            obj.set(js_string!(cache_key), val.clone(), false, ctx)?;
            Ok(val)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(prop_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

#[allow(clippy::too_many_lines, clippy::similar_names)] // classList registration boilerplate + getter/setter pairs
fn create_class_list_object(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);

    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    // add(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.add", ctx)?;
                invoke_dom_handler_void(
                    "classList.add",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("add"),
        1,
    );

    // remove(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.remove", ctx)?;
                invoke_dom_handler_void(
                    "classList.remove",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("remove"),
        1,
    );

    // toggle(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.toggle", ctx)?;
                invoke_dom_handler(
                    "classList.toggle",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("toggle"),
        1,
    );

    // contains(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.contains", ctx)?;
                invoke_dom_handler(
                    "classList.contains",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("contains"),
        1,
    );

    // replace(oldClass, newClass)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let old = require_js_string_arg(args, 0, "classList.replace", ctx)?;
                let new = require_js_string_arg(args, 1, "classList.replace", ctx)?;
                invoke_dom_handler(
                    "classList.replace",
                    entity,
                    &[ElidexJsValue::String(old), ElidexJsValue::String(new)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("replace"),
        2,
    );

    // item(index)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let index = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler(
                    "classList.item",
                    entity,
                    &[ElidexJsValue::Number(index)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("item"),
        1,
    );

    // supports() — throws (not supported for classList)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler("classList.supports", entity, &[], bridge)
            },
            b,
        ),
        js_string!("supports"),
        1,
    );

    // value accessor (getter/setter).
    let realm = init.context().realm().clone();

    let b = bridge.clone();
    let val_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("classList.value.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    let b = bridge.clone();
    let val_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "classList.value.set",
                entity,
                &[ElidexJsValue::String(val)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(&realm);

    init.accessor(
        js_string!("value"),
        Some(val_getter),
        Some(val_setter),
        Attribute::CONFIGURABLE,
    );

    // length accessor (read-only).
    let b = bridge.clone();
    let len_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("classList.length", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    init.accessor(
        js_string!("length"),
        Some(len_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    init.build().into()
}

// ---------------------------------------------------------------------------
// CharacterData methods (data, length, substringData, appendData, etc.)
// ---------------------------------------------------------------------------

/// Register `CharacterData` interface methods on the node wrapper.
///
/// Registered on all node wrappers (including Elements). On non-`CharacterData`
/// nodes (i.e., anything other than Text/Comment), the handler layer returns
/// `InvalidStateError`, matching browser behavior where `CharacterData`
/// methods exist on the prototype chain but throw on incorrect node types.
#[allow(clippy::too_many_lines)] // Registration boilerplate; splitting would not improve clarity
fn register_char_data_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // data getter
    let b = bridge.clone();
    let data_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("data.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    // data setter
    let b = bridge.clone();
    let data_set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void("data.set", entity, &[ElidexJsValue::String(text)], bridge)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("data"),
        Some(data_getter),
        Some(data_set_fn),
        Attribute::CONFIGURABLE,
    );

    // length getter (read-only)
    reg_val_accessor(init, bridge, realm, "length", "length.get");

    // substringData(offset, count)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let count = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler(
                    "substringData",
                    entity,
                    &[ElidexJsValue::Number(offset), ElidexJsValue::Number(count)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("substringData"),
        2,
    );

    // appendData(data)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let data = require_js_string_arg(args, 0, "appendData", ctx)?;
                invoke_dom_handler_void(
                    "appendData",
                    entity,
                    &[ElidexJsValue::String(data)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("appendData"),
        1,
    );

    // insertData(offset, data)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let data = require_js_string_arg(args, 1, "insertData", ctx)?;
                invoke_dom_handler_void(
                    "insertData",
                    entity,
                    &[ElidexJsValue::Number(offset), ElidexJsValue::String(data)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("insertData"),
        2,
    );

    // deleteData(offset, count)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let count = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler_void(
                    "deleteData",
                    entity,
                    &[ElidexJsValue::Number(offset), ElidexJsValue::Number(count)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("deleteData"),
        2,
    );

    // replaceData(offset, count, data)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let count = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let data = require_js_string_arg(args, 2, "replaceData", ctx)?;
                invoke_dom_handler_void(
                    "replaceData",
                    entity,
                    &[
                        ElidexJsValue::Number(offset),
                        ElidexJsValue::Number(count),
                        ElidexJsValue::String(data),
                    ],
                    bridge,
                )
            },
            b,
        ),
        js_string!("replaceData"),
        3,
    );

    // splitText(offset) — Text nodes only
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler_ref(
                    "splitText",
                    entity,
                    &[ElidexJsValue::Number(offset)],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("splitText"),
        1,
    );
}

// ---------------------------------------------------------------------------
// Attr node methods (getAttributeNode, setAttributeNode, removeAttributeNode)
// ---------------------------------------------------------------------------

/// Register Attr-related element methods.
fn register_attr_node_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // getAttributeNode(name) → Attr object or null
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "getAttributeNode", ctx)?;
                let result = invoke_dom_handler_ref(
                    "getAttributeNode",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                    ctx,
                )?;
                // Wrap the returned ObjectRef as an Attr object if not null.
                if result.is_null() || result.is_undefined() {
                    return Ok(JsValue::null());
                }
                // The result is already an element wrapper via invoke_dom_handler_ref.
                // Re-wrap it as an Attr-specific object by building attr accessors.
                if let Some(obj) = result.as_object() {
                    let entity_val = obj.get(js_string!(ENTITY_KEY), ctx)?;
                    if !entity_val.is_undefined() {
                        let attr_entity = extract_entity(&result, ctx)?;
                        return Ok(create_attr_object(attr_entity, bridge, ctx));
                    }
                }
                Ok(result)
            },
            b,
        ),
        js_string!("getAttributeNode"),
        1,
    );

    // setAttributeNode(attr) → old Attr or null
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let attr_arg = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("setAttributeNode requires an Attr argument")
                })?;
                let attr_elidex = boa_arg_to_elidex(attr_arg, bridge, ctx)?;
                invoke_dom_handler_ref("setAttributeNode", entity, &[attr_elidex], bridge, ctx)
            },
            b,
        ),
        js_string!("setAttributeNode"),
        1,
    );

    // removeAttributeNode(attr) → removed Attr
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let attr_arg = args.first().ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("removeAttributeNode requires an Attr argument")
                })?;
                let attr_elidex = boa_arg_to_elidex(attr_arg, bridge, ctx)?;
                invoke_dom_handler_ref("removeAttributeNode", entity, &[attr_elidex], bridge, ctx)
            },
            b,
        ),
        js_string!("removeAttributeNode"),
        1,
    );
}

// ---------------------------------------------------------------------------
// Attr wrapper object
// ---------------------------------------------------------------------------

/// Create a JS wrapper object for an `Attr` node entity.
///
/// Provides `name` (getter), `value` (getter/setter), `ownerElement` (getter),
/// and `specified` (getter) — matching the WHATWG `Attr` interface.
pub(crate) fn create_attr_object(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);
    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    let realm = init.context().realm().clone();

    // name (getter)
    reg_val_accessor(&mut init, bridge, &realm, "name", "attr.name.get");

    // value (getter/setter)
    let b = bridge.clone();
    let val_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("attr.value.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    let b = bridge.clone();
    let val_set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "attr.value.set",
                entity,
                &[ElidexJsValue::String(val)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(&realm);

    init.accessor(
        js_string!("value"),
        Some(val_getter),
        Some(val_set_fn),
        Attribute::CONFIGURABLE,
    );

    // ownerElement (getter) — returns element ref or null
    reg_ref_accessor(
        &mut init,
        bridge,
        &realm,
        "ownerElement",
        "attr.ownerElement.get",
    );

    // specified (getter) — always true per modern spec
    reg_val_accessor(&mut init, bridge, &realm, "specified", "attr.specified.get");

    init.build().into()
}

// ---------------------------------------------------------------------------
// DocumentType wrapper object
// ---------------------------------------------------------------------------

/// Create a JS wrapper object for a `DocumentType` node entity.
///
/// Provides `name`, `publicId`, and `systemId` getters — matching the
/// WHATWG `DocumentType` interface.
#[allow(dead_code)] // M4-3.10: Will be used when resolving DocumentType entity wrappers in iframe/multi-Document support.
pub(crate) fn create_doctype_object(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);
    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    let realm = init.context().realm().clone();

    reg_val_accessor(&mut init, bridge, &realm, "name", "doctype.name.get");
    reg_val_accessor(
        &mut init,
        bridge,
        &realm,
        "publicId",
        "doctype.publicId.get",
    );
    reg_val_accessor(
        &mut init,
        bridge,
        &realm,
        "systemId",
        "doctype.systemId.get",
    );

    // Also register tree nav + node info so DocumentType nodes are navigable.
    register_tree_nav_accessors(&mut init, bridge, &realm);
    register_node_info_accessors(&mut init, bridge, &realm);

    init.build().into()
}

/// Resolve an elidex `JsValue::ObjectRef` to a boa element wrapper.
///
/// Used by document methods (querySelector, getElementById, createElement)
/// to return element objects to JS.
pub fn resolve_object_ref(
    result: &ElidexJsValue,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    match result {
        ElidexJsValue::ObjectRef(id) => {
            let obj_ref = JsObjectRef::from_raw(*id);
            bridge.with(|session, _dom| {
                if let Some((entity, _kind)) = session.identity_map().get(obj_ref) {
                    create_element_wrapper(entity, bridge, obj_ref, ctx)
                } else {
                    JsValue::null()
                }
            })
        }
        ElidexJsValue::Null => JsValue::null(),
        other => value_conv::to_boa(other),
    }
}
