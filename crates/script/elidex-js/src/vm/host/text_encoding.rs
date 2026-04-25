//! `TextEncoder` and `TextDecoder` (WHATWG Encoding §8).
//!
//! Both interfaces are rooted at `Object` — not `EventTarget` —
//! so their prototype chains terminate at `Object.prototype`:
//!
//! ```text
//! TextEncoder instance (ObjectKind::TextEncoder, payload-free)
//!   → TextEncoder.prototype  (this module)
//!     → Object.prototype
//!
//! TextDecoder instance (ObjectKind::TextDecoder, payload-free)
//!   → TextDecoder.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## Scope
//!
//! - `TextEncoder()` — no args, always UTF-8.
//!   - `.encoding` → `"utf-8"` (IDL readonly).
//!   - `.encode(input?)` → fresh `Uint8Array` over a fresh
//!     `ArrayBuffer` of the UTF-8 encoded bytes.
//!   - `.encodeInto(source, destination)` → `{read, written}`
//!     plain object.  Writes UTF-8 bytes directly into the
//!     destination `Uint8Array`'s backing buffer up to capacity.
//! - `TextDecoder(label?, options?)` — WHATWG §10.1.2.
//!   - `label` defaults to `"utf-8"`; resolved via
//!     `encoding_rs::Encoding::for_label` (case-insensitive, trims
//!     ASCII whitespace).  Unsupported labels throw `RangeError`.
//!   - `options.fatal` / `.ignoreBOM` — boolean bag keys, both
//!     default `false`.
//!   - `.encoding` / `.fatal` / `.ignoreBOM` IDL readonly getters.
//!   - `.decode(input?, options?)` — accepts any BufferSource
//!     (`ArrayBuffer` / `TypedArray` / `DataView`).  `options.stream`
//!     (default `false`) keeps partial sequences buffered in the
//!     decoder state across calls.
//!
//! ## State model
//!
//! `TextEncoder` is stateless — the variant exists purely for the
//! brand check so `{encode: TextEncoder.prototype.encode}.encode()`
//! throws TypeError per WebIDL §3.10.
//!
//! `TextDecoder` owns a live [`encoding_rs::Decoder`] — that type
//! manages both the BOM-sniffing state (if enabled) and partial
//! multi-byte sequence state across streaming calls, so we do not
//! need to maintain our own leftover-byte buffer.  The decoder is
//! rebuilt on each non-stream `decode()` call so the next call
//! starts fresh.
//!
//! ## Encoding coverage
//!
//! Labels beyond `utf-8` / `utf-16le` / `utf-16be` resolve via
//! `encoding_rs::Encoding::for_label` and decode correctly; only
//! the three above are pre-interned in `WellKnownStrings` for
//! accessor fast-pathing.  Canonical names for other encodings
//! are interned per-call (lowercase) when the `encoding` getter
//! fires.
//!
//! **Intentionally rejected**: the WHATWG "replacement" encoding
//! is filtered out in `resolve_encoding_label` per Encoding
//! §10.2.1 step 2 ("if encoding is failure **or replacement**,
//! then throw a RangeError") — labels like `iso-2022-kr` /
//! `hz-gb-2312` / `iso-2022-cn` that map to "replacement" are a
//! cross-site scripting defence surface and TextDecoder is
//! explicitly barred from exposing them.

#![cfg(feature = "engine")]

use encoding_rs::{DecoderResult, Encoding};

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Per-`TextDecoder` out-of-band state, keyed in
/// [`super::super::VmInner::text_decoder_states`] by the instance's
/// `ObjectId`.
///
/// `decoder` carries the resolved encoding (via
/// [`encoding_rs::Decoder::encoding`]) so no separate slot is
/// needed.  It is rebuilt per non-stream `decode()` call to match
/// the spec's "at end of stream, reset I/O queue" semantics
/// (§10.1.3 step 9) *and* so that BOM-sniffing state is reset —
/// `encoding_rs` exposes no public "reset BOM state" method.
/// Streaming calls re-use the same decoder so its internal
/// partial-sequence buffer persists.
pub(crate) struct TextDecoderState {
    fatal: bool,
    ignore_bom: bool,
    decoder: encoding_rs::Decoder,
}

