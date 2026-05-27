//! WebAssembly runtime for the elidex browser engine — engine-bridge
//! crate that encapsulates wasmtime behind an engine-independent public
//! surface. Wasm modules access the DOM through the same
//! `DomHandlerRegistry` used by the JS engine, making Wasm a first-class
//! citizen alongside JavaScript.
//!
//! ## File tiers (see plan-memo `m4-12-pr-wasm-runtime-engine-indep-completion-plan.md`)
//!
//! | Tier | Files | Wasmtime token | Notes |
//! |---|---|---|---|
//! | A. Engine-indep semantic | `value.rs`, `imports.rs` | none | pure data, wasmtime-free |
//! | B. Engine-bridge structural | `module.rs`, `instance.rs`, `handle.rs` | `pub(crate) inner: wasmtime::*` only | public methods return engine-indep types |
//! | C. Engine-bridge glue | `engine_conv.rs`, `runtime.rs` | free use (pub(crate)) | wasmtime ↔ engine-indep conversion home |
//! | D. Engine-bound internal | `host/state.rs`, `host/funcs.rs` | required | bind/unbind, host fn registration; all pub(crate) |
//! | E. Documented exception | `error.rs` | `pub source: Option<wasmtime::Error>` + `pub fn source_err` | error chain inspection at engine-bridge layer |
//!
//! Trip-wires for the tier discipline live in
//! `tools/wasm-runtime-trip-wire-verify.sh` at the workspace root.

mod engine_conv;
mod error;
mod handle;
mod host;
mod imports;
mod instance;
mod module;
mod runtime;
mod value;

pub use error::{WasmError, WasmErrorKind};
pub use handle::{WasmFunc, WasmGlobal, WasmMemory, WasmTable};
pub use imports::{ImportObject, WasmImportValue};
pub use instance::{ScriptHostBinding, WasmExportItem, WasmInstance};
pub use module::{ImportExportKind, ModuleExportDescriptor, ModuleImportDescriptor, WasmModule};
pub use runtime::WasmRuntime;
pub use value::{
    ExternRefHandle, GrowResult, HeapType, RefType, WasmFuncType, WasmRef, WasmValue,
    WasmValueType,
};
