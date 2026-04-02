# CLAUDE.md — elidex project notes

## Project Overview

elidex is an experimental browser engine written in Rust. Phase 0 (foundation) is complete.

### Workspace Structure

```
crates/
  core/
    elidex-plugin/        — Plugin traits, SpecLevel enums, PluginRegistry, css_resolve helpers
    elidex-plugin-macros/ — Procedural macros (#[derive(SpecLevel)] etc.)
    elidex-ecs/           — ECS (hecs) based DOM prototype
    elidex-render/        — Rendering backend abstraction
  css/
    elidex-css/           — CSS parser, value types, selector engine
    elidex-css-box/       — CSS box model property handler plugin (display, position, margin, padding, border, etc.)
    elidex-css-text/      — CSS text/font property handler plugin (color, font-*, text-*, etc.)
    elidex-css-flex/      — CSS flexbox property handler plugin (flex-*, justify/align-*, order)
    elidex-css-grid/      — CSS grid property handler plugin (grid-*)
    elidex-css-table/     — CSS table property handler plugin (border-collapse, table-layout, etc.)
    elidex-css-float/     — CSS float/clear/visibility/vertical-align property handler plugin
    elidex-css-anim/      — CSS Animations + Transitions property handler plugin
    elidex-style/         — Cascade, inheritance, style resolution
  dom/
    elidex-html-parser/   — HTML/XML parser (html5ever wrapper, charset detection)
    elidex-dom-api/       — DOM API handler implementations (engine-independent)
    elidex-dom-compat/    — Legacy/compat DOM layer
    elidex-a11y/          — Accessibility tree builder (ECS DOM → AccessKit)
  layout/
    elidex-layout/        — Layout orchestrator (block, inline, dispatch)
    elidex-layout-block/  — Block layout
    elidex-layout-flex/   — Flexbox layout
    elidex-layout-grid/   — Grid layout
    elidex-layout-table/  — Table layout
  text/
    elidex-shaping/       — Text shaping (rustybuzz)
    elidex-linebreak/     — Line breaking (UAX #14)
    elidex-bidi/          — BiDi algorithm (UAX #9)
    elidex-text/          — Text facade (shaping + linebreak + bidi)
  net/
    elidex-net/           — HTTP network stack (hyper, TLS, connection pool, cookies)
  script/
    elidex-script-session/ — Script session abstraction (JS ↔ ECS DOM bridge)
    elidex-js/            — JavaScript parser (custom ES2020+ parser)
    elidex-js-boa/        — Boa JS engine integration (boa_engine 0.21)
    elidex-wasm-runtime/  — WebAssembly runtime (wasmtime, DOM host functions)
  web/
    elidex-web-canvas/    — Canvas 2D API (tiny-skia CPU rasterization)
  shell/
    elidex-shell/         — Window management, event loop shell, browser chrome (egui)
    elidex-navigation/    — Navigation controller (history, load pipeline)
  tools/
    elidex-crawler/       — Web compatibility survey tool (binary crate)
    elidex-wpt/           — WPT-style CSS conformance test harness
```

### Common Commands

```sh
mise run ci          # Run all CI checks locally (lint + test-all + deny)
mise run test        # cargo nextest run --workspace (no doc-tests, fast)
mise run test-all    # nextest + doc-tests (full)
mise run test-doc    # doc-tests only
mise run lint        # clippy + fmt check
mise run fmt         # cargo fmt --all
cargo doc --workspace --no-deps  # Build docs
mise run bench                   # Run all benchmarks (CSS, style, layout)
```

### Key Files

- `SECURITY.md` — Vulnerability disclosure policy
- `CONTRIBUTING.md` — Contribution guidelines
- `.github/dependabot.yml` — Automated dependency updates (Cargo + Actions)
- `deny.toml` — License allow-list + supply chain (`unknown-registry`/`unknown-git` = deny)
- `docs/design/ja/29-survey-analysis.md` — JA/EN 900-site compatibility survey analysis (Ch. 29)

---

## Architecture Notes

### elidex-crawler

- **SSRF protection**: `validate_url()` checks scheme, private IPs (IPv4/IPv6), reserved hostnames. Custom reqwest redirect policy validates each hop.
- **robots.txt**: RFC 9309 subset. `Allow`/`Disallow` with longest-match-wins. `best_agent_match()` shared by `is_allowed`/`crawl_delay`. `Crawl-delay` validated (`is_finite() && >= 0.0`). Body fetch has 5s timeout + 512KB limit.
- **Concurrency**: Global semaphore + per-host semaphore (2 concurrent). Semaphore acquire uses `let...else` for graceful error handling. Host map evicts stale entries at 10,000 threshold via `Arc::strong_count`.
- **Content-Type**: MIME type extracted before `;` for exact match (`text/html`, `application/xhtml+xml`, `text/xml`).
- **Shared utilities**: `analyzer/util.rs` provides `strip_comments()` (CSS/JS) and `extract_tag_blocks()` (style/script), both `pub(crate)`. `MAX_EXTRACT_ITERATIONS` in `analyzer/mod.rs`.
- **Type alias**: `FeatureCount = HashMap<String, usize>` in `analyzer/mod.rs`, used across all feature structs.
- **Report aggregation**: `FeatureAggregator` pattern + `FEATURE_CSV_HEADER` constant deduplicates feature counting/CSV logic.
- **Error chain**: Retry errors use `format!("{e:#}")` to preserve full anyhow error chain.
- **`to_ascii_lowercase()` safety**: Verified — only modifies ASCII bytes, preserving byte positions. Safe to cross-index with original HTML.

