
# 15. レンダリングパイプライン

## 15.1 ECSベースDOM

DOMは従来のオブジェクト指向ツリーではなくEntity Component Systemとして格納される。各DOMノードはエンティティ（整数ID）であり、コンポーネントが型付き連続配列に格納される：

| コンポーネント | 内容 |
| --- | --- |
| TreeRelation | 親、最初の子、次の兄弟、前の兄弟のインデックス |
| TagType | 要素種別enum（HtmlElementHandlerプラグインにディスパッチ） |
| Attributes | キー値属性ストレージ |
| ComputedStyle | 解決済みCSSプロパティ（StyleSystemの出力） |
| LayoutBox | 位置、サイズ、マージン、パディング（LayoutSystemの出力） |
| PaintData | ディスプレイリストフラグメント（PaintSystemの出力） |
| Accessibility | ARIAロール、アクセシブル名、関係（a11yツリービルダーが消費） |

このレイアウトにより、同じコンポーネントタイプを処理するシステム（例：StyleSystemがAttributesを読みComputedStyleを書く）が連続メモリをイテレートし、L1/L2キャッシュヒット率を最大化。

## 15.2 並列パイプライン

レンダリングパイプラインはステージで進行。スタイル解決とレイアウト計算はServoの実証済みアプローチに従いrayonで並列化：

```
HTMLバイト
  │  [Parse]            ─ 逐次（ストリーミング、インクリメンタル）
  ▼
DOM (ECS)
  │  [Script Execution] ─ JS/WasmがScriptSession経由でDOM/CSSOMを変更
  │  [Session Flush]    ─ Mutation Buffer → ECSコンポーネント書込（バッチ）
  │  [StyleSystem]      ─ サブツリー単位で並列（rayon）
  ▼
Styled DOM
  │  [LayoutSystem]     ─ 独立サブツリーで並列
  ▼
Layout Tree
  │  [PaintSystem]      ─ レイヤー単位で並列
  ▼
Display List + Layer Tree
  │  [Compositor]       ─ コンポジタスレッド（第6章）
  ▼
Vello Scene
  │  [GPU]              ─ wgpu + Velloコンピュートシェーダー
  ▼
画面上のピクセル
```

ScriptSessionのflushはスクリプト世界とレンダリングパイプラインの境界。スクリプト実行中のすべてのDOMおよびCSSOM変更はセッションにバッファされ、flush時にECSコンポーネントに一括適用。これによりレンダリングパイプラインが部分的に変更されたDOMを見ることがなく、MutationObserverが一貫したレコードを受け取る。

elidexがquirksモードとレガシーレイアウトアルゴリズムを排除するため、各ステージのブランチが既存エンジンより大幅に少なく、シングルスレッドスループットと並列スケーリングの両方が改善。

## 15.3 互換レイヤー統合

ブラウザモードでは、互換レイヤーがコアの前に正規化フェーズとして位置する。コアはレガシーHTMLを見ない：

```
[ブラウザモード]
  Raw HTML ───▶ elidex-compat（正規化）───▶ Clean HTML5 ───▶ elidex-core

[アプリモード]
  HTML5 ───▶ elidex-core（直接）
```

互換レイヤーはトランスパイラとして機能：非推奨タグをHTML5相当に変換し、ベンダープレフィクスを解決し、レガシー文字エンコーディングをUTF-8にトランスコード。入力がすでにクリーンなHTML5なら高速パススルー。

## 15.4 レイヤーツリー

### 15.4.1 独立構造としてのレイヤーツリー

レイヤーツリーはECS DOMとは別の独立したデータ構造。PaintSystemがECSコンポーネント（ComputedStyle、LayoutBox）を読み取り、出力としてレイヤーツリーを構築：

