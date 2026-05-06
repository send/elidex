//! `Storage` interface (WHATWG HTML §11.2) — VM thin binding.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! ToString argument coercion, named-property exotic dispatch, and
//! `StorageError → DOMException(QuotaExceededError)` mapping.  The
//! actual storage algorithm — quota tracking, JSON-on-disk
//! persistence, origin-keyed registry, insertion-order iteration —
//! lives in [`elidex_storage_core::WebStorageManager`] and
//! [`elidex_storage_core::SessionStorageState`].
//!
//! ## State
//!
//! There are two `Storage` instances per VM, identified by
//! [`crate::vm::value::ObjectKind::Storage::is_local`]:
//!
//! - `localStorage` (`is_local = true`) — origin-scoped, persistent.
//!   Routes through `HostData::web_storage` (an `Arc<WebStorageManager>`
//!   installed by the embedder); when no manager is installed (tests
//!   that do not opt in), falls back to `HostData::fallback_local_storage`
//!   for in-memory parity.
//! - `sessionStorage` (`is_local = false`) — per-VM, in-memory.  Routes
//!   through `HostData::session_storage`.
//!
//! Identity is preserved via `VmInner::storage_local_instance` /
//! `VmInner::storage_session_instance` so the Window getters return a
//! single cached `ObjectId` across reads (`localStorage ===
//! localStorage`).  Both fields are cleared on `Vm::unbind` to prevent
//! cross-origin data leak through a retained reference after a rebind.
//!
//! ## Named-property exotic
//!
//! WebIDL §3.10.  Storage is *not* `[LegacyOverrideBuiltIns]`, so a
//! key already exposed as an own property of the prototype chain
//! (`getItem` / `setItem` / `length` / …) takes precedence over the
//! supported-name → stored-value mapping.  Mirrors
//! [`super::dataset`]'s precedent.
//!
//! ## Origin scoping
//!
//! `localStorage` is origin-keyed.  The origin is derived from
//! `VmInner::navigation.current_url`:
//!
//! - Tuple origins (http/https/ws/wss/ftp/file with a host) → ASCII
//!   serialisation (`https://example.com`).
//! - Opaque origins (`about:blank`, `data:`, `javascript:`) →
//!   `HostData::opaque_origin_sentinel`, a per-VM stable string so
//!   two VMs with `about:blank` documents do not alias.

#![cfg(feature = "engine")]

use std::sync::Arc;

use elidex_storage_core::{StorageError, WebStorageManager};

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::named_property_exotic::{coerce_key_or_none, is_bound, key_on_prototype_chain};

// ---------------------------------------------------------------------------
// Origin derivation
// ---------------------------------------------------------------------------

/// Derive the origin string used for `localStorage` keying.  Tuple
/// origins serialise via `url::Origin::ascii_serialization`; opaque
/// origins fall back to the per-VM `HostData::opaque_origin_sentinel`
/// so `about:blank` / `data:` / `javascript:` documents do not alias
/// across VMs in the same process.
fn current_origin(vm: &VmInner) -> String {
    let parsed = vm.navigation.current_url.origin();
    if parsed.is_tuple() {
        return parsed.ascii_serialization();
    }
    vm.host_data.as_deref().map_or_else(
        || "null".to_string(),
        |hd| hd.opaque_origin_sentinel().to_string(),
    )
}

// ---------------------------------------------------------------------------
// Backend dispatch
// ---------------------------------------------------------------------------

/// Backend handle for `Storage` operations on a single area.
enum StorageBackend<'a> {
    /// Disk-backed `WebStorageManager` (the embedder-installed
    /// shared backend).  Captures the origin string so the call
    /// sites stay short.
    Local {
        manager: Arc<WebStorageManager>,
        origin: String,
    },
    /// In-memory `SessionStorageState` — used by `sessionStorage` and
    /// as the fallback for `localStorage` when no backend is installed.
    /// `&mut` borrowed from `HostData::session_storage` /
    /// `HostData::fallback_local_storage`.
    InMemory {
        state: &'a mut elidex_storage_core::SessionStorageState,
    },
}

