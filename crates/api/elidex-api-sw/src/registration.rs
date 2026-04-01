//! Service Worker registration state machine (WHATWG SW §4.3).

use std::time::Instant;

/// Service Worker lifecycle state (WHATWG SW §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwState {
    /// Script downloaded, not yet evaluated.
    Parsed,
    /// Install event being processed.
    Installing,
    /// Install succeeded, waiting for activation conditions.
    Installed,
    /// Activate event being processed.
    Activating,
    /// Active and handling fetch events.
    Activated,
    /// Replaced by a newer SW, or waitUntil() rejected, or script eval failed.
    Redundant,
}

impl SwState {
    /// Whether this state can handle fetch events.
    pub fn is_active(&self) -> bool {
        *self == Self::Activated
    }

    /// Whether this state can receive new events (install, activate, or active).
    pub fn is_alive(&self) -> bool {
        !matches!(self, Self::Redundant)
    }
}

/// updateViaCache option (WHATWG SW §4.4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateViaCache {
    /// HTTP cache not consulted for main script, consulted for imports (default).
    Imports,
    /// HTTP cache consulted for both main script and imports.
    All,
    /// HTTP cache never consulted.
    None,
}

impl UpdateViaCache {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "imports" => Some(Self::Imports),
            "all" => Some(Self::All),
            "none" => Some(Self::None),
            _ => Option::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Imports => "imports",
            Self::All => "all",
            Self::None => "none",
        }
    }
}

impl Default for UpdateViaCache {
    fn default() -> Self {
        Self::Imports
    }
}

/// A single Service Worker registration.
#[derive(Debug, Clone)]
pub struct SwRegistration {
    /// URL scope this registration controls.
    pub scope: url::Url,
    /// URL of the SW script.
    pub script_url: url::Url,
    /// Current lifecycle state.
    pub state: SwState,
    /// Hash of the SW script body (for byte-for-byte update comparison).
    pub script_hash: Option<u64>,
    /// Last time an update check was performed.
    pub last_update_check: Option<Instant>,
    /// HTTP cache behavior for updates.
    pub update_via_cache: UpdateViaCache,
}

/// In-memory store of SW registrations, keyed by (origin, scope).
#[derive(Debug, Default)]
pub struct SwRegistrationStore {
    registrations: Vec<SwRegistration>,
}

impl SwRegistrationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new SW or update an existing one for the same scope.
    ///
    /// If a registration with the same scope exists, the new one replaces it
    /// and the old one transitions to Redundant.
    pub fn register(&mut self, reg: SwRegistration) {
        // Mark any existing registration with same scope as Redundant
        for existing in &mut self.registrations {
            if existing.scope == reg.scope && existing.state != SwState::Redundant {
                existing.state = SwState::Redundant;
            }
        }
        // Remove Redundant entries to avoid accumulation
        self.registrations.retain(|r| r.state != SwState::Redundant);
        self.registrations.push(reg);
    }

    /// Find a registration by scope URL.
    pub fn get_by_scope(&self, scope: &url::Url) -> Option<&SwRegistration> {
        self.registrations
            .iter()
            .find(|r| &r.scope == scope && r.state != SwState::Redundant)
    }

    /// Find a mutable registration by scope URL.
    pub fn get_by_scope_mut(&mut self, scope: &url::Url) -> Option<&mut SwRegistration> {
        self.registrations
            .iter_mut()
            .find(|r| &r.scope == scope && r.state != SwState::Redundant)
    }

    /// List all non-redundant registrations.
    pub fn all(&self) -> &[SwRegistration] {
        &self.registrations
    }

    /// Remove a registration by scope (unregister).
    pub fn unregister(&mut self, scope: &url::Url) -> bool {
        let before = self.registrations.len();
        self.registrations.retain(|r| &r.scope != scope);
        self.registrations.len() < before
    }

    /// Transition a registration to a new state.
    pub fn set_state(&mut self, scope: &url::Url, new_state: SwState) {
        if let Some(reg) = self.get_by_scope_mut(scope) {
            reg.state = new_state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    fn sample_reg(scope: &str) -> SwRegistration {
        SwRegistration {
            scope: url(scope),
            script_url: url(&format!("{scope}sw.js")),
            state: SwState::Parsed,
            script_hash: None,
            last_update_check: None,
            update_via_cache: UpdateViaCache::default(),
        }
    }

    #[test]
    fn state_is_active() {
        assert!(!SwState::Parsed.is_active());
        assert!(!SwState::Installing.is_active());
        assert!(!SwState::Installed.is_active());
        assert!(!SwState::Activating.is_active());
        assert!(SwState::Activated.is_active());
        assert!(!SwState::Redundant.is_active());
    }

    #[test]
    fn state_is_alive() {
        assert!(SwState::Parsed.is_alive());
        assert!(SwState::Installing.is_alive());
        assert!(SwState::Activated.is_alive());
        assert!(!SwState::Redundant.is_alive());
    }

    #[test]
    fn register_and_lookup() {
        let mut store = SwRegistrationStore::new();
        store.register(sample_reg("https://example.com/"));

        assert!(store.get_by_scope(&url("https://example.com/")).is_some());
        assert!(store.get_by_scope(&url("https://other.com/")).is_none());
    }

    #[test]
    fn register_replaces_existing() {
        let mut store = SwRegistrationStore::new();
        let mut reg1 = sample_reg("https://example.com/");
        reg1.state = SwState::Activated;
        store.register(reg1);

        let reg2 = sample_reg("https://example.com/");
        store.register(reg2);

        // Only the new one should be present
        assert_eq!(store.all().len(), 1);
        assert_eq!(
            store
                .get_by_scope(&url("https://example.com/"))
                .unwrap()
                .state,
            SwState::Parsed
        );
    }

    #[test]
    fn unregister() {
        let mut store = SwRegistrationStore::new();
        store.register(sample_reg("https://example.com/"));

        assert!(store.unregister(&url("https://example.com/")));
        assert!(store.get_by_scope(&url("https://example.com/")).is_none());

        assert!(!store.unregister(&url("https://example.com/"))); // already removed
    }

    #[test]
    fn set_state() {
        let mut store = SwRegistrationStore::new();
        store.register(sample_reg("https://example.com/"));

        store.set_state(&url("https://example.com/"), SwState::Activated);
        assert_eq!(
            store
                .get_by_scope(&url("https://example.com/"))
                .unwrap()
                .state,
            SwState::Activated
        );
    }

    #[test]
    fn update_via_cache_parse() {
        assert_eq!(UpdateViaCache::parse("imports"), Some(UpdateViaCache::Imports));
        assert_eq!(UpdateViaCache::parse("all"), Some(UpdateViaCache::All));
        assert_eq!(UpdateViaCache::parse("none"), Some(UpdateViaCache::None));
        assert_eq!(UpdateViaCache::parse("invalid"), Option::None);
    }

    #[test]
    fn multiple_scopes_coexist() {
        let mut store = SwRegistrationStore::new();
        store.register(sample_reg("https://example.com/"));
        store.register(sample_reg("https://example.com/app/"));

        assert_eq!(store.all().len(), 2);
    }
}
