//! §6.6.4 canonical focus transition — the **focusing steps**
//! (`#focusing-steps`), **focus update steps** (`#focus-update-steps`) and
//! **unfocusing steps** (`#unfocusing-steps`) of WHATWG HTML §6.6.4, driven by
//! every focus entry point through an injected [`FocusEventSink`].
//!
//! This is the single canonical replacement for the previously hand-rolled
//! per-writer transitions (the shell `set_focus`/`blur_current` + the VM
//! `HTMLElement.focus()`/`blur()`). The transition logic — who designates the
//! [`super::set_focus_bit`] source-of-truth, in what order, and the reentrancy
//! discipline — is engine-independent and lives here; the engine-bound
//! side-effects (3-phase event dispatch + the `elidex-form` change-on-blur
//! snapshot, neither reachable from this crate) are injected via the sink.
//!
//! ## Single-element model
//!
//! elidex designates a single focusable element per document (the canonical
//! [`super::ElementState::FOCUS`] bit). The spec's multi-entry **focus chain**
//! and its step-1 common-ancestor pop are a cross-frame concern (a document's
//! focus chain spans nested navigables); A2a specialises the algorithm to the
//! chain *leaf* — `old`/`new` focusable elements — and defers the chain + pop to
//! PR-A3. For an element-to-element move within a single document this is
//! exact: the shell fires `blur`/`focus` only at the elements (never the
//! `Document`/viewport), which is what an `[element, Document]` chain with the
//! shared-`Document` tail popped produces. A move to the **viewport** (the
//! step-17 [`focus_fixup_steps`], [`unfocusing_steps`] step 8) is not: the new
//! chain's Document entry keeps its place, and focus update step 4 gives it a
//! `focus` event at its Window, which this model does not fire (see
//! [`focus_fixup_steps`]).
//!
//! ## Reentrancy (reentrant-wins)
//!
//! A `change`/`blur` listener fired mid-transition may call `focus()`/`blur()`,
//! writing the same FOCUS bit. The losing-side clear runs first, then step 4
//! re-reads [`super::current_focus`]: if a reentrant focus already designated a
//! new area, it **wins** (the outer transition does not clobber it). "Designate
//! the new area last" alone is insufficient — it defends the old clear-then-set
//! race but not the outer-clobbers-inner case.

use elidex_ecs::{EcsDom, Entity};

use super::{
    current_focus, get_the_focusable_area, is_focusable, is_focusable_area, is_in_document,
    set_focus_bit, FocusTrigger,
};

/// The §6.6.4 focus events the canonical transition fires through the sink. Only
/// `blur`/`focus` are §6.6.4 events; the bubbling `focusin` (UI Events §3.3.4.3)
/// / `focusout` (§3.3.4.4) are the shell sink's engine-bound derivation, ordered
/// per UI Events §3.3.2 "Focus Event Order".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FocusEventKind {
    /// §6.6.4 focus-update step 2.4 — the element losing focus.
    Blur,
    /// §6.6.4 focus-update step 4.4 — the element gaining focus.
    Focus,
}

/// The engine-bound seam of the canonical focus transition: DOM access plus the
/// engine-bound side-effects the engine-independent steps cannot perform
/// themselves — 3-phase event dispatch and the `elidex-form` change-on-blur
/// snapshot (this crate has no `elidex-form` dependency).
///
/// The shell impl (live) wraps the content pipeline. The VM impl (PR-A2c) wraps
/// the host context and may no-op [`fire_focus_event`](Self::fire_focus_event) /
/// [`commit_change_on_blur`](Self::commit_change_on_blur) until the synthetic
/// event-dispatch primitive lands (`#11-vm-host-synthetic-dom-event-dispatch`) —
/// but [`seed_focus_snapshot`](Self::seed_focus_snapshot) and the C2-correct SoT
/// designation run on every engine. This is a single injected dispatcher per
/// engine (cf. `MutationDispatcher`), not an observer registry.
pub trait FocusEventSink {
    /// The bound document's DOM — the transition's single mutable resource.
    fn dom(&mut self) -> &mut EcsDom;

    /// The bound (active) document.
    fn document(&self) -> Entity;

