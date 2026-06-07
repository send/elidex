// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.

//! `CryptoKey` interface (WebCrypto §13) — VM thin binding to the
//! per-key state held in `VmInner::crypto_key_states`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check, and
//! marshalling the engine-independent
//! [`elidex_api_crypto::CryptoKeyData`] into the four readonly
//! accessors.  All algorithm + key validation lives in
//! `elidex-api-crypto`.
//!
//! ## State + GC
//!
//! The per-key `CryptoKeyData` (algorithm / extractable / usages / secret
//! material) lives in `VmInner::crypto_key_states` keyed by the wrapper's
//! `ObjectId`.  The `algorithm` / `usages` accessors return the **cached**
//! ECMAScript object (WebCrypto §13.4 — `[[algorithm_cached]]` /
//! `[[usages_cached]]` internal slots, §13.3), built once on first read
//! and stored in [`VmInner::crypto_key_js_cache`]; `key.algorithm ===
//! key.algorithm` and `key.usages === key.usages` are therefore `true`.
//! Because those cached objects outlive the native call that built them
//! (GC runs between calls), [`ObjectKind::CryptoKey`] is **not**
//! payload-free for trace: its `vm/gc/trace.rs` arm marks the two cached
//! `ObjectId`s, and the sweep tail / `Vm::unbind` prune the cache
//! alongside `crypto_key_states`.

#![cfg(feature = "engine")]

use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

/// Per-`CryptoKey` cache of the ECMAScript objects returned by the
/// `algorithm` / `usages` accessors — the `[[algorithm_cached]]` /
/// `[[usages_cached]]` internal slots (WebCrypto §13.3 / §13.4).
///
/// §13.4 requires both getters to return the **cached** object (not a
/// fresh build per read), so each is built lazily on first access and the
/// resulting `ObjectId` is stored here; later reads return the same id.
/// Web IDL declares neither member `[SameObject]`, but the prose mandates
/// the caching independently of that attribute (so `key.algorithm ===
/// key.algorithm` MUST hold).
///
/// Keyed in `VmInner::crypto_key_js_cache` by the `CryptoKey` wrapper's
/// own `ObjectId` (the same key as `crypto_key_states`).  GC contract:
/// the `ObjectKind::CryptoKey` trace arm marks the two cached `ObjectId`s
/// (they persist across native calls, when GC may run), and the sweep
/// tail + `Vm::unbind` prune the entry with the key.
#[derive(Default)]
pub(crate) struct CryptoKeyJsCache {
    pub(crate) algorithm: Option<ObjectId>,
    pub(crate) usages: Option<ObjectId>,
}

