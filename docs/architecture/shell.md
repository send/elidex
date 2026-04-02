# Architecture: Shell & Canvas (elidex-shell, elidex-web-canvas)

## elidex-shell

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
- **SwFetchRelay**: `app/sw_fetch_relay.rs` — `initiate()`/`resolve()`/`check_timeouts()`. Tracks pending fetch by `fetch_id`.
- **App.browser_db**: Initialized in `init_browser_db()`. Cookie sync via `sync_cookies_if_dirty()` in frame loop (generation-based dirty check, persistent cookies only).
- **content/event_loop.rs**: Extracted `run_event_loop()` + `handle_message()` from `content/mod.rs`.
- **Navigation SW filter**: Fragment-only skip, fetch_id verification loop, POST method/headers pass-through.
- **Dependencies**: egui 0.33, egui-wgpu 0.33, egui-winit 0.33 (all MIT/Apache-2.0, wgpu 27 compatible), accesskit 0.24, accesskit_winit 0.32, elidex-a11y, crossbeam-channel 0.5.

## elidex-web-canvas

- **Canvas2dContext**: Wraps `tiny_skia::Pixmap` with `DrawingState` stack (fill/stroke color, line_width, global_alpha, transform). Default 300×150 pixels.
- **Drawing methods**: `fillRect`, `strokeRect`, `clearRect` (rectangle methods), `beginPath`/`moveTo`/`lineTo`/`closePath`/`rect`/`arc`/`fill`/`stroke` (path methods), `save`/`restore` (state), `translate`/`rotate`/`scale` (transform).
- **Image data**: `getImageData`/`putImageData`/`createImageData` with premultiplied↔straight alpha conversion. `to_rgba8_straight()` for ECS `ImageData` sync.
- **Arc approximation**: `arc_to_beziers()` converts Canvas 2D `arc()` to cubic Bezier curves — splits into ≤90° segments, k = (4/3)*tan(half_angle).
- **Style parsing**: `parse_color_string()` delegates to `elidex_css::parse_color` for CSS color string support.
- **Dependencies**: elidex-plugin (CssColor), elidex-css (parse_color), tiny-skia 0.11.
