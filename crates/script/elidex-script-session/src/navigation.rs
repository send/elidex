//! Navigation back-channel intent types — the engine↔shell contract.
//!
//! A script engine bound to one document cannot navigate itself: navigation
//! replaces the whole pipeline (network + parse + render), which the shell
//! owns. So `location.*` / `history.*` globals do not navigate directly — they
//! record an **intent** (a [`NavigationRequest`] or a [`HistoryAction`]) that
//! the shell drains after the script turn and applies to its single
//! session-history source of truth (the shell's `NavigationController`).
//!
//! These types are the wire format of that channel, shared by every engine
//! (boa, the elidex-js VM) and the shell. They live in this engine-agnostic
//! seam crate — alongside [`ScriptEngine`](crate::ScriptEngine) /
//! [`DispatchEvent`](crate::DispatchEvent) — rather than in the shell's
//! `elidex-navigation` (which also carries the loader + the `NavigationController`
//! implementation), so a `crates/script/` engine never depends on a
//! `crates/shell/` crate just to produce the contract.
//!
//! `window.open` (WHATWG HTML §7.2.2.1) rides the same model with two more
//! intent types — [`OpenTabRequest`] (popup / `_blank`) and
//! [`NamedFrameNavigation`] (named target) — plus the pure
//! [`window_open_disposition`] function that owns the spec's target dispatch
//! and its sandbox gates (§7.3.1.7 / §7.4.2.4), so every engine routes one
//! decision function into the channels rather than open-coding the gate.

use elidex_plugin::sandbox;
use elidex_plugin::IframeSandboxFlags;

/// The navigation TYPE (WHATWG HTML §7.4.2.2 "Beginning navigation" entry
/// points). Orthogonal to the same-document URL classification: it distinguishes
/// the history-cursor effect + excludes reload from the fragment (no-rebuild)
/// path. Single-homes the nav-type that a `replace: bool` could not — `bool`
/// collapsed `location.reload()` and `location.replace()` (both were `true`),
/// so a fragment-URL reload could not be told apart from a same-page replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationType {
    /// `location.href = …` / `location.assign()` / `<a href>` — push a new entry.
    Push,
    /// `location.replace()` — replace the current entry (§7.4.4 historyHandling
    /// "replace").
    Replace,
    /// `location.reload()` — a distinct algorithm (HTML §7.4.3 Reloading),
    /// `isSameDocument=false` (never takes the fragment no-rebuild path).
    Reload,
}

/// A pending navigation request from `location.assign()` / `location.href = …`
/// / `location.replace()` / `location.reload()` (WHATWG HTML §7.4.2.2
/// "Beginning navigation"). The shell runs the navigate algorithm and commits
/// the new URL back into the engine after the load.
#[derive(Clone, Debug)]
pub struct NavigationRequest {
    /// The page-supplied target URL. The **VM** resolves it at enqueue against
    /// the document URL — encoding-parse **relative to the entry settings
    /// object** (WHATWG HTML §7.2.4 "The Location interface" — `href` setter
    /// step 2 / `assign` step 3 / `replace` step 2; base ≈ the document base URL,
    /// the VM's `current_url`) — so the VM stores an **absolute** URL (the same
    /// enqueue-time resolution as [`HistoryAction::PushState::url`]). The **boa**
    /// engine stores the **raw** page string, which the shell resolves +
    /// scheme-validates against the document URL at drain time. These agree
    /// except for a boa-only same-turn history+navigation edge (a `pushState`
    /// changing the document URL before the shell drains the raw nav — boa's
    /// `pushState` does not update `current_url`): that boa relative-nav base is a
    /// **deletion-bound divergence deferred to the S5-6 flip** (the VM is correct
    /// by construction; boa is not touched to fix it — §0 pre-decision (2)).
    pub url: String,
    /// The navigation type ([`NavigationType`]) the setter recorded — `Push`
    /// (`href=`/`assign`/`<a href>`), `Replace` (`replace()`), or `Reload`
    /// (`reload()`). Replaces the earlier `replace: bool`, which could not
    /// distinguish `reload()` from `replace()`. The shell drains map it to a
    /// history-cursor effect (reload → no cursor advance; thread-mode still
    /// collapses replace → push for the cursor op, deferred §10-D6).
    pub nav_type: NavigationType,
}

