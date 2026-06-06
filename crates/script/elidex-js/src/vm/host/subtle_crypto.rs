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
    self as crypto, AlgorithmError, ExportedKey, JsonWebKey, KeyData, KeyFormat,
    NormalizedAlgorithm, Operation, RawAlgorithm,
};

use super::super::coerce;
use super::super::natives_promise::create_promise;
use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::webidl_sequence::{webidl_sequence_to_vec, SeqMessages};
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

fn native_subtle_crypto_digest(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "digest", move |ctx| {
        // Web IDL converts every argument in order before the digest
        // operation normalizes the algorithm: the `algorithm`
        // `(object or DOMString)` conversion (arg 1) runs *first* — so
        // `digest(Symbol(), 123)` rejects for the algorithm `TypeError`,
        // not the `data` one — then the `data` BufferSource snapshot
        // (§13.2; required, `allow_undefined_as_empty: false`).  Only then
        // is the algorithm normalized (`marshal_algorithm` reads `name`;
        // name-only — `Operation::Digest` ignores `hash` / `length`).
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let bytes = extract_buffer_source_bytes(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "Failed to execute 'digest' on 'SubtleCrypto'",
            2,
            false,
        )?;
        let raw = marshal_algorithm(ctx, algorithm, "digest", Operation::Digest)?;
        let normalized = crypto::normalize(Operation::Digest, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let NormalizedAlgorithm::Digest(hash) = normalized else {
            return Err(algorithm_error_to_vm(
                ctx.vm,
                &AlgorithmError::NotSupported("algorithm is not supported for digest".into()),
            ));
        };
        let buf = create_array_buffer_from_bytes(ctx.vm, hash.digest(&bytes));
        Ok(JsValue::Object(buf))
    })
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
///
/// Confirms the side-store entry exists alongside the `ObjectKind` brand
/// so the subsequent `crypto_key_states[&id]` index cannot panic (a
/// brand surviving without its entry — e.g. retained across `Vm::unbind`
/// — surfaces as the same not-a-CryptoKey `TypeError`).
fn require_crypto_key_arg(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
    param: u32,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = value {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::CryptoKey)
            && ctx.vm.crypto_key_states.contains_key(&id)
        {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'SubtleCrypto': parameter {param} is not of type 'CryptoKey'."
    )))
}

