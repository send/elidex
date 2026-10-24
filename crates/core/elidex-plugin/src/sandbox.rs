//! Content sandboxing (WHATWG HTML §7.1.5) — the sandboxing flag set and its
//! laws in one module: [`SandboxingFlagSet`] (the spec's *restriction* form:
//! a set bit is a restriction that applies; the empty set is an unsandboxed
//! document), the directive parser [`parse_sandboxing_directive`], and the
//! capability predicates decided over the set: [`scripts_allowed`] /
//! [`forms_allowed`] / [`popups_allowed`] / [`modals_allowed`] /
//! [`same_origin_allowed`] / [`top_navigation_allowed`] (the spec's 2-flag
//! top-navigation decision, taking the caller's transient-activation fact as
//! a parameter) / [`scripting_enabled`]. This module is the canonical
//! predicate home: every capability check delegates here rather than
//! open-coding a flag test — the flag constants are private to this module,
//! so no other code can name one.
//!
//! Distinct from the OS *process* sandbox ([`crate::process_sandbox`]),
//! which shares the word "sandbox" and nothing else.

bitflags::bitflags! {
    /// The represented sandboxing flags (module-private; WHATWG HTML §7.1.5
    /// *sandboxing flag set*, `html#sandboxing-flag-set`). A set bit is a
    /// restriction that applies.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    struct Flags: u32 {
        /// `html#sandboxed-navigation-browsing-context-flag`.
        const SANDBOXED_NAVIGATION = 1 << 0;
        /// `html#sandboxed-auxiliary-navigation-browsing-context-flag`.
        const SANDBOXED_AUXILIARY_NAVIGATION = 1 << 1;
        /// `html#sandboxed-top-level-navigation-without-user-activation-browsing-context-flag`.
        const SANDBOXED_TOP_LEVEL_NAVIGATION_WITHOUT_USER_ACTIVATION = 1 << 2;
        /// `html#sandboxed-top-level-navigation-with-user-activation-browsing-context-flag`.
        const SANDBOXED_TOP_LEVEL_NAVIGATION_WITH_USER_ACTIVATION = 1 << 3;
        /// `html#sandboxed-origin-browsing-context-flag`.
        const SANDBOXED_ORIGIN = 1 << 4;
        /// `html#sandboxed-forms-browsing-context-flag`.
        const SANDBOXED_FORMS = 1 << 5;
        /// `html#sandboxed-scripts-browsing-context-flag`.
        const SANDBOXED_SCRIPTS = 1 << 6;
        /// `html#sandbox-propagates-to-auxiliary-browsing-contexts-flag`.
        const SANDBOX_PROPAGATES_TO_AUXILIARY_BROWSING_CONTEXTS = 1 << 7;
        /// `html#sandboxed-modals-flag`.
        const SANDBOXED_MODALS = 1 << 8;
    }
}

/// A sandboxing flag set (WHATWG HTML §7.1.5 *sandboxing flag set*,
/// `html#sandboxing-flag-set`) in the spec's restriction form.
///
/// Nine of the spec's sixteen flags are represented; the other seven
/// (pointer lock, automatic features, `document.domain`, orientation lock,
/// presentation, downloads, custom protocols navigation) have no reader yet
/// and belong to `#11-sandbox-flag-set-sources`.
///
/// The constructors are the whole public surface for building a value:
/// [`Self::unsandboxed`], [`Self::all_represented`],
/// [`parse_sandboxing_directive`] and [`Self::union_restrictions`]. There is
/// deliberately no `empty()` / `all()`, flag constant, `Default`, `Option`
/// form or set operator: any of those would compile against the previous
/// allow-form representation with the opposite meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SandboxingFlagSet(Flags);

impl SandboxingFlagSet {
    /// The empty sandboxing flag set — no restriction applies (HTML §7.1.5:
    /// "When the Document is created, its active sandboxing flag set must be
    /// empty"). The interim value of every store for a document that is not
    /// a sandboxed iframe.
    #[must_use]
    pub const fn unsandboxed() -> Self {
        Self(Flags::empty())
    }

