//! Cookie management with `Set-Cookie` parsing and `SameSite` enforcement.
//!
//! Stores cookies per domain/path and applies them to outgoing requests.
//! Defaults to `SameSite=Lax` when the attribute is not specified.

use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// Maximum cookies per domain (Chromium uses 180).
const MAX_COOKIES_PER_DOMAIN: usize = 180;

/// Maximum total cookies across all domains (RFC 6265 recommends >= 3000).
const MAX_TOTAL_COOKIES: usize = 3000;

/// A stored cookie with metadata.
#[derive(Clone, Debug)]
#[allow(dead_code)] // http_only and same_site are stored for M2-7 cross-site filtering
struct StoredCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    expires: Option<SystemTime>,
}

/// `SameSite` cookie attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SameSite {
    Strict,
    Lax,
    None,
}

/// Cookie jar for storing and retrieving cookies.
///
/// Thread-safe via internal `Mutex`.
pub struct CookieJar {
    cookies: Mutex<Vec<StoredCookie>>,
}

impl std::fmt::Debug for CookieJar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        f.debug_struct("CookieJar").field("count", &count).finish()
    }
}

impl CookieJar {
    /// Create an empty cookie jar.
    pub fn new() -> Self {
        Self {
            cookies: Mutex::new(Vec::new()),
        }
    }

    /// Store cookies from `Set-Cookie` response headers.
    pub fn store_from_response(&self, request_url: &url::Url, headers: &[(String, String)]) {
        let Some(request_domain) = request_url.host_str() else {
            return;
        };
        let request_path = request_url.path();

        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        for (name, value) in headers {
            if !name.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            if let Some(cookie) = parse_set_cookie(value, request_domain, request_path) {
                let domain = cookie.domain.clone();
                // Remove existing cookie with same name + domain + path
                jar.retain(|c| {
                    !(c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path)
                });
                jar.push(cookie);

                // Enforce per-domain limit: evict oldest cookies for this domain
                let domain_count = jar.iter().filter(|c| c.domain == domain).count();
                if domain_count > MAX_COOKIES_PER_DOMAIN {
                    let excess = domain_count - MAX_COOKIES_PER_DOMAIN;
                    let mut removed = 0;
                    jar.retain(|c| {
                        if removed < excess && c.domain == domain {
                            removed += 1;
                            return false;
                        }
                        true
                    });
                }

                // Enforce global limit: evict oldest cookies overall
                if jar.len() > MAX_TOTAL_COOKIES {
                    let excess = jar.len() - MAX_TOTAL_COOKIES;
                    jar.drain(..excess);
                }
            }
        }
    }

    /// Get cookies applicable to the given URL.
    ///
    /// Returns name-value pairs suitable for the `Cookie` header.
    /// Also removes expired cookies from the jar opportunistically.
    ///
    /// **Note:** This returns all matching cookies including `HttpOnly` ones.
    /// Callers exposing cookies to scripts (e.g. `document.cookie`) must
    /// filter out `HttpOnly` cookies themselves.
    pub fn cookies_for_url(&self, url: &url::Url) -> Vec<(String, String)> {
        let Some(domain) = url.host_str() else {
            return Vec::new();
        };
        let path = url.path();
        let is_secure = url.scheme() == "https";
        let now = SystemTime::now();

        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Opportunistically remove expired cookies
        jar.retain(|c| c.expires.is_none_or(|exp| now <= exp));
        // Expired cookies already removed by retain() above.
        jar.iter()
            .filter(|c| {
                // Check Secure flag
                if c.secure && !is_secure {
                    return false;
                }
                // Check domain match
                if !domain_matches(domain, &c.domain) {
                    return false;
                }
                // Check path match
                if !path_matches(path, &c.path) {
                    return false;
                }
                true
            })
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect()
    }

    /// Remove expired cookies.
    pub fn remove_expired(&self) {
        let now = SystemTime::now();
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        jar.retain(|c| c.expires.is_none_or(|exp| now <= exp));
    }

    /// Clear all cookies.
    pub fn clear(&self) {
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        jar.clear();
    }

    /// Number of stored cookies.
    pub fn len(&self) -> usize {
        let jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        jar.len()
    }

