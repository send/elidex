
# 20. メディアパイプライン

## 20.1 概要

メディアパイプラインはオーディオ/ビデオ再生（`<audio>`、`<video>`）、リアルタイムオーディオ処理（Web Audio API）、メディアキャプチャ（getUserMedia）、ストリーミング拡張（MSE）、暗号化メディア（EME）を処理。コーデックネゴシエーション、ハードウェアアクセラレーション、リアルタイムスケジューリング、クロスプロセス協調を含む、ブラウザエンジンで最も複雑なサブシステムの一つ。

```
┌─────────────────────────────────────────────────────────────────────┐
│ Rendererプロセス                                                     │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────────┐ │
│  │ HTMLMediaElement│ │ MediaSource  │  │ Web Audio API             │ │
│  │ (<video>,     │  │ Extensions   │  │ (AudioContext,            │ │
│  │  <audio>)     │  │ (MSE)        │  │  AudioNodeグラフ)          │ │
│  └──────┬───────┘  └──────┬───────┘  └─────────────┬─────────────┘ │
│         │                 │                         │               │
│         ▼                 ▼                         ▼               │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ MediaPlayer                                                  │   │
│  │  ├── MediaDemuxer（コンテナ解析）                              │   │
│  │  ├── DecoderProxy（Decoderプロセスへの IPC）                    │   │
│  │  ├── AudioRenderer（オーディオデバイスまたはWeb Audioに供給）    │   │
│  │  └── VideoRenderer（コンポジタレイヤーに供給）                  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                          │ IPC                                      │
├──────────────────────────┼──────────────────────────────────────────┤
│ Decoderプロセス           │（サンドボックス化ユーティリティ）         │
│                          ▼                                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ DecoderHost                                                  │   │
│  │  ├── ソフトウェアデコーダ（dav1d, libvpx, opus, vorbis, flac）│   │
│  │  ├── プラットフォームデコーダ（AVFoundation, MediaFoundation, │   │
│  │  │    VA-API）                                                │   │
│  │  └── CdmProxy（EME → CDMプラグイン、将来）                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ GPUプロセス                                                          │
│  ├── ハードウェアデコード済みフレーム（ゼロコピーテクスチャインポート）│
│  └── ビデオフレーム → コンポジタレイヤー（第15章）                    │
└─────────────────────────────────────────────────────────────────────┘
```

## 20.2 コーデック戦略

### 20.2.1 デコーダトレイト

すべてのデコーダが共通トレイトを実装。ImageDecoder（第18章、ADR #31）やHttpTransport（第10章）と同パターン：

```rust
pub trait MediaDecoder: Send {
    /// コーデック設定でデコーダを初期化。
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), DecoderError>;

    /// エンコード済みパケットをデコードに投入。
    fn decode(&mut self, packet: EncodedPacket) -> Result<(), DecoderError>;

    /// デコード済みフレームを取得。decode呼び出しごとに0以上のフレームを返す。
    fn receive_frame(&mut self) -> Result<Option<DecodedFrame>, DecoderError>;

    /// デコーダをフラッシュ（ストリーム終端またはシーク）。
    fn flush(&mut self) -> Result<(), DecoderError>;

    /// デコーダ能力を問い合わせ。
    fn capabilities(&self) -> DecoderCapabilities;
}

pub struct DecoderConfig {
    pub codec: Codec,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub extra_data: Option<Bytes>,  // コーデック固有の初期化データ
}

pub enum DecodedFrame {
    Video(VideoFrame),
    Audio(AudioFrame),
}

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub planes: Vec<Plane>,
    pub timestamp: Duration,
    pub duration: Duration,
    /// ハードウェアデコード：GPUプロセスからの不透明テクスチャハンドル
    pub hw_texture: Option<GpuTextureHandle>,
}

pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
    pub timestamp: Duration,
    pub duration: Duration,
}
```

### 20.2.2 Core / Compatコーデック分類

**ビデオコーデック：**

