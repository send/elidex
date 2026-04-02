//! Bytecode IR for the elidex-js engine (Stage 2).
//!
//! Defines a stack-based bytecode instruction set and compilation units
//! for ES2020+ JavaScript. The bytecode is produced by the compiler
//! (see [`crate::compiler`]) and consumed by the interpreter (M4-10).

pub mod compiled;
pub mod disasm;
pub mod opcode;
pub mod source_map;

pub use compiled::{CompiledFunction, CompiledScript, Constant, ExceptionHandler, UpvalueDesc};
pub use opcode::Op;
pub use source_map::SourceMap;
