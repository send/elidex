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
//! ## Dispatch model
//!
//! The `postMessage` path constructs a MessageEvent-shaped Event
//! object (core-9 layout + `data` / `origin` / `lastEventId` /
//! `source` / `ports` slots) and routes it through
//! [`super::event_target::dispatch_script_event`].  That helper
//! builds the dispatch plan via the session crate's
//! `build_dispatch_plan` and applies standard per-listener option
//! semantics: registration order walk, shared Event identity across
//! handlers (WHATWG DOM §2.9 "same Event object"),
//! `stopImmediatePropagation`, plus `{once}` auto-removal,
//! `{signal}` back-ref cleanup, and the `passive` flag toggle.
//!
//! For `Window` targets, the propagation path is target-only in
//! practice: Window is a leaf in the composed-path walk and
//! `MessageEvent.bubbles === false` per spec, so capture / bubble
//! phases have no listeners.  The shared machinery still handles
//! future non-leaf dispatch (cross-window / Worker, plan §Deferred
//! #15 / PR5d) without further rewiring.
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

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::VmInner;
use super::event_target::dispatch_script_event;
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

/// Build a MessageEvent and dispatch it at `target_window_id`'s
/// backing entity through the shared `dispatch_script_event` walker.
///
/// Matches WHATWG HTML §9.4.3 step 14 + §2.9 "fire a trusted event
/// with name `message` at a Window".  Routing through
/// `dispatch_script_event` gives correct per-listener `{once}` /
/// `{signal}` / `{passive}` handling for free — the manual walk
/// that predated this path leaked `{once}` entries and ignored
/// aborted signals.
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

    let message_type_sid = vm.well_known.message;

    // Allocate the Event with the authoritative internal slots.
    // `cancelable = false` makes `preventDefault()` a spec-visible
    // no-op per MessageEvent's §2.2 default.  `is_trusted = true`
    // because UA is the synthesizer (postMessage is a browser-
    // initiated dispatch, not a user script `dispatchEvent(...)` call).
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

    // Root the event across the subsequent slot installs + dispatch
    // — the MessageEvent's `data` payload holds arbitrary user
    // objects that would otherwise be only reachable from the Rust
    // locals here, and the precomputed-shape install below allocates.
    let mut g = vm.push_temp_root(JsValue::Object(event_id));

    // Install core-9 + MessageEvent payload via the precomputed
    // `shapes.message` terminal shape.  Slot order MUST match
    // `build_precomputed_event_shapes`'s `core_keys` ordering so
    // `set_event_slot_raw(event_id, EVENT_SLOT_TARGET=4, ...)` from
    // inside `dispatch_script_event` hits the `target` slot.
    let message_shape = g
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .message;
    let timestamp_ms = g.start_instant.elapsed().as_secs_f64() * 1000.0;
    // Slot order MUST match the `core_keys` ordering in
    // `build_precomputed_event_shapes` + the `message` payload
    // extension (`data` / `origin` / `lastEventId`).
    let slots: Vec<PropertyValue> = vec![
        PropertyValue::Data(JsValue::String(message_type_sid)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Number(0.0)),
        PropertyValue::Data(JsValue::Object(target_window_id)),
        PropertyValue::Data(JsValue::Object(target_window_id)),
        PropertyValue::Data(JsValue::Number(timestamp_ms)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(true)),
        PropertyValue::Data(data),
        PropertyValue::Data(JsValue::String(origin_sid)),
        PropertyValue::Data(JsValue::String(last_event_id_sid)),
    ];
    g.define_with_precomputed_shape(event_id, message_shape, slots);

    // `source` + `ports` extend past the precomputed shape (the
    // shell-side MessageEvent shape doesn't carry them; see
    // `event_shapes::dispatch_payload` Message arm).  Installed as
    // ordinary shape-transition properties — only the core-9 slot
    // indices matter for dispatch, the rest are JS-visible own
    // data.  `source` is `null` when the producer did not identify
    // a window (WHATWG HTML §9.4.3 step 8).
    let source_val = source_window_id
        .map(JsValue::Object)
        .unwrap_or(JsValue::Null);
    let source_key = PropertyKey::String(g.well_known.source);
    g.define_shaped_property(
        event_id,
        source_key,
        PropertyValue::Data(source_val),
        PropertyAttrs::WEBIDL_RO,
    );
    // `ports` — fresh empty Array until MessagePort lands (plan
    // §Deferred #16).  Allocating per dispatch keeps identity fresh
    // per event, matching browser behaviour.
    let ports_arr = g.create_array_object(Vec::new());
    let ports_key = PropertyKey::String(g.strings.intern("ports"));
    g.define_shaped_property(
        event_id,
        ports_key,
        PropertyValue::Data(JsValue::Object(ports_arr)),
        PropertyAttrs::WEBIDL_RO,
    );

    // Bracket `dispatched_events` membership around the dispatch.
    // `dispatch_script_event`'s doc contract requires the event to
    // already be present; the outer `native_event_target_dispatch_event`
    // does the same insert/remove dance.
    g.dispatched_events.insert(event_id);
    let mut ctx = NativeContext { vm: &mut g };
    let dispatch_result = dispatch_script_event(&mut ctx, event_id, target_entity);
    g.dispatched_events.remove(&event_id);
    // VM-level errors (allocation failure etc.) are very rare and
    // swallowed here — the dispatch has already advanced through
    // as many listeners as possible, and postMessage has no
    // observable error channel to the enqueuing caller (the task
    // is an async point, §9.4.3 step 13).
    let _ = dispatch_result;
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
        // WebIDL dictionary conversion uses ordinary `Get` (§7.3.1):
        // walks the prototype chain and fires accessor getters.
        // `storage.get` alone would silently ignore inherited /
        // accessor-defined `targetOrigin` / `transfer` entries.
        let target_origin_key = PropertyKey::String(ctx.vm.strings.intern("targetOrigin"));
        let target_origin_val = ctx.vm.get_property_value(opts_id, target_origin_key)?;
        let target_origin_sid = coerce_to_string_or_default(ctx, target_origin_val, "/")?;
        let transfer_key = PropertyKey::String(ctx.vm.strings.intern("transfer"));
        let transfer_val = ctx.vm.get_property_value(opts_id, transfer_key)?;
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
/// - `"/"` → spec: "restrict the message to the same origin as the
///   source".  Phase 2 is same-window only (source and target
///   share the settings object), so the comparison is trivially
///   satisfied and we short-circuit to `true`.  Cross-window
///   postMessage (PR5d) adds the actual own-vs-target comparison.
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
