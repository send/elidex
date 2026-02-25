
# 6. Intra-Process Thread Model

Chapter 5 defines the inter-process architecture — which processes exist, how they communicate, and how the process model relaxes over time. This chapter defines the intra-process thread topology: which threads run inside each process, how data flows between them, and how concurrency primitives are chosen to maintain both performance and correctness.

## 6.1 Design Principles

**Compositor independence is non-negotiable.** The compositor thread must never block on the main thread. This is the foundation of smooth scrolling, pinch-zoom, and CSS transform/opacity animations at 60fps regardless of JS execution load.

**Ownership transfer over shared state.** Following Rust's ownership model, data is moved between threads rather than shared behind locks wherever possible. DisplayList ownership transfers from main thread to compositor. IPC messages are moved, not cloned.

**B→C migration path.** The initial design (Phase 1–3) uses message passing between main thread and compositor (Approach B). When process isolation relaxes in Phase 4+, the compositor can transition to direct ECS reads (Approach C). The FrameSource trait abstracts this boundary, making the transition a configuration change.

**Thread pools are shared, not duplicated.** A single rayon pool handles all CPU-parallel work (style, layout, image decode). tokio's runtime handles all async I/O. They coexist without competing for cores.

## 6.2 Renderer Process Thread Topology

The Renderer is the most complex process. It has four thread classes:

```
Renderer Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Main Thread (1)              Compositor Thread (1)                 │
│  ┌─────────────────────┐     ┌──────────────────────┐              │
│  │ Event Loop (Ch. 5) │     │ Frame scheduling     │              │
│  │ JS/Wasm execution   │     │ Layer compositing    │              │
│  │ DOM (ECS owner)      │────▶│ Scroll/zoom/anim    │              │
│  │ Style → Layout → Paint│ DP │ GPU submit           │              │
│  │ ScriptSession        │    │ Input fast path      │              │
│  └─────────────────────┘     └──────────────────────┘              │
│           │                                                         │
│           │ work stealing                                           │
│           ▼                                                         │
│  rayon Pool (N threads)       Worker Threads (0–M)                 │
│  ┌─────────────────────┐     ┌──────────────────────┐              │
│  │ Parallel style       │     │ Dedicated Worker (1:1)│              │
│  │ Parallel layout      │     │ JS/Wasm execution    │              │
│  │ Image decode         │     │ Own WorkerSession    │              │
│  │ Font rasterization   │     │ postMessage IPC      │              │
│  └─────────────────────┘     └──────────────────────┘              │
│                                                                     │
│  tokio reactor (current_thread, on main thread)                    │
│  ┌─────────────────────┐                                           │
│  │ Fetch responses      │                                           │
│  │ IPC message receive  │                                           │
│  │ Timer management     │                                           │
│  └─────────────────────┘                                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

DP = DisplayPipeline channel (Approach B) or ECS shared read (Approach C)
```

### 6.2.1 Main Thread

The main thread is the single owner of the ECS DOM. All DOM mutations, script execution, style computation initiation, and layout happen here. Its structure is defined by the event loop (Ch. 5, Section 5.4.2):

| Responsibility | Details |
| --- | --- |
| ECS DOM ownership | Sole writer. All component mutations go through ScriptSession flush. |
| Script execution | SpiderMonkey (Phase 1–3) or elidex-js (Phase 4+). Single-threaded JS semantics. |
| Event loop | 5-phase loop: collect → JS → render → idle → wait (Ch. 5). |
| Style initiation | Kicks off parallel style resolution on rayon pool, waits for completion. |
| Layout initiation | Kicks off parallel layout on rayon pool, waits for completion. |
| Paint | Generates Layer components in ECS. Serializes to DisplayList for compositor (Approach B). |
| tokio reactor | Polled in Phase 1 of event loop. I/O completions and timer wakeups. |

**Critical constraint:** The main thread blocks during rayon parallel work (style, layout). This is intentional — the pipeline is sequential (script → style → layout → paint → composite), and rayon's work-stealing ensures the main thread participates as a worker during parallel phases.

### 6.2.2 Compositor Thread

A dedicated OS thread running independently of the main thread. Receives frame data and produces GPU output:

