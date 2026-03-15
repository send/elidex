use super::*;

#[test]
fn attach_shadow_success() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open);
    assert!(sr.is_ok());
    let sr = sr.unwrap();
    assert!(dom.contains(sr));
    assert_eq!(dom.get_shadow_root(host), Some(sr));
}

#[test]
fn attach_shadow_all_valid_tags() {
    for tag in VALID_SHADOW_HOST_TAGS {
        let mut dom = EcsDom::new();
        let host = elem(&mut dom, tag);
        let sr = dom.attach_shadow(host, ShadowRootMode::Open);
        assert!(sr.is_ok(), "attach_shadow should succeed for <{tag}>");
    }
}

#[test]
fn attach_shadow_invalid_tag() {
    let mut dom = EcsDom::new();
    let input = elem(&mut dom, "input");
    assert!(dom.attach_shadow(input, ShadowRootMode::Open).is_err());

    let img = elem(&mut dom, "img");
    assert!(dom.attach_shadow(img, ShadowRootMode::Open).is_err());

    let a = elem(&mut dom, "a");
    assert!(dom.attach_shadow(a, ShadowRootMode::Open).is_err());
}

#[test]
fn attach_shadow_custom_element() {
    let mut dom = EcsDom::new();
    let ce = elem(&mut dom, "my-component");
    assert!(
        dom.attach_shadow(ce, ShadowRootMode::Open).is_ok(),
        "custom elements should be valid shadow hosts"
    );

    let ce2 = elem(&mut dom, "x-widget");
    assert!(
        dom.attach_shadow(ce2, ShadowRootMode::Open).is_ok(),
        "custom elements with any hyphen should be valid"
    );
}

#[test]
fn attach_shadow_reserved_custom_element_names_rejected() {
    let mut dom = EcsDom::new();
    // Reserved names per HTML §4.13.2 — contain hyphen but are NOT valid custom elements.
    for name in [
        "annotation-xml",
        "color-profile",
        "font-face",
        "font-face-format",
        "font-face-name",
        "font-face-src",
        "font-face-uri",
        "missing-glyph",
    ] {
        let el = elem(&mut dom, name);
        assert!(
            dom.attach_shadow(el, ShadowRootMode::Open).is_err(),
            "reserved name '{name}' should be rejected as shadow host"
        );
    }
}

#[test]
fn attach_shadow_invalid_custom_element_format() {
    let mut dom = EcsDom::new();
    // Must start with lowercase ASCII letter.
    let upper = elem(&mut dom, "My-Component");
    assert!(
        dom.attach_shadow(upper, ShadowRootMode::Open).is_err(),
        "uppercase start should be rejected"
    );

    let digit = elem(&mut dom, "1-component");
    assert!(
        dom.attach_shadow(digit, ShadowRootMode::Open).is_err(),
        "digit start should be rejected"
    );
}

#[test]
fn attach_shadow_double_attach_fails() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    assert!(dom.attach_shadow(host, ShadowRootMode::Open).is_ok());
    assert!(dom.attach_shadow(host, ShadowRootMode::Open).is_err());
}

#[test]
fn shadow_root_not_in_children() {
    // M1: ShadowRoot entities are not exposed via children().
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let children = dom.children(host);
    assert!(
        !children.contains(&sr),
        "shadow root should not appear in children()"
    );
    assert!(
        children.contains(&light),
        "light DOM children should still appear"
    );
    // But we can still access via get_shadow_root.
    assert_eq!(dom.get_shadow_root(host), Some(sr));
}

#[test]
fn shadow_root_not_in_children_iter() {
    // M1: ShadowRoot entities are not exposed via children_iter().
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let children: Vec<Entity> = dom.children_iter(host).collect();
    assert!(!children.contains(&sr));
    assert!(children.contains(&light));
}

#[test]
fn get_shadow_root_none() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    assert_eq!(dom.get_shadow_root(host), None);
}

#[test]
fn composed_children_shadow_host() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "p");
    dom.append_child(sr, shadow_child);

    // composed_children of host should return shadow tree content.
    let composed = dom.composed_children(host);
    assert!(composed.contains(&shadow_child));
    // Light DOM children should NOT appear (they're distributed via slots).
    assert!(!composed.contains(&light));
}

#[test]
fn composed_children_slot_assigned() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Assign light child to slot.
    dom.world_mut()
        .insert_one(
            slot,
            SlotAssignment {
                assigned_nodes: vec![light],
            },
        )
        .unwrap();

    let composed = dom.composed_children(slot);
    assert_eq!(composed, vec![light]);
}

#[test]
fn composed_children_slot_fallback() {
    let mut dom = EcsDom::new();
    let slot = elem(&mut dom, "slot");
    let fallback = dom.create_text("fallback");
    dom.append_child(slot, fallback);

    // Empty SlotAssignment — should return slot's own children (fallback).
    dom.world_mut()
        .insert_one(
            slot,
            SlotAssignment {
                assigned_nodes: vec![],
            },
        )
        .unwrap();

    let composed = dom.composed_children(slot);
    assert_eq!(composed, vec![fallback]);
}

#[test]
fn composed_children_normal_element() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    dom.append_child(parent, child);

    // No shadow root, no slot — should return normal children.
    let composed = dom.composed_children(parent);
    assert_eq!(composed, vec![child]);
}

#[test]
fn shadow_root_closed_mode() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Closed).unwrap();
    assert_eq!(dom.get_shadow_root(host), Some(sr));
    // The shadow root entity exists, but JS access would check mode.
    let mode = dom.world().get::<&ShadowRoot>(sr).unwrap().mode;
    assert_eq!(mode, ShadowRootMode::Closed);
}

// --- find_tree_root tests ---

#[test]
fn find_tree_root_shadow_root_returns_self() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    // ShadowRoot itself should be returned as its own tree root.
    assert_eq!(dom.find_tree_root(sr), sr);
}

#[test]
fn find_tree_root_shadow_child_returns_shadow_root() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(doc, host);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let child = elem(&mut dom, "span");
    dom.append_child(sr, child);
    // Child in shadow tree should find shadow root as tree root.
    assert_eq!(dom.find_tree_root(child), sr);
}

#[test]
fn find_tree_root_normal_returns_document_root() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let div = elem(&mut dom, "div");
    dom.append_child(doc, div);
    let span = elem(&mut dom, "span");
    dom.append_child(div, span);
    // Normal DOM node should find document root.
    assert_eq!(dom.find_tree_root(span), doc);
}

// --- Stale entity tests ---

#[test]
fn get_shadow_root_returns_none_after_destroy() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert_eq!(dom.get_shadow_root(host), Some(sr));
    dom.destroy_entity(sr);
    // After destroying shadow root, get_shadow_root should return None.
    assert_eq!(dom.get_shadow_root(host), None);
}

// --- Custom element name validation ---

#[test]
fn custom_element_uppercase_rejected() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "MyElement");
    assert!(
        dom.attach_shadow(el, ShadowRootMode::Open).is_err(),
        "uppercase custom element names should be rejected"
    );
}

#[test]
fn custom_element_non_ascii_allowed() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "my-élément");
    assert!(
        dom.attach_shadow(el, ShadowRootMode::Open).is_ok(),
        "non-ASCII characters in custom element names should be allowed"
    );
}

#[test]
fn custom_element_invalid_char_rejected() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "my-element!");
    assert!(
        dom.attach_shadow(el, ShadowRootMode::Open).is_err(),
        "invalid PCENChar (!) should be rejected"
    );
}
