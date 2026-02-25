
# 9. Navigation & Page Lifecycle

## 9.1 Overview

Navigation is the end-to-end process of turning a URL into a rendered page. It spans multiple processes, involves security checks, process selection, resource loading, parsing, and rendering. This chapter unifies the navigation flow described across Ch. 15 (rendering pipeline), Ch. 11 (HTML parser), Ch. 10 (network), Ch. 13 (script), and Ch. 24 (browser shell) into a single coherent description.

```
User action (URL bar, link click, JS navigation)
  │
  ▼
Browser Process: NavigationController
  ├── URL resolution & validation
  ├── Security checks (CSP, Mixed Content, HTTPS upgrade)
  ├── Redirect following (301/302/307/308)
  ├── Site isolation: process selection
  │     ├── Same-site → reuse existing Renderer
  │     └── Cross-site → create new Renderer, swap
  ├── Response header processing
  │     ├── Content-Type → select handler (HTML, download, PDF, etc.)
  │     └── Cross-Origin policies (COOP, COEP, CORP)
  │
  ▼
Renderer Process: DocumentLoader
  ├── Create Document (ECS World)
  ├── HTML parser begins (Ch. 11)
  │     ├── Preload scanner runs ahead of main parser
  │     ├── Subresource fetching begins (CSS, JS, images)
  │     └── Script execution interleaves with parsing (Ch. 13)
  ├── DOMContentLoaded event
  ├── Subresource loading completes
  ├── load event
  │
  ▼
Page is interactive and fully rendered
```

## 9.2 Navigation Types

### 9.2.1 Classification

| Type | Trigger | Example |
| --- | --- | --- |
| Standard | URL bar, `<a href>`, `window.location` | `window.location.href = "https://example.com"` |
| Form submission | `<form>` submit | `<form action="/search" method="POST">` |
| History | Back/forward button, `history.back()` | Browser back button |
| Reload | Reload button, `location.reload()` | F5, Ctrl+R |
| Same-document | Fragment change, `pushState`, Navigation API `intercept()` | `history.pushState({}, "", "/page2")` |
| Restore | bfcache restore, tab reopen | Back button restoring cached page |

### 9.2.2 Same-Document vs. Cross-Document

Same-document navigations do not create a new Document or trigger a full page load:

| Mechanism | Effect |
| --- | --- |
| `history.pushState(state, "", url)` | Updates URL and history entry. No network request. No parsing. |
| `history.replaceState(state, "", url)` | Replaces current history entry. |
| `navigation.navigate(url, { history: "push" })` with `intercept()` | Navigation API handler runs. No default navigation. |
| Fragment change (`#section`) | Scrolls to element. `hashchange` event. |

Cross-document navigations create a new Document (new ECS World), triggering the full pipeline.

## 9.3 Navigation Flow: Cross-Document

### 9.3.1 Phase 1: Request (Browser Process)

```rust
pub struct NavigationController {
    /// Current navigation state
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

Steps:
1. **URL validation**: Resolve relative URLs. Validate scheme (http, https, blob, data, about).
2. **Navigation API event**: Fire `navigate` event in the source Renderer. If `intercept()` is called, abort cross-document navigation (becomes same-document).
3. **`beforeunload` event**: Fire in current page. User can cancel navigation.
4. **Security checks**: CSP `navigate-to`, Mixed Content blocking (http → https upgrade), HSTS.
5. **Network request**: Delegate to Network Process (Ch. 10). Follow redirects (up to 20, matching browsers).
6. **Response processing**: Check Content-Type. If not HTML → handle as download/PDF/image. Check COOP/COEP headers.
7. **Process selection**: Determine target Renderer Process (§9.4).

### 9.3.2 Phase 2: Commit (Process Handoff)

Once the response headers confirm an HTML document:

```
Browser Process                     Old Renderer              New Renderer
    │                                   │                         │
    ├── CommitNavigation ──────────────────────────────────────►│
    │   (url, response headers,         │                         │
    │    security origin, CSP)          │                         │
    │                                   │                         │
    │   [if cross-site swap]            │                         │
    ├── Unload old page ───────────────►│                         │
    │                                   ├── unload event          │
    │                                   ├── cleanup               │
    │◄── UnloadAck ─────────────────────┤                         │
    │                                   │                         │
    │                                   │                         ├── Create Document
    │                                   │                         ├── Begin parsing
    │   [response body streaming]       │                         │
    ├── DataPipe ──────────────────────────────────────────────►│
    │   (HTML bytes stream via          │                         ├── Parse HTML
    │    shared memory)                 │                         │