    /// Every flag this representation holds, set — the result of parsing an
    /// empty directive (HTML §7.1.5 *parse a sandboxing directive* step 3
    /// over an empty token list). The seven unrepresented flags are absent
    /// from it.
    #[must_use]
    pub const fn all_represented() -> Self {
        Self(Flags::all())
    }

    /// The union of two restriction sets: restricted at least as much as
    /// either operand (elidex's own J7; no spec algorithm in this module
    /// takes a union). Note this is the reverse of what a bitwise `|` meant
    /// on allow-form bits.
    #[must_use]
    pub const fn union_restrictions(self, other: Self) -> Self {
        Self(self.0.union(other.0))
    }
}

/// Parse a sandboxing directive into a [`SandboxingFlagSet`] (WHATWG HTML
/// §7.1.5 *parse a sandboxing directive*, `html#parse-a-sandboxing-directive`,
/// steps 1–3).
///
/// The set starts empty and each represented flag is **added** unless its
/// clearing token is present, so an unrecognised or mis-cased token clears
/// nothing. The with-user-activation top-level navigation flag is cleared by
/// either `allow-top-navigation-by-user-activation` or `allow-top-navigation`;
/// the navigation flag is always added. Tokens are matched ASCII
/// case-insensitively: the `sandbox` attribute's value is "an unordered set
/// of unique space-separated tokens that are **ASCII case-insensitive**"
/// (WHATWG HTML §4.8.5, `html#attr-iframe-sandbox`). An empty directive
/// yields [`SandboxingFlagSet::all_represented`].
#[must_use]
pub fn parse_sandboxing_directive(input: &str) -> SandboxingFlagSet {
    let has = |name: &str| {
        input
            .split_ascii_whitespace()
            .any(|t| t.eq_ignore_ascii_case(name))
    };
    let allow_top = has("allow-top-navigation");
    let mut flags = Flags::SANDBOXED_NAVIGATION;
    if !has("allow-popups") {
        flags |= Flags::SANDBOXED_AUXILIARY_NAVIGATION;
    }
    if !allow_top {
        flags |= Flags::SANDBOXED_TOP_LEVEL_NAVIGATION_WITHOUT_USER_ACTIVATION;
    }
    if !(allow_top || has("allow-top-navigation-by-user-activation")) {
        flags |= Flags::SANDBOXED_TOP_LEVEL_NAVIGATION_WITH_USER_ACTIVATION;
    }
    if !has("allow-same-origin") {
        flags |= Flags::SANDBOXED_ORIGIN;
    }
    if !has("allow-forms") {
        flags |= Flags::SANDBOXED_FORMS;
    }
    if !has("allow-scripts") {
        flags |= Flags::SANDBOXED_SCRIPTS;
    }
    if !has("allow-popups-to-escape-sandbox") {
        flags |= Flags::SANDBOX_PROPAGATES_TO_AUXILIARY_BROWSING_CONTEXTS;
    }
    if !has("allow-modals") {
        flags |= Flags::SANDBOXED_MODALS;
    }
    SandboxingFlagSet(flags)
}

// ---------------------------------------------------------------------------
// Content-sandbox capability predicates (WHATWG HTML §7.1.5 Sandboxing)
// ---------------------------------------------------------------------------
//
// Contract (workspace-wide): [`SandboxingFlagSet::unsandboxed`] = everything
// allowed; a set flag is a restriction that applies. Each predicate below
// records which §7.1.5 flag it tests.

/// Whether script execution is allowed (WHATWG HTML §7.1.5): the *sandboxed
/// scripts browsing context flag* (`html#sandboxed-scripts-browsing-context-flag`)
/// is not set. This is the flag clause of the §8.1.3.4 scripting-disabled
/// predicate — see [`scripting_enabled`] for the settings-level composition.
#[must_use]
pub fn scripts_allowed(flags: SandboxingFlagSet) -> bool {
    !flags.0.contains(Flags::SANDBOXED_SCRIPTS)
}

/// Whether form submission is allowed (WHATWG HTML §7.1.5; §4.10.22.3 step 4):
/// the *sandboxed forms browsing context flag*
/// (`html#sandboxed-forms-browsing-context-flag`) is not set.
#[must_use]
pub fn forms_allowed(flags: SandboxingFlagSet) -> bool {
    !flags.0.contains(Flags::SANDBOXED_FORMS)
}

