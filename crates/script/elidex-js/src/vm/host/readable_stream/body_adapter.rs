//! Body integration helpers (§4.2 + WHATWG Fetch §5 .body).
//!
//! [`create_body_backed_stream`] wraps a one-shot byte payload as
//! a `ReadableStream` whose queue carries one `Uint8Array` chunk
//! and whose state is already `close_requested`.  Used by
//! `Request.body` / `Response.body` / `Blob.prototype.stream()`
//! to expose body bytes as a stream without an embedded JS source
//! callback.

use std::collections::VecDeque;

use super::super::super::shape::{self};
use super::super::super::value::{JsValue, Object, ObjectId, ObjectKind, PropertyStorage, VmError};
use super::super::super::VmInner;
use super::controller::{error_stream, finalize_close};
use super::{ReadableStreamState, ReadableStreamStateKind};

/// Allocate a fresh `ReadableStream` whose queue carries one
/// `Uint8Array` chunk built from `bytes` and whose state is
/// already `close_requested`.  Used by `Request.body` /
/// `Response.body` / `Blob.prototype.stream()` to expose body
/// bytes as a stream without an embedded JS source callback.
///
/// Phase-2 simplification: emits one chunk regardless of size.
/// Chunked streaming (e.g. broker push of partial response
/// payloads) lands with Phase 5 PR-streams-network.
pub(crate) fn create_body_backed_stream(vm: &mut VmInner, bytes: Vec<u8>) -> ObjectId {
    // TypedArray's `byte_length` is stored as `u32`; bodies > 4 GiB
    // would silently truncate, exposing a Uint8Array view that
    // doesn't cover the full payload.  Phase 2 doesn't yet split
    // oversized bodies into multiple chunks (Phase 5
    // PR-streams-network covers chunked emit), so for the rare
    // >4 GiB case we ship an immediately-errored stream — and we
    // skip the ArrayBuffer / Uint8Array allocation entirely
    // (Copilot R6 perf finding: there's no chunk to read so the
    // huge `Vec<u8>` would be wasted in `body_data`).
    let bytes_len = bytes.len();
    let oversize = bytes_len > u32::MAX as usize;
    let stream_proto = vm.readable_stream_prototype;
    let stream_id = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStream,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: stream_proto,
        extensible: true,
    });
    // Root `stream_id` across the controller (and possibly
    // ArrayBuffer + Uint8Array) allocations so the not-yet-
    // installed stream object can't be collected mid-construction
    // (Copilot R7 GC-safety finding).
    let mut g_s = vm.push_temp_root(JsValue::Object(stream_id));

    let controller_proto = g_s.readable_stream_default_controller_prototype;
    let controller_id = g_s.alloc_object(Object {
        kind: ObjectKind::ReadableStreamDefaultController { stream_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: controller_proto,
        extensible: true,
    });
    let mut g_c = g_s.push_temp_root(JsValue::Object(controller_id));

    // Materialise a single `Uint8Array` chunk from `bytes` only
    // when there's a non-empty payload to read.  Empty bodies and
    // oversize bodies skip the buffer + view alloc — the stream
    // closes (empty) or errors (oversize) without ever exposing
    // a chunk to user code.  R6 perf: avoids allocating two
    // unused `Object`s for those paths.
    let mut queue: VecDeque<(JsValue, f64)> = VecDeque::new();
    if !oversize && bytes_len > 0 {
        let buf_id = super::super::array_buffer::create_array_buffer_from_bytes(&mut g_c, bytes);
        // `buf_id` is unrooted-local until the typed-array view
        // captures it — root it across the next alloc.
        let mut g_b = g_c.push_temp_root(JsValue::Object(buf_id));
        #[allow(clippy::cast_possible_truncation)]
        let byte_length = bytes_len as u32;
        let element_kind = super::super::super::value::ElementKind::Uint8;
        let typed_proto = g_b.subclass_array_prototypes[element_kind.index()];
        let typed_id = g_b.alloc_object(Object {
            kind: ObjectKind::TypedArray {
                buffer_id: buf_id,
                byte_offset: 0,
                byte_length,
                element_kind,
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: typed_proto,
            extensible: true,
        });
        queue.push_back((JsValue::Object(typed_id), 1.0));
        drop(g_b);
    }
    // (For empty / oversize, `bytes` is dropped here — its
    // backing allocation is freed at end of scope.  Oversize
    // bodies in particular avoid moving GiBs into `body_data`
    // only to error the stream right after.)
    let queue_total_size = if queue.is_empty() { 0.0 } else { 1.0 };

    g_c.readable_stream_states.insert(
        stream_id,
        ReadableStreamState {
            state: ReadableStreamStateKind::Readable,
            controller_id,
            reader_id: None,
            queue,
            queue_total_size,
            high_water_mark: 1.0,
            size_algorithm: None,
            start_called: true,
            pull_in_flight: false,
            pull_again: false,
            close_requested: true,
            source_start: None,
            source_pull: None,
            source_cancel: None,
            underlying_source: None,
            stored_error: JsValue::Undefined,
        },
    );

    // Stream is `Readable` with a queued chunk + close_requested
    // — the spec's "wait for the queue to drain before closing"
    // path matches exactly.  Oversize must check FIRST: a
    // `finalize_close` flips state to Closed, after which
    // `error_stream` early-returns and the oversize stream
    // would silently report `done: true` instead of rejecting
    // (R5 finding).  Empty bodies have no chunk to read so the
    // close finalises immediately.
    if oversize {
        let err = VmError::range_error(
            "Failed to materialise body stream: payload exceeds 4 GiB Uint8Array view limit",
        );
        let reason = g_c.vm_error_to_thrown(&err);
        error_stream(&mut g_c, stream_id, reason);
    } else if bytes_len == 0 {
        finalize_close(&mut g_c, stream_id);
    }
    drop(g_c);
    drop(g_s);
    stream_id
}
