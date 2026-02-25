
# 10. Network Architecture

The network layer is designed as a pluggable pipeline rather than a fixed implementation. This serves three needs: protocol evolution (HTTP/3 today, HTTP/4 tomorrow), dual-use flexibility (browser needs full networking, app mode may need none or custom schemes), and extensibility for third-party middleware.

## 10.1 Resource Loader Abstraction

All resource fetching goes through a ResourceLoader trait, enabling custom URL schemes and protocol implementations:

```rust
pub trait ResourceLoader: Send + Sync {
    /// URL schemes this loader handles
    fn schemes(&self) -> &[&str];

    /// Fetch a resource
    async fn fetch(&self, request: Request) -> Result<Response, FetchError>;
}
```

| Loader | Schemes | Notes |
| --- | --- | --- |
| HttpLoader | http://, https:// | Built-in. hyper + rustls + h3. Handles redirects, cookies, CORS. |
| FileLoader | file:// | Built-in. Local file access (sandboxed in browser mode). |
| DataLoader | data: | Built-in. Inline data URIs (base64/plaintext). |
| AppResourceLoader | app://, elidex:// | elidex-app only. Developer provides custom resource resolution. Analogous to Tauri’s tauri:// scheme. |

App developers register custom loaders at startup:

```rust
let app = elidex_app::App::new()
    .resource_loader(AppResourceLoader::new(\|path\| {
        // Serve UI assets from embedded resources
        Assets::get(path).map(\|a\| Response::ok(a.data))
    }));
```

## 10.2 Network Middleware Pipeline

All requests and responses pass through a middleware chain, enabling inspection, modification, and blocking without modifying the core networking code:

```rust
pub trait NetworkMiddleware: Send + Sync {
    fn name(&self) -> &str;

    /// Intercept before request is sent
    fn on_request(&self, req: &mut Request) -> MiddlewareAction {
        MiddlewareAction::Continue
    }

    /// Intercept after response is received
    fn on_response(&self, req: &Request, res: &mut Response) -> MiddlewareAction {
        MiddlewareAction::Continue
    }
}

pub enum MiddlewareAction {
    Continue,                // Pass to next middleware
    Block,                   // Drop the request/response
    Redirect(Url),           // Redirect to another URL
    MockResponse(Response),   // Return synthetic response (skip fetch)
}
```

The middleware pipeline is ordered; each middleware sees the request/response after all preceding middleware have processed it:

```
Request ─▶ [DevTools Logger] ─▶ [Content Blocker] ─▶ [Custom Headers] ─▶ HTTP Fetch
                                                                          │
Response ◀─ [DevTools Logger] ◀─ [Content Filter] ◀──────────────────◀─┘
```

## 10.3 Middleware Use Cases

| Use Case | Provider | Implementation |
| --- | --- | --- |
| Network monitoring (DevTools) | elidex-browser | Logs request/response timing, headers, and body size in on_request and on_response. Feeds DevTools network tab. |
| Content blocking (ads, trackers) | Third-party / extension | URL pattern matching against filter lists (e.g., EasyList format). Returns MiddlewareAction::Block for matched URLs. Elidex provides the hook; blocking policy is external. |
| Privacy protection | Third-party / extension | Strips tracking query parameters (utm_*, fbclid), modifies Referer headers, blocks known fingerprinting endpoints. |
| API mocking (testing) | elidex-app developer | Returns MockResponse for matched API endpoints during development/testing. No actual network calls needed. |
| Request modification | elidex-app developer | Injects auth headers, API keys, or custom headers into outgoing requests without modifying application code. |
| Caching override | Either | Custom caching strategies (e.g., aggressive offline caching for app mode, bypass cache for development). |

## 10.4 Design Principle: Engine Neutrality

Elidex deliberately provides the middleware mechanism without bundling any content-blocking policy. This is a conscious design decision:

**What elidex provides: ** The NetworkMiddleware trait, the pipeline executor, and a reference DevTools logging middleware.

**What elidex does not provide: ** Filter lists, ad-blocking rules, tracker databases, or any opinion on what should be blocked. These are the domain of extensions and third-party middleware crates.

This keeps elidex out of the political and legal complexities of content blocking while ensuring that users and extension developers have full power to implement whatever policy they choose. The middleware API is expressive enough to support everything from simple URL blocking to full request/response rewriting.

## 10.5 HTTP Client Architecture

### 10.5.1 HttpTransport Trait

