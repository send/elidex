
# 8. Security Model

Security boundaries must be established at project inception as they fundamentally constrain the architecture.

## 8.1 Process Sandboxing

Renderer Processes are sandboxed with minimal system access:

| Platform | Sandbox Mechanism | Notes |
| --- | --- | --- |
| Linux | seccomp-bpf + namespaces | Chromium-proven approach |
| macOS | App Sandbox (sandbox-exec) | Restricts file system, network |
| Windows | Restricted tokens + job objects | Win32 API sandboxing |

The Browser Process acts as a privileged broker. Renderers request network access, file I/O, and clipboard operations through IPC, and the Browser Process enforces policy.

## 8.2 Web Security Policies

Same-Origin Policy, CORS, and CSP are implemented in the elidex-security crate. CORS is enforced by the Network Process (Ch. 10 §10.8); CSP is parsed by the Network Process and enforced by the Renderer. The elidex-origin crate manages origin computation and comparison, which must handle the full complexity of URL schemes, ports, and opaque origins.

## 8.3 Unified Permission Model

Elidex uses a single permission type across both browser and app modes. The permission set is the same; only the grant mechanism differs.

### 8.3.1 Permission Types

```rust
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum Permission {
    Geolocation,
    Camera,
    Microphone,
    CameraAndMicrophone,
    Notifications,
    ClipboardRead,
    ClipboardWrite,
    Push,
    BackgroundSync,
    PersistentStorage,
    Midi,
    MidiSysex,
    Bluetooth,
    Usb,
    Serial,
    Hid,
    ScreenCapture,
    WindowManagement,
    LocalFonts,
    IdleDetection,
    /// File system access beyond Origin Private File System
    FileSystemAccess,
    /// Sensor APIs (accelerometer, gyroscope, etc.)
    Sensors,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PermissionState {
    /// Access is allowed without further user interaction.
    Granted,
    /// Access is denied. In browser mode, the user explicitly declined.
    Denied,
    /// User has not yet been asked. Browser mode only.
    /// A permission request will trigger a user-visible prompt.
    Prompt,
}
```

### 8.3.2 Grant Mechanisms

| Mode | Default State | Grant Mechanism | Persistence |
| --- | --- | --- | --- |
| Browser | Prompt | Runtime user prompt | Per-origin in browser.sqlite (Ch. 22) |
| App | Denied | Build-time manifest / `App::grant()` | Static (compiled into app) |

In browser mode, when a web page calls a permission-gated API (e.g., `navigator.geolocation.getCurrentPosition()`), the permission check flows through the Browser Process:

```
Renderer (web content calls gated API)
  │  IPC: PermissionRequest { origin, permission }
  ▼
Browser Process: PermissionManager
  │  1. Check origin-level stored decision
  │     → If Granted or Denied: return immediately
  │  2. Check Permissions-Policy (document-level, §8.4)
  │     → If blocked by policy: return Denied
  │  3. State is Prompt: request UI from BrowserShell
  │     → BrowserShell shows prompt to user
  │     → User grants or denies
  │  4. Store decision per-origin
  │  IPC: PermissionResponse { state }
  ▼
Renderer (API call succeeds or fails)
```

In app mode, there is no prompt step. Permissions are resolved at build time:

```rust
let app = elidex_app::App::new()
    .grant(Permission::Camera)
    .grant(Permission::Microphone)
    .grant(Permission::FileSystemAccess)
    .deny(Permission::Geolocation)
    .build();
    // All other permissions default to Denied.
```

This capability-based model avoids Electron's historical problem where `nodeIntegration` exposed the full Node.js runtime to web content by default.

### 8.3.3 PermissionManager

The PermissionManager lives in the Browser Process and is the single authority for all permission decisions:

```rust
pub struct PermissionManager {
    /// Persistent per-origin decisions (browser mode)
    store: PermissionStore,
    /// Static grants (app mode)
    app_grants: HashMap<Permission, PermissionState>,
    /// Temporary session overrides (e.g., one-time grants)
    session_overrides: HashMap<(Origin, Permission), PermissionState>,
}

impl PermissionManager {
    /// Core permission check. Called on every gated API invocation.
    pub fn check(
        &self,
        origin: &Origin,
        permission: Permission,
        document_policy: &PermissionsPolicy,
        frame_policy: &FramePermissions,
    ) -> PermissionState {
        // Layer 1: Document-level Permissions-Policy header
        if !document_policy.allows(permission) {
            return PermissionState::Denied;
        }

        // Layer 2: Frame-level allow attribute (for iframes)
        if !frame_policy.allows(permission) {
            return PermissionState::Denied;
        }

        // Layer 3: Origin-level decision
        if let Some(state) = self.session_overrides.get(&(origin.clone(), permission)) {
            return *state;
        }

        if let Some(state) = self.store.get(origin, permission) {
            return state;
        }

        // No stored decision
        if !self.app_grants.is_empty() {
            // App mode: check static grants, default Denied
            self.app_grants.get(&permission).copied()
                .unwrap_or(PermissionState::Denied)
        } else {
            // Browser mode: Prompt
            PermissionState::Prompt
        }
    }

    /// Store a user decision (browser mode only).
    pub fn set_decision(
        &mut self,
        origin: &Origin,
        permission: Permission,
        state: PermissionState,
        duration: DecisionDuration,
    ) {
        match duration {
            DecisionDuration::Persistent => {
                self.store.set(origin, permission, state);
            }
            DecisionDuration::Session => {
                self.session_overrides.insert((origin.clone(), permission), state);
            }
        }
    }

    /// Reset all permissions for an origin (user action: "reset site permissions").
    pub fn reset_origin(&mut self, origin: &Origin) {
        self.store.remove_origin(origin);
        self.session_overrides.retain(|(o, _), _| o != origin);
    }
}
```

### 8.3.4 Permission Storage

Permission decisions are stored in `browser.sqlite` (Ch. 22, browser-owned data):

```sql
CREATE TABLE permissions (
    origin     TEXT NOT NULL,
    permission TEXT NOT NULL,
    state      TEXT NOT NULL CHECK (state IN ('granted', 'denied')),
    granted_at INTEGER NOT NULL,  -- unix timestamp
    PRIMARY KEY (origin, permission)
);
```

On profile clear ("clear site data"), all permission rows for the affected origins are deleted. On browser-wide reset, the table is truncated.

### 8.3.5 Permissions API (Web)

The `navigator.permissions` API exposes permission state to web content:

```rust
// navigator.permissions.query({ name: "geolocation" })
pub async fn query_permission(
    &self,
    origin: &Origin,
    descriptor: PermissionDescriptor,
) -> PermissionStatus {
    let state = self.permission_manager.check(
        origin,
        descriptor.name,
        &self.document_policy,
        &self.frame_permissions,
    );

    PermissionStatus {
        state,
        // PermissionStatus.onchange fires when state changes
        // (e.g., user revokes permission in browser settings)
        change_signal: self.watch(origin, descriptor.name),
    }
}
```

The `onchange` event on `PermissionStatus` is fired when the user modifies permissions through browser settings. The PermissionManager broadcasts changes to all Renderer Processes holding a PermissionStatus for the affected origin+permission.

## 8.4 Permissions-Policy

The Permissions-Policy HTTP header (formerly Feature-Policy) allows servers to control which permissions are available in a document and its embedded frames.

### 8.4.1 Policy Parsing

```
Permissions-Policy: camera=(), microphone=(self "https://meet.example.com"), geolocation=*
```

This header means: camera is blocked entirely, microphone is allowed for the document's own origin and `https://meet.example.com`, geolocation is allowed for all origins.

