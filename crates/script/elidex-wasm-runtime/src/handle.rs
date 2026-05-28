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

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

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
    /// the spec model gives each Exported Function a `[[FunctionAddress]]`
    /// interpreted relative to the surrounding agent's associated store
    /// (§4.1 "Interaction of the WebAssembly Store with JavaScript"). By
    /// dispatching via `self.store` (rather than a separate instance's
    /// store), cross-store mismatch is structurally impossible.
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
///
/// `view_flags` tracks every live `WasmMemoryView` issued from this
/// Memory so that `grow` can detach them per WASM JS API §5.3 "refresh
/// the Memory buffer" step 5.1. The tracking lives in an `Rc<RefCell<…>>`
/// so the `#[derive(Clone)]` impl shares it across all clones of the
/// same Memory — required for spec correctness, because `WasmMemory:
/// Clone` shares the underlying wasmtime backing, so a grow via *any*
/// clone must invalidate views allocated via *any* clone.
#[derive(Clone)]
pub struct WasmMemory {
    pub(crate) inner: wasmtime::Memory,
    pub(crate) store: WasmStoreHandle,
    pub(crate) view_flags: Rc<RefCell<Vec<Weak<Cell<bool>>>>>,
}

/// Threshold for opportunistic cleanup of dead `Weak<Cell<bool>>` entries
/// in `WasmMemory::view_flags`. The cleanup runs at `view()` time when the
/// vector length exceeds this value — bounds unbounded growth when JS
/// allocates many views without growing the memory (each `m.buffer` access
/// in D-16 may issue a fresh view + drop it). Threshold-only knob: only
/// affects cleanup batch frequency, not detach correctness.
const VIEW_FLAGS_CLEANUP_THRESHOLD: usize = 64;

impl std::fmt::Debug for WasmMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmMemory").finish_non_exhaustive()
    }
}

impl WasmMemory {
    /// Internal ctor used by `runtime::WasmRuntime::new_memory` and by
    /// `engine_conv::export_item_from_wasmtime_extern` — centralises
    /// construction so `view_flags` is initialised in one place rather
    /// than at every struct-literal call site.
    pub(crate) fn from_parts(inner: wasmtime::Memory, store: WasmStoreHandle) -> Self {
        Self {
            inner,
            store,
            view_flags: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Issue a fresh `WasmMemoryView` aliasing this Memory's current
    /// backing per WASM JS API §5.3 "create a fixed length memory buffer"
    /// (the view's detach flag corresponds to the `[[ArrayBufferDetachKey]]`
    /// "WebAssembly.Memory" state). The returned view is live
    /// (`is_detached()` is `false`) until the next
    /// successful `grow` on this Memory (or any clone of it). The view's
    /// detach flag is tracked via a `Weak<Cell<bool>>` so the Memory →
    /// view edge is non-owning (dropping the view does not require
    /// touching `view_flags`).
    pub fn view(&self) -> WasmMemoryView {
        let flag = Rc::new(Cell::new(false));
        let mut flags = self.view_flags.borrow_mut();
        if flags.len() > VIEW_FLAGS_CLEANUP_THRESHOLD {
            flags.retain(|w| w.strong_count() > 0);
        }
        flags.push(Rc::downgrade(&flag));
        WasmMemoryView {
            inner: self.inner,
            store: self.store.clone(),
            detached: flag,
        }
    }

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
        // Drop the store borrow before walking view_flags. `WasmMemoryView`
        // does not touch the store during detach (only flips its own
        // `Cell<bool>`), so this isn't strictly required for soundness —
        // but releasing the borrow keeps the surface contract narrow.
        drop(store);
        // Flip-after-Ok: per WASM JS API §5.3 "grow the memory buffer
        // associated with" algorithm step 6 ("Refresh the memory buffer")
        // / "refresh the Memory buffer" step 5.1, buffer detach runs only
        // after a successful underlying grow. The wasmtime grow returns
        // `Err` on failure (e.g. maximum exceeded) and the `?` above
        // short-circuits — views remain live, matching spec ordering.
        let mut flags = self.view_flags.borrow_mut();
        flags.retain(|w| {
            if let Some(cell) = w.upgrade() {
                cell.set(true);
                true
            } else {
                false
            }
        });
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

/// Stable byte-view alias to a `WasmMemory`'s backing per WASM JS API
/// §5.3 Memory `[[BufferObject]]` aliasing — the engine-indep counterpart
/// to a host-side `ArrayBuffer`. The view holds a `Cell<bool>` detach
/// flag tracked weakly by the owning `WasmMemory` so a successful `grow`
/// invalidates the view per §5.3 step 5.1 `DetachArrayBuffer(buffer,
/// "WebAssembly.Memory")`.
///
/// `read` / `write` / `byte_size` short-circuit with `WasmErrorKind::Runtime`
/// once detached — D-16 maps those to the JS `TypeError` raised by
/// detached-`ArrayBuffer` access. The detach flag is host-side only
/// (the wasmtime backing is unchanged); detach is a JS-API observable
/// detail, not a Wasm-execution effect.
///
/// Methods take `&self` so a wrapper that has multiple JS aliases can
/// dispatch through any of them without ownership friction. Concurrent
/// reentrant `write` (e.g. via a nested host callback) double-borrows
/// the underlying `RefCell<Store<…>>` and panics — JS's single-thread
/// model makes this unreachable in practice but the surface is honest
/// about the failure mode.
pub struct WasmMemoryView {
    pub(crate) inner: wasmtime::Memory,
    pub(crate) store: WasmStoreHandle,
    pub(crate) detached: Rc<Cell<bool>>,
}

impl std::fmt::Debug for WasmMemoryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmMemoryView")
            .field("detached", &self.detached.get())
            .finish_non_exhaustive()
    }
}

impl WasmMemoryView {
    /// `true` once a `WasmMemory::grow` on the issuing Memory (or any
    /// clone of it) succeeded after this view was constructed.
    pub fn is_detached(&self) -> bool {
        self.detached.get()
    }

