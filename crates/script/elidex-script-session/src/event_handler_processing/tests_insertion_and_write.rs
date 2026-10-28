//! Unit cells U1–U6, U9, P1, P2 (umbrella §6.1; plan §4.3, §4.5): insertion
//! (T-a2) and the write-side entry (T-a3), driven through real
//! `append_child` / `set_attribute` calls with the consumer installed.
//!
//! Kinds follow the plan. Where an umbrella oracle mixes a HEAD-equality part
//! with a new-state part (plan §0.6 C-1: U3 (i), U5 (iii), P1 (ii)), the two
//! parts are separate tests: the HEAD part is a guard, the new-state part is
//! red-first.

use elidex_ecs::{EcsDom, Entity, ShadowRootMode};
use elidex_plugin::EngineMode;

use super::test_support::{
    append, compat, elem, entry, init_fresh, install, is_marked, page, raw, take_uncompiled, Entry,
};
use super::{
    is_window_delegating_pair, process_before_handler_write,
    WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET,
};
use crate::{EventListeners, HandlerScope, EVENT_HANDLER_ATTRS};

/// Live page with the consumer installed: `(dom, p1, p2)`, where `p1`,
/// `p2` are two connected `<div>` containers under `body`.
fn live_page(policy: elidex_plugin::SpecLevelPolicy) -> (EcsDom, Entity, Entity) {
    let mut dom = EcsDom::new();
    let (_doc, body) = page(&mut dom);
    dom.create_window_root();
    install(&mut dom, policy);
    let p1 = elem(&mut dom, "div", &[]);
    let p2 = elem(&mut dom, "div", &[]);
    append(&mut dom, body, p1);
    append(&mut dom, body, p2);
    (dom, p1, p2)
}

// ---- U1 (red, K-1): a marked element is skipped at a later insertion ----
#[test]
fn u1_second_insertion_does_not_reraw() {
    let (mut dom, p1, p2) = live_page(compat());
    let e = elem(&mut dom, "div", &[("onclick", "A")]);
    append(&mut dom, p1, e);
    assert_eq!(
        entry(&dom, e, "click"),
        raw("A"),
        "control: delivered at the first insertion"
    );
    take_uncompiled(&mut dom, e, "click");
    append(&mut dom, p2, e);
    assert_eq!(
        entry(&dom, e, "click"),
        Entry::NoSource,
        "U1: the move must not re-raw A (HEAD: raw A)"
    );
}

// ---- U2 (window pin, X-W): a Window-delegating pair is re-raw'd at every insertion ----
#[test]
fn u2_window_delegating_pair_reraw_at_move() {
    let (mut dom, p1, p2) = live_page(compat());
    let window = dom.window_entity().expect("window");
    let b = elem(&mut dom, "body", &[("onhashchange", "X")]);
    append(&mut dom, p1, b);
    assert_eq!(entry(&dom, window, "hashchange"), raw("X"));
    take_uncompiled(&mut dom, window, "hashchange");
    append(&mut dom, p2, b);
    // HEAD's re-raw. Spec: None; slice b (S-5) inverts this pin.
    assert_eq!(entry(&dom, window, "hashchange"), raw("X"));
}

fn core_policy() -> elidex_plugin::SpecLevelPolicy {
    EngineMode::BrowserCore.spec_level_policy()
}

// ---- U3 (i) guard part: a policy-excluded ordinary pair is never wired ----
#[test]
fn u3_i_excluded_ordinary_pair_not_wired() {
    let (mut dom, p1, p2) = live_page(core_policy());
    let d = elem(&mut dom, "div", &[("onstorage", "globalThis.hit = 1")]);
    append(&mut dom, p1, d);
    assert_eq!(entry(&dom, d, "storage"), Entry::Absent);
    append(&mut dom, p2, d);
    assert_eq!(entry(&dom, d, "storage"), Entry::Absent);
}

// ---- U3 (i) new-state part (C-1, red-first): an excluded-only element is still marked ----
#[test]
fn u3_i_excluded_ordinary_pair_marks_element() {
    let (mut dom, p1, _p2) = live_page(core_policy());
    let d = elem(&mut dom, "div", &[("onstorage", "globalThis.hit = 1")]);
    append(&mut dom, p1, d);
    assert!(
        is_marked(&dom, d),
        "U3 (i): the div is marked after the first append (I1×I6)"
    );
}

// ---- U3 (ii) guard: an excluded Window-delegating pair is never wired on the Window ----
#[test]
fn u3_ii_excluded_window_delegating_pair_not_wired() {
    let (mut dom, p1, p2) = live_page(core_policy());
    let window = dom.window_entity().expect("window");
    let b = elem(&mut dom, "body", &[("onstorage", "globalThis.hit = 1")]);
    append(&mut dom, p1, b);
    assert_eq!(entry(&dom, window, "storage"), Entry::Absent);
    append(&mut dom, p2, b);
    assert_eq!(entry(&dom, window, "storage"), Entry::Absent);
}

