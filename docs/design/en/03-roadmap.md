
# 3. Development Roadmap

## Phase 0: Foundation (Months 1-3)

Establish project infrastructure and run the compatibility survey.

| Deliverable | Status | Depends On |
| --- | --- | --- |
| Cargo workspace with crate structure | Done | — |
| CI pipeline (GitHub Actions) | Done | — |
| Run elidex-crawler survey (900 sites) | Done | — |
| Analyze survey results; prioritize compat rules | Done ([Ch. 29](29-survey-analysis.md)) | Crawler results |
| Plugin trait definitions (CssPropertyHandler, HtmlElementHandler, LayoutModel) | Done | — |
| ECS DOM storage prototype | Done | — |
| Architecture decision records (ADRs) for security model, text pipeline, writing-mode coordinate system | Done (35 ADRs) | — |

## Phase 0.5: Extended Survey (Months 3-4, conditional)

Conditional phase triggered if Phase 0 results indicate significant HTML error prevalence. Expands the crawl to determine whether LLM-assisted error recovery is worth the investment.

> **Phase 0 Survey Result (Ch. 29):** The Phase 0 survey (900 sites) confirmed an unrecoverable parser error rate of 0%, and the trigger condition (significant HTML error prevalence) was not met. This phase is **skipped**.

| Deliverable | Status | Depends On |
| --- | --- | --- |
| Expand site list to 3,000–5,000 sites | Skipped (trigger condition not met) | Phase 0 analysis |
| Crawl top page + 5–10 subpages per site | Skipped (trigger condition not met) | Expanded site list |
| Parse all pages with html5ever; record error events and classify recoverability | Skipped (trigger condition not met) | Crawl data |
| Build broken-HTML corpus for LLM training/evaluation | Skipped (trigger condition not met) | Error classification |
| Decision gate: LLM fallback go/no-go based on unrecoverable error rate | **No-Go confirmed** — unrecoverable error rate 0% ([Ch. 29](29-survey-analysis.md)) | Analysis complete |

**Decision gate: ** If the unrecoverable error rate is below approximately 2%, the LLM runtime fallback is deferred and rule-based recovery alone is implemented. LLM developer diagnostics for elidex-app proceed regardless.

> **Phase 0 Survey Result (Ch. 29):** The 900-site survey found an unrecoverable error rate of 0% (< 2% threshold). **No-Go confirmed** — LLM runtime fallback is deferred; rule-based recovery only. LLM developer diagnostics for elidex-app (elidex-llm-diag) proceed as planned in Phase 3.

## Phase 1: Minimal Rendering (Months 4-8)

Achieve first pixels on screen. The goal is rendering <div>, <p>, <span>, <a>, <img> with block layout and basic CSS (color, font, margin, padding, border, display, position).

| Deliverable | Duration Est. | Depends On |
| --- | --- | --- |
| HTML5 strict parser (parse_strict) | 4 weeks | Plugin traits |
| CSS parser + core property handlers | 4 weeks | Plugin traits |
| StyleSystem (parallel style resolution) | 3 weeks | ECS DOM, CSS parser |
| Block layout engine | 4 weeks | StyleSystem |
| Text shaping pipeline (rustybuzz + fontdb) | 3 weeks | — |
| wgpu rendering backend + text rasterization | 4 weeks | Layout, text shaping |
| Window shell (iced/egui) displaying rendered output | 2 weeks | wgpu backend |

**Milestone: ** A window that renders a static HTML5 document with styled text and images.

## Phase 2: Interactive Engine (Months 9-14)

Add JavaScript execution, Flexbox layout, and networking. The engine becomes capable of displaying simple dynamic web pages.

| Deliverable | Duration Est. | Depends On |
| --- | --- | --- |
| wasmtime integration (elidex-wasm-runtime) | 4 weeks | ECS DOM |
| DOM API plugin layer (elidex-dom-api, Living Standard) | 5 weeks | ECS DOM |
| Shared DOM host functions (elidex-dom-host, JS + Wasm) | 3 weeks | DOM API, wasmtime |
| Boa integration (elidex-js, Phase 1-3 JS) | 5 weeks | DOM host functions |
| Event system (click, input, keyboard) | 3 weeks | DOM bindings |
| Flexbox layout plugin | 4 weeks | Block layout |
| Networking stack (hyper + rustls) | 3 weeks | — |
| Fetch API implementation | 2 weeks | Networking, wasmtime |
| Process sandboxing (Linux first) | 3 weeks | Multi-process arch |
| Tolerant parser (elidex-parser-tolerant) | 3 weeks | Crawler data analysis |
| WPT integration in CI | 2 weeks | Rendering pipeline |

**Milestone: ** Navigate to a URL, render a JavaScript-driven page (e.g., a simple SPA), and interact with it.

