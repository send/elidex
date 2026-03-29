//! Document-level state methods on `HostBridge`.
//!
//! Focus target, visibility, and window name.

use super::HostBridge;

impl HostBridge {
    /// Get the currently focused entity.
    pub fn focus_target(&self) -> Option<elidex_ecs::Entity> {
        self.inner.borrow().focus_target
    }

    /// Set the focused entity (synced from ContentState before eval).
    pub fn set_focus_target(&self, entity: Option<elidex_ecs::Entity>) {
        self.inner.borrow_mut().focus_target = entity;
    }

    /// Whether the tab is hidden (not the active tab).
    pub fn is_tab_hidden(&self) -> bool {
        self.inner.borrow().tab_hidden
    }

    /// Set the tab hidden state (synced from browser thread on tab switch).
    pub fn set_tab_hidden(&self, hidden: bool) {
        self.inner.borrow_mut().tab_hidden = hidden;
    }

    /// Get the window name.
    pub fn window_name(&self) -> String {
        self.inner.borrow().window_name.clone()
    }

    /// Set the window name.
    pub fn set_window_name(&self, name: String) {
        self.inner.borrow_mut().window_name = name;
    }
}
