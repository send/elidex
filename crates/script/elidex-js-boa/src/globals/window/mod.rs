//! `window` global and related registrations.

mod computed_style;
mod media_query;
mod selection;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::{extract_entity, ENTITY_KEY};
use crate::globals::{
    invoke_dom_handler, invoke_dom_handler_ref, invoke_dom_handler_void, require_js_string_arg,
};

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
    register_window_event_target(ctx, bridge);
    register_screen_and_window_props(ctx, bridge);
    register_performance(ctx, bridge);
    register_atob_btoa(ctx);
    register_crypto(ctx);
    register_queue_microtask(ctx);
    register_image_constructor(ctx, bridge);
    register_dom_geometry(ctx);
    register_visual_viewport(ctx, bridge);
    register_dom_parser(ctx, bridge);
    register_xml_serializer(ctx, bridge);
    register_idle_callbacks(ctx);
    register_structured_clone(ctx);
}

/// Register `Image()` named constructor (WHATWG HTML §4.8.3).
fn register_image_constructor(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    ctx.register_global_callable(
        js_string!("Image"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                // Create <img> element via createElement.
                let doc = bridge.document_entity();
                let handler = bridge.dom_registry().resolve("createElement").ok_or_else(|| {
                    boa_engine::JsNativeError::typ().with_message("createElement handler not found")
                })?;
                let result = bridge.with(|session, dom| {
                    handler
                        .invoke(
                            doc,
                            &[elidex_plugin::JsValue::String("img".to_string())],
                            session,
                            dom,
                        )
                        .map_err(crate::error_conv::dom_error_to_js_error)
                })?;
                let wrapper =
                    crate::globals::element::resolve_object_ref(&result, bridge, ctx);

                // Set width/height content attributes if provided.
                if let Some(entity) =
                    wrapper.as_object().and_then(|_| {
                        crate::globals::element::extract_entity(&wrapper, ctx).ok()
                    })
                {
                    if let Some(w) = args.first().and_then(JsValue::as_number) {
                        bridge.with(|_session, dom| {
                            if let Ok(mut attrs) =
                                dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity)
                            {
                                #[allow(clippy::cast_possible_truncation)]
                                attrs.set("width", &(w as i64).to_string());
                            }
                        });
                    }
                    if let Some(h) = args.get(1).and_then(JsValue::as_number) {
                        bridge.with(|_session, dom| {
                            if let Ok(mut attrs) =
                                dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity)
                            {
                                #[allow(clippy::cast_possible_truncation)]
                                attrs.set("height", &(h as i64).to_string());
                            }
                        });
                    }
                }

                Ok(wrapper)
            },
            b,
        ),
    )
    .expect("failed to register Image");
}

/// Register `addEventListener`, `removeEventListener`, `dispatchEvent` on window.
///
/// Window events target the document entity (matching browser behavior where
/// window-level listeners participate in the DOM propagation path).
fn register_window_event_target(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let add_listener = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let doc = bridge.document_entity();
            crate::globals::add_event_listener_for(doc, args, bridge, ctx)
        },
        b,
    );
    ctx.register_global_builtin_callable(js_string!("addEventListener"), 2, add_listener)
        .expect("failed to register window.addEventListener");

    let b = bridge.clone();
    let rm_listener = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let doc = bridge.document_entity();
            crate::globals::remove_event_listener_for(doc, args, bridge, ctx)
        },
        b,
    );
    ctx.register_global_builtin_callable(js_string!("removeEventListener"), 2, rm_listener)
        .expect("failed to register window.removeEventListener");

    let b = bridge.clone();
    let dispatch = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let doc = bridge.document_entity();
            crate::globals::dispatch_event_for(doc, args, bridge, ctx)
        },
        b,
    );
    ctx.register_global_builtin_callable(js_string!("dispatchEvent"), 1, dispatch)
        .expect("failed to register window.dispatchEvent");
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

/// Register `screen` object and additional window properties (M4-4.5 Step 8).
fn register_screen_and_window_props(ctx: &mut Context, bridge: &HostBridge) {
    use boa_engine::property::PropertyDescriptorBuilder;

    let global = ctx.global_object();

    // --- screen object ---
    let mut screen_init = ObjectInitializer::new(ctx);

    // screen.width / screen.height — monitor resolution from bridge.
    let b = bridge.clone();
    let realm = screen_init.context().realm().clone();
    let sw_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.monitor_width() as f64))
        },
        b,
    )
    .to_js_function(&realm);

    let b = bridge.clone();
    let sh_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.monitor_height() as f64))
        },
        b,
    )
    .to_js_function(&realm);

    for name in ["width", "availWidth"] {
        screen_init.accessor(
            js_string!(name),
            Some(sw_getter.clone()),
            None,
            Attribute::CONFIGURABLE,
        );
    }
    for name in ["height", "availHeight"] {
        screen_init.accessor(
            js_string!(name),
            Some(sh_getter.clone()),
            None,
            Attribute::CONFIGURABLE,
        );
    }

    // colorDepth / pixelDepth — from bridge (default 24 for 8-bit-per-channel).
    let b = bridge.clone();
    let cd_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.color_depth())),
        b,
    )
    .to_js_function(&realm);
    for name in ["colorDepth", "pixelDepth"] {
        screen_init.accessor(
            js_string!(name),
            Some(cd_getter.clone()),
            None,
            Attribute::CONFIGURABLE,
        );
    }

    let screen_obj = screen_init.build();
    global
        .set(js_string!("screen"), JsValue::from(screen_obj), false, ctx)
        .expect("failed to register screen");

    // --- window.name (getter/setter) ---
    let b = bridge.clone();
    let name_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(js_string!(bridge.window_name()))),
        b,
    );
    let b = bridge.clone();
    let name_setter = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());
            bridge.set_window_name(name);
            Ok(JsValue::undefined())
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(name_getter.to_js_function(&realm))
        .set(name_setter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("name"), desc, ctx)
        .expect("failed to register window.name");

    // --- Simple window properties as globals ---
    // window.self / window.window — self-reference.
    global
        .set(js_string!("self"), JsValue::from(global.clone()), false, ctx)
        .expect("failed to register window.self");
    global
        .set(js_string!("window"), JsValue::from(global.clone()), false, ctx)
        .expect("failed to register window.window");

    // window.closed
    global
        .set(js_string!("closed"), JsValue::from(false), false, ctx)
        .expect("failed to register window.closed");

    // window.devicePixelRatio — dynamic getter from bridge.
    let b = bridge.clone();
    let dpr_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.device_pixel_ratio() as f64))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(dpr_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("devicePixelRatio"), desc, ctx)
        .expect("failed to register window.devicePixelRatio");

    // window.outerWidth / outerHeight — viewport + chrome bar heights.
    // CHROME_HEIGHT (36.0) + TAB_BAR_HEIGHT (28.0) = 64.0 total chrome.
    const CHROME_OVERHEAD: f64 = 64.0;
    let b = bridge.clone();
    let ow_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.viewport_width() as f64))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(ow_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("outerWidth"), desc, ctx)
        .expect("failed to register window.outerWidth");

    let b = bridge.clone();
    let oh_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.viewport_height() as f64 + CHROME_OVERHEAD))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(oh_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("outerHeight"), desc, ctx)
        .expect("failed to register window.outerHeight");

    // window.screenX/Y, screenLeft/Top — dynamic getters from bridge.
    let b = bridge.clone();
    let sx_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.screen_x())),
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(sx_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("screenX"), desc.clone(), ctx)
        .expect("failed to register window.screenX");
    global
        .define_property_or_throw(js_string!("screenLeft"), desc, ctx)
        .expect("failed to register window.screenLeft");

    let b = bridge.clone();
    let sy_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.screen_y())),
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(sy_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("screenY"), desc.clone(), ctx)
        .expect("failed to register window.screenY");
    global
        .define_property_or_throw(js_string!("screenTop"), desc, ctx)
        .expect("failed to register window.screenTop");

    // window.isSecureContext
    let b = bridge.clone();
    let isc_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            let is_secure = bridge.current_url().is_some_and(|url| {
                is_potentially_trustworthy_url(&url)
            });
            // WHATWG HTML §3.4: an iframe context is secure only if both the
            // current context and all ancestor contexts are secure. If this
            // document is in an iframe (frame_element is Some), also check:
            // - the origin must not be opaque (sandboxed without allow-same-origin)
            // - the parent origin must be a tuple origin (non-opaque)
            let is_secure = is_secure && {
                if bridge.frame_element().is_some() {
                    let origin = bridge.origin();
                    match origin {
                        elidex_plugin::SecurityOrigin::Opaque(_) => false,
                        elidex_plugin::SecurityOrigin::Tuple { ref scheme, .. } => {
                            scheme == "https" || scheme == "file"
                        }
                    }
                } else {
                    true
                }
            };
            Ok(JsValue::from(is_secure))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(isc_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("isSecureContext"), desc, ctx)
        .expect("failed to register window.isSecureContext");

    // window.origin
    let b = bridge.clone();
    let origin_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            let origin = bridge
                .current_url()
                .map_or("null".to_string(), |url| url.origin().ascii_serialization());
            Ok(JsValue::from(js_string!(origin)))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(origin_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("origin"), desc, ctx)
        .expect("failed to register window.origin");

    // window.focus() — requests window focus via IPC (WHATWG HTML §7.2.7.1).
    let b_focus = bridge.clone();
    ctx.register_global_builtin_callable(
        js_string!("focus"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| {
                bridge.request_focus();
                Ok(JsValue::undefined())
            },
            b_focus,
        ),
    )
    .expect("failed to register window.focus");

    // window.blur() — no-op per WHATWG HTML §7.2.7.1.
    ctx.register_global_builtin_callable(
        js_string!("blur"),
        0,
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
    )
    .expect("failed to register window.blur");

    // window.stop() — abort pending fetches + cancel timers (WHATWG HTML §7.1.5).
    let b = bridge.clone();
    ctx.register_global_builtin_callable(
        js_string!("stop"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| {
                bridge.clear_all_timers();
                Ok(JsValue::undefined())
            },
            b,
        ),
    )
    .expect("failed to register window.stop");
}

