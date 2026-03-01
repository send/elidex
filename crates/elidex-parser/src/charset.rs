//! Character encoding detection and transcoding.
//!
//! Detects encoding from BOM, `<meta>` prescan, or HTTP charset hint,
//! then decodes bytes to UTF-8 via `encoding_rs`.

use encoding_rs::Encoding;

/// Maximum number of bytes to prescan for `<meta charset>` declarations.
const PRESCAN_LIMIT: usize = 1024;

/// Result of encoding detection and decoding.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// UTF-8 text produced from the input bytes.
    pub text: String,
    /// Canonical encoding name (e.g. `"UTF-8"`, `"Shift_JIS"`).
    pub encoding: &'static str,
    /// How the encoding was determined.
    pub confidence: EncodingConfidence,
}

/// Confidence level of encoding detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingConfidence {
    /// BOM or HTTP `charset` hint — authoritative source.
    Definite,
    /// `<meta>` tag or fallback default — may be wrong.
    Tentative,
}

/// Detect encoding and decode `bytes` to UTF-8.
///
/// Priority:
/// 1. `charset_hint` (HTTP `Content-Type` header charset)
/// 2. BOM (UTF-8 / UTF-16 LE / UTF-16 BE)
/// 3. `<meta charset="…">` prescan (first 1024 bytes)
/// 4. `<meta http-equiv="Content-Type" content="…; charset=…">` prescan
/// 5. UTF-8 default
#[must_use]
pub fn detect_and_decode(bytes: &[u8], charset_hint: Option<&str>) -> DecodeResult {
    // Strip BOM unconditionally — BOM is not content regardless of encoding source.
    let (content, bom_encoding) = strip_bom(bytes);

    // 1. HTTP charset hint (highest priority).
    if let Some(hint) = charset_hint {
        if let Some(enc) = Encoding::for_label(hint.as_bytes()) {
            return decode_with(content, enc, EncodingConfidence::Definite);
        }
    }

    // 2. BOM-detected encoding.
    if let Some(enc) = bom_encoding {
        return decode_with(content, enc, EncodingConfidence::Definite);
    }

    // 3–4. Meta prescan.
    let scan = &content[..content.len().min(PRESCAN_LIMIT)];
    if let Some(label) = prescan_meta_charset(scan) {
        if let Some(enc) = Encoding::for_label(label.as_bytes()) {
            return decode_with(content, enc, EncodingConfidence::Tentative);
        }
    }

    // 5. Default to UTF-8.
    decode_with(content, encoding_rs::UTF_8, EncodingConfidence::Tentative)
}

/// Strip BOM from the start of `bytes`.
///
/// Returns the content after BOM (or the original slice if no BOM) and the
/// detected encoding (if any). BOM is always stripped regardless of which
/// encoding source ultimately wins — BOM bytes are not content.
fn strip_bom(bytes: &[u8]) -> (&[u8], Option<&'static Encoding>) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (&bytes[3..], Some(encoding_rs::UTF_8))
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        (&bytes[2..], Some(encoding_rs::UTF_16LE))
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (&bytes[2..], Some(encoding_rs::UTF_16BE))
    } else {
        (bytes, None)
    }
}

/// Decode `bytes` using `encoding` without BOM sniffing.
fn decode_with(
    bytes: &[u8],
    encoding: &'static Encoding,
    confidence: EncodingConfidence,
) -> DecodeResult {
    let mut decoder = encoding.new_decoder_without_bom_handling();
    let max_len = decoder
        .max_utf8_buffer_length(bytes.len())
        .unwrap_or(bytes.len().saturating_mul(4));
    let mut output = String::with_capacity(max_len);
    let (result, _read, _had_errors) = decoder.decode_to_string(bytes, &mut output, true);
    debug_assert!(
        result == encoding_rs::CoderResult::InputEmpty,
        "decoder output buffer too small"
    );
    DecodeResult {
        text: output,
        encoding: encoding.name(),
        confidence,
    }
}

