
# 22. Storage & Cache Architecture

Browsers manage a surprisingly large amount of persistent and transient state — HTTP cache, cookies, history, bookmarks, IndexedDB databases, origin-private file systems, and more. These systems differ in ownership (browser vs. web content), lifetime (session vs. persistent), isolation requirements (global vs. per-origin), and performance characteristics (memory vs. disk). This chapter defines the unified storage architecture that underpins all of them.

## 22.1 Design Principles

**Origin isolation by construction.** Web-content storage is physically separated by origin — one SQLite database per origin per storage type. Cross-origin data leakage cannot occur through query bugs because the data lives in separate files. This is a structural guarantee, not a convention enforced by code review.

**Browser data is centralized.** Data owned by the browser itself (cookies, HSTS, history, bookmarks, settings, permissions) lives in a single shared database. Only the Browser Process accesses this database; Renderers never touch it directly.

**Trait abstraction at every layer.** The storage stack has two layers: a low-level StorageBackend trait (database open/close/migrate) and high-level domain traits (CookieStore, HistoryStore, OriginStorage). SQLite is the initial implementation but is not a structural dependency.

**Secure defaults for quota and eviction.** Origin storage is quota-managed with LRU eviction. Persistent storage requires explicit user/API grant.

## 22.2 Storage Taxonomy

All persistent and cached state falls into one of two categories:

| Category | Owner | Isolation | Access | Examples |
| --- | --- | --- | --- | --- |
| Browser-owned | Browser Process | Global (single DB) | Browser Process only | Cookies, HSTS, history, bookmarks, settings, permissions, certificate overrides |
| Web-content-owned | Origin | Per-origin (separate DB/directory) | Browser Process mediates; Renderer requests via IPC | elidex.storage (KV), IndexedDB, Cache API (Service Worker), OPFS, localStorage (compat) |

### 22.2.1 Filesystem Layout

```
{profile_dir}/
├── browser.sqlite              # Browser-owned centralized DB
├── browser.sqlite-wal          # WAL journal
├── http-cache/                 # HTTP cache (Section 22.6)
│   ├── index.sqlite            # Cache metadata
│   └── entries/                # Cached response bodies (content-addressed)
│       ├── a1b2c3d4...
│       └── e5f6a7b8...
└── origins/                    # Web-content storage, per-origin
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

The origin key is derived as `{scheme}_{host}_{port}` with filesystem-safe escaping. This mirrors the Origin tuple from the security model (Ch. 8) and provides a 1:1 mapping between web origins and filesystem directories.

## 22.3 Storage Backend Abstraction

### 22.3.1 StorageBackend Trait

The low-level trait abstracts database lifecycle. It does not expose SQL — that is an implementation detail of the SQLite backend:

```rust
pub trait StorageBackend: Send + Sync {
    type Connection: StorageConnection;

    /// Open or create a database at the given path.
    fn open(&self, path: &Path, options: OpenOptions) -> Result<Self::Connection, StorageError>;

    /// Run schema migrations to bring the database to the current version.
    fn migrate(&self, conn: &Self::Connection, migrations: &[Migration]) -> Result<(), StorageError>;

    /// Backend name for diagnostics.
    fn name(&self) -> &str;
}

pub trait StorageConnection: Send {
    /// Execute a named operation. Operations are defined by domain traits,
    /// not as raw SQL. The backend maps them to its native query language.
    fn execute(&self, op: &StorageOp) -> Result<StorageResult, StorageError>;

    /// Begin a transaction.
    fn transaction<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Self) -> Result<T, StorageError>;
}

pub struct OpenOptions {
    pub read_only: bool,
    pub create_if_missing: bool,
    pub wal_mode: bool,            // SQLite WAL — ignored by non-SQLite backends
    pub busy_timeout: Duration,     // Lock contention timeout
}
```

### 22.3.2 SQLite Implementation

```rust
pub struct SqliteBackend;

impl StorageBackend for SqliteBackend {
    type Connection = SqliteConnection;  // Wraps rusqlite::Connection

