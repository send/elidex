//! `FileReader` interface (File API §6) — async byte reader for `Blob`
//! / `File` instances.
//!
//! ```text
//! FileReader instance (ObjectKind::FileReader, payload-free)
//!   → FileReader.prototype  (this module)
//!     → EventTarget.prototype  (vm/host/event_target.rs)
//! ```
//!
//! ## Scope
//!
//! - `readAsText(blob, encoding?)` / `readAsArrayBuffer(blob)` /
//!   `readAsDataURL(blob)` / `readAsBinaryString(blob)` — set state to
//!   LOADING, fire `loadstart` synchronously via on* handler, then
//!   enqueue a [`super::pending_tasks::PendingTask::FileRead`] task
//!   whose drain performs the actual read + fires terminal events.
//! - `abort()` — cancels an in-flight read by incrementing `abort_seq`
//!   in the side-data; the drained task compares its snapshot to the
//!   current value and silent-discards on mismatch.
//! - `readyState` / `result` / `error` IDL readonly attrs.
//! - Event handler attributes: `onloadstart` / `onprogress` / `onload`
//!   / `onloadend` / `onerror` / `onabort` (per spec §6.2 IDL).
//! - EMPTY / LOADING / DONE constants on both ctor and prototype.
//!
//! ## Event delivery scope (Phase 4 scope decision)
//!
//! Phase 4 fires events ONLY through the on* handler attributes —
//! `addEventListener('load', cb)` against a FileReader instance does
//! NOT deliver because FileReader is a non-Entity EventTarget (unlike
//! Element / Document) and the dispatch infrastructure
//! (`event_target_dispatch::dispatch_script_event`) requires an
//! `elidex_ecs::Entity` target.  Side-table-backed listener storage
//! plus a custom dispatch path (modelled on `AbortSignal::abort_listeners`)
//! is deferred to `#11-filereader-event-listeners` (added by this PR).
//! Real-world FileReader code overwhelmingly uses on* handlers; the
//! addEventListener gap is small + future PR.

#![cfg(feature = "engine")]

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, PropertyValue, StringId,
    VmError,
};
use super::super::{NativeFn, VmInner};
use super::events::{
    install_ctor, set_event_slot_raw, EventInit, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_TARGET,
};

// ---------------------------------------------------------------------------
// State enums
// ---------------------------------------------------------------------------

/// `FileReader.readyState` enum per FileAPI §6 (3 values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ReadyState {
    #[default]
    Empty = 0,
    Loading = 1,
    Done = 2,
}

/// `FileReader.result` typed payload per FileAPI §6.2.  Always `None`
/// while state != DONE; populated by the read task drain on success.
#[derive(Clone, Debug, Default)]
pub(crate) enum ReaderResult {
    #[default]
    None,
    Text(String),
    /// `ObjectId` of the freshly allocated ArrayBuffer wrapper —
    /// traced by `vm/gc/trace.rs` FileReader fan-out.
    ArrayBuffer(ObjectId),
    DataUrl(String),
    BinaryString(String),
}

/// Read operation kind carried by
/// [`super::pending_tasks::PendingTask::FileRead`] so the drain knows
/// which decode path to take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadKind {
    Text,
    ArrayBuffer,
    DataUrl,
    BinaryString,
}

