//! Custom element reaction queue drain — HTML §4.13.6 "Invoke custom
//! element reactions".
//!
//! Invoked from `VmInner::flush_ce_reactions` at script-execution /
//! event-dispatch / microtask checkpoints. Iterates the queue until
//! empty (callbacks may enqueue more reactions); bounded by
//! [`MAX_CE_DRAIN_ITERATIONS`] to defend against pathological cycles.
//!
//! Exception policy (HTML §4.13.6 "invoke custom element reactions"
//! step 4): each callback runs inside its own try/catch — a throw is
//! reported via `eprintln!` (the VM-side analog of `Window.onerror`)
//! and the drain continues. The
//! one exception is `Upgrade` reactions, where a constructor throw
//! also sets the entity's state to `CEState::Failed` (handled inside
//! [`super::upgrade::invoke_upgrade`]).

#![cfg(feature = "engine")]

use std::sync::PoisonError;

use elidex_custom_elements::{CustomElementReaction, CustomElementState};
use elidex_ecs::Entity;

use super::super::super::value::{JsValue, NativeContext, PropertyKey, VmError};
use super::super::super::VmInner;

/// Maximum drain-wave count per `flush_ce_reactions` call. A wave =
/// drain the queue once, invoke each reaction's callback, drain again
/// if those callbacks enqueued more reactions. Bounded so a pathological
/// cycle (`connectedCallback() { newCe.appendChild(otherCe) }` infinite
/// nesting) cannot hot-loop the VM. Matches boa's `MAX_CE_DRAIN_ITERATIONS`
/// (see `crates/script/elidex-js-boa/src/runtime/ce.rs`). Realistic
/// pages drain in 1-3 waves; sites that exceed this cap get a stderr
/// warning and their tail reactions defer to the next checkpoint.
const MAX_CE_DRAIN_ITERATIONS: usize = 16;

impl VmInner {
    /// Drain every pending custom element reaction. Safe to call when
    /// `HostData` is unbound (no-op).
    pub(crate) fn flush_ce_reactions(&mut self) {
        let Some(host) = self.host_data.as_deref() else {
            return;
        };
        if !host.is_bound() {
            return;
        }

        for _iteration in 0..MAX_CE_DRAIN_ITERATIONS {
            let reactions: Vec<CustomElementReaction> = {
                // Re-check host_data per iteration: a callback fired by
                // a prior iteration could (via future shell-exposed
                // hooks) unbind the VM mid-drain. Graceful early-return
                // instead of panicking via `.expect`.
                let Some(host) = self.host_data.as_deref() else {
                    return;
                };
                if !host.is_bound() {
                    return;
                }
                let mut queue = host
                    .ce_reaction_queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if queue.is_empty() {
                    return;
                }
                queue.drain(..).collect()
            };

            for reaction in reactions {
                let _ = invoke_one(self, reaction);
            }
            // HTML "Cleanup after handling a reaction" — perform a
            // microtask checkpoint after each batch so Promise
            // reactions queued inside lifecycle callbacks
            // (`constructor(){ fetch().then(...) }` pattern) drain
            // before the next CE reaction batch runs. Without this,
            // microtasks would only fire after the OUTER eval-tail
            // drain, observably one tick late vs Chrome.
            self.drain_microtasks();
        }

        // F13: only warn on real overflow — the loop above always
        // exits early via `queue.is_empty()` when drained, so reaching
        // here means we exhausted MAX_CE_DRAIN_ITERATIONS waves AND
        // the queue is STILL non-empty.
        let still_pending = self
            .host_data
            .as_deref()
            .and_then(|h| h.is_bound().then_some(h))
            .is_some_and(|h| h.ce_reaction_queue.lock().is_ok_and(|q| !q.is_empty()));
        if still_pending {
            eprintln!(
                "[CE] reaction drain hit max iterations ({MAX_CE_DRAIN_ITERATIONS}); \
                 some lifecycle callbacks deferred to the next checkpoint"
            );
        }
    }
}

