
# 23. Platform Abstraction Layer

The engine chapters (Chapters 5–22) describe platform-independent internals. This chapter defines the abstraction layer that bridges those internals to the host operating system. The design philosophy shifts from the engine's spec-driven pluggability (core/compat/deprecated based on web standards) to **trait-contract pluggability**, where OS-specific implementations are injected via platform plugins.

| Layer | Plugin Basis | Philosophy |
| --- | --- | --- |
| Engine (HTML/CSS/DOM/JS) | Web standards + SpecLevel | **Trim**: core/compat/deprecated |
| Platform Abstraction | OS APIs + trait contract | **Swap**: inject OS-specific implementations |
| Browser Shell (Chapter 24) | Trait contract only | **Extend/Replace**: customizable UI and UX |

## 23.1 Platform Trait Architecture

Each OS-dependent subsystem is defined as a trait. A platform plugin provides concrete implementations for a specific OS:

```rust
pub trait PlatformProvider: Send + Sync {
    fn windowing(&self) -> &dyn WindowManager;
    fn input(&self) -> &dyn InputManager;
    fn clipboard(&self) -> &dyn ClipboardManager;
    fn ime(&self) -> &dyn ImeManager;
    fn file_dialogs(&self) -> &dyn FileDialogManager;
    fn notifications(&self) -> &dyn NotificationManager;
    fn drag_drop(&self) -> &dyn DragDropManager;
    fn accessibility(&self) -> &dyn PlatformAccessibility;
    fn surface(&self) -> &dyn RenderSurface;
}
```

At startup, elidex selects the appropriate PlatformProvider based on the compilation target:

```
[features]
platform-linux = ["elidex-platform-linux"]
platform-macos = ["elidex-platform-macos"]
platform-windows = ["elidex-platform-windows"]
```

This is the same Cargo feature flag mechanism used in the engine layer, but applied to OS-level concerns rather than web specifications.

## 23.2 Subsystem Traits

### 23.2.1 Window Management

```rust
pub trait WindowManager: Send + Sync {
    fn create_window(&mut self, config: WindowConfig) -> Result<WindowId>;
    fn close_window(&mut self, id: WindowId);
    fn resize(&mut self, id: WindowId, size: PhysicalSize);
    fn set_fullscreen(&mut self, id: WindowId, mode: FullscreenMode);
    fn set_title(&mut self, id: WindowId, title: &str);
    fn request_redraw(&mut self, id: WindowId);
    fn monitors(&self) -> Vec<MonitorInfo>;
    fn scale_factor(&self, id: WindowId) -> f64;
}
```

winit is the likely foundation, providing cross-platform window creation and event loop integration. However, the trait abstraction allows replacing or wrapping winit where browser-specific requirements exceed its capabilities (e.g., custom title bar rendering, window-level tab management).

### 23.2.2 Input

```rust
pub trait InputManager: Send + Sync {
    fn keyboard_layout(&self) -> KeyboardLayout;
    fn pointer_capabilities(&self) -> PointerCapabilities;
    fn register_global_shortcut(&mut self, shortcut: Shortcut) -> Result<ShortcutId>;
    fn cursor_position(&self) -> Option<PhysicalPosition>;
}
```

Keyboard, mouse, touch, and pen events are normalized to a platform-independent event stream. OS-specific keyboard layouts and input quirks are handled by the platform plugin.

### 23.2.3 IME (Input Method Editor)

```rust
pub trait ImeManager: Send + Sync {
    fn activate(&mut self, config: ImeConfig);
    fn deactivate(&mut self);
    fn set_cursor_area(&mut self, rect: Rect);
    fn composition_state(&self) -> Option<CompositionState>;
}
```

IME integration is critical for CJK language support. Each OS has a fundamentally different IME protocol (IBus/Fcitx on Linux, Input Method Kit on macOS, TSF on Windows). The trait provides a unified interface; platform plugins handle the OS-specific protocol. This directly supports the elidex-text CJK and vertical writing pipeline (Chapter 16).

### 23.2.4 Clipboard and Drag & Drop

```rust
pub trait ClipboardManager: Send + Sync {
    fn read_text(&self) -> Result<Option<String>>;
    fn write_text(&mut self, text: &str) -> Result<()>;
    fn read_rich(&self) -> Result<Option<ClipboardContent>>;
    fn write_rich(&mut self, content: ClipboardContent) -> Result<()>;
    fn available_formats(&self) -> Vec<ClipboardFormat>;
}

pub trait DragDropManager: Send + Sync {
    fn start_drag(&mut self, data: DragData, image: Option<DragImage>) -> Result<()>;
    fn register_drop_target(&mut self, window: WindowId, handler: Box<dyn DropHandler>);
}
```

### 23.2.5 Render Surface

```rust
pub trait RenderSurface: Send + Sync {
    fn create_surface(&mut self, window: WindowId) -> Result<wgpu::Surface>;
    fn preferred_format(&self) -> wgpu::TextureFormat;
}
```

This connects the platform's window system to wgpu, providing the surface that the Vello-based rendering pipeline (Chapter 15) draws to.

### 23.2.6 Platform Accessibility

```rust
pub trait PlatformAccessibility: Send + Sync {
    fn init(&mut self, window: WindowId) -> Result<()>;
    fn update_tree(&mut self, tree: &AccessibilityTree);
    fn handle_action(&self, action: AccessibilityAction) -> Result<()>;
}
```

| Platform | API | Notes |
| --- | --- | --- |
| Linux | AT-SPI2 (via atspi crate) | D-Bus based. Screen reader (Orca) communication. |
| macOS | NSAccessibility | Objective-C bridge. VoiceOver communication. |
| Windows | UI Automation (UIA) | COM-based. Narrator/JAWS/NVDA communication. |

AccessKit (Chapter 25) provides the cross-platform accessibility tree; the platform plugin translates it to the OS-specific protocol.

## 23.3 elidex-app Platform Usage

In elidex-app mode, the platform abstraction layer is shared with elidex-browser but with reduced scope:

| Subsystem | elidex-browser | elidex-app |
| --- | --- | --- |
| Window management | Full (tabs, popups, DevTools windows) | Single or multi-window (app-defined) |
| Input | Full (keyboard, mouse, touch, pen) | Same |
| IME | Full | Same |
| Clipboard | Full | Same |
| File dialogs | Full | Optional (capability-gated, Chapter 8) |
| Notifications | Full | Optional (capability-gated) |
| Drag & drop | Full | Optional |
| Accessibility | Full | Same (required) |
| Render surface | Full | Same |

The same PlatformProvider trait serves both use cases. elidex-app simply uses a subset of the capabilities, controlled by the capability-based security model (Chapter 8).

## 23.4 Crate Structure

```
elidex-platform/
├── elidex-platform-api/       # Trait definitions (PlatformProvider, all subsystem traits)
├── elidex-platform-linux/     # Linux implementation (X11/Wayland, IBus/Fcitx, AT-SPI2)
├── elidex-platform-macos/     # macOS implementation (Cocoa, Input Method Kit, NSAccessibility)
├── elidex-platform-windows/   # Windows implementation (Win32, TSF, UIA)
└── elidex-platform-common/    # Shared utilities (event normalization, key mapping tables)
```
