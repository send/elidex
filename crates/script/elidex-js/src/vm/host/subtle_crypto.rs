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
//! engine-bound responsibilities: prototype install, brand check, and
//! marshalling JS values ↔ the engine-independent `elidex-api-crypto`
//! API (algorithm normalization, key validation, HMAC, digest, and JWK
//! all live in the crate).  BufferSource coercion is reused via
//! [`super::text_encoding::extract_buffer_source_bytes`].
//!
//! Current scope (`#11-crypto-subtle-full` PR-1): `digest` +
//! `CryptoKey` lifecycle + the HMAC vertical (`generateKey` /
//! `importKey` / `exportKey` / `sign` / `verify`).  AES (PR-2),
//! KDF + wrap/unwrap (PR-3), ECDSA/ECDH (PR-4), and RSA (PR-5) extend
//! the crate registry by adding rows.
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

use elidex_api_crypto::key::KeyUsage;
use elidex_api_crypto::{
    self as crypto, AlgorithmError, ExportedKey, HashAlgorithm, JsonWebKey, KeyData, KeyFormat,
    NormalizedAlgorithm, Operation, RawAlgorithm,
};

use super::super::coerce;
use super::super::natives_promise::create_promise;
use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::array_buffer::create_array_buffer_from_bytes;
use super::blob::{reject_promise_sync, resolve_promise_sync};
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

        // `SubtleCrypto.prototype` operation natives (WebCrypto §14.3).
        let methods: [(_, NativeFn); 6] = [
            (
                self.well_known.digest,
                native_subtle_crypto_digest as NativeFn,
            ),
            (
                self.well_known.generate_key,
                native_subtle_crypto_generate_key,
            ),
            (self.well_known.import_key, native_subtle_crypto_import_key),
            (self.well_known.export_key, native_subtle_crypto_export_key),
            (self.well_known.sign, native_subtle_crypto_sign),
            (self.well_known.verify, native_subtle_crypto_verify),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        self.subtle_crypto_prototype = Some(proto_id);

        // `SubtleCrypto` declares no constructor operation — registered as
        // IllegalConstructor so both call/construct throw at the gate
        // (Web Cryptography API §14 SubtleCrypto interface).
        let ctor = self.create_illegal_constructor_function(
            "SubtleCrypto",
            super::super::value::native_illegal_constructor_unreachable,
        );
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
// `SubtleCrypto.prototype.digest(algorithm, data)` (WebCrypto §14.3.5)
// ---------------------------------------------------------------------------

/// Canonical digest algorithm picked from the user-supplied
/// `AlgorithmIdentifier` per §18.4.4 (case-insensitive
/// match against the registered names).
#[derive(Clone, Copy)]
enum DigestAlgo {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl DigestAlgo {
    /// Map to the engine-independent crate hash.  The actual digest
    /// computation lives in `elidex-api-crypto` (CLAUDE.md "Layering
    /// mandate" — no RustCrypto driver calls in the VM layer).
    fn to_hash(self) -> HashAlgorithm {
        match self {
            Self::Sha1 => HashAlgorithm::Sha1,
            Self::Sha256 => HashAlgorithm::Sha256,
            Self::Sha384 => HashAlgorithm::Sha384,
            Self::Sha512 => HashAlgorithm::Sha512,
        }
    }
}

/// `normalize an algorithm` per WebCrypto §18.4.4 for the `digest`
/// operation: accept DOMString (sole algorithm name) OR an object
/// dictionary whose `name` member is the algorithm name.  Extra
/// dictionary keys are IGNORED (spec §18.4.4 — only the
/// `name` member is consulted for digest).  Returns the canonical
/// algorithm; the user-supplied raw name (case-as-typed) is echoed
/// in the `NotSupportedError` message per §18.4.4, truncated
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
            // §18.4.4 references `[[Algorithm]]` which is
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
    // §18.4.4.  Compare against the WTF-16 backing storage
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
        // algorithm names are pure ASCII per §18.4.4 table, so any
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
    let mut rooted = NativeContext::new_call(&mut g);
    let ctx = &mut rooted;

    let algorithm_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    // §18.4.4 normalize algorithm.  Failures settle the returned
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

    let digest_bytes = algo.to_hash().digest(&bytes);
    let buf_id = create_array_buffer_from_bytes(ctx.vm, digest_bytes);
    resolve_promise_sync(ctx.vm, promise, JsValue::Object(buf_id));
    Ok(JsValue::Object(promise))
}

// ===========================================================================
// HMAC vertical: generateKey / importKey / exportKey / sign / verify
// (`#11-crypto-subtle-full` PR-1).  Each native is a thin pipeline:
// brand-check `this` (the only sync throw) → create Promise → marshal JS
// args into the engine-independent `elidex-api-crypto` inputs → call the
// crate `ops::*` entry → settle the Promise.  ALL spec-validation lives in
// the crate; this file only marshals + maps `AlgorithmError` → DOMException.
// ===========================================================================

/// Map an engine-independent [`AlgorithmError`] to the JS exception the VM
/// throws / rejects with (DOMException, or a plain `TypeError`).
fn algorithm_error_to_vm(vm: &VmInner, err: &AlgorithmError) -> VmError {
    let msg = err.message().to_string();
    match err {
        AlgorithmError::NotSupported(_) => {
            VmError::dom_exception(vm.well_known.dom_exc_not_supported_error, msg)
        }
        AlgorithmError::Data(_) => VmError::dom_exception(vm.well_known.dom_exc_data_error, msg),
        AlgorithmError::Syntax(_) => {
            VmError::dom_exception(vm.well_known.dom_exc_syntax_error, msg)
        }
        AlgorithmError::InvalidAccess(_) => {
            VmError::dom_exception(vm.well_known.dom_exc_invalid_access_error, msg)
        }
        AlgorithmError::Operation(_) => {
            VmError::dom_exception(vm.well_known.dom_exc_operation_error, msg)
        }
        AlgorithmError::Type(_) => VmError::type_error(msg),
    }
}

/// Settle a pre-allocated Promise with the result of an operation body.
/// Brand-check failures are handled before this; every other failure
/// (marshalling `VmError` or crate `AlgorithmError` already mapped to
/// `VmError`) rejects the Promise rather than throwing synchronously.
fn settle_promise(
    ctx: &mut NativeContext<'_>,
    promise: ObjectId,
    result: Result<JsValue, VmError>,
) {
    match result {
        Ok(value) => resolve_promise_sync(ctx.vm, promise, value),
        Err(e) => {
            let reason = ctx.vm.vm_error_to_thrown(&e);
            reject_promise_sync(ctx.vm, promise, reason);
        }
    }
}

/// Brand-check a `CryptoKey` operation argument (NOT `this`).  A wrong
/// type is a WebIDL conversion `TypeError`, settled on the Promise.
fn require_crypto_key_arg(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
    param: u32,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = value {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::CryptoKey) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'SubtleCrypto': parameter {param} is not of type 'CryptoKey'."
    )))
}

