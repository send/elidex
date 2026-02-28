//! Resource loading abstraction and scheme dispatch.
//!
//! `ResourceLoader` is the trait for loading resources by URL.
//! `SchemeDispatcher` routes requests to the appropriate handler
//! based on the URL scheme (http/https, data:, file:).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;

use crate::cookie_jar::CookieJar;
use crate::data_url;
use crate::error::{NetError, NetErrorKind};
use crate::redirect;
use crate::transport::HttpTransport;
use crate::Request;

/// Response from a resource load.
#[derive(Clone, Debug)]
pub struct ResourceResponse {
    /// HTTP status code (or 200 for `data:`/`file:`).
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Response body.
    pub body: Bytes,
    /// Final URL after any redirects.
    pub url: url::Url,
    /// Content type (from headers or data: URL metadata).
    pub content_type: Option<String>,
}

/// Trait for loading resources by URL.
pub trait ResourceLoader: Send + Sync {
    /// Load a resource at the given URL.
    fn load(
        &self,
        url: &url::Url,
    ) -> Pin<Box<dyn Future<Output = Result<ResourceResponse, NetError>> + Send + '_>>;
}

/// Dispatches resource loads to scheme-specific handlers.
pub struct SchemeDispatcher {
    transport: Arc<HttpTransport>,
    cookie_jar: Arc<CookieJar>,
    file_access: bool,
}

impl std::fmt::Debug for SchemeDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemeDispatcher")
            .field("file_access", &self.file_access)
            .finish_non_exhaustive()
    }
}

impl SchemeDispatcher {
    /// Create a new dispatcher with the given transport and cookie jar.
    pub fn new(transport: Arc<HttpTransport>, cookie_jar: Arc<CookieJar>) -> Self {
        Self {
            transport,
            cookie_jar,
            file_access: false,
        }
    }

    /// Enable or disable `file://` URL access.
    #[must_use]
    pub fn with_file_access(mut self, enabled: bool) -> Self {
        self.file_access = enabled;
        self
    }
}

impl ResourceLoader for SchemeDispatcher {
    fn load(
        &self,
        url: &url::Url,
    ) -> Pin<Box<dyn Future<Output = Result<ResourceResponse, NetError>> + Send + '_>> {
        let url = url.clone();
        Box::pin(async move {
            match url.scheme() {
                "http" | "https" => self.load_http(&url).await,
                "data" => self.load_data(&url),
                "file" => self.load_file(&url).await,
                scheme => Err(NetError::new(
                    NetErrorKind::InvalidUrl,
                    format!("unsupported scheme: {scheme}"),
                )),
            }
        })
    }
}

impl SchemeDispatcher {
    /// Load an HTTP/HTTPS resource with cookies and redirects.
    async fn load_http(&self, url: &url::Url) -> Result<ResourceResponse, NetError> {
        // URL validation (defense-in-depth: also checked in Connector::resolve_and_validate).
        // Skipped when allow_private_ips is set (for testing).
        if !self.transport.config().allow_private_ips {
            elidex_plugin::url_security::validate_url(url)?;
        }

        // Build request with cookies
        let mut headers: Vec<(String, String)> = Vec::new();
        let cookies = self.cookie_jar.cookies_for_url(url);
        if !cookies.is_empty() {
            let cookie_header = cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            headers.push(("cookie".to_string(), cookie_header));
        }

        let request = Request {
            method: "GET".to_string(),
            url: url.clone(),
            headers,
            body: Bytes::new(),
        };

        let max_redirects = self.transport.config().max_redirects;
        let response = redirect::follow_redirects(&self.transport, request, max_redirects).await?;

        // Store cookies from response
        self.cookie_jar
            .store_from_response(&response.url, &response.headers);

        let content_type = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());

