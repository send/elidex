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
/// Provides `window.getComputedStyle(element)`, `window.innerWidth/Height`,
/// `window.scrollX/Y`, `window.scrollTo()`, `window.scrollBy()`, and
/// `window.matchMedia()`.
#[allow(clippy::too_many_lines)]
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

    // innerWidth (read-only getter)
    let b_iw = b.clone();
    let inner_width = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.viewport_width()))),
        b_iw,
    );
    ctx.register_global_builtin_callable(js_string!("__elidex_innerWidth"), 0, inner_width)
        .expect("failed to register innerWidth helper");
    // Register as property on globalThis.
    let global = ctx.global_object();
    let _ = global.set(
        js_string!("innerWidth"),
        JsValue::from(f64::from(bridge.viewport_width())),
        false,
        ctx,
    );

    // innerHeight
    let _ = global.set(
        js_string!("innerHeight"),
        JsValue::from(f64::from(bridge.viewport_height())),
        false,
        ctx,
    );

    // scrollX / scrollY (read-only, updated by content thread)
    let _ = global.set(js_string!("scrollX"), JsValue::from(0.0_f64), false, ctx);
    let _ = global.set(js_string!("scrollY"), JsValue::from(0.0_f64), false, ctx);
    // Aliases per spec.
    let _ = global.set(
        js_string!("pageXOffset"),
        JsValue::from(0.0_f64),
        false,
        ctx,
    );
    let _ = global.set(
        js_string!("pageYOffset"),
        JsValue::from(0.0_f64),
        false,
        ctx,
    );

    // scrollTo(x, y) / scrollTo({top, left, behavior})
    let b_scroll = b.clone();
    let scroll_to = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let (x, y) = parse_scroll_args(args, ctx)?;
            bridge
                .with(|session, _dom| {
                    session.enqueue_event(elidex_script_session::QueuedEvent {
                        event_type: "__scroll_to".to_string(),
                        target: elidex_ecs::Entity::DANGLING,
                        bubbles: false,
                        cancelable: false,
                        payload: elidex_plugin::EventPayload::None,
                    });
                    // Store scroll target in session for content thread to pick up.
                    // For now, we set it directly via a side channel.
                    let _ = (x, y); // TODO: propagate via IPC
                    Ok::<_, elidex_script_session::DomApiError>(())
                })
                .map_err(|e| boa_engine::JsNativeError::typ().with_message(e.to_string()))?;
            Ok(JsValue::undefined())
        },
        b_scroll,
    );
    ctx.register_global_builtin_callable(js_string!("scrollTo"), 2, scroll_to)
        .expect("failed to register scrollTo");

    // scrollBy(x, y) — alias that adds to current scroll.
    let scroll_by = NativeFunction::from_copy_closure(|_this, _args, _ctx| {
        // Simplified: scrollBy not yet functional (needs scroll offset addition).
        Ok(JsValue::undefined())
    });
    ctx.register_global_builtin_callable(js_string!("scrollBy"), 2, scroll_by)
        .expect("failed to register scrollBy");

    // matchMedia(query) — returns a MediaQueryList-like object.
    let match_media = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let query = args
            .first()
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map_or(String::new(), |s| s.to_std_string_escaped());

        // Simplified: always return { matches: false, media: query }.
        let mut obj = ObjectInitializer::new(ctx);
        obj.property(
            js_string!("matches"),
            JsValue::from(false),
            Attribute::READONLY,
        );
        obj.property(
            js_string!("media"),
            JsValue::from(js_string!(query.as_str())),
            Attribute::READONLY,
        );
        Ok(obj.build().into())
    });
    ctx.register_global_builtin_callable(js_string!("matchMedia"), 1, match_media)
        .expect("failed to register matchMedia");
}

/// Parse scroll arguments: either `(x, y)` numbers or `{top, left}` options object.
fn parse_scroll_args(args: &[JsValue], ctx: &mut Context) -> JsResult<(f64, f64)> {
    if let Some(first) = args.first() {
        if let Some(obj) = first.as_object() {
            // Options object: { top, left, behavior }
            let top = obj
                .get(js_string!("top"), ctx)?
                .to_number(ctx)
                .unwrap_or(0.0);
            let left = obj
                .get(js_string!("left"), ctx)?
                .to_number(ctx)
                .unwrap_or(0.0);
            return Ok((left, top));
        }
        // Numeric arguments: scrollTo(x, y)
        let x = first.to_number(ctx).unwrap_or(0.0);
        let y = args
            .get(1)
            .map(|v| v.to_number(ctx))
            .transpose()?
            .unwrap_or(0.0);
        return Ok((x, y));
    }
    Ok((0.0, 0.0))
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

/// Create a CSSStyleDeclaration-like object for `element.style`.
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
                    "style.setProperty",
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
                    "style.getPropertyValue",
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
                    "style.removeProperty",
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