impl core::fmt::Debug for TextDecoderState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `encoding_rs::Decoder` is opaque and intentionally does
        // not derive `Debug`; render only the fields we own.
        f.debug_struct("TextDecoderState")
            .field("encoding", &self.decoder.encoding().name())
            .field("fatal", &self.fatal)
            .field("ignore_bom", &self.ignore_bom)
            .finish_non_exhaustive()
    }
}

impl TextDecoderState {
    fn new(encoding: &'static Encoding, fatal: bool, ignore_bom: bool) -> Self {
        Self {
            fatal,
            ignore_bom,
            decoder: build_decoder(encoding, ignore_bom),
        }
    }

    /// Recreate the decoder so the next call starts from a fresh
    /// state.  Called after every non-stream `decode()` per §10.1.3.
    fn reset_decoder(&mut self) {
        self.decoder = build_decoder(self.decoder.encoding(), self.ignore_bom);
    }
}

fn build_decoder(encoding: &'static Encoding, ignore_bom: bool) -> encoding_rs::Decoder {
    if ignore_bom {
        encoding.new_decoder_without_bom_handling()
    } else {
        encoding.new_decoder_with_bom_removal()
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `TextEncoder.prototype`, install its accessor /
    /// method suite, and expose the `TextEncoder` constructor on
    /// `globals`.  Runs during `register_globals()` after
    /// `register_typed_array_prototype_global` (TextEncoder.encode
    /// allocates `Uint8Array` instances).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_text_encoder_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_text_encoder_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_text_encoder_members(proto_id);
        self.text_encoder_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("TextEncoder", native_text_encoder_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.text_encoder_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_text_encoder_members(&mut self, proto_id: ObjectId) {
        let encoding_sid = self.well_known.encoding;
        let gid = self.create_native_function("get encoding", native_text_encoder_get_encoding);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(encoding_sid),
            PropertyValue::Accessor {
                getter: Some(gid),
                setter: None,
            },
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let methods: [(StringId, NativeFn); 2] = [
            (
                self.well_known.encode,
                native_text_encoder_encode as NativeFn,
            ),
            (
                self.well_known.encode_into,
                native_text_encoder_encode_into as NativeFn,
            ),
        ];
        for (name_sid, func) in methods {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }

    /// Allocate `TextDecoder.prototype`, install its accessor /
    /// method suite, and expose the `TextDecoder` constructor on
    /// `globals`.  Must run during `register_globals()` after
    /// `register_text_encoder_global` — ordering is cosmetic; the
    /// two interfaces are independent.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None`.
    pub(in crate::vm) fn register_text_decoder_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_text_decoder_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_text_decoder_members(proto_id);
        self.text_decoder_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("TextDecoder", native_text_decoder_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.text_decoder_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_text_decoder_members(&mut self, proto_id: ObjectId) {
        let accessors: [(StringId, NativeFn); 3] = [
            (
                self.well_known.encoding,
                native_text_decoder_get_encoding as NativeFn,
            ),
            (
                self.well_known.fatal,
                native_text_decoder_get_fatal as NativeFn,
            ),
            (
                self.well_known.ignore_bom,
                native_text_decoder_get_ignore_bom as NativeFn,
            ),
        ];
        for (name_sid, getter) in accessors {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        let decode_sid = self.well_known.decode;
        let name = self.strings.get_utf8(decode_sid);
        let fn_id = self.create_native_function(&name, native_text_decoder_decode);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(decode_sid),
            PropertyValue::Data(JsValue::Object(fn_id)),
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand checks
// ---------------------------------------------------------------------------

fn require_text_encoder_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::TextEncoder) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "TextEncoder.prototype.{method} called on non-TextEncoder"
    )))
}

fn require_text_decoder_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::TextDecoder) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "TextDecoder.prototype.{method} called on non-TextDecoder"
    )))
}

// ---------------------------------------------------------------------------
// TextEncoder constructor + methods
// ---------------------------------------------------------------------------

fn native_text_encoder_ctor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'TextEncoder': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    // Mutate `kind` only — leave `prototype` alone so a subclass
    // `new.target.prototype` set by `do_new` survives.
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::TextEncoder;
    Ok(JsValue::Object(inst_id))
}

