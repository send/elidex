
# 6. プロセス内スレッドモデル

第5章ではプロセス間アーキテクチャ — どのプロセスが存在し、どう通信し、プロセスモデルが時間とともにどう緩和されるかを定義した。本章ではプロセス内スレッドトポロジーを定義する：各プロセス内でどのスレッドが動作し、スレッド間でデータがどう流れ、性能と正確性の両方を維持するためにどの並行プリミティブが選択されるか。

## 6.1 設計原則

**コンポジタの独立性は譲れない。** コンポジタスレッドはメインスレッドでブロックしてはならない。これはJS実行負荷に関係なく60fpsでの滑らかなスクロール、ピンチズーム、CSS transform/opacityアニメーションの基盤。

**共有状態より所有権移転。** Rustの所有権モデルに従い、可能な限りスレッド間でデータをロック越しに共有するのではなく移動する。DisplayListの所有権はメインスレッドからコンポジタに移転。IPCメッセージはクローンではなく移動。

**B→C移行パス。** 初期設計（Phase 1–3）ではメインスレッドとコンポジタ間でメッセージパッシングを使用（アプローチB）。Phase 4+でプロセス分離が緩和された時、コンポジタは直接ECS読み取りに移行可能（アプローチC）。FrameSourceトレイトがこの境界を抽象化し、移行を設定変更にする。

**スレッドプールは共有、重複しない。** 単一のrayonプールがすべてのCPU並列作業（スタイル、レイアウト、画像デコード）を処理。tokioのランタイムがすべての非同期I/Oを処理。コアを奪い合うことなく共存。

## 6.2 Rendererプロセスのスレッドトポロジー

Rendererは最も複雑なプロセス。4種類のスレッドクラスを持つ：

```
Renderer Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  メインスレッド (1)              コンポジタスレッド (1)               │
│  ┌─────────────────────┐     ┌──────────────────────┐              │
│  │ イベントループ(Ch.5) │     │ フレームスケジューリング│              │
│  │ JS/Wasm実行          │     │ レイヤー合成          │              │
│  │ DOM (ECSオーナー)    │────▶│ スクロール/ズーム/anim │              │
│  │ Style→Layout→Paint  │ DP │ GPU送信               │              │
│  │ ScriptSession       │     │ 入力ファストパス      │              │
│  └─────────────────────┘     └──────────────────────┘              │
│           │                                                         │
│           │ ワークスティーリング                                      │
│           ▼                                                         │
│  rayonプール (Nスレッド)       Workerスレッド (0–M)                  │
│  ┌─────────────────────┐     ┌──────────────────────┐              │
│  │ 並列スタイル          │     │ Dedicated Worker(1:1) │              │
│  │ 並列レイアウト        │     │ JS/Wasm実行           │              │
│  │ 画像デコード          │     │ 独自WorkerSession     │              │
│  │ フォントラスタライズ   │     │ postMessage IPC      │              │
│  └─────────────────────┘     └──────────────────────┘              │
│                                                                     │
│  tokio reactor (current_thread、メインスレッド上)                    │
│  ┌─────────────────────┐                                           │
│  │ Fetchレスポンス       │                                           │
│  │ IPCメッセージ受信     │                                           │
│  │ タイマー管理          │                                           │
│  └─────────────────────┘                                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

DP = DisplayPipelineチャネル（アプローチB）またはECS共有読み取り（アプローチC）
```

### 6.2.1 メインスレッド

メインスレッドはECS DOMの唯一のオーナー。すべてのDOM変更、スクリプト実行、スタイル計算の開始、レイアウトがここで発生。構造はイベントループ（第5章、5.4.2節）で定義：

| 責務 | 詳細 |
| --- | --- |
| ECS DOM所有権 | 唯一の書き手。すべてのコンポーネント変更はScriptSession flushを通過。 |
| スクリプト実行 | SpiderMonkey（Phase 1–3）またはelidex-js（Phase 4+）。シングルスレッドJSセマンティクス。 |
| イベントループ | 5フェーズループ：収集→JS→レンダー→アイドル→待機（第5章）。 |
| スタイル開始 | rayonプールで並列スタイル解決を開始し、完了を待つ。 |
| レイアウト開始 | rayonプールで並列レイアウトを開始し、完了を待つ。 |
| ペイント | ECSにLayerコンポーネントを生成。コンポジタ用にDisplayListにシリアライズ（アプローチB）。 |
| tokio reactor | イベントループのPhase 1でポーリング。I/O完了とタイマーのウェイクアップ。 |

