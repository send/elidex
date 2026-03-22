//! `WebAssembly` global for boa — `WebAssembly.instantiate()` MVP.
//!
//! Phase 3.5 scope: `WebAssembly.instantiate(bufferSource | Module)`,
//! `WebAssembly.compile(bufferSource)`, and `WebAssembly.validate(bufferSource)`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use boa_engine::object::builtins::{JsArray, JsPromise};
use boa_engine::object::{IntegrityLevel, ObjectInitializer};
use boa_engine::property::{Attribute, PropertyDescriptorBuilder};
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsSymbol, JsValue, NativeFunction};
use elidex_wasm_runtime::{WasmError, WasmErrorKind, WasmInstance, WasmModule, WasmRuntime};

use crate::bridge::HostBridge;

/// Hidden property name for storing the module index on Module JS objects.
const MODULE_IDX_PROP: &str = "__wasm_module_idx__";

/// Maximum number of compiled modules stored in the module cache.
/// Prevents unbounded memory growth from repeated `compile()` calls.
const MAX_STORED_MODULES: usize = 1024;

/// Maximum allowed size for Wasm module bytes (256 MiB).
/// Prevents OOM from malicious length values.
const MAX_WASM_MODULE_BYTES: usize = 256 * 1024 * 1024;

/// Shared state for WebAssembly instances — stored in closures.
#[derive(Clone)]
struct WasmCaptures {
    runtime: Rc<WasmRuntime>,
    bridge: HostBridge,
    /// Compiled modules indexed by ID, shared across instantiate/compile.
    module_store: Rc<RefCell<HashMap<u64, WasmModule>>>,
    next_module_id: Rc<RefCell<u64>>,
}

impl_empty_trace!(WasmCaptures);

/// State for a single Wasm export function closure.
#[derive(Clone)]
struct ExportCaptures {
    instance: Rc<RefCell<WasmInstance>>,
    export_name: String,
    bridge: HostBridge,
}

impl_empty_trace!(ExportCaptures);

/// Register the `WebAssembly` global on the boa context.
///
/// Only registered if the bridge is available (always true in the browser engine).
pub fn register_wasm(ctx: &mut Context, bridge: &HostBridge) {
    let runtime = match WasmRuntime::new() {
        Ok(rt) => Rc::new(rt),
        Err(_) => return, // wasmtime engine creation failed — skip registration
    };

    let captures = WasmCaptures {
        runtime,
        bridge: bridge.clone(),
        module_store: Rc::new(RefCell::new(HashMap::new())),
        next_module_id: Rc::new(RefCell::new(0)),
    };

    // WebAssembly.instantiate(bufferSource | Module) -> Promise<{module, instance}>
    let instantiate_fn = NativeFunction::from_copy_closure_with_captures(
        move |_this, args, captures, ctx| wasm_instantiate(args, captures, ctx),
        captures.clone(),
    );

    // WebAssembly.compile(bufferSource) -> Promise<Module>
    let compile_fn = NativeFunction::from_copy_closure_with_captures(
        move |_this, args, captures, ctx| wasm_compile(args, captures, ctx),
        captures.clone(),
    );

    // WebAssembly.validate(bufferSource) -> boolean
    let validate_fn = NativeFunction::from_copy_closure_with_captures(
        move |_this, args, captures, ctx| wasm_validate(args, captures, ctx),
        captures,
    );

    let wasm_obj = ObjectInitializer::new(ctx)
        .function(instantiate_fn, js_string!("instantiate"), 1)
        .function(compile_fn, js_string!("compile"), 1)
        .function(validate_fn, js_string!("validate"), 1)
        .property(
            JsSymbol::to_string_tag(),
            js_string!("WebAssembly"),
            Attribute::READONLY | Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
        )
        .build();

    // Per spec §2, WebAssembly is writable + configurable but NOT enumerable.
    ctx.global_object()
        .define_property_or_throw(
            js_string!("WebAssembly"),
            PropertyDescriptorBuilder::new()
                .value(wasm_obj)
                .writable(true)
                .enumerable(false)
                .configurable(true)
                .build(),
            ctx,
        )
        .expect("failed to register WebAssembly");
}