### elidex-html-parser

- **Three entry points**: `parse_html(&str)` (existing, UTF-8), `parse_tolerant(&[u8], charset_hint)` (byte input with charset auto-detection), `parse_strict(&str)` (rejects documents with parse errors).
- **Charset detection** (`charset.rs`): BOM always stripped first (`strip_bom()`), then encoding priority: HTTP charset hint → BOM encoding → `<meta charset>` prescan (1024 bytes) → `<meta http-equiv="Content-Type" content="…;charset=…">` prescan → UTF-8 default. Uses `encoding_rs` with `new_decoder_without_bom_handling()` to avoid encoding_rs's built-in BOM sniffing overriding our priority logic.
- **ParseResult**: Extended with `encoding: Option<&'static str>` (set by `parse_tolerant`, `None` for `parse_html`).
- **StrictParseError**: `Display` + `Error` impl, contains `Vec<String>` of html5ever error messages.
- **Dependencies**: `encoding_rs 0.8` for charset detection/transcoding.

### elidex-ecs

- **Tree invariants**: No cycles (ancestor walk with depth counter, capped at 10,000), consistent sibling links, parent↔child consistency, destroyed entity safety. `#[must_use]` on all mutation methods.
- **Internal helpers**: `update_rel()`, `read_rel()`, `clear_rel()` for TreeRelation access. `is_child_of()` for parent validation. `all_exist()` for entity checks.
- **API**: `append_child`, `insert_before`, `replace_child` (validates before detach), `detach`, `destroy_entity`. Helpers: `get_parent`, `get_first_child`, `get_last_child`, `get_next_sibling`, `get_prev_sibling`, `contains`.
- **Attributes**: `get/set/remove/contains` accessors on `Attributes` struct.
- **Shadow DOM**: `ShadowRoot` (mode + host), `ShadowHost` (shadow_root), `ShadowRootMode` (Open/Closed), `SlotAssignment` (assigned_nodes), `SlottedMarker`, `TemplateContent` (marker) components. `attach_shadow(host, mode)` with WHATWG element whitelist (18 tags). `get_shadow_root(host)`. `composed_children(entity)` — shadow hosts return shadow tree children, slots return assigned nodes (or fallback), others return normal children.

### elidex-plugin

- **Traits**: `CssPropertyHandler`, `HtmlElementHandler`, `LayoutModel`, `NetworkMiddleware` (all `Send + Sync`).
- **PluginRegistry**: Generic (`Debug` impl), static-first lookup, `#[must_use]` on `resolve()`, same-name re-registration overwrites. `is_shadowed()` helper for dedup.
- **SpecLevel enums**: All `#[non_exhaustive]`, `Default` with `#[default]` on first variant.
- **Error types**: `define_error_type!` macro for DRY error boilerplate (`ParseError`, `HtmlParseError`, `NetworkError`).
- **JsValue**: `#[non_exhaustive]` enum (Undefined/Null/Bool/Number/String/ObjectRef) — cross-engine JS value type.
- **Network types**: `HttpRequest` (method/url/headers), `HttpResponse` (status/headers), `NetworkError` (kind/message), `NetworkErrorKind` enum.
- **ProcessModel**: `SiteIsolation`/`PerTab`/`Shared{max_renderers}`/`SingleProcess` — `#[non_exhaustive]`, Phase 3.5 implements `SingleProcess` only.
- **Sandbox types** (`sandbox.rs`): `FilesystemAccess` (None/ReadOnly/ReadWrite), `NetworkAccess` (None/SameOrigin/Full), `SandboxPolicy` (filesystem/network/ipc/gpu) with `strict()`/`permissive()`/`web_content()` constructors, `PlatformSandbox` (LinuxSeccomp/MacOSAppSandbox/WindowsRestricted/Unsandboxed). Type-only — enforcement deferred to OS process isolation phase.
- **Built-in handlers** (`handlers/`): Demo trait implementations for HTML and layout plugins. `create_html_element_registry()` (div/a/img/script/button), `create_layout_registry()` (block/flex/grid/table). Layout models use stub layout (actual dispatch remains in elidex-layout). CSS property handlers moved to dedicated plugin crates (elidex-css-{box,text,flex,grid,table,float}).
- **css_resolve module**: Shared resolution utilities re-exported for plugin crates — `resolve_length`, `resolve_dimension`, `resolve_to_px`, `resolve_calc_expr`, `resolve_non_negative_f32`, `resolve_i32`, `keyword_from`, `parse_length_unit`.

### elidex-layout