// ---- U4 (guard): an element with no handler attribute is never marked ----
#[test]
fn u4_no_handler_attribute_unmarked() {
    let (mut dom, p1, _p2) = live_page(compat());
    let d = elem(&mut dom, "div", &[("id", "x")]);
    append(&mut dom, p1, d);
    assert!(!is_marked(&dom, d));
}

// ---- U5 (i) (red, K-2): the first write to the only ordinary attribute processes E ----
#[test]
fn u5_i_first_write_processes_single_attribute_element() {
    let (mut dom, p1, _p2) = live_page(compat());
    let e = elem(&mut dom, "div", &[("onclick", "A")]);
    let _ = dom.set_attribute(e, "onclick", "B");
    assert!(
        is_marked(&dom, e),
        "U5 (i): marked by the write (T-a3 passes)"
    );
    assert_eq!(
        entry(&dom, e, "click"),
        raw("B"),
        "HEAD's state after the write"
    );
    take_uncompiled(&mut dom, e, "click");
    append(&mut dom, p1, e);
    assert_eq!(
        entry(&dom, e, "click"),
        Entry::NoSource,
        "the insertion skips the marked element"
    );
}

// ---- U5 (ii) (guard): two ordinary attributes, the restriction returns ----
#[test]
fn u5_ii_two_ordinary_attributes_take_heads_path() {
    let (mut dom, p1, _p2) = live_page(compat());
    let e = elem(&mut dom, "div", &[("onclick", "A"), ("onmouseover", "C")]);
    let _ = dom.set_attribute(e, "onclick", "B");
    append(&mut dom, p1, e);
    assert_eq!(entry(&dom, e, "click"), raw("B"));
    assert_eq!(entry(&dom, e, "mouseover"), raw("C"));
}

// ---- U5 (iii) guard part: the write-side entry after an insertion changes nothing ----
#[test]
fn u5_iii_entry_on_processed_element_changes_nothing() {
    let (mut dom, p1, _p2) = live_page(compat());
    let e = elem(&mut dom, "div", &[("onclick", "A")]);
    append(&mut dom, p1, e);
    take_uncompiled(&mut dom, e, "click");
    process_before_handler_write(&mut dom, e, "onclick", compat());
    assert_eq!(entry(&dom, e, "click"), Entry::NoSource);
}

// ---- U5 (iii) new-state part (C-1, red-first): the element the entry sees is marked ----
#[test]
fn u5_iii_entry_on_marked_element_keeps_marker() {
    let (mut dom, p1, _p2) = live_page(compat());
    let e = elem(&mut dom, "div", &[("onclick", "A")]);
    append(&mut dom, p1, e);
    assert!(
        is_marked(&dom, e),
        "U5 (iii): marked at its first insertion"
    );
    process_before_handler_write(&mut dom, e, "onclick", compat());
    assert!(is_marked(&dom, e));
}

// ---- U6 (guard, I1×I4): a clone copies no marker ----
#[test]
fn u6_clone_is_unmarked() {
    let (mut dom, p1, _p2) = live_page(compat());
    let e = elem(&mut dom, "div", &[("onclick", "A")]);
    append(&mut dom, p1, e);
    let c = dom.clone_node_shallow(e).expect("clone");
    assert!(!is_marked(&dom, c), "U6: clone unmarked");
}

// ---- U9 (guard, K-0): a shadow-root-direct element is reached by no later bind or Insert ----
#[test]
fn u9_shadow_direct_element_untouched_by_later_bind_and_host_move() {
    let mut dom = EcsDom::new();
    let (doc, body) = page(&mut dom);
    dom.create_window_root();
    init_fresh(&mut dom, doc, compat());
    install(&mut dom, compat());
    let h = elem(&mut dom, "div", &[]);
    append(&mut dom, body, h);
    let sr = dom
        .attach_shadow(h, ShadowRootMode::Open)
        .expect("attach_shadow");
    let x = elem(&mut dom, "div", &[("onclick", "A"), ("onmouseover", "C")]);
    // No Insert under a shadow root (H2).
    append(&mut dom, sr, x);
    // The write-side entry for `click` (two deliverable ordinary attributes: returns).
    process_before_handler_write(&mut dom, x, "onclick", compat());
    // An IDL write's store state: a `click` entry with no source.
    if dom.world().get::<&EventListeners>(x).is_err() {
        let _ = dom.world_mut().insert_one(x, EventListeners::new());
    }
    {
        let mut l = dom
            .world_mut()
            .get::<&mut EventListeners>(x)
            .expect("listeners");
        if l.find_event_handler("click").is_none() {
            l.add_event_handler("click".to_string());
        }
    }
    let p2 = elem(&mut dom, "div", &[]);
    append(&mut dom, body, p2);
    append(&mut dom, p2, h); // host move
    init_fresh(&mut dom, doc, compat()); // second bind: fresh consumer
    assert!(!is_marked(&dom, x), "U9: x unmarked");
    assert_eq!(
        entry(&dom, x, "click"),
        Entry::NoSource,
        "U9: the click entry still has no source"
    );
}

