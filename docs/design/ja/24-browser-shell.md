
# 24. ブラウザシェル

ブラウザシェルはエンジンとプラットフォーム層の上にあるすべて：タブ、ナビゲーション、アドレスバー、ブックマーク、設定、DevTools、そしてユーザーが操作するビジュアルchrome。エンジン層（Web標準がcore/compat境界を定義）と異なり、シェルには権威ある仕様がない。ここでのプラガビリティ思想は**トレイト契約ベース**：elidexが最小限の振る舞いインターフェースを定義し、実装をまるごと差し替え・カスタマイズ可能にする。

## 24.1 設計思想

| レイヤー | プラグイン根拠 | プラガビリティの方向 |
| --- | --- | --- |
| エンジン | Web標準 + SpecLevel | **削る** — レガシー機能を除去 |
| プラットフォーム | OS API + トレイト契約 | **差し替える** — OS実装を注入 |
| ブラウザシェル | トレイト契約のみ | **拡げる/差し替える** — UIとUXをカスタマイズ |

シェルはエンジンを変更することなくブラウザUI全体を差し替えられるよう設計されている。これにより以下のユースケースが可能：elidex上にブランドブラウザを構築（カスタムchrome、同一エンジン）、UIを最小化したキオスクモード、根本的に異なるインターフェースのアクセシビリティ特化ブラウザ、新しいナビゲーションパラダイムの研究ブラウザなど。

## 24.2 シェルアーキテクチャ

シェルはプラットフォーム非依存の状態/ロジック層と、プラットフォーム依存（またはフレームワーク依存）のUI層に分割される：

```
┌─────────────────────────────────┐
│  Browser Chrome (UI)            │  ← プラガブル：トレイトベース
│  タブバー、アドレスバー、        │     セルフホスト（HTML/CSS）、
│  ブックマークバー、設定、        │     ネイティブツールキット、
│  DevTools、ダウンロードシェルフ  │     またはRust GUIが選択可能
├─────────────────────────────────┤
│  Shell State Manager            │  ← プラットフォーム非依存ロジック
│  TabManager, NavigationManager, │     状態を管理し、chromeとエンジン
│  BookmarkStore, SettingsStore,  │     間を調整
│  DownloadManager, ProfileManager│
├─────────────────────────────────┤
│  プラットフォーム抽象化（第23章）│  ← OS統合
├─────────────────────────────────┤
│  エンジン（第5〜22章）          │  ← Webコンテンツレンダリング
└─────────────────────────────────┘
```

### 24.2.1 Shell State Manager

State Managerはすべてのブラウザレベルの状態を保持し、トレイト経由で公開する。プラットフォーム非依存でUIコードを含まない：

```rust
pub trait TabManager: Send + Sync {
    fn create_tab(&mut self, url: Url, position: TabPosition) -> TabId;
    fn close_tab(&mut self, id: TabId) -> Result<()>;
    fn active_tab(&self) -> TabId;
    fn set_active(&mut self, id: TabId);
    fn tabs(&self) -> &[TabInfo];
    fn move_tab(&mut self, id: TabId, position: TabPosition);
    fn duplicate_tab(&mut self, id: TabId) -> TabId;
}

pub trait NavigationManager: Send + Sync {
    fn navigate(&mut self, tab: TabId, url: Url);
    fn back(&mut self, tab: TabId) -> bool;
    fn forward(&mut self, tab: TabId) -> bool;
    fn reload(&mut self, tab: TabId);
    fn stop(&mut self, tab: TabId);
    fn history(&self, tab: TabId) -> &[HistoryEntry];
}

pub trait BookmarkStore: Send + Sync {
    fn add(&mut self, bookmark: Bookmark) -> BookmarkId;
    fn remove(&mut self, id: BookmarkId);
    fn list(&self, folder: Option<FolderId>) -> Vec<BookmarkEntry>;
    fn search(&self, query: &str) -> Vec<BookmarkEntry>;
    fn import(&mut self, format: BookmarkFormat, data: &[u8]) -> Result<usize>;
    fn export(&self, format: BookmarkFormat) -> Result<Vec<u8>>;
}

pub trait DownloadManager: Send + Sync {
    fn start(&mut self, url: Url, destination: Option<PathBuf>) -> DownloadId;
    fn pause(&mut self, id: DownloadId);
    fn resume(&mut self, id: DownloadId);
    fn cancel(&mut self, id: DownloadId);
    fn downloads(&self) -> &[DownloadInfo];
}

pub trait SettingsStore: Send + Sync {
    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    fn set<T: Serialize>(&mut self, key: &str, value: &T);
    fn reset(&mut self, key: &str);
    fn observe(&mut self, key: &str, cb: SettingsCallback) -> ObserverId;
}

pub trait ProfileManager: Send + Sync {
    fn current(&self) -> &Profile;
    fn profiles(&self) -> &[Profile];
    fn switch(&mut self, id: ProfileId);
    fn create(&mut self, name: &str) -> ProfileId;
}
```