/// Convert a `WasmError` to the appropriate boa `JsNativeError`.
///
/// Maps `WasmErrorKind` to the closest standard JS error type:
/// - `CompileError`  -> `TypeError` (compilation/validation failure)
/// - `LinkError`     -> `TypeError` (import resolution failure)
/// - `RuntimeError`  -> `RangeError` (trap, stack overflow, OOM)
///
/// The error message is prefixed with the WebAssembly error class name
/// (e.g. "`CompileError`: ...") so JS code can pattern-match on the name.
/// Boa 0.21 does not support custom error subclasses (WebAssembly.CompileError
/// etc. are not standard JS built-ins); once boa adds WebIDL/custom error
/// class support, these can be replaced with proper subclass instances.
fn wasm_error_to_js(e: &WasmError) -> JsNativeError {
    match e.kind {
        WasmErrorKind::Compile => {
            JsNativeError::typ().with_message(format!("CompileError: {}", e.message()))
        }
        WasmErrorKind::Link => {
            JsNativeError::typ().with_message(format!("LinkError: {}", e.message()))
        }
        WasmErrorKind::Runtime => {
            JsNativeError::range().with_message(format!("RuntimeError: {}", e.message()))
        }
    }
}

/// Return a rejected promise wrapping a `WasmError`.
///
/// Shared by `compile()` and `instantiate()` for spec-compliant error handling:
/// `CompileError`/`LinkError` are returned as rejected promises, not thrown.
#[allow(clippy::unnecessary_wraps)] // JsResult return needed at call sites.
fn reject_wasm_error(e: &WasmError, ctx: &mut Context) -> JsResult<JsValue> {
    let promise = JsPromise::reject(wasm_error_to_js(e), ctx);
    Ok(promise.into())
}

/// Store a compiled module and return its ID.
///
/// Evicts the oldest entry when the store exceeds `MAX_STORED_MODULES`.
fn store_module(captures: &WasmCaptures, module: WasmModule) -> u64 {
    let mut id = captures.next_module_id.borrow_mut();
    let module_id = *id;
    *id += 1;
    let mut store = captures.module_store.borrow_mut();
    // Evict oldest entry if at capacity (IDs are monotonic, so min key = oldest).
    if store.len() >= MAX_STORED_MODULES {
        if let Some(&oldest) = store.keys().min() {
            store.remove(&oldest);
        }
    }
    store.insert(module_id, module);
    module_id
}

/// Build a Module JS object with a hidden index property.
fn build_module_object(module_id: u64, ctx: &mut Context) -> JsValue {
    #[allow(clippy::cast_precision_loss)]
    let id_val = JsValue::from(module_id as f64);
    ObjectInitializer::new(ctx)
        .property(
            js_string!(MODULE_IDX_PROP),
            id_val,
            Attribute::empty(), // non-enumerable, non-configurable (hidden)
        )
        .build()
        .into()
}

/// Try to extract a stored `WasmModule` from a JS Module object.
fn extract_stored_module(
    arg: &JsValue,
    captures: &WasmCaptures,
    ctx: &mut Context,
) -> Option<WasmModule> {
    let obj = arg.as_object()?;
    let idx_val = obj.get(js_string!(MODULE_IDX_PROP), ctx).ok()?;
    let idx = idx_val.as_number()?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let module_id = idx as u64;
    captures.module_store.borrow().get(&module_id).cloned()
}

/// Extract Wasm bytes from a JS argument.
///
/// Accepts an array-like object (e.g. `[0x00, 0x61, ...]` or `Uint8Array`).
fn extract_wasm_bytes(args: &[JsValue], ctx: &mut Context) -> JsResult<Vec<u8>> {
    let arg = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("WebAssembly: argument 0 is required"))?;

    let obj = arg.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("WebAssembly: argument must be an array-like object")
    })?;

    // Read "length" property and iterate.
    let length_val = obj.get(js_string!("length"), ctx)?;
    let length = length_val.to_number(ctx)?;
    if !length.is_finite() || length < 0.0 {
        return Err(JsNativeError::typ()
            .with_message("WebAssembly: argument has invalid length")
            .into());
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let len = length as usize;
    if len > MAX_WASM_MODULE_BYTES {
        return Err(JsNativeError::range()
            .with_message(format!(
                "WebAssembly: module exceeds maximum size ({MAX_WASM_MODULE_BYTES} bytes)"
            ))
            .into());
    }

    let mut bytes = Vec::with_capacity(len);
    for i in 0..len {
        let val = obj.get(i as u32, ctx)?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let byte = val.to_number(ctx)? as u8;
        bytes.push(byte);
    }

    Ok(bytes)
}

