//! Algorithm-identifier marshalling: convert + normalize a JS
//! `AlgorithmIdentifier` into a [`RawAlgorithm`] for an [`Operation`]
//! (WebCrypto §18.4.4), reading the recognized params-dictionary members in
//! Web IDL order via [`read_params`].  The ECDH `public` CryptoKey member is
//! brand-checked + extracted to an [`EcdhPeer`] here (the Layering-mandate
//! marshalling boundary).

use elidex_api_crypto::{self as crypto, AlgorithmParams, EcdhPeer, Operation, RawAlgorithm};

use super::super::super::super::coerce;
use super::super::super::super::value::{
    ElementKind, JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, StringId, VmError,
};
use super::super::super::text_encoding::{extract_buffer_source_member, is_buffer_source};

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
pub(in super::super) fn convert_algorithm_identifier(
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
pub(in super::super) fn marshal_algorithm(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
    op: Operation,
) -> Result<RawAlgorithm, VmError> {
    match value {
        JsValue::String(sid) => Ok(RawAlgorithm::from_name(ctx.vm.strings.get_utf8(sid))),
        JsValue::Object(id) => {
            // §18.4.4 step 2 + steps 4-5: convert to `Algorithm` (reads
            // `name`) and recognize the `(op, name)` pair against the registry,
            // which decides the step-6 `desiredType`.
            let name = read_required_name(ctx, id, method)?;
            let mut raw = RawAlgorithm::from_name(name.clone());
            match crypto::params_shape(op, &name) {
                // Unregistered name: §18.4.4 returns `NotSupportedError` at
                // step 5, BEFORE the step-6 conversion — so no second `name`
                // read and no params getters fire (`crypto::normalize` then
                // produces the `NotSupportedError`).
                None => {}
                // Registered name-only `Algorithm` (digest / sign / verify /
                // AES importKey / HKDF + PBKDF2 importKey + get-key-length):
                // step 6 still converts the object to the `Algorithm`
                // `desiredType`, which re-reads the inherited required `name`
                // member — so a `name` getter that throws / changes on the
                // second read is observed (the step-5 name stays authoritative,
                // step 7).  There are no further params members to read.
                Some(AlgorithmParams::NameOnly) => {
                    read_required_name(ctx, id, method)?;
                }
                // Params-carrying `desiredType`: step 6 re-reads the inherited
                // `name` member first (inherited members precede the derived
                // ones in the Web IDL dictionary conversion), then the derived
                // params members.
                Some(shape) => {
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
#[allow(clippy::too_many_lines)] // flat exhaustive match: one member-read block per AlgorithmParams shape — splitting scatters the spec-ordered per-dictionary marshalling
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
        AlgorithmParams::EcKeyGen => {
            // `EcKeyGenParams` / `EcKeyImportParams`: namedCurve (required
            // `NamedCurve` = DOMString typedef → ToString coercion; the
            // curve-recognition / NotSupportedError is the crate's job).
            let curve_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.named_curve))?;
            if matches!(curve_val, JsValue::Undefined) {
                return Err(required_member_error(method, "namedCurve"));
            }
            let sid = coerce::to_string(ctx.vm, curve_val)?;
            raw.named_curve = Some(ctx.vm.strings.get_utf8(sid));
        }
        AlgorithmParams::EcdsaParams => {
            // `EcdsaParams`: hash (required `HashAlgorithmIdentifier`, the only
            // member) — same step-6 / step-10 split as HMAC / HKDF.
            let hash_val = read_required_hash_value(ctx, id, method)?;
            raw.hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
        }
        AlgorithmParams::EcdhKeyDeriveParams => {
            // `EcdhKeyDeriveParams`: public (required `CryptoKey` peer) — the
            // novel CryptoKey-valued member.  Brand-check + extract the peer's
            // metadata + SEC1 point (the Layering-mandate marshalling boundary);
            // the §24.4.2 InvalidAccessError checks against the base key run
            // later in the crate.
            raw.peer = Some(read_ecdh_public_member(ctx, id, method)?);
        }
        AlgorithmParams::RsaHashedKeyGen => {
            // `RsaHashedKeyGenParams : RsaKeyGenParams` (§20.4 / §20.3): Web IDL
            // dictionary conversion processes **inherited members before derived
            // ones** (Web IDL §3.2.17), so the §18.4.4 step-6 getter order is
            // modulusLength, publicExponent (the RsaKeyGenParams base,
            // lexicographic) THEN hash (the RsaHashedKeyGenParams member) — NOT
            // hash-first.  `modulusLength` is ToNumber/EnforceRange-converted;
            // `publicExponent` is a `BigInteger` = the `Uint8Array` typedef
            // (§20.3), so a non-Uint8Array is a TypeError (not any BufferSource);
            // `hash`'s getter fires last (its nested identifier normalizes at
            // step 10).  Step 10 snapshots the publicExponent bytes (inherited)
            // then normalizes the nested hash (derived).
            let ml_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.modulus_length))?;
            if matches!(ml_val, JsValue::Undefined) {
                return Err(required_member_error(method, "modulusLength"));
            }
            raw.modulus_length = Some(coerce_enforce_range(
                ctx,
                ml_val,
                method,
                "modulusLength",
                "unsigned long",
                f64::from(u32::MAX),
            )?);
            let exp_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.public_exponent))?;
            if matches!(exp_val, JsValue::Undefined) {
                return Err(required_member_error(method, "publicExponent"));
            }
            if !is_uint8_array(ctx, exp_val) {
                return Err(not_uint8_array_error(method, "publicExponent"));
            }
            let hash_val = read_required_hash_value(ctx, id, method)?;
            // step 10: snapshot publicExponent (inherited) then normalize hash.
            raw.public_exponent = Some(snapshot_buffer_source(
                ctx,
                exp_val,
                method,
                "publicExponent",
            )?);
            raw.hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
        }
        AlgorithmParams::RsaHashedImport => {
            // `RsaHashedImportParams` (§20.7): hash (required
            // `HashAlgorithmIdentifier`, the only member) — same step-6 /
            // step-10 split as EcdsaParams.
            let hash_val = read_required_hash_value(ctx, id, method)?;
            raw.hash = Some(Box::new(marshal_hash_identifier(ctx, hash_val, method)?));
        }
        AlgorithmParams::RsaPssParams => {
            // `RsaPssParams` (§21.3): saltLength (required `[EnforceRange]
            // unsigned long`, the only member).
            let salt_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.salt_length))?;
            if matches!(salt_val, JsValue::Undefined) {
                return Err(required_member_error(method, "saltLength"));
            }
            raw.salt_length = Some(coerce_enforce_range(
                ctx,
                salt_val,
                method,
                "saltLength",
                "unsigned long",
                f64::from(u32::MAX),
            )?);
        }
        AlgorithmParams::RsaOaepParams => {
            // `RsaOaepParams` (§22.3): label (OPTIONAL `BufferSource`, the only
            // member).  Validate the type when present (a non-BufferSource is a
            // `TypeError`), then snapshot the bytes — like the AES-GCM
            // `additionalData` optional member (`undefined` → `None`).  Reuses
            // the existing `label` interned identifier.
            let label_val =
                ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.label_attr))?;
            if !matches!(label_val, JsValue::Undefined) && !is_buffer_source(ctx, label_val) {
                return Err(not_buffer_source_error(method, "label"));
            }
            raw.label = snapshot_optional_buffer_source(ctx, label_val, method, "label")?;
        }
    }
    Ok(())
}

