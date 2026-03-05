
# 26. エンベディングAPI

## 26.1 概要

エンベディングAPIはelidex-appの公開契約：サードパーティアプリケーションがelidexをWebレンダリングエンジンとして組み込むために使用するRust API。CEF（Chromium Embedded Framework）、WebView2、TauriのwryのRustネイティブ版に相当。

```rust
use elidex_app::{Engine, EngineConfig, View, ViewConfig};

fn main() {
    // 1. エンジン初期化
    let engine = Engine::builder()
        .with_config(EngineConfig::default())
        .build()
        .expect("engine init");

    // 2. ビュー作成（Webコンテンツ領域）
    let view = engine.create_view(ViewConfig {
        url: "https://example.com".into(),
        width: 1280,
        height: 720,
        ..Default::default()
    });

    // 3. イベントループ実行
    engine.run();
}
```

### 26.1.1 既存ソリューションとの比較

|  | Electron | Tauri | elidex-app |
| --- | --- | --- | --- |
| エンジン | Chromium（フル） | OS WebView | elidex-core（スリム） |
| スクリプトランタイム | V8（JSのみ） | OS JSエンジン | Boa → elidex-js (Rust) + wasmtime（Ch. 14 §14.1.2参照） |
| バイナリサイズ | ~150 MB | ~3-8 MB | 目標: ~5-15 MB |
| レンダリング一貫性 | 同一（バンドル） | OSにより異なる | 同一（バンドル） |
| レガシーオーバーヘッド | フル後方互換 | フル（OS依存） | ゼロ（HTML5のみ） |
| アプリ言語 | JS/TSのみ | Rustバックエンド + JSフロントエンド | Wasm対応全言語 |
| ネイティブ統合 | Node.js | Rustバックエンド | Wasmホスト関数 |
| カスタマイズ可能エンジン | 不可 | 不可 | 可（フィーチャーフラグ） |

elidex-appの主要優位性：OS WebViewの差異なく一貫したレンダリング、厳格なHTML5パーサーがマークアップのコンパイル時的エラー検出を提供、フィーチャーフラグでアプリに必要な機能のみ組み込み可能。

### 26.1.2 設計原則

- **Rustファースト**：主要APIはRust。非Rustエンベッダー向けにcbindgen経由でCバインディングを生成。
- **ビルダーパターン**：合理的なデフォルト付きのビルダーで設定。シンプルなケースでは最小ボイラープレート。
- **階層化**：シンプルなことはシンプルに（URLロード、ウィンドウ取得）。複雑なことも可能（カスタムリソースローダー、ネイティブ↔Webブリッジ、ヘッドレスレンダリング）。
- **ウィンドウイングに無頓着**：任意のウィンドウイングツールキット（winit、SDL2、ネイティブプラットフォーム、ヘッドレス）で動作。

## 26.2 Engine

### 26.2.1 エンジン初期化

```rust
pub struct Engine {
    // 内部：プロセス管理、GPUコンテキスト、共有リソース
}

pub struct EngineConfig {
    /// プロセスモデル
    pub process_mode: ProcessMode,
    /// 機能フラグ
    pub features: FeatureFlags,
    /// コーデック設定（第20章）
    pub codecs: CodecConfig,
    /// OPFS、キャッシュ等のストレージディレクトリ
    pub data_dir: Option<PathBuf>,
    /// ユーザーエージェント文字列
    pub user_agent: Option<String>,
    /// ログ設定
    pub log_level: LogLevel,
}

pub enum ProcessMode {
    /// すべてのコンポーネントが現在のプロセス内。最もシンプル。分離なし。
    SingleProcess,
    /// Rendererを別プロセスに。信頼できないコンテンツに推奨。
    MultiProcess,
}

pub struct FeatureFlags {
    /// compat層を有効化（FileReader、SMIL、レガシーコーデック等）
    pub compat: bool,
    /// DevToolsサーバーを有効化
    pub devtools: bool,
    /// Web Audio APIを有効化
    pub web_audio: bool,
    /// メディアパイプラインを有効化（ビデオ/オーディオ再生）
    pub media: bool,
    /// WebGLを有効化
    pub webgl: bool,
    /// WebGPUを有効化
    pub webgpu: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            compat: false,       // elidex-appデフォルト：モダンのみ
            devtools: cfg!(debug_assertions),
            web_audio: true,
            media: true,
            webgl: true,
            webgpu: true,
        }
    }
}

impl Engine {
    pub fn builder() -> EngineBuilder { EngineBuilder::new() }

    /// 新しいビュー（Webコンテンツ領域）を作成。
    pub fn create_view(&self, config: ViewConfig) -> View { /* ... */ }

    /// エンジンイベントループを実行。すべてのビューが閉じるまでブロック。
    pub fn run(&self) { /* ... */ }

    /// イベントループの1回のイテレーションを実行（エンベッダー駆動ループ用）。
    pub fn pump(&self) -> PumpResult { /* ... */ }

    /// エンジンをシャットダウン。
    pub fn shutdown(self) { /* ... */ }
}

pub enum PumpResult {
    /// まだ作業あり。
    Continue,
    /// すべてのビューが閉じた、エンジンをシャットダウン可能。
    Exit,
}
```