```

### 9.3.3 Phase 3: Loading (Renderer Process)

```rust
pub struct DocumentLoader {
    /// The new ECS World for this document.
    world: World,
    /// Document lifecycle state.
    state: DocumentState,
    /// Preload scanner (runs ahead of parser).
    preload_scanner: PreloadScanner,
    /// Subresource load tracker.
    pending_resources: HashSet<ResourceId>,
}

pub enum DocumentState {
    Loading,        // Parser active, subresources loading
    Interactive,    // Parser complete, subresources still loading
    Complete,       // All subresources loaded
}
```

Loading sequence:

```
Response body arrives (streaming)
  │
  ├── 1. Preload scanner extracts resource URLs
  │      (CSS, JS, images, fonts — issues early fetches)
  │
  ├── 2. HTML parser constructs DOM (ECS entities)
  │      ├── <link rel="stylesheet"> → fetch CSS, block rendering
  │      ├── <script src> → fetch JS
  │      │     ├── No defer/async: block parser, execute, resume
  │      │     ├── async: fetch parallel, execute when ready
  │      │     └── defer: fetch parallel, execute after parsing
  │      ├── <img src> → fetch image (non-blocking)
  │      └── <link rel="preload"> → high-priority fetch
  │
  ├── 3. Parser completes
  │      ├── document.readyState = "interactive"
  │      ├── Execute deferred scripts (in order)
  │      └── Fire DOMContentLoaded event
  │
  ├── 4. Subresources complete (images, iframes, etc.)
  │      ├── document.readyState = "complete"
  │      └── Fire load event (window.onload)
  │
  └── 5. Post-load
         ├── Idle callbacks (requestIdleCallback)
         └── Layout stability (LCP, CLS finalized)
```

### 9.3.4 Navigation Timing

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

Exposed to JavaScript via `performance.getEntriesByType("navigation")` (PerformanceNavigationTiming).

## 9.4 Process Selection (Site Isolation)

When a navigation commits, the Browser Process decides which Renderer Process handles the new document:

```rust
pub enum ProcessDecision {
    /// Reuse existing Renderer (same-site navigation)
    ReuseExisting(ProcessId),
    /// Create new Renderer (cross-site navigation)
    CreateNew,
    /// Use shared process (under memory pressure)
    UseShared(ProcessId),
}

impl NavigationController {
    fn select_process(&self, target_site: &Site, current_process: Option<ProcessId>) -> ProcessDecision {
        // Same-site: reuse if possible
        if let Some(pid) = current_process {
            if self.process_site(pid) == Some(target_site) {
                return ProcessDecision::ReuseExisting(pid);
            }
        }

        // Cross-site: need new process for isolation
        // Exception: memory-constrained mode allows sharing
        if self.under_memory_pressure() {
            if let Some(pid) = self.find_shared_process(target_site) {
                return ProcessDecision::UseShared(pid);
            }
        }

        ProcessDecision::CreateNew
    }
}
```

A "site" is defined as scheme + eTLD+1 (e.g., `https://example.com`). Subdomains share the same site.

## 9.5 Preload Scanner

The preload scanner runs in parallel with the HTML parser when the parser is blocked (typically waiting for a synchronous script to fetch and execute):

