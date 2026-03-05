# Elidex Architecture Design Document

Elidex is an experimental browser engine written in Rust that eliminates legacy backward compatibility to achieve maximum performance. It serves as both a web browser and a lightweight application runtime (elidex-app) competing with Electron and Tauri.

## Document Structure

### Part I — Overview
| Ch. | Title | Description |
| --- | --- | --- |
| 1 | [Executive Summary](01-executive-summary.md) | Project vision, core principles, architecture at a glance |
| 2 | [Architecture Overview](02-architecture-overview.md) | High-level system architecture and component relationships |
| 3 | [Roadmap](03-roadmap.md) | Development phases and milestones |
| 4 | [Risks & Mitigations](04-risks.md) | Technical, scope, and ecosystem risks |

### Part II — Core Architecture
| Ch. | Title | Description |
| --- | --- | --- |
| 5 | [Process Architecture & Async Runtime](05-process-async.md) | Multi-process model, Browser/Renderer split, tokio runtime |
| 6 | [Thread Model](06-thread-model.md) | Intra-process threading: main, compositor, rayon pool, workers |
| 7 | [Plugin System](07-plugin-system.md) | Core/compat/deprecated pattern, static dispatch, feature flags |
| 8 | [Security Model](08-security-model.md) | Sandbox, permissions, CSP, CORS, site isolation |

### Part III — Content Pipeline
| Ch. | Title | Description |
| --- | --- | --- |
| 9 | [Navigation & Page Lifecycle](09-navigation-lifecycle.md) | URL → rendered page flow, history, bfcache, preload scanner |
| 10 | [Network Architecture](10-network-architecture.md) | HTTP stack, fetch, caching, service workers |
| 11 | [HTML Parser](11-parser-design.md) | Strict core parser, compat error recovery, LLM repair |
| 12 | [DOM & CSSOM](12-dom-cssom.md) | ECS DOM, CSS cascade, selector matching, computed styles |
| 13 | [ScriptSession](13-script-session.md) | Script ↔ ECS boundary, identity mapping, mutation buffering |
| 14 | [Script Engines & Web API](14-script-engines-webapi.md) | Boa, elidex-js, wasmtime, Web API bindings |

### Part IV — Rendering
| Ch. | Title | Description |
| --- | --- | --- |
| 15 | [Rendering Pipeline](15-rendering-pipeline.md) | Layout, paint, display list, compositor, GPU (Vello/wgpu) |
| 16 | [Text & Font Pipeline](16-text-pipeline.md) | Font matching, shaping, BiDi, line breaking, CJK, vertical writing |
| 17 | [Animation & Scroll](17-animation-scroll.md) | WAAPI, compositor animations, scroll physics, IntersectionObserver |
| 18 | [Image Decode](18-image-decode.md) | Format support, progressive decode, lazy loading, GPU upload |
| 19 | [SVG Rendering](19-svg-rendering.md) | Inline SVG (ECS), SVG-as-image (Vello direct), filters |

### Part V — Media & Data
| Ch. | Title | Description |
| --- | --- | --- |
| 20 | [Media Pipeline](20-media-pipeline.md) | Audio/video, codecs, MSE, EME/DRM, Web Audio, WebRTC interface |
| 21 | [File API & Streams](21-file-api-streams.md) | Blob, ReadableStream, OPFS, File System Access, compression |
| 22 | [Storage & Cache](22-storage-cache.md) | IndexedDB, Cache API, quota, memory pressure |

### Part VI — Platform & UI
| Ch. | Title | Description |
| --- | --- | --- |
| 23 | [Platform Abstraction](23-platform-abstraction.md) | OS traits, windowing, input, clipboard, file dialogs |
| 24 | [Browser Shell](24-browser-shell.md) | Tab bar, address bar, DevTools, settings, UI framework |
| 25 | [Accessibility](25-accessibility.md) | A11y tree, AccessKit, ARIA, focus, live regions |

### Part VII — elidex-app
| Ch. | Title | Description |
| --- | --- | --- |
| 26 | [Embedding API](26-embedding-api.md) | Engine/View API, native↔web bridge, headless, C bindings |

### Part VIII — Quality & Appendix
| Ch. | Title | Description |
| --- | --- | --- |
| 27 | [Testing Strategy](27-testing-strategy.md) | WPT, benchmarks, fuzz testing, visual regression |
| 28 | [Architecture Decision Records](28-adr.md) | 35 ADRs documenting key design choices and rationale |
| 29 | [Survey Analysis](29-survey-analysis.md) | JA/EN 900-site compatibility survey results and compat rule priorities |

## Key Design Decisions

See [Ch. 28 (ADR)](28-adr.md) for the full list of 35 architecture decision records.

## Languages

This document is maintained in parallel English (`en/`) and Japanese (`ja/`) editions.
