//! `Blob` and `File` APIs (WHATWG File API §4-§5).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

/// Hidden property key for blob data (stored as a JsValue string of comma-separated bytes).
const BLOB_DATA_KEY: &str = "__elidex_blob_data__";
/// Hidden property key marking an object as a Blob.
const BLOB_MARKER: &str = "__elidex_blob__";
/// Hidden property key marking an object as a File.
const FILE_MARKER: &str = "__elidex_file__";

/// Register `Blob` and `File` constructors on the global object.
pub fn register_blob_file(ctx: &mut Context) {
    register_blob_constructor(ctx);
    register_file_constructor(ctx);
}

/// Collect bytes from blobParts (WHATWG File API §4.1).
///
/// Supports: strings (→ UTF-8), array-like of numbers (→ bytes), other Blob objects.
fn collect_blob_parts(parts: &JsValue, ctx: &mut Context) -> JsResult<Vec<u8>> {
    let mut result = Vec::new();

    let obj = match parts.as_object() {
        Some(o) => o,
        None => return Ok(result),
    };

    let len = obj
        .get(js_string!("length"), ctx)?
        .to_number(ctx)
        .unwrap_or(0.0) as u32;

    for i in 0..len {
        let part = obj.get(i, ctx)?;
        if let Some(s) = part.as_string() {
            result.extend_from_slice(s.to_std_string_escaped().as_bytes());
        } else if let Some(part_obj) = part.as_object() {
            // Check if it's a Blob (has our marker).
            let is_blob = part_obj
                .get(js_string!(BLOB_MARKER), ctx)?
                .to_boolean();
            if is_blob {
                let data = extract_blob_bytes(&part_obj, ctx)?;
                result.extend_from_slice(&data);
            } else {
                // Treat as array-like of numbers.
                let part_len = part_obj
                    .get(js_string!("length"), ctx)?
                    .to_number(ctx)
                    .unwrap_or(0.0) as u32;
                for j in 0..part_len {
                    let byte = part_obj.get(j, ctx)?.to_number(ctx).unwrap_or(0.0);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    result.push(byte as u8);
                }
            }
        } else {
            // Convert to string.
            let s = part.to_string(ctx)?.to_std_string_escaped();
            result.extend_from_slice(s.as_bytes());
        }
    }

    Ok(result)
}

/// Extract raw bytes from a Blob JS object.
///
/// Reads from a `JsArrayBuffer` stored in the hidden `__elidex_blob_data__` property.
fn extract_blob_bytes(obj: &boa_engine::JsObject, ctx: &mut Context) -> JsResult<Vec<u8>> {
    let data_val = obj.get(js_string!(BLOB_DATA_KEY), ctx)?;
    let data_obj = data_val.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("Blob: internal data missing")
    })?;
    let ab = boa_engine::object::builtins::JsArrayBuffer::from_object(data_obj.clone())
        .map_err(|_| JsNativeError::typ().with_message("Blob: internal data is not an ArrayBuffer"))?;
    let data_ref = ab.data().ok_or_else(|| {
        JsNativeError::typ().with_message("Blob: ArrayBuffer is detached")
    })?;
    Ok(data_ref.to_vec())
}

/// Register `Blob` constructor.
fn register_blob_constructor(ctx: &mut Context) {
    let constructor = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parts = args.first().cloned().unwrap_or(JsValue::undefined());
        let options = args.get(1);

        let bytes = if parts.is_undefined() || parts.is_null() {
            Vec::new()
        } else {
            collect_blob_parts(&parts, ctx)?
        };

        // Extract type from options.
        let blob_type = options
            .and_then(|o| o.as_object())
            .map(|o| {
                o.get(js_string!("type"), ctx)
                    .ok()
                    .and_then(|v| {
                        if v.is_undefined() {
                            None
                        } else {
                            Some(v.to_string(ctx).ok()?.to_std_string_escaped().to_ascii_lowercase())
                        }
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        build_blob_from_bytes(&bytes, &blob_type, ctx)
    });

    ctx.register_global_callable(js_string!("Blob"), 0, constructor)
        .expect("failed to register Blob");
}

/// Build a new Blob JS object from raw bytes and content type.
///
/// Creates the Blob with marker, data (JsArrayBuffer), size, type,
/// and all standard methods (text, arrayBuffer, slice).
fn build_blob_from_bytes(bytes: &[u8], content_type: &str, ctx: &mut Context) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);
    init.property(js_string!(BLOB_MARKER), JsValue::from(true), Attribute::empty());

    // Store bytes as JsArrayBuffer internally.
    let aligned = boa_engine::object::builtins::AlignedVec::from_iter(0, bytes.iter().copied());
    let ab = boa_engine::object::builtins::JsArrayBuffer::from_byte_block(aligned, init.context())?;
    init.property(
        js_string!(BLOB_DATA_KEY),
        JsValue::from(ab),
        Attribute::empty(),
    );

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!("size"),
        JsValue::from(bytes.len() as f64),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );
    init.property(
        js_string!("type"),
        JsValue::from(js_string!(content_type)),
        Attribute::READONLY | Attribute::CONFIGURABLE,
    );

    add_blob_methods(&mut init);

    Ok(JsValue::from(init.build()))
}

