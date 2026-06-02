//! IndexedDB global + prototype registration (W3C Indexed Database API 3.0,
//! slot `#11-indexed-db-vm` / D-20 — Stage 5).
//!
//! Wires the interface objects, prototype chains, accessors, methods, and
//! handler attributes built in Stages 2–4 into `globalThis`.  The shape:
//!
//! ```text
//! indexedDB                       (ObjectKind::IdbFactory) → IDBFactory.prototype → Object.prototype
//! IDBRequest.prototype            → EventTarget.prototype
//! IDBOpenDBRequest.prototype      → IDBRequest.prototype
//! IDBDatabase.prototype           → EventTarget.prototype
//! IDBTransaction.prototype        → EventTarget.prototype
//! IDBObjectStore.prototype        → Object.prototype
//! IDBKeyRange.prototype           → Object.prototype   (ctor carries the statics)
//! IDBIndex.prototype              → Object.prototype
//! IDBCursor.prototype             → Object.prototype
//! IDBCursorWithValue.prototype    → IDBCursor.prototype
//! IDBVersionChangeEvent.prototype → Event.prototype
//! ```
//!
//! Every interface object is installed with `CallShape::IllegalConstructor`
//! (WebIDL §3.7.1 — none of the IDB interfaces has a constructor operation;
//! `new IDBRequest()` throws "Illegal constructor").
//!
//! `IDBVersionChangeEvent` IS spec-constructible (it has an
//! `IDBVersionChangeEventInit` dictionary), but the bridge only ever
//! constructs it internally (`fire_version_change_event` builds the
//! `oldVersion` / `newVersion` props dynamically — there is no precomputed
//! event shape for it).  A constructible `new IDBVersionChangeEvent(...)`
//! needs a precomputed shape (`event_shapes.rs`) + an init-dict constructor
//! (`oldVersion` `[EnforceRange] unsigned long long`, `newVersion`
//! `unsigned long long?`); that feature work is DEFERRED to
//! `#11-idbversionchangeevent-constructor`.  Until then it is installed
//! `IllegalConstructor` (interface object + `instanceof` present, construction
//! rejected) — script-side construction is niche in the single-VM model.

#![cfg(feature = "engine")]

use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    native_illegal_constructor_unreachable, CallShape, JsValue, Object, ObjectId, ObjectKind,
    PropertyStorage,
};
use super::super::super::{NativeFn, VmInner};
use super::super::events::install_ctor;
use super::{cursor, database, factory, index, key_range, object_store, request, txn};

