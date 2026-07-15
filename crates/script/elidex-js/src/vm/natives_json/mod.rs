//! `JSON.stringify` and `JSON.parse` (ECMA-262 §25.5).
//!
//! Split along the module's natural seam — the serializer and the parser share
//! nothing but the depth cap below — so neither half carries the other's weight
//! (CLAUDE.md 1000-line touch-time split; Codex R7). The re-exports keep every
//! call site's path unchanged.

mod parse;
mod stringify;

pub(in crate::vm) use parse::{native_json_parse, parse_json_str};
pub(in crate::vm) use stringify::{
    native_json_stringify, stringify_for_structured_shortcut, stringify_to_string,
};

/// Maximum nesting depth for `JSON.stringify` / `JSON.parse` recursion.
/// Prevents Rust-stack exhaustion from attacker-crafted deep nesting
/// (`"[[[[[...]]]]]"` etc.) — release builds match V8's ~1000 limit.
/// Debug builds drop to 800: each `serialize_object` frame is larger
/// without optimizer inlining, so 1000-deep inputs can outgrow the
/// thread's stack budget; the
/// `tests_string_complement::json_stringify_depth_cap` 2000-deep
/// regression still trips the cap and surfaces a RangeError under
/// either limit.
#[cfg(debug_assertions)]
const MAX_JSON_DEPTH: usize = 800;
#[cfg(not(debug_assertions))]
const MAX_JSON_DEPTH: usize = 1000;