```rust
pub struct PreloadScanner {
    /// Lightweight tokenizer that extracts resource URLs
    /// without building a DOM.
    tokenizer: PreloadTokenizer,
}

impl PreloadScanner {
    /// Scan ahead in the HTML byte stream.
    /// Returns resource hints for early fetching.
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
                        priority: Priority::High,  // render-blocking
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

Preload hints are sent to the Network Process immediately, enabling parallel fetching while the parser is blocked on script execution.

## 9.6 History and Navigation API

### 9.6.1 Session History

Each browsing context maintains a session history:

```rust
pub struct SessionHistory {
    /// Ordered list of history entries
    entries: Vec<HistoryEntry>,
    /// Current index
    current_index: usize,
}

pub struct HistoryEntry {
    pub url: Url,
    pub title: String,
    pub state: Option<SerializedJsValue>,  // pushState/replaceState data
    pub scroll_position: (f64, f64),
    /// bfcache reference (if eligible)
    pub cached_page: Option<CachedPageRef>,
    /// Navigation API key (stable identifier)
    pub navigation_key: String,
    /// Navigation API id (unique per entry)
    pub navigation_id: String,
}
```

### 9.6.2 History API (Core)

```javascript
// Push new entry
history.pushState({ page: 2 }, "", "/page/2");

// Replace current entry
history.replaceState({ page: 2, updated: true }, "", "/page/2");

// Navigate
history.back();
history.forward();
history.go(-2);
```

`pushState` and `replaceState` trigger a `popstate` event on traversal (back/forward), but not on the push/replace itself.

### 9.6.3 Navigation API (Core)

The Navigation API provides a more structured interface for SPA navigation:

```javascript
// Intercept navigation
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

// Programmatic navigation
await navigation.navigate("/page/2", { state: { page: 2 } });

// Traverse
await navigation.back();