The HTTP client implementation is abstracted behind a trait, decoupling elidex from any specific HTTP library. The initial implementation uses hyper + rustls + h3, but the trait boundary ensures future replacement without affecting the rest of the engine:

```rust
pub trait HttpTransport: Send + Sync {
    /// Send a request and receive the response headers.
    /// The body is returned as a stream for incremental processing.
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;

    /// Check if the transport supports a given protocol version.
    fn supports(&self, protocol: HttpProtocol) -> bool;

    /// Connection pool statistics (for DevTools and diagnostics).
    fn pool_stats(&self) -> ConnectionPoolStats;
}

pub enum HttpProtocol {
    Http11,
    H2,
    H3,
}
```

The initial implementation:

```rust
pub struct HyperTransport {
    http11_client: hyper::Client<HttpsConnector>,   // HTTP/1.1 + HTTP/2 (via ALPN)
    h3_client: h3::client::Connection,              // HTTP/3 (QUIC)
    pool: ConnectionPool,
}

impl HttpTransport for HyperTransport { /* ... */ }
```

If hyper is superseded in the future, only `HyperTransport` is replaced. All code above — ResourceLoader, middleware pipeline, Renderer fetch requests — programs against `dyn HttpTransport` and is unaffected.

### 10.5.2 Protocol Negotiation

Elidex supports HTTP/1.1, HTTP/2, and HTTP/3 with automatic negotiation:

```
Client ──── DNS lookup (DoH) ────▶ IP addresses
  │
  ├── Attempt HTTPS (TLS 1.3, ALPN: h2, http/1.1)
  │     ├── If h2 negotiated → HTTP/2 multiplexed connection
  │     └── If http/1.1 negotiated → HTTP/1.1 connection
  │
  ├── If Alt-Svc header received with h3 → attempt QUIC
  │     └── If QUIC handshake succeeds → HTTP/3 connection
  │
  └── Fallback: if HTTPS fails and user/policy allows → HTTP (with warning)
```

| Protocol | Status | Notes |
| --- | --- | --- |
| HTTP/3 (QUIC) | Preferred when available | Lower latency (0-RTT), better performance on lossy networks. Discovered via Alt-Svc header or DNS HTTPS record. |
| HTTP/2 | Default for HTTPS | Multiplexed streams over single TCP connection. Eliminates head-of-line blocking at HTTP layer. |
| HTTP/1.1 | Fallback | Required for servers that don't support HTTP/2. Many internal/legacy servers. |
| HTTP (plaintext) | Discouraged | HTTPS-Only mode is default (see Section 10.6.2). Plaintext HTTP requires explicit user opt-in or HSTS preload miss. |

Unlike engine-layer core/compat where legacy features can be compile-time excluded, HTTP protocol support depends on server capability. All three protocol versions remain available; the engine automatically selects the best supported option.

### 10.5.3 Connection Management

The Network Process (Ch. 5) manages a centralized connection pool shared across all tabs:

```rust
pub struct ConnectionPool {
    /// Per-origin connection limits
    max_per_origin: usize,          // Default: 6 for HTTP/1.1, 1 for HTTP/2 (multiplexed)
    max_total: usize,               // Default: 256
    idle_timeout: Duration,          // Default: 90s
    /// Connection coalescing for HTTP/2: origins sharing a TLS certificate
    /// can reuse the same connection
    coalesce_h2: bool,              // Default: true
}
```

| Feature | Behavior |
| --- | --- |
| Keep-alive | Connections held open for `idle_timeout`. Reused for subsequent requests to the same origin. |
| HTTP/2 multiplexing | Single TCP connection per origin carries all concurrent requests as multiplexed streams. Eliminates the 6-connection limit. |
| Connection coalescing | HTTP/2 connections can be reused across origins sharing the same TLS certificate and IP. Reduces connection overhead for CDNs. |
| Preconnect | `<link rel="preconnect">` and `dns-prefetch` hints trigger early connection setup. DNS, TCP, TLS handshake completed before first request. |

### 10.5.4 Request Prioritization

HTTP/2 and HTTP/3 support stream prioritization, allowing the browser to signal which resources are most important:

| Priority | Resource Types | Mechanism |
| --- | --- | --- |
| Highest | HTML document, CSS (render-blocking) | HTTP/2: PRIORITY frame, weight 256. HTTP/3: Urgency 0 (Extensible Priorities). |
| High | JS (parser-blocking), web fonts | Urgency 1–2. |
| Medium | Images above the fold, preload resources | Urgency 3. |
| Low | Images below the fold, prefetch | Urgency 4–5. `fetchpriority="low"` attribute. |
| Lowest | Background fetch, analytics beacons | Urgency 6–7. |

