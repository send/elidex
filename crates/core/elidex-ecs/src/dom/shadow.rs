//! Shadow DOM methods for [`EcsDom`].

use crate::components::{
    NodeKind, ShadowHost, ShadowRoot, ShadowRootMode, SlotAssignment, SlotAssignmentMode, TagType,
    TreeRelation,
};
use hecs::Entity;

use super::EcsDom;

/// Initializer for [`EcsDom::attach_shadow_with_init`], mirroring the
/// WebIDL `ShadowRootInit` dictionary (WHATWG DOM §4.2.14).
#[derive(Clone, Copy, Debug)]
pub struct ShadowInit {
    pub mode: ShadowRootMode,
    pub delegates_focus: bool,
    pub slot_assignment: SlotAssignmentMode,
    pub clonable: bool,
    pub serializable: bool,
}

impl Default for ShadowInit {
    fn default() -> Self {
        Self {
            mode: ShadowRootMode::Open,
            delegates_focus: false,
            slot_assignment: SlotAssignmentMode::Named,
            clonable: false,
            serializable: false,
        }
    }
}

/// Error variants for [`EcsDom::attach_shadow_with_init`] per WHATWG
/// DOM §4.2.14 "attach a shadow root" validation steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowAttachError {
    /// Host entity doesn't exist or has no `TagType` component.
    InvalidEntity,
    /// Host's tag is not in the WHATWG allowlist + not a valid custom element name.
    InvalidTag,
    /// Host already has a shadow root attached (declarative-reuse path
    /// deferred to slot `#11-shadow-declarative-reuse`).
    AlreadyAttached,
}

