//! Iframe lifecycle: mutation detection, lazy loading, unloading, DOM scanning.

use elidex_ecs::Entity;
use elidex_script_session::HostDriver;

use super::load::load_iframe;
use super::render::set_iframe_display_list;
use super::types::{BrowserToIframe, IframeEntry, IframeHandle, IframeLoadContext};

/// Reconcile the iframe registry against the live document tree (§4.3.8).
///
/// Replaces the record-driven `detect_iframe_mutations`, which STARVES under the
/// VM flip: VM-native DOM mutations write the `EcsDom` immediately and never enter
/// `SessionCore::pending`, so `crate::re_render`'s flush produces an empty record
/// stream and the record-fed add/remove/re-nav detection never fires. Instead the
/// shell's version-delta (`ContentState::last_render_dom_version`) gates ONE full
/// document walk that diffs the connected `<iframe>` set against the registry,
/// idempotently reproducing all three behaviors of the old record scan:
///
/// - **ADD**: a connected `<iframe>` not yet registered → `try_load` (respecting
///   `loading="lazy"` — a lazy iframe defers to the pending queue rather than
///   force-loading, same as `scan_initial_iframes`).
/// - **REMOVE**: a registered (or lazy-pending) entity no longer reachable from
///   the document root (detached / removed) → unload + drop from the lazy queue.
/// - **CHANGE**: a registered iframe whose live `IframeData` `src` OR `srcdoc`
///   differs from what it was loaded with → re-navigate (HTML "process the iframe
///   attributes"; both `src` and `srcdoc` trigger, `srcdoc` taking precedence in
///   `load_iframe`).
///
/// Returns `true` iff a load/unload/re-nav happened (a parent display-list rebuild
/// is needed). Only runs when the caller has already observed a document-tree
/// change this turn, so a full walk is acceptable. `collect_iframe_entities` walks
/// from the document root, so it yields exactly the CONNECTED iframe entities —
/// connectedness (the old `is_connected` gate) is structural here: a detached
/// iframe simply is not in the set, so it neither loads (ADD) nor survives
/// (REMOVE), matching HTML §4.8.5 "content navigable on connection".
pub(in crate::content) fn rescan_iframes_by_diff(state: &mut crate::content::ContentState) -> bool {
    // 1. Walk the document for the current CONNECTED <iframe> entity set.
    let mut current = Vec::new();
    collect_iframe_entities(
        &state.pipeline.dom,
        state.pipeline.document,
        &mut current,
        0,
    );
    let current_set: std::collections::HashSet<Entity> = current.iter().copied().collect();
    let mut changed = false;

    // 2. REMOVE: registered entities no longer in the connected set (detached or
    //    removed → the walk can't reach them). Unload + drop any lazy-pending.
    let gone: Vec<Entity> = state
        .iframes
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !current_set.contains(e))
        .collect();
    for entity in gone {
        if let Some(entry) = state.iframes.remove(entity) {
            unload_iframe_entry(state, entity, entry);
        }
        state.iframes.remove_lazy_pending(entity);
        changed = true;
    }
    // Prune lazy-pending entities that vanished without ever registering (a lazy
    // iframe detached before it scrolled into view).
    let vanished_lazy: Vec<Entity> = state
        .iframes
        .lazy_pending_iter()
        .copied()
        .filter(|e| !current_set.contains(e))
        .collect();
    if !vanished_lazy.is_empty() {
        state.iframes.remove_lazy_pending_list(&vanished_lazy);
        changed = true;
    }

    // 3. For each current iframe: ADD (new) or CHANGE (src/srcdoc differs).
    for &entity in &current {
        let current_data = state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::IframeData>(entity)
            .ok()
            .map(|d| (*d).clone());
        let Some(current_data) = current_data else {
            continue;
        };
        match state.iframes.get(entity) {
            None => {
                // Not registered. Skip if already lazy-pending (avoid re-thrash):
                // its `IframeData` was re-derived at the attribute write, so a
                // later `check_lazy_iframes` load reads the fresh src/srcdoc.
                if state.iframes.is_lazy_pending(entity) {
                    continue;
                }
                // force=false: a `loading="lazy"` iframe defers to the pending
                // queue rather than force-loading (same as `scan_initial_iframes`
                // / the old ChildList path); a non-lazy iframe loads immediately.
                try_load_iframe_entity(state, entity, false);
                // Only a REAL load (not a lazy defer) rebuilds the display list.
                if state.iframes.get(entity).is_some() {
                    changed = true;
                }
            }
            Some(entry) => {
                // Registered → re-navigate iff live src OR srcdoc drifted from
                // what it loaded with (HTML "process the iframe attributes";
                // matches the old `name == "src" || "srcdoc"` Attribute arm).
                if entry.loaded_src != current_data.src
                    || entry.loaded_srcdoc != current_data.srcdoc
                {
                    state.iframes.remove_lazy_pending(entity);
                    if let Some(old) = state.iframes.remove(entity) {
                        unload_iframe_entry(state, entity, old);
                    }
                    // `IframeData` is already re-derived from the new src/srcdoc
                    // by the `set_attribute` / flush reconcile seam, so `try_load`
                    // reads it fresh (srcdoc-over-src precedence in `load_iframe`).
                    // force=false: a lazy iframe re-defers rather than force-loads.
                    try_load_iframe_entity(state, entity, false);
                    changed = true;
                }
            }
        }
    }
    changed
}

