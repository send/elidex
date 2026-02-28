//! HTTPS-Only mode: automatic upgrade of HTTP URLs to HTTPS.

use crate::error::{NetError, NetErrorKind};

/// Upgrade an HTTP URL to HTTPS if applicable.
///
/// Returns a new URL with `https` scheme if the input was `http`.
/// Returns the original URL unchanged if already `https`.
///
/// # Errors
///
/// Returns `NetError` if the scheme is neither `http` nor `https`.
pub fn upgrade_to_https(url: &url::Url) -> Result<url::Url, NetError> {
    match url.scheme() {
        "https" => Ok(url.clone()),
        "http" => {
            let mut upgraded = url.clone();
            upgraded.set_scheme("https").map_err(|()| {
                NetError::new(
                    NetErrorKind::InvalidUrl,
                    format!("failed to upgrade URL to HTTPS: {url}"),
                )
            })?;
            Ok(upgraded)
        }
        scheme => Err(NetError::new(
            NetErrorKind::InvalidUrl,
            format!("cannot upgrade scheme '{scheme}' to HTTPS"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_http_to_https() {
        let url = url::Url::parse("http://example.com/path?q=1").unwrap();
        let upgraded = upgrade_to_https(&url).unwrap();
        assert_eq!(upgraded.scheme(), "https");
        assert_eq!(upgraded.host_str(), Some("example.com"));
        assert_eq!(upgraded.path(), "/path");
        assert_eq!(upgraded.query(), Some("q=1"));
    }

    #[test]
    fn https_unchanged() {
        let url = url::Url::parse("https://example.com/path").unwrap();
        let result = upgrade_to_https(&url).unwrap();
        assert_eq!(result.as_str(), url.as_str());
    }

    #[test]
    fn http_with_port_upgraded() {
        let url = url::Url::parse("http://example.com:8080/").unwrap();
        let upgraded = upgrade_to_https(&url).unwrap();
        assert_eq!(upgraded.scheme(), "https");
        assert_eq!(upgraded.port(), Some(8080));
    }

    #[test]
    fn unsupported_scheme_error() {
        let url = url::Url::parse("ftp://example.com/").unwrap();
        let result = upgrade_to_https(&url);
        assert!(result.is_err());
    }
}
