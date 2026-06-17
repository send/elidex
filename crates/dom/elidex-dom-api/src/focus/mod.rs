//! Focus state + focusable-area helpers (WHATWG HTML §6.6).
//!
//! The engine-independent home for focus as a DOM concept, so the shell's
//! UA-input path and the JS VM's `HTMLElement.focus()`/`blur()` drive focus
//! through one source of truth — the canonical [`ElementState::FOCUS`]
//! component — rather than parallel `Option<Entity>` side-stores. Three
//! responsibilities:
//!
//! - **focusable area** ([`tab_index_default_for`] / [`is_focusable`], WHATWG
//!   HTML §6.6.2 Data model / §6.6.3 The tabindex attribute) — the per-element
//!   default tab index and whether an element can receive focus (incl. the
//!   §6.6.2 *connectedness* requirement, the write-side gate of the invariant).
//! - **the READ model** ([`current_focus`]) — the single query for the focused
//!   element; its connectedness walk is a *defensive guard* (the bit is
//!   connected by construction: gated at focus, cleared at removal).
//! - **the WRITE model** ([`set_focus_bit`]) — clear-all-then-set, so the
//!   single-focus invariant holds *by construction* across every writer (no
//!   "previously focused" record to keep in sync).
//!
//! The `FOCUS`-set ⟹ connected invariant is maintained by [`is_focusable`]
//! (rejects disconnected `focus()` targets) and `EcsDom::fire_after_remove`
//! (clears the bit when its holder leaves the tree, WHATWG HTML §2.1.4 removing
//! steps — silently). So focus needs **one** read model: there is no by-identity
//! second read.
//!
//! Engine- and form-independent: this crate has no `elidex-form` dependency, so
//! the focusable predicate is attribute-based. Event dispatch (the focusing
//! steps §6.6.4 fire `focusout`/`focusin`/`blur`/`focus`) is engine-bound and
//! stays with the caller; these helpers only reconcile the `FOCUS` bit.
//!
//! ## Module layout
//!
//! - `predicate` — the §6.6.2/§6.6.3 focusable-area predicates
//!   ([`is_focusable`] / [`tab_index_default_for`] / [`parse_tab_index_value`]).
//! - `sot` — the focus source-of-truth: the [`ElementState::FOCUS`] bit's
//!   read ([`current_focus`]) / write ([`set_focus_bit`] / [`blur`]) models, the
//!   active-document membership test ([`is_in_document`]), and the asynchronous
//!   focusability fixup ([`reconcile_focus`]).
//! - `delegate` — §6.6.4 "get the focusable area" / "focus delegate" (the
//!   shadow-`delegatesFocus` retarget, PR-A1).
//! - `update_steps` — the canonical §6.6.4 transition ([`focusing_steps`] /
//!   [`unfocusing_steps`] + the [`FocusEventSink`] seam), PR-A2a.

use elidex_ecs::{EcsDom, ElementState, Entity};

mod delegate;
mod predicate;
mod sot;
mod update_steps;

pub use delegate::*;
pub use predicate::*;
pub use sot::*;
pub use update_steps::*;

#[cfg(test)]
mod tests;
