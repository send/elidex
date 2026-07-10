//! Cross-context host-effect intent types — the engine↔shell drain contract.
//!
//! A script engine bound to one document cannot reach another browsing
//! context or the OS window: cross-context effects — a `localStorage` write
//! that must fire `storage` events on *other* same-origin documents, an
//! `indexedDB.open()` upgrade that must fire `versionchange` on other tabs'
//! open connections, a `window.focus()` that must focus the containing OS
//! window, an iframe document's `postMessage` that must reach its parent —
//! are recorded as **intents** the shell drains after each engine turn (the
//! [`HostDriver`](crate::HostDriver) drain group) and routes through its own
//! IPC / window machinery.  The same enqueue-then-drain model as the
//! [`navigation`](crate::NavigationRequest) back-channel and
//! [`WindowOpenIntent`](crate::WindowOpenIntent).
//!
//! These types are the wire format of that channel, shared by every engine
//! and the shell.  They live in this engine-agnostic seam crate — alongside
//! [`ScriptEngine`](crate::ScriptEngine) / the navigation intents — so a
//! `crates/script/` engine never depends on a `crates/shell/` crate just to
//! produce the contract.

/// A pending `localStorage` mutation broadcast (WHATWG HTML §12.2.1 The
/// Storage interface — `setItem` step 7 / `removeItem` step 5 / `clear`
/// step 3, "Broadcast this with …").
///
/// The engine enqueues one of these only when the mutation actually changed
/// the storage map: `setItem` returns before broadcasting when the old value
/// equals the new one (§12.2.1 setItem step 3.2 "If oldValue is value, then
/// return"), `removeItem` of an absent key returns at step 1, and `clear` of
/// an empty map returns at step 1.  The originating document never receives
/// its own event (§12.2.1 *broadcast a Storage object* step 3 excludes the
/// originating storage) — enforced structurally by the shell's broadcast
/// topology, which only fans out to OTHER contexts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageChange {
    /// The storage-BUCKET origin string that keys the mutated `localStorage`
    /// area — the shell's broadcast targeting key (only same-bucket contexts
    /// receive the event, §12.2.1 *broadcast a Storage object* step 3's
    /// same-origin clause).
    ///
    /// This is the engine's bucket key (a serialized tuple origin, or the
    /// per-VM opaque-origin **sentinel** string for sandboxed / `about:blank`
    /// / `data:` documents), NOT a re-derivable `origin().serialize()`:
    /// serialization collapses every opaque document to `"null"`, which would
    /// alias unrelated sandboxed iframes' broadcasts across the exact
    /// isolation boundary the sentinel bucket (S5-4b) establishes.  Only the
    /// enqueue site has the bucket key in hand, so it rides the payload.
    pub origin: String,
    /// The key that changed; `None` for `clear()` (the StorageEvent `key`
    /// member is null).
    pub key: Option<String>,
    /// The previous value; `None` when the key was newly created, or for
    /// `clear()`.
    pub old_value: Option<String>,
    /// The new value; `None` when the key was removed, or for `clear()`.
    pub new_value: Option<String>,
    /// The serialization of the originating document's URL (§12.2.1
    /// *broadcast a Storage object* step 2) — the StorageEvent `url` member.
    pub url: String,
}

/// A pending cross-context IndexedDB version-change request (IndexedDB-3
/// §4.2 Event interfaces, dfn *fire a version change event*).
///
/// An `indexedDB.open()` naming a version higher than the database's current
/// one requires every OTHER context's open connection to the same database
/// to receive a `versionchange` event (and close) before the upgrade can
/// proceed.  The opening engine enqueues this request; the shell drains it,
/// broadcasts to the other same-origin contexts, and each receiving engine
/// fires the event on its own open connections via
/// [`HostDriver::deliver_idb_versionchange`](crate::HostDriver::deliver_idb_versionchange)
/// — the receive half of the same §4.2 wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdbVersionChangeRequest {
    /// The database name.
    pub db_name: String,
    /// The database's current version at the time of the request.
    pub old_version: u64,
    /// The requested new version; `None` corresponds to a database-deletion
    /// version change (the `IDBVersionChangeEvent.newVersion = null` case).
    pub new_version: Option<u64>,
}

/// A pending iframe→parent `window.postMessage` (WHATWG HTML §9.3.3 Posting
/// messages, the `postMessage(message, options)` method —
/// `#dom-window-postmessage-options`).
///
/// An iframe document's engine cannot deliver to its parent window itself
/// (the parent lives in another engine / thread), and it cannot evaluate the
/// `targetOrigin` gate either — §9.3.3's origin check compares against the
/// **target** (parent) window's origin, which only the receiving side knows.
/// So the message carries `target_origin` verbatim and the gate is applied
/// at delivery.
///
/// **boa-parity interim**: routing by iframe depth (top-level → VM-internal
/// self-delivery; iframe → this parent-directed queue) mirrors boa's
/// context-routed single queue, including its `ToString`-serialized `data`
/// wire format.  The real `WindowProxy` browsing-context targeting model
/// replaces this wholesale at S5-8 / B1
/// (`#11-browsing-context-model-window-open-postmessage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParentMessage {
    /// The message payload, `ToString`-serialized (the boa-parity interim
    /// wire format; structured serialization rides the S5-8/B1 model).
    pub data: String,
    /// The `targetOrigin` argument verbatim (`"*"`, `"/"`, or a URL string —
    /// already syntax-validated at the call site per §9.3.3).  The receiving
    /// side applies the origin gate against the target window's origin.
    pub target_origin: String,
}