fn native_text_encoder_get_encoding(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_text_encoder_this(ctx, this, "encoding")?;
    Ok(JsValue::String(ctx.vm.well_known.utf_8))
}

fn native_text_encoder_encode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_text_encoder_this(ctx, this, "encode")?;
    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = match input_arg {
        JsValue::Undefined => ctx.vm.well_known.empty,
        other => super::super::coerce::to_string(ctx.vm, other)?,
    };
    // `get_utf8` allocates a fresh `String`; take its backing
    // `Vec<u8>` directly instead of copying via `.to_vec()`.
    let bytes: Vec<u8> = ctx.vm.strings.get_utf8(sid).into_bytes();
    let id = create_uint8_array_from_bytes(ctx.vm, bytes)?;
    Ok(JsValue::Object(id))
}

fn native_text_encoder_encode_into(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_text_encoder_this(ctx, this, "encodeInto")?;
    let source_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let dest_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    // WebIDL: source is `USVString` (→ ToString), destination is
    // `Uint8Array`.  Non-Uint8Array → TypeError.
    let source_sid = super::super::coerce::to_string(ctx.vm, source_arg)?;
    let JsValue::Object(dest_id) = dest_arg else {
        return Err(VmError::type_error(
            "Failed to execute 'encodeInto' on 'TextEncoder': parameter 2 is not of type 'Uint8Array'",
        ));
    };
    let (buffer_id, dest_offset, dest_len) = match ctx.vm.get_object(dest_id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind: ElementKind::Uint8,
        } => (buffer_id, byte_offset as usize, byte_length as usize),
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'encodeInto' on 'TextEncoder': parameter 2 is not of type 'Uint8Array'",
            ));
        }
    };

    // WHATWG §8.2.3 `encodeInto`: walk source code points, encode
    // into a scratch buffer until the next code point would
    // overflow `dest_len`, then perform a single read-modify-write
    // on `body_data[buffer_id]`.  Doing the copy inside the loop
    // would clone the entire backing `Arc<[u8]>` per code point —
    // O(N·B) work and allocations for an N-char source over a
    // B-byte buffer.
    let source = ctx.vm.strings.get_utf8(source_sid);
    let source_bytes = source.as_bytes();
    let mut read: usize = 0;
    let mut written: usize = 0;
    let mut scratch: Vec<u8> = Vec::with_capacity(dest_len.min(source.len()));
    // `source` is already UTF-8; re-encoding each char via
    // `encode_utf8` would be redundant.  Walk `char_indices()` to
    // get `(byte_offset, char)` pairs and copy the matching byte
    // slice from `source_bytes` directly.
    for (byte_idx, ch) in source.char_indices() {
        let ch_len = ch.len_utf8();
        if written + ch_len > dest_len {
            break;
        }
        scratch.extend_from_slice(&source_bytes[byte_idx..byte_idx + ch_len]);
        written += ch_len;
        read += ch.len_utf16();
    }
    if written > 0 {
        let current: &[u8] = ctx
            .vm
            .body_data
            .get(&buffer_id)
            .map(AsRef::as_ref)
            .unwrap_or(&[]);
        let needed = dest_offset + written;
        let mut new_bytes: Vec<u8> = current.to_vec();
        if new_bytes.len() < needed {
            new_bytes.resize(needed, 0);
        }
        new_bytes[dest_offset..dest_offset + written].copy_from_slice(&scratch);
        ctx.vm
            .body_data
            .insert(buffer_id, std::sync::Arc::from(new_bytes));
    }

    // Build the `{read, written}` result object.  Data properties
    // per §8.2.3 step 10-11 are plain enumerable/writable/configurable
    // (default `Object.create()` semantics).
    let result_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    #[allow(clippy::cast_precision_loss)]
    let read_val = JsValue::Number(read as f64);
    #[allow(clippy::cast_precision_loss)]
    let written_val = JsValue::Number(written as f64);
    let read_sid = ctx.vm.well_known.read;
    let written_sid = ctx.vm.well_known.written;
    ctx.vm.define_shaped_property(
        result_id,
        PropertyKey::String(read_sid),
        PropertyValue::Data(read_val),
        PropertyAttrs::DATA,
    );
    ctx.vm.define_shaped_property(
        result_id,
        PropertyKey::String(written_sid),
        PropertyValue::Data(written_val),
        PropertyAttrs::DATA,
    );
    Ok(JsValue::Object(result_id))
}