/// Per-`FileReader` out-of-band state, keyed in
/// [`super::super::VmInner::file_reader_data`] by the instance's
/// `ObjectId`.
#[derive(Debug, Default)]
pub(crate) struct FileReaderSideData {
    pub(crate) state: ReadyState,
    pub(crate) result: ReaderResult,
    /// `DOMException` wrapper `ObjectId` populated on error / abort.
    pub(crate) error: Option<ObjectId>,
    /// `ObjectId` of the Blob / File being read.  `None` while EMPTY.
    pub(crate) target_blob: Option<ObjectId>,
    /// Monotonic counter — incremented on `abort()` AND on each
    /// `readAs*()`.  Drain snapshots vs current to invalidate stale
    /// completion if abort intervened OR a new read superseded.
    pub(crate) abort_seq: u32,
    // Event handler attributes (FileAPI §6.2 IDL) — `None` until
    // user JS assigns.  Direct callable invocation (NOT
    // dispatchEvent — see module doc on event-delivery scope).
    pub(crate) onloadstart: Option<ObjectId>,
    pub(crate) onprogress: Option<ObjectId>,
    pub(crate) onload: Option<ObjectId>,
    pub(crate) onloadend: Option<ObjectId>,
    pub(crate) onerror: Option<ObjectId>,
    pub(crate) onabort: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `FileReader.prototype` (chains to
    /// `EventTarget.prototype`), install accessor / method suite,
    /// expose `FileReader` constructor on `globals` along with EMPTY
    /// / LOADING / DONE constants on both ctor and prototype.
    pub(in crate::vm) fn register_file_reader_global(&mut self) {
        let et_proto = self
            .event_target_prototype
            .expect("register_file_reader_global called before register_event_target_prototype");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(et_proto),
            extensible: true,
        });
        self.install_file_reader_members(proto_id);
        self.file_reader_prototype = Some(proto_id);

        let global_sid = self.well_known.file_reader_global;
        install_ctor(
            self,
            proto_id,
            "FileReader",
            native_file_reader_constructor,
            global_sid,
        );

        // Install EMPTY / LOADING / DONE constants on BOTH the ctor
        // and the prototype per FileAPI §6.  Use the ctor we just
        // installed (now reachable via `self.globals[global_sid]`).
        let Some(JsValue::Object(ctor_id)) = self.globals.get(&global_sid).copied() else {
            return;
        };
        let constants: [(StringId, u8); 3] = [
            (self.well_known.empty_const, 0),
            (self.well_known.loading_const, 1),
            (self.well_known.done_const, 2),
        ];
        for (name_sid, value) in constants {
            let key = super::super::value::PropertyKey::String(name_sid);
            self.define_shaped_property(
                ctor_id,
                key,
                PropertyValue::Data(JsValue::Number(f64::from(value))),
                PropertyAttrs::BUILTIN,
            );
            self.define_shaped_property(
                proto_id,
                key,
                PropertyValue::Data(JsValue::Number(f64::from(value))),
                PropertyAttrs::BUILTIN,
            );
        }
    }

    fn install_file_reader_members(&mut self, proto_id: ObjectId) {
        // Read-only accessors: readyState / result / error.
        let ro_accessors: [(StringId, NativeFn); 3] = [
            (
                self.well_known.ready_state,
                native_fr_get_ready_state as NativeFn,
            ),
            (
                self.well_known.result_attr,
                native_fr_get_result as NativeFn,
            ),
            (self.well_known.error, native_fr_get_error as NativeFn),
        ];
        for (name_sid, getter) in ro_accessors {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Event handler attributes: 6 RW getter/setter pairs.
        let on_handlers: [(StringId, NativeFn, NativeFn); 6] = [
            (
                self.well_known.onloadstart,
                native_fr_get_onloadstart as NativeFn,
                native_fr_set_onloadstart as NativeFn,
            ),
            (
                self.well_known.onprogress,
                native_fr_get_onprogress as NativeFn,
                native_fr_set_onprogress as NativeFn,
            ),
            (
                self.well_known.onload,
                native_fr_get_onload as NativeFn,
                native_fr_set_onload as NativeFn,
            ),
            (
                self.well_known.onloadend,
                native_fr_get_onloadend as NativeFn,
                native_fr_set_onloadend as NativeFn,
            ),
            (
                self.well_known.onerror,
                native_fr_get_onerror as NativeFn,
                native_fr_set_onerror as NativeFn,
            ),
            (
                self.well_known.onabort,
                native_fr_get_onabort as NativeFn,
                native_fr_set_onabort as NativeFn,
            ),
        ];
        for (name_sid, getter, setter) in on_handlers {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Methods: 4 readAs* + abort.
        let methods: [(StringId, NativeFn); 5] = [
            (
                self.well_known.read_as_text,
                native_fr_read_as_text as NativeFn,
            ),
            (
                self.well_known.read_as_array_buffer,
                native_fr_read_as_array_buffer as NativeFn,
            ),
            (
                self.well_known.read_as_data_url,
                native_fr_read_as_data_url as NativeFn,
            ),
            (
                self.well_known.read_as_binary_string,
                native_fr_read_as_binary_string as NativeFn,
            ),
            (self.well_known.abort, native_fr_abort as NativeFn),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_file_reader_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "FileReader.prototype.{method} called on non-FileReader"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::FileReader) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "FileReader.prototype.{method} called on non-FileReader"
        )))
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

