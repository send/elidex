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
use elidex_api_crypto::{
    self as crypto, AlgorithmParams, JsonWebKey, KeyFormat, Operation, RawAlgorithm,
};

use super::super::super::coerce;
use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::super::webidl_sequence::{webidl_sequence_to_vec, SeqMessages};
use super::super::text_encoding::{extract_buffer_source_member, is_buffer_source};

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
/// The derived params-dictionary members (HMAC `hash` / `length`; AES
/// `iv` / `counter` / `additionalData` / `tagLength` / key-gen `length`)
/// are read by [`read_params`] **only** once `crypto::params_shape(op,
/// name)` recognizes the `(op, name)` pair, and only those members that
/// pair's dictionary carries.  This mirrors §18.4.4: step 5 recognizes
/// `algName` (returning `NotSupportedError` for an unregistered pair)
/// *before* step 6 converts `alg` to the params dictionary, which is what
/// fires those getters.  An unregistered name (e.g.
/// `encrypt({name:'AES-Magic', get iv(){throw}}, …)`) must therefore reject
/// with `NotSupportedError` and never fire the getter.  For `digest` /
/// `sign` / `verify` / AES `importKey` the identifier is a name-only
/// `Algorithm`, so no derived member is consulted.
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
    match value {
        JsValue::String(sid) => Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid))),
        JsValue::Object(id) => {
            let name = read_required_name(ctx, id, method)?;
            let mut raw = RawAlgorithm::from_name(name.clone());
            // §18.4.4 step 5 recognition gate: the registry decides which
            // params-dictionary members this `(op, name)` carries.  An
            // unregistered name (`None`) or a name-only `Algorithm`
            // (`NameOnly` — digest / sign / verify / AES importKey) reads no
            // further getters; `crypto::normalize` then accepts the
            // name-only form or rejects it as `NotSupportedError`, never
            // touching a user-defined params getter for an unregistered name.
            match crypto::params_shape(op, &name) {
                None | Some(AlgorithmParams::NameOnly) => {}
                Some(shape) => {
                    // §18.4.4 step 6: converting `alg` to its params
                    // dictionary (each of which inherits `Algorithm`)
                    // re-reads the required inherited `name` member first —
                    // its getter fires again before the derived members, so
                    // a throw / now-missing `name` on the second access
                    // propagates (the step-5 name stays authoritative,
                    // step 7).
                    read_required_name(ctx, id, method)?;
                    read_params(ctx, id, method, shape, &mut raw)?;
                }
            }
            Ok(raw)
        }
        // `value` is post-[`convert_algorithm_identifier`], so it is always
        // an Object or a `String` — any other variant is a caller bug.
        other => unreachable!("algorithm must be converted first, got {other:?}"),
    }
}

