//! `NavigationController` — session history management.
//!
//! Implements a linear history stack with back/forward/go navigation,
//! push, and replace operations. Mirrors the browser's session history
//! for a single browsing context.

/// Scroll restoration mode — the session-history-entry field (WHATWG HTML §7.4.1.1
/// `#she-scroll-restoration-mode`); the `ScrollRestoration` WebIDL enum is §7.2.5.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollRestorationMode {
    #[default]
    Auto,
    Manual,
}

/// A single entry in the session history (WHATWG HTML §7.4.1).
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    /// The URL for this history entry.
    pub url: url::Url,
    /// The document title (may be empty).
    pub title: String,
    /// Unique key for this entry (Navigation API). Survives replaceState.
    pub navigation_api_key: String,
    /// Unique ID for this entry (Navigation API). Changes on replaceState.
    pub navigation_api_id: String,
    /// Scroll restoration mode.
    pub scroll_restoration: ScrollRestorationMode,
    /// Saved scroll position (x, y) for scroll restoration.
    pub scroll_position: Option<(f64, f64)>,
    /// `StructuredSerializeForStorage(state)` bytes from pushState/replaceState
    /// (WHATWG HTML §7.2.5 step 3), restored on traversal via
    /// `StructuredDeserialize`. `Vec<u8>` (not `String`) is the single serialized
    /// representation shared with the `HistoryAction`/`HistoryStepEvents`
    /// `SerializedState` wire — so the JSON-shortcut interim → full
    /// `StructuredSerializeForStorage` upgrade (`#11-history-state-structured-serialize-fidelity`)
    /// swaps the serializer body, not this field type. `None` = the entry has no
    /// classic state (a plain navigation, a fragment nav, or a boa-`None` push).
    pub classic_history_api_state: Option<Vec<u8>>,
    /// Navigation API state (WHATWG HTML §7.4.1, initially undefined).
    pub navigation_api_state: Option<String>,
    /// Opaque identity of the **document** this entry belongs to (a monotonic-id
    /// proxy for the session-history-entry *document* field, WHATWG HTML §7.4.1.1).
    /// Two entries are the **same document** ⇔ equal `document_sequence`: a
    /// cross-document navigation / `location.replace()` / reload allocates a fresh
    /// id; `pushState` / `replaceState` / a fragment navigation **inherit** the
    /// current document's id. A traversal is same-document (restore + fire popstate
    /// in place, no rebuild) iff the target entry's `document_sequence` equals the
    /// current entry's (WHATWG HTML §7.4.6.1 *apply the history step* step 14.10 —
    /// "targetEntry's document is displayedDocument"), NOT a URL comparison.
    pub document_sequence: u64,
}

// `NavigationRequest` + `HistoryAction` — the engine↔shell navigation intent
// types — moved to the engine-agnostic seam `elidex_script_session` (so a
// `crates/script/` engine produces the contract without depending on this
// `crates/shell/` crate). This crate owns the session-history *implementation*
// (`NavigationController` / `HistoryEntry`), not the wire intents.

/// Maximum number of history entries before oldest entries are evicted.
const MAX_HISTORY_ENTRIES: usize = 50;

/// Session history controller for a single browsing context.
///
/// Manages a linear stack of [`HistoryEntry`] values with a current index.
#[derive(Debug)]
pub struct NavigationController {
    entries: Vec<HistoryEntry>,
    /// Current position in the history. `None` means no page loaded.
    index: Option<usize>,
    /// Monotonic counter for generating unique entry keys/IDs.
    next_entry_id: u64,
    /// Monotonic counter for allocating document identities
    /// ([`HistoryEntry::document_sequence`]).
    next_document_sequence: u64,
}

