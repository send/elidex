//! Host functions linking Wasm modules to the elidex DOM via `DomHandlerRegistry`.
//!
//! All host functions live in the `"elidex"` namespace. String arguments use
//! `(ptr, len)` pairs referencing Wasm linear memory. Entity handles are `i64`
//! (0 = null). String return values use packed `i64`: `(ptr << 32) | len`,
//! allocated via the guest's exported `__alloc(len: i32) -> i32`.

use elidex_ecs::Entity;
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, JsObjectRef};
use wasmtime::{AsContextMut, Caller, Linker};

use crate::host_state::HostState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Packed i64 sentinel for empty strings: ptr=1, len=0.
/// Distinguishes empty string from null (0).
const EMPTY_STRING_PACKED: i64 = 1_i64 << 32;

// ---------------------------------------------------------------------------
// Memory helpers
// ---------------------------------------------------------------------------

/// Read a UTF-8 string from Wasm linear memory, looking up the `"memory"` export.
fn read_wasm_str(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
    if len < 0 || ptr < 0 {
        return None;
    }
    let memory = get_memory(caller)?;
    #[allow(clippy::cast_sign_loss)]
    let (ptr, len) = (ptr as usize, len as usize);
    let data = memory.data(&*caller);
    if ptr.checked_add(len)? > data.len() {
        return None;
    }
    String::from_utf8(data[ptr..ptr + len].to_vec()).ok()
}

/// Write a string into Wasm linear memory via the guest's `__alloc` export.
/// Returns the packed `i64` value `(ptr << 32) | len`, or 0 on failure.
fn write_string_to_wasm(caller: &mut Caller<'_, HostState>, s: &str) -> i64 {
    let len = s.len();
    if len == 0 {
        return EMPTY_STRING_PACKED;
    }
    // Guard: string length must fit in i32 for the Wasm __alloc ABI.
    if len > i32::MAX as usize {
        return 0;
    }

    let Some(wasmtime::Extern::Func(alloc_fn)) = caller.get_export("__alloc") else {
        return 0;
    };

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let len_i32 = len as i32;
    let mut results = [wasmtime::Val::I32(0)];
    if alloc_fn
        .call(
            caller.as_context_mut(),
            &[wasmtime::Val::I32(len_i32)],
            &mut results,
        )
        .is_err()
    {
        return 0;
    }

    let wasmtime::Val::I32(ptr) = results[0] else {
        return 0;
    };
    if ptr <= 0 {
        return 0;
    }

    let Some(wasmtime::Extern::Memory(memory)) = caller.get_export("memory") else {
        return 0;
    };
    #[allow(clippy::cast_sign_loss)]
    let ptr_usize = ptr as usize;
    let data = memory.data_mut(caller.as_context_mut());
    let Some(end) = ptr_usize.checked_add(len) else {
        return 0;
    };
    if end > data.len() {
        return 0;
    }
    data[ptr_usize..ptr_usize + len].copy_from_slice(s.as_bytes());

    #[allow(clippy::cast_possible_wrap)]
    {
        (i64::from(ptr) << 32) | (len as i64)
    }
}

// ---------------------------------------------------------------------------
// Entity conversion
// ---------------------------------------------------------------------------

/// Convert an `i64` entity handle to an `Entity`.
/// Returns `None` for 0 (null entity).
fn entity_from_i64(val: i64) -> Option<Entity> {
    if val == 0 {
        return None;
    }
    #[allow(clippy::cast_sign_loss)]
    Entity::from_bits(val as u64)
}

/// Convert an `Entity` to an `i64` handle.
#[allow(clippy::cast_possible_wrap)]
fn entity_to_i64(entity: Entity) -> i64 {
    entity.to_bits().get() as i64
}

// ---------------------------------------------------------------------------
// Handler dispatch helpers
// ---------------------------------------------------------------------------

