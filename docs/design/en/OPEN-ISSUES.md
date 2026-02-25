
# Open Design Issues

This document tracks architectural areas that are identified as gaps or insufficiently developed in the current design. Items are prioritized by their impact on other design decisions (blocking dependencies are more urgent than self-contained gaps).

## Priority Definitions

- **P0 (Blocking)**: Other design decisions depend on this. Must be resolved before affected areas can be finalized.
- **P1 (Major Gap)**: Required for a functional browser, but can be designed somewhat independently.
- **P2 (Incomplete)**: Partially addressed, needs deeper treatment.

---

## OPEN-001: Multi-Process Architecture [P0] — RESOLVED

**Resolved in**: Ch. 5 (Process Architecture & Async Runtime)

**Summary**: Staged relaxation model — site-based process isolation in Phase 1–3 (SpiderMonkey era), relaxable to crash-isolation-only after full Rust migration. ProcessModel enum (SiteIsolation / PerTab / Shared / SingleProcess) configurable at build/startup. IPC abstracted behind trait (ProcessChannel / LocalChannel) enabling zero-cost process merging. elidex-app defaults to SingleProcess.

---

## OPEN-002: Media Pipeline (Audio/Video) [P1] — RESOLVED

**Resolved in**: Ch. 20 (Media Pipeline)

