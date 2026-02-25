
# 9. ナビゲーション＆ページライフサイクル

## 9.1 概要

ナビゲーションはURLをレンダリング済みページに変換するエンドツーエンドのプロセス。複数プロセスにまたがり、セキュリティチェック、プロセス選択、リソース読み込み、パース、レンダリングを含む。本章は第15章（レンダリングパイプライン）、第11章（HTMLパーサー）、第10章（ネットワーク）、第13章（スクリプト）、第24章（ブラウザシェル）にまたがるナビゲーションフローを統一的に記述。

```
ユーザーアクション（URLバー、リンククリック、JSナビゲーション）
  │
  ▼
Browserプロセス: NavigationController
  ├── URL解決＆検証
  ├── セキュリティチェック（CSP、Mixed Content、HTTPSアップグレード）
  ├── リダイレクト追跡（301/302/307/308）
  ├── サイト分離：プロセス選択
  │     ├── 同一サイト → 既存Rendererを再利用
  │     └── 異サイト → 新Rendererを作成、スワップ
  ├── レスポンスヘッダ処理
  │     ├── Content-Type → ハンドラ選択（HTML、ダウンロード、PDF等）
  │     └── Cross-Originポリシー（COOP、COEP、CORP）
  │
  ▼
Rendererプロセス: DocumentLoader
  ├── Document作成（ECS World）
  ├── HTMLパーサー開始（第11章）
  │     ├── プリロードスキャナーがメインパーサーに先行
  │     ├── サブリソースフェッチ開始（CSS、JS、画像）
  │     └── スクリプト実行がパースとインターリーブ（第13章）
  ├── DOMContentLoadedイベント
  ├── サブリソース読み込み完了
  ├── loadイベント
  │
  ▼
ページがインタラクティブかつ完全にレンダリング済み
```

## 9.2 ナビゲーション種別

### 9.2.1 分類

| 種別 | トリガー | 例 |
| --- | --- | --- |
| 標準 | URLバー、`<a href>`、`window.location` | `window.location.href = "https://example.com"` |
| フォーム送信 | `<form>`送信 | `<form action="/search" method="POST">` |
| 履歴 | 戻る/進むボタン、`history.back()` | ブラウザ戻るボタン |
| リロード | リロードボタン、`location.reload()` | F5、Ctrl+R |
| 同一ドキュメント | フラグメント変更、`pushState`、Navigation API `intercept()` | `history.pushState({}, "", "/page2")` |
| 復元 | bfcache復元、タブ再開 | 戻るボタンでキャッシュページ復元 |

### 9.2.2 同一ドキュメント vs. クロスドキュメント

同一ドキュメントナビゲーションは新Documentを作成せず、完全なページロードをトリガーしない：

| メカニズム | 効果 |
| --- | --- |
| `history.pushState(state, "", url)` | URLと履歴エントリを更新。ネットワークリクエストなし。パースなし。 |
| `history.replaceState(state, "", url)` | 現在の履歴エントリを置換。 |
| `navigation.navigate(url, { history: "push" })` + `intercept()` | Navigation APIハンドラが実行。デフォルトナビゲーションなし。 |
| フラグメント変更（`#section`） | 要素にスクロール。`hashchange`イベント。 |

クロスドキュメントナビゲーションは新Document（新ECS World）を作成し、完全なパイプラインをトリガー。

## 9.3 ナビゲーションフロー：クロスドキュメント

### 9.3.1 フェーズ1：リクエスト（Browserプロセス）

```rust
pub struct NavigationController {
    /// 現在のナビゲーション状態
    active_navigations: HashMap<NavigationId, NavigationState>,
}

pub struct NavigationState {
    pub id: NavigationId,
    pub url: Url,
    pub referrer: Option<Url>,
    pub method: HttpMethod,
    pub body: Option<Bytes>,
    pub initiator: NavigationInitiator,
    pub redirect_chain: Vec<Url>,
    pub timing: NavigationTiming,
}

pub enum NavigationInitiator {
    UserTyped,
    LinkClick { source_origin: Origin },
    FormSubmission { source_origin: Origin },
    Script { source_origin: Origin },
    Reload,
    HistoryTraversal,
    Restore,
}
```

