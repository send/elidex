//! WHATWG HTML §6.6.4 "get the focusable area" / "focus delegate" — the
//! shadow-`delegatesFocus` retarget (PR-A1): map a focus target that is not
//! itself a focusable area to the area that should receive focus, plus the
//! criterion-2-aware focusable-area predicate ([`is_focusable_area`]) and the
//! [`FocusTrigger`] the focus-processing algorithms thread through.

use super::*;
use elidex_ecs::ShadowRoot;

/// §6.6.4 "focus trigger" (WHATWG HTML `#get-the-focusable-area`, which defines
/// it) — the optional string
/// passed to the focus-processing algorithms, defaulting to "other". The spec
/// models it as an open string and behaviourally distinguishes only `"click"`
/// (the `autofocus_delegate` click-focusable filter). Sequential focus
/// navigation never reaches these steps for shadow hosts (the note under
/// `get the focusable area`), so no "sequential" value is needed here; the
/// `:focus-visible` keyboard/script heuristic is a **separate** signal and is
/// intentionally not modelled by this enum.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FocusTrigger {
    /// The focus was triggered by a pointer click (the spec's `"click"`).
    Click,
    /// Any other trigger (the spec default, `"other"`).
    #[default]
    Other,
}

/// §6.6.4 "get the focusable area" (WHATWG HTML `#get-the-focusable-area`) — map
/// a focus target that is **not itself a focusable area** to the focusable area
/// that should receive focus, performing the shadow-`delegatesFocus` retarget.
///
/// Spec branches, in first-match order:
/// 1. an `area` element with focusable shapes → the shape of the first `img`
///    using its image map — **unmodelled** (elidex has no image-map focusable
///    shapes); falls through.
/// 2. an element with scrollable regions that are focusable areas → the first
///    region — **unmodelled** (scrollable regions are not focusable-area
///    entities); falls through.
/// 3. the document element → the `Document`'s viewport — **unmodelled** (no
///    viewport focusable-area entity); falls through.
/// 4. a navigable → its active document — **cross-frame**, out of a single
///    `EcsDom`; slot `#11-cross-frame-sequential-focus-nav`. Falls through.
/// 5. a navigable container with a content navigable → the content document —
///    **cross-frame** (the shell routes iframe clicks before `set_focus`, into a
///    separate `EcsDom`); slot `#11-oop-iframe-focus-lifecycle`. Falls through.
/// 6. a shadow host whose shadow root delegates focus → the `focus_delegate`
///    (with the 6.1/6.2 "keep focus already inside this host" shortcut).
///    **Implemented.**
/// 7. otherwise → null.
///
/// Branches 1–5 are not reachable in elidex's current model (their underlying
/// state is unmodelled or cross-frame), so this returns non-null **only** via the
/// shadow-`delegatesFocus` branch. That makes it safe to call on *any* click
/// target without a §6.6.2-complete "is it a focusable area" pre-gate: the only
/// targets it retargets (returns non-null for) are delegates-focus shadow hosts
/// (branch 6), which §6.6.2 criterion 2 says are **not** focusable areas — so
/// retargeting them to their delegate is correct; any genuine §6.6.2 focusable
/// area has no delegates-focus shadow root, falls to branch 7 (null), and the
/// caller keeps the original target (`unwrap_or(hit)`) — reproducing the
/// focusing-steps step-1 gate. (The global [`is_focusable`] predicate omits
/// §6.6.2 criterion 2 — slot `#11-shadow-focus-delegation` — but that is not
/// relied on here: the criterion-2-aware `is_focusable_area` predicate is applied
/// only inside the focus-delegate descendant walk, never to the Tab-nav /
/// `reconcile_focus` predicate, which is PR-A3's sequential-nav axis.)
#[must_use]
pub fn get_the_focusable_area(
    dom: &EcsDom,
    target: Entity,
    trigger: FocusTrigger,
) -> Option<Entity> {
    // Branch 6 — a shadow host whose shadow root delegates focus. (Branches 1–5
    // are unmodelled / cross-frame per the doc above; branch 7 = the final
    // `None`. A delegates-focus host never matches 1–5, so testing 6 first is
    // order-equivalent in elidex's model.)
    if !delegates_focus_host(dom, target) {
        return None; // not branch 6 → branch 7 (null)
    }
    // 6.1/6.2 — if `target` is a shadow-including inclusive ancestor of the
    // currently focused area, keep that focus rather than re-delegating to the
    // first focusable. (`is_host_including_ancestor_or_self` jumps ShadowRoot→host,
    // so it is the shadow-including inclusive-ancestor walk for elidex's only
    // host-bearing roots.)
    if let Some(document) = dom.owner_document(target) {
        if let Some(focused) = current_focus(dom, document) {
            if dom.is_host_including_ancestor_or_self(target, focused) {
                return Some(focused);
            }
        }
    }
    // 6.3 — the focus delegate.
    focus_delegate(dom, target, trigger)
}

