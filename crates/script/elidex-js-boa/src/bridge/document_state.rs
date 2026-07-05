//! Document-level state methods on `HostBridge`.
//!
//! Focus target, visibility, window name, and cookie access.

use super::HostBridge;

impl HostBridge {
    /// Get cookies for script access (filters out `HttpOnly` cookies).
    ///
    /// Uses the shared `CookieJar` reference from the Network Process.
    pub fn cookies_for_script(&self, url: &url::Url) -> String {
        let inner = self.inner.borrow();
        if let Some(jar) = &inner.cookie_jar {
            jar.cookies_for_script(url)
        } else {
            String::new()
        }
    }

    /// Get full cookie details for CookieStore API (WHATWG Cookie Store spec).
    ///
    /// Returns structured cookie data with domain, path, expires, secure, sameSite.
    pub fn cookie_details_for_script(&self, url: &url::Url) -> Vec<elidex_net::CookieSnapshot> {
        let inner = self.inner.borrow();
        if let Some(jar) = &inner.cookie_jar {
            jar.cookie_details_for_script(url)
        } else {
            Vec::new()
        }
    }

    /// Set a cookie from script (`document.cookie = "..."` setter).
    ///
    /// Rejects `HttpOnly` and Secure-over-non-HTTPS cookies.
    pub fn set_cookie_from_script(&self, url: &url::Url, value: &str) {
        let inner = self.inner.borrow();
        if let Some(jar) = &inner.cookie_jar {
            jar.set_cookie_from_script(url, value);
        }
    }

    /// Get the currently focused entity.
    pub fn focus_target(&self) -> Option<elidex_ecs::Entity> {
        self.inner.borrow().focus_target
    }

    /// Set the focused entity (synced from `ContentState` before eval).
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
        if let Some(old_value) = inner.session_storage.shift_remove(key) {
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
    /// Reads from the cached origin string in `HostBridgeInner`, which is
    /// updated by `set_current_url` (URL-derived default — but NOT while an
    /// opaque origin is installed, so a same-document fragment nav cannot clobber
    /// it, `bridge/navigation.rs`) and by `set_origin` (the installed override,
    /// which WINS — so a sandboxed/credentialless iframe partitions storage under
    /// its opaque origin, not the URL tuple; see `bridge/iframe_bridge.rs`).
    ///
    /// Opaque origins (data: URLs, sandboxed iframes) serialize to "null"
    /// per the URL spec. To prevent storage cross-contamination between
    /// unrelated opaque origins, we append a unique per-bridge ID:
    /// `"null:<bridge_id>"`.
    fn local_storage_origin(&self) -> String {
        let inner = self.inner.borrow();
        if inner.cached_origin == "null" {
            format!("null:{}", inner.bridge_id)
        } else {
            inner.cached_origin.clone()
        }
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
        self.push_storage_change(Some(key.to_string()), old_value, Some(value.to_string()));
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

#[cfg(test)]
mod tests {
    use super::super::HostBridge;
    use elidex_plugin::SecurityOrigin;

    /// F1 (sandbox origin-isolation): a sandboxed / credentialless URL iframe
    /// installs an OPAQUE origin via `set_origin` **after** `set_current_url`
    /// already seeded the tuple `cached_origin` from the URL. The installed
    /// opaque origin must WIN the localStorage partition key — else the frame
    /// aliases the real origin's storage bucket (a sandbox bypass). Falsify by
    /// reverting the `set_origin` → `cached_origin` sync.
    #[test]
    fn installed_opaque_origin_wins_localstorage_partition_over_url() {
        let bridge = HostBridge::new();
        bridge.set_current_url(Some(url::Url::parse("https://example.com/page").unwrap()));
        bridge.set_origin(SecurityOrigin::opaque());
        let key = bridge.local_storage_origin();
        assert!(
            key.starts_with("null:"),
            "sandboxed URL iframe must partition storage under the isolated \
             opaque key, not the URL tuple origin; got {key}"
        );
    }

    /// Regression pin: a same-origin (unsandboxed) URL iframe / top-level
    /// document installs a TUPLE origin — the partition key stays the tuple, not
    /// isolated-opaque.
    #[test]
    fn installed_tuple_origin_keeps_url_partition() {
        let bridge = HostBridge::new();
        let u = url::Url::parse("https://example.com/page").unwrap();
        bridge.set_current_url(Some(u.clone()));
        bridge.set_origin(SecurityOrigin::from_url(&u));
        assert_eq!(bridge.local_storage_origin(), "https://example.com");
    }

    /// F3 (S5-5b, Codex): a same-document (fragment) navigation calls
    /// `set_current_url` WITHOUT re-running `set_origin` (no pipeline rebuild).
    /// It must NOT re-derive `cached_origin` from the URL — that would switch an
    /// installed OPAQUE origin's isolated `null:<id>` partition to the URL tuple,
    /// the sandbox origin-isolation bypass the no-rebuild fragment path newly
    /// exposes. The isolated opaque partition must survive the fragment nav.
    /// Falsify by dropping the `!Opaque` guard in `set_current_url`.
    #[test]
    fn fragment_nav_preserves_opaque_localstorage_partition() {
        let bridge = HostBridge::new();
        let u = url::Url::parse("https://example.com/page").unwrap();
        bridge.set_current_url(Some(u.clone()));
        bridge.set_origin(SecurityOrigin::opaque());
        let opaque_key = bridge.local_storage_origin();
        assert!(
            opaque_key.starts_with("null:"),
            "installed opaque partition"
        );
        // The fragment nav: same URL + a fragment, `set_current_url` only.
        bridge.set_current_url(Some(
            url::Url::parse("https://example.com/page#sec").unwrap(),
        ));
        assert_eq!(
            bridge.local_storage_origin(),
            opaque_key,
            "the fragment nav preserves the isolated opaque partition, not the URL tuple"
        );
    }

    /// Regression pin (F3 sibling): a TUPLE-origin document's fragment nav is
    /// unaffected — the partition key stays the tuple (the fragment does not
    /// change the URL-tuple origin, so the re-derive is a harmless no-op).
    #[test]
    fn fragment_nav_keeps_tuple_localstorage_partition() {
        let bridge = HostBridge::new();
        let u = url::Url::parse("https://example.com/page").unwrap();
        bridge.set_current_url(Some(u.clone()));
        bridge.set_origin(SecurityOrigin::from_url(&u));
        bridge.set_current_url(Some(
            url::Url::parse("https://example.com/page#sec").unwrap(),
        ));
        assert_eq!(bridge.local_storage_origin(), "https://example.com");
    }
}
