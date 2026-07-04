//! Content sandboxing (WHATWG HTML §7.1.5) — the `<iframe sandbox>` flag set
//! and its laws in one module: [`IframeSandboxFlags`] (positive allow-token
//! representation; `None` = unsandboxed document, `Some(empty)` = maximum
//! restriction, i.e. an empty `sandbox=""` attribute), the token parser
//! [`parse_sandbox_attribute`], and the capability predicates decided over
//! them: [`scripts_allowed`] / [`forms_allowed`] / [`popups_allowed`] /
//! [`modals_allowed`] / [`top_navigation_allowed`] (the spec's 2-flag
//! top-navigation decision, taking the caller's transient-activation fact
//! as a parameter) / [`scripting_enabled`]. This module is the canonical
//! predicate home: every capability check delegates here rather than
//! open-coding a `contains` test — the VM `HostData` accessors, the boa
//! `iframe_bridge.rs` / `globals/window/mod.rs` sites, and the shell
//! `content/event_handlers.rs` link-target top-navigation gate all route
//! through these functions (S5-4c closed the last open-coded sites).
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
        /// Allow navigation of the top-level browsing context, but only
        /// when the source has transient user activation
        /// (`allow-top-navigation-by-user-activation`,
        /// `html#attr-iframe-sandbox-allow-top-navigation-by-user-activation`).
        /// The token maps 1:1 onto this bit at parse time; the §7.1.5
        /// "`allow-top-navigation` implies it" conformance note is encoded
        /// in [`top_navigation_allowed`]'s disjunction, not here.
        const ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION = 1 << 6;
    }
}

