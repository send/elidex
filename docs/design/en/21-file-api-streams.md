
# 21. File API & Streams

## 21.1 Overview

File and streaming APIs form a foundational layer for modern web applications: fetching and processing large files, streaming media, compressing data in-browser, and providing high-performance local storage for WebAssembly applications. Elidex addresses three interconnected API surfaces:

1. **File/Blob API**: Immutable binary data objects, Blob URLs, and file reading.
2. **Streams API**: ReadableStream, WritableStream, TransformStream for incremental processing with backpressure.
3. **File System Access API**: Picker-based access to the user's local filesystem, and the Origin Private File System (OPFS) for sandboxed per-origin storage.

```
┌─────────────────────────────────────────────────────────────┐
│ Renderer Process (Sandboxed)                                │
│                                                             │
│  ┌─────────────┐   ┌────────────┐   ┌───────────────────┐  │
│  │ ScriptSession│   │ ByteStream │   │ SyncAccessHandle  │  │
│  │ (JS API)     │◄──│ (Rust)     │   │ (shared memory)   │  │
│  │              │   │            │   │                   │  │
│  │ ReadableStream│  │ fetch body │   │ Worker-only sync  │  │
│  │ WritableStream│  │ file read  │   │ read/write on     │  │
│  │ Blob         │   │ compress   │   │ mmapped region    │  │
│  └──────┬───────┘   └─────┬──────┘   └────────┬──────────┘  │
│         │                 │                    │             │
│─────────┼─────────────────┼────────────────────┼─────────────│
│         │ IPC             │ IPC                │ shared mem  │
└─────────┼─────────────────┼────────────────────┼─────────────┘
          ▼                 ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│ Browser Process                                             │
│                                                             │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐ │
│  │ BlobStore   │   │ File I/O     │   │ OPFS Storage     │ │
│  │ (memory +   │   │ (native fs)  │   │ (per-origin dir) │ │
│  │  disk spill) │   │              │   │                  │ │
│  └─────────────┘   └──────────────┘   └──────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 21.2 Blob and File

### 21.2.1 Blob Model

A Blob represents an immutable sequence of bytes with a MIME type. In elidex, Blobs are managed by the BlobStore in the Browser Process:

```rust
pub struct BlobId(u64);

pub struct BlobMetadata {
    pub id: BlobId,
    pub size: u64,
    pub mime_type: String,
    pub backing: BlobBacking,
}

pub enum BlobBacking {
    /// Small blobs (≤256 KB): held in memory
    Inline(Bytes),
    /// Large blobs (>256 KB): spilled to temp file on disk
    DiskSpill {
        path: PathBuf,
        offset: u64,
        length: u64,
    },
    /// Slice of another blob (lazy, no copy)
    Slice {
        parent: BlobId,
        offset: u64,
        length: u64,
    },
    /// Concatenation of multiple blobs (lazy, no copy)
    Concat(Vec<BlobId>),
}
```

### 21.2.2 BlobStore

The BlobStore lives in the Browser Process because Renderer Processes are sandboxed and cannot perform direct file I/O:

```rust
pub struct BlobStore {
    blobs: HashMap<BlobId, BlobMetadata>,
    next_id: AtomicU64,
    /// Disk-backed blobs are stored here
    spill_dir: PathBuf,
    /// Total memory used by inline blobs
    memory_usage: AtomicU64,
    /// Threshold for spilling to disk (default: 256 KB per blob)
    spill_threshold: u64,
    /// Total memory budget for inline blobs (default: 64 MB)
    memory_budget: u64,
}

impl BlobStore {
    /// Create a new blob from data.
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

    /// Create a blob slice (zero-copy).
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

    /// Read blob data. Resolves slices/concats, reads from disk if needed.
    pub async fn read(&self, id: BlobId, range: Range<u64>) -> Result<Bytes, BlobError> {
        let meta = self.blobs.get(&id).ok_or(BlobError::NotFound)?;
        self.read_backing(&meta.backing, range).await
    }