```rust
pub struct PermissionsPolicy {
    directives: HashMap<Permission, AllowList>,
}

pub enum AllowList {
    /// No origin is allowed.
    None,
    /// Only the document's own origin.
    SelfOnly,
    /// Specific origins.
    Origins(Vec<Origin>),
    /// All origins.
    All,
}
```

### 8.4.2 Iframe Integration

The `<iframe>` `allow` attribute provides frame-level policy:

```html
<iframe src="https://meet.example.com"
        allow="camera; microphone">
</iframe>
```

```rust
pub struct FramePermissions {
    /// Permissions explicitly allowed by the parent frame's allow attribute.
    allowed: HashSet<Permission>,
    /// The parent document's Permissions-Policy (inherited).
    parent_policy: Arc<PermissionsPolicy>,
}

impl FramePermissions {
    pub fn allows(&self, permission: Permission) -> bool {
        // The iframe must be explicitly granted AND the parent policy must allow
        self.allowed.contains(&permission)
            && self.parent_policy.allows_for_origin(permission, &self.frame_origin)
    }
}
```

Permissions-Policy is enforced even if the origin has a stored `Granted` decision. A server can revoke a permission for a specific document regardless of user-level grants.

### 8.4.3 Default Policy

For permissions not mentioned in the header, the default policy applies. Most permissions default to `self` (allowed only for the document's own origin, not embedded iframes). Some permissions default to `*` (e.g., `autoplay` in some browsers). Elidex follows the W3C Permissions-Policy specification defaults.

## 8.5 Permission-Gated Features

| Permission | API | Sensitivity | Notes |
| --- | --- | --- | --- |
| Geolocation | `navigator.geolocation` | High | Physical location tracking |
| Camera | `getUserMedia({ video })` | High | Video capture |
| Microphone | `getUserMedia({ audio })` | High | Audio capture |
| Notifications | `Notification` constructor | Medium | System-level alerts |
| ClipboardRead | `navigator.clipboard.read()` | Medium | Access to clipboard contents |
| ClipboardWrite | `navigator.clipboard.write()` | Low | Most browsers auto-grant for user gesture |
| Push | `PushManager.subscribe()` | Medium | Background messaging |
| PersistentStorage | `navigator.storage.persist()` | Low | Prevents eviction of site data |
| Midi | `navigator.requestMIDIAccess()` | Medium | MIDI device access |
| MidiSysex | `requestMIDIAccess({ sysex: true })` | High | System-exclusive MIDI (firmware writes) |
| Bluetooth | `navigator.bluetooth.requestDevice()` | High | Bluetooth device pairing |
| Usb | `navigator.usb.requestDevice()` | High | USB device access |
| Serial | `navigator.serial.requestPort()` | High | Serial port access |
| Hid | `navigator.hid.requestDevice()` | High | HID device access |
| ScreenCapture | `getDisplayMedia()` | High | Screen/window recording |
| LocalFonts | `queryLocalFonts()` | Medium | Fingerprinting vector |
| IdleDetection | `IdleDetector.start()` | Medium | Monitors user activity |
| Sensors | `Accelerometer`, `Gyroscope` | Medium | Motion/orientation data |

High-sensitivity permissions require a user gesture (click, key press) to trigger the prompt. Without a user gesture, the request is silently denied.

## 8.6 Permission Prompt UI

Permission prompts are the responsibility of the BrowserShell (Ch. 24). The PermissionManager requests a prompt through the shell trait:

```rust
pub trait PermissionPrompter {
    /// Show a permission prompt to the user.
    /// Returns the user's decision, or None if the prompt was dismissed.
    async fn show_prompt(
        &self,
        request: PermissionPromptRequest,
    ) -> Option<PermissionDecision>;
}

pub struct PermissionPromptRequest {
    pub origin: Origin,
    pub permission: Permission,
    pub has_user_gesture: bool,
    pub favicon: Option<ImageData>,
}

pub struct PermissionDecision {
    pub state: PermissionState,
    pub duration: DecisionDuration,
}

pub enum DecisionDuration {
    /// Remember for future visits
    Persistent,
    /// Only for this session (cleared on browser exit)
    Session,
}
```

### 8.6.1 Prompt Behavior

| Behavior | Rule |
| --- | --- |
| One prompt at a time | Queue concurrent requests. Never stack multiple prompts. |
| User gesture required | High-sensitivity permissions require a user gesture. No gesture → silent deny. |
| Dismiss = Deny (session) | Closing the prompt without choosing is session-scoped Deny. User can try again next visit. |
| Repeated denial | After 3 consecutive denials for the same origin+permission, auto-deny without prompt. Reset via site settings. |
| Background tabs | Prompts suppressed for background tabs. Request queued until tab is focused. |

### 8.6.2 Native vs. Self-Hosted Chrome

| Chrome Mode | Prompt Implementation |
| --- | --- |
| Native (egui/iced, Phase 1–2) | Platform-appropriate dialog or inline bar in browser chrome. |
| Self-hosted (HTML/CSS, Phase 3+) | HTML-rendered prompt within browser chrome document. Rendered in Browser Process, not requesting Renderer, to prevent spoofing. |

## 8.7 Permission Revocation

Users can revoke permissions through:

| Mechanism | Scope |
| --- | --- |
| Site info panel (padlock icon) | Per-origin, per-permission |
| Browser settings → Site permissions | Per-origin or global |
| Clear browsing data | All permissions for affected origins |

When a permission is revoked, the PermissionManager broadcasts a `PermissionChanged` event to all Renderer Processes with active `PermissionStatus` watchers. Active sessions using the permission (e.g., an ongoing getUserMedia stream) are terminated.

```rust
pub enum PermissionEvent {
    Changed {
        origin: Origin,
        permission: Permission,
        new_state: PermissionState,
    },
    OriginReset { origin: Origin },
}
```

## 8.8 App Mode Security

### 8.8.1 Capability Model

In elidex-app mode, permissions are static capabilities declared at build time. There is no user prompt:

```rust
let app = elidex_app::App::new()
    .grant(Permission::Camera)
    .grant(Permission::Microphone)
    .grant(Permission::FileSystemAccess)
    .deny(Permission::Geolocation)
    // All other permissions default to Denied
    .build();
```

Conceptually similar to Android's `AndroidManifest.xml` permissions or macOS App Sandbox entitlements, but enforced at the engine level.

### 8.8.2 Extended App Capabilities

App mode supports additional capabilities beyond web permissions, corresponding to the elevated trust of a locally-installed application:

```rust
pub enum AppCapability {
    /// All web-standard permissions
    WebPermission(Permission),

    /// File system access beyond OPFS
    FileRead(PathPattern),
    FileWrite(PathPattern),

    /// Unrestricted network access (bypass CORS)
    NetworkUnrestricted,

    /// Access to system commands
    ProcessSpawn(Vec<String>),

    /// Access to environment variables
    EnvRead(Vec<String>),

    /// IPC with other elidex-app instances or native processes
    Ipc,
}
```

Each extended capability is checked by the Browser Process (or the merged single-process in app mode) before the operation proceeds. Undeclared capabilities are denied at runtime with an error.

### 8.8.3 Capability Audit

At app startup, all granted capabilities are logged. In debug builds, capability checks emit tracing spans so developers can verify their app only uses the permissions it declares.

```rust
impl CapabilityChecker {
    pub fn check(&self, capability: &AppCapability) -> Result<(), CapabilityError> {
        if self.is_granted(capability) {
            tracing::trace!(?capability, "capability check passed");
            Ok(())
        } else {
            tracing::warn!(?capability, "capability denied — not granted in manifest");
            Err(CapabilityError::NotGranted(capability.clone()))
        }
    }
}
```
