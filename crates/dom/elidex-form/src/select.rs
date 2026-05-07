//! `<select>` element initialization and interaction.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

use crate::{FormControlState, SelectOption};

/// `<option>` disabledness predicate (HTML §4.10.10.2).
///
/// Re-exported from `elidex-dom-api` (canonical home) per slot
/// `#11-tags-T1-v2-drift-hoist` (D-6) — the algorithm walks the DOM
/// ancestor chain over content attributes, which is engine-independent
/// DOM API territory rather than form-specific.  `elidex-form`
/// continues to surface the predicate for back-compat with the
/// historical caller surface (`vm/host/html_select_proto.rs` /
/// `init_select_options` etc.).
pub use elidex_dom_api::element::is_option_disabled;

/// HTML §4.10.10.2 "ask for a reset" implicit-default predicate.
///
/// Returns `true` when a `<select>`'s implicit default selection (the
/// first non-disabled option) is in effect: the select must not be
/// `multiple` AND its display size must be 1.  Display size = parsed
/// positive `size` attribute, defaulting to 1 when missing / "0" /
/// invalid.
///
/// Three call sites (`selectedIndex` getter / `value` getter /
/// `init_select_options` / `populate_selected_options`) must agree
/// on this gate so the surfaces stay consistent.
#[must_use]
pub fn select_uses_implicit_default(dom: &EcsDom, select: Entity) -> bool {
    if dom
        .world()
        .get::<&Attributes>(select)
        .is_ok_and(|a| a.contains("multiple"))
    {
        return false;
    }
    let display_size = dom
        .world()
        .get::<&Attributes>(select)
        .ok()
        .and_then(|a| a.get("size").and_then(|s| s.parse::<u32>().ok()))
        .filter(|&n| n > 0)
        .unwrap_or(1);
    display_size <= 1
}

/// Resolve `<select>.selectedIndex` (HTML §4.10.10.2).
///
/// Returns the index of the first option whose own `selected` content
/// attribute is set.  When no option is explicitly selected and the
/// select is in implicit-default mode (see [`select_uses_implicit_default`]),
/// returns the index of the first non-disabled option.  Returns `-1`
/// when the select has no usable selection.
#[must_use]
pub fn select_selected_index(dom: &EcsDom, select: Entity) -> i32 {
    let mut opts = elidex_dom_api::LiveCollection::new(
        select,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let snap = opts.snapshot(dom).to_vec();
    for (idx, opt) in snap.iter().enumerate() {
        if dom
            .world()
            .get::<&Attributes>(*opt)
            .is_ok_and(|a| a.contains("selected"))
        {
            return i32::try_from(idx).unwrap_or(i32::MAX);
        }
    }
    if select_uses_implicit_default(dom, select) {
        for (idx, opt) in snap.iter().enumerate() {
            if !is_option_disabled(dom, *opt) {
                return i32::try_from(idx).unwrap_or(i32::MAX);
            }
        }
    }
    -1
}

/// Find the nearest `<select>` ancestor of `option` (HTML §4.10.10).
///
/// Used by `option.form` (HTML §4.10.10 — the form owner of an
/// option is the form owner of its enclosing `<select>`, walking
/// past any `<optgroup>` or other wrapper element JS DOM mutation
/// can introduce). Bounded by `MAX_ANCESTOR_DEPTH` so a buggy
/// `appendChild` cycle-check regression cannot wedge this accessor
/// in an infinite loop. Returns `None` for detached options or
/// options whose ancestor chain doesn't reach a `<select>`.
///
/// Tag matching is ASCII case-insensitive so JS-driven creation
/// (`document.createElement("SELECT")`) is tolerated.
#[must_use]
pub fn find_option_select(dom: &EcsDom, option: Entity) -> Option<Entity> {
    let mut current = dom.get_parent(option);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let p = current?;
        let is_select = dom
            .world()
            .get::<&TagType>(p)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("select"));
        if is_select {
            return Some(p);
        }
        current = dom.get_parent(p);
    }
    None
}

/// Compute `<option>.index` (HTML §4.10.10): walks up to the
/// enclosing `<select>` / `<datalist>` (skipping any `<optgroup>` /
/// other wrapper, bounded by `MAX_ANCESTOR_DEPTH`), then descends
/// through the container's option / optgroup tree to count this
/// option's position.  Returns `None` for detached options or
/// options with no enclosing container.
///
/// `<optgroup>` nesting is technically forbidden by the spec but
/// JS-driven `appendChild` can construct it; this walker tolerates
/// arbitrary depth (capped by `MAX_ANCESTOR_DEPTH`) so the index
/// stays meaningful for malformed-but-constructible trees.
#[must_use]
pub fn find_option_index_in_tree(dom: &EcsDom, option: Entity) -> Option<i32> {
    let container = find_options_container(dom, option)?;
    let mut count: u32 = 0;
    let mut found: i32 = -1;
    walk_options(dom, container, &mut count, option, &mut found, 0);
    if found >= 0 {
        Some(found)
    } else {
        None
    }
}

