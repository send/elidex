//! `CharacterData` methods, `Attr` node methods, `DocumentType` wrapper, and
//! `resolve_object_ref` utility.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use elidex_ecs::Entity;
use elidex_plugin::JsValue as ElidexJsValue;
use elidex_script_session::JsObjectRef;

use super::core::{create_element_wrapper, entity_bits_as_f64, extract_entity};
use super::tree_nav::{
    reg_ref_accessor, reg_val_accessor, register_node_info_accessors, register_tree_nav_accessors,
};
use super::ENTITY_KEY;
use crate::bridge::HostBridge;
use crate::globals::{
    boa_arg_to_elidex, invoke_dom_handler, invoke_dom_handler_ref, invoke_dom_handler_void,
    require_js_string_arg,
};
use crate::value_conv;

// ---------------------------------------------------------------------------
// CharacterData methods (data, length, substringData, appendData, etc.)
// ---------------------------------------------------------------------------

/// Register `CharacterData` interface methods on the node wrapper.
///
/// Registered on all node wrappers (including Elements). On non-`CharacterData`
/// nodes (i.e., anything other than Text/Comment), the handler layer returns
/// `InvalidStateError`, matching browser behavior where `CharacterData`
/// methods exist on the prototype chain but throw on incorrect node types.
#[allow(clippy::too_many_lines)]
pub(crate) fn register_char_data_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // data getter
    let b = bridge.clone();
    let data_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("data.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    // data setter
    let b = bridge.clone();
    let data_set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void("data.set", entity, &[ElidexJsValue::String(text)], bridge)
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("data"),
        Some(data_getter),
        Some(data_set_fn),
        Attribute::CONFIGURABLE,
    );

    // length getter (read-only)
    reg_val_accessor(init, bridge, realm, "length", "length.get");

    // substringData(offset, count)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let count = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler(
                    "substringData",
                    entity,
                    &[ElidexJsValue::Number(offset), ElidexJsValue::Number(count)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("substringData"),
        2,
    );

    // appendData(data)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let data = require_js_string_arg(args, 0, "appendData", ctx)?;
                invoke_dom_handler_void(
                    "appendData",
                    entity,
                    &[ElidexJsValue::String(data)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("appendData"),
        1,
    );

    // insertData(offset, data)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let data = require_js_string_arg(args, 1, "insertData", ctx)?;
                invoke_dom_handler_void(
                    "insertData",
                    entity,
                    &[ElidexJsValue::Number(offset), ElidexJsValue::String(data)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("insertData"),
        2,
    );

    // deleteData(offset, count)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let count = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler_void(
                    "deleteData",
                    entity,
                    &[ElidexJsValue::Number(offset), ElidexJsValue::Number(count)],
                    bridge,
                )
            },
            b,
        ),
        js_string!("deleteData"),
        2,
    );

    // replaceData(offset, count, data)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let count = args
                    .get(1)
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                let data = require_js_string_arg(args, 2, "replaceData", ctx)?;
                invoke_dom_handler_void(
                    "replaceData",
                    entity,
                    &[
                        ElidexJsValue::Number(offset),
                        ElidexJsValue::Number(count),
                        ElidexJsValue::String(data),
                    ],
                    bridge,
                )
            },
            b,
        ),
        js_string!("replaceData"),
        3,
    );

    // splitText(offset) — Text nodes only
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let offset = args
                    .first()
                    .map(|v| v.to_number(ctx))
                    .transpose()?
                    .unwrap_or(0.0);
                invoke_dom_handler_ref(
                    "splitText",
                    entity,
                    &[ElidexJsValue::Number(offset)],
                    bridge,
                    ctx,
                )
            },
            b,
        ),
        js_string!("splitText"),
        1,
    );
}

// ---------------------------------------------------------------------------
// Attr node methods (getAttributeNode, setAttributeNode, removeAttributeNode)
// ---------------------------------------------------------------------------

