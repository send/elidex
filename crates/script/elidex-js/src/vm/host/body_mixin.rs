//! `Body` mixin (WHATWG Fetch §5 Body, minimal Phase 2 form).
//!
//! Provides the four consumer methods shared by `Request` and
//! `Response`:
//!
//! - `.text()` → `Promise<string>` — UTF-8 decode (lossy).
//! - `.json()` → `Promise<any>` — decode as UTF-8 then feed to
//!   `JSON.parse`; parse error bubbles through the returned
//!   Promise's reject path.
//! - `.arrayBuffer()` → `Promise<ArrayBuffer>` — new ArrayBuffer
//!   sharing the backing `Arc<[u8]>`.
//! - `.blob()` → `Promise<Blob>` — new Blob whose `type` defaults
//!   from the receiver's `Content-Type` header (or `""` if
//!   absent).
//!
//! ## Contracts
//!
//! - `bodyUsed` tracking: every consumer marks the receiver in
//!   [`super::super::VmInner::body_used`].  A second consumer
//!   call rejects the returned Promise with `TypeError`.
//! - Promise settlement is synchronous (uses
//!   [`super::blob::resolve_promise_sync`] /
//!   [`super::blob::reject_promise_sync`]).  Body bytes live in
//!   memory — the spec models reads as async, but a resolved
//!   promise with in-memory data matches observable behaviour
//!   once `await` has run a microtask cycle.
//! - Empty / missing body: a Request or Response without a
//!   `body_data` entry decodes to the empty string (`text()`),
//!   parses to `SyntaxError` (`json()`), or produces an empty
//!   ArrayBuffer / Blob.  Matches browsers.
//!
//! ## Dispatch
//!
//! Each native first resolves the receiver to either a `Request`
//! or `Response` ObjectId via a small brand check.  The two
//! variants share identical body handling, so we dispatch once and
//! forward to the shared `do_*` helpers below.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::natives_promise::create_promise;
use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, VmError};
use super::blob::{create_blob_from_bytes, reject_promise_sync, resolve_promise_sync};

/// Brand-check `this` against `Request` / `Response`.  Returns
/// the `ObjectId` for downstream body lookup.  Non-Body receivers
/// yield `TypeError` (WebIDL §3.2 brand checks).
fn require_body_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Body.prototype.{method} called on non-Body"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Request | ObjectKind::Response => Ok(id),
        _ => Err(VmError::type_error(format!(
            "Body.prototype.{method} called on non-Body"
        ))),
    }
}

/// Record the receiver as body-consumed.  Called before any read
/// path so a concurrent re-read in the same tick still observes
/// the consumed state (relevant for `r.text(); r.text()` back-to-
/// back: the second call must reject immediately).
fn mark_body_used(ctx: &mut NativeContext<'_>, id: ObjectId) {
    ctx.vm.body_used.insert(id);
}

/// Produce a `TypeError` `JsValue` suitable for rejecting a
/// settle_promise-returned capability.  Uses the same `vm_error_to_thrown`
/// mechanism as the runtime's other throwers so `.catch(e => e)`
/// receives a real Error instance.
fn thrown_type_error(ctx: &mut NativeContext<'_>, msg: &str) -> JsValue {
    let err = VmError::type_error(msg);
    ctx.vm.vm_error_to_thrown(&err)
}

/// Return the receiver's body bytes (or empty if no entry).
fn read_body_bytes(ctx: &NativeContext<'_>, id: ObjectId) -> Arc<[u8]> {
    ctx.vm
        .body_data
        .get(&id)
        .cloned()
        .unwrap_or_else(|| Arc::from(&[][..]))
}

/// Return the receiver's companion-Headers `Content-Type` value
/// or the empty `StringId` if absent.  Used by `.blob()` to seed
/// the new Blob's `type`.
fn content_type_of(ctx: &NativeContext<'_>, id: ObjectId) -> super::super::value::StringId {
    let headers_id = match ctx.vm.get_object(id).kind {
        ObjectKind::Request => ctx.vm.request_states.get(&id).map(|s| s.headers_id),
        ObjectKind::Response => ctx.vm.response_states.get(&id).map(|s| s.headers_id),
        _ => None,
    };
    let empty = ctx.vm.well_known.empty;
    let ct_name = ctx.vm.well_known.content_type;
    let Some(headers_id) = headers_id else {
        return empty;
    };
    ctx.vm
        .headers_states
        .get(&headers_id)
        .and_then(|state| {
            state
                .list
                .iter()
                .find(|(n, _)| *n == ct_name)
                .map(|(_, v)| *v)
        })
        .unwrap_or(empty)
}