    /// Read blob data as a ByteStream for streaming consumption.
    pub fn read_stream(&self, id: BlobId, chunk_size: usize) -> ByteStream {
        // Returns an async stream that reads the blob in chunks
        // Disk-backed blobs: async file read per chunk
        // Memory-backed blobs: slice per chunk
        ByteStream::from_blob(self, id, chunk_size)
    }
}
```

### 21.2.3 Blob URL

`URL.createObjectURL(blob)` creates a `blob:` URL that can be used in `<img>`, `<a>`, fetch, etc. The Blob URL registry lives in the BlobStore:

```rust
pub struct BlobUrlRegistry {
    /// Maps blob: URLs to BlobIds. Per-origin.
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

When a fetch or image load encounters a `blob:` URL, the Browser Process resolves it via BlobStore (which it owns, see §21.2.2) and returns the data as a response body. This is already referenced in Ch. 18 §18.9.

### 21.2.4 File

A `File` is a Blob with additional metadata (name, lastModified). It is created when the user selects files through `<input type="file">` or the File System Access API:

```rust
pub struct FileMetadata {
    pub blob_id: BlobId,
    pub name: String,
    pub last_modified: u64,   // milliseconds since epoch
}
```

The file contents are read into the BlobStore when selected. For large files, the BlobStore's disk spill ensures they are not held entirely in memory.

### 21.2.5 FileReader — Compat

FileReader is an event-based API (`onload`, `onerror`, `onprogress`) superseded by Promise-based alternatives:

| Legacy (Compat) | Modern (Core) |
| --- | --- |
| `FileReader.readAsText(blob)` | `await blob.text()` |
| `FileReader.readAsArrayBuffer(blob)` | `await blob.arrayBuffer()` |
| `FileReader.readAsDataURL(blob)` | `URL.createObjectURL(blob)` |
| `FileReader.readAsBinaryString(blob)` | `await blob.arrayBuffer()` + view |

In elidex-compat, FileReader is implemented as a wrapper around the modern Blob methods:

```rust
// elidex-compat: FileReader implementation
pub struct FileReaderCompat {
    // Wraps blob.stream() / blob.arrayBuffer() internally
    // Emits progress events at chunk boundaries
    // Translates Promise completion to load/error events
}
```

The core engine only implements `blob.text()`, `blob.arrayBuffer()`, and `blob.stream()`. FileReader is purely a compat shim.

## 21.3 Streams API

### 21.3.1 ByteStream — Internal Representation

The Streams API is exposed to JavaScript as ReadableStream / WritableStream / TransformStream. Internally, elidex represents streaming data as `ByteStream`, a Rust async stream:

```rust
use futures::Stream;

/// Internal byte stream with backpressure support.
/// The unit of streaming data within the engine.
pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>,
}

impl ByteStream {
    /// Create from an async reader (file, network socket, etc.)
    pub fn from_async_read<R: AsyncRead + Send + 'static>(reader: R, chunk_size: usize) -> Self {
        let stream = ReaderStream::with_capacity(reader, chunk_size);
        Self { inner: Box::pin(stream) }
    }

    /// Create from a Blob in the BlobStore
    pub fn from_blob(store: &BlobStore, id: BlobId, chunk_size: usize) -> Self {
        // Async stream that reads blob chunks on demand
        // ...
    }

    /// Create from an in-memory buffer
    pub fn from_bytes(data: Bytes) -> Self {
        Self { inner: Box::pin(futures::stream::once(async move { Ok(data) })) }
    }

    /// Pipe through a transform (compression, decompression, etc.)
    pub fn pipe_through<T: StreamTransform>(self, transform: T) -> Self {
        Self { inner: Box::pin(transform.transform(self.inner)) }
    }
}
```

### 21.3.2 JS Bridge

ScriptSession bridges between Rust `ByteStream` and JavaScript `ReadableStream`:

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
│ ScriptSession (Rust ↔ JS bridge)            │                   │
│                                             │                   │
│  ReadableStreamSource {                     │                   │
│    byte_stream: ByteStream,                 │ ◄─── pull protocol│
│    pull() → next chunk from byte_stream     │                   │
│    cancel() → drop byte_stream              │                   │
│  }                                          │                   │
├─────────────────────────────────────────────┤
│ Native (Rust)                               │
│                                             │
│  ByteStream (fetch response body)           │
│    → chunks arrive from Network Process     │
│    → async yield when no data available     │
│       (backpressure: stops reading socket)  │
└─────────────────────────────────────────────┘
```

The pull protocol provides natural backpressure: when JavaScript stops calling `reader.read()`, the ByteStream stops producing chunks, which propagates back to the network socket (TCP flow control) or file read.

### 21.3.3 ReadableStream

A ReadableStream in elidex can be backed by:

| Source | Backing | Notes |
| --- | --- | --- |
| Fetch response body | ByteStream from network | Ch. 10 |
| `blob.stream()` | ByteStream from BlobStore | §21.2.2 |
| OPFS file read | ByteStream from file | §21.5 |
| `new ReadableStream({ pull() })` | JS-driven | User-created stream |
| Compression stream | ByteStream piped through transform | §21.4 |

For native-backed streams, the entire data path can stay in Rust without crossing the JS boundary until the application actually reads chunks:

```
[Rust-to-Rust fast path]
  fetch response body (ByteStream)
    → pipe_through(DecompressionTransform)  // gzip decompress in Rust
    → write to OPFS (SyncAccessHandle)      // direct file write

