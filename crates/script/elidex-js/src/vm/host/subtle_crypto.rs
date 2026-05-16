// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `SubtleCrypto` interface (WebCrypto §14) — VM thin binding to
//! the lazy-allocated `crypto.subtle` singleton.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! algorithm-name normalisation, BufferSource argument coercion
//! (reused via [`super::text_encoding::extract_buffer_source_bytes`])
//! and RustCrypto `Digest` driver calls.  Current scope ships only
//! `digest(algorithm, data)`; full SubtleCrypto (sign / verify /
//! encrypt / decrypt / deriveKey / generateKey / importKey /
//! exportKey / wrapKey / unwrapKey + `CryptoKey` lifecycle) is
//! deferred to slot `#11-crypto-subtle-full` — trigger: M4-13
//! entry kickoff OR auth-heavy framework adoption signal (WebAuthn
//! / request-signing library appearing in the test suite).
//!
//! ## Singleton storage
//!
//! - [`VmInner::subtle_crypto_instance`][]: cached `[SameObject]`
//!   `SubtleCrypto` wrapper returned by the `Crypto.prototype.subtle`
//!   accessor.  Lazily allocated via
//!   [`VmInner::alloc_or_cached_subtle_crypto`] on the first
//!   `crypto.subtle` read.  Cleared on `Vm::unbind` for GC-root
//!   hygiene (the wrapper is stateless, so re-allocation is cheap).
//! - [`VmInner::subtle_crypto_prototype`][]: `SubtleCrypto.prototype`
//!   chained to `Object.prototype`; rooted via the proto-roots
//!   array in `vm/gc/collect.rs`.
//!
//! ## GC interaction
//!
//! [`ObjectKind::SubtleCrypto`] is payload-free.  Singleton rooted
//! via `VmInner::subtle_crypto_instance` (mark-roots step in
//! `vm/gc/collect.rs`); trace fan-out is a no-op (see
//! `vm/gc/trace.rs`).

#![cfg(feature = "engine")]

use sha1::Digest as _;

use super::super::natives_promise::create_promise;
use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::array_buffer::create_array_buffer_from_bytes;
use super::blob::resolve_promise_sync;
use super::text_encoding::extract_buffer_source_bytes;

impl VmInner {
    /// Allocate `SubtleCrypto.prototype` chained to `Object.prototype`,
    /// install the `digest` method native, and expose the
    /// `SubtleCrypto` constructor stub on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`.
    /// Ordering relative to `register_crypto_global` does NOT matter
    /// for the prototype install itself — `Crypto.prototype.subtle`'s
    /// getter reads `subtle_crypto_prototype` lazily on the first
    /// JS invocation of `crypto.subtle`, which always happens after
    /// `register_globals()` has finished both registrations.  The
    /// current ordering (subtle before crypto, see
    /// `vm/globals.rs::register_globals`) is alphabetical / topological-
    /// hint convenience, not a correctness requirement.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — call-order
    /// invariant from `register_globals()` violated.
    pub(in crate::vm) fn register_subtle_crypto_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_subtle_crypto_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // `digest` native (WebCrypto §14.3.5).
        let methods: [(_, NativeFn); 1] = [(
            self.well_known.digest,
            native_subtle_crypto_digest as NativeFn,
        )];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        self.subtle_crypto_prototype = Some(proto_id);

        // `SubtleCrypto` constructor stub — throws per WebIDL §14.
        let ctor =
            self.create_constructable_function("SubtleCrypto", native_subtle_crypto_illegal_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            shape::PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            shape::PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.subtle_crypto_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Return the per-VM `SubtleCrypto` `[SameObject]` wrapper,
    /// allocating it on the first `Crypto.prototype.subtle` accessor
    /// read.  Mirrors the [`super::dom_selection_proto`]
    /// `alloc_or_cached_selection` shape: cached singleton, payload-
    /// free, prototype-via-`subtle_crypto_prototype`.
    ///
    /// Re-allocates after `Vm::unbind` (which clears
    /// `subtle_crypto_instance` for GC-root hygiene); the prototype
    /// stays live so retained JS references continue to brand-check
    /// and dispatch through the same `digest` native.
    pub(in crate::vm) fn alloc_or_cached_subtle_crypto(&mut self) -> ObjectId {
        if let Some(id) = self.subtle_crypto_instance {
            return id;
        }
        let proto = self
            .subtle_crypto_prototype
            .expect("alloc_or_cached_subtle_crypto before register_subtle_crypto_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::SubtleCrypto,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.subtle_crypto_instance = Some(id);
        id
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Confirm `this` is a `SubtleCrypto` instance, returning a
/// TypeError with the spec-conformant "Illegal invocation" wording
/// otherwise.  Used by every `SubtleCrypto.prototype.*` method
/// native.
fn require_subtle_crypto_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::SubtleCrypto) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': Illegal invocation"
        )));
    }
    Ok(id)
}

