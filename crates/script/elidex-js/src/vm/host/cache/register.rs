//! Cache API global + prototype registration (WHATWG Service Workers §5;
//! slot `#11-cache-api-vm` / D-19 PR-1).
//!
//! ```text
//! caches              (ObjectKind::CacheStorage singleton) → CacheStorage.prototype → Object.prototype
//! CacheStorage.prototype  → Object.prototype
//! Cache.prototype         → Object.prototype
//! ```
//!
//! Both interface objects are installed `IllegalConstructor` (WebIDL
//! §3.7.1 — `new Cache()` / `new CacheStorage()` throw "Illegal
//! constructor"); the `caches` singleton is the only way to reach a
//! `CacheStorage`, and `caches.open(...)` the only way to reach a `Cache`.

#![cfg(feature = "engine")]

use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    native_illegal_constructor_unreachable, CallShape, JsValue, Object, ObjectId, ObjectKind,
    PropertyStorage,
};
use super::super::super::{NativeFn, VmInner};
use super::super::events::install_ctor;
use super::natives;

impl VmInner {
    /// Register the `caches` global + the `CacheStorage` / `Cache`
    /// prototypes.  Called from `register_globals` after
    /// `register_request_global` / `register_response_global` (the `caches`
    /// natives build `Response` / `Request` wrappers, so those prototypes —
    /// and their body-mixin methods — must already exist).
    pub(in crate::vm) fn register_cache_api_global(&mut self) {
        let obj_proto = self.object_prototype;

        // --- Cache : Object ---------------------------------------------
        // `add` / `addAll` are intentionally absent (slot
        // `#11-cache-add-fetch-integration`) — they need fetch-broker
        // continuation the VM lacks; faking them would corrupt the cache.
        let cache_proto = self.alloc_cache_proto(obj_proto);
        self.cache_install_method(cache_proto, "match", natives::native_cache_match);
        self.cache_install_method(cache_proto, "matchAll", natives::native_cache_match_all);
        self.cache_install_method(cache_proto, "put", natives::native_cache_put);
        self.cache_install_method(cache_proto, "delete", natives::native_cache_delete);
        self.cache_install_method(cache_proto, "keys", natives::native_cache_keys);
        self.cache_prototype = Some(cache_proto);
        self.cache_install_interface(cache_proto, "Cache");

        // --- CacheStorage : Object --------------------------------------
        let cs_proto = self.alloc_cache_proto(obj_proto);
        self.cache_install_method(cs_proto, "open", natives::native_caches_open);
        self.cache_install_method(cs_proto, "has", natives::native_caches_has);
        self.cache_install_method(cs_proto, "delete", natives::native_caches_delete);
        self.cache_install_method(cs_proto, "keys", natives::native_caches_keys);
        self.cache_install_method(cs_proto, "match", natives::native_caches_match);
        self.cache_storage_prototype = Some(cs_proto);
        self.cache_install_interface(cs_proto, "CacheStorage");

        // --- `caches` singleton (§5.3.1 `self.caches`) ------------------
        let singleton = self.alloc_object(Object {
            kind: ObjectKind::CacheStorage,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(cs_proto),
            extensible: true,
        });
        let caches_sid = self.strings.intern("caches");
        self.globals.insert(caches_sid, JsValue::Object(singleton));
    }

    /// Allocate an empty Cache API interface prototype chained to `parent`.
    fn alloc_cache_proto(&mut self, parent: Option<ObjectId>) -> ObjectId {
        self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: parent,
            extensible: true,
        })
    }

    /// Install an `IllegalConstructor` interface object wired to `proto`
    /// and exposed on `globalThis` under `name`.
    fn cache_install_interface(&mut self, proto: ObjectId, name: &str) {
        let global_sid = self.strings.intern(name);
        install_ctor(
            self,
            proto,
            name,
            native_illegal_constructor_unreachable,
            global_sid,
            CallShape::IllegalConstructor,
        );
    }

    /// Install one method (data property) keyed by `name`.
    fn cache_install_method(&mut self, proto: ObjectId, name: &str, func: NativeFn) {
        let sid = self.strings.intern(name);
        self.install_native_method(proto, sid, func, PropertyAttrs::METHOD);
    }
}
