
# 18. 画像デコードパイプライン

## 18.1 概要

画像は通常Webページで最も重いリソース。デコードパイプラインはフォーマット検出、オフメインスレッドデコード、メモリ効率の良いキャッシュ、プログレッシブレンダリング、GPU転送を処理しなければならない — すべてレンダリングパイプラインをブロックせずに。

```
ネットワークレスポンス（バイト列）
  │
  ├─ フォーマット検出（マジックバイト）
  ├─ ImageDecoder選択（フォーマット毎）
  ├─ rayonプールでデコード（第6章）
  │   ├─ プログレッシブ：利用可能な部分フレームを順次生成
  │   └─ フル：完全なDecodedImageを生成
  ├─ 画像デコードキャッシュ（第22章）
  │   └─ キー：(URL, ターゲットサイズ, DPI)
  ├─ TextureManager経由でGPUアップロード（第15章§15.7）
  │   └─ StagingBeltでCPU→GPU転送
  └─ コンポジタがレイヤー内でテクスチャを描画
```

## 18.2 フォーマットサポート

### 18.2.1 Core / Compat分類

| フォーマット | 分類 | デコーダクレート | 備考 |
| --- | --- | --- | --- |
| PNG | Core | png | ロスレス。ユビキタス。 |
| JPEG | Core | zune-jpeg | ロッシー。写真の標準。SIMD最適化デコード。 |
| WebP | Core | webp | ロッシー+ロスレス+アニメーション。全主要ブラウザ対応。 |
| AVIF | Core | ravif / libdav1d | AV1ベース。次世代標準。 |
| GIF | Core | gif | アニメーションGIFは依然広く使用。 |
| APNG | Core | png（APNG拡張） | アニメーションPNG。全主要ブラウザ対応。 |
| SVG-as-image | Core | （第19章） | `<img src="icon.svg">`がターゲットサイズでラスタライズ。 |
| ICO/CUR | Compat | ico | ファビコン。レガシーだがfavicon.icoは依然多い。 |
| BMP | Compat | image（bmpモジュール） | モダンWebでは稀。 |
| TIFF | Compat | image（tiffモジュール） | Web上では極めて稀。 |
| JPEG XL | 未対応 | — | ブラウザサポートが不安定（Chrome実装後削除）。仕様安定時に再評価。 |

elidex-appモードではCompatフォーマットをコンパイル時に除外可能、バイナリサイズを削減。

### 18.2.2 フォーマット検出

フォーマットはファイル拡張子やMIMEタイプではなくマジックバイト（コンテンツスニッフィング）で決定：

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

## 18.3 ImageDecoderトレイト

デコーダはトレイトで抽象化され、性能が必要な場合にRustクレート実装をプラットフォームネイティブデコーダに差し替え可能。

```rust
pub trait ImageDecoder: Send + Sync {
    /// サポートフォーマット。
    fn format(&self) -> ImageFormat;

    /// ピクセルデータをデコードせずに画像ヘッダーをデコードし
    /// 寸法とメタデータを取得。
    fn read_header(&self, data: &[u8]) -> Result<ImageHeader, DecodeError>;

    /// RGBAビットマップへのフルデコード。
    fn decode(&self, data: &[u8], params: DecodeParams) -> Result<DecodedImage, DecodeError>;

    /// プログレッシブデコード：利用可能バイトから部分的結果を返す。
    /// 有用な出力に十分なデータがなければNoneを返す。
    fn decode_progressive(
        &self,
        data: &[u8],
        params: DecodeParams,
    ) -> Result<Option<PartialImage>, DecodeError>;

    /// アニメーションフォーマット用：指定インデックスのフレームをデコード。
    fn decode_frame(
        &self,
        data: &[u8],
        frame_index: usize,
        params: DecodeParams,
    ) -> Result<DecodedFrame, DecodeError>;

    /// アニメーションフォーマット用：総フレーム数とフレーム遅延テーブル。
    fn animation_info(&self, data: &[u8]) -> Result<Option<AnimationInfo>, DecodeError>;
}

pub struct DecodeParams {
    /// ダウンスケールデコード用ターゲットサイズ（JPEGは1/2, 1/4, 1/8でネイティブデコード可能）。
    pub target_size: Option<Size>,
    /// ターゲット色空間。
    pub color_space: ColorSpace,
    /// DPIスケールファクター（SVG-as-imageラスタライズ用）。
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
    pub pixels: Vec<u8>,       // ソースに応じてRGBA8またはRGBA16
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
    pub color_space: ColorSpace,
}
```

