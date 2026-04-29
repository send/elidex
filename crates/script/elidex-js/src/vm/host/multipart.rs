//! `multipart/form-data` body encoder (RFC 7578 / WHATWG XHR §4.3.6).
//!
//! Entry point [`encode`] takes a `FormData` entry list + a
//! `&VmInner` (so it can read `string_pool` and `blob_data`) and
//! returns `(body_bytes, boundary)` ready to be threaded into a
//! `Content-Type: multipart/form-data; boundary=…` header.
//!
//! ## Boundary generation
//!
//! Boundaries must not appear inside the encoded body.  We derive a
//! deterministic boundary from a `std::hash::DefaultHasher` over the
//! concatenated entry bytes, then test for collision against the
//! body and bump a salt counter if necessary.  In practice the
//! 16-hex-digit fingerprint collides with arbitrary user data with
//! probability ≪ 2⁻⁶⁴, so the loop terminates after one pass for
//! every realistic input — but the explicit collision check keeps
//! the encoder correct against pathological inputs that intentionally
//! contain the would-be boundary string.
//!
//! Stdlib-only — no `getrandom` / `rand` workspace dependency
//! introduced for this PR.

#![cfg(feature = "engine")]

use std::hash::{Hash, Hasher};

use super::super::value::ObjectId;
use super::super::VmInner;
use super::form_data::{FormDataEntry, FormDataValue};

/// Encode a `FormData` entry list as a `multipart/form-data` body.
///
/// Returns `(bytes, boundary)`.  `boundary` is the random ASCII
/// suffix that goes into the `Content-Type` header (without the
/// `multipart/form-data; boundary=` prefix).
pub(super) fn encode(vm: &VmInner, entries: &[FormDataEntry]) -> (Vec<u8>, String) {
    // 1. Materialise each entry's name/value/filename + bytes once
    //    so the boundary-collision loop reads the same payload it
    //    will eventually emit.  Snapshotting up front also keeps
    //    the borrow on `vm` immutable for the entire encode pass.
    let materialised: Vec<MaterialisedEntry> = entries.iter().map(|e| materialise(vm, e)).collect();

    // 2. Derive a candidate boundary, looping with a salt increment
    //    if the candidate appears in any entry's bytes.  Practically
    //    bounded: a 64-bit fingerprint collides with adversarial
    //    user data only when the user pre-computes the hash, which
    //    cannot succeed against a per-call salt.
    let mut salt: u64 = 0;
    let boundary = loop {
        let candidate = derive_boundary(&materialised, salt);
        let needle = format!("--{candidate}");
        let needle_bytes = needle.as_bytes();
        let collides = materialised.iter().any(|e| {
            byte_slice_contains(&e.name, needle_bytes)
                || e.filename
                    .as_deref()
                    .is_some_and(|f| byte_slice_contains(f, needle_bytes))
                || e.content_type
                    .as_deref()
                    .is_some_and(|t| byte_slice_contains(t, needle_bytes))
                || byte_slice_contains(&e.bytes, needle_bytes)
        });
        if !collides {
            break candidate;
        }
        salt = salt.wrapping_add(1);
    };

    // 3. Serialise.  Per RFC 7578 each part is preceded by `--<boundary>\r\n`,
    //    headers terminate with `\r\n\r\n`, the value is followed by `\r\n`,
    //    and the body ends with `--<boundary>--\r\n`.
    let mut body: Vec<u8> = Vec::new();
    let bdash = format!("--{boundary}");
    for entry in &materialised {
        body.extend_from_slice(bdash.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"");
        body.extend_from_slice(&header_quote(&entry.name));
        body.extend_from_slice(b"\"");
        if let Some(filename) = &entry.filename {
            body.extend_from_slice(b"; filename=\"");
            body.extend_from_slice(&header_quote(filename));
            body.extend_from_slice(b"\"");
        }
        body.extend_from_slice(b"\r\n");
        if let Some(ct) = &entry.content_type {
            body.extend_from_slice(b"Content-Type: ");
            body.extend_from_slice(ct);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(&entry.bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(bdash.as_bytes());
    body.extend_from_slice(b"--\r\n");

    (body, boundary)
}

/// Per-entry snapshot the encoder operates on.  All four byte
/// vectors are `Vec<u8>` so the caller's `&VmInner` borrow can be
/// released immediately after [`materialise`] returns.
struct MaterialisedEntry {
    name: Vec<u8>,
    filename: Option<Vec<u8>>,
    /// `Content-Type` line bytes, **not including** the `Content-Type: `
    /// prefix or the trailing `\r\n` — those are written by [`encode`].
    /// `None` for string entries (no header line emitted).
    content_type: Option<Vec<u8>>,
    bytes: Vec<u8>,
}

fn materialise(vm: &VmInner, entry: &FormDataEntry) -> MaterialisedEntry {
    let name = vm.strings.get_utf8(entry.name).into_bytes();
    match &entry.value {
        FormDataValue::String(sid) => MaterialisedEntry {
            name,
            filename: None,
            content_type: None,
            bytes: vm.strings.get_utf8(*sid).into_bytes(),
        },
        FormDataValue::Blob(blob_id) => {
            let filename = entry
                .filename
                .map(|sid| vm.strings.get_utf8(sid).into_bytes())
                .unwrap_or_else(|| {
                    vm.strings
                        .get_utf8(vm.well_known.blob_default_filename)
                        .into_bytes()
                });
            let blob_type_sid = super::blob::blob_type(vm, *blob_id);
            let content_type = if blob_type_sid == vm.well_known.empty {
                Some(b"application/octet-stream".to_vec())
            } else {
                Some(vm.strings.get_utf8(blob_type_sid).into_bytes())
            };
            let bytes = blob_bytes_owned(vm, *blob_id);
            MaterialisedEntry {
                name,
                filename: Some(filename),
                content_type,
                bytes,
            }
        }
    }
}

fn blob_bytes_owned(vm: &VmInner, blob_id: ObjectId) -> Vec<u8> {
    super::blob::blob_bytes(vm, blob_id).to_vec()
}

/// Derive a hex boundary string from a hasher seeded over every
/// materialised entry's bytes plus `salt`.  Always 24 hex chars +
/// the `elidexFormBoundary` prefix, well within RFC 2046's 70-char
/// limit and unique enough to avoid collisions in practice.
fn derive_boundary(entries: &[MaterialisedEntry], salt: u64) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    salt.hash(&mut hasher);
    for e in entries {
        e.name.hash(&mut hasher);
        e.filename.hash(&mut hasher);
        e.content_type.hash(&mut hasher);
        e.bytes.hash(&mut hasher);
    }
    let h = hasher.finish();
    format!("----elidexFormBoundary{:016x}{:08x}", h, salt as u32)
}

/// `slice::contains_slice` is unstable; this is the obvious O(N·M)
/// scan, fine for the tiny inputs the boundary collision check
/// faces.  Empty `needle` short-circuits to `true` per
/// substring-search convention.
fn byte_slice_contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Apply RFC 7578 header escaping to a name / filename: replace
/// CR / LF / `"` with their `%0D` / `%0A` / `%22` percent-encoded
/// forms, leaving every other byte untouched (matching Chromium /
/// Firefox; the spec calls for HTML form-data escape rules).
fn header_quote(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for &b in input {
        match b {
            b'\r' => out.extend_from_slice(b"%0D"),
            b'\n' => out.extend_from_slice(b"%0A"),
            b'"' => out.extend_from_slice(b"%22"),
            _ => out.push(b),
        }
    }
    out
}