/// Whether popups (auxiliary navigation) are allowed (WHATWG HTML §7.1.5):
/// the *sandboxed auxiliary navigation browsing context flag*
/// (`html#sandboxed-auxiliary-navigation-browsing-context-flag`) is not set.
/// Its gate site is §7.3.1.7 *the rules for choosing a navigable* step 8 (a
/// blocked popup never gets a new top-level traversable).
#[must_use]
pub fn popups_allowed(flags: SandboxingFlagSet) -> bool {
    !flags.0.contains(Flags::SANDBOXED_AUXILIARY_NAVIGATION)
}

/// Whether simple dialogs / modals are allowed (WHATWG HTML §7.1.5): the
/// *sandboxed modals flag* (`html#sandboxed-modals-flag`) is not set. Its
/// prose prevents `window.alert()` / `confirm()` / `print()` / `prompt()` and
/// the prompting in response to the `beforeunload` event. The gate site is
/// §8.9.1 *cannot show simple dialogs* (`html#cannot-show-simple-dialogs`)
/// step 1.
#[must_use]
pub fn modals_allowed(flags: SandboxingFlagSet) -> bool {
    !flags.0.contains(Flags::SANDBOXED_MODALS)
}

/// Whether the document keeps its own origin (WHATWG HTML §7.1.5; §7.3.2.1
/// *determine the origin* step 1, `html#determining-the-origin`): the
/// *sandboxed origin browsing context flag*
/// (`html#sandboxed-origin-browsing-context-flag`) is not set.
#[must_use]
pub fn same_origin_allowed(flags: SandboxingFlagSet) -> bool {
    !flags.0.contains(Flags::SANDBOXED_ORIGIN)
}

/// Whether navigating the top-level traversable is allowed (WHATWG HTML
/// §7.4.2.4 *allowed by sandboxing to navigate*, `html#allowed-to-navigate`,
/// steps 3.2/3.3).
///
/// The spec keeps TWO top-navigation restriction flags (§7.1.5): the
/// *sandboxed top-level navigation without user activation browsing context
/// flag* blocks a source with **no** transient activation (step 3.3), and the
/// *…with user activation…* flag blocks a source that has it (step 3.2). The
/// `allow-top-navigation` implication of the by-user-activation grant is in
/// [`parse_sandboxing_directive`], not here. Callers pass their own
/// transient-activation fact: script-initiated paths pass `false` (the
/// conservative constant — elidex has no user-activation tracking yet), a
/// direct user-gesture site passes `true` statically.
#[must_use]
pub fn top_navigation_allowed(flags: SandboxingFlagSet, has_transient_activation: bool) -> bool {
    if has_transient_activation {
        !flags
            .0
            .contains(Flags::SANDBOXED_TOP_LEVEL_NAVIGATION_WITH_USER_ACTIVATION)
    } else {
        !flags
            .0
            .contains(Flags::SANDBOXED_TOP_LEVEL_NAVIGATION_WITHOUT_USER_ACTIVATION)
    }
}