/// Wrapper around `Instant` to implement `boa_gc::Trace` (no GC objects inside).
#[derive(Clone, Copy)]
struct TracedInstant(std::time::Instant);
impl_empty_trace!(TracedInstant);

/// Register `performance` object (W3C HR-Time §4 + User Timing §3-4).
fn register_performance(ctx: &mut Context, _bridge: &HostBridge) {
    // Capture time origin at registration (approximates navigation start).
    let origin = TracedInstant(std::time::Instant::now());

    // Pre-build closures that capture origin before ObjectInitializer borrows ctx.
    let now_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, origin, _ctx| {
            let elapsed_ms = origin.0.elapsed().as_secs_f64() * 1000.0;
            // Round to 100μs for security (W3C HR-Time §4.4).
            let rounded = (elapsed_ms * 10.0).floor() / 10.0;
            Ok(JsValue::from(rounded))
        },
        origin,
    );

    // timeOrigin — Unix epoch milliseconds at navigation start.
    let time_origin = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0);

    // Performance entries stored in a shared JsArray.
    let entries = boa_engine::object::builtins::JsArray::new(ctx);
    let entries_obj = JsValue::from(entries);

    let mut init = ObjectInitializer::new(ctx);

    init.function(now_fn, js_string!("now"), 0);

    init.property(
        js_string!("timeOrigin"),
        JsValue::from(time_origin),
        Attribute::READONLY,
    );

    // Hidden entries storage (writable so clearMarks/clearMeasures can replace).
    init.property(
        js_string!("__entries__"),
        entries_obj,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // performance.mark(name, options?) — W3C User Timing §3.
    let o2 = origin;
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, origin, ctx| {
                let name = crate::globals::require_js_string_arg(args, 0, "performance.mark", ctx)?;

                let start_time = args
                    .get(1)
                    .and_then(|o| o.as_object())
                    .and_then(|o| {
                        o.get(js_string!("startTime"), ctx)
                            .ok()
                            .and_then(|v| v.as_number())
                    })
                    .unwrap_or_else(|| {
                        let elapsed_ms = origin.0.elapsed().as_secs_f64() * 1000.0;
                        (elapsed_ms * 10.0).floor() / 10.0
                    });

                // Build PerformanceMark entry.
                let mut entry = ObjectInitializer::new(ctx);
                entry.property(
                    js_string!("entryType"),
                    JsValue::from(js_string!("mark")),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                entry.property(
                    js_string!("name"),
                    JsValue::from(js_string!(name.as_str())),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                entry.property(
                    js_string!("startTime"),
                    JsValue::from(start_time),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                entry.property(
                    js_string!("duration"),
                    JsValue::from(0.0),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );
                let mark_obj = entry.build();

                // Append to entries list.
                let perf = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("performance: this is not an object")
                })?;
                let entries_val = perf.get(js_string!("__entries__"), ctx)?;
                if let Some(arr) = entries_val.as_object() {
                    let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                    arr.set(len, JsValue::from(mark_obj.clone()), false, ctx)?;
                }

                Ok(JsValue::from(mark_obj))
            },
            o2,
        ),
        js_string!("mark"),
        1,
    );

    // performance.measure(name, startOrOptions?, endMark?) — W3C User Timing §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let name = crate::globals::require_js_string_arg(args, 0, "performance.measure", ctx)?;

            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            let entries_arr = entries_val.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: internal error")
            })?;

            // Helper: find a mark by name.
            let find_mark = |mark_name: &str, ctx: &mut Context| -> JsResult<f64> {
                let len = entries_arr
                    .get(js_string!("length"), ctx)?
                    .to_number(ctx)? as u32;
                // Search from end (latest mark with this name wins).
                let mut i = len;
                while i > 0 {
                    i -= 1;
                    let e = entries_arr.get(i, ctx)?;
                    if let Some(e_obj) = e.as_object() {
                        let e_type = e_obj
                            .get(js_string!("entryType"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        let e_name = e_obj
                            .get(js_string!("name"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if e_type == "mark" && e_name == mark_name {
                            return e_obj.get(js_string!("startTime"), ctx)?.to_number(ctx);
                        }
                    }
                }
                Err(JsNativeError::syntax()
                    .with_message(format!(
                        "SyntaxError: performance.measure: mark '{mark_name}' not found"
                    ))
                    .into())
            };

            // Resolve start and end times.
            let (start_time, end_time) = match args.get(1) {
                Some(v) if v.is_object() => {
                    // Options object form: { start, end, duration }.
                    let opts = v.as_object().unwrap();
                    let s = opts
                        .get(js_string!("start"), ctx)?;
                    let e = opts
                        .get(js_string!("end"), ctx)?;

                    let st = if let Some(n) = s.as_number() {
                        n
                    } else if !s.is_undefined() && !s.is_null() {
                        find_mark(&s.to_string(ctx)?.to_std_string_escaped(), ctx)?
                    } else {
                        0.0
                    };

                    let et = if let Some(n) = e.as_number() {
                        n
                    } else if !e.is_undefined() && !e.is_null() {
                        find_mark(&e.to_string(ctx)?.to_std_string_escaped(), ctx)?
                    } else {
                        // Use performance.now().
                        let now_val = perf.get(js_string!("now"), ctx)?;
                        if let Some(now_fn) = now_val.as_callable() {
                            now_fn.call(&JsValue::from(perf.clone()), &[], ctx)?.to_number(ctx)?
                        } else {
                            0.0
                        }
                    };
                    (st, et)
                }
                Some(v) if !v.is_undefined() && !v.is_null() => {
                    // String form: startMark name.
                    let start_mark = v.to_string(ctx)?.to_std_string_escaped();
                    let st = find_mark(&start_mark, ctx)?;

                    let et = if let Some(end_v) = args.get(2) {
                        if !end_v.is_undefined() && !end_v.is_null() {
                            let end_mark = end_v.to_string(ctx)?.to_std_string_escaped();
                            find_mark(&end_mark, ctx)?
                        } else {
                            let now_val = perf.get(js_string!("now"), ctx)?;
                            if let Some(now_fn) = now_val.as_callable() {
                                now_fn.call(&JsValue::from(perf.clone()), &[], ctx)?.to_number(ctx)?
                            } else {
                                0.0
                            }
                        }
                    } else {
                        let now_val = perf.get(js_string!("now"), ctx)?;
                        if let Some(now_fn) = now_val.as_callable() {
                            now_fn.call(&JsValue::from(perf.clone()), &[], ctx)?.to_number(ctx)?
                        } else {
                            0.0
                        }
                    };
                    (st, et)
                }
                _ => {
                    // No start specified → start from 0.
                    let et = {
                        let now_val = perf.get(js_string!("now"), ctx)?;
                        if let Some(now_fn) = now_val.as_callable() {
                            now_fn.call(&JsValue::from(perf.clone()), &[], ctx)?.to_number(ctx)?
                        } else {
                            0.0
                        }
                    };
                    (0.0, et)
                }
            };

            let duration = end_time - start_time;

            let mut entry = ObjectInitializer::new(ctx);
            entry.property(
                js_string!("entryType"),
                JsValue::from(js_string!("measure")),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            entry.property(
                js_string!("name"),
                JsValue::from(js_string!(name.as_str())),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            entry.property(
                js_string!("startTime"),
                JsValue::from(start_time),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            entry.property(
                js_string!("duration"),
                JsValue::from(duration),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            let measure_obj = entry.build();

            // Append to entries list.
            let len = entries_arr
                .get(js_string!("length"), ctx)?
                .to_number(ctx)? as u32;
            entries_arr.set(len, JsValue::from(measure_obj.clone()), false, ctx)?;

            Ok(JsValue::from(measure_obj))
        }),
        js_string!("measure"),
        1,
    );

    // performance.getEntries() — W3C Performance Timeline §4.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            // Return a copy of the entries array.
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            if let Some(arr) = entries_val.as_object() {
                let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                for i in 0..len {
                    result.push(arr.get(i, ctx)?, ctx)?;
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getEntries"),
        0,
    );

    // performance.getEntriesByType(type) — W3C Performance Timeline §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let entry_type =
                crate::globals::require_js_string_arg(args, 0, "getEntriesByType", ctx)?;
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            if let Some(arr) = entries_val.as_object() {
                let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                for i in 0..len {
                    let e = arr.get(i, ctx)?;
                    if let Some(e_obj) = e.as_object() {
                        let t = e_obj
                            .get(js_string!("entryType"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if t == entry_type {
                            result.push(e, ctx)?;
                        }
                    }
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getEntriesByType"),
        1,
    );

    // performance.getEntriesByName(name, type?) — W3C Performance Timeline §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let perf = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("performance: this is not an object")
            })?;
            let name = crate::globals::require_js_string_arg(args, 0, "getEntriesByName", ctx)?;
            let type_filter = args
                .get(1)
                .and_then(|v| {
                    if v.is_undefined() || v.is_null() {
                        None
                    } else {
                        Some(v.to_string(ctx).ok()?.to_std_string_escaped())
                    }
                });
            let entries_val = perf.get(js_string!("__entries__"), ctx)?;
            let result = boa_engine::object::builtins::JsArray::new(ctx);
            if let Some(arr) = entries_val.as_object() {
                let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
                for i in 0..len {
                    let e = arr.get(i, ctx)?;
                    if let Some(e_obj) = e.as_object() {
                        let n = e_obj
                            .get(js_string!("name"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if n != name {
                            continue;
                        }
                        if let Some(ref tf) = type_filter {
                            let t = e_obj
                                .get(js_string!("entryType"), ctx)?
                                .to_string(ctx)?
                                .to_std_string_escaped();
                            if &t != tf {
                                continue;
                            }
                        }
                        result.push(e, ctx)?;
                    }
                }
            }
            Ok(JsValue::from(result))
        }),
        js_string!("getEntriesByName"),
        1,
    );

    // performance.clearMarks(name?) — W3C User Timing §3.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            clear_entries_by_type(this, args, "mark", ctx)
        }),
        js_string!("clearMarks"),
        0,
    );

    // performance.clearMeasures(name?) — W3C User Timing §4.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            clear_entries_by_type(this, args, "measure", ctx)
        }),
        js_string!("clearMeasures"),
        0,
    );

    let perf = init.build();
    ctx.register_global_property(js_string!("performance"), perf, Attribute::all())
        .expect("failed to register performance");
}

