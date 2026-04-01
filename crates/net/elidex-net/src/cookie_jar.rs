//! Cookie management with `Set-Cookie` parsing and `SameSite` enforcement.
//!
//! Stores cookies per domain/path and applies them to outgoing requests.
//! Defaults to `SameSite=Lax` when the attribute is not specified.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use psl::Psl;

/// Maximum cookies per domain (Chromium uses 180).
const MAX_COOKIES_PER_DOMAIN: usize = 180;

/// Maximum total cookies across all domains (RFC 6265 recommends >= 3000).
const MAX_TOTAL_COOKIES: usize = 3000;

/// A stored cookie with metadata (RFC 6265 §5.7).
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
struct StoredCookie {
    name: String,
    value: String,
    /// The domain the cookie is scoped to.
    domain: String,
    /// The original request host (for host-only matching).
    host: String,
    path: String,
    /// True if Domain attribute was absent (exact host match only).
    host_only: bool,
    /// True if Max-Age/Expires was present (survives session end).
    persistent: bool,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    expires: Option<SystemTime>,
    /// CHIPS partition key (empty = first-party).
    partition_key: String,
    creation_time: SystemTime,
    last_access_time: SystemTime,
}

/// `SameSite` cookie attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SameSite {
    Strict,
    Lax,
    None,
}

/// Public cookie data for persistence sync.
///
/// `StoredCookie` is private; this struct is the public interface for
/// loading/snapshotting cookies to/from external persistence.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct CookieSnapshot {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub host: String,
    pub path: String,
    pub partition_key: String,
    pub host_only: bool,
    pub persistent: bool,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
    pub expires: Option<SystemTime>,
    pub creation_time: SystemTime,
    pub last_access_time: SystemTime,
}

/// Cookie jar for storing and retrieving cookies.
///
/// Thread-safe via internal `Mutex`. Supports decoupled persistence
/// via `generation()` / `snapshot()` / `load()` — the Browser Process
/// checks `generation()` periodically and persists changes externally.
pub struct CookieJar {
    cookies: Mutex<Vec<StoredCookie>>,
    generation: AtomicU64,
}

impl std::fmt::Debug for CookieJar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        f.debug_struct("CookieJar")
            .field("count", &count)
            .field("generation", &self.generation.load(Ordering::Relaxed))
            .finish()
    }
}

impl CookieJar {
    /// Create an empty cookie jar.
    pub fn new() -> Self {
        Self {
            cookies: Mutex::new(Vec::new()),
            generation: AtomicU64::new(0),
        }
    }

    /// Monotonic generation counter. Incremented on every mutation.
    /// Compare against a cached value to detect changes.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    /// Bump the generation counter (called after mutations).
    fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot all cookies for external persistence.
    ///
    /// Bounded by `MAX_TOTAL_COOKIES` (3000), so this is cheap.
    pub fn snapshot(&self) -> Vec<CookieSnapshot> {
        let jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        jar.iter().map(stored_to_snapshot).collect()
    }

