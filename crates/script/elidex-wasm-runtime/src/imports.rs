//! Engine-indep import descriptor for `WasmRuntime::instantiate`.
//!
//! WASM JS API §5.2 Instance ctor step 4 ("Read the imports") reads a
//! user-provided record-of-records (`{ module_name: { name: value } }`)
//! and matches it against the module's import list. The storage shape
//! mirrors that spec input exactly — an outer module → inner name → value
//! nesting — so `get(&str, &str)` is alloc-free and `define` writes into
//! the natural bucket. The JS host (D-16) flattens the user-facing
//! record-of-records into `ImportObject::define` calls.
//!
//! Tier-A engine-indep semantic file: no wasmtime token appears here.
//! The conversion `WasmImportValue → wasmtime::Extern` lives in
//! `engine_conv.rs` (`import_value_to_extern` free function — consumed
//! by `WasmRuntime::instantiate` when walking the import set).

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

/// Builder for the record-of-records import map used by
/// `WasmRuntime::instantiate`. Per WASM JS API §5.2 step 4 — the
/// `Default` impl yields an empty import set (used when a module
/// declares no imports).
#[derive(Default, Clone, Debug)]
pub struct ImportObject {
    entries: HashMap<String, HashMap<String, WasmImportValue>>,
}

impl ImportObject {
    pub fn new() -> Self {
        Self::default()
    }

    /// Define an import slot. Overwrites any existing entry at
    /// `(module, name)`.
    pub fn define(&mut self, module: &str, name: &str, value: WasmImportValue) {
        self.entries
            .entry(module.to_string())
            .or_default()
            .insert(name.to_string(), value);
    }

    /// Look up an import by `(module, name)`. Alloc-free — borrows the
    /// `&str` arguments directly into the nested map.
    pub fn get(&self, module: &str, name: &str) -> Option<&WasmImportValue> {
        self.entries.get(module)?.get(name)
    }

    /// Iterate `(module, name, value)` triples. Used by
    /// `WasmRuntime::instantiate` to walk the import set and convert
    /// each value to a wasmtime `Extern` via `engine_conv`.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &str, &WasmImportValue)> {
        self.entries.iter().flat_map(|(module, inner)| {
            inner
                .iter()
                .map(move |(name, value)| (module.as_str(), name.as_str(), value))
        })
    }

    /// Number of import entries currently defined (sum across modules).
    pub fn len(&self) -> usize {
        self.entries.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.values().all(HashMap::is_empty)
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