/// Unload a single iframe entry: dispatch lifecycle events, clean up ECS state,
/// and clear focus tracking.
pub(super) fn unload_iframe_entry(
    state: &mut crate::content::ContentState,
    entity: Entity,
    entry: IframeEntry,
) {
    unload_iframe_handle(entry.handle);
    let _ = state
        .pipeline
        .dom
        .world_mut()
        .remove_one::<elidex_render::IframeDisplayList>(entity);
    if state.focused_iframe == Some(entity) {
        state.focused_iframe = None;
    }
}

/// Dispatch unload events or send shutdown for an iframe handle.
///
/// Shared by `unload_iframe_entry` and `shutdown_all`.
pub(super) fn unload_iframe_handle(handle: IframeHandle) {
    match handle {
        IframeHandle::InProcess(mut ip) => {
            crate::pipeline::dispatch_unload_events(
                &mut ip.pipeline.runtime,
                &mut ip.pipeline.session,
                &mut ip.pipeline.dom,
                ip.pipeline.document,
            );
        }
        IframeHandle::OutOfProcess(mut oop) => {
            let _ = oop.channel.send(BrowserToIframe::Shutdown);
            if let Some(thread) = oop.thread.take() {
                if let Err(e) = thread.join() {
                    eprintln!("iframe thread panicked: {e:?}");
                }
            }
        }
    }
}

/// Try to load an iframe for `entity` if it has `IframeData`.
///
/// Respects `loading="lazy"`: lazy iframes are deferred and loaded when
/// `check_lazy_iframes` detects proximity to the viewport.
///
/// When `force` is `true`, the lazy check is bypassed (explicit navigation).
pub(in crate::content) fn try_load_iframe_entity(
    state: &mut crate::content::ContentState,
    entity: Entity,
    force: bool,
) {
    let iframe_data = state
        .pipeline
        .dom
        .world()
        .get::<&elidex_ecs::IframeData>(entity)
        .ok()
        .map(|d| (*d).clone());
    let Some(data) = iframe_data else { return };

    if !force && data.loading == elidex_ecs::LoadingAttribute::Lazy {
        state.iframes.add_lazy_pending(entity);
        return;
    }

    let parent_origin = state.pipeline.runtime.origin();
    let ctx = build_load_context(state, entity, &parent_origin);
    let entry = load_iframe(&data, &ctx);
    register_iframe_entry(state, entity, &data, entry);
}