### 26.2.2 イベントループ統合

イベントループ所有権の2モード：

**エンジン所有**（シンプルケース）：
```rust
engine.run();  // ブロック、すべてのイベントを内部処理
```

**エンベッダー所有**（既存イベントループとの統合）：
```rust
loop {
    // エンベッダー独自のイベント処理
    process_my_events();

    // elidexをポンプ
    match engine.pump() {
        PumpResult::Continue => {},
        PumpResult::Exit => break,
    }

    // エンベッダーのレンダリング
    render_my_ui();
}
```

## 26.3 View

### 26.3.1 ビュー設定

```rust
pub struct ViewConfig {
    /// 初期コンテンツ
    pub content: ViewContent,
    /// ウィンドウ/サーフェス設定
    pub surface: SurfaceConfig,
    /// パーミッション
    pub permissions: PermissionConfig,
    /// ナビゲーションポリシー
    pub navigation_policy: NavigationPolicy,
    /// カスタムリソースローダー
    pub resource_loader: Option<Box<dyn ResourceLoader>>,
}

pub enum ViewContent {
    /// URLをロード
    Url(String),
    /// 文字列からHTMLをロード
    Html(String),
    /// ローカルファイルからロード
    File(PathBuf),
    /// ブランクページ
    Blank,
}

pub enum SurfaceConfig {
    /// 指定設定で新ウィンドウを作成
    CreateWindow {
        title: String,
        width: u32,
        height: u32,
        resizable: bool,
        decorations: bool,
        transparent: bool,
    },
    /// 既存ウィンドウにアタッチ
    AttachToWindow {
        handle: raw_window_handle::RawWindowHandle,
        display: raw_window_handle::RawDisplayHandle,
        size: (u32, u32),
    },
    /// ヘッドレスレンダリング（ウィンドウなし）
    Headless {
        width: u32,
        height: u32,
    },
}

pub struct PermissionConfig {
    /// 事前付与パーミッション（プロンプトなし）
    pub grants: Vec<Permission>,
    /// アプリケイパビリティ（拡張パーミッション、第8章§8.8）
    pub capabilities: Vec<AppCapability>,
}

pub enum NavigationPolicy {
    /// すべてのナビゲーションを許可（ブラウザのデフォルト）
    AllowAll,
    /// すべてのナビゲーションをブロック（シングルページアプリ）
    BlockAll,
    /// カスタムハンドラ
    Custom(Box<dyn NavigationHandler>),
}

pub trait NavigationHandler: Send + Sync {
    /// 各ナビゲーション前に呼び出し。決定を返す。
    fn on_navigate(&self, request: &NavigationRequest) -> NavigationDecision;
}

pub struct NavigationRequest {
    pub url: Url,
    pub initiator: NavigationInitiator,
    pub is_main_frame: bool,
}

pub enum NavigationDecision {
    Allow,
    Block,
    /// 別URLにリダイレクト
    Redirect(Url),
}
```

### 26.3.2 View API

