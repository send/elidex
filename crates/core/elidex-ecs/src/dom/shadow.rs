//! Shadow DOM methods for [`EcsDom`].

use crate::components::{
    NodeKind, ShadowHost, ShadowRoot, ShadowRootMode, SlotAssignment, SlotAssignmentMode, TagType,
    TreeRelation,
};
use hecs::Entity;

use super::EcsDom;

/// Tags allowed as shadow hosts per WHATWG DOM 4.2.14.
/// Custom elements (valid custom element names) are also valid shadow hosts.
pub(super) const VALID_SHADOW_HOST_TAGS: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "body",
    "div",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "main",
    "nav",
    "p",
    "section",
    "span",
];

/// Reserved custom element names per HTML 4.13.2.
/// These contain a hyphen but are NOT valid custom element names.
const RESERVED_CUSTOM_ELEMENT_NAMES: &[&str] = &[
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-format",
    "font-face-name",
    "font-face-src",
    "font-face-uri",
    "missing-glyph",
];

/// Check if a tag name is a valid custom element name per HTML 4.13.2.
///
/// A valid custom element name must:
/// - Start with a lowercase ASCII letter
/// - Contain a hyphen
/// - Not be a reserved name
/// - Contain no uppercase ASCII letters
/// - Contain only `PCENChar` characters (simplified: ASCII lowercase, digits,
///   `-`, `_`, `.`, and non-ASCII)
fn is_valid_custom_element_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.contains('-')
        && !RESERVED_CUSTOM_ELEMENT_NAMES.contains(&name)
        && name.chars().all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || c == '-'
                || c == '_'
                || c == '.'
                || !c.is_ascii()
        })
}

/// Check if a tag name is valid as a shadow host (custom element or WHATWG whitelist).
fn is_valid_shadow_host(tag: &str) -> bool {
    is_valid_custom_element_name(tag) || VALID_SHADOW_HOST_TAGS.contains(&tag)
}

impl EcsDom {
    /// Attach a shadow root to the given host element.
    ///
    /// Creates a new shadow root entity as a child of `host` and marks
    /// `host` with a `ShadowHost` component. Returns the shadow root entity.
    ///
    /// Returns `Err(())` if:
    /// - The host element's tag is not in the valid shadow host list (WHATWG DOM 4.2.14)
    /// - The host already has a shadow root attached
    /// - The entity does not exist or has no `TagType`
    #[must_use = "returns Err if the operation failed"]
    #[allow(clippy::result_unit_err)] // WHATWG convention: attach_shadow fails with no useful error detail.
    pub fn attach_shadow(&mut self, host: Entity, mode: ShadowRootMode) -> Result<Entity, ()> {
        // Validate host exists and has a valid tag per WHATWG DOM 4.2.14.
        let tag = self.world.get::<&TagType>(host).map_err(|_| ())?.0.clone();
        if !is_valid_shadow_host(&tag) {
            return Err(());
        }

        // Reject if already a shadow host.
        if self.world.get::<&ShadowHost>(host).is_ok() {
            return Err(());
        }

        // Create shadow root entity.
        let shadow_root_entity = self.world.spawn((
            ShadowRoot {
                mode,
                host,
                delegates_focus: false,
                slot_assignment: SlotAssignmentMode::default(),
            },
            TreeRelation::default(),
            NodeKind::DocumentFragment,
        ));

        // Attach shadow root as child of host.
        if !self.append_child(host, shadow_root_entity) {
            let _ = self.world.despawn(shadow_root_entity);
            return Err(());
        }

        // Mark host.
        let _ = self.world.insert_one(
            host,
            ShadowHost {
                shadow_root: shadow_root_entity,
            },
        );

        Ok(shadow_root_entity)
    }

    /// Returns the shadow root entity for the given host, if any.
    ///
    /// Returns `None` if the shadow root entity has been destroyed (stale reference).
    #[must_use]
    pub fn get_shadow_root(&self, host: Entity) -> Option<Entity> {
        self.world
            .get::<&ShadowHost>(host)
            .ok()
            .map(|sh| sh.shadow_root)
            .filter(|&sr| self.world.contains(sr))
    }

    /// Returns the composed children for layout/render traversal.
    ///
    /// - Shadow host -> shadow root's children (skip shadow root entity itself)
    /// - `<slot>` with `SlotAssignment` -> assigned nodes (or fallback: slot's own children)
    /// - Otherwise -> normal `children()`
    #[must_use]
    pub fn composed_children(&self, entity: Entity) -> Vec<Entity> {
        // If entity is a shadow host, return shadow tree content.
        // Verify shadow root still exists (stale reference safety).
        if let Ok(sh) = self.world.get::<&ShadowHost>(entity) {
            if self.world.contains(sh.shadow_root) {
                return self.children(sh.shadow_root);
            }
            // Stale shadow root -- fall through to normal children.
        }

        // If entity is a <slot> with SlotAssignment, return assigned nodes.
        if let Ok(slot) = self.world.get::<&SlotAssignment>(entity) {
            if !slot.assigned_nodes.is_empty() {
                return slot.assigned_nodes.clone();
            }
            // Fallback: slot's own children (default content).
            return self.children(entity);
        }

        self.children(entity)
    }
}
