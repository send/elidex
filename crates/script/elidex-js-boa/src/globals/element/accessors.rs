//! Node methods, ChildNode/ParentNode mixin, element extra methods/accessors,
//! dataset, classList, cached accessor pattern.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;

use super::core::{entity_bits_as_f64, extract_entity};
use super::DATASET_CACHE_KEY;
use super::ENTITY_KEY;
use crate::bridge::HostBridge;
use crate::globals::{
    boa_arg_to_elidex, boa_args_to_elidex, invoke_dom_handler, invoke_dom_handler_ref,
    invoke_dom_handler_void, require_js_string_arg,
};

/// Read-only attribute for `DOMRect` properties.
const RO_ATTR: Attribute = Attribute::READONLY;

// ---------------------------------------------------------------------------
// Node methods (contains, compareDocumentPosition, cloneNode, etc.)
// ---------------------------------------------------------------------------

/// Register node methods that take entity arguments.
pub(crate) fn register_node_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // contains(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("contains", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("contains"),
        1,
    );

    // compareDocumentPosition(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("compareDocumentPosition", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("compareDocumentPosition"),
        1,
    );

    // cloneNode(deep?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let deep = args.first().is_some_and(JsValue::to_boolean);
                invoke_dom_handler_ref(
                    "cloneNode",
                    entity,
                    &[ElidexJsValue::Bool(deep)],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("cloneNode"),
        1,
    );

    // normalize()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_void("normalize", entity, &[], bridge)
            },
            b,
        ),
        js_string!("normalize"),
        0,
    );

    // getRootNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_ref("getRootNode", entity, &[], bridge, ctx)
            },
            b,
        ),
        js_string!("getRootNode"),
        0,
    );

    // isSameNode(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("isSameNode", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("isSameNode"),
        1,
    );

    // isEqualNode(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("isEqualNode", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("isEqualNode"),
        1,
    );
}

// ---------------------------------------------------------------------------
// ChildNode / ParentNode mixin methods
// ---------------------------------------------------------------------------

/// Register variadic ChildNode/ParentNode mixin methods (before, after, remove, etc.).
pub(crate) fn register_child_parent_mixin_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    // Variadic methods: before, after, replaceWith, prepend, append, replaceChildren.
    static VARIADIC_METHODS: &[&str] = &[
        "before",
        "after",
        "replaceWith",
        "prepend",
        "append",
        "replaceChildren",
    ];
    for &method_name in VARIADIC_METHODS {
        let b = bridge.clone();
        init.function(
            NativeFunction::from_copy_closure_with_captures(
                move |this, args, bridge, ctx| {
                    let entity = extract_entity(this, ctx)?;
                    let elidex_args = boa_args_to_elidex(args, bridge, ctx)?;
                    invoke_dom_handler_void(method_name, entity, &elidex_args, bridge)
                },
                b,
            ),
            js_string!(method_name),
            0,
        );
    }

    // remove() — no args.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_void("remove", entity, &[], bridge)
            },
            b,
        ),
        js_string!("remove"),
        0,
    );
}

// ---------------------------------------------------------------------------
// Element extra methods
// ---------------------------------------------------------------------------

/// Register additional Element methods (matches, closest, insertAdjacent*, etc.).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(crate) fn register_element_extra_methods(
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

// ---------------------------------------------------------------------------
// Element extra accessors (className, id)
// ---------------------------------------------------------------------------

