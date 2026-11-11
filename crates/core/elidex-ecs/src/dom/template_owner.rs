//! The inert template contents owner document (WHATWG HTML §4.12.3 *The
//! template element*, `#the-template-element`): the one rule that computes a
//! template's contents' node document, the predicate that reads its marker,
//! and its lifetime hook.
//!
//! The *associated inert template document* of a Document D is a Document
//! entity I linked from D by [`AssociatedInertTemplateDocument`] and marked
//! [`InertTemplateDocument`]: a forward-link component plus a marker, not a
//! side-store (both values are per-entity, `Send + Sync` and not per-VM
//! identity handles). I lives as long as D ([`EcsDom::destroy_entity`]
//! follows the link).

use hecs::Entity;

use super::EcsDom;
use crate::components::{AssociatedInertTemplateDocument, InertTemplateDocument, NodeKind};

impl EcsDom {
    /// WHATWG HTML §4.12.3 *appropriate template contents owner document*
    /// (`#appropriate-template-contents-owner-document`), steps 1–2, of the
    /// Document `document`.
    ///
    /// Step 1: if `document` is not a Document created by this algorithm
    /// (no [`InertTemplateDocument`] marker), it gets its *associated inert
    /// template document*, created on first use: step 1.1.1 creates a new
    /// Document (browsing context null) and marks it, step 1.1.3 links it from
    /// `document`; step 1.2 sets `document` to it. Step 1.1.2 ("if document is
    /// an HTML document, mark newDocument as an HTML document also") holds by
    /// construction: every elidex Document is an HTML document (no XML
    /// Document kind exists), so the new one is too, with no flag to set. A
    /// future XML Document kind (`#11-domparser-xml-real-parsing`) makes it a
    /// copy of that flag. Step 2: return `document`.
    ///
    /// Idempotent over the pair: `f(D) = f(I) = I`, so a caller may pass either.
    /// Step 1.1.1 creates the new Document "given document's relevant realm";
    /// elidex has one realm per `EcsDom`.
    pub fn appropriate_template_contents_owner_document(&mut self, document: Entity) -> Entity {
        debug_assert!(
            self.is_live_document(document),
            "appropriate_template_contents_owner_document: `document` must be a live Document"
        );
        if self.is_inert_template_document(document) {
            return document;
        }
        if let Ok(link) = self.world.get::<&AssociatedInertTemplateDocument>(document) {
            let inert = link.0;
            debug_assert!(
                self.is_live_document(inert)
                    && self.world.get::<&InertTemplateDocument>(inert).is_ok(),
                "the associated inert template document is a live marked Document \
                 (destroyed only with its document)"
            );
            return inert;
        }
        let inert = self.create_document_node();
        let _ = self.world.insert_one(inert, InertTemplateDocument);
        let _ = self
            .world
            .insert_one(document, AssociatedInertTemplateDocument(inert));
        inert
    }

    /// Whether `document` is a Document "created by" HTML §4.12.3's
    /// *appropriate template contents owner document* algorithm (step 1.1.1),
    /// i.e. an inert template document.
    #[must_use]
    pub fn is_inert_template_document(&self, document: Entity) -> bool {
        matches!(self.node_kind(document), Some(NodeKind::Document))
            && self.world.get::<&InertTemplateDocument>(document).is_ok()
    }

    /// The lifetime hook of [`Self::destroy_entity`]: the inert template
    /// document linked from `entity`, which is destroyed with it. No spec
    /// counterpart (the spec's lifetime is garbage collection).
    pub(super) fn linked_inert_template_document(&self, entity: Entity) -> Option<Entity> {
        self.world
            .get::<&AssociatedInertTemplateDocument>(entity)
            .ok()
            .map(|link| link.0)
    }
}