// ---------------------------------------------------------------------------
// TextDecoder constructor + accessors + decode
// ---------------------------------------------------------------------------

fn native_text_decoder_ctor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'TextDecoder': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let label_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let encoding = resolve_encoding_label(ctx, label_arg)?;
    let (fatal, ignore_bom) = parse_decoder_options(ctx, options_arg)?;

    let state = TextDecoderState::new(encoding, fatal, ignore_bom);
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::TextDecoder;
    ctx.vm.text_decoder_states.insert(inst_id, state);
    Ok(JsValue::Object(inst_id))
}

fn resolve_encoding_label(
    ctx: &mut NativeContext<'_>,
    label_arg: JsValue,
) -> Result<&'static Encoding, VmError> {
    let label = match label_arg {
        JsValue::Undefined => return Ok(encoding_rs::UTF_8),
        other => super::super::coerce::to_string(ctx.vm, other)?,
    };
    let raw = ctx.vm.strings.get_utf8(label);
    // WHATWG §4.2 "get an encoding" normalisation (ASCII
    // whitespace strip + case fold) runs inside `for_label`.
    // Encoding §10.2.1 step 2 rejects both `failure` (unknown
    // label) *and* `replacement` — the latter covers labels like
    // `iso-2022-kr` that map to the XSS-defence `replacement`
    // encoder, which TextDecoder is explicitly forbidden from
    // exposing.
    match Encoding::for_label(raw.as_bytes()) {
        Some(enc) if enc != encoding_rs::REPLACEMENT => Ok(enc),
        _ => Err(VmError::range_error(format!(
            "Failed to construct 'TextDecoder': The encoding label provided ('{raw}') is invalid"
        ))),
    }
}

fn parse_decoder_options(
    ctx: &mut NativeContext<'_>,
    options_arg: JsValue,
) -> Result<(bool, bool), VmError> {
    // WebIDL dictionary: `undefined` / `null` → defaults; objects →
    // read the two members via normal [[Get]] and ToBoolean per
    // §3.10.23.  A non-object non-nullish value is a TypeError
    // (WebIDL bindings gate dictionaries on IsCallable / IsObject).
    let obj_id = match options_arg {
        JsValue::Undefined | JsValue::Null => return Ok((false, false)),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'TextDecoder': The provided value is not of type 'TextDecoderOptions'",
            ));
        }
    };
    let fatal_sid = ctx.vm.well_known.fatal;
    let ignore_bom_sid = ctx.vm.well_known.ignore_bom;
    let fatal_val = ctx.get_property_value(obj_id, PropertyKey::String(fatal_sid))?;
    let ignore_bom_val = ctx.get_property_value(obj_id, PropertyKey::String(ignore_bom_sid))?;
    Ok((ctx.to_boolean(fatal_val), ctx.to_boolean(ignore_bom_val)))
}

fn native_text_decoder_get_encoding(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_text_decoder_this(ctx, this, "encoding")?;
    // `encoding_rs::Encoding::name()` returns the canonical
    // Pascal-Case name (e.g. "UTF-8"); WHATWG §8.1 requires lower-
    // case.  Fast-path the three interned names so the common case
    // skips both the `to_ascii_lowercase` allocation and the
    // `intern` hashmap probe.  Other encodings fall through to the
    // per-call intern.
    // `require_text_decoder_this` already asserted `kind ==
    // TextDecoder`, and `text_decoder_states` is populated
    // atomically with that kind promotion in the ctor — a missing
    // entry here is an internal invariant violation, not a
    // user-observable state.
    //
    // `Encoding::name()` is `&'static str`, so extracting it
    // before the fast-path match + fallback drops the side-table
    // borrow and lets `intern_lowercase` take `&mut ctx.vm`.
    let name: &'static str = ctx
        .vm
        .text_decoder_states
        .get(&id)
        .expect("internal invariant: TextDecoder kind without state entry")
        .decoder
        .encoding()
        .name();
    let encoding_sid = match name {
        "UTF-8" => ctx.vm.well_known.utf_8,
        "UTF-16LE" => ctx.vm.well_known.utf_16le,
        "UTF-16BE" => ctx.vm.well_known.utf_16be,
        _ => intern_lowercase(ctx.vm, name),
    };
    Ok(JsValue::String(encoding_sid))
}

