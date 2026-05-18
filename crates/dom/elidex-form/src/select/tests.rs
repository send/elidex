use super::*;
use crate::FormControlKind;
use elidex_ecs::EcsDom;

fn make_select_dom() -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());

    for (text, val) in [("Red", "r"), ("Green", "g"), ("Blue", "b")] {
        let opt = dom.create_element("option", {
            let mut a = Attributes::default();
            a.set("value", val);
            a
        });
        let tn = dom.create_text(text);
        let _ = dom.append_child(opt, tn);
        let _ = dom.append_child(sel, opt);
    }
    (dom, sel)
}

#[test]
fn init_select_collects_options() {
    let (dom, sel) = make_select_dom();
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert_eq!(state.options.len(), 3);
    assert_eq!(state.options[0].text, "Red");
    assert_eq!(state.options[0].value, "r");
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.value, "r");
}

#[test]
fn select_option_changes_value() {
    let (dom, sel) = make_select_dom();
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    select_option(&mut state, 2);
    assert_eq!(state.selected_index, 2);
    assert_eq!(state.value, "b");
}

#[test]
fn navigate_select_stops_at_end() {
    let (dom, sel) = make_select_dom();
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    select_option(&mut state, 2);
    // At the last option, forward navigation should not wrap.
    assert!(!navigate_select(&mut state, true));
    assert_eq!(state.selected_index, 2);
}

#[test]
fn navigate_select_stops_at_start() {
    let (dom, sel) = make_select_dom();
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    select_option(&mut state, 0);
    // At the first option, backward navigation should not wrap.
    assert!(!navigate_select(&mut state, false));
    assert_eq!(state.selected_index, 0);
}

#[test]
fn selected_attribute_respected() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    for (text, selected) in [("A", false), ("B", true), ("C", false)] {
        let mut a = Attributes::default();
        if selected {
            a.set("selected", "");
        }
        let opt = dom.create_element("option", a);
        let tn = dom.create_text(text);
        let _ = dom.append_child(opt, tn);
        let _ = dom.append_child(sel, opt);
    }
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert_eq!(state.selected_index, 1);
}

#[test]
fn optgroup_options() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let mut g_attrs = Attributes::default();
    g_attrs.set("label", "Fruits");
    let grp = dom.create_element("optgroup", g_attrs);
    let opt = dom.create_element("option", Attributes::default());
    let tn = dom.create_text("Apple");
    let _ = dom.append_child(opt, tn);
    let _ = dom.append_child(grp, opt);
    let _ = dom.append_child(sel, grp);

    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert_eq!(state.options.len(), 1);
    assert_eq!(state.options[0].group.as_deref(), Some("Fruits"));
}

#[test]
fn disabled_option_skipped_in_navigation() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let opt1 = dom.create_element("option", Attributes::default());
    let tn1 = dom.create_text("A");
    let _ = dom.append_child(opt1, tn1);
    let _ = dom.append_child(sel, opt1);

    let mut da = Attributes::default();
    da.set("disabled", "");
    let opt2 = dom.create_element("option", da);
    let tn2 = dom.create_text("B");
    let _ = dom.append_child(opt2, tn2);
    let _ = dom.append_child(sel, opt2);

    let opt3 = dom.create_element("option", Attributes::default());
    let tn3 = dom.create_text("C");
    let _ = dom.append_child(opt3, tn3);
    let _ = dom.append_child(sel, opt3);

    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert!(navigate_select(&mut state, true));
    // Should skip disabled "B" and go to "C"
    assert_eq!(state.selected_index, 2);
}

#[test]
fn option_value_defaults_to_text() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let tn = dom.create_text("Hello");
    let _ = dom.append_child(opt, tn);
    let _ = dom.append_child(sel, opt);

    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert_eq!(state.options[0].value, "Hello");
}

#[test]
fn empty_select() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert!(state.options.is_empty());
    assert_eq!(state.selected_index, -1);
}

#[test]
fn navigate_empty_select() {
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    assert!(!navigate_select(&mut state, true));
}

#[test]
fn select_negative_index() {
    let (dom, sel) = make_select_dom();
    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    select_option(&mut state, -1);
    assert_eq!(state.selected_index, -1);
    assert!(state.value.is_empty());
}

#[test]
fn multiple_select_no_auto_select() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", {
        let mut a = Attributes::default();
        a.set("multiple", "");
        a
    });
    let opt = dom.create_element("option", Attributes::default());
    let tn = dom.create_text("A");
    let _ = dom.append_child(opt, tn);
    let _ = dom.append_child(sel, opt);

    let mut state = FormControlState {
        kind: FormControlKind::Select,
        multiple: true,
        ..FormControlState::default()
    };
    init_select_options(&dom, sel, &mut state);
    // Multiple select should not auto-select.
    assert_eq!(state.selected_index, -1);
}