// ---------------------------------------------------------------------------
// Constructor stub — `new SubtleCrypto()` throws per WebIDL §14
// ---------------------------------------------------------------------------

fn native_subtle_crypto_illegal_ctor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Failed to construct 'SubtleCrypto': Illegal constructor",
    ))
}

// ---------------------------------------------------------------------------
// `SubtleCrypto.prototype.digest(algorithm, data)` (WebCrypto §14.3.5)
// ---------------------------------------------------------------------------

/// Canonical digest algorithm picked from the user-supplied
/// `AlgorithmIdentifier` per §18.2.1 step 3 (case-insensitive
/// match against the registered names).
#[derive(Clone, Copy)]
enum DigestAlgo {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl DigestAlgo {
    fn compute(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha1 => sha1::Sha1::digest(data).to_vec(),
            Self::Sha256 => sha2::Sha256::digest(data).to_vec(),
            Self::Sha384 => sha2::Sha384::digest(data).to_vec(),
            Self::Sha512 => sha2::Sha512::digest(data).to_vec(),
        }
    }
}

/// `normalize an algorithm` per WebCrypto §18.2.1 for the `digest`
/// operation: accept DOMString (sole algorithm name) OR an object
/// dictionary whose `name` member is the algorithm name.  Extra
/// dictionary keys are IGNORED (spec §18.2.1 step 4-5 — only the
/// `name` member is consulted for digest).  Returns the canonical
/// algorithm; the user-supplied raw name (case-as-typed) is echoed
/// in the `NotSupportedError` message per §18.2.1 step 9, truncated
/// at [`MAX_ECHOED_ALGO_NAME_LEN`] to bound the per-call DOMException
/// message allocation.
fn normalize_digest_algorithm(
    ctx: &mut NativeContext<'_>,
    algorithm_arg: JsValue,
) -> Result<DigestAlgo, VmError> {
    let name_sid = match algorithm_arg {
        JsValue::String(sid) => sid,
        JsValue::Object(id) => {
            // Dictionary form: read `name` member.  WebCrypto
            // §18.2.1 step 4 references `[[Algorithm]]` which is
            // `dictionary Algorithm { required DOMString name; }`
            // (WebCrypto §10.1) — `required` means a missing /
            // `undefined` member must throw TypeError during
            // dictionary conversion, NOT ToString-coerce to
            // `"undefined"` then reject with NotSupportedError.
            let name_key_sid = ctx.vm.well_known.name;
            let name_val = ctx.get_property_value(id, PropertyKey::String(name_key_sid))?;
            if matches!(name_val, JsValue::Undefined) {
                return Err(VmError::type_error(
                    "Failed to execute 'digest' on 'SubtleCrypto': \
                     Algorithm: name: Missing or not a string",
                ));
            }
            super::super::coerce::to_string(ctx.vm, name_val)?
        }
        // Primitives other than string → coerce-via-ToString (matches
        // Chrome where `crypto.subtle.digest(42, …)` ToString-coerces
        // "42" then rejects with NotSupportedError).
        other => super::super::coerce::to_string(ctx.vm, other)?,
    };
    // ASCII case-insensitive match against canonical names per
    // §18.2.1 step 3.  Compare against the WTF-16 backing storage
    // directly so the recognised-algorithm hot path does NOT
    // allocate (the prior `get_utf8` path materialised a fresh
    // `String` per call).  WTF-16 comparison also avoids a real
    // semantic hazard: `get_utf8` is lossy for lone surrogates,
    // and the recognised-vs-rejected decision should not depend
    // on a lossy decode — `matches_ascii_ci_wtf16` rejects any
    // code unit ≥ 128, so a name containing a lone surrogate
    // unambiguously falls through to the rejected-name path.
    let raw_wtf16 = ctx.vm.strings.get(name_sid);
    if matches_ascii_ci_wtf16(raw_wtf16, b"SHA-1") {
        Ok(DigestAlgo::Sha1)
    } else if matches_ascii_ci_wtf16(raw_wtf16, b"SHA-256") {
        Ok(DigestAlgo::Sha256)
    } else if matches_ascii_ci_wtf16(raw_wtf16, b"SHA-384") {
        Ok(DigestAlgo::Sha384)
    } else if matches_ascii_ci_wtf16(raw_wtf16, b"SHA-512") {
        Ok(DigestAlgo::Sha512)
    } else {
        // Rejected-name echo path — pay the UTF-8 conversion only
        // here.  Truncate at a UTF-8 boundary to bound the message
        // allocation — attacker-supplied `'A'.repeat(10_000_000)`
        // would otherwise allocate a 10 MB error string per call.
        //
        // `get_utf8` is intentionally used here even though it
        // replaces lone surrogates with U+FFFD: valid WebCrypto
        // algorithm names are pure ASCII per §18.2.1 table, so any
        // input containing a lone surrogate is by definition
        // unrecognised and the `'\u{FFFD}'` rendering is no less
        // informative than the original ill-formed sequence would
        // have been in a console.
        let raw = ctx.vm.strings.get_utf8(name_sid);
        let echo = truncate_at_char_boundary(&raw, MAX_ECHOED_ALGO_NAME_LEN);
        Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_not_supported_error,
            format!("Unrecognized algorithm name: '{echo}'"),
        ))
    }
}

