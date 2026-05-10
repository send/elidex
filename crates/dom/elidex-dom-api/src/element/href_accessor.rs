//! HTMLHyperlinkElementUtils mixin URL accessor algorithm
//! (slot `#11-tags-T2a-url-bearing`、HTML §4.6.5).
//!
//! `<a>` and `<area>` share an 11-property URL accessor surface
//! plus a `toString()` method.  Each getter reads the `href` content
//! attribute, resolves it against the document base URL, parses with
//! `url::Url::parse_with_base_url` (relative resolution), and emits
//! one component string.  Setters mutate one URL component, serialise
//! the parsed URL, and write back to the `href` content attribute via
//! `EcsDom::set_attribute` (lesson #181 — canonical write path).
//!
//! ## Layering
//!
//! Engine-independent.  All URL manipulation is performed against
//! `url::Url`; the VM `host/` side is restricted to handler dispatch
//! per CLAUDE.md "Layering mandate".  Defer slot
//! `#11-base-href-resolution` covers `<base href>` walk + the actual
//! `document.URL` populate; in this PR the base URL is the
//! `"about:blank"` placeholder.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use url::Url;

use crate::util::{not_found_error, require_string_arg};
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, SessionCore};

/// Document base-URL placeholder until `#11-base-href-resolution`
/// lands real navigation state + `<base href>` walking.  Matches the
/// stub returned by `document.URL` (`char_data/document_props.rs`).
const BASE_URL_PLACEHOLDER: &str = "about:blank";

/// Read the `href` content attribute, resolve against the base URL,
/// parse with `url::Url`, and call the supplied closure with the
/// parsed URL.  Returns the closure's `String` result, or `""` if the
/// `href` attribute is absent / unparseable per WHATWG URL §6.2.
pub fn href_url_component<F>(entity: Entity, dom: &EcsDom, f: F) -> Result<String, DomApiError>
where
    F: FnOnce(&Url) -> String,
{
    let href = read_href_attr(entity, dom)?;
    match parse_with_base(&href) {
        Some(url) => Ok(f(&url)),
        None => Ok(String::new()),
    }
}

/// Read `href`, parse, mutate one component via the supplied closure,
/// serialise, and write back to the `href` attribute.  No-op if the
/// `href` attribute is unparseable (matches V8 / Firefox).  Returns
/// `Err` only for ECS access failures.
pub fn href_url_set_component<F>(
    entity: Entity,
    dom: &mut EcsDom,
    mutate: F,
) -> Result<(), DomApiError>
where
    F: FnOnce(&mut Url),
{
    let href = read_href_attr(entity, dom)?;
    if let Some(mut url) = parse_with_base(&href) {
        mutate(&mut url);
        write_href_attr(entity, dom, url.as_str().to_string())?;
    }
    Ok(())
}

/// Set the `href` attribute to a freshly-parsed URL string.  Used by
/// the `href` setter directly (no component mutation, just
/// re-serialisation through `url::Url` to canonicalise).
pub fn set_href(entity: Entity, dom: &mut EcsDom, value: &str) -> Result<(), DomApiError> {
    // WHATWG URL setter parses + re-serialises; if parsing fails the
    // setter still stores the raw string.  Match V8: store as-is.
    write_href_attr(entity, dom, value.to_string())
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn read_href_attr(entity: Entity, dom: &EcsDom) -> Result<String, DomApiError> {
    let attrs = dom
        .world()
        .get::<&Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))?;
    Ok(attrs.get("href").unwrap_or("").to_string())
}

fn write_href_attr(entity: Entity, dom: &mut EcsDom, value: String) -> Result<(), DomApiError> {
    let mut attrs = dom
        .world_mut()
        .get::<&mut Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))?;
    attrs.set("href", value);
    Ok(())
}

fn parse_with_base(href: &str) -> Option<Url> {
    if href.is_empty() {
        return None;
    }
    // Try absolute parse first; if relative, resolve against base.
    Url::parse(href).ok().or_else(|| {
        Url::parse(BASE_URL_PLACEHOLDER)
            .ok()
            .and_then(|base| base.join(href).ok())
    })
}

// ---------------------------------------------------------------------------
// Per-component closure helpers — the IDL accessors call these via
// `href_url_component` so the closure body stays small.
// ---------------------------------------------------------------------------

/// Emit `URL.protocol` per WHATWG URL §6.1 (trailing `:`).
pub fn component_protocol(u: &Url) -> String {
    format!("{}:", u.scheme())
}