/// Whether the shadow root entity `shadow_root` has `delegates focus` set
/// (`ShadowRoot` is `Copy`; a missing/failed read is treated as `false`).
fn shadow_delegates_focus(dom: &EcsDom, shadow_root: Entity) -> bool {
    dom.world()
        .get::<&ShadowRoot>(shadow_root)
        .is_ok_and(|r| r.delegates_focus)
}

/// Whether `entity` is a shadow host whose shadow root delegates focus — the
/// §6.6.2 criterion-2 exclusion, and the [`get_the_focusable_area`] branch-6
/// trigger (the single home for "is this the delegates-focus host case").
fn delegates_focus_host(dom: &EcsDom, entity: Entity) -> bool {
    dom.get_shadow_root(entity)
        .is_some_and(|sr| shadow_delegates_focus(dom, sr))
}

/// Whether `entity` is a WHATWG HTML §6.6.2 focusable area **including criterion
/// 2** — an element that is a shadow host whose shadow root delegates focus is
/// *not* a focusable area (its delegate is). [`is_focusable`] intentionally omits
/// criterion 2 (slot `#11-shadow-focus-delegation`: a global exclusion would make
/// `host.focus()` a no-op, since the host *is* the `focus()` target), so the
/// callers that implement the §6.6.4 focusing-steps step-1 "is the target a
/// focusable area, else retarget" gate apply C2 here:
/// - the focus-delegate descendant walk (`autofocus_delegate` /
///   `first_delegate_descendant`) — a nested delegates-focus host among the
///   descendants must fall to the get-the-focusable-area recursion (§6.6.4
///   focus-delegate step 6.3→6.4 / autofocus-delegate step 1.2), not be returned
///   as the delegate itself;
/// - the shell pointer-click gate (`focus_target_for_click`) — a clicked
///   delegates-focus host with no delegate is not a focusable area, so it must not
///   itself receive focus.
///
/// **Not** the Tab-nav / `reconcile_focus` predicate, which stay on the C2-blind
/// [`is_focusable`] until PR-A3 unifies the focusability predicate (the
/// sequential-nav axis — folding C2 there changes the Tab order and needs the
/// delegate descent, out of A1's scope).
#[must_use]
pub fn is_focusable_area(dom: &EcsDom, entity: Entity) -> bool {
    is_focusable(dom, entity) && !delegates_focus_host(dom, entity)
}

/// §6.6.4 "the descendant's focusable area" (focus-delegate step 6.3→6.4 /
/// autofocus-delegate step 1.2) — the `child` itself when it is a §6.6.2 focusable
/// area ([`is_focusable_area`], criterion-2-aware), else its get-the-focusable-area
/// retarget (a nested delegates-focus host → its delegate; a non-host non-area →
/// null). The single home for "resolve a delegate-walk descendant to its focusable
/// area", shared by both delegate walkers ([`autofocus_delegate`] /
/// [`first_delegate_descendant`]) — the only step that differs between them is the
/// autofocus pre-gate + click filter, not this resolution.
fn child_focusable_area(dom: &EcsDom, child: Entity, trigger: FocusTrigger) -> Option<Entity> {
    if is_focusable_area(dom, child) {
        Some(child)
    } else {
        get_the_focusable_area(dom, child, trigger)
    }
}

