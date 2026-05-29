//! `WebAssembly.Memory` constructor + `.grow` method + `.buffer`
//! accessor with DR-11 live-view routing — slot `#11-wasm-vm` /
//! D-16, plan-memo §5 Stage 4.1.
//!
//! Per WASM JS API §5.3, `new WebAssembly.Memory({initial,
//! maximum?})`:
//!
//! - `initial`: u32 page count (WebIDL `[EnforceRange]`).
//! - `maximum`: optional u32 cap.
//! - Page size: 64 KiB.
//!
//! `.grow(delta)` returns pre-grow page count + detaches the cached
//! ArrayBuffer per `refresh the Memory buffer` step 5.1.
//! `.buffer` is wrapper-identity-stable (DR-11 elidex impl choice;
//! IDL has no `[SameObject]`) — first access allocates a fresh
//! ArrayBuffer wrapper backed by a live [`WasmMemoryView`] stashed
//! on the payload, and inserts the
//! [`VmInner::wasm_backed_buffers`] reverse-lookup entry so the
//! ArrayBuffer hot-path routing in `byte_io::*_with_routing` +
//! `array_buffer_*` accessors can dispatch through the view.
//!
//! Coupling invariant (plan-memo §5 Stage 4.1):
//! `wasm_backed_buffers[buf_id] = Some(mem_id) ⇔
//! wasm_memory_storage[mem_id].view = Some(_)`.  Both writes (insert
//! `+ Some`) at `.buffer` first-fire run paired; both removes (clear
//! `+ None`) at detach time run paired.

use elidex_wasm_runtime::{WasmMemoryDescriptor, WasmRuntime};

use super::super::super::error::VmError;
use super::super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind};
use super::super::super::wasm_payload::WasmMemoryPayload;
use super::super::array_buffer;
use super::errors::wasm_error_to_vm_error;
use super::table::coerce_uint32_pub;

/// `new WebAssembly.Memory({initial, maximum?})` — WASM JS API §5.3.
pub(super) fn native_wasm_memory_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let descriptor_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let (initial, maximum) = coerce_memory_descriptor(ctx, descriptor_arg)?;

    let descriptor = WasmMemoryDescriptor { initial, maximum };
    let runtime = match ctx.vm.vm_wasm_runtime() {
        Ok(rt) => rt.clone(),
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };
    let memory = match WasmRuntime::new_memory(&runtime, descriptor) {
        Ok(m) => m,
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };

    let proto = ctx
        .vm
        .wasm_memory_prototype
        .expect("wasm_memory_prototype populated in register_wasm_namespace");
    let receiver = ctx.vm.ensure_instance_or_alloc(this, Some(proto), ctx.mode);
    let JsValue::Object(id) = receiver else {
        unreachable!("ensure_instance_or_alloc returns an Object");
    };
    ctx.vm.get_object_mut(id).kind = ObjectKind::WasmMemory;
    ctx.vm.wasm_memory_storage.insert(
        id,
        WasmMemoryPayload {
            memory,
            buffer_id: None,
            view: None,
        },
    );
    Ok(receiver)
}

/// `Memory.prototype.buffer` getter — WASM JS API §5.3 IDL.
/// Returns `this.[[BufferObject]]`; in elidex this is the cached
/// `ArrayBuffer` wrapper aliasing the wasm linear memory, lazily
/// allocated on first access via [`array_buffer::create_wasm_backed_array_buffer`].
///
/// Per plan-memo DR-11 the wrapper is wrapper-identity-stable until
/// detach (elidex impl choice — IDL has no `[SameObject]`).  The
/// stashed [`WasmMemoryView`] (in
/// [`WasmMemoryPayload::view`]) carries the live alias used by the
/// `byte_io::*_with_routing` + extended `array_buffer_*` accessors
/// to route ArrayBuffer reads/writes through the underlying wasm
/// memory rather than the standard `body_data` path.
pub(super) fn native_wasm_memory_buffer_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_memory_this(ctx, this, "buffer")?;
    if let Some(cached) = ctx
        .vm
        .wasm_memory_storage
        .get(&id)
        .and_then(|p| p.buffer_id)
    {
        return Ok(JsValue::Object(cached));
    }

    // First-fire allocation — pull a fresh view + allocate a
    // wasm-backed ArrayBuffer wrapper (no `body_data` entry; routing
    // through `wasm_backed_buffers` takes over).
    let view = {
        let payload = ctx
            .vm
            .wasm_memory_storage
            .get(&id)
            .expect("brand-check guarantees storage entry exists");
        payload.memory.view()
    };
    let buf_id = array_buffer::create_wasm_backed_array_buffer(ctx.vm);
    // Stash view + buffer_id ⇒ wasm_backed_buffers entry — paired
    // writes per the coupling invariant.
    if let Some(payload) = ctx.vm.wasm_memory_storage.get_mut(&id) {
        payload.buffer_id = Some(buf_id);
        payload.view = Some(view);
    }
    ctx.vm.wasm_backed_buffers.insert(buf_id, id);
    Ok(JsValue::Object(buf_id))
}