```rust
fn compositor_thread(
    frame_source: Box<dyn FrameSource>,
    gpu: GpuContext,
    input_rx: Receiver<InputEvent>,
) {
    loop {
        // 1. Check for new frame data from main thread
        frame_source.poll_update();

        // 2. Process compositor-handled input events
        while let Ok(event) = input_rx.try_recv() {
            match event {
                InputEvent::Scroll(delta) => {
                    frame_source.update_scroll(target, delta);
                }
                InputEvent::PinchZoom(scale) => {
                    frame_source.update_zoom(scale);
                }
                _ => {} // Other events go to main thread
            }
        }

        // 3. Advance compositor-driven animations
        frame_source.advance_animations(dt);

        // 4. Composite layers and submit to GPU
        let frame = frame_source.composite();
        gpu.submit(frame);

        // 5. Wait for vsync
        gpu.wait_vsync();
    }
}
```

| Responsibility | Details |
| --- | --- |
| Layer compositing | Combine layers with correct z-order, transforms, opacity, clipping. |
| Independent scroll | Update scroll offsets without main thread involvement. Scroll events that require JS handlers (non-passive listeners) are forwarded to main thread. |
| CSS animations (subset) | transform and opacity animations run on compositor (no layout/repaint needed). Other animated properties require main thread. |
| Pinch-zoom | Compositor scales and translates layers. Viewport change notified to main thread asynchronously. |
| GPU submission | Send composite frame to GPU process (Approach B, cross-process) or directly to wgpu (Approach C, same process). |
| Frame scheduling | Aligns work to vsync. Drops frames gracefully if main thread is behind. |

### 6.2.3 FrameSource Trait — The B/C Abstraction

```rust
pub trait FrameSource: Send {
    /// Check for new frame data. In B mode, receives DisplayList from channel.
    /// In C mode, checks ECS frame-ready signal.
    fn poll_update(&mut self);

    /// Get current layer snapshot for compositing.
    fn layers(&self) -> &LayerTree;

    /// Update scroll offset (compositor-driven, independent of main thread).
    fn update_scroll(&mut self, target: ScrollTarget, offset: ScrollDelta);

    /// Update pinch-zoom scale.
    fn update_zoom(&mut self, scale: f32);

    /// Advance compositor-driven animations (transform, opacity).
    fn advance_animations(&mut self, dt: Duration);

    /// Composite all layers into a final frame.
    fn composite(&self) -> CompositeFrame;

    /// Report compositor-side scroll position back to main thread
    /// (for JS scroll event handlers, position: sticky, etc.)
    fn sync_scroll_to_main(&self);
}
```

**Approach B implementation (Phase 1–3):**

```rust
pub struct DisplayListFrameSource {
    /// Channel receiving DisplayLists from main thread
    rx: Receiver<DisplayList>,
    /// Current active layer tree (owned by compositor)
    active_tree: LayerTree,
    /// Pending layer tree (received but not yet activated)
    pending_tree: Option<LayerTree>,
    /// Scroll state (compositor-owned, synced back to main thread)
    scroll_state: ScrollState,
    /// Animation state
    animations: AnimationState,
    /// Channel to send scroll updates back to main thread
    scroll_tx: Sender<ScrollUpdate>,
}

impl FrameSource for DisplayListFrameSource {
    fn poll_update(&mut self) {
        if let Ok(display_list) = self.rx.try_recv() {
            // Build new layer tree from display list
            self.pending_tree = Some(LayerTree::from_display_list(display_list));
        }
        // Activate pending tree at frame boundary
        if let Some(tree) = self.pending_tree.take() {
            self.active_tree = tree;
        }
    }

    fn update_scroll(&mut self, target: ScrollTarget, delta: ScrollDelta) {
        // Update scroll on compositor's own copy — no lock, no IPC
        self.scroll_state.apply(target, delta);
    }
    // ...
}
```

**Approach C implementation (Phase 4+):**

