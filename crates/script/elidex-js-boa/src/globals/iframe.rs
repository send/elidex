//! HTMLIFrameElement-specific JS property registration (WHATWG HTML §4.8.5).
//!
//! Registers `contentDocument`, `contentWindow`, `src`, `srcdoc`, `sandbox`,
//! `width`, `height`, `name`, `loading`, `allowFullscreen`, `referrerPolicy`,
//! and `allow` accessors on iframe element wrappers.

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;
use crate::globals::element::extract_entity;

/// Register iframe-specific accessors on an element wrapper.
///
/// Called from `build_element_object` when the element's tag is `iframe`.
pub(crate) fn register_iframe_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    // --- contentDocument (read-only) ---
    // Same-origin: should return iframe's document object.
    // Cross-origin: returns null (WHATWG HTML §4.8.5).
    //
    // Boa limitation: each iframe has its own JsRuntime with separate boa Context.
    // Objects from one Context can't be used in another. contentDocument would need
    // to return a Document object from the iframe's Context into the parent's Context,
    // which boa doesn't support. Returns null (cross-origin behavior) for all cases.
    // Self-hosted JS engine (M4-9+) will implement proper cross-context document proxies.
    let cd_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, _captures, _ctx| Ok(JsValue::null()),
        (),
    )
    .to_js_function(realm);
    init.accessor(
        js_string!("contentDocument"),
        Some(cd_getter),
        None,
        boa_engine::property::Attribute::CONFIGURABLE,
    );

    // --- contentWindow (read-only) ---
    // Same-origin: should return iframe's window proxy.
    // Cross-origin: returns restricted window proxy (postMessage only).
    // Boa limitation: same as contentDocument — cross-context object sharing not supported.
    // Returns null for all cases. Self-hosted engine (M4-9+) will implement window proxies.
    let content_window_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, _captures, _ctx| Ok(JsValue::null()),
        (),
    )
    .to_js_function(realm);
    init.accessor(
        js_string!("contentWindow"),
        Some(content_window_getter),
        None,
        boa_engine::property::Attribute::CONFIGURABLE,
    );

    // --- Attribute accessors (read from IframeData ECS component) ---
    register_iframe_string_attr(init, bridge, realm, "src");
    register_iframe_string_attr(init, bridge, realm, "srcdoc");
    register_iframe_string_attr(init, bridge, realm, "name");
    register_iframe_string_attr(init, bridge, realm, "referrerPolicy");
    register_iframe_string_attr(init, bridge, realm, "allow");

    // --- width / height (string attributes per HTML spec) ---
    register_iframe_string_attr(init, bridge, realm, "width");
    register_iframe_string_attr(init, bridge, realm, "height");

    // --- loading (string: "eager" | "lazy") ---
    register_iframe_string_attr(init, bridge, realm, "loading");

    // --- allowFullscreen (boolean IDL attribute, WHATWG HTML §4.8.5) ---
    {
        let b_get = bridge.clone();
        let getter = NativeFunction::from_copy_closure_with_captures(
            move |this, _args, bridge, ctx| -> JsResult<JsValue> {
                let entity = extract_entity(this, ctx)?;
                bridge.with(|_session, dom| {
                    let allowed = dom
                        .world()
                        .get::<&elidex_ecs::IframeData>(entity)
                        .map(|d| d.allow_fullscreen)
                        .unwrap_or(false);
                    Ok(JsValue::from(allowed))
                })
            },
            b_get,
        )
        .to_js_function(realm);

        let b_set = bridge.clone();
        let setter = NativeFunction::from_copy_closure_with_captures(
            move |this, args, bridge, ctx| -> JsResult<JsValue> {
                let entity = extract_entity(this, ctx)?;
                let value = args.first().is_some_and(JsValue::to_boolean);
                bridge.with(|session, dom| {
                    // Record mutation FIRST so flush() captures correct old_value.
                    if value {
                        session.record_mutation(elidex_script_session::Mutation::SetAttribute {
                            entity,
                            name: "allowfullscreen".to_string(),
                            value: String::new(),
                        });
                    } else {
                        session.record_mutation(elidex_script_session::Mutation::RemoveAttribute {
                            entity,
                            name: "allowfullscreen".to_string(),
                        });
                    }
                    // Update IframeData eagerly (not tracked by MutationObserver).
                    if let Ok(mut iframe_data) =
                        dom.world_mut().get::<&mut elidex_ecs::IframeData>(entity)
                    {
                        iframe_data.allow_fullscreen = value;
                    }
                });
                Ok(JsValue::undefined())
            },
            b_set,
        )
        .to_js_function(realm);

        init.accessor(
            js_string!("allowFullscreen"),
            Some(getter),
            Some(setter),
            boa_engine::property::Attribute::CONFIGURABLE,
        );
    }

    // --- sandbox (DOMTokenList-like string getter) ---
    // Full DOMTokenList is complex; MVP returns the raw attribute string.
    register_iframe_string_attr(init, bridge, realm, "sandbox");
}

