
# 21. File API & Streams

## 21.1 概要

ファイルおよびストリーミングAPIはモダンWebアプリケーションの基盤層：大容量ファイルの取得と処理、メディアストリーミング、ブラウザ内データ圧縮、WebAssemblyアプリケーション向け高性能ローカルストレージ。Elidexは相互接続された3つのAPI面を対処：

1. **File/Blob API**：不変バイナリデータオブジェクト、Blob URL、ファイル読み取り。
2. **Streams API**：バックプレッシャー付きインクリメンタル処理のためのReadableStream、WritableStream、TransformStream。
3. **File System Access API**：ピッカーベースのユーザーローカルファイルシステムアクセス、およびサンドボックス化されたオリジン単位ストレージのOrigin Private File System (OPFS)。

```
┌─────────────────────────────────────────────────────────────┐
│ Rendererプロセス（サンドボックス）                            │
│                                                             │
│  ┌─────────────┐   ┌────────────┐   ┌───────────────────┐  │
│  │ ScriptSession│   │ ByteStream │   │ SyncAccessHandle  │  │
│  │ (JS API)     │◄──│ (Rust)     │   │ (共有メモリ)       │  │
│  │              │   │            │   │                   │  │
│  │ ReadableStream│  │ fetch body │   │ Worker専用同期     │  │
│  │ WritableStream│  │ file read  │   │ mmapped領域上の    │  │
│  │ Blob         │   │ compress   │   │ read/write        │  │
│  └──────┬───────┘   └─────┬──────┘   └────────┬──────────┘  │
│         │                 │                    │             │
│─────────┼─────────────────┼────────────────────┼─────────────│
│         │ IPC             │ IPC                │ shared mem  │
└─────────┼─────────────────┼────────────────────┼─────────────┘
          ▼                 ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│ Browserプロセス                                              │
│                                                             │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐ │
│  │ BlobStore   │   │ ファイルI/O   │   │ OPFSストレージ    │ │
│  │ (メモリ +    │   │ (ネイティブfs)│   │ (オリジン単位dir) │ │
│  │  ディスク    │   │              │   │                  │ │
│  │  スピル)     │   │              │   │                  │ │
│  └─────────────┘   └──────────────┘   └──────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 21.2 BlobとFile

### 21.2.1 Blobモデル

BlobはMIMEタイプ付きの不変バイト列を表す。ElidexではBlobはBrowserプロセス内のBlobStoreで管理：

```rust
pub struct BlobId(u64);

pub struct BlobMetadata {
    pub id: BlobId,
    pub size: u64,
    pub mime_type: String,
    pub backing: BlobBacking,
}

pub enum BlobBacking {
    /// 小Blob（≤256 KB）：メモリ上に保持
    Inline(Bytes),
    /// 大Blob（>256 KB）：ディスク上の一時ファイルにスピル
    DiskSpill {
        path: PathBuf,
        offset: u64,
        length: u64,
    },
    /// 他のBlobのスライス（遅延、コピーなし）
    Slice {
        parent: BlobId,
        offset: u64,
        length: u64,
    },
    /// 複数Blobの結合（遅延、コピーなし）
    Concat(Vec<BlobId>),
}
```

### 21.2.2 BlobStore

BlobStoreはBrowserプロセスに存在。Rendererプロセスはサンドボックス化されており直接ファイルI/Oを実行不可：

```rust
pub struct BlobStore {
    blobs: HashMap<BlobId, BlobMetadata>,
    next_id: AtomicU64,
    /// ディスクバックのBlobの格納先
    spill_dir: PathBuf,
    /// インラインBlobの合計メモリ使用量
    memory_usage: AtomicU64,
    /// ディスクスピルの閾値（デフォルト：Blobあたり256 KB）
    spill_threshold: u64,
    /// インラインBlobの合計メモリ予算（デフォルト：64 MB）
    memory_budget: u64,
}

