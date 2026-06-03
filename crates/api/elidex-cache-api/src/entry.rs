/// Response type (Fetch spec §3.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Basic,
    Cors,
    Default,
    Error,
    Opaque,
    OpaqueRedirect,
}

impl ResponseType {
    /// Parse from string (case-insensitive). Unknown values default to `Basic`.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "cors" => Self::Cors,
            "error" => Self::Error,
            "opaque" => Self::Opaque,
            "opaqueredirect" => Self::OpaqueRedirect,
            "default" => Self::Default,
            _ => Self::Basic,
        }
    }

    /// Serialize to the spec-defined string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Cors => "cors",
            Self::Default => "default",
            Self::Error => "error",
            Self::Opaque => "opaque",
            Self::OpaqueRedirect => "opaqueredirect",
        }
    }
}

impl std::fmt::Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A cached request/response pair stored in the Cache API.
#[derive(Debug, Clone)]
pub struct CachedEntry {
    /// Request URL.
    pub request_url: String,
    /// Request HTTP method.
    pub request_method: String,
    /// Request headers as (name, value) pairs (for `cache.keys()` returning Request objects).
    pub request_headers: Vec<(String, String)>,
    /// Response HTTP status code.
    pub response_status: u16,
    /// Response HTTP status text.
    pub response_status_text: String,
    /// Response headers as (name, value) pairs.
    pub response_headers: Vec<(String, String)>,
    /// Response body bytes.
    pub response_body: Vec<u8>,
    /// Redirect chain URLs (Fetch spec §3.1.4). First = original, last = final.
    pub response_url_list: Vec<String>,
    /// Response type (Fetch spec §3.1.1).
    pub response_type: ResponseType,
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
            "request_headers": self.request_headers,
            "response_status": self.response_status,
            "response_status_text": self.response_status_text,
            "response_headers": self.response_headers,
            "response_body": base64_encode(&self.response_body),
            "response_url_list": self.response_url_list,
            "response_type": self.response_type.as_str(),
            "vary_headers": self.vary_headers,
            "is_opaque": self.is_opaque,
        })
    }

    fn from_json(json: &serde_json::Value) -> Option<Self> {
        Some(Self {
            request_url: json.get("request_url")?.as_str()?.to_owned(),
            request_method: json.get("request_method")?.as_str()?.to_owned(),
            request_headers: parse_header_pairs(json.get("request_headers")),
            response_status: u16::try_from(json.get("response_status")?.as_u64()?).ok()?,
            response_status_text: json
                .get("response_status_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            response_headers: parse_header_pairs(json.get("response_headers")),
            response_body: base64_decode(json.get("response_body")?.as_str()?),
            response_url_list: json
                .get("response_url_list")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            response_type: ResponseType::from_str_lossy(
                json.get("response_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("basic"),
            ),
            vary_headers: parse_header_pairs(json.get("vary_headers")),
            is_opaque: json
                .get("is_opaque")
                .and_then(serde_json::Value::as_bool)
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
            let request_value = join_header_values(request_headers, header_name);
            if cached_value != &request_value {
                return false;
            }
        }
    }

    true
}

/// Combine every request header field-line whose name matches `name`
/// (ASCII-case-insensitively) into a single value joined by `", "`,
/// matching Fetch `Headers.get` semantics.  Multiple same-name lines (e.g.
/// built via `Headers.append`) are equivalent to one comma-joined field, so
/// Vary selection must compare the combined value, not just the first line.
/// No match yields the empty string (parity with an absent header).
fn join_header_values(headers: &[(String, String)], name: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case(name) {
            if !first {
                out.push_str(", ");
            }
            out.push_str(v);
            first = false;
        }
    }
    out
}

/// Derive a cached entry's `vary_headers` match key from a response's
/// `Vary` header (WHATWG Service Workers §5.4.5 "Cache.put").
///
/// For each header name listed in the response's `Vary` header, captures
/// the request's value for that header — the `(name, request-value)` pairs
/// that [`entry_matches`] later compares a new request against.  `Vary: *`
/// is the §5.4.5 put rejection: returns [`crate::CacheError::Invalid`] so
/// the caller can surface a `TypeError`.  This is the production half of
/// the Vary algorithm whose consumption half is `entry_matches`, kept in
/// the same crate so both halves stay together.
pub fn compute_vary_key(
    response_headers: &[(String, String)],
    request_headers: &[(String, String)],
) -> Result<Vec<(String, String)>, crate::CacheError> {
    let mut out = Vec::new();
    for (name, value) in response_headers {
        if !name.eq_ignore_ascii_case("vary") {
            continue;
        }
        for token in value.split(',') {
            let token = token.trim();
            if token == "*" {
                return Err(crate::CacheError::Invalid(
                    "a response with 'Vary: *' cannot be cached".to_owned(),
                ));
            }
            if token.is_empty() {
                continue;
            }
            let lower = token.to_ascii_lowercase();
            let req_value = join_header_values(request_headers, &lower);
            out.push((lower, req_value));
        }
    }
    Ok(out)
}

/// Parse a JSON array of [name, value] pairs into `Vec<(String, String)>`.
fn parse_header_pairs(val: Option<&serde_json::Value>) -> Vec<(String, String)> {
    val.and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|pair| {
                    let a = pair.as_array()?;
                    Some((
                        a.first()?.as_str()?.to_owned(),
                        a.get(1)?.as_str()?.to_owned(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

use base64::{engine::general_purpose::STANDARD, Engine};

fn base64_encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Vec<u8> {
    STANDARD.decode(s).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_roundtrip() {
        let entry = CachedEntry {
            request_url: "https://example.com/page".into(),
            request_method: "GET".into(),
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![("content-type".into(), "text/html".into())],
            response_body: b"<h1>Hello</h1>".to_vec(),
            response_url_list: vec![],
            response_type: ResponseType::Basic,
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
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            response_url_list: vec![],
            response_type: ResponseType::Basic,
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
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            response_url_list: vec![],
            response_type: ResponseType::Basic,
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
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            response_url_list: vec![],
            response_type: ResponseType::Basic,
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
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            response_url_list: vec![],
            response_type: ResponseType::Basic,
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
    fn vary_key_joins_multivalued_request_header() {
        // Fetch `Headers.get` joins same-name field-lines with ", "; Vary
        // selection compares the combined value, not just the first line.
        let resp = vec![("vary".to_string(), "accept".to_string())];
        let req_put = vec![
            ("accept".to_string(), "a".to_string()),
            ("accept".to_string(), "b".to_string()),
        ];
        let key = compute_vary_key(&resp, &req_put).unwrap();
        assert_eq!(key, vec![("accept".to_string(), "a, b".to_string())]);

        let entry = CachedEntry {
            request_url: "https://e.com/".into(),
            request_method: "GET".into(),
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            response_url_list: vec![],
            response_type: ResponseType::Basic,
            vary_headers: key,
            is_opaque: false,
        };
        // A request whose *combined* Accept differs must NOT match, even
        // though its first Accept line ("a") equals the stored key's first.
        let req_diff = vec![
            ("accept".to_string(), "a".to_string()),
            ("accept".to_string(), "c".to_string()),
        ];
        assert!(!entry_matches(
            &entry,
            "https://e.com/",
            "GET",
            &req_diff,
            &MatchOptions::default()
        ));
        // The same combined value DOES match.
        assert!(entry_matches(
            &entry,
            "https://e.com/",
            "GET",
            &req_put,
            &MatchOptions::default()
        ));
    }

    #[test]
    fn vary_star_never_matches() {
        let entry = CachedEntry {
            request_url: "https://example.com/".into(),
            request_method: "GET".into(),
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: vec![],
            response_url_list: vec![],
            response_type: ResponseType::Basic,
            vary_headers: vec![("*".into(), String::new())],
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
            request_headers: vec![],
            response_status: 0,
            response_status_text: String::new(),
            response_headers: vec![],
            response_body: vec![0; 1000],
            response_url_list: vec![],
            response_type: ResponseType::Opaque,
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
    fn compute_vary_key_captures_request_values() {
        let resp = vec![("vary".to_owned(), "Accept, Accept-Language".to_owned())];
        let req = vec![
            ("accept".to_owned(), "application/json".to_owned()),
            ("accept-language".to_owned(), "en".to_owned()),
        ];
        let key = compute_vary_key(&resp, &req).unwrap();
        assert_eq!(
            key,
            vec![
                ("accept".to_owned(), "application/json".to_owned()),
                ("accept-language".to_owned(), "en".to_owned()),
            ]
        );
    }

    #[test]
    fn compute_vary_key_missing_request_header_is_empty_value() {
        let resp = vec![("Vary".to_owned(), "Accept".to_owned())];
        let key = compute_vary_key(&resp, &[]).unwrap();
        assert_eq!(key, vec![("accept".to_owned(), String::new())]);
    }

    #[test]
    fn compute_vary_key_star_is_rejected() {
        let resp = vec![("vary".to_owned(), "*".to_owned())];
        assert!(matches!(
            compute_vary_key(&resp, &[]),
            Err(crate::CacheError::Invalid(_))
        ));
    }

    #[test]
    fn compute_vary_key_no_vary_header_is_empty() {
        let resp = vec![("content-type".to_owned(), "text/html".to_owned())];
        assert!(compute_vary_key(&resp, &[]).unwrap().is_empty());
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
        assert_eq!(
            strip_query("https://example.com/page?v=1"),
            "https://example.com/page"
        );
        assert_eq!(
            strip_query("https://example.com/page"),
            "https://example.com/page"
        );
    }
}
