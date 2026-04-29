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
//! (cheap `Arc::clone`).  Header names + filenames are
//! percent-escaped once during [`materialise`] and reused unchanged
//! by the encode loop, so escaping never touches the hot path.  The
//! final body `Vec<u8>` is pre-allocated to the exact total length
//! so the encode loop never reallocates.
//!
//! ## Boundary generation
//!
//! Boundaries must not appear inside the encoded body.  The
//! candidate boundary is derived from a salt counter mixed with a
//! per-process [`process_nonce`] (seeded once at first call from
//! `RandomState`'s cryptographically strong system random data).
//! Collision check runs against the small parts —
//! quoted-name / quoted-filename / Content-Type / string-entry
//! values — and bumps the salt on the rare collision.
//!
//! **Blob payload bytes are NOT scanned for collisions** — the O(N)
//! sweep over multi-MiB Blobs would dominate encode cost.  The
//! per-process nonce makes it computationally infeasible for
//! adversarial Blob content to match the boundary (the attacker
//! cannot predict the runtime nonce), and the boundary is always
//! emitted with the leading `--` marker which would have to appear
//! verbatim inside Blob bytes for the encoded body to be malformed.
//! This matches Chromium / Firefox's "random boundary, no body
//! scan" approach.
//!
//! Stdlib-only — no `getrandom` / `rand` workspace dependency
//! introduced for this PR.

#![cfg(feature = "engine")]

use std::collections::hash_map::{DefaultHasher, RandomState};
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::{Arc, OnceLock};

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
    //    `body.extend_from_slice` later.  Header names / filenames
    //    are escape-quoted up-front so the encode loop emits them
    //    verbatim (no per-iteration alloc).
    let materialised: Vec<MaterialisedEntry> = entries.iter().map(|e| materialise(vm, e)).collect();

    // 2. Boundary derivation.  Hash a per-process nonce + the salt
    //    counter to produce a 64-bit fingerprint that an adversarial
    //    input cannot predict (RandomState seeds the nonce from
    //    system random data).  Collision-check the candidate against
    //    the small parts (quoted-name / quoted-filename /
    //    Content-Type / string-entry value bytes) and bump the salt
    //    on the rare collision.  Blob bytes are NOT scanned — see
    //    module docs for the entropy / perf trade-off.
    let mut salt: u64 = 0;
    let boundary = loop {
        let candidate = derive_boundary(salt);
        let needle = format!("--{candidate}");
        let needle_bytes = needle.as_bytes();
        let collides = materialised.iter().any(|e| {
            byte_slice_contains(&e.quoted_name, needle_bytes)
                || e.quoted_filename
                    .as_deref()
                    .is_some_and(|f| byte_slice_contains(f, needle_bytes))
                || e.content_type
                    .as_deref()
                    .is_some_and(|t| byte_slice_contains(t, needle_bytes))
                || (!e.is_blob && byte_slice_contains(&e.bytes, needle_bytes))
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
        body.extend_from_slice(&entry.quoted_name);
        body.extend_from_slice(b"\"");
        if let Some(filename) = &entry.quoted_filename {
            body.extend_from_slice(b"; filename=\"");
            body.extend_from_slice(filename);
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
/// entry on the small-string path.  `quoted_name` and
/// `quoted_filename` carry the already-escaped header bytes so the
/// encode loop never re-escapes.
struct MaterialisedEntry {
    /// `name` after RFC 7578 escape (`\r` → `%0D` etc.).  Stored
    /// pre-quoted so the encode loop emits it verbatim and the
    /// boundary-collision scan compares against the form actually
    /// written into the body.
    quoted_name: Vec<u8>,
    /// `filename` after RFC 7578 escape (when the entry has one).
    quoted_filename: Option<Vec<u8>>,
    /// `Content-Type` line bytes, **not including** the `Content-Type: `
    /// prefix or the trailing `\r\n` — those are written by [`encode`].
    /// `None` for string entries (no header line emitted).
    content_type: Option<Vec<u8>>,
    bytes: Arc<[u8]>,
    /// `true` for `FormDataValue::Blob` entries — large-payload
    /// hint that the boundary-collision scan skips.  String
    /// values stay short enough to scan cheaply.
    is_blob: bool,
}

fn materialise(vm: &VmInner, entry: &FormDataEntry) -> MaterialisedEntry {
    let quoted_name = header_quote(vm.strings.get_utf8(entry.name).as_bytes());
    match &entry.value {
        FormDataValue::String(sid) => MaterialisedEntry {
            quoted_name,
            quoted_filename: None,
            content_type: None,
            bytes: Arc::from(vm.strings.get_utf8(*sid).into_bytes()),
            is_blob: false,
        },
        FormDataValue::Blob(blob_id) => {
            let raw_filename = entry
                .filename
                .map(|sid| vm.strings.get_utf8(sid))
                .unwrap_or_else(|| vm.strings.get_utf8(vm.well_known.blob_default_filename));
            let quoted_filename = header_quote(raw_filename.as_bytes());
            let blob_type_sid = super::blob::blob_type(vm, *blob_id);
            let content_type = if blob_type_sid == vm.well_known.empty {
                Some(b"application/octet-stream".to_vec())
            } else {
                Some(vm.strings.get_utf8(blob_type_sid).into_bytes())
            };
            MaterialisedEntry {
                quoted_name,
                quoted_filename: Some(quoted_filename),
                content_type,
                // Reference-counted handoff from BlobData — no
                // payload clone, even for multi-MB Blobs.
                bytes: blob_bytes_arc(vm, *blob_id),
                is_blob: true,
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
        sum += CD_PREFIX + entry.quoted_name.len() + 1; // ...name="..."`
        if let Some(filename) = &entry.quoted_filename {
            sum += FILENAME_PREFIX + filename.len() + 1;
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

/// Per-process nonce mixed into [`derive_boundary`] so the
/// generated boundary cannot be predicted from input data alone.
/// `RandomState` seeds the hasher from system random data at
/// first call (cryptographically strong on every supported
/// platform); the `OnceLock` makes subsequent calls O(1).
fn process_nonce() -> u64 {
    static NONCE: OnceLock<u64> = OnceLock::new();
    *NONCE.get_or_init(|| {
        // `RandomState::build_hasher()` returns a `DefaultHasher`
        // keyed with per-process random state.  With no input
        // written, `finish()` yields the hash output for an empty
        // input under those random keys — the keys themselves are
        // not exposed, so this 64-bit value is sufficient as a
        // per-process nonce (an attacker cannot recover the keys
        // from the output, and we re-hash it through a fresh
        // `DefaultHasher` in `derive_boundary` anyway).
        RandomState::new().build_hasher().finish()
    })
}

/// Derive a hex boundary string from the `salt` mixed with the
/// per-process [`process_nonce`] — no payload bytes touched.
/// Always 24 hex chars (16 from the hashed mix + 8 from `salt as
/// u32`) appended to the `----elidexFormBoundary` prefix, well
/// within RFC 2046's 70-char limit.  Unpredictable across
/// processes thanks to the random nonce; deterministic within a
/// process given the same salt so the collision-retry loop
/// converges on the same boundary even after a salt bump.
fn derive_boundary(salt: u64) -> String {
    let mut hasher = DefaultHasher::new();
    process_nonce().hash(&mut hasher);
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
