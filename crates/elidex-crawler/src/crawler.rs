//! HTTP fetching and concurrent crawl orchestration.

use crate::analyzer;
use crate::robots;
use crate::sites::Site;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};

/// Maximum response body size (50 MB).
const MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024;

/// Maximum number of HTTP redirects to follow per request.
const MAX_REDIRECTS: usize = 5;

/// Maximum concurrent requests to a single host.
///
/// Note: this limits concurrency only, not request rate (requests/second).
/// A per-host rate limiter (e.g. token bucket) can be added if needed.
const PER_HOST_CONCURRENCY: usize = 2;

/// Maximum number of per-host semaphore entries to retain.
const MAX_HOST_SEMAPHORE_ENTRIES: usize = 10_000;

/// Configuration for the crawl operation.
pub struct CrawlConfig {
    /// Maximum number of concurrent requests across all hosts.
    pub concurrency: usize,
    /// Per-request timeout in seconds.
    pub timeout_secs: u64,
    /// Number of retry attempts after a failed request.
    pub retries: usize,
    /// User-Agent header sent with each request.
    pub user_agent: String,
}

/// The result of crawling a single site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteResult {
    pub url: String,
    pub category: String,
    pub language: String,
    pub status_code: Option<u16>,
    pub error: Option<String>,
    pub html_features: analyzer::html::HtmlFeatures,
    pub css_features: analyzer::css::CssFeatures,
    pub js_features: analyzer::js::JsFeatures,
    pub parser_errors: Vec<String>,
}

/// Result of a full crawl run, including statistics.
pub struct CrawlOutput {
    pub results: Vec<SiteResult>,
    /// Number of tasks that panicked during crawling.
    pub panicked: usize,
}

/// Crawl all sites concurrently, respecting global and per-host concurrency limits.
pub async fn crawl_sites(sites: &[Site], config: &CrawlConfig) -> anyhow::Result<CrawlOutput> {
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let host_semaphores: Arc<Mutex<HashMap<String, Arc<Semaphore>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let client = reqwest::Client::builder()
        .user_agent(&config.user_agent)
        .timeout(Duration::from_secs(config.timeout_secs))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                attempt.stop()
            } else if validate_url(attempt.url()).is_ok() {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()?;

    let mut handles = Vec::new();
    let user_agent = config.user_agent.clone();

    for site in sites {
        let sem = semaphore.clone();
        let host_sems = host_semaphores.clone();
        let client = client.clone();
        let site = site.clone();
        let retries = config.retries;
        let ua = user_agent.clone();

        let handle = tokio::spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                return error_result(&site, "global semaphore closed");
            };

            // Per-host rate limiting.
            let host_sem = {
                let host = reqwest::Url::parse(&site.url)
                    .ok()
                    .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
                    .unwrap_or_default();
                let mut map = host_sems.lock().await;
                // Evict stale entries when the map grows too large.
                if map.len() > MAX_HOST_SEMAPHORE_ENTRIES {
                    map.retain(|_, sem| Arc::strong_count(sem) > 1);
                }
                map.entry(host)
                    .or_insert_with(|| Arc::new(Semaphore::new(PER_HOST_CONCURRENCY)))
                    .clone()
            };
            let Ok(_host_permit) = host_sem.acquire().await else {
                return error_result(&site, "host semaphore closed");
            };

            crawl_one(&client, &site, retries, &ua).await
        });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(handles.len());
    let mut panicked = 0;
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::error!("Task panicked: {e}");
                panicked += 1;
            }
        }
    }
    Ok(CrawlOutput { results, panicked })
}

/// Create a `SiteResult` with default (empty) analysis fields.
fn base_result(site: &Site) -> SiteResult {
    SiteResult {
        url: site.url.clone(),
        category: site.category.clone(),
        language: site.language.clone(),
        status_code: None,
        error: None,
        html_features: analyzer::html::HtmlFeatures::default(),
        css_features: analyzer::css::CssFeatures::default(),
        js_features: analyzer::js::JsFeatures::default(),
        parser_errors: Vec::new(),
    }
}

/// Create a `SiteResult` representing a failed crawl.
fn error_result(site: &Site, msg: &str) -> SiteResult {
    SiteResult {
        error: Some(msg.to_string()),
        ..base_result(site)
    }
}