ステップ：
1. **URL検証**：相対URLを解決。スキーム検証（http、https、blob、data、about）。
2. **Navigation APIイベント**：ソースRendererで`navigate`イベントを発火。`intercept()`呼び出し時はクロスドキュメントナビゲーションを中止（同一ドキュメントに）。
3. **`beforeunload`イベント**：現在のページで発火。ユーザーがナビゲーションをキャンセル可能。
4. **セキュリティチェック**：CSP `navigate-to`、Mixed Contentブロッキング（http → httpsアップグレード）、HSTS。
5. **ネットワークリクエスト**：Network Process（第10章）に委譲。リダイレクト追跡（最大20回、ブラウザに合致）。
6. **レスポンス処理**：Content-Type確認。HTMLでなければダウンロード/PDF/画像として処理。COOP/COEPヘッダ確認。
7. **プロセス選択**：ターゲットRendererプロセスを決定（§9.4）。

### 9.3.2 フェーズ2：コミット（プロセスハンドオフ）

レスポンスヘッダがHTMLドキュメントを確認した後：

```
Browserプロセス                   旧Renderer                新Renderer
    │                                   │                         │
    ├── CommitNavigation ──────────────────────────────────────►│
    │   (url、レスポンスヘッダ、          │                         │
    │    セキュリティオリジン、CSP)       │                         │
    │                                   │                         │
    │   [クロスサイトスワップの場合]      │                         │
    ├── 旧ページをアンロード ──────────►│                         │
    │                                   ├── unloadイベント         │
    │                                   ├── クリーンアップ          │
    │◄── UnloadAck ─────────────────────┤                         │
    │                                   │                         │
    │                                   │                         ├── Document作成
    │                                   │                         ├── パース開始
    │   [レスポンスボディストリーミング]  │                         │
    ├── DataPipe ──────────────────────────────────────────────►│
    │   (共有メモリ経由HTML                │                         ├── HTML解析
    │    バイトストリーム)               │                         │
```

### 9.3.3 フェーズ3：ローディング（Rendererプロセス）

```rust
pub struct DocumentLoader {
    /// このドキュメントの新ECS World。
    world: World,
    /// ドキュメントライフサイクル状態。
    state: DocumentState,
    /// プリロードスキャナー（パーサーに先行）。
    preload_scanner: PreloadScanner,
    /// サブリソースロードトラッカー。
    pending_resources: HashSet<ResourceId>,
}

pub enum DocumentState {
    Loading,        // パーサーアクティブ、サブリソースロード中
    Interactive,    // パーサー完了、サブリソースまだロード中
    Complete,       // すべてのサブリソースロード済み
}
```

ローディングシーケンス：

```
レスポンスボディ到着（ストリーミング）
  │
  ├── 1. プリロードスキャナーがリソースURLを抽出
  │      （CSS、JS、画像、フォント — 早期フェッチを発行）
  │
  ├── 2. HTMLパーサーがDOM構築（ECSエンティティ）
  │      ├── <link rel="stylesheet"> → CSSフェッチ、レンダリングブロック
  │      ├── <script src> → JSフェッチ
  │      │     ├── defer/asyncなし：パーサーブロック、実行、再開
  │      │     ├── async：並列フェッチ、準備完了時実行
  │      │     └── defer：並列フェッチ、パース後に実行
  │      ├── <img src> → 画像フェッチ（非ブロッキング）
  │      └── <link rel="preload"> → 高優先度フェッチ
  │
  ├── 3. パーサー完了
  │      ├── document.readyState = "interactive"
  │      ├── deferredスクリプトを実行（順序通り）
  │      └── DOMContentLoadedイベント発火
  │
  ├── 4. サブリソース完了（画像、iframe等）
  │      ├── document.readyState = "complete"
  │      └── loadイベント発火（window.onload）
  │
  └── 5. ポストロード
         ├── アイドルコールバック（requestIdleCallback）
         └── レイアウト安定性（LCP、CLS確定）
```

