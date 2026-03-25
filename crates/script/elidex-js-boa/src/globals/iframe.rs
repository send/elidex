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
    let b_cd = bridge.clone();
    let cd_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, _bridge, _ctx| Ok(JsValue::null()),
        b_cd,
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
    let b_content_window = bridge.clone();
    let content_window_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, _bridge, _ctx| Ok(JsValue::null()),
        b_content_window,
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
    let b_load = bridge.clone();
    let load_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let loading = dom
                    .world()
                    .get::<&elidex_ecs::IframeData>(entity)
                    .ok()
                    .map_or("eager", |d| match d.loading {
                        elidex_ecs::LoadingAttribute::Eager => "eager",
                        elidex_ecs::LoadingAttribute::Lazy => "lazy",
                    });
                Ok(JsValue::from(js_string!(loading)))
            })
        },
        b_load,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!("loading"),
        Some(load_getter),
        None,
        boa_engine::property::Attribute::CONFIGURABLE,
    );

    // --- allowFullscreen (boolean) ---
    let b_af = bridge.clone();
    let af_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let entity = extract_entity(this, ctx)?;
            bridge.with(|_session, dom| {
                let val = dom
                    .world()
                    .get::<&elidex_ecs::IframeData>(entity)
                    .ok()
                    .is_some_and(|d| d.allow_fullscreen);
                Ok(JsValue::from(val))
            })
        },
        b_af,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!("allowFullscreen"),
        Some(af_getter),
        None,
        boa_engine::property::Attribute::CONFIGURABLE,
    );

    // --- sandbox (DOMTokenList-like string getter) ---
    // Full DOMTokenList is complex; MVP returns the raw attribute string.
    register_iframe_string_attr(init, bridge, realm, "sandbox");
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
                // Update the Attributes component (mirrors setAttribute behavior).
                if let Ok(mut attrs) = dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity) {
                    attrs.set(attr_name, &value);
                }
                // Update the IframeData component field directly.
                if let Ok(mut iframe_data) =
                    dom.world_mut().get::<&mut elidex_ecs::IframeData>(entity)
                {
                    set_iframe_attr(&mut iframe_data, attr_name, &value);
                }
                // Record a SetAttribute mutation so detect_iframe_mutations
                // picks up the change (especially for `src` re-navigation).
                session.record_mutation(elidex_script_session::Mutation::SetAttribute {
                    entity,
                    name: attr_name.to_string(),
                    value: value.clone(),
                });
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
        _ => None,
    }
}