/// Register className and id getter/setter accessors.
#[allow(clippy::similar_names)] // Getter/setter pairs (e.g., cls_getter/cls_setter) intentionally similar
pub(crate) fn register_element_extra_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // className getter/setter.
    let b = bridge.clone();
    let cn_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("className.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let cn_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "className.set",
                entity,
                &[ElidexJsValue::String(val)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("className"),
        Some(cn_getter),
        Some(cn_setter),
        Attribute::CONFIGURABLE,
    );

    // id getter/setter.
    let b = bridge.clone();
    let id_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("id.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let id_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void("id.set", entity, &[ElidexJsValue::String(val)], bridge)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("id"),
        Some(id_getter),
        Some(id_setter),
        Attribute::CONFIGURABLE,
    );

    // attributes (NamedNodeMap-like object) — read-only
    let b = bridge.clone();
    let attrs_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            build_named_node_map(entity, bridge, ctx)
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!("attributes"),
        Some(attrs_getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

/// Build a `NamedNodeMap`-like JS object for the given element's attributes.
///
/// All accessors (`length`, `item()`, `getNamedItem()`) are live: they query
/// the `Attributes` component on each access, reflecting mutations made after
/// the `NamedNodeMap` was obtained.
#[allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
fn build_named_node_map(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    // Store entity for dynamic lookups.
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits_as_f64(entity)),
        Attribute::empty(),
    );

    // length — dynamic getter that reads current attribute count.
    let realm = init.context().realm().clone();
    let b_len = bridge.clone();
    let length_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let len = bridge.with(|_session, dom| {
                dom.world()
                    .get::<&elidex_ecs::Attributes>(entity)
                    .map_or(0, |a| a.iter().count())
            });
            #[allow(clippy::cast_precision_loss)]
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

    // item(index) — reads attributes at call time, returns proper Attr wrapper.
    let b_item = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                // Get the attribute name at the given index.
                let attr_name = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|a| a.iter().nth(index).map(|(k, _)| k.to_string()))
                });
                match attr_name {
                    Some(name) => {
                        // Use getAttributeNode DOM handler to get proper Attr entity.
                        let result = invoke_dom_handler_ref(
                            "getAttributeNode",
                            entity,
                            &[ElidexJsValue::String(name)],
                            bridge,
                            ctx,
                        )?;
                        if result.is_null() || result.is_undefined() {
                            return Ok(JsValue::null());
                        }
                        if let Some(obj) = result.as_object() {
                            let entity_val = obj.get(js_string!(ENTITY_KEY), ctx)?;
                            if !entity_val.is_undefined() {
                                let attr_entity = extract_entity(&result, ctx)?;
                                return Ok(super::special_nodes::create_attr_object(
                                    attr_entity,
                                    bridge,
                                    ctx,
                                ));
                            }
                        }
                        Ok(result)
                    }
                    None => Ok(JsValue::null()),
                }
            },
            b_item,
        ),
        js_string!("item"),
        1,
    );

    // getNamedItem(name) — reads attributes at call time, returns proper Attr wrapper.
    let b_named = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                // Check attribute exists before invoking handler.
                let exists = bridge.with(|_session, dom| {
                    dom.world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .is_some_and(|a| a.get(&name).is_some())
                });
                if !exists {
                    return Ok(JsValue::null());
                }
                // Use getAttributeNode DOM handler to get proper Attr entity with identity.
                let result = invoke_dom_handler_ref(
                    "getAttributeNode",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                    ctx,
                )?;
                if result.is_null() || result.is_undefined() {
                    return Ok(JsValue::null());
                }
                if let Some(obj) = result.as_object() {
                    let entity_val = obj.get(js_string!(ENTITY_KEY), ctx)?;
                    if !entity_val.is_undefined() {
                        let attr_entity = extract_entity(&result, ctx)?;
                        return Ok(super::special_nodes::create_attr_object(
                            attr_entity,
                            bridge,
                            ctx,
                        ));
                    }
                }
                Ok(result)
            },
            b_named,
        ),
        js_string!("getNamedItem"),
        1,
    );

    // removeNamedItem(name) — removes the attribute from the element.
    let b_rm = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                invoke_dom_handler_void(
                    "removeAttribute",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b_rm,
        ),
        js_string!("removeNamedItem"),
        1,
    );

    Ok(init.build().into())
}

// ---------------------------------------------------------------------------
// Layout query accessors (getBoundingClientRect, offset*, client*, scroll*)
// ---------------------------------------------------------------------------

