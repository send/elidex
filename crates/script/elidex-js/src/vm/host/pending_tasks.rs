//! Same-window task queue + `window.postMessage` (WHATWG HTML
//! §9.4.3).
//!
//! The HTML event loop's *task queue* (§8.1.5 "Task"), restricted to
//! the single JavaScript realm that Phase 2 supports.  The only
//! producer currently implemented is [`Window::postMessage`]; future
//! producers include `Worker.postMessage`, `MessageChannel` /
//! `MessagePort`, `BroadcastChannel`, and `fetch()` once the async
//! refactor lands.  A task is drained at the end of every top-level
//! [`super::super::VmInner::eval`] — after the script's stack
//! unwinds, after all microtasks have been flushed, matching the
//! spec's "perform a microtask checkpoint" step between each task.
//!
//! ## Dispatch model (Phase 2 simplification)
//!
//! The `postMessage` path constructs a minimal `MessageEvent`-shaped
//! Event object and walks the listeners registered on the target
//! window entity for `"message"` directly — no capture / bubble
//! tree walk, because `Window` is a leaf target and
//! `MessageEvent.bubbles === false` per spec.  The listeners are
//! invoked in registration order, the Event identity is shared
//! across every handler (WHATWG DOM §2.9 "same Event object"), and
//! stopImmediatePropagation short-circuits remaining handlers.
//! Full three-phase dispatch lands with the cross-window / Worker
//! postMessage wiring (plan §Deferred #15 / PR5d).
//!
//! ## GC contract
//!
//! [`PendingTask`] variants hold `JsValue` / `ObjectId`s that would
//! otherwise become unreachable between the queue and the drain
//! step.  `mark_pending_tasks` in `gc.rs` traces every queued
//! task's payload (data, target, source, ports) so the message
//! payload survives a GC cycle triggered between `postMessage`
//! returning and the eval-boundary drain.

#![cfg(feature = "engine")]

use elidex_script_session::event_listener::EventListeners;

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::VmInner;
use super::structured_clone::clone_value;

// ---------------------------------------------------------------------------
// Task variants
// ---------------------------------------------------------------------------

/// A queued task awaiting dispatch on the HTML event loop (§8.1.5).
///
/// Each variant captures the minimum state needed to re-run the task
/// at drain time *without* re-running any of the producer's own
/// validation / cloning — those steps already ran at enqueue time
/// and their side effects (e.g. `DataCloneError`) were surfaced to
/// the caller synchronously.
#[derive(Clone, Debug)]
pub(crate) enum PendingTask {
    /// Produced by `window.postMessage(message, targetOrigin)` after
    /// the origin has been matched.  The stored `data` is the
    /// already-`structuredClone`d payload; `origin` is the source
    /// window's origin (WHATWG HTML §9.4.3 step 12 "origin of the
    /// source's relevant settings").
    PostMessage {
        target_window_id: ObjectId,
        data: JsValue,
        origin_sid: StringId,
        last_event_id_sid: StringId,
        source_window_id: Option<ObjectId>,
    },
}

// ---------------------------------------------------------------------------
// Queue API on VmInner
// ---------------------------------------------------------------------------

impl VmInner {
    /// Enqueue `task` for the next drain.  Producer-visible wrapper
    /// around `pending_tasks.push_back` so the field can stay
    /// `pub(crate)` without letting every module mutate it.
    pub(crate) fn queue_task(&mut self, task: PendingTask) {
        self.pending_tasks.push_back(task);
    }

    /// Drain every queued task in FIFO order, running a microtask
    /// checkpoint after each one (WHATWG HTML §8.1.5 step 5).
    ///
    /// Reentrancy-guarded: a nested call (a drained task enqueued
    /// another task that ran inline during its listener body) is
    /// a no-op — the outer loop picks up the newly-enqueued task
    /// on its next iteration.  Mirrors
    /// [`Self::microtask_drain_depth`]'s guard for the microtask
    /// queue.
    pub(crate) fn drain_tasks(&mut self) {
        if self.task_drain_depth > 0 {
            return;
        }
        self.task_drain_depth = self.task_drain_depth.saturating_add(1);
        while let Some(task) = self.pending_tasks.pop_front() {
            self.execute_task(task);
            // Per §8.1.5 step 5: microtask checkpoint between tasks.
            self.drain_microtasks();
        }
        self.task_drain_depth = self.task_drain_depth.saturating_sub(1);
    }