impl StorageBackend<'_> {
    fn get(&self, key: &str) -> Option<String> {
        match self {
            Self::Local { manager, origin } => manager.local_get(origin, key),
            Self::InMemory { state } => state.get(key),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<Option<String>, StorageError> {
        match self {
            Self::Local { manager, origin } => manager.local_set(origin, key, value),
            Self::InMemory { state } => state.set(key, value),
        }
    }

    fn remove(&mut self, key: &str) -> Option<String> {
        match self {
            Self::Local { manager, origin } => manager.local_remove(origin, key),
            Self::InMemory { state } => state.remove(key),
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Local { manager, origin } => manager.local_clear(origin),
            Self::InMemory { state } => state.clear(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Local { manager, origin } => manager.local_len(origin),
            Self::InMemory { state } => state.len(),
        }
    }

    fn key(&self, index: usize) -> Option<String> {
        match self {
            Self::Local { manager, origin } => manager.local_key(origin, index),
            Self::InMemory { state } => state.key(index),
        }
    }

    fn keys(&self) -> Vec<String> {
        match self {
            Self::Local { manager, origin } => manager.local_keys(origin),
            Self::InMemory { state } => state.keys(),
        }
    }
}

/// Resolve the backend for a `Storage` instance.  Returns the
/// borrow from `HostData` plus any captured `Arc<WebStorageManager>`
/// for `localStorage`.  The `is_local` flag is read from the
/// receiver's `ObjectKind::Storage` payload via [`require_receiver`].
fn backend_for(vm: &mut VmInner, is_local: bool) -> StorageBackend<'_> {
    if is_local {
        let origin = current_origin(vm);
        let host = vm
            .host_data
            .as_deref_mut()
            .expect("backend_for: HostData required (caller checks bound)");
        if let Some(manager) = host.web_storage().cloned() {
            return StorageBackend::Local { manager, origin };
        }
        return StorageBackend::InMemory {
            state: &mut host.fallback_local_storage,
        };
    }
    let host = vm
        .host_data
        .as_deref_mut()
        .expect("backend_for: HostData required (caller checks bound)");
    StorageBackend::InMemory {
        state: &mut host.session_storage,
    }
}

// ---------------------------------------------------------------------------
// Prototype install + cached instance allocation
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `Storage.prototype` chained to `Object.prototype`,
    /// install its 5 method natives + `length` getter, and expose the
    /// `Storage` constructor stub on `globalThis`.
    ///
    /// Per WebIDL §3.7 Storage has no public constructor; the global
    /// `Storage` is installed for `instanceof` / brand-check parity
    /// with browsers and throws TypeError when invoked.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_storage_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_storage_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // 5 method natives.
        let wk = &self.well_known;
        let methods: [(StringId, NativeFn); 5] = [
            (wk.get_item, native_storage_get_item),
            (wk.set_item, native_storage_set_item),
            (wk.remove_item, native_storage_remove_item),
            (wk.clear_method, native_storage_clear),
            (wk.key, native_storage_key),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }
        // `length` is a read-only accessor (not a method) per WebIDL
        // attribute semantics — `Object.getOwnPropertyDescriptor(
        // Storage.prototype, 'length').get` returns the function.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_storage_get_length,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.storage_prototype = Some(proto_id);

        // `Storage` constructor stub — throws on call/construct, but
        // is required as a global so `localStorage instanceof Storage`
        // and `Storage.prototype` parity work (WebIDL §3.7 +
        // browser-observed behaviour).
        let ctor = self.create_constructable_function("Storage", native_storage_illegal_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.storage_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Return the cached `localStorage` / `sessionStorage` Storage
    /// wrapper for `is_local`, allocating + caching on the first call.
    /// Called from the Window prototype getters
    /// (`native_window_get_local_storage` /
    /// `native_window_get_session_storage`).
    pub(in crate::vm) fn alloc_or_cached_storage(&mut self, is_local: bool) -> ObjectId {
        if is_local {
            if let Some(id) = self.storage_local_instance {
                return id;
            }
        } else if let Some(id) = self.storage_session_instance {
            return id;
        }
        let proto = self
            .storage_prototype
            .expect("alloc_or_cached_storage before register_storage_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::Storage { is_local },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        if is_local {
            self.storage_local_instance = Some(id);
        } else {
            self.storage_session_instance = Some(id);
        }
        id
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<bool, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Storage': Illegal invocation"
        )));
    };
    let ObjectKind::Storage { is_local } = ctx.vm.get_object(id).kind else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Storage': Illegal invocation"
        )));
    };
    Ok(is_local)
}

