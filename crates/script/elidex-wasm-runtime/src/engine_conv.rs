//! Engine-bridge conversion glue: wasmtime ↔ engine-indep types.
//!
//! All items are `pub(crate)`; no symbol from this file escapes the crate
//! surface. This is the tier-C "engine-bridge glue" file (see
//! `lib.rs` doc and the F1 plan-memo §4.2 file-tier table) — concentrating
//! the conversion impls here keeps `value.rs` and `imports.rs` truly
//! wasmtime-free, which is what makes the trip-wire `grep -E "wasmtime::"
//! value.rs imports.rs` → 0 hit verification meaningful.
//!
//! Direction summary:
//! - `wasm_value_type_from_wasmtime`  fallible — MVP scope accepts
//!   I32/I64/F32/F64/V128 + Ref{Func,Extern}; other `HeapType` variants
//!   (Any/Eq/I31/Struct/Array/None/NoFunc/NoExtern/Exn/NoExn/ConcreteFunc)
//!   return `Err` until proposals land additively.
//! - `wasmtime_val_type_from`  infallible (MVP types are always
//!   representable in wasmtime).
//! - `wasm_value_from_wasmtime` / `wasm_value_to_wasmtime`  Store-aware
//!   because ExternRef payload construction needs `&mut Store` (wasmtime
//!   `ExternRef::new(store, payload)`) and funcref extraction needs a
//!   `WasmStoreHandle` to attach to the freshly produced `WasmFunc`.
//! - `wasm_ref_from_wasmtime` / `wasm_ref_to_wasmtime`  same Store-aware
//!   path for the externref / funcref legs.
//! - `wasm_error_from_wasmtime(wasmtime::Error, default_kind)`  classifies
//!   wasmtime errors via Trap detection.

use wasmtime::{ExternRef, Rooted, Store, Val};

use crate::error::{WasmError, WasmErrorKind};
use crate::handle::{WasmFunc, WasmGlobal, WasmMemory, WasmStoreHandle, WasmTable};
use crate::host::state::HostState;
use crate::imports::WasmImportValue;
use crate::value::{ExternRefHandle, HeapType, RefType, WasmRef, WasmValue, WasmValueType};

// ---------------------------------------------------------------------------
// ValType ↔ WasmValueType
// ---------------------------------------------------------------------------

/// MVP scope: I32/I64/F32/F64/V128 + Ref{Func,Extern}. Anything else
/// (GC heap types, exception types, concrete func types) returns Err so
/// future proposals can land additively without breaking callers.
pub(crate) fn wasm_value_type_from_wasmtime(
    ty: wasmtime::ValType,
) -> Result<WasmValueType, WasmError> {
    match ty {
        wasmtime::ValType::I32 => Ok(WasmValueType::I32),
        wasmtime::ValType::I64 => Ok(WasmValueType::I64),
        wasmtime::ValType::F32 => Ok(WasmValueType::F32),
        wasmtime::ValType::F64 => Ok(WasmValueType::F64),
        wasmtime::ValType::V128 => Ok(WasmValueType::V128),
        wasmtime::ValType::Ref(rt) => {
            let heap = wasm_heap_type_from_wasmtime(rt.heap_type())?;
            Ok(WasmValueType::Ref(RefType {
                nullable: rt.is_nullable(),
                heap,
            }))
        }
    }
}

fn wasm_heap_type_from_wasmtime(h: &wasmtime::HeapType) -> Result<HeapType, WasmError> {
    match h {
        wasmtime::HeapType::Func => Ok(HeapType::Func),
        wasmtime::HeapType::Extern => Ok(HeapType::Extern),
        other => Err(WasmError::new(
            WasmErrorKind::Runtime,
            format!("unsupported HeapType variant: {other:?}"),
        )),
    }
}

#[allow(dead_code)] // Consumed by Stage 7 (instance.rs export-type build) and the boa migration in Stage 10.
pub(crate) fn wasmtime_val_type_from(ty: WasmValueType) -> wasmtime::ValType {
    match ty {
        WasmValueType::I32 => wasmtime::ValType::I32,
        WasmValueType::I64 => wasmtime::ValType::I64,
        WasmValueType::F32 => wasmtime::ValType::F32,
        WasmValueType::F64 => wasmtime::ValType::F64,
        WasmValueType::V128 => wasmtime::ValType::V128,
        WasmValueType::Ref(rt) => wasmtime::ValType::Ref(wasmtime::RefType::new(
            rt.nullable,
            wasmtime_heap_type_from(rt.heap),
        )),
    }
}

#[allow(dead_code)] // Called transitively by `wasmtime_val_type_from`; flagged on its own because the call site is currently dead.
fn wasmtime_heap_type_from(h: HeapType) -> wasmtime::HeapType {
    match h {
        HeapType::Func => wasmtime::HeapType::Func,
        HeapType::Extern => wasmtime::HeapType::Extern,
    }
}

