//! WebAssembly runtime for the elidex browser engine.
//!
//! Provides wasmtime-based Wasm module compilation, instantiation, and execution.
//! Wasm modules access the DOM through the same `DomHandlerRegistry` used by the
//! JS engine, making Wasm a first-class citizen alongside JavaScript.

mod engine_conv;
mod error;
mod handle;
mod host;
mod runtime;
mod value;

pub use error::{WasmError, WasmErrorKind};
pub use handle::{WasmFunc, WasmGlobal, WasmMemory, WasmTable};
pub use runtime::{WasmInstance, WasmModule, WasmRuntime};
pub use value::{
    ExternRefHandle, GrowResult, HeapType, RefType, WasmFuncType, WasmRef, WasmValue,
    WasmValueType,
};
