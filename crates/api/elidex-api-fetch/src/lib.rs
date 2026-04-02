//! Engine-independent Fetch API protocol types and utilities.
//!
//! This crate provides the WHATWG Fetch spec logic that is shared across
//! script engine bindings (boa, future elidex-js bytecode VM, etc.).
//! It does NOT depend on any JS engine crate.

mod headers;
mod response;

pub use headers::{
    is_valid_header_name, is_valid_header_value, HeaderGuard, FORBIDDEN_REQUEST_HEADERS,
};
pub use response::{status_text_for, ResponseParts, REDIRECT_STATUSES};
