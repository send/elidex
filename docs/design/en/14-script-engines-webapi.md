
# 14. Script Engines & Web API

This chapter covers the script execution engines (ECMAScript and WebAssembly) and the Web API surface beyond DOM/CSSOM. All script engines interact with the ECS exclusively through the ScriptSession (Chapter 13).

## 14.1 ECMAScript Engine

ECMAScript execution follows the same "core + compat" pattern. The engine targets ES2020+ as the modern baseline, with legacy semantics handled by optional plugins:

```rust
pub enum EsSpecLevel {
    Modern,          // ES2020+: let/const, arrow fn, class, async/await, modules,
                     //          destructuring, template literals, optional chaining
    LegacySemantics, // var hoisting quirks, function hoisting in blocks,
                     //          sloppy mode == with type coercion
    AnnexB,          // HTML comments in JS, __proto__ accessor,
                     //          RegExp legacy features, string HTML methods
}
```

### 14.1.1 What Gets Cut

| Feature | Core | Compat | Rationale |
| --- | --- | --- | --- |
| let / const / class / arrow | ✓ | — | Modern variable declarations and syntax |
| async / await / Promise | ✓ | — | Async model. Tightly coupled with event loop (Chapter 13). |
| ES Modules (import/export) | ✓ | — | Primary module system. CommonJS is compat-layer only. |
| Proxy / Reflect | ✓ | — | Meta-programming. Required by modern frameworks (Vue 3 reactivity). |
| var hoisting (function-scoped) | ✗ | ✓ | Source of countless bugs. var is parsed but treated as let in core; compat restores quirky hoisting. |
| with statement | ✗ | ✓ | Prevents engine optimization (scope unpredictable). Already forbidden in strict mode. |
| arguments.callee / .caller | ✗ | ✓ | Forbidden in strict mode. Prevents tail-call and inlining optimizations. |
| __proto__ accessor | ✗ | ✓ | Annex B. Use Object.getPrototypeOf() instead. |
| HTML comments in JS (<!-- -->) | ✗ | ✓ | Annex B relic from 1990s <script> hiding. |
| eval() (direct) | Limited | ✓ | Core supports strict-mode eval only (new scope). Sloppy eval (local scope injection) in compat. |

### 14.1.2 Implementation Strategy

Building a JS engine is the largest single component in elidex. The implementation proceeds in stages, with SpiderMonkey providing a working browser throughout:

| Stage | Deliverable | elidex-browser uses | elidex-app uses |
| --- | --- | --- | --- |
| 1 | Parser + AST (ES2020+ syntax) | SpiderMonkey | SpiderMonkey or Wasm |
| 2 | Bytecode compiler + interpreter | SpiderMonkey | elidex-js (self-built) |
| 3 | Inline caches + hidden classes | Switchable: SpiderMonkey or elidex-js | elidex-js |
| 4 | Baseline JIT (Cranelift backend) | elidex-js | elidex-js |
| 5 | Optimizing JIT (if needed) | elidex-js | elidex-js |

The Boa project (a Rust JS engine) provides a reference point but is not production-ready for browser use. The elidex-js engine can study Boa's architecture while making different trade-offs—particularly, elidex-js can omit Annex B and sloppy mode from its core, simplifying the implementation substantially.

The ScriptEngine trait abstraction ensures SpiderMonkey and elidex-js are interchangeable at any point. Note that the engine receives a ScriptSession rather than direct ECS access:

```rust
pub trait ScriptEngine: Send + Sync {
    fn name(&self) -> &str;
    fn eval(&self, source: &str, ctx: &mut ScriptContext) -> Result<JsValue>;
    fn call(&self, func: &JsFunction, args: &[JsValue]) -> Result<JsValue>;
    fn bind_session(&mut self, session: &mut dyn ScriptSession);
    fn run_microtasks(&mut self);
}

enum ScriptBackend {
    SpiderMonkey(MozJsEngine),    // Phase 1-3: mature, full compat
    ElidexJs(ElidexJsEngine),     // Phase 2+: self-built, growing
}
```

## 14.2 Wasm Runtime

WebAssembly support is provided via wasmtime, a mature Rust-native Wasm runtime. Wasm is a first-class citizen alongside JS, not a subsidiary:

| Context | JS Engine | Wasm Runtime |
| --- | --- | --- |
| elidex-browser | SpiderMonkey → elidex-js (phase migration). Handles <script> tags. | wasmtime. Handles WebAssembly.instantiate() from JS, and native .wasm modules. |
| elidex-app | elidex-js (ES2020+ only, no compat). For apps using JS/TS. | wasmtime. Primary runtime for non-JS languages (Rust, Go, C++, Zig, etc.). |

Wasm modules interact with the DOM and CSSOM via the same ScriptSession used by the JS engine, ensuring consistent behavior regardless of the calling language:

```rust
// Host functions shared by JS engine and Wasm runtime
// All writes go through the shared ScriptSession
pub trait SessionHostFunctions {
    fn query_selector(&self, root: EntityId, selector: &str) -> Option<EntityId>;
    fn get_attribute(&self, entity: EntityId, name: &str) -> Option<String>;
    fn set_attribute(&mut self, entity: EntityId, name: &str, value: &str);
    fn add_event_listener(&mut self, entity: EntityId, event: &str, cb: CallbackRef);
    fn set_inline_style(&mut self, entity: EntityId, property: &str, value: &str);
    fn batch_update(&mut self, ops: &[SessionOperation]) -> Vec<OperationResult>;
}
```