async fn crawl_one(
    client: &reqwest::Client,
    site: &Site,
    retries: usize,
    user_agent: &str,
) -> SiteResult {
    let mut last_error = None;

    for attempt in 0..=retries {
        if attempt > 0 {
            // Exponential backoff: 1s, 2s, 4s, ... capped at 60s.
            #[allow(clippy::cast_possible_truncation)]
            let shift = (attempt - 1).min(63) as u32; // max 63, safe for u32
            let delay = Duration::from_secs(1u64.checked_shl(shift).unwrap_or(60).min(60));
            tracing::debug!("Retry {attempt} for {} (after {delay:?})", site.url);
            tokio::time::sleep(delay).await;
        }

        match fetch_and_analyze(client, site, user_agent).await {
            Ok(result) => return result,
            Err(e) => {
                tracing::warn!("Error fetching {}: {e:#}", site.url);
                last_error = Some(format!("{e:#}"));
            }
        }
    }

    // WWW fallback: if all retries failed with a DNS or TLS error on a bare domain,
    // try the www-prefixed URL once.
    if let Some(ref err_msg) = last_error {
        if is_bare_domain_error(err_msg) {
            if let Some(www_url) = add_www_prefix(&site.url) {
                tracing::info!(
                    "Bare domain error for {}; trying www fallback: {www_url}",
                    site.url
                );
                let mut www_site = site.clone();
                www_site.url = www_url;
                if let Ok(mut result) = fetch_and_analyze(client, &www_site, user_agent).await {
                    // Keep the original URL in the result for consistency.
                    result.url.clone_from(&site.url);
                    return result;
                }
            }
        }
    }

    let mut result = error_result(site, last_error.as_deref().unwrap_or("unknown error"));
    result.error = last_error;
    result
}

async fn fetch_and_analyze(
    client: &reqwest::Client,
    site: &Site,
    user_agent: &str,
) -> anyhow::Result<SiteResult> {
    // Validate URL before fetching.
    let parsed = reqwest::Url::parse(&site.url)?;
    validate_url(&parsed)?;

    // Check robots.txt.
    if let Some(rules) = robots::fetch_robots(client, &site.url).await {
        let path = parsed.path();
        if !rules.is_allowed(user_agent, path) {
            anyhow::bail!("blocked by robots.txt: {}", site.url);
        }
        // Respect crawl-delay.
        if let Some(delay) = rules.crawl_delay(user_agent) {
            let capped = delay.min(Duration::from_secs(10));
            tokio::time::sleep(capped).await;
        }
    }

    let response = client.get(&site.url).send().await?;

    // Validate final URL after redirects.
    validate_url(response.url())?;

    let status = response.status().as_u16();

    // Validate Content-Type is HTML-like (compare MIME type only, ignoring charset params).
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mime = content_type.split(';').next().unwrap_or("").trim();
    if !mime.is_empty()
        && mime != "text/html"
        && mime != "application/xhtml+xml"
        && mime != "text/xml"
    {
        anyhow::bail!("non-HTML content type: {mime}");
    }

    // Enforce response size limit.
    let content_length = response
        .content_length()
        .unwrap_or(0)
        .try_into()
        .unwrap_or(usize::MAX);
    if content_length > MAX_RESPONSE_BYTES {
        anyhow::bail!(
            "response too large: {content_length} bytes (limit: {MAX_RESPONSE_BYTES} bytes)"
        );
    }

    let body = read_body_limited(response).await?;

    let (html_features, parser_errors) = analyzer::html::analyze(&body);
    let css_features = analyzer::css::analyze(&body);
    let js_features = analyzer::js::analyze(&body);

    Ok(SiteResult {
        status_code: Some(status),
        html_features,
        css_features,
        js_features,
        parser_errors,
        ..base_result(site)
    })
}

/// Read the response body with a streaming size limit.
///
/// Reads the body in chunks, aborting as soon as the accumulated size exceeds
/// [`MAX_RESPONSE_BYTES`]. This prevents OOM from chunked transfer-encoded
/// responses that lack a `Content-Length` header.
async fn read_body_limited(mut response: reqwest::Response) -> anyhow::Result<String> {
    let mut buf = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        buf.extend_from_slice(&chunk);
        if buf.len() > MAX_RESPONSE_BYTES {
            anyhow::bail!("response body too large (limit: {MAX_RESPONSE_BYTES} bytes)");
        }
    }
    match String::from_utf8(buf) {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::warn!("Response contains invalid UTF-8, using lossy conversion");
            Ok(String::from_utf8_lossy(e.as_bytes()).into_owned())
        }
    }
}

/// Validate that a URL doesn't point to a private/internal address.
///
/// **Known limitation (Phase 0):** This validates the *hostname string*, not
/// the resolved IP address. A DNS name like `attacker.com` that resolves to
/// `127.0.0.1` (DNS rebinding) will pass validation. Phase 1 should add a
/// custom [`reqwest::dns::Resolve`] implementation that checks resolved IPs
/// at the socket level before connecting.
pub(crate) fn validate_url(url: &reqwest::Url) -> anyhow::Result<()> {
    // Only allow http/https schemes.
    match url.scheme() {
        "http" | "https" => {}
        scheme => anyhow::bail!("unsupported URL scheme: {scheme}"),
    }

    // Check for private/loopback hostnames.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if let Some(host) = url.host_str() {
        let lower = host.to_ascii_lowercase();
        if lower == "localhost"
            || lower.ends_with(".local")
            || lower.ends_with(".internal")
            || lower == "::1"
        {
            anyhow::bail!("blocked private host: {host}");
        }

        // Parse as IP and check for private ranges.
        if let Ok(ip) = host.parse::<IpAddr>() {
            if is_private_ip(ip) {
                anyhow::bail!("blocked private IP: {ip}");
            }
        }
    } else {
        anyhow::bail!("URL has no host");
    }

    Ok(())
}

