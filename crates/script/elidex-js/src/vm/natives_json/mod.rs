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

/// Maximum nesting depth shared by the three recursive JSON walkers:
/// `JSON.parse`'s parser (`parse_array` / `parse_object`), `JSON.stringify`'s
/// serializer, and the reviver walk `internalize` (a reviver can install
/// cyclic or arbitrarily deep graphs after the parser's own cap has passed).
/// Exceeding it is a catchable `RangeError`, never a Rust-stack-overflow abort.
///
/// Release matches V8's ~1000 and was verified to hold for all three walkers
/// (depth 990 passes, 1000+ throws the RangeError, no overflow).
///
/// Debug builds use 600. The binding constraint is `internalize`: measured
/// 2026-11-05 on a 2 MiB thread (the smallest stack any production VM thread
/// gets — every VM thread in elidex-shell `app/tab.rs`, `app/sw_coordinator.rs`,
/// `content/iframe/load.rs`, `content/mod.rs` and the worker spawn in
/// `elidex-api-workers` is a plain `std::thread::spawn`, with no `stack_size`
/// and no `RUST_MIN_STACK`; the event-loop main thread runs no content VM
/// except the test-only inline `build_pipeline` path), its debug frames use
/// about 2.7 KiB/level: 760 levels pass, 770 overflow. `stringify` passed
/// 790 and the parser 800 on the same thread. 600 = 760 less >= 20%
/// (608), rounded down to a multiple of 50. The `parse_reviver_*` cells in
/// `tests_json.rs` (cap - 10 must pass) ratchet this against frame growth.
/// An iterative `internalize` would remove the constraint (not done here).
#[cfg(debug_assertions)]
pub(in crate::vm) const MAX_JSON_DEPTH: usize = 600;
#[cfg(not(debug_assertions))]
pub(in crate::vm) const MAX_JSON_DEPTH: usize = 1000;