```rust
pub struct EcsFrameSource {
    /// Shared reference to ECS world (same process)
    ecs: Arc<EcsWorld>,
    /// Frame-ready signal from main thread
    frame_signal: AtomicBool,
    /// Compositor-mutable state (lock-free)
    compositor_state: Arc<CompositorMutableState>,
}

pub struct CompositorMutableState {
    /// Per-scroll-container offsets. Updated by compositor, read by main thread.
    scroll_offsets: DashMap<EntityId, AtomicScrollOffset>,
    /// Compositor-driven animation progress.
    animation_ticks: DashMap<AnimationId, AtomicF64>,
}

impl FrameSource for EcsFrameSource {
    fn poll_update(&mut self) {
        // No data transfer needed — just check if main thread signaled a new frame
        if self.frame_signal.swap(false, Ordering::Acquire) {
            // Main thread has completed style/layout/paint.
            // Layer components in ECS are up to date.
        }
    }

    fn update_scroll(&mut self, target: ScrollTarget, delta: ScrollDelta) {
        // Atomic update — no lock contention with main thread
        self.compositor_state.scroll_offsets
            .get(&target.entity)
            .map(|offset| offset.apply_delta(delta));
    }
    // ...
}
```

### 6.2.4 Display Pipeline: Main Thread → Compositor Data Flow

In Approach B, the main thread produces a DisplayList and sends it through a bounded channel:

```rust
pub struct DisplayPipeline {
    tx: SyncSender<DisplayList>,  // Bounded(1) — backpressure if compositor is behind
}

impl DisplayPipeline {
    pub fn submit(&self, display_list: DisplayList) {
        match self.tx.try_send(display_list) {
            Ok(()) => {}                    // Compositor will pick up next frame
            Err(TrySendError::Full(_)) => {
                // Compositor still processing previous frame.
                // Drop this frame (frame skip). Main thread does not block.
            }
            Err(TrySendError::Disconnected(_)) => {
                // Compositor thread has died — error recovery
            }
        }
    }
}
```

The bounded(1) channel provides natural backpressure: if the compositor can't keep up, the main thread drops frames rather than accumulating unbounded latency. This is the correct behavior — rendering should never queue.

### 6.2.5 Input Event Routing

Input events are routed to either the compositor thread or the main thread based on the event type and the page's event listener registration:

```
Platform Input (Ch. 23)
  │
  ├─ Scroll/Touch/Pinch ─▶ Compositor thread (fast path)
  │                           │
  │                           ├─ If passive listener or no listener:
  │                           │    Handle entirely on compositor (no main thread)
  │                           │
  │                           └─ If non-passive listener registered:
  │                                Forward to main thread for JS handling.
  │                                Compositor scrolls optimistically,
  │                                main thread can call preventDefault() to cancel.
  │
  ├─ Mouse/Pointer ──────▶ Main thread (hit testing requires DOM)
  │
  ├─ Keyboard ───────────▶ Main thread (focus management, text input)
  │
  └─ Resize ─────────────▶ Both (compositor adjusts viewport immediately,
                                  main thread triggers relayout)
```

The passive event listener distinction is critical. If `addEventListener('scroll', handler, { passive: true })` (or no listener at all), the compositor never waits for the main thread. This is why Chrome warns about non-passive scroll listeners — they force compositor → main thread round-trips that add scroll latency.

## 6.3 rayon Thread Pool

### 6.3.1 Configuration

A single rayon ThreadPool is shared across all CPU-parallel work within the Renderer:

```rust
pub fn create_renderer_thread_pool() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(renderer_worker_count())
        .thread_name(|i| format!("elidex-rayon-{i}"))
        .build()
        .expect("rayon pool creation")
}

fn renderer_worker_count() -> usize {
    let cores = num_cpus::get_physical();
    // Reserve 2 cores: 1 for main thread, 1 for compositor
    // Minimum 2 rayon workers even on low-core machines
    (cores.saturating_sub(2)).max(2)
}
```

| Machine | Physical cores | rayon workers | Main | Compositor |
| --- | --- | --- | --- | --- |
| Laptop (4 core) | 4 | 2 | 1 | 1 |
| Desktop (8 core) | 8 | 6 | 1 | 1 |
| Workstation (16 core) | 16 | 14 | 1 | 1 |

### 6.3.2 Work Distribution

