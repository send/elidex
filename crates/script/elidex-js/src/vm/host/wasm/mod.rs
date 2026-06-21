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
pub(in crate::vm) mod exported_func;
pub(super) mod global;
pub(super) mod instance;
pub(super) mod memory;
pub(super) mod module;
pub(super) mod namespace;
pub(super) mod table;

use std::sync::Arc;

use elidex_wasm_runtime::{WasmError, WasmRuntime};

use super::super::VmInner;

impl VmInner {
    /// Lazily-initialized accessor for the engine-bridge `WasmRuntime`
    /// singleton (plan-memo §4.1 / §5 Stage 1.3).
    ///
    /// First access builds the runtime via [`WasmRuntime::with_registries`]
    /// with a **policy-aware** DOM registry derived from this VM's engine mode
    /// (`spec_level_policy`) — so the A1 Web-API core/compat gate is engine-wide
    /// for Wasm-enabled sessions too: a `BrowserCore`/`App` VM's Wasm `elidex`
    /// imports cannot reach a `Legacy` DOM handler the mode excludes (Codex R5).
    /// The registries are runtime-internal (not per-DOM-session) so the runtime
    /// is cross-DOM reusable and retained across `Vm::unbind` per plan-memo §2.4
    /// — sound because the policy is fixed at VM construction and never mutated.
    ///
    /// # Errors
    ///
    /// Surfaces a [`WasmError`] whose `kind` reflects the underlying
    /// wasmtime failure: typically `WasmErrorKind::Compile` (e.g.
    /// platforms where the wasmtime cranelift backend is unavailable
    /// to construct the engine) or `WasmErrorKind::Link` (host
    /// function registration failure during `with_registries`).  All
    /// 3 namespace methods (`validate` / `compile` / `instantiate`) +
    /// 5 ctors propagate this via [`errors::wasm_error_to_js_value`],
    /// which is **kind-based**: `Compile → WebAssembly.CompileError`,
    /// `Link → WebAssembly.LinkError`, `Runtime → WebAssembly.RuntimeError`.
    pub(crate) fn vm_wasm_runtime(&self) -> Result<&Arc<WasmRuntime>, WasmError> {
        if let Some(rt) = self.wasm_runtime.get() {
            return Ok(rt);
        }
        // Build the runtime's DOM registry under THIS VM's engine-mode policy
        // (not the default `BrowserCompat`), so the gate covers the Wasm
        // `elidex` import path as well as the direct DOM bridge (Codex R5).
        let dom_registry = Arc::new(elidex_dom_api::registry::create_dom_registry_with_policy(
            self.spec_level_policy,
        ));
        let cssom_registry = Arc::new(elidex_dom_api::registry::create_cssom_registry());
        let rt = WasmRuntime::with_registries(dom_registry, cssom_registry)?;
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
