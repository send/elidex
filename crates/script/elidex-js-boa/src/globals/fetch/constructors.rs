//! `Request`, `Headers`, and `Response` constructors (WHATWG Fetch §5).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};

/// Register `Request`, `Headers`, and `Response` global constructors.
pub fn register_fetch_constructors(ctx: &mut Context) {
    // Headers constructor: new Headers(init?)
    ctx.register_global_builtin_callable(
        js_string!("Headers"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            build_headers_object(args.first(), ctx)
        }),
    )
    .expect("failed to register Headers");

    // Request constructor: new Request(input, init?)
    ctx.register_global_builtin_callable(
        js_string!("Request"),
        1,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            build_request_object(args, ctx)
        }),
    )
    .expect("failed to register Request");

    // Response constructor: new Response(body?, init?)
    ctx.register_global_builtin_callable(
        js_string!("Response"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            build_response_object(args, ctx)
        }),
    )
    .expect("failed to register Response");

    // Attach static methods to the Response constructor.
    let response_ctor = ctx
        .global_object()
        .get(js_string!("Response"), ctx)
        .expect("Response constructor must exist");
    if let Some(response_obj) = response_ctor.as_object() {
        // Response.error() — returns a network error Response (WHATWG Fetch §5.4).
        let _ = response_obj.set(
            js_string!("error"),
            NativeFunction::from_copy_closure(|_this, _args, ctx| {
                build_error_response(ctx)
            })
            .to_js_function(ctx.realm()),
            false,
            ctx,
        );

        // Response.redirect(url, status?) — returns a redirect Response (WHATWG Fetch §5.4).
        let _ = response_obj.set(
            js_string!("redirect"),
            NativeFunction::from_copy_closure(|_this, args, ctx| {
                build_redirect_response(args, ctx)
            })
            .to_js_function(ctx.realm()),
            false,
            ctx,
        );
    }
}

/// Build a mutable Headers object.
fn build_headers_object(
    _init: Option<&JsValue>,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    // Start with empty headers. Init parsing from objects requires
    // property enumeration which conflicts with ObjectInitializer borrow.
    // Headers populated via set/append after construction.
    let encoded = String::new();

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(
        js_string!("__headers__"),
        JsValue::from(js_string!(encoded.as_str())),
        Attribute::empty(),
    );

    // get(name) — combines duplicate headers per Fetch spec.
    init_obj.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let headers = parse_headers(this, ctx)?;
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped().to_ascii_lowercase())
                .unwrap_or_default();
            let values: Vec<&str> = headers
                .iter()
                .filter(|(k, _)| k.to_ascii_lowercase() == name)
                .map(|(_, v)| v.as_str())
                .collect();
            if values.is_empty() {
                Ok(JsValue::null())
            } else {
                Ok(JsValue::from(js_string!(values.join(", "))))
            }
        }),
        js_string!("get"),
        1,
    );

    // has(name)
    init_obj.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let headers = parse_headers(this, ctx)?;
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped().to_ascii_lowercase())
                .unwrap_or_default();
            Ok(JsValue::from(
                headers.iter().any(|(k, _)| k.to_ascii_lowercase() == name),
            ))
        }),
        js_string!("has"),
        1,
    );

    // set(name, value)
    init_obj.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let value = args
                .get(1)
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut headers = parse_headers(this, ctx)?;
            headers.retain(|(k, _)| k.to_ascii_lowercase() != name.to_ascii_lowercase());
            headers.push((name, value));
            store_headers(this, &headers, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("set"),
        2,
    );

    // append(name, value)
    init_obj.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let value = args
                .get(1)
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut headers = parse_headers(this, ctx)?;
            headers.push((name, value));
            store_headers(this, &headers, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("append"),
        2,
    );

    // delete(name)
    init_obj.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped().to_ascii_lowercase())
                .unwrap_or_default();
            let mut headers = parse_headers(this, ctx)?;
            headers.retain(|(k, _)| k.to_ascii_lowercase() != name);
            store_headers(this, &headers, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("delete"),
        1,
    );

    // forEach(callback)
    init_obj.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let headers = parse_headers(this, ctx)?;
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                JsNativeError::typ().with_message("Headers.forEach: callback required")
            })?;
            for (name, value) in &headers {
                let _ = callback.call(
                    &JsValue::undefined(),
                    &[
                        JsValue::from(js_string!(value.as_str())),
                        JsValue::from(js_string!(name.as_str())),
                    ],
                    ctx,
                );
            }
            Ok(JsValue::undefined())
        }),
        js_string!("forEach"),
        1,
    );

    Ok(init_obj.build().into())
}

