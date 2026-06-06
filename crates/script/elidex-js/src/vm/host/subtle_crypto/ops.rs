//! The six `SubtleCrypto` operation natives — `digest` plus the HMAC
//! vertical (`generateKey` / `importKey` / `exportKey` / `sign` /
//! `verify`, `#11-crypto-subtle-full` PR-1).
//!
//! Each native is a thin pipeline: [`super::run_op`] creates the
//! Promise + runs the receiver brand check (the only async-reported
//! error), the body marshals JS args into the engine-independent
//! `elidex-api-crypto` inputs (via [`super::marshal`]), calls the crate
//! `ops::*` entry, and returns the value `run_op` settles the Promise
//! with.  ALL spec-validation lives in the crate; this module only
//! marshals + maps [`AlgorithmError`] → DOMException.

use elidex_api_crypto::{
    self as crypto, AlgorithmError, ExportedKey, KeyData, KeyFormat, NormalizedAlgorithm, Operation,
};

use super::super::super::coerce;
use super::super::super::value::{JsValue, NativeContext, VmError};
use super::super::super::VmInner;
use super::super::array_buffer::create_array_buffer_from_bytes;
use super::super::text_encoding::extract_buffer_source_bytes;
use super::marshal::{
    build_jwk_object, convert_algorithm_identifier, marshal_algorithm, marshal_format, marshal_jwk,
    marshal_usages, require_crypto_key_arg,
};
use super::run_op;

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
// the crate; this module only marshals + maps `AlgorithmError` → DOMException.
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

        let raw = marshal_algorithm(ctx, algorithm, "importKey", Operation::ImportKey)?;
        let normalized = crypto::normalize(Operation::ImportKey, &raw)
            .map_err(|e| algorithm_error_to_vm(ctx.vm, &e))?;

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

pub(super) fn native_subtle_crypto_verify(
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
