//! Content sandboxing (WHATWG HTML §7.1.5) — the `<iframe sandbox>` flag set
//! and its laws in one module: [`IframeSandboxFlags`] (positive allow-token
//! representation; `None` = unsandboxed document, `Some(empty)` = maximum
//! restriction, i.e. an empty `sandbox=""` attribute), the token parser
//! [`parse_sandbox_attribute`], and the capability predicates decided over
//! them. Delivered predicates: [`scripts_allowed`] / [`forms_allowed`] /
//! [`popups_allowed`] / [`scripting_enabled`]; `modals_allowed` and
//! `top_navigation_allowed` land with their consumers in S5-4c.
//!
//! Distinct from the OS *process* sandbox ([`crate::process_sandbox`]),
//! which shares the word "sandbox" and nothing else.

bitflags::bitflags! {
    /// Sandbox flags for `<iframe sandbox>` attribute (WHATWG HTML §4.8.5).
    ///
    /// An empty `sandbox` attribute (no tokens) means all flags are cleared
    /// (maximum restrictions). Each `allow-*` token sets a corresponding flag.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub struct IframeSandboxFlags: u16 {
        /// Allow script execution in the sandboxed iframe.
        const ALLOW_SCRIPTS        = 1 << 0;
        /// Treat the iframe as same-origin with its parent (instead of opaque).
        const ALLOW_SAME_ORIGIN    = 1 << 1;
        /// Allow form submission.
        const ALLOW_FORMS          = 1 << 2;
        /// Allow `window.open()` and `target="_blank"` links.
        const ALLOW_POPUPS         = 1 << 3;
        /// Allow navigation of the top-level browsing context.
        const ALLOW_TOP_NAVIGATION = 1 << 4;
        /// Allow `alert()`, `confirm()`, and `prompt()` modals.
        const ALLOW_MODALS         = 1 << 5;
    }
}

/// Parse the `sandbox` attribute value into [`IframeSandboxFlags`].
///
/// An empty string or `None` returns empty flags (all restrictions enabled).
/// Unrecognized tokens are silently ignored per WHATWG HTML §4.8.5.
#[must_use]
pub fn parse_sandbox_attribute(value: &str) -> IframeSandboxFlags {
    let mut flags = IframeSandboxFlags::empty();
    for token in value.split_ascii_whitespace() {
        match token {
            "allow-scripts" => flags |= IframeSandboxFlags::ALLOW_SCRIPTS,
            "allow-same-origin" => flags |= IframeSandboxFlags::ALLOW_SAME_ORIGIN,
            "allow-forms" => flags |= IframeSandboxFlags::ALLOW_FORMS,
            "allow-popups" => flags |= IframeSandboxFlags::ALLOW_POPUPS,
            "allow-top-navigation" => flags |= IframeSandboxFlags::ALLOW_TOP_NAVIGATION,
            "allow-modals" => flags |= IframeSandboxFlags::ALLOW_MODALS,
            _ => {} // Unrecognized tokens silently ignored per spec.
        }
    }
    flags
}

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
/// object (`html#enabling-and-disabling-scripting`): enabled iff scripts are
/// allowed ([`scripts_allowed`]). The other §8.1.3.4 conditions (UA support,
/// user disable, WebDriver BiDi) are constant today — this is the named seam
/// for a future user-disable toggle.
#[must_use]
pub fn scripting_enabled(flags: Option<IframeSandboxFlags>) -> bool {
    scripts_allowed(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── token parser (§4.8.5) ───────────────────────────────────────────────

    #[test]
    fn sandbox_empty_string() {
        let flags = parse_sandbox_attribute("");
        assert!(flags.is_empty());
    }

    #[test]
    fn sandbox_single_token() {
        let flags = parse_sandbox_attribute("allow-scripts");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_FORMS));
    }

    #[test]
    fn sandbox_multiple_tokens() {
        let flags = parse_sandbox_attribute("allow-scripts allow-same-origin allow-forms");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_POPUPS));
    }

    #[test]
    fn sandbox_unrecognized_tokens_ignored() {
        let flags = parse_sandbox_attribute("allow-scripts unknown-token allow-forms");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
    }

    #[test]
    fn sandbox_all_flags() {
        let flags = parse_sandbox_attribute(
            "allow-scripts allow-same-origin allow-forms allow-popups allow-top-navigation allow-modals",
        );
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_POPUPS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_MODALS));
    }

    // ── capability predicates (§7.1.5 / §8.1.3.4) ──────────────────────────

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
}
