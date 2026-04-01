//! Service Worker scope matching (WHATWG SW §8.1).
//!
//! A URL is within a registration's scope if:
//! 1. Same origin (scheme + host + port)
//! 2. URL path starts with the scope path
//!
//! When multiple registrations match, the longest scope wins.

use url::Url;

use crate::registration::SwRegistration;

/// Check if a URL is within the given scope.
///
/// Same-origin + path prefix match per WHATWG SW §8.1.
pub fn matches_scope(scope: &Url, url: &Url) -> bool {
    // Same-origin check
    if scope.scheme() != url.scheme()
        || scope.host() != url.host()
        || scope.port_or_known_default() != url.port_or_known_default()
    {
        return false;
    }

    // blob: and data: URLs are never within SW scope
    if url.scheme() == "blob" || url.scheme() == "data" {
        return false;
    }

    // Path prefix match
    url.path().starts_with(scope.path())
}

/// Find the best matching registration for a URL (longest scope wins).
pub fn find_registration<'a>(
    registrations: &'a [SwRegistration],
    url: &Url,
) -> Option<&'a SwRegistration> {
    registrations
        .iter()
        .filter(|r| r.state.is_active() && matches_scope(&r.scope, url))
        .max_by_key(|r| r.scope.path().len())
}

/// Compute the default scope for a Service Worker script URL.
///
/// Per WHATWG SW §4.4.2: default scope is the directory of the script URL.
pub fn default_scope(script_url: &Url) -> Url {
    let mut scope = script_url.clone();
    // Remove everything after the last '/' in the path
    if let Some(pos) = scope.path().rfind('/') {
        let new_path = scope.path()[..=pos].to_owned();
        scope.set_path(&new_path);
    }
    scope
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registration::{SwRegistration, SwState};

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn reg(scope: &str, state: SwState) -> SwRegistration {
        SwRegistration {
            scope: url(scope),
            script_url: url(&format!("{scope}sw.js")),
            state,
            script_hash: None,
            last_update_check: None,
            update_via_cache: crate::registration::UpdateViaCache::Imports,
        }
    }

    #[test]
    fn same_origin_path_prefix() {
        let scope = url("https://example.com/app/");
        assert!(matches_scope(&scope, &url("https://example.com/app/")));
        assert!(matches_scope(
            &scope,
            &url("https://example.com/app/page.html")
        ));
        assert!(matches_scope(
            &scope,
            &url("https://example.com/app/sub/page")
        ));
    }

    #[test]
    fn different_origin_no_match() {
        let scope = url("https://example.com/");
        assert!(!matches_scope(
            &scope,
            &url("https://other.com/")
        ));
        assert!(!matches_scope(
            &scope,
            &url("http://example.com/")
        ));
        assert!(!matches_scope(
            &scope,
            &url("https://example.com:8080/")
        ));
    }

    #[test]
    fn path_prefix_mismatch() {
        let scope = url("https://example.com/app/");
        assert!(!matches_scope(
            &scope,
            &url("https://example.com/other/")
        ));
        // Note: /app (no trailing slash) does NOT match /app2/
        assert!(!matches_scope(
            &scope,
            &url("https://example.com/app2/")
        ));
    }

    #[test]
    fn scope_without_trailing_slash() {
        let scope = url("https://example.com/app");
        // /app matches /app, /app/, /app/page, and also /app2/ (prefix match)
        assert!(matches_scope(&scope, &url("https://example.com/app")));
        assert!(matches_scope(
            &scope,
            &url("https://example.com/app/page")
        ));
        // This is spec-correct but confusing: /app matches /app2
        assert!(matches_scope(
            &scope,
            &url("https://example.com/app2/")
        ));
    }

    #[test]
    fn blob_and_data_urls_never_match() {
        let scope = url("https://example.com/");
        assert!(!matches_scope(
            &scope,
            &url("data:text/html,<h1>hi</h1>")
        ));
        // blob: URLs have different origin format, won't match anyway
    }

    #[test]
    fn longest_scope_wins() {
        let registrations = vec![
            reg("https://example.com/", SwState::Activated),
            reg("https://example.com/app/", SwState::Activated),
            reg("https://example.com/app/admin/", SwState::Activated),
        ];

        let result = find_registration(&registrations, &url("https://example.com/app/admin/page"));
        assert_eq!(
            result.unwrap().scope.path(),
            "/app/admin/"
        );

        let result = find_registration(&registrations, &url("https://example.com/app/page"));
        assert_eq!(result.unwrap().scope.path(), "/app/");

        let result = find_registration(&registrations, &url("https://example.com/other"));
        assert_eq!(result.unwrap().scope.path(), "/");
    }

    #[test]
    fn only_active_registrations_match() {
        let registrations = vec![
            reg("https://example.com/", SwState::Installing),
            reg("https://example.com/app/", SwState::Activated),
        ];

        let result = find_registration(&registrations, &url("https://example.com/page"));
        // Root registration is Installing, not active — should not match
        assert!(result.is_none());
    }

    #[test]
    fn default_scope_is_script_directory() {
        assert_eq!(
            default_scope(&url("https://example.com/sw.js")),
            url("https://example.com/")
        );
        assert_eq!(
            default_scope(&url("https://example.com/js/sw.js")),
            url("https://example.com/js/")
        );
        assert_eq!(
            default_scope(&url("https://example.com/a/b/c/sw.js")),
            url("https://example.com/a/b/c/")
        );
    }
}
