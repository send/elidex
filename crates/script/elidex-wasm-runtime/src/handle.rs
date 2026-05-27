//! Opaque handles to wasmtime store-tied items: `WasmFunc`, `WasmMemory`,
//! `WasmTable`, `WasmGlobal`.
//!
//! Tier-B engine-bridge structural file — wasmtime types appear only in
//! `pub(crate) inner` fields and in the `pub(crate) WasmStoreHandle`
//! wrapper. All public methods return / accept engine-independent types
//! (`WasmValue`, `WasmValueType`, `WasmFuncType`, `WasmRef`, ...).
//! Conversion between wasmtime values/types and the engine-indep
//! counterparts lives in `engine_conv.rs` (tier C).
//!
//! Thread-affinity: every handle's `WasmStoreHandle` ties back to a
//! `wasmtime::Store<HostState>`. `HostState` is `!Send + !Sync` by design
//! (raw pointer fields used for bind/unbind — `host::state` regression
//! guard). The `static_assertions` block at the bottom of this file pins
//! handles as `!Send + !Sync` too, so an accidental `Send` change on a
//! field surfaces as a compile error rather than a runtime UB risk.
//!
//! Spec anchors:
//! - WebAssembly JS API §5.3 Memories (ctor + grow + buffer detach signal)
//! - WebAssembly JS API §5.4 Tables (ctor + get / set / grow / length)
//! - WebAssembly JS API §5.5 Globals (ctor + value getter/setter +
//!   immutable-set TypeError per setter step 5)
//! - WebAssembly JS API §5.6 Exported Functions (`WasmFunc` + funcref
//!   value carrier)

use std::cell::RefCell;
use std::rc::Rc;

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;
use static_assertions::assert_not_impl_any;
use wasmtime::Store;

use crate::engine_conv;
use crate::error::{WasmError, WasmErrorKind};
use crate::host::state::{HostState, UnbindGuard};
use crate::runtime::DEFAULT_FUEL;
use crate::value::{GrowResult, WasmFuncType, WasmRef, WasmValue, WasmValueType};

/// Per-call host binding — bundles the session, the DOM world, and the
/// document root so `WasmFunc::call` can attach the live references to
/// `HostState` for the duration of the wasm call. Per CLAUDE.md
/// "Layering mandate": this is the minimal engine-bridge surface the
/// VM host needs to invoke wasm; mutations/algorithms still flow
/// through `DomHandlerRegistry`.
pub struct ScriptHostBinding<'a> {
    pub session: &'a mut SessionCore,
    pub dom: &'a mut EcsDom,
    pub document: Entity,
}

/// Shared handle to a wasmtime `Store<HostState>`. Uses `Rc<RefCell<…>>`
/// because the store is thread-affine — `HostState` is `!Send + !Sync` by
/// design (raw pointer fields used for bind/unbind), so the
/// `Arc<Mutex<…>>` shape from the plan would be wasted indirection (and
/// would trip `clippy::arc_with_non_send_sync`). `RefCell` makes
/// double-borrow bugs surface as panics at the borrow site, matching the
/// `Mutex` panic-on-deadlock invariant the plan was reaching for.
///
/// Plan deviation: §2 D-4 originally specified
/// `Arc<Mutex<Store<HostState>>>`; this is a clippy-driven idiom swap
/// with no semantic change (still single-thread, still serial borrows).
#[derive(Clone)]
pub(crate) struct WasmStoreHandle {
    inner: Rc<RefCell<Store<HostState>>>,
}

impl WasmStoreHandle {
    pub(crate) fn new(store: Store<HostState>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(store)),
        }
    }

    pub(crate) fn borrow(&self) -> std::cell::Ref<'_, Store<HostState>> {
        self.inner.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> std::cell::RefMut<'_, Store<HostState>> {
        self.inner.borrow_mut()
    }
}

/// Opaque function reference. Wraps a wasmtime `Func` plus a shared
/// handle to the owning `Store<HostState>`. `Clone` is shallow: cloning
/// shares the underlying function reference and store.
#[derive(Clone)]
pub struct WasmFunc {
    pub(crate) inner: wasmtime::Func,
    pub(crate) store: WasmStoreHandle,
}