/// Check if an IP address is in a private/reserved range.
fn is_private_ip(ip: IpAddr) -> bool {
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
        }
    }
}

/// Check if an error indicates that a bare domain cannot connect but `www.` might work.
///
/// Matches DNS resolution failures and TLS hostname mismatches, both of which
/// commonly occur when a bare domain lacks proper DNS/certificate configuration
/// but the `www.` subdomain works.
fn is_bare_domain_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("dns error") || lower.contains("host name mismatch")
}

/// Add a `www.` prefix to a URL's host if it doesn't already have one.
///
/// Returns `None` if the host already starts with `www.` or if parsing fails.
fn add_www_prefix(url: &str) -> Option<String> {
    let mut parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if host.to_ascii_lowercase().starts_with("www.") {
        return None;
    }
    let new_host = format!("www.{host}");
    parsed.set_host(Some(&new_host)).ok()?;
    Some(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ips_blocked() {
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(is_private_ip("169.254.1.1".parse().unwrap()));
        assert!(is_private_ip("::1".parse().unwrap()));
    }

    #[test]
    fn public_ips_allowed() {
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip("1.1.1.1".parse().unwrap()));
        assert!(!is_private_ip("93.184.216.34".parse().unwrap()));
    }

    #[test]
    fn ipv4_mapped_ipv6_blocked() {
        // ::ffff:127.0.0.1 should be blocked (maps to loopback).
        assert!(is_private_ip("::ffff:127.0.0.1".parse().unwrap()));
        // ::ffff:10.0.0.1 should be blocked (maps to private).
        assert!(is_private_ip("::ffff:10.0.0.1".parse().unwrap()));
        // ::ffff:192.168.1.1 should be blocked (maps to private).
        assert!(is_private_ip("::ffff:192.168.1.1".parse().unwrap()));
        // ::ffff:8.8.8.8 should be allowed (maps to public).
        assert!(!is_private_ip("::ffff:8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn validate_url_rejects_private() {
        let url = reqwest::Url::parse("http://127.0.0.1/secret").unwrap();
        assert!(validate_url(&url).is_err());

        let url = reqwest::Url::parse("http://localhost/admin").unwrap();
        assert!(validate_url(&url).is_err());

        let url = reqwest::Url::parse("ftp://example.com/file").unwrap();
        assert!(validate_url(&url).is_err());
    }

    #[test]
    fn validate_url_allows_public() {
        let url = reqwest::Url::parse("https://example.com/page").unwrap();
        assert!(validate_url(&url).is_ok());

        let url = reqwest::Url::parse("http://93.184.216.34/").unwrap();
        assert!(validate_url(&url).is_ok());
    }

    #[test]
    fn is_bare_domain_error_detects_dns_failures() {
        assert!(is_bare_domain_error(
            "error sending request for url: dns error: failed to lookup address information"
        ));
        assert!(is_bare_domain_error("DNS error: no such host"));
        assert!(is_bare_domain_error("Dns Error in chain"));
    }

    #[test]
    fn is_bare_domain_error_detects_tls_mismatch() {
        assert!(is_bare_domain_error(
            "error sending request: Host name mismatch: CertNotValidForName"
        ));
        assert!(is_bare_domain_error("host name mismatch"));
        assert!(is_bare_domain_error("HOST NAME MISMATCH in TLS handshake"));
    }

    #[test]
    fn is_bare_domain_error_rejects_unrelated() {
        assert!(!is_bare_domain_error("connection refused"));
        assert!(!is_bare_domain_error("timeout waiting for response"));
        assert!(!is_bare_domain_error("blocked by robots.txt"));
        assert!(!is_bare_domain_error(""));
    }

    #[test]
    fn add_www_prefix_bare_domain() {
        let result = add_www_prefix("https://nhk.or.jp/news").unwrap();
        assert_eq!(result, "https://www.nhk.or.jp/news");
    }

    #[test]
    fn add_www_prefix_already_www() {
        assert!(add_www_prefix("https://www.example.com/page").is_none());
    }

    #[test]
    fn add_www_prefix_preserves_scheme_and_port() {
        let result = add_www_prefix("http://example.com:8080/path?q=1").unwrap();
        assert_eq!(result, "http://www.example.com:8080/path?q=1");
    }

    #[test]
    fn add_www_prefix_invalid_url() {
        assert!(add_www_prefix("not a url").is_none());
    }
}