/// Walk the DOM tree and load any `<iframe>` elements found during initial parse.
pub(in crate::content) fn scan_initial_iframes(state: &mut crate::content::ContentState) {
    let mut iframes_to_load = Vec::new();
    collect_iframe_entities(
        &state.pipeline.dom,
        state.pipeline.document,
        &mut iframes_to_load,
        0,
    );
    for entity in iframes_to_load {
        try_load_iframe_entity(state, entity, false);
    }
}

/// Check lazy iframes and load those near the viewport.
///
/// Uses `LayoutBox` position to determine if a lazy iframe is within 200px
/// of the viewport bounds.
pub(in crate::content) fn check_lazy_iframes(state: &mut crate::content::ContentState) -> bool {
    if !state.iframes.has_lazy_pending() {
        return false;
    }

    let vp_width = state.pipeline.viewport.width;
    let vp_height = state.pipeline.viewport.height;
    let scroll_x = state.viewport_scroll.scroll_offset.x;
    let scroll_y = state.viewport_scroll.scroll_offset.y;
    let margin = 200.0_f32;

    let visible_left = scroll_x - margin;
    let visible_right = scroll_x + vp_width + margin;
    let visible_top = scroll_y - margin;
    let visible_bottom = scroll_y + vp_height + margin;

    let to_load: Vec<Entity> = state
        .iframes
        .lazy_pending_iter()
        .copied()
        .filter(|&entity| {
            if state.iframes.get(entity).is_some() {
                return false;
            }
            state
                .pipeline
                .dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(entity)
                .ok()
                .is_some_and(|lb| {
                    let left = lb.content.origin.x;
                    let right = left + lb.content.size.width;
                    let top = lb.content.origin.y;
                    let bottom = top + lb.content.size.height;
                    right >= visible_left
                        && left <= visible_right
                        && bottom >= visible_top
                        && top <= visible_bottom
                })
        })
        .collect();

    if to_load.is_empty() {
        return false;
    }

    state.iframes.remove_lazy_pending_list(&to_load);

    for entity in to_load {
        let iframe_data = state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::IframeData>(entity)
            .ok()
            .map(|d| (*d).clone());
        if let Some(data) = iframe_data {
            let parent_origin = state.pipeline.runtime.origin();
            let ctx = build_load_context(state, entity, &parent_origin);
            let entry = load_iframe(&data, &ctx);
            register_iframe_entry(state, entity, &data, entry);
        }
    }
    true
}

