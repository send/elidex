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

/// Whether `entity` is a focusable area (WHATWG HTML §6.6.2 "Data model").
///
/// §6.6.2 gives five criteria an Element must *all* meet to be a focusable area;
/// this is the engine-independent, **attribute-based** evaluation of them (this
/// crate has no `elidex-form` / computed-style dependency). Per-criterion
/// coverage:
///
/// **C1 — tabindex value non-null, OR UA-determined focusable** — *enforced*: a
/// `tabindex` that parses as a valid integer (§6.6.3 "rules for parsing
/// integers", via [`parse_tab_index_value`] — shared with the `tabIndex` IDL
/// getter), an editing host ([`crate::element::is_content_editable`] — the
/// `isContentEditable` algorithm, so case-insensitive `true`/`plaintext-only` +
/// ancestor inheritance), or a non-negative per-element default
/// ([`tab_index_default_for`]: `<a href>` / button / input(non-hidden) / select
/// / textarea / iframe / object / embed). UA-list residue not yet modelled:
/// `summary` as a `details`' first child; `draggable`.
///
/// **C2 — not a shadow host, or shadow root delegates-focus = false** — *not
/// enforced here*: a delegates-focus host is not itself the focusable area (its
/// first focusable shadow descendant is), which needs the shadow focus
/// *delegation* algorithm, not a bare exclusion (excluding it would wrongly make
/// `host.focus()` a no-op). Slot `#11-shadow-focus-delegation`.
///
/// **C3 — not actually disabled** — *enforced* (`is_actually_disabled`: a direct
/// `disabled` on a disablable element). Fieldset-inherited `disabled` lives in
/// the form subsystem (the shell overlays `FormControlState`); slot
/// `#11-focusable-area-fieldset-inherited-disabled` brings it to this path.
///
/// **C4 — not inert** — `inert` is not modelled by the engine, so there is
/// nothing to exclude (no gap today; revisit if `inert` lands).
///
/// **C5 — being rendered** — *partially enforced* from attributes. Three
/// attribute-reachable non-rendered cases are excluded: **disconnected**
/// ([`EcsDom::is_connected`] — a disconnected element is not rendered, so
/// `createElement('input').focus()` is a no-op); **`<input type=hidden>`**
/// (`is_hidden_input` — never rendered, even with a `tabindex`; §6.6.3 notes a
/// tabindex cannot grant focusability §6.6.2 withholds, and the shell rejects it
/// via `FormControlKind::Hidden` — the two focus writers must agree); and the
/// global **`hidden` attribute** (`is_in_hidden_subtree`, self or ancestor —
/// §6.1: both the Hidden and Hidden-Until-Found states are "will not be
/// rendered", and a `hidden` element hides its subtree). The CSS residue
/// (`display:none` / `visibility:hidden`, and a `[hidden] { display: block }`
/// author override), which needs computed style, is slot
/// `#11-focusable-area-being-rendered`. (Elements never rendered by category —
/// `<head>` metadata content — are a niche residual, not yet excluded.)
#[must_use]
pub fn is_focusable(dom: &EcsDom, entity: Entity) -> bool {
    // §6.6.2 criterion 5 (being rendered) — the attribute-reachable slice. Gate
    // these BEFORE the criterion-1 tabindex short-circuit, else a `tabindex`
    // would wrongly grant focusability to a non-rendered element.
    if !dom.is_connected(entity) {
        return false;
    }
    if is_hidden_input(dom, entity) {
        return false;
    }
    if is_in_hidden_subtree(dom, entity) {
        return false;
    }
    // §6.6.2 criterion 3 (not actually disabled).
    if is_actually_disabled(dom, entity) {
        return false;
    }
    // §6.6.2 criterion 1 (tabindex value non-null, or UA-determined focusable):
    // the `tabindex` attribute participates only when it parses as a valid
    // integer (§6.6.3 "rules for parsing integers"); an invalid value
    // (`tabindex="foo"`) yields a null tabindex and falls through to the
    // per-element default — matching the `tabIndex` IDL getter.
    dom.with_attribute(entity, "tabindex", |v| {
        v.and_then(parse_tab_index_value).is_some()
    }) || crate::element::is_content_editable(dom, entity)
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

/// `<input type=hidden>` — a hidden input is never "being rendered" (WHATWG HTML
/// §6.6.2 criterion 5), so it is not a focusable area even with a `tabindex`
/// (§6.6.3: a tabindex cannot grant focusability §6.6.2 withholds). The
/// attribute-based mirror of the shell's `FormControlKind::Hidden` rejection, so
/// the VM `focus()` and shell UA-input writers agree.
fn is_hidden_input(dom: &EcsDom, entity: Entity) -> bool {
    dom.with_tag_name(entity, |t| {
        t.is_some_and(|s| s.eq_ignore_ascii_case("input"))
    }) && dom.with_attribute(entity, "type", |v| {
        v.is_some_and(|s| s.eq_ignore_ascii_case("hidden"))
    })
}

/// Whether `entity` is in a `hidden`-attribute subtree — itself or a
/// shadow-including ancestor carries the global `hidden` content attribute
/// (WHATWG HTML §6.1). Both the *Hidden* and *Hidden Until Found* states are
/// "will not be rendered", and a hidden element hides its whole subtree, so an
/// element anywhere under a `hidden` node is not "being rendered" (§6.6.2
/// criterion 5) and hence not a focusable area — `<button hidden>` and a
/// `<button>` inside `<div hidden>` alike. Walks via [`EcsDom::get_parent`]
/// (shadow-inclusive, so a `hidden` host hides its shadow tree), bounded by
/// [`MAX_ANCESTOR_DEPTH`]. Attribute-based: a `[hidden] { display: block }`
/// author override (computed-style residue, slot
/// `#11-focusable-area-being-rendered`) is not reflected here.
///
/// [`MAX_ANCESTOR_DEPTH`]: elidex_ecs::MAX_ANCESTOR_DEPTH
fn is_in_hidden_subtree(dom: &EcsDom, entity: Entity) -> bool {
    let mut cur = Some(entity);
    let mut depth = 0;
    while let Some(c) = cur {
        if dom.has_attribute(c, "hidden") {
            return true;
        }
        if depth >= elidex_ecs::MAX_ANCESTOR_DEPTH {
            return false;
        }
        cur = dom.get_parent(c);
        depth += 1;
    }
    false
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
    // The connectedness walk to `document` is the defensive guard (the bit is
    // connected by construction) — and doubles as the document scoping that
    // keeps this read reporting only the bound document's focused area, never a
    // holder in some other document tree sharing this world (e.g. a
    // `document.cloneNode()` subtree).
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
    fn is_focusable_inherits_contenteditable_uppercase_and_plaintext_only() {
        // Regression (Codex R4 F3): the lower-level `EcsDom::is_contenteditable`
        // matched only exact lowercase `"true"`/`""`, so an editing host with
        // `contenteditable="TRUE"` or `"plaintext-only"` left its descendants
        // non-focusable — diverging from the canonical `is_content_editable`
        // (the `isContentEditable` algorithm) that this predicate now shares.
        for value in ["TRUE", "plaintext-only", "PLAINTEXT-ONLY"] {
            let mut dom = EcsDom::new();
            let doc = dom.create_document_root();
            let editor = connect_el(&mut dom, doc, "div");
            dom.set_attribute(editor, "contenteditable", value);
            let span = dom.create_element("span", Attributes::default());
            let _ = dom.append_child(editor, span);
            assert!(
                is_focusable(&dom, span),
                "an editing-host descendant inherits focusability for contenteditable={value:?}"
            );
        }
    }

    #[test]
    fn is_focusable_excludes_hidden_input_even_with_tabindex() {
        // Regression (Codex R5 F2): §6.6.2 criterion 5 (being rendered) — an
        // `<input type=hidden>` is never rendered, so it is not a focusable area
        // even with a `tabindex` (§6.6.3: a tabindex cannot grant focusability
        // §6.6.2 withholds). The VM `focus()` path must agree with the shell's
        // `FormControlKind::Hidden` rejection, so the tabindex short-circuit must
        // not bypass the hidden-input gate.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let hidden = connect_el(&mut dom, doc, "input");
        dom.set_attribute(hidden, "type", "hidden");
        assert!(
            !is_focusable(&dom, hidden),
            "a hidden input is not focusable"
        );
        dom.set_attribute(hidden, "tabindex", "0");
        assert!(
            !is_focusable(&dom, hidden),
            "a tabindex does not make a hidden input focusable"
        );
        // A non-hidden input with the same tabindex IS focusable — the gate is
        // specific to the hidden type, not all inputs.
        let text = connect_el(&mut dom, doc, "input");
        dom.set_attribute(text, "tabindex", "0");
        assert!(is_focusable(&dom, text), "a non-hidden input is focusable");
    }

    #[test]
    fn is_focusable_excludes_hidden_attribute_subtree() {
        // Regression (Codex R6 F2): §6.6.2 criterion 5 (being rendered) — the
        // global `hidden` attribute (§6.1) makes content non-rendered, so an
        // element that is itself hidden, OR inside a hidden subtree, is not a
        // focusable area even with a tabindex / intrinsic default.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        // Self-hidden: `<button hidden>` (intrinsic default tabindex 0).
        let btn = connect_el(&mut dom, doc, "button");
        assert!(is_focusable(&dom, btn), "a connected <button> is focusable");
        dom.set_attribute(btn, "hidden", "");
        assert!(!is_focusable(&dom, btn), "<button hidden> is not focusable");

        // An explicit tabindex does not override `hidden`.
        let div = connect_el(&mut dom, doc, "div");
        dom.set_attribute(div, "tabindex", "0");
        dom.set_attribute(div, "hidden", "hidden");
        assert!(
            !is_focusable(&dom, div),
            "tabindex does not override hidden"
        );

        // Ancestor-hidden: a `<button>` inside `<section hidden>`.
        let section = connect_el(&mut dom, doc, "section");
        dom.set_attribute(section, "hidden", "");
        let inner = dom.create_element("button", Attributes::default());
        let _ = dom.append_child(section, inner);
        assert!(
            !is_focusable(&dom, inner),
            "a control inside a hidden subtree is not focusable"
        );

        // `hidden="until-found"` is also "will not be rendered" (§6.1) ⇒ excluded.
        let uf = connect_el(&mut dom, doc, "button");
        dom.set_attribute(uf, "hidden", "until-found");
        assert!(
            !is_focusable(&dom, uf),
            "hidden=until-found is not focusable"
        );
    }

    #[test]
    fn is_in_document_scopes_to_the_named_document() {
        // Two documents can share one world (e.g. `document.cloneNode()`):
        // membership is scoped to the *named* document, not "any document root"
        // (which `is_connected` reports). A clone-document descendant is
        // connected yet NOT in the bound document — so a focus *writer* gated on
        // `is_in_document` will not let it clobber the bound document's holder.
        let mut dom = EcsDom::new();
        let bound = dom.create_document_root();
        let live = connect_el(&mut dom, bound, "input");
        assert!(is_in_document(&dom, live, bound));

        // A second Document-rooted tree standing in for a cloned/non-bound doc.
        let clone = dom.create_document_root();
        let clone_child = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(clone, clone_child);
        assert!(
            dom.is_connected(clone_child),
            "a clone-document descendant is connected (its root is a Document)"
        );
        assert!(
            !is_in_document(&dom, clone_child, bound),
            "but it is NOT in the bound document"
        );
        assert!(
            is_in_document(&dom, clone_child, clone),
            "it IS in its own (clone) document"
        );
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
