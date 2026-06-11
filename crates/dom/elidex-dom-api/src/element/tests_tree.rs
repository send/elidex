use super::*;
use elidex_ecs::{Attributes, EcsDom, Entity, ShadowRootMode};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiErrorKind, DomApiHandler, SessionCore};

fn setup() -> (EcsDom, Entity, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    let mut session = SessionCore::new();
    // Pre-register entities so we can pass ObjectRef args.
    session.get_or_create_wrapper(parent, ComponentKind::Element);
    session.get_or_create_wrapper(child, ComponentKind::Element);
    (dom, parent, child, session)
}

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

#[test]
fn inner_html_serialization() {
    let (mut dom, parent, _, mut session) = setup();
    let text = dom.create_text("hello <world>");
    dom.append_child(parent, text);

    let handler = GetInnerHtml;
    let result = handler.invoke(parent, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("hello &lt;world&gt;".into()));
}

#[test]
fn inner_html_void_elements() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let br = dom.create_element("br", Attributes::default());
    let mut img_attrs = Attributes::default();
    img_attrs.set("src", "test.png");
    let img = dom.create_element("img", img_attrs);
    dom.append_child(div, br);
    dom.append_child(div, img);

    let mut session = SessionCore::new();
    let handler = GetInnerHtml;
    let result = handler.invoke(div, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("<br><img src=\"test.png\">".into()));
}

