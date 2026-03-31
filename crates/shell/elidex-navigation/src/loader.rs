//! Document loader — fetches a URL and produces a `LoadedDocument`.
//!
//! Coordinates HTTP fetch, charset detection, HTML parsing, and
//! sub-resource extraction (CSS, JS, images) into a single pipeline.

use std::fmt;
use std::sync::Arc;

use std::collections::HashMap;

use elidex_css::{Origin, Stylesheet};
use elidex_dom_compat::parse_compat_stylesheet;
use elidex_ecs::{BackgroundImages, EcsDom, Entity, ImageData};
use elidex_net::broker::NetworkHandle;
use elidex_net::NetError;
use elidex_plugin::background::BackgroundImage;
use elidex_plugin::ComputedStyle;

use crate::resource::{
    extract_image_sources, extract_script_sources, extract_style_sources, ScriptSource, StyleSource,
};

/// A fully loaded document with parsed DOM and resolved sub-resources.
pub struct LoadedDocument {
    /// The ECS DOM tree.
    pub dom: EcsDom,
    /// The document root entity.
    pub document: Entity,
    /// Parsed stylesheets (`<style>` inline + external CSS).
    pub stylesheets: Vec<Stylesheet>,
    /// Resolved scripts (inline source + fetched external source), in document order.
    pub scripts: Vec<ResolvedScript>,
    /// The final URL of the document (after redirects).
    pub url: url::Url,
    /// HTTP response headers (for CSP frame-ancestors, X-Frame-Options, Permissions-Policy).
    /// Multiple values for the same header name are preserved as separate entries.
    pub response_headers: HashMap<String, Vec<String>>,
}

/// A script ready for execution.
#[derive(Debug)]
pub struct ResolvedScript {
    /// The JavaScript source code.
    pub source: String,
    /// The entity of the `<script>` element in the DOM.
    pub entity: Entity,
}

/// Error returned by [`load_document`].
#[derive(Debug)]
pub enum LoadError {
    /// A network error occurred while fetching the document or a sub-resource.
    Network(NetError),
    /// The URL is invalid or unsupported.
    InvalidUrl(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::InvalidUrl(msg) => write!(f, "invalid URL: {msg}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Network(e) => Some(e),
            Self::InvalidUrl(_) => None,
        }
    }
}

impl From<NetError> for LoadError {
    fn from(err: NetError) -> Self {
        Self::Network(err)
    }
}

/// Extract a charset from a `Content-Type` header value.
///
/// Iterates all `;`-separated parameters for a case-insensitive `charset=`
/// prefix (RFC 7231: parameter names are case-insensitive).
/// Returns `None` if no charset parameter is found.
///
/// Note: A similar function exists in `elidex_html_parser::charset::extract_charset_from_content_type`
/// for `<meta>` tag prescan. That version handles quote styles differently and operates
/// on raw byte prescan results. Keeping both avoids a cross-crate dependency for a
/// small function with slightly different input contexts (HTTP header vs. HTML attribute).
fn extract_charset(content_type: &str) -> Option<String> {
    let prefix = "charset=";
    for part in content_type.split(';').skip(1) {
        let trimmed = part.trim();
        if trimmed.len() >= prefix.len() && trimmed[..prefix.len()].eq_ignore_ascii_case(prefix) {
            let value = &trimmed[prefix.len()..];
            return Some(value.trim_matches('"').trim().to_string());
        }
    }
    None
}

/// Create a GET request with no headers or body for the given URL.
fn make_get_request(url: url::Url) -> elidex_net::Request {
    elidex_net::Request {
        method: "GET".to_string(),
        url,
        headers: Vec::new(),
        body: bytes::Bytes::new(),
    }
}

