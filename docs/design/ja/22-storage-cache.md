
# 22. ストレージ & キャッシュアーキテクチャ

ブラウザは驚くほど大量の永続的・一時的な状態を管理する — HTTPキャッシュ、Cookie、履歴、ブックマーク、IndexedDBデータベース、Origin Private File System等。これらのシステムは所有権（ブラウザ vs Webコンテンツ）、寿命（セッション vs 永続）、隔離要件（グローバル vs オリジン単位）、性能特性（メモリ vs ディスク）が異なる。本章ではこれらすべてを支える統一ストレージアーキテクチャを定義する。

## 22.1 設計原則

**構造によるオリジン隔離。** Webコンテンツストレージはオリジンごとに物理的に分離される — ストレージ種別ごとにオリジン1つにつき1つのSQLiteデータベース。クロスオリジンデータ漏洩はクエリのバグでは発生し得ない。データが別ファイルに存在するためである。これはコードレビューで施行される慣例ではなく、構造的保証である。

**ブラウザデータは集中管理。** ブラウザ自体が所有するデータ（Cookie、HSTS、履歴、ブックマーク、設定、パーミッション）は単一の共有データベースに格納。Browser Processのみがこのデータベースにアクセスし、Rendererは直接触れない。

**あらゆる層でのトレイト抽象化。** ストレージスタックは2層構成：低レベルのStorageBackendトレイト（データベースのオープン/クローズ/マイグレーション）と高レベルのドメイントレイト（CookieStore、HistoryStore、OriginStorage）。SQLiteが初期実装だが構造的依存ではない。

**クォータとエビクションのセキュアデフォルト。** オリジンストレージはクォータ管理付きでLRUエビクション。永続ストレージは明示的なユーザー/APIの許可を要求。

## 22.2 ストレージ分類

すべての永続的・キャッシュされた状態は2つのカテゴリのいずれかに分類される：

| カテゴリ | 所有者 | 隔離 | アクセス | 例 |
| --- | --- | --- | --- | --- |
| ブラウザ所有 | Browser Process | グローバル（単一DB） | Browser Processのみ | Cookie、HSTS、履歴、ブックマーク、設定、パーミッション、証明書オーバーライド |
| Webコンテンツ所有 | オリジン | オリジン単位（個別DB/ディレクトリ） | Browser Processが仲介；RendererがIPC経由で要求 | elidex.storage（KV）、IndexedDB、Cache API（Service Worker）、OPFS、localStorage（compat） |

### 22.2.1 ファイルシステムレイアウト

```
{profile_dir}/
├── browser.sqlite              # ブラウザ所有の集中管理DB
├── browser.sqlite-wal          # WALジャーナル
├── http-cache/                 # HTTPキャッシュ（22.6節）
│   ├── index.sqlite            # キャッシュメタデータ
│   └── entries/                # キャッシュされたレスポンスボディ（内容アドレス指定）
│       ├── a1b2c3d4...
│       └── e5f6a7b8...
└── origins/                    # Webコンテンツストレージ、オリジン単位
    ├── https_example.com_443/
    │   ├── kv.sqlite           # elidex.storage
    │   ├── idb.sqlite          # IndexedDB
    │   ├── cache/              # Cache API
    │   │   ├── index.sqlite
    │   │   └── entries/
    │   └── opfs/               # Origin Private File System
    ├── https_app.example.com_443/
    │   ├── kv.sqlite
    │   └── idb.sqlite
    └── ...
```

オリジンキーは`{scheme}_{host}_{port}`としてファイルシステム安全なエスケープで導出。セキュリティモデル（第8章）のOriginタプルと一致し、Webオリジンとファイルシステムディレクトリの1:1マッピングを提供。

## 22.3 ストレージバックエンド抽象化

### 22.3.1 StorageBackendトレイト

低レベルトレイトはデータベースのライフサイクルを抽象化。SQLは公開しない — SQLiteバックエンドの実装詳細：

