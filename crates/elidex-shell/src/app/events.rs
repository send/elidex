//! Mouse and keyboard event handling.

use elidex_ecs::{
    Attributes, ElementState as DomElementState, Entity, TagType, MAX_ANCESTOR_DEPTH,
};
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit};
use elidex_script_session::DispatchEvent;
use winit::event::MouseButton;

use super::hover::update_element_state;
use super::App;

impl App {
    /// Handle a mouse click event.
    ///
    /// Phase 2 simplification: mousedown, mouseup, and click are all
    /// dispatched synchronously on button press. Per DOM spec, mouseup
    /// should fire on button *release* (which may target a different
    /// element if the cursor moved), and click should only fire if
    /// press and release hit the same element.
    // TODO(Phase 3): split into handle_mouse_down / handle_mouse_up,
    // track press target per button, and dispatch mouseup on release.
    pub(super) fn handle_click(&mut self, button: MouseButton) {
        // Dispatch events and re-render. Capture values needed after the
        // mutable borrow of `self.interactive` is released.
        let (button_num, click_prevented, hit_entity) = {
            let Some(interactive) = &mut self.interactive else {
                return;
            };
            let Some((cx, cy)) = interactive.cursor_pos else {
                return;
            };
            #[allow(clippy::cast_possible_truncation)]
            let x = cx as f32;
            // Offset Y by chrome bar height so hit testing is relative to content.
            #[allow(clippy::cast_possible_truncation)]
            let y = (cy as f32) - crate::chrome::CHROME_HEIGHT;
            if y < 0.0 {
                return; // Click is within the chrome bar.
            }

            let pipeline = &mut interactive.pipeline;
            let Some(hit) = elidex_layout::hit_test(&pipeline.dom, x, y) else {
                return;
            };
            let hit_entity = hit.entity;

            // Update focus: remove FOCUS from old target, set on new.
            if interactive.focus_target != Some(hit_entity) {
                if let Some(old_focus) = interactive.focus_target {
                    update_element_state(&mut pipeline.dom, old_focus, |s| {
                        s.remove(DomElementState::FOCUS);
                    });
                }
                update_element_state(&mut pipeline.dom, hit_entity, |s| {
                    s.insert(DomElementState::FOCUS);
                });
                interactive.focus_target = Some(hit_entity);
            }

            // DOM spec: 0=primary, 1=auxiliary, 2=secondary, 3=back, 4=forward.
            let button_num = match button {
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                MouseButton::Back => 3,
                MouseButton::Forward => 4,
                MouseButton::Left | MouseButton::Other(_) => 0,
            };

            let mods = interactive.modifiers.state();
            let mouse_init = MouseEventInit {
                client_x: cx,
                client_y: cy,
                button: button_num,
                alt_key: mods.alt_key(),
                ctrl_key: mods.control_key(),
                meta_key: mods.super_key(),
                shift_key: mods.shift_key(),
                ..Default::default()
            };

            // Dispatch mousedown, mouseup, and (for primary button only) click.
            // DOM spec: click fires only for the primary button (button 0).
            // TODO(Phase 3): dispatch auxclick for non-primary buttons.
            let event_types: &[&str] = if button_num == 0 {
                &["mousedown", "mouseup", "click"]
            } else {
                &["mousedown", "mouseup"]
            };
            let mut click_prevented = false;
            for event_type in event_types {
                let mut event = DispatchEvent::new(*event_type, hit_entity);
                event.payload = EventPayload::Mouse(mouse_init.clone());
                let prevented = pipeline.runtime.dispatch_event(
                    &mut event,
                    &mut pipeline.session,
                    &mut pipeline.dom,
                    pipeline.document,
                );
                if *event_type == "click" {
                    click_prevented = prevented;
                }
            }

            // Re-render after event handling.
            crate::re_render(pipeline);
            self.display_list = pipeline.display_list.clone();

            (button_num, click_prevented, hit_entity)
        };

        // Process any pending JS navigation or history action.
        if self.process_pending_navigation() {
            return;
        }

        // Link navigation: if click was not prevented, check for <a href>.
        if button_num == 0 && !click_prevented {
            let nav_target = {
                let Some(interactive) = &self.interactive else {
                    return;
                };
                let pipeline = &interactive.pipeline;
                find_link_ancestor(&pipeline.dom, hit_entity).and_then(|href| {
                    if let Some(base_url) = &pipeline.url {
                        base_url.join(&href).ok()
                    } else {
                        url::Url::parse(&href).ok()
                    }
                })
            };
            if let Some(target_url) = nav_target {
                self.navigate(&target_url, false);
            }
        }
    }

    /// Handle a keyboard event.
    pub(super) fn handle_keyboard(&mut self, event_type: &str, init: KeyboardEventInit) {
        let Some(interactive) = &mut self.interactive else {
            return;
        };
        let Some(target) = interactive.focus_target else {
            return;
        };

        let pipeline = &mut interactive.pipeline;
        if !pipeline.dom.contains(target) {
            interactive.focus_target = None;
            return;
        }

        let mut event = DispatchEvent::new(event_type, target);
        event.payload = EventPayload::Keyboard(init);

        // TODO(Phase 3): Check default_prevented to suppress default keyboard actions.
        let _default_prevented = pipeline.runtime.dispatch_event(
            &mut event,
            &mut pipeline.session,
            &mut pipeline.dom,
            pipeline.document,
        );

        crate::re_render(pipeline);
        self.display_list = pipeline.display_list.clone();

        // Process any pending JS navigation or history action.
        self.process_pending_navigation();
    }
}

/// Find the nearest `<a href="...">` ancestor of an entity (including itself).
///
/// Depth-limited to [`MAX_ANCESTOR_DEPTH`] to guard against cycles (consistent with
/// `build_propagation_path` and other tree walkers in the codebase).
pub(super) fn find_link_ancestor(dom: &elidex_ecs::EcsDom, entity: Entity) -> Option<String> {
    let mut current = Some(entity);
    let mut depth = 0;
    while let Some(e) = current {
        if depth > MAX_ANCESTOR_DEPTH {
            break;
        }
        if let Ok(tag) = dom.world().get::<&TagType>(e) {
            if tag.0 == "a" {
                if let Ok(attrs) = dom.world().get::<&Attributes>(e) {
                    if let Some(href) = attrs.get("href") {
                        if !href.is_empty() {
                            return Some(href.to_string());
                        }
                    }
                }
            }
        }
        current = dom.get_parent(e);
        depth += 1;
    }
    None
}
