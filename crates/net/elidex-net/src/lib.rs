//! HTTP network stack for the elidex browser engine.
//!
//! Provides TCP+TLS connections, HTTP/1.1 and HTTP/2 transport, connection
//! pooling, cookie management, redirect handling, CORS enforcement, and
//! resource loading (http/https, data: URLs).
//!
//! # Architecture
//!
//! ```text
//! NetClient (top-level API)
//! ├── MiddlewareChain (request/response interception)
//! ├── SchemeDispatcher (http/https, data:, file://)
//! │   ├── HttpTransport (HTTP/1.1 + HTTP/2 via hyper)
//! │   │   └── ConnectionPool (per-origin pooling)
//! │   │       └── Connector (TCP + TLS + DNS + SSRF)
//! │   ├── data_url parser
//! │   └── file loader
//! ├── CookieJar (Set-Cookie / Cookie management)
//! └── redirect tracker (SSRF re-validation)
//! ```

pub mod broker;
pub(crate) mod connector;
pub mod cookie_jar;
pub mod cors;
pub mod data_url;
pub mod error;
pub(crate) mod fetch_handle;
pub mod https_upgrade;
pub mod middleware;
pub(crate) mod pool;
pub(crate) mod redirect;
pub mod resource_loader;
pub mod sse;
pub(crate) mod tls;
pub mod transport;
pub mod ws;

use std::sync::Arc;

use bytes::Bytes;
use elidex_plugin::NetworkMiddleware;

pub use cookie_jar::{CookieJar, CookieSnapshot};
pub use cors::CorsContext;
pub use error::{NetError, NetErrorKind};
pub use fetch_handle::FetchHandle;
pub use middleware::MiddlewareChain;
pub use resource_loader::{ResourceLoader, ResourceResponse, SchemeDispatcher};
pub use transport::{HttpTransport, HttpVersion, TransportConfig};

/// `RequestRedirect` (WHATWG Fetch §5.3) — controls how the
/// broker `redirect::follow_redirects` loop reacts to a 3xx
/// response.
///
/// - [`RedirectMode::Follow`] (default): auto-follow up to the
///   transport's `max_redirects`.
/// - [`RedirectMode::Error`]: return [`NetErrorKind::BadRedirect`]
///   on the first 3xx; the JS-side surfaces this as a network
///   error.
/// - [`RedirectMode::Manual`]: return the 3xx response as-is so
///   the JS path can wrap it in an `OpaqueRedirect`-typed
///   Response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RedirectMode {
    /// Auto-follow up to `max_redirects` (spec default).
    #[default]
    Follow,
    /// Return [`NetErrorKind::BadRedirect`] on the first 3xx.
    Error,
    /// Return the 3xx response as-is (for opaque-redirect Responses).
    Manual,
}

/// `RequestCredentials` (WHATWG Fetch §5.3) — controls cookie
/// attach + storage on the request.
///
/// - [`CredentialsMode::Omit`]: never attach Cookie / never
///   store Set-Cookie from the response.
/// - [`CredentialsMode::SameOrigin`] (default): attach + store
///   only when `request.origin == request.url.origin()` (both
///   are [`url::Origin`] values; the comparison is opaque-aware
///   per WHATWG HTML §3.2.1.2).  When `request.origin` is
///   `None` (no document origin context — e.g. embedder-driven
///   loads), behaves as `Include` since there's no cross-origin
///   boundary to gate.
/// - [`CredentialsMode::Include`]: always attach + always store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CredentialsMode {
    /// Never attach Cookie or store Set-Cookie.
    Omit,
    /// Attach + store only when same-origin (spec default).
    #[default]
    SameOrigin,
    /// Always attach + always store.
    Include,
}

