
# 5. プロセスアーキテクチャ & 非同期ランタイム

本章では2つの基盤的設計課題を解決する：マルチプロセスモデル（OPEN-001）と非同期I/Oランタイム（OPEN-011）。両者は深く絡み合っている——プロセスモデルがランタイムインスタンスの数と各々の役割を決定し、非同期ランタイムが各プロセス内でのI/O、IPC、イベントループの動作を決定する。

## 5.1 設計思想：段階的緩和

elidexのエンジン層で使われるcore/compatパターンがプロセスモデルにもそのまま適用される：

| 側面 | コア（長期目標） | 互換（Phase 1–3の現実） | 互換の理由 |
| --- | --- | --- | --- |
| Renderer分離 | クラッシュ隔離のみ。単一または少数のRendererプロセス。 | サイト単位プロセス分離。 | SpiderMonkeyはC++。JSエンジンのメモリ破壊がクロスサイトデータを漏洩しうる。 |
| Network Process | Browserプロセス内スレッドに統合可能。 | 別プロセス。 | RendererにC++コードが存在する間の多層防御。 |
| GPU Process | Rendererスレッドに統合可能。 | 別プロセスまたはスレッド。 | GPUドライバクラッシュはOSレベル。分離の価値は残るが形態は柔軟。 |
| 非同期ランタイム | 完全自前イベントループ（Rendererで実証済み、拡張）。 | Network/Browserにtokio、Rendererは自前。 | tokioがエコシステム速度を提供。Rendererは初日からカスタム制御が必要。 |

主要な設計制約：**Phase 1–3のアーキテクチャが長期的な簡素化を妨げてはならない**。プロセス境界とランタイム選択はトレイト抽象の背後にあり、緩和が設定変更で完了し、書き直しにならないようにする。

## 5.2 プロセスモデル

### 5.2.1 プロセス役割

```
┌────────────────────────────────────────────────────────┐
│  Browser Process（特権、シングルトン）                   │
│                                                        │
│  Shell State (Ch. 24)     Process Lifecycle Manager      │
│  Persistence (OPEN-012) Permission Broker (Ch. 8)        │
│  Extension Host         Download Manager               │
└────────┬──────────────────┬──────────────┬─────────────┘
         │ IPC              │ IPC          │ IPC
┌────────▼────────┐ ┌──────▼───────┐ ┌────▼──────────────┐
│ Renderer Process │ │ Network      │ │ GPU Process       │
│ (サイト単位†)     │ │ Process      │ │ (シングルトン)     │
│                  │ │ (シングルトン)│ │                   │
│ DOM/ECS          │ │ HTTPスタック  │ │ wgpuサーフェス     │
│ ScriptSession    │ │ DNSリゾルバ   │ │ レイヤー合成       │
│ Style & Layout   │ │ 接続プール    │ │ ラスタライゼーション│
│ Paint            │ │ Cookie jar   │ │ スクロール & anim  │
│ スクリプトエンジン │ │ TLS/証明書   │ │                   │
│ Wasmランタイム    │ │ WebSocket    │ │                   │
│ イベントループ    │ │              │ │                   │
└──────────────────┘ └──────────────┘ └───────────────────┘
                                       ┌───────────────────┐
                                       │ Utility Process    │
                                       │ (オンデマンド)      │
                                       │                    │
                                       │ メディアデコード    │
                                       │ 音声処理           │
                                       └───────────────────┘
```

† Phase 1–3ではサイト単位分離、長期的には緩和可能。

