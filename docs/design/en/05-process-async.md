
# 5. Process Architecture & Async Runtime

This chapter resolves two foundational design issues: the multi-process model (OPEN-001) and the async I/O runtime (OPEN-011). These are deeply intertwined — the process model determines how many runtime instances exist and what each does; the async runtime determines how I/O, IPC, and the event loop work within each process.

## 5.1 Design Philosophy: Staged Relaxation

The same core/compat pattern used in elidex's engine layer applies to the process model:

| Aspect | Core (long-term goal) | Compat (Phase 1–3 reality) | Reason for compat |
| --- | --- | --- | --- |
| Renderer isolation | Crash isolation only. Single or few Renderer Processes. | Site-based process isolation. | Boa is a third-party JS runtime still being hardened for browser embedding. Isolation limits blast radius of engine/embedder bugs. |
| Network Process | Mergeable into Browser Process thread. | Separate process. | Defense in depth while runtime integration and host bindings are still stabilizing. |
| GPU Process | Mergeable into Renderer thread. | Separate process or thread. | GPU driver crashes are OS-level; isolation value persists but form is flexible. |
| Async runtime | Fully self-built event loop (Renderer-proven, expanded). | tokio for Network/Browser; self-built for Renderer. | tokio provides ecosystem velocity; Renderer needs custom control from day one. |

The key design constraint: **the Phase 1–3 architecture must not preclude the long-term simplification**. Process boundaries and runtime choices are abstracted behind traits so that relaxation is a configuration change, not a rewrite.

## 5.2 Process Model

### 5.2.1 Process Roles

```
┌────────────────────────────────────────────────────────┐
│  Browser Process (privileged, singleton)                │
│                                                        │
│  Shell State (Ch. 24)   Process Lifecycle Manager      │
│  Persistence (OPEN-012) Permission Broker (Ch. 8)      │
│  Extension Host         Download Manager               │
└────────┬──────────────────┬──────────────┬─────────────┘
         │ IPC              │ IPC          │ IPC
┌────────▼────────┐ ┌──────▼───────┐ ┌────▼──────────────┐
│ Renderer Process │ │ Network      │ │ GPU Process       │
│ (per site†)      │ │ Process      │ │ (singleton)       │
│                  │ │ (singleton)  │ │                   │
│ DOM/ECS          │ │ HTTP stack   │ │ wgpu surfaces     │
│ ScriptSession    │ │ DNS resolver │ │ Layer compositing │
│ Style & Layout   │ │ Conn pool    │ │ Rasterization     │
│ Paint            │ │ Cookie jar   │ │ Scroll & anim     │
│ Script engine    │ │ TLS/certs    │ │                   │
│ Wasm runtime     │ │ WebSocket    │ │                   │
│ Event loop       │ │              │ │                   │
└──────────────────┘ └──────────────┘ └───────────────────┘
                                       ┌───────────────────┐
                                       │ Utility Process(es)│
                                       │ (on demand)        │
                                       │                    │
                                       │ Media decode       │
                                       │ Audio processing   │
                                       └───────────────────┘
```

† Site-based isolation in Phase 1–3; relaxable in long-term.

| Process | Count | Sandbox | Responsibilities | Long-term Outlook |
| --- | --- | --- | --- | --- |
| Browser | 1 | None (privileged) | Shell state (Ch. 24), persistence (OPEN-012), permission brokering (Ch. 8), download management, extension lifecycle, process lifecycle management. The only process with full OS access. | Always separate — privileged broker role is permanent. |
| Renderer | 1 per site (Phase 1–3) | Strict (seccomp-bpf / App Sandbox / Restricted token) | DOM/ECS, ScriptSession (Ch. 13), style/layout/paint, script engine, Wasm runtime. No direct network or filesystem access. | Relaxable to fewer processes once elidex-js migration and security hardening are validated. Crash isolation remains valuable. |
| Network | 1 | Moderate (network-only) | HTTP/HTTPS stack (hyper + rustls + h3), DNS resolution, connection pooling, cookie jar, TLS, WebSocket. | Mergeable into Browser Process as a thread. Connection pool sharing across tabs is a practical reason to keep centralized. |
| GPU | 1 | Moderate | wgpu surface management, Vello rasterization, layer compositing, compositor-driven scroll/animation. | Mergeable into Renderer as a thread. GPU driver crash isolation has value but is less critical than Renderer isolation. |
| Utility | 0–N | Strict | Media decoding (OPEN-002), audio processing. Spawned on demand, terminated when idle. | Retained for C/C++ library isolation (FFmpeg, etc.). Unnecessary for pure-Rust decoders. |

### 5.2.2 Site Isolation (Phase 1–3)