/// Read the derived params-dictionary members for a recognized `(op, name)`
/// (WebCrypto §18.4.4 step 6), in Web IDL **lexicographic member order** so
/// getter side effects fire in the spec order.  A missing required member is
/// a `TypeError`; the value validity (iv/counter byte length, tagLength
/// value, key length) is validated later in the engine-independent crate
/// (`crypto::normalize` + `crypto::ops`), at the op step where the spec
/// throws.
fn read_params(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    method: &str,
    shape: AlgorithmParams,
    raw: &mut RawAlgorithm,
) -> Result<(), VmError> {
    match shape {
        // Handled by the caller (no derived members).
        AlgorithmParams::NameOnly => {}
        AlgorithmParams::HmacKeyParams => {
            // `HmacKeyGenParams` / `HmacImportParams`: hash (required), then
            // length (optional `unsigned long`) — lexicographic order.
            // step 6 (top-level getters, lexicographic hash < length):
            let hash_val = read_required_hash_value(ctx, id, method)?;
            raw.length =
                read_optional_length(ctx, id, method, "unsigned long", f64::from(u32::MAX))?;
            // step 10: normalize the nested HashAlgorithmIdentifier (`hash.name`).
            raw.hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
        }
        AlgorithmParams::AesKeyGen => {
            // `AesKeyGenParams`: length (required `[EnforceRange] unsigned
            // short`).
            let length_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.length))?;
            if matches!(length_val, JsValue::Undefined) {
                return Err(required_member_error(method, "length"));
            }
            raw.length = Some(coerce_enforce_range(
                ctx,
                length_val,
                method,
                "length",
                "unsigned short",
                65535.0,
            )?);
        }
        AlgorithmParams::AesGcmParams => {
            // `AesGcmParams`: additionalData (optional), iv (required),
            // tagLength (optional `[EnforceRange] octet`).  WebCrypto §18.4.4
            // step 6 converts the dictionary **member-by-member** in
            // lexicographic order — each member's getter is read and its
            // required-ness / type validated *before the next member is
            // read* — while step 10 snapshots the `BufferSource` bytes
            // afterwards (so a later getter that mutated an earlier member's
            // buffer is reflected, but a missing/invalid earlier member still
            // errors before a later getter runs).
            let (aad_sid, iv_sid, tag_sid) = (
                ctx.vm.well_known.additional_data,
                ctx.vm.well_known.iv,
                ctx.vm.well_known.tag_length,
            );
            // step 6, member-by-member (validate type, defer the byte copy):
            let aad_val = ctx.get_property_value(id, PropertyKey::String(aad_sid))?;
            if !matches!(aad_val, JsValue::Undefined) && !is_buffer_source(ctx, aad_val) {
                return Err(not_buffer_source_error(method, "additionalData"));
            }
            let iv_val = ctx.get_property_value(id, PropertyKey::String(iv_sid))?;
            require_buffer_source_member(ctx, iv_val, method, "iv")?;
            let tag_length = read_optional_octet(ctx, id, method, tag_sid, "tagLength")?;
            // step 10: snapshot the BufferSource bytes (detached check + copy).
            raw.additional_data =
                snapshot_optional_buffer_source(ctx, aad_val, method, "additionalData")?;
            raw.iv = Some(snapshot_buffer_source(ctx, iv_val, method, "iv")?);
            raw.tag_length = tag_length;
        }
        AlgorithmParams::AesCbcParams => {
            // `AesCbcParams`: iv (required `BufferSource`).
            let iv_sid = ctx.vm.well_known.iv;
            let iv_val = ctx.get_property_value(id, PropertyKey::String(iv_sid))?;
            require_buffer_source_member(ctx, iv_val, method, "iv")?;
            raw.iv = Some(snapshot_buffer_source(ctx, iv_val, method, "iv")?);
        }
        AlgorithmParams::AesCtrParams => {
            // `AesCtrParams`: counter (required), length (required
            // `[EnforceRange] octet`).  §18.4.4 step 6 validates each member
            // (counter then length) before step 10's counter-byte snapshot.
            let (counter_sid, length_sid) = (ctx.vm.well_known.counter, ctx.vm.well_known.length);
            let counter_val = ctx.get_property_value(id, PropertyKey::String(counter_sid))?;
            require_buffer_source_member(ctx, counter_val, method, "counter")?;
            let length_val = ctx.get_property_value(id, PropertyKey::String(length_sid))?;
            if matches!(length_val, JsValue::Undefined) {
                return Err(required_member_error(method, "length"));
            }
            let length = coerce_enforce_range(ctx, length_val, method, "length", "octet", 255.0)?;
            // §18.4.4 step 10: snapshot the counter bytes (post-getters).
            raw.counter = Some(snapshot_buffer_source(ctx, counter_val, method, "counter")?);
            raw.length = Some(length);
        }
        AlgorithmParams::HkdfParams => {
            // `HkdfParams`: hash (required), info (required `BufferSource`),
            // salt (required `BufferSource`) — Web IDL lexicographic member
            // order is hash < info < salt.  §18.4.4 step 6 reads every
            // top-level member value in that order (the `hash` getter fires
            // here, but its nested `HashAlgorithmIdentifier` is NOT yet
            // normalized); step 10 then normalizes `hash` (reading `hash.name`,
            // again first lexicographically) and copies the `info` / `salt`
            // BufferSource bytes — so a throwing / mutating getter rejects in
            // the spec order.
            let hash_val = read_required_hash_value(ctx, id, method)?;
            let info_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.info))?;
            require_buffer_source_member(ctx, info_val, method, "info")?;
            let salt_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.salt))?;
            require_buffer_source_member(ctx, salt_val, method, "salt")?;
            // step 10 (lexicographic hash < info < salt): normalize hash, then
            // snapshot the BufferSource bytes.
            raw.hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
            raw.info = Some(snapshot_buffer_source(ctx, info_val, method, "info")?);
            raw.salt = Some(snapshot_buffer_source(ctx, salt_val, method, "salt")?);
        }
        AlgorithmParams::Pbkdf2Params => {
            // `Pbkdf2Params`: hash (required), iterations (required
            // `[EnforceRange] unsigned long`), salt (required `BufferSource`)
            // — lexicographic order hash < iterations < salt.  §18.4.4 step 6
            // reads every top-level member value in that order (`hash` getter
            // + `iterations` ToNumber/EnforceRange + `salt` getter); step 10
            // then normalizes `hash` (`hash.name`) and copies the `salt` bytes.
            let hash_val = read_required_hash_value(ctx, id, method)?;
            let iter_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.iterations))?;
            if matches!(iter_val, JsValue::Undefined) {
                return Err(required_member_error(method, "iterations"));
            }
            raw.iterations = Some(coerce_enforce_range(
                ctx,
                iter_val,
                method,
                "iterations",
                "unsigned long",
                f64::from(u32::MAX),
            )?);
            let salt_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.salt))?;
            require_buffer_source_member(ctx, salt_val, method, "salt")?;
            // step 10 (lexicographic hash < iterations < salt): normalize hash,
            // then snapshot the salt bytes.
            raw.hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
            raw.salt = Some(snapshot_buffer_source(ctx, salt_val, method, "salt")?);
        }
    }
    Ok(())
}

