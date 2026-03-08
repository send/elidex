
# 26. Embedding API

## 26.1 Overview

The Embedding API is the public contract for elidex-app: the Rust API that third-party applications use to embed elidex as a web rendering engine. It is analogous to CEF (Chromium Embedded Framework), WebView2, or Tauri's wry, but designed as a Rust-native library with type-safe bindings.

```rust
use elidex_app::{Engine, EngineConfig, View, ViewConfig};

fn main() {
    // 1. Initialize the engine
    let engine = Engine::builder()
        .with_config(EngineConfig::default())
        .build()
        .expect("engine init");

    // 2. Create a view (web content area)
    let view = engine.create_view(ViewConfig {
        url: "https://example.com".into(),
        width: 1280,
        height: 720,
        ..Default::default()
    });

    // 3. Run the event loop
    engine.run();
}
```

### 26.1.1 Comparison with Existing Solutions

|  | Electron | Tauri | elidex-app |
| --- | --- | --- | --- |
| Engine | Chromium (full) | OS WebView | elidex-core (slim) |
| Script runtime | V8 (JS only) | OS JS engine | Boa → elidex-js (Rust) + wasmtime (see Ch. 14 §14.1.2) |
| Binary size | ~150 MB | ~3-8 MB | Target: ~5-15 MB |
| Rendering consistency | Identical (bundled) | Varies by OS | Identical (bundled) |
| Legacy overhead | Full backward compat | Full (OS dependent) | Zero (HTML5 only) |
| App languages | JS/TS only | Rust backend + JS frontend | Any Wasm-targeting language |
| Native integration | Node.js | Rust backend | Wasm host functions |
| Customizable engine | No | No | Yes (feature flags) |

Elidex-app's key advantages: consistent rendering without OS WebView differences, strict HTML5 parser provides compile-time-like error detection for markup, and feature flags allow embedding only the capabilities an app needs.

### 26.1.2 Design Principles

- **Rust-first**: The primary API is Rust. C bindings generated via cbindgen for non-Rust embedders.
- **Builder pattern**: Configuration via builders with sensible defaults. Minimal boilerplate for simple cases.
- **Layered**: Simple things are simple (load a URL, get a window). Complex things are possible (custom resource loaders, native↔web bridges, headless rendering).
- **Non-opinionated about windowing**: Works with any windowing toolkit (winit, SDL2, native platform, or headless).

## 26.2 Engine

### 26.2.1 Engine Initialization

```rust
pub struct Engine {
    // Internal: process management, GPU context, shared resources
}

pub struct EngineConfig {
    /// Process model
    pub process_mode: ProcessMode,
    /// Feature flags
    pub features: FeatureFlags,
    /// Codec configuration (Ch. 20)
    pub codecs: CodecConfig,
    /// Storage directory for OPFS, caches, etc.
    pub data_dir: Option<PathBuf>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Logging configuration
    pub log_level: LogLevel,
}

pub enum ProcessMode {
    /// All components in the current process. Simplest. No isolation.
    SingleProcess,
    /// Renderer in separate process. Recommended for untrusted content.
    MultiProcess,
}

pub struct FeatureFlags {
    /// Enable compat layer (FileReader, SMIL, legacy codecs, etc.)
    pub compat: bool,
    /// Enable DevTools server
    pub devtools: bool,
    /// Enable Web Audio API
    pub web_audio: bool,
    /// Enable media pipeline (video/audio playback)
    pub media: bool,
    /// Enable WebGL
    pub webgl: bool,
    /// Enable WebGPU
    pub webgpu: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            compat: false,       // elidex-app default: modern only
            devtools: cfg!(debug_assertions),
            web_audio: true,
            media: true,
            webgl: true,
            webgpu: true,
        }
    }
}

impl Engine {
    pub fn builder() -> EngineBuilder { EngineBuilder::new() }

    /// Create a new view (web content area).
    pub fn create_view(&self, config: ViewConfig) -> View { /* ... */ }

    /// Run the engine event loop. Blocks until all views are closed.
    pub fn run(&self) { /* ... */ }

    /// Run one iteration of the event loop (for embedder-driven loops).
    pub fn pump(&self) -> PumpResult { /* ... */ }

    /// Shut down the engine.
    pub fn shutdown(self) { /* ... */ }
}

pub enum PumpResult {
    /// More work to do.
    Continue,
    /// All views closed, engine can shut down.
    Exit,
}
```

