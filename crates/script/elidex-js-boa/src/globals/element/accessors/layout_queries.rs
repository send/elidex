//! Layout query methods and property accessors (getBoundingClientRect, offset*, client*, scroll*).

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::core::extract_entity;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void};

/// Read-only attribute for `DOMRect` properties.
const RO_ATTR: Attribute = Attribute::READONLY;

/// Register layout query method and property accessors on an element object.
#[allow(clippy::too_many_lines)]
#[allow(clippy::many_single_char_names)]
pub(in crate::globals::element) fn register_layout_query_accessors(
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
    // offsetParent returns an element reference (or null), not a number.
    // Use invoke_dom_handler_ref to resolve ObjectRef to an element wrapper.
    {
        use crate::globals::invoke_dom_handler_ref;
        let b = bridge.clone();
        let getter = NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_ref("offsetParent.get", entity, &[], bridge, ctx)
            },
            b,
        )
        .to_js_function(realm);
        init.accessor(
            js_string!("offsetParent"),
            Some(getter),
            None,
            Attribute::CONFIGURABLE,
        );
    }
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