/// Build a Request object.
fn build_request_object(
    args: &[JsValue],
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let url = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let init = args.get(1);

    let method = init
        .and_then(JsValue::as_object)
        .map(|obj| obj.get(js_string!("method"), ctx))
        .transpose()?
        .filter(|v| !v.is_undefined())
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped().to_ascii_uppercase())
        .unwrap_or_else(|| "GET".to_string());

    let redirect = extract_string_opt(init, "redirect", "follow", ctx)?;
    let mode = extract_string_opt(init, "mode", "cors", ctx)?;
    let credentials = extract_string_opt(init, "credentials", "same-origin", ctx)?;

    // Pre-build headers before ObjectInitializer borrows ctx.
    let headers = build_headers_object(None, ctx)?;

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(js_string!("url"), JsValue::from(js_string!(url.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("method"), JsValue::from(js_string!(method.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("bodyUsed"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("headers"), headers, Attribute::READONLY);
    init_obj.property(js_string!("redirect"), JsValue::from(js_string!(redirect.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("mode"), JsValue::from(js_string!(mode.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("credentials"), JsValue::from(js_string!(credentials.as_str())), Attribute::READONLY);

    Ok(init_obj.build().into())
}

/// Extract an optional string from init object, with default.
fn extract_string_opt(
    init: Option<&JsValue>,
    key: &str,
    default: &str,
    ctx: &mut Context,
) -> boa_engine::JsResult<String> {
    init.and_then(JsValue::as_object)
        .map(|obj| obj.get(js_string!(key), ctx))
        .transpose()?
        .filter(|v| !v.is_undefined())
        .map(|v| v.to_string(ctx))
        .transpose()
        .map(|opt| {
            opt.map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|| default.to_string())
        })
}

/// Build a Response object from constructor.
#[allow(clippy::cast_possible_truncation)]
fn build_response_object(
    args: &[JsValue],
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let body = args.first().cloned().unwrap_or(JsValue::null());
    let init = args.get(1);

    let status = init
        .and_then(JsValue::as_object)
        .map(|obj| obj.get(js_string!("status"), ctx))
        .transpose()?
        .and_then(|v| v.as_number())
        .map_or(200_u16, |n| n as u16);

    let status_text = extract_string_opt(init, "statusText", "", ctx)?;

    let body_str = if body.is_null() || body.is_undefined() {
        String::new()
    } else {
        body.to_string(ctx)?.to_std_string_escaped()
    };

    // Pre-build headers before ObjectInitializer.
    let headers = build_headers_object(None, ctx)?;

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(js_string!("status"), JsValue::from(f64::from(status)), Attribute::READONLY);
    init_obj.property(js_string!("statusText"), JsValue::from(js_string!(status_text.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("ok"), JsValue::from((200..300).contains(&status)), Attribute::READONLY);
    init_obj.property(js_string!("type"), JsValue::from(js_string!("default")), Attribute::READONLY);
    init_obj.property(js_string!("url"), JsValue::from(js_string!("")), Attribute::READONLY);
    init_obj.property(js_string!("redirected"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("bodyUsed"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("headers"), headers, Attribute::READONLY);
    init_obj.property(
        js_string!("__body__"),
        JsValue::from(js_string!(body_str.as_str())),
        Attribute::empty(),
    );

    // text() → Promise<string>
    init_obj.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Response.text: not a Response")
            })?;
            let body = obj.get(js_string!("__body__"), ctx)?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(body, ctx);
            Ok(promise.into())
        }),
        js_string!("text"),
        0,
    );

    // json() → Promise<parsed>
    init_obj.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Response.json: not a Response")
            })?;
            let body_str = obj
                .get(js_string!("__body__"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            let json_global = ctx.global_object().get(js_string!("JSON"), ctx)?;
            let json_obj = json_global.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("JSON global not found")
            })?;
            let parse_fn = json_obj.get(js_string!("parse"), ctx)?;
            let parse_callable = parse_fn.as_callable().ok_or_else(|| {
                JsNativeError::typ().with_message("JSON.parse not callable")
            })?;
            let result = parse_callable.call(
                &json_global,
                &[JsValue::from(js_string!(body_str.as_str()))],
                ctx,
            )?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(result, ctx);
            Ok(promise.into())
        }),
        js_string!("json"),
        0,
    );

    Ok(init_obj.build().into())
}

/// Parse headers from the hidden `__headers__` property.
fn parse_headers(
    this: &JsValue,
    ctx: &mut Context,
) -> boa_engine::JsResult<Vec<(String, String)>> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("Headers: not a Headers object")
    })?;
    let encoded = obj
        .get(js_string!("__headers__"), ctx)?
        .to_string(ctx)?
        .to_std_string_escaped();
    if encoded.is_empty() {
        return Ok(Vec::new());
    }
    Ok(encoded
        .split('\n')
        .filter_map(|line| {
            let mut parts = line.splitn(2, '\0');
            let k = parts.next()?;
            let v = parts.next().unwrap_or("");
            Some((k.to_string(), v.to_string()))
        })
        .collect())
}

