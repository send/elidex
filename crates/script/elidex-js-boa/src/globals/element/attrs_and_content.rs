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

    // outerHTML getter + setter
    let b = bridge.clone();
    let oh_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            // outerHTML = opening tag + innerHTML + closing tag.
            let (tag, attrs_str, inner) = bridge.with(|session, dom| {
                let tag_name = dom
                    .world()
                    .get::<&elidex_ecs::TagType>(entity)
                    .map_or("div".to_string(), |t| t.0.clone());
                let attrs = dom
                    .world()
                    .get::<&elidex_ecs::Attributes>(entity)
                    .ok()
                    .map(|a| {
                        a.iter()
                            .map(|(k, v)| {
                                format!(
                                    " {}=\"{}\"",
                                    k,
                                    v.replace('&', "&amp;")
                                        .replace('"', "&quot;")
                                )
                            })
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                let handler = bridge.dom_registry().resolve("innerHTML.get");
                let inner_html = handler
                    .and_then(|h| h.invoke(entity, &[], session, dom).ok())
                    .and_then(|v| {
                        if let elidex_plugin::JsValue::String(s) = v {
                            Some(s)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                (tag_name, attrs, inner_html)
            });
            let html = format!("<{tag}{attrs_str}>{inner}</{tag}>");
            Ok(boa_engine::JsValue::from(js_string!(html.as_str())))
        },
        b,
    )
    .to_js_function(realm);

    let b = bridge.clone();
    let oh_setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            let html = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());
            // WHATWG HTML §3.1.5: throw if no parent or parent is Document.
            bridge.with(|session, dom| {
                let parent = dom.get_parent(entity);
                if parent.is_none() {
                    return;
                }
                let parent = parent.unwrap();
                // Parse HTML fragment and replace this element.
                let handler = bridge.dom_registry().resolve("innerHTML.set");
                if let Some(h) = handler {
                    // Create a temp container, set innerHTML, then replace.
                    let temp = dom.create_element("div", elidex_ecs::Attributes::default());
                    let _ = h.invoke(
                        temp,
                        &[ElidexJsValue::String(html)],
                        session,
                        dom,
                    );
                    // Move children from temp to before entity, then remove entity.
                    let mut child = dom.get_first_child(temp);
                    while let Some(c) = child {
                        let next = dom.get_next_sibling(c);
                        let _ = dom.insert_before(parent, c, entity);
                        child = next;
                    }
                    let _ = dom.destroy_entity(temp);
                }
                let _ = dom.destroy_entity(entity);
            });
            Ok(boa_engine::JsValue::undefined())
        },
        b,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!("outerHTML"),
        Some(oh_getter),
        Some(oh_setter),
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
