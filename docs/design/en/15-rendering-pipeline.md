
# 15. Rendering Pipeline

## 15.1 ECS-Based DOM

The DOM is stored as an Entity Component System rather than a traditional object-oriented tree. Each DOM node is an entity (integer ID) with components stored in contiguous typed arrays:

| Component | Contents |
| --- | --- |
| TreeRelation | Parent, first child, next sibling, previous sibling indices |
| TagType | Element kind enum (dispatches to HtmlElementHandler plugin) |
| Attributes | Key-value attribute storage |
| ComputedStyle | Resolved CSS properties (output of StyleSystem) |
| LayoutBox | Position, size, margins, padding (output of LayoutSystem) |
| PaintData | Display list fragment (output of PaintSystem) |
| Accessibility | ARIA role, accessible name, relations (consumed by a11y tree builder) |

This layout ensures that systems processing the same component type (e.g., StyleSystem reading Attributes and writing ComputedStyle) iterate over contiguous memory, maximizing L1/L2 cache hit rates.

## 15.2 Parallel Pipeline

The rendering pipeline proceeds in stages. Style resolution and layout computation are parallelized using rayon, following Servo's proven approach:

```
HTML bytes
  │  [Parse]            ─ Sequential (streaming, incremental)
  ▼
DOM (ECS)
  │  [Script Execution] ─ JS/Wasm modifies DOM/CSSOM via ScriptSession
  │  [Session Flush]    ─ Mutation Buffer → ECS component writes (batched)
  │  [StyleSystem]      ─ Parallel per subtree (rayon)
  ▼
Styled DOM
  │  [LayoutSystem]     ─ Parallel for independent subtrees
  ▼
Layout Tree
  │  [PaintSystem]      ─ Parallel per layer
  ▼
Display List + Layer Tree
  │  [Compositor]       ─ Compositor thread (Ch. 6)
  ▼
Vello Scene
  │  [GPU]              ─ wgpu + Vello compute shaders
  ▼
Pixels on screen
```

The ScriptSession flush is the boundary between the script world and the rendering pipeline. All DOM and CSSOM mutations made during script execution are buffered in the session and applied to ECS components in a single batch at flush time. This ensures that the rendering pipeline never sees a partially-mutated DOM, and that mutation observers receive consistent records.

Because elidex eliminates quirks mode and legacy layout algorithms, each stage has significantly less branching than existing engines, improving both single-threaded throughput and parallel scaling.

## 15.3 Compatibility Layer Integration

In browser mode, the compatibility layer sits before the core as a normalization phase. The core never sees legacy HTML:

```
[Browser mode]
  Raw HTML ───▶ elidex-compat (normalize) ───▶ Clean HTML5 ───▶ elidex-core

[App mode]
  HTML5 ───▶ elidex-core (directly)
```

The compat layer acts as a transpiler: it converts deprecated tags to their HTML5 equivalents, resolves vendor prefixes, and transcodes legacy character encodings to UTF-8. If the input is already clean HTML5, the compat layer is a fast pass-through.

## 15.4 Layer Tree

### 15.4.1 Layer Tree as Independent Structure

The layer tree is a standalone data structure, separate from the ECS DOM. The PaintSystem reads ECS components (ComputedStyle, LayoutBox) and constructs the layer tree as output:

```
ECS DOM (entities + components)
  │
  │  [PaintSystem reads LayoutBox, ComputedStyle, etc.]
  ▼
LayerTree (independent structure, owned by main thread)
  │
  │  [Serialized as DisplayList for Approach B (Ch. 6)]
  │  [or shared via Arc for Approach C]
  ▼
Compositor Thread
```

The layer tree is not an ECS component because DOM nodes and layers have a fundamentally different structure. Multiple DOM nodes may be squashed into a single layer; a single DOM node may produce multiple layers (e.g., a scrollable element creates both a container layer and a content layer). The N:M relationship between entities and layers does not fit the ECS model of one component per entity.