```
ECS DOM（エンティティ + コンポーネント）
  │
  │  [PaintSystemがLayoutBox、ComputedStyle等を読取]
  ▼
LayerTree（独立構造、メインスレッドが所有）
  │
  │  [アプローチBではDisplayListにシリアライズ（第6章）]
  │  [アプローチCではArcで共有]
  ▼
コンポジタスレッド
```

レイヤーツリーがECSコンポーネントでないのは、DOMノードとレイヤーの構造が根本的に異なるため。複数DOMノードが1レイヤーにスカッシュされうるし、1 DOMノードから複数レイヤーが生まれることもある（例：スクロール可能要素はコンテナレイヤーとコンテンツレイヤーの両方を作成）。エンティティとレイヤーのN:M関係は1エンティティ:1コンポーネントのECSモデルに合わない。

```rust
pub struct LayerTree {
    layers: Vec<Layer>,
    root: LayerId,
}

pub struct Layer {
    pub id: LayerId,
    pub parent: Option<LayerId>,
    pub children: Vec<LayerId>,
    /// レイヤーローカル座標でのコンテンツ境界
    pub bounds: Rect,
    /// レイヤーローカルから親座標への変換
    pub transform: Transform3D,
    pub opacity: f32,
    pub clip: Option<ClipRegion>,
    pub blend_mode: BlendMode,
    /// スクロールオフセット（スクロール可能レイヤー用、コンポジタが更新）
    pub scroll_offset: ScrollOffset,
    /// オフスクリーンバッファが必要か
    pub needs_surface: bool,
    pub content: LayerContent,
}

pub enum LayerContent {
    /// ペイントされたDOMコンテンツ（ディスプレイリストコマンド）
    DisplayList(DisplayListSlice),
    /// ビデオフレーム（GPUテクスチャ参照）
    VideoFrame(TextureHandle),
    /// Canvas 2D / WebGL / WebGPU出力
    Canvas(TextureHandle),
    /// WorkerからのOffscreenCanvas（第6章）
    OffscreenCanvas(TextureHandle),
}
```

### 15.4.2 レイヤー昇格基準

以下のトリガーに基づき要素を独自レイヤーに昇格：

| トリガー | 条件 | 理由 |
| --- | --- | --- |
| 明示的ヒント | `will-change: transform`、`will-change: opacity` | 開発者がコンポジタアニメーションの意図を宣言。昇格ジャンク回避のため事前割当。 |
| アクティブアニメーション | `transform`または`opacity`がアニメーション中 | コンポジタがメインスレッドなしで補間可能。 |
| Fixed/Sticky配置 | `position: fixed`または`position: sticky` | スクロール時に独立移動。 |
| スクロール可能オーバーフロー | `overflow: auto/scroll`でコンテンツ超過 | コンポジタ駆動スクロール用に別レイヤー。 |
| Video | `<video>` | デコード済みフレームはGPUテクスチャ。直接合成。 |
| Canvas | `<canvas>`（2D、WebGL、WebGPU） | レンダリング出力はすでにGPUテクスチャ。 |
| 隔離 | `isolation: isolate`、非normalの`mix-blend-mode` | 正しいブレンドに隔離サーフェスが必要。 |
| CSSフィルタ | `filter`、`backdrop-filter` | フィルタ効果にオフスクリーンレンダリング必要。 |
| クリップパス/マスク | `clip-path`、`mask` | ステンシルバッファまたはオフスクリーンパス。 |
| 3D変換 | perspectiveまたはZ成分を持つ`transform` | 3Dでの深度ソート。 |

すべてのスタッキングコンテキストがレイヤーになるわけではない。PaintSystemがヒューリスティクスで合成の柔軟性とGPUメモリコストをバランス。

### 15.4.3 レイヤー爆発防止

各レイヤーがGPUテクスチャを消費。過度なレイヤー昇格はVRAMを浪費：

