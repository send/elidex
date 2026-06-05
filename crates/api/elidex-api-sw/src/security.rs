//! Service Worker security validation (WHATWG SW §4.2).

use url::Url;

/// Allowed JavaScript MIME types for SW scripts.
const ALLOWED_MIME_TYPES: &[&str] = &[
    "text/javascript",
    "application/javascript",
    "application/x-javascript",
    "application/ecmascript",
    "text/ecmascript",
];

/// A typed Service Worker registration failure (WHATWG SW §3.1).
///
/// The variant is the JS exception the failure surfaces as, so an
/// engine-bound binding can map it 1:1 onto a `TypeError` /
/// `SecurityError` `DOMException` without re-deriving *which* rule was
/// violated (the whole origin / scheme / scope-path / secure-context
/// decision stays in this crate — the binding only marshals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwRegisterError {
    /// §3.1 "Start Register" — the script or scope URL does not use an
    /// `http`/`https` scheme.  Surfaces as a `TypeError`.
    TypeError(String),
    /// §3.1 "Register"/"Update" — cross-origin script/scope, a non-secure
    /// context, or a scope outside the script directory.  Surfaces as a
    /// `SecurityError` `DOMException`.
    SecurityError(String),
}

impl SwRegisterError {
    /// The human-readable failure message (both variants carry one).
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::TypeError(m) | Self::SecurityError(m) => m,
        }
    }
}

impl std::fmt::Display for SwRegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// Validate a Service Worker registration request (WHATWG SW §3.1).
///
/// Folds every client-side registration check into one typed result so the
/// caller never re-implements an origin/scheme/scope comparison (CLAUDE.md
/// Layering mandate):
/// - §3.1 "Start Register": script + scope must use an `http`/`https` scheme
///   (`data:`/`file:`/`blob:`/… → [`SwRegisterError::TypeError`]).
/// - §3.1 "Register": secure context (HTTPS or localhost/::1).
/// - §3.1 "Register": script same-origin as the page; scope same-origin as
///   the script.
/// - §3.1 "Update": scope within the script directory.  The client cannot see
///   a `Service-Worker-Allowed` response header at this point, so the
///   conservative directory check applies (a server widening the scope is the
///   later [`validate_service_worker_allowed`] path).
///
/// The last four surface as [`SwRegisterError::SecurityError`].
pub fn validate_registration(
    script_url: &Url,
    scope: &Url,
    page_url: &Url,
) -> Result<(), SwRegisterError> {
    // §3.1 "Start Register" — http(s) scheme only (rejects data:/file:/blob:/…).
    if !is_http_scheme(script_url) {
        return Err(SwRegisterError::TypeError(
            "Service Worker script URL scheme must be \"http\" or \"https\"".into(),
        ));
    }
    if !is_http_scheme(scope) {
        return Err(SwRegisterError::TypeError(
            "Service Worker scope URL scheme must be \"http\" or \"https\"".into(),
        ));
    }

    // §3.1 "Register" — secure context required (HTTPS, localhost, or ::1).
    if !is_secure_context(page_url) {
        return Err(SwRegisterError::SecurityError(
            "Service Workers require a secure context (HTTPS or localhost)".into(),
        ));
    }

    // §3.1 "Register" — script same-origin as the page.
    if !same_origin(script_url, page_url) {
        return Err(SwRegisterError::SecurityError(format!(
            "Service Worker script must be same-origin as the page \
             (script: {}, page: {})",
            script_url.as_str(),
            page_url.as_str()
        )));
    }

    // §3.1 "Register" — scope same-origin as the script.
    if !same_origin(scope, script_url) {
        return Err(SwRegisterError::SecurityError(
            "Service Worker scope must be same-origin as the script URL".into(),
        ));
    }

    // §3.1 "Update" — scope within the script directory (no SW-Allowed header).
    if !validate_scope_path(script_url, scope) {
        return Err(SwRegisterError::SecurityError(
            "Service Worker scope is not within the script directory".into(),
        ));
    }

    Ok(())
}

