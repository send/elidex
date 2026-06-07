//! The twelve `SubtleCrypto` operation natives — `digest` plus the HMAC
//! vertical (`generateKey` / `importKey` / `exportKey` / `sign` /
//! `verify`, `#11-crypto-subtle-full` PR-1), the AES `encrypt` /
//! `decrypt` (PR-2), the derive vertical (`deriveBits` / `deriveKey`,
//! PR-3a), and the wrap vertical (`wrapKey` / `unwrapKey`, PR-3b).
//!
//! Each native is a thin pipeline: [`super::run_op`] creates the
//! Promise + runs the receiver brand check (the only async-reported
//! error), the body marshals JS args into the engine-independent
//! `elidex-api-crypto` inputs (via [`super::marshal`]), calls the crate
//! `ops::*` entry, and returns the value `run_op` settles the Promise
//! with.  ALL spec-validation lives in the crate; this module only
//! marshals + maps [`AlgorithmError`] → DOMException.  The `importKey` /
//! `exportKey` JWK marshalling reads / builds the caller's JS object here
//! (`marshal_jwk` / `build_jwk_object`), but the `wrapKey` / `unwrapKey` JSON
//! round-trip is done in the crate over the `JsonWebKey` struct
//! ([`elidex_api_crypto::jwk::to_json_bytes`] / `from_json_bytes`) — WebCrypto
//! §14.3.11 step 14 / §9 require it "in the context of a new global object",
//! i.e. isolated from the page realm (no `Object.prototype.toJSON`, no
//! caller-mutated prototypes).

use elidex_api_crypto::{
    self as crypto, AlgorithmError, ExportedKey, KeyData, KeyFormat, NormalizedAlgorithm, Operation,
};

use super::super::super::coerce;
use super::super::super::value::{JsValue, NativeContext, VmError};
use super::super::super::VmInner;
use super::super::array_buffer::create_array_buffer_from_bytes;
use super::super::text_encoding::{extract_buffer_source_bytes, is_buffer_source};
use super::marshal::{
    build_crypto_key_pair, build_jwk_object, convert_algorithm_identifier, marshal_algorithm,
    marshal_format, marshal_jwk, marshal_usages, require_crypto_key_arg,
};
use super::run_op;

/// Marshal an already-[`convert_algorithm_identifier`]-converted
/// `AlgorithmIdentifier` and normalize it for `op` in one step (WebCrypto
/// §18.4.4) — the marshal+normalize unit every operation native shares.
fn marshal_normalize(
    ctx: &mut NativeContext<'_>,
    algorithm: JsValue,
    method: &str,
    op: Operation,
) -> Result<NormalizedAlgorithm, VmError> {
    let raw = marshal_algorithm(ctx, algorithm, method, op)?;
    crypto::normalize(op, raw).map_err(|e| algorithm_error_to_vm(ctx.vm, &e))
}

/// Normalize an `AlgorithmIdentifier` for a wrap/unwrap op with the §14.3.11 /
/// §14.3.12 encrypt/decrypt fallback: normalize for `primary` (`wrapKey` /
/// `unwrapKey`); on **any** error, normalize for `fallback` (`encrypt` /
/// `decrypt`) so an AES-GCM/CBC/CTR key can wrap/unwrap via its cipher op.
///
/// Each branch independently re-marshals the identifier — matching the spec's
/// double "normalize an algorithm" — so the (name-only) wrap normalize reads
/// only `name` while the encrypt normalize reads the cipher params (`iv` etc.);
/// a getter on a cipher param therefore fires once, on the fallback path.
fn normalize_wrap_with_fallback(
    ctx: &mut NativeContext<'_>,
    algorithm: JsValue,
    method: &str,
    primary: Operation,
    fallback: Operation,
) -> Result<NormalizedAlgorithm, VmError> {
    match marshal_normalize(ctx, algorithm, method, primary) {
        Ok(normalized) => Ok(normalized),
        Err(_) => marshal_normalize(ctx, algorithm, method, fallback),
    }
}

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

