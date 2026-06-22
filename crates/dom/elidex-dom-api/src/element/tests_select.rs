//! Handler-direct tests for `OptionsAdd` / `OptionsRemove` / `OptionsSetLength`
//! (HTML §2.6.4.3 — the `HTMLOptionsCollection` / `HTMLSelectElement` option
//! tree-mutation surface). B1.2b-2-select.
//!
//! These exercise the engine-independent handlers directly (the boa/wasm-style
//! path), where the handler's own tag-guard / validity-ordering is the sole
//! protection. The VM end-to-end coverage (real JS + delivered `MutationRecord`s)
//! lives in `elidex-js` `vm::tests::tests_mutation_observer::select_options`.

use super::*;
use elidex_ecs::{Attributes, EcsDom, Entity, TagType};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiErrorKind, DomApiHandler, SessionCore};

/// A `<select>` (the collection root = handler `this`) and a session that can
/// mint `ObjectRef`s. The select is not attached to any document tree.
fn setup() -> (EcsDom, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let select = dom.create_element("select", Attributes::default());
    let mut session = SessionCore::new();
    session.get_or_create_wrapper(select, ComponentKind::Element);
    (dom, select, session)
}

/// Create an element of `tag`, register a wrapper, return `(entity, ref)`.
fn elem(dom: &mut EcsDom, session: &mut SessionCore, tag: &str) -> (Entity, u64) {
    let e = dom.create_element(tag, Attributes::default());
    let r = session
        .get_or_create_wrapper(e, ComponentKind::Element)
        .to_raw();
    (e, r)
}

// ---------------------------------------------------------------------------
// add
// ---------------------------------------------------------------------------