**重要な制約：** メインスレッドはrayon並列作業（スタイル、レイアウト）中にブロックする。これは意図的 — パイプラインは逐次的（スクリプト→スタイル→レイアウト→ペイント→合成）であり、rayonのワークスティーリングによりメインスレッドが並列フェーズ中にワーカーとして参加する。

### 6.2.2 コンポジタスレッド

メインスレッドから独立して動作する専用OSスレッド。フレームデータを受信しGPU出力を生成：

```rust
fn compositor_thread(
    frame_source: Box<dyn FrameSource>,
    gpu: GpuContext,
    input_rx: Receiver<InputEvent>,
) {
    loop {
        // 1. メインスレッドからの新フレームデータを確認
        frame_source.poll_update();

        // 2. コンポジタ処理入力イベントを処理
        while let Ok(event) = input_rx.try_recv() {
            match event {
                InputEvent::Scroll(delta) => {
                    frame_source.update_scroll(target, delta);
                }
                InputEvent::PinchZoom(scale) => {
                    frame_source.update_zoom(scale);
                }
                _ => {} // その他のイベントはメインスレッドへ
            }
        }

        // 3. コンポジタ駆動アニメーションを進行
        frame_source.advance_animations(dt);

        // 4. レイヤーを合成しGPUに送信
        let frame = frame_source.composite();
        gpu.submit(frame);

        // 5. vsync待ち
        gpu.wait_vsync();
    }
}
```

| 責務 | 詳細 |
| --- | --- |
| レイヤー合成 | 正しいz順序、transform、opacity、クリッピングでレイヤーを合成。 |
| 独立スクロール | メインスレッドの関与なしにスクロールオフセットを更新。JSハンドラを必要とするスクロールイベント（non-passiveリスナー）はメインスレッドに転送。 |
| CSSアニメーション（サブセット） | transformとopacityアニメーションはコンポジタで実行（レイアウト/再描画不要）。他のアニメーションプロパティはメインスレッドが必要。 |
| ピンチズーム | コンポジタがレイヤーをスケール・移動。ビューポート変更はメインスレッドに非同期通知。 |
| GPU送信 | 合成フレームをGPUプロセスに送信（アプローチB、クロスプロセス）またはwgpuに直接送信（アプローチC、同一プロセス）。 |
| フレームスケジューリング | vsyncに作業を整列。メインスレッドが遅れている場合はグレースフルにフレームドロップ。 |

### 6.2.3 FrameSourceトレイト — B/C抽象化

```rust
pub trait FrameSource: Send {
    /// 新フレームデータを確認。BモードではチャネルからDisplayListを受信。
    /// Cモードではecs frame-readyシグナルを確認。
    fn poll_update(&mut self);

    /// 合成用の現在のレイヤースナップショットを取得。
    fn layers(&self) -> &LayerTree;

    /// スクロールオフセットを更新（コンポジタ駆動、メインスレッドから独立）。
    fn update_scroll(&mut self, target: ScrollTarget, offset: ScrollDelta);

    /// ピンチズームスケールを更新。
    fn update_zoom(&mut self, scale: f32);

    /// コンポジタ駆動アニメーション（transform、opacity）を進行。
    fn advance_animations(&mut self, dt: Duration);

    /// すべてのレイヤーを最終フレームに合成。
    fn composite(&self) -> CompositeFrame;

    /// コンポジタ側のスクロール位置をメインスレッドに報告
    ///（JSスクロールイベントハンドラ、position: sticky等用）。
    fn sync_scroll_to_main(&self);
}
```

**アプローチB実装（Phase 1–3）：**

```rust
pub struct DisplayListFrameSource {
    /// メインスレッドからDisplayListを受信するチャネル
    rx: Receiver<DisplayList>,
    /// 現在のアクティブレイヤーツリー（コンポジタが所有）
    active_tree: LayerTree,
    /// 保留中のレイヤーツリー（受信済みだが未アクティベート）
    pending_tree: Option<LayerTree>,
    /// スクロール状態（コンポジタ所有、メインスレッドに同期返却）
    scroll_state: ScrollState,
    /// アニメーション状態
    animations: AnimationState,
    /// スクロール更新をメインスレッドに返送するチャネル
    scroll_tx: Sender<ScrollUpdate>,
}

impl FrameSource for DisplayListFrameSource {
    fn poll_update(&mut self) {
        if let Ok(display_list) = self.rx.try_recv() {
            // DisplayListから新しいレイヤーツリーを構築
            self.pending_tree = Some(LayerTree::from_display_list(display_list));
        }
        // フレーム境界で保留ツリーをアクティベート
        if let Some(tree) = self.pending_tree.take() {
            self.active_tree = tree;
        }
    }

    fn update_scroll(&mut self, target: ScrollTarget, delta: ScrollDelta) {
        // コンポジタ自身のコピー上でスクロール更新 — ロックなし、IPCなし
        self.scroll_state.apply(target, delta);
    }
    // ...
}
```

