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

/// A pending navigation request from `location.assign()`, `location.href = ...`, etc.
#[derive(Clone, Debug)]
pub struct NavigationRequest {
    /// The target URL string.
    pub url: String,
    /// `true` for `location.replace()` / `history.replaceState()`.
    pub replace: bool,
}

/// A pending history action from `history.back()`, `history.forward()`, etc.
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
        /// Optional URL to push.
        url: Option<String>,
        /// Title (ignored in Phase 2 but accepted for API compat).
        title: String,
    },
    /// `history.replaceState(state, title, url?)`
    ReplaceState {
        /// Optional URL to replace.
        url: Option<String>,
        /// Title (ignored in Phase 2 but accepted for API compat).
        title: String,
    },
}

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

    /// Navigate back one step. Returns the new current URL, or `None`
    /// if already at the beginning.
    pub fn go_back(&mut self) -> Option<&url::Url> {
        let i = self.index.filter(|&i| i > 0)?;
        self.index = Some(i - 1);
        self.current_url()
    }

    /// Navigate forward one step. Returns the new current URL, or `None`
    /// if already at the end.
    pub fn go_forward(&mut self) -> Option<&url::Url> {
        let i = self.index?;
        if i + 1 < self.entries.len() {
            self.index = Some(i + 1);
            self.current_url()
        } else {
            None
        }
    }

    /// Navigate by a relative offset. Positive = forward, negative = back.
    /// Returns the new current URL, or `None` if the offset is out of range.
    ///
    /// Note: `go(0)` returns the current URL. The shell re-fetches the same
    /// URL, effectively reloading the page (matching browser `history.go(0)`
    /// semantics).
    pub fn go(&mut self, delta: i32) -> Option<&url::Url> {
        let current = self.index?;
        let abs = delta.unsigned_abs() as usize;
        let new_index = if delta >= 0 {
            current.checked_add(abs)?
        } else {
            current.checked_sub(abs)?
        };
        if new_index < self.entries.len() {
            self.index = Some(new_index);
            self.current_url()
        } else {
            None
        }
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
}
