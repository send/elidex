//! Navigation back-channel intent types — the engine↔shell contract.
//!
//! A script engine bound to one document cannot navigate itself: navigation
//! replaces the whole pipeline (network + parse + render), which the shell
//! owns. So `location.*` / `history.*` globals do not navigate directly — they
//! record an **intent** (a [`NavigationRequest`] or a [`HistoryAction`]) that
//! the shell drains after the script turn and applies to its single
//! session-history source of truth (the shell's `NavigationController`).
//!
//! These two types are the wire format of that channel, shared by every engine
//! (boa, the elidex-js VM) and the shell. They live in this engine-agnostic
//! seam crate — alongside [`ScriptEngine`](crate::ScriptEngine) /
//! [`DispatchEvent`](crate::DispatchEvent) — rather than in the shell's
//! `elidex-navigation` (which also carries the loader + the `NavigationController`
//! implementation), so a `crates/script/` engine never depends on a
//! `crates/shell/` crate just to produce the contract.

/// A pending navigation request from `location.assign()` / `location.href = …`
/// / `location.replace()` / `location.reload()` (WHATWG HTML §7.4.2.2
/// "Beginning navigation"). The shell runs the navigate algorithm and commits
/// the new URL back into the engine after the load.
#[derive(Clone, Debug)]
pub struct NavigationRequest {
    /// The target URL string (page-supplied; the shell resolves + validates it).
    pub url: String,
    /// `true` for `location.replace()` / `location.reload()` (replace the
    /// current session-history entry rather than pushing a new one).
    pub replace: bool,
}

/// A pending history action from the `History` interface (WHATWG HTML §7.2.5).
///
/// `Back` / `Forward` / `Go` are session-history *traversals* (§7.4.6 "Applying
/// the history step" — an async document load the shell owns); `PushState` /
/// `ReplaceState` are the §7.2.5 "shared history push/replace state steps" the
/// shell persists into its `NavigationController` (the engine has already run
/// the synchronous URL-and-history-update half, §7.4.4).
#[derive(Clone, Debug)]
pub enum HistoryAction {
    /// `history.back()`
    Back,
    /// `history.forward()`
    Forward,
    /// `history.go(delta)`
    Go(i32),
    /// `history.pushState(state, title, url?)`
    PushState {
        /// Optional URL to push (already resolved against the document URL).
        url: Option<String>,
        /// Title (ignored per §7.2.5 — `unused` — but accepted for API compat).
        title: String,
    },
    /// `history.replaceState(state, title, url?)`
    ReplaceState {
        /// Optional URL to replace the current entry with.
        url: Option<String>,
        /// Title (ignored per §7.2.5 — `unused` — but accepted for API compat).
        title: String,
    },
}
