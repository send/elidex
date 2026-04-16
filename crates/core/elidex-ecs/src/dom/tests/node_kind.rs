use super::*;

#[test]
fn create_element_has_node_kind() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert_eq!(dom.node_kind(e), Some(NodeKind::Element));
}

#[test]
fn create_text_has_node_kind() {
    let mut dom = EcsDom::new();
    let t = dom.create_text("hello");
    assert_eq!(dom.node_kind(t), Some(NodeKind::Text));
}

#[test]
fn create_document_has_node_kind() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    assert_eq!(dom.node_kind(doc), Some(NodeKind::Document));
}

#[test]
fn create_shadow_root_has_node_kind() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert_eq!(dom.node_kind(sr), Some(NodeKind::DocumentFragment));
}

#[test]
fn create_document_fragment_has_node_kind() {
    let mut dom = EcsDom::new();
    let frag = dom.create_document_fragment();
    assert_eq!(dom.node_kind(frag), Some(NodeKind::DocumentFragment));
}

#[test]
fn create_comment_has_node_kind() {
    let mut dom = EcsDom::new();
    let c = dom.create_comment("test comment");
    assert_eq!(dom.node_kind(c), Some(NodeKind::Comment));
    let data = dom.world().get::<&CommentData>(c).unwrap();
    assert_eq!(data.0, "test comment");
}

#[test]
fn create_document_type_has_node_kind() {
    let mut dom = EcsDom::new();
    let dt = dom.create_document_type("html", "", "");
    assert_eq!(dom.node_kind(dt), Some(NodeKind::DocumentType));
    let data = dom.world().get::<&DocTypeData>(dt).unwrap();
    assert_eq!(data.name, "html");
}

#[test]
fn create_attribute_has_node_kind() {
    let mut dom = EcsDom::new();
    let a = dom.create_attribute("class");
    assert_eq!(dom.node_kind(a), Some(NodeKind::Attribute));
    let data = dom.world().get::<&AttrData>(a).unwrap();
    assert_eq!(data.local_name, "class");
    assert!(data.owner_element.is_none());
}

#[test]
fn node_type_round_trip() {
    for kind in [
        NodeKind::Element,
        NodeKind::Attribute,
        NodeKind::Text,
        NodeKind::CdataSection,
        NodeKind::ProcessingInstruction,
        NodeKind::Comment,
        NodeKind::Document,
        NodeKind::DocumentType,
        NodeKind::DocumentFragment,
    ] {
        let nt = kind.node_type();
        let back = NodeKind::from_node_type(nt).unwrap();
        assert_eq!(kind, back, "round-trip failed for {kind:?} (nodeType={nt})");
    }
}

#[test]
fn from_node_type_invalid() {
    assert!(NodeKind::from_node_type(0).is_none());
    assert!(NodeKind::from_node_type(5).is_none());
    assert!(NodeKind::from_node_type(6).is_none());
    assert!(NodeKind::from_node_type(12).is_none());
    assert!(NodeKind::from_node_type(99).is_none());
}

#[test]
fn create_window_root_has_node_kind() {
    let mut dom = EcsDom::new();
    let w = dom.create_window_root();
    assert_eq!(dom.node_kind(w), Some(NodeKind::Window));
    // Window is not a Node per WHATWG — it carries no `TreeRelation`.
    assert!(dom.world().get::<&TreeRelation>(w).is_err());
    // And does not have a `nodeType`.
    assert_eq!(NodeKind::Window.node_type(), 0);
    assert!(NodeKind::from_node_type(0).is_none());
}

#[test]
fn attributes_insertion_order() {
    let mut attrs = Attributes::default();
    attrs.set("c", "3");
    attrs.set("a", "1");
    attrs.set("b", "2");
    let keys = attrs.keys();
    assert_eq!(keys, vec!["c", "a", "b"]);
}

#[test]
fn attributes_keys_order() {
    let mut attrs = Attributes::default();
    attrs.set("z", "1");
    attrs.set("m", "2");
    attrs.set("a", "3");
    let pairs: Vec<_> = attrs.iter().collect();
    assert_eq!(pairs[0], ("z", "1"));
    assert_eq!(pairs[1], ("m", "2"));
    assert_eq!(pairs[2], ("a", "3"));
}

#[test]
fn attributes_len() {
    let mut attrs = Attributes::default();
    assert_eq!(attrs.len(), 0);
    attrs.set("a", "1");
    assert_eq!(attrs.len(), 1);
    attrs.set("b", "2");
    assert_eq!(attrs.len(), 2);
}

#[test]
fn attributes_is_empty() {
    let attrs = Attributes::default();
    assert!(attrs.is_empty());
    let mut attrs2 = Attributes::default();
    attrs2.set("a", "1");
    assert!(!attrs2.is_empty());
}

#[test]
fn attributes_overwrite_preserves_order() {
    let mut attrs = Attributes::default();
    attrs.set("a", "1");
    attrs.set("b", "2");
    attrs.set("c", "3");
    // Overwrite "b" — should keep its position
    attrs.set("b", "updated");
    let keys = attrs.keys();
    assert_eq!(keys, vec!["a", "b", "c"]);
    assert_eq!(attrs.get("b"), Some("updated"));
}
