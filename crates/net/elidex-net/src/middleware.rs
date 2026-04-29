//! Middleware chain for intercepting HTTP requests and responses.
//!
//! Wraps the `NetworkMiddleware` trait from `elidex-plugin` to transform
//! between the internal `Request`/`Response` types and the plugin types.

use elidex_plugin::{HttpRequest, HttpResponse, NetworkMiddleware};

use crate::error::NetError;
use crate::Request;

/// An ordered chain of network middleware.
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn NetworkMiddleware>>,
}

impl std::fmt::Debug for MiddlewareChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.middlewares.iter().map(|m| m.name()).collect();
        f.debug_struct("MiddlewareChain")
            .field("middlewares", &names)
            .finish()
    }
}

impl MiddlewareChain {
    /// Create an empty middleware chain.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the end of the chain.
    pub fn add(&mut self, mw: Box<dyn NetworkMiddleware>) {
        self.middlewares.push(mw);
    }

    /// Run all middlewares on a request (pre-send).
    ///
    /// Converts the internal `Request` to plugin `HttpRequest`, runs each
    /// middleware in order, and converts back. If any middleware rejects
    /// the request, processing stops and an error is returned.
    pub fn process_request(&self, request: &mut Request) -> Result<(), NetError> {
        let mut plugin_req = to_plugin_request(request);

        for mw in &self.middlewares {
            mw.on_request(&mut plugin_req).map_err(NetError::from)?;
        }

        apply_plugin_request(request, &plugin_req);
        Ok(())
    }

    /// Run all middlewares on a response (post-receive).
    ///
    /// Converts the internal response headers to plugin `HttpResponse`,
    /// runs each middleware in order, and applies any modifications.
    pub fn process_response(
        &self,
        status: u16,
        headers: &mut Vec<(String, String)>,
    ) -> Result<(), NetError> {
        let mut plugin_resp = HttpResponse::default();
        plugin_resp.status = status;
        plugin_resp.headers.clone_from(headers);

        for mw in &self.middlewares {
            mw.on_response(&mut plugin_resp).map_err(NetError::from)?;
        }

        *headers = plugin_resp.headers;
        Ok(())
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert internal Request to plugin `HttpRequest`.
fn to_plugin_request(request: &Request) -> HttpRequest {
    let mut req = HttpRequest::default();
    req.method.clone_from(&request.method);
    req.url = request.url.to_string();
    req.headers.clone_from(&request.headers);
    req
}

/// Apply plugin `HttpRequest` modifications back to internal Request.
fn apply_plugin_request(request: &mut Request, plugin_req: &HttpRequest) {
    request.method.clone_from(&plugin_req.method);
    request.headers.clone_from(&plugin_req.headers);
    // URL is not modified by middleware (validated separately)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use elidex_plugin::NetworkError;

    struct AddHeaderMw {
        name: String,
        value: String,
    }

    impl NetworkMiddleware for AddHeaderMw {
        fn name(&self) -> &'static str {
            "add-header"
        }
        fn on_request(&self, request: &mut HttpRequest) -> Result<(), NetworkError> {
            request
                .headers
                .push((self.name.clone(), self.value.clone()));
            Ok(())
        }
        fn on_response(&self, _response: &mut HttpResponse) -> Result<(), NetworkError> {
            Ok(())
        }
    }

    struct BlockAllMw;

    impl NetworkMiddleware for BlockAllMw {
        fn name(&self) -> &'static str {
            "block-all"
        }
        fn on_request(&self, _request: &mut HttpRequest) -> Result<(), NetworkError> {
            Err(NetworkError {
                kind: elidex_plugin::NetworkErrorKind::Other,
                message: "blocked".to_string(),
            })
        }
        fn on_response(&self, _response: &mut HttpResponse) -> Result<(), NetworkError> {
            Ok(())
        }
    }

    #[test]
    fn chain_adds_header() {
        let mut chain = MiddlewareChain::new();
        chain.add(Box::new(AddHeaderMw {
            name: "X-Test".into(),
            value: "yes".into(),
        }));

        let mut request = Request {
            method: "GET".into(),
            url: url::Url::parse("https://example.com").unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        chain.process_request(&mut request).unwrap();
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].0, "X-Test");
    }

    #[test]
    fn chain_block_short_circuits() {
        let mut chain = MiddlewareChain::new();
        chain.add(Box::new(BlockAllMw));
        chain.add(Box::new(AddHeaderMw {
            name: "X-After".into(),
            value: "no".into(),
        }));

        let mut request = Request {
            method: "GET".into(),
            url: url::Url::parse("https://example.com").unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let result = chain.process_request(&mut request);
        assert!(result.is_err());
        // The header should NOT have been added because the first mw blocked
        assert!(request.headers.is_empty());
    }

    #[test]
    fn chain_processes_response() {
        let chain = MiddlewareChain::new();
        let mut headers = vec![("content-type".into(), "text/html".into())];
        chain.process_response(200, &mut headers).unwrap();
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn chain_order_preserved() {
        let mut chain = MiddlewareChain::new();
        chain.add(Box::new(AddHeaderMw {
            name: "X-First".into(),
            value: "1".into(),
        }));
        chain.add(Box::new(AddHeaderMw {
            name: "X-Second".into(),
            value: "2".into(),
        }));

        let mut request = Request {
            method: "GET".into(),
            url: url::Url::parse("https://example.com").unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        chain.process_request(&mut request).unwrap();
        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers[0].0, "X-First");
        assert_eq!(request.headers[1].0, "X-Second");
    }
}
