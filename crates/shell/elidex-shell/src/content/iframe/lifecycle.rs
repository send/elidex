//! Iframe lifecycle: mutation detection, lazy loading, unloading, DOM scanning.

use elidex_ecs::Entity;

use super::load::load_iframe;
use super::render::set_iframe_display_list;
use super::types::{BrowserToIframe, IframeEntry, IframeHandle, IframeLoadContext};

/// Detect iframe additions/removals from mutation records.
///
/// Scans `MutationRecord` added/removed nodes for entities with `IframeData`
/// components, and triggers iframe loading/unloading accordingly.
///
/// Also detects `src` attribute changes on existing `<iframe>` elements
/// to trigger re-navigation.
pub(in crate::content) fn detect_iframe_mutations(
    records: &[elidex_script_session::MutationRecord],
    state: &mut crate::content::ContentState,
) -> bool {
    use elidex_script_session::MutationKind;
    let mut changed = false;

    for record in records {
        match record.kind {
            MutationKind::ChildList => {
                // Check added nodes (and their subtrees) for <iframe> elements.
                for &entity in &record.added_nodes {
                    let mut nested = Vec::new();
                    collect_iframe_entities(&state.pipeline.dom, entity, &mut nested, 0);
                    for iframe_entity in nested {
                        if state.iframes.get(iframe_entity).is_some() {
                            continue;
                        }
                        try_load_iframe_entity(state, iframe_entity, false);
                        changed = true;
                    }
                }
                // Check removed nodes (and their subtrees) for <iframe> elements.
                let mut removed_set = std::collections::HashSet::new();
                for &entity in &record.removed_nodes {
                    let mut nested = Vec::new();
                    collect_iframe_entities(&state.pipeline.dom, entity, &mut nested, 0);
                    for iframe_entity in nested {
                        if let Some(removed_entry) = state.iframes.remove(iframe_entity) {
                            unload_iframe_entry(state, iframe_entity, removed_entry);
                        }
                        removed_set.insert(iframe_entity);
                    }
                }
                if !removed_set.is_empty() {
                    state.iframes.remove_lazy_pending_batch(&removed_set);
                    changed = true;
                }
            }
            MutationKind::Attribute => {
                // src attribute change on <iframe> → re-navigate.
                if record
                    .attribute_name
                    .as_deref()
                    .is_some_and(|name| name == "src")
                {
                    let target = record.target;
                    if let Some(removed_entry) = state.iframes.remove(target) {
                        unload_iframe_entry(state, target, removed_entry);
                    }
                    // Sync IframeData.src from Attributes.
                    sync_iframe_src_from_attrs(state, target);
                    state.iframes.remove_lazy_pending(target);
                    // force=true: src change is explicit navigation.
                    try_load_iframe_entity(state, target, true);
                    changed = true;
                }
            }
            _ => {}
        }
    }
    changed
}

/// Sync `IframeData.src` from `Attributes` — `setAttribute('src', ...)`
/// updates `Attributes` via mutation flush but not `IframeData` directly.
fn sync_iframe_src_from_attrs(state: &mut crate::content::ContentState, entity: Entity) {
    if let Some(new_src) = state
        .pipeline
        .dom
        .world()
        .get::<&elidex_ecs::Attributes>(entity)
        .ok()
        .and_then(|a| a.get("src").map(String::from))
    {
        if let Ok(mut ifd) = state
            .pipeline
            .dom
            .world_mut()
            .get::<&mut elidex_ecs::IframeData>(entity)
        {
            ifd.src = Some(new_src);
        }
    }
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

    let parent_origin = state.pipeline.runtime.bridge().origin();
    let ctx = build_load_context(state, entity, &parent_origin);
    let entry = load_iframe(&data, &ctx);
    register_iframe_entry(state, entity, entry);
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
            let parent_origin = state.pipeline.runtime.bridge().origin();
            let ctx = build_load_context(state, entity, &parent_origin);
            let entry = load_iframe(&data, &ctx);
            register_iframe_entry(state, entity, entry);
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
    // Update IframeData.src and Attributes directly (no mutation record).
    // Recording a mutation would cause detect_iframe_mutations to re-trigger
    // loading, resulting in a double load.
    let url_str = url.to_string();
    if let Ok(mut attrs) = state
        .pipeline
        .dom
        .world_mut()
        .get::<&mut elidex_ecs::Attributes>(iframe_entity)
    {
        attrs.set("src", &url_str);
    }
    if let Ok(mut iframe_data) = state
        .pipeline
        .dom
        .world_mut()
        .get::<&mut elidex_ecs::IframeData>(iframe_entity)
    {
        iframe_data.src = Some(url_str);
    }
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
    let parent_depth = state.pipeline.runtime.bridge().iframe_depth();
    IframeLoadContext {
        parent_origin,
        parent_url: state.pipeline.url.as_ref(),
        font_db: &state.pipeline.font_db,
        fetch_handle: &state.pipeline.fetch_handle,
        depth: parent_depth + 1,
        registry: &state.pipeline.registry,
    }
}

/// Register a loaded iframe: store display list, insert into registry, fire load event.
fn register_iframe_entry(
    state: &mut crate::content::ContentState,
    entity: Entity,
    mut entry: IframeEntry,
) {
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
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
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