fn invoke_one(vm: &mut VmInner, reaction: CustomElementReaction) -> Result<(), VmError> {
    let mut ctx = NativeContext { vm };
    match reaction {
        CustomElementReaction::Upgrade(entity) => {
            if let Err(err) = super::upgrade::invoke_upgrade(&mut ctx, entity) {
                eprintln!("[CE Upgrade Error] {}", err.message);
            }
        }
        CustomElementReaction::Connected(entity) => {
            let cb_sid = ctx.vm.well_known.connected_callback;
            invoke_callback(&mut ctx, entity, cb_sid, &[]);
        }
        CustomElementReaction::Disconnected(entity) => {
            let cb_sid = ctx.vm.well_known.disconnected_callback;
            invoke_callback(&mut ctx, entity, cb_sid, &[]);
        }
        CustomElementReaction::AttributeChanged {
            entity,
            name,
            old_value,
            new_value,
        } => {
            let name_sid = ctx.vm.strings.intern(&name);
            let old_val = old_value
                .as_deref()
                .map_or(JsValue::Null, |v| JsValue::String(ctx.vm.strings.intern(v)));
            let new_val = new_value
                .as_deref()
                .map_or(JsValue::Null, |v| JsValue::String(ctx.vm.strings.intern(v)));
            let args = [JsValue::String(name_sid), old_val, new_val, JsValue::Null];
            let cb_sid = ctx.vm.well_known.attribute_changed_callback;
            invoke_callback(&mut ctx, entity, cb_sid, &args);
        }
        CustomElementReaction::Adopted { .. } => {
            // v1 single-document VM never enqueues Adopted — the slot
            // is reserved by `#11-custom-elements-adopted-callback`.
            // Defensive no-op so an externally-injected Adopted does
            // not panic.
        }
    }
    Ok(())
}

/// Look up the named callback on `entity`'s constructor prototype and
/// invoke it with `args` (this = the element wrapper). Errors are
/// reported but not propagated — HTML §4.13.6 "Invoke custom element
/// reactions" routes them to Window.onerror.
fn invoke_callback(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    callback_sid: super::super::super::value::StringId,
    args: &[JsValue],
) {
    let Some(host) = ctx.host_if_bound() else {
        return;
    };
    let (definition_name, is_custom) =
        match host.dom_shared().world().get::<&CustomElementState>(entity) {
            Ok(state) => (
                state.definition_name.clone(),
                matches!(state.state, elidex_custom_elements::CEState::Custom),
            ),
            Err(_) => return,
        };
    if !is_custom {
        return;
    }
    let constructor_id_u64 = {
        let registry = host
            .ce_registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match registry.get(&definition_name) {
            Some(def) => def.constructor_id,
            None => return,
        }
    };
    let Some(constructor) = host.ce_constructors.get(&constructor_id_u64).copied() else {
        return;
    };
    // Resolve `constructor.prototype.<callback_sid>` — present iff the
    // CE class defines the lifecycle method.
    let proto_key = PropertyKey::String(ctx.vm.well_known.prototype);
    let proto_value = match ctx.vm.get_property_value(constructor, proto_key) {
        Ok(v) => v,
        Err(err) => {
            // Throwing prototype accessor / proxy `get` trap — report
            // via the same eprintln/Window.onerror path as runtime
            // callback throws below.
            eprintln!("[CE Callback Error] {}", err.message);
            return;
        }
    };
    let JsValue::Object(proto_obj) = proto_value else {
        // Non-Object constructor.prototype — TypeError per
        // HTML §4.13.5 (upgrade requires constructor.prototype to be
        // an object). Lifecycle dispatch can't proceed.
        eprintln!("[CE Callback Error] constructor.prototype is not an object");
        return;
    };
    let cb_value = match ctx
        .vm
        .get_property_value(proto_obj, PropertyKey::String(callback_sid))
    {
        Ok(v) => v,
        Err(err) => {
            eprintln!("[CE Callback Error] {}", err.message);
            return;
        }
    };
    // Absent lifecycle property (undefined / null) — silent no-op
    // per HTML §4.13.6 "invoke custom element callback" step 2
    // ("If callback is null, then return"). Optional lifecycle hooks
    // (e.g. a class that only defines `connectedCallback`) MUST NOT
    // log a TypeError for the missing siblings; that's the common
    // case, not an error.
    if matches!(cb_value, JsValue::Undefined | JsValue::Null) {
        return;
    }
    let JsValue::Object(cb_id) = cb_value else {
        // Present-but-non-object lifecycle property (e.g.
        // `connectedCallback = 1` / a primitive) — TypeError per
        // HTML §4.13.6 "invoke a custom element callback". Reported
        // via the same stderr / Window.onerror path as runtime
        // callback throws; not propagated because the reactions-
        // stack frame swallows individual errors.
        eprintln!("[CE Callback Error] non-callable lifecycle property");
        return;
    };
    if !ctx.vm.get_object(cb_id).kind.is_callable() {
        eprintln!("[CE Callback Error] non-callable lifecycle property");
        return;
    }
    let wrapper_id = ctx.vm.create_element_wrapper(entity);
    if let Err(err) = ctx.vm.call(cb_id, JsValue::Object(wrapper_id), args) {
        eprintln!("[CE Callback Error] {}", err.message);
    }
}