/// Whether `url` uses an `http`/`https` scheme (SW §3.1 "Start Register").
fn is_http_scheme(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

/// Validate scope against script URL directory (without Service-Worker-Allowed).
///
/// The scope must be within the script's directory unless the server sends
/// a `Service-Worker-Allowed` response header.
pub fn validate_scope_path(script_url: &Url, scope: &Url) -> bool {
    let script_dir = script_directory(script_url);
    scope.path().starts_with(&script_dir)
}

/// Validate scope with Service-Worker-Allowed header.
///
/// The header value must be a valid absolute path, same-origin as the script.
pub fn validate_service_worker_allowed(
    script_url: &Url,
    scope: &Url,
    allowed_path: &str,
) -> Result<(), String> {
    // Must be an absolute path
    if !allowed_path.starts_with('/') {
        return Err("Service-Worker-Allowed must be an absolute path".into());
    }

    // No fragments or query strings
    if allowed_path.contains('?') || allowed_path.contains('#') {
        return Err("Service-Worker-Allowed must not contain query or fragment".into());
    }

    // Scope must start with the allowed path
    if !scope.path().starts_with(allowed_path) {
        return Err(format!(
            "scope '{}' is not within Service-Worker-Allowed '{}'",
            scope.path(),
            allowed_path
        ));
    }

    // Must be same-origin as script
    let mut allowed_url = script_url.clone();
    allowed_url.set_path(allowed_path);
    if !same_origin(&allowed_url, script_url) {
        return Err("Service-Worker-Allowed must be same-origin as script".into());
    }

    Ok(())
}

/// Validate a SW script's MIME type.
pub fn validate_mime_type(content_type: &str) -> bool {
    let mime = content_type.split(';').next().unwrap_or("").trim();
    ALLOWED_MIME_TYPES
        .iter()
        .any(|&allowed| mime.eq_ignore_ascii_case(allowed))
}

/// Check if a URL represents a secure context.
///
/// HTTPS, localhost, and ::1 (IPv6 loopback) are secure.
pub fn is_secure_context(url: &Url) -> bool {
    match url.scheme() {
        "http" | "ws" => {
            matches!(
                url.host_str(),
                Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
            )
        }
        "https" | "wss" => true,
        // file: URLs are not secure contexts for SW registration.
        // SW scripts must be served over HTTP(S).
        _ => false,
    }
}

/// Get the directory portion of a script URL.
fn script_directory(url: &Url) -> String {
    let path = url.path();
    if let Some(pos) = path.rfind('/') {
        path[..=pos].to_owned()
    } else {
        "/".to_owned()
    }
}

/// Check same-origin (scheme + host + port).
fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host() == b.host()
        && a.port_or_known_default() == b.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn secure_context() {
        assert!(is_secure_context(&url("https://example.com/")));
        assert!(is_secure_context(&url("http://localhost:8080/")));
        assert!(is_secure_context(&url("http://127.0.0.1/")));
        assert!(is_secure_context(&url("http://[::1]/")));
        assert!(!is_secure_context(&url("file:///tmp/test.html")));
        assert!(!is_secure_context(&url("http://example.com/")));
    }

    #[test]
    fn validate_registration_https() {
        let script = url("https://example.com/sw.js");
        let scope = url("https://example.com/");
        let page = url("https://example.com/index.html");
        assert!(validate_registration(&script, &scope, &page).is_ok());
    }

    #[test]
    fn validate_registration_http_rejected() {
        let script = url("http://example.com/sw.js");
        let scope = url("http://example.com/");
        let page = url("http://example.com/index.html");
        assert!(validate_registration(&script, &scope, &page).is_err());
    }

    #[test]
    fn validate_registration_localhost_ok() {
        let script = url("http://localhost:3000/sw.js");
        let scope = url("http://localhost:3000/");
        let page = url("http://localhost:3000/index.html");
        assert!(validate_registration(&script, &scope, &page).is_ok());
    }

    #[test]
    fn validate_registration_cross_origin_rejected() {
        let script = url("https://cdn.example.com/sw.js");
        let scope = url("https://example.com/");
        let page = url("https://example.com/index.html");
        assert!(validate_registration(&script, &scope, &page).is_err());
    }

    #[test]
    fn validate_registration_data_url_rejected() {
        let script = url("data:application/javascript,self.onmessage=()=>{}");
        let scope = url("https://example.com/");
        let page = url("https://example.com/index.html");
        assert!(validate_registration(&script, &scope, &page).is_err());
    }

    #[test]
    fn validate_scope_path_within_directory() {
        let script = url("https://example.com/js/sw.js");
        assert!(validate_scope_path(
            &script,
            &url("https://example.com/js/")
        ));
        assert!(validate_scope_path(
            &script,
            &url("https://example.com/js/sub/")
        ));
        assert!(!validate_scope_path(&script, &url("https://example.com/")));
    }

    #[test]
    fn validate_service_worker_allowed_extends_scope() {
        let script = url("https://example.com/js/sw.js");
        let scope = url("https://example.com/");
        assert!(validate_service_worker_allowed(&script, &scope, "/").is_ok());
    }

    #[test]
    fn validate_service_worker_allowed_rejects_relative() {
        let script = url("https://example.com/js/sw.js");
        let scope = url("https://example.com/");
        assert!(validate_service_worker_allowed(&script, &scope, "relative").is_err());
    }

    #[test]
    fn validate_service_worker_allowed_rejects_fragment() {
        let script = url("https://example.com/js/sw.js");
        let scope = url("https://example.com/");
        assert!(validate_service_worker_allowed(&script, &scope, "/#hash").is_err());
    }

    #[test]
    fn mime_type_validation() {
        assert!(validate_mime_type("text/javascript"));
        assert!(validate_mime_type("application/javascript"));
        assert!(validate_mime_type("text/javascript; charset=utf-8"));
        assert!(validate_mime_type("TEXT/JAVASCRIPT"));
        assert!(!validate_mime_type("text/plain"));
        assert!(!validate_mime_type("text/html"));
    }

    #[test]
    fn script_directory_extraction() {
        assert_eq!(script_directory(&url("https://example.com/sw.js")), "/");
        assert_eq!(
            script_directory(&url("https://example.com/js/sw.js")),
            "/js/"
        );
    }

    // --- Typed registration error (SW §3.1) — the variant a binding maps to ---

    #[test]
    fn bad_scheme_is_type_error() {
        // data: (and any non-http(s)) script scheme → TypeError per Start Register.
        let err = validate_registration(
            &url("data:application/javascript,self.onmessage=()=>{}"),
            &url("https://example.com/"),
            &url("https://example.com/index.html"),
        )
        .unwrap_err();
        assert!(matches!(err, SwRegisterError::TypeError(_)));
    }

    #[test]
    fn non_secure_is_security_error() {
        let err = validate_registration(
            &url("http://example.com/sw.js"),
            &url("http://example.com/"),
            &url("http://example.com/index.html"),
        )
        .unwrap_err();
        assert!(matches!(err, SwRegisterError::SecurityError(_)));
    }

    #[test]
    fn cross_origin_is_security_error() {
        let err = validate_registration(
            &url("https://cdn.example.com/sw.js"),
            &url("https://example.com/"),
            &url("https://example.com/index.html"),
        )
        .unwrap_err();
        assert!(matches!(err, SwRegisterError::SecurityError(_)));
    }

    #[test]
    fn scope_outside_script_dir_is_security_error() {
        // Folded scope-path check (SW §3.1 Update) — scope above the script
        // directory, no Service-Worker-Allowed header at the client → SecurityError.
        let err = validate_registration(
            &url("https://example.com/js/sw.js"),
            &url("https://example.com/"),
            &url("https://example.com/index.html"),
        )
        .unwrap_err();
        assert!(matches!(err, SwRegisterError::SecurityError(_)));
    }

    #[test]
    fn register_error_message_accessor() {
        let err = SwRegisterError::TypeError("boom".into());
        assert_eq!(err.message(), "boom");
        assert_eq!(err.to_string(), "boom");
    }
}