/// Add `text()`, `arrayBuffer()`, and `slice()` methods to a Blob ObjectInitializer.
fn add_blob_methods(init: &mut ObjectInitializer<'_>) {
    // text() → Promise<string> (resolves synchronously via microtask).
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Blob.text: this is not a Blob")
            })?;
            let data = extract_blob_bytes(&obj, ctx)?;
            let text = String::from_utf8_lossy(&data).into_owned();
            let promise = boa_engine::object::builtins::JsPromise::resolve(
                JsValue::from(js_string!(text.as_str())),
                ctx,
            );
            Ok(promise.into())
        }),
        js_string!("text"),
        0,
    );

    // arrayBuffer() → Promise<ArrayBuffer>.
    init.function(
        NativeFunction::from_copy_closure(|this, _args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Blob.arrayBuffer: this is not a Blob")
            })?;
            let data = extract_blob_bytes(&obj, ctx)?;
            let aligned =
                boa_engine::object::builtins::AlignedVec::from_iter(0, data.into_iter());
            let array_buffer =
                boa_engine::object::builtins::JsArrayBuffer::from_byte_block(aligned, ctx)?;
            let promise = boa_engine::object::builtins::JsPromise::resolve(
                JsValue::from(array_buffer),
                ctx,
            );
            Ok(promise.into())
        }),
        js_string!("arrayBuffer"),
        0,
    );

    // slice(start?, end?, contentType?) — WHATWG File API §4.2.1.
    init.function(
        NativeFunction::from_copy_closure(|this, args, ctx| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("Blob.slice: this is not a Blob")
            })?;

            let data = extract_blob_bytes(&obj, ctx)?;
            let size = data.len() as i64;

            // Resolve start/end with negative index support (relative to size).
            let raw_start = args.first().and_then(JsValue::as_number).unwrap_or(0.0) as i64;
            let raw_end = args
                .get(1)
                .and_then(JsValue::as_number)
                .unwrap_or(size as f64) as i64;

            let start = if raw_start < 0 {
                (size + raw_start).max(0) as usize
            } else {
                raw_start.min(size) as usize
            };
            let end = if raw_end < 0 {
                (size + raw_end).max(0) as usize
            } else {
                raw_end.min(size) as usize
            };

            let slice_bytes = if start < end {
                &data[start..end]
            } else {
                &[]
            };

            // contentType — ASCII lowercase (File API §4.2.1).
            let content_type = args
                .get(2)
                .and_then(|v| {
                    if v.is_undefined() || v.is_null() {
                        None
                    } else {
                        Some(v.to_string(ctx).ok()?.to_std_string_escaped().to_ascii_lowercase())
                    }
                })
                .unwrap_or_default();

            build_blob_from_bytes(slice_bytes, &content_type, ctx)
        }),
        js_string!("slice"),
        0,
    );
}

/// Register `File` constructor (WHATWG File API §5).
fn register_file_constructor(ctx: &mut Context) {
    let constructor = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let bits = args.first().cloned().unwrap_or(JsValue::undefined());
        let name = crate::globals::require_js_string_arg(args, 1, "File", ctx)?;
        let options = args.get(2);

        let bytes = if bits.is_undefined() || bits.is_null() {
            Vec::new()
        } else {
            collect_blob_parts(&bits, ctx)?
        };

        let file_type = options
            .and_then(|o| o.as_object())
            .map(|o| {
                o.get(js_string!("type"), ctx)
                    .ok()
                    .and_then(|v| {
                        if v.is_undefined() {
                            None
                        } else {
                            Some(v.to_string(ctx).ok()?.to_std_string_escaped().to_ascii_lowercase())
                        }
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let last_modified = options
            .and_then(|o| o.as_object())
            .and_then(|o| {
                o.get(js_string!("lastModified"), ctx)
                    .ok()
                    .and_then(|v| v.as_number())
            })
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0.0, |d| d.as_millis() as f64)
            });

        let size = bytes.len();

        let mut init = ObjectInitializer::new(ctx);
        init.property(js_string!(BLOB_MARKER), JsValue::from(true), Attribute::empty());
        init.property(js_string!(FILE_MARKER), JsValue::from(true), Attribute::empty());

        // Store bytes as JsArrayBuffer internally.
        let aligned = boa_engine::object::builtins::AlignedVec::from_iter(0, bytes.iter().copied());
        let ab = boa_engine::object::builtins::JsArrayBuffer::from_byte_block(aligned, init.context())?;
        init.property(
            js_string!(BLOB_DATA_KEY),
            JsValue::from(ab),
            Attribute::empty(),
        );

        #[allow(clippy::cast_precision_loss)]
        init.property(
            js_string!("size"),
            JsValue::from(size as f64),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );
        init.property(
            js_string!("type"),
            JsValue::from(js_string!(file_type.as_str())),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );
        init.property(
            js_string!("name"),
            JsValue::from(js_string!(name.as_str())),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );
        init.property(
            js_string!("lastModified"),
            JsValue::from(last_modified),
            Attribute::READONLY | Attribute::CONFIGURABLE,
        );

        add_blob_methods(&mut init);

        Ok(JsValue::from(init.build()))
    });

    ctx.register_global_callable(js_string!("File"), 2, constructor)
        .expect("failed to register File");
}

/// Check if a JS value is a Blob object.
#[allow(dead_code)]
pub(crate) fn is_blob(val: &JsValue, ctx: &mut Context) -> bool {
    val.as_object().is_some_and(|obj| {
        obj.get(js_string!(BLOB_MARKER), ctx)
            .ok()
            .is_some_and(|v| v.to_boolean())
    })
}
