//! WHATWG HTML §8.1.3.4 "scripting is disabled for a platform object" —
//! the engine-independent composition of the settings-level rule
//! ([`elidex_plugin::sandbox`]) with the platform-object clauses, evaluated
//! over the ECS DOM. Script engines marshal their bound-document /
//! sandbox-flag state into [`scripting_disabled_for_platform_object`]
//! rather than composing the clauses host-side (Layering mandate: `vm/host/`
//! is marshalling only; §8.1.3.4 is an HTML algorithm and lives here).

use elidex_ecs::{EcsDom, Entity, NodeKind};
use elidex_plugin::IframeSandboxFlags;

/// WHATWG HTML §8.1.3.4 "scripting is disabled for a platform object"
/// (`html#enabling-and-disabling-scripting`) — the canonical predicate
/// behind *the event handler processing algorithm* step 1
/// (`html#the-event-handler-processing-algorithm`: "If scripting is
/// disabled for eventTarget, then return"). Step 1 gates event HANDLERS
/// only; plain `addEventListener` listeners are never suppressed. `target`
/// is the dispatch target's entity; `None` = a non-entity platform object
/// (an engine-local listener home), which gets the settings-level check
/// only. `bound_document` is the active document of the engine's (single)
/// browsing context; `None` = unbound, which fails open (no document
/// context to evaluate — dispatch requires a bound engine).
///
/// Composition (the settings-level rule lives in [`elidex_plugin::sandbox`]):
///
/// - **settings-level**: `scripting_enabled(sandbox_flags)`;
/// - **clause (b)**: target is a `Node` whose node document is not the
///   bound document — in the single-browsing-context engine model exactly
///   those documents have a null browsing context (`DOMParser` /
///   cloned / fragment-parse documents);
/// - **clause (c)** (`Window`) never fires today: the Window entity is
///   not a Node and its associated document IS the bound document while
///   bound.
///
/// **Effective node document (adopt-equivalent, clause (b))**: elidex's
/// insertion path does not yet run the DOM §4.2.3 insert adoption
/// step ("adopt node into parent's node document" —
/// `EcsDom::append_child` relinks without re-homing `AssociatedDocument`),
/// so a *connected* node's stored owner pointer is unreliable in BOTH
/// directions (a node parsed into a throwaway document then appended into
/// the bound tree keeps its foreign owner; a bound-created node appended
/// into a `DOMParser` / null-BC document keeps the bound owner). Clause
/// (b) therefore resolves the node's *effective node document* from its
/// composed tree root — the value a spec-correct adopt would have written
/// — via one rule: if the composed tree root IS a Document (the node is
/// connected, the same query `Node.isConnected` uses) that root is the
/// effective node document; otherwise (a detached node) fall back to
/// `owner_document(entity)` (`AssociatedDocument`). Clause (b) suppresses
/// iff the effective node document is not the bound document. The
/// underlying missing insertion-adoption is carved as defer slot
/// `#11-cross-document-adopt-on-insert`; when it lands `AssociatedDocument`
/// becomes reliable for connected nodes and this rule collapses to a plain
/// `owner_document(entity) != bound_document` compare.
///
/// A Node whose effective node document is unresolvable fails OPEN (not
/// suppressed): such nodes belong to the bound document's realm by
/// construction, and failing closed would regress handlers on
/// parser-built nodes that scripts detach. Caveats: detached-iframe
/// documents = unreachable in the single-BC model; the `<template>`
/// contents false-negative rides `#11-template-contents-owner-document`.
/// The composed-tree-root proxy is correct for the reachable cases — a
/// node's live tree position, including a `DOMParser` node appended into
/// the active document — but is a best-effort approximation for any node
/// MOVED between documents, because elidex does not implement DOM §4.2.3
/// insertion adoption (`AssociatedDocument` is not re-homed on a
/// cross-document insert). The known imperfect facets — adopt-then-remove
/// (over-suppress), a live main-document node appended into a detached
/// foreign subtree (under-suppress), and their reverses — are ALL bounded
/// by this and deferred to `#11-cross-document-adopt-on-insert`; no single
/// tree / owner-document rule resolves every facet (each refinement trades
/// which facet is wrong), so the proxy is not refined per-facet. It fails
/// CLOSED on the suppress-vs-run ambiguity where it can (the safe
/// direction for a security gate).
///
/// Clause (b) applies to objects implementing `Node` only — Window /
/// Worker / OffscreenCanvas entities (`is_node() == false`) and non-DOM
/// entities fall through to the settings-level verdict.
/// [`EcsDom::node_kind_inferred`] (the brand-check convention) also covers
/// legacy entities carrying `TagType`/`TextContent` but no `NodeKind`
/// component, so a NodeKind-less node of a null-BC document is still
/// suppressed.
#[must_use]
pub fn scripting_disabled_for_platform_object(
    dom: &EcsDom,
    target: Option<Entity>,
    bound_document: Option<Entity>,
    sandbox_flags: Option<IframeSandboxFlags>,
) -> bool {
    if !elidex_plugin::sandbox::scripting_enabled(sandbox_flags) {
        return true;
    }
    let (Some(entity), Some(document)) = (target, bound_document) else {
        return false;
    };
    match dom.node_kind_inferred(entity) {
        Some(NodeKind::Document) => entity != document,
        Some(kind) if kind.is_node() => {
            // Effective node document via the composed tree root
            // (adopt-equivalent proxy — see the doc-comment). A connected
            // node's node document IS its tree-root Document (whatever a
            // stale `AssociatedDocument` says, in either direction); a
            // detached node falls back to `owner_document`
            // (`AssociatedDocument`). `None` = unresolvable → fail open.
            let root = dom.find_tree_root_composed(entity);
            let effective_document =
                if matches!(dom.node_kind_inferred(root), Some(NodeKind::Document)) {
                    Some(root)
                } else {
                    dom.owner_document(entity)
                };
            effective_document.is_some_and(|node_doc| node_doc != document)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    /// Bound document + a second (throwaway, `DOMParser`-like) document
    /// with one element owned by it.
    fn two_doc_fixture() -> (EcsDom, Entity, Entity, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let doc2 = dom.create_document_node();
        let foreign = dom.create_element_with_owner("div", Attributes::default(), Some(doc2));
        assert!(dom.append_child(doc2, foreign));
        (dom, doc, doc2, foreign)
    }

    #[test]
    fn sandboxed_flags_disable_regardless_of_target() {
        let (dom, doc, _doc2, _foreign) = two_doc_fixture();
        let flags = Some(IframeSandboxFlags::empty());
        // Settings-level clause fires even for the bound document itself
        // and for a `None` target.
        assert!(scripting_disabled_for_platform_object(
            &dom,
            Some(doc),
            Some(doc),
            flags
        ));
        assert!(scripting_disabled_for_platform_object(
            &dom,
            None,
            Some(doc),
            flags
        ));
    }

    #[test]
    fn none_target_or_unbound_document_fails_open() {
        let (dom, doc, _doc2, foreign) = two_doc_fixture();
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            None,
            Some(doc),
            None
        ));
        // Unbound (no document context): fail open even for a clause-(b)
        // candidate node.
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            Some(foreign),
            None,
            None
        ));
    }

    #[test]
    fn document_targets_compare_by_identity() {
        let (dom, doc, doc2, _foreign) = two_doc_fixture();
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            Some(doc),
            Some(doc),
            None
        ));
        assert!(scripting_disabled_for_platform_object(
            &dom,
            Some(doc2),
            Some(doc),
            None
        ));
    }

    #[test]
    fn foreign_document_node_is_suppressed() {
        let (dom, doc, _doc2, foreign) = two_doc_fixture();
        assert!(scripting_disabled_for_platform_object(
            &dom,
            Some(foreign),
            Some(doc),
            None
        ));
    }

    #[test]
    fn appended_foreign_node_is_adopt_equivalent_not_suppressed() {
        let (mut dom, doc, doc2, foreign) = two_doc_fixture();
        // Move the foreign node into the bound document's tree. elidex's
        // `append_child` does NOT adopt (`AssociatedDocument` still points
        // at doc2) — the predicate's tree-root rule must treat it as
        // adopted (DOM §4.2.3 insert step 7 substep 1, "adopt node into
        // parent's node document").
        assert!(dom.remove_child(doc2, foreign));
        let container = dom.create_element_with_owner("div", Attributes::default(), Some(doc));
        assert!(dom.append_child(doc, container));
        assert!(dom.append_child(container, foreign));
        assert_eq!(
            dom.get_associated_document(foreign),
            Some(doc2),
            "fixture: append must NOT have adopted (else this test pins nothing)"
        );
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            Some(foreign),
            Some(doc),
            None
        ));
        // Removed again: back to the stale-owner comparison → suppressed.
        assert!(dom.remove_child(container, foreign));
        assert!(scripting_disabled_for_platform_object(
            &dom,
            Some(foreign),
            Some(doc),
            None
        ));
    }

    #[test]
    fn bound_created_node_appended_into_foreign_doc_is_suppressed() {
        // Mirror of `appended_foreign_node_is_adopt_equivalent_not_suppressed`:
        // a node CREATED by the bound document then appended INTO the
        // foreign (`DOMParser`-like) tree. `append_child` does NOT adopt,
        // so `AssociatedDocument` still points at the bound document
        // (stale) — the directional rule would read that and NOT suppress,
        // but the effective node document is the foreign tree root, whose
        // browsing context is null → clause (b) SUPPRESSES.
        let (mut dom, doc, doc2, _foreign) = two_doc_fixture();
        let el = dom.create_element_with_owner("div", Attributes::default(), Some(doc));
        assert!(dom.append_child(doc2, el));
        assert_eq!(
            dom.get_associated_document(el),
            Some(doc),
            "fixture: append must NOT have adopted (else this test pins nothing)"
        );
        assert!(scripting_disabled_for_platform_object(
            &dom,
            Some(el),
            Some(doc),
            None
        ));
    }

    #[test]
    fn bound_document_element_not_suppressed_connected_or_detached() {
        let (mut dom, doc, _doc2, _foreign) = two_doc_fixture();
        let el = dom.create_element_with_owner("div", Attributes::default(), Some(doc));
        // Detached, owner = bound document.
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            Some(el),
            Some(doc),
            None
        ));
        assert!(dom.append_child(doc, el));
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            Some(el),
            Some(doc),
            None
        ));
    }

    #[test]
    fn unresolvable_owner_fails_open() {
        let (mut dom, doc, _doc2, _foreign) = two_doc_fixture();
        // Legacy-style element: no `AssociatedDocument`, detached — the
        // owner is unresolvable and the verdict fails open.
        let orphan = dom.create_element("div", Attributes::default());
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            Some(orphan),
            Some(doc),
            None
        ));
    }
}
