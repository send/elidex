//! Content accessors (textContent, innerHTML), style, classList, event listener
//! registration on element objects.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;

use super::accessors::{create_class_list_object, register_cached_accessor};
use super::core::extract_entity;
use super::CLASSLIST_CACHE_KEY;
use super::STYLE_CACHE_KEY;
use crate::bridge::HostBridge;
use crate::globals::{invoke_dom_handler, invoke_dom_handler_void};

/// Register textContent (getter/setter) and innerHTML (getter) accessors.
pub(crate) fn register_content_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // textContent getter
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("textContent.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    // textContent setter
    let b = bridge.clone();
    let setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let text = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            invoke_dom_handler_void(
                "textContent.set",
                entity,
                &[ElidexJsValue::String(text)],
                bridge,
            )
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("textContent"),
        Some(getter),
        Some(setter),
        Attribute::CONFIGURABLE,
    );

    // innerHTML getter + setter
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            invoke_dom_handler("innerHTML.get", entity, &[], bridge)
        },
        b,
    )
    .to_js_function(realm);

    let b2 = bridge.clone();
    let setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let html = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());
            invoke_dom_handler_void(
                "innerHTML.set",
                entity,
                &[ElidexJsValue::String(html)],
                bridge,
            )
        },
        b2,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("innerHTML"),
        Some(getter),
        Some(setter),
        Attribute::CONFIGURABLE,
    );
}

/// Register the `style` cached accessor.
pub(crate) fn register_style_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_cached_accessor(
        init,
        realm,
        bridge,
        "style",
        STYLE_CACHE_KEY,
        crate::globals::window::create_style_object,
    );
}

/// Register the `classList` cached accessor.
pub(crate) fn register_class_list_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    register_cached_accessor(
        init,
        realm,
        bridge,
        "classList",
        CLASSLIST_CACHE_KEY,
        create_class_list_object,
    );
}

/// Register addEventListener and removeEventListener methods.
pub(crate) fn register_event_listener_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                crate::globals::add_event_listener_for(entity, args, bridge, ctx)
            },
            b,
        ),
        js_string!("addEventListener"),
        2,
    );

    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                crate::globals::remove_event_listener_for(entity, args, bridge, ctx)
            },
            b,
        ),
        js_string!("removeEventListener"),
        2,
    );

    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                crate::globals::dispatch_event_for(entity, args, bridge, ctx)
            },
            b,
        ),
        js_string!("dispatchEvent"),
        1,
    );
}
