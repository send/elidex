//! `window` global and related registrations.

mod computed_style;
mod media_query;
mod selection;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::{extract_entity, ENTITY_KEY};
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void, require_js_string_arg};

/// Register `window` global (aliased as `globalThis`).
///
/// Provides `window.getComputedStyle(element)`, `window.innerWidth/Height`,
/// `window.scrollX/Y`, `window.scrollTo()`, `window.scrollBy()`, and
/// `window.matchMedia()`.
pub fn register_window(ctx: &mut Context, bridge: &HostBridge) {
    // getComputedStyle(element) → returns object with property getters
    let b_gcs = bridge.clone();
    let get_computed_style = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> {
            let elem = args.first().ok_or_else(|| {
                boa_engine::JsNativeError::typ()
                    .with_message("getComputedStyle requires an element argument")
            })?;
            let entity = extract_entity(elem, ctx)?;
            Ok(computed_style::create_computed_style_proxy(
                entity, bridge, ctx,
            ))
        },
        b_gcs,
    );
    ctx.register_global_builtin_callable(js_string!("getComputedStyle"), 1, get_computed_style)
        .expect("failed to register getComputedStyle");

    register_viewport_accessors(ctx, bridge);
    register_scroll_methods(ctx, bridge);
    media_query::register_media_query(ctx, bridge);
    selection::register_selection(ctx, bridge);
    register_iframe_window_props(ctx);
    register_window_open(ctx, bridge);
    register_messaging(ctx, bridge);
    register_modals(ctx, bridge);
}

/// Register `innerWidth`, `innerHeight`, `scrollX`, `scrollY` (and `pageXOffset`/`pageYOffset`
/// aliases) as dynamic getters on the global object.
#[allow(clippy::similar_names)] // b_iw/b_ih/b_sx/b_sy are intentionally named per property.
fn register_viewport_accessors(ctx: &mut Context, bridge: &HostBridge) {
    use boa_engine::property::PropertyDescriptorBuilder;

    let global = ctx.global_object();

    let b_iw = bridge.clone();
    let iw_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.viewport_width()))),
        b_iw,
    )
    .to_js_function(ctx.realm());
    let _ = global.define_property_or_throw(
        js_string!("innerWidth"),
        PropertyDescriptorBuilder::new()
            .get(iw_getter)
            .enumerable(true)
            .configurable(true)
            .build(),
        ctx,
    );

    let b_ih = bridge.clone();
    let ih_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.viewport_height()))),
        b_ih,
    )
    .to_js_function(ctx.realm());
    let _ = global.define_property_or_throw(
        js_string!("innerHeight"),
        PropertyDescriptorBuilder::new()
            .get(ih_getter)
            .enumerable(true)
            .configurable(true)
            .build(),
        ctx,
    );

    // scrollX / scrollY — dynamic getters reading from bridge scroll offset.
    let b_sx = bridge.clone();
    let sx_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.scroll_x()))),
        b_sx,
    )
    .to_js_function(ctx.realm());
    let _ = global.define_property_or_throw(
        js_string!("scrollX"),
        PropertyDescriptorBuilder::new()
            .get(sx_getter.clone())
            .enumerable(true)
            .configurable(true)
            .build(),
        ctx,
    );
    // pageXOffset is an alias for scrollX per spec.
    let _ = global.define_property_or_throw(
        js_string!("pageXOffset"),
        PropertyDescriptorBuilder::new()
            .get(sx_getter)
            .enumerable(true)
            .configurable(true)
            .build(),
        ctx,
    );

    let b_sy = bridge.clone();
    let sy_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.scroll_y()))),
        b_sy,
    )
    .to_js_function(ctx.realm());
    let _ = global.define_property_or_throw(
        js_string!("scrollY"),
        PropertyDescriptorBuilder::new()
            .get(sy_getter.clone())
            .enumerable(true)
            .configurable(true)
            .build(),
        ctx,
    );
    let _ = global.define_property_or_throw(
        js_string!("pageYOffset"),
        PropertyDescriptorBuilder::new()
            .get(sy_getter)
            .enumerable(true)
            .configurable(true)
            .build(),
        ctx,
    );
}