**アプローチC実装（Phase 4+）：**

```rust
pub struct EcsFrameSource {
    /// ECSワールドへの共有参照（同一プロセス）
    ecs: Arc<EcsWorld>,
    /// メインスレッドからのフレーム準備完了シグナル
    frame_signal: AtomicBool,
    /// コンポジタ可変状態（ロックフリー）
    compositor_state: Arc<CompositorMutableState>,
}

pub struct CompositorMutableState {
    /// スクロールコンテナごとのオフセット。コンポジタが更新、メインスレッドが読み取り。
    scroll_offsets: DashMap<EntityId, AtomicScrollOffset>,
    /// コンポジタ駆動アニメーションの進行。
    animation_ticks: DashMap<AnimationId, AtomicF64>,
}

impl FrameSource for EcsFrameSource {
    fn poll_update(&mut self) {
        // データ転送不要 — メインスレッドが新フレームをシグナルしたか確認するだけ
        if self.frame_signal.swap(false, Ordering::Acquire) {
            // メインスレッドがスタイル/レイアウト/ペイントを完了。
            // ECS内のLayerコンポーネントは最新。
        }
    }

    fn update_scroll(&mut self, target: ScrollTarget, delta: ScrollDelta) {
        // アトミック更新 — メインスレッドとのロック競合なし
        self.compositor_state.scroll_offsets
            .get(&target.entity)
            .map(|offset| offset.apply_delta(delta));
    }
    // ...
}
```

### 6.2.4 ディスプレイパイプライン：メインスレッド→コンポジタのデータフロー

アプローチBでは、メインスレッドがDisplayListを生成しバウンドチャネルで送信：

```rust
pub struct DisplayPipeline {
    tx: SyncSender<DisplayList>,  // Bounded(1) — コンポジタが遅れている場合のバックプレッシャー
}

impl DisplayPipeline {
    pub fn submit(&self, display_list: DisplayList) {
        match self.tx.try_send(display_list) {
            Ok(()) => {}                    // コンポジタが次フレームでピックアップ
            Err(TrySendError::Full(_)) => {
                // コンポジタがまだ前のフレームを処理中。
                // このフレームをドロップ（フレームスキップ）。メインスレッドはブロックしない。
            }
            Err(TrySendError::Disconnected(_)) => {
                // コンポジタスレッドが死亡 — エラーリカバリ
            }
        }
    }
}
```

bounded(1)チャネルが自然なバックプレッシャーを提供：コンポジタが追いつけない場合、メインスレッドはバウンドなしのレイテンシを蓄積するのではなくフレームをドロップする。これが正しい動作 — レンダリングはキューイングすべきでない。

### 6.2.5 入力イベントルーティング

入力イベントはイベントタイプとページのイベントリスナー登録に基づいてコンポジタスレッドまたはメインスレッドにルーティング：

```
プラットフォーム入力（第23章）
  │
  ├─ スクロール/タッチ/ピンチ ─▶ コンポジタスレッド（ファストパス）
  │                               │
  │                               ├─ passiveリスナーまたはリスナーなし：
  │                               │    コンポジタで完全処理（メインスレッド不要）
  │                               │
  │                               └─ non-passiveリスナー登録済み：
  │                                    JS処理のためメインスレッドに転送。
  │                                    コンポジタは楽観的にスクロール、
  │                                    メインスレッドがpreventDefault()でキャンセル可能。
  │
  ├─ マウス/ポインター ──────▶ メインスレッド（ヒットテストにDOMが必要）
  │
  ├─ キーボード ───────────▶ メインスレッド（フォーカス管理、テキスト入力）
  │
  └─ リサイズ ─────────────▶ 両方（コンポジタが即座にビューポート調整、
                                    メインスレッドが再レイアウトをトリガー）
```