impl BlobStore {
    /// データから新しいBlobを作成。
    pub fn create(&mut self, data: Bytes, mime_type: &str) -> BlobId {
        let id = BlobId(self.next_id.fetch_add(1, Ordering::Relaxed));

        let backing = if data.len() as u64 <= self.spill_threshold {
            self.memory_usage.fetch_add(data.len() as u64, Ordering::Relaxed);
            BlobBacking::Inline(data)
        } else {
            let path = self.spill_to_disk(&id, &data);
            BlobBacking::DiskSpill { path, offset: 0, length: data.len() as u64 }
        };

        self.blobs.insert(id, BlobMetadata { id, size: data.len() as u64, mime_type: mime_type.to_string(), backing });
        id
    }

    /// Blobスライスを作成（ゼロコピー）。
    pub fn slice(&mut self, parent: BlobId, start: u64, end: u64, mime_type: &str) -> BlobId {
        let id = BlobId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let length = end - start;
        self.blobs.insert(id, BlobMetadata {
            id,
            size: length,
            mime_type: mime_type.to_string(),
            backing: BlobBacking::Slice { parent, offset: start, length },
        });
        id
    }

    /// Blobデータを読み取り。スライス/結合を解決、必要に応じディスクから読み取り。
    pub async fn read(&self, id: BlobId, range: Range<u64>) -> Result<Bytes, BlobError> {
        let meta = self.blobs.get(&id).ok_or(BlobError::NotFound)?;
        self.read_backing(&meta.backing, range).await
    }

    /// ストリーミング消費用にBlobデータをByteStreamとして読み取り。
    pub fn read_stream(&self, id: BlobId, chunk_size: usize) -> ByteStream {
        // Blobをチャンク単位で読み取る非同期ストリームを返す
        // ディスクバックのBlob：チャンクごとに非同期ファイル読み取り
        // メモリバックのBlob：チャンクごとにスライス
        ByteStream::from_blob(self, id, chunk_size)
    }
}
```

### 21.2.3 Blob URL

`URL.createObjectURL(blob)`が`<img>`、`<a>`、fetch等で使用可能な`blob:` URLを作成。Blob URLレジストリはBlobStoreに存在：

```rust
pub struct BlobUrlRegistry {
    /// blob: URLからBlobIdへのマッピング。オリジン単位。
    urls: HashMap<(Origin, String), BlobId>,
}

impl BlobUrlRegistry {
    pub fn create_url(&mut self, origin: &Origin, blob_id: BlobId) -> String {
        let uuid = Uuid::new_v4();
        let url = format!("blob:{}/{}", origin, uuid);
        self.urls.insert((origin.clone(), url.clone()), blob_id);
        url
    }

    pub fn revoke_url(&mut self, origin: &Origin, url: &str) {
        self.urls.remove(&(origin.clone(), url.to_string()));
    }

    pub fn resolve(&self, origin: &Origin, url: &str) -> Option<BlobId> {
        self.urls.get(&(origin.clone(), url.to_string())).copied()
    }
}
```

fetchまたは画像読み込みが`blob:` URLに遭遇した場合、Browser Process がBlobStore経由で解決し（BlobStoreはBrowser Process所有、§21.2.2参照）レスポンスボディとしてデータを返す。これは第18章§18.9で既に参照済み。

### 21.2.4 File

`File`は追加メタデータ（name、lastModified）を持つBlob。ユーザーが`<input type="file">`またはFile System Access API経由でファイルを選択した時に作成：

```rust
pub struct FileMetadata {
    pub blob_id: BlobId,
    pub name: String,
    pub last_modified: u64,   // エポックからのミリ秒
}
```

ファイル内容は選択時にBlobStoreに読み込まれる。大ファイルについてはBlobStoreのディスクスピルにより完全にメモリ上に保持されない。

### 21.2.5 FileReader — Compat

FileReaderはPromiseベースの代替で置き換えられたイベントベースAPI（`onload`、`onerror`、`onprogress`）：

| レガシー（Compat） | モダン（Core） |
| --- | --- |
| `FileReader.readAsText(blob)` | `await blob.text()` |
| `FileReader.readAsArrayBuffer(blob)` | `await blob.arrayBuffer()` |
| `FileReader.readAsDataURL(blob)` | `URL.createObjectURL(blob)` |
| `FileReader.readAsBinaryString(blob)` | `await blob.arrayBuffer()` + view |

elidex-compatではFileReaderをモダンBlobメソッドのラッパーとして実装：

```rust
// elidex-compat: FileReader実装
pub struct FileReaderCompat {
    // 内部的にblob.stream() / blob.arrayBuffer()をラップ
    // チャンク境界でprogressイベントを発行
    // Promise完了をload/errorイベントに変換
}
```

コアエンジンは`blob.text()`、`blob.arrayBuffer()`、`blob.stream()`のみ実装。FileReaderは純粋にcompatシム。

## 21.3 Streams API

### 21.3.1 ByteStream — 内部表現

Streams APIはJavaScriptにReadableStream / WritableStream / TransformStreamとして公開。内部的にelidexはストリーミングデータを`ByteStream`（Rust非同期ストリーム）として表現：

```rust
use futures::Stream;

