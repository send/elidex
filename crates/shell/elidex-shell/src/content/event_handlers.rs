//! Input event handlers: click, mouse move/release, cursor leave, keyboard.

use elidex_ecs::ElementState as DomElementState;
use elidex_form::{FormControlKind, FormControlState, KeyAction};
use elidex_layout::{hit_test_with_scroll, HitTestQuery};
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit, Point};
use elidex_script_session::DispatchEvent;

use crate::app::hover::{apply_hover_diff, collect_hover_chain, update_element_state};
use crate::app::navigation::resolve_nav_url;
use crate::ipc::ModifierState;

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

    // Update focus.
    set_focus(state, hit_entity);

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
        let prevented = state.pipeline.runtime.dispatch_event(
            &mut event,
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );
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
            Some(FormControlKind::Radio) => {
                if elidex_form::toggle_radio(
                    &mut state.pipeline.dom,
                    hit_entity,
                    &mut state.pipeline.ancestor_cache,
                ) {
                    dispatch_state_change_events(state, hit_entity);
                }
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
                    Some("_top" | "_parent") => {
                        // Sandbox allow-top-navigation check (WHATWG HTML §4.8.5):
                        // block navigation to parent/top from sandboxed iframes
                        // without the allow-top-navigation flag.
                        if state
                            .pipeline
                            .runtime
                            .bridge()
                            .sandbox_flags()
                            .is_some_and(|f| {
                                !f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_TOP_NAVIGATION)
                            })
                        {
                            state.send_display_list();
                            return;
                        }
                        // Fall through to navigate current document
                        // (true parent/top navigation requires multi-process IPC).
                    }
                    Some(name) if !name.is_empty() && !name.starts_with('_') => {
                        // Named target: look for an iframe with matching name.
                        if let Some(iframe_entity) = find_iframe_by_name(state, name) {
                            navigate_iframe(state, iframe_entity, &target_url);
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
    let Some(target) = state.focus_target else {
        return;
    };
    if !state.pipeline.dom.contains(target) {
        state.focus_target = None;
        return;
    }

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

    let default_prevented = state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );

    // Tab/Shift+Tab: move focus to next/previous focusable element.
    if event_type == "keydown" && !default_prevented && key == "Tab" {
        let forward = !mods.shift;
        if let Some(next) = find_next_focusable(state, forward) {
            set_focus(state, next);
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

    let current_idx = state
        .focus_target
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
        let tabindex = dom
            .world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .and_then(|a| a.get("tabindex").and_then(|v| v.parse::<i32>().ok()))
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
        Some(FormControlKind::Checkbox) if key == " " => {
            if toggle_checkbox_if_needed(&mut state.pipeline.dom, target) {
                // S15: Checkbox Space fires a synthetic click event.
                let mut click_event = DispatchEvent::new_composed("click", target);
                click_event.payload = EventPayload::Mouse(MouseEventInit::default());
                state.pipeline.runtime.dispatch_event(
                    &mut click_event,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                dispatch_state_change_events(state, target);
            }
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
        set_focus(state, next);
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

/// Find an iframe entity by its `name` attribute.
///
/// Searches the parent DOM for `<iframe>` elements whose `IframeData.name`
/// matches the given target name (WHATWG HTML §7.1.3).
fn find_iframe_by_name(state: &ContentState, name: &str) -> Option<elidex_ecs::Entity> {
    for (&entity, _entry) in state.iframes.iter() {
        let matches = state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::IframeData>(entity)
            .ok()
            .is_some_and(|d| d.name.as_deref() == Some(name));
        if matches {
            return Some(entity);
        }
    }
    None
}

/// Navigate an in-process iframe to a new URL.
///
/// Dispatches unload events on the old iframe, removes it, loads the new URL,
/// and inserts the new entry.
fn navigate_iframe(state: &mut ContentState, iframe_entity: elidex_ecs::Entity, url: &url::Url) {
    // Dispatch unload events on the old iframe before removing it (WHATWG HTML §7.1.3).
    if let Some(mut removed_entry) = state.iframes.remove(iframe_entity) {
        if let super::iframe::IframeHandle::InProcess(ref mut ip) = removed_entry.handle {
            crate::pipeline::dispatch_unload_events(
                &mut ip.pipeline.runtime,
                &mut ip.pipeline.session,
                &mut ip.pipeline.dom,
                ip.pipeline.document,
            );
        }
    }
    // Update IframeData.src and Attributes directly (no mutation record).
    // This is a programmatic navigation (link target / window.open), not a
    // JS setAttribute call. Recording a mutation would cause detect_iframe_mutations
    // to re-trigger loading on the next flush, resulting in a double load.
    let url_str = url.to_string();
    if let Ok(mut iframe_data) = state
        .pipeline
        .dom
        .world_mut()
        .get::<&mut elidex_ecs::IframeData>(iframe_entity)
    {
        iframe_data.src = Some(url_str.clone());
    }
    if let Ok(mut attrs) = state
        .pipeline
        .dom
        .world_mut()
        .get::<&mut elidex_ecs::Attributes>(iframe_entity)
    {
        attrs.set("src", &url_str);
    }
    super::iframe::try_load_iframe_entity(state, iframe_entity, true);
}

// ---------------------------------------------------------------------------
// Iframe event routing
// ---------------------------------------------------------------------------

/// Check if a hit-test result landed on an `<iframe>` element that has a loaded
/// iframe context. Returns `true` if the event was routed to the iframe
/// (caller should skip normal dispatch).
///
/// For same-origin (in-process) iframes, the click is transformed to iframe-local
/// coordinates and dispatched directly to the iframe's `JsRuntime`.
/// For cross-origin (out-of-process) iframes, the click is forwarded via IPC.
///
/// Events do NOT bubble out of iframe boundaries (WHATWG HTML: iframe is an
/// event boundary).
#[allow(dead_code)] // Used when iframe loading is implemented.
pub(super) fn try_route_click_to_iframe(
    state: &mut ContentState,
    hit_entity: elidex_ecs::Entity,
    click: &crate::ipc::MouseClickEvent,
) -> bool {
    use super::iframe::{BrowserToIframe, IframeHandle};

    // Check if the hit entity is an iframe with a loaded context.
    // Check iframe exists before computing offset.
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

    let local_point = elidex_plugin::Point::new(click.point.x - offset.x, click.point.y - offset.y);
    let local_client = elidex_plugin::Point::new(
        click.client_point.x - f64::from(offset.x),
        click.client_point.y - f64::from(offset.y),
    );

    let local_click = crate::ipc::MouseClickEvent {
        point: local_point,
        client_point: local_client,
        button: click.button,
        mods: click.mods,
    };

    // Re-borrow mutably for dispatch.
    let Some(entry) = state.iframes.get_mut(hit_entity) else {
        return false;
    };

    match &mut entry.handle {
        IframeHandle::InProcess(iframe) => {
            // Same-origin: run hit test in iframe's DOM and dispatch events.
            let iframe_query = elidex_layout::HitTestQuery {
                point: local_point,
                scroll: iframe.scroll_state.scroll_offset,
            };
            if let Some(iframe_hit) =
                elidex_layout::hit_test_with_scroll(&iframe.pipeline.dom, &iframe_query)
            {
                let mouse_init = elidex_plugin::MouseEventInit {
                    client_x: local_client.x,
                    client_y: local_client.y,
                    button: i16::from(local_click.button),
                    ..Default::default()
                };
                for event_type in ["mousedown", "mouseup", "click"] {
                    let mut event = elidex_script_session::DispatchEvent::new_composed(
                        event_type,
                        iframe_hit.entity,
                    );
                    event.payload = elidex_plugin::EventPayload::Mouse(mouse_init.clone());
                    iframe.pipeline.runtime.dispatch_event(
                        &mut event,
                        &mut iframe.pipeline.session,
                        &mut iframe.pipeline.dom,
                        iframe.pipeline.document,
                    );
                }
            }
            iframe.needs_render = true;
        }
        IframeHandle::OutOfProcess(oop) => {
            // Cross-origin: forward via IPC.
            let _ = oop.channel.send(BrowserToIframe::MouseClick(local_click));
        }
    }

    // Set focused_iframe so keyboard events go to this iframe.
    state.focused_iframe = Some(hit_entity);

    true
}

/// Check if keyboard events should be routed to a focused iframe.
/// Returns `true` if the event was routed.
#[allow(dead_code)] // Used when iframe loading is implemented.
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
    let Some(entry) = state.iframes.get_mut(iframe_entity) else {
        state.focused_iframe = None;
        return false;
    };

    match &mut entry.handle {
        IframeHandle::InProcess(iframe) => {
            // Same-origin: dispatch key event to iframe's runtime.
            if let Some(target) = iframe.focus_target {
                if iframe.pipeline.dom.contains(target) {
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
                    iframe.pipeline.runtime.dispatch_event(
                        &mut event,
                        &mut iframe.pipeline.session,
                        &mut iframe.pipeline.dom,
                        iframe.pipeline.document,
                    );
                    iframe.needs_render = true;
                }
            }
        }
        IframeHandle::OutOfProcess(oop) => {
            if event_type == "keydown" {
                let _ = oop.channel.send(BrowserToIframe::KeyDown {
                    key: key.to_string(),
                    code: code.to_string(),
                    repeat,
                    mods,
                });
            }
        }
    }

    true
}