/// Web IDL conversion of an `AlgorithmIdentifier` argument to its
/// `(object or DOMString)` union form — the **argument-conversion** step,
/// run before the operation's later arguments and before normalization.
///
/// An Object is kept as-is (its members are read later, at normalize
/// time); any other value is coerced to a `DOMString` via `ToString`,
/// which throws for a `Symbol` / a BigInt-less primitive.  Hoisting this
/// ahead of the other argument conversions makes `digest(Symbol(), 123)`
/// reject for the first argument (a `TypeError` from the algorithm), not
/// the second.
fn convert_algorithm_identifier(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<JsValue, VmError> {
    match value {
        JsValue::Object(_) | JsValue::String(_) => Ok(value),
        // Not an object → the `DOMString` arm: ToString-coerce (matches
        // Chrome, e.g. `digest(42, …)` coerces "42"; `Symbol()` throws).
        other => Ok(JsValue::String(coerce::to_string(ctx.vm, other)?)),
    }
}

/// Marshal an already-[`convert_algorithm_identifier`]-converted
/// `AlgorithmIdentifier` (an Object, or a `DOMString`) into a
/// [`RawAlgorithm`] for operation `op` — the **operation** step (§18.4.4
/// normalization), run after every argument has been converted.  A missing
/// / `undefined` required `name` member is a `TypeError`.
///
/// The `hash` / `length` members are read **only** for the operations
/// whose params dictionaries carry them (`generateKey` / `importKey` —
/// `HmacKeyGenParams` / `HmacImportParams`) **and** only once the `name`
/// has been recognized against the registry for `op`.  This mirrors
/// §18.4.4: step 5 recognizes `algName` (returning `NotSupportedError`
/// for an unregistered pair) *before* step 6 converts `alg` to the params
/// dictionary, which is what reads `hash` / `length`.  An unregistered
/// name (e.g. `generateKey({name:'AES-Magic', get hash(){throw}})`) must
/// therefore reject with `NotSupportedError` and never fire the getter.
/// For `digest` / `sign` / `verify` the identifier is name-only (the
/// spec's `Algorithm` dict), so those members are not consulted either.
///
/// Bounding the recursion: the nested `hash` is a
/// [`marshal_hash_identifier`] **leaf** (a `HashAlgorithmIdentifier` never
/// has its own `hash`), so a self-referential / deeply-nested algorithm
/// object cannot recurse.
fn marshal_algorithm(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
    op: Operation,
) -> Result<RawAlgorithm, VmError> {
    let reads_key_params = matches!(op, Operation::GenerateKey | Operation::ImportKey);
    match value {
        JsValue::String(sid) => Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid))),
        JsValue::Object(id) => {
            let name = read_required_name(ctx, id, method)?;
            // §18.4.4 step 5 recognition gate: only read the params-
            // dictionary getters (step 6) for a registered `(op, name)`.
            // An unrecognized name yields a name-only `RawAlgorithm`,
            // which `crypto::normalize` rejects as `NotSupportedError`
            // without ever touching `hash` / `length`.
            let (hash, length) = if reads_key_params && crypto::is_supported(op, &name) {
                // §18.4.4 step 6: converting to the params dictionary
                // (`HmacKeyGenParams` / `HmacImportParams`, both of which
                // inherit `Algorithm`) re-reads the required inherited
                // `name` member — before the derived `hash` / `length`, in
                // dictionary member order.  The recognized name from step 5
                // is authoritative (step 7), so the second read's *value*
                // is discarded, but its getter still fires, so a throw (or
                // a now-missing `name`) on the second access propagates.
                read_required_name(ctx, id, method)?;
                let hash_val =
                    ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.hash_attr))?;
                let hash = if matches!(hash_val, JsValue::Undefined) {
                    None
                } else {
                    Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?))
                };
                let length_val =
                    ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.length))?;
                let length = if matches!(length_val, JsValue::Undefined) {
                    None
                } else {
                    Some(coerce_enforce_range_u32(ctx, length_val, method)?)
                };
                (hash, length)
            } else {
                (None, None)
            };
            Ok(RawAlgorithm { name, hash, length })
        }
        // `value` is post-[`convert_algorithm_identifier`], so it is always
        // an Object or a `String` — any other variant is a caller bug.
        other => unreachable!("algorithm must be converted first, got {other:?}"),
    }
}

/// Marshal a `HashAlgorithmIdentifier` (string or `{name}`) — a **leaf**
/// digest identifier with no nested `hash`/`length`, so it cannot recurse.
fn marshal_hash_identifier(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<RawAlgorithm, VmError> {
    match value {
        JsValue::String(sid) => Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid))),
        JsValue::Object(id) => Ok(RawAlgorithm::from_name(read_required_name(
            ctx, id, method,
        )?)),
        other => {
            let sid = coerce::to_string(ctx.vm, other)?;
            Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid)))
        }
    }
}

/// Read the required `name` member of an algorithm dictionary; a missing
/// / `undefined` value is a `TypeError` (per Web IDL `required DOMString`).
fn read_required_name(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    method: &str,
) -> Result<String, VmError> {
    let name_val = ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.name))?;
    if matches!(name_val, JsValue::Undefined) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': \
             Algorithm: name: Missing or not a string"
        )));
    }
    let name_sid = coerce::to_string(ctx.vm, name_val)?;
    Ok(ctx.vm.strings.get_utf8(name_sid))
}

/// WebIDL `[EnforceRange] unsigned long` conversion for the `length`
/// member (Web IDL §3.3.6 `[EnforceRange]` / ConvertToInt step 6):
/// ToNumber, reject NaN / ±∞ with a `TypeError`, then take `IntegerPart`
/// (**truncate toward zero**) and range-check.  A finite fractional value
/// such as `31.9` is therefore accepted as `31` — NOT rejected (and NOT
/// the wrapping `ToUint32`).
fn coerce_enforce_range_u32(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<u32, VmError> {
    let n = coerce::to_number(ctx.vm, value)?;
    // Web IDL: NaN / ±∞ throw before truncation; otherwise IntegerPart
    // truncates toward zero, then the result is bounds-checked.
    let truncated = n.trunc();
    if n.is_finite() && truncated >= 0.0 && truncated <= f64::from(u32::MAX) {
        // Truncated integer within range; the cast is lossless.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(truncated as u32)
    } else {
        Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': \
             Algorithm: length: Value is outside the 'unsigned long' value range"
        )))
    }
}

