//! Header name / value validation + normalisation primitives
//! (WHATWG Fetch §5.2 + RFC 7230).
//!
//! Pure functions over `VmInner` string pool handles — no Headers
//! state is touched, so this module has no dependency on the
//! surrounding `Headers` object machinery.  Split out of
//! `headers.rs` to keep that file under the project's 1000-line
//! convention (Copilot R23.1).

#![cfg(feature = "engine")]

use super::super::value::{StringId, VmError};
use super::super::VmInner;

/// RFC 7230 ABNF `tchar` — the permitted bytes in a header field
/// name.  Used by [`is_valid_header_name`]; any other byte
/// (including non-ASCII, CR, LF, NUL, whitespace, or the delimiter
/// characters) disqualifies the name.
#[inline]
fn is_tchar(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

fn is_valid_header_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(is_tchar)
}

/// Spec §5.2 "header value": no `0x0D` (CR), `0x0A` (LF), or
/// `0x00` (NUL).  Leading / trailing HTAB (`0x09`) and SP (`0x20`)
/// must be trimmed *before* validation per §5.2 "normalize a byte
/// sequence"; [`validate_and_normalise`] handles the trim.
fn is_valid_header_value_content(s: &str) -> bool {
    !s.bytes().any(|b| matches!(b, 0x00 | 0x0A | 0x0D))
}

/// Trim leading/trailing HTAB (`0x09`) + SP (`0x20`) per WHATWG
/// Fetch §5.2 "normalize a byte sequence".  Works on bytes directly
/// because the spec only trims ASCII whitespace, and the input
/// must already be ASCII for validation to have any chance of
/// passing.
fn trim_http_whitespace(s: &str) -> &str {
    s.trim_matches(|c: char| c == '\t' || c == ' ')
}

/// Combined validation + interning pass: lowercase the name, trim
/// the value, validate both, return interned `(name, value)`.
///
/// Returns `TypeError` per WHATWG Fetch §5.2 validation steps —
/// `DOMException` would be the wrong choice here (spec says
/// `TypeError`).
/// `pub(super)` so the `fetch` module can route broker-delivered
/// response headers through the same name/value invariants as
/// script-constructed Headers (§5.2 normalisation — lowercased
/// name, trimmed value, no CR/LF/NUL).
pub(super) fn validate_and_normalise(
    vm: &mut VmInner,
    name_sid: StringId,
    value_sid: StringId,
    error_prefix: &str,
) -> Result<(StringId, StringId), VmError> {
    let name_sid = validate_and_normalise_name(vm, name_sid, error_prefix)?;
    let value_raw = vm.strings.get_utf8(value_sid);
    let trimmed = trim_http_whitespace(&value_raw);
    if !is_valid_header_value_content(trimmed) {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Invalid header value — contains CR, LF, or NUL"
        )));
    }
    // Re-intern the trimmed form only if trimming changed bytes —
    // otherwise keep the original StringId so repeated adds share
    // pool entries.
    let value_sid = if trimmed.len() == value_raw.len() {
        value_sid
    } else {
        vm.strings.intern(trimmed)
    };
    Ok((name_sid, value_sid))
}

/// WHATWG Fetch §4.6 forbidden request header names.  Compared
/// against the lowercased name returned by
/// [`validate_and_normalise_name`].  Includes the `proxy-` and
/// `sec-` byte-prefix matches.
///
/// Used by [`super::headers`]'s `Request`-guard mutation gate and
/// by [`super::fetch`]'s URL-input init.headers snapshot path.
/// Spec semantics: matched names are *silently ignored*, not
/// rejected with TypeError — matches browsers (Chrome / Firefox /
/// Safari) and is what user code expects from `Headers.append`.
#[must_use]
pub(super) fn is_forbidden_request_header(lower_name: &str) -> bool {
    if lower_name.starts_with("proxy-") || lower_name.starts_with("sec-") {
        return true;
    }
    matches!(
        lower_name,
        "accept-charset"
            | "accept-encoding"
            | "access-control-request-headers"
            | "access-control-request-method"
            | "connection"
            | "content-length"
            | "cookie"
            | "cookie2"
            | "date"
            | "dnt"
            | "expect"
            | "host"
            | "keep-alive"
            | "origin"
            | "referer"
            | "set-cookie"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "via"
    )
}

/// `pub(super)` so `Headers.prototype.{get,has,delete}` can reuse
/// the name-only validation path for their single-name argument
/// (§5.2 validation covers both name and value, but those methods
/// only operate on the name).
pub(super) fn validate_and_normalise_name(
    vm: &mut VmInner,
    name_sid: StringId,
    error_prefix: &str,
) -> Result<StringId, VmError> {
    let raw = vm.strings.get_utf8(name_sid);
    if !is_valid_header_name(&raw) {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Invalid header name '{raw}' — must match RFC 7230 token syntax"
        )));
    }
    let lower = raw.to_ascii_lowercase();
    // Avoid the re-intern if already lowercase (common case for
    // wire-format names like `content-type`).
    let name_sid = if lower == raw {
        name_sid
    } else {
        vm.strings.intern(&lower)
    };
    Ok(name_sid)
}
