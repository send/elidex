//! Core element wrapper: entity extraction, wrapper creation, child mutation methods,
//! attribute methods.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::{ComponentKind, JsObjectRef};

use super::{register_all_methods, ENTITY_KEY};
use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void, require_js_string_arg};

/// Extract the entity from a JS value that has an `__elidex_entity__` property.
pub fn extract_entity(value: &JsValue, ctx: &mut Context) -> JsResult<Entity> {
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
pub(crate) fn entity_bits_as_f64(entity: Entity) -> f64 {
    entity.to_bits().get() as f64
}

/// Create a boa element wrapper object for the given entity.
pub fn create_element_wrapper(
    entity: Entity,
    bridge: &HostBridge,
    session_entity_ref: JsObjectRef,
    ctx: &mut Context,
) -> JsValue {
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

    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits_as_f64(entity)),
        Attribute::empty(),
    );

    register_child_mutation_methods(&mut init, bridge);
    register_attribute_methods(&mut init, bridge);

    let realm = init.context().realm().clone();

    register_all_methods(&mut init, bridge, &realm);

    init.build()
}

/// Register appendChild and removeChild methods.
pub(crate) fn register_child_mutation_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| dom_child_operation(this, args, bridge, ctx, "appendChild"),
            b,
        ),
        js_string!("appendChild"),
        1,
    );

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

        // Enqueue CE lifecycle reactions for connected/disconnected.
        if let Ok(ce_state) = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(child_entity)
        {
            if ce_state.state == elidex_custom_elements::CEState::Custom {
                match handler_name {
                    "appendChild" | "insertBefore" => {
                        bridge.enqueue_ce_reaction(
                            elidex_custom_elements::CustomElementReaction::Connected(child_entity),
                        );
                    }
                    "removeChild" => {
                        bridge.enqueue_ce_reaction(
                            elidex_custom_elements::CustomElementReaction::Disconnected(
                                child_entity,
                            ),
                        );
                    }
                    _ => {}
                }
            }
        }

        Ok(child_val.clone())
    })
}

/// Register setAttribute, getAttribute, and removeAttribute methods.
pub(crate) fn register_attribute_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "setAttribute", ctx)?;
                let value = require_js_string_arg(args, 1, "setAttribute", ctx)?;

                // Capture old value before mutation for attributeChangedCallback.
                let old_value = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|attrs| attrs.get(&name).map(String::from))
                });

                let result = invoke_dom_handler_void(
                    "setAttribute",
                    entity,
                    &[ElidexJsValue::String(name.clone()), ElidexJsValue::String(value.clone())],
                    bridge,
                );

                // Enqueue CE attributeChangedCallback if applicable.
                if result.is_ok() {
                    bridge.with(|_session, dom| {
                        if let Ok(ce_state) = dom.world().get::<&elidex_custom_elements::CustomElementState>(entity) {
                            if ce_state.state == elidex_custom_elements::CEState::Custom {
                                let observed = bridge.ce_observed_attributes(&ce_state.definition_name);
                                if observed.contains(&name) {
                                    bridge.enqueue_ce_reaction(
                                        elidex_custom_elements::CustomElementReaction::AttributeChanged {
                                            entity,
                                            name: name.clone(),
                                            old_value,
                                            new_value: Some(value.clone()),
                                        },
                                    );
                                }
                            }
                        }
                    });
                }

                result
            },
            b,
        ),
        js_string!("setAttribute"),
        2,
    );

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