/// Web IDL `BufferSource` *type* conversion for a positional `data` /
/// `signature` argument: argument conversion precedes the operation's method
/// steps (Web IDL §3.6), so a non-BufferSource is a `TypeError` *before* the
/// algorithm is normalized — the algorithm's params getters never fire.  The
/// matching byte snapshot ([`extract_buffer_source_bytes`]) runs *after*
/// normalization (the WebCrypto §14.3.x "let data be … a copy of the bytes"
/// step, after the "normalize an algorithm" step), so an algorithm getter that
/// mutates the buffer is reflected in the snapshot — and a huge buffer is not
/// copied if normalization rejects first.
fn require_buffer_source_arg(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    error_prefix: &str,
    index: usize,
) -> Result<(), VmError> {
    if is_buffer_source(ctx, value) {
        Ok(())
    } else {
        Err(VmError::type_error(format!(
            "{error_prefix}: parameter {index} is not of type 'BufferSource'"
        )))
    }
}

// ---------------------------------------------------------------------------
// `SubtleCrypto.prototype.digest(algorithm, data)` (WebCrypto §14.3.5)
// ---------------------------------------------------------------------------

pub(super) fn native_subtle_crypto_digest(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "digest", move |ctx| {
        // Ordering (Web IDL §3.6 + WebCrypto §14.3.5): argument conversion runs
        // left-to-right *before* the method steps — the `algorithm`
        // `(object or DOMString)` conversion (arg 1) first (so
        // `digest(Symbol(), 123)` rejects for the algorithm `TypeError`, not
        // the `data` one), then the `data` BufferSource *type* check.  §14.3.5
        // then step 2 normalizes the algorithm (`marshal_algorithm` reads
        // `name`; name-only — `Operation::Digest` ignores `hash` / `length`),
        // and step 4 gets a copy of the data bytes.  So: data type-check →
        // normalize → data byte-snapshot (after normalization).
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        require_buffer_source_arg(
            ctx,
            data_arg,
            "Failed to execute 'digest' on 'SubtleCrypto'",
            2,
        )?;
        let normalized = marshal_normalize(ctx, algorithm, "digest", Operation::Digest)?;
        let bytes = extract_buffer_source_bytes(
            ctx,
            data_arg,
            "Failed to execute 'digest' on 'SubtleCrypto'",
            2,
            false,
        )?;
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
// (`#11-crypto-subtle-full` PR-1).  Same `run_op` pipeline as `digest`
// above (see the module-level doc): the Promise is created first and the
// receiver brand check runs *inside* the settled closure, so a bad
// receiver — like any later marshalling error — rejects the Promise rather
// than throwing synchronously (WebCrypto §14.3 async-error contract).
// ===========================================================================

pub(super) fn native_subtle_crypto_generate_key(
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

        let normalized = marshal_normalize(ctx, algorithm, "generateKey", Operation::GenerateKey)?;

        // The crate owns usage validation → length sizing / curve keygen →
        // fill ordering (§14.3.6 + the per-algorithm steps); the VM only
        // supplies entropy via the closure, so an invalid usage / zero length
        // rejects before any buffer is sized.
        let generated = crypto::ops::generate_key(normalized, extractable, usages, |buf| {
            getrandom::fill(buf)
                .map_err(|e| AlgorithmError::Operation(format!("OS CSPRNG failure ({e})")))
        })
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        match generated {
            // Symmetric (HMAC / AES): one `CryptoKey` wrapper (as before).
            crypto::GeneratedKey::Single(key_data) => {
                Ok(JsValue::Object(ctx.vm.alloc_crypto_key(key_data)))
            }
            // Asymmetric (EC): two wrappers assembled into a `CryptoKeyPair`
            // dict (§14.3.6 union → §17).  GC is disabled for the whole native
            // call, so the two allocs + the dict assembly have no
            // mid-collection window.
            crypto::GeneratedKey::Pair { public, private } => {
                let public_id = ctx.vm.alloc_crypto_key(public);
                let private_id = ctx.vm.alloc_crypto_key(private);
                Ok(JsValue::Object(build_crypto_key_pair(
                    ctx, public_id, private_id,
                )))
            }
        }
    })
}

pub(super) fn native_subtle_crypto_import_key(
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

        let normalized = marshal_normalize(ctx, algorithm, "importKey", Operation::ImportKey)?;

        let key = crypto::ops::import_key(format, normalized, extractable, usages, key_data)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let id = ctx.vm.alloc_crypto_key(key);
        Ok(JsValue::Object(id))
    })
}