```rust
pub struct View {
    // 内部ハンドル
}

impl View {
    // === コンテンツロード ===

    /// URLにナビゲート。
    pub fn load_url(&self, url: &str) { /* ... */ }

    /// HTMLコンテンツを直接ロード。
    pub fn load_html(&self, html: &str, base_url: Option<&str>) { /* ... */ }

    /// 現在のページをリロード。
    pub fn reload(&self) { /* ... */ }

    /// ローディングを停止。
    pub fn stop(&self) { /* ... */ }

    // === ナビゲーション ===

    /// 履歴を戻る。
    pub fn go_back(&self) -> bool { /* 履歴なしならfalseを返す */ }

    /// 履歴を進む。
    pub fn go_forward(&self) -> bool { /* ... */ }

    /// 戻るナビゲーションが可能か。
    pub fn can_go_back(&self) -> bool { /* ... */ }

    /// 進むナビゲーションが可能か。
    pub fn can_go_forward(&self) -> bool { /* ... */ }

    /// 現在のURL。
    pub fn url(&self) -> String { /* ... */ }

    /// 現在のページタイトル。
    pub fn title(&self) -> String { /* ... */ }

    // === JavaScript実行 ===

    /// JavaScriptを実行し結果を返す。
    pub async fn evaluate_script(&self, script: &str) -> Result<JsValue, JsError> { /* ... */ }

    /// 結果を待たずにJavaScriptを実行。
    pub fn execute_script(&self, script: &str) { /* ... */ }

    // === ネイティブ ↔ Webブリッジ ===

    /// Rust関数をJavaScriptに公開。
    /// JSから`window.__elidex.call(name, args)`として呼び出し可能。
    pub fn expose_function<F, A, R>(&self, name: &str, handler: F)
    where
        F: Fn(A) -> R + Send + Sync + 'static,
        A: serde::de::DeserializeOwned,
        R: serde::Serialize,
    { /* ... */ }

    /// 双方向メッセージチャネルを作成。
    pub fn create_channel(&self) -> (ChannelSender, ChannelReceiver) { /* ... */ }

    /// ページにメッセージを送信（window.__elidex.onMessage経由で受信）。
    pub fn post_message(&self, message: &impl serde::Serialize) { /* ... */ }

    // === イベントフック ===

    /// ページロードイベントのコールバックを設定。
    pub fn on_load_state_changed(&self, callback: impl Fn(LoadState) + Send + 'static) { /* ... */ }

    /// タイトル変更のコールバックを設定。
    pub fn on_title_changed(&self, callback: impl Fn(&str) + Send + 'static) { /* ... */ }

    /// URL変更のコールバックを設定。
    pub fn on_url_changed(&self, callback: impl Fn(&str) + Send + 'static) { /* ... */ }

    /// コンソールメッセージのコールバックを設定。
    pub fn on_console_message(&self, callback: impl Fn(ConsoleMessage) + Send + 'static) { /* ... */ }

    /// パーミッションリクエストのコールバックを設定。
    pub fn on_permission_request(&self, callback: impl Fn(PermissionRequest) -> PermissionResponse + Send + 'static) { /* ... */ }

    /// JavaScriptダイアログ（alert、confirm、prompt）のコールバックを設定。
    pub fn on_dialog(&self, callback: impl Fn(DialogRequest) -> DialogResponse + Send + 'static) { /* ... */ }

    /// ダウンロードリクエストのコールバックを設定。
    pub fn on_download(&self, callback: impl Fn(DownloadRequest) -> DownloadDecision + Send + 'static) { /* ... */ }

    // === レンダリング制御 ===

    /// フレームポリシーを設定（第15章）。
    pub fn set_frame_policy(&self, policy: FramePolicy) { /* ... */ }

    /// 現在のページを画像としてキャプチャ。
    pub async fn capture_screenshot(&self) -> Result<ImageBuffer, CaptureError> { /* ... */ }

    /// ビューをリサイズ。
    pub fn resize(&self, width: u32, height: u32) { /* ... */ }

    /// デバイススケールファクターを設定。
    pub fn set_scale_factor(&self, factor: f64) { /* ... */ }

    // === ライフサイクル ===

    /// ビューを閉じリソースを解放。
    pub fn close(self) { /* ... */ }
}

pub enum LoadState {
    Started,
    Committed,
    DomContentLoaded,
    Complete,
    Failed(NavigationError),
}

pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub message: String,
    pub source: String,
    pub line: u32,
}
```

## 26.4 ネイティブ ↔ Webブリッジ

### 26.4.1 関数インジェクション

JavaScriptから呼び出し可能なRust関数を公開：

```rust
// Rust側
view.expose_function("greet", |name: String| -> String {
    format!("Hello, {}!", name)
});

view.expose_function("get_user", |id: u64| -> User {
    database.find_user(id)
});

// JavaScript側
const greeting = await window.__elidex.call("greet", "World");
// greeting === "Hello, World!"

const user = await window.__elidex.call("get_user", 42);
// user === { name: "Alice", email: "alice@example.com" }
```

引数と戻り値はserde_json経由でシリアライズ。JavaScriptの観点からは呼び出しは非同期（Promiseを返す）。

### 26.4.2 メッセージチャネル

双方向ストリーミング通信用：

```rust
// Rust側
let (sender, receiver) = view.create_channel();

// JSに送信
sender.send(&MyEvent { kind: "update", data: 42 });

// JSから受信
tokio::spawn(async move {
    while let Some(msg) = receiver.recv().await {
        handle_message(msg);
    }
});

// JavaScript側
window.__elidex.onMessage = (msg) => {
    console.log("From Rust:", msg);
};

window.__elidex.postMessage({ action: "click", x: 100, y: 200 });
```