| コーデック | 分類 | デコーダ | 備考 |
| --- | --- | --- | --- |
| VP8 | Core | libvpx（Rustバインディング） | WebM標準、ロイヤリティフリー |
| VP9 | Core | libvpx（Rustバインディング） | YouTube主要コーデック |
| AV1 | Core | dav1d（Rustバインディング） | 次世代ロイヤリティフリー、第18章とdav1dをAVIFで共有 |
| H.264/AVC | Core | プラットフォームデコーダ | 特許負担あり；OSに委譲（AVFoundation, MediaFoundation, VA-API） |
| H.265/HEVC | Core | プラットフォームデコーダ | 特許負担あり；プラットフォーム専用 |
| MPEG-1 | Compat | ソフトウェア（オプション） | レガシー |
| WMV | 非サポート | — | プロプライエタリ、Web使用率微小 |

**オーディオコーデック：**

| コーデック | 分類 | デコーダ | 備考 |
| --- | --- | --- | --- |
| Opus | Core | opus（Rustバインディング） | WebRTC必須、最良の汎用 |
| Vorbis | Core | lewton（純粋Rust）またはlibvorbis | Oggコンテナ標準 |
| AAC | Core | プラットフォームデコーダ | 特許負担あり；OSに委譲 |
| FLAC | Core | claxon（純粋Rust）またはlibflac | ロスレス |
| MP3 | Core | minimp3（Rustバインディング） | 遍在、特許失効済み（2017年） |
| WMA | 非サポート | — | プロプライエタリ |

**コンテナフォーマット：**

| コンテナ | 分類 | 備考 |
| --- | --- | --- |
| MP4 / fMP4 | Core | H.264/H.265/AV1/AACコンテナ |
| WebM | Core | VP8/VP9/AV1/Opus/Vorbisコンテナ |
| Ogg | Core | Vorbis/Opus/FLACコンテナ |
| WAV | Core | PCMオーディオ |
| MPEG-TS | Compat | HLSセグメント |
| AVI | Compat | レガシー |
| FLV | 非サポート | Flashレガシー |

### 20.2.3 デコーダ選択戦略

```rust
pub enum DecoderStrategy {
    /// 純粋ソフトウェアデコーダ（Rust/Cライブラリ）。最大可搬性。
    SoftwareOnly,
    /// プラットフォームハードウェアデコーダ優先、ソフトウェアにフォールバック。
    PlatformPreferred,
    /// コーデック単位選択（デフォルト）。
    Mixed(HashMap<Codec, DecoderPreference>),
}

pub enum DecoderPreference {
    Software,
    Platform,
    /// プラットフォーム優先、ソフトウェアにフォールバック
    PlatformWithFallback,
}
```

デフォルト戦略（`Mixed`）：
- VP8/VP9: ソフトウェア（libvpx）。一部GPUでプラットフォームHWデコード可能；検出時使用。
- AV1: ソフトウェア（dav1d）。新世代GPU（Intel Gen12+、Apple M3+）でプラットフォームHWデコード；検出時使用。
- H.264/H.265: プラットフォーム専用。ソフトウェアデコーダ未同梱（特許ライセンス）。
- Opus/Vorbis/FLAC/MP3: 常にソフトウェア（軽量、特許問題なし、CPU コスト無視可能）。
- AAC: プラットフォーム専用。

### 20.2.4 プラットフォームデコーダアダプタ

```rust
/// macOS / iOS
pub struct AvFoundationDecoder {
    // ビデオ用VTDecompressionSession
    // オーディオ用AudioConverter
}

/// Windows
pub struct MediaFoundationDecoder {
    // ビデオおよびオーディオ用IMFTransform
}

/// Linux
pub struct VaApiDecoder {
    // ハードウェアアクセラレーション ビデオデコード用VA-API
    // テクスチャインポートにGPUプロセス連携が必要
}

/// Linuxオーディオのフォールバック
pub struct PulseAudioDecoder {
    // VA-APIオーディオが存在しないLinuxでのAAC用GStreamerパイプライン
}
```

各アダプタが`MediaDecoder`トレイトを実装し、ランタイムのプラットフォームと利用可能なハードウェアに基づいて選択。

## 20.3 メディアデマクサ

デマクサがコンテナフォーマットを解析しエンコード済みパケットを抽出：