// ---------------------------------------------------------------------------
// Val ↔ WasmValue
// ---------------------------------------------------------------------------

/// Reads a wasmtime `Val` and returns the engine-indep `WasmValue`. The
/// `store` parameter is the (locked) wasmtime store backing the value;
/// `store_handle` is a clone-source for any fresh `WasmFunc` / handle
/// attached to the result (so the new handle shares the same
/// `Arc<Mutex<Store>>`).
pub(crate) fn wasm_value_from_wasmtime(
    v: Val,
    store: &Store<HostState>,
    store_handle: &WasmStoreHandle,
) -> WasmValue {
    match v {
        Val::I32(x) => WasmValue::I32(x),
        Val::I64(x) => WasmValue::I64(x),
        Val::F32(bits) => WasmValue::F32(f32::from_bits(bits)),
        Val::F64(bits) => WasmValue::F64(f64::from_bits(bits)),
        Val::V128(b) => WasmValue::V128(b.as_u128().to_le_bytes()),
        // Func null + future proposals (anyref / exnref / contref) — all
        // surface as a Null(Func) fallback for now; proposal-landing PRs
        // will add explicit `HeapType::Any` / `Exn` variants and route
        // them here.
        Val::FuncRef(None) | Val::AnyRef(_) | Val::ExnRef(_) | Val::ContRef(_) => {
            WasmValue::Ref(WasmRef::Null(HeapType::Func))
        }
        Val::FuncRef(Some(f)) => WasmValue::Ref(WasmRef::Func(WasmFunc {
            inner: f,
            store: store_handle.clone(),
        })),
        Val::ExternRef(None) => WasmValue::Ref(WasmRef::Null(HeapType::Extern)),
        Val::ExternRef(Some(rooted)) => {
            let handle =
                extern_ref_from_wasmtime(&rooted, store).unwrap_or_else(|| ExternRefHandle::new(0));
            WasmValue::Ref(WasmRef::Extern(handle))
        }
    }
}

