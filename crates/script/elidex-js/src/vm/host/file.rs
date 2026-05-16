//! `File` interface (File API §4) — `Blob` subclass.
//!
//! ```text
//! File instance (ObjectKind::File, payload-free)
//!   → File.prototype  (this module)
//!     → Blob.prototype  (vm/host/blob.rs)
//!       → Object.prototype
//! ```
//!
//! ## Storage design (v1.2 simplification of plan v1.1)
//!
//! Each File has its own `ObjectId` with `ObjectKind::File` (brand
//! distinct from `Blob`).  The bytes + MIME type live in
//! [`super::super::VmInner::blob_data`] keyed by the File's own
//! `ObjectId` — so inherited `Blob.prototype.size` / `.type` /
//! `.slice()` / `.text()` / `.stream()` / `.arrayBuffer()` accessors
//! work after widening their brand checks to accept `Blob | File`
//! (see [`super::blob::require_blob_or_file_this`]).  The File-only
//! side-table here carries the spec-extension fields
//! (`name`, `lastModified`) that Blob does not have.
//!
//! This avoids the alternative "inner Blob ObjectId" design (plan v1.1)
//! which required an extra allocation per File + GC trace fan-out for
//! the inner Blob — the simpler design matches the spec's single
//! inheritance semantics (File IS-A Blob).
//!
//! ## Scope
//!
//! - `new File(bits, name, options?)` — `bits` = `Array<BufferSource |
//!   Blob | USVString>`; `options` = `{type?: USVString, lastModified?:
//!   long long, endings?: "transparent" | "native"}`.  Reuses
//!   `Blob`'s `collect_blob_parts_bytes` (now `endings`-aware) so
//!   File and Blob ctors share the same iterator-protocol + part
//!   coercion path.
//! - `.name` IDL readonly attr (USVString, `/` → `:` per FileAPI §4.1
//!   step 2).
//! - `.lastModified` IDL readonly attr (long long, epoch ms; default
//!   `Date.now()` at construction time per FileAPI §4.1 step 3).
//! - `endings: "native"` line-ending normalize: USVString entries in
//!   `bits` have `\r\n` and lone `\r` collapsed to `\n` (Web platform
//!   default per FileAPI §4.1 step 1 note).  BufferSource / Blob
//!   entries are not normalized.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, StringId,
    VmError,
};
use super::super::{NativeFn, VmInner};
use super::blob::{
    collect_blob_parts_bytes_with_endings, parse_blob_options_endings, parse_blob_options_type,
    BlobData, EndingsKind,
};
use super::events::install_ctor;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Per-`File` out-of-band state, keyed in
/// [`super::super::VmInner::file_data`] by the instance's `ObjectId`.
///
/// Only carries File-specific extension fields — bytes + MIME type
/// live alongside Blob entries in [`super::super::VmInner::blob_data`]
/// keyed by the same `ObjectId` (the File IS-A Blob in WebIDL terms).
#[derive(Debug)]
pub(crate) struct FileSideData {
    /// `File.name` USVString — `/` already replaced with `:` per
    /// FileAPI §4.1 step 2 at construction time.
    pub(crate) name: StringId,
    /// `File.lastModified` long long, epoch ms.  Default `Date.now()`
    /// at construction time per FileAPI §4.1 step 3.  Stored as f64
    /// because JS Number is the only numeric representation; values
    /// are coerced from WebIDL `long long` at ctor time.
    pub(crate) last_modified_ms: f64,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `File.prototype` (chains through `Blob.prototype` per
    /// FileAPI §4 inheritance), install the `name` + `lastModified`
    /// accessors, and expose the `File` constructor on `globals`.
    ///
    /// Must run after `register_blob_global` because the prototype
    /// chains on `blob_prototype`.
    pub(in crate::vm) fn register_file_global(&mut self) {
        let blob_proto = self
            .blob_prototype
            .expect("register_file_global called before register_blob_global");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(blob_proto),
            extensible: true,
        });
        self.install_file_members(proto_id);
        self.file_prototype = Some(proto_id);

