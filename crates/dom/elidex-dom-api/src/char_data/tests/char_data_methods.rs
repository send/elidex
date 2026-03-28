use super::*;
use elidex_ecs::{Attributes, TextContent};
use elidex_script_session::{ComponentKind, DomApiErrorKind};

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
