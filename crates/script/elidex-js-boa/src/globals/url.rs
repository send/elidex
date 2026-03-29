//! `URL` and `URLSearchParams` constructors (WHATWG URL §6.1, §6.2).

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::{Attribute, PropertyDescriptorBuilder};
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key storing the serialized URL string on URL objects.
const URL_HIDDEN_KEY: &str = "__url__";
/// Hidden property key storing the encoded query string on `URLSearchParams`.
const PARAMS_HIDDEN_KEY: &str = "__params__";
/// Hidden property key linking `URLSearchParams` back to its parent URL object.
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
                    JsNativeError::typ().with_message(format!("URL: invalid base URL: {base_str}"))
                })?;
                base_url.join(&url_str).map_err(|_| {
                    JsNativeError::typ().with_message(format!("URL: invalid URL: {url_str}"))
                })?
            } else {
                url::Url::parse(&url_str).map_err(|_| {
                    JsNativeError::typ().with_message(format!("URL: invalid URL: {url_str}"))
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
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("URL: not a URL object"))?;
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

/// After mutating a URL, update the hidden `__url__` property and sync `searchParams`.
///
/// All visible properties (href, origin, protocol, etc.) are accessor descriptors
/// that derive their values from `__url__` on each access, so only the hidden
/// property and searchParams need updating here.
fn sync_url_properties(obj: &boa_engine::JsObject, url: &url::Url, ctx: &mut Context) {
    let href = url.as_str();
    let _ = obj.set(
        js_string!(URL_HIDDEN_KEY),
        JsValue::from(js_string!(href)),
        false,
        ctx,
    );

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
///
/// Creates an object with hidden `__url__`, searchParams, accessor properties
/// for all URL components (getter reads from `__url__`, setter parses/syncs),
/// and `toString()`/`toJSON()` methods.
#[allow(clippy::too_many_lines)]
fn build_url_object(url: &url::Url, ctx: &mut Context) -> JsValue {
    // Build searchParams with back-reference to URL object.
    let params: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    let search_params = build_search_params_object(&params, ctx);

    let mut init = ObjectInitializer::new(ctx);

    // Hidden property storing the full URL for accessors to parse/modify.
    init.property(
        js_string!(URL_HIDDEN_KEY),
        JsValue::from(js_string!("")),
        Attribute::WRITABLE,
    );

    init.property(
        js_string!("searchParams"),
        search_params,
        Attribute::CONFIGURABLE,
    );

    // toString() / toJSON() — return href.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("URL: not a URL object"))?;
            let parsed = read_url(&JsValue::from(obj), ctx)?;
            Ok(JsValue::from(js_string!(parsed.as_str())))
        }),
        js_string!("toString"),
        0,
    );
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("URL: not a URL object"))?;
            let parsed = read_url(&JsValue::from(obj), ctx)?;
            Ok(JsValue::from(js_string!(parsed.as_str())))
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

    // Populate the hidden URL from the parsed value.
    sync_url_properties(&url_obj, url, ctx);

    // --- Define accessor properties (getter/setter) ---
    // Each getter reads from __url__ and derives the property.
    // Each setter parses the URL, applies the change, and calls sync_url_properties.
    let realm = ctx.realm().clone();

    // Helper macro: define a URL accessor with getter and setter.
    macro_rules! url_accessor {
        ($name:expr, $getter_fn:expr, $setter_fn:expr) => {
            let getter = NativeFunction::from_copy_closure($getter_fn).to_js_function(&realm);
            let setter = NativeFunction::from_copy_closure($setter_fn).to_js_function(&realm);
            let desc = PropertyDescriptorBuilder::new()
                .get(getter)
                .set(setter)
                .configurable(true)
                .enumerable(true)
                .build();
            let _ = url_obj.define_property_or_throw(js_string!($name), desc, ctx);
        };
    }

    // Helper macro: define a read-only URL accessor (getter only).
    macro_rules! url_getter_only {
        ($name:expr, $getter_fn:expr) => {
            let getter = NativeFunction::from_copy_closure($getter_fn).to_js_function(&realm);
            let desc = PropertyDescriptorBuilder::new()
                .get(getter)
                .configurable(true)
                .enumerable(true)
                .build();
            let _ = url_obj.define_property_or_throw(js_string!($name), desc, ctx);
        };
    }

    // href: full reparse on set.
    url_accessor!(
        "href",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            Ok(JsValue::from(js_string!(parsed.as_str())))
        },
        |this, args, ctx| {
            let new_href = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let parsed = url::Url::parse(&new_href).map_err(|_| {
                JsNativeError::typ().with_message(format!("URL: invalid URL: {new_href}"))
            })?;
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("URL: not a URL object"))?;
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    // origin: read-only.
    url_getter_only!("origin", |this, _args, ctx| {
        let parsed = read_url(this, ctx)?;
        Ok(JsValue::from(js_string!(parsed
            .origin()
            .ascii_serialization()
            .as_str())))
    });

    // protocol: strip trailing colon on set.
    url_accessor!(
        "protocol",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            let protocol = format!("{}:", parsed.scheme());
            Ok(JsValue::from(js_string!(protocol.as_str())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let scheme = val.strip_suffix(':').unwrap_or(&val);
            let mut parsed = read_url(this, ctx)?;
            if parsed.set_scheme(scheme).is_ok() {
                let obj = this.as_object().unwrap();
                sync_url_properties(&obj, &parsed, ctx);
            }
            Ok(JsValue::undefined())
        }
    );

    // host
    url_accessor!(
        "host",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            let host = parsed
                .host_str()
                .map(|h| {
                    if let Some(port) = parsed.port() {
                        format!("{h}:{port}")
                    } else {
                        h.to_string()
                    }
                })
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(host.as_str())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut parsed = read_url(this, ctx)?;
            let _ = parsed.set_host(Some(&val));
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    // hostname
    url_accessor!(
        "hostname",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            Ok(JsValue::from(js_string!(parsed.host_str().unwrap_or(""))))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut parsed = read_url(this, ctx)?;
            let _ = parsed.set_host(Some(&val));
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    // port: "" removes port.
    url_accessor!(
        "port",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            Ok(JsValue::from(js_string!(parsed
                .port()
                .map_or(String::new(), |p| p.to_string())
                .as_str())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut parsed = read_url(this, ctx)?;
            if val.is_empty() {
                let _ = parsed.set_port(None);
            } else if let Ok(p) = val.parse::<u16>() {
                let _ = parsed.set_port(Some(p));
            }
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    // pathname
    url_accessor!(
        "pathname",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            Ok(JsValue::from(js_string!(parsed.path())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut parsed = read_url(this, ctx)?;
            parsed.set_path(&val);
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    // search: strip leading ?.
    url_accessor!(
        "search",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            let search = parsed.query().map_or(String::new(), |q| format!("?{q}"));
            Ok(JsValue::from(js_string!(search.as_str())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
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
        }
    );

    // hash: strip leading #.
    url_accessor!(
        "hash",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            let hash = parsed.fragment().map_or(String::new(), |f| format!("#{f}"));
            Ok(JsValue::from(js_string!(hash.as_str())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
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
        }
    );

    // username
    url_accessor!(
        "username",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            Ok(JsValue::from(js_string!(parsed.username())))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut parsed = read_url(this, ctx)?;
            let _ = parsed.set_username(&val);
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    // password
    url_accessor!(
        "password",
        |this, _args, ctx| {
            let parsed = read_url(this, ctx)?;
            Ok(JsValue::from(js_string!(parsed.password().unwrap_or(""))))
        },
        |this, args, ctx| {
            let val = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let mut parsed = read_url(this, ctx)?;
            let _ = parsed.set_password(if val.is_empty() { None } else { Some(&val) });
            let obj = this.as_object().unwrap();
            sync_url_properties(&obj, &parsed, ctx);
            Ok(JsValue::undefined())
        }
    );

    url_obj.into()
}

/// Build a JS `URLSearchParams` object from a list of (key, value) pairs.
#[allow(clippy::too_many_lines)]
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
            let value = params
                .iter()
                .find(|(k, _)| k == &name)
                .map(|(_, v)| v.as_str());
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
            let callback = args
                .first()
                .and_then(JsValue::as_callable)
                .ok_or_else(|| JsNativeError::typ().with_message("forEach: callback required"))?;
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

/// Parse `URLSearchParams` init argument (WHATWG URL §6.2).
///
/// Handles three init forms:
/// 1. String: `"key=value&key2=value2"` (strip leading `?`)
/// 2. Array (sequence of sequences): `[["key", "value"], ...]`
/// 3. Object (record): `{ key: "value", ... }`
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

    // Object forms: array or record.
    if let Some(obj) = val.as_object() {
        // Check for array-like (has numeric `length` property).
        let len_val = obj.get(js_string!("length"), ctx)?;
        if let Some(len) = len_val.as_number() {
            // Array form: sequence of [key, value] pairs.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let len = len as u32;
            let mut params = Vec::with_capacity(len as usize);
            for i in 0..len {
                let pair = obj.get(i, ctx)?;
                let pair_obj = pair.as_object().ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("URLSearchParams: each entry must be a [key, value] pair")
                })?;
                let k = pair_obj
                    .get(0_u32, ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                let v = pair_obj
                    .get(1_u32, ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                params.push((k, v));
            }
            return Ok(params);
        }

        // Record form: iterate own property keys.
        let keys = obj.own_property_keys(ctx)?;
        let mut params = Vec::with_capacity(keys.len());
        for key in keys {
            let k = key.to_string();
            // Skip internal/hidden properties.
            if k.starts_with("__") {
                continue;
            }
            let v = obj
                .get(js_string!(&*k), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            params.push((k.clone(), v));
        }
        return Ok(params);
    }

    // Fallback: convert to string.
    let s = val.to_string(ctx)?.to_std_string_escaped();
    let s = s.strip_prefix('?').unwrap_or(&s);
    Ok(url::form_urlencoded::parse(s.as_bytes())
        .into_owned()
        .collect())
}

/// Get params from the hidden `__params__` property.
fn get_params(this: &JsValue, ctx: &mut Context) -> JsResult<Vec<(String, String)>> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("URLSearchParams: not an object"))?;
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
/// If the `URLSearchParams` is linked to a parent URL object (via `__url_obj__`),
/// also update the parent URL's query string and sync all its properties.
fn set_params(this: &JsValue, params: &[(String, String)], ctx: &mut Context) -> JsResult<()> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("URLSearchParams: not an object"))?;
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