```rust
pub trait MediaDemuxer: Send {
    /// メディアソースを開きコンテナヘッダを読み取り。
    fn open(&mut self, source: MediaSource) -> Result<MediaInfo, DemuxError>;

    /// 次のエンコード済みパケットを読み取り。
    fn read_packet(&mut self) -> Result<Option<EncodedPacket>, DemuxError>;

    /// タイムスタンプにシーク。実際のシーク位置を返す。
    fn seek(&mut self, timestamp: Duration) -> Result<Duration, DemuxError>;
}

pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub tracks: Vec<TrackInfo>,
    pub is_seekable: bool,
}

pub struct TrackInfo {
    pub id: TrackId,
    pub kind: TrackKind,
    pub codec: Codec,
    pub config: DecoderConfig,
    pub language: Option<String>,
    pub label: Option<String>,
}

pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
}

pub enum MediaSource {
    /// URLベース：データがfetch（第10章）経由でByteStream（第21章）として到着
    Stream(ByteStream),
    /// MSE：SourceBufferがセグメントを提供
    Mse(MseSource),
}
```

デマクサ実装：

| コンテナ | 実装 | 備考 |
| --- | --- | --- |
| MP4 / fMP4 | mp4parse（Mozilla、純粋Rust） | MPEG-DASHセグメントも処理 |
| WebM | Rust WebMパーサー | Matroskaサブセット |
| Ogg | oggクレート（純粋Rust） | ストリーミングフレンドリー |
| WAV | インラインパーサー | 自明なフォーマット |
| MPEG-TS（compat） | TSパーサー | HLSセグメント |

## 20.4 メディアプレーヤー

### 20.4.1 アーキテクチャ

MediaPlayerは単一の`<video>`または`<audio>`要素の中央コーディネーター：

```rust
pub struct MediaPlayer {
    /// 現在の状態
    state: PlayerState,
    /// 専用スレッドで動作するデマクサ
    demuxer: Box<dyn MediaDemuxer>,
    /// デコーダプロセスへのプロキシ
    decoder_proxy: DecoderProxy,
    /// オーディオ出力
    audio_renderer: AudioRenderer,
    /// ビデオフレームスケジューラ
    video_renderer: VideoRenderer,
    /// A/V同期クロック
    clock: MediaClock,
    /// バッファ済み時間範囲
    buffered: TimeRanges,
    /// 再生速度
    playback_rate: f64,
    /// 音量とミュート状態
    volume: f64,
    muted: bool,
}

pub enum PlayerState {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Seeking(Duration),
    Ended,
    Error(MediaError),
}
```

### 20.4.2 再生パイプライン

```
データソース（ネットワーク / MSE）
  │
  ▼
MediaDemuxer（コンテナ解析）
  │  エンコード済みオーディオ/ビデオパケットを抽出
  ▼
DecoderProxy ──── IPC ───► Decoderプロセス
  │                         │  オーディオ/ビデオをデコード
  │  ◄── デコード済みフレーム ──┤
  ▼
┌─────────────┐  ┌──────────────┐
│AudioRenderer│  │VideoRenderer │
│（オーディオ  │  │（フレームキュー│
│  デバイスまた│  │  + コンポジタ │
│  はWeb Audio）│ │  レイヤー）   │
└──────┬──────┘  └──────┬───────┘
       │                │
       ▼                ▼
  オーディオ出力   コンポジタ（第15章）
  （プラットフォームAPI）（ビデオテクスチャレイヤー）
```

### 20.4.3 A/V同期

MediaClockがオーディオとビデオ間の同期を駆動：

```rust
pub struct MediaClock {
    /// 参照クロック：オーディオデバイス位置（最も正確）
    audio_position: Arc<AtomicU64>,  // マイクロ秒
    /// フォールバック：オーディオがミュート/不在時のシステムタイマー
    system_base: Instant,
    system_offset: Duration,
    /// 再生速度倍率
    rate: f64,
}

impl MediaClock {
    /// 現在のメディア時間。
    pub fn position(&self) -> Duration {
        // オーディオデバイスクロックを優先（ハードウェア駆動、ドリフトフリー）
        // オーディオ出力がない場合はシステムクロックにフォールバック
        let audio_pos = self.audio_position.load(Ordering::Relaxed);
        if audio_pos > 0 {
            Duration::from_micros(audio_pos)
        } else {
            let elapsed = self.system_base.elapsed();
            self.system_offset + elapsed.mul_f64(self.rate)
        }
    }
}
```

VideoRendererがクロックを使用してフレーム提示をスケジュール：

```rust
impl VideoRenderer {
    fn select_frame(&self, clock: &MediaClock) -> Option<&VideoFrame> {
        let target = clock.position();

        // ターゲットに最も近い（かつ超えない）タイムスタンプのフレームを検索
        // 遅延フレームをドロップ（1フレーム時間以上遅れ）
        // 早いフレームはタイムスタンプまで保持
        self.frame_queue.iter()
            .filter(|f| f.timestamp <= target)
            .last()
    }
}
```

