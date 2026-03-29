//! `window` global and related registrations.

mod computed_style;
mod dom_parser;
mod encoding;
mod geometry;
mod media_query;
mod performance;
mod screen;
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
    // getComputedStyle(element) -> returns object with property getters
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
    screen::register_screen_and_window_props(ctx, bridge);
    performance::register_performance(ctx, bridge);
    encoding::register_atob_btoa(ctx);
    encoding::register_crypto(ctx);
    encoding::register_queue_microtask(ctx);
    register_image_constructor(ctx, bridge);
    geometry::register_dom_geometry(ctx);
    geometry::register_visual_viewport(ctx, bridge);
    dom_parser::register_dom_parser(ctx, bridge);
    dom_parser::register_xml_serializer(ctx, bridge);
    dom_parser::register_idle_callbacks(ctx);
    dom_parser::register_structured_clone(ctx);
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
                let handler = bridge
                    .dom_registry()
                    .resolve("createElement")
                    .ok_or_else(|| {
                        boa_engine::JsNativeError::typ()
                            .with_message("createElement handler not found")
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
                let wrapper = crate::globals::element::resolve_object_ref(&result, bridge, ctx);

                // Set width/height content attributes if provided.
                if let Some(entity) = wrapper
                    .as_object()
                    .and_then(|_| crate::globals::element::extract_entity(&wrapper, ctx).ok())
                {
                    if let Some(w) = args.first().and_then(JsValue::as_number) {
                        bridge.with(|_session, dom| {
                            if let Ok(mut attrs) =
                                dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity)
                            {
                                #[allow(clippy::cast_possible_truncation)]
                                attrs.set("width", (w as i64).to_string());
                            }
                        });
                    }
                    if let Some(h) = args.get(1).and_then(JsValue::as_number) {
                        bridge.with(|_session, dom| {
                            if let Ok(mut attrs) =
                                dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity)
                            {
                                #[allow(clippy::cast_possible_truncation)]
                                attrs.set("height", (h as i64).to_string());
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
            let x = opt_x.unwrap_or_else(|| f64::from(bridge.scroll_x()));
            let y = opt_y.unwrap_or_else(|| f64::from(bridge.scroll_y()));
            if x.is_finite() && y.is_finite() {
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
    ctx.register_global_property(
        js_string!("parent"),
        JsValue::from(ctx.global_object()),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.parent");

    ctx.register_global_property(
        js_string!("top"),
        JsValue::from(ctx.global_object()),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.top");

    ctx.register_global_property(
        js_string!("frameElement"),
        JsValue::null(),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.frameElement");

    ctx.register_global_property(
        js_string!("length"),
        JsValue::from(0),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.length");

    ctx.register_global_property(
        js_string!("opener"),
        JsValue::null(),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.opener");

    ctx.register_global_property(
        js_string!("frames"),
        JsValue::from(ctx.global_object()),
        Attribute::CONFIGURABLE,
    )
    .expect("failed to register window.frames");
}

/// Register `window.open(url, target, features)` (WHATWG HTML §7.5.2).
fn register_window_open(ctx: &mut Context, bridge: &HostBridge) {
    let b_open = bridge.clone();
    let open_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> {
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

            let resolved = if url_str.trim().is_empty() {
                Ok(url::Url::parse("about:blank").expect("about:blank is valid"))
            } else {
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
                        bridge.queue_open_tab(url);
                    }
                    "_self" => {
                        bridge.set_pending_navigation(elidex_navigation::NavigationRequest {
                            url: url.to_string(),
                            replace: false,
                        });
                    }
                    "_parent" | "_top" => {
                        if bridge.sandbox_flags().is_some_and(|f| {
                            !f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_TOP_NAVIGATION)
                        }) {
                            return Ok(JsValue::null());
                        }
                        bridge.set_pending_navigation(elidex_navigation::NavigationRequest {
                            url: url.to_string(),
                            replace: false,
                        });
                    }
                    named => {
                        bridge.set_pending_navigate_iframe(named.to_string(), url);
                    }
                }
            }

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

            let own_origin = bridge.origin();
            let own_serialized = own_origin.serialize();
            let origin_matches =
                target_origin == "*" || target_origin == "/" || own_serialized == target_origin;

            if origin_matches {
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
            Ok(JsValue::null())
        },
        b_prompt,
    );
    ctx.register_global_builtin_callable(js_string!("prompt"), 1, prompt_fn)
        .expect("failed to register prompt");
}

/// Parse scroll arguments: either `(x, y)` numbers or `{top, left}` options object.
fn parse_scroll_args(args: &[JsValue], ctx: &mut Context) -> JsResult<(Option<f64>, Option<f64>)> {
    if let Some(first) = args.first() {
        if let Some(obj) = first.as_object() {
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
    let b_css_set = bridge.clone();
    let css_text_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());

            let existing_props: Vec<String> = bridge.with(|_session, dom| {
                dom.world()
                    .get::<&elidex_ecs::InlineStyle>(entity)
                    .ok()
                    .map_or_else(Vec::new, |style| {
                        style.iter().map(|(k, _)| k.to_string()).collect()
                    })
            });

            for prop in &existing_props {
                let _ = invoke_dom_handler_void(
                    "style.removeProperty",
                    entity,
                    &[ElidexJsValue::String(prop.clone())],
                    bridge,
                );
            }

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

/// Check if a URL is potentially trustworthy (WHATWG Secure Contexts §3.1).
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