#[test]
fn inner_html_preserves_inter_element_whitespace() {
    // §11.3 whitespace unify: with the tolerant parser retaining inter-element
    // whitespace text nodes, serialization must round-trip them — a
    // whitespace-only text child is emitted verbatim, not dropped (outerHTML /
    // innerHTML fidelity for indented markup).
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let p1 = dom.create_element("p", Attributes::default());
    let a = dom.create_text("A");
    dom.append_child(p1, a);
    let ws = dom.create_text("\n  ");
    let p2 = dom.create_element("p", Attributes::default());
    let b = dom.create_text("B");
    dom.append_child(p2, b);
    dom.append_child(div, p1);
    dom.append_child(div, ws);
    dom.append_child(div, p2);

    let mut session = SessionCore::new();
    let result = GetInnerHtml
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(
        result,
        JsValue::String("<p>A</p>\n  <p>B</p>".into()),
        "inter-element whitespace must round-trip through innerHTML serialization"
    );
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

#[test]
fn inner_html_nested() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let mut p_attrs = Attributes::default();
    p_attrs.set("class", "intro");
    let p = dom.create_element("p", p_attrs);
    let text = dom.create_text("hi");
    dom.append_child(div, p);
    dom.append_child(p, text);

    let mut session = SessionCore::new();
    let handler = GetInnerHtml;
    let result = handler.invoke(div, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("<p class=\"intro\">hi</p>".into()));
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
fn validate_attr_name_rejects_whitespace() {
    let err = validate_attribute_name("a b").unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
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
fn remove_attribute_rejects_invalid_name() {
    let (mut dom, parent, _, mut session) = setup();
    let err = RemoveAttribute
        .invoke(
            parent,
            &[JsValue::String("a b".into())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

// -----------------------------------------------------------------------
// insertAdjacentElement / insertAdjacentText tests
// -----------------------------------------------------------------------

#[test]
fn insert_adjacent_element_beforebegin() {
    let (mut dom, parent, _child, mut session) = setup();
    let root = dom.create_element("body", Attributes::default());
    dom.append_child(root, parent);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    let result = InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("beforebegin".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(new_ref));
    let children = dom.children(root);
    assert_eq!(children, vec![new_elem, parent]);
}

#[test]
fn insert_adjacent_element_afterbegin() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("afterbegin".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(parent);
    assert_eq!(children[0], new_elem);
    assert_eq!(children[1], child);
}

#[test]
fn insert_adjacent_element_beforeend() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("beforeend".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(parent);
    assert_eq!(children[0], child);
    assert_eq!(children[1], new_elem);
}

#[test]
fn insert_adjacent_element_afterend() {
    let (mut dom, parent, _, mut session) = setup();
    let root = dom.create_element("body", Attributes::default());
    dom.append_child(root, parent);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("afterend".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(root);
    assert_eq!(children, vec![parent, new_elem]);
}

#[test]
fn insert_adjacent_element_invalid_position() {
    let (mut dom, parent, child, mut session) = setup();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let err = InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("invalid".into()),
                JsValue::ObjectRef(child_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
}

#[test]
fn insert_adjacent_element_case_insensitive() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("BeforeEnd".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(parent);
    assert_eq!(children[1], new_elem);
}

#[test]
fn insert_adjacent_text_beforeend() {
    let (mut dom, parent, _, mut session) = setup();
    InsertAdjacentText
        .invoke(
            parent,
            &[
                JsValue::String("beforeend".into()),
                JsValue::String("hello".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let text = collect_text_content(parent, &dom);
    assert_eq!(text, "hello");
}

#[test]
fn serialize_inner_html_excludes_shadow_root_subtree() {
    // WHATWG DOM §4.8 + HTML §2.7.3: `host.innerHTML` MUST NOT leak
    // shadow content (encapsulation).  Without the `ShadowRoot` skip
    // in `serialize_node`, the shadow root child entity (which carries
    // no TagType) would recurse into its own children + serialize them
    // as if they belonged to the host's light DOM — breaching
    // encapsulation for both open and closed shadows.  This regression
    // test locks the skip introduced in PR `#11-shadow-dom-surface` (D-15).
    let mut dom = EcsDom::new();
    let host = dom.create_element("div", Attributes::default());
    // Light-DOM child + shadow root with its own content.
    let light = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(host, light));
    let shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div> succeeds");
    let shadow_child = dom.create_element("b", Attributes::default());
    assert!(dom.append_child(shadow, shadow_child));

    let html = super::tree::serialize_inner_html(host, &dom);
    assert!(
        html.contains("<span>"),
        "light DOM child must serialize: {html}"
    );
    assert!(
        !html.contains("<b>"),
        "shadow root content must NOT serialize: {html}"
    );
}

// ---------------------------------------------------------------------------
// HTML §13.3 "Serializing HTML fragments" — is-value compensation step
// ---------------------------------------------------------------------------

#[test]
fn serialize_emits_is_for_customized_builtin_without_attr() {
    // createElement(tag, {is}) sets no `is` content attribute (DOM
    // §4.5); the serializer must append ` is="..."` from the
    // CustomElementState (the is-value slot) so the markup
    // round-trips. The round-trip itself: element_init re-derives the
    // same component from the emitted attribute.
    let mut dom = EcsDom::new();
    let button = dom.create_element("button", Attributes::default());
    dom.world_mut()
        .insert_one(
            button,
            elidex_custom_elements::CustomElementState::for_created_element(
                "button",
                Some("my-btn"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(button, &dom);
    assert_eq!(html, r#"<button is="my-btn"></button>"#);
}

#[test]
fn serialize_no_double_is_when_attr_present() {
    // Author markup `<button is="my-btn">` carries the attribute in
    // Attributes — the compensation step must not emit a second copy.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("is", "my-btn");
    let button = dom.create_element("button", attrs);
    dom.world_mut()
        .insert_one(
            button,
            elidex_custom_elements::CustomElementState::undefined("my-btn"),
        )
        .unwrap();
    let html = serialize_outer_html(button, &dom);
    assert_eq!(html.matches("is=").count(), 1);
}

#[test]
fn serialize_no_is_for_autonomous_custom_element() {
    // Autonomous CE: definition_name == local name — nothing to
    // compensate (the tag itself carries the identity).
    let mut dom = EcsDom::new();
    let el = dom.create_element("my-widget", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            elidex_custom_elements::CustomElementState::undefined("my-widget"),
        )
        .unwrap();
    let html = serialize_outer_html(el, &dom);
    assert_eq!(html, "<my-widget></my-widget>");
}

#[test]
fn serialize_escapes_synthetic_is_value() {
    // The is value is an arbitrary author string (step 6.3 imposes no
    // validity) — raw emission would inject markup.
    let mut dom = EcsDom::new();
    let button = dom.create_element("button", Attributes::default());
    dom.world_mut()
        .insert_one(
            button,
            elidex_custom_elements::CustomElementState::for_created_element(
                "button",
                Some(r#"x" onclick="evil"#),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(button, &dom);
    assert!(
        html.contains("is=\"x&quot; onclick=&quot;evil\""),
        "synthetic is value must route through escape_attr: {html}"
    );
}

#[test]
fn serialize_emits_is_for_autonomous_element_with_is_value() {
    // Codex PR331 R1: `createElement('my-el', {is: 'my-other'})` — the
    // autonomous branch keys the definition on the tag but the
    // non-null is value lives in its own slot and must serialize
    // (DOM §4.9 sets the is value independently of step 6.3).
    let mut dom = EcsDom::new();
    let el = dom.create_element("my-el", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            elidex_custom_elements::CustomElementState::for_created_element(
                "my-el",
                Some("my-other"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(el, &dom);
    assert_eq!(html, r#"<my-el is="my-other"></my-el>"#);
}

#[test]
fn serialize_emits_is_equal_to_local_name() {
    // Codex PR331 R2: an is value equal to the local name is still
    // non-null — HTML §13.3's condition is nullness, not inequality.
    let mut dom = EcsDom::new();
    let el = dom.create_element("button", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            elidex_custom_elements::CustomElementState::for_created_element(
                "button",
                Some("button"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(el, &dom);
    assert_eq!(html, r#"<button is="button"></button>"#);
}