### 26.4.3 ブリッジAPI名前空間

すべてのブリッジAPIが`window.__elidex`下に存在：

```typescript
interface ElidexBridge {
    // 関数呼び出し（expose_function経由で公開）
    call(name: string, ...args: any[]): Promise<any>;

    // メッセージパッシング
    postMessage(message: any): void;
    onMessage: ((message: any) => void) | null;

    // アプリメタデータ
    readonly appName: string;
    readonly appVersion: string;

    // プラットフォーム情報
    readonly platform: "macos" | "windows" | "linux" | "android" | "ios";
}
```

## 26.5 カスタムリソースローダー

エンベッダーがリソースリクエストをインターセプトして処理可能：

```rust
pub trait ResourceLoader: Send + Sync {
    /// リソースリクエストをインターセプト。
    /// Some(response)で処理、Noneで通常ロードにフォールスルー。
    fn load(&self, request: &ResourceRequest) -> Option<ResourceResponse>;
}

pub struct ResourceRequest {
    pub url: Url,
    pub method: HttpMethod,
    pub headers: HeaderMap,
    pub resource_type: ResourceType,
}

pub struct ResourceResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: ResourceBody,
}

pub enum ResourceBody {
    Bytes(Bytes),
    Stream(ByteStream),
}

// 例：埋め込みデータからアプリアセットを配信
struct EmbeddedAssets;

impl ResourceLoader for EmbeddedAssets {
    fn load(&self, request: &ResourceRequest) -> Option<ResourceResponse> {
        if request.url.scheme() == "app" {
            let path = request.url.path();
            let data = include_bytes_matching(path)?;
            Some(ResourceResponse {
                status: 200,
                headers: content_type_for(path),
                body: ResourceBody::Bytes(data.into()),
            })
        } else {
            None  // ネットワークにフォールスルー
        }
    }
}
```

これは第10章のAppResourceLoaderパターンと統合し、そのエンベディングAPIサーフェスを提供。

## 26.6 マルチビュー

Engineが複数のViewをホスト可能。各Viewが独立したWebコンテンツをレンダリング：

```rust
let engine = Engine::builder().build().unwrap();

let main_view = engine.create_view(ViewConfig {
    content: ViewContent::Url("https://app.example.com".into()),
    surface: SurfaceConfig::CreateWindow { title: "Main".into(), width: 1280, height: 720, .. },
    ..Default::default()
});

let settings_view = engine.create_view(ViewConfig {
    content: ViewContent::Html(include_str!("settings.html").into()),
    surface: SurfaceConfig::CreateWindow { title: "Settings".into(), width: 600, height: 400, .. },
    ..Default::default()
});

// ViewはEngineのGPUコンテキストとプロセスインフラを共有
// ただしDOM、スクリプトコンテキスト、ストレージは独立
```

MultiProcessモードでは各Viewが独自のRendererプロセスを取得。SingleProcessモードではViewがプロセスを共有するが、ECS Worldは分離。

## 26.7 ヘッドレスモード

サーバーサイドレンダリング、テスト、スクリーンショット生成用：

```rust
let engine = Engine::builder()
    .with_config(EngineConfig {
        process_mode: ProcessMode::SingleProcess,
        ..Default::default()
    })
    .build()
    .unwrap();

let view = engine.create_view(ViewConfig {
    content: ViewContent::Url("https://example.com".into()),
    surface: SurfaceConfig::Headless { width: 1920, height: 1080 },
    ..Default::default()
});

// ページロードを待機
view.on_load_state_changed(|state| {
    if matches!(state, LoadState::Complete) {
        // スクリーンショットキャプチャ
        let image = view.capture_screenshot().await.unwrap();
        image.save("screenshot.png");
    }
});

engine.run();
```

ヘッドレスモードはGPUコンテキストなしでVelloのCPUバックエンド（第15章§15.6.4）を使用してレンダリング。

## 26.8 DevTools

DevToolsが有効（`features.devtools = true`）の場合、エンジンがChrome DevTools Protocol (CDP)サーバーを起動：

```rust
let engine = Engine::builder()
    .with_config(EngineConfig {
        features: FeatureFlags { devtools: true, ..Default::default() },
        ..Default::default()
    })
    .build()
    .unwrap();

// DevToolsが ws://localhost:9222 で利用可能
// Chrome DevToolsまたは任意のCDPクライアントで接続
```

CDPサーバーが提供：DOM検査、CSS編集、JavaScriptデバッグ、ネットワーク監視、パフォーマンスプロファイリング、コンソールアクセス。