| プロセス | 数 | サンドボックス | 責務 | 長期展望 |
| --- | --- | --- | --- | --- |
| Browser | 1 | なし（特権） | シェル状態（第24章）、永続化（OPEN-012）、パーミッション仲介（第8章）、ダウンロード管理、拡張機能ライフサイクル、プロセスライフサイクル管理。フルOSアクセスを持つ唯一のプロセス。 | 常に分離 — 特権ブローカー役割は恒久的。 |
| Renderer | サイトごとに1（Phase 1–3） | 厳格（seccomp-bpf / App Sandbox / Restricted token） | DOM/ECS、ScriptSession（第13章）、スタイル/レイアウト/ペイント、スクリプトエンジン、Wasmランタイム。直接のネットワーク/ファイルシステムアクセスなし。 | C++（SpiderMonkey）除去後にプロセス数を緩和可能。クラッシュ隔離の価値は残る。 |
| Network | 1 | 中程度（ネットワークのみ） | HTTP/HTTPSスタック（hyper + rustls + h3）、DNS解決、接続プール、cookie jar、TLS、WebSocket。 | Browserプロセス内スレッドに統合可能。タブ間の接続プール共有は集中化の実用的理由。 |
| GPU | 1 | 中程度 | wgpuサーフェス管理、Velloラスタライゼーション、レイヤー合成、コンポジター駆動スクロール/アニメーション。 | Renderer内スレッドに統合可能。GPUドライバクラッシュ隔離の価値はあるがRenderer分離ほど重要ではない。 |
| Utility | 0–N | 厳格 | メディアデコード（OPEN-002）、音声処理。オンデマンドで起動、アイドル時に終了。 | C/C++ライブラリ分離のために維持。純粋Rustデコーダには不要。 |

### 5.2.2 サイト分離（Phase 1–3）

Rendererプロセス分離はサイト単位の粒度で、サイトはscheme + eTLD+1（例：`https://example.com`）として定義される：

```
Tab: https://news.example.com/article
  ├── メインフレーム: news.example.com       → Renderer A
  ├── <iframe src="ads.tracker.com/..."> → Renderer B（クロスサイト）
  └── <iframe src="cdn.example.com/..."> → Renderer A（同一サイト）
```

Spectre/Meltdown緩和を提供：クロスサイトiframeが別OSプロセス・別アドレス空間に配置される。COOP/COEP施行（第8章）と組み合わせ、オリジン間の投機的実行サイドチャネル攻撃を防止。

### 5.2.3 分離粒度設定

プロセスモデルはビルド時と起動時に設定可能で、段階的緩和を実現する：

```rust
pub enum ProcessModel {
    /// Phase 1–3のelidex-browserデフォルト。
    /// サイトごとに独自のRendererプロセス。
    SiteIsolation,

    /// 全Rust移行後のelidex-browserの長期オプション。
    /// タブごとに1 Renderer（サイト分離なしのクラッシュ隔離）。
    PerTab,

    /// 最小分離。複数タブがRendererを共有。
    /// 1タブのクラッシュが同一プロセスの他タブに影響しうる。
    Shared { max_renderers: usize },

    /// elidex-appデフォルト。すべて1プロセス。
    /// 最大パフォーマンス、最小オーバーヘッド。
    SingleProcess,
}
```

`SiteIsolation`から`PerTab`や`Shared`への移行は非破壊的。IPC抽象化（5.3節）が「相手先プロセス」が実OSプロセスでも同一プロセス内の論理境界でも同一に機能するため。

### 5.2.4 elidex-appプロセスモデル

elidex-appのデフォルトは`SingleProcess` — エンジン全体がアプリケーションのプロセス内で動作：

```rust
let app = elidex_app::App::new()
    // デフォルト: SingleProcess。IPCオーバーヘッドなし、最小起動時間。
    // スクリプトエンジンはelidex-js（Rust）なのでC++メモリ安全性の懸念なし。
    .build();
```

信頼できないWebコンテンツを埋め込むアプリ（例：任意のHTMLをレンダリングするRSSリーダー）は分離をオプトインできる：

```rust
let app = elidex_app::App::new()
    .process_model(ProcessModel::PerTab)  // 各WebViewを分離
    .build();
```

### 5.2.5 プロセスライフサイクル

| イベント | 動作 |
| --- | --- |
| タブ開設 | Process Lifecycle Managerがターゲットサイト用のRendererを提供。同サイトの既存Rendererがあれば再利用、なければ新プロセスを起動。 |
| タブ閉鎖 | 他のタブがそのRendererを参照していなければ、タイムアウト付きでグレースフルシャットダウン。 |
| Rendererクラッシュ | Browser ProcessがIPCチャネルEOFで検出。タブにクラッシュページを表示、リロードオプション付き。他タブは影響なし。クラッシュダンプ取得（minidump形式）。 |
| OOMプレッシャー | OSメモリプレッシャー通知 → Browser Processがバックグラウンドの Rendererを LRU で破棄選択。破棄タブはプレースホルダーを表示、フォーカス時のリロードで状態復元。 |
| Network Processクラッシュ | 自動再起動。飛行中のリクエストは失敗、Rendererはエラーレスポンスを受信しリトライ可能。 |
| GPU Processクラッシュ | 自動再起動。一瞬の視覚的グリッチ。Rendererがディスプレイリストを再送信。コンポジター状態を再構築。 |

