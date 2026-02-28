//! `FetchHandle`: blocking HTTP fetch via tokio Runtime + `NetClient`.
//!
//! Wraps a tokio `Runtime` (current-thread) and a `NetClient` to provide
//! synchronous `send_blocking()` for use in the JS `fetch()` global.
//!
//! # Phase 2 limitation
//!
//! All HTTP requests block the UI thread. A future phase will introduce true
//! async I/O with a shared tokio runtime.

use elidex_net::{NetClient, NetError, Request, Response};

/// Handle for blocking HTTP requests from JavaScript.
///
/// Owns a lightweight tokio current-thread runtime and a `NetClient`.
/// The runtime is used exclusively for blocking on async `NetClient::send()`.
pub struct FetchHandle {
    rt: tokio::runtime::Runtime,
    client: NetClient,
}

impl FetchHandle {
    /// Create a new `FetchHandle` with the given `NetClient`.
    ///
    /// Builds a current-thread tokio runtime with I/O and timer drivers enabled.
    pub fn new(client: NetClient) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for fetch");
        Self { rt, client }
    }

    /// Send an HTTP request synchronously, blocking the current thread.
    pub fn send_blocking(&self, request: Request) -> Result<Response, NetError> {
        self.rt.block_on(self.client.send(request))
    }
}

impl std::fmt::Debug for FetchHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchHandle")
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_net::{NetClientConfig, TransportConfig};

    fn test_client() -> NetClient {
        NetClient::with_config(NetClientConfig {
            transport: TransportConfig {
                allow_private_ips: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[test]
    fn new_creates_handle() {
        let handle = FetchHandle::new(test_client());
        assert!(format!("{handle:?}").contains("FetchHandle"));
    }

    #[test]
    fn send_blocking_success() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        // Spin up a tiny HTTP server on a background tokio runtime.
        let server_rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let addr = server_rt.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 4096];
                let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
                let resp =
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello";
                stream.write_all(resp).await.unwrap();
            });
            addr
        });

        // Use a separate FetchHandle (which creates its own runtime) to send.
        // We need to run the server on a thread so it can accept while we block.
        let server_handle = std::thread::spawn(move || {
            server_rt.block_on(async {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            });
        });

        let handle = FetchHandle::new(test_client());
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
        };

        let response = handle.send_blocking(request).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"hello");

        // Server thread sleeps for 5s to keep the runtime alive; it's no longer
        // needed after we got our response, so we just detach it.
        let _ = server_handle;
    }

    #[test]
    fn send_blocking_connection_refused() {
        let handle = FetchHandle::new(test_client());
        let request = Request {
            method: "GET".to_string(),
            // Port 1 is almost certainly not listening.
            url: url::Url::parse("http://127.0.0.1:1/").unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
        };

        let result = handle.send_blocking(request);
        assert!(result.is_err());
    }
}
