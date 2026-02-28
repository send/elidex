//! DOM event dispatch with capture/target/bubble phases.
//!
//! Implements the DOM Events spec dispatch algorithm. The `dispatch_event`
//! function traverses the propagation path and invokes listeners via a
//! callback, keeping the implementation engine-independent.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{EventPayload, EventPhase};

use crate::event_listener::{EventListeners, ListenerId};

/// A DOM event being dispatched through the tree.
#[allow(clippy::struct_excessive_bools)]
#[non_exhaustive]
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
    /// Set to `true` by `preventDefault()`.
    pub default_prevented: bool,
    /// Set to `true` by `stopPropagation()`.
    pub propagation_stopped: bool,
    /// Set to `true` by `stopImmediatePropagation()`.
    pub immediate_propagation_stopped: bool,
}

impl DispatchEvent {
    /// Create a new dispatch event with the given type and target.
    ///
    /// Defaults: `bubbles = true`, `cancelable = true`.
    /// Override fields as needed for non-bubbling (`focus`, `blur`) or
    /// non-cancelable (`mousemove`) events.
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
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
        }
    }
}

/// Build the propagation path from the document root down to `target`.
///
/// Returns `[root, ..., parent, target]`. If `target` has no parent,
/// returns `[target]`.
#[must_use]
pub fn build_propagation_path(dom: &EcsDom, target: Entity) -> Vec<Entity> {
    let mut path = Vec::new();
    let mut current = Some(target);
    let mut depth = 0;
    while let Some(entity) = current {
        path.push(entity);
        depth += 1;
        if depth > 10_000 {
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
    let path = build_propagation_path(dom, event.target);
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
    let at_target_ids = collect_all_listeners(dom, target, event_type);
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

/// Collect all listener IDs for an event type on an entity (both capture and bubble).
fn collect_all_listeners(dom: &EcsDom, entity: Entity, event_type: &str) -> Vec<ListenerId> {
    collect_listeners(dom, entity, event_type, None)
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

    // Phase 1: Capture (root → target, exclusive)
    event.phase = EventPhase::Capturing;
    for (entity, ids) in &plan.capture {
        if event.propagation_stopped {
            break;
        }
        event.current_target = Some(*entity);
        for &id in ids {
            if event.immediate_propagation_stopped {
                break;
            }
            invoke(id, *entity, event);
        }
    }

    // Phase 2: At-target
    if !event.propagation_stopped {
        if let Some((target, ids)) = &plan.at_target {
            event.phase = EventPhase::AtTarget;
            event.current_target = Some(*target);
            for &id in ids {
                if event.immediate_propagation_stopped {
                    break;
                }
                invoke(id, *target, event);
            }
        }
    }

    // Phase 3: Bubble (target → root, exclusive, reversed)
    if event.bubbles && !event.propagation_stopped {
        event.phase = EventPhase::Bubbling;
        for (entity, ids) in &plan.bubble {
            if event.propagation_stopped {
                break;
            }
            event.current_target = Some(*entity);
            for &id in ids {
                if event.immediate_propagation_stopped {
                    break;
                }
                invoke(id, *entity, event);
            }
        }
    }

    event.phase = EventPhase::None;
    event.current_target = None;

    event.default_prevented
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn propagation_path_single_node() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        let path = build_propagation_path(&dom, e);
        assert_eq!(path, vec![e]);
    }

    #[test]
    fn propagation_path_deep() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let child = elem(&mut dom, "p");
        let grandchild = elem(&mut dom, "span");
        dom.append_child(root, child);
        dom.append_child(child, grandchild);

        let path = build_propagation_path(&dom, grandchild);
        assert_eq!(path, vec![root, child, grandchild]);
    }

    #[test]
    fn propagation_path_detached() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        // Detached node — should just return itself.
        let path = build_propagation_path(&dom, e);
        assert_eq!(path, vec![e]);
    }

    #[test]
    fn dispatch_capture_phase() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let target = elem(&mut dom, "span");
        dom.append_child(root, target);

        // Add capture listener on root.
        let mut root_listeners = EventListeners::new();
        let lid = root_listeners.add("click", true);
        dom.world_mut().insert_one(root, root_listeners).unwrap();

        let mut event = DispatchEvent::new("click", target);
        let mut invoked = Vec::new();
        dispatch_event(&dom, &mut event, &mut |id, entity, _ev| {
            invoked.push((id, entity));
        });

        assert_eq!(invoked.len(), 1);
        assert_eq!(invoked[0], (lid, root));
    }

    #[test]
    fn dispatch_bubble_phase() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let target = elem(&mut dom, "span");
        dom.append_child(root, target);

        // Add bubble listener on root.
        let mut root_listeners = EventListeners::new();
        let lid = root_listeners.add("click", false);
        dom.world_mut().insert_one(root, root_listeners).unwrap();

        let mut event = DispatchEvent::new("click", target);
        let mut invoked = Vec::new();
        dispatch_event(&dom, &mut event, &mut |id, entity, _ev| {
            invoked.push((id, entity));
        });

        assert_eq!(invoked.len(), 1);
        assert_eq!(invoked[0], (lid, root));
    }

    #[test]
    fn dispatch_no_bubble() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let target = elem(&mut dom, "span");
        dom.append_child(root, target);

        // Bubble listener on root.
        let mut root_listeners = EventListeners::new();
        root_listeners.add("focus", false);
        dom.world_mut().insert_one(root, root_listeners).unwrap();

        let mut event = DispatchEvent::new("focus", target);
        event.bubbles = false;

        let mut invoked = Vec::new();
        dispatch_event(&dom, &mut event, &mut |id, entity, _ev| {
            invoked.push((id, entity));
        });

        // Bubble listener should NOT fire for non-bubbling event.
        assert!(invoked.is_empty());
    }

    #[test]
    fn dispatch_stop_propagation() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let mid = elem(&mut dom, "p");
        let target = elem(&mut dom, "span");
        dom.append_child(root, mid);
        dom.append_child(mid, target);

        // Capture listener on root that stops propagation.
        let mut root_listeners = EventListeners::new();
        root_listeners.add("click", true);
        dom.world_mut().insert_one(root, root_listeners).unwrap();

        // Capture listener on mid — should NOT fire.
        let mut mid_listeners = EventListeners::new();
        mid_listeners.add("click", true);
        dom.world_mut().insert_one(mid, mid_listeners).unwrap();

        let mut event = DispatchEvent::new("click", target);
        let mut count = 0;
        dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
            count += 1;
            ev.propagation_stopped = true;
        });

        assert_eq!(count, 1);
    }

    #[test]
    fn dispatch_stop_immediate_propagation() {
        let mut dom = EcsDom::new();
        let target = elem(&mut dom, "span");

        // Two listeners on target.
        let mut listeners = EventListeners::new();
        listeners.add("click", false);
        listeners.add("click", false);
        dom.world_mut().insert_one(target, listeners).unwrap();

        let mut event = DispatchEvent::new("click", target);
        let mut count = 0;
        dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
            count += 1;
            ev.immediate_propagation_stopped = true;
        });

        // Only the first listener should fire.
        assert_eq!(count, 1);
    }

    #[test]
    fn dispatch_prevent_default() {
        let mut dom = EcsDom::new();
        let target = elem(&mut dom, "span");

        let mut listeners = EventListeners::new();
        listeners.add("click", false);
        dom.world_mut().insert_one(target, listeners).unwrap();

        let mut event = DispatchEvent::new("click", target);
        let prevented = dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
            ev.default_prevented = true;
        });

        assert!(prevented);
    }

    #[test]
    fn dispatch_at_target_fires_both_capture_and_bubble() {
        let mut dom = EcsDom::new();
        let target = elem(&mut dom, "span");

        let mut listeners = EventListeners::new();
        let cap_id = listeners.add("click", true);
        let bub_id = listeners.add("click", false);
        dom.world_mut().insert_one(target, listeners).unwrap();

        let mut event = DispatchEvent::new("click", target);
        let mut invoked = Vec::new();
        dispatch_event(&dom, &mut event, &mut |id, _entity, _ev| {
            invoked.push(id);
        });

        // Both should fire at target.
        assert_eq!(invoked, vec![cap_id, bub_id]);
    }

    #[test]
    fn dispatch_full_lifecycle() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let target = elem(&mut dom, "span");
        dom.append_child(root, target);

        // Capture listener on root.
        let mut root_listeners = EventListeners::new();
        let cap_id = root_listeners.add("click", true);
        let bub_id = root_listeners.add("click", false);
        dom.world_mut().insert_one(root, root_listeners).unwrap();

        // Listener on target.
        let mut target_listeners = EventListeners::new();
        let tgt_id = target_listeners.add("click", false);
        dom.world_mut()
            .insert_one(target, target_listeners)
            .unwrap();

        let mut event = DispatchEvent::new("click", target);
        let mut phases = Vec::new();
        dispatch_event(&dom, &mut event, &mut |id, entity, ev| {
            phases.push((id, entity, ev.phase));
        });

        // Capture on root, at-target on target, bubble on root.
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0], (cap_id, root, EventPhase::Capturing));
        assert_eq!(phases[1], (tgt_id, target, EventPhase::AtTarget));
        assert_eq!(phases[2], (bub_id, root, EventPhase::Bubbling));
    }
}
