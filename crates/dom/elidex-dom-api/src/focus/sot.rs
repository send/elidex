//! The focus source-of-truth: the canonical [`ElementState::FOCUS`] bit, its
//! single READ model ([`current_focus`]) and single WRITE model
//! ([`set_focus_bit`] / [`blur`]) and the active-document membership test
//! ([`is_in_document`]). The asynchronous focusability fixup (WHATWG HTML
//! "update the rendering" step 17) is [`focus_fixup_steps`] in `update_steps`.

// Cohesive `focus` module split: the submodules share the parent namespace.
#[allow(clippy::wildcard_imports)]
use super::*;

/// The raw [`ElementState::FOCUS`]-bit holder, with NO connectedness or
/// focusability filtering — the single canonical bit query shared by
/// [`current_focus`] (which then *derives* the effective focused area) and
/// [`blur`] (which must clear a connected-but-non-focusable holder whatever its
/// document, so it reads the bit unfiltered).
pub(crate) fn raw_focus_holder(dom: &EcsDom) -> Option<Entity> {
    dom.world()
        .query::<(Entity, &ElementState)>()
        .iter()
        .find(|(_, s)| s.contains(ElementState::FOCUS))
        .map(|(e, _)| e)
}

/// The currently focused element of `document`, if any (WHATWG HTML §6.6 — **the
/// single READ model** behind `document.activeElement` / `hasFocus`, the
/// `:focus` selector, and every shell focus read site). Reads the canonical
/// [`ElementState::FOCUS`] bit, scoped to the bound document.
///
/// The bit **is** the document's *focused area* (the single SoT), so this read
/// stays consistent with every other focus consumer — crucially the `:focus`
/// selector, which matches the same bit directly (`elidex-css`
/// `selector/matching.rs`). A focused element that becomes non-focusable in the
/// same JS turn (what makes it so: see [`focus_fixup_steps`])
/// **remains** the focused area until the render-time [`focus_fixup_steps`]
/// moves focus to the viewport: WHATWG HTML "update the rendering" step 17 makes that
/// focusability fixup **asynchronous** (run at the next rendering update), in
/// contrast to the **synchronous** fixup for *removal* (§2.1.4 removing steps,
/// `EcsDom::fire_after_remove`). So `activeElement` keeps reporting the
/// soon-to-be-blurred element until the frame fixup — matching the spec and
/// staying consistent with `:focus`.
///
/// Filtering focusability *here* (a `&& is_focusable` derive-on-read) would
/// instead split `activeElement` from the `:focus` selector within a turn and
/// make `activeElement` non-spec (it would blur eagerly, before the async
/// rendering fixup) — Codex S2 R7.
///
/// The `is_in_document` walk scopes the read to the bound document (never a
/// `document.cloneNode()` subtree sharing the world); it is a defensive guard —
/// `ElementState` is a non-copied component (clones never carry the bit) and
/// `focus()` gates document membership, so it should never actually filter.
#[must_use]
pub fn current_focus(dom: &EcsDom, document: Entity) -> Option<Entity> {
    let focused = raw_focus_holder(dom)?;
    is_in_document(dom, focused, document).then_some(focused)
}

/// Whether `entity` is an inclusive descendant of `document` — its light-tree
/// ancestor chain reaches `document`. The **active-document membership** test:
/// focus is the active document's focused area (WHATWG HTML §6.6), so a focus
/// *writer* must reject a target outside the bound document. [`is_connected`]
/// alone is insufficient — a `document.cloneNode()` subtree reports connected
/// (its root *is* a `Document`) yet is not the bound document, and the
/// world-wide [`set_focus_bit`] sweep would otherwise clobber the live
/// document's holder. Shares the bounded ancestor walk with [`current_focus`]
/// (one home for the "is this entity in that document" question).
///
/// [`is_connected`]: EcsDom::is_connected
#[must_use]
pub fn is_in_document(dom: &EcsDom, entity: Entity, document: Entity) -> bool {
    let mut cur = Some(entity);
    let mut depth = 0;
    while let Some(c) = cur {
        if c == document {
            return true;
        }
        // Defensive depth cap, matching the codebase's other ancestor walkers
        // (`find_link_ancestor`, `build_propagation_path`): a malformed parent
        // cycle must not hang this read, which runs on hot UA paths (keydown /
        // caret blink / IME / a11y rebuild).
        if depth >= elidex_ecs::MAX_ANCESTOR_DEPTH {
            return false;
        }
        cur = dom.get_parent(c);
        depth += 1;
    }
    false
}