        Ok(ResourceResponse {
            status: response.status,
            headers: response.headers,
            body: response.body,
            url: response.url,
            content_type,
        })
    }

    /// Load a `data:` URL.
    #[allow(clippy::unused_self)]
    fn load_data(&self, url: &url::Url) -> Result<ResourceResponse, NetError> {
        let parsed = data_url::parse_data_url(url)?;
        Ok(ResourceResponse {
            status: 200,
            headers: vec![("content-type".to_string(), parsed.media_type.clone())],
            body: parsed.body,
            url: url.clone(),
            content_type: Some(parsed.media_type),
        })
    }

    /// Load a `file:` URL.
    ///
    /// Respects the transport's `max_response_bytes` limit.
    async fn load_file(&self, url: &url::Url) -> Result<ResourceResponse, NetError> {
        if !self.file_access {
            return Err(NetError::new(
                NetErrorKind::Other,
                "file:// access is disabled",
            ));
        }

        let path = url.to_file_path().map_err(|()| {
            NetError::new(NetErrorKind::InvalidUrl, format!("invalid file URL: {url}"))
        })?;

        // Check file size before reading
        let max_bytes = self.transport.config().max_response_bytes;
        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            NetError::with_source(
                NetErrorKind::Other,
                format!("failed to stat file: {}", path.display()),
                e,
            )
        })?;
        // Compare as u64 to avoid truncation on 32-bit platforms
        if metadata.len() > max_bytes as u64 {
            return Err(NetError::new(
                NetErrorKind::ResponseTooLarge,
                format!(
                    "file too large: {} bytes (limit: {max_bytes})",
                    metadata.len()
                ),
            ));
        }

        let body = tokio::fs::read(&path).await.map_err(|e| {
            NetError::with_source(
                NetErrorKind::Other,
                format!("failed to read file: {}", path.display()),
                e,
            )
        })?;

        // Guess content type from extension
        let content_type = guess_content_type(&path);

        Ok(ResourceResponse {
            status: 200,
            headers: content_type
                .as_ref()
                .map(|ct| vec![("content-type".to_string(), ct.clone())])
                .unwrap_or_default(),
            body: Bytes::from(body),
            url: url.clone(),
            content_type,
        })
    }
}

/// Simple content type guessing based on file extension.
fn guess_content_type(path: &std::path::Path) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    let ct = match ext.to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "txt" => "text/plain",
        _ => return None,
    };
    Some(ct.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guess_content_type_html() {
        let path = std::path::Path::new("index.html");
        assert_eq!(guess_content_type(path).unwrap(), "text/html");
    }

    #[test]
    fn guess_content_type_css() {
        let path = std::path::Path::new("style.CSS");
        assert_eq!(guess_content_type(path).unwrap(), "text/css");
    }

    #[test]
    fn guess_content_type_unknown() {
        let path = std::path::Path::new("file.xyz");
        assert!(guess_content_type(path).is_none());
    }

    #[test]
    fn load_data_url() {
        let transport = Arc::new(HttpTransport::new());
        let cookie_jar = Arc::new(CookieJar::new());
        let dispatcher = SchemeDispatcher::new(transport, cookie_jar);

        let url = url::Url::parse("data:text/plain,Hello").unwrap();
        let result = dispatcher.load_data(&url).unwrap();
        assert_eq!(result.status, 200);
        assert_eq!(result.body.as_ref(), b"Hello");
        assert_eq!(result.content_type.as_deref(), Some("text/plain"));
    }

    #[tokio::test]
    async fn file_access_disabled() {
        let transport = Arc::new(HttpTransport::new());
        let cookie_jar = Arc::new(CookieJar::new());
        let dispatcher = SchemeDispatcher::new(transport, cookie_jar);

        let url = url::Url::parse("file:///etc/passwd").unwrap();
        let result = dispatcher.load(&url).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("disabled"));
    }

    #[tokio::test]
    async fn unsupported_scheme() {
        let transport = Arc::new(HttpTransport::new());
        let cookie_jar = Arc::new(CookieJar::new());
        let dispatcher = SchemeDispatcher::new(transport, cookie_jar);

        let url = url::Url::parse("ftp://example.com/").unwrap();
        let result = dispatcher.load(&url).await;
        assert!(result.is_err());
    }
}
