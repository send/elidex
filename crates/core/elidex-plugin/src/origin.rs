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
/// Distinct from `elidex_css::Origin` (cascade origin: UserAgent/Author).
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
// CSP frame-ancestors (W3C CSP Level 3 §7.7.3)
// ---------------------------------------------------------------------------

/// A source in a CSP `frame-ancestors` directive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameAncestorSource {
    /// `'self'` — matches the document's own origin.
    SelfOrigin,
    /// `*` — matches all origins.
    All,
    /// Host source (e.g. `"example.com"`, `"*.example.com"`).
    Host(String),
    /// Scheme source (e.g. `"https"`). Stored without trailing colon.
    Scheme(String),
}

/// Parsed CSP `frame-ancestors` directive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameAncestorsPolicy {
    /// `frame-ancestors 'none'` — deny all framing.
    Deny,
    /// `frame-ancestors <source-list>` — allow framing from listed sources.
    AllowList(Vec<FrameAncestorSource>),
}

/// Parse the CSP `frame-ancestors` directive from a `Content-Security-Policy` header.
///
/// Extracts the `frame-ancestors` directive value and parses its source list.
/// Returns `None` if the header doesn't contain a `frame-ancestors` directive.
#[must_use]
pub fn parse_frame_ancestors(csp_header: &str) -> Option<FrameAncestorsPolicy> {
    // CSP directives are `;`-separated.
    // Directive names are case-insensitive per W3C CSP L3 §2.1.
    for directive in csp_header.split(';') {
        let trimmed = directive.trim();
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("frame-ancestors") {
            // Verify word boundary: next char must be whitespace or end-of-string.
            // This prevents matching "frame-ancestors-foo" as "frame-ancestors".
            if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let value = rest.trim();
            if value.is_empty() {
                // Whitespace-only value = no directive per W3C CSP L3.
                // Fall through to X-Frame-Options check.
                return None;
            }
            if value == "'none'" {
                return Some(FrameAncestorsPolicy::Deny);
            }
            let mut sources = Vec::new();
            for token in value.split_ascii_whitespace() {
                if token == "'self'" {
                    sources.push(FrameAncestorSource::SelfOrigin);
                } else if token == "*" {
                    sources.push(FrameAncestorSource::All);
                } else if token.ends_with(':') {
                    // Strip trailing colon: "https:" → "https"
                    sources.push(FrameAncestorSource::Scheme(
                        token.trim_end_matches(':').to_string(),
                    ));
                } else {
                    sources.push(FrameAncestorSource::Host(token.to_string()));
                }
            }
            return Some(FrameAncestorsPolicy::AllowList(sources));
        }
    }
    None
}