### 18.3.1 デフォルト実装

初期実装はすべてのフォーマットにRustクレートを使用：

| フォーマット | クレート | 備考 |
| --- | --- | --- |
| PNG / APNG | `png` | Pure Rust。APNGフレーム抽出。 |
| JPEG | `zune-jpeg` | SIMD最適化。1/2, 1/4, 1/8スケールデコードサポート。 |
| WebP | `webp` | libwebpへのRustバインディング、またはpure-Rust `webp`クレート。 |
| AVIF | `ravif` / `dav1d` | ravifはpure Rust、dav1d（C）はハードウェアアクセラレートAV1デコード。 |
| GIF | `gif` | Pure Rust。ストリーミングフレームデコード。 |
| ICO | `ico` | Pure Rust。埋め込みPNGまたはBMP。 |
| BMP | `image`（bmp） | Pure Rust。 |

### 18.3.2 プラットフォームデコーダオプション

プロファイリングでRustデコーダがボトルネックと判明した場合（特にモバイルでのJPEGとAVIF）、プラットフォームデコーダを代替可能：

| プラットフォーム | デコーダ | 優位点 |
| --- | --- | --- |
| macOS / iOS | ImageIO (CGImageSource) | Apple SiliconでのハードウェアJPEG/HEIFデコード。 |
| Windows | WIC (Windows Imaging Component) | GPUアシストデコード。 |
| Linux | プラットフォームライブラリは様々 | SIMD付きlibjpeg-turboが一般的。 |
| Android | Android Bitmap decoder | ハードウェアデコードパス。 |

プラットフォームデコーダは同じ`ImageDecoder`トレイトを実装。選択は設定可能：

```rust
pub enum DecoderStrategy {
    /// Rustクレートのみ。最大の可搬性、C依存なし。
    RustOnly,
    /// プラットフォームデコーダ優先、Rustにフォールバック。
    PlatformPreferred,
    /// 特定フォーマットのみプラットフォームを使用。
    Mixed(HashMap<ImageFormat, DecoderBackend>),
}
```

## 18.4 デコードパイプライン

### 18.4.1 オフメインスレッドデコード

すべての画像デコードはRendererのrayonプール（第6章）で実行。メインスレッドがデコードでブロックされない：

```
メインスレッド                         rayonプール
  │                                    │
  ├─ <img>パースまたはsrc変更          │
  ├─ ネットワークから画像をリクエスト    │
  │  （Browser Process IPC経由）       │
  │                                    │
  ├─ バイト到着（ストリーミング）        │
  ├─ dispatch_decode(bytes) ──────────▶│
  │                                    ├─ detect_format()
  │                                    ├─ read_header()
  │  ◄── ImageHeader ─────────────────│  （レイアウト用固有サイズ）
  │                                    │
  │  [レイアウトが固有サイズを使用]      ├─ decode()またはdecode_progressive()
  │                                    │
  │  ◄── DecodedImage ────────────────│
  ├─ デコードキャッシュに挿入           │
  ├─ GPUにアップロード（TextureManager）│
  ├─ レイヤー無効化（再ペイント）       │
  └─ コンポジタがテクスチャを描画       │
```

### 18.4.2 ヘッダーファーストレイアウト

十分なバイトが到着次第（通常最初の数百バイト）画像ヘッダーを読取。画像全体のデコード前にレイアウトに固有寸法を提供：

```rust
pub struct PendingImage {
    pub url: Url,
    pub state: ImageLoadState,
}

pub enum ImageLoadState {
    /// ネットワークレスポンス待ち。
    Loading,
    /// ヘッダーデコード済み。固有サイズ既知。レイアウト続行可能。
    HeaderReady {
        header: ImageHeader,
        bytes_so_far: Vec<u8>,
    },
    /// 部分的にデコード済み（プログレッシブJPEG）。
    Progressive {
        header: ImageHeader,
        partial: PartialImage,
        bytes_so_far: Vec<u8>,
    },
    /// 完全にデコード済みでキャッシュ済み。
    Complete {
        cache_key: ImageCacheKey,
        texture: TextureHandle,
    },
    /// デコードまたはネットワークエラー。
    Error(ImageError),
}
```