passiveイベントリスナーの区別は重要。`addEventListener('scroll', handler, { passive: true })`（またはリスナーなし）の場合、コンポジタはメインスレッドを待たない。Chromeがnon-passiveスクロールリスナーについて警告するのはこのため — コンポジタ→メインスレッドの往復がスクロールレイテンシを追加する。

## 6.3 rayonスレッドプール

### 6.3.1 設定

単一のrayon ThreadPoolがRenderer内のすべてのCPU並列作業で共有：

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
    // 2コアを予約：メインスレッド用1、コンポジタ用1
    // 低コアマシンでも最低2 rayonワーカー
    (cores.saturating_sub(2)).max(2)
}
```

| マシン | 物理コア | rayonワーカー | メイン | コンポジタ |
| --- | --- | --- | --- | --- |
| ラップトップ（4コア） | 4 | 2 | 1 | 1 |
| デスクトップ（8コア） | 8 | 6 | 1 | 1 |
| ワークステーション（16コア） | 16 | 14 | 1 | 1 |

### 6.3.2 作業分散

| 作業タイプ | 並列パターン | 備考 |
| --- | --- | --- |
| スタイル解決 | DOMサブツリーごとに並列 | Servo実証済み。独立サブツリーはデータ依存なし。メインスレッドもワーカーとして参加。 |
| レイアウト | 独立フォーマッティングコンテキストで並列 | ブロック、flex、gridコンテナで相互依存なしのものが並列レイアウト可能。スタイルより並列度は低い（レイアウトは逐次依存が多い）。 |
| 画像デコード | rayonタスクとしてスポーン | オフメインスレッドデコード。デコード済みビットマップは画像キャッシュ（第22章）に格納。 |
| フォントラスタライズ | rayonタスクとしてスポーン | フォントアトラス用グリフラスタライズ。グリフごとに独立。 |
| ペイント | レイヤーごとに並列 | 独立レイヤー（will-change、position:fixed等で作成）は並列ペイント可能。 |

### 6.3.3 メインスレッドの参加

並列フェーズ中、メインスレッドは`rayon::scope()`または`pool.install()`を呼び出し自身もワーカーとなる：

```rust
// スタイルフェーズ — メインスレッドが参加
pool.install(|| {
    dom.par_iter_subtrees()
        .for_each(|subtree| resolve_styles(subtree));
});
// すべてのサブツリー完了後にメインスレッドがここで再開
```

rayonワーカーが全作業を行う間メインスレッドが遊ぶのを回避。メインスレッドは最も価値あるコア — 並列フェーズ中も貢献すべき。

## 6.4 Web Workers

### 6.4.1 スレッドモデル

各Dedicated Workerは独自のJS/Wasm実行コンテキストを持つ独自のOSスレッドを取得：

```rust
pub struct DedicatedWorkerThread {
    /// 独自のScriptEngineインスタンス（個別JSヒープ）
    script_engine: ScriptEngine,
    /// 独自のWorkerSession（DOM アクセスなし — WorkerにはDOMがない）
    session: WorkerSession,
    /// 親（メインスレッドまたは別Worker）へのメッセージチャネル
    port: MessagePort,
    /// オプション：SharedArrayBufferマッピング
    shared_buffers: Vec<SharedArrayBuffer>,
}
```

| Workerタイプ | スレッドモデル | ライフタイム | DOMアクセス |
| --- | --- | --- | --- |
| Dedicated Worker | Worker1つにつき1 OSスレッド | 作成ドキュメントに紐付き | なし |
| Shared Worker | Worker1つにつき1 OSスレッド | オリジンに紐付き（タブ間で永続） | なし |
| Service Worker | 1 OSスレッド、イベント駆動 | オリジン単位、オンデマンドでアクティベート、アイドルタイムアウト | なし |

**なぜ1:1スレッディング（スレッドプールではなく）：** Workerは無期限に実行可能（ゲームループ、リアルタイム処理）。固定サイズのスレッドプールでは、すべてのプールスレッドが長寿命Workerを実行し短いタスクが実行できなくなるデッドロックが発生する。

### 6.4.2 postMessageとStructured Clone

Worker通信は`postMessage`を使用し、structured cloneシリアライゼーションでデータを転送：

```
メインスレッド                     Workerスレッド
  │                                │
  │  postMessage(data)             │
  │  ├─ structured cloneシリアライズ │
  │  ├─ send(bytes) ──────────▶   │
  │                                ├─ structured cloneデシリアライズ
  │                                ├─ MessageEvent配信
