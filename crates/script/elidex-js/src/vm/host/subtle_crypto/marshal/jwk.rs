//! `JsonWebKey` marshalling: the live (JS-object) dictionary conversion
//! [`marshal_jwk`] (the §15 member read, the spec-forced mirror of
//! `elidex_api_crypto::jwk::from_json_bytes`) and the exported-`oct`/EC/RSA
//! JWK object builder [`build_jwk_object`].

use elidex_api_crypto::{JsonWebKey, RsaOtherPrimesInfo, MAX_CRYPTO_SEQUENCE_LEN};

use super::super::super::super::coerce;
use super::super::super::super::shape;
use super::super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::super::webidl_sequence::{webidl_sequence_to_vec, SeqMessages};

/// Marshal a JS value into a [`JsonWebKey`] via Web IDL dictionary
/// conversion.
///
/// - `null` / `undefined` convert to an **empty** dictionary (all members
///   absent); the HMAC import path then rejects it with `DataError`
///   (missing `kty` / `k`), not a `TypeError`.
/// - A non-object, non-nullish value cannot be a dictionary → `TypeError`.
/// - For an object, every declared `JsonWebKey` member is read **in
///   lexicographic identifier order**, firing each getter and propagating
///   its throws — even members the importing algorithm ignores.  The `oct`
///   (`kty` / `use` / `key_ops` / `alg` / `ext` / `k`) and EC (`crv` / `x` /
///   `y` / `d`) members are retained; the RSA fields are still read (for the
///   getter side-effects) but discarded until PR-5.
///
/// This is the **live (JS-object) half** of a spec-forced mirror: `wrapKey` /
/// `unwrapKey` re-implement the identical `JsonWebKey` dictionary conversion
/// over JSON *bytes* in `elidex_api_crypto::jwk::from_json_bytes` (realm-
/// isolated, no JS object — WebCrypto §9 "a new global object").  The two must
/// stay in lockstep: a member read here must be retained identically in
/// `from_json_bytes`, or wrap↔import coercion / error precedence diverge.  The
/// EC members landed in PR-4 (RSA in PR-5); the differential equivalence test
/// (`#11-crypto-subtle-full`) mechanically pins the two halves in lockstep.
// The single-char locals (d / e / k / n / p / q / x / y) are the canonical JWK
// member identifiers (RFC 7517 / 7518) — renaming them would obscure, not
// clarify, the spec mapping.
#[allow(clippy::many_single_char_names)]
pub(in super::super) fn marshal_jwk(
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
    // qi, use, x, y.  Read every member in this order (firing getters /
    // propagating throws); retain the oct + EC + RSA subsets.
    let alg = read_jwk_string(ctx, id, "alg")?;
    let crv = read_jwk_string(ctx, id, "crv")?;
    let d = read_jwk_string(ctx, id, "d")?;
    let dp = read_jwk_string(ctx, id, "dp")?;
    let dq = read_jwk_string(ctx, id, "dq")?;
    let e = read_jwk_string(ctx, id, "e")?;
    let ext = read_jwk_bool(ctx, id, "ext")?;
    let k = read_jwk_string(ctx, id, "k")?;
    let key_ops = read_jwk_key_ops(ctx, id)?;
    let kty = read_jwk_string(ctx, id, "kty")?;
    let n = read_jwk_string(ctx, id, "n")?;
    let oth = read_jwk_oth(ctx, id)?;
    let p = read_jwk_string(ctx, id, "p")?;
    let q = read_jwk_string(ctx, id, "q")?;
    let qi = read_jwk_string(ctx, id, "qi")?;
    let use_ = read_jwk_string(ctx, id, "use")?;
    let x = read_jwk_string(ctx, id, "x")?;
    let y = read_jwk_string(ctx, id, "y")?;
    Ok(JsonWebKey {
        kty,
        k,
        alg,
        use_,
        key_ops,
        ext,
        crv,
        x,
        y,
        d,
        n,
        e,
        p,
        q,
        dp,
        dq,
        qi,
        oth,
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
/// converting each entry per Web IDL.  `undefined` → absent (`None`);
/// otherwise the value is converted to a sequence (a non-iterable such as
/// `oth: 123` → `TypeError`), and each entry is converted to an
/// `RsaOtherPrimesInfo` dictionary: `undefined` / `null` → an empty dict,
/// an object → its (optional) `d` / `r` / `t` `DOMString` members read in
/// lexicographic order (firing each getter), any other value → `TypeError`
/// (e.g. `oth: [123]`).  The converted entries are retained (PR-5a) so the
/// live↔bytes mirror holds; multi-prime import itself is a DataError.
fn read_jwk_oth(
    ctx: &mut NativeContext<'_>,
    obj: ObjectId,
) -> Result<Option<Vec<RsaOtherPrimesInfo>>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern("oth"));
    let val = ctx.get_property_value(obj, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(None);
    }
    let msgs = SeqMessages {
        not_iterable: "Failed to execute 'importKey' on 'SubtleCrypto': \
                       JWK 'oth' member is not a sequence.",
        iter_not_object: "Failed to execute 'importKey' on 'SubtleCrypto': \
                          JWK 'oth' @@iterator must return an object.",
        cap_exceeded: "Failed to execute 'importKey' on 'SubtleCrypto': \
                       JWK 'oth' exceeds the maximum length.",
    };
    let out = webidl_sequence_to_vec(ctx, val, MAX_CRYPTO_SEQUENCE_LEN, &msgs, |ctx, _idx, el| {
        let entry = match el {
            // `null` / `undefined` → an empty RsaOtherPrimesInfo dict.
            JsValue::Undefined | JsValue::Null => return Ok(RsaOtherPrimesInfo::default()),
            JsValue::Object(id) => id,
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'importKey' on 'SubtleCrypto': \
                     JWK 'oth' entry is not an RsaOtherPrimesInfo dictionary.",
                ))
            }
        };
        // RsaOtherPrimesInfo members in lexicographic order (d, r, t), all
        // optional `DOMString`s — read (firing getters) + retain.
        let d = read_jwk_string(ctx, entry, "d")?;
        let r = read_jwk_string(ctx, entry, "r")?;
        let t = read_jwk_string(ctx, entry, "t")?;
        Ok(RsaOtherPrimesInfo { r, d, t })
    })?;
    Ok(Some(out))
}