ヘッダーなし（ネットワーク停止、無効フォーマット）の場合、`<img>`要素は明示的な`width`/`height`属性（指定されている場合）を使用するか、プレースホルダーとしてレンダリング。HTML仕様が常に画像寸法の指定を推奨する理由。

### 18.4.3 プログレッシブJPEGレンダリング

プログレッシブJPEGは最初のスキャンでぼやけたプレビューを配信し、後続スキャンで精細化。Elidexは到着した部分結果を順次レンダリング：

```
スキャン1（DC係数）：ぼやけたフルサイズプレビュー
  → GPUにアップロード、即座に表示
スキャン2：改善された品質
  → 再アップロード、レイヤー無効化
...
最終スキャン：フル品質
  → 最終アップロード、最終無効化
```

プログレッシブ更新ごとにGPUテクスチャを置換。コンポジタは利用可能なものを描画。ユーザーは空白ではなく徐々にシャープになる画像を見る。

### 18.4.4 ダウンスケールデコード

レンダリングサイズが固有サイズより小さい場合、フル解像度デコードはメモリとCPU時間を浪費。JPEGはネイティブダウンスケールデコード（1/2, 1/4, 1/8）をサポート：

```rust
fn compute_decode_size(intrinsic: Size, rendered: Size, scale_factor: f64) -> Size {
    let physical = Size {
        width: (rendered.width * scale_factor).ceil() as u32,
        height: (rendered.height * scale_factor).ceil() as u32,
    };

    // JPEG：物理サイズをカバーする最小のネイティブスケールを検索
    // 他フォーマット：フルサイズでデコード後にダウンスケール
    physical
}
```

非JPEGフォーマットではフルデコード後にGPU側またはCPU側でダウンスケール。`image-rendering` CSSプロパティがフィルタを選択：

| `image-rendering` | フィルタ | ユースケース |
| --- | --- | --- |
| `auto`（デフォルト） | バイリニア | 汎用。スムーズなダウンスケール。 |
| `smooth` | Lanczos3 | 高品質。写真。 |
| `pixelated` | 最近傍 | ピクセルアート、レトログラフィクス。 |
| `crisp-edges` | 最近傍 | pixelatedに類似。 |

## 18.5 レスポンシブイメージ

### 18.5.1 ソース選択

`srcset`と`<picture>`要素により、ビューポートサイズ、デバイスピクセル比、アートディレクションに基づき適切な画像ソースをブラウザが選択：

```html
<!-- 解像度切り替え -->
<img srcset="photo-320w.jpg 320w,
             photo-640w.jpg 640w,
             photo-1280w.jpg 1280w"
     sizes="(max-width: 600px) 100vw, 50vw"
     src="photo-640w.jpg"
     alt="Photo">

<!-- アートディレクション -->
<picture>
  <source media="(max-width: 600px)" srcset="photo-portrait.jpg">
  <source media="(min-width: 601px)" srcset="photo-landscape.jpg">
  <img src="photo-landscape.jpg" alt="Photo">
</picture>
```

ソース選択アルゴリズム：

```rust
pub fn select_source(
    srcset: &[SrcsetEntry],
    sizes: &SizesAttribute,
    viewport: &Viewport,
    device_pixel_ratio: f64,
) -> &SrcsetEntry {
    // 1. sizesを評価して実効画像幅を取得
    let effective_width = sizes.evaluate(viewport);

    // 2. 各候補のターゲット密度を計算
    // 3. device_pixel_ratioに最も近い候補を選択
    //    （下回らない：やや大きい方を優先）
    srcset.iter()
        .min_by_key(|entry| {
            let density = entry.width as f64 / effective_width;
            let diff = density - device_pixel_ratio;
            if diff < 0.0 { f64::MAX as i64 } else { (diff * 1000.0) as i64 }
        })
        .unwrap_or(&srcset[0])
}
```

### 18.5.2 リサイズ時のソース変更

ビューポートまたはコンテナサイズ変更時（ウィンドウリサイズ、コンテナクエリ）、ソースの再評価が必要な場合がある。より高解像度のソースが適切になった場合、新しいフェッチ+デコードを開始。新しい画像が準備できるまで古い画像を表示し続ける（空白フラッシュなし）。

## 18.6 遅延読み込み

### 18.6.1 `loading="lazy"`

`loading="lazy"`の画像はビューポートに入るか近づくまでフェッチされない：