The `fetchpriority` HTML attribute and Priority Hints API allow web developers to override default priorities.

## 10.6 TLS & Certificate Handling

### 10.6.1 TLS Implementation

TLS is provided by rustls, a Rust-native TLS library. This avoids the C dependency on OpenSSL and benefits from Rust's memory safety for the most security-critical code in the network stack:

| Aspect | Decision | Rationale |
| --- | --- | --- |
| Library | rustls + aws-lc-rs (crypto backend) | Rust-native. Memory-safe. aws-lc-rs provides FIPS-validated cryptography for enterprise use cases. |
| TLS versions | TLS 1.3 preferred, TLS 1.2 supported | TLS 1.2 is still widely required (many corporate environments, legacy servers). TLS 1.0/1.1 not supported (deprecated). |
| Certificate verification | webpki (Rust-native) | Standard X.509 chain verification against OS trust store or bundled Mozilla root certificates. |
| Certificate Transparency | Enforced for publicly-trusted certificates | SCT (Signed Certificate Timestamp) verification. Detects misissued certificates. |

### 10.6.2 HTTPS-Only Mode

HTTPS-Only is the default for elidex-browser. Plaintext HTTP requests are automatically upgraded:

```
User navigates to http://example.com
  → Engine rewrites to https://example.com
  → If HTTPS connection succeeds → proceed normally
  → If HTTPS connection fails → show interstitial warning
     → User can opt to proceed over HTTP (per-site exception)
```

This is a "secure by default" stance. Unlike engine-layer core/compat where legacy is removed for performance, here legacy (HTTP) is retained for practical compatibility but requires explicit user action.

## 10.7 Security-First Defaults

Elidex's network layer applies the same philosophy as the engine's graduated degradation: the secure modern path is the default, with legacy available as an explicit fallback.

| Feature | elidex Default | Industry Status | Configuration |
| --- | --- | --- | --- |
| HTTPS-Only mode | ON | Chrome/Firefox: OFF by default, opt-in | Users can disable per-site or globally |
| DNS over HTTPS (DoH) | ON (Cloudflare or system resolver) | Chrome: varies by region. Firefox: ON in US. | Configurable resolver. Enterprises can disable via policy. |
| Third-party cookies | Blocked | Chrome: phasing out. Firefox: blocked (ETP). Safari: blocked (ITP). | Sites can request Storage Access API for exceptions |
| HSTS preload list | Bundled, auto-updated | Standard across browsers | — |
| HSTS dynamic entries | Respected, persisted | Standard | Per-profile persistence |
| Mixed content | Blocked (active), upgraded (passive) | Chrome: auto-upgrades passive. Firefox: similar. | Active mixed content (scripts, iframes) always blocked. Passive (images) auto-upgraded to HTTPS. |

### 10.7.1 DNS Resolution

```rust
pub trait DnsResolver: Send + Sync {
    async fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, DnsError>;
}

pub struct DohResolver {
    endpoint: Url,          // e.g., https://1.1.1.1/dns-query
    fallback: Option<Box<dyn DnsResolver>>,  // System resolver as fallback
    cache: DnsCache,         // TTL-based in-memory cache
}

pub struct SystemResolver;   // OS-provided resolution (getaddrinfo)
```

| Resolver | Default | Notes |
| --- | --- | --- |
| DoH (DNS over HTTPS) | Browser default | Encrypted DNS queries. Prevents ISP/network-level DNS manipulation. |
| System resolver | elidex-app default, browser fallback | Uses OS DNS configuration. Respects /etc/resolv.conf, corporate DNS. |

The DnsResolver trait allows enterprise deployments to inject custom resolvers (e.g., corporate DNS with internal domain resolution).

## 10.8 CORS Enforcement

Cross-Origin Resource Sharing is enforced by the Network Process, not the Renderer. This ensures that even a compromised Renderer cannot bypass CORS:

```
Renderer sends Fetch request
  → Network Process receives RendererToNetwork::Fetch
  → Check request type:
     ├── Same-origin → proceed
     ├── Simple cross-origin → send request, check Access-Control-Allow-Origin in response
     └── Non-simple cross-origin → send OPTIONS preflight first
         ├── Preflight succeeds → send actual request
         └── Preflight fails → return CORS error to Renderer (no response body exposed)
```