impl NavigationController {
    /// Create a new empty navigation controller.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: None,
            next_entry_id: 1,
            next_document_sequence: 1,
        }
    }

    /// Generate a unique opaque string for navigation API key/ID.
    fn next_id(&mut self) -> String {
        let id = self.next_entry_id;
        self.next_entry_id += 1;
        format!("{id:016x}")
    }

    /// Allocate a fresh [`HistoryEntry::document_sequence`] (a new-document event).
    fn next_document_sequence(&mut self) -> u64 {
        let seq = self.next_document_sequence;
        self.next_document_sequence += 1;
        seq
    }

    /// The `document_sequence` of the current entry, or `None` when no page is
    /// loaded — the id a [`push_same_document`](Self::push_same_document) /
    /// [`replace_same_document`](Self::replace_same_document) inherits.
    fn current_document_sequence(&self) -> Option<u64> {
        self.index.map(|i| self.entries[i].document_sequence)
    }

    /// Append a new entry at `url` carrying `document_sequence`, discarding forward
    /// entries and evicting over the cap. The two public variants differ only in
    /// the document identity they stamp (see [`push`](Self::push) /
    /// [`push_same_document`](Self::push_same_document)).
    fn push_entry(&mut self, url: url::Url, document_sequence: u64) {
        // Truncate forward entries.
        let new_index = self.index.map_or(0, |i| i + 1);
        self.entries.truncate(new_index);

        let key = self.next_id();
        let id = self.next_id();
        self.entries.push(HistoryEntry {
            url,
            title: String::new(),
            navigation_api_key: key,
            navigation_api_id: id,
            scroll_restoration: ScrollRestorationMode::default(),
            scroll_position: None,
            classic_history_api_state: None,
            navigation_api_state: None,
            document_sequence,
        });
        self.index = Some(new_index);

        // Evict oldest entries if over the cap.
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            let excess = self.entries.len() - MAX_HISTORY_ENTRIES;
            self.entries.drain(..excess);
            // Adjust index to account for removed entries.
            self.index = self.index.map(|i| i - excess);
        }
    }

    /// Push a new **cross-document** entry (a fresh navigation / initial load),
    /// discarding any forward entries and allocating a NEW `document_sequence` — so
    /// a later traversal to it (or from it) is classified cross-document (rebuild).
    pub fn push(&mut self, url: url::Url) {
        let seq = self.next_document_sequence();
        self.push_entry(url, seq);
    }

    /// Push a new **same-document** entry (`pushState` / a fresh fragment
    /// navigation), inheriting the current entry's `document_sequence` — so a
    /// traversal between it and its document-siblings is same-document (restore +
    /// fire in place, no rebuild). With no current entry, allocates a new document
    /// (there is nothing to be "same-document" with).
    pub fn push_same_document(&mut self, url: url::Url) {
        let seq = self
            .current_document_sequence()
            .unwrap_or_else(|| self.next_document_sequence());
        self.push_entry(url, seq);
    }

    /// Replace the current entry's URL in place with a **NEW document**
    /// (`location.replace()` — a cross-document replace, reached only after the
    /// same-document early-return). Allocates a fresh `document_sequence`. With no
    /// entries, behaves like [`push`](Self::push).
    /// The `navigation_api_key` is preserved (spec: key survives replace);
    /// `navigation_api_id` gets a new value.
    pub fn replace(&mut self, url: url::Url) {
        if let Some(idx) = self.index {
            let new_id = self.next_id();
            let seq = self.next_document_sequence();
            self.entries[idx].url = url;
            self.entries[idx].navigation_api_id = new_id;
            self.entries[idx].document_sequence = seq;
            // A NEW document replaces the entry in place, so it carries NONE of the
            // replaced entry's classic `history.state` or persisted scroll — else a
            // later reload/traversal of the replacement would seed/restore stale
            // state from the prior (e.g. `pushState`'d) document.
            self.entries[idx].classic_history_api_state = None;
            self.entries[idx].scroll_position = None;
        } else {
            self.push(url);
        }
    }

    /// Replace the current entry's URL in place within the **CURRENT document**
    /// (`replaceState` / an equal-URL fragment replace) — keeps the
    /// `document_sequence`. With no entries, behaves like
    /// [`push_same_document`](Self::push_same_document).
    pub fn replace_same_document(&mut self, url: url::Url) {
        if let Some(idx) = self.index {
            let new_id = self.next_id();
            self.entries[idx].url = url;
            self.entries[idx].navigation_api_id = new_id;
            // document_sequence intentionally unchanged (same document).
        } else {
            self.push_same_document(url);
        }
    }

    /// Re-stamp the current entry with a fresh `document_sequence` — a reload
    /// replaces the navigable's *document* (`isSameDocument=false`) without moving
    /// the cursor or creating an entry, so a neighbor entry that shared the
    /// pre-reload id must NOT mis-classify same-document against the reloaded entry
    /// on a later traversal (WHATWG HTML §7.4.6.1 — reload populates a new
    /// Document). No-op when no page is loaded.
    pub fn restamp_current_document(&mut self) {
        if let Some(idx) = self.index {
            let seq = self.next_document_sequence();
            self.entries[idx].document_sequence = seq;
        }
    }

    /// Peek the back-traversal target WITHOUT moving the cursor: the
    /// `(index, URL)` a [`go_back`](Self::go_back) WOULD land on, or `None` at
    /// the start of history. Paired with [`commit_index`](Self::commit_index) for
    /// an **atomic traversal** — the shell peeks the target, loads it, and
    /// commits the cursor ONLY on a successful load, so a failed traversal never
    /// leaves the cursor speculatively moved (this retires the eager-move +
    /// rollback pattern a capture/restore cursor would need).
    pub fn peek_back(&self) -> Option<(usize, &url::Url)> {
        let target = self.index.filter(|&i| i > 0)? - 1;
        Some((target, &self.entries[target].url))
    }

    /// Peek the forward-traversal target WITHOUT moving the cursor (see
    /// [`peek_back`](Self::peek_back)); `None` at the end of history.
    pub fn peek_forward(&self) -> Option<(usize, &url::Url)> {
        let target = self.index? + 1;
        (target < self.entries.len()).then(|| (target, &self.entries[target].url))
    }

    /// Peek the `go(delta)` target WITHOUT moving the cursor (see
    /// [`peek_back`](Self::peek_back)); `None` if the delta resolves out of
    /// range. `delta == 0` resolves to the current entry (a reload).
    pub fn peek_go(&self, delta: i32) -> Option<(usize, &url::Url)> {
        let current = self.index?;
        let abs = delta.unsigned_abs() as usize;
        let target = if delta >= 0 {
            current.checked_add(abs)?
        } else {
            current.checked_sub(abs)?
        };
        (target < self.entries.len()).then(|| (target, &self.entries[target].url))
    }

    /// Commit the cursor to a peeked target index (from
    /// [`peek_back`](Self::peek_back) / [`peek_forward`](Self::peek_forward) /
    /// [`peek_go`](Self::peek_go)) after its load succeeded — the second half of
    /// an atomic traversal. A peek only returns in-range targets, so `index` is
    /// always a valid entry position; the `debug_assert` pins that peek-then-commit
    /// invariant (a violation would mean `entries` was mutated between peek and
    /// commit — the reentrant-drain case deferred to `#11-session-history-task-queue-model`).
    pub fn commit_index(&mut self, index: usize) {
        debug_assert!(
            index < self.entries.len(),
            "commit_index: peeked target {index} out of range (entries.len() = {}) — \
             entries mutated between peek and commit",
            self.entries.len()
        );
        self.index = Some(index);
    }

    /// Navigate back one step. Returns the new current URL, or `None`
    /// if already at the beginning. Eager convenience over
    /// [`peek_back`](Self::peek_back) + [`commit_index`](Self::commit_index) for
    /// the chrome-button path (which always commits); the atomic JS-history drain
    /// uses peek-then-commit directly.
    pub fn go_back(&mut self) -> Option<&url::Url> {
        let target = self.peek_back().map(|(i, _)| i)?;
        self.commit_index(target);
        self.current_url()
    }

    /// Navigate forward one step. Returns the new current URL, or `None`
    /// if already at the end. Eager convenience over
    /// [`peek_forward`](Self::peek_forward) + [`commit_index`](Self::commit_index).
    pub fn go_forward(&mut self) -> Option<&url::Url> {
        let target = self.peek_forward().map(|(i, _)| i)?;
        self.commit_index(target);
        self.current_url()
    }

    /// Navigate by a relative offset. Positive = forward, negative = back.
    /// Returns the new current URL, or `None` if the offset is out of range.
    /// Eager convenience over [`peek_go`](Self::peek_go) +
    /// [`commit_index`](Self::commit_index).
    ///
    /// Note: `go(0)` returns the current URL. The shell re-fetches the same
    /// URL, effectively reloading the page (matching browser `history.go(0)`
    /// semantics).
    pub fn go(&mut self, delta: i32) -> Option<&url::Url> {
        let target = self.peek_go(delta).map(|(i, _)| i)?;
        self.commit_index(target);
        self.current_url()
    }

    /// Returns the URL of the current entry, or `None` if no page is loaded.
    pub fn current_url(&self) -> Option<&url::Url> {
        self.index.map(|i| &self.entries[i].url)
    }

    /// The current entry's serialized `history.state`, cloned — the value a
    /// **reload** re-seeds into the rebuilt document (WHATWG HTML §7.4.6.1: a
    /// reload restores the entry's classic state). `None` when no page is loaded or
    /// the current entry carries no state.
    #[must_use]
    pub fn current_serialized_state(&self) -> Option<Vec<u8>> {
        self.index
            .and_then(|i| self.entries[i].classic_history_api_state.clone())
    }

    /// The current entry's persisted scroll offset `(x, y)` — the value a **reload**
    /// restores into the rebuilt document (scrollRestoration `auto`; the sibling of
    /// a traversal's `entry(target).scroll_position`). `None` when no page is loaded
    /// or nothing was captured.
    #[must_use]
    pub fn current_scroll_position(&self) -> Option<(f64, f64)> {
        self.index.and_then(|i| self.entries[i].scroll_position)
    }

    /// Classify a traversal to the peeked `target_index` by **document identity**
    /// (WHATWG HTML §7.4.6.1 *apply the history step* — same-document ⇔ the target
    /// entry's document is the current document, step 14.10), returning either the
    /// same-document restore payload (serialized `history.state` from step 6.3 +
    /// persisted scroll) or [`TraversalKind::Rebuild`]. Reads the **peeked target**
    /// entry (`entries[target_index]`), never the current entry — the cursor has
    /// not committed yet under peek-then-commit (both shells share this one
    /// engine-independent decision, so the classification is not re-derived per
    /// driver, and — critically — it is NOT a URL comparison: `pushState` routing
    /// gives same-document entries different URLs, and a fresh nav to an existing
    /// URL gives a different document the same URL).
    ///
    /// - `target_index == self.index` (a `go(0)`) → [`TraversalKind::Rebuild`]:
    ///   WHATWG HTML History.go step 4 — "If delta is 0, then reload … and return"
    ///   — a reload, NOT a same-document no-op.
    /// - target entry's `document_sequence` == current entry's (a different entry
    ///   in the same document) → [`TraversalKind::SameDocument`] with its restore
    ///   payload.
    /// - otherwise (a different document, or an out-of-range target — peek only
    ///   returns in-range, so unreachable in practice) → [`TraversalKind::Rebuild`].
    #[must_use]
    pub fn resolve_traversal(&self, target_index: usize) -> TraversalKind {
        // go(0) = reload (History.go step 4), never a same-document no-op.
        if self.index == Some(target_index) {
            return TraversalKind::Rebuild;
        }
        let same_document = self
            .current_document_sequence()
            .zip(self.entries.get(target_index))
            .is_some_and(|(current_seq, target)| target.document_sequence == current_seq);
        match self.entries.get(target_index) {
            Some(entry) if same_document => TraversalKind::SameDocument {
                state: entry.classic_history_api_state.clone(),
                scroll: entry.scroll_position,
            },
            _ => TraversalKind::Rebuild,
        }
    }

    /// Returns `true` if there is a previous entry to navigate to.
    pub fn can_go_back(&self) -> bool {
        self.index.is_some_and(|i| i > 0)
    }

    /// Returns `true` if there is a next entry to navigate to.
    pub fn can_go_forward(&self) -> bool {
        self.index.is_some_and(|i| i + 1 < self.entries.len())
    }

    /// Returns the number of entries in the history.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Update the title of the current entry.
    pub fn set_current_title(&mut self, title: String) {
        if let Some(idx) = self.index {
            // Invariant: `index` always points to a valid entry.
            self.entries[idx].title = title;
        }
    }

    /// Returns the title of the current entry, or `None` if no page is loaded.
    pub fn current_title(&self) -> Option<&str> {
        self.index
            .and_then(|i| self.entries.get(i))
            .map(|e| e.title.as_str())
    }

    /// Store the `StructuredSerializeForStorage(state)` bytes on the **current**
    /// entry (WHATWG HTML §7.4.4 "URL and history update steps" step 3 —
    /// `newEntry`'s serialized state). Called by the shell's pushState/replaceState
    /// drain immediately after [`push`](Self::push)/[`replace`](Self::replace), so
    /// the just-committed entry carries its state for a later cross-document
    /// traversal to restore. A plain navigation never calls this, so its entry
    /// keeps `classic_history_api_state = None`.
    pub fn set_current_state(&mut self, serialized_state: Option<Vec<u8>>) {
        if let Some(idx) = self.index {
            // Invariant: `index` always points to a valid entry.
            self.entries[idx].classic_history_api_state = serialized_state;
        }
    }

    /// Capture the current viewport scroll offset onto the **current** entry
    /// **before** a traversal moves the cursor (WHATWG HTML §7.4.6.2 step 6.4.4
    /// "restore persisted state" reads what leave captured). `Auto` mode only
    /// (the `Manual`-mode suppression is `#11-history-scroll-restoration-manual-mode`).
    pub fn set_current_scroll(&mut self, pos: (f64, f64)) {
        if let Some(idx) = self.index {
            self.entries[idx].scroll_position = Some(pos);
        }
    }

    /// Borrow the entry at `index` (e.g. a peeked traversal target) to read its
    /// `classic_history_api_state` + `scroll_position` at commit time — the
    /// read-source for delivering restored state (`popstate`) + scroll on a
    /// traversal (paired with [`peek_back`](Self::peek_back) etc., which return
    /// the target index). Reads the **target** entry, never `current()` (the
    /// cursor has not committed yet at read time under peek-then-commit).
    pub fn entry(&self, index: usize) -> Option<&HistoryEntry> {
        self.entries.get(index)
    }
}

