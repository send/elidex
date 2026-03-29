//! `URL` and `URLSearchParams` constructors (WHATWG URL §6.1, §6.2).

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key storing the serialized URL string on URL objects.
const URL_HIDDEN_KEY: &str = "__url__";
/// Hidden property key storing the encoded query string on URLSearchParams.
const PARAMS_HIDDEN_KEY: &str = "__params__";
/// Hidden property key linking URLSearchParams back to its parent URL object.
const URL_OBJ_KEY: &str = "__url_obj__";

/// Register `URL` and `URLSearchParams` global constructors.
pub fn register_url_constructors(ctx: &mut Context, _bridge: &HostBridge) {
    // URL constructor: new URL(url, base?)
    ctx.register_global_callable(
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
    ctx.register_global_callable(
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

/// Read the hidden URL string from a URL object, parse it, and return it.
fn read_url(this: &JsValue, ctx: &mut Context) -> JsResult<url::Url> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("URL: not a URL object")
    })?;
    let href = obj
        .get(js_string!(URL_HIDDEN_KEY), ctx)?
        .to_string(ctx)?
        .to_std_string_escaped();
    url::Url::parse(&href).map_err(|_| {
        JsNativeError::typ()
            .with_message(format!("URL: invalid stored URL: {href}"))
            .into()
    })
}

