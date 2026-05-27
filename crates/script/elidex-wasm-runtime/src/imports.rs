//! Engine-indep import descriptor for `WasmRuntime::instantiate`.
//!
//! WASM JS API §5.2 Instance ctor step 4 ("Read the imports") reads a
//! user-provided record-of-records (`{ module_name: { name: value } }`)
//! and matches it against the module's import list. Inside this crate
//! we represent that input as a flat `(module, name) -> value` map —
//! the JS host (D-16) is responsible for flattening the user-facing
//! record-of-records into `ImportObject::define` calls.
//!
//! Tier-A engine-indep semantic file: no wasmtime token appears here.
//! The conversion `WasmImportValue → wasmtime::Extern` lives in
//! `engine_conv.rs` (`ImportObject::to_extern`).

use std::collections::HashMap;

use crate::handle::{WasmFunc, WasmGlobal, WasmMemory, WasmTable};

/// Engine-indep value supplied to `WasmRuntime::instantiate` for a
/// single `(module, name)` import slot. Mirrors the four MVP import
/// kinds (`Function` / `Memory` / `Table` / `Global`); future
/// proposals (Exception Handling `Tag`) extend additively when
/// `ImportExportKind::Tag` is plumbed.
#[derive(Clone, Debug)]
pub enum WasmImportValue {
    Func(WasmFunc),
    Memory(WasmMemory),
    Table(WasmTable),
    Global(WasmGlobal),
}

/// Builder for the `(module, name) → WasmImportValue` map used by
/// `WasmRuntime::instantiate`. Per WASM JS API §5.2 step 4 — the
/// `Default` impl yields an empty import set (used when a module
/// declares no imports).
#[derive(Default, Clone, Debug)]
pub struct ImportObject {
    entries: HashMap<(String, String), WasmImportValue>,
}

impl ImportObject {
    pub fn new() -> Self {
        Self::default()
    }

    /// Define an import slot. Overwrites any existing entry at
    /// `(module, name)`.
    pub fn define(&mut self, module: &str, name: &str, value: WasmImportValue) {
        self.entries
            .insert((module.to_string(), name.to_string()), value);
    }

    /// Look up an import by `(module, name)`.
    pub fn get(&self, module: &str, name: &str) -> Option<&WasmImportValue> {
        self.entries.get(&(module.to_string(), name.to_string()))
    }

    /// Iterate `((module, name), value)` pairs. Used by
    /// `WasmRuntime::instantiate` to walk the import set and convert
    /// each value to a wasmtime `Extern` via `engine_conv`.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&(String, String), &WasmImportValue)> {
        self.entries.iter()
    }

    /// Number of import entries currently defined.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let imports = ImportObject::default();
        assert!(imports.is_empty());
        assert_eq!(imports.len(), 0);
        assert!(imports.get("any", "any").is_none());
    }

    // Note: `define` / `get` / `iter` round-trip tests require constructing
    // a `WasmImportValue` variant, which requires a live `WasmStoreHandle`
    // (and thus a `WasmRuntime`).  Stage 11 integration tests cover the
    // populated-import scenario; here we exercise only the empty / shape
    // invariants which are wasmtime-free.

    #[test]
    fn new_equivalent_to_default() {
        let a = ImportObject::new();
        let b = ImportObject::default();
        assert_eq!(a.len(), b.len());
        assert_eq!(a.is_empty(), b.is_empty());
    }
}