```rust
pub trait StorageBackend: Send + Sync {
    type Connection: StorageConnection;

    /// 指定パスでデータベースを開くまたは作成。
    fn open(&self, path: &Path, options: OpenOptions) -> Result<Self::Connection, StorageError>;

    /// スキーママイグレーションを実行しデータベースを現在のバージョンにする。
    fn migrate(&self, conn: &Self::Connection, migrations: &[Migration]) -> Result<(), StorageError>;

    /// 診断用バックエンド名。
    fn name(&self) -> &str;
}

pub trait StorageConnection: Send {
    /// 名前付き操作を実行。操作はドメイントレイトで定義され、
    /// 生SQLではない。バックエンドがネイティブクエリ言語にマッピング。
    fn execute(&self, op: &StorageOp) -> Result<StorageResult, StorageError>;

    /// トランザクション開始。
    fn transaction<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Self) -> Result<T, StorageError>;
}

pub struct OpenOptions {
    pub read_only: bool,
    pub create_if_missing: bool,
    pub wal_mode: bool,            // SQLite WAL — 非SQLiteバックエンドでは無視
    pub busy_timeout: Duration,     // ロック競合タイムアウト
}
```

### 22.3.2 SQLite実装

```rust
pub struct SqliteBackend;

impl StorageBackend for SqliteBackend {
    type Connection = SqliteConnection;  // rusqlite::Connectionをラップ

    fn open(&self, path: &Path, options: OpenOptions) -> Result<SqliteConnection, StorageError> {
        let conn = rusqlite::Connection::open(path)?;
        if options.wal_mode {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.busy_timeout(options.busy_timeout)?;
        // セキュリティ強化
        conn.pragma_update(None, "secure_delete", "ON")?;
        Ok(SqliteConnection(conn))
    }

    fn name(&self) -> &str { "sqlite" }
    // ...
}
```

統一的に適用されるSQLite固有設定：

| プラグマ | 値 | 根拠 |
| --- | --- | --- |
| journal_mode | WAL | 書き込み中の並行読み取り。ブラウザの応答性に不可欠。 |
| synchronous | NORMAL | 耐久性/性能のバランス。WALモードではNORMALで破損に安全（アプリクラッシュには耐性、OSクラッシュにのみ脆弱）。 |
| secure_delete | ON | 削除データをゼロで上書き。クリアされた閲覧データのディスクからの復元を防止。 |
| foreign_keys | ON | browser.sqliteのリレーショナルテーブルの参照整合性。 |
| busy_timeout | 5000ms | 競合時の即座のSQLITE_BUSYエラーを回避。 |
| cache_size | -8000（8MB） | 接続ごとのページキャッシュ。ストレージ種別ごとに調整可能。 |

### 22.3.3 StorageOp抽象化

トレイトを通じてSQLを漏洩させるのではなく、ドメイン操作を型付きenumで表現：

```rust
pub enum StorageOp {
    Get { table: &'static str, key: Vec<u8> },
    Put { table: &'static str, key: Vec<u8>, value: Vec<u8> },
    Delete { table: &'static str, key: Vec<u8> },
    Scan { table: &'static str, prefix: Vec<u8>, limit: usize },
    Custom(Box<dyn CustomOp>),  // 複雑なクエリ用のエスケープハッチ（IndexedDB）
}
```

これは意図的に最小限。ほとんどのドメイントレイト（CookieStore、HistoryStore）はこれらのプリミティブの上に高レベル操作を構築。IndexedDBの複雑なカーソル/インデックスクエリは`Custom`バリアントを使用し、SQLiteバックエンドがプリペアドステートメントにマッピング。

将来バックエンドが置換される場合、StorageOp→クエリのマッピングのみが変更。ドメイントレイトとその呼び出し元は影響を受けない。

## 22.4 ブラウザ所有ストレージ

### 22.4.1 browser.sqliteスキーマ

集中管理ブラウザデータベースはすべてのブラウザ所有状態を含む。Browser Processのみがこのファイルを開く：