pub(crate) fn wasm_value_to_wasmtime(
    v: WasmValue,
    store: &mut Store<HostState>,
) -> Result<Val, WasmError> {
    match v {
        WasmValue::I32(x) => Ok(Val::I32(x)),
        WasmValue::I64(x) => Ok(Val::I64(x)),
        WasmValue::F32(x) => Ok(Val::F32(x.to_bits())),
        WasmValue::F64(x) => Ok(Val::F64(x.to_bits())),
        WasmValue::V128(b) => Ok(Val::V128(u128::from_le_bytes(b).into())),
        WasmValue::Ref(r) => match r {
            WasmRef::Null(HeapType::Func) => Ok(Val::FuncRef(None)),
            WasmRef::Null(HeapType::Extern) => Ok(Val::ExternRef(None)),
            WasmRef::Func(f) => Ok(Val::FuncRef(Some(f.inner))),
            WasmRef::Extern(h) => {
                let rooted = extern_ref_to_wasmtime(h, store)?;
                Ok(Val::ExternRef(Some(rooted)))
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Ref ↔ WasmRef
// ---------------------------------------------------------------------------

pub(crate) fn wasm_ref_from_wasmtime(
    r: &wasmtime::Ref,
    store: &Store<HostState>,
    store_handle: &WasmStoreHandle,
) -> WasmRef {
    match r {
        // GC / exception proposals not yet plumbed — fall back to null
        // funcref. Real variants land additively when those proposals
        // are wired through `HeapType`.
        wasmtime::Ref::Func(None) | wasmtime::Ref::Any(_) | wasmtime::Ref::Exn(_) => {
            WasmRef::Null(HeapType::Func)
        }
        wasmtime::Ref::Func(Some(f)) => WasmRef::Func(WasmFunc {
            inner: *f,
            store: store_handle.clone(),
        }),
        wasmtime::Ref::Extern(None) => WasmRef::Null(HeapType::Extern),
        wasmtime::Ref::Extern(Some(rooted)) => {
            let handle =
                extern_ref_from_wasmtime(rooted, store).unwrap_or_else(|| ExternRefHandle::new(0));
            WasmRef::Extern(handle)
        }
    }
}

pub(crate) fn wasm_ref_to_wasmtime(
    r: WasmRef,
    store: &mut Store<HostState>,
) -> Result<wasmtime::Ref, WasmError> {
    match r {
        WasmRef::Null(HeapType::Func) => Ok(wasmtime::Ref::Func(None)),
        WasmRef::Null(HeapType::Extern) => Ok(wasmtime::Ref::Extern(None)),
        WasmRef::Func(f) => Ok(wasmtime::Ref::Func(Some(f.inner))),
        WasmRef::Extern(h) => {
            let rooted = extern_ref_to_wasmtime(h, store)?;
            Ok(wasmtime::Ref::Extern(Some(rooted)))
        }
    }
}

// ---------------------------------------------------------------------------
// ExternRef (Store-aware payload-preserving helpers)
// ---------------------------------------------------------------------------

/// Write path: wrap the host-issued `u64` payload in a wasmtime
/// `Rooted<ExternRef>`. The Rooted handle is GC-managed by the store; the
/// payload itself is opaque to wasmtime.
pub(crate) fn extern_ref_to_wasmtime(
    handle: ExternRefHandle,
    store: &mut Store<HostState>,
) -> Result<Rooted<ExternRef>, WasmError> {
    ExternRef::new(&mut *store, handle.payload())
        .map_err(|e| wasm_error_from_wasmtime(e, WasmErrorKind::Runtime))
}

/// Read path: extract the `u64` payload from a `Rooted<ExternRef>`.
/// Returns `None` if the externref's payload is not a `u64` — that
/// indicates the externref was produced outside the elidex host (host
/// bypass / malformed input), and the call-site uses safe degradation
/// rather than a panic.
pub(crate) fn extern_ref_from_wasmtime(
    rooted: &Rooted<ExternRef>,
    store: &Store<HostState>,
) -> Option<ExternRefHandle> {
    let data = rooted.data(store).ok()??;
    data.downcast_ref::<u64>()
        .copied()
        .map(ExternRefHandle::new)
}

// ---------------------------------------------------------------------------
// Import / Export Extern conversions
// ---------------------------------------------------------------------------

/// Convert an engine-indep `WasmImportValue` into the wasmtime `Extern`
/// used by `Linker::define`. The function / memory / table / global
/// inner handle is moved out — call-sites must clone the value first
/// if they need to keep it. Per WASM JS API §5.2 Instance ctor step 4
/// ("Read the imports").
#[allow(dead_code)] // Consumed by Stage 8 `WasmRuntime::instantiate` rewrite.
pub(crate) fn import_value_to_extern(value: WasmImportValue) -> wasmtime::Extern {
    match value {
        WasmImportValue::Func(f) => wasmtime::Extern::Func(f.inner),
        WasmImportValue::Memory(m) => wasmtime::Extern::Memory(m.inner),
        WasmImportValue::Table(t) => wasmtime::Extern::Table(t.inner),
        WasmImportValue::Global(g) => wasmtime::Extern::Global(g.inner),
    }
}

/// Convert a wasmtime `Extern` into the engine-indep
/// `instance::WasmExportItem`, attaching `store_handle` so produced
/// handles share the store. Returns `None` for variants outside the
/// MVP scope (`Tag` requires Exception Handling host machinery;
/// `SharedMemory` requires the Threads proposal). Stage 7 export
/// iteration skips such entries.
pub(crate) fn export_item_from_wasmtime_extern(
    e: &wasmtime::Extern,
    store_handle: &WasmStoreHandle,
) -> Option<crate::instance::WasmExportItem> {
    use crate::instance::WasmExportItem;
    match e {
        wasmtime::Extern::Func(f) => Some(WasmExportItem::Func(WasmFunc {
            inner: *f,
            store: store_handle.clone(),
        })),
        wasmtime::Extern::Memory(m) => Some(WasmExportItem::Memory(WasmMemory {
            inner: *m,
            store: store_handle.clone(),
        })),
        wasmtime::Extern::Table(t) => Some(WasmExportItem::Table(WasmTable {
            inner: *t,
            store: store_handle.clone(),
        })),
        wasmtime::Extern::Global(g) => Some(WasmExportItem::Global(WasmGlobal {
            inner: *g,
            store: store_handle.clone(),
        })),
        wasmtime::Extern::Tag(_) | wasmtime::Extern::SharedMemory(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Error classification
// ---------------------------------------------------------------------------

/// Maps wasmtime `Error` → `WasmError` with Trap detection promoting to
/// `WasmErrorKind::Runtime` per WASM JS API §5.10 + §7.1 / §7.2 trap
/// mapping.
///
/// Heuristic: if the error chain contains a `wasmtime::Trap`, the
/// classification is `Runtime` regardless of the supplied default —
/// traps are always runtime failures. Otherwise the call-site's
/// `default_kind` is used (compile / link / runtime). The owned
/// `wasmtime::Error` is preserved in `WasmError::source` for chain
/// inspection by the host (D-16 surfaces it as a JS error cause).
pub(crate) fn wasm_error_from_wasmtime(
    err: wasmtime::Error,
    default_kind: WasmErrorKind,
) -> WasmError {
    let kind = if err.downcast_ref::<wasmtime::Trap>().is_some() {
        WasmErrorKind::Runtime
    } else {
        default_kind
    };
    WasmError::with_source(kind, err)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn empty_store() -> Store<HostState> {
        use elidex_dom_api::registry::{create_cssom_registry, create_dom_registry};
        let dom = Arc::new(create_dom_registry());
        let cssom = Arc::new(create_cssom_registry());
        let engine = wasmtime::Engine::default();
        Store::new(&engine, HostState::new(dom, cssom))
    }

    #[test]
    fn val_type_round_trip_numeric() {
        for (src, expected) in [
            (wasmtime::ValType::I32, WasmValueType::I32),
            (wasmtime::ValType::I64, WasmValueType::I64),
            (wasmtime::ValType::F32, WasmValueType::F32),
            (wasmtime::ValType::F64, WasmValueType::F64),
            (wasmtime::ValType::V128, WasmValueType::V128),
        ] {
            let mid = wasm_value_type_from_wasmtime(src).unwrap();
            assert_eq!(mid, expected);
            let back = wasmtime_val_type_from(mid);
            let again = wasm_value_type_from_wasmtime(back).unwrap();
            assert_eq!(mid, again);
        }
    }

    #[test]
    fn val_type_round_trip_ref_func() {
        let src = wasmtime::ValType::Ref(wasmtime::RefType::new(true, wasmtime::HeapType::Func));
        let mid = wasm_value_type_from_wasmtime(src).unwrap();
        assert_eq!(
            mid,
            WasmValueType::Ref(RefType {
                nullable: true,
                heap: HeapType::Func,
            })
        );
        let back = wasmtime_val_type_from(mid);
        let again = wasm_value_type_from_wasmtime(back).unwrap();
        assert_eq!(mid, again);
    }

    #[test]
    fn val_type_round_trip_ref_extern_nonnull() {
        let src = wasmtime::ValType::Ref(wasmtime::RefType::new(false, wasmtime::HeapType::Extern));
        let mid = wasm_value_type_from_wasmtime(src).unwrap();
        assert_eq!(
            mid,
            WasmValueType::Ref(RefType {
                nullable: false,
                heap: HeapType::Extern,
            })
        );
        let back = wasmtime_val_type_from(mid);
        let again = wasm_value_type_from_wasmtime(back).unwrap();
        assert_eq!(mid, again);
    }

    #[test]
    fn val_type_unsupported_heap_type_returns_err() {
        let src = wasmtime::ValType::Ref(wasmtime::RefType::new(true, wasmtime::HeapType::Any));
        let err = wasm_value_type_from_wasmtime(src).unwrap_err();
        assert!(matches!(err.kind, WasmErrorKind::Runtime));
        assert!(err.message().contains("unsupported HeapType"));
    }

    #[test]
    fn extern_ref_handle_round_trip_via_store() {
        let mut store = empty_store();
        let h = ExternRefHandle::new(42);
        let rooted = extern_ref_to_wasmtime(h, &mut store).unwrap();
        let h2 = extern_ref_from_wasmtime(&rooted, &store).unwrap();
        assert_eq!(h.payload(), h2.payload());
    }

    #[test]
    fn extern_ref_zero_payload_round_trip() {
        let mut store = empty_store();
        let h = ExternRefHandle::new(0);
        let rooted = extern_ref_to_wasmtime(h, &mut store).unwrap();
        let h2 = extern_ref_from_wasmtime(&rooted, &store).unwrap();
        assert_eq!(h2.payload(), 0);
    }

    #[test]
    fn extern_ref_non_u64_payload_returns_none() {
        let mut store = empty_store();
        let rooted = ExternRef::new(&mut store, String::from("not-a-u64")).unwrap();
        let result = extern_ref_from_wasmtime(&rooted, &store);
        assert!(
            result.is_none(),
            "non-u64 ExternRef payload must yield None"
        );
    }

    #[test]
    fn wasm_value_to_wasmtime_f32_preserves_bits() {
        let mut store = empty_store();
        let v = WasmValue::F32(3.5_f32);
        let out = wasm_value_to_wasmtime(v, &mut store).unwrap();
        match out {
            Val::F32(bits) => assert_eq!(f32::from_bits(bits), 3.5_f32),
            _ => panic!("expected F32"),
        }
    }

    #[test]
    fn wasm_value_v128_round_trip_bytes() {
        let mut store = empty_store();
        let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let out = wasm_value_to_wasmtime(WasmValue::V128(bytes), &mut store).unwrap();
        match out {
            Val::V128(v) => {
                assert_eq!(v.as_u128().to_le_bytes(), bytes);
            }
            _ => panic!("expected V128"),
        }
    }
}
