//! Custom element prototype-chain upgrade — HTML §4.13.2 "upgrade an
//! element" algorithm.
//!
//! Sync dual-path return: callers from `customElements.upgrade(root)`
//! propagate the `Err(VmError)` to the JS caller, while reaction-queue
//! drain in [`super::flush`] catches the `Err` and reports it to
//! `Window.onerror` per HTML §4.13.3 "Invoke custom element reactions".

#![cfg(feature = "engine")]

use elidex_custom_elements::{CEState, CustomElementReaction, CustomElementState};
use elidex_ecs::{Attributes, Entity};

use super::super::super::value::{JsValue, NativeContext, VmError};

/// Invoke the upgrade algorithm on `entity` per HTML §4.13.2.
///
/// Returns:
/// - `Ok(())` on a successful upgrade (state transitions to
///   `CEState::Custom`), with `attributeChangedCallback` /
///   `connectedCallback` reactions appended to the queue for the
///   element's existing observed attributes + connectedness.
/// - `Err(VmError)` on a constructor throw — caller decides whether to
///   rethrow (sync `customElements.upgrade(root)` path) or report to
///   Window.onerror (reaction-queue flush path).
pub(crate) fn invoke_upgrade(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<(), VmError> {
    // 1. Resolve the registered definition for `entity`'s state. Skip
    //    if the entity has already been upgraded or marked Failed
    //    (Upgrade reactions can be queued multiple times from the
    //    define-time DOM walk + the pending-upgrade queue drain).
    let Some(host) = ctx.host_if_bound() else {
        return Ok(());
    };
    let (definition_name, current_state) =
        match host.dom_shared().world().get::<&CustomElementState>(entity) {
            Ok(state) => (state.definition_name.clone(), state.state),
            Err(_) => return Ok(()),
        };
    if matches!(current_state, CEState::Custom | CEState::Failed) {
        return Ok(());
    }
    let (constructor_id, observed_attrs) = {
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
        let Some(def) = registry.get(&definition_name) else {
            return Ok(());
        };
        (def.constructor_id, def.observed_attributes.clone())
    };
    let Some(constructor) = host.ce_constructors.get(&constructor_id).copied() else {
        return Ok(());
    };

    // 2. Validate `constructor.prototype` is an Object — even though
    //    we do NOT splice it into the wrapper's prototype chain in v1
    //    (see step 3 below), the spec requires the property to be an
    //    object for the upgrade to proceed. Any throw inside the
    //    getter (adversarial proxy / Object.defineProperty getter)
    //    marks the entity Failed before propagating so re-enqueued
    //    Upgrades short-circuit at the early-return in step 1 rather
    //    than retrying the same throw.
    let proto_key = super::super::super::value::PropertyKey::String(ctx.vm.well_known.prototype);
    let proto_value = match ctx.vm.get_property_value(constructor, proto_key) {
        Ok(v) => v,
        Err(err) => {
            mark_failed(ctx, entity);
            return Err(err);
        }
    };
    if !matches!(proto_value, JsValue::Object(_)) {
        mark_failed(ctx, entity);
        return Err(VmError::type_error(
            "Custom element upgrade failed: constructor.prototype must be an object.",
        ));
    }

    // 3. Resolve the element wrapper.
    //
    // Spec (HTML §4.13.2 step 5) sets element's [[Prototype]] to
    // constructor.prototype, but in v1 elidex `class MyEl extends
    // HTMLElement` is unwired (`HTMLElement` is not a global
    // constructor — `#11-html-element-constructor-base` slot). Without
    // an HTMLElement super-class, splicing constructor.prototype onto
    // the wrapper would orphan the wrapper from Element.prototype and
    // break `el.setAttribute` / `el.appendChild` / every Element /
    // Node method. Until that slot lands, the wrapper keeps its
    // standard Element-chain prototype; lifecycle callbacks are looked
    // up on `constructor.prototype` directly by
    // [`super::flush::invoke_callback`] rather than via the wrapper's
    // own chain.
    let wrapper_id = ctx.vm.create_element_wrapper(entity);

    // 4. State transition + construct.
    set_state(ctx, entity, CEState::Precustomized);
    let saved_construct = ctx.vm.in_construct;
    ctx.vm.in_construct = true;
    let result = ctx.vm.call(constructor, JsValue::Object(wrapper_id), &[]);
    ctx.vm.in_construct = saved_construct;
    match result {
        Ok(_) => {
            set_state(ctx, entity, CEState::Custom);
        }
        Err(err) => {
            mark_failed(ctx, entity);
            // Drop any pending reactions targeting this entity per
            // HTML §4.13.2 step 8 (Failed elements skip lifecycle
            // callbacks).
            scrub_entity_reactions(ctx, entity);
            return Err(err);
        }
    }

    // 5. Enqueue attributeChangedCallback reactions for already-present
    //    observed attributes (HTML §4.13.2 step 4.1).
    if !observed_attrs.is_empty() {
        let host = ctx.host();
        let to_enqueue: Vec<(String, String)> = {
            let dom = host.dom_shared();
            match dom.world().get::<&Attributes>(entity) {
                Ok(attrs) => attrs
                    .iter()
                    .filter(|(name, _)| observed_attrs.iter().any(|n| n == *name))
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect::<Vec<_>>(),
                Err(_) => Vec::new(),
            }
        };
        if !to_enqueue.is_empty() {
            let mut queue = host
                .ce_reaction_queue
                .lock()
                .expect("CE reaction queue mutex poisoned");
            for (name, value) in to_enqueue {
                queue.push_back(CustomElementReaction::AttributeChanged {
                    entity,
                    name,
                    old_value: None,
                    new_value: Some(value),
                });
            }
        }
    }

    // 6. If element is connected, enqueue Connected.
    let connected = ctx.host().dom_shared().is_connected(entity);
    if connected {
        ctx.host()
            .ce_reaction_queue
            .lock()
            .expect("CE reaction queue mutex poisoned")
            .push_back(CustomElementReaction::Connected(entity));
    }

    Ok(())
}

fn set_state(ctx: &mut NativeContext<'_>, entity: Entity, new_state: CEState) {
    let host = ctx.host();
    let dom = host.dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut CustomElementState>(entity) {
        state.state = new_state;
    }
}

fn mark_failed(ctx: &mut NativeContext<'_>, entity: Entity) {
    set_state(ctx, entity, CEState::Failed);
}

/// Drop every queued reaction that targets `entity`. Called after a
/// Failed upgrade per HTML §4.13.2 step 8 ("empty element's CE
/// reaction queue").
fn reaction_target(r: &CustomElementReaction) -> Entity {
    match r {
        CustomElementReaction::Upgrade(e)
        | CustomElementReaction::Connected(e)
        | CustomElementReaction::Disconnected(e) => *e,
        CustomElementReaction::AttributeChanged { entity, .. }
        | CustomElementReaction::Adopted { entity, .. } => *entity,
    }
}

fn scrub_entity_reactions(ctx: &mut NativeContext<'_>, entity: Entity) {
    let mut queue = ctx
        .host()
        .ce_reaction_queue
        .lock()
        .expect("CE reaction queue mutex poisoned");
    queue.retain(|r| reaction_target(r) != entity);
}
