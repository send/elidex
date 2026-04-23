//! Promise (ES2020 §25.6).
//!
//! Implements the core Promise state machine — constructor, static
//! `resolve`/`reject`, and `prototype.then` — with reactions dispatched via
//! the VM's microtask queue.  Thenable assimilation is limited to actual
//! `ObjectKind::Promise` values for now; arbitrary thenables (`{then: fn}`)
//! are queued up under the "Test262 alignment" follow-up described in the
//! PR2 plan.
//!
//! Entry points from outside this module:
//! - [`settle_promise`] — called from `interpreter::call` when a
//!   `PromiseResolver` object is invoked.
//! - [`create_promise`] — public helper used by the constructor.
//! - Registration lives in [`globals::register_promise_global`].

use super::shape;
use super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PromiseState, PromiseStatus,
    PropertyStorage, Reaction, ReactionKind, VmError,
};
use super::VmInner;

// ---------------------------------------------------------------------------
// Microtask queue payload
// ---------------------------------------------------------------------------

/// A queued microtask.  Lives here (rather than in `mod.rs`) because every
/// variant concerns Promise / queueMicrotask semantics and keeping the
/// enum next to its dispatch logic makes ownership clear.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Microtask {
    /// A pending Promise reaction: run `handler(resolution)` (or propagate
    /// the resolution directly if `handler` is `None`) and settle `capability`
    /// accordingly.  `capability` is `None` for reactions whose derived
    /// promise is never observed (the async-function driver and Promise
    /// combinator per-item subscribers), which skips allocation of that
    /// otherwise-wasted Promise.  Mirrors ES2020 §25.6.1.3
    /// `NewPromiseReactionJob`.
    PromiseReaction {
        kind: ReactionKind,
        handler: Option<ObjectId>,
        capability: Option<ObjectId>,
        resolution: JsValue,
    },
    /// A bare callback enqueued via `globalThis.queueMicrotask(fn)`
    /// (HTML §8.1.4.3).  Invoked with `this = undefined` and no arguments;
    /// exceptions are reported to the host and do not propagate out of the
    /// drain loop (spec: "If the callback throws an exception, report the
    /// exception.").
    Callback { func: ObjectId },
}

// ---------------------------------------------------------------------------
// Low-level helpers (crate-internal)
// ---------------------------------------------------------------------------

