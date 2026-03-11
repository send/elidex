//! Tab management for multi-tab browsing.
//!
//! Each tab owns a dedicated content thread, display list, and chrome state.

use std::thread::JoinHandle;

use elidex_render::DisplayList;

use crate::chrome::ChromeState;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// Unique identifier for a tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TabId(u64);

/// Monotonic tab ID generator.
struct TabIdGenerator(u64);

impl TabIdGenerator {
    fn new() -> Self {
        Self(0)
    }

    fn next(&mut self) -> TabId {
        self.0 = self.0.checked_add(1).expect("TabId overflow");
        TabId(self.0)
    }
}

/// A single browser tab with its own content thread.
pub(super) struct Tab {
    pub(super) id: TabId,
    pub(super) channel: LocalChannel<BrowserToContent, ContentToBrowser>,
    pub(super) thread: JoinHandle<()>,
    pub(super) can_go_back: bool,
    pub(super) can_go_forward: bool,
    pub(super) chrome: ChromeState,
    pub(super) display_list: DisplayList,
    pub(super) window_title: String,
}

impl Tab {
    /// Create a new tab.
    fn new(
        id: TabId,
        channel: LocalChannel<BrowserToContent, ContentToBrowser>,
        thread: JoinHandle<()>,
        chrome: ChromeState,
        window_title: String,
    ) -> Self {
        Self {
            id,
            channel,
            thread,
            can_go_back: false,
            can_go_forward: false,
            chrome,
            display_list: DisplayList::default(),
            window_title,
        }
    }

    /// Shut down this tab's content thread.
    ///
    /// Sends `Shutdown` and blocks until the thread exits. Note: `thread.join()`
    /// may block indefinitely if the content thread is deadlocked. `std::thread`
    /// does not support timed joins; a timeout-based approach would require
    /// additional infrastructure (e.g. a shared `AtomicBool`).
    fn shutdown(self) {
        let _ = self.channel.send(BrowserToContent::Shutdown);
        if let Err(e) = self.thread.join() {
            eprintln!("Content thread panicked: {e:?}");
        }
    }
}

/// Manages multiple tabs and tracks the active tab.
pub(super) struct TabManager {
    tabs: Vec<Tab>,
    active_id: Option<TabId>,
    id_gen: TabIdGenerator,
}

