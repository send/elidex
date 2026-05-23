//! `OffscreenCanvas.convertToBlob(options)` (WHATWG HTML §4.12.5.1.7 —
//! "convertToBlob(options)" algorithm, which dispatches to "serialize a bitmap
//! to a file"). Options parsing + format dispatch + Promise glue ONLY —
//! the actual encoding lives in `elidex_web_canvas::Canvas2dContext::encode_blob`
//! per the Layering mandate.

#![cfg(feature = "engine")]

use elidex_api_canvas::with_context;
use elidex_web_canvas::BlobImageFormat;

use super::super::super::coerce;
use super::super::super::value::{JsValue, NativeContext, PropertyKey, VmError};
use super::require_offscreen_canvas;

/// `OffscreenCanvas.prototype.convertToBlob(options?)` (WHATWG HTML §4.12.5.1.7).
///
/// Returns `Promise<Blob>`. Rejects with `IndexSizeError` if the OC has no
/// bitmap yet (pre-`getContext`) or with `EncodingError` if the encoder fails.
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

    // Snapshot the encoded bytes synchronously. The OC entity may not yet have
    // a `Canvas2dContext` (no `getContext` called yet); per spec, an
    // unrenderable bitmap rejects with `InvalidStateError` on the next
    // microtask (matching the async contract of the returned Promise).
    //
    // Borrow split: `with_context` needs `&mut EcsDom`, which means a
    // `&mut HostData` (the only owner of the DOM). Reach it through the
    // rooted `&mut VmInner` guard via the standard `host_data.as_deref_mut`
    // path (mirrors `vm_api.rs::unbind` and blob.rs accessors).
    let encoded = g
        .host_data
        .as_deref_mut()
        .and_then(|hd| with_context(hd.dom(), entity, |c| c.encode_blob(format, quality)))
        .flatten();

    if let Some(bytes) = encoded {
        // Materialize a Blob whose `type` matches the negotiated format
        // (post-fallback per `from_mime`).
        let type_sid = g.strings.intern(format.mime());
        let blob_id = super::super::blob::create_blob_from_bytes(&mut g, bytes.into(), type_sid);
        super::super::blob::resolve_promise_sync(&mut g, promise, JsValue::Object(blob_id));
    } else {
        let exc = g.well_known.dom_exc_invalid_state_error;
        let reason = g.build_dom_exception(
            exc,
            "Failed to execute 'convertToBlob' on 'OffscreenCanvas': no bitmap or encoding failed.",
        );
        super::super::blob::reject_promise_sync(&mut g, promise, reason);
    }
    drop(g);
    Ok(JsValue::Object(promise))
}

/// Parse the optional `{ type, quality }` options dictionary. `type` is
/// ASCII-lowercased then mapped via [`BlobImageFormat::from_mime`] (unknown →
/// PNG fallback per spec). `quality` is `unrestricted double` clamped to
/// `[0.0, 1.0]` here so encoders downstream receive a sanitized value
/// (non-finite / NaN → defaults to 1.0 per spec wording "if quality is not in
/// the closed interval [0.0, 1.0], let it be 1.0"). Returns the negotiated
/// format + quality.
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
            let lower = raw.to_ascii_lowercase();
            BlobImageFormat::from_mime(&lower)
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