#[test]
fn options_add_appends_option_when_before_is_null() {
    let (mut dom, select, mut session) = setup();
    let (opt, opt_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(opt_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![opt]);
}

#[test]
fn options_add_before_element_inserts_before_reference() {
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(a_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(b_ref), JsValue::ObjectRef(a_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![b, a]);
}

#[test]
fn options_add_before_integer_inserts_before_indexth() {
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    for r in [a_ref, b_ref] {
        OptionsAdd
            .invoke(
                select,
                &[JsValue::ObjectRef(r), JsValue::Null],
                &mut session,
                &mut dom,
            )
            .unwrap();
    }
    let (c, c_ref) = elem(&mut dom, &mut session, "option");
    // before index 1 (== b) → [a, c, b]
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(c_ref), JsValue::Number(1.0)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![a, c, b]);
}

#[test]
fn options_add_out_of_range_integer_appends() {
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(a_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(b_ref), JsValue::Number(99.0)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![a, b]);
}

#[test]
fn options_add_accepts_optgroup() {
    let (mut dom, select, mut session) = setup();
    let (og, og_ref) = elem(&mut dom, &mut session, "optgroup");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(og_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![og]);
}

#[test]
fn options_add_non_option_is_type_error() {
    // The engine-independent tag-guard is the sole protection on this path: a
    // `<div>` (neither option nor optgroup) → TypeError, NOT a relink.
    let (mut dom, select, mut session) = setup();
    let (_div, div_ref) = elem(&mut dom, &mut session, "div");
    let err = OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(div_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
    assert!(dom.children(select).is_empty());
}

#[test]
fn options_add_ancestor_element_is_hierarchy_request_error() {
    // step 1: element is an ancestor of select → HierarchyRequestError. An
    // optgroup containing the select passes the tag-guard yet is an ancestor.
    let (mut dom, select, mut session) = setup();
    let (og, og_ref) = elem(&mut dom, &mut session, "optgroup");
    dom.append_child(og, select);
    let err = OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(og_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn options_add_before_not_descendant_is_not_found_error() {
    // step 2: before is an element not a descendant of select → NotFoundError.
    let (mut dom, select, mut session) = setup();
    let (_opt, opt_ref) = elem(&mut dom, &mut session, "option");
    let (_stray, stray_ref) = elem(&mut dom, &mut session, "option");
    let err = OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(opt_ref), JsValue::ObjectRef(stray_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn options_add_element_equals_before_is_noop() {
    // step 3: element == before → return (no mutation).
    let (mut dom, select, mut session) = setup();
    let (opt, opt_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(opt_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(opt_ref), JsValue::ObjectRef(opt_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![opt]);
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn options_remove_index_detaches_option() {
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    for r in [a_ref, b_ref] {
        OptionsAdd
            .invoke(
                select,
                &[JsValue::ObjectRef(r), JsValue::Null],
                &mut session,
                &mut dom,
            )
            .unwrap();
    }
    OptionsRemove
        .invoke(select, &[JsValue::Number(0.0)], &mut session, &mut dom)
        .unwrap();
    let _ = a;
    assert_eq!(dom.children(select), vec![b]);
}

#[test]
fn options_remove_out_of_range_is_noop() {
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(a_ref), JsValue::Null],
            &mut session,
            &mut dom,
        )
        .unwrap();
    OptionsRemove
        .invoke(select, &[JsValue::Number(5.0)], &mut session, &mut dom)
        .unwrap();
    OptionsRemove
        .invoke(select, &[JsValue::Number(-1.0)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(dom.children(select), vec![a]);
}

// ---------------------------------------------------------------------------
// WebIDL `long` (ToInt32) coercion on the handler-direct path (Codex R2)
// ---------------------------------------------------------------------------
//
// A boa/wasm caller reaches the handler with a RAW number (no VM pre-coercion),
// so the handler itself must apply `ToInt32` to the `long` index: NaN/±∞ → 0,
// 2³² wraps → 0. (The VM path pre-coerces, so these exercise the direct path.)

#[test]
fn options_add_before_index_wraps_via_to_int32() {
    // ToInt32(2³²) == 0 → insert before options[0], NOT append.
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    for r in [a_ref, b_ref] {
        OptionsAdd
            .invoke(
                select,
                &[JsValue::ObjectRef(r), JsValue::Null],
                &mut session,
                &mut dom,
            )
            .unwrap();
    }
    let (c, c_ref) = elem(&mut dom, &mut session, "option");
    OptionsAdd
        .invoke(
            select,
            &[JsValue::ObjectRef(c_ref), JsValue::Number(4_294_967_296.0)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(dom.children(select), vec![c, a, b]);
}

#[test]
fn options_remove_nan_index_targets_zero_via_to_int32() {
    // ToInt32(NaN) == 0 → remove options[0], NOT a no-op.
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    for r in [a_ref, b_ref] {
        OptionsAdd
            .invoke(
                select,
                &[JsValue::ObjectRef(r), JsValue::Null],
                &mut session,
                &mut dom,
            )
            .unwrap();
    }
    OptionsRemove
        .invoke(select, &[JsValue::Number(f64::NAN)], &mut session, &mut dom)
        .unwrap();
    let _ = a;
    assert_eq!(dom.children(select), vec![b]);
}

#[test]
fn options_remove_wrapping_index_targets_zero_via_to_int32() {
    // ToInt32(2³²) == 0 → remove options[0].
    let (mut dom, select, mut session) = setup();
    let (a, a_ref) = elem(&mut dom, &mut session, "option");
    let (b, b_ref) = elem(&mut dom, &mut session, "option");
    for r in [a_ref, b_ref] {
        OptionsAdd
            .invoke(
                select,
                &[JsValue::ObjectRef(r), JsValue::Null],
                &mut session,
                &mut dom,
            )
            .unwrap();
    }
    OptionsRemove
        .invoke(
            select,
            &[JsValue::Number(4_294_967_296.0)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let _ = a;
    assert_eq!(dom.children(select), vec![b]);
}

// ---------------------------------------------------------------------------
// length setter
// ---------------------------------------------------------------------------

#[test]
fn options_set_length_grow_appends_bare_options() {
    let (mut dom, select, mut session) = setup();
    OptionsSetLength
        .invoke(select, &[JsValue::Number(3.0)], &mut session, &mut dom)
        .unwrap();
    let children = dom.children(select);
    assert_eq!(children.len(), 3);
    assert!(children.iter().all(|&c| dom
        .world()
        .get::<&TagType>(c)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("option"))));
}

#[test]
fn options_set_length_truncate_removes_from_end() {
    let (mut dom, select, mut session) = setup();
    OptionsSetLength
        .invoke(select, &[JsValue::Number(4.0)], &mut session, &mut dom)
        .unwrap();
    let before = dom.children(select);
    OptionsSetLength
        .invoke(select, &[JsValue::Number(1.0)], &mut session, &mut dom)
        .unwrap();
    // Only the first option survives (last 3 removed).
    assert_eq!(dom.children(select), vec![before[0]]);
}

#[test]
fn options_set_length_over_cap_grow_is_noop() {
    // Grow above the cap on an empty select → no-op (HTML §2.6.4.3 step 2.1).
    let (mut dom, select, mut session) = setup();
    OptionsSetLength
        .invoke(select, &[JsValue::Number(10_001.0)], &mut session, &mut dom)
        .unwrap();
    assert!(dom.children(select).is_empty());
}

#[test]
fn options_set_length_shrink_runs_even_above_cap() {
    // Codex R3: the over-cap gate must live ONLY in the grow branch. When the
    // Options collection already exceeds `MAX_OPTIONS` (reachable by spreading
    // options across multiple `<optgroup>`s — the per-child-list sibling cap does
    // NOT bound the flattened descendant walk), a shrink to a target that is still
    // above the cap must remove the last `current - target` options, not no-op.
    let (mut dom, select, mut session) = setup();
    // Build current = MAX_OPTIONS + 2 across two optgroups (each child list under
    // the sibling cap), so the flattened Options count exceeds the cap.
    let half = elidex_ecs::MAX_ANCESTOR_DEPTH / 2 + 1; // 5001 each → 10002 total
    for _ in 0..2 {
        let og = dom.create_element("optgroup", Attributes::default());
        dom.append_child(select, og);
        for _ in 0..half {
            let opt = dom.create_element("option", Attributes::default());
            dom.append_child(og, opt);
        }
    }
    let current = elidex_ecs::MAX_ANCESTOR_DEPTH + 2;
    let mut opts = crate::LiveCollection::new(
        select,
        crate::CollectionFilter::Options,
        crate::CollectionKind::HtmlCollection,
    );
    assert_eq!(
        opts.length(&dom),
        current,
        "precondition: current exceeds the cap"
    );

    // target = current - 1 (a shrink), still > MAX_OPTIONS.
    #[expect(clippy::cast_precision_loss)]
    let target = (current - 1) as f64;
    OptionsSetLength
        .invoke(select, &[JsValue::Number(target)], &mut session, &mut dom)
        .unwrap();

    let mut opts_after = crate::LiveCollection::new(
        select,
        crate::CollectionFilter::Options,
        crate::CollectionKind::HtmlCollection,
    );
    assert_eq!(
        opts_after.length(&dom),
        current - 1,
        "shrink above the cap must remove the last option, not no-op"
    );
}
