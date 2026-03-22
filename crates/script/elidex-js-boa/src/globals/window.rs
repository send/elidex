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
#[allow(clippy::similar_names)]
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

    // innerWidth / innerHeight — dynamic getters that read from bridge.
    let global = ctx.global_object();
    {
        use boa_engine::property::PropertyDescriptorBuilder;

        let b_iw = b.clone();
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

        let b_ih = b.clone();
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
        let b_sx = b.clone();
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

        let b_sy = b.clone();
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

    // scrollTo(x, y) / scrollTo({top, left, behavior})
    let b_scroll = b.clone();
    let scroll_to = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let (x, y) = parse_scroll_args(args, ctx)?;
            // Directly update the bridge's scroll offset. The content thread
            // will pick up the new scroll position on the next render frame.
            #[allow(clippy::cast_possible_truncation)]
            bridge.set_scroll_offset(x as f32, y as f32);
            Ok(JsValue::undefined())
        },
        b_scroll,
    );
    ctx.register_global_builtin_callable(js_string!("scrollTo"), 2, scroll_to)
        .expect("failed to register scrollTo");

    // scrollBy(x, y) — adds delta to current scroll offset.
    let b_scroll_by = b.clone();
    let scroll_by = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let (x, y) = parse_scroll_args(args, ctx)?;
            let cur_x = f64::from(bridge.scroll_x());
            let cur_y = f64::from(bridge.scroll_y());
            #[allow(clippy::cast_possible_truncation)]
            bridge.set_scroll_offset((cur_x + x) as f32, (cur_y + y) as f32);
            Ok(JsValue::undefined())
        },
        b_scroll_by,
    );
    ctx.register_global_builtin_callable(js_string!("scrollBy"), 2, scroll_by)
        .expect("failed to register scrollBy");

    // matchMedia(query) — returns a MediaQueryList-like object.
    // Supports basic (min-width), (max-width), (min-height), (max-height) evaluation.
    // Listeners registered via addEventListener("change", cb) are dispatched on viewport resize.
    let b_mm = b.clone();
    let match_media = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let query = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());

            let matches = evaluate_media_query(&query, bridge);
            let mq_id = bridge.create_media_query(&query, matches);

            build_media_query_list_object(mq_id, &query, bridge, ctx)
        },
        b_mm,
    );
    ctx.register_global_builtin_callable(js_string!("matchMedia"), 1, match_media)
        .expect("failed to register matchMedia");

    // window.getSelection() → returns a Selection object integrated with Range.
    let b_sel = b.clone();
    let get_selection = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, ctx| {
            let has_range = bridge.selection_range_id().is_some();
            let range_count = i32::from(has_range);

            let mut obj = ObjectInitializer::new(ctx);

            let sel_type = if has_range {
                js_string!("Range")
            } else {
                js_string!("None")
            };
            obj.property(
                js_string!("type"),
                JsValue::from(sel_type),
                Attribute::READONLY,
            );
            obj.property(
                js_string!("rangeCount"),
                JsValue::from(range_count),
                Attribute::READONLY,
            );

            // isCollapsed — read from the underlying Range if present.
            let collapsed = bridge
                .selection_range_id()
                .is_none_or(|rid| bridge.with_range(rid, |r| r.collapsed()).unwrap_or(true));
            obj.property(
                js_string!("isCollapsed"),
                JsValue::from(collapsed),
                Attribute::READONLY,
            );

            // toString() → return selected text from the underlying Range, if any.
            let b_tostr = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, _args, bridge, _ctx| {
                        let text = bridge.selection_range_id().and_then(|rid| {
                            bridge
                                .with(|_session, dom| bridge.with_range(rid, |r| r.to_string(dom)))
                        });
                        Ok(JsValue::from(js_string!(text.unwrap_or_default().as_str())))
                    },
                    b_tostr,
                ),
                js_string!("toString"),
                0,
            );

            // getRangeAt(index) → returns the Range JS object if selection exists.
            let b_gra = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, args, bridge, ctx| {
                        let index = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
                        if index as u32 != 0 {
                            return Err(boa_engine::JsNativeError::range()
                                .with_message("index out of range")
                                .into());
                        }
                        match bridge.selection_range_id() {
                            Some(rid) => {
                                crate::globals::document::build_range_object(rid, bridge, ctx)
                            }
                            None => Err(boa_engine::JsNativeError::range()
                                .with_message("no range in selection")
                                .into()),
                        }
                    },
                    b_gra,
                ),
                js_string!("getRangeAt"),
                1,
            );

            // addRange(range) → store range_id in bridge.selection_range_id.
            let b_ar = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, args, bridge, ctx| {
                        if let Some(range_obj) = args.first().and_then(JsValue::as_object) {
                            if let Ok(id_val) =
                                range_obj.get(js_string!("__elidex_traversal_id__"), ctx)
                            {
                                if let Some(id) = id_val.as_number() {
                                    #[allow(
                                        clippy::cast_possible_truncation,
                                        clippy::cast_sign_loss
                                    )]
                                    bridge.set_selection_range_id(Some(id as u64));
                                }
                            }
                        }
                        Ok(JsValue::undefined())
                    },
                    b_ar,
                ),
                js_string!("addRange"),
                1,
            );

            // removeAllRanges() → clears selection_range_id.
            let b_rar = bridge.clone();
            obj.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, _args, bridge, _ctx| {
                        bridge.set_selection_range_id(None);
                        Ok(JsValue::undefined())
                    },
                    b_rar,
                ),
                js_string!("removeAllRanges"),
                0,
            );

            Ok(obj.build().into())
        },
        b_sel,
    );
    ctx.register_global_builtin_callable(js_string!("getSelection"), 0, get_selection)
        .expect("failed to register getSelection");
}

