//! `ReadableStreamDefaultReader` — `getReader`, the reader
//! constructor, and the four reader natives (`read` /
//! `releaseLock` / `cancel` / `closed`).

use std::collections::VecDeque;

use super::super::super::natives_promise::{create_promise, settle_promise};
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::super::{NativeFn, VmInner};
use super::controller::{deliver_pending_reads, pull_if_needed};
use super::{
    do_stream_cancel, require_stream_this, stream_state, ReadableStreamStateKind, ReaderState,
};

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_reader_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "ReadableStreamDefaultReader.prototype.{method} called on non-reader"
        )));
    };
    if matches!(
        ctx.vm.get_object(id).kind,
        ObjectKind::ReadableStreamDefaultReader
    ) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "ReadableStreamDefaultReader.prototype.{method} called on non-reader"
        )))
    }
}

// ---------------------------------------------------------------------------
// Reader acquisition
// ---------------------------------------------------------------------------

/// Allocate + initialise a default reader against `stream_id`.
/// Used by the `getReader()` path on the stream prototype.
/// Errors if the stream is already locked.
fn acquire_default_reader(vm: &mut VmInner, stream_id: ObjectId) -> Result<ObjectId, VmError> {
    if stream_state(vm, stream_id).reader_id.is_some() {
        return Err(VmError::type_error(
            "Failed to execute 'getReader' on 'ReadableStream': stream is already locked to a reader",
        ));
    }
    let proto = vm.readable_stream_default_reader_prototype;
    let reader_id = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStreamDefaultReader,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    initialise_default_reader(vm, reader_id, stream_id);
    Ok(reader_id)
}

/// Wire `reader_id` ↔ `stream_id`: allocate the reader's `closed`
/// Promise, settle it immediately if the stream is already Closed
/// or Errored (spec §4.3.3), insert the reader state and lock
/// the stream.  Shared between the `getReader()` path
/// (`acquire_default_reader`) and the
/// `new ReadableStreamDefaultReader(stream)` constructor — the
/// latter promotes a pre-allocated `this` instead of allocating,
/// so the wiring step alone needs to be reusable.
fn initialise_default_reader(vm: &mut VmInner, reader_id: ObjectId, stream_id: ObjectId) {
    let closed_promise = create_promise(vm);
    let state_kind = stream_state(vm, stream_id).state;
    match state_kind {
        ReadableStreamStateKind::Closed => {
            let _ = settle_promise(vm, closed_promise, false, JsValue::Undefined);
        }
        ReadableStreamStateKind::Errored => {
            let stored = stream_state(vm, stream_id).stored_error;
            let _ = settle_promise(vm, closed_promise, true, stored);
        }
        ReadableStreamStateKind::Readable => {}
    }
    vm.readable_stream_reader_states.insert(
        reader_id,
        ReaderState {
            stream_id: Some(stream_id),
            pending_read_promises: VecDeque::new(),
            closed_promise,
        },
    );
    super::stream_state_mut(vm, stream_id).reader_id = Some(reader_id);
}

// ---------------------------------------------------------------------------
// Stream-side `getReader`
// ---------------------------------------------------------------------------

pub(super) fn native_readable_stream_get_reader(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let stream_id = require_stream_this(ctx, this, "getReader")?;
    // WebIDL: the `options` argument is a dictionary, so non-`null` /
    // non-`undefined` non-objects must throw TypeError (Copilot R5
    // finding).
    let opts_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    match opts_arg {
        JsValue::Undefined | JsValue::Null => {}
        JsValue::Object(_) => {}
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getReader' on 'ReadableStream': options must be an object",
            ));
        }
    }
    // Spec §4.2.4 step 2: `options.mode` is undefined ⇒ default
    // reader; `"byob"` ⇒ BYOB reader (Phase 2 unsupported, throws);
    // any other value ⇒ TypeError per WebIDL `ReadableStreamReaderMode`
    // enumeration.  Comparing against the literal `"byob"` avoids
    // R1's bug of accepting `mode: ""` and rejecting `mode: "default"`.
    if let JsValue::Object(opts_id) = opts_arg {
        let mode_sid = ctx.vm.strings.intern("mode");
        let mode_key = PropertyKey::String(mode_sid);
        if let Some(prop) = super::super::super::coerce::get_property(ctx.vm, opts_id, mode_key) {
            let v = ctx.vm.resolve_property(prop, JsValue::Object(opts_id))?;
            match v {
                JsValue::Undefined => {}
                JsValue::String(sid) => {
                    let s = ctx.vm.strings.get_utf8(sid);
                    if s == "byob" {
                        return Err(VmError::type_error(
                            "Failed to execute 'getReader' on 'ReadableStream': BYOB readers are not yet supported",
                        ));
                    }
                    // Any other string is not a valid enumeration
                    // member.  WebIDL throws TypeError.
                    return Err(VmError::type_error(
                        "Failed to execute 'getReader' on 'ReadableStream': options.mode must be 'byob' or undefined",
                    ));
                }
                _ => {
                    return Err(VmError::type_error(
                        "Failed to execute 'getReader' on 'ReadableStream': options.mode must be 'byob' or undefined",
                    ));
                }
            }
        }
    }
    let reader_id = acquire_default_reader(ctx.vm, stream_id)?;
    Ok(JsValue::Object(reader_id))
}

