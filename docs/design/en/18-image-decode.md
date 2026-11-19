
# 18. Image Decode Pipeline

## 18.1 Overview

Images are typically the heaviest resources on a web page. The decode pipeline must handle format detection, off-main-thread decoding, memory-efficient caching, progressive rendering, and GPU upload — all without blocking the rendering pipeline.

```
Network response (bytes)
  │
  ├─ Format detection (magic bytes)
  ├─ ImageDecoder selection (per format)
  ├─ Decode on rayon pool (Ch. 6)
  │   ├─ Progressive: yield partial frames as available
  │   └─ Full: yield complete DecodedImage
  ├─ Image decode cache (Ch. 22)
  │   └─ Key: (URL, target size, DPI)
  ├─ GPU upload via TextureManager (Ch. 15 §15.7)
  │   └─ StagingBelt for CPU→GPU transfer
  └─ Compositor draws texture in layer
```

## 18.2 Format Support

### 18.2.1 Core / Compat Classification

| Format | Classification | Decoder Crate | Notes |
| --- | --- | --- | --- |
| PNG | Core | png | Lossless. Ubiquitous. |
| JPEG | Core | zune-jpeg | Lossy. Photo standard. zune-jpeg for SIMD-optimized decode. |
| WebP | Core | webp | Lossy + lossless + animated. Supported by all major browsers. |
| AVIF | Core | ravif / libdav1d | AV1-based. Next-generation standard. |
| GIF | Core | gif | Animated GIF remains widespread. |
| APNG | Core | png (APNG extension) | Animated PNG. All major browsers support. |
| SVG-as-image | Core | (Ch. 19) | `<img src="icon.svg">` rasterized at target size. |
| ICO/CUR | Compat | ico | Favicon. Legacy but favicon.ico still common. |
| BMP | Compat | image (bmp module) | Rare on modern web. |
| TIFF | Compat | image (tiff module) | Extremely rare on web. |
| JPEG XL | Not supported | — | Browser support unstable (Chrome added then removed). Reconsider when spec stabilizes. |

In elidex-app mode, compat formats can be excluded at compile time, reducing binary size.

### 18.2.2 Format Detection

Format is determined by magic bytes (content sniffing), not file extension or MIME type:

```rust
pub fn detect_format(header: &[u8]) -> Option<ImageFormat> {
    match header {
        [0x89, b'P', b'N', b'G', ..] => Some(ImageFormat::Png),
        [0xFF, 0xD8, 0xFF, ..] => Some(ImageFormat::Jpeg),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some(ImageFormat::WebP),
        [b'G', b'I', b'F', b'8', ..] => Some(ImageFormat::Gif),
        [0x00, 0x00, 0x01, 0x00, ..] => Some(ImageFormat::Ico),
        [0x00, 0x00, 0x02, 0x00, ..] => Some(ImageFormat::Cur),
        _ if is_avif(header) => Some(ImageFormat::Avif),
        [b'B', b'M', ..] => Some(ImageFormat::Bmp),
        _ => None,
    }
}
```

## 18.3 ImageDecoder Trait

Decoders are abstracted behind a trait, allowing Rust-crate implementations to be swapped for platform-native decoders when performance requires it.

```rust
pub trait ImageDecoder: Send + Sync {
    /// Supported format.
    fn format(&self) -> ImageFormat;

    /// Decode image header to obtain dimensions and metadata
    /// without decoding pixel data.
    fn read_header(&self, data: &[u8]) -> Result<ImageHeader, DecodeError>;

    /// Full decode to RGBA bitmap.
    fn decode(&self, data: &[u8], params: DecodeParams) -> Result<DecodedImage, DecodeError>;

    /// Progressive decode: return partial result from available bytes.
    /// Returns None if not enough data for any useful output.
    fn decode_progressive(
        &self,
        data: &[u8],
        params: DecodeParams,
    ) -> Result<Option<PartialImage>, DecodeError>;

    /// For animated formats: decode frame at given index.
    fn decode_frame(
        &self,
        data: &[u8],
        frame_index: usize,
        params: DecodeParams,
    ) -> Result<DecodedFrame, DecodeError>;

    /// For animated formats: total frame count and frame delay table.
    fn animation_info(&self, data: &[u8]) -> Result<Option<AnimationInfo>, DecodeError>;
}

pub struct DecodeParams {
    /// Target size for downscaled decode (JPEG can decode at 1/2, 1/4, 1/8 natively).
    pub target_size: Option<Size>,
    /// Target color space.
    pub color_space: ColorSpace,
    /// DPI scale factor (for SVG-as-image rasterization).
    pub scale_factor: f64,
}

pub struct ImageHeader {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub has_alpha: bool,
    pub is_animated: bool,
    pub color_space: Option<ColorSpace>,
    pub exif_orientation: Option<Orientation>,
}

pub struct DecodedImage {
    pub pixels: Vec<u8>,       // RGBA8 or RGBA16 depending on source
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
    pub color_space: ColorSpace,
}
```

