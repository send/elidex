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
//!
//! ## Module layout
//!
//! - `predicate` — the §6.6.2/§6.6.3 focusable-area predicates
//!   ([`is_focusable`] / [`tab_index_default_for`] / [`parse_tab_index_value`]).
//! - `sot` — the focus source-of-truth: the [`ElementState::FOCUS`] bit's
//!   read ([`current_focus`]) / write ([`set_focus_bit`] / [`blur`]) models, the
//!   active-document membership test ([`is_in_document`]), and the asynchronous
//!   focusability fixup ([`reconcile_focus`]).
//! - `delegate` — §6.6.4 "get the focusable area" / "focus delegate" (the
//!   shadow-`delegatesFocus` retarget, PR-A1).
//! - `update_steps` — the canonical §6.6.4 transition ([`focusing_steps`] /
//!   [`unfocusing_steps`] + the [`FocusEventSink`] seam), PR-A2a.

use elidex_ecs::{EcsDom, ElementState, Entity};

mod delegate;
mod predicate;
mod sot;
mod update_steps;

pub use delegate::*;
pub use predicate::*;
pub use sot::*;
pub use update_steps::*;

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    /// A `<div tabindex="0">` — a real focusable area, so the asynchronous
    /// [`reconcile_focus`] fixup keeps it (it never GCs the bit). Tests that
    /// exercise persistence across reconcile use this rather than a bare `<div>`
    /// (non-focusable, which `reconcile_focus` would clear).
    fn focusable_div(dom: &mut EcsDom) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("tabindex".to_string(), "0".to_string());
        dom.create_element("div", attrs)
    }

    #[test]
    fn set_focus_bit_enforces_single_focus() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let a = focusable_div(&mut dom);
        let b = focusable_div(&mut dom);
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
    fn current_focus_keeps_holder_until_async_fixup() {
        // Spec: a same-turn mutation that makes the focused element non-focusable
        // (hidden / disabled) is an ASYNCHRONOUS fixup — WHATWG HTML "update the
        // rendering" step 17 runs it at the next rendering update, NOT
        // synchronously (only *removal*, §2.1.4, is synchronous). So
        // `document.activeElement` / `:focus` keep reporting the holder until
        // `reconcile_focus` runs. (Reverted the R4 derive-on-read `is_focusable`
        // filter, which hid it eagerly = non-spec + split `activeElement` from
        // the `:focus` selector — Codex S2 R7.)
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = focusable_div(&mut dom);
        let _ = dom.append_child(doc, el);
        set_focus_bit(&mut dom, Some(el));
        assert_eq!(current_focus(&dom, doc), Some(el));

        // Same-turn `hidden` lands, but `set_attribute` on a dispatcher-less
        // `EcsDom` does NOT run the reconciler — the pre-render window. The
        // holder is still the focused area (async fixup pending), consistent with
        // the raw bit the `:focus` selector reads.
        dom.set_attribute(el, "hidden", "");
        assert_eq!(
            current_focus(&dom, doc),
            Some(el),
            "stays the focused area until the async render-time fixup"
        );
        assert_eq!(raw_focus_holder(&dom), Some(el));

        // `reconcile_focus` IS that asynchronous fixup: it GCs the connected-but-
        // non-focusable bit, resetting the focused area to the viewport.
        reconcile_focus(&mut dom, doc);
        assert_eq!(
            current_focus(&dom, doc),
            None,
            "async fixup cleared the focus"
        );
        assert_eq!(raw_focus_holder(&dom), None);
    }

    #[test]
    fn blur_clears_the_lingering_bit_so_unhide_does_not_resurrect() {
        // Codex (S2 R6): `blur()` is an explicit WRITE on the focus SoT, so it
        // clears the raw FOCUS bit even when a same-turn mutation has made the
        // holder non-focusable but the async render fixup has not run yet —
        // otherwise `el.focus(); el.hidden = true; el.blur(); el.hidden = false`
        // leaves the bit lingering and the un-hide resurrects `activeElement`.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = focusable_div(&mut dom);
        let _ = dom.append_child(doc, el);
        set_focus_bit(&mut dom, Some(el));

        // Same-turn `hidden` lands; the bit lingers and `el` is still the focused
        // area (async fixup pending) — `blur()` must clear it regardless.
        dom.set_attribute(el, "hidden", "");
        assert_eq!(
            current_focus(&dom, doc),
            Some(el),
            "still focused (async fixup)"
        );
        assert_eq!(raw_focus_holder(&dom), Some(el), "raw bit lingers");

        blur(&mut dom, el);
        assert_eq!(raw_focus_holder(&dom), None, "blur cleared the raw bit");
        assert_eq!(current_focus(&dom, doc), None, "blurred");

        // Un-hiding in the same turn no longer resurrects focus.
        dom.remove_attribute(el, "hidden");
        assert_eq!(
            current_focus(&dom, doc),
            None,
            "blur honored across un-hide"
        );
    }

    #[test]
    fn blur_of_a_non_holder_is_a_noop() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let a = focusable_div(&mut dom);
        let b = focusable_div(&mut dom);
        let _ = dom.append_child(doc, a);
        let _ = dom.append_child(doc, b);
        set_focus_bit(&mut dom, Some(a));

        // Blurring an element that is not the focus holder leaves focus intact.
        blur(&mut dom, b);
        assert_eq!(raw_focus_holder(&dom), Some(a));
        assert_eq!(current_focus(&dom, doc), Some(a));
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
    fn is_focusable_editing_host_not_inherited() {
        // Regression (Codex S2 final-pass #3, correcting R1-F3/R4-F3): only an
        // *editing host* is a focusable area, NOT its merely-editable descendants.
        // §6.6.3 lists editing hosts as UA-focusable; §6.8.4 defines an editing
        // host as the element with its OWN `contenteditable` in the true/
        // plaintext-only state — the inherited `isContentEditable` algorithm
        // (true for editable descendants too) is the wrong axis for focusability.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let editor = connect_el(&mut dom, doc, "div");
        dom.set_attribute(editor, "contenteditable", "");
        assert!(
            is_focusable(&dom, editor),
            "an editing host (own contenteditable) is focusable"
        );
        // A plain descendant inherits editability but is not itself an editing
        // host, so it is not a focusable area. (A click/`focus()` retargets to
        // the host via "get the focusable area", §6.6.4 — slot
        // `#11-focusing-steps-fallback-target`, the same path as any other
        // non-focusable target, not a contenteditable special case.)
        let span = dom.create_element("span", Attributes::default());
        let _ = dom.append_child(editor, span);
        assert!(
            !is_focusable(&dom, span),
            "a merely-editable descendant of an editing host is not focusable"
        );
        // `contenteditable="false"` is not an editing host.
        let off = connect_el(&mut dom, doc, "div");
        dom.set_attribute(off, "contenteditable", "false");
        assert!(
            !is_focusable(&dom, off),
            "contenteditable=false is not an editing host"
        );
        // A <span> with no editing context at all.
        let plain = connect_el(&mut dom, doc, "span");
        assert!(!is_focusable(&dom, plain));
    }

    #[test]
    fn is_focusable_editing_host_case_insensitive_and_plaintext_only() {
        // The own-`contenteditable` editing-host check (via
        // `tab_index_default_for`'s generic arm) matches the true state
        // case-insensitively and the plaintext-only state (WHATWG HTML §6.8.1
        // states) — so `TRUE` / `plaintext-only` editing hosts are focusable,
        // while their descendants (which merely inherit editability) are not.
        for value in ["TRUE", "plaintext-only", "PLAINTEXT-ONLY"] {
            let mut dom = EcsDom::new();
            let doc = dom.create_document_root();
            let editor = connect_el(&mut dom, doc, "div");
            dom.set_attribute(editor, "contenteditable", value);
            assert!(
                is_focusable(&dom, editor),
                "an editing host is focusable for contenteditable={value:?}"
            );
            let span = dom.create_element("span", Attributes::default());
            let _ = dom.append_child(editor, span);
            assert!(
                !is_focusable(&dom, span),
                "an editable descendant is not focusable for contenteditable={value:?}"
            );
        }
    }

    #[test]
    fn tab_index_default_is_html_namespace_only() {
        // §6.6.3 UA-determined focus defaults are HTML-only (Codex S2): a foreign
        // (SVG/MathML) element whose local name matches an HTML control gets no
        // per-element default, so it is not a focusable area — but an explicit
        // `tabindex` still makes it focusable cross-namespace (the attribute is
        // global). Mirrors the repo's namespace gating on form-control state.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        // An SVG-namespaced <button> look-alike: not an HTML control.
        let svg_button = dom.create_element_ns(
            "button",
            elidex_ecs::Namespace::Svg,
            Attributes::default(),
            None,
        );
        let _ = dom.append_child(doc, svg_button);
        assert_eq!(
            tab_index_default_for(&dom, svg_button),
            -1,
            "a foreign-namespace control look-alike has no HTML focus default"
        );
        assert!(
            !is_focusable(&dom, svg_button),
            "an SVG <button> is not a focusable area by default"
        );
        // An HTML <button> with the same local name IS focusable by default.
        let html_button = connect_el(&mut dom, doc, "button");
        assert!(
            is_focusable(&dom, html_button),
            "an HTML <button> is focusable by default"
        );
        // An explicit tabindex still grants focusability cross-namespace.
        dom.set_attribute(svg_button, "tabindex", "0");
        assert!(
            is_focusable(&dom, svg_button),
            "an explicit tabindex makes a foreign element focusable"
        );
        // An SVG <a href> also gets no HTML link default (SVG focus is a
        // separate, unmodelled concern).
        let svg_a =
            dom.create_element_ns("a", elidex_ecs::Namespace::Svg, Attributes::default(), None);
        let _ = dom.append_child(doc, svg_a);
        dom.set_attribute(svg_a, "href", "#x");
        assert_eq!(
            tab_index_default_for(&dom, svg_a),
            -1,
            "an SVG <a href> has no HTML link focus default"
        );
    }

    #[test]
    fn is_focusable_foreign_lookalike_ignores_html_exclusions() {
        // The HTML form-control exclusions (`is_hidden_input`,
        // `is_actually_disabled`) are HTML-namespace only (Codex S2): a foreign
        // element merely *named* like a control must not be excluded by them, so
        // an explicit `tabindex` still grants it focusability. The HTML versions
        // stay excluded (a tabindex can't grant focusability §6.6.2 withholds).
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        // SVG <input type=hidden tabindex=0>: the hidden-input exclusion is
        // HTML-only, so the explicit tabindex grants focusability.
        let svg_input = dom.create_element_ns(
            "input",
            elidex_ecs::Namespace::Svg,
            Attributes::default(),
            None,
        );
        let _ = dom.append_child(doc, svg_input);
        dom.set_attribute(svg_input, "type", "hidden");
        dom.set_attribute(svg_input, "tabindex", "0");
        assert!(
            is_focusable(&dom, svg_input),
            "an SVG <input type=hidden tabindex=0> is focusable via explicit tabindex"
        );
        // The HTML <input type=hidden tabindex=0> stays excluded.
        let html_input = connect_el(&mut dom, doc, "input");
        dom.set_attribute(html_input, "type", "hidden");
        dom.set_attribute(html_input, "tabindex", "0");
        assert!(
            !is_focusable(&dom, html_input),
            "an HTML hidden input is not focusable even with a tabindex"
        );

        // SVG <button disabled tabindex=0>: the disabled exclusion is HTML-only.
        let svg_button = dom.create_element_ns(
            "button",
            elidex_ecs::Namespace::Svg,
            Attributes::default(),
            None,
        );
        let _ = dom.append_child(doc, svg_button);
        dom.set_attribute(svg_button, "disabled", "");
        dom.set_attribute(svg_button, "tabindex", "0");
        assert!(
            is_focusable(&dom, svg_button),
            "an SVG <button disabled tabindex=0> is focusable via explicit tabindex"
        );
        // The HTML <button disabled tabindex=0> stays excluded.
        let html_button = connect_el(&mut dom, doc, "button");
        dom.set_attribute(html_button, "disabled", "");
        dom.set_attribute(html_button, "tabindex", "0");
        assert!(
            !is_focusable(&dom, html_button),
            "an HTML disabled button is not focusable even with a tabindex"
        );
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
        let el = focusable_div(&mut dom);
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

    #[test]
    fn first_summary_of_details_is_focusable() {
        // Codex (S2 R10): the first `<summary>` child of a `<details>` is a
        // UA-determined focusable area (§6.6.2) — the disclosure widget's
        // built-in control — so it gets a default tabIndex of 0 with no author
        // `tabindex`. A second summary, or a summary outside a details, is not.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let details = connect_el(&mut dom, doc, "details");
        let summary = dom.create_element("summary", Attributes::default());
        let _ = dom.append_child(details, summary);
        assert_eq!(
            tab_index_default_for(&dom, summary),
            0,
            "first summary focusable"
        );
        assert!(is_focusable(&dom, summary));

        // A second summary in the same details is NOT the UA control.
        let second = dom.create_element("summary", Attributes::default());
        let _ = dom.append_child(details, second);
        assert_eq!(
            tab_index_default_for(&dom, second),
            -1,
            "second summary not focusable"
        );

        // A summary outside any details is not UA-focusable.
        let orphan = connect_el(&mut dom, doc, "summary");
        assert_eq!(
            tab_index_default_for(&dom, orphan),
            -1,
            "summary sans details not focusable"
        );

        // An explicit author `tabindex` still grants focusability (criterion 1
        // first arm) independent of the UA default.
        dom.set_attribute(orphan, "tabindex", "0");
        assert!(is_focusable(&dom, orphan));
    }

    #[test]
    fn summary_details_disclosure_widget_is_html_namespace_only() {
        // Codex S2: the disclosure-widget focus default requires an HTML
        // <summary> that is the first HTML <summary> child of an HTML <details>.
        // Foreign (SVG/MathML) look-alikes don't count, and a foreign <summary>
        // sibling must not displace the first HTML <summary>.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        // HTML <summary> under a FOREIGN <details> → not the widget.
        let svg_details = dom.create_element_ns(
            "details",
            elidex_ecs::Namespace::Svg,
            Attributes::default(),
            None,
        );
        let _ = dom.append_child(doc, svg_details);
        let html_summary = dom.create_element("summary", Attributes::default());
        let _ = dom.append_child(svg_details, html_summary);
        assert_eq!(
            tab_index_default_for(&dom, html_summary),
            -1,
            "an HTML <summary> under a foreign <details> is not the disclosure widget"
        );

        // FOREIGN <summary> under an HTML <details> → not the widget (foreign self).
        let html_details = connect_el(&mut dom, doc, "details");
        let svg_summary = dom.create_element_ns(
            "summary",
            elidex_ecs::Namespace::Svg,
            Attributes::default(),
            None,
        );
        let _ = dom.append_child(html_details, svg_summary);
        assert_eq!(
            tab_index_default_for(&dom, svg_summary),
            -1,
            "a foreign <summary> is not the disclosure widget"
        );

        // A foreign <summary> sibling preceding the HTML <summary> must not
        // displace it: the HTML summary is still the first HTML summary child.
        let real_summary = dom.create_element("summary", Attributes::default());
        let _ = dom.append_child(html_details, real_summary);
        assert_eq!(
            tab_index_default_for(&dom, real_summary),
            0,
            "the first HTML <summary> is the widget even behind a foreign <summary> sibling"
        );
    }

    #[test]
    fn parse_tab_index_value_follows_rules_for_parsing_integers() {
        // §2.3.4.1: skip leading whitespace, optional sign, collect the leading
        // ASCII-digit run, ignore trailing characters; `None` only when no digit
        // follows the optional sign.
        assert_eq!(parse_tab_index_value("0"), Some(0));
        assert_eq!(parse_tab_index_value("-1"), Some(-1));
        assert_eq!(parse_tab_index_value("+5"), Some(5));
        assert_eq!(parse_tab_index_value("  3  "), Some(3));
        // Trailing non-digits are ignored (the prior `trim().parse::<i32>()`
        // wrongly rejected these).
        assert_eq!(parse_tab_index_value("1foo"), Some(1));
        assert_eq!(parse_tab_index_value("-3px"), Some(-3));
        // No leading digit ⇒ null tabindex.
        assert_eq!(parse_tab_index_value("foo"), None);
        assert_eq!(parse_tab_index_value(""), None);
        assert_eq!(parse_tab_index_value("-"), None);
        assert_eq!(parse_tab_index_value("   "), None);
    }

    #[test]
    fn tabindex_with_trailing_junk_grants_focusability() {
        // Regression (Codex R12 F2): `<div tabindex="1foo">` parses to 1 per
        // §2.3.4.1, so the element IS focusable — the old `trim().parse::<i32>()`
        // returned `None` and wrongly skipped it as non-focusable.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let div = connect_el(&mut dom, doc, "div");
        dom.set_attribute(div, "tabindex", "1foo");
        assert!(
            is_focusable(&dom, div),
            "tabindex=\"1foo\" parses to 1 (§2.3.4.1) ⇒ focusable"
        );
    }

    #[test]
    fn reconcile_focus_clears_when_focused_element_stops_being_focusable() {
        // Regression (Codex R12 F1): §6.6.2 is a focus-time gate; a focused
        // element that LATER becomes non-focusable keeps the FOCUS bit until
        // reconciled. `reconcile_focus` restores `current_focus ⟹ is_focusable`.
        // Each arm: focus a focusable element, mutate it non-focusable, reconcile.
        // hidden on the element itself:
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = connect_el(&mut dom, doc, "button");
        set_focus_bit(&mut dom, Some(el));
        assert_eq!(current_focus(&dom, doc), Some(el));
        dom.set_attribute(el, "hidden", "");
        reconcile_focus(&mut dom, doc);
        assert_eq!(current_focus(&dom, doc), None, "hidden clears focus");

        // hidden on an ancestor:
        let section = connect_el(&mut dom, doc, "section");
        let inner = dom.create_element("button", Attributes::default());
        let _ = dom.append_child(section, inner);
        set_focus_bit(&mut dom, Some(inner));
        dom.set_attribute(section, "hidden", "");
        reconcile_focus(&mut dom, doc);
        assert_eq!(
            current_focus(&dom, doc),
            None,
            "ancestor hidden clears focus"
        );

        // disabled lands:
        let btn = connect_el(&mut dom, doc, "button");
        set_focus_bit(&mut dom, Some(btn));
        dom.set_attribute(btn, "disabled", "");
        reconcile_focus(&mut dom, doc);
        assert_eq!(current_focus(&dom, doc), None, "disabled clears focus");

        // <input type> flips to hidden:
        let input = connect_el(&mut dom, doc, "input");
        set_focus_bit(&mut dom, Some(input));
        dom.set_attribute(input, "type", "hidden");
        reconcile_focus(&mut dom, doc);
        assert_eq!(current_focus(&dom, doc), None, "type=hidden clears focus");

        // <a> loses its href (focusable only via the href-gated Link default):
        let a = connect_el(&mut dom, doc, "a");
        dom.set_attribute(a, "href", "x");
        set_focus_bit(&mut dom, Some(a));
        assert_eq!(current_focus(&dom, doc), Some(a));
        dom.remove_attribute(a, "href");
        reconcile_focus(&mut dom, doc);
        assert_eq!(current_focus(&dom, doc), None, "losing href clears focus");
    }

    #[test]
    fn reconcile_focus_keeps_a_still_focusable_element() {
        // The per-re-render reconcile must not blur a live focus: an unrelated
        // attribute change leaves the element focusable, so focus is retained.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = connect_el(&mut dom, doc, "button");
        set_focus_bit(&mut dom, Some(el));
        dom.set_attribute(el, "title", "hi");
        reconcile_focus(&mut dom, doc);
        assert_eq!(
            current_focus(&dom, doc),
            Some(el),
            "a still-focusable element keeps focus across reconcile"
        );
    }

    #[test]
    fn removal_clears_focus_in_wide_dom_past_index_cap() {
        // Regression (Codex R12 F3): the removal focus-clear rode inside
        // `fire_after_remove`, which `remove_child` calls only when
        // `index_in_parent` returns `Some`. `index_in_parent` returns `None` past
        // `MAX_ANCESTOR_DEPTH` previous siblings (a wide-but-valid DOM), so the
        // §2.1.4 reset was skipped and reattach resurrected `activeElement`. The
        // clear is now run via the `else` fallback independent of the index.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let parent = connect_el(&mut dom, doc, "div");
        // MAX_ANCESTOR_DEPTH + 1 leading siblings so the focused child's
        // `index_in_parent` walk exceeds the cap and returns `None`.
        for _ in 0..=elidex_ecs::MAX_ANCESTOR_DEPTH {
            let sib = dom.create_element("span", Attributes::default());
            let _ = dom.append_child(parent, sib);
        }
        let focused = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(parent, focused);
        assert_eq!(
            dom.index_in_parent(focused),
            None,
            "the focused child is past the index cap (precondition for the bug)"
        );
        set_focus_bit(&mut dom, Some(focused));
        assert_eq!(current_focus(&dom, doc), Some(focused));

        let _ = dom.remove_child(parent, focused);
        let still_set = dom
            .world()
            .get::<&ElementState>(focused)
            .is_ok_and(|s| s.contains(ElementState::FOCUS));
        assert!(
            !still_set,
            "wide-DOM removal clears the FOCUS bit even when index_in_parent is None"
        );
    }

    // ---- §6.6.4 get the focusable area / focus delegate (PR-A1) ----

    /// A `<div>` host (a valid shadow host) connected to `doc`, with an open
    /// shadow root whose `delegatesFocus` is `delegates`. Returns
    /// `(host, shadow_root)`.
    fn shadow_host(dom: &mut EcsDom, doc: Entity, delegates: bool) -> (Entity, Entity) {
        let host = connect_el(dom, doc, "div");
        let sr = dom
            .attach_shadow_with_init(
                host,
                elidex_ecs::ShadowInit {
                    mode: elidex_ecs::ShadowRootMode::Open,
                    delegates_focus: delegates,
                    ..Default::default()
                },
            )
            .expect("attach_shadow on <div> succeeds");
        (host, sr)
    }

    /// Append a `<div tabindex="0">` (a focusable area) under `parent`.
    fn focusable_child(dom: &mut EcsDom, parent: Entity) -> Entity {
        let child = focusable_div(dom);
        let _ = dom.append_child(parent, child);
        child
    }

    #[test]
    fn get_focusable_area_delegates_to_first_shadow_focusable() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let (host, sr) = shadow_host(&mut dom, doc, true);
        let delegate = focusable_child(&mut dom, sr);
        assert_eq!(
            get_the_focusable_area(&dom, host, FocusTrigger::Click),
            Some(delegate),
            "a delegatesFocus host retargets to the first focusable in its shadow tree"
        );
    }

    #[test]
    fn get_focusable_area_none_when_delegates_focus_false() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let (host, sr) = shadow_host(&mut dom, doc, false);
        let _ = focusable_child(&mut dom, sr);
        assert_eq!(
            get_the_focusable_area(&dom, host, FocusTrigger::Click),
            None,
            "delegatesFocus=false → branch 6 skipped → branch 7 (null)"
        );
    }

    #[test]
    fn get_focusable_area_none_for_plain_element() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = connect_el(&mut dom, doc, "div");
        assert_eq!(
            get_the_focusable_area(&dom, el, FocusTrigger::Other),
            None,
            "a non-host element is not a get-the-focusable-area target → null"
        );
    }

    #[test]
    fn get_focusable_area_keeps_focus_already_inside_host() {
        // §6.6.4 branch 6.2: if focus is already inside the host's shadow tree,
        // keep it rather than re-delegating to the first focusable.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let (host, sr) = shadow_host(&mut dom, doc, true);
        let first = focusable_child(&mut dom, sr);
        let second = focusable_child(&mut dom, sr);
        set_focus_bit(&mut dom, Some(second));
        assert_eq!(
            get_the_focusable_area(&dom, host, FocusTrigger::Click),
            Some(second),
            "host is a shadow-including inclusive ancestor of the focused `second` → keep it, not `first`"
        );
        let _ = first;
    }

    #[test]
    fn autofocus_delegate_wins_over_tree_order() {
        // §6.6.4 autofocus delegate: an `autofocus` descendant takes precedence
        // over the tree-order-first focusable.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let (host, sr) = shadow_host(&mut dom, doc, true);
        let _first = focusable_child(&mut dom, sr); // focusable, no autofocus, earlier in tree order
        let mut attrs = Attributes::default();
        attrs.set("tabindex".to_string(), "0".to_string());
        attrs.set("autofocus".to_string(), String::new());
        let autofocus = dom.create_element("div", attrs);
        let _ = dom.append_child(sr, autofocus);
        assert_eq!(
            get_the_focusable_area(&dom, host, FocusTrigger::Click),
            Some(autofocus),
            "the autofocus descendant wins over the tree-order-first focusable"
        );
    }

    #[test]
    fn focus_delegate_recurses_into_nested_shadow_host() {
        // §6.6.4 focus-delegate step 6.4: a nested delegatesFocus shadow host
        // among the descendants delegates recursively.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let (outer, outer_sr) = shadow_host(&mut dom, doc, true);
        // A nested <div> host inside the outer shadow tree, itself delegatesFocus.
        let inner = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(outer_sr, inner);
        let inner_sr = dom
            .attach_shadow_with_init(
                inner,
                elidex_ecs::ShadowInit {
                    mode: elidex_ecs::ShadowRootMode::Open,
                    delegates_focus: true,
                    ..Default::default()
                },
            )
            .expect("attach_shadow on nested <div>");
        let deep = focusable_child(&mut dom, inner_sr);
        assert_eq!(
            get_the_focusable_area(&dom, outer, FocusTrigger::Click),
            Some(deep),
            "recurses through the nested delegatesFocus host to its delegate"
        );
    }

    #[test]
    fn focus_delegate_recurses_into_nested_focusable_shadow_host() {
        // §6.6.2 criterion 2 + §6.6.4 focus-delegate step 6.3→6.4: a nested
        // delegatesFocus host that ALSO carries `tabindex` (so `is_focusable`
        // returns true) is still NOT a focusable area — it must delegate into its
        // shadow tree, not be returned as itself. Guards the C2-aware
        // `is_focusable_area` gate (a `is_focusable`-only gate returns the host).
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let (outer, outer_sr) = shadow_host(&mut dom, doc, true);
        // Nested host carrying tabindex="0" → intrinsically `is_focusable`, but a
        // delegatesFocus shadow host, so not a §6.6.2 focusable area.
        let mut inner_attrs = Attributes::default();
        inner_attrs.set("tabindex".to_string(), "0".to_string());
        let inner = dom.create_element("div", inner_attrs);
        let _ = dom.append_child(outer_sr, inner);
        let inner_sr = dom
            .attach_shadow_with_init(
                inner,
                elidex_ecs::ShadowInit {
                    mode: elidex_ecs::ShadowRootMode::Open,
                    delegates_focus: true,
                    ..Default::default()
                },
            )
            .expect("attach_shadow on nested <div tabindex>");
        let deep = focusable_child(&mut dom, inner_sr);
        assert!(
            is_focusable(&dom, inner),
            "the nested host is intrinsically focusable (tabindex) — the case that masks the bug"
        );
        assert_eq!(
            get_the_focusable_area(&dom, outer, FocusTrigger::Click),
            Some(deep),
            "a tabindex-bearing nested delegatesFocus host still delegates to its inner area (C2), not itself"
        );
    }

    #[test]
    fn focus_trigger_default_is_other() {
        assert_eq!(FocusTrigger::default(), FocusTrigger::Other);
        assert_ne!(FocusTrigger::Click, FocusTrigger::Other);
    }
}