// ---------------------------------------------------------------------------
// Native methods
// ---------------------------------------------------------------------------

pub(super) fn native_body_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_body_this(ctx, this, "text")?;
    let promise = create_promise(ctx.vm);
    if ctx.vm.body_used.contains(&id) {
        let reason = thrown_type_error(ctx, "Body stream is already used");
        reject_promise_sync(ctx.vm, promise, reason);
        return Ok(JsValue::Object(promise));
    }
    mark_body_used(ctx, id);
    let bytes = read_body_bytes(ctx, id);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let sid = ctx.vm.strings.intern(&text);
    resolve_promise_sync(ctx.vm, promise, JsValue::String(sid));
    Ok(JsValue::Object(promise))
}

pub(super) fn native_body_json(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_body_this(ctx, this, "json")?;
    let promise = create_promise(ctx.vm);
    if ctx.vm.body_used.contains(&id) {
        let reason = thrown_type_error(ctx, "Body stream is already used");
        reject_promise_sync(ctx.vm, promise, reason);
        return Ok(JsValue::Object(promise));
    }
    mark_body_used(ctx, id);
    let bytes = read_body_bytes(ctx, id);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let sid = ctx.vm.strings.intern(&text);
    // Delegate to `JSON.parse` — matches spec §5 "consume body" →
    // "parse JSON from bytes" step.  Errors propagate via
    // `vm_error_to_thrown`.
    let parse_result = super::super::natives_json::native_json_parse(
        ctx,
        JsValue::Undefined,
        &[JsValue::String(sid)],
    );
    match parse_result {
        Ok(val) => resolve_promise_sync(ctx.vm, promise, val),
        Err(err) => {
            let reason = ctx.vm.vm_error_to_thrown(&err);
            reject_promise_sync(ctx.vm, promise, reason);
        }
    }
    Ok(JsValue::Object(promise))
}

pub(super) fn native_body_array_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_body_this(ctx, this, "arrayBuffer")?;
    let promise = create_promise(ctx.vm);
    if ctx.vm.body_used.contains(&id) {
        let reason = thrown_type_error(ctx, "Body stream is already used");
        reject_promise_sync(ctx.vm, promise, reason);
        return Ok(JsValue::Object(promise));
    }
    mark_body_used(ctx, id);
    let bytes = read_body_bytes(ctx, id);
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
    resolve_promise_sync(ctx.vm, promise, JsValue::Object(buf_id));
    Ok(JsValue::Object(promise))
}

pub(super) fn native_body_blob(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_body_this(ctx, this, "blob")?;
    let promise = create_promise(ctx.vm);
    if ctx.vm.body_used.contains(&id) {
        let reason = thrown_type_error(ctx, "Body stream is already used");
        reject_promise_sync(ctx.vm, promise, reason);
        return Ok(JsValue::Object(promise));
    }
    mark_body_used(ctx, id);
    let bytes = read_body_bytes(ctx, id);
    let type_sid = content_type_of(ctx, id);
    let blob_id = create_blob_from_bytes(ctx.vm, bytes, type_sid);
    resolve_promise_sync(ctx.vm, promise, JsValue::Object(blob_id));
    Ok(JsValue::Object(promise))
}

// ---------------------------------------------------------------------------
// Prototype install helper
// ---------------------------------------------------------------------------

impl super::super::VmInner {
    /// Install the four Body-mixin methods (`text` / `json` /
    /// `arrayBuffer` / `blob`) on a given prototype.  Called
    /// separately for `Request.prototype` and `Response.prototype`
    /// during `register_globals` — the two interfaces share the
    /// same method bodies so we can't just copy one prototype's
    /// property table into the other without a second install
    /// pass.
    pub(in crate::vm) fn install_body_mixin_methods(&mut self, proto_id: ObjectId) {
        // The `.blob()` method name is lowercase `"blob"` —
        // distinct from the ctor global `"Blob"` (which uses
        // `blob_global`).  Every call site re-interns "blob"
        // against the dedup-ing pool, so the extra intern is
        // effectively free after the first install.
        let blob_method_sid = self.strings.intern("blob");
        let method_sids: [super::super::value::StringId; 4] = [
            self.well_known.text,
            self.well_known.json,
            self.well_known.array_buffer,
            blob_method_sid,
        ];
        let method_fns: [super::super::NativeFn; 4] = [
            native_body_text,
            native_body_json,
            native_body_array_buffer,
            native_body_blob,
        ];
        for (name_sid, func) in method_sids.into_iter().zip(method_fns) {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                super::super::value::PropertyValue::Data(JsValue::Object(fn_id)),
                super::super::shape::PropertyAttrs::METHOD,
            );
        }
    }
}