## 26.9 C API（cbindgen）

非Rustエンベッダー向けにcbindgen経由でC互換APIを生成：

```c
// elidex.h（自動生成）

typedef struct ElidexEngine ElidexEngine;
typedef struct ElidexView ElidexView;

ElidexEngine* elidex_engine_create(const ElidexEngineConfig* config);
void elidex_engine_destroy(ElidexEngine* engine);
void elidex_engine_run(ElidexEngine* engine);
int elidex_engine_pump(ElidexEngine* engine);

ElidexView* elidex_view_create(ElidexEngine* engine, const ElidexViewConfig* config);
void elidex_view_destroy(ElidexView* view);
void elidex_view_load_url(ElidexView* view, const char* url);
void elidex_view_load_html(ElidexView* view, const char* html, const char* base_url);

int elidex_view_evaluate_script(
    ElidexView* view,
    const char* script,
    ElidexJsResultCallback callback,
    void* user_data
);

void elidex_view_expose_function(
    ElidexView* view,
    const char* name,
    ElidexFunctionCallback callback,
    void* user_data
);

void elidex_view_post_message(ElidexView* view, const char* json_message);
void elidex_view_set_message_callback(
    ElidexView* view,
    ElidexMessageCallback callback,
    void* user_data
);
```

C APIはRust APIの薄いラッパーで、不透明ポインタとコールバック関数を使用。

## 26.10 API安定性

| コンポーネント | 安定性 | ポリシー |
| --- | --- | --- |
| `Engine`、`View`、`ViewConfig` | 安定 | セマンティックバージョニング。破壊的変更にはメジャーバージョンバンプが必要。 |
| `expose_function`、`post_message` | 安定 | コアブリッジAPI、バージョン間で維持。 |
| `FramePolicy`、`NavigationPolicy` | 安定 | 公開enum、追加的変更のみ（新バリアント）。 |
| `FeatureFlags` | 準安定 | 新フラグ追加可能。既存フラグは廃止なしに削除しない。 |
| `ResourceLoader`トレイト | 準安定 | デフォルト実装付きのトレイトメソッド追加可能。 |
| 内部型（`BlobId`、`EntityId`等） | 不安定 | 公開APIに露出しない。 |
| C API | 安定 | マイナーバージョン間でABI互換。 |

廃止ポリシー：廃止APIは次のメジャーバージョンでの削除前に少なくとも1マイナーバージョンの間`#[deprecated]`をマーク。

## 26.11 例：完全なアプリケーション

```rust
use elidex_app::*;

fn main() {
    let engine = Engine::builder()
        .with_config(EngineConfig {
            process_mode: ProcessMode::SingleProcess,
            features: FeatureFlags {
                compat: false,
                media: true,
                ..Default::default()
            },
            data_dir: Some("./app_data".into()),
            ..Default::default()
        })
        .build()
        .expect("engine init");

    let view = engine.create_view(ViewConfig {
        content: ViewContent::Url("app://index.html".into()),
        surface: SurfaceConfig::CreateWindow {
            title: "My App".into(),
            width: 1280,
            height: 720,
            resizable: true,
            decorations: true,
            transparent: false,
        },
        permissions: PermissionConfig {
            grants: vec![Permission::Notifications, Permission::ClipboardRead],
            capabilities: vec![
                AppCapability::FileRead("./documents/*".into()),
                AppCapability::FileWrite("./documents/*".into()),
            ],
        },
        navigation_policy: NavigationPolicy::Custom(Box::new(AppNavigationHandler)),
        resource_loader: Some(Box::new(EmbeddedAssets)),
    });

    // ネイティブ機能をWebコンテンツに公開
    view.expose_function("save_file", |args: SaveFileArgs| -> Result<(), String> {
        std::fs::write(&args.path, &args.content).map_err(|e| e.to_string())
    });

    view.expose_function("list_files", |dir: String| -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|entries| entries.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into())).collect())
            .unwrap_or_default()
    });

    // イベントをリッスン
    view.on_title_changed(|title| {
        println!("Title: {}", title);
    });

    view.on_console_message(|msg| {
        println!("[{}] {}", msg.level, msg.message);
    });

    engine.run();
}

struct AppNavigationHandler;

impl NavigationHandler for AppNavigationHandler {
    fn on_navigate(&self, request: &NavigationRequest) -> NavigationDecision {
        if request.url.scheme() == "app" || request.url.scheme() == "https" {
            NavigationDecision::Allow
        } else {
            NavigationDecision::Block
        }
    }
}
```