/// `WebAssembly.instantiate(bufferSource | Module)` implementation.
///
/// Per JS API spec §4.5.4:
/// - `instantiate(bufferSource)` → `Promise<{module, instance}>`
/// - `instantiate(Module)`       → `Promise<Instance>`
///
/// `TypeError` for invalid arguments is thrown synchronously.
/// `CompileError`/`LinkError` are returned as rejected promises (per spec).
fn wasm_instantiate(
    args: &[JsValue],
    captures: &WasmCaptures,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    // Check if the first argument is a previously compiled Module object.
    let stored = args
        .first()
        .and_then(|a| extract_stored_module(a, captures, ctx));
    let is_module_arg = stored.is_some();

    let module = if let Some(m) = stored {
        m
    } else {
        // Treat as bufferSource — extract bytes and compile.
        // Argument validation — TypeError thrown synchronously (per spec).
        let bytes = extract_wasm_bytes(args, ctx)?;
        // CompileError — returned as rejected promise (per spec §4.5.4).
        match captures.runtime.compile(&bytes) {
            Ok(m) => m,
            Err(e) => return reject_wasm_error(&e, ctx),
        }
    };

    // LinkError — returned as rejected promise (per spec §4.5.4).
    let instance = match captures.runtime.instantiate(&module) {
        Ok(i) => i,
        Err(e) => return reject_wasm_error(&e, ctx),
    };

    let instance_rc = Rc::new(RefCell::new(instance));

    // Build the exports object by iterating wasmtime exports.
    let exports_obj = build_exports_object(&instance_rc, &captures.bridge, ctx)?;

    // Build the instance object.
    let instance_obj: JsValue = ObjectInitializer::new(ctx)
        .property(
            js_string!("exports"),
            exports_obj,
            Attribute::READONLY | Attribute::ENUMERABLE,
        )
        .build()
        .into();

    let result = if is_module_arg {
        // §4.5.4 overload: instantiate(Module) → Promise<Instance>
        instance_obj
    } else {
        // §4.5.4 overload: instantiate(bufferSource) → Promise<{module, instance}>
        let module_id = store_module(captures, module);
        let module_js = build_module_object(module_id, ctx);
        ObjectInitializer::new(ctx)
            .property(
                js_string!("module"),
                module_js,
                Attribute::READONLY | Attribute::ENUMERABLE,
            )
            .property(
                js_string!("instance"),
                instance_obj,
                Attribute::READONLY | Attribute::ENUMERABLE,
            )
            .build()
            .into()
    };

    // Return a resolved promise (synchronous in Phase 3.5).
    let promise = JsPromise::resolve(result, ctx);
    Ok(promise.into())
}

/// `WebAssembly.compile(bufferSource)` implementation.
///
/// Per spec §4.5.1: `TypeError` for invalid arguments is thrown synchronously.
/// `CompileError` is returned as a rejected promise.
fn wasm_compile(args: &[JsValue], captures: &WasmCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    // Argument validation — TypeError thrown synchronously (per spec).
    let bytes = extract_wasm_bytes(args, ctx)?;

    // Compile errors — returned as rejected promise (per spec §4.5.1).
    let module = match captures.runtime.compile(&bytes) {
        Ok(m) => m,
        Err(e) => return reject_wasm_error(&e, ctx),
    };

    let module_id = store_module(captures, module);
    let module_js = build_module_object(module_id, ctx);

    let promise = JsPromise::resolve(module_js, ctx);
    Ok(promise.into())
}

