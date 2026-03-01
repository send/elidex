//! `window.location` object registration.
//!
//! Provides getters for URL components (href, protocol, host, hostname, port,
//! pathname, search, hash, origin) and navigation methods (assign, replace, reload).
//!
//! Navigation methods set a `pending_navigation` on the bridge rather than
//! navigating immediately, since navigation requires replacing the entire
//! pipeline (done by the shell after eval completes).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use elidex_navigation::NavigationRequest;

use crate::bridge::HostBridge;

/// Register the `window.location` object.
///
/// The object provides getters for URL components and navigation methods.
/// Setting `location.href` or calling `assign()`/`replace()` sets a
/// `pending_navigation` on the bridge.
#[allow(clippy::too_many_lines)]
pub fn register_location(ctx: &mut Context, bridge: &HostBridge) -> JsValue {
    // Clone the realm before creating ObjectInitializer to avoid borrow conflict.
    let realm = ctx.realm().clone();
    let mut init = ObjectInitializer::new(ctx);

    // --- Getters as computed properties ---

    // href getter + setter
    let b = bridge.clone();
    let b_set = bridge.clone();
    init.accessor(
        js_string!("href"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| u.as_str().to_string()))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, args, bridge, ctx| -> JsResult<JsValue> {
                    let href = args
                        .first()
                        .map(|v| v.to_string(ctx))
                        .transpose()?
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    bridge.set_pending_navigation(NavigationRequest {
                        url: href,
                        replace: false,
                    });
                    Ok(JsValue::undefined())
                },
                b_set,
            )
            .to_js_function(&realm),
        ),
        Attribute::CONFIGURABLE,
    );

    // protocol
    let b = bridge.clone();
    init.accessor(
        js_string!("protocol"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| format!("{}:", u.scheme())))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // host (hostname:port or just hostname)
    let b = bridge.clone();
    init.accessor(
        js_string!("host"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| {
                        if let Some(port) = u.port() {
                            format!("{}:{port}", u.host_str().unwrap_or(""))
                        } else {
                            u.host_str().unwrap_or("").to_string()
                        }
                    }))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // hostname
    let b = bridge.clone();
    init.accessor(
        js_string!("hostname"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| u.host_str().unwrap_or("").to_string()))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // port
    let b = bridge.clone();
    init.accessor(
        js_string!("port"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| {
                        u.port().map_or_else(String::new, |p| p.to_string())
                    }))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // pathname
    let b = bridge.clone();
    init.accessor(
        js_string!("pathname"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| u.path().to_string()))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // search
    let b = bridge.clone();
    init.accessor(
        js_string!("search"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| {
                        u.query().map_or_else(String::new, |q| format!("?{q}"))
                    }))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // hash
    let b = bridge.clone();
    init.accessor(
        js_string!("hash"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| {
                        u.fragment().map_or_else(String::new, |f| format!("#{f}"))
                    }))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // origin
    let b = bridge.clone();
    init.accessor(
        js_string!("origin"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                    Ok(url_prop(bridge, |u| u.origin().unicode_serialization()))
                },
                b,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // --- Methods ---

    // assign(url)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let url = args
                    .first()
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("location.assign: URL argument required")
                    })?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                bridge.set_pending_navigation(NavigationRequest {
                    url,
                    replace: false,
                });
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("assign"),
        1,
    );

    // replace(url)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| -> JsResult<JsValue> {
                let url = args
                    .first()
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("location.replace: URL argument required")
                    })?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                bridge.set_pending_navigation(NavigationRequest { url, replace: true });
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("replace"),
        1,
    );

    // reload()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                if let Some(url) = bridge.current_url() {
                    bridge.set_pending_navigation(NavigationRequest {
                        url: url.to_string(),
                        replace: true,
                    });
                }
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("reload"),
        0,
    );

    // toString() — same as href
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, bridge, _ctx| -> JsResult<JsValue> {
                Ok(url_prop(bridge, |u| u.as_str().to_string()))
            },
            b,
        ),
        js_string!("toString"),
        0,
    );

    init.build().into()
}