/// Evaluate a basic media query string against the current viewport.
///
/// Supports:
/// - `(max-width: Npx)` / `(min-width: Npx)`
/// - `(max-height: Npx)` / `(min-height: Npx)`
/// - `(prefers-color-scheme: dark|light)` → false (no theme support yet)
/// - Other queries → false
fn evaluate_media_query(query: &str, bridge: &HostBridge) -> bool {
    crate::bridge::evaluate_media_query_raw(
        query,
        bridge.viewport_width(),
        bridge.viewport_height(),
    )
}

/// Hidden property key for the media query list ID.
const MQ_ID_KEY: &str = "__elidex_mq_id__";

/// Build a `MediaQueryList`-like JS object with dynamic `matches` getter
/// and `addEventListener`/`removeEventListener` for "change" events.
#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
fn build_media_query_list_object(
    mq_id: u64,
    query: &str,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut obj = ObjectInitializer::new(ctx);

    // Store the media query ID as a hidden property.
    #[allow(clippy::cast_precision_loss)]
    obj.property(
        js_string!(MQ_ID_KEY),
        JsValue::from(mq_id as f64),
        Attribute::empty(),
    );

    obj.property(
        js_string!("media"),
        JsValue::from(js_string!(query)),
        Attribute::READONLY,
    );

    // matches — dynamic getter that re-evaluates against current viewport.
    let realm = obj.context().realm().clone();
    let b_matches = bridge.clone();
    let matches_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let id = extract_mq_id(this, ctx)?;
            Ok(JsValue::from(bridge.media_query_matches(id)))
        },
        b_matches,
    )
    .to_js_function(&realm);
    obj.accessor(
        js_string!("matches"),
        Some(matches_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // addEventListener(type, callback)
    let b_add = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or(String::new(), |s| s.to_std_string_escaped());
                if event_type == "change" {
                    if let Some(callback) = args.get(1).and_then(JsValue::as_object) {
                        let id = extract_mq_id(this, ctx)?;
                        bridge.add_media_query_listener(id, callback.clone());
                    }
                }
                Ok(JsValue::undefined())
            },
            b_add,
        ),
        js_string!("addEventListener"),
        2,
    );

    // removeEventListener(type, callback)
    let b_rm = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or(String::new(), |s| s.to_std_string_escaped());
                if event_type == "change" {
                    if let Some(callback) = args.get(1).and_then(JsValue::as_object) {
                        let id = extract_mq_id(this, ctx)?;
                        bridge.remove_media_query_listener(id, &callback);
                    }
                }
                Ok(JsValue::undefined())
            },
            b_rm,
        ),
        js_string!("removeEventListener"),
        2,
    );

    // Legacy aliases: addListener / removeListener (CSSOM View spec §4.2)
    let b_add_legacy = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                if let Some(callback) = args.first().and_then(JsValue::as_object) {
                    let id = extract_mq_id(this, ctx)?;
                    bridge.add_media_query_listener(id, callback.clone());
                }
                Ok(JsValue::undefined())
            },
            b_add_legacy,
        ),
        js_string!("addListener"),
        1,
    );

    let b_rm_legacy = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                if let Some(callback) = args.first().and_then(JsValue::as_object) {
                    let id = extract_mq_id(this, ctx)?;
                    bridge.remove_media_query_listener(id, &callback);
                }
                Ok(JsValue::undefined())
            },
            b_rm_legacy,
        ),
        js_string!("removeListener"),
        1,
    );

    Ok(obj.build().into())
}

/// Extract the media query ID from a JS object's hidden property.
fn extract_mq_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this.as_object().ok_or_else(|| {
        boa_engine::JsNativeError::typ().with_message("matchMedia method called on non-object")
    })?;
    let id_val = obj.get(js_string!(MQ_ID_KEY), ctx)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let id = id_val.as_number().ok_or_else(|| {
        boa_engine::JsNativeError::typ().with_message("invalid MediaQueryList object")
    })? as u64;
    Ok(id)
}

/// Parse scroll arguments: either `(x, y)` numbers or `{top, left}` options object.
fn parse_scroll_args(args: &[JsValue], ctx: &mut Context) -> JsResult<(f64, f64)> {
    if let Some(first) = args.first() {
        if let Some(obj) = first.as_object() {
            // Options object: { top, left, behavior }
            let top_val = obj.get(js_string!("top"), ctx)?;
            let top = if top_val.is_undefined() {
                0.0
            } else {
                top_val.to_number(ctx)?
            };
            let left_val = obj.get(js_string!("left"), ctx)?;
            let left = if left_val.is_undefined() {
                0.0
            } else {
                left_val.to_number(ctx)?
            };
            return Ok((left, top));
        }
        // Numeric arguments: scrollTo(x, y)
        let x = first.to_number(ctx)?;
        let y = if let Some(v) = args.get(1) {
            v.to_number(ctx)?
        } else {
            0.0
        };
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
    let b_css_set = bridge.clone();
    let css_text_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());
            bridge.with(|_session, dom| {
                // Clear existing inline style, then set new properties.
                let _ = dom
                    .world_mut()
                    .insert_one(entity, elidex_ecs::InlineStyle::default());
                let mut style = dom
                    .world_mut()
                    .get::<&mut elidex_ecs::InlineStyle>(entity)
                    .ok();
                if let Some(ref mut s) = style {
                    for decl in text.split(';') {
                        let decl = decl.trim();
                        if let Some((prop, val)) = decl.split_once(':') {
                            s.set(prop.trim().to_string(), val.trim().to_string());
                        }
                    }
                }
            });
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