- **Block layout**: `layout_block_inner()` handles block formatting context — width resolution, margin collapse (adjacent siblings, positive/negative), padding/border, vertical stacking. `is_block_level()` classifies display types.
- **Inline layout**: `layout_inline()` handles inline formatting context — text shaping, line breaking, line box construction.
- **Flexbox layout**: `flex.rs` implements CSS Flexbox Level 1 (simplified). `layout_flex()` entry point: box model resolution → item collection (`display:none` skipped) → `order` stable sort → line splitting (nowrap/wrap/wrap-reverse) → flexible length resolution (grow/shrink with frozen/unfrozen loop) → cross size resolution → main axis positioning (justify-content: 6 values) → cross axis alignment (align-items/align-self: stretch/flex-start/flex-end/center) → multi-line align-content distribution.
- **Phase 2 simplifications**: `baseline` alignment → `flex-start`, `flex-basis: content` → `auto`, `InlineFlex` treated as block-level.
- **Routing**: `block.rs` and `layout.rs` route `Display::Flex`/`InlineFlex` children to `flex::layout_flex()`.

### elidex-script-session

- **SessionCore**: Owns `IdentityMap` + `Vec<Mutation>`. `record_mutation()` buffers, `flush()` applies all via `apply_mutation()`. `get_or_create_wrapper()` for identity mapping, `release()` for cleanup.
- **IdentityMap**: Bidirectional `(Entity, ComponentKind) ↔ JsObjectRef`. Monotonic counter for unique refs. `get_or_create()` is idempotent. `release()` single ref, `release_entity()` all refs for an entity.
- **Mutation enum**: `AppendChild`, `InsertBefore`, `RemoveChild`, `ReplaceChild`, `SetAttribute`, `RemoveAttribute`, `SetTextContent`, `SetInnerHtml` (stub), `SetInlineStyle`, `RemoveInlineStyle`, `InsertCssRule`/`DeleteCssRule` (stubs).
- **apply_mutation()**: Delegates tree ops to `EcsDom`, attribute/style ops via `world_mut()`. `SetInlineStyle` auto-inserts `InlineStyle` component if missing. Returns `Option<MutationRecord>`.
- **DomApiHandler / CssomApiHandler traits**: `Send + Sync`, `method_name()`, `spec_level()` (default Living/Standard), `invoke(this, args, session, dom) -> Result<JsValue, DomApiError>`.
- **Types**: `JsObjectRef(u64)`, `ComponentKind` enum (Element/Style/ClassList/Attributes/Dataset/ChildNodes), `DomApiError` + `DomApiErrorKind` (NotFoundError/HierarchyRequestError/InvalidStateError/SyntaxError/TypeError/Other).
- **Event dispatch**: `DispatchEvent` with `composed: bool` (default true) and `original_target: Option<Entity>`. `build_propagation_path(dom, target, composed)` — non-composed events stop at `ShadowRoot`. Event retargeting: shadow-internal targets → shadow host for outside listeners (slotted elements exempt).

### elidex-dom-api

- **Engine-independent DOM API handlers**: Concrete implementations of `DomApiHandler`/`CssomApiHandler` traits from `elidex-script-session`. No dependency on boa or any JS engine.
- **document.rs**: `QuerySelector` (CSS selector matching via `elidex_css::Selector::matches()`), `GetElementById`, `CreateElement`, `CreateTextNode`, `query_selector_all()` standalone function.
- **element.rs**: `AppendChild`, `InsertBefore`, `RemoveChild` (direct `EcsDom` operations), `Get/Set/RemoveAttribute`, `Get/SetTextContent`, `GetInnerHtml` (HTML serialization with escaping).
- **class_list.rs**: `ClassListAdd/Remove/Toggle/Contains` — operates on `Attributes` class string.
- **style.rs**: `StyleSetProperty/GetPropertyValue/RemoveProperty` — `InlineStyle` component operations. Auto-inserts `InlineStyle` if missing.
- **computed_style.rs**: `GetComputedStyle` (CssomApiHandler) — delegates to `elidex_style::get_computed_as_css_value()`.
- **util.rs**: `require_string_arg()`, `require_object_ref_arg()`, `escape_html()`, `escape_attr()`.

### elidex-js

