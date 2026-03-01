//! `document` global object registration.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::DomApiHandler;

use crate::bridge::HostBridge;
use crate::error_conv::dom_error_to_js_error;
use crate::globals::element::resolve_object_ref;
use crate::globals::require_js_string_arg;

/// Register the `document` global object.
#[allow(clippy::too_many_lines)]
pub fn register_document(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();

    let mut init = ObjectInitializer::new(ctx);

    // document.querySelector(selector)
    let b_qs = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let selector = require_js_string_arg(args, 0, "querySelector", ctx)?;
                let doc = bridge.document_entity();
                let result = bridge.with(|session, dom| {
                    elidex_dom_api::QuerySelector
                        .invoke(doc, &[ElidexJsValue::String(selector)], session, dom)
                        .map_err(dom_error_to_js_error)
                })?;
                Ok(resolve_object_ref(&result, bridge, ctx))
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
                let doc = bridge.document_entity();
                let result = bridge.with(|session, dom| {
                    elidex_dom_api::GetElementById
                        .invoke(doc, &[ElidexJsValue::String(id)], session, dom)
                        .map_err(dom_error_to_js_error)
                })?;
                Ok(resolve_object_ref(&result, bridge, ctx))
            },
            b_id,
        ),
        js_string!("getElementById"),
        1,
    );

    // document.createElement(tagName)
    let b_ce = b.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let tag = require_js_string_arg(args, 0, "createElement", ctx)?;
                let doc = bridge.document_entity();
                let result = bridge.with(|session, dom| {
                    elidex_dom_api::CreateElement
                        .invoke(doc, &[ElidexJsValue::String(tag)], session, dom)
                        .map_err(dom_error_to_js_error)
                })?;
                Ok(resolve_object_ref(&result, bridge, ctx))
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
                let doc = bridge.document_entity();
                let result = bridge.with(|session, dom| {
                    elidex_dom_api::CreateTextNode
                        .invoke(doc, &[ElidexJsValue::String(text)], session, dom)
                        .map_err(dom_error_to_js_error)
                })?;
                Ok(resolve_object_ref(&result, bridge, ctx))
            },
            b_ctn,
        ),
        js_string!("createTextNode"),
        1,
    );

    // document.body — accessor returning the <body> element
    let b_body = b.clone();
    let realm = init.context().realm().clone();
    init.accessor(
        js_string!("body"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, ctx| -> JsResult<JsValue> {
                    let doc = bridge.document_entity();
                    // Find <body> by querySelector.
                    let result = bridge.with(|session, dom| {
                        elidex_dom_api::QuerySelector
                            .invoke(doc, &[ElidexJsValue::String("body".into())], session, dom)
                            .map_err(dom_error_to_js_error)
                    })?;
                    Ok(resolve_object_ref(&result, bridge, ctx))
                },
                b_body,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
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

    let document = init.build();
    ctx.register_global_property(js_string!("document"), document, Attribute::all())
        .expect("failed to register document");
}
