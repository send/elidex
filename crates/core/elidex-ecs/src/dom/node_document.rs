//! The node document at creation (WHATWG DOM §4.4 *node document*: "set upon
//! creation"): the one creation entry
//! [`EcsDom::node_document_for_new_nodes`], the owner token [`NewNodeOwner`]
//! every production creator takes, and the stamp-only read the entry builds on.

use hecs::{DynamicBundle, Entity, EntityBuilder};

use super::EcsDom;
use crate::components::{AssociatedDocument, FragmentParseRoot, NodeKind};

/// The node document a newly created node receives (WHATWG DOM §4.4 *node
/// document*).
///
/// Every production node creator takes one. It has no public constructor:
/// outside `elidex-ecs` it is obtained only from
/// [`EcsDom::node_document_for_new_nodes`], so production code cannot ask for
/// an ownerless node — it names a source, and a node is ownerless only when
/// that source has no node document. Ownerless creation is test-only by
/// construction (the argumentless fixture creators, compiled only under
/// `cfg(any(test, feature = "fixtures"))`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewNodeOwner(Option<Entity>);

impl NewNodeOwner {
    /// An ownerless node: the fixture creators' owner.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) const NONE: Self = Self(None);

    /// An ownerless node, for fixtures outside `elidex-ecs` that call a
    /// production creator (`create_element_ns`, `create_document_type`,
    /// `create_attribute`, ...) with no node document. Test-only by
    /// construction, like the argumentless fixture creators.
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    pub fn ownerless_fixture() -> Self {
        Self::NONE
    }

    /// The document the new node is stamped with, `None` for an ownerless node.
    #[must_use]
    pub fn document(self) -> Option<Entity> {
        self.0
    }
}

impl EcsDom {
    /// The node document a node created on behalf of `source`, or inserted
    /// under `source`, receives (WHATWG DOM §4.4 *node document*, "set upon
    /// creation"; DOM §4.2.3 *insert* step 7.1's adopt target).
    ///
    /// `source`'s node document, except when `source` is the HTML §13.4
    /// fragment-parse root (*HTML fragment parsing algorithm*,
    /// `#html-fragment-parsing-algorithm`, steps 14–15): root stands for the
    /// insertion target redirection map's fragment, so a node created on its
    /// behalf (text, comments, top-level elements) gets the fragment's node
    /// document, the context's. This is how HTML §13.2.6.1
    /// (`#creating-and-inserting-nodes`) resolves *create an element for the
    /// token* step 3 ("intendedParent's node document"), *insert a character*
    /// step 5 and *insert a comment* step 4 ("targetParent's node document"
    /// of the *adjusted insertion location*, steps 3–5).
    #[must_use]
    pub fn node_document_for_new_nodes(&self, source: Entity) -> NewNodeOwner {
        if let Ok(root) = self.world.get::<&FragmentParseRoot>(source) {
            return NewNodeOwner(root.redirect_document);
        }
        NewNodeOwner(self.node_document(source))
    }

    /// WHATWG DOM §4.4 *node document* (`#concept-node-document`): a Document
    /// is its own; any other node reads its [`AssociatedDocument`] if that is a
    /// live Document, else `None`. Reads the stored fact only — no position
    /// derivation.
    pub(crate) fn node_document(&self, node: Entity) -> Option<Entity> {
        if !self.contains(node) {
            return None;
        }
        if matches!(self.node_kind(node), Some(NodeKind::Document)) {
            return Some(node);
        }
        self.get_associated_document(node)
            .filter(|&doc| self.contains(doc) && self.node_kind(doc) == Some(NodeKind::Document))
    }

    /// Spawn a node bundle with its node document in the spawn tuple (DOM
    /// §4.4: the node document is set upon creation), so the node lands in its
    /// final archetype directly. `owner` passes the liveness check of
    /// [`Self::validate_owner_document`] first.
    pub(super) fn spawn_node(&mut self, bundle: impl DynamicBundle, owner: NewNodeOwner) -> Entity {
        let owner = self.validate_owner_document(owner.0);
        let mut builder = EntityBuilder::new();
        builder.add_bundle(bundle);
        if let Some(doc) = owner {
            builder.add(AssociatedDocument(doc));
        }
        self.world.spawn(builder.build())
    }
}