/// An outgoing HTTP request (internal type with body).
///
/// Constructed by JS-facing fetch paths (`elidex-js` /
/// `elidex-js-boa`) and by shell navigation / form-submit
/// paths.  Threaded through [`NetClient::send`] which honours
/// the [`Request::redirect`] / [`Request::credentials`] modes
/// during the broker dispatch loop.
#[derive(Clone, Debug)]
pub struct Request {
    /// HTTP method.
    pub method: String,
    /// Request URL.
    pub url: url::Url,
    /// Header name-value pairs.
    pub headers: Vec<(String, String)>,
    /// Request body.
    pub body: Bytes,
    /// Document / worker-script origin that initiated the
    /// request.  `None` is reserved for **embedder-driven
    /// loads** with no document context (initial navigation,
    /// favicon prefetch, etc.) — these set the field via
    /// `..Default::default()`.  Script-initiated requests from
    /// the VM-side fetch path **always** populate this field,
    /// including opaque initiator origins (`about:blank` /
    /// `data:` scripts), which use [`url::Origin::Opaque`] and
    /// serialise as `"null"` (Copilot R3 + R4 PR #133).  Used
    /// for cookie attach gating when [`Request::credentials`]
    /// is [`CredentialsMode::SameOrigin`].  Stored as
    /// [`url::Origin`] (rather than a full URL) so the broker
    /// never sees the initiator's path / query / fragment —
    /// the comparison surface matches the cookie-attach
    /// contract exactly.
    pub origin: Option<url::Origin>,
    /// How the broker should handle 3xx redirects.  Default:
    /// [`RedirectMode::Follow`].
    pub redirect: RedirectMode,
    /// Whether to attach cookies on this request.  Default:
    /// [`CredentialsMode::SameOrigin`].
    pub credentials: CredentialsMode,
}

/// Decide whether to attach the cookie jar's contents to a
/// request based on its [`CredentialsMode`] (WHATWG Fetch §5.3 /
/// §3.1.7).
///
/// - `Omit`: never attach.
/// - `SameOrigin`: attach iff `request.origin` (an
///   [`url::Origin`]) equals the request URL's origin.  When
///   `request.origin` is `None` (genuinely no document
///   context — embedder-driven loads such as initial
///   navigation / favicon prefetch), attach unconditionally so
///   the navigation pipeline keeps the pre-PR5-cors behaviour
///   for top-level loads.  **Opaque initiator origins**
///   (`about:blank` / `data:` scripts) must be represented as
///   `Some(url::Origin::Opaque(_))`, **not** `None` — opaque
///   never matches a tuple origin so SameOrigin correctly
///   blocks cookies for opaque-initiator cross-origin fetches
///   (Copilot R3 + R5 PR #133).
/// - `Include`: always attach.
fn should_attach_cookies(request: &Request) -> bool {
    match request.credentials {
        CredentialsMode::Omit => false,
        CredentialsMode::Include => true,
        CredentialsMode::SameOrigin => match &request.origin {
            None => true,
            Some(source) => *source == request.url.origin(),
        },
    }
}

/// Decide whether to **persist** Set-Cookie from a final
/// (post-redirect) response.  Mirrors [`should_attach_cookies`]
/// but evaluates against the response URL — required because a
/// redirect chain can change the effective origin between
/// dispatch and settlement (Copilot R3).
///
/// Without this re-evaluation, a request that started same-
/// origin and redirected cross-origin would persist cookies
/// from the cross-origin response under `SameOrigin`
/// credentials, contradicting the spec.
fn should_store_set_cookie_from(
    credentials: CredentialsMode,
    origin: Option<&url::Origin>,
    response_url: &url::Url,
) -> bool {
    match credentials {
        CredentialsMode::Omit => false,
        CredentialsMode::Include => true,
        CredentialsMode::SameOrigin => match origin {
            None => true,
            Some(source) => *source == response_url.origin(),
        },
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: String::new(),
            // `about:blank` is the canonical initial document
            // URL per WHATWG HTML §7.3.3 — used as a sentinel
            // here so test sites that only care about the
            // method / headers / body / new fields can
            // `..Default::default()` without naming a URL.
            url: url::Url::parse("about:blank").expect("about:blank parses"),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: None,
            redirect: RedirectMode::Follow,
            credentials: CredentialsMode::SameOrigin,
        }
    }
}

/// An incoming HTTP response (internal type with body).
#[derive(Clone, Debug)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// Header name-value pairs.
    pub headers: Vec<(String, String)>,
    /// Response body.
    pub body: Bytes,
    /// Final URL after redirects.
    pub url: url::Url,
    /// HTTP version used.
    pub version: HttpVersion,
    /// URL list (redirect chain). First = original, last = final.
    /// Empty for non-redirected responses (Fetch spec §3.1.4).
    pub url_list: Vec<url::Url>,
}