| Work Type | Parallelism Pattern | Notes |
| --- | --- | --- |
| Style resolution | Parallel per DOM subtree | Servo-proven. Independent subtrees have no data dependencies. Main thread joins rayon to participate as worker. |
| Layout | Parallel for independent formatting contexts | Block, flex, and grid containers with no cross-dependencies can layout in parallel. Less parallelism than style (layout has more sequential dependencies). |
| Image decode | Spawned as rayon task | Off-main-thread decode. Decoded bitmap stored in image cache (Ch. 22). |
| Font rasterization | Spawned as rayon task | Glyph rasterization for font atlas. Independent per glyph. |
| Painting | Parallel per layer | Independent layers (created by will-change, position:fixed, etc.) can paint in parallel. |

### 6.3.3 Main Thread Participation

During parallel phases, the main thread calls `rayon::scope()` or `pool.install()` and becomes a worker itself:

```rust
// Style phase — main thread participates
pool.install(|| {
    dom.par_iter_subtrees()
        .for_each(|subtree| resolve_styles(subtree));
});
// Main thread resumes here after all subtrees complete
```

This avoids the main thread sitting idle while rayon workers do all the work. The main thread is the most valuable core — it should contribute during parallel phases.

## 6.4 Web Workers

### 6.4.1 Thread Model

Each Dedicated Worker gets its own OS thread with its own JS/Wasm execution context:

```rust
pub struct DedicatedWorkerThread {
    /// Own ScriptEngine instance (separate JS heap)
    script_engine: ScriptEngine,
    /// Own WorkerSession (no DOM access — Workers have no DOM)
    session: WorkerSession,
    /// Message channel to parent (main thread or another worker)
    port: MessagePort,
    /// Optional: SharedArrayBuffer mappings
    shared_buffers: Vec<SharedArrayBuffer>,
}
```

| Worker Type | Thread Model | Lifetime | DOM Access |
| --- | --- | --- | --- |
| Dedicated Worker | 1 OS thread per Worker | Tied to creating document | No |
| Shared Worker | 1 OS thread per Worker | Tied to origin (persists across tabs) | No |
| Service Worker | 1 OS thread, event-driven | Per-origin, activated on demand, idle timeout | No |

**Why 1:1 threading (not thread pool):** Workers can run indefinitely (game loops, real-time processing). A thread pool with a fixed size would cause deadlocks if all pool threads run long-lived Workers and short tasks cannot execute.

### 6.4.2 postMessage and Structured Clone

Worker communication uses `postMessage`, which transfers data via structured clone serialization:

```
Main Thread                     Worker Thread
  │                                │
  │  postMessage(data)             │
  │  ├─ structured clone serialize │
  │  ├─ send(bytes) ──────────▶   │
  │                                ├─ structured clone deserialize
  │                                ├─ deliver MessageEvent
```

Transferable objects (ArrayBuffer, MessagePort, ImageBitmap) are zero-copy — ownership is moved, not cloned. The sending side loses access.

The structured clone format is shared with IPC serialization (Ch. 5) and IndexedDB value storage (Ch. 22), avoiding redundant serialization implementations.

### 6.4.3 SharedArrayBuffer and Atomics

SharedArrayBuffer enables true shared memory between threads. Gated by security requirements:

| Requirement | Reason |
| --- | --- |
| Cross-Origin-Opener-Policy: same-origin | Prevents Spectre-style attacks via high-resolution timers |
| Cross-Origin-Embedder-Policy: require-corp | Ensures all subresources opt in to cross-origin loading |
| Secure context (HTTPS) | Basic security requirement |

When both COOP and COEP are set, SharedArrayBuffer is available. Multiple Workers (and the main thread) can map the same underlying memory:

```rust
pub struct SharedArrayBuffer {
    /// Shared memory region, accessible from multiple threads
    memory: Arc<SharedMemory>,
    /// Byte length
    len: usize,
}

pub struct SharedMemory {
    /// mmap'd region or aligned allocation
    ptr: *mut u8,
    len: usize,
}

// Safety: SharedMemory is explicitly designed for concurrent access.
// All accesses must use Atomics (JS) or atomic operations (Wasm).
unsafe impl Send for SharedMemory {}
unsafe impl Sync for SharedMemory {}
```

Atomics operations (Atomics.wait, Atomics.notify, Atomics.compareExchange, etc.) map to CPU atomic instructions. Atomics.wait suspends the calling thread (using OS futex or equivalent), Atomics.notify wakes waiting threads.