/// `WebAssembly.validate(bufferSource)` implementation.
///
/// Per JS API spec §4.5.2: returns `true` if the bytes form a valid Wasm module.
///
/// `TypeError` is thrown for non-`BufferSource` arguments (via `WebIDL` binding).
fn wasm_validate(
    args: &[JsValue],
    captures: &WasmCaptures,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let bytes = extract_wasm_bytes(args, ctx)?;
    Ok(JsValue::from(captures.runtime.validate(&bytes)))
}

/// Build a JS object mapping exported names to callable wrappers / memory objects.
///
/// The returned exports object is frozen per WebAssembly JS API spec §4.5.4.
fn build_exports_object(
    instance_rc: &Rc<RefCell<WasmInstance>>,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    // Categorize exports in a single borrow.
    let (func_exports, memory_exports) = {
        let mut inst = instance_rc.borrow_mut();
        let names = inst.export_names();
        let mut funcs: Vec<(String, usize)> = Vec::new();
        let mut memories = Vec::new();
        for name in names {
            if let Some(param_count) = inst.export_param_types(&name).map(|types| types.len()) {
                funcs.push((name, param_count));
            } else if let Some(size) = inst.memory_byte_size(&name) {
                memories.push((name, size));
            }
            // Known limitation: Wasm globals and tables are not exposed as
            // exports. These are uncommon in practice (most modules export
            // only functions and memory). Supporting them requires mapping
            // wasmtime Global/Table types to JS wrapper objects with get/set
            // semantics, which is deferred to a future milestone.
        }
        (funcs, memories)
    };

    // Build memory wrapper objects first (before the exports builder borrows ctx).
    let memory_objs: Vec<(String, JsValue)> = memory_exports
        .into_iter()
        .map(|(name, byte_size)| {
            #[allow(clippy::cast_precision_loss)]
            let buffer_obj = ObjectInitializer::new(ctx)
                .property(
                    js_string!("byteLength"),
                    JsValue::from(byte_size as f64),
                    Attribute::READONLY | Attribute::ENUMERABLE,
                )
                .build();
            let memory_obj = ObjectInitializer::new(ctx)
                .property(
                    js_string!("buffer"),
                    buffer_obj,
                    Attribute::READONLY | Attribute::ENUMERABLE,
                )
                .build();
            (name, JsValue::from(memory_obj))
        })
        .collect();

    let mut builder = ObjectInitializer::new(ctx);

    // Function exports.
    for (name, param_count) in func_exports {
        let captures = ExportCaptures {
            instance: instance_rc.clone(),
            export_name: name.clone(),
            bridge: bridge.clone(),
        };

        let func = NativeFunction::from_copy_closure_with_captures(
            move |_this, args, captures, ctx| call_wasm_export(args, captures, ctx),
            captures,
        );

        builder.function(func, js_string!(name.as_str()), param_count);
    }

    // Memory exports — exposed as WebAssembly.Memory-like objects.
    for (name, memory_obj) in memory_objs {
        builder.property(
            js_string!(name.as_str()),
            memory_obj,
            Attribute::READONLY | Attribute::ENUMERABLE,
        );
    }

    let exports_obj = builder.build();

    // Freeze the exports object per WebAssembly JS API spec.
    exports_obj.set_integrity_level(IntegrityLevel::Frozen, ctx)?;

    Ok(exports_obj.into())
}

/// ECMAScript §7.1.6 `ToInt32`: modular conversion of f64 to i32.
///
/// Unlike Rust's saturating `n as i32`, this wraps via 2^32 modular arithmetic.
/// Example: `4294967296.0` → `0`, `2147483648.0` → `-2147483648`.
#[allow(clippy::cast_possible_truncation)]
fn to_int32(n: f64) -> i32 {
    if !n.is_finite() || n == 0.0 {
        return 0;
    }
    let n = n.trunc(); // step 3: truncate toward zero
    let n = n.rem_euclid(4_294_967_296.0); // step 4: modulo 2^32
                                           // step 5: if >= 2^31, subtract 2^32
    if n >= 2_147_483_648.0 {
        (n - 4_294_967_296.0) as i32
    } else {
        n as i32
    }
}