impl Default for NavigationController {
    fn default() -> Self {
        Self::new()
    }
}

/// The outcome of classifying a traversal to a peeked target index by document
/// identity (WHATWG HTML §7.4.6.1), produced by
/// [`NavigationController::resolve_traversal`].
#[derive(Clone, Debug, PartialEq)]
pub enum TraversalKind {
    /// A **same-document** traversal (the target entry is in the current
    /// document): restore its serialized `history.state` (step 6.3, `state`) and
    /// persisted scroll (step 6.4.4, `scroll`) and fire popstate in place — no
    /// rebuild. Either field may be `None` (a plain-nav / boa-`None` entry carries
    /// no state; an entry never left carries no scroll).
    SameDocument {
        /// `StructuredSerializeForStorage` bytes to restore, or `None` for null.
        state: Option<Vec<u8>>,
        /// Persisted scroll offset `(x, y)` to restore, or `None`.
        scroll: Option<(f64, f64)>,
    },
    /// A **cross-document** traversal (a different document), OR a `go(0)` reload
    /// (History.go step 4): rebuild the document. Fires no popstate in place; the
    /// rebuilt document seeds `history.state` at the pre-eval chokepoint (§6.5).
    Rebuild,
}

/// The same-document determination for a navigation (WHATWG HTML §7.4.2.2
/// "Beginning navigation", the *navigate* algorithm step 15).
///
/// Distinguishes a **fragment** navigation — which updates the active
/// document's URL, session-history entry, and scroll position *in place* (no
/// fetch, no reparse, the existing document and its focus state persist) — from
/// a navigation that rebuilds the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavClass {
    /// The two URLs are equal excluding fragments AND the target's fragment is
    /// non-null: a fragment navigation (navigate step 15 → *navigate to a
    /// fragment*). Handled in place, without rebuilding the document.
    SameDocument,
    /// Every other navigation — a genuine cross-document load, a same-URL
    /// reload, or a fragment **removal** (`…#x` → `…`, target fragment null).
    /// Handled by rebuilding the document.
    CrossDocument,
}