/// Load a document from a URL.
///
/// 1. Fetches the HTML via `FetchHandle::send_blocking()`.
/// 2. Detects charset from the `Content-Type` header.
/// 3. Parses the HTML with `parse_tolerant()`.
/// 4. Extracts and fetches external stylesheets.
/// 5. Extracts and fetches external scripts.
/// 6. Extracts, fetches, and decodes images (`<img src="...">`).
///
/// Sub-resource fetch errors are logged and skipped (the page still loads).
///
/// When `request` is `Some`, that request is sent instead of a default GET.
/// This enables POST form submissions.
pub fn load_document(
    url: &url::Url,
    network_handle: &NetworkHandle,
    request: Option<elidex_net::Request>,
) -> Result<LoadedDocument, LoadError> {
    // 1. Fetch the HTML document.
    let req = request.unwrap_or_else(|| make_get_request(url.clone()));
    let response = network_handle
        .fetch_blocking(req)
        .map_err(|e| NetError::new(elidex_net::NetErrorKind::Other, e))?;
    if !(200..300).contains(&response.status) {
        tracing::warn!("HTTP {}: {}", response.status, url);
    }

    // 2. Extract charset from Content-Type header.
    let charset_hint = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, v)| extract_charset(v));

    // Collect response headers for security policy enforcement (CSP, X-Frame-Options).
    // Preserve multiple values per header name for correct per-value evaluation.
    let mut response_headers: HashMap<String, Vec<String>> = HashMap::new();
    for (k, v) in &response.headers {
        let key = k.to_ascii_lowercase();
        response_headers.entry(key).or_default().push(v.clone());
    }

    // 3. Parse the HTML.
    let parse_result = elidex_html_parser::parse_tolerant(&response.body, charset_hint.as_deref());
    for err in &parse_result.errors {
        tracing::warn!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    // 4. Extract and fetch stylesheets.
    let style_sources = extract_style_sources(&dom, document);
    let mut stylesheets = Vec::new();
    for source in &style_sources {
        match source {
            StyleSource::Inline(css) => {
                stylesheets.push(parse_compat_stylesheet(css, Origin::Author));
            }
            StyleSource::External(href) => {
                match resolve_and_fetch_text(&response.url, href, network_handle) {
                    Ok(css_text) => {
                        stylesheets.push(parse_compat_stylesheet(&css_text, Origin::Author));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch stylesheet {href}: {e}");
                    }
                }
            }
        }
    }

    // 5. Extract and fetch scripts.
    let script_sources = extract_script_sources(&dom, document);
    let mut scripts = Vec::new();
    for source in script_sources {
        match source {
            ScriptSource::Inline { source, entity } => {
                scripts.push(ResolvedScript { source, entity });
            }
            ScriptSource::External { src, entity } => {
                match resolve_and_fetch_text(&response.url, &src, network_handle) {
                    Ok(js_text) => {
                        scripts.push(ResolvedScript {
                            source: js_text,
                            entity,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch script {src}: {e}");
                    }
                }
            }
        }
    }

    // 6. Extract and fetch images.
    let image_sources = extract_image_sources(&dom, document);
    for source in &image_sources {
        match resolve_and_fetch_binary(&response.url, &source.src, network_handle) {
            Ok(data) => match decode_image(&data) {
                Ok(image_data) => {
                    let _ = dom.world_mut().insert_one(source.entity, image_data);
                }
                Err(e) => {
                    tracing::warn!("Failed to decode image {}: {e}", source.src);
                }
            },
            Err(e) => {
                tracing::warn!("Failed to fetch image {}: {e}", source.src);
            }
        }
    }

    Ok(LoadedDocument {
        dom,
        document,
        stylesheets,
        scripts,
        url: response.url,
        response_headers,
    })
}

/// Resolve a potentially relative URL against a base and fetch the response.
///
/// Shared by [`resolve_and_fetch_binary`] and [`resolve_and_fetch_text`].
/// Supports `http:`, `https:`, and `data:` schemes.
fn resolve_and_fetch(
    base: &url::Url,
    href: &str,
    network_handle: &NetworkHandle,
) -> Result<elidex_net::Response, LoadError> {
    let resolved = base
        .join(href)
        .map_err(|e| LoadError::InvalidUrl(format!("{href}: {e}")))?;
    if resolved.scheme() == "data" {
        let parsed = elidex_net::data_url::parse_data_url(&resolved)?;
        return Ok(elidex_net::Response {
            status: 200,
            headers: Vec::new(),
            body: parsed.body,
            url: resolved,
            version: elidex_net::HttpVersion::H1,
        });
    }
    let response = network_handle
        .fetch_blocking(make_get_request(resolved))
        .map_err(|e| NetError::new(elidex_net::NetErrorKind::Other, e))?;
    if !(200..300).contains(&response.status) {
        tracing::warn!("HTTP {}: {}", response.status, response.url);
    }
    Ok(response)
}

/// Resolve a potentially relative URL against a base and fetch its raw bytes.
fn resolve_and_fetch_binary(
    base: &url::Url,
    href: &str,
    network_handle: &NetworkHandle,
) -> Result<Vec<u8>, LoadError> {
    let response = resolve_and_fetch(base, href, network_handle)?;
    Ok(response.body.to_vec())
}

/// Decode image bytes into RGBA8 pixel data.
fn decode_image(bytes: &[u8]) -> Result<ImageData, image::ImageError> {
    let img = image::load_from_memory(bytes)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(ImageData {
        pixels: Arc::new(rgba.into_raw()),
        width,
        height,
    })
}

/// Resolve a potentially relative URL against a base and fetch its text content.
fn resolve_and_fetch_text(
    base: &url::Url,
    href: &str,
    network_handle: &NetworkHandle,
) -> Result<String, LoadError> {
    let response = resolve_and_fetch(base, href, network_handle)?;
    // L-10: Log non-UTF-8 sub-resources before lossy conversion.
    if std::str::from_utf8(&response.body).is_err() {
        tracing::debug!(
            "Non-UTF-8 response body for {}, using lossy conversion",
            response.url
        );
    }
    Ok(String::from_utf8_lossy(&response.body).into_owned())
}

/// Fetch a single image URL, using a cache to avoid duplicate requests.
///
/// Returns `Some(ImageData)` on success, `None` on failure (logged).
fn fetch_and_cache<S: std::hash::BuildHasher>(
    url_str: &str,
    base: &url::Url,
    network_handle: &NetworkHandle,
    cache: &mut HashMap<String, Arc<ImageData>, S>,
) -> Option<Arc<ImageData>> {
    if let Some(cached) = cache.get(url_str) {
        return Some(Arc::clone(cached));
    }
    match resolve_and_fetch_binary(base, url_str, network_handle) {
        Ok(data) => match decode_image(&data) {
            Ok(image_data) => {
                let arc = Arc::new(image_data);
                cache.insert(url_str.to_string(), Arc::clone(&arc));
                Some(arc)
            }
            Err(e) => {
                tracing::warn!("Failed to decode background image {url_str}: {e}");
                None
            }
        },
        Err(e) => {
            tracing::warn!("Failed to fetch background image {url_str}: {e}");
            None
        }
    }
}

/// Fetch and attach background images for all elements with `background_layers`.
///
/// Walks all entities with a `ComputedStyle`, checks for URL-based background
/// layers, fetches/decodes images, and inserts `BackgroundImages` components.
/// Uses a URL cache to avoid duplicate fetches (shared with `<img>` images).
pub fn fetch_background_images<S: std::hash::BuildHasher>(
    dom: &mut EcsDom,
    base_url: &url::Url,
    network_handle: &NetworkHandle,
    url_cache: &mut std::collections::HashMap<String, Arc<ImageData>, S>,
) {
    // Collect entities that need background images.
    let mut entities_with_bg: Vec<(Entity, Vec<BackgroundImage>)> = Vec::new();
    {
        let world = dom.world();
        for (entity, style) in &mut world.query::<(Entity, &ComputedStyle)>() {
            if let Some(ref layers) = style.background_layers {
                let images: Vec<BackgroundImage> = layers.iter().map(|l| l.image.clone()).collect();
                entities_with_bg.push((entity, images));
            }
        }
    }

    for (entity, images) in entities_with_bg {
        let has_url = images
            .iter()
            .any(|img| matches!(img, BackgroundImage::Url(_)));
        if !has_url {
            continue;
        }

        let layers: Vec<Option<Arc<ImageData>>> = images
            .iter()
            .map(|img| match img {
                BackgroundImage::Url(url_str) => {
                    fetch_and_cache(url_str, base_url, network_handle, url_cache)
                }
                _ => None,
            })
            .collect();

        let _ = dom
            .world_mut()
            .insert_one(entity, BackgroundImages { layers });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_charset_basic() {
        assert_eq!(
            extract_charset("text/html; charset=UTF-8"),
            Some("UTF-8".to_string())
        );
    }

    #[test]
    fn extract_charset_quoted() {
        assert_eq!(
            extract_charset("text/html; charset=\"ISO-8859-1\""),
            Some("ISO-8859-1".to_string())
        );
    }

    #[test]
    fn extract_charset_missing() {
        assert_eq!(extract_charset("text/html"), None);
    }

    #[test]
    fn extract_charset_uppercase() {
        assert_eq!(
            extract_charset("text/html; CHARSET=utf-8"),
            Some("utf-8".to_string())
        );
    }

    #[test]
    fn extract_charset_mixed_case() {
        assert_eq!(
            extract_charset("text/html; Charset=UTF-8"),
            Some("UTF-8".to_string())
        );
    }

    #[test]
    fn extract_charset_second_param() {
        assert_eq!(
            extract_charset("text/html; boundary=something; charset=UTF-8"),
            Some("UTF-8".to_string())
        );
    }

    #[test]
    fn load_error_display() {
        let err = LoadError::InvalidUrl("bad url".to_string());
        assert!(err.to_string().contains("bad url"));
    }

    #[test]
    fn load_error_from_net_error() {
        let net_err = NetError::new(elidex_net::NetErrorKind::Timeout, "timed out");
        let err: LoadError = net_err.into();
        assert!(matches!(err, LoadError::Network(_)));
    }

    // --- Image decode tests ---

    #[test]
    fn decode_minimal_png() {
        // Valid 1×1 white RGB PNG (69 bytes, generated with correct CRCs).
        let png_bytes: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // 8-bit RGB + CRC
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk (12 bytes)
            0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, // zlib compressed
            0x05, 0xFE, 0x02, 0xFE, 0x0D, 0xEF, 0x46, 0xB8, // + CRC
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82,
        ];
        let result = decode_image(png_bytes);
        assert!(result.is_ok(), "decode failed: {:?}", result.err());
        let img = result.unwrap();
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
        assert_eq!(img.pixels.len(), 4); // 1×1×4 (RGBA8)
    }

    #[test]
    fn decode_minimal_jpeg() {
        // Create a valid 1×1 JPEG using the image crate's encoder.
        let pixel = image::RgbImage::from_pixel(1, 1, image::Rgb([255, 255, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        pixel.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();

        let result = decode_image(buf.get_ref());
        assert!(result.is_ok(), "decode failed: {:?}", result.err());
        let img = result.unwrap();
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
        assert_eq!(img.pixels.len(), 4); // 1×1×4 (RGBA8)
    }

    #[test]
    fn decode_invalid_bytes_fails() {
        let result = decode_image(b"not an image");
        assert!(result.is_err());
    }

    #[test]
    fn decode_empty_bytes_fails() {
        let result = decode_image(b"");
        assert!(result.is_err());
    }
}