Renderer Process isolation uses site-based granularity, where a site is scheme + eTLD+1 (e.g., `https://example.com`):

```
Tab: https://news.example.com/article
  ├── Main frame: news.example.com       → Renderer A
  ├── <iframe src="ads.tracker.com/..."> → Renderer B  (cross-site)
  └── <iframe src="cdn.example.com/..."> → Renderer A  (same site)
```

This provides Spectre/Meltdown mitigation: cross-site iframes in separate OS processes with separate address spaces. Combined with COOP/COEP enforcement (Ch. 8), this prevents speculative execution side-channel attacks between origins.

### 5.2.3 Isolation Granularity Configuration

The process model is configurable at build time and startup, enabling the staged relaxation:

```rust
pub enum ProcessModel {
    /// Phase 1–3 default for elidex-browser.
    /// Each site gets its own Renderer Process.
    SiteIsolation,

    /// Long-term option for elidex-browser after full Rust migration.
    /// One Renderer per tab (crash isolation without site isolation).
    PerTab,

    /// Minimal isolation. Multiple tabs share Renderers.
    /// Crash in one tab may affect others in the same process.
    Shared { max_renderers: usize },

    /// elidex-app default. Everything in one process.
    /// Maximum performance, minimum overhead.
    SingleProcess,
}
```

The transition from `SiteIsolation` to `PerTab` or `Shared` is non-breaking because the IPC abstraction (Section 5.3) works identically regardless of whether the "other process" is a real OS process or a logical boundary within the same process.

### 5.2.4 elidex-app Process Model

elidex-app defaults to `SingleProcess` — the entire engine runs in the application's process:

```rust
let app = elidex_app::App::new()
    // Default: SingleProcess. No IPC overhead, minimal startup time.
    // Script engine is elidex-js (Rust), so no C++ memory safety concern.
    .build();
```

Apps that embed untrusted web content (e.g., an RSS reader rendering arbitrary HTML) can opt into isolation:

```rust
let app = elidex_app::App::new()
    .process_model(ProcessModel::PerTab)  // Isolate each WebView
    .build();
```

### 5.2.5 Process Lifecycle

| Event | Behavior |
| --- | --- |
| Tab opened | Process Lifecycle Manager provides a Renderer for the target site. Reuses existing Renderer if one exists for that site, otherwise spawns new process. |
| Tab closed | If no other tabs reference the Renderer, graceful shutdown with timeout. |
| Renderer crash | Browser Process detects via IPC channel EOF. Tab shows crash page with reload option. Other tabs unaffected. Crash dump captured (minidump format). |
| OOM pressure | OS memory pressure notification → Browser Process selects background Renderers to discard (LRU). Discarded tabs show placeholder; reload on focus restores state. |
| Network Process crash | Auto-restart. In-flight requests fail; Renderers receive error responses and can retry. |
| GPU Process crash | Auto-restart. Brief visual glitch. Renderers re-submit display lists. Compositor state rebuilt. |

## 5.3 IPC Architecture

### 5.3.1 IPC Trait Abstraction

The critical design: IPC is abstracted behind traits, so the same code works whether the target is in another OS process or in-process. This is what makes the staged relaxation possible:

```rust
/// Core IPC abstraction. Implemented for both cross-process and in-process.
pub trait IpcChannel<Req, Resp>: Send + Sync {
    async fn send(&self, message: Req) -> Result<()>;
    async fn recv(&self) -> Result<Resp>;
    async fn call(&self, request: Req) -> Result<Resp>;  // send + await response
}

/// Cross-process implementation: serialization over OS pipe.
pub struct ProcessChannel<Req, Resp> { /* ipc-channel internals */ }

/// In-process implementation: direct async channel (zero-copy).
pub struct LocalChannel<Req, Resp> { /* tokio::sync::mpsc or similar */ }
```

When `ProcessModel::SingleProcess` is selected, `LocalChannel` is used instead of `ProcessChannel`. No serialization, no OS pipe overhead, no data copying. The engine code is identical — it programs against `dyn IpcChannel<Req, Resp>`.

### 5.3.2 IPC Transport Mechanisms

