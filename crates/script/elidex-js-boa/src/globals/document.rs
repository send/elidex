//! `document` global object registration.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::element::resolve_object_ref;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_ref, require_js_string_arg};

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
                    let wrapper = bridge.with(|session, _dom| {
                        let obj_ref = session.get_or_create_wrapper(
                            entity,
                            elidex_script_session::ComponentKind::Element,
                        );
                        super::element::create_element_wrapper(entity, bridge, obj_ref, ctx)
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

// ---------------------------------------------------------------------------
// TreeWalker / NodeIterator / Range JS object builders
// ---------------------------------------------------------------------------

/// Hidden property key for the traversal object ID.
const TRAVERSAL_ID_KEY: &str = "__elidex_traversal_id__";

/// Build a JS object wrapping a `TreeWalker`.
#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
fn build_tree_walker_object(
    tw_id: u64,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!(TRAVERSAL_ID_KEY),
        JsValue::from(tw_id as f64),
        Attribute::empty(),
    );

    // nextNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_tree_walker(id, |tw| tw.next_node(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("nextNode"),
        0,
    );

    // previousNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_tree_walker(id, |tw| tw.previous_node(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("previousNode"),
        0,
    );

    // parentNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_tree_walker(id, |tw| tw.parent_node(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("parentNode"),
        0,
    );

    // firstChild()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_tree_walker(id, |tw| tw.first_child(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("firstChild"),
        0,
    );

    // lastChild()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_tree_walker(id, |tw| tw.last_child(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("lastChild"),
        0,
    );

    // nextSibling()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_tree_walker(id, |tw| tw.next_sibling(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("nextSibling"),
        0,
    );

    // previousSibling()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge.with(|_session, dom| {
                    bridge.with_tree_walker(id, |tw| tw.previous_sibling(dom))
                });
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("previousSibling"),
        0,
    );

    Ok(init.build().into())
}

/// Build a JS object wrapping a `NodeIterator`.
#[allow(clippy::unnecessary_wraps)]
fn build_node_iterator_object(
    ni_id: u64,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!(TRAVERSAL_ID_KEY),
        JsValue::from(ni_id as f64),
        Attribute::empty(),
    );

    // nextNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_node_iterator(id, |ni| ni.next_node(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("nextNode"),
        0,
    );

    // previousNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge.with(|_session, dom| {
                    bridge.with_node_iterator(id, |ni| ni.previous_node(dom))
                });
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("previousNode"),
        0,
    );

    Ok(init.build().into())
}

/// Build a JS object wrapping a Range.
#[allow(clippy::too_many_lines)]
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn build_range_object(
    range_id: u64,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!(TRAVERSAL_ID_KEY),
        JsValue::from(range_id as f64),
        Attribute::empty(),
    );

    // collapsed (getter)
    let b = bridge.clone();
    let realm = init.context().realm().clone();
    let collapsed_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let id = extract_traversal_id(this, ctx)?;
            let result = bridge.with_range(id, |r| r.collapsed());
            Ok(JsValue::from(result.unwrap_or(true)))
        },
        b,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("collapsed"),
        Some(collapsed_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // setStart(node, offset)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("setStart: node required")
                    })?,
                    ctx,
                )?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let offset = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                bridge.with_range(id, |r| r.set_start(node, offset));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("setStart"),
        2,
    );

    // setEnd(node, offset)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("setEnd: node required")
                    })?,
                    ctx,
                )?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let offset = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                bridge.with_range(id, |r| r.set_end(node, offset));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("setEnd"),
        2,
    );

    // collapse(toStart?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let to_start = args.first().is_some_and(JsValue::to_boolean);
                bridge.with_range(id, |r| r.collapse(to_start));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("collapse"),
        0,
    );

    // selectNode(node)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("selectNode: node required")
                    })?,
                    ctx,
                )?;
                bridge.with(|_session, dom| {
                    bridge.with_range(id, |r| r.select_node(node, dom));
                });
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("selectNode"),
        1,
    );

    // toString()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let text = bridge.with(|_session, dom| bridge.with_range(id, |r| r.to_string(dom)));
                Ok(JsValue::from(js_string!(text.unwrap_or_default())))
            },
            b,
        ),
        js_string!("toString"),
        0,
    );

    // deleteContents()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                bridge.with(|_session, dom| {
                    bridge.with_range(id, |r| r.delete_contents(dom));
                });
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("deleteContents"),
        0,
    );

    Ok(init.build().into())
}

/// Extract the traversal ID from a JS object's hidden property.
fn extract_traversal_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("traversal method called on non-object")
    })?;
    let id_val = obj.get(js_string!(TRAVERSAL_ID_KEY), ctx)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let id = id_val
        .as_number()
        .ok_or_else(|| JsNativeError::typ().with_message("invalid traversal object"))?
        as u64;
    Ok(id)
}

/// Resolve an Entity to a JS element wrapper.
fn resolve_entity_to_js(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let obj_ref = bridge.with(|session, dom| {
        let kind = dom.node_kind(entity).map_or(
            elidex_script_session::ComponentKind::Element,
            elidex_script_session::ComponentKind::from_node_kind,
        );
        session.get_or_create_wrapper(entity, kind)
    });
    crate::globals::element::create_element_wrapper(entity, bridge, obj_ref, ctx)
}
