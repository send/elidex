//! `multipart/form-data` body encoder (RFC 7578 / WHATWG XHR §4.3.6).
//!
//! Entry point [`encode`] takes a `FormData` entry list + a
//! `&VmInner` (so it can read `string_pool` and `blob_data`) and
//! returns `(body_bytes, boundary)` ready to be threaded into a
//! `Content-Type: multipart/form-data; boundary=…` header.
//!
//! ## Allocation discipline
//!
//! Blob payloads are not cloned: each entry's `bytes` field is an
//! `Arc<[u8]>` carried directly from [`super::blob::BlobData::bytes`]
//! (cheap `Arc::clone`).  String entries materialise the value once
//! into an owned `Arc<[u8]>`.  The collision-check pass operates on
//! `&[u8]` views without further copies, and the final body
//! `Vec<u8>` is pre-allocated to the exact total length so the
//! encode loop never reallocates.
//!
//! ## Boundary generation
//!
//! Boundaries must not appear inside the encoded body.  We derive a
//! salt-only fingerprint (no payload bytes) and test for collision
//! against every entry's `name` / `filename` / `content_type` /
//! `bytes`; on collision the salt counter increments and we retry.
//! A 64-bit fingerprint collides with arbitrary user data with
//! probability ≪ 2⁻⁶⁴, so the loop exits on the first salt for
//! every realistic input — but the explicit collision check keeps
//! the encoder correct against pathological inputs that intentionally
//! contain the would-be boundary string.
//!
//! Stdlib-only — no `getrandom` / `rand` workspace dependency
//! introduced for this PR.

#![cfg(feature = "engine")]

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::super::value::ObjectId;
use super::super::VmInner;
use super::form_data::{FormDataEntry, FormDataValue};

