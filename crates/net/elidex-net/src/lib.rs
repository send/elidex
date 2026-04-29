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
pub mod preflight;
pub mod preflight_cache;
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
pub use preflight::{
    build_preflight_request, requires_preflight, run_preflight, validate_actual_against_allowance,
    validate_preflight_response, PreflightAllowance,
};
pub use preflight_cache::{PreflightCache, PreflightCacheKey};
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

/// `RequestMode` (WHATWG Fetch §5.3) — selects the broker's
/// CORS treatment for this request.
///
/// - [`RequestMode::Cors`]: cross-origin requests are subject to
///   the §4.4 CORS check and §4.8 preflight.  This is the
///   default for `fetch()` but **not** the broker's default
///   (embedder-driven paths typically have no document context).
/// - [`RequestMode::NoCors`] (default): no preflight, no CORS
///   check; embedder-driven loads (initial navigation, favicon
///   prefetch) and `<img>` / `<script>` `crossorigin=""` loads
///   take this path.  The broker treats this as a transparent
///   fetch with no CORS gating.
/// - [`RequestMode::SameOrigin`]: cross-origin requests are
///   rejected at the broker level (§5.3 step 14 — we surface
///   `NetErrorKind::CorsBlocked`).
/// - [`RequestMode::Navigate`]: reserved for the navigation
///   pipeline's internal Request construction; the JS-facing
///   `init` parser rejects `"navigate"` per §5.3 step 23.  The
///   broker treats Navigate exactly like NoCors (no preflight,
///   no CORS check).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RequestMode {
    /// CORS mode — preflight + ACAO check applies.
    Cors,
    /// No CORS — transparent fetch (broker default).
    #[default]
    NoCors,
    /// Same-origin only — cross-origin is a network error.
    SameOrigin,
    /// Navigation request — internal to the navigation
    /// pipeline; the JS-facing init parser rejects this string.
    Navigate,
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
    /// CORS mode (WHATWG Fetch §5.3).  Default:
    /// [`RequestMode::NoCors`] — embedder-driven paths bypass
    /// the §4.8 preflight + §4.4 CORS check.  VM-side
    /// `fetch()` paths set [`RequestMode::Cors`] (the JS
    /// default) so cross-origin custom-header / non-simple-method
    /// fetches go through preflight.
    pub mode: RequestMode,
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
/// dispatch and settlement (Copilot R3 PR #133).
///
/// Without this re-evaluation, a request that started same-
/// origin and redirected cross-origin would persist cookies
/// from the cross-origin response under `SameOrigin`
/// credentials, contradicting the spec.
///
/// `redirect_tainted` is the WHATWG Fetch *redirect-tainted
/// origin flag* (§4.4 step 14.3) — `true` if the chain crossed
/// origin at any hop, even if the final URL landed back on the
/// initiator origin.  Under `SameOrigin` credentials, a tainted
/// chain MUST NOT persist Set-Cookie even when the final URL is
/// nominally same-origin: a malicious cross-origin hop could
/// otherwise emit a `Set-Cookie` that the same-origin landing
/// hop "blesses" through this gate.
fn should_store_set_cookie_from(
    credentials: CredentialsMode,
    origin: Option<&url::Origin>,
    response_url: &url::Url,
    redirect_tainted: bool,
) -> bool {
    match credentials {
        CredentialsMode::Omit => false,
        CredentialsMode::Include => true,
        CredentialsMode::SameOrigin => match origin {
            None => true,
            Some(source) => *source == response_url.origin() && !redirect_tainted,
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
            mode: RequestMode::NoCors,
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
    /// URL list — the full redirect chain (WHATWG Fetch §3.1.4).
    /// First entry is the original request URL; on a redirect, each
    /// hop's resolved URL is appended; the last entry equals
    /// [`Response::url`].  For a non-redirected response the list
    /// contains exactly one URL (the request URL).
    pub url_list: Vec<url::Url>,
    /// WHATWG Fetch *redirect-tainted origin flag* (§4.4 step 14.3).
    /// `true` when the redirect chain crossed origin at least once
    /// — the broker's redirect loop sets this so the JS-side
    /// response classifier (in `elidex-js`'s `vm/host/cors.rs`,
    /// **not** [`crate::cors`] which is the broker-side §4.4 ACAO
    /// check) can drop the "current URL is same-origin" Basic
    /// shortcut and run the cors path even when the **final** URL
    /// happens to land back on the initiator origin.  Not relevant for embedder-driven (NoCors)
    /// loads but defaults to `false` so all callers that don't
    /// destructure this field behave as before.
    pub is_redirect_tainted: bool,
    /// Whether the **final-hop** request was sent with credentials
    /// per WHATWG Fetch §3.2.5 *credentialed network*.  Equals
    /// `true` iff the final-hop credentials mode is
    /// [`CredentialsMode::Include`], independent of
    /// [`Request::mode`] — the broker stamps this from
    /// `request.credentials` after any §4.4 step 14.5 cross-
    /// origin redirect downgrade has been applied.  The JS-side
    /// response classifier reads this so the strict credentialed
    /// CORS rules (`ACAO: *` rejected, `ACAC: true` required)
    /// only fire when the final hop actually carried credentials
    /// (Copilot R2 PR-cors-redirect-preflight).
    pub credentialed_network: bool,
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
    /// CORS preflight cache (WHATWG Fetch §4.8 step 22) shared
    /// across all requests routed through this client.
    preflight_cache: Arc<PreflightCache>,
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
            preflight_cache: Arc::new(PreflightCache::new()),
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

        // Cors-mode requests MUST have a document origin context
        // so the §4.4 ACAO check (VM-side) and §4.8 preflight
        // (broker-side) have a value to gate against.  A `None`
        // origin at this point is a misconfigured caller — fail
        // closed before any network activity, including for
        // simple GET/HEAD/POST requests that bypass the
        // §4.8.1 preflight detection (Copilot R5 PR #134).
        if request.mode == RequestMode::Cors && request.origin.is_none() {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                "cors-mode request requires an origin context",
            ));
        }

        // CORS preflight (WHATWG Fetch §4.8) — for `mode = Cors`
        // cross-origin non-simple requests, issue an OPTIONS
        // preflight (or hit the cache) before the actual request
        // so the server gets a chance to allow / deny the
        // method + non-safelisted headers up front.
        if requires_preflight(&request) {
            run_preflight(&self.transport, &self.preflight_cache, &request).await?;
        }

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

        // Snapshot the origin before moving `request` into
        // `follow_redirects`, so the post-redirect Set-Cookie
        // storage decision can honour `SameOrigin` against the
        // **final** response URL (Copilot R3 PR #133): a request
        // that started same-origin and got redirected cross-origin
        // must NOT persist Set-Cookie from the cross-origin
        // response under `SameOrigin` credentials.
        let origin_for_store = request.origin.clone();

        // Send with redirect following.  `follow_redirects`
        // returns the **post-redirect** credentials so any
        // §4.4 step 14 cors-redirect downgrade (`Include` →
        // `SameOrigin`) is honoured by the Set-Cookie storage
        // gate below — Copilot R1 PR #134.  The shared
        // [`PreflightCache`] is threaded so cross-origin
        // CORS redirects can re-issue OPTIONS preflights
        // against the redirect target (§4.4 step 14).
        let max_redirects = self.transport.config().max_redirects;
        let (mut response, credentials_for_store) = redirect::follow_redirects(
            &self.transport,
            request,
            max_redirects,
            Some(&self.preflight_cache),
        )
        .await?;

        // Store cookies from response — re-evaluate gating
        // against the **final** response URL because the
        // redirect chain may have crossed origin (Copilot R3 PR #133).
        // For `SameOrigin`, we compare the snapshotted initiator
        // origin against `response.url.origin()`; for `Omit` /
        // `Include` the decision is URL-independent so the
        // re-eval is a no-op.  An additional gate fires when the
        // redirect chain triggered the *redirect-tainted origin
        // flag* (§4.4 step 14.3): even if the final URL is back
        // on the initiator origin, a chain that crossed origin
        // mid-flight must not persist Set-Cookie under
        // `SameOrigin` credentials — Copilot R1 PR-cors-redirect-
        // preflight.
        let store_cookies = !self.credentialless
            && should_store_set_cookie_from(
                credentials_for_store,
                origin_for_store.as_ref(),
                &response.url,
                response.is_redirect_tainted,
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

    /// Access the shared preflight cache (for embedder reset /
    /// tests).
    pub fn preflight_cache(&self) -> &Arc<PreflightCache> {
        &self.preflight_cache
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
            false,
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
            false,
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
            false,
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
            false,
        ));
    }

    /// PR-cors-redirect-preflight: SameOrigin credentials must
    /// reject cookie storage when the redirect chain crossed
    /// origin even if the final URL landed back same-origin
    /// (`redirect_tainted = true`).  Without this gate, a
    /// cross-origin hop could emit a `Set-Cookie` that the
    /// same-origin landing hop "blesses" through this gate.
    #[test]
    fn should_store_set_cookie_blocks_tainted_chain_under_same_origin() {
        let source_origin = url::Url::parse("http://example.com/page").unwrap().origin();
        let final_url = url::Url::parse("http://example.com/landing").unwrap();
        assert!(!should_store_set_cookie_from(
            CredentialsMode::SameOrigin,
            Some(&source_origin),
            &final_url,
            true,
        ));
    }

    /// Sentinel: `Include` ignores the redirect-tainted flag —
    /// the spec doesn't restrict cookie storage on `Include`
    /// chains; it's the caller's responsibility to keep that
    /// path off untrusted endpoints.
    #[test]
    fn should_store_set_cookie_include_ignores_tainted_flag() {
        let source_origin = url::Url::parse("http://example.com/page").unwrap().origin();
        let final_url = url::Url::parse("http://example.com/landing").unwrap();
        assert!(should_store_set_cookie_from(
            CredentialsMode::Include,
            Some(&source_origin),
            &final_url,
            true,
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

#[cfg(test)]
mod preflight_integration_tests {
    use super::*;
    use std::sync::{Arc as StdArc, Mutex as StdMutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use transport::TransportConfig;

    /// Stand up a TCP server that answers each accepted
    /// connection with the next scripted response, recording the
    /// raw request bytes seen on each connection so test
    /// assertions can verify the OPTIONS preflight + actual
    /// request shape.
    ///
    /// The returned `Arc<Mutex<Vec<String>>>` accumulates request
    /// strings (UTF-8-lossy) in accept order.  The function
    /// shuts down once `responses.len()` connections have been
    /// served.
    async fn spawn_scripted_server(
        responses: Vec<Vec<u8>>,
    ) -> (u16, StdArc<StdMutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let recorded: StdArc<StdMutex<Vec<String>>> = StdArc::new(StdMutex::new(Vec::new()));
        let recorded_clone = StdArc::clone(&recorded);
        // `Vec<u8>` (not `&'static [u8]`) so callers can build
        // dynamically formatted bodies without `Box::leak`'ing for
        // a `'static` upgrade (Copilot R4).  Literal slices use
        // `.to_vec()`; formatted strings use `.into_bytes()`.
        tokio::spawn(async move {
            for body in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                // Read until end-of-headers `\r\n\r\n` (or EOF) so
                // we don't truncate on a TCP fragment boundary —
                // tests assert on header content (ACRM / ACRH /
                // method line) which can land arbitrarily late
                // depending on hyper's write batching (Copilot R3).
                let mut buf = Vec::with_capacity(4096);
                let mut chunk = [0u8; 1024];
                loop {
                    let n = stream.read(&mut chunk).await.expect("scripted server read");
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let req = String::from_utf8_lossy(&buf).to_string();
                recorded_clone.lock().unwrap().push(req);
                stream
                    .write_all(&body)
                    .await
                    .expect("scripted server write");
            }
        });
        (port, recorded)
    }

    fn test_client() -> NetClient {
        NetClient::with_config(NetClientConfig {
            transport: TransportConfig {
                allow_private_ips: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    fn cors_request(method: &str, port: u16, headers: Vec<(String, String)>) -> Request {
        Request {
            method: method.to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/data")).unwrap(),
            headers,
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        }
    }

    #[tokio::test]
    async fn cors_simple_request_skips_preflight() {
        // Single-response server: cross-origin GET with no custom
        // headers → no preflight needed; a single GET round-trip
        // suffices.
        let (port, recorded) = spawn_scripted_server(vec![
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: http://example.com\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(),
        ]).await;

        let client = test_client();
        let request = cors_request("GET", port, vec![]);
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        let recorded = recorded.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert!(recorded[0].starts_with("GET "));
    }

    #[tokio::test]
    async fn cors_custom_header_issues_preflight_first() {
        // Two-response server: OPTIONS (204 with allow headers) → GET (200).
        let (port, recorded) = spawn_scripted_server(vec![
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Headers: x-custom\r\n\
              Access-Control-Allow-Methods: GET\r\n\
              Access-Control-Max-Age: 60\r\n\
              Content-Length: 0\r\n\
              Connection: close\r\n\r\n"
                .to_vec(),
            b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 2\r\nConnection: close\r\n\r\nok"
                .to_vec(),
        ])
        .await;

        let client = test_client();
        let request = cors_request("GET", port, vec![("X-Custom".into(), "1".into())]);
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        let recorded = recorded.lock().unwrap();
        assert_eq!(recorded.len(), 2, "expected OPTIONS + GET");
        assert!(
            recorded[0].starts_with("OPTIONS "),
            "first request should be OPTIONS, got: {}",
            recorded[0]
        );
        assert!(
            recorded[0]
                .to_ascii_lowercase()
                .contains("access-control-request-method: get"),
            "OPTIONS should include ACRM"
        );
        assert!(
            recorded[0]
                .to_ascii_lowercase()
                .contains("access-control-request-headers: x-custom"),
            "OPTIONS should include ACRH"
        );
        assert!(recorded[1].starts_with("GET "));
    }

    #[tokio::test]
    async fn preflight_method_rejection_blocks_request() {
        // OPTIONS responds without listing PUT in ACAM → preflight
        // fails closed; actual PUT is never sent.
        let (port, recorded) = spawn_scripted_server(vec![b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: GET\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec()])
        .await;
        let client = test_client();
        let request = cors_request("PUT", port, vec![]);
        let err = client.send(request).await.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
        let recorded = recorded.lock().unwrap();
        assert_eq!(recorded.len(), 1, "actual PUT must NOT be dispatched");
        assert!(recorded[0].starts_with("OPTIONS "));
    }

    #[tokio::test]
    async fn preflight_5xx_blocks_request() {
        let (port, recorded) = spawn_scripted_server(vec![
            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
        ])
        .await;
        let client = test_client();
        let request = cors_request("PUT", port, vec![]);
        let err = client.send(request).await.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
        let recorded = recorded.lock().unwrap();
        assert_eq!(recorded.len(), 1);
    }

    #[tokio::test]
    async fn preflight_acao_mismatch_blocks_request() {
        let (port, _recorded) = spawn_scripted_server(vec![b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://attacker.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec()])
        .await;
        let client = test_client();
        let request = cors_request("PUT", port, vec![]);
        let err = client.send(request).await.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    #[tokio::test]
    async fn preflight_cache_hit_skips_options_round_trip() {
        // First request: OPTIONS + actual.  Cache stores
        // allowance with max-age=60.  Second identical request
        // should skip OPTIONS and dispatch only the actual.
        let (port, recorded) = spawn_scripted_server(vec![
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Headers: x-custom\r\n\
              Access-Control-Allow-Methods: GET\r\n\
              Access-Control-Max-Age: 60\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
            b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 2\r\nConnection: close\r\n\r\nok"
                .to_vec(),
            b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 2\r\nConnection: close\r\n\r\nok"
                .to_vec(),
        ])
        .await;

        let client = test_client();
        let request1 = cors_request("GET", port, vec![("X-Custom".into(), "1".into())]);
        let request2 = cors_request("GET", port, vec![("X-Custom".into(), "1".into())]);
        client.send(request1).await.unwrap();
        client.send(request2).await.unwrap();
        let recorded = recorded.lock().unwrap();
        assert_eq!(recorded.len(), 3, "OPTIONS + GET + GET (cache hit on 2nd)");
        assert!(recorded[0].starts_with("OPTIONS "));
        assert!(recorded[1].starts_with("GET "));
        assert!(recorded[2].starts_with("GET "));
    }

    /// Regression for Copilot R1 finding 6: a `mode = Cors` +
    /// `credentials = Include` request that gets redirected
    /// cross-origin must NOT persist Set-Cookie from the
    /// cross-origin response (per WHATWG Fetch §4.4 step 14
    /// credentials downgrade).  Pre-fix `NetClient::send`
    /// snapshotted credentials BEFORE `follow_redirects` so the
    /// downgrade had no effect on the storage gate; post-fix
    /// `follow_redirects` returns the post-redirect credentials
    /// so the gate honours the downgrade.
    #[tokio::test]
    async fn cors_redirect_with_include_downgrades_credentials_for_set_cookie_storage() {
        // Server A: returns 302 → server B (cross-origin via
        // different port).  Server B: returns 200 with Set-Cookie.
        let (port_b, _rec_b) = spawn_scripted_server(vec![
            b"HTTP/1.1 200 OK\r\nSet-Cookie: leak=cross_origin; Path=/\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec()
                .to_vec(),
        ]).await;
        let location = format!("http://127.0.0.1:{port_b}/landing");
        let response_a = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let (port_a, _rec_a) = spawn_scripted_server(vec![response_a.into_bytes()]).await;

        let client = test_client();
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port_a}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::Include,
            redirect: RedirectMode::Follow,
        };
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        // Cookie jar must remain empty: the §4.4 step 14
        // downgrade flipped credentials Include → SameOrigin
        // mid-redirect, and the cross-origin response URL fails
        // the SameOrigin storage gate (response.url.origin() !=
        // request.origin).
        assert!(
            client.cookie_jar().is_empty(),
            "cross-origin Set-Cookie must not leak under credentials-downgrade"
        );
    }

    /// Regression for Copilot R5 finding 2: a SIMPLE cors-mode
    /// GET (no preflight needed per §4.8.1) with `origin=None`
    /// must STILL fail closed — pre-R5 the broker entry only
    /// gated through `requires_preflight`, so simple cors GET
    /// without origin context bypassed the §4.4 / §4.8 fail-
    /// closed gate entirely.  Closed at the broker entry now,
    /// before middleware / preflight detection / dispatch.
    #[tokio::test]
    async fn simple_cors_mode_without_origin_fails_closed() {
        let (port, recorded) = spawn_scripted_server(vec![
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ])
        .await;
        let client = test_client();
        let request = Request {
            // Simple safelisted-method request — would NOT trigger
            // preflight under §4.8.1, so the R2 origin-None gate
            // inside `run_preflight` doesn't catch it.
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/data")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: None, // ← the bug condition
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let err = client.send(request).await.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
        let recorded = recorded.lock().unwrap();
        assert!(
            recorded.is_empty(),
            "simple cors-mode without origin must NOT be dispatched"
        );
    }

    /// Regression for Copilot R2 finding 1: a `mode = Cors`
    /// request that reaches the preflight stage but has no
    /// origin must fail closed rather than silently bypass the
    /// CORS gate.  In normal flow the VM-side fetch path always
    /// sets `Request.origin` (see `attach_default_origin`), so
    /// reaching this branch means a misconfigured embedder
    /// caller — fail closed defensively.
    #[tokio::test]
    async fn cors_mode_without_origin_fails_closed_at_preflight() {
        // Server should never be hit — preflight must reject
        // before dispatch.
        let (port, recorded) = spawn_scripted_server(vec![
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ])
        .await;
        let client = test_client();
        let request = Request {
            method: "PUT".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/data")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: None, // ← the bug condition
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let err = client.send(request).await.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
        let recorded = recorded.lock().unwrap();
        assert!(
            recorded.is_empty(),
            "actual request must NOT be dispatched when origin is missing in cors mode"
        );
    }

    /// PR-cors-redirect-preflight: cross-origin CORS redirects
    /// to a non-simple-method target now re-issue the §4.8
    /// preflight against the redirect URL, then dispatch the
    /// actual request when the second preflight succeeds.
    /// Previously this path failed closed with `CorsBlocked`.
    #[tokio::test]
    async fn cors_redirect_re_preflights_and_succeeds() {
        // Landing server: receives a re-issued preflight at
        // `/dest`, responds with allowance, then receives the
        // PUT and replies 200.  Spawned first so the origin
        // server's 302 can encode the actual landing port.
        let (land_port, land_rec) = spawn_scripted_server(vec![
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
            b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 7\r\nConnection: close\r\n\r\nlanding"
                .to_vec(),
        ])
        .await;
        // Origin server: receives the initial PUT preflight
        // (`OPTIONS /start`), responds with allowance, then
        // receives the actual PUT and emits a 302 to the
        // cross-origin landing server.
        let location_header = format!("http://127.0.0.1:{land_port}/dest");
        let redirect_response = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let (origin_port, origin_rec) = spawn_scripted_server(vec![
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
            redirect_response.into_bytes(),
        ])
        .await;

        let client = test_client();
        let request = Request {
            method: "PUT".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"landing");

        // Origin server saw OPTIONS then PUT.  Landing server
        // saw OPTIONS (re-issued preflight) then PUT.  Each
        // server records exactly two requests.
        let origin_reqs = origin_rec.lock().unwrap().clone();
        assert_eq!(origin_reqs.len(), 2);
        assert!(origin_reqs[0].starts_with("OPTIONS "));
        assert!(origin_reqs[1].starts_with("PUT "));
        let land_reqs = land_rec.lock().unwrap().clone();
        assert_eq!(land_reqs.len(), 2);
        assert!(land_reqs[0].starts_with("OPTIONS "));
        assert!(land_reqs[1].starts_with("PUT "));

        // Final response carries the redirect-tainted flag —
        // the chain crossed origin from 127.0.0.1 → 127.0.0.1
        // (different ports = different origins per RFC 6454).
        assert!(
            response.is_redirect_tainted,
            "redirect-tainted flag must be set after a cross-origin redirect"
        );
        // url_list captures both hops.
        assert_eq!(
            response.url_list.len(),
            2,
            "url_list must record both redirect hops"
        );
    }

    /// Re-preflight failure at the redirect target surfaces
    /// `CorsBlocked` (the actual request is never dispatched).
    #[tokio::test]
    async fn cors_redirect_re_preflight_failure_blocks() {
        // First spawn the landing server with a *failing*
        // preflight response (no ACAO).
        let (land_port, _land_rec) = spawn_scripted_server(vec![
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ])
        .await;
        let location_header = format!("http://127.0.0.1:{land_port}/dest");
        let response_2 = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let (origin_port, _origin_rec) = spawn_scripted_server(vec![
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
            response_2.into_bytes(),
        ])
        .await;

        let client = test_client();
        let request = Request {
            method: "PUT".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let err = client.send(request).await.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Simple-method (`GET`) redirect chain: even when crossing
    /// origin, no preflight is required at the redirect target.
    /// Sentinel against accidentally re-issuing OPTIONS for
    /// every cross-origin GET redirect.
    #[tokio::test]
    async fn cors_redirect_simple_request_no_re_preflight() {
        // Landing: single GET response (no OPTIONS ahead of it
        // — if the broker mis-issues a preflight, this single-
        // response server hangs and the test times out).
        let (land_port, land_rec) = spawn_scripted_server(vec![b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 4\r\nConnection: close\r\n\r\nland"
            .to_vec()])
        .await;
        let location_header = format!("http://127.0.0.1:{land_port}/dest");
        let response_redirect = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nAccess-Control-Allow-Origin: http://example.com\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let (origin_port, _origin_rec) =
            spawn_scripted_server(vec![response_redirect.into_bytes()]).await;

        let client = test_client();
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"land");

        // Landing server saw exactly one request (the GET — no
        // preflight).
        let land_reqs = land_rec.lock().unwrap().clone();
        assert_eq!(
            land_reqs.len(),
            1,
            "simple-method redirect must not trigger re-preflight"
        );
        assert!(land_reqs[0].starts_with("GET "));
        // Tainted flag still set (origin differs).
        assert!(response.is_redirect_tainted);
    }

    /// PR-cors-redirect-preflight: a preflight cache hit on the
    /// redirect target skips the re-issued OPTIONS, so the
    /// landing server only sees the actual PUT on the second
    /// run of the same chain.
    #[tokio::test]
    async fn cors_redirect_re_preflight_cache_hit() {
        // Landing server scripts: (run1) OPTIONS + PUT, (run2)
        // PUT only — the cache hit avoids the second OPTIONS.
        let (land_port, land_rec) = spawn_scripted_server(vec![
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Access-Control-Max-Age: 3600\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
            b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 1\r\nConnection: close\r\n\r\nA"
                .to_vec(),
            b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 1\r\nConnection: close\r\n\r\nB"
                .to_vec(),
        ])
        .await;
        let location_header = format!("http://127.0.0.1:{land_port}/dest");
        let redirect_response = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let preflight: Vec<u8> = b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Access-Control-Max-Age: 3600\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec();
        // Origin server: run1 sees OPTIONS+PUT, run2's OPTIONS
        // is short-circuited by the cache hit on `/start` so it
        // sees only PUT.  Total 3 responses.
        let (origin_port, _origin_rec) = spawn_scripted_server(vec![
            preflight,
            redirect_response.clone().into_bytes(),
            redirect_response.into_bytes(),
        ])
        .await;

        let client = test_client();
        let mk_request = |port| Request {
            method: "PUT".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let first = client.send(mk_request(origin_port)).await.unwrap();
        assert_eq!(first.status, 200);
        let second = client.send(mk_request(origin_port)).await.unwrap();
        assert_eq!(second.status, 200);

        // Landing server saw OPTIONS+PUT on run1, then PUT only
        // on run2 (the OPTIONS was short-circuited by the cache
        // hit on the redirect target's preflight key).
        let land_reqs = land_rec.lock().unwrap().clone();
        assert_eq!(
            land_reqs.len(),
            3,
            "landing server should receive 3 requests across two runs (OPTIONS+PUT then PUT only)"
        );
        assert!(land_reqs[0].starts_with("OPTIONS "));
        assert!(land_reqs[1].starts_with("PUT "));
        assert!(land_reqs[2].starts_with("PUT "));
    }

    /// PR-cors-redirect-preflight: cookie storage gate honours
    /// the redirect-tainted flag — a chain that crossed origin
    /// must not persist `Set-Cookie` from the final response
    /// even when the landing URL is back on the initiator
    /// origin under `SameOrigin` credentials.
    #[tokio::test]
    async fn cors_redirect_tainted_chain_blocks_cookie_storage() {
        // Landing server: same origin as initiator (example.com)
        // — but reached through a cross-origin hop, so the chain
        // is tainted.  Set-Cookie on the final response must not
        // be stored.  We can't easily run example.com on
        // 127.0.0.1, so we use a same-origin-with-initiator
        // setup by aligning the request `origin` with the
        // landing port.
        let (land_port, _land_rec) = spawn_scripted_server(vec![
            b"HTTP/1.1 200 OK\r\nSet-Cookie: tainted=yes; Path=/\r\nContent-Length: 1\r\nConnection: close\r\n\r\nL".to_vec(),
        ])
        .await;
        let initiator_origin = url::Url::parse(&format!("http://127.0.0.1:{land_port}/page"))
            .unwrap()
            .origin();
        // Cross-origin redirector at a *different* port.
        let location_header = format!("http://127.0.0.1:{land_port}/dest");
        let redirect_response = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nAccess-Control-Allow-Origin: {origin_str}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            origin_str = initiator_origin.ascii_serialization(),
        );
        let (origin_port, _origin_rec) =
            spawn_scripted_server(vec![redirect_response.into_bytes()]).await;

        let client = test_client();
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(initiator_origin),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);
        assert!(
            response.is_redirect_tainted,
            "chain crossed origin (different ports) — must be tainted"
        );

        // Cookie jar must NOT have stored the `tainted=yes`
        // cookie despite the final URL being same-origin with
        // the initiator (same port).  Cookie jar `cookie_header_for_url`
        // returns `None` when no cookies match.
        let landing_url = url::Url::parse(&format!("http://127.0.0.1:{land_port}/page")).unwrap();
        let attached = client.cookie_jar().cookie_header_for_url(&landing_url);
        assert!(
            attached.is_none(),
            "tainted-chain Set-Cookie must not be persisted under SameOrigin"
        );
    }

    /// PR-cors-redirect-preflight Copilot R3: a same-origin
    /// redirect hop within a cross-origin server (e.g.
    /// `https://api.other.com/start` → `/dest`, both on the same
    /// cross-origin host) must still re-issue OPTIONS against
    /// `/dest` because the §4.8 preflight cache is keyed
    /// per-URL.  Without this the broker would skip the
    /// per-URL preflight and dispatch the actual non-simple
    /// request without a fresh allowance for `/dest`.
    #[tokio::test]
    async fn cors_redirect_same_origin_hop_to_different_url_re_preflights() {
        // Cross-origin server hosts both /start and /dest.  /start
        // gets the initial OPTIONS+PUT (PUT responds with 302 →
        // /dest).  /dest gets its own OPTIONS+PUT.  All on the
        // same port (= same origin from the initiator's POV) but
        // different URLs.
        let preflight: Vec<u8> = b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Access-Control-Allow-Methods: PUT\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec();
        // We use a relative `Location: /dest` so the port doesn't
        // need to be pre-known: it resolves against the current
        // request URL.
        let redirect_response: Vec<u8> = b"HTTP/1.1 302 Found\r\n\
              Location: /dest\r\n\
              Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec();
        let final_response: Vec<u8> = b"HTTP/1.1 200 OK\r\n\
              Access-Control-Allow-Origin: http://example.com\r\n\
              Content-Length: 1\r\nConnection: close\r\n\r\nD"
            .to_vec();
        // Server scripted: OPTIONS /start → PUT /start (302) →
        // OPTIONS /dest → PUT /dest (200).
        let (port, recorded) = spawn_scripted_server(vec![
            preflight.clone(),
            redirect_response,
            preflight,
            final_response,
        ])
        .await;

        let client = test_client();
        let request = Request {
            method: "PUT".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/start")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
        };
        let response = client.send(request).await.unwrap();
        assert_eq!(response.status, 200);

        // Server saw exactly 4 requests: OPTIONS /start, PUT /start,
        // OPTIONS /dest, PUT /dest.  Without the R3 fix, the second
        // OPTIONS would be skipped because `is_same_origin(/start, /dest)`
        // returned true, mis-treating the same-origin hop as
        // "no preflight needed".
        let reqs = recorded.lock().unwrap().clone();
        assert_eq!(reqs.len(), 4, "must see 4 requests (OPTIONS+PUT × 2)");
        assert!(reqs[0].starts_with("OPTIONS /start"));
        assert!(reqs[1].starts_with("PUT /start"));
        assert!(
            reqs[2].starts_with("OPTIONS /dest"),
            "redirect target on same cross-origin server must still be re-preflighted (R3): got {:?}",
            reqs[2].lines().next()
        );
        assert!(reqs[3].starts_with("PUT /dest"));
    }
}
