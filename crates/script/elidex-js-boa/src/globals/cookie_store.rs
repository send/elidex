//! CookieStore API (WHATWG — cookiestore.spec.whatwg.org).
//!
//! Provides async cookie access via `cookieStore` global.
//! Available in both Window and ServiceWorker contexts.

use std::fmt::Write;

use boa_engine::object::builtins::{JsArray, JsPromise};
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsArgs, JsNativeError, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Build a JS CookieListItem from a `CookieSnapshot`.
///
/// Properties per WHATWG Cookie Store spec §4.1:
/// name, value, domain, path, expires, secure, sameSite.
fn build_cookie_list_item(snap: &elidex_net::CookieSnapshot, ctx: &mut Context) -> JsValue {
    let mut obj = ObjectInitializer::new(ctx);
    obj.property(
        js_string!("name"),
        JsValue::from(js_string!(snap.name.as_str())),
        Attribute::all(),
    );
    obj.property(
        js_string!("value"),
        JsValue::from(js_string!(snap.value.as_str())),
        Attribute::all(),
    );
    obj.property(
        js_string!("domain"),
        if snap.domain.is_empty() {
            JsValue::null()
        } else {
            JsValue::from(js_string!(snap.domain.as_str()))
        },
        Attribute::all(),
    );
    obj.property(
        js_string!("path"),
        JsValue::from(js_string!(snap.path.as_str())),
        Attribute::all(),
    );
    obj.property(
        js_string!("expires"),
        snap.expires.map_or(JsValue::null(), |exp| {
            let ms = exp
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_millis() as f64);
            JsValue::from(ms)
        }),
        Attribute::all(),
    );
    obj.property(
        js_string!("secure"),
        JsValue::from(snap.secure),
        Attribute::all(),
    );
    obj.property(
        js_string!("sameSite"),
        JsValue::from(js_string!(snap.same_site.as_str())),
        Attribute::all(),
    );
    JsValue::from(obj.build())
}

