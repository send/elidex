
# 1. Executive Summary

## 1.1 What Is Elidex

Elidex is an experimental browser engine written in Rust that deliberately eliminates legacy backward compatibility to achieve maximum performance. The name derives from "elide" (to omit or suppress), reflecting the core philosophy of removing accumulated technical debt that makes modern browser engines slow and bloated.

The project serves dual purposes: as a **web browser** that handles modern websites, and as a **lightweight application runtime** (elidex-app) that competes with Electron and Tauri. By making legacy support pluggable rather than embedded, elidex maintains a clean, fast core while optionally accommodating real-world web content.

## 1.2 Core Design Principles

**HTML5-only core.** The rendering engine accepts only valid HTML5. All legacy tag handling, quirks mode, and error recovery exist outside the core as optional compatibility plugins.

**Three-layer consistency.** HTML, DOM API, CSSOM, ECMAScript, and Web APIs all follow the same pattern: a clean core with legacy handled by optional compat plugins. Every layer declares its spec level (core / compat / deprecated) and can independently remove features.

**Pluggable everything.** Every HTML tag handler, CSS property, layout algorithm, network middleware, DOM API method, and JS language feature is a discrete plugin resolved at compile time via static dispatch.

**Designed to deprecate.** The architecture explicitly supports periodic removal of outdated features, driven by usage data from automated web surveys.

**Data-oriented design.** DOM internals use an ECS (Entity Component System) for cache efficiency and parallelism, inspired by game engine architecture (Bevy).

**Session-mediated scripting.** A unified ScriptSession layer mediates between the object-oriented script world and the data-oriented ECS world, analogous to a database ORM's Unit of Work pattern.

**Parallel by default.** Style resolution and layout are parallelized using Rust's ownership model, following Servo's proven approach.

## 1.3 Architecture at a Glance

Elidex is a multi-process architecture with a Browser Process (trusted, manages state) and sandboxed Renderer Processes (one per site, untrusted). An async runtime (tokio) drives I/O in the Browser Process; Renderer Processes use a deterministic event loop. A rayon thread pool handles parallel CPU work (style, layout).

```
┌─────────────────────────────────────────────────────────┐
│ Browser Process (trusted)                               │
│  NavigationController · PermissionManager · BlobStore   │
│  CookieStore · StorageManager                           │
└──────────────────┬──────────────────────────────────────┘
                   │ IPC (serde + shared memory)
     ┌─────────┬───┼───────┬─────────────┐
     ▼         ▼   ▼       ▼             ▼
┌────────┐ ┌────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
│Renderer│ │Renderer│ │ Network │ │  GPU    │ │ Utility │
│Process │ │Process │ │ Process │ │ Process │ │ Process │
│(site A)│ │(site B)│ │         │ │ (Vello) │ │ (media) │
└────────┘ └────────┘ └─────────┘ └─────────┘ └─────────┘
```

Within each Renderer Process:

```
HTML → Parser → ECS DOM → Style → Layout → Paint (DisplayList) → Compositor → GPU (Vello/wgpu)
                  ↑                                                     ↑
            ScriptSession                                        Animation/Scroll
           (SpiderMonkey /                                      (compositor thread,
            elidex-js)                                           off-main-thread)
```

## 1.4 Key Technical Choices

| Decision | Choice | Rationale |
| --- | --- | --- |
| Language | Rust | Memory safety, fearless concurrency, ownership enables parallel layout |
| DOM storage | ECS (hecs) | Cache-line friendly, natural parallelism, ~40% less memory than tree-of-objects |
| GPU rendering | Vello + wgpu | Modern 2D vector GPU renderer. Trait-abstracted — no upstream Vello dependency |
| JS engine | SpiderMonkey (Phase 1–3), self-built elidex-js (Phase 4+) | Proven engine for bootstrap; Rust-native engine long-term |
| CSS parsing | cssparser + lightningcss | Proven Mozilla crates, extended with plugin property registry |
| Font shaping | rustybuzz | Pure-Rust HarfBuzz port; first-class CJK and BiDi |
| Accessibility | AccessKit | Cross-platform a11y (NSAccessibility, UI Automation, AT-SPI2) |
| Media codecs | Rust/C for royalty-free, platform for patent-encumbered | Avoids patent costs; H.264/AAC via OS decoders |

## 1.5 Document Structure

This design document is organized into eight parts spanning 28 chapters:

**Part I — Overview** (Ch. 1–4): Executive summary, architecture overview, roadmap, and risk analysis.

**Part II — Core Architecture** (Ch. 5–8): Process model and async runtime, thread model, plugin system (core/compat pattern), security model and permissions.

**Part III — Content Pipeline** (Ch. 9–14): Navigation and page lifecycle, network architecture, HTML parser, DOM and CSSOM, ScriptSession, script engines and Web API.

**Part IV — Rendering** (Ch. 15–19): Rendering pipeline (GPU, compositor, layers), text and font pipeline, animation and scroll, image decode pipeline, SVG rendering.

**Part V — Media & Data** (Ch. 20–22): Media pipeline (audio/video, Web Audio, MSE/EME), File API and streams, storage and cache.

**Part VI — Platform & UI** (Ch. 23–25): Platform abstraction layer, browser shell, accessibility.

**Part VII — elidex-app** (Ch. 26): Embedding API and dual-use design (browser vs. app runtime).

**Part VIII — Quality & Appendix** (Ch. 27–28): Testing strategy, architecture decision records (35 ADRs).