/// Resolve a `JsValue::ObjectRef` returned by a handler back to an entity i64.
///
/// The handler returns `JsValue::ObjectRef(ref_raw)` which is a `JsObjectRef`.
/// We look up the entity in the identity map and return entity bits as i64.
fn objref_to_entity_i64(caller: &mut Caller<'_, HostState>, ref_raw: u64) -> i64 {
    let obj_ref = JsObjectRef::from_raw(ref_raw);
    let state = caller.data_mut();
    state
        .with(|session, _dom| session.identity_map().get(obj_ref))
        .map_or(0, |(e, _)| entity_to_i64(e))
}

/// Convert an entity i64 from Wasm to a `JsValue::ObjectRef` for handler args.
///
/// Looks up or creates a `JsObjectRef` in the identity map for the entity.
fn entity_i64_to_objref(caller: &mut Caller<'_, HostState>, val: i64) -> Option<JsValue> {
    let entity = entity_from_i64(val)?;
    let state = caller.data_mut();
    // Validate entity exists in the ECS world before polluting the identity map.
    let obj_ref = state.with(|session, dom| {
        if !dom.contains(entity) {
            return None;
        }
        Some(session.get_or_create_wrapper(entity, ComponentKind::Element))
    })?;
    Some(JsValue::ObjectRef(obj_ref.to_raw()))
}

/// Resolve and invoke a DOM handler, returning the result `JsValue`.
///
/// Shared core of `invoke_entity_handler`, `invoke_string_handler`, and
/// `invoke_void_handler`. Callers interpret the return value differently.
///
/// Validates that the entity exists in the ECS world before dispatching.
/// This prevents fabricated entity handles from reaching DOM handlers — important
/// for future Wasm sandboxing where modules should not access arbitrary entities.
fn invoke_handler(
    caller: &mut Caller<'_, HostState>,
    handler_name: &str,
    entity: Entity,
    args: &[JsValue],
) -> Option<JsValue> {
    let state = caller.data_mut();
    let registry = state.dom_registry.clone();
    let handler = registry.resolve(handler_name)?;
    state
        .with(|session, dom| {
            if !dom.contains(entity) {
                return Err(elidex_script_session::DomApiError::not_found(
                    "entity does not exist",
                ));
            }
            handler.invoke(entity, args, session, dom)
        })
        .ok()
}

/// Invoke a DOM handler that returns an entity (`ObjectRef`).
/// Translates the result back to entity bits as `i64`.
///
/// ## Error reporting limitation
///
/// All host functions return 0 on error with no way for Wasm guests to
/// distinguish "entity not found" from "invalid entity handle" or "UTF-8
/// decode error". Improving this requires one of:
///
/// - **errno-style global**: A Wasm global (e.g. `__elidex_errno`) set by
///   each host function before returning 0. The guest reads it after each call.
///   Downside: requires the guest to export a mutable global, adding ABI
///   complexity.
///
/// - **Multi-value return**: Return `(result: i64, error_code: i32)` from
///   each host function. Clean but requires updating all function signatures
///   and guest-side call conventions.
///
/// Both approaches require coordinated changes to the guest SDK and are
/// deferred until the elidex Wasm SDK stabilizes.
fn invoke_entity_handler(
    caller: &mut Caller<'_, HostState>,
    handler_name: &str,
    entity: Entity,
    args: &[JsValue],
) -> i64 {
    match invoke_handler(caller, handler_name, entity, args) {
        Some(JsValue::ObjectRef(r)) => objref_to_entity_i64(caller, r),
        _ => 0,
    }
}

/// Invoke a DOM handler that returns a string.
fn invoke_string_handler(
    caller: &mut Caller<'_, HostState>,
    handler_name: &str,
    entity: Entity,
    args: &[JsValue],
) -> i64 {
    match invoke_handler(caller, handler_name, entity, args) {
        Some(JsValue::String(s)) => write_string_to_wasm(caller, &s),
        _ => 0,
    }
}