/// Build a fresh JS object for an exported `oct` JWK.
///
/// The intermediate `obj` / `key_ops` array are not separately rooted
/// across the inner `alloc_object` calls: GC is disabled for the whole
/// duration of a `NativeFunction` call (`interpreter.rs` /
/// `dispatch.rs` set `gc_enabled = false`; see `natives_array_hof.rs`),
/// so `alloc_object` here never triggers a collection. Add temp-roots
/// only if GC is ever permitted to run during native→JS callbacks.
pub(in super::super) fn build_jwk_object(
    ctx: &mut NativeContext<'_>,
    jwk: &JsonWebKey,
) -> ObjectId {
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
    // properties in **lexicographic member order** — across the `oct` + EC +
    // RSA subsets: alg, crv, d, dp, dq, e, ext, k, key_ops, kty, n, p, q, qi,
    // use, x, y — so `Object.keys(exportedJwk)` matches the spec / other
    // engines.  (`oth` is never emitted: multi-prime export does not occur.)
    if let Some(alg) = &jwk.alg {
        set_string(ctx, "alg", alg);
    }
    if let Some(crv) = &jwk.crv {
        set_string(ctx, "crv", crv);
    }
    if let Some(d) = &jwk.d {
        set_string(ctx, "d", d);
    }
    if let Some(dp) = &jwk.dp {
        set_string(ctx, "dp", dp);
    }
    if let Some(dq) = &jwk.dq {
        set_string(ctx, "dq", dq);
    }
    if let Some(e) = &jwk.e {
        set_string(ctx, "e", e);
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
    if let Some(n) = &jwk.n {
        set_string(ctx, "n", n);
    }
    if let Some(p) = &jwk.p {
        set_string(ctx, "p", p);
    }
    if let Some(q) = &jwk.q {
        set_string(ctx, "q", q);
    }
    if let Some(qi) = &jwk.qi {
        set_string(ctx, "qi", qi);
    }
    if let Some(use_) = &jwk.use_ {
        set_string(ctx, "use", use_);
    }
    if let Some(x) = &jwk.x {
        set_string(ctx, "x", x);
    }
    if let Some(y) = &jwk.y {
        set_string(ctx, "y", y);
    }
    obj
}
