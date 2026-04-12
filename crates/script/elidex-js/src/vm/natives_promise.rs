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

use super::shape::{self, PropertyAttrs};
use super::value::{
    CombinatorKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PromiseCombinatorState,
    PromiseCombinatorStep, PromiseState, PromiseStatus, PropertyKey, PropertyStorage,
    PropertyValue, Reaction, ReactionKind, VmError, VmErrorKind,
};
use super::{Microtask, VmInner};

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
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Create a `(resolve, reject)` pair bound to `promise`.
fn create_resolver_pair(vm: &mut VmInner, promise: ObjectId) -> (ObjectId, ObjectId) {
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
/// Idempotent via the promise's own status field: if `status != Pending`
/// the call is a no-op (spec `[[AlreadyResolved]]` check in §25.6.1.3.1/3.2).
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
    // AlreadyResolved check: once the promise has settled (or is being
    // resolved through a nested thenable), further resolve/reject calls
    // become no-ops.
    match &vm.get_object(promise).kind {
        ObjectKind::Promise(state) if state.status == PromiseStatus::Pending => {}
        _ => return Ok(JsValue::Undefined),
    }

    // resolve(thisPromise) — §25.6.1.3.2 step 7: reject with a
    // SelfResolutionError-ish TypeError (spec step 7.a).
    if !is_reject {
        if let JsValue::Object(resolution_id) = value {
            if resolution_id == promise {
                let err = build_type_error(vm, "Chaining cycle detected for promise");
                reject_promise(vm, promise, err);
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

/// Build a TypeError instance matching the shape used elsewhere in the VM
/// (`ObjectKind::Error { name }` + `.name` / `.message` data properties).
/// Returned as a `JsValue::Object` ready to use as a rejection reason.
pub(super) fn build_type_error(vm: &mut VmInner, message: &str) -> JsValue {
    let name_id = vm.strings.intern("TypeError");
    let msg_id = vm.strings.intern(message);
    let obj = vm.alloc_object(Object {
        kind: ObjectKind::Error { name: name_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let name_key = PropertyKey::String(vm.well_known.name);
    vm.define_shaped_property(
        obj,
        name_key,
        PropertyValue::Data(JsValue::String(name_id)),
        PropertyAttrs::DATA,
    );
    let message_key = PropertyKey::String(vm.well_known.message);
    vm.define_shaped_property(
        obj,
        message_key,
        PropertyValue::Data(JsValue::String(msg_id)),
        PropertyAttrs::DATA,
    );
    JsValue::Object(obj)
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
    match status {
        PromiseStatus::Fulfilled => {
            vm.microtask_queue.push_back(Microtask::PromiseReaction {
                kind: ReactionKind::Fulfill,
                handler: None,
                capability: dst,
                resolution: result,
            });
        }
        PromiseStatus::Rejected => {
            vm.microtask_queue.push_back(Microtask::PromiseReaction {
                kind: ReactionKind::Reject,
                handler: None,
                capability: dst,
                resolution: result,
            });
        }
        PromiseStatus::Pending => {
            let fulfill_r = Reaction {
                kind: ReactionKind::Fulfill,
                handler: None,
                capability: dst,
            };
            let reject_r = Reaction {
                kind: ReactionKind::Reject,
                handler: None,
                capability: dst,
            };
            if let ObjectKind::Promise(state) = &mut vm.get_object_mut(src).kind {
                state.fulfill_reactions.push(fulfill_r);
                state.reject_reactions.push(reject_r);
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
        }
        // End-of-drain: emit unhandled-rejection warnings.  The spec hook
        // (HostPromiseRejectionTracker → PromiseRejectionEvent) is deferred
        // to PR3 when event dispatch is wired up; for now an eprintln keeps
        // the diagnostic visible during development.
        warn_unhandled_rejections(self);
        self.microtask_drain_depth -= 1;
    }
}

/// Walk `pending_rejections`, warn on any entry still unhandled, and clear
/// the list.  Marks the reported promise `handled` so a second drain pass
/// doesn't re-warn.
fn warn_unhandled_rejections(vm: &mut VmInner) {
    if vm.pending_rejections.is_empty() {
        return;
    }
    let pending = std::mem::take(&mut vm.pending_rejections);
    for id in pending {
        let Some(obj) = vm.objects.get(id.0 as usize).and_then(|o| o.as_ref()) else {
            continue;
        };
        let ObjectKind::Promise(state) = &obj.kind else {
            continue;
        };
        if state.status != PromiseStatus::Rejected || state.handled {
            continue;
        }
        // Format the reason for display.  Intern via `to_display_string`
        // so Error instances render as "TypeError: msg" etc.
        let reason = state.result;
        let reason_id = super::coerce::to_display_string(vm, reason);
        let reason_str = vm.strings.get_utf8(reason_id);
        eprintln!("Uncaught (in promise): {reason_str}");
        // Re-borrow mutably now that to_display_string/get_utf8 are done.
        if let Some(ObjectKind::Promise(state)) = vm
            .objects
            .get_mut(id.0 as usize)
            .and_then(|o| o.as_mut())
            .map(|o| &mut o.kind)
        {
            state.handled = true;
        }
    }
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
fn run_reaction(
    vm: &mut VmInner,
    kind: ReactionKind,
    handler: Option<ObjectId>,
    capability: ObjectId,
    resolution: JsValue,
) {
    let Some(handler) = handler else {
        // Default passthrough — Fulfill propagates the resolution, Reject
        // propagates as a rejection reason.
        let _ = settle_promise(vm, capability, kind == ReactionKind::Reject, resolution);
        return;
    };
    match vm.call(handler, JsValue::Undefined, &[resolution]) {
        Ok(value) => {
            let _ = settle_promise(vm, capability, false, value);
        }
        Err(e) => {
            let thrown = thrown_value(vm, &e);
            let _ = settle_promise(vm, capability, true, thrown);
        }
    }
}

/// Extract the JS-visible reason from a [`VmError`].  Non-`ThrowValue`
/// errors (e.g. internal TypeErrors raised by native builtins) are surfaced
/// as interned strings — the same bridge used elsewhere when a Rust-side
/// error has to become a JS reason.
fn thrown_value(vm: &mut VmInner, e: &VmError) -> JsValue {
    if let VmErrorKind::ThrowValue(v) = &e.kind {
        *v
    } else {
        let msg = vm.strings.intern(&e.to_string());
        JsValue::String(msg)
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
        let thrown = thrown_value(ctx.vm, &e);
        let _ = settle_promise(ctx.vm, promise_id, true, thrown);
    }
    Ok(JsValue::Object(promise_id))
}

/// `Promise.resolve(value)` — §25.6.4.5
pub(super) fn native_promise_resolve(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    // Pass-through for Promise instances (§25.6.4.5.1 step 2 when C === %Promise%).
    if let JsValue::Object(id) = value {
        if matches!(ctx.get_object(id).kind, ObjectKind::Promise(_)) {
            return Ok(value);
        }
    }
    let id = create_promise(ctx.vm);
    let _ = settle_promise(ctx.vm, id, false, value);
    Ok(JsValue::Object(id))
}

/// `Promise.reject(reason)` — §25.6.4.4
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
/// fulfil/reject handler ObjectIds (e.g. async driver continuations).
pub(super) fn subscribe_then(
    vm: &mut VmInner,
    src: ObjectId,
    on_fulfilled: ObjectId,
    on_rejected: ObjectId,
) {
    // `then_impl` only errors if the src isn't a Promise; callers here
    // are expected to have verified that already.
    let _ = then_impl(vm, src, Some(on_fulfilled), Some(on_rejected));
}

fn then_impl(
    vm: &mut VmInner,
    src: ObjectId,
    on_fulfilled: Option<ObjectId>,
    on_rejected: Option<ObjectId>,
) -> Result<JsValue, VmError> {
    let capability = create_promise(vm);

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
            unreachable!("then_impl caller verified Promise kind");
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
    Ok(JsValue::Object(capability))
}

// ---------------------------------------------------------------------------
// Combinators: Promise.all / allSettled / race / any + prototype.finally
// ---------------------------------------------------------------------------

/// Allocate a fresh `PromiseCombinatorState` object.  Pre-fills `values`
/// with `Undefined` placeholders so each step can write its own slot
/// without further resizing.
fn alloc_combinator_state(
    vm: &mut VmInner,
    kind: CombinatorKind,
    result: ObjectId,
    total: u32,
) -> ObjectId {
    let placeholder = vec![JsValue::Undefined; total as usize];
    vm.alloc_object(Object {
        kind: ObjectKind::PromiseCombinatorState(PromiseCombinatorState {
            kind,
            result,
            values: placeholder,
            remaining: total,
            total,
            settled: false,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: false,
    })
}

/// Allocate a step object as a standalone callable.
fn alloc_step(vm: &mut VmInner, step: PromiseCombinatorStep) -> ObjectId {
    let proto = vm.function_prototype;
    vm.alloc_object(Object {
        kind: ObjectKind::PromiseCombinatorStep(step),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Invoke a combinator step on `value`.  Mutates the shared state, and
/// settles the result promise once the last step has run.
pub(super) fn step_combinator(
    vm: &mut VmInner,
    step: PromiseCombinatorStep,
    value: JsValue,
) -> Result<JsValue, VmError> {
    use PromiseCombinatorStep as Step;

    let state_id = match step {
        Step::AllFulfill { state, .. }
        | Step::AllReject { state }
        | Step::AllSettledFulfill { state, .. }
        | Step::AllSettledReject { state, .. }
        | Step::AnyFulfill { state }
        | Step::AnyReject { state, .. } => state,
    };

    match step {
        Step::AllFulfill { index, .. } => {
            let (result, finished, values) = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.values[index as usize] = value;
                state.remaining -= 1;
                if state.remaining == 0 {
                    state.settled = true;
                    (state.result, true, std::mem::take(&mut state.values))
                } else {
                    (state.result, false, Vec::new())
                }
            };
            if finished {
                let arr = vm.create_array_object(values);
                let _ = settle_promise(vm, result, false, JsValue::Object(arr));
            }
        }
        Step::AllReject { .. } => {
            let result = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.settled = true;
                state.result
            };
            let _ = settle_promise(vm, result, true, value);
        }
        Step::AllSettledFulfill { index, .. } => {
            let entry = make_settled_entry(vm, true, value);
            settle_all_settled_slot(vm, state_id, index, entry);
        }
        Step::AllSettledReject { index, .. } => {
            let entry = make_settled_entry(vm, false, value);
            settle_all_settled_slot(vm, state_id, index, entry);
        }
        Step::AnyFulfill { .. } => {
            let result = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.settled = true;
                state.result
            };
            let _ = settle_promise(vm, result, false, value);
        }
        Step::AnyReject { index, .. } => {
            let (result, finished, errors) = {
                let ObjectKind::PromiseCombinatorState(state) =
                    &mut vm.get_object_mut(state_id).kind
                else {
                    return Ok(JsValue::Undefined);
                };
                if state.settled {
                    return Ok(JsValue::Undefined);
                }
                state.values[index as usize] = value;
                state.remaining -= 1;
                if state.remaining == 0 {
                    state.settled = true;
                    (state.result, true, std::mem::take(&mut state.values))
                } else {
                    (state.result, false, Vec::new())
                }
            };
            if finished {
                let agg = build_aggregate_error(vm, errors);
                let _ = settle_promise(vm, result, true, agg);
            }
        }
    }
    Ok(JsValue::Undefined)
}

/// Build a `{status: ..., value|reason: ...}` result object used by
/// `Promise.allSettled`.  Uses an Ordinary object with Dictionary storage
/// to avoid allocating a dedicated shape for every entry.
fn make_settled_entry(vm: &mut VmInner, fulfilled: bool, value: JsValue) -> JsValue {
    let obj = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let status_key = PropertyKey::String(vm.strings.intern("status"));
    let status_str = if fulfilled { "fulfilled" } else { "rejected" };
    let status_val = JsValue::String(vm.strings.intern(status_str));
    vm.define_shaped_property(
        obj,
        status_key,
        PropertyValue::Data(status_val),
        PropertyAttrs::DATA,
    );
    let payload_name = if fulfilled { "value" } else { "reason" };
    let payload_key = PropertyKey::String(vm.strings.intern(payload_name));
    vm.define_shaped_property(
        obj,
        payload_key,
        PropertyValue::Data(value),
        PropertyAttrs::DATA,
    );
    JsValue::Object(obj)
}

/// Shared tail for `AllSettledFulfill` / `AllSettledReject`: write the
/// `{status,value|reason}` entry at `index`, dec the counter, and resolve
/// when every slot has arrived.
fn settle_all_settled_slot(vm: &mut VmInner, state_id: ObjectId, index: u32, entry: JsValue) {
    let (result, finished, values) = {
        let ObjectKind::PromiseCombinatorState(state) = &mut vm.get_object_mut(state_id).kind
        else {
            return;
        };
        if state.settled {
            return;
        }
        state.values[index as usize] = entry;
        state.remaining -= 1;
        if state.remaining == 0 {
            state.settled = true;
            (state.result, true, std::mem::take(&mut state.values))
        } else {
            (state.result, false, Vec::new())
        }
    };
    if finished {
        let arr = vm.create_array_object(values);
        let _ = settle_promise(vm, result, false, JsValue::Object(arr));
    }
}

/// Build an `AggregateError` for `Promise.any` when every input rejects.
/// The shape here is a minimal `Error` object carrying `.errors` and a
/// fixed message — full AggregateError wiring (inheritance chain, proper
/// `[[Prototype]]`) comes with the rest of the Error cleanup in PR4.
fn build_aggregate_error(vm: &mut VmInner, errors: Vec<JsValue>) -> JsValue {
    let name_id = vm.strings.intern("AggregateError");
    let obj = vm.alloc_object(Object {
        kind: ObjectKind::Error { name: name_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let name_key = PropertyKey::String(vm.well_known.name);
    vm.define_shaped_property(
        obj,
        name_key,
        PropertyValue::Data(JsValue::String(name_id)),
        PropertyAttrs::DATA,
    );
    let message_key = PropertyKey::String(vm.well_known.message);
    let message_val = JsValue::String(vm.strings.intern("All promises were rejected"));
    vm.define_shaped_property(
        obj,
        message_key,
        PropertyValue::Data(message_val),
        PropertyAttrs::DATA,
    );
    let errors_arr = vm.create_array_object(errors);
    let errors_key = PropertyKey::String(vm.strings.intern("errors"));
    vm.define_shaped_property(
        obj,
        errors_key,
        PropertyValue::Data(JsValue::Object(errors_arr)),
        PropertyAttrs::DATA,
    );
    JsValue::Object(obj)
}

/// Shared body for `Promise.all` / `allSettled` / `any` / `race`.  Reads
/// the iterable, allocates a result promise + (optional) aggregator state,
/// and subscribes per-item reactions via `.then(...)`.  `race` passes
/// `None` for `kind` and uses outer resolve/reject directly.
fn run_combinator(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    kind: Option<CombinatorKind>,
) -> Result<JsValue, VmError> {
    let iterable = args.first().copied().unwrap_or(JsValue::Undefined);

    let iterator = match ctx.vm.resolve_iterator(iterable)? {
        Some(JsValue::Object(id)) => JsValue::Object(id),
        Some(_) => return Err(VmError::type_error("@@iterator must return an object")),
        None => {
            return Err(VmError::type_error(
                "Promise.<combinator> input is not iterable",
            ))
        }
    };

    let result = create_promise(ctx.vm);
    let (resolve, reject) = create_resolver_pair(ctx.vm, result);

    // Collect all items into a buffer; step allocation needs to know
    // `total` up front to pre-size the state's values vec.  This also
    // matches the spec's eager `IteratorStep` loop — values are awaited
    // via `.then` attachment, not pulled lazily.
    let items = collect_items(ctx.vm, iterator)?;
    let total = u32::try_from(items.len())
        .map_err(|_| VmError::range_error("Promise combinator input exceeded u32 length limit"))?;

    // Empty iterable: spec-specific resolution.  For all/allSettled, resolve
    // immediately with []; for any, reject immediately with an empty
    // AggregateError; for race, stay Pending forever (resolve/reject never
    // called).  Returning eagerly also avoids allocating a no-op state.
    if total == 0 {
        match kind {
            Some(CombinatorKind::All | CombinatorKind::AllSettled) => {
                let empty = ctx.vm.create_array_object(Vec::new());
                let _ = settle_promise(ctx.vm, result, false, JsValue::Object(empty));
            }
            Some(CombinatorKind::Any) => {
                let agg = build_aggregate_error(ctx.vm, Vec::new());
                let _ = settle_promise(ctx.vm, result, true, agg);
            }
            None => {} // race: stays pending
        }
        return Ok(JsValue::Object(result));
    }

    match kind {
        None => {
            // race: attach outer resolve/reject to every input.
            for item in items {
                subscribe(ctx, item, resolve, reject)?;
            }
        }
        Some(k) => {
            let state = alloc_combinator_state(ctx.vm, k, result, total);
            // Pre-allocate the shared reject step for `all` so every item
            // shares the same `AllReject` callable (spec doesn't mandate
            // identity but saving allocations makes per-iteration cheaper).
            let shared_all_reject = if k == CombinatorKind::All {
                Some(alloc_step(
                    ctx.vm,
                    PromiseCombinatorStep::AllReject { state },
                ))
            } else {
                None
            };
            let shared_any_fulfill = if k == CombinatorKind::Any {
                Some(alloc_step(
                    ctx.vm,
                    PromiseCombinatorStep::AnyFulfill { state },
                ))
            } else {
                None
            };

            for (i, item) in items.into_iter().enumerate() {
                let idx = u32::try_from(i).expect("items length already bounded by u32");
                let (on_fulfilled_id, on_rejected_id) = match k {
                    CombinatorKind::All => (
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AllFulfill { state, index: idx },
                        ),
                        shared_all_reject.expect("AllReject step allocated above for All kind"),
                    ),
                    CombinatorKind::AllSettled => (
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AllSettledFulfill { state, index: idx },
                        ),
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AllSettledReject { state, index: idx },
                        ),
                    ),
                    CombinatorKind::Any => (
                        shared_any_fulfill.expect("AnyFulfill step allocated above for Any kind"),
                        alloc_step(
                            ctx.vm,
                            PromiseCombinatorStep::AnyReject { state, index: idx },
                        ),
                    ),
                };
                subscribe(ctx, item, on_fulfilled_id, on_rejected_id)?;
            }
        }
    }

    Ok(JsValue::Object(result))
}

/// Collect every value produced by `iterator` into a `Vec`.  Honours
/// IteratorClose on error: if iteration panics (JS throw), the error
/// propagates — `resolve_iterator` / `iter_next` already close via their
/// own error paths when the next step throws, so we don't need a manual
/// IteratorClose here.
fn collect_items(vm: &mut VmInner, iterator: JsValue) -> Result<Vec<JsValue>, VmError> {
    let mut out = Vec::new();
    while let Some(v) = vm.iter_next(iterator)? {
        out.push(v);
    }
    Ok(out)
}

/// `item.then(on_fulfilled, on_rejected)` after `Promise.resolve(item)`
/// normalisation.  Used by every combinator to wire per-item reactions
/// onto the outer state machine.
fn subscribe(
    ctx: &mut NativeContext<'_>,
    item: JsValue,
    on_fulfilled: ObjectId,
    on_rejected: ObjectId,
) -> Result<(), VmError> {
    // Normalise non-promise inputs via Promise.resolve.
    let promise_id = if let JsValue::Object(id) = item {
        if matches!(ctx.get_object(id).kind, ObjectKind::Promise(_)) {
            id
        } else {
            let p = create_promise(ctx.vm);
            let _ = settle_promise(ctx.vm, p, false, item);
            p
        }
    } else {
        let p = create_promise(ctx.vm);
        let _ = settle_promise(ctx.vm, p, false, item);
        p
    };
    then_impl(ctx.vm, promise_id, Some(on_fulfilled), Some(on_rejected))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// finally
// ---------------------------------------------------------------------------

/// Run the `finally` step: invoke `on_finally()`, then pass through the
/// original value (fulfill path) or re-throw the original reason (reject
/// path).  If `on_finally` itself throws, its error propagates as the
/// reaction result and the capability rejects with it — spec §25.6.5.3.1/2
/// semantics under the simplification that the `on_finally` return value
/// is not awaited (see PR2 plan "Test262 alignment").
pub(super) fn run_finally_step(
    vm: &mut VmInner,
    on_finally: ObjectId,
    is_reject: bool,
    value: JsValue,
) -> Result<JsValue, VmError> {
    vm.call(on_finally, JsValue::Undefined, &[])?;
    if is_reject {
        // Re-throw so the promise reaction rejects the derived capability
        // with the original reason.
        Err(VmError {
            kind: VmErrorKind::ThrowValue(value),
            message: String::new(),
        })
    } else {
        Ok(value)
    }
}

// ---------------------------------------------------------------------------
// Native entry points
// ---------------------------------------------------------------------------

/// `Promise.all(iterable)` — §25.6.4.1
pub(super) fn native_promise_all(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, Some(CombinatorKind::All))
}

/// `Promise.allSettled(iterable)` — §25.6.4.2
pub(super) fn native_promise_all_settled(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, Some(CombinatorKind::AllSettled))
}

/// `Promise.race(iterable)` — §25.6.4.5
pub(super) fn native_promise_race(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, None)
}

/// `Promise.any(iterable)` — §25.6.4.3
pub(super) fn native_promise_any(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    run_combinator(ctx, args, Some(CombinatorKind::Any))
}

/// `Promise.prototype.finally(onFinally)` — §25.6.5.3
pub(super) fn native_promise_prototype_finally(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(src) = this else {
        return Err(VmError::type_error(
            "Promise.prototype.finally called on non-object",
        ));
    };
    if !matches!(ctx.get_object(src).kind, ObjectKind::Promise(_)) {
        return Err(VmError::type_error(
            "Promise.prototype.finally called on non-Promise",
        ));
    }
    let on_finally = match args.first().copied() {
        Some(JsValue::Object(id)) if ctx.get_object(id).kind.is_callable() => Some(id),
        _ => None,
    };

    // Short-circuit: if onFinally isn't callable, finally is a pure
    // passthrough — `then(undefined, undefined)` already propagates in
    // then_impl.
    let Some(on_finally) = on_finally else {
        return then_impl(ctx.vm, src, None, None);
    };

    let proto = ctx.vm.function_prototype;
    let fulfill_step = ctx.vm.alloc_object(Object {
        kind: ObjectKind::PromiseFinallyStep {
            on_finally,
            is_reject: false,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let reject_step = ctx.vm.alloc_object(Object {
        kind: ObjectKind::PromiseFinallyStep {
            on_finally,
            is_reject: true,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    then_impl(ctx.vm, src, Some(fulfill_step), Some(reject_step))
}