| ヒューリスティック | 戦略 |
| --- | --- |
| スカッシング | 同じスタッキングコンテキスト内の個別レイヤー不要な隣接要素は親レイヤーにまとめてペイント。 |
| オーバーラップ処理 | 合成レイヤーとオーバーラップする要素はz順序の正確性のために昇格されうる。軽微なオーバーラップはスカッシング優先。 |
| メモリ予算 | 合計レイヤーテクスチャメモリを追跡。デフォルト予算: 256MB。予算超過時は低優先度レイヤー（アクティブなアニメーション/スクロールなし）をスカッシュバック。 |
| will-change制限 | ページ内20以上の`will-change`要素でコンソール警告。予算超過時はエンジンが昇格を拒否する場合あり。 |

### 15.4.4 無効化

DOMまたはスタイル変更時、影響を受けたレイヤーのみ再ペイント：

```rust
pub struct Invalidation {
    pub layer_id: LayerId,
    /// レイヤー内のダーティ矩形
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

変更のないレイヤーはキャッシュテクスチャから再合成。影響レイヤー内のダーティ矩形のみ再ラスタライズ。

## 15.5 ディスプレイリスト

ペイントとラスタライゼーションの間の中間表現。描画コマンドをコンパクトでシリアライズ可能な形式でエンコード。

### 15.5.1 コマンド

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

各レイヤーが`DisplayListSlice`（`DisplayItem`の連続範囲）を生成。フォントと画像はキー（FontKey、ImageKey）で参照；実データは別途管理。

### 15.5.2 シリアライゼーション

アプローチB（第6章）では、ディスプレイリストをシリアライズしDisplayPipelineチャネル経由でコンポジタに送信（postcard形式、IPCシリアライゼーションと同一）：

| 最適化 | 戦略 |
| --- | --- |
| キーのインターン化 | フォントと画像は軽量キーで参照。実GPUデータは別途アップロード。 |
| デルタ更新 | 無効化レイヤーのディスプレイリストのみ再送。変更なしレイヤーは前フレームのIDで参照。 |
| アリーナアロケーション | ディスプレイアイテムはフレーム単位アリーナから割当。フレーム終了時にアリーナ全体をドロップ。 |

## 15.6 GPUレンダリング：wgpu上のVello

### 15.6.1 アーキテクチャ

Elidexは2層GPUスタックを使用：

| 層 | ライブラリ | 役割 |
| --- | --- | --- |
| GPU抽象化 | wgpu | クロスプラットフォームGPU API。Vulkan、Metal、DX12、WebGPUをターゲット。 |
| 2Dレンダリング | Vello | GPUアクセラレート2Dベクターレンダラー。コンピュートシェーダーによるパス平坦化、面積計算、タイルビニング。 |

VelloはすべてのDOMコンテンツの主要レンダラー。CPU側ラスタライザと異なり、ラスタライゼーションを完全にGPU上で実行。複雑なベクターコンテンツ（角丸、グラデーション、大量テキスト、SVG）で大きな性能優位。

**Velloはトレイト抽象化せず直接依存。** hyper（第10章HttpTransport）やSQLite（第22章StorageBackend）と異なり、VelloにはGPU 2Dレンダリングの実用的なRustネイティブ代替が存在しない。トレイト定義はVelloのシーンAPIを実質的利点なく複製するだけになる。代わりに接触面を隔離：DisplayList→Vello Scene変換層のみがVelloのAPIに直接触れる。レイヤーツリー、ディスプレイリスト、上流コード（ECS、スタイル、レイアウト、ペイント）はVelloに依存しない。将来有力な代替が出現した場合、変換層のみが置換対象。（ADR #26参照。）

### 15.6.2 DisplayList → Vello Scene変換

コンポジタスレッドがディスプレイリストコマンドをVelloのシーングラフに変換。Vello APIとの唯一の接触点：

```rust
/// vello型をインポートする唯一のモジュール。
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
                        // VelloがGPU上でグリフアウトラインを直接レンダリング。
                        // ベクターテキストにCPU側グリフラスタライゼーション不要。
                        scene.draw_glyphs(font_to_vello(*font))
                            .brush(color_to_vello(*color))
                            .draw(glyphs_to_vello(glyphs));
                    }
                    DisplayItem::Image { bounds, key, .. } => {
                        scene.draw_image(&texture_for_key(*key), *bounds);
                    }
                    // ... 他のアイテムも同様にマッピング
                }
            }

            scene.pop_layer();
        }

        scene
    }
}
```

### 15.6.3 GPUレンダリングパイプライン

```
ディスプレイリスト（レイヤー単位）
  │
  ├─ vello_backend::build_scene()
  │   └─ Vello Scene（パス、フィル、ストローク、グリフ、画像）
  │
  ├─ Velloエンコーディング（CPU）
  │   └─ シーンをGPUバッファにエンコード（パスセグメント、スタイル、変換）
  │
  ├─ Velloコンピュートパス（GPU）
  │   ├─ パス平坦化（コンピュートシェーダー）
  │   ├─ タイルビニング（コンピュートシェーダー）
  │   └─ ファインラスタライゼーション（コンピュートシェーダー）
  │
  ├─ レイヤー合成（レンダーパス）
  │   └─ 変換、クリップ、不透明度、ブレンドでラスタライズ済みレイヤーを合成
  │
  └─ RenderSurface（第23章）にプレゼント