    /// Linear-memory byte length per WASM JS API §5.3 — returns `Err`
    /// once detached, matching `read` / `write`. Returned as `u32` to
    /// match `WasmMemoryDescriptor::initial` units; values exceeding
    /// `u32::MAX` (Memory64 future) surface as `Runtime`-kind error
    /// rather than silently saturating, mirroring `WasmTable::length`.
    pub fn byte_size(&self) -> Result<u32, WasmError> {
        self.ensure_attached()?;
        let store = self.store.borrow();
        u32::try_from(self.inner.data_size(&*store)).map_err(|_| {
            WasmError::new(
                WasmErrorKind::Runtime,
                "memory byte size exceeds u32::MAX".to_string(),
            )
        })
    }

    /// Read `len` bytes starting at `offset`. Returns `Err` if detached
    /// or if `offset + len` exceeds the current byte length (bounds
    /// check matches `DataView.prototype.get*` per WASM JS API
    /// §5.3-derived JS-side surface that D-16 wraps).
    pub fn read(&self, offset: u32, len: u32) -> Result<Vec<u8>, WasmError> {
        self.ensure_attached()?;
        let store = self.store.borrow();
        let data = self.inner.data(&*store);
        let start = offset as usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(Self::oob_error)?;
        if end > data.len() {
            return Err(Self::oob_error());
        }
        Ok(data[start..end].to_vec())
    }

    /// Write `src` starting at `offset`. Returns `Err` if detached or
    /// out of bounds. Uses `&self` (interior mutability via the shared
    /// `WasmStoreHandle::borrow_mut`) so JS-side wrappers that share
    /// the view through `Rc` / Boa `JsObject` can write without
    /// `&mut self` plumbing.
    pub fn write(&self, offset: u32, src: &[u8]) -> Result<(), WasmError> {
        self.ensure_attached()?;
        let mut store = self.store.borrow_mut();
        let data = self.inner.data_mut(&mut *store);
        let start = offset as usize;
        let end = start.checked_add(src.len()).ok_or_else(Self::oob_error)?;
        if end > data.len() {
            return Err(Self::oob_error());
        }
        data[start..end].copy_from_slice(src);
        Ok(())
    }

    fn ensure_attached(&self) -> Result<(), WasmError> {
        if self.is_detached() {
            return Err(WasmError::new(
                WasmErrorKind::Runtime,
                "WasmMemoryView is detached (memory was grown)".to_string(),
            ));
        }
        Ok(())
    }

