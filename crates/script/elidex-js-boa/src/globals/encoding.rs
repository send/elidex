//! `TextEncoder` and `TextDecoder` (WHATWG Encoding §8).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Register `TextEncoder` and `TextDecoder` global constructors.
pub fn register_encoding(ctx: &mut Context, _bridge: &HostBridge) {
    // TextEncoder: new TextEncoder()
    ctx.register_global_builtin_callable(
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

                    let written = if let Some(dest_obj) = dest {
                        let len_val = dest_obj.get(js_string!("length"), ctx)?;
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let len = len_val.to_number(ctx)? as usize;
                        let write_count = bytes.len().min(len);
                        for (i, &b) in bytes[..write_count].iter().enumerate() {
                            dest_obj.set(i as u32, JsValue::from(f64::from(b)), false, ctx)?;
                        }
                        write_count
                    } else {
                        0
                    };

                    let mut result = ObjectInitializer::new(ctx);
                    #[allow(clippy::cast_precision_loss)]
                    {
                        result.property(
                            js_string!("read"),
                            JsValue::from(source.chars().count() as f64),
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
    ctx.register_global_builtin_callable(
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

            let fatal = args
                .get(1)
                .and_then(JsValue::as_object)
                .map(|obj| obj.get(js_string!("fatal"), ctx))
                .transpose()?
                .is_some_and(|v| v.to_boolean());

            let mut init = ObjectInitializer::new(ctx);

            init.property(
                js_string!("encoding"),
                JsValue::from(js_string!(label.as_str())),
                Attribute::READONLY,
            );
            init.property(
                js_string!("fatal"),
                JsValue::from(fatal),
                Attribute::READONLY,
            );

            // decode(input) → string
            let fatal_copy = fatal;
            init.function(
                NativeFunction::from_copy_closure_with_captures(
                    |_this, args, fatal, ctx| {
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

                        // UTF-8 decode (default).
                        match std::str::from_utf8(&bytes) {
                            Ok(s) => Ok(JsValue::from(js_string!(s))),
                            Err(_) if *fatal => Err(JsNativeError::typ()
                                .with_message("TextDecoder: decoding failed (fatal mode)")
                                .into()),
                            Err(_) => {
                                // Lossy decode.
                                let s = String::from_utf8_lossy(&bytes);
                                Ok(JsValue::from(js_string!(s.as_ref())))
                            }
                        }
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