```

Transferableオブジェクト（ArrayBuffer、MessagePort、ImageBitmap）はゼロコピー — 所有権が移動しクローンされない。送信側はアクセスを失う。

structured clone形式はIPCシリアライゼーション（第5章）とIndexedDB値格納（第22章）と共有され、冗長なシリアライゼーション実装を回避。

### 6.4.3 SharedArrayBufferとAtomics

SharedArrayBufferはスレッド間の真の共有メモリを可能にする。セキュリティ要件によりゲート：

| 要件 | 理由 |
| --- | --- |
| Cross-Origin-Opener-Policy: same-origin | 高精度タイマーによるSpectre型攻撃を防止 |
| Cross-Origin-Embedder-Policy: require-corp | すべてのサブリソースがクロスオリジンローディングにオプトインすることを保証 |
| セキュアコンテキスト（HTTPS） | 基本的なセキュリティ要件 |

COOPとCOEPの両方が設定された場合、SharedArrayBufferが利用可能。複数のWorker（とメインスレッド）が同じ基盤メモリをマッピング可能：

```rust
pub struct SharedArrayBuffer {
    /// 共有メモリ領域、複数スレッドからアクセス可能
    memory: Arc<SharedMemory>,
    /// バイト長
    len: usize,
}

pub struct SharedMemory {
    /// mmapされた領域またはアラインされたアロケーション
    ptr: *mut u8,
    len: usize,
}

// Safety: SharedMemoryは並行アクセスのために明示的に設計。
// すべてのアクセスはAtomics（JS）またはアトミック操作（Wasm）を使用する必要がある。
unsafe impl Send for SharedMemory {}
unsafe impl Sync for SharedMemory {}
```

Atomics操作（Atomics.wait、Atomics.notify、Atomics.compareExchange等）はCPUアトミック命令にマッピング。Atomics.waitは呼び出しスレッドを中断（OS futexまたは同等物を使用）、Atomics.notifyは待機スレッドを起床。

**Atomics.waitはメインスレッドで禁止** — イベントループをブロックする。Workerのみがatomics.waitを呼び出せる。

### 6.4.4 OffscreenCanvas

OffscreenCanvasによりWorkerがメインスレッドの関与なしにcanvasにレンダリング可能：

```
メインスレッド                     Workerスレッド
  │                                │
  │  canvas.transferControlToOffscreen()
  │  ├─ 描画サーフェスの所有権 ──────▶
  │                                │
  │                                ├─ ctx = offscreen.getContext('2d')
  │                                ├─ ctx.drawImage(...)
  │                                ├─ offscreen.commit()
  │                                │     └─ フレームがコンポジタに送信