impl TabManager {
    /// Create an empty tab manager.
    pub(super) fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_id: None,
            id_gen: TabIdGenerator::new(),
        }
    }

    /// Create a new tab and return its ID. The new tab becomes active.
    pub(super) fn create_tab(
        &mut self,
        channel: LocalChannel<BrowserToContent, ContentToBrowser>,
        thread: JoinHandle<()>,
        chrome: ChromeState,
        title: String,
    ) -> TabId {
        let id = self.id_gen.next();
        self.tabs.push(Tab::new(id, channel, thread, chrome, title));
        self.active_id = Some(id);
        id
    }

    /// Close a tab by ID. Sends Shutdown and joins the thread.
    ///
    /// If the closed tab was active, selects a neighbor (prefer right, then left).
    /// Returns `true` if there are remaining tabs.
    pub(super) fn close_tab(&mut self, id: TabId) -> bool {
        let Some(idx) = self.tabs.iter().position(|t| t.id == id) else {
            return !self.tabs.is_empty();
        };
        let tab = self.tabs.remove(idx);
        tab.shutdown();

        if self.active_id == Some(id) {
            self.active_id = if self.tabs.is_empty() {
                None
            } else {
                let new_idx = idx.min(self.tabs.len() - 1);
                Some(self.tabs[new_idx].id)
            };
        }
        !self.tabs.is_empty()
    }

    /// Get the active tab (immutable).
    pub(super) fn active_tab(&self) -> Option<&Tab> {
        let id = self.active_id?;
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Get the active tab (mutable).
    pub(super) fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let id = self.active_id?;
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Switch to a different tab.
    pub(super) fn set_active(&mut self, id: TabId) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_id = Some(id);
        }
    }

    /// Get all tabs as a slice.
    pub(super) fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Get all tabs mutably.
    pub(super) fn tabs_mut(&mut self) -> &mut [Tab] {
        &mut self.tabs
    }

    /// Number of open tabs.
    #[must_use]
    #[allow(dead_code)]
    pub(super) fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get the active tab's ID.
    pub(super) fn active_id(&self) -> Option<TabId> {
        self.active_id
    }

    /// Get the tab ID after the active tab (wraps around).
    pub(super) fn next_tab_id(&self) -> Option<TabId> {
        let id = self.active_id?;
        let idx = self.tabs.iter().position(|t| t.id == id)?;
        let next_idx = (idx + 1) % self.tabs.len();
        Some(self.tabs[next_idx].id)
    }

    /// Get the tab ID before the active tab (wraps around).
    pub(super) fn prev_tab_id(&self) -> Option<TabId> {
        let id = self.active_id?;
        let idx = self.tabs.iter().position(|t| t.id == id)?;
        let prev_idx = if idx == 0 {
            self.tabs.len() - 1
        } else {
            idx - 1
        };
        Some(self.tabs[prev_idx].id)
    }

    /// Get the tab ID at position n (0-indexed). For Ctrl+1-9 shortcuts.
    pub(super) fn nth_tab_id(&self, n: usize) -> Option<TabId> {
        self.tabs.get(n).map(|t| t.id)
    }

    /// Shut down all tabs.
    pub(super) fn shutdown_all(&mut self) {
        for tab in self.tabs.drain(..) {
            tab.shutdown();
        }
        self.active_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc;

    fn create_test_tab(manager: &mut TabManager) -> TabId {
        let (browser_ch, content_ch) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        // Spawn a minimal thread that waits for shutdown.
        let thread = std::thread::spawn(move || {
            use std::time::Duration;
            loop {
                match content_ch.recv_timeout(Duration::from_secs(5)) {
                    Ok(BrowserToContent::Shutdown) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        });
        manager.create_tab(
            browser_ch,
            thread,
            ChromeState::new(None),
            "Test".to_string(),
        )
    }

    #[test]
    fn tab_id_monotonic() {
        let mut gen = TabIdGenerator::new();
        let a = gen.next();
        let b = gen.next();
        let c = gen.next();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_eq!(a.0, 1);
        assert_eq!(b.0, 2);
        assert_eq!(c.0, 3);
    }

    #[test]
    fn create_and_count() {
        let mut mgr = TabManager::new();
        assert_eq!(mgr.tab_count(), 0);
        assert!(mgr.active_tab().is_none());

        let id = create_test_tab(&mut mgr);
        assert_eq!(mgr.tab_count(), 1);
        assert_eq!(mgr.active_id(), Some(id));
    }

    #[test]
    fn close_selects_neighbor() {
        let mut mgr = TabManager::new();
        let _id1 = create_test_tab(&mut mgr);
        let id2 = create_test_tab(&mut mgr);
        let _id3 = create_test_tab(&mut mgr);

        // Active is id3 (last created). Close id2 (middle).
        mgr.set_active(id2);
        assert_eq!(mgr.active_id(), Some(id2));
        let has_tabs = mgr.close_tab(id2);
        assert!(has_tabs);
        // Should select the tab that took id2's position (id3).
        assert_ne!(mgr.active_id(), Some(id2));
        assert!(mgr.active_tab().is_some());

        // Close remaining.
        mgr.shutdown_all();
        assert_eq!(mgr.tab_count(), 0);
    }

    #[test]
    fn close_last_tab() {
        let mut mgr = TabManager::new();
        let id = create_test_tab(&mut mgr);
        let has_tabs = mgr.close_tab(id);
        assert!(!has_tabs);
        assert!(mgr.active_tab().is_none());
    }

    #[test]
    fn switch_tab() {
        let mut mgr = TabManager::new();
        let id1 = create_test_tab(&mut mgr);
        let id2 = create_test_tab(&mut mgr);
        assert_eq!(mgr.active_id(), Some(id2));

        mgr.set_active(id1);
        assert_eq!(mgr.active_id(), Some(id1));

        // Invalid ID is ignored.
        mgr.set_active(TabId(9999));
        assert_eq!(mgr.active_id(), Some(id1));

        mgr.shutdown_all();
    }

    #[test]
    fn next_prev_tab() {
        let mut mgr = TabManager::new();
        let id1 = create_test_tab(&mut mgr);
        let id2 = create_test_tab(&mut mgr);
        let id3 = create_test_tab(&mut mgr);

        mgr.set_active(id1);
        assert_eq!(mgr.next_tab_id(), Some(id2));
        assert_eq!(mgr.prev_tab_id(), Some(id3)); // wraps

        mgr.set_active(id3);
        assert_eq!(mgr.next_tab_id(), Some(id1)); // wraps
        assert_eq!(mgr.prev_tab_id(), Some(id2));

        mgr.shutdown_all();
    }

    #[test]
    fn nth_tab() {
        let mut mgr = TabManager::new();
        let id1 = create_test_tab(&mut mgr);
        let id2 = create_test_tab(&mut mgr);

        assert_eq!(mgr.nth_tab_id(0), Some(id1));
        assert_eq!(mgr.nth_tab_id(1), Some(id2));
        assert_eq!(mgr.nth_tab_id(2), None);

        mgr.shutdown_all();
    }

    #[test]
    fn tabs_slice() {
        let mut mgr = TabManager::new();
        let _id1 = create_test_tab(&mut mgr);
        let _id2 = create_test_tab(&mut mgr);
        assert_eq!(mgr.tabs().len(), 2);

        mgr.shutdown_all();
    }

    #[test]
    fn active_tab_mut() {
        let mut mgr = TabManager::new();
        let _id = create_test_tab(&mut mgr);
        let tab = mgr.active_tab_mut().unwrap();
        tab.window_title = "Modified".to_string();
        assert_eq!(mgr.active_tab().unwrap().window_title, "Modified");

        mgr.shutdown_all();
    }

    #[test]
    fn shutdown_all() {
        let mut mgr = TabManager::new();
        create_test_tab(&mut mgr);
        create_test_tab(&mut mgr);
        create_test_tab(&mut mgr);
        assert_eq!(mgr.tab_count(), 3);

        mgr.shutdown_all();
        assert_eq!(mgr.tab_count(), 0);
        assert!(mgr.active_id().is_none());
    }
}