// ---------------------------------------------------------------------------
// Constructor stub — Storage is `[NoInterfaceObject]` in practice
// ---------------------------------------------------------------------------

fn native_storage_illegal_ctor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Failed to construct 'Storage': Illegal constructor",
    ))
}

// ---------------------------------------------------------------------------
// Quota error mapping
// ---------------------------------------------------------------------------

fn quota_exceeded(vm: &VmInner, message: impl Into<String>) -> VmError {
    VmError::dom_exception(vm.well_known.dom_exc_quota_exceeded_error, message.into())
}

// ---------------------------------------------------------------------------
// Method natives
// ---------------------------------------------------------------------------

fn native_storage_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let is_local = require_receiver(ctx, this, "length")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    let backend = backend_for(ctx.vm, is_local);
    #[allow(clippy::cast_precision_loss)]
    let len = backend.len() as f64;
    Ok(JsValue::Number(len))
}

fn native_storage_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let is_local = require_receiver(ctx, this, "key")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    // WebIDL `unsigned long` — ToUint32; absent / undefined → 0
    // (rather than the 1-arg error a `[Throws]` modifier would
    // surface, matching browser-observed leniency).
    let raw = args.first().copied().unwrap_or(JsValue::Undefined);
    let raw_num = ctx.to_number(raw)?;
    let idx = super::super::coerce::f64_to_uint32(raw_num) as usize;
    let backend = backend_for(ctx.vm, is_local);
    let result = backend.key(idx);
    drop(backend);
    Ok(match result {
        Some(s) => {
            let sid = ctx.vm.strings.intern(&s);
            JsValue::String(sid)
        }
        None => JsValue::Null,
    })
}

fn native_storage_get_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let is_local = require_receiver(ctx, this, "getItem")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let key_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let key_sid = super::super::coerce::to_string(ctx.vm, key_val)?;
    let key_str = ctx.vm.strings.get_utf8(key_sid);
    let backend = backend_for(ctx.vm, is_local);
    let result = backend.get(&key_str);
    drop(backend);
    Ok(match result {
        Some(s) => JsValue::String(ctx.vm.strings.intern(&s)),
        None => JsValue::Null,
    })
}

fn native_storage_set_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let is_local = require_receiver(ctx, this, "setItem")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let key_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_val = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let key_sid = super::super::coerce::to_string(ctx.vm, key_val)?;
    let value_sid = super::super::coerce::to_string(ctx.vm, value_val)?;
    let key_str = ctx.vm.strings.get_utf8(key_sid);
    let value_str = ctx.vm.strings.get_utf8(value_sid);
    let mut backend = backend_for(ctx.vm, is_local);
    let result = backend.set(&key_str, &value_str);
    drop(backend);
    match result {
        Ok(_) => Ok(JsValue::Undefined),
        Err(err) => Err(quota_exceeded(ctx.vm, err.message)),
    }
}

fn native_storage_remove_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let is_local = require_receiver(ctx, this, "removeItem")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let key_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let key_sid = super::super::coerce::to_string(ctx.vm, key_val)?;
    let key_str = ctx.vm.strings.get_utf8(key_sid);
    let mut backend = backend_for(ctx.vm, is_local);
    backend.remove(&key_str);
    Ok(JsValue::Undefined)
}