pub(super) fn native_subtle_crypto_export_key(
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

pub(super) fn native_subtle_crypto_sign(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "sign", move |ctx| {
        // Ordering (Web IDL §3.6 + WebCrypto §14.3.3): argument conversion runs
        // left-to-right *before* the method steps — `algorithm`
        // `(object or DOMString)`, then `key` (CryptoKey), then the `data`
        // `BufferSource` *type* check — so a `Symbol()` algorithm beats a
        // non-CryptoKey `key`, which beats a non-BufferSource `data`.  §14.3.3
        // then step 2 normalizes the algorithm and step 4 gets a copy of the
        // data bytes.  So: data type-check → normalize → data byte-snapshot.
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "sign",
            2,
        )?;
        let data_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        require_buffer_source_arg(
            ctx,
            data_arg,
            "Failed to execute 'sign' on 'SubtleCrypto'",
            3,
        )?;
        let normalized = marshal_normalize(ctx, algorithm, "sign", Operation::Sign)?;
        let data = extract_buffer_source_bytes(
            ctx,
            data_arg,
            "Failed to execute 'sign' on 'SubtleCrypto'",
            3,
            false,
        )?;
        let signature = {
            let key_data = &ctx.vm.crypto_key_states[&key_id];
            crypto::ops::sign(normalized, key_data, &data)
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let buf = create_array_buffer_from_bytes(ctx.vm, signature);
        Ok(JsValue::Object(buf))
    })
}

pub(super) fn native_subtle_crypto_verify(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "verify", move |ctx| {
        // Ordering (Web IDL §3.6 + WebCrypto §14.3.4): argument conversion runs
        // left-to-right *before* the method steps — `algorithm`
        // `(object or DOMString)`, then `key` (CryptoKey), then the `signature`
        // and `data` `BufferSource` *type* checks — so a `Symbol()` algorithm
        // beats a non-CryptoKey `key`, which beats a non-BufferSource
        // `signature` / `data`.  §14.3.4 then step 2 normalizes the algorithm,
        // step 4 copies the signature bytes, step 5 copies the data bytes.  So:
        // type-checks → normalize → byte-snapshots (after normalization).
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "verify",
            2,
        )?;
        let signature_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        let data_arg = args.get(3).copied().unwrap_or(JsValue::Undefined);
        require_buffer_source_arg(
            ctx,
            signature_arg,
            "Failed to execute 'verify' on 'SubtleCrypto'",
            3,
        )?;
        require_buffer_source_arg(
            ctx,
            data_arg,
            "Failed to execute 'verify' on 'SubtleCrypto'",
            4,
        )?;
        let normalized = marshal_normalize(ctx, algorithm, "verify", Operation::Verify)?;
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
        let ok = {
            let key_data = &ctx.vm.crypto_key_states[&key_id];
            crypto::ops::verify(normalized, key_data, &signature, &data)
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        Ok(JsValue::Boolean(ok))
    })
}

// ===========================================================================
// AES vertical: encrypt / decrypt (`#11-crypto-subtle-full` PR-2).  Same
// `run_op` pipeline as `sign` above; the algorithm `(object or DOMString)`
// conversion (arg 1) runs first, then the `CryptoKey` brand check (arg 2),
// then the `data` BufferSource snapshot (arg 3) — so a `Symbol()` algorithm
// beats a non-CryptoKey `key`, which beats NotSupportedError.  All cipher
// math + validation live in `elidex-api-crypto`; the result is an
// `ArrayBuffer` (digest's return shape).
// ===========================================================================

pub(super) fn native_subtle_crypto_encrypt(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "encrypt", move |ctx| {
        run_cipher(
            ctx,
            &args,
            "encrypt",
            Operation::Encrypt,
            crypto::ops::encrypt,
        )
    })
}

pub(super) fn native_subtle_crypto_decrypt(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "decrypt", move |ctx| {
        run_cipher(
            ctx,
            &args,
            "decrypt",
            Operation::Decrypt,
            crypto::ops::decrypt,
        )
    })
}

/// The crate `encrypt` / `decrypt` entry point shared by [`run_cipher`].
type CipherOp =
    fn(NormalizedAlgorithm, &crypto::CryptoKeyData, &[u8]) -> Result<Vec<u8>, AlgorithmError>;