- **JsRuntime**: Owns boa `Context`, `HostBridge`, `ConsoleOutput`, `TimerQueueHandle`. `eval()` binds bridge, evaluates source, unbinds. `drain_timers()` runs ready timer callbacks.
- **HostBridge**: `Rc<RefCell<HostBridgeInner>>` with raw pointers to `SessionCore`/`EcsDom`. `bind()`/`unbind()` bracket eval. `with(|session, dom| ...)` for native function access. `!Send` via `Rc`. JsObject cache for entity identity (`HashMap<JsObjectRef, JsObject>`).
- **globals/document.rs**: `register_document()` — querySelector, querySelectorAll (JsArray), getElementById, createElement, createTextNode, body accessor.
- **globals/element.rs**: `build_element_object()` — appendChild, removeChild, setAttribute, getAttribute, removeAttribute, textContent (accessor), innerHTML (getter), style (accessor → `create_style_object`), classList (accessor → `create_class_list_object`), attachShadow({mode}) (creates shadow root via EcsDom), shadowRoot (accessor: open→root, closed/none→null). Entity stored as f64 in hidden `__elidex_entity__` property.
- **globals/window.rs**: `register_window()` — `getComputedStyle(element)` returning computed style proxy. `create_style_object()` — setProperty/getPropertyValue/removeProperty.
- **globals/console.rs**: `register_console()` — log/error/warn. `ConsoleOutput` captures messages for testing.
- **globals/timers.rs**: `register_timers()` — setTimeout/setInterval/clearTimeout/clearInterval/requestAnimationFrame/cancelAnimationFrame. `TimerQueueHandle` wraps `Rc<RefCell<TimerQueue>>`.
- **timer_queue.rs**: `TimerQueue` with `BinaryHeap<Reverse<TimerEntry>>` min-heap. Min 1ms interval to prevent infinite loops. `drain_ready()` collects and re-schedules intervals.
- **script_extract.rs**: `extract_scripts()` — pre-order DOM walk collecting inline `<script>` text. External scripts (`src="..."`) logged and skipped.
- **value_conv.rs**: `to_boa()`/`from_boa()` bidirectional elidex↔boa JsValue conversion.
- **error_conv.rs**: `dom_error_to_js_error()` — DomApiErrorKind → boa JsNativeError.
- **boa 0.20 notes**: `ObjectInitializer` methods return `&mut Self`, accessors need `JsFunction` (via `to_js_function(&realm)`), `custom_trace!(this, mark, {body})` with 3 args, `from_copy_closure_with_captures` for safe closure registration.
- **globals/fetch.rs**: `register_fetch()` — `fetch(url, options?)` global. Blocking HTTP via `FetchHandle::send_blocking()`. Returns `JsPromise::resolve(Response)` or `JsPromise::reject(TypeError)`. Response object: ok/status/statusText/url/type/redirected/headers properties + `text()`/`json()`/`clone()` methods. `json()` uses boa `JSON.parse()` via global object. Headers object: `get()` (combines duplicates per Fetch spec), `has()`, `forEach()`. `set()`/`delete()` omitted (Response headers immutable).
- **run_jobs() integration**: `eval()` calls `ctx.run_jobs()` after evaluation (bridge still bound) to drain microtask queue. `dispatch_event()` similarly calls `ctx.run_jobs()` after dispatch loop. Enables `fetch().then(r => r.text())` chains.
- **globals/canvas.rs**: `create_context2d_object()` — CanvasRenderingContext2D JS object with drawing methods delegating to `Canvas2dContext` in `HostBridge`. `sync_canvas_to_image_data()` syncs pixels to ECS `ImageData` after each draw. `extract_entity_bits()` reads entity from hidden property.
- **HostBridge canvas support**: `canvas_contexts: HashMap<u64, Canvas2dContext>` in `HostBridgeInner`. `ensure_canvas_context(entity_bits, width, height)` creates context, `with_canvas(entity_bits, f)` accesses it.
- **globals/element.rs**: `getContext("2d")` on `<canvas>` elements — reads width/height attributes (defaults 300×150), creates Canvas2dContext via bridge, returns cached context2d JS object.
- **Dependencies**: boa_engine 0.20 (annex-b), boa_gc 0.20, elidex-net, elidex-navigation, elidex-web-canvas, url, bytes.

### elidex-net

- **NetClient**: Top-level API integrating transport, cookies, middleware, redirect, CORS, HTTPS-upgrade. `send()` for raw HTTP, `load()` for resource loading (http/data/file).
- **HttpTransport**: Sends requests via connection pool with timeout. Wraps hyper HTTP/1.1 and HTTP/2.
- **ConnectionPool**: Per-origin pooling. `OriginKey(scheme, host, port)`. H1: up to 6 idle connections. H2: single multiplexed `SendRequest` clone. 90s idle eviction.
- **Connector**: TCP+TLS with DNS-level SSRF protection. Uses `TokioIo<StreamWrapper>` for hyper compatibility. ALPN negotiation for H2.
- **TLS**: rustls with ring provider, webpki-roots, TLS 1.2/1.3, ALPN `h2, http/1.1`.
- **CookieJar**: `Set-Cookie` parsing via `cookie` crate. Domain/path matching (RFC 6265). `SameSite=Lax` default. Third-party blocking (simplified domain comparison, TODO: eTLD+1 in M2-7). Thread-safe via `Mutex`.
- **Redirect**: Follows 301/302/303/307/308 up to max 20. 301-303 change to GET. SSRF re-validation on each hop (skipped when `allow_private_ips`).
- **CORS**: `validate_cors()` checks `Access-Control-Allow-Origin` against request origin.
- **HTTPS upgrade**: `upgrade_to_https()` rewrites HTTP URLs to HTTPS.
- **MiddlewareChain**: Adapts plugin `NetworkMiddleware` trait to internal Request/Response types.
- **data_url**: RFC 2397 parser (plain text + base64).
- **ResourceLoader trait + SchemeDispatcher**: Routes http/https, data:, file:// with cookie injection and redirect following.
- **FetchHandle**: Wraps tokio current-thread `Runtime` + `NetClient`. `send_blocking(&self, request)` blocks via `rt.block_on(client.send(request))`. Used by elidex-js (`JsRuntime::with_fetch`) and elidex-navigation (`load_document`).
- **SSRF shared module**: `elidex_plugin::url_security` — `validate_url()` + `is_private_ip()`, shared by elidex-net and elidex-crawler.