オーディオがマスタークロック。ビデオはオーディオタイミングに合わせて調整。ビデオが遅れた場合（デコーダが遅い）、フレームをドロップ。ビデオが先行している場合、レンダラは現フレームを保持。

### 20.4.4 シーク

```
ユーザーが時間Tにシーク
  │
  ├─ 1. デコーダパイプラインをフラッシュ（保留中のフレームを破棄）
  ├─ 2. デマクサがT以前の最寄りのキーフレームにシーク
  ├─ 3. キーフレームからデコード再開
  ├─ 4. デコード（T以前のフレームを破棄）
  ├─ 5. Tまたは直後の最初のフレーム → 表示
  └─ 6. 通常再生を再開
```

フラグメンテッドMP4（DASH/HLS）では、シークにネットワーク経由での別セグメントのフェッチが必要な場合あり。

## 20.5 プロセスアーキテクチャ

### 20.5.1 Decoderプロセス

メディアデコードはサンドボックス化されたDecoderプロセスで実行（第5章のプロセスモデルを拡張）：

```rust
pub enum ProcessType {
    Browser,
    Renderer,
    Network,
    Gpu,
    Decoder,   // ← 新規
}
```

プロセス分離の根拠：
- コーデックライブラリ（特にCベース：libvpx、dav1d、プラットフォームAPI）はメモリ安全性バグのリスクが高い。
- デコーダクラッシュはメディア再生のみに影響し、ページやブラウザには波及しない。
- サンドボックスがデコーダプロセスの能力を制限（ネットワーク、ファイルシステム、GPUなし — HWデコードを除く）。

**IPCプロトコル：**

```rust
// Renderer → Decoder
pub enum DecoderRequest {
    Configure(DecoderConfig),
    Decode(EncodedPacket),         // 共有メモリバッファ
    Flush,
    Shutdown,
}

// Decoder → Renderer
pub enum DecoderResponse {
    Configured(DecoderCapabilities),
    Frame(DecodedFrame),           // 共有メモリバッファ（ビデオ）またはインライン（オーディオ）
    Flushed,
    Error(DecoderError),
}
```

エンコード済みパケットとデコード済みビデオフレームは共有メモリ経由で転送し、プロセス境界を越える大容量バッファのコピーを回避。

### 20.5.2 ハードウェアアクセラレーションデコード

ハードウェアデコード（VA-API、VideoToolbox、DXVA）の場合：

```
Rendererプロセス             Decoderプロセス             GPUプロセス
    │                             │                            │
    ├── Decode(packet) ──────────►│                            │
    │                             ├── HWデコード要求 ──────────►│
    │                             │                            ├── GPUデコード
    │                             │                            │  （専用HWユニット）
    │                             │◄── テクスチャハンドル ──────┤
    │◄── Frame(hw_texture) ───────┤                            │
    │                                                          │
    ├── コンポジタがテクスチャを直接インポート ────────────────►│
    │  （ゼロコピー：デコード済みテクスチャ → コンポジタレイヤー）│
```

ハードウェアデコード済みフレームはGPUテクスチャハンドルとして到着。コンポジタ（第15章）がこれらのテクスチャを合成シーンに直接インポート — CPUリードバックなし。ビデオ再生の電力効率最適パス。

### 20.5.3 elidex-app SingleProcess

elidex-app SingleProcessモードでは、デコーダが専用スレッドプール上でインプロセスで実行。IPCオーバーヘッドなしだがプロセス分離もなし。信頼されたアプリコンテンツには許容可能。

## 20.6 HTMLMediaElement

### 20.6.1 ECS表現

`<video>`と`<audio>`要素はメディア固有コンポーネントを持つECSエンティティ：

```rust
pub struct MediaElement {
    pub player: MediaPlayer,
    pub network_state: NetworkState,
    pub ready_state: ReadyState,
    pub current_src: String,
}

pub struct VideoSurface {
    /// ビデオフレーム表示用のコンポジタ（第15章）内レイヤー。
    pub layer_id: LayerId,
    /// 固有ビデオ寸法（置換要素としてCSSレイアウトに影響）。
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}
```