    /// §6.6.4 focus-update step 2.1 — fire `change` if a focused text control's
    /// value was user-edited since focus. **Owns the change-on-blur snapshot
    /// consume**: the sink calls `elidex_form::take_change_on_blur` (the one
    /// place the verdict is computed) and dispatches `change` iff it returns
    /// `true`; a no-op impl (VM, until its dispatch primitive lands) leaves the
    /// snapshot intact so the later dispatch can still observe it.
    fn commit_change_on_blur(&mut self, old: Entity);

    /// Seed the engine-bound change-on-blur baseline for the element gaining
    /// focus. There is no spec step for this — it is elidex's device for §6.6.4
    /// focus-update step 2.1's "the user has changed the element's value ...
    /// such that it is different to what it was when the control was first
    /// focused". The canonical transition calls it **after** the `focus`/`focusin`
    /// listeners (step 4.4): the seed is unmarked; the baseline is the value
    /// before the first user value change (plan D-1). (Seed timing does not decide whether a
    /// non-user write fires `change`; only the user-edit mark does.) Runs on
    /// **every** engine (the snapshot is `elidex-form` state, not event dispatch),
    /// so the canonical transition owns the seed *timing* while the form-layer call
    /// lives behind the sink.
    fn seed_focus_snapshot(&mut self, new: Entity);

    /// §6.6.4 focus-update steps 2.2-2.4 / 4.2-4.4 — fire a `blur`/`focus` event
    /// at `target`, with `related` the element on the other side of the
    /// transition. (The shell additionally fires the bubbling `focusin`/
    /// `focusout`; the VM no-ops until its dispatch primitive lands.)
    fn fire_focus_event(&mut self, target: Entity, kind: FocusEventKind, related: Option<Entity>);
}

/// §6.6.4 **focusing steps** (`#focusing-steps`) — move focus to `new_target`,
/// resolving it to a focusable area first.
///
/// `fallback` is the spec's optional fallback target (dialog / autofocus
/// callers); the shell UA path passes `None` — its candidate is pre-resolved and
/// the click→ancestor climb is PR-A2b. `trigger` selects the
/// get-the-focusable-area `"click"` vs `"other"` behaviour.
pub fn focusing_steps(
    sink: &mut dyn FocusEventSink,
    new_target: Entity,
    fallback: Option<Entity>,
    trigger: FocusTrigger,
) {
    // Step 1 — resolve a non-focusable-area target to its focusable area.
    let target = if is_focusable_area(sink.dom(), new_target) {
        new_target
    } else {
        match get_the_focusable_area(sink.dom(), new_target, trigger) {
            Some(area) => area,
            // Step 2 — null candidate: use the fallback target, else leave focus
            // unchanged (no fallback ⇒ return).
            None => match fallback {
                Some(fallback_target) => fallback_target,
                None => return,
            },
        }
    };
    // Step 3 (navigable container → content document) — unmodelled (iframe focus
    // = PR-A2b / `#11-oop-iframe-focus-lifecycle`). Step 4 (inert) — `inert`
    // unmodelled. Both no-op.
    let doc = sink.document();
    // The resolved target must belong to the bound document. `is_focusable_area`
    // only checks connectedness, so a focusable element in *another* connected
    // document in the same `EcsDom` (e.g. a `document.cloneNode()` subtree, reached
    // via the VM `HTMLElement.focus()` cutover, PR-A2c) would otherwise reach the
    // transition and have the world-wide `set_focus_bit` sweep the live document's
    // `FOCUS` bit. Guard it here, matching the bit-level writer's own check.
    if !is_in_document(sink.dom(), target, doc) {
        return;
    }
    let old = current_focus(sink.dom(), doc);
    // Step 5 — already the focused area: nothing to do. (Also preserves the
    // change-on-blur reseed suppression — a re-`focus()` after a user edit must
    // not refresh the snapshot baseline, returning here before step 4's seed.)
    if old == Some(target) {
        return;
    }
    // Steps 6-8 — the transition (single-element model; chain + pop = PR-A3).
    focus_update_steps(sink, old, Some(target));
}