```rust
pub struct LayerTree {
    layers: Vec<Layer>,
    root: LayerId,
}

pub struct Layer {
    pub id: LayerId,
    pub parent: Option<LayerId>,
    pub children: Vec<LayerId>,
    /// Content bounds in layer-local coordinates
    pub bounds: Rect,
    /// Transform from layer-local to parent coordinates
    pub transform: Transform3D,
    pub opacity: f32,
    pub clip: Option<ClipRegion>,
    pub blend_mode: BlendMode,
    /// Scroll offset (updated by compositor for scrollable layers)
    pub scroll_offset: ScrollOffset,
    /// Whether this layer needs its own offscreen buffer
    pub needs_surface: bool,
    pub content: LayerContent,
}

pub enum LayerContent {
    /// Painted DOM content (display list commands)
    DisplayList(DisplayListSlice),
    /// Video frame (GPU texture reference)
    VideoFrame(TextureHandle),
    /// Canvas 2D / WebGL / WebGPU output
    Canvas(TextureHandle),
    /// OffscreenCanvas from Worker (Ch. 6)
    OffscreenCanvas(TextureHandle),
}
```

### 15.4.2 Layer Promotion Criteria

Elements are promoted to their own layer based on these triggers:

| Trigger | Condition | Reason |
| --- | --- | --- |
| Explicit hint | `will-change: transform`, `will-change: opacity` | Developer declares intent for compositor animation. Layer pre-allocated to avoid promotion jank. |
| Active animation | `transform` or `opacity` being animated | Compositor can interpolate without main thread. |
| Fixed/Sticky positioning | `position: fixed` or `position: sticky` | Moves independently during scroll. |
| Scrollable overflow | `overflow: auto/scroll` with overflowing content | Scroll content is a separate layer for compositor-driven scroll. |
| Video | `<video>` | Decoded frames are GPU textures. Direct compositing. |
| Canvas | `<canvas>` (2D, WebGL, WebGPU) | Rendering output is already a GPU texture. |
| Isolation | `isolation: isolate`, non-normal `mix-blend-mode` | Requires isolated surface for correct blending. |
| CSS filters | `filter`, `backdrop-filter` | Offscreen rendering for filter effects. |
| Clip path / Mask | `clip-path`, `mask` | Stencil buffer or offscreen pass. |
| 3D transform | `transform` with perspective or Z component | Depth sorting in 3D. |

Not all stacking contexts become layers. The PaintSystem uses heuristics to balance compositing flexibility against GPU memory cost.

### 15.4.3 Layer Explosion Prevention

Each layer consumes a GPU texture. Excessive layer promotion wastes VRAM:

| Heuristic | Strategy |
| --- | --- |
| Squashing | Adjacent elements in the same stacking context that don't individually need layers are painted into their parent layer. |
| Overlap handling | An element overlapping a composited layer may be promoted for z-order correctness. Trivial overlaps prefer squashing. |
| Memory budget | Total layer texture memory is tracked. Default budget: 256MB. When approaching budget, low-priority layers (no active animation or scroll) are squashed back. |
| will-change throttling | More than 20 `will-change` elements per page triggers a console warning. Engine may decline promotion if budget is exceeded. |

### 15.4.4 Invalidation

When DOM or style changes occur, only affected layers need repainting:

```rust
pub struct Invalidation {
    pub layer_id: LayerId,
    /// Dirty rectangle within the layer
    pub dirty_rect: Rect,
    pub cause: InvalidationCause,
}

pub enum InvalidationCause {
    StyleChange,
    LayoutShift,
    ContentChange,
    SubtreeChange,
}
```

Unchanged layers are recomposited from their cached textures. Only the dirty rectangle within affected layers is re-rasterized.

## 15.5 Display List

The display list is the intermediate representation between paint and rasterization. It encodes drawing commands in a compact, serializable format.

### 15.5.1 Commands

```rust
pub enum DisplayItem {
    Rect { bounds: Rect, color: Color },
    Border { bounds: Rect, widths: SideOffsets, styles: BorderStyles, colors: BorderColors },
    Text { glyphs: Vec<GlyphInstance>, font: FontKey, color: Color },
    Image { bounds: Rect, key: ImageKey, rendering: ImageRendering },
    BoxShadow { bounds: Rect, shadow: BoxShadowParams },
    Gradient { bounds: Rect, gradient: GradientParams },
    PushClip { clip: ClipRegion },
    PopClip,
    PushTransform { transform: Transform3D },
    PopTransform,
    PushOpacity { opacity: f32 },
    PopOpacity,
    PushFilter { filter: FilterParams },
    PopFilter,
}
```

