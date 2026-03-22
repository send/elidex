//! DOM event dispatch with capture/target/bubble phases.
//!
//! Implements the DOM Events spec dispatch algorithm. The `dispatch_event`
//! function traverses the propagation path and invokes listeners via a
//! callback, keeping the implementation engine-independent.

use elidex_ecs::{EcsDom, Entity, ShadowRoot, ShadowRootMode, SlottedMarker, MAX_ANCESTOR_DEPTH};
use elidex_plugin::{EventPayload, EventPhase};

use crate::event_listener::{EventListeners, ListenerId};

/// Mutable dispatch state flags, set by event handler methods
/// (`preventDefault`, `stopPropagation`, `stopImmediatePropagation`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DispatchFlags {
    /// Set to `true` by `preventDefault()`.
    pub default_prevented: bool,
    /// Set to `true` by `stopPropagation()`.
    pub propagation_stopped: bool,
    /// Set to `true` by `stopImmediatePropagation()`.
    pub immediate_propagation_stopped: bool,
}

/// A DOM event being dispatched through the tree.
#[non_exhaustive]
#[allow(clippy::struct_excessive_bools)] // DOM Event spec requires bubbles, cancelable, composed
pub struct DispatchEvent {
    /// The event type (e.g. `"click"`, `"keydown"`).
    pub event_type: String,
    /// Whether this event bubbles up the tree.
    pub bubbles: bool,
    /// Whether this event can be cancelled via `preventDefault()`.
    pub cancelable: bool,
    /// Event-specific data (mouse coordinates, key info, etc.).
    pub payload: EventPayload,
    /// Current propagation phase.
    pub phase: EventPhase,
    /// The original target entity.
    pub target: Entity,
    /// The entity whose listeners are currently being invoked.
    pub current_target: Option<Entity>,
    /// Mutable dispatch state flags (preventDefault, stopPropagation, etc.).
    pub flags: DispatchFlags,
    /// Whether this event crosses shadow DOM boundaries.
    ///
    /// Most UA events (click, input, etc.) are composed. Custom events
    /// default to `false`. Non-composed events stop at shadow boundaries.
    pub composed: bool,
    /// The original target before retargeting across shadow boundaries.
    ///
    /// Set when a shadow-internal target is retargeted to the shadow host
    /// for listeners outside the shadow tree.
    pub original_target: Option<Entity>,
    /// The full propagation path for `composedPath()`.
    ///
    /// Contains all entities in the event propagation path, including shadow
    /// root entities for composed events. Built by `dispatch_event()`.
    ///
    /// WHATWG DOM §2.10 specifies per-entry metadata for each path entry:
    /// - `invocationTarget`: the event target for this entry
    /// - `invocationTargetInShadowTree`: whether the target is in a shadow tree
    /// - `shadowAdjustedTarget`: the retargeted target for this listener scope
    /// - `relatedTarget`: retargeted related target (for mouse/focus events)
    /// - `touchTargets`: retargeted touch targets (for touch events)
    ///
    /// Currently we store only the entity list; per-listener filtering and
    /// retargeting is done lazily in `composed_path_for_js()` and
    /// `apply_retarget()`. This is sufficient for correctness: the retarget
    /// algorithm produces the same result whether computed eagerly per-entry
    /// or lazily per-listener. The `relatedTarget` and `touchTargets` fields
    /// are not yet needed (mouse `relatedTarget` is not yet implemented;
    /// touch events are not supported). When those features are added, this
    /// should be changed to `Vec<PathEntry>` with the full metadata.
    pub composed_path: Vec<Entity>,
    /// Whether the event is currently being dispatched.
    ///
    /// WHATWG DOM §2.10: `composedPath()` returns an empty sequence when
    /// this flag is not set.
    pub dispatch_flag: bool,
}