/// Shared `encrypt` / `decrypt` body: marshal `(algorithm, key, data)`,
/// normalize for `op`, run the crate `cipher_op`, and wrap the bytes in an
/// `ArrayBuffer`.
fn run_cipher(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &'static str,
    op: Operation,
    cipher_op: CipherOp,
) -> Result<JsValue, VmError> {
    // Argument/operation ordering (Web IDL §3.6 + WebCrypto §14.3.1/§14.3.2):
    //  1. Web IDL converts every argument left-to-right *before* the method
    //     steps run — algorithm `(object or DOMString)`, key `CryptoKey`
    //     brand, and the `data` `BufferSource` **type** check.  A non-
    //     BufferSource `data` is therefore a TypeError before the algorithm
    //     is normalized (so its params getters never fire).
    //  2. §14.3.x step 2 normalizes the algorithm (which reads + snapshots
    //     the AES `iv` / `counter` / `additionalData` members), THEN step 4
    //     gets a copy of the data bytes (detached → error).
    // So: data type-check → normalize → data byte-copy.
    let algorithm =
        convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    let key_id = require_crypto_key_arg(
        ctx,
        args.get(1).copied().unwrap_or(JsValue::Undefined),
        method,
        2,
    )?;
    let data_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    let error_prefix = format!("Failed to execute '{method}' on 'SubtleCrypto'");
    // Web IDL `data` BufferSource *type* conversion (bind time, no byte copy /
    // no detached check) — runs before the algorithm normalization.
    require_buffer_source_arg(ctx, data_arg, &error_prefix, 3)?;
    let normalized = marshal_normalize(ctx, algorithm, method, op)?;
    // §14.3.x step 4: get a copy of the data bytes (detached → TypeError),
    // after normalization.
    let data = extract_buffer_source_bytes(ctx, data_arg, &error_prefix, 3, false)?;
    let bytes = {
        let key_data = &ctx.vm.crypto_key_states[&key_id];
        cipher_op(normalized, key_data, &data)
    }
    .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
    let buf = create_array_buffer_from_bytes(ctx.vm, bytes);
    Ok(JsValue::Object(buf))
}

// ===========================================================================
// Derive vertical: deriveBits / deriveKey (`#11-crypto-subtle-full` PR-3a).
// Same `run_op` pipeline as `sign` above; Web IDL converts every argument
// left-to-right *before* the method steps normalize the algorithm(s) — so a
// `Symbol()` algorithm beats a non-CryptoKey `baseKey`, which beats a bad
// `length` / `keyUsages`, which beats NotSupportedError.  All KDF math +
// composition live in `elidex-api-crypto`; the VM only marshals + normalizes
// (deriveKey normalizes the `derivedKeyType` twice — for importKey and for
// get-key-length — the §14.3.7 two-algorithm pattern).
// ===========================================================================

pub(super) fn native_subtle_crypto_derive_bits(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "deriveBits", move |ctx| {
        // Web IDL arg conversion (§3.6) left-to-right: algorithm
        // `(object or DOMString)`, baseKey `CryptoKey`, then `length`
        // (`optional unsigned long? = null`).  §14.3.8 then step 2 normalizes.
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let base_key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "deriveBits",
            2,
        )?;
        let length =
            marshal_optional_length(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;
        let normalized = marshal_normalize(ctx, algorithm, "deriveBits", Operation::DeriveBits)?;
        let bits = {
            let base_key = &ctx.vm.crypto_key_states[&base_key_id];
            crypto::ops::derive_bits(normalized, base_key, length)
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let buf = create_array_buffer_from_bytes(ctx.vm, bits);
        Ok(JsValue::Object(buf))
    })
}