/// After mutating a URL, update all derived properties on the JS object.
fn sync_url_properties(obj: &boa_engine::JsObject, url: &url::Url, ctx: &mut Context) {
    let href = url.as_str();
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

    let _ = obj.set(js_string!(URL_HIDDEN_KEY), JsValue::from(js_string!(href)), false, ctx);
    let _ = obj.set(js_string!("href"), JsValue::from(js_string!(href)), false, ctx);
    let _ = obj.set(js_string!("origin"), JsValue::from(js_string!(origin.as_str())), false, ctx);
    let _ = obj.set(js_string!("protocol"), JsValue::from(js_string!(protocol.as_str())), false, ctx);
    let _ = obj.set(js_string!("host"), JsValue::from(js_string!(host.as_str())), false, ctx);
    let _ = obj.set(js_string!("hostname"), JsValue::from(js_string!(hostname.as_str())), false, ctx);
    let _ = obj.set(js_string!("port"), JsValue::from(js_string!(port.as_str())), false, ctx);
    let _ = obj.set(js_string!("pathname"), JsValue::from(js_string!(pathname.as_str())), false, ctx);
    let _ = obj.set(js_string!("search"), JsValue::from(js_string!(search.as_str())), false, ctx);
    let _ = obj.set(js_string!("hash"), JsValue::from(js_string!(hash.as_str())), false, ctx);
    let _ = obj.set(js_string!("username"), JsValue::from(js_string!(url.username())), false, ctx);
    let _ = obj.set(js_string!("password"), JsValue::from(js_string!(url.password().unwrap_or(""))), false, ctx);

    // Update the searchParams object's __params__ to reflect the new query.
    if let Ok(sp_val) = obj.get(js_string!("searchParams"), ctx) {
        if let Some(sp_obj) = sp_val.as_object() {
            let query = url.query().unwrap_or("");
            let encoded = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect::<Vec<_>>();
            let new_encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(&encoded)
                .finish();
            let _ = sp_obj.set(
                js_string!(PARAMS_HIDDEN_KEY),
                JsValue::from(js_string!(new_encoded.as_str())),
                false,
                ctx,
            );
        }
    }
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

    // Build searchParams with back-reference to URL object.
    let params: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    let search_params = build_search_params_object(&params, ctx);

    let mut init = ObjectInitializer::new(ctx);

    // Hidden property storing the full URL for setters to parse/modify.
    init.property(
        js_string!(URL_HIDDEN_KEY),
        JsValue::from(js_string!(href.as_str())),
        Attribute::empty(),
    );

    init.property(js_string!("href"), JsValue::from(js_string!(href.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("origin"), JsValue::from(js_string!(origin.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("protocol"), JsValue::from(js_string!(protocol.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("host"), JsValue::from(js_string!(host.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("hostname"), JsValue::from(js_string!(hostname.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("port"), JsValue::from(js_string!(port.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("pathname"), JsValue::from(js_string!(pathname.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("search"), JsValue::from(js_string!(search.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("hash"), JsValue::from(js_string!(hash.as_str())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("username"), JsValue::from(js_string!(url.username())), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("password"), JsValue::from(js_string!(url.password().unwrap_or(""))), Attribute::CONFIGURABLE | Attribute::WRITABLE);
    init.property(js_string!("searchParams"), search_params, Attribute::CONFIGURABLE);

    // toString() / toJSON() — return href.
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

    let url_obj = init.build();

    // Link searchParams back to the URL object for mutation sync.
    if let Ok(sp_val) = url_obj.get(js_string!("searchParams"), ctx) {
        if let Some(sp_obj) = sp_val.as_object() {
            let _ = sp_obj.set(
                js_string!(URL_OBJ_KEY),
                JsValue::from(url_obj.clone()),
                false,
                ctx,
            );
        }
    }

    // --- URL property setters ---
    // We set these as methods since boa ObjectInitializer's property() creates
    // data properties. The setter pattern: modify the hidden URL, then sync.

    // Helper: define a setter function on the URL object.
    macro_rules! url_setter {
        ($obj:expr, $name:expr, $setter_fn:expr, $ctx:expr) => {
            let _ = $obj.set(
                js_string!(concat!("__set_", $name)),
                NativeFunction::from_copy_closure($setter_fn).to_js_function($ctx.realm()),
                false,
                $ctx,
            );
        };
    }

    // href setter: full reparse.
    url_setter!(url_obj, "href", |this, args, ctx| {
        let new_href = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let parsed = url::Url::parse(&new_href).map_err(|_| {
            JsNativeError::typ().with_message(format!("URL: invalid URL: {new_href}"))
        })?;
        let obj = this.as_object().ok_or_else(|| {
            JsNativeError::typ().with_message("URL: not a URL object")
        })?;
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // protocol setter: strip trailing colon.
    url_setter!(url_obj, "protocol", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let scheme = val.strip_suffix(':').unwrap_or(&val);
        let mut parsed = read_url(this, ctx)?;
        if parsed.set_scheme(scheme).is_ok() {
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
        }
        Ok(JsValue::undefined())
    }, ctx);

    // host setter.
    url_setter!(url_obj, "host", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut parsed = read_url(this, ctx)?;
        let _ = parsed.set_host(Some(&val));
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // hostname setter.
    url_setter!(url_obj, "hostname", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut parsed = read_url(this, ctx)?;
        let _ = parsed.set_host(Some(&val));
        // Preserve port — set_host may clear it if the value has no port.
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // port setter: "" removes port.
    url_setter!(url_obj, "port", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut parsed = read_url(this, ctx)?;
        if val.is_empty() {
            let _ = parsed.set_port(None);
        } else if let Ok(p) = val.parse::<u16>() {
            let _ = parsed.set_port(Some(p));
        }
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // pathname setter.
    url_setter!(url_obj, "pathname", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut parsed = read_url(this, ctx)?;
        parsed.set_path(&val);
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // search setter: strip leading ?.
    url_setter!(url_obj, "search", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let query = val.strip_prefix('?').unwrap_or(&val);
        let mut parsed = read_url(this, ctx)?;
        if query.is_empty() {
            parsed.set_query(None);
        } else {
            parsed.set_query(Some(query));
        }
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // hash setter: strip leading #.
    url_setter!(url_obj, "hash", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let frag = val.strip_prefix('#').unwrap_or(&val);
        let mut parsed = read_url(this, ctx)?;
        if frag.is_empty() {
            parsed.set_fragment(None);
        } else {
            parsed.set_fragment(Some(frag));
        }
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // username setter.
    url_setter!(url_obj, "username", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut parsed = read_url(this, ctx)?;
        let _ = parsed.set_username(&val);
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    // password setter.
    url_setter!(url_obj, "password", |this, args, ctx| {
        let val = args.first().map(|v| v.to_string(ctx)).transpose()?
            .map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let mut parsed = read_url(this, ctx)?;
        let _ = parsed.set_password(if val.is_empty() { None } else { Some(&val) });
        let obj = this.as_object().unwrap();
        sync_url_properties(&obj, &parsed, ctx);
        Ok(JsValue::undefined())
    }, ctx);

    url_obj.into()
}

/// Build a JS URLSearchParams object from a list of (key, value) pairs.
fn build_search_params_object(params: &[(String, String)], ctx: &mut Context) -> JsValue {
    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();

    let mut init = ObjectInitializer::new(ctx);

    // Store the encoded form for methods to use.
    init.property(
        js_string!(PARAMS_HIDDEN_KEY),
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

    // getAll(name) -> array
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let params = get_params(this, ctx)?;
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let arr = JsArray::new(ctx);
            for (k, v) in &params {
                if k == &name {
                    let _ = arr.push(JsValue::from(js_string!(v.as_str())), ctx);
                }
            }
            Ok(arr.into())
        }),
        js_string!("getAll"),
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

    // set(name, value) — replaces all existing entries with that name.
    init.function(
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
            let mut params = get_params(this, ctx)?;
            params.retain(|(k, _)| k != &name);
            params.push((name, value));
            set_params(this, &params, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("set"),
        2,
    );

    // append(name, value) — adds a new entry.
    init.function(
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
            let mut params = get_params(this, ctx)?;
            params.push((name, value));
            set_params(this, &params, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("append"),
        2,
    );

    // delete(name) — removes all entries with that name.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let name = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut params = get_params(this, ctx)?;
            params.retain(|(k, _)| k != &name);
            set_params(this, &params, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("delete"),
        1,
    );

    // sort() — sorts all entries by name, stable.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let mut params = get_params(this, ctx)?;
            params.sort_by(|(a, _), (b, _)| a.cmp(b));
            set_params(this, &params, ctx)?;
            Ok(JsValue::undefined())
        }),
        js_string!("sort"),
        0,
    );

    // entries() -> array of [key, value] pairs.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let params = get_params(this, ctx)?;
            let arr = JsArray::new(ctx);
            for (k, v) in &params {
                let pair = JsArray::new(ctx);
                let _ = pair.push(JsValue::from(js_string!(k.as_str())), ctx);
                let _ = pair.push(JsValue::from(js_string!(v.as_str())), ctx);
                let _ = arr.push(JsValue::from(pair), ctx);
            }
            Ok(arr.into())
        }),
        js_string!("entries"),
        0,
    );

    // keys() -> array of key strings.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let params = get_params(this, ctx)?;
            let arr = JsArray::new(ctx);
            for (k, _) in &params {
                let _ = arr.push(JsValue::from(js_string!(k.as_str())), ctx);
            }
            Ok(arr.into())
        }),
        js_string!("keys"),
        0,
    );

    // values() -> array of value strings.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let params = get_params(this, ctx)?;
            let arr = JsArray::new(ctx);
            for (_, v) in &params {
                let _ = arr.push(JsValue::from(js_string!(v.as_str())), ctx);
            }
            Ok(arr.into())
        }),
        js_string!("values"),
        0,
    );

    // toString() — without ? prefix (WHATWG URL §6.2).
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("URLSearchParams: not an object")
            })?;
            obj.get(js_string!(PARAMS_HIDDEN_KEY), ctx)
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

/// Get params from the hidden `__params__` property.
fn get_params(this: &JsValue, ctx: &mut Context) -> JsResult<Vec<(String, String)>> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("URLSearchParams: not an object")
    })?;
    let encoded = obj
        .get(js_string!(PARAMS_HIDDEN_KEY), ctx)?
        .to_string(ctx)?
        .to_std_string_escaped();
    Ok(url::form_urlencoded::parse(encoded.as_bytes())
        .into_owned()
        .collect())
}

/// Set params back into the hidden `__params__` property.
///
/// If the URLSearchParams is linked to a parent URL object (via `__url_obj__`),
/// also update the parent URL's query string and sync all its properties.
fn set_params(this: &JsValue, params: &[(String, String)], ctx: &mut Context) -> JsResult<()> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("URLSearchParams: not an object")
    })?;
    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();
    obj.set(
        js_string!(PARAMS_HIDDEN_KEY),
        JsValue::from(js_string!(encoded.as_str())),
        false,
        ctx,
    )?;

    // Sync back to parent URL object if linked.
    let url_obj_val = obj.get(js_string!(URL_OBJ_KEY), ctx)?;
    if let Some(url_obj) = url_obj_val.as_object() {
        let href = url_obj
            .get(js_string!(URL_HIDDEN_KEY), ctx)?
            .to_string(ctx)?
            .to_std_string_escaped();
        if let Ok(mut parsed) = url::Url::parse(&href) {
            if encoded.is_empty() {
                parsed.set_query(None);
            } else {
                parsed.set_query(Some(&encoded));
            }
            sync_url_properties(&url_obj, &parsed, ctx);
        }
    }
    Ok(())
}
