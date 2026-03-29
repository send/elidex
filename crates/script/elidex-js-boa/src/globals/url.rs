//! `URL` and `URLSearchParams` constructors (WHATWG URL §6.1, §6.2).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `URL` and `URLSearchParams` global constructors.
pub fn register_url_constructors(ctx: &mut Context, _bridge: &HostBridge) {
    // URL constructor: new URL(url, base?)
    ctx.register_global_builtin_callable(
        js_string!("URL"),
        1,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let url_str = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let base = args
                .get(1)
                .filter(|v| !v.is_undefined() && !v.is_null())
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped());

            let parsed = if let Some(base_str) = base {
                let base_url = url::Url::parse(&base_str).map_err(|_| {
                    JsNativeError::typ()
                        .with_message(format!("URL: invalid base URL: {base_str}"))
                })?;
                base_url.join(&url_str).map_err(|_| {
                    JsNativeError::typ()
                        .with_message(format!("URL: invalid URL: {url_str}"))
                })?
            } else {
                url::Url::parse(&url_str).map_err(|_| {
                    JsNativeError::typ()
                        .with_message(format!("URL: invalid URL: {url_str}"))
                })?
            };

            Ok(build_url_object(&parsed, ctx))
        }),
    )
    .expect("failed to register URL");

    // URLSearchParams constructor: new URLSearchParams(init?)
    ctx.register_global_builtin_callable(
        js_string!("URLSearchParams"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let init = args.first();
            let params = parse_search_params_init(init, ctx)?;
            Ok(build_search_params_object(&params, ctx))
        }),
    )
    .expect("failed to register URLSearchParams");
}

/// Build a JS URL object from a parsed `url::Url`.
fn build_url_object(url: &url::Url, ctx: &mut Context) -> JsValue {
    let href = url.as_str().to_string();
    let origin = url.origin().ascii_serialization();
    let protocol = format!("{}:", url.scheme());
    let host = url
        .host_str()
        .map(|h| {
            if let Some(port) = url.port() {
                format!("{h}:{port}")
            } else {
                h.to_string()
            }
        })
        .unwrap_or_default();
    let hostname = url.host_str().unwrap_or("").to_string();
    let port = url.port().map_or(String::new(), |p| p.to_string());
    let pathname = url.path().to_string();
    let search = url.query().map_or(String::new(), |q| format!("?{q}"));
    let hash = url.fragment().map_or(String::new(), |f| format!("#{f}"));

    // Build searchParams
    let params: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    let search_params = build_search_params_object(&params, ctx);

    let mut init = ObjectInitializer::new(ctx);

    init.property(js_string!("href"), JsValue::from(js_string!(href.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("origin"), JsValue::from(js_string!(origin.as_str())), Attribute::READONLY);
    init.property(js_string!("protocol"), JsValue::from(js_string!(protocol.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("host"), JsValue::from(js_string!(host.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("hostname"), JsValue::from(js_string!(hostname.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("port"), JsValue::from(js_string!(port.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("pathname"), JsValue::from(js_string!(pathname.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("search"), JsValue::from(js_string!(search.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("hash"), JsValue::from(js_string!(hash.as_str())), Attribute::CONFIGURABLE);
    init.property(js_string!("username"), JsValue::from(js_string!(url.username())), Attribute::CONFIGURABLE);
    init.property(js_string!("password"), JsValue::from(js_string!(url.password().unwrap_or(""))), Attribute::CONFIGURABLE);
    init.property(js_string!("searchParams"), search_params, Attribute::CONFIGURABLE);

    // toString() / toJSON() — return href from property.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("URL: not a URL object")
            })?;
            obj.get(js_string!("href"), ctx)
        }),
        js_string!("toString"),
        0,
    );
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("URL: not a URL object")
            })?;
            obj.get(js_string!("href"), ctx)
        }),
        js_string!("toJSON"),
        0,
    );

    init.build().into()
}

/// Build a JS URLSearchParams object from a list of (key, value) pairs.
fn build_search_params_object(params: &[(String, String)], ctx: &mut Context) -> JsValue {
    // Store params as a hidden string for mutation.
    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();

    let mut init = ObjectInitializer::new(ctx);

    // Store the encoded form for methods to use.
    init.property(
        js_string!("__params__"),
        JsValue::from(js_string!(encoded.as_str())),
        Attribute::empty(),
    );

    // get(name)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let params = get_params(this, ctx)?;
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let value = params.iter().find(|(k, _)| k == &name).map(|(_, v)| v.as_str());
            Ok(value.map_or(JsValue::null(), |v| JsValue::from(js_string!(v))))
        }),
        js_string!("get"),
        1,
    );

    // has(name)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let params = get_params(this, ctx)?;
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(params.iter().any(|(k, _)| k == &name)))
        }),
        js_string!("has"),
        1,
    );

    // toString() — without ? prefix (WHATWG URL §6.2).
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("URLSearchParams: not an object")
            })?;
            let encoded = obj.get(js_string!("__params__"), ctx)?;
            Ok(encoded)
        }),
        js_string!("toString"),
        0,
    );

    // forEach(callback)
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let params = get_params(this, ctx)?;
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                JsNativeError::typ().with_message("forEach: callback required")
            })?;
            for (key, value) in &params {
                let _ = callback.call(
                    &JsValue::undefined(),
                    &[
                        JsValue::from(js_string!(value.as_str())),
                        JsValue::from(js_string!(key.as_str())),
                    ],
                    ctx,
                );
            }
            Ok(JsValue::undefined())
        }),
        js_string!("forEach"),
        1,
    );

    init.build().into()
}

/// Parse URLSearchParams init argument (string, object, or entries).
fn parse_search_params_init(
    init: Option<&JsValue>,
    ctx: &mut Context,
) -> JsResult<Vec<(String, String)>> {
    let Some(val) = init else {
        return Ok(Vec::new());
    };
    if val.is_undefined() || val.is_null() {
        return Ok(Vec::new());
    }

    // String form: "key=value&key2=value2" (strip leading ?).
    if let Some(s) = val.as_string() {
        let s = s.to_std_string_escaped();
        let s = s.strip_prefix('?').unwrap_or(&s);
        return Ok(url::form_urlencoded::parse(s.as_bytes())
            .into_owned()
            .collect());
    }

    // Try to convert to string.
    let s = val.to_string(ctx)?.to_std_string_escaped();
    let s = s.strip_prefix('?').unwrap_or(&s);
    Ok(url::form_urlencoded::parse(s.as_bytes())
        .into_owned()
        .collect())
}

/// Get params from the hidden __params__ property.
fn get_params(this: &JsValue, ctx: &mut Context) -> JsResult<Vec<(String, String)>> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("URLSearchParams: not an object")
    })?;
    let encoded = obj
        .get(js_string!("__params__"), ctx)?
        .to_string(ctx)?
        .to_std_string_escaped();
    Ok(url::form_urlencoded::parse(encoded.as_bytes())
        .into_owned()
        .collect())
}
