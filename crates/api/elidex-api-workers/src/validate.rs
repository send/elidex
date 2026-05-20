//! Engine-independent worker-script validation + URL resolution.
//!
//! Pure algorithm extracted from the binding layer per the elidex Layering
//! mandate: URL same-origin/`blob:`/`data:` resolution (WHATWG HTML §10.2.6.3)
//! and worker-script MIME/status validation (WHATWG HTML §10.2.4, invoked by
//! "fetch a classic worker script"). No JS-runtime / VM / boa types — the VM
//! binding layer calls these and marshals the [`WorkerScriptError`] into the
//! appropriate `TypeError` / `SecurityError` / `NetworkError`.

use url::Url;

/// JavaScript MIME types accepted for worker scripts (WHATWG HTML §10.2.4 —
/// "fetch a classic worker script", essence check against the JavaScript MIME
/// type set defined in MIME Sniffing §4.6).
pub const JS_MIME_TYPES: &[&str] = &[
    "text/javascript",
    "application/javascript",
    "application/x-javascript",
];

/// Failure reasons for worker-script setup. The binding layer maps each to the
/// spec-mandated DOM exception (URL/origin → `SecurityError`/`TypeError`,
/// fetch → `NetworkError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerScriptError {
    /// `new Worker("")` — the script URL argument was empty.
    EmptyUrl,
    /// The script URL failed to parse / resolve against the base URL.
    InvalidUrl(String),
    /// The resolved URL is not same-origin with the base URL (`SecurityError`).
    NotSameOrigin {
        /// The resolved (target) URL.
        resolved: String,
        /// The base URL the worker was created from.
        base: String,
    },
    /// `{ type: "module" }` — module workers are not yet supported.
    UnsupportedType,
    /// `{ credentials: ... }` with a value outside the allowed set.
    InvalidCredentials(String),
    /// The fetched script had a non-JavaScript MIME type.
    InvalidMimeType(String),
    /// The script fetch returned a non-2xx HTTP status.
    BadStatus {
        /// The HTTP status code returned.
        status: u16,
        /// The script URL that was fetched.
        url: String,
    },
}

impl std::fmt::Display for WorkerScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyUrl => write!(f, "Worker: script URL is required"),
            Self::InvalidUrl(e) => write!(f, "Worker: invalid URL: {e}"),
            Self::NotSameOrigin { resolved, base } => write!(
                f,
                "SecurityError: Worker script URL {resolved} is not same-origin with {base}"
            ),
            Self::UnsupportedType => write!(f, "Worker: type 'module' is not supported"),
            Self::InvalidCredentials(c) => write!(f, "Worker: invalid credentials value: {c}"),
            Self::InvalidMimeType(m) => write!(f, "Invalid MIME type for worker script: {m}"),
            Self::BadStatus { status, url } => {
                write!(
                    f,
                    "Worker script fetch failed with status {status} for {url}"
                )
            }
        }
    }
}

impl std::error::Error for WorkerScriptError {}

/// Resolve a worker script URL against a base URL and enforce the same-origin
/// restriction (WHATWG HTML §10.2.6.3 — the `Worker(scriptURL, options)`
/// constructor steps).
///
/// `data:` URLs are always permitted; `blob:` URLs carry their origin in the
/// path and are compared against `base`. Any other scheme must match the base
/// origin exactly.
///
/// # Errors
/// [`WorkerScriptError::EmptyUrl`] when `url_str` is empty,
/// [`WorkerScriptError::InvalidUrl`] when it fails to resolve, and
/// [`WorkerScriptError::NotSameOrigin`] when the resolved URL is cross-origin.
pub fn resolve_worker_script_url(base: &Url, url_str: &str) -> Result<Url, WorkerScriptError> {
    if url_str.is_empty() {
        return Err(WorkerScriptError::EmptyUrl);
    }

    let resolved = base
        .join(url_str)
        .map_err(|e| WorkerScriptError::InvalidUrl(e.to_string()))?;

    // `data:` scheme is same-origin by fiat; `blob:` embeds its origin in the
    // path (e.g. `blob:https://example.com/uuid`); everything else compares the
    // resolved origin against the base origin.
    let is_same_origin = if resolved.scheme() == "data" {
        true
    } else if resolved.scheme() == "blob" {
        Url::parse(resolved.path()).is_ok_and(|inner| inner.origin() == base.origin())
    } else {
        base.origin() == resolved.origin()
    };

    if !is_same_origin {
        return Err(WorkerScriptError::NotSameOrigin {
            resolved: resolved.to_string(),
            base: base.to_string(),
        });
    }

    Ok(resolved)
}

/// Validate the `type` worker option. Only classic workers are supported;
/// `{ type: "module" }` is rejected (WHATWG HTML §10.2.6.3).
///
/// # Errors
/// [`WorkerScriptError::UnsupportedType`] when `type_opt` is `"module"`.
pub fn validate_worker_type(type_opt: Option<&str>) -> Result<(), WorkerScriptError> {
    if type_opt == Some("module") {
        return Err(WorkerScriptError::UnsupportedType);
    }
    Ok(())
}

