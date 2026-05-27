//! Engine-bound internal: `HostState` raw-ptr bind/unbind lifecycle + DOM host
//! function registration. All items here are `pub(crate)` — external consumers
//! interact only through the engine-indep crate surface (`runtime`, `instance`,
//! etc.). This follows the CLAUDE.md Layering mandate "VM host/ は engine-bound
//! 責務のみ" applied to the wasm engine bridge.

pub(crate) mod funcs;
pub(crate) mod state;