```

### 15.6.4 ソフトウェアフォールバック

適切なGPUサポートがないシステム（ヘッドレスサーバー、CI、一部のVM）ではVelloのCPUバックエンドを使用：

```rust
pub enum RenderBackend {
    Gpu(wgpu::Device, vello::Renderer),
    Software(vello::cpu::Renderer),
}
```

選択は自動（wgpuデバイス作成失敗時にフォールバック）または設定で強制。ソフトウェアモードは決定論的テストにも有用 — GPUレンダリングはドライバ間でビット一致しない。

## 15.7 テクスチャ管理

### 15.7.1 段階的アプローチ

テクスチャ管理はフェーズで進化：

| フェーズ | 戦略 | タイミング |
| --- | --- | --- |
| Phase 1 | 個別テクスチャ | 初期実装。各デコード済み画像とCanvas出力が独自のwgpu::Textureを取得。シンプル、正確、デバッグ容易。 |
| Phase 2 | 個別テクスチャ＋小画像用アトラス | プロファイリングでテクスチャ切替オーバーヘッドが判明時。小画像（<256×256）とカラー絵文字をアトラステクスチャにパック。大画像は個別のまま。 |
| Phase 3 | 統一テクスチャ管理 | タブ間でフルGPUメモリ予算制御が必要時。全テクスチャ割り当てがエビクションポリシー付き中央アロケータを通過。 |

Phase 1で初期開発に十分。エンジンの他部分がテクスチャとやり取りするAPI（TextureHandle、ImageKey）は不透明で全フェーズで安定。

### 15.7.2 テクスチャハンドル

GPUモジュール外のすべてのコードが不透明ハンドルでテクスチャを参照：

```rust
/// GPUテクスチャへの不透明ハンドル。内部表現は
/// テクスチャ管理フェーズ間で変更される。
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct TextureHandle(u64);

pub struct TextureManager {
    textures: HashMap<TextureHandle, GpuTexture>,
    next_handle: AtomicU64,
}

impl TextureManager {
    /// デコード済み画像データをGPUにアップロード。ディスプレイリスト参照用ハンドルを返す。
    pub fn upload_image(&mut self, data: &DecodedImage) -> TextureHandle { /* ... */ }

    /// GPUテクスチャを解放（画像がどのレイヤーからも参照されなくなった）。
    pub fn release(&mut self, handle: TextureHandle) { /* ... */ }

    /// レンダリング用wgpu::TextureViewを取得。vello_backendが呼び出す。
    pub fn get_view(&self, handle: TextureHandle) -> &wgpu::TextureView { /* ... */ }
}
```

### 15.7.3 CPU→GPU転送

```rust
pub struct StagingBelt {
    /// CPU→GPU転送用の再利用可能ステージングバッファ
    buffers: Vec<wgpu::Buffer>,
}

