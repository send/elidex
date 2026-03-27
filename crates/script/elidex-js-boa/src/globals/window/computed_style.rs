//! `getComputedStyle()` proxy object construction.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::element::{extract_entity, ENTITY_KEY};
use crate::globals::require_js_string_arg;

/// Create a computed style proxy object for the given element.
///
/// The returned object's `getPropertyValue(prop)` method looks up
/// the element's `ComputedStyle` and returns the CSS value string.
pub(super) fn create_computed_style_proxy(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
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
                // Look up from the CSSOM registry.
                let handler = bridge
                    .cssom_registry()
                    .resolve("getComputedStyle")
                    .ok_or_else(|| {
                        boa_engine::JsNativeError::typ()
                            .with_message("Unknown CSSOM method: getComputedStyle")
                    })?;
                bridge.with(|session, dom| {
                    let result = handler
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
