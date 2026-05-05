//! Element-level DOM API handlers: appendChild, insertBefore, removeChild,
//! getAttribute/setAttribute/removeAttribute, textContent, innerHTML.

mod attrs;
pub(crate) mod layout_query;
mod props;
pub(crate) mod tree;

pub use attrs::{camel_to_data_attr, data_attr_to_camel};
pub use attrs::{
    DatasetDelete, DatasetGet, DatasetKeys, DatasetSet, GetAttributeNames, GetClassName, GetId,
    HasAttribute, SetClassName, SetId, ToggleAttribute,
};
pub use layout_query::{
    GetBoundingClientRect, GetClientHeight, GetClientLeft, GetClientRects, GetClientTop,
    GetClientWidth, GetOffsetHeight, GetOffsetLeft, GetOffsetParent, GetOffsetTop, GetOffsetWidth,
    GetScrollHeight, GetScrollLeft, GetScrollTop, GetScrollWidth, ScrollIntoView,
};
pub use props::{GetAttribute, RemoveAttribute, SetAttribute};
pub use tree::{
    collect_text_content, serialize_inner_html, validate_attribute_name, AppendChild, GetInnerHtml,
    InsertAdjacentElement, InsertAdjacentHtml, InsertAdjacentText, InsertBefore, RemoveChild,
    ReplaceChild, SetInnerHtml,
};

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom, Entity};
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
    fn replace_child_self_replace_is_hierarchy_request_error() {
        // newChild == parent: WHATWG §4.4 step 2 (host-including
        // ancestor check). EcsDom::replace_child rejects when
        // newChild == parent.
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

    // -----------------------------------------------------------------------
    // hasAttribute tests
    // -----------------------------------------------------------------------

    #[test]
    fn has_attribute_true() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("id", "test");
        }
        let result = HasAttribute
            .invoke(
                parent,
                &[JsValue::String("id".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn has_attribute_false() {
        let (mut dom, parent, _, mut session) = setup();
        let result = HasAttribute
            .invoke(
                parent,
                &[JsValue::String("id".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // toggleAttribute tests
    // -----------------------------------------------------------------------

    #[test]
    fn toggle_attribute_adds_when_absent() {
        let (mut dom, parent, _, mut session) = setup();
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert_eq!(attrs.get("hidden"), Some(""));
    }

    #[test]
    fn toggle_attribute_removes_when_present() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("hidden", "");
        }
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("hidden"));
    }

    #[test]
    fn toggle_attribute_force_true() {
        let (mut dom, parent, _, mut session) = setup();
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into()), JsValue::Bool(true)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(attrs.contains("hidden"));
    }

    #[test]
    fn toggle_attribute_force_false() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("hidden", "");
        }
        let result = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into()), JsValue::Bool(false)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("hidden"));
    }

    #[test]
    fn toggle_attribute_rejects_invalid_name() {
        let (mut dom, parent, _, mut session) = setup();
        let err = ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String(String::new())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    // -----------------------------------------------------------------------
    // getAttributeNames tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_attribute_names() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("id", "x");
            attrs.set("class", "y");
        }
        let result = GetAttributeNames
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::String(s) = result {
            let names: Vec<&str> = s.split('\0').collect();
            assert!(names.contains(&"id"));
            assert!(names.contains(&"class"));
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn get_attribute_names_empty() {
        let (mut dom, parent, _, mut session) = setup();
        let result = GetAttributeNames
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    // -----------------------------------------------------------------------
    // className getter/setter tests
    // -----------------------------------------------------------------------

    #[test]
    fn classname_get_set() {
        let (mut dom, parent, _, mut session) = setup();
        // Initially empty.
        let result = GetClassName
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));

        // Set.
        SetClassName
            .invoke(
                parent,
                &[JsValue::String("foo bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetClassName
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("foo bar".into()));
    }

    // -----------------------------------------------------------------------
    // id getter/setter tests
    // -----------------------------------------------------------------------

    #[test]
    fn id_get_set() {
        let (mut dom, parent, _, mut session) = setup();
        let result = GetId.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String(String::new()));

        SetId
            .invoke(
                parent,
                &[JsValue::String("main".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetId.invoke(parent, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("main".into()));
    }

    // -----------------------------------------------------------------------
    // data_attr_to_camel / camel_to_data_attr tests
    // -----------------------------------------------------------------------

    #[test]
    fn data_attr_to_camel_basic() {
        assert_eq!(data_attr_to_camel("data-foo-bar"), "fooBar");
        assert_eq!(data_attr_to_camel("data-x"), "x");
        assert_eq!(data_attr_to_camel("data-foo-bar-baz"), "fooBarBaz");
    }

    #[test]
    fn camel_to_data_attr_basic() {
        assert_eq!(camel_to_data_attr("fooBar"), "data-foo-bar");
        assert_eq!(camel_to_data_attr("x"), "data-x");
        assert_eq!(camel_to_data_attr("fooBarBaz"), "data-foo-bar-baz");
    }

    #[test]
    fn data_attr_roundtrip() {
        let camel = data_attr_to_camel("data-my-value");
        let attr = camel_to_data_attr(&camel);
        assert_eq!(attr, "data-my-value");
    }

    // -----------------------------------------------------------------------
    // dataset tests
    // -----------------------------------------------------------------------

    #[test]
    fn dataset_set_and_get() {
        let (mut dom, parent, _, mut session) = setup();
        DatasetSet
            .invoke(
                parent,
                &[
                    JsValue::String("fooBar".into()),
                    JsValue::String("42".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = DatasetGet
            .invoke(
                parent,
                &[JsValue::String("fooBar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("42".into()));

        // Verify it's stored as data-foo-bar.
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert_eq!(attrs.get("data-foo-bar"), Some("42"));
    }

    #[test]
    fn dataset_get_missing() {
        let (mut dom, parent, _, mut session) = setup();
        let result = DatasetGet
            .invoke(
                parent,
                &[JsValue::String("missing".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Undefined);
    }

    #[test]
    fn dataset_delete() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("data-foo-bar", "val");
        }
        DatasetDelete
            .invoke(
                parent,
                &[JsValue::String("fooBar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert!(!attrs.contains("data-foo-bar"));
    }

    #[test]
    fn dataset_keys() {
        let (mut dom, parent, _, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("data-x", "1");
            attrs.set("data-foo-bar", "2");
            attrs.set("class", "ignore");
        }
        let result = DatasetKeys
            .invoke(parent, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::String(s) = result {
            let keys: Vec<&str> = s.split('\0').collect();
            assert_eq!(keys.len(), 2);
            assert!(keys.contains(&"x"));
            assert!(keys.contains(&"fooBar"));
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn toggle_attribute_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        ToggleAttribute
            .invoke(
                parent,
                &[JsValue::String("hidden".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn set_class_name_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        SetClassName
            .invoke(
                parent,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn set_id_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        SetId
            .invoke(
                parent,
                &[JsValue::String("myid".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn dataset_set_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        let v1 = dom.inclusive_descendants_version(parent);
        DatasetSet
            .invoke(
                parent,
                &[JsValue::String("foo".into()), JsValue::String("bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn dataset_delete_rev_version() {
        let (mut dom, parent, _child, mut session) = setup();
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
            attrs.set("data-foo", "bar");
        }
        let v1 = dom.inclusive_descendants_version(parent);
        DatasetDelete
            .invoke(
                parent,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(parent);
        assert_ne!(v1, v2);
    }

    #[test]
    fn data_attr_to_camel_non_lowercase() {
        // Dash followed by non-lowercase should preserve dash + char.
        assert_eq!(data_attr_to_camel("data-foo-Bar"), "foo-Bar");
        assert_eq!(data_attr_to_camel("data-foo-1"), "foo-1");
        assert_eq!(data_attr_to_camel("data-foo-bar"), "fooBar");
        // Trailing dash should be preserved.
        assert_eq!(data_attr_to_camel("data-foo-"), "foo-");
    }
}