| テーブル | 主要カラム | 備考 |
| --- | --- | --- |
| cookies | (host, path, name, partition_key) | 完全なCookie属性：value、expires、secure、httponly、samesite、creation_time、last_access_time。CHIPS用パーティションキー。 |
| hsts | (host) | include_subdomains、expiry、source（preload vs dynamic）。 |
| history | (url, visit_time) | title、visit_count、typed_count。サジェスト用のFrecencyスコア。 |
| bookmarks | (id, parent_id) | ツリー構造。title、url、position、date_added。 |
| permissions | (origin, permission_type) | granted/denied/prompt、expiry。Permissions API（OPEN-010）にマッピング。 |
| settings | (key) | ブラウザ設定。JSON値。 |
| cert_overrides | (host, port) | ユーザーが許可した証明書例外。 |
| content_prefs | (origin, key) | サイト単位の設定（ズームレベル、通知許可）。 |

### 22.4.2 ドメイントレイト（ブラウザ所有）

これらのトレイトはエンジンの他の部分が消費する公開API。いくつかは先行章で導入済みで、ここで具体的なストレージバッキングを得る：

```rust
/// Cookie永続化 — CookieJar（第10章）のバッキング
pub trait CookiePersistence: Send + Sync {
    fn load_all(&self) -> Result<Vec<Cookie>>;
    fn persist(&self, cookie: &Cookie) -> Result<()>;
    fn delete(&self, domain: &str, path: &str, name: &str) -> Result<()>;
    fn delete_expired(&self, now: SystemTime) -> Result<usize>;
    fn clear_origin(&self, origin: &Origin) -> Result<()>;
}

/// 履歴 — NavigationManager（第24章）のバッキング
pub trait HistoryStore: Send + Sync {
    fn record_visit(&self, url: &Url, title: &str, transition: TransitionType) -> Result<()>;
    fn query(&self, text: &str, limit: usize) -> Result<Vec<HistoryEntry>>;
    fn frecency_suggest(&self, prefix: &str, limit: usize) -> Result<Vec<Suggestion>>;
    fn delete_range(&self, from: SystemTime, to: SystemTime) -> Result<()>;
    fn delete_url(&self, url: &Url) -> Result<()>;
}

/// ブックマーク — BookmarkStore（第24章）のバッキング
pub trait BookmarkPersistence: Send + Sync {
    fn load_tree(&self) -> Result<BookmarkNode>;
    fn add(&self, parent_id: BookmarkId, bookmark: Bookmark) -> Result<BookmarkId>;
    fn update(&self, id: BookmarkId, changes: BookmarkUpdate) -> Result<()>;
    fn remove(&self, id: BookmarkId) -> Result<()>;
    fn move_to(&self, id: BookmarkId, new_parent: BookmarkId, position: usize) -> Result<()>;
}
```

CookieJar（第10章）はインメモリのワーキングセットを保持し、耐久性のためにCookiePersistenceを呼び出す。起動時に`load_all()`がインメモリjarを初期化。変更は非同期にライトスルー（タイマーまたはシャットダウン時のバッチ書き込み）。

### 22.4.3 Cookieパーティショニング

cookieテーブルにはCHIPS（Cookies Having Independent Partitioned State）をサポートする`partition_key`カラムが含まれる。パーティション化されたCookieモデルでは、サードパーティCookieは`(cookie-domain)`だけでなく`(top-level-site, cookie-domain)`でキーイングされる。クロスサイトトラッキングを防止しつつ正当な埋め込みユースケースを許可：

| シナリオ | partition_key | 動作 |
| --- | --- | --- |
| example.comでのファーストパーティCookie | NULL | 標準的なファーストパーティ動作 |
| news.comに埋め込まれたcdn.example.comからのサードパーティCookie | "https://news.com" | Cookieはnews.comコンテキストにスコープ。shop.com上の同じCDN Cookieは別パーティション。 |

## 22.5 Webコンテンツストレージ