impl VmInner {
    /// Allocate `CryptoKey.prototype` chained to `Object.prototype`,
    /// install the `type` / `extractable` / `algorithm` / `usages`
    /// readonly accessors, and expose the `CryptoKey` illegal-constructor
    /// stub on `globalThis`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — call-order invariant
    /// from `register_globals()` violated.
    pub(in crate::vm) fn register_crypto_key_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_crypto_key_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // Four readonly accessors (WebCrypto §13 `CryptoKey`):
        // `readonly attribute KeyType type` / `boolean extractable` /
        // `object algorithm` / `object usages`.  `algorithm` / `usages`
        // return the cached object (§13.4) via `crypto_key_js_cache`.
        let accessors: [(_, NativeFn); 4] = [
            (
                self.well_known.event_type,
                native_crypto_key_get_type as NativeFn,
            ),
            (
                self.well_known.extractable,
                native_crypto_key_get_extractable,
            ),
            (self.well_known.algorithm, native_crypto_key_get_algorithm),
            (self.well_known.usages, native_crypto_key_get_usages),
        ];
        for (name_sid, getter) in accessors {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        self.crypto_key_prototype = Some(proto_id);

        // `CryptoKey` declares no constructor operation — registered as
        // an illegal constructor so call/construct both throw at the
        // gate (keys are produced only by `SubtleCrypto` operations).
        let ctor = self.create_illegal_constructor_function(
            "CryptoKey",
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
        let name_sid = self.well_known.crypto_key_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Allocate a `CryptoKey` wrapper backed by `data`, inserting the
    /// per-key state into `crypto_key_states`.  Used by
    /// `SubtleCrypto.{generateKey,importKey}`.
    ///
    /// # Panics
    ///
    /// Panics if `crypto_key_prototype` is `None` (registration order
    /// invariant).
    pub(in crate::vm) fn alloc_crypto_key(&mut self, data: CryptoKeyData) -> ObjectId {
        let proto = self
            .crypto_key_prototype
            .expect("alloc_crypto_key before register_crypto_key_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::CryptoKey,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.crypto_key_states.insert(id, data);
        id
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Confirm `this` is a `CryptoKey` **with a live side-store entry**,
/// returning its `ObjectId`.  Used by the accessors and by
/// `SubtleCrypto.{exportKey,sign,verify}` so the subsequent
/// `crypto_key_states[&id]` index can never panic.
///
/// The side-store presence is checked alongside the `ObjectKind` brand
/// (mirroring the other side-table-backed brands): the two are an
/// invariant pair (`alloc_crypto_key` always inserts), but a brand
/// surviving without its entry — e.g. a reference retained across
/// `Vm::unbind`, which clears the side-store — must surface as an
/// illegal invocation, not a panic / stale-material read.
pub(super) fn require_crypto_key_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    accessor: &'static str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::CryptoKey)
            && ctx.vm.crypto_key_states.contains_key(&id)
        {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "Failed to read the '{accessor}' property from 'CryptoKey': Illegal invocation"
    )))
}

// ---------------------------------------------------------------------------
// Accessors (WebCrypto §13)
// ---------------------------------------------------------------------------

fn native_crypto_key_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_crypto_key_this(ctx, this, "type")?;
    let type_str = ctx.vm.crypto_key_states[&id].key_type.as_str();
    let sid = ctx.intern(type_str);
    Ok(JsValue::String(sid))
}

fn native_crypto_key_get_extractable(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_crypto_key_this(ctx, this, "extractable")?;
    Ok(JsValue::Boolean(ctx.vm.crypto_key_states[&id].extractable))
}

fn native_crypto_key_get_algorithm(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_crypto_key_this(ctx, this, "algorithm")?;
    // §13.4: return the cached `[[algorithm_cached]]` object; build it
    // once on first read.
    if let Some(cached) = ctx
        .vm
        .crypto_key_js_cache
        .get(&id)
        .and_then(|c| c.algorithm)
    {
        return Ok(JsValue::Object(cached));
    }
    let algorithm = ctx.vm.crypto_key_states[&id].algorithm;
    let obj = build_algorithm_object(ctx, algorithm);
    ctx.vm.crypto_key_js_cache.entry(id).or_default().algorithm = Some(obj);
    Ok(JsValue::Object(obj))
}

fn native_crypto_key_get_usages(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_crypto_key_this(ctx, this, "usages")?;
    // §13.4: return the cached `[[usages_cached]]` array; build it once on
    // first read.
    if let Some(cached) = ctx.vm.crypto_key_js_cache.get(&id).and_then(|c| c.usages) {
        return Ok(JsValue::Object(cached));
    }
    let usages = ctx.vm.crypto_key_states[&id].usages.clone();
    let elements = usages
        .iter()
        .map(|u| JsValue::String(ctx.intern(u.as_str())))
        .collect::<Vec<_>>();
    let array_proto = ctx.vm.array_prototype;
    let arr = ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: array_proto,
        extensible: true,
    });
    ctx.vm.crypto_key_js_cache.entry(id).or_default().usages = Some(arr);
    Ok(JsValue::Object(arr))
}