    fn oob_error() -> WasmError {
        WasmError::new(
            WasmErrorKind::Runtime,
            "WasmMemoryView access out of bounds".to_string(),
        )
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
    /// Engine-indep element type per WASM JS API §5.4 — the type
    /// reported by `table_type(store, this.[[Table]])` for this Table's
    /// `[[Table]]` internal slot (§5.4 only defines `[[Table]]` — there
    /// is no `[[Type]]` slot). Fallible because
    /// `engine_conv::wasm_value_type_from_wasmtime` returns `Err` for
    /// future-proposal `HeapType` variants — propagating keeps the
    /// surface aligned with `WasmGlobal::value_type` (handle.rs nearby)
    /// and `WasmFunc::func_type` (handle.rs above).
    pub fn element_kind(&self) -> Result<WasmValueType, WasmError> {
        let store = self.store.borrow();
        let ty = self.inner.ty(&*store);
        let element = ty.element();
        engine_conv::wasm_value_type_from_wasmtime(wasmtime::ValType::Ref(element.clone()))
    }

    /// Current size in entries per WASM JS API §5.4
    /// Table.prototype.length. Fallible to match the u32-overflow
    /// classification in `WasmTable::grow` and `WasmMemory::grow` — a
    /// 64-bit table (Memory64 proposal) with entry count exceeding
    /// `u32::MAX` surfaces a `WasmError` rather than silently saturating.
    pub fn length(&self) -> Result<u32, WasmError> {
        let store = self.store.borrow();
        u32::try_from(self.inner.size(&*store)).map_err(|_| {
            WasmError::new(
                WasmErrorKind::Runtime,
                "table entry count exceeds u32::MAX".to_string(),
            )
        })
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
    assert_not_impl_any!(WasmMemoryView: Send, Sync);
    assert_not_impl_any!(WasmTable: Send, Sync);
    assert_not_impl_any!(WasmGlobal: Send, Sync);
};

#[cfg(test)]
mod view_tests {
    use crate::runtime::WasmRuntime;
    use crate::value::WasmMemoryDescriptor;

    fn alloc_memory(initial: u32, maximum: Option<u32>) -> super::WasmMemory {
        let runtime = WasmRuntime::new().unwrap();
        runtime
            .new_memory(WasmMemoryDescriptor { initial, maximum })
            .unwrap()
    }

    #[test]
    fn basic_detach_on_grow() {
        let mut mem = alloc_memory(1, Some(4));
        let view = mem.view();
        assert!(!view.is_detached());
        view.write(0, &[1, 2, 3, 4]).unwrap();
        assert_eq!(view.read(0, 4).unwrap(), vec![1, 2, 3, 4]);
        mem.grow(1).unwrap();
        assert!(view.is_detached());
        assert!(view.write(0, &[5]).is_err());
        assert!(view.read(0, 1).is_err());
        assert!(view.byte_size().is_err());
    }

    #[test]
    fn fresh_view_post_grow_is_live() {
        let mut mem = alloc_memory(1, Some(4));
        mem.grow(1).unwrap();
        let view = mem.view();
        assert!(!view.is_detached());
        // 2 pages = 128 KiB
        assert_eq!(view.byte_size().unwrap(), 2 * 64 * 1024);
        view.write(0, &[9]).unwrap();
        assert_eq!(view.read(0, 1).unwrap(), vec![9]);
    }

    #[test]
    fn chained_grow_detaches_all_pre_grow_views() {
        let mut mem = alloc_memory(1, Some(8));
        let view_a = mem.view();
        mem.grow(1).unwrap();
        let view_b = mem.view();
        mem.grow(1).unwrap();
        assert!(view_a.is_detached());
        assert!(view_b.is_detached());
    }

    #[test]
    fn clone_and_grow_detach_symmetry() {
        // Regression for the plan-stage Axis-2 CRIT: WasmMemory: Clone
        // shares the underlying wasmtime backing, so a grow on one clone
        // must detach views allocated via any other clone.
        let mut mem_a = alloc_memory(1, Some(8));
        let mem_b = mem_a.clone();
        let view_b = mem_b.view();
        assert!(!view_b.is_detached());
        mem_a.grow(1).unwrap();
        assert!(view_b.is_detached());
    }

    #[test]
    fn view_flags_opportunistic_cleanup_bounds_growth() {
        // Allocate 200 views without growing; each is dropped before the
        // next is allocated, so all Weaks dangle. After the 65th view
        // alloc the cleanup path runs and trims dead entries.
        let mem = alloc_memory(1, None);
        for _ in 0..200 {
            let _view = mem.view();
        }
        // Bound is the cleanup threshold + 1 (we push the new entry
        // after the retain).
        let len = mem.view_flags.borrow().len();
        assert!(
            len <= super::VIEW_FLAGS_CLEANUP_THRESHOLD + 1,
            "view_flags grew unbounded: {len}"
        );
    }

    #[test]
    fn grow_err_leaves_views_live() {
        // Regression for the plan-stage Stage 2.3 flip-after-Ok ordering:
        // a failed grow (maximum exceeded) must NOT detach views, per
        // WASM JS API §5.3 Memory.grow algorithm step 4 RangeError
        // short-circuit.
        let mut mem = alloc_memory(1, Some(1));
        let view = mem.view();
        let err = mem.grow(1).unwrap_err();
        assert!(
            matches!(err.kind(), crate::error::WasmErrorKind::Runtime),
            "expected Runtime kind, got {:?}",
            err.kind()
        );
        assert!(!view.is_detached(), "view must remain live after grow Err");
        view.write(0, &[42]).unwrap();
        assert_eq!(view.read(0, 1).unwrap(), vec![42]);
    }

    #[test]
    fn read_write_out_of_bounds_errors() {
        let mem = alloc_memory(1, None);
        let view = mem.view();
        let len = view.byte_size().unwrap();
        assert!(view.read(len - 1, 2).is_err());
        assert!(view.write(len - 1, &[1, 2]).is_err());
        // Overflow path on offset+len addition.
        assert!(view.read(u32::MAX, 1).is_err());
    }
}