/// Prescan bytes for `<meta charset="X">` or
/// `<meta http-equiv="Content-Type" content="…;charset=X">`.
///
/// Operates on raw bytes (charset names are always ASCII).
fn prescan_meta_charset(bytes: &[u8]) -> Option<String> {
    let lower: Vec<u8> = bytes.iter().map(|&b| b.to_ascii_lowercase()).collect();

    let mut pos = 0;
    while pos < lower.len() {
        let Some(offset) = find_subsequence(&lower[pos..], b"<meta") else {
            break;
        };
        let meta_start = pos + offset;

        // Find the closing > for this tag.
        let tag_end = lower[meta_start..]
            .iter()
            .position(|&b| b == b'>')
            .map_or(lower.len(), |i| meta_start + i);

        let tag_lower = &lower[meta_start..tag_end];
        let tag_orig = &bytes[meta_start..tag_end];

        let attrs = parse_tag_attrs(tag_lower, tag_orig);

        // Try <meta charset="…">
        if let Some(val) = find_attr(&attrs, "charset") {
            return Some(val.to_string());
        }

        // Try <meta http-equiv="content-type" content="…;charset=…">
        let is_content_type =
            find_attr(&attrs, "http-equiv").is_some_and(|v| v.eq_ignore_ascii_case("content-type"));
        if is_content_type {
            if let Some(content) = find_attr(&attrs, "content") {
                if let Some(cs) = extract_charset_from_content_type(content) {
                    return Some(cs);
                }
            }
        }

        pos = tag_end + 1;
    }

    None
}

/// A parsed attribute: name (lowercased) and original-case value.
struct Attr<'a> {
    name: &'a str,
    value: &'a str,
}

/// Parse attributes from a `<meta …>` tag.
///
/// `tag_lower` is the lowercased bytes, `tag_orig` is original-case bytes.
/// Both slices must have the same length and start at `<meta`.
fn parse_tag_attrs<'a>(tag_lower: &'a [u8], tag_orig: &'a [u8]) -> Vec<Attr<'a>> {
    let mut attrs = Vec::new();

    // Skip past the tag name: "<meta" + optional whitespace.
    let Some(start) = find_subsequence(tag_lower, b"<meta") else {
        return attrs;
    };
    let mut i = start + 5; // past "<meta"

    while i < tag_lower.len() {
        // Skip whitespace.
        while i < tag_lower.len() && tag_lower[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= tag_lower.len() {
            break;
        }

        // Read attribute name (lowercase).
        let name_start = i;
        while i < tag_lower.len()
            && tag_lower[i] != b'='
            && !tag_lower[i].is_ascii_whitespace()
            && tag_lower[i] != b'>'
        {
            i += 1;
        }
        if i == name_start {
            break;
        }
        let Ok(name) = std::str::from_utf8(&tag_lower[name_start..i]) else {
            break;
        };

        // Skip whitespace before '='.
        while i < tag_lower.len() && tag_lower[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= tag_lower.len() || tag_lower[i] != b'=' {
            // Attribute without value — skip it.
            continue;
        }
        i += 1; // skip '='

        // Skip whitespace after '='.
        while i < tag_lower.len() && tag_lower[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= tag_lower.len() {
            break;
        }

        // Read value (from original bytes for case preservation).
        let (value, new_i) = if tag_orig[i] == b'"' || tag_orig[i] == b'\'' {
            let quote = tag_orig[i];
            let val_start = i + 1;
            let val_end = tag_orig[val_start..]
                .iter()
                .position(|&b| b == quote)
                .map_or(tag_orig.len(), |p| val_start + p);
            let v = std::str::from_utf8(&tag_orig[val_start..val_end]).unwrap_or("");
            (v, val_end + 1)
        } else {
            let val_start = i;
            let val_end = tag_orig[val_start..]
                .iter()
                .position(|&b| b.is_ascii_whitespace() || b == b'>')
                .map_or(tag_orig.len(), |p| val_start + p);
            let v = std::str::from_utf8(&tag_orig[val_start..val_end]).unwrap_or("");
            (v, val_end)
        };

        attrs.push(Attr { name, value });
        i = new_i;
    }

    attrs
}

/// Find an attribute by lowercased name, return its original-case value.
fn find_attr<'a>(attrs: &[Attr<'a>], name: &str) -> Option<&'a str> {
    attrs.iter().find(|a| a.name == name).map(|a| a.value)
}