/// Helper: clear performance entries by type, optionally filtered by name.
fn clear_entries_by_type(
    this: &JsValue,
    args: &[JsValue],
    entry_type: &str,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let perf = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("performance: this is not an object")
    })?;
    let name_filter = args
        .first()
        .and_then(|v| {
            if v.is_undefined() || v.is_null() {
                None
            } else {
                Some(v.to_string(ctx).ok()?.to_std_string_escaped())
            }
        });

    let entries_val = perf.get(js_string!("__entries__"), ctx)?;
    if let Some(arr) = entries_val.as_object() {
        let new_arr = boa_engine::object::builtins::JsArray::new(ctx);
        let len = arr.get(js_string!("length"), ctx)?.to_number(ctx)? as u32;
        for i in 0..len {
            let e = arr.get(i, ctx)?;
            let mut keep = true;
            if let Some(e_obj) = e.as_object() {
                let t = e_obj
                    .get(js_string!("entryType"), ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                if t == entry_type {
                    if let Some(ref nf) = name_filter {
                        let n = e_obj
                            .get(js_string!("name"), ctx)?
                            .to_string(ctx)?
                            .to_std_string_escaped();
                        if &n == nf {
                            keep = false;
                        }
                    } else {
                        keep = false;
                    }
                }
            }
            if keep {
                new_arr.push(e, ctx)?;
            }
        }
        perf.set(js_string!("__entries__"), JsValue::from(new_arr), false, ctx)?;
    }
    Ok(JsValue::undefined())
}

/// Register `atob()` and `btoa()` (WHATWG HTML §8.3).
fn register_atob_btoa(ctx: &mut Context) {
    use base64::Engine;

    // btoa(str) — Latin1 → Base64.
    let btoa_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let input = args
            .first()
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map_or(String::new(), |s| s.to_std_string_escaped());

        // Check for non-Latin1 characters (> U+00FF).
        if input.chars().any(|c| c as u32 > 0xFF) {
            return Err(boa_engine::JsNativeError::eval()
                .with_message("InvalidCharacterError: btoa: string contains non-Latin1 character")
                .into());
        }

        let bytes: Vec<u8> = input.chars().map(|c| c as u8).collect();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(JsValue::from(js_string!(encoded.as_str())))
    });
    ctx.register_global_builtin_callable(js_string!("btoa"), 1, btoa_fn)
        .expect("failed to register btoa");

    // atob(str) — Base64 → Latin1.
    let atob_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let input = args
            .first()
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map_or(String::new(), |s| s.to_std_string_escaped());

        // Strip ASCII whitespace (WHATWG HTML §8.3).
        let stripped: String = input
            .chars()
            .filter(|c| !matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' '))
            .collect();

        // Forgiving decode — accept missing padding.
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::GeneralPurposeConfig::new()
                .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
        );
        let bytes = engine.decode(&stripped).map_err(|_| {
            boa_engine::JsNativeError::eval()
                .with_message("InvalidCharacterError: atob: invalid base64 input")
        })?;

        // Convert bytes to Latin1 string.
        let result: String = bytes.iter().map(|&b| b as char).collect();
        Ok(JsValue::from(js_string!(result.as_str())))
    });
    ctx.register_global_builtin_callable(js_string!("atob"), 1, atob_fn)
        .expect("failed to register atob");
}

/// Register `crypto` object (W3C WebCrypto).
fn register_crypto(ctx: &mut Context) {
    let mut init = ObjectInitializer::new(ctx);

    // crypto.getRandomValues(typedArray) — fill with random bytes.
    init.function(
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let arr = args.first().and_then(JsValue::as_object).ok_or_else(|| {
                boa_engine::JsNativeError::typ()
                    .with_message("crypto.getRandomValues: argument must be a typed array")
            })?;
            let len_val = arr.get(js_string!("length"), ctx)?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let len = len_val.to_number(ctx)? as usize;

            // W3C WebCrypto §10.1.1: max 65536 bytes.
            if len > 65536 {
                return Err(boa_engine::JsNativeError::eval()
                    .with_message("QuotaExceededError: getRandomValues: array too large")
                    .into());
            }

            let mut bytes = vec![0u8; len];
            getrandom::fill(&mut bytes).map_err(|_| {
                boa_engine::JsNativeError::eval()
                    .with_message("crypto.getRandomValues: random generation failed")
            })?;

            for (i, &b) in bytes.iter().enumerate() {
                arr.set(i as u32, JsValue::from(f64::from(b)), false, ctx)?;
            }

            Ok(args.first().cloned().unwrap_or(JsValue::undefined()))
        }),
        js_string!("getRandomValues"),
        1,
    );

    // crypto.randomUUID() — UUID v4.
    init.function(
        NativeFunction::from_copy_closure(|_this, _args, _ctx| {
            let mut bytes = [0u8; 16];
            let _ = getrandom::fill(&mut bytes);
            // Set version (4) and variant (RFC 4122).
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let uuid = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                bytes[0], bytes[1], bytes[2], bytes[3],
                bytes[4], bytes[5],
                bytes[6], bytes[7],
                bytes[8], bytes[9],
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            );
            Ok(JsValue::from(js_string!(uuid.as_str())))
        }),
        js_string!("randomUUID"),
        0,
    );

    let crypto = init.build();
    ctx.register_global_property(js_string!("crypto"), crypto, Attribute::all())
        .expect("failed to register crypto");
}

/// Register `queueMicrotask()` (WHATWG HTML §8.6).
fn register_queue_microtask(ctx: &mut Context) {
    ctx.register_global_builtin_callable(
        js_string!("queueMicrotask"),
        1,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                boa_engine::JsNativeError::typ()
                    .with_message("queueMicrotask: argument must be a function")
            })?;
            // Execute the callback immediately via microtask semantics.
            // boa's run_jobs() drains microtasks after eval, so calling now
            // achieves the same effect for synchronous JS contexts.
            if let Err(err) = callback.call(&JsValue::undefined(), &[], ctx) {
                eprintln!("[queueMicrotask Error] {err}");
            }
            Ok(JsValue::undefined())
        }),
    )
    .expect("failed to register queueMicrotask");
}

// ---------------------------------------------------------------------------
// DOM Geometry (CSSWG Geometry §5-6)
// ---------------------------------------------------------------------------

