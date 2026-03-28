use super::*;

// -----------------------------------------------------------------------
// Normalize
// -----------------------------------------------------------------------

#[test]
fn normalize_merge() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("hello ");
    let t2 = dom.create_text("world");
    dom.append_child(div, t1);
    dom.append_child(div, t2);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 1);
    let text = dom
        .world()
        .get::<&TextContent>(children[0])
        .unwrap()
        .0
        .clone();
    assert_eq!(text, "hello world");
}

#[test]
fn normalize_remove_empty() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t = dom.create_text("");
    dom.append_child(div, t);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    assert!(dom.children(div).is_empty());
}

#[test]
fn normalize_no_change() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let t = dom.create_text("hello");
    dom.append_child(div, span);
    dom.append_child(div, t);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    assert_eq!(dom.children(div).len(), 2);
}

#[test]
fn normalize_recursive() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let t1 = dom.create_text("a");
    let t2 = dom.create_text("b");
    dom.append_child(div, span);
    dom.append_child(span, t1);
    dom.append_child(span, t2);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    let span_children = dom.children(span);
    assert_eq!(span_children.len(), 1);
    let text = dom
        .world()
        .get::<&TextContent>(span_children[0])
        .unwrap()
        .0
        .clone();
    assert_eq!(text, "ab");
}

// -----------------------------------------------------------------------
// Normalize — sibling-walk fix (H4)
// -----------------------------------------------------------------------

#[test]
fn normalize_adjacent_merge() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("hello ");
    let t2 = dom.create_text("world");
    dom.append_child(div, t1);
    dom.append_child(div, t2);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 1);
    let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
    assert_eq!(tc.0, "hello world");
}

#[test]
fn normalize_removes_empty() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("");
    let t2 = dom.create_text("hello");
    dom.append_child(div, t1);
    dom.append_child(div, t2);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 1);
    let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
    assert_eq!(tc.0, "hello");
}

#[test]
fn normalize_three_adjacent() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("a");
    let t2 = dom.create_text("b");
    let t3 = dom.create_text("c");
    dom.append_child(div, t1);
    dom.append_child(div, t2);
    dom.append_child(div, t3);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 1);
    let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
    assert_eq!(tc.0, "abc");
}

#[test]
fn normalize_comment_boundary() {
    // Text nodes separated by a comment should NOT be merged.
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("before");
    let comment = dom.create_comment("separator");
    let t2 = dom.create_text("after");
    dom.append_child(div, t1);
    dom.append_child(div, comment);
    dom.append_child(div, t2);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 3, "comment should prevent text merge");
    let tc1 = dom.world().get::<&TextContent>(children[0]).unwrap();
    assert_eq!(tc1.0, "before");
    let tc2 = dom.world().get::<&TextContent>(children[2]).unwrap();
    assert_eq!(tc2.0, "after");
}

#[test]
fn normalize_all_empty_text() {
    // All empty text nodes should be removed.
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t1 = dom.create_text("");
    let t2 = dom.create_text("");
    dom.append_child(div, t1);
    dom.append_child(div, t2);

    Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

    assert!(dom.children(div).is_empty());
}

// -----------------------------------------------------------------------
// GetTextContentNodeKind
// -----------------------------------------------------------------------

#[test]
fn text_content_get_element() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let t = dom.create_text("hello");
    dom.append_child(div, t);

    let r = GetTextContentNodeKind
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::String("hello".into()));
}

#[test]
fn text_content_get_document_null() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();

    let r = GetTextContentNodeKind
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Null);
}

#[test]
fn text_content_get_doctype_null() {
    let (mut dom, mut session) = setup();
    let dt = dom.create_document_type("html", "", "");

    let r = GetTextContentNodeKind
        .invoke(dt, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Null);
}

#[test]
fn text_content_get_comment() {
    let (mut dom, mut session) = setup();
    let comment = dom.create_comment("test comment");

    let r = GetTextContentNodeKind
        .invoke(comment, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::String("test comment".into()));
}

#[test]
fn text_content_get_text_node() {
    let (mut dom, mut session) = setup();
    let t = dom.create_text("direct text");

    let r = GetTextContentNodeKind
        .invoke(t, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::String("direct text".into()));
}

#[test]
fn text_content_get_element_empty() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());

    let r = GetTextContentNodeKind
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::String(String::new()));
}

// -----------------------------------------------------------------------
// SetTextContentNodeKind
// -----------------------------------------------------------------------

#[test]
fn text_content_set_element() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let old = dom.create_text("old");
    dom.append_child(div, old);

    SetTextContentNodeKind
        .invoke(
            div,
            &[JsValue::String("new".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 1);
    let text = dom
        .world()
        .get::<&TextContent>(children[0])
        .unwrap()
        .0
        .clone();
    assert_eq!(text, "new");
    // Old child removed (different entity).
    assert_ne!(children[0], old);
}

#[test]
fn text_content_set_document_noop() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let child = dom.create_element("html", Attributes::default());
    dom.append_child(doc, child);

    SetTextContentNodeKind
        .invoke(
            doc,
            &[JsValue::String("ignored".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    // Children unchanged.
    assert_eq!(dom.children(doc).len(), 1);
}

#[test]
fn text_content_set_comment() {
    let (mut dom, mut session) = setup();
    let comment = dom.create_comment("old");

    SetTextContentNodeKind
        .invoke(
            comment,
            &[JsValue::String("new".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let data = dom.world().get::<&CommentData>(comment).unwrap().0.clone();
    assert_eq!(data, "new");
}

// -----------------------------------------------------------------------
// SetNodeValue
// -----------------------------------------------------------------------

#[test]
fn node_value_set_text() {
    let (mut dom, mut session) = setup();
    let t = dom.create_text("old");

    SetNodeValue
        .invoke(t, &[JsValue::String("new".into())], &mut session, &mut dom)
        .unwrap();

    let text = dom.world().get::<&TextContent>(t).unwrap().0.clone();
    assert_eq!(text, "new");
}

#[test]
fn node_value_set_comment() {
    let (mut dom, mut session) = setup();
    let c = dom.create_comment("old");

    SetNodeValue
        .invoke(c, &[JsValue::String("new".into())], &mut session, &mut dom)
        .unwrap();

    let data = dom.world().get::<&CommentData>(c).unwrap().0.clone();
    assert_eq!(data, "new");
}

#[test]
fn node_value_set_element_noop() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());

    let r = SetNodeValue
        .invoke(
            div,
            &[JsValue::String("ignored".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Undefined);
}

#[test]
fn node_value_set_document_noop() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();

    let r = SetNodeValue
        .invoke(
            doc,
            &[JsValue::String("ignored".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Undefined);
}

// -----------------------------------------------------------------------
// textContent / nodeValue — CdataSection (M7) + existing behavior
// -----------------------------------------------------------------------

#[test]
fn text_content_text_node_direct() {
    let (mut dom, mut session) = setup();
    let text = dom.create_text("hello");

    let result = GetTextContentNodeKind
        .invoke(text, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("hello".into()));
}

#[test]
fn set_text_content_text_node_direct() {
    let (mut dom, mut session) = setup();
    let text = dom.create_text("old");

    SetTextContentNodeKind
        .invoke(
            text,
            &[JsValue::String("new".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let tc = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(tc.0, "new");
}

#[test]
fn set_node_value_text_node_direct() {
    let (mut dom, mut session) = setup();
    let text = dom.create_text("old");

    SetNodeValue
        .invoke(
            text,
            &[JsValue::String("new".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let tc = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(tc.0, "new");
}