impl VmInner {
    /// Register the `indexedDB` global, the `IDBKeyRange` constructor, and all
    /// IndexedDB interface prototypes.  Called from `register_globals` after
    /// `register_event_target_prototype` and the base `Event` prototype are in
    /// place (the IDB EventTargets chain to `EventTarget.prototype` and
    /// `IDBVersionChangeEvent.prototype` chains to `Event.prototype`).
    #[allow(clippy::too_many_lines)] // a flat one-shot registration sequence; splitting would obscure the per-interface grouping
    pub(in crate::vm) fn register_indexeddb_global(&mut self) {
        let et_proto = self
            .event_target_prototype
            .expect("register_indexeddb_global called before register_event_target_prototype");
        let obj_proto = self.object_prototype;
        let event_proto = self.event_prototype;

        // --- IDBRequest : EventTarget --------------------------------------
        // Inherits `add/remove/dispatchEvent` from `EventTarget.prototype`
        // (the shared dispatch core routes an IDB receiver to its
        // `vm_event_listeners` home via `DispatchTarget::VmObject`).
        let req_proto = self.alloc_idb_proto(Some(et_proto));
        self.idb_install_ro_getter(req_proto, "readyState", request::native_req_get_ready_state);
        self.idb_install_ro_getter(req_proto, "result", request::native_req_get_result);
        self.idb_install_ro_getter(req_proto, "error", request::native_req_get_error);
        self.idb_install_ro_getter(req_proto, "source", request::native_req_get_source);
        self.idb_install_ro_getter(
            req_proto,
            "transaction",
            request::native_req_get_transaction,
        );
        self.install_vm_object_handler_attrs(req_proto, &["onsuccess", "onerror"]);
        self.idb_request_prototype = Some(req_proto);
        self.idb_install_interface(req_proto, "IDBRequest");

        // --- IDBOpenDBRequest : IDBRequest ---------------------------------
        let open_proto = self.alloc_idb_proto(Some(req_proto));
        // Inherits the IDBRequest accessors / EventTarget shadow; adds the two
        // open-request-only handler attributes (§4.3).
        self.install_vm_object_handler_attrs(open_proto, &["onupgradeneeded", "onblocked"]);
        self.idb_open_db_request_prototype = Some(open_proto);
        self.idb_install_interface(open_proto, "IDBOpenDBRequest");

        // --- IDBDatabase : EventTarget -------------------------------------
        let db_proto = self.alloc_idb_proto(Some(et_proto));
        self.idb_install_ro_getter(db_proto, "name", database::native_db_get_name);
        self.idb_install_ro_getter(db_proto, "version", database::native_db_get_version);
        self.idb_install_ro_getter(
            db_proto,
            "objectStoreNames",
            database::native_db_get_object_store_names,
        );
        self.idb_install_method(
            db_proto,
            "createObjectStore",
            database::native_db_create_object_store,
        );
        self.idb_install_method(
            db_proto,
            "deleteObjectStore",
            database::native_db_delete_object_store,
        );
        self.idb_install_method(db_proto, "transaction", database::native_db_transaction);
        self.idb_install_method(db_proto, "close", database::native_db_close);
        self.install_vm_object_handler_attrs(
            db_proto,
            &["onabort", "onclose", "onerror", "onversionchange"],
        );
        self.idb_database_prototype = Some(db_proto);
        self.idb_install_interface(db_proto, "IDBDatabase");

        // --- IDBTransaction : EventTarget ----------------------------------
        let txn_proto = self.alloc_idb_proto(Some(et_proto));
        self.idb_install_ro_getter(txn_proto, "mode", txn::native_txn_get_mode);
        self.idb_install_ro_getter(txn_proto, "durability", txn::native_txn_get_durability);
        self.idb_install_ro_getter(txn_proto, "db", txn::native_txn_get_db);
        self.idb_install_ro_getter(txn_proto, "error", txn::native_txn_get_error);
        self.idb_install_ro_getter(
            txn_proto,
            "objectStoreNames",
            txn::native_txn_get_object_store_names,
        );
        self.idb_install_method(txn_proto, "objectStore", txn::native_txn_object_store);
        self.idb_install_method(txn_proto, "commit", txn::native_txn_commit);
        self.idb_install_method(txn_proto, "abort", txn::native_txn_abort);
        self.install_vm_object_handler_attrs(txn_proto, &["oncomplete", "onerror", "onabort"]);
        self.idb_transaction_prototype = Some(txn_proto);
        self.idb_install_interface(txn_proto, "IDBTransaction");

        // --- IDBObjectStore : Object ---------------------------------------
        let os_proto = self.alloc_idb_proto(obj_proto);
        self.idb_install_ro_getter(os_proto, "name", object_store::native_os_get_name);
        self.idb_install_ro_getter(os_proto, "keyPath", object_store::native_os_get_key_path);
        self.idb_install_ro_getter(
            os_proto,
            "autoIncrement",
            object_store::native_os_get_auto_increment,
        );
        self.idb_install_ro_getter(
            os_proto,
            "indexNames",
            object_store::native_os_get_index_names,
        );
        self.idb_install_ro_getter(
            os_proto,
            "transaction",
            object_store::native_os_get_transaction,
        );
        self.idb_install_method(os_proto, "add", object_store::native_os_add);
        self.idb_install_method(os_proto, "put", object_store::native_os_put);
        self.idb_install_method(os_proto, "get", object_store::native_os_get);
        self.idb_install_method(os_proto, "getKey", object_store::native_os_get_key);
        self.idb_install_method(os_proto, "getAll", object_store::native_os_get_all);
        self.idb_install_method(os_proto, "getAllKeys", object_store::native_os_get_all_keys);
        self.idb_install_method(os_proto, "delete", object_store::native_os_delete);
        self.idb_install_method(os_proto, "clear", object_store::native_os_clear);
        self.idb_install_method(os_proto, "count", object_store::native_os_count);
        self.idb_install_method(os_proto, "openCursor", object_store::native_os_open_cursor);
        self.idb_install_method(
            os_proto,
            "openKeyCursor",
            object_store::native_os_open_key_cursor,
        );
        self.idb_install_method(os_proto, "index", object_store::native_os_index);
        self.idb_install_method(
            os_proto,
            "createIndex",
            object_store::native_os_create_index,
        );
        self.idb_install_method(
            os_proto,
            "deleteIndex",
            object_store::native_os_delete_index,
        );
        self.idb_object_store_prototype = Some(os_proto);
        self.idb_install_interface(os_proto, "IDBObjectStore");

        // --- IDBIndex : Object ---------------------------------------------
        let idx_proto = self.alloc_idb_proto(obj_proto);
        self.idb_install_ro_getter(idx_proto, "name", index::native_index_get_name);
        self.idb_install_ro_getter(idx_proto, "keyPath", index::native_index_get_key_path);
        self.idb_install_ro_getter(idx_proto, "unique", index::native_index_get_unique);
        self.idb_install_ro_getter(idx_proto, "multiEntry", index::native_index_get_multi_entry);
        self.idb_install_ro_getter(
            idx_proto,
            "objectStore",
            index::native_index_get_object_store,
        );
        self.idb_install_method(idx_proto, "get", index::native_index_get);
        self.idb_install_method(idx_proto, "getKey", index::native_index_get_key);
        self.idb_install_method(idx_proto, "getAll", index::native_index_get_all);
        self.idb_install_method(idx_proto, "getAllKeys", index::native_index_get_all_keys);
        self.idb_install_method(idx_proto, "count", index::native_index_count);
        self.idb_install_method(idx_proto, "openCursor", index::native_index_open_cursor);
        self.idb_install_method(
            idx_proto,
            "openKeyCursor",
            index::native_index_open_key_cursor,
        );
        self.idb_index_prototype = Some(idx_proto);
        self.idb_install_interface(idx_proto, "IDBIndex");

        // --- IDBCursor : Object --------------------------------------------
        let cursor_proto = self.alloc_idb_proto(obj_proto);
        self.idb_install_ro_getter(
            cursor_proto,
            "direction",
            cursor::native_cursor_get_direction,
        );
        self.idb_install_ro_getter(cursor_proto, "key", cursor::native_cursor_get_key);
        self.idb_install_ro_getter(
            cursor_proto,
            "primaryKey",
            cursor::native_cursor_get_primary_key,
        );
        self.idb_install_ro_getter(cursor_proto, "source", cursor::native_cursor_get_source);
        self.idb_install_ro_getter(cursor_proto, "request", cursor::native_cursor_get_request);
        self.idb_install_method(cursor_proto, "advance", cursor::native_cursor_advance);
        self.idb_install_method(cursor_proto, "continue", cursor::native_cursor_continue);
        self.idb_install_method(
            cursor_proto,
            "continuePrimaryKey",
            cursor::native_cursor_continue_primary_key,
        );
        self.idb_install_method(cursor_proto, "update", cursor::native_cursor_update);
        self.idb_install_method(cursor_proto, "delete", cursor::native_cursor_delete);
        self.idb_cursor_prototype = Some(cursor_proto);
        self.idb_install_interface(cursor_proto, "IDBCursor");

        // --- IDBCursorWithValue : IDBCursor --------------------------------
        // Inherits every IDBCursor accessor / method; adds only `value`.
        let cwv_proto = self.alloc_idb_proto(Some(cursor_proto));
        self.idb_install_ro_getter(cwv_proto, "value", cursor::native_cursor_get_value);
        self.idb_cursor_with_value_prototype = Some(cwv_proto);
        self.idb_install_interface(cwv_proto, "IDBCursorWithValue");

        // --- IDBKeyRange : Object (statics live on the ctor) ---------------
        let kr_proto = self.alloc_idb_proto(obj_proto);
        self.idb_install_ro_getter(kr_proto, "lower", key_range::native_key_range_get_lower);
        self.idb_install_ro_getter(kr_proto, "upper", key_range::native_key_range_get_upper);
        self.idb_install_ro_getter(
            kr_proto,
            "lowerOpen",
            key_range::native_key_range_get_lower_open,
        );
        self.idb_install_ro_getter(
            kr_proto,
            "upperOpen",
            key_range::native_key_range_get_upper_open,
        );
        self.idb_install_method(kr_proto, "includes", key_range::native_key_range_includes);
        self.idb_key_range_prototype = Some(kr_proto);
        let kr_ctor = self.idb_install_interface(kr_proto, "IDBKeyRange");
        self.idb_install_method(kr_ctor, "only", key_range::native_key_range_only);
        self.idb_install_method(
            kr_ctor,
            "lowerBound",
            key_range::native_key_range_lower_bound,
        );
        self.idb_install_method(
            kr_ctor,
            "upperBound",
            key_range::native_key_range_upper_bound,
        );
        self.idb_install_method(kr_ctor, "bound", key_range::native_key_range_bound);

        // --- IDBFactory + the `indexedDB` singleton ------------------------
        let factory_proto = self.alloc_idb_proto(obj_proto);
        self.idb_install_method(factory_proto, "open", factory::native_idb_open);
        self.idb_install_method(
            factory_proto,
            "deleteDatabase",
            factory::native_idb_delete_database,
        );
        self.idb_install_method(factory_proto, "databases", factory::native_idb_databases);
        self.idb_install_method(factory_proto, "cmp", factory::native_idb_cmp);
        self.idb_factory_prototype = Some(factory_proto);
        self.idb_install_interface(factory_proto, "IDBFactory");
        let singleton = self.alloc_object(Object {
            kind: ObjectKind::IdbFactory,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(factory_proto),
            extensible: true,
        });
        let indexeddb_sid = self.strings.intern("indexedDB");
        self.globals
            .insert(indexeddb_sid, JsValue::Object(singleton));

        // --- IDBVersionChangeEvent : Event ---------------------------------
        // `oldVersion` / `newVersion` are installed as own data properties on
        // each event instance by `fire_version_change_event`, so the prototype
        // only needs to exist + chain to `Event.prototype`.
        let vce_proto = self.alloc_idb_proto(event_proto);
        self.idb_version_change_event_prototype = Some(vce_proto);
        self.idb_install_interface(vce_proto, "IDBVersionChangeEvent");
    }