/// Helper: build a DOMPoint-like JS object (shared by DOMPoint and DOMPointReadOnly).
fn build_dom_point(
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    mutable: bool,
    ctx: &mut Context,
) -> JsResult<boa_engine::JsObject> {
    let mut init = ObjectInitializer::new(ctx);
    let attr = if mutable {
        Attribute::WRITABLE | Attribute::CONFIGURABLE
    } else {
        Attribute::READONLY | Attribute::CONFIGURABLE
    };
    init.property(js_string!("x"), JsValue::from(x), attr);
    init.property(js_string!("y"), JsValue::from(y), attr);
    init.property(js_string!("z"), JsValue::from(z), attr);
    init.property(js_string!("w"), JsValue::from(w), attr);

    // toJSON()
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("DOMPoint: this is not an object")
            })?;
            let vx = obj.get(js_string!("x"), ctx)?;
            let vy = obj.get(js_string!("y"), ctx)?;
            let vz = obj.get(js_string!("z"), ctx)?;
            let vw = obj.get(js_string!("w"), ctx)?;
            let mut json_init = ObjectInitializer::new(ctx);
            json_init.property(js_string!("x"), vx, Attribute::all());
            json_init.property(js_string!("y"), vy, Attribute::all());
            json_init.property(js_string!("z"), vz, Attribute::all());
            json_init.property(js_string!("w"), vw, Attribute::all());
            Ok(JsValue::from(json_init.build()))
        }),
        js_string!("toJSON"),
        0,
    );

    Ok(init.build())
}

/// Extract point coordinates from args (x?, y?, z?, w?) with defaults.
fn extract_point_args(args: &[JsValue]) -> (f64, f64, f64, f64) {
    let x = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
    let y = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
    let z = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
    let w = args.get(3).and_then(JsValue::as_number).unwrap_or(1.0);
    (x, y, z, w)
}

/// Extract point from an object dict (for `fromPoint` static methods).
fn extract_point_dict(val: &JsValue, ctx: &mut Context) -> JsResult<(f64, f64, f64, f64)> {
    if let Some(obj) = val.as_object() {
        let x = dict_number(&obj, "x", 0.0, ctx)?;
        let y = dict_number(&obj, "y", 0.0, ctx)?;
        let z = dict_number(&obj, "z", 0.0, ctx)?;
        let w = dict_number(&obj, "w", 1.0, ctx)?;
        Ok((x, y, z, w))
    } else {
        Ok((0.0, 0.0, 0.0, 1.0))
    }
}

/// Read a numeric property from a JS object, returning `default` when the
/// property is `undefined` or `null` (boa's `to_number` converts `undefined`
/// to `NaN`, so we must check explicitly).
fn dict_number(
    obj: &boa_engine::JsObject,
    key: &str,
    default: f64,
    ctx: &mut Context,
) -> JsResult<f64> {
    let v = obj.get(js_string!(key), ctx)?;
    if v.is_undefined() || v.is_null() {
        Ok(default)
    } else {
        Ok(v.to_number(ctx).unwrap_or(default))
    }
}

/// Register `DOMPoint`, `DOMPointReadOnly`, `DOMMatrix`, `DOMMatrixReadOnly`, `DOMRect`.
fn register_dom_geometry(ctx: &mut Context) {
    // DOMPointReadOnly constructor.
    ctx.register_global_callable(
        js_string!("DOMPointReadOnly"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let (x, y, z, w) = extract_point_args(args);
            Ok(JsValue::from(build_dom_point(x, y, z, w, false, ctx)?))
        }),
    )
    .expect("failed to register DOMPointReadOnly");

    // DOMPointReadOnly.fromPoint static method.
    {
        let global = ctx.global_object();
        let ctor = global
            .get(js_string!("DOMPointReadOnly"), ctx)
            .expect("DOMPointReadOnly must exist");
        if let Some(ctor_obj) = ctor.as_object() {
            let from_point = NativeFunction::from_copy_closure(|_this, args, ctx| {
                let dict = args.first().cloned().unwrap_or(JsValue::undefined());
                let (x, y, z, w) = extract_point_dict(&dict, ctx)?;
                Ok(JsValue::from(build_dom_point(x, y, z, w, false, ctx)?))
            });
            let desc = boa_engine::property::PropertyDescriptorBuilder::new()
                .value(from_point.to_js_function(ctx.realm()))
                .writable(true)
                .enumerable(false)
                .configurable(true)
                .build();
            ctor_obj
                .define_property_or_throw(js_string!("fromPoint"), desc, ctx)
                .expect("failed to set DOMPointReadOnly.fromPoint");
        }
    }

    // DOMPoint constructor.
    ctx.register_global_callable(
        js_string!("DOMPoint"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let (x, y, z, w) = extract_point_args(args);
            Ok(JsValue::from(build_dom_point(x, y, z, w, true, ctx)?))
        }),
    )
    .expect("failed to register DOMPoint");

    // DOMPoint.fromPoint static method.
    {
        let global = ctx.global_object();
        let ctor = global
            .get(js_string!("DOMPoint"), ctx)
            .expect("DOMPoint must exist");
        if let Some(ctor_obj) = ctor.as_object() {
            let from_point = NativeFunction::from_copy_closure(|_this, args, ctx| {
                let dict = args.first().cloned().unwrap_or(JsValue::undefined());
                let (x, y, z, w) = extract_point_dict(&dict, ctx)?;
                Ok(JsValue::from(build_dom_point(x, y, z, w, true, ctx)?))
            });
            let desc = boa_engine::property::PropertyDescriptorBuilder::new()
                .value(from_point.to_js_function(ctx.realm()))
                .writable(true)
                .enumerable(false)
                .configurable(true)
                .build();
            ctor_obj
                .define_property_or_throw(js_string!("fromPoint"), desc, ctx)
                .expect("failed to set DOMPoint.fromPoint");
        }
    }

    // DOMRect constructor.
    ctx.register_global_callable(
        js_string!("DOMRect"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let x = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
            let y = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
            let w = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
            let h = args.get(3).and_then(JsValue::as_number).unwrap_or(0.0);
            let mut init = ObjectInitializer::new(ctx);
            let attr = Attribute::WRITABLE | Attribute::CONFIGURABLE;
            init.property(js_string!("x"), JsValue::from(x), attr);
            init.property(js_string!("y"), JsValue::from(y), attr);
            init.property(js_string!("width"), JsValue::from(w), attr);
            init.property(js_string!("height"), JsValue::from(h), attr);
            // Derived read-only properties.
            init.property(
                js_string!("top"),
                JsValue::from(y.min(y + h)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!("right"),
                JsValue::from(x.max(x + w)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!("bottom"),
                JsValue::from(y.max(y + h)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.property(
                js_string!("left"),
                JsValue::from(x.min(x + w)),
                Attribute::READONLY | Attribute::CONFIGURABLE,
            );
            init.function(
                NativeFunction::from_copy_closure(|this, _args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMRect: this is not an object")
                    })?;
                    let vals: Vec<(String, JsValue)> = ["x", "y", "width", "height", "top", "right", "bottom", "left"]
                        .iter()
                        .map(|key| {
                            let v = obj.get(js_string!(*key), ctx).unwrap_or(JsValue::from(0.0));
                            ((*key).to_string(), v)
                        })
                        .collect();
                    let mut json_init = ObjectInitializer::new(ctx);
                    for (key, v) in vals {
                        json_init.property(js_string!(key.as_str()), v, Attribute::all());
                    }
                    Ok(JsValue::from(json_init.build()))
                }),
                js_string!("toJSON"),
                0,
            );
            Ok(JsValue::from(init.build()))
        }),
    )
    .expect("failed to register DOMRect");

    // DOMMatrix / DOMMatrixReadOnly — 4×4 identity matrix by default.
    register_dom_matrix(ctx, "DOMMatrixReadOnly", false);
    register_dom_matrix(ctx, "DOMMatrix", true);
}