/// §6.6.4 **unfocusing steps** (`#unfocusing-steps`) — the blur path for
/// `old_target`.
///
/// **Steps 5-6 guard**: `old_target` must be the document's currently focused
/// area, else return — do not clear the `FOCUS` bit or fire `blur` for a
/// non-holder. The shell `blur_current` always passes the live holder
/// ([`current_focus`]), so this is a no-op there; the guard makes the public seam
/// correct for a VM `element.blur()` on an *unfocused* receiver (PR-A2c), where
/// without it `other.blur()` would clear whichever element is actually focused and
/// fire `blur` at the wrong target — matching the bit-level [`super::blur`], which
/// already no-ops unless the receiver is the holder.
///
/// Step 1 (a `delegatesFocus` host `old_target` → its focused shadow area) and
/// step 3 (area / scrollable-region retarget) are unmodelled (PR-A2c); the
/// single-element model has no multi-entry old chain (PR-A3).
///
/// Step 8 has two branches — "run the focusing steps for topDocument's
/// viewport" when the system has focus, else "run the focus update steps given
/// old chain, an empty list, and null". elidex does not model system focus, so
/// both are the one viewport transition, `focusing_steps_for_viewport` (the
/// same function the step-17 fixup, [`focus_fixup_steps`], runs).
pub fn unfocusing_steps(sink: &mut dyn FocusEventSink, old_target: Entity) {
    let doc = sink.document();
    if current_focus(sink.dom(), doc) != Some(old_target) {
        return;
    }
    // Step 8.
    focusing_steps_for_viewport(sink);
}

/// WHATWG HTML "update the rendering" step 17 — the render-time **focus
/// fixup**: if the document's focused area is no longer a focusable area, run
/// the §6.6.4 focusing steps for the document's viewport.
///
/// §6.6.2 focusability is enforced as a focus-*time* gate on the writers
/// ([`is_focusable`] at every `focus()` entry), but a connected, focusable
/// element can *become* non-focusable while focus is still on it — its `hidden`
/// attribute lands (on it or an ancestor), `<input type>` flips to `hidden`, a
/// direct `disabled` lands, or it loses the `tabindex` / `contenteditable` /
/// `href` that made it focusable (§6.6.2 criteria 1/3/5). Fieldset-inherited
/// `disabled` is not in this crate's predicate (slot
/// `#11-focusable-area-fieldset-inherited-disabled`). The spec fixes that up at
/// the next rendering update, i.e. **asynchronously**, unlike the
/// **synchronous** fixup for *removal* (§2.1.4 removing steps,
/// `EcsDom::fire_after_remove`, which fires nothing). Until it runs,
/// `current_focus` / `activeElement` / the `:focus` selector all still report
/// the holder.
///
/// The transition is the canonical one, so it fires events through `sink`:
/// step 2.1 `change` (judged by the sink), then `blur` at the holder with a null
/// related target (the shell sink adds `focusout`). The spec chain's `focus` at
/// the Window (the new chain's Document entry) is not fired — the
/// single-element model fires only at elements (PR-A3). The predicate is
/// [`is_focusable`], the VM `focus()` gate's, so the fixup resets exactly what
/// that writer would refuse. Runs every rendering update, ungated; an idle frame
/// fires nothing (no holder, or a focusable one).
pub fn focus_fixup_steps(sink: &mut dyn FocusEventSink) {
    let doc = sink.document();
    let Some(holder) = current_focus(sink.dom(), doc) else {
        return;
    };
    if is_focusable(sink.dom(), holder) {
        return;
    }
    focusing_steps_for_viewport(sink);
}

/// §6.6.4 **focusing steps** (`#focusing-steps`) with the document's
/// **viewport** as the new focus target, in the single-element model: steps
/// 1–4 are vacuous (the viewport is a focusable area, not a navigable
/// container; `inert` is unmodelled), step 5 returns when the viewport is
/// already focused (no holder), and steps 6–8 are the focus update steps from
/// the holder to the viewport — no element is designated. The one
/// implementation of "focus the viewport": [`unfocusing_steps`] step 8 and the
/// step-17 [`focus_fixup_steps`] both run it.
fn focusing_steps_for_viewport(sink: &mut dyn FocusEventSink) {
    let doc = sink.document();
    // Step 5 — the viewport is already the focused area.
    let Some(old) = current_focus(sink.dom(), doc) else {
        return;
    };
    // Steps 6-8 — the transition (single-element model; chain + pop = PR-A3).
    focus_update_steps(sink, Some(old), None);
}