impl DispatchEvent {
    /// Creates a custom event with `composed: false` (WHATWG default for custom events).
    ///
    /// Defaults: `bubbles: true`, `cancelable: true`, `composed: false`.
    #[must_use]
    pub fn new(event_type: impl Into<String>, target: Entity) -> Self {
        Self {
            event_type: event_type.into(),
            bubbles: true,
            cancelable: true,
            payload: EventPayload::None,
            phase: EventPhase::None,
            target,
            current_target: None,
            flags: DispatchFlags::default(),
            composed: false,
            original_target: None,
            composed_path: Vec::new(),
            dispatch_flag: false,
        }
    }

    /// Creates a UA event with `composed: true` (most UA events cross shadow boundaries).
    ///
    /// Use this for browser-initiated events like `click`, `mousedown`, `input`, etc.
    #[must_use]
    pub fn new_composed(event_type: impl Into<String>, target: Entity) -> Self {
        let mut event = Self::new(event_type, target);
        event.composed = true;
        event
    }
}

/// Return the `composedPath()` for JS, applying WHATWG DOM §2.10 filtering.
///
/// - If the dispatch flag is not set, returns an empty vec.
/// - For closed shadow roots that are NOT shadow-inclusive ancestors of
///   `current_target`, entries inside that closed shadow tree are excluded.
#[must_use]
pub fn composed_path_for_js(event: &DispatchEvent, dom: &EcsDom) -> Vec<Entity> {
    if !event.dispatch_flag {
        return Vec::new();
    }
    let Some(current_target) = event.current_target else {
        return event.composed_path.clone();
    };
    // Filter: exclude entries whose tree root is a closed ShadowRoot that
    // does not contain current_target.
    event
        .composed_path
        .iter()
        .copied()
        .filter(|&entry| {
            // Check if entry itself is a closed shadow root.
            let self_closed = dom
                .world()
                .get::<&ShadowRoot>(entry)
                .ok()
                .is_some_and(|sr| sr.mode == ShadowRootMode::Closed);
            if self_closed {
                return is_in_subtree_of(dom, current_target, entry);
            }

            // Check if entry's tree root is a closed shadow root.
            let root = find_tree_root(dom, entry);
            let is_closed_shadow = dom
                .world()
                .get::<&ShadowRoot>(root)
                .ok()
                .is_some_and(|sr| sr.mode == ShadowRootMode::Closed);
            if is_closed_shadow {
                is_in_subtree_of(dom, current_target, root)
            } else {
                true
            }
        })
        .collect()
}

/// Build the propagation path from the document root down to `target`.
///
/// Returns `[root, ..., parent, target]`. If `target` has no parent,
/// returns `[target]`.
///
/// For composed events, the path crosses shadow boundaries (shadow root
/// entities are included but shadow host continues the traversal).
/// For non-composed events, the path stops at shadow root boundaries.
#[must_use]
pub fn build_propagation_path(dom: &EcsDom, target: Entity, composed: bool) -> Vec<Entity> {
    let mut path = Vec::new();
    let mut current = Some(target);
    let mut depth = 0;
    while let Some(entity) = current {
        path.push(entity);
        depth += 1;
        if depth > MAX_ANCESTOR_DEPTH {
            break;
        }
        // For non-composed events, stop at shadow root boundaries.
        if !composed && dom.world().get::<&ShadowRoot>(entity).is_ok() {
            break;
        }
        current = dom.get_parent(entity);
    }
    path.reverse();
    path
}

/// Pre-collected dispatch plan: propagation path with listener IDs per entity.
///
/// Built from `&EcsDom` before invoking any callbacks, so the DOM borrow
/// is released before listener functions execute (enabling DOM mutation
/// from callbacks).
struct DispatchPlan {
    /// `(entity, capture_listener_ids)` for capture phase (root → target exclusive).
    capture: Vec<(Entity, Vec<ListenerId>)>,
    /// `(entity, all_listener_ids)` for at-target phase.
    at_target: Option<(Entity, Vec<ListenerId>)>,
    /// `(entity, bubble_listener_ids)` for bubble phase (target exclusive → root).
    bubble: Vec<(Entity, Vec<ListenerId>)>,
}

