//! Focus state + focusable-area helpers (WHATWG HTML §6.6).
//!
//! The engine-independent home for focus as a DOM concept, so the shell's
//! UA-input path and the JS VM's `HTMLElement.focus()`/`blur()` drive focus
//! through one source of truth — the canonical [`ElementState::FOCUS`]
//! component — rather than parallel `Option<Entity>` side-stores. Three
//! responsibilities:
//!
//! - **focusable area** ([`tab_index_default_for`] / [`is_focusable`], WHATWG
//!   HTML §6.6.2 Data model / §6.6.3 The tabindex attribute) — the per-element
//!   default tab index and whether an element can receive focus (incl. the
//!   §6.6.2 *connectedness* requirement, the write-side gate of the invariant).
//! - **the READ model** ([`current_focus`]) — the single query for the focused
//!   element; its connectedness walk is a *defensive guard* (the bit is
//!   connected by construction: gated at focus, cleared at removal).
//! - **the WRITE model** ([`set_focus_bit`]) — clear-all-then-set, so the
//!   single-focus invariant holds *by construction* across every writer (no
//!   "previously focused" record to keep in sync).
//!
//! The `FOCUS`-set ⟹ connected invariant is maintained by [`is_focusable`]
//! (rejects disconnected `focus()` targets) and `EcsDom::fire_after_remove`
//! (clears the bit when its holder leaves the tree, WHATWG HTML §2.1.4 removing
//! steps — silently). So focus needs **one** read model: there is no by-identity
//! second read.
//!
//! Engine- and form-independent: this crate has no `elidex-form` dependency, so
//! the focusable predicate is attribute-based. Event dispatch (the focusing
//! steps §6.6.4 fire `focusout`/`focusin`/`blur`/`focus`) is engine-bound and
//! stays with the caller; these helpers only reconcile the `FOCUS` bit.

use elidex_ecs::{EcsDom, ElementState, Entity};

/// Per-element default `tabIndex` value (WHATWG HTML §6.6.3 "tabindex value" —
/// the value when no `tabindex` content attribute is present): `0` for
/// intrinsically focusable areas (button / select / textarea / iframe / object
/// / embed, `<a>`/`<area>` with `href`, `<input>` other than `type=hidden`,
/// `contenteditable` elements), `-1` otherwise.
///
/// Backs the `tabIndex` IDL getter — it reflects the default tab *order* and is
/// independent of disabled state (a disabled `<button>` still has `tabIndex`
/// `0`); see [`is_focusable`] for the focusability decision, which does honour
/// `disabled`.
#[must_use]
pub fn tab_index_default_for(dom: &EcsDom, entity: Entity) -> i32 {
    // Tag-driven branch decisions read the borrowed tag directly so the
    // lowercase comparison is zero-allocation; an explicit `Option<TagDefault>`
    // enum lets the inner `dom.with_attribute` / `dom.has_attribute` calls run
    // AFTER the tag borrow drops.
    enum TagDefault {
        // Definitely focus-zero (button / select / textarea / iframe / object
        // / embed) — no further attribute lookup needed.
        Zero,
        // Link — focus-zero only when the element also carries `href`.
        Link,
        // `<input>` — focus-zero unless `type="hidden"`.
        Input,
        // Generic element — depends on `contenteditable`.
        Generic,
    }
    let kind = dom.with_tag_name(entity, |t| match t {
        None => None,
        Some(s) => {
            if s.eq_ignore_ascii_case("button")
                || s.eq_ignore_ascii_case("select")
                || s.eq_ignore_ascii_case("textarea")
                || s.eq_ignore_ascii_case("iframe")
                || s.eq_ignore_ascii_case("object")
                || s.eq_ignore_ascii_case("embed")
            {
                Some(TagDefault::Zero)
            } else if s.eq_ignore_ascii_case("a") || s.eq_ignore_ascii_case("area") {
                Some(TagDefault::Link)
            } else if s.eq_ignore_ascii_case("input") {
                Some(TagDefault::Input)
            } else {
                Some(TagDefault::Generic)
            }
        }
    });
    let focusable = match kind {
        None => false,
        Some(TagDefault::Zero) => true,
        Some(TagDefault::Link) => dom.has_attribute(entity, "href"),
        Some(TagDefault::Input) => {
            // `<input type="hidden">` is unfocusable; everything else
            // participates in sequential focus navigation.
            !dom.with_attribute(entity, "type", |t| {
                t.is_some_and(|s| s.eq_ignore_ascii_case("hidden"))
            })
        }
        Some(TagDefault::Generic) => dom.with_attribute(entity, "contenteditable", |v| {
            v.is_some_and(|s| {
                s.is_empty()
                    || s.eq_ignore_ascii_case("true")
                    || s.eq_ignore_ascii_case("plaintext-only")
            })
        }),
    };
    if focusable {
        0
    } else {
        -1
    }
}