#[test]
fn disabled_optgroup_disables_children() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let mut g_attrs = Attributes::default();
    g_attrs.set("label", "G");
    g_attrs.set("disabled", "");
    let grp = dom.create_element("optgroup", g_attrs);
    let opt = dom.create_element("option", Attributes::default());
    let tn = dom.create_text("X");
    let _ = dom.append_child(opt, tn);
    let _ = dom.append_child(grp, opt);
    let _ = dom.append_child(sel, grp);

    let mut state = FormControlState::from_element("select", &Attributes::default()).unwrap();
    init_select_options(&dom, sel, &mut state);
    assert!(state.options[0].disabled);
}

#[test]
fn is_option_disabled_via_own_attribute() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("disabled", "");
    let opt = dom.create_element("option", attrs);
    assert!(is_option_disabled(&dom, opt));
}

#[test]
fn is_option_disabled_via_optgroup_ancestor() {
    let mut dom = EcsDom::new();
    let mut grp_attrs = Attributes::default();
    grp_attrs.set("disabled", "");
    let grp = dom.create_element("optgroup", grp_attrs);
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(grp, opt);
    assert!(is_option_disabled(&dom, opt));
}

#[test]
fn is_option_disabled_returns_false_when_neither_set() {
    let mut dom = EcsDom::new();
    let grp = dom.create_element("optgroup", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(grp, opt);
    assert!(!is_option_disabled(&dom, opt));
}

#[test]
fn is_option_disabled_returns_false_for_non_option_tag() {
    // R9 M2 regression — defensive tag gate: a `<div disabled>`
    // (or any non-option) must not be reported as
    // option-disabled even though the attribute matches.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("disabled", "");
    let div = dom.create_element("div", attrs);
    assert!(!is_option_disabled(&dom, div));
}

#[test]
fn is_option_disabled_stops_at_select() {
    // The select itself being disabled is a fieldset-style
    // concern; this helper is purely about option / optgroup
    // disable propagation, so a `disabled` attribute on the
    // enclosing select must NOT make every option disabled.
    let mut dom = EcsDom::new();
    let mut sel_attrs = Attributes::default();
    sel_attrs.set("disabled", "");
    let sel = dom.create_element("select", sel_attrs);
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(sel, opt);
    assert!(!is_option_disabled(&dom, opt));
}

// -- find_option_select tests (D-1 hoist target) ---------------

#[test]
fn find_option_select_direct_parent() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(sel, opt);
    assert_eq!(find_option_select(&dom, opt), Some(sel));
}

#[test]
fn find_option_select_via_optgroup() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let grp = dom.create_element("optgroup", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(sel, grp);
    let _ = dom.append_child(grp, opt);
    assert_eq!(find_option_select(&dom, opt), Some(sel));
}

#[test]
fn find_option_select_through_arbitrary_wrapper() {
    // JS-driven `<select><div><option>...` — JS DOM mutation can
    // introduce wrappers between option and select; the walker
    // climbs through them.
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    let wrapper = dom.create_element("div", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(sel, wrapper);
    let _ = dom.append_child(wrapper, opt);
    assert_eq!(find_option_select(&dom, opt), Some(sel));
}

#[test]
fn find_option_select_returns_none_for_detached() {
    let mut dom = EcsDom::new();
    let opt = dom.create_element("option", Attributes::default());
    assert_eq!(find_option_select(&dom, opt), None);
}

#[test]
fn find_option_select_returns_none_when_no_select_ancestor() {
    // Option attached to a non-select container (e.g. a stray
    // `<div>`) — `option.form` should return null.
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(div, opt);
    assert_eq!(find_option_select(&dom, opt), None);
}

#[test]
fn find_option_select_case_insensitive_tag() {
    // JS-driven `document.createElement("SELECT")` keeps the
    // mixed-case tag.  ASCII-CI match should still resolve.
    let mut dom = EcsDom::new();
    let sel = dom.create_element("SELECT", Attributes::default());
    let opt = dom.create_element("option", Attributes::default());
    let _ = dom.append_child(sel, opt);
    assert_eq!(find_option_select(&dom, opt), Some(sel));
}

// -- select_uses_implicit_default tests (D-3 hoist target) -----

#[test]
fn implicit_default_true_for_default_select() {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", Attributes::default());
    assert!(select_uses_implicit_default(&dom, sel));
}

#[test]
fn implicit_default_false_when_multiple() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("multiple", "");
    let sel = dom.create_element("select", attrs);
    assert!(!select_uses_implicit_default(&dom, sel));
}