/// §6.6.4 **focus update steps** (`#focus-update-steps`), specialised to the
/// single-element model: `old`/`new` are the focusable elements (chain leaves).
fn focus_update_steps(sink: &mut dyn FocusEventSink, old: Option<Entity>, new: Option<Entity>) {
    let doc = sink.document();
    // Step 2.1 — `change`, fired while `old` is *still the designated focused
    // area*: §6.6.4 keeps `old` designated until step 4.1 (it is never undesignated
    // in step 2), so `document.activeElement` / `hasFocus` / `:focus` inside a
    // `change` handler still see `old`, the control being committed. (Step 2.1's
    // sub-step 1, "set the control's user validity to true", is deferred — elidex
    // does not model the user-validity / `:user-valid` flag yet; slot
    // `#11-focus-change-user-validity`.)
    if let Some(old) = old {
        sink.commit_change_on_blur(old); // step 2.1 — change (snapshot consume)
                                         // The `change` handler may have reentrantly designated a *different* focused
                                         // area via `focus()`. If so, that reentrant transition wins (reentrant-wins)
                                         // and the outer transition must not clobber it: a *canonical-seam* reentrant
                                         // focus (the shell sink, or any `focusing_steps` caller) fired its own full
                                         // `blur`(`old`)/`focus`(other) before returning here, so the outer path
                                         // re-firing them would double-dispatch. (The one non-firing reentrant writer
                                         // is the un-migrated VM `HTMLElement.focus()`, which only sets the FOCUS bit
                                         // and defers event dispatch — slot `#11-vm-host-synthetic-dom-event-dispatch`.
                                         // It reaches this path: the VM is the shell's engine, so a `change`
                                         // listener — including one fired by the step-17 `focus_fixup_steps` — can
                                         // call VM `focus()`, which wins without `focus`/`focusin` and leaves `old`
                                         // without a `blur`. PR-A2c routes the VM writer through this seam, so once
                                         // that event-dispatch slot lands `old`'s blur is fired before the bail on
                                         // the VM path too.) But a
                                         // handler that merely *clears* the old focus (removes `old`, so
                                         // `fire_after_remove` clears the bit, or calls the bit-level `blur()`) leaves
                                         // `current_focus` None with no reentrant target — there the outer transition
                                         // must still proceed to designate `new` (don't cancel the user's pending
                                         // click/Tab move). So return only on a reentrant focus to some *other*
                                         // element, never on a bare clear.
                                         // Fixup-fired listener interaction → `#11-vm-host-synthetic-dom-event-dispatch`.
        if current_focus(sink.dom(), doc).is_some_and(|c| c != old) {
            return;
        }
    }
    // Steps 2.2-2.4 — clear the FOCUS bit *before* `blur`/`focusout`, then fire
    // them. elidex browser-parity: `document.activeElement` / `hasFocus` report
    // `<body>` during `blur`/`focusout` (the pre-A2a shell + real browsers both do
    // this; the literal spec keeps `old` designated until step 4.1). Clearing here
    // — after `change`, before `blur` — also makes the step-4 gate's
    // `current_focus().is_some()` an exact reentrancy signal for the blur side
    // (the change side is guarded above, while `old` is still designated).
    set_focus_bit(sink.dom(), None);
    if let Some(old) = old {
        sink.fire_focus_event(old, FocusEventKind::Blur, new); // steps 2.2-2.4
    }
    // Step 3 — platform conventions: no-op.
    // Step 4 — reentrancy gate (reentrant-wins): a `blur`/`focusout` listener may
    // have designated a new focused area after the step-2 clear. If focus already
    // moved, that reentrant focus wins.
    if current_focus(sink.dom(), doc).is_some() {
        return;
    }
    if let Some(new) = new {
        // Step 4.1 — designate `new` as the focused area (SoT last). Sub-step 1,
        // "set the navigation API's focus changed during ongoing navigation to
        // true", is unmodelled (elidex has no Navigation API; permanent no-op until
        // that surface lands).
        set_focus_bit(sink.dom(), Some(new));
        sink.fire_focus_event(new, FocusEventKind::Focus, old); // steps 4.2-4.4
                                                                // Seed the change-on-blur baseline AFTER the `focus`/`focusin` listeners:
                                                                // the seed is unmarked; the baseline is the value before the first user
                                                                // value change (plan D-1).
                                                                // Guard: a `focusin` listener may have reentrantly moved focus on, so only
                                                                // seed if `new` is still the focused element (reentrant-wins).
        if current_focus(sink.dom(), doc) == Some(new) {
            sink.seed_focus_snapshot(new);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        focus_fixup_steps, focusing_steps, unfocusing_steps, FocusEventKind, FocusEventSink,
    };
    use crate::focus::tests::{connect_el, focusable_div};
    use crate::focus::{current_focus, raw_focus_holder, set_focus_bit, FocusTrigger};
    use elidex_ecs::{Attributes, EcsDom, Entity};

    #[derive(Default)]
    struct Recorded {
        events: Vec<(Entity, FocusEventKind, Option<Entity>)>,
        changes: Vec<Entity>,
        seeds: Vec<Entity>,
        /// `seeds.len()` captured when the `Focus` event fires — pins that the
        /// change-on-blur seed runs AFTER the focus/focusin dispatch (Codex R2:
        /// a focus-handler `value` write must be in the baseline, so `seeds` is
        /// still empty at focus-event time).
        seeds_len_at_focus: Option<usize>,
        /// `current_focus` captured when the `change` event fires — pins that the
        /// old control is STILL the designated focused area during `change` (Codex
        /// R3: §6.6.4 keeps `old` designated until step 4.1). `None` means either
        /// no `change` fired or no focus at that point; `changes` disambiguates.
        focus_at_change: Option<Entity>,
    }

    struct TestSink<'a> {
        dom: &'a mut EcsDom,
        document: Entity,
        rec: Recorded,
        /// On the first `blur`, set focus to this entity — simulating a reentrant
        /// `focus()` from a `blur`/`change` listener (exercises the reentrant-wins
        /// gate without re-entering the public API).
        reentrant_on_blur: Option<Entity>,
        /// On `change`, clear the FOCUS bit (no reentrant target) — simulating a
        /// `change` handler that removes / bit-level-`blur()`s the old control
        /// (Codex R5: a bare clear must not cancel the pending focus move).
        clear_on_change: bool,
    }

    impl<'a> TestSink<'a> {
        fn new(dom: &'a mut EcsDom, document: Entity) -> Self {
            Self {
                dom,
                document,
                rec: Recorded::default(),
                reentrant_on_blur: None,
                clear_on_change: false,
            }
        }
    }

    impl FocusEventSink for TestSink<'_> {
        fn dom(&mut self) -> &mut EcsDom {
            self.dom
        }
        fn document(&self) -> Entity {
            self.document
        }
        fn commit_change_on_blur(&mut self, old: Entity) {
            // Capture who is the designated focused area at `change` time — must
            // still be `old` (§6.6.4 step 2.1 precedes the step-4.1 re-designation).
            let doc = self.document;
            self.rec.focus_at_change = current_focus(self.dom, doc);
            self.rec.changes.push(old);
            if self.clear_on_change {
                // A `change` handler that removes / `blur()`s the old control: the
                // FOCUS bit is cleared with NO reentrant target.
                set_focus_bit(self.dom, None);
            }
        }
        fn seed_focus_snapshot(&mut self, new: Entity) {
            self.rec.seeds.push(new);
        }
        fn fire_focus_event(
            &mut self,
            target: Entity,
            kind: FocusEventKind,
            related: Option<Entity>,
        ) {
            self.rec.events.push((target, kind, related));
            if kind == FocusEventKind::Focus {
                self.rec.seeds_len_at_focus = Some(self.rec.seeds.len());
            }
            if kind == FocusEventKind::Blur {
                if let Some(reentrant) = self.reentrant_on_blur.take() {
                    set_focus_bit(self.dom, Some(reentrant));
                }
            }
        }
    }

    fn doc_with_two(dom: &mut EcsDom) -> (Entity, Entity, Entity) {
        let doc = dom.create_document_root();
        let a = focusable_div(dom);
        let b = focusable_div(dom);
        let _ = dom.append_child(doc, a);
        let _ = dom.append_child(doc, b);
        (doc, a, b)
    }

    #[test]
    fn focusing_steps_designates_last_and_fires_in_spec_order() {
        let mut dom = EcsDom::new();
        let (doc, a, b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, b, None, FocusTrigger::Other);
            sink.rec
        };

        // change(a) → blur(a, related=b) → [designate b] → focus(b, related=a).
        assert_eq!(rec.changes, vec![a]);
        assert_eq!(
            rec.events,
            vec![
                (a, FocusEventKind::Blur, Some(b)),
                (b, FocusEventKind::Focus, Some(a)),
            ]
        );
        assert_eq!(rec.seeds, vec![b], "the gaining element seeds the snapshot");
        assert_eq!(
            rec.seeds_len_at_focus,
            Some(0),
            "the change-on-blur snapshot is seeded AFTER the focus event (Codex R2: \
             a focus-handler value write must be in the baseline, not a user edit)"
        );
        assert_eq!(
            rec.focus_at_change,
            Some(a),
            "the losing control is STILL the designated focused area during `change` \
             (Codex R3: §6.6.4 keeps `old` designated until step 4.1)"
        );
        assert_eq!(current_focus(&dom, doc), Some(b));
    }

    #[test]
    fn focusing_steps_already_focused_is_a_no_op() {
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, a, None, FocusTrigger::Other);
            sink.rec
        };

        // Step 5 early-out: no events, and crucially NO reseed (a re-focus after a
        // user edit must not refresh the change-on-blur baseline).
        assert!(rec.events.is_empty());
        assert!(rec.seeds.is_empty());
        assert!(rec.changes.is_empty());
        assert_eq!(current_focus(&dom, doc), Some(a));
    }

    #[test]
    fn focusing_steps_non_focusable_no_fallback_leaves_focus() {
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        let plain = dom.create_element("span", Attributes::default()); // not a focusable area
        let _ = dom.append_child(doc, plain);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, plain, None, FocusTrigger::Other);
            sink.rec
        };

        // Step 2 (null candidate, no fallback) → return: focus is LEFT unchanged.
        assert!(rec.events.is_empty());
        assert_eq!(current_focus(&dom, doc), Some(a));
    }

    #[test]
    fn unfocusing_steps_blurs_and_clears() {
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            unfocusing_steps(&mut sink, a);
            sink.rec
        };

        // change(a) → blur(a, related=None); empty new chain → no designation.
        assert_eq!(rec.changes, vec![a]);
        assert_eq!(rec.events, vec![(a, FocusEventKind::Blur, None)]);
        assert!(rec.seeds.is_empty());
        assert_eq!(current_focus(&dom, doc), None);
    }

    #[test]
    fn unfocusing_steps_on_a_non_holder_is_a_noop() {
        // Codex R1 F3: §6.6.4 unfocusing-steps steps 5-6 — if the receiver is not
        // the document's currently focused area, return without clearing the bit or
        // firing `blur`. (Guards the public seam against a VM `element.blur()` on an
        // unfocused element, PR-A2c: `b.blur()` must not blur whoever holds focus.)
        let mut dom = EcsDom::new();
        let (doc, a, b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a)); // `a` holds focus; blur `b` (a non-holder)

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            unfocusing_steps(&mut sink, b);
            sink.rec
        };

        // No events fired, and `a` keeps focus (the bit was not cleared).
        assert!(rec.events.is_empty());
        assert!(rec.changes.is_empty());
        assert_eq!(current_focus(&dom, doc), Some(a));
    }

    #[test]
    fn reentrant_focus_during_blur_wins() {
        let mut dom = EcsDom::new();
        let (doc, a, b) = doc_with_two(&mut dom);
        let c = focusable_div(&mut dom);
        let _ = dom.append_child(doc, c);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            // A `blur` listener on `a` calls `c.focus()` mid-transition.
            sink.reentrant_on_blur = Some(c);
            focusing_steps(&mut sink, b, None, FocusTrigger::Other);
            sink.rec
        };

        // The reentrant focus on `c` wins: `b` is NEVER designated, no focus(b)
        // event fires, and the outer transition does not clobber `c`.
        assert_eq!(current_focus(&dom, doc), Some(c));
        assert_eq!(rec.events, vec![(a, FocusEventKind::Blur, Some(b))]);
        assert!(rec.seeds.is_empty(), "the outer transition seeds nothing");
    }

    #[test]
    fn change_handler_clearing_old_focus_still_focuses_new() {
        // Codex R5: a `change` listener that merely CLEARS the old focus (removes
        // the control so `fire_after_remove` clears the bit, or calls bit-level
        // `blur()`) — without focusing another element — must NOT cancel the pending
        // focus move; the transition still designates `new`. The change-side gate
        // returns only on a reentrant focus to a *different* element, not a bare
        // clear (which would leave focus on the viewport).
        let mut dom = EcsDom::new();
        let (doc, a, b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            sink.clear_on_change = true; // the `change` handler clears `a`'s bit
            focusing_steps(&mut sink, b, None, FocusTrigger::Other);
            sink.rec
        };

        // `b` is focused (the bare clear did not cancel the move), and the full
        // losing→gaining sequence still ran.
        assert_eq!(current_focus(&dom, doc), Some(b));
        assert_eq!(
            rec.events,
            vec![
                (a, FocusEventKind::Blur, Some(b)),
                (b, FocusEventKind::Focus, Some(a)),
            ]
        );
        assert_eq!(rec.seeds, vec![b]);
    }

    #[test]
    fn focusing_a_target_outside_the_bound_document_is_a_noop() {
        // Codex R5: the seam must reject a focus target in another connected
        // document in the same `EcsDom` (e.g. a cloned subtree reachable via the VM
        // `focus()` cutover) — else the world-wide `set_focus_bit` would sweep the
        // bound document's bit. `is_focusable_area` only checks connectedness.
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));
        // A focusable element in a SEPARATE document root.
        let other_doc = dom.create_document_root();
        let other = focusable_div(&mut dom);
        let _ = dom.append_child(other_doc, other);

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, other, None, FocusTrigger::Other);
            sink.rec
        };

        // The cross-document target is rejected: the bound document keeps `a`, and
        // no transition events fire.
        assert_eq!(current_focus(&dom, doc), Some(a));
        assert!(rec.events.is_empty());
        assert!(rec.changes.is_empty());
    }

    // ---- C14: the "update the rendering" step-17 focus fixup ----------------

    /// Run the fixup over `doc` with a fresh [`TestSink`].
    fn run_fixup(dom: &mut EcsDom, doc: Entity) -> Recorded {
        let mut sink = TestSink::new(dom, doc);
        focus_fixup_steps(&mut sink);
        sink.rec
    }

    /// Assert the fixup ran the canonical viewport transition from `h`:
    /// `change` judged at `h` while still designated, then `blur` at `h` with a
    /// null related target, nothing designated after.
    fn assert_fixup_transition(dom: &EcsDom, doc: Entity, h: Entity, rec: &Recorded, arm: &str) {
        assert_eq!(rec.changes, vec![h], "{arm}: step 2.1 judged at the holder");
        assert_eq!(
            rec.focus_at_change,
            Some(h),
            "{arm}: holder designated at `change`"
        );
        assert_eq!(
            rec.events,
            vec![(h, FocusEventKind::Blur, None)],
            "{arm}: blur at the holder, related target null"
        );
        assert!(rec.seeds.is_empty(), "{arm}: the viewport seeds nothing");
        assert_eq!(
            current_focus(dom, doc),
            None,
            "{arm}: focus is the viewport"
        );
    }

    /// C14 (a): a holder made non-focusable is reset through the canonical
    /// transition, on each focusability arm (relocated from
    /// `reconcile_focus_clears_when_focused_element_stops_being_focusable`).
    #[test]
    fn focus_fixup_steps_resets_a_non_focusable_holder_with_events() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        // hidden on the element itself:
        let el = connect_el(&mut dom, doc, "button");
        set_focus_bit(&mut dom, Some(el));
        dom.set_attribute(el, "hidden", "");
        let rec = run_fixup(&mut dom, doc);
        assert_fixup_transition(&dom, doc, el, &rec, "hidden");

        // hidden on an ancestor:
        let section = connect_el(&mut dom, doc, "section");
        let inner = connect_el(&mut dom, section, "button");
        set_focus_bit(&mut dom, Some(inner));
        dom.set_attribute(section, "hidden", "");
        let rec = run_fixup(&mut dom, doc);
        assert_fixup_transition(&dom, doc, inner, &rec, "ancestor hidden");

        // a direct disabled lands:
        let btn = connect_el(&mut dom, doc, "button");
        set_focus_bit(&mut dom, Some(btn));
        dom.set_attribute(btn, "disabled", "");
        let rec = run_fixup(&mut dom, doc);
        assert_fixup_transition(&dom, doc, btn, &rec, "disabled");

        // <input type> flips to hidden:
        let input = connect_el(&mut dom, doc, "input");
        set_focus_bit(&mut dom, Some(input));
        dom.set_attribute(input, "type", "hidden");
        let rec = run_fixup(&mut dom, doc);
        assert_fixup_transition(&dom, doc, input, &rec, "type=hidden");

        // <a> loses its href (focusable only via the href-gated Link default):
        let a = connect_el(&mut dom, doc, "a");
        dom.set_attribute(a, "href", "x");
        set_focus_bit(&mut dom, Some(a));
        dom.remove_attribute(a, "href");
        let rec = run_fixup(&mut dom, doc);
        assert_fixup_transition(&dom, doc, a, &rec, "href removed");
    }

    /// C14 (b): a still-focusable holder keeps focus and nothing fires
    /// (relocated from `reconcile_focus_keeps_a_still_focusable_element`).
    #[test]
    fn focus_fixup_steps_keeps_a_focusable_holder() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = connect_el(&mut dom, doc, "button");
        set_focus_bit(&mut dom, Some(el));
        dom.set_attribute(el, "title", "hi");
        let rec = run_fixup(&mut dom, doc);
        assert!(rec.events.is_empty() && rec.changes.is_empty());
        assert_eq!(current_focus(&dom, doc), Some(el));
    }

    /// C14 (c): no holder (the viewport is focused) — nothing fires.
    #[test]
    fn focus_fixup_steps_without_a_holder_is_a_noop() {
        let mut dom = EcsDom::new();
        let (doc, _a, _b) = doc_with_two(&mut dom);
        let rec = run_fixup(&mut dom, doc);
        assert!(rec.events.is_empty() && rec.changes.is_empty() && rec.seeds.is_empty());
        assert_eq!(current_focus(&dom, doc), None);
    }

    /// C14 (d): a non-focusable bit holder in another document of the same
    /// `EcsDom` (a `cloneNode` document) is not the bound document's focused
    /// area — nothing fires and its bit is kept.
    #[test]
    fn focus_fixup_steps_ignores_a_holder_outside_the_bound_document() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let other_doc = dom.create_document_root();
        let other = connect_el(&mut dom, other_doc, "button");
        set_focus_bit(&mut dom, Some(other));
        dom.set_attribute(other, "hidden", "");
        let rec = run_fixup(&mut dom, doc);
        assert!(rec.events.is_empty() && rec.changes.is_empty());
        assert_eq!(raw_focus_holder(&dom), Some(other), "the bit is kept");
    }

    /// C14 (e): a `blur` listener's reentrant `focus()` wins — the outer
    /// transition designates nothing after it.
    #[test]
    fn focus_fixup_steps_reentrant_focus_wins() {
        let mut dom = EcsDom::new();
        let (doc, a, c) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));
        dom.set_attribute(a, "hidden", "");
        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            sink.reentrant_on_blur = Some(c);
            focus_fixup_steps(&mut sink);
            sink.rec
        };
        assert_eq!(rec.events, vec![(a, FocusEventKind::Blur, None)]);
        assert!(rec.seeds.is_empty(), "the outer transition seeds nothing");
        assert_eq!(current_focus(&dom, doc), Some(c));
    }

    /// The fixup is asynchronous: a same-turn `hidden` leaves the holder the
    /// focused area until the fixup runs (relocated fixup half of
    /// `current_focus_keeps_holder_until_async_fixup`).
    #[test]
    fn focus_fixup_steps_is_the_async_fixup() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = focusable_div(&mut dom);
        let _ = dom.append_child(doc, el);
        set_focus_bit(&mut dom, Some(el));
        dom.set_attribute(el, "hidden", "");
        assert_eq!(
            current_focus(&dom, doc),
            Some(el),
            "pending until the fixup"
        );
        let _ = run_fixup(&mut dom, doc);
        assert_eq!(current_focus(&dom, doc), None, "the fixup cleared focus");
        assert_eq!(raw_focus_holder(&dom), None);
    }
}
