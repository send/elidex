
# 24. Browser Shell

The browser shell is everything above the engine and platform layers: tabs, navigation, address bar, bookmarks, settings, DevTools, and the visual chrome that users interact with. Unlike the engine layer (where web standards define core/compat boundaries), the shell has no authoritative specification. The pluggability philosophy here is **trait-contract based**: elidex defines minimal behavioral interfaces, and implementations can be swapped or customized entirely.

## 24.1 Design Philosophy

| Layer | Plugin Basis | Pluggability Direction |
| --- | --- | --- |
| Engine | Web standards + SpecLevel | **Trim** — remove legacy features |
| Platform | OS APIs + trait contract | **Swap** — inject OS implementations |
| Browser Shell | Trait contract only | **Extend / Replace** — customize UI and UX |

The shell is designed so that the entire browser UI can be replaced without modifying the engine. This enables use cases such as: a company building a branded browser on elidex (custom chrome, same engine), a kiosk mode with stripped-down UI, an accessibility-focused browser with a radically different interface, or a research browser experimenting with novel navigation paradigms.

## 24.2 Shell Architecture

The shell is split into a platform-independent state/logic layer and a platform-dependent (or framework-dependent) UI layer:

```
┌─────────────────────────────────┐
│  Browser Chrome (UI)            │  ← Pluggable: trait-based
│  Tab bar, address bar,          │     Can be self-hosted (HTML/CSS),
│  bookmarks bar, settings,       │     native toolkit, or Rust GUI
│  DevTools, download shelf       │
├─────────────────────────────────┤
│  Shell State Manager            │  ← Platform-independent logic
│  TabManager, NavigationManager, │     Manages state and coordinates
│  BookmarkStore, SettingsStore,  │     between chrome and engine
│  DownloadManager, ProfileManager│
├─────────────────────────────────┤
│  Platform Abstraction (Ch. 23)  │  ← OS integration
├─────────────────────────────────┤
│  Engine (Ch. 5–22)              │  ← Web content rendering
└─────────────────────────────────┘
```

### 24.2.1 Shell State Manager

The state manager holds all browser-level state and exposes it through traits. It is platform-independent and has no UI code:

```rust
pub trait TabManager: Send + Sync {
    fn create_tab(&mut self, url: Url, position: TabPosition) -> TabId;
    fn close_tab(&mut self, id: TabId) -> Result<()>;
    fn active_tab(&self) -> TabId;
    fn set_active(&mut self, id: TabId);
    fn tabs(&self) -> &[TabInfo];
    fn move_tab(&mut self, id: TabId, position: TabPosition);
    fn duplicate_tab(&mut self, id: TabId) -> TabId;
}

pub trait NavigationManager: Send + Sync {
    fn navigate(&mut self, tab: TabId, url: Url);
    fn back(&mut self, tab: TabId) -> bool;
    fn forward(&mut self, tab: TabId) -> bool;
    fn reload(&mut self, tab: TabId);
    fn stop(&mut self, tab: TabId);
    fn history(&self, tab: TabId) -> &[HistoryEntry];
}

pub trait BookmarkStore: Send + Sync {
    fn add(&mut self, bookmark: Bookmark) -> BookmarkId;
    fn remove(&mut self, id: BookmarkId);
    fn list(&self, folder: Option<FolderId>) -> Vec<BookmarkEntry>;
    fn search(&self, query: &str) -> Vec<BookmarkEntry>;
    fn import(&mut self, format: BookmarkFormat, data: &[u8]) -> Result<usize>;
    fn export(&self, format: BookmarkFormat) -> Result<Vec<u8>>;
}

pub trait DownloadManager: Send + Sync {
    fn start(&mut self, url: Url, destination: Option<PathBuf>) -> DownloadId;
    fn pause(&mut self, id: DownloadId);
    fn resume(&mut self, id: DownloadId);
    fn cancel(&mut self, id: DownloadId);
    fn downloads(&self) -> &[DownloadInfo];
}

pub trait SettingsStore: Send + Sync {
    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    fn set<T: Serialize>(&mut self, key: &str, value: &T);
    fn reset(&mut self, key: &str);
    fn observe(&mut self, key: &str, cb: SettingsCallback) -> ObserverId;
}

pub trait ProfileManager: Send + Sync {
    fn current(&self) -> &Profile;
    fn profiles(&self) -> &[Profile];
    fn switch(&mut self, id: ProfileId);
    fn create(&mut self, name: &str) -> ProfileId;
}
```

