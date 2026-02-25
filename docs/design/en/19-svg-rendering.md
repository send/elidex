
# 19. SVG Rendering

## 19.1 Overview

SVG is ubiquitous in modern web content: icons, logos, illustrations, charts, and data visualization. Elidex must handle SVG in two distinct modes:

1. **Inline SVG**: `<svg>` elements embedded in the HTML DOM. Full DOM API access, CSS styling, event handling, scripting.
2. **SVG-as-image**: `<img src="icon.svg">`, CSS `background-image: url(icon.svg)`. Rendered to bitmap at target resolution. No DOM access, no scripting, restricted external resource loading.

These two modes share the SVG parser and Vello rendering path but differ in how they integrate with the rest of the engine.

```
[Inline SVG]
  HTML parser encounters <svg>
    → SVG elements become ECS entities (same DOM)
    → StyleSystem resolves CSS (including SVG presentation attributes)
    → SvgLayoutSystem computes SVG coordinate geometry
    → PaintSystem emits DisplayItems (paths, fills, strokes)
    → Vello renders via normal pipeline (Ch. 15)

[SVG-as-image]
  Image decode pipeline (Ch. 18) receives SVG data
    → SVG parser builds lightweight scene (no ECS)
    → Direct Vello Scene construction
    → Rasterize to offscreen texture at target size
    → Result cached as DecodedImage (bitmap)
```

## 19.2 Inline SVG: ECS Representation

### 19.2.1 SVG Elements as ECS Entities

SVG elements are stored in the same ECS as HTML elements. Each SVG element is an entity with the standard TreeRelation component (parent/child/sibling) plus SVG-specific components:

```rust
// Standard components (shared with HTML)
// - TreeRelation: tree structure
// - TagType: SvgRect, SvgCircle, SvgPath, etc.
// - Attributes: key-value attributes
// - ComputedStyle: resolved CSS (includes SVG properties like fill, stroke)

// SVG-specific components
pub struct SvgGeometry {
    pub shape: SvgShape,
    /// Computed bounding box in local coordinates
    pub bbox: Rect,
}

pub enum SvgShape {
    Rect { x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64 },
    Circle { cx: f64, cy: f64, r: f64 },
    Ellipse { cx: f64, cy: f64, rx: f64, ry: f64 },
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
    Polyline { points: Vec<(f64, f64)> },
    Polygon { points: Vec<(f64, f64)> },
    Path { data: PathData },
}

pub struct SvgTransform {
    pub matrix: Transform2D,
}

pub struct SvgViewport {
    pub view_box: Option<ViewBox>,
    pub preserve_aspect_ratio: PreserveAspectRatio,
    pub width: SvgLength,
    pub height: SvgLength,
}

pub struct SvgClipPath {
    pub clip_rule: FillRule,
    pub referenced_id: Option<String>,
}

pub struct SvgMask {
    pub referenced_id: String,
}

pub struct SvgFilter {
    pub filter_chain: SvgFilterGraph,
}
```

This unified representation means:

- `document.querySelector('svg rect')` works through the same ScriptSession DOM API.
- CSS selectors (`svg .highlight { fill: red; }`) apply through the same StyleSystem.
- MutationObserver fires for SVG DOM changes.
- Accessibility tree (Ch. 25) includes SVG `<title>` and `<desc>` elements.

### 19.2.2 SVG Presentation Attributes

SVG elements have presentation attributes that map to CSS properties (`fill`, `stroke`, `opacity`, `transform`, etc.). These are lower priority than CSS rules in the cascade:

```
Author stylesheets (highest)
  > Inline style attribute
  > SVG presentation attributes (treated as author-level declarations with specificity 0)
  > User agent stylesheet
```

The StyleSystem treats SVG presentation attributes as CSS declarations during cascade resolution. No special path needed — they are parsed from the `Attributes` component and injected into the cascade.

### 19.2.3 SVG-Specific CSS Properties

Beyond standard CSS properties, SVG defines additional properties:

| Property | Type | Notes |
| --- | --- | --- |
| `fill` | Paint (color, gradient ref, pattern ref, none) | Interior fill |
| `stroke` | Paint | Outline stroke |
| `stroke-width` | Length | Stroke thickness |
| `stroke-linecap` | butt / round / square | Line end style |
| `stroke-linejoin` | miter / round / bevel | Corner join style |
| `stroke-dasharray` | Length list | Dash pattern |
| `stroke-dashoffset` | Length | Dash offset |
| `fill-rule` | nonzero / evenodd | Fill algorithm |
| `clip-rule` | nonzero / evenodd | Clip algorithm |
| `fill-opacity` | Number | Fill alpha |
| `stroke-opacity` | Number | Stroke alpha |
| `marker-start/mid/end` | URL reference | Arrow markers |
| `paint-order` | fill / stroke / markers ordering | Paint sequence |

These are registered in the CSS property system alongside HTML properties.

## 19.3 SVG Layout

### 19.3.1 SvgLayoutSystem

SVG elements do not participate in CSS box layout (no block/inline flow, no flexbox, no grid). Instead, they use SVG's coordinate-based geometry:

```rust
pub struct SvgLayoutSystem;

impl SvgLayoutSystem {
    /// Compute SVG geometry for all SVG subtrees in the ECS.
    /// Runs after the CSS StyleSystem but in parallel with (or before) the HTML LayoutSystem.
    pub fn compute(&self, world: &mut World) {
        // Find all root <svg> elements
        for (entity, viewport) in world.query::<(Entity, &SvgViewport)>() {
            self.layout_svg_subtree(world, entity, viewport);
        }
    }

    fn layout_svg_subtree(
        &self,
        world: &mut World,
        svg_root: EntityId,
        viewport: &SvgViewport,
    ) {
        // 1. Establish SVG viewport coordinate system
        let viewport_transform = viewport.compute_transform();

        // 2. Walk SVG subtree, resolving geometry
        self.walk(world, svg_root, viewport_transform);
    }

    fn walk(
        &self,
        world: &mut World,
        entity: EntityId,
        parent_transform: Transform2D,
    ) {
        // Resolve local transform (transform attribute + CSS transform)
        let local_transform = world.get::<SvgTransform>(entity)
            .map(|t| t.matrix)
            .unwrap_or(Transform2D::IDENTITY);

        let accumulated = parent_transform * local_transform;

        // Compute bounding box for this element
        if let Some(geom) = world.get_mut::<SvgGeometry>(entity) {
            geom.bbox = geom.shape.bounding_box();
        }

        // Recurse into children
        for child in children_of(world, entity) {
            self.walk(world, child, accumulated);
        }
    }
}
```

### 19.3.2 ViewBox and Coordinate Systems

The `<svg>` element's `viewBox` and `preserveAspectRatio` attributes establish the mapping between SVG user coordinates and the CSS box assigned to the `<svg>` element:

```rust
pub struct ViewBox {
    pub min_x: f64,
    pub min_y: f64,
    pub width: f64,
    pub height: f64,
}

pub enum PreserveAspectRatio {
    None,
    Meet(Align),    // Scale uniformly, fit inside viewport
    Slice(Align),   // Scale uniformly, cover viewport
}

pub enum Align {
    XMinYMin, XMidYMin, XMaxYMin,
    XMinYMid, XMidYMid, XMaxYMid,
    XMinYMax, XMidYMax, XMaxYMax,
}

impl SvgViewport {
    pub fn compute_transform(&self) -> Transform2D {
        let css_width = self.width.resolve();
        let css_height = self.height.resolve();

        match &self.view_box {
            None => Transform2D::IDENTITY,
            Some(vb) => {
                // Scale from viewBox coordinates to CSS box coordinates
                // applying preserveAspectRatio alignment
                compute_viewbox_transform(vb, css_width, css_height, self.preserve_aspect_ratio)
            }
        }
    }
}
```

### 19.3.3 SVG–HTML Layout Interaction

The `<svg>` element itself participates in HTML layout as a replaced element. The HTML LayoutSystem assigns it a CSS box (width, height, position). Inside that box, SvgLayoutSystem takes over with SVG coordinate geometry.

```
HTML LayoutSystem
  │  assigns CSS box to <svg> element (e.g., 200×150 at position (50, 80))
  ▼
SvgLayoutSystem
  │  viewBox maps SVG coordinates into the 200×150 box
  │  all child <rect>, <circle>, <path> use SVG coordinates
  ▼
PaintSystem
  │  emits DisplayItems for SVG shapes within the <svg>'s layer
```

