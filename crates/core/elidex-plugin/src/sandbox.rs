//! Sandboxing — the two independent planes elidex enforces.
//!
//! 1. **Process sandbox** (`SandboxPolicy` / `PlatformSandbox`): security
//!    constraints applied to content *processes*. Enforcement is implemented
//!    in the `elidex-sandbox` crate using platform-specific mechanisms
//!    (seccomp-bpf, sandbox-exec, restricted tokens).
//! 2. **Content sandbox** (the capability predicates below): WHATWG HTML
//!    §7.1.5 `<iframe sandbox>` browsing-context restrictions, decided over
//!    [`IframeSandboxFlags`] (parsed in [`crate::origin`]). This module is
//!    the **one canonical home** for these predicates — every engine
//!    (VM / boa) and the shell delegate here instead of re-testing flag
//!    bits at call sites.
//!
//! The two planes share the word "sandbox" and nothing else: the process
//! sandbox is an OS isolation boundary, the content sandbox is a per-
//! browsing-context capability set.

use std::fmt;

use crate::origin::IframeSandboxFlags;

// ---------------------------------------------------------------------------
// Content-sandbox capability predicates (WHATWG HTML §7.1.5 Sandboxing)
// ---------------------------------------------------------------------------
//
// Contract (workspace-wide): `None` = unsandboxed document (everything
// allowed); `Some(IframeSandboxFlags::empty())` = maximum restriction (an
// empty `sandbox=""` attribute). The representation is the positive
// "allow-token" form — the total inversion of the spec's *restriction*
// flags over the delivered token subset — so each predicate below records
// which §7.1.5 browsing-context flag its bit *clears*.

/// Whether script execution is allowed (WHATWG HTML §7.1.5).
///
/// `ALLOW_SCRIPTS` (`allow-scripts`) clears the spec's *sandboxed scripts
/// browsing context flag* (`html#sandboxed-scripts-browsing-context-flag`).
/// This is the flag clause of the §8.1.3.4 scripting-disabled predicate —
/// see [`scripting_enabled`] for the full settings-level composition.
#[must_use]
pub fn scripts_allowed(flags: Option<IframeSandboxFlags>) -> bool {
    flags.is_none_or(|f| f.contains(IframeSandboxFlags::ALLOW_SCRIPTS))
}

/// Whether form submission is allowed (WHATWG HTML §7.1.5).
///
/// `ALLOW_FORMS` (`allow-forms`) clears the spec's *sandboxed forms
/// browsing context flag* (`html#sandboxed-forms-browsing-context-flag`).
#[must_use]
pub fn forms_allowed(flags: Option<IframeSandboxFlags>) -> bool {
    flags.is_none_or(|f| f.contains(IframeSandboxFlags::ALLOW_FORMS))
}

/// Whether popups (auxiliary navigation) are allowed (WHATWG HTML §7.1.5).
///
/// There is no "sandboxed popups flag" in the spec: `ALLOW_POPUPS`
/// (`allow-popups`) clears the *sandboxed auxiliary navigation browsing
/// context flag* (`html#sandboxed-auxiliary-navigation-browsing-context-flag`),
/// whose gate site is §7.3.1.7 *the rules for choosing a navigable* step 8
/// (a blocked popup never gets a new top-level traversable).
#[must_use]
pub fn popups_allowed(flags: Option<IframeSandboxFlags>) -> bool {
    flags.is_none_or(|f| f.contains(IframeSandboxFlags::ALLOW_POPUPS))
}

/// WHATWG HTML §8.1.3.4 "scripting is enabled" for an environment settings
/// object (`html#enabling-and-disabling-scripting`), settings-level
/// composition over the delivered surface. Scripting is enabled when ALL of:
///
/// 1. the UA supports scripting — constant `true` (elidex ships a script
///    engine unconditionally);
/// 2. the user has not disabled scripting for this settings object —
///    constant `false` today (no user toggle exists; this predicate is the
///    hook to thread one through when it does);
/// 3. the settings object's global is not a `Window`, **or** its associated
///    `Document`'s active sandboxing flag set does not have the *sandboxed
///    scripts browsing context flag* set — over the delivered surface this
///    is exactly [`scripts_allowed`] (the flag is the §7.1.5 inversion of
///    `ALLOW_SCRIPTS`);
/// 4. scripting is enabled for WebDriver BiDi — constant `true` (no
///    WebDriver BiDi surface exists).
///
/// The §8.1.3.4 *platform-object* clauses (browsing-context-null Node /
/// Window targets) are per-object facts a pure flag predicate cannot see;
/// they compose at the caller (the VM's event-handler processing step-1
/// gate).
#[must_use]
pub fn scripting_enabled(flags: Option<IframeSandboxFlags>) -> bool {
    // (1) UA-supports ∧ (2) ¬user-disabled ∧ (4) WebDriver-BiDi are the
    // documented constants above; (3) is the live clause.
    scripts_allowed(flags)
}

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

    // ── content-sandbox capability predicates (§7.1.5 / §8.1.3.4) ──────────

    #[test]
    fn unsandboxed_allows_everything() {
        assert!(scripts_allowed(None));
        assert!(forms_allowed(None));
        assert!(popups_allowed(None));
        assert!(scripting_enabled(None));
    }

    #[test]
    fn empty_flags_is_maximum_restriction() {
        let flags = Some(IframeSandboxFlags::empty());
        assert!(!scripts_allowed(flags));
        assert!(!forms_allowed(flags));
        assert!(!popups_allowed(flags));
        assert!(!scripting_enabled(flags));
    }

    #[test]
    fn each_bit_grants_only_its_own_capability() {
        let scripts = Some(IframeSandboxFlags::ALLOW_SCRIPTS);
        assert!(scripts_allowed(scripts));
        assert!(scripting_enabled(scripts));
        assert!(!forms_allowed(scripts));
        assert!(!popups_allowed(scripts));

        let forms = Some(IframeSandboxFlags::ALLOW_FORMS);
        assert!(forms_allowed(forms));
        assert!(!scripts_allowed(forms));
        assert!(!popups_allowed(forms));

        let popups = Some(IframeSandboxFlags::ALLOW_POPUPS);
        assert!(popups_allowed(popups));
        assert!(!scripts_allowed(popups));
        assert!(!forms_allowed(popups));
    }

    #[test]
    fn unrelated_bits_do_not_leak_capabilities() {
        // `allow-same-origin` affects the origin, not any predicate here.
        let same_origin = Some(IframeSandboxFlags::ALLOW_SAME_ORIGIN);
        assert!(!scripts_allowed(same_origin));
        assert!(!forms_allowed(same_origin));
        assert!(!popups_allowed(same_origin));
        assert!(!scripting_enabled(same_origin));
    }

    #[test]
    fn scripting_enabled_tracks_the_scripts_flag_clause() {
        // §8.1.3.4 conditions (1)/(2)/(4) are constants today, so the
        // settings-level predicate coincides with the flag clause — pinned
        // so a future user-disable hook must revisit this equivalence.
        for flags in [
            None,
            Some(IframeSandboxFlags::empty()),
            Some(IframeSandboxFlags::ALLOW_SCRIPTS),
            Some(IframeSandboxFlags::all()),
        ] {
            assert_eq!(scripting_enabled(flags), scripts_allowed(flags));
        }
    }

    // ── process-sandbox policy types ────────────────────────────────────────

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
