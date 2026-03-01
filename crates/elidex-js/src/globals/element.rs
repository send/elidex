//! Element wrapper objects for boa — provides DOM methods on element instances.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::{ComponentKind, DomApiHandler, JsObjectRef};

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

#[allow(clippy::too_many_lines)]
fn build_element_object(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> boa_engine::JsObject {
    let entity_bits = entity.to_bits().get() as f64;

    let mut init = ObjectInitializer::new(ctx);

    // Store entity reference.
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    // --- DOM mutation methods ---

    // appendChild(child)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let parent = extract_entity(this, ctx)?;
                let child_val = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("appendChild requires a node argument")
                })?;
                let child_entity = extract_entity(child_val, ctx)?;
                bridge.with(|session, dom| {
                    let child_ref =
                        session.get_or_create_wrapper(child_entity, ComponentKind::Element);
                    elidex_dom_api::AppendChild
                        .invoke(
                            parent,
                            &[ElidexJsValue::ObjectRef(child_ref.to_raw())],
                            session,
                            dom,
                        )
                        .map_err(dom_error_to_js_error)?;
                    Ok(child_val.clone())
                })
            },
            b,
        ),
        js_string!("appendChild"),
        1,
    );

    // removeChild(child)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let parent = extract_entity(this, ctx)?;
                let child_val = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("removeChild requires a node argument")
                })?;
                let child_entity = extract_entity(child_val, ctx)?;
                bridge.with(|session, dom| {
                    let child_ref =
                        session.get_or_create_wrapper(child_entity, ComponentKind::Element);
                    elidex_dom_api::RemoveChild
                        .invoke(
                            parent,
                            &[ElidexJsValue::ObjectRef(child_ref.to_raw())],
                            session,
                            dom,
                        )
                        .map_err(dom_error_to_js_error)?;
                    Ok(child_val.clone())
                })
            },
            b,
        ),
        js_string!("removeChild"),
        1,
    );

    // setAttribute(name, value)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "setAttribute", ctx)?;
                let value = require_js_string_arg(args, 1, "setAttribute", ctx)?;
                invoke_dom_handler_void(
                    &elidex_dom_api::SetAttribute,
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
                    &elidex_dom_api::GetAttribute,
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
                    &elidex_dom_api::RemoveAttribute,
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

    // --- textContent property (getter/setter) ---
    let realm = init.context().realm().clone();

    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler(&elidex_dom_api::GetTextContent, entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

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
                &elidex_dom_api::SetTextContent,
                entity,
                &[ElidexJsValue::String(text)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(&realm);

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
            invoke_dom_handler(&elidex_dom_api::GetInnerHtml, entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    init.accessor(
        js_string!("innerHTML"),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // --- style property (cached on first access) ---
    register_cached_accessor(
        &mut init,
        &realm,
        bridge,
        "style",
        STYLE_CACHE_KEY,
        crate::globals::window::create_style_object,
    );

    // --- classList property (cached on first access) ---
    register_cached_accessor(
        &mut init,
        &realm,
        bridge,
        "classList",
        CLASSLIST_CACHE_KEY,
        create_class_list_object,
    );

    // --- Event listener methods ---

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

    init.build()
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
    let entity_bits = entity.to_bits().get() as f64;

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
                    &elidex_dom_api::ClassListAdd,
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
                    &elidex_dom_api::ClassListRemove,
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
                    &elidex_dom_api::ClassListToggle,
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
                    &elidex_dom_api::ClassListContains,
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