/// Register `scrollTo(x, y)` and `scrollBy(x, y)` global functions.
fn register_scroll_methods(ctx: &mut Context, bridge: &HostBridge) {
    // scrollTo(x, y) / scrollTo({top, left, behavior})
    let b_scroll = bridge.clone();
    let scroll_to = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let (opt_x, opt_y) = parse_scroll_args(args, ctx)?;
            // When an axis is not specified, keep the current scroll position.
            let x = opt_x.unwrap_or_else(|| f64::from(bridge.scroll_x()));
            let y = opt_y.unwrap_or_else(|| f64::from(bridge.scroll_y()));
            // CSSOM View §4.2: NaN/Infinity values must be ignored.
            if x.is_finite() && y.is_finite() {
                // Store as pending scroll; the content thread picks it up on the
                // next frame, updates viewport_scroll, and syncs back to bridge.
                #[allow(clippy::cast_possible_truncation)]
                bridge.set_pending_scroll(x as f32, y as f32);
            }
            Ok(JsValue::undefined())
        },
        b_scroll,
    );
    ctx.register_global_builtin_callable(js_string!("scrollTo"), 2, scroll_to)
        .expect("failed to register scrollTo");

    // scrollBy(x, y) — adds delta to current scroll offset.
    let b_scroll_by = bridge.clone();
    let scroll_by = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let (opt_x, opt_y) = parse_scroll_args(args, ctx)?;
            let x = opt_x.unwrap_or(0.0);
            let y = opt_y.unwrap_or(0.0);
            let cur_x = f64::from(bridge.scroll_x());
            let cur_y = f64::from(bridge.scroll_y());
            let new_x = cur_x + x;
            let new_y = cur_y + y;
            // CSSOM View §4.2: NaN/Infinity values must be ignored.
            if new_x.is_finite() && new_y.is_finite() {
                #[allow(clippy::cast_possible_truncation)]
                bridge.set_pending_scroll(new_x as f32, new_y as f32);
            }
            Ok(JsValue::undefined())
        },
        b_scroll_by,
    );
    ctx.register_global_builtin_callable(js_string!("scrollBy"), 2, scroll_by)
        .expect("failed to register scrollBy");
}