/// §6.6.4 "focus delegate" (WHATWG HTML `#focus-delegate`) — resolve the
/// focusable area that a `delegatesFocus` shadow host delegates to: the autofocus
/// delegate if any, else the first focusable area in tree order.
///
/// **Dialog branch omitted (step 6.2).** The spec's focus delegate has a
/// `<dialog>`-specific branch (delegate to a *sequentially* focusable descendant),
/// but a `<dialog>` is **not a valid shadow host** (WHATWG DOM `attachShadow`
/// host list), so [`get_the_focusable_area`]'s branch 6 — the only A1 caller —
/// never passes a dialog here. The dialog focus-delegate path is reached only
/// from `<dialog>` `showModal()` / autofocus processing; it lands with that
/// caller (slot `#11-flush-autofocus-candidates-page-load`), which also supplies
/// the *sequentially focusable* predicate it needs (kept out of A1 rather than
/// adding an unreachable, untestable branch — "dead code は接続するか削除").
fn focus_delegate(dom: &EcsDom, focus_target: Entity, trigger: FocusTrigger) -> Option<Entity> {
    // Steps 1–3 — pick `whereToLook`.
    let where_to_look = match dom.get_shadow_root(focus_target) {
        Some(shadow_root) => {
            if !shadow_delegates_focus(dom, shadow_root) {
                return None; // step 1
            }
            shadow_root // step 3 — whereToLook = the shadow root
        }
        None => focus_target, // step 2
    };
    // Steps 4–5 — an `autofocus` descendant wins.
    if let Some(area) = autofocus_delegate(dom, where_to_look, trigger, 0) {
        return Some(area);
    }
    // Step 6 — the first focusable area among `whereToLook`'s descendants in tree
    // order; step 7 — null otherwise.
    first_delegate_descendant(dom, where_to_look, trigger, 0)
}

/// §6.6.4 "autofocus delegate" (WHATWG HTML `#autofocus-delegate`) — the first
/// descendant carrying an `autofocus` content attribute whose focusable area is
/// suitable for `trigger`, in tree order. Walks **plain** (light-tree)
/// descendants via [`EcsDom::child_list_uncapped`] (excludes shadow roots) — the
/// delegate search is a spec-ordered *exhaustive* traversal, so it must not
/// silently truncate a wide sibling list the way the `MAX_ANCESTOR_DEPTH`-capped
/// `children_iter` would; a shadow host descendant's tree is entered only via
/// [`get_the_focusable_area`] when that host itself carries `autofocus`. (The
/// `depth` recursion guard is retained for stack safety against a pathologically
/// deep tree — that bounds *nesting depth*, not sibling breadth.)
pub(super) fn autofocus_delegate(
    dom: &EcsDom,
    where_to_look: Entity,
    trigger: FocusTrigger,
    depth: usize,
) -> Option<Entity> {
    if depth >= elidex_ecs::MAX_ANCESTOR_DEPTH {
        return None;
    }
    for child in dom.child_list_uncapped(where_to_look) {
        if dom.has_attribute(child, "autofocus") {
            // step 1.2 — the descendant's focusable area.
            if let Some(area) = child_focusable_area(dom, child, trigger) {
                // step 1.4 — skip a non-click-focusable area for a "click" trigger;
                // step 1.5 — otherwise it is the delegate.
                if trigger != FocusTrigger::Click || is_click_focusable(dom, area) {
                    return Some(area);
                }
            }
        }
        if let Some(found) = autofocus_delegate(dom, child, trigger, depth + 1) {
            return Some(found);
        }
    }
    None
}

/// §6.6.4 focus-delegate step 6 — the first focusable area among `node`'s
/// (light-tree) descendants in tree order. Walks **plain** descendants (the spec
/// is explicit it is *not* the shadow-including descendants) via
/// [`EcsDom::child_list_uncapped`] (excludes shadow roots) — like
/// [`autofocus_delegate`], this is an exhaustive spec-ordered search, so it uses
/// the uncapped traversal rather than the `MAX_ANCESTOR_DEPTH`-capped
/// `children_iter` (silently dropping a later sibling would skip the real focus
/// delegate); a nested shadow host is handled by recursing through
/// [`get_the_focusable_area`] (step 6.4), not by walking into its shadow tree.
fn first_delegate_descendant(
    dom: &EcsDom,
    node: Entity,
    trigger: FocusTrigger,
    depth: usize,
) -> Option<Entity> {
    if depth >= elidex_ecs::MAX_ANCESTOR_DEPTH {
        return None;
    }
    for child in dom.child_list_uncapped(node) {
        // steps 6.3/6.4 — the child if it is a focusable area, else its
        // get-the-focusable-area retarget (a nested delegates-focus host recurses
        // into its shadow tree rather than being returned as itself).
        if let Some(area) = child_focusable_area(dom, child, trigger) {
            return Some(area); // step 6.5
        }
        if let Some(found) = first_delegate_descendant(dom, child, trigger, depth + 1) {
            return Some(found);
        }
    }
    None
}