### 26.2.2 Event Loop Integration

Two modes for event loop ownership:

**Engine-owned** (simple case):
```rust
engine.run();  // Blocks, handles all events internally
```

**Embedder-owned** (integration with existing event loop):
```rust
loop {
    // Embedder's own event processing
    process_my_events();

    // Pump elidex
    match engine.pump() {
        PumpResult::Continue => {},
        PumpResult::Exit => break,
    }

    // Embedder's rendering
    render_my_ui();
}
```

## 26.3 View

### 26.3.1 View Configuration

```rust
pub struct ViewConfig {
    /// Initial content
    pub content: ViewContent,
    /// Window/surface configuration
    pub surface: SurfaceConfig,
    /// Permissions
    pub permissions: PermissionConfig,
    /// Navigation policy
    pub navigation_policy: NavigationPolicy,
    /// Custom resource loader
    pub resource_loader: Option<Box<dyn ResourceLoader>>,
}

pub enum ViewContent {
    /// Load a URL
    Url(String),
    /// Load HTML from a string
    Html(String),
    /// Load from a local file
    File(PathBuf),
    /// Blank page
    Blank,
}

pub enum SurfaceConfig {
    /// Create a new window with the given configuration
    CreateWindow {
        title: String,
        width: u32,
        height: u32,
        resizable: bool,
        decorations: bool,
        transparent: bool,
    },
    /// Attach to an existing window
    AttachToWindow {
        handle: raw_window_handle::RawWindowHandle,
        display: raw_window_handle::RawDisplayHandle,
        size: (u32, u32),
    },
    /// Headless rendering (no window)
    Headless {
        width: u32,
        height: u32,
    },
}

pub struct PermissionConfig {
    /// Pre-granted permissions (no prompt)
    pub grants: Vec<Permission>,
    /// App capabilities (extended permissions, Ch. 8 §8.8)
    pub capabilities: Vec<AppCapability>,
}

pub enum NavigationPolicy {
    /// Allow all navigations (default for browser)
    AllowAll,
    /// Block all navigations (single-page app)
    BlockAll,
    /// Custom handler
    Custom(Box<dyn NavigationHandler>),
}

pub trait NavigationHandler: Send + Sync {
    /// Called before each navigation. Return the decision.
    fn on_navigate(&self, request: &NavigationRequest) -> NavigationDecision;
}

pub struct NavigationRequest {
    pub url: Url,
    pub initiator: NavigationInitiator,
    pub is_main_frame: bool,
}

pub enum NavigationDecision {
    Allow,
    Block,
    /// Redirect to a different URL
    Redirect(Url),
}
```

### 26.3.2 View API

