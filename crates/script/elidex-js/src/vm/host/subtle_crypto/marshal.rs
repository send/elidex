//! JS-value → `elidex-api-crypto` input marshalling for the
//! `SubtleCrypto` operations.
//!
//! Per CLAUDE.md "Layering mandate", these helpers only convert Web
//! IDL argument values into the engine-independent crate's input
//! types (algorithm-identifier conversion + normalization inputs,
//! `sequence<KeyUsage>` / `KeyFormat` / `JsonWebKey` conversion, the
//! `[EnforceRange]` length coercion) and build the exported `oct` JWK
//! object — all spec-validation lives in `elidex-api-crypto`.

use elidex_api_crypto::key::KeyUsage;
use elidex_api_crypto::{self as crypto, JsonWebKey, KeyFormat, Operation, RawAlgorithm};

use super::super::super::coerce;
use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::webidl_sequence::{webidl_sequence_to_vec, SeqMessages};

/// Brand-check a `CryptoKey` operation argument (NOT `this`).  A wrong
/// type is a WebIDL conversion `TypeError`, settled on the Promise.
///
/// Confirms the side-store entry exists alongside the `ObjectKind` brand
/// so the subsequent `crypto_key_states[&id]` index cannot panic (a
/// brand surviving without its entry — e.g. retained across `Vm::unbind`
/// — surfaces as the same not-a-CryptoKey `TypeError`).
pub(super) fn require_crypto_key_arg(
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
pub(super) fn convert_algorithm_identifier(
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
pub(super) fn marshal_algorithm(
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
                // Members are read in lexicographic order — `hash` before
                // `length`.  `hash` is a **required** `HmacKeyGenParams` /
                // `HmacImportParams` member, so an undefined `hash` is a
                // `TypeError` *before* the `length` getter is read (Web IDL
                // dictionary conversion throws at the first missing required
                // member).
                let hash_val =
                    ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.hash_attr))?;
                if matches!(hash_val, JsValue::Undefined) {
                    return Err(VmError::type_error(format!(
                        "Failed to execute '{method}' on 'SubtleCrypto': \
                         Algorithm: member hash is required"
                    )));
                }
                let hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
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

/// Marshal a JS `sequence<KeyUsage>` into a `Vec<KeyUsage>` (WebIDL
/// §3.2.21): any iterable Object is accepted, a string primitive / other
/// non-Object value is a step-1 conversion `TypeError`, and an unrecognized
/// enum string is a `TypeError`.  Delegates the whole iterator protocol
/// (step-1 non-Object guard, IteratorClose precedence, runaway cap) to the
/// canonical [`webidl_sequence_to_vec`].
pub(super) fn marshal_usages(
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
    webidl_sequence_to_vec(
        ctx,
        value,
        MAX_CRYPTO_SEQUENCE_LEN,
        &msgs,
        |ctx, _idx, el| {
            let sid = coerce::to_string(ctx.vm, el)?;
            let s = ctx.vm.strings.get_utf8(sid);
            KeyUsage::from_ident(&s).ok_or_else(|| {
                VmError::type_error(format!(
                "{prefix}: The provided value '{s}' is not a valid enum value of type KeyUsage."
            ))
            })
        },
    )
}

/// Marshal a JS `KeyFormat` enum string into [`KeyFormat`].
pub(super) fn marshal_format(
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
pub(super) fn marshal_jwk(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<JsonWebKey, VmError> {
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
    let out = webidl_sequence_to_vec(ctx, val, MAX_CRYPTO_SEQUENCE_LEN, &msgs, |ctx, _idx, el| {
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
    webidl_sequence_to_vec(ctx, val, MAX_CRYPTO_SEQUENCE_LEN, &msgs, |ctx, _idx, el| {
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
pub(super) fn build_jwk_object(ctx: &mut NativeContext<'_>, jwk: &JsonWebKey) -> ObjectId {
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
    // Web IDL "convert a dictionary to an ECMAScript value" creates own
    // properties in **lexicographic member order** — for the `oct` subset:
    // alg, ext, k, key_ops, kty, use — so `Object.keys(exportedJwk)`
    // matches the spec / other engines.
    if let Some(alg) = &jwk.alg {
        set_string(ctx, "alg", alg);
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
    if let Some(k) = &jwk.k {
        set_string(ctx, "k", k);
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
    if let Some(kty) = &jwk.kty {
        set_string(ctx, "kty", kty);
    }
    if let Some(use_) = &jwk.use_ {
        set_string(ctx, "use", use_);
    }
    obj
}
