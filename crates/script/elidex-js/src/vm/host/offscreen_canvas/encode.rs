//! `OffscreenCanvas.convertToBlob(options)` (WHATWG HTML §4.12.5.1.7 —
//! "convertToBlob(options)" algorithm, which dispatches to "serialize a bitmap
//! to a file"). Options parsing + format dispatch + Promise glue ONLY —
//! the actual encoding lives in `elidex_web_canvas::Canvas2dContext::encode_blob`
//! per the Layering mandate.

#![cfg(feature = "engine")]

use elidex_api_canvas::{offscreen_canvas_dimensions, with_context};
use elidex_web_canvas::BlobImageFormat;

use super::super::super::coerce;
use super::super::super::value::{JsValue, NativeContext, PropertyKey, VmError};
use super::require_offscreen_canvas;

/// `OffscreenCanvas.prototype.convertToBlob(options?)` (WHATWG HTML §4.12.5.1.7).
///
/// Returns `Promise<Blob>`. Rejection mapping mirrors the spec algorithm
/// (steps numbered per HTML §4.12.5.1.7 "convertToBlob(options)" + dispatched
/// "serialize a bitmap to a file" §4.12.5.1.7.4):
///
/// - **`IndexSizeError`** — step "If this OffscreenCanvas object's bitmap has
///   no pixels (i.e. either its horizontal dimension or its vertical dimension
///   is zero)".
/// - **`InvalidStateError`** — step "If this OffscreenCanvas object's context
///   mode is set to none" (no `getContext('2d')` call yet, so no
///   [`elidex_web_canvas::Canvas2dContext`] component on the entity).
/// - **`EncodingError`** — "serialize a bitmap to a file" step "If file is
///   null, reject result with an EncodingError DOMException" (encoder
///   returned `None`, or the defensive empty-bytes guard fired).
///
/// Options: `{ type: DOMString = "image/png", quality: unrestricted double = 1.0 }`.
/// Unknown / unsupported `type` falls back to `image/png` per spec (see
/// `BlobImageFormat::from_mime`).
///
/// v1 resolves synchronously on the next microtask (D-14 `Blob.text` precedent
/// — the spec mandates async, but the bitmap snapshot + encode happen on the
/// main thread without an actual worker queue; the observable difference is
/// only timing under heavy load, not correctness).
pub(super) fn native_oc_convert_to_blob(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_offscreen_canvas(ctx, this, "convertToBlob")?;
    let options = args.first().copied().unwrap_or(JsValue::Undefined);
    let (format, quality) = parse_convert_options(ctx, options)?;
    let promise = super::super::super::natives_promise::create_promise(ctx.vm);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));

    // Step "If bitmap has no pixels": zero on either axis → IndexSizeError.
    // `HostData::dom` requires `&mut self`, but the read is logically
    // immutable; the borrow ends with the let-bind so the later
    // `with_context` call can re-acquire `as_deref_mut`.
    let dims = g
        .host_data
        .as_deref_mut()
        .map(|hd| offscreen_canvas_dimensions(hd.dom(), entity));
    if matches!(dims, Some((0, _) | (_, 0))) {
        let exc = g.well_known.dom_exc_index_size_error;
        let reason = g.build_dom_exception(
            exc,
            "Failed to execute 'convertToBlob' on 'OffscreenCanvas': bitmap has no pixels.",
        );
        super::super::blob::reject_promise_sync(&mut g, promise, reason);
        drop(g);
        return Ok(JsValue::Object(promise));
    }

    // Snapshot the encoded bytes synchronously. The nested `Option<Option<_>>`
    // differentiates the two failure modes per spec:
    //   `None`             → no `Canvas2dContext` component (context mode = none) → InvalidStateError
    //   `Some(None)`       → encoder returned `None` → EncodingError
    //   `Some(Some(vec))`  → success (defensive empty-Vec guard maps to EncodingError)
    //
    // Borrow split: `with_context` needs `&mut EcsDom`, which means a
    // `&mut HostData` (the only owner of the DOM). Reach it through the
    // rooted `&mut VmInner` guard via the standard `host_data.as_deref_mut`
    // path (mirrors `vm_api.rs::unbind` and blob.rs accessors).
    let encoded = g
        .host_data
        .as_deref_mut()
        .and_then(|hd| with_context(hd.dom(), entity, |c| c.encode_blob(format, quality)));

    match encoded {
        Some(Some(bytes)) if !bytes.is_empty() => {
            // Materialize a Blob whose `type` matches the negotiated format
            // (post-fallback per `from_mime`).
            let type_sid = g.strings.intern(format.mime());
            let blob_id =
                super::super::blob::create_blob_from_bytes(&mut g, bytes.into(), type_sid);
            super::super::blob::resolve_promise_sync(&mut g, promise, JsValue::Object(blob_id));
        }
        Some(_) => {
            // Encoder failed OR returned an empty byte stream. The empty case
            // is never a valid image-format payload (every supported encoder
            // emits a non-empty header), so a defensive treatment as encoder
            // failure preserves the spec contract.
            let exc = g.well_known.dom_exc_encoding_error;
            let reason = g.build_dom_exception(
                exc,
                "Failed to execute 'convertToBlob' on 'OffscreenCanvas': encoder failed to produce output.",
            );
            super::super::blob::reject_promise_sync(&mut g, promise, reason);
        }
        None => {
            let exc = g.well_known.dom_exc_invalid_state_error;
            let reason = g.build_dom_exception(
                exc,
                "Failed to execute 'convertToBlob' on 'OffscreenCanvas': context mode is none (no getContext call).",
            );
            super::super::blob::reject_promise_sync(&mut g, promise, reason);
        }
    }
    drop(g);
    Ok(JsValue::Object(promise))
}

