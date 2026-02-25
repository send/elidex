
# 19. SVGレンダリング

## 19.1 概要

SVGはモダンWebコンテンツで遍在：アイコン、ロゴ、イラスト、チャート、データビジュアライゼーション。Elidexは2つの異なるモードでSVGを処理：

1. **インラインSVG**：HTML DOMに埋め込まれた`<svg>`要素。完全なDOM APIアクセス、CSSスタイリング、イベントハンドリング、スクリプティング。
2. **SVG-as-image**：`<img src="icon.svg">`、CSS `background-image: url(icon.svg)`。ターゲット解像度でビットマップにレンダリング。DOMアクセスなし、スクリプティングなし、外部リソース読み込み制限あり。

これら2モードはSVGパーサーとVelloレンダリングパスを共有するが、エンジンの残りとの統合方法が異なる。

```
[インラインSVG]
  HTMLパーサーが<svg>を検出
    → SVG要素がECSエンティティになる（同じDOM）
    → StyleSystemがCSS解決（SVGプレゼンテーション属性含む）
    → SvgLayoutSystemがSVG座標ジオメトリを計算
    → PaintSystemがDisplayItemを発行（パス、フィル、ストローク）
    → Velloが通常パイプラインでレンダリング（第15章）

[SVG-as-image]
  画像デコードパイプライン（第18章）がSVGデータを受信
    → SVGパーサーが軽量シーンを構築（ECSなし）
    → 直接Vello Scene構築
    → ターゲットサイズでオフスクリーンテクスチャにラスタライズ
    → 結果をDecodedImage（ビットマップ）としてキャッシュ
```

## 19.2 インラインSVG：ECS表現

### 19.2.1 ECSエンティティとしてのSVG要素

SVG要素はHTML要素と同じECSに格納。各SVG要素は標準TreeRelationコンポーネント（親/子/兄弟）とSVG固有コンポーネントを持つエンティティ：

```rust
// 標準コンポーネント（HTMLと共有）
// - TreeRelation: ツリー構造
// - TagType: SvgRect, SvgCircle, SvgPath等
// - Attributes: キー値属性
// - ComputedStyle: 解決済みCSS（fill, stroke等のSVGプロパティ含む）

// SVG固有コンポーネント
pub struct SvgGeometry {
    pub shape: SvgShape,
    /// ローカル座標でのバウンディングボックス
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

この統一表現の意味：

- `document.querySelector('svg rect')`が同じScriptSession DOM APIで動作。
- CSSセレクタ（`svg .highlight { fill: red; }`）が同じStyleSystemで適用。
- MutationObserverがSVG DOM変更で発火。
- アクセシビリティツリー（第25章）がSVG `<title>`と`<desc>`要素を含む。

### 19.2.2 SVGプレゼンテーション属性

SVG要素はCSSプロパティにマッピングするプレゼンテーション属性を持つ（`fill`、`stroke`、`opacity`、`transform`等）。カスケードではCSS規則より低い優先度：

```
作者スタイルシート（最高）
  > インラインstyle属性
  > SVGプレゼンテーション属性（詳細度0の作者レベル宣言として扱い）
  > ユーザーエージェントスタイルシート
```

StyleSystemがカスケード解決中にSVGプレゼンテーション属性をCSS宣言として扱う。特別なパスは不要 — `Attributes`コンポーネントからパースしてカスケードに注入。

### 19.2.3 SVG固有CSSプロパティ

標準CSSプロパティ以外にSVGが追加プロパティを定義：

| プロパティ | 型 | 備考 |
| --- | --- | --- |
| `fill` | Paint（色、グラデーション参照、パターン参照、none） | 内部塗りつぶし |
| `stroke` | Paint | 輪郭ストローク |
| `stroke-width` | Length | ストロークの太さ |
| `stroke-linecap` | butt / round / square | 線端スタイル |
| `stroke-linejoin` | miter / round / bevel | 角の結合スタイル |
| `stroke-dasharray` | Length list | 破線パターン |
| `stroke-dashoffset` | Length | 破線オフセット |
| `fill-rule` | nonzero / evenodd | 塗りつぶしアルゴリズム |
| `clip-rule` | nonzero / evenodd | クリッピングアルゴリズム |
| `fill-opacity` | Number | 塗りつぶしアルファ |
| `stroke-opacity` | Number | ストロークアルファ |
| `marker-start/mid/end` | URL参照 | 矢印マーカー |
| `paint-order` | fill / stroke / markers の順序 | ペイント順序 |

これらはHTMLプロパティと共にCSSプロパティシステムに登録。

## 19.3 SVGレイアウト

### 19.3.1 SvgLayoutSystem

SVG要素はCSSボックスレイアウトに参加しない（ブロック/インラインフロー、flexbox、gridなし）。代わりにSVGの座標ベースジオメトリを使用：

```rust
pub struct SvgLayoutSystem;

