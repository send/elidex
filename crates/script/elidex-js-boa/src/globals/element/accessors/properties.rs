//! Simple property accessors (`className`, `id`, attributes `NamedNodeMap`).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::core::{entity_bits_as_f64, extract_entity};
use crate::globals::element::ENTITY_KEY;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_ref, invoke_dom_handler_void};

/// Register className and id getter/setter accessors.
#[allow(clippy::similar_names)] // Getter/setter pairs (e.g., cls_getter/cls_setter) intentionally similar
pub(in crate::globals::element) fn register_element_extra_accessors(
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

    // attributes (NamedNodeMap-like object) — read-only
    let b = bridge.clone();
    let attrs_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            build_named_node_map(entity, bridge, ctx)
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!("attributes"),
        Some(attrs_getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

/// Build a `NamedNodeMap`-like JS object for the given element's attributes.
///
/// All accessors (`length`, `item()`, `getNamedItem()`) are live: they query
/// the `Attributes` component on each access, reflecting mutations made after
/// the `NamedNodeMap` was obtained.
///
/// Attribute iteration order follows insertion order per WHATWG DOM spec,
/// backed by `IndexMap` in the `Attributes` component.
#[allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
fn build_named_node_map(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    // Store entity for dynamic lookups.
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits_as_f64(entity)),
        Attribute::empty(),
    );

    // length — dynamic getter that reads current attribute count.
    let realm = init.context().realm().clone();
    let b_len = bridge.clone();
    let length_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let len = bridge.with(|_session, dom| {
                dom.world()
                    .get::<&elidex_ecs::Attributes>(entity)
                    .map_or(0, |a| a.iter().count())
            });
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(len as f64))
        },
        b_len,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("length"),
        Some(length_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // item(index) — reads attributes at call time, returns proper Attr wrapper.
    let b_item = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                // Get the attribute name at the given index.
                let attr_name = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|a| a.iter().nth(index).map(|(k, _)| k.to_string()))
                });
                match attr_name {
                    Some(name) => {
                        // Use getAttributeNode DOM handler to get proper Attr entity.
                        let result = invoke_dom_handler_ref(
                            "getAttributeNode",
                            entity,
                            &[ElidexJsValue::String(name)],
                            bridge,
                            ctx,
                        )?;
                        if result.is_null() || result.is_undefined() {
                            return Ok(JsValue::null());
                        }
                        if let Some(obj) = result.as_object() {
                            let entity_val = obj.get(js_string!(ENTITY_KEY), ctx)?;
                            if !entity_val.is_undefined() {
                                let attr_entity = extract_entity(&result, ctx)?;
                                return Ok(super::super::special_nodes::create_attr_object(
                                    attr_entity,
                                    bridge,
                                    ctx,
                                ));
                            }
                        }
                        Ok(result)
                    }
                    None => Ok(JsValue::null()),
                }
            },
            b_item,
        ),
        js_string!("item"),
        1,
    );

    // getNamedItem(name) — reads attributes at call time, returns proper Attr wrapper.
    let b_named = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                // Check attribute exists before invoking handler.
                let exists = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .is_some_and(|a| a.get(&name).is_some())
                });
                if !exists {
                    return Ok(JsValue::null());
                }
                // Use getAttributeNode DOM handler to get proper Attr entity with identity.
                let result = invoke_dom_handler_ref(
                    "getAttributeNode",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                    ctx,
                )?;
                if result.is_null() || result.is_undefined() {
                    return Ok(JsValue::null());
                }
                if let Some(obj) = result.as_object() {
                    let entity_val = obj.get(js_string!(ENTITY_KEY), ctx)?;
                    if !entity_val.is_undefined() {
                        let attr_entity = extract_entity(&result, ctx)?;
                        return Ok(super::super::special_nodes::create_attr_object(
                            attr_entity,
                            bridge,
                            ctx,
                        ));
                    }
                }
                Ok(result)
            },
            b_named,
        ),
        js_string!("getNamedItem"),
        1,
    );

    // removeNamedItem(name) — removes the attribute from the element.
    let b_rm = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                invoke_dom_handler_void(
                    "removeAttribute",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b_rm,
        ),
        js_string!("removeNamedItem"),
        1,
    );

    Ok(init.build().into())
}
