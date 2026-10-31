use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiErrorKind, DomApiHandler};

use crate::test_util::setup;

#[test]
fn append_child_success() {
    let (mut dom, parent, child, mut session) = setup();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let handler = AppendChild;
    let result = handler
        .invoke(
            parent,
            &[JsValue::ObjectRef(child_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(child_ref));
    assert_eq!(dom.children(parent), vec![child]);
}

#[test]
fn remove_child_success() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let handler = RemoveChild;
    let result = handler
        .invoke(
            parent,
            &[JsValue::ObjectRef(child_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(child_ref));
    assert!(dom.children(parent).is_empty());
}

#[test]
fn get_set_attribute() {
    let (mut dom, parent, _, mut session) = setup();

    let set_handler = SetAttribute;
    set_handler
        .invoke(
            parent,
            &[
                JsValue::String("data-x".into()),
                JsValue::String("42".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let get_handler = GetAttribute;
    let result = get_handler
        .invoke(
            parent,
            &[JsValue::String("data-x".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::String("42".into()));
}

#[test]
fn get_attribute_missing() {
    let (mut dom, parent, _, mut session) = setup();
    let handler = GetAttribute;
    let result = handler
        .invoke(
            parent,
            &[JsValue::String("nonexistent".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn remove_attribute() {
    let (mut dom, parent, _, mut session) = setup();
    // Set first.
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("class", "active");
    }
    let handler = RemoveAttribute;
    handler
        .invoke(
            parent,
            &[JsValue::String("class".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert!(!attrs.contains("class"));
}

#[test]
fn text_content_get_set() {
    let (mut dom, parent, _, mut session) = setup();
    let text_node = dom.create_text("original");
    dom.append_child(parent, text_node);

    // Get.
    let get = crate::node_methods::GetTextContentNodeKind;
    let result = get.invoke(parent, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("original".into()));

    // Set.
    let set = crate::node_methods::SetTextContentNodeKind;
    set.invoke(
        parent,
        &[JsValue::String("replaced".into())],
        &mut session,
        &mut dom,
    )
    .unwrap();

    let result = get.invoke(parent, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("replaced".into()));
}

// -----------------------------------------------------------------------
// replaceChild tests (WHATWG DOM §4.4)
// -----------------------------------------------------------------------

#[test]
fn replace_child_returns_old_node() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);
    let new_child = dom.create_element("h1", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_child, ComponentKind::Element)
        .to_raw();
    let old_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let result = ReplaceChild
        .invoke(
            parent,
            &[JsValue::ObjectRef(new_ref), JsValue::ObjectRef(old_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(old_ref));
    assert_eq!(dom.children(parent), vec![new_child]);
    // Old child is detached.
    assert_eq!(dom.get_parent(child), None);
}

#[test]
fn replace_child_old_not_a_child_is_not_found_error() {
    // §4.4 step 5: oldChild whose parent is not `parent` raises
    // NotFoundError, *not* HierarchyRequestError.
    let (mut dom, parent, _child, mut session) = setup();
    // `child` from setup is NOT appended to `parent`, so it counts
    // as a non-child for this test.
    let other = dom.create_element("h1", Attributes::default());
    let new_child = dom.create_element("section", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_child, ComponentKind::Element)
        .to_raw();
    let other_ref = session
        .get_or_create_wrapper(other, ComponentKind::Element)
        .to_raw();
    let err = ReplaceChild
        .invoke(
            parent,
            &[JsValue::ObjectRef(new_ref), JsValue::ObjectRef(other_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn replace_child_when_new_child_is_receiver_is_hierarchy_request_error() {
    // newChild == parent: WHATWG §4.4 step 2 (host-including
    // ancestor check). EcsDom::replace_child rejects when
    // newChild == parent. Distinct from the self-replace
    // (newChild == oldChild) no-op case covered separately below.
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);
    let parent_ref = session
        .get_or_create_wrapper(parent, ComponentKind::Element)
        .to_raw();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let err = ReplaceChild
        .invoke(
            parent,
            &[
                JsValue::ObjectRef(parent_ref),
                JsValue::ObjectRef(child_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn replace_child_ancestor_cycle_is_hierarchy_request_error() {
    // newChild is an ancestor of parent (host-including ancestor
    // check, §4.4 step 2). Layout: grand → parent → old_child.
    // Replace old_child with grand — must throw HierarchyRequestError.
    let (mut dom, parent, child, mut session) = setup();
    let grand = dom.create_element("body", Attributes::default());
    dom.append_child(grand, parent);
    dom.append_child(parent, child);
    let grand_ref = session
        .get_or_create_wrapper(grand, ComponentKind::Element)
        .to_raw();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let err = ReplaceChild
        .invoke(
            parent,
            &[JsValue::ObjectRef(grand_ref), JsValue::ObjectRef(child_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn replace_child_detaches_new_from_prior_parent() {
    // newChild that already lives elsewhere is detached and
    // re-parented (the §4.4 "adopt" + reparent semantics).
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);
    let donor = dom.create_element("section", Attributes::default());
    let new_child = dom.create_element("img", Attributes::default());
    dom.append_child(donor, new_child);
    let new_ref = session
        .get_or_create_wrapper(new_child, ComponentKind::Element)
        .to_raw();
    let old_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    ReplaceChild
        .invoke(
            parent,
            &[JsValue::ObjectRef(new_ref), JsValue::ObjectRef(old_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(parent), vec![new_child]);
    assert!(dom.children(donor).is_empty());
}

#[test]
fn replace_child_requires_object_ref_args() {
    // Missing args → TypeError (require_object_ref_arg).
    let (mut dom, parent, _, mut session) = setup();
    let err = ReplaceChild
        .invoke(parent, &[], &mut session, &mut dom)
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
}

#[test]
fn replace_child_self_replace_is_noop() {
    // Browser parity (Chrome / Firefox / WebKit):
    // `parent.replaceChild(x, x)` returns x without throwing —
    // the spec §4.4 step 8 reference-child adjustment makes the
    // insert+remove sequence collapse.  EcsDom::replace_child
    // rejects `new == old` early, so the handler must short-circuit
    // before dispatch or it would surface as HierarchyRequestError.
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let result = ReplaceChild
        .invoke(
            parent,
            &[JsValue::ObjectRef(child_ref), JsValue::ObjectRef(child_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(child_ref));
    assert_eq!(
        dom.children(parent),
        vec![child],
        "self-replace must leave the tree unchanged"
    );
    assert_eq!(dom.get_parent(child), Some(parent));
}

#[test]
fn replace_child_unknown_object_ref_is_not_found() {
    // ObjectRef that doesn't resolve via the session identity map
    // produces NotFoundError (matches AppendChild / RemoveChild).
    let (mut dom, parent, _, mut session) = setup();
    let err = ReplaceChild
        .invoke(
            parent,
            &[
                JsValue::ObjectRef(0xDEAD_BEEF),
                JsValue::ObjectRef(0xCAFE_BABE),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn insert_before_null_ref_appends() {
    let (mut dom, parent, child, mut session) = setup();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let handler = InsertBefore;
    let result = handler
        .invoke(
            parent,
            &[JsValue::ObjectRef(child_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(child_ref));
    assert_eq!(dom.children(parent), vec![child]);
}

#[test]
fn insert_before_undefined_ref_appends() {
    // WebIDL `Node?` treats both `null` and `undefined` as
    // "no reference child" — `parent.insertBefore(x, undefined)`
    // must append, not raise TypeError.
    let (mut dom, parent, child, mut session) = setup();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let result = InsertBefore
        .invoke(
            parent,
            &[JsValue::ObjectRef(child_ref), JsValue::Undefined],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(child_ref));
    assert_eq!(dom.children(parent), vec![child]);
}

#[test]
fn insert_before_ref_not_a_child_is_not_found_error() {
    // §4.2.3 ensure-pre-insertion-validity step 3: a non-null reference child that
    // is not a child of `this` → NotFoundError. The B1.2b-3 fold routes insertBefore
    // through the validator; before the fold the lax empty-`apply_*`-list path mapped
    // this to HierarchyRequestError (spec-wrong kind). Locks the spec-correct kind.
    let (mut dom, parent, child, mut session) = setup();
    let orphan = dom.create_element("aside", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let orphan_ref = session
        .get_or_create_wrapper(orphan, ComponentKind::Element)
        .to_raw();
    let err = InsertBefore
        .invoke(
            parent,
            &[JsValue::ObjectRef(new_ref), JsValue::ObjectRef(orphan_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn get_attribute_case_insensitive() {
    let (mut dom, parent, _, mut session) = setup();
    let set_handler = SetAttribute;
    set_handler
        .invoke(
            parent,
            &[
                JsValue::String("Data-X".into()),
                JsValue::String("42".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let get_handler = GetAttribute;
    // Both "data-x" and "Data-X" should find the attribute (stored as "data-x")
    let result = get_handler
        .invoke(
            parent,
            &[JsValue::String("Data-X".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::String("42".into()));
}

#[test]
fn remove_attribute_case_insensitive() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("class", "active");
    }
    let handler = RemoveAttribute;
    handler
        .invoke(
            parent,
            &[JsValue::String("CLASS".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert!(!attrs.contains("class"));
}

// -----------------------------------------------------------------------
// validate_attribute_name tests
// -----------------------------------------------------------------------

#[test]
fn validate_attr_name_rejects_empty() {
    let err = validate_attribute_name("").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn validate_attr_name_rejects_ascii_whitespace() {
    // DOM §1.4 forbids ASCII whitespace = {space, tab, LF, FF, CR}.
    for ws in ["a b", "a\tb", "a\nb", "a\u{000C}b", "a\rb"] {
        let err = validate_attribute_name(ws).unwrap_err();
        assert_eq!(
            err.kind,
            DomApiErrorKind::InvalidCharacterError,
            "expected reject for {ws:?}"
        );
    }
}

#[test]
fn validate_attr_name_accepts_non_ascii_whitespace() {
    // §1.4 forbids only *ASCII* whitespace; non-ASCII whitespace (NBSP,
    // U+2028, ideographic space) is a valid attribute-name character.
    // Regression: a Codex P2 on the dataset setter step-4 reuse — Rust
    // `char::is_whitespace()` had over-rejected these.
    for ok in ["a\u{00A0}b", "a\u{2028}b", "a\u{3000}b"] {
        assert!(
            validate_attribute_name(ok).is_ok(),
            "expected accept for {ok:?}"
        );
    }
}

#[test]
fn validate_attr_name_rejects_null() {
    let err = validate_attribute_name("a\0b").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn validate_attr_name_rejects_slash() {
    let err = validate_attribute_name("a/b").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn validate_attr_name_rejects_equals() {
    let err = validate_attribute_name("a=b").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn validate_attr_name_rejects_gt() {
    let err = validate_attribute_name("a>b").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn validate_attr_name_accepts_valid() {
    assert!(validate_attribute_name("data-foo").is_ok());
    assert!(validate_attribute_name("class").is_ok());
}

#[test]
fn set_attribute_rejects_invalid_name() {
    let (mut dom, parent, _, mut session) = setup();
    let err = SetAttribute
        .invoke(
            parent,
            &[JsValue::String(String::new()), JsValue::String("v".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

#[test]
fn remove_attribute_invalid_name_is_noop() {
    // WHATWG DOM §4.9 `removeAttribute(qualifiedName)` = "remove an attribute
    // given qualifiedName and this" — NO valid-name validation (unlike
    // setAttribute/toggleAttribute step 1). An invalid (or absent) name is a
    // silent no-op returning undefined, NOT an InvalidCharacterError.
    let (mut dom, parent, _, mut session) = setup();
    let out = RemoveAttribute
        .invoke(
            parent,
            &[JsValue::String("a b".into())],
            &mut session,
            &mut dom,
        )
        .expect("removeAttribute must not throw on an invalid name");
    assert!(matches!(out, JsValue::Undefined));
    // Nothing matched "a b" → no attribute removed → no MutationObserver record.
    assert!(session.take_notify_records().is_empty());
}

// ---------------------------------------------------------------------------
// B1.2b-3 — pre-insertion-validity fold. The core Node insert methods now run
// the canonical WHATWG DOM §4.2.3 ensure-pre-insertion-validity (steps 1/4/5/6)
// that the empty-`apply_*`-list signal did NOT encode (silently accepted before).
// ---------------------------------------------------------------------------

#[test]
fn append_child_into_text_parent_throws_hierarchy() {
    // §4.2.3 step 1: parent must be Document / DocumentFragment / Element.
    let (mut dom, _div, _span, mut session) = setup();
    let text_parent = dom.create_text("data");
    let el = dom.create_element("b", Attributes::default());
    let el_ref = session
        .get_or_create_wrapper(el, ComponentKind::Element)
        .to_raw();
    let err = AppendChild
        .invoke(
            text_parent,
            &[JsValue::ObjectRef(el_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn append_child_document_node_throws_hierarchy() {
    // §4.2.3 step 4: a Document node is not an allowed insertion node type.
    let (mut dom, parent, _span, mut session) = setup();
    let doc = dom.create_document_node();
    let doc_ref = session
        .get_or_create_wrapper(doc, ComponentKind::Document)
        .to_raw();
    let err = AppendChild
        .invoke(
            parent,
            &[JsValue::ObjectRef(doc_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn append_child_text_into_document_throws_hierarchy() {
    // §4.2.3 step 5: a Text node may not be inserted under a Document.
    let (mut dom, _div, _span, mut session) = setup();
    let doc = dom.create_document_node();
    let text = dom.create_text("x");
    let text_ref = session
        .get_or_create_wrapper(text, ComponentKind::TextNode)
        .to_raw();
    let err = AppendChild
        .invoke(doc, &[JsValue::ObjectRef(text_ref)], &mut session, &mut dom)
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn append_child_second_element_into_document_throws_hierarchy() {
    // §4.2.3 step 6 (Element branch): a Document may have at most one element child.
    let (mut dom, _div, _span, mut session) = setup();
    let doc = dom.create_document_node();
    let root = dom.create_element("html", Attributes::default());
    dom.append_child(doc, root); // the documentElement
    let second = dom.create_element("html", Attributes::default());
    let second_ref = session
        .get_or_create_wrapper(second, ComponentKind::Element)
        .to_raw();
    let err = AppendChild
        .invoke(
            doc,
            &[JsValue::ObjectRef(second_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn append_child_existing_document_element_throws_hierarchy() {
    // §4.2.3 pre-insert step 6 (Element branch) has NO self-exclusion: it tests
    // "parent has an element child" counting ALL element children INCLUDING the node
    // being inserted, and runs BEFORE the pre-insert self-reference adjustment
    // (pre-insert step 3). So `document.appendChild(document.documentElement)` throws
    // HierarchyRequestError — the existing documentElement is still an element child
    // of the document when validity runs. (Only the *replace* algorithm excludes
    // `child`: "an element child that is not child"; pre-insert does not — the spec
    // states the replace statements "differ from the pre-insert algorithm".) Codex
    // B1.2b-3 R1 caught the plan's wrong "step-6 self-exclusion" framing.
    let (mut dom, _div, _span, mut session) = setup();
    let doc = dom.create_document_node();
    let root = dom.create_element("html", Attributes::default());
    dom.append_child(doc, root);
    let root_ref = session
        .get_or_create_wrapper(root, ComponentKind::Element)
        .to_raw();
    let err = AppendChild
        .invoke(doc, &[JsValue::ObjectRef(root_ref)], &mut session, &mut dom)
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
    // The throw precedes any mutation — the document keeps its single element child.
    assert_eq!(dom.children(doc), vec![root]);
}

#[test]
fn replace_child_self_replace_orphan_throws_not_found() {
    // B1.2b-3 F1: the validator runs BEFORE the `new == old` self-replace
    // short-circuit, so replacing an orphan with itself throws §4.2.3 replace
    // step-3 NotFoundError (orphan is not a child of `this`), not a silent no-op.
    let (mut dom, parent, _span, mut session) = setup();
    let orphan = dom.create_element("i", Attributes::default());
    let orphan_ref = session
        .get_or_create_wrapper(orphan, ComponentKind::Element)
        .to_raw();
    let err = ReplaceChild
        .invoke(
            parent,
            &[
                JsValue::ObjectRef(orphan_ref),
                JsValue::ObjectRef(orphan_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::NotFoundError);
    // (the valid self-replace `replaceChild(child, child)` no-op is covered by
    // `replace_child_self_replace_is_noop` above — it still passes after the
    // validator is re-ordered before the short-circuit = the F1 regression proof.)
}