## 5.3 IPCアーキテクチャ

### 5.3.1 IPCトレイト抽象化

重要な設計：IPCはトレイトの背後に抽象化され、ターゲットが別OSプロセスでもインプロセスでも同じコードが動く。これが段階的緩和を可能にする：

```rust
/// コアIPC抽象。クロスプロセスとインプロセスの両方に実装。
pub trait IpcChannel<Req, Resp>: Send + Sync {
    async fn send(&self, message: Req) -> Result<()>;
    async fn recv(&self) -> Result<Resp>;
    async fn call(&self, request: Req) -> Result<Resp>;  // send + await response
}

/// クロスプロセス実装：OSパイプ上のシリアライゼーション。
pub struct ProcessChannel<Req, Resp> { /* ipc-channel内部 */ }

/// インプロセス実装：直接asyncチャネル（ゼロコピー）。
pub struct LocalChannel<Req, Resp> { /* tokio::sync::mpscまたは同等 */ }
```

`ProcessModel::SingleProcess`選択時は`ProcessChannel`の代わりに`LocalChannel`が使われる。シリアライゼーションなし、OSパイプオーバーヘッドなし、データコピーなし。エンジンコードは同一 — `dyn IpcChannel<Req, Resp>`に対してプログラムする。

### 5.3.2 IPCトランスポートメカニズム

| メカニズム | ユースケース | 実装 |
| --- | --- | --- |
| 型付きチャネル | コマンド、イベント、小ペイロード（<64KB）。 | ipc-channel（Servo由来）をOSパイプ上で使用。メッセージはRust enum、postcard（コンパクトバイナリ、no-std互換）でシリアライズ。 |
| 共有メモリ | 大データ：ディスプレイリスト（Renderer→GPU）、デコード済みビットマップ（Utility→Renderer）。 | mmapバックの共有メモリ領域。ハンドルを含むチャネルメッセージで所有権を転送。 |
| インプロセスバイパス | SingleProcessモード。すべての通信。 | tokio::sync::mpscまたはcrossbeamチャネル。ゼロコピー。 |

### 5.3.3 メッセージ型

各プロセスペアが型付きRust enumで通信。コンパイル時の網羅性チェックがプロトコル不一致を防止：

```rust
/// Browser → Renderer
pub enum BrowserToRenderer {
    NavigateTo(Url),
    ExecuteScript(String),
    SetViewportSize(PhysicalSize),
    GrantPermission(PermissionType),
    Suspend,   // バックグラウンドタブ
    Resume,    // フォアグラウンドタブ
}

/// Renderer → Browser
pub enum RendererToBrowser {
    NavigationRequest(Url),           // ユーザーがリンクをクリック
    PermissionRequest(PermissionType), // スクリプトがカメラ等を要求
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
    FetchBodyChunk(FetchId, Vec<u8>),  // ストリーミング
    FetchComplete(FetchId),
    FetchError(FetchId, FetchError),
    WebSocketMessage(WsId, Vec<u8>),
    WebSocketClosed(WsId, CloseReason),
}

/// Renderer → GPU
pub enum RendererToGpu {
    SubmitDisplayList(SurfaceId, DisplayList),  // 共有メモリ経由
    UpdateScrollOffset(SurfaceId, Vec2),
    Resize(SurfaceId, PhysicalSize),
}

/// GPU → Renderer
pub enum GpuToRenderer {
    FramePresented(SurfaceId, FrameTimestamp),
    SurfaceLost(SurfaceId),
}
```

## 5.4 非同期ランタイムアーキテクチャ

### 5.4.1 プロセスごとのランタイム戦略

各プロセス型がワークロードに最適な非同期ランタイムを使用する：