これらのトレイトはエンジン層のSpecLevelと同じ役割を果たす：任意のブラウザシェル実装が満たすべき**契約**を定義する。デフォルトのelidex-browserがフル実装を提供するが、いずれも差し替え可能。

### 24.2.2 Browser Chrome（UIレイヤー）

ChromeレイヤーはブラウザUIをレンダリングし、ユーザーアクションをShell State Managerへの呼び出しに変換する。これはスタックの中で最も差し替えやすい部分。

elidexのchromeレンダリングアプローチは段階的な判断：

| フェーズ | Chromeアプローチ | 根拠 |
| --- | --- | --- |
| Phase 1-2 | 最小限のネイティブchrome（winit + egui/iced） | エンジンがセルフホスティングに十分成熟していない。エンジンテスト用に基本UIが必要。 |
| Phase 3+ | セルフホストオプション利用可能（elidexが自身のchromeをHTML/CSSでレンダリング） | エンジンが十分に成熟。ドッグフーディングが複雑な実世界UIでエンジンを検証。 |
| 長期 | 両方が共存。ビルド時に選択可能 | パフォーマンス重視用途にネイティブchrome。最大カスタマイズにセルフホスト。 |

Chromeトレイト：

```rust
pub trait BrowserChrome: Send + Sync {
    fn init(&mut self, state: &dyn ShellState, platform: &dyn PlatformProvider);
    fn render_frame(&mut self, state: &dyn ShellState);
    fn handle_event(&mut self, event: ChromeEvent) -> ChromeAction;
    fn layout_regions(&self) -> ChromeLayout;
}

pub struct ChromeLayout {
    pub content_area: Rect,   // Webコンテンツビューポートの配置先
    pub tab_bar: Option<Rect>,
    pub address_bar: Option<Rect>,
    pub sidebar: Option<Rect>,
}
```

これによりchromeの実装方法に関係なく、エンジンのコンポジターがWebコンテンツビューポートの配置場所を知ることができる。

## 24.3 拡張機能統合

ブラウザ拡張機能（広告ブロッカー、パスワードマネージャー等）はエンジン層（NetworkMiddleware、DOMアクセス経由）とシェル層（ツールバーボタン、サイドバー、ポップアップウィンドウ）の両方とやり取りする。シェルは拡張機能のマウントポイントを提供する：

```rust
pub trait ExtensionHost: Send + Sync {
    fn register_toolbar_button(&mut self, ext: ExtensionId, button: ToolbarButton);
    fn register_sidebar(&mut self, ext: ExtensionId, sidebar: SidebarConfig);
    fn register_context_menu(&mut self, ext: ExtensionId, items: Vec<ContextMenuItem>);
    fn register_page_action(&mut self, ext: ExtensionId, action: PageAction);
    fn open_popup(&mut self, ext: ExtensionId, url: Url, size: Size);
}
```

拡張機能APIの設計は後のフェーズに延期（Phase 4+）されるが、シェルアーキテクチャはこれらのトレイトベースのマウントポイントを提供することで最初から対応する。

## 24.4 DevTools

DevToolsはエンジンの内部状態に接続する特別なブラウザchromeコンポーネントとして実装される：

| DevToolsパネル | エンジン接続 |
| --- | --- |
| Elements | ECS DOMツリー検査、ScriptSession変更履歴 |
| Styles | CSSOM検査、computed styleクエリ |
| Console | ScriptEngine eval、ログキャプチャ |
| Network | NetworkMiddlewareパイプライン検査（第10章） |
| Performance | ECSシステムティックタイミング、レンダリングパイプラインプロファイリング |
| Sources | ScriptEngineデバッガプロトコル |

DevToolsはセルフホストchrome（エンジン自身でレンダリング）の有力候補。本質的にエンジンを徹底的に試す複雑なWebアプリケーションであるため。

## 24.5 クレート構成

```
elidex-shell/
├── elidex-shell-api/          # トレイト定義（TabManager、NavigationManager等）
├── elidex-shell-state/        # シェル状態マネージャーのデフォルト実装
├── elidex-chrome-native/      # ネイティブchrome（egui/iced、Phase 1-2）
├── elidex-chrome-selfhost/    # セルフホストchrome（HTML/CSS、Phase 3+）
├── elidex-devtools/           # DevTools実装
└── elidex-extension-host/     # 拡張機能マウントポイントとライフサイクル
```
