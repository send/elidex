//! Document-level state methods on `HostBridge`.
//!
//! Focus target, visibility, window name, and cookie access.

use super::HostBridge;

impl HostBridge {
    /// Get cookies for script access (filters out HttpOnly cookies).
    ///
    /// Uses the shared `CookieJar` from the realtime state.
    pub fn cookies_for_script(&self, url: &url::Url) -> String {
        let inner = self.inner.borrow();
        if let Some(ref jar) = inner.realtime.cookie_jar_ref() {
            jar.cookies_for_script(url)
        } else {
            String::new()
        }
    }

    /// Set a cookie from script (`document.cookie = "..."` setter).
    ///
    /// Rejects HttpOnly and Secure-over-non-HTTPS cookies.
    pub fn set_cookie_from_script(&self, url: &url::Url, value: &str) {
        let inner = self.inner.borrow();
        if let Some(ref jar) = inner.realtime.cookie_jar_ref() {
            jar.set_cookie_from_script(url, value);
        }
    }

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

    // --- Session storage (tab-scoped) ---

    pub fn session_storage_get(&self, key: &str) -> Option<String> {
        self.inner.borrow().session_storage.get(key).cloned()
    }

    pub fn session_storage_set(&self, key: &str, value: &str) {
        self.inner
            .borrow_mut()
            .session_storage
            .insert(key.to_string(), value.to_string());
    }

    pub fn session_storage_remove(&self, key: &str) {
        self.inner.borrow_mut().session_storage.remove(key);
    }

    pub fn session_storage_clear(&self) {
        self.inner.borrow_mut().session_storage.clear();
    }

    pub fn session_storage_len(&self) -> usize {
        self.inner.borrow().session_storage.len()
    }

    pub fn session_storage_key(&self, index: usize) -> Option<String> {
        self.inner
            .borrow()
            .session_storage
            .keys()
            .nth(index)
            .cloned()
    }

    pub fn session_storage_byte_size(&self) -> usize {
        self.inner
            .borrow()
            .session_storage
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum()
    }

    // --- Local storage (origin-scoped, disk-persisted) ---

    /// Get the origin string for localStorage keying.
    fn local_storage_origin(&self) -> String {
        self.current_url()
            .map_or("null".to_string(), |url| url.origin().ascii_serialization())
    }

    pub fn local_storage_get(&self, key: &str) -> Option<String> {
        super::local_storage::local_storage_get(&self.local_storage_origin(), key)
    }

    pub fn local_storage_set(&self, key: &str, value: &str) {
        super::local_storage::local_storage_set(&self.local_storage_origin(), key, value);
    }

    pub fn local_storage_remove(&self, key: &str) {
        super::local_storage::local_storage_remove(&self.local_storage_origin(), key);
    }

    pub fn local_storage_clear(&self) {
        super::local_storage::local_storage_clear(&self.local_storage_origin());
    }

    pub fn local_storage_len(&self) -> usize {
        super::local_storage::local_storage_len(&self.local_storage_origin())
    }

    pub fn local_storage_key(&self, index: usize) -> Option<String> {
        super::local_storage::local_storage_key(&self.local_storage_origin(), index)
    }

    pub fn local_storage_byte_size(&self) -> usize {
        super::local_storage::local_storage_byte_size(&self.local_storage_origin())
    }
}