### 22.5.1 OriginStorageManager

すべてのオリジン単位ストレージの中央コーディネータ。Browser Processに存在：

```rust
pub struct OriginStorageManager {
    profile_dir: PathBuf,
    backend: Box<dyn StorageBackend>,
    /// オープン接続、(origin, storage_type)でキーイング
    connections: Mutex<HashMap<(OriginKey, StorageType), Box<dyn StorageConnection>>>,
    /// クォータ追跡
    quota_manager: QuotaManager,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct OriginKey(String);  // 例: "https_example.com_443"

pub enum StorageType {
    KeyValue,       // elidex.storage
    IndexedDb,      // IndexedDB
    CacheApi,       // Service Worker Cache API
    Opfs,           // Origin Private File System
    LocalStorage,   // compat — localStorage
}

impl OriginStorageManager {
    /// 指定オリジンとストレージ種別のデータベースパスを解決。
    fn db_path(&self, origin: &OriginKey, storage_type: StorageType) -> PathBuf {
        self.profile_dir
            .join("origins")
            .join(origin.as_str())
            .join(storage_type.filename())
    }

    /// 接続を取得または開く。遅延 — データベースは初回アクセス時に作成。
    pub fn connection(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
    ) -> Result<&dyn StorageConnection, StorageError> {
        // ...
    }

    /// オリジンのすべてのストレージを削除。「Clear Site Data」で呼び出し。
    pub fn clear_origin(&self, origin: &OriginKey) -> Result<()> {
        // 接続をクローズし、オリジンディレクトリを削除
    }
}
```

### 22.5.2 Webコンテンツストレージ用IPCフロー

Rendererはオリジンデータベースに直接アクセスしない。すべてのストレージ操作はIPCを通じてBrowser Processに向かう：

```
Renderer（サイト: example.com）
  │
  │  RendererToBrowser::StorageRequest {
  │      origin: "https_example.com_443",
  │      storage_type: KeyValue,
  │      op: Put { key: "user-prefs", value: ... }
  │  }
  │
  ├──────── IPC（第5章）────────▶  Browser Process
  │                                   │
  │                                   ├─ オリジンがRendererのサイトと一致することを検証
  │                                   ├─ OriginStorageManager.connection(origin, KV)
  │                                   ├─ connection.execute(op)
  │                                   │
  │  BrowserToRenderer::StorageResponse {    │
  │      result: Ok(())                      │
  │  }                                       │
  │◀──────── IPC ────────────────────────────┘
```

Browser Processは実行前に**オリジンの主張を検証**する。異なるオリジンを主張する侵害されたRendererは拒否される。Network ProcessでのCORS施行（第10章§10.8）と同じ信頼モデル。

### 22.5.3 elidex.storage（非同期KV）

第14章で定義されたコアストレージAPI（AsyncStorageトレイト）。バッキングはシンプルなスキーマを持つオリジン単位SQLiteデータベース：

```sql
-- kv.sqlite（オリジン単位）
CREATE TABLE kv (
    key   TEXT PRIMARY KEY,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL  -- Unixエポックミリ秒
);
```

localStorageの推奨代替。非同期、ノンブロッキング、5MBサイズ制限なし（代わりにオリジンクォータで管理）。

### 22.5.4 IndexedDB

IndexedDBはオリジン単位のSQLiteデータベースでバッキングされる。IndexedDB概念からSQLiteへのマッピング：

| IndexedDB概念 | SQLiteマッピング |
| --- | --- |
| Database | メタデータテーブルでスキーマバージョンを追跡 |
| Object Store | テーブル |
| Key | PRIMARY KEY（シリアライズされたIDBキー） |
| Value | BLOB（structured cloneシリアライゼーション） |
| Index | シリアライズされたキーパス上のSQLite INDEX |
| Cursor | ORDER BYとLIMIT/OFFSETを使用したSELECT（前方/後方イテレーション） |
| Key range（IDBKeyRange） | シリアライズされたキー上の>, >=, <, <=を使用したWHERE句 |
| Transaction（readonly/readwrite） | SQLiteトランザクション（readonlyはDEFERRED、readwriteはIMMEDIATE） |