    /// Whether the jar is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if storing a cookie from `cookie_domain` for a request to
    /// `request_domain` would be a third-party cookie.
    ///
    /// # Limitations (M2-1)
    ///
    /// This implementation compares domain strings directly, without
    /// consulting a Public Suffix List (PSL). This means:
    /// - `sub.example.com` and `example.com` are treated as different sites
    /// - Country-code TLDs like `example.co.jp` are not correctly scoped
    ///
    /// # TODO(M2-7): Full eTLD+1 comparison
    ///
    /// Integrate `publicsuffix` crate (or similar) to extract registrable
    /// domains (eTLD+1) for accurate same-site determination. This is
    /// required for correct third-party cookie blocking on real-world sites.
    /// See: <https://publicsuffix.org/>
    pub fn is_third_party(request_domain: &str, cookie_domain: &str) -> bool {
        !is_same_site(request_domain, cookie_domain)
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a `Set-Cookie` header value into a `StoredCookie`.
///
/// The `cookie` crate handles parsing and rejects malformed values.
/// CRLF injection in cookie names/values is not a concern here because
/// outgoing `Cookie` headers are constructed by joining stored name=value
/// pairs, and hyper validates header values before sending.
fn parse_set_cookie(
    header_value: &str,
    request_domain: &str,
    request_path: &str,
) -> Option<StoredCookie> {
    let cookie = cookie::Cookie::parse(header_value).ok()?;

    let name = cookie.name().to_string();
    let value = cookie.value().to_string();

    // Domain: use cookie's domain attribute or default to request domain
    let domain = match cookie.domain() {
        Some(d) => d.strip_prefix('.').unwrap_or(d).to_ascii_lowercase(),
        None => request_domain.to_ascii_lowercase(),
    };

    // Path: use cookie's path attribute or derive from request path
    let path = match cookie.path() {
        Some(p) => p.to_string(),
        None => default_path(request_path),
    };

    let secure = cookie.secure().unwrap_or(false);
    let http_only = cookie.http_only().unwrap_or(false);

    // SameSite: default to Lax per modern browser behavior
    let same_site = match cookie.same_site() {
        Some(cookie::SameSite::Strict) => SameSite::Strict,
        Some(cookie::SameSite::None) => {
            // SameSite=None requires Secure (Chrome 80+, Firefox 69+)
            if !secure {
                return None;
            }
            SameSite::None
        }
        _ => SameSite::Lax, // default
    };

    // Expiry: Max-Age takes priority over Expires
    let expires = if let Some(max_age) = cookie.max_age() {
        // cookie::time::Duration → std::time::Duration
        let secs = max_age.whole_seconds();
        if secs <= 0 {
            // Max-Age=0 means delete immediately
            return None;
        }
        #[allow(clippy::cast_sign_loss)] // secs > 0 checked above
        Some(SystemTime::now() + Duration::from_secs(secs as u64))
    } else if let Some(exp) = cookie.expires_datetime() {
        // Convert cookie::time::OffsetDateTime to SystemTime
        let unix_ts = exp.unix_timestamp();
        if unix_ts <= 0 {
            return None;
        }
        #[allow(clippy::cast_sign_loss)] // unix_ts > 0 checked above
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(unix_ts as u64))
    } else {
        None // session cookie
    };

    // Reject single-label domains (public suffixes like "com", "org", "net").
    // A proper implementation would use a Public Suffix List, but rejecting
    // domains with no dots catches the most obvious cases.
    let bare_domain = domain.strip_prefix('.').unwrap_or(&domain);
    if !bare_domain.contains('.') {
        return None;
    }

    // Third-party cookie blocking (simplified)
    if !is_same_site(request_domain, &domain) {
        return None;
    }

    Some(StoredCookie {
        name,
        value,
        domain,
        path,
        secure,
        http_only,
        same_site,
        expires,
    })
}

/// Default cookie path from request URI path (RFC 6265 §5.1.4).
fn default_path(request_path: &str) -> String {
    if !request_path.starts_with('/') {
        return "/".to_string();
    }
    match request_path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => request_path[..i].to_string(),
    }
}

/// Domain matching (RFC 6265 §5.1.3).
///
/// The cookie domain must match or be a parent of the request domain.
fn domain_matches(request_domain: &str, cookie_domain: &str) -> bool {
    let req = request_domain.to_ascii_lowercase();
    let cookie = cookie_domain.to_ascii_lowercase();

    if req == cookie {
        return true;
    }

    // request_domain must end with ".cookie_domain"
    req.strip_suffix(cookie.as_str())
        .is_some_and(|prefix| prefix.ends_with('.'))
}

