//! WebAssembly runtime for the elidex browser engine.
//!
//! Provides wasmtime-based Wasm module compilation, instantiation, and execution.
//! Wasm modules access the DOM through the same `DomHandlerRegistry` used by the
//! JS engine, making Wasm a first-class citizen alongside JavaScript.

mod engine_conv;
mod error;
mod handle;
mod host;
mod imports;
mod module;
mod runtime;
mod value;

pub use error::{WasmError, WasmErrorKind};
pub use handle::{WasmFunc, WasmGlobal, WasmMemory, WasmTable};
pub use imports::{ImportObject, WasmImportValue};
pub use module::{ImportExportKind, ModuleExportDescriptor, ModuleImportDescriptor, WasmModule};
pub use runtime::{WasmInstance, WasmRuntime};
pub use value::{
    ExternRefHandle, GrowResult, HeapType, RefType, WasmFuncType, WasmRef, WasmValue,
    WasmValueType,
};
