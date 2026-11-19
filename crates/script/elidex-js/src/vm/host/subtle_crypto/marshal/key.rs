//! Key-related marshalling: the `CryptoKey` operation-argument brand check
//! ([`require_crypto_key_arg`]), the `sequence<KeyUsage>` / `KeyFormat` enum
//! conversions ([`marshal_usages`] / [`marshal_format`]), and the
//! `CryptoKeyPair` result builder ([`build_crypto_key_pair`]).

use elidex_api_crypto::key::KeyUsage;
use elidex_api_crypto::{KeyFormat, MAX_CRYPTO_SEQUENCE_LEN};

use super::super::super::super::coerce;
use super::super::super::super::shape;
use super::super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::super::webidl_sequence::{webidl_sequence_to_vec, SeqMessages};

/// Brand-check a `CryptoKey` operation argument (NOT `this`).  A wrong
/// type is a WebIDL conversion `TypeError`, settled on the Promise.
///
/// Confirms the side-store entry exists alongside the `ObjectKind` brand
/// so the subsequent `crypto_key_states[&id]` index cannot panic (a
/// brand surviving without its entry — e.g. retained across `Vm::unbind`
/// — surfaces as the same not-a-CryptoKey `TypeError`).
pub(in super::super) fn require_crypto_key_arg(
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

/// Marshal a JS `sequence<KeyUsage>` into a `Vec<KeyUsage>` (WebIDL
/// §3.2.21): any iterable Object is accepted, a string primitive / other
/// non-Object value is a step-1 conversion `TypeError`, and an unrecognized
/// enum string is a `TypeError`.  Delegates the whole iterator protocol
/// (step-1 non-Object guard, IteratorClose precedence, runaway cap) to the
/// canonical [`webidl_sequence_to_vec`].
pub(in super::super) fn marshal_usages(
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
pub(in super::super) fn marshal_format(
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

/// Build the `CryptoKeyPair` dictionary (WebCrypto §17) returned by an EC
/// `generateKey` (§14.3.6 `(CryptoKey or CryptoKeyPair)`): a plain object
/// `{ privateKey, publicKey }` with **no** `ObjectKind` brand — an ordinary
/// object like the exported JWK object, not a `CryptoKey`.  Web IDL converts
/// a dictionary to an ECMAScript value member-by-member in **lexicographic**
/// order (Web IDL §3.2.17), so `privateKey` (`p-r`) precedes `publicKey`
/// (`p-u`) — matching `Object.keys(keyPair)` in other engines.
///
/// GC is disabled for the whole `NativeFunction` call (see
/// `build_jwk_object`), so the two `alloc_crypto_key` calls in the caller
/// plus this assembly have no mid-collection window; the two wrappers stay
/// reachable through these own properties (and via `crypto_key_states` /
/// `crypto_key_js_cache`, traced + unbind-cleared per their `ObjectId`).
pub(in super::super) fn build_crypto_key_pair(
    ctx: &mut NativeContext<'_>,
    public_id: ObjectId,
    private_id: ObjectId,
) -> ObjectId {
    let object_proto = ctx.vm.object_prototype;
    let pair = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });
    let private_key = PropertyKey::String(ctx.intern("privateKey"));
    ctx.vm.define_shaped_property(
        pair,
        private_key,
        PropertyValue::Data(JsValue::Object(private_id)),
        shape::PropertyAttrs::DATA,
    );
    let public_key = PropertyKey::String(ctx.intern("publicKey"));
    ctx.vm.define_shaped_property(
        pair,
        public_key,
        PropertyValue::Data(JsValue::Object(public_id)),
        shape::PropertyAttrs::DATA,
    );
    pair
}
