//! Sandbox policy type definitions.
//!
//! These types describe the security constraints applied to content processes.
//! Enforcement is implemented in the `elidex-sandbox` crate using platform-
//! specific mechanisms (seccomp-bpf, sandbox-exec, restricted tokens).

use std::fmt;

/// Error returned when sandbox enforcement fails.
#[derive(Debug, Clone)]
pub struct SandboxError {
    /// Human-readable description of the failure.
    pub message: String,
}

impl SandboxError {
    /// Create a new `SandboxError` with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sandbox error: {}", self.message)
    }
}

impl std::error::Error for SandboxError {}

/// Filesystem access level for a sandboxed process.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FilesystemAccess {
    /// No filesystem access allowed.
    #[default]
    None,
    /// Read-only access to allowed paths.
    ReadOnly,
    /// Full read-write access to allowed paths.
    ReadWrite,
}

/// Network access level for a sandboxed process.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NetworkAccess {
    /// No network access allowed.
    None,
    /// Only same-origin requests allowed.
    #[default]
    SameOrigin,
    /// Unrestricted network access.
    Full,
}

/// Security policy applied to a content process.
///
/// Defines what system resources a sandboxed process may access.
/// Currently type-only; enforcement requires OS process isolation (future phase).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SandboxPolicy {
    /// Filesystem access level.
    pub filesystem: FilesystemAccess,
    /// Network access level.
    pub network: NetworkAccess,
    /// Whether IPC with the browser process is allowed.
    pub ipc: bool,
    /// Whether GPU access is allowed.
    pub gpu: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::strict()
    }
}

impl SandboxPolicy {
    /// Strict policy: no filesystem, same-origin network, IPC only.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            filesystem: FilesystemAccess::None,
            network: NetworkAccess::SameOrigin,
            ipc: true,
            gpu: false,
        }
    }

    /// Permissive policy: full access to all resources.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            filesystem: FilesystemAccess::ReadWrite,
            network: NetworkAccess::Full,
            ipc: true,
            gpu: true,
        }
    }

    /// Web content policy: no filesystem, same-origin network, IPC + GPU.
    #[must_use]
    pub fn web_content() -> Self {
        Self {
            filesystem: FilesystemAccess::None,
            network: NetworkAccess::SameOrigin,
            ipc: true,
            gpu: true,
        }
    }
}

/// Platform-specific sandbox implementation.
///
/// Each variant carries a [`SandboxPolicy`] describing the desired constraints.
/// The `Unsandboxed` variant is used when no OS-level sandboxing is available
/// (e.g. single-process mode).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlatformSandbox {
    /// Linux seccomp-bpf sandbox.
    LinuxSeccomp {
        /// The sandbox policy to enforce.
        policy: SandboxPolicy,
    },
    /// macOS App Sandbox.
    MacOSAppSandbox {
        /// The sandbox policy to enforce.
        policy: SandboxPolicy,
    },
    /// Windows restricted token sandbox.
    WindowsRestricted {
        /// The sandbox policy to enforce.
        policy: SandboxPolicy,
    },
    /// No OS-level sandboxing.
    Unsandboxed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_policy() {
        let p = SandboxPolicy::strict();
        assert_eq!(p.filesystem, FilesystemAccess::None);
        assert_eq!(p.network, NetworkAccess::SameOrigin);
        assert!(p.ipc);
        assert!(!p.gpu);
    }

    #[test]
    fn permissive_policy() {
        let p = SandboxPolicy::permissive();
        assert_eq!(p.filesystem, FilesystemAccess::ReadWrite);
        assert_eq!(p.network, NetworkAccess::Full);
        assert!(p.ipc);
        assert!(p.gpu);
    }

    #[test]
    fn web_content_policy() {
        let p = SandboxPolicy::web_content();
        assert_eq!(p.filesystem, FilesystemAccess::None);
        assert_eq!(p.network, NetworkAccess::SameOrigin);
        assert!(p.ipc);
        assert!(p.gpu);
    }

    #[test]
    fn default_is_strict() {
        let p = SandboxPolicy::default();
        assert_eq!(p, SandboxPolicy::strict());
    }

    #[test]
    fn platform_sandbox_variants() {
        let policy = SandboxPolicy::web_content();
        let linux = PlatformSandbox::LinuxSeccomp {
            policy: policy.clone(),
        };
        let macos = PlatformSandbox::MacOSAppSandbox {
            policy: policy.clone(),
        };
        let windows = PlatformSandbox::WindowsRestricted { policy };
        let unsandboxed = PlatformSandbox::Unsandboxed;

        // Verify Debug + PartialEq work.
        assert_ne!(linux, unsandboxed);
        assert_ne!(macos, unsandboxed);
        assert_ne!(windows, unsandboxed);
        assert_eq!(unsandboxed, PlatformSandbox::Unsandboxed);
    }
}