/// Check whether a parent origin is allowed to frame a document
/// according to the document's `frame-ancestors` policy.
///
/// Returns `true` if framing is allowed, `false` if blocked.
#[must_use]
pub fn is_framing_allowed(
    policy: &FrameAncestorsPolicy,
    parent_origin: &SecurityOrigin,
    document_origin: &SecurityOrigin,
) -> bool {
    match policy {
        FrameAncestorsPolicy::Deny => false,
        FrameAncestorsPolicy::AllowList(sources) => {
            for source in sources {
                match source {
                    FrameAncestorSource::SelfOrigin => {
                        if parent_origin == document_origin {
                            return true;
                        }
                    }
                    FrameAncestorSource::All => {
                        return true;
                    }
                    FrameAncestorSource::Host(pattern) => {
                        if let SecurityOrigin::Tuple {
                            scheme, host, port, ..
                        } = parent_origin
                        {
                            // CSP host-source may include scheme: "https://example.com"
                            // or be bare: "example.com" or "*.example.com".
                            let (pattern_scheme, pattern_host) =
                                if let Some(rest) = pattern.strip_prefix("https://") {
                                    (Some("https"), rest)
                                } else if let Some(rest) = pattern.strip_prefix("http://") {
                                    (Some("http"), rest)
                                } else {
                                    (None, pattern.as_str())
                                };

                            // Scheme check (if specified in pattern).
                            if let Some(ps) = pattern_scheme {
                                if scheme != ps {
                                    continue;
                                }
                            }

                            // Host check with wildcard support.
                            // Strip port from pattern if present.
                            let (ph, pp) = pattern_host
                                .rsplit_once(':')
                                .map_or((pattern_host, None), |(h, p)| (h, p.parse::<u16>().ok()));

                            if let Some(expected_port) = pp {
                                if *port != expected_port {
                                    continue;
                                }
                            }

                            if let Some(_domain) = ph.strip_prefix("*.") {
                                // W3C CSP L3: *.example.com matches sub.example.com
                                // but NOT example.com itself (apex domain).
                                let suffix = &ph[1..]; // ".example.com"
                                if host.ends_with(suffix) {
                                    return true;
                                }
                            } else if host == ph {
                                return true;
                            }
                        }
                    }
                    FrameAncestorSource::Scheme(scheme_source) => {
                        if let SecurityOrigin::Tuple { scheme, .. } = parent_origin {
                            if scheme == scheme_source {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
    }
}

/// Check `X-Frame-Options` header value (RFC 7034).
///
/// Returns `true` if framing is allowed, `false` if blocked.
/// Only checks `DENY` and `SAMEORIGIN` values.
#[must_use]
pub fn check_x_frame_options(
    header_value: &str,
    parent_origin: &SecurityOrigin,
    document_origin: &SecurityOrigin,
) -> bool {
    // Handle comma-separated values and multiple tokens (e.g. "DENY, SAMEORIGIN").
    // Most restrictive value wins: DENY > SAMEORIGIN > unknown/allow.
    let mut allowed = true; // Default: allow framing.
    for token in header_value.split(',') {
        let t = token.trim();
        if t.eq_ignore_ascii_case("DENY") {
            return false; // Most restrictive — block immediately.
        } else if t.eq_ignore_ascii_case("SAMEORIGIN") {
            allowed = parent_origin == document_origin;
        }
        // Unknown tokens ignored per RFC 7034.
    }
    allowed
}

// ---------------------------------------------------------------------------
// Permissions-Policy framework (WHATWG §6.9)
// ---------------------------------------------------------------------------

/// Allow-list for a Permissions-Policy feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllowList {
    /// `*` — allow all origins.
    All,
    /// `()` — deny all origins.
    None,
    /// `(self)` — allow only the document's own origin.
    SelfOnly,
    /// Explicit origin list.
    Origins(Vec<SecurityOrigin>),
}

/// Permissions-Policy for a document (WHATWG HTML §6.9, 08-security-model.md §8.4).
///
/// Framework type only — actual enforcement is deferred to individual Web API
/// implementations that call [`PermissionsPolicy::is_feature_allowed`].
#[derive(Clone, Debug, Default)]
pub struct PermissionsPolicy {
    /// Feature name → allow-list mapping.
    policies: std::collections::HashMap<String, AllowList>,
}

impl PermissionsPolicy {
    /// Create an empty policy (all features use default behavior).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the allow-list for a feature.
    pub fn set_feature(&mut self, feature: impl Into<String>, allow_list: AllowList) {
        self.policies.insert(feature.into(), allow_list);
    }

    /// Check whether a feature is allowed for a given origin.
    ///
    /// `origin` is the origin requesting the feature.
    /// `document_origin` is the document's own origin (used for `SelfOnly` check).
    /// If no policy is set for the feature, returns `true` (default allow).
    #[must_use]
    pub fn is_feature_allowed(
        &self,
        feature: &str,
        origin: &SecurityOrigin,
        document_origin: &SecurityOrigin,
    ) -> bool {
        let Some(allow_list) = self.policies.get(feature) else {
            return true; // No policy = default allow.
        };
        match allow_list {
            AllowList::All => true,
            AllowList::None => false,
            AllowList::SelfOnly => origin == document_origin,
            AllowList::Origins(origins) => origins.iter().any(|o| o == origin),
        }
    }
}

/// Parse the `<iframe allow>` attribute value into a `PermissionsPolicy`.
///
/// Format: `"camera; fullscreen 'src'"` → feature names with optional allowlist.
/// Simplified: each semicolon-separated feature is allowed for `SelfOnly`.
#[must_use]
pub fn parse_iframe_allow_attribute(value: &str) -> PermissionsPolicy {
    let mut policy = PermissionsPolicy::new();
    for token in value.split(';') {
        let feature = token.trim().split_ascii_whitespace().next();
        if let Some(name) = feature {
            if !name.is_empty() {
                policy.set_feature(name, AllowList::SelfOnly);
            }
        }
    }
    policy
}

/// Hex-encode a string for use as a filesystem-safe directory name.
///
/// Encodes each byte as two hex digits, preventing path traversal via
/// characters like `/`, `..`, `:` in origin strings.
///
/// Used by `elidex-indexeddb` for per-origin DB paths and by
/// `elidex-shell` `QuotaManager` for eviction paths.
pub fn hex_encode_for_path(value: &str) -> String {
    value
        .bytes()
        .fold(String::with_capacity(value.len() * 2), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
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
        assert!(a == b);
    }

    #[test]
    fn cross_origin_different_scheme() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(a != b);
    }

    #[test]
    fn cross_origin_different_port() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("http://example.com:8080").unwrap());
        assert!(a != b);
    }

    #[test]
    fn cross_origin_different_host() {
        let a = SecurityOrigin::from_url(&url::Url::parse("https://a.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("https://b.com").unwrap());
        assert!(a != b);
    }

    #[test]
    fn default_port_normalization() {
        let a = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        let b = SecurityOrigin::from_url(&url::Url::parse("http://example.com:80").unwrap());
        assert!(a == b);

        let c = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let d = SecurityOrigin::from_url(&url::Url::parse("https://example.com:443").unwrap());
        assert!(c == d);
    }

    #[test]
    fn opaque_origins_same_id_matches() {
        let a = SecurityOrigin::opaque();
        let b = SecurityOrigin::opaque();
        // Different opaque origins (different IDs) are not same-origin.
        assert!(a != b);
        // Same opaque origin (same ID) IS same-origin.
        assert!(a == a);
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

    // --- CSP frame-ancestors tests ---

    #[test]
    fn frame_ancestors_none() {
        let policy = parse_frame_ancestors("frame-ancestors 'none'").unwrap();
        assert_eq!(policy, FrameAncestorsPolicy::Deny);

        let parent = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(!is_framing_allowed(&policy, &parent, &doc));
    }

    #[test]
    fn frame_ancestors_self() {
        let policy = parse_frame_ancestors("frame-ancestors 'self'").unwrap();
        let same = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(is_framing_allowed(&policy, &same, &doc));

        let cross = SecurityOrigin::from_url(&url::Url::parse("https://evil.com").unwrap());
        assert!(!is_framing_allowed(&policy, &cross, &doc));
    }

    #[test]
    fn frame_ancestors_host() {
        let csp = "default-src 'self'; frame-ancestors https://trusted.com *.example.com";
        let policy = parse_frame_ancestors(csp).unwrap();

        let trusted = SecurityOrigin::from_url(&url::Url::parse("https://trusted.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://target.com").unwrap());
        assert!(is_framing_allowed(&policy, &trusted, &doc));

        let sub = SecurityOrigin::from_url(&url::Url::parse("https://sub.example.com").unwrap());
        assert!(is_framing_allowed(&policy, &sub, &doc));

        let evil = SecurityOrigin::from_url(&url::Url::parse("https://evil.com").unwrap());
        assert!(!is_framing_allowed(&policy, &evil, &doc));
    }

    #[test]
    fn frame_ancestors_wildcard_does_not_match_apex() {
        // W3C CSP L3: *.example.com must NOT match example.com (apex domain).
        let csp = "frame-ancestors *.example.com";
        let policy = parse_frame_ancestors(csp).unwrap();
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://target.com").unwrap());

        let apex = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(
            !is_framing_allowed(&policy, &apex, &doc),
            "apex must NOT match"
        );

        let sub = SecurityOrigin::from_url(&url::Url::parse("https://sub.example.com").unwrap());
        assert!(
            is_framing_allowed(&policy, &sub, &doc),
            "subdomain must match"
        );
    }

    #[test]
    fn frame_ancestors_case_insensitive() {
        // CSP directive names are case-insensitive per W3C CSP L3 §2.1.
        let policy = parse_frame_ancestors("Frame-Ancestors 'self'").unwrap();
        let same = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(is_framing_allowed(&policy, &same, &doc));

        let upper = parse_frame_ancestors("FRAME-ANCESTORS 'none'").unwrap();
        assert_eq!(upper, FrameAncestorsPolicy::Deny);
    }

    #[test]
    fn frame_ancestors_not_present() {
        let policy = parse_frame_ancestors("default-src 'self'");
        assert!(policy.is_none());
    }

    #[test]
    fn x_frame_options_deny() {
        let parent = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(!check_x_frame_options("DENY", &parent, &doc));
    }

    #[test]
    fn x_frame_options_sameorigin() {
        let same = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(check_x_frame_options("SAMEORIGIN", &same, &doc));

        let cross = SecurityOrigin::from_url(&url::Url::parse("https://other.com").unwrap());
        assert!(!check_x_frame_options("SAMEORIGIN", &cross, &doc));
    }

    // --- Permissions-Policy tests ---

    #[test]
    fn permissions_policy_default_allow() {
        let policy = PermissionsPolicy::new();
        let origin = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(policy.is_feature_allowed("camera", &origin, &origin));
    }

    #[test]
    fn permissions_policy_deny_feature() {
        let mut policy = PermissionsPolicy::new();
        policy.set_feature("camera", AllowList::None);
        let origin = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(!policy.is_feature_allowed("camera", &origin, &origin));
        assert!(policy.is_feature_allowed("fullscreen", &origin, &origin)); // Not set = allow.
    }

    #[test]
    fn parse_iframe_allow() {
        let policy = parse_iframe_allow_attribute("camera; fullscreen");
        let origin = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let other = SecurityOrigin::from_url(&url::Url::parse("https://other.com").unwrap());
        // SelfOnly allows when origin matches document origin.
        assert!(policy.is_feature_allowed("camera", &origin, &origin));
        // SelfOnly denies when origin differs from document origin.
        assert!(!policy.is_feature_allowed("camera", &other, &origin));
        // Features not in the policy default to allow.
        assert!(policy.is_feature_allowed("geolocation", &origin, &origin));
    }

    #[test]
    fn x_frame_options_comma_separated() {
        let parent = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        // DENY wins over SAMEORIGIN
        assert!(!check_x_frame_options("DENY, SAMEORIGIN", &parent, &doc));
    }

    #[test]
    fn x_frame_options_case_insensitive() {
        let parent = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(!check_x_frame_options("deny", &parent, &doc));
        assert!(check_x_frame_options("sameorigin", &parent, &doc));
    }

    #[test]
    fn x_frame_options_unknown_allows() {
        let parent = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(check_x_frame_options("ALLOWALL", &parent, &doc));
    }

    #[test]
    fn frame_ancestors_star_allows_all() {
        let policy = parse_frame_ancestors("frame-ancestors *").unwrap();
        let parent = SecurityOrigin::from_url(&url::Url::parse("https://evil.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://target.com").unwrap());
        assert!(is_framing_allowed(&policy, &parent, &doc));
    }

    #[test]
    fn frame_ancestors_scheme_source() {
        let policy = parse_frame_ancestors("frame-ancestors https:").unwrap();
        let secure = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://target.com").unwrap());
        assert!(is_framing_allowed(&policy, &secure, &doc));

        let insecure = SecurityOrigin::from_url(&url::Url::parse("http://example.com").unwrap());
        assert!(!is_framing_allowed(&policy, &insecure, &doc));
    }

    #[test]
    fn frame_ancestors_host_with_port() {
        let policy = parse_frame_ancestors("frame-ancestors example.com:8080").unwrap();
        let matching = SecurityOrigin::Tuple {
            scheme: "https".into(),
            host: "example.com".into(),
            port: 8080,
        };
        let doc = SecurityOrigin::from_url(&url::Url::parse("https://target.com").unwrap());
        assert!(is_framing_allowed(&policy, &matching, &doc));

        let wrong_port = SecurityOrigin::Tuple {
            scheme: "https".into(),
            host: "example.com".into(),
            port: 443,
        };
        assert!(!is_framing_allowed(&policy, &wrong_port, &doc));
    }

    #[test]
    fn frame_ancestors_word_boundary() {
        // "frame-ancestors-foo" should NOT match as "frame-ancestors"
        let policy = parse_frame_ancestors("frame-ancestors-foo 'none'");
        assert!(policy.is_none());
    }

    #[test]
    fn permissions_policy_self_only_allows_own_origin() {
        let mut policy = PermissionsPolicy::new();
        policy.set_feature("camera", AllowList::SelfOnly);
        let origin = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        // SelfOnly allows when origin == document origin
        assert!(policy.is_feature_allowed("camera", &origin, &origin));
        let other = SecurityOrigin::from_url(&url::Url::parse("https://other.com").unwrap());
        assert!(!policy.is_feature_allowed("camera", &other, &origin));
    }

    #[test]
    fn permissions_policy_all_allows_any() {
        let mut policy = PermissionsPolicy::new();
        policy.set_feature("fullscreen", AllowList::All);
        let origin = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        let doc_origin = SecurityOrigin::from_url(&url::Url::parse("https://other.com").unwrap());
        assert!(policy.is_feature_allowed("fullscreen", &origin, &doc_origin));
    }

    #[test]
    fn permissions_policy_origins_list() {
        let mut policy = PermissionsPolicy::new();
        let allowed = SecurityOrigin::from_url(&url::Url::parse("https://trusted.com").unwrap());
        policy.set_feature("camera", AllowList::Origins(vec![allowed.clone()]));
        let doc_origin = SecurityOrigin::from_url(&url::Url::parse("https://example.com").unwrap());
        assert!(policy.is_feature_allowed("camera", &allowed, &doc_origin));
        let other = SecurityOrigin::from_url(&url::Url::parse("https://other.com").unwrap());
        assert!(!policy.is_feature_allowed("camera", &other, &doc_origin));
    }
}
