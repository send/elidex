//! `document` global object registration.

mod traversal;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::element::resolve_object_ref;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_ref, require_js_string_arg};

pub(crate) use traversal::build_range_object;

/// Common pattern for document methods that take a single string argument,
/// invoke a DOM API handler by name on the document entity, and return an element ref.
fn invoke_doc_handler_returning_ref(
    handler_name: &str,
    arg: String,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let doc = bridge.document_entity();
    let handler = bridge.dom_registry().resolve(handler_name).ok_or_else(|| {
        JsNativeError::typ().with_message(format!("Unknown DOM method: {handler_name}"))
    })?;
    let result = bridge.with(|session, dom| {
        handler
            .invoke(doc, &[ElidexJsValue::String(arg)], session, dom)
            .map_err(dom_error_to_js_error)
    })?;
    Ok(resolve_object_ref(&result, bridge, ctx))
}

/// Register the `document` global object.
#[allow(clippy::too_many_lines)]
// Sequential property/method registration on a single JS object.
#[allow(clippy::similar_names)]
pub fn register_document(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();

    let mut init = ObjectInitializer::new(ctx);

    // document.querySelector(selector)
    let b_qs = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let selector = require_js_string_arg(args, 0, "querySelector", ctx)?;
                invoke_doc_handler_returning_ref("querySelector", selector, bridge, ctx)
            },
            b_qs,
        ),
        js_string!("querySelector"),
        1,
    );

    // document.querySelectorAll(selector)
    let b_qsa = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let selector = require_js_string_arg(args, 0, "querySelectorAll", ctx)?;
                let doc = bridge.document_entity();
                let entities = bridge.with(|_session, dom| {
                    elidex_dom_api::query_selector_all(doc, &selector, dom)
                        .map_err(dom_error_to_js_error)
                })?;
                // Convert to JS array.
                let array = boa_engine::object::builtins::JsArray::new(ctx);
                for entity in entities {
                    let wrapper = bridge.with(|session, dom| {
                        let obj_ref = session.get_or_create_wrapper(
                            entity,
                            elidex_script_session::ComponentKind::Element,
                        );
                        let is_iframe = dom
                            .world()
                            .get::<&elidex_ecs::TagType>(entity)
                            .ok()
                            .is_some_and(|t| t.0 == "iframe");
                        super::element::create_element_wrapper(
                            entity, bridge, obj_ref, ctx, is_iframe,
                        )
                    });
                    array.push(wrapper, ctx)?;
                }
                Ok(array.into())
            },
            b_qsa,
        ),
        js_string!("querySelectorAll"),
        1,
    );

    // document.getElementById(id)
    let b_id = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = require_js_string_arg(args, 0, "getElementById", ctx)?;
                invoke_doc_handler_returning_ref("getElementById", id, bridge, ctx)
            },
            b_id,
        ),
        js_string!("getElementById"),
        1,
    );

    // document.createElement(tagName, options?)
    let b_ce = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let tag = require_js_string_arg(args, 0, "createElement", ctx)?;

                // Extract options.is if present (customized built-in elements).
                // Per spec, invalid `is` values are silently ignored.
                let is_value = if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
                    let v = opts.get(js_string!("is"), ctx)?;
                    if v.is_undefined() || v.is_null() {
                        None
                    } else {
                        let is_name = v.to_string(ctx)?.to_std_string_escaped();
                        if elidex_custom_elements::is_valid_custom_element_name(&is_name) {
                            Some(is_name)
                        } else {
                            None
                        }
                    }
                } else {
                    None
                };

                let result =
                    invoke_doc_handler_returning_ref("createElement", tag.clone(), bridge, ctx)?;

                // Mark custom elements for upgrade tracking.
                if let Ok(entity) = crate::globals::element::extract_entity(&result, ctx) {
                    let ce_name = if elidex_custom_elements::is_valid_custom_element_name(&tag) {
                        Some(tag.clone())
                    } else {
                        is_value.clone()
                    };

                    if let Some(name) = ce_name {
                        bridge.with(|_session, dom| {
                            // For customized built-ins, verify the definition matches the tag.
                            let defined = if is_value.is_some() {
                                bridge.ce_lookup_by_is(&name, &tag)
                            } else {
                                bridge.is_custom_element_defined(&name)
                            };
                            if defined {
                                // Definition exists — enqueue Upgrade.
                                let ce_state =
                                    elidex_custom_elements::CustomElementState::undefined(&name);
                                let _ = dom.world_mut().insert_one(entity, ce_state);
                                bridge.enqueue_ce_reaction(
                                    elidex_custom_elements::CustomElementReaction::Upgrade(entity),
                                );
                            } else {
                                // Not yet defined — mark as undefined and queue.
                                let ce_state =
                                    elidex_custom_elements::CustomElementState::undefined(&name);
                                let _ = dom.world_mut().insert_one(entity, ce_state);
                                bridge.queue_for_ce_upgrade(&name, entity);
                            }
                        });
                    }

                    // Set the `is` attribute on the element per WHATWG spec §4.13.3.
                    if let Some(ref is_name) = is_value {
                        bridge.with(|_session, dom| {
                            if let Ok(mut attrs) =
                                dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity)
                            {
                                attrs.set("is", is_name);
                            }
                            // Bump version so LiveCollections / caches invalidate.
                            dom.rev_version(entity);
                        });
                    }
                }

                Ok(result)
            },
            b_ce,
        ),
        js_string!("createElement"),
        1,
    );

    // document.createTextNode(data)
    let b_ctn = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let text = require_js_string_arg(args, 0, "createTextNode", ctx)?;
                invoke_doc_handler_returning_ref("createTextNode", text, bridge, ctx)
            },
            b_ctn,
        ),
        js_string!("createTextNode"),
        1,
    );

    let realm = init.context().realm().clone();

    // --- Document property accessors ---
    register_doc_ref_accessor(
        &mut init,
        &b,
        &realm,
        "documentElement",
        "document.documentElement.get",
    );
    register_doc_ref_accessor(&mut init, &b, &realm, "head", "document.head.get");
    register_doc_ref_accessor(&mut init, &b, &realm, "body", "document.body.get");
    register_doc_val_accessor(&mut init, &b, &realm, "URL", "document.URL.get");
    register_doc_val_accessor(
        &mut init,
        &b,
        &realm,
        "readyState",
        "document.readyState.get",
    );
    register_doc_val_accessor(
        &mut init,
        &b,
        &realm,
        "compatMode",
        "document.compatMode.get",
    );
    register_doc_val_accessor(
        &mut init,
        &b,
        &realm,
        "characterSet",
        "document.characterSet.get",
    );

    // document.title — getter/setter
    {
        let b_get = b.clone();
        let getter = NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| {
                let doc = bridge.document_entity();
                invoke_dom_handler("document.title.get", doc, &[], bridge)
            },
            b_get,
        )
        .to_js_function(&realm);
        let b_set = b.clone();
        let setter = NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let doc = bridge.document_entity();
                let text = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                crate::globals::invoke_dom_handler_void(
                    "document.title.set",
                    doc,
                    &[ElidexJsValue::String(text)],
                    bridge,
                )
            },
            b_set,
        )
        .to_js_function(&realm);
        init.accessor(
            js_string!("title"),
            Some(getter),
            Some(setter),
            Attribute::CONFIGURABLE,
        );
    }

    // document.doctype — read-only ref accessor
    register_doc_ref_accessor(&mut init, &b, &realm, "doctype", "doctype.get");

    // --- Document creation methods ---

    // document.createDocumentFragment()
    let b_cdf = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| -> JsResult<JsValue> {
                invoke_doc_handler_returning_ref(
                    "createDocumentFragment",
                    String::new(),
                    bridge,
                    ctx,
                )
            },
            b_cdf,
        ),
        js_string!("createDocumentFragment"),
        0,
    );

    // document.createComment(data)
    let b_cc = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let data = require_js_string_arg(args, 0, "createComment", ctx)?;
                invoke_doc_handler_returning_ref("createComment", data, bridge, ctx)
            },
            b_cc,
        ),
        js_string!("createComment"),
        1,
    );

    // document.createAttribute(name)
    let b_ca = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let name = require_js_string_arg(args, 0, "createAttribute", ctx)?;
                invoke_doc_handler_returning_ref("createAttribute", name, bridge, ctx)
            },
            b_ca,
        ),
        js_string!("createAttribute"),
        1,
    );

    // document.addEventListener(type, listener, capture?)
    let b_add_listener = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let doc = bridge.document_entity();
                crate::globals::add_event_listener_for(doc, args, bridge, ctx)
            },
            b_add_listener,
        ),
        js_string!("addEventListener"),
        2,
    );

    // document.removeEventListener(type, listener, capture?)
    let b_rm_listener = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let doc = bridge.document_entity();
                crate::globals::remove_event_listener_for(doc, args, bridge, ctx)
            },
            b_rm_listener,
        ),
        js_string!("removeEventListener"),
        2,
    );

    // document.dispatchEvent(event)
    let b_dispatch = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let doc = bridge.document_entity();
                crate::globals::dispatch_event_for(doc, args, bridge, ctx)
            },
            b_dispatch,
        ),
        js_string!("dispatchEvent"),
        1,
    );

    // --- TreeWalker / NodeIterator / Range ---

    // document.createTreeWalker(root, whatToShow?)
    let b_ctw = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let root_entity = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("createTreeWalker: root argument required")
                    })?,
                    ctx,
                )?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let what_to_show = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(elidex_dom_api::SHOW_ALL, |n| n as u32);

                let tw_id = bridge.create_tree_walker(root_entity, what_to_show);
                build_tree_walker_object(tw_id, bridge, ctx)
            },
            b_ctw,
        ),
        js_string!("createTreeWalker"),
        1,
    );

    // document.createNodeIterator(root, whatToShow?)
    let b_cni = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let root_entity = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ()
                            .with_message("createNodeIterator: root argument required")
                    })?,
                    ctx,
                )?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let what_to_show = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(elidex_dom_api::SHOW_ALL, |n| n as u32);

                let ni_id = bridge.create_node_iterator(root_entity, what_to_show);
                build_node_iterator_object(ni_id, bridge, ctx)
            },
            b_cni,
        ),
        js_string!("createNodeIterator"),
        1,
    );

    // document.createRange()
    let b_cr = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| -> JsResult<JsValue> {
                let doc = bridge.document_entity();
                let range_id = bridge.create_range(doc);
                build_range_object(range_id, bridge, ctx)
            },
            b_cr,
        ),
        js_string!("createRange"),
        0,
    );

    // document.styleSheets — read-only accessor returning a StyleSheetList
    {
        let b_ss = b.clone();
        let ss_getter = NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| {
                Ok(crate::globals::cssom::build_stylesheet_list(bridge, ctx))
            },
            b_ss,
        )
        .to_js_function(&realm);
        init.accessor(
            js_string!("styleSheets"),
            Some(ss_getter),
            None,
            Attribute::CONFIGURABLE,
        );
    }

    // --- M4-4.5: Document API additions ---

    // document.hidden — getter (W3C Page Visibility §4).
    let b_hidden = b.clone();
    let hidden_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.is_tab_hidden())),
        b_hidden,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("hidden"),
        Some(hidden_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // document.visibilityState — getter.
    let b_vis = b.clone();
    let vis_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            let state = if bridge.is_tab_hidden() {
                "hidden"
            } else {
                "visible"
            };
            Ok(JsValue::from(js_string!(state)))
        },
        b_vis,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("visibilityState"),
        Some(vis_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // document.hasFocus() — WHATWG HTML §6.5.4.
    let b_focus = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.focus_target().is_some())),
            b_focus,
        ),
        js_string!("hasFocus"),
        0,
    );

    // document.activeElement — WHATWG HTML §6.5.4.2.
    // Returns focused element, or document.body if none, or null if no body.
    let b_ae = b.clone();
    let ae_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, ctx| {
            if let Some(focused) = bridge.focus_target() {
                return Ok(traversal::resolve_entity_to_js(focused, bridge, ctx));
            }
            // No focus → return document.body (or null).
            let doc = bridge.document_entity();
            let body_handler = bridge.dom_registry().resolve("document.body.get");
            if let Some(handler) = body_handler {
                let result = bridge.with(|session, dom| {
                    handler.invoke(doc, &[], session, dom).ok()
                });
                if let Some(val) = result {
                    return Ok(resolve_object_ref(&val, bridge, ctx));
                }
            }
            Ok(JsValue::null())
        },
        b_ae,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("activeElement"),
        Some(ae_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // document.getElementsByClassName(className) — static array (live HTMLCollection in Step 5).
    let b_gbcn = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let class_name = crate::globals::require_js_string_arg(
                    args,
                    0,
                    "getElementsByClassName",
                    ctx,
                )?;
                let doc = bridge.document_entity();
                let entities = bridge.with(|_session, dom| {
                    collect_elements_by_class(doc, &class_name, dom)
                });
                Ok(entities_to_js_array(&entities, bridge, ctx))
            },
            b_gbcn,
        ),
        js_string!("getElementsByClassName"),
        1,
    );

    // document.getElementsByTagName(tagName) — static array.
    let b_gbtn = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let tag = crate::globals::require_js_string_arg(
                    args,
                    0,
                    "getElementsByTagName",
                    ctx,
                )?;
                let doc = bridge.document_entity();
                let entities = bridge.with(|_session, dom| {
                    collect_elements_by_tag(doc, &tag, dom)
                });
                Ok(entities_to_js_array(&entities, bridge, ctx))
            },
            b_gbtn,
        ),
        js_string!("getElementsByTagName"),
        1,
    );

    // document.getElementsByName(name) — static array.
    let b_gbn = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let name = crate::globals::require_js_string_arg(
                    args,
                    0,
                    "getElementsByName",
                    ctx,
                )?;
                let doc = bridge.document_entity();
                let entities = bridge.with(|_session, dom| {
                    collect_elements_by_name(doc, &name, dom)
                });
                Ok(entities_to_js_array(&entities, bridge, ctx))
            },
            b_gbn,
        ),
        js_string!("getElementsByName"),
        1,
    );

    // document.importNode(node, deep?) — WHATWG DOM §4.5.
    let b_import = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("importNode: node required")
                    })?,
                    ctx,
                )?;
                // WHATWG DOM §4.5: throw NotSupportedError if node is a Document.
                let doc_entity = bridge.document_entity();
                if node == doc_entity {
                    return Err(JsNativeError::eval()
                        .with_message("NotSupportedError: importNode: Document nodes cannot be imported")
                        .into());
                }
                let deep = args.get(1).is_some_and(JsValue::to_boolean);
                // Use cloneNode — single-document assumption.
                invoke_dom_handler_ref(
                    "cloneNode",
                    node,
                    &[ElidexJsValue::Bool(deep)],
                    bridge,
                    ctx,
                )
            },
            b_import,
        ),
        js_string!("importNode"),
        2,
    );

    // document.adoptNode(node) — WHATWG DOM §4.5.
    let b_adopt = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let node_val = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("adoptNode: node required")
                })?;
                let entity = crate::globals::element::extract_entity(node_val, ctx)?;
                // WHATWG DOM §4.5: throw NotSupportedError if node is a Document.
                let doc_entity = bridge.document_entity();
                if entity == doc_entity {
                    return Err(JsNativeError::eval()
                        .with_message("NotSupportedError: adoptNode: Document nodes cannot be adopted")
                        .into());
                }
                // Detach from parent if attached (via removeChild on parent).
                bridge.with(|session, dom| {
                    if let Some(parent) = dom.get_parent(entity) {
                        let obj_ref = session.get_or_create_wrapper(
                            entity,
                            elidex_script_session::ComponentKind::Element,
                        );
                        let handler = bridge.dom_registry().resolve("removeChild");
                        if let Some(h) = handler {
                            let _ = h.invoke(
                                parent,
                                &[ElidexJsValue::ObjectRef(obj_ref.to_raw())],
                                session,
                                dom,
                            );
                        }
                    }
                });
                Ok(node_val.clone())
            },
            b_adopt,
        ),
        js_string!("adoptNode"),
        1,
    );

    // document.createEvent(interface) — legacy (WHATWG DOM §4.1).
    init.function(
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let iface = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped().to_ascii_lowercase())
                .unwrap_or_default();
            match iface.as_str() {
                "event" | "events" | "customevent" | "mouseevent" | "mouseevents"
                | "uievent" | "uievents" => {
                    // Create event with initialized=false via the Event constructor path,
                    // then override initialized to false.
                    let event = crate::globals::event_constructors::build_uninit_event(ctx)?;
                    Ok(event)
                }
                _ => Err(JsNativeError::eval()
                    .with_message("NotSupportedError: unsupported event interface")
                    .into()),
            }
        }),
        js_string!("createEvent"),
        1,
    );

    // document.forms — live-ish getter (re-queries on each access).
    register_collection_getter(&mut init, &b, &realm, "forms", "form");
    // document.images
    register_collection_getter(&mut init, &b, &realm, "images", "img");
    // document.scripts
    register_collection_getter(&mut init, &b, &realm, "scripts", "script");
    // document.links — <a href> + <area href>
    {
        let b_links = b.clone();
        let links_getter = NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, ctx| {
                let doc = bridge.document_entity();
                let entities = bridge.with(|_session, dom| {
                    let mut results = Vec::new();
                    walk_descendants(doc, dom, &mut |entity| {
                        if let Ok(tt) = dom.world().get::<&elidex_ecs::TagType>(entity) {
                            if (tt.0 == "a" || tt.0 == "area")
                                && dom
                                    .world()
                                    .get::<&elidex_ecs::Attributes>(entity)
                                    .ok()
                                    .is_some_and(|a| a.get("href").is_some())
                            {
                                results.push(entity);
                            }
                        }
                    });
                    results
                });
                Ok(entities_to_js_array(&entities, bridge, ctx))
            },
            b_links,
        )
        .to_js_function(&realm);
        init.accessor(
            js_string!("links"),
            Some(links_getter),
            None,
            Attribute::CONFIGURABLE,
        );
    }

    // document.cookie — getter/setter (RFC 6265, WHATWG HTML §3.1.3).
    {
        let b_cg = b.clone();
        let cookie_getter = NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| {
                // Return cookie string for current URL, filtering HttpOnly cookies.
                let url = bridge.current_url();
                let cookie_str = if let Some(ref url) = url {
                    bridge.with(|_session, _dom| bridge.cookies_for_script(url))
                } else {
                    String::new()
                };
                Ok(JsValue::from(js_string!(cookie_str)))
            },
            b_cg,
        )
        .to_js_function(&realm);

        let b_cs = b.clone();
        let cookie_setter = NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let value = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                if let Some(ref url) = bridge.current_url() {
                    bridge.set_cookie_from_script(url, &value);
                }
                Ok(JsValue::undefined())
            },
            b_cs,
        )
        .to_js_function(&realm);

        init.accessor(
            js_string!("cookie"),
            Some(cookie_getter),
            Some(cookie_setter),
            Attribute::CONFIGURABLE,
        );
    }

    // --- Legacy compat stubs ---

    // document.all → undefined (compat stub, Phase 4 TODO: HTMLAllCollection)
    init.property(js_string!("all"), JsValue::undefined(), Attribute::READONLY);

    // document.write() requires re-entrant HTML parsing (streaming parser
    // that can be called during script execution). Deferred to M4-3.10+
    // when the HTML parser supports incremental/streaming mode.
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("write"),
        1,
    );

    // document.writeln(...) → no-op (compat stub)
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
        js_string!("writeln"),
        1,
    );

    // document.referrer — returns the referrer URL (parent URL for iframe documents).
    let b_referrer = b.clone();
    let referrer_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            let referrer = bridge.referrer().unwrap_or_default();
            Ok(JsValue::from(js_string!(referrer)))
        },
        b_referrer,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("referrer"),
        Some(referrer_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    let document = init.build();
    ctx.register_global_property(js_string!("document"), document, Attribute::all())
        .expect("failed to register document");
}