/// A pending history action from the `History` interface (WHATWG HTML §7.2.5).
///
/// `Back` / `Forward` / `Go` are session-history *traversals* (§7.4.6 "Applying
/// the history step" — an async document load the shell owns); `PushState` /
/// `ReplaceState` are the §7.2.5 "shared history push/replace state steps" the
/// shell persists into its `NavigationController` (the engine has already run
/// the synchronous URL-and-history-update half, §7.4.4).
#[derive(Clone, Debug)]
pub enum HistoryAction {
    /// `history.back()`
    Back,
    /// `history.forward()`
    Forward,
    /// `history.go(delta)`
    Go(i32),
    /// `history.pushState(state, title, url?)`
    PushState {
        /// Optional URL to push (already resolved against the document URL).
        url: Option<String>,
        /// Title (ignored per §7.2.5 — `unused` — but accepted for API compat).
        title: String,
    },
    /// `history.replaceState(state, title, url?)`
    ReplaceState {
        /// Optional URL to replace the current entry with.
        url: Option<String>,
        /// Title (ignored per §7.2.5 — `unused` — but accepted for API compat).
        title: String,
    },
}

/// A minimal serialized `history.state` placeholder. 5b (synchronous fragment
/// navigation) never carries a value — a fragment nav's popstate state is always
/// `null` (WHATWG HTML §7.4.2.3.3 *navigate to a fragment* step 11.1 "Set
/// history's state to null"). S5-5c (traversal) fills in the real
/// `StructuredSerializeForStorage` form the engine `StructuredDeserialize`s.
pub type SerializedState = Vec<u8>;

/// The popstate / hashchange to fire for a same-document history-step
/// application (WHATWG HTML §7.4.6.2 "update document for history step
/// application" step 6.4). The **shell** decides which fire from its
/// session-history entry model (the engine-independent decision); the **engine**
/// reconstructs `history.state` and fires at the `Window`.
///
/// popstate fires **synchronously** (§7.4.6.2 step 6.4.3 "fire an event");
/// hashchange is **enqueued** as a task (step 6.4.5 "queue a global task on the
/// DOM manipulation task source"), so popstate is observed strictly before
/// hashchange.
#[derive(Clone, Debug, Default)]
pub struct HistoryStepEvents {
    /// `Some(None)` = fire popstate with `state = null` (fragment nav, 5b);
    /// `Some(Some(bytes))` = fire popstate with `StructuredDeserialize(restored)`
    /// (5c traversal); `None` = do not fire popstate.
    pub popstate_state: Option<Option<SerializedState>>,
    /// `Some((oldURL, newURL))` iff the fragment differs (§7.4.6.2 step 6.4.5);
    /// `None` = do not fire hashchange.
    pub hashchange: Option<(String, String)>,
}

/// A **gate-passed** `window.open` popup / `_blank` request (WHATWG HTML
/// §7.2.2.1 window open steps → §7.3.1.7 step 8's "create a new top-level
/// traversable" case). The shell drains these each pump and opens a new tab
/// per request.
///
/// The enqueue itself is popup-gated (the [`WindowOpenDisposition::OpenTab`]
/// arm exists only when `popups_allowed` holds): a sandbox-blocked popup
/// never enters the queue, so a shell drain cannot leak what was never
/// queued (security by structure — the gate is the enqueue chokepoint, not
/// the drain).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenTabRequest {
    /// The resolved absolute target URL (parsed at the call boundary; an
    /// empty `url` argument resolves to `about:blank` per §7.2.2.1).
    pub url: String,
}

