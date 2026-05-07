//! `<select>` element initialization and interaction.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

use crate::{FormControlState, SelectOption};

/// Returns `true` if `entity` is an `<option>` whose own `disabled`
/// attribute is set OR whose enclosing tree contains a disabled
/// `<optgroup>` ancestor.  HTML §4.10.10.2 — an option is "disabled"
/// when either condition holds.  In well-formed markup optgroup
/// elements don't nest (parser flattens them), but the walker
/// climbs up to `MAX_ANCESTOR_DEPTH` ancestors and stops at the
/// enclosing `<select>`, so any disabled optgroup encountered
/// before that cutoff disables the option — mirrors browsers
/// that accept malformed nested-optgroup trees gracefully.
///
/// Returns `false` when `entity` is not actually an `<option>` (so
/// callers can pass arbitrary entities defensively without
/// mis-attributing a `disabled` attribute on, say, a `<button>`
/// to "option-disabled" semantics).
#[must_use]
pub fn is_option_disabled(dom: &EcsDom, entity: Entity) -> bool {
    let is_option = dom
        .world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("option"));
    if !is_option {
        return false;
    }
    if dom
        .world()
        .get::<&Attributes>(entity)
        .is_ok_and(|a| a.contains("disabled"))
    {
        return true;
    }
    let mut current = dom.get_parent(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let Some(ancestor) = current else {
            return false;
        };
        let is_disabled_optgroup = dom
            .world()
            .get::<&TagType>(ancestor)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("optgroup"))
            && dom
                .world()
                .get::<&Attributes>(ancestor)
                .is_ok_and(|a| a.contains("disabled"));
        if is_disabled_optgroup {
            return true;
        }
        // Stop at the enclosing `<select>` — disabled propagation
        // beyond the select is the form's `<fieldset>` problem.
        if dom
            .world()
            .get::<&TagType>(ancestor)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("select"))
        {
            return false;
        }
        current = dom.get_parent(ancestor);
    }
    false
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
}