/// Build a dispatch plan by pre-collecting all listener IDs from the DOM.
///
/// This releases the `&EcsDom` borrow *before* any listener callbacks run,
/// allowing callbacks to safely mutate the DOM via the bridge.
fn build_dispatch_plan(dom: &EcsDom, event: &DispatchEvent) -> DispatchPlan {
    let path = build_propagation_path(dom, event.target, event.composed);
    if path.is_empty() {
        return DispatchPlan {
            capture: Vec::new(),
            at_target: None,
            bubble: Vec::new(),
        };
    }

    let target_idx = path.len() - 1;
    let event_type = &event.event_type;

    // Capture: root → target (exclusive).
    let capture: Vec<_> = path[..target_idx]
        .iter()
        .map(|&entity| {
            let ids = collect_listeners(dom, entity, event_type, Some(true));
            (entity, ids)
        })
        .collect();

    // At-target: all listeners on target.
    let target = path[target_idx];
    let at_target_ids = collect_listeners(dom, target, event_type, None);
    let at_target = Some((target, at_target_ids));

    // Bubble: target (exclusive) → root (reversed).
    let bubble: Vec<_> = path[..target_idx]
        .iter()
        .rev()
        .map(|&entity| {
            let ids = collect_listeners(dom, entity, event_type, Some(false));
            (entity, ids)
        })
        .collect();

    DispatchPlan {
        capture,
        at_target,
        bubble,
    }
}

/// Collect listener IDs matching (`event_type`, `capture`) on an entity.
fn collect_listeners(
    dom: &EcsDom,
    entity: Entity,
    event_type: &str,
    capture: Option<bool>,
) -> Vec<ListenerId> {
    dom.world()
        .get::<&EventListeners>(entity)
        .ok()
        .map(|listeners| match capture {
            Some(cap) => listeners.matching(event_type, cap),
            None => listeners.matching_all_ids(event_type),
        })
        .unwrap_or_default()
}

/// Invoke listeners on an entity, stopping if immediate propagation is halted.
fn invoke_listeners(
    ids: &[ListenerId],
    entity: Entity,
    event: &mut DispatchEvent,
    invoke: &mut dyn FnMut(ListenerId, Entity, &mut DispatchEvent),
) {
    for &id in ids {
        if event.flags.immediate_propagation_stopped {
            break;
        }
        invoke(id, entity, event);
    }
}

/// Dispatch an event through the propagation path.
///
/// Executes the DOM spec 3-phase dispatch:
/// 1. **Capture**: root → target (exclusive) — capture listeners only
/// 2. **At-target**: target — all listeners (capture + bubble)
/// 3. **Bubble**: target (exclusive) → root — bubble listeners only
///    (only if `event.bubbles` is `true`)
///
/// Listener IDs are pre-collected from the DOM before any callbacks run,
/// so the `&EcsDom` borrow is released before `invoke` executes. This
/// allows callbacks to safely mutate the DOM.
///
/// The `invoke` callback is called for each matching listener, receiving
/// the `ListenerId`, the current entity, and the event. The JS layer
/// uses this to look up and call the actual JS function.
///
/// Returns `true` if `preventDefault()` was called.
pub fn dispatch_event(
    dom: &EcsDom,
    event: &mut DispatchEvent,
    invoke: &mut dyn FnMut(ListenerId, Entity, &mut DispatchEvent),
) -> bool {
    // Pre-collect all listener IDs so DOM borrow is released before callbacks run.
    let plan = build_dispatch_plan(dom, event);

    // Build the composed path for composedPath() JS method.
    event.composed_path = build_propagation_path(dom, event.target, event.composed);
    event.dispatch_flag = true;

    let saved_target = event.target;

    // Phase 1: Capture (root → target, exclusive)
    event.phase = EventPhase::Capturing;
    for (entity, ids) in &plan.capture {
        if event.flags.propagation_stopped || event.flags.immediate_propagation_stopped {
            break;
        }
        // B3: Per-listener retarget using WHATWG iterative algorithm.
        apply_retarget(event, *entity, saved_target, dom);
        event.current_target = Some(*entity);
        invoke_listeners(ids, *entity, event, invoke);
    }

    // Phase 2: At-target
    if !event.flags.propagation_stopped && !event.flags.immediate_propagation_stopped {
        if let Some((target, ids)) = &plan.at_target {
            event.phase = EventPhase::AtTarget;
            event.target = saved_target;
            event.original_target = None;
            event.current_target = Some(*target);
            invoke_listeners(ids, *target, event, invoke);
        }
    }

    // Phase 3: Bubble (target → root, exclusive, reversed)
    if event.bubbles
        && !event.flags.propagation_stopped
        && !event.flags.immediate_propagation_stopped
    {
        event.phase = EventPhase::Bubbling;
        for (entity, ids) in &plan.bubble {
            if event.flags.propagation_stopped || event.flags.immediate_propagation_stopped {
                break;
            }
            apply_retarget(event, *entity, saved_target, dom);
            event.current_target = Some(*entity);
            invoke_listeners(ids, *entity, event, invoke);
        }
    }

    event.phase = EventPhase::None;
    event.current_target = None;
    event.target = saved_target;
    event.original_target = None;
    event.dispatch_flag = false;

    event.flags.default_prevented
}