/// Register Attr-related element methods.
pub(crate) fn register_attr_node_methods(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    // getAttributeNode(name) → Attr object or null
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let name = require_js_string_arg(args, 0, "getAttributeNode", ctx)?;
                let result = invoke_dom_handler_ref(
                    "getAttributeNode",
                    entity,
                    &[ElidexJsValue::String(name)],
                    bridge,
                    ctx,
                )?;
                // Wrap the returned ObjectRef as an Attr object if not null.
                if result.is_null() || result.is_undefined() {
                    return Ok(JsValue::null());
                }
                // The result is already an element wrapper via invoke_dom_handler_ref.
                // Re-wrap it as an Attr-specific object by building attr accessors.
                if let Some(obj) = result.as_object() {
                    let entity_val = obj.get(js_string!(ENTITY_KEY), ctx)?;
                    if !entity_val.is_undefined() {
                        let attr_entity = extract_entity(&result, ctx)?;
                        return Ok(create_attr_object(attr_entity, bridge, ctx));
                    }
                }
                Ok(result)
            },
            b,
        ),
        js_string!("getAttributeNode"),
        1,
    );

    // setAttributeNode(attr) → old Attr or null
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let attr_arg = args.first().ok_or_else(|| {
                    JsNativeError::typ().with_message("setAttributeNode requires an Attr argument")
                })?;
                let attr_elidex = boa_arg_to_elidex(attr_arg, bridge, ctx)?;
                invoke_dom_handler_ref("setAttributeNode", entity, &[attr_elidex], bridge, ctx)
            },
            b,
        ),
        js_string!("setAttributeNode"),
        1,
    );

    // removeAttributeNode(attr) → removed Attr
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let attr_arg = args.first().ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("removeAttributeNode requires an Attr argument")
                })?;
                let attr_elidex = boa_arg_to_elidex(attr_arg, bridge, ctx)?;
                invoke_dom_handler_ref("removeAttributeNode", entity, &[attr_elidex], bridge, ctx)
            },
            b,
        ),
        js_string!("removeAttributeNode"),
        1,
    );
}

// ---------------------------------------------------------------------------
// Attr wrapper object
// ---------------------------------------------------------------------------

/// Create a JS wrapper object for an `Attr` node entity.
///
/// Provides `name` (getter), `value` (getter/setter), `ownerElement` (getter),
/// and `specified` (getter) — matching the WHATWG `Attr` interface.
pub fn create_attr_object(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);
    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    let realm = init.context().realm().clone();

    // name (getter)
    reg_val_accessor(&mut init, bridge, &realm, "name", "attr.name.get");

    // value (getter/setter)
    let b = bridge.clone();
    let val_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("attr.value.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(&realm);

    let b = bridge.clone();
    let val_set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "attr.value.set",
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
        Some(val_set_fn),
        Attribute::CONFIGURABLE,
    );

    // ownerElement (getter) — returns element ref or null
    reg_ref_accessor(
        &mut init,
        bridge,
        &realm,
        "ownerElement",
        "attr.ownerElement.get",
    );

    // specified (getter) — always true per modern spec
    reg_val_accessor(&mut init, bridge, &realm, "specified", "attr.specified.get");

    init.build().into()
}

// ---------------------------------------------------------------------------
// DocumentType wrapper object
// ---------------------------------------------------------------------------

/// Create a JS wrapper object for a `DocumentType` node entity.
///
/// Provides `name`, `publicId`, and `systemId` getters — matching the
/// WHATWG `DocumentType` interface.
#[allow(dead_code)] // M4-3.10: Will be used when resolving DocumentType entity wrappers in iframe/multi-Document support.
pub fn create_doctype_object(entity: Entity, bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let entity_bits = entity_bits_as_f64(entity);
    let mut init = ObjectInitializer::new(ctx);
    init.property(
        js_string!(ENTITY_KEY),
        JsValue::from(entity_bits),
        Attribute::empty(),
    );

    let realm = init.context().realm().clone();

    reg_val_accessor(&mut init, bridge, &realm, "name", "doctype.name.get");
    reg_val_accessor(
        &mut init,
        bridge,
        &realm,
        "publicId",
        "doctype.publicId.get",
    );
    reg_val_accessor(
        &mut init,
        bridge,
        &realm,
        "systemId",
        "doctype.systemId.get",
    );

    // Also register tree nav + node info so DocumentType nodes are navigable.
    register_tree_nav_accessors(&mut init, bridge, &realm);
    register_node_info_accessors(&mut init, bridge, &realm);

    init.build().into()
}

/// Resolve an elidex `JsValue::ObjectRef` to a boa element wrapper.
///
/// Used by document methods (querySelector, getElementById, createElement)
/// to return element objects to JS.
pub fn resolve_object_ref(
    result: &ElidexJsValue,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    match result {
        ElidexJsValue::ObjectRef(id) => {
            let obj_ref = JsObjectRef::from_raw(*id);
            bridge.with(|session, _dom| {
                if let Some((entity, _kind)) = session.identity_map().get(obj_ref) {
                    create_element_wrapper(entity, bridge, obj_ref, ctx, false)
                } else {
                    JsValue::null()
                }
            })
        }
        ElidexJsValue::Null => JsValue::null(),
        other => value_conv::to_boa(other),
    }
}