HTMLMediaElement API（`play()`、`pause()`、`currentTime`、`volume`、イベント）はScriptSession（第13章）経由で公開され、MediaPlayer操作にマッピング。

### 20.6.2 ビデオ表示

ビデオフレームは専用レイヤー（第15章§15.4）として合成：

```
HTML LayoutSystem
  │  <video>にCSSボックスを割り当て（例：位置(100, 200)で640×360）
  ▼
VideoRenderer
  │  デコード済みキューから現在のフレームを選択
  │  フレームテクスチャをアップロード（またはHWテクスチャを直接使用）
  ▼
Layer Tree（第15章§15.4）
  │  ビデオ要素位置にビデオレイヤー
  │  object-fit CSSプロパティがボックス内のスケーリングを制御
  ▼
コンポジタ
  │  ビデオレイヤーを他のコンテンツと合成
```

`object-fit`（contain、cover、fill、none、scale-down）はビデオテクスチャをCSSボックスにマッピングする際にコンポジタが適用。

### 20.6.3 ポスターとコントロール

- **ポスター**：`poster`属性が画像（第18章パイプライン）を読み込み、再生開始前に表示。
- **コントロール**：デフォルトブラウザコントロールはBrowserShell（第24章）がオーバーレイとしてレンダリング。elidex-appではコントロールはアプリの責任。

## 20.7 MediaSource Extensions (MSE)

MSEがJavaScript駆動のアダプティブストリーミング（DASH、JavaScript経由のHLS）を可能に：

```rust
pub struct MediaSourceHandle {
    /// アクティブなSourceBuffer群
    source_buffers: Vec<SourceBuffer>,
    /// JavaScriptが設定したduration
    duration: f64,
    /// 準備状態
    ready_state: MseReadyState,
}

pub struct SourceBuffer {
    pub id: SourceBufferId,
    pub mime_type: String,
    pub codec: Codec,
    /// バッファ済み時間範囲
    pub buffered: TimeRanges,
    /// 追記バッファ：JSがエンコード済みセグメントを提供
    pub pending_append: VecDeque<Bytes>,
}
```

MSEフロー：
```
JavaScript（アダプティブビットレートロジック）
  │
  ├── new MediaSource()
  ├── video.src = URL.createObjectURL(mediaSource)
  ├── sourceBuffer = mediaSource.addSourceBuffer('video/mp4; codecs="avc1.42E01E"')
  │
  │   [CDNからセグメントをfetch]
  ├── sourceBuffer.appendBuffer(segment)
  │     → デマクサがセグメントを解析
  │     → パケットがデコーダにキューイング
  │
  │   [アダプティブ：品質切替]
  ├── sourceBuffer.appendBuffer(higher_quality_segment)
  │
  └── mediaSource.endOfStream()
```

SourceBufferデータは標準メディアと同じデマクサ → デコーダ → レンダラパイプラインに流れる。唯一の違いはデータソース：連続的なネットワークストリームの代わりにJS提供のセグメント。

## 20.8 Encrypted Media Extensions (EME)

### 20.8.1 アーキテクチャ

EMEがDRM保護コンテンツの標準化APIを提供。ElidexはEMEインターフェースとCDMプラグインスロットを定義するが、初期はCDM実装を同梱しない：

```rust
pub trait ContentDecryptionModule: Send {
    /// ライセンス交換用セッションを作成。
    fn create_session(&mut self, session_type: SessionType) -> Result<SessionId, CdmError>;

    /// ライセンスリクエストを生成。
    fn generate_request(
        &mut self,
        session: SessionId,
        init_data_type: &str,
        init_data: &[u8],
    ) -> Result<Bytes, CdmError>;

    /// ライセンスサーバーからのライセンスレスポンスを提供。
    fn update_session(
        &mut self,
        session: SessionId,
        response: &[u8],
    ) -> Result<(), CdmError>;

    /// 暗号化メディアサンプルを復号。
    fn decrypt(
        &mut self,
        encrypted: &[u8],
        iv: &[u8],
        key_id: &[u8],
        subsample_info: &[SubsampleEntry],
    ) -> Result<Bytes, CdmError>;

    /// セッションを閉じ解放。
    fn close_session(&mut self, session: SessionId) -> Result<(), CdmError>;
}

pub enum SessionType {
    Temporary,
    PersistentLicense,
}
```

### 20.8.2 CDM統合フロー

