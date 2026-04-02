# Architecture: Script (session, JS/boa, WASM)

## elidex-script-session

- **SessionCore**: Owns `IdentityMap` + `Vec<Mutation>`. `record_mutation()` buffers, `flush()` applies all via `apply_mutation()`. `get_or_create_wrapper()` for identity mapping, `release()` for cleanup.
- **IdentityMap**: Bidirectional `(Entity, ComponentKind) ↔ JsObjectRef`. Monotonic counter for unique refs. `get_or_create()` is idempotent. `release()` single ref, `release_entity()` all refs for an entity.
- **Mutation enum**: `AppendChild`, `InsertBefore`, `RemoveChild`, `ReplaceChild`, `SetAttribute`, `RemoveAttribute`, `SetTextContent`, `SetInnerHtml` (stub), `SetInlineStyle`, `RemoveInlineStyle`, `InsertCssRule`/`DeleteCssRule` (stubs).
- **apply_mutation()**: Delegates tree ops to `EcsDom`, attribute/style ops via `world_mut()`. `SetInlineStyle` auto-inserts `InlineStyle` component if missing. Returns `Option<MutationRecord>`.
- **DomApiHandler / CssomApiHandler traits**: `Send + Sync`, `method_name()`, `spec_level()` (default Living/Standard), `invoke(this, args, session, dom) -> Result<JsValue, DomApiError>`.
- **Types**: `JsObjectRef(u64)`, `ComponentKind` enum (Element/Style/ClassList/Attributes/Dataset/ChildNodes), `DomApiError` + `DomApiErrorKind` (NotFoundError/HierarchyRequestError/InvalidStateError/SyntaxError/TypeError/Other).
- **Event dispatch**: `DispatchEvent` with `composed: bool` (default true) and `original_target: Option<Entity>`. `build_propagation_path(dom, target, composed)` — non-composed events stop at `ShadowRoot`. Event retargeting: shadow-internal targets → shadow host for outside listeners (slotted elements exempt).
- **ScriptEngine trait**: `eval()`, `dispatch_event()`, `drain_timers()` — engine-agnostic interface. Navigation state methods intentionally excluded (engine-specific).
- **EvalResult**: Moved from `elidex-js-boa` to `elidex-script-session` (canonical location). Re-exported from `elidex-js-boa` for compatibility.

## elidex-js-boa