impl SvgLayoutSystem {
    /// ECS内のすべてのSVGサブツリーのジオメトリを計算。
    /// CSS StyleSystemの後、HTMLのLayoutSystemと並列（または前）に実行。
    pub fn compute(&self, world: &mut World) {
        // すべてのルート<svg>要素を検索
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
        // 1. SVGビューポート座標系を確立
        let viewport_transform = viewport.compute_transform();

        // 2. SVGサブツリーを走査、ジオメトリを解決
        self.walk(world, svg_root, viewport_transform);
    }

    fn walk(
        &self,
        world: &mut World,
        entity: EntityId,
        parent_transform: Transform2D,
    ) {
        // ローカル変換を解決（transform属性 + CSS transform）
        let local_transform = world.get::<SvgTransform>(entity)
            .map(|t| t.matrix)
            .unwrap_or(Transform2D::IDENTITY);

        let accumulated = parent_transform * local_transform;

        // この要素のバウンディングボックスを計算
        if let Some(geom) = world.get_mut::<SvgGeometry>(entity) {
            geom.bbox = geom.shape.bounding_box();
        }

        // 子要素に再帰
        for child in children_of(world, entity) {
            self.walk(world, child, accumulated);
        }
    }
}
```

### 19.3.2 ViewBoxと座標系

`<svg>`要素の`viewBox`と`preserveAspectRatio`属性がSVGユーザー座標と`<svg>`要素に割り当てられたCSSボックス間のマッピングを確立：

```rust
pub struct ViewBox {
    pub min_x: f64,
    pub min_y: f64,
    pub width: f64,
    pub height: f64,
}

pub enum PreserveAspectRatio {
    None,
    Meet(Align),    // 均一スケール、ビューポート内に収める
    Slice(Align),   // 均一スケール、ビューポートをカバー
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
                // viewBox座標からCSSボックス座標へのスケール
                // preserveAspectRatioアラインメント適用
                compute_viewbox_transform(vb, css_width, css_height, self.preserve_aspect_ratio)
            }
        }
    }
}
```

### 19.3.3 SVG–HTMLレイアウト相互作用

`<svg>`要素自体はHTMLレイアウトに置換要素として参加。HTML LayoutSystemがCSSボックス（幅、高さ、位置）を割り当て。そのボックス内でSvgLayoutSystemがSVG座標ジオメトリを引き継ぐ。

```
HTML LayoutSystem
  │  <svg>要素にCSSボックスを割り当て（例：位置(50, 80)で200×150）
  ▼
SvgLayoutSystem
  │  viewBoxがSVG座標を200×150ボックスにマッピング
  │  すべての子<rect>、<circle>、<path>がSVG座標を使用
  ▼
PaintSystem
  │  <svg>のレイヤー内でSVGシェイプのDisplayItemを発行
```

ネストされた`<svg>`要素はネストされたビューポートを作成し、それぞれ独自の座標系を持つ。

## 19.4 SVGペイント

### 19.4.1 DisplayItem拡張

PaintSystem（第15章）がHTML コンテンツと共にSVGコンテンツをDisplayItemとして発行：

```rust
// DisplayItem enumへの追加（第15章§15.5.1）
pub enum DisplayItem {
    // ... 既存アイテム（Rect、Border、Text、Image等）

    /// フィルおよび/またはストローク付きSVGパス
    SvgPath {
        path: PathData,
        fill: Option<SvgPaint>,
        fill_rule: FillRule,
        stroke: Option<SvgStroke>,
        transform: Transform2D,
    },

    /// SVGテキスト（配置済みグリフ、HTMLテキストレイアウトではない）
    SvgText {
        glyphs: Vec<PositionedGlyph>,
        font: FontKey,
        fill: Option<SvgPaint>,
        stroke: Option<SvgStroke>,
    },