```
JavaScript                    Rendererプロセス           CDMプロセス（将来）
    │                             │                        │
    ├── navigator.requestMediaKeySystemAccess()           │
    │   ("com.widevine.alpha", configs)                   │
    │                             │                        │
    │   [elidexがCDMプラグインの可用性を確認]              │
    │                             │                        │
    ├── mediaKeys.createSession() │                        │
    ├── session.generateRequest() ─►│                      │
    │                             ├─── IPC ───────────────►│
    │                             │                        ├── generate_request()
    │◄── "message"イベント ────────┤◄── ライセンスリクエスト──┤
    │                             │                        │
    │   [アプリがリクエストをライセンスサーバーに送信]       │
    │   [アプリがライセンスレスポンスを受信]                │
    │                             │                        │
    ├── session.update(response) ─►│                       │
    │                             ├─── IPC ───────────────►│
    │                             │                        ├── update_session()
    │                             │                        ├── （鍵が利用可能に）
    │                             │                        │
    │   [暗号化パケットが到着]     │                        │
    │                             ├── decrypt(packet) ────►│
    │                             │◄── 復号済みパケット ────┤
    │                             ├── 通常通りデコード      │
```

### 20.8.3 初期ステータス

Elidex v1はCDMなしで出荷。EME JavaScript APIはすべての鍵システムに`NotSupportedError`を返す。CDMトレイトとCDMプロセスアーキテクチャが定義されており、アーキテクチャ変更なしで将来CDMを統合可能。Netflix、Disney+、Spotify等のDRM依存サービスに影響。

Clear Key（テスト用の自明な非プロプライエタリCDM）をリファレンスCDMとして実装する可能性あり。

## 20.9 Web Audio API

### 20.9.1 概要

Web Audio APIがゲーム、音楽アプリケーション、リアルタイムオーディオエフェクト向けのグラフベースオーディオ処理パイプラインを提供。

```
AudioContext
  │
  ├── ソースノード
  │   ├── AudioBufferSourceNode（デコード済みオーディオバッファ）
  │   ├── MediaElementAudioSourceNode（<audio>/<video>）
  │   ├── MediaStreamAudioSourceNode（getUserMedia）
  │   └── OscillatorNode（生成波形）
  │
  ├── 処理ノード
  │   ├── GainNode
  │   ├── BiquadFilterNode
  │   ├── ConvolverNode（インパルスレスポンスによるリバーブ）
  │   ├── DelayNode
  │   ├── DynamicsCompressorNode
  │   ├── WaveShaperNode（ディストーション）
  │   ├── StereoPannerNode
  │   ├── AnalyserNode（ビジュアライゼーション用FFT）
  │   ├── ChannelSplitterNode / ChannelMergerNode
  │   └── AudioWorkletNode（Wasm/JSによるカスタム処理）
  │
  └── デスティネーション
      └── AudioDestinationNode → プラットフォームオーディオ出力
```

### 20.9.2 オーディオスレッド

Web Audioは専用リアルタイムオーディオスレッドを必要とする（第6章のスレッドモデルを拡張）：

```rust
pub struct AudioThread {
    /// このスレッドで評価されるオーディオグラフ。
    graph: AudioGraph,
    /// オーディオデバイスコールバック周期（通常128–1024サンプル）。
    buffer_size: usize,
    /// サンプルレート（通常44100または48000 Hz）。
    sample_rate: u32,
}
```

オーディオスレッドにはリアルタイム制約：
- コールバックデッドライン内にオーディオバッファを充填する必要（例：44.1kHzで128サンプルなら~2.9ms）。
- ホットパスでアロケーション、ロック、I/O禁止。
- メインスレッドとの通信にロックフリーリングバッファ。

```rust
/// ロックフリーコマンドキュー：メインスレッド → オーディオスレッド
pub struct AudioCommandQueue {
    queue: crossbeam::queue::SegQueue<AudioCommand>,
}

pub enum AudioCommand {
    Connect { source: NodeId, destination: NodeId, output: u32, input: u32 },
    Disconnect { source: NodeId, destination: NodeId },
    SetParam { node: NodeId, param: ParamId, value: f32, time: f64 },
    ScheduleParam { node: NodeId, param: ParamId, automation: ParamAutomation },
    StartNode { node: NodeId, when: f64 },
    StopNode { node: NodeId, when: f64 },
}
```

