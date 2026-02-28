//! TCP + TLS connection establishment with DNS resolution and SSRF protection.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::error::{NetError, NetErrorKind};
use crate::tls;

/// A connected stream, either plain TCP or TLS-wrapped.
#[derive(Debug)]
pub enum ConnectedStream {
    /// Plain TCP (HTTP).
    Plain(TcpStream),
    /// TLS-wrapped TCP (HTTPS). The `bool` is `true` if ALPN negotiated `h2`.
    Tls(Box<tokio_rustls::client::TlsStream<TcpStream>>, bool),
}

impl ConnectedStream {
    /// Returns `true` if ALPN negotiated HTTP/2.
    ///
    /// Plain TCP connections always return `false` — HTTP/2 over cleartext
    /// (h2c) is intentionally not supported, matching browser behavior.
    pub fn is_h2(&self) -> bool {
        match self {
            Self::Plain(_) => false,
            Self::Tls(_, h2) => *h2,
        }
    }
}

/// Configuration for the connector.
#[derive(Clone, Debug)]
pub struct ConnectorConfig {
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Allow connections to private/reserved IPs (for testing).
    pub allow_private_ips: bool,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            allow_private_ips: false,
        }
    }
}

/// Establishes TCP (+TLS) connections with DNS-level SSRF protection.
pub struct Connector {
    tls_config: Arc<rustls::ClientConfig>,
    config: ConnectorConfig,
}

impl std::fmt::Debug for Connector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connector")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Default for Connector {
    fn default() -> Self {
        Self::new()
    }
}

impl Connector {
    /// Create a new `Connector` with default settings.
    pub fn new() -> Self {
        Self::with_config(ConnectorConfig::default())
    }

    /// Create a new `Connector` with the given configuration.
    pub fn with_config(config: ConnectorConfig) -> Self {
        Self {
            tls_config: tls::build_tls_config(),
            config,
        }
    }

    /// Connect to the given host and port.
    ///
    /// Resolves DNS, validates all resolved IPs against SSRF rules,
    /// then establishes a TCP connection. For HTTPS (port 443 or explicit),
    /// performs TLS handshake with ALPN negotiation.
    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        use_tls: bool,
    ) -> Result<ConnectedStream, NetError> {
        // Resolve DNS
        let addrs = self.resolve_and_validate(host, port).await?;

        // Connect with timeout
        let stream = tokio::time::timeout(self.config.connect_timeout, TcpStream::connect(&*addrs))
            .await
            .map_err(|_| {
                NetError::new(
                    NetErrorKind::Timeout,
                    format!("connection to {host}:{port} timed out"),
                )
            })?
            .map_err(|e| {
                let kind = if e.kind() == std::io::ErrorKind::ConnectionRefused {
                    NetErrorKind::ConnectionRefused
                } else {
                    NetErrorKind::Other
                };
                NetError::with_source(kind, format!("failed to connect to {host}:{port}"), e)
            })?;

        stream.set_nodelay(true).ok();

        if use_tls {
            self.tls_handshake(stream, host).await
        } else {
            Ok(ConnectedStream::Plain(stream))
        }
    }

    /// Resolve DNS and validate all addresses against SSRF rules.
    async fn resolve_and_validate(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>, NetError> {
        let lookup = format!("{host}:{port}");
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&lookup)
            .await
            .map_err(|e| {
                NetError::with_source(
                    NetErrorKind::DnsFailure,
                    format!("DNS resolution failed for {host}"),
                    e,
                )
            })?
            .collect();

        if addrs.is_empty() {
            return Err(NetError::new(
                NetErrorKind::DnsFailure,
                format!("no addresses found for {host}"),
            ));
        }

        if !self.config.allow_private_ips {
            for addr in &addrs {
                if elidex_plugin::url_security::is_private_ip(addr.ip()) {
                    return Err(NetError::ssrf_blocked(format!(
                        "{host} resolved to private IP {}",
                        addr.ip()
                    )));
                }
            }
        }

        Ok(addrs)
    }

    /// Perform TLS handshake over an established TCP connection.
    ///
    /// The handshake is wrapped in `connect_timeout` to prevent stalling.
    async fn tls_handshake(
        &self,
        stream: TcpStream,
        host: &str,
    ) -> Result<ConnectedStream, NetError> {
        let server_name = tls::server_name(host)?;
        let connector = TlsConnector::from(self.tls_config.clone());

        let tls_stream = tokio::time::timeout(
            self.config.connect_timeout,
            connector.connect(server_name, stream),
        )
        .await
        .map_err(|_| {
            NetError::new(
                NetErrorKind::Timeout,
                format!("TLS handshake timed out for {host}"),
            )
        })?
        .map_err(|e| {
            NetError::with_source(
                NetErrorKind::TlsFailure,
                format!("TLS handshake failed for {host}"),
                e,
            )
        })?;

        // Check ALPN negotiated protocol
        let is_h2 = tls_stream
            .get_ref()
            .1
            .alpn_protocol()
            .is_some_and(|p| p == b"h2");

        Ok(ConnectedStream::Tls(Box::new(tls_stream), is_h2))
    }
}

/// Wrapper to provide `AsyncRead + AsyncWrite` over `ConnectedStream`.
///
/// hyper requires a single type implementing both traits.
pub struct StreamWrapper(pub ConnectedStream);

impl AsyncRead for StreamWrapper {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut self.get_mut().0 {
            ConnectedStream::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            ConnectedStream::Tls(s, _) => std::pin::Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for StreamWrapper {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match &mut self.get_mut().0 {
            ConnectedStream::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            ConnectedStream::Tls(s, _) => std::pin::Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut self.get_mut().0 {
            ConnectedStream::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            ConnectedStream::Tls(s, _) => std::pin::Pin::new(s.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut self.get_mut().0 {
            ConnectedStream::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            ConnectedStream::Tls(s, _) => std::pin::Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ssrf_blocks_private_resolved_ip() {
        let connector = Connector::with_config(ConnectorConfig {
            allow_private_ips: false,
            ..Default::default()
        });
        // localhost resolves to 127.0.0.1 — should be blocked
        let result = connector.connect("localhost", 80, false).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, NetErrorKind::SsrfBlocked);
    }

    #[tokio::test]
    async fn connect_timeout_on_unreachable() {
        let connector = Connector::with_config(ConnectorConfig {
            connect_timeout: Duration::from_millis(100),
            allow_private_ips: true,
        });
        // Connect to a port unlikely to be open
        let result = connector.connect("127.0.0.1", 1, false).await;
        assert!(result.is_err());
    }

    #[test]
    fn connector_config_defaults() {
        let config = ConnectorConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert!(!config.allow_private_ips);
    }
}
