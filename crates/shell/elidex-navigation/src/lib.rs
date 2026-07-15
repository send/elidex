//! URL navigation, document loading, and page lifecycle for elidex.
//!
//! This crate provides:
//! - [`NavigationController`] — session history management (back/forward/go).
//! - [`TraversalQueue`] / [`DrainCoordinator`] — the session-history
//!   task-boundary phase-separation substrate (deferred traversal queue + shared
//!   drain-coordinator; `docs/plans/2026-07-session-history-task-queue-model.md`).
//! - [`load_document`] — fetch a URL, parse HTML, extract and fetch sub-resources.
//! - Resource extraction helpers for `<style>`, `<link>`, and `<script>` elements.

pub mod loader;
pub mod navigation;
pub mod resource;
pub mod traversal_queue;

pub use loader::{
    extract_inline_scripts, load_document, LoadError, LoadedDocument, ResolvedScript,
};
pub use navigation::{
    classify_navigation, HistoryEntry, NavClass, NavigationController, TraversalKind,
};
pub use resource::{ImageSource, ScriptSource, StyleSource};
pub use traversal_queue::{
    DrainCoordinator, DrainHost, DrainOutcome, PendingHistoryStep, PendingTraversal,
    TraversalApplyOutcome, TraversalDelta, TraversalQueue, UserInvolvement,
};