These traits serve the same role as SpecLevel in the engine layer: they define the **contract** that any browser shell implementation must satisfy. The default elidex-browser provides a full implementation, but any of these can be swapped.

### 24.2.2 Browser Chrome (UI Layer)

The chrome layer renders the browser UI and translates user actions into calls to the shell state manager. This is the most replaceable part of the stack.

Elidex's approach to chrome rendering is a phased decision:

| Phase | Chrome Approach | Rationale |
| --- | --- | --- |
| Phase 1–2 | Minimal native chrome (winit + egui/iced) | Engine not mature enough for self-hosting. Need basic UI to test engine. |
| Phase 3+ | Self-hosted option available (elidex renders its own chrome via HTML/CSS) | Engine is capable. Dogfooding validates the engine with complex real-world UI. |
| Long-term | Both coexist; selectable at build time | Native chrome for performance-critical use; self-hosted for maximum customization. |

The chrome trait:

```rust
pub trait BrowserChrome: Send + Sync {
    fn init(&mut self, state: &dyn ShellState, platform: &dyn PlatformProvider);
    fn render_frame(&mut self, state: &dyn ShellState);
    fn handle_event(&mut self, event: ChromeEvent) -> ChromeAction;
    fn layout_regions(&self) -> ChromeLayout;
}

pub struct ChromeLayout {
    pub content_area: Rect,   // Where the web content viewport goes
    pub tab_bar: Option<Rect>,
    pub address_bar: Option<Rect>,
    pub sidebar: Option<Rect>,
}
```

This allows the engine's compositor to know where to place the web content viewport, regardless of how the chrome is implemented.

## 24.3 Extension Integration

Browser extensions (ad blockers, password managers, etc.) interact with both the engine layer (via NetworkMiddleware, DOM access) and the shell layer (via toolbar buttons, sidebars, popup windows). The shell provides extension mounting points:

```rust
pub trait ExtensionHost: Send + Sync {
    fn register_toolbar_button(&mut self, ext: ExtensionId, button: ToolbarButton);
    fn register_sidebar(&mut self, ext: ExtensionId, sidebar: SidebarConfig);
    fn register_context_menu(&mut self, ext: ExtensionId, items: Vec<ContextMenuItem>);
    fn register_page_action(&mut self, ext: ExtensionId, action: PageAction);
    fn open_popup(&mut self, ext: ExtensionId, url: Url, size: Size);
}
```

The extension API design is deferred to later phases (Phase 4+), but the shell architecture accommodates it from the start by providing these trait-based mounting points.

## 24.4 DevTools

DevTools is implemented as a special browser chrome component that connects to the engine's internal state:

| DevTools Panel | Engine Connection |
| --- | --- |
| Elements | ECS DOM tree inspection, ScriptSession mutation history |
| Styles | CSSOM inspection, computed style queries |
| Console | ScriptEngine eval, log capture |
| Network | NetworkMiddleware pipeline inspection (Chapter 10) |
| Performance | ECS system tick timing, rendering pipeline profiling |
| Sources | ScriptEngine debugger protocol |

DevTools is a strong candidate for self-hosted chrome (rendered by the engine itself), since it is essentially a complex web application that exercises the engine thoroughly.

## 24.5 Crate Structure

```
elidex-shell/
├── elidex-shell-api/          # Trait definitions (TabManager, NavigationManager, etc.)
├── elidex-shell-state/        # Default implementations of shell state managers
├── elidex-chrome-native/      # Native chrome (egui/iced, Phase 1–2)
├── elidex-chrome-selfhost/    # Self-hosted chrome (HTML/CSS, Phase 3+)
├── elidex-devtools/           # DevTools implementation
└── elidex-extension-host/     # Extension mounting points and lifecycle
```