/// Walk up the option's ancestor chain (bounded by
/// `MAX_ANCESTOR_DEPTH`) until reaching the first `<select>` or
/// `<datalist>` element.  Skips intermediate `<optgroup>` /
/// `<div>` / etc. so JS-constructed nested-optgroup trees still
/// resolve correctly.
fn find_options_container(dom: &EcsDom, option: Entity) -> Option<Entity> {
    let mut current = dom.get_parent(option)?;
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let is_container = dom.world().get::<&TagType>(current).is_ok_and(|t| {
            t.0.eq_ignore_ascii_case("select") || t.0.eq_ignore_ascii_case("datalist")
        });
        if is_container {
            return Some(current);
        }
        current = dom.get_parent(current)?;
    }
    None
}

fn walk_options(
    dom: &EcsDom,
    parent: Entity,
    count: &mut u32,
    target: Entity,
    found: &mut i32,
    depth: usize,
) {
    // Cap recursion depth — JS can construct pathologically nested
    // `<optgroup>` (spec forbids, parser doesn't reject).  Bail at
    // `MAX_ANCESTOR_DEPTH` so `option.index` can't stack-overflow.
    if depth >= MAX_ANCESTOR_DEPTH {
        return;
    }
    let Some(mut child) = dom.get_first_child(parent) else {
        return;
    };
    loop {
        let tag_is_option = dom
            .world()
            .get::<&TagType>(child)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("option"));
        let tag_is_optgroup = dom
            .world()
            .get::<&TagType>(child)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("optgroup"));
        if tag_is_option {
            if child == target {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                {
                    *found = i32::try_from(*count).unwrap_or(i32::MAX);
                }
                return;
            }
            *count += 1;
        } else if tag_is_optgroup {
            walk_options(dom, child, count, target, found, depth + 1);
            if *found >= 0 {
                return;
            }
        }
        let Some(next) = dom.get_next_sibling(child) else {
            return;
        };
        child = next;
    }
}

/// Try to mark an option index as selected, returning the i32 index.
fn try_mark_selected(options: &[crate::SelectOption], selected_index: &mut i32) {
    if options.last().is_some_and(|opt| opt.selected) && *selected_index < 0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        {
            *selected_index = (options.len() - 1) as i32;
        }
    }
}

/// Initialize select options by walking child `<option>` and `<optgroup>` elements.
pub fn init_select_options(dom: &EcsDom, entity: Entity, state: &mut FormControlState) {
    let mut options = Vec::new();
    let mut selected_index: i32 = -1;

    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        if dom.has_tag(c, "option") {
            let opt = parse_option(dom, c, None);
            options.push(opt);
            try_mark_selected(&options, &mut selected_index);
        } else if dom.has_tag(c, "optgroup") {
            // Read attributes once per optgroup (L5).
            let (group_label, group_disabled) = dom
                .world()
                .get::<&Attributes>(c)
                .ok()
                .map_or((None, false), |a| {
                    (a.get("label").map(String::from), a.contains("disabled"))
                });

            let mut opt_child = dom.get_first_child(c);
            while let Some(oc) = opt_child {
                if dom.has_tag(oc, "option") {
                    let mut opt = parse_option(dom, oc, group_label.clone());
                    if group_disabled {
                        opt.disabled = true;
                    }
                    options.push(opt);
                    try_mark_selected(&options, &mut selected_index);
                }
                opt_child = dom.get_next_sibling(oc);
            }
        }
        child = dom.get_next_sibling(c);
    }

    // Guard: option count must fit in i32 for selected_index.
    if options.len() > i32::MAX as usize {
        state.options = options;
        return;
    }

    // HTML spec §4.10.5: if no option is selected, select the first non-disabled
    // option — but only for single-select without explicit size > 1.
    // For `<select multiple>` or `<select size="N">` (N > 1), no auto-selection.
    if selected_index < 0 && !options.is_empty() && !state.multiple && state.size <= 1 {
        for (i, opt) in options.iter().enumerate() {
            if !opt.disabled {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                {
                    selected_index = i as i32;
                }
                break;
            }
        }
    }

    if selected_index >= 0 {
        #[allow(clippy::cast_sign_loss)]
        if let Some(opt) = options.get_mut(selected_index as usize) {
            opt.selected = true;
            state.value.clone_from(&opt.value);
        }
    }

    state.selected_index = selected_index;
    state.options = options;
}

fn parse_option(dom: &EcsDom, entity: Entity, group: Option<String>) -> SelectOption {
    let attrs = dom.world().get::<&Attributes>(entity).ok();
    let text = get_option_text(dom, entity);
    let value = attrs
        .as_ref()
        .and_then(|a| a.get("value").map(String::from))
        .unwrap_or_else(|| text.clone());
    let disabled = attrs.as_ref().is_some_and(|a| a.contains("disabled"));
    let selected = attrs.as_ref().is_some_and(|a| a.contains("selected"));
    SelectOption {
        text,
        value,
        disabled,
        group,
        selected,
    }
}