impl std::fmt::Debug for WasmFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmFunc").finish_non_exhaustive()
    }
}

impl WasmFunc {
    /// Engine-indep function signature per WASM JS API §5.6 / §5.1
    /// import-descriptor `kind == "function"`. Fallible because
    /// `engine_conv::wasm_value_type_from_wasmtime` returns `Err` for
    /// future-proposal `HeapType` variants (Any/Eq/I31/Struct/Array/Exn/…)
    /// — propagating that error keeps the engine-bridge surface aligned
    /// with engine_conv's "Err so proposals land additively" intent.
    pub fn func_type(&self) -> Result<WasmFuncType, WasmError> {
        let store = self.store.borrow();
        let ty = self.inner.ty(&*store);
        let params = ty
            .params()
            .map(engine_conv::wasm_value_type_from_wasmtime)
            .collect::<Result<Vec<_>, _>>()?;
        let results = ty
            .results()
            .map(engine_conv::wasm_value_type_from_wasmtime)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WasmFuncType { params, results })
    }

    /// Number of result values produced by this function. Reads the
    /// wasmtime function type directly (independent of `func_type`'s
    /// `engine_conv` conversion — result arity is observable even when
    /// individual `HeapType` variants aren't yet supported).
    pub fn result_count(&self) -> usize {
        let store = self.store.borrow();
        self.inner.ty(&*store).results().len()
    }

    /// Invoke this function per WASM JS API §5.6 Exported Functions —
    /// the spec model attaches `[[Store]]` to each Exported Function, so
    /// dispatching via `self.store` (rather than a separate instance's
    /// store) makes cross-store mismatch structurally impossible.
    ///
    /// `args` must be arity- and type-matched to `self.func_type()` —
    /// the host (D-16) coerces JS arguments to `WasmValue` before
    /// calling, so by the time we reach here the values are well-typed.
    ///
    /// The dispatch is wrapped in `UnbindGuard` so that
    /// `HostState::bind` and the matching `unbind` are paired on every
    /// exit path: `Ok`, `Err`, or panic. Without this the raw pointer
    /// fields in `HostState` could outlive the live `SessionCore` /
    /// `EcsDom` references and produce undefined behaviour from host-fn
    /// callbacks that arrive after the borrow ends.
    pub fn call(
        &self,
        args: &[WasmValue],
        bridge: ScriptHostBinding<'_>,
    ) -> Result<Vec<WasmValue>, WasmError> {
        let mut store_mut = self.store.borrow_mut();
        let ScriptHostBinding {
            session,
            dom,
            document,
        } = bridge;
        let mut guard = UnbindGuard::new(&mut store_mut, session, dom, document);

        // Reset the fuel budget for this call. Without this, cumulative
        // fuel consumption across calls eventually exhausts the budget
        // and every subsequent call — even trivial ones — traps with
        // out-of-fuel. Per-call reset bounds runaway risk to a single
        // call rather than the instance lifetime.
        guard
            .store()
            .set_fuel(DEFAULT_FUEL)
            .map_err(|e| engine_conv::wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;

        // Coerce args. Non-ExternRef paths go through the plain
        // `wasm_value_to_wasmtime`; ExternRef construction needs the
        // store for `wasmtime::ExternRef::new(store, payload)` which
        // the helper handles internally.
        let mut wasm_args: Vec<wasmtime::Val> = Vec::with_capacity(args.len());
        for arg in args {
            let val = engine_conv::wasm_value_to_wasmtime(arg.clone(), guard.store())?;
            wasm_args.push(val);
        }

        // Size the result buffer from the function's type signature.
        // Read the result arity directly from the locked store (we
        // already hold `borrow_mut`, so a separate `result_count` call
        // would panic on re-borrow).
        let result_count = self.inner.ty(&*guard.store()).results().len();
        let mut results_buf = vec![wasmtime::Val::null_func_ref(); result_count];

        self.inner
            .call(&mut *guard.store(), &wasm_args, &mut results_buf)
            .map_err(|e| engine_conv::wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;

        // Coerce results back into engine-indep values. For funcref
        // and externref the converter needs both the live store (for
        // ExternRef payload extraction) and the `WasmStoreHandle`
        // (for attaching to fresh `WasmFunc` from `Val::FuncRef`).
        let store_handle = self.store.clone();
        let mut results: Vec<WasmValue> = Vec::with_capacity(results_buf.len());
        for val in results_buf {
            let wv = engine_conv::wasm_value_from_wasmtime(val, &*guard.store(), &store_handle);
            results.push(wv);
        }
        Ok(results)
        // guard drops here: HostState::unbind() runs on Ok / Err / panic.
    }
}

/// Opaque linear memory reference per WASM JS API §5.3.
#[derive(Clone)]
pub struct WasmMemory {
    pub(crate) inner: wasmtime::Memory,
    pub(crate) store: WasmStoreHandle,
}

impl std::fmt::Debug for WasmMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmMemory").finish_non_exhaustive()
    }
}

