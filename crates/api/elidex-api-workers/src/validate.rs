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
/// type set defined in MIME Sniffing §4.6 — the full essence list, incl. the
/// `ecmascript` / `x-` family and the legacy `text/javascript1.x` essences).
pub const JS_MIME_TYPES: &[&str] = &[
    "application/ecmascript",
    "application/javascript",
    "application/x-ecmascript",
    "application/x-javascript",
    "text/ecmascript",
    "text/javascript",
    "text/javascript1.0",
    "text/javascript1.1",
    "text/javascript1.2",
    "text/javascript1.3",
    "text/javascript1.4",
    "text/javascript1.5",
    "text/jscript",
    "text/livescript",
    "text/x-ecmascript",
    "text/x-javascript",
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
    /// The resolved URL uses a scheme the worker runtime cannot load yet
    /// (currently `blob:` — needs a blob-URL source loader).
    UnsupportedScheme(String),
    /// `{ type: "module" }` — module workers are not yet supported.
    UnsupportedType,
    /// `{ type: ... }` with a value outside the `WorkerType` enum
    /// (`"classic"` / `"module"`) — a WebIDL enum-coercion `TypeError`.
    InvalidType(String),
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
                "Worker script URL {resolved} is not same-origin with {base}"
            ),
            Self::UnsupportedScheme(scheme) => write!(
                f,
                "Worker: '{scheme}:' worker scripts are not supported yet"
            ),
            Self::UnsupportedType => write!(f, "Worker: type 'module' is not supported"),
            Self::InvalidType(t) => write!(
                f,
                "Worker: '{t}' is not a valid WorkerType ('classic' or 'module')"
            ),
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
/// `data:` URLs are always permitted (decoded inline); any other scheme must be
/// same-origin with `base`. `blob:` is rejected until the worker runtime gains a
/// blob-URL source loader (blocked on `URL.createObjectURL`; defer slot
/// `#11-worker-blob-script`) — accepting it here would only fail later at the
/// source-fetch step.
///
/// # Errors
/// [`WorkerScriptError::EmptyUrl`] when `url_str` is empty,
/// [`WorkerScriptError::InvalidUrl`] when it fails to resolve,
/// [`WorkerScriptError::UnsupportedScheme`] for `blob:`, and
/// [`WorkerScriptError::NotSameOrigin`] when the resolved URL is cross-origin.
pub fn resolve_worker_script_url(base: &Url, url_str: &str) -> Result<Url, WorkerScriptError> {
    if url_str.is_empty() {
        return Err(WorkerScriptError::EmptyUrl);
    }

    let resolved = base
        .join(url_str)
        .map_err(|e| WorkerScriptError::InvalidUrl(e.to_string()))?;

    // `blob:` worker scripts need a blob-URL source loader that does not yet
    // exist (no `URL.createObjectURL` in the runtime) — reject rather than
    // accept-then-fail.
    if resolved.scheme() == "blob" {
        return Err(WorkerScriptError::UnsupportedScheme("blob".to_string()));
    }

    // `data:` is same-origin by fiat (decoded inline); everything else compares
    // the resolved origin against the base origin.
    let is_same_origin = resolved.scheme() == "data" || base.origin() == resolved.origin();

    if !is_same_origin {
        return Err(WorkerScriptError::NotSameOrigin {
            resolved: resolved.to_string(),
            base: base.to_string(),
        });
    }

    Ok(resolved)
}

/// Validate the `type` worker option against the WebIDL `WorkerType` enum
/// (`"classic"` / `"module"`, WHATWG HTML §10.2.6.3). Absent / `"classic"` is
/// accepted; `"module"` is a recognized-but-unsupported value; any other string
/// is an invalid enum value (WebIDL §3.10 enum coercion → `TypeError`).
///
/// # Errors
/// [`WorkerScriptError::UnsupportedType`] for `"module"`;
/// [`WorkerScriptError::InvalidType`] for a value outside the enum.
pub fn validate_worker_type(type_opt: Option<&str>) -> Result<(), WorkerScriptError> {
    match type_opt {
        None | Some("classic") => Ok(()),
        Some("module") => Err(WorkerScriptError::UnsupportedType),
        Some(other) => Err(WorkerScriptError::InvalidType(other.to_string())),
    }
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
    fn blob_rejected_until_loader_exists() {
        // `blob:` needs a blob-URL source loader the runtime lacks (no
        // `URL.createObjectURL`) — reject rather than accept-then-fail
        // (defer slot `#11-worker-blob-script`).
        let err = resolve_worker_script_url(&base(), "blob:https://example.com/uuid-123")
            .expect_err("blob: unsupported");
        assert!(matches!(err, WorkerScriptError::UnsupportedScheme(s) if s == "blob"));
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
    fn invalid_worker_type_rejected() {
        // WebIDL `WorkerType` enum: anything outside classic/module is a
        // coercion error, not silently treated as classic.
        assert_eq!(
            validate_worker_type(Some("bogus")),
            Err(WorkerScriptError::InvalidType("bogus".to_string()))
        );
        assert_eq!(
            validate_worker_type(Some("")),
            Err(WorkerScriptError::InvalidType(String::new()))
        );
    }

    #[test]
    fn ecmascript_mime_essences_accepted() {
        let url = base();
        for mime in [
            "application/ecmascript",
            "text/ecmascript",
            "application/x-ecmascript",
            "text/x-javascript",
            "text/javascript1.5",
        ] {
            assert!(
                validate_worker_script_response(Some(mime), 200, b"1", &url).is_ok(),
                "{mime} is a JavaScript MIME essence and must be accepted"
            );
        }
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