Each layer produces a `DisplayListSlice` — a contiguous range of `DisplayItem`s. Fonts and images are referenced by key (FontKey, ImageKey); actual data is managed separately.

### 15.5.2 Serialization

In Approach B (Ch. 6), the display list is serialized and sent to the compositor via the DisplayPipeline channel (postcard format, matching IPC serialization):

| Optimization | Strategy |
| --- | --- |
| Key interning | Fonts and images referenced by lightweight key. Actual GPU data uploaded separately. |
| Delta updates | Only invalidated layers' display lists are re-sent. Unchanged layers referenced by previous frame's ID. |
| Arena allocation | Display items allocated from per-frame arena. Entire arena dropped at end of frame. |

## 15.6 GPU Rendering: Vello on wgpu

### 15.6.1 Architecture

Elidex uses a two-layer GPU stack:

| Layer | Library | Role |
| --- | --- | --- |
| GPU abstraction | wgpu | Cross-platform GPU API. Targets Vulkan, Metal, DX12, WebGPU. |
| 2D rendering | Vello | GPU-accelerated 2D vector renderer. Path flattening, area computation, and tile binning via compute shaders on wgpu. |

Vello is the primary renderer for all DOM content. Unlike CPU-side rasterizers, Vello performs rasterization entirely on the GPU. This gives significant performance advantages for complex vector content (rounded corners, gradients, large text, SVG).