fn native_storage_clear(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let is_local = require_receiver(ctx, this, "clear")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let mut backend = backend_for(ctx.vm, is_local);
    backend.clear();
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Named-property exotic — see `super::dataset` for the precedent.
// ---------------------------------------------------------------------------

fn area_from_id(vm: &VmInner, id: ObjectId) -> Option<bool> {
    if let ObjectKind::Storage { is_local } = vm.get_object(id).kind {
        Some(is_local)
    } else {
        None
    }
}

/// `[[HasProperty]]` trap.
pub(crate) fn try_has(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<bool, VmError>> {
    let is_local = area_from_id(vm, id)?;
    let sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, sid) {
        return None;
    }
    if !is_bound(vm) {
        return None;
    }
    let key_str = vm.strings.get_utf8(sid);
    let backend = backend_for(vm, is_local);
    let present = backend.get(&key_str).is_some();
    drop(backend);
    if present {
        Some(Ok(true))
    } else {
        None
    }
}

/// `[[Get]]` trap.
pub(crate) fn try_get(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let is_local = area_from_id(vm, id)?;
    let sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, sid) {
        return None;
    }
    if !is_bound(vm) {
        return None;
    }
    let key_str = vm.strings.get_utf8(sid);
    let backend = backend_for(vm, is_local);
    let stored = backend.get(&key_str);
    drop(backend);
    match stored {
        Some(value) => {
            let v_sid = vm.strings.intern(&value);
            Some(Ok(JsValue::String(v_sid)))
        }
        None => None,
    }
}

/// `[[Set]]` trap.  Returns `Some(Err(QuotaExceededError))` when
/// the new total bytes would overflow [`elidex_storage_core::STORAGE_QUOTA_BYTES`].
pub(crate) fn try_set(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
    value: JsValue,
) -> Option<Result<(), VmError>> {
    let is_local = area_from_id(vm, id)?;
    let key_sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, key_sid) {
        return None;
    }
    let val_sid = match super::super::coerce::to_string(vm, value) {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if !is_bound(vm) {
        return Some(Ok(()));
    }
    let key_str = vm.strings.get_utf8(key_sid);
    let value_str = vm.strings.get_utf8(val_sid);
    let mut backend = backend_for(vm, is_local);
    let result = backend.set(&key_str, &value_str);
    drop(backend);
    match result {
        Ok(_) => Some(Ok(())),
        Err(err) => Some(Err(quota_exceeded(vm, err.message))),
    }
}

/// `[[Delete]]` trap.  Always succeeds at the WebIDL level.
pub(crate) fn try_delete(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<bool, VmError>> {
    let is_local = area_from_id(vm, id)?;
    let key_sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, key_sid) {
        return None;
    }
    if !is_bound(vm) {
        return Some(Ok(true));
    }
    let key_str = vm.strings.get_utf8(key_sid);
    let mut backend = backend_for(vm, is_local);
    backend.remove(&key_str);
    Some(Ok(true))
}

/// `[[OwnPropertyKeys]]` / for-in enumeration helper.  Returns the
/// supported property names (stored keys) in insertion order, with
/// any prototype-shadowed keys filtered out.
pub(crate) fn collect_keys(
    vm: &mut VmInner,
    id: ObjectId,
) -> Option<Result<Vec<StringId>, VmError>> {
    let is_local = area_from_id(vm, id)?;
    if !is_bound(vm) {
        return Some(Ok(Vec::new()));
    }
    let backend = backend_for(vm, is_local);
    let raw_keys = backend.keys();
    drop(backend);
    let filtered = raw_keys
        .into_iter()
        .filter_map(|k| {
            let sid = vm.strings.intern(&k);
            (!key_on_prototype_chain(vm, id, sid)).then_some(sid)
        })
        .collect();
    Some(Ok(filtered))
}

impl VmInner {
    /// Clear the per-VM Storage instance cache.  Called from
    /// `Vm::unbind` so a retained `localStorage` reference cannot
    /// serve the previous origin's data after a rebind to a document
    /// with a different origin.
    pub(in crate::vm) fn clear_storage_instance_cache(&mut self) {
        self.storage_local_instance = None;
        self.storage_session_instance = None;
    }
}