    /// SVGグラデーションまたはパターン定義への参照
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

### 19.4.2 Velloマッピング

vello_backendモジュール（第15章§15.6.2）がSVG DisplayItemをVelloシーンコマンドにマッピング：

```rust
// vello_backend::build_scene()内
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
        if let Some(ref dashes) = stroke.dash_array {
            let vello_stroke = vello_stroke.with_dashes(stroke.dash_offset, dashes);
        }
        scene.stroke(&vello_stroke, vello::Affine::IDENTITY, stroke_to_brush(&stroke.paint), None, &kurbo_path);
    }

    scene.pop_transform();
}
```

VelloはSVGに自然適合：両者が同じプリミティブ（パス、フィル、ストローク、グラデーション）で動作。Vello→GPU（コンピュートシェーダーラスタライゼーション）が複雑なSVGパスをCPU側テッセレーションなしで効率的に処理。

## 19.5 SVGグラデーションとパターン

### 19.5.1 グラデーション

SVG `<linearGradient>`と`<radialGradient>`がVelloグラデーションブラシにマッピング：

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
    Pad,       // 端の色を延長
    Reflect,   // グラデーションを反転
    Repeat,    // グラデーションをタイル
}
```

### 19.5.2 パターン

SVG `<pattern>`要素がタイル塗りつぶしを定義。パターン内容はオフスクリーンテクスチャにレンダリングし、塗りつぶしとしてタイル：

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

パターンレンダリング：PaintSystemがパターン内容サブツリーを小さなVello Sceneにレンダリングし、テクスチャタイルにラスタライズし、そのテクスチャを繰り返し塗りつぶしブラシとして使用。

## 19.6 SVGフィルタエフェクト

### 19.6.1 フィルタグラフ

SVGフィルタエフェクトはフィルタプリミティブの有向非巡回グラフ（DAG）として構成。各プリミティブは1つまたは2つの入力を取り、1つの出力を生成：

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

### 19.6.2 フィルタ実行

フィルタグラフはGPU上でレンダーパスのシーケンスとして実行：

```
SourceGraphic（レンダリング済み要素 → オフスクリーンテクスチャ）
  │
  ├─ feGaussianBlur → テクスチャA（コンピュートシェーダー：分離可能ブラー）
  ├─ feOffset → テクスチャB（単純なUVシフト）
  ├─ feComposite(A, B) → テクスチャC（ブレンドシェーダー）
  │
  └─ 最終結果がレイヤーに合成
```

各フィルタプリミティブがGPU操作にマッピング：

| プリミティブ | GPU実装 |
| --- | --- |
| `feGaussianBlur` | 分離可能2パスコンピュートシェーダー（水平+垂直） |
| `feColorMatrix` | 5×4行列乗算のフラグメントシェーダー |
| `feComposite` | ブレンド操作（Porter-Duffまたは算術） |
| `feOffset` | UV座標シフト |
| `feMerge` | マルチテクスチャ合成 |
| `feFlood` | ソリッドカラーにクリア |
| `feTurbulence` | Perlin/simplexノイズコンピュートシェーダー |
| `feMorphology` | min/maxフィルタによるエロード/ディレート |
| `feConvolveMatrix` | 畳み込みコンピュートシェーダー |
| `feDiffuseLighting` / `feSpecularLighting` | ピクセル単位ライティングシェーダー |

中間テクスチャはプールから割り当てられ、フィルタグラフ完了後にリサイクル。

### 19.6.3 CSSフィルタとの関係

CSS `filter: blur(5px) drop-shadow(...)`は同じSVGフィルタインフラストラクチャを使用して実装。CSSフィルタ関数は内部的に等価なSVGフィルタプリミティブに変換：

| CSSフィルタ | SVG等価 |
| --- | --- |
| `blur(5px)` | `<feGaussianBlur stdDeviation="5">` |
| `brightness(1.5)` | 線形関数付き`<feComponentTransfer>` |
| `contrast(2)` | `<feComponentTransfer>` |
| `grayscale(1)` | `<feColorMatrix type="saturate" values="0">` |
| `drop-shadow(...)` | `<feGaussianBlur>` + `<feOffset>` + `<feMerge>` |
| `hue-rotate(90deg)` | `<feColorMatrix type="hueRotate">` |
| `invert(1)` | テーブル関数付き`<feComponentTransfer>` |
| `saturate(2)` | `<feColorMatrix type="saturate">` |
| `sepia(1)` | セピアマトリクス付き`<feColorMatrix>` |

この統一実装によりCSSとSVGフィルタの重複コードパスを回避。

## 19.7 SVGテキスト

SVGテキスト（`<text>`、`<tspan>`、`<textPath>`）はHTMLテキストと異なるレイアウト規則に従う：

- 文字が個別に配置可能（グリフごとに`x`、`y`、`dx`、`dy`属性）。
- テキストがパスに沿う（`<textPath>`）。
- テキストがCSS ボックスレイアウトではなくSVG座標空間を使用。
- フィルとストロークがグリフアウトラインに適用（テキストにストローク可能）。

```rust
pub struct SvgTextLayout {
    pub positioned_glyphs: Vec<PositionedGlyph>,
}