impl StagingBelt {
    /// テクスチャアップロードをスケジュール。ノンブロッキング — 実際の転送は
    /// コマンドエンコーダがGPUキューに送信された時に発生。
    pub fn upload_texture(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        data: &[u8],
        region: TextureRegion,
    ) { /* ... */ }
}
```

| コンテンツタイプ | アップロードタイミング | 備考 |
| --- | --- | --- |
| デコード済み画像 | デコード完了時（非同期） | rayonプールがデコード（第6章）、その後ステージングバッファアップロード。ノンブロッキング。 |
| Canvas 2D出力 | フレームごと | CPUビットマップ → ステージングバッファ。OffscreenCanvas（Worker）はすでにGPUテクスチャの場合あり。 |
| ビデオフレーム | ビデオフレームごと | プラットフォームがハードウェアデコーダ→GPUテクスチャをサポートする場合ゼロコピー。それ以外はステージングバッファ。 |
| カラー絵文字 | 初回使用時 | CPUでラスタライズ、テクスチャにアップロード（Phase 2ではアトラスに）。 |

### 15.7.4 GPUメモリ追跡

```rust
pub struct GpuMemoryTracker {
    allocated: AtomicU64,
    /// デフォルト: 512MB、報告されたVRAMの75%が上限
    budget: u64,
}
```

予算超過時：未使用画像テクスチャを最初にエビクト（画像デコードキャッシュ（第22章）と連携）、次にレイヤーテクスチャ解像度を下げる（低DPIでレンダリングしアップスケール）。タブ破棄（第22章、メモリ圧迫）は破棄タブの全GPUリソースを解放。

## 15.8 フレームスケジューリング

### 15.8.1 フレームポリシー

フレームスケジューラはブラウザとアプリケーション両方のユースケースに対応する設定可能なポリシーで駆動：

```rust
pub enum FramePolicy {
    /// ディスプレイのvsyncに同期。視覚変更保留がなければフレームスキップ。
    /// elidex-browserのデフォルト。
    Vsync,

    /// vsync機会ごとに無条件レンダリング。
    /// ゲーム、リアルタイム可視化、連続アニメーション用。
    Continuous,

    /// コンテンツ変更時のみレンダリング（dirtyフラグ）。
    /// 最大電力効率。elidex-appのデフォルト。
    OnDemand,

    /// 固定フレームレート。ディスプレイのvsyncとは独立。
    FixedRate(u32),  // 例: 30
}
```

| ポリシー | コンポジタの動作 | ユースケース |
| --- | --- | --- |
| Vsync | vsyncで起床。新DisplayListまたは保留中のスクロール/アニメーションがあれば合成。なければスリープ。 | ブラウザタブ、大半のWebコンテンツ。 |
| Continuous | vsyncごとに起床。常に合成（変更がなくても）。 | ゲーム、WebGL/WebGPUアプリ、リアルタイムダッシュボード。 |
| OnDemand | dirtyフラグが立つまでスリープ（DOM変更、スクロール、アニメーション開始、リサイズ）。1フレームレンダリング後に再スリープ。 | ドキュメントビューアー、フォームアプリ、Electron系ツール。電力効率重視。 |
| FixedRate(n) | タイマーで起床（1/n秒）。vsyncに関係なく合成。 | デジタルサイネージ、特定レートでのビデオ再生、テスト。 |

```rust
// elidex-app設定
let app = elidex_app::App::new()
    .frame_policy(FramePolicy::OnDemand)   // 電力効率のデフォルト
    .build();

// ゲームアプリのオーバーライド
let game = elidex_app::App::new()
    .frame_policy(FramePolicy::Continuous)  // 最大フレームレート
    .build();
