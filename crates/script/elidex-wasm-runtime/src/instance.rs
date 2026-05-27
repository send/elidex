//! Live module instance + host-binding dispatch.
//!
//! `WasmInstance` wraps a wasmtime `Instance` plus a shared `WasmStoreHandle`
//! that all its exported handles (`WasmFunc` / `WasmMemory` / ...) share.
//! `call_func` brackets the dispatch in a `host::state::UnbindGuard` so the
//! raw-pointer lifecycle inside `HostState` is panic-safe.
//!
//! Spec anchors:
//! - WASM JS API §5.2 Instance ctor (instance is the output of
//!   `WasmRuntime::instantiate`)
//! - WASM JS API §5.6 Exported Functions invocation (`call_func`)
//! - WASM JS API §5.2 `initialize an Instance object` step 3
//!   (`instanceObject.[[Exports]]` set to the exportsObject). The IDL
//!   for `Instance.exports` has no `[SameObject]` attribute; stable
//!   wrapper identity is an elidex implementation choice (the
//!   wrapper-cache layer in D-16), not a spec mandate.

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

use crate::engine_conv;
use crate::error::{WasmError, WasmErrorKind};
use crate::handle::{WasmFunc, WasmGlobal, WasmMemory, WasmStoreHandle, WasmTable};
use crate::host::state::UnbindGuard;
use crate::runtime::DEFAULT_FUEL;
use crate::value::WasmValue;

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

/// Per-call host binding — bundles the session, the DOM world, and the
/// document root so `call_func` can attach the live references to
/// `HostState` for the duration of the wasm call. Per CLAUDE.md
/// "Layering mandate": this is the minimal engine-bridge surface the
/// VM host needs to invoke wasm; mutations/algorithms still flow
/// through `DomHandlerRegistry`.
pub struct ScriptHostBinding<'a> {
    pub session: &'a mut SessionCore,
    pub dom: &'a mut EcsDom,
    pub document: Entity,
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

    /// Invoke an exported function. Per WASM JS API §5.6 Exported
    /// Functions invocation. `args` must be arity- and type-matched to
    /// `func.func_type()` — the host (D-16) coerces JS arguments to
    /// `WasmValue` before calling, so by the time we reach here the
    /// values are well-typed.
    ///
    /// The dispatch is wrapped in `UnbindGuard` so that
    /// `HostState::bind` and the matching `unbind` are paired on
    /// every exit path: `Ok`, `Err`, or panic. Without this the raw
    /// pointer fields in `HostState` could outlive the live
    /// `SessionCore` / `EcsDom` references and produce undefined
    /// behaviour from host-fn callbacks that arrive after the
    /// borrow ends.
    pub fn call_func(
        &self,
        func: &WasmFunc,
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
        // and every subsequent call_func — even trivial ones — traps
        // with out-of-fuel. Per-call reset bounds runaway risk to a
        // single call rather than the instance lifetime.
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
        // `result_count_with_store` takes the locked store directly so
        // we don't `RefCell::borrow` while already holding `borrow_mut`.
        let result_count = func.result_count_with_store(guard.store());
        let mut results_buf = vec![wasmtime::Val::null_func_ref(); result_count];

        func.inner
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