/// Validate an **already-read** required `BufferSource` member's type
/// (WebCrypto §18.4.4 step 6, member-by-member — run before the next member
/// is read, and before the step-10 byte snapshot): `undefined` → required
/// `TypeError`; a present non-BufferSource → type `TypeError`.
fn require_buffer_source_member(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
    member: &str,
) -> Result<(), VmError> {
    if matches!(value, JsValue::Undefined) {
        return Err(required_member_error(method, member));
    }
    if !is_buffer_source(ctx, value) {
        return Err(not_buffer_source_error(method, member));
    }
    Ok(())
}

/// A member-named "not a `BufferSource`" `TypeError` (Web IDL type-check at
/// §18.4.4 step 6).
fn not_buffer_source_error(method: &str, member: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'SubtleCrypto': \
         the '{member}' member is not of type 'BufferSource'"
    ))
}

/// Snapshot-copy an **already-read** required `BufferSource` member value
/// (AES `iv` / `counter`) to bytes — WebCrypto §18.4.4 step 10, run after
/// every member getter has fired (step 6).  An absent / non-BufferSource
/// value is a member-named `TypeError`.
fn snapshot_buffer_source(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
    member: &str,
) -> Result<Vec<u8>, VmError> {
    let prefix = format!("Failed to execute '{method}' on 'SubtleCrypto'");
    extract_buffer_source_member(ctx, value, &prefix, member)
}

/// Snapshot-copy an **already-read** optional `BufferSource` member value
/// (AES `additionalData`): `undefined` → absent (`None`).
fn snapshot_optional_buffer_source(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
    member: &str,
) -> Result<Option<Vec<u8>>, VmError> {
    if matches!(value, JsValue::Undefined) {
        return Ok(None);
    }
    Ok(Some(snapshot_buffer_source(ctx, value, method, member)?))
}

/// Read an optional `[EnforceRange] octet` member (AES-GCM `tagLength`):
/// `undefined` → absent.
fn read_optional_octet(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    method: &str,
    member_sid: StringId,
    member: &str,
) -> Result<Option<u32>, VmError> {
    let val = ctx.get_property_value(id, PropertyKey::String(member_sid))?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    Ok(Some(coerce_enforce_range(
        ctx, val, method, member, "octet", 255.0,
    )?))
}

