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
//! use elidex_js::{JsRuntime, extract_scripts};
//!
//! let scripts = extract_scripts(&dom, document);
//! let mut runtime = JsRuntime::new(document);
//! for script in &scripts {
//!     runtime.eval(&script.source, &mut session, &mut dom, document);
//! }
//! ```

pub mod bridge;
pub mod error_conv;
pub mod globals;
pub mod runtime;
pub mod script_extract;
pub mod timer_queue;
pub mod value_conv;

pub use runtime::{EvalResult, JsRuntime};
pub use script_extract::{extract_scripts, ScriptEntry};
