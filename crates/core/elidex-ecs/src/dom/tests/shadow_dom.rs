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
    // Reserved names per HTML §4.13.3 `valid custom element name` — contain hyphen but are NOT valid custom elements.
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

// --- D-15 PR-A: attach_shadow_with_init / slot_assign / assigned_nodes ----

#[test]
fn attach_shadow_with_init_plumbs_all_fields() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom
        .attach_shadow_with_init(
            host,
            ShadowInit {
                mode: ShadowRootMode::Closed,
                delegates_focus: true,
                slot_assignment: SlotAssignmentMode::Manual,
                clonable: true,
                serializable: true,
                null_registry: true,
            },
        )
        .unwrap();
    let stored = dom.world().get::<&ShadowRoot>(sr).unwrap();
    assert_eq!(stored.mode, ShadowRootMode::Closed);
    assert!(stored.delegates_focus);
    assert!(stored.null_registry);
    assert_eq!(stored.slot_assignment, SlotAssignmentMode::Manual);
    assert!(stored.clonable);
    assert!(stored.serializable);
}

#[test]
fn attach_shadow_error_variants() {
    let mut dom = EcsDom::new();
    // InvalidEntity — DANGLING entity has no TagType
    assert_eq!(
        dom.attach_shadow(Entity::DANGLING, ShadowRootMode::Open),
        Err(ShadowAttachError::InvalidEntity)
    );
    // InvalidTag — <input> is not in the allowlist + not a custom element
    let input = elem(&mut dom, "input");
    assert_eq!(
        dom.attach_shadow(input, ShadowRootMode::Open),
        Err(ShadowAttachError::InvalidTag)
    );
    // AlreadyAttached — second attach on the same host
    let host = elem(&mut dom, "div");
    dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert_eq!(
        dom.attach_shadow(host, ShadowRootMode::Open),
        Err(ShadowAttachError::AlreadyAttached)
    );
}

#[test]
fn slot_assign_manual_mode_success() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom
        .attach_shadow_with_init(
            host,
            ShadowInit {
                mode: ShadowRootMode::Open,
                slot_assignment: SlotAssignmentMode::Manual,
                ..Default::default()
            },
        )
        .unwrap();
    let slot = elem(&mut dom, "slot");
    assert!(dom.append_child(sr, slot));

    // Two light-DOM children of the host.
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    assert!(dom.append_child(host, a));
    assert!(dom.append_child(host, b));

    dom.slot_assign(slot, vec![b, a]).unwrap();
    assert_eq!(dom.assigned_nodes(slot, false), vec![b, a]);
}

#[test]
fn slot_assign_named_mode_rejects() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    assert!(dom.append_child(sr, slot));
    let a = elem(&mut dom, "span");
    assert!(dom.append_child(host, a));
    assert_eq!(
        dom.slot_assign(slot, vec![a]),
        Err(SlotAssignError::NotManualMode)
    );
}

#[test]
fn slot_assign_validation_errors() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom
        .attach_shadow_with_init(
            host,
            ShadowInit {
                mode: ShadowRootMode::Open,
                slot_assignment: SlotAssignmentMode::Manual,
                ..Default::default()
            },
        )
        .unwrap();
    let slot = elem(&mut dom, "slot");
    assert!(dom.append_child(sr, slot));

    // NotASlot — pass an Element that isn't a <slot>
    let not_slot = elem(&mut dom, "div");
    assert!(dom.append_child(sr, not_slot));
    let span = elem(&mut dom, "span");
    assert!(dom.append_child(host, span));
    assert_eq!(
        dom.slot_assign(not_slot, vec![span]),
        Err(SlotAssignError::NotASlot)
    );

    // NotHostChild — the node is NOT a child of the host
    let stray = elem(&mut dom, "span");
    assert_eq!(
        dom.slot_assign(slot, vec![stray]),
        Err(SlotAssignError::NotHostChild)
    );

    // NoShadowRoot — the slot isn't inside a shadow tree
    let orphan_slot = elem(&mut dom, "slot");
    assert_eq!(
        dom.slot_assign(orphan_slot, vec![]),
        Err(SlotAssignError::NoShadowRoot)
    );
}

#[test]
fn assigned_nodes_empty_when_no_assignment_component() {
    let mut dom = EcsDom::new();
    let slot = elem(&mut dom, "slot");
    assert_eq!(dom.assigned_nodes(slot, false), Vec::<Entity>::new());
}

/// WHATWG HTML §2.1.4 removing steps step 2 — the focus reset on removal is
/// SHADOW-INCLUSIVE (via `is_connected`), so focus held inside a removed host's
/// shadow tree is cleared even though the light-tree event snapshot never
/// enters the shadow tree. (Regression guard: code-review found the original
/// light-tree-snapshot clear missed this.)
#[test]
fn removing_shadow_host_clears_focus_in_shadow_tree() {
    use crate::ElementState;
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let host = elem(&mut dom, "div");
    let _ = dom.append_child(doc, host);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let inner = elem(&mut dom, "span");
    let _ = dom.append_child(sr, inner);
    assert!(
        dom.is_connected(inner),
        "shadow descendant is connected via host"
    );
    let _ = dom
        .world_mut()
        .insert_one(inner, ElementState(ElementState::FOCUS));

    assert!(dom.remove_child(doc, host));
    let still_focused = dom
        .world()
        .get::<&ElementState>(inner)
        .is_ok_and(|s| s.contains(ElementState::FOCUS));
    assert!(
        !still_focused,
        "focus inside a removed shadow host's tree is reset"
    );
}

/// The light-tree `MutationEvent::Remove` is suppressed when `parent` is a
/// shadow root, but the focus reset must still run (it precedes the
/// suppression). Removing a focused light child of a shadow root clears focus.
#[test]
fn removing_light_child_of_shadow_root_clears_focus() {
    use crate::ElementState;
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let host = elem(&mut dom, "div");
    let _ = dom.append_child(doc, host);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let child = elem(&mut dom, "span");
    let _ = dom.append_child(sr, child);
    assert!(dom.is_connected(child));
    let _ = dom
        .world_mut()
        .insert_one(child, ElementState(ElementState::FOCUS));

    // parent = shadow root ⇒ Remove event suppressed, but focus still resets.
    assert!(dom.remove_child(sr, child));
    let still_focused = dom
        .world()
        .get::<&ElementState>(child)
        .is_ok_and(|s| s.contains(ElementState::FOCUS));
    assert!(
        !still_focused,
        "focus reset runs even when the Remove event is shadow-suppressed"
    );
}
