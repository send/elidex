//! `window` global and `getComputedStyle()` registration.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void, require_js_string_arg};

use super::element::{extract_entity, ENTITY_KEY};

/// Register `window` global (aliased as `globalThis`).
///
/// Provides `window.getComputedStyle(element)`.
/// Also makes `window.location` and `window.history` accessible
/// (the actual objects are registered as global properties in `register_all_globals`).
pub fn register_window(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();

    // getComputedStyle(element) → returns object with property getters
    let b_gcs = b.clone();
    let get_computed_style = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> {
            let elem = args.first().ok_or_else(|| {
                boa_engine::JsNativeError::typ()
                    .with_message("getComputedStyle requires an element argument")
            })?;
            let entity = extract_entity(elem, ctx)?;
            Ok(create_computed_style_proxy(entity, bridge, ctx))
        },
        b_gcs,
    );

    ctx.register_global_builtin_callable(js_string!("getComputedStyle"), 1, get_computed_style)
        .expect("failed to register getComputedStyle");
}

/// Create a computed style proxy object for the given element.
///
/// The returned object's `getPropertyValue(prop)` method looks up
/// the element's `ComputedStyle` and returns the CSS value string.
fn create_computed_style_proxy(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let entity_bits = entity.to_bits().get() as f64;

    let b = bridge.clone();
    let mut obj = ObjectInitializer::new(ctx);
    obj.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let prop = require_js_string_arg(args, 0, "getPropertyValue", ctx)?;
                // GetComputedStyle is a CssomApiHandler, not DomApiHandler.
                // Invoke it directly via the bridge.
                bridge.with(|session, dom| {
                    use elidex_script_session::CssomApiHandler;
                    let result = elidex_dom_api::GetComputedStyle
                        .invoke(entity, &[ElidexJsValue::String(prop)], session, dom)
                        .map_err(dom_error_to_js_error)?;
                    Ok(crate::value_conv::to_boa(&result))
                })
            },
            b,
        ),
        js_string!("getPropertyValue"),
        1,
    );

    obj.build().into()
}

/// Create a CSSStyleDeclaration-like object for `element.style`.
#[allow(clippy::too_many_lines)]
pub fn create_style_object(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let entity_bits = entity.to_bits().get() as f64;

    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    // setProperty(name, value)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "style.setProperty", ctx)?;
                let value = require_js_string_arg(args, 1, "style.setProperty", ctx)?;
                invoke_dom_handler_void(
                    &elidex_dom_api::StyleSetProperty,
                    entity,
                    &[ElidexJsValue::String(name), ElidexJsValue::String(value)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("setProperty"),
        2,
    );

    // getPropertyValue(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "style.getPropertyValue", ctx)?;
                invoke_dom_handler(
                    &elidex_dom_api::StyleGetPropertyValue,
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("getPropertyValue"),
        1,
    );

    // removeProperty(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "style.removeProperty", ctx)?;
                invoke_dom_handler(
                    &elidex_dom_api::StyleRemoveProperty,
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("removeProperty"),
        1,
    );

    init.build().into()
}
