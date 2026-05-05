use super::*;
use elidex_ecs::CommentData;
use elidex_script_session::JsObjectRef;

#[test]
fn document_url() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetDocumentUrl
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("about:blank".into()));
}

#[test]
fn ready_state() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetReadyState
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("loading".into()));
}

#[test]
fn compat_mode() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetCompatMode
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("CSS1Compat".into()));
}

#[test]
fn character_set() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetCharacterSet
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("UTF-8".into()));
}

#[test]
fn document_element() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetDocumentElement
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn document_element_empty() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = GetDocumentElement
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn document_head() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetHead.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn document_head_missing() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = GetHead.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn document_body() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn document_body_missing() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(doc, html);
    let mut session = SessionCore::new();
    let result = GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn title_get() {
    let (mut dom, doc, mut session) = setup_document();
    let html = find_child_element(&dom, doc, "html").unwrap();
    let head = find_child_element(&dom, html, "head").unwrap();
    let title = dom.create_element("title", Attributes::default());
    let text = dom.create_text("  Hello  World  ");
    dom.append_child(head, title);
    dom.append_child(title, text);

    let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("Hello World".into()));
}

#[test]
fn title_get_empty() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String(String::new()));
}

#[test]
fn title_get_preserves_non_ascii_whitespace() {
    // WHATWG HTML §dom-document-title strips and collapses **ASCII**
    // whitespace only (U+0009/A/C/D/20).  NBSP (U+00A0) and the
    // ideographic space (U+3000) are NOT in that set and must be
    // preserved as content.  Rust's `split_whitespace` would collapse
    // them too — pinning the ASCII-only behaviour here.
    let (mut dom, doc, mut session) = setup_document();
    let html = find_child_element(&dom, doc, "html").unwrap();
    let head = find_child_element(&dom, html, "head").unwrap();
    let title = dom.create_element("title", Attributes::default());
    let text = dom.create_text("a\u{00A0}b\u{3000}c");
    dom.append_child(head, title);
    dom.append_child(title, text);

    let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("a\u{00A0}b\u{3000}c".into()));
}

