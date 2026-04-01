//! Service Worker update check (WHATWG SW §4.4.4).
//!
//! Byte-for-byte comparison of decompressed SW script body.
//! BOM is preserved (not stripped) per spec.
//! ETag/304 responses are not shortcutted — full body fetch + comparison required.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Minimum interval between soft update checks (WHATWG SW §2.3).
const SOFT_UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Result of a SW update check.
#[derive(Debug)]
pub enum UpdateResult {
    /// Script body is identical (byte-for-byte). No update needed.
    NoChange,
    /// Script body changed. Contains the new body bytes and hash.
    Updated { new_body: Vec<u8>, new_hash: u64 },
    /// Network error prevented the update check.
    NetworkError(String),
}

/// Tracks when update checks were last performed.
#[derive(Debug, Default)]
pub struct UpdateChecker {
    last_check: HashMap<url::Url, Instant>,
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a soft update should be performed (24h elapsed since last check).
    ///
    /// Called on navigation to a URL within the SW's scope.
    pub fn should_soft_update(&self, script_url: &url::Url) -> bool {
        self.last_check
            .get(script_url)
            .map_or(true, |last| last.elapsed() >= SOFT_UPDATE_INTERVAL)
    }

    /// Record that an update check was performed.
    pub fn record_check(&mut self, script_url: &url::Url) {
        self.last_check.insert(script_url.clone(), Instant::now());
    }

    /// Force a check regardless of timing (for registration.update()).
    /// Always returns `true`.
    pub fn should_hard_update(&self) -> bool {
        true
    }
}

/// Compare two script bodies byte-for-byte.
///
/// Per WHATWG SW §4.4.4: comparison happens on decompressed body.
/// BOM is preserved (not stripped). Transport compression is transparent.
pub fn scripts_differ(old_body: &[u8], new_body: &[u8]) -> bool {
    old_body != new_body
}

/// Compute a hash of the script body for storage.
///
/// Uses a simple FNV-1a hash for fast comparison.
pub fn hash_script(body: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    for &byte in body {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn soft_update_first_time() {
        let checker = UpdateChecker::new();
        assert!(checker.should_soft_update(&url("https://example.com/sw.js")));
    }

    #[test]
    fn soft_update_recently_checked() {
        let mut checker = UpdateChecker::new();
        let sw = url("https://example.com/sw.js");
        checker.record_check(&sw);

        // Just checked — should not trigger
        assert!(!checker.should_soft_update(&sw));
    }

    #[test]
    fn hard_update_always_true() {
        let checker = UpdateChecker::new();
        assert!(checker.should_hard_update());
    }

    #[test]
    fn scripts_differ_identical() {
        assert!(!scripts_differ(b"console.log('v1')", b"console.log('v1')"));
    }

    #[test]
    fn scripts_differ_changed() {
        assert!(scripts_differ(b"console.log('v1')", b"console.log('v2')"));
    }

    #[test]
    fn scripts_differ_bom_preserved() {
        // BOM (UTF-8: EF BB BF) is significant per spec
        let with_bom = b"\xEF\xBB\xBFconsole.log('hi')";
        let without_bom = b"console.log('hi')";
        assert!(scripts_differ(with_bom, without_bom));
    }

    #[test]
    fn scripts_differ_whitespace_matters() {
        assert!(scripts_differ(b"a = 1", b"a  = 1"));
    }

    #[test]
    fn hash_deterministic() {
        let h1 = hash_script(b"test");
        let h2 = hash_script(b"test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_different_inputs() {
        assert_ne!(hash_script(b"a"), hash_script(b"b"));
    }
}
