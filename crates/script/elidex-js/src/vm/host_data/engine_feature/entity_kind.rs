//! Wrapper-prototype classification of bound DOM entities.
//!
//! [`PrototypeKind`] and the read-only ECS brand probes
//! (`is_element_entity` / `is_shadow_root_entity` /
//! `tag_matches_ascii_case` / `prototype_kind_for`) that
//! `VmInner::create_element_wrapper` uses to pick a wrapper prototype.
//! Carved out of `host_data/mod.rs` as a cohesion seam (pure move).

use super::HostData;
use elidex_ecs::{Entity, NodeKind};

/// Four-way partition of DOM wrapper prototype chains used by
/// `VmInner::create_element_wrapper`.  The
/// [`PrototypeKind`] is derived from the entity's ECS components
/// so the caller does not need to run independent
/// `is_element_entity` / `is_character_data_entity` checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrototypeKind {
    /// Entity carries a `TagType` — Element wrapper chain:
    /// `Element.prototype → Node.prototype → …`.
    Element,
    /// `NodeKind::Text` — chains via `Text.prototype →
    /// CharacterData.prototype → Node.prototype → …`.
    Text,
    /// Other CharacterData (currently `Comment`; future
    /// `ProcessingInstruction` / `CdataSection`) — chains via
    /// `CharacterData.prototype → Node.prototype → …`.
    OtherCharacterData,
    /// `NodeKind::DocumentType` — chains via
    /// `DocumentType.prototype → Node.prototype → …`.  Carries
    /// `name` / `publicId` / `systemId`.
    DocumentType,
    /// `NodeKind::DocumentFragment` — chains via
    /// `DocumentFragment.prototype → Node.prototype → …`.
    /// Carries the ParentNode mixin (`prepend` / `append` /
    /// `replaceChildren`) per WHATWG DOM §4.7.
    DocumentFragment,
    /// Entity carries an `elidex_ecs::ShadowRoot` component —
    /// chains via `ShadowRoot.prototype → DocumentFragment.prototype
    /// → Node.prototype → …`.  Brand check for ShadowRoot
    /// receivers is the engine-component lookup
    /// `world.get::<&elidex_ecs::ShadowRoot>(entity).is_ok()`,
    /// uniform with how other ECS-backed wrapper kinds are
    /// dispatched here ([feedback_objectkind-resolution-uniformity]).
    ShadowRoot,
    /// Document or anything without a recognised `NodeKind`.
    /// Chains directly to `Node.prototype`.
    OtherNode,
}

impl HostData {
    /// Return `true` if this `HostData` is bound AND `entity` is
    /// an Element (has the `TagType` component) in the bound
    /// world.  Returns `false` when unbound so pre-bind or
    /// post-unbind wrapper allocation can still decide a
    /// reasonable fallback prototype (`Node.prototype`).
    ///
    /// Takes `&self` (not `&mut self`) because the lookup is
    /// read-only — callers holding a shared borrow of `HostData`
    /// (e.g. `create_element_wrapper`'s prototype branch) cannot
    /// otherwise reach the world.
    ///
    /// # Aliasing contract
    ///
    /// Callers must not hold a live `&mut EcsDom` produced via
    /// [`Self::dom`] when invoking this method.  Creating the
    /// shared reference while a mutable reference to the same
    /// `EcsDom` exists would be undefined behaviour under the
    /// Rust aliasing rules.  In practice this is guaranteed by
    /// `create_element_wrapper`'s borrow discipline — it calls
    /// `is_element_entity` through `self.host_data.as_deref()`,
    /// so no `&mut` borrow of the DOM can be live at that call
    /// site.  Future callers must preserve the same ordering.
    #[allow(unsafe_code)]
    pub fn is_element_entity(&self, entity: Entity) -> bool {
        if !self.is_bound() {
            return false;
        }
        // SAFETY: `is_bound` implies `dom_ptr` is non-null and
        // points at the `EcsDom` supplied by the most recent
        // `bind()`.  The pointer lifetime is tied to that bind
        // window; callers must not drop or move the `EcsDom`
        // between bind and unbind (documented on `bind` itself).
        // The aliasing contract above guarantees no `&mut`
        // reference to the same `EcsDom` is live at the call
        // site, so creating a `&` here cannot violate Rust
        // aliasing rules.
        let dom = unsafe { &*self.dom_ptr };
        dom.world().get::<&elidex_ecs::TagType>(entity).is_ok()
    }