/// Invoke a DOM handler that returns nothing meaningful.
fn invoke_void_handler(
    caller: &mut Caller<'_, HostState>,
    handler_name: &str,
    entity: Entity,
    args: &[JsValue],
) {
    invoke_handler(caller, handler_name, entity, args);
}

/// Get the Wasm linear memory export.
fn get_memory(caller: &mut Caller<'_, HostState>) -> Option<wasmtime::Memory> {
    match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(m)) => Some(m),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all host functions in the `"elidex"` namespace.
pub fn register_host_functions(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    register_query_functions(linker)?;
    register_creation_functions(linker)?;
    register_tree_functions(linker)?;
    register_attribute_functions(linker)?;
    register_content_functions(linker)?;
    Ok(())
}

/// Document query: `get_document`, `query_selector`, `get_element_by_id`.
fn register_query_functions(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    // get_document() -> i64
    linker.func_wrap(
        "elidex",
        "get_document",
        |caller: Caller<'_, HostState>| -> i64 { entity_to_i64(caller.data().document_entity()) },
    )?;

    // query_selector(entity: i64, sel_ptr: i32, sel_len: i32) -> i64
    linker.func_wrap(
        "elidex",
        "query_selector",
        |mut caller: Caller<'_, HostState>, entity: i64, sel_ptr: i32, sel_len: i32| -> i64 {
            let Some(entity) = entity_from_i64(entity) else {
                return 0;
            };
            let Some(sel) = read_wasm_str(&mut caller, sel_ptr, sel_len) else {
                return 0;
            };
            invoke_entity_handler(
                &mut caller,
                "querySelector",
                entity,
                &[JsValue::String(sel)],
            )
        },
    )?;

    // get_element_by_id(entity: i64, id_ptr: i32, id_len: i32) -> i64
    linker.func_wrap(
        "elidex",
        "get_element_by_id",
        |mut caller: Caller<'_, HostState>, entity: i64, id_ptr: i32, id_len: i32| -> i64 {
            let Some(entity) = entity_from_i64(entity) else {
                return 0;
            };
            let Some(id) = read_wasm_str(&mut caller, id_ptr, id_len) else {
                return 0;
            };
            invoke_entity_handler(
                &mut caller,
                "getElementById",
                entity,
                &[JsValue::String(id)],
            )
        },
    )?;

    Ok(())
}

/// Element/text creation: `create_element`, `create_text_node`.
fn register_creation_functions(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    // create_element(tag_ptr: i32, tag_len: i32) -> i64
    linker.func_wrap(
        "elidex",
        "create_element",
        |mut caller: Caller<'_, HostState>, tag_ptr: i32, tag_len: i32| -> i64 {
            let Some(tag) = read_wasm_str(&mut caller, tag_ptr, tag_len) else {
                return 0;
            };
            let doc = caller.data().document_entity();
            invoke_entity_handler(&mut caller, "createElement", doc, &[JsValue::String(tag)])
        },
    )?;

    // create_text_node(text_ptr: i32, text_len: i32) -> i64
    linker.func_wrap(
        "elidex",
        "create_text_node",
        |mut caller: Caller<'_, HostState>, text_ptr: i32, text_len: i32| -> i64 {
            let Some(text) = read_wasm_str(&mut caller, text_ptr, text_len) else {
                return 0;
            };
            let doc = caller.data().document_entity();
            invoke_entity_handler(&mut caller, "createTextNode", doc, &[JsValue::String(text)])
        },
    )?;

    Ok(())
}

/// Tree mutation: `append_child`, `remove_child`.
fn register_tree_functions(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    // append_child(parent: i64, child: i64)
    linker.func_wrap(
        "elidex",
        "append_child",
        |mut caller: Caller<'_, HostState>, parent: i64, child: i64| {
            let Some(parent) = entity_from_i64(parent) else {
                return;
            };
            let Some(child_ref) = entity_i64_to_objref(&mut caller, child) else {
                return;
            };
            invoke_void_handler(&mut caller, "appendChild", parent, &[child_ref]);
        },
    )?;

    // remove_child(parent: i64, child: i64)
    linker.func_wrap(
        "elidex",
        "remove_child",
        |mut caller: Caller<'_, HostState>, parent: i64, child: i64| {
            let Some(parent) = entity_from_i64(parent) else {
                return;
            };
            let Some(child_ref) = entity_i64_to_objref(&mut caller, child) else {
                return;
            };
            invoke_void_handler(&mut caller, "removeChild", parent, &[child_ref]);
        },
    )?;

    Ok(())
}