/// Register a DOMMatrix or DOMMatrixReadOnly constructor.
fn register_dom_matrix(ctx: &mut Context, name: &str, mutable: bool) {
    let constructor = NativeFunction::from_copy_closure(move |_this, _args, ctx| {
        // Default: 4×4 identity matrix.
        let attr = if mutable {
            Attribute::WRITABLE | Attribute::CONFIGURABLE
        } else {
            Attribute::READONLY | Attribute::CONFIGURABLE
        };

        let mut init = ObjectInitializer::new(ctx);

        // 2D aliases (CSS transform): a=m11, b=m12, c=m21, d=m22, e=m41, f=m42.
        let identity = [
            ("a", 1.0),
            ("b", 0.0),
            ("c", 0.0),
            ("d", 1.0),
            ("e", 0.0),
            ("f", 0.0),
        ];
        for (key, val) in &identity {
            init.property(js_string!(*key), JsValue::from(*val), attr);
        }

        // Full 4×4 matrix elements.
        let m4x4 = [
            ("m11", 1.0),
            ("m12", 0.0),
            ("m13", 0.0),
            ("m14", 0.0),
            ("m21", 0.0),
            ("m22", 1.0),
            ("m23", 0.0),
            ("m24", 0.0),
            ("m31", 0.0),
            ("m32", 0.0),
            ("m33", 1.0),
            ("m34", 0.0),
            ("m41", 0.0),
            ("m42", 0.0),
            ("m43", 0.0),
            ("m44", 1.0),
        ];
        for (key, val) in &m4x4 {
            init.property(js_string!(*key), JsValue::from(*val), attr);
        }

        init.property(
            js_string!("is2D"),
            JsValue::from(true),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );
        init.property(
            js_string!("isIdentity"),
            JsValue::from(true),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );

        // transformPoint(point) — applies matrix transform to a point.
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let dict = args.first().cloned().unwrap_or(JsValue::undefined());
                let (px, py, pz, pw) = extract_point_dict(&dict, ctx)?;
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let e = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let f = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
                // 2D affine: x' = a*px + c*py + e, y' = b*px + d*py + f
                let rx = a * px + c * py + e * pw;
                let ry = b * px + d * py + f * pw;
                Ok(JsValue::from(build_dom_point(rx, ry, pz, pw, true, ctx)?))
            }),
            js_string!("transformPoint"),
            0,
        );

        if mutable {
            // --- Mutation methods (return `this` for chaining) ---

            // translateSelf(tx, ty, tz?)
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let tx = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
                    let ty = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
                    let tz = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
                    let e_val = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    let f_val = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    let m43 = obj.get(js_string!("m43"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    obj.set(js_string!("e"), JsValue::from(e_val + tx), false, ctx)?;
                    obj.set(js_string!("m41"), JsValue::from(e_val + tx), false, ctx)?;
                    obj.set(js_string!("f"), JsValue::from(f_val + ty), false, ctx)?;
                    obj.set(js_string!("m42"), JsValue::from(f_val + ty), false, ctx)?;
                    obj.set(js_string!("m43"), JsValue::from(m43 + tz), false, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("translateSelf"),
                2,
            );

            // scaleSelf(scaleX, scaleY?, scaleZ?)
            // Post-multiply by a scale matrix: a' = a*sx, b' = b*sx, c' = c*sy, d' = d*sy.
            // e and f are unchanged by pure scale.
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let sx = args.first().and_then(JsValue::as_number).unwrap_or(1.0);
                    let sy = args.get(1).and_then(JsValue::as_number).unwrap_or(sx);
                    let sz = args.get(2).and_then(JsValue::as_number).unwrap_or(1.0);
                    let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                    let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                    let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                    let m33 = obj.get(js_string!("m33"), ctx)?.to_number(ctx).unwrap_or(1.0);
                    obj.set(js_string!("a"), JsValue::from(a * sx), false, ctx)?;
                    obj.set(js_string!("m11"), JsValue::from(a * sx), false, ctx)?;
                    obj.set(js_string!("b"), JsValue::from(b * sx), false, ctx)?;
                    obj.set(js_string!("m12"), JsValue::from(b * sx), false, ctx)?;
                    obj.set(js_string!("c"), JsValue::from(c * sy), false, ctx)?;
                    obj.set(js_string!("m21"), JsValue::from(c * sy), false, ctx)?;
                    obj.set(js_string!("d"), JsValue::from(d * sy), false, ctx)?;
                    obj.set(js_string!("m22"), JsValue::from(d * sy), false, ctx)?;
                    obj.set(js_string!("m33"), JsValue::from(m33 * sz), false, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("scaleSelf"),
                1,
            );

            // rotateSelf(rotX, rotY?, rotZ?)
            // For 2D: when only one arg is given, it's the Z rotation angle in degrees.
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let angle_deg = if args.len() <= 1 {
                        args.first().and_then(JsValue::as_number).unwrap_or(0.0)
                    } else {
                        args.get(2).and_then(JsValue::as_number).unwrap_or(0.0)
                    };
                    let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                    let (na, nb, nc, nd) = rotate_2d(a, b, c, d, e, f, angle_deg);
                    write_matrix_to_obj(&obj, na, nb, nc, nd, e, f, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("rotateSelf"),
                1,
            );

            // multiplySelf(other) — post-multiply this by other (2D).
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let other = args.first().and_then(JsValue::as_object).ok_or_else(|| {
                        JsNativeError::typ().with_message("multiplySelf: argument must be a DOMMatrix")
                    })?;
                    let (a1, b1, c1, d1, e1, f1) = read_matrix_components(&obj, ctx)?;
                    let (a2, b2, c2, d2, e2, f2) = read_matrix_components(&other, ctx)?;
                    let (na, nb, nc, nd, ne, nf) = multiply_2d(a1, b1, c1, d1, e1, f1, a2, b2, c2, d2, e2, f2);
                    write_matrix_to_obj(&obj, na, nb, nc, nd, ne, nf, ctx)?;
                    Ok(this.clone())
                }),
                js_string!("multiplySelf"),
                1,
            );

            // invertSelf() — invert the 2D matrix in-place.
            init.function(
                NativeFunction::from_copy_closure(|this, _args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                    if let Some((na, nb, nc, nd, ne, nf)) = invert_2d(a, b, c, d, e, f) {
                        write_matrix_to_obj(&obj, na, nb, nc, nd, ne, nf, ctx)?;
                    } else {
                        // Singular matrix — set all to NaN per spec.
                        write_matrix_to_obj(
                            &obj,
                            f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN,
                            ctx,
                        )?;
                    }
                    Ok(this.clone())
                }),
                js_string!("invertSelf"),
                0,
            );
            // setMatrixValue(transformList) — parse "matrix(a,b,c,d,e,f)" or "none".
            init.function(
                NativeFunction::from_copy_closure(|this, args, ctx| {
                    let obj = this.as_object().ok_or_else(|| {
                        JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                    })?;
                    let input = args
                        .first()
                        .map(|v| v.to_string(ctx))
                        .transpose()?
                        .map_or(String::new(), |s| s.to_std_string_escaped());
                    let trimmed = input.trim();
                    if trimmed == "none" || trimmed.is_empty() {
                        for key in ["a", "m11"] {
                            obj.set(js_string!(key), JsValue::from(1.0), false, ctx)?;
                        }
                        for key in ["b", "m12", "c", "m21", "e", "m41", "f", "m42"] {
                            obj.set(js_string!(key), JsValue::from(0.0), false, ctx)?;
                        }
                        for key in ["d", "m22"] {
                            obj.set(js_string!(key), JsValue::from(1.0), false, ctx)?;
                        }
                    } else if let Some(inner) =
                        trimmed.strip_prefix("matrix(").and_then(|s| s.strip_suffix(')'))
                    {
                        let parts: Vec<f64> = inner
                            .split(',')
                            .filter_map(|s| s.trim().parse::<f64>().ok())
                            .collect();
                        if parts.len() == 6 {
                            let keys = [
                                ("a", "m11"),
                                ("b", "m12"),
                                ("c", "m21"),
                                ("d", "m22"),
                                ("e", "m41"),
                                ("f", "m42"),
                            ];
                            for (i, (k1, k2)) in keys.iter().enumerate() {
                                obj.set(js_string!(*k1), JsValue::from(parts[i]), false, ctx)?;
                                obj.set(js_string!(*k2), JsValue::from(parts[i]), false, ctx)?;
                            }
                        } else {
                            return Err(JsNativeError::syntax()
                                .with_message("setMatrixValue: invalid matrix() format")
                                .into());
                        }
                    } else {
                        return Err(JsNativeError::syntax()
                            .with_message(
                                "setMatrixValue: unsupported transform (only matrix(a,b,c,d,e,f) and none)",
                            )
                            .into());
                    }
                    Ok(this.clone())
                }),
                js_string!("setMatrixValue"),
                1,
            );
        }

        // --- Immutable methods (return new DOMMatrix) ---

        // multiply(other) — return new DOMMatrix = this * other (2D).
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let other = args.first().and_then(JsValue::as_object).ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("multiply: argument must be a DOMMatrix")
                })?;
                let (a1, b1, c1, d1, e1, f1) = read_matrix_components(&obj, ctx)?;
                let (a2, b2, c2, d2, e2, f2) = read_matrix_components(&other, ctx)?;
                let (na, nb, nc, nd, ne, nf) = multiply_2d(a1, b1, c1, d1, e1, f1, a2, b2, c2, d2, e2, f2);
                Ok(JsValue::from(build_dom_matrix_obj(na, nb, nc, nd, ne, nf, ctx)?))
            }),
            js_string!("multiply"),
            1,
        );

        // inverse() — return new inverted DOMMatrix (2D).
        init.function(
            NativeFunction::from_copy_closure(|this, _args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                let (na, nb, nc, nd, ne, nf) = invert_2d(a, b, c, d, e, f)
                    .unwrap_or((f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN));
                Ok(JsValue::from(build_dom_matrix_obj(na, nb, nc, nd, ne, nf, ctx)?))
            }),
            js_string!("inverse"),
            0,
        );

        // rotate(rotX, rotY?, rotZ?) — return new rotated DOMMatrix (2D Z-rotation).
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let angle_deg = if args.len() <= 1 {
                    args.first().and_then(JsValue::as_number).unwrap_or(0.0)
                } else {
                    args.get(2).and_then(JsValue::as_number).unwrap_or(0.0)
                };
                let (a, b, c, d, e, f) = read_matrix_components(&obj, ctx)?;
                let (na, nb, nc, nd) = rotate_2d(a, b, c, d, e, f, angle_deg);
                Ok(JsValue::from(build_dom_matrix_obj(na, nb, nc, nd, e, f, ctx)?))
            }),
            js_string!("rotate"),
            1,
        );

        // translate(tx, ty, tz?) — returns a new DOMMatrix with translation applied.
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let tx = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
                let ty = args.get(1).and_then(JsValue::as_number).unwrap_or(0.0);
                let _tz = args.get(2).and_then(JsValue::as_number).unwrap_or(0.0);
                let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let e = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let f = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let ne = e + tx;
                let nf = f + ty;
                let result = build_dom_matrix_obj(a, b, c, d, ne, nf, ctx)?;
                Ok(JsValue::from(result))
            }),
            js_string!("translate"),
            2,
        );

        // scale(scaleX, scaleY?, scaleZ?) — returns a new scaled DOMMatrix.
        // Post-multiply by a scale matrix: a' = a*sx, b' = b*sx, c' = c*sy, d' = d*sy.
        init.function(
            NativeFunction::from_copy_closure(|this, args, ctx| {
                let obj = this.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("DOMMatrix: this is not an object")
                })?;
                let sx = args.first().and_then(JsValue::as_number).unwrap_or(1.0);
                let sy = args.get(1).and_then(JsValue::as_number).unwrap_or(sx);
                let a = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let b = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let c = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let d = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
                let e = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let f = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
                let result = build_dom_matrix_obj(a * sx, b * sx, c * sy, d * sy, e, f, ctx)?;
                Ok(JsValue::from(result))
            }),
            js_string!("scale"),
            1,
        );

        Ok(JsValue::from(init.build()))
    });

    ctx.register_global_callable(js_string!(name), 0, constructor)
        .expect("failed to register DOMMatrix");
}

