//! `DOMParser`, `XMLSerializer`, `requestIdleCallback`, `structuredClone` registrations.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::extract_entity;
use crate::globals::{invoke_dom_handler_ref, require_js_string_arg};

/// Register `DOMParser` constructor.
///
/// `DOMParser` is a global constructor that provides `parseFromString(string, mimeType)`.
/// Since elidex uses a single `EcsDom`, the implementation creates a temporary container
/// element, sets its `innerHTML`, and returns a document-like wrapper object with
/// `querySelector`, `querySelectorAll`, `body`, `head`, and `documentElement` accessors.
pub(super) fn register_dom_parser(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    ctx.register_global_callable(
        js_string!("DOMParser"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| {
                let mut init = ObjectInitializer::new(ctx);

                // parseFromString(string, mimeType) -> document-like object
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
                            let container_entity = bridge.with(|_session, dom| {
                                dom.create_element("div", elidex_ecs::Attributes::default())
                            });

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
#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
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
                let entity = Entity::from_bits(*bits as u64)
                    .ok_or_else(|| JsNativeError::typ().with_message("invalid entity"))?;
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

    // querySelectorAll(selector) -> array
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, (bridge, bits), ctx| {
                let entity = Entity::from_bits(*bits as u64)
                    .ok_or_else(|| JsNativeError::typ().with_message("invalid entity"))?;
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
                let entity = Entity::from_bits(*bits as u64)
                    .ok_or_else(|| JsNativeError::typ().with_message("invalid entity"))?;
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
            let entity = Entity::from_bits(*bits as u64)
                .ok_or_else(|| JsNativeError::typ().with_message("invalid entity"))?;
            let first_elem = bridge.with(|_session, dom| {
                dom.children_iter(entity)
                    .find(|&child| dom.world().get::<&elidex_ecs::TagType>(child).is_ok())
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
            let entity = Entity::from_bits(*bits as u64)
                .ok_or_else(|| JsNativeError::typ().with_message("invalid entity"))?;
            let body_entity =
                bridge.with(|_session, dom| find_first_tag_descendant(dom, entity, "body"));
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
            let entity = Entity::from_bits(*bits as u64)
                .ok_or_else(|| JsNativeError::typ().with_message("invalid entity"))?;
            let head_entity =
                bridge.with(|_session, dom| find_first_tag_descendant(dom, entity, "head"));
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
fn find_first_tag_descendant(dom: &elidex_ecs::EcsDom, root: Entity, tag: &str) -> Option<Entity> {
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

/// Register `XMLSerializer` constructor.
pub(super) fn register_xml_serializer(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    ctx.register_global_callable(
        js_string!("XMLSerializer"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| {
                let mut init = ObjectInitializer::new(ctx);

                // serializeToString(node) -> string
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
                                    && dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok();

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
                                                    &v.replace('&', "&amp;").replace('"', "&quot;"),
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
                                        .and_then(|h| h.invoke(entity, &[], session, dom).ok())
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

/// Register `requestIdleCallback` and `cancelIdleCallback` global functions.
pub(crate) fn register_idle_callbacks(ctx: &mut Context) {
    // requestIdleCallback(callback, options?) -> id
    let ric_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
            JsNativeError::typ().with_message("requestIdleCallback: argument 1 must be a function")
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

        let delay = if timeout_ms > 0.0 { timeout_ms } else { 0.0 };

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
        let result = set_timeout
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("setTimeout not found"))?
            .call(
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

/// Register `structuredClone(value, options?)` global function.
pub(crate) fn register_structured_clone(ctx: &mut Context) {
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
        let json_obj = json
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("JSON global not found"))?;

        let stringify = json_obj.get(js_string!("stringify"), ctx)?;
        let parse = json_obj.get(js_string!("parse"), ctx)?;

        let stringify_fn = stringify
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("JSON.stringify is not callable"))?;
        let parse_fn = parse
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("JSON.parse is not callable"))?;

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
