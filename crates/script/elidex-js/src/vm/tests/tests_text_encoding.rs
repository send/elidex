//! `TextEncoder` / `TextDecoder` tests (WHATWG Encoding §8).
//!
//! Covers constructors (label normalisation, options dictionary),
//! IDL accessors (`encoding` / `fatal` / `ignoreBOM`), `encode` /
//! `encodeInto`, streaming `decode`, BOM handling, fatal mode, and
//! structuredClone unclonability.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// TextEncoder
// ---------------------------------------------------------------------------

#[test]
fn text_encoder_encoding_is_utf8() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new TextEncoder().encoding;"), "utf-8");
}

#[test]
fn text_encoder_requires_new() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; try { TextEncoder(); } catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn text_encoder_encode_ascii() {
    let mut vm = Vm::new();
    // "hi" → [0x68, 0x69]
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('hi').length;"),
        2.0
    );
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('hi')[0];"),
        f64::from(0x68_u8)
    );
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('hi')[1];"),
        f64::from(0x69_u8)
    );
}

#[test]
fn text_encoder_encode_returns_uint8array() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new TextEncoder().encode('x') instanceof Uint8Array;"
    ));
}

#[test]
fn text_encoder_encode_undefined_input() {
    let mut vm = Vm::new();
    // Spec: input defaults to empty string → zero-length Uint8Array.
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode().length;"),
        0.0
    );
}

#[test]
fn text_encoder_encode_multibyte_utf8() {
    let mut vm = Vm::new();
    // "€" = U+20AC = UTF-8 0xE2 0x82 0xAC (3 bytes).
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('€').length;"),
        3.0
    );
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('€')[0];"),
        f64::from(0xE2_u8)
    );
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('€')[1];"),
        f64::from(0x82_u8)
    );
    assert_eq!(
        eval_number(&mut vm, "new TextEncoder().encode('€')[2];"),
        f64::from(0xAC_u8)
    );
}

#[test]
fn text_encoder_encode_into_written_bytes() {
    let mut vm = Vm::new();
    // "hi" fits entirely in 8-byte destination; read=2, written=2.
    assert_eq!(
        eval_number(
            &mut vm,
            "var enc = new TextEncoder(); \
             var dest = new Uint8Array(8); \
             enc.encodeInto('hi', dest).written;"
        ),
        2.0
    );
}

#[test]
fn text_encoder_encode_into_partial_fill() {
    let mut vm = Vm::new();
    // Destination too small for "€" (3 bytes) in a 2-byte buffer —
    // spec: skip the overflowing char, return read=0, written=0.
    assert_eq!(
        eval_number(
            &mut vm,
            "var enc = new TextEncoder(); \
             var dest = new Uint8Array(2); \
             enc.encodeInto('€', dest).written;"
        ),
        0.0
    );
}

#[test]
fn text_encoder_encode_into_reads_utf16_units() {
    let mut vm = Vm::new();
    // "a€" in UTF-16 is 2 code units ("a" + "€").  UTF-8 bytes: 1
    // + 3 = 4.  Destination fits exactly — read=2, written=4.
    assert_eq!(
        eval_number(
            &mut vm,
            "var enc = new TextEncoder(); \
             var dest = new Uint8Array(4); \
             enc.encodeInto('a€', dest).read;"
        ),
        2.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var enc = new TextEncoder(); \
             var dest = new Uint8Array(4); \
             enc.encodeInto('a€', dest).written;"
        ),
        4.0
    );
}

#[test]
fn text_encoder_encode_into_requires_uint8array() {
    let mut vm = Vm::new();
    // A plain Array is not a Uint8Array — TypeError per §8.2.3.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new TextEncoder().encodeInto('x', [0,0,0]); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn text_encoder_brand_check() {
    let mut vm = Vm::new();
    // `{encode: TextEncoder.prototype.encode}.encode()` must throw.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { ({encode: TextEncoder.prototype.encode}).encode('x'); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

// ---------------------------------------------------------------------------
// TextDecoder
// ---------------------------------------------------------------------------

#[test]
fn text_decoder_default_is_utf8() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new TextDecoder().encoding;"), "utf-8");
    assert!(!eval_bool(&mut vm, "new TextDecoder().fatal;"));
    assert!(!eval_bool(&mut vm, "new TextDecoder().ignoreBOM;"));
}

#[test]
fn text_decoder_label_normalises_case_and_whitespace() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new TextDecoder(' UTF-8 ').encoding;"),
        "utf-8"
    );
    assert_eq!(
        eval_string(&mut vm, "new TextDecoder('utf8').encoding;"),
        "utf-8"
    );
    assert_eq!(
        eval_string(&mut vm, "new TextDecoder('UTF-16LE').encoding;"),
        "utf-16le"
    );
    assert_eq!(
        eval_string(&mut vm, "new TextDecoder('UTF-16BE').encoding;"),
        "utf-16be"
    );
}

#[test]
fn text_decoder_invalid_label_throws_range_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; try { new TextDecoder('not-a-real-encoding'); } \
         catch (e) { r = e instanceof RangeError; } r;"
    ));
}

#[test]
fn text_decoder_options_flags() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new TextDecoder('utf-8', {fatal: true}).fatal;"
    ));
    assert!(eval_bool(
        &mut vm,
        "new TextDecoder('utf-8', {ignoreBOM: true}).ignoreBOM;"
    ));
}