Chromeの実証済みアプローチに準拠。値のstructured cloneシリアライゼーション形式は`postMessage`（第5章IPC）と共有し、複数のシリアライゼーション形式を回避。

### 22.5.5 Cache API（Service Worker）

Cache APIはRequest/Responseペアの名前付きキャッシュを提供。オリジン単位：

```sql
-- cache/index.sqlite（オリジン単位）
CREATE TABLE caches (
    id   INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE entries (
    cache_id    INTEGER REFERENCES caches(id),
    request_url TEXT NOT NULL,
    request_method TEXT NOT NULL,
    vary_header TEXT,           -- Varyベースマッチング用
    response_status INTEGER,
    response_headers BLOB,     -- シリアライズされたヘッダマップ
    body_path   TEXT,           -- entries/内のボディファイルへの相対パス
    body_size   INTEGER,
    created_at  INTEGER,
    PRIMARY KEY (cache_id, request_url, request_method)
);
```

レスポンスボディはファイルとして格納（BLOBではない）。大きなキャッシュリソースでのSQLite肥大化を回避。

## 22.6 HTTPキャッシュ

HTTPキャッシュはWebコンテンツストレージとは別の重要な性能システム。Network Processに存在し全オリジン間で共有される（ただしプライバシー配慮のキーイング付き）。

### 22.6.1 キャッシュキー

キャッシュプローブによるクロスサイトトラッキングを防止するため、elidexはダブルキーキャッシングを使用（Chromeのパーティション戦略に準拠）：

```rust
pub struct HttpCacheKey {
    /// ナビゲーションを開始したトップレベルサイト
    top_level_site: SchemefulSite,
    /// 実際のリソースURL
    resource_url: Url,
    /// クロスサイトサブリソースかどうか（パーティショニングに影響）
    is_cross_site: bool,
}
```

| リクエストコンテキスト | キャッシュキー | 共有 |
| --- | --- | --- |
| A.comへのトップレベルナビゲーションがCDNからscript.jsを読み込み | (A.com, cdn.example.com/script.js) | 同じスクリプトを読み込むB.comとは共有されない |
| A.comへのトップレベルナビゲーションが自身のリソースを読み込み | (A.com, A.com/style.css) | A.comのページ間で共有 |

プライバシー/性能のトレードオフ。ダブルキーイングはキャッシュヒット率を低下させる（同じCDNリソースがトップレベルサイトごとに別々にキャッシュされる）が、キャッシュベースのトラッキングを排除。

### 22.6.2 キャッシュストレージ構造

```rust
pub struct HttpCache {
    /// メタデータインデックス — キャッシュキーからエントリメタデータへのマッピング
    index: HttpCacheIndex,       // SQLiteバッキング（http-cache/index.sqlite）
    /// エントリボディストレージ — 内容アドレス指定ファイル
    body_store: BodyStore,       // ファイルベース（http-cache/entries/）
    /// ホットエントリ用インメモリLRU
    memory_cache: LruCache<HttpCacheKey, CachedResponse>,
    /// ディスク予算
    max_disk_size: u64,          // デフォルト: 512MB、設定可能
    max_memory_size: u64,        // デフォルト: 64MB
}
```

メタデータ（URL、ヘッダ、有効期限、ETag、Last-Modified）は効率的なクエリのためSQLiteに格納。レスポンスボディは内容アドレス指定ファイルとしてディスクに格納。この分離によりメタデータルックアップ（キャッシュヒット確認）が完全なレスポンスボディの読み取りを必要としない。

### 22.6.3 Cache-Control準拠