    /// Allocate an empty IDB interface prototype chained to `parent`.
    fn alloc_idb_proto(&mut self, parent: Option<ObjectId>) -> ObjectId {
        self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: parent,
            extensible: true,
        })
    }

    /// Install an `IllegalConstructor` interface object wired to `proto`, add
    /// it to `globalThis` under `name`, and return its `ObjectId` (so callers
    /// can attach statics, e.g. `IDBKeyRange.only`).
    fn idb_install_interface(&mut self, proto: ObjectId, name: &str) -> ObjectId {
        let global_sid = self.strings.intern(name);
        install_ctor(
            self,
            proto,
            name,
            native_illegal_constructor_unreachable,
            global_sid,
            CallShape::IllegalConstructor,
        );
        match self.globals.get(&global_sid).copied() {
            Some(JsValue::Object(id)) => id,
            _ => unreachable!("install_ctor did not insert the interface object"),
        }
    }

    /// Install one read-only getter accessor keyed by `name`.
    fn idb_install_ro_getter(&mut self, proto: ObjectId, name: &str, getter: NativeFn) {
        let sid = self.strings.intern(name);
        self.install_accessor_pair(proto, sid, getter, None, PropertyAttrs::WEBIDL_RO_ACCESSOR);
    }

    /// Install one method (data property) keyed by `name`.
    fn idb_install_method(&mut self, proto: ObjectId, name: &str, func: NativeFn) {
        let sid = self.strings.intern(name);
        self.install_native_method(proto, sid, func, PropertyAttrs::METHOD);
    }
}
