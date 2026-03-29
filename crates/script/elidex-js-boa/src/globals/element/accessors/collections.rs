//! Dataset and classList accessor registration and object creation.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::core::{entity_bits_as_f64, extract_entity};
use crate::globals::element::{DATASET_CACHE_KEY, ENTITY_KEY};
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void, require_js_string_arg};

/// Register the `dataset` cached accessor.
pub(in crate::globals::element) fn register_dataset_accessor(
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
pub(crate) fn create_dataset_object(
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
pub(crate) fn register_cached_accessor(
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
pub(crate) fn create_class_list_object(
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

    // forEach(callback) — iterates over all class names.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("classList.forEach: argument must be a function")
                })?;
                let class_str = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|a| a.get("class").map(String::from))
                        .unwrap_or_default()
                });
                for (i, class_name) in class_str.split_whitespace().enumerate() {
                    #[allow(clippy::cast_precision_loss)]
                    let _ = callback.call(
                        &JsValue::undefined(),
                        &[
                            JsValue::from(js_string!(class_name)),
                            JsValue::from(i as f64),
                        ],
                        ctx,
                    );
                }
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("forEach"),
        1,
    );

    init.build().into()
}
