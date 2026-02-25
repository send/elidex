
# 23. プラットフォーム抽象化レイヤー

エンジン章（第5〜22章）はプラットフォーム非依存の内部を記述する。本章ではそれらの内部をホストOSに接続する抽象化レイヤーを定義する。設計思想はエンジンの仕様駆動プラガビリティ（Web標準に基づくcore/compat/deprecated）から**トレイト契約プラガビリティ**に移行し、OS固有の実装がプラットフォームプラグインとして注入される。

| レイヤー | プラグイン根拠 | 思想 |
| --- | --- | --- |
| エンジン（HTML/CSS/DOM/JS） | Web標準 + SpecLevel | **削る**：core/compat/deprecated |
| プラットフォーム抽象化 | OS API + トレイト契約 | **差し替える**：OS固有実装を注入 |
| ブラウザシェル（第24章） | トレイト契約のみ | **拡げる/差し替える**：UIとUXをカスタマイズ |

## 23.1 プラットフォームトレイトアーキテクチャ

OS依存の各サブシステムはトレイトとして定義される。プラットフォームプラグインが特定のOS向けの具体実装を提供する：

```rust
pub trait PlatformProvider: Send + Sync {
    fn windowing(&self) -> &dyn WindowManager;
    fn input(&self) -> &dyn InputManager;
    fn clipboard(&self) -> &dyn ClipboardManager;
    fn ime(&self) -> &dyn ImeManager;
    fn file_dialogs(&self) -> &dyn FileDialogManager;
    fn notifications(&self) -> &dyn NotificationManager;
    fn drag_drop(&self) -> &dyn DragDropManager;
    fn accessibility(&self) -> &dyn PlatformAccessibility;
    fn surface(&self) -> &dyn RenderSurface;
}
```

起動時に、elidexはコンパイルターゲットに基づいて適切なPlatformProviderを選択する：

```toml
[features]
platform-linux = ["elidex-platform-linux"]
platform-macos = ["elidex-platform-macos"]
platform-windows = ["elidex-platform-windows"]
```

これはエンジン層で使用されるのと同じCargoフィーチャーフラグメカニズムだが、Web仕様ではなくOSレベルの関心事に適用される。

## 23.2 サブシステムトレイト

### 23.2.1 ウィンドウ管理

```rust
pub trait WindowManager: Send + Sync {
    fn create_window(&mut self, config: WindowConfig) -> Result<WindowId>;
    fn close_window(&mut self, id: WindowId);
    fn resize(&mut self, id: WindowId, size: PhysicalSize);
    fn set_fullscreen(&mut self, id: WindowId, mode: FullscreenMode);
    fn set_title(&mut self, id: WindowId, title: &str);
    fn request_redraw(&mut self, id: WindowId);
    fn monitors(&self) -> Vec<MonitorInfo>;
    fn scale_factor(&self, id: WindowId) -> f64;
}
```

winitが基盤として有力で、クロスプラットフォームのウィンドウ生成とイベントループ統合を提供する。ただし、ブラウザ固有の要件（カスタムタイトルバーレンダリング、ウィンドウレベルのタブ管理等）がwinitの機能を超える場合に備え、トレイト抽象化によりwinitの置換やラップが可能。

### 23.2.2 入力

```rust
pub trait InputManager: Send + Sync {
    fn keyboard_layout(&self) -> KeyboardLayout;
    fn pointer_capabilities(&self) -> PointerCapabilities;
    fn register_global_shortcut(&mut self, shortcut: Shortcut) -> Result<ShortcutId>;
    fn cursor_position(&self) -> Option<PhysicalPosition>;
}
```

キーボード、マウス、タッチ、ペンイベントはプラットフォーム非依存のイベントストリームに正規化される。OS固有のキーボードレイアウトと入力の癖はプラットフォームプラグインが処理する。

### 23.2.3 IME（Input Method Editor）

```rust
pub trait ImeManager: Send + Sync {
    fn activate(&mut self, config: ImeConfig);
    fn deactivate(&mut self);
    fn set_cursor_area(&mut self, rect: Rect);
    fn composition_state(&self) -> Option<CompositionState>;
}
```

