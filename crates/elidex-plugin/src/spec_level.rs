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
/// Phase 2+ will add DOM4/Mutation Observer and Shadow DOM variants.
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
/// Phase 2+ will add TC39 stage-based variants for proposal features.
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