/// Get the text content of an `<option>` element.
///
/// Per WHATWG §4.10.5 "option text": strip/collapse whitespace — leading/trailing
/// whitespace is removed, and internal runs of whitespace are collapsed to a single space.
fn get_option_text(dom: &EcsDom, entity: Entity) -> String {
    let mut text = String::new();
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        if let Ok(tc) = dom.world().get::<&elidex_ecs::TextContent>(c) {
            text.push_str(&tc.0);
        }
        child = dom.get_next_sibling(c);
    }
    // WHATWG §4.10.5: strip and collapse whitespace.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Clear all `selected` flags on options.
fn clear_all_selected(state: &mut FormControlState) {
    for opt in &mut state.options {
        opt.selected = false;
    }
}

/// Select an option by index.
///
/// Programmatic selection (JS `selectedIndex = n`) is allowed even for
/// disabled options per HTML spec §4.10.10.3.
pub fn select_option(state: &mut FormControlState, index: i32) {
    if index < 0 {
        clear_all_selected(state);
        state.selected_index = -1;
        state.value.clear();
        return;
    }
    #[allow(clippy::cast_sign_loss)]
    let idx = index as usize;
    if idx < state.options.len() {
        clear_all_selected(state);
        state.options[idx].selected = true;
        state.selected_index = index;
        state.value.clone_from(&state.options[idx].value);
    }
}

/// Navigate select options with arrow keys.
///
/// Returns `true` if the selection changed.
/// Stops at the first/last option (no wraparound, per native browser behavior).
pub fn navigate_select(state: &mut FormControlState, forward: bool) -> bool {
    if state.options.is_empty() {
        return false;
    }
    let current = state.selected_index.max(0);
    #[allow(clippy::cast_sign_loss)]
    let mut idx = current as usize;
    let len = state.options.len();

    // Find next non-disabled option without wrapping.
    loop {
        if forward {
            if idx + 1 >= len {
                return false; // Already at end.
            }
            idx += 1;
        } else {
            if idx == 0 {
                return false; // Already at start.
            }
            idx -= 1;
        }
        if !state.options[idx].disabled {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            {
                select_option(state, idx as i32);
            }
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
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

    // -- select_selected_index tests (D-3 hoist target) ------------

    fn build_select_with_options(
        attrs: Attributes,
        options: &[(bool, bool)], // (selected, disabled)
    ) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let sel = dom.create_element("select", attrs);
        for (selected, disabled) in options {
            let mut opt_attrs = Attributes::default();
            if *selected {
                opt_attrs.set("selected", "");
            }
            if *disabled {
                opt_attrs.set("disabled", "");
            }
            let opt = dom.create_element("option", opt_attrs);
            assert!(dom.append_child(sel, opt));
        }
        (dom, sel)
    }

    #[test]
    fn selected_index_explicit_selection_wins() {
        let (dom, sel) =
            build_select_with_options(Attributes::default(), &[(false, false), (true, false)]);
        assert_eq!(select_selected_index(&dom, sel), 1);
    }

    #[test]
    fn selected_index_implicit_default_first_non_disabled() {
        let (dom, sel) = build_select_with_options(
            Attributes::default(),
            &[(false, true), (false, true), (false, false)],
        );
        assert_eq!(select_selected_index(&dom, sel), 2);
    }

    #[test]
    fn selected_index_returns_neg1_when_all_disabled_and_none_selected() {
        let (dom, sel) =
            build_select_with_options(Attributes::default(), &[(false, true), (false, true)]);
        assert_eq!(select_selected_index(&dom, sel), -1);
    }

    #[test]
    fn selected_index_returns_neg1_for_listbox_with_no_selection() {
        let mut attrs = Attributes::default();
        attrs.set("size", "5");
        let (dom, sel) = build_select_with_options(attrs, &[(false, false), (false, false)]);
        assert_eq!(select_selected_index(&dom, sel), -1);
    }

    #[test]
    fn selected_index_returns_neg1_for_multiple_with_no_selection() {
        let mut attrs = Attributes::default();
        attrs.set("multiple", "");
        let (dom, sel) = build_select_with_options(attrs, &[(false, false), (false, false)]);
        assert_eq!(select_selected_index(&dom, sel), -1);
    }

    #[test]
    fn selected_index_explicit_overrides_disabled_and_listbox() {
        // Even on a listbox `<select size=5>`, an explicit `selected`
        // attr wins.
        let mut attrs = Attributes::default();
        attrs.set("size", "5");
        let (dom, sel) =
            build_select_with_options(attrs, &[(false, false), (true, false), (false, false)]);
        assert_eq!(select_selected_index(&dom, sel), 1);
    }

    #[test]
    fn selected_index_first_explicit_wins_over_later_explicit() {
        let (dom, sel) =
            build_select_with_options(Attributes::default(), &[(true, false), (true, false)]);
        assert_eq!(select_selected_index(&dom, sel), 0);
    }
}