/// Marshal a JS `AlgorithmIdentifier` (string, or object with `name` +
/// op-relevant `hash` / `length` members) into a [`RawAlgorithm`].  A
/// missing / `undefined` required `name` member is a `TypeError`.
fn marshal_algorithm(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<RawAlgorithm, VmError> {
    match value {
        JsValue::String(sid) => Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid))),
        JsValue::Object(id) => {
            let name_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.name))?;
            if matches!(name_val, JsValue::Undefined) {
                return Err(VmError::type_error(format!(
                    "Failed to execute '{method}' on 'SubtleCrypto': \
                     Algorithm: name: Missing or not a string"
                )));
            }
            let name_sid = coerce::to_string(ctx.vm, name_val)?;
            let name = ctx.vm.strings.get_utf8(name_sid);

            let hash_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.hash_attr))?;
            let hash = if matches!(hash_val, JsValue::Undefined) {
                None
            } else {
                Some(Box::new(marshal_algorithm(ctx, hash_val, method)?))
            };

            let length_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.length))?;
            let length = if matches!(length_val, JsValue::Undefined) {
                None
            } else {
                Some(coerce_enforce_range_u32(ctx, length_val, method)?)
            };

            Ok(RawAlgorithm { name, hash, length })
        }
        // Primitive other than string → ToString-coerce (matches Chrome,
        // e.g. `digest(42, …)` coerces "42" then rejects NotSupported).
        other => {
            let sid = coerce::to_string(ctx.vm, other)?;
            Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid)))
        }
    }
}