## Phase 3: Real-World Usability (Months 15-20)

Add CSS Grid, compatibility layer, accessibility, and the app runtime. Elidex becomes usable for daily browsing of modern sites and for building desktop applications.

| Deliverable | Duration Est. | Depends On |
| --- | --- | --- |
| CSS Grid layout plugin | 5 weeks | Flex layout |
| Table layout plugin | 3 weeks | Block layout |
| Compat layer: tag normalization + CSS prefix resolution | 3 weeks | Crawler data |
| Compat layer: charset transcoding (Shift_JIS, EUC-JP) | 2 weeks | Crawler data |
| BiDi text support | 3 weeks | Text pipeline |
| CJK vertical writing mode | 4 weeks | Layout engine, text pipeline |
| Accessibility tree + AccessKit integration | 4 weeks | ECS DOM, layout |
| Canvas 2D API | 3 weeks | wgpu backend |
| elidex-app runtime MVP (Wasm + JS, multi-language) | 3 weeks | Core stable, wasmtime |
| Legacy DOM API compat layer (elidex-dom-compat) | 3 weeks | DOM API core |
| Shadow DOM + Web Components support | 4 weeks | DOM API, ECS tree scoping |
| elidex-js parser + AST (ES2020+ Stage 1) | 6 weeks | — |
| Browser chrome (tabs, address bar, history) | 4 weeks | Navigation, UI framework |
| Performance benchmarks vs Chromium/Firefox | 2 weeks | Rendering pipeline |
| LLM-powered dev diagnostics for elidex-app (elidex-llm-diag) | 3 weeks | Strict parser, candle/llama.cpp |
| Broken HTML corpus collection + LLM fine-tuning dataset | 2 weeks | Crawler results |

**Milestone: ** Browse claude.ai, major news sites, and GitHub. Build a sample desktop app with elidex-app using Rust or any Wasm-targeting language.

## Phase 4: Production Readiness (Months 21-30)

Harden for daily use. Security audit, cross-platform support, WebWorkers, Service Workers, and PWA support.

| Deliverable | Duration Est. | Depends On |
| --- | --- | --- |
| Sandbox hardening (macOS, Windows) | 4 weeks | Linux sandbox |
| Web Workers (Wasm instances on threads) | 4 weeks | wasmtime, threading model |
| WebSocket + Server-Sent Events | 2 weeks | Networking |
| IndexedDB | 3 weeks | wasmtime |
| Service Workers + PWA support | 5 weeks | Web Workers, Fetch, cache |
| CSS Animations + Transitions (compositor-driven) | 4 weeks | Compositor |
| Form controls (native rendering) | 4 weeks | Layout, event system |
| Security audit (external) | Ongoing | All security code |
| First deprecation cycle (data-driven) | 2 weeks | Crawler re-run |
| elidex-js bytecode compiler + interpreter (Stage 2) | 8 weeks | elidex-js parser |
| elidex-js inline caches + hidden classes (Stage 3) | 6 weeks | Bytecode interpreter |
| ES legacy compat layer (elidex-js-compat: Annex B, var quirks) | 4 weeks | elidex-js core |
| LLM runtime fallback (elidex-llm-runtime) integration | 4 weeks | Fine-tuned model, tolerant parser |
| Offline rule generation pipeline (LLM → rule-based parser) | 3 weeks | LLM runtime, crawler corpus |

> **Phase 0 Survey Result (Ch. 29 §29.6):** The two items above (LLM runtime fallback, offline rule generation pipeline) are **provisionally deferred** due to the Phase 0 No-Go decision. Can be revisited if future crawl data reveals significant unrecoverable errors.

**Milestone: ** Elidex as a daily-driver browser for modern sites. elidex-app 1.0 release.

## Phase 5: Long-Term (Month 30+)

**elidex-js baseline JIT (Stage 4): ** Cranelift-based JIT compiler for the elidex-js bytecode. Bridges the performance gap with Boa for compute-heavy JS.

**Boa removal: ** Once elidex-js achieves acceptable real-world performance (validated via benchmarks), remove Boa. Achieves the pure-Rust stack goal.

**elidex-js optimizing JIT (Stage 5): ** If needed, add speculative optimization passes. Only pursued if Stage 4 baseline JIT proves insufficient for target workloads.

**WebGPU API: ** Expose GPU compute to JavaScript and Wasm, leveraging elidex’s native wgpu backend.

**DevTools: ** Built-in inspector and profiler, designed with elidex’s ECS architecture in mind.

**Extension system: ** Lightweight extension API, scoped to elidex’s dual-dispatch plugin model.

**Periodic deprecation: ** Ongoing crawler surveys inform feature removal decisions across all three layers (HTML, DOM API, ECMAScript) on a regular cadence.