### 9.3.4 ナビゲーションタイミング

```rust
pub struct NavigationTiming {
    pub navigation_start: Instant,
    pub redirect_start: Option<Instant>,
    pub redirect_end: Option<Instant>,
    pub fetch_start: Instant,
    pub dns_start: Option<Instant>,
    pub dns_end: Option<Instant>,
    pub connect_start: Option<Instant>,
    pub secure_connection_start: Option<Instant>,
    pub connect_end: Option<Instant>,
    pub request_start: Instant,
    pub response_start: Instant,
    pub response_end: Instant,
    pub dom_interactive: Instant,
    pub dom_content_loaded_start: Instant,
    pub dom_content_loaded_end: Instant,
    pub dom_complete: Instant,
    pub load_event_start: Instant,
    pub load_event_end: Instant,
}
```

JavaScript経由で`performance.getEntriesByType("navigation")`（PerformanceNavigationTiming）で公開。

## 9.4 プロセス選択（サイト分離）

ナビゲーションコミット時にBrowserプロセスがどのRendererプロセスが新ドキュメントを処理するか決定：

```rust
pub enum ProcessDecision {
    /// 既存Rendererを再利用（同一サイトナビゲーション）
    ReuseExisting(ProcessId),
    /// 新Rendererを作成（クロスサイトナビゲーション）
    CreateNew,
    /// 共有プロセスを使用（メモリ圧迫時）
    UseShared(ProcessId),
}

impl NavigationController {
    fn select_process(&self, target_site: &Site, current_process: Option<ProcessId>) -> ProcessDecision {
        // 同一サイト：可能なら再利用
        if let Some(pid) = current_process {
            if self.process_site(pid) == Some(target_site) {
                return ProcessDecision::ReuseExisting(pid);
            }
        }

        // クロスサイト：分離のため新プロセスが必要
        // 例外：メモリ制約モードでは共有を許可
        if self.under_memory_pressure() {
            if let Some(pid) = self.find_shared_process(target_site) {
                return ProcessDecision::UseShared(pid);
            }
        }

        ProcessDecision::CreateNew
    }
}
```

「サイト」はスキーム + eTLD+1（例：`https://example.com`）として定義。サブドメインは同じサイトを共有。

## 9.5 プリロードスキャナー

プリロードスキャナーはHTMLパーサーがブロックされている間（通常は同期スクリプトのフェッチと実行を待機中）にパーサーと並列で実行：

```rust
pub struct PreloadScanner {
    /// DOM構築なしでリソースURLを抽出する軽量トークナイザー。
    tokenizer: PreloadTokenizer,
}

impl PreloadScanner {
    /// HTMLバイトストリーム内を先読みスキャン。
    /// 早期フェッチのためのリソースヒントを返す。
    pub fn scan(&mut self, html_chunk: &[u8]) -> Vec<PreloadHint> {
        let mut hints = Vec::new();

        for token in self.tokenizer.feed(html_chunk) {
            match token {
                PreloadToken::Script { src, module, async_, defer } => {
                    hints.push(PreloadHint {
                        url: src,
                        resource_type: if module { ResourceType::ModuleScript } else { ResourceType::Script },
                        priority: if async_ || defer { Priority::Low } else { Priority::High },
                    });
                }
                PreloadToken::Stylesheet { href } => {
                    hints.push(PreloadHint {
                        url: href,
                        resource_type: ResourceType::Stylesheet,
                        priority: Priority::High,  // レンダリングブロッキング
                    });
                }
                PreloadToken::Image { src, srcset, sizes, loading } => {
                    if loading != "lazy" {
                        hints.push(PreloadHint {
                            url: src,
                            resource_type: ResourceType::Image,
                            priority: Priority::Low,
                        });
                    }
                }
                PreloadToken::Preload { href, as_ } => {
                    hints.push(PreloadHint {
                        url: href,
                        resource_type: ResourceType::from_as(as_),
                        priority: Priority::High,
                    });
                }
                PreloadToken::ModulePreload { href } => {
                    hints.push(PreloadHint {
                        url: href,
                        resource_type: ResourceType::ModuleScript,
                        priority: Priority::High,
                    });
                }
                _ => {}
            }
        }

        hints
    }
}
```