/// ASCII case-insensitive equality between a WTF-16 haystack and an
/// ASCII byte needle.  Returns `false` immediately for any code unit
/// outside ASCII (≥ 128) so a lone surrogate cannot accidentally
/// equal `'a'..'z'` after a lossy decode.  Used by
/// [`normalize_digest_algorithm`] to compare against canonical
/// digest names (`"SHA-1"` etc.) without allocating.
fn matches_ascii_ci_wtf16(haystack: &[u16], needle: &[u8]) -> bool {
    haystack.len() == needle.len()
        && haystack
            .iter()
            .zip(needle)
            .all(|(&h, &n)| h < 0x80 && (h as u8).eq_ignore_ascii_case(&n))
}

/// Maximum byte length echoed back from an attacker-supplied
/// algorithm name into the `NotSupportedError` message.  Bounds
/// the per-call allocation so `crypto.subtle.digest('A'.repeat(N),
/// data)` cannot trigger an O(N) error-message alloc.
const MAX_ECHOED_ALGO_NAME_LEN: usize = 64;

fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Walk back to the nearest preceding UTF-8 boundary so we never
    // cut mid-codepoint (which would panic on the slice).
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn native_subtle_crypto_digest(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_subtle_crypto_this(ctx, this, "digest")?;

    // Pre-allocate the Promise so reject paths share the same
    // exit shape as resolve paths (every public spec algorithm
    // returns a Promise; failure modes settle the Promise, they
    // do NOT throw synchronously).  See WebCrypto §10.3 step 1.
    let promise = create_promise(ctx.vm);
    // Root the promise across allocations below (algorithm
    // normalization can trigger ToString → allocator).
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted = NativeContext { vm: &mut g };
    let ctx = &mut rooted;

    let algorithm_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    // §18.2.1 normalize algorithm.  Failures settle the returned
    // Promise with the rejection rather than throwing synchronously.
    let algo = match normalize_digest_algorithm(ctx, algorithm_arg) {
        Ok(algo) => algo,
        Err(e) => {
            let reason = ctx.vm.vm_error_to_thrown(&e);
            super::blob::reject_promise_sync(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    // §13.2 BufferSource snapshot: spec mandates a copy at call
    // time so post-call mutation of the input view does not affect
    // the digest result.  `extract_buffer_source_bytes` returns an
    // owned `Vec<u8>` so the snapshot is implicit.
    //
    // `allow_undefined_as_empty: false` per WebCrypto §14.3.5
    // IDL signature (`BufferSource data` — required, no `?`); a
    // missing 2nd arg defaults to `JsValue::Undefined` and must
    // settle the Promise with a TypeError, not silently hash empty
    // input.
    let bytes = match extract_buffer_source_bytes(
        ctx,
        data_arg,
        "Failed to execute 'digest' on 'SubtleCrypto'",
        2,
        false,
    ) {
        Ok(b) => b,
        Err(e) => {
            let reason = ctx.vm.vm_error_to_thrown(&e);
            super::blob::reject_promise_sync(ctx.vm, promise, reason);
            return Ok(JsValue::Object(promise));
        }
    };

    let digest_bytes = algo.compute(&bytes);
    let buf_id = create_array_buffer_from_bytes(ctx.vm, digest_bytes);
    resolve_promise_sync(ctx.vm, promise, JsValue::Object(buf_id));
    Ok(JsValue::Object(promise))
}