/// Build the JS algorithm dictionary for the `algorithm` accessor —
/// called once per key on the first read (the result is then cached in
/// `crypto_key_js_cache`, §13.4).  For HMAC:
/// `{ name: "HMAC", hash: { name: "SHA-256" }, length: N }`
/// (WebCrypto §31 `HmacKeyAlgorithm`); for AES:
/// `{ name: "AES-GCM", length: N }` (WebCrypto `AesKeyAlgorithm`, no
/// `hash` member); for HKDF / PBKDF2: `{ name: "HKDF" }` / `{ name:
/// "PBKDF2" }` (WebCrypto §33.4.2 / §34.4.2 — a name-only `KeyAlgorithm`,
/// no `hash` / `length`; the derive params live on the `deriveBits` call).
///
/// The intermediate `hash_obj` / `obj` are not separately rooted across
/// the inner `alloc_object` calls: GC is disabled for the whole duration
/// of a `NativeFunction` call (see `natives_array_hof.rs`), so
/// `alloc_object` here never triggers a collection.
fn build_algorithm_object(ctx: &mut NativeContext<'_>, algorithm: KeyAlgorithm) -> ObjectId {
    let object_proto = ctx.vm.object_prototype;
    match algorithm {
        KeyAlgorithm::Hmac { hash, length } => {
            // Nested hash dictionary `{ name: "SHA-256" }`.
            let hash_obj = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: object_proto,
                extensible: true,
            });
            let name_key = PropertyKey::String(ctx.vm.well_known.name);
            let hash_name = ctx.intern(hash.canonical_name());
            ctx.vm.define_shaped_property(
                hash_obj,
                name_key,
                PropertyValue::Data(JsValue::String(hash_name)),
                shape::PropertyAttrs::DATA,
            );

            let obj = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: object_proto,
                extensible: true,
            });
            let hmac_name = ctx.intern("HMAC");
            ctx.vm.define_shaped_property(
                obj,
                PropertyKey::String(ctx.vm.well_known.name),
                PropertyValue::Data(JsValue::String(hmac_name)),
                shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                obj,
                PropertyKey::String(ctx.vm.well_known.hash_attr),
                PropertyValue::Data(JsValue::Object(hash_obj)),
                shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                obj,
                PropertyKey::String(ctx.vm.well_known.length),
                PropertyValue::Data(JsValue::Number(f64::from(length))),
                shape::PropertyAttrs::DATA,
            );
            obj
        }
        // AES: `{ name: "AES-GCM", length: N }` (no `hash` member)
        // (WebCrypto §27.4 `AesKeyAlgorithm`, reused by AES-CBC / AES-GCM).
        KeyAlgorithm::Aes { variant, length } => {
            let obj = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: object_proto,
                extensible: true,
            });
            let name_sid = ctx.intern(variant.canonical_name());
            ctx.vm.define_shaped_property(
                obj,
                PropertyKey::String(ctx.vm.well_known.name),
                PropertyValue::Data(JsValue::String(name_sid)),
                shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                obj,
                PropertyKey::String(ctx.vm.well_known.length),
                PropertyValue::Data(JsValue::Number(f64::from(length))),
                shape::PropertyAttrs::DATA,
            );
            obj
        }
        // HKDF / PBKDF2: a name-only `{ name: "HKDF" }` / `{ name: "PBKDF2" }`
        // (WebCrypto §33.4.2 / §34.4.2 set only the `name` attribute).
        KeyAlgorithm::Hkdf | KeyAlgorithm::Pbkdf2 => {
            let obj = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: object_proto,
                extensible: true,
            });
            let name = match algorithm {
                KeyAlgorithm::Hkdf => "HKDF",
                KeyAlgorithm::Pbkdf2 => "PBKDF2",
                _ => unreachable!("matched HKDF / PBKDF2 arm"),
            };
            let name_sid = ctx.intern(name);
            ctx.vm.define_shaped_property(
                obj,
                PropertyKey::String(ctx.vm.well_known.name),
                PropertyValue::Data(JsValue::String(name_sid)),
                shape::PropertyAttrs::DATA,
            );
            obj
        }
    }
}
