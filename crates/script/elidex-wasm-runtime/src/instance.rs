//! Live module instance — engine-indep wrapper over `wasmtime::Instance`.
//!
//! `WasmInstance` wraps a wasmtime `Instance` plus a shared `WasmStoreHandle`
//! that all its exported handles (`WasmFunc` / `WasmMemory` / ...) share.
//! Function dispatch lives on `WasmFunc::call` (handle.rs) — the WASM JS
//! API §5.6 model attaches `[[Store]]` to each Exported Function, so
//! calling through the function (not the instance) makes cross-store
//! mismatch structurally impossible.
//!
//! Spec anchors:
//! - WASM JS API §5.2 Instance ctor (instance is the output of
//!   `WasmRuntime::instantiate`)
//! - WASM JS API §5.2 `initialize an Instance object` step 3
//!   (`instanceObject.[[Exports]]` set to the exportsObject). The IDL
//!   for `Instance.exports` has no `[SameObject]` attribute; stable
//!   wrapper identity is an elidex implementation choice (the
//!   wrapper-cache layer in D-16), not a spec mandate.

use crate::engine_conv;
use crate::handle::{WasmFunc, WasmGlobal, WasmMemory, WasmStoreHandle, WasmTable};

/// Engine-indep representation of one exported item from a
/// `WasmInstance`. Returned by `WasmInstance::exports()` so the host
/// can iterate exports without touching wasmtime types.
#[derive(Clone, Debug)]
pub enum WasmExportItem {
    Func(WasmFunc),
    Memory(WasmMemory),
    Table(WasmTable),
    Global(WasmGlobal),
}

/// A live wasm module instance. Clone-shared with its exported handles
/// via `WasmStoreHandle` — dropping the last `WasmInstance` (and the
/// last export referencing the same store) drops the underlying
/// wasmtime `Store<HostState>`.
#[derive(Clone)]
pub struct WasmInstance {
    pub(crate) inner: wasmtime::Instance,
    pub(crate) store: WasmStoreHandle,
}

impl std::fmt::Debug for WasmInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmInstance").finish_non_exhaustive()
    }
}

impl WasmInstance {
    pub(crate) fn new(inner: wasmtime::Instance, store: WasmStoreHandle) -> Self {
        Self { inner, store }
    }

    /// Iterate exports of this instance — engine-indep view per WASM JS
    /// API §5.2 `[[Exports]]`. Unsupported variants (Tag, SharedMemory)
    /// are skipped silently; they only appear when future proposals
    /// (Exception Handling / Threads) land additively.
    pub fn exports(&self) -> Vec<(String, WasmExportItem)> {
        let mut store = self.store.borrow_mut();
        self.inner
            .exports(&mut *store)
            .filter_map(|exp| {
                let name = exp.name().to_string();
                let ext = exp.into_extern();
                engine_conv::export_item_from_wasmtime_extern(&ext, &self.store)
                    .map(|item| (name, item))
            })
            .collect()
    }

    pub fn get_func(&self, name: &str) -> Option<WasmFunc> {
        let mut store = self.store.borrow_mut();
        let f = self.inner.get_func(&mut *store, name)?;
        Some(WasmFunc {
            inner: f,
            store: self.store.clone(),
        })
    }

    pub fn get_memory(&self, name: &str) -> Option<WasmMemory> {
        let mut store = self.store.borrow_mut();
        let m = self.inner.get_memory(&mut *store, name)?;
        Some(WasmMemory {
            inner: m,
            store: self.store.clone(),
        })
    }

    pub fn get_table(&self, name: &str) -> Option<WasmTable> {
        let mut store = self.store.borrow_mut();
        let t = self.inner.get_table(&mut *store, name)?;
        Some(WasmTable {
            inner: t,
            store: self.store.clone(),
        })
    }

    pub fn get_global(&self, name: &str) -> Option<WasmGlobal> {
        let mut store = self.store.borrow_mut();
        let g = self.inner.get_global(&mut *store, name)?;
        Some(WasmGlobal {
            inner: g,
            store: self.store.clone(),
        })
    }
}