/// Register a read-only document accessor that returns an element ref via a DOM handler.
fn register_doc_ref_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, bridge, ctx| {
            let doc = bridge.document_entity();
            invoke_dom_handler_ref(handler, doc, &[], bridge, ctx)
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!(js_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

/// Register a read-only document accessor that returns a primitive value via a DOM handler.
fn register_doc_val_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, bridge, _ctx| {
            let doc = bridge.document_entity();
            invoke_dom_handler(handler, doc, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!(js_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

// TreeWalker / NodeIterator / Range JS object builders are in traversal.rs.
use traversal::{build_node_iterator_object, build_tree_walker_object};

// Traversal builders (TreeWalker, NodeIterator, Range) are in traversal.rs.

// ---------------------------------------------------------------------------
// getElementsBy* helpers
// ---------------------------------------------------------------------------

/// Collect all descendant elements matching a class name (space-separated class list).
pub(crate) fn collect_elements_by_class(
    root: Entity,
    class_name: &str,
    dom: &elidex_ecs::EcsDom,
) -> Vec<Entity> {
    let target_classes: Vec<&str> = class_name.split_whitespace().collect();
    if target_classes.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    walk_descendants(root, dom, &mut |entity| {
        if let Ok(attrs) = dom.world().get::<&elidex_ecs::Attributes>(entity) {
            if let Some(cls) = attrs.get("class") {
                let element_classes: Vec<&str> = cls.split_whitespace().collect();
                if target_classes
                    .iter()
                    .all(|tc| element_classes.contains(tc))
                {
                    results.push(entity);
                }
            }
        }
    });
    results
}

/// Collect all descendant elements matching a tag name (case-insensitive).
pub(crate) fn collect_elements_by_tag(
    root: Entity,
    tag: &str,
    dom: &elidex_ecs::EcsDom,
) -> Vec<Entity> {
    let tag_lower = tag.to_ascii_lowercase();
    let match_all = tag == "*";
    let mut results = Vec::new();
    walk_descendants(root, dom, &mut |entity| {
        if match_all {
            // "*" matches all elements.
            if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
                results.push(entity);
            }
        } else if let Ok(tt) = dom.world().get::<&elidex_ecs::TagType>(entity) {
            if tt.0.eq_ignore_ascii_case(&tag_lower) {
                results.push(entity);
            }
        }
    });
    results
}

/// Collect all descendant elements with a matching `name` attribute.
fn collect_elements_by_name(
    root: Entity,
    name: &str,
    dom: &elidex_ecs::EcsDom,
) -> Vec<Entity> {
    let mut results = Vec::new();
    walk_descendants(root, dom, &mut |entity| {
        if let Ok(attrs) = dom.world().get::<&elidex_ecs::Attributes>(entity) {
            if attrs.get("name").is_some_and(|n| n == name) {
                results.push(entity);
            }
        }
    });
    results
}

/// Pre-order walk of all descendants (excluding root).
fn walk_descendants(
    root: Entity,
    dom: &elidex_ecs::EcsDom,
    callback: &mut dyn FnMut(Entity),
) {
    let mut stack = Vec::new();
    // Push children in reverse order so first child is processed first.
    let mut child = dom.get_first_child(root);
    let mut children = Vec::new();
    while let Some(c) = child {
        children.push(c);
        child = dom.get_next_sibling(c);
    }
    stack.extend(children.into_iter().rev());

    while let Some(entity) = stack.pop() {
        callback(entity);
        // Push children in reverse order.
        let mut child = dom.get_first_child(entity);
        let mut children = Vec::new();
        while let Some(c) = child {
            children.push(c);
            child = dom.get_next_sibling(c);
        }
        stack.extend(children.into_iter().rev());
    }
}

/// Convert a list of entities to a JS array of element wrappers.
pub(crate) fn entities_to_js_array(
    entities: &[Entity],
    bridge: &HostBridge,
    ctx: &mut boa_engine::Context,
) -> JsValue {
    let array = boa_engine::object::builtins::JsArray::new(ctx);
    for &entity in entities {
        let wrapper = traversal::resolve_entity_to_js(entity, bridge, ctx);
        let _ = array.push(wrapper, ctx);
    }
    array.into()
}

/// Register a document collection getter that returns elements matching a tag name.
fn register_collection_getter(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    tag: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, bridge, ctx| {
            let doc = bridge.document_entity();
            let entities =
                bridge.with(|_session, dom| collect_elements_by_tag(doc, tag, dom));
            Ok(entities_to_js_array(&entities, bridge, ctx))
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!(js_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}
