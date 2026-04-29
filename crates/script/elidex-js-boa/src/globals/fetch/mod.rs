//! `fetch()` global, Response and Headers objects for boa.
//!
//! # Phase 2 limitations
//!
//! - `fetch()` blocks the UI thread via `FetchHandle::send_blocking()`.
//!   Returns an already-resolved/rejected `JsPromise`.
//! - `Headers.set()` and `Headers.delete()` are intentionally omitted:
//!   Response headers are immutable per the Fetch spec. A mutable `Headers`
//!   constructor will be added in Phase 3 alongside the `Request` object.

pub(crate) mod constructors;

use std::rc::Rc;

use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use elidex_net::broker::NetworkHandle;

/// Hidden property key storing the response body bytes on a Response object.
const BODY_KEY: &str = "__elidex_fetch_body__";
/// Hidden property key storing the response headers on a Response object.
const HEADERS_KEY: &str = "__elidex_fetch_headers__";

/// Captures type for `fetch()` closure.
///
/// Contains an `Rc<NetworkHandle>` for sending fetch requests to the
/// Network Process broker. `Rc` is `!Send` but boa is also `!Send`.
#[derive(Clone)]
struct FetchCaptures {
    handle: Rc<NetworkHandle>,
}

// Trace/Finalize: NetworkHandle contains only Rust types, no GC objects.
impl_empty_trace!(FetchCaptures);

/// Register the `fetch()` global function on the boa context.
///
/// If `network_handle` is `None`, `fetch()` is not registered (test mode).
pub fn register_fetch(ctx: &mut Context, network_handle: Option<Rc<NetworkHandle>>) {
    let Some(handle) = network_handle else {
        return;
    };

    let captures = FetchCaptures { handle };

    ctx.register_global_builtin_callable(
        js_string!("fetch"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, captures, ctx| -> JsResult<JsValue> { fetch_impl(args, captures, ctx) },
            captures,
        ),
    )
    .expect("failed to register fetch");
}

/// Core `fetch()` implementation.
fn fetch_impl(args: &[JsValue], captures: &FetchCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    // 1. Parse URL argument.
    let url_str = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| JsNativeError::typ().with_message("fetch: URL argument is required"))?;

    let url = url::Url::parse(&url_str)
        .map_err(|e| JsNativeError::typ().with_message(format!("fetch: invalid URL: {e}")))?;

    // 2. Parse options (method, headers, body, signal).
    let mut method = "GET".to_string();
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body = bytes::Bytes::new();

    if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
        // signal — check for AbortSignal.
        let signal_val = opts.get(js_string!("signal"), ctx)?;
        if crate::globals::abort::is_abort_signal(&signal_val, ctx) {
            let signal_obj = signal_val.as_object().expect("is_abort_signal verified");
            if crate::globals::abort::is_signal_aborted(&signal_obj, ctx) {
                // Signal already aborted: reject immediately with AbortError.
                let reason = signal_obj
                    .get(js_string!("reason"), ctx)
                    .unwrap_or_else(|_| {
                        JsValue::from(js_string!("AbortError: The operation was aborted"))
                    });
                let promise = JsPromise::reject(
                    JsNativeError::typ().with_message(
                        reason
                            .to_string(ctx)
                            .map_or("The operation was aborted".into(), |s| {
                                s.to_std_string_escaped()
                            }),
                    ),
                    ctx,
                );
                return Ok(promise.into());
            }
            // Signal not yet aborted: proceed with fetch. The blocking fetch
            // cannot be cancelled mid-flight in Phase 2 (single-threaded),
            // but the pre-check covers the common case.
        }

        // method
        let m = opts.get(js_string!("method"), ctx)?;
        if !m.is_undefined() {
            method = m
                .to_string(ctx)?
                .to_std_string_escaped()
                .to_ascii_uppercase();
        }

        // headers (plain object)
        let h = opts.get(js_string!("headers"), ctx)?;
        if let Some(h_obj) = h.as_object() {
            let keys = h_obj.own_property_keys(ctx)?;
            for key in keys {
                let val = h_obj.get(key.clone(), ctx)?;
                let key_str = match &key {
                    boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                    _ => continue, // skip symbol keys
                };
                let val_str = val.to_string(ctx)?.to_std_string_escaped();
                headers.push((key_str, val_str));
            }
        }

        // body
        let b = opts.get(js_string!("body"), ctx)?;
        if !b.is_undefined() && !b.is_null() {
            let body_str = b.to_string(ctx)?.to_std_string_escaped();
            body = bytes::Bytes::from(body_str);
        }
    }

    let request_url = url.clone();
    let request = elidex_net::Request {
        method,
        url,
        headers,
        body,
        ..Default::default()
    };

    // 3. Execute the request via the Network Process broker (blocking).
    match captures.handle.fetch_blocking(request) {
        Ok(response) => {
            let response_obj = create_response_object(&response, &request_url, ctx);
            let promise = JsPromise::resolve(response_obj, ctx);
            Ok(promise.into())
        }
        Err(err) => {
            let promise = JsPromise::reject(
                JsNativeError::typ().with_message(format!("fetch failed: {err}")),
                ctx,
            );
            Ok(promise.into())
        }
    }
}