/// Find an iframe entity by its `name` attribute.
///
/// Searches the parent DOM for `<iframe>` elements whose `IframeData.name`
/// matches the given target name (WHATWG HTML §7.1.3).
pub(in crate::content) fn find_iframe_by_name(
    state: &crate::content::ContentState,
    name: &str,
) -> Option<Entity> {
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

/// Navigate an iframe to a new URL.
///
/// Dispatches unload events on the old iframe, removes it, loads the new URL,
/// and inserts the new entry.
pub(in crate::content) fn navigate_iframe(
    state: &mut crate::content::ContentState,
    iframe_entity: Entity,
    url: &url::Url,
) {
    // Dispatch unload on old iframe (WHATWG HTML §7.1.3).
    if let Some(mut removed_entry) = state.iframes.remove(iframe_entity) {
        if let IframeHandle::InProcess(ref mut ip) = removed_entry.handle {
            crate::pipeline::dispatch_unload_events(
                &mut ip.pipeline.runtime,
                &mut ip.pipeline.session,
                &mut ip.pipeline.dom,
                ip.pipeline.document,
            );
        }
    }
    // Update `Attributes` directly, then re-derive `IframeData` from the
    // just-written attributes via the canonical reconcile seam. This
    // `navigate_iframe` path force-loads the new `src` itself (below), and
    // `register_iframe_entry` stamps `loaded_src`/`loaded_srcdoc` from that same
    // `IframeData` — so the next `rescan_iframes_by_diff` sees the registered
    // entry already matching the live src/srcdoc and does NOT re-navigate (no
    // double load), while the whole component (not just `.src`) stays consistent
    // with its attributes.
    let url_str = url.to_string();
    if let Ok(mut attrs) = state
        .pipeline
        .dom
        .world_mut()
        .get::<&mut elidex_ecs::Attributes>(iframe_entity)
    {
        attrs.set("src", &url_str);
    }
    state
        .pipeline
        .dom
        .reconcile_attribute_derived_components(iframe_entity, "src");
    try_load_iframe_entity(state, iframe_entity, /* force */ true);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build an `IframeLoadContext` from `ContentState`.
///
/// The returned context borrows the `parent_origin` reference, so the caller
/// must provide a `&SecurityOrigin` that outlives the context.
///
/// Depth is computed from the parent document's `iframe_depth` (stored in the
/// bridge) plus 1, ensuring correct tracking across separate `EcsDom` instances
/// for nested same-origin iframes.
fn build_load_context<'a>(
    state: &'a crate::content::ContentState,
    _entity: Entity,
    parent_origin: &'a elidex_plugin::SecurityOrigin,
) -> IframeLoadContext<'a> {
    // `iframe_depth` converges to the engine-agnostic `HostDriver` trait; the
    // `bridge` alias below survives only for `cookie_jar_clone` (still boa-bound
    // until a later flip stage converges the cookie-jar surface).
    let parent_depth = state.pipeline.runtime.iframe_depth();
    let bridge = state.pipeline.runtime.bridge();
    IframeLoadContext {
        parent_origin,
        parent_url: state.pipeline.url.as_ref(),
        font_db: &state.pipeline.font_db,
        network_handle: &state.pipeline.network_handle,
        cookie_jar: bridge.cookie_jar_clone(),
        depth: parent_depth + 1,
        registry: &state.pipeline.registry,
        // Inherit the parent's live device facts — window/display facts the sub-frame
        // shares (C3). The parent bridge holds the dppx/color-scheme delivered by the
        // shell `SetDeviceFacts` arm, so this is available by construction at build time.
        device_facts: crate::ipc::DeviceFacts {
            dppx: bridge.device_pixel_ratio(),
            color_scheme: bridge.color_scheme(),
        },
    }
}

/// Register a loaded iframe: store display list, insert into registry, fire load event.
///
/// Stamps `loaded_src`/`loaded_srcdoc` from the exact `IframeData` used to load —
/// the single registry chokepoint, so the §4.3.8 `rescan_iframes_by_diff` re-nav
/// diff always compares against what was really loaded.
fn register_iframe_entry(
    state: &mut crate::content::ContentState,
    entity: Entity,
    data: &elidex_ecs::IframeData,
    mut entry: IframeEntry,
) {
    entry.loaded_src = data.src.clone();
    entry.loaded_srcdoc = data.srcdoc.clone();
    if let IframeHandle::InProcess(ref mut ip) = entry.handle {
        let arc_dl = std::sync::Arc::new(ip.pipeline.display_list.clone());
        ip.cached_display_list = Some(std::sync::Arc::clone(&arc_dl));
        set_iframe_display_list(&mut state.pipeline.dom, entity, arc_dl);
    }
    state.iframes.insert(entity, entry);
    dispatch_iframe_load_event(state, entity);
}

/// Dispatch a "load" event on an iframe element entity in the parent document.
fn dispatch_iframe_load_event(state: &mut crate::content::ContentState, iframe_entity: Entity) {
    let mut event = elidex_script_session::DispatchEvent::new("load", iframe_entity);
    event.bubbles = false;
    event.cancelable = false;
    state.pipeline.dispatch_event(&mut event);
}

/// Recursively collect entities with `IframeData` components.
fn collect_iframe_entities(
    dom: &elidex_ecs::EcsDom,
    entity: Entity,
    result: &mut Vec<Entity>,
    walk_depth: usize,
) {
    if walk_depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
        return;
    }
    if dom.world().get::<&elidex_ecs::IframeData>(entity).is_ok() {
        result.push(entity);
    }
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        collect_iframe_entities(dom, c, result, walk_depth + 1);
        child = dom.get_next_sibling(c);
    }
}