/// バックプレッシャーサポート付き内部バイトストリーム。
/// エンジン内のストリーミングデータの単位。
pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>,
}

impl ByteStream {
    /// 非同期リーダーから作成（ファイル、ネットワークソケット等）
    pub fn from_async_read<R: AsyncRead + Send + 'static>(reader: R, chunk_size: usize) -> Self {
        let stream = ReaderStream::with_capacity(reader, chunk_size);
        Self { inner: Box::pin(stream) }
    }

    /// BlobStore内のBlobから作成
    pub fn from_blob(store: &BlobStore, id: BlobId, chunk_size: usize) -> Self {
        // 要求に応じてBlobチャンクを読み取る非同期ストリーム
        // ...
    }

    /// インメモリバッファから作成
    pub fn from_bytes(data: Bytes) -> Self {
        Self { inner: Box::pin(futures::stream::once(async move { Ok(data) })) }
    }

    /// 変換を通してパイプ（圧縮、展開等）
    pub fn pipe_through<T: StreamTransform>(self, transform: T) -> Self {
        Self { inner: Box::pin(transform.transform(self.inner)) }
    }
}
```

### 21.3.2 JSブリッジ

ScriptSessionがRust `ByteStream`とJavaScript `ReadableStream`間をブリッジ：

```
┌─────────────────────────────────────────────┐
│ JavaScript                                  │
│                                             │
│  const response = await fetch(url);         │
│  const reader = response.body.getReader();  │
│  while (true) {                             │
│    const { done, value } = await reader.read();  // ─── pull ──┐
│    if (done) break;                         │                   │
│    process(value);                           │                   │
│  }                                          │                   │
│                                             │                   │
├─────────────────────────────────────────────┤                   │
│ ScriptSession (Rust ↔ JS ブリッジ)          │                   │
│                                             │                   │
│  ReadableStreamSource {                     │                   │
│    byte_stream: ByteStream,                 │ ◄─── pullプロトコル│
│    pull() → byte_streamから次のチャンク      │                   │
│    cancel() → byte_streamをdrop             │                   │
│  }                                          │                   │
├─────────────────────────────────────────────┤
│ ネイティブ（Rust）                           │
│                                             │
│  ByteStream（fetchレスポンスボディ）          │
│    → Network Processからチャンク到着         │
│    → データがない時はasync yield             │
│       （バックプレッシャー：ソケット読み取り停止）│
└─────────────────────────────────────────────┘
```

pullプロトコルが自然なバックプレッシャーを提供：JavaScriptが`reader.read()`呼び出しを停止すると、ByteStreamはチャンク生成を停止し、それがネットワークソケット（TCPフロー制御）またはファイル読み取りに伝播。

### 21.3.3 ReadableStream

elidexのReadableStreamは以下でバッキング可能：

| ソース | バッキング | 備考 |
| --- | --- | --- |
| Fetchレスポンスボディ | ネットワークからのByteStream | 第10章 |
| `blob.stream()` | BlobStoreからのByteStream | §21.2.2 |
| OPFSファイル読み取り | ファイルからのByteStream | §21.5 |
| `new ReadableStream({ pull() })` | JS駆動 | ユーザー作成ストリーム |
| 圧縮ストリーム | 変換を通したByteStream | §21.4 |

ネイティブバッキングのストリームでは、アプリケーションが実際にチャンクを読み取るまで、データパス全体がJS境界を越えずRust内に留まる：

```
[Rust-to-Rustファストパス]
  fetchレスポンスボディ（ByteStream）
    → pipe_through(DecompressionTransform)  // Rustでgzip展開
    → OPFSに書き込み（SyncAccessHandle）     // 直接ファイル書き込み

  JS関与なし。最大スループット。