/// Attribute access: `set_attribute`, `get_attribute`.
fn register_attribute_functions(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    // set_attribute(entity: i64, name_ptr: i32, name_len: i32, val_ptr: i32, val_len: i32)
    linker.func_wrap(
        "elidex",
        "set_attribute",
        |mut caller: Caller<'_, HostState>,
         entity: i64,
         name_ptr: i32,
         name_len: i32,
         val_ptr: i32,
         val_len: i32| {
            let Some(entity) = entity_from_i64(entity) else {
                return;
            };
            let Some(name) = read_wasm_str(&mut caller, name_ptr, name_len) else {
                return;
            };
            let Some(val) = read_wasm_str(&mut caller, val_ptr, val_len) else {
                return;
            };
            invoke_void_handler(
                &mut caller,
                "setAttribute",
                entity,
                &[JsValue::String(name), JsValue::String(val)],
            );
        },
    )?;

    // get_attribute(entity: i64, name_ptr: i32, name_len: i32) -> i64
    linker.func_wrap(
        "elidex",
        "get_attribute",
        |mut caller: Caller<'_, HostState>, entity: i64, name_ptr: i32, name_len: i32| -> i64 {
            let Some(entity) = entity_from_i64(entity) else {
                return 0;
            };
            let Some(name) = read_wasm_str(&mut caller, name_ptr, name_len) else {
                return 0;
            };
            invoke_string_handler(
                &mut caller,
                "getAttribute",
                entity,
                &[JsValue::String(name)],
            )
        },
    )?;

    Ok(())
}

/// Content and style: `set/get_text_content`, `style_set_property`.
fn register_content_functions(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    // set_text_content(entity: i64, text_ptr: i32, text_len: i32)
    linker.func_wrap(
        "elidex",
        "set_text_content",
        |mut caller: Caller<'_, HostState>, entity: i64, text_ptr: i32, text_len: i32| {
            let Some(entity) = entity_from_i64(entity) else {
                return;
            };
            let Some(text) = read_wasm_str(&mut caller, text_ptr, text_len) else {
                return;
            };
            invoke_void_handler(
                &mut caller,
                "textContent.set",
                entity,
                &[JsValue::String(text)],
            );
        },
    )?;

    // get_text_content(entity: i64) -> i64
    linker.func_wrap(
        "elidex",
        "get_text_content",
        |mut caller: Caller<'_, HostState>, entity: i64| -> i64 {
            let Some(entity) = entity_from_i64(entity) else {
                return 0;
            };
            invoke_string_handler(&mut caller, "textContent.get", entity, &[])
        },
    )?;

    // style_set_property(entity: i64, name_ptr: i32, name_len: i32, val_ptr: i32, val_len: i32)
    linker.func_wrap(
        "elidex",
        "style_set_property",
        |mut caller: Caller<'_, HostState>,
         entity: i64,
         name_ptr: i32,
         name_len: i32,
         val_ptr: i32,
         val_len: i32| {
            let Some(entity) = entity_from_i64(entity) else {
                return;
            };
            let Some(name) = read_wasm_str(&mut caller, name_ptr, name_len) else {
                return;
            };
            let Some(val) = read_wasm_str(&mut caller, val_ptr, val_len) else {
                return;
            };
            invoke_void_handler(
                &mut caller,
                "style.setProperty",
                entity,
                &[JsValue::String(name), JsValue::String(val)],
            );
        },
    )?;

    Ok(())
}
