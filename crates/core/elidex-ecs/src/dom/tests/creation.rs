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