| ディレクティブ | 動作 |
| --- | --- |
| max-age | 指定秒数後にエントリが期限切れ。新鮮な場合はキャッシュから提供。 |
| no-cache | エントリは格納されるが、使用のたびに再検証（条件付きGET）が必要。 |
| no-store | エントリはディスクに格納されない。ナビゲーション期間中メモリキャッシュに存在可能。 |
| must-revalidate | 古いエントリは再検証必須；古い状態では提供しない。 |
| stale-while-revalidate | 古いエントリを即座に提供しつつバックグラウンドで再検証。体感性能を改善。 |
| immutable | 新鮮な間は再検証しない。バージョン付きアセット用（例：style.a1b2c3.css）。 |
| private | 共有キャッシュに格納しない（ブラウザキャッシュには無関係だが尊重）。 |
| Vary | レスポンスがリクエストヘッダにより変化。キャッシュキーにVaryヘッダ値を含む。 |

条件付き再検証：

```
キャッシュにリソースの古いエントリがある
  → 条件付きGETを送信：
     If-None-Match: "etag-value"       （ETagが存在した場合）
     If-Modified-Since: <date>          （Last-Modifiedが存在した場合）
  → サーバーが応答：
     ├── 304 Not Modified → キャッシュボディを再利用、ヘッダ/有効期限を更新
     └── 200 OK → キャッシュエントリを新しいレスポンスで置換
```

### 22.6.4 エビクション

ディスク使用量が`max_disk_size`を超過した場合：

1. `no-cache`マーク付きまたは期限切れのエントリを優先的にエビクト
2. 残りのエントリは最終アクセス時刻によるLRU
3. エビクションはバッチ処理（書き込みごとではなく定期実行）でI/Oストームを回避

## 22.7 メモリキャッシュ

いくつかの性能重要キャッシュはRenderer Process内の完全なメモリ上に存在。永続化されずページロードまたはプロセス再起動時に再作成：

| キャッシュ | 場所 | 容量 | エビクション | 目的 |
| --- | --- | --- | --- | --- |
| **画像デコードキャッシュ** | Renderer | メモリ予算（デフォルト128MB） | デコードサイズによるLRU | スクロール/再描画時の再デコード回避のためデコード済みビットマップを格納。キー：(URL, デコードサイズ, スケールファクター) — 第18章§18.7参照。 |
| **スタイル共有キャッシュ** | Renderer | フレーム単位、約1000エントリ | スタイル再計算時にクリア | Servoに着想：同一スタイル入力を持つ兄弟要素が計算スタイルを共有。冗長なプロパティ解決を回避。 |
| **フォントグリフキャッシュ** | Renderer | フォントフェイス単位、LRU | グリフ年齢によるLRU | 特定サイズでのラスタライズ済みグリフビットマップ。GPUアトラステクスチャに供給。 |
| **バイトコードキャッシュ** | Renderer → ディスク | オリジン単位ディレクトリ | スクリプト年齢によるLRU | Boaバイトコードキャッシュ。変更のないスクリプトの再パース/再コンパイルを回避。セッション間の恩恵のためオリジンのストレージディレクトリに永続化。 |
| **DNSキャッシュ** | Network Process | インメモリ、TTLベース | TTL期限切れ | 第10章で定義（DnsCache）。DNS TTLを尊重。短いTTLでのネガティブキャッシング。 |
| **CORSプリフライトキャッシュ** | Network Process | インメモリ、(origin, method, headers)単位 | Access-Control-Max-Age期限切れ | 第10章で定義。冗長なOPTIONSリクエストを回避。 |
| **接続プール** | Network Process | 最大256接続 | アイドルタイムアウト（90秒） | 第10章で定義（ConnectionPool）。再利用のためのKeep-alive接続。 |

### 22.7.1 メモリ圧迫時の処理

OSがメモリ圧迫をシグナルした場合（またはタブ単位のメモリが予算を超過）：

| 圧迫レベル | アクション |
| --- | --- |
| 中程度 | 画像デコードキャッシュを50%に縮小。スタイル共有キャッシュをエビクト。アイドル接続を解放。 |
| 危機的 | 画像デコードキャッシュを完全にフラッシュ。非表示タブのRendererプロセスを破棄（タブ破棄 — Browser Processがアクティベーション時に再作成）。すべてのアイドル接続を解放。SQLiteデータベースをコンパクト化（`PRAGMA incremental_vacuum`）。 |

