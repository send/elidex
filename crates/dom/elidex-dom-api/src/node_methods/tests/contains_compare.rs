use super::*;

// -----------------------------------------------------------------------
// Contains
// -----------------------------------------------------------------------

#[test]
fn contains_self() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    wrap(div, &mut session);
    let r = Contains
        .invoke(
            div,
            &[obj_ref_arg(div, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Bool(true));
}

#[test]
fn contains_descendant() {
    let (mut dom, mut session) = setup();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    dom.append_child(parent, child);
    wrap(parent, &mut session);
    wrap(child, &mut session);
    let r = Contains
        .invoke(
            parent,
            &[obj_ref_arg(child, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Bool(true));
}

#[test]
fn contains_not_ancestor() {
    let (mut dom, mut session) = setup();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    dom.append_child(parent, child);
    wrap(parent, &mut session);
    wrap(child, &mut session);
    // child does NOT contain parent.
    let r = Contains
        .invoke(
            child,
            &[obj_ref_arg(parent, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

#[test]
fn contains_disconnected() {
    let (mut dom, mut session) = setup();
    let a = dom.create_element("div", Attributes::default());
    let b = dom.create_element("span", Attributes::default());
    wrap(a, &mut session);
    wrap(b, &mut session);
    let r = Contains
        .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

#[test]
fn contains_null() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let r = Contains
        .invoke(div, &[JsValue::Null], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

// -----------------------------------------------------------------------
// CompareDocumentPosition
// -----------------------------------------------------------------------

#[test]
fn compare_position_same() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    wrap(div, &mut session);
    let r = CompareDocumentPosition
        .invoke(
            div,
            &[obj_ref_arg(div, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Number(0.0));
}

#[test]
fn compare_position_following() {
    let (mut dom, mut session) = setup();
    let root = dom.create_document_root();
    let a = dom.create_element("a", Attributes::default());
    let b = dom.create_element("b", Attributes::default());
    dom.append_child(root, a);
    dom.append_child(root, b);
    wrap(a, &mut session);
    wrap(b, &mut session);
    let r = CompareDocumentPosition
        .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Number(f64::from(DOCUMENT_POSITION_FOLLOWING)));
}

#[test]
fn compare_position_preceding() {
    let (mut dom, mut session) = setup();
    let root = dom.create_document_root();
    let a = dom.create_element("a", Attributes::default());
    let b = dom.create_element("b", Attributes::default());
    dom.append_child(root, a);
    dom.append_child(root, b);
    wrap(a, &mut session);
    wrap(b, &mut session);
    let r = CompareDocumentPosition
        .invoke(b, &[obj_ref_arg(a, &mut session)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Number(f64::from(DOCUMENT_POSITION_PRECEDING)));
}

#[test]
fn compare_position_contains() {
    let (mut dom, mut session) = setup();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    dom.append_child(parent, child);
    wrap(parent, &mut session);
    wrap(child, &mut session);
    // child.compareDocumentPosition(parent) → parent CONTAINS child → CONTAINS | PRECEDING
    let r = CompareDocumentPosition
        .invoke(
            child,
            &[obj_ref_arg(parent, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(
        r,
        JsValue::Number(f64::from(
            DOCUMENT_POSITION_CONTAINS | DOCUMENT_POSITION_PRECEDING
        ))
    );
}

#[test]
fn compare_position_contained_by() {
    let (mut dom, mut session) = setup();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    dom.append_child(parent, child);
    wrap(parent, &mut session);
    wrap(child, &mut session);
    // parent.compareDocumentPosition(child) → this CONTAINED_BY child → CONTAINED_BY | FOLLOWING
    let r = CompareDocumentPosition
        .invoke(
            parent,
            &[obj_ref_arg(child, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(
        r,
        JsValue::Number(f64::from(
            DOCUMENT_POSITION_CONTAINED_BY | DOCUMENT_POSITION_FOLLOWING
        ))
    );
}

#[test]
fn compare_position_disconnected() {
    let (mut dom, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let b = dom.create_element("b", Attributes::default());
    wrap(a, &mut session);
    wrap(b, &mut session);
    let r = CompareDocumentPosition
        .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::Number(v) = r {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = v as u32;
        assert!(v & DOCUMENT_POSITION_DISCONNECTED != 0);
        assert!(v & DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC != 0);
    } else {
        panic!("expected Number");
    }
}

// -----------------------------------------------------------------------
// CompareDocumentPosition — Attr node handling
// -----------------------------------------------------------------------

#[test]
fn compare_document_position_attr_uses_owner_element() {
    let (mut dom, mut session) = setup();
    let root = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(root, div);
    let span = dom.create_element("span", Attributes::default());
    dom.append_child(div, span);

    // Create an attr node with owner = div.
    let attr = dom.create_attribute("class");
    {
        let mut ad = dom
            .world_mut()
            .get::<&mut elidex_ecs::AttrData>(attr)
            .unwrap();
        ad.owner_element = Some(div);
    }

    wrap(attr, &mut session);
    wrap(span, &mut session);

    // Attr owned by div should be "before" span (div is before span in tree).
    // The Attr's position should be determined by its ownerElement (div),
    // so span follows div → attr is FOLLOWING from span's perspective.
    let r = CompareDocumentPosition
        .invoke(
            span,
            &[obj_ref_arg(attr, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    if let JsValue::Number(bits) = r {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bits = bits as u32;
        // div contains span, so from span, attr (at div) should have CONTAINS | PRECEDING.
        assert!(bits & DOCUMENT_POSITION_CONTAINS != 0 || bits & DOCUMENT_POSITION_PRECEDING != 0);
    } else {
        panic!("expected number");
    }
}

#[test]
fn compare_document_position_two_attrs_same_element() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());

    let attr1 = dom.create_attribute("id");
    {
        let mut ad = dom
            .world_mut()
            .get::<&mut elidex_ecs::AttrData>(attr1)
            .unwrap();
        ad.owner_element = Some(div);
    }
    let attr2 = dom.create_attribute("class");
    {
        let mut ad = dom
            .world_mut()
            .get::<&mut elidex_ecs::AttrData>(attr2)
            .unwrap();
        ad.owner_element = Some(div);
    }

    wrap(attr1, &mut session);
    wrap(attr2, &mut session);

    let r = CompareDocumentPosition
        .invoke(
            attr1,
            &[obj_ref_arg(attr2, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    if let JsValue::Number(bits) = r {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bits = bits as u32;
        // Same owner element: should have IMPLEMENTATION_SPECIFIC set.
        assert!(bits & DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC != 0);
    } else {
        panic!("expected number");
    }
}