/// Classify a navigation as same-document (fragment) or cross-document (WHATWG
/// HTML §7.4.2.2 *navigate* step 15).
///
/// `current` is the active document's URL; `target` is the requested URL.
/// Returns [`NavClass::SameDocument`] **iff** the two URLs are equal excluding
/// their fragments AND `target`'s fragment is non-null (step 15 conjuncts 3-4);
/// otherwise [`NavClass::CrossDocument`] — which covers a true cross-document
/// load, a same-URL reload, AND a fragment removal.
///
/// The predicate is deliberately **URL-pure** so it can live in this
/// engine-independent crate: step 15's other two conjuncts (`documentResource
/// is null`, `response is null`) are not URL facts and are gated by the shell
/// caller.
///
/// The fragment-**removal** case (`http://x/a#x` → `http://x/a`) is
/// `CrossDocument`: the target's fragment is null, so step 15's fourth conjunct
/// fails and the navigation is a full reload — matching real browsers. A naive
/// "fragments differ" predicate would wrongly treat removal as same-document
/// (pinned in the truth-table test).
pub fn classify_navigation(current: &url::Url, target: &url::Url) -> NavClass {
    // navigate step 15 conjunct 3 ("url equals … with exclude fragments set to
    // true") and conjunct 4 ("url's fragment is non-null").
    if url_equals_excluding_fragments(current, target) && target.fragment().is_some() {
        NavClass::SameDocument
    } else {
        NavClass::CrossDocument
    }
}