// ---------------------------------------------------------------------------
// Response method helpers (shared between create_response_object and clone)
// ---------------------------------------------------------------------------

/// Create a `NativeFunction` for the `text()` method on a Response.
fn create_text_fn() -> NativeFunction {
    NativeFunction::from_copy_closure(|this, _args, ctx| -> JsResult<JsValue> {
        let obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("text: not a Response"))?;
        let body = obj.get(js_string!(BODY_KEY), ctx)?;
        let promise = JsPromise::resolve(body, ctx);
        Ok(promise.into())
    })
}

/// Create a `NativeFunction` for the `json()` method on a Response.
///
/// Uses boa's built-in `JSON.parse()` via the global object rather than
/// eval-based string interpolation, avoiding injection risks.
fn create_json_fn() -> NativeFunction {
    NativeFunction::from_copy_closure(|this, _args, ctx| -> JsResult<JsValue> {
        let obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("json: not a Response"))?;
        let body_val = obj.get(js_string!(BODY_KEY), ctx)?;

        // Get JSON.parse from the global object.
        let global = ctx.global_object();
        let json_obj = global.get(js_string!("JSON"), ctx)?;
        let parse_fn = json_obj
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("JSON global is not an object"))?
            .get(js_string!("parse"), ctx)?;
        let parse_callable = parse_fn
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("JSON.parse is not callable"))?;

        match parse_callable.call(&JsValue::undefined(), &[body_val], ctx) {
            Ok(parsed) => {
                let promise = JsPromise::resolve(parsed, ctx);
                Ok(promise.into())
            }
            Err(err) => {
                let promise = JsPromise::reject(
                    JsNativeError::syntax().with_message(format!("json: invalid JSON: {err}")),
                    ctx,
                );
                Ok(promise.into())
            }
        }
    })
}

/// Create a `NativeFunction` for the `clone()` method on a Response.
///
/// The cloned Response has all the same properties and methods (including
/// `clone()` itself) as the original.
fn create_clone_fn() -> NativeFunction {
    NativeFunction::from_copy_closure(|this, _args, ctx| -> JsResult<JsValue> {
        let obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("clone: not a Response"))?;

        let ok = obj.get(js_string!("ok"), ctx)?;
        let status = obj.get(js_string!("status"), ctx)?;
        let status_text = obj.get(js_string!("statusText"), ctx)?;
        let url = obj.get(js_string!("url"), ctx)?;
        let type_val = obj.get(js_string!("type"), ctx)?;
        let redirected = obj.get(js_string!("redirected"), ctx)?;
        let headers = obj.get(js_string!(HEADERS_KEY), ctx)?;
        let body = obj.get(js_string!(BODY_KEY), ctx)?;

        let mut clone_init = ObjectInitializer::new(ctx);
        clone_init
            .property(js_string!("ok"), ok, Attribute::READONLY)
            .property(js_string!("status"), status, Attribute::READONLY)
            .property(js_string!("statusText"), status_text, Attribute::READONLY)
            .property(js_string!("url"), url, Attribute::READONLY)
            .property(js_string!("type"), type_val, Attribute::READONLY)
            .property(js_string!("redirected"), redirected, Attribute::READONLY)
            .property(js_string!("headers"), headers.clone(), Attribute::READONLY)
            .property(js_string!(HEADERS_KEY), headers, Attribute::empty())
            .property(js_string!(BODY_KEY), body, Attribute::empty());

        register_response_methods(&mut clone_init);

        Ok(clone_init.build().into())
    })
}

/// Register `text()`, `json()`, and `clone()` methods on an `ObjectInitializer`.
fn register_response_methods(init: &mut ObjectInitializer<'_>) {
    init.function(create_text_fn(), js_string!("text"), 0)
        .function(create_json_fn(), js_string!("json"), 0)
        .function(create_clone_fn(), js_string!("clone"), 0);
}

