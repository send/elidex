//! WebAssembly runtime for the elidex browser engine.
//!
//! Provides wasmtime-based Wasm module compilation, instantiation, and execution.
//! Wasm modules access the DOM through the same `DomHandlerRegistry` used by the
//! JS engine, making Wasm a first-class citizen alongside JavaScript.

mod error;
mod host_funcs;
mod host_state;
mod runtime;

pub use error::{WasmError, WasmErrorKind};
pub use runtime::{WasmInstance, WasmModule, WasmRuntime};
