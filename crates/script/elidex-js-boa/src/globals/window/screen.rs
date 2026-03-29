//! `screen` object and window properties (name, self, closed, DPR, etc.).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::{Attribute, PropertyDescriptorBuilder};
use boa_engine::{js_string, Context, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `screen` object and additional window properties (M4-4.5 Step 8).
#[allow(clippy::similar_names, clippy::too_many_lines)]
pub(super) fn register_screen_and_window_props(ctx: &mut Context, bridge: &HostBridge) {
    const CHROME_OVERHEAD: f64 = 64.0;

    let global = ctx.global_object();

    // --- screen object ---
    let mut screen_init = ObjectInitializer::new(ctx);

    // screen.width / screen.height — monitor resolution from bridge.
    let b = bridge.clone();
    let realm = screen_init.context().realm().clone();
    let sw_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(f64::from(bridge.monitor_width())))
        },
        b,
    )
    .to_js_function(&realm);

    let b = bridge.clone();
    let sh_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(f64::from(bridge.monitor_height())))
        },
        b,
    )
    .to_js_function(&realm);

    for name in ["width", "availWidth"] {
        screen_init.accessor(
            js_string!(name),
            Some(sw_getter.clone()),
            None,
            Attribute::CONFIGURABLE,
        );
    }
    for name in ["height", "availHeight"] {
        screen_init.accessor(
            js_string!(name),
            Some(sh_getter.clone()),
            None,
            Attribute::CONFIGURABLE,
        );
    }

    // colorDepth / pixelDepth — from bridge (default 24 for 8-bit-per-channel).
    let b = bridge.clone();
    let cd_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.color_depth())),
        b,
    )
    .to_js_function(&realm);
    for name in ["colorDepth", "pixelDepth"] {
        screen_init.accessor(
            js_string!(name),
            Some(cd_getter.clone()),
            None,
            Attribute::CONFIGURABLE,
        );
    }

    let screen_obj = screen_init.build();
    global
        .set(js_string!("screen"), JsValue::from(screen_obj), false, ctx)
        .expect("failed to register screen");

    // --- window.name (getter/setter) ---
    let b = bridge.clone();
    let name_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(js_string!(bridge.window_name()))),
        b,
    );
    let b = bridge.clone();
    let name_setter = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map_or(String::new(), |s| s.to_std_string_escaped());
            bridge.set_window_name(name);
            Ok(JsValue::undefined())
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(name_getter.to_js_function(&realm))
        .set(name_setter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("name"), desc, ctx)
        .expect("failed to register window.name");

    // --- Simple window properties as globals ---
    // window.self / window.window — self-reference.
    global
        .set(
            js_string!("self"),
            JsValue::from(global.clone()),
            false,
            ctx,
        )
        .expect("failed to register window.self");
    global
        .set(
            js_string!("window"),
            JsValue::from(global.clone()),
            false,
            ctx,
        )
        .expect("failed to register window.window");

    // window.closed
    global
        .set(js_string!("closed"), JsValue::from(false), false, ctx)
        .expect("failed to register window.closed");

    // window.devicePixelRatio — dynamic getter from bridge.
    let b = bridge.clone();
    let dpr_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(f64::from(bridge.device_pixel_ratio())))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(dpr_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("devicePixelRatio"), desc, ctx)
        .expect("failed to register window.devicePixelRatio");

    // window.outerWidth / outerHeight — viewport + chrome bar heights.
    // CHROME_HEIGHT (36.0) + TAB_BAR_HEIGHT (28.0) = 64.0 total chrome.
    let b = bridge.clone();
    let ow_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(f64::from(bridge.viewport_width())))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(ow_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("outerWidth"), desc, ctx)
        .expect("failed to register window.outerWidth");

    let b = bridge.clone();
    let oh_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            #[allow(clippy::cast_precision_loss)]
            Ok(JsValue::from(
                f64::from(bridge.viewport_height()) + CHROME_OVERHEAD,
            ))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(oh_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("outerHeight"), desc, ctx)
        .expect("failed to register window.outerHeight");

    // window.screenX/Y, screenLeft/Top — dynamic getters from bridge.
    let b = bridge.clone();
    let sx_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.screen_x())),
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(sx_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("screenX"), desc.clone(), ctx)
        .expect("failed to register window.screenX");
    global
        .define_property_or_throw(js_string!("screenLeft"), desc, ctx)
        .expect("failed to register window.screenLeft");

    let b = bridge.clone();
    let sy_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.screen_y())),
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(sy_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("screenY"), desc.clone(), ctx)
        .expect("failed to register window.screenY");
    global
        .define_property_or_throw(js_string!("screenTop"), desc, ctx)
        .expect("failed to register window.screenTop");

    // window.isSecureContext
    let b = bridge.clone();
    let isc_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            let is_secure = bridge
                .current_url()
                .is_some_and(|url| super::is_potentially_trustworthy_url(&url));
            // WHATWG HTML §3.4: an iframe context is secure only if both the
            // current context and all ancestor contexts are secure. If this
            // document is in an iframe (frame_element is Some), also check:
            // - the origin must not be opaque (sandboxed without allow-same-origin)
            // - the parent origin must be a tuple origin (non-opaque)
            let is_secure = is_secure && {
                if bridge.frame_element().is_some() {
                    let origin = bridge.origin();
                    match origin {
                        elidex_plugin::SecurityOrigin::Opaque(_) => false,
                        elidex_plugin::SecurityOrigin::Tuple { ref scheme, .. } => {
                            scheme == "https" || scheme == "file"
                        }
                    }
                } else {
                    true
                }
            };
            Ok(JsValue::from(is_secure))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(isc_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("isSecureContext"), desc, ctx)
        .expect("failed to register window.isSecureContext");

    // window.origin
    let b = bridge.clone();
    let origin_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| {
            let origin = bridge
                .current_url()
                .map_or("null".to_string(), |url| url.origin().ascii_serialization());
            Ok(JsValue::from(js_string!(origin)))
        },
        b,
    );
    let realm = ctx.realm().clone();
    let desc = PropertyDescriptorBuilder::new()
        .get(origin_getter.to_js_function(&realm))
        .configurable(true)
        .enumerable(true)
        .build();
    global
        .define_property_or_throw(js_string!("origin"), desc, ctx)
        .expect("failed to register window.origin");

    // window.focus() — requests window focus via IPC (WHATWG HTML §7.2.7.1).
    let b_focus = bridge.clone();
    ctx.register_global_builtin_callable(
        js_string!("focus"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| {
                bridge.request_focus();
                Ok(JsValue::undefined())
            },
            b_focus,
        ),
    )
    .expect("failed to register window.focus");

    // window.blur() — no-op per WHATWG HTML §7.2.7.1.
    ctx.register_global_builtin_callable(
        js_string!("blur"),
        0,
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::undefined())),
    )
    .expect("failed to register window.blur");

    // window.stop() — abort pending fetches + cancel timers (WHATWG HTML §7.1.5).
    let b = bridge.clone();
    ctx.register_global_builtin_callable(
        js_string!("stop"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| {
                bridge.clear_all_timers();
                Ok(JsValue::undefined())
            },
            b,
        ),
    )
    .expect("failed to register window.stop");
}