pub struct PositionedGlyph {
    pub glyph_id: GlyphId,
    pub x: f64,
    pub y: f64,
    pub rotate: f64,     // グリフ単位の回転
    pub font: FontKey,
}
```

SVGテキストシェーピングはフォント選択とグリフシェーピング（rustybuzz）に同じテキストパイプライン（第16章）を使用するが、レイアウト配置はCSSインラインフローではなくSVG固有。

VelloがSVGテキストグリフをGPU上でベクターアウトラインとして直接レンダリング — HTMLテキストレンダリング（第15章§15.6.2）と同じパスだが、グリフ単位の配置とオプショナルストローク付き。

## 19.8 SVGアニメーション

### 19.8.1 Core / Compat分類

| メカニズム | 分類 | 備考 |
| --- | --- | --- |
| SVG要素へのCSSアニメーション | Core | 標準CSS、AnimationEngine（第17章）で処理 |
| SVGプロパティへのCSSトランジション | Core | `transition: fill 0.3s`がAnimationEngine経由で動作 |
| SVGへのWeb Animations API | Core | `svgElement.animate()`が統一モデル（第17章）経由 |
| SMIL（`<animate>`、`<animateTransform>`、`<set>`） | Compat | elidex-compat層で処理 |

### 19.8.2 SVGへのCSS/WAAPIアニメーション

SVG要素へのCSSおよびWeb Animationsは同じAnimationEngine（第17章）で動作。SVG固有のアニメーション可能プロパティ（fill、stroke、d、viewBox等）がPropertyInterpolatorに追加：

```rust
// PropertyInterpolatorへの拡張（第17章§17.4.2）
AnimatableProperty::SvgFill => {
    // 2つのSVGペイント値間の補間
    interpolate_svg_paint(from, to, progress)
}
AnimatableProperty::SvgD => {
    // パスデータ補間：同じ構造を持つ対応パスコマンドが
    // 座標ごとに補間可能
    interpolate_path_data(from, to, progress)
}
```

パスモーフィング（`d`属性アニメーション）はfrom/toパスが互換構造を持つ（同数・同種のコマンド）必要あり。非互換の場合は離散補間（50%で切替）。

### 19.8.3 SMIL処理

SMILアニメーションはelidex-compat層で前処理ステップとして処理：

```
[ブラウザモード、compat有効]
  SMIL要素（<animate>、<set>、<animateTransform>）
    → elidex-compatがWeb Animations API呼び出しに変換
    → AnimationEngineがWAAPIインスタンスとして処理

[アプリモード、compat無効]
  SMIL要素は無視（アニメーションなし）
```

compat層がSMIL宣言をパース時に等価な`element.animate()`呼び出しに変換。SMILの`begin`/`end`タイミングモデル（イベントベース、syncbase、アクセスキー）は適切な遅延とイベントリスナー付きのWAAPIタイミングにマッピング。

## 19.9 SVG-as-image

### 19.9.1 直接Velloレンダリング

SVGが画像として読み込まれる場合（`<img src>`、CSS `background-image`、SVG内`<image>`）、ECSをバイパスする軽量パスでレンダリング：

```rust
pub struct SvgImageRenderer {
    parser: SvgParser,
}