/// Compare two URLs ignoring their fragments — navigate step 15's "url equals
/// navigable's active session history entry's URL with exclude fragments set to
/// true".
///
/// Clears each URL's fragment on a clone and compares by the `url` crate's
/// serialization. This is a robustness upgrade over a crude `split('#').next()`
/// string compare: it uses normalized serializations, so default-port,
/// percent-encoding, and other normalization differences are handled correctly.
fn url_equals_excluding_fragments(a: &url::Url, b: &url::Url) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.set_fragment(None);
    b.set_fragment(None);
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn new_is_empty() {
        let nav = NavigationController::new();
        assert!(nav.is_empty());
        assert_eq!(nav.len(), 0);
        assert!(nav.current_url().is_none());
        assert!(!nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn push_and_current() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
        assert_eq!(nav.len(), 1);
    }

    #[test]
    fn push_multiple() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.push(url("https://c.com/"));
        assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
        assert_eq!(nav.len(), 3);
    }

    #[test]
    fn go_back_and_forward() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.push(url("https://c.com/"));

        assert!(nav.can_go_back());
        assert_eq!(nav.go_back().unwrap().as_str(), "https://b.com/");

        assert!(nav.can_go_forward());
        assert_eq!(nav.go_forward().unwrap().as_str(), "https://c.com/");
    }

    #[test]
    fn go_back_at_start_returns_none() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        assert!(nav.go_back().is_none());
        assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
    }

    #[test]
    fn go_forward_at_end_returns_none() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        assert!(nav.go_forward().is_none());
    }

    #[test]
    fn push_after_back_truncates_forward() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.push(url("https://c.com/"));
        nav.go_back();
        nav.go_back();
        // Now at a.com. Push a new URL.
        nav.push(url("https://d.com/"));
        assert_eq!(nav.current_url().unwrap().as_str(), "https://d.com/");
        assert_eq!(nav.len(), 2); // a, d
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn go_with_delta() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.push(url("https://c.com/"));

        assert_eq!(nav.go(-2).unwrap().as_str(), "https://a.com/");

        assert_eq!(nav.go(2).unwrap().as_str(), "https://c.com/");
    }

    #[test]
    fn go_out_of_range() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        assert!(nav.go(-1).is_none());
        assert!(nav.go(1).is_none());
        // Current position unchanged.
        assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
    }

    #[test]
    fn peek_does_not_move_cursor() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.push(url("https://c.com/"));
        // At c.com (index 2). Peeks must NOT move the cursor.
        assert_eq!(
            nav.peek_back().map(|(i, u)| (i, u.as_str())),
            Some((1, "https://b.com/"))
        );
        assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
        assert_eq!(
            nav.peek_go(-2).map(|(i, u)| (i, u.as_str())),
            Some((0, "https://a.com/"))
        );
        assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
        // Forward is a no-op at the end.
        assert!(nav.peek_forward().is_none());
        // Out-of-range peeks return None without moving.
        assert!(nav.peek_go(5).is_none());
        assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
    }

    #[test]
    fn peek_then_commit_is_atomic() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        // At b.com (index 1). Peek back but DON'T commit — cursor unmoved.
        let (target, _) = nav.peek_back().unwrap();
        assert_eq!(nav.current_url().unwrap().as_str(), "https://b.com/");
        // Commit the peeked index — now the cursor moves.
        nav.commit_index(target);
        assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
    }

    #[test]
    fn peek_go_zero_is_current() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        assert_eq!(
            nav.peek_go(0).map(|(i, u)| (i, u.as_str())),
            Some((1, "https://b.com/"))
        );
    }

    #[test]
    fn replace_current() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.replace(url("https://b-replaced.com/"));
        assert_eq!(
            nav.current_url().unwrap().as_str(),
            "https://b-replaced.com/"
        );
        assert_eq!(nav.len(), 2);
    }

    #[test]
    fn replace_empty_acts_as_push() {
        let mut nav = NavigationController::new();
        nav.replace(url("https://a.com/"));
        assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
        assert_eq!(nav.len(), 1);
    }

    #[test]
    fn set_title() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.set_current_title("Page A".to_string());
        assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
        assert_eq!(nav.current_title(), Some("Page A"));
    }

    #[test]
    fn go_zero_returns_current() {
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        assert_eq!(nav.go(0).unwrap().as_str(), "https://b.com/");
    }

    #[test]
    fn current_title_empty_history() {
        let nav = NavigationController::new();
        assert_eq!(nav.current_title(), None);
    }

    #[test]
    fn push_evicts_oldest_when_over_cap() {
        let mut nav = NavigationController::new();
        for i in 0..=MAX_HISTORY_ENTRIES {
            nav.push(url(&format!("https://page{i}.com/")));
        }
        // Should have been capped at MAX_HISTORY_ENTRIES.
        assert_eq!(nav.len(), MAX_HISTORY_ENTRIES);
        // The oldest entry (page0) should have been evicted.
        assert_eq!(nav.entries[0].url.as_str(), "https://page1.com/");
        // Current URL is the last pushed.
        assert_eq!(
            nav.current_url().unwrap().as_str(),
            &format!("https://page{MAX_HISTORY_ENTRIES}.com/")
        );
        // Index should point to the last entry.
        assert_eq!(nav.index, Some(MAX_HISTORY_ENTRIES - 1));
    }

    #[test]
    fn push_evicts_preserves_back_navigation() {
        let mut nav = NavigationController::new();
        for i in 0..=MAX_HISTORY_ENTRIES {
            nav.push(url(&format!("https://page{i}.com/")));
        }
        // Can still go back.
        assert!(nav.can_go_back());
        let expected_idx = MAX_HISTORY_ENTRIES - 1;
        assert_eq!(
            nav.go_back().unwrap().as_str(),
            &format!("https://page{expected_idx}.com/")
        );
    }

    // --- Same-document classifier (WHATWG HTML §7.4.2.2 navigate step 15) ---

    /// Pin url 2.x's `fragment()` distinction that the classifier's step-15
    /// "url's fragment is non-null" conjunct rests on: a *removed* fragment is
    /// `None` (⇒ CrossDocument), an *emptied* `#` fragment is `Some("")`
    /// (⇒ SameDocument), and a present fragment is `Some("x")`. If a url-crate
    /// change ever collapsed emptied and removed, the classifier correction
    /// (removal ⇒ CrossDocument, emptied ⇒ SameDocument) would silently regress.
    #[test]
    fn url_crate_fragment_semantics_pinned() {
        assert_eq!(url("http://x/a").fragment(), None, "removed ⇒ None");
        assert_eq!(
            url("http://x/a#").fragment(),
            Some(""),
            "emptied ⇒ Some(\"\")"
        );
        assert_eq!(
            url("http://x/a#x").fragment(),
            Some("x"),
            "present ⇒ Some(\"x\")"
        );
    }

    /// The full same-document classification truth table (plan §4.2 / §9).
    /// SameDocument IFF the URLs are equal excluding fragments AND the target
    /// fragment is non-null (navigate step 15 conjuncts 3-4). The **removal**
    /// (`/a#x → /a`) and **query-differ** (`/a?q=1 → /a?q=2#x`) rows are exactly
    /// the ones a naive "fragments differ" predicate gets wrong, so they are
    /// pinned here alongside every other case.
    #[test]
    fn classify_navigation_truth_table() {
        use NavClass::{CrossDocument, SameDocument};
        // (current, target, expected, label)
        let cases = [
            ("http://x/a", "http://x/a#x", SameDocument, "add fragment"),
            (
                "http://x/a#x",
                "http://x/a#y",
                SameDocument,
                "change fragment",
            ),
            (
                "http://x/a#x",
                "http://x/a",
                CrossDocument,
                "remove fragment (target frag null)",
            ),
            (
                "http://x/a#x",
                "http://x/a#",
                SameDocument,
                "empty fragment (target frag Some(\"\"))",
            ),
            (
                "http://x/a",
                "http://x/a#",
                SameDocument,
                "add empty fragment",
            ),
            (
                "http://x/a#x",
                "http://x/a#x",
                SameDocument,
                "identical incl. fragment",
            ),
            (
                "http://x/a",
                "http://x/a",
                CrossDocument,
                "identical, no fragment",
            ),
            ("http://x/a", "http://x/b", CrossDocument, "path differs"),
            (
                "http://x/a?q=1",
                "http://x/a?q=2#x",
                CrossDocument,
                "query differs (even with a fragment)",
            ),
            (
                "http://x/a",
                "https://x/a#x",
                CrossDocument,
                "scheme differs",
            ),
            ("http://x/a", "http://y/a#x", CrossDocument, "host differs"),
        ];
        for (current, target, expected, label) in cases {
            assert_eq!(
                classify_navigation(&url(current), &url(target)),
                expected,
                "classify_navigation({current}, {target}) [{label}]",
            );
        }
    }

    /// The helper compares normalized `url`-crate serializations, so it is
    /// robust where a crude `split('#')` string compare would differ: a
    /// default-port target is equal-excluding-fragments to its port-less form,
    /// so it classifies SameDocument given a non-null fragment.
    #[test]
    fn classify_navigation_normalizes_default_port() {
        assert_eq!(
            classify_navigation(&url("http://x:80/a"), &url("http://x/a#f")),
            NavClass::SameDocument,
        );
    }

    // --- 5c: serialized-state + scroll storage / traversal read path ---

    #[test]
    fn set_current_state_and_scroll_write_current_entry_and_entry_reads_them() {
        // The pushState/replaceState drain writes the serialized state + captured
        // scroll onto the CURRENT entry; a traversal reads them back from the
        // PEEKED TARGET entry via `entry(index)` (the traversal read-source).
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push(url("https://b.com/"));
        nav.set_current_state(Some(b"{\"n\":2}".to_vec()));
        nav.set_current_scroll((10.0, 20.0));
        let e = nav.entry(1).expect("current entry exists");
        assert_eq!(
            e.classic_history_api_state.as_deref(),
            Some(b"{\"n\":2}".as_slice())
        );
        assert_eq!(e.scroll_position, Some((10.0, 20.0)));
        // The other (a) entry is untouched — default `None` state + scroll.
        let a = nav.entry(0).expect("entry 0 exists");
        assert_eq!(a.classic_history_api_state, None);
        assert_eq!(a.scroll_position, None);
        // Out-of-range index → `None` (no panic).
        assert!(nav.entry(99).is_none());
    }

    #[test]
    fn push_starts_state_and_scroll_at_none() {
        // A fresh navigation entry carries no classic state / scroll until the
        // drain writes them — a plain nav's traversal restores `null` + no scroll.
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.set_current_state(Some(b"x".to_vec()));
        nav.set_current_scroll((1.0, 2.0));
        nav.push(url("https://b.com/"));
        let e = nav.entry(1).expect("new entry exists");
        assert_eq!(e.classic_history_api_state, None);
        assert_eq!(e.scroll_position, None);
    }

    #[test]
    fn resolve_traversal_classifies_by_document_identity_not_url() {
        // The one engine-independent traversal decision, by DOCUMENT IDENTITY
        // (§7.4.6.1 step 14.10), NOT URL: `pushState` routing gives same-document
        // entries different URLs (must be SameDocument), and a fresh nav to an
        // existing URL gives a different document the same URL (must be Rebuild).
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/")); // doc 1, index 0
        nav.push_same_document(url("https://a.com/products")); // doc 1, index 1 (pushState routing)
        nav.set_current_state(Some(b"{\"n\":2}".to_vec()));
        nav.set_current_scroll((5.0, 6.0));
        nav.push_same_document(url("https://a.com/products/2")); // doc 1, index 2
                                                                 // At index 2. A `go(0)` (target == current) is a RELOAD (Rebuild), NOT a
                                                                 // same-document no-op (History.go step 4).
        assert_eq!(nav.resolve_traversal(2), TraversalKind::Rebuild);
        // back() to /products (index 1) — DIFFERENT URL, SAME document → restore.
        assert_eq!(
            nav.resolve_traversal(1),
            TraversalKind::SameDocument {
                state: Some(b"{\"n\":2}".to_vec()),
                scroll: Some((5.0, 6.0)),
            }
        );
        // back() to / (index 0) — same document, no state/scroll.
        assert_eq!(
            nav.resolve_traversal(0),
            TraversalKind::SameDocument {
                state: None,
                scroll: None,
            }
        );
    }

    #[test]
    fn resolve_traversal_rebuilds_across_documents_and_same_url_different_doc() {
        // A cross-document navigation (different `document_sequence`) → Rebuild,
        // even when the URLs are equal-excluding-fragments — the case a URL-based
        // classifier gets WRONG (stale document). `location.replace()` and reload
        // are new-document events too.
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/")); // doc 1, index 0
        nav.push(url("https://a.com/")); // doc 2, index 1 — SAME url, fresh nav
                                         // At index 1 (doc 2). back() to index 0 (doc 1) — same URL, DIFFERENT
                                         // document → Rebuild (not a stale same-document no-rebuild).
        assert_eq!(nav.resolve_traversal(0), TraversalKind::Rebuild);

        // `location.replace()` stamps a NEW document even in place.
        let mut nav2 = NavigationController::new();
        nav2.push(url("https://a.com/")); // doc 1
        nav2.push_same_document(url("https://a.com/x")); // doc 1 (pushState)
        nav2.replace(url("https://a.com/y")); // location.replace() → doc 2 at index 1
        nav2.commit_index(0); // pretend a traversal committed to index 0
                              // From doc-1 entry 0, resolving to index 1 (now doc 2) → Rebuild.
        assert_eq!(nav2.resolve_traversal(1), TraversalKind::Rebuild);

        // reload re-stamps: after a fragment push (shared doc), reloading the base
        // makes a later back to the fragment cross-document.
        let mut nav3 = NavigationController::new();
        nav3.push(url("https://a.com/")); // doc 1, index 0
        nav3.push_same_document(url("https://a.com/#x")); // doc 1, index 1
        nav3.commit_index(0); // back to base
        nav3.restamp_current_document(); // reload the base → doc 2 at index 0
                                         // From reloaded base (doc 2, index 0), forward to #x (still doc 1) → Rebuild.
        assert_eq!(nav3.resolve_traversal(1), TraversalKind::Rebuild);
    }

    #[test]
    fn replace_clears_prior_state_and_scroll_new_document() {
        // `location.replace()` stamps a NEW document in place → it must carry NONE
        // of the replaced (e.g. pushState'd) entry's classic state or scroll, else a
        // later reload/traversal resurrects stale state (Codex R1 F2).
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/"));
        nav.push_same_document(url("https://a.com/x")); // pushState entry
        nav.set_current_state(Some(b"{\"n\":1}".to_vec()));
        nav.set_current_scroll((10.0, 20.0));
        let seq_before = nav.entry(1).unwrap().document_sequence;
        // location.replace() → new document, state + scroll cleared.
        nav.replace(url("https://a.com/y"));
        let e = nav.entry(1).unwrap();
        assert_eq!(
            e.classic_history_api_state, None,
            "replace clears classic state"
        );
        assert_eq!(e.scroll_position, None, "replace clears scroll");
        assert_ne!(
            e.document_sequence, seq_before,
            "replace stamps a new document_sequence"
        );
        // replace_same_document (replaceState) does NOT clear — same document, the
        // caller writes the new state.
        nav.set_current_state(Some(b"keep".to_vec()));
        nav.set_current_scroll((1.0, 2.0));
        nav.replace_same_document(url("https://a.com/z"));
        let e = nav.entry(1).unwrap();
        assert_eq!(
            e.classic_history_api_state.as_deref(),
            Some(b"keep".as_slice()),
            "replace_same_document keeps state (caller overwrites)"
        );
        assert_eq!(e.scroll_position, Some((1.0, 2.0)));
    }

    #[test]
    fn rebuild_traversal_restamps_target_so_siblings_stay_cross_document() {
        // A cross-document traversal REBUILDS the target as a fresh document, so the
        // shell re-stamps it (`restamp_current_document` after `commit_index`).
        // Without that re-stamp, the rebuilt target keeps the `document_sequence` it
        // shared with its former pushState siblings and a later traversal to such a
        // sibling mis-classifies same-document (stale document under a swapped URL).
        let mut nav = NavigationController::new();
        nav.push(url("https://a.com/")); // doc 1, index 0 (A)
        nav.push_same_document(url("https://a.com/a2")); // doc 1, index 1 (pushState sibling)
        nav.push(url("https://b.com/")); // doc 2, index 2 (cross-doc — destroys A's doc)
                                         // back() to /a2 (index 1): cross-document (D2 vs D1) → Rebuild.
        assert_eq!(nav.resolve_traversal(1), TraversalKind::Rebuild);
        // The shell rebuilds /a2 fresh, then commits + re-stamps.
        nav.commit_index(1);
        nav.restamp_current_document();
        // back() to A (index 0): A's document was destroyed → must be Rebuild. Without
        // the re-stamp above, entry[1] would still be D1 == entry[0] D1 → a wrong
        // SameDocument (stale /a2 content shown under the /a URL).
        assert_eq!(nav.resolve_traversal(0), TraversalKind::Rebuild);
    }

    #[test]
    fn eviction_preserves_entry_state() {
        // FIFO eviction over the cap keeps each surviving entry's serialized state
        // (the state rides the entry — a single Vec — not a parallel side-store, so
        // it evicts + re-indexes atomically with the entry).
        let mut nav = NavigationController::new();
        for i in 0..=MAX_HISTORY_ENTRIES {
            nav.push(url(&format!("https://page{i}.com/")));
            nav.set_current_state(Some(format!("{{\"i\":{i}}}").into_bytes()));
        }
        assert_eq!(nav.len(), MAX_HISTORY_ENTRIES);
        // page0 evicted; the oldest survivor is page1 carrying its own state.
        let first = nav.entry(0).expect("oldest survivor exists");
        assert_eq!(first.url.as_str(), "https://page1.com/");
        assert_eq!(
            first.classic_history_api_state.as_deref(),
            Some(b"{\"i\":1}".as_slice())
        );
        // The current (last) entry keeps its state too.
        let last = nav
            .entry(MAX_HISTORY_ENTRIES - 1)
            .expect("current entry exists");
        assert_eq!(
            last.classic_history_api_state.as_deref(),
            Some(format!("{{\"i\":{MAX_HISTORY_ENTRIES}}}").as_bytes())
        );
    }
}