```rust
pub struct LazyLoadController {
    /// 読み込みトリガーのビューポート端からの距離。
    /// デフォルト：垂直1250px、水平2500px（Chrome準拠）。
    pub root_margin: EdgeInsets,
}

impl LazyLoadController {
    pub fn should_load(&self, element_rect: Rect, viewport: Rect) -> bool {
        let expanded_viewport = viewport.expand(self.root_margin);
        expanded_viewport.intersects(&element_rect)
    }
}
```

コントローラは内部的にIntersectionObserverで駆動。スクロール位置変更時、コンポジタがメインスレッドに通知（第15章§15.9.2）し、保留中の遅延画像をチェック。

### 18.6.2 `loading="eager"`（デフォルト）

Eager画像はパース中に即座にフェッチ開始。プリロードスキャナ（第11章）がパーサーに先行して`<img>`要素を発見し早期フェッチを開始。

### 18.6.3 `decoding`属性

| 値 | 動作 |
| --- | --- |
| `auto`（デフォルト） | エンジンが決定。現在は`async`と同等。 |
| `async` | デコードがレンダリングをブロックしない。準備完了時に画像表示。 |
| `sync` | 画像表示前にデコード完了。この要素のレンダリングをブロック。画像なしフラッシュ回避が重要な場合に使用。 |

## 18.7 画像デコードキャッシュ

画像デコードキャッシュ（第22章§22.7メモリキャッシュ）はデコード済みビットマップを保存し、画像がスクロールで再表示されたり要素間で再利用される際の再デコードを回避。

### 18.7.1 キャッシュキー

```rust
#[derive(Hash, Eq, PartialEq)]
pub struct ImageCacheKey {
    pub url: Url,
    /// デコードサイズ（ダウンスケールデコードの場合固有サイズと異なる可能性）。
    pub decoded_size: Size,
    /// デコード時のスケールファクター。
    pub scale_factor: OrderedFloat<f64>,
}
```

同一URLで異なるサイズは別キャッシュエントリ。同一URLの100×100サムネイルと1000×1000ヒーロー画像は独立してキャッシュ。

### 18.7.2 エビクション

```rust
pub struct ImageDecodeCache {
    entries: LinkedHashMap<ImageCacheKey, CacheEntry>,
    total_bytes: u64,
    /// デフォルト：128MB。GpuMemoryTracker（第15章§15.7.4）と連携。
    budget: u64,
}

pub struct CacheEntry {
    pub decoded: DecodedImage,
    pub texture: Option<TextureHandle>,  // GPUアップロード済みコピー
    pub last_access: Instant,
    pub byte_size: u64,
}
```

エビクションポリシー：LRU。`total_bytes`が`budget`を超過時、最も最近使用されていないエントリをエビクト。エビクトされたエントリはCPUメモリ（デコード済みピクセル）とGPUメモリ（TextureManager経由でテクスチャ）の両方を解放。

システムメモリ圧迫時（第22章メモリ圧迫処理）、キャッシュ予算を一時的に削減。エビクトされた画像はHTTPキャッシュまたはネットワークからオンデマンドで再デコード。

### 18.7.3 キャッシュ連携

```
HTTPキャッシュ（第22章）     画像デコードキャッシュ        GPUテクスチャ（第15章）
  圧縮バイト列          →    デコード済みピクセル     →    wgpu::Texture
  （ディスク/メモリ）         （メモリ、LRUエビクト）       （VRAM、キャッシュと共にエビクト）
```

エビクション時、デコードキャッシュエントリとそのGPUテクスチャが同時に解放。画像が再度必要になった場合、HTTPキャッシュから再デコード（ネットワークフェッチを回避）してGPUに再アップロード。

## 18.8 アニメーション画像

### 18.8.1 ImageAnimationScheduler

アニメーション画像（GIF、APNG、animated WebP）はCSS/JSアニメーションシステム（第17章）とは独立した独自のフレームスケジューリングを持つ。各アニメーション画像がフレームタイミングを管理する`ImageAnimationScheduler`を持つ：