| CORS Aspect | Implementation |
| --- | --- |
| Preflight cache | Cached per (origin, method, headers) tuple. `Access-Control-Max-Age` header respected. Stored in Network Process memory. |
| Credentialed requests | `Access-Control-Allow-Credentials: true` required. `Access-Control-Allow-Origin` must not be wildcard. |
| Opaque responses | `no-cors` mode returns opaque response (status 0, empty headers/body) to Renderer. Body is available to Service Worker cache but not to scripts. |

## 10.9 Cookie Management

### 10.9.1 Cookie Jar

The cookie jar lives in the Network Process, enforcing cookie policy at the network layer:

```rust
pub struct CookieJar {
    store: CookieStore,              // Per-origin cookie storage
    policy: CookiePolicy,
}

pub struct CookiePolicy {
    block_third_party: bool,         // Default: true
    same_site_default: SameSite,     // Default: Lax (matches Chrome behavior)
    require_secure_for_same_site_none: bool,  // Default: true
    partitioned_cookies: bool,       // CHIPS support. Default: true
}
```

### 10.9.2 Cookie Classification

| Cookie Type | Default Behavior | Notes |
| --- | --- | --- |
| First-party, SameSite=Lax | Allowed | Standard modern cookie behavior |
| First-party, SameSite=Strict | Allowed | More restrictive — not sent on cross-site navigation |
| First-party, SameSite=None; Secure | Allowed | Must be Secure (HTTPS only) |
| Third-party | Blocked | Default. Sites can use Storage Access API to request access. |
| Partitioned (CHIPS) | Allowed | Per-top-level-site partitioning. Privacy-preserving third-party state. |

### 10.9.3 Security Response Headers

The Network Process parses and enforces security headers before forwarding responses to the Renderer:

| Header | Enforcement |
| --- | --- |
| Content-Security-Policy (CSP) | Parsed into policy object. Renderer enforces inline script/style blocking, eval restrictions, source allowlists. Violations reported via Reporting API. |
| Strict-Transport-Security (HSTS) | Dynamic entries stored in persistence layer (Ch. 22). Subsequent requests to the domain automatically upgraded to HTTPS. |
| X-Frame-Options | Enforced on navigation. DENY/SAMEORIGIN prevent framing by cross-origin pages. Superseded by CSP `frame-ancestors` but still supported. |
| Permissions-Policy | Parsed. Controls which features (camera, microphone, geolocation) are available to the document and its iframes. |
| Cross-Origin-Opener-Policy (COOP) | Determines whether the document gets its own browsing context group. Required for SharedArrayBuffer access. |
| Cross-Origin-Embedder-Policy (COEP) | Requires all subresources to opt-in to cross-origin loading. Required for SharedArrayBuffer access. |

## 10.10 Response Decompression

| Encoding | Support | Library |
| --- | --- | --- |
| Brotli | Core | brotli (Rust-native) |
| gzip / deflate | Core | flate2 (Rust-native) |
| zstd | Core | zstd (Rust binding) |

Accept-Encoding header is automatically constructed based on supported encodings. Decompression is streaming (integrated with response body stream) to avoid buffering entire responses in memory.

## 10.11 Proxy Support

```rust
pub enum ProxyConfig {
    None,
    Http(Url),                    // HTTP CONNECT proxy
    Socks5(Url),                   // SOCKS5 proxy
    Pac(Url),                      // PAC script URL (auto-configuration)
    System,                        // Use OS proxy settings
}
```

PAC script evaluation runs in a sandboxed JS environment (the script engine, Ch. 14) within the Network Process. This is a standard browser requirement for enterprise proxy auto-configuration.

## 10.12 elidex-app Network Configuration

In elidex-app mode, the application developer has fine-grained control over the network stack:

```rust
let app = elidex_app::App::new()
    // Custom DNS resolver (e.g., for internal service discovery)
    .dns_resolver(CustomDnsResolver::new())
    // Disable third-party cookie blocking (app controls all content)
    .cookie_policy(CookiePolicy { block_third_party: false, ..Default::default() })
    // Custom TLS certificate for internal APIs
    .add_root_certificate(internal_ca)
    // Proxy configuration
    .proxy(ProxyConfig::Http("http://proxy.internal:8080".parse().unwrap()))
    .build();
```

In SingleProcess mode (Ch. 5), the Network "Process" is a tokio task within the application's process. The HttpTransport trait, middleware pipeline, and all network configuration work identically — the only difference is that IPC is replaced by in-process channels.