/// Encode a `FormData` entry list as a `multipart/form-data` body.
///
/// Returns `(bytes, boundary)`.  `boundary` is the random ASCII
/// suffix that goes into the `Content-Type` header (without the
/// `multipart/form-data; boundary=` prefix).
pub(super) fn encode(vm: &VmInner, entries: &[FormDataEntry]) -> (Vec<u8>, String) {
    // 1. Materialise each entry once.  String values intern into
    //    owned `Arc<[u8]>` (small, one allocation each); Blob
    //    values share the `Arc<[u8]>` from the BlobData side
    //    table without copying — the bytes flow through to
    //    `body.extend_from_slice` later.
    let materialised: Vec<MaterialisedEntry> = entries.iter().map(|e| materialise(vm, e)).collect();

    // 2. Salt-only boundary derivation, looping with a salt
    //    increment if the candidate appears in any entry's bytes.
    //    No payload hashing — practically bounded because a
    //    64-bit space collides with adversarial user data only at
    //    ≪ 2⁻⁶⁴ rate, and the salt counter increments on
    //    collision.
    let mut salt: u64 = 0;
    let boundary = loop {
        let candidate = derive_boundary(salt);
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

    // 3. Pre-compute the exact body length so the encode loop never
    //    triggers a reallocation.  `total_len` covers every
    //    boundary marker, header line, separator and value byte;
    //    matches the writes in step 4 byte-for-byte.
    let bdash = format!("--{boundary}");
    let total_len = compute_body_len(&materialised, bdash.len());

    // 4. Serialise.  Per RFC 7578 each part is preceded by `--<boundary>\r\n`,
    //    headers terminate with `\r\n\r\n`, the value is followed by `\r\n`,
    //    and the body ends with `--<boundary>--\r\n`.
    let mut body: Vec<u8> = Vec::with_capacity(total_len);
    let bdash_bytes = bdash.as_bytes();
    for entry in &materialised {
        body.extend_from_slice(bdash_bytes);
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
    body.extend_from_slice(bdash_bytes);
    body.extend_from_slice(b"--\r\n");
    debug_assert_eq!(
        body.len(),
        total_len,
        "multipart pre-reserved capacity must match actual write length"
    );

    (body, boundary)
}

/// Per-entry snapshot the encoder operates on.  `bytes` is an
/// `Arc<[u8]>` so Blob payloads flow through reference-counted
/// rather than cloned; string entries spend one allocation per
/// entry on the small-string path.
struct MaterialisedEntry {
    name: Vec<u8>,
    filename: Option<Vec<u8>>,
    /// `Content-Type` line bytes, **not including** the `Content-Type: `
    /// prefix or the trailing `\r\n` — those are written by [`encode`].
    /// `None` for string entries (no header line emitted).
    content_type: Option<Vec<u8>>,
    bytes: Arc<[u8]>,
}

fn materialise(vm: &VmInner, entry: &FormDataEntry) -> MaterialisedEntry {
    let name = vm.strings.get_utf8(entry.name).into_bytes();
    match &entry.value {
        FormDataValue::String(sid) => MaterialisedEntry {
            name,
            filename: None,
            content_type: None,
            bytes: Arc::from(vm.strings.get_utf8(*sid).into_bytes()),
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
            MaterialisedEntry {
                name,
                filename: Some(filename),
                content_type,
                // Reference-counted handoff from BlobData — no
                // payload clone, even for multi-MB Blobs.
                bytes: blob_bytes_arc(vm, *blob_id),
            }
        }
    }
}

fn blob_bytes_arc(vm: &VmInner, blob_id: ObjectId) -> Arc<[u8]> {
    super::blob::blob_bytes(vm, blob_id)
}

/// Sum the byte length the encode loop will write.  Mirrors the
/// `extend_from_slice` calls in [`encode`] step 4 1:1; the
/// `debug_assert_eq!` at the end of `encode` catches any drift.
fn compute_body_len(entries: &[MaterialisedEntry], bdash_len: usize) -> usize {
    // Per-entry: `--<boundary>\r\n`
    //          + `Content-Disposition: form-data; name="..."`
    //          + (optional `; filename="..."`)
    //          + `\r\n`
    //          + (optional `Content-Type: ...\r\n`)
    //          + `\r\n` + value + `\r\n`.
    // Plus the closing `--<boundary>--\r\n`.
    const CD_PREFIX: usize = "Content-Disposition: form-data; name=\"".len();
    const FILENAME_PREFIX: usize = "; filename=\"".len();
    const CT_PREFIX: usize = "Content-Type: ".len();
    let mut sum = 0usize;
    for entry in entries {
        sum += bdash_len + 2; // `--<boundary>\r\n`
        sum += CD_PREFIX + header_quoted_len(&entry.name) + 1; // ...name="..."`
        if let Some(filename) = &entry.filename {
            sum += FILENAME_PREFIX + header_quoted_len(filename) + 1;
        }
        sum += 2; // `\r\n` (end of Content-Disposition line)
        if let Some(ct) = &entry.content_type {
            sum += CT_PREFIX + ct.len() + 2;
        }
        sum += 2; // header/value separator
        sum += entry.bytes.len();
        sum += 2; // value terminator
    }
    sum += bdash_len + 4; // closing `--<boundary>--\r\n`
    sum
}

/// Length of `header_quote(input)` without allocating.  Mirrors
/// the substitution table in [`header_quote`]: each CR / LF / `"`
/// expands from 1 to 3 bytes.
fn header_quoted_len(input: &[u8]) -> usize {
    let mut len = input.len();
    for &b in input {
        if matches!(b, b'\r' | b'\n' | b'"') {
            len += 2;
        }
    }
    len
}

/// Derive a hex boundary string from the `salt` only — no payload
/// bytes touched.  Always 25 hex chars + the `elidexFormBoundary`
/// prefix, well within RFC 2046's 70-char limit and unique enough
/// to avoid collisions in practice (the explicit collision check
/// in [`encode`] retries with an incremented salt if it ever does).
fn derive_boundary(salt: u64) -> String {
    // Hash the salt through DefaultHasher to spread the bits — a
    // simple `format!("{:016x}", salt)` would emit predictable
    // mostly-zero prefixes for small salt values, raising the
    // (already ≪ 2⁻⁶⁴) collision probability against pathological
    // inputs.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    salt.hash(&mut hasher);
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
    let mut out = Vec::with_capacity(header_quoted_len(input));
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