/// WHATWG DOM §2.5 `retarget(A, B)` — iterative retarget algorithm.
///
/// Adjusts the event target `A` relative to a scope entity `B` (the listener
/// entity). If `A` is inside a shadow tree that does not contain `B`, `A` is
/// retargeted to the shadow host. This process repeats for nested shadow DOMs.
///
/// Returns the retargeted entity (which may be `A` itself if no retargeting
/// is needed).
fn retarget(dom: &EcsDom, mut a: Entity, b: Entity) -> Entity {
    let mut depth = 0;
    loop {
        depth += 1;
        if depth > MAX_ANCESTOR_DEPTH {
            break;
        }

        // Find tree root of A.
        let root = find_tree_root(dom, a);

        // If root is not a ShadowRoot → no more retargeting needed.
        let Ok(sr) = dom.world().get::<&ShadowRoot>(root) else {
            break;
        };

        // If root's tree contains B → B is inside the same shadow tree, no retarget.
        if is_in_subtree_of(dom, b, root) {
            break;
        }

        // Retarget A to the shadow host and repeat.
        a = sr.host;
    }
    a
}

// Wrappers for EcsDom tree utilities used in event dispatch.
fn find_tree_root(dom: &EcsDom, entity: Entity) -> Entity {
    dom.find_tree_root(entity)
}

fn is_in_subtree_of(dom: &EcsDom, entity: Entity, root: Entity) -> bool {
    dom.is_ancestor_or_self(root, entity)
}

/// Apply per-listener retargeting using the WHATWG iterative retarget algorithm.
///
/// For each listener invocation, the event target is `retarget(original_target,
/// listener_entity)`. Slotted elements (light DOM nodes assigned to a slot)
/// are exempt from retargeting only when the listener is in light DOM context.
/// Shadow-internal listeners still see the retargeted target.
fn apply_retarget(
    event: &mut DispatchEvent,
    listener_entity: Entity,
    original_target: Entity,
    dom: &EcsDom,
) {
    // M6: Slotted exemption only applies to light DOM listeners.
    // Shadow-internal listeners should see the retargeted target.
    if dom.world().get::<&SlottedMarker>(original_target).is_ok() {
        let listener_root = find_tree_root(dom, listener_entity);
        let listener_in_shadow = dom.world().get::<&ShadowRoot>(listener_root).is_ok();
        if !listener_in_shadow {
            event.target = original_target;
            event.original_target = None;
            return;
        }
    }

    let retargeted = retarget(dom, original_target, listener_entity);
    if retargeted == original_target {
        event.target = original_target;
        event.original_target = None;
    } else {
        event.target = retargeted;
        event.original_target = Some(original_target);
    }
}

#[cfg(test)]
mod tests;