タブ破棄の優先度：最後のユーザーインタラクションによるLRU。例外：オーディオ再生中、WebRTC接続保持中、未保存フォームデータを持つタブ。

## 22.8 Back/Forwardキャッシュ（bfcache）

bfcacheはユーザーがナビゲーションで離れた際にページの完全な状態（DOM、JSヒープ、Rendererプロセス）を保存し、即座の戻る/進むナビゲーションを可能にする。

### 22.8.1 ライフサイクル

```
ユーザーがページAからページBにナビゲート：
  1. ページAが'pagehide'イベントを受信（persisted = true）
  2. ページAのRenderer状態が凍結（JS実行一時停止、タイマー中断）
  3. ページAの凍結状態がbfcacheに格納（Browser Processがキャッシュを管理）
  4. ページBが新しい（または再利用された）Rendererで通常ロード

ユーザーが戻るを押す：
  1. ページBが'pagehide'を受信（自身もbfcacheに入る可能性）
  2. ページAのRendererがbfcacheから解凍
  3. ページAが'pageshow'イベントを受信（persisted = true）
  4. 即座に表示 — ネットワークフェッチなし、パースなし、スタイル/レイアウトなし
```

### 22.8.2 適格性

すべてのページがbfcacheに入れるわけではない。失格条件：

| 条件 | 理由 |
| --- | --- |
| WebSocketまたはWebRTC接続がオープン | アクティブなネットワーク接続は凍結不可 |
| `unload`イベントリスナーが登録済み | レガシー動作がunloadの実行に依存；凍結はこれをスキップ |
| メインドキュメントに`Cache-Control: no-store` | キャッシングからの明示的オプトアウト |
| アクティブなIndexedDBトランザクション | トランザクション状態を信頼性高く凍結不可 |
| ボディストリーミング中の`fetch()` | ストリーミング状態をシリアライズ不可 |

`pagehide`（`unload`ではなく）を登録し、`SharedWorker`の代わりに`BroadcastChannel`を使用するページは設計上bfcacheフレンドリー。

### 22.8.3 キャッシュサイズ

| パラメータ | デフォルト | 備考 |
| --- | --- | --- |
| 最大キャッシュページ数 | 6 | ブラウジングセッション単位。LRUエビクション。 |
| エントリ単位の最大メモリ | 256MB | これを超えるエントリはエビクトされる。 |
| bfcache合計最大メモリ | 512MB | 全エントリで共有。 |

bfcacheエントリはメモリ圧迫時（22.7.1節）のエビクション第一候補。

## 22.9 クォータ管理

### 22.9.1 ストレージクォータモデル

各オリジンはWebコンテンツデータに対するストレージ予算を持つ：

```rust
pub struct QuotaManager {
    /// グローバルストレージ上限（デフォルト: 利用可能ディスクの80%、最大100GB）
    global_limit: u64,
    /// オリジン単位の上限（デフォルト: min(グローバルの20%, 10GB)）
    per_origin_limit: u64,
    /// 永続ストレージ許可されたオリジン（エビクション免除）
    persistent_origins: HashSet<OriginKey>,
}
```

| ストレージ種別 | クォータに算入 | 備考 |
| --- | --- | --- |
| elidex.storage（KV） | はい | |
| IndexedDB | はい | |
| Cache API | はい | |
| OPFS | はい | |
| localStorage（compat） | はい（5MBハードリミット維持） | Web互換 — localStorageは常に5MB |
| Cookie | いいえ | 別管理（Cookieあたり4KB、ブラウザ所有） |
| HTTPキャッシュ | いいえ | 独自のディスク予算で管理（22.6節） |

### 22.9.2 Web API表面