pub(super) fn native_subtle_crypto_derive_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "deriveKey", move |ctx| {
        // Web IDL arg conversion left-to-right: algorithm `(object or
        // DOMString)`, baseKey `CryptoKey`, derivedKeyType `(object or
        // DOMString)`, extractable `boolean`, keyUsages `sequence<KeyUsage>`.
        let algorithm =
            convert_algorithm_identifier(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
        let base_key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "deriveKey",
            2,
        )?;
        let derived_key_type =
            convert_algorithm_identifier(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;
        let extractable =
            coerce::to_boolean(ctx.vm, args.get(3).copied().unwrap_or(JsValue::Undefined));
        let usages = marshal_usages(
            ctx,
            args.get(4).copied().unwrap_or(JsValue::Undefined),
            "deriveKey",
        )?;

        // §14.3.7 steps 2/4/6: normalize the base algorithm (op deriveBits)
        // then the derivedKeyType twice (op importKey, then op get-key-length)
        // — each normalize independently reads the dict members (firing
        // getters in that step order, propagating any throw).
        let derive_alg = marshal_normalize(ctx, algorithm, "deriveKey", Operation::DeriveBits)?;
        let import_alg =
            marshal_normalize(ctx, derived_key_type, "deriveKey", Operation::ImportKey)?;
        let length_alg =
            marshal_normalize(ctx, derived_key_type, "deriveKey", Operation::GetKeyLength)?;

        let key_data = {
            let base_key = &ctx.vm.crypto_key_states[&base_key_id];
            crypto::ops::derive_key(
                derive_alg,
                base_key,
                import_alg,
                length_alg,
                extractable,
                usages,
            )
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let id = ctx.vm.alloc_crypto_key(key_data);
        Ok(JsValue::Object(id))
    })
}

// ===========================================================================
// Wrap vertical: wrapKey / unwrapKey (`#11-crypto-subtle-full` PR-3b).  Same
// `run_op` pipeline as `sign` above.  wrapKey wraps an exported key under a
// wrapping key (AES-KW, or the AES-GCM/CBC/CTR encrypt fallback); unwrapKey
// reverses it and imports the recovered key.  All wrap/export/import +
// composition live in `elidex-api-crypto`; the VM only marshals + normalizes
// (with the §14.3.11 / §14.3.12 wrap→encrypt / unwrap→decrypt fallback).  The
// `jwk` JSON round-trip (§14.3.11 step 14 / §9 "parse a JWK") runs in the
// crate over the `JsonWebKey` struct, realm-isolated from the page ("a new
// global object") — not over a JS object — so a page-defined
// `Object.prototype` cannot observe or hijack a wrap / unwrap.
// ===========================================================================

pub(super) fn native_subtle_crypto_wrap_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "wrapKey", move |ctx| {
        // Web IDL arg conversion (§3.6) left-to-right: format (KeyFormat),
        // key (CryptoKey), wrappingKey (CryptoKey), wrapAlgorithm
        // (object or DOMString) — so a bad `format` beats a non-CryptoKey
        // `key`, which beats a `Symbol()` `wrapAlgorithm`.
        let format = marshal_format(
            ctx,
            args.first().copied().unwrap_or(JsValue::Undefined),
            "wrapKey",
        )?;
        let key_id = require_crypto_key_arg(
            ctx,
            args.get(1).copied().unwrap_or(JsValue::Undefined),
            "wrapKey",
            2,
        )?;
        let wrapping_key_id = require_crypto_key_arg(
            ctx,
            args.get(2).copied().unwrap_or(JsValue::Undefined),
            "wrapKey",
            3,
        )?;
        let wrap_algorithm =
            convert_algorithm_identifier(ctx, args.get(3).copied().unwrap_or(JsValue::Undefined))?;
        // §14.3.11 steps 2-4: normalize op=wrapKey → on error op=encrypt.
        let normalized = normalize_wrap_with_fallback(
            ctx,
            wrap_algorithm,
            "wrapKey",
            Operation::WrapKey,
            Operation::Encrypt,
        )?;
        // §14.3.11 steps 9-15 run entirely in the crate (gate → export →
        // JSON-serialize the `jwk` form in a realm-isolated way → wrap): two
        // shared side-store borrows, no clone of the secret material.
        let wrapped = {
            let wrapping_key = &ctx.vm.crypto_key_states[&wrapping_key_id];
            let key = &ctx.vm.crypto_key_states[&key_id];
            crypto::ops::wrap_key(normalized, wrapping_key, key, format)
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let buf = create_array_buffer_from_bytes(ctx.vm, wrapped);
        Ok(JsValue::Object(buf))
    })
}

