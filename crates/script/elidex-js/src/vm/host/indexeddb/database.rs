//! IDBDatabase wrapper allocation (W3C IndexedDB §4.4).
//!
//! The connection-level members (`createObjectStore` / `transaction` /
//! `close` / `name` / `version`) are installed on `IDBDatabase.prototype`
//! in Stage 4/5; this module provides the wrapper + side-store allocation
//! shared by the factory `open` flow and (future) `versionchange` paths.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::super::VmInner;
use super::IdbDatabaseState;

/// Allocate an `IDBDatabase` connection wrapper + its side-store state.
pub(crate) fn create_database_wrapper(vm: &mut VmInner, db_name: &str, version: u64) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbDatabase,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_database_prototype,
        extensible: true,
    });
    vm.idb_database_states.insert(
        id,
        IdbDatabaseState {
            db_name: db_name.to_string(),
            version,
            closed: false,
            ..Default::default()
        },
    );
    id
}