/// Register layout query method and property accessors on an element object.
#[allow(clippy::too_many_lines)]
#[allow(clippy::many_single_char_names)]
pub(crate) fn register_layout_query_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // getBoundingClientRect() — returns a DOMRect object with x,y,width,height,top,right,bottom,left.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let result = invoke_dom_handler("getBoundingClientRect", entity, &[], bridge)?;
                // The handler returns "x,y,width,height" as a comma-separated string.
                // Parse and construct a proper DOMRect object.
                if let Some(s) = result.as_string() {
                    let s = s.to_std_string_escaped();
                    let parts: Vec<f64> = s
                        .split(',')
                        .filter_map(|p| p.trim().parse::<f64>().ok())
                        .collect();
                    if parts.len() == 4 {
                        let (x, y, w, h) = (parts[0], parts[1], parts[2], parts[3]);
                        let obj = ObjectInitializer::new(ctx)
                            .property(js_string!("x"), boa_engine::JsValue::from(x), RO_ATTR)
                            .property(js_string!("y"), boa_engine::JsValue::from(y), RO_ATTR)
                            .property(js_string!("width"), boa_engine::JsValue::from(w), RO_ATTR)
                            .property(js_string!("height"), boa_engine::JsValue::from(h), RO_ATTR)
                            .property(js_string!("top"), boa_engine::JsValue::from(y), RO_ATTR)
                            .property(
                                js_string!("right"),
                                boa_engine::JsValue::from(x + w),
                                RO_ATTR,
                            )
                            .property(
                                js_string!("bottom"),
                                boa_engine::JsValue::from(y + h),
                                RO_ATTR,
                            )
                            .property(js_string!("left"), boa_engine::JsValue::from(x), RO_ATTR)
                            .build();
                        return Ok(boa_engine::JsValue::from(obj));
                    }
                }
                Ok(result)
            },
            b,
        ),
        js_string!("getBoundingClientRect"),
        0,
    );

    // getClientRects() — returns an array of DOMRect objects.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let result = invoke_dom_handler("getClientRects", entity, &[], bridge)?;
                // Handler returns "x,y,w,h" or newline-separated entries for multi-line inlines.
                let array = JsArray::new(ctx);
                if let Some(s) = result.as_string() {
                    let s = s.to_std_string_escaped();
                    for line in s.lines() {
                        let parts: Vec<f64> = line
                            .split(',')
                            .filter_map(|p| p.trim().parse::<f64>().ok())
                            .collect();
                        if parts.len() != 4 {
                            continue;
                        }
                        let (x, y, w, h) = (parts[0], parts[1], parts[2], parts[3]);
                        let obj = ObjectInitializer::new(ctx)
                            .property(js_string!("x"), boa_engine::JsValue::from(x), RO_ATTR)
                            .property(js_string!("y"), boa_engine::JsValue::from(y), RO_ATTR)
                            .property(js_string!("width"), boa_engine::JsValue::from(w), RO_ATTR)
                            .property(js_string!("height"), boa_engine::JsValue::from(h), RO_ATTR)
                            .property(js_string!("top"), boa_engine::JsValue::from(y), RO_ATTR)
                            .property(
                                js_string!("right"),
                                boa_engine::JsValue::from(x + w),
                                RO_ATTR,
                            )
                            .property(
                                js_string!("bottom"),
                                boa_engine::JsValue::from(y + h),
                                RO_ATTR,
                            )
                            .property(js_string!("left"), boa_engine::JsValue::from(x), RO_ATTR)
                            .build();
                        array.push(boa_engine::JsValue::from(obj), ctx)?;
                    }
                }
                Ok(array.into())
            },
            b,
        ),
        js_string!("getClientRects"),
        0,
    );

    // scrollIntoView(arg?) — scroll nearest scrollable ancestor to make element visible.
    // Parses boolean or options object to determine block alignment.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                // Parse the block alignment from the first argument.
                let block = if let Some(first) = args.first() {
                    if let Some(b) = first.as_boolean() {
                        // scrollIntoView(true) → "start", scrollIntoView(false) → "end"
                        if b { "start" } else { "end" }.to_string()
                    } else if let Some(obj) = first.as_object() {
                        // scrollIntoView({ block: "center" })
                        obj.get(js_string!("block"), ctx)?
                            .as_string()
                            .map_or_else(|| "start".to_string(), |s| s.to_std_string_escaped())
                    } else {
                        "start".to_string()
                    }
                } else {
                    "start".to_string()
                };
                invoke_dom_handler_void(
                    "scrollIntoView",
                    entity,
                    &[ElidexJsValue::String(block)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("scrollIntoView"),
        1,
    );

    // Read-only numeric property accessors via macro.
    macro_rules! layout_getter {
        ($name:expr, $handler:expr) => {{
            let b = bridge.clone();
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| {
                    let entity = extract_entity(this, ctx)?;
                    invoke_dom_handler($handler, entity, &[], bridge)
                },
                b,
            )
            .to_js_function(realm)
        }};
    }

    init.accessor(
        js_string!("offsetWidth"),
        Some(layout_getter!("offsetWidth", "offsetWidth.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("offsetHeight"),
        Some(layout_getter!("offsetHeight", "offsetHeight.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("offsetTop"),
        Some(layout_getter!("offsetTop", "offsetTop.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("offsetLeft"),
        Some(layout_getter!("offsetLeft", "offsetLeft.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("offsetParent"),
        Some(layout_getter!("offsetParent", "offsetParent.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("clientWidth"),
        Some(layout_getter!("clientWidth", "clientWidth.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("clientHeight"),
        Some(layout_getter!("clientHeight", "clientHeight.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("clientTop"),
        Some(layout_getter!("clientTop", "clientTop.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("clientLeft"),
        Some(layout_getter!("clientLeft", "clientLeft.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("scrollWidth"),
        Some(layout_getter!("scrollWidth", "scrollWidth.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("scrollHeight"),
        Some(layout_getter!("scrollHeight", "scrollHeight.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("scrollTop"),
        Some(layout_getter!("scrollTop", "scrollTop.get")),
        None,
        Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("scrollLeft"),
        Some(layout_getter!("scrollLeft", "scrollLeft.get")),
        None,
        Attribute::CONFIGURABLE,
    );
}

// ---------------------------------------------------------------------------
// Dataset accessor
// ---------------------------------------------------------------------------

/// Register the `dataset` cached accessor.
pub(crate) fn register_dataset_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_cached_accessor(
        init,
        realm,
        bridge,
        "dataset",
        DATASET_CACHE_KEY,
        create_dataset_object,
    );
}

/// Create a dataset proxy object with get/set/delete methods.
pub(crate) fn create_dataset_object(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);
    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    // get(key)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let key = require_js_string_arg(args, 0, "dataset.get", ctx)?;
                invoke_dom_handler("dataset.get", entity, &[ElidexJsValue::String(key)], bridge)
            },
            b,
        ),
        js_string!("get"),
        1,
    );

    // set(key, value)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let key = require_js_string_arg(args, 0, "dataset.set", ctx)?;
                let value = require_js_string_arg(args, 1, "dataset.set", ctx)?;
                invoke_dom_handler_void(
                    "dataset.set",
                    entity,
                    &[ElidexJsValue::String(key), ElidexJsValue::String(value)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("set"),
        2,
    );

    // delete(key)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let key = require_js_string_arg(args, 0, "dataset.delete", ctx)?;
                invoke_dom_handler_void(
                    "dataset.delete",
                    entity,
                    &[ElidexJsValue::String(key)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("delete"),
        1,
    );

    init.build().into()
}

/// Register a cached read-only accessor (style, classList) on an element object.
///
/// The accessor returns a cached sub-object on subsequent accesses (identity
/// preservation: `el.style === el.style`). The `create_fn` builds the object
/// on first access, and it's stored under `cache_key` on the element wrapper.
pub(crate) fn register_cached_accessor(
    init: &mut ObjectInitializer<'_>,
    realm: &boa_engine::realm::Realm,
    bridge: &HostBridge,
    prop_name: &str,
    cache_key: &'static str,
    create_fn: fn(Entity, &HostBridge, &mut Context) -> JsValue,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("expected an element object"))?;
            // Return cached object if available.
            let cached = obj.get(js_string!(cache_key), ctx)?;
            if !cached.is_undefined() {
                return Ok(cached);
            }
            let entity = extract_entity(this, ctx)?;
            let val = create_fn(entity, bridge, ctx);
            // Cache on the element for identity preservation.
            obj.set(js_string!(cache_key), val.clone(), false, ctx)?;
            Ok(val)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(prop_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}

#[allow(clippy::too_many_lines, clippy::similar_names)] // classList registration boilerplate + getter/setter pairs
pub(crate) fn create_class_list_object(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);

    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    // add(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.add", ctx)?;
                invoke_dom_handler_void(
                    "classList.add",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("add"),
        1,
    );

    // remove(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.remove", ctx)?;
                invoke_dom_handler_void(
                    "classList.remove",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("remove"),
        1,
    );

    // toggle(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.toggle", ctx)?;
                invoke_dom_handler(
                    "classList.toggle",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("toggle"),
        1,
    );

    // contains(className)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "classList.contains", ctx)?;
                invoke_dom_handler(
                    "classList.contains",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("contains"),
        1,
    );

    // replace(oldClass, newClass)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let old = require_js_string_arg(args, 0, "classList.replace", ctx)?;
                let new = require_js_string_arg(args, 1, "classList.replace", ctx)?;
                invoke_dom_handler(
                    "classList.replace",
                    entity,
                    &[ElidexJsValue::String(old), ElidexJsValue::String(new)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("replace"),
        2,
    );

    // item(index)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let index = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler(
                    "classList.item",
                    entity,
                    &[ElidexJsValue::Number(index)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("item"),
        1,
    );

    // supports() — throws (not supported for classList)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler("classList.supports", entity, &[], bridge)
            },
            b,
        ),
        js_string!("supports"),
        1,
    );

    // value accessor (getter/setter).
    let realm = init.context().realm().clone();

    let b = bridge.clone();
    let val_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("classList.value.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    let b = bridge.clone();
    let val_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "classList.value.set",
                entity,
                &[ElidexJsValue::String(val)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(&realm);

    init.accessor(
        js_string!("value"),
        Some(val_getter),
        Some(val_setter),
        Attribute::CONFIGURABLE,
    );

    // length accessor (read-only).
    let b = bridge.clone();
    let len_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("classList.length", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    init.accessor(
        js_string!("length"),
        Some(len_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    init.build().into()
}