### 20.9.3 AudioWorklet

AudioWorkletがオーディオスレッドで実行されるユーザー定義オーディオ処理を許可：

```rust
pub struct AudioWorkletProcessor {
    /// オーディオスレッドコンテキストに読み込まれたWasmまたはJSモジュール。
    /// 128サンプルのレンダクォンタムを処理。
    module: WorkletModule,
}
```

AudioWorkletは別のWorkerスレッドではなくオーディオスレッドで実行。同じリアルタイム制約を満たす必要あり。AudioWorkletGlobalScopeは制限された環境：DOM、ネットワーク不可、限定的なAPIサーフェス。

メインスレッドとAudioWorklet間の通信はMessagePort（内部はロックフリーリングバッファ）経由。

### 20.9.4 オーディオグラフ評価

```rust
impl AudioGraph {
    /// 1レンダクォンタム（128サンプル）分のグラフを評価。
    /// オーディオスレッドのオーディオデバイスコールバックから呼び出される。
    pub fn render(&mut self, output: &mut [f32]) {
        // 1. メインスレッドからのコマンドを処理（ロックフリーキュー）
        self.process_commands();

        // 2. ノードのトポロジカルソート（キャッシュ、connect/disconnectで無効化）
        // 3. 順序に従って各ノードを評価
        for node_id in &self.evaluation_order {
            let node = &mut self.nodes[*node_id];
            // 入力バッファをミックス
            let input = self.collect_inputs(*node_id);
            // 処理
            node.process(&input, &mut self.scratch_buffer);
            // 下流ノード用に出力を格納
            self.outputs[*node_id] = self.scratch_buffer.clone();
        }

        // 4. デスティネーションノード出力をデバイスバッファにコピー
        output.copy_from_slice(&self.outputs[self.destination]);
    }
}
```

### 20.9.5 AudioParamオートメーション

Web Audio AudioParamがスケジュールされたオートメーションをサポート：

```rust
pub enum ParamAutomation {
    SetValueAtTime { value: f32, time: f64 },
    LinearRampToValueAtTime { value: f32, end_time: f64 },
    ExponentialRampToValueAtTime { value: f32, end_time: f64 },
    SetTargetAtTime { target: f32, start_time: f64, time_constant: f64 },
    SetValueCurveAtTime { values: Vec<f32>, start_time: f64, duration: f64 },
}
```

オートメーションはサンプル精度タイミングのためにオーディオスレッド上でサンプル単位で評価。

## 20.10 メディアキャプチャ

### 20.10.1 getUserMedia / getDisplayMedia

メディアキャプチャAPIがカメラ、マイクロフォン、画面コンテンツへのアクセスを提供：

```rust
pub struct MediaStream {
    pub id: String,
    pub tracks: Vec<MediaStreamTrack>,
}

pub struct MediaStreamTrack {
    pub id: String,
    pub kind: TrackKind,
    pub label: String,
    pub constraints: MediaTrackConstraints,
    pub state: TrackState,
}

pub enum TrackState {
    Live,
    Ended,
}
```

パーミッションフローは第8章と統合：
- `getUserMedia({ video: true })` → カメラパーミッションプロンプト
- `getUserMedia({ audio: true })` → マイクロフォンパーミッションプロンプト
- `getDisplayMedia()` → ScreenCaptureパーミッション + プラットフォーム画面ピッカー

### 20.10.2 MediaStream統合

MediaStreamトラックのルーティング先：

| 宛先 | メカニズム |
| --- | --- |
| `<video>`要素 | `video.srcObject = stream` |
| Web Audio | `audioContext.createMediaStreamSource(stream)` |
| MediaRecorder | Blobに記録（BlobStore経由、第21章） |
| WebRTC（将来） | `peerConnection.addTrack(track, stream)` |
| Canvas | `canvas.captureStream()` / `ctx.drawImage(video, ...)` |

### 20.10.3 カメラ/マイクロフォンアクセス

カメラとマイクロフォンのアクセスはBrowserプロセス（特権付き、非サンドボックス）が処理しRendererにストリーミング：

```
Browserプロセス                          Rendererプロセス
    │                                        │
    ├── カメラデバイスオープン                  │
    ├── フレームキャプチャ → 共有メモリ ───────►│
    │                                        ├── MediaStreamトラック
    │                                        ├── （<video>またはWeb Audioに供給）
    │                                        │
    ├── マイクロフォンオープン                  │
    ├── オーディオキャプチャ → 共有メモリ ─────►│
    │                                        ├── MediaStreamトラック
```

