
# 2. Architecture Overview

## 2.1 Process Architecture

Elidex uses a multi-process architecture inspired by Chromium’s security model and Ladybird’s clean separation. Five process types communicate via IPC (see Chapter 5 for full details):

| Process | Responsibilities | Key Dependencies |
| --- | --- | --- |
| Browser Process | Chrome UI (tabs, address bar, settings), Navigation & session management, Profile & cookie storage, Permission brokering (Ch. 8) | iced or egui (Rust native GUI), ipc-channel |
| Renderer Process | HTML/CSS parsing, DOM management (ECS), Style resolution (parallel), Layout computation (parallel), Display list generation, JavaScript execution | elidex-core + plugins, SpiderMonkey (Phase 1–3) → elidex-js, wasmtime (Wasm runtime), rayon (parallelism) |
| Network Process | HTTP/HTTPS stack, DNS resolution, connection pooling, cookie jar, TLS, WebSocket | hyper + rustls, h3 |
| GPU Process | GPU rasterization, Layer compositing, Compositor-driven scroll & animation | wgpu, Vello |
| Utility Process | Media decoding, audio processing. Spawned on demand, terminated when idle | dav1d, platform codecs |

Each tab spawns its own Renderer Process, sandboxed from the Browser Process. The GPU Process handles wgpu surface management; the compositor thread within each Renderer coordinates with it.

## 2.2 Crate Structure

The project is organized as a Cargo workspace with clear dependency boundaries:

```
elidex/
├── elidex-core/              # Framework (never contains feature-specific logic)
│   ├── elidex-ecs/           # Entity Component System
│   ├── elidex-pipeline/      # Rendering pipeline orchestration
│   ├── elidex-plugin/        # Plugin trait definitions & registry
│   ├── elidex-plugin-macros/ # Proc macro for dual dispatch generation
│   └── elidex-render/        # GPU rendering framework (wgpu)
├── elidex-plugins/            # Feature plugins (individually toggleable)
│   ├── elidex-html-base/     # Core HTML5 elements (<div>, <span>, <a>, <img>)
│   ├── elidex-html-media/    # <video>, <audio>, <canvas>
│   ├── elidex-html-forms/    # <input>, <form>, <select>
│   ├── elidex-css-box/       # display, position, margin, padding, box model
│   ├── elidex-css-flex/      # Flexbox
│   ├── elidex-css-grid/      # CSS Grid
│   ├── elidex-css-text/      # Fonts, text decoration, writing modes
│   ├── elidex-css-anim/      # Transitions, animations
│   ├── elidex-layout-block/  # Block layout algorithm
│   ├── elidex-layout-flex/   # Flex layout algorithm
│   ├── elidex-layout-grid/   # Grid layout algorithm
│   ├── elidex-layout-table/  # Table layout algorithm
│   ├── elidex-dom-api/       # DOM API plugin traits + handlers (Living Standard)
│   ├── elidex-dom-compat/    # Legacy DOM API shims (live collections, document.write)
│   └── elidex-a11y/          # Accessibility tree generation
├── elidex-script/             # Scripting layer
│   ├── elidex-script-session/ # ScriptSession: unified Script ↔ ECS boundary
│   │                          #   Identity Map, Mutation Buffer, GC coordination
│   ├── elidex-js/            # Self-built JS engine (ES2020+ core, Rust)
│   ├── elidex-js-compat/     # ES legacy semantics (Annex B, var quirks)
│   ├── elidex-js-spidermonkey/ # SpiderMonkey bridge (Phase 1-3 fallback)
│   ├── elidex-wasm-runtime/  # wasmtime integration
│   └── elidex-dom-host/      # Shared DOM host functions (JS + Wasm)
├── elidex-text/               # Text pipeline
│   ├── elidex-shaping/       # Text shaping (rustybuzz)
│   ├── elidex-bidi/          # Bi-directional text (unicode-bidi)
│   └── elidex-linebreak/     # Line breaking (icu4x)
├── elidex-compat/             # Browser-mode compatibility (optional)
│   ├── elidex-parser-tolerant/  # Error-recovering HTML parser
│   ├── elidex-compat-tags/      # Deprecated tag → HTML5 transform
│   ├── elidex-compat-css/       # Vendor prefix resolution
│   ├── elidex-compat-charset/   # Shift_JIS/EUC-JP → UTF-8
│   └── elidex-compat-dom/       # Legacy JS API shims (document.all, etc.)
├── elidex-llm-repair/         # LLM-assisted error recovery
│   ├── elidex-llm-runtime/   # Local inference (candle/llama.cpp)
│   └── elidex-llm-diag/      # Dev-mode diagnostic message generation
├── elidex-net/                # Networking
│   ├── elidex-http/          # HTTP/1.1, HTTP/2, HTTP/3 (hyper + h3)
│   ├── elidex-tls/           # TLS (rustls)
│   ├── elidex-cache/         # Disk/memory cache
│   ├── elidex-net-middleware/ # Middleware trait + pipeline
│   └── elidex-resource/      # ResourceLoader trait (http://, file://, app://)
├── elidex-security/           # Security model
│   ├── elidex-sandbox/       # Process sandboxing
│   ├── elidex-origin/        # Same-origin policy, CORS
│   └── elidex-csp/           # Content Security Policy
├── elidex-api/                # Web API implementations (beyond DOM)
│   ├── elidex-api-fetch/     # Fetch API (P0 core)
│   ├── elidex-api-canvas/    # Canvas 2D (P0 core)
│   ├── elidex-api-workers/   # Web Workers (P1 core)
│   ├── elidex-api-ws/        # WebSocket (P1 core)
│   ├── elidex-api-observers/ # Intersection/Resize Observer (P1 core)
│   ├── elidex-api-crypto/    # Web Crypto API (P1 core)
│   ├── elidex-api-cookies/   # CookieStore API (P1 core)
│   ├── elidex-api-storage/   # elidex.storage async KV API (P1 core)
│   ├── elidex-api-idb/       # IndexedDB (P2 core)
│   ├── elidex-api-gpu/       # WebGL/WebGPU (P2 core, wgpu backend)
│   ├── elidex-api-sw/        # Service Workers (P3 core)
│   ├── elidex-api-xhr/       # XMLHttpRequest compat (shimmed via Fetch)
│   ├── elidex-api-storage-compat/ # localStorage/sessionStorage compat (blocking IPC shim)
│   └── elidex-api-cookies-compat/ # document.cookie compat (shimmed via CookieStore)
├── elidex-platform/            # Platform Abstraction Layer
│   ├── elidex-platform-api/   # Trait definitions (PlatformProvider, subsystem traits)
│   ├── elidex-platform-linux/  # Linux (X11/Wayland, IBus/Fcitx, AT-SPI2)
│   ├── elidex-platform-macos/  # macOS (Cocoa, Input Method Kit, NSAccessibility)
│   ├── elidex-platform-windows/ # Windows (Win32, TSF, UIA)
│   └── elidex-platform-common/ # Shared utilities (event normalization, key mapping)
├── elidex-shell/               # Browser Shell
│   ├── elidex-shell-api/      # Trait definitions (TabManager, NavigationManager, etc.)
│   ├── elidex-shell-state/    # Default state manager implementations
│   ├── elidex-chrome-native/  # Native chrome (egui/iced, Phase 1-2)
│   ├── elidex-chrome-selfhost/ # Self-hosted chrome (HTML/CSS, Phase 3+)
│   ├── elidex-devtools/       # DevTools implementation
│   └── elidex-extension-host/ # Extension mounting points and lifecycle
├── elidex-browser/            # Full browser (core + all plugins + compat + shell)
├── elidex-app/                # App runtime (core + selected plugins only)
└── elidex-crawler/            # Web compatibility survey tool
```

