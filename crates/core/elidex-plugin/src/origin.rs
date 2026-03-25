//! Security origin and iframe sandbox types (WHATWG HTML §7.5, §4.8.5).
//!
//! Provides [`SecurityOrigin`] for same-origin policy enforcement and
//! [`IframeSandboxFlags`] for `<iframe sandbox>` attribute parsing.

use std::sync::atomic::{AtomicU64, Ordering};

/// Maximum nesting depth for iframes.
///
/// Prevents runaway recursion from `<iframe>` nesting. Typical browser
/// implementations cap at 10–500; 128 is a safe middle ground.
pub const MAX_IFRAME_DEPTH: usize = 128;

/// Counter for generating unique opaque origin IDs.
static OPAQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Security origin per WHATWG HTML §7.5.
///
/// Used for same-origin policy enforcement on `<iframe>` boundaries.
/// Distinct from [`elidex_css::Origin`] (cascade origin: UserAgent/Author).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SecurityOrigin {
    /// Tuple origin for http/https URLs: (scheme, host, port).
    Tuple {
        scheme: String,
        host: String,
        port: u16,
    },
    /// Opaque origin for sandboxed iframes, data: URLs, file: URLs, etc.
    /// Each opaque origin is globally unique (even from the same sandbox).
    Opaque(u64),
}

impl SecurityOrigin {
    /// Derive a security origin from a URL.
    ///
    /// - `http`/`https` → `Tuple { scheme, host, port }` (default ports: 80/443)
    /// - `file` → `Opaque` (WHATWG §7.5.2: file URLs have opaque origin)
    /// - `data` → `Opaque` (WHATWG §7.5.3)
    /// - `blob` → `Opaque` (TODO: blob URL registry for creator origin)
    /// - Other schemes → `Opaque`
    #[must_use]
    pub fn from_url(url: &url::Url) -> Self {
        match url.scheme() {
            "http" | "https" => {
                let host = url.host_str().unwrap_or("").to_string();
                let port = url.port_or_known_default().unwrap_or(0);
                Self::Tuple {
                    scheme: url.scheme().to_string(),
                    host,
                    port,
                }
            }
            // file:// URLs get opaque origin per WHATWG §7.5.2.
            // data: URLs get opaque origin per WHATWG §7.5.3.
            // blob: URLs should inherit creator origin, but we don't have
            // a blob URL registry yet — use opaque as safe default.
            _ => Self::opaque(),
        }
    }

    /// Check whether two origins are the same.
    ///
    /// Two opaque origins are never same-origin (even with the same ID),
    /// matching WHATWG §7.5 which states each opaque origin is unique.
    /// In practice, opaque origins created by [`Self::opaque()`] always
    /// have different IDs, so this check is equivalent.
    #[must_use]
    pub fn same_origin(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Tuple {
                    scheme: s1,
                    host: h1,
                    port: p1,
                },
                Self::Tuple {
                    scheme: s2,
                    host: h2,
                    port: p2,
                },
            ) => s1 == s2 && h1 == h2 && p1 == p2,
            // Opaque origins are never same-origin per WHATWG §7.5.
            _ => false,
        }
    }

    /// Create a new unique opaque origin.
    ///
    /// Each call returns a distinct opaque origin, matching WHATWG §7.5
    /// which requires every opaque origin to be globally unique.
    #[must_use]
    pub fn opaque() -> Self {
        Self::Opaque(OPAQUE_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the origin as a serialized string (WHATWG §7.5).
    ///
    /// Tuple origins serialize as `"scheme://host:port"`.
    /// Opaque origins serialize as `"null"`.
    #[must_use]
    pub fn serialize(&self) -> String {
        match self {
            Self::Tuple { scheme, host, port } => {
                let default_port = match scheme.as_str() {
                    "http" => 80,
                    "https" => 443,
                    _ => 0,
                };
                if *port == default_port {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                }
            }
            Self::Opaque(_) => "null".to_string(),
        }
    }
}

impl std::fmt::Display for SecurityOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.serialize())
    }
}

// ---------------------------------------------------------------------------
// IframeSandboxFlags
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    /// Sandbox flags for `<iframe sandbox>` attribute (WHATWG HTML §4.8.5).
    ///
    /// An empty `sandbox` attribute (no tokens) means all flags are cleared
    /// (maximum restrictions). Each `allow-*` token sets a corresponding flag.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub struct IframeSandboxFlags: u16 {
        /// Allow script execution in the sandboxed iframe.
        const ALLOW_SCRIPTS        = 1 << 0;
        /// Treat the iframe as same-origin with its parent (instead of opaque).
        const ALLOW_SAME_ORIGIN    = 1 << 1;
        /// Allow form submission.
        const ALLOW_FORMS          = 1 << 2;
        /// Allow `window.open()` and `target="_blank"` links.
        const ALLOW_POPUPS         = 1 << 3;
        /// Allow navigation of the top-level browsing context.
        const ALLOW_TOP_NAVIGATION = 1 << 4;
        /// Allow `alert()`, `confirm()`, and `prompt()` modals.
        const ALLOW_MODALS         = 1 << 5;
    }
}

