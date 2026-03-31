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

pub use cookie_jar::CookieJar;
pub use cors::CorsContext;
pub use error::{NetError, NetErrorKind};
pub use fetch_handle::FetchHandle;
pub use middleware::MiddlewareChain;
pub use resource_loader::{ResourceLoader, ResourceResponse, SchemeDispatcher};
pub use transport::{HttpTransport, HttpVersion, TransportConfig};

/// An outgoing HTTP request (internal type with body).
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

        // Add cookies (skip for credentialless clients).
        if !self.credentialless {
            if let Some(cookie_header) = self.cookie_jar.cookie_header_for_url(&request.url) {
                request.headers.push(("cookie".to_string(), cookie_header));
            }
        }

        // Send with redirect following
        let max_redirects = self.transport.config().max_redirects;
        let mut response =
            redirect::follow_redirects(&self.transport, request, max_redirects).await?;

        // Store cookies from response (skip for credentialless clients).
        if !self.credentialless {
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
}