#[test]
fn implicit_default_false_when_size_gt_one() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("size", "5");
    let sel = dom.create_element("select", attrs);
    assert!(!select_uses_implicit_default(&dom, sel));
}

#[test]
fn implicit_default_true_when_size_invalid() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("size", "0");
    let sel = dom.create_element("select", attrs);
    assert!(select_uses_implicit_default(&dom, sel));
}

// -- shared test fixture (used across D-3 and D-4 hoist tests) -

/// Per-option spec for [`build_select`].  All fields default to
/// the absent / unset state; tests opt in to the aspects they
/// need (`marked`, `valued`, free-form struct literal).
#[derive(Default)]
struct TestOpt {
    selected: bool,
    disabled: bool,
    value_attr: Option<&'static str>,
    text: Option<&'static str>,
}

impl TestOpt {
    fn marked(selected: bool, disabled: bool) -> Self {
        Self {
            selected,
            disabled,
            ..Default::default()
        }
    }
    fn valued(value_attr: &'static str) -> Self {
        Self {
            value_attr: Some(value_attr),
            ..Default::default()
        }
    }
}

fn build_select(sel_attrs: Attributes, opts: &[TestOpt]) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let sel = dom.create_element("select", sel_attrs);
    for o in opts {
        let mut attrs = Attributes::default();
        if o.selected {
            attrs.set("selected", "");
        }
        if o.disabled {
            attrs.set("disabled", "");
        }
        if let Some(v) = o.value_attr {
            attrs.set("value", v);
        }
        let opt = dom.create_element("option", attrs);
        assert!(dom.append_child(sel, opt));
        if let Some(t) = o.text {
            let tn = dom.create_text(t);
            assert!(dom.append_child(opt, tn));
        }
    }
    (dom, sel)
}

// -- select_selected_index tests (D-3 hoist target) ------------

#[test]
fn selected_index_explicit_selection_wins() {
    let (dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::marked(false, false), TestOpt::marked(true, false)],
    );
    assert_eq!(select_selected_index(&dom, sel), 1.0);
}

#[test]
fn selected_index_implicit_default_first_non_disabled() {
    let (dom, sel) = build_select(
        Attributes::default(),
        &[
            TestOpt::marked(false, true),
            TestOpt::marked(false, true),
            TestOpt::marked(false, false),
        ],
    );
    assert_eq!(select_selected_index(&dom, sel), 2.0);
}

#[test]
fn selected_index_returns_neg1_when_all_disabled_and_none_selected() {
    let (dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::marked(false, true), TestOpt::marked(false, true)],
    );
    assert_eq!(select_selected_index(&dom, sel), -1.0);
}

#[test]
fn selected_index_returns_neg1_for_listbox_with_no_selection() {
    let mut attrs = Attributes::default();
    attrs.set("size", "5");
    let (dom, sel) = build_select(attrs, &[TestOpt::default(), TestOpt::default()]);
    assert_eq!(select_selected_index(&dom, sel), -1.0);
}

#[test]
fn selected_index_returns_neg1_for_multiple_with_no_selection() {
    let mut attrs = Attributes::default();
    attrs.set("multiple", "");
    let (dom, sel) = build_select(attrs, &[TestOpt::default(), TestOpt::default()]);
    assert_eq!(select_selected_index(&dom, sel), -1.0);
}

#[test]
fn selected_index_explicit_overrides_disabled_and_listbox() {
    // Even on a listbox `<select size=5>`, an explicit `selected`
    // attr wins.
    let mut attrs = Attributes::default();
    attrs.set("size", "5");
    let (dom, sel) = build_select(
        attrs,
        &[
            TestOpt::default(),
            TestOpt::marked(true, false),
            TestOpt::default(),
        ],
    );
    assert_eq!(select_selected_index(&dom, sel), 1.0);
}

#[test]
fn selected_index_first_explicit_wins_over_later_explicit() {
    let (dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::marked(true, false), TestOpt::marked(true, false)],
    );
    assert_eq!(select_selected_index(&dom, sel), 0.0);
}

// -- option_value_string / select_get_value tests (D-4 hoist) --

#[test]
fn option_value_string_uses_value_attr_when_present() {
    let (dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt {
            value_attr: Some("blue"),
            text: Some("Blue"),
            ..Default::default()
        }],
    );
    let opt = dom.children(sel)[0];
    assert_eq!(option_value_string(&dom, opt), "blue");
}