**Summary**: MediaDecoder trait with Rust/C library decoders for royalty-free codecs (VP8/VP9/AV1 via dav1d/libvpx, Opus, Vorbis/lewton, FLAC/claxon, MP3/minimp3) and platform decoders for patent-encumbered codecs (H.264/H.265 via AVFoundation/MediaFoundation/VA-API, AAC via platform) (ADR #34). Sandboxed Decoder Process for codec isolation. Hardware-accelerated decode with zero-copy GPU texture import to compositor. MediaPlayer coordinates demuxer → decoder → A/V sync → renderers. Audio master clock for A/V synchronization. MSE via SourceBuffer feeding same pipeline. EME with CDM trait and CDM Process defined but v1 ships without CDM (Clear Key possible as reference). Web Audio API with dedicated real-time audio thread, lock-free command queue, AudioGraph evaluation, AudioWorklet on audio thread, AudioParam sample-accurate automation. Media capture (getUserMedia/getDisplayMedia) integrated with permissions (Ch. 8). WebRTC interface defined (MediaStream model), full implementation deferred. Platform audio output abstraction (CoreAudio/WASAPI/PipeWire). Core/compat: MPEG-1/AVI = compat, WMV/FLV = not supported.

---

## OPEN-003: Storage & Cache Architecture [P1] — RESOLVED

**Resolved in**: Ch. 22 (Storage & Cache Architecture)

**Summary**: Two-category model: browser-owned data in centralized browser.sqlite (cookies, HSTS, history, bookmarks, settings, permissions), web-content data in per-origin isolated SQLite databases (elidex.storage, IndexedDB, Cache API, OPFS). StorageBackend trait abstracts SQLite dependency. OriginStorageManager coordinates per-origin access with IPC-mediated security verification. HTTP cache with double-keyed partitioning. Memory caches (image decode, style sharing, font glyph, bytecode). bfcache for instant back/forward navigation. QuotaManager with LRU eviction and persistent storage grants.

---

## OPEN-004: Navigation & Page Lifecycle [P2] — RESOLVED

**Resolved in**: Ch. 9 (Navigation & Page Lifecycle)

**Summary**: Unified end-to-end navigation flow across Browser Process (NavigationController: URL validation, security checks, redirect following, site-isolation process selection) and Renderer Process (DocumentLoader: Document creation, HTML parsing with preload scanner, subresource loading, DOMContentLoaded/load events). Navigation types: standard, form, history, reload, same-document, restore. History API (Core) and Navigation API (Core, recommended for SPAs). bfcache via Renderer Process freeze with memory-bounded eviction (ADR #35, default 6 entries / 512 MB). Full page lifecycle event sequence (beforeunload → pagehide → freeze → resume → pageshow). Preload scanner for parallel resource fetching during parser-blocking scripts. NavigationTiming exposed via PerformanceNavigationTiming. Site isolation: same-site reuses Renderer, cross-site creates new Renderer.

---

## OPEN-005: GPU Process & Compositor Details [P2] — RESOLVED

**Resolved in**: Ch. 15 expansion (Sections 15.4–15.11), Ch. 6 (Thread Model)

**Summary**: Layer tree as independent structure (not ECS components) with promotion criteria and explosion prevention. Display list intermediate representation with delta updates and arena allocation. Vello on wgpu as direct dependency (no trait abstraction — ADR #26) with isolated contact surface. Phased texture management: individual textures → atlas for small images → unified management. Frame scheduling via FramePolicy enum (Vsync/Continuous/OnDemand/FixedRate) accommodating both browser and app use cases. Pipelined main thread + compositor. VRR support. Compositor-driven scroll, animations (transform/opacity/filter), and scroll-linked effects.

---

## OPEN-006: HTTP/HTTPS Implementation Details [P1] — RESOLVED

**Resolved in**: Ch. 10 expansion (Sections 10.5–10.12)

**Summary**: HttpTransport trait abstracts hyper dependency. Full protocol negotiation (HTTP/1.1, HTTP/2, HTTP/3), connection management, TLS (rustls + aws-lc-rs), HTTPS-Only default, security-first defaults (DoH, third-party cookie blocking), CORS enforcement in Network Process, cookie management with CHIPS, security response headers, proxy support including PAC.

---

## OPEN-007: Image Decode Pipeline [P1] — RESOLVED

**Resolved in**: Ch. 18 (Image Decode Pipeline)

**Summary**: Full image decode pipeline from network bytes to GPU texture. Format core/compat classification (PNG/JPEG/WebP/AVIF/GIF/APNG core; ICO/BMP/TIFF compat). ImageDecoder trait with Rust-crate defaults, platform-native decoders swappable (ADR #31). Off-main-thread decode on rayon pool (Ch. 6). Header-first layout, progressive JPEG rendering, downscaled JPEG decode. Responsive images (srcset/picture source selection). Lazy loading via IntersectionObserver. Image decode cache (LRU, 128MB default) coordinated with HTTP cache (Ch. 22) and GPU textures (Ch. 15). Animated image scheduling via independent ImageAnimationScheduler (not Web Animations). Blob URL and data URL support. elidex-app configurable decoder strategy and cache budget.

---

## OPEN-008: SVG Rendering [P1] — RESOLVED

**Resolved in**: Ch. 19 (SVG Rendering)

**Summary**: Inline SVG elements as ECS entities with SVG-specific components (SvgGeometry, SvgTransform, SvgViewport). SvgLayoutSystem for coordinate-based geometry (separate from CSS box layout). DisplayItem extended with SvgPath/SvgText for unified paint pipeline through Vello (ADR #32). SVG-as-image rendered via direct Vello path (no ECS), cached as bitmap in image decode cache (Ch. 18). SVG filter effects as GPU render pass DAG, shared with CSS filter implementation. SVG gradients/patterns mapped to Vello brushes. SVG text uses shared text pipeline (Ch. 16) with SVG-specific positioning. SMIL classified as compat, translated to WAAPI by elidex-compat. Clipping, masking, `<use>` element reuse.

---

## OPEN-009: Animation & Scroll Architecture [P1] — RESOLVED

**Resolved in**: Ch. 17 (Animation & Scroll Architecture), Ch. 15 §15.9 (Compositor-Driven Operations)

**Summary**: FrameProducer coordinates between event loop (Ch. 5) and rendering pipeline (Ch. 15). Web Animations API as unified internal model — CSS Transitions and CSS Animations are translated to WAAPI instances at creation time (ADR #29). AnimationEngine uses ECS parallel queries (ActiveAnimations component, DocumentTimeline resource) for tick — C+B hybrid pattern (ADR #30). Compositor promotion flow with demotion handling. PropertyInterpolator with Oklab color, transform decompose/interpolate/recompose. Animation composition stack (Replace/Add/Accumulate). Smooth scrolling, scroll snap, scroll anchoring. ScrollTimeline for scroll-linked animations. Full animation event lifecycle (transition/animation/WAAPI events). elidex-app can drive FrameProducer directly without browser event loop.

---

## OPEN-010: Permissions Model (Browser Mode) [P2] — RESOLVED

**Resolved in**: Ch. 8 expansion (Sections 8.3–8.8)

**Summary**: Unified Permission enum shared across browser and app modes; only grant mechanism differs (runtime user prompt vs. build-time manifest). PermissionManager in Browser Process as single authority. Three-layer check: Permissions-Policy (document) AND iframe allow (frame) AND origin-level decision. Per-origin persistent storage in browser.sqlite (Ch. 22). Permissions API (navigator.permissions) with onchange events. Prompt UI delegated to BrowserShell (Ch. 24) via PermissionPrompter trait. App mode uses static AppCapability with extended capabilities (FileRead/Write, NetworkUnrestricted, ProcessSpawn, etc.). Capability audit logging in debug builds.

---

## OPEN-011: Async I/O Runtime & Event Loop Integration [P0] — RESOLVED

**Resolved in**: Ch. 5 (Process Architecture & Async Runtime)

**Summary**: Per-process runtime strategy. Renderer: custom elidex event loop owns main thread, tokio current_thread reactor as I/O backend. Network/Browser: tokio multi-thread. AsyncRuntime trait abstraction preserves future replacement option. Renderer event loop integrates JS event loop (Ch. 13), IPC, I/O, and vsync in a single unified loop. Long-term: Renderer's proven event loop can expand to replace tokio in other processes.

---

## OPEN-012: Persistence Infrastructure [P1] — RESOLVED

**Resolved in**: Ch. 22 (Storage & Cache Architecture)

**Summary**: Unified two-layer architecture: StorageBackend trait (low-level, abstracts SQLite) and domain traits (high-level, CookiePersistence/HistoryStore/BookmarkPersistence). SQLite with WAL mode, secure_delete, and hardened pragmas as initial backend. Browser-owned data centralized in browser.sqlite; web-content data in per-origin isolated databases. Profile isolation through directory structure. elidex-app gets configurable storage directory and backend.

---

## OPEN-013: File API & Streams [P1] — RESOLVED

**Resolved in**: Ch. 21 (File API & Streams)

**Summary**: BlobStore in Browser Process with hybrid memory/disk backing (inline ≤256 KB, disk spill >256 KB). Blob URL registry per-origin. FileReader classified as compat (modern core: `blob.text()`, `blob.arrayBuffer()`, `blob.stream()`). Streams API backed by Rust `ByteStream` (async Stream trait) with ScriptSession bridge to JS ReadableStream/WritableStream — pull protocol provides natural backpressure. Rust-to-Rust fast path enables streaming without JS boundary crossing (e.g., fetch → decompress → OPFS write). Compression Streams via native flate2/miniz_oxide. File System Access API integrates with permission model (Ch. 8) and platform file dialogs (Ch. 23). OPFS with SyncAccessHandle via shared memory mapped file for zero-IPC read/write (ADR #33) — critical for SQLite-on-web. OPFS storage under quota system (Ch. 22).

---

## OPEN-014: Embedding API [P1] — RESOLVED

**Resolved in**: Ch. 26 (Embedding API)

**Summary**: Rust-native embedding API for elidex-app. Engine struct with builder pattern, EngineConfig (ProcessMode SingleProcess/MultiProcess, FeatureFlags, CodecConfig). View struct for web content areas with ViewConfig (ViewContent: URL/HTML/File/Blank, SurfaceConfig: CreateWindow/AttachToWindow/Headless via raw-window-handle, PermissionConfig, NavigationPolicy). Native↔Web bridge: expose_function (Rust→JS via serde, async call from JS), bidirectional message channel, window.__elidex namespace. Custom ResourceLoader trait for intercepting resource requests (app:// scheme, embedded assets). Multi-view support (independent Views sharing Engine). Headless mode with Vello CPU backend for SSR/testing/screenshots. DevTools via CDP server. C API via cbindgen for non-Rust embedders. API stability tiers (Stable/Semi-stable/Unstable) with semantic versioning.

---

## OPEN-015: Intra-Process Thread Model [P1] — RESOLVED

**Resolved in**: Ch. 6 (Intra-Process Thread Model)

**Summary**: Renderer has 4 thread classes: main thread (ECS DOM owner, event loop, script), compositor thread (independent layer compositing, scroll, GPU submit), rayon pool (parallel style/layout/decode), Web Worker threads (1:1 OS thread per Worker). Compositor abstracted behind FrameSource trait enabling staged B→C migration: Approach B (DisplayList message passing, Phase 1–3) transitions to Approach C (shared ECS reads, Phase 4+) when process isolation relaxes. ECS concurrency model based on phase-separated access (no locks in Approach B, double-buffered layers in Approach C). Browser/Network processes use tokio multi-thread with spawn_blocking for SQLite. GPU process: 1 GPU thread + 1 IPC thread.

**Blocks**: ~~OPEN-005~~ (compositor threading), ~~OPEN-009~~ (off-main-thread scroll/animation)
---

## Minor Gaps (absorbable into existing chapters or OPEN items)

These are not large enough for their own OPEN issue, but should be addressed when their parent chapter or related OPEN item is worked on.

| Area | Absorb Into | Notes |
| --- | --- | --- |
| Web Fonts (@font-face loading, FOIT/FOUT, font-display, variable fonts) | Ch. 16 expansion | Current Ch. 16 covers shaping only. Font loading is a render-blocking resource and affects perceived performance. Network fetch → decode → shaping pipeline. |
| Canvas 2D implementation architecture | Ch. 12 expansion | Listed as P0 Web API but described in one line. Immediate-mode rendering model differs fundamentally from retained-mode DOM. GPU acceleration (OffscreenCanvas), worker thread rendering. |
| Memory management strategy | OPEN-001 / OPEN-003 | Per-tab memory budgets, memory pressure handling (OS low-memory notifications), image eviction, tab discarding. Cross-cutting concern across processes. |
| Selection / editing / contentEditable | Ch. 13 (future OM) | Mentioned as "future OM plugin" in ScriptSession. Complex (cursor movement, range selection, input events, execCommand). Not blocking but significant implementation scope. |
| Scroll behavior details | ~~OPEN-005~~ / ~~OPEN-009~~ | smooth scrolling, scroll anchoring, overscroll-behavior, scroll-snap. Covered in Ch. 15 §15.9 and Ch. 17 §17.7. |
| Forms & input widgets | Ch. 23 expansion | Crate listed in Ch. 2 (elidex-html-forms). Platform-native date/color pickers (Ch. 23), validation API, autofill integration. |
| Printing / @media print | Deferred | Needed eventually but low design impact. Separate layout pass with print-specific styles. |
| URL scheme dispatch (mailto:, tel:, custom) | Ch. 13 / Ch. 14 expansion | Browser needs to hand off URLs to external apps. Platform-dependent (xdg-open, NSWorkspace, ShellExecute). |
| PDF display (inline or external) | Ch. 14 expansion or new minor section | Inline PDF viewer (pdf.js-equivalent) or delegate to OS. Significant feature but self-contained. |
| Crash reporting | OPEN-001 expansion | Process crash capture, minidump generation, optional upload. Especially important for Renderer crash recovery. |
| Engine telemetry / logging | Ch. 27 (testing) expansion | Structured logging (tracing crate), performance counters, error telemetry pipeline. Ch. 5 mentions telemetry for parser patterns but no general framework. |
| Server-Sent Events (EventSource) | Ch. 12 expansion | Simpler than WebSocket. HTTP-based streaming. Depends on OPEN-011 (async runtime). |
| Browser automation protocol (WebDriver / CDP) | Ch. 24 (DevTools) expansion | Selenium, Playwright, Puppeteer rely on WebDriver or Chrome DevTools Protocol. WebDriver BiDi is the emerging standard unifying both. Shares infrastructure with DevTools (Ch. 24 §24.4). elidex needs at least one automation protocol for CI testing of the engine itself (Ch. 27). |
| Auto-update mechanism | Deferred | Critical for production deployment but not an engine architecture concern. Separate updater process, delta updates, rollback. Product-level infrastructure. |
| Spectre/Meltdown & side-channel mitigations | OPEN-001 expansion | Site Isolation (process-per-site), high-resolution timer restrictions, SharedArrayBuffer gating behind COOP/COEP. Naturally addressed when process model is designed. |
| Performance Observer / Reporting APIs | Ch. 12 expansion | PerformanceObserver, Long Tasks API, Reporting API (CSP violation reports, deprecation reports). Instrumentation points throughout the engine. |

---

## Summary Matrix

> **Note**: All 15 items are resolved. Chapter numbers reflect the current post-refactoring numbering.

| ID | Title | Priority | Status | Depends On | Estimated Scope |
| --- | --- | --- | --- | --- | --- |
| OPEN-001 | Multi-Process Architecture | P0 | **Resolved (Ch. 5)** | — | — |
| OPEN-002 | Media Pipeline | P1 | **Resolved (Ch. 20)** | ~~OPEN-001~~ | — |
| OPEN-003 | Storage & Cache | P1 | **Resolved (Ch. 22)** | ~~OPEN-001~~ | — |
| OPEN-004 | Navigation & Lifecycle | P2 | **Resolved (Ch. 9)** | ~~OPEN-001~~, ~~003~~ | — |
| OPEN-005 | GPU & Compositor | P2 | **Resolved (Ch. 15)** | ~~OPEN-001~~ | — |
| OPEN-006 | HTTP/HTTPS Implementation | P1 | **Resolved (Ch. 10)** | ~~OPEN-001~~ | — |
| OPEN-007 | Image Decode Pipeline | P1 | **Resolved (Ch. 18)** | ~~OPEN-001~~, ~~003~~, ~~005~~ | — |
| OPEN-008 | SVG Rendering | P1 | **Resolved (Ch. 19)** | ~~OPEN-005~~ | — |
| OPEN-009 | Animation & Scroll | P1 | **Resolved (Ch. 17)** | ~~OPEN-005~~ | — |
| OPEN-010 | Permissions Model | P2 | **Resolved (Ch. 8)** | Ch. 8, 23, 24 | — |
| OPEN-011 | Async I/O Runtime | P0 | **Resolved (Ch. 5)** | ~~OPEN-001~~ | — |
| OPEN-012 | Persistence Infrastructure | P1 | **Resolved (Ch. 22)** | ~~OPEN-001~~, ~~003~~ | — |
| OPEN-013 | File API & Streams | P1 | **Resolved (Ch. 21)** | ~~OPEN-001~~, ~~011~~, ~~012~~ | — |
| OPEN-014 | Embedding API | P1 | **Resolved (Ch. 26)** | ~~OPEN-001~~, Ch. 23, 24 | — |
| OPEN-015 | Intra-Process Thread Model | P1 | **Resolved (Ch. 6)** | ~~OPEN-001~~, ~~011~~ | — |