impl WasmMemory {
    /// Linear memory size in bytes (page-aligned to 64 KiB).
    pub fn byte_size(&self) -> usize {
        let store = self.store.borrow();
        self.inner.data_size(&*store)
    }

    /// Grow the memory by `delta` pages. Returns the previous size and
    /// a signal indicating that any `ArrayBuffer` aliasing the old
    /// backing store must be detached.
    ///
    /// Per WASM JS API §5.3 `refresh the Memory buffer` step 5, the spec
    /// detaches `memory.[[BufferObject]]` and rebinds a fresh ArrayBuffer
    /// on every successful grow **for fixed-length backing buffers**
    /// (`IsFixedLengthArrayBuffer(buffer)` true) — independent of whether
    /// wasmtime relocated the backing allocation. Step 6 (resizable
    /// buffers) refreshes in-place without detach; elidex MVP does not
    /// surface resizable ArrayBuffer support, so the fixed-length branch
    /// is the only observable path. We therefore always signal
    /// invalidation; the host (D-16) is responsible for detaching the JS
    /// ArrayBuffer per spec. (An earlier draft optimized this via
    /// `data_ptr` pre/post compare, but that yields spec-violating
    /// false-negatives in the fixed-length branch — a successful grow
    /// with unchanged base pointer must still detach.)
    pub fn grow(&mut self, delta: u32) -> Result<GrowResult, WasmError> {
        let mut store = self.store.borrow_mut();
        let pre_pages = self
            .inner
            .grow(&mut *store, u64::from(delta))
            .map_err(|e| engine_conv::wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;
        let pre_pages_u32 = u32::try_from(pre_pages).map_err(|_| {
            WasmError::new(
                WasmErrorKind::Runtime,
                "memory page count exceeds u32::MAX".to_string(),
            )
        })?;
        Ok(GrowResult {
            pre_pages: pre_pages_u32,
            buffer_handle_invalidated: true,
        })
    }

    /// Read-only byte access with the wasmtime store lock held for the
    /// duration of the closure. Raw pointer is not exposed across the
    /// crate boundary — the closure-form keeps lifetime safety structural.
    pub fn with_data<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        let store = self.store.borrow();
        let data = self.inner.data(&*store);
        f(data)
    }

    /// Mutable byte access with the wasmtime store lock held for the
    /// duration of the closure.
    pub fn with_data_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut store = self.store.borrow_mut();
        let data = self.inner.data_mut(&mut *store);
        f(data)
    }
}

/// Opaque table reference per WASM JS API §5.4.
#[derive(Clone)]
pub struct WasmTable {
    pub(crate) inner: wasmtime::Table,
    pub(crate) store: WasmStoreHandle,
}

impl std::fmt::Debug for WasmTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTable").finish_non_exhaustive()
    }
}

impl WasmTable {
    /// Current size in entries.
    pub fn length(&self) -> u32 {
        let store = self.store.borrow();
        u32::try_from(self.inner.size(&*store)).unwrap_or(u32::MAX)
    }

