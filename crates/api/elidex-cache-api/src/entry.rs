/// A cached request/response pair stored in the Cache API.
#[derive(Debug, Clone)]
pub struct CachedEntry {
    /// Request URL.
    pub request_url: String,
    /// Request HTTP method.
    pub request_method: String,
    /// Response HTTP status code.
    pub response_status: u16,
    /// Response HTTP status text.
    pub response_status_text: String,
    /// Response headers as (name, value) pairs.
    pub response_headers: Vec<(String, String)>,
    /// Response body bytes.
    pub response_body: Vec<u8>,
    /// Vary headers from the response (for matching).
    /// Stored as the request header values that Vary references.
    pub vary_headers: Vec<(String, String)>,
    /// Whether this is an opaque response (status 0, no-cors).
    pub is_opaque: bool,
}

/// Options for matching cached entries.
#[derive(Debug, Clone, Default)]
pub struct MatchOptions {
    /// Ignore the query string when matching URLs.
    pub ignore_search: bool,
    /// Ignore the HTTP method (default: only GET matches).
    pub ignore_method: bool,
    /// Ignore Vary header matching.
    pub ignore_vary: bool,
}

impl CachedEntry {
    /// Serialize the entry to bytes for storage.
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(&self.to_json()).unwrap_or_default()
    }

    /// Deserialize from stored bytes.
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_slice(data).ok()?;
        Self::from_json(&json)
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "request_url": self.request_url,
            "request_method": self.request_method,
            "response_status": self.response_status,
            "response_status_text": self.response_status_text,
            "response_headers": self.response_headers,
            "response_body": base64_encode(&self.response_body),
            "vary_headers": self.vary_headers,
            "is_opaque": self.is_opaque,
        })
    }

    fn from_json(json: &serde_json::Value) -> Option<Self> {
        Some(Self {
            request_url: json.get("request_url")?.as_str()?.to_owned(),
            request_method: json.get("request_method")?.as_str()?.to_owned(),
            response_status: json.get("response_status")?.as_u64()? as u16,
            response_status_text: json
                .get("response_status_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            response_headers: json
                .get("response_headers")?
                .as_array()?
                .iter()
                .filter_map(|pair| {
                    let arr = pair.as_array()?;
                    Some((arr.first()?.as_str()?.to_owned(), arr.get(1)?.as_str()?.to_owned()))
                })
                .collect(),
            response_body: base64_decode(json.get("response_body")?.as_str()?),
            vary_headers: json
                .get("vary_headers")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|pair| {
                            let a = pair.as_array()?;
                            Some((a.first()?.as_str()?.to_owned(), a.get(1)?.as_str()?.to_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            is_opaque: json
                .get("is_opaque")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        })
    }

    /// Compute the storage key for this entry (url + method).
    pub fn storage_key(&self) -> Vec<u8> {
        format!("{}|{}", self.request_method, self.request_url).into_bytes()
    }

    /// Build a storage key from url and method.
    pub fn make_key(url: &str, method: &str) -> Vec<u8> {
        format!("{method}|{url}").into_bytes()
    }

    /// Estimated size for quota tracking.
    /// Opaque responses are padded (~3x) per spec to prevent timing attacks.
    pub fn quota_size(&self) -> u64 {
        let base = self.response_body.len() as u64
            + self.request_url.len() as u64
            + self
                .response_headers
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>() as u64;
        if self.is_opaque {
            base * 3 // padded size for privacy
        } else {
            base
        }
    }
}

/// Strip the query string from a URL for `ignoreSearch` matching.
pub fn strip_query(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// Check if a request matches a cached entry given match options.
pub fn entry_matches(
    entry: &CachedEntry,
    url: &str,
    method: &str,
    request_headers: &[(String, String)],
    options: &MatchOptions,
) -> bool {
    // Method check
    if !options.ignore_method && entry.request_method != method {
        return false;
    }

    // URL check
    let entry_url = if options.ignore_search {
        strip_query(&entry.request_url)
    } else {
        &entry.request_url
    };
    let match_url = if options.ignore_search {
        strip_query(url)
    } else {
        url
    };
    if entry_url != match_url {
        return false;
    }

    // Vary header check
    if !options.ignore_vary && !entry.vary_headers.is_empty() {
        for (header_name, cached_value) in &entry.vary_headers {
            if header_name == "*" {
                return false; // Vary: * means never match
            }
            let request_value = request_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(header_name))
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            if cached_value != request_value {
                return false;
            }
        }
    }

    true
}

// Simple base64 encoding/decoding for response body storage.
// Using a minimal implementation to avoid adding a base64 dependency.

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(s: &str) -> Vec<u8> {
    fn char_val(c: u8) -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|&b| b != b'=').collect();
    let mut result = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let b0 = char_val(chunk[0]) as u32;
        let b1 = char_val(chunk[1]) as u32;
        result.push(((b0 << 2) | (b1 >> 4)) as u8);
        if chunk.len() > 2 {
            let b2 = char_val(chunk[2]) as u32;
            result.push((((b1 & 0xF) << 4) | (b2 >> 2)) as u8);
            if chunk.len() > 3 {
                let b3 = char_val(chunk[3]) as u32;
                result.push((((b2 & 0x3) << 6) | b3) as u8);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_roundtrip() {
        let entry = CachedEntry {
            request_url: "https://example.com/page".into(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![("content-type".into(), "text/html".into())],
            response_body: b"<h1>Hello</h1>".to_vec(),
            vary_headers: vec![],
            is_opaque: false,
        };
        let bytes = entry.serialize();
        let restored = CachedEntry::deserialize(&bytes).unwrap();
        assert_eq!(restored.request_url, entry.request_url);
        assert_eq!(restored.response_status, 200);
        assert_eq!(restored.response_body, b"<h1>Hello</h1>");
    }

    #[test]
    fn storage_key() {
        let key = CachedEntry::make_key("https://example.com/", "GET");
        assert_eq!(key, b"GET|https://example.com/");
    }

    #[test]
    fn entry_matches_basic() {
        let entry = CachedEntry {
            request_url: "https://example.com/page?v=1".into(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            vary_headers: vec![],
            is_opaque: false,
        };
        assert!(entry_matches(
            &entry,
            "https://example.com/page?v=1",
            "GET",
            &[],
            &MatchOptions::default()
        ));
        assert!(!entry_matches(
            &entry,
            "https://example.com/page?v=2",
            "GET",
            &[],
            &MatchOptions::default()
        ));
    }

    #[test]
    fn entry_matches_ignore_search() {
        let entry = CachedEntry {
            request_url: "https://example.com/page?v=1".into(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            vary_headers: vec![],
            is_opaque: false,
        };
        let opts = MatchOptions {
            ignore_search: true,
            ..Default::default()
        };
        assert!(entry_matches(
            &entry,
            "https://example.com/page?v=2",
            "GET",
            &[],
            &opts
        ));
    }

    #[test]
    fn entry_matches_ignore_method() {
        let entry = CachedEntry {
            request_url: "https://example.com/api".into(),
            request_method: "POST".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            vary_headers: vec![],
            is_opaque: false,
        };
        assert!(!entry_matches(
            &entry,
            "https://example.com/api",
            "GET",
            &[],
            &MatchOptions::default()
        ));
        let opts = MatchOptions {
            ignore_method: true,
            ..Default::default()
        };
        assert!(entry_matches(
            &entry,
            "https://example.com/api",
            "GET",
            &[],
            &opts
        ));
    }

    #[test]
    fn entry_matches_vary_header() {
        let entry = CachedEntry {
            request_url: "https://api.com/data".into(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            vary_headers: vec![("accept".into(), "application/json".into())],
            is_opaque: false,
        };
        // Matching Accept header
        assert!(entry_matches(
            &entry,
            "https://api.com/data",
            "GET",
            &[("accept".into(), "application/json".into())],
            &MatchOptions::default()
        ));
        // Different Accept header
        assert!(!entry_matches(
            &entry,
            "https://api.com/data",
            "GET",
            &[("accept".into(), "text/html".into())],
            &MatchOptions::default()
        ));
        // ignore_vary bypasses
        let opts = MatchOptions {
            ignore_vary: true,
            ..Default::default()
        };
        assert!(entry_matches(
            &entry,
            "https://api.com/data",
            "GET",
            &[("accept".into(), "text/html".into())],
            &opts
        ));
    }

    #[test]
    fn vary_star_never_matches() {
        let entry = CachedEntry {
            request_url: "https://example.com/".into(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            vary_headers: vec![("*".into(), "".into())],
            is_opaque: false,
        };
        assert!(!entry_matches(
            &entry,
            "https://example.com/",
            "GET",
            &[],
            &MatchOptions::default()
        ));
    }

    #[test]
    fn opaque_response_padded_quota() {
        let entry = CachedEntry {
            request_url: "https://cdn.com/lib.js".into(),
            request_method: "GET".into(),
            response_status: 0,
            response_status_text: "".into(),
            response_headers: vec![],
            response_body: vec![0; 1000],
            vary_headers: vec![],
            is_opaque: true,
        };
        let non_opaque = CachedEntry {
            is_opaque: false,
            ..entry.clone()
        };
        // Opaque should be ~3x normal
        assert!(entry.quota_size() > non_opaque.quota_size() * 2);
    }

    #[test]
    fn base64_roundtrip() {
        let data = b"Hello, World! \x00\xFF";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded);
        assert_eq!(decoded, data);
    }

    #[test]
    fn strip_query_test() {
        assert_eq!(strip_query("https://example.com/page?v=1"), "https://example.com/page");
        assert_eq!(strip_query("https://example.com/page"), "https://example.com/page");
    }
}
