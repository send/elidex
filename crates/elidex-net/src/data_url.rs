//! `data:` URL parser (RFC 2397).
//!
//! Parses `data:[<mediatype>][;base64],<data>` URLs into their components.

use bytes::Bytes;

use crate::error::{NetError, NetErrorKind};

/// Maximum decoded data URL body size (2 MB).
const MAX_DATA_URL_BYTES: usize = 2 * 1024 * 1024;

/// Parsed data URL components.
#[derive(Clone, Debug)]
pub struct DataUrl {
    /// MIME type (e.g. `text/plain;charset=US-ASCII`).
    pub media_type: String,
    /// Decoded body bytes.
    pub body: Bytes,
}

/// Parse a `data:` URL into its media type and body.
///
/// Supports both plain text (percent-encoded) and base64 encoding.
/// Default media type is `text/plain;charset=US-ASCII` per RFC 2397.
/// Decoded body is limited to 2 MB.
pub fn parse_data_url(url: &url::Url) -> Result<DataUrl, NetError> {
    if url.scheme() != "data" {
        return Err(NetError::new(
            NetErrorKind::InvalidDataUrl,
            format!("not a data URL: {}", url.scheme()),
        ));
    }

    // data: URL path is everything after "data:"
    // The url crate stores the "path" as the part after the scheme
    let raw = &url.as_str()["data:".len()..];

    // Split at the first comma
    let comma_pos = raw.find(',').ok_or_else(|| {
        NetError::new(
            NetErrorKind::InvalidDataUrl,
            "data URL missing comma separator",
        )
    })?;

    let header = &raw[..comma_pos];
    let data_part = &raw[comma_pos + 1..];

    // Check for base64 encoding
    let is_base64 = header.ends_with(";base64");
    let media_type_part = if is_base64 {
        &header[..header.len() - ";base64".len()]
    } else {
        header
    };

    // Default media type
    let media_type = if media_type_part.is_empty() {
        "text/plain;charset=US-ASCII".to_string()
    } else {
        media_type_part.to_string()
    };

    // Reject if the raw data part is too large (conservative pre-check)
    if data_part.len() > MAX_DATA_URL_BYTES * 2 {
        return Err(NetError::new(
            NetErrorKind::InvalidDataUrl,
            format!(
                "data URL too large: {} bytes (limit: {MAX_DATA_URL_BYTES} decoded bytes)",
                data_part.len()
            ),
        ));
    }

    // Decode body
    let body = if is_base64 {
        decode_base64(data_part)?
    } else {
        decode_percent(data_part)
    };

    if body.len() > MAX_DATA_URL_BYTES {
        return Err(NetError::new(
            NetErrorKind::InvalidDataUrl,
            format!(
                "data URL decoded body too large: {} bytes (limit: {MAX_DATA_URL_BYTES})",
                body.len()
            ),
        ));
    }

    Ok(DataUrl {
        media_type,
        body: Bytes::from(body),
    })
}

/// Decode base64 data (ignoring whitespace).
///
/// Uses a minimal inline decoder (RFC 4648 standard alphabet) to avoid
/// adding the `base64` crate as a dependency for this single use case.
/// If the crate is needed elsewhere in the future, this should be replaced.
fn decode_base64(data: &str) -> Result<Vec<u8>, NetError> {
    // Strip whitespace (some data URLs contain spaces/newlines)
    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();

    // Simple base64 decoder (RFC 4648)
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;

    for ch in cleaned.bytes() {
        let val = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break, // padding
            _ => {
                return Err(NetError::new(
                    NetErrorKind::InvalidDataUrl,
                    format!("invalid base64 character: {}", ch as char),
                ));
            }
        };
        buf = (buf << 6) | u32::from(val);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            #[allow(clippy::cast_possible_truncation)]
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(out)
}

/// Decode percent-encoded data.
fn decode_percent(data: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let bytes = data.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_data_url() {
        let url = url::Url::parse("data:text/plain,Hello%20World").unwrap();
        let result = parse_data_url(&url).unwrap();
        assert_eq!(result.media_type, "text/plain");
        assert_eq!(result.body.as_ref(), b"Hello World");
    }

    #[test]
    fn base64_data_url() {
        let url = url::Url::parse("data:text/plain;base64,SGVsbG8=").unwrap();
        let result = parse_data_url(&url).unwrap();
        assert_eq!(result.media_type, "text/plain");
        assert_eq!(result.body.as_ref(), b"Hello");
    }

    #[test]
    fn default_media_type() {
        let url = url::Url::parse("data:,Hello").unwrap();
        let result = parse_data_url(&url).unwrap();
        assert_eq!(result.media_type, "text/plain;charset=US-ASCII");
        assert_eq!(result.body.as_ref(), b"Hello");
    }

    #[test]
    fn html_data_url() {
        let url = url::Url::parse("data:text/html,%3Ch1%3EHi%3C%2Fh1%3E").unwrap();
        let result = parse_data_url(&url).unwrap();
        assert_eq!(result.media_type, "text/html");
        assert_eq!(result.body.as_ref(), b"<h1>Hi</h1>");
    }

    #[test]
    fn missing_comma_error() {
        let url = url::Url::parse("data:text/plain").unwrap();
        let result = parse_data_url(&url);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, NetErrorKind::InvalidDataUrl);
    }

    #[test]
    fn not_data_url_error() {
        let url = url::Url::parse("https://example.com").unwrap();
        let result = parse_data_url(&url);
        assert!(result.is_err());
    }

    #[test]
    fn base64_with_padding() {
        // "AB" in base64 = "QUI="
        let url = url::Url::parse("data:;base64,QUI=").unwrap();
        let result = parse_data_url(&url).unwrap();
        assert_eq!(result.body.as_ref(), b"AB");
    }
}