/// A named-target `window.open` navigation (WHATWG HTML §7.3.1.7 *the rules
/// for choosing a navigable* — target is a name, not a `_`-keyword). The
/// shell resolves the name against its frame tree at drain time: on HIT it
/// navigates the found navigable, on MISS it may promote the request to a
/// new tab — but **only** when [`Self::aux_nav_allowed`] permits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedFrameNavigation {
    /// The target name as the script gave it (only `_`-keyword DETECTION is
    /// ASCII case-insensitive per §7.3.1.7 — the name itself is preserved
    /// verbatim for the shell's own name-matching rules).
    pub name: String,
    /// The resolved absolute target URL, or `None` when `window.open` was
    /// called with an **empty** url (WHATWG HTML §7.2.2.1 *window open steps*
    /// step 3-4: the urlRecord stays null). The distinction is load-bearing
    /// and cannot be pre-resolved to about:blank at the VM, because whether
    /// the named target hits an existing navigable (step 16.1 — navigate only
    /// when urlRecord is non-null; empty = no-op) or creates a new one
    /// (step 15.3 — a new navigable's null urlRecord defaults to about:blank)
    /// is decided by the shell's frame-tree lookup at drain time.
    pub url: Option<String>,
    /// The §7.3.1.7 step-3 sandboxing-flag-set **snapshot** taken at call
    /// time: whether the *sandboxed auxiliary navigation* flag permitted a
    /// new top-level traversable when `window.open` ran. The shell's
    /// named-target-MISS → new-tab promotion must consult THIS verdict,
    /// never re-read live flags — a flag change between call and drain must
    /// not re-evaluate the gate (the aux-nav flag gates only the
    /// create-a-new-traversable case of step 8, which is why a named
    /// request is enqueued even when `false`: the HIT path is not
    /// popup-gated).
    pub aux_nav_allowed: bool,
}

/// One `window.open` effect that creates or targets a navigable, carried on a
/// **single ordered queue** (WHATWG HTML §7.2.2.1 — each `window.open` call is
/// a distinct step, and the resulting tab-creation / frame-navigation must
/// happen in call order). Popup (`_blank`) and named-target opens both end up
/// as user-visible browser actions (a new tab, or a named-MISS promotion), so
/// they share ONE queue: two independent queues would let a later `_blank`
/// surface before an earlier named MISS, reversing the order the page issued
/// them. (`_self`/`_parent`/`_top` are own-context navigations on the separate
/// [`NavigationRequest`] channel — a different effect, not tab creation.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowOpenIntent {
    /// A `_blank`/popup open (gate-passed — a sandbox-blocked popup never
    /// reaches the queue): open a new tab.
    Popup(OpenTabRequest),
    /// A named-target open: the shell resolves the name against its frame tree
    /// at drain time (HIT → navigate the iframe; MISS → gated new-tab
    /// promotion via [`NamedFrameNavigation::aux_nav_allowed`]).
    NamedFrame(NamedFrameNavigation),
}

/// The outcome of `window.open`'s target dispatch — which back-channel (if
/// any) a request routes to. Produced by [`window_open_disposition`]; each
/// variant maps to exactly one enqueue site in the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowOpenDisposition {
    /// A sandbox gate failed (`_blank` without `allow-popups`, or
    /// `_parent`/`_top` without a top-navigation grant): enqueue nothing and
    /// return `null` silently (the spec's "may report to a developer
    /// console").
    Blocked,
    /// `_self` — navigate the source's own navigable via a
    /// [`NavigationRequest`]. Never gated: §7.3.1.7 resolves `_self` to
    /// `currentNavigable` before any sandboxing-flag check.
    SelfNavigate,
    /// `_parent` / `_top` with the §7.4.2.4 top-navigation gate passed —
    /// a [`NavigationRequest`] (the single-navigable model routes it to the
    /// own context until S5-8's real navigable tree).
    TopNavigate,
    /// A named target — always enqueued as a [`NamedFrameNavigation`]
    /// carrying the snapshotted auxiliary-navigation verdict (hit/miss
    /// resolution is shell-side; see [`NamedFrameNavigation::aux_nav_allowed`]).
    Named {
        /// The call-time `popups_allowed` snapshot for the MISS-promotion
        /// gate.
        aux_nav_allowed: bool,
    },
    /// `_blank` (or an empty target) with `allow-popups` — an
    /// [`OpenTabRequest`].
    OpenTab,
}