```

### 21.3.4 WritableStream

WritableStreamはシンク側。ネイティブバッキングのシンク：

| シンク | バッキング | 備考 |
| --- | --- | --- |
| Fetchリクエストボディ | ネットワークへのByteStream | ストリーミングアップロード |
| OPFSファイル書き込み | ファイルへのByteStream | §21.5 |
| `new WritableStream({ write() })` | JS駆動 | ユーザー作成シンク |

### 21.3.5 TransformStream

TransformStreamがWritableStream（入力）をReadableStream（出力）に接続し、変換を通してデータを処理：

```rust
pub trait StreamTransform: Send + 'static {
    fn transform(
        self,
        input: Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>;
}
```

組み込み変換はネイティブStreamTransformとして実装（§21.4参照）。ユーザー作成TransformStreamはJavaScriptで変換ロジックを実行。

### 21.3.6 バックプレッシャー

バックプレッシャーはStreams APIの基本。Elidexのバックプレッシャーモデル：

```
プロデューサー               コンシューマー
   │                           │
   ├── チャンク生成 ──────────►│ highWaterMark未到達
   ├── チャンク生成 ──────────►│ highWaterMark到達
   │◄── desiredSize ≤ 0 ──────┤（バックプレッシャー信号）
   │   （プロデューサー一時停止）│
   │                           ├── チャンク消費
   │                           ├── チャンク消費
   │◄── desiredSize > 0 ──────┤（バックプレッシャー解除）
   ├── チャンク生成 ──────────►│
```

ネイティブByteStreamではバックプレッシャーは暗黙的：Rust async Streamトレイトはコンシューマーがポールしていない時に自然にyield。JSバッキングのストリームではScriptSessionが`desiredSize`を追跡し、基盤ソースに一時停止/再開を通知。

デフォルトハイウォーターマーク：

| ストリームタイプ | デフォルトハイウォーターマーク |
| --- | --- |
| バイトストリーム | 65,536バイト（64 KB） |
| オブジェクトストリーム | 1チャンク |

## 21.4 圧縮ストリーム

Compression Streams APIがブラウザ内圧縮・展開を提供：

```rust
pub struct CompressionTransform {
    algorithm: CompressionAlgorithm,
    direction: Direction,
}

pub enum CompressionAlgorithm {
    Gzip,
    Deflate,
    DeflateRaw,
}

pub enum Direction {
    Compress,
    Decompress,
}

impl StreamTransform for CompressionTransform {
    fn transform(
        self,
        input: Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>> {
        match (self.algorithm, self.direction) {
            (CompressionAlgorithm::Gzip, Direction::Compress) => {
                // flate2またはminiz_oxideのGzEncoderで入力ストリームをラップ
                Box::pin(GzipCompressStream::new(input))
            }
            (CompressionAlgorithm::Gzip, Direction::Decompress) => {
                Box::pin(GzipDecompressStream::new(input))
            }
            // ... deflate、deflate-rawバリアント
        }
    }
}
```

JavaScriptでの使用：
```javascript
const compressed = readableStream.pipeThrough(new CompressionStream("gzip"));
const decompressed = compressed.pipeThrough(new DecompressionStream("gzip"));
```

Rustクレート（flate2 / miniz_oxide）を使用したネイティブStreamTransformとして実装し、圧縮でのJSオーバーヘッドを回避。

## 21.5 File System Access API

### 21.5.1 ピッカーベースアクセス

`showOpenFilePicker()`、`showSaveFilePicker()`、`showDirectoryPicker()`がユーザー仲介のローカルファイルシステムアクセスを提供：

```
JavaScript                 Browserプロセス              プラットフォーム
    │                          │                          │
    ├─ showOpenFilePicker() ──►│                          │
    │                          ├─ Permission確認 ────────►│
    │                          │  (FileSystemAccess、     │
    │                          │   第8章§8.3)              │
    │                          │                          │
    │                          ├─ PlatformFileDialog ────►│
    │                          │  (第23章)                 │ ネイティブダイアログ
    │                          │                          ├── ユーザーが選択
    │                          │◄─────── ファイルパス ─────┤
    │                          │                          │
    │                          ├─ FileHandle作成 ─────────┤
    │                          │  (BlobStoreにBlob        │
    │◄── FileSystemFileHandle ─┤   読み込み)               │
    │                          │                          │
```

### 21.5.2 FileSystemFileHandle

```rust
pub struct FileSystemFileHandle {
    /// BrowserプロセスハンドルID
    pub handle_id: HandleId,
    /// ファイル名
    pub name: String,
    /// このハンドルのパーミッション状態
    pub read_permission: PermissionState,
    pub write_permission: PermissionState,
}

impl FileSystemFileHandle {
    /// Fileオブジェクトを取得（BlobStoreに読み込み）
    pub async fn get_file(&self) -> Result<FileMetadata, FileError> {
        // Browserプロセスへの IPC → ファイル読み取り → Blob作成 → File返却
    }