/// Register iframe-related window properties: `parent`, `top`, `frames`,
/// `frameElement`, `length`, `opener` (WHATWG HTML §7.1.3).
fn register_iframe_window_props(ctx: &mut Context) {
    // window.parent — returns `self` for top-level, parent window for iframes.
    // Boa limitation: each iframe has its own JsRuntime/Context, so we can't
    // return the actual parent's global object. Returns `self` as a proxy
    // (correct for top-level, degraded-but-safe for iframes).
    // Self-hosted engine (M4-9+) will implement cross-context window proxies.
    ctx.register_global_property(
        js_string!("parent"),
        JsValue::from(ctx.global_object()),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.parent");

    // window.top — same limitation as window.parent.
    ctx.register_global_property(
        js_string!("top"),
        JsValue::from(ctx.global_object()),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.top");

    // window.frameElement — null for top-level, iframe element for embedded docs.
    // Cross-origin: always null (WHATWG HTML §7.1.3).
    // For same-origin iframes, should return the <iframe> element from parent DOM.
    // Boa limitation: can't return an object from parent's Context.
    // Returns null for now; self-hosted engine will implement cross-context proxies.
    ctx.register_global_property(
        js_string!("frameElement"),
        JsValue::null(),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.frameElement");

    // window.length — number of child iframes.
    // MVP: always 0 (iframe counting will be added when iframe loading is implemented).
    ctx.register_global_property(
        js_string!("length"),
        JsValue::from(0),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.length");

    // window.opener — null for iframes (only set by window.open).
    ctx.register_global_property(
        js_string!("opener"),
        JsValue::null(),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.opener");

    // window.frames — alias for window (WHATWG HTML §7.1.3).
    // Returns the window itself; window[0], window[1] etc. access child iframes.
    ctx.register_global_property(
        js_string!("frames"),
        JsValue::from(ctx.global_object()),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.frames");
}

/// Register `window.open(url, target, features)` (WHATWG HTML §7.5.2).
///
/// MVP limitations: `features` string is ignored, returns `null` (no `WindowProxy`).
fn register_window_open(ctx: &mut Context, bridge: &HostBridge) {
    let b_open = bridge.clone();
    let open_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> {
            // Sandbox allow-popups check.
            if !bridge.popups_allowed() {
                return Ok(JsValue::null());
            }
            let url_str = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let target = args
                .get(1)
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or_else(|| String::from("_blank"), |s| s.to_std_string_escaped());
            // _features arg is intentionally ignored (MVP).

            // Empty/whitespace-only URL means "about:blank" per WHATWG HTML §7.5.2.
            let resolved = if url_str.trim().is_empty() {
                Ok(url::Url::parse("about:blank").expect("about:blank is valid"))
            } else {
                // Resolve relative URLs against the document's URL.
                url::Url::parse(&url_str).or_else(|_| {
                    bridge
                        .current_url()
                        .unwrap_or_else(|| {
                            url::Url::parse("about:blank").expect("about:blank is valid")
                        })
                        .join(&url_str)
                })
            };

            if let Ok(url) = resolved {
                match target.as_str() {
                    "_blank" | "" => {
                        // Open in new tab via ContentToBrowser::OpenNewTab.
                        bridge.queue_open_tab(url);
                    }
                    "_self" => {
                        bridge.set_pending_navigation(elidex_navigation::NavigationRequest {
                            url: url.to_string(),
                            replace: false,
                        });
                    }
                    "_parent" | "_top" => {
                        // Sandbox allow-top-navigation check (WHATWG HTML §4.8.5):
                        // if sandboxed without allow-top-navigation, block navigation.
                        if bridge.sandbox_flags().is_some_and(|f| {
                            !f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_TOP_NAVIGATION)
                        }) {
                            return Ok(JsValue::null());
                        }
                        // For top-level documents, _parent and _top are same as _self.
                        // For iframes, boa cross-context limitation means we navigate
                        // the current document (same as _self).
                        bridge.set_pending_navigation(elidex_navigation::NavigationRequest {
                            url: url.to_string(),
                            replace: false,
                        });
                    }
                    named => {
                        // Named target: queue iframe navigation by name.
                        // Content thread will search iframes registry; if not
                        // found, falls back to opening a new tab.
                        bridge.set_pending_navigate_iframe(named.to_string(), url);
                    }
                }
            }

            // Return null (no WindowProxy for the opened window).
            Ok(JsValue::null())
        },
        b_open,
    );
    ctx.register_global_builtin_callable(js_string!("open"), 3, open_fn)
        .expect("failed to register window.open");
}

/// Register `window.postMessage(message, targetOrigin)` (WHATWG HTML §9.4.3).
fn register_messaging(ctx: &mut Context, bridge: &HostBridge) {
    let b_pm = bridge.clone();
    let post_message_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> {
            // WHATWG HTML §9.4.3: targetOrigin is required.
            if args.len() < 2 {
                return Err(boa_engine::JsNativeError::typ()
                    .with_message("Failed to execute 'postMessage': 2 arguments required")
                    .into());
            }
            let message = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let target_origin = args
                .get(1)
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or_else(|| String::from("/"), |s| s.to_std_string_escaped());

            // targetOrigin check per WHATWG HTML §9.4.3:
            // "*" matches all origins, "/" is shorthand for own origin.
            let own_origin = bridge.origin();
            let own_serialized = own_origin.serialize();
            let origin_matches =
                target_origin == "*" || target_origin == "/" || own_serialized == target_origin;

            if origin_matches {
                // Buffer the message for delivery in the next event loop tick.
                // Delivery is asynchronous per WHATWG HTML §9.4.3.
                bridge.queue_post_message(message, own_origin.serialize());
            }

            Ok(JsValue::undefined())
        },
        b_pm,
    );
    ctx.register_global_builtin_callable(js_string!("postMessage"), 2, post_message_fn)
        .expect("failed to register postMessage");
}

/// Register `alert`, `confirm`, `prompt` with sandbox `allow-modals` enforcement.
fn register_modals(ctx: &mut Context, bridge: &HostBridge) {
    let b_alert = bridge.clone();
    let alert_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            if !bridge.modals_allowed() {
                return Ok(JsValue::undefined());
            }
            // MVP: alert is a no-op (no modal UI).
            Ok(JsValue::undefined())
        },
        b_alert,
    );
    ctx.register_global_builtin_callable(js_string!("alert"), 1, alert_fn)
        .expect("failed to register alert");

    let b_confirm = bridge.clone();
    let confirm_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            if !bridge.modals_allowed() {
                return Ok(JsValue::from(false));
            }
            // MVP: confirm always returns false (no modal UI).
            Ok(JsValue::from(false))
        },
        b_confirm,
    );
    ctx.register_global_builtin_callable(js_string!("confirm"), 1, confirm_fn)
        .expect("failed to register confirm");

    let b_prompt = bridge.clone();
    let prompt_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            if !bridge.modals_allowed() {
                return Ok(JsValue::null());
            }
            // MVP: prompt always returns null (no modal UI).
            Ok(JsValue::null())
        },
        b_prompt,
    );
    ctx.register_global_builtin_callable(js_string!("prompt"), 1, prompt_fn)
        .expect("failed to register prompt");
}