```
// navigator.storage.estimate()
{ quota: 2147483648, usage: 52428800 }   // 2GBクォータ、50MB使用

// navigator.storage.persist()
// → ユーザーにプロンプト（またはエンゲージメントヒューリスティックで自動許可）
// → 永続オリジンはストレージ圧迫時のエビクションから免除
```

### 22.9.3 圧迫時のエビクション

合計ストレージが`global_limit`に近づいた場合：

1. 非永続オリジンを特定
2. 最終アクセス時刻でソート（LRU）
3. 最も古いものから全オリジンストレージ（全ストレージ種別）をエビクト
4. ロード中の影響を受けるページに`storage`イベントで通知
5. 使用量が`global_limit`の80%を下回るまで継続

永続オリジン（`navigator.storage.persist()`が許可されたもの）は、すべての非永続オリジンがクリアされなお圧迫が残る場合にのみエビクト。

### 22.9.4 Clear-Site-Data

`Clear-Site-Data` HTTPレスポンスヘッダによりサーバーがストレージ削除を要求可能：

| ヘッダ値 | アクション |
| --- | --- |
| `"cache"` | オリジンのHTTPキャッシュエントリをクリア |
| `"cookies"` | オリジンのCookieをクリア |
| `"storage"` | オリジンの全Webコンテンツストレージをクリア |
| `"executionContexts"` | オリジンの全アクティブドキュメントをリロード |
| `"*"` | 上記すべて |

Network Processが処理し、ストレージ削除のためBrowser Processと、実行コンテキストリロードのためRendererと連携。

## 22.10 elidex-appストレージ

elidex-appモードではストレージ動作が異なる：

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| オリジンモデル | 標準Webオリジン | アプリ定義（通常は単一オリジン） |
| ストレージ場所 | プロファイルディレクトリ | アプリ定義（例：プラットフォーム標準のアプリデータディレクトリ） |
| クォータ | 管理あり、オリジン単位の制限 | デフォルトではクォータなし（アプリが自身のストレージを制御） |
| ブラウザ所有テーブル | フルセット | サブセット：設定のみ（履歴、ブックマークなし） |
| IndexedDB | 利用可能 | 利用可能 |
| elidex.storage | 利用可能 | 利用可能（推奨プライマリAPI） |
| HTTPキャッシュ | フル | 利用可能、サイズ設定可能 |

```rust
let app = elidex_app::App::new()
    .storage_dir("/path/to/app/data")
    .storage_backend(SqliteBackend::new())   // またはカスタムバックエンド
    .http_cache_size(100 * 1024 * 1024)      // 100MB HTTPキャッシュ
    .build();
```

SingleProcessモード（第5章）では、OriginStorageManagerはインプロセスサービスとして実行。IPC仲介アクセスパターンは（LocalChannelを使用して）維持され、プロセス境界がなくても同じセキュリティ検証を保持。

## 22.11 プリロードとリソースヒント

リソースヒントはネットワーク層とキャッシュ層に影響する：

| ヒント | 処理 | キャッシュとの相互作用 |
| --- | --- | --- |
| `<link rel="preconnect">` | DNS + TCP + TLSハンドシェイクを即座に開始 | キャッシュ効果なし。接続プールの恩恵。 |
| `<link rel="dns-prefetch">` | DNS解決のみ | DNSキャッシュを充填。 |
| `<link rel="preload">` | フルフェッチ、高優先度 | リソースをメモリキャッシュに格納。ページで使用されない場合はコンソール警告。 |
| `<link rel="prefetch">` | フルフェッチ、アイドル優先度 | リソースをHTTPキャッシュに格納。次のナビゲーションで使われる可能性の高いリソース用。 |
| `<link rel="prerender">` | 非表示Rendererでのフルページロード | 予測されたナビゲーションのためのbfcache的機能。 |

プリロード/プリフェッチリクエストは通常のフェッチと同じミドルウェアパイプライン（第10章）を通過し、コンテンツブロッキングとプライバシーミドルウェアが適用される。
