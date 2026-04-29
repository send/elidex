//! Error types for the network stack.

use std::fmt;

/// Error returned by network operations.
#[derive(Debug)]
pub struct NetError {
    /// The kind of network error.
    pub kind: NetErrorKind,
    /// A human-readable error message.
    pub message: String,
    /// Optional source error for chaining.
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl NetError {
    /// Create a new `NetError` with the given kind and message.
    pub fn new(kind: NetErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Create a new `NetError` with a source error.
    pub fn with_source(
        kind: NetErrorKind,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an SSRF-blocked error.
    pub fn ssrf_blocked(message: impl Into<String>) -> Self {
        Self::new(NetErrorKind::SsrfBlocked, message)
    }
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.message.is_empty() {
            write!(f, "{}", self.kind)
        } else {
            write!(f, "{}: {}", self.kind, self.message)
        }
    }
}

impl std::error::Error for NetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &_)
    }
}

impl From<elidex_plugin::NetworkError> for NetError {
    fn from(err: elidex_plugin::NetworkError) -> Self {
        let kind = match err.kind {
            elidex_plugin::NetworkErrorKind::ConnectionRefused => NetErrorKind::ConnectionRefused,
            elidex_plugin::NetworkErrorKind::Timeout => NetErrorKind::Timeout,
            elidex_plugin::NetworkErrorKind::DnsFailure => NetErrorKind::DnsFailure,
            elidex_plugin::NetworkErrorKind::TlsFailure => NetErrorKind::TlsFailure,
            elidex_plugin::NetworkErrorKind::SsrfBlocked => NetErrorKind::SsrfBlocked,
            _ => NetErrorKind::Other,
        };
        Self::new(kind, err.message)
    }
}

/// The kind of network error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NetErrorKind {
    /// The remote host refused the connection.
    ConnectionRefused,
    /// The operation timed out.
    Timeout,
    /// DNS resolution failed.
    DnsFailure,
    /// TLS handshake or certificate error.
    TlsFailure,
    /// The request was blocked by SSRF protection.
    SsrfBlocked,
    /// Too many redirects.
    TooManyRedirects,
    /// Redirect blocked by `request.redirect = "error"` (WHATWG
    /// Fetch §5.3).
    BadRedirect,
    /// CORS policy violation.
    CorsBlocked,
    /// Response body exceeded the size limit.
    ResponseTooLarge,
    /// Invalid or malformed URL.
    InvalidUrl,
    /// Invalid data: URL.
    InvalidDataUrl,
    /// An unclassified network error.
    #[default]
    Other,
}

impl fmt::Display for NetErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionRefused => f.write_str("connection refused"),
            Self::Timeout => f.write_str("timeout"),
            Self::DnsFailure => f.write_str("DNS failure"),
            Self::TlsFailure => f.write_str("TLS failure"),
            Self::SsrfBlocked => f.write_str("SSRF blocked"),
            Self::TooManyRedirects => f.write_str("too many redirects"),
            Self::BadRedirect => f.write_str("redirect blocked by request.redirect=error"),
            Self::CorsBlocked => f.write_str("CORS blocked"),
            Self::ResponseTooLarge => f.write_str("response too large"),
            Self::InvalidUrl => f.write_str("invalid URL"),
            Self::InvalidDataUrl => f.write_str("invalid data URL"),
            Self::Other => f.write_str("network error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_error_display_with_message() {
        let err = NetError::new(NetErrorKind::Timeout, "after 30s");
        assert_eq!(err.to_string(), "timeout: after 30s");
    }

    #[test]
    fn net_error_display_without_message() {
        let err = NetError::new(NetErrorKind::DnsFailure, "");
        assert_eq!(err.to_string(), "DNS failure");
    }

    #[test]
    fn net_error_ssrf_blocked() {
        let err = NetError::ssrf_blocked("resolved to private IP 10.0.0.1");
        assert_eq!(err.kind, NetErrorKind::SsrfBlocked);
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn net_error_with_source() {
        use std::error::Error;
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let err = NetError::with_source(NetErrorKind::ConnectionRefused, "connect failed", io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn net_error_from_plugin_network_error() {
        let plugin_err = elidex_plugin::NetworkError {
            kind: elidex_plugin::NetworkErrorKind::Timeout,
            message: "plugin timeout".into(),
        };
        let err: NetError = plugin_err.into();
        assert_eq!(err.kind, NetErrorKind::Timeout);
        assert_eq!(err.message, "plugin timeout");
    }

    #[test]
    fn net_error_from_plugin_ssrf_error() {
        let plugin_err = elidex_plugin::NetworkError {
            kind: elidex_plugin::NetworkErrorKind::SsrfBlocked,
            message: "blocked private IP: 10.0.0.1".into(),
        };
        let err: NetError = plugin_err.into();
        assert_eq!(err.kind, NetErrorKind::SsrfBlocked);
    }

    #[test]
    fn net_error_from_plugin_scheme_error() {
        let plugin_err = elidex_plugin::NetworkError {
            kind: elidex_plugin::NetworkErrorKind::SsrfBlocked,
            message: "unsupported URL scheme: ftp".into(),
        };
        let err: NetError = plugin_err.into();
        assert_eq!(err.kind, NetErrorKind::SsrfBlocked);
    }

    #[test]
    fn net_error_kind_default() {
        assert_eq!(NetErrorKind::default(), NetErrorKind::Other);
    }
}