### 18.3.1 Default Implementations

The initial implementation uses Rust crates for all formats:

| Format | Crate | Notes |
| --- | --- | --- |
| PNG / APNG | `png` | Pure Rust. APNG frame extraction. |
| JPEG | `zune-jpeg` | SIMD-optimized. Supports 1/2, 1/4, 1/8 scale decode. |
| WebP | `webp` | Rust bindings to libwebp, or pure-Rust `webp` crate. |
| AVIF | `ravif` / `dav1d` | ravif for pure Rust; dav1d (C) for hardware-accelerated AV1 decode. |
| GIF | `gif` | Pure Rust. Streaming frame decode. |
| ICO | `ico` | Pure Rust. Embedded PNG or BMP. |
| BMP | `image` (bmp) | Pure Rust. |

### 18.3.2 Platform Decoder Option

When profiling reveals Rust decoders as a bottleneck (especially JPEG and AVIF on mobile), platform decoders can be substituted:

| Platform | Decoder | Advantage |
| --- | --- | --- |
| macOS / iOS | ImageIO (CGImageSource) | Hardware JPEG/HEIF decode on Apple Silicon. |
| Windows | WIC (Windows Imaging Component) | GPU-assisted decode. |
| Linux | Platform libraries vary | libjpeg-turbo with SIMD is common. |
| Android | Android Bitmap decoder | Hardware decode path. |

Platform decoders implement the same `ImageDecoder` trait. Selection is configurable:

```rust
pub enum DecoderStrategy {
    /// Rust crates only. Maximum portability, no C dependencies.
    RustOnly,
    /// Prefer platform decoders, fall back to Rust.
    PlatformPreferred,
    /// Use platform for specific formats only.
    Mixed(HashMap<ImageFormat, DecoderBackend>),
}
```

## 18.4 Decode Pipeline

### 18.4.1 Off-Main-Thread Decode

All image decoding runs on the Renderer's rayon pool (Ch. 6). The main thread never blocks on decode:

```
Main thread                          rayon pool
  │                                    │
  ├─ <img> parsed or src changed       │
  ├─ Request image from Network        │
  │  (via Browser Process IPC)         │
  │                                    │
  ├─ Bytes arrive (streaming)          │
  ├─ dispatch_decode(bytes) ──────────▶│
  │                                    ├─ detect_format()
  │                                    ├─ read_header()
  │  ◄── ImageHeader ─────────────────│  (intrinsic size for layout)
  │                                    │
  │  [layout uses intrinsic size]      ├─ decode() or decode_progressive()
  │                                    │
  │  ◄── DecodedImage ────────────────│
  ├─ Insert into decode cache          │
  ├─ Upload to GPU (TextureManager)    │
  ├─ Invalidate layer (repaint)        │
  └─ Compositor draws texture          │
```

### 18.4.2 Header-First Layout

The image header is read as soon as enough bytes arrive (typically the first few hundred bytes). This provides intrinsic dimensions for layout before the full image is decoded:

```rust
pub struct PendingImage {
    pub url: Url,
    pub state: ImageLoadState,
}

pub enum ImageLoadState {
    /// Waiting for network response.
    Loading,
    /// Header decoded. Intrinsic size known. Layout can proceed.
    HeaderReady {
        header: ImageHeader,
        bytes_so_far: Vec<u8>,
    },
    /// Partially decoded (progressive JPEG).
    Progressive {
        header: ImageHeader,
        partial: PartialImage,
        bytes_so_far: Vec<u8>,
    },
    /// Fully decoded and cached.
    Complete {
        cache_key: ImageCacheKey,
        texture: TextureHandle,
    },
    /// Decode or network error.
    Error(ImageError),
}
```

Without a header (network stall, invalid format), the `<img>` element either uses explicit `width`/`height` attributes (if provided) or renders as a placeholder. This is why the HTML spec recommends always specifying image dimensions.

### 18.4.3 Progressive JPEG Rendering

Progressive JPEGs deliver a blurry preview with the first scan, refining with subsequent scans. Elidex renders partial results as they arrive:

```
Scan 1 (DC coefficients): blurry full-size preview
  → Upload to GPU, display immediately
Scan 2: improved quality
  → Re-upload, invalidate layer
...
Final scan: full quality
  → Final upload, final invalidation
```

Each progressive update replaces the GPU texture. The compositor draws whatever is available. The user sees a progressively sharpening image rather than a blank space.

### 18.4.4 Downscaled Decode

When the rendered size is smaller than the intrinsic size, decoding at full resolution wastes memory and CPU time. JPEG supports native downscaled decode (1/2, 1/4, 1/8):

```rust
fn compute_decode_size(intrinsic: Size, rendered: Size, scale_factor: f64) -> Size {
    let physical = Size {
        width: (rendered.width * scale_factor).ceil() as u32,
        height: (rendered.height * scale_factor).ceil() as u32,
    };

    // For JPEG: find smallest native scale that covers the physical size
    // For other formats: decode at full size, downscale after
    physical
}
```

For non-JPEG formats, full decode followed by GPU-side or CPU-side downscale is used. The `image-rendering` CSS property selects the filter:

| `image-rendering` | Filter | Use Case |
| --- | --- | --- |
| `auto` (default) | Bilinear | General purpose. Smooth downscale. |
| `smooth` | Lanczos3 | High quality. Photos. |
| `pixelated` | Nearest-neighbor | Pixel art, retro graphics. |
| `crisp-edges` | Nearest-neighbor | Similar to pixelated. |

## 18.5 Responsive Images

### 18.5.1 Source Selection

The `srcset` and `<picture>` elements allow the browser to choose an appropriate image source based on viewport size, device pixel ratio, and art direction:

```html
<!-- Resolution switching -->
<img srcset="photo-320w.jpg 320w,
             photo-640w.jpg 640w,
             photo-1280w.jpg 1280w"
     sizes="(max-width: 600px) 100vw, 50vw"
     src="photo-640w.jpg"
     alt="Photo">

<!-- Art direction -->
<picture>
  <source media="(max-width: 600px)" srcset="photo-portrait.jpg">
  <source media="(min-width: 601px)" srcset="photo-landscape.jpg">
  <img src="photo-landscape.jpg" alt="Photo">
</picture>
```

Source selection algorithm:

```rust
pub fn select_source(
    srcset: &[SrcsetEntry],
    sizes: &SizesAttribute,
    viewport: &Viewport,
    device_pixel_ratio: f64,
) -> &SrcsetEntry {
    // 1. Evaluate sizes to get effective image width
    let effective_width = sizes.evaluate(viewport);

    // 2. Compute target density for each candidate
    // 3. Select candidate closest to device_pixel_ratio
    //    without going below it (prefer slightly larger)
    srcset.iter()
        .min_by_key(|entry| {
            let density = entry.width as f64 / effective_width;
            let diff = density - device_pixel_ratio;
            if diff < 0.0 { f64::MAX as i64 } else { (diff * 1000.0) as i64 }
        })
        .unwrap_or(&srcset[0])
}
```

### 18.5.2 Source Change on Resize

When the viewport or container size changes (window resize, container query), the source may need re-evaluation. If a higher-resolution source is now appropriate, a new fetch+decode is initiated. The old image remains displayed until the new one is ready (no flash of blank).

## 18.6 Lazy Loading