| プロセス | ランタイム | 根拠 |
| --- | --- | --- |
| Browser | tokioマルチスレッド | 汎用I/O多重化。UIイベント、IPCディスパッチ、永続化I/O。標準的なasyncサーバーワークロード。 |
| Network | tokioマルチスレッド | I/O集約的。数千の同時接続、HTTP/2多重化、DNSクエリ。tokioはまさにこの用途向け。 |
| Renderer | elidexイベントループ（カスタム）+ tokio current_threadをI/Oバックエンドとして | フレームタイミング、rAFスケジューリング、フラッシュポイントの制御が必要。JSイベントループ（第13章）が主要シーケンサー。tokioのreactorが制御を奪わずI/O readiness通知を提供。 |
| GPU | 軽量イベントループ | vsync駆動。ディスプレイリスト受信、合成、表示。最小限の非同期I/O。 |
| Utility | tokio current_thread | 短命で集中的なタスク。作業受信と結果返却のシンプルなasync。 |

### 5.4.2 Rendererイベントループ：統合設計

Rendererのメインスレッドは最も複雑で、JS実行、非同期I/O、IPCメッセージ、フレームレンダリングを厳しいタイミング制約内でインターリーブする必要がある。elidexイベントループがメインスレッドを所有しすべてを駆動する：

```rust
// Rendererメインスレッド — elidexがループを所有
fn renderer_main(ipc: RendererIpc, tokio_rt: tokio::runtime::Runtime) {
    let mut script_engine = ScriptEngine::new();
    let mut session = ScriptSession::new();
    let mut dom = EcsDom::new();
    let mut task_queue = TaskQueue::new();

    loop {
        // ── Phase 1: 外部イベント収集（ノンブロッキング）──────────
        // tokio reactorをポーリングしてI/O完了を確認（fetchレスポンス等）
        // tokioのスケジューラに制御を渡さない。
        tokio_rt.block_on(async {
            tokio::task::yield_now().await;
        });

        // Browser/Network/GPUからのIPCメッセージをドレイン
        while let Some(msg) = ipc.try_recv() {
            task_queue.enqueue_from_ipc(msg);
        }

        // ── Phase 2: JSイベントループ（第13章セマンティクス）──────
        // 最古のマクロタスクを実行
        if let Some(task) = task_queue.pop() {
            script_engine.eval(task, &mut session);
        }

        // マイクロタスクをドレイン
        script_engine.drain_microtasks(&mut session);

        // ScriptSession → ECSフラッシュ
        let records = session.flush(&mut dom);
        deliver_mutation_observers(records, &mut script_engine, &mut session);
        script_engine.drain_microtasks(&mut session);

        // ── Phase 3: レンダリング（vsync機会がある場合）──────────
        if vsync_ready() {
            // requestAnimationFrameコールバック
            for cb in animation_frame_callbacks.drain(..) {
                script_engine.eval(cb, &mut session);
            }
            script_engine.drain_microtasks(&mut session);
            session.flush(&mut dom);

            // Style → Layout → Paint → コンポジタスレッドに送信（第6章）
            run_style_system(&dom);
            run_layout_system(&dom);
            let display_list = run_paint_system(&dom);
            compositor_channel.send(CompositorMsg::SubmitDisplayList(display_list));
        }

        // ── Phase 4: アイドルワーク────────────────────────────
        if has_idle_time() {
            for cb in idle_callbacks.drain(..) {
                script_engine.eval(cb, &mut session);
            }
        }

        // ── Phase 5: 次イベントまたはvsyncを待機─────────────────
        // 待機条件：IPCメッセージ到着、I/O完了、タイマー発火、
        // またはvsyncシグナル。ここでmio/tokioのepoll/kqueueが待機。
        wait_for_event_or_vsync();
    }
}
```

重要なポイント：elidexのループ構造がHTML Living Standardのイベントループ仕様（第13章）と正確に一致し、I/OとIPCがイベントソースとして統合されている。tokioはループを「実行」しない——`wait_for_event_or_vsync()`が委譲するポーリングインフラを提供するのみ。

### 5.4.3 非同期ランタイムトレイト抽象化

将来のtokio置換オプションを保持するため、プロセスレベルのasync操作はトレイト経由：