## 20.11 WebRTC（インターフェース定義）

WebRTCは大規模な独立サブシステム。本セクションは統合インターフェースのみを定義し、完全設計は将来の章に延期。

### 20.11.1 統合ポイント

```rust
/// WebRTCのメディアパイプラインとの統合サーフェス。
pub trait RtcMediaInterface {
    /// ローカルMediaStreamTrackをピアコネクションに追加。
    fn add_track(&mut self, track: &MediaStreamTrack, stream: &MediaStream);

    /// ピアコネクションからリモートトラックを受信。
    fn on_track(&self) -> Receiver<(MediaStreamTrack, MediaStream)>;

    /// モニタリング用統計を取得。
    fn get_stats(&self) -> RtcStats;
}
```

### 20.11.2 スコープ境界

| 本章スコープ内 | 延期（将来のWebRTC章） |
| --- | --- |
| MediaStream / MediaStreamTrackモデル | ICE / STUN / TURN |
| getUserMedia / getDisplayMedia | SDPネゴシエーション |
| MediaStream → `<video>`、Web Audio、canvas | SRTP / DTLS暗号化 |
| MediaRecorder | SCTPデータチャネル |
| — | RTCPeerConnection完全ライフサイクル |
| — | コーデックネゴシエーション（SDPコーデックパラメータ） |
| — | 帯域幅推定 / 輻輳制御 |

## 20.12 プラットフォームオーディオ出力

### 20.12.1 オーディオ出力抽象化

```rust
pub trait AudioOutput: Send {
    /// 希望設定でオーディオデバイスをオープン。
    fn open(&mut self, config: AudioOutputConfig) -> Result<(), AudioError>;

    /// 再生を開始。コールバックがバッファを充填するために定期的に呼び出される。
    fn start(&mut self, callback: AudioCallback) -> Result<(), AudioError>;

    /// 再生を停止。
    fn stop(&mut self) -> Result<(), AudioError>;

    /// デバイス能力を問い合わせ。
    fn capabilities(&self) -> AudioDeviceCapabilities;
}

pub struct AudioOutputConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub buffer_size: u32,  // コールバックあたりのサンプル数
}

pub type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send>;
```

プラットフォーム実装：

| プラットフォーム | API | 備考 |
| --- | --- | --- |
| macOS / iOS | CoreAudio (AudioUnit) | 低レイテンシ、ハードウェアミキシング |
| Windows | WASAPI | 排他モードと共有モード |
| Linux | PipeWire / PulseAudio | PipeWire推奨（低レイテンシ） |
| Android | AAudio / Oboe | OboeがクロスAPI抽象化を提供 |

### 20.12.2 オーディオルーティング

HTMLMediaElementとWeb Audioの両方が同じAudioOutputに供給：

```
HTMLMediaElement AudioRenderer ──►┐
                                  ├── AudioMixer ──► AudioOutput ──► スピーカー
Web Audio AudioDestinationNode ──►┘
```

AudioMixerが複数のオーディオソースを合算。タブ単位のミュートと音量制御がミキシング前に適用。

## 20.13 elidex-app メディア

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| HTMLMediaElement | フルサポート | フルサポート |
| MSE | フルサポート | フルサポート |
| EME/DRM | CDMプラグインスロット（v1：空） | CDMプラグインスロット（v1：空） |
| Web Audio | フルサポート | フルサポート |
| getUserMedia | パーミッションプロンプト | `App::grant(Camera)`、`App::grant(Microphone)` |
| getDisplayMedia | パーミッションプロンプト | `App::grant(ScreenCapture)` |
| WebRTC | 将来 | 将来 |
| コーデック戦略 | Mixed（デフォルト） | アプリごとに設定可能 |
| Decoderプロセス | 別プロセス | インプロセス（SingleProcessモード） |
| オーディオ出力 | 他タブと共有 | アプリ排他オーディオセッション |

elidex-appはビルド時にコーデック可用性を設定可能。メディアを使用しないアプリはバイナリサイズ削減のため全メディアパイプラインを除外可能。特定コーデックが必要なアプリ（例：専門フォーマットを使う医療画像）はカスタムMediaDecoder実装を登録可能。
