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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PromiseState, PromiseStatus, PropertyKey,
    PropertyStorage, PropertyValue, Reaction, ReactionKind, VmError, VmErrorKind,
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
    /// accordingly.  Mirrors ES2020 §25.6.1.3 `NewPromiseReactionJob`.
    PromiseReaction {
        kind: ReactionKind,
        handler: Option<ObjectId>,
        capability: ObjectId,
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

pub(super) fn then_impl(
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