pub(super) fn native_subtle_crypto_unwrap_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let args = args.to_vec();
    run_op(ctx, this, "unwrapKey", move |ctx| {
        // Web IDL arg conversion left-to-right: format (KeyFormat), wrappedKey
        // (BufferSource *type* check — the byte copy is the later step 7),
        // unwrappingKey (CryptoKey), unwrapAlgorithm + unwrappedKeyAlgorithm
        // (object or DOMString), extractable (boolean), keyUsages
        // (sequence<KeyUsage>).
        let format = marshal_format(
            ctx,
            args.first().copied().unwrap_or(JsValue::Undefined),
            "unwrapKey",
        )?;
        let wrapped_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        require_buffer_source_arg(
            ctx,
            wrapped_arg,
            "Failed to execute 'unwrapKey' on 'SubtleCrypto'",
            2,
        )?;
        let unwrapping_key_id = require_crypto_key_arg(
            ctx,
            args.get(2).copied().unwrap_or(JsValue::Undefined),
            "unwrapKey",
            3,
        )?;
        let unwrap_algorithm =
            convert_algorithm_identifier(ctx, args.get(3).copied().unwrap_or(JsValue::Undefined))?;
        let unwrapped_key_algorithm =
            convert_algorithm_identifier(ctx, args.get(4).copied().unwrap_or(JsValue::Undefined))?;
        let extractable =
            coerce::to_boolean(ctx.vm, args.get(5).copied().unwrap_or(JsValue::Undefined));
        let usages = marshal_usages(
            ctx,
            args.get(6).copied().unwrap_or(JsValue::Undefined),
            "unwrapKey",
        )?;

        // §14.3.12 steps 2-3: normalize unwrapAlgorithm op=unwrapKey → on error
        // op=decrypt.  step 5: normalize unwrappedKeyAlgorithm op=importKey.
        let unwrap_alg = normalize_wrap_with_fallback(
            ctx,
            unwrap_algorithm,
            "unwrapKey",
            Operation::UnwrapKey,
            Operation::Decrypt,
        )?;
        let import_alg = marshal_normalize(
            ctx,
            unwrapped_key_algorithm,
            "unwrapKey",
            Operation::ImportKey,
        )?;
        // step 7: snapshot the wrappedKey bytes (after the normalizations).
        let wrapped = extract_buffer_source_bytes(
            ctx,
            wrapped_arg,
            "Failed to execute 'unwrapKey' on 'SubtleCrypto'",
            2,
            false,
        )?;

        // steps 12-14: name-match + unwrapKey-usage gate, then unwrap/decrypt.
        let bytes = {
            let unwrapping_key = &ctx.vm.crypto_key_states[&unwrapping_key_id];
            crypto::ops::unwrap_key(unwrap_alg, unwrapping_key, &wrapped)
        }
        .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;

        // step 15: "parse a JWK" (§9) for the `jwk` format, else the raw bytes.
        // The JWK parse is done in the crate over the bytes (realm-isolated per
        // §9 "new global object" — no page `Object.prototype` / `Array.prototype`
        // is consulted), NOT via a JS object in the page realm.
        let key_data = match format {
            KeyFormat::Jwk => KeyData::Jwk(
                crypto::jwk::from_json_bytes(&bytes)
                    .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?,
            ),
            _ => KeyData::Raw(bytes),
        };
        // step 16: importKey(normalizedKeyAlgorithm, format, key, extractable,
        // usages) — also raises the step-17 empty-secret-usages SyntaxError.
        let key = crypto::ops::import_key(format, import_alg, extractable, usages, key_data)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;
        let id = ctx.vm.alloc_crypto_key(key);
        Ok(JsValue::Object(id))
    })
}

/// Marshal the `deriveBits` `length` argument (Web IDL `optional unsigned
/// long? = null`): an absent / `undefined` / `null` value is `None`
/// (the §33.4.1 / §34.4.1 step-1 OperationError); any other value is the
/// default (non-`[EnforceRange]`) `unsigned long` `ToUint32` conversion.
fn marshal_optional_length(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<Option<u32>, VmError> {
    match value {
        JsValue::Undefined | JsValue::Null => Ok(None),
        v => Ok(Some(coerce::to_uint32(ctx.vm, v)?)),
    }
}