```

コミットされたフレームは（メインスレッドを経由せず）直接コンポジタスレッドに送られ、メインスレッド負荷に依存しない60fpsのWorker駆動レンダリングを実現。コンポジタはOffscreenCanvas出力をDOMレンダリングレイヤーと並んでレイヤーとして統合。

## 6.5 ECS並行モデル

### 6.5.1 コンポーネントアクセスルール

ECSはすべてのDOMおよびレンダリング状態をエンティティ上のコンポーネントとして格納。異なるスレッドが異なるコンポーネントに異なるパーミッションでアクセス：

| コンポーネントカテゴリ | メインスレッド | rayonプール | コンポジタ | Workers |
| --- | --- | --- | --- | --- |
| DOM構造（Parent, Children, NextSibling） | 読取/書込 | 読取（スタイル中） | アクセスなし | アクセスなし |
| Attributes | 読取/書込 | 読取（スタイル中） | アクセスなし | アクセスなし |
| ComputedStyle | 書込（flush経由）、読取 | **書込**（並列スタイル） | 読取（B: DL経由、C: 直接） | アクセスなし |
| LayoutResult（位置、サイズ） | 書込（レイアウト）、読取 | **書込**（並列レイアウト） | 読取（B: DL経由、C: 直接） | アクセスなし |
| Layer（合成ヒント） | 書込（ペイント） | 書込（並列ペイント） | 読取（B: DL経由、C: 直接） | アクセスなし |
| ScrollState | 読取/書込 | アクセスなし | **書込**（コンポジタスクロール） | アクセスなし |
| AnimationState | 読取/書込 | アクセスなし | **書込**（コンポジタanim） | アクセスなし |

### 6.5.2 同期戦略

重要な洞察は、**異なるスレッドクラスがイベントループの異なるフェーズでECSにアクセスする**ことで、自然な非重複ウィンドウが生まれること：

```
イベントループフェーズ         ECS上のアクティブスレッド
─────────────────────────────────────────────────
Phase 1: イベント収集     メイン（IPC読取）       コンポジタ（独立）
Phase 2: JS実行          メイン（SS経由読書）    コンポジタ（独立）
Phase 2: セッションflush  メイン（コンポーネント書込）コンポジタ（独立）
Phase 3a: スタイル        rayon（スタイル書込）   コンポジタ（旧フレーム読取）
Phase 3b: レイアウト      rayon（レイアウト書込） コンポジタ（旧フレーム読取）
Phase 3c: ペイント        メイン+rayon（レイヤー書込）コンポジタ（旧フレーム読取）
Phase 3d: コミット        メイン（DL送信）       コンポジタ（新フレームアクティベート）
Phase 4: アイドル         メイン（読取/アイドル作業）コンポジタ（独立）
Phase 5: 待機             — スリープ —          コンポジタ（独立）
```

アプローチBでは、コンポジタは自身のコピー上で動作しECSに直接アクセスしない。競合はゼロ。

アプローチCでは、コンポジタがECSコンポーネントを読む。潜在的な競合はPhase 3（rayon書込）とコンポジタの読み取りの間。これは以下で解決：

1. **ダブルバッファレイヤーデータ：** メインスレッドがPhase 3中に「バックバッファ」に書き込み、コミット（Phase 3d）でアトミックに「フロントバッファ」にスワップ。コンポジタは常にフロントバッファを読む。
2. **アトミックスクロール/アニメーション状態：** CompositorMutableState（6.2.3節）がロックフリーatomicsを使用。競合なし。

### 6.5.3 Send/Sync分類

```rust
// ECS World — NOT Send、NOT Sync。メインスレッドが所有。
pub struct EcsWorld { /* ... */ }

// コンポーネントストレージ — Send（rayon用にスレッド間転送可能）、
// ただしアクセスはフェーズで協調（ロックではない）。
pub struct ComponentStorage<T: Component> { /* ... */ }
unsafe impl<T: Component + Send> Send for ComponentStorage<T> {}

// DisplayList — Send（コンポジタスレッドに転送）。
pub struct DisplayList { /* ... */ }
unsafe impl Send for DisplayList {}

// CompositorMutableState — Send + Sync（アトミック操作）。
pub struct CompositorMutableState { /* ... */ }
unsafe impl Send for CompositorMutableState {}
unsafe impl Sync for CompositorMutableState {}

// ScriptSession — NOT Send。メインスレッド専用。
pub struct ScriptSession { /* ... */ }

// WorkerSession — Send。Workerスレッドごとに1つ。
pub struct WorkerSession { /* ... */ }
unsafe impl Send for WorkerSession {}
```

EcsWorldは`!Send` — 別スレッドに移動できない。コンパイル時に強制。rayonアクセスは並列フェーズ中にスコープ参照を通じて明示的に許可（メインスレッドが`rayon::scope()`に`&mut`参照を渡す）。

## 6.6 Browser Processスレッドモデル

Rendererより単純。tokioマルチスレッドランタイムがすべてのI/Oを処理：

```
Browser Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  tokioランタイム（マルチスレッド）                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 非同期タスク：                                                │   │
│  │   ├─ IPCディスパッチ（Renderer、Network、GPUから受信）        │   │
│  │   ├─ ストレージI/O（spawn_blocking経由のSQLite）             │   │
│  │   ├─ プロセスライフサイクル管理                                │   │
│  │   ├─ 拡張ホスト                                              │   │
│  │   └─ プロファイル管理                                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  UIスレッド (1)                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ Chromeレンダリング（egui/iced）                               │   │
│  │ プラットフォームイベントループ統合（winit）                     │   │
│  │ メニュー、ダイアログ、ファイルピッカーディスパッチ              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

