//! `WebAssembly` global host bindings — slot `#11-wasm-vm` / D-16.
//!
//! Sub-directory consolidating the JS-wrapper layer over the
//! engine-bridge `elidex-wasm-runtime` crate (F1 + F2 + F3 surface).
//! All ObjectKind brand checks + side-store inserts + spec-prescribed
//! marshalling live here; wasm execution + memory backing + linker
//! state are encapsulated inside the engine-bridge crate.
//!
//! ## Layering (CLAUDE.md "Layering mandate" + plan-memo §4.3 trip-wires)
//!
//! The 8 files in this sub-directory (`namespace.rs` / `errors.rs` /
//! `module.rs` / `instance.rs` / `memory.rs` / `table.rs` / `global.rs`
//! / `exported_func.rs`) must hold zero `wasmtime::` token references
//! (trip-wire #1).  All wasm algorithms route through engine-indep
//! API surfaces re-exported from
//! [`elidex_wasm_runtime`]: `WasmRuntime::{new, validate, compile,
//! instantiate, new_memory, new_table, new_global}` / `WasmModule::
//! {imports, exports, custom_sections}` / `WasmInstance::exports` /
//! `WasmMemory::{view, grow}` / `WasmTable::{element_kind, length,
//! get, set, grow}` / `WasmGlobal::{value_type, mutable, get, set}` /
//! `WasmFunc::{func_type, call}` / `WasmError::{kind, message}`.
//!
//! ## Stage layout (plan-memo §5)
//!
//! - **Stage 1** (this PR): `VmInner` storage fields + 6 ObjectKind
//!   variants + payload structs + GC trace/sweep + unbind scrub.
//! - **Stage 2** (this PR): namespace install + 3 error class install
//!   + `validate` / `compile` natives + `Module` ctor + 3 static methods.
//! - **Stage 3**: `Instance` + exports + exported function exotic.
//! - **Stage 4**: Memory / Table / Global standalone ctors + DR-11
//!   byte_io routing infrastructure (plan-memo §5 Stage 4.1).
//! - **Stage 5**: Final GC integration + trip-wire script + tests.

#![cfg(feature = "engine")]

pub(super) mod errors;
pub(super) mod module;
pub(super) mod namespace;

use std::sync::Arc;

use elidex_wasm_runtime::{WasmError, WasmRuntime};

use super::super::VmInner;

impl VmInner {
    /// Lazily-initialized accessor for the engine-bridge `WasmRuntime`
    /// singleton (plan-memo §4.1 / §5 Stage 1.3).
    ///
    /// First access calls [`WasmRuntime::new`] which builds fresh
    /// `Arc<DomHandlerRegistry>` + `Arc<CssomHandlerRegistry>`
    /// internally — those registries are runtime-internal (not
    /// per-DOM-session) so the runtime is cross-DOM reusable and
    /// retained across `Vm::unbind` per plan-memo §2.4.
    ///
    /// # Errors
    ///
    /// Surfaces [`WasmError::Runtime`] when the underlying wasmtime
    /// engine cannot be constructed (engine-bridge concern — e.g. on
    /// platforms where the wasmtime cranelift backend is unavailable).
    /// All 3 namespace methods (`validate` / `compile` / `instantiate`) +
    /// 5 ctors propagate this as a JS `WebAssembly.RuntimeError` via
    /// [`errors::wasm_error_to_js_value`].
    pub(crate) fn vm_wasm_runtime(&self) -> Result<&Arc<WasmRuntime>, WasmError> {
        if let Some(rt) = self.wasm_runtime.get() {
            return Ok(rt);
        }
        let rt = WasmRuntime::new()?;
        // First-write wins — `OnceCell::set` returns `Err(value)` if
        // a concurrent setter beat us (impossible in single-threaded
        // VM, but the API surface is uniform).  On a `set` collision
        // we discard our `rt` and return the winning `Arc`.
        let _ = self.wasm_runtime.set(Arc::new(rt));
        Ok(self
            .wasm_runtime
            .get()
            .expect("OnceCell populated immediately above"))
    }
}