    /// Engine-component brand check: `entity` carries an
    /// `elidex_ecs::ShadowRoot` component in the currently-bound
    /// DOM.  Sibling of [`Self::is_element_entity`] with identical
    /// aliasing contract.
    ///
    /// Used to distinguish ShadowRoot wrappers from other DOM
    /// wrappers post-H-migration ([feedback_objectkind-resolution-uniformity]):
    /// both kinds are `ObjectKind::HostObject { entity_bits }` and
    /// the ECS component is the only discriminator.
    #[allow(unsafe_code)]
    pub fn is_shadow_root_entity(&self, entity: Entity) -> bool {
        if !self.is_bound() {
            return false;
        }
        // SAFETY: see `is_element_entity` — same contract.
        let dom = unsafe { &*self.dom_ptr };
        dom.world().get::<&elidex_ecs::ShadowRoot>(entity).is_ok()
    }

    /// ASCII-case-insensitive tag-name match — used by
    /// `create_element_wrapper`'s per-tag prototype dispatch
    /// (e.g. `<iframe>` → `HTMLIFrameElement.prototype`).
    ///
    /// Aliasing contract mirrors [`Self::is_element_entity`]: no
    /// `&mut EcsDom` must be live at the call site.
    #[allow(unsafe_code)]
    pub fn tag_matches_ascii_case(&self, entity: Entity, tag: &str) -> bool {
        if !self.is_bound() {
            return false;
        }
        // SAFETY: see `is_element_entity`.
        let dom = unsafe { &*self.dom_ptr };
        dom.world()
            .get::<&elidex_ecs::TagType>(entity)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case(tag))
    }

    /// Classify `entity` into a [`PrototypeKind`] used by
    /// `create_element_wrapper` to pick the appropriate wrapper
    /// prototype.  Probes a small set of ECS components
    /// (`TagType` → `Element` fast path for the ~99% case, then
    /// `ShadowRoot` to disambiguate shadow-root DF entities, then
    /// `node_kind` for the remaining non-Element node families)
    /// with a legacy-payload fallback (`TextContent` / `CommentData`
    /// / `DocTypeData`) for entities lacking an explicit `NodeKind`
    /// component.  Returns [`PrototypeKind::OtherNode`] when the
    /// `HostData` is not bound (pre-bind wrapper allocation paths).
    ///
    /// Aliasing contract mirrors [`Self::is_element_entity`] — the
    /// caller must not hold a live `&mut EcsDom` produced via
    /// [`Self::dom`] at the call site.
    #[allow(unsafe_code)]
    pub fn prototype_kind_for(&self, entity: Entity) -> PrototypeKind {
        if !self.is_bound() {
            return PrototypeKind::OtherNode;
        }
        // SAFETY: see `is_element_entity` — same contract.
        let dom = unsafe { &*self.dom_ptr };
        // Element has highest priority (matches the pre-PR4e
        // behaviour of `is_element_entity` short-circuit).  Shadow
        // root entities lack `TagType` so this probe naturally
        // short-circuits to the non-shadow path for the ~99% case;
        // the `ShadowRoot` probe below disambiguates SR-vs-plain-DF
        // only for the DF-typed minority without paying for an
        // extra lookup on every Element wrapper allocation.
        if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
            return PrototypeKind::Element;
        }
        // ShadowRoot entities carry `NodeKind::DocumentFragment`
        // but must route through `ShadowRoot.prototype` (not
        // `DocumentFragment.prototype`) for accurate brand checks
        // (`shadowRoot instanceof ShadowRoot === true`).  Probe
        // after the Element fast-path so non-shadow DF allocations
        // also benefit from the early TagType short-circuit.
        if dom.world().get::<&elidex_ecs::ShadowRoot>(entity).is_ok() {
            return PrototypeKind::ShadowRoot;
        }
        match dom.node_kind(entity) {
            Some(NodeKind::Text) => PrototypeKind::Text,
            Some(NodeKind::Comment) => PrototypeKind::OtherCharacterData,
            // CDATA / ProcessingInstruction would also be
            // CharacterData per WHATWG §4.10; they are not
            // created by the current parser but are listed here
            // so future support is a one-line add.
            Some(NodeKind::CdataSection | NodeKind::ProcessingInstruction) => {
                PrototypeKind::OtherCharacterData
            }
            Some(NodeKind::DocumentType) => PrototypeKind::DocumentType,
            Some(NodeKind::DocumentFragment) => PrototypeKind::DocumentFragment,
            None => {
                // Defensive inference for legacy entities that
                // carry CharacterData payload without an explicit
                // `NodeKind` component.  Mirrors the same
                // fallback in `EcsDom::clone_node_shallow`.
                if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
                    PrototypeKind::Element
                } else if dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok() {
                    PrototypeKind::Text
                } else if dom.world().get::<&elidex_ecs::CommentData>(entity).is_ok() {
                    PrototypeKind::OtherCharacterData
                } else if dom.world().get::<&elidex_ecs::DocTypeData>(entity).is_ok() {
                    PrototypeKind::DocumentType
                } else {
                    PrototypeKind::OtherNode
                }
            }
            _ => PrototypeKind::OtherNode,
        }
    }
}
