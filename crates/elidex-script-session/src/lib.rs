//! Script session abstraction bridging JS engines and the ECS DOM.
//!
//! This crate provides the unified boundary between script engines
//! (`SpiderMonkey`, V8, etc.) and the elidex ECS-based DOM. It includes:
//!
//! - **`SessionCore`** — Mutation buffer and identity mapping coordinator
//! - **`IdentityMap`** — Bidirectional mapping between ECS entities and JS object references
//! - **`Mutation`** — Buffered DOM mutation operations applied on flush
//! - **`DomApiHandler`** / **`CssomApiHandler`** — Traits for DOM/CSSOM method dispatch

mod cssom_api;
mod dom_api;
mod identity_map;
mod mutation;
mod session;
mod types;

pub use cssom_api::CssomApiHandler;
pub use dom_api::DomApiHandler;
pub use identity_map::IdentityMap;
pub use mutation::{apply_mutation, Mutation, MutationKind, MutationRecord};
pub use session::SessionCore;
pub use types::{ComponentKind, DomApiError, DomApiErrorKind, JsObjectRef};