/// Cap on every `SubtleCrypto` `sequence<T>` conversion (`keyUsages`, JWK
/// `key_ops` / `oth`) — bounds a script-controlled iterable whose `.next()`
/// never reports `done` (which would otherwise hang the Promise forever).
/// Far above any legitimate list (there are only a handful of `KeyUsage`
/// values); mirrors the `dom_inner_html` shadow-roots cap.
const MAX_CRYPTO_SEQUENCE_LEN: usize = 4096;

/// `SubtleCrypto` `sequence<T>` conversion: WebIDL §3.10.16 **step 1**
/// (a non-Object value — notably a **string primitive** — is a `TypeError`
/// before `@@iterator` is consulted, matching Chrome, which rejects e.g.
/// `generateKey(…, 'sign')`) followed by the canonical
/// [`webidl_sequence_to_vec`] (steps 2-3 + IteratorClose precedence + the
/// runaway cap).
///
/// The shared helper does not itself apply step 1 — several existing
/// callers (`Blob` / `TouchEvent` / `getHTML`) instead iterate a string's
/// code points, a pre-existing project-wide divergence from the current
/// spec.  Until that is unified, crypto applies step 1 here so the new
/// `SubtleCrypto` surface is spec-faithful.
fn crypto_sequence_to_vec<T, F>(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    msgs: &SeqMessages<'_>,
    validator: F,
) -> Result<Vec<T>, VmError>
where
    F: FnMut(&mut NativeContext<'_>, usize, JsValue) -> Result<T, VmError>,
{
    if !matches!(value, JsValue::Object(_)) {
        return Err(VmError::type_error(msgs.not_iterable.to_owned()));
    }
    webidl_sequence_to_vec(ctx, value, MAX_CRYPTO_SEQUENCE_LEN, msgs, validator)
}

/// Marshal a JS `sequence<KeyUsage>` into a `Vec<KeyUsage>` (WebIDL
/// §3.10.16): any iterable Object is accepted, a string primitive / other
/// non-Object value is a conversion `TypeError`, and an unrecognized enum
/// string is a `TypeError`.  Delegates the iterator protocol (IteratorClose
/// precedence, runaway cap) to the canonical [`webidl_sequence_to_vec`].
fn marshal_usages(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<Vec<KeyUsage>, VmError> {
    let prefix = format!("Failed to execute '{method}' on 'SubtleCrypto'");
    let not_iterable = format!("{prefix}: The provided value cannot be converted to a sequence.");
    let iter_not_object = format!("{prefix}: keyUsages @@iterator must return an object.");
    let cap_exceeded =
        format!("{prefix}: keyUsages exceeds the maximum length of {MAX_CRYPTO_SEQUENCE_LEN}.");
    let msgs = SeqMessages {
        not_iterable: &not_iterable,
        iter_not_object: &iter_not_object,
        cap_exceeded: &cap_exceeded,
    };
    crypto_sequence_to_vec(ctx, value, &msgs, |ctx, _idx, el| {
        let sid = coerce::to_string(ctx.vm, el)?;
        let s = ctx.vm.strings.get_utf8(sid);
        KeyUsage::from_ident(&s).ok_or_else(|| {
            VmError::type_error(format!(
                "{prefix}: The provided value '{s}' is not a valid enum value of type KeyUsage."
            ))
        })
    })
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

/// Marshal a JS value into a [`JsonWebKey`] via Web IDL dictionary
/// conversion.
///
/// - `null` / `undefined` convert to an **empty** dictionary (all members
///   absent); the HMAC import path then rejects it with `DataError`
///   (missing `kty` / `k`), not a `TypeError`.
/// - A non-object, non-nullish value cannot be a dictionary → `TypeError`.
/// - For an object, every declared `JsonWebKey` member is read **in
///   lexicographic identifier order**, firing each getter and propagating
///   its throws — even members HMAC ignores (the EC / RSA fields).  Only
///   the `oct`-relevant subset (`kty` / `use` / `key_ops` / `alg` / `ext`
///   / `k`) is retained.
fn marshal_jwk(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<JsonWebKey, VmError> {
    let id = match value {
        JsValue::Undefined | JsValue::Null => return Ok(JsonWebKey::default()),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'importKey' on 'SubtleCrypto': \
                 The provided value is not a JSON Web Key dictionary.",
            ))
        }
    };
    // Lexicographic identifier order of `JsonWebKey` members (WebCrypto
    // §15): alg, crv, d, dp, dq, e, ext, k, key_ops, kty, n, oth, p, q,
    // qi, use, x, y.  Read every member (firing getters / propagating
    // throws); retain only the oct subset.
    let alg = read_jwk_string(ctx, id, "alg")?;
    read_jwk_string(ctx, id, "crv")?;
    read_jwk_string(ctx, id, "d")?;
    read_jwk_string(ctx, id, "dp")?;
    read_jwk_string(ctx, id, "dq")?;
    read_jwk_string(ctx, id, "e")?;
    let ext = read_jwk_bool(ctx, id, "ext")?;
    let k = read_jwk_string(ctx, id, "k")?;
    let key_ops = read_jwk_key_ops(ctx, id)?;
    let kty = read_jwk_string(ctx, id, "kty")?;
    read_jwk_string(ctx, id, "n")?;
    read_jwk_oth(ctx, id)?;
    read_jwk_string(ctx, id, "p")?;
    read_jwk_string(ctx, id, "q")?;
    read_jwk_string(ctx, id, "qi")?;
    let use_ = read_jwk_string(ctx, id, "use")?;
    read_jwk_string(ctx, id, "x")?;
    read_jwk_string(ctx, id, "y")?;
    Ok(JsonWebKey {
        kty,
        k,
        alg,
        use_,
        key_ops,
        ext,
    })
}

/// Read a `DOMString` `JsonWebKey` member (Web IDL): `undefined` → absent
/// (`None`); any other present value (including `null` → `"null"`) is
/// converted via `ToString`.  Reading fires the member's getter.
fn read_jwk_string(
    ctx: &mut NativeContext<'_>,
    obj: ObjectId,
    member: &str,
) -> Result<Option<String>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(member));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = coerce::to_string(ctx.vm, val)?;
    Ok(Some(ctx.vm.strings.get_utf8(sid)))
}

