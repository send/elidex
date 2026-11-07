//! WHATWG HTML §8.1.3.4 "scripting is disabled for a platform object" —
//! the engine-independent composition of the settings-level rule
//! ([`elidex_plugin::sandbox`]) with the platform-object clauses, evaluated
//! over the ECS DOM. Script engines marshal their bound-document /
//! sandbox-flag state into [`scripting_disabled_for_platform_object`]
//! rather than composing the clauses host-side (Layering mandate: `vm/host/`
//! is marshalling only; §8.1.3.4 is an HTML algorithm and lives here).

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::SandboxingFlagSet;

/// WHATWG HTML §8.1.3.4 "scripting is disabled for a platform object"
/// (`html#enabling-and-disabling-scripting`) — the compile gate of an event
/// handler's value: *get the current value of the event handler*
/// (`html#getting-the-current-value-of-the-event-handler`) step 3.2, "If
/// document's browsing context is null, or document's active sandboxing flag
/// set has its sandboxed scripts browsing context flag set, then return
/// null." (webref, current text). The step is consulted only for an *internal
/// raw uncompiled handler* (see `reconcile_event_handler`); a callback value
/// is never gated, and the processing algorithm has no §8.1.3.4 check at
/// invocation (its step 1 is WebDriver BiDi, which elidex does not have).
///
/// Step 3.2 as written and "scripting is disabled for *el's node document*"
/// give the same result for every document the engine can hold: it has no
/// WebDriver BiDi and no user scripting switch, so §8.1.3.4's settings clause
/// reduces to the sandboxed scripts flag of the (single) realm's document,
/// which is the bound document's; a document other than the bound one has a
/// null browsing context in the single-browsing-context model, and for the
/// bound document the second disjunct reads the same flag set. Which reading
/// the gate implements is N4's (the step-3.2 reading question; N4 of the IBP-sandbox node
/// memo `docs/plans/2026-09-ibp-sandbox.md`).
///
/// `target` is the handler's target entity (an element, or the Window
/// entity for a `WindowEventHandlers` attribute on `<body>`); `bound_document`
/// is the active document of the engine's (single) browsing context; callers
/// run on a bound engine, so both are always present.
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
/// **Node document (clause (b))**: HTML §8.1.8.1 *get the current value of
/// the event handler* (`#getting-the-current-value-of-the-event-handler`)
/// step 3.1, "Let document be element's node document". elidex reads the
/// stored fact, `EcsDom::node_document` (DOM §4.4 *node document*): set at
/// the node's creation and re-homed by DOM §4.5 *adopt* at every insertion
/// (DOM §4.2.3 *insert* step 7.1), so it follows a node moved between
/// documents in either direction, connected or detached, and keeps its last
/// document after a removal. Clause (b) fires iff that document is not the
/// bound document; a Document is its own node document.
///
/// A Node without a node document (its document was destroyed): the node
/// clause is false (the settings clause has already been evaluated), as it
/// was for an unresolvable document before. Caveats: detached-iframe
/// documents = unreachable in the single-BC model; the `<template>` contents
/// share the template's node document (no inert document yet), which rides
/// `#11-template-contents-owner-document`.
///
/// Clause (b) applies to objects implementing `Node` only — Window /
/// Worker / OffscreenCanvas entities (`is_node() == false`) and non-DOM
/// entities fall through to the settings-level verdict.
/// [`EcsDom::node_kind_inferred`] (the brand-check convention) also covers
/// legacy entities carrying `TagType`/`TextContent` but no `NodeKind`
/// component, so a NodeKind-less node of a null-BC document is still
/// gated.
#[must_use]
pub fn scripting_disabled_for_platform_object(
    dom: &EcsDom,
    target: Entity,
    bound_document: Entity,
    sandbox_flags: SandboxingFlagSet,
) -> bool {
    if !elidex_plugin::sandbox::scripting_enabled(sandbox_flags) {
        return true;
    }
    // A Document is its own node document (`target != bound_document`).
    dom.node_kind_inferred(target)
        .is_some_and(|kind| kind.is_node())
        && dom
            .node_document(target)
            .is_some_and(|node_doc| node_doc != bound_document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    const UNSANDBOXED: SandboxingFlagSet = SandboxingFlagSet::unsandboxed();

    /// Bound document + a second (throwaway, `DOMParser`-like) document
    /// with one element owned by it.
    fn two_doc_fixture() -> (EcsDom, Entity, Entity, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let doc2 = dom.create_document_node();
        let foreign = dom.create_element_with_owner(
            "div",
            Attributes::default(),
            dom.node_document_for_new_nodes(doc2),
        );
        assert!(dom.append_child(doc2, foreign));
        (dom, doc, doc2, foreign)
    }

    #[test]
    fn sandboxed_flags_disable_regardless_of_target() {
        let (dom, doc, _doc2, foreign) = two_doc_fixture();
        let flags = SandboxingFlagSet::all_represented();
        // Settings-level clause fires even for the bound document itself.
        assert!(scripting_disabled_for_platform_object(
            &dom, doc, doc, flags
        ));
        // ... and the settings clause decides for a foreign node too.
        assert!(scripting_disabled_for_platform_object(
            &dom, foreign, doc, flags
        ));
    }

    #[test]
    fn document_targets_compare_by_identity() {
        let (dom, doc, doc2, _foreign) = two_doc_fixture();
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            doc,
            doc,
            UNSANDBOXED
        ));
        assert!(scripting_disabled_for_platform_object(
            &dom,
            doc2,
            doc,
            UNSANDBOXED
        ));
    }

    #[test]
    fn foreign_document_node_is_suppressed() {
        let (dom, doc, _doc2, foreign) = two_doc_fixture();
        assert!(scripting_disabled_for_platform_object(
            &dom,
            foreign,
            doc,
            UNSANDBOXED
        ));
    }

    #[test]
    fn appended_foreign_node_is_adopted_not_suppressed() {
        let (mut dom, doc, doc2, foreign) = two_doc_fixture();
        // Move the foreign node into the bound document's tree: the append
        // adopts it (DOM §4.2.3 insert step 7.1, "adopt node into parent's
        // node document"), so its node document is the bound document.
        assert!(dom.remove_child(doc2, foreign));
        let container = dom.create_element_with_owner(
            "div",
            Attributes::default(),
            dom.node_document_for_new_nodes(doc),
        );
        assert!(dom.append_child(doc, container));
        assert!(dom.append_child(container, foreign));
        assert_eq!(dom.node_document(foreign), Some(doc), "adopted on insert");
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            foreign,
            doc,
            UNSANDBOXED
        ));
        // Removed again: a removal does not adopt, so the node keeps the
        // bound document → still not suppressed.
        assert!(dom.remove_child(container, foreign));
        assert_eq!(dom.node_document(foreign), Some(doc));
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            foreign,
            doc,
            UNSANDBOXED
        ));
    }

    #[test]
    fn bound_created_node_appended_into_foreign_doc_is_suppressed() {
        // Mirror of `appended_foreign_node_is_adopted_not_suppressed`: a node
        // CREATED by the bound document then appended INTO the foreign
        // (`DOMParser`-like) tree is adopted into it; that document's
        // browsing context is null → clause (b) SUPPRESSES.
        let (mut dom, doc, doc2, _foreign) = two_doc_fixture();
        let el = dom.create_element_with_owner(
            "div",
            Attributes::default(),
            dom.node_document_for_new_nodes(doc),
        );
        assert!(dom.append_child(doc2, el));
        assert_eq!(dom.node_document(el), Some(doc2), "adopted on insert");
        assert!(scripting_disabled_for_platform_object(
            &dom,
            el,
            doc,
            UNSANDBOXED
        ));
    }

    #[test]
    fn bound_document_element_not_suppressed_connected_or_detached() {
        let (mut dom, doc, _doc2, _foreign) = two_doc_fixture();
        let el = dom.create_element_with_owner(
            "div",
            Attributes::default(),
            dom.node_document_for_new_nodes(doc),
        );
        // Detached, owner = bound document.
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            el,
            doc,
            UNSANDBOXED
        ));
        assert!(dom.append_child(doc, el));
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            el,
            doc,
            UNSANDBOXED
        ));
    }

    #[test]
    fn unresolvable_owner_fails_open() {
        let (mut dom, doc, _doc2, _foreign) = two_doc_fixture();
        // A fixture element without a node document (production cannot
        // create one): the node document is unresolvable and the verdict
        // fails open.
        let orphan = dom.create_element("div", Attributes::default());
        assert!(!scripting_disabled_for_platform_object(
            &dom,
            orphan,
            doc,
            UNSANDBOXED
        ));
    }
}
