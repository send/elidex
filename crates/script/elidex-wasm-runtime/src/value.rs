//! Engine-independent value types for the WebAssembly bridge.
//!
//! Pure data — no wasmtime knowledge lives in this file (per the crate's
//! tier-A engine-indep semantic layer). Conversion glue to and from
//! `wasmtime::ValType` / `wasmtime::Val` is concentrated in
//! `engine_conv.rs` (tier C).
//!
//! Spec anchors:
//! - WebAssembly JS API §5.5 ValueType (numeric variants + reference union)
//! - WebAssembly JS API §5.4 TableKind (`HeapType` projection used for tables)
//! - WebAssembly JS API §5.3 Memory `grow` algorithm (`GrowResult`)
//! - WebAssembly JS API §5.6 Exported Functions (`WasmValue` / `WasmRef`
//!   argument/result types, `externref` opaque payload)
//! - WebAssembly Core Spec §2.3.5 Reference Types (typed null
//!   `(ref null T)` vs `(ref T)`; the JS API enum flattens this, but the
//!   distinction is preserved on the Rust side via `RefType.nullable` and
//!   `WasmRef::Null(HeapType)`)

use crate::handle::WasmFunc;

/// Structured value type per WASM JS API §5.5 (numeric + `Ref`).
///
/// `Ref(RefType)` carries `(nullable, heap)`, capturing the typed-null
/// distinction from WebAssembly Core Spec §2.3.5 Reference Types — the JS
/// API `ValueType` enum flattens to `funcref` / `externref`, but the
/// structured form lets Exception Handling / GC / Function References
/// proposals land as additive `HeapType` variants without breaking the
/// `WasmValueType` shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    Ref(RefType),
}

/// Reference type per WebAssembly Core Spec §2.3.5: a `HeapType` plus a
/// nullability bit (`(ref null T)` vs `(ref T)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefType {
    pub nullable: bool,
    pub heap: HeapType,
}

/// Heap reference kind. MVP scope is `Func` / `Extern`; future proposals
/// (Exception Handling: `Exn` / `NoExn`; GC: `Any` / `Eq` / `I31` /
/// `Struct` / `Array` / `None` / `NoFunc` / `NoExtern`; Function
/// References: `ConcreteFunc(u32)`) extend additively. The
/// `#[non_exhaustive]` attribute forces downstream `match` arms to keep a
/// `_` fallback, so adding variants is a semver-compatible change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeapType {
    Func,
    Extern,
}

/// Value carried into / out of a Wasm call. Numeric variants own their
/// payload; `Ref` holds a typed reference value.
#[derive(Clone, Debug)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
    Ref(WasmRef),
}

/// Reference value with explicit typed-null encoding. `Null(HeapType)`
/// distinguishes a null funcref from a null externref per WebAssembly
/// Core Spec §2.3.5 (typed null `(ref null T)`); future typed-null
/// proposals (Function References' `(ref null $T)`) fit the same enum.
#[derive(Clone, Debug)]
pub enum WasmRef {
    Null(HeapType),
    Func(WasmFunc),
    Extern(ExternRefHandle),
}

/// Opaque externref payload issued by the host. The Wasm engine treats
/// the inner `u64` as an identifier with no host-specific meaning —
/// per WASM JS API §5.6 the externref value is opaque to the engine.
/// The host (D-16) maps this back to a `JsValue` via a side-table; the
/// engine is the system of record for liveness via wasmtime's
/// `Rooted<ExternRef>` GC root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternRefHandle(u64);

impl ExternRefHandle {
    pub fn new(payload: u64) -> Self {
        Self(payload)
    }

    pub fn payload(self) -> u64 {
        self.0
    }
}

/// Function signature per WASM JS API §5.6 Exported Functions / §5.1 Module
/// import-descriptor `kind == "function"` — `(params, results)` pair of
/// engine-indep value types. Used by `handle::WasmFunc::func_type()` and by
/// `instance::WasmInstance::call_func` to size the result buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmFuncType {
    pub params: Vec<WasmValueType>,
    pub results: Vec<WasmValueType>,
}

/// Outcome of a `WasmMemory::grow` call per WASM JS API §5.3
/// Memory.prototype.grow algorithm.
///
/// `buffer_handle_invalidated` signals that any host-issued
/// `ArrayBuffer` aliasing the previous backing store must be detached
/// and re-allocated. Per spec §5.3 the buffer is unconditionally
/// replaced on every successful grow, so `WasmMemory::grow` always
/// sets this field to `true`. It is kept as a struct field (rather
/// than removed) for future extensions where additional signals may
/// be packed in (e.g. resize-vs-replace once the resizable-buffer
/// proposal lands).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrowResult {
    pub pre_pages: u32,
    pub buffer_handle_invalidated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_type_variants_construct() {
        let nullable_func = RefType {
            nullable: true,
            heap: HeapType::Func,
        };
        let nonnull_extern = RefType {
            nullable: false,
            heap: HeapType::Extern,
        };
        assert_eq!(nullable_func.heap, HeapType::Func);
        assert!(nullable_func.nullable);
        assert_eq!(nonnull_extern.heap, HeapType::Extern);
        assert!(!nonnull_extern.nullable);
    }

    #[test]
    fn wasm_value_type_ref_carries_ref_type() {
        let t = WasmValueType::Ref(RefType {
            nullable: true,
            heap: HeapType::Extern,
        });
        match t {
            WasmValueType::Ref(rt) => {
                assert!(rt.nullable);
                assert_eq!(rt.heap, HeapType::Extern);
            }
            _ => panic!("expected Ref"),
        }
    }

    #[test]
    fn extern_ref_handle_round_trip_identity() {
        let h = ExternRefHandle::new(42);
        assert_eq!(h.payload(), 42);
        let h0 = ExternRefHandle::new(0);
        assert_eq!(h0.payload(), 0);
        let h_max = ExternRefHandle::new(u64::MAX);
        assert_eq!(h_max.payload(), u64::MAX);
    }

    #[test]
    fn wasm_ref_null_carries_heap_type() {
        let nf = WasmRef::Null(HeapType::Func);
        let ne = WasmRef::Null(HeapType::Extern);
        match nf {
            WasmRef::Null(HeapType::Func) => {}
            _ => panic!("expected Null(Func)"),
        }
        match ne {
            WasmRef::Null(HeapType::Extern) => {}
            _ => panic!("expected Null(Extern)"),
        }
    }

    #[test]
    fn grow_result_fields_round_trip() {
        let g = GrowResult {
            pre_pages: 7,
            buffer_handle_invalidated: true,
        };
        assert_eq!(g.pre_pages, 7);
        assert!(g.buffer_handle_invalidated);
    }
}