/// The `window.open` target dispatch (WHATWG HTML §7.3.1.7 *the rules for
/// choosing a navigable*), composed with its sandbox gates over the
/// call-time `(target, flags, activation)` facts — a pure engine-independent
/// decision function; the engine natives only marshal arguments in and
/// enqueue per the returned disposition.
///
/// - Keyword matching is ASCII case-insensitive per §7.3.1.7 ("an ASCII
///   case-insensitive match for `_blank`" etc.); a non-keyword target —
///   including a `_`-prefixed name matching no keyword — is a name.
/// - An **empty** target is mapped to `_blank` HERE (§7.2.2.1 window open
///   steps step 5: "If target is the empty string, then set target to
///   `_blank`"), so this function owns the complete outcome set over any
///   coerced target string.
/// - `_self` resolves before any flag check (spec-shaped: §7.3.1.7 returns
///   `currentNavigable` for `_self` without consulting the sandboxing flag
///   set).
/// - `_parent`/`_top` gate on [`sandbox::top_navigation_allowed`]
///   (§7.4.2.4 *allowed by sandboxing to navigate* steps 3.2/3.3, the
///   2-flag pair); callers pass their own transient-activation fact
///   (script-initiated `window.open` passes `false` — the conservative
///   constant).
/// - `_blank` gates on [`sandbox::popups_allowed`] (§7.3.1.7 step 8's
///   *sandboxed auxiliary navigation* case).
/// - A named target is **never blocked at enqueue** — the aux-nav flag
///   gates only the create-a-new-traversable case of step 8, so the
///   verdict is snapshotted onto [`WindowOpenDisposition::Named`] for the
///   shell's MISS branch instead.
#[must_use]
pub fn window_open_disposition(
    target: &str,
    flags: Option<IframeSandboxFlags>,
    has_transient_activation: bool,
) -> WindowOpenDisposition {
    if target.eq_ignore_ascii_case("_self") {
        return WindowOpenDisposition::SelfNavigate;
    }
    if target.eq_ignore_ascii_case("_parent") || target.eq_ignore_ascii_case("_top") {
        return if sandbox::top_navigation_allowed(flags, has_transient_activation) {
            WindowOpenDisposition::TopNavigate
        } else {
            WindowOpenDisposition::Blocked
        };
    }
    if target.is_empty() || target.eq_ignore_ascii_case("_blank") {
        return if sandbox::popups_allowed(flags) {
            WindowOpenDisposition::OpenTab
        } else {
            WindowOpenDisposition::Blocked
        };
    }
    WindowOpenDisposition::Named {
        aux_nav_allowed: sandbox::popups_allowed(flags),
    }
}

#[cfg(test)]
mod tests {
    use elidex_plugin::IframeSandboxFlags as F;

    use super::WindowOpenDisposition as D;
    use super::*;

    const UNSANDBOXED: Option<F> = None;
    const MAX_RESTRICTION: Option<F> = Some(F::empty());

    #[test]
    fn self_target_is_never_gated() {
        // §7.3.1.7 resolves `_self` before any flag check — even maximum
        // restriction with no activation navigates the own context.
        for flags in [
            UNSANDBOXED,
            MAX_RESTRICTION,
            Some(F::ALLOW_POPUPS),
            Some(F::ALLOW_TOP_NAVIGATION),
        ] {
            for activation in [false, true] {
                assert_eq!(
                    window_open_disposition("_self", flags, activation),
                    D::SelfNavigate,
                    "flags = {flags:?}, activation = {activation}"
                );
            }
        }
        // Keyword detection is ASCII case-insensitive.
        assert_eq!(
            window_open_disposition("_SELF", MAX_RESTRICTION, false),
            D::SelfNavigate
        );
    }