/// Build a new DOMMatrix JS object with the given 2D affine values.
// --- Pure 2D matrix math helpers ---

/// 2D matrix multiply: `[a1 c1 e1; b1 d1 f1; 0 0 1] * [a2 c2 e2; b2 d2 f2; 0 0 1]`.
#[allow(clippy::too_many_arguments)]
fn multiply_2d(
    a1: f64, b1: f64, c1: f64, d1: f64, e1: f64, f1: f64,
    a2: f64, b2: f64, c2: f64, d2: f64, e2: f64, f2: f64,
) -> (f64, f64, f64, f64, f64, f64) {
    (
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * e2 + c1 * f2 + e1,
        b1 * e2 + d1 * f2 + f1,
    )
}

/// Invert a 2D matrix. Returns `None` if the matrix is singular.
fn invert_2d(
    a: f64, b: f64, c: f64, d: f64, e: f64, f: f64,
) -> Option<(f64, f64, f64, f64, f64, f64)> {
    let det = a * d - b * c;
    if det.abs() < f64::EPSILON {
        return None;
    }
    let inv = 1.0 / det;
    Some((
        d * inv,
        -b * inv,
        -c * inv,
        a * inv,
        (c * f - d * e) * inv,
        (b * e - a * f) * inv,
    ))
}

/// Rotate a 2D matrix by `angle_deg` degrees around Z.
fn rotate_2d(
    a: f64, b: f64, c: f64, d: f64, _e: f64, _f: f64,
    angle_deg: f64,
) -> (f64, f64, f64, f64) {
    let angle = angle_deg * std::f64::consts::PI / 180.0;
    let cos = angle.cos();
    let sin = angle.sin();
    (
        a * cos + c * sin,
        b * cos + d * sin,
        a * -sin + c * cos,
        b * -sin + d * cos,
    )
}

/// Read the 6 2D matrix components (a-f) from a JS object.
fn read_matrix_components(
    obj: &boa_engine::JsObject,
    ctx: &mut Context,
) -> JsResult<(f64, f64, f64, f64, f64, f64)> {
    Ok((
        obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0),
        obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0),
        obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0),
        obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0),
        obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0),
        obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0),
    ))
}

/// Write 2D matrix components (a-f + m aliases) to a mutable DOMMatrix JS object.
fn write_matrix_to_obj(
    obj: &boa_engine::JsObject,
    a: f64, b: f64, c: f64, d: f64, e: f64, f: f64,
    ctx: &mut Context,
) -> JsResult<()> {
    obj.set(js_string!("a"), JsValue::from(a), false, ctx)?;
    obj.set(js_string!("m11"), JsValue::from(a), false, ctx)?;
    obj.set(js_string!("b"), JsValue::from(b), false, ctx)?;
    obj.set(js_string!("m12"), JsValue::from(b), false, ctx)?;
    obj.set(js_string!("c"), JsValue::from(c), false, ctx)?;
    obj.set(js_string!("m21"), JsValue::from(c), false, ctx)?;
    obj.set(js_string!("d"), JsValue::from(d), false, ctx)?;
    obj.set(js_string!("m22"), JsValue::from(d), false, ctx)?;
    obj.set(js_string!("e"), JsValue::from(e), false, ctx)?;
    obj.set(js_string!("m41"), JsValue::from(e), false, ctx)?;
    obj.set(js_string!("f"), JsValue::from(f), false, ctx)?;
    obj.set(js_string!("m42"), JsValue::from(f), false, ctx)?;
    Ok(())
}

fn build_dom_matrix_obj(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    ctx: &mut Context,
) -> JsResult<boa_engine::JsObject> {
    let attr = Attribute::WRITABLE | Attribute::CONFIGURABLE;
    let mut init = ObjectInitializer::new(ctx);
    init.property(js_string!("a"), JsValue::from(a), attr);
    init.property(js_string!("b"), JsValue::from(b), attr);
    init.property(js_string!("c"), JsValue::from(c), attr);
    init.property(js_string!("d"), JsValue::from(d), attr);
    init.property(js_string!("e"), JsValue::from(e), attr);
    init.property(js_string!("f"), JsValue::from(f), attr);
    init.property(js_string!("m11"), JsValue::from(a), attr);
    init.property(js_string!("m12"), JsValue::from(b), attr);
    init.property(js_string!("m13"), JsValue::from(0.0), attr);
    init.property(js_string!("m14"), JsValue::from(0.0), attr);
    init.property(js_string!("m21"), JsValue::from(c), attr);
    init.property(js_string!("m22"), JsValue::from(d), attr);
    init.property(js_string!("m23"), JsValue::from(0.0), attr);
    init.property(js_string!("m24"), JsValue::from(0.0), attr);
    init.property(js_string!("m31"), JsValue::from(0.0), attr);
    init.property(js_string!("m32"), JsValue::from(0.0), attr);
    init.property(js_string!("m33"), JsValue::from(1.0), attr);
    init.property(js_string!("m34"), JsValue::from(0.0), attr);
    init.property(js_string!("m41"), JsValue::from(e), attr);
    init.property(js_string!("m42"), JsValue::from(f), attr);
    init.property(js_string!("m43"), JsValue::from(0.0), attr);
    init.property(js_string!("m44"), JsValue::from(1.0), attr);
    let is_identity = (a - 1.0).abs() < f64::EPSILON
        && b.abs() < f64::EPSILON
        && c.abs() < f64::EPSILON
        && (d - 1.0).abs() < f64::EPSILON
        && e.abs() < f64::EPSILON
        && f.abs() < f64::EPSILON;
    init.property(
        js_string!("is2D"),
        JsValue::from(true),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    init.property(
        js_string!("isIdentity"),
        JsValue::from(is_identity),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    // transformPoint on the result matrix.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let dict = args.first().cloned().unwrap_or(JsValue::undefined());
            let (px, py, pz, pw) = extract_point_dict(&dict, ctx)?;
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("DOMMatrix: this is not an object")
            })?;
            let ma = obj.get(js_string!("a"), ctx)?.to_number(ctx).unwrap_or(1.0);
            let mb = obj.get(js_string!("b"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let mc = obj.get(js_string!("c"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let md = obj.get(js_string!("d"), ctx)?.to_number(ctx).unwrap_or(1.0);
            let me = obj.get(js_string!("e"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let mf = obj.get(js_string!("f"), ctx)?.to_number(ctx).unwrap_or(0.0);
            let rx = ma * px + mc * py + me * pw;
            let ry = mb * px + md * py + mf * pw;
            Ok(JsValue::from(build_dom_point(rx, ry, pz, pw, true, ctx)?))
        }),
        js_string!("transformPoint"),
        0,
    );
    Ok(init.build())
}

// ---------------------------------------------------------------------------
// Visual Viewport (CSSWG Visual Viewport §4)
// ---------------------------------------------------------------------------

/// Register `window.visualViewport` object.
fn register_visual_viewport(ctx: &mut Context, bridge: &HostBridge) {
    use boa_engine::property::PropertyDescriptorBuilder;

    let global = ctx.global_object();

    let b = bridge.clone();
    let realm = ctx.realm().clone();

    // Build the visualViewport object with dynamic getters.
    let mut init = ObjectInitializer::new(ctx);

    // width — same as innerWidth.
    let b_w = b.clone();
    let w_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.viewport_width() as f64))
        },
        b_w,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("width"),
        Some(w_getter.clone()),
        None,
        Attribute::CONFIGURABLE,
    );

    // height — same as innerHeight.
    let b_h = b.clone();
    let h_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(bridge.viewport_height() as f64))
        },
        b_h,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("height"),
        Some(h_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // offsetLeft, offsetTop — offset of visual viewport from layout viewport.
    init.property(
        js_string!("offsetLeft"),
        JsValue::from(0.0),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    init.property(
        js_string!("offsetTop"),
        JsValue::from(0.0),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    // pageLeft, pageTop — offset relative to page origin.
    let b_pl = b.clone();
    let pl_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.scroll_x()))),
        b_pl,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("pageLeft"),
        Some(pl_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    let b_pt = b.clone();
    let pt_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(f64::from(bridge.scroll_y()))),
        b_pt,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("pageTop"),
        Some(pt_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // scale — pinch-zoom scale factor (1.0 = no zoom).
    init.property(
        js_string!("scale"),
        JsValue::from(1.0),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    // addEventListener / removeEventListener stubs.
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("addEventListener"),
        2,
    );
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("removeEventListener"),
        2,
    );

    let vv = init.build();

    let desc = PropertyDescriptorBuilder::new()
        .value(vv)
        .writable(false)
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("visualViewport"), desc, ctx)
        .expect("failed to register visualViewport");
}

