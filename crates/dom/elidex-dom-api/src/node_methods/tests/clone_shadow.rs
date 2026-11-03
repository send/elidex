//! Shadow-honor clone tests (DOM clone-a-node step 6) — split out from
//! `clone.rs` (~1000-line file convention).

use super::*;
use crate::test_util::entity_of as cloned_entity;
use elidex_custom_elements::{CEState, CustomElementState};
use elidex_ecs::{ShadowInit, ShadowRootMode};

// -----------------------------------------------------------------------
// Shadow honor — DOM §4.4 clone-a-node step 6, applied per node via
// step 5's re-entry (descendant hosts) and regardless of the subtree
// flag (shallow clones replicate clonable shadow trees too; only the
// shadow children's own clone depth follows the flag, step 6.7).
// -----------------------------------------------------------------------

fn clonable_init() -> ShadowInit {
    ShadowInit {
        clonable: true,
        ..ShadowInit::default()
    }
}

#[test]
fn clone_node_deep_replicates_descendant_shadow_host() {
    let (mut dom, mut session) = setup();
    let root = dom.create_element("div", Attributes::default());
    let host = dom.create_element("section", Attributes::default());
    dom.append_child(root, host);
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let span = dom.create_element("span", Attributes::default());
    let text = dom.create_text("shadowed");
    dom.append_child(span, text);
    dom.append_child(sr, span);
    wrap(root, &mut session);

    let r = CloneNode
        .invoke(root, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let cloned_root = cloned_entity(&r, &session);
    let kids = dom.children(cloned_root);
    assert_eq!(kids.len(), 1);
    let cloned_host = kids[0];
    let cloned_sr = dom
        .get_shadow_root(cloned_host)
        .expect("descendant clonable shadow root replicated on deep clone");
    let shadow_kids = dom.children(cloned_sr);
    assert_eq!(shadow_kids.len(), 1, "shadow children deep-cloned");
    let span_kids = dom.children(shadow_kids[0]);
    assert_eq!(span_kids.len(), 1, "deep flag recurses into shadow tree");
}

#[test]
fn clone_node_deep_skips_non_clonable_descendant_shadow() {
    let (mut dom, mut session) = setup();
    let root = dom.create_element("div", Attributes::default());
    let host = dom.create_element("section", Attributes::default());
    dom.append_child(root, host);
    let _sr = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach non-clonable shadow");
    wrap(root, &mut session);

    let r = CloneNode
        .invoke(root, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let cloned_root = cloned_entity(&r, &session);
    let cloned_host = dom.children(cloned_root)[0];
    assert!(
        dom.get_shadow_root(cloned_host).is_none(),
        "non-clonable shadow must not replicate"
    );
}

#[test]
fn clone_node_shallow_replicates_own_clonable_shadow_shallowly() {
    // Step 6 is not gated on the subtree flag: cloneNode(false) of a
    // clonable shadow host still attaches a replicated shadow root;
    // step 6.7 clones each shadow child with subtree=false, so the
    // child itself appears but its descendants do not.
    let (mut dom, mut session) = setup();
    let host = dom.create_element("section", Attributes::default());
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let span = dom.create_element("span", Attributes::default());
    let text = dom.create_text("deep");
    dom.append_child(span, text);
    dom.append_child(sr, span);
    let light = dom.create_element("p", Attributes::default());
    dom.append_child(host, light);
    wrap(host, &mut session);

    let r = CloneNode
        .invoke(host, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned_host = cloned_entity(&r, &session);
    assert!(
        dom.children(cloned_host).is_empty(),
        "light children excluded from shallow clone"
    );
    let cloned_sr = dom
        .get_shadow_root(cloned_host)
        .expect("clonable shadow root replicated even on shallow clone");
    let shadow_kids = dom.children(cloned_sr);
    assert_eq!(shadow_kids.len(), 1, "shadow child cloned");
    assert!(
        dom.children(shadow_kids[0]).is_empty(),
        "subtree=false: shadow child's own descendants not cloned (step 6.7)"
    );
}

#[test]
fn clone_node_propagates_ce_identity_inside_cloned_shadow_tree() {
    // CE elements nested inside a replicated shadow tree get identity
    // propagation through the same pair worklist.
    let (mut dom, mut session) = setup();
    let host = dom.create_element("section", Attributes::default());
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let widget = dom.create_element("my-widget", Attributes::default());
    dom.world_mut()
        .insert_one(widget, CustomElementState::custom("my-widget"))
        .unwrap();
    dom.append_child(sr, widget);
    wrap(host, &mut session);

    let r = CloneNode
        .invoke(host, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let cloned_host = cloned_entity(&r, &session);
    let cloned_sr = dom.get_shadow_root(cloned_host).expect("shadow replicated");
    let shadow_kids = dom.children(cloned_sr);
    let ce = dom
        .world()
        .get::<&CustomElementState>(shadow_kids[0])
        .expect("CE identity propagated inside cloned shadow tree");
    assert_eq!(ce.state, CEState::Undefined, "Custom resets to Undefined");
    assert_eq!(ce.definition_name, "my-widget");
}

#[test]
fn clone_node_document_deep_threads_clone_doc_into_replicated_shadow() {
    // The shadow-replication pass issues its own clone calls; without
    // explicit document threading those re-derive the owner document
    // from the SOURCE child — stamping the original document onto a
    // cloned Document's shadow contents while the light tree carries
    // the clone (the spec threads one `document` through the whole
    // recursion, shadow children included, step 6.7).
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(doc, html);
    let host = dom.create_element("section", Attributes::default());
    dom.append_child(html, host);
    dom.set_associated_document(html, doc);
    dom.set_associated_document(host, doc);
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let span = dom.create_element("span", Attributes::default());
    dom.set_associated_document(span, doc);
    dom.append_child(sr, span);
    wrap(doc, &mut session);

    let r = CloneNode
        .invoke(doc, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let cloned_doc = cloned_entity(&r, &session);
    let cloned_html = dom.children(cloned_doc)[0];
    let cloned_host = dom.children(cloned_html)[0];
    // Light-tree invariant (pre-existing): descendants adopt the clone.
    assert_eq!(dom.owner_document(cloned_host), Some(cloned_doc));
    let cloned_sr = dom.get_shadow_root(cloned_host).expect("shadow replicated");
    let shadow_kids = dom.children(cloned_sr);
    assert_eq!(shadow_kids.len(), 1);
    assert_eq!(
        dom.owner_document(shadow_kids[0]),
        Some(cloned_doc),
        "replicated shadow contents must adopt the CLONED document, not the source"
    );
}

#[test]
fn clone_node_shallow_shadow_children_carry_owner_document() {
    // The shallow cloner stamps no AssociatedDocument; the shadow pass
    // must thread the host's owner document onto replicated shadow
    // children explicitly.
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let host = dom.create_element("section", Attributes::default());
    dom.set_associated_document(host, doc);
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let span = dom.create_element("span", Attributes::default());
    dom.set_associated_document(span, doc);
    dom.append_child(sr, span);
    wrap(host, &mut session);

    let r = CloneNode
        .invoke(host, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned_host = cloned_entity(&r, &session);
    let cloned_sr = dom.get_shadow_root(cloned_host).expect("shadow replicated");
    let shadow_kids = dom.children(cloned_sr);
    assert_eq!(
        dom.owner_document(shadow_kids[0]),
        Some(doc),
        "shallow-replicated shadow children keep the source's node document"
    );
}

#[test]
fn clone_node_propagates_is_value_slot() {
    // Codex PR331: the is-value slot (now separate from
    // definition_name) must survive cloning alongside the identity.
    let (mut dom, mut session) = setup();
    let el = dom.create_element("my-el", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            CustomElementState::for_created_element(
                "my-el",
                Some("my-other"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    wrap(el, &mut session);

    let r = CloneNode
        .invoke(el, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let ce = dom.world().get::<&CustomElementState>(cloned).unwrap();
    assert_eq!(ce.definition_name, "my-el");
    assert_eq!(ce.is_value(), Some("my-other"));
}

#[test]
fn clone_node_shallow_shadow_root_carries_owner_document() {
    // Codex PR331 R3: the replicated shadow root entity (and the
    // shallow-cloned host) must carry the operation's document — the
    // children were stamped but the root/host were not, leaving
    // `clonedHost.shadowRoot.ownerDocument` null.
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let host = dom.create_element("section", Attributes::default());
    dom.set_associated_document(host, doc);
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let span = dom.create_element("span", Attributes::default());
    dom.set_associated_document(span, doc);
    dom.append_child(sr, span);
    wrap(host, &mut session);

    let r = CloneNode
        .invoke(host, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned_host = cloned_entity(&r, &session);
    assert_eq!(
        dom.owner_document(cloned_host),
        Some(doc),
        "shallow-cloned shadow host stamped with the operation's document"
    );
    let cloned_sr = dom.get_shadow_root(cloned_host).expect("shadow replicated");
    assert_eq!(
        dom.owner_document(cloned_sr),
        Some(doc),
        "replicated shadow root stamped with the operation's document"
    );
}
