//! Tests for `event_handlers`' focus-collection helpers, split out of the (large)
//! input event handler module per the Axis-5 1000-line growth guard (Codex R4).
//! Included via `#[cfg(test)] #[path = "event_handlers_tests.rs"] mod tests;`, so
//! `super` is the `event_handlers` module.

use super::collect_focusable_entities;
use elidex_ecs::{Attributes, EcsDom, ShadowInit, ShadowRootMode};

#[test]
fn collect_focusable_entities_excludes_unresolvable_delegates_focus_host() {
    // Codex R3: a `delegatesFocus` host with `tabindex` but no focusable delegate
    // is C2-blind `is_focusable` yet resolves to NO focusable area, so it must not
    // enter the Tab order — otherwise Tab selects it as `next` and `set_focus`
    // blurs on the `None` resolution, dropping focus (and getting stuck on the same
    // candidate next Tab). The collector admits only entities that are their own
    // §6.6.3 tab stop (`focus_target(entity) == Some(entity)`).
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    // A normal focusable element — must be collected (resolves to itself).
    let mut plain_attrs = Attributes::default();
    plain_attrs.set("tabindex".to_string(), "0".to_string());
    let plain = dom.create_element("div", plain_attrs);
    let _ = dom.append_child(doc, plain);

    // A `delegatesFocus` host with `tabindex` but an empty shadow tree — its
    // `focus_target` is `None`, so it is excluded.
    let mut host_attrs = Attributes::default();
    host_attrs.set("tabindex".to_string(), "0".to_string());
    let host = dom.create_element("div", host_attrs);
    let _ = dom.append_child(doc, host);
    let _sr = dom
        .attach_shadow_with_init(
            host,
            ShadowInit {
                mode: ShadowRootMode::Open,
                delegates_focus: true,
                ..Default::default()
            },
        )
        .expect("attach_shadow on <div tabindex>");

    let mut result = Vec::new();
    collect_focusable_entities(&dom, doc, &mut result, 0);
    let collected: Vec<_> = result.iter().map(|(e, _)| *e).collect();
    assert!(
        collected.contains(&plain),
        "a normal focusable element is collected into the Tab order"
    );
    assert!(
        !collected.contains(&host),
        "a delegatesFocus host with no delegate is excluded from the Tab order"
    );
}

#[test]
fn collect_focusable_entities_excludes_delegates_focus_host_with_delegate() {
    // Codex R4: a `delegatesFocus` host with a focusable delegate resolves to the
    // DELEGATE, not the host (`focus_target(host) == Some(delegate) != Some(host)`),
    // so the host must NOT enter the Tab order. If it did, Tab would `set_focus`
    // the host → retarget to the delegate, but the delegate is not in the cache, so
    // `find_next_focusable`'s exact-entity lookup of `current_focus` would miss and
    // restart at `focusables[0]`. (Making the shadow delegate itself Tab-reachable,
    // with C2-aware ordering, is PR-A3.)
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    let host = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, host);
    let sr = dom
        .attach_shadow_with_init(
            host,
            ShadowInit {
                mode: ShadowRootMode::Open,
                delegates_focus: true,
                ..Default::default()
            },
        )
        .expect("attach_shadow on <div>");
    let mut delegate_attrs = Attributes::default();
    delegate_attrs.set("tabindex".to_string(), "0".to_string());
    let delegate = dom.create_element("div", delegate_attrs);
    let _ = dom.append_child(sr, delegate);

    let mut result = Vec::new();
    collect_focusable_entities(&dom, doc, &mut result, 0);
    let collected: Vec<_> = result.iter().map(|(e, _)| *e).collect();
    assert!(
        !collected.contains(&host),
        "a delegatesFocus host that resolves to its delegate is excluded from the \
         Tab cache (so find_next_focusable does not miss on the focused delegate)"
    );
}