// ---------------------------------------------------------------------------
// DOMParser (WHATWG HTML §8.4)
// ---------------------------------------------------------------------------

/// Register `DOMParser` constructor.
///
/// `DOMParser` is a global constructor that provides `parseFromString(string, mimeType)`.
/// Since elidex uses a single `EcsDom`, the implementation creates a temporary container
/// element, sets its `innerHTML`, and returns a document-like wrapper object with
/// `querySelector`, `querySelectorAll`, `body`, `head`, and `documentElement` accessors.
fn register_dom_parser(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    ctx.register_global_callable(
        js_string!("DOMParser"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| {
                let mut init = ObjectInitializer::new(ctx);

                // parseFromString(string, mimeType) → document-like object
                init.function(
                    NativeFunction::from_copy_closure_with_captures(
                        |_this, args, bridge, ctx| {
                            let markup =
                                require_js_string_arg(args, 0, "DOMParser.parseFromString", ctx)?;
                            let mime =
                                require_js_string_arg(args, 1, "DOMParser.parseFromString", ctx)?;

                            // Validate MIME type.
                            match mime.as_str() {
                                "text/html"
                                | "text/xml"
                                | "application/xml"
                                | "application/xhtml+xml"
                                | "image/svg+xml" => {}
                                _ => {
                                    return Err(JsNativeError::typ()
                                        .with_message(format!(
                                            "DOMParser.parseFromString: unsupported MIME type '{mime}'"
                                        ))
                                        .into());
                                }
                            }

                            // Create a temporary container and set innerHTML.
                            // We record the mutation then flush immediately so the
                            // DOM tree is available for querySelector.
                            let container_entity = bridge.with(|_session, dom| {
                                dom.create_element("div", elidex_ecs::Attributes::default())
                            });

                            // Use the innerHTML.set handler to record the mutation,
                            // then flush to apply it to the DOM immediately.
                            bridge.with(|session, dom| {
                                if let Some(handler) =
                                    bridge.dom_registry().resolve("innerHTML.set")
                                {
                                    let _ = handler.invoke(
                                        container_entity,
                                        &[ElidexJsValue::String(markup)],
                                        session,
                                        dom,
                                    );
                                }
                                // Flush mutations so the parsed nodes are in the DOM.
                                session.flush(dom);
                            });

                            // Build a document-like wrapper object.
                            build_parsed_document(container_entity, bridge, ctx)
                        },
                        bridge.clone(),
                    ),
                    js_string!("parseFromString"),
                    2,
                );

                Ok(JsValue::from(init.build()))
            },
            b,
        ),
    )
    .expect("failed to register DOMParser");
}

/// Build a document-like wrapper for `DOMParser.parseFromString()`.
///
/// The wrapper delegates `querySelector`/`querySelectorAll` to the container element
/// and provides `body`, `head`, and `documentElement` getters.
#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
// Sequential property/method registration on a single JS object.
fn build_parsed_document(
    container: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let realm = ctx.realm().clone();
    let mut init = ObjectInitializer::new(ctx);
    let container_bits = container.to_bits().get() as f64;

    // querySelector(selector)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, (bridge, bits), ctx| {
                let entity = Entity::from_bits(*bits as u64).ok_or_else(|| {
                    JsNativeError::typ().with_message("invalid entity")
                })?;
                let selector = require_js_string_arg(args, 0, "querySelector", ctx)?;
                invoke_dom_handler_ref(
                    "querySelector",
                    entity,
                    &[ElidexJsValue::String(selector)],
                    bridge,
                    ctx,
                )
            },
            (b, container_bits),
        ),
        js_string!("querySelector"),
        1,
    );

    // querySelectorAll(selector) → array
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, (bridge, bits), ctx| {
                let entity = Entity::from_bits(*bits as u64).ok_or_else(|| {
                    JsNativeError::typ().with_message("invalid entity")
                })?;
                let selector = require_js_string_arg(args, 0, "querySelectorAll", ctx)?;
                let entities = bridge.with(|_session, dom| {
                    elidex_dom_api::query_selector_all(entity, &selector, dom)
                        .map_err(crate::error_conv::dom_error_to_js_error)
                })?;
                Ok(crate::globals::document::entities_to_js_array(
                    &entities, bridge, ctx,
                ))
            },
            (b, container_bits),
        ),
        js_string!("querySelectorAll"),
        1,
    );

    // getElementById(id)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, (bridge, bits), ctx| {
                let entity = Entity::from_bits(*bits as u64).ok_or_else(|| {
                    JsNativeError::typ().with_message("invalid entity")
                })?;
                let id = require_js_string_arg(args, 0, "getElementById", ctx)?;
                invoke_dom_handler_ref(
                    "getElementById",
                    entity,
                    &[ElidexJsValue::String(id)],
                    bridge,
                    ctx,
                )
            },
            (b, container_bits),
        ),
        js_string!("getElementById"),
        1,
    );

    // documentElement — getter (returns first child element of container).
    let b = bridge.clone();
    let doc_elem_getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, (bridge, bits), ctx| {
            let entity = Entity::from_bits(*bits as u64).ok_or_else(|| {
                JsNativeError::typ().with_message("invalid entity")
            })?;
            let first_elem = bridge.with(|_session, dom| {
                dom.children_iter(entity).find(|&child| {
                    dom.world()
                        .get::<&elidex_ecs::TagType>(child)
                        .is_ok()
                })
            });
            match first_elem {
                Some(e) => {
                    let wrapper = bridge.with(|session, _dom| {
                        let obj_ref = session.get_or_create_wrapper(
                            e,
                            elidex_script_session::ComponentKind::Element,
                        );
                        crate::globals::element::create_element_wrapper(
                            e, bridge, obj_ref, ctx, false,
                        )
                    });
                    Ok(wrapper)
                }
                None => Ok(JsValue::null()),
            }
        },
        (b, container_bits),
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("documentElement"),
        Some(doc_elem_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // body — getter (returns first <body> descendant).
    let b = bridge.clone();
    let body_getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, (bridge, bits), ctx| {
            let entity = Entity::from_bits(*bits as u64).ok_or_else(|| {
                JsNativeError::typ().with_message("invalid entity")
            })?;
            let body_entity = bridge.with(|_session, dom| {
                find_first_tag_descendant(dom, entity, "body")
            });
            match body_entity {
                Some(e) => {
                    let wrapper = bridge.with(|session, _dom| {
                        let obj_ref = session.get_or_create_wrapper(
                            e,
                            elidex_script_session::ComponentKind::Element,
                        );
                        crate::globals::element::create_element_wrapper(
                            e, bridge, obj_ref, ctx, false,
                        )
                    });
                    Ok(wrapper)
                }
                None => Ok(JsValue::null()),
            }
        },
        (b, container_bits),
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("body"),
        Some(body_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // head — getter (returns first <head> descendant).
    let b = bridge.clone();
    let head_getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, (bridge, bits), ctx| {
            let entity = Entity::from_bits(*bits as u64).ok_or_else(|| {
                JsNativeError::typ().with_message("invalid entity")
            })?;
            let head_entity = bridge.with(|_session, dom| {
                find_first_tag_descendant(dom, entity, "head")
            });
            match head_entity {
                Some(e) => {
                    let wrapper = bridge.with(|session, _dom| {
                        let obj_ref = session.get_or_create_wrapper(
                            e,
                            elidex_script_session::ComponentKind::Element,
                        );
                        crate::globals::element::create_element_wrapper(
                            e, bridge, obj_ref, ctx, false,
                        )
                    });
                    Ok(wrapper)
                }
                None => Ok(JsValue::null()),
            }
        },
        (b, container_bits),
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("head"),
        Some(head_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    Ok(JsValue::from(init.build()))
}

/// Find the first descendant element with the given tag name (depth-first).
fn find_first_tag_descendant(
    dom: &elidex_ecs::EcsDom,
    root: Entity,
    tag: &str,
) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if entity != root {
            if let Ok(t) = dom.world().get::<&elidex_ecs::TagType>(entity) {
                if t.0 == tag {
                    return Some(entity);
                }
            }
        }
        // Push children in reverse for depth-first left-to-right traversal.
        let mut children = Vec::new();
        let mut child = dom.get_first_child(entity);
        while let Some(c) = child {
            children.push(c);
            child = dom.get_next_sibling(c);
        }
        stack.extend(children.into_iter().rev());
    }
    None
}

// ---------------------------------------------------------------------------
// XMLSerializer (WHATWG DOM Parsing §3.2)
// ---------------------------------------------------------------------------