/// Configuration for [`NetClient`].
#[derive(Clone, Debug, Default)]
pub struct NetClientConfig {
    /// Transport configuration.
    pub transport: TransportConfig,
    /// Enable `file://` URL access.
    pub file_access: bool,
    /// HTTPS-only mode: auto-upgrade HTTP to HTTPS.
    pub https_only: bool,
}

/// Top-level network client integrating all subsystems.
///
/// Combines transport, cookie jar, middleware, and resource loading
/// into a single entry point for the browser engine.
pub struct NetClient {
    transport: Arc<HttpTransport>,
    cookie_jar: Arc<CookieJar>,
    middleware: MiddlewareChain,
    dispatcher: SchemeDispatcher,
    config: NetClientConfig,
    /// When `true`, Cookie headers are not sent and `Set-Cookie` headers are
    /// not stored. Used for iframe `credentialless` attribute (WHATWG HTML §4.8.5).
    credentialless: bool,
}

impl std::fmt::Debug for NetClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetClient")
            .field("config", &self.config)
            .field("middleware", &self.middleware)
            .finish_non_exhaustive()
    }
}

impl NetClient {
    /// Create a new client with default configuration.
    pub fn new() -> Self {
        Self::with_config(NetClientConfig::default())
    }

    /// Create a credentialless client that does not store or send cookies.
    ///
    /// Used for iframe `credentialless` attribute (WHATWG HTML §4.8.5):
    /// the iframe's requests omit cookies and other credentials.
    pub fn new_credentialless() -> Self {
        Self::with_config_credentialless(NetClientConfig::default())
    }

    /// Create a credentialless client with the given configuration.
    ///
    /// Like `with_config`, but the client never sends Cookie headers and
    /// never stores cookies from `Set-Cookie` response headers. This
    /// implements the iframe `credentialless` attribute (WHATWG HTML §4.8.5).
    pub fn with_config_credentialless(config: NetClientConfig) -> Self {
        let mut client = Self::with_config(config);
        client.credentialless = true;
        client
    }

    /// Create a new client with the given configuration.
    pub fn with_config(config: NetClientConfig) -> Self {
        let transport = Arc::new(HttpTransport::with_config(config.transport.clone()));
        let cookie_jar = Arc::new(CookieJar::new());
        let dispatcher = SchemeDispatcher::new(transport.clone(), cookie_jar.clone())
            .with_file_access(config.file_access);

        Self {
            transport,
            cookie_jar,
            middleware: MiddlewareChain::new(),
            dispatcher,
            config,
            credentialless: false,
        }
    }

    /// Send a raw HTTP request with middleware, cookies, and redirect handling.
    pub async fn send(&self, mut request: Request) -> Result<Response, NetError> {
        // HTTPS upgrade
        if self.config.https_only {
            request.url = https_upgrade::upgrade_to_https(&request.url)?;
        }

        // URL validation (defense-in-depth: also checked in Connector::resolve_and_validate).
        // Skipped for testing with private IPs.
        if !self.config.transport.allow_private_ips {
            elidex_plugin::url_security::validate_url(&request.url)?;
        }

        // Apply middleware (pre-request)
        self.middleware.process_request(&mut request)?;

        // Add cookies — gated by `request.credentials` (WHATWG
        // Fetch §5.3) AND the client-level `credentialless`
        // flag (HTML §4.8.5 iframe credentialless attribute).
        // `Omit` always suppresses; `SameOrigin` (default) only
        // attaches when the request URL is same-origin with
        // `request.origin` (or always when `origin` is `None`,
        // matching the embedder-driven path that has no
        // document context); `Include` always attaches.
        let attach_cookies = !self.credentialless && should_attach_cookies(&request);
        if attach_cookies {
            if let Some(cookie_header) = self.cookie_jar.cookie_header_for_url(&request.url) {
                request.headers.push(("cookie".to_string(), cookie_header));
            }
        }

        // Snapshot the credentials/origin pair before moving
        // `request` into `follow_redirects`, so the post-
        // redirect Set-Cookie storage decision can honour
        // `SameOrigin` against the **final** response URL
        // (Copilot R3): a request that started same-origin and
        // got redirected cross-origin must NOT persist Set-
        // Cookie from the cross-origin response under
        // `SameOrigin` credentials.
        let credentials_for_store = request.credentials;
        let origin_for_store = request.origin.clone();

        // Send with redirect following
        let max_redirects = self.transport.config().max_redirects;
        let mut response =
            redirect::follow_redirects(&self.transport, request, max_redirects).await?;

        // Store cookies from response — re-evaluate gating
        // against the **final** response URL because the
        // redirect chain may have crossed origin (Copilot R3).
        // For `SameOrigin`, we compare the snapshotted initiator
        // origin against `response.url.origin()`; for `Omit` /
        // `Include` the decision is URL-independent so the
        // re-eval is a no-op.
        let store_cookies = !self.credentialless
            && should_store_set_cookie_from(
                credentials_for_store,
                origin_for_store.as_ref(),
                &response.url,
            );
        if store_cookies {
            self.cookie_jar
                .store_from_response(&response.url, &response.headers);
        }

        // Apply middleware (post-response)
        self.middleware
            .process_response(response.status, &mut response.headers)?;

        Ok(response)
    }