/// `Memory.prototype.grow(delta)` — WASM JS API §5.3 IDL + algorithm
/// `refresh the Memory buffer` step 5.
///
/// Returns the pre-grow page count (per spec).  On
/// `Ok(GrowResult { buffer_handle_invalidated: true })` — which
/// `WasmMemory::grow` always returns per the F1 contract since
/// elidex MVP only handles fixed-length backing buffers — detach
/// the cached `[[BufferObject]]` via [`array_buffer::array_buffer_detach`]
/// (F3 surface) AND drop the stashed view + remove the
/// `wasm_backed_buffers` reverse-lookup entry to maintain the
/// coupling invariant.
pub(super) fn native_wasm_memory_grow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_memory_this(ctx, this, "grow")?;
    let delta_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let delta = coerce_uint32_pub(ctx, delta_arg)?;
    let mut memory = ctx
        .vm
        .wasm_memory_storage
        .get(&id)
        .expect("brand-check guarantees storage entry exists")
        .memory
        .clone();
    let grow_result = match memory.grow(delta) {
        Ok(r) => r,
        Err(e) => return Err(wasm_error_to_vm_error(ctx, &e)),
    };
    if grow_result.buffer_handle_invalidated {
        // §5.3 step 5.1 — `DetachArrayBuffer(buffer,
        // "WebAssembly.Memory")`.  Pair the F3 detach with the
        // coupling-invariant cleanup (drop view + remove
        // `wasm_backed_buffers` entry) so subsequent `.buffer` reads
        // allocate fresh.
        let cached_buffer_id = ctx
            .vm
            .wasm_memory_storage
            .get(&id)
            .and_then(|p| p.buffer_id);
        if let Some(buf_id) = cached_buffer_id {
            array_buffer::array_buffer_detach(ctx.vm, buf_id);
            ctx.vm.wasm_backed_buffers.remove(&buf_id);
        }
        if let Some(payload) = ctx.vm.wasm_memory_storage.get_mut(&id) {
            payload.buffer_id = None;
            payload.view = None;
        }
    }
    Ok(JsValue::Number(f64::from(grow_result.pre_pages)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_memory_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "WebAssembly.Memory.prototype.{method} called on non-Memory"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::WasmMemory) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "WebAssembly.Memory.prototype.{method} called on non-Memory"
        )))
    }
}

/// Coerce the `MemoryDescriptor` JS dict into engine-indep parts per
/// WASM JS API §5.3 IDL.  Returns `(initial, maximum)`.  Unknown
/// descriptor keys (incl. `shared` per `wasm-js-api-2-fork-threads`
/// — deferred `#11-wasm-threads`) are silently ignored per WebIDL
/// dictionary coerce.
fn coerce_memory_descriptor(
    ctx: &mut NativeContext<'_>,
    descriptor_arg: JsValue,
) -> Result<(u32, Option<u32>), VmError> {
    use super::super::super::value::PropertyKey;
    let JsValue::Object(dict) = descriptor_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'Memory' on 'WebAssembly': parameter 1 must be a MemoryDescriptor",
        ));
    };
    let initial_sid = ctx.vm.strings.intern("initial");
    let maximum_sid = ctx.vm.strings.intern("maximum");
    let initial_val = ctx.get_property_value(dict, PropertyKey::String(initial_sid))?;
    let initial = coerce_uint32_pub(ctx, initial_val)?;
    let maximum_val = ctx.get_property_value(dict, PropertyKey::String(maximum_sid))?;
    let maximum = if matches!(maximum_val, JsValue::Undefined) {
        None
    } else {
        Some(coerce_uint32_pub(ctx, maximum_val)?)
    };
    Ok((initial, maximum))
}