プリロードヒントは即座にNetwork Processに送信され、パーサーがスクリプト実行でブロックされている間に並列フェッチを可能に。

## 9.6 HistoryとNavigation API

### 9.6.1 セッション履歴

各ブラウジングコンテキストがセッション履歴を維持：

```rust
pub struct SessionHistory {
    /// 履歴エントリの順序付きリスト
    entries: Vec<HistoryEntry>,
    /// 現在のインデックス
    current_index: usize,
}

pub struct HistoryEntry {
    pub url: Url,
    pub title: String,
    pub state: Option<SerializedJsValue>,  // pushState/replaceStateデータ
    pub scroll_position: (f64, f64),
    /// bfcache参照（適格な場合）
    pub cached_page: Option<CachedPageRef>,
    /// Navigation APIキー（安定識別子）
    pub navigation_key: String,
    /// Navigation API id（エントリごとにユニーク）
    pub navigation_id: String,
}
```

### 9.6.2 History API（Core）

```javascript
// 新エントリをプッシュ
history.pushState({ page: 2 }, "", "/page/2");

// 現エントリを置換
history.replaceState({ page: 2, updated: true }, "", "/page/2");

// ナビゲート
history.back();
history.forward();
history.go(-2);
```

`pushState`と`replaceState`は走査時（戻る/進む）に`popstate`イベントをトリガーするが、push/replace自体ではトリガーしない。

### 9.6.3 Navigation API（Core）

Navigation APIがSPAナビゲーションのためのより構造化されたインターフェースを提供：

```javascript
// ナビゲーションをインターセプト
navigation.addEventListener("navigate", (event) => {
    if (shouldHandleClientSide(event.destination.url)) {
        event.intercept({
            handler: async () => {
                const content = await fetchContent(event.destination.url);
                renderPage(content);
            },
        });
    }
});

// プログラム的ナビゲーション
await navigation.navigate("/page/2", { state: { page: 2 } });

// 走査
await navigation.back();

// エントリにアクセス
const entries = navigation.entries();
const current = navigation.currentEntry;
```

History APIに対する主要な利点：
- `navigate`イベントがすべてのナビゲーション種別で発火（リンククリック、フォーム送信、戻る/進む、`location.href`）。
- `intercept()`がデフォルトナビゲーションをキャンセルしハンドラを実行（SPAルーティング）。
- `navigation.entries()`が完全な履歴スタックを提供（History APIは`length`のみ）。
- エントリが安定した`key`（ナビゲーションをまたいで存続）とユニークな`id`を持つ。

### 9.6.4 内部モデル

Navigation API状態はRendererプロセス内にECSリソースとして存在：

```rust
pub struct NavigationApiState {
    pub entries: Vec<NavigationEntry>,
    pub current_index: usize,
    pub transition: Option<NavigationTransition>,
}

pub struct NavigationEntry {
    pub key: String,
    pub id: String,
    pub url: Url,
    pub state: Option<SerializedJsValue>,
    pub same_document: bool,
}
```

Browserプロセスがクロスドキュメントナビゲーションをコミットする際、更新されたエントリリストを新Rendererに送信。同一ドキュメントナビゲーション（intercept）では、Rendererが自身の状態を更新。

## 9.7 Back/Forwardキャッシュ（bfcache）

### 9.7.1 設計

bfcacheは即時の戻る/進むナビゲーションのためにページ全体をメモリに保存。ユーザーが離脱時、ページを破棄する代わりにRendererプロセスを凍結し状態を保存。

