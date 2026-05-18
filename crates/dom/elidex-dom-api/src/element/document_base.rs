//! Base URL maintenance for `<base>` elements (D-31 PR Phase B).
//!
//! Owns 3-layer ECS state per WHATWG HTML §2.4.3 (Document base
//! URLs) + §4.2.3 (The base element):
//!
//! - **Layer 1**: per-element `BaseFrozenUrl` component on each
//!   `<base>` (frozen URL invariant per HTML §4.2.3 "set the frozen
//!   base URL" algorithm).
//! - **Layer 2**: per-document `DocumentBaseUrl` derived cache +
//!   `DocumentFirstBase` positional index (HTML §2.4.3 first `<base>`
//!   rule).
//! - **Layer 3**: per-document `DocumentBaseUrlVersion` monotonic
//!   counter (HTML §2.4.3 "Respond to base URL changes" plug-in
//!   point for future reactive consumers; synchronous-drain
//!   semantics preclude late subscription).
//!
//! [`BaseUrlMaintainer`] is the [`MutationEvent`] consumer that
//! maintains all 3 layers, composed by
//! [`crate::ConsumerDispatcher`].
//!
//! Phase A scaffolding: only the `BaseUrlMaintainer` skeleton + the
//! `compute_frozen_url` algorithm are present; layer maintenance
//! lands in Phase B together with the component definitions in
//! `elidex_ecs::components`.

use elidex_ecs::{EcsDom, MutationEvent};
use url::Url;

// TODO swap fallback source to `dom.document_url(doc)` when
// `#11-document-url-real-navigation` slot lands.  The "about:blank"
// const here is placeholder until that slot provides a real
// `DocumentUrl` component reader.  Phase B uses this in
// `document_base_url`; Phase A test scaffolding uses it directly.
#[allow(dead_code)]
const FALLBACK_BASE_URL: &str = "about:blank";

/// Compute the frozen base URL per HTML §4.2.3 "set the frozen base
/// URL" algorithm:
///
/// 1. Let urlRecord be the result of parsing `href` against
///    `fallback` (URL spec §4.4 Basic URL parser via
///    [`Url::join`]).
/// 2. (step 3 "if any of the following are true" three-part
///    disjunction): If urlRecord is failure OR urlRecord's scheme
///    is `data` / `javascript` OR `Is base allowed for Document?`
///    (CSP base-uri directive) returns "Blocked", set the frozen
///    base URL to `fallback` and return.
/// 3. Otherwise set the frozen base URL to urlRecord and return.
///
/// CSP `Is base allowed for Document?` is currently always-allow
/// stub; CSP wiring deferred to `#11-csp-base-uri` defer slot.
/// Scheme blocklist is implemented inline.
#[must_use]
pub fn compute_frozen_url(href: &str, fallback: &Url) -> Url {
    let parsed = fallback.join(href).ok();
    match parsed {
        Some(url) if matches!(url.scheme(), "data" | "javascript") => fallback.clone(),
        Some(url) => url,
        None => fallback.clone(),
    }
}

/// [`MutationEvent`] consumer for the D-31 3-layer base URL state.
///
/// Plain unit struct (no state) — all state lives in ECS components
/// on entities. Composed as a typed field of
/// [`crate::ConsumerDispatcher`].
///
/// Phase A: skeleton with no-op `handle` (layer maintenance lands
/// in Phase B together with the ECS component definitions).
pub struct BaseUrlMaintainer;

impl BaseUrlMaintainer {
    /// Single-method dispatch entry invoked by
    /// [`crate::ConsumerDispatcher`].
    pub fn handle(&mut self, _event: &MutationEvent<'_>, _dom: &EcsDom) {
        // Phase B: pattern-match on Insert / Remove / AttributeChange
        // variants and maintain Layer 1 (BaseFrozenUrl per element)
        // + Layer 2 (DocumentBaseUrl + DocumentFirstBase per doc) +
        // Layer 3 (DocumentBaseUrlVersion bump on diff).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fallback() -> Url {
        Url::parse(FALLBACK_BASE_URL).unwrap()
    }

    #[test]
    fn compute_frozen_url_returns_parsed_url_when_valid() {
        let url = compute_frozen_url("https://example.com/page", &fallback());
        assert_eq!(url.as_str(), "https://example.com/page");
    }

    #[test]
    fn compute_frozen_url_returns_fallback_on_data_scheme() {
        let url = compute_frozen_url("data:text/plain,hello", &fallback());
        assert_eq!(url.as_str(), FALLBACK_BASE_URL);
    }

    #[test]
    fn compute_frozen_url_returns_fallback_on_javascript_scheme() {
        let url = compute_frozen_url("javascript:alert(1)", &fallback());
        assert_eq!(url.as_str(), FALLBACK_BASE_URL);
    }

    #[test]
    fn compute_frozen_url_resolves_relative_against_fallback() {
        let base = Url::parse("https://example.com/page/").unwrap();
        let url = compute_frozen_url("sub/path", &base);
        assert_eq!(url.as_str(), "https://example.com/page/sub/path");
    }
}