**Atomics.wait is forbidden on the main thread** — it would block the event loop. Only Workers can call Atomics.wait.

### 6.4.4 OffscreenCanvas

OffscreenCanvas allows Workers to render to a canvas without main thread involvement:

```
Main Thread                     Worker Thread
  │                                │
  │  canvas.transferControlToOffscreen()
  │  ├─ ownership of drawing surface ──▶
  │                                │
  │                                ├─ ctx = offscreen.getContext('2d')
  │                                ├─ ctx.drawImage(...)
  │                                ├─ offscreen.commit()
  │                                │     └─ frame submitted to compositor
```

The committed frame goes directly to the compositor thread (not through the main thread), enabling Worker-driven rendering at 60fps independent of main thread load. The compositor integrates OffscreenCanvas output as a layer alongside DOM-rendered layers.

## 6.5 ECS Concurrency Model

### 6.5.1 Component Access Rules

The ECS stores all DOM and rendering state as components on entities. Different threads access different components with different permissions:

| Component Category | Main Thread | rayon Pool | Compositor | Workers |
| --- | --- | --- | --- | --- |
| DOM structure (Parent, Children, NextSibling) | Read/Write | Read (during style) | No access | No access |
| Attributes | Read/Write | Read (during style) | No access | No access |
| ComputedStyle | Write (via flush), Read | **Write** (parallel style) | Read (B: via DL, C: direct) | No access |
| LayoutResult (position, size) | Write (layout), Read | **Write** (parallel layout) | Read (B: via DL, C: direct) | No access |
| Layer (compositing hints) | Write (paint) | Write (parallel paint) | Read (B: via DL, C: direct) | No access |
| ScrollState | Read/Write | No access | **Write** (compositor scroll) | No access |
| AnimationState | Read/Write | No access | **Write** (compositor anim) | No access |

### 6.5.2 Synchronization Strategy

The key insight is that **different thread classes access ECS at different phases of the event loop**, creating natural non-overlapping windows:

```
Event Loop Phase          Active Threads on ECS
─────────────────────────────────────────────────
Phase 1: Collect events   Main (read IPC)         Compositor (independent)
Phase 2: JS execution     Main (read/write via SS) Compositor (independent)
Phase 2: Session flush    Main (write components)  Compositor (independent)
Phase 3a: Style           rayon (write styles)     Compositor (read old frame)
Phase 3b: Layout          rayon (write layout)     Compositor (read old frame)
Phase 3c: Paint           Main+rayon (write layers) Compositor (read old frame)
Phase 3d: Commit          Main (send DisplayList)  Compositor (activate new frame)
Phase 4: Idle             Main (read/idle work)    Compositor (independent)
Phase 5: Wait             — sleeping —             Compositor (independent)
```

In Approach B, the compositor operates on its own copy and never accesses ECS directly. Contention is zero.

In Approach C, the compositor reads ECS components. The potential conflict is between Phase 3 (rayon writing) and compositor reading. This is resolved by:

1. **Double-buffered layer data:** Main thread writes to "back buffer" during Phase 3, atomically swaps to "front buffer" at commit (Phase 3d). Compositor always reads front buffer.
2. **Atomic scroll/animation state:** CompositorMutableState (Section 6.2.3) uses lock-free atomics. No contention.

### 6.5.3 Send/Sync Classification

```rust
// ECS World — NOT Send, NOT Sync. Owned by main thread.
pub struct EcsWorld { /* ... */ }

// Component storage — Send (can transfer between threads for rayon),
// but access coordinated by phase (not locks).
pub struct ComponentStorage<T: Component> { /* ... */ }
unsafe impl<T: Component + Send> Send for ComponentStorage<T> {}

// DisplayList — Send (transferred to compositor thread).
pub struct DisplayList { /* ... */ }
unsafe impl Send for DisplayList {}

// CompositorMutableState — Send + Sync (atomic operations).
pub struct CompositorMutableState { /* ... */ }
unsafe impl Send for CompositorMutableState {}
unsafe impl Sync for CompositorMutableState {}

// ScriptSession — NOT Send. Main-thread-only.
pub struct ScriptSession { /* ... */ }

// WorkerSession — Send. One per Worker thread.
pub struct WorkerSession { /* ... */ }
unsafe impl Send for WorkerSession {}
```