#[allow(clippy::needless_pass_by_value)]
fn native_file_reader_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'FileReader': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::FileReader;
    ctx.vm
        .file_reader_data
        .insert(inst_id, FileReaderSideData::default());
    Ok(JsValue::Object(inst_id))
}

// ---------------------------------------------------------------------------
// Accessors — readyState / result / error
// ---------------------------------------------------------------------------

fn native_fr_get_ready_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_reader_this(ctx, this, "readyState")?;
    let state = ctx
        .vm
        .file_reader_data
        .get(&id)
        .map_or(ReadyState::Empty, |d| d.state);
    Ok(JsValue::Number(f64::from(state as u8)))
}

fn native_fr_get_result(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_reader_this(ctx, this, "result")?;
    // Spec §6.2 — `result` returns null while state != DONE OR there
    // is an error.  Snapshot to drop the borrow before we intern.
    let snap = ctx
        .vm
        .file_reader_data
        .get(&id)
        .map(|d| (d.state, d.result.clone(), d.error.is_some()));
    let Some((state, result, has_error)) = snap else {
        return Ok(JsValue::Null);
    };
    if state != ReadyState::Done || has_error {
        return Ok(JsValue::Null);
    }
    Ok(match result {
        ReaderResult::None => JsValue::Null,
        ReaderResult::Text(s) | ReaderResult::DataUrl(s) | ReaderResult::BinaryString(s) => {
            JsValue::String(ctx.vm.strings.intern(&s))
        }
        ReaderResult::ArrayBuffer(buf_id) => JsValue::Object(buf_id),
    })
}

fn native_fr_get_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_reader_this(ctx, this, "error")?;
    Ok(ctx
        .vm
        .file_reader_data
        .get(&id)
        .and_then(|d| d.error)
        .map_or(JsValue::Null, JsValue::Object))
}

// ---------------------------------------------------------------------------
// Event handler attribute accessors — 6 getter/setter pairs
// ---------------------------------------------------------------------------

macro_rules! fr_on_handler {
    ($getter:ident, $setter:ident, $field:ident, $name:literal) => {
        fn $getter(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let id = require_file_reader_this(ctx, this, $name)?;
            Ok(ctx
                .vm
                .file_reader_data
                .get(&id)
                .and_then(|d| d.$field)
                .map_or(JsValue::Null, JsValue::Object))
        }
        fn $setter(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let id = require_file_reader_this(ctx, this, $name)?;
            let new_val = args.first().copied().unwrap_or(JsValue::Undefined);
            // Spec: only callable values are retained; any other
            // value silently nulls the slot (matches Chrome/Firefox).
            let stored = match new_val {
                JsValue::Object(obj_id) if ctx.vm.get_object(obj_id).kind.is_callable() => {
                    Some(obj_id)
                }
                _ => None,
            };
            if let Some(d) = ctx.vm.file_reader_data.get_mut(&id) {
                d.$field = stored;
            }
            Ok(JsValue::Undefined)
        }
    };
}