/// Convert a boa `JsValue` to a wasmtime `Val` using the expected type from
/// the function signature.
///
/// Uses JS `ToNumber` coercion (§7.1.4) so that strings like `"42"` and
/// booleans are correctly converted, matching the WebAssembly JS API spec.
fn js_to_wasm_val(
    arg: &JsValue,
    expected: Option<&wasmtime::ValType>,
    ctx: &mut Context,
) -> JsResult<wasmtime::Val> {
    let n = arg.to_number(ctx)?;
    Ok(match expected {
        Some(wasmtime::ValType::I32) => wasmtime::Val::I32(to_int32(n)),
        Some(wasmtime::ValType::I64) =>
        {
            #[allow(clippy::cast_possible_truncation)]
            wasmtime::Val::I64(n as i64)
        }
        Some(wasmtime::ValType::F32) =>
        {
            #[allow(clippy::cast_possible_truncation)]
            wasmtime::Val::F32((n as f32).to_bits())
        }
        Some(wasmtime::ValType::F64) => wasmtime::Val::F64(n.to_bits()),
        _ => {
            // Fallback heuristic for unknown/unsupported types.
            if n.fract() == 0.0 && n >= f64::from(i32::MIN) && n <= f64::from(i32::MAX) {
                wasmtime::Val::I32(to_int32(n))
            } else {
                wasmtime::Val::F64(n.to_bits())
            }
        }
    })
}

/// Call a Wasm export function from JS.
fn call_wasm_export(
    args: &[JsValue],
    captures: &ExportCaptures,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    // Get parameter types from the function signature (separate borrow).
    let param_types = {
        let mut inst = captures.instance.borrow_mut();
        inst.export_param_types(&captures.export_name)
    };

    // Per JS API §4.4.7 steps 6-7: missing args → undefined, extra args ignored.
    let param_count = param_types.as_ref().map_or(0, Vec::len);
    let undefined = JsValue::undefined();
    let wasm_args: Vec<wasmtime::Val> = (0..param_count)
        .map(|i| {
            let arg = args.get(i).unwrap_or(&undefined);
            let expected = param_types.as_ref().and_then(|types| types.get(i));
            js_to_wasm_val(arg, expected, ctx)
        })
        .collect::<JsResult<Vec<_>>>()?;

    // Call the export through the bridge (which is already bound during JS eval).
    let results = captures.bridge.with(|session, dom| {
        let doc = captures.bridge.document_entity();
        let mut inst = captures.instance.borrow_mut();
        inst.call_export(&captures.export_name, &wasm_args, session, dom, doc)
    });

    match results {
        Ok(vals) => Ok(wasm_vals_to_js(&vals, ctx)),
        Err(ref e) => Err(wasm_error_to_js(e).into()),
    }
}

/// Convert wasmtime result `Val`s to a boa `JsValue`.
///
/// Per WebAssembly JS API spec §4.4.7:
/// - 0 results → `undefined`
/// - 1 result  → `ToJSValue(val)`
/// - 2+ results → JS Array of converted values (multi-value proposal)
fn wasm_vals_to_js(vals: &[wasmtime::Val], ctx: &mut Context) -> JsValue {
    match vals.len() {
        0 => JsValue::undefined(),
        1 => wasm_val_to_js(&vals[0]),
        _ => {
            let array = JsArray::new(ctx);
            for (i, val) in vals.iter().enumerate() {
                let _ = array.set(i as u32, wasm_val_to_js(val), false, ctx);
            }
            array.into()
        }
    }
}

/// Convert a single wasmtime `Val` to a boa `JsValue`.
///
/// NOTE: `I64` values are converted to `f64`, losing precision beyond 2^53.
/// The WebAssembly JS API spec requires `BigInt` for I64 values, but boa 0.20
/// does not support `BigInt`. TODO: use `BigInt` when boa adds support.
fn wasm_val_to_js(val: &wasmtime::Val) -> JsValue {
    match val {
        wasmtime::Val::I32(n) => JsValue::from(*n),
        #[allow(clippy::cast_precision_loss)]
        wasmtime::Val::I64(n) => JsValue::from(*n as f64),
        wasmtime::Val::F32(n) => JsValue::from(f64::from(f32::from_bits(*n))),
        wasmtime::Val::F64(n) => JsValue::from(f64::from_bits(*n)),
        _ => JsValue::undefined(),
    }
}
