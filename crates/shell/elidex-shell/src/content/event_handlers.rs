//! Input event handlers: click, mouse move/release, cursor leave, keyboard.

use elidex_ecs::ElementState as DomElementState;
use elidex_form::{FormControlKind, FormControlState, KeyAction};
use elidex_layout::{hit_test_with_scroll, HitTestQuery};
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit, Point};
use elidex_script_session::DispatchEvent;

use crate::app::hover::{apply_hover_diff, collect_hover_chain, update_element_state};
use crate::app::navigation::resolve_nav_url;
use crate::ipc::ModifierState;

use elidex_dom_api::focus::current_focus;

use super::focus::set_focus;
use super::form_input::{
    dispatch_input_event, dispatch_input_event_typed, dispatch_state_change_events,
    handle_form_reset, handle_form_submit, handle_label_click, toggle_checkbox_if_needed,
};
use super::navigation::{handle_navigate, process_pending_actions};
use super::ContentState;

/// Clear `:active` state from all entities in the active chain.
fn clear_active_chain(state: &mut ContentState) {
    for &e in &std::mem::take(&mut state.active_chain) {
        if state.pipeline.dom.contains(e) {
            update_element_state(&mut state.pipeline.dom, e, |s| {
                s.remove(DomElementState::ACTIVE);
            });
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn handle_click(state: &mut ContentState, click: &crate::ipc::MouseClickEvent) {
    let query = HitTestQuery {
        point: click.point,
        scroll: state.viewport_scroll.scroll_offset,
    };
    let Some(hit) = hit_test_with_scroll(&state.pipeline.dom, &query) else {
        return;
    };
    let hit_entity = hit.entity;

    // Route clicks on iframe elements to the iframe's own DOM.
    // Events do not bubble out of iframe boundaries (WHATWG HTML).
    if try_route_click_to_iframe(state, hit_entity, click) {
        state.re_render();
        state.send_display_list();
        return;
    }

    // Focus moved to a top-level element — the parent document holds focus.
    // Blur any iframe that previously held focus: its separate `EcsDom` is
    // unreachable from the parent `set_focus` below, so without this its control
    // keeps `:focus` / caret and the iframe's `activeElement` stays stale (the
    // cross-frame counterpart of the parent blur in `try_route_click_to_iframe`).
    // `take()` also clears key routing so `try_route_key_to_iframe` stops
    // sending keystrokes into the now-unfocused iframe.
    if let Some(prev) = state.focused_iframe.take() {
        blur_iframe_focus(state, prev);
    }
    // Update focus.
    set_focus(&mut state.pipeline, hit_entity);

    // Set ACTIVE state on press. Per UI Events spec, :active applies from
    // mousedown to mouseup — cleared in handle_mouse_release().
    // Clear any stale ACTIVE from a previous press (e.g. MouseRelease lost
    // due to window focus change).
    clear_active_chain(state);
    state.active_chain = state.hover_chain.clone();
    for &e in &state.active_chain {
        update_element_state(&mut state.pipeline.dom, e, |s| {
            s.insert(DomElementState::ACTIVE);
        });
    }

    // Use viewport-relative coordinates for DOM event properties (clientX/clientY).
    let mouse_init = MouseEventInit {
        client_x: click.client_point.x,
        client_y: click.client_point.y,
        button: i16::from(click.button),
        alt_key: click.mods.alt,
        ctrl_key: click.mods.ctrl,
        meta_key: click.mods.meta,
        shift_key: click.mods.shift,
        ..Default::default()
    };

    // DOM spec: click fires only for the primary button (button 0);
    // auxclick fires for non-primary buttons (UI Events §3.5).
    let event_types: &[&str] = if click.button == 0 {
        &["mousedown", "mouseup", "click"]
    } else {
        &["mousedown", "mouseup", "auxclick"]
    };

    let mut click_prevented = false;
    for event_type in event_types {
        let mut event = DispatchEvent::new_composed(*event_type, hit_entity);
        // UI Events §3.5: auxclick is not cancelable.
        if *event_type == "auxclick" {
            event.cancelable = false;
        }
        event.payload = EventPayload::Mouse(mouse_init.clone());
        let prevented = state.pipeline.dispatch_event(&mut event);
        if *event_type == "click" {
            click_prevented = prevented;
        }
    }

    // Checkbox toggle on click (if not prevented).
    let checkbox_toggled = click.button == 0
        && !click_prevented
        && toggle_checkbox_if_needed(&mut state.pipeline.dom, hit_entity);
    if checkbox_toggled {
        dispatch_state_change_events(state, hit_entity);
    }

    // Read FormControlState once for radio/select/submit/reset dispatch (R14).
    if click.button == 0 && !click_prevented {
        let control_kind = state
            .pipeline
            .dom
            .world()
            .get::<&FormControlState>(hit_entity)
            .ok()
            .filter(|fcs| !fcs.disabled)
            .map(|fcs| fcs.kind);

        match control_kind {
            Some(FormControlKind::Radio)
                if elidex_form::toggle_radio(
                    &mut state.pipeline.dom,
                    hit_entity,
                    &mut state.pipeline.ancestor_cache,
                ) =>
            {
                dispatch_state_change_events(state, hit_entity);
            }
            Some(FormControlKind::Select) => {
                if let Ok(mut fcs) = state
                    .pipeline
                    .dom
                    .world_mut()
                    .get::<&mut FormControlState>(hit_entity)
                {
                    fcs.dropdown_open = !fcs.dropdown_open;
                }
            }
            Some(FormControlKind::SubmitButton) => {
                handle_form_submit(state, hit_entity);
            }
            Some(FormControlKind::ResetButton) => {
                handle_form_reset(state, hit_entity);
            }
            _ => {}
        }
    }

    // Label click → focus associated control.
    // Skip label-triggered toggle if the hit entity itself was already toggled
    // (prevents double-toggle when clicking a checkbox inside a <label>).
    if click.button == 0 && !click_prevented {
        handle_label_click(state, hit_entity, checkbox_toggled);
    }

    state.re_render();

    if process_pending_actions(state) {
        return;
    }

    // Link navigation: if click was not prevented, check for <a href>.
    if click.button == 0 && !click_prevented {
        if let Some((href, target_attr)) =
            crate::app::events::find_link_ancestor_with_target(&state.pipeline.dom, hit_entity)
        {
            let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &href);
            if let Some(target_url) = resolved {
                match target_attr.as_deref() {
                    Some("_blank") => {
                        // Sandbox allow-popups check (WHATWG HTML §4.8.5):
                        // block popup navigation from sandboxed iframes without
                        // the allow-popups flag.
                        if !state.pipeline.runtime.bridge().popups_allowed() {
                            state.send_display_list();
                            return;
                        }
                        // Open in a new tab.
                        let _ = state
                            .channel
                            .send(crate::ipc::ContentToBrowser::OpenNewTab(target_url));
                        state.send_display_list();
                        return;
                    }
                    Some("_top" | "_parent")
                        if state
                            .pipeline
                            .runtime
                            .bridge()
                            .sandbox_flags()
                            .is_some_and(|f| {
                                !f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_TOP_NAVIGATION)
                            }) =>
                    {
                        // Sandbox allow-top-navigation check (WHATWG HTML §4.8.5):
                        // block navigation to parent/top from sandboxed iframes
                        // without the allow-top-navigation flag.
                        state.send_display_list();
                        return;
                    }
                    Some("_top" | "_parent") => {
                        // Fall through to navigate current document
                        // (true parent/top navigation requires multi-process IPC).
                    }
                    Some(name) if !name.is_empty() && !name.starts_with('_') => {
                        // Named target: look for an iframe with matching name.
                        if let Some(iframe_entity) = super::iframe::find_iframe_by_name(state, name)
                        {
                            super::iframe::navigate_iframe(state, iframe_entity, &target_url);
                            state.re_render();
                            state.send_display_list();
                            return;
                        }
                        // No matching iframe → fall through to normal navigation.
                    }
                    _ => {
                        // _self or no target → navigate current document.
                    }
                }
                state.send_display_list();
                handle_navigate(state, &target_url, false, None);
                return;
            }
        }
    }

    state.send_display_list();
}

/// Handle mouse button release — clear `:active` state.
///
/// Per UI Events spec, `:active` applies from mousedown to mouseup.
pub(super) fn handle_mouse_release(state: &mut ContentState) {
    if state.active_chain.is_empty() {
        return;
    }
    clear_active_chain(state);
    state.re_render();
    state.send_display_list();
}

pub(super) fn handle_mouse_move(state: &mut ContentState, point: Point) {
    let new_chain = if point.x >= 0.0 && point.y >= 0.0 {
        hit_test_with_scroll(
            &state.pipeline.dom,
            &HitTestQuery {
                point,
                scroll: state.viewport_scroll.scroll_offset,
            },
        )
        .map(|hit| collect_hover_chain(&state.pipeline.dom, hit.entity))
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    if new_chain == state.hover_chain {
        return;
    }

    let old_chain = std::mem::take(&mut state.hover_chain);
    apply_hover_diff(&mut state.pipeline.dom, &old_chain, &new_chain);
    state.hover_chain = new_chain;

    state.re_render();
    state.send_display_list();
}

pub(super) fn handle_cursor_left(state: &mut ContentState) {
    let had_hover = !state.hover_chain.is_empty();
    let had_active = !state.active_chain.is_empty();

    clear_active_chain(state);
    for &e in &std::mem::take(&mut state.hover_chain) {
        if state.pipeline.dom.contains(e) {
            update_element_state(&mut state.pipeline.dom, e, |s| {
                s.remove(DomElementState::HOVER);
                s.remove(DomElementState::ACTIVE);
            });
        }
    }

    if had_hover || had_active {
        state.re_render();
        state.send_display_list();
    }
}

pub(super) fn handle_key(
    state: &mut ContentState,
    event_type: &str,
    key: &str,
    code: &str,
    repeat: bool,
    mods: ModifierState,
) {
    // Route keyboard events to focused iframe if applicable.
    if try_route_key_to_iframe(state, event_type, key, code, repeat, mods) {
        state.re_render();
        state.send_display_list();
        return;
    }

    // The focused element (from the canonical FOCUS bit); `current_focus`
    // filters connectedness, so a despawned/detached prior target is
    // naturally absent — no stale-target field to clear.
    let Some(target) = current_focus(&state.pipeline.dom, state.pipeline.document) else {
        return;
    };

    let init = KeyboardEventInit {
        key: key.to_string(),
        code: code.to_string(),
        repeat,
        alt_key: mods.alt,
        ctrl_key: mods.ctrl,
        meta_key: mods.meta,
        shift_key: mods.shift,
    };

    let mut event = DispatchEvent::new_composed(event_type, target);
    event.payload = EventPayload::Keyboard(init);

    let default_prevented = state.pipeline.dispatch_event(&mut event);

    // Tab/Shift+Tab: move focus to next/previous focusable element.
    if event_type == "keydown" && !default_prevented && key == "Tab" {
        let forward = !mods.shift;
        if let Some(next) = find_next_focusable(state, forward) {
            // When `next` is an in-process `<iframe>`, §6.6.3 sequential focus
            // navigation should descend into the frame's own focus scope (the
            // flattened tabindex-ordered navigation order treats the frame's
            // focusable areas as part of this document's sequence): set
            // `state.focused_iframe` + route focus into the child pipeline the
            // way the click path does (`try_route_click_to_iframe`), and on the
            // way out resume the parent sequence. The Tab handler only runs the
            // parent `set_focus` today, so keyboard-only users cannot enter or traverse
            // iframe content — deferred to slot
            // `#11-cross-frame-sequential-focus-nav` (a discrete keyboard-nav
            // feature: enter / traverse-within / exit the frame boundary; gated
            // on the in-process-iframe test harness, `#11-iframe-focus-test-infra`).
            set_focus(&mut state.pipeline, next);
        }
        state.re_render();
        state.send_display_list();
        return;
    }

    // If keydown was not prevented, process form control input.
    if event_type == "keydown" && !default_prevented {
        let control_info = state
            .pipeline
            .dom
            .world()
            .get::<&FormControlState>(target)
            .ok()
            .filter(|fcs| !fcs.disabled)
            .map(|fcs| fcs.kind);

        if control_info.is_some_and(FormControlKind::is_text_control) {
            handle_key_text(state, target, key, code, mods);
        } else {
            handle_key_widget(state, target, key, control_info);
        }

        // Update scroll_offset_x to keep caret visible in single-line text inputs (M-12).
        update_scroll_offset(state, target);
    }

    state.re_render();

    if !process_pending_actions(state) {
        state.send_display_list();
    }
}

/// Update `scroll_offset_x` so the caret stays within the visible content area.
///
/// Uses `font_size * 0.6` as an approximation of average character width.
/// This is a rough estimate; accurate measurement would require text shaping
/// per keystroke which is expensive. Acceptable for Phase 4 caret tracking.
fn update_scroll_offset(state: &mut ContentState, target: elidex_ecs::Entity) {
    use elidex_plugin::{ComputedStyle, LayoutBox};

    // Only applies to single-line text controls.
    let info = {
        let w = state.pipeline.dom.world();
        w.get::<&LayoutBox>(target).ok().and_then(|lb| {
            let content_w = lb.content.size.width;
            w.get::<&ComputedStyle>(target)
                .ok()
                .map(|cs| (content_w, cs.font_size))
        })
    };

    if let (Some((content_w, font_size)), Ok(mut fcs)) = (
        info,
        state
            .pipeline
            .dom
            .world_mut()
            .get::<&mut FormControlState>(target),
    ) {
        if !fcs.kind.is_single_line_text() {
            return;
        }
        // Estimate caret x by counting chars up to cursor_pos.
        // Approximate average char width as font_size * 0.6 (P8/R13).
        let caret_pos = fcs.safe_cursor_pos();
        let before_cursor = &fcs.value()[..caret_pos];
        let char_width = font_size * 0.6;
        #[allow(clippy::cast_precision_loss)]
        let estimated_caret_x = (before_cursor.chars().count() as f32) * char_width;

        // If caret is past the right edge, scroll right.
        if estimated_caret_x > fcs.scroll_offset_x + content_w {
            fcs.scroll_offset_x = estimated_caret_x - content_w;
        }
        // If caret is before the left edge, scroll left.
        if estimated_caret_x < fcs.scroll_offset_x {
            fcs.scroll_offset_x = estimated_caret_x;
        }
    }
}

/// Find the next/previous focusable element for Tab navigation.
///
/// Per HTML §6.6.3: elements with positive tabindex come first (in tabindex
/// order, then DOM order), followed by tabindex=0 elements in DOM order.
/// Uses a cached focusable list; cache is invalidated on DOM changes.
fn find_next_focusable(state: &mut ContentState, forward: bool) -> Option<elidex_ecs::Entity> {
    use elidex_ecs::Entity;

    // Build cache if not present.
    if state.focusable_cache.is_none() {
        let mut raw: Vec<(Entity, i32)> = Vec::new();
        collect_focusable_entities(&state.pipeline.dom, state.pipeline.document, &mut raw, 0);
        // Stable sort: positive tabindex first (ascending), then tabindex=0.
        // Within the same tabindex, DOM order is preserved (stable sort).
        raw.sort_by_key(|&(_, ti)| if ti > 0 { (0, ti) } else { (1, 0) });
        let focusables: Vec<Entity> = raw.into_iter().map(|(e, _)| e).collect();
        state.focusable_cache = Some(focusables);
    }

    let focusables = state.focusable_cache.as_ref().unwrap();
    if focusables.is_empty() {
        return None;
    }

    let current_idx = current_focus(&state.pipeline.dom, state.pipeline.document)
        .and_then(|target| focusables.iter().position(|&e| e == target));

    match current_idx {
        Some(idx) => {
            let next = if forward {
                (idx + 1) % focusables.len()
            } else {
                (idx + focusables.len() - 1) % focusables.len()
            };
            Some(focusables[next])
        }
        None => Some(focusables[0]),
    }
}

/// Recursively collect focusable entities in pre-order.
///
/// Per HTML §6.6.3: elements with negative tabindex are focusable but not
/// in the sequential focus navigation order (Tab key).
/// Elements with `contenteditable` are also focusable.
fn collect_focusable_entities(
    dom: &elidex_ecs::EcsDom,
    entity: elidex_ecs::Entity,
    result: &mut Vec<(elidex_ecs::Entity, i32)>,
    depth: usize,
) {
    if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
        return;
    }

    if super::focus::is_focusable(dom, entity) {
        // §6.6.3 "rules for parsing integers" via the shared tabindex parser, so
        // the Tab-order collector agrees with `is_focusable` and the `tabIndex`
        // getter instead of running a second `str::parse::<i32>()` path: a
        // leading-integer value like `tabindex="2foo"` sorts as order 2, and
        // `tabindex="-1foo"` is correctly excluded from Tab order. A null/absent
        // value defaults to 0 (the per-element default already gated focusable).
        let tabindex = dom
            .world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .and_then(|a| {
                a.get("tabindex")
                    .and_then(elidex_dom_api::focus::parse_tab_index_value)
            })
            .unwrap_or(0);
        // Negative tabindex: focusable but not in Tab order.
        if tabindex >= 0 {
            result.push((entity, tabindex));
        }
    }
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        collect_focusable_entities(dom, c, result, depth + 1);
        child = dom.get_next_sibling(c);
    }
}