The EcsWorld is `!Send` — it cannot be moved to another thread. This is enforced at compile time. rayon access is granted explicitly through scoped references during parallel phases (the main thread passes `&mut` references into `rayon::scope()`).

## 6.6 Browser Process Thread Model

Simpler than the Renderer. tokio multi-thread runtime handles all I/O:

```
Browser Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  tokio runtime (multi-thread)                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ async tasks:                                                 │   │
│  │   ├─ IPC dispatch (receive from Renderers, Network, GPU)    │   │
│  │   ├─ Storage I/O (SQLite via spawn_blocking)                │   │
│  │   ├─ Process lifecycle management                            │   │
│  │   ├─ Extension host                                          │   │
│  │   └─ Profile management                                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  UI Thread (1)                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ Chrome rendering (egui/iced)                                 │   │
│  │ Platform event loop integration (winit)                      │   │
│  │ Menu, dialog, file picker dispatch                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

| Thread | Role | Notes |
| --- | --- | --- |
| UI thread | Chrome UI rendering, platform event loop | Must be the "main thread" on macOS (Cocoa requires UI on main thread). Receives processed results from tokio tasks. |
| tokio workers | Async I/O, IPC dispatch, task execution | Default: min(4, num_cpus). Handles all IPC message routing. |
| spawn_blocking pool | SQLite operations, file I/O | tokio's blocking pool. SQLite calls are synchronous; wrapping in `spawn_blocking` prevents blocking async workers. |

### 6.6.1 Storage I/O Pattern

All SQLite operations (Ch. 22) run on tokio's blocking pool to avoid starving async workers:

```rust
// Browser Process — handling storage request from Renderer
async fn handle_storage_request(
    req: StorageRequest,
    storage: Arc<OriginStorageManager>,
) -> StorageResponse {
    // Offload SQLite I/O to blocking thread
    tokio::task::spawn_blocking(move || {
        let conn = storage.connection(&req.origin, req.storage_type)?;
        conn.execute(&req.op)
    })
    .await
    .unwrap()
}
```

## 6.7 Network Process Thread Model

Designed for maximum I/O throughput:

```
Network Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  tokio runtime (multi-thread)                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ async tasks:                                                 │   │
│  │   ├─ HTTP client (hyper) — per-request async tasks          │   │
│  │   ├─ DNS resolution (DoH queries)                           │   │
│  │   ├─ TLS handshake (rustls async)                           │   │
│  │   ├─ WebSocket connections                                   │   │
│  │   ├─ IPC dispatch (requests from Renderers)                 │   │
│  │   ├─ Connection pool management                              │   │
│  │   └─ CORS / cookie / security header enforcement            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  spawn_blocking pool                                               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ HTTP cache SQLite operations                                 │   │
│  │ Response body file I/O (cache read/write)                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

The Network Process is almost entirely async. The only blocking operations are disk I/O for the HTTP cache, which runs on the blocking pool. hyper, rustls, and h3 are all async-native and run directly on tokio workers.

## 6.8 GPU Process Thread Model