```rust
pub struct View {
    // Internal handle
}

impl View {
    // === Content Loading ===

    /// Navigate to a URL.
    pub fn load_url(&self, url: &str) { /* ... */ }

    /// Load HTML content directly.
    pub fn load_html(&self, html: &str, base_url: Option<&str>) { /* ... */ }

    /// Reload the current page.
    pub fn reload(&self) { /* ... */ }

    /// Stop loading.
    pub fn stop(&self) { /* ... */ }

    // === Navigation ===

    /// Navigate back in history.
    pub fn go_back(&self) -> bool { /* returns false if no history */ }

    /// Navigate forward in history.
    pub fn go_forward(&self) -> bool { /* ... */ }

    /// Check if back navigation is possible.
    pub fn can_go_back(&self) -> bool { /* ... */ }

    /// Check if forward navigation is possible.
    pub fn can_go_forward(&self) -> bool { /* ... */ }

    /// Current URL.
    pub fn url(&self) -> String { /* ... */ }

    /// Current page title.
    pub fn title(&self) -> String { /* ... */ }

    // === JavaScript Execution ===

    /// Execute JavaScript and return the result.
    pub async fn evaluate_script(&self, script: &str) -> Result<JsValue, JsError> { /* ... */ }

    /// Execute JavaScript without waiting for result.
    pub fn execute_script(&self, script: &str) { /* ... */ }

    // === Native ↔ Web Bridge ===

    /// Expose a Rust function to JavaScript.
    /// Callable from JS as `window.__elidex.call(name, args)`.
    pub fn expose_function<F, A, R>(&self, name: &str, handler: F)
    where
        F: Fn(A) -> R + Send + Sync + 'static,
        A: serde::de::DeserializeOwned,
        R: serde::Serialize,
    { /* ... */ }

    /// Create a bidirectional message channel.
    pub fn create_channel(&self) -> (ChannelSender, ChannelReceiver) { /* ... */ }

    /// Post a message to the page (received via window.__elidex.onMessage).
    pub fn post_message(&self, message: &impl serde::Serialize) { /* ... */ }

    // === Event Hooks ===

    /// Set callback for page load events.
    pub fn on_load_state_changed(&self, callback: impl Fn(LoadState) + Send + 'static) { /* ... */ }

    /// Set callback for title changes.
    pub fn on_title_changed(&self, callback: impl Fn(&str) + Send + 'static) { /* ... */ }

    /// Set callback for URL changes.
    pub fn on_url_changed(&self, callback: impl Fn(&str) + Send + 'static) { /* ... */ }

    /// Set callback for console messages.
    pub fn on_console_message(&self, callback: impl Fn(ConsoleMessage) + Send + 'static) { /* ... */ }

    /// Set callback for permission requests.
    pub fn on_permission_request(&self, callback: impl Fn(PermissionRequest) -> PermissionResponse + Send + 'static) { /* ... */ }

    /// Set callback for JavaScript dialogs (alert, confirm, prompt).
    pub fn on_dialog(&self, callback: impl Fn(DialogRequest) -> DialogResponse + Send + 'static) { /* ... */ }

    /// Set callback for download requests.
    pub fn on_download(&self, callback: impl Fn(DownloadRequest) -> DownloadDecision + Send + 'static) { /* ... */ }

    // === Rendering Control ===

    /// Set the frame policy (Ch. 15).
    pub fn set_frame_policy(&self, policy: FramePolicy) { /* ... */ }

    /// Capture the current page as an image.
    pub async fn capture_screenshot(&self) -> Result<ImageBuffer, CaptureError> { /* ... */ }

    /// Resize the view.
    pub fn resize(&self, width: u32, height: u32) { /* ... */ }

    /// Set device scale factor.
    pub fn set_scale_factor(&self, factor: f64) { /* ... */ }

    // === Lifecycle ===

    /// Close the view and release resources.
    pub fn close(self) { /* ... */ }
}

pub enum LoadState {
    Started,
    Committed,
    DomContentLoaded,
    Complete,
    Failed(NavigationError),
}

pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub message: String,
    pub source: String,
    pub line: u32,
}
```

## 26.4 Native ↔ Web Bridge

### 26.4.1 Function Injection

Expose Rust functions callable from JavaScript:

```rust
// Rust side
view.expose_function("greet", |name: String| -> String {
    format!("Hello, {}!", name)
});

view.expose_function("get_user", |id: u64| -> User {
    database.find_user(id)
});

// JavaScript side
const greeting = await window.__elidex.call("greet", "World");
// greeting === "Hello, World!"

const user = await window.__elidex.call("get_user", 42);
// user === { name: "Alice", email: "alice@example.com" }
```

Arguments and return values are serialized via serde_json. The call is asynchronous from JavaScript's perspective (returns a Promise).

### 26.4.2 Message Channel

For bidirectional streaming communication:

```rust
// Rust side
let (sender, receiver) = view.create_channel();

// Send to JS
sender.send(&MyEvent { kind: "update", data: 42 });

// Receive from JS
tokio::spawn(async move {
    while let Some(msg) = receiver.recv().await {
        handle_message(msg);
    }
});

// JavaScript side
window.__elidex.onMessage = (msg) => {
    console.log("From Rust:", msg);
};

window.__elidex.postMessage({ action: "click", x: 100, y: 200 });
```

### 26.4.3 Bridge API Namespace