/// Handle keyboard input for text-like controls (text, password, textarea, email, etc.).
fn handle_key_text(
    state: &mut ContentState,
    target: elidex_ecs::Entity,
    key: &str,
    code: &str,
    mods: ModifierState,
) {
    // Check for Ctrl/Cmd+A (select all).
    let is_select_all = (mods.ctrl || mods.meta) && matches!(key, "a" | "A");

    // Check for Ctrl/Cmd+C/X/V (clipboard).
    let is_clipboard = (mods.ctrl || mods.meta) && matches!(key, "c" | "C" | "x" | "X" | "v" | "V");

    if is_select_all {
        if let Ok(mut fcs) = state
            .pipeline
            .dom
            .world_mut()
            .get::<&mut FormControlState>(target)
        {
            elidex_form::select_all(&mut fcs);
        }
        state.reset_caret_blink();
    } else if is_clipboard {
        handle_clipboard(state, target, key);
    } else if mods.shift && (key == "ArrowLeft" || key == "ArrowRight") {
        // Shift+Arrow extends selection.
        if let Ok(mut fcs) = state
            .pipeline
            .dom
            .world_mut()
            .get::<&mut FormControlState>(target)
        {
            if fcs.selection_start() == fcs.selection_end() {
                let pos = fcs.cursor_pos();
                fcs.set_selection(pos, pos);
            }
            elidex_form::extend_selection(&mut fcs, key == "ArrowRight");
        }
        state.reset_caret_blink();
    } else {
        let action = state
            .pipeline
            .dom
            .world_mut()
            .get::<&mut FormControlState>(target)
            .ok()
            .map_or(KeyAction::None, |mut fcs| {
                elidex_form::form_control_key_input_action(&mut fcs, key, code)
            });

        match action {
            KeyAction::Consumed => {
                // Determine input type for InputEvent.
                let input_type = match key {
                    "Backspace" => "deleteContentBackward",
                    "Delete" => "deleteContentForward",
                    "Enter" => "insertLineBreak",
                    _ if key.chars().count() == 1 => "insertText",
                    _ => "",
                };
                let data = if input_type == "insertText" {
                    Some(key)
                } else {
                    None
                };
                dispatch_input_event_typed(state, target, input_type, data);
                state.reset_caret_blink();
            }
            KeyAction::Submit => {
                // Implicit form submission.
                handle_form_submit(state, target);
            }
            KeyAction::None => {}
        }
    }
}