/// Path matching (RFC 6265 §5.1.4).
fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    if request_path == cookie_path {
        return true;
    }
    if request_path.starts_with(cookie_path) {
        // cookie_path ends with '/' or request_path has '/' after cookie_path
        if cookie_path.ends_with('/') {
            return true;
        }
        if request_path.as_bytes().get(cookie_path.len()) == Some(&b'/') {
            return true;
        }
    }
    false
}

/// Simplified same-site check (M2-1).
///
/// The request domain must be the same as or a subdomain of the cookie domain
/// (one-directional matching per RFC 6265 §5.3 step 6).
///
/// See [`CookieJar::is_third_party`] docs for known limitations.
fn is_same_site(request_domain: &str, cookie_domain: &str) -> bool {
    let req = request_domain.to_ascii_lowercase();
    let cookie = cookie_domain.to_ascii_lowercase();
    if req == cookie {
        return true;
    }
    // Check if req ends with ".{cookie}" without allocating a format string
    req.strip_suffix(cookie.as_str())
        .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_jar() -> CookieJar {
        CookieJar::new()
    }

    #[test]
    fn store_and_retrieve_cookie() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/path").unwrap();
        jar.store_from_response(
            &url,
            &[("Set-Cookie".to_string(), "name=value".to_string())],
        );
        let cookies = jar.cookies_for_url(&url);
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0], ("name".to_string(), "value".to_string()));
    }

    #[test]
    fn domain_matching_subdomain() {
        let jar = make_jar();
        let url = url::Url::parse("https://www.example.com/").unwrap();
        jar.store_from_response(
            &url,
            &[(
                "Set-Cookie".to_string(),
                "sid=abc; Domain=example.com".to_string(),
            )],
        );

        // Should match both the parent and subdomain
        let cookies = jar.cookies_for_url(&url::Url::parse("https://www.example.com/").unwrap());
        assert_eq!(cookies.len(), 1);

        let cookies = jar.cookies_for_url(&url::Url::parse("https://example.com/").unwrap());
        assert_eq!(cookies.len(), 1);

        // Should not match other domains
        let cookies = jar.cookies_for_url(&url::Url::parse("https://other.com/").unwrap());
        assert!(cookies.is_empty());
    }

    #[test]
    fn path_matching_basic() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/a/b").unwrap();
        jar.store_from_response(
            &url,
            &[("Set-Cookie".to_string(), "k=v; Path=/a".to_string())],
        );

        let cookies = jar.cookies_for_url(&url::Url::parse("https://example.com/a/b").unwrap());
        assert_eq!(cookies.len(), 1);

        let cookies = jar.cookies_for_url(&url::Url::parse("https://example.com/a").unwrap());
        assert_eq!(cookies.len(), 1);

        let cookies = jar.cookies_for_url(&url::Url::parse("https://example.com/b").unwrap());
        assert!(cookies.is_empty());
    }

    #[test]
    fn secure_cookie_not_sent_over_http() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/").unwrap();
        jar.store_from_response(
            &url,
            &[("Set-Cookie".to_string(), "s=1; Secure".to_string())],
        );

        let cookies = jar.cookies_for_url(&url::Url::parse("https://example.com/").unwrap());
        assert_eq!(cookies.len(), 1);

        let cookies = jar.cookies_for_url(&url::Url::parse("http://example.com/").unwrap());
        assert!(cookies.is_empty());
    }

    #[test]
    fn same_site_default_lax() {
        // Cookie without SameSite attribute should default to Lax
        let cookie = parse_set_cookie("k=v", "example.com", "/");
        assert!(cookie.is_some());
        assert_eq!(cookie.unwrap().same_site, SameSite::Lax);
    }

    #[test]
    fn same_site_none_requires_secure() {
        // SameSite=None without Secure should be rejected
        let result = parse_set_cookie("k=v; SameSite=None", "example.com", "/");
        assert!(result.is_none());

        // SameSite=None with Secure should be accepted
        let result = parse_set_cookie("k=v; SameSite=None; Secure", "example.com", "/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().same_site, SameSite::None);
    }

    #[test]
    fn max_age_zero_deletes() {
        let result = parse_set_cookie("k=v; Max-Age=0", "example.com", "/");
        assert!(result.is_none());
    }

    #[test]
    fn max_age_sets_expiry() {
        let cookie = parse_set_cookie("k=v; Max-Age=3600", "example.com", "/").unwrap();
        assert!(cookie.expires.is_some());
    }

    #[test]
    fn remove_expired() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/").unwrap();
        // Set a cookie that expires in the past (Max-Age=1)
        jar.store_from_response(
            &url,
            &[("Set-Cookie".to_string(), "k=v; Max-Age=1".to_string())],
        );
        assert_eq!(jar.len(), 1);
        // We can't easily wait for it to expire in tests, but verify
        // remove_expired doesn't panic
        jar.remove_expired();
    }

    #[test]
    fn clear_all_cookies() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/").unwrap();
        jar.store_from_response(&url, &[("Set-Cookie".to_string(), "a=1".to_string())]);
        jar.store_from_response(&url, &[("Set-Cookie".to_string(), "b=2".to_string())]);
        assert_eq!(jar.len(), 2);
        jar.clear();
        assert!(jar.is_empty());
    }

    #[test]
    fn third_party_cookie_blocked() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/").unwrap();
        jar.store_from_response(
            &url,
            &[(
                "Set-Cookie".to_string(),
                "t=1; Domain=tracker.com".to_string(),
            )],
        );
        assert!(jar.is_empty(), "third-party cookie should be rejected");
    }

    #[test]
    fn is_third_party_check() {
        assert!(!CookieJar::is_third_party("example.com", "example.com"));
        assert!(!CookieJar::is_third_party("www.example.com", "example.com"));
        assert!(CookieJar::is_third_party("example.com", "tracker.com"));
    }

    #[test]
    fn default_path_derivation() {
        assert_eq!(default_path("/a/b/c"), "/a/b");
        assert_eq!(default_path("/a"), "/");
        assert_eq!(default_path("/"), "/");
        assert_eq!(default_path(""), "/");
    }

    #[test]
    fn domain_match_fn() {
        assert!(domain_matches("example.com", "example.com"));
        assert!(domain_matches("www.example.com", "example.com"));
        assert!(!domain_matches("example.com", "other.com"));
        assert!(!domain_matches("notexample.com", "example.com"));
    }

    #[test]
    fn domain_match_psl_limitation() {
        // Without a Public Suffix List, domain_matches treats "com" as a
        // valid cookie domain. A PSL-aware implementation (M2-7) would
        // reject this because "com" is a public suffix.
        assert!(domain_matches("example.com", "com"));
    }

    #[test]
    fn path_match_fn() {
        assert!(path_matches("/a/b", "/a"));
        assert!(path_matches("/a", "/a"));
        assert!(path_matches("/a/b/c", "/a/"));
        assert!(!path_matches("/abc", "/a/b"));
    }

    #[test]
    fn per_domain_cookie_limit() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/").unwrap();
        // Insert more than MAX_COOKIES_PER_DOMAIN cookies
        let headers: Vec<(String, String)> = (0..=MAX_COOKIES_PER_DOMAIN)
            .map(|i| ("Set-Cookie".to_string(), format!("k{i}=v{i}")))
            .collect();
        jar.store_from_response(&url, &headers);
        assert_eq!(jar.len(), MAX_COOKIES_PER_DOMAIN);
    }

    #[test]
    fn single_label_domain_rejected() {
        // Cookies with single-label domains (public suffixes like "com")
        // should be rejected.
        let result = parse_set_cookie("k=v; Domain=com", "example.com", "/");
        assert!(
            result.is_none(),
            "single-label domain 'com' should be rejected"
        );

        let result = parse_set_cookie("k=v; Domain=org", "example.org", "/");
        assert!(
            result.is_none(),
            "single-label domain 'org' should be rejected"
        );

        // Multi-label domains should still work
        let result = parse_set_cookie("k=v; Domain=example.com", "example.com", "/");
        assert!(result.is_some(), "multi-label domain should be accepted");
    }

    #[test]
    fn duplicate_cookie_replaced() {
        let jar = make_jar();
        let url = url::Url::parse("https://example.com/").unwrap();
        jar.store_from_response(&url, &[("Set-Cookie".to_string(), "k=1".to_string())]);
        jar.store_from_response(&url, &[("Set-Cookie".to_string(), "k=2".to_string())]);
        assert_eq!(jar.len(), 1);
        let cookies = jar.cookies_for_url(&url);
        assert_eq!(cookies[0].1, "2");
    }
}