```rust
pub struct ImageAnimationScheduler {
    frames: Vec<FrameInfo>,
    current_frame: usize,
    next_frame_time: Instant,
    play_count: u32,           // 0 = 無限
    completed_loops: u32,
    state: AnimPlayState,
}

pub struct FrameInfo {
    pub delay: Duration,       // フレームごとの遅延（GIF/APNGメタデータから）
    pub dispose: DisposeOp,    // 次フレーム前のクリア方法
    pub blend: BlendOp,        // 前フレームへの合成方法
}

pub enum DisposeOp {
    None,            // 現在フレームを残す
    Background,      // 背景色にクリア
    Previous,        // 前フレームに復元
}

impl ImageAnimationScheduler {
    /// FrameProducer（第17章）が各フレームで呼び出し。
    /// 新しいフレームをデコード・表示すべき場合はtrueを返す。
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

### 18.8.2 フレームデコード戦略

アニメーション画像は数百フレームになりうる。全フレームを事前デコードすると過大なメモリ消費。代わりに小さな先読みバッファでオンデマンドデコード：

```rust
pub struct AnimatedImageBuffer {
    /// 生の圧縮データ（再デコード用にメモリに保持）。
    source_data: Arc<Vec<u8>>,
    /// デコード済みフレームリングバッファ。通常2–4フレーム。
    decoded_frames: VecDeque<(usize, DecodedImage)>,
    /// メモリに保持するデコード済みフレームの最大数。
    buffer_size: usize,  // デフォルト: 3
}
```

フロー：
1. スケジューラが次フレームインデックスを決定。
2. フレームがバッファ内にあれば直接使用。
3. なければrayonプールにデコードをディスパッチ。
4. デコード完了時、GPUテクスチャにアップロードしレイヤー無効化。
5. バッファが満杯なら最古フレームをエビクト。

`DisposeOp::Previous`のフレームでは前フレームを保持する必要あり。

### 18.8.3 可視性最適化

画面外のアニメーション画像（ビューポート外にスクロール、バックグラウンドタブ内）はアニメーションを一時停止。FrameProducerがそのtickをスキップ。再びビューに入った時、現在フレームからアニメーション再開。

タブレベルのFramePolicy（第15章§15.8.1）と連携：バックグラウンドタブはOnDemandモードを使用し、バックグラウンドタブ内のアニメーション画像はCPU消費ゼロ。

## 18.9 Blob URLとData URL

### 18.9.1 Blob URL

`URL.createObjectURL(blob)`がメモリ内データを参照する`blob:` URLを作成。画像デコードパイプラインはBrowser Process（Blobストアを所有）経由でblob URLを解決：

```
Renderer: <img src="blob:https://example.com/uuid">
  │  IPC: ResolveBlobUrl { url }
  ▼
Browser Process: BlobStore
  │  blobデータを返却（またはディスクバックblobへの参照）
  ▼
Renderer: 通常の画像としてデコード
```

### 18.9.2 Data URL

`data:image/png;base64,...` URLは画像データをインラインに含む。ネットワークフェッチなしで直接デコード。Base64デコード + 画像デコードの両方がrayonプールで実行。

## 18.10 エラー処理

| エラー | 動作 |
| --- | --- |
| ネットワークエラー（404、タイムアウト） | 壊れた画像アイコン表示。`<img>`で`error`イベント発火。 |
| 未サポートフォーマット | 壊れた画像アイコン表示。コンソール警告。 |
| 破損画像データ | 正常にデコードできた部分を表示（部分的）、または壊れた画像アイコン。`error`イベント発火。 |
| デコードOOM | デコード解像度を下げて再試行。それでも失敗なら壊れた画像アイコン。 |
| GPUアップロード失敗 | この画像のソフトウェア合成にフォールバック。 |

壊れた画像アイコンはエンジン組み込みのSVG（ネットワークから読み込まない）。

## 18.11 elidex-app画像

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| フォーマットサポート | Core + Compat | Coreのみ（デフォルト）。Compatはビルド時オプトイン。 |
| デコーダ戦略 | RustOnly（デフォルト） | 設定可能（RustOnly / PlatformPreferred / Mixed） |
| 遅延読み込み | `loading="lazy"`属性 | 同一API。ただしアプリがビューポートセマンティクスを制御。 |
| 画像キャッシュ予算 | 管理あり（128MBデフォルト） | 設定可能 |
| アニメーション画像 | 可視性最適化付き自動再生 | 同一動作。アプリはAPI経由で一時停止/再開可能。 |

```rust
let app = elidex_app::App::new()
    .decoder_strategy(DecoderStrategy::PlatformPreferred)
    .image_cache_budget(64 * 1024 * 1024)  // 軽量アプリ向け64MB
    .build();
```