/// WHATWG HTML §8.1.3.4 "scripting is enabled" for an environment settings
/// object (`html#enabling-and-disabling-scripting`): enabled iff scripts are
/// allowed ([`scripts_allowed`]). The other §8.1.3.4 conditions (UA support,
/// user disable, WebDriver BiDi) are constant: user disable is a UA settings
/// feature elidex does not implement (nothing to gate, no slot owed); if such
/// a feature ever lands, its condition composes here.
#[must_use]
pub fn scripting_enabled(flags: SandboxingFlagSet) -> bool {
    scripts_allowed(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD_TOKENS: [&str; 7] = [
        "allow-scripts",
        "allow-same-origin",
        "allow-forms",
        "allow-popups",
        "allow-top-navigation",
        "allow-modals",
        "allow-top-navigation-by-user-activation",
    ];

    fn subset(tokens: &[&'static str], mask: u32) -> Vec<&'static str> {
        tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, t)| *t)
            .collect()
    }

    // ── C10: parse truth table (§7.1.5 step 3) ──────────────────────────────

    #[test]
    fn parse_matches_spec_arms_over_every_token_subset() {
        // (flag, clearing tokens) transcribed from step 3.
        let table: [(Flags, &[&str]); 9] = [
            (Flags::SANDBOXED_NAVIGATION, &[]),
            (Flags::SANDBOXED_AUXILIARY_NAVIGATION, &["allow-popups"]),
            (
                Flags::SANDBOXED_TOP_LEVEL_NAVIGATION_WITHOUT_USER_ACTIVATION,
                &["allow-top-navigation"],
            ),
            (
                Flags::SANDBOXED_TOP_LEVEL_NAVIGATION_WITH_USER_ACTIVATION,
                &[
                    "allow-top-navigation-by-user-activation",
                    "allow-top-navigation",
                ],
            ),
            (Flags::SANDBOXED_ORIGIN, &["allow-same-origin"]),
            (Flags::SANDBOXED_FORMS, &["allow-forms"]),
            (Flags::SANDBOXED_SCRIPTS, &["allow-scripts"]),
            (
                Flags::SANDBOX_PROPAGATES_TO_AUXILIARY_BROWSING_CONTEXTS,
                &["allow-popups-to-escape-sandbox"],
            ),
            (Flags::SANDBOXED_MODALS, &["allow-modals"]),
        ];
        let tokens = [
            "allow-scripts",
            "allow-same-origin",
            "allow-forms",
            "allow-popups",
            "allow-top-navigation",
            "allow-top-navigation-by-user-activation",
            "allow-modals",
            "allow-popups-to-escape-sandbox",
        ];
        for mask in 0..256u32 {
            let present = subset(&tokens, mask);
            let mut expected = Flags::empty();
            for (flag, clearers) in &table {
                if !clearers.iter().any(|c| present.contains(c)) {
                    expected |= *flag;
                }
            }
            assert_eq!(
                parse_sandboxing_directive(&present.join(" ")),
                SandboxingFlagSet(expected),
                "tokens = {present:?}"
            );
        }
    }

    #[test]
    fn parse_empty_directive_is_all_represented() {
        assert_eq!(
            parse_sandboxing_directive(""),
            SandboxingFlagSet::all_represented()
        );
        assert_eq!(
            parse_sandboxing_directive(" \t\n \r\x0c "),
            SandboxingFlagSet::all_represented()
        );
    }

    #[test]
    fn parse_navigation_flag_is_always_set() {
        // Observed by equality: adding every clearing token still differs
        // from `unsandboxed()`.
        let everything = parse_sandboxing_directive(&format!(
            "{} allow-popups-to-escape-sandbox",
            HEAD_TOKENS.join(" ")
        ));
        assert_ne!(everything, SandboxingFlagSet::unsandboxed());
    }

    #[test]
    fn parse_allow_top_navigation_clears_both_top_level_flags() {
        let s = parse_sandboxing_directive("allow-top-navigation");
        assert!(top_navigation_allowed(s, false));
        assert!(top_navigation_allowed(s, true));
        let ua = parse_sandboxing_directive("allow-top-navigation-by-user-activation");
        assert!(!top_navigation_allowed(ua, false));
        assert!(top_navigation_allowed(ua, true));
    }

    #[test]
    fn parse_tokens_are_ascii_case_insensitive() {
        // §4.8.5 `attr-iframe-sandbox`: the value is "an unordered set of
        // unique space-separated tokens that are ASCII case-insensitive".
        let expected = parse_sandboxing_directive("allow-scripts");
        for value in ["ALLOW-SCRIPTS", "Allow-Scripts", "aLLoW-sCrIpTs"] {
            assert_eq!(parse_sandboxing_directive(value), expected, "{value:?}");
        }
        assert_eq!(
            parse_sandboxing_directive("Allow-Forms ALLOW-TOP-NAVIGATION allow-Modals"),
            parse_sandboxing_directive("allow-forms allow-top-navigation allow-modals")
        );
    }

    #[test]
    fn parse_non_ascii_lookalike_clears_nothing() {
        assert_eq!(
            parse_sandboxing_directive("ALLOW-SCR\u{130}PTS"),
            SandboxingFlagSet::all_represented()
        );
    }

    #[test]
    fn parse_unknown_and_unrepresented_tokens_clear_nothing() {
        for value in [
            "unknown-token",
            "allow-downloads",
            "allow-orientation-lock",
            "allow-pointer-lock",
            "allow-presentation",
            "allow-top-navigation-to-custom-protocols",
        ] {
            assert_eq!(
                parse_sandboxing_directive(value),
                SandboxingFlagSet::all_represented(),
                "{value:?}"
            );
        }
        assert_eq!(
            parse_sandboxing_directive("allow-scripts unknown-token allow-forms"),
            parse_sandboxing_directive("allow-scripts allow-forms")
        );
    }

    #[test]
    fn parse_accepts_every_ascii_whitespace_separator() {
        assert_eq!(
            parse_sandboxing_directive(
                "allow-scripts\tallow-forms\nallow-modals\x0callow-popups\rallow-same-origin"
            ),
            parse_sandboxing_directive(
                "allow-scripts allow-forms allow-modals allow-popups allow-same-origin"
            )
        );
    }

    // ── C10b: HEAD equivalence (K1) over token strings ──────────────────────

    #[test]
    fn predicates_match_head_token_presence_verdicts() {
        for mask in 0..128u32 {
            let present = subset(&HEAD_TOKENS, mask);
            let has = |t: &str| present.contains(&t);
            let set = parse_sandboxing_directive(&present.join(" "));
            assert_eq!(scripts_allowed(set), has("allow-scripts"), "{present:?}");
            assert_eq!(scripting_enabled(set), has("allow-scripts"), "{present:?}");
            assert_eq!(forms_allowed(set), has("allow-forms"), "{present:?}");
            assert_eq!(popups_allowed(set), has("allow-popups"), "{present:?}");
            assert_eq!(modals_allowed(set), has("allow-modals"), "{present:?}");
            assert_eq!(
                same_origin_allowed(set),
                has("allow-same-origin"),
                "{present:?}"
            );
            for activation in [false, true] {
                assert_eq!(
                    top_navigation_allowed(set, activation),
                    has("allow-top-navigation")
                        || (activation && has("allow-top-navigation-by-user-activation")),
                    "{present:?}, activation = {activation}"
                );
            }
        }
    }

    // ── C10c: restriction-form union (K2, J7) ───────────────────────────────

    #[test]
    fn union_restrictions_restricts_at_least_as_much_as_either() {
        let sets: Vec<SandboxingFlagSet> = (0..128u32)
            .map(|m| parse_sandboxing_directive(&subset(&HEAD_TOKENS, m).join(" ")))
            .collect();
        let singles: [fn(SandboxingFlagSet) -> bool; 6] = [
            scripts_allowed,
            forms_allowed,
            popups_allowed,
            modals_allowed,
            same_origin_allowed,
            scripting_enabled,
        ];
        for &a in &sets {
            for &b in &sets {
                let u = a.union_restrictions(b);
                for p in singles {
                    assert_eq!(p(u), p(a) && p(b));
                }
                for act in [false, true] {
                    if top_navigation_allowed(u, act) {
                        assert!(top_navigation_allowed(a, act));
                        assert!(top_navigation_allowed(b, act));
                    }
                }
            }
            assert_eq!(a.union_restrictions(SandboxingFlagSet::unsandboxed()), a);
            assert_eq!(
                a.union_restrictions(SandboxingFlagSet::all_represented()),
                SandboxingFlagSet::all_represented()
            );
        }
    }

    // ── capability predicates (§7.1.5 / §8.1.3.4) ──────────────────────────

    // C10f: `unsandboxed()` is permissive for every predicate.
    #[test]
    fn unsandboxed_allows_everything() {
        let none = SandboxingFlagSet::unsandboxed();
        assert!(scripts_allowed(none));
        assert!(forms_allowed(none));
        assert!(popups_allowed(none));
        assert!(modals_allowed(none));
        assert!(scripting_enabled(none));
        assert!(same_origin_allowed(none));
        assert!(top_navigation_allowed(none, false));
        assert!(top_navigation_allowed(none, true));
    }

    #[test]
    fn empty_directive_is_maximum_restriction() {
        let flags = SandboxingFlagSet::all_represented();
        assert!(!scripts_allowed(flags));
        assert!(!forms_allowed(flags));
        assert!(!popups_allowed(flags));
        assert!(!modals_allowed(flags));
        assert!(!scripting_enabled(flags));
        assert!(!same_origin_allowed(flags));
    }

    #[test]
    fn modals_allowed_tracks_only_the_modals_flag() {
        assert!(modals_allowed(parse_sandboxing_directive("allow-modals")));
        // Sibling grants do not leak into the modals capability.
        assert!(!modals_allowed(parse_sandboxing_directive("allow-scripts")));
        assert!(!modals_allowed(parse_sandboxing_directive("allow-popups")));
        // Nor does the modals grant leak outward.
        let modals = parse_sandboxing_directive("allow-modals");
        assert!(!scripts_allowed(modals));
        assert!(!popups_allowed(modals));
    }

    #[test]
    fn top_navigation_allowed_full_truth_table() {
        // §7.4.2.4 steps 3.2/3.3 over the 2-flag pair:
        // flags ∈ {unsandboxed, all, TOP_NAV, BY_UA, both} × activation ∈ {f, t}.
        let by_ua = "allow-top-navigation-by-user-activation";
        let cases: [(SandboxingFlagSet, bool, bool); 10] = [
            // Unsandboxed: allowed regardless of activation.
            (SandboxingFlagSet::unsandboxed(), false, true),
            (SandboxingFlagSet::unsandboxed(), true, true),
            // Maximum restriction: both restriction flags set → never.
            (SandboxingFlagSet::all_represented(), false, false),
            (SandboxingFlagSet::all_represented(), true, false),
            // `allow-top-navigation` clears BOTH restriction flags (the
            // §7.1.5 implication): allowed regardless of activation.
            (
                parse_sandboxing_directive("allow-top-navigation"),
                false,
                true,
            ),
            (
                parse_sandboxing_directive("allow-top-navigation"),
                true,
                true,
            ),
            // `allow-top-navigation-by-user-activation` alone clears only
            // the *with-user-activation* flag: activation-gated.
            (parse_sandboxing_directive(by_ua), false, false),
            (parse_sandboxing_directive(by_ua), true, true),
            // Both tokens: `allow-top-navigation` wins — same as TOP_NAV.
            (
                parse_sandboxing_directive(
                    "allow-top-navigation allow-top-navigation-by-user-activation",
                ),
                false,
                true,
            ),
            (
                parse_sandboxing_directive(
                    "allow-top-navigation allow-top-navigation-by-user-activation",
                ),
                true,
                true,
            ),
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
    fn each_token_grants_only_its_own_capability() {
        let scripts = parse_sandboxing_directive("allow-scripts");
        assert!(scripts_allowed(scripts));
        assert!(scripting_enabled(scripts));
        assert!(!forms_allowed(scripts));
        assert!(!popups_allowed(scripts));

        let forms = parse_sandboxing_directive("allow-forms");
        assert!(forms_allowed(forms));
        assert!(!scripts_allowed(forms));
        assert!(!popups_allowed(forms));

        let popups = parse_sandboxing_directive("allow-popups");
        assert!(popups_allowed(popups));
        assert!(!scripts_allowed(popups));
        assert!(!forms_allowed(popups));
    }

    #[test]
    fn same_origin_token_grants_only_the_origin() {
        let same_origin = parse_sandboxing_directive("allow-same-origin");
        assert!(same_origin_allowed(same_origin));
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
            SandboxingFlagSet::unsandboxed(),
            SandboxingFlagSet::all_represented(),
            parse_sandboxing_directive("allow-scripts"),
            parse_sandboxing_directive(&HEAD_TOKENS.join(" ")),
        ] {
            assert_eq!(scripting_enabled(flags), scripts_allowed(flags));
        }
    }
}
