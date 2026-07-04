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
pub mod cancel;
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
pub(crate) mod redirect;
pub mod resource_loader;
pub mod sse;
pub(crate) mod tls;
pub mod transport;
pub mod ws;

use std::sync::Arc;

use bytes::Bytes;
use elidex_plugin::{NetworkMiddleware, SecurityOrigin};

pub use cancel::CancelHandle;
pub use cookie_jar::{CookieJar, CookieSnapshot};
pub use cors::CorsContext;
pub use error::{NetError, NetErrorKind};
pub use fetch_handle::FetchHandle;
pub use middleware::MiddlewareChain;
pub use preflight::{
    build_preflight_request, requires_preflight, run_preflight, validate_actual_against_allowance,
    validate_preflight_response, PreflightAllowance, PreflightCache, PreflightCacheKey,
};
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
///   only when `request.origin` equals the request URL's
///   [`SecurityOrigin`].  Both are engine [`SecurityOrigin`]
///   values, so an opaque initiator (sandboxed document /
///   `data:` script) never matches a tuple URL origin — the
///   credential strip is structural, not an ad hoc scheme check
///   (WHATWG HTML §7.1.1).  When `request.origin` is
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
    /// the VM-side fetch path **always** populate this field
    /// with the initiator's document [`SecurityOrigin`]
    /// (resolved through the VM's `document_origin()` settings-
    /// object-origin resolver), including opaque origins from a
    /// sandboxed iframe / `about:blank` / `data:` script, which
    /// serialise as `"null"` and never match a tuple origin.
    /// Used for cookie attach gating when
    /// [`Request::credentials`] is [`CredentialsMode::SameOrigin`].
    /// Stored as an engine [`SecurityOrigin`] (rather than
    /// `url::Origin`) so the broker speaks one origin type end to
    /// end and can carry an identity-stable opaque origin — a
    /// `url::Origin::Opaque` is freshly-unique per construction
    /// and could not represent "same sandboxed document across
    /// two fetches" (S5-4d).
    pub origin: Option<SecurityOrigin>,
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
/// - `SameOrigin`: attach iff `request.origin` (a
///   [`SecurityOrigin`]) equals the request URL's
///   [`SecurityOrigin::from_url`].  When `request.origin` is
///   `None` (genuinely no document context — embedder-driven
///   loads such as initial navigation / favicon prefetch),
///   attach unconditionally so the navigation pipeline keeps
///   the pre-PR5-cors behaviour for top-level loads.  **Opaque
///   initiator origins** (sandboxed document / `about:blank` /
///   `data:` scripts) are represented as
///   `Some(SecurityOrigin::Opaque(_))`, **not** `None` — opaque
///   never equals a tuple origin *by type*, so SameOrigin
///   strips cookies for an opaque-initiator cross-origin fetch
///   structurally, not via a per-call scheme check (S5-4d).
/// - `Include`: always attach.
fn should_attach_cookies(request: &Request) -> bool {
    match request.credentials {
        CredentialsMode::Omit => false,
        CredentialsMode::Include => true,
        CredentialsMode::SameOrigin => match &request.origin {
            None => true,
            Some(source) => source.same_origin_with_url(&request.url),
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
    origin: Option<&SecurityOrigin>,
    response_url: &url::Url,
    redirect_tainted: bool,
) -> bool {
    match credentials {
        CredentialsMode::Omit => false,
        CredentialsMode::Include => true,
        CredentialsMode::SameOrigin => match origin {
            None => true,
            Some(source) => source.same_origin_with_url(response_url) && !redirect_tainted,
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
    ///
    /// Convenience wrapper around [`Self::send_cancellable`] for
    /// callers that don't need to abort an in-flight request.
    pub async fn send(&self, request: Request) -> Result<Response, NetError> {
        self.send_cancellable(request, None).await
    }

    /// Send a raw HTTP request with optional cancellation.
    ///
    /// `cancel`, when `Some`, lets the broker abort the request
    /// before its hyper future resolves — the
    /// `MAX_CONCURRENT_FETCHES` inflight slot is released
    /// immediately rather than waiting on the underlying network
    /// IO to drain (R7.1).  The token is threaded through the
    /// preflight + redirect-follow + transport layers so any
    /// pending await point can observe the cancel and return a
    /// [`NetErrorKind::Cancelled`] error.
    pub async fn send_cancellable(
        &self,
        mut request: Request,
        cancel: Option<&CancelHandle>,
    ) -> Result<Response, NetError> {
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
            run_preflight(&self.transport, &self.preflight_cache, &request, cancel).await?;
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
            cancel,
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
mod tests;

#[cfg(test)]
mod preflight_integration_tests;