    /// Read entry at `index`; `None` if out of bounds.
    pub fn get(&self, index: u32) -> Option<WasmRef> {
        let mut store = self.store.borrow_mut();
        let r = self.inner.get(&mut *store, u64::from(index))?;
        Some(engine_conv::wasm_ref_from_wasmtime(&r, &store, &self.store))
    }

    /// Write `value` at `index`.
    pub fn set(&mut self, index: u32, value: WasmRef) -> Result<(), WasmError> {
        let mut store = self.store.borrow_mut();
        let r = engine_conv::wasm_ref_to_wasmtime(value, &mut store)?;
        self.inner
            .set(&mut *store, u64::from(index), r)
            .map_err(|e| engine_conv::wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))
    }

    /// Grow by `delta` entries, filling new slots with `init`. Returns
    /// the previous size in entries.
    pub fn grow(&mut self, delta: u32, init: WasmRef) -> Result<u32, WasmError> {
        let mut store = self.store.borrow_mut();
        let init = engine_conv::wasm_ref_to_wasmtime(init, &mut store)?;
        let prev = self
            .inner
            .grow(&mut *store, u64::from(delta), init)
            .map_err(|e| engine_conv::wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))?;
        u32::try_from(prev).map_err(|_| {
            WasmError::new(
                WasmErrorKind::Runtime,
                "table entry count exceeds u32::MAX".to_string(),
            )
        })
    }
}

/// Opaque global reference per WASM JS API §5.5.
#[derive(Clone)]
pub struct WasmGlobal {
    pub(crate) inner: wasmtime::Global,
    pub(crate) store: WasmStoreHandle,
}

impl std::fmt::Debug for WasmGlobal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmGlobal").finish_non_exhaustive()
    }
}

impl WasmGlobal {
    /// Engine-indep content type.
    pub fn value_type(&self) -> Result<WasmValueType, WasmError> {
        let store = self.store.borrow();
        let ty = self.inner.ty(&*store);
        engine_conv::wasm_value_type_from_wasmtime(ty.content().clone())
    }

    /// `true` if the global is declared `(mut ...)`.
    pub fn mutable(&self) -> bool {
        let store = self.store.borrow();
        matches!(
            self.inner.ty(&*store).mutability(),
            wasmtime::Mutability::Var
        )
    }

    /// Read the current value.
    pub fn get(&self) -> WasmValue {
        let mut store = self.store.borrow_mut();
        let v = self.inner.get(&mut *store);
        engine_conv::wasm_value_from_wasmtime(v, &store, &self.store)
    }

    /// Write a new value. Per WASM JS API §5.5 setter step 5, writing to
    /// an immutable global is a TypeError on the JS side — surfaced here
    /// as `WasmError { kind: Runtime, ... }`; the host (D-16) is
    /// responsible for marshalling the TypeError shape.
    pub fn set(&mut self, value: WasmValue) -> Result<(), WasmError> {
        let mut store = self.store.borrow_mut();
        let v = engine_conv::wasm_value_to_wasmtime(value, &mut store)?;
        self.inner
            .set(&mut *store, v)
            .map_err(|e| engine_conv::wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))
    }
}

// HostState is `!Send + !Sync` by design (`host::state` regression guard
// on raw pointer fields). These compile-time asserts pin each handle
// type to inherit that invariant; any future field change that
// accidentally introduces `Send` / `Sync` fails to compile here,
// blocking the UB risk at the type system layer.
//
// `WasmStoreHandle` wraps `Rc<RefCell<Store<HostState>>>`. `Rc` is
// `!Send + !Sync` unconditionally, so the property holds transitively
// for every handle that owns a `WasmStoreHandle`.
const _: () = {
    assert_not_impl_any!(WasmStoreHandle: Send, Sync);
    assert_not_impl_any!(WasmFunc: Send, Sync);
    assert_not_impl_any!(WasmMemory: Send, Sync);
    assert_not_impl_any!(WasmTable: Send, Sync);
    assert_not_impl_any!(WasmGlobal: Send, Sync);
};