    fn open(&self, path: &Path, options: OpenOptions) -> Result<SqliteConnection, StorageError> {
        let conn = rusqlite::Connection::open(path)?;
        if options.wal_mode {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.busy_timeout(options.busy_timeout)?;
        // Security hardening
        conn.pragma_update(None, "secure_delete", "ON")?;
        Ok(SqliteConnection(conn))
    }

    fn name(&self) -> &str { "sqlite" }
    // ...
}
```

SQLite-specific configuration applied uniformly:

| Pragma | Value | Rationale |
| --- | --- | --- |
| journal_mode | WAL | Concurrent reads during write. Critical for browser responsiveness. |
| synchronous | NORMAL | Balanced durability/performance. WAL mode makes NORMAL safe against corruption (only vulnerable to OS crash, not app crash). |
| secure_delete | ON | Overwrite deleted data with zeros. Prevents recovery of cleared browsing data from disk. |
| foreign_keys | ON | Referential integrity for browser.sqlite relational tables. |
| busy_timeout | 5000ms | Avoid immediate SQLITE_BUSY errors under contention. |
| cache_size | -8000 (8MB) | Per-connection page cache. Tunable per storage type. |

### 22.3.3 StorageOp Abstraction

Rather than leaking SQL through the trait, domain operations are expressed as typed enums:

```rust
pub enum StorageOp {
    Get { table: &'static str, key: Vec<u8> },
    Put { table: &'static str, key: Vec<u8>, value: Vec<u8> },
    Delete { table: &'static str, key: Vec<u8> },
    Scan { table: &'static str, prefix: Vec<u8>, limit: usize },
    Custom(Box<dyn CustomOp>),  // Escape hatch for complex queries (IndexedDB)
}
```

This is intentionally minimal. Most domain traits (CookieStore, HistoryStore) build higher-level operations on top of these primitives. IndexedDB's complex cursor/index queries use the `Custom` variant, which the SQLite backend maps to prepared statements.

If the backend is replaced in the future, only the StorageOp-to-query mapping changes. Domain traits and their callers are unaffected.

## 22.4 Browser-Owned Storage

### 22.4.1 browser.sqlite Schema

The centralized browser database contains all browser-owned state. Only the Browser Process opens this file:

| Table | Key Columns | Notes |
| --- | --- | --- |
| cookies | (host, path, name, partition_key) | Full cookie attributes: value, expires, secure, httponly, samesite, creation_time, last_access_time. Partition key for CHIPS. |
| hsts | (host) | include_subdomains, expiry, source (preload vs. dynamic). |
| history | (url, visit_time) | Title, visit_count, typed_count. Frecency score for suggestions. |
| bookmarks | (id, parent_id) | Tree structure. title, url, position, date_added. |
| permissions | (origin, permission_type) | granted/denied/prompt, expiry. Maps to Permissions API (OPEN-010). |
| settings | (key) | Browser preferences. JSON value. |
| cert_overrides | (host, port) | User-accepted certificate exceptions. |
| content_prefs | (origin, key) | Per-site preferences (zoom level, notification permission). |

### 22.4.2 Domain Traits (Browser-Owned)

These traits are the public API consumed by the rest of the engine. Several were introduced in earlier chapters; they now get a concrete storage backing:

```rust
/// Cookie persistence — backs CookieJar (Ch. 10)
pub trait CookiePersistence: Send + Sync {
    fn load_all(&self) -> Result<Vec<Cookie>>;
    fn persist(&self, cookie: &Cookie) -> Result<()>;
    fn delete(&self, domain: &str, path: &str, name: &str) -> Result<()>;
    fn delete_expired(&self, now: SystemTime) -> Result<usize>;
    fn clear_origin(&self, origin: &Origin) -> Result<()>;
}

/// History — backs NavigationManager (Ch. 24)
pub trait HistoryStore: Send + Sync {
    fn record_visit(&self, url: &Url, title: &str, transition: TransitionType) -> Result<()>;
    fn query(&self, text: &str, limit: usize) -> Result<Vec<HistoryEntry>>;
    fn frecency_suggest(&self, prefix: &str, limit: usize) -> Result<Vec<Suggestion>>;
    fn delete_range(&self, from: SystemTime, to: SystemTime) -> Result<()>;
    fn delete_url(&self, url: &Url) -> Result<()>;
}

/// Bookmark — backs BookmarkStore (Ch. 24)
pub trait BookmarkPersistence: Send + Sync {
    fn load_tree(&self) -> Result<BookmarkNode>;
    fn add(&self, parent_id: BookmarkId, bookmark: Bookmark) -> Result<BookmarkId>;
    fn update(&self, id: BookmarkId, changes: BookmarkUpdate) -> Result<()>;
    fn remove(&self, id: BookmarkId) -> Result<()>;
    fn move_to(&self, id: BookmarkId, new_parent: BookmarkId, position: usize) -> Result<()>;
}
```

The CookieJar (Ch. 10) holds the in-memory working set and calls CookiePersistence for durability. On startup, `load_all()` populates the in-memory jar. Mutations are written through asynchronously (batch writes on a timer or at shutdown).

### 22.4.3 Cookie Partitioning

The cookie table includes a `partition_key` column to support CHIPS (Cookies Having Independent Partitioned State). In a partitioned cookie model, third-party cookies are keyed by `(top-level-site, cookie-domain)` rather than just `(cookie-domain)`. This prevents cross-site tracking while allowing legitimate embedded use cases.

| Scenario | partition_key | Behavior |
| --- | --- | --- |
| First-party cookie on example.com | NULL | Standard first-party behavior |
| Third-party cookie from cdn.example.com embedded on news.com | "https://news.com" | Cookie is scoped to the news.com context. Same CDN cookie on shop.com gets a separate partition. |

## 22.5 Web-Content Storage

### 22.5.1 OriginStorageManager

The central coordinator for all per-origin storage. Lives in the Browser Process:

```rust
pub struct OriginStorageManager {
    profile_dir: PathBuf,
    backend: Box<dyn StorageBackend>,
    /// Open connections, keyed by (origin, storage_type)
    connections: Mutex<HashMap<(OriginKey, StorageType), Box<dyn StorageConnection>>>,
    /// Quota tracking
    quota_manager: QuotaManager,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct OriginKey(String);  // e.g., "https_example.com_443"

pub enum StorageType {
    KeyValue,       // elidex.storage
    IndexedDb,      // IndexedDB
    CacheApi,       // Service Worker Cache API
    Opfs,           // Origin Private File System
    LocalStorage,   // compat — localStorage
}

impl OriginStorageManager {
    /// Resolve the database path for a given origin and storage type.
    fn db_path(&self, origin: &OriginKey, storage_type: StorageType) -> PathBuf {
        self.profile_dir
            .join("origins")
            .join(origin.as_str())
            .join(storage_type.filename())
    }

    /// Get or open a connection. Lazy — databases are created on first access.
    pub fn connection(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
    ) -> Result<&dyn StorageConnection, StorageError> {
        // ...
    }

    /// Delete all storage for an origin. Called by "Clear Site Data".
    pub fn clear_origin(&self, origin: &OriginKey) -> Result<()> {
        // Close connections, delete origin directory
    }
}
```

### 22.5.2 IPC Flow for Web-Content Storage

Renderers never access origin databases directly. All storage operations go through IPC to the Browser Process:

```
Renderer (site: example.com)
  │
  │  RendererToBrowser::StorageRequest {
  │      origin: "https_example.com_443",
  │      storage_type: KeyValue,
  │      op: Put { key: "user-prefs", value: ... }
  │  }
  │
  ├──────── IPC (Ch. 5) ────────▶  Browser Process
  │                                   │
  │                                   ├─ Verify origin matches Renderer's site
  │                                   ├─ OriginStorageManager.connection(origin, KV)
  │                                   ├─ connection.execute(op)
  │                                   │
  │  BrowserToRenderer::StorageResponse {    │
  │      result: Ok(())                      │
  │  }                                       │
  │◀──────── IPC ────────────────────────────┘
```

The Browser Process **verifies the origin claim** before executing. A compromised Renderer claiming a different origin is rejected. This is the same trust model as CORS enforcement in the Network Process (Ch. 10.8).

### 22.5.3 elidex.storage (Async KV)

The core storage API defined in Ch. 14 (AsyncStorage trait). The backing is a per-origin SQLite database with a simple schema:

```sql
-- kv.sqlite (per-origin)
CREATE TABLE kv (
    key   TEXT PRIMARY KEY,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL  -- Unix epoch milliseconds
);
```

This is the recommended replacement for localStorage. Async, non-blocking, no 5MB size limit (governed by origin quota instead).

### 22.5.4 IndexedDB

IndexedDB is backed by a per-origin SQLite database. The mapping from IndexedDB concepts to SQLite:

| IndexedDB Concept | SQLite Mapping |
| --- | --- |
| Database | Schema version tracked in metadata table |
| Object Store | Table |
| Key | PRIMARY KEY (serialized IDB key) |
| Value | BLOB (structured clone serialization) |
| Index | SQLite INDEX on serialized key path |
| Cursor | SELECT with ORDER BY and LIMIT/OFFSET (forward/backward iteration) |
| Key range (IDBKeyRange) | WHERE clause with >, >=, <, <= on serialized key |
| Transaction (readonly/readwrite) | SQLite transaction (DEFERRED for readonly, IMMEDIATE for readwrite) |

This follows Chrome's proven approach. The structured clone serialization format for values is shared with `postMessage` (Ch. 5 IPC) to avoid multiple serialization formats.

### 22.5.5 Cache API (Service Worker)

The Cache API provides named caches of Request/Response pairs. Per-origin:

```sql
-- cache/index.sqlite (per-origin)
CREATE TABLE caches (
    id   INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE entries (
    cache_id    INTEGER REFERENCES caches(id),
    request_url TEXT NOT NULL,
    request_method TEXT NOT NULL,
    vary_header TEXT,           -- For Vary-based matching
    response_status INTEGER,
    response_headers BLOB,     -- Serialized header map
    body_path   TEXT,           -- Relative path to body file in entries/
    body_size   INTEGER,
    created_at  INTEGER,
    PRIMARY KEY (cache_id, request_url, request_method)
);
```

Response bodies are stored as files (not BLOBs) to avoid SQLite bloat on large cached resources.

## 22.6 HTTP Cache

The HTTP cache is a critical performance system, separate from web-content storage. It lives in the Network Process and is shared across all origins (but with privacy-aware keying).

### 22.6.1 Cache Key

To prevent cross-site tracking via cache probing, elidex uses double-keyed caching (matching Chrome's partition strategy):

```rust
pub struct HttpCacheKey {
    /// Top-level site that initiated the navigation
    top_level_site: SchemefulSite,
    /// The actual resource URL
    resource_url: Url,
    /// Whether this is a cross-site subresource (affects partitioning)
    is_cross_site: bool,
}
```

| Request Context | Cache Key | Sharing |
| --- | --- | --- |
| Top-level navigation to A.com loading script.js from CDN | (A.com, cdn.example.com/script.js) | Not shared with B.com loading same script |
| Top-level navigation to A.com loading own resource | (A.com, A.com/style.css) | Shared across A.com pages |

This is a privacy/performance trade-off. Double-keying reduces cache hit rates (the same CDN resource is cached separately per top-level site) but eliminates cache-based tracking.

### 22.6.2 Cache Storage Structure

```rust
pub struct HttpCache {
    /// Metadata index — maps cache keys to entry metadata
    index: HttpCacheIndex,       // SQLite-backed (http-cache/index.sqlite)
    /// Entry body storage — content-addressed files
    body_store: BodyStore,       // File-based (http-cache/entries/)
    /// In-memory LRU for hot entries
    memory_cache: LruCache<HttpCacheKey, CachedResponse>,
    /// Disk budget
    max_disk_size: u64,          // Default: 512MB, configurable
    max_memory_size: u64,        // Default: 64MB
}
```

Metadata (URL, headers, expiry, ETag, Last-Modified) is stored in SQLite for efficient querying. Response bodies are stored as content-addressed files on disk. This separation means metadata lookups (cache hit checks) don't require reading the full response body.

### 22.6.3 Cache-Control Compliance

| Directive | Behavior |
| --- | --- |
| max-age | Entry expires after specified seconds. Served from cache if fresh. |
| no-cache | Entry stored but must be revalidated (conditional GET) on every use. |
| no-store | Entry is never stored to disk. May exist in memory cache for the duration of the navigation. |
| must-revalidate | Stale entries must be revalidated; never served stale. |
| stale-while-revalidate | Serve stale entry immediately while revalidating in background. Improves perceived performance. |
| immutable | Entry never revalidated while fresh. Used for versioned assets (e.g., style.a1b2c3.css). |
| private | Not stored in shared caches (irrelevant for browser cache, but respected). |
| Vary | Response varies by request header. Cache key includes Vary header values. |

Conditional revalidation:

```
Cache has stale entry for resource
  → Send conditional GET:
     If-None-Match: "etag-value"       (if ETag was present)
     If-Modified-Since: <date>          (if Last-Modified was present)
  → Server responds:
     ├── 304 Not Modified → reuse cached body, update headers/expiry
     └── 200 OK → replace cached entry with new response
```

### 22.6.4 Eviction

When disk usage exceeds `max_disk_size`:

1. Evict entries marked `no-cache` or expired first
2. LRU by last access time for remaining entries
3. Eviction is batched (run periodically, not on every write) to avoid I/O storms

## 22.7 Memory Caches

Several performance-critical caches live entirely in memory within the Renderer Process. These are not persisted and are recreated on page load or process restart:

| Cache | Location | Capacity | Eviction | Purpose |
| --- | --- | --- | --- | --- |
| **Image decode cache** | Renderer | Memory budget (default 128MB) | LRU by decoded size | Stores decoded bitmaps to avoid re-decoding on scroll/repaint. Key: (URL, decoded size, scale factor) — see Ch. 18 §18.7. |
| **Style sharing cache** | Renderer | Per-frame, ~1000 entries | Cleared on style recalc | Servo-inspired: siblings with identical style inputs share computed style. Avoids redundant property resolution. |
| **Font glyph cache** | Renderer | Per font face, LRU | LRU by glyph age | Rasterized glyph bitmaps at specific sizes. Fed to GPU atlas texture. |
| **Bytecode cache** | Renderer → disk | Per-origin directory | LRU by script age | SpiderMonkey bytecode cache. Avoids reparse/recompile of unchanged scripts. Persisted to origin's storage directory for cross-session benefit. |
| **DNS cache** | Network Process | In-memory, TTL-based | TTL expiry | Defined in Ch. 10 (DnsCache). Respects DNS TTL. Negative caching with shorter TTL. |
| **CORS preflight cache** | Network Process | In-memory, per (origin, method, headers) | Access-Control-Max-Age expiry | Defined in Ch. 10. Avoids redundant OPTIONS requests. |
| **Connection pool** | Network Process | 256 connections max | Idle timeout (90s) | Defined in Ch. 10 (ConnectionPool). Keep-alive connections for reuse. |

### 22.7.1 Memory Pressure Handling

When the OS signals memory pressure (or per-tab memory exceeds budget):

| Pressure Level | Action |
| --- | --- |
| Moderate | Trim image decode cache to 50%. Evict style sharing cache. Release idle connections. |
| Critical | Flush image decode cache entirely. Discard non-visible tab Renderer processes (tab discarding — Browser Process recreates them on activation). Release all idle connections. Compact SQLite databases (`PRAGMA incremental_vacuum`). |

Tab discarding priority: LRU by last user interaction, with exceptions for tabs playing audio, holding WebRTC connections, or with unsaved form data.

## 22.8 Back/Forward Cache (bfcache)

The bfcache preserves complete page state (DOM, JS heap, Renderer process) when the user navigates away, enabling instant back/forward navigation.

### 22.8.1 Lifecycle

```
User navigates from Page A to Page B:
  1. Page A receives 'pagehide' event (persisted = true)
  2. Page A's Renderer state is frozen (JS execution paused, timers suspended)
  3. Page A's frozen state is stored in bfcache (Browser Process manages the cache)
  4. Page B loads normally in a new (or reused) Renderer

User presses Back:
  1. Page B receives 'pagehide' (may itself enter bfcache)
  2. Page A's Renderer is thawed from bfcache
  3. Page A receives 'pageshow' event (persisted = true)
  4. Instant display — no network fetch, no parse, no style/layout
```

### 22.8.2 Eligibility

Not all pages can enter bfcache. Disqualifying conditions:

| Condition | Reason |
| --- | --- |
| Open WebSocket or WebRTC connection | Active network connections cannot be frozen |
| `unload` event listener registered | Legacy behavior depends on running unload; freezing skips it |
| `Cache-Control: no-store` on main document | Explicitly opts out of caching |
| Active IndexedDB transaction | Transaction state cannot be reliably frozen |
| In-progress `fetch()` with body streaming | Streaming state cannot be serialized |

Pages that register `pagehide` (not `unload`) and use `BroadcastChannel` instead of `SharedWorker` are bfcache-friendly by design.

### 22.8.3 Cache Size

| Parameter | Default | Notes |
| --- | --- | --- |
| Max cached pages | 6 | Per browsing session. LRU eviction. |
| Max memory per entry | 256MB | Entries exceeding this are evicted. |
| Max total bfcache memory | 512MB | Shared across all entries. |

bfcache entries are prime candidates for eviction under memory pressure (Section 22.7.1).

## 22.9 Quota Management

### 22.9.1 Storage Quota Model

Each origin has a storage budget for its web-content data:

```rust
pub struct QuotaManager {
    /// Global storage limit (default: 80% of available disk, max 100GB)
    global_limit: u64,
    /// Per-origin limit (default: min(20% of global, 10GB))
    per_origin_limit: u64,
    /// Origins with persistent storage grant (exempt from eviction)
    persistent_origins: HashSet<OriginKey>,
}
```

| Storage Type | Counted Against Quota | Notes |
| --- | --- | --- |
| elidex.storage (KV) | Yes | |
| IndexedDB | Yes | |
| Cache API | Yes | |
| OPFS | Yes | |
| localStorage (compat) | Yes (5MB hard limit retained) | Web compat — localStorage has always been 5MB |
| Cookies | No | Managed separately (4KB per cookie, browser-owned) |
| HTTP cache | No | Managed by its own disk budget (Section 22.6) |

### 22.9.2 Web API Surface

```
// navigator.storage.estimate()
{ quota: 2147483648, usage: 52428800 }   // 2GB quota, 50MB used

// navigator.storage.persist()
// → prompts user (or auto-grants based on engagement heuristic)
// → persistent origins exempt from eviction under storage pressure
```

### 22.9.3 Eviction Under Pressure

When total storage approaches `global_limit`:

1. Identify non-persistent origins
2. Sort by last access time (LRU)
3. Evict entire origin storage (all storage types) starting from least recent
4. Notify affected pages via `storage` event if they are still loaded
5. Continue until usage drops below 80% of `global_limit`

Persistent origins (those granted `navigator.storage.persist()`) are only evicted if all non-persistent origins have been cleared and pressure remains.

### 22.9.4 Clear-Site-Data

The `Clear-Site-Data` HTTP response header allows servers to request storage deletion:

| Header Value | Action |
| --- | --- |
| `"cache"` | Clear HTTP cache entries for the origin |
| `"cookies"` | Clear cookies for the origin |
| `"storage"` | Clear all web-content storage for the origin |
| `"executionContexts"` | Reload all active documents for the origin |
| `"*"` | All of the above |

Processed by the Network Process, which coordinates with Browser Process for storage deletion and Renderer for execution context reload.

## 22.10 elidex-app Storage

In elidex-app mode, storage behavior differs:

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| Origin model | Standard web origins | App-defined (typically single origin) |
| Storage location | Profile directory | App-defined (e.g., platform-standard app data directory) |
| Quota | Managed, per-origin limits | No quota by default (app controls its own storage) |
| Browser-owned tables | Full set | Subset: settings only (no history, bookmarks) |
| IndexedDB | Available | Available |
| elidex.storage | Available | Available (primary recommended API) |
| HTTP cache | Full | Available, configurable size |

```rust
let app = elidex_app::App::new()
    .storage_dir("/path/to/app/data")
    .storage_backend(SqliteBackend::new())   // Or a custom backend
    .http_cache_size(100 * 1024 * 1024)      // 100MB HTTP cache
    .build();
```

In SingleProcess mode (Ch. 5), the OriginStorageManager runs as an in-process service. The IPC-mediated access pattern is preserved (using LocalChannel), maintaining the same security verification even without process boundaries.

## 22.11 Preload and Resource Hints

Resource hints affect the network and cache layers:

| Hint | Processing | Cache Interaction |
| --- | --- | --- |
| `<link rel="preconnect">` | DNS + TCP + TLS handshake initiated immediately | No cache effect. Connection pool benefit. |
| `<link rel="dns-prefetch">` | DNS resolution only | Populates DNS cache. |
| `<link rel="preload">` | Full fetch, high priority | Resource stored in memory cache. Must be used by the page or console warning. |
| `<link rel="prefetch">` | Full fetch, idle priority | Resource stored in HTTP cache. For likely-next-navigation resources. |
| `<link rel="prerender">` | Full page load in hidden Renderer | Essentially bfcache for a predicted navigation. |

Preload/prefetch requests go through the same middleware pipeline (Ch. 10) as regular fetches, meaning content blocking and privacy middleware apply.