/// Register `XMLSerializer` constructor.
///
/// `XMLSerializer` provides `serializeToString(node)` which returns the outerHTML
/// of an element node or the textContent of a text node.
fn register_xml_serializer(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    ctx.register_global_callable(
        js_string!("XMLSerializer"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| {
                let mut init = ObjectInitializer::new(ctx);

                // serializeToString(node) → string
                init.function(
                    NativeFunction::from_copy_closure_with_captures(
                        |_this, args, bridge, ctx| {
                            let node_val = args.first().ok_or_else(|| {
                                JsNativeError::typ().with_message(
                                    "XMLSerializer.serializeToString: argument 1 is required",
                                )
                            })?;

                            let entity = extract_entity(node_val, ctx)?;

                            // Check if it is a text node (has TextContent but no TagType).
                            let result = bridge.with(|session, dom| {
                                let is_text = dom
                                    .world()
                                    .get::<&elidex_ecs::TagType>(entity)
                                    .is_err()
                                    && dom
                                        .world()
                                        .get::<&elidex_ecs::TextContent>(entity)
                                        .is_ok();

                                if is_text {
                                    // Return text content directly.
                                    dom.world()
                                        .get::<&elidex_ecs::TextContent>(entity)
                                        .map(|tc| tc.0.clone())
                                        .unwrap_or_default()
                                } else {
                                    // Build outerHTML: opening tag + innerHTML + closing tag.
                                    let tag_name = dom
                                        .world()
                                        .get::<&elidex_ecs::TagType>(entity)
                                        .map_or_else(|_| "div".to_string(), |t| t.0.clone());

                                    let attrs_str = dom
                                        .world()
                                        .get::<&elidex_ecs::Attributes>(entity)
                                        .ok()
                                        .map(|attrs| {
                                            let mut s = String::new();
                                            for (k, v) in attrs.iter() {
                                                s.push(' ');
                                                s.push_str(k);
                                                s.push_str("=\"");
                                                s.push_str(
                                                    &v.replace('&', "&amp;")
                                                        .replace('"', "&quot;"),
                                                );
                                                s.push('"');
                                            }
                                            s
                                        })
                                        .unwrap_or_default();

                                    // Get innerHTML via handler.
                                    let inner = bridge
                                        .dom_registry()
                                        .resolve("innerHTML.get")
                                        .and_then(|h| {
                                            h.invoke(entity, &[], session, dom).ok()
                                        })
                                        .and_then(|v| match v {
                                            elidex_plugin::JsValue::String(s) => Some(s),
                                            _ => None,
                                        })
                                        .unwrap_or_default();

                                    format!("<{tag_name}{attrs_str}>{inner}</{tag_name}>")
                                }
                            });

                            Ok(JsValue::from(js_string!(result.as_str())))
                        },
                        bridge.clone(),
                    ),
                    js_string!("serializeToString"),
                    1,
                );

                Ok(JsValue::from(init.build()))
            },
            b,
        ),
    )
    .expect("failed to register XMLSerializer");
}

// ---------------------------------------------------------------------------
// requestIdleCallback / cancelIdleCallback (W3C requestIdleCallback §2)
// ---------------------------------------------------------------------------

/// Register `requestIdleCallback` and `cancelIdleCallback` global functions.
///
/// Simplified implementation: uses `setTimeout(wrappedCallback, timeout || 0)`
/// as an approximation. The callback receives an `IdleDeadline` object with
/// `timeRemaining()` (returns 50ms) and `didTimeout` (boolean).
fn register_idle_callbacks(ctx: &mut Context) {
    // requestIdleCallback(callback, options?) → id
    let ric_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
            JsNativeError::typ()
                .with_message("requestIdleCallback: argument 1 must be a function")
        })?;

        // Extract optional timeout from options.
        let timeout_ms = args
            .get(1)
            .and_then(JsValue::as_object)
            .and_then(|obj| {
                obj.get(js_string!("timeout"), ctx)
                    .ok()
                    .and_then(|v| v.as_number())
            })
            .unwrap_or(0.0);

        let delay = if timeout_ms > 0.0 {
            timeout_ms
        } else {
            0.0
        };

        // Build IdleDeadline object.
        let did_timeout = delay > 0.0;

        // Wrap the callback to provide IdleDeadline.
        let wrapped = NativeFunction::from_copy_closure_with_captures(
            move |_this, _args, (cb, did_to), ctx| {
                let mut deadline_init = ObjectInitializer::new(ctx);

                // Simplified: always report 50ms remaining (idle budget).
                let remaining = 50.0_f64;

                deadline_init.function(
                    NativeFunction::from_copy_closure_with_captures(
                        |_this, _args, remaining, _ctx| Ok(JsValue::from(*remaining)),
                        remaining,
                    ),
                    js_string!("timeRemaining"),
                    0,
                );
                deadline_init.property(
                    js_string!("didTimeout"),
                    JsValue::from(*did_to),
                    Attribute::READONLY | Attribute::CONFIGURABLE,
                );

                let deadline = deadline_init.build();
                let _ = cb.call(&JsValue::undefined(), &[JsValue::from(deadline)], ctx);
                Ok(JsValue::undefined())
            },
            (callback.clone(), did_timeout),
        )
        .to_js_function(ctx.realm());

        // Use setTimeout to schedule.
        let global = ctx.global_object();
        let set_timeout = global.get(js_string!("setTimeout"), ctx)?;
        let result = set_timeout.as_callable().ok_or_else(|| {
            JsNativeError::typ().with_message("setTimeout not found")
        })?.call(
            &JsValue::undefined(),
            &[JsValue::from(wrapped), JsValue::from(delay)],
            ctx,
        )?;

        Ok(result)
    });
    ctx.register_global_builtin_callable(js_string!("requestIdleCallback"), 1, ric_fn)
        .expect("failed to register requestIdleCallback");

    // cancelIdleCallback(id) — delegates to clearTimeout.
    let cic_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = args.first().cloned().unwrap_or(JsValue::undefined());
        let global = ctx.global_object();
        let clear_timeout = global.get(js_string!("clearTimeout"), ctx)?;
        if let Some(callable) = clear_timeout.as_callable() {
            callable.call(&JsValue::undefined(), &[id], ctx)?;
        }
        Ok(JsValue::undefined())
    });
    ctx.register_global_builtin_callable(js_string!("cancelIdleCallback"), 1, cic_fn)
        .expect("failed to register cancelIdleCallback");
}

// ---------------------------------------------------------------------------
// structuredClone (WHATWG HTML §2.7.6)
// ---------------------------------------------------------------------------

/// Register `structuredClone(value, options?)` global function.
///
/// Simplified implementation using JSON roundtrip (`JSON.stringify` + `JSON.parse`).
/// Handles plain objects, arrays, strings, numbers, booleans, null.
/// Transfer option is accepted but ignored (transferables not supported).
fn register_structured_clone(ctx: &mut Context) {
    let sc_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let value = args.first().cloned().unwrap_or(JsValue::undefined());

        // Handle primitives directly (no need for JSON roundtrip).
        if value.is_undefined() {
            return Ok(JsValue::undefined());
        }
        if value.is_null() {
            return Ok(JsValue::null());
        }
        if let Some(b) = value.as_boolean() {
            return Ok(JsValue::from(b));
        }
        if let Some(n) = value.as_number() {
            return Ok(JsValue::from(n));
        }
        if value.is_string() {
            // Strings are immutable, but clone for spec correctness.
            return Ok(value);
        }

        // For objects/arrays: JSON roundtrip.
        let global = ctx.global_object();
        let json = global.get(js_string!("JSON"), ctx)?;
        let json_obj = json.as_object().ok_or_else(|| {
            JsNativeError::typ().with_message("JSON global not found")
        })?;

        let stringify = json_obj.get(js_string!("stringify"), ctx)?;
        let parse = json_obj.get(js_string!("parse"), ctx)?;

        let stringify_fn = stringify.as_callable().ok_or_else(|| {
            JsNativeError::typ().with_message("JSON.stringify is not callable")
        })?;
        let parse_fn = parse.as_callable().ok_or_else(|| {
            JsNativeError::typ().with_message("JSON.parse is not callable")
        })?;

        let json_str = stringify_fn.call(&json, &[value], ctx)?;

        // If stringify returns undefined (e.g. for functions, symbols), throw.
        if json_str.is_undefined() {
            return Err(JsNativeError::eval()
                .with_message("DataCloneError: value could not be cloned")
                .into());
        }

        parse_fn.call(&json, &[json_str], ctx)
    });
    ctx.register_global_builtin_callable(js_string!("structuredClone"), 1, sc_fn)
        .expect("failed to register structuredClone");
}

/// Check if a URL is potentially trustworthy (WHATWG Secure Contexts §3.1).
///
/// A URL is potentially trustworthy if it uses `https`, `file`, or is localhost.
fn is_potentially_trustworthy_url(url: &url::Url) -> bool {
    let scheme = url.scheme();
    if scheme == "https" || scheme == "file" {
        return true;
    }
    url.host_str().is_some_and(|host| {
        host == "localhost"
            || host.ends_with(".localhost")
            || host == "127.0.0.1"
            || host.starts_with("127.")
            || host == "[::1]"
    })
}