#[test]
fn option_value_string_falls_back_to_text_content() {
    let (dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt {
            text: Some("Red"),
            ..Default::default()
        }],
    );
    let opt = dom.children(sel)[0];
    assert_eq!(option_value_string(&dom, opt), "Red");
}

#[test]
fn option_value_string_empty_when_neither_present() {
    let (dom, sel) = build_select(Attributes::default(), &[TestOpt::default()]);
    let opt = dom.children(sel)[0];
    assert_eq!(option_value_string(&dom, opt), "");
}

#[test]
fn select_get_value_returns_first_selected_option_value() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[
            TestOpt::valued("a"),
            TestOpt::valued("b"),
            TestOpt::valued("c"),
        ],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[1], "selected", "");
    assert_eq!(select_get_value(&dom, sel), "b");
}

#[test]
fn select_get_value_implicit_default_first_non_disabled() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[
            TestOpt::valued("a"),
            TestOpt::valued("b"),
            TestOpt::valued("c"),
        ],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[0], "disabled", "");
    assert_eq!(select_get_value(&dom, sel), "b");
}

#[test]
fn select_get_value_returns_empty_string_when_listbox_with_no_selection() {
    let mut attrs = Attributes::default();
    attrs.set("size", "5");
    let (dom, sel) = build_select(attrs, &[TestOpt::valued("a")]);
    assert_eq!(select_get_value(&dom, sel), "");
}

// -- select_set_value tests (D-4 hoist) ------------------------

#[test]
fn select_set_value_marks_first_matching_option() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[
            TestOpt::valued("a"),
            TestOpt::valued("b"),
            TestOpt::valued("c"),
        ],
    );
    select_set_value(&mut dom, sel, "b");
    assert_eq!(select_selected_index(&dom, sel), 1.0);
    assert_eq!(select_get_value(&dom, sel), "b");
}

#[test]
fn select_set_value_clears_other_selections() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::valued("a"), TestOpt::valued("b")],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[0], "selected", "");
    select_set_value(&mut dom, sel, "b");
    // First option's `selected` attribute was cleared.
    assert!(!dom
        .world()
        .get::<&Attributes>(opts[0])
        .unwrap()
        .contains("selected"));
    assert!(dom
        .world()
        .get::<&Attributes>(opts[1])
        .unwrap()
        .contains("selected"));
}

#[test]
fn select_set_value_no_match_leaves_no_selection() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::valued("a"), TestOpt::valued("b")],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[0], "selected", "");
    select_set_value(&mut dom, sel, "missing");
    // All options' `selected` attribute is cleared.
    assert!(!dom
        .world()
        .get::<&Attributes>(opts[0])
        .unwrap()
        .contains("selected"));
}

#[test]
fn select_set_value_first_match_wins() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[
            TestOpt::valued("dup"),
            TestOpt::valued("dup"),
            TestOpt::valued("other"),
        ],
    );
    select_set_value(&mut dom, sel, "dup");
    assert_eq!(select_selected_index(&dom, sel), 0.0);
}

// -- select_set_selected_index tests (D-4 hoist) ---------------

#[test]
fn select_set_selected_index_in_range() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::valued("a"), TestOpt::valued("b")],
    );
    select_set_selected_index(&mut dom, sel, 1);
    assert_eq!(select_selected_index(&dom, sel), 1.0);
}

#[test]
fn select_set_selected_index_clears_existing_then_sets() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::valued("a"), TestOpt::valued("b")],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[0], "selected", "");
    select_set_selected_index(&mut dom, sel, 1);
    assert!(!dom
        .world()
        .get::<&Attributes>(opts[0])
        .unwrap()
        .contains("selected"));
    assert!(dom
        .world()
        .get::<&Attributes>(opts[1])
        .unwrap()
        .contains("selected"));
}

#[test]
fn select_set_selected_index_negative_clears_all() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::valued("a"), TestOpt::valued("b")],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[0], "selected", "");
    select_set_selected_index(&mut dom, sel, -1);
    for opt in &opts {
        assert!(!dom
            .world()
            .get::<&Attributes>(*opt)
            .unwrap()
            .contains("selected"));
    }
}

#[test]
fn select_set_selected_index_out_of_range_clears_all() {
    let (mut dom, sel) = build_select(
        Attributes::default(),
        &[TestOpt::valued("a"), TestOpt::valued("b")],
    );
    let opts: Vec<Entity> = dom.children(sel);
    dom.set_attribute(opts[0], "selected", "");
    select_set_selected_index(&mut dom, sel, 99);
    for opt in &opts {
        assert!(!dom
            .world()
            .get::<&Attributes>(*opt)
            .unwrap()
            .contains("selected"));
    }
}