/// Whether `entity` is a focusable area (WHATWG HTML §6.6.2): an explicit
/// `tabindex` content attribute OR a non-negative per-element default
/// ([`tab_index_default_for`]), unless the element is an *actually disabled*
/// disablable form element (a direct `disabled` attribute on `button` /
/// `input` / `select` / `textarea` / `optgroup` / `option` / `fieldset`).
///
/// A focusable area must also be **connected** — WHATWG HTML §6.6.2 requires it
/// be "being rendered", which a disconnected element is not, so
/// `createElement('input').focus()` on an unattached element is a no-op. This
/// predicate enforces the *connectedness* half via [`EcsDom::is_connected`]; the
/// remaining "being rendered" fidelity (display:none / visibility:hidden, which
/// needs computed style / layout, a layer this attribute-based predicate cannot
/// reach) is slot `#11-focusable-area-being-rendered`.
///
/// Attribute-based by necessity (this crate has no `elidex-form` dependency).
/// `contenteditable` is checked via [`EcsDom::is_contenteditable`], which honours
/// **ancestor inheritance** (WHATWG HTML §6.8.1: the missing/invalid state
/// inherits from the parent), so a descendant of an editing host
/// (`<div contenteditable><span>…</span></div>`) is focusable.
/// **Fieldset-inherited** disabled (a control nested in `<fieldset disabled>`)
/// is *not* captured here — that lives in the form subsystem; the shell overlays
/// `FormControlState` for it on its UA-input path, and slot
/// `#11-focusable-area-fieldset-inherited-disabled` tracks bringing it to this
/// predicate for the VM `focus()` path.
#[must_use]
pub fn is_focusable(dom: &EcsDom, entity: Entity) -> bool {
    if !dom.is_connected(entity) {
        return false;
    }
    if is_actually_disabled(dom, entity) {
        return false;
    }
    // The `tabindex` attribute participates only when it parses as a valid
    // integer (WHATWG HTML §6.6.3 "rules for parsing integers"); an invalid
    // value (`tabindex="foo"`) yields no tabindex and falls through to the
    // per-element default — matching the `tabIndex` IDL getter (one parse, one
    // home: [`parse_tab_index_value`]).
    dom.with_attribute(entity, "tabindex", |v| {
        v.and_then(parse_tab_index_value).is_some()
    }) || dom.is_contenteditable(entity)
        || tab_index_default_for(dom, entity) >= 0
}

/// Parse a `tabindex` content attribute value (WHATWG HTML §6.6.3 "tabindex
/// value" — the attribute is parsed using the rules for parsing integers; a
/// failure yields a null tabindex). The engine-independent home shared by the
/// focusable-area predicate ([`is_focusable`]) and the VM `tabIndex` IDL getter,
/// so the two never diverge (one-issue-one-way).
#[must_use]
pub fn parse_tab_index_value(raw: &str) -> Option<i32> {
    raw.trim().parse::<i32>().ok()
}

/// A disablable form element carrying a direct `disabled` content attribute.
/// (Direct-attribute only; fieldset inheritance is the slot above.)
fn is_actually_disabled(dom: &EcsDom, entity: Entity) -> bool {
    dom.has_attribute(entity, "disabled")
        && dom.with_tag_name(entity, |t| {
            t.is_some_and(|s| {
                s.eq_ignore_ascii_case("button")
                    || s.eq_ignore_ascii_case("input")
                    || s.eq_ignore_ascii_case("select")
                    || s.eq_ignore_ascii_case("textarea")
                    || s.eq_ignore_ascii_case("optgroup")
                    || s.eq_ignore_ascii_case("option")
                    || s.eq_ignore_ascii_case("fieldset")
            })
        })
}

/// The currently focused element of `document`, if any (WHATWG HTML §6.6 — **the
/// single READ model** behind `document.activeElement` / `hasFocus` and every
/// shell focus read site). Queries the canonical [`ElementState::FOCUS`] bit.
///
/// The `FOCUS` bit is maintained **connected by construction**: [`is_focusable`]
/// (the write-side gate at every `focus()` entry) rejects disconnected targets,
/// and `EcsDom::fire_after_remove` (WHATWG HTML §2.1.4 removing steps) clears the
/// bit when its holder leaves the tree. So the trailing **connectedness walk** to
/// `document` is a *defensive guard* for this UA-critical read — not the primary
/// mechanism: it should never actually filter, but it keeps `activeElement`
/// correct if any future writer bypasses the gate. (Its sole former rival, the
/// by-identity `is_focused`, is gone — single read model.)
#[must_use]
pub fn current_focus(dom: &EcsDom, document: Entity) -> Option<Entity> {
    let focused = dom
        .world()
        .query::<(Entity, &ElementState)>()
        .iter()
        .find(|(_, s)| s.contains(ElementState::FOCUS))
        .map(|(e, _)| e)?;
    if !dom.contains(focused) {
        return None;
    }
    let mut cur = Some(focused);
    let mut depth = 0;
    while let Some(c) = cur {
        if c == document {
            return Some(focused);
        }
        // Defensive depth cap, matching the codebase's other ancestor walkers
        // (`find_link_ancestor`, `build_propagation_path`): a malformed parent
        // cycle must not hang this read, which now runs on hot UA paths
        // (keydown / caret blink / IME / a11y rebuild).
        if depth >= elidex_ecs::MAX_ANCESTOR_DEPTH {
            break;
        }
        cur = dom.get_parent(c);
        depth += 1;
    }
    None
}

