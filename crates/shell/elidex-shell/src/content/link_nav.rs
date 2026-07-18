//! The `<a href>` **default-navigation** path for a primary-button click — the
//! WHATWG HTML §4.6.5 Following hyperlinks / §7.4.2.4 link-activation resolution
//! (target attribute §4.6.2 + `_blank` / `_top` / `_parent` / named-frame sandbox
//! gates [§7.1.5 Sandboxing `allow-popups` / §7.4.2.4 allowed-to-navigate] + navigate), carved
//! out of [`super::event_handlers::handle_click`] as a cohesive unit distinct
//! from event dispatch (touch-time split, CLAUDE.md "1000-line debt = touch-time
//! split").

use elidex_ecs::Entity;
use elidex_script_session::HostDriver;

use crate::app::navigation::resolve_nav_url;

use super::navigation::{handle_navigate, HistoryCursorOp};
use super::ContentState;

/// Perform an `<a href>` element's **default navigation** for a click that the
/// Phase-1 drain did NOT suppress: resolve the hit entity's link-ancestor target,
/// apply the `_blank` / `_top` / `_parent` / named-frame sandbox gates, and
/// navigate — or, when there is no eligible link, ship the current frame.
///
/// `eligible` is the caller's `click.button == 0 && !click_prevented`: a
/// non-primary button (auxclick) or a `preventDefault()`'d primary click skips
/// the default action and simply ships the re-rendered frame. Every navigate path
/// ships its own display list before returning, so this is the sole owner of the
/// click turn's post-drain frame shipping (its callers do not ship again).
pub(super) fn perform_link_default_navigation(
    state: &mut ContentState,
    hit_entity: Entity,
    eligible: bool,
) {
    // Link navigation: if click was not prevented, check for <a href>.
    if eligible {
        if let Some((href, target_attr)) =
            crate::app::events::find_link_ancestor_with_target(&state.pipeline.dom, hit_entity)
        {
            let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &href);
            if let Some(target_url) = resolved {
                match target_attr.as_deref() {
                    Some("_blank") => {
                        // Sandbox allow-popups check (WHATWG HTML §7.1.5 Sandboxing,
                        // `allow-popups` keyword / sandboxed auxiliary navigation
                        // browsing context flag):
                        // block popup navigation from sandboxed iframes without
                        // the allow-popups flag.
                        if !state.pipeline.runtime.popups_allowed() {
                            state.send_display_list();
                            return;
                        }
                        // Open in a new tab.
                        state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(target_url));
                        state.send_display_list();
                        return;
                    }
                    Some("_top" | "_parent")
                        if !elidex_plugin::sandbox::top_navigation_allowed(
                            state.pipeline.runtime.sandbox_flags(),
                            true,
                        ) =>
                    {
                        // Sandbox top-navigation check (WHATWG HTML §7.4.2.4
                        // *allowed by sandboxing to navigate* steps 3.2/3.3):
                        // block navigation to parent/top from a sandboxed iframe
                        // lacking a top-navigation grant. A link CLICK is a user
                        // gesture, so `activation = true` is the statically-
                        // correct per-call-site truth here (memo §4.3.3) — this
                        // is what makes `allow-top-navigation-by-user-activation`
                        // permit the navigation while `window.open` (activation
                        // = false) does not. Real transient-activation tracking
                        // is carved (`#11-transient-activation-tracking`).
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
                handle_navigate(state, &target_url, HistoryCursorOp::Push, None);
                return;
            }
        }
    }

    state.send_display_list();
}
