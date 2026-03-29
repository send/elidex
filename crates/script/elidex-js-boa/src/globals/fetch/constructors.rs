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
            build_headers_object(args.first(), "none", ctx)
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

/// Forbidden request header names (Fetch spec §2.2.1).
const FORBIDDEN_REQUEST_HEADERS: &[&str] = &[
    "accept-charset",
    "accept-encoding",
    "access-control-request-headers",
    "access-control-request-method",
    "connection",
    "content-length",
    "cookie",
    "cookie2",
    "date",
    "dnt",
    "expect",
    "host",
    "keep-alive",
    "origin",
    "referer",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "via",
];

/// Hidden property key for the headers guard.
const GUARD_KEY: &str = "__guard__";

/// Check a header guard before mutation. Returns Err if disallowed.
fn check_headers_guard(
    this: &JsValue,
    name: &str,
    ctx: &mut Context,
) -> boa_engine::JsResult<()> {
    let guard = this
        .as_object()
        .map(|obj| obj.get(js_string!(GUARD_KEY), ctx))
        .transpose()?
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    match guard.as_str() {
        "immutable" => Err(JsNativeError::typ()
            .with_message("Headers: cannot modify immutable headers")
            .into()),
        "request" => {
            let lower = name.to_ascii_lowercase();
            if FORBIDDEN_REQUEST_HEADERS.contains(&lower.as_str())
                || lower.starts_with("proxy-")
                || lower.starts_with("sec-")
            {
                Err(JsNativeError::typ()
                    .with_message(format!("Headers: '{name}' is a forbidden header name"))
                    .into())
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

/// Build a mutable Headers object.
///
/// `guard` determines mutation rules: "none" (default), "request" (rejects
/// forbidden header names), or "immutable" (rejects all mutations).
fn build_headers_object(
    init: Option<&JsValue>,
    guard: &str,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let encoded = String::new();

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(
        js_string!("__headers__"),
        JsValue::from(js_string!(encoded.as_str())),
        Attribute::empty(),
    );

    init_obj.property(
        js_string!(GUARD_KEY),
        JsValue::from(js_string!(guard)),
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
            check_headers_guard(this, &name, ctx)?;
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
            check_headers_guard(this, &name, ctx)?;
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
            check_headers_guard(this, &name, ctx)?;
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

    let headers_obj = init_obj.build();

    // Parse init argument (Gap 11: Headers constructor parses init).
    if let Some(init_val) = init {
        if !init_val.is_undefined() && !init_val.is_null() {
            apply_headers_init(&headers_obj, init_val, ctx)?;
        }
    }

    Ok(headers_obj.into())
}

/// Apply headers init data to an existing Headers object.
///
/// Handles three init forms (WHATWG Fetch §5.1):
/// 1. Headers object (has `__headers__` hidden property) — copy entries
/// 2. Array of [name, value] pairs (sequence<sequence<ByteString>>)
/// 3. Object (record<ByteString, ByteString>) — iterate own properties
fn apply_headers_init(
    headers_obj: &boa_engine::JsObject,
    init_val: &JsValue,
    ctx: &mut Context,
) -> boa_engine::JsResult<()> {
    let Some(init_obj) = init_val.as_object() else {
        return Ok(());
    };

    // Case 1: another Headers object (has __headers__ property).
    let h_val = init_obj.get(js_string!("__headers__"), ctx)?;
    if !h_val.is_undefined() {
        let encoded = h_val.to_string(ctx)?.to_std_string_escaped();
        if !encoded.is_empty() {
            // Merge: parse and set each entry.
            let existing = headers_obj
                .get(js_string!("__headers__"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            let merged = if existing.is_empty() {
                encoded
            } else {
                format!("{existing}\n{encoded}")
            };
            headers_obj.set(
                js_string!("__headers__"),
                JsValue::from(js_string!(merged.as_str())),
                false,
                ctx,
            )?;
        }
        return Ok(());
    }

    // Case 2: array (sequence of sequences).
    let len_val = init_obj.get(js_string!("length"), ctx)?;
    if let Some(len) = len_val.as_number() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let len = len as u32;
        let mut entries: Vec<(String, String)> = Vec::new();
        for i in 0..len {
            let pair = init_obj.get(i, ctx)?;
            if let Some(pair_obj) = pair.as_object() {
                let k = pair_obj
                    .get(0_u32, ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                let v = pair_obj
                    .get(1_u32, ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                entries.push((k, v));
            }
        }
        if !entries.is_empty() {
            store_headers(&JsValue::from(headers_obj.clone()), &entries, ctx)?;
        }
        return Ok(());
    }

    // Case 3: plain object (record).
    let keys = init_obj.own_property_keys(ctx)?;
    let mut entries: Vec<(String, String)> = Vec::new();
    for key in keys {
        let k = key.to_string();
        // Skip hidden/internal properties.
        if k.starts_with("__") {
            continue;
        }
        let v = init_obj
            .get(js_string!(&*k), ctx)?
            .to_string(ctx)?
            .to_std_string_escaped();
        entries.push((k.to_string(), v));
    }
    if !entries.is_empty() {
        store_headers(&JsValue::from(headers_obj.clone()), &entries, ctx)?;
    }
    Ok(())
}

/// Build a Request object.
///
/// WHATWG Fetch §5.5: If `input` is a Request object, clone its URL/method/headers.
/// Then apply any `init` overrides on top.
fn build_request_object(
    args: &[JsValue],
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let input = args.first();
    let init = args.get(1);

    // Check if input is a Request object (has both url and method properties).
    let is_request_input = input
        .and_then(JsValue::as_object)
        .is_some_and(|obj| {
            obj.has_own_property(js_string!("method"), ctx)
                .unwrap_or(false)
        });

    let (url, mut method, base_headers_encoded) = if is_request_input {
        let input_obj = input.unwrap().as_object().unwrap();

        // WHATWG Fetch §5.5 step 13: if body is used, throw TypeError.
        let body_used = input_obj
            .get(js_string!("bodyUsed"), ctx)?
            .to_boolean();
        if body_used {
            return Err(JsNativeError::typ()
                .with_message("Request: body has already been consumed")
                .into());
        }

        let u = input_obj
            .get(js_string!("url"), ctx)?
            .to_string(ctx)?
            .to_std_string_escaped();
        let m = input_obj
            .get(js_string!("method"), ctx)?
            .to_string(ctx)?
            .to_std_string_escaped();

        // Clone the serialized headers from the input Request.
        let h_encoded: String = input_obj
            .get(js_string!("headers"), ctx)
            .ok()
            .and_then(|v| {
                let h_obj = v.as_object()?;
                h_obj
                    .get(js_string!("__headers__"), ctx)
                    .ok()
                    .and_then(|hv| {
                        hv.to_string(ctx)
                            .ok()
                            .map(|s| s.to_std_string_escaped())
                    })
            })
            .unwrap_or_default();

        (u, m, h_encoded)
    } else {
        let u = input
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        (u, "GET".to_string(), String::new())
    };

    // Apply init overrides.
    if let Some(init_obj) = init.and_then(JsValue::as_object) {
        let m = init_obj.get(js_string!("method"), ctx)?;
        if !m.is_undefined() {
            method = m.to_string(ctx)?.to_std_string_escaped().to_ascii_uppercase();
        }
    }

    let redirect = extract_string_opt(init, "redirect", "follow", ctx)?;
    let mode = extract_string_opt(init, "mode", "cors", ctx)?;
    let credentials = extract_string_opt(init, "credentials", "same-origin", ctx)?;

    // Build headers with "request" guard.
    let headers = build_headers_object(None, "request", ctx)?;
    // Populate from base (cloned from input Request).
    if let Some(h_obj) = headers.as_object() {
        if !base_headers_encoded.is_empty() {
            let _ = h_obj.set(
                js_string!("__headers__"),
                JsValue::from(js_string!(base_headers_encoded.as_str())),
                false,
                ctx,
            );
        }
    }
    // Apply init.headers overrides if provided.
    if let Some(init_headers_val) = init
        .and_then(JsValue::as_object)
        .map(|obj| obj.get(js_string!("headers"), ctx))
        .transpose()?
        .filter(|v| !v.is_undefined() && !v.is_null())
    {
        if let Some(h_obj) = headers.as_object() {
            apply_headers_init(&h_obj, &init_headers_val, ctx)?;
        }
    }

    // Extract signal from init (WHATWG Fetch §5.5).
    let signal = init
        .and_then(JsValue::as_object)
        .map(|obj| obj.get(js_string!("signal"), ctx))
        .transpose()?
        .filter(|v| !v.is_undefined())
        .unwrap_or(JsValue::null());

    let mut init_obj = ObjectInitializer::new(ctx);

    init_obj.property(js_string!("url"), JsValue::from(js_string!(url.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("method"), JsValue::from(js_string!(method.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("bodyUsed"), JsValue::from(false), Attribute::READONLY);
    init_obj.property(js_string!("headers"), headers, Attribute::READONLY);
    init_obj.property(js_string!("redirect"), JsValue::from(js_string!(redirect.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("mode"), JsValue::from(js_string!(mode.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("credentials"), JsValue::from(js_string!(credentials.as_str())), Attribute::READONLY);
    init_obj.property(js_string!("signal"), signal, Attribute::READONLY);

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

/// Extract the body string from a Response constructor body argument.
///
/// WHATWG Fetch §5.4: Handles null/undefined, URLSearchParams, Blob, and generic objects.
fn extract_body_string(body: &JsValue, ctx: &mut Context) -> boa_engine::JsResult<String> {
    if body.is_null() || body.is_undefined() {
        return Ok(String::new());
    }
    if let Some(body_obj) = body.as_object() {
        // Check for URLSearchParams (has __params__ hidden property).
        let params_val = body_obj.get(js_string!("__params__"), ctx)?;
        if !params_val.is_undefined() {
            return Ok(params_val.to_string(ctx)?.to_std_string_escaped());
        }
        // Check for Blob-like (has __blob_data__ hidden property).
        let blob_data = body_obj.get(js_string!("__blob_data__"), ctx)?;
        if !blob_data.is_undefined() {
            return Ok(blob_data.to_string(ctx)?.to_std_string_escaped());
        }
    }
    Ok(body.to_string(ctx)?.to_std_string_escaped())
}

/// Add common Response properties to an ObjectInitializer.
///
/// Sets status, statusText, ok, type, url, redirected, bodyUsed, headers, and __body__.
fn add_response_properties(
    init: &mut ObjectInitializer<'_>,
    status: u16,
    status_text: &str,
    response_type: &str,
    url_str: &str,
    redirected: bool,
    headers: JsValue,
    body_str: &str,
) {
    init.property(js_string!("status"), JsValue::from(f64::from(status)), Attribute::READONLY);
    init.property(js_string!("statusText"), JsValue::from(js_string!(status_text)), Attribute::READONLY);
    init.property(js_string!("ok"), JsValue::from((200..300).contains(&status)), Attribute::READONLY);
    init.property(js_string!("type"), JsValue::from(js_string!(response_type)), Attribute::READONLY);
    init.property(js_string!("url"), JsValue::from(js_string!(url_str)), Attribute::READONLY);
    init.property(js_string!("redirected"), JsValue::from(redirected), Attribute::READONLY);
    init.property(js_string!("bodyUsed"), JsValue::from(false), Attribute::READONLY);
    init.property(js_string!("headers"), headers, Attribute::READONLY);
    init.property(
        js_string!("__body__"),
        JsValue::from(js_string!(body_str)),
        Attribute::empty(),
    );
}

/// Add text(), json(), and clone() methods to a constructor-built Response.
fn add_response_methods(init: &mut ObjectInitializer<'_>) {
    // text() → Promise<string>
    init.function(
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
    init.function(
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
    let body_str = extract_body_string(&body, ctx)?;
    let headers = build_headers_object(None, "immutable", ctx)?;

    let mut init_obj = ObjectInitializer::new(ctx);
    add_response_properties(
        &mut init_obj, status, &status_text, "default", "", false, headers, &body_str,
    );
    add_response_methods(&mut init_obj);

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
    let headers = build_headers_object(None, "immutable", ctx)?;

    let mut init_obj = ObjectInitializer::new(ctx);
    add_response_properties(&mut init_obj, 0, "", "error", "", false, headers, "");
    add_response_methods(&mut init_obj);

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
    let headers = build_headers_object(None, "immutable", ctx)?;
    if let Some(h_obj) = headers.as_object() {
        let mut h = parse_headers(&headers, ctx)?;
        h.push(("Location".to_string(), url_str.clone()));
        store_headers(&headers, &h, ctx)?;
        let _ = h_obj; // keep borrow checker happy
    }

    let mut init_obj = ObjectInitializer::new(ctx);
    add_response_properties(&mut init_obj, status, "", "default", "", false, headers, "");
    add_response_methods(&mut init_obj);

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