/// Helper: extract a property from the current URL, returning "" if no URL is set.
fn url_prop(bridge: &HostBridge, f: impl FnOnce(&url::Url) -> String) -> JsValue {
    match bridge.current_url() {
        Some(url) => JsValue::from(js_string!(f(&url))),
        None => JsValue::from(js_string!("")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;

    fn setup_with_url(url_str: &str) -> (Context, HostBridge) {
        let bridge = HostBridge::new();
        bridge.set_current_url(Some(url::Url::parse(url_str).unwrap()));

        let mut ctx = Context::default();
        let location_obj = register_location(&mut ctx, &bridge);

        let global = ctx.global_object();
        global
            .set(js_string!("location"), location_obj, false, &mut ctx)
            .unwrap();

        (ctx, bridge)
    }

    fn eval_str(ctx: &mut Context, code: &str) -> String {
        ctx.eval(Source::from_bytes(code))
            .unwrap()
            .to_string(ctx)
            .unwrap()
            .to_std_string_escaped()
    }

    #[test]
    fn location_href() {
        let (mut ctx, _) = setup_with_url("https://example.com/path?q=1#frag");
        assert_eq!(
            eval_str(&mut ctx, "location.href"),
            "https://example.com/path?q=1#frag"
        );
    }

    #[test]
    fn location_protocol() {
        let (mut ctx, _) = setup_with_url("https://example.com/");
        assert_eq!(eval_str(&mut ctx, "location.protocol"), "https:");
    }

    #[test]
    fn location_host_with_port() {
        let (mut ctx, _) = setup_with_url("https://example.com:8080/");
        assert_eq!(eval_str(&mut ctx, "location.host"), "example.com:8080");
    }

    #[test]
    fn location_host_without_port() {
        let (mut ctx, _) = setup_with_url("https://example.com/");
        assert_eq!(eval_str(&mut ctx, "location.host"), "example.com");
    }

    #[test]
    fn location_hostname() {
        let (mut ctx, _) = setup_with_url("https://example.com:8080/");
        assert_eq!(eval_str(&mut ctx, "location.hostname"), "example.com");
    }

    #[test]
    fn location_port() {
        let (mut ctx, _) = setup_with_url("https://example.com:8080/");
        assert_eq!(eval_str(&mut ctx, "location.port"), "8080");
    }

    #[test]
    fn location_port_default() {
        let (mut ctx, _) = setup_with_url("https://example.com/");
        assert_eq!(eval_str(&mut ctx, "location.port"), "");
    }

    #[test]
    fn location_pathname() {
        let (mut ctx, _) = setup_with_url("https://example.com/foo/bar");
        assert_eq!(eval_str(&mut ctx, "location.pathname"), "/foo/bar");
    }

    #[test]
    fn location_search() {
        let (mut ctx, _) = setup_with_url("https://example.com/?key=val");
        assert_eq!(eval_str(&mut ctx, "location.search"), "?key=val");
    }

    #[test]
    fn location_search_empty() {
        let (mut ctx, _) = setup_with_url("https://example.com/");
        assert_eq!(eval_str(&mut ctx, "location.search"), "");
    }

    #[test]
    fn location_hash() {
        let (mut ctx, _) = setup_with_url("https://example.com/#section");
        assert_eq!(eval_str(&mut ctx, "location.hash"), "#section");
    }

    #[test]
    fn location_origin() {
        let (mut ctx, _) = setup_with_url("https://example.com:8080/path");
        assert_eq!(
            eval_str(&mut ctx, "location.origin"),
            "https://example.com:8080"
        );
    }

    #[test]
    fn location_assign() {
        let (mut ctx, bridge) = setup_with_url("https://example.com/");
        ctx.eval(Source::from_bytes("location.assign('https://other.com/')"))
            .unwrap();
        let nav = bridge.take_pending_navigation().unwrap();
        assert_eq!(nav.url, "https://other.com/");
        assert!(!nav.replace);
    }

    #[test]
    fn location_replace() {
        let (mut ctx, bridge) = setup_with_url("https://example.com/");
        ctx.eval(Source::from_bytes("location.replace('https://other.com/')"))
            .unwrap();
        let nav = bridge.take_pending_navigation().unwrap();
        assert_eq!(nav.url, "https://other.com/");
        assert!(nav.replace);
    }

    #[test]
    fn location_href_setter() {
        let (mut ctx, bridge) = setup_with_url("https://example.com/");
        ctx.eval(Source::from_bytes("location.href = 'https://new.com/'"))
            .unwrap();
        let nav = bridge.take_pending_navigation().unwrap();
        assert_eq!(nav.url, "https://new.com/");
        assert!(!nav.replace);
    }

    #[test]
    fn location_reload() {
        let (mut ctx, bridge) = setup_with_url("https://example.com/page");
        ctx.eval(Source::from_bytes("location.reload()")).unwrap();
        let nav = bridge.take_pending_navigation().unwrap();
        assert_eq!(nav.url, "https://example.com/page");
        assert!(nav.replace);
    }

    #[test]
    fn location_no_url() {
        let bridge = HostBridge::new();
        let mut ctx = Context::default();
        let location_obj = register_location(&mut ctx, &bridge);
        let global = ctx.global_object();
        global
            .set(js_string!("location"), location_obj, false, &mut ctx)
            .unwrap();
        assert_eq!(eval_str(&mut ctx, "location.href"), "");
    }

    #[test]
    fn location_to_string() {
        let (mut ctx, _) = setup_with_url("https://example.com/path");
        assert_eq!(
            eval_str(&mut ctx, "location.toString()"),
            "https://example.com/path"
        );
    }
}