### CSS Plugin Crates (elidex-css-{box,text,flex,grid,table,float,anim})

- **Plugin architecture**: Each crate implements `CssPropertyHandler` for a group of related CSS properties. Handlers provide `parse()`, `resolve()`, `initial_value()`, `is_inherited()`, `affects_layout()`, and `get_computed()`.
- **elidex-css-box**: `BoxHandler` — display, position, width/height/min/max, margin-*, padding-*, border-*-{width,style,color}, box-sizing, border-radius, opacity, overflow, background-color, content, row-gap, column-gap (33 tests).
- **elidex-css-text**: `TextHandler` — color, font-size/weight/style/family, line-height, text-align/transform, white-space, letter/word-spacing, text-decoration-*, writing-mode, text-orientation, direction, unicode-bidi, list-style-type (36 tests).
- **elidex-css-flex**: `FlexHandler` — flex-direction/wrap, justify-content, align-items/content/self, flex-grow/shrink, flex-basis, order (20 tests).
- **elidex-css-grid**: `GridHandler` — grid-template-columns/rows, grid-auto-flow/columns/rows, grid-column/row-start/end (20 tests).
- **elidex-css-table**: `TableHandler` — border-collapse (inherited), border-spacing-h/v (inherited), table-layout, caption-side (inherited) (12 tests).
- **elidex-css-float**: `FloatHandler` — float, clear, visibility (inherited), vertical-align (10 tests).
- **elidex-css-anim**: `AnimHandler` — transition/animation shorthands + 12 longhands (113 tests). See below.
- **Registration**: Each handler has `register(&mut CssPropertyRegistry)`. All 7 registered in `elidex_shell::create_css_property_registry()`.
- **Dependencies**: All depend on elidex-plugin (traits, CssValue, ComputedStyle). Some depend on elidex-css (color parsing) and cssparser (tokenization).

### elidex-css-anim

- **AnimHandler**: `CssPropertyHandler` impl for 14 properties: `transition` (shorthand), `transition-property/duration/timing-function/delay`, `animation` (shorthand), `animation-name/duration/timing-function/delay/iteration-count/direction/fill-mode/play-state`. None inherited, none affects layout directly.
- **timing.rs**: `TimingFunction` enum — `CubicBezier(x1,y1,x2,y2)`, `Steps(count, StepPosition)`, `Linear`. Cubic-bezier solver: Newton-Raphson with bisection fallback. Named easings: `EASE`, `EASE_IN`, `EASE_OUT`, `EASE_IN_OUT`. `StepPosition`: JumpStart/JumpEnd/JumpNone/JumpBoth.
- **style.rs**: `AnimStyle` ECS component — 12 `Vec` fields for transition/animation property lists. Separate from `ComputedStyle` (only inserted when animation/transition props are set). Types: `TransitionProperty`, `IterationCount`, `AnimationDirection`, `AnimationFillMode`, `PlayState`.
- **parse.rs**: `parse_time()` (s/ms), `parse_timing_function()` (keywords + cubic-bezier() + steps()), `parse_transition_property/shorthand()`, `parse_animation_name/iteration_count/direction/fill_mode/play_state()`. `KeyframesRule`/`Keyframe` types, `parse_keyframes()` for `@keyframes` block parsing.
- **resolve.rs**: `resolve_anim_property()` dispatches by property name into `AnimStyle` fields. Helpers: `resolve_transition_property()`, `resolve_time_list()`, `resolve_timing_function_list()`, `resolve_animation_names()`, etc.
- **interpolate.rs**: `interpolate(from, to, t)` — numeric lerp for Number/Length/Percentage/Time/Color, discrete flip at 50% for keywords/auto. `interpolate_color()` component-wise RGBA. `is_animatable()` checks ~35 animatable property names.
- **instance.rs**: `AnimationInstance` (name, timing, iteration, direction, fill, play state, elapsed, finished). `progress()` computes iteration-aware progress with direction/fill. `TransitionInstance` (property, from/to values, elapsed). `current_value()` returns interpolated value.
- **engine.rs**: `AnimationEngine` — manages running transitions/animations per entity (`HashMap<u64, Vec>`). `tick(dt)` advances all, returns `Vec<(EntityId, AnimationEvent)>` for `transitionend`/`animationend`. `register_keyframes()`, `add_transition()`/`add_animation()`, auto-cleanup on finish.
- **timeline.rs**: `DocumentTimeline` — monotonic time tracker (`current_time`, `advance(dt)`).
- **detection.rs**: `detect_transitions()` — compares old/new computed values against `AnimStyle.transition_property` list, produces `Vec<DetectedTransition>` with CSS list cycling behavior.

### elidex-shell