/// Build a `Response.error()` response (WHATWG Fetch §5.4).
///
/// Returns a Response with type "error", status 0, empty statusText, and no body.
fn build_error_response(ctx: &mut Context) -> boa_engine::JsResult<JsValue> {
    let headers = build_headers_object(None, ctx)?;

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(js_string!("status"), JsValue::from(0), Attribute::READONLY);
    init_obj.property(js_string!("statusText"), JsValue::from(js_string!("")), Attribute::READONLY);
    init_obj.property(js_string!("ok"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("type"), JsValue::from(js_string!("error")), Attribute::READONLY);
    init_obj.property(js_string!("url"), JsValue::from(js_string!("")), Attribute::READONLY);
    init_obj.property(js_string!("redirected"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("bodyUsed"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("headers"), headers, Attribute::READONLY);
    init_obj.property(
        js_string!("__body__"),
        JsValue::from(js_string!("")),
        Attribute::empty(),
    );

    // text() -> Promise<string>
    init_obj.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Response.text: not a Response")
            })?;
            let body = obj.get(js_string!("__body__"), ctx)?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(body, ctx);
            Ok(promise.into())
        }),
        js_string!("text"),
        0,
    );

    // json() -> Promise<parsed>
    init_obj.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Response.json: not a Response")
            })?;
            let body_str = obj
                .get(js_string!("__body__"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            let json_global = ctx.global_object().get(js_string!("JSON"), ctx)?;
            let json_obj = json_global.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("JSON global not found")
            })?;
            let parse_fn = json_obj.get(js_string!("parse"), ctx)?;
            let parse_callable = parse_fn.as_callable().ok_or_else(|| {
                JsNativeError::typ().with_message("JSON.parse not callable")
            })?;
            let result = parse_callable.call(
                &json_global,
                &[JsValue::from(js_string!(body_str.as_str()))],
                ctx,
            )?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(result, ctx);
            Ok(promise.into())
        }),
        js_string!("json"),
        0,
    );

    Ok(init_obj.build().into())
}

/// Build a `Response.redirect(url, status?)` response (WHATWG Fetch §5.4).
///
/// Returns a Response with the given redirect status (default 302) and a
/// `Location` header set to the provided URL. Valid redirect statuses are
/// 301, 302, 303, 307, and 308.
#[allow(clippy::cast_possible_truncation)]
fn build_redirect_response(
    args: &[JsValue],
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let url_str = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let status = args
        .get(1)
        .and_then(|v| v.as_number())
        .map_or(302_u16, |n| n as u16);

    // Validate redirect status.
    if !matches!(status, 301 | 302 | 303 | 307 | 308) {
        return Err(JsNativeError::range()
            .with_message(format!("Response.redirect: invalid status {status}"))
            .into());
    }

    // Build headers with Location set.
    let headers = build_headers_object(None, ctx)?;
    if let Some(h_obj) = headers.as_object() {
        let mut h = parse_headers(&headers, ctx)?;
        h.push(("Location".to_string(), url_str.clone()));
        store_headers(&headers, &h, ctx)?;
        let _ = h_obj; // keep borrow checker happy
    }

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(js_string!("status"), JsValue::from(f64::from(status)), Attribute::READONLY);
    init_obj.property(js_string!("statusText"), JsValue::from(js_string!("")), Attribute::READONLY);
    init_obj.property(js_string!("ok"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("type"), JsValue::from(js_string!("default")), Attribute::READONLY);
    init_obj.property(js_string!("url"), JsValue::from(js_string!("")), Attribute::READONLY);
    init_obj.property(js_string!("redirected"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("bodyUsed"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("headers"), headers, Attribute::READONLY);
    init_obj.property(
        js_string!("__body__"),
        JsValue::from(js_string!("")),
        Attribute::empty(),
    );

    // text() -> Promise<string>
    init_obj.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Response.text: not a Response")
            })?;
            let body = obj.get(js_string!("__body__"), ctx)?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(body, ctx);
            Ok(promise.into())
        }),
        js_string!("text"),
        0,
    );

    // json() -> Promise<parsed>
    init_obj.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Response.json: not a Response")
            })?;
            let body_str = obj
                .get(js_string!("__body__"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            let json_global = ctx.global_object().get(js_string!("JSON"), ctx)?;
            let json_obj = json_global.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("JSON global not found")
            })?;
            let parse_fn = json_obj.get(js_string!("parse"), ctx)?;
            let parse_callable = parse_fn.as_callable().ok_or_else(|| {
                JsNativeError::typ().with_message("JSON.parse not callable")
            })?;
            let result = parse_callable.call(
                &json_global,
                &[JsValue::from(js_string!(body_str.as_str()))],
                ctx,
            )?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(result, ctx);
            Ok(promise.into())
        }),
        js_string!("json"),
        0,
    );

    Ok(init_obj.build().into())
}

/// Store headers back into the hidden `__headers__` property.
fn store_headers(
    this: &JsValue,
    headers: &[(String, String)],
    ctx: &mut Context,
) -> boa_engine::JsResult<()> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("Headers: not a Headers object")
    })?;
    let encoded = headers
        .iter()
        .map(|(k, v)| format!("{k}\0{v}"))
        .collect::<Vec<_>>()
        .join("\n");
    obj.set(
        js_string!("__headers__"),
        JsValue::from(js_string!(encoded.as_str())),
        false,
        ctx,
    )?;
    Ok(())
}