    /// このファイルの書き込み可能ストリームを作成
    pub async fn create_writable(&self) -> Result<FileSystemWritableFileStream, FileError> {
        // BrowserプロセスへのIPC → 書き込み用にファイルをオープン
        // ネイティブファイルI/OバッキングのWritableStreamを返す
    }
}
```

### 21.5.3 FileSystemDirectoryHandle

```rust
pub struct FileSystemDirectoryHandle {
    pub handle_id: HandleId,
    pub name: String,
}

impl FileSystemDirectoryHandle {
    /// このディレクトリのエントリを列挙
    pub async fn entries(&self) -> Result<Vec<FileSystemEntry>, FileError> {
        // BrowserプロセスへのIPC → readdir
    }

    /// 名前でファイルハンドルを取得
    pub async fn get_file_handle(&self, name: &str, create: bool) -> Result<FileSystemFileHandle, FileError> {
        // BrowserプロセスへのIPC
    }

    /// 名前でサブディレクトリハンドルを取得
    pub async fn get_directory_handle(&self, name: &str, create: bool) -> Result<FileSystemDirectoryHandle, FileError> {
        // BrowserプロセスへのIPC
    }

    /// エントリを削除
    pub async fn remove_entry(&self, name: &str, recursive: bool) -> Result<(), FileError> {
        // BrowserプロセスへのIPC
    }
}

pub enum FileSystemEntry {
    File(FileSystemFileHandle),
    Directory(FileSystemDirectoryHandle),
}
```

### 21.5.4 パーミッションモデル

File System Accessはパーミッションモデル（第8章§8.3）と統合：

- **読み取りパーミッション**：ユーザーがピッカー経由でファイル/ディレクトリを選択した時に暗黙的に付与。
- **書き込みパーミッション**：ユーザー選択ファイルへの最初の書き込み時に明示的なプロンプトが必要。
- **永続パーミッション**：ハンドルをIndexedDB（第22章）に保存し次セッションで再検証可能。パーミッション失効時にブラウザが再プロンプト。

elidex-appでは`App::grant(FileRead(PathPattern))`と`App::grant(FileWrite(PathPattern))`がビルド時付与を提供（第8章§8.8）。

## 21.6 Origin Private File System (OPFS)

### 21.6.1 概要

OPFSはユーザーのディスク上の可視ファイルに対応しない、サンドボックス化されたオリジン単位ファイルシステムを提供。WebAssemblyアプリケーション（SQLite-on-web、Figma等）向けの推奨高性能ストレージメカニズム。

```rust
/// 指定オリジンのOPFSルート
pub struct OpfsRoot {
    pub origin: Origin,
    /// ディスク上の実際のストレージ場所
    /// 例：~/.local/share/elidex/opfs/<origin_hash>/
    pub base_path: PathBuf,
}
```

アクセスは`navigator.storage.getDirectory()`経由。OPFSルートを指すFileSystemDirectoryHandleを返す。

### 21.6.2 非同期アクセス（メインスレッド / Worker）

標準の`FileSystemFileHandle`メソッド（`getFile()`、`createWritable()`）がOPFSファイルでも動作。BrowserプロセスへのIPC経由で、ローカルファイルシステムのファイルと同様。散発的な読み書きに適合。

### 21.6.3 SyncAccessHandle（Worker専用）

パフォーマンスクリティカルなAPI。`FileSystemSyncAccessHandle`がOPFSファイルでの同期read/write/truncate/flushを提供。ただし専用Worker内のみ（メインスレッドではUI ブロッキング回避のため不可）。

**設計：共有メモリマップドファイル**

```
┌─────────────────────────────────────────────┐
│ Workerスレッド（Rendererプロセス）            │
│                                             │
│  SyncAccessHandle                           │
│    ├── read(buffer, offset)  ──► memcpy     │
│    ├── write(buffer, offset) ──► memcpy     │
│    ├── truncate(size)        ──► IPC        │
│    ├── flush()               ──► IPC        │
│    └── close()               ──► IPC        │
│                                    │        │
│         共有メモリ領域 ◄───────────┘        │
│         （mmappedファイル）                   │
│                                             │
├─────────────────────────────────────────────┤
│ Browserプロセス                              │
│                                             │
│  OpfsFileManager                            │
│    ├── ファイルオープン                       │
│    ├── 共有メモリにmmap                       │
│    ├── Workerとマッピング共有                 │
│    ├── truncate/flush/close IPCを処理        │
│    └── 排他ロックを強制                       │
└─────────────────────────────────────────────┘
```

```rust
pub struct SyncAccessHandle {
    /// OPFSファイルにマップされた共有メモリ領域
    mapping: SharedMemoryMapping,
    /// 現在のファイルサイズ
    size: AtomicU64,
    /// 制御操作用IPCチャネル（truncate、flush、close）
    control: IpcSender<OpfsControlMsg>,
}

impl SyncAccessHandle {
    /// 同期読み取り。IPCなし — 直接メモリアクセス。
    pub fn read(&self, buffer: &mut [u8], offset: u64) -> Result<usize, FileError> {
        let file_size = self.size.load(Ordering::Acquire);
        if offset >= file_size {
            return Ok(0);
        }
        let available = (file_size - offset) as usize;
        let to_read = buffer.len().min(available);
        buffer[..to_read].copy_from_slice(&self.mapping.as_slice()[offset as usize..][..to_read]);
        Ok(to_read)
    }