/// Emit `URL.host` (`hostname[:port]`).
pub fn component_host(u: &Url) -> String {
    match (u.host_str(), u.port()) {
        (Some(h), Some(p)) => format!("{h}:{p}"),
        (Some(h), None) => h.to_string(),
        (None, _) => String::new(),
    }
}

/// Emit `URL.hostname`.
pub fn component_hostname(u: &Url) -> String {
    u.host_str().unwrap_or("").to_string()
}

/// Emit `URL.port` ("" when absent, decimal otherwise).
pub fn component_port(u: &Url) -> String {
    u.port().map_or_else(String::new, |p| p.to_string())
}

/// Emit `URL.pathname`.
pub fn component_pathname(u: &Url) -> String {
    u.path().to_string()
}

/// Emit `URL.search` (leading `?` retained when non-empty per spec).
pub fn component_search(u: &Url) -> String {
    u.query().map_or_else(String::new, |q| format!("?{q}"))
}

/// Emit `URL.hash` (leading `#` retained when non-empty per spec).
pub fn component_hash(u: &Url) -> String {
    u.fragment().map_or_else(String::new, |h| format!("#{h}"))
}

/// Emit `URL.username`.
pub fn component_username(u: &Url) -> String {
    u.username().to_string()
}

/// Emit `URL.password`.
pub fn component_password(u: &Url) -> String {
    u.password().unwrap_or("").to_string()
}

/// Emit `URL.origin` ASCII serialisation.  Read-only per WHATWG.
pub fn component_origin(u: &Url) -> String {
    u.origin().ascii_serialization()
}

/// Emit `URL.href` (full serialisation per WHATWG URL §4.5).  Used
/// for `toString()` and the `href` getter.
pub fn component_href(u: &Url) -> String {
    u.as_str().to_string()
}

// ---------------------------------------------------------------------------
// Attribute-validation helper (shared with setters)
// ---------------------------------------------------------------------------

/// Wrap a `url::Url::set_*` failure as a TypeError.  Setters that
/// silently no-op on bad input use `let _ = ...set_*(...)` instead.
pub fn invalid_url_setter(component: &'static str) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::TypeError,
        message: format!("Invalid value for `{component}`"),
    }
}

// ===========================================================================
// DomApiHandler structs — registered as `"hyperlink.<component>.{get,set}"`
// in the dom registry.  VM host calls `invoke_dom_api(ctx, "...", entity, ...)`.
//
// Each getter calls `href_url_component` with the matching component
// closure.  Each setter (where allowed by spec) calls
// `href_url_set_component` with a `url::Url::set_*` mutation.
// ===========================================================================

macro_rules! getter_handler {
    ($name:ident, $method_name:literal, $component:path) => {
        pub struct $name;
        impl DomApiHandler for $name {
            fn method_name(&self) -> &str {
                $method_name
            }
            fn invoke(
                &self,
                this: Entity,
                _args: &[JsValue],
                _session: &mut SessionCore,
                dom: &mut EcsDom,
            ) -> Result<JsValue, DomApiError> {
                Ok(JsValue::String(href_url_component(this, dom, $component)?))
            }
        }
    };
}

getter_handler!(HyperlinkHrefGet, "hyperlink.href.get", component_href);
getter_handler!(HyperlinkOriginGet, "hyperlink.origin.get", component_origin);
getter_handler!(
    HyperlinkProtocolGet,
    "hyperlink.protocol.get",
    component_protocol
);
getter_handler!(
    HyperlinkUsernameGet,
    "hyperlink.username.get",
    component_username
);
getter_handler!(
    HyperlinkPasswordGet,
    "hyperlink.password.get",
    component_password
);
getter_handler!(HyperlinkHostGet, "hyperlink.host.get", component_host);
getter_handler!(
    HyperlinkHostnameGet,
    "hyperlink.hostname.get",
    component_hostname
);
getter_handler!(HyperlinkPortGet, "hyperlink.port.get", component_port);
getter_handler!(
    HyperlinkPathnameGet,
    "hyperlink.pathname.get",
    component_pathname
);
getter_handler!(HyperlinkSearchGet, "hyperlink.search.get", component_search);
getter_handler!(HyperlinkHashGet, "hyperlink.hash.get", component_hash);