- **JsRuntime**: Owns boa `Context`, `HostBridge`, `ConsoleOutput`, `TimerQueueHandle`. `eval()` binds bridge, evaluates source, unbinds. `drain_timers()` runs ready timer callbacks.
- **HostBridge**: `Rc<RefCell<HostBridgeInner>>` with raw pointers to `SessionCore`/`EcsDom`. `bind()`/`unbind()` bracket eval. `with(|session, dom| ...)` for native function access. `!Send` via `Rc`. JsObject cache for entity identity (`HashMap<JsObjectRef, JsObject>`).
- **globals/document.rs**: `register_document()` — querySelector, querySelectorAll (JsArray), getElementById, createElement, createTextNode, body accessor.
- **globals/element.rs**: `build_element_object()` — appendChild, removeChild, setAttribute, getAttribute, removeAttribute, textContent (accessor), innerHTML (getter), style (accessor → `create_style_object`), classList (accessor → `create_class_list_object`), attachShadow({mode}) (creates shadow root via EcsDom), shadowRoot (accessor: open→root, closed/none→null). Entity stored as f64 in hidden `__elidex_entity__` property.
- **globals/window.rs**: `register_window()` — `getComputedStyle(element)` returning computed style proxy. `create_style_object()` — setProperty/getPropertyValue/removeProperty.
- **globals/console.rs**: `register_console()` — log/error/warn. `ConsoleOutput` captures messages for testing.
- **globals/timers.rs**: `register_timers()` — setTimeout/setInterval/clearTimeout/clearInterval/requestAnimationFrame/cancelAnimationFrame. `TimerQueueHandle` wraps `Rc<RefCell<TimerQueue>>`.
- **timer_queue.rs**: `TimerQueue` with `BinaryHeap<Reverse<TimerEntry>>` min-heap. Min 1ms interval to prevent infinite loops. `drain_ready()` collects and re-schedules intervals.
- **script_extract.rs**: `extract_scripts()` — pre-order DOM walk collecting inline `<script>` text. External scripts (`src="..."`) logged and skipped.
- **value_conv.rs**: `to_boa()`/`from_boa()` bidirectional elidex↔boa JsValue conversion.
- **error_conv.rs**: `dom_error_to_js_error()` — DomApiErrorKind → boa JsNativeError.
- **boa 0.20 notes**: `ObjectInitializer` methods return `&mut Self`, accessors need `JsFunction` (via `to_js_function(&realm)`), `custom_trace!(this, mark, {body})` with 3 args, `from_copy_closure_with_captures` for safe closure registration.
- **globals/fetch.rs**: `register_fetch()` — `fetch(url, options?)` global. Blocking HTTP via `FetchHandle::send_blocking()`. Returns `JsPromise::resolve(Response)` or `JsPromise::reject(TypeError)`. Response object: ok/status/statusText/url/type/redirected/headers properties + `text()`/`json()`/`clone()` methods. `json()` uses boa `JSON.parse()` via global object. Headers object: `get()` (combines duplicates per Fetch spec), `has()`, `forEach()`. `set()`/`delete()` omitted (Response headers immutable).
- **run_jobs() integration**: `eval()` calls `ctx.run_jobs()` after evaluation (bridge still bound) to drain microtask queue. `dispatch_event()` similarly calls `ctx.run_jobs()` after dispatch loop. Enables `fetch().then(r => r.text())` chains.
- **globals/canvas.rs**: `create_context2d_object()` — CanvasRenderingContext2D JS object with drawing methods delegating to `Canvas2dContext` in `HostBridge`. `sync_canvas_to_image_data()` syncs pixels to ECS `ImageData` after each draw. `extract_entity_bits()` reads entity from hidden property.
- **HostBridge canvas support**: `canvas_contexts: HashMap<u64, Canvas2dContext>` in `HostBridgeInner`. `ensure_canvas_context(entity_bits, width, height)` creates context, `with_canvas(entity_bits, f)` accesses it.
- **globals/element.rs**: `getContext("2d")` on `<canvas>` elements — reads width/height attributes (defaults 300×150), creates Canvas2dContext via bridge, returns cached context2d JS object.
- **globals/wasm.rs**: `register_wasm()` — `WebAssembly.instantiate(bufferSource)` / `WebAssembly.compile(bufferSource)`. `WasmRuntime` + `WasmInstance` stored in `Rc<RefCell>` closures. Export functions wrapped as JS callables via `ExportCaptures`. `extract_wasm_bytes()` reads array-like objects.
- **impl ScriptEngine for JsRuntime**: Delegates to concrete methods.
- **CookieStore API**: `globals/cookie_store.rs`. `get`/`getAll` return full `CookieListItem` (7 props, `READONLY`). `set` accepts `(name, value)` or options object. `delete` uses `Max-Age=0`. Registered in Window + SW contexts.
- **Response builder**: `fetch/mod.rs::build_response_from_parts(ResponseParts)` shared by fetch + cache. Provides `text()`/`json()`/`clone()`.
- **cached_entry_to_response**: Delegates to `build_response_from_parts`. `response.url` uses final URL from `response_url_list`.
- **ResponseType enum**: `entry.rs` — `Basic`/`Cors`/`Default`/`Error`/`Opaque`/`OpaqueRedirect`. `from_str_lossy` is case-insensitive.
- **HostBridge**: `client_id()` returns UUID v4. `enable_sw_messages()` for startMessages(). `cookie_details_for_script()` for CookieStore API.
- **Dependencies**: boa_engine 0.20 (annex-b), boa_gc 0.20, elidex-net, elidex-navigation, elidex-web-canvas, url, bytes.

## elidex-wasm-runtime

- **WasmRuntime**: Owns wasmtime `Engine` + `Linker<HostState>` + `Arc<DomHandlerRegistry>` / `Arc<CssomHandlerRegistry>`. `compile(bytes)` → `WasmModule`, `instantiate(module)` → `WasmInstance`.
- **WasmInstance**: Owns `Store<HostState>` + `Instance`. `call_export(name, args, session, dom, doc)` with bind/unbind bracket + `UnbindGuard`. `export_names()` / `get_func()` for introspection.
- **HostState**: Raw pointers to `SessionCore`/`EcsDom` (bind/unbind pattern from `HostBridge`). `Arc<Registry>` for `Send` compatibility. `unsafe impl Send` (single-threaded usage).
- **Host functions** ("elidex" namespace): 12 functions registered via `Linker`. Entity handles as `i64` (0 = null). Strings via `(ptr, len)` pairs in Wasm linear memory. Returns: packed `i64` `(ptr << 32) | len`, allocated via guest's `__alloc(len) -> ptr`.
- **Identity map bridge**: `objref_to_entity_i64()` / `entity_i64_to_objref()` translate between Wasm entity handles and `JsObjectRef` for handler dispatch.
- **Dependencies**: wasmtime 29, elidex-plugin, elidex-ecs, elidex-script-session, elidex-dom-api. Dev: wat 1.