/// Error variants for [`EcsDom::slot_assign`] per WHATWG DOM §4.2.2.5
/// "manually assignable" validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotAssignError {
    /// Target entity is not a `<slot>` element.
    NotASlot,
    /// Slot is not inside a shadow root.
    NoShadowRoot,
    /// Owning shadow root is `Named` mode (only `Manual` permits `slot.assign()`).
    NotManualMode,
    /// One of the nodes isn't a direct child of the shadow host.
    NotHostChild,
    /// One of the nodes is not an Element or Text (Comment / Document not slottable).
    InvalidNodeKind,
}

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
    /// Attach a shadow root to the given host element with default init.
    ///
    /// Convenience wrapper around [`Self::attach_shadow_with_init`] for
    /// the simple two-arg case (mode only, all other init fields default).
    /// Retained for callers (CSS / style / test fixtures) that don't need
    /// to specify the full init.
    ///
    /// Returns `Err(ShadowAttachError)` per the same validation rules as
    /// [`Self::attach_shadow_with_init`].
    pub fn attach_shadow(
        &mut self,
        host: Entity,
        mode: ShadowRootMode,
    ) -> Result<Entity, ShadowAttachError> {
        self.attach_shadow_with_init(
            host,
            ShadowInit {
                mode,
                ..Default::default()
            },
        )
    }

    /// Attach a shadow root to the given host element with full init.
    ///
    /// Creates a new shadow root entity as a child of `host` and marks
    /// `host` with a `ShadowHost` component. Returns the shadow root entity.
    ///
    /// Returns `Err(ShadowAttachError)` per WHATWG DOM §4.2.14 "attach a
    /// shadow root" validation:
    /// - [`ShadowAttachError::InvalidEntity`]: host doesn't exist or has no `TagType`
    /// - [`ShadowAttachError::InvalidTag`]: tag not in allowlist + not a valid custom element
    /// - [`ShadowAttachError::AlreadyAttached`]: host already has a shadow root
    pub fn attach_shadow_with_init(
        &mut self,
        host: Entity,
        init: ShadowInit,
    ) -> Result<Entity, ShadowAttachError> {
        let tag = self
            .world
            .get::<&TagType>(host)
            .map_err(|_| ShadowAttachError::InvalidEntity)?
            .0
            .clone();
        if !is_valid_shadow_host(&tag) {
            return Err(ShadowAttachError::InvalidTag);
        }
        if self.world.get::<&ShadowHost>(host).is_ok() {
            return Err(ShadowAttachError::AlreadyAttached);
        }

        let shadow_root_entity = self.world.spawn((
            ShadowRoot {
                mode: init.mode,
                host,
                delegates_focus: init.delegates_focus,
                slot_assignment: init.slot_assignment,
                clonable: init.clonable,
                serializable: init.serializable,
            },
            TreeRelation::default(),
            NodeKind::DocumentFragment,
        ));

        if !self.append_child(host, shadow_root_entity) {
            let _ = self.world.despawn(shadow_root_entity);
            return Err(ShadowAttachError::InvalidEntity);
        }

        let _ = self.world.insert_one(
            host,
            ShadowHost {
                shadow_root: shadow_root_entity,
            },
        );

        Ok(shadow_root_entity)
    }

    /// Assign a list of light-DOM nodes to a `<slot>` element (WHATWG DOM
    /// §4.2.2.5 "manually assignable" mode).  Validates:
    /// - `slot` must be a `<slot>` element
    /// - The slot's owning shadow root must use [`SlotAssignmentMode::Manual`]
    /// - Each node must be a Element-or-Text child of the shadow host
    ///
    /// Returns `Err(SlotAssignError)` on validation failure.  On success
    /// returns `Ok(changed)` where `changed` is `true` when the
    /// resulting `SlotAssignment.assigned_nodes` differs from the
    /// previous list — callers gate the `slotchange` signal on this
    /// per the spec's "assign slottables" step 2 ("if slottables and
    /// slot's assigned nodes are not identical, then signal a slot
    /// change").  Repeated `slot.assign(child)` with an unchanged
    /// list returns `Ok(false)` and produces no event.
    pub fn slot_assign(
        &mut self,
        slot: Entity,
        nodes: Vec<Entity>,
    ) -> Result<bool, SlotAssignError> {
        // Slot must be a <slot> element.  Case-insensitive match
        // mirrors `first_child_with_tag` / sibling HTML tag lookups
        // (HTML §13.2 normalises tag names case-insensitively but
        // the stored `TagType` may originate from a custom source).
        let is_slot = self
            .world
            .get::<&TagType>(slot)
            .ok()
            .is_some_and(|t| t.0.eq_ignore_ascii_case("slot"));
        if !is_slot {
            return Err(SlotAssignError::NotASlot);
        }

        // Walk up to the owning ShadowRoot to check mode.  The ShadowRoot
        // entity is an ancestor reachable via the TreeRelation parent chain.
        let host = self
            .shadow_root_for_slot(slot)
            .ok_or(SlotAssignError::NoShadowRoot)?;
        let mode = self
            .world
            .get::<&ShadowRoot>(
                self.shadow_root_entity_for_slot(slot)
                    .expect("validated above"),
            )
            .map(|sr| sr.slot_assignment)
            .map_err(|_| SlotAssignError::NoShadowRoot)?;
        if mode != SlotAssignmentMode::Manual {
            return Err(SlotAssignError::NotManualMode);
        }

        // Each node must be a Element-or-Text child of the host.
        for &node in &nodes {
            let parent = self.get_parent(node);
            if parent != Some(host) {
                return Err(SlotAssignError::NotHostChild);
            }
            let kind = self.world.get::<&NodeKind>(node).map(|k| *k).ok();
            if !matches!(kind, Some(NodeKind::Element | NodeKind::Text)) {
                return Err(SlotAssignError::InvalidNodeKind);
            }
        }

        // Apply assignment (insert SlotAssignment component if absent).
        // Split the existence check from the mutate-or-insert path so the
        // immutable probe borrow drops before the mutating call.  Compare
        // current vs new list to decide whether the assignment is a
        // semantic change (which gates the `slotchange` signal — see
        // doc-comment above).  A missing `SlotAssignment` component
        // represents the implicit empty list (spec: "manually assigned
        // nodes is initially empty"), so `slot.assign()` with no args
        // on a never-assigned slot is a no-op (`Ok(false)`).
        let existing_nodes: Option<Vec<Entity>> = self
            .world
            .get::<&SlotAssignment>(slot)
            .ok()
            .map(|sa| sa.assigned_nodes.clone());
        let existing_slice: &[Entity] = existing_nodes.as_deref().unwrap_or(&[]);
        let changed = existing_slice != nodes.as_slice();
        if existing_nodes.is_some() {
            if let Ok(mut existing) = self.world.get::<&mut SlotAssignment>(slot) {
                existing.assigned_nodes = nodes;
            }
        } else {
            let _ = self.world.insert_one(
                slot,
                SlotAssignment {
                    assigned_nodes: nodes,
                },
            );
        }
        Ok(changed)
    }

    /// Return the assigned (distributed) nodes for a `<slot>` element
    /// (WHATWG DOM §4.2.2.5 "find slottables" + `assignedNodes()`
    /// algorithm).
    ///
    /// Dispatch by the owning shadow root's slot-assignment mode:
    /// - **Manual** — return `SlotAssignment.assigned_nodes` (populated
    ///   by `slot.assign(...)`); empty if no manual assignment yet.
    /// - **Named** — walk the host's light-DOM children in tree order,
    ///   filtering to Element-or-Text whose effective slot name (the
    ///   `slot` content attribute for Elements, `""` for Text) matches
    ///   the slot's own `name` attribute (`""` for an unnamed slot).
    ///
    /// `flatten=true` should recursively expand nested-slot
    /// assignments per the "find flattened slottables" algorithm, but
    /// for now degrades to the non-flatten result — full recursion
    /// lands at slot `#11-shadow-slot-flatten`.
    ///
    /// Returns an empty vec when `slot` isn't inside a shadow tree
    /// (matches the spec's vacuous "no assigned slot" case).
    #[must_use]
    pub fn assigned_nodes(&self, slot: Entity, _flatten: bool) -> Vec<Entity> {
        let Some(sr_entity) = self.shadow_root_entity_for_slot(slot) else {
            return Vec::new();
        };
        let mode = self
            .world
            .get::<&ShadowRoot>(sr_entity)
            .map_or(SlotAssignmentMode::Named, |sr| sr.slot_assignment);
        if mode == SlotAssignmentMode::Manual {
            return self
                .world
                .get::<&SlotAssignment>(slot)
                .map(|sa| sa.assigned_nodes.clone())
                .unwrap_or_default();
        }
        // Named mode — walk light-DOM children of the host, match
        // by `slot` attribute vs. the slot's `name` attribute.
        let Some(host) = self
            .world
            .get::<&ShadowRoot>(sr_entity)
            .ok()
            .map(|sr| sr.host)
        else {
            return Vec::new();
        };
        let slot_name = self.get_attribute(slot, "name").unwrap_or_default();
        // Per WHATWG DOM §4.2.2.4 "find a slot", a slottable is
        // assigned to the FIRST slot in the shadow tree (in tree
        // order) whose name matches.  Duplicate-named slots later
        // in tree order report no matches — early-out here keeps
        // the per-child walk below O(host.children) instead of
        // duplicating across N slots with the same name.
        if self.first_named_slot_in_shadow(sr_entity, &slot_name) != Some(slot) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for child in self.children_iter(host) {
            let kind = self.world.get::<&NodeKind>(child).map(|k| *k).ok();
            let matches = match kind {
                Some(NodeKind::Element) => {
                    let child_slot = self.get_attribute(child, "slot").unwrap_or_default();
                    child_slot == slot_name
                }
                Some(NodeKind::Text) => slot_name.is_empty(),
                _ => false,
            };
            if matches {
                out.push(child);
            }
        }
        out
    }

    /// Locate the first `<slot>` element in the shadow tree (rooted
    /// at `sr`) whose `name` attribute equals `name`, in WHATWG
    /// "tree order" (depth-first pre-order).  Returns `None` if no
    /// slot matches.
    ///
    /// Used by [`Self::assigned_nodes`] to early-out for
    /// duplicate-named slots that come after the canonical match.
    fn first_named_slot_in_shadow(&self, sr: Entity, name: &str) -> Option<Entity> {
        for child in self.children_iter(sr).collect::<Vec<_>>() {
            if self
                .world
                .get::<&TagType>(child)
                .is_ok_and(|t| t.0.eq_ignore_ascii_case("slot"))
            {
                let n = self.get_attribute(child, "name").unwrap_or_default();
                if n == name {
                    return Some(child);
                }
            }
            if let Some(found) = self.first_named_slot_in_shadow(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// Locate the host Element for a `<slot>` by walking up to the
    /// nearest `ShadowRoot` ancestor and reading its `host` field.
    /// Returns `None` if the slot isn't inside a shadow tree.
    fn shadow_root_for_slot(&self, slot: Entity) -> Option<Entity> {
        let sr = self.shadow_root_entity_for_slot(slot)?;
        self.world.get::<&ShadowRoot>(sr).ok().map(|s| s.host)
    }

    /// Locate the `ShadowRoot` ENTITY for a `<slot>` by walking up the
    /// parent chain.  Returns `None` if the slot isn't inside a shadow tree.
    fn shadow_root_entity_for_slot(&self, slot: Entity) -> Option<Entity> {
        let mut current = self.get_parent(slot)?;
        loop {
            if self.world.get::<&ShadowRoot>(current).is_ok() {
                return Some(current);
            }
            current = self.get_parent(current)?;
        }
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