```

elidex-browserではポリシーはタブ単位：アクティブなフォアグラウンドタブはVsync、バックグラウンドタブは自動的にOnDemandに降格（タブがアクティベートされるまでレンダリングなし）。

### 15.8.2 パイプライン構成

メインスレッドとコンポジタスレッドはパイプライン動作：コンポジタがフレームNを描画中にメインスレッドがフレームN+1を準備。

```
時間 ──────────────────────────────────────────────────►

メインスレッド:  [──── Frame N ────][──── Frame N+1 ────][──── Frame N+2 ────]
                 JS│Style│Layout│Paint  JS│Style│Layout│Paint  ...
                          │ DL送信          │ DL送信
                          ▼                  ▼
コンポジタ:          [── Frame N ──]    [── Frame N+1 ──]
                      受信│合成│プレゼント  受信│合成│プレゼント
                               │                      │
ディスプレイ:                  vsync                  vsync
```

パイプライニングにより1フレームのレイテンシが追加（コンポジタは常に前フレームの出力を描画）されるが、スループットは大幅に向上 — メインスレッドはコンポジタの完了を待たず、コンポジタはメインスレッドを待たない。

bounded(1) DisplayPipelineチャネル（第6章）が同期を提供：コンポジタが前フレームを未消費なら、メインスレッドはキューイングせず新フレームをドロップ。バウンドなしのレイテンシ蓄積を防止。

### 15.8.3 可変リフレッシュレート

フレームスケジューラはDisplayCapabilities（第23章、RenderSurface）経由でディスプレイのリフレッシュレートに適応：

```rust
pub struct DisplayCapabilities {
    pub refresh: RefreshRate,
    pub hdr: bool,
    pub color_space: ColorSpace,
    pub scale_factor: f64,
}

pub enum RefreshRate {
    Fixed(u32),                        // 例: 60
    Variable { min: u32, max: u32 },   // 例: 48–120（ProMotion、FreeSync、G-Sync）
}
```

| ディスプレイタイプ | フレーム予算 | 動作 |
| --- | --- | --- |
| 60Hz固定 | 16.67ms | 標準。予算超過時フレームスキップ。 |
| 120Hz固定 | 8.33ms | 予算半分。スタイル/レイアウト/ペイントの高速化必要。 |
| 48–120Hz VRR | 可変 | 準備完了時即座にプレゼント。ディスプレイがリフレッシュ適応。8.33ms–20.83msのフレームすべてスムーズに表示。 |

VRRディスプレイでは、コンポジタは固定vsyncインターバルを待たずフレーム準備完了時に即座にプレゼント。ティアリング（VRRが処理）と待機による人為的レイテンシの両方を排除。

### 15.8.4 フレームタイミングとジャンク検出

各フレームがタイミングデータを記録：

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
    /// このフレームがターゲットを逃したか
    pub janked: bool,
}
```

長いフレーム（予算の50%超過）はPerformance Observer API（第14章）経由でLong Animation Frameエントリとして報告。Web開発者がジャンク源を特定可能。

## 15.9 コンポジタ駆動操作

コンポジタスレッド（第6章）上でメインスレッドの関与なく実行。重いJS実行中もスムーズ。

### 15.9.1 コンポジタアニメーション

| プロパティ | コンポジタのみ | 備考 |
| --- | --- | --- |
| `transform` | はい | レイヤー変換。レイアウトも再ペイントも不要。 |
| `opacity` | はい | レイヤーアルファ。再ペイント不要。 |
| `filter`（サブセット） | はい | レイヤーテクスチャへのGPUシェーダー効果。 |
| `background-color` | いいえ | 再ペイント必要。 |
| `width`、`height` | いいえ | レイアウト＋再ペイント必要。 |
| `top`、`left` | いいえ | レイアウト必要。コンポジタパスには`transform: translate()`を使用。 |

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