/// WebIDL `[EnforceRange] unsigned long` conversion for the `length`
/// member: ToNumber, then reject non-integers / out-of-range with a
/// `TypeError` (per `[EnforceRange]`, NOT the wrapping `ToUint32`).
fn coerce_enforce_range_u32(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<u32, VmError> {
    let n = coerce::to_number(ctx.vm, value)?;
    if n.is_finite() && n.fract() == 0.0 && n >= 0.0 && n <= f64::from(u32::MAX) {
        // Exact integer within range; the cast is lossless.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(n as u32)
    } else {
        Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': \
             Algorithm: length: Value is outside the 'unsigned long' value range"
        )))
    }
}

/// Marshal a JS `sequence<KeyUsage>` (an Array of enum strings) into a
/// `Vec<KeyUsage>`.  A non-Array value or an unrecognized enum string is
/// a WebIDL conversion `TypeError`.
fn marshal_usages(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<Vec<KeyUsage>, VmError> {
    let elements = read_array_elements(ctx, value).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': \
             The provided value cannot be converted to a sequence."
        ))
    })?;
    let mut usages = Vec::with_capacity(elements.len());
    for el in elements {
        let sid = coerce::to_string(ctx.vm, el)?;
        let s = ctx.vm.strings.get_utf8(sid);
        let usage = KeyUsage::from_ident(&s).ok_or_else(|| {
            VmError::type_error(format!(
                "Failed to execute '{method}' on 'SubtleCrypto': \
                 The provided value '{s}' is not a valid enum value of type KeyUsage."
            ))
        })?;
        usages.push(usage);
    }
    Ok(usages)
}

/// Marshal a JS `KeyFormat` enum string into [`KeyFormat`].
fn marshal_format(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<KeyFormat, VmError> {
    let sid = coerce::to_string(ctx.vm, value)?;
    let s = ctx.vm.strings.get_utf8(sid);
    match s.as_str() {
        "raw" => Ok(KeyFormat::Raw),
        "pkcs8" => Ok(KeyFormat::Pkcs8),
        "spki" => Ok(KeyFormat::Spki),
        "jwk" => Ok(KeyFormat::Jwk),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': \
             The provided value '{s}' is not a valid enum value of type KeyFormat."
        ))),
    }
}

/// Marshal a JS object into a [`JsonWebKey`] (the `oct`-relevant members).
/// A non-object value is a `TypeError` (the `jwk` branch requires a
/// dictionary).
fn marshal_jwk(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<JsonWebKey, VmError> {
    let JsValue::Object(id) = value else {
        return Err(VmError::type_error(
            "Failed to execute 'importKey' on 'SubtleCrypto': \
             The provided value is not a JSON Web Key dictionary.",
        ));
    };
    Ok(JsonWebKey {
        kty: read_optional_string(ctx, id, "kty")?,
        k: read_optional_string(ctx, id, "k")?,
        alg: read_optional_string(ctx, id, "alg")?,
        use_: read_optional_string(ctx, id, "use")?,
        key_ops: read_optional_string_array(ctx, id, "key_ops")?,
        ext: read_optional_bool(ctx, id, "ext")?,
    })
}

/// Read the `Array` elements of `value`, or `None` if it is not an Array.
fn read_array_elements(ctx: &NativeContext<'_>, value: JsValue) -> Option<Vec<JsValue>> {
    let JsValue::Object(id) = value else {
        return None;
    };
    match &ctx.vm.get_object(id).kind {
        ObjectKind::Array { elements } => Some(elements.clone()),
        _ => None,
    }
}

fn read_optional_string(
    ctx: &mut NativeContext<'_>,
    obj: ObjectId,
    member: &str,
) -> Result<Option<String>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(member));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined | JsValue::Null) {
        return Ok(None);
    }
    let sid = coerce::to_string(ctx.vm, val)?;
    Ok(Some(ctx.vm.strings.get_utf8(sid)))
}

fn read_optional_bool(
    ctx: &mut NativeContext<'_>,
    obj: ObjectId,
    member: &str,
) -> Result<Option<bool>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(member));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    Ok(Some(coerce::to_boolean(ctx.vm, val)))
}