/// Parse the optional `{ type, quality }` options dictionary. `type` is
/// passed raw to [`BlobImageFormat::from_mime`], which does WHATWG MIME
/// essence extraction + ASCII case-insensitive compare itself (unknown →
/// PNG fallback per spec). `quality` is `unrestricted double`; per WHATWG
/// HTML §4.12.5.1 wording "if quality is not in the closed interval
/// `[0.0, 1.0]`, let it be 1.0", any out-of-range value (including
/// negative, non-finite, NaN, > 1.0) is substituted with 1.0 — note this
/// is substitution, NOT clamping. Returns the negotiated format + quality.
fn parse_convert_options(
    ctx: &mut NativeContext<'_>,
    options: JsValue,
) -> Result<(BlobImageFormat, f32), VmError> {
    let opts_id = match options {
        JsValue::Undefined | JsValue::Null => return Ok((BlobImageFormat::Png, 1.0)),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'convertToBlob' on 'OffscreenCanvas': options must be an object",
            ));
        }
    };

    let type_sid = ctx.vm.strings.intern("type");
    let quality_sid = ctx.vm.strings.intern("quality");
    let type_val = ctx.get_property_value(opts_id, PropertyKey::String(type_sid))?;
    let quality_val = ctx.get_property_value(opts_id, PropertyKey::String(quality_sid))?;

    let format = match type_val {
        JsValue::Undefined => BlobImageFormat::Png,
        other => {
            let sid = coerce::to_string(ctx.vm, other)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            // `from_mime` does WHATWG MIME-parser essence extraction +
            // ASCII case-insensitive compare internally (handles
            // `"image/jpeg; charset=utf-8"`, leading/trailing whitespace,
            // mixed case), so callers pass raw user input verbatim.
            BlobImageFormat::from_mime(&raw)
        }
    };
    #[allow(clippy::cast_possible_truncation)]
    let quality = match quality_val {
        JsValue::Undefined => 1.0_f32,
        other => {
            let n = coerce::to_number(ctx.vm, other)?;
            // Per spec: "If quality is not in the closed interval [0.0, 1.0],
            // let it be 1.0" — covers NaN, ±∞, negative, and >1.0.
            if (0.0..=1.0).contains(&n) {
                n as f32
            } else {
                1.0
            }
        }
    };
    Ok((format, quality))
}
