//! `TextEncoder` and `TextDecoder` (WHATWG Encoding §8).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `TextEncoder` and `TextDecoder` global constructors.
pub fn register_encoding(ctx: &mut Context, _bridge: &HostBridge) {
    // TextEncoder: new TextEncoder()
    ctx.register_global_callable(
        js_string!("TextEncoder"),
        0,
        NativeFunction::from_copy_closure(|_this, _args, ctx| {
            let mut init = ObjectInitializer::new(ctx);

            init.property(
                js_string!("encoding"),
                JsValue::from(js_string!("utf-8")),
                Attribute::READONLY,
            );

            // encode(string) → Array<number> (Uint8Array requires boa TypedArray setup).
            init.function(
                NativeFunction::from_copy_closure(|_this, args, ctx| {
                    let input = args
                        .first()
                        .map(|v| v.to_string(ctx))
                        .transpose()?
                        .map_or(String::new(), |s| s.to_std_string_escaped());
                    let bytes = input.as_bytes();
                    let arr = boa_engine::object::builtins::JsArray::new(ctx);
                    for &b in bytes {
                        let _ = arr.push(JsValue::from(f64::from(b)), ctx);
                    }
                    Ok(arr.into())
                }),
                js_string!("encode"),
                1,
            );

            // encodeInto(source, destination) → { read, written }
            init.function(
                NativeFunction::from_copy_closure(|_this, args, ctx| {
                    let source = args
                        .first()
                        .map(|v| v.to_string(ctx))
                        .transpose()?
                        .map_or(String::new(), |s| s.to_std_string_escaped());
                    let dest = args.get(1).and_then(JsValue::as_object);
                    let bytes = source.as_bytes();

                    let dest_len = if let Some(dest_obj) = &dest {
                        let len_val = dest_obj.get(js_string!("length"), ctx)?;
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let len = len_val.to_number(ctx)? as usize;
                        len
                    } else {
                        0
                    };

                    // Count characters consumed corresponding to bytes that fit
                    // in the destination buffer (UTF-8 chars may be multi-byte).
                    let mut read = 0usize;
                    let mut written = 0usize;
                    for (_i, ch) in source.char_indices() {
                        let ch_len = ch.len_utf8();
                        if written + ch_len > dest_len {
                            break;
                        }
                        written += ch_len;
                        read += 1;
                    }

                    if let Some(dest_obj) = &dest {
                        for (i, &b) in bytes[..written].iter().enumerate() {
                            dest_obj.set(i as u32, JsValue::from(f64::from(b)), false, ctx)?;
                        }
                    }

                    let mut result = ObjectInitializer::new(ctx);
                    #[allow(clippy::cast_precision_loss)]
                    {
                        result.property(
                            js_string!("read"),
                            JsValue::from(read as f64),
                            Attribute::all(),
                        );
                        result.property(
                            js_string!("written"),
                            JsValue::from(written as f64),
                            Attribute::all(),
                        );
                    }
                    Ok(result.build().into())
                }),
                js_string!("encodeInto"),
                2,
            );

            Ok(init.build().into())
        }),
    )
    .expect("failed to register TextEncoder");

    // TextDecoder: new TextDecoder(label?, options?)
    ctx.register_global_callable(
        js_string!("TextDecoder"),
        0,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let label = args
                .first()
                .filter(|v| !v.is_undefined())
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|| "utf-8".to_string());

            // Resolve the label to an encoding via encoding_rs.
            let encoding = encoding_rs::Encoding::for_label(label.as_bytes()).ok_or_else(|| {
                JsNativeError::range()
                    .with_message(format!("TextDecoder: unsupported encoding: {label}"))
            })?;
            let encoding_name = encoding.name().to_string();

            let fatal = args
                .get(1)
                .and_then(JsValue::as_object)
                .map(|obj| obj.get(js_string!("fatal"), ctx))
                .transpose()?
                .is_some_and(|v| v.to_boolean());

            let mut init = ObjectInitializer::new(ctx);

            init.property(
                js_string!("encoding"),
                JsValue::from(js_string!(encoding_name.to_ascii_lowercase().as_str())),
                Attribute::READONLY,
            );
            init.property(
                js_string!("fatal"),
                JsValue::from(fatal),
                Attribute::READONLY,
            );

            // Store the encoding name in a hidden property for decode() to use.
            init.property(
                js_string!("__encoding_name__"),
                JsValue::from(js_string!(encoding_name.as_str())),
                Attribute::empty(),
            );

            // decode(input) → string
            let fatal_copy = fatal;
            init.function(
                NativeFunction::from_copy_closure_with_captures(
                    |this, args, fatal, ctx| {
                        let input = args.first().and_then(JsValue::as_object);
                        let bytes: Vec<u8> = if let Some(obj) = input {
                            let len_val = obj.get(js_string!("length"), ctx)?;
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let len = len_val.to_number(ctx)? as usize;
                            let mut buf = Vec::with_capacity(len);
                            for i in 0..len {
                                let val = obj.get(i as u32, ctx)?;
                                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                buf.push(val.to_number(ctx)? as u8);
                            }
                            buf
                        } else {
                            Vec::new()
                        };

                        // Look up the encoding from the hidden property.
                        let enc_name = this
                            .as_object()
                            .and_then(|obj| {
                                obj.get(js_string!("__encoding_name__"), ctx)
                                    .ok()
                                    .map(|v| v.to_string(ctx))
                            })
                            .transpose()?
                            .map(|s| s.to_std_string_escaped())
                            .unwrap_or_else(|| "UTF-8".to_string());
                        let encoding = encoding_rs::Encoding::for_label(enc_name.as_bytes())
                            .unwrap_or(encoding_rs::UTF_8);

                        let (decoded, _enc, had_errors) = encoding.decode(&bytes);
                        if had_errors && *fatal {
                            return Err(JsNativeError::typ()
                                .with_message("TextDecoder: decoding failed (fatal mode)")
                                .into());
                        }
                        Ok(JsValue::from(js_string!(decoded.as_ref())))
                    },
                    fatal_copy,
                ),
                js_string!("decode"),
                1,
            );

            Ok(init.build().into())
        }),
    )
    .expect("failed to register TextDecoder");
}
