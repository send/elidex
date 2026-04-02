# Architecture: Network & Storage (elidex-net, elidex-storage-core, elidex-api-sw)

## elidex-net

- **NetClient**: Top-level API integrating transport, cookies, middleware, redirect, CORS, HTTPS-upgrade. `send()` for raw HTTP, `load()` for resource loading (http/data/file).
- **HttpTransport**: Sends requests via connection pool with timeout. Wraps hyper HTTP/1.1 and HTTP/2.
- **ConnectionPool**: Per-origin pooling. `OriginKey(scheme, host, port)`. H1: up to 6 idle connections. H2: single multiplexed `SendRequest` clone. 90s idle eviction.
- **Connector**: TCP+TLS with DNS-level SSRF protection. Uses `TokioIo<StreamWrapper>` for hyper compatibility. ALPN negotiation for H2.
- **TLS**: rustls with ring provider, webpki-roots, TLS 1.2/1.3, ALPN `h2, http/1.1`.
- **CookieJar**: RFC 6265bis §5.7 compliant. `generation()` / `snapshot()` / `load()` for decoupled persistence sync. `cookie_details_for_script()` returns full `CookieSnapshot` for CookieStore API. `Set-Cookie` parsing via `cookie` crate. Domain/path matching. `SameSite=Lax` default. Third-party blocking (simplified domain comparison). Thread-safe via `Mutex`. **Cookie deletion**: `Max-Age=0` returns cookie with `expires = UNIX_EPOCH` + `persistent = false` (not `None`). Caller's `retain()` removes existing cookie. **Host-only matching**: `cookie_domain_matches()` uses exact match for `host_only = true`. **`stored_to_snapshot()`**: Shared conversion, lowercase `same_site`.
- **Redirect**: Follows 301/302/303/307/308 up to max 20. 301-303 change to GET. SSRF re-validation on each hop (skipped when `allow_private_ips`).
- **CORS**: `validate_cors()` checks `Access-Control-Allow-Origin` against request origin.
- **HTTPS upgrade**: `upgrade_to_https()` rewrites HTTP URLs to HTTPS.
- **MiddlewareChain**: Adapts plugin `NetworkMiddleware` trait to internal Request/Response types.
- **data_url**: RFC 2397 parser (plain text + base64).
- **ResourceLoader trait + SchemeDispatcher**: Routes http/https, data:, file:// with cookie injection and redirect following.
- **FetchHandle**: Wraps tokio current-thread `Runtime` + `NetClient`. `send_blocking(&self, request)` blocks via `rt.block_on(client.send(request))`. Used by elidex-js (`JsRuntime::with_fetch`) and elidex-navigation (`load_document`).
- **SSRF shared module**: `elidex_plugin::url_security` — `validate_url()` + `is_private_ip()`, shared by elidex-net and elidex-crawler.

## elidex-storage-core (browser_db module, M4-8.5)

- **BrowserDb**: `browser.sqlite` centralized database (8 tables). Typed sub-stores: `CookieStore<'db>`, `HistoryStore<'db>`, `BookmarkStore<'db>`.
- **OriginKey**: Typed struct `{scheme, host, port}` (not string wrapper). `from_url()`, `from_origin()`. IPv6 brackets in `Display`.
- **system_time_to_unix / unix_to_system_time**: Shared helpers in `browser_db/mod.rs`. `ts == 0` → `Some(UNIX_EPOCH)`, `ts < 0` → `None`.
- **History**: 2-table design (`urls` + `visits`). `record_visit` and `delete_range` use transactions. Frecency with time-decay buckets.
- **Cookies**: RFC 6265bis §5.7 full fields. `sync_all()` does DELETE + bulk INSERT in transaction. `PersistedCookie.delete()` includes `partition_key`.

## elidex-api-sw (router module, M4-8.5)

- **URLPattern**: `UrlPattern::pathname()` / `hostname_and_pathname()` return `Result<Self, String>`. Pattern syntax: `:name` → `([^/]+)`, `*` → `(.*)` with unique group names `*0`/`*1`. `test(&url::Url)` / `exec(&url::Url)` avoid re-parsing.
- **RouterSource**: `FetchEvent` / `Network` / `Cache(name)` / `RaceNetworkAndFetchHandler`.
- **ClientState enums**: `ClientType`, `FrameType`, `VisibilityState` (not strings).
