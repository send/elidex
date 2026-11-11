use super::*;

// -----------------------------------------------------------------------
// CloneNode
// -----------------------------------------------------------------------

#[test]
fn clone_node_shallow() {
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("class", "test");
    let div = dom.create_element_ownerless_fixture("div", attrs);
    let child = dom.create_text_ownerless_fixture("hello");
    dom.append_child(div, child);
    wrap(div, &mut session);

    let r = CloneNode
        .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        // Tag preserved.
        assert_eq!(dom.world().get::<&TagType>(cloned).unwrap().0, "div");
        // Attributes preserved.
        assert_eq!(
            dom.world().get::<&Attributes>(cloned).unwrap().get("class"),
            Some("test")
        );
        // No children (shallow).
        assert!(dom.children(cloned).is_empty());
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_deep() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    let text = dom.create_text_ownerless_fixture("hello");
    dom.append_child(div, text);
    wrap(div, &mut session);

    let r = CloneNode
        .invoke(div, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        let children = dom.children(cloned);
        assert_eq!(children.len(), 1);
        let child_text = dom
            .world()
            .get::<&TextContent>(children[0])
            .unwrap()
            .0
            .clone();
        assert_eq!(child_text, "hello");
        // Cloned child is a different entity.
        assert_ne!(children[0], text);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_input_attaches_cloned_from_marker() {
    // The form-control cloning-steps link: a cloned `<input>` carries a
    // `ClonedFrom` pointing at its source, for the `elidex-form` consumer.
    let (mut dom, mut session) = setup();
    let input = dom.create_element_ownerless_fixture("input", Attributes::default());
    wrap(input, &mut session);
    let r = CloneNode
        .invoke(input, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let marker = dom.world().get::<&ClonedFrom>(cloned).expect("ClonedFrom");
    assert_eq!(marker.source, input);
}

#[test]
fn clone_textarea_attaches_cloned_from_marker() {
    let (mut dom, mut session) = setup();
    let ta = dom.create_element_ownerless_fixture("textarea", Attributes::default());
    wrap(ta, &mut session);
    let r = CloneNode
        .invoke(ta, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    assert_eq!(dom.world().get::<&ClonedFrom>(cloned).unwrap().source, ta);
}

#[test]
fn clone_non_form_control_has_no_cloned_from_marker() {
    // The attach is gated on `<input>` / `<textarea>` — a `<div>` clone is
    // never marked (so a large subtree clone does not churn markers).
    let (mut dom, mut session) = setup();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    wrap(div, &mut session);
    let r = CloneNode
        .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    assert!(dom.world().get::<&ClonedFrom>(cloned).is_err());
}

#[test]
fn clone_deep_marks_nested_form_control_with_its_own_source() {
    // Deep clone of `<div><input></div>`: the cloned `<input>` descendant is
    // marked, with `source` pointing at the ORIGINAL nested input (not the root).
    let (mut dom, mut session) = setup();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    let input = dom.create_element_ownerless_fixture("input", Attributes::default());
    dom.append_child(div, input);
    wrap(div, &mut session);
    let r = CloneNode
        .invoke(div, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let cloned_div = cloned_entity(&r, &session);
    let cloned_input = dom.children(cloned_div)[0];
    // The root `<div>` clone carries no marker; only the nested `<input>` does.
    assert!(dom.world().get::<&ClonedFrom>(cloned_div).is_err());
    assert_eq!(
        dom.world().get::<&ClonedFrom>(cloned_input).unwrap().source,
        input
    );
}

#[test]
fn clone_node_template_deep_clones_content_into_fresh_fragment() {
    // HTML §4.12.3 cloning steps: a deep clone of a `<template>` gets its own
    // fresh content fragment populated with deep-cloned children — never the
    // source fragment (no aliasing).
    let (mut dom, mut session) = setup();
    let template = dom.create_element_ownerless_fixture("template", Attributes::default());
    let src_fragment =
        dom.attach_template_contents(template, elidex_ecs::NewNodeOwner::ownerless_fixture());
    let span = dom.create_element_ownerless_fixture("span", Attributes::default());
    let txt = dom.create_text_ownerless_fixture("hi");
    dom.append_child(span, txt);
    dom.append_child(src_fragment, span);
    wrap(template, &mut session);

    let r = CloneNode
        .invoke(template, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let JsValue::ObjectRef(ref_id) = r else {
        panic!("expected ObjectRef");
    };
    let (cloned, _) = session
        .identity_map()
        .get(JsObjectRef::from_raw(ref_id))
        .unwrap();

    let cloned_fragment = dom
        .template_contents_fragment(cloned)
        .expect("clone has its own content fragment");
    assert_ne!(
        cloned_fragment, src_fragment,
        "clone must not alias the source content fragment"
    );
    let kids = dom.children(cloned_fragment);
    assert_eq!(kids.len(), 1);
    assert!(dom.has_tag(kids[0], "span"));
    assert_ne!(kids[0], span, "content deep-cloned to fresh entities");
    let cloned_text = dom.children(kids[0]);
    assert_eq!(cloned_text.len(), 1);
    assert_eq!(
        dom.world().get::<&TextContent>(cloned_text[0]).unwrap().0,
        "hi"
    );
    // Source fragment is unchanged.
    assert_eq!(dom.children(src_fragment).len(), 1);
}

#[test]
fn clone_node_template_shallow_gets_fresh_empty_fragment() {
    // Per §4.12.3 cloning steps, the contents are cloned only when the
    // clone-children flag is set; a shallow clone still gets its own *empty*
    // content fragment (every template has one).
    let (mut dom, mut session) = setup();
    let template = dom.create_element_ownerless_fixture("template", Attributes::default());
    let src_fragment =
        dom.attach_template_contents(template, elidex_ecs::NewNodeOwner::ownerless_fixture());
    let span = dom.create_element_ownerless_fixture("span", Attributes::default());
    dom.append_child(src_fragment, span);
    wrap(template, &mut session);

    let r = CloneNode
        .invoke(template, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let JsValue::ObjectRef(ref_id) = r else {
        panic!("expected ObjectRef");
    };
    let (cloned, _) = session
        .identity_map()
        .get(JsObjectRef::from_raw(ref_id))
        .unwrap();

    let cloned_fragment = dom
        .template_contents_fragment(cloned)
        .expect("shallow clone still has a content fragment");
    assert_ne!(cloned_fragment, src_fragment);
    assert!(
        dom.children(cloned_fragment).is_empty(),
        "shallow clone's content fragment is empty"
    );
}

#[test]
fn clone_node_template_shallow_owner_document_is_inert_template_document() {
    // Codex PR380 R2 (P2): `cloneNode(false)` on a document-owned `<template>`
    // must give the (empty) clone fragment the operation's node document: the
    // contents are created with the operation's `NewNodeOwner`, so
    // `content.ownerDocument` is not null. Since C1 it is the document's inert
    // template document (HTML §4.12.3 *appropriate template contents owner
    // document*; the clone's contents take `f(document)`, D-C6).
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let template = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc),
    );
    let _ = dom.attach_template_contents(template, dom.node_document_for_new_nodes(doc));
    wrap(template, &mut session);

    let r = CloneNode
        .invoke(template, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let JsValue::ObjectRef(ref_id) = r else {
        panic!("expected ObjectRef");
    };
    let (cloned, _) = session
        .identity_map()
        .get(JsObjectRef::from_raw(ref_id))
        .unwrap();
    let cloned_fragment = dom
        .template_contents_fragment(cloned)
        .expect("shallow clone has a content fragment");
    let inert = dom.appropriate_template_contents_owner_document(doc);
    assert_ne!(inert, doc);
    assert_eq!(
        dom.owner_document(cloned_fragment),
        Some(inert),
        "shallow template clone's content fragment ownerDocument is the document's inert template document, not null"
    );
}

#[test]
fn clone_node_no_identity() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    wrap(div, &mut session);

    let r = CloneNode.invoke(div, &[], &mut session, &mut dom).unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        // Cloned entity is different from original.
        assert_ne!(cloned, div);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_copies_inline_style() {
    // Inverted from the pre-copy-set behaviour this test used to pin:
    // InlineStyle is a derived-copy component (clone-policy table in
    // elidex-ecs tree_clone) — without the copy, `.style.length` /
    // `cssText` reads on the clone see an empty declaration block
    // because the CSSOM read handlers treat component absence as
    // empty and only mutation paths re-hydrate from the attribute.
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("style", "color: red");
    let div = dom.create_element_ownerless_fixture("div", attrs);
    let mut style = InlineStyle::default();
    style.set("color", "red");
    dom.world_mut().insert_one(div, style).unwrap();
    wrap(div, &mut session);

    let r = CloneNode.invoke(div, &[], &mut session, &mut dom).unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_eq!(
            dom.world()
                .get::<&InlineStyle>(cloned)
                .expect("InlineStyle copied to clone")
                .get("color"),
            Some("red")
        );
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_shadow_root_error() {
    let (mut dom, mut session) = setup();
    let host = dom.create_element_ownerless_fixture("div", Attributes::default());
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    wrap(sr, &mut session);

    let r = CloneNode.invoke(sr, &[], &mut session, &mut dom);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().kind, DomApiErrorKind::NotSupportedError);
}

#[test]
fn clone_node_document_type() {
    let (mut dom, mut session) = setup();
    let dt = dom.create_document_type(
        "html",
        "-//W3C",
        "http://example.com",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    wrap(dt, &mut session);

    let r = CloneNode.invoke(dt, &[], &mut session, &mut dom).unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        let data = dom.world().get::<&DocTypeData>(cloned).unwrap();
        assert_eq!(data.name, "html");
        assert_eq!(data.public_id, "-//W3C");
        assert_eq!(data.system_id, "http://example.com");
        assert_ne!(cloned, dt);
    } else {
        panic!("expected ObjectRef");
    }
}

// -----------------------------------------------------------------------
// CloneNode — ComponentKind (M6)
// -----------------------------------------------------------------------

#[test]
fn clone_node_component_kind_element() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    wrap(div, &mut session);

    let result = CloneNode
        .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn clone_node_component_kind_document() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    wrap(doc, &mut session);

    let result = CloneNode
        .invoke(doc, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = result {
        let (cloned, kind) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_ne!(cloned, doc);
        assert_eq!(kind, ComponentKind::Document);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_destroyed_source_yields_not_found_error() {
    // Pin Copilot R1 vLGj: when EcsDom::clone_subtree returns None
    // it means the source entity no longer exists, which maps to
    // DOMException("NotFoundError"), not NotSupportedError.
    let (mut dom, mut session) = setup();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    wrap(div, &mut session);
    // Despawn the source so the cloner returns None.
    let _ = dom.world_mut().despawn(div);
    let err = CloneNode
        .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
        .expect_err("destroyed source must surface as DomApiError");
    assert_eq!(err.kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn clone_node_document_fragment_deep_preserves_children() {
    // Pin DocumentFragment.cloneNode(true) — the handler test
    // surface previously covered Element / Text / DocumentType but
    // not DocumentFragment, leaving the kind branch in
    // EcsDom::clone_subtree unpinned at this layer.
    let (mut dom, mut session) = setup();
    let frag = dom.create_document_fragment_ownerless_fixture();
    let child = dom.create_element_ownerless_fixture("p", Attributes::default());
    let grandchild = dom.create_text_ownerless_fixture("hello");
    dom.append_child(child, grandchild);
    dom.append_child(frag, child);
    wrap(frag, &mut session);

    let r = CloneNode
        .invoke(frag, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let JsValue::ObjectRef(ref_id) = r else {
        panic!("expected ObjectRef");
    };
    let (cloned_frag, kind) = session
        .identity_map()
        .get(JsObjectRef::from_raw(ref_id))
        .unwrap();
    assert_ne!(cloned_frag, frag);
    assert_eq!(kind, ComponentKind::DocumentFragment);
    let kids = dom.children(cloned_frag);
    assert_eq!(kids.len(), 1, "deep clone preserves direct children");
    let cloned_text = dom.children(kids[0]);
    assert_eq!(
        cloned_text.len(),
        1,
        "deep clone recurses into grandchildren"
    );
    assert_eq!(
        dom.world().get::<&TextContent>(cloned_text[0]).unwrap().0,
        "hello"
    );
}

#[test]
fn clone_node_attribute_kind_yields_not_supported_error() {
    // Pin Copilot R2 vLY2J: ECS cloners snapshot only TagType /
    // TextContent / CommentData / DocTypeData / Attributes — they
    // don't carry AttrData, so dispatching an Attribute entity
    // through them would produce a structurally invalid clone
    // (NodeKind=Attribute, AttrData missing).  Refuse early.
    let (mut dom, mut session) = setup();
    let attr = dom.create_attribute("id", elidex_ecs::NewNodeOwner::ownerless_fixture());
    wrap(attr, &mut session);
    let err = CloneNode
        .invoke(attr, &[JsValue::Bool(false)], &mut session, &mut dom)
        .expect_err("Attribute kind must surface as DomApiError");
    assert_eq!(err.kind, DomApiErrorKind::NotSupportedError);
}

#[test]
fn clone_node_window_kind_yields_type_error() {
    // Pin Copilot R3 vLknj: Window is not a Node per WHATWG DOM
    // (EventTarget mixin only, no nodeType).  Calling cloneNode on
    // a Window receiver is a WebIDL §3.6.5 "illegal invocation",
    // which must surface as a plain TypeError, NOT a DOMException
    // (the latter is reserved for Node receivers whose operation
    // can't be performed — Attribute / ProcessingInstruction above).
    let (mut dom, mut session) = setup();
    let window = dom.create_window_root();
    wrap(window, &mut session);
    let err = CloneNode
        .invoke(window, &[JsValue::Bool(false)], &mut session, &mut dom)
        .expect_err("Window receiver must surface as DomApiError");
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

// -----------------------------------------------------------------------
// CustomElementState identity propagation (DOM §4.4 "clone a single
// node" step 2.4: the source's *is value* — materialized as the
// component's `definition_name` — propagates; lifecycle state resets).
// -----------------------------------------------------------------------

use crate::test_util::entity_of as cloned_entity;
use elidex_custom_elements::{CEState, CustomElementState};

#[test]
fn clone_node_propagates_customized_builtin_identity() {
    // `<button is="my-btn">` equivalent: a creation path attached
    // Undefined("my-btn"). The clone must carry the same identity —
    // shallow and on deep descendants.
    let (mut dom, mut session) = setup();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("is", "my-btn");
    let button = dom.create_element_ownerless_fixture("button", attrs);
    dom.world_mut()
        .insert_one(button, CustomElementState::undefined("my-btn"))
        .unwrap();
    dom.append_child(parent, button);
    wrap(parent, &mut session);

    // Shallow clone of the button itself.
    wrap(button, &mut session);
    let r = CloneNode
        .invoke(button, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let shallow = cloned_entity(&r, &session);
    {
        let ce = dom
            .world()
            .get::<&CustomElementState>(shallow)
            .expect("CE identity propagated on shallow clone");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "my-btn");
    }

    // Deep clone of the parent — descendant pair propagation.
    let r = CloneNode
        .invoke(parent, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    let deep = cloned_entity(&r, &session);
    let kids = dom.children(deep);
    assert_eq!(kids.len(), 1);
    let ce = dom
        .world()
        .get::<&CustomElementState>(kids[0])
        .expect("CE identity propagated to deep descendant");
    assert_eq!(ce.state, CEState::Undefined);
    assert_eq!(ce.definition_name, "my-btn");
}

#[test]
fn clone_node_resets_failed_state_to_undefined() {
    // DOM §4.4 "clone a node": the clone re-enters the upgrade
    // pipeline fresh — a Failed source still yields an Undefined
    // clone with the same definition name.
    let (mut dom, mut session) = setup();
    let el = dom.create_element_ownerless_fixture("my-x", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            CustomElementState {
                state: CEState::Failed,
                definition_name: "my-x".to_string(),
                is_value: None,
                registry: elidex_custom_elements::RegistryAssociation::Document,
            },
        )
        .unwrap();
    wrap(el, &mut session);

    let r = CloneNode
        .invoke(el, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let ce = dom.world().get::<&CustomElementState>(cloned).unwrap();
    assert_eq!(ce.state, CEState::Undefined);
    assert_eq!(ce.definition_name, "my-x");
}

#[test]
fn clone_node_is_value_slot_not_rederived_from_attr() {
    // The is value is a creation-time slot, not the live attribute
    // (DOM §4.9 / §4.4). A source whose `is` attribute appeared only
    // after creation (no CustomElementState component) clones to an
    // unmarked element — re-deriving from the attribute here would be
    // a spec violation.
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("is", "my-btn");
    let button = dom.create_element_ownerless_fixture("button", attrs);
    wrap(button, &mut session);

    let r = CloneNode
        .invoke(button, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    assert!(
        dom.world().get::<&CustomElementState>(cloned).is_err(),
        "clone must not re-derive CE state from the is attribute"
    );
}

#[test]
fn clone_node_foreign_hyphenated_keeps_namespace_no_ce_state() {
    // `<svg><my-foo>`: namespace must copy (intrinsic), and no CE
    // state appears (the creation path never marked the foreign
    // element; propagation has nothing to propagate). Guards the
    // pre-fix regression where the clone path tag-derived CE state
    // without a namespace guard.
    let (mut dom, mut session) = setup();
    let el = dom.create_element_ns(
        "my-foo",
        elidex_ecs::Namespace::Svg,
        Attributes::default(),
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    wrap(el, &mut session);

    let r = CloneNode
        .invoke(el, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    assert_eq!(dom.namespace_of(cloned), elidex_ecs::Namespace::Svg);
    assert!(
        dom.world().get::<&CustomElementState>(cloned).is_err(),
        "foreign-namespace hyphenated clone must not be marked custom"
    );
}

#[test]
fn clone_node_copies_iframe_data() {
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("src", "x");
    let iframe = dom.create_element_ownerless_fixture("iframe", attrs.clone());
    dom.world_mut()
        .insert_one(iframe, elidex_ecs::IframeData::from_attributes(&attrs))
        .unwrap();
    wrap(iframe, &mut session);

    let r = CloneNode
        .invoke(iframe, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let data = dom
        .world()
        .get::<&elidex_ecs::IframeData>(cloned)
        .expect("IframeData copied to clone");
    assert_eq!(data.src.as_deref(), Some("x"));
}

#[test]
fn clone_propagates_null_registry_association() {
    // Codex PR331 R12: DOM 4.4 "clone a single node" passes the
    // source's custom element registry through *create an element* --
    // a null-registry custom element clones to a null-registry clone
    // (still excluded from every upgrade path).
    let (mut dom, mut session) = setup();
    let el = dom.create_element_ownerless_fixture("x-nullclone", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            CustomElementState {
                state: CEState::Undefined,
                definition_name: "x-nullclone".to_string(),
                is_value: None,
                registry: elidex_custom_elements::RegistryAssociation::Null,
            },
        )
        .unwrap();
    wrap(el, &mut session);

    let r = CloneNode
        .invoke(el, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let state = dom
        .world()
        .get::<&CustomElementState>(cloned)
        .expect("CE identity propagated");
    assert_eq!(
        state.registry,
        elidex_custom_elements::RegistryAssociation::Null,
        "null-registry association must propagate to the clone"
    );
}

#[test]
fn clonable_shadow_clone_preserves_null_registry() {
    // DOM 4.4 clone-a-node step 6.5: the clone's shadow is attached
    // with the source root's registry -- a null-registry clonable
    // shadow tree stays null-registry on the clone.
    let (mut dom, mut session) = setup();
    let host = dom.create_element_ownerless_fixture("div", Attributes::default());
    let init = elidex_ecs::ShadowInit {
        clonable: true,
        null_registry: true,
        ..elidex_ecs::ShadowInit::default()
    };
    let _sr = dom.attach_shadow_with_init(host, init).unwrap();
    wrap(host, &mut session);

    let r = CloneNode
        .invoke(host, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let cloned_sr = dom
        .world()
        .get::<&elidex_ecs::ShadowHost>(cloned)
        .expect("clonable shadow replicated")
        .shadow_root;
    let sr = dom
        .world()
        .get::<&elidex_ecs::ShadowRoot>(cloned_sr)
        .expect("ShadowRoot component");
    assert!(
        sr.null_registry,
        "null-registry shadow association must survive cloning"
    );
}

#[test]
fn clone_node_rederives_iframe_data_from_cloned_attributes() {
    // Codex PR331 R10: generic `setAttribute("src", ...)` has no
    // `IframeData` re-derivation pass (slot
    // `#11-derived-component-attr-maintenance`), so a clone taken
    // inside that stale window must NOT copy the stale component --
    // the cloner re-derives from the cloned attributes ("derived
    // re-derive" policy), keeping the clone's attrs<->component pair
    // consistent by construction.
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("src", "old.html");
    let iframe = dom.create_element_ownerless_fixture("iframe", attrs.clone());
    dom.world_mut()
        .insert_one(iframe, elidex_ecs::IframeData::from_attributes(&attrs))
        .unwrap();
    // Stale window: the attribute moves, the derived component does not.
    dom.set_attribute(iframe, "src", "new.html");
    wrap(iframe, &mut session);

    let r = CloneNode
        .invoke(iframe, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    let cloned = cloned_entity(&r, &session);
    let data = dom
        .world()
        .get::<&elidex_ecs::IframeData>(cloned)
        .expect("IframeData present on clone");
    assert_eq!(
        data.src.as_deref(),
        Some("new.html"),
        "clone must derive IframeData from its cloned attributes, not copy the stale component"
    );
}
