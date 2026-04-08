//! Iframe rendering: display list management for parent compositing.

use super::types::IframeHandle;

/// Set the `IframeDisplayList` component on an entity in the parent DOM.
///
/// Handles the hecs quirk where `insert_one` fails if the component already
/// exists — removes first, then inserts.
pub(in crate::content) fn set_iframe_display_list(
    dom: &mut elidex_ecs::EcsDom,
    entity: elidex_ecs::Entity,
    display_list: std::sync::Arc<elidex_render::DisplayList>,
) {
    let _ = dom
        .world_mut()
        .remove_one::<elidex_render::IframeDisplayList>(entity);
    let _ = dom
        .world_mut()
        .insert_one(entity, elidex_render::IframeDisplayList(display_list));
}

/// Re-render all in-process iframes that need it.
///
/// Iterates registered iframes and calls `crate::re_render()` on each
/// in-process iframe whose `needs_render` flag is set. Updates the
/// `IframeDisplayList` component on the parent DOM for compositing.
pub(in crate::content) fn re_render_all_iframes(state: &mut crate::content::ContentState) {
    let mut updated: Vec<(
        elidex_ecs::Entity,
        std::sync::Arc<elidex_render::DisplayList>,
    )> = Vec::new();

    for (&entity, entry) in state.iframes.iter_mut() {
        if let IframeHandle::InProcess(ref mut ip) = entry.handle {
            if ip.needs_render {
                crate::re_render(&mut ip.pipeline);
                ip.needs_render = false;
                let arc_dl = std::sync::Arc::new(ip.pipeline.display_list.clone());
                ip.cached_display_list = Some(std::sync::Arc::clone(&arc_dl));
                updated.push((entity, arc_dl));
            }
        }
    }

    for (entity, dl) in updated {
        set_iframe_display_list(&mut state.pipeline.dom, entity, dl);
    }
}

/// Drain timers for in-process (same-origin) iframes.
///
/// Timers always run (for correctness), but layout/render is deferred
/// to the next `re_render_all_iframes` call via the `needs_render` flag.
pub(in crate::content) fn tick_iframe_timers(state: &mut crate::content::ContentState) {
    let now = std::time::Instant::now();
    for (_, entry) in state.iframes.iter_mut() {
        if let IframeHandle::InProcess(ref mut ip) = entry.handle {
            if ip
                .pipeline
                .runtime
                .next_timer_deadline()
                .is_some_and(|d| d <= now)
            {
                ip.pipeline.drain_timers();
                ip.needs_render = true;
            }
        }
    }
}