- **CSS property registry**: `create_css_property_registry()` builds a `CssPropertyRegistry` with all 7 CSS plugin handlers (box, text, flex, grid, table, float, anim). Passed to `resolve_styles_with_compat()` for handler-based dispatch. `get_computed_with_registry()` in elidex-style uses handlers for computed value extraction. Registry also passed to `parse_compat_stylesheet_with_registry()` for handler-based property parsing (transition/animation properties).
- **AnimationEngine**: `PipelineResult.animation_engine` — initialized in both `build_pipeline_interactive()` and `build_pipeline_from_loaded()`. `@keyframes` rules from all stylesheets are parsed and registered via `register_keyframes_from_stylesheets()`. Dependencies: elidex-css-anim.
- **chrome.rs**: Browser chrome UI (egui overlay). `ChromeState` (address_text, address_focused, tab_bar_position), `ChromeAction` enum (Navigate/Back/Forward/Reload/NewTab/CloseTab/SwitchTab), `build()` draws egui `TopBottomPanel` with back/forward/reload buttons and address bar. `CHROME_HEIGHT = 36.0`, `TAB_BAR_HEIGHT = 28.0`, `TAB_SIDEBAR_WIDTH = 200.0` logical pixels. `TabBarPosition` (Top/Left/Right), `TabBarInfo` for tab bar rendering, `build_tab_bar()` renders horizontal or side-panel tabs, `chrome_content_offset()` computes content area `(x, y)` offset.
- **Tab management** (`app/tab.rs`): `TabId(u64)` unique identifier, `Tab` (channel, thread, display_list, chrome, window_title), `TabManager` (Vec<Tab>, active_id, id_gen). Methods: `create_tab()`, `close_tab()` (shutdown + neighbor select), `active_tab()`/`active_tab_mut()`, `set_active()`, `next_tab_id()`/`prev_tab_id()` (wrap-around), `nth_tab_id()`, `shutdown_all()`.
- **egui integration**: `RenderState` holds `egui::Context`, `egui_winit::State`, `egui_wgpu::Renderer`. Initialized in `try_init_render_state()`. Overlay rendered via `render_egui_overlay()` / `render_egui_output()` using `LoadOp::Load` render pass after Vello blit. `handle_redraw_with_tabs()` renders tab bar + chrome bar.
- **Event routing**: egui-first — `on_window_event()` passes events to `egui_state` first; if consumed, content handlers are skipped. Address bar focus (`address_focused`) suppresses content keyboard events. Mouse coordinate offset via `chrome_content_offset()`.
- **Chrome actions**: `handle_chrome_action_threaded()` — Navigate/Back/Forward/Reload + NewTab/CloseTab/SwitchTab. `handle_chrome_action()` for legacy mode.
- **Keyboard shortcuts**: `check_tab_shortcut()` — Cmd/Ctrl+T (new tab), Cmd/Ctrl+W (close tab), Ctrl+Tab/Ctrl+Shift+Tab (cycle), Cmd/Ctrl+1-9 (nth tab).
- **URL sync**: `chrome.set_url()` called in `navigate()`, `navigate_to_history_url()`, and `handle_history_action()` (PushState/ReplaceState).
- **Accessibility**: `RenderState.a11y_adapter` — `accesskit_winit::Adapter` initialized via `with_direct_handlers()` with stub handlers (NoopActivation/Action/Deactivation). Window created `with_visible(false)` for AccessKit init safety, then shown.
- **Multi-tab architecture (M3.5-10)**: `App.tab_manager: Option<TabManager>` replaces single `ContentHandle`. Each tab has independent content thread, display list, chrome state. `drain_content_messages()` drains all tabs, active tab title synced to window. `cursor_pos`/`modifiers` at App level (window-wide). `BLANK_TAB_HTML`/`BLANK_TAB_CSS` constants, `spawn_content_thread_blank()` for new tabs.
- **IPC module** (`ipc.rs`): `BrowserToContent` (Navigate/MouseClick/MouseMove/CursorLeft/KeyDown/KeyUp/SetViewport/GoBack/GoForward/Reload/Shutdown), `ContentToBrowser` (DisplayListReady/TitleChanged/NavigationState/UrlChanged/NavigationFailed), `ModifierState`, `LocalChannel<S,R>`, `channel_pair()`.
- **Content thread** (`content.rs`): `spawn_content_thread()`/`spawn_content_thread_url()`/`spawn_content_thread_blank()`, `content_thread_main()` event loop, hover/focus/active management, link navigation detection, JS timer drain via `recv_timeout`, history action handling.
- **Dependencies**: egui 0.33, egui-wgpu 0.33, egui-winit 0.33 (all MIT/Apache-2.0, wgpu 27 compatible), accesskit 0.24, accesskit_winit 0.32, elidex-a11y, crossbeam-channel 0.5.

### elidex-web-canvas