/// Move focus to `new` (or clear it when `None`) — the single WRITE model
/// (WHATWG HTML §6.6). Clears [`ElementState::FOCUS`] from **all** current
/// holders in the world, then sets it on `new` if `Some`. The clear-all sweep
/// makes the single-focus invariant hold *by construction* across every writer
/// (shell UA input ∪ VM `focus()`), with no separate "previously focused"
/// record to keep in sync.
///
/// The world-wide sweep is the per-document single-focus reconcile because
/// every writer targets the *active* document only: a shell pipeline owns one
/// `EcsDom` per rendered document, and although the VM may hold additional
/// non-active documents in its world (e.g. a `document.cloneNode()` subtree),
/// `HTMLElement.focus()` gates on bound-document membership ([`is_in_document`])
/// so a non-active document's element is never passed here. A caller that can
/// hold multiple live documents in one world MUST preserve that gate, else this
/// sweep would clobber the active document's holder.
///
/// Does **not** dispatch focus events (engine-bound; the focusing-steps §6.6.4
/// `focusout`/`focusin`/`blur`/`focus` stay with the caller — the shell
/// reconciler brackets its event dispatch around `set_focus_bit(_, None)` then
/// `set_focus_bit(_, Some(new))`; the VM `focus()`/`blur()` defer events to slot
/// `#11-vm-host-synthetic-dom-event-dispatch`).
pub fn set_focus_bit(dom: &mut EcsDom, new: Option<Entity>) {
    let holders: Vec<Entity> = dom
        .world()
        .query::<(Entity, &ElementState)>()
        .iter()
        .filter(|(_, s)| s.contains(ElementState::FOCUS))
        .map(|(e, _)| e)
        .collect();
    for e in holders {
        if Some(e) == new {
            continue;
        }
        update_state(dom, e, |s| s.remove(ElementState::FOCUS));
    }
    if let Some(e) = new {
        update_state(dom, e, |s| s.insert(ElementState::FOCUS));
    }
}

/// Unfocus `entity` at the bit level (WHATWG HTML §6.6.4 unfocusing steps): if
/// `entity` currently holds the canonical [`ElementState::FOCUS`] bit, clear it;
/// otherwise a no-op (blurring an unfocused element does nothing).
///
/// Operates on the raw bit holder (`raw_focus_holder`) — `blur()` is an explicit
/// WRITE on the focus SoT, so it clears the bit even when a same-turn mutation
/// has made the holder non-focusable but the asynchronous render fixup
/// ([`focus_fixup_steps`]) has not run yet. Without the explicit clear, the
/// lingering bit (the holder is still the focused area until the async fixup)
/// would survive to a same-turn un-hide and resurrect `document.activeElement`
/// despite the `blur()`, e.g.
/// `el.focus(); el.hidden = true; el.blur(); el.hidden = false` (Codex S2 R6).
///
/// Event dispatch (`blur` / `focusout`) is deferred with the rest of the
/// VM-host synthetic events (slot `#11-vm-host-synthetic-dom-event-dispatch`),
/// so this is a component-only mutation.
pub fn blur(dom: &mut EcsDom, entity: Entity) {
    if raw_focus_holder(dom) == Some(entity) {
        set_focus_bit(dom, None);
    }
}

/// Read-modify-write one entity's [`ElementState`] (creating it from the
/// `Default` when absent). Mirrors the shell's `update_element_state`, kept
/// private so `set_focus_bit` is the only public mutator of the `FOCUS` bit.
fn update_state(dom: &mut EcsDom, entity: Entity, f: impl FnOnce(&mut ElementState)) {
    let mut state = dom
        .world()
        .get::<&ElementState>(entity)
        .ok()
        .map_or(ElementState::default(), |s| *s);
    f(&mut state);
    let _ = dom.world_mut().insert_one(entity, state);
}