/// Move focus to `new` (or clear it when `None`) — the single WRITE model
/// (WHATWG HTML §6.6). Clears [`ElementState::FOCUS`] from **all** current
/// holders in the world, then sets it on `new` if `Some`. The clear-all sweep
/// makes the single-focus invariant hold *by construction* across every writer
/// (shell UA input ∪ VM `focus()`), with no separate "previously focused"
/// record to keep in sync — each pipeline owns one `EcsDom` per document, so
/// the world-wide sweep is exactly the per-document single-focus reconcile.
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

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    #[test]
    fn set_focus_bit_enforces_single_focus() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let a = dom.create_element("div", Attributes::default());
        let b = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, a);
        let _ = dom.append_child(doc, b);

        set_focus_bit(&mut dom, Some(a));
        assert_eq!(current_focus(&dom, doc), Some(a));
        // Focusing `b` sweeps `a`'s bit — single-focus by construction.
        set_focus_bit(&mut dom, Some(b));
        assert_eq!(current_focus(&dom, doc), Some(b));
        // Confirm only one holder remains (no stale bit on `a`).
        let holders = dom
            .world()
            .query::<(Entity, &ElementState)>()
            .iter()
            .filter(|(_, s)| s.contains(ElementState::FOCUS))
            .count();
        assert_eq!(holders, 1);

        set_focus_bit(&mut dom, None);
        assert_eq!(current_focus(&dom, doc), None);
    }

    #[test]
    fn current_focus_filters_disconnected() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        // Focused but never connected to `doc` (e.g. createElement + .focus()).
        let orphan = dom.create_element("div", Attributes::default());
        set_focus_bit(&mut dom, Some(orphan));
        assert_eq!(
            current_focus(&dom, doc),
            None,
            "a disconnected focus holder is filtered out at read"
        );
    }

    /// Create `tag` and attach it under `doc` so it is connected (focusable
    /// areas must be connected — `is_focusable` gates on `is_connected`, §6.6.2).
    fn connect_el(dom: &mut EcsDom, doc: Entity, tag: &str) -> Entity {
        let el = dom.create_element(tag, Attributes::default());
        let _ = dom.append_child(doc, el);
        el
    }

    #[test]
    fn is_focusable_attribute_based() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let plain = connect_el(&mut dom, doc, "div");
        assert!(!is_focusable(&dom, plain), "plain <div> is not focusable");

        let with_tabindex = connect_el(&mut dom, doc, "div");
        dom.set_attribute(with_tabindex, "tabindex", "0");
        assert!(
            is_focusable(&dom, with_tabindex),
            "tabindex makes it focusable"
        );

        let anchor = connect_el(&mut dom, doc, "a");
        assert!(
            !is_focusable(&dom, anchor),
            "<a> without href is not focusable"
        );
        dom.set_attribute(anchor, "href", "x");
        assert!(is_focusable(&dom, anchor), "<a href> is focusable");

        let input = connect_el(&mut dom, doc, "input");
        assert!(is_focusable(&dom, input), "<input> is focusable");

        let disabled = connect_el(&mut dom, doc, "button");
        dom.set_attribute(disabled, "disabled", "");
        assert!(
            !is_focusable(&dom, disabled),
            "a disabled disablable element is not focusable"
        );

        // An invalid `tabindex` (not a valid integer) does NOT make a plain
        // element focusable (§6.6.3 parse, not mere presence).
        let bad_tabindex = connect_el(&mut dom, doc, "div");
        dom.set_attribute(bad_tabindex, "tabindex", "foo");
        assert!(
            !is_focusable(&dom, bad_tabindex),
            "tabindex=\"foo\" is not a valid integer ⇒ not focusable"
        );
    }

    #[test]
    fn is_focusable_requires_connectedness() {
        // §6.6.2: a focusable area must be "being rendered" (⊇ connected);
        // `createElement('input').focus()` on a never-attached element is a
        // no-op. A disconnected element — even an intrinsically-focusable
        // `<input>` — is not a focusable area.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let orphan = dom.create_element("input", Attributes::default());
        assert!(
            !is_focusable(&dom, orphan),
            "a disconnected <input> is not focusable"
        );
        let _ = dom.append_child(doc, orphan);
        assert!(
            is_focusable(&dom, orphan),
            "once connected, the <input> becomes focusable"
        );
    }

    #[test]
    fn is_focusable_inherits_contenteditable() {
        // Regression (Codex R1 F3): an editing-host descendant must stay
        // focusable via ancestor-inherited `contenteditable` (WHATWG §6.8.1),
        // not only a direct attribute — else clicking a child inside a
        // `<div contenteditable>` blurs the editor.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let editor = connect_el(&mut dom, doc, "div");
        dom.set_attribute(editor, "contenteditable", "");
        let span = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(editor, span);
        assert!(
            is_focusable(&dom, span),
            "an editing-host descendant inherits focusability"
        );
        // A <span> outside any editing host is not focusable.
        let plain = connect_el(&mut dom, doc, "span");
        assert!(!is_focusable(&dom, plain));
    }

    #[test]
    fn removal_clears_focus_bit() {
        // WHATWG HTML §2.1.4 removing steps step 2: removing the focused
        // element resets focus to the viewport — `EcsDom::fire_after_remove`
        // clears the `FOCUS` bit at removal (silent). So a detached holder
        // never carries a stale bit, and reattaching does not resurrect focus.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, el);
        set_focus_bit(&mut dom, Some(el));
        assert_eq!(current_focus(&dom, doc), Some(el));

        // Removal clears the bit at the chokepoint — no stale bit remains.
        let _ = dom.remove_child(doc, el);
        let still_set = dom
            .world()
            .get::<&ElementState>(el)
            .is_ok_and(|s| s.contains(ElementState::FOCUS));
        assert!(!still_set, "removal clears the FOCUS bit (no stale bit)");

        // Reattaching does not resurrect focus.
        let _ = dom.append_child(doc, el);
        assert_eq!(
            current_focus(&dom, doc),
            None,
            "reattach does not resurrect a removed element's focus"
        );
    }

    #[test]
    fn removal_clears_focus_on_descendant() {
        // The focused area may be a *descendant* of the removed node; the
        // inclusive-descendant snapshot covers it.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let container = dom.create_element("div", Attributes::default());
        let child = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(doc, container);
        let _ = dom.append_child(container, child);
        set_focus_bit(&mut dom, Some(child));
        assert_eq!(current_focus(&dom, doc), Some(child));

        // Removing the container disconnects `child`; its FOCUS bit clears.
        let _ = dom.remove_child(doc, container);
        let still_set = dom
            .world()
            .get::<&ElementState>(child)
            .is_ok_and(|s| s.contains(ElementState::FOCUS));
        assert!(
            !still_set,
            "removing an ancestor clears a focused descendant"
        );
    }

    #[test]
    fn move_clears_focus() {
        // A re-parent is a classic remove+insert (no `moveBefore` in elidex):
        // `append_child` of a focused element fires the implicit remove
        // (`detach_with_hook` → `fire_after_remove`), so focus is lost on a
        // move — matching browser behaviour (WHATWG HTML §2.1.4 runs on the
        // implicit remove).
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let a = connect_el(&mut dom, doc, "div");
        let b = connect_el(&mut dom, doc, "div");
        let el = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(a, el);
        set_focus_bit(&mut dom, Some(el));
        assert_eq!(current_focus(&dom, doc), Some(el));

        // Move `el` from `a` to `b` (still connected) — focus is cleared.
        let _ = dom.append_child(b, el);
        assert_eq!(
            current_focus(&dom, doc),
            None,
            "re-parenting a focused element loses focus (classic remove+insert)"
        );
    }

    #[test]
    fn tab_index_default_values() {
        let mut dom = EcsDom::new();
        let button = dom.create_element("button", Attributes::default());
        assert_eq!(tab_index_default_for(&dom, button), 0);

        let div = dom.create_element("div", Attributes::default());
        assert_eq!(tab_index_default_for(&dom, div), -1);

        let hidden_input = dom.create_element("input", Attributes::default());
        dom.set_attribute(hidden_input, "type", "hidden");
        assert_eq!(tab_index_default_for(&dom, hidden_input), -1);

        // `tabIndex` reflects the default tab order independent of disabled
        // state — a disabled <button> still defaults to 0 (focusability is
        // `is_focusable`'s concern, not the tab-index default).
        let disabled_button = dom.create_element("button", Attributes::default());
        dom.set_attribute(disabled_button, "disabled", "");
        assert_eq!(tab_index_default_for(&dom, disabled_button), 0);
    }
}