IME統合はCJK言語サポートに不可欠。各OSは根本的に異なるIMEプロトコルを持つ（Linux: IBus/Fcitx、macOS: Input Method Kit、Windows: TSF）。トレイトが統一インターフェースを提供し、プラットフォームプラグインがOS固有のプロトコルを処理する。これはelidex-textのCJKおよび縦書きパイプライン（第16章）を直接サポートする。

### 23.2.4 クリップボードとドラッグ&ドロップ

```rust
pub trait ClipboardManager: Send + Sync {
    fn read_text(&self) -> Result<Option<String>>;
    fn write_text(&mut self, text: &str) -> Result<()>;
    fn read_rich(&self) -> Result<Option<ClipboardContent>>;
    fn write_rich(&mut self, content: ClipboardContent) -> Result<()>;
    fn available_formats(&self) -> Vec<ClipboardFormat>;
}

pub trait DragDropManager: Send + Sync {
    fn start_drag(&mut self, data: DragData, image: Option<DragImage>) -> Result<()>;
    fn register_drop_target(&mut self, window: WindowId, handler: Box<dyn DropHandler>);
}
```

### 23.2.5 レンダーサーフェス

```rust
pub trait RenderSurface: Send + Sync {
    fn create_surface(&mut self, window: WindowId) -> Result<wgpu::Surface>;
    fn preferred_format(&self) -> wgpu::TextureFormat;
}
```

プラットフォームのウィンドウシステムをwgpuに接続し、Velloベースのレンダリングパイプライン（第15章）が描画するサーフェスを提供する。

### 23.2.6 プラットフォームアクセシビリティ

```rust
pub trait PlatformAccessibility: Send + Sync {
    fn init(&mut self, window: WindowId) -> Result<()>;
    fn update_tree(&mut self, tree: &AccessibilityTree);
    fn handle_action(&self, action: AccessibilityAction) -> Result<()>;
}
```

| プラットフォーム | API | 備考 |
| --- | --- | --- |
| Linux | AT-SPI2（atspiクレート経由） | D-Busベース。スクリーンリーダー（Orca）通信。 |
| macOS | NSAccessibility | Objective-Cブリッジ。VoiceOver通信。 |
| Windows | UI Automation (UIA) | COMベース。Narrator/JAWS/NVDA通信。 |

AccessKit（第25章）がクロスプラットフォームのアクセシビリティツリーを提供し、プラットフォームプラグインがOS固有のプロトコルに変換する。

## 23.3 elidex-appでのプラットフォーム利用

elidex-appモードでは、プラットフォーム抽象化レイヤーはelidex-browserと共有されるが、スコープが縮小される：

| サブシステム | elidex-browser | elidex-app |
| --- | --- | --- |
| ウィンドウ管理 | フル（タブ、ポップアップ、DevToolsウィンドウ） | 単一またはマルチウィンドウ（アプリ定義） |
| 入力 | フル（キーボード、マウス、タッチ、ペン） | 同一 |
| IME | フル | 同一 |
| クリップボード | フル | 同一 |
| ファイルダイアログ | フル | オプショナル（ケーパビリティゲート、第8章） |
| 通知 | フル | オプショナル（ケーパビリティゲート） |
| ドラッグ&ドロップ | フル | オプショナル |
| アクセシビリティ | フル | 同一（必須） |
| レンダーサーフェス | フル | 同一 |

同じPlatformProviderトレイトが両方のユースケースに対応する。elidex-appは単に機能のサブセットを使用し、ケーパビリティベースのセキュリティモデル（第8章）で制御される。

## 23.4 クレート構成

```
elidex-platform/
├── elidex-platform-api/       # トレイト定義（PlatformProvider、全サブシステムトレイト）
├── elidex-platform-linux/     # Linux実装（X11/Wayland、IBus/Fcitx、AT-SPI2）
├── elidex-platform-macos/     # macOS実装（Cocoa、Input Method Kit、NSAccessibility）
├── elidex-platform-windows/   # Windows実装（Win32、TSF、UIA）
└── elidex-platform-common/    # 共有ユーティリティ（イベント正規化、キーマッピングテーブル）
```
