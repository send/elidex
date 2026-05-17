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
/// Four call sites (`selectedIndex` getter / `value` getter /
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
/// returns the index of the first non-disabled option.  Returns
/// `-1.0` when the select has no usable selection.
///
/// Returns `f64` (matching the JS-observable `JsValue::Number(...)`
/// shape) rather than WebIDL-spec `long` / `i32`, so the pre-PR
/// VM-host saturation cap (`u32::MAX` → `f64::from`) is preserved
/// exactly.  The "pure hoist / no behavior change" invariant for
/// slot `#11-tags-T1-v2-drift-hoist` requires preserving the prior
/// saturation; tightening to a true WebIDL `long` (i32-clamped)
/// would be a separate intentional behaviour change.
#[must_use]
pub fn select_selected_index(dom: &EcsDom, select: Entity) -> f64 {
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
            return f64::from(u32::try_from(idx).unwrap_or(u32::MAX));
        }
    }
    if select_uses_implicit_default(dom, select) {
        for (idx, opt) in snap.iter().enumerate() {
            if !is_option_disabled(dom, *opt) {
                return f64::from(u32::try_from(idx).unwrap_or(u32::MAX));
            }
        }
    }
    -1.0
}

/// Compute a single `<option>`'s value (HTML §4.10.10).
///
/// Returns the `value` content attribute when present; otherwise
/// falls back to the descendant text content (concatenation of all
/// text-node descendants).
#[must_use]
pub fn option_value_string(dom: &EcsDom, option: Entity) -> String {
    if let Ok(attrs) = dom.world().get::<&Attributes>(option) {
        if let Some(v) = attrs.get("value") {
            return v.to_string();
        }
    }
    elidex_dom_api::element::collect_text_content(option, dom)
}

/// Resolve `<select>.value` (HTML §4.10.10).
///
/// Returns the first selected option's value; if no option carries
/// the `selected` attribute and the select is in implicit-default
/// mode (see [`select_uses_implicit_default`]), returns the value of
/// the first non-disabled option.  Returns an empty string when no
/// usable option is found.
#[must_use]
pub fn select_get_value(dom: &EcsDom, select: Entity) -> String {
    let mut opts = elidex_dom_api::LiveCollection::new(
        select,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let snap = opts.snapshot(dom).to_vec();
    for opt in &snap {
        if dom
            .world()
            .get::<&Attributes>(*opt)
            .is_ok_and(|a| a.contains("selected"))
        {
            return option_value_string(dom, *opt);
        }
    }
    if select_uses_implicit_default(dom, select) {
        for opt in &snap {
            if !is_option_disabled(dom, *opt) {
                return option_value_string(dom, *opt);
            }
        }
    }
    String::new()
}

/// Set `<select>.value` (HTML §4.10.7.4 value setter).
///
/// The first option whose value (per [`option_value_string`]) equals
/// `target` becomes selected; all other options have their `selected`
/// attribute cleared.  When no option matches, every option ends up
/// without a `selected` attribute.
pub fn select_set_value(dom: &mut EcsDom, select: Entity, target: &str) {
    let mut opts = elidex_dom_api::LiveCollection::new(
        select,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let snap = opts.snapshot(dom).to_vec();
    let mut found_first = false;
    for opt in &snap {
        let candidate = option_value_string(dom, *opt);
        if !found_first && candidate == target {
            dom.set_attribute(*opt, "selected", String::new());
            found_first = true;
        } else {
            dom.remove_attribute(*opt, "selected");
        }
    }
}

/// Set `<select>.selectedIndex` (HTML §4.10.10 selectedIndex setter).
///
/// All `selected` attributes are cleared from the option list.  If
/// `n` is a valid in-range non-negative index, the option at that
/// index gets `selected` set; out-of-range / negative indices result
/// in no option being selected.
pub fn select_set_selected_index(dom: &mut EcsDom, select: Entity, n: i32) {
    let mut opts = elidex_dom_api::LiveCollection::new(
        select,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let snap = opts.snapshot(dom).to_vec();
    for opt in &snap {
        dom.remove_attribute(*opt, "selected");
    }
    if let Ok(idx) = usize::try_from(n) {
        if let Some(target) = snap.get(idx) {
            dom.set_attribute(*target, "selected", String::new());
        }
    }
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

/// If the most recently appended option carries `selected` AND no
/// earlier selection has been recorded (`*selected_index < 0`), update
/// `*selected_index` in place to that option's position.  Returns
/// nothing — the result is signalled by the mutation.
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
mod tests;
