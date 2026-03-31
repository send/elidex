//! Platform-specific process sandbox enforcement.
//!
//! Implements the sandbox mechanisms described in design doc §8.1:
//!
//! | Platform | Mechanism                    |
//! |----------|------------------------------|
//! | Linux    | seccomp-bpf + namespaces     |
//! | macOS    | `sandbox_init()` profiles    |
//! | Windows  | Restricted tokens + Job Objects |
//!
//! The [`SandboxEnforcer`] trait abstracts over platform differences.
//! [`apply_sandbox`] is the entry point called early in content thread
//! startup (before processing any messages).
//!
//! In `SingleProcess` mode, [`PlatformSandbox::Unsandboxed`] produces a
//! no-op. When multi-process isolation is enabled, the appropriate platform
//! variant is passed and enforcement is irreversible.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod noop;
#[cfg(target_os = "windows")]
mod windows;

use elidex_plugin::{PlatformSandbox, SandboxError, SandboxPolicy};

/// Platform-specific sandbox enforcer.
///
/// Implementations apply OS-level restrictions to the calling process/thread.
/// Once applied, restrictions are **irreversible** — the process cannot
/// regain dropped privileges.
pub trait SandboxEnforcer: Send {
    /// Apply the sandbox policy to the current process.
    ///
    /// This must be called before any untrusted code executes. After this
    /// call, system calls not in the allow-list will be blocked by the OS.
    fn apply(&self, policy: &SandboxPolicy) -> Result<(), SandboxError>;

    /// Human-readable name of the sandbox mechanism.
    fn name(&self) -> &str;
}

/// Apply the appropriate platform sandbox based on the [`PlatformSandbox`]
/// variant.
///
/// Called at the top of `content_thread_main()` before entering the event
/// loop. In `SingleProcess` mode, pass [`PlatformSandbox::Unsandboxed`]
/// for a no-op.
pub fn apply_sandbox(platform: &PlatformSandbox) -> Result<(), SandboxError> {
    match platform {
        #[cfg(target_os = "linux")]
        PlatformSandbox::LinuxSeccomp { policy } => linux::LinuxSeccompEnforcer.apply(policy),

        #[cfg(target_os = "macos")]
        PlatformSandbox::MacOSAppSandbox { policy } => macos::MacOSEnforcer.apply(policy),

        #[cfg(target_os = "windows")]
        PlatformSandbox::WindowsRestricted { policy } => {
            windows::WindowsRestrictedEnforcer.apply(policy)
        }

        PlatformSandbox::Unsandboxed => noop::NoopEnforcer.apply(&SandboxPolicy::permissive()),

        // Platform mismatch: requested a sandbox for a different OS.
        #[allow(unreachable_patterns)]
        _ => Err(SandboxError::new(
            "requested sandbox mechanism is not available on this platform",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::SandboxPolicy;

    #[test]
    fn unsandboxed_is_noop() {
        let result = apply_sandbox(&PlatformSandbox::Unsandboxed);
        assert!(result.is_ok());
    }

    #[test]
    fn noop_enforcer_name() {
        let enforcer = noop::NoopEnforcer;
        assert_eq!(enforcer.name(), "noop");
    }

    #[test]
    fn noop_enforcer_accepts_any_policy() {
        let enforcer = noop::NoopEnforcer;
        assert!(enforcer.apply(&SandboxPolicy::strict()).is_ok());
        assert!(enforcer.apply(&SandboxPolicy::permissive()).is_ok());
        assert!(enforcer.apply(&SandboxPolicy::web_content()).is_ok());
    }
}