/// Parse scroll arguments: either `(x, y)` numbers or `{top, left}` options object.
///
/// Returns `(Option<x>, Option<y>)` — `None` means the axis was not specified,
/// so the caller should preserve the current scroll position for that axis.
fn parse_scroll_args(args: &[JsValue], ctx: &mut Context) -> JsResult<(Option<f64>, Option<f64>)> {
    if let Some(first) = args.first() {
        if let Some(obj) = first.as_object() {
            // Options object: { top, left, behavior }
            let top_val = obj.get(js_string!("top"), ctx)?;
            let top = if top_val.is_undefined() {
                None
            } else {
                Some(top_val.to_number(ctx)?)
            };
            let left_val = obj.get(js_string!("left"), ctx)?;
            let left = if left_val.is_undefined() {
                None
            } else {
                Some(left_val.to_number(ctx)?)
            };
            return Ok((left, top));
        }
        // Numeric arguments: scrollTo(x, y)
        let x = Some(first.to_number(ctx)?);
        let y = if let Some(v) = args.get(1) {
            Some(v.to_number(ctx)?)
        } else {
            Some(0.0)
        };
        return Ok((x, y));
    }
    Ok((Some(0.0), Some(0.0)))
}

/// Create a `CSSStyleDeclaration`-like object for `element.style`.
#[allow(clippy::too_many_lines, clippy::similar_names)]
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

    // cssText — getter that serializes all inline style properties.
    let realm = init.context().realm().clone();
    let b_css_get = bridge.clone();
    let css_text_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = bridge.with(|_session, dom| {
                dom.world()
                    .get::<&elidex_ecs::InlineStyle>(entity)
                    .ok()
                    .map_or(String::new(), |style| style.css_text())
            });
            Ok(JsValue::from(js_string!(text.as_str())))
        },
        b_css_get,
    )
    .to_js_function(&realm);
    // cssText — setter that parses and replaces all inline style properties.
    // Routes through the mutation system (style.removeProperty / style.setProperty)
    // so that MutationObservers see the changes.
    let b_css_set = bridge.clone();
    let css_text_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());

            // Collect existing property names to remove.
            let existing_props: Vec<String> = bridge.with(|_session, dom| {
                dom.world()
                    .get::<&elidex_ecs::InlineStyle>(entity)
                    .ok()
                    .map_or_else(Vec::new, |style| {
                        style.iter().map(|(k, _)| k.to_string()).collect()
                    })
            });

            // Remove each existing property through the mutation system.
            for prop in &existing_props {
                let _ = invoke_dom_handler_void(
                    "style.removeProperty",
                    entity,
                    &[ElidexJsValue::String(prop.clone())],
                    bridge,
                );
            }

            // Parse new declarations and set each through the mutation system.
            for decl in text.split(';') {
                let decl = decl.trim();
                if let Some((prop, val)) = decl.split_once(':') {
                    let prop = prop.trim().to_string();
                    let val = val.trim().to_string();
                    if !prop.is_empty() && !val.is_empty() {
                        let _ = invoke_dom_handler_void(
                            "style.setProperty",
                            entity,
                            &[ElidexJsValue::String(prop), ElidexJsValue::String(val)],
                            bridge,
                        );
                    }
                }
            }

            Ok(JsValue::undefined())
        },
        b_css_set,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("cssText"),
        Some(css_text_getter),
        Some(css_text_setter),
        Attribute::CONFIGURABLE,
    );

    // length — getter that returns the number of inline style properties.
    let b_len = bridge.clone();
    let length_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let len = bridge.with(|_session, dom| {
                dom.world()
                    .get::<&elidex_ecs::InlineStyle>(entity)
                    .ok()
                    .map_or(0, |style| style.len())
            });
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

    // item(index) — returns the property name at the given index.
    let b_item = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                let name = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::InlineStyle>(entity)
                        .ok()
                        .and_then(|style| style.property_at(index).map(str::to_owned))
                });
                Ok(name.map_or(JsValue::from(js_string!("")), |n| {
                    JsValue::from(js_string!(n.as_str()))
                }))
            },
            b_item,
        ),
        js_string!("item"),
        1,
    );

    init.build().into()
}