```rust
pub struct BfCache {
    /// キャッシュページ、履歴エントリでキー付け
    entries: VecDeque<BfCacheEntry>,
    /// 最大エントリ数（デフォルト：6）
    max_entries: usize,
    /// 合計メモリ予算
    memory_budget: usize,
}

pub struct BfCacheEntry {
    pub history_entry_id: String,
    pub renderer_process_id: ProcessId,
    pub url: Url,
    pub timestamp: Instant,
    pub estimated_memory: usize,
}
```

### 9.7.2 凍結 / 復帰

```
[ナビゲーション離脱：ページがbfcache適格に]
  1. `pagehide`イベント発火（persisted = true）
  2. `freeze`イベント発火
  3. すべてのタイマーを停止（setTimeout、setInterval、requestAnimationFrame）
  4. すべてのネットワークリクエストを停止
  5. WebSocket/WebRTC接続を閉じ（オープンの場合は不適格）
  6. メディア再生を一時停止
  7. Rendererプロセスが凍結状態に（イベントループ停止）

[戻る/進む：bfcacheから復元]
  1. Rendererプロセスを解凍（イベントループ再開）
  2. `resume`イベント発火
  3. `pageshow`イベント発火（persisted = true）
  4. タイマー、メディア、アニメーションを再開
  5. 必要なネットワーク接続を再確立
```

### 9.7.3 適格性

すべてのページがbfcacheに適格とは限らない。不適格条件：

| 条件 | 理由 |
| --- | --- |
| オープンWebSocket | 双方向接続を凍結不可 |
| アクティブWebRTCピア接続 | メディアストリームを凍結不可 |
| `unload`イベントリスナー | 仕様非互換（復元時`unload`が発火しない） |
| `Cache-Control: no-store` | ページがキャッシュ不可を要求 |
| `window.opener`参照 | クロスウィンドウ依存 |
| アクティブIndexedDBトランザクション | トランザクション途中の凍結不可 |
| 保留中`SharedWorker`メッセージ | クロスページ共有状態 |
| リスナー付き`BroadcastChannel` | クロスページ通信 |
| 未解決`beforeunload` | ユーザー意図が曖昧 |

ページが不適格の場合、ナビゲーション時にRendererプロセスは通常通り破棄。

### 9.7.4 退去

```rust
impl BfCache {
    fn evict_if_needed(&mut self) {
        // 上限超過時に最古のエントリを退去
        while self.entries.len() > self.max_entries
            || self.total_memory() > self.memory_budget
        {
            if let Some(entry) = self.entries.pop_front() {
                // 凍結されたRendererプロセスを破棄
                self.destroy_process(entry.renderer_process_id);
            }
        }
    }
}
```

デフォルト上限：6エントリ、合計512 MB。ブラウザ設定で設定可能。メモリ圧迫時、bfcacheエントリはアクティブタブより先に退去。

## 9.8 ページライフサイクルイベント

標準クロスドキュメントナビゲーションの完全なイベントシーケンス：

```
[前のページ]
  ├── beforeunload（キャンセル可能）
  ├── pagehide
  │     ├── persisted=true → bfcacheに入る
  │     └── persisted=false → 破棄される
  ├── （bfcacheの場合）freeze
  └── （bfcacheでない場合）unload → 破棄

[新ページローディング]
  ├── DOMContentLoaded（パース完了、deferredスクリプト完了）
  ├── load（すべてのサブリソースロード済み）
  ├── pageshow（新ロードでpersisted=false）
  └── （アイドル）requestIdleCallback

[bfcacheから復元]
  ├── resume
  └── pageshow（persisted=true）
```

### 9.8.1 ページ可視性

`document.visibilityState`と`visibilitychange`イベント：

| 状態 | 条件 |
| --- | --- |
| `visible` | タブがフォアグラウンド、ウィンドウが最小化されていない |
| `hidden` | タブがバックグラウンドまたはウィンドウが最小化 |