/// Parse the `sandbox` attribute value into [`IframeSandboxFlags`].
///
/// The attribute value is "an unordered set of unique space-separated
/// tokens that are **ASCII case-insensitive**" (WHATWG HTML §4.8.5,
/// `html#attr-iframe-sandbox`), so each `allow-*` token is matched ASCII
/// case-insensitively. An empty string returns empty flags (all
/// restrictions enabled). Unrecognized tokens are silently ignored per
/// spec.
#[must_use]
pub fn parse_sandbox_attribute(value: &str) -> IframeSandboxFlags {
    let mut flags = IframeSandboxFlags::empty();
    for token in value.split_ascii_whitespace() {
        flags |= match token {
            t if t.eq_ignore_ascii_case("allow-scripts") => IframeSandboxFlags::ALLOW_SCRIPTS,
            t if t.eq_ignore_ascii_case("allow-same-origin") => {
                IframeSandboxFlags::ALLOW_SAME_ORIGIN
            }
            t if t.eq_ignore_ascii_case("allow-forms") => IframeSandboxFlags::ALLOW_FORMS,
            t if t.eq_ignore_ascii_case("allow-popups") => IframeSandboxFlags::ALLOW_POPUPS,
            t if t.eq_ignore_ascii_case("allow-top-navigation") => {
                IframeSandboxFlags::ALLOW_TOP_NAVIGATION
            }
            // 1:1 token→bit mapping: the §7.1.5 both-tokens rule
            // (`allow-top-navigation` wins / implies this one; using both is
            // a document conformance error) lives in the
            // `top_navigation_allowed` predicate's disjunction, so the parse
            // stays a pure token table.
            t if t.eq_ignore_ascii_case("allow-top-navigation-by-user-activation") => {
                IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION
            }
            t if t.eq_ignore_ascii_case("allow-modals") => IframeSandboxFlags::ALLOW_MODALS,
            // Unrecognized tokens silently ignored per spec.
            _ => IframeSandboxFlags::empty(),
        };
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

/// Whether simple dialogs / modals are allowed (WHATWG HTML §7.1.5).
///
/// `ALLOW_MODALS` (`allow-modals`) clears the spec's *sandboxed modals
/// flag* (`html#sandboxed-modals-flag`), whose prose prevents
/// `window.alert()` / `confirm()` / `print()` / `prompt()` and the
/// prompting in response to the `beforeunload` event. The gate site is
/// §8.9.1 *cannot show simple dialogs* (`html#cannot-show-simple-dialogs`)
/// step 1.
#[must_use]
pub fn modals_allowed(flags: Option<IframeSandboxFlags>) -> bool {
    flags.is_none_or(|f| f.contains(IframeSandboxFlags::ALLOW_MODALS))
}

/// Whether navigating the top-level traversable is allowed (WHATWG HTML
/// §7.4.2.4 *allowed by sandboxing to navigate*, `html#allowed-to-navigate`,
/// steps 3.2/3.3 — mirrored here in allow-form).
///
/// The spec keeps TWO top-navigation restriction flags (§7.1.5): the
/// *sandboxed top-level navigation without user activation browsing context
/// flag* (`html#sandboxed-top-level-navigation-without-user-activation-browsing-context-flag`,
/// cleared by `ALLOW_TOP_NAVIGATION`) blocks a source with **no** transient
/// activation (step 3.3), and the *…with user activation…* flag
/// (`html#sandboxed-top-level-navigation-with-user-activation-browsing-context-flag`,
/// cleared by `ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION` **or** by
/// `ALLOW_TOP_NAVIGATION` — the §7.1.5 both-tokens note: `allow-top-navigation`
/// wins / implies the by-user-activation grant) blocks a source that has it
/// (step 3.2). In the positive allow-token form that is exactly the
/// disjunction below. Callers pass their own transient-activation fact:
/// script-initiated paths pass `false` (the conservative constant — elidex
/// has no user-activation tracking yet), a direct user-gesture site passes
/// `true` statically.
#[must_use]
pub fn top_navigation_allowed(
    flags: Option<IframeSandboxFlags>,
    has_transient_activation: bool,
) -> bool {
    let allowed = |bit| flags.is_none_or(|f| f.contains(bit));
    allowed(IframeSandboxFlags::ALLOW_TOP_NAVIGATION)
        || (has_transient_activation
            && allowed(IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION))
}

/// WHATWG HTML §8.1.3.4 "scripting is enabled" for an environment settings
/// object (`html#enabling-and-disabling-scripting`): enabled iff scripts are
/// allowed ([`scripts_allowed`]). The other §8.1.3.4 conditions (UA support,
/// user disable, WebDriver BiDi) are constant: user disable is a UA settings
/// feature elidex does not implement (nothing to gate, no slot owed); if such
/// a feature ever lands, its condition composes here.
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
    fn sandbox_tokens_are_ascii_case_insensitive() {
        // §4.8.5 `attr-iframe-sandbox`: the value is "an unordered set of
        // unique space-separated tokens that are ASCII case-insensitive".
        for value in ["ALLOW-SCRIPTS", "Allow-Scripts", "aLLoW-sCrIpTs"] {
            let flags = parse_sandbox_attribute(value);
            assert_eq!(
                flags,
                IframeSandboxFlags::ALLOW_SCRIPTS,
                "value = {value:?}"
            );
        }
        // Mixed-case across multiple distinct tokens; casing must not leak
        // capabilities beyond the named token.
        let flags = parse_sandbox_attribute("Allow-Forms ALLOW-TOP-NAVIGATION allow-Modals");
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_MODALS));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN));
        assert!(!flags.contains(IframeSandboxFlags::ALLOW_POPUPS));
    }

    #[test]
    fn sandbox_all_flags() {
        let flags = parse_sandbox_attribute(
            "allow-scripts allow-same-origin allow-forms allow-popups allow-top-navigation \
             allow-modals allow-top-navigation-by-user-activation",
        );
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SCRIPTS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_FORMS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_POPUPS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_MODALS));
        assert!(flags.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION));
    }

    #[test]
    fn sandbox_top_navigation_by_user_activation_token() {
        // 1:1 token→bit mapping (case-insensitive like every token); the
        // `allow-top-navigation` implication lives in the predicate, so the
        // token sets ONLY its own bit …
        for value in [
            "allow-top-navigation-by-user-activation",
            "ALLOW-TOP-NAVIGATION-BY-USER-ACTIVATION",
            "Allow-Top-Navigation-By-User-Activation",
        ] {
            assert_eq!(
                parse_sandbox_attribute(value),
                IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION,
                "value = {value:?}"
            );
        }
        // … and `allow-top-navigation` does NOT set it at parse time (the
        // §7.1.5 both-tokens combination is a document conformance error
        // whose "allow-top-navigation wins" resolution is encoded in
        // `top_navigation_allowed`, not the token table).
        let top_only = parse_sandbox_attribute("allow-top-navigation");
        assert!(!top_only.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION));
        let both =
            parse_sandbox_attribute("allow-top-navigation allow-top-navigation-by-user-activation");
        assert!(both.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION));
        assert!(both.contains(IframeSandboxFlags::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION));
    }

    // ── capability predicates (§7.1.5 / §8.1.3.4) ──────────────────────────

    #[test]
    fn unsandboxed_allows_everything() {
        assert!(scripts_allowed(None));
        assert!(forms_allowed(None));
        assert!(popups_allowed(None));
        assert!(modals_allowed(None));
        assert!(scripting_enabled(None));
    }

    #[test]
    fn empty_flags_is_maximum_restriction() {
        let flags = Some(IframeSandboxFlags::empty());
        assert!(!scripts_allowed(flags));
        assert!(!forms_allowed(flags));
        assert!(!popups_allowed(flags));
        assert!(!modals_allowed(flags));
        assert!(!scripting_enabled(flags));
    }

    #[test]
    fn modals_allowed_tracks_only_the_modals_bit() {
        assert!(modals_allowed(Some(IframeSandboxFlags::ALLOW_MODALS)));
        // Sibling grants do not leak into the modals capability.
        assert!(!modals_allowed(Some(IframeSandboxFlags::ALLOW_SCRIPTS)));
        assert!(!modals_allowed(Some(IframeSandboxFlags::ALLOW_POPUPS)));
        // Nor does the modals grant leak outward.
        let modals = Some(IframeSandboxFlags::ALLOW_MODALS);
        assert!(!scripts_allowed(modals));
        assert!(!popups_allowed(modals));
    }

    #[test]
    fn top_navigation_allowed_full_truth_table() {
        // §7.4.2.4 steps 3.2/3.3 over the 2-flag pair (§4.3.3 disjunction):
        // flags ∈ {None, empty, TOP_NAV, BY_UA, both} × activation ∈ {f, t}.
        use IframeSandboxFlags as F;
        let by_ua = F::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION;
        let cases: [(Option<F>, bool, bool); 10] = [
            // Unsandboxed: allowed regardless of activation.
            (None, false, true),
            (None, true, true),
            // Maximum restriction: both restriction flags set → never.
            (Some(F::empty()), false, false),
            (Some(F::empty()), true, false),
            // `allow-top-navigation` clears BOTH restriction flags (the
            // §7.1.5 implication): allowed regardless of activation.
            (Some(F::ALLOW_TOP_NAVIGATION), false, true),
            (Some(F::ALLOW_TOP_NAVIGATION), true, true),
            // `allow-top-navigation-by-user-activation` alone clears only
            // the *with-user-activation* flag: activation-gated (step 3.3
            // still blocks the no-activation source).
            (Some(by_ua), false, false),
            (Some(by_ua), true, true),
            // Both tokens: `allow-top-navigation` wins — same as TOP_NAV.
            (Some(F::ALLOW_TOP_NAVIGATION | by_ua), false, true),
            (Some(F::ALLOW_TOP_NAVIGATION | by_ua), true, true),
        ];
        for (flags, activation, expected) in cases {
            assert_eq!(
                top_navigation_allowed(flags, activation),
                expected,
                "flags = {flags:?}, activation = {activation}"
            );
        }
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