/// Read the `boolean ext` `JsonWebKey` member: `undefined` → absent; any
/// other value via `ToBoolean` (`null` → `false`).
fn read_jwk_bool(
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

/// Read the `sequence<DOMString> key_ops` `JsonWebKey` member: `undefined`
/// → absent; otherwise a Web IDL sequence conversion (any iterable, each
/// element via `ToString`).  A non-iterable present value is a `TypeError`.
fn read_jwk_key_ops(
    ctx: &mut NativeContext<'_>,
    obj: ObjectId,
) -> Result<Option<Vec<String>>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern("key_ops"));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    let msgs = SeqMessages {
        not_iterable: "Failed to execute 'importKey' on 'SubtleCrypto': \
                       JWK 'key_ops' member is not a sequence.",
        iter_not_object: "Failed to execute 'importKey' on 'SubtleCrypto': \
                          JWK 'key_ops' @@iterator must return an object.",
        cap_exceeded: "Failed to execute 'importKey' on 'SubtleCrypto': \
                       JWK 'key_ops' exceeds the maximum length.",
    };
    let out = crypto_sequence_to_vec(ctx, val, &msgs, |ctx, _idx, el| {
        let sid = coerce::to_string(ctx.vm, el)?;
        Ok(ctx.vm.strings.get_utf8(sid))
    })?;
    Ok(Some(out))
}

