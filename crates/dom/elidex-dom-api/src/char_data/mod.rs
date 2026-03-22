//! `CharacterData` interface methods, `Attr` node handlers, `DocumentType` handlers,
//! and additional `Document` property handlers.

mod attr;
mod char_data_handlers;
mod doctype;
mod document_props;

pub use attr::{
    CreateAttribute, GetAttrName, GetAttrSpecified, GetAttrValue, GetAttributeNode,
    GetOwnerElement, RemoveAttributeNode, SetAttrValue, SetAttributeNode,
};
pub use char_data_handlers::{
    AppendData, DeleteData, GetData, GetLength, InsertData, ReplaceData, SetData, SplitText,
    SubstringData,
};
pub use doctype::{GetDoctype, GetDoctypeName, GetDoctypePublicId, GetDoctypeSystemId};
pub use document_props::{
    CreateComment, CreateDocumentFragment, GetBody, GetCharacterSet, GetCompatMode,
    GetDocumentElement, GetDocumentUrl, GetHead, GetReadyState, GetTitle, SetTitle,
};

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, CommentData, EcsDom, Entity, NodeKind, TextContent};
    use elidex_plugin::JsValue;
    use elidex_script_session::{
        ComponentKind, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
    };

    use elidex_ecs::AttrData;

    // -----------------------------------------------------------------------
    // Setup helpers
    // -----------------------------------------------------------------------

    fn setup_text() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let text = dom.create_text("Hello, world!");
        let session = SessionCore::new();
        (dom, text, session)
    }

    fn setup_comment() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let comment = dom.create_comment("a comment");
        let session = SessionCore::new();
        (dom, comment, session)
    }

    fn setup_document() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let doctype = dom.create_document_type(
            "html",
            "-//W3C//DTD HTML 4.01//EN",
            "http://www.w3.org/TR/html4/strict.dtd",
        );
        let html = dom.create_element("html", Attributes::default());
        let head = dom.create_element("head", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, doctype);
        dom.append_child(doc, html);
        dom.append_child(html, head);
        dom.append_child(html, body);
        let session = SessionCore::new();
        (dom, doc, session)
    }

    /// Find the first child element of `parent` with tag matching `tag_name`.
    fn find_child_element(dom: &EcsDom, parent: Entity, tag_name: &str) -> Option<Entity> {
        use elidex_ecs::TagType;
        for child in dom.children_iter(parent) {
            if let Ok(tag) = dom.world().get::<&TagType>(child) {
                if tag.0 == tag_name {
                    return Some(child);
                }
            }
        }
        None
    }

    /// Walk document children to find the first entity with `NodeKind::DocumentType`.
    fn find_doctype(dom: &EcsDom, doc: Entity) -> Option<Entity> {
        for child in dom.children_iter(doc) {
            if let Ok(nk) = dom.world().get::<&NodeKind>(child) {
                if *nk == NodeKind::DocumentType {
                    return Some(child);
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // CharacterData tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_data_text() {
        let (mut dom, text, mut session) = setup_text();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, world!".into()));
    }

    #[test]
    fn get_data_comment() {
        let (mut dom, comment, mut session) = setup_comment();
        let result = GetData
            .invoke(comment, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("a comment".into()));
    }

    #[test]
    fn get_data_element_error() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = GetData.invoke(div, &[], &mut session, &mut dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::InvalidStateError);
    }

    #[test]
    fn set_data_text() {
        let (mut dom, text, mut session) = setup_text();
        SetData
            .invoke(
                text,
                &[JsValue::String("new data".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("new data".into()));
    }

    #[test]
    fn set_data_comment() {
        let (mut dom, comment, mut session) = setup_comment();
        SetData
            .invoke(
                comment,
                &[JsValue::String("updated".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData
            .invoke(comment, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("updated".into()));
    }

    #[test]
    fn get_length() {
        let (mut dom, text, mut session) = setup_text();
        let result = GetLength.invoke(text, &[], &mut session, &mut dom).unwrap();
        // "Hello, world!" = 13 UTF-16 code units (all BMP)
        assert_eq!(result, JsValue::Number(13.0));
    }

    #[test]
    fn get_length_utf16_surrogate() {
        let mut dom = EcsDom::new();
        // U+1F44D is 1 Unicode code point but 2 UTF-16 code units
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        let result = GetLength.invoke(text, &[], &mut session, &mut dom).unwrap();
        // 'A' = 1, thumbs up = 2, 'B' = 1 -> 4 UTF-16 code units
        assert_eq!(result, JsValue::Number(4.0));
    }

    #[test]
    fn substring_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        // substringData(1, 2) should extract the emoji (2 UTF-16 code units)
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(1.0), JsValue::Number(2.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("\u{1F44D}".into()));
    }

    #[test]
    fn split_text_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        // splitText(3) -- after 'A' (1) + emoji (2) = offset 3
        SplitText
            .invoke(text, &[JsValue::Number(3.0)], &mut session, &mut dom)
            .unwrap();
        let head = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(head, JsValue::String("A\u{1F44D}".into()));
    }

    #[test]
    fn split_text_offset_zero() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");
        let mut session = SessionCore::new();
        SplitText
            .invoke(text, &[JsValue::Number(0.0)], &mut session, &mut dom)
            .unwrap();
        let head = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(head, JsValue::String(String::new()));
    }

    #[test]
    fn split_text_offset_at_length() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");
        let mut session = SessionCore::new();
        SplitText
            .invoke(text, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        let head = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(head, JsValue::String("hello".into()));
    }

    #[test]
    fn insert_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        InsertData
            .invoke(
                text,
                &[JsValue::Number(3.0), JsValue::String("X".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("A\u{1F44D}XB".into()));
    }

    #[test]
    fn delete_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        DeleteData
            .invoke(
                text,
                &[JsValue::Number(1.0), JsValue::Number(2.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("AB".into()));
    }

    #[test]
    fn replace_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        ReplaceData
            .invoke(
                text,
                &[
                    JsValue::Number(1.0),
                    JsValue::Number(2.0),
                    JsValue::String("XY".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("AXYB".into()));
    }

    #[test]
    fn insert_data_at_length() {
        let (mut dom, text, mut session) = setup_text();
        InsertData
            .invoke(
                text,
                &[JsValue::Number(13.0), JsValue::String("!".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("Hello, world!!".into()));
    }

    #[test]
    fn substring_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(0.0), JsValue::Number(5.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("Hello".into()));
    }

    #[test]
    fn substring_data_middle() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(7.0), JsValue::Number(5.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("world".into()));
    }

    #[test]
    fn substring_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData.invoke(
            text,
            &[JsValue::Number(100.0), JsValue::Number(5.0)],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn substring_data_count_exceeds() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(10.0), JsValue::Number(100.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("ld!".into()));
    }

    #[test]
    fn append_data() {
        let (mut dom, text, mut session) = setup_text();
        AppendData
            .invoke(
                text,
                &[JsValue::String(" Goodbye!".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, world! Goodbye!".into()));
    }

    #[test]
    fn insert_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        InsertData
            .invoke(
                text,
                &[JsValue::Number(7.0), JsValue::String("beautiful ".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, beautiful world!".into()));
    }

    #[test]
    fn insert_data_at_start() {
        let (mut dom, text, mut session) = setup_text();
        InsertData
            .invoke(
                text,
                &[JsValue::Number(0.0), JsValue::String(">> ".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String(">> Hello, world!".into()));
    }

    #[test]
    fn insert_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = InsertData.invoke(
            text,
            &[JsValue::Number(100.0), JsValue::String("x".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn delete_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        DeleteData
            .invoke(
                text,
                &[JsValue::Number(5.0), JsValue::Number(7.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello!".into()));
    }

    #[test]
    fn delete_data_count_exceeds() {
        let (mut dom, text, mut session) = setup_text();
        DeleteData
            .invoke(
                text,
                &[JsValue::Number(10.0), JsValue::Number(100.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, wor".into()));
    }

    #[test]
    fn delete_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = DeleteData.invoke(
            text,
            &[JsValue::Number(100.0), JsValue::Number(1.0)],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn replace_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        ReplaceData
            .invoke(
                text,
                &[
                    JsValue::Number(7.0),
                    JsValue::Number(5.0),
                    JsValue::String("Rust".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, Rust!".into()));
    }

    #[test]
    fn replace_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = ReplaceData.invoke(
            text,
            &[
                JsValue::Number(100.0),
                JsValue::Number(1.0),
                JsValue::String("x".into()),
            ],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn split_text_valid() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let text = dom.create_text("HelloWorld");
        dom.append_child(parent, text);
        let mut session = SessionCore::new();
        session.get_or_create_wrapper(text, ComponentKind::Element);

        let result = SplitText
            .invoke(text, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));

        let orig = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(orig, JsValue::String("Hello".into()));

        let children: Vec<Entity> = dom.children_iter(parent).collect();
        assert_eq!(children.len(), 2);

        let second_data = GetData
            .invoke(children[1], &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(second_data, JsValue::String("World".into()));
    }

    #[test]
    fn split_text_out_of_bounds() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("Hello");
        let mut session = SessionCore::new();
        let result = SplitText.invoke(text, &[JsValue::Number(100.0)], &mut session, &mut dom);
        assert!(result.is_err());
    }

    #[test]
    fn split_text_inserts_after() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let text1 = dom.create_text("AB");
        let text2 = dom.create_text("CD");
        dom.append_child(parent, text1);
        dom.append_child(parent, text2);
        let mut session = SessionCore::new();

        SplitText
            .invoke(text1, &[JsValue::Number(1.0)], &mut session, &mut dom)
            .unwrap();

        let children: Vec<Entity> = dom.children_iter(parent).collect();
        assert_eq!(children.len(), 3);
        let d0 = GetData
            .invoke(children[0], &[], &mut session, &mut dom)
            .unwrap();
        let d1 = GetData
            .invoke(children[1], &[], &mut session, &mut dom)
            .unwrap();
        let d2 = GetData
            .invoke(children[2], &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(d0, JsValue::String("A".into()));
        assert_eq!(d1, JsValue::String("B".into()));
        assert_eq!(d2, JsValue::String("CD".into()));
    }

    #[test]
    fn split_text_on_element_error() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = SplitText.invoke(div, &[JsValue::Number(0.0)], &mut session, &mut dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::InvalidStateError);
    }

    // -----------------------------------------------------------------------
    // Attr node tests
    // -----------------------------------------------------------------------

    #[test]
    fn create_attribute() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = CreateAttribute
            .invoke(
                doc,
                &[JsValue::String("Data-X".into())],
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
            let ad = dom.world().get::<&AttrData>(entity).unwrap();
            assert_eq!(ad.local_name, "data-x");
        }
    }

    #[test]
    fn get_attribute_node_exists() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "active");
        let div = dom.create_element("div", attrs);
        let mut session = SessionCore::new();

        let result = GetAttributeNode
            .invoke(
                div,
                &[JsValue::String("class".into())],
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
            let ad = dom.world().get::<&AttrData>(entity).unwrap();
            assert_eq!(ad.local_name, "class");
            assert_eq!(ad.value, "active");
            assert_eq!(ad.owner_element, Some(div));
        }
    }

    #[test]
    fn get_attribute_node_missing() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();

        let result = GetAttributeNode
            .invoke(
                div,
                &[JsValue::String("nonexistent".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn set_attribute_node() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let attr = dom.create_attribute("data-x");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "42".into();
        }
        let mut session = SessionCore::new();
        let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

        SetAttributeNode
            .invoke(
                div,
                &[JsValue::ObjectRef(attr_ref.to_raw())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(attrs.get("data-x"), Some("42"));

        let ad = dom.world().get::<&AttrData>(attr).unwrap();
        assert_eq!(ad.owner_element, Some(div));
    }

    #[test]
    fn set_attribute_node_in_use_error() {
        let mut dom = EcsDom::new();
        let div1 = dom.create_element("div", Attributes::default());
        let div2 = dom.create_element("div", Attributes::default());
        let attr = dom.create_attribute("data-x");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "42".into();
            ad.owner_element = Some(div1);
        }
        let mut session = SessionCore::new();
        let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

        let result = SetAttributeNode.invoke(
            div2,
            &[JsValue::ObjectRef(attr_ref.to_raw())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::InUseAttributeError
        );
    }

    #[test]
    fn remove_attribute_node() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("data-x", "42");
        let div = dom.create_element("div", attrs);
        let attr = dom.create_attribute("data-x");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "42".into();
            ad.owner_element = Some(div);
        }
        let mut session = SessionCore::new();
        let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

        RemoveAttributeNode
            .invoke(
                div,
                &[JsValue::ObjectRef(attr_ref.to_raw())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let el_attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert!(!el_attrs.contains("data-x"));

        let ad = dom.world().get::<&AttrData>(attr).unwrap();
        assert_eq!(ad.owner_element, None);
    }

    #[test]
    fn attr_name() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("class");
        let mut session = SessionCore::new();
        let result = GetAttrName
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("class".into()));
    }

    #[test]
    fn attr_value_get_set() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("class");
        let mut session = SessionCore::new();

        let result = GetAttrValue
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));

        SetAttrValue
            .invoke(
                attr,
                &[JsValue::String("active".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetAttrValue
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("active".into()));
    }

    #[test]
    fn attr_value_syncs_to_owner() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "old");
        let div = dom.create_element("div", attrs);
        let attr = dom.create_attribute("class");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "old".into();
            ad.owner_element = Some(div);
        }
        let mut session = SessionCore::new();

        SetAttrValue
            .invoke(
                attr,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let el_attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(el_attrs.get("class"), Some("new"));
    }

    #[test]
    fn attr_owner_element() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let attr = dom.create_attribute("id");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.owner_element = Some(div);
        }
        let mut session = SessionCore::new();

        let result = GetOwnerElement
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn attr_owner_element_null() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("id");
        let mut session = SessionCore::new();
        let result = GetOwnerElement
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn attr_specified() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("id");
        let mut session = SessionCore::new();
        let result = GetAttrSpecified
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    // -----------------------------------------------------------------------
    // DocumentType tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_doctype() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn get_doctype_name() {
        let (mut dom, doc, mut session) = setup_document();
        let dt_entity = find_doctype(&dom, doc).unwrap();
        let result = GetDoctypeName
            .invoke(dt_entity, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("html".into()));
    }

    #[test]
    fn get_doctype_public_id() {
        let (mut dom, doc, mut session) = setup_document();
        let dt_entity = find_doctype(&dom, doc).unwrap();
        let result = GetDoctypePublicId
            .invoke(dt_entity, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("-//W3C//DTD HTML 4.01//EN".into()));
    }

    #[test]
    fn get_doctype_system_id() {
        let (mut dom, doc, mut session) = setup_document();
        let dt_entity = find_doctype(&dom, doc).unwrap();
        let result = GetDoctypeSystemId
            .invoke(dt_entity, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(
            result,
            JsValue::String("http://www.w3.org/TR/html4/strict.dtd".into())
        );
    }

    #[test]
    fn get_doctype_none() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        dom.append_child(doc, html);
        let mut session = SessionCore::new();
        let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // -----------------------------------------------------------------------
    // Document property tests
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Step 4 tests: rev_version, IndexSizeError, validation, spec fixes
    // -----------------------------------------------------------------------

    #[test]
    fn set_data_rev_version() {
        let (mut dom, text, mut session) = setup_text();
        let parent = dom.create_element("div", Attributes::default());
        dom.append_child(parent, text);
        let v1 = dom.inclusive_descendants_version(text);
        SetData
            .invoke(
                text,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(text);
        assert_ne!(v1, v2);
    }

    #[test]
    fn append_data_rev_version() {
        let (mut dom, text, mut session) = setup_text();
        let parent = dom.create_element("div", Attributes::default());
        dom.append_child(parent, text);
        let v1 = dom.inclusive_descendants_version(text);
        AppendData
            .invoke(
                text,
                &[JsValue::String(" extra".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(text);
        assert_ne!(v1, v2);
    }

    #[test]
    fn index_size_error_kind() {
        let (mut dom, text, mut session) = setup_text();
        let err = SubstringData
            .invoke(
                text,
                &[JsValue::Number(999.0), JsValue::Number(1.0)],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::IndexSizeError);
    }

    #[test]
    fn create_attribute_validates_name() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = CreateAttribute.invoke(
            doc,
            &[JsValue::String("invalid name".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn split_text_still_works() {
        let (mut dom, text, mut session) = setup_text();
        let parent = dom.create_element("div", Attributes::default());
        dom.append_child(parent, text);
        session.get_or_create_wrapper(text, ComponentKind::Element);

        let result = SplitText
            .invoke(text, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "Hello");
    }

    #[test]
    fn remove_attribute_node_wrong_owner() {
        let mut dom = EcsDom::new();
        let elem1 = dom.create_element("div", Attributes::default());
        let elem2 = dom.create_element("span", Attributes::default());
        {
            let mut a1 = dom.world_mut().get::<&mut Attributes>(elem1).unwrap();
            a1.set("foo", "bar");
        }
        let mut session = SessionCore::new();
        session.get_or_create_wrapper(elem1, ComponentKind::Element);
        session.get_or_create_wrapper(elem2, ComponentKind::Element);

        let attr_result = GetAttributeNode
            .invoke(
                elem1,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = RemoveAttributeNode.invoke(elem2, &[attr_result], &mut session, &mut dom);
        assert!(result.is_err());
    }

    #[test]
    fn set_attribute_node_returns_null() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        session.get_or_create_wrapper(elem, ComponentKind::Element);

        let attr = dom.create_attribute("foo");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "bar".into();
        }
        let attr_ref = session
            .get_or_create_wrapper(attr, ComponentKind::Element)
            .to_raw();

        let result = SetAttributeNode
            .invoke(
                elem,
                &[JsValue::ObjectRef(attr_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
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
}