/// Handle keyboard input for non-text widget controls (checkbox, radio, select).
fn handle_key_widget(
    state: &mut ContentState,
    target: elidex_ecs::Entity,
    key: &str,
    control_info: Option<FormControlKind>,
) {
    match control_info {
        Some(FormControlKind::Checkbox)
            if key == " " && toggle_checkbox_if_needed(&mut state.pipeline.dom, target) =>
        {
            // S15: Checkbox Space fires a synthetic click event.
            let mut click_event = DispatchEvent::new_composed("click", target);
            click_event.payload = EventPayload::Mouse(MouseEventInit::default());
            state.pipeline.dispatch_event(&mut click_event);
            dispatch_state_change_events(state, target);
        }
        Some(FormControlKind::Radio) => {
            if key == " " {
                if elidex_form::toggle_radio(
                    &mut state.pipeline.dom,
                    target,
                    &mut state.pipeline.ancestor_cache,
                ) {
                    dispatch_input_event(state, target);
                }
            } else if key == "ArrowDown" || key == "ArrowRight" {
                toggle_radio_with_events(state, target, true);
            } else if key == "ArrowUp" || key == "ArrowLeft" {
                toggle_radio_with_events(state, target, false);
            }
        }
        Some(FormControlKind::Select) => {
            if key == "ArrowDown" || key == "ArrowUp" {
                let changed = state
                    .pipeline
                    .dom
                    .world_mut()
                    .get::<&mut FormControlState>(target)
                    .ok()
                    .is_some_and(|mut fcs| {
                        elidex_form::navigate_select(&mut fcs, key == "ArrowDown")
                    });
                if changed {
                    dispatch_input_event(state, target);
                }
            } else if key == "Enter" || key == "Escape" || key == " " {
                // Toggle dropdown open/close.
                if let Ok(mut fcs) = state
                    .pipeline
                    .dom
                    .world_mut()
                    .get::<&mut FormControlState>(target)
                {
                    fcs.dropdown_open = !fcs.dropdown_open;
                }
            }
        }
        _ => {}
    }
}

