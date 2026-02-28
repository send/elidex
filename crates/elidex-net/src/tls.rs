//! TLS configuration for the network stack.
//!
//! Uses rustls with webpki-roots for certificate verification.
//! Supports TLS 1.2 and 1.3 with ALPN negotiation for HTTP/2.

use std::sync::Arc;

use rustls::ClientConfig;
use rustls_pki_types::ServerName;

/// Create a rustls `ClientConfig` configured for browser-like TLS.
///
/// - TLS 1.2 and 1.3
/// - Mozilla/webpki root certificates
/// - ALPN: `h2, http/1.1`
pub fn build_tls_config() -> Arc<ClientConfig> {
    let root_store: rustls::RootCertStore =
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned().collect();

    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("ring supports TLS 1.2 and 1.3")
            .with_root_certificates(root_store)
            .with_no_client_auth();

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Arc::new(config)
}

/// Parse a hostname string into a `ServerName` for TLS.
///
/// # Errors
///
/// Returns an error if the hostname is not a valid DNS name or IP address.
pub fn server_name(host: &str) -> Result<ServerName<'static>, crate::error::NetError> {
    ServerName::try_from(host.to_string()).map_err(|e| {
        crate::error::NetError::with_source(
            crate::error::NetErrorKind::TlsFailure,
            format!("invalid server name: {host}"),
            e,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tls_config_succeeds() {
        let config = build_tls_config();
        assert_eq!(config.alpn_protocols.len(), 2);
        assert_eq!(config.alpn_protocols[0], b"h2");
        assert_eq!(config.alpn_protocols[1], b"http/1.1");
    }

    #[test]
    fn server_name_valid_dns() {
        let name = server_name("example.com");
        assert!(name.is_ok());
    }

    #[test]
    fn server_name_valid_ip() {
        let name = server_name("93.184.216.34");
        assert!(name.is_ok());
    }
}