/// Register the `cookieStore` global object.
#[allow(clippy::too_many_lines)]
pub fn register_cookie_store(ctx: &mut Context, bridge: &HostBridge) {
    let mut init = ObjectInitializer::new(ctx);

    // get(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let name = args
                    .get_or_undefined(0)
                    .as_string()
                    .map(|s| s.to_std_string_escaped())
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("cookieStore.get requires a name")
                    })?;

                let details = bridge
                    .current_url()
                    .map(|u| bridge.cookie_details_for_script(&u))
                    .unwrap_or_default();

                let found = details.iter().find(|c| c.name == name);
                match found {
                    Some(snap) => {
                        let item = build_cookie_list_item(snap, ctx);
                        Ok(JsPromise::resolve(item, ctx).into())
                    }
                    None => Ok(JsPromise::resolve(JsValue::null(), ctx).into()),
                }
            },
            b,
        ),
        js_string!("get"),
        1,
    );

    // getAll(name?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let filter = args
                    .get_or_undefined(0)
                    .as_string()
                    .map(|s| s.to_std_string_escaped());

                let details = bridge
                    .current_url()
                    .map(|u| bridge.cookie_details_for_script(&u))
                    .unwrap_or_default();

                let arr = JsArray::new(ctx);
                for snap in &details {
                    if let Some(ref f) = filter {
                        if snap.name != *f {
                            continue;
                        }
                    }
                    arr.push(build_cookie_list_item(snap, ctx), ctx)?;
                }
                Ok(JsPromise::resolve(arr, ctx).into())
            },
            b,
        ),
        js_string!("getAll"),
        0,
    );

    // set(name, value) or set(options)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let first = args.get_or_undefined(0);
                let (name, value, attrs) = if let Some(obj) = first.as_object() {
                    // set(options) form: {name, value, domain?, path?, expires?, secure?, sameSite?}
                    let name = obj
                        .get(js_string!("name"), ctx)?
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                        .ok_or_else(|| {
                            JsNativeError::typ()
                                .with_message("cookieStore.set: options.name is required")
                        })?;
                    let value = obj
                        .get(js_string!("value"), ctx)?
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                        .ok_or_else(|| {
                            JsNativeError::typ()
                                .with_message("cookieStore.set: options.value is required")
                        })?;
                    let mut attrs = String::new();
                    if let Some(d) = obj
                        .get(js_string!("domain"), ctx)?
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                    {
                        write!(attrs, "; Domain={d}").unwrap();
                    }
                    if let Some(p) = obj
                        .get(js_string!("path"), ctx)?
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                    {
                        write!(attrs, "; Path={p}").unwrap();
                    }
                    if let Some(exp) = obj.get(js_string!("expires"), ctx)?.as_number() {
                        // expires is milliseconds since epoch; Max-Age in seconds from now.
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .map_or(0.0, |d| d.as_millis() as f64);
                        let max_age = ((exp - now_ms) / 1000.0).max(0.0) as u64;
                        write!(attrs, "; Max-Age={max_age}").unwrap();
                    }
                    if obj
                        .get(js_string!("secure"), ctx)?
                        .as_boolean()
                        .unwrap_or(false)
                    {
                        attrs.push_str("; Secure");
                    }
                    if let Some(ss) = obj
                        .get(js_string!("sameSite"), ctx)?
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                    {
                        write!(attrs, "; SameSite={ss}").unwrap();
                    }
                    (name, value, attrs)
                } else {
                    // set(name, value) form.
                    let name = first
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                        .ok_or_else(|| {
                            JsNativeError::typ().with_message("cookieStore.set requires a name")
                        })?;
                    let value = args
                        .get_or_undefined(1)
                        .as_string()
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    (name, value, String::new())
                };

                if let Some(url) = bridge.current_url() {
                    bridge.set_cookie_from_script(&url, &format!("{name}={value}{attrs}"));
                }
                Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
            },
            b,
        ),
        js_string!("set"),
        2,
    );

    // delete(name)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                let name = args
                    .get_or_undefined(0)
                    .as_string()
                    .map(|s| s.to_std_string_escaped())
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("cookieStore.delete requires a name")
                    })?;

                if let Some(url) = bridge.current_url() {
                    bridge.set_cookie_from_script(&url, &format!("{name}=; Max-Age=0"));
                }
                Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
            },
            b,
        ),
        js_string!("delete"),
        1,
    );

    let cookie_store = init.build();
    ctx.register_global_property(js_string!("cookieStore"), cookie_store, Attribute::all())
        .expect("register cookieStore");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_snap(name: &str, value: &str) -> elidex_net::CookieSnapshot {
        elidex_net::CookieSnapshot {
            name: name.into(),
            value: value.into(),
            domain: "example.com".into(),
            host: "example.com".into(),
            path: "/".into(),
            partition_key: String::new(),
            host_only: true,
            persistent: false,
            secure: true,
            http_only: false,
            same_site: "lax".into(),
            expires: None,
            creation_time: std::time::SystemTime::now(),
            last_access_time: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn build_cookie_list_item_has_all_properties() {
        let mut ctx = Context::default();
        let snap = mock_snap("sid", "abc123");
        let val = build_cookie_list_item(&snap, &mut ctx);
        let obj = val.as_object().unwrap();

        let name = obj.get(js_string!("name"), &mut ctx).unwrap();
        assert_eq!(name.as_string().unwrap().to_std_string_escaped(), "sid");

        let value = obj.get(js_string!("value"), &mut ctx).unwrap();
        assert_eq!(value.as_string().unwrap().to_std_string_escaped(), "abc123");

        let domain = obj.get(js_string!("domain"), &mut ctx).unwrap();
        assert_eq!(
            domain.as_string().unwrap().to_std_string_escaped(),
            "example.com"
        );

        let path = obj.get(js_string!("path"), &mut ctx).unwrap();
        assert_eq!(path.as_string().unwrap().to_std_string_escaped(), "/");

        let secure = obj.get(js_string!("secure"), &mut ctx).unwrap();
        assert_eq!(secure.as_boolean(), Some(true));

        let same_site = obj.get(js_string!("sameSite"), &mut ctx).unwrap();
        assert_eq!(
            same_site.as_string().unwrap().to_std_string_escaped(),
            "lax"
        );

        // expires is null for session cookies
        let expires = obj.get(js_string!("expires"), &mut ctx).unwrap();
        assert!(expires.is_null());
    }

    #[test]
    fn build_cookie_list_item_with_expiry() {
        let mut ctx = Context::default();
        let mut snap = mock_snap("token", "xyz");
        snap.expires =
            Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000));
        let val = build_cookie_list_item(&snap, &mut ctx);
        let obj = val.as_object().unwrap();

        let expires = obj.get(js_string!("expires"), &mut ctx).unwrap();
        let ms = expires.as_number().unwrap();
        assert!((ms - 1_700_000_000_000.0).abs() < 1.0);
    }

    #[test]
    fn build_cookie_list_item_null_domain() {
        let mut ctx = Context::default();
        let mut snap = mock_snap("k", "v");
        snap.domain = String::new();
        let val = build_cookie_list_item(&snap, &mut ctx);
        let obj = val.as_object().unwrap();
        let domain = obj.get(js_string!("domain"), &mut ctx).unwrap();
        assert!(domain.is_null());
    }
}