fn read_optional_string_array(
    ctx: &mut NativeContext<'_>,
    obj: ObjectId,
    member: &str,
) -> Result<Option<Vec<String>>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(member));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined | JsValue::Null) {
        return Ok(None);
    }
    let Some(elements) = read_array_elements(ctx, val) else {
        return Err(VmError::type_error(
            "Failed to execute 'importKey' on 'SubtleCrypto': \
             JWK 'key_ops' member is not a sequence.",
        ));
    };
    let mut out = Vec::with_capacity(elements.len());
    for el in elements {
        let sid = coerce::to_string(ctx.vm, el)?;
        out.push(ctx.vm.strings.get_utf8(sid));
    }
    Ok(Some(out))
}

/// Build a fresh JS object for an exported `oct` JWK.
fn build_jwk_object(ctx: &mut NativeContext<'_>, jwk: &JsonWebKey) -> ObjectId {
    let object_proto = ctx.vm.object_prototype;
    let obj = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });
    let set_string = |ctx: &mut NativeContext<'_>, member: &str, value: &str| {
        let key = PropertyKey::String(ctx.intern(member));
        let val_sid = ctx.intern(value);
        ctx.vm.define_shaped_property(
            obj,
            key,
            PropertyValue::Data(JsValue::String(val_sid)),
            shape::PropertyAttrs::DATA,
        );
    };
    if let Some(kty) = &jwk.kty {
        set_string(ctx, "kty", kty);
    }
    if let Some(k) = &jwk.k {
        set_string(ctx, "k", k);
    }
    if let Some(alg) = &jwk.alg {
        set_string(ctx, "alg", alg);
    }
    if let Some(use_) = &jwk.use_ {
        set_string(ctx, "use", use_);
    }
    if let Some(key_ops) = &jwk.key_ops {
        let elements = key_ops
            .iter()
            .map(|s| JsValue::String(ctx.intern(s)))
            .collect::<Vec<_>>();
        let array_proto = ctx.vm.array_prototype;
        let arr = ctx.alloc_object(Object {
            kind: ObjectKind::Array { elements },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: array_proto,
            extensible: true,
        });
        let key = PropertyKey::String(ctx.intern("key_ops"));
        ctx.vm.define_shaped_property(
            obj,
            key,
            PropertyValue::Data(JsValue::Object(arr)),
            shape::PropertyAttrs::DATA,
        );
    }
    if let Some(ext) = jwk.ext {
        let key = PropertyKey::String(ctx.intern("ext"));
        ctx.vm.define_shaped_property(
            obj,
            key,
            PropertyValue::Data(JsValue::Boolean(ext)),
            shape::PropertyAttrs::DATA,
        );
    }
    obj
}

/// Run an operation body against a pre-rooted Promise, settling it.
/// Shared shape for the five operation natives.
fn run_op(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    body: impl FnOnce(&mut NativeContext<'_>) -> Result<JsValue, VmError>,
) -> Result<JsValue, VmError> {
    require_subtle_crypto_this(ctx, this, method)?;
    let promise = create_promise(ctx.vm);
    let mut guard = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted = NativeContext::new_call(&mut guard);
    let ctx = &mut rooted;
    let result = body(ctx);
    settle_promise(ctx, promise, result);
    Ok(JsValue::Object(promise))
}

fn native_subtle_crypto_generate_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "generateKey", move |ctx| {
        let algorithm_arg = args.first().copied().unwrap_or(JsValue::Undefined);
        let extractable =
            coerce::to_boolean(ctx.vm, args.get(1).copied().unwrap_or(JsValue::Undefined));
        let usages_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);

        let raw = marshal_algorithm(ctx, algorithm_arg, "generateKey")?;
        let normalized = crypto::normalize(Operation::GenerateKey, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let usages = marshal_usages(ctx, usages_arg, "generateKey")?;

        let NormalizedAlgorithm::HmacKeyParams { hash, length } = normalized else {
            return Err(algorithm_error_to_vm(
                ctx.vm,
                &AlgorithmError::NotSupported("algorithm is not supported for generateKey".into()),
            ));
        };
        let byte_len = crypto::hmac::generate_key_byte_len(hash, length)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let mut bytes = vec![0u8; byte_len];
        getrandom::fill(&mut bytes).map_err(|e| {
            VmError::dom_exception(
                ctx.vm.well_known.dom_exc_operation_error,
                format!(
                    "Failed to execute 'generateKey' on 'SubtleCrypto': OS CSPRNG failure ({e})"
                ),
            )
        })?;
        let key_data = crypto::ops::generate_key(normalized, extractable, usages, &bytes)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let id = ctx.vm.alloc_crypto_key(key_data);
        Ok(JsValue::Object(id))
    })
}

