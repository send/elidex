//! SSRF protection utilities shared across the workspace.
//!
//! Validates URLs against private/reserved IP ranges and hostname patterns
//! to prevent Server-Side Request Forgery attacks.

use crate::{NetworkError, NetworkErrorKind};
use std::net::IpAddr;

/// Validate that a URL is safe to fetch (not targeting private/internal addresses).
///
/// Checks the URL scheme (only `http`/`https` allowed) and hostname/IP against
/// known private ranges. Returns `NetworkError` on rejection.
///
/// # Known limitation
///
/// This validates the *hostname string*, not the resolved IP address.
/// Callers that resolve DNS themselves should additionally check the
/// resolved IP with [`is_private_ip`].
pub fn validate_url(url: &url::Url) -> Result<(), NetworkError> {
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(NetworkError {
                kind: NetworkErrorKind::SsrfBlocked,
                message: format!("unsupported URL scheme: {scheme}"),
            });
        }
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if let Some(host) = url.host_str() {
        let lower = host.to_ascii_lowercase();
        if lower == "localhost"
            || lower.ends_with(".local")
            || lower.ends_with(".internal")
            || lower == "::1"
        {
            return Err(NetworkError {
                kind: NetworkErrorKind::SsrfBlocked,
                message: format!("blocked private host: {host}"),
            });
        }

        if let Ok(ip) = host.parse::<IpAddr>() {
            if is_private_ip(ip) {
                return Err(NetworkError {
                    kind: NetworkErrorKind::SsrfBlocked,
                    message: format!("blocked private IP: {ip}"),
                });
            }
        }
    } else {
        return Err(NetworkError {
            kind: NetworkErrorKind::SsrfBlocked,
            message: "URL has no host".to_string(),
        });
    }

    Ok(())
}

/// Check if an IP address is in a private/reserved range.
///
/// Covers: loopback, private (RFC 1918), link-local, broadcast, unspecified,
/// CGNAT (100.64/10), documentation, IPv4-mapped IPv6, ULA (`fc00::/7`),
/// multicast (`ff00::/8`), and IPv6 link-local (`fe80::/10`).
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()           // 127.0.0.0/8
                || v4.is_private()     // 10/8, 172.16/12, 192.168/16
                || v4.is_link_local()  // 169.254/16
                || v4.is_broadcast()   // 255.255.255.255
                || v4.is_unspecified() // 0.0.0.0
                || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64 // 100.64/10 (CGNAT)
                || v4.is_documentation() // 192.0.2/24, 198.51.100/24, 203.0.113/24
        }
        IpAddr::V6(v6) => {
            // Check IPv4-mapped addresses (::ffff:0:0/96) — delegate to V4 checks.
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private_ip(IpAddr::V4(v4));
            }
            v6.is_loopback()       // ::1
                || v6.is_unspecified() // ::
                // Multicast ff00::/8
                || v6.segments()[0] >> 8 == 0xff
                // ULA (Unique Local Address) fc00::/7
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // Link-local fe80::/10
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // Documentation 2001:db8::/32 (RFC 3849)
                || (v6.segments()[0] == 0x2001 && v6.segments()[1] == 0x0db8)
                // Benchmarking 2001:2::/48 (RFC 5180)
                || (v6.segments()[0] == 0x2001 && v6.segments()[1] == 0x0002 && v6.segments()[2] == 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ipv4_blocked() {
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(is_private_ip("169.254.1.1".parse().unwrap()));
        assert!(is_private_ip("0.0.0.0".parse().unwrap()));
    }

    #[test]
    fn cgnat_blocked() {
        assert!(is_private_ip("100.64.0.1".parse().unwrap()));
        assert!(is_private_ip("100.127.255.254".parse().unwrap()));
        // Just outside CGNAT range
        assert!(!is_private_ip("100.63.255.255".parse().unwrap()));
        assert!(!is_private_ip("100.128.0.0".parse().unwrap()));
    }

    #[test]
    fn documentation_blocked() {
        assert!(is_private_ip("192.0.2.1".parse().unwrap()));
        assert!(is_private_ip("198.51.100.1".parse().unwrap()));
        assert!(is_private_ip("203.0.113.1".parse().unwrap()));
    }

    #[test]
    fn public_ipv4_allowed() {
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip("1.1.1.1".parse().unwrap()));
        assert!(!is_private_ip("93.184.216.34".parse().unwrap()));
    }

    #[test]
    fn private_ipv6_blocked() {
        assert!(is_private_ip("::1".parse().unwrap()));
        assert!(is_private_ip("::".parse().unwrap()));
        assert!(is_private_ip("fc00::1".parse().unwrap()));
        assert!(is_private_ip("fd00::1".parse().unwrap()));
        assert!(is_private_ip("fe80::1".parse().unwrap()));
        assert!(is_private_ip("ff02::1".parse().unwrap()));
    }

    #[test]
    fn ipv4_mapped_ipv6() {
        assert!(is_private_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_private_ip("::ffff:10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("::ffff:192.168.1.1".parse().unwrap()));
        assert!(!is_private_ip("::ffff:8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn ipv6_documentation_blocked() {
        assert!(is_private_ip("2001:db8::1".parse().unwrap()));
        assert!(is_private_ip("2001:db8:1::1".parse().unwrap()));
    }

    #[test]
    fn ipv6_benchmarking_blocked() {
        assert!(is_private_ip("2001:2:0::1".parse().unwrap()));
    }

    #[test]
    fn public_ipv6_allowed() {
        assert!(!is_private_ip("2606:4700::1111".parse().unwrap()));
        assert!(!is_private_ip("2001:3::1".parse().unwrap()));
    }

    #[test]
    fn validate_url_rejects_private_host() {
        let url = url::Url::parse("http://127.0.0.1/secret").unwrap();
        assert!(validate_url(&url).is_err());

        let url = url::Url::parse("http://localhost/admin").unwrap();
        assert!(validate_url(&url).is_err());

        let url = url::Url::parse("http://foo.local/bar").unwrap();
        assert!(validate_url(&url).is_err());

        let url = url::Url::parse("http://foo.internal/bar").unwrap();
        assert!(validate_url(&url).is_err());
    }

    #[test]
    fn validate_url_rejects_bad_scheme() {
        let url = url::Url::parse("ftp://example.com/file").unwrap();
        assert!(validate_url(&url).is_err());

        let url = url::Url::parse("file:///etc/passwd").unwrap();
        assert!(validate_url(&url).is_err());
    }

    #[test]
    fn validate_url_allows_public() {
        let url = url::Url::parse("https://example.com/page").unwrap();
        assert!(validate_url(&url).is_ok());

        let url = url::Url::parse("http://93.184.216.34/").unwrap();
        assert!(validate_url(&url).is_ok());
    }
}