    /// 同期書き込み。IPCなし — 直接メモリアクセス。
    pub fn write(&self, buffer: &[u8], offset: u64) -> Result<usize, FileError> {
        let end = offset + buffer.len() as u64;
        if end > self.mapping.capacity() {
            // マッピング拡張が必要 — BrowserプロセスへのIPC
            self.control.send(OpfsControlMsg::ExtendMapping { new_size: end })?;
            // マッピング拡張完了までブロック
        }
        self.mapping.as_mut_slice()[offset as usize..][..buffer.len()].copy_from_slice(buffer);
        self.size.fetch_max(end, Ordering::Release);
        Ok(buffer.len())
    }

    /// ディスクにフラッシュ。IPCが必要。
    pub fn flush(&self) -> Result<(), FileError> {
        self.control.send(OpfsControlMsg::Flush)?;
        self.control.recv_ack()?;
        Ok(())
    }

    /// ファイルを切り詰め。IPCが必要。
    pub fn truncate(&self, size: u64) -> Result<(), FileError> {
        self.control.send(OpfsControlMsg::Truncate { size })?;
        self.control.recv_ack()?;
        self.size.store(size, Ordering::Release);
        Ok(())
    }

    /// ハンドルを閉じる。排他ロックを解放。
    pub fn close(self) -> Result<(), FileError> {
        self.control.send(OpfsControlMsg::Close)?;
        Ok(())
    }
}
```

この設計はread/write（ホットパス）のIPC往復を排除し、毎秒数千回の小さなread/writeを実行するSQLiteに適合。制御操作（truncate、flush、close）のみIPCが必要。

**排他ロック**：ファイルごとに同時に1つのSyncAccessHandleのみオープン可能。Browserプロセスがファイル単位のロックでこれを強制。2つ目のハンドル作成を試みるとエラーを返す。仕様要件に一致し、並行書き込み競合を回避。

### 21.6.4 OPFSストレージ

OPFSファイルは構造化ディレクトリにディスク上で格納：

```
~/.local/share/elidex/opfs/
  └── <origin_hash>/
      ├── .metadata.json      （ファイル名 → 内部IDマッピング）
      ├── 0001.dat             （ファイル内容）
      ├── 0002.dat
      └── dirs/
          └── <subdir_hash>/
              ├── .metadata.json
              └── 0001.dat
```

ファイルはファイルシステム文字エンコーディング問題を回避するため、不透明な内部名（アプリケーション可視名ではない）を使用。メタデータファイルがアプリケーション名を内部IDにマッピング。

OPFSストレージはストレージクォータシステム（第22章§22.9）の対象。`navigator.storage.estimate()`にOPFS使用量を含む。

## 21.7 Core / Compat分類

| API | 分類 | 備考 |
| --- | --- | --- |
| `Blob` | Core | 基本データ型 |
| `File` | Core | Blobを拡張 |
| `URL.createObjectURL()` / `revokeObjectURL()` | Core | Blob URLライフサイクル |
| `blob.text()` / `blob.arrayBuffer()` / `blob.stream()` | Core | モダンBlob読み取り |
| `FileReader` | Compat | イベントベース、Blobメソッドで置換 |
| `ReadableStream` / `WritableStream` / `TransformStream` | Core | ストリーミングプリミティブ |
| `CompressionStream` / `DecompressionStream` | Core | ネイティブ圧縮 |
| `showOpenFilePicker()` / `showSaveFilePicker()` / `showDirectoryPicker()` | Core | ユーザー仲介FSアクセス |
| `FileSystemFileHandle` / `FileSystemDirectoryHandle` | Core | FSハンドルAPI |
| `FileSystemSyncAccessHandle` | Core | Worker専用同期OPFS |
| `navigator.storage.getDirectory()`（OPFS） | Core | サンドボックス化FSルート |

## 21.8 統合ポイント

| システム | 統合 | 参照 |
| --- | --- | --- |
| Fetch（第10章） | レスポンス/リクエストボディをByteStreamとして | §21.3.3 |
| 画像デコード（第18章） | BlobStore経由のBlob URL解決 | 第18章§18.9 |
| ストレージ（第22章） | OPFSクォータ管理 | 第22章§22.9 |
| パーミッション（第8章） | FileSystemAccessパーミッション | 第8章§8.3 |
| Platform Abstraction（第23章） | ファイルピッカーダイアログ | §21.5.1 |
| プロセスアーキテクチャ（第5章） | BlobStore、OPFS制御用IPC | §21.2.2、§21.6.3 |
| ScriptSession（第13章） | JS ReadableStream ↔ Rust ByteStream | §21.3.2 |

## 21.9 elidex-app

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| Blob/File | フルサポート | フルサポート |
| Blob URL | フルサポート | フルサポート |
| FileReader | Compat層 | 除外（デフォルト） |
| Streams API | フルサポート | フルサポート |
| File System Access | ピッカー + パーミッションプロンプト | `App::grant(FileRead/Write(PathPattern))` |
| OPFS | オリジン単位サンドボックス | アプリ単位サンドボックス |
| SyncAccessHandle | Worker専用 | Worker専用 |
| 直接ファイルシステム | 不可（サンドボックス） | AppCapability::FileRead/FileWrite経由 |

elidex-appではBlobStoreがインプロセスで動作可能（SingleProcessモード、第5章）でIPCオーバーヘッドなし。OPFSはオリジン単位ではなくアプリ単位ディレクトリでバッキング。`FileRead`/`FileWrite`ケイパビリティを持つアプリは、付与されたパスパターンに従い、ピッカープロンプトなしでFile System Access API経由でネイティブファイルシステムにも直接アクセス可能。