/// `href` setter — replaces the entire URL via `EcsDom::set_attribute`
/// canonical write path (lesson #181).  V8/Firefox precedent: invalid
/// URL strings are still stored as-is (no parse-time TypeError).
pub struct HyperlinkHrefSet;
impl DomApiHandler for HyperlinkHrefSet {
    fn method_name(&self) -> &str {
        "hyperlink.href.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;
        set_href(this, dom, &value)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

macro_rules! setter_handler {
    ($name:ident, $method_name:literal, |$url:ident, $val:ident| $body:expr) => {
        pub struct $name;
        impl DomApiHandler for $name {
            fn method_name(&self) -> &str {
                $method_name
            }
            fn invoke(
                &self,
                this: Entity,
                args: &[JsValue],
                _session: &mut SessionCore,
                dom: &mut EcsDom,
            ) -> Result<JsValue, DomApiError> {
                let $val = require_string_arg(args, 0)?;
                href_url_set_component(this, dom, |$url| {
                    $body;
                })?;
                dom.rev_version(this);
                Ok(JsValue::Undefined)
            }
        }
    };
}

setter_handler!(HyperlinkProtocolSet, "hyperlink.protocol.set", |u, v| {
    let scheme = v.trim_end_matches(':');
    let _ = u.set_scheme(scheme);
});
setter_handler!(HyperlinkUsernameSet, "hyperlink.username.set", |u, v| {
    let _ = u.set_username(&v);
});
setter_handler!(HyperlinkPasswordSet, "hyperlink.password.set", |u, v| {
    let _ = u.set_password(if v.is_empty() { None } else { Some(&v) });
});
setter_handler!(HyperlinkHostSet, "hyperlink.host.set", |u, v| {
    // host setter accepts "hostname[:port]" — split on last `:` to
    // separate (matches WHATWG URL §6.1 host setter).
    if let Some((host_part, port_part)) = v.rsplit_once(':') {
        let _ = u.set_host(if host_part.is_empty() {
            None
        } else {
            Some(host_part)
        });
        if let Ok(p) = port_part.parse::<u16>() {
            let _ = u.set_port(Some(p));
        }
    } else {
        let _ = u.set_host(if v.is_empty() { None } else { Some(&v) });
    }
});
setter_handler!(HyperlinkHostnameSet, "hyperlink.hostname.set", |u, v| {
    let _ = u.set_host(if v.is_empty() { None } else { Some(&v) });
});
setter_handler!(HyperlinkPortSet, "hyperlink.port.set", |u, v| {
    let port = if v.is_empty() {
        None
    } else {
        v.parse::<u16>().ok()
    };
    let _ = u.set_port(port);
});
setter_handler!(HyperlinkPathnameSet, "hyperlink.pathname.set", |u, v| {
    u.set_path(&v);
});
setter_handler!(HyperlinkSearchSet, "hyperlink.search.set", |u, v| {
    let stripped = v.strip_prefix('?').unwrap_or(&v);
    u.set_query(if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    });
});
setter_handler!(HyperlinkHashSet, "hyperlink.hash.set", |u, v| {
    let stripped = v.strip_prefix('#').unwrap_or(&v);
    u.set_fragment(if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    });
});

/// `toString()` — alias for `href` getter (HTMLHyperlinkElementUtils §4.6.5).
pub struct HyperlinkToString;
impl DomApiHandler for HyperlinkToString {
    fn method_name(&self) -> &str {
        "hyperlink.toString"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String(href_url_component(
            this,
            dom,
            component_href,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_with_trailing_colon() {
        let u = Url::parse("https://example.com/").unwrap();
        assert_eq!(component_protocol(&u), "https:");
    }

    #[test]
    fn host_with_port() {
        let u = Url::parse("https://example.com:8443/").unwrap();
        assert_eq!(component_host(&u), "example.com:8443");
    }

    #[test]
    fn host_default_port_stripped() {
        let u = Url::parse("https://example.com/").unwrap();
        assert_eq!(component_host(&u), "example.com");
    }

    #[test]
    fn search_includes_leading_question() {
        let u = Url::parse("https://example.com/?q=1").unwrap();
        assert_eq!(component_search(&u), "?q=1");
    }

    #[test]
    fn search_empty_when_absent() {
        let u = Url::parse("https://example.com/").unwrap();
        assert_eq!(component_search(&u), "");
    }

    #[test]
    fn hash_includes_leading_hash() {
        let u = Url::parse("https://example.com/#section").unwrap();
        assert_eq!(component_hash(&u), "#section");
    }
}
