//! EH-attrs-at-creation T3-a: unit cells U-T8, U-A1 to U-A3 (plan
//! `docs/plans/2026-11-eh-attrs-t3a-applicability.md` §6.2) — a name is
//! processed only if it is an event handler content attribute of its element
//! (WHATWG HTML §8.1.8.1 Event handlers, attribute change steps step 1,
//! local-name half; §8.1.8.2; SVG2 §15.8; MathML Core §2.1.4).
//!
//! U-T8's oracle reads HEAD's target rule (`resolve_handler_target`) and HEAD's
//! insertion delivery, both of which T3 replaces (plan §8.4).

use elidex_ecs::{EcsDom, Entity, MutationEvent, Namespace};
use elidex_plugin::{EngineMode, SpecLevelPolicy};

use super::test_support::{
    append, compat, elem, entry, foreign_elem, install, is_marked, page, raw, Entry,
};
use super::{
    is_event_handler_content_attribute_of, process_before_handler_write, resolve_handler_target,
    set_inline_handler,
};
use crate::{EventHandlerAttributeConsumer, EventListeners, HandlerScope, EVENT_HANDLER_ATTRS};

#[derive(Clone, Copy, Debug)]
enum Kind {
    HtmlBody,
    HtmlFrameset,
    HtmlDiv,
    SvgBody,
    SvgRect,
    MathBody,
    MathFrameset,
    MathMi,
}

const KINDS: [Kind; 8] = [
    Kind::HtmlBody,
    Kind::HtmlFrameset,
    Kind::HtmlDiv,
    Kind::SvgBody,
    Kind::SvgRect,
    Kind::MathBody,
    Kind::MathFrameset,
    Kind::MathMi,
];

/// Whether the `event_type` handler entry on `target` is deactivated (`cleared`).
fn is_handler_cleared(dom: &EcsDom, target: Entity, event_type: &str) -> bool {
    let listeners = dom
        .world()
        .get::<&EventListeners>(target)
        .expect("target has EventListeners");
    let id = listeners
        .find_event_handler(event_type)
        .expect("handler entry exists");
    listeners.is_handler_cleared(id)
}

/// D-T8's table, written from the spec rows (plan §0.4), not from the predicate.
fn expected(kind: Kind, scope: HandlerScope) -> bool {
    let global = matches!(scope, HandlerScope::Global | HandlerScope::DocumentElement);
    match kind {
        Kind::SvgBody | Kind::SvgRect => true,
        // MathML kinds (not HTML body/frameset, not SVG) and HTML div (not body/frameset) share one ground: global scope only.
        Kind::MathBody | Kind::MathFrameset | Kind::MathMi | Kind::HtmlDiv => global,
        Kind::HtmlBody | Kind::HtmlFrameset => global || scope == HandlerScope::Window,
    }
}

fn make(dom: &mut EcsDom, kind: Kind, attrs: &[(&str, &str)]) -> Entity {
    match kind {
        Kind::HtmlBody => elem(dom, "body", attrs),
        Kind::HtmlFrameset => elem(dom, "frameset", attrs),
        Kind::HtmlDiv => elem(dom, "div", attrs),
        Kind::SvgBody => foreign_elem(dom, "body", Namespace::Svg, attrs),
        Kind::SvgRect => foreign_elem(dom, "rect", Namespace::Svg, attrs),
        Kind::MathBody => foreign_elem(dom, "body", Namespace::MathMl, attrs),
        Kind::MathFrameset => foreign_elem(dom, "frameset", Namespace::MathMl, attrs),
        Kind::MathMi => foreign_elem(dom, "mi", Namespace::MathMl, attrs),
    }
}

/// A fresh DOM with a page and a Window, and no dispatcher.
fn bare_dom() -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let (_doc, body) = page(&mut dom);
    dom.create_window_root();
    (dom, body)
}

fn attribute_change(
    dom: &mut EcsDom,
    node: Entity,
    name: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    policy: SpecLevelPolicy,
) {
    EventHandlerAttributeConsumer::with_policy(policy).handle(
        &MutationEvent::AttributeChange {
            node,
            name,
            old_value,
            new_value,
        },
        dom,
    );
}