fr_on_handler!(
    native_fr_get_onloadstart,
    native_fr_set_onloadstart,
    onloadstart,
    "onloadstart"
);
fr_on_handler!(
    native_fr_get_onprogress,
    native_fr_set_onprogress,
    onprogress,
    "onprogress"
);
fr_on_handler!(native_fr_get_onload, native_fr_set_onload, onload, "onload");
fr_on_handler!(
    native_fr_get_onloadend,
    native_fr_set_onloadend,
    onloadend,
    "onloadend"
);
fr_on_handler!(
    native_fr_get_onerror,
    native_fr_set_onerror,
    onerror,
    "onerror"
);
fr_on_handler!(
    native_fr_get_onabort,
    native_fr_set_onabort,
    onabort,
    "onabort"
);

// ---------------------------------------------------------------------------
// readAs* methods — synchronously enter LOADING + enqueue task
// ---------------------------------------------------------------------------

fn read_as_common(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    kind: ReadKind,
    method: &str,
) -> Result<JsValue, VmError> {
    let id = require_file_reader_this(ctx, this, method)?;
    // Validate blob argument (must be Blob | File).
    let blob_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let blob_id = match blob_arg {
        JsValue::Object(b_id) => {
            if matches!(
                ctx.vm.get_object(b_id).kind,
                ObjectKind::Blob | ObjectKind::File
            ) {
                b_id
            } else {
                return Err(VmError::type_error(format!(
                    "Failed to execute '{method}' on 'FileReader': parameter 1 is not of type 'Blob'."
                )));
            }
        }
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'FileReader': parameter 1 is not of type 'Blob'."
            )));
        }
    };
    // Spec §6.4 step 1 — InvalidStateError if state == LOADING.
    let was_loading = ctx
        .vm
        .file_reader_data
        .get(&id)
        .is_some_and(|d| d.state == ReadyState::Loading);
    if was_loading {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'FileReader': The object is already busy reading Blobs.",
            ),
        ));
    }
    // Optional encoding arg (readAsText only).
    let encoding = if kind == ReadKind::Text {
        match args.get(1).copied() {
            Some(JsValue::Undefined) | None => None,
            Some(v) => Some(super::super::coerce::to_string(ctx.vm, v)?),
        }
    } else {
        None
    };

    // Transition LOADING + bump abort_seq (so any stale task from a
    // prior aborted+restarted reader is invalidated).
    let abort_seq = {
        let d = ctx.vm.file_reader_data.entry(id).or_default();
        d.state = ReadyState::Loading;
        d.target_blob = Some(blob_id);
        d.result = ReaderResult::None;
        d.error = None;
        d.abort_seq = d.abort_seq.wrapping_add(1);
        d.abort_seq
    };

    // Fire `loadstart` synchronously (before queueing the task) per
    // FileAPI §6.5.
    fire_progress_event(ctx, id, EventKind::Loadstart, None);

    // Queue the actual read for drain.
    ctx.vm
        .queue_task(super::pending_tasks::PendingTask::FileRead {
            reader_id: id,
            abort_seq_snapshot: abort_seq,
            kind,
            encoding,
        });

    Ok(JsValue::Undefined)
}

fn native_fr_read_as_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_as_common(ctx, this, args, ReadKind::Text, "readAsText")
}

fn native_fr_read_as_array_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_as_common(ctx, this, args, ReadKind::ArrayBuffer, "readAsArrayBuffer")
}

fn native_fr_read_as_data_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_as_common(ctx, this, args, ReadKind::DataUrl, "readAsDataURL")
}

fn native_fr_read_as_binary_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_as_common(
        ctx,
        this,
        args,
        ReadKind::BinaryString,
        "readAsBinaryString",
    )
}

// ---------------------------------------------------------------------------
// abort()
// ---------------------------------------------------------------------------