  No JS involved. Maximum throughput.
```

### 21.3.4 WritableStream

WritableStream is the sink side. Native-backed sinks:

| Sink | Backing | Notes |
| --- | --- | --- |
| Fetch request body | ByteStream to network | Streaming upload |
| OPFS file write | ByteStream to file | §21.5 |
| `new WritableStream({ write() })` | JS-driven | User-created sink |

### 21.3.5 TransformStream

TransformStream connects a WritableStream (input) to a ReadableStream (output), processing data through a transformation:

```rust
pub trait StreamTransform: Send + 'static {
    fn transform(
        self,
        input: Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>;
}
```

Built-in transforms are implemented as native StreamTransform (see §21.4). User-created TransformStreams execute the transform logic in JavaScript.

### 21.3.6 Backpressure

Backpressure is fundamental to the Streams API. Elidex's backpressure model:

```
Producer                    Consumer
   │                           │
   ├── produce chunk ─────────►│ highWaterMark not reached
   ├── produce chunk ─────────►│ highWaterMark reached
   │◄── desiredSize ≤ 0 ──────┤ (backpressure signal)
   │   (producer pauses)       │
   │                           ├── consume chunk
   │                           ├── consume chunk
   │◄── desiredSize > 0 ──────┤ (backpressure relieved)
   ├── produce chunk ─────────►│
```

For native ByteStreams, backpressure is implicit: the Rust async Stream trait naturally yields when the consumer hasn't polled. For JS-backed streams, the ScriptSession tracks `desiredSize` and signals the underlying source to pause/resume.

Default high water marks:

| Stream Type | Default High Water Mark |
| --- | --- |
| Byte stream | 65,536 bytes (64 KB) |
| Object stream | 1 chunk |

## 21.4 Compression Streams

The Compression Streams API provides in-browser compression and decompression:

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
                // flate2 or miniz_oxide GzEncoder wrapping the input stream
                Box::pin(GzipCompressStream::new(input))
            }
            (CompressionAlgorithm::Gzip, Direction::Decompress) => {
                Box::pin(GzipDecompressStream::new(input))
            }
            // ... deflate, deflate-raw variants
        }
    }
}
```

JavaScript usage:
```javascript
const compressed = readableStream.pipeThrough(new CompressionStream("gzip"));
const decompressed = compressed.pipeThrough(new DecompressionStream("gzip"));
```

These are implemented as native StreamTransforms using Rust crates (flate2 / miniz_oxide), avoiding JS overhead for compression.

## 21.5 File System Access API

### 21.5.1 Picker-Based Access

`showOpenFilePicker()`, `showSaveFilePicker()`, and `showDirectoryPicker()` provide user-mediated access to the local filesystem:

```
JavaScript                 Browser Process              Platform
    │                          │                          │
    ├─ showOpenFilePicker() ──►│                          │
    │                          ├─ check Permission ──────►│
    │                          │  (FileSystemAccess,      │
    │                          │   Ch. 8 §8.3)            │
    │                          │                          │
    │                          ├─ PlatformFileDialog ────►│
    │                          │  (Ch. 23)                │ native dialog
    │                          │                          ├── user selects
    │                          │◄─────── file paths ──────┤
    │                          │                          │
    │                          ├─ create FileHandle ──────┤
    │                          │  (read blob into         │
    │◄── FileSystemFileHandle ─┤   BlobStore)             │
    │                          │                          │
```

### 21.5.2 FileSystemFileHandle

```rust
pub struct FileSystemFileHandle {
    /// Browser Process handle ID
    pub handle_id: HandleId,
    /// File name
    pub name: String,
    /// Permission state for this handle
    pub read_permission: PermissionState,
    pub write_permission: PermissionState,
}

impl FileSystemFileHandle {
    /// Get a File object (reads into BlobStore)
    pub async fn get_file(&self) -> Result<FileMetadata, FileError> {
        // IPC to Browser Process → read file → create Blob → return File
    }

    /// Create a writable stream for this file
    pub async fn create_writable(&self) -> Result<FileSystemWritableFileStream, FileError> {
        // IPC to Browser Process → open file for writing
        // Returns a WritableStream backed by native file I/O
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
    /// Iterate entries in this directory
    pub async fn entries(&self) -> Result<Vec<FileSystemEntry>, FileError> {
        // IPC to Browser Process → readdir
    }

    /// Get a file handle by name
    pub async fn get_file_handle(&self, name: &str, create: bool) -> Result<FileSystemFileHandle, FileError> {
        // IPC to Browser Process
    }

    /// Get a subdirectory handle by name
    pub async fn get_directory_handle(&self, name: &str, create: bool) -> Result<FileSystemDirectoryHandle, FileError> {
        // IPC to Browser Process
    }

    /// Remove an entry
    pub async fn remove_entry(&self, name: &str, recursive: bool) -> Result<(), FileError> {
        // IPC to Browser Process
    }
}

pub enum FileSystemEntry {
    File(FileSystemFileHandle),
    Directory(FileSystemDirectoryHandle),
}
```

### 21.5.4 Permission Model

File System Access integrates with the permission model (Ch. 8 §8.3):

- **Read permission**: Granted implicitly when user selects a file/directory via picker.
- **Write permission**: Requires explicit prompt on first write to a user-selected file.
- **Persistent permission**: Handles can be stored in IndexedDB (Ch. 22) and re-validated on next session. The browser re-prompts if the permission has expired.

For elidex-app, `App::grant(FileRead(PathPattern))` and `App::grant(FileWrite(PathPattern))` provide build-time grants (Ch. 8 §8.8).

## 21.6 Origin Private File System (OPFS)

### 21.6.1 Overview

OPFS provides a sandboxed, per-origin filesystem that does not correspond to visible files on the user's disk. It is the recommended high-performance storage mechanism for WebAssembly applications (SQLite-on-web, Figma, etc.).

```rust
/// Root of OPFS for a given origin
pub struct OpfsRoot {
    pub origin: Origin,
    /// Actual storage location on disk
    /// e.g., ~/.local/share/elidex/opfs/<origin_hash>/
    pub base_path: PathBuf,
}
```

Access is via `navigator.storage.getDirectory()`, which returns a `FileSystemDirectoryHandle` pointing to the OPFS root.

### 21.6.2 Async Access (Main Thread / Workers)

Standard `FileSystemFileHandle` methods (`getFile()`, `createWritable()`) work on OPFS files through IPC to the Browser Process, same as local filesystem files. This is suitable for occasional reads/writes.

### 21.6.3 SyncAccessHandle (Workers Only)

The performance-critical API. `FileSystemSyncAccessHandle` provides synchronous read/write/truncate/flush on OPFS files, but only in dedicated Workers (not on the main thread, to avoid blocking UI).

**Design: Shared Memory Mapped File**

```
┌─────────────────────────────────────────────┐
│ Worker Thread (Renderer Process)            │
│                                             │
│  SyncAccessHandle                           │
│    ├── read(buffer, offset)  ──► memcpy     │
│    ├── write(buffer, offset) ──► memcpy     │
│    ├── truncate(size)        ──► IPC        │
│    ├── flush()               ──► IPC        │
│    └── close()               ──► IPC        │
│                                    │        │
│         shared memory region ◄─────┘        │
│         (mmapped file)                      │
│                                             │
├─────────────────────────────────────────────┤
│ Browser Process                             │
│                                             │
│  OpfsFileManager                            │
│    ├── open file                            │
│    ├── mmap into shared memory              │
│    ├── share mapping with Worker            │
│    ├── handle truncate/flush/close IPC      │
│    └── enforce exclusive lock               │
└─────────────────────────────────────────────┘
```

```rust
pub struct SyncAccessHandle {
    /// Shared memory region mapped to the OPFS file
    mapping: SharedMemoryMapping,
    /// Current file size
    size: AtomicU64,
    /// IPC channel for control operations (truncate, flush, close)
    control: IpcSender<OpfsControlMsg>,
}

impl SyncAccessHandle {
    /// Synchronous read. No IPC — direct memory access.
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

    /// Synchronous write. No IPC — direct memory access.
    pub fn write(&self, buffer: &[u8], offset: u64) -> Result<usize, FileError> {
        let end = offset + buffer.len() as u64;
        if end > self.mapping.capacity() {
            // Need to extend mapping — IPC to Browser Process
            self.control.send(OpfsControlMsg::ExtendMapping { new_size: end })?;
            // Block until mapping is extended
        }
        self.mapping.as_mut_slice()[offset as usize..][..buffer.len()].copy_from_slice(buffer);
        self.size.fetch_max(end, Ordering::Release);
        Ok(buffer.len())
    }

    /// Flush to disk. Requires IPC.
    pub fn flush(&self) -> Result<(), FileError> {
        self.control.send(OpfsControlMsg::Flush)?;
        self.control.recv_ack()?;
        Ok(())
    }

    /// Truncate file. Requires IPC.
    pub fn truncate(&self, size: u64) -> Result<(), FileError> {
        self.control.send(OpfsControlMsg::Truncate { size })?;
        self.control.recv_ack()?;
        self.size.store(size, Ordering::Release);
        Ok(())
    }

    /// Close the handle. Releases exclusive lock.
    pub fn close(self) -> Result<(), FileError> {
        self.control.send(OpfsControlMsg::Close)?;
        Ok(())
    }
}
```

This design eliminates IPC round-trips for read/write (the hot path), making it suitable for SQLite which performs thousands of small reads/writes per second. Only control operations (truncate, flush, close) require IPC.

**Exclusive locking**: Only one SyncAccessHandle can be open per file at a time. The Browser Process enforces this with a per-file lock. Attempting to create a second handle returns an error. This matches the spec requirement and avoids concurrent write conflicts.

### 21.6.4 OPFS Storage

OPFS files are stored on disk in a structured directory:

```
~/.local/share/elidex/opfs/
  └── <origin_hash>/
      ├── .metadata.json      (file name → internal ID mapping)
      ├── 0001.dat             (file contents)
      ├── 0002.dat
      └── dirs/
          └── <subdir_hash>/
              ├── .metadata.json
              └── 0001.dat
```

Files use opaque internal names (not the application-visible names) to avoid filesystem character encoding issues. The metadata file maps application names to internal IDs.

OPFS storage is subject to the storage quota system (Ch. 22 §22.9). `navigator.storage.estimate()` includes OPFS usage.

## 21.7 Core / Compat Classification

| API | Classification | Notes |
| --- | --- | --- |
| `Blob` | Core | Fundamental data type |
| `File` | Core | Extends Blob |
| `URL.createObjectURL()` / `revokeObjectURL()` | Core | Blob URL lifecycle |
| `blob.text()` / `blob.arrayBuffer()` / `blob.stream()` | Core | Modern Blob reading |
| `FileReader` | Compat | Event-based, superseded by Blob methods |
| `ReadableStream` / `WritableStream` / `TransformStream` | Core | Streaming primitives |
| `CompressionStream` / `DecompressionStream` | Core | Native compression |
| `showOpenFilePicker()` / `showSaveFilePicker()` / `showDirectoryPicker()` | Core | User-mediated FS access |
| `FileSystemFileHandle` / `FileSystemDirectoryHandle` | Core | FS handle API |
| `FileSystemSyncAccessHandle` | Core | Worker-only sync OPFS |
| `navigator.storage.getDirectory()` (OPFS) | Core | Sandboxed FS root |

## 21.8 Integration Points

| System | Integration | Reference |
| --- | --- | --- |
| Fetch (Ch. 10) | Response/request bodies as ByteStream | §21.3.3 |
| Image decode (Ch. 18) | Blob URL resolution via BlobStore | Ch. 18 §18.9 |
| Storage (Ch. 22) | OPFS quota management | Ch. 22 §22.9 |
| Permissions (Ch. 8) | FileSystemAccess permission | Ch. 8 §8.3 |
| Platform Abstraction (Ch. 23) | File picker dialogs | §21.5.1 |
| Process Architecture (Ch. 5) | IPC for BlobStore, OPFS control | §21.2.2, §21.6.3 |
| ScriptSession (Ch. 13) | JS ReadableStream ↔ Rust ByteStream | §21.3.2 |

## 21.9 elidex-app

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| Blob/File | Full support | Full support |
| Blob URL | Full support | Full support |
| FileReader | Compat layer | Excluded (default) |
| Streams API | Full support | Full support |
| File System Access | Picker + permission prompt | `App::grant(FileRead/Write(PathPattern))` |
| OPFS | Per-origin sandboxed | Per-app sandboxed |
| SyncAccessHandle | Workers only | Workers only |
| Direct filesystem | No (sandboxed) | Via AppCapability::FileRead/FileWrite |

In elidex-app, the BlobStore can operate in-process (SingleProcess mode, Ch. 5) without IPC overhead. OPFS is backed by a per-app directory rather than per-origin. Apps with `FileRead`/`FileWrite` capabilities can also access the native filesystem directly through the File System Access API without picker prompts, subject to the granted path patterns.