/// Toggle a radio button via arrow navigation and dispatch input + change events.
fn toggle_radio_with_events(state: &mut ContentState, current: elidex_ecs::Entity, forward: bool) {
    if let Some(next) = elidex_form::radio::radio_arrow_navigate(
        &state.pipeline.dom,
        current,
        forward,
        &mut state.pipeline.ancestor_cache,
    ) {
        set_focus(&mut state.pipeline, next);
        if elidex_form::toggle_radio(
            &mut state.pipeline.dom,
            next,
            &mut state.pipeline.ancestor_cache,
        ) {
            dispatch_state_change_events(state, next);
        }
    }
}

/// Set text into the system clipboard.
fn set_clipboard_text(text: &str) {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            if let Err(e) = clipboard.set_text(text) {
                tracing::warn!(error = %e, "clipboard: copy/cut failed to set text");
            }
        }
        Err(e) => tracing::warn!(error = %e, op = "copy", "clipboard: init failed"),
    }
}

/// Get text from the system clipboard.
fn get_clipboard_text() -> Option<String> {
    match arboard::Clipboard::new() {
        Ok(mut c) => match c.get_text() {
            Ok(text) => Some(text),
            Err(e) => {
                tracing::warn!(error = %e, "clipboard: paste failed to get text");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, op = "paste", "clipboard: init failed");
            None
        }
    }
}