| スレッド | 役割 | 備考 |
| --- | --- | --- |
| UIスレッド | Chrome UIレンダリング、プラットフォームイベントループ | macOSでは「メインスレッド」でなければならない（CocoaはUIをメインスレッドで要求）。tokioタスクの処理結果を受信。 |
| tokioワーカー | 非同期I/O、IPCディスパッチ、タスク実行 | デフォルト：min(4, num_cpus)。すべてのIPCメッセージルーティングを処理。 |
| spawn_blockingプール | SQLite操作、ファイルI/O | tokioのブロッキングプール。SQLite呼び出しは同期的；`spawn_blocking`でラップし非同期ワーカーのスターベーションを防止。 |

### 6.6.1 ストレージI/Oパターン

すべてのSQLite操作（第22章）はtokioのブロッキングプールで実行し非同期ワーカーのスターベーションを回避：

```rust
// Browser Process — Rendererからのストレージリクエスト処理
async fn handle_storage_request(
    req: StorageRequest,
    storage: Arc<OriginStorageManager>,
) -> StorageResponse {
    // SQLite I/Oをブロッキングスレッドにオフロード
    tokio::task::spawn_blocking(move || {
        let conn = storage.connection(&req.origin, req.storage_type)?;
        conn.execute(&req.op)
    })
    .await
    .unwrap()
}
```

## 6.7 Network Processスレッドモデル

最大I/Oスループットのために設計：

```
Network Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  tokioランタイム（マルチスレッド）                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 非同期タスク：                                                │   │
│  │   ├─ HTTPクライアント（hyper） — リクエスト単位の非同期タスク  │   │
│  │   ├─ DNS解決（DoHクエリ）                                    │   │
│  │   ├─ TLSハンドシェイク（rustls async）                       │   │
│  │   ├─ WebSocket接続                                           │   │
│  │   ├─ IPCディスパッチ（Rendererからのリクエスト）              │   │
│  │   ├─ 接続プール管理                                          │   │
│  │   └─ CORS / Cookie / セキュリティヘッダ施行                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  spawn_blockingプール                                               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ HTTPキャッシュSQLite操作                                      │   │
│  │ レスポンスボディファイルI/O（キャッシュ読み書き）              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

Network Processはほぼ完全に非同期。唯一のブロッキング操作はHTTPキャッシュのディスクI/Oで、ブロッキングプールで実行。hyper、rustls、h3はすべて非同期ネイティブでtokioワーカー上で直接実行。

## 6.8 GPU Processスレッドモデル

```
GPU Process
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  GPUスレッド (1)                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ wgpuデバイス管理                                              │   │
│  │ Rendererからディスプレイリスト/合成フレームを受信             │   │
│  │ テクスチャアップロードと管理                                  │   │
│  │ ラスタライゼーション（Vello）                                 │   │
│  │ サーフェスプレゼンテーション（vsync）                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  IPCスレッド (1)                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ tokio current_thread                                         │   │
│  │ RendererとBrowserからIPC受信                                 │   │
│  │ ディスプレイリストのデシリアライズ                            │   │
│  │ GPUスレッドへの作業キューイング                               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

GPU Processは意図的にシンプル：GPUワーク用1スレッド、IPC用1スレッド。GPUドライバはしばしばスレッドセーフでないため、すべてのwgpu呼び出しは単一スレッドを通る。GPUプロセスがRenderer にマージされる場合（Phase 4+）、GPUスレッドがコンポジタスレッドとなる。

## 6.9 スレッドアフィニティとコア割り当て

### 6.9.1 全体コア予算

典型的な8コアマシンで1つのアクティブRendererの場合：

| スレッド | コア | アフィニティ | 備考 |
| --- | --- | --- | --- |
| Rendererメイン | 1 | ソフト（OSスケジュール） | システムで最高優先度のスレッド |
| コンポジタ | 1 | ソフト | 2番目に高い優先度。スターベーション禁止。 |
| rayonプール | 6ワーカー | ソフト | 並列フェーズ中に利用可能コアを埋める |
| Web Workers | rayonコアと共有 | ソフト | WorkerはrayonとCPUを競合。許容可能 — Workerはレンダーフェーズ中にCPUを飽和させることは稀。 |
| Browser tokio | 2–4ワーカー | 共有 | I/Oバウンド、CPU集約は稀 |
| Network tokio | 2–4ワーカー | 共有 | I/Oバウンド |
| GPUスレッド | 1 | ソフト | しばしばGPUバウンドでCPUバウンドではない |

elidexはスレッドを特定コアにピン止めしない（ハードアフィニティ）。OSスケジューリングがほとんどのワークロードに十分。プラットフォームがサポートする場合にスレッド優先度を設定：