    /// Load a resource by URL (http/https, data:, file://).
    pub async fn load(&self, url: &url::Url) -> Result<ResourceResponse, NetError> {
        let mut url = url.clone();
        if self.config.https_only && url.scheme() == "http" {
            url = https_upgrade::upgrade_to_https(&url)?;
        }
        self.dispatcher.load(&url).await
    }

    /// Access the cookie jar.
    pub fn cookie_jar(&self) -> &CookieJar {
        &self.cookie_jar
    }

    /// Get an `Arc` reference to the shared cookie jar.
    ///
    /// Used by `FetchHandle` and `EventSource` to share cookies across
    /// connections (e.g., for `withCredentials` support).
    pub fn cookie_jar_arc(&self) -> Arc<CookieJar> {
        Arc::clone(&self.cookie_jar)
    }

    /// Add a network middleware to the chain.
    pub fn add_middleware(&mut self, mw: Box<dyn NetworkMiddleware>) {
        self.middleware.add(mw);
    }
}

impl Default for NetClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_client_default() {
        let client = NetClient::new();
        assert!(client.cookie_jar().is_empty());
    }

    #[test]
    fn net_client_config_defaults() {
        let config = NetClientConfig::default();
        assert!(!config.file_access);
        assert!(!config.https_only);
    }

    #[test]
    fn request_clone() {
        let req = Request {
            method: "POST".to_string(),
            url: url::Url::parse("https://example.com").unwrap(),
            headers: vec![("content-type".to_string(), "text/plain".to_string())],
            body: Bytes::from("hello"),
            ..Default::default()
        };
        let cloned = req.clone();
        assert_eq!(cloned.method, "POST");
        assert_eq!(cloned.body.as_ref(), b"hello");
    }

    #[tokio::test]
    async fn send_to_local_server() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            stream.write_all(response).await.unwrap();
        });

        let client = NetClient::with_config(NetClientConfig {
            transport: TransportConfig {
                allow_private_ips: true,
                ..Default::default()
            },
            ..Default::default()
        });

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"ok");
    }

    #[tokio::test]
    async fn load_data_url() {
        let client = NetClient::new();
        let url = url::Url::parse("data:text/plain,Hello%20World").unwrap();
        let response = client.load(&url).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"Hello World");
        assert_eq!(response.content_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn should_attach_cookies_omit_returns_false() {
        let request = Request {
            url: url::Url::parse("http://example.com/").unwrap(),
            credentials: CredentialsMode::Omit,
            ..Default::default()
        };
        assert!(!should_attach_cookies(&request));
    }

    #[test]
    fn should_attach_cookies_include_always_true() {
        let request = Request {
            url: url::Url::parse("http://example.com/").unwrap(),
            origin: Some(url::Url::parse("http://other.com/").unwrap().origin()),
            credentials: CredentialsMode::Include,
            ..Default::default()
        };
        assert!(should_attach_cookies(&request));
    }

    #[test]
    fn should_attach_cookies_same_origin_default_attaches_when_no_origin() {
        // Embedder-driven loads have no document-origin context;
        // PR5-cors preserves the pre-PR top-level navigation
        // attach behaviour when origin is None.
        let request = Request {
            url: url::Url::parse("http://example.com/").unwrap(),
            origin: None,
            credentials: CredentialsMode::SameOrigin,
            ..Default::default()
        };
        assert!(should_attach_cookies(&request));
    }

    #[test]
    fn should_attach_cookies_same_origin_blocks_cross_origin() {
        let request = Request {
            url: url::Url::parse("http://api.other.com/data").unwrap(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            credentials: CredentialsMode::SameOrigin,
            ..Default::default()
        };
        assert!(!should_attach_cookies(&request));
    }

    #[test]
    fn should_attach_cookies_same_origin_passes_same_origin_match() {
        let request = Request {
            url: url::Url::parse("http://example.com/data").unwrap(),
            origin: Some(url::Url::parse("http://example.com/page").unwrap().origin()),
            credentials: CredentialsMode::SameOrigin,
            ..Default::default()
        };
        assert!(should_attach_cookies(&request));
    }

    /// Copilot R3 regression (finding 1): cookie storage from
    /// the **final** post-redirect response must re-evaluate
    /// the SameOrigin check against `response.url` so a
    /// same-origin → cross-origin redirect under
    /// `CredentialsMode::SameOrigin` does NOT persist cookies
    /// from the cross-origin response.
    #[test]
    fn should_store_set_cookie_blocks_cross_origin_redirect_under_same_origin() {
        let source_origin = url::Url::parse("http://example.com/page").unwrap().origin();
        let final_url = url::Url::parse("http://attacker.com/landing").unwrap();
        // Same-origin credentials, but final URL crossed origin
        // (the redirect chain landed at attacker.com).  Storage
        // decision must be `false`.
        assert!(!should_store_set_cookie_from(
            CredentialsMode::SameOrigin,
            Some(&source_origin),
            &final_url,
        ));
    }

    /// Counterpart sentinel: a same-origin → same-origin
    /// redirect chain still stores cookies under SameOrigin.
    #[test]
    fn should_store_set_cookie_allows_same_origin_redirect_under_same_origin() {
        let source_origin = url::Url::parse("http://example.com/page").unwrap().origin();
        let final_url = url::Url::parse("http://example.com/landing").unwrap();
        assert!(should_store_set_cookie_from(
            CredentialsMode::SameOrigin,
            Some(&source_origin),
            &final_url,
        ));
    }

    /// Counterpart sentinel: `Include` always stores even
    /// across cross-origin redirects.
    #[test]
    fn should_store_set_cookie_include_always_stores() {
        let source_origin = url::Url::parse("http://example.com/page").unwrap().origin();
        let final_url = url::Url::parse("http://attacker.com/landing").unwrap();
        assert!(should_store_set_cookie_from(
            CredentialsMode::Include,
            Some(&source_origin),
            &final_url,
        ));
    }

    /// Counterpart sentinel: `Omit` never stores.
    #[test]
    fn should_store_set_cookie_omit_never_stores() {
        let source_origin = url::Url::parse("http://example.com/page").unwrap().origin();
        let final_url = url::Url::parse("http://example.com/landing").unwrap();
        assert!(!should_store_set_cookie_from(
            CredentialsMode::Omit,
            Some(&source_origin),
            &final_url,
        ));
    }

    /// Regression test for Copilot R1 finding: `Request.origin`
    /// must be a [`url::Origin`] (not a full URL) so the broker
    /// never sees the initiator's path / query / fragment.  This
    /// test exercises the type-level guarantee — if `Request.origin`
    /// is ever changed back to `Option<url::Url>`, the
    /// `.origin()` call below would become a no-op round-trip
    /// and this test wouldn't catch the regression — but the
    /// surrounding `should_attach_cookies` semantics would
    /// silently regress to the path-leaking comparison the type
    /// change is meant to prevent.
    #[test]
    fn request_origin_is_origin_not_url_with_path() {
        let initiator = url::Url::parse("http://example.com/page?secret=1#frag").unwrap();
        let request = Request {
            url: url::Url::parse("http://example.com/api").unwrap(),
            // The path / query / fragment of the initiator are
            // discarded by `.origin()` — this is the contract.
            origin: Some(initiator.origin()),
            credentials: CredentialsMode::SameOrigin,
            ..Default::default()
        };
        assert!(should_attach_cookies(&request));
        // ascii_serialization() of the Origin must NOT contain
        // path / query / fragment.
        let serialised = request.origin.as_ref().unwrap().ascii_serialization();
        assert_eq!(serialised, "http://example.com");
        assert!(!serialised.contains("/page"));
        assert!(!serialised.contains("secret"));
        assert!(!serialised.contains("frag"));
    }
}
