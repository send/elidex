//! URL navigation, document loading, and page lifecycle for elidex.
//!
//! This crate provides:
//! - [`NavigationController`] — session history management (back/forward/go).
//! - [`load_document`] — fetch a URL, parse HTML, extract and fetch sub-resources.
//! - Resource extraction helpers for `<style>`, `<link>`, and `<script>` elements.

pub mod loader;
pub mod navigation;
pub mod resource;

pub use loader::{load_document, LoadError, LoadedDocument, ResolvedScript};
pub use navigation::{HistoryAction, HistoryEntry, NavigationController, NavigationRequest};
pub use resource::{ImageSource, ScriptSource, StyleSource};