/// Read the required `public` CryptoKey member of `EcdhKeyDeriveParams`
/// (WebCrypto §24.3) and extract its spec-relevant metadata + SEC1 public
/// point into an [`EcdhPeer`].  Per the Layering mandate this conveys
/// **bytes + metadata** into the engine-independent crate, never a VM handle
/// (the marshalling boundary): a missing / non-CryptoKey value is a WebIDL
/// `TypeError` here, while the §24.4.2 `InvalidAccessError` precedence (peer
/// `[[type]]` = "public"; peer name = base name; peer curve = base curve) is
/// validated against the base key in `crate::ops::derive_bits`.
fn read_ecdh_public_member(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    method: &str,
) -> Result<EcdhPeer, VmError> {
    let public_val =
        ctx.get_property_value(id, PropertyKey::String(ctx.vm.well_known.public_member))?;
    if matches!(public_val, JsValue::Undefined) {
        return Err(required_member_error(method, "public"));
    }
    let peer_id = require_crypto_key_member(ctx, public_val, method, "public")?;
    let data = &ctx.vm.crypto_key_states[&peer_id];
    Ok(EcdhPeer {
        key_type: data.key_type,
        algorithm: data.algorithm.name(),
        curve: data.algorithm.named_curve(),
        public_point: data.material.ec_public_point().map(<[u8]>::to_vec),
    })
}

/// Brand-check a `CryptoKey`-valued **algorithm dictionary member** (e.g.
/// `EcdhKeyDeriveParams.public`), returning its `ObjectId`.  Mirrors
/// `require_crypto_key_arg` but reports a member-named (not parameter-
/// indexed) WebIDL `TypeError`, and confirms the side-store entry alongside
/// the brand so the subsequent `crypto_key_states[&id]` index cannot panic.
fn require_crypto_key_member(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
    member: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = value {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::CryptoKey)
            && ctx.vm.crypto_key_states.contains_key(&id)
        {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'SubtleCrypto': \
         the '{member}' member is not of type 'CryptoKey'."
    )))
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

/// Whether `value` is specifically a `Uint8Array` — the WebCrypto `BigInteger`
/// typedef (§20.3 `RsaKeyGenParams.publicExponent`), NOT any `BufferSource`
/// (an `ArrayBuffer` / `DataView` / other typed array is a Web IDL `TypeError`).
fn is_uint8_array(ctx: &NativeContext<'_>, value: JsValue) -> bool {
    matches!(
        value,
        JsValue::Object(id)
            if matches!(
                ctx.vm.get_object(id).kind,
                ObjectKind::TypedArray { element_kind: ElementKind::Uint8, .. }
            )
    )
}

/// A member-named "not a `Uint8Array`" `TypeError` (the `BigInteger` typedef
/// type-check at §18.4.4 step 6).
fn not_uint8_array_error(method: &str, member: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'SubtleCrypto': \
         the '{member}' member is not of type 'Uint8Array'"
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
        // [`read_required_hash_value`] already took the §18.4.4 step-6 DOMString
        // arm (`ToString`) for every non-object primitive, so this leaf only
        // ever receives a `String` or an `Object` — the invariant the step-6 /
        // step-10 split established.
        other => {
            unreachable!("hash identifier was converted to String/Object at step 6, got {other:?}")
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