    #[test]
    fn blank_and_empty_target_track_popups_allowed() {
        // §7.2.2.1 step 5 maps "" → `_blank`; both then gate on the
        // *sandboxed auxiliary navigation* flag (§7.3.1.7 step 8) —
        // activation plays no part in the popup gate.
        for target in ["", "_blank", "_BLANK", "_Blank"] {
            for activation in [false, true] {
                assert_eq!(
                    window_open_disposition(target, UNSANDBOXED, activation),
                    D::OpenTab,
                    "target = {target:?}"
                );
                assert_eq!(
                    window_open_disposition(target, MAX_RESTRICTION, activation),
                    D::Blocked,
                    "target = {target:?}"
                );
                assert_eq!(
                    window_open_disposition(target, Some(F::ALLOW_POPUPS), activation),
                    D::OpenTab,
                    "target = {target:?}"
                );
            }
        }
        // An unrelated grant does not open the popup gate.
        assert_eq!(
            window_open_disposition("_blank", Some(F::ALLOW_TOP_NAVIGATION), false),
            D::Blocked
        );
    }

    #[test]
    fn parent_and_top_track_top_navigation_allowed() {
        // §7.4.2.4 steps 3.2/3.3 — the 2-flag pair with the activation
        // parameter (the full truth table over the predicate itself lives
        // in elidex-plugin; this pins the dispatch arm's wiring).
        let by_ua = F::ALLOW_TOP_NAVIGATION_BY_USER_ACTIVATION;
        for target in ["_parent", "_top", "_TOP", "_Parent"] {
            // Unsandboxed: allowed either way.
            assert_eq!(
                window_open_disposition(target, UNSANDBOXED, false),
                D::TopNavigate,
                "target = {target:?}"
            );
            // Maximum restriction: blocked either way.
            assert_eq!(
                window_open_disposition(target, MAX_RESTRICTION, true),
                D::Blocked,
                "target = {target:?}"
            );
            // `allow-top-navigation`: allowed regardless of activation.
            assert_eq!(
                window_open_disposition(target, Some(F::ALLOW_TOP_NAVIGATION), false),
                D::TopNavigate,
                "target = {target:?}"
            );
            // `allow-top-navigation-by-user-activation` alone: gated on the
            // activation fact.
            assert_eq!(
                window_open_disposition(target, Some(by_ua), false),
                D::Blocked,
                "target = {target:?}"
            );
            assert_eq!(
                window_open_disposition(target, Some(by_ua), true),
                D::TopNavigate,
                "target = {target:?}"
            );
            // A popup grant is not a top-navigation grant.
            assert_eq!(
                window_open_disposition(target, Some(F::ALLOW_POPUPS), true),
                D::Blocked,
                "target = {target:?}"
            );
        }
    }

    #[test]
    fn named_target_is_always_named_with_snapshot_verdict() {
        // A name is never Blocked at enqueue — the aux-nav verdict rides
        // the payload for the shell's MISS branch. `_`-prefixed non-keywords
        // are names, not keywords.
        for target in ["frameA", "_weird", "_blankx", "content"] {
            for activation in [false, true] {
                assert_eq!(
                    window_open_disposition(target, UNSANDBOXED, activation),
                    D::Named {
                        aux_nav_allowed: true
                    },
                    "target = {target:?}"
                );
                assert_eq!(
                    window_open_disposition(target, MAX_RESTRICTION, activation),
                    D::Named {
                        aux_nav_allowed: false
                    },
                    "target = {target:?}"
                );
                assert_eq!(
                    window_open_disposition(target, Some(F::ALLOW_POPUPS), activation),
                    D::Named {
                        aux_nav_allowed: true
                    },
                    "target = {target:?}"
                );
                // The verdict tracks popups_allowed, not top-navigation.
                assert_eq!(
                    window_open_disposition(target, Some(F::ALLOW_TOP_NAVIGATION), activation),
                    D::Named {
                        aux_nav_allowed: false
                    },
                    "target = {target:?}"
                );
            }
        }
    }
}
