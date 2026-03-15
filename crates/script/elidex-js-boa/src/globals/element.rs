//! Element wrapper objects for boa — provides DOM methods on element instances.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::{ComponentKind, JsObjectRef};

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void, require_js_string_arg};
use crate::value_conv;

/// Hidden property key storing the entity bits on element wrapper objects.
pub(crate) const ENTITY_KEY: &str = "__elidex_entity__";

/// Hidden property key for caching the `style` object on an element wrapper.
const STYLE_CACHE_KEY: &str = "__elidex_style__";

/// Hidden property key for caching the `classList` object on an element wrapper.
const CLASSLIST_CACHE_KEY: &str = "__elidex_classList__";

/// Hidden property key for caching the context2d object on a canvas element.
const CONTEXT2D_CACHE_KEY: &str = "__elidex_ctx2d__";

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
                    // TODO(L12): boa 0.20 lacks DOMException — we emulate via TypeError
                    // with a spec-compliant name prefix for feature detection. Replace
                    // with proper DOMException when boa adds WebIDL exception support.
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