```rust
pub trait AsyncRuntime: Send + Sync {
    fn spawn<F>(&self, future: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static;

    fn spawn_blocking<F, R>(&self, func: F) -> TaskHandle
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;

    /// ノンブロッキングポーリング：制御を渡さずに保留中のI/Oを駆動。
    /// Rendererのカスタムイベントループで使用。
    fn poll_reactor(&self);

    /// ブロッキング待機：I/O、タイマー、または外部シグナルまでスリープ。
    /// Rendererのwait_for_event_or_vsync()で使用。
    fn park_until(&self, deadline: Option<Instant>);
}
```

Phase 1–3：`TokioRuntime`がこのトレイトを実装。長期：`ElidexRuntime`が置換可能。Rendererのイベントループ（すでにカスタム）から始めて外側に拡張。

### 5.4.4 タイマー統合

JSタイマー（setTimeout、setInterval、requestAnimationFrame）とRust asyncタイマーがRendererのメインスレッドで共存する必要がある：

| タイマーソース | 統合方法 |
| --- | --- |
| setTimeout / setInterval | TaskQueueにデッドライン付きで登録。イベントループのPhase 1でチェック。非同期ランタイムのタイマーホイールでバッキング。 |
| requestAnimationFrame | GPU Processからのvsyncシグナルに紐付け。フレーム間で蓄積。Phase 3で実行。 |
| requestIdleCallback | フレーム予算に残り時間があればPhase 4で実行。 |
| Rust asyncタイマー（tokio::time::sleep） | I/Oと同じreactorで駆動。Phase 5のスリープからイベントループを起床。 |

すべてのタイマーソースが同じ`wait_for_event_or_vsync()`メカニズムに供給され、最も早いデッドラインでスレッドが起床する。

## 5.5 バックプレッシャーとフロー制御

データがプロセス境界を越えて流れる場合（例：NetworkがRendererのパース速度より速くfetchデータをストリーム）、バックプレッシャーが無限のメモリ成長を防止：

```rust
/// バックプレッシャー付き有界チャネル。
/// バッファが満杯のとき送信者がブロック（async）。
pub struct BackpressureChannel<T> {
    capacity: usize,  // 最大バッファアイテム数
    // ...
}
```

| データフロー | バックプレッシャーメカニズム |
| --- | --- |
| Network → Renderer（fetchボディ） | FetchIdごとの有界チャネル。チャネルが満杯のときNetwork ProcessがTCPソケットからの読み取りを一時停止。Rendererが消費すると自動再開。 |
| Renderer → GPU（ディスプレイリスト） | ダブルバッファリング。Rendererがフレーム N+1を生成中にGPUがフレーム Nを表示。GPUが遅れた場合、Rendererはsubmit時にブロック（フレームペーシング）。 |
| Utility → Renderer（デコード済みメディア） | デコード済みフレームの有界キュー。キューが満杯のときデコーダが一時停止。 |

## 5.6 段階的移行パス

```
Phase 1–3（SpiderMonkey時代）
├── ProcessModel::SiteIsolation（ブラウザのデフォルト）
├── Browser/Network: tokioマルチスレッド
├── Renderer: elidexイベントループ + tokio reactor
├── IPC: ProcessChannel（クロスプロセス、シリアライズ）
└── セキュリティ: プロセス分離がC++リスクを補償

Phase 4–5（elidex-js移行）
├── ProcessModel::PerTabまたはShared（緩和、設定可能）
├── Network: Browserスレッドへの統合を検討
├── GPU: Rendererスレッドへの統合を検討
├── Rendererイベントループが成熟、最適化を蓄積
└── セキュリティ: Rustメモリ安全性によりサイト分離はオプショナル

長期（全Rust）
├── ProcessModel::SharedまたはSingleProcess（信頼コンテンツ向け）
├── elidexイベントループが全プロセスでtokioを置換（オプショナル）
├── IPC: プロセス統合箇所でLocalChannel（ゼロコピー）
└── セキュリティ: クラッシュ隔離のみ。Spectre緩和はOS/ハードウェアに委譲
```

各ステップが構造的書き直しではなく設定変更（またはCargo featureフラグ）で完了する。`dyn IpcChannel`と`dyn AsyncRuntime`を使うコードはすべてのフェーズで同一に動作する。