コンポジタアニメーション可能プロパティのアニメーション開始時、メインスレッドが要素を独自レイヤーに昇格（未昇格の場合）し、アニメーション定義をコンポジタに送信。コンポジタがフレームごとに独立して値を補間。

JSがコンポジタアニメーションを中断した場合（例：`element.style.transform = ...`）、コンポジタアニメーションはキャンセルされ制御がメインスレッドに戻る。

### 15.9.2 コンポジタ駆動スクロール

スクロールは最も一般的なコンポジタ操作：

```
ユーザーがスクロール（タッチ/ホイール）
  │
  ├─ コンポジタが入力を受信（第6章ファストパス）
  ├─ ヒットテスト：最内のスクロール可能レイヤーを検索
  ├─ スクロール物理適用（モメンタム、摩擦、オーバースクロールバウンス）
  ├─ レイヤーのスクロールオフセット更新
  │   ├─ スクロールコンテンツレイヤーを移動
  │   ├─ position:fixedレイヤーは影響なし
  │   ├─ position:stickyレイヤーは範囲にクランプ
  │   └─ スクロールバー位置を更新
  ├─ 合成してプレゼント（メインスレッド不要）
  └─ メインスレッドに新スクロール位置を非同期通知
      └─ メインスレッドがscrollイベント発火、遅延読み込みトリガー
```

passiveスクロールリスナー（またはリスナーなし）では、このフロー全体がコンポジタのみ。non-passiveリスナーではコンポジタが楽観的にスクロールし、メインスレッドがpreventDefault()可能 — ただし1フレームのレイテンシ追加。

### 15.9.3 スクロール連動効果

| 機能 | コンポジタの処理 |
| --- | --- |
| `position: fixed` | スクロールでレイヤーは移動しない。 |
| `position: sticky` | レイヤー位置が通常位置としきい値の間にクランプ。 |
| `background-attachment: fixed` | 背景レイヤーがコンテンツとスクロールしない。 |
| `scroll-snap` | スクロール減速中にスナップポイント適用。 |
| `overscroll-behavior` | `contain`でチェーン防止、`none`でチェーン＋オーバースクロール効果防止。 |

## 15.10 プラットフォーム統合

### 15.10.1 RenderSurface接続

パイプラインはRenderSurface（第23章）経由でプラットフォームに接続：

| プラットフォーム | サーフェス | GPUバックエンド |
| --- | --- | --- |
| macOS | CAMetalLayer | Metal |
| Linux (Wayland) | wl_surface | Vulkan |
| Linux (X11) | X11 Window | Vulkan |
| Windows | HWND | DX12またはVulkan |
| ヘッドレス | オフスクリーンテクスチャ | ソフトウェア（Vello CPU） |

### 15.10.2 DPIスケーリング

すべてのレイアウトは論理（CSS）ピクセルを使用。プラットフォームのスケールファクターはラスタライゼーション時に適用：

```
CSSピクセル × scale_factor → 物理ピクセル → Velloが物理解像度でレンダリング
```

レイヤーテクスチャは鮮明なHiDPIレンダリングのため物理ピクセル寸法で割り当て。スケールファクター変更（ディスプレイ間のウィンドウ移動）は完全な再レイアウトとテクスチャ再割当をトリガー。

## 15.11 elidex-appレンダリング

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| フレームポリシーデフォルト | Vsync | OnDemand |
| GPU Process | 分離（Phase 1–3） | マージ（SingleProcess） |
| レイヤー予算 | 管理あり | 高めのデフォルト（アプリがコンテンツ制御） |
| コンポジタアプローチ | B（メッセージパッシング） | C可能（共有LayerTree） |
| ソフトウェアフォールバック | 自動 | 設定可能（アプリがGPU必須の場合あり） |

```rust
let app = elidex_app::App::new()
    .frame_policy(FramePolicy::OnDemand)
    .render_backend(RenderBackend::Auto)
    .gpu_memory_budget(256 * 1024 * 1024)
    .build();
```