- **Canvas2dContext**: Wraps `tiny_skia::Pixmap` with `DrawingState` stack (fill/stroke color, line_width, global_alpha, transform). Default 300×150 pixels.
- **Drawing methods**: `fillRect`, `strokeRect`, `clearRect` (rectangle methods), `beginPath`/`moveTo`/`lineTo`/`closePath`/`rect`/`arc`/`fill`/`stroke` (path methods), `save`/`restore` (state), `translate`/`rotate`/`scale` (transform).
- **Image data**: `getImageData`/`putImageData`/`createImageData` with premultiplied↔straight alpha conversion. `to_rgba8_straight()` for ECS `ImageData` sync.
- **Arc approximation**: `arc_to_beziers()` converts Canvas 2D `arc()` to cubic Bezier curves — splits into ≤90° segments, k = (4/3)*tan(half_angle).
- **Style parsing**: `parse_color_string()` delegates to `elidex_css::parse_color` for CSS color string support.
- **Dependencies**: elidex-plugin (CssColor), elidex-css (parse_color), tiny-skia 0.11.

### elidex-a11y

- **build_tree_update()**: Walks ECS DOM pre-order → AccessKit `TreeUpdate`. `TREE_ROOT_ID = 0` sentinel for document root (safe because hecs entities are `NonZeroU64`). Skips `aria-hidden="true"` elements.
- **Role mapping**: `tag_to_role()` maps ~30 HTML tags, `aria_role_from_str()` maps ~60 ARIA role strings. Special cases: `img` with empty `alt` → GenericContainer, `a` without `href` → GenericContainer.
- **ACCNAME algorithm**: `compute_accessible_name()` — priority: `aria-labelledby` (id reference resolution) → `aria-label` → `alt` (img) → text content → `title`.
- **entity_to_node_id()**: `Entity.to_bits().get()` → `NodeId(u64)`.
- **Dependencies**: elidex-ecs, elidex-plugin (LayoutBox), accesskit 0.24.

### elidex-wasm-runtime

- **WasmRuntime**: Owns wasmtime `Engine` + `Linker<HostState>` + `Arc<DomHandlerRegistry>` / `Arc<CssomHandlerRegistry>`. `compile(bytes)` → `WasmModule`, `instantiate(module)` → `WasmInstance`.
- **WasmInstance**: Owns `Store<HostState>` + `Instance`. `call_export(name, args, session, dom, doc)` with bind/unbind bracket + `UnbindGuard`. `export_names()` / `get_func()` for introspection.
- **HostState**: Raw pointers to `SessionCore`/`EcsDom` (bind/unbind pattern from `HostBridge`). `Arc<Registry>` for `Send` compatibility. `unsafe impl Send` (single-threaded usage).
- **Host functions** ("elidex" namespace): 12 functions registered via `Linker`. Entity handles as `i64` (0 = null). Strings via `(ptr, len)` pairs in Wasm linear memory. Returns: packed `i64` `(ptr << 32) | len`, allocated via guest's `__alloc(len) -> ptr`.
- **Identity map bridge**: `objref_to_entity_i64()` / `entity_i64_to_objref()` translate between Wasm entity handles and `JsObjectRef` for handler dispatch.
- **Dependencies**: wasmtime 29, elidex-plugin, elidex-ecs, elidex-script-session, elidex-dom-api. Dev: wat 1.

### elidex-style (parallel)

- **Feature flag**: `parallel` enables rayon-based sibling parallel style resolution.
- **Strategy**: Cascade (`collect_and_cascade`) runs sequentially (requires `&EcsDom`), then `build_computed_style` runs in parallel across siblings via rayon, then results applied and children recursed sequentially.
- **parallel.rs**: `OwnedPropertyMap`, `to_owned_map()`, `par_resolve_siblings()` (threshold 8), `build_computed_style_owned()`.
- **walk.rs**: `walk_children_parallel()` — 3-phase approach (cascade → parallel resolve → apply+recurse). Falls back to sequential for shadow hosts.
- **Dependencies**: rayon 1 (optional).

### elidex-wpt

- **WPT-style test harness**: JSON-based test case format for CSS conformance testing.
- **WptTestCase**: name, html, css, expected (selector → property → value string).
- **run_test_case()**: parse_html → parse_stylesheet → resolve_styles → find element by selector → compare computed values.
- **run_test_suite()**: batch runner returning `Vec<WptTestResult>`.
- **Built-in suites**: `suites::cascade::cascade_suite()` — 10 cascade/specificity/inheritance test cases.
- **Dependencies**: elidex-html-parser, elidex-css, elidex-ecs, elidex-style, elidex-plugin, serde, serde_json.

### Benchmarks (criterion)

- **elidex-css** (`benches/css_parse.rs`): `css_parse_10/100/1000_rules`.
- **elidex-style** (`benches/style_resolve.rs`): `resolve_100/1000_flat`, `resolve_deep_100`.
- **elidex-layout** (`benches/layout_bench.rs`): `block_100`, `flex_20`.
- **Run**: `mise run bench` or `cargo bench -p <crate>`.

### elidex-script-session (additions)

- **ScriptEngine trait**: `eval()`, `dispatch_event()`, `drain_timers()` — engine-agnostic interface. Navigation state methods intentionally excluded (engine-specific).
- **EvalResult**: Moved from `elidex-js-boa` to `elidex-script-session` (canonical location). Re-exported from `elidex-js-boa` for compatibility.