/// Create a fresh Pending Promise inheriting from `Promise.prototype`.
pub(super) fn create_promise(vm: &mut VmInner) -> ObjectId {
    let proto = vm.promise_prototype;
    vm.alloc_object(Object {
        kind: ObjectKind::Promise(PromiseState {
            status: PromiseStatus::Pending,
            result: JsValue::Undefined,
            fulfill_reactions: Vec::new(),
            reject_reactions: Vec::new(),
            handled: false,
            already_resolved: false,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Create a `(resolve, reject)` pair bound to `promise`.
pub(super) fn create_resolver_pair(vm: &mut VmInner, promise: ObjectId) -> (ObjectId, ObjectId) {
    let resolve = vm.alloc_object(Object {
        kind: ObjectKind::PromiseResolver {
            promise,
            is_reject: false,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.function_prototype,
        extensible: true,
    });
    let reject = vm.alloc_object(Object {
        kind: ObjectKind::PromiseResolver {
            promise,
            is_reject: true,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.function_prototype,
        extensible: true,
    });
    (resolve, reject)
}

/// Settle `promise` from an external `PromiseResolver` invocation.
///
/// Idempotent via the promise's `already_resolved` flag — the spec models
/// this as an `[[AlreadyResolved]]` record shared between the resolve /
/// reject pair (§25.6.1.3 step 2).  Checking `status == Pending` alone is
/// not sufficient: when `resolve(p2)` adopts a pending thenable, the
/// outer promise stays `Pending` until `p2` settles, but any subsequent
/// resolver call must still be a no-op.
///
/// Promise-value pass-through (`resolve(p2)` where `p2` is a Promise):
/// the spec (§25.6.1.3.2 PromiseResolveThenableJob) would schedule a
/// microtask to call `p2.then(resolve, reject)` asynchronously.  Here we
/// just register reactions synchronously via `then_impl`, which preserves
/// the observable "resolve with a pending promise ⇒ stay pending until
/// that promise settles" invariant.  Arbitrary thenables are not yet
/// assimilated — see the PR2 plan "Test262 alignment" note.
pub(super) fn settle_promise(
    vm: &mut VmInner,
    promise: ObjectId,
    is_reject: bool,
    value: JsValue,
) -> Result<JsValue, VmError> {
    // AlreadyResolved check: once any resolver call has fired (either
    // settling directly or adopting a pending thenable), subsequent calls
    // become no-ops even while `status` is still `Pending`.
    {
        let ObjectKind::Promise(state) = &mut vm.get_object_mut(promise).kind else {
            return Ok(JsValue::Undefined);
        };
        if state.already_resolved {
            return Ok(JsValue::Undefined);
        }
        state.already_resolved = true;
    }

    // resolve(thisPromise) — §25.6.1.3.2 step 7: reject with a
    // SelfResolutionError-ish TypeError (spec step 7.a).
    if !is_reject {
        if let JsValue::Object(resolution_id) = value {
            if resolution_id == promise {
                let err = VmError::type_error("Chaining cycle detected for promise");
                let reason = vm.vm_error_to_thrown(&err);
                reject_promise(vm, promise, reason);
                return Ok(JsValue::Undefined);
            }
            // If resolution is a Promise, wait for it to settle and then
            // propagate (equivalent to §25.6.1.3.2 thenable assimilation
            // for the restricted case where the thenable is a real Promise).
            if matches!(vm.get_object(resolution_id).kind, ObjectKind::Promise(_)) {
                forward_promise(vm, resolution_id, promise);
                return Ok(JsValue::Undefined);
            }
        }
    }

    if is_reject {
        reject_promise(vm, promise, value);
    } else {
        fulfill_promise(vm, promise, value);
    }
    Ok(JsValue::Undefined)
}

/// Transition `promise` to Fulfilled and queue each fulfill reaction.
fn fulfill_promise(vm: &mut VmInner, promise: ObjectId, value: JsValue) {
    let reactions = {
        let obj = vm.get_object_mut(promise);
        let ObjectKind::Promise(state) = &mut obj.kind else {
            return;
        };
        if state.status != PromiseStatus::Pending {
            return;
        }
        state.status = PromiseStatus::Fulfilled;
        state.result = value;
        // Swap both reaction vecs out with empty ones so we don't hold
        // refs to handler/capability objects beyond their scheduled run.
        let fulfill = std::mem::take(&mut state.fulfill_reactions);
        state.reject_reactions.clear();
        fulfill
    };
    for r in reactions {
        enqueue_reaction(vm, r, value);
    }
}

/// Transition `promise` to Rejected and queue each reject reaction.
///
/// Also tracks unhandled rejections: if no reject reactions were attached
/// when we settle, add the promise to `VmInner::pending_rejections` so the
/// end-of-drain check can warn about it.
fn reject_promise(vm: &mut VmInner, promise: ObjectId, reason: JsValue) {
    let (reactions, unhandled) = {
        let obj = vm.get_object_mut(promise);
        let ObjectKind::Promise(state) = &mut obj.kind else {
            return;
        };
        if state.status != PromiseStatus::Pending {
            return;
        }
        state.status = PromiseStatus::Rejected;
        state.result = reason;
        state.fulfill_reactions.clear();
        let taken = std::mem::take(&mut state.reject_reactions);
        // If we have no reactions to dispatch AND no prior .catch/.then(_, f)
        // has marked us handled, queue an unhandled-rejection check.
        let unhandled = taken.is_empty() && !state.handled;
        (taken, unhandled)
    };
    if unhandled {
        vm.pending_rejections.push(promise);
    }
    for r in reactions {
        enqueue_reaction(vm, r, reason);
    }
}

/// Subscribe `dst` to `src`'s settlement: when `src` settles (or
/// immediately, if already settled), propagate the result to `dst`.  Used
/// when the executor (or `Promise.resolve`) is passed a real Promise value.
fn forward_promise(vm: &mut VmInner, src: ObjectId, dst: ObjectId) {
    // Snapshot src's current state; if already settled, propagate via
    // microtask to preserve the async invariant.  Otherwise attach
    // reactions that will relay on settle.
    let (status, result) = {
        let ObjectKind::Promise(state) = &vm.get_object(src).kind else {
            return;
        };
        (state.status, state.result)
    };
    let relay = |kind| Reaction {
        kind,
        handler: None,
        capability: Some(dst),
    };
    match status {
        PromiseStatus::Fulfilled => enqueue_reaction(vm, relay(ReactionKind::Fulfill), result),
        PromiseStatus::Rejected => enqueue_reaction(vm, relay(ReactionKind::Reject), result),
        PromiseStatus::Pending => {
            if let ObjectKind::Promise(state) = &mut vm.get_object_mut(src).kind {
                state.fulfill_reactions.push(relay(ReactionKind::Fulfill));
                state.reject_reactions.push(relay(ReactionKind::Reject));
            }
        }
    }
}

/// Enqueue a single reaction as a PromiseReaction microtask.
fn enqueue_reaction(vm: &mut VmInner, reaction: Reaction, resolution: JsValue) {
    vm.microtask_queue.push_back(Microtask::PromiseReaction {
        kind: reaction.kind,
        handler: reaction.handler,
        capability: reaction.capability,
        resolution,
    });
}

// ---------------------------------------------------------------------------
// Microtask drain (used by interpreter::eval() and ScriptEngine::run_microtasks)
// ---------------------------------------------------------------------------

impl VmInner {
    /// Drain all pending microtasks.  Runs until the queue is empty,
    /// including microtasks enqueued by earlier microtasks in the same drain.
    /// Reentrancy-guarded: a nested `drain_microtasks` call is a no-op so
    /// that native functions or event listeners invoked from within a
    /// microtask do not reorder the rest of the queue.
    pub(crate) fn drain_microtasks(&mut self) {
        if self.microtask_drain_depth > 0 {
            return;
        }
        self.microtask_drain_depth += 1;
        while let Some(task) = self.microtask_queue.pop_front() {
            // Install the popped task as a GC root before invoking user
            // code — the callback we're about to run can trigger GC, and
            // without this slot the reaction's handler/capability/resolution
            // (or bare callback func) are only held in Rust locals and
            // would be collected.  `Microtask` is `Copy`, so we hold a
            // snapshot locally for dispatch while `current_microtask`
            // keeps the originals rooted.
            self.current_microtask = Some(task);
            match task {
                Microtask::PromiseReaction {
                    kind,
                    handler,
                    capability,
                    resolution,
                } => {
                    run_reaction(self, kind, handler, capability, resolution);
                }
                Microtask::Callback { func } => {
                    run_callback(self, func);
                }
            }
            self.current_microtask = None;
        }
        // End-of-drain: dispatch a `PromiseRejectionEvent` to the
        // document's `unhandledrejection` listeners (HTML §8.1.5.5
        // HostPromiseRejectionTracker hook), falling back to an
        // `eprintln!` when no listener calls `preventDefault`.  Wired
        // in PR3 C10.
        process_pending_rejections(self);
        self.microtask_drain_depth -= 1;
    }
}

/// Walk `pending_rejections`, dispatch a `PromiseRejectionEvent` to the
/// host (if bound) for each, and fall back to `eprintln!` when no
/// listener has called `preventDefault` (matches HTML §8.1.5.5
/// "report the exception" hook).  Marks the reported promise
/// `handled` so a second drain pass doesn't re-warn.
fn process_pending_rejections(vm: &mut VmInner) {
    if vm.pending_rejections.is_empty() {
        return;
    }
    // Iterate by index over the live `pending_rejections` Vec
    // instead of `mem::take`-ing it.  `pending_rejections` is part
    // of the GC root set (see `gc.rs::GcRoots::pending_rejections`),
    // so the rejected Promise objects stay marked across the
    // dispatch loop.  `dispatch_unhandled_rejection_event` allocates
    // (wrappers, event objects), each of which can trigger GC; if
    // we'd taken the Vec out, an otherwise-unreachable rejected
    // Promise (the common case — `Promise.reject('x')` with no JS
    // reference) would be reclaimed mid-loop, leaving `id` dangling
    // when we install `event.promise`.
    //
    // `initial_count` snapshots the boundary: any pending_rejections
    // pushed *during* the loop (a listener creating + rejecting
    // another Promise) wait for the next outer drain — matches the
    // microtask-checkpoint semantics in `script_dispatch_event_core`.
    let initial_count = vm.pending_rejections.len();
    for i in 0..initial_count {
        let id = vm.pending_rejections[i];
        let Some(obj) = vm.objects.get(id.0 as usize).and_then(|o| o.as_ref()) else {
            continue;
        };
        let ObjectKind::Promise(state) = &obj.kind else {
            continue;
        };
        if state.status != PromiseStatus::Rejected || state.handled {
            continue;
        }
        let reason = state.result;

        // Try host-side dispatch first; only fall back to stderr if
        // no listener prevented the default.
        let suppressed = dispatch_unhandled_rejection_event(vm, id, reason);
        if !suppressed {
            // Format the reason for display.  Intern via `to_display_string`
            // so Error instances render as "TypeError: msg" etc.
            let reason_id = super::coerce::to_display_string(vm, reason);
            let reason_str = vm.strings.get_utf8(reason_id);
            eprintln!("Uncaught (in promise): {reason_str}");
        }
        // Re-borrow mutably now that we're done reading.
        if let Some(ObjectKind::Promise(state)) = vm
            .objects
            .get_mut(id.0 as usize)
            .and_then(|o| o.as_mut())
            .map(|o| &mut o.kind)
        {
            state.handled = true;
        }
    }
    // Remove the entries we processed.  Anything pushed during the
    // loop (idx >= initial_count) survives for the next drain.
    vm.pending_rejections.drain(..initial_count);
}

/// Dispatch a `PromiseRejectionEvent` to the document's
/// `unhandledrejection` listeners.  Returns `true` if dispatch
/// happened AND a listener called `preventDefault()` — caller uses
/// this to suppress the stderr fallback.
///
/// Bypasses the shared 3-phase dispatch core: per HTML §8.1.5.5 the
/// event is non-bubbling and targeted at the global, so capture-phase
/// listeners on ancestors are not in play.  Direct invocation also
/// avoids the cross-crate `EventPayload::PromiseRejection { ... }`
/// variant that would be required to thread VM-specific `ObjectId`
/// and `JsValue` through `elidex-plugin`'s engine-agnostic payload
/// enum.
#[cfg(feature = "engine")]
fn dispatch_unhandled_rejection_event(
    vm: &mut VmInner,
    promise_id: super::value::ObjectId,
    reason: JsValue,
) -> bool {
    use super::shape::PropertyAttrs;
    use super::value::{PropertyKey, PropertyValue};

    // `HostData` must be bound and the document entity must have at
    // least one `unhandledrejection` listener — otherwise nothing to
    // dispatch.  Cheap pre-checks done first so the no-listener case
    // bails before any allocation.
    let document = match vm.host_data.as_deref() {
        Some(host) if host.is_bound() => host.document(),
        _ => return false,
    };
    // Collect the full (id, once, passive) tuple per matching
    // listener — `once` gates ECS-component + listener_store
    // removal after firing, `passive` threads through to the event
    // object so `preventDefault` no-ops correctly.  Keeping the
    // metadata captured at plan-build time mirrors
    // `script_dispatch_event_core`'s behaviour and prevents the bug
    // where `{once: true, passive: true}` listeners on
    // unhandledrejection silently bypassed both options.
    struct PendingListener {
        id: elidex_script_session::ListenerId,
        once: bool,
        passive: bool,
    }
    let pending: Vec<PendingListener> = {
        let dom = vm.host_data.as_deref_mut().unwrap().dom();
        let Ok(listeners) = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(document)
        else {
            return false;
        };
        // `EventListeners::iter_matching` takes `&str` (the ECS
        // component stores Rust `String`s, not `StringId`s) — the
        // literal here matches the cached `well_known.unhandledrejection`
        // by definition.  Replacing with `get_utf8(well_known.…)` would
        // allocate a `String` per call, defeating the cache.
        // `iter_matching` (vs `matching_all`) skips the
        // `Vec<&ListenerEntry>` intermediate alloc.
        listeners
            .iter_matching("unhandledrejection")
            .map(|e| PendingListener {
                id: e.id,
                once: e.once,
                passive: e.passive,
            })
            .collect()
    };
    if pending.is_empty() {
        return false;
    }

    // Build a synthetic DispatchEvent — flag state is updated across
    // iterations so the next listener's freshly-built event object
    // sees prior `preventDefault` / `stopPropagation` calls.  Never
    // enters the session event queue or 3-phase dispatch; the loop
    // honours `stopImmediatePropagation` directly.
    //
    // Initialise `phase` / `current_target` / `dispatch_flag` /
    // `composed_path` to spec-consistent at-target state so JS
    // observers see `e.eventPhase === 2` (AT_TARGET), `e.currentTarget
    // === document`, and `e.composedPath() === [document]` — matching
    // what a regular dispatch through `script_dispatch_event_core`
    // would produce.  `create_event_object` resolves
    // `composed_path`'s Entity list to `HostObject` wrappers and
    // seeds the Event's internal slot.
    let mut event = elidex_script_session::DispatchEvent::new("unhandledrejection", document);
    event.bubbles = false;
    event.cancelable = true;
    event.composed = false;
    event.phase = elidex_plugin::EventPhase::AtTarget;
    event.current_target = Some(document);
    event.dispatch_flag = true;
    event.composed_path = vec![document];

    let doc_wrapper = vm.create_element_wrapper(document);
    let promise_key = vm.well_known.promise;
    let reason_key = vm.well_known.reason;

    // Per-listener rebuild (matches design D4 + `engine.rs::call_listener`).
    // Reusing one event object across listeners would leak listener-side
    // mutations (e.g. `e.foo = 1`) into the next listener's view, which
    // diverges from the regular dispatch path.
    for entry in pending {
        let Some(func_id) = vm
            .host_data
            .as_deref()
            .and_then(|h| h.get_listener(entry.id))
        else {
            continue;
        };

        // WHATWG DOM §2.10 step 15: remove `once` listeners BEFORE
        // invoking, so re-entrant dispatch in the callback doesn't
        // re-fire them.  Mirrors `script_dispatch_event_core` line 452.
        if entry.once {
            if let Ok(mut listeners) = vm
                .host_data
                .as_deref_mut()
                .unwrap()
                .dom()
                .world_mut()
                .get::<&mut elidex_script_session::EventListeners>(document)
            {
                listeners.remove(entry.id);
            }
        }

        // Build fresh event object for this listener.  `event.flags`
        // carries forward accumulated state from previous iterations.
        // `entry.passive` threads through so `preventDefault` no-ops
        // for `{passive: true}` listeners.
        let event_obj_id = vm.create_event_object(&event, doc_wrapper, doc_wrapper, entry.passive);
        // Augment with the spec-required `promise` + `reason` props.
        vm.define_shaped_property(
            event_obj_id,
            PropertyKey::String(promise_key),
            PropertyValue::Data(JsValue::Object(promise_id)),
            PropertyAttrs::WEBIDL_RO,
        );
        vm.define_shaped_property(
            event_obj_id,
            PropertyKey::String(reason_key),
            PropertyValue::Data(reason),
            PropertyAttrs::WEBIDL_RO,
        );

        // Root + invoke + sync flags back into DispatchEvent.
        let mut g = vm.push_temp_root(JsValue::Object(event_obj_id));
        // Errors swallowed — dispatch is a fire-and-forget host hook.
        let _ = g.call(
            func_id,
            JsValue::Object(doc_wrapper),
            &[JsValue::Object(event_obj_id)],
        );
        let mut should_break = false;
        if let ObjectKind::Event {
            default_prevented,
            propagation_stopped,
            immediate_propagation_stopped,
            ..
        } = g.get_object(event_obj_id).kind
        {
            event.flags.default_prevented = default_prevented;
            event.flags.propagation_stopped = propagation_stopped;
            event.flags.immediate_propagation_stopped = immediate_propagation_stopped;
            should_break = immediate_propagation_stopped;
        }
        drop(g);

        // Clean up the `listener_store` entry for once-listeners now
        // that the call returned (paired with the ECS-side removal
        // above).  Together they prevent the leak where a function
        // ObjectId stays GC-rooted via listener_store after its ECS
        // entry is gone.
        if entry.once {
            vm.host_data
                .as_deref_mut()
                .unwrap()
                .remove_listener(entry.id);
        }
        if should_break {
            break;
        }
    }

    event.flags.default_prevented
}

/// Stub for builds without the `engine` feature — no host means no
/// listeners, so `false` always; the caller falls back to stderr.
#[cfg(not(feature = "engine"))]
#[allow(clippy::needless_pass_by_value)]
fn dispatch_unhandled_rejection_event(
    _vm: &mut VmInner,
    _promise_id: super::value::ObjectId,
    _reason: JsValue,
) -> bool {
    false
}

/// Execute a bare `queueMicrotask` callback (HTML §8.1.4.3).  Exceptions are
/// swallowed with a best-effort `eprintln!` report so that a misbehaving
/// callback cannot abort the drain loop and strand the rest of the queue.
/// Once a proper host error-reporting channel exists (PR6), the eprintln
/// should be swapped for `host.session().report_error(...)`.
fn run_callback(vm: &mut VmInner, func: ObjectId) {
    if let Err(e) = vm.call(func, JsValue::Undefined, &[]) {
        eprintln!("queueMicrotask callback threw: {e}");
    }
}

/// Execute a single PromiseReaction (ES2020 §25.6.1.3 NewPromiseReactionJob).
///
/// - If the reaction has a handler, call `handler(resolution)`:
///   - success ⇒ resolve `capability` with the return value
///   - throw ⇒ reject `capability` with the thrown value
/// - No handler ⇒ propagate the resolution directly: Fulfill-kind
///   resolves the capability; Reject-kind rejects it (spec default
///   passthrough behaviour).
/// - `capability` is `None` for internal subscribers (async-function
///   driver / combinator per-item reactions) that never observe the
///   derived promise — the handler runs for its side effect and the
///   result is discarded.
fn run_reaction(
    vm: &mut VmInner,
    kind: ReactionKind,
    handler: Option<ObjectId>,
    capability: Option<ObjectId>,
    resolution: JsValue,
) {
    let Some(handler) = handler else {
        // Default passthrough — Fulfill propagates the resolution, Reject
        // propagates as a rejection reason.  This path is how
        // `forward_promise` relays a settled source to an outer promise
        // whose resolver has already fired (so `already_resolved` is set);
        // the gate in `settle_promise` would reject the relay.
        // `forward_promise` always supplies a capability; the `None`
        // branch here is defensive.
        if let Some(cap) = capability {
            if kind == ReactionKind::Reject {
                reject_promise(vm, cap, resolution);
            } else {
                fulfill_promise(vm, cap, resolution);
            }
        }
        return;
    };
    match vm.call(handler, JsValue::Undefined, &[resolution]) {
        Ok(value) => {
            if let Some(cap) = capability {
                let _ = settle_promise(vm, cap, false, value);
            }
        }
        Err(e) => {
            if let Some(cap) = capability {
                let thrown = vm.vm_error_to_thrown(&e);
                let _ = settle_promise(vm, cap, true, thrown);
            }
            // No capability → internal caller (async driver, combinator
            // subscribe) owns any error semantics; handlers for those
            // paths are fn-pointer natives that drive their own state.
        }
    }
}

// ---------------------------------------------------------------------------
// Native functions exposed to JS
// ---------------------------------------------------------------------------

/// `new Promise(executor)` — §25.6.3.1
pub(super) fn native_promise_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Promise constructor cannot be invoked without 'new'",
        ));
    }
    let executor = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(executor_id) = executor else {
        return Err(VmError::type_error("Promise resolver is not a function"));
    };
    if !ctx.get_object(executor_id).kind.is_callable() {
        return Err(VmError::type_error("Promise resolver is not a function"));
    }

    // `do_new` has already pre-allocated an Ordinary instance at `this`.  We
    // repurpose that slot as the Promise to avoid a second allocation — the
    // prototype was already wired from `Promise.prototype`.
    let promise_id = match this {
        JsValue::Object(id) => {
            let obj = ctx.vm.get_object_mut(id);
            obj.kind = ObjectKind::Promise(PromiseState {
                status: PromiseStatus::Pending,
                result: JsValue::Undefined,
                fulfill_reactions: Vec::new(),
                reject_reactions: Vec::new(),
                handled: false,
                already_resolved: false,
            });
            id
        }
        _ => create_promise(ctx.vm),
    };

    let (resolve, reject) = create_resolver_pair(ctx.vm, promise_id);
    // Executor runs synchronously with `this = undefined` (§25.6.3.1 step 9).
    let exec_args = [JsValue::Object(resolve), JsValue::Object(reject)];
    let exec_result = ctx.call_function(executor_id, JsValue::Undefined, &exec_args);
    if let Err(e) = exec_result {
        // If the executor throws, reject the promise with the thrown value
        // (spec step 10).  If the promise was already settled (executor
        // resolved before throwing), this is a no-op.
        let thrown = ctx.vm.vm_error_to_thrown(&e);
        let _ = settle_promise(ctx.vm, promise_id, true, thrown);
    }
    Ok(JsValue::Object(promise_id))
}

/// `Promise.resolve(value)` — §25.6.4.7 (ES2021; `Promise.any` shifted
/// this one down from §25.6.4.5 in earlier editions).
pub(super) fn native_promise_resolve(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    // Pass-through for Promise instances (§25.6.4.7.1 step 2 when C === %Promise%).
    if let JsValue::Object(id) = value {
        if matches!(ctx.get_object(id).kind, ObjectKind::Promise(_)) {
            return Ok(value);
        }
    }
    let id = create_promise(ctx.vm);
    let _ = settle_promise(ctx.vm, id, false, value);
    Ok(JsValue::Object(id))
}

/// `Promise.reject(reason)` — §25.6.4.6 (ES2021; `Promise.any` at §25.6.4.3
/// shifted `.reject` / `.resolve` down by two).
pub(super) fn native_promise_reject(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    let id = create_promise(ctx.vm);
    let _ = settle_promise(ctx.vm, id, true, reason);
    Ok(JsValue::Object(id))
}

/// `Promise.prototype.then(onFulfilled, onRejected)` — §25.6.5.4
pub(super) fn native_promise_prototype_then(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(src) = this else {
        return Err(VmError::type_error(
            "Promise.prototype.then called on non-object",
        ));
    };
    if !matches!(ctx.get_object(src).kind, ObjectKind::Promise(_)) {
        return Err(VmError::type_error(
            "Promise.prototype.then called on non-Promise",
        ));
    }
    let on_fulfilled = coerce_then_handler(ctx, args.first().copied())?;
    let on_rejected = coerce_then_handler(ctx, args.get(1).copied())?;

    then_impl(ctx.vm, src, on_fulfilled, on_rejected)
}

/// `Promise.prototype.catch(onRejected)` — §25.6.5.1
/// Implemented as sugar for `then(undefined, onRejected)`.
pub(super) fn native_promise_prototype_catch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(src) = this else {
        return Err(VmError::type_error(
            "Promise.prototype.catch called on non-object",
        ));
    };
    if !matches!(ctx.get_object(src).kind, ObjectKind::Promise(_)) {
        return Err(VmError::type_error(
            "Promise.prototype.catch called on non-Promise",
        ));
    }
    let on_rejected = coerce_then_handler(ctx, args.first().copied())?;
    then_impl(ctx.vm, src, None, on_rejected)
}

/// Validate a `then` argument: non-callable values are ignored (treated as
/// `None` which activates the default passthrough).  Spec §25.6.5.4 steps
/// 3–4 use `IsCallable(onFulfilled)` the same way.
fn coerce_then_handler(
    ctx: &NativeContext<'_>,
    val: Option<JsValue>,
) -> Result<Option<ObjectId>, VmError> {
    let Some(v) = val else { return Ok(None) };
    let JsValue::Object(id) = v else {
        return Ok(None);
    };
    if ctx.get_object(id).kind.is_callable() {
        Ok(Some(id))
    } else {
        Ok(None)
    }
}

/// `queueMicrotask(callback)` — HTML §8.1.4.3.
///
/// Validates the callback is callable and appends a `Microtask::Callback`
/// to the VM queue.  Drain happens at the next microtask checkpoint (end
/// of `eval`, end of event listener invocation, etc.).
pub(super) fn native_queue_microtask(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let callback = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(func) = callback else {
        return Err(VmError::type_error(
            "queueMicrotask argument is not a function",
        ));
    };
    if !ctx.get_object(func).kind.is_callable() {
        return Err(VmError::type_error(
            "queueMicrotask argument is not a function",
        ));
    }
    ctx.vm
        .microtask_queue
        .push_back(Microtask::Callback { func });
    Ok(JsValue::Undefined)
}

/// Thin wrapper around [`then_impl`] for callers that already hold the
/// fulfil/reject handler ObjectIds (e.g. async-function driver
/// continuations).  Skips the derived-promise allocation — the internal
/// handlers settle the wrapper Promise directly, so the capability
/// would be immediately unreachable anyway.
pub(super) fn subscribe_then(
    vm: &mut VmInner,
    src: ObjectId,
    on_fulfilled: ObjectId,
    on_rejected: ObjectId,
) {
    // `then_impl_internal` only errors if the src isn't a Promise;
    // callers here are expected to have verified that already.
    let _ = then_impl_internal(vm, src, Some(on_fulfilled), Some(on_rejected), None);
}

/// User-visible `.then` entry: always allocates a derived Promise and
/// returns it as the result of the call.
pub(super) fn then_impl(
    vm: &mut VmInner,
    src: ObjectId,
    on_fulfilled: Option<ObjectId>,
    on_rejected: Option<ObjectId>,
) -> Result<JsValue, VmError> {
    let capability = create_promise(vm);
    then_impl_internal(vm, src, on_fulfilled, on_rejected, Some(capability))?;
    Ok(JsValue::Object(capability))
}

/// Internal entry: registers reactions against `src`, optionally with a
/// derived Promise `capability` that settles on each reaction's result.
/// `capability = None` elides the wasted Promise allocation for internal
/// subscribers (async driver continuations, Promise combinator per-item
/// reactions) that don't observe the derived promise.
///
/// The capability-bearing path is the user-visible `.then` / `.catch` /
/// `.finally`; those go through [`then_impl`] which always supplies
/// `Some`.
pub(super) fn then_impl_internal(
    vm: &mut VmInner,
    src: ObjectId,
    on_fulfilled: Option<ObjectId>,
    on_rejected: Option<ObjectId>,
    capability: Option<ObjectId>,
) -> Result<(), VmError> {
    let fulfill_r = Reaction {
        kind: ReactionKind::Fulfill,
        handler: on_fulfilled,
        capability,
    };
    let reject_r = Reaction {
        kind: ReactionKind::Reject,
        handler: on_rejected,
        capability,
    };

    // Inspect the source promise's status; if still Pending, attach
    // reactions.  Otherwise queue the matching reaction immediately so that
    // `.then` on an already-settled promise still fires asynchronously.
    let (status, resolution) = {
        let ObjectKind::Promise(state) = &mut vm.get_object_mut(src).kind else {
            unreachable!("then_impl_internal caller verified Promise kind");
        };
        // Mark as handled once any reject reaction is attached so the
        // end-of-drain unhandled-rejection scan doesn't warn.
        if on_rejected.is_some() {
            state.handled = true;
        }
        match state.status {
            PromiseStatus::Pending => {
                state.fulfill_reactions.push(fulfill_r);
                state.reject_reactions.push(reject_r);
                (PromiseStatus::Pending, JsValue::Undefined)
            }
            other => (other, state.result),
        }
    };
    match status {
        PromiseStatus::Fulfilled => enqueue_reaction(vm, fulfill_r, resolution),
        PromiseStatus::Rejected => enqueue_reaction(vm, reject_r, resolution),
        PromiseStatus::Pending => {}
    }
    Ok(())
}
