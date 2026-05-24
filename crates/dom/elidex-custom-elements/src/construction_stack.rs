//! Per-definition construction stack (\[C2\] WHATWG HTML §4.13.3
//! `custom element definition` record: "construction stack — a list,
//! initially empty, manipulated by upgrade and HTML element
//! constructors").
//!
//! Populated by the CE upgrade algorithm (\[C4\] §4.13.5 step 6: push)
//! and drained by the `HTMLElement` constructor (\[C1\] §3.2.3 step 15:
//! replace top entry with the already-constructed marker; step 13:
//! re-entrant construct against an already-constructed marker throws
//! `TypeError`).
//!
//! Stored as a private field on [`crate::CustomElementDefinition`];
//! callers reach it via the registry-level accessors
//! (`peek_construction_stack` / `push_construction_stack` /
//! `replace_construction_stack_top_with_marker`).

use elidex_ecs::Entity;

/// One entry on a custom element definition's construction stack.
///
/// - [`Self::Element`] — an in-construction entity pushed by the
///   upgrade algorithm (\[C4\] step 6).
/// - [`Self::AlreadyConstructed`] — the sentinel that replaces an
///   `Element` entry once the `HTMLElement` constructor has consumed
///   it (\[C1\] step 15). A second construct against the same slot
///   sees this marker and throws `TypeError` (\[C1\] step 13).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstructionStackEntry {
    /// Entity currently being constructed.
    Element(Entity),
    /// Marker left behind after the HTMLElement constructor consumed
    /// the original element entry. Re-entrant construct → TypeError.
    AlreadyConstructed,
}
