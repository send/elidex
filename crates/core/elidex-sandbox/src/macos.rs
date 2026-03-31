//! macOS sandbox enforcer using `sandbox_init()`.
//!
//! Uses the `sandbox_init()` API to apply a Scheme-Based Profile (SBPL)
//! that restricts filesystem, network, and process capabilities.
//!
//! Design reference: docs/design/en/08-security-model.md §8.1 (macOS row).
//!
//! Note: `sandbox_init()` is deprecated by Apple but remains functional and
//! is used by Chromium. The replacement (App Sandbox entitlements) requires
//! code signing and cannot be applied at runtime.

use std::ffi::CString;
use std::ptr;

use elidex_plugin::sandbox::{FilesystemAccess, NetworkAccess};
use elidex_plugin::{SandboxError, SandboxPolicy};

use crate::SandboxEnforcer;

pub struct MacOSEnforcer;

impl SandboxEnforcer for MacOSEnforcer {
    fn apply(&self, policy: &SandboxPolicy) -> Result<(), SandboxError> {
        let profile = build_profile(policy);
        let c_profile = CString::new(profile)
            .map_err(|e| SandboxError::new(format!("invalid profile: {e}")))?;

        let mut err_buf: *mut libc::c_char = ptr::null_mut();

        // SAFETY: `sandbox_init` is a stable macOS API. We pass a valid
        // C string and a pointer to receive error messages. The error
        // string (if any) must be freed with `sandbox_free_error`.
        //
        // Flags: 0 = raw SBPL profile string. The profile must start with
        // `(version 1)`. SANDBOX_NAMED (0x0001) would interpret it as a
        // profile name lookup instead.
        let ret = unsafe { sandbox_init(c_profile.as_ptr(), 0, &mut err_buf) };

        if ret != 0 {
            let msg = if err_buf.is_null() {
                "sandbox_init failed (no error message)".to_string()
            } else {
                // SAFETY: err_buf points to a NUL-terminated C string
                // allocated by the sandbox API.
                let msg = unsafe { std::ffi::CStr::from_ptr(err_buf) }
                    .to_string_lossy()
                    .to_string();
                unsafe { sandbox_free_error(err_buf) };
                msg
            };
            return Err(SandboxError::new(msg));
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "macos-sandbox-init"
    }
}

extern "C" {
    fn sandbox_init(
        profile: *const libc::c_char,
        flags: u64,
        errorbuf: *mut *mut libc::c_char,
    ) -> libc::c_int;

    fn sandbox_free_error(errorbuf: *mut libc::c_char);
}

/// Build a Scheme-Based Profile Language (SBPL) string from the policy.
fn build_profile(policy: &SandboxPolicy) -> String {
    let mut rules = vec![
        "(version 1)".to_string(),
        "(deny default)".to_string(),
        // Always allow: signals, sysctl reads.
        "(allow signal (target self))".to_string(),
        "(allow sysctl-read)".to_string(),
        "(allow process-info-pidinfo)".to_string(),
    ];

    // IPC: mach-lookup is required for cross-process communication.
    if policy.ipc {
        rules.push("(allow mach-lookup)".to_string());
        rules.push("(allow ipc-posix-shm-read-data)".to_string());
        rules.push("(allow ipc-posix-shm-write-data)".to_string());
    }

    // Filesystem access.
    match policy.filesystem {
        FilesystemAccess::None => {
            // Allow reading system libraries (required for process to function).
            rules.push(
                "(allow file-read-data (subpath \"/usr/lib\") (subpath \"/System\") (subpath \"/Library/Frameworks\") (subpath \"/private/var/db/dyld\"))"
                    .to_string(),
            );
            rules.push(
                "(allow file-read-metadata (subpath \"/usr/lib\") (subpath \"/System\"))"
                    .to_string(),
            );
        }
        FilesystemAccess::ReadOnly => {
            rules.push("(allow file-read*)".to_string());
        }
        FilesystemAccess::ReadWrite => {
            rules.push("(allow file-read*)".to_string());
            rules.push("(allow file-write*)".to_string());
        }
        _ => {
            // Future variants: default to no filesystem access.
        }
    }

    // Network access.
    match policy.network {
        NetworkAccess::None => {
            // No network rules — outbound blocked by deny default.
        }
        NetworkAccess::SameOrigin | NetworkAccess::Full => {
            // NOTE: SBPL cannot enforce same-origin at the OS level.
            // Same-origin restriction is enforced at the application layer
            // (Network Process validates origins). OS sandbox allows outbound.
            rules.push("(allow network-outbound)".to_string());
            rules.push("(allow network-inbound)".to_string());
            rules.push("(allow system-socket)".to_string());
        }
        _ => {
            // Future variants: default to no network access.
        }
    }

    // GPU access (IOKit for Metal/OpenGL).
    if policy.gpu {
        rules.push("(allow iokit-open)".to_string());
    }

    // Never allow process creation.
    // (deny process-exec) is implicit from deny default.

    rules.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::SandboxPolicy;

    #[test]
    fn profile_strict() {
        let profile = build_profile(&SandboxPolicy::strict());
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(allow mach-lookup)"));
        assert!(!profile.contains("(allow file-write*)"));
        // Strict has SameOrigin network — OS allows outbound, app layer enforces origin.
        assert!(profile.contains("(allow network-outbound)"));
        assert!(!profile.contains("(allow iokit-open)")); // no GPU
    }

    #[test]
    fn profile_web_content() {
        let profile = build_profile(&SandboxPolicy::web_content());
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(allow mach-lookup)"));
        assert!(profile.contains("(allow iokit-open)"));
        assert!(profile.contains("(allow network-outbound)"));
        assert!(!profile.contains("(allow file-write*)"));
    }

    #[test]
    fn profile_permissive() {
        let profile = build_profile(&SandboxPolicy::permissive());
        assert!(profile.contains("(allow file-read*)"));
        assert!(profile.contains("(allow file-write*)"));
        assert!(profile.contains("(allow network-outbound)"));
        assert!(profile.contains("(allow iokit-open)"));
    }

    #[test]
    fn enforcer_name() {
        assert_eq!(MacOSEnforcer.name(), "macos-sandbox-init");
    }

    // NOTE: Actually applying the sandbox would restrict THIS test process.
    // Integration tests for enforcement should run in a child process.
}