// ---- P1 (i) (white-box guard, A2-MIN-1): Window-delegating-only element is not marked ----
#[test]
fn p1_i_window_delegating_only_element_unmarked() {
    let (mut dom, p1, _p2) = live_page(compat());
    let window = dom.window_entity().expect("window");
    let b = elem(&mut dom, "body", &[("onhashchange", "X")]);
    append(&mut dom, p1, b);
    assert!(!is_marked(&dom, b), "P1 (i): not marked");
    assert_eq!(entry(&dom, window, "hashchange"), raw("X"));
}

// ---- P1 (ii) (new state, C-1, red-first): a mixed element is marked ----
#[test]
fn p1_ii_mixed_element_marked() {
    let (mut dom, p1, _p2) = live_page(compat());
    let b = elem(&mut dom, "body", &[("onclick", "A"), ("onhashchange", "X")]);
    append(&mut dom, p1, b);
    assert!(is_marked(&dom, b), "P1 (ii): marked");
}

/// WindowEventHandlers members, `.claude/tools/webref idl html WindowEventHandlers`.
const WEH_LITERAL: [&str; 18] = [
    "onafterprint",
    "onbeforeprint",
    "onbeforeunload",
    "onhashchange",
    "onlanguagechange",
    "onmessage",
    "onmessageerror",
    "onoffline",
    "ononline",
    "onpagehide",
    "onpagereveal",
    "onpageshow",
    "onpageswap",
    "onpopstate",
    "onrejectionhandled",
    "onstorage",
    "onunhandledrejection",
    "onunload",
];

/// Window-reflecting body element event handler set (WHATWG HTML §8.1.8.2).
const REFLECTING_LITERAL: [&str; 6] = [
    "onblur", "onerror", "onfocus", "onload", "onresize", "onscroll",
];

// ---- P2 (white-box guard, A2-MIN-3): the X-W classifier ----
#[test]
fn p2_classifier_derivation_and_fixed_ratchet() {
    let mut dom = EcsDom::new();
    let body = elem(&mut dom, "body", &[]);
    let frameset = elem(&mut dom, "frameset", &[]);
    let div = elem(&mut dom, "div", &[]);

    // Derived: every HandlerScope::Window row delegates on body and frameset.
    for (name, scope) in EVENT_HANDLER_ATTRS {
        if *scope == HandlerScope::Window {
            assert!(
                is_window_delegating_pair(&dom, body, name),
                "{name} on body (HEAD's predicate `scope == Window && is_body_element` ⊆ X-W)"
            );
            assert!(
                is_window_delegating_pair(&dom, frameset, name),
                "{name} on frameset"
            );
        }
        assert!(!is_window_delegating_pair(&dom, div, name), "{name} on div");
    }
    // Every reflecting name delegates on both, and is a table row.
    for name in WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET {
        assert!(
            is_window_delegating_pair(&dom, body, name),
            "{name} on body"
        );
        assert!(
            is_window_delegating_pair(&dom, frameset, name),
            "{name} on frameset"
        );
        assert!(
            EVENT_HANDLER_ATTRS.iter().any(|(a, _)| *a == name),
            "{name} is a table row"
        );
    }
    assert!(!is_window_delegating_pair(&dom, body, "onclick"));

    // Fixed ratchet, independent of the derivation.
    let mut window_rows: Vec<&str> = EVENT_HANDLER_ATTRS
        .iter()
        .filter(|(_, s)| *s == HandlerScope::Window)
        .map(|(a, _)| *a)
        .collect();
    window_rows.sort_unstable();
    let mut weh = WEH_LITERAL.to_vec();
    weh.sort_unstable();
    assert_eq!(
        window_rows, weh,
        "HandlerScope::Window rows == WindowEventHandlers (webref)"
    );
    assert_eq!(
        WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET, REFLECTING_LITERAL,
        "the const equals the spec's six (A-11)"
    );
    for name in WEH_LITERAL.iter().chain(REFLECTING_LITERAL.iter()) {
        assert!(
            is_window_delegating_pair(&dom, body, name),
            "ratchet: {name} on body"
        );
        assert!(
            is_window_delegating_pair(&dom, frameset, name),
            "ratchet: {name} on frameset"
        );
    }
}
