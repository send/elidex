//! Spec-level enums for each web platform layer (Ch.7 §7.6).
//!
//! Each enum classifies features by their specification status,
//! enabling the plugin system to selectively enable or disable
//! support for legacy and non-standard features.

/// HTML tag specification levels.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HtmlSpecLevel {
    /// Standard HTML5 Living Standard elements.
    #[default]
    Html5,
    /// Legacy elements still widely used but not recommended.
    Legacy,
    /// Deprecated elements that should not be used.
    Deprecated,
}

/// DOM API specification levels.
///
/// Future: may add DOM4/Mutation Observer and Shadow DOM variants.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DomSpecLevel {
    /// Current DOM Living Standard APIs.
    #[default]
    Living,
    /// Legacy DOM APIs kept for backward compatibility.
    Legacy,
    /// Deprecated DOM APIs.
    Deprecated,
}

/// ECMAScript specification levels.
///
/// Future: may add TC39 stage-based variants for proposal features.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum EsSpecLevel {
    /// Modern ECMAScript features (ES2015+).
    #[default]
    Modern,
    /// Legacy semantics maintained for web compatibility.
    LegacySemantics,
    /// Annex B features for web browser legacy behavior.
    AnnexB,
}

/// CSS specification levels.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum CssSpecLevel {
    /// Standard CSS properties and values.
    #[default]
    Standard,
    /// Aliased properties (e.g. `word-wrap` to `overflow-wrap`).
    Aliased,
    /// Non-standard vendor-prefixed properties.
    NonStandard,
    /// Deprecated CSS features.
    Deprecated,
}

/// Web API specification levels.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WebApiSpecLevel {
    /// Modern Web APIs.
    #[default]
    Modern,
    /// Legacy Web APIs kept for backward compatibility.
    Legacy,
    /// Deprecated Web APIs.
    Deprecated,
}

/// Engine-wide operating mode, supplied by the embedder at VM construction.
///
/// The single authority for the whole-engine core / compat / deprecated split:
/// every layer (Web-API install plumbing, style, DOM) derives its own
/// [`SpecLevelPolicy`] from this one mode rather than carrying a private
/// per-domain switch. Fixed at construction — *before* any installer runs — so
/// a mode can *prevent* an install rather than needing a later removal path.
///
/// ⚠ `BrowserCore` / `App` must **not** be selected for a real session until
/// the async core storage (`#11-async-core-storage-cookiestore`) lands: a core
/// session is contracted to expose `elidex.storage` (design §14.4.3), so
/// selecting these modes before then yields a session with *no* storage API.
/// Until the async core exists they are exercised by unit tests only; the shell
/// supplies [`BrowserCompat`](EngineMode::BrowserCompat).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum EngineMode {
    /// Browser with the compatibility surface installed (Modern + Legacy).
    /// The default — byte-identical to the pre-gate engine.
    #[default]
    BrowserCompat,
    /// Browser core: the modern standard baseline only; `Legacy` excluded.
    BrowserCore,
    /// Application embedding: modern baseline only. In addition to the runtime
    /// exclusion, the compat shims are compile-excluded via
    /// `feature = "compat-webapi"` so they are absent from the binary.
    App,
}

impl EngineMode {
    /// Derive the spec-level install policy for this mode.
    ///
    /// `BrowserCompat` installs `Legacy`; `BrowserCore` / `App` do not.
    #[must_use]
    pub fn spec_level_policy(self) -> SpecLevelPolicy {
        SpecLevelPolicy {
            exclude_legacy: !matches!(self, EngineMode::BrowserCompat),
        }
    }
}

/// The install policy a layer consults at every registration seam.
///
/// Derived once at VM construction from an [`EngineMode`]
/// ([`EngineMode::spec_level_policy`]) and asked, at each install seam,
/// whether an API of a given spec level installs for this session. Shared by
/// the Web-API ([`installs`](SpecLevelPolicy::installs)) and DOM
/// ([`installs_dom`](SpecLevelPolicy::installs_dom)) surfaces, which gate
/// `Legacy` identically; the struct can grow fields if a future mode needs the
/// two surfaces to diverge.
///
/// The field is phrased as an *exclusion* (`exclude_legacy`) so the derived
/// [`Default`] (all-false = nothing excluded) equals
/// `EngineMode::default()` = [`BrowserCompat`](EngineMode::BrowserCompat) — the
/// zero-behavior-change baseline. A caller that constructs a registry without an
/// explicit mode therefore gets the full compat surface, never an accidental
/// core-mode prune.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct SpecLevelPolicy {
    /// When `true`, `Legacy`-classified APIs are withheld. `Deprecated` never
    /// installs regardless (no Deprecated API is implemented; reserved).
    exclude_legacy: bool,
}