| Mechanism | Use Case | Implementation |
| --- | --- | --- |
| Typed channels | Commands, events, small payloads (<64KB). | ipc-channel (Servo's crate) over OS pipes. Messages are Rust enums, serialized with postcard (compact binary, no-std compatible). |
| Shared memory | Large data: display lists (Renderer→GPU), decoded bitmaps (Utility→Renderer). | mmap-backed shared memory regions. Ownership transferred via channel messages containing handles. |
| In-process bypass | SingleProcess mode. All communication. | tokio::sync::mpsc or crossbeam channels. Zero-copy. |

### 5.3.3 Message Types

Each process pair communicates through typed Rust enums. Compile-time exhaustiveness checking prevents protocol mismatches:

```rust
/// Browser → Renderer
pub enum BrowserToRenderer {
    NavigateTo(Url),
    ExecuteScript(String),
    SetViewportSize(PhysicalSize),
    GrantPermission(PermissionType),
    Suspend,   // Background tab
    Resume,    // Foreground tab
}

/// Renderer → Browser
pub enum RendererToBrowser {
    NavigationRequest(Url),           // User clicked link
    PermissionRequest(PermissionType), // Script requests camera, etc.
    TitleChanged(String),
    FaviconUpdated(Vec<u8>),
    ConsoleMessage(LogLevel, String),
    CrashReport(CrashDump),
}

/// Renderer → Network
pub enum RendererToNetwork {
    Fetch(FetchId, Request),
    CancelFetch(FetchId),
    WebSocketOpen(WsId, Url),
    WebSocketSend(WsId, Vec<u8>),
    WebSocketClose(WsId),
}

/// Network → Renderer
pub enum NetworkToRenderer {
    FetchResponse(FetchId, Response),
    FetchBodyChunk(FetchId, Vec<u8>),  // Streaming
    FetchComplete(FetchId),
    FetchError(FetchId, FetchError),
    WebSocketMessage(WsId, Vec<u8>),
    WebSocketClosed(WsId, CloseReason),
}

/// Renderer → GPU
pub enum RendererToGpu {
    SubmitDisplayList(SurfaceId, DisplayList),  // Via shared memory
    UpdateScrollOffset(SurfaceId, Vec2),
    Resize(SurfaceId, PhysicalSize),
}

/// GPU → Renderer
pub enum GpuToRenderer {
    FramePresented(SurfaceId, FrameTimestamp),
    SurfaceLost(SurfaceId),
}
```

## 5.4 Async Runtime Architecture

### 5.4.1 Per-Process Runtime Strategy

Each process type uses the async runtime best suited to its workload:

| Process | Runtime | Rationale |
| --- | --- | --- |
| Browser | tokio multi-thread | General-purpose I/O multiplexing. UI events, IPC dispatch, persistence I/O. Standard async server workload. |
| Network | tokio multi-thread | I/O intensive. Thousands of concurrent connections, HTTP/2 multiplexing, DNS queries. tokio is designed for exactly this. |
| Renderer | elidex event loop (custom) + tokio current_thread as I/O backend | Must control frame timing, rAF scheduling, flush points. The JS event loop (Ch. 13) is the primary sequencer. tokio's reactor provides I/O readiness notifications without taking control. |
| GPU | Lightweight event loop | Vsync-driven. Receives display lists, composites, presents. Minimal async I/O. |
| Utility | tokio current_thread | Short-lived, focused tasks. Simple async for receiving work and returning results. |

### 5.4.2 Renderer Event Loop: The Integrated Design

The Renderer's main thread is the most complex, because it must interleave JS execution, async I/O, IPC messages, and frame rendering within tight timing constraints. The elidex event loop owns the main thread and drives everything:

```rust
// Renderer main thread — elidex owns the loop
fn renderer_main(ipc: RendererIpc, tokio_rt: tokio::runtime::Runtime) {
    let mut script_engine = ScriptEngine::new();
    let mut session = ScriptSession::new();
    let mut dom = EcsDom::new();
    let mut task_queue = TaskQueue::new();

    loop {
        // ── Phase 1: Collect external events (non-blocking) ──────────
        // Poll tokio reactor for I/O completions (fetch responses, etc.)
        // This does NOT yield control to tokio's scheduler.
        tokio_rt.block_on(async {
            tokio::task::yield_now().await;
        });

        // Drain IPC messages from Browser/Network/GPU
        while let Some(msg) = ipc.try_recv() {
            task_queue.enqueue_from_ipc(msg);
        }

        // ── Phase 2: JS event loop (Ch. 13 semantics) ────────────────
        // Execute oldest macrotask
        if let Some(task) = task_queue.pop() {
            script_engine.eval(task, &mut session);
        }

        // Drain microtasks
        script_engine.drain_microtasks(&mut session);

        // Flush ScriptSession → ECS
        let records = session.flush(&mut dom);
        deliver_mutation_observers(records, &mut script_engine, &mut session);
        script_engine.drain_microtasks(&mut session);

        // ── Phase 3: Rendering (if vsync opportunity) ────────────────
        if vsync_ready() {
            // requestAnimationFrame callbacks
            for cb in animation_frame_callbacks.drain(..) {
                script_engine.eval(cb, &mut session);
            }
            script_engine.drain_microtasks(&mut session);
            session.flush(&mut dom);

            // Style → Layout → Paint → submit to compositor thread (Ch. 6)
            run_style_system(&dom);
            run_layout_system(&dom);
            let display_list = run_paint_system(&dom);
            compositor_channel.send(CompositorMsg::SubmitDisplayList(display_list));
        }

        // ── Phase 4: Idle work ───────────────────────────────────────
        if has_idle_time() {
            for cb in idle_callbacks.drain(..) {
                script_engine.eval(cb, &mut session);
            }
        }

        // ── Phase 5: Wait for next event or vsync ────────────────────
        // Sleep until: IPC message arrives, I/O completes, timer fires,
        // or vsync signal. This is where mio/tokio's epoll/kqueue waits.
        wait_for_event_or_vsync();
    }
}
```

The critical point: elidex's loop structure matches the HTML Living Standard's event loop specification (Ch. 13) exactly, with I/O and IPC integrated as event sources. tokio never "runs" the loop — it provides the polling infrastructure that `wait_for_event_or_vsync()` delegates to.

### 5.4.3 Async Runtime Trait Abstraction

To preserve the option of replacing tokio in the future, process-level async operations go through a trait:

```rust
pub trait AsyncRuntime: Send + Sync {
    fn spawn<F>(&self, future: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static;

    fn spawn_blocking<F, R>(&self, func: F) -> TaskHandle
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;

    /// Non-blocking poll: drive pending I/O without yielding control.
    /// Used by Renderer's custom event loop.
    fn poll_reactor(&self);

    /// Blocking wait: sleep until I/O, timer, or external signal.
    /// Used by Renderer's wait_for_event_or_vsync().
    fn park_until(&self, deadline: Option<Instant>);
}
```

Phase 1–3: `TokioRuntime` implements this trait. Long-term: `ElidexRuntime` can replace it, starting from the Renderer's event loop (which is already custom) and expanding outward.

### 5.4.4 Timer Integration

JS timers (setTimeout, setInterval, requestAnimationFrame) and Rust async timers must coexist on the Renderer's main thread:

| Timer Source | Integration |
| --- | --- |
| setTimeout / setInterval | Registered in TaskQueue with deadline. Checked at Phase 1 of event loop. Backed by the async runtime's timer wheel. |
| requestAnimationFrame | Tied to vsync signal from GPU Process. Accumulated between frames. Executed at Phase 3. |
| requestIdleCallback | Executed at Phase 4 if frame budget has remaining time. |
| Rust async timers (tokio::time::sleep) | Driven by the same reactor as I/O. Wakes the event loop from Phase 5 sleep. |

All timer sources feed into the same `wait_for_event_or_vsync()` mechanism, ensuring the thread wakes at the earliest deadline.

## 5.5 Backpressure and Flow Control

When data flows across process boundaries (e.g., Network streams fetch data faster than Renderer can parse), backpressure prevents unbounded memory growth:

```rust
/// Bounded channel with backpressure.
/// Sender blocks (async) when buffer is full.
pub struct BackpressureChannel<T> {
    capacity: usize,  // Max buffered items
    // ...
}
```

| Data Flow | Backpressure Mechanism |
| --- | --- |
| Network → Renderer (fetch body) | Bounded channel per FetchId. Network Process pauses reading from TCP socket when channel is full. Automatically resumes when Renderer consumes. |
| Renderer → GPU (display lists) | Double buffering. Renderer produces frame N+1 while GPU presents frame N. If GPU falls behind, Renderer blocks at submit (frame pacing). |
| Utility → Renderer (decoded media) | Bounded queue for decoded frames. Decoder pauses when queue is full. |

## 5.6 Staged Migration Path

```
Phase 1–3 (Boa era)
├── ProcessModel::SiteIsolation (default for browser)
├── Browser/Network: tokio multi-thread
├── Renderer: elidex event loop + tokio reactor
├── IPC: ProcessChannel (cross-process, serialized)
└── Security: process isolation compensates for bootstrap hardening risk

Phase 4–5 (elidex-js migration)
├── ProcessModel::PerTab or Shared (relaxed, configurable)
├── Network: consider merging into Browser thread
├── GPU: consider merging into Renderer thread
├── Renderer event loop matures, accumulates optimization
└── Security: hardened Rust-first stack makes site isolation optional by threat model

Long-term (all Rust)
├── ProcessModel::Shared or SingleProcess for trusted content
├── elidex event loop replaces tokio in all processes (optional)
├── IPC: LocalChannel where processes merge (zero-copy)
└── Security: crash isolation only; Spectre mitigation delegated to OS/hardware
```

The architecture ensures that each step is a configuration change (or Cargo feature flag), not a structural rewrite. Code that uses `dyn IpcChannel` and `dyn AsyncRuntime` works identically across all phases.