Nested `<svg>` elements create nested viewports, each with their own coordinate system.

## 19.4 SVG Paint

### 19.4.1 DisplayItem Extensions

The PaintSystem (Ch. 15) emits SVG content as DisplayItems alongside HTML content:

```rust
// Additions to the DisplayItem enum (Ch. 15 §15.5.1)
pub enum DisplayItem {
    // ... existing items (Rect, Border, Text, Image, etc.)

    /// SVG path with fill and/or stroke
    SvgPath {
        path: PathData,
        fill: Option<SvgPaint>,
        fill_rule: FillRule,
        stroke: Option<SvgStroke>,
        transform: Transform2D,
    },

    /// SVG text (positioned glyphs, not HTML text layout)
    SvgText {
        glyphs: Vec<PositionedGlyph>,
        font: FontKey,
        fill: Option<SvgPaint>,
        stroke: Option<SvgStroke>,
    },

    /// Reference to SVG gradient or pattern definition
    PushSvgPaint { paint: SvgPaintServer },
    PopSvgPaint,
}

pub struct SvgPaint {
    pub color: Color,
    pub opacity: f32,
}

pub struct SvgStroke {
    pub paint: SvgPaint,
    pub width: f64,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f64,
    pub dash_array: Option<Vec<f64>>,
    pub dash_offset: f64,
}

pub enum SvgPaintServer {
    Color(Color),
    LinearGradient(SvgLinearGradient),
    RadialGradient(SvgRadialGradient),
    Pattern(SvgPattern),
}
```

### 19.4.2 Vello Mapping

The vello_backend module (Ch. 15 §15.6.2) maps SVG DisplayItems to Vello scene commands:

```rust
// In vello_backend::build_scene()
DisplayItem::SvgPath { path, fill, fill_rule, stroke, transform } => {
    let kurbo_path = svg_path_to_kurbo(path);

    scene.push_transform(transform_to_vello(*transform));

    if let Some(fill) = fill {
        let rule = match fill_rule {
            FillRule::NonZero => vello::Fill::NonZero,
            FillRule::EvenOdd => vello::Fill::EvenOdd,
        };
        scene.fill(rule, vello::Affine::IDENTITY, fill_to_brush(fill), None, &kurbo_path);
    }

    if let Some(stroke) = stroke {
        let vello_stroke = vello::Stroke::new(stroke.width)
            .with_caps(cap_to_vello(stroke.line_cap))
            .with_join(join_to_vello(stroke.line_join))
            .with_miter_limit(stroke.miter_limit);
        // Dash pattern
        if let Some(ref dashes) = stroke.dash_array {
            let vello_stroke = vello_stroke.with_dashes(stroke.dash_offset, dashes);
        }
        scene.stroke(&vello_stroke, vello::Affine::IDENTITY, stroke_to_brush(&stroke.paint), None, &kurbo_path);
    }

    scene.pop_transform();
}
```

Vello is a natural fit for SVG: both operate on the same primitives (paths, fills, strokes, gradients). The Vello→GPU path (compute shader rasterization) handles complex SVG paths efficiently without CPU-side tessellation.

## 19.5 SVG Gradients and Patterns

### 19.5.1 Gradients

SVG `<linearGradient>` and `<radialGradient>` map to Vello gradient brushes:

```rust
pub struct SvgLinearGradient {
    pub x1: f64, pub y1: f64,
    pub x2: f64, pub y2: f64,
    pub stops: Vec<GradientStop>,
    pub units: GradientUnits,
    pub spread: SpreadMethod,
    pub transform: Transform2D,
}

pub struct SvgRadialGradient {
    pub cx: f64, pub cy: f64, pub r: f64,
    pub fx: f64, pub fy: f64, pub fr: f64,
    pub stops: Vec<GradientStop>,
    pub units: GradientUnits,
    pub spread: SpreadMethod,
    pub transform: Transform2D,
}

pub struct GradientStop {
    pub offset: f64,     // 0.0–1.0
    pub color: Color,
}

pub enum SpreadMethod {
    Pad,       // Extend end colors
    Reflect,   // Mirror gradient
    Repeat,    // Tile gradient
}
```

### 19.5.2 Patterns

SVG `<pattern>` elements define tiled fills. The pattern content is rendered to an offscreen texture, then tiled as a fill:

```rust
pub struct SvgPattern {
    pub content_entities: Vec<EntityId>,
    pub view_box: Option<ViewBox>,
    pub width: f64,
    pub height: f64,
    pub units: PatternUnits,
    pub content_units: PatternContentUnits,
    pub transform: Transform2D,
}
```

Pattern rendering: the PaintSystem renders the pattern content subtree to a small Vello Scene, rasterizes it to a texture tile, and uses that texture as a repeating fill brush.

## 19.6 SVG Filter Effects

### 19.6.1 Filter Graph

SVG filter effects are composed as a directed acyclic graph (DAG) of filter primitives. Each primitive takes one or two inputs and produces one output:

```rust
pub struct SvgFilterGraph {
    pub primitives: Vec<SvgFilterPrimitive>,
    pub region: FilterRegion,
}

pub struct SvgFilterPrimitive {
    pub kind: FilterPrimitiveKind,
    pub input1: FilterInput,
    pub input2: Option<FilterInput>,
    pub result: String,
    pub region: Option<FilterRegion>,
}

pub enum FilterInput {
    SourceGraphic,
    SourceAlpha,
    BackgroundImage,
    BackgroundAlpha,
    FillPaint,
    StrokePaint,
    PreviousResult(String),
}

pub enum FilterPrimitiveKind {
    GaussianBlur { std_dev_x: f64, std_dev_y: f64 },
    ColorMatrix { matrix_type: ColorMatrixType, values: Vec<f64> },
    Composite { operator: CompositeOperator },
    Offset { dx: f64, dy: f64 },
    Merge { inputs: Vec<FilterInput> },
    Flood { color: Color },
    Turbulence { base_freq: (f64, f64), num_octaves: u32, seed: f64, stitch: bool, fractal_noise: bool },
    DisplacementMap { scale: f64, x_channel: Channel, y_channel: Channel },
    Morphology { operator: MorphOperator, radius: (f64, f64) },
    ConvolveMatrix { kernel: Vec<f64>, order: (u32, u32), divisor: f64, bias: f64 },
    Image { href: Url },
    Tile,
    Blend { mode: BlendMode },
    ComponentTransfer { functions: [TransferFunction; 4] },
    DiffuseLighting { surface_scale: f64, diffuse_constant: f64, light: LightSource },
    SpecularLighting { surface_scale: f64, specular_constant: f64, specular_exponent: f64, light: LightSource },
}
```

### 19.6.2 Filter Execution

Filter graphs are executed on the GPU as a sequence of render passes:

```
SourceGraphic (rendered element → offscreen texture)
  │
  ├─ feGaussianBlur → texture A (compute shader: separable blur)
  ├─ feOffset → texture B (simple UV shift)
  ├─ feComposite(A, B) → texture C (blend shader)
  │
  └─ Final result composited into layer
```

Each filter primitive maps to a GPU operation:

| Primitive | GPU Implementation |
| --- | --- |
| `feGaussianBlur` | Separable two-pass compute shader (horizontal + vertical) |
| `feColorMatrix` | Fragment shader with 5×4 matrix multiply |
| `feComposite` | Blend operation (Porter-Duff or arithmetic) |
| `feOffset` | UV coordinate shift |
| `feMerge` | Multi-texture composite |
| `feFlood` | Clear to solid color |
| `feTurbulence` | Perlin/simplex noise compute shader |
| `feMorphology` | Erode/dilate via min/max filter |
| `feConvolveMatrix` | Convolution compute shader |
| `feDiffuseLighting` / `feSpecularLighting` | Per-pixel lighting shader |

Intermediate textures are allocated from a pool and recycled after the filter graph completes.

### 19.6.3 CSS Filter Relationship

CSS `filter: blur(5px) drop-shadow(...)` is implemented using the same SVG filter infrastructure. CSS filter functions are internally converted to equivalent SVG filter primitives:

| CSS Filter | SVG Equivalent |
| --- | --- |
| `blur(5px)` | `<feGaussianBlur stdDeviation="5">` |
| `brightness(1.5)` | `<feComponentTransfer>` with linear functions |
| `contrast(2)` | `<feComponentTransfer>` |
| `grayscale(1)` | `<feColorMatrix type="saturate" values="0">` |
| `drop-shadow(...)` | `<feGaussianBlur>` + `<feOffset>` + `<feMerge>` |
| `hue-rotate(90deg)` | `<feColorMatrix type="hueRotate">` |
| `invert(1)` | `<feComponentTransfer>` with table functions |
| `saturate(2)` | `<feColorMatrix type="saturate">` |
| `sepia(1)` | `<feColorMatrix>` with sepia matrix |

This unified implementation avoids duplicate code paths for CSS and SVG filters.

## 19.7 SVG Text

SVG text (`<text>`, `<tspan>`, `<textPath>`) follows different layout rules than HTML text:

- Characters are individually positionable (`x`, `y`, `dx`, `dy` attributes per glyph).
- Text can follow a path (`<textPath>`).
- Text uses SVG coordinate space, not CSS box layout.
- Fill and stroke apply to glyph outlines (text can be stroked).

```rust
pub struct SvgTextLayout {
    pub positioned_glyphs: Vec<PositionedGlyph>,
}

pub struct PositionedGlyph {
    pub glyph_id: GlyphId,
    pub x: f64,
    pub y: f64,
    pub rotate: f64,     // per-glyph rotation
    pub font: FontKey,
}
```

SVG text shaping uses the same text pipeline (Ch. 16) for font selection and glyph shaping (rustybuzz), but layout positioning is SVG-specific rather than CSS inline flow.

Vello renders SVG text glyphs directly as vector outlines on the GPU — the same path as HTML text rendering (Ch. 15 §15.6.2), but with per-glyph positioning and optional stroke.

## 19.8 SVG Animation

### 19.8.1 Core / Compat Classification

| Mechanism | Classification | Notes |
| --- | --- | --- |
| CSS animations on SVG elements | Core | Standard CSS, handled by AnimationEngine (Ch. 17) |
| CSS transitions on SVG properties | Core | `transition: fill 0.3s` works via AnimationEngine |
| Web Animations API on SVG | Core | `svgElement.animate()` via unified model (Ch. 17) |
| SMIL (`<animate>`, `<animateTransform>`, `<set>`) | Compat | Processed by elidex-compat layer |

### 19.8.2 CSS/WAAPI Animation on SVG

CSS and Web Animations on SVG elements work through the same AnimationEngine (Ch. 17). SVG-specific animatable properties (fill, stroke, d, viewBox, etc.) are added to the PropertyInterpolator:

```rust
// Extensions to PropertyInterpolator (Ch. 17 §17.4.2)
AnimatableProperty::SvgFill => {
    // Interpolate between two SVG paint values
    interpolate_svg_paint(from, to, progress)
}
AnimatableProperty::SvgD => {
    // Path data interpolation: corresponding path commands
    // with same structure can be interpolated per-coordinate
    interpolate_path_data(from, to, progress)
}
```

Path morphing (`d` attribute animation) requires that the from/to paths have compatible structure (same number and types of commands). If incompatible, discrete interpolation (flip at 50%) is used.

### 19.8.3 SMIL Processing

SMIL animations are handled by the elidex-compat layer as a pre-processing step:

```
[Browser mode with compat enabled]
  SMIL elements (<animate>, <set>, <animateTransform>)
    → elidex-compat translates to Web Animations API calls
    → AnimationEngine processes as WAAPI instances

[App mode, compat disabled]
  SMIL elements are ignored (no animation)
```

The compat layer converts SMIL declarations to equivalent `element.animate()` calls at parse time. SMIL's `begin`/`end` timing model (event-based, syncbase, access key) is mapped to WAAPI timing with appropriate delays and event listeners.

## 19.9 SVG-as-Image

### 19.9.1 Direct Vello Rendering

When SVG is loaded as an image (`<img src>`, CSS `background-image`, `<image>` within SVG), it is rendered through a lightweight path that bypasses the ECS:

```rust
pub struct SvgImageRenderer {
    parser: SvgParser,
}

impl SvgImageRenderer {
    /// Render SVG to bitmap at the specified size.
    /// Called by the image decode pipeline (Ch. 18).
    pub fn render(
        &self,
        svg_data: &[u8],
        target_size: Size,
        scale_factor: f64,
    ) -> Result<DecodedImage, SvgError> {
        // 1. Parse SVG into lightweight scene (no ECS)
        let scene = self.parser.parse_to_scene(svg_data)?;

        // 2. Compute viewBox → target size transform
        let transform = scene.viewport.fit_to(target_size, scale_factor);

        // 3. Build Vello Scene
        let mut vello_scene = vello::Scene::new();
        self.build_vello_scene(&scene, &mut vello_scene, transform);

        // 4. Render to offscreen texture
        let physical_size = Size {
            width: (target_size.width * scale_factor).ceil() as u32,
            height: (target_size.height * scale_factor).ceil() as u32,
        };
        let pixels = self.rasterize(&vello_scene, physical_size)?;

        Ok(DecodedImage {
            pixels,
            width: physical_size.width,
            height: physical_size.height,
            stride: physical_size.width * 4,
            pixel_format: PixelFormat::Rgba8,
            color_space: ColorSpace::Srgb,
        })
    }
}
```

### 19.9.2 SVG-as-Image Restrictions

Per the SVG specification, SVG loaded as an image has restricted capabilities:

| Feature | Inline SVG | SVG-as-image |
| --- | --- | --- |
| JavaScript | Yes | No |
| External resources (images, fonts, CSS) | Yes | No |
| Interactive events (click, hover) | Yes | No |
| CSS animations | Yes | Yes (self-contained) |
| SMIL animations | Yes (compat) | Yes (self-contained, compat) |
| `<use>` referencing external SVG | Yes | No |
| `<a>` hyperlinks | Yes | No |

These restrictions ensure SVG-as-image is a pure rendering operation with no side effects.

### 19.9.3 Cache Integration

SVG-as-image results are cached in the image decode cache (Ch. 18 §18.7) as regular bitmaps:

```rust
// Cache key includes target size because SVG is resolution-independent
ImageCacheKey {
    url: svg_url,
    decoded_size: target_physical_size,
    scale_factor: device_pixel_ratio,
}
```

When the same SVG is displayed at different sizes, each size produces a separate cached bitmap. On DPI change (window moved between displays), SVG-as-image is re-rasterized at the new resolution — a significant advantage over raster formats.

## 19.10 SVG Clipping and Masking

### 19.10.1 `clip-path`

SVG `<clipPath>` defines a clipping region using arbitrary shapes:

```rust
impl SvgClipHandler {
    fn apply_clip(
        &self,
        scene: &mut vello::Scene,
        clip_path: &SvgClipPath,
        world: &World,
    ) {
        // Resolve the <clipPath> element by ID
        let clip_entity = world.find_by_id(&clip_path.referenced_id);

        // Build clip shape from child elements
        let clip_shape = self.build_clip_shape(world, clip_entity);

        // Apply as Vello clip
        scene.push_clip(clip_shape);
    }
}
```

CSS `clip-path` on HTML elements uses the same SVG clip infrastructure when referencing an SVG `<clipPath>` element via `url(#clipId)`.

### 19.10.2 `mask`

SVG `<mask>` elements use luminance or alpha of the mask content to modulate the masked element's opacity. The mask content is rendered to an offscreen texture, then applied as an alpha mask during compositing.

## 19.11 `<use>` Element and Symbol Reuse

The `<use>` element creates a clone of a referenced element. In the ECS, `<use>` is represented as an entity with a reference to the original:

```rust
pub struct SvgUseRef {
    pub href: String,            // "#iconId"
    pub x: f64, pub y: f64,     // translation offset
    pub width: Option<f64>,
    pub height: Option<f64>,
}
```

The PaintSystem resolves `<use>` references at paint time, applying the `<use>` element's transform and style overrides to the referenced subtree. The referenced content is not duplicated in the ECS — only the rendering output is duplicated.

For `<symbol>` elements (commonly used with `<use>`), the symbol's `viewBox` establishes an additional coordinate transformation.

## 19.12 elidex-app SVG

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| Inline SVG | Full support | Full support |
| SVG-as-image | Full support | Full support |
| SMIL animation | Compat layer | Excluded (default). Opt-in at build time. |
| SVG filters | Full support | Full support |
| External SVG resources | Allowed (with CORS) | Configurable per capability |

SVG's vector nature makes it especially useful for elidex-app: resolution-independent icons, scalable UI components, and data visualization that adapts to any display density without raster assets.