```
GPU Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  GPU Thread (1)                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ wgpu device management                                       │   │
│  │ Receive display lists / composite frames from Renderers     │   │
│  │ Texture upload and management                                │   │
│  │ Rasterization (Vello)                                        │   │
│  │ Surface presentation (vsync)                                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  IPC Thread (1)                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ tokio current_thread                                         │   │
│  │ Receive IPC from Renderers and Browser                      │   │
│  │ Deserialize display lists                                    │   │
│  │ Queue work for GPU thread                                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

The GPU Process is deliberately simple: one thread for GPU work, one for IPC. GPU drivers are often not thread-safe, so all wgpu calls go through a single thread. When GPU process merges into the Renderer (Phase 4+), the GPU thread becomes the compositor thread.

## 6.9 Thread Affinity and Core Allocation

### 6.9.1 Overall Core Budget

On a typical 8-core machine with one active Renderer:

| Thread(s) | Cores | Affinity | Notes |
| --- | --- | --- | --- |
| Renderer main | 1 | Soft (OS-scheduled) | Highest priority thread in the system |
| Compositor | 1 | Soft | Second highest priority. Must never be starved. |
| rayon pool | 6 workers | Soft | Expands to fill available cores during parallel phases |
| Web Workers | Shared with rayon cores | Soft | Workers compete with rayon. Acceptable — Workers rarely CPU-saturate during render phases. |
| Browser tokio | 2–4 workers | Shared | I/O-bound, rarely CPU-intensive |
| Network tokio | 2–4 workers | Shared | I/O-bound |
| GPU thread | 1 | Soft | Often GPU-bound, not CPU-bound |

elidex does not pin threads to specific cores (hard affinity). OS scheduling is sufficient for most workloads. Thread priorities are set where the platform supports it:

```rust
// Platform-specific priority hints
fn set_compositor_priority() {
    #[cfg(target_os = "linux")]
    {
        // SCHED_FIFO or elevated nice value
        unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -5); }
    }
    #[cfg(target_os = "macos")]
    {
        // QoS: user-interactive (highest non-realtime)
        // Set via pthread_set_qos_class_self_np
    }
}
```

### 6.9.2 tokio and rayon Coexistence

tokio and rayon serve fundamentally different purposes and do not interfere:

| Aspect | rayon | tokio |
| --- | --- | --- |
| Workload type | CPU-bound (style, layout, decode) | I/O-bound (network, IPC, timers) |
| Scheduling | Work-stealing, fork-join | Async task scheduling, epoll/kqueue |
| Active during | Render phases (Phase 3) | All phases (especially Phase 1, 5) |
| Core usage pattern | Burst (100% CPU during parallel phases) | Low CPU (waiting on I/O most of the time) |

During render phases, rayon saturates available cores. tokio workers have little to do (no I/O completions during rendering). Outside render phases (JS execution, idle), rayon workers are idle and tokio handles I/O. The workloads are naturally complementary.

## 6.10 Staged Migration: B → C

The compositor abstraction (FrameSource trait) enables a staged migration aligned with the overall process model relaxation (Ch. 5):

| Phase | Process Model | Compositor Approach | FrameSource Impl |
| --- | --- | --- | --- |
| Phase 1–3 | SiteIsolation, separate GPU Process | **B (message passing)** | DisplayListFrameSource |
| Phase 4 (transition) | PerTab or Shared, GPU merged into Renderer | B (same-process channel) | DisplayListFrameSource (LocalChannel) |
| Phase 4+ (optimization) | SingleProcess or Shared | **C (shared ECS)** | EcsFrameSource |

The B → C transition requires:

1. **Add double-buffered layer components to ECS.** Paint writes to back buffer; atomic swap at commit.
2. **Add CompositorMutableState.** Atomic scroll offsets and animation ticks, shared between main and compositor threads.
3. **Swap FrameSource implementation.** Configuration change, no architectural rewrite.
4. **Benchmark.** C eliminates DisplayList serialization cost but introduces atomic read overhead. Profile to verify net benefit.

Step 4 is important — C is not guaranteed to be faster than B for all workloads. Pages with many small layers may benefit from C (avoiding serialization of many small structures). Pages with a few large layers may not see a difference. The FrameSource abstraction allows A/B testing between approaches.

## 6.11 Summary: Thread Map

```
┌─ Browser Process ─────────────────────────────┐
│  UI thread ──── chrome rendering               │
│  tokio pool ─── IPC, storage I/O, management   │
│  blocking pool ─ SQLite operations              │
└────────────────────────────────────────────────┘
         │ IPC
┌─ Renderer Process (× N) ──────────────────────┐
│  Main thread ── event loop, DOM, script, paint │
│  Compositor ─── layers, scroll, GPU submit     │
│  rayon pool ─── style, layout, decode          │
│  Worker threads ── Web Workers (1:1 mapping)   │
│  tokio reactor ── I/O polling (on main thread) │
└────────────────────────────────────────────────┘
         │ IPC
┌─ Network Process ─────────────────────────────┐
│  tokio pool ─── HTTP, DNS, TLS, WebSocket      │
│  blocking pool ─ cache I/O                      │
└────────────────────────────────────────────────┘
         │ IPC
┌─ GPU Process ─────────────────────────────────┐
│  GPU thread ─── wgpu, rasterize, present       │
│  IPC thread ─── receive display lists          │
└────────────────────────────────────────────────┘
```
