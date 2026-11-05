//! Shadow-honor clone tests (DOM clone-a-node step 6) — split out from
//! `clone.rs` (~1000-line file convention). Also the `CloneCreated` emitter
//! tests (one event per entry call) and the D14 pins (step 6.7 clones shadow
//! children with subtree true, at every pair).

use super::*;
use crate::test_util::entity_of as cloned_entity;
use crate::test_util::CloneCreatedProbe;
use elidex_custom_elements::{CEState, CustomElementState};
use elidex_ecs::{CloneScope, ShadowInit, ShadowRootMode};

// -----------------------------------------------------------------------
// Shadow honor — DOM §4.4 clone-a-node step 6, applied per node via
// step 5's re-entry (descendant hosts) and regardless of the subtree
// flag (shallow clones replicate clonable shadow trees too; step 6.7
// clones every shadow child with subtree TRUE, so their descendants are
// cloned whatever the operation's flag).
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
fn clone_node_shallow_replicates_own_clonable_shadow_deeply() {
    // Step 6 is not gated on the subtree flag: cloneNode(false) of a
    // clonable shadow host still attaches a replicated shadow root; step 6.7
    // clones each shadow child with subtree TRUE (the spec's literal "true"),
    // so the shadow child's own descendants are cloned too.  The light
    // children (step 5) stay excluded.  (Codex #589 shape.)
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
    assert_eq!(
        dom.children(shadow_kids[0]).len(),
        1,
        "step 6.7 clones the shadow child with subtree=true: its descendants are cloned"
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
    // The shadow pass (pass 4) threads the host's owner document onto the
    // replicated shadow children as the cloner's `doc_override`. The shadow
    // sources belong to a DIFFERENT document (`other`) than the host (`doc`),
    // so a pass that dropped the override would fall back to
    // `owner_document(src)` (tree_clone.rs) and stamp `other` — on the shadow
    // child and, through the recursion, on its descendants.
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let other = dom.create_document_root();
    assert_ne!(doc, other);
    let host = dom.create_element("section", Attributes::default());
    dom.set_associated_document(host, doc);
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    let span = dom.create_element("span", Attributes::default());
    dom.set_associated_document(span, other);
    let leaf = dom.create_element("em", Attributes::default());
    dom.set_associated_document(leaf, other);
    dom.append_child(span, leaf);
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
        "shallow-replicated shadow children take the host's node document, not the source's"
    );
    let grandchildren = dom.children(shadow_kids[0]);
    assert_eq!(grandchildren.len(), 1, "step 6.7: descendants cloned");
    assert_eq!(
        dom.owner_document(grandchildren[0]),
        Some(doc),
        "the shadow child's own descendants carry the host's node document too"
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

#[test]
fn clone_created_fires_once_per_entry_call_with_scope() {
    // (a) One event per entry call, `root == cloned`: `Subtree` for deep,
    // `SelfOnly` for shallow — however many hosts / templates the copy holds.
    for (deep, want) in [(true, CloneScope::Subtree), (false, CloneScope::SelfOnly)] {
        let (mut dom, mut session) = setup();
        let host = dom.create_element("section", Attributes::default());
        let sr = dom
            .attach_shadow_with_init(host, clonable_init())
            .expect("attach clonable shadow");
        let nested = dom.create_element("article", Attributes::default());
        dom.append_child(sr, nested);
        dom.attach_shadow_with_init(nested, clonable_init())
            .expect("nested clonable shadow");
        let tpl = dom.create_element("template", Attributes::default());
        dom.attach_template_contents(tpl, elidex_ecs::NewNodeOwner::ownerless_fixture());
        dom.append_child(sr, tpl);
        wrap(host, &mut session);

        let probe = CloneCreatedProbe::default();
        dom.set_mutation_dispatcher(Box::new(probe.clone()));
        let r = CloneNode
            .invoke(host, &[JsValue::Bool(deep)], &mut session, &mut dom)
            .unwrap();
        let cloned_host = cloned_entity(&r, &session);
        let events = probe.events.lock().unwrap().clone();
        assert_eq!(events, vec![(cloned_host, want)], "deep={deep}");
    }
}

#[test]
fn clone_shallow_deep_clones_template_contents_and_nested_host_inside_shadow() {
    // (b) D14, the non-root pairs: inside the shadow tree of a
    // `cloneNode(false)` root, a `<template>` with contents has its contents
    // cloned (pass 3 with the pair's own flag, true), and a nested clonable
    // host has its own shadow children deep-cloned (pass 4 at a non-root pair).
    let (mut dom, mut session) = setup();
    let host = dom.create_element("section", Attributes::default());
    let sr = dom
        .attach_shadow_with_init(host, clonable_init())
        .expect("attach clonable shadow");
    // <template> with a child in its contents.
    let tpl = dom.create_element("template", Attributes::default());
    let frag = dom.attach_template_contents(tpl, elidex_ecs::NewNodeOwner::ownerless_fixture());
    let content_child = dom.create_element("b", Attributes::default());
    dom.append_child(frag, content_child);
    dom.append_child(sr, tpl);
    // Nested clonable host whose shadow child has a descendant.
    let inner_host = dom.create_element("article", Attributes::default());
    dom.append_child(sr, inner_host);
    let inner_sr = dom
        .attach_shadow_with_init(inner_host, clonable_init())
        .expect("nested clonable shadow");
    let inner_child = dom.create_element("i", Attributes::default());
    let inner_text = dom.create_text("t");
    dom.append_child(inner_child, inner_text);
    dom.append_child(inner_sr, inner_child);
    wrap(host, &mut session);

    let r = CloneNode
        .invoke(host, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned_host = cloned_entity(&r, &session);
    let cloned_sr = dom.get_shadow_root(cloned_host).expect("shadow replicated");
    let kids = dom.children(cloned_sr);
    assert_eq!(kids.len(), 2);
    let cloned_frag = dom
        .template_contents_fragment(kids[0])
        .expect("cloned template has its own fragment");
    assert_ne!(cloned_frag, frag);
    assert_eq!(
        dom.children(cloned_frag).len(),
        1,
        "pass 3 at a non-root pair: contents cloned (subtree true)"
    );
    let cloned_inner_sr = dom
        .get_shadow_root(kids[1])
        .expect("nested host's shadow replicated");
    let inner_kids = dom.children(cloned_inner_sr);
    assert_eq!(inner_kids.len(), 1);
    assert_eq!(
        dom.children(inner_kids[0]).len(),
        1,
        "pass 4 at a non-root pair: shadow child deep-cloned"
    );
}

#[test]
fn clone_created_fires_after_the_whole_copy_exists() {
    // (d) At dispatch the recorded root already has its shadow root, and for
    // `Subtree` its light children; for `SelfOnly` no light children.
    for (deep, want_light) in [(true, true), (false, false)] {
        let (mut dom, mut session) = setup();
        let host = dom.create_element("section", Attributes::default());
        dom.attach_shadow_with_init(host, clonable_init())
            .expect("attach clonable shadow");
        let light = dom.create_element("p", Attributes::default());
        dom.append_child(host, light);
        let sr = dom.get_shadow_root(host).expect("shadow root");
        let shadow_child = dom.create_element("b", Attributes::default());
        dom.append_child(sr, shadow_child);
        wrap(host, &mut session);

        let probe = CloneCreatedProbe::default();
        dom.set_mutation_dispatcher(Box::new(probe.clone()));
        let _ = CloneNode
            .invoke(host, &[JsValue::Bool(deep)], &mut session, &mut dom)
            .unwrap();
        let shapes = probe.shape_at_dispatch.lock().unwrap().clone();
        assert_eq!(shapes, vec![(true, want_light)], "deep={deep}");
        // Step 6.7: the shadow root's children already exist at dispatch,
        // under both scopes.
        let shadow_kids = probe.shadow_children_at_dispatch.lock().unwrap().clone();
        assert_eq!(shadow_kids, vec![1], "deep={deep}");
    }
}
