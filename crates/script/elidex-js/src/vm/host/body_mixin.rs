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
/// yield `TypeError` (WebIDL §3.2 brand checks).  The error
/// message names `Request`/`Response` rather than a fictional
/// `Body` prototype because the Body mixin is installed directly
/// on the two concrete prototypes — there is no script-visible
/// `Body` interface.
fn require_body_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(body_illegal_invocation(method));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Request | ObjectKind::Response => Ok(id),
        _ => Err(body_illegal_invocation(method)),
    }
}

/// Shared "brand check failed" error for the Body mixin methods.
/// Uses the same `"Failed to execute '{method}' on '{Interface}':
/// Illegal invocation"` shape as `event_target::require_receiver`
/// and the WebIDL brand checks on Request / Response / Headers,
/// so user-facing error messages stay uniform across built-ins.
/// There is no script-visible `Body` interface, so the `{Interface}`
/// slot names the two concrete interfaces that expose the mixin
/// (R28.1).
fn body_illegal_invocation(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'Request or Response': Illegal invocation"
    ))
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

/// Return the receiver's companion-Headers `Content-Type` value,
/// or the empty `StringId` if absent.  Used by `.blob()` to seed
/// the new Blob's `type`.
///
/// Multi-valued `Content-Type` headers are combined with `", "`
/// so the returned string matches what
/// `resp.headers.get('content-type')` would produce (WHATWG
/// Fetch §5.2 "get").  Without this, `resp.headers.get` and
/// `(await resp.blob()).type` could disagree on the same
/// Response — a consumer-observable inconsistency.
fn content_type_of(ctx: &mut NativeContext<'_>, id: ObjectId) -> super::super::value::StringId {
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
    // Collect every matching value in insertion order.  Typical
    // responses have 0 or 1 `Content-Type` so the Vec allocation
    // rarely grows.
    let values: Vec<super::super::value::StringId> = ctx
        .vm
        .headers_states
        .get(&headers_id)
        .map(|state| {
            state
                .list
                .iter()
                .filter(|(n, _)| *n == ct_name)
                .map(|(_, v)| *v)
                .collect()
        })
        .unwrap_or_default();
    if values.is_empty() {
        return empty;
    }
    // Share the exact algorithm `Headers.get` uses so the two
    // surfaces cannot drift (WHATWG §5.2 combine).
    super::headers::join_values_comma_space(ctx.vm, &values)
}

// ---------------------------------------------------------------------------
// Native methods
// ---------------------------------------------------------------------------

/// Outcome of the alloc-free body-mixin prologue
/// ([`check_body`]): either the body has already been consumed
/// (caller must reject its Promise with TypeError) or the owner
/// object's id plus the consumed bytes are ready to use.
///
/// The Promise itself is **not** created here — callers own
/// creation so they can root the Promise across the subsequent
/// allocation-heavy post-processing (UTF-8 decode / JSON.parse /
/// ArrayBuffer / Blob alloc).  Rooting at the call site is the
/// R13 GC-safety invariant that this split enforces.
enum BodyReadCheck {
    /// Body already consumed — caller must reject its Promise
    /// with `TypeError("Body stream is already used")`.
    AlreadyUsed,
    /// Body successfully locked for consumption; `bytes` carries
    /// the full content and `owner_id` is the Request / Response
    /// the bytes came from (needed by `.blob()` for its
    /// `content-type` lookup).
    Ready {
        owner_id: ObjectId,
        bytes: Arc<[u8]>,
    },
}

/// Allocation-free Body-mixin prologue: brand-check, check
/// `body_used`, mark as consumed, and return the owner's bytes.
/// Matches WHATWG Fetch §5.2 "consume body" steps 1-3 minus the
/// Promise allocation — the caller handles Promise creation
/// + rooting + settlement.
///
/// This helper is deliberately alloc-free (in the VM
/// `alloc_object` sense) so it can run **before** the caller
/// creates and roots the Promise that the post-processing step
/// will eventually settle.  `HashSet::insert` may grow its
/// backing Rust `Vec` but does not touch `VmInner.objects`.
fn check_body(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<BodyReadCheck, VmError> {
    let id = require_body_this(ctx, this, method)?;
    if ctx.vm.body_used.contains(&id) {
        return Ok(BodyReadCheck::AlreadyUsed);
    }
    mark_body_used(ctx, id);
    let bytes = read_body_bytes(ctx, id);
    Ok(BodyReadCheck::Ready {
        owner_id: id,
        bytes,
    })
}

/// Immediately reject `promise` with the "Body stream is
/// already used" TypeError.  The caller holds the root for
/// `promise`; this helper only mutates Promise state.
fn reject_body_already_used(ctx: &mut NativeContext<'_>, promise: ObjectId) {
    let reason = thrown_type_error(ctx, "Body stream is already used");
    reject_promise_sync(ctx.vm, promise, reason);
}

pub(super) fn native_body_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Check first (alloc-free), then create + root the Promise
    // for the alloc-heavy settle path (R13 GC safety).
    let check = check_body(ctx, this, "text")?;
    let promise = create_promise(ctx.vm);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    match check {
        BodyReadCheck::AlreadyUsed => reject_body_already_used(ctx, promise),
        BodyReadCheck::Ready { bytes, .. } => {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let sid = ctx.vm.strings.intern(&text);
            resolve_promise_sync(ctx.vm, promise, JsValue::String(sid));
        }
    }
    Ok(JsValue::Object(promise))
}

pub(super) fn native_body_json(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let check = check_body(ctx, this, "json")?;
    let promise = create_promise(ctx.vm);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    match check {
        BodyReadCheck::AlreadyUsed => reject_body_already_used(ctx, promise),
        BodyReadCheck::Ready { bytes, .. } => {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let sid = ctx.vm.strings.intern(&text);
            // Delegate to `JSON.parse` — matches spec §5 "consume
            // body" → "parse JSON from bytes" step.  Errors
            // propagate via `vm_error_to_thrown`.
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
        }
    }
    Ok(JsValue::Object(promise))
}

pub(super) fn native_body_array_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let check = check_body(ctx, this, "arrayBuffer")?;
    let promise = create_promise(ctx.vm);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    match check {
        BodyReadCheck::AlreadyUsed => reject_body_already_used(ctx, promise),
        BodyReadCheck::Ready { bytes, .. } => {
            let buf_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
            resolve_promise_sync(ctx.vm, promise, JsValue::Object(buf_id));
        }
    }
    Ok(JsValue::Object(promise))
}

pub(super) fn native_body_blob(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let check = check_body(ctx, this, "blob")?;
    let promise = create_promise(ctx.vm);
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted_holder = super::super::value::NativeContext { vm: &mut *g };
    let ctx = &mut rooted_holder;
    match check {
        BodyReadCheck::AlreadyUsed => reject_body_already_used(ctx, promise),
        BodyReadCheck::Ready { owner_id, bytes } => {
            let type_sid = content_type_of(ctx, owner_id);
            let blob_id = create_blob_from_bytes(ctx.vm, bytes, type_sid);
            resolve_promise_sync(ctx.vm, promise, JsValue::Object(blob_id));
        }
    }
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