fn native_fr_abort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_reader_this(ctx, this, "abort")?;
    // Per FileAPI §6.4 abort algorithm:
    // 1. If state != LOADING return (early exit, no event fires).
    // 2. Else: set state = DONE, null result, fire `abort` + `loadend`.
    // Also: bump abort_seq so the in-flight task discards on drain.
    let was_loading = ctx
        .vm
        .file_reader_data
        .get(&id)
        .is_some_and(|d| d.state == ReadyState::Loading);
    if !was_loading {
        return Ok(JsValue::Undefined);
    }
    // Capture blob size BEFORE clearing target_blob so abort + loadend
    // can fire with meaningful `loaded` / `total` (this implementation
    // reads synchronously, so on abort the blob's full byte length is
    // the conceptual "processed" amount).  Without this snapshot,
    // `fire_progress_event` would see target_blob = None and emit
    // loaded = total = 0 — observer-side regression.
    let blob_size_for_abort = ctx
        .vm
        .file_reader_data
        .get(&id)
        .and_then(|d| d.target_blob)
        .map_or(0.0, |b_id| {
            #[allow(clippy::cast_precision_loss)]
            let n = super::blob::blob_byte_length(ctx.vm, b_id) as f64;
            n
        });
    if let Some(d) = ctx.vm.file_reader_data.get_mut(&id) {
        d.state = ReadyState::Done;
        d.result = ReaderResult::None;
        d.abort_seq = d.abort_seq.wrapping_add(1);
        d.target_blob = None;
    }
    fire_progress_event(ctx, id, EventKind::Abort, Some(blob_size_for_abort));
    fire_progress_event(ctx, id, EventKind::Loadend, Some(blob_size_for_abort));
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Drain (called from pending_tasks::dispatch_file_read in Phase 4)
// ---------------------------------------------------------------------------

/// Execute a queued `PendingTask::FileRead`.  Invoked from
/// `pending_tasks.rs::dispatch_file_read`.  Spec §6.5 event sequence
/// (post-loadstart, fired here on completion):
/// `progress` → `load` | `error` → `loadend`.
pub(crate) fn dispatch_file_read_task(
    vm: &mut VmInner,
    reader_id: ObjectId,
    abort_seq_snapshot: u32,
    kind: ReadKind,
    encoding: Option<StringId>,
) {
    // Stale-snapshot check: abort_seq may have been bumped by abort()
    // or a superseding readAs*().  Silent-discard if so — the abort
    // path or the new read owns subsequent state transitions.
    let target_blob = {
        let Some(d) = vm.file_reader_data.get(&reader_id) else {
            return;
        };
        if d.abort_seq != abort_seq_snapshot {
            return;
        }
        d.target_blob
    };
    let Some(blob_id) = target_blob else {
        return;
    };
    // Snapshot bytes + Blob MIME type (drop borrow before mutating).
    let (bytes, type_sid) = {
        let bytes = super::blob::blob_bytes(vm, blob_id);
        let type_sid = super::blob::blob_type(vm, blob_id);
        (bytes, type_sid)
    };

    // In-memory blob reads are infallible — Phase 4 has no I/O path
    // that can fail.  When remote-blob support lands, switch the
    // decode arms to return `Result` and re-introduce the
    // NotReadableError branch (see `#11-filereader-event-listeners`
    // sibling defer).
    let result = match kind {
        ReadKind::Text => decode_as_text(vm, &bytes, encoding, type_sid),
        ReadKind::ArrayBuffer => {
            let buf_id = super::array_buffer::create_array_buffer_from_bytes(vm, bytes.to_vec());
            ReaderResult::ArrayBuffer(buf_id)
        }
        ReadKind::DataUrl => ReaderResult::DataUrl(encode_data_url(vm, &bytes, type_sid)),
        ReadKind::BinaryString => ReaderResult::BinaryString(decode_binary_string(&bytes)),
    };

    // Re-check abort_seq AFTER decode — abort_seq could change during
    // a re-entrant decode path that allocates ArrayBuffer (which can
    // trigger GC).  For string-based kinds the intern + decode is
    // borrow-safe; ArrayBuffer alloc is the only realistic risk site.
    let still_current = vm
        .file_reader_data
        .get(&reader_id)
        .is_some_and(|d| d.abort_seq == abort_seq_snapshot);
    if !still_current {
        return;
    }

    // Build minimal NativeContext for the event-fire phase.  Keep
    // `target_blob` populated through the event-fire sequence so
    // `ProgressEvent.loaded` / `.total` can read the blob's byte
    // length; cleared after `loadend` returns.
    let mut ctx = super::super::value::NativeContext { vm };
    if let Some(d) = ctx.vm.file_reader_data.get_mut(&reader_id) {
        d.state = ReadyState::Done;
        d.result = result;
        d.error = None;
    }
    fire_progress_event(&mut ctx, reader_id, EventKind::Progress, None);
    fire_progress_event(&mut ctx, reader_id, EventKind::Load, None);
    fire_progress_event(&mut ctx, reader_id, EventKind::Loadend, None);
    // Clear target_blob post-events so subsequent `readAs*` calls
    // don't observe a stale blob reference.  Result + error remain
    // populated for retained `r.result` / `r.error` reads.
    if let Some(d) = ctx.vm.file_reader_data.get_mut(&reader_id) {
        d.target_blob = None;
    }
}

