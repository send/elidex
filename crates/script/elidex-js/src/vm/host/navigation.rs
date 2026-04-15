//! Navigation state — `Location` / `History` / `document.URL` / reload.
//!
//! The VM owns a single [`NavigationState`] per `Vm`, read and written
//! by the `location` / `history` host globals (PR4b C6 / C7).  Until the
//! shell integration lands (PR6), the state is purely VM-internal:
//! assignments to `location.href` and `history.pushState` update the
//! current URL and the history entry list in place, but do not trigger
//! an actual browser navigation.  `history.back()` / `history.forward()`
//! move within the in-memory stack.
//!
//! WHATWG HTML §7.4 "The History interface" uses a structured clone
//! for `state`, but at this tier we store it as a bare [`JsValue`] so
//! the getter round-trip is identity — structured-clone serialisation
//! lands alongside the shell navigation bridge.

#![cfg(feature = "engine")]
// C3 introduces the state struct.  Its consumers (location setter,
// history.pushState, document.URL getter) land in C6 / C7 / C9 — the
// fields are read exclusively by those later commits.
#![allow(dead_code)]

use super::super::value::JsValue;

/// A single entry in [`NavigationState::history_entries`] (WHATWG HTML
/// §7.4.1 "session history entry").
#[derive(Clone, Debug)]
pub(crate) struct HistoryEntry {
    /// The URL the entry points at.  Updated in place when
    /// `history.replaceState(…, url)` is called.
    pub(crate) url: String,
    /// The serialised state associated with the entry (`history.state`).
    ///
    /// Held as a bare [`JsValue`] — structured clone happens when the
    /// shell navigation bridge is wired (PR6).  GC-roots this entry's
    /// values are traced because `NavigationState` lives inside
    /// `VmInner` and is iterated by the GC root set (see `gc.rs`
    /// `NavigationState` visit when wired).
    pub(crate) state: JsValue,
}

/// Per-`Vm` navigation state.
///
/// Phase 2 scope — the fields are read by the `location` / `history`
/// host objects and written by their setters.  Shell navigation
/// coupling (actual URL loads, popstate firing) is deferred to PR6 per
/// the PR4b plan.
#[derive(Debug)]
pub(crate) struct NavigationState {
    /// The current browsing-context URL.  Backs `location.href`,
    /// `document.URL`, and `document.documentURI`.  Initialised to
    /// `"about:blank"` per WHATWG HTML §7.3.3 "Creating documents".
    pub(crate) current_url: String,
    /// The in-memory session history stack.
    pub(crate) history_entries: Vec<HistoryEntry>,
    /// The index of the current entry within [`Self::history_entries`].
    /// Always a valid index (invariant: `history_entries` is non-empty
    /// after construction).
    pub(crate) history_index: usize,
}

impl NavigationState {
    /// Create a fresh navigation state pointing at `about:blank`.
    pub(crate) fn new() -> Self {
        let initial_url = String::from("about:blank");
        Self {
            current_url: initial_url.clone(),
            history_entries: vec![HistoryEntry {
                url: initial_url,
                state: JsValue::Null,
            }],
            history_index: 0,
        }
    }
}