// ---------------------------------------------------------------------------
// Reader constructor + natives
// ---------------------------------------------------------------------------

/// `new ReadableStreamDefaultReader(stream)` — spec §4.3.3.
///
/// Promotes the pre-allocated `this` (built by `do_new` with the
/// caller's `new.target.prototype`) to
/// [`ObjectKind::ReadableStreamDefaultReader`] so subclassing /
/// `new.target` semantics survive — matches the
/// `Blob` / `Headers` / `Request` / `Response` ctor pattern.  An
/// earlier draft routed through `acquire_default_reader`, which
/// always allocated a fresh `Object` and discarded the
/// pre-allocated receiver, breaking subclassing and leaking the
/// unused `this` (Copilot R1 finding).
fn native_readable_stream_default_reader_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };
    let stream_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(stream_id) = stream_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': argument is not a ReadableStream",
        ));
    };
    if !matches!(
        ctx.vm.get_object(stream_id).kind,
        ObjectKind::ReadableStream
    ) {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': argument is not a ReadableStream",
        ));
    }
    if stream_state(ctx.vm, stream_id).reader_id.is_some() {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': stream is already locked to a reader",
        ));
    }
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::ReadableStreamDefaultReader;
    initialise_default_reader(ctx.vm, inst_id, stream_id);
    Ok(JsValue::Object(inst_id))
}

fn native_reader_read(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "read")?;
    let p = create_promise(ctx.vm);

    let stream_id_opt = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .and_then(|s| s.stream_id);
    let Some(stream_id) = stream_id_opt else {
        // Reader is released — spec §4.3.4: reject with TypeError.
        let err = VmError::type_error(
            "Failed to execute 'read' on 'ReadableStreamDefaultReader': reader is released",
        );
        let reason = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, p, true, reason);
        return Ok(JsValue::Object(p));
    };

    match stream_state(ctx.vm, stream_id).state {
        ReadableStreamStateKind::Errored => {
            let stored = stream_state(ctx.vm, stream_id).stored_error;
            let _ = settle_promise(ctx.vm, p, true, stored);
            return Ok(JsValue::Object(p));
        }
        ReadableStreamStateKind::Closed if stream_state(ctx.vm, stream_id).queue.is_empty() => {
            let result = ctx.vm.create_iter_result(JsValue::Undefined, true);
            let _ = settle_promise(ctx.vm, p, false, JsValue::Object(result));
            return Ok(JsValue::Object(p));
        }
        _ => {}
    }

    // If a chunk is already queued, deliver synchronously (after
    // queueing the promise on the reader so deliver_pending_reads
    // pops it).  Otherwise the promise stays pending until a
    // future `controller.enqueue` / `controller.close` /
    // `controller.error`.
    if let Some(reader_state) = ctx.vm.readable_stream_reader_states.get_mut(&id) {
        reader_state.pending_read_promises.push_back(p);
    }
    deliver_pending_reads(ctx.vm, stream_id);
    // Even if nothing was delivered, a queued read may now satisfy
    // backpressure for `pull` — recompute pump.
    pull_if_needed(ctx.vm, stream_id);
    Ok(JsValue::Object(p))
}

