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

/// Validate a Service Worker registration request.
///
/// Checks per WHATWG SW §4.2:
/// - HTTPS-only (localhost/::1 exception)
/// - Same-origin (script_url vs page_origin)
/// - data: URL prohibition
/// - Scope within script directory (unless Service-Worker-Allowed header)
pub fn validate_registration(
    script_url: &Url,
    scope: &Url,
    page_url: &Url,
) -> Result<(), String> {
    // HTTPS-only (localhost/::1 exception)
    if !is_secure_context(page_url) {
        return Err("Service Workers require a secure context (HTTPS or localhost)".into());
    }

    // data: URL prohibition
    if script_url.scheme() == "data" {
        return Err("data: URLs cannot be used as Service Worker scripts".into());
    }

    // Same-origin check (script_url vs page_url)
    if !same_origin(script_url, page_url) {
        return Err(format!(
            "Service Worker script must be same-origin as the page. \
             Script: {}, Page: {}",
            script_url.as_str(),
            page_url.as_str()
        ));
    }

    // Scope must be same-origin as script
    if !same_origin(scope, script_url) {
        return Err("Service Worker scope must be same-origin as the script URL".into());
    }

    Ok(())
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
        "https" | "wss" => true,
        "http" | "ws" => {
            matches!(
                url.host_str(),
                Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
            )
        }
        "file" => true,
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
        assert!(is_secure_context(&url("file:///tmp/test.html")));
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
        assert!(!validate_scope_path(
            &script,
            &url("https://example.com/")
        ));
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
        assert_eq!(
            script_directory(&url("https://example.com/sw.js")),
            "/"
        );
        assert_eq!(
            script_directory(&url("https://example.com/js/sw.js")),
            "/js/"
        );
    }
}