/// Read the `sequence<RsaOtherPrimesInfo> oth` `JsonWebKey` member, fully
/// converting (then discarding) each entry per Web IDL.  `undefined` →
/// absent; otherwise the value is converted to a sequence (a non-iterable
/// such as `oth: 123` → `TypeError`), and each entry is converted to an
/// `RsaOtherPrimesInfo` dictionary: `undefined` / `null` → an empty dict,
/// an object → its (optional) `d` / `r` / `t` `DOMString` members read in
/// lexicographic order (firing each getter), any other value → `TypeError`
/// (e.g. `oth: [123]`).  HMAC never consults `oth`, but Web IDL dictionary
/// conversion still performs the full member walk; the converted values
/// are retained once the RSA vertical (`#11-crypto-subtle-full` PR-5)
/// extends [`JsonWebKey`] to carry them.
fn read_jwk_oth(ctx: &mut NativeContext<'_>, obj: ObjectId) -> Result<(), VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern("oth"));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(());
    }
    let msgs = SeqMessages {
        not_iterable: "Failed to execute 'importKey' on 'SubtleCrypto': \
                       JWK 'oth' member is not a sequence.",
        iter_not_object: "Failed to execute 'importKey' on 'SubtleCrypto': \
                          JWK 'oth' @@iterator must return an object.",
        cap_exceeded: "Failed to execute 'importKey' on 'SubtleCrypto': \
                       JWK 'oth' exceeds the maximum length.",
    };
    // Returns `Vec<()>` — `oth` entries are converted (firing getters) for
    // Web IDL conformance, then discarded (HMAC ignores them).
    crypto_sequence_to_vec(ctx, val, &msgs, |ctx, _idx, el| {
        let entry = match el {
            // `null` / `undefined` → an empty RsaOtherPrimesInfo dict.
            JsValue::Undefined | JsValue::Null => return Ok(()),
            JsValue::Object(id) => id,
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'importKey' on 'SubtleCrypto': \
                     JWK 'oth' entry is not an RsaOtherPrimesInfo dictionary.",
                ))
            }
        };
        // RsaOtherPrimesInfo members in lexicographic order (d, r, t), all
        // optional `DOMString`s — read (firing getters), discard.
        read_jwk_string(ctx, entry, "d")?;
        read_jwk_string(ctx, entry, "r")?;
        read_jwk_string(ctx, entry, "t")?;
        Ok(())
    })?;
    Ok(())
}

