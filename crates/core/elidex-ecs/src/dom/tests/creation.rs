use super::*;

#[test]
fn create_element() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let tags = dom.query_by_tag("div");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], div);
}

#[test]
fn create_text_node() {
    let mut dom = EcsDom::new();
    let text = dom.create_text("Hello, world!");
    let content = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(content.0, "Hello, world!");
}

#[test]
fn query_by_tag_multiple() {
    let mut dom = EcsDom::new();
    let _div1 = elem(&mut dom, "div");
    let _span = elem(&mut dom, "span");
    let _div2 = elem(&mut dom, "div");

    assert_eq!(dom.query_by_tag("div").len(), 2);
    assert_eq!(dom.query_by_tag("span").len(), 1);
    assert!(dom.query_by_tag("p").is_empty());
}

#[test]
fn text_node_as_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "p");
    let text = dom.create_text("Hello");

    dom.append_child(parent, text);

    assert_eq!(dom.children(parent), vec![text]);
    let content = dom.world().get::<&TextContent>(text).unwrap();
    assert_eq!(content.0, "Hello");
}

#[test]
fn contains_method() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert!(dom.contains(e));
    dom.destroy_entity(e);
    assert!(!dom.contains(e));
}

#[test]
fn attributes_accessors() {
    let mut attrs = Attributes::default();
    assert!(!attrs.contains("class"));
    assert_eq!(attrs.get("class"), None);

    attrs.set("class", "foo");
    assert!(attrs.contains("class"));
    assert_eq!(attrs.get("class"), Some("foo"));

    let old = attrs.set("class", "bar");
    assert_eq!(old, Some("foo".to_string()));
    assert_eq!(attrs.get("class"), Some("bar"));

    let removed = attrs.remove("class");
    assert_eq!(removed, Some("bar".to_string()));
    assert!(!attrs.contains("class"));
}

#[test]
fn ecs_dom_set_get_attribute_roundtrip() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    assert_eq!(dom.get_attribute(el, "id"), None);
    assert!(dom.set_attribute(el, "id", "main".to_owned()));
    assert_eq!(dom.get_attribute(el, "id"), Some("main".to_owned()));
    // Second set overwrites in place.
    assert!(dom.set_attribute(el, "id", "hero".to_owned()));
    assert_eq!(dom.get_attribute(el, "id"), Some("hero".to_owned()));
}

#[test]
fn ecs_dom_remove_attribute() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    assert!(dom.set_attribute(el, "class", "big".to_owned()));
    dom.remove_attribute(el, "class");
    assert_eq!(dom.get_attribute(el, "class"), None);
    // Removing a missing attribute is a silent no-op.
    dom.remove_attribute(el, "class");
    dom.remove_attribute(el, "not-set");
}

#[test]
fn ecs_dom_set_attribute_on_destroyed_entity_returns_false() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    dom.destroy_entity(el);
    assert!(!dom.set_attribute(el, "id", "lost".to_owned()));
}

#[test]
fn ecs_dom_with_attribute_present_missing_and_no_component() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let text = dom.create_text("hello");
    // Element with no attributes yet → None for any key.
    assert_eq!(dom.with_attribute(el, "id", |v| v.map(str::to_owned)), None);
    // After set, with_attribute borrows the same value get_attribute clones.
    assert!(dom.set_attribute(el, "id", "main".to_owned()));
    let viewed = dom.with_attribute(el, "id", |v| v.map(str::to_owned));
    assert_eq!(viewed.as_deref(), Some("main"));
    assert_eq!(viewed, dom.get_attribute(el, "id"));
    // Missing key on a component-bearing element → None.
    assert_eq!(
        dom.with_attribute(el, "absent", |v| v.map(str::to_owned)),
        None
    );
    // Entity without an Attributes component → None (text node).
    assert_eq!(
        dom.with_attribute(text, "id", |v| v.map(str::to_owned)),
        None
    );
}

#[test]
fn ecs_dom_has_attribute_matches_get_attribute_is_some() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let text = dom.create_text("hi");
    assert!(!dom.has_attribute(el, "id"));
    assert!(!dom.has_attribute(text, "id"));
    assert!(dom.set_attribute(el, "id", "main".to_owned()));
    assert!(dom.has_attribute(el, "id"));
    assert_eq!(
        dom.has_attribute(el, "id"),
        dom.get_attribute(el, "id").is_some()
    );
    assert!(!dom.has_attribute(el, "absent"));
}

#[test]
fn ecs_dom_with_tag_name_element_text_and_consistency() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let text = dom.create_text("hi");
    // Element returns its tag verbatim.
    assert_eq!(
        dom.with_tag_name(el, |t| t.map(str::to_owned)),
        Some("div".to_owned())
    );
    // Non-element entity → None.
    assert_eq!(dom.with_tag_name(text, |t| t.map(str::to_owned)), None);
    // Borrow form must agree with the owned getter for both arms.
    assert_eq!(
        dom.with_tag_name(el, |t| t.map(str::to_owned)),
        dom.get_tag_name(el)
    );
    assert_eq!(
        dom.with_tag_name(text, |t| t.map(str::to_owned)),
        dom.get_tag_name(text)
    );
}

#[test]
fn document_root_none_initially() {
    let dom = EcsDom::new();
    assert!(dom.document_root().is_none());
}

#[test]
fn document_root_set_by_create() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    assert_eq!(dom.document_root(), Some(root));
}