    fn execute_task(&mut self, task: PendingTask) {
        match task {
            PendingTask::PostMessage {
                target_window_id,
                data,
                origin_sid,
                last_event_id_sid,
                source_window_id,
            } => {
                dispatch_post_message(
                    self,
                    target_window_id,
                    data,
                    origin_sid,
                    last_event_id_sid,
                    source_window_id,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// postMessage dispatch
// ---------------------------------------------------------------------------

/// Build a message Event object and invoke every `message` listener
/// registered on `target_window_id`'s backing entity.
///
/// Matches WHATWG HTML §9.4.3 step 14 + §2.9 "fire a trusted event
/// with name `message` at a Window".  Phase 2 simplification: no
/// capture / bubble walk (Window is a leaf target).
fn dispatch_post_message(
    vm: &mut VmInner,
    target_window_id: ObjectId,
    data: JsValue,
    origin_sid: StringId,
    last_event_id_sid: StringId,
    source_window_id: Option<ObjectId>,
) {
    // Resolve the target window's backing entity.  If the VM lost
    // its HostData between enqueue and drain (vm.unbind()), silently
    // drop — matching WHATWG §9.4.3 step 11 "if the target's
    // associated Document is not fully active, then abort".
    let Some(host) = vm.host_data.as_deref() else {
        return;
    };
    if !host.is_bound() {
        return;
    }
    let ObjectKind::HostObject {
        entity_bits: target_entity_bits,
    } = vm.get_object(target_window_id).kind
    else {
        return;
    };
    let Some(target_entity) = elidex_ecs::Entity::from_bits(target_entity_bits) else {
        return;
    };

    // Collect matching listener IDs up front; we then invoke them
    // outside the DOM borrow so the listener body can mutate ECS /
    // allocate / dispatch further events without aliasing.  Both
    // `capture` and non-capture listeners are collected — Window is
    // the leaf, so capture-phase listeners fire with the same event
    // object as the bubble-phase ones.
    let listener_ids: Vec<elidex_script_session::event_listener::ListenerId> = {
        let dom = vm.host_data.as_mut().expect("host_data check above").dom();
        let Ok(listeners) = dom.world().get::<&EventListeners>(target_entity) else {
            return;
        };
        listeners.matching_all_ids("message")
    };
    if listener_ids.is_empty() {
        return;
    }

    // Intern `"message"` for the `type` data property.  The single
    // string is pool-permanent; subsequent dispatches hit the dedup
    // fast path in `StringPool::intern`.
    let message_type_sid = vm.strings.intern("message");

    // Build a MessageEvent-like Event object: the ObjectKind::Event
    // internal slots (type_sid / bubbles / composed / etc.) are
    // authoritative for dispatch; own data properties mirror the
    // WebIDL attrs for JS visibility.  `cancelable = false` so
    // `preventDefault()` is a spec-visible no-op matching browser
    // MessageEvent semantics (§2.2 `cancelable` default).
    let event_id = vm.alloc_object(Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: false,
            passive: false,
            type_sid: message_type_sid,
            bubbles: false,
            composed: false,
            composed_path: None,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.event_prototype,
        extensible: true,
    });

    // Own data properties mirroring the MessageEvent WebIDL attrs +
    // core Event attrs.  Walked via shape transition so every
    // MessageEvent dispatched during one VM lifetime shares the
    // same terminal shape (IC-friendly).
    let type_key = PropertyKey::String(vm.well_known.event_type);
    vm.define_shaped_property(
        event_id,
        type_key,
        PropertyValue::Data(JsValue::String(message_type_sid)),
        PropertyAttrs::WEBIDL_RO,
    );
    let bubbles_key = PropertyKey::String(vm.well_known.bubbles);
    vm.define_shaped_property(
        event_id,
        bubbles_key,
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyAttrs::WEBIDL_RO,
    );
    let target_key = PropertyKey::String(vm.well_known.target);
    vm.define_shaped_property(
        event_id,
        target_key,
        PropertyValue::Data(JsValue::Object(target_window_id)),
        PropertyAttrs::WEBIDL_RO,
    );
    let current_target_key = PropertyKey::String(vm.well_known.current_target);
    vm.define_shaped_property(
        event_id,
        current_target_key,
        PropertyValue::Data(JsValue::Object(target_window_id)),
        PropertyAttrs::WEBIDL_RO,
    );
    let data_key = PropertyKey::String(vm.strings.intern("data"));
    vm.define_shaped_property(
        event_id,
        data_key,
        PropertyValue::Data(data),
        PropertyAttrs::WEBIDL_RO,
    );
    let origin_key = PropertyKey::String(vm.strings.intern("origin"));
    vm.define_shaped_property(
        event_id,
        origin_key,
        PropertyValue::Data(JsValue::String(origin_sid)),
        PropertyAttrs::WEBIDL_RO,
    );
    let last_event_id_key = PropertyKey::String(vm.strings.intern("lastEventId"));
    vm.define_shaped_property(
        event_id,
        last_event_id_key,
        PropertyValue::Data(JsValue::String(last_event_id_sid)),
        PropertyAttrs::WEBIDL_RO,
    );
    // `source` is the Window that posted the message; `null` when
    // the producer did not identify one (WHATWG HTML §9.4.3 step 8).
    let source_key = PropertyKey::String(vm.strings.intern("source"));
    let source_val = source_window_id
        .map(JsValue::Object)
        .unwrap_or(JsValue::Null);
    vm.define_shaped_property(
        event_id,
        source_key,
        PropertyValue::Data(source_val),
        PropertyAttrs::WEBIDL_RO,
    );
    // `ports` — empty Array until MessagePort lands (plan §Deferred
    // #16).  Allocating per dispatch keeps identity fresh per
    // event, matching browser behaviour.
    let ports_arr = vm.create_array_object(Vec::new());
    let ports_key = PropertyKey::String(vm.strings.intern("ports"));
    vm.define_shaped_property(
        event_id,
        ports_key,
        PropertyValue::Data(JsValue::Object(ports_arr)),
        PropertyAttrs::WEBIDL_RO,
    );

    // Root the event object across the listener invocations —
    // listener bodies may trigger GC before the event goes out of
    // scope on the VM stack.  `push_temp_root` returns an RAII
    // guard that derefs to `&mut VmInner`; writing through `g`
    // keeps the root alive for the whole walk.
    let mut g = vm.push_temp_root(JsValue::Object(event_id));
    let target_this = JsValue::Object(target_window_id);
    let event_arg = [JsValue::Object(event_id)];
    for listener_id in listener_ids {
        // `immediate_propagation_stopped` short-circuits remaining
        // listeners — spec-mandated for `stopImmediatePropagation`.
        if let ObjectKind::Event {
            immediate_propagation_stopped: true,
            ..
        } = g.get_object(event_id).kind
        {
            break;
        }
        let Some(callback) = g
            .host_data
            .as_deref()
            .and_then(|h| h.get_listener(listener_id))
        else {
            continue;
        };
        // Listener-body throws are caught and dropped (WHATWG §2.10
        // step 10 "report the exception").  We have no window to
        // forward the error to yet — `onerror` dispatch lands with
        // the error-reporting tranche.
        let _ = g.call(callback, target_this, &event_arg);
    }
}

// ---------------------------------------------------------------------------
// `window.postMessage` native
// ---------------------------------------------------------------------------

/// `window.postMessage(message, targetOrigin, transfer?)` — WHATWG
/// HTML §9.4.3.  Also accepts the dictionary signature
/// `postMessage(message, options)` where `options` is
/// `{targetOrigin?, transfer?}`.
pub(super) fn native_window_post_message(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // 1. Binding-level arg count check (WebIDL "not enough
    //    arguments" — sync TypeError before §9.4.3 proper).
    let Some(&message) = args.first() else {
        return Err(VmError::type_error(
            "Failed to execute 'postMessage' on 'Window': 1 argument required, but only 0 present.",
        ));
    };

    // 2. Signature dispatch: legacy (message, targetOrigin string,
    //    transfer array) vs dict (message, options object).
    let arg1 = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (target_origin, transfer_val) = extract_signature(ctx, arg1, args.get(2).copied())?;

    // 3. transfer validation — Phase 2 accepts only `undefined` /
    //    `null` / `[]`.  Non-empty list → DataCloneError (transfer
    //    semantics not yet wired).  Spec prescribes DataCloneError
    //    for "not all items are transferable" (§9.4.3 step 5).
    validate_transfer(ctx, transfer_val)?;

    // 4. Structured serialize `message`.  Clone throws surface
    //    synchronously; origin mismatch is a silent return (checked
    //    after, step 6).  Matches spec order: step 5 (serialize)
    //    runs before step 7 (origin match).
    let cloned = clone_value(ctx.vm, message)?;

    // 5. Resolve target origin match vs own origin.
    let own_origin_sid = compute_own_origin_sid(ctx.vm);
    let origin_match = match_target_origin(ctx, target_origin, own_origin_sid)?;
    if !origin_match {
        // Silent return (spec §9.4.3 step 9 "if the origin of
        // targetWindow is not … then return").
        return Ok(JsValue::Undefined);
    }

    // 6. Determine target & source windows.  Phase 2 same-window
    //    only — target is the caller's globalThis, source is the
    //    same window (round-trip postMessage is observable).
    let target_window_id = ctx.vm.global_object;
    let source_window_id = Some(ctx.vm.global_object);

    // 7. Queue the task.  Drain runs at the end of the current
    //    `eval` (after microtasks); until then `message` listeners
    //    observe nothing.
    ctx.vm.queue_task(PendingTask::PostMessage {
        target_window_id,
        data: cloned,
        origin_sid: own_origin_sid,
        last_event_id_sid: ctx.vm.well_known.empty,
        source_window_id,
    });

    Ok(JsValue::Undefined)
}

/// Decompose the 2nd / 3rd arg into a `(targetOrigin, transfer)`
/// pair, matching the two WHATWG signatures:
///
/// - `postMessage(msg, targetOrigin, transfer?)` — arg1 is the
///   `targetOrigin` string, arg2 is the transfer array.
/// - `postMessage(msg, options)` — arg1 is an options dict whose
///   `targetOrigin` / `transfer` properties drive dispatch.
///
/// Overload selection: arg1 is an (non-null) Object → dict form;
/// otherwise legacy.
fn extract_signature(
    ctx: &mut NativeContext<'_>,
    arg1: JsValue,
    arg2: Option<JsValue>,
) -> Result<(StringId, JsValue), VmError> {
    let is_dict = matches!(arg1, JsValue::Object(_));
    if is_dict {
        let JsValue::Object(opts_id) = arg1 else {
            unreachable!()
        };
        let target_origin_key = PropertyKey::String(ctx.vm.strings.intern("targetOrigin"));
        let target_origin_val = ctx
            .vm
            .get_object(opts_id)
            .storage
            .get(target_origin_key, &ctx.vm.shapes)
            .and_then(|(pv, _)| match pv {
                PropertyValue::Data(v) => Some(*v),
                PropertyValue::Accessor { .. } => None,
            })
            .unwrap_or(JsValue::Undefined);
        let target_origin_sid = coerce_to_string_or_default(ctx, target_origin_val, "/")?;
        let transfer_key = PropertyKey::String(ctx.vm.strings.intern("transfer"));
        let transfer_val = ctx
            .vm
            .get_object(opts_id)
            .storage
            .get(transfer_key, &ctx.vm.shapes)
            .and_then(|(pv, _)| match pv {
                PropertyValue::Data(v) => Some(*v),
                PropertyValue::Accessor { .. } => None,
            })
            .unwrap_or(JsValue::Undefined);
        Ok((target_origin_sid, transfer_val))
    } else {
        // Legacy: arg1 is `targetOrigin` (ToString-coerced; default
        // `"/"` when `undefined`), arg2 is `transfer`.
        let target_origin_sid = coerce_to_string_or_default(ctx, arg1, "/")?;
        let transfer_val = arg2.unwrap_or(JsValue::Undefined);
        Ok((target_origin_sid, transfer_val))
    }
}

fn coerce_to_string_or_default(
    ctx: &mut NativeContext<'_>,
    v: JsValue,
    default_str: &str,
) -> Result<StringId, VmError> {
    match v {
        JsValue::Undefined => Ok(ctx.vm.strings.intern(default_str)),
        other => coerce::to_string(ctx.vm, other),
    }
}

fn validate_transfer(ctx: &mut NativeContext<'_>, transfer: JsValue) -> Result<(), VmError> {
    match transfer {
        JsValue::Undefined | JsValue::Null => Ok(()),
        JsValue::Object(arr_id) => {
            if let ObjectKind::Array { elements } = &ctx.vm.get_object(arr_id).kind {
                if elements.is_empty() {
                    return Ok(());
                }
            }
            Err(VmError::dom_exception(
                ctx.vm.well_known.dom_exc_data_clone_error,
                "Failed to execute 'postMessage' on 'Window': Transferable objects are not yet supported.",
            ))
        }
        _ => Err(VmError::type_error(
            "Failed to execute 'postMessage' on 'Window': The provided transfer value is not iterable.",
        )),
    }
}

/// Current window's origin as a StringId.  WHATWG "Origin" serialisation
/// of [`super::navigation::NavigationState::current_url`].
fn compute_own_origin_sid(vm: &mut VmInner) -> StringId {
    let origin = vm.navigation.current_url.origin();
    let origin_str = origin.ascii_serialization();
    vm.strings.intern(&origin_str)
}

/// Match `targetOrigin` against own origin.
///
/// - `"*"` → always match.
/// - `"/"` → match if identical to own origin.
/// - otherwise → parse as URL; `SyntaxError` on failure, then
///   compare the parsed URL's origin to own origin.
fn match_target_origin(
    ctx: &mut NativeContext<'_>,
    target_origin: StringId,
    own_origin_sid: StringId,
) -> Result<bool, VmError> {
    let target_origin_str = ctx.vm.strings.get_utf8(target_origin);
    if target_origin_str == "*" {
        return Ok(true);
    }
    if target_origin_str == "/" {
        return Ok(true); // same window ⇒ always own origin
    }
    match url::Url::parse(&target_origin_str) {
        Ok(u) => {
            let target_origin_serialized = u.origin().ascii_serialization();
            let own_origin_str = ctx.vm.strings.get_utf8(own_origin_sid);
            Ok(target_origin_serialized == own_origin_str)
        }
        Err(_) => Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_syntax_error,
            format!(
                "Failed to execute 'postMessage' on 'Window': Invalid target origin '{target_origin_str}'.",
            ),
        )),
    }
}
