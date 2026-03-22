//! Shadow DOM and Canvas element method registration.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_script_session::ComponentKind;

use crate::bridge::HostBridge;
use super::core::{extract_entity, create_element_wrapper};
use super::CONTEXT2D_CACHE_KEY;

/// Register attachShadow method and shadowRoot accessor.
pub(crate) fn register_shadow_dom_methods(
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
pub(crate) fn register_canvas_method(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
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