```rust
// プラットフォーム固有の優先度ヒント
fn set_compositor_priority() {
    #[cfg(target_os = "linux")]
    {
        // SCHED_FIFOまたはnice値の引き上げ
        unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -5); }
    }
    #[cfg(target_os = "macos")]
    {
        // QoS: user-interactive（最高の非リアルタイム）
        // pthread_set_qos_class_self_np経由で設定
    }
}
```

### 6.9.2 tokioとrayonの共存

tokioとrayonは根本的に異なる目的に対応し干渉しない：

| 側面 | rayon | tokio |
| --- | --- | --- |
| ワークロードタイプ | CPUバウンド（スタイル、レイアウト、デコード） | I/Oバウンド（ネットワーク、IPC、タイマー） |
| スケジューリング | ワークスティーリング、fork-join | 非同期タスクスケジューリング、epoll/kqueue |
| アクティブな期間 | レンダーフェーズ（Phase 3） | 全フェーズ（特にPhase 1、5） |
| コア使用パターン | バースト（並列フェーズ中100% CPU） | 低CPU（ほとんどの時間I/O待ち） |

レンダーフェーズ中、rayonが利用可能コアを飽和。tokioワーカーはほとんど何もしない（レンダリング中のI/O完了はない）。レンダーフェーズ外（JS実行、アイドル）では、rayonワーカーが遊びtokioがI/Oを処理。ワークロードは自然に相補的。

## 6.10 段階的移行：B → C

コンポジタ抽象化（FrameSourceトレイト）が全体的なプロセスモデル緩和（第5章）に連動した段階的移行を可能に：

| フェーズ | プロセスモデル | コンポジタアプローチ | FrameSource実装 |
| --- | --- | --- | --- |
| Phase 1–3 | SiteIsolation、GPU Process分離 | **B（メッセージパッシング）** | DisplayListFrameSource |
| Phase 4（移行期） | PerTabまたはShared、GPUがRendererにマージ | B（同一プロセスチャネル） | DisplayListFrameSource (LocalChannel) |
| Phase 4+（最適化） | SingleProcessまたはShared | **C（共有ECS）** | EcsFrameSource |

B → C移行に必要なこと：

1. **ダブルバッファレイヤーコンポーネントをECSに追加。** ペイントがバックバッファに書き込み、コミットでアトミックスワップ。
2. **CompositorMutableStateを追加。** アトミックスクロールオフセットとアニメーションtick、メインスレッドとコンポジタスレッド間で共有。
3. **FrameSource実装を差し替え。** 設定変更、アーキテクチャの書き直し不要。
4. **ベンチマーク。** CはDisplayListシリアライゼーションコストを排除するがアトミック読み取りオーバーヘッドを導入。正味の恩恵をプロファイルで検証。

ステップ4は重要 — Cがすべてのワークロードでbより速いとは限らない。多数の小さなレイヤーを持つページはCの恩恵を受ける可能性がある（多数の小構造体のシリアライゼーション回避）。少数の大きなレイヤーのページでは差がないかもしれない。FrameSource抽象化によりアプローチ間のA/Bテストが可能。

## 6.11 まとめ：スレッドマップ

```
┌─ Browser Process ─────────────────────────────┐
│  UIスレッド ─── chromeレンダリング              │
│  tokioプール ── IPC、ストレージI/O、管理        │
│  blockingプール ─ SQLite操作                    │
└────────────────────────────────────────────────┘
         │ IPC
┌─ Renderer Process (× N) ──────────────────────┐
│  メインスレッド ── イベントループ、DOM、script、paint │
│  コンポジタ ──── レイヤー、スクロール、GPU送信  │
│  rayonプール ─── スタイル、レイアウト、デコード  │
│  Workerスレッド ── Web Workers（1:1マッピング） │
│  tokio reactor ── I/Oポーリング（メインスレッド上）│
└────────────────────────────────────────────────┘
         │ IPC
┌─ Network Process ─────────────────────────────┐
│  tokioプール ── HTTP、DNS、TLS、WebSocket       │
│  blockingプール ─ キャッシュI/O                  │
└────────────────────────────────────────────────┘
         │ IPC
┌─ GPU Process ─────────────────────────────────┐
│  GPUスレッド ── wgpu、ラスタライズ、プレゼント  │
│  IPCスレッド ── ディスプレイリスト受信           │
└────────────────────────────────────────────────┘
```
