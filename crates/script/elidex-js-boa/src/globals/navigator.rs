//! `navigator` global object (WHATWG HTML §7.11.1).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `navigator` as a global property.
pub fn register_navigator(ctx: &mut Context, _bridge: &HostBridge) {
    let language = detect_language();

    // Pre-build values that need ctx before ObjectInitializer borrows it.
    let languages_arr = boa_engine::object::builtins::JsArray::new(ctx);
    let _ = languages_arr.push(JsValue::from(js_string!(language.as_str())), ctx);
    let languages_val: JsValue = languages_arr.into();
    let permissions_val = build_permissions_object(ctx);

    let mut init = ObjectInitializer::new(ctx);

    // userAgent
    init.property(
        js_string!("userAgent"),
        JsValue::from(js_string!("elidex/0.1")),
        Attribute::CONFIGURABLE,
    );

    // platform
    let platform = match std::env::consts::OS {
        "macos" => "MacIntel",
        "linux" => "Linux x86_64",
        "windows" => "Win32",
        other => other,
    };
    init.property(
        js_string!("platform"),
        JsValue::from(js_string!(platform)),
        Attribute::CONFIGURABLE,
    );

    // language
    init.property(
        js_string!("language"),
        JsValue::from(js_string!(language.as_str())),
        Attribute::CONFIGURABLE,
    );

    // languages
    init.property(
        js_string!("languages"),
        languages_val,
        Attribute::CONFIGURABLE,
    );

    // onLine
    init.property(
        js_string!("onLine"),
        JsValue::from(true),
        Attribute::CONFIGURABLE,
    );

    // cookieEnabled
    init.property(
        js_string!("cookieEnabled"),
        JsValue::from(true),
        Attribute::CONFIGURABLE,
    );

    // hardwareConcurrency
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!("hardwareConcurrency"),
        JsValue::from(cores as f64),
        Attribute::CONFIGURABLE,
    );

    // javaEnabled()
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::from(false))),
        js_string!("javaEnabled"),
        0,
    );

    // vendor
    init.property(
        js_string!("vendor"),
        JsValue::from(js_string!("")),
        Attribute::CONFIGURABLE,
    );

    // permissions.query({name}) → Promise<PermissionStatus>
    init.property(
        js_string!("permissions"),
        permissions_val,
        Attribute::CONFIGURABLE,
    );

    let navigator = init.build();
    ctx.register_global_property(js_string!("navigator"), navigator, Attribute::all())
        .expect("failed to register navigator");
}

/// Detect the system language (BCP 47 tag).
fn detect_language() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
}

/// Build the `navigator.permissions` object with `query()` method.
fn build_permissions_object(ctx: &mut Context) -> JsValue {
    let mut init = ObjectInitializer::new(ctx);

    // permissions.query({name}) → Promise<PermissionStatus>
    init.function(
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let name = args
                .first()
                .and_then(JsValue::as_object)
                .map(|obj| obj.get(js_string!("name"), ctx))
                .transpose()?
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            // Determine state based on permission name.
            let state = match name.as_str() {
                "clipboard-read" | "clipboard-write" => "granted",
                _ => "denied",
            };

            // Build PermissionStatus object.
            let mut status = ObjectInitializer::new(ctx);
            status.property(
                js_string!("state"),
                JsValue::from(js_string!(state)),
                Attribute::CONFIGURABLE,
            );
            status.property(
                js_string!("name"),
                JsValue::from(js_string!(name.as_str())),
                Attribute::CONFIGURABLE,
            );
            let status_obj = status.build();

            // Return resolved Promise.
            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::from(status_obj), ctx);
            Ok(promise.into())
        }),
        js_string!("query"),
        1,
    );

    init.build().into()
}
