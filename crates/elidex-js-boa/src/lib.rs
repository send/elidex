// JS numbers are f64; entity IDs and timer IDs are u64. Casting between
// them is inherent to the bridge and safe for the small values used in Phase 2.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

//! JavaScript engine integration (boa) and DOM bindings for elidex.
//!
//! This crate wraps `boa_engine` to provide JavaScript execution within the
//! elidex browser engine. It registers DOM globals (`document`, `window`,
//! `console`, timers) and bridges JS calls to the ECS DOM via `HostBridge`.
//!
//! # Usage
//!
//! ```ignore
//! use elidex_js_boa::{JsRuntime, extract_scripts};
//!
//! let scripts = extract_scripts(&dom, document);
//! let mut runtime = JsRuntime::new(document);
//! for script in &scripts {
//!     runtime.eval(&script.source, &mut session, &mut dom, document);
//! }
//! ```

/// Implement no-op `boa_gc::Trace` and `boa_gc::Finalize` for types that
/// contain no GC-managed objects (e.g. `Rc`-wrapped Rust data).
macro_rules! impl_empty_trace {
    ($ty:ty) => {
        #[allow(unsafe_code)]
        unsafe impl boa_gc::Trace for $ty {
            boa_gc::custom_trace!(this, mark, {
                let _ = this;
            });
        }
        impl boa_gc::Finalize for $ty {
            fn finalize(&self) {}
        }
    };
}

mod bridge;
mod error_conv;
mod globals;
pub mod runtime;
pub mod script_extract;
mod timer_queue;
mod value_conv;

pub use runtime::{EvalResult, JsRuntime};
pub use script_extract::{extract_scripts, ScriptEntry};