/// Map an IDL property name to the corresponding HTML content attribute name.
///
/// HTML attribute names are all-lowercase (e.g. `referrerpolicy`, `allowfullscreen`),
/// while IDL property names use camelCase (e.g. `referrerPolicy`, `allowFullscreen`).
/// This mapping ensures that `setAttribute` and `Attributes` use the correct
/// lowercase name as required by the WHATWG HTML spec.
fn idl_to_content_attr(idl_name: &str) -> &str {
    match idl_name {
        "referrerPolicy" => "referrerpolicy",
        "allowFullscreen" => "allowfullscreen",
        _ => idl_name,
    }
}

/// Register a string attribute getter and setter for an iframe property.
///
/// Getter reads the corresponding field from the `IframeData` ECS component.
/// Setter updates the `Attributes` component (like `setAttribute`) and records
/// a `SetAttribute` mutation. When `src` is set, this triggers re-navigation
/// via `detect_iframe_mutations` in the content thread's re-render cycle.
fn register_iframe_string_attr(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    attr_name: &'static str,
) {
    let b_get = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| -> JsResult<JsValue> {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                // For width/height, read from Attributes directly so that
                // non-numeric values round-trip correctly through JS reflection.
                // IframeData.width/height are u32 and silently drop non-numeric values.
                if attr_name == "width" || attr_name == "height" {
                    let value = dom
                        .world()
                        .get::<&elidex_ecs::Attributes>(entity)
                        .ok()
                        .and_then(|attrs| attrs.get(attr_name).map(ToString::to_string))
                        .unwrap_or_default();
                    return Ok(JsValue::from(js_string!(value)));
                }
                let iframe_ref = dom.world().get::<&elidex_ecs::IframeData>(entity).ok();
                let value = iframe_ref
                    .as_ref()
                    .and_then(|d| get_iframe_attr(d, attr_name))
                    .unwrap_or_default();
                Ok(JsValue::from(js_string!(value)))
            })
        },
        b_get,
    )
    .to_js_function(realm);

    let b_set = bridge.clone();
    let setter = NativeFunction::from_copy_closure_with_captures(
        move |this, args, bridge, ctx| -> JsResult<JsValue> {
            let entity = extract_entity(this, ctx)?;
            let value = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            bridge.with(|session, dom| {
                let html_attr = idl_to_content_attr(attr_name);
                // Record the mutation FIRST so that flush() captures the correct
                // old_value from the current Attributes state (before modification).
                // The mutation application in flush() will update Attributes.
                session.record_mutation(elidex_script_session::Mutation::SetAttribute {
                    entity,
                    name: html_attr.to_string(),
                    value: value.clone(),
                });
                // Update IframeData eagerly (not tracked by MutationObserver,
                // needed immediately for layout intrinsic sizing).
                if let Ok(mut iframe_data) =
                    dom.world_mut().get::<&mut elidex_ecs::IframeData>(entity)
                {
                    set_iframe_attr(&mut iframe_data, attr_name, &value);
                }
            });
            Ok(JsValue::undefined())
        },
        b_set,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(attr_name),
        Some(getter),
        Some(setter),
        boa_engine::property::Attribute::CONFIGURABLE,
    );
}

/// Set an `IframeData` field from an attribute name and value.
fn set_iframe_attr(data: &mut elidex_ecs::IframeData, attr_name: &str, value: &str) {
    match attr_name {
        "src" => data.src = Some(value.to_string()),
        "srcdoc" => data.srcdoc = Some(value.to_string()),
        "name" => data.name = Some(value.to_string()),
        "referrerPolicy" => data.referrer_policy = Some(value.to_string()),
        "allow" => data.allow = Some(value.to_string()),
        "sandbox" => data.sandbox = Some(value.to_string()),
        "width" => data.width = value.parse().unwrap_or(data.width),
        "height" => data.height = value.parse().unwrap_or(data.height),
        "loading" => {
            data.loading = if value.eq_ignore_ascii_case("lazy") {
                elidex_ecs::LoadingAttribute::Lazy
            } else {
                elidex_ecs::LoadingAttribute::Eager
            };
        }
        "allowFullscreen" => {
            data.allow_fullscreen = !value.is_empty();
        }
        _ => {}
    }
}

/// Map an attribute name to its `IframeData` field value.
fn get_iframe_attr(data: &elidex_ecs::IframeData, attr_name: &str) -> Option<String> {
    match attr_name {
        "src" => data.src.clone(),
        "srcdoc" => data.srcdoc.clone(),
        "name" => data.name.clone(),
        "referrerPolicy" => data.referrer_policy.clone(),
        "allow" => data.allow.clone(),
        "sandbox" => data.sandbox.clone(),
        "width" => Some(data.width.to_string()),
        "height" => Some(data.height.to_string()),
        "loading" => Some(
            match data.loading {
                elidex_ecs::LoadingAttribute::Lazy => "lazy",
                elidex_ecs::LoadingAttribute::Eager => "eager",
            }
            .to_string(),
        ),
        "allowFullscreen" => Some(if data.allow_fullscreen {
            String::new() // Boolean attribute: present = empty string
        } else {
            return None; // Boolean attribute: absent = None
        }),
        _ => None,
    }
}