fn native_text_decoder_get_fatal(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_text_decoder_this(ctx, this, "fatal")?;
    let fatal = ctx
        .vm
        .text_decoder_states
        .get(&id)
        .expect("internal invariant: TextDecoder kind without state entry")
        .fatal;
    Ok(JsValue::Boolean(fatal))
}

fn native_text_decoder_get_ignore_bom(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_text_decoder_this(ctx, this, "ignoreBOM")?;
    let ignore_bom = ctx
        .vm
        .text_decoder_states
        .get(&id)
        .expect("internal invariant: TextDecoder kind without state entry")
        .ignore_bom;
    Ok(JsValue::Boolean(ignore_bom))
}

fn native_text_decoder_decode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_text_decoder_this(ctx, this, "decode")?;
    let input_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let bytes = extract_buffer_source_bytes(ctx, input_arg)?;
    let stream = parse_decode_stream(ctx, options_arg)?;

    let decoded = {
        let state = ctx
            .vm
            .text_decoder_states
            .get_mut(&id)
            .expect("internal invariant: TextDecoder kind without state entry");
        let fatal = state.fatal;
        // Size the output buffer so the decoder can write every
        // output byte in a single call — `decode_to_string*`
        // never grows the `String` itself and returns
        // `OutputFull` the moment there is not enough spare
        // capacity.  The replacement character U+FFFD is 3 UTF-8
        // bytes, so even a single malformed input byte needs more
        // than 1 byte of output capacity.
        let needed = state
            .decoder
            .max_utf8_buffer_length(bytes.len())
            .ok_or_else(|| {
                VmError::range_error(
                    "Failed to execute 'decode' on 'TextDecoder': input length overflows",
                )
            })?;
        let mut out = String::with_capacity(needed);
        let last = !stream;
        if fatal {
            let (result, _read) = state
                .decoder
                .decode_to_string_without_replacement(&bytes, &mut out, last);
            if matches!(result, DecoderResult::Malformed(_, _)) {
                // Per §10.1.3 step 6: a malformed token with fatal
                // flag set throws TypeError.  Rebuild the decoder
                // so subsequent calls start fresh rather than
                // inheriting the aborted state.
                state.reset_decoder();
                return Err(VmError::type_error(
                    "Failed to execute 'decode' on 'TextDecoder': The encoded data was not valid",
                ));
            }
        } else {
            let (_result, _read, _had_errors) =
                state.decoder.decode_to_string(&bytes, &mut out, last);
        }
        if last {
            state.reset_decoder();
        }
        out
    };
    let sid = ctx.vm.strings.intern(&decoded);
    Ok(JsValue::String(sid))
}

fn parse_decode_stream(ctx: &mut NativeContext<'_>, options_arg: JsValue) -> Result<bool, VmError> {
    let obj_id = match options_arg {
        JsValue::Undefined | JsValue::Null => return Ok(false),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'decode' on 'TextDecoder': The provided value is not of type 'TextDecodeOptions'",
            ));
        }
    };
    let stream_sid = ctx.vm.well_known.stream;
    let stream_val = ctx.get_property_value(obj_id, PropertyKey::String(stream_sid))?;
    Ok(ctx.to_boolean(stream_val))
}