        install_ctor(
            self,
            proto_id,
            "File",
            native_file_constructor,
            self.well_known.file_global,
        );
    }

    fn install_file_members(&mut self, proto_id: ObjectId) {
        let accessors: [(StringId, NativeFn); 2] = [
            (self.well_known.name, native_file_get_name as NativeFn),
            (
                self.well_known.last_modified,
                native_file_get_last_modified as NativeFn,
            ),
        ];
        for (name_sid, getter) in accessors {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_file_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "File.prototype.{method} called on non-File"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::File) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "File.prototype.{method} called on non-File"
        )))
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new File(fileBits, fileName, options?)` (File API §4.1).
fn native_file_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'File': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    // WebIDL `required` enforcement — both `fileBits` and `fileName`
    // are required positional args; missing them is a TypeError
    // before any coercion (FileAPI §4.1 IDL).
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to construct 'File': 2 arguments required, but only {} present.",
            args.len()
        )));
    }
    let bits_arg = args[0];
    let name_arg = args[1];
    let options_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);

    // Parse `options` first so `endings` is known before bits coercion
    // (need to thread endings into the per-part normalize step).
    let (type_sid, endings, last_modified_ms) = parse_file_options(ctx, options_arg)?;

    // Coerce bits using Blob's helper extended with the endings flag.
    // Spec §4.1 step 1 + §3.2 step 2 are unified through this helper.
    let bytes: Arc<[u8]> = collect_blob_parts_bytes_with_endings(ctx, bits_arg, endings)?;

    // Coerce name to USVString + replace `/` with `:` per §4.1 step 2.
    // Spec calls for USVString (replace unpaired surrogates with U+FFFD);
    // our StringPool holds WTF-16 internally and ToString already runs
    // §7.1.17 ToString which preserves the input.  The `/→:` is the
    // only mandated transformation here.
    let name_sid_raw = super::super::coerce::to_string(ctx.vm, name_arg)?;
    let name_str = ctx.vm.strings.get_utf8(name_sid_raw);
    let name_sid = if name_str.contains('/') {
        let replaced = name_str.replace('/', ":");
        ctx.vm.strings.intern(&replaced)
    } else {
        name_sid_raw
    };

    // Promote the pre-allocated Ordinary instance to File — do not
    // touch `prototype` so the `new.target.prototype` chain installed
    // by `do_new` survives (PR5a2 R7.2/R7.3 lesson, same invariant as
    // Blob ctor).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::File;
    // Insert into BOTH side-tables.  blob_data is keyed by the File's
    // own ObjectId so inherited Blob.prototype accessors (size / type /
    // slice / text / stream / arrayBuffer) work transparently once
    // their brand checks accept Blob | File.
    ctx.vm
        .blob_data
        .insert(inst_id, BlobData { type_sid, bytes });
    ctx.vm.file_data.insert(
        inst_id,
        FileSideData {
            name: name_sid,
            last_modified_ms,
        },
    );

    Ok(JsValue::Object(inst_id))
}

/// Parse the options dict into `(type, endings, lastModified)`.
/// FileAPI §4.1 step 3 — `lastModified` defaults to "current epoch
/// timestamp" (i.e. `Date.now()` at construction time).
fn parse_file_options(
    ctx: &mut NativeContext<'_>,
    options: JsValue,
) -> Result<(StringId, EndingsKind, f64), VmError> {
    match options {
        JsValue::Undefined | JsValue::Null => Ok((
            ctx.vm.well_known.empty,
            EndingsKind::Transparent,
            now_epoch_ms(ctx.vm),
        )),
        JsValue::Object(opts_id) => {
            let type_sid = parse_blob_options_type(ctx, options)?;
            let endings = parse_blob_options_endings(ctx, options, "File")?;
            let last_modified_key = PropertyKey::String(ctx.vm.well_known.last_modified);
            let last_modified_val = ctx.get_property_value(opts_id, last_modified_key)?;
            let last_modified_ms = match last_modified_val {
                JsValue::Undefined => now_epoch_ms(ctx.vm),
                other => {
                    // WebIDL `long long` coercion: ToNumber then
                    // ToInt64 (truncate toward zero, modulo 2^64).
                    // For File.lastModified, negative and large
                    // values are spec-allowed (the value is opaque
                    // to UA — just stored verbatim).
                    let n = super::super::coerce::to_number(ctx.vm, other)?;
                    // Truncate to long long range — finite trunc, NaN → 0.
                    // Normalize `-0` (from e.g. `-0.5.trunc()`) to `+0`
                    // per WebIDL §3.10.10 step 2; same hazard as the
                    // ProgressEvent `loaded` / `total` fix (lesson #216).
                    if n.is_finite() {
                        let t = n.trunc();
                        if t == 0.0 {
                            0.0
                        } else {
                            t
                        }
                    } else {
                        0.0
                    }
                }
            };
            Ok((type_sid, endings, last_modified_ms))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'File': options must be an object",
        )),
    }
}

/// Current Unix epoch milliseconds, matching what `Date.now()` returns
/// in browsers — spec-mandated for `File.lastModified` default per
/// FileAPI §4.1 step 3.  Sourced from `SystemTime::now()` so
/// `new Date(file.lastModified)` renders the current wall-clock time
/// rather than 1970 (Copilot R1 spec finding).
///
/// `SystemTime::duration_since(UNIX_EPOCH)` fails only when the system
/// clock predates 1970 — extraordinarily unusual; fall back to 0 in
/// that case (the spec-mandated default for invalid timestamps).
#[allow(clippy::cast_precision_loss)]
fn now_epoch_ms(_vm: &VmInner) -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_file_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_this(ctx, this, "name")?;
    let sid = ctx
        .vm
        .file_data
        .get(&id)
        .map_or(ctx.vm.well_known.empty, |d| d.name);
    Ok(JsValue::String(sid))
}

fn native_file_get_last_modified(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_this(ctx, this, "lastModified")?;
    let ms = ctx
        .vm
        .file_data
        .get(&id)
        .map_or(0.0, |d| d.last_modified_ms);
    Ok(JsValue::Number(ms))
}