### 18.6.1 `loading="lazy"`

Images with `loading="lazy"` are not fetched until they enter or approach the viewport:

```rust
pub struct LazyLoadController {
    /// Distance from viewport edge to trigger loading.
    /// Default: 1250px vertical, 2500px horizontal (matches Chrome).
    pub root_margin: EdgeInsets,
}

impl LazyLoadController {
    pub fn should_load(&self, element_rect: Rect, viewport: Rect) -> bool {
        let expanded_viewport = viewport.expand(self.root_margin);
        expanded_viewport.intersects(&element_rect)
    }
}
```

The controller is driven by IntersectionObserver internally. When scroll position changes, the compositor notifies the main thread (Ch. 15 §15.9.2), which checks pending lazy images.

### 18.6.2 `loading="eager"` (default)

Eager images begin fetching immediately during parse. The preload scanner (Ch. 11) discovers `<img>` elements ahead of the parser and initiates early fetches.

### 18.6.3 `decoding` attribute

| Value | Behavior |
| --- | --- |
| `auto` (default) | Engine decides. Currently same as `async`. |
| `async` | Decode does not block rendering. Image appears when ready. |
| `sync` | Decode completes before the image is displayed. Blocks rendering for this element. Used when avoiding a flash of no-image is critical. |

## 18.7 Image Decode Cache

The image decode cache (Ch. 22 §22.7 memory caches) stores decoded bitmaps to avoid re-decoding when images are scrolled back into view or reused across elements.

### 18.7.1 Cache Key

```rust
#[derive(Hash, Eq, PartialEq)]
pub struct ImageCacheKey {
    pub url: Url,
    /// Decoded size (may differ from intrinsic if downscaled decode).
    pub decoded_size: Size,
    /// Scale factor at decode time.
    pub scale_factor: OrderedFloat<f64>,
}
```

The same URL at different sizes produces separate cache entries. A 100×100 thumbnail and a 1000×1000 hero image of the same URL are cached independently.

### 18.7.2 Eviction

```rust
pub struct ImageDecodeCache {
    entries: LinkedHashMap<ImageCacheKey, CacheEntry>,
    total_bytes: u64,
    /// Default: 128MB. Coordinated with GpuMemoryTracker (Ch. 15 §15.7.4).
    budget: u64,
}

pub struct CacheEntry {
    pub decoded: DecodedImage,
    pub texture: Option<TextureHandle>,  // GPU-uploaded copy
    pub last_access: Instant,
    pub byte_size: u64,
}
```

Eviction policy: LRU. When `total_bytes` exceeds `budget`, least-recently-used entries are evicted. Evicted entries release both CPU memory (decoded pixels) and GPU memory (texture via TextureManager).

Under system memory pressure (Ch. 22 memory pressure handling), the cache budget is temporarily reduced. Evicted images are re-decoded on demand from the HTTP cache or network.

### 18.7.3 Cache Coordination

```
HTTP cache (Ch. 22)        Image decode cache       GPU textures (Ch. 15)
  compressed bytes    →    decoded pixels       →    wgpu::Texture
  (disk/memory)            (memory, LRU evict)       (VRAM, evict with cache)
```

On eviction, the decode cache entry and its GPU texture are released together. If the image is needed again, it is re-decoded from the HTTP cache (avoiding a network fetch) and re-uploaded to GPU.

## 18.8 Animated Images

### 18.8.1 ImageAnimationScheduler

Animated images (GIF, APNG, animated WebP) have their own frame scheduling, independent of the CSS/JS animation system (Ch. 17). Each animated image has an `ImageAnimationScheduler` that manages frame timing:

```rust
pub struct ImageAnimationScheduler {
    frames: Vec<FrameInfo>,
    current_frame: usize,
    next_frame_time: Instant,
    play_count: u32,           // 0 = infinite
    completed_loops: u32,
    state: AnimPlayState,
}

pub struct FrameInfo {
    pub delay: Duration,       // Per-frame delay (from GIF/APNG metadata)
    pub dispose: DisposeOp,    // How to clear before next frame
    pub blend: BlendOp,        // How to composite onto previous frame
}

pub enum DisposeOp {
    None,            // Leave current frame
    Background,      // Clear to background color
    Previous,        // Restore to previous frame
}

impl ImageAnimationScheduler {
    /// Called each frame by the FrameProducer (Ch. 17).
    /// Returns true if a new frame should be decoded and displayed.
    pub fn tick(&mut self, now: Instant) -> Option<usize> {
        if self.state != AnimPlayState::Playing {
            return None;
        }

        if now >= self.next_frame_time {
            self.current_frame = (self.current_frame + 1) % self.frames.len();

            if self.current_frame == 0 {
                self.completed_loops += 1;
                if self.play_count > 0 && self.completed_loops >= self.play_count {
                    self.state = AnimPlayState::Finished;
                    return None;
                }
            }

            self.next_frame_time = now + self.frames[self.current_frame].delay;
            Some(self.current_frame)
        } else {
            None
        }
    }
}
```

### 18.8.2 Frame Decode Strategy

Animated images can have hundreds of frames. Decoding all frames upfront would consume excessive memory. Instead, frames are decoded on demand with a small lookahead buffer:

```rust
pub struct AnimatedImageBuffer {
    /// Raw compressed data (kept in memory for re-decode).
    source_data: Arc<Vec<u8>>,
    /// Decoded frame ring buffer. Typically 2–4 frames.
    decoded_frames: VecDeque<(usize, DecodedImage)>,
    /// Maximum decoded frames to keep in memory.
    buffer_size: usize,  // default: 3
}
```

The flow:
1. Scheduler determines next frame index.
2. If frame is in the buffer, use it directly.
3. If not, dispatch decode to rayon pool.
4. On decode completion, upload to GPU texture and invalidate layer.
5. Evict oldest frame from buffer if buffer is full.

For frames with `DisposeOp::Previous`, the previous frame must be retained.

### 18.8.3 Visibility Optimization

Animated images that are off-screen (scrolled out of viewport, in background tab) pause their animation. The FrameProducer skips their tick. When they scroll back into view, animation resumes from the current frame.

This is coordinated with the tab-level FramePolicy (Ch. 15 §15.8.1): background tabs use OnDemand mode, so animated images in background tabs consume zero CPU.

## 18.9 Blob URLs and Data URLs

### 18.9.1 Blob URLs

`URL.createObjectURL(blob)` creates a `blob:` URL referencing in-memory data. The image decode pipeline resolves blob URLs through the Browser Process (which owns the Blob store):

```
Renderer: <img src="blob:https://example.com/uuid">
  │  IPC: ResolveBlobUrl { url }
  ▼
Browser Process: BlobStore
  │  Returns blob data (or reference to disk-backed blob)
  ▼
Renderer: decode as normal image
```

### 18.9.2 Data URLs

`data:image/png;base64,...` URLs contain the image data inline. These are decoded directly without network fetch. Base64 decoding + image decoding both run on the rayon pool.

## 18.10 Error Handling

| Error | Behavior |
| --- | --- |
| Network error (404, timeout) | Display broken image icon. Fire `error` event on `<img>`. |
| Unsupported format | Display broken image icon. Console warning. |
| Corrupt image data | Display whatever was successfully decoded (partial), or broken image icon. Fire `error` event. |
| Decode OOM | Reduce decode resolution and retry. If still fails, broken image icon. |
| GPU upload failure | Fall back to software compositing for this image. |

The broken image icon is a built-in SVG rendered by the engine (not loaded from network).

## 18.11 elidex-app Images

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| Format support | Core + Compat | Core only (default). Compat opt-in at build time. |
| Decoder strategy | RustOnly (default) | Configurable (RustOnly / PlatformPreferred / Mixed) |
| Lazy loading | `loading="lazy"` attribute | Same API, but app controls viewport semantics |
| Image cache budget | Managed (128MB default) | Configurable |
| Animated images | Auto-play with visibility optimization | Same behavior. App can pause/resume via API. |

```rust
let app = elidex_app::App::new()
    .decoder_strategy(DecoderStrategy::PlatformPreferred)
    .image_cache_budget(64 * 1024 * 1024)  // 64MB for lightweight app
    .build();
```