/// Read an optional `[EnforceRange] unsigned long` `length` member (HMAC):
/// `undefined` → absent.
fn read_optional_length(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    method: &str,
    idl_type: &str,
    max_inclusive: f64,
) -> Result<Option<u32>, VmError> {
    let val = ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.length))?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    Ok(Some(coerce_enforce_range(
        ctx,
        val,
        method,
        "length",
        idl_type,
        max_inclusive,
    )?))
}

fn required_member_error(method: &str, member: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'SubtleCrypto': \
         Algorithm: member {member} is required"
    ))
}

/// Read + step-6-convert the required `hash` member of a params dictionary that
/// carries one (`HmacKeyGenParams` / `HmacImportParams`, `HkdfParams`,
/// `Pbkdf2Params`) — the WebCrypto §18.4.4 **step 6** dictionary conversion
/// (`hash` sorts first: before `info` / `iterations` / `length` / `salt`).  An
/// absent / `undefined` value is the required-member `TypeError`.
///
/// Per Web IDL the `hash` member's `HashAlgorithmIdentifier` =
/// `(object or DOMString)` union is converted *at step 6* (in lexicographic
/// position, before the sibling members): an **object** is kept as-is, while
/// any **non-object primitive** takes the DOMString arm and is `ToString`-ed
/// **now** — so `hash: Symbol()` throws here, before the `info` / `salt` /
/// `iterations` / `length` getters run.  Only the object case's `name` lookup
/// is deferred: §18.4.4 **step 10** normalizes the (now object-or-DOMString)
/// member, and the caller therefore reads every sibling top-level member
/// first, then calls [`marshal_hash_identifier`] on the value returned here.
fn read_required_hash_value(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    method: &str,
) -> Result<JsValue, VmError> {
    let hash_val = ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.hash_attr))?;
    match hash_val {
        JsValue::Undefined => Err(required_member_error(method, "hash")),
        // Object arm: keep it; the `name` lookup is the step-10 deferral.
        JsValue::Object(_) => Ok(hash_val),
        // DOMString arm: ToString now (step 6) — a `Symbol` / `BigInt` throws
        // here, before the sibling members are read.
        other => Ok(JsValue::String(coerce::to_string(ctx.vm, other)?)),
    }
}

/// Normalize a step-6-converted `HashAlgorithmIdentifier` (§18.4.4 step 10) —
/// a **leaf** digest identifier (a `HashAlgorithmIdentifier` never has its own
/// nested `hash`/`length`, so it cannot recurse).  The input is the value from
/// [`read_required_hash_value`], so it is already either a `DOMString` (the
/// step-6 union result for any primitive) or an object whose `name` is read
/// here (the deferred step-10 lookup).
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

/// WebIDL `[EnforceRange]` integer conversion for an algorithm `member`
/// (Web IDL §3.3.6 `[EnforceRange]` / ConvertToInt step 6): ToNumber,
/// reject NaN / ±∞ with a `TypeError`, then take `IntegerPart` (**truncate
/// toward zero**) and range-check against `[0, max_inclusive]`.  A finite
/// fractional value such as `31.9` is therefore accepted as `31` — NOT
/// rejected (and NOT the wrapping `ToUint32`).  `idl_type` names the IDL
/// integer type (`unsigned long` / `unsigned short` / `octet`) for the
/// error message and `max_inclusive` is that type's maximum.
fn coerce_enforce_range(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
    member: &str,
    idl_type: &str,
    max_inclusive: f64,
) -> Result<u32, VmError> {
    let n = coerce::to_number(ctx.vm, value)?;
    // Web IDL: NaN / ±∞ throw before truncation; otherwise IntegerPart
    // truncates toward zero, then the result is bounds-checked.
    let truncated = n.trunc();
    if n.is_finite() && truncated >= 0.0 && truncated <= max_inclusive {
        // Truncated integer within range (max_inclusive ≤ u32::MAX); the
        // cast is lossless.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(truncated as u32)
    } else {
        Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': \
             Algorithm: {member}: Value is outside the '{idl_type}' value range"
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