#[test]
fn title_set() {
    let (mut dom, doc, mut session) = setup_document();

    SetTitle
        .invoke(
            doc,
            &[JsValue::String("New Title".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("New Title".into()));
}

#[test]
fn title_set_creates_element() {
    let (mut dom, doc, mut session) = setup_document();
    SetTitle
        .invoke(
            doc,
            &[JsValue::String("Created".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let html = find_child_element(&dom, doc, "html").unwrap();
    let head = find_child_element(&dom, html, "head").unwrap();
    assert!(find_child_element(&dom, head, "title").is_some());
}

#[test]
fn document_accessors_match_html_tags_case_insensitively() {
    // Pre-arch-hoist-c, the VM-side getters used
    // `EcsDom::first_child_with_tag` which is ASCII case-insensitive
    // (matching HTML element identity rules — TagType stores the
    // raw tag, not the normalized localName).  After migration the
    // handlers must preserve that policy: `<HTML>` / `<HEAD>` /
    // `<BODY>` / `<TITLE>` constructed via `dom.create_element` with
    // upper-case tags must still resolve through
    // `document.{documentElement, head, body, title}` accessors.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("HTML", Attributes::default());
    let head = dom.create_element("HEAD", Attributes::default());
    let body = dom.create_element("BODY", Attributes::default());
    let title = dom.create_element("TITLE", Attributes::default());
    let title_text = dom.create_text("Mixed Case");
    dom.append_child(doc, html);
    dom.append_child(html, head);
    dom.append_child(html, body);
    dom.append_child(head, title);
    dom.append_child(title, title_text);

    let mut session = SessionCore::new();

    assert!(matches!(
        GetDocumentElement
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap(),
        JsValue::ObjectRef(_)
    ));
    assert!(matches!(
        GetHead.invoke(doc, &[], &mut session, &mut dom).unwrap(),
        JsValue::ObjectRef(_)
    ));
    assert!(matches!(
        GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap(),
        JsValue::ObjectRef(_)
    ));
    assert_eq!(
        GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap(),
        JsValue::String("Mixed Case".into())
    );
}

#[test]
fn document_body_accepts_uppercase_frameset() {
    // Same case-insensitive policy for the `<frameset>` branch of
    // GetBody.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("HTML", Attributes::default());
    let frameset = dom.create_element("FRAMESET", Attributes::default());
    dom.append_child(doc, html);
    dom.append_child(html, frameset);
    let mut session = SessionCore::new();

    assert!(matches!(
        GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap(),
        JsValue::ObjectRef(_)
    ));
}

#[test]
fn title_set_anchors_synthesised_nodes_to_receiver_document() {
    // The receiver Document's "node document" must own the synthesised
    // <title> element + its text-node child even when the receiver is
    // not the bound document — same WHATWG DOM §4.4 contract as the
    // create* family.  Pre-fix, `SetTitle` used `create_element` /
    // `create_text` (no owner) so a setter call on a cloned doc
    // would put the new nodes under whatever document the parent
    // tree happened to point at.
    let (mut dom, doc, mut session) = setup_document();
    SetTitle
        .invoke(
            doc,
            &[JsValue::String("Anchored".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let html = find_child_element(&dom, doc, "html").unwrap();
    let head = find_child_element(&dom, html, "head").unwrap();
    let title = find_child_element(&dom, head, "title").expect("title created");
    assert_eq!(dom.owner_document(title), Some(doc));
    let text = dom.children_iter(title).next().expect("text child");
    assert_eq!(dom.owner_document(text), Some(doc));
}

#[test]
fn title_set_replaces_existing() {
    let (mut dom, doc, mut session) = setup_document();
    let html = find_child_element(&dom, doc, "html").unwrap();
    let head = find_child_element(&dom, html, "head").unwrap();
    let title = dom.create_element("title", Attributes::default());
    let text = dom.create_text("Old Title");
    dom.append_child(head, title);
    dom.append_child(title, text);

    SetTitle
        .invoke(
            doc,
            &[JsValue::String("New Title".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("New Title".into()));
}

#[test]
fn create_document_fragment() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = CreateDocumentFragment
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));

    if let JsValue::ObjectRef(id) = result {
        let (entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(id))
            .unwrap();
        let nk = dom.world().get::<&NodeKind>(entity).unwrap();
        assert_eq!(*nk, NodeKind::DocumentFragment);
    }
}

#[test]
fn create_comment() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = CreateComment
        .invoke(
            doc,
            &[JsValue::String("test comment".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));

    if let JsValue::ObjectRef(id) = result {
        let (entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(id))
            .unwrap();
        let cd = dom.world().get::<&CommentData>(entity).unwrap();
        assert_eq!(cd.0, "test comment");
    }
}

#[test]
fn title_child_text_only() {
    let (mut dom, doc, mut session) = setup_document();
    let html_entity = dom
        .children_iter(doc)
        .find(|e| dom.has_tag(*e, "html"))
        .unwrap();
    let head = dom
        .children_iter(html_entity)
        .find(|e| dom.has_tag(*e, "head"))
        .unwrap();
    let title = dom.create_element("title", Attributes::default());
    dom.append_child(head, title);
    let text = dom.create_text("Hello ");
    dom.append_child(title, text);
    let span = dom.create_element("span", Attributes::default());
    dom.append_child(title, span);
    let inner_text = dom.create_text("World");
    dom.append_child(span, inner_text);

    let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
    // Per spec: should only get direct child text, not descendant.
    assert_eq!(result, JsValue::String("Hello".into()));
}

#[test]
fn body_frameset() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(doc, html);
    let frameset = dom.create_element("frameset", Attributes::default());
    dom.append_child(html, frameset);
    let mut session = SessionCore::new();

    let result = GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}