    /// Bulk-load cookies from external persistence (startup).
    ///
    /// Replaces all current cookies. Does not bump generation (load is
    /// a restore, not a mutation).
    pub fn load(&self, cookies: Vec<CookieSnapshot>) {
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        jar.clear();
        for c in cookies {
            jar.push(StoredCookie {
                name: c.name,
                value: c.value,
                domain: c.domain,
                host: c.host,
                path: c.path,
                partition_key: c.partition_key,
                host_only: c.host_only,
                persistent: c.persistent,
                secure: c.secure,
                http_only: c.http_only,
                same_site: match c.same_site.to_ascii_lowercase().as_str() {
                    "strict" => SameSite::Strict,
                    "none" => SameSite::None,
                    _ => SameSite::Lax,
                },
                expires: c.expires,
                creation_time: c.creation_time,
                last_access_time: c.last_access_time,
            });
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

        let mut mutated = false;
        for (name, value) in headers {
            if !name.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            if let Some(mut cookie) = parse_set_cookie(value, request_domain, request_path) {
                let domain = cookie.domain.clone();
                // RFC 6265bis §5.7 step 23: preserve creation-time of existing cookie.
                if let Some(existing) = jar.iter().find(|c| {
                    c.name == cookie.name
                        && c.domain == cookie.domain
                        && c.path == cookie.path
                        && c.partition_key == cookie.partition_key
                }) {
                    cookie.creation_time = existing.creation_time;
                }
                // Remove existing cookie with same name + domain + path.
                jar.retain(|c| {
                    !(c.name == cookie.name
                        && c.domain == cookie.domain
                        && c.path == cookie.path
                        && c.partition_key == cookie.partition_key)
                });
                jar.push(cookie);
                mutated = true;

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
        if mutated {
            self.bump_generation();
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
        // RFC 6265bis §5.8.3: update last-access-time on retrieval.
        // Use iter_mut to update matching cookies in-place.
        jar.iter_mut()
            .filter(|c| {
                if c.secure && !is_secure {
                    return false;
                }
                if !cookie_domain_matches(c, domain) {
                    return false;
                }
                if !path_matches(path, &c.path) {
                    return false;
                }
                true
            })
            .map(|c| {
                c.last_access_time = now;
                (c.name.clone(), c.value.clone())
            })
            .collect()
    }

    /// Remove expired cookies.
    pub fn remove_expired(&self) {
        let now = SystemTime::now();
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = jar.len();
        jar.retain(|c| c.expires.is_none_or(|exp| now <= exp));
        if jar.len() != before {
            self.bump_generation();
        }
    }

    /// Clear all cookies.
    pub fn clear(&self) {
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !jar.is_empty() {
            jar.clear();
            self.bump_generation();
        }
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

    /// Build a `Cookie` header value for the given URL.
    ///
    /// Returns `None` if no cookies match the URL.
    /// Format: `"name1=value1; name2=value2"`.
    pub fn cookie_header_for_url(&self, url: &url::Url) -> Option<String> {
        let cookies = self.cookies_for_url(url);
        if cookies.is_empty() {
            return None;
        }
        Some(
            cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    /// Check if storing a cookie from `cookie_domain` for a request to
    /// `request_domain` would be a third-party cookie.
    ///
    /// Uses the Mozilla Public Suffix List (via the `psl` crate) to extract
    /// registrable domains (eTLD+1) for accurate same-site determination.
    /// This prevents Cookie Monster attacks where a cookie set on a public
    /// suffix (e.g. `co.jp`) would be sent to all sites under that suffix.
    pub fn is_third_party(request_domain: &str, cookie_domain: &str) -> bool {
        !is_same_site(request_domain, cookie_domain)
    }

    /// Return cookies for script access (`document.cookie` getter).
    ///
    /// Filters out `HttpOnly` cookies (RFC 6265 §5.3) and `Secure` cookies
    /// on non-HTTPS pages. Returns a `"name=value; name2=value2"` string
    /// (empty string when no cookies, never null).
    pub fn cookies_for_script(&self, url: &url::Url) -> String {
        let details = self.cookie_details_for_script(url);
        details
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Return full cookie details for CookieStore API (WHATWG Cookie Store spec).
    ///
    /// Filters out `HttpOnly` cookies and `Secure` cookies on non-HTTPS.
    /// Updates `last_access_time` on matched cookies (RFC 6265bis §5.8.3).
    pub fn cookie_details_for_script(&self, url: &url::Url) -> Vec<CookieSnapshot> {
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
        let before_len = jar.len();
        jar.retain(|c| c.expires.is_none_or(|exp| now <= exp));

        let result: Vec<CookieSnapshot> = jar
            .iter_mut()
            .filter(|c| is_script_visible(c, domain, path, is_secure))
            .map(|c| {
                c.last_access_time = now;
                stored_to_snapshot(c)
            })
            .collect();

        // Only bump generation if expired cookies were actually removed.
        // last_access_time updates are internal bookkeeping, not mutations
        // that require persistence (they'll be captured on the next real mutation).
        if jar.len() != before_len {
            self.bump_generation();
        }
        result
    }

    /// Set a cookie from script (`document.cookie` setter).
    ///
    /// Parses the value as a `Set-Cookie` header. Rejects cookies with
    /// `HttpOnly` attribute (scripts cannot set `HttpOnly` cookies) and
    /// `Secure` cookies on non-HTTPS pages.
    pub fn set_cookie_from_script(&self, url: &url::Url, value: &str) {
        let Some(domain) = url.host_str() else {
            return;
        };
        let is_secure = url.scheme() == "https";
        let path = url.path();

        let Some(mut cookie) = parse_set_cookie(value, domain, path) else {
            return;
        };
        // Reject HttpOnly cookies set from script.
        if cookie.http_only {
            return;
        }
        // Reject Secure cookies on non-HTTPS pages.
        if cookie.secure && !is_secure {
            return;
        }
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // RFC 6265bis §5.7 step 23: preserve creation-time of existing cookie.
        if let Some(existing) = jar.iter().find(|c| {
            c.name == cookie.name
                && c.domain == cookie.domain
                && c.path == cookie.path
                && c.partition_key == cookie.partition_key
        }) {
            cookie.creation_time = existing.creation_time;
        }
        // Remove existing cookie with same name/domain/path.
        jar.retain(|c| {
            !(c.name == cookie.name
                && c.domain == cookie.domain
                && c.path == cookie.path
                && c.partition_key == cookie.partition_key)
        });
        // Evict the oldest cookie (first inserted) if at the global limit.
        if jar.len() >= MAX_TOTAL_COOKIES {
            jar.remove(0);
        }
        jar.push(cookie);
        self.bump_generation();
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a `StoredCookie` to a `CookieSnapshot`.
///
/// Uses lowercase `same_site` values per WHATWG Cookie Store spec.
fn stored_to_snapshot(c: &StoredCookie) -> CookieSnapshot {
    CookieSnapshot {
        name: c.name.clone(),
        value: c.value.clone(),
        domain: c.domain.clone(),
        host: c.host.clone(),
        path: c.path.clone(),
        partition_key: c.partition_key.clone(),
        host_only: c.host_only,
        persistent: c.persistent,
        secure: c.secure,
        http_only: c.http_only,
        same_site: match c.same_site {
            SameSite::Strict => "strict".to_string(),
            SameSite::Lax => "lax".to_string(),
            SameSite::None => "none".to_string(),
        },
        expires: c.expires,
        creation_time: c.creation_time,
        last_access_time: c.last_access_time,
    }
}

/// Check if a cookie is visible to scripts (not HttpOnly, matching domain/path/secure).
fn is_script_visible(c: &StoredCookie, domain: &str, path: &str, is_secure: bool) -> bool {
    if c.http_only {
        return false;
    }
    if c.secure && !is_secure {
        return false;
    }
    if !cookie_domain_matches(c, domain) {
        return false;
    }
    if !path_matches(path, &c.path) {
        return false;
    }
    true
}

/// Check if a cookie's domain matches the request domain,
/// respecting the host-only flag (RFC 6265bis §5.7).
fn cookie_domain_matches(c: &StoredCookie, request_domain: &str) -> bool {
    if c.host_only {
        // Host-only cookies require exact domain match.
        request_domain.eq_ignore_ascii_case(&c.domain)
    } else {
        // Domain cookies use suffix matching.
        domain_matches(request_domain, &c.domain)
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

    // Domain: use cookie's domain attribute or default to request domain.
    // host_only = true when no Domain attribute (RFC 6265 §5.7).
    let host_only = cookie.domain().is_none();
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

    // Reject cookies whose domain is a public suffix (RFC 6265 §5.3 step 5).
    // This prevents Cookie Monster attacks (e.g. setting a cookie on "co.jp"
    // that would be sent to all sites under that TLD).
    if is_public_suffix(&domain) {
        return None;
    }

    // Third-party cookie blocking (simplified)
    if !is_same_site(request_domain, &domain) {
        return None;
    }

    let persistent = expires.is_some();
    let now = SystemTime::now();

    Some(StoredCookie {
        name,
        value,
        domain,
        host: request_domain.to_ascii_lowercase(),
        path,
        host_only,
        persistent,
        secure,
        http_only,
        same_site,
        expires,
        // CHIPS: partition_key requires parsing the `Partitioned` Set-Cookie
        // attribute and computing the top-level site. Not yet implemented —
        // all cookies are first-party (empty partition_key) for now.
        partition_key: String::new(),
        creation_time: now,
        last_access_time: now,
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

/// Same-site check using the Public Suffix List (eTLD+1 comparison).
///
/// Two domains are considered same-site if they share the same registrable
/// domain (eTLD+1). For example, `www.example.co.jp` and `api.example.co.jp`
/// are same-site because both have registrable domain `example.co.jp`.
fn is_same_site(request_domain: &str, cookie_domain: &str) -> bool {
    let req = request_domain.to_ascii_lowercase();
    let cookie = cookie_domain.to_ascii_lowercase();

    // If either domain is a public suffix itself, they can only be same-site
    // if they are identical.
    match (registrable_domain(&req), registrable_domain(&cookie)) {
        (Some(r), Some(c)) => r == c,
        _ => req == cookie,
    }
}

/// Extract the registrable domain (eTLD+1) using the Public Suffix List.
///
/// Returns `None` if the domain is itself a public suffix (e.g. "com", "co.jp")
/// or is not a valid domain.
fn registrable_domain(domain: &str) -> Option<String> {
    let bytes = domain.as_bytes();
    let d = psl::List.domain(bytes)?;
    Some(std::str::from_utf8(d.as_bytes()).ok()?.to_ascii_lowercase())
}

/// Check if a domain is a public suffix (e.g. "com", "co.jp", "github.io").
///
/// Public suffixes must not be used as cookie domains (RFC 6265 §5.3 step 5).
fn is_public_suffix(domain: &str) -> bool {
    let bare = domain.strip_prefix('.').unwrap_or(domain);
    let bytes = bare.as_bytes();
    // If psl::List.domain() returns None, the domain is a public suffix.
    // Also check that the suffix is known (not a wildcard/unknown rule).
    if psl::List.domain(bytes).is_some() {
        return false;
    }
    // It's a public suffix if the suffix lookup matches the whole domain.
    psl::List
        .suffix(bytes)
        .is_some_and(|s: psl::Suffix<'_>| s.as_bytes() == bytes)
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
        assert!(!CookieJar::is_third_party(
            "api.example.com",
            "cdn.example.com"
        ));
        assert!(CookieJar::is_third_party("example.com", "tracker.com"));
        // Different github.io subdomains are different sites (github.io is a public suffix).
        assert!(CookieJar::is_third_party(
            "user1.github.io",
            "user2.github.io"
        ));
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
    fn public_suffix_cookie_rejected() {
        // Cookies set on public suffixes must be rejected (Cookie Monster prevention).
        assert!(is_public_suffix("com"));
        assert!(is_public_suffix("co.jp"));
        assert!(is_public_suffix("github.io"));
        assert!(!is_public_suffix("example.com"));
        assert!(!is_public_suffix("example.co.jp"));
        assert!(!is_public_suffix("user.github.io"));

        // parse_set_cookie rejects public suffix domains.
        let result = parse_set_cookie("k=v; Domain=co.jp", "example.co.jp", "/");
        assert!(result.is_none(), "co.jp is a public suffix");

        let result = parse_set_cookie("k=v; Domain=github.io", "user.github.io", "/");
        assert!(result.is_none(), "github.io is a public suffix");
    }

    #[test]
    fn registrable_domain_extraction() {
        assert_eq!(
            registrable_domain("www.example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            registrable_domain("sub.example.co.jp"),
            Some("example.co.jp".to_string())
        );
        assert_eq!(
            registrable_domain("user.github.io"),
            Some("user.github.io".to_string())
        );
        // Public suffixes have no registrable domain.
        assert_eq!(registrable_domain("com"), None);
        assert_eq!(registrable_domain("co.jp"), None);
    }

    #[test]
    fn same_site_etld1() {
        // Same registrable domain = same site.
        assert!(is_same_site("www.example.com", "example.com"));
        assert!(is_same_site("api.example.com", "cdn.example.com"));
        assert!(is_same_site("sub.example.co.jp", "example.co.jp"));

        // Different registrable domain = different site.
        assert!(!is_same_site("example.com", "other.com"));
        assert!(!is_same_site("user1.github.io", "user2.github.io"));
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
    fn public_suffix_domain_rejected_in_set_cookie() {
        // Cookies with public suffix domains should be rejected.
        let result = parse_set_cookie("k=v; Domain=com", "example.com", "/");
        assert!(result.is_none(), "TLD 'com' should be rejected");

        let result = parse_set_cookie("k=v; Domain=org", "example.org", "/");
        assert!(result.is_none(), "TLD 'org' should be rejected");

        let result = parse_set_cookie("k=v; Domain=co.uk", "example.co.uk", "/");
        assert!(result.is_none(), "ccTLD 'co.uk' should be rejected");

        // Registrable domains should still work.
        let result = parse_set_cookie("k=v; Domain=example.com", "example.com", "/");
        assert!(result.is_some(), "registrable domain should be accepted");

        let result = parse_set_cookie("k=v; Domain=example.co.jp", "example.co.jp", "/");
        assert!(
            result.is_some(),
            "registrable ccTLD domain should be accepted"
        );
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
