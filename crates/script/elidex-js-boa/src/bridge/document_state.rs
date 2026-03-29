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

    // is_tab_hidden / set_visibility are in viewport.rs

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
        let mut inner = self.inner.borrow_mut();
        // Subtract old key+value len if the key already exists.
        if let Some(old_value) = inner.session_storage.get(key) {
            inner.session_storage_bytes -= key.len() + old_value.len();
        }
        inner.session_storage_bytes += key.len() + value.len();
        inner
            .session_storage
            .insert(key.to_string(), value.to_string());
    }

    pub fn session_storage_remove(&self, key: &str) {
        let mut inner = self.inner.borrow_mut();
        if let Some(old_value) = inner.session_storage.remove(key) {
            inner.session_storage_bytes -= key.len() + old_value.len();
        }
    }

    pub fn session_storage_clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.session_storage.clear();
        inner.session_storage_bytes = 0;
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
        self.inner.borrow().session_storage_bytes
    }

    // --- Local storage (origin-scoped, disk-persisted) ---

    /// Get the origin string for localStorage keying.
    ///
    /// Reads from the cached origin string in `HostBridgeInner`,
    /// which is updated whenever `set_current_url` is called.
    fn local_storage_origin(&self) -> String {
        self.inner.borrow().cached_origin.clone()
    }

    /// Push a storage change notification for cross-tab broadcast.
    fn push_storage_change(
        &self,
        key: Option<String>,
        old_value: Option<String>,
        new_value: Option<String>,
    ) {
        let origin = self.local_storage_origin();
        let url = self.current_url().map_or(String::new(), |u| u.to_string());
        self.inner
            .borrow_mut()
            .pending_storage_changes
            .push(super::StorageChange {
                origin,
                key,
                old_value,
                new_value,
                url,
            });
    }

    pub fn local_storage_get(&self, key: &str) -> Option<String> {
        super::local_storage::local_storage_get(&self.local_storage_origin(), key)
    }

    pub fn local_storage_set(&self, key: &str, value: &str) {
        let origin = self.local_storage_origin();
        let old_value = super::local_storage::local_storage_get(&origin, key);
        super::local_storage::local_storage_set(&origin, key, value);
        self.push_storage_change(
            Some(key.to_string()),
            old_value,
            Some(value.to_string()),
        );
    }

    pub fn local_storage_remove(&self, key: &str) {
        let origin = self.local_storage_origin();
        let old_value = super::local_storage::local_storage_get(&origin, key);
        super::local_storage::local_storage_remove(&origin, key);
        self.push_storage_change(Some(key.to_string()), old_value, None);
    }

    pub fn local_storage_clear(&self) {
        let origin = self.local_storage_origin();
        super::local_storage::local_storage_clear(&origin);
        self.push_storage_change(None, None, None);
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

    /// Drain pending localStorage change notifications.
    pub fn drain_storage_changes(&self) -> Vec<super::StorageChange> {
        std::mem::take(&mut self.inner.borrow_mut().pending_storage_changes)
    }
}