#[test]
fn text_decoder_requires_new() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; try { TextDecoder(); } catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn text_decoder_decode_undefined_input_returns_empty() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new TextDecoder().decode();"), "");
}

#[test]
fn text_decoder_decode_uint8array_roundtrip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var enc = new TextEncoder(); \
             var dec = new TextDecoder(); \
             dec.decode(enc.encode('hello world'));"
        ),
        "hello world"
    );
}

#[test]
fn text_decoder_decode_multibyte_roundtrip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var enc = new TextEncoder(); var dec = new TextDecoder(); dec.decode(enc.encode('€100'));"
        ),
        "€100"
    );
}

#[test]
fn text_decoder_decode_array_buffer() {
    let mut vm = Vm::new();
    // Decode from a raw ArrayBuffer (no view).
    assert_eq!(
        eval_string(
            &mut vm,
            "var buf = new ArrayBuffer(2); \
             var v = new Uint8Array(buf); v[0] = 0x68; v[1] = 0x69; \
             new TextDecoder().decode(buf);"
        ),
        "hi"
    );
}

#[test]
fn text_decoder_decode_data_view() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var buf = new ArrayBuffer(3); \
             var v = new Uint8Array(buf); v[0] = 0x61; v[1] = 0x62; v[2] = 0x63; \
             new TextDecoder().decode(new DataView(buf));"
        ),
        "abc"
    );
}

#[test]
fn text_decoder_fatal_throws_on_invalid_utf8() {
    let mut vm = Vm::new();
    // 0xFF is not a valid UTF-8 leading byte.
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { \
             var dec = new TextDecoder('utf-8', {fatal: true}); \
             var bad = new Uint8Array([0xFF]); \
             dec.decode(bad); \
         } catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn text_decoder_non_fatal_uses_replacement() {
    let mut vm = Vm::new();
    // 0xFF → U+FFFD "REPLACEMENT CHARACTER".
    assert_eq!(
        eval_string(&mut vm, "new TextDecoder().decode(new Uint8Array([0xFF]));"),
        "\u{FFFD}"
    );
}

#[test]
fn text_decoder_utf8_bom_stripped_by_default() {
    let mut vm = Vm::new();
    // BOM (0xEF 0xBB 0xBF) + 'x'.  Default ignoreBOM=false: BOM removed.
    assert_eq!(
        eval_string(
            &mut vm,
            "new TextDecoder().decode(new Uint8Array([0xEF, 0xBB, 0xBF, 0x78]));"
        ),
        "x"
    );
}

#[test]
fn text_decoder_utf8_bom_preserved_with_ignore_bom() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new TextDecoder('utf-8', {ignoreBOM: true}).decode(new Uint8Array([0xEF, 0xBB, 0xBF, 0x78]));"
        ),
        "\u{FEFF}x"
    );
}

#[test]
fn text_decoder_stream_preserves_partial_multibyte() {
    let mut vm = Vm::new();
    // UTF-8 'é' = 0xC3 0xA9 split across two chunks.  With
    // stream:true, the first chunk returns "" (partial); the
    // second completes the code point.
    assert_eq!(
        eval_string(
            &mut vm,
            "var dec = new TextDecoder(); \
             var a = dec.decode(new Uint8Array([0xC3]), {stream: true}); \
             var b = dec.decode(new Uint8Array([0xA9])); \
             a + b;"
        ),
        "é"
    );
}

#[test]
fn text_decoder_non_stream_flushes_incomplete_sequence() {
    let mut vm = Vm::new();
    // Incomplete multi-byte sequence without stream flag: flush
    // yields replacement character.
    assert_eq!(
        eval_string(&mut vm, "new TextDecoder().decode(new Uint8Array([0xC3]));"),
        "\u{FFFD}"
    );
}

#[test]
fn text_decoder_reset_after_non_stream_call() {
    let mut vm = Vm::new();
    // After a non-stream decode, the decoder is reset — a
    // subsequent decode of a complete sequence must not inherit
    // any residual state.
    assert_eq!(
        eval_string(
            &mut vm,
            "var dec = new TextDecoder(); \
             dec.decode(new Uint8Array([0xC3]));  /* terminal, flushes FFFD */ \
             dec.decode(new Uint8Array([0x78]));  /* fresh 'x' */"
        ),
        "x"
    );
}

#[test]
fn text_decoder_decode_rejects_non_buffer_source() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { new TextDecoder().decode([1, 2, 3]); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

#[test]
fn text_decoder_brand_check() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { ({decode: TextDecoder.prototype.decode}).decode(); } \
         catch (e) { r = e instanceof TypeError; } r;"
    ));
}

// ---------------------------------------------------------------------------
// structuredClone unclonability
// ---------------------------------------------------------------------------

#[test]
fn structured_clone_text_encoder_throws_data_clone_error() {
    let mut vm = Vm::new();
    // Spec §2.9: TextEncoder is not listed as serializable → throws
    // DOMException("DataCloneError").
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { structuredClone(new TextEncoder()); } \
         catch (e) { r = e.name === 'DataCloneError'; } r;"
    ));
}

#[test]
fn structured_clone_text_decoder_throws_data_clone_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = false; \
         try { structuredClone(new TextDecoder()); } \
         catch (e) { r = e.name === 'DataCloneError'; } r;"
    ));
}
