//! `atob`/`btoa`, `crypto`, and `queueMicrotask` registrations.

use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};

use boa_engine::object::ObjectInitializer;

/// Register `atob()` and `btoa()` (WHATWG HTML §8.3).
pub(super) fn register_atob_btoa(ctx: &mut Context) {
    use base64::Engine;

    // btoa(str) — Latin1 -> Base64.
    let btoa_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let input = args
            .first()
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map_or(String::new(), |s| s.to_std_string_escaped());

        // Check for non-Latin1 characters (> U+00FF).
        if input.chars().any(|c| c as u32 > 0xFF) {
            return Err(boa_engine::JsNativeError::eval()
                .with_message("InvalidCharacterError: btoa: string contains non-Latin1 character")
                .into());
        }

        let bytes: Vec<u8> = input.chars().map(|c| c as u8).collect();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(JsValue::from(js_string!(encoded.as_str())))
    });
    ctx.register_global_builtin_callable(js_string!("btoa"), 1, btoa_fn)
        .expect("failed to register btoa");

    // atob(str) — Base64 -> Latin1.
    let atob_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let input = args
            .first()
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map_or(String::new(), |s| s.to_std_string_escaped());

        // Strip ASCII whitespace (WHATWG HTML §8.3).
        let stripped: String = input
            .chars()
            .filter(|c| !matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' '))
            .collect();

        // Forgiving decode — accept missing padding.
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::GeneralPurposeConfig::new()
                .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
        );
        let bytes = engine.decode(&stripped).map_err(|_| {
            boa_engine::JsNativeError::eval()
                .with_message("InvalidCharacterError: atob: invalid base64 input")
        })?;

        // Convert bytes to Latin1 string.
        let result: String = bytes.iter().map(|&b| b as char).collect();
        Ok(JsValue::from(js_string!(result.as_str())))
    });
    ctx.register_global_builtin_callable(js_string!("atob"), 1, atob_fn)
        .expect("failed to register atob");
}

/// Register `crypto` object (W3C `WebCrypto`).
pub(super) fn register_crypto(ctx: &mut Context) {
    let mut init = ObjectInitializer::new(ctx);

    // crypto.getRandomValues(typedArray) — fill with random bytes.
    init.function(
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let arr = args.first().and_then(JsValue::as_object).ok_or_else(|| {
                boa_engine::JsNativeError::typ()
                    .with_message("crypto.getRandomValues: argument must be a typed array")
            })?;
            let len_val = arr.get(js_string!("length"), ctx)?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let len = len_val.to_number(ctx)? as usize;

            // W3C WebCrypto §10.1.1: max 65536 bytes.
            if len > 65536 {
                return Err(boa_engine::JsNativeError::eval()
                    .with_message("QuotaExceededError: getRandomValues: array too large")
                    .into());
            }

            let mut bytes = vec![0u8; len];
            getrandom::fill(&mut bytes).map_err(|_| {
                boa_engine::JsNativeError::eval()
                    .with_message("crypto.getRandomValues: random generation failed")
            })?;

            for (i, &b) in bytes.iter().enumerate() {
                arr.set(i as u32, JsValue::from(f64::from(b)), false, ctx)?;
            }

            Ok(args.first().cloned().unwrap_or(JsValue::undefined()))
        }),
        js_string!("getRandomValues"),
        1,
    );

    // crypto.randomUUID() — UUID v4.
    init.function(
        NativeFunction::from_copy_closure(|_this, _args, _ctx| {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).map_err(|_| {
                boa_engine::JsNativeError::eval()
                    .with_message("crypto.randomUUID: random generation failed")
            })?;
            // Set version (4) and variant (RFC 4122).
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let uuid = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                bytes[0], bytes[1], bytes[2], bytes[3],
                bytes[4], bytes[5],
                bytes[6], bytes[7],
                bytes[8], bytes[9],
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            );
            Ok(JsValue::from(js_string!(uuid.as_str())))
        }),
        js_string!("randomUUID"),
        0,
    );

    let crypto = init.build();
    ctx.register_global_property(js_string!("crypto"), crypto, Attribute::all())
        .expect("failed to register crypto");
}

/// Register `queueMicrotask()` (WHATWG HTML §8.6).
pub(super) fn register_queue_microtask(ctx: &mut Context) {
    ctx.register_global_builtin_callable(
        js_string!("queueMicrotask"),
        1,
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let callback = args.first().and_then(JsValue::as_callable).ok_or_else(|| {
                boa_engine::JsNativeError::typ()
                    .with_message("queueMicrotask: argument must be a function")
            })?;
            // Queue the callback as a microtask via Promise.resolve().then().
            // boa's run_jobs() drains microtasks after eval completes, giving
            // correct WHATWG microtask timing.
            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
            // The callback is already verified callable, so from_object always succeeds.
            if let Some(cb_fn) =
                boa_engine::object::builtins::JsFunction::from_object(callback.clone())
            {
                let _ = promise.then(Some(cb_fn), None, ctx);
            }
            Ok(JsValue::undefined())
        }),
    )
    .expect("failed to register queueMicrotask");
}