The batch_update API is critical for Wasm performance. Each Wasm→host boundary crossing has overhead, so batching multiple operations into a single call reduces this cost dramatically. All operations in a batch are recorded in the session buffer and flushed together.

## 14.3 Multi-Language Application Runtime

The Wasm runtime makes elidex-app a multi-language application platform. Developers choose their language:

| Language | Toolchain | Example Use Case |
| --- | --- | --- |
| Rust | wasm-bindgen + wasm-pack | Maximum performance. Shared type system with elidex internals. |
| TypeScript / JS | elidex-js (ES2020+) directly | Web developers' familiar language. Gradual migration from Electron. |
| Go | TinyGo → Wasm | Backend teams building internal tools. |
| C / C++ | Emscripten → Wasm | Porting existing native applications. |
| Zig | zig build → Wasm | Systems programming with simple toolchain. |
| C# / Kotlin | Blazor / Kotlin/Wasm | Enterprise teams with .NET or JVM ecosystem. |

## 14.4 Web API Scope

Web APIs beyond the DOM follow the same core/compat/deprecated pattern applied to every other layer. Each API is classified by its WebApiSpecLevel and implemented as a plugin:

### 14.4.1 Core Web APIs

| Priority | API | Crate | Notes |
| --- | --- | --- | --- |
| P0 | Fetch API | elidex-api-fetch | Promise-based networking. Replaces XMLHttpRequest. |
| P0 | Canvas 2D | elidex-api-canvas | Widely used by web apps and frameworks. |
| P0 | setTimeout / setInterval | (built-in) | Macrotask scheduling. Fundamental to event loop (Chapter 13). |
| P0 | requestAnimationFrame | (built-in) | Render-synchronized callbacks. Step 4 of event loop. |
| P1 | Web Workers | elidex-api-workers | Separate JS or Wasm instance per worker thread. |
| P1 | WebSocket | elidex-api-ws | Real-time communication. |
| P1 | requestIdleCallback | (built-in) | Low-priority task scheduling. Used by React Scheduler. |
| P1 | Intersection / Resize Observer | elidex-api-observers | ECS change tracking maps naturally, like MutationObserver. |
| P1 | Web Crypto API | elidex-api-crypto | Security foundation. Backed by ring/aws-lc-rs in Rust. |
| P1 | CookieStore API | elidex-api-cookies | Async, Promise-based cookie access. Modern replacement for document.cookie. |
| P1 | Broadcast Channel | elidex-api-broadcast | Cross-tab communication. Maps to multi-process IPC. |
| P2 | IndexedDB | elidex-api-idb | Async client-side storage. |
| P2 | WebGL / WebGPU | elidex-api-gpu | GPU compute; natural fit with wgpu backend. |
| P2 | navigator.sendBeacon | elidex-api-beacon | Reliable telemetry delivery. |
| P3 | Service Workers | elidex-api-sw | Offline support, PWAs. |

### 14.4.2 Compat Web APIs

These APIs are available in elidex-browser with the compat layer enabled, but excluded from elidex-app core. Each is shimmed to a modern equivalent:

| API | Compat Shim | Core Equivalent | Rationale |
| --- | --- | --- | --- |
| XMLHttpRequest | elidex-api-xhr | Fetch API | Callback-based, sync mode blocks main thread. Shimmed internally via Fetch. |
| localStorage / sessionStorage | elidex-api-storage-compat | elidex.storage (async) | Synchronous API requires blocking IPC from Renderer to Browser Process. See Section 14.4.3. |
| document.cookie | elidex-api-cookies-compat | CookieStore API | Synchronous string-parsing API. Shimmed via CookieStore. |

### 14.4.3 Storage Architecture

The synchronous storage APIs (localStorage, sessionStorage) are fundamentally incompatible with elidex's multi-process architecture. In a multi-process browser, these APIs require the Renderer Process to issue a blocking IPC call to the Browser Process, stalling the main thread until the response arrives.

Elidex introduces a modern async alternative for elidex-app and makes it available in browser mode as well:

```rust
// elidex.storage — async KV storage (core, available in both modes)
pub trait AsyncStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn keys(&self) -> Result<Vec<String>>;
}
```

| Mode | Sync Storage (localStorage) | Async Storage (elidex.storage) |
| --- | --- | --- |
| elidex-app | Not available (compile-time excluded) | Primary storage API |
| elidex-browser (core) | Not available | Available, recommended |
| elidex-browser (compat) | Available via blocking IPC shim | Available |

This follows the same principle as document.write → innerHTML and getElementsByClassName → querySelectorAll: the synchronous legacy API exists in compat, while core provides a modern non-blocking alternative.

### 14.4.4 Deprecated Web APIs

| API | Status | Notes |
| --- | --- | --- |
| WebSQL | Not implemented | Already removed from browsers. Use IndexedDB. |
| Application Cache (AppCache) | Not implemented | Replaced by Service Workers. |
| document.domain setter | Not implemented | Security risk. Removed in modern browsers. |

> **Phase 0 Survey Result (Ch. 29 §29.4):** document.all 0% (exclusion validated), document.write JA 12.4% / EN 5.3% (compat-only classification confirmed — usage primarily from ad/analytics scripts).