// ---------------------------------------------------------------------------
// Event fire (on* handler attribute invocation only)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum EventKind {
    Loadstart,
    Progress,
    Load,
    Loadend,
    Abort,
}

fn fire_progress_event(
    ctx: &mut NativeContext<'_>,
    reader_id: ObjectId,
    kind: EventKind,
    blob_size_override: Option<f64>,
) {
    let handler = ctx
        .vm
        .file_reader_data
        .get(&reader_id)
        .and_then(|d| match kind {
            EventKind::Loadstart => d.onloadstart,
            EventKind::Progress => d.onprogress,
            EventKind::Load => d.onload,
            EventKind::Loadend => d.onloadend,
            EventKind::Abort => d.onabort,
        });
    let Some(handler_id) = handler else {
        return;
    };

    let type_sid = match kind {
        EventKind::Loadstart => ctx.vm.well_known.loadstart_event_type,
        EventKind::Progress => ctx.vm.well_known.progress_event_type,
        EventKind::Load => ctx.vm.well_known.load_event_type,
        EventKind::Loadend => ctx.vm.well_known.loadend_event_type,
        EventKind::Abort => ctx.vm.well_known.abort,
    };
    let blob_size = blob_size_override.unwrap_or_else(|| {
        ctx.vm
            .file_reader_data
            .get(&reader_id)
            .and_then(|d| d.target_blob)
            .map_or(0.0, |b_id| {
                #[allow(clippy::cast_precision_loss)]
                let n = super::blob::blob_byte_length(ctx.vm, b_id) as f64;
                n
            })
    });

    // Build via the shared ctor helper so the event lands on the
    // `progress_event` precomputed shape (core-9 + lengthComputable /
    // loaded / total) — avoids forking a fresh shape transition
    // tree per FileReader fire and picks up timeStamp / eventPhase /
    // isTrusted in the standard slot positions.
    let shape_id = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing — register_globals did not run")
        .progress_event;
    let payload_slots = vec![
        PropertyValue::Data(JsValue::Boolean(true)),
        PropertyValue::Data(JsValue::Number(blob_size)),
        PropertyValue::Data(JsValue::Number(blob_size)),
    ];
    let event_id = ctx.vm.create_fresh_event_object(
        JsValue::Undefined,
        type_sid,
        EventInit::default(),
        shape_id,
        payload_slots,
        true,
    );
    // Override prototype to ProgressEvent.prototype so
    // `e instanceof ProgressEvent` holds; `create_fresh_event_object`
    // defaults to `Event.prototype` for non-construct allocation.
    if let Some(proto) = ctx.vm.progress_event_prototype {
        ctx.vm.get_object_mut(event_id).prototype = Some(proto);
    }
    // UA-fire: target == currentTarget == reader.  Core-9 slots are
    // ctor-seeded to null; `set_event_slot_raw` overwrites in-place
    // without a shape transition.
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_TARGET,
        JsValue::Object(reader_id),
    );
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_CURRENT_TARGET,
        JsValue::Object(reader_id),
    );

    // Errors swallowed per WHATWG IDL event-handler attribute semantics
    // (uncaught exceptions log to console, don't propagate).
    let _ = ctx.call_function(
        handler_id,
        JsValue::Object(reader_id),
        &[JsValue::Object(event_id)],
    );
}