// ---------------------------------------------------------------------------
// Response / Headers object construction
// ---------------------------------------------------------------------------

/// Input for building a JS Response object.
pub(crate) struct ResponseParts<'a> {
    pub status: u16,
    pub status_text: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a str,
    pub url: &'a str,
    pub response_type: &'a str,
    pub redirected: bool,
}

/// Build a JS Response object from constituent parts.
///
/// Shared by `create_response_object` (fetch) and `cached_entry_to_response` (cache).
pub(crate) fn build_response_from_parts(parts: &ResponseParts<'_>, ctx: &mut Context) -> JsValue {
    let status = parts.status;
    let ok = (200..300).contains(&status);
    let headers_obj = create_headers_object(parts.headers, ctx);

    let mut init = ObjectInitializer::new(ctx);
    init.property(js_string!("ok"), JsValue::from(ok), Attribute::READONLY)
        .property(
            js_string!("status"),
            JsValue::from(f64::from(status)),
            Attribute::READONLY,
        )
        .property(
            js_string!("statusText"),
            js_string!(parts.status_text),
            Attribute::READONLY,
        )
        .property(
            js_string!("url"),
            js_string!(parts.url),
            Attribute::READONLY,
        )
        .property(
            js_string!("type"),
            js_string!(parts.response_type),
            Attribute::READONLY,
        )
        .property(
            js_string!("redirected"),
            JsValue::from(parts.redirected),
            Attribute::READONLY,
        )
        .property(
            js_string!("headers"),
            headers_obj.clone(),
            Attribute::READONLY,
        );

    init.property(
        js_string!(BODY_KEY),
        js_string!(parts.body),
        Attribute::empty(),
    );
    init.property(js_string!(HEADERS_KEY), headers_obj, Attribute::empty());

    register_response_methods(&mut init);
    init.build().into()
}

/// Create a JS Response object from an `elidex_net::Response`.
fn create_response_object(
    response: &elidex_net::Response,
    request_url: &url::Url,
    ctx: &mut Context,
) -> JsValue {
    let body_string = String::from_utf8_lossy(&response.body);
    let url_string = response.url.to_string();
    let redirected = response.url.as_str() != request_url.as_str();
    build_response_from_parts(
        &ResponseParts {
            status: response.status,
            status_text: status_text_for(response.status),
            headers: &response.headers,
            body: &body_string,
            url: &url_string,
            response_type: "basic",
            redirected,
        },
        ctx,
    )
}

/// Create a JS Headers object from header pairs.
///
/// `get()` combines all values for a given name with `", "` per the Fetch spec.
pub(crate) fn create_headers_object(headers: &[(String, String)], ctx: &mut Context) -> JsValue {
    let mut init = ObjectInitializer::new(ctx);

    let header_map: Vec<(String, String)> = headers.to_vec();

    // get(name) — combines duplicate header values per Fetch spec.
    let hm = header_map.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, headers, ctx| -> JsResult<JsValue> {
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
            },
            hm,
        ),
        js_string!("get"),
        1,
    );

    // has(name)
    let hm = header_map.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, headers, ctx| -> JsResult<JsValue> {
                let name = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped().to_ascii_lowercase())
                    .unwrap_or_default();
                let found = headers.iter().any(|(k, _)| k.to_ascii_lowercase() == name);
                Ok(JsValue::from(found))
            },
            hm,
        ),
        js_string!("has"),
        1,
    );

    // forEach(callback)
    let hm = header_map;
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, headers, ctx| -> JsResult<JsValue> {
                let callback = args
                    .first()
                    .and_then(JsValue::as_callable)
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("forEach: callback is not a function")
                    })?
                    .clone();
                for (name, value) in headers {
                    callback.call(
                        &JsValue::undefined(),
                        &[
                            JsValue::from(js_string!(value.as_str())),
                            JsValue::from(js_string!(name.as_str())),
                        ],
                        ctx,
                    )?;
                }
                Ok(JsValue::undefined())
            },
            hm,
        ),
        js_string!("forEach"),
        1,
    );

    init.build().into()
}

/// Map an HTTP status code to its standard reason phrase.
///
/// Delegates to [`elidex_api_fetch::status_text_for`].
fn status_text_for(status: u16) -> &'static str {
    elidex_api_fetch::status_text_for(status)
}

#[cfg(test)]
mod tests;
