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

    // children — Element children only (static array, WHATWG DOM §4.2.6).
    let b = bridge.clone();
    let realm = init.context().realm().clone();
    let children_getter = boa_engine::NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let children = bridge.with(|_session, dom| {
                let mut result = Vec::new();
                let mut child = dom.get_first_child(entity);
                while let Some(c) = child {
                    // Only include element nodes (have TagType).
                    if dom.world().get::<&elidex_ecs::TagType>(c).is_ok() {
                        result.push(c);
                    }
                    child = dom.get_next_sibling(c);
                }
                result
            });
            let array = JsArray::new(ctx);
            for child_entity in children {
                let wrapper = bridge.with(|session, dom| {
                    let obj_ref = session.get_or_create_wrapper(
                        child_entity,
                        elidex_script_session::ComponentKind::Element,
                    );
                    let is_iframe = dom
                        .world()
                        .get::<&elidex_ecs::TagType>(child_entity)
                        .ok()
                        .is_some_and(|t| t.0 == "iframe");
                    crate::globals::element::create_element_wrapper(
                        child_entity,
                        bridge,
                        obj_ref,
                        ctx,
                        is_iframe,
                    )
                });
                let _ = array.push(wrapper, ctx);
            }
            Ok(array.into())
        },
        b,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("children"),
        Some(children_getter),
        None,
        boa_engine::property::Attribute::CONFIGURABLE,
    );

    // getElementsByClassName(className)
    let b = bridge.clone();
    init.function(
        boa_engine::NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let class_name = require_js_string_arg(args, 0, "getElementsByClassName", ctx)?;
                let entities = bridge.with(|_session, dom| {
                    crate::globals::document::collect_elements_by_class(entity, &class_name, dom)
                });
                Ok(crate::globals::document::entities_to_js_array(&entities, bridge, ctx))
            },
            b,
        ),
        js_string!("getElementsByClassName"),
        1,
    );

    // getElementsByTagName(tagName)
    let b = bridge.clone();
    init.function(
        boa_engine::NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let tag = require_js_string_arg(args, 0, "getElementsByTagName", ctx)?;
                let entities = bridge.with(|_session, dom| {
                    crate::globals::document::collect_elements_by_tag(entity, &tag, dom)
                });
                Ok(crate::globals::document::entities_to_js_array(&entities, bridge, ctx))
            },
            b,
        ),
        js_string!("getElementsByTagName"),
        1,
    );
}
