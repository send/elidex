//! `NavigationController` — session history management.
//!
//! Implements a linear history stack with back/forward/go navigation,
//! push, and replace operations. Mirrors the browser's session history
//! for a single browsing context.

/// Scroll restoration mode (WHATWG HTML §7.4.2).
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
    /// Serialized state from pushState/replaceState (JSON string).
    pub classic_history_api_state: Option<String>,
    /// Navigation API state (WHATWG HTML §7.4.1, initially undefined).
    pub navigation_api_state: Option<String>,
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
}

impl NavigationController {
    /// Create a new empty navigation controller.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: None,
            next_entry_id: 1,
        }
    }

    /// Generate a unique opaque string for navigation API key/ID.
    fn next_id(&mut self) -> String {
        let id = self.next_entry_id;
        self.next_entry_id += 1;
        format!("{id:016x}")
    }

    /// Push a new URL onto the history, discarding any forward entries.
    pub fn push(&mut self, url: url::Url) {
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

    /// Replace the current entry's URL without adding a new entry.
    ///
    /// If there are no entries, this behaves like `push`.
    /// The `navigation_api_key` is preserved (spec: key survives replace),
    /// but `navigation_api_id` gets a new value.
    pub fn replace(&mut self, url: url::Url) {
        if let Some(idx) = self.index {
            let new_id = self.next_id();
            self.entries[idx].url = url;
            self.entries[idx].navigation_api_id = new_id;
        } else {
            self.push(url);
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
}

impl Default for NavigationController {
    fn default() -> Self {
        Self::new()
    }
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
}
