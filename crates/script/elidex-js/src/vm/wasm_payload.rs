//! Side-store payload structs for the 6 `WebAssembly.*` `ObjectKind`
//! variants (WASM JS API §5.1-§5.6).
//!
//! Split from `vm/mod.rs` per plan-memo D-16 DR-2 — the file is
//! kept small enough that inlining would violate the project's
//! 1000-line-file discipline once the parent grows further (currently
//! >2000 lines).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate" + plan-memo §4.3 trip-wire #2, the
//! payload structs hold only **engine-independent** handles from
//! [`elidex_wasm_runtime`].  No `wasmtime::` token may appear in this
//! file; the engine-bridge crate encapsulates wasmtime internally
//! behind `pub(crate)` visibility (see
//! [`elidex_wasm_runtime`] tier table in its lib docstring).
//!
//! ## ObjectId fields
//!
//! Several payloads carry lazily-initialized `Option<ObjectId>` fields
//! (`buffer_id`, `exports_id`) rather than an `ObjectId(0)` sentinel —
//! the type-level `None` makes "not yet allocated" structurally visible
//! and avoids genesis-object aliasing.  GC trace skips `None` arms.
//!
//! ## Cross-DOM unbind
//!
//! All 6 storage maps + the `wasm_backed_buffers` reverse-lookup map
//! are flushed in `Vm::unbind` per plan-memo §2.4 — the payloads carry
//! per-VM identity handles whose `WasmStoreHandle` lifetime is bounded
//! to the bind cycle.  The `wasm_runtime: OnceCell<Arc<WasmRuntime>>`
//! field is **retained** across unbind (the runtime owns its own
//! `Arc<DomHandlerRegistry>` independently of the per-DOM session
//! state — shared cross-cutting state per CLAUDE.md
//! side-store→component rule).

#![cfg(feature = "engine")]

use elidex_wasm_runtime::{
    WasmFunc, WasmGlobal, WasmInstance, WasmMemory, WasmMemoryView, WasmModule, WasmTable,
    WasmValueType,
};

use super::value::ObjectId;

/// `WebAssembly.Module` side-store payload (WASM JS API §5.1).
///
/// The engine-indep `WasmModule` owns its source bytes internally
/// (`Arc<[u8]>`) for `customSections(name)` lookup — no duplicate
/// `source_bytes` field at the VM layer (plan-memo §2.2).
pub(crate) struct WasmModulePayload {
    pub module: WasmModule,
}

// Stages 3-4 will wire the remaining `pub` fields below.  Stage 1+2
// (this PR) only inserts payloads — readers land alongside the
// matching JS-host code (Instance.exports / Memory.buffer /
// Table.get-set / Global.value / exported function call adapter).
// Per-struct `#[allow(dead_code)]` keeps clippy `-D warnings` green
// until each reader lands.

/// `WebAssembly.Instance` side-store payload (WASM JS API §5.2).
///
/// `module_id` keeps the parent Module alive while the instance
/// exists (GC trace marks it — see [`crate::vm::object_kind::ObjectKind::WasmInstance`]).
/// `exports_id` caches the wrapper-identity-stable exports namespace
/// (IDL has no `[SameObject]`, but `Object.isFrozen(i.exports) === true`
/// + cycle-avoidance motivate stable identity — plan-memo DR-4).
#[allow(dead_code)]
pub(crate) struct WasmInstancePayload {
    pub instance: WasmInstance,
    /// Parent Module ObjectId — always set at ctor time, traced by GC.
    pub module_id: ObjectId,
    /// Cached exports namespace ObjectId — `None` until first
    /// `instance.exports` access; lazily-allocated `Object.freeze`d
    /// dict mapping export-name → per-export wrapper.
    pub exports_id: Option<ObjectId>,
}

/// `WebAssembly.Memory` side-store payload (WASM JS API §5.3).
///
/// The cached ArrayBuffer aliasing wasm linear memory + the live
/// [`WasmMemoryView`] backing it both live here:
/// - `buffer_id` is the JS-visible `ArrayBuffer` wrapper; `None` until
///   first `.buffer` access.
/// - `view` is the engine-bridge live view (`read` / `write` /
///   `byte_size` / `is_detached`) per F2 DR-1.  Stashed so the
///   wasm-backed routing path at byte_io layer can dispatch through
///   it via `wasm_backed_buffers` reverse-lookup (plan-memo DR-11).
///
/// On detach (Memory.grow via F1 `GrowResult { buffer_handle_invalidated: true }`
/// or future explicit detach), both fields reset to `None` so the
/// next `.buffer` access allocates a fresh wrapper + fresh view over
/// the post-grow backing (plan-memo §5 Stage 4.1).
#[allow(dead_code)]
pub(crate) struct WasmMemoryPayload {
    pub memory: WasmMemory,
    pub buffer_id: Option<ObjectId>,
    pub view: Option<WasmMemoryView>,
}

/// `WebAssembly.Table` side-store payload (WASM JS API §5.4).
///
/// `element_kind` is read once via F2 `WasmTable::element_kind()`
/// at ctor / exports-wrap time (IMMUTABLE post-build: wasm validation
/// fixes the table element type) and cached for JS-side `.set(idx,
/// value)` coerce per declared kind.
#[allow(dead_code)]
pub(crate) struct WasmTablePayload {
    pub table: WasmTable,
    pub element_kind: WasmValueType,
}

/// `WebAssembly.Global` side-store payload (WASM JS API §5.5).
///
/// `value_type` / `mutable` are read on demand via
/// `WasmGlobal::value_type()` / `mutable()` (sentinel discipline per
/// plan-memo §2.2 — no duplicate metadata fields).
#[allow(dead_code)]
pub(crate) struct WasmGlobalPayload {
    pub global: WasmGlobal,
}

/// Exported function side-store payload (WASM JS API §5.6).
///
/// `func` carries its own `WasmStoreHandle` clone (F1 D-ii encapsulated
/// `Rc<RefCell<Store<HostState>>>`), so cross-store mismatch is
/// structurally impossible (`[[FunctionAddress]]` interpreted relative
/// to the surrounding agent's associated store per §4.1).
///
/// `params` is cached at exports-build time so the per-call path skips
/// `WasmFunc::func_type()` — that walk borrows the wasmtime store
/// (`RefCell::borrow_mut`) and re-traverses the engine type table on
/// every call (F1 Ω-2 / F8 lesson).  Caching also moves any
/// future-proposal HeapType conversion error from per-call to
/// module-load time, matching the fail-fast intent.
///
/// `instance_id` keeps the parent `WasmInstance` (and through it, the
/// module + linker state) alive for the lifetime of the exported
/// function — GC trace marks it (see
/// [`crate::vm::object_kind::ObjectKind::WasmExportedFunction`]).
#[allow(dead_code)]
pub(crate) struct WasmExportedFuncPayload {
    pub func: WasmFunc,
    pub params: Vec<WasmValueType>,
    pub instance_id: ObjectId,
}