/// Handle clipboard operations (Ctrl/Cmd+C/X/V).
fn handle_clipboard(state: &mut ContentState, target: elidex_ecs::Entity, key: &str) {
    match key {
        "c" | "C" => {
            let text = state
                .pipeline
                .dom
                .world()
                .get::<&FormControlState>(target)
                .ok()
                .map(|fcs| elidex_form::clipboard_copy(&fcs))
                .unwrap_or_default();
            if !text.is_empty() {
                set_clipboard_text(&text);
            }
        }
        "x" | "X" => {
            let text = state
                .pipeline
                .dom
                .world_mut()
                .get::<&mut FormControlState>(target)
                .ok()
                .map(|mut fcs| elidex_form::clipboard_cut(&mut fcs))
                .unwrap_or_default();
            if !text.is_empty() {
                set_clipboard_text(&text);
                dispatch_input_event_typed(state, target, "deleteByCut", None);
                state.reset_caret_blink();
            }
        }
        "v" | "V" => {
            if let Some(text) = get_clipboard_text() {
                if let Ok(mut fcs) = state
                    .pipeline
                    .dom
                    .world_mut()
                    .get::<&mut FormControlState>(target)
                {
                    elidex_form::clipboard_paste(&mut fcs, &text);
                }
                dispatch_input_event_typed(state, target, "insertFromPaste", Some(&text));
                state.reset_caret_blink();
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Iframe event routing
// ---------------------------------------------------------------------------

/// Blur the focus held inside in-process iframe `entity` (its separate `EcsDom`
/// is unreachable from the parent and sibling-iframe focus writers). Called
/// whenever focus leaves an iframe — to a top-level element ([`handle_click`])
/// or to a sibling iframe ([`try_route_click_to_iframe`]) — so a frame the user
/// has left does not keep its `activeElement` / `:focus` / caret live. No-op for
/// an unknown or out-of-process iframe.
fn blur_iframe_focus(state: &mut ContentState, entity: elidex_ecs::Entity) {
    if let Some(super::iframe::IframeHandle::InProcess(iframe)) =
        state.iframes.get_mut(entity).map(|entry| &mut entry.handle)
    {
        super::focus::blur_current(&mut iframe.pipeline);
        iframe.needs_render = true;
    }
}

/// Reconcile the `focused_iframe` side field against the parent document's
/// canonical `FOCUS` bit after a JS turn. A parent-side script `HTMLElement.
/// focus()` (e.g. from a timer / `postMessage` handler) can move the parent's
/// focused area off the `<iframe>` element WITHOUT going through `handle_click`'s
/// `focused_iframe.take()` + `blur_iframe_focus`. When the canonical bit no
/// longer points at the focused iframe element, focus has left the frame — blur
/// the in-process child so it stops painting `:focus` / caret and clears its
/// `activeElement`, then drop the side field so key routing stops. OOP frames
/// need a cross-process blur message (slot `#11-oop-iframe-focus-lifecycle`;
/// `blur_iframe_focus` is a no-op for them, so only the side field is dropped).
pub(super) fn reconcile_focused_iframe(state: &mut ContentState) {
    let Some(iframe_entity) = state.focused_iframe else {
        return;
    };
    if current_focus(&state.pipeline.dom, state.pipeline.document) != Some(iframe_entity) {
        blur_iframe_focus(state, iframe_entity);
        state.focused_iframe = None;
    }
}

/// Check if a hit-test result landed on an `<iframe>` element that has a loaded
/// iframe context. Returns `true` if the event was routed to the iframe
/// (caller should skip normal dispatch).
///
/// Events do NOT bubble out of iframe boundaries (WHATWG HTML: iframe is an
/// event boundary).
pub(super) fn try_route_click_to_iframe(
    state: &mut ContentState,
    hit_entity: elidex_ecs::Entity,
    click: &crate::ipc::MouseClickEvent,
) -> bool {
    use super::iframe::{click_event_types, mouse_event_init_from_click};
    use super::iframe::{BrowserToIframe, IframeHandle};

    if state.iframes.get(hit_entity).is_none() {
        return false;
    }

    let offset = state
        .pipeline
        .dom
        .world()
        .get::<&elidex_plugin::LayoutBox>(hit_entity)
        .ok()
        .map(|lb| lb.content.origin)
        .unwrap_or_default();

    let local_click = crate::ipc::MouseClickEvent {
        point: elidex_plugin::Point::new(click.point.x - offset.x, click.point.y - offset.y),
        client_point: elidex_plugin::Point::new(
            click.client_point.x - f64::from(offset.x),
            click.client_point.y - f64::from(offset.y),
        ),
        button: click.button,
        mods: click.mods,
    };

    // Focus is moving into the iframe's browsing context, so in the PARENT
    // document the focused area becomes the `<iframe>` element itself (WHATWG
    // HTML §6.6: a navigable container is the parent's focusable area / DOM
    // anchor while focus is inside the nested context). Focus the `<iframe>`
    // element (`hit_entity`) in the parent — `set_focus` first blurs+change-on-
    // blurs the parent's previous focus, then designates the iframe element so
    // parent scripts read `document.activeElement === iframeEl` and
    // `hasFocus() === true` (both routed via `current_focus`) instead of falling
    // back to `<body>`/false while keyboard focus is actually inside the frame.
    // Each pipeline owns its own `EcsDom`, so the iframe-side `set_focus` below
    // cannot reach the parent `FOCUS` bit. Runs for both in-process and OOP
    // iframes (the `<iframe>` element is default-focusable as a navigable
    // container).
    super::focus::set_focus(&mut state.pipeline, hit_entity);
    // ...and any *other* iframe that previously held focus — a sibling
    // iframe-to-iframe click. Each frame owns a separate `EcsDom`, so the
    // iframe-side `set_focus` below reaches none of the others; without this the
    // old iframe keeps its `activeElement` / `:focus` / caret live, leaving two
    // iframes focused at once. (Re-clicking the same iframe is `prev ==
    // hit_entity` — nothing to blur.)
    if let Some(prev) = state.focused_iframe {
        if prev != hit_entity {
            blur_iframe_focus(state, prev);
        }
    }

    let Some(entry) = state.iframes.get_mut(hit_entity) else {
        return false;
    };

    match &mut entry.handle {
        IframeHandle::InProcess(iframe) => {
            let iframe_query = elidex_layout::HitTestQuery {
                point: local_click.point,
                scroll: iframe.scroll_state.scroll_offset,
            };
            if let Some(iframe_hit) =
                elidex_layout::hit_test_with_scroll(&iframe.pipeline.dom, &iframe_query)
            {
                // Move focus within the iframe through the same reconciler the
                // top-level document uses (operating on the iframe's own
                // `PipelineResult`) — one focusing-steps path for every
                // document, and the FOCUS bit `try_route_key_to_iframe` reads.
                super::focus::set_focus(&mut iframe.pipeline, iframe_hit.entity);
                // Shared helpers for MouseEventInit + event type selection (B3/B4 fix).
                let mouse_init = mouse_event_init_from_click(&local_click);
                for &event_type in click_event_types(local_click.button) {
                    let mut event = elidex_script_session::DispatchEvent::new_composed(
                        event_type,
                        iframe_hit.entity,
                    );
                    if event_type == "auxclick" {
                        event.cancelable = false;
                    }
                    event.payload = elidex_plugin::EventPayload::Mouse(mouse_init.clone());
                    iframe.pipeline.dispatch_event(&mut event);
                }
            }
            iframe.needs_render = true;
        }
        IframeHandle::OutOfProcess(oop) => {
            let _ = oop.channel.send(BrowserToIframe::MouseClick(local_click));
        }
    }

    state.focused_iframe = Some(hit_entity);
    true
}

/// Route keyboard events to the focused iframe. Returns `true` if routed.
pub(super) fn try_route_key_to_iframe(
    state: &mut ContentState,
    event_type: &str,
    key: &str,
    code: &str,
    repeat: bool,
    mods: ModifierState,
) -> bool {
    use super::iframe::{BrowserToIframe, IframeHandle};

    let Some(iframe_entity) = state.focused_iframe else {
        return false;
    };
    // Gate on the parent document's canonical `FOCUS` bit, not just this side
    // field: a parent-side `HTMLElement.focus()` (e.g. from a timer / postMessage
    // handler) can move the parent's focused area off the `<iframe>` element
    // without touching `state.focused_iframe`, after which the key belongs to the
    // now-focused parent control. `set_focus` keeps the parent `FOCUS` bit ON the
    // `<iframe>` element while focus is inside it, so
    // `current_focus(parent) != iframe_entity` means focus has left the frame —
    // let the parent handle the key. The side field is left intact so the next
    // top-level click's `focused_iframe.take()` + `blur_iframe_focus` still runs
    // the iframe's deferred blur.
    if current_focus(&state.pipeline.dom, state.pipeline.document) != Some(iframe_entity) {
        return false;
    }
    let Some(entry) = state.iframes.get_mut(iframe_entity) else {
        state.focused_iframe = None;
        return false;
    };

    match &mut entry.handle {
        IframeHandle::InProcess(iframe) => {
            // The iframe's focused element, read from the canonical FOCUS bit
            // in its own `EcsDom` (connectedness-filtered by `current_focus`).
            if let Some(target) = current_focus(&iframe.pipeline.dom, iframe.pipeline.document) {
                let init = elidex_plugin::KeyboardEventInit {
                    key: key.to_string(),
                    code: code.to_string(),
                    repeat,
                    alt_key: mods.alt,
                    ctrl_key: mods.ctrl,
                    meta_key: mods.meta,
                    shift_key: mods.shift,
                };
                let mut event =
                    elidex_script_session::DispatchEvent::new_composed(event_type, target);
                event.payload = elidex_plugin::EventPayload::Keyboard(init);
                iframe.pipeline.dispatch_event(&mut event);
                iframe.needs_render = true;
            }
        }
        IframeHandle::OutOfProcess(oop) => {
            // Forward both keydown and keyup to OOP iframes (B5 fix).
            let _ = oop.channel.send(BrowserToIframe::KeyEvent {
                event_type: event_type.to_string(),
                key: key.to_string(),
                code: code.to_string(),
                repeat,
                mods,
            });
        }
    }

    true
}