// ---------------------------------------------------------------------------
// Decoders
// ---------------------------------------------------------------------------

/// `readAsText` decoder — FileAPI §6.3 encoding-determination
/// 4-step fallback chain.
fn decode_as_text(
    vm: &VmInner,
    bytes: &Arc<[u8]>,
    arg_encoding: Option<StringId>,
    blob_type_sid: StringId,
) -> ReaderResult {
    // Step 1: user-provided arg label.
    if let Some(label_sid) = arg_encoding {
        let label = vm.strings.get_utf8(label_sid);
        if let Some(enc) = encoding_rs::Encoding::for_label_no_replacement(label.as_bytes()) {
            let (text, _, _) = enc.decode(bytes);
            return ReaderResult::Text(text.into_owned());
        }
    }
    // Step 2: parse Blob.type for `charset=…`.
    let blob_type = vm.strings.get_utf8(blob_type_sid);
    if let Some(charset) = parse_charset_from_mime(&blob_type) {
        if let Some(enc) = encoding_rs::Encoding::for_label_no_replacement(charset.as_bytes()) {
            let (text, _, _) = enc.decode(bytes);
            return ReaderResult::Text(text.into_owned());
        }
    }
    // Step 3: BOM sniff (first 3 bytes for UTF-8, first 2 for UTF-16).
    let bom_enc = sniff_bom(bytes);
    if let Some(enc) = bom_enc {
        let (text, _, _) = enc.decode(bytes);
        return ReaderResult::Text(text.into_owned());
    }
    // Step 4: fallback UTF-8.
    let (text, _, _) = encoding_rs::UTF_8.decode(bytes);
    ReaderResult::Text(text.into_owned())
}

/// Extract `charset=…` parameter from a MIME type string (case-insensitive
/// key match).  Returns the value verbatim (encoding_rs lookup is
/// case-insensitive).  Tolerates whitespace / quoted values minimally.
fn parse_charset_from_mime(mime: &str) -> Option<&str> {
    for param in mime.split(';').skip(1) {
        let param = param.trim();
        if let Some(eq) = param.find('=') {
            let (k, v) = param.split_at(eq);
            if k.trim().eq_ignore_ascii_case("charset") {
                let v = v[1..].trim().trim_matches('"');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn sniff_bom(bytes: &[u8]) -> Option<&'static encoding_rs::Encoding> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some(encoding_rs::UTF_8)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some(encoding_rs::UTF_16BE)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some(encoding_rs::UTF_16LE)
    } else {
        None
    }
}

/// `readAsDataURL` encoder — `data:<type>;base64,<base64>` per
/// FileAPI §6.3 "package data" algorithm.  Empty `type` retains the
/// semicolon (`data:;base64,...`) per RFC 2397 + Chrome/Firefox
/// parity.
fn encode_data_url(vm: &VmInner, bytes: &Arc<[u8]>, type_sid: StringId) -> String {
    let mime = vm.strings.get_utf8(type_sid);
    let prefix_len = "data:".len() + mime.len() + ";base64,".len();
    let body_len = bytes.len().saturating_mul(4).div_ceil(3) + 4;
    let mut out = String::with_capacity(prefix_len + body_len);
    out.push_str("data:");
    out.push_str(&mime);
    out.push_str(";base64,");
    BASE64_STANDARD.encode_string(bytes.as_ref(), &mut out);
    out
}

/// `readAsBinaryString` decoder — 1 byte → 1 UTF-16 code unit per
/// legacy FileAPI §6.3 (deprecated but mandated).  Output is a string
/// of length `bytes.len()` where each char is `bytes[i] as char`;
/// non-ASCII bytes expand to 2-byte UTF-8 sequences, so worst-case
/// capacity is `2 * bytes.len()`.
fn decode_binary_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for &b in bytes {
        out.push(b as char);
    }
    out
}
