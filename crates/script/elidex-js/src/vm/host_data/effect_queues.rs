//! Cross-context effect queues for [`HostData`] (S5-6a).
//!
//! Transient FIFO intents the shell drains per turn via the `HostDriver`
//! drain group — a cohesive group carved out of `host_data.rs` so the
//! effect-queue state ([`HostEffectQueues`]) and its enqueue/take behaviour
//! live in one bounded module.  Standing counterpart to the
//! `pending_history` / `pending_window_open` event queues (B1-neutral).
//!
//! **Not** cleared on `Vm::unbind`: like the navigation back-channel, the
//! shell drains these after the batch bracket closes, so they must survive
//! it.

use super::engine_feature::HostData;

/// The S5-6a cross-context effect queues held on [`HostData`].
///
/// All four are drain-once FIFO intents:
/// - `storage_changes` — `localStorage` mutation broadcasts (WHATWG HTML
///   §12.2.1 "Broadcast this…"), enqueued change-gated by the
///   `vm/host/storage.rs` mutation paths.  `compat-webapi`-gated with the
///   rest of the Web Storage family.
/// - `idb_versionchange_requests` — cross-context IndexedDB version-change
///   requests (IndexedDB-3 §4.2, dfn *fire a version change event*),
///   enqueued by the `indexedDB.open()` upgrade branch.
/// - `window_focus` — `window.focus()` request flag (WHATWG HTML §6.6.6,
///   `#dom-window-focus`), set by the `window.focus()` native.
/// - `parent_messages` — iframe→parent `postMessage` intents (WHATWG HTML
///   §9.3.3), enqueued by the `postMessage` native when `iframe_depth > 0`
///   (boa-parity context routing — see
///   [`elidex_script_session::ParentMessage`]).
#[derive(Default)]
pub(super) struct HostEffectQueues {
    /// See [`HostEffectQueues`] — `localStorage` broadcasts.
    #[cfg(feature = "compat-webapi")]
    storage_changes: Vec<elidex_script_session::StorageChange>,
    /// See [`HostEffectQueues`] — IndexedDB version-change requests.
    idb_versionchange_requests: Vec<elidex_script_session::IdbVersionChangeRequest>,
    /// See [`HostEffectQueues`] — `window.focus()` request flag.
    window_focus: bool,
    /// See [`HostEffectQueues`] — iframe→parent `postMessage` intents.
    parent_messages: Vec<elidex_script_session::ParentMessage>,
}

// -------------------------------------------------------------------------
// S5-6a: cross-context effect queues — enqueue (VM natives) + take (the
// `HostDriver` drain group).  All FIFO, drain-once.  These stay on
// `HostData` (matching the `mutation_observers` registry idiom — grouped
// field + methods on the owner) so the `HostDriver` drain surface and every
// enqueue call site are unchanged by the carve.
// -------------------------------------------------------------------------
impl HostData {
    /// Enqueue a `localStorage` mutation broadcast (WHATWG HTML §12.2.1
    /// "Broadcast this…").  The caller (the `vm/host/storage.rs`
    /// mutation paths) has already applied the §12.2.1 change gates —
    /// same-value `setItem`, absent-key `removeItem`, and empty-map
    /// `clear` never reach here.
    #[cfg(feature = "compat-webapi")]
    pub(crate) fn enqueue_storage_change(&mut self, change: elidex_script_session::StorageChange) {
        self.effect_queues.storage_changes.push(change);
    }

    /// Drain the pending `localStorage` broadcasts in mutation order
    /// (the `HostDriver::take_pending_storage_changes` body).
    #[cfg(feature = "compat-webapi")]
    pub fn take_pending_storage_changes(&mut self) -> Vec<elidex_script_session::StorageChange> {
        std::mem::take(&mut self.effect_queues.storage_changes)
    }

    /// Enqueue a cross-context IndexedDB version-change request
    /// (IndexedDB-3 §4.2, dfn *fire a version change event*) — called by
    /// the `indexedDB.open()` upgrade branch.
    pub(crate) fn enqueue_idb_versionchange_request(
        &mut self,
        request: elidex_script_session::IdbVersionChangeRequest,
    ) {
        self.effect_queues.idb_versionchange_requests.push(request);
    }

    /// Drain the pending IndexedDB version-change requests in request
    /// order (the `HostDriver::take_pending_idb_versionchange_requests`
    /// body).
    pub fn take_pending_idb_versionchange_requests(
        &mut self,
    ) -> Vec<elidex_script_session::IdbVersionChangeRequest> {
        std::mem::take(&mut self.effect_queues.idb_versionchange_requests)
    }

    /// Stage a `window.focus()` request (WHATWG HTML §6.6.6,
    /// `#dom-window-focus`) — called by the `window.focus()` native.
    pub(crate) fn request_window_focus(&mut self) {
        self.effect_queues.window_focus = true;
    }

    /// Take (and clear) the pending `window.focus()` request — `true` at
    /// most once per staged request (the `HostDriver::take_pending_focus`
    /// body).
    pub fn take_pending_focus(&mut self) -> bool {
        std::mem::take(&mut self.effect_queues.window_focus)
    }

    /// Enqueue an iframe→parent `postMessage` intent (WHATWG HTML
    /// §9.3.3) — called by the `postMessage` native when
    /// `iframe_depth > 0`.
    pub(crate) fn enqueue_parent_message(&mut self, message: elidex_script_session::ParentMessage) {
        self.effect_queues.parent_messages.push(message);
    }

    /// Drain the pending iframe→parent messages in call order (the
    /// `HostDriver::take_pending_parent_messages` body).
    pub fn take_pending_parent_messages(&mut self) -> Vec<elidex_script_session::ParentMessage> {
        std::mem::take(&mut self.effect_queues.parent_messages)
    }
}