/// Build a fresh JS object for an exported `oct` JWK.
///
/// The intermediate `obj` / `key_ops` array are not separately rooted
/// across the inner `alloc_object` calls: GC is disabled for the whole
/// duration of a `NativeFunction` call (`interpreter.rs` /
/// `dispatch.rs` set `gc_enabled = false`; see `natives_array_hof.rs`),
/// so `alloc_object` here never triggers a collection. Add temp-roots
/// only if GC is ever permitted to run during native→JS callbacks.
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
/// Shared shape for the six operation natives (digest + the five HMAC ops).
///
/// WebCrypto §14.3 reports **all** errors asynchronously, including the
/// Web IDL receiver brand check: a non-`SubtleCrypto` `this` (e.g.
/// `crypto.subtle.sign.call({}, …)`) must reject the returned Promise, not
/// throw synchronously.  So the brand check runs *inside* the settled
/// closure, after the Promise is created.
fn run_op(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    body: impl FnOnce(&mut NativeContext<'_>) -> Result<JsValue, VmError>,
) -> Result<JsValue, VmError> {
    let promise = create_promise(ctx.vm);
    let mut guard = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted = NativeContext::new_call(&mut guard);
    let ctx = &mut rooted;
    let result = match require_subtle_crypto_this(ctx, this, method) {
        Ok(_) => body(ctx),
        Err(e) => Err(e),
    };
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
        // Web IDL converts every argument in order before the operation
        // normalizes the algorithm: the `algorithm` `(object or DOMString)`
        // conversion (arg 1) first, then `extractable`, then the
        // `keyUsages` sequence — so a `Symbol()` algorithm beats a bad
        // `keyUsages`, and a bad `keyUsages` beats NotSupportedError.
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let extractable =
            coerce::to_boolean(ctx.vm, args.get(1).copied().unwrap_or(JsValue::Undefined));
        let usages_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        let usages = marshal_usages(ctx, usages_arg, "generateKey")?;

        let raw = marshal_algorithm(ctx, algorithm, "generateKey", Operation::GenerateKey)?;
        let normalized = crypto::normalize(Operation::GenerateKey, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;

        // The crate owns usage validation → length sizing → fill ordering
        // (§31.6.3); the VM only supplies entropy via the closure, so an
        // invalid usage / zero length rejects before any buffer is sized.
        let key_data = crypto::ops::generate_key(normalized, extractable, usages, |buf| {
            getrandom::fill(buf)
                .map_err(|e| AlgorithmError::Operation(format!("OS CSPRNG failure ({e})")))
        })
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
        let usages_arg = args.get(4).copied().unwrap_or(JsValue::Undefined);

        // Web IDL converts every argument in order before the operation
        // normalizes the algorithm (§14.3.9 step 2): `format` (above),
        // `keyData` (`(BufferSource or JsonWebKey)`), the `algorithm`
        // `(object or DOMString)` conversion, `extractable`, then the
        // `keyUsages` sequence — so a JWK getter throw / `Symbol()`
        // algorithm / bad-usage TypeError beats NotSupportedError.  `null` /
        // `undefined` `keyData` converts to an empty `JsonWebKey` dictionary
        // (the import then rejects with DataError, not TypeError).
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
        let algorithm =
            convert_algorithm_identifier(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;
        let extractable =
            coerce::to_boolean(ctx.vm, args.get(3).copied().unwrap_or(JsValue::Undefined));
        let usages = marshal_usages(ctx, usages_arg, "importKey")?;

        let raw = marshal_algorithm(ctx, algorithm, "importKey", Operation::ImportKey)?;
        let normalized = crypto::normalize(Operation::ImportKey, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;

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
        // Borrow the side-store key (incl. secret material) only for the
        // pure crate call; drop it before re-borrowing `ctx.vm` to build
        // the result — avoids cloning the secret material.
        let exported = {
            let key_data = &ctx.vm.crypto_key_states[&key_id];
            crypto::ops::export_key(format, key_data)
        }
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
        // Web IDL converts the arguments in order — `algorithm`
        // `(object or DOMString)`, then `key` (CryptoKey), then `data`
        // (BufferSource) — before the sign operation normalizes the
        // algorithm, so a `Symbol()` algorithm beats a non-CryptoKey `key`,
        // and a non-CryptoKey `key` beats NotSupportedError.
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "sign",
            2,
        )?;
        let data = extract_buffer_source_bytes(
            ctx,
            args.get(2).copied().unwrap_or(JsValue::Undefined),
            "Failed to execute 'sign' on 'SubtleCrypto'",
            3,
            false,
        )?;
        let raw = marshal_algorithm(ctx, algorithm, "sign", Operation::Sign)?;
        let normalized = crypto::normalize(Operation::Sign, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let signature = {
            let key_data = &ctx.vm.crypto_key_states[&key_id];
            crypto::ops::sign(normalized, key_data, &data)
        }
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
        // Web IDL converts the arguments in order — `algorithm`
        // `(object or DOMString)`, then `key` (CryptoKey), then `signature`
        // and `data` (BufferSource) — before the verify operation
        // normalizes the algorithm, so a `Symbol()` algorithm beats a
        // non-CryptoKey `key`, which beats NotSupportedError.
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "verify",
            2,
        )?;
        let signature = extract_buffer_source_bytes(
            ctx,
            args.get(2).copied().unwrap_or(JsValue::Undefined),
            "Failed to execute 'verify' on 'SubtleCrypto'",
            3,
            false,
        )?;
        let data = extract_buffer_source_bytes(
            ctx,
            args.get(3).copied().unwrap_or(JsValue::Undefined),
            "Failed to execute 'verify' on 'SubtleCrypto'",
            4,
            false,
        )?;
        let raw = marshal_algorithm(ctx, algorithm, "verify", Operation::Verify)?;
        let normalized = crypto::normalize(Operation::Verify, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let ok = {
            let key_data = &ctx.vm.crypto_key_states[&key_id];
            crypto::ops::verify(normalized, key_data, &signature, &data)
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        Ok(JsValue::Boolean(ok))
    })
}