fn native_reader_release_lock(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "releaseLock")?;
    let stream_id_opt = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .and_then(|s| s.stream_id);
    let Some(stream_id) = stream_id_opt else {
        return Ok(JsValue::Undefined);
    };
    // Spec §4.3.5: reject any pending reads with TypeError, then
    // unlock.
    let pending: Vec<ObjectId> =
        if let Some(reader_state) = ctx.vm.readable_stream_reader_states.get_mut(&id) {
            std::mem::take(&mut reader_state.pending_read_promises)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
    for read_p in pending {
        let err = VmError::type_error(
            "Pending reader read was released before the stream produced a value",
        );
        let reason = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, read_p, true, reason);
    }
    // Spec §4.3.4 ReadableStreamReaderGenericRelease step 1-2:
    // - If stream state is "readable" → reject the *existing*
    //   closedPromise (so `const p = r.closed; r.releaseLock();`
    //   sees `p` reject — Copilot R10 finding).
    // - Otherwise → replace closedPromise with a fresh rejected
    //   Promise (the previous one was already settled on
    //   close/error, so rejection is a no-op via the
    //   already_resolved gate).
    // We always do "reject prev + install fresh rejected" because
    // settle_promise no-ops on settled promises, so the readable
    // case naturally rejects the prev and the closed/errored
    // case simply swaps in a new rejected one.  Anyone holding
    // the prev reference observes the rejection in the readable
    // case and observes the prior settlement otherwise (matching
    // spec semantics either way).
    let prev_closed = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .map(|s| s.closed_promise);
    let err = VmError::type_error("ReadableStream reader was released");
    let reason = ctx.vm.vm_error_to_thrown(&err);
    if let Some(prev) = prev_closed {
        let _ = settle_promise(ctx.vm, prev, true, reason);
    }
    let new_closed = create_promise(ctx.vm);
    let _ = settle_promise(ctx.vm, new_closed, true, reason);
    if let Some(reader_state) = ctx.vm.readable_stream_reader_states.get_mut(&id) {
        reader_state.stream_id = None;
        reader_state.closed_promise = new_closed;
    }
    if let Some(stream_state) = ctx.vm.readable_stream_states.get_mut(&stream_id) {
        if stream_state.reader_id == Some(id) {
            stream_state.reader_id = None;
        }
    }
    Ok(JsValue::Undefined)
}

fn native_reader_get_closed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "closed")?;
    let p = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .map(|s| s.closed_promise);
    match p {
        Some(p) => Ok(JsValue::Object(p)),
        None => Err(VmError::type_error("Invalid reader state")),
    }
}

fn native_reader_cancel(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "cancel")?;
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    let stream_id_opt = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .and_then(|s| s.stream_id);
    let Some(stream_id) = stream_id_opt else {
        let p = create_promise(ctx.vm);
        let err = VmError::type_error(
            "Failed to execute 'cancel' on 'ReadableStreamDefaultReader': reader is released",
        );
        let r = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, p, true, r);
        return Ok(JsValue::Object(p));
    };
    Ok(JsValue::Object(do_stream_cancel(ctx.vm, stream_id, reason)))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    pub(in crate::vm) fn register_readable_stream_reader_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_readable_stream_reader_global called before register_prototypes");
        let reader_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_readable_stream_reader_members(reader_proto);
        self.readable_stream_default_reader_prototype = Some(reader_proto);

        let reader_ctor = self.create_constructable_function(
            "ReadableStreamDefaultReader",
            native_readable_stream_default_reader_constructor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            reader_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(reader_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            reader_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(reader_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.readable_stream_default_reader_global,
            JsValue::Object(reader_ctor),
        );

        // Install `getReader` on the stream prototype now that the
        // reader prototype exists (avoids forward-reference at the
        // earlier stream-registration site).
        let stream_proto = self
            .readable_stream_prototype
            .expect("readable_stream_prototype must be set");
        self.install_native_method(
            stream_proto,
            self.well_known.get_reader,
            native_readable_stream_get_reader as NativeFn,
            PropertyAttrs::METHOD,
        );
    }

    fn install_readable_stream_reader_members(&mut self, proto_id: ObjectId) {
        // `closed` getter.
        let closed_sid = self.well_known.closed_attr;
        self.install_accessor_pair(
            proto_id,
            closed_sid,
            native_reader_get_closed as NativeFn,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let methods: [(StringId, NativeFn); 3] = [
            (self.well_known.read, native_reader_read as NativeFn),
            (
                self.well_known.release_lock,
                native_reader_release_lock as NativeFn,
            ),
            (self.well_known.cancel, native_reader_cancel as NativeFn),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }
    }
}