/// Parse the `sandbox` attribute value into [`IframeSandboxFlags`].
///
/// An empty string or `None` returns empty flags (all restrictions enabled).
/// Unrecognized tokens are silently ignored per WHATWG HTML §4.8.5.
#[must_use]
pub fn parse_sandbox_attribute(value: &str) -> IframeSandboxFlags {
    let mut flags = IframeSandboxFlags::empty();
    for token in value.split_ascii_whitespace() {
        match token {
            "allow-scripts" => flags |= IframeSandboxFlags::ALLOW_SCRIPTS,
            "allow-same-origin" => flags |= IframeSandboxFlags::ALLOW_SAME_ORIGIN,
            "allow-forms" => flags |= IframeSandboxFlags::ALLOW_FORMS,
            "allow-popups" => flags |= IframeSandboxFlags::ALLOW_POPUPS,
            "allow-top-navigation" => flags |= IframeSandboxFlags::ALLOW_TOP_NAVIGATION,
            "allow-modals" => flags |= IframeSandboxFlags::ALLOW_MODALS,
            _ => {} // Unrecognized tokens silently ignored per spec.
        }
    }
    flags
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_origin_http() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("http://example.com/other").unwrap());
        assert!(a.same_origin(&b));
    }

    #[test]
    fn cross_origin_different_scheme() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(!a.same_origin(&b));
    }

    #[test]
    fn cross_origin_different_port() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("http://example.com:8080").unwrap());
        assert!(!a.same_origin(&b));
    }

    #[test]
    fn cross_origin_different_host() {
        let a = SecurityOrigin::from_url(&url::Url::parse("https://a.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("https://b.com").unwrap());
        assert!(!a.same_origin(&b));
    }

    #[test]
    fn default_port_normalization() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("http://example.com:80").unwrap());
        assert!(a.same_origin(&b));

        let c = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let d = SecurityOrigin::from_url(&url::Url::parse("https://example.com:443").unwrap());
        assert!(c.same_origin(&d));
    }

    #[test]
    fn opaque_origins_never_same() {
        let a = SecurityOrigin::opaque();
        let b = SecurityOrigin::opaque();
        assert!(!a.same_origin(&b));
        assert!(!a.same_origin(&a));
    }

    #[test]
    fn opaque_ids_are_unique() {
        let a = SecurityOrigin::opaque();
        let b = SecurityOrigin::opaque();
        match (&a, &b) {
            (SecurityOrigin::Opaque(id_a), SecurityOrigin::Opaque(id_b)) => {
                assert_ne!(id_a, id_b);
            }
            _ => panic!("expected Opaque variants"),
        }
    }

    #[test]
    fn file_url_is_opaque() {
        let origin = SecurityOrigin::from_url(&url::Url::parse("file:///tmp/test.html").unwrap());
        assert!(matches!(origin, SecurityOrigin::Opaque(_)));
    }

    #[test]
    fn data_url_is_opaque() {
        let origin =
            SecurityOrigin::from_url(&url::Url::parse("data:text/html,<h1>Hi</h1>").unwrap());
        assert!(matches!(origin, SecurityOrigin::Opaque(_)));
    }

    #[test]
    fn serialize_tuple_origin() {
        let origin =
            SecurityOrigin::from_url(&url::Url::parse("https://example.com/page").unwrap());
        assert_eq!(origin.serialize(), "https://example.com");

        let origin_port =
            SecurityOrigin::from_url(&url::Url::parse("http://example.com:8080").unwrap());
        assert_eq!(origin_port.serialize(), "http://example.com:8080");
    }

    #[test]
    fn serialize_opaque_origin() {
        assert_eq!(SecurityOrigin::opaque().serialize(), "null");
    }

    #[test]
    fn sandbox_empty_string() {
        let flags = parse_sandbox_attribute("");
        assert!(flags.is_empty());
    }

    #[test]
    fn sandbox_single_token() {
        let flags = parse_sandbox_attribute("allow-scripts");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_FORMS));
    }

    #[test]
    fn sandbox_multiple_tokens() {
        let flags = parse_sandbox_attribute("allow-scripts allow-same-origin allow-forms");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_POPUPS));
    }

    #[test]
    fn sandbox_unrecognized_tokens_ignored() {
        let flags = parse_sandbox_attribute("allow-scripts unknown-token allow-forms");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
    }

    #[test]
    fn sandbox_all_flags() {
        let flags = parse_sandbox_attribute(
            "allow-scripts allow-same-origin allow-forms allow-popups allow-top-navigation allow-modals",
        );
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_POPUPS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_MODALS));
    }
}