可視性変更の影響：タイマースロットリング（バックグラウンドタブ）、rAF停止、メディア自動再生ポリシー、FramePolicy（第15章：バックグラウンドタブはOnDemandに移行可能）。

### 9.8.2 ページライフサイクル状態（拡張）

```
         ┌──────────────────────┐
         │       Active         │（可視、インタラクティブ）
         └──────┬───────────────┘
                │ タブ非表示
                ▼
         ┌──────────────────────┐
         │       Hidden         │（バックグラウンド、スロットル）
         └──────┬───────────────┘
                │ ユーザーが離脱
                ▼
         ┌──────────────────────┐
  ┌──────│       Frozen         │（bfcache、実行なし）
  │      └──────┬───────────────┘
  │             │ 退去または不適格
  │             ▼
  │      ┌──────────────────────┐
  │      │      Discarded       │（プロセス破棄）
  │      └──────────────────────┘
  │
  └──── 戻る/進む → Active
```

## 9.9 リダイレクト

```rust
impl NavigationController {
    fn handle_redirect(&mut self, nav_id: NavigationId, response: &Response) -> NavigationAction {
        let status = response.status();

        if status.is_redirect() {
            let location = response.headers().get("location");

            self.state(nav_id).redirect_chain.push(self.state(nav_id).url.clone());

            if self.state(nav_id).redirect_chain.len() > 20 {
                return NavigationAction::Error(NavigationError::TooManyRedirects);
            }

            let new_url = resolve_url(location, &self.state(nav_id).url);

            // 301/302: メソッドがGETに変更される可能性（ブラウザ互換）
            // 307/308: 元のメソッドを保持
            let method = match status.as_u16() {
                301 | 302 => HttpMethod::GET,
                307 | 308 => self.state(nav_id).method,
                _ => HttpMethod::GET,
            };

            self.state(nav_id).url = new_url;
            self.state(nav_id).method = method;

            NavigationAction::Redirect
        } else {
            NavigationAction::Commit
        }
    }
}
```

追加リダイレクト種別：
- **Metaリフレッシュ**：`<meta http-equiv="refresh" content="5;url=...">`  — パース後にRendererが処理し、Browserプロセスにナビゲーションリクエストを送信。
- **JavaScript**：`window.location.href = "..."` — Rendererからの標準ナビゲーションフローをトリガー。

## 9.10 エラーページ

| エラー | ハンドリング |
| --- | --- |
| DNS失敗 | ブラウザ生成エラーページ |
| 接続タイムアウト | ブラウザ生成エラーページ |
| TLS証明書エラー | インタースティシャル警告（非HSTSでは続行オプション） |
| HTTP 4xx/5xx | サーバーレスポンスをレンダリング（サーバーがエラーページを制御） |
| ネットワークオフライン | ブラウザ生成オフラインページ（ServiceWorkerがインターセプト可能） |
| CSP/CORSブロック | コンソールエラー、リソース未ロード（ページは部分的にレンダリング可能） |

エラーページはWebコンテンツではなくBrowserShell（第24章）がレンダリング。

## 9.11 elidex-appナビゲーション

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| URLバーナビゲーション | あり | なし（アプリがナビゲーションを制御） |
| リンククリック | 標準ナビゲーション | 設定可能：許可、ブロック、またはインターセプト |
| History API | フルサポート | フルサポート |
| Navigation API | フルサポート | フルサポート |
| bfcache | 有効 | 無効（シングルページアプリモデル） |
| プリロードスキャナー | 有効 | 有効 |
| プロセス選択 | サイト分離 | SingleProcess（デフォルト） |
| エラーページ | ブラウザ生成 | アプリ制御 |

elidex-appでは、エンベッダーが`NavigationPolicy`フック（第26章）経由でナビゲーションポリシーを制御。アプリはすべてのナビゲーションリクエストをインターセプトし、許可、ブロック、またはネイティブで処理するかを決定可能。