// ---- U-T8 (red; full enumeration): D-T8's table over every table row x element class ----
#[test]
fn u_t8_applicability_table_is_complete_and_applied_by_every_reader() {
    for &(name, scope) in EVENT_HANDLER_ATTRS {
        let event_type = &name[2..];
        for kind in KINDS {
            let want = expected(kind, scope);
            let ctx = format!("{kind:?} x {name} ({scope:?})");
            // Legs (i) and (ii) share one oracle: the handler is wired iff applicable.
            let set_expect = if want { raw("X") } else { Entry::Absent };

            // The predicate's value.
            {
                let (mut dom, _) = bare_dom();
                let el = make(&mut dom, kind, &[]);
                assert_eq!(
                    is_event_handler_content_attribute_of(&dom, el, name),
                    want,
                    "predicate: {ctx}"
                );
            }

            // (i) an AttributeChange set through the consumer; fresh DOM.
            {
                let (mut dom, _) = bare_dom();
                let el = make(&mut dom, kind, &[]);
                let target = resolve_handler_target(el, scope, &dom);
                attribute_change(&mut dom, el, name, None, Some("X"), compat());
                assert_eq!(
                    entry(&dom, target, event_type),
                    set_expect,
                    "(i) set: {ctx}"
                );
            }

            // (ii) a first insertion of an element carrying the attribute; fresh DOM.
            {
                let (mut dom, body) = bare_dom();
                install(&mut dom, compat());
                let el = make(&mut dom, kind, &[(name, "X")]);
                let target = resolve_handler_target(el, scope, &dom);
                append(&mut dom, body, el);
                assert_eq!(
                    entry(&dom, target, event_type),
                    set_expect,
                    "(ii) insertion: {ctx}"
                );
            }

            // (iii) a removal: a handler placed by other means is cleared iff applicable.
            {
                let (mut dom, _) = bare_dom();
                let el = make(&mut dom, kind, &[]);
                let target = resolve_handler_target(el, scope, &dom);
                set_inline_handler(target, event_type, "X", &mut dom);
                assert_eq!(
                    entry(&dom, target, event_type),
                    raw("X"),
                    "(iii) control: {ctx}"
                );
                attribute_change(&mut dom, el, name, Some("X"), None, compat());
                let expect = if want { Entry::NoSource } else { raw("X") };
                assert_eq!(
                    entry(&dom, target, event_type),
                    expect,
                    "(iii) removal: {ctx}"
                );
                // `NoSource` alone cannot tell a deactivated handler (`mark_cleared`)
                // from one whose source was merely taken; read the flag itself.
                assert_eq!(
                    is_handler_cleared(&dom, target, event_type),
                    want,
                    "(iii) removal deactivates the handler iff applicable: {ctx}"
                );
            }
        }
    }
}

// ---- U-A1 (red; AP-8, D-T8 (b)): a non-applicable name does not count toward the D-8 restriction ----
#[test]
fn u_a1_non_applicable_name_does_not_defeat_the_write_entry() {
    let (mut dom, _) = bare_dom();
    let d = elem(&mut dom, "div", &[("onclick", "A"), ("onhashchange", "B")]);
    process_before_handler_write(&mut dom, d, "onclick", compat());
    assert!(is_marked(&dom, d), "processed at the write: marked");
    assert_eq!(entry(&dom, d, "click"), raw("A"));
    assert_eq!(entry(&dom, d, "hashchange"), Entry::Absent);
}

// ---- U-A2 (red; AP-7, B3xB5): an element whose only handler-named attribute is not its own is not marked ----
#[test]
fn u_a2_element_with_only_non_applicable_names_is_not_marked() {
    let (mut dom, body) = bare_dom();
    install(&mut dom, compat());
    let d = elem(&mut dom, "div", &[("onhashchange", "B")]);
    append(&mut dom, body, d);
    assert!(!is_marked(&dom, d), "AP-7: not marked");
    assert_eq!(entry(&dom, d, "hashchange"), Entry::Absent);
}

// ---- U-A3 (guard; B3xB6): applicability precedes the policy filter and the mark's count ----
#[test]
fn u_a3_div_onstorage_is_neither_wired_nor_marked_under_either_policy() {
    for mode in [EngineMode::BrowserCore, EngineMode::BrowserCompat] {
        let policy = mode.spec_level_policy();
        // First insertion.
        let (mut dom, body) = bare_dom();
        install(&mut dom, policy);
        let d = elem(&mut dom, "div", &[("onstorage", "X")]);
        append(&mut dom, body, d);
        assert!(!is_marked(&dom, d), "{mode:?}: insertion: not marked");
        assert_eq!(entry(&dom, d, "storage"), Entry::Absent, "{mode:?}");
        // An AttributeChange.
        let (mut dom, _) = bare_dom();
        let d = elem(&mut dom, "div", &[]);
        attribute_change(&mut dom, d, "onstorage", None, Some("X"), policy);
        assert!(!is_marked(&dom, d), "{mode:?}: set: not marked");
        assert_eq!(entry(&dom, d, "storage"), Entry::Absent, "{mode:?}");
    }
}