/// Find subsequence `needle` in `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Extract charset from a `Content-Type` value, e.g. `"text/html; charset=EUC-JP"`.
///
/// Note: A similar function exists in `elidex_navigation::loader::extract_charset`
/// for HTTP `Content-Type` headers. That version iterates `;`-separated parameters.
/// Keeping both avoids a cross-crate dependency for a small function with slightly
/// different input contexts (HTML attribute prescan vs. HTTP header).
fn extract_charset_from_content_type(content: &str) -> Option<String> {
    let lower = content.to_ascii_lowercase();
    let idx = lower.find("charset=")?;
    let rest = &content[(idx + "charset=".len())..];
    let rest = rest.trim();
    if rest.starts_with('"') || rest.starts_with('\'') {
        let quote = rest.as_bytes()[0];
        let end = rest[1..].find(char::from(quote))?;
        Some(rest[1..=end].trim().to_string())
    } else {
        let end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == ';')
            .unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_utf8_no_bom() {
        let result = detect_and_decode(b"<html><body>Hello</body></html>", None);
        assert_eq!(result.encoding, "UTF-8");
        assert_eq!(result.confidence, EncodingConfidence::Tentative);
        assert!(result.text.contains("Hello"));
    }

    #[test]
    fn detect_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"<html><body>Hello</body></html>");
        let result = detect_and_decode(&bytes, None);
        assert_eq!(result.encoding, "UTF-8");
        assert_eq!(result.confidence, EncodingConfidence::Definite);
        // BOM stripped.
        assert!(!result.text.starts_with('\u{FEFF}'));
        assert!(result.text.contains("Hello"));
    }

    #[test]
    fn detect_utf16le_bom() {
        let bytes: Vec<u8> = vec![
            0xFF, 0xFE, // BOM
            b'H', 0x00, b'i', 0x00,
        ];
        let result = detect_and_decode(&bytes, None);
        assert_eq!(result.encoding, "UTF-16LE");
        assert_eq!(result.confidence, EncodingConfidence::Definite);
        assert!(result.text.contains("Hi"));
    }

    #[test]
    fn detect_utf16be_bom() {
        let bytes: Vec<u8> = vec![
            0xFE, 0xFF, // BOM
            0x00, b'H', 0x00, b'i',
        ];
        let result = detect_and_decode(&bytes, None);
        assert_eq!(result.encoding, "UTF-16BE");
        assert_eq!(result.confidence, EncodingConfidence::Definite);
        assert!(result.text.contains("Hi"));
    }

    #[test]
    fn detect_meta_charset() {
        let html = br#"<html><head><meta charset="Shift_JIS"></head></html>"#;
        let result = detect_and_decode(html, None);
        assert_eq!(result.encoding, "Shift_JIS");
        assert_eq!(result.confidence, EncodingConfidence::Tentative);
    }

    #[test]
    fn detect_meta_http_equiv() {
        let html = br#"<html><head><meta http-equiv="Content-Type" content="text/html; charset=EUC-JP"></head></html>"#;
        let result = detect_and_decode(html, None);
        assert_eq!(result.encoding, "EUC-JP");
        assert_eq!(result.confidence, EncodingConfidence::Tentative);
    }

    #[test]
    fn hint_overrides_meta() {
        let html = br#"<html><head><meta charset="Shift_JIS"></head></html>"#;
        let result = detect_and_decode(html, Some("EUC-JP"));
        assert_eq!(result.encoding, "EUC-JP");
        assert_eq!(result.confidence, EncodingConfidence::Definite);
    }

    #[test]
    fn hint_overrides_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"<html></html>");
        let result = detect_and_decode(&bytes, Some("Shift_JIS"));
        assert_eq!(result.encoding, "Shift_JIS");
        assert_eq!(result.confidence, EncodingConfidence::Definite);
        // BOM must be stripped even when hint overrides encoding detection.
        assert!(
            result.text.starts_with("<html>"),
            "BOM not stripped: {:?}",
            &result.text[..10]
        );
    }

    #[test]
    fn transcode_shift_jis() {
        // "日本語" in Shift_JIS
        let sjis_bytes: &[u8] = &[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA];
        let result = detect_and_decode(sjis_bytes, Some("Shift_JIS"));
        assert_eq!(result.text, "日本語");
        assert_eq!(result.encoding, "Shift_JIS");
    }

    #[test]
    fn transcode_euc_jp() {
        // "日本" in EUC-JP
        let euc_bytes: &[u8] = &[0xC6, 0xFC, 0xCB, 0xDC];
        let result = detect_and_decode(euc_bytes, Some("EUC-JP"));
        assert_eq!(result.text, "日本");
        assert_eq!(result.encoding, "EUC-JP");
    }

    #[test]
    fn replacement_char_on_invalid() {
        let bytes: &[u8] = &[0x80, 0x81, 0x82];
        let result = detect_and_decode(bytes, Some("UTF-8"));
        assert!(result.text.contains('\u{FFFD}'));
    }

    #[test]
    fn prescan_limit_1024() {
        let mut html = vec![b' '; 1100];
        let meta = br#"<meta charset="Shift_JIS">"#;
        html[1050..1050 + meta.len()].copy_from_slice(meta);
        let result = detect_and_decode(&html, None);
        assert_eq!(result.encoding, "UTF-8");
        assert_eq!(result.confidence, EncodingConfidence::Tentative);
    }
}