// Access entries
const entries = navigation.entries();
const current = navigation.currentEntry;
```

Key advantages over History API:
- `navigate` event fires for all navigation types (link clicks, form submissions, back/forward, `location.href`).
- `intercept()` cancels default navigation and runs a handler (SPA routing).
- `navigation.entries()` provides the full history stack (History API only has `length`).
- Entries have stable `key` (survives navigations) and unique `id`.

### 9.6.4 Internal Model

The Navigation API state lives in the Renderer Process as an ECS resource:

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

When the Browser Process commits a cross-document navigation, it sends the updated entry list to the new Renderer. For same-document navigations (intercept), the Renderer updates its own state.

## 9.7 Back/Forward Cache (bfcache)

### 9.7.1 Design

bfcache preserves entire pages in memory for instant back/forward navigation. When a user navigates away, instead of destroying the page, the Renderer Process is frozen and its state preserved.

```rust
pub struct BfCache {
    /// Cached pages, keyed by history entry
    entries: VecDeque<BfCacheEntry>,
    /// Maximum entries (default: 6)
    max_entries: usize,
    /// Total memory budget
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

### 9.7.2 Freeze / Resume

```
[Navigation away: page becomes eligible for bfcache]
  1. Fire `pagehide` event (persisted = true)
  2. Fire `freeze` event
  3. Suspend all timers (setTimeout, setInterval, requestAnimationFrame)
  4. Suspend all network requests
  5. Close WebSocket/WebRTC connections (ineligible if open)
  6. Pause media playback
  7. Renderer Process enters frozen state (event loop stops)

[Back/forward: restore from bfcache]
  1. Thaw Renderer Process (event loop resumes)
  2. Fire `resume` event
  3. Fire `pageshow` event (persisted = true)
  4. Resume timers, media, animations
  5. Re-establish any needed network connections
```

### 9.7.3 Eligibility

Not all pages are eligible for bfcache. Disqualifying conditions:

| Condition | Reason |
| --- | --- |
| Open WebSocket | Cannot freeze bidirectional connections |
| Active WebRTC peer connection | Cannot freeze media streams |
| `unload` event listener | Spec incompatibility (`unload` won't fire on restore) |
| `Cache-Control: no-store` | Page requested no caching |
| `window.opener` reference | Cross-window dependency |
| Active IndexedDB transaction | Cannot freeze mid-transaction |
| Pending `SharedWorker` messages | Cross-page shared state |
| `BroadcastChannel` with listeners | Cross-page communication |
| Unresolved `beforeunload` | Ambiguous user intent |

When a page is ineligible, the Renderer Process is destroyed normally on navigation.

### 9.7.4 Eviction

```rust
impl BfCache {
    fn evict_if_needed(&mut self) {
        // Evict oldest entries when over limits
        while self.entries.len() > self.max_entries
            || self.total_memory() > self.memory_budget
        {
            if let Some(entry) = self.entries.pop_front() {
                // Destroy the frozen Renderer Process
                self.destroy_process(entry.renderer_process_id);
            }
        }
    }
}
```

Default limits: 6 entries, 512 MB total. Configurable via browser settings. Under memory pressure, bfcache entries are evicted first (before active tabs).

## 9.8 Page Lifecycle Events

Complete event sequence for a standard cross-document navigation:

```
[Previous page]
  ├── beforeunload (can cancel)
  ├── pagehide
  │     ├── persisted=true → entering bfcache
  │     └── persisted=false → being destroyed
  ├── (if bfcache) freeze
  └── (if not bfcache) unload → destroy

[New page loading]
  ├── DOMContentLoaded (parsing complete, deferred scripts done)
  ├── load (all subresources loaded)
  ├── pageshow (persisted=false for new load)
  └── (idle) requestIdleCallback

[Restored from bfcache]
  ├── resume
  └── pageshow (persisted=true)
```

### 9.8.1 Page Visibility

`document.visibilityState` and `visibilitychange` event:

| State | Condition |
| --- | --- |
| `visible` | Tab is foreground, window not minimized |
| `hidden` | Tab is background or window minimized |

Visibility changes affect: timer throttling (background tabs), rAF suspension, media autoplay policy, FramePolicy (Ch. 15: background tabs may drop to OnDemand).

### 9.8.2 Page Lifecycle States (Extended)

```
         ┌──────────────────────┐
         │       Active         │ (visible, interactive)
         └──────┬───────────────┘
                │ tab hidden
                ▼
         ┌──────────────────────┐
         │       Hidden         │ (background, throttled)
         └──────┬───────────────┘
                │ user navigates away
                ▼
         ┌──────────────────────┐
  ┌──────│       Frozen         │ (bfcache, no execution)
  │      └──────┬───────────────┘
  │             │ evicted or ineligible
  │             ▼
  │      ┌──────────────────────┐
  │      │      Discarded       │ (process destroyed)
  │      └──────────────────────┘
  │
  └──── back/forward → Active
```

## 9.9 Redirects

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

            // 301/302: method may change to GET (browser compat)
            // 307/308: preserve original method
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

Additional redirect types:
- **Meta refresh**: `<meta http-equiv="refresh" content="5;url=...">` — handled by Renderer after parsing, sends navigation request to Browser Process.
- **JavaScript**: `window.location.href = "..."` — triggers standard navigation flow from Renderer.

## 9.10 Error Pages

| Error | Handling |
| --- | --- |
| DNS failure | Browser-generated error page |
| Connection timeout | Browser-generated error page |
| TLS certificate error | Interstitial warning (proceed option for non-HSTS) |
| HTTP 4xx/5xx | Render server response (server controls the error page) |
| Network offline | Browser-generated offline page (ServiceWorker may intercept) |
| CSP/CORS block | Console error, resource not loaded (page may partially render) |

Error pages are rendered by the BrowserShell (Ch. 24), not by web content.

## 9.11 elidex-app Navigation

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| URL bar navigation | Yes | No (app controls navigation) |
| Link clicks | Standard navigation | Configurable: allow, block, or intercept |
| History API | Full support | Full support |
| Navigation API | Full support | Full support |
| bfcache | Enabled | Disabled (single-page app model) |
| Preload scanner | Enabled | Enabled |
| Process selection | Site isolation | SingleProcess (default) |
| Error pages | Browser-generated | App-controlled |

In elidex-app, the embedder controls navigation policy via `NavigationPolicy` hooks (Ch. 26). The app can intercept all navigation requests and decide whether to allow, block, or handle them natively.
