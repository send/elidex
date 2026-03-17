//! Legacy DOM API definitions.
//!
//! Documents the compat behavior for legacy DOM APIs. The actual stubs
//! are implemented directly in elidex-js (`globals/document.rs`).
//!
//! # TODO(M4-3.8)
//!
//! - `document.write` full implementation (re-entrant self-built parser)
//!
//! # Phase 4 TODO
//!
//! - `document.all` as `HTMLAllCollection` (callable + typeof === "undefined")
//! - `document.images`, `document.forms`, `document.links` live collections
