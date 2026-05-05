//! WHATWG DOM §4.4 equality + position-comparison primitives.
//!
//! Single source of truth for `Node.isEqualNode` and
//! `Node.compareDocumentPosition` semantics.  Both `elidex-dom-api`
//! handlers and `elidex-js` VM-side natives delegate here — the
//! algorithm proper is engine-independent and lives next to
//! [`tree_clone`](super::tree_clone) by the same layering rationale:
//! DOM tree algorithms belong in `elidex-ecs`, JS-binding wrappers
//! are marshalling-only.
//!
//! Both functions walk iteratively (`Vec`-backed stack) so a
//! malicious depth ≤ [`MAX_ANCESTOR_DEPTH`] cannot overflow the Rust
//! call stack — same convention used by `clone_children_recursive`
//! and `despawn_subtree`.

use crate::components::{AttrData, Attributes, CommentData, DocTypeData, NodeKind, TextContent};
use hecs::Entity;

use super::EcsDom;

/// `compareDocumentPosition` returned bit: the two nodes are in
/// disconnected trees (WHATWG DOM §4.4).
pub const DOCUMENT_POSITION_DISCONNECTED: u32 = 0x01;
/// `compareDocumentPosition` returned bit: `other` precedes `this`
/// in tree order.
pub const DOCUMENT_POSITION_PRECEDING: u32 = 0x02;
/// `compareDocumentPosition` returned bit: `other` follows `this`
/// in tree order.
pub const DOCUMENT_POSITION_FOLLOWING: u32 = 0x04;
/// `compareDocumentPosition` returned bit: `other` is an ancestor
/// of `this`.
pub const DOCUMENT_POSITION_CONTAINS: u32 = 0x08;
/// `compareDocumentPosition` returned bit: `other` is a descendant
/// of `this`.
pub const DOCUMENT_POSITION_CONTAINED_BY: u32 = 0x10;
/// `compareDocumentPosition` returned bit: the relative ordering is
/// implementation-specific (set whenever PRECEDING / FOLLOWING is
/// the only stable signal we can offer — disconnected nodes and
/// Attr-vs-Attr same-owner comparisons).
pub const DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: u32 = 0x20;

impl EcsDom {
    /// Structural deep equality of two nodes (WHATWG DOM §4.4
    /// "equals" algorithm).
    ///
    /// Iterative `Vec<(Entity, Entity)>` stack — matches the
    /// explicit-stack pattern used by `despawn_subtree` and
    /// `clone_children_recursive`, so a malicious deep DOM cannot
    /// overflow the Rust call stack.
    ///
    /// Uses [`Self::node_kind_inferred`] so legacy entities (payload
    /// component but no explicit `NodeKind`) still discriminate
    /// correctly — comparing two legacy entities of different
    /// payload kinds (e.g. legacy Text vs legacy Comment, both
    /// reporting `node_kind == None`) would otherwise treat them as
    /// equal because the per-kind match arms below would both be
    /// skipped.
    ///
    /// Element comparison: tag name match (case-sensitive — the
    /// parser canonicalises HTML tags to lowercase before they reach
    /// the ECS) plus attribute-set match (order-independent).
    /// Character-data nodes compare their payload string.
    /// `DocumentType` compares name / public-id / system-id.
    /// `Document` and `DocumentFragment` carry no payload so the
    /// comparison reduces to children equality.
    #[must_use]
    pub fn nodes_equal(&self, a: Entity, b: Entity) -> bool {
        let mut stack: Vec<(Entity, Entity)> = vec![(a, b)];
        while let Some((a, b)) = stack.pop() {
            let kind = self.node_kind_inferred(a);
            if kind != self.node_kind_inferred(b) {
                return false;
            }
            let tags_match = self.with_tag_name(a, |ta| self.with_tag_name(b, |tb| ta == tb));
            if !tags_match {
                return false;
            }
            let payload_equal = match kind {
                Some(NodeKind::Text | NodeKind::CdataSection) => text_content_equal(self, a, b),
                Some(NodeKind::Comment) => comment_data_equal(self, a, b),
                Some(NodeKind::DocumentType) => doctype_data_equal(self, a, b),
                _ => true,
            };
            if !payload_equal {
                return false;
            }
            if !attributes_equal(self, a, b) {
                return false;
            }
            let kids_a: Vec<Entity> = self.children_iter(a).collect();
            let kids_b: Vec<Entity> = self.children_iter(b).collect();
            if kids_a.len() != kids_b.len() {
                return false;
            }
            for (ca, cb) in kids_a.iter().zip(kids_b.iter()).rev() {
                stack.push((*ca, *cb));
            }
        }
        true
    }