/// Pull raw bytes out of a BufferSource argument.  Accepts
/// `ArrayBuffer`, any `TypedArray`, or `DataView`; `undefined` →
/// empty byte slice (spec §10.1.3 step 2).  Anything else throws
/// TypeError — matches the WebIDL `BufferSource` union.
///
/// Returns `Arc<[u8]>` so the full-buffer `ArrayBuffer` case is a
/// pure refcount bump (no copy).  View cases (offset / length
/// sub-range) allocate a fresh `Arc<[u8]>` covering the sub-slice
/// — the clone is linear in the view's byte length, not the
/// backing buffer's.
fn extract_buffer_source_bytes(
    ctx: &NativeContext<'_>,
    input_arg: JsValue,
) -> Result<std::sync::Arc<[u8]>, VmError> {
    use std::sync::Arc;
    match input_arg {
        JsValue::Undefined => Ok(Arc::from(&[][..])),
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::ArrayBuffer => Ok(super::array_buffer::array_buffer_bytes(ctx.vm, id)),
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
            } => {
                let backing = super::array_buffer::array_buffer_bytes(ctx.vm, buffer_id);
                let start = byte_offset as usize;
                let end = start + byte_length as usize;
                let slice = backing.get(start..end).unwrap_or(&[]);
                // Sub-range view: fresh `Arc<[u8]>` sized to the
                // view, not the backing buffer.  If `byte_offset
                // == 0 && byte_length == backing.len()`, skip the
                // clone and hand back the original Arc.
                if start == 0 && end == backing.len() {
                    Ok(backing)
                } else {
                    Ok(Arc::from(slice))
                }
            }
            _ => Err(VmError::type_error(
                "Failed to execute 'decode' on 'TextDecoder': parameter 1 is not of type 'BufferSource'",
            )),
        },
        _ => Err(VmError::type_error(
            "Failed to execute 'decode' on 'TextDecoder': parameter 1 is not of type 'BufferSource'",
        )),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Allocate a fresh `Uint8Array` whose underlying buffer owns
/// `bytes`.  Uses the shared `body_data` / `array_buffer_prototype`
/// + `uint8_array_prototype` so GC sweep prunes it like any
/// other view allocation.
///
/// Returns `RangeError` if `bytes.len()` exceeds `u32::MAX` — the
/// TypedArray `[[ByteLength]]` slot is `u32` so a silent truncation
/// would produce a view with a length inconsistent with the
/// backing buffer.
fn create_uint8_array_from_bytes(vm: &mut VmInner, bytes: Vec<u8>) -> Result<ObjectId, VmError> {
    let byte_length = u32::try_from(bytes.len()).map_err(|_| {
        VmError::range_error(
            "Failed to execute 'encode' on 'TextEncoder': encoded byte length exceeds 4 GiB",
        )
    })?;
    let buffer_id =
        super::array_buffer::create_array_buffer_from_bytes(vm, std::sync::Arc::from(bytes));
    // Temp-root `buffer_id` across the Uint8Array allocation via
    // the RAII `push_temp_root` guard so the stack is restored on
    // every exit path (normal return and panic unwinding alike) —
    // a bare `stack.push` / `stack.pop` pair would leak the root
    // through any `catch_unwind` upstream.  GC is disabled inside
    // native calls today (`gc_enabled = false`), so the second
    // `alloc_object` cannot currently trigger a collection, but
    // the rooting matches the invariant used by `wrap_in_array_-
    // iterator` / event constructors / the typed-array ctor.
    let mut g = vm.push_temp_root(JsValue::Object(buffer_id));
    let proto = g.subclass_array_prototypes[ElementKind::Uint8.index()];
    let view_id = g.alloc_object(Object {
        kind: ObjectKind::TypedArray {
            buffer_id,
            byte_offset: 0,
            byte_length,
            element_kind: ElementKind::Uint8,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Guard `g` is dropped here, restoring the stack — either
    // cleanly (post-alloc return) or during panic unwinding.
    Ok(view_id)
}

fn intern_lowercase(vm: &mut VmInner, s: &str) -> StringId {
    // Fast path: encoding labels are ASCII-only per WHATWG §4.2
    // ("get an encoding"); when already lowercase, skip the
    // `to_ascii_lowercase` clone.  Byte-level check avoids the
    // UTF-8 decode `chars()` would trigger for no gain.
    if s.bytes().any(|b| b.is_ascii_uppercase()) {
        vm.strings.intern(&s.to_ascii_lowercase())
    } else {
        vm.strings.intern(s)
    }
}