### elidex-js-boa (additions)

- **globals/wasm.rs**: `register_wasm()` — `WebAssembly.instantiate(bufferSource)` / `WebAssembly.compile(bufferSource)`. `WasmRuntime` + `WasmInstance` stored in `Rc<RefCell>` closures. Export functions wrapped as JS callables via `ExportCaptures`. `extract_wasm_bytes()` reads array-like objects.
- **impl ScriptEngine for JsRuntime**: Delegates to concrete methods.

### CI

- 4 jobs: `changes` (path filter), `check` (ubuntu/macos/windows: fmt + clippy + test), `doc` (cargo doc -D warnings), `deny` (standalone).
- No MSRV policy during active development (ADR #36). Will adopt N-2 stable releases once the project stabilizes.
- Path-based skip: `dorny/paths-filter@v3` detects `rust` (crates/\*\*, Cargo.\*, toolchain/fmt/clippy config, mise.toml) and `config` (deny.toml, Cargo.\*) changes. Push to main always runs all jobs.
- Actions pinned: `actions/checkout@v4`, `Swatinem/rust-cache@v2`, `dorny/paths-filter@v3`, `taiki-e/install-action@v2`.
- `rust-toolchain.toml`: `channel = "stable"`.

### elidex-storage-core (browser_db module, M4-8.5)

- **BrowserDb**: `browser.sqlite` centralized database (8 tables). Typed sub-stores: `CookieStore<'db>`, `HistoryStore<'db>`, `BookmarkStore<'db>`.
- **OriginKey**: Typed struct `{scheme, host, port}` (not string wrapper). `from_url()`, `from_origin()`. IPv6 brackets in `Display`.
- **system_time_to_unix / unix_to_system_time**: Shared helpers in `browser_db/mod.rs`. `ts == 0` → `Some(UNIX_EPOCH)`, `ts < 0` → `None`.
- **History**: 2-table design (`urls` + `visits`). `record_visit` and `delete_range` use transactions. Frecency with time-decay buckets.
- **Cookies**: RFC 6265bis §5.7 full fields. `sync_all()` does DELETE + bulk INSERT in transaction. `PersistedCookie.delete()` includes `partition_key`.

### elidex-net (CookieJar, M4-8.5)

- **CookieJar**: RFC 6265bis §5.7 compliant. `generation()` / `snapshot()` / `load()` for decoupled persistence sync. `cookie_details_for_script()` returns full `CookieSnapshot` for CookieStore API.
- **Cookie deletion**: `Max-Age=0` returns cookie with `expires = UNIX_EPOCH` + `persistent = false` (not `None`). Caller's `retain()` removes existing cookie.
- **Host-only matching**: `cookie_domain_matches()` uses exact match for `host_only = true`.
- **`stored_to_snapshot()`**: Shared conversion, lowercase `same_site`.

### elidex-api-sw (router module, M4-8.5)

- **URLPattern**: `UrlPattern::pathname()` / `hostname_and_pathname()` return `Result<Self, String>`. Pattern syntax: `:name` → `([^/]+)`, `*` → `(.*)` with unique group names `*0`/`*1`. `test(&url::Url)` / `exec(&url::Url)` avoid re-parsing.
- **RouterSource**: `FetchEvent` / `Network` / `Cache(name)` / `RaceNetworkAndFetchHandler`.
- **ClientState enums**: `ClientType`, `FrameType`, `VisibilityState` (not strings).

### elidex-js-boa (M4-8.5 additions)

- **CookieStore API**: `globals/cookie_store.rs`. `get`/`getAll` return full `CookieListItem` (7 props, `READONLY`). `set` accepts `(name, value)` or options object. `delete` uses `Max-Age=0`. Registered in Window + SW contexts.
- **Response builder**: `fetch/mod.rs::build_response_from_parts(ResponseParts)` shared by fetch + cache. Provides `text()`/`json()`/`clone()`.
- **cached_entry_to_response**: Delegates to `build_response_from_parts`. `response.url` uses final URL from `response_url_list`.
- **ResponseType enum**: `entry.rs` — `Basic`/`Cors`/`Default`/`Error`/`Opaque`/`OpaqueRedirect`. `from_str_lossy` is case-insensitive.
- **HostBridge**: `client_id()` returns UUID v4. `enable_sw_messages()` for startMessages(). `cookie_details_for_script()` for CookieStore API.

### elidex-shell (M4-8.5 additions)

- **SwFetchRelay**: `app/sw_fetch_relay.rs` — `initiate()`/`resolve()`/`check_timeouts()`. Tracks pending fetch by `fetch_id`.
- **App.browser_db**: Initialized in `init_browser_db()`. Cookie sync via `sync_cookies_if_dirty()` in frame loop (generation-based dirty check, persistent cookies only).
- **content/event_loop.rs**: Extracted `run_event_loop()` + `handle_message()` from `content/mod.rs`.
- **Navigation SW filter**: Fragment-only skip, fetch_id verification loop, POST method/headers pass-through.

