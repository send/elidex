//! Element interface methods (matches, closest, insertAdjacent*, attribute methods).

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, JsValue, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::core::extract_entity;
use crate::globals::{
    boa_arg_to_elidex, invoke_dom_handler, invoke_dom_handler_ref, invoke_dom_handler_void,
    require_js_string_arg,
};

/// Register additional Element methods (matches, closest, insertAdjacent*, etc.).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(in crate::globals::element) fn register_element_extra_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    // matches(selector)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let sel = require_js_string_arg(args, 0, "matches", ctx)?;
                invoke_dom_handler("matches", entity, &[ElidexJsValue::String(sel)], bridge)
            },
            b,
        ),
        js_string!("matches"),
        1,
    );

    // closest(selector)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let sel = require_js_string_arg(args, 0, "closest", ctx)?;
                invoke_dom_handler_ref(
                    "closest",
                    entity,
                    &[ElidexJsValue::String(sel)],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("closest"),
        1,
    );

    // insertAdjacentElement(position, element)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let position = require_js_string_arg(args, 0, "insertAdjacentElement", ctx)?;
                let elem = boa_arg_to_elidex(args.get(1).unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler_ref(
                    "insertAdjacentElement",
                    entity,
                    &[ElidexJsValue::String(position), elem],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("insertAdjacentElement"),
        2,
    );

    // insertAdjacentText(position, text)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let position = require_js_string_arg(args, 0, "insertAdjacentText", ctx)?;
                let text = require_js_string_arg(args, 1, "insertAdjacentText", ctx)?;
                invoke_dom_handler_void(
                    "insertAdjacentText",
                    entity,
                    &[ElidexJsValue::String(position), ElidexJsValue::String(text)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("insertAdjacentText"),
        2,
    );

    // insertAdjacentHTML(position, html)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let position = require_js_string_arg(args, 0, "insertAdjacentHTML", ctx)?;
                let html = require_js_string_arg(args, 1, "insertAdjacentHTML", ctx)?;
                invoke_dom_handler_void(
                    "insertAdjacentHTML",
                    entity,
                    &[ElidexJsValue::String(position), ElidexJsValue::String(html)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("insertAdjacentHTML"),
        2,
    );

    // hasAttribute(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "hasAttribute", ctx)?;
                invoke_dom_handler(
                    "hasAttribute",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("hasAttribute"),
        1,
    );

    // toggleAttribute(name, force?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "toggleAttribute", ctx)?;
                let mut elidex_args = vec![ElidexJsValue::String(name)];
                if let Some(v) = args.get(1) {
                    elidex_args.push(ElidexJsValue::Bool(v.to_boolean()));
                }
                invoke_dom_handler("toggleAttribute", entity, &elidex_args, bridge)
            },
            b,
        ),
        js_string!("toggleAttribute"),
        1,
    );

    // getAttributeNames()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let result = invoke_dom_handler("getAttributeNames", entity, &[], bridge)?;
                let s = result.to_string(ctx)?.to_std_string_escaped();
                let array = JsArray::new(ctx);
                if !s.is_empty() {
                    for name in s.split('\0') {
                        array.push(JsValue::from(js_string!(name)), ctx)?;
                    }
                }
                Ok(array.into())
            },
            b,
        ),
        js_string!("getAttributeNames"),
        0,
    );
}