All bridge APIs live under `window.__elidex`:

```typescript
interface ElidexBridge {
    // Function calls (exposed via expose_function)
    call(name: string, ...args: any[]): Promise<any>;

    // Message passing
    postMessage(message: any): void;
    onMessage: ((message: any) => void) | null;

    // App metadata
    readonly appName: string;
    readonly appVersion: string;

    // Platform info
    readonly platform: "macos" | "windows" | "linux" | "android" | "ios";
}
```

## 26.5 Custom Resource Loader

Embedders can intercept and handle resource requests:

```rust
pub trait ResourceLoader: Send + Sync {
    /// Intercept a resource request.
    /// Return Some(response) to handle it, None to fall through to normal loading.
    fn load(&self, request: &ResourceRequest) -> Option<ResourceResponse>;
}

pub struct ResourceRequest {
    pub url: Url,
    pub method: HttpMethod,
    pub headers: HeaderMap,
    pub resource_type: ResourceType,
}

pub struct ResourceResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: ResourceBody,
}

pub enum ResourceBody {
    Bytes(Bytes),
    Stream(ByteStream),
}

// Example: serve app assets from embedded data
struct EmbeddedAssets;

impl ResourceLoader for EmbeddedAssets {
    fn load(&self, request: &ResourceRequest) -> Option<ResourceResponse> {
        if request.url.scheme() == "app" {
            let path = request.url.path();
            let data = include_bytes_matching(path)?;
            Some(ResourceResponse {
                status: 200,
                headers: content_type_for(path),
                body: ResourceBody::Bytes(data.into()),
            })
        } else {
            None  // fall through to network
        }
    }
}
```

This integrates with Ch. 10's AppResourceLoader pattern, providing the embedding API surface for it.

## 26.6 Multi-View

An Engine can host multiple Views, each rendering independent web content:

```rust
let engine = Engine::builder().build().unwrap();

let main_view = engine.create_view(ViewConfig {
    content: ViewContent::Url("https://app.example.com".into()),
    surface: SurfaceConfig::CreateWindow { title: "Main".into(), width: 1280, height: 720, .. },
    ..Default::default()
});

let settings_view = engine.create_view(ViewConfig {
    content: ViewContent::Html(include_str!("settings.html").into()),
    surface: SurfaceConfig::CreateWindow { title: "Settings".into(), width: 600, height: 400, .. },
    ..Default::default()
});

// Views share the Engine's GPU context and process infrastructure
// but have independent DOM, script contexts, and storage
```

In MultiProcess mode, each View gets its own Renderer Process. In SingleProcess mode, Views share the process but have isolated ECS Worlds.

## 26.7 Headless Mode

For server-side rendering, testing, and screenshot generation:

```rust
let engine = Engine::builder()
    .with_config(EngineConfig {
        process_mode: ProcessMode::SingleProcess,
        ..Default::default()
    })
    .build()
    .unwrap();

let view = engine.create_view(ViewConfig {
    content: ViewContent::Url("https://example.com".into()),
    surface: SurfaceConfig::Headless { width: 1920, height: 1080 },
    ..Default::default()
});

// Wait for page load
view.on_load_state_changed(|state| {
    if matches!(state, LoadState::Complete) {
        // Capture screenshot
        let image = view.capture_screenshot().await.unwrap();
        image.save("screenshot.png");
    }
});

engine.run();
```

Headless mode uses Vello's CPU backend (Ch. 15 §15.6.4) for rendering without a GPU context.

## 26.8 DevTools

When DevTools are enabled (`features.devtools = true`), the engine starts a Chrome DevTools Protocol (CDP) server:

```rust
let engine = Engine::builder()
    .with_config(EngineConfig {
        features: FeatureFlags { devtools: true, ..Default::default() },
        ..Default::default()
    })
    .build()
    .unwrap();

// DevTools available at ws://localhost:9222
// Connect with Chrome DevTools or any CDP client
```

The CDP server provides: DOM inspection, CSS editing, JavaScript debugging, network monitoring, performance profiling, and console access.

## 26.9 C API (cbindgen)

For non-Rust embedders, a C-compatible API is generated via cbindgen:

```c
// elidex.h (auto-generated)

typedef struct ElidexEngine ElidexEngine;
typedef struct ElidexView ElidexView;

ElidexEngine* elidex_engine_create(const ElidexEngineConfig* config);
void elidex_engine_destroy(ElidexEngine* engine);
void elidex_engine_run(ElidexEngine* engine);
int elidex_engine_pump(ElidexEngine* engine);

ElidexView* elidex_view_create(ElidexEngine* engine, const ElidexViewConfig* config);
void elidex_view_destroy(ElidexView* view);
void elidex_view_load_url(ElidexView* view, const char* url);
void elidex_view_load_html(ElidexView* view, const char* html, const char* base_url);

int elidex_view_evaluate_script(
    ElidexView* view,
    const char* script,
    ElidexJsResultCallback callback,
    void* user_data
);

void elidex_view_expose_function(
    ElidexView* view,
    const char* name,
    ElidexFunctionCallback callback,
    void* user_data
);

void elidex_view_post_message(ElidexView* view, const char* json_message);
void elidex_view_set_message_callback(
    ElidexView* view,
    ElidexMessageCallback callback,
    void* user_data
);
```

The C API is a thin wrapper over the Rust API, using opaque pointers and callback functions.

## 26.10 API Stability

| Component | Stability | Policy |
| --- | --- | --- |
| `Engine`, `View`, `ViewConfig` | Stable | Semantic versioning. Breaking changes require major version bump. |
| `expose_function`, `post_message` | Stable | Core bridge API, maintained across versions. |
| `FramePolicy`, `NavigationPolicy` | Stable | Public enum, additive changes only (new variants). |
| `FeatureFlags` | Semi-stable | New flags may be added. Existing flags not removed without deprecation. |
| `ResourceLoader` trait | Semi-stable | Trait methods may be added with default implementations. |
| Internal types (`BlobId`, `EntityId`, etc.) | Unstable | Not exposed in public API. |
| C API | Stable | ABI-compatible across minor versions. |

Deprecation policy: deprecated APIs are marked `#[deprecated]` for at least one minor version before removal in the next major version.

## 26.11 Example: Complete Application

```rust
use elidex_app::*;

fn main() {
    let engine = Engine::builder()
        .with_config(EngineConfig {
            process_mode: ProcessMode::SingleProcess,
            features: FeatureFlags {
                compat: false,
                media: true,
                ..Default::default()
            },
            data_dir: Some("./app_data".into()),
            ..Default::default()
        })
        .build()
        .expect("engine init");

    let view = engine.create_view(ViewConfig {
        content: ViewContent::Url("app://index.html".into()),
        surface: SurfaceConfig::CreateWindow {
            title: "My App".into(),
            width: 1280,
            height: 720,
            resizable: true,
            decorations: true,
            transparent: false,
        },
        permissions: PermissionConfig {
            grants: vec![Permission::Notifications, Permission::ClipboardRead],
            capabilities: vec![
                AppCapability::FileRead("./documents/*".into()),
                AppCapability::FileWrite("./documents/*".into()),
            ],
        },
        navigation_policy: NavigationPolicy::Custom(Box::new(AppNavigationHandler)),
        resource_loader: Some(Box::new(EmbeddedAssets)),
    });

    // Expose native functionality to web content
    view.expose_function("save_file", |args: SaveFileArgs| -> Result<(), String> {
        std::fs::write(&args.path, &args.content).map_err(|e| e.to_string())
    });

    view.expose_function("list_files", |dir: String| -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|entries| entries.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into())).collect())
            .unwrap_or_default()
    });

    // Listen for events
    view.on_title_changed(|title| {
        println!("Title: {}", title);
    });

    view.on_console_message(|msg| {
        println!("[{}] {}", msg.level, msg.message);
    });

    engine.run();
}

struct AppNavigationHandler;

impl NavigationHandler for AppNavigationHandler {
    fn on_navigate(&self, request: &NavigationRequest) -> NavigationDecision {
        if request.url.scheme() == "app" || request.url.scheme() == "https" {
            NavigationDecision::Allow
        } else {
            NavigationDecision::Block
        }
    }
}
```