impl SvgImageRenderer {
    /// 指定サイズでSVGをビットマップにレンダリング。
    /// 画像デコードパイプライン（第18章）から呼び出される。
    pub fn render(
        &self,
        svg_data: &[u8],
        target_size: Size,
        scale_factor: f64,
    ) -> Result<DecodedImage, SvgError> {
        // 1. SVGを軽量シーンにパース（ECSなし）
        let scene = self.parser.parse_to_scene(svg_data)?;

        // 2. viewBox → ターゲットサイズ変換を計算
        let transform = scene.viewport.fit_to(target_size, scale_factor);

        // 3. Vello Sceneを構築
        let mut vello_scene = vello::Scene::new();
        self.build_vello_scene(&scene, &mut vello_scene, transform);

        // 4. オフスクリーンテクスチャにレンダリング
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

### 19.9.2 SVG-as-imageの制限

SVG仕様に従い、画像として読み込まれたSVGは制限された機能を持つ：

| 機能 | インラインSVG | SVG-as-image |
| --- | --- | --- |
| JavaScript | あり | なし |
| 外部リソース（画像、フォント、CSS） | あり | なし |
| インタラクティブイベント（click、hover） | あり | なし |
| CSSアニメーション | あり | あり（自己完結） |
| SMILアニメーション | あり（compat） | あり（自己完結、compat） |
| 外部SVGを参照する`<use>` | あり | なし |
| `<a>`ハイパーリンク | あり | なし |

これらの制限によりSVG-as-imageが副作用のない純粋なレンダリング操作であることを保証。

### 19.9.3 キャッシュ統合

SVG-as-imageの結果は画像デコードキャッシュ（第18章§18.7）に通常ビットマップとしてキャッシュ：

```rust
// SVGは解像度非依存なのでキャッシュキーにターゲットサイズを含む
ImageCacheKey {
    url: svg_url,
    decoded_size: target_physical_size,
    scale_factor: device_pixel_ratio,
}
```

同じSVGが異なるサイズで表示される場合、各サイズが別個のキャッシュビットマップを生成。DPI変更時（ディスプレイ間のウィンドウ移動）、SVG-as-imageは新解像度で再ラスタライズ — ラスターフォーマットに対する大きな利点。

## 19.10 SVGクリッピングとマスキング

### 19.10.1 `clip-path`

SVG `<clipPath>`が任意のシェイプを使用したクリッピング領域を定義：

```rust
impl SvgClipHandler {
    fn apply_clip(
        &self,
        scene: &mut vello::Scene,
        clip_path: &SvgClipPath,
        world: &World,
    ) {
        // IDで<clipPath>要素を解決
        let clip_entity = world.find_by_id(&clip_path.referenced_id);

        // 子要素からクリップシェイプを構築
        let clip_shape = self.build_clip_shape(world, clip_entity);

        // Velloクリップとして適用
        scene.push_clip(clip_shape);
    }
}
```

HTML要素へのCSS `clip-path`がSVG `<clipPath>`要素を`url(#clipId)`経由で参照する場合、同じSVGクリップインフラストラクチャを使用。

### 19.10.2 `mask`

SVG `<mask>`要素がマスク内容の輝度またはアルファを使用してマスクされた要素の不透明度を変調。マスク内容はオフスクリーンテクスチャにレンダリングされ、合成時にアルファマスクとして適用。

## 19.11 `<use>`要素とシンボル再利用

`<use>`要素が参照された要素のクローンを作成。ECSでは`<use>`はオリジナルへの参照を持つエンティティとして表現：

```rust
pub struct SvgUseRef {
    pub href: String,            // "#iconId"
    pub x: f64, pub y: f64,     // 移動オフセット
    pub width: Option<f64>,
    pub height: Option<f64>,
}
```

PaintSystemがペイント時に`<use>`参照を解決し、`<use>`要素の変換とスタイルオーバーライドを参照サブツリーに適用。参照内容はECSで複製されない — レンダリング出力のみ複製。

`<symbol>`要素（`<use>`と共に一般的に使用）では、シンボルの`viewBox`が追加の座標変換を確立。

## 19.12 elidex-app SVG

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| インラインSVG | フルサポート | フルサポート |
| SVG-as-image | フルサポート | フルサポート |
| SMILアニメーション | Compat層 | 除外（デフォルト）。ビルド時オプトイン。 |
| SVGフィルタ | フルサポート | フルサポート |
| 外部SVGリソース | 許可（CORS付き） | ケイパビリティごとに設定可能 |

SVGのベクター特性はelidex-appで特に有用：解像度非依存のアイコン、スケーラブルUIコンポーネント、ラスターアセットなしで任意のディスプレイ密度に適応するデータビジュアライゼーション。
