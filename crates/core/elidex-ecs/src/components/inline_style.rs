//! Inline-style declaration block component ([`InlineStyle`]).
//!
//! The CSSOM `CSSStyleDeclaration` backing store for `el.style.*`,
//! materialized lazily from `attrs("style")` (see
//! `elidex_dom_api::style::ensure_inline_style`). Split out of
//! `components.rs` to keep the shared component-definition bucket under
//! the 1000-line limit (the priority-carrying declaration model grew it
//! over).

use indexmap::IndexMap;

/// A single inline-style declaration: value text plus its `!important`
/// priority flag (CSSOM §6.6.1 — `getPropertyValue` returns the value
/// only; `getPropertyPriority` returns the flag).
#[derive(Debug, Clone, PartialEq, Eq)]
struct InlineDeclaration {
    value: String,
    important: bool,
}

/// Inline style declarations on an element.
///
/// Properties are stored in an `IndexMap` to preserve insertion order
/// (matching CSSOM `style.cssText` serialization order) while enforcing
/// uniqueness (last declaration wins, matching CSS cascade behavior).
///
/// Each declaration carries its `!important` priority flag, and
/// [`css_text`](Self::css_text) re-emits it. This is load-bearing for the
/// cascade: `sync_to_attribute` (elidex-dom-api) rewrites `attrs("style")`
/// from `css_text()` after every `el.style.*` mutation, and the cascade
/// re-parses that attribute — a priority-stripping serialization would
/// silently demote `!important` inline declarations on the first
/// unrelated style write. (Not `impl_string_map!`: the value carries the
/// priority flag, so this is no longer a plain string map.)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineStyle {
    properties: IndexMap<String, InlineDeclaration>,
}

impl InlineStyle {
    /// Get a style property value by name (value only, without priority —
    /// CSSOM `getPropertyValue`).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(|d| d.value.as_str())
    }

    /// Set a style property value with normal (non-important) priority.
    ///
    /// Clears any existing `!important` flag on the property — CSSOM
    /// §6.6.1 `setProperty` with an empty priority resets importance.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.set_with_priority(name, value, false);
    }

    /// Set a style property value with an explicit `!important` flag.
    pub fn set_with_priority(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        important: bool,
    ) {
        self.properties.insert(
            name.into(),
            InlineDeclaration {
                value: value.into(),
                important,
            },
        );
    }

    /// Remove a style property by name. Returns the removed value if
    /// present (the priority flag is removed with it).
    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.properties.shift_remove(name).map(|d| d.value)
    }

    /// Returns `true` if the property exists and is flagged `!important`
    /// (CSSOM `getPropertyPriority` ⇒ `"important"`).
    #[must_use]
    pub fn is_important(&self, name: &str) -> bool {
        self.properties.get(name).is_some_and(|d| d.important)
    }

    /// Returns the number of properties.
    #[must_use]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Returns `true` if there are no properties.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Serialize all properties to a CSS text string, including
    /// `!important` flags (CSSOM "serialize a CSS declaration block").
    ///
    /// Accepted divergences from the cited algorithm: declarations join
    /// with `"; "` and the block carries no trailing `;` (the spec
    /// emits one per declaration), and no shorthand reconstitution is
    /// performed — the block serializes its longhand-expanded canonical
    /// form (read-side shorthand serialization is deferred, slot
    /// `#11-style-shorthand-expand`). Both round-trip losslessly
    /// through `parse_declaration_block`.
    #[must_use]
    pub fn css_text(&self) -> String {
        self.properties
            .iter()
            .map(|(k, d)| {
                if d.important {
                    format!("{k}: {} !important", d.value)
                } else {
                    format!("{k}: {}", d.value)
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Get the property name at the given index (insertion order).
    #[must_use]
    pub fn property_at(&self, index: usize) -> Option<&str> {
        self.properties.keys().nth(index).map(String::as_str)
    }
}