impl SpecLevelPolicy {
    /// Whether a Web API classified at `level` installs under this policy.
    #[must_use]
    pub fn installs(&self, level: WebApiSpecLevel) -> bool {
        match level {
            WebApiSpecLevel::Modern => true,
            WebApiSpecLevel::Legacy => !self.exclude_legacy,
            WebApiSpecLevel::Deprecated => false,
        }
    }

    /// Whether a DOM API classified at `level` installs under this policy.
    #[must_use]
    pub fn installs_dom(&self, level: DomSpecLevel) -> bool {
        match level {
            DomSpecLevel::Living => true,
            DomSpecLevel::Legacy => !self.exclude_legacy,
            DomSpecLevel::Deprecated => false,
        }
    }

    /// Return this policy with `Legacy` APIs unconditionally excluded.
    ///
    /// Used to express a **compile-time hard ceiling above the runtime mode**:
    /// an embedder that does not compile the compat shims in (e.g. an
    /// `App`-profile build with the `compat-webapi` feature off) applies this so
    /// no `Legacy` API can install regardless of the [`EngineMode`] selected at
    /// runtime. `Modern` / `Living` are unaffected.
    #[must_use]
    pub fn with_legacy_excluded(self) -> Self {
        Self {
            exclude_legacy: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_mode_default_is_browser_compat() {
        // The default mode must be the full compat surface so a registry/VM
        // built without an explicit mode is byte-identical to the pre-gate
        // engine. `SpecLevelPolicy::default()` must agree.
        assert_eq!(EngineMode::default(), EngineMode::BrowserCompat);
        assert_eq!(
            SpecLevelPolicy::default(),
            EngineMode::BrowserCompat.spec_level_policy()
        );
    }

    #[test]
    fn browser_compat_installs_modern_and_legacy() {
        let p = EngineMode::BrowserCompat.spec_level_policy();
        assert!(p.installs(WebApiSpecLevel::Modern));
        assert!(p.installs(WebApiSpecLevel::Legacy));
        assert!(p.installs_dom(DomSpecLevel::Living));
        assert!(p.installs_dom(DomSpecLevel::Legacy));
    }

    #[test]
    fn browser_core_and_app_exclude_legacy_keep_modern() {
        for mode in [EngineMode::BrowserCore, EngineMode::App] {
            let p = mode.spec_level_policy();
            // Modern / Living baseline always installs.
            assert!(p.installs(WebApiSpecLevel::Modern), "{mode:?} drops Modern");
            assert!(
                p.installs_dom(DomSpecLevel::Living),
                "{mode:?} drops Living"
            );
            // Legacy is excluded.
            assert!(
                !p.installs(WebApiSpecLevel::Legacy),
                "{mode:?} must exclude Legacy Web API"
            );
            assert!(
                !p.installs_dom(DomSpecLevel::Legacy),
                "{mode:?} must exclude Legacy DOM API"
            );
        }
    }

    #[test]
    fn deprecated_never_installs_in_any_mode() {
        for mode in [
            EngineMode::BrowserCompat,
            EngineMode::BrowserCore,
            EngineMode::App,
        ] {
            assert!(
                !mode
                    .spec_level_policy()
                    .installs(WebApiSpecLevel::Deprecated),
                "{mode:?} must never install Deprecated"
            );
        }
    }

    #[test]
    fn with_legacy_excluded_is_the_compile_time_ceiling() {
        // Even starting from BrowserCompat (Legacy on), the hard ceiling drops
        // Legacy while leaving Modern/Living intact — the `compat-webapi`-off
        // app-profile behavior.
        let ceiled = EngineMode::BrowserCompat
            .spec_level_policy()
            .with_legacy_excluded();
        assert!(ceiled.installs(WebApiSpecLevel::Modern));
        assert!(ceiled.installs_dom(DomSpecLevel::Living));
        assert!(!ceiled.installs(WebApiSpecLevel::Legacy));
        assert!(!ceiled.installs_dom(DomSpecLevel::Legacy));
    }
}