fn native_subtle_crypto_import_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "importKey", move |ctx| {
        let format = marshal_format(
            ctx,
            args.first().copied().unwrap_or(JsValue::Undefined),
            "importKey",
        )?;
        let key_data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let algorithm_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        let extractable =
            coerce::to_boolean(ctx.vm, args.get(3).copied().unwrap_or(JsValue::Undefined));
        let usages_arg = args.get(4).copied().unwrap_or(JsValue::Undefined);

        let raw = marshal_algorithm(ctx, algorithm_arg, "importKey")?;
        let normalized = crypto::normalize(Operation::ImportKey, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let usages = marshal_usages(ctx, usages_arg, "importKey")?;

        let key_data = match format {
            KeyFormat::Jwk => KeyData::Jwk(marshal_jwk(ctx, key_data_arg)?),
            _ => KeyData::Raw(extract_buffer_source_bytes(
                ctx,
                key_data_arg,
                "Failed to execute 'importKey' on 'SubtleCrypto'",
                2,
                false,
            )?),
        };
        let key = crypto::ops::import_key(format, normalized, extractable, usages, key_data)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let id = ctx.vm.alloc_crypto_key(key);
        Ok(JsValue::Object(id))
    })
}

fn native_subtle_crypto_export_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "exportKey", move |ctx| {
        let format = marshal_format(
            ctx,
            args.first().copied().unwrap_or(JsValue::Undefined),
            "exportKey",
        )?;
        let key_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let key_id = require_crypto_key_arg(ctx, key_arg, "exportKey", 2)?;
        let key_data = ctx.vm.crypto_key_states[&key_id].clone();
        let exported = crypto::ops::export_key(format, &key_data)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        match exported {
            ExportedKey::Raw(bytes) => {
                let buf = create_array_buffer_from_bytes(ctx.vm, bytes);
                Ok(JsValue::Object(buf))
            }
            ExportedKey::Jwk(jwk) => Ok(JsValue::Object(build_jwk_object(ctx, &jwk))),
        }
    })
}

fn native_subtle_crypto_sign(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "sign", move |ctx| {
        let algorithm_arg = args.first().copied().unwrap_or(JsValue::Undefined);
        let key_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let data_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);

        let raw = marshal_algorithm(ctx, algorithm_arg, "sign")?;
        let normalized = crypto::normalize(Operation::Sign, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let key_id = require_crypto_key_arg(ctx, key_arg, "sign", 2)?;
        let data = extract_buffer_source_bytes(
            ctx,
            data_arg,
            "Failed to execute 'sign' on 'SubtleCrypto'",
            3,
            false,
        )?;
        let key_data = ctx.vm.crypto_key_states[&key_id].clone();
        let signature = crypto::ops::sign(normalized, &key_data, &data)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let buf = create_array_buffer_from_bytes(ctx.vm, signature);
        Ok(JsValue::Object(buf))
    })
}

fn native_subtle_crypto_verify(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "verify", move |ctx| {
        let algorithm_arg = args.first().copied().unwrap_or(JsValue::Undefined);
        let key_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let signature_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        let data_arg = args.get(3).copied().unwrap_or(JsValue::Undefined);

        let raw = marshal_algorithm(ctx, algorithm_arg, "verify")?;
        let normalized = crypto::normalize(Operation::Verify, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let key_id = require_crypto_key_arg(ctx, key_arg, "verify", 2)?;
        let signature = extract_buffer_source_bytes(
            ctx,
            signature_arg,
            "Failed to execute 'verify' on 'SubtleCrypto'",
            3,
            false,
        )?;
        let data = extract_buffer_source_bytes(
            ctx,
            data_arg,
            "Failed to execute 'verify' on 'SubtleCrypto'",
            4,
            false,
        )?;
        let key_data = ctx.vm.crypto_key_states[&key_id].clone();
        let ok = crypto::ops::verify(normalized, &key_data, &signature, &data)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        Ok(JsValue::Boolean(ok))
    })
}