**Vello is a direct dependency, not abstracted behind a trait.** Unlike hyper (Ch. 10 HttpTransport) or SQLite (Ch. 22 StorageBackend), Vello has no viable Rust-native alternative for GPU 2D rendering. Defining a trait would amount to replicating Vello's scene API with no practical benefit. Instead, the contact surface is isolated: only the DisplayList-to-Vello-Scene conversion layer touches Vello's API directly. The layer tree, display list, and all upstream code (ECS, style, layout, paint) have no Vello dependency. If a credible alternative emerges in the future, the conversion layer is the only code that needs replacement. (See ADR #26.)

### 15.6.2 DisplayList → Vello Scene Conversion

The compositor thread converts display list commands to Vello's scene graph. This is the sole contact point with the Vello API:

```rust
/// The only module that imports vello types.
mod vello_backend {
    pub fn build_scene(layer_tree: &LayerTree) -> vello::Scene {
        let mut scene = vello::Scene::new();

        for layer in layer_tree.iter_back_to_front() {
            scene.push_layer(
                blend_to_vello(layer.blend_mode),
                layer.opacity,
                transform_to_vello(layer.transform),
                &bounds_to_vello(layer.bounds),
            );

            for item in layer.display_items() {
                match item {
                    DisplayItem::Rect { bounds, color } => {
                        scene.fill(
                            vello::Fill::NonZero,
                            vello::Affine::IDENTITY,
                            color_to_vello(*color),
                            None,
                            &rect_to_kurbo(*bounds),
                        );
                    }
                    DisplayItem::Text { glyphs, font, color } => {
                        // Vello renders glyph outlines directly on GPU.
                        // No CPU-side glyph rasterization for vector text.
                        scene.draw_glyphs(font_to_vello(*font))
                            .brush(color_to_vello(*color))
                            .draw(glyphs_to_vello(glyphs));
                    }
                    DisplayItem::Image { bounds, key, .. } => {
                        scene.draw_image(&texture_for_key(*key), *bounds);
                    }
                    // ... other items map similarly
                }
            }

            scene.pop_layer();
        }

        scene
    }
}
```

### 15.6.3 GPU Rendering Pipeline

```
DisplayList (per layer)
  │
  ├─ vello_backend::build_scene()
  │   └─ Vello Scene (paths, fills, strokes, glyphs, images)
  │
  ├─ Vello encoding (CPU)
  │   └─ Encodes scene into GPU buffers (path segments, styles, transforms)
  │
  ├─ Vello compute passes (GPU)
  │   ├─ Path flattening (compute shader)
  │   ├─ Tile binning (compute shader)
  │   └─ Fine rasterization (compute shader)
  │
  ├─ Layer compositing (render pass)
  │   └─ Composite rasterized layers with transforms, clips, opacity, blend
  │
  └─ Present to RenderSurface (Ch. 23)
```

### 15.6.4 Software Fallback

On systems without adequate GPU support (headless servers, CI, some VMs), Vello's CPU backend is used:

```rust
pub enum RenderBackend {
    Gpu(wgpu::Device, vello::Renderer),
    Software(vello::cpu::Renderer),
}
```

Selection is automatic (fall back if wgpu device creation fails) or forced via configuration. Software mode is also useful for deterministic testing — GPU rendering is not bit-exact across drivers.

## 15.7 Texture Management

### 15.7.1 Staged Approach

Texture management evolves in phases:

| Phase | Strategy | When |
| --- | --- | --- |
| Phase 1 | Individual textures | Initial implementation. Each decoded image and canvas output gets its own wgpu::Texture. Simple, correct, easy to debug. |
| Phase 2 | Individual textures + atlas for small images | When profiling shows texture switching overhead. Small images (<256×256) and color emoji packed into atlas textures. Large images remain individual. |
| Phase 3 | Unified texture management | When elidex needs full GPU memory budget control across tabs. All texture allocation flows through a central allocator with eviction policy. |

Phase 1 is sufficient for initial development. The API through which the rest of the engine interacts with textures (TextureHandle, ImageKey) is opaque and remains stable across all phases.

### 15.7.2 Texture Handle

All code outside the GPU module references textures by opaque handle:

```rust
/// Opaque handle to a GPU texture. The internal representation
/// changes across texture management phases.
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct TextureHandle(u64);

pub struct TextureManager {
    textures: HashMap<TextureHandle, GpuTexture>,
    next_handle: AtomicU64,
}

impl TextureManager {
    /// Upload decoded image data to GPU. Returns handle for display list reference.
    pub fn upload_image(&mut self, data: &DecodedImage) -> TextureHandle { /* ... */ }

    /// Release GPU texture (image no longer referenced by any layer).
    pub fn release(&mut self, handle: TextureHandle) { /* ... */ }

    /// Get wgpu::TextureView for rendering. Called by vello_backend.
    pub fn get_view(&self, handle: TextureHandle) -> &wgpu::TextureView { /* ... */ }
}
```

### 15.7.3 CPU→GPU Transfer

```rust
pub struct StagingBelt {
    /// Reusable staging buffers for CPU→GPU transfer
    buffers: Vec<wgpu::Buffer>,
}

impl StagingBelt {
    /// Schedule texture upload. Non-blocking — actual transfer happens
    /// when the command encoder is submitted to the GPU queue.
    pub fn upload_texture(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        data: &[u8],
        region: TextureRegion,
    ) { /* ... */ }
}
```

| Content Type | Upload Timing | Notes |
| --- | --- | --- |
| Decoded images | On decode completion (async) | rayon pool decodes (Ch. 6), then staging buffer upload. Non-blocking. |
| Canvas 2D output | Per frame | CPU bitmap → staging buffer. OffscreenCanvas (Worker) may already be GPU texture. |
| Video frames | Per video frame | Zero-copy where platform supports hardware decoder → GPU texture. Otherwise staging buffer. |
| Color emoji | On first use | Rasterized on CPU, uploaded to texture (or atlas in Phase 2). |

### 15.7.4 GPU Memory Tracking

```rust
pub struct GpuMemoryTracker {
    allocated: AtomicU64,
    /// Default: 512MB, capped at 75% of reported VRAM
    budget: u64,
}
```

When approaching budget: evict unused image textures first (coordinated with image decode cache, Ch. 22), then reduce layer texture resolution (render at lower DPI and upscale). Tab discarding (Ch. 22, memory pressure) releases all GPU resources for discarded tabs.

## 15.8 Frame Scheduling

### 15.8.1 Frame Policy

The frame scheduler is driven by a configurable policy that accommodates both browser and application use cases:

```rust
pub enum FramePolicy {
    /// Sync to display vsync. Skip frames if no visual changes pending.
    /// Default for elidex-browser.
    Vsync,

    /// Render every vsync opportunity unconditionally.
    /// For games, real-time visualization, continuous animation.
    Continuous,

    /// Render only when content has changed (dirty flag).
    /// Maximum power efficiency. Default for elidex-app.
    OnDemand,

    /// Fixed frame rate, independent of display vsync.
    FixedRate(u32),  // e.g., 30
}
```

| Policy | Compositor Behavior | Use Case |
| --- | --- | --- |
| Vsync | Wake on vsync. Composite if new DisplayList or pending scroll/animation. Sleep otherwise. | Browser tabs, most web content. |
| Continuous | Wake on every vsync. Always composite (even if no changes). | Games, WebGL/WebGPU apps, real-time dashboards. |
| OnDemand | Sleep until dirty flag is set (DOM change, scroll, animation start, resize). Then render one frame and sleep again. | Document viewers, form apps, electron-style tools. Power-efficient. |
| FixedRate(n) | Wake on timer (1/n seconds). Composite regardless of vsync. | Digital signage, video playback at specific rate, testing. |

```rust
// elidex-app configuration
let app = elidex_app::App::new()
    .frame_policy(FramePolicy::OnDemand)   // Power-efficient default
    .build();

// Game app overrides
let game = elidex_app::App::new()
    .frame_policy(FramePolicy::Continuous)  // Maximum frame rate
    .build();
```

In elidex-browser, the policy is per-tab: active foreground tabs use Vsync, background tabs are automatically downgraded to OnDemand (no rendering until the tab is activated).

### 15.8.2 Pipeline Structure

The main thread and compositor thread operate in pipeline: the compositor draws frame N while the main thread prepares frame N+1.

```
Time ──────────────────────────────────────────────────►

Main thread:   [──── Frame N ────][──── Frame N+1 ────][──── Frame N+2 ────]
                JS│Style│Layout│Paint  JS│Style│Layout│Paint  ...
                         │ send DL          │ send DL
                         ▼                  ▼
Compositor:         [── Frame N ──]    [── Frame N+1 ──]
                     Recv│Composite│Present  Recv│Composite│Present
                              │                      │
Display:                    vsync                  vsync
```

This pipelining adds one frame of latency (the compositor always draws the previous frame's output) but significantly improves throughput — the main thread never waits for the compositor to finish, and the compositor never waits for the main thread.

The bounded(1) DisplayPipeline channel (Ch. 6) provides the synchronization: if the compositor hasn't consumed the previous frame, the main thread drops the new frame rather than queuing. This prevents unbounded latency accumulation.

### 15.8.3 Variable Refresh Rate

The frame scheduler adapts to the display's refresh rate via DisplayCapabilities (Ch. 23, RenderSurface):

```rust
pub struct DisplayCapabilities {
    pub refresh: RefreshRate,
    pub hdr: bool,
    pub color_space: ColorSpace,
    pub scale_factor: f64,
}

pub enum RefreshRate {
    Fixed(u32),                        // e.g., 60
    Variable { min: u32, max: u32 },   // e.g., 48–120 (ProMotion, FreeSync, G-Sync)
}
```

| Display Type | Frame Budget | Behavior |
| --- | --- | --- |
| 60Hz fixed | 16.67ms | Standard. Frame skip if budget exceeded. |
| 120Hz fixed | 8.33ms | Half the budget. Style/layout/paint must be faster. |
| 48–120Hz VRR | Variable | Present as soon as ready. Display adapts refresh. Frames between 8.33ms–20.83ms all display smoothly. |

For VRR displays, the compositor presents immediately when a frame is ready rather than waiting for a fixed vsync interval. This eliminates both tearing (VRR handles this) and artificial latency from waiting.

### 15.8.4 Frame Timing and Jank Detection

Every frame records timing data:

```rust
pub struct FrameTiming {
    pub frame_id: u64,
    pub policy: FramePolicy,
    pub target_interval: Duration,
    pub style_duration: Duration,
    pub layout_duration: Duration,
    pub paint_duration: Duration,
    pub composite_duration: Duration,
    pub total_main_thread: Duration,
    pub total_compositor: Duration,
    pub presented_at: Instant,
    /// True if this frame missed its target
    pub janked: bool,
}
```

Long frames (exceeding budget by >50%) are reported as Long Animation Frame entries via the Performance Observer API (Ch. 14). This enables web developers to identify jank sources.

## 15.9 Compositor-Driven Operations

These operations run on the compositor thread (Ch. 6) without main thread involvement, remaining smooth even during heavy JS execution.

### 15.9.1 Compositor Animations

| Property | Compositor-Only | Notes |
| --- | --- | --- |
| `transform` | Yes | Layer transform. No layout or repaint. |
| `opacity` | Yes | Layer alpha. No repaint. |
| `filter` (subset) | Yes | GPU shader effect on layer texture. |
| `background-color` | No | Requires repaint. |
| `width`, `height` | No | Requires layout + repaint. |
| `top`, `left` | No | Requires layout. Use `transform: translate()` for compositor path. |

```rust
pub struct CompositorAnimation {
    pub target_layer: LayerId,
    pub property: AnimatableProperty,
    pub keyframes: Vec<Keyframe>,
    pub timing: AnimationTiming,
    pub current_time: f64,
}

pub enum AnimatableProperty {
    Transform(Transform3D, Transform3D),
    Opacity(f32, f32),
    Filter(FilterList, FilterList),
}
```

When an animation targets a compositor-animatable property, the main thread promotes the element to its own layer (if not already) and sends the animation definition to the compositor. The compositor interpolates values per-frame independently.

If JS interrupts a compositor animation (e.g., `element.style.transform = ...`), the compositor animation is cancelled and control returns to the main thread.

### 15.9.2 Compositor-Driven Scroll

Scroll is the most common compositor operation:

```
User scrolls (touch/wheel)
  │
  ├─ Compositor receives input (Ch. 6 fast path)
  ├─ Hit test: find innermost scrollable layer
  ├─ Apply scroll physics (momentum, friction, overscroll bounce)
  ├─ Update scroll offset on layer
  │   ├─ Translate scroll content layer
  │   ├─ position:fixed layers unaffected
  │   ├─ position:sticky layers clamped to range
  │   └─ Scrollbar position updated
  ├─ Composite and present (no main thread)
  └─ Async notify main thread of new scroll position
      └─ Main thread fires scroll events, triggers lazy loading
```

For passive scroll listeners (or no listener), this flow is compositor-only. For non-passive listeners, the compositor scrolls optimistically and the main thread can preventDefault() — adding one frame of latency.

### 15.9.3 Scroll-Linked Effects

| Feature | Compositor Handling |
| --- | --- |
| `position: fixed` | Layer not translated during scroll. |
| `position: sticky` | Layer position clamped between normal position and threshold. |
| `background-attachment: fixed` | Background layer doesn't scroll with content. |
| `scroll-snap` | Snap points applied during scroll deceleration. |
| `overscroll-behavior` | `contain` prevents chaining, `none` prevents chaining + overscroll effect. |

## 15.10 Platform Integration

### 15.10.1 RenderSurface Connection

The pipeline connects to the platform through RenderSurface (Ch. 23):

| Platform | Surface | GPU Backend |
| --- | --- | --- |
| macOS | CAMetalLayer | Metal |
| Linux (Wayland) | wl_surface | Vulkan |
| Linux (X11) | X11 Window | Vulkan |
| Windows | HWND | DX12 or Vulkan |
| Headless | Offscreen texture | Software (Vello CPU) |

### 15.10.2 DPI Scaling

All layout uses logical (CSS) pixels. The platform's scale factor is applied at rasterization:

```
CSS pixels × scale_factor → physical pixels → Vello renders at physical resolution
```

Layer textures are allocated at physical pixel dimensions for crisp HiDPI rendering. Scale factor changes (window moved between displays) trigger full relayout and texture reallocation.

## 15.11 elidex-app Rendering

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| Frame policy default | Vsync | OnDemand |
| GPU Process | Separate (Phase 1–3) | Merged (SingleProcess) |
| Layer budget | Managed | Higher default (app controls content) |
| Compositor approach | B (message passing) | C possible (shared LayerTree) |
| Software fallback | Automatic | Configurable (app may require GPU) |

```rust
let app = elidex_app::App::new()
    .frame_policy(FramePolicy::OnDemand)
    .render_backend(RenderBackend::Auto)
    .gpu_memory_budget(256 * 1024 * 1024)
    .build();
```
