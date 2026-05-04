//! Body coercion for `BodyInit` (WHATWG Fetch §5 "extract a body").
//!
//! Split from [`super`] (`request_response/mod.rs`) to keep each
//! file under the project's 1000-line convention.  Both helpers
//! are shared between the Request constructor (`init.body`) and
//! the Response constructor (positional `body` argument), and
//! between those and the `fetch()` host's URL-input init.body
//! parsing path.

#![cfg(feature = "engine")]

use super::super::super::value::{JsValue, NativeContext, ObjectKind, StringId, VmError};

/// Coerce a body init value into raw UTF-8 bytes plus, when
/// applicable, an override Content-Type that supersedes the
/// generic [`content_type_for_body`] mapping (the encoder-derived
/// boundary for `multipart/form-data` is only known after the body
/// is serialised, so it has to be returned together with the
/// bytes).  Accepts `String` / `ArrayBuffer` / `Blob` /
/// `URLSearchParams` / `FormData` / `BufferSource` views (per
/// WHATWG §5 "extract a body"); any other non-null / non-undefined
/// value is `ToString`-coerced, matching browsers' forgiving
/// `new Request(url, {body: 42})` → `"42"` behaviour.
///
/// `ReadableStream` lands with the PR5-streams tranche.
///
/// `pub(in crate::vm::host)` so the `fetch()` host (`vm/host/fetch.rs`)
/// can reuse the exact same coercion path for `init.body` without
/// duplicating the ArrayBuffer / Blob extraction branches — the
/// two code paths would otherwise drift.
#[allow(clippy::type_complexity)] // private return shape, refactor would not improve clarity
pub(in crate::vm::host) fn extract_body_bytes(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<Option<(Vec<u8>, Option<StringId>)>, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::String(sid) => {
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(Some((raw.into_bytes(), None)))
        }
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::ArrayBuffer => Ok(Some((
                super::super::array_buffer::array_buffer_bytes(ctx.vm, obj_id),
                None,
            ))),
            ObjectKind::Blob => {
                // `BlobData.bytes` is the source of truth as
                // `Arc<[u8]>` (per-spec immutable).  Snapshot the
                // bytes into a fresh Vec at the pool boundary so
                // the new body owns its bytes independently.
                Ok(Some((
                    super::super::blob::blob_bytes(ctx.vm, obj_id).to_vec(),
                    None,
                )))
            }
            // TypedArray / DataView as BufferSource (WHATWG Fetch
            // §5 — BodyInit union accepts any BufferSource).
            // Extract the view's byte range from the underlying
            // ArrayBuffer.
            ObjectKind::TypedArray {
                buffer_id,
                byte_offset,
                byte_length,
                ..
            }
            | ObjectKind::DataView {
                buffer_id,
                byte_offset,
                byte_length,
            } => Ok(Some((
                super::super::array_buffer::array_buffer_view_bytes(
                    ctx.vm,
                    buffer_id,
                    byte_offset,
                    byte_length,
                ),
                None,
            ))),
            ObjectKind::URLSearchParams => {
                // Always serialise via the `serialize_for_body`
                // helper so the wire bytes match `toString()`'s
                // observable output.
                let serialized =
                    super::super::url_search_params::serialize_for_body(ctx.vm, obj_id);
                Ok(Some((serialized.into_bytes(), None)))
            }
            ObjectKind::FormData => {
                // Snapshot the entry list because the multipart
                // encoder needs `&VmInner` (read-only) and we
                // cannot keep a `&mut` borrow open across the
                // subsequent `intern` of the boundary string.
                let entries = ctx
                    .vm
                    .form_data_states
                    .get(&obj_id)
                    .cloned()
                    .unwrap_or_default();
                let (body, boundary) = super::super::multipart::encode(ctx.vm, &entries);
                let prefix = ctx
                    .vm
                    .strings
                    .get_utf8(ctx.vm.well_known.multipart_form_data_prefix);
                let ct_string = format!("{prefix}{boundary}");
                let ct_sid = ctx.vm.strings.intern(&ct_string);
                Ok(Some((body, Some(ct_sid))))
            }
            // ReadableStream as request body input: explicit
            // TypeError until M4-13.2 PR-streams-body-input wires
            // up async lazy-drain.  Phase 2 only supports
            // ReadableStream as response *output* (Response.body /
            // Blob.stream()).  The error message matches the
            // intent of Chromium's "ReadableStream upload not yet
            // supported" — clear per-spec failure rather than a
            // silent string-coercion (which would land
            // `[object ReadableStream]` bytes on the wire).
            ObjectKind::ReadableStream => Err(VmError::type_error(
                "ReadableStream bodies are not yet supported",
            )),
            _ => {
                // Generic fallback: stringify.  Covers plain
                // objects / Arrays / numbers once wrapped.
                let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
                let raw = ctx.vm.strings.get_utf8(sid);
                Ok(Some((raw.into_bytes(), None)))
            }
        },
        _ => {
            // String coercion covers number / bool / symbol-throws,
            // matching browsers' `new Request(url, {body: 42})` → "42".
            let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            Ok(Some((raw.into_bytes(), None)))
        }
    }
}

/// Default `Content-Type` for a body argument (WHATWG §5 "extract
/// a body").  `String` bodies default to
/// `"text/plain;charset=UTF-8"`; `Blob` bodies carry their own
/// `type` (or nothing if the Blob's type is empty);
/// `URLSearchParams` bodies default to
/// `"application/x-www-form-urlencoded;charset=UTF-8"`.
/// `ArrayBuffer` has no default CT — matches spec (§5 step 4.7
/// "If object is a BufferSource, ... set Content-Type to null").
///
/// `FormData` is **not** handled here — its boundary-bearing
/// `Content-Type` is computed inline by [`extract_body_bytes`]
/// because the boundary is only known after serialisation.  Builds
/// that consult `content_type_for_body` for a FormData body
/// receive `None`; the [`super::response_ctor::build_response_instance`]
/// / [`super::request_ctor`] paths thread the override returned
/// by `extract_body_bytes` ahead of this fallback.
pub(in crate::vm::host) fn content_type_for_body(
    ctx: &NativeContext<'_>,
    body: JsValue,
) -> Option<StringId> {
    match body {
        JsValue::String(_) => Some(ctx.vm.well_known.text_plain_charset_utf8),
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::Blob => {
                let ty = super::super::blob::blob_type(ctx.vm, obj_id);
                // An empty type means "don't expose a Content-Type"
                // per WHATWG §5 step 4.4.3 "If object's type
                // attribute is not the empty string, set
                // Content-Type to its value".
                if ty == ctx.vm.well_known.empty {
                    None
                } else {
                    Some(ty)
                }
            }
            ObjectKind::URLSearchParams => Some(ctx.vm.well_known.application_form_urlencoded),
            // `ArrayBuffer` / `FormData` / others fall through to
            // `None` — see fn doc.
            _ => None,
        },
        _ => None,
    }
}