/// Validate the `credentials` worker option, returning the effective value
/// (default `"same-origin"`). Allowed: `"omit"`, `"same-origin"`, `"include"`
/// (WHATWG HTML §10.2.6.3).
///
/// # Errors
/// [`WorkerScriptError::InvalidCredentials`] for any other value.
pub fn validate_credentials(cred_opt: Option<&str>) -> Result<String, WorkerScriptError> {
    let cred = cred_opt.unwrap_or("same-origin");
    if !["omit", "same-origin", "include"].contains(&cred) {
        return Err(WorkerScriptError::InvalidCredentials(cred.to_string()));
    }
    Ok(cred.to_string())
}

/// Validate a fetched worker-script response and decode its body to source
/// text (WHATWG HTML §10.2.4 — "fetch a classic worker script": the response
/// must be a 2xx and, when a `Content-Type` is present, carry a JavaScript MIME
/// type). The same algorithm validates `importScripts(...)` responses.
///
/// `content_type` is the raw `Content-Type` header value (parameters such as
/// `; charset=utf-8` are stripped before the essence comparison).
///
/// # Errors
/// [`WorkerScriptError::InvalidMimeType`] for a non-JavaScript essence and
/// [`WorkerScriptError::BadStatus`] for a non-2xx status.
pub fn validate_worker_script_response(
    content_type: Option<&str>,
    status: u16,
    body: &[u8],
    url: &Url,
) -> Result<String, WorkerScriptError> {
    if let Some(ct) = content_type {
        let mime = ct
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !mime.is_empty() && !JS_MIME_TYPES.contains(&mime.as_str()) {
            return Err(WorkerScriptError::InvalidMimeType(mime));
        }
    }

    if !(200..300).contains(&status) {
        return Err(WorkerScriptError::BadStatus {
            status,
            url: url.to_string(),
        });
    }

    Ok(String::from_utf8_lossy(body).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://example.com/app/index.html").expect("valid base")
    }

    #[test]
    fn resolves_relative_same_origin() {
        let resolved = resolve_worker_script_url(&base(), "worker.js").expect("same-origin");
        assert_eq!(resolved.as_str(), "https://example.com/app/worker.js");
    }

    #[test]
    fn rejects_empty_url() {
        assert_eq!(
            resolve_worker_script_url(&base(), ""),
            Err(WorkerScriptError::EmptyUrl)
        );
    }

    #[test]
    fn rejects_cross_origin() {
        let err = resolve_worker_script_url(&base(), "https://evil.example/w.js")
            .expect_err("cross-origin");
        assert!(matches!(err, WorkerScriptError::NotSameOrigin { .. }));
    }

    #[test]
    fn allows_data_url() {
        let resolved =
            resolve_worker_script_url(&base(), "data:text/javascript,close()").expect("data: ok");
        assert_eq!(resolved.scheme(), "data");
    }

    #[test]
    fn blob_same_origin_allowed() {
        let resolved = resolve_worker_script_url(&base(), "blob:https://example.com/uuid-123")
            .expect("same-origin blob");
        assert_eq!(resolved.scheme(), "blob");
    }

    #[test]
    fn blob_cross_origin_rejected() {
        let err = resolve_worker_script_url(&base(), "blob:https://evil.example/uuid-123")
            .expect_err("cross-origin blob");
        assert!(matches!(err, WorkerScriptError::NotSameOrigin { .. }));
    }

    #[test]
    fn module_type_rejected() {
        assert_eq!(
            validate_worker_type(Some("module")),
            Err(WorkerScriptError::UnsupportedType)
        );
        assert_eq!(validate_worker_type(Some("classic")), Ok(()));
        assert_eq!(validate_worker_type(None), Ok(()));
    }

    #[test]
    fn credentials_validated() {
        assert_eq!(validate_credentials(None).as_deref(), Ok("same-origin"));
        assert_eq!(validate_credentials(Some("omit")).as_deref(), Ok("omit"));
        assert!(validate_credentials(Some("bogus")).is_err());
    }

    #[test]
    fn response_mime_and_status() {
        let url = base();
        assert_eq!(
            validate_worker_script_response(Some("text/javascript"), 200, b"close()", &url),
            Ok("close()".to_string())
        );
        // charset parameter stripped before essence check.
        assert!(validate_worker_script_response(
            Some("application/javascript; charset=utf-8"),
            200,
            b"1",
            &url
        )
        .is_ok());
        // absent Content-Type is permitted.
        assert!(validate_worker_script_response(None, 200, b"1", &url).is_ok());
        // wrong MIME rejected.
        assert!(matches!(
            validate_worker_script_response(Some("text/html"), 200, b"1", &url),
            Err(WorkerScriptError::InvalidMimeType(_))
        ));
        // non-2xx rejected.
        assert!(matches!(
            validate_worker_script_response(Some("text/javascript"), 404, b"1", &url),
            Err(WorkerScriptError::BadStatus { status: 404, .. })
        ));
    }
}