    /// WHATWG DOM §4.4 `compareDocumentPosition`.  Returns the
    /// bitmask of `DOCUMENT_POSITION_*` constants describing
    /// `other`'s position relative to `this`.
    ///
    /// Behaviour summary (per WHATWG §4.4 + §5.4 step 3 + §4.2.8
    /// step 5 for Attr operands):
    ///
    /// - `this == other` → `0`.
    /// - Two `Attribute` nodes with the same `ownerElement`:
    ///   `IMPLEMENTATION_SPECIFIC | (PRECEDING|FOLLOWING)` ordered
    ///   by entity bits (Attr entities are allocated in attribute
    ///   insertion order, so the natural ECS order matches WHATWG
    ///   "attribute list order" closely enough for the
    ///   "implementation-specific" contract — no stronger guarantee
    ///   is required).
    /// - Either operand is an `Attribute`: replace it with its
    ///   `owner_element` for the tree comparison so an Attr in the
    ///   tree compares as if rooted at its owning Element.
    /// - Light-tree containment (shadow boundaries are NOT crossed —
    ///   Phase 2 convention; full composed-tree semantics land with
    ///   shadow DOM completion).
    /// - Disconnected operands (different tree roots): `DISCONNECTED
    ///   | IMPLEMENTATION_SPECIFIC | (PRECEDING|FOLLOWING)` ordered
    ///   by entity bits for antisymmetric stability — `cmp(a,b) ^
    ///   cmp(b,a) == PRECEDING | FOLLOWING`.
    /// - Otherwise: tree-order comparison via [`Self::tree_order_cmp`]
    ///   yields `PRECEDING` / `FOLLOWING` / `0`.
    ///
    /// Pure read-only (`&self`) — safe to call alongside other
    /// reads.
    #[must_use]
    pub fn compare_document_position(&self, this: Entity, other: Entity) -> u32 {
        if this == other {
            return 0;
        }

        let this_owner = if self.node_kind(this) == Some(NodeKind::Attribute) {
            attr_owner(self, this)
        } else {
            None
        };
        let other_owner = if self.node_kind(other) == Some(NodeKind::Attribute) {
            attr_owner(self, other)
        } else {
            None
        };

        if let (Some(to), Some(oo)) = (this_owner, other_owner) {
            if to == oo {
                let dir = if this.to_bits() < other.to_bits() {
                    DOCUMENT_POSITION_PRECEDING
                } else {
                    DOCUMENT_POSITION_FOLLOWING
                };
                return DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC | dir;
            }
        }

        let effective_this = this_owner.unwrap_or(this);
        let effective_other = other_owner.unwrap_or(other);

        // Roots first: a single ancestor walk per operand decides
        // disconnection, which would otherwise force the
        // is_light_tree_ancestor_or_self probes below to walk the
        // full chain twice each just to discover the operands aren't
        // related.
        let root_this = self.find_tree_root(effective_this);
        let root_other = self.find_tree_root(effective_other);
        if root_this != root_other {
            let dir = if effective_this.to_bits() < effective_other.to_bits() {
                DOCUMENT_POSITION_FOLLOWING
            } else {
                DOCUMENT_POSITION_PRECEDING
            };
            return DOCUMENT_POSITION_DISCONNECTED
                | DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC
                | dir;
        }

        if effective_other != effective_this
            && self.is_light_tree_ancestor_or_self(effective_other, effective_this)
        {
            return DOCUMENT_POSITION_CONTAINS | DOCUMENT_POSITION_PRECEDING;
        }
        if effective_this != effective_other
            && self.is_light_tree_ancestor_or_self(effective_this, effective_other)
        {
            return DOCUMENT_POSITION_CONTAINED_BY | DOCUMENT_POSITION_FOLLOWING;
        }

        match self.tree_order_cmp(effective_this, effective_other) {
            std::cmp::Ordering::Less => DOCUMENT_POSITION_FOLLOWING,
            std::cmp::Ordering::Greater => DOCUMENT_POSITION_PRECEDING,
            std::cmp::Ordering::Equal => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// File-private helpers
// ---------------------------------------------------------------------------

fn text_content_equal(dom: &EcsDom, a: Entity, b: Entity) -> bool {
    let ta = dom.world().get::<&TextContent>(a).ok();
    let tb = dom.world().get::<&TextContent>(b).ok();
    ta.as_deref().map(|t| &t.0) == tb.as_deref().map(|t| &t.0)
}

fn comment_data_equal(dom: &EcsDom, a: Entity, b: Entity) -> bool {
    let ca = dom.world().get::<&CommentData>(a).ok();
    let cb = dom.world().get::<&CommentData>(b).ok();
    ca.as_deref().map(|c| &c.0) == cb.as_deref().map(|c| &c.0)
}

fn doctype_data_equal(dom: &EcsDom, a: Entity, b: Entity) -> bool {
    let da = dom.world().get::<&DocTypeData>(a).ok();
    let db = dom.world().get::<&DocTypeData>(b).ok();
    match (da.as_deref(), db.as_deref()) {
        (Some(x), Some(y)) => {
            x.name == y.name && x.public_id == y.public_id && x.system_id == y.system_id
        }
        (None, None) => true,
        _ => false,
    }
}

fn attributes_equal(dom: &EcsDom, a: Entity, b: Entity) -> bool {
    let attrs_a = dom.world().get::<&Attributes>(a).ok();
    let attrs_b = dom.world().get::<&Attributes>(b).ok();
    match (attrs_a, attrs_b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().all(|(k, v)| b.get(k) == Some(v))
        }
        // One side has an `Attributes` component, the other does not —
        // treat an absent component as an empty attribute set so two
        // freshly-cloned Elements compare as equal regardless of
        // whether `clone_attributes` chose to skip the empty insert.
        (Some(a), None) => a.is_empty(),
        (None, Some(b)) => b.is_empty(),
    }
}

fn attr_owner(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    dom.world()
        .get::<&AttrData>(entity)
        .ok()
        .and_then(|a| a.owner_element)
}
