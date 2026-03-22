//! Tree navigation and node info accessors.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::{
    invoke_dom_handler, invoke_dom_handler_ref,
    invoke_dom_handler_void,
};
use super::core::extract_entity;

// ---------------------------------------------------------------------------
// Helper for read-only ref-returning accessors (tree navigation)
// ---------------------------------------------------------------------------

/// Register a read-only accessor that returns an element ref (or null) via `invoke_dom_handler_ref`.
pub(crate) fn reg_ref_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler_ref(handler, entity, &[], bridge, ctx)
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

/// Register a read-only accessor that returns a value (string/number/bool) via `invoke_dom_handler`.
pub(crate) fn reg_val_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler(handler, entity, &[], bridge)
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
// Tree navigation accessors
// ---------------------------------------------------------------------------

/// Register 10 read-only tree navigation accessors (parentNode, firstChild, etc.).
pub(crate) fn register_tree_nav_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    static NAV_PROPS: &[(&str, &str)] = &[
        ("parentNode", "parentNode.get"),
        ("parentElement", "parentElement.get"),
        ("firstChild", "firstChild.get"),
        ("lastChild", "lastChild.get"),
        ("nextSibling", "nextSibling.get"),
        ("previousSibling", "previousSibling.get"),
        ("firstElementChild", "firstElementChild.get"),
        ("lastElementChild", "lastElementChild.get"),
        ("nextElementSibling", "nextElementSibling.get"),
        ("previousElementSibling", "previousElementSibling.get"),
    ];
    for &(js_name, handler) in NAV_PROPS {
        reg_ref_accessor(init, bridge, realm, js_name, handler);
    }
}

// ---------------------------------------------------------------------------
// Node info accessors
// ---------------------------------------------------------------------------

/// Register node info accessors (tagName, nodeName, nodeType, etc.) and hasChildNodes method.
#[allow(clippy::similar_names)] // Getter/setter pairs (e.g., tag_getter/tag_setter) intentionally similar
pub(crate) fn register_node_info_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // Read-only value accessors.
    reg_val_accessor(init, bridge, realm, "tagName", "tagName.get");
    reg_val_accessor(init, bridge, realm, "nodeName", "nodeName.get");
    reg_val_accessor(init, bridge, realm, "nodeType", "nodeType.get");
    reg_val_accessor(
        init,
        bridge,
        realm,
        "childElementCount",
        "childElementCount.get",
    );
    reg_val_accessor(init, bridge, realm, "isConnected", "isConnected.get");

    // ownerDocument — returns ref.
    reg_ref_accessor(init, bridge, realm, "ownerDocument", "ownerDocument.get");

    // nodeValue getter/setter.
    let b = bridge.clone();
    let nv_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("nodeValue.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let nv_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "nodeValue.set",
                entity,
                &[ElidexJsValue::String(text)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("nodeValue"),
        Some(nv_getter),
        Some(nv_setter),
        Attribute::CONFIGURABLE,
    );

    // hasChildNodes() method.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler("hasChildNodes", entity, &[], bridge)
            },
            b,
        ),
        js_string!("hasChildNodes"),
        0,
    );
}
